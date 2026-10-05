//! Denetçi kipi ayarı (aşama 3c, Görev 7): `ayarlar.json` ve komutlar.

use kalfa_core::denetci::DenetimKipi;
use kalfa_core::test_destek::SahteSaglayici;
use kalfa_uygulama::test_destek::test_uygulamasi;

#[test]
fn ayar_dosyasi_yoksa_kapali() {
    let d = tempfile::tempdir().unwrap();
    let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
    let durum = t.uygulama.denetci_kipi();
    assert_eq!(durum.kip, DenetimKipi::Kapali);
    assert_eq!(durum.ayar_hatasi, None);
}

#[test]
fn bozuk_ayar_dosyasi_kapali_ve_bildirilir() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("ayarlar.json"), b"{ bozuk").unwrap();
    let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
    let durum = t.uygulama.denetci_kipi();
    assert_eq!(durum.kip, DenetimKipi::Kapali);
    assert!(
        durum
            .ayar_hatasi
            .as_deref()
            .is_some_and(|h| h.contains("ayarlar.json bozuk")),
        "{durum:?}"
    );
}

#[tokio::test]
async fn kip_yukseltmek_onaysiz_ve_dosyaya_yazilir() {
    let d = tempfile::tempdir().unwrap();
    let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
    t.iletisim.onay_ver(false);
    let durum = t
        .uygulama
        .denetci_kipini_ayarla(DenetimKipi::Kati)
        .await
        .unwrap();
    assert_eq!(durum.kip, DenetimKipi::Kati);
    assert!(t.iletisim.sorulanlar().is_empty());
    let metin = std::fs::read_to_string(d.path().join("ayarlar.json")).unwrap();
    assert!(metin.contains("Kati"), "{metin}");
    // Yeniden açılan uygulama ayarı dosyadan okur.
    drop(t);
    let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
    assert_eq!(t.uygulama.denetci_kipi().kip, DenetimKipi::Kati);
}

#[tokio::test]
async fn kip_dusurmek_yerel_onay_ister_vazgecilince_degismez() {
    let d = tempfile::tempdir().unwrap();
    let t = test_uygulamasi(d.path(), SahteSaglayici::yeni(vec![]), Some("anahtar"));
    t.uygulama
        .denetci_kipini_ayarla(DenetimKipi::Kati)
        .await
        .unwrap();

    t.iletisim.onay_ver(false);
    let durum = t
        .uygulama
        .denetci_kipini_ayarla(DenetimKipi::Uyar)
        .await
        .unwrap();
    assert_eq!(durum.kip, DenetimKipi::Kati, "vazgeçilince kip değişmez");
    assert_eq!(t.iletisim.sorulanlar().len(), 1);

    t.iletisim.onay_ver(true);
    let durum = t
        .uygulama
        .denetci_kipini_ayarla(DenetimKipi::Kapali)
        .await
        .unwrap();
    assert_eq!(durum.kip, DenetimKipi::Kapali);
    assert_eq!(t.iletisim.sorulanlar().len(), 2);
}
