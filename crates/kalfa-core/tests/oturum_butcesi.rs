//! Oturumun deneme ve token defteri: tek izin kapısı, kalıcı başlangıç/bitiş
//! ve eski kayıtlardan açılış. Sayılan tek şey sahte sağlayıcının `akis`
//! çağrısıdır: ağa çıkılan deneme.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use kalfa_core::arac::Arac;
use kalfa_core::butce::{ButceGorunumu, ButceIslemId};
use kalfa_core::gonderim::IstekSonucu;
use kalfa_core::izin::Kip;
use kalfa_core::kayit::{EskiButceSinirlari, KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::OturumId;
use kalfa_core::oturum::{OturumAyarlari, baslat_kayitla, devam_et};
use kalfa_core::protokol::{BitisNedeni, Komut, Olay, SEMA_SURUMU, Zarf};
use kalfa_core::test_destek::*;
use kalfa_evren::{AkisOlayi, EvrenError};
use serde_json::{Value, json};
use tempfile::TempDir;

fn ayarlar() -> OturumAyarlari {
    OturumAyarlari::yeni("glm-5.3", Path::new("calisma"))
}

fn a(arac: &Arc<SahteArac>) -> Arc<dyn Arac> {
    arac.clone()
}

fn oturum(s: &Arc<SahteSaglayici>, araclar: Vec<Arc<dyn Arac>>, ayar: OturumAyarlari) -> Surucu {
    test_oturumu(
        test_baglami(s.clone(), araclar, sanal_siniflandirici()),
        ayar,
    )
}

fn mesaj(metin: &str) -> Komut {
    Komut::KullaniciMesaji {
        metin: metin.into(),
    }
}

fn yol(y: &str) -> Value {
    json!({ "yol": y })
}

/// Kullanımı bildirilmeyen başarılı yanıt: bilinmeyen tüketimdir.
fn kullanimsiz(metin: &str) -> SahteYanit {
    SahteYanit::Akis(vec![
        Ok(AkisOlayi::Metin(metin.into())),
        Ok(AkisOlayi::Bitti {
            neden: "stop".into(),
        }),
    ])
}

fn butce_durumlari(d: &Surucu) -> Vec<ButceGorunumu> {
    d.olaylar()
        .into_iter()
        .filter_map(|o| match o {
            Olay::ButceDurumu { durum } => Some(durum),
            _ => None,
        })
        .collect()
}

fn acilis() -> KayitOgesi {
    KayitOgesi::OturumAcildi {
        model: "glm-5.3".into(),
        kip: Kip::Siki,
        calisma_alani: None,
    }
}

/// Elle yazılmış bir kayıt (sürüm eski ya da yeni): her öğe kalıcı yazılır.
fn kayit_yaz(yol: &Path, ogeler: Vec<KayitOgesi>) {
    let mut kayit = OturumKaydi::olustur(yol).expect("kayıt oluşturulmalı");
    let oturum = OturumId::yeni();
    for (i, oge) in ogeler.into_iter().enumerate() {
        let zarf = Zarf {
            sema: SEMA_SURUMU,
            oturum,
            tur: None,
            sira: i as u64 + 1,
            yuk: oge,
        };
        kayit.yaz_kalici(&zarf).expect("kayıt yazılmalı");
    }
}

fn bitis_sayisi(yol: &Path) -> usize {
    OturumKaydi::oku(yol)
        .expect("kayıt okunmalı")
        .iter()
        .filter(|z| matches!(z.yuk, KayitOgesi::ButceIstegiBitti { kullanim: None, .. }))
        .count()
}

/// 205 ana isteklik bir tur hiçbir bütçe sınırına takılmadan tamamlanır.
#[tokio::test]
async fn ikiyuz_istegi_asan_oturum_durmaz() {
    let oku = Arc::new(SahteArac::okuyan("oku"));
    let mut senaryo: Vec<SahteYanit> = (0..204)
        .map(|_| SahteYanit::arac("oku", yol("a")))
        .collect();
    senaryo.push(SahteYanit::metin("son"));
    let s = SahteSaglayici::yeni(senaryo);
    let mut ayar = ayarlar();
    ayar.sinirlar.azami_model_turu = 210;
    ayar.sinirlar.azami_arac_cagrisi = 210;
    let mut d = oturum(&s, vec![a(&oku)], ayar);
    d.gonder(mesaj("oku")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(s.istekler().len(), 205);
    let son = butce_durumlari(&d).last().copied().expect("bütçe durumu");
    assert_eq!(
        son,
        ButceGorunumu {
            baslatilan: 205,
            bilinen_token: 205 * 15,
            bilinmeyen: 0,
            aktif: 0,
        }
    );
}

/// Kullanımsız başarılı akış bilinmeyendir; sonraki isteği durdurmaz.
#[tokio::test]
async fn bilinmeyen_tuketim_sonraki_istegi_durdurmaz() {
    let s = SahteSaglayici::yeni(vec![kullanimsiz("ilk"), SahteYanit::metin("ikinci")]);
    let mut d = oturum(&s, vec![], ayarlar());
    d.gonder(mesaj("ilk")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    d.gonder(mesaj("ikinci")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(s.istekler().len(), 2);
    let son = butce_durumlari(&d).last().copied().expect("bütçe durumu");
    assert_eq!(
        son,
        ButceGorunumu {
            baslatilan: 2,
            bilinen_token: 15,
            bilinmeyen: 1,
            aktif: 0,
        }
    );
}

/// Hız sınırı üretimden önce reddedilir: deneme bilinen 0 tokenla sayılır,
/// tek tekrar soru sormadan sürer ve iki istek sayılır.
#[tokio::test]
async fn hiz_siniri_tekrari_soru_sormadan_surer_iki_istek_sayilir() {
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::Hata(EvrenError::HizSiniri {
            tekrar_dene: Some(Duration::from_millis(1)),
        }),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = oturum(&s, vec![], ayarlar());
    d.gonder(mesaj("merhaba")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(s.istekler().len(), 2);
    let son = butce_durumlari(&d).last().copied().expect("bütçe durumu");
    assert_eq!(
        (son.baslatilan, son.bilinen_token, son.bilinmeyen),
        (2, 15, 0)
    );
}

/// Başlangıç kaydı yazılamazsa izin verilmez ve ağa hiç çıkılmaz.
#[tokio::test]
async fn kalici_baslangic_yazimi_basarisizsa_ag_cagrilmaz() {
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("yasak")]);
    let kayit = BozulanHedef::kalicida_bozuk();
    let baglam = test_baglami(s.clone(), vec![], sanal_siniflandirici());
    let (tutamac, olaylar) = baslat_kayitla(ayarlar(), baglam, kayit).expect("oturum başlamalı");
    let mut d = Surucu::yeni(tutamac, olaylar);
    d.gonder(mesaj("merhaba")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Hata);
    assert!(s.istekler().is_empty(), "kayıt yazılamadan ağa çıkılmamalı");
}

/// Sonuçsuz başlangıç açılışta bir kez bilinmeyen olarak kapanır; ikinci
/// açılış yeniden yazmaz ve sayaç sıfırlanmaz.
#[tokio::test]
async fn sonuclanmamis_baslangic_kayittan_acilirken_bir_kez_kapanir() {
    let dizin = TempDir::new().expect("geçici dizin");
    let kayit_yolu = dizin.path().join("oturum.jsonl");
    kayit_yaz(
        &kayit_yolu,
        vec![
            acilis(),
            KayitOgesi::ButceAyarlandi {
                sinirlar: EskiButceSinirlari {
                    istek: 10,
                    token: 100,
                },
                devir: None,
            },
            KayitOgesi::ButceIstegiBasladi {
                islem: ButceIslemId(1),
                amac: "ana".into(),
            },
        ],
    );
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("yeni")]);
    let baglam = || test_baglami(s.clone(), vec![], sanal_siniflandirici());

    let (tutamac, olaylar, durum) =
        devam_et(&kayit_yolu, ayarlar(), baglam()).expect("kayıt açılmalı");
    assert_eq!(
        (
            durum.butce.baslatilan,
            durum.butce.bilinmeyen,
            durum.butce.aktif
        ),
        (1, 1, 0)
    );
    let mut d = Surucu::yeni(tutamac, olaylar);
    d.bekle(|o| matches!(o, Olay::ButceDurumu { .. })).await;
    d.kapat().await;
    assert_eq!(bitis_sayisi(&kayit_yolu), 1);

    let (tutamac, olaylar, durum) =
        devam_et(&kayit_yolu, ayarlar(), baglam()).expect("kayıt yeniden açılmalı");
    assert_eq!(
        (
            durum.butce.baslatilan,
            durum.butce.bilinmeyen,
            durum.butce.aktif
        ),
        (1, 1, 0)
    );
    let mut d = Surucu::yeni(tutamac, olaylar);
    d.gonder(mesaj("merhaba")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(s.istekler().len(), 1);
    d.kapat().await;
    assert_eq!(
        bitis_sayisi(&kayit_yolu),
        1,
        "ikinci açılış yeniden yazmamalı"
    );
}

/// Bütçesiz eski kayıt açılınca tüketim bellekte devralınır, kayda `ButceAyarlandi`
/// yazılmaz ve sonraki istek kartsız sürer.
#[tokio::test]
async fn butcesiz_eski_kayit_bellekte_devralinir_kayda_yazmaz() {
    let dizin = TempDir::new().expect("geçici dizin");
    let kayit_yolu = dizin.path().join("oturum.jsonl");
    kayit_yaz(
        &kayit_yolu,
        vec![
            acilis(),
            KayitOgesi::Istek {
                amac: "ana".into(),
                istek_id: None,
                kredi: None,
                yonlendirilen_model: None,
                model: None,
                yeni: Vec::new(),
                onceki: 0,
                sonuc: IstekSonucu::Basarili,
                butce_islem: None,
            },
            KayitOgesi::Kullanim {
                istem: 10,
                tamamlama: 5,
                dusunce: 0,
            },
        ],
    );
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("tamam")]);
    let baglam = test_baglami(s.clone(), vec![], sanal_siniflandirici());
    let (tutamac, olaylar, durum) =
        devam_et(&kayit_yolu, ayarlar(), baglam).expect("eski kayıt açılmalı");
    assert_eq!(
        durum.butce,
        ButceGorunumu {
            baslatilan: 1,
            bilinen_token: 15,
            bilinmeyen: 1,
            aktif: 0,
        }
    );
    let mut d = Surucu::yeni(tutamac, olaylar);
    d.gonder(mesaj("yeni")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(s.istekler().len(), 1);
    let son = butce_durumlari(&d).last().copied().expect("bütçe durumu");
    assert_eq!(
        son,
        ButceGorunumu {
            baslatilan: 2,
            bilinen_token: 30,
            bilinmeyen: 1,
            aktif: 0,
        }
    );
    d.kapat().await;

    let kayitlar = OturumKaydi::oku(&kayit_yolu).expect("kayıt okunmalı");
    assert!(
        !kayitlar
            .iter()
            .any(|z| matches!(z.yuk, KayitOgesi::ButceAyarlandi { .. })),
        "eski kayda ButceAyarlandi yazılmamalı"
    );
}
