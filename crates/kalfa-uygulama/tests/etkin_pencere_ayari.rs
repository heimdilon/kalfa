//! Etkin bağlam penceresi ayarı: `ayarlar.json`'daki varsayılan, eski
//! `oturum_butcesi` alanının yok sayılması, alt/üst sınır doğrulaması ve yeni
//! oturumlara aktarılması.

use kalfa_core::denetci::DenetimKipi;
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::SahteSaglayici;
use kalfa_uygulama::test_destek::test_uygulamasi;

const VARSAYILAN: u64 = 200_000;
const ASGARI: u64 = 32_000;
const AZAMI: u64 = 9_007_199_254_740_991;

#[test]
fn ayar_dosyasi_yoksa_varsayilan_etkin_pencere() {
    let d = tempfile::tempdir().unwrap();
    let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
    assert_eq!(t.uygulama.etkin_pencere(), VARSAYILAN);
}

#[tokio::test]
async fn eski_oturum_butcesi_alani_hatasiz_okunur() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(
        d.path().join("ayarlar.json"),
        r#"{"denetci_kipi":"Uyar","oturum_butcesi":{"istek":200,"token":5000000}}"#,
    )
    .unwrap();
    let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
    assert_eq!(t.uygulama.etkin_pencere(), VARSAYILAN);
    assert_eq!(t.uygulama.denetci_kipi().kip, DenetimKipi::Uyar);
    assert_eq!(t.uygulama.denetci_kipi().ayar_hatasi, None);

    t.uygulama
        .denetci_kipini_ayarla(DenetimKipi::Kati)
        .await
        .unwrap();
    let yazilan = std::fs::read_to_string(d.path().join("ayarlar.json")).unwrap();
    assert!(
        !yazilan.contains("oturum_butcesi"),
        "yeniden yazılan dosyada oturum_butcesi geçmemeli: {yazilan}"
    );
}

#[test]
fn etkin_pencere_alt_sinirin_altinda_reddedilir() {
    let d = tempfile::tempdir().unwrap();
    let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
    assert!(t.uygulama.etkin_pencereyi_ayarla(31_999).is_err());
    assert_eq!(t.uygulama.etkin_pencere(), VARSAYILAN);

    assert_eq!(t.uygulama.etkin_pencereyi_ayarla(ASGARI).unwrap(), ASGARI);
    assert_eq!(t.uygulama.etkin_pencere(), ASGARI);
    drop(t);

    let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
    assert_eq!(t.uygulama.etkin_pencere(), ASGARI);
}

#[tokio::test]
async fn yeni_oturum_etkin_pencereyi_ayardan_alir() {
    let d = tempfile::tempdir().unwrap();
    let alan_d = tempfile::tempdir().unwrap();
    let saglayici = SahteSaglayici::yeni(vec![]);
    saglayici.sart_kabul(1).await.unwrap();
    let t = test_uygulamasi(d.path(), saglayici, Some("evren_llm_a"));
    t.uygulama.baslat().await;
    t.uygulama.etkin_pencereyi_ayarla(50_000).unwrap();
    t.iletisim.klasor_ver(alan_d.path());
    let alan = t.uygulama.calisma_alani_sec().await.unwrap();
    let acilis = t
        .uygulama
        .oturum_baslat(alan.kimlik, "glm-5.3".into())
        .await
        .unwrap();
    let sinirlar = t
        .uygulama
        .oturum_sinirlari(acilis.oturum)
        .expect("açık oturumun sınırları");
    assert_eq!(sinirlar.etkin_pencere, 50_000);
}

#[test]
fn bozuk_etkin_pencere_degeri_yalnizca_kendisini_dusurur() {
    for metin in [
        r#"{"denetci_kipi":"Uyar","etkin_pencere":"yuzbin"}"#,
        r#"{"denetci_kipi":"Uyar","etkin_pencere":-1}"#,
        r#"{"denetci_kipi":"Uyar","etkin_pencere":0}"#,
        r#"{"denetci_kipi":"Uyar","etkin_pencere":31999}"#,
        r#"{"denetci_kipi":"Uyar","etkin_pencere":null}"#,
    ] {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("ayarlar.json"), metin).unwrap();
        let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
        assert_eq!(t.uygulama.etkin_pencere(), VARSAYILAN, "{metin}");
        let durum = t.uygulama.denetci_kipi();
        assert_eq!(durum.kip, DenetimKipi::Uyar, "{metin}");
        assert_eq!(durum.ayar_hatasi, None, "{metin}");
    }
}

#[test]
fn sinirin_en_ustu_kabul_edilir_ustu_reddedilir() {
    let d = tempfile::tempdir().unwrap();
    let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
    assert_eq!(t.uygulama.etkin_pencereyi_ayarla(AZAMI).unwrap(), AZAMI);
    assert!(t.uygulama.etkin_pencereyi_ayarla(AZAMI + 1).is_err());
    assert_eq!(t.uygulama.etkin_pencere(), AZAMI);
}
