//! Denetçinin oturum bütçesi (5b tasarımı §4.1, §6, §7): her ağ denemesi kendi
//! izni ve tek bitişini alır; izin beklemesi 30 saniyelik ağ süresini tüketmez.
//! Sayılan tek şey sahte sağlayıcının `sohbet` çağrısıdır: ağa çıkılan deneme.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use kalfa_core::arac::Arac;
use kalfa_core::butce::{ButceEngeli, ButceIslemId, ButceSinirlari};
use kalfa_core::denetci::{
    DenetimGirdisi, DenetimKipi, DenetimSonucu, GuvenlikDenetcisi, YapilandirilmisSiniflandirici,
};
use kalfa_core::gonderim::{GunlukToken, IstekSonucu, gunluk_uret};
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::oturum::OturumAyarlari;
use kalfa_core::protokol::{BitisNedeni, ButceKarari, Komut, Olay};
use kalfa_core::test_destek::*;
use kalfa_evren::EvrenError;
use serde_json::{Value, json};

/// Oturum ayarı: istek sınırı verilen, bilinen token sınırı geniş; denetçi açık.
fn ayarlar(istek: u64) -> OturumAyarlari {
    let mut ayar = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    ayar.butce = ButceSinirlari {
        istek,
        token: 5_000_000,
    };
    ayar.denetci_kipi = DenetimKipi::Uyar;
    ayar
}

fn oku() -> Arc<dyn Arac> {
    Arc::new(SahteArac::okuyan("oku"))
}

fn mesaj() -> Komut {
    Komut::KullaniciMesaji {
        metin: "oku".into(),
    }
}

fn yol(y: &str) -> Value {
    json!({ "yol": y })
}

/// Üretimden önce reddedilen ağ denemesi (hız sınırı): bilinen 0 tokenla sayılır.
fn hiz_siniri() -> SahteSohbet {
    SahteSohbet::Hata(EvrenError::HizSiniri {
        tekrar_dene: Some(Duration::from_millis(1)),
    })
}

