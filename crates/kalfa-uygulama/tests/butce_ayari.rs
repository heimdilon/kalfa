//! Oturum bütçesi ayarı (5b tasarımı §9): `ayarlar.json`'daki varsayılan, bozuk
//! ve geçersiz değerler, doğrulanmış yazma ve diğer ayarların korunması.

use kalfa_core::butce::{AZAMI_SINIR, ButceSinirlari, VARSAYILAN_ISTEK, VARSAYILAN_TOKEN};
use kalfa_core::denetci::DenetimKipi;
use kalfa_core::test_destek::SahteSaglayici;
use kalfa_uygulama::test_destek::test_uygulamasi;

fn varsayilan() -> ButceSinirlari {
    ButceSinirlari {
        istek: VARSAYILAN_ISTEK,
        token: VARSAYILAN_TOKEN,
    }
}

#[test]
fn ayar_dosyasi_yoksa_varsayilan_butce() {
    let d = tempfile::tempdir().unwrap();
    let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
    assert_eq!(t.uygulama.oturum_butcesi(), varsayilan());
    assert_eq!(
        varsayilan(),
        ButceSinirlari {
            istek: 200,
            token: 5_000_000
        }
    );
}

#[test]
fn eski_ayar_dosyasinda_butce_alani_yoksa_varsayilan_kullanilir() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("ayarlar.json"), r#"{"denetci_kipi":"Uyar"}"#).unwrap();
    let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
    assert_eq!(t.uygulama.oturum_butcesi(), varsayilan());
    assert_eq!(t.uygulama.denetci_kipi().kip, DenetimKipi::Uyar);
}

#[test]
fn bozuk_butce_degeri_yalnizca_kendisini_dusurur() {
    for metin in [
        r#"{"denetci_kipi":"Uyar","oturum_butcesi":{"istek":"on","token":5}}"#,
        r#"{"denetci_kipi":"Uyar","oturum_butcesi":{"istek":-1,"token":5}}"#,
        r#"{"denetci_kipi":"Uyar","oturum_butcesi":{"istek":3}}"#,
        r#"{"denetci_kipi":"Uyar","oturum_butcesi":null}"#,
    ] {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("ayarlar.json"), metin).unwrap();
        let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
        assert_eq!(t.uygulama.oturum_butcesi(), varsayilan(), "{metin}");
        let durum = t.uygulama.denetci_kipi();
        assert_eq!(durum.kip, DenetimKipi::Uyar, "{metin}");
        assert_eq!(durum.ayar_hatasi, None, "{metin}");
    }
}

#[test]
fn sifir_ve_sinir_ustu_deger_varsayilana_doner() {
    for (istek, token) in [
        (0, 10),
        (10, 0),
        (AZAMI_SINIR + 1, 10),
        (10, AZAMI_SINIR + 1),
    ] {
        let d = tempfile::tempdir().unwrap();
        let metin = format!(r#"{{"oturum_butcesi":{{"istek":{istek},"token":{token}}}}}"#);
        std::fs::write(d.path().join("ayarlar.json"), &metin).unwrap();
        let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
        assert_eq!(t.uygulama.oturum_butcesi(), varsayilan(), "{metin}");
    }
}

#[test]
fn gecerli_butce_yeniden_acilista_okunur() {
    let d = tempfile::tempdir().unwrap();
    let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
    let yeni = t.uygulama.oturum_butcesini_ayarla(3, 1000).unwrap();
    assert_eq!(
        yeni,
        ButceSinirlari {
            istek: 3,
            token: 1000
        }
    );
    drop(t);
    let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
    assert_eq!(t.uygulama.oturum_butcesi(), yeni);
}

#[test]
fn sinirin_en_ustu_kabul_edilir() {
    let d = tempfile::tempdir().unwrap();
    let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
    let yeni = t
        .uygulama
        .oturum_butcesini_ayarla(AZAMI_SINIR, AZAMI_SINIR)
        .unwrap();
    assert_eq!(
        yeni,
        ButceSinirlari {
            istek: AZAMI_SINIR,
            token: AZAMI_SINIR
        }
    );
}

#[test]
fn gecersiz_yeni_varsayilan_reddedilir_dosya_ve_bellek_degismez() {
    let d = tempfile::tempdir().unwrap();
    let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
    t.uygulama.oturum_butcesini_ayarla(3, 1000).unwrap();
    let once = std::fs::read_to_string(d.path().join("ayarlar.json")).unwrap();
    for (istek, token) in [(0, 1000), (3, 0), (AZAMI_SINIR + 1, 1000)] {
        assert!(
            t.uygulama.oturum_butcesini_ayarla(istek, token).is_err(),
            "{istek}/{token} kabul edilmemeli"
        );
    }
    assert_eq!(
        std::fs::read_to_string(d.path().join("ayarlar.json")).unwrap(),
        once
    );
    assert_eq!(
        t.uygulama.oturum_butcesi(),
        ButceSinirlari {
            istek: 3,
            token: 1000
        }
    );
}

#[tokio::test]
async fn butce_ayari_denetci_kipini_korur() {
    let d = tempfile::tempdir().unwrap();
    let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
    t.uygulama
        .denetci_kipini_ayarla(DenetimKipi::Kati)
        .await
        .unwrap();
    t.uygulama.oturum_butcesini_ayarla(3, 1000).unwrap();
    assert_eq!(t.uygulama.denetci_kipi().kip, DenetimKipi::Kati);
    drop(t);
    let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
    assert_eq!(t.uygulama.denetci_kipi().kip, DenetimKipi::Kati);
}
