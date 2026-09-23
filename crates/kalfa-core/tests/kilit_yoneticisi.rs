use std::sync::Arc;

use kalfa_core::kilit::{KaynakKilitleri, KilitYoneticisi};
use kalfa_core::yol::CalismaYolu;

#[test]
fn ayni_kok_ayni_kilitleri_paylasir() {
    let d = tempfile::tempdir().unwrap();
    let y = KilitYoneticisi::yeni();
    let a = y.al(d.path()).unwrap();
    let b = y.al(&d.path().join(".")).unwrap();
    assert!(Arc::ptr_eq(&a, &b));
}

#[test]
fn farkli_kokler_farkli_kilitler() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir(d.path().join("a")).unwrap();
    std::fs::create_dir(d.path().join("b")).unwrap();
    let y = KilitYoneticisi::yeni();
    let a = y.al(&d.path().join("a")).unwrap();
    let b = y.al(&d.path().join("b")).unwrap();
    assert!(!Arc::ptr_eq(&a, &b));
}

#[cfg(windows)]
#[test]
fn buyuk_kucuk_harf_farki_ayni_koku_verir() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir(d.path().join("Proje")).unwrap();
    let y = KilitYoneticisi::yeni();
    let a = y.al(&d.path().join("Proje")).unwrap();
    let b = y.al(&d.path().join("PROJE")).unwrap();
    assert!(Arc::ptr_eq(&a, &b));
}

#[cfg(unix)]
#[test]
fn sembolik_baglanti_ayni_koku_verir() {
    let d = tempfile::tempdir().unwrap();
    let hedef = d.path().join("proje");
    std::fs::create_dir(&hedef).unwrap();
    let bag = d.path().join("bag");
    std::os::unix::fs::symlink(&hedef, &bag).unwrap();
    let y = KilitYoneticisi::yeni();
    assert!(Arc::ptr_eq(&y.al(&hedef).unwrap(), &y.al(&bag).unwrap()));
}

#[cfg(windows)]
#[test]
fn junction_ayni_koku_verir() {
    let d = tempfile::tempdir().unwrap();
    let hedef = d.path().join("proje");
    std::fs::create_dir(&hedef).unwrap();
    let bag = d.path().join("bag");
    // Yalnızca bu testin geçici dizinindeki iki yol; kullanıcı girdisi yok.
    let cikti = std::process::Command::new("cmd.exe")
        .args(["/D", "/C", "mklink", "/J"])
        .arg(&bag)
        .arg(&hedef)
        .output()
        .unwrap();
    assert!(cikti.status.success(), "junction kurulamadı: {cikti:?}");
    let y = KilitYoneticisi::yeni();
    assert!(Arc::ptr_eq(&y.al(&hedef).unwrap(), &y.al(&bag).unwrap()));
}

#[test]
fn son_oturum_birakinca_kok_kaydi_ve_handle_temizlenir() {
    let d = tempfile::tempdir().unwrap();
    let y = KilitYoneticisi::yeni();
    let a = y.al(d.path()).unwrap();
    assert_eq!(y.acik_kok_sayisi(), 1);
    drop(a);
    d.close().expect("dizin handle bırakılınca silinebilir");
    assert_eq!(y.acik_kok_sayisi(), 0);
}

#[tokio::test]
async fn yol_tablosu_buyumez() {
    let k = KaynakKilitleri::yeni();
    for i in 0..100 {
        let yol = CalismaYolu::coz(&format!("d{i}.txt")).unwrap();
        let _set = k.al(&[], &[yol], false).await;
    }
    assert!(k.yol_tablosu_boyutu() <= 1);
}
