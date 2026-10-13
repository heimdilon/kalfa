//! Oturum bütçesi (5b tasarımı §4–§8): tek izin kapısı, kullanıcı kararı,
//! iptal ve kayıttan açılış. Sayılan tek şey sahte sağlayıcının `akis`
//! çağrısıdır: ağa çıkılan deneme.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use kalfa_core::arac::Arac;
use kalfa_core::butce::{ButceDevri, ButceEngeli, ButceGorunumu, ButceIslemId, ButceSinirlari};
use kalfa_core::gonderim::IstekSonucu;
use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::OturumId;
use kalfa_core::oturum::{OturumAyarlari, baslat_kayitla, devam_et};
use kalfa_core::protokol::{BitisNedeni, ButceKarari, Komut, Olay, SEMA_SURUMU, Zarf};
use kalfa_core::test_destek::*;
use kalfa_evren::{AkisOlayi, EvrenError};
use serde_json::{Value, json};
use tempfile::TempDir;

fn ayarlar(istek: u64, token: u64) -> OturumAyarlari {
    let mut ayar = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    ayar.butce = ButceSinirlari { istek, token };
    ayar
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

/// Kullanımı bildirilmeyen başarılı yanıt: 5b'de bilinmeyen tüketimdir.
fn kullanimsiz(metin: &str) -> SahteYanit {
    SahteYanit::Akis(vec![
        Ok(AkisOlayi::Metin(metin.into())),
        Ok(AkisOlayi::Bitti {
            neden: "stop".into(),
        }),
    ])
}

fn durdur() -> Komut {
    Komut::ButceKarari {
        karar: ButceKarari::Durdur,
    }
}

fn devam(sinirlar: ButceSinirlari, baslatilan: u64, bilinmeyen: u64) -> Komut {
    Komut::ButceKarari {
        karar: ButceKarari::DevamEt {
            sinirlar,
            gozlenen_baslatilan: baslatilan,
            gozlenen_bilinmeyen: bilinmeyen,
        },
    }
}

async fn kart_bekle(d: &mut Surucu) -> ButceGorunumu {
    match d
        .bekle(|o| matches!(o, Olay::ButceKarariGerekli { .. }))
        .await
        .yuk
    {
        Olay::ButceKarariGerekli { durum } => durum,
        diger => unreachable!("{diger:?}"),
    }
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

fn kart_var_mi(d: &Surucu) -> bool {
    d.olaylar()
        .iter()
        .any(|o| matches!(o, Olay::ButceKarariGerekli { .. }))
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

/// Bir model isteği araç çağrısı döner; sınır 1 ise ikinci model isteği
/// kartsız başlamaz. Durdur turu bitirir ve ağ çağrısı sayısı değişmez.
#[tokio::test]
async fn sinir_dolunca_ikinci_istek_kartla_bekler_ve_durdur_turu_bitirir() {
    let oku = Arc::new(SahteArac::okuyan("oku"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("oku", yol("a")),
        SahteYanit::metin("son"),
    ]);
    let mut d = oturum(&s, vec![a(&oku)], ayarlar(1, 5_000_000));
    d.gonder(mesaj("oku")).await;
    let kart = kart_bekle(&mut d).await;
    assert_eq!(
        s.istekler().len(),
        1,
        "ikinci model isteği karardan önce gitmemeli"
    );
    assert_eq!(
        (kart.baslatilan, kart.aktif, kart.engel),
        (1, 0, Some(ButceEngeli::Istek))
    );
    d.gonder(durdur()).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);
    assert_eq!(s.istekler().len(), 1);
}

/// Aynı kart `DevamEt` ile sürer; sayaçlar sıfırlanmaz, deneme 1'den devam eder.
#[tokio::test]
async fn devam_karari_ayni_isi_surdurur_ve_sayaci_sifirlamaz() {
    let oku = Arc::new(SahteArac::okuyan("oku"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("oku", yol("a")),
        SahteYanit::metin("son"),
    ]);
    let mut d = oturum(&s, vec![a(&oku)], ayarlar(1, 5_000_000));
    d.gonder(mesaj("oku")).await;
    let kart = kart_bekle(&mut d).await;
    let yeni = ButceSinirlari {
        istek: 3,
        token: 5_000_000,
    };
    d.gonder(devam(yeni, kart.baslatilan, kart.bilinmeyen))
        .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(s.istekler().len(), 2);
    let son = butce_durumlari(&d).last().copied().expect("bütçe durumu");
    assert_eq!(
        (son.baslatilan, son.sinirlar.istek, son.onaylanan_bilinmeyen),
        (2, 3, 0)
    );
}

/// Kullanımsız başarılı akış bilinmeyendir; sonraki istekten önce kart çıkar.
/// Yanlış gözlem akış açmadan reddedilir; doğru gözlem işi sürdürür.
#[tokio::test]
async fn bilinmeyen_tuketim_sonraki_istekten_once_sorulur_ve_yanlis_gozlem_reddedilir() {
    let s = SahteSaglayici::yeni(vec![kullanimsiz("ilk"), SahteYanit::metin("ikinci")]);
    let mut d = oturum(&s, vec![], ayarlar(10, 5_000_000));
    d.gonder(mesaj("ilk")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    d.gonder(mesaj("ikinci")).await;
    let kart = kart_bekle(&mut d).await;
    assert_eq!(
        (kart.bilinmeyen, kart.engel),
        (1, Some(ButceEngeli::Bilinmeyen))
    );
    assert_eq!(
        s.istekler().len(),
        1,
        "bilinmeyen tüketim varken akış açılmamalı"
    );
    let sinirlar = ButceSinirlari {
        istek: 10,
        token: 5_000_000,
    };
    d.gonder(devam(sinirlar, kart.baslatilan, 0)).await;
    d.bekle(|o| matches!(o, Olay::Hata { .. })).await;
    assert_eq!(s.istekler().len(), 1, "eski gözlem akış açmamalı");
    d.gonder(devam(sinirlar, kart.baslatilan, kart.bilinmeyen))
        .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(s.istekler().len(), 2);
}

/// Hız sınırı üretimden önce reddedilir: deneme bilinen 0 tokenla sayılır,
/// tek tekrar kart sormadan sürer ve iki istek sayılır.
#[tokio::test]
async fn hiz_siniri_tekrari_soru_sormadan_surer_iki_istek_sayilir() {
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::Hata(EvrenError::HizSiniri {
            tekrar_dene: Some(Duration::from_millis(1)),
        }),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = oturum(&s, vec![], ayarlar(10, 5_000_000));
    d.gonder(mesaj("merhaba")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(s.istekler().len(), 2);
    assert!(!kart_var_mi(&d), "tekrar için kart çıkmamalı");
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
    // İlk kalıcı yazım (bütçe ayarı) geçer; sonraki (istek başlangıcı) düşer.
    let kayit = BozulanHedef::kalicidan_sonra_bozuk(1);
    let baglam = test_baglami(s.clone(), vec![], sanal_siniflandirici());
    let (tutamac, olaylar) =
        baslat_kayitla(ayarlar(10, 5_000_000), baglam, kayit).expect("oturum başlamalı");
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
                sinirlar: ButceSinirlari {
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
    let s = SahteSaglayici::yeni(vec![]);
    let baglam = || test_baglami(s.clone(), vec![], sanal_siniflandirici());

    let (tutamac, olaylar, durum) =
        devam_et(&kayit_yolu, ayarlar(10, 100), baglam()).expect("kayıt açılmalı");
    assert_eq!(
        durum.butce.map(|g| (g.baslatilan, g.bilinmeyen, g.aktif)),
        Some((1, 1, 0))
    );
    let mut d = Surucu::yeni(tutamac, olaylar);
    d.bekle(|o| matches!(o, Olay::ButceDurumu { .. })).await;
    d.kapat().await;
    assert_eq!(bitis_sayisi(&kayit_yolu), 1);

    let (tutamac, olaylar, durum) =
        devam_et(&kayit_yolu, ayarlar(10, 100), baglam()).expect("kayıt yeniden açılmalı");
    assert_eq!(
        durum.butce.map(|g| (g.baslatilan, g.bilinmeyen, g.aktif)),
        Some((1, 1, 0))
    );
    let mut d = Surucu::yeni(tutamac, olaylar);
    d.gonder(mesaj("merhaba")).await;
    kart_bekle(&mut d).await;
    assert!(s.istekler().is_empty());
    d.gonder(durdur()).await;
    d.tur_sonu().await;
    d.kapat().await;
    assert_eq!(
        bitis_sayisi(&kayit_yolu),
        1,
        "ikinci açılış yeniden yazmamalı"
    );
}

/// Bütçesiz eski kayıt açılınca ayar ve devir kalıcı yazılır; ilk yeni istekten
/// önce kart çıkar.
#[tokio::test]
async fn butcesiz_eski_kayit_devir_yazar_ve_ilk_istekten_once_sorar() {
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
    let s = SahteSaglayici::yeni(vec![]);
    let baglam = test_baglami(s.clone(), vec![], sanal_siniflandirici());
    let (tutamac, olaylar, durum) =
        devam_et(&kayit_yolu, ayarlar(10, 1_000), baglam).expect("eski kayıt açılmalı");
    assert!(
        durum.butce.is_none(),
        "5b öncesi kayıtta bütçe görünümü yok"
    );
    let mut d = Surucu::yeni(tutamac, olaylar);
    d.gonder(mesaj("yeni")).await;
    let kart = kart_bekle(&mut d).await;
    assert_eq!(
        (kart.baslatilan, kart.bilinmeyen, kart.engel),
        (1, 1, Some(ButceEngeli::Bilinmeyen))
    );
    assert!(
        s.istekler().is_empty(),
        "kart kararından önce akış açılmamalı"
    );
    d.gonder(durdur()).await;
    d.tur_sonu().await;
    d.kapat().await;

    let kayitlar = OturumKaydi::oku(&kayit_yolu).expect("kayıt okunmalı");
    let devir = kayitlar.iter().find_map(|z| match &z.yuk {
        KayitOgesi::ButceAyarlandi { devir, .. } => *devir,
        _ => None,
    });
    assert_eq!(
        devir,
        Some(ButceDevri {
            istek: 1,
            token: 15,
            belirsiz: true,
        })
    );
    assert!(kayitlar.iter().any(|z| matches!(
        z.yuk,
        KayitOgesi::ButceAyarlandi { sinirlar, .. } if sinirlar == ButceSinirlari { istek: 10, token: 1_000 }
    )));
}

/// İptal, bütçe kartındaki beklemeyi sonlandırır; yeni akış açılmaz.
#[tokio::test]
async fn iptal_butce_beklemesini_sonlandirir() {
    let oku = Arc::new(SahteArac::okuyan("oku"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("oku", yol("a")),
        SahteYanit::metin("son"),
    ]);
    let mut d = oturum(&s, vec![a(&oku)], ayarlar(1, 5_000_000));
    d.gonder(mesaj("oku")).await;
    let tur = d
        .bekle(|o| matches!(o, Olay::TurBasladi))
        .await
        .tur
        .expect("tur kimliği");
    kart_bekle(&mut d).await;
    d.gonder(Komut::Iptal { tur }).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);
    assert_eq!(s.istekler().len(), 1);
}