fn guvenli() -> SahteSohbet {
    SahteSohbet::Json(json!({"guvenli": true, "kategori": "yok", "gerekce": "temiz"}))
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

fn denetim_sonucu_guvenli(d: &Surucu) -> bool {
    d.olaylar().iter().any(|o| {
        matches!(
            o,
            Olay::DenetimSonucu {
                sonuc: DenetimSonucu::Guvenli,
                ..
            }
        )
    })
}

/// Sınır bir istek kalınca: ana istek bir hakkı alır, denetçinin ilk denemesi
/// kalan son hakkı alır; 429 sonrası ikinci deneme kartta bekler.
fn sinir_bir_istek_kalir_senaryosu(s: &Arc<SahteSaglayici>, ayar: OturumAyarlari) -> Surucu {
    s.sohbet_ekle(hiz_siniri());
    s.sohbet_ekle(guvenli());
    test_oturumu(
        test_baglami(s.clone(), vec![oku()], sanal_siniflandirici()),
        ayar,
    )
}

fn sinir_bir_istek_kalir_saglayici() -> Arc<SahteSaglayici> {
    SahteSaglayici::yeni(vec![
        SahteYanit::arac("oku", yol("a")),
        SahteYanit::metin("son"),
    ])
}

/// Hız sınırı tekrarı yeni izin alır: iki başlangıç, iki bitiş (ilki bilinen 0),
/// günlükte iki denetçi satırı (ilki hata, ikincisi başarılı).
#[tokio::test]
async fn denetci_429_sonrasi_tekrar_yeni_izin_alir_ve_her_deneme_bir_satir() {
    let dizin = tempfile::tempdir().unwrap();
    let kayit_yolu = dizin.path().join("oturum.jsonl");
    let s = sinir_bir_istek_kalir_saglayici();
    s.sohbet_ekle(hiz_siniri());
    s.sohbet_ekle(guvenli());
    let mut ayar = ayarlar(10);
    ayar.kayit_yolu = Some(kayit_yolu.clone());
    let mut d = test_oturumu(
        test_baglami(s.clone(), vec![oku()], sanal_siniflandirici()),
        ayar,
    );
    d.gonder(mesaj()).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(s.sohbet_istekleri().len(), 2);
    d.kapat().await;

    let kayitlar = OturumKaydi::oku(&kayit_yolu).unwrap();
    let basla: Vec<ButceIslemId> = kayitlar
        .iter()
        .filter_map(|z| match &z.yuk {
            KayitOgesi::ButceIstegiBasladi { islem, amac } if amac == "denetçi" => Some(*islem),
            _ => None,
        })
        .collect();
    assert_eq!(basla.len(), 2, "tekrar yeni izin almalı");
    let bitisler: Vec<(ButceIslemId, Option<GunlukToken>)> = kayitlar
        .iter()
        .filter_map(|z| match &z.yuk {
            KayitOgesi::ButceIstegiBitti { islem, kullanim } if basla.contains(islem) => {
                Some((*islem, *kullanim))
            }
            _ => None,
        })
        .collect();
    assert_eq!(bitisler.len(), 2, "her deneme tek bitiş almalı");
    assert_eq!(bitisler[0], (basla[0], Some(GunlukToken::default())));
    assert_eq!(
        bitisler[1].1,
        Some(GunlukToken {
            istem: 10,
            tamamlama: 5,
            dusunce: 0,
        })
    );

    let gunluk = gunluk_uret(&kayitlar);
    let denetci: Vec<_> = gunluk.iter().filter(|g| g.amac == "denetci").collect();
    assert_eq!(denetci.len(), 2, "{gunluk:?}");
    assert!(matches!(denetci[0].sonuc, IstekSonucu::Hata { .. }));
    assert_eq!(denetci[1].sonuc, IstekSonucu::Basarili);
}

/// Bir istek kalmışken denetçinin ikinci izni kart bekler; kart gelmeden ağa
/// çıkılmaz. `DevamEt` sınırı yükseltir ve deneme sürer.
#[tokio::test]
async fn sinir_bir_istek_kalinca_denetcinin_ikinci_izni_kartta_bekler_ve_devam_surer() {
    let s = sinir_bir_istek_kalir_saglayici();
    let mut d = sinir_bir_istek_kalir_senaryosu(&s, ayarlar(2));
    d.gonder(mesaj()).await;
    let kart = match d
        .bekle(|o| matches!(o, Olay::ButceKarariGerekli { .. }))
        .await
        .yuk
    {
        Olay::ButceKarariGerekli { durum } => durum,
        diger => unreachable!("{diger:?}"),
    };
    assert_eq!(
        (kart.baslatilan, kart.aktif, kart.engel),
        (2, 0, Some(ButceEngeli::Istek))
    );
    assert_eq!(
        s.sohbet_istekleri().len(),
        1,
        "kart kararından önce ikinci deneme ağa çıkmamalı"
    );
    let yeni = ButceSinirlari {
        istek: 5,
        token: 5_000_000,
    };
    d.gonder(devam(yeni, kart.baslatilan, kart.bilinmeyen))
        .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(s.sohbet_istekleri().len(), 2);
    assert!(denetim_sonucu_guvenli(&d), "deneme sürmeliydi");
}

/// Kart `Durdur` ile kapanırsa denetçinin ikinci denemesi ağa çıkmaz.
#[tokio::test]
async fn durdur_kartta_bekleyen_denetim_ek_cagri_yapmaz() {
    let s = sinir_bir_istek_kalir_saglayici();
    let mut d = sinir_bir_istek_kalir_senaryosu(&s, ayarlar(2));
    d.gonder(mesaj()).await;
    d.bekle(|o| matches!(o, Olay::ButceKarariGerekli { .. }))
        .await;
    d.gonder(Komut::ButceKarari {
        karar: ButceKarari::Durdur,
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);
    assert_eq!(s.sohbet_istekleri().len(), 1, "izinsiz ağa çıkılmamalı");
    assert_eq!(s.istekler().len(), 1, "ana istek de sürmemeli");
}

/// Kart 60 saniye açık beklese de denetim zaman aşımına uğramaz: 30 saniyelik
/// süre yalnız ağ ve yeniden deneme beklemesini sayar.
#[tokio::test(start_paused = true)]
async fn butce_beklemesi_denetim_zaman_asimini_tuketmez() {
    let s = sinir_bir_istek_kalir_saglayici();
    let mut d = sinir_bir_istek_kalir_senaryosu(&s, ayarlar(2));
    d.gonder(mesaj()).await;
    let kart = match d
        .bekle(|o| matches!(o, Olay::ButceKarariGerekli { .. }))
        .await
        .yuk
    {
        Olay::ButceKarariGerekli { durum } => durum,
        diger => unreachable!("{diger:?}"),
    };
    tokio::time::sleep(Duration::from_secs(60)).await;
    let yeni = ButceSinirlari {
        istek: 5,
        token: 5_000_000,
    };
    d.gonder(devam(yeni, kart.baslatilan, kart.bilinmeyen))
        .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert!(
        denetim_sonucu_guvenli(&d),
        "bütçe beklemesi zaman aşımı saymamalı"
    );
}

/// İzin vermeyen kanalda denetçi ağa çıkmaz: `Bilinmiyor` döner, deneme kaydı
/// oluşmaz.
#[tokio::test]
async fn izin_yoksa_denetci_ag_cikmaz_ve_bilinmiyor_der() {
    let s = SahteSaglayici::yeni(vec![]);
    let d = YapilandirilmisSiniflandirici::yeni(s.clone());
    let girdi = DenetimGirdisi {
        kaynak: "dosya_oku: x".into(),
        icerik: "test".into(),
    };
    let yanit = d.denetle(&girdi, &reddeden_butce_talepci()).await;
    assert_eq!(
        yanit.sonuc,
        DenetimSonucu::Bilinmiyor {
            neden: "oturum bütçesi izin vermedi".into(),
        }
    );
    assert!(s.sohbet_istekleri().is_empty());
    assert!(yanit.denemeler.is_empty());
}

/// Zaman aşımına uğrayan denemenin kullanımı bilinmez; deneme yine de kaydedilir.
#[tokio::test(start_paused = true)]
async fn zaman_asimli_deneme_kullanimsiz_kaydedilir() {
    let s = SahteSaglayici::yeni(vec![]);
    s.sohbet_ekle(SahteSohbet::Gecikme(Duration::from_secs(35)));
    let d = YapilandirilmisSiniflandirici::yeni(s.clone());
    let girdi = DenetimGirdisi {
        kaynak: "dosya_oku: yavas".into(),
        icerik: "test".into(),
    };
    let yanit = d.denetle(&girdi, &sinirsiz_butce_talepci()).await;
    assert!(matches!(yanit.sonuc, DenetimSonucu::Bilinmiyor { .. }));
    assert_eq!(yanit.denemeler.len(), 1);
    assert_eq!(yanit.denemeler[0].kullanim, None);
    assert!(matches!(yanit.denemeler[0].sonuc, IstekSonucu::Hata { .. }));
}

/// Tur özeti her denemenin kullanımını bir kez toplar: iki ana istek ve bir
/// denetçi isteği, her biri 10/5, toplam 30/15 (denetçi iki kez sayılmaz).
#[tokio::test]
async fn tur_ozeti_denetci_kullanimini_bir_kez_sayar() {
    let s = sinir_bir_istek_kalir_saglayici();
    s.sohbet_ekle(guvenli());
    let mut d = test_oturumu(
        test_baglami(s.clone(), vec![oku()], sanal_siniflandirici()),
        ayarlar(10),
    );
    d.gonder(mesaj()).await;
    let zarf = d.bekle(|o| matches!(o, Olay::TurOzeti { .. })).await;
    let Olay::TurOzeti { ozet } = zarf.yuk else {
        unreachable!()
    };
    assert_eq!(s.sohbet_istekleri().len(), 1, "tek denetim isteği");
    assert_eq!(
        (ozet.istekler.istem, ozet.istekler.tamamlama),
        (30, 15),
        "{:?}",
        ozet.istekler
    );
}

/// İptal, biten denetçi denemesinin bitişi kuyruktayken gelirse (429 sonrası
/// tekrar beklenirken) bitiş tur sonu süpürmesinden önce işlenir: bilinen 0
/// token bilinmeyene dönmez.
#[tokio::test]
async fn iptalden_once_kuyruktaki_denetci_bitisi_bilinen_kapanir() {
    let dizin = tempfile::tempdir().unwrap();
    let kayit_yolu = dizin.path().join("oturum.jsonl");
    let s = sinir_bir_istek_kalir_saglayici();
    s.sohbet_ekle(hiz_siniri());
    let mut ayar = ayarlar(10);
    ayar.kayit_yolu = Some(kayit_yolu.clone());
    let mut d = test_oturumu(
        test_baglami(s.clone(), vec![oku()], sanal_siniflandirici()),
        ayar,
    );
    let tutamac = d.tutamac.clone();
    s.sohbet_kancasi(move || {
        tutamac.iptal_et();
    });
    d.gonder(mesaj()).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);
    assert_eq!(s.sohbet_istekleri().len(), 1);
    d.kapat().await;

    let kayitlar = OturumKaydi::oku(&kayit_yolu).unwrap();
    let denetci: Vec<ButceIslemId> = kayitlar
        .iter()
        .filter_map(|z| match &z.yuk {
            KayitOgesi::ButceIstegiBasladi { islem, amac } if amac == "denetçi" => Some(*islem),
            _ => None,
        })
        .collect();
    assert_eq!(denetci.len(), 1);
    let bitisler: Vec<Option<GunlukToken>> = kayitlar
        .iter()
        .filter_map(|z| match &z.yuk {
            KayitOgesi::ButceIstegiBitti { islem, kullanim } if *islem == denetci[0] => {
                Some(*kullanim)
            }
            _ => None,
        })
        .collect();
    assert_eq!(bitisler, vec![Some(GunlukToken::default())]);
}
