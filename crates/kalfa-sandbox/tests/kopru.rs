#![cfg(windows)]

use std::process::Command;

#[test]
fn eksik_ortamda_125_ile_cikar() {
    let kopru_yolu = env!("CARGO_BIN_EXE_kalfa-kopru");
    let c = Command::new(kopru_yolu)
        .env_remove("KALFA_KOPRU_UYGULAMA")
        .env_remove("KALFA_KOPRU_SATIR")
        .env_remove("KALFA_KOPRU_BORU")
        .output()
        .expect("kopru calistirilamadi");

    assert_eq!(c.status.code(), Some(125));
    let hata = String::from_utf8_lossy(&c.stderr);
    assert!(hata.contains("[kalfa-kopru]"));
}

#[tokio::test]
async fn kalfa_kopru_degiskenleri_cocuga_gecmez() {
    let kopru_yolu = env!("CARGO_BIN_EXE_kalfa-kopru");
    let c = Command::new(kopru_yolu)
        .env("KALFA_KOPRU_UYGULAMA", r"C:\Windows\System32\cmd.exe")
        .env("KALFA_KOPRU_SATIR", r"cmd.exe /c set")
        .env("KALFA_KOPRU_BORU", r"\\.\pipe\kalfa-test-gecersiz")
        .output()
        .expect("kopru calistirilamadi");

    assert_eq!(c.status.code(), Some(0));
    let cikti = String::from_utf8_lossy(&c.stdout);

    // KALFA_KOPRU_* çocuğa geçmemeli
    assert!(!cikti.contains("KALFA_KOPRU_UYGULAMA"));
    assert!(!cikti.contains("KALFA_KOPRU_SATIR"));
    assert!(!cikti.contains("KALFA_KOPRU_BORU"));

    // Vekil değişkenleri eklenmeli
    assert!(cikti.contains("HTTPS_PROXY="));
    assert!(cikti.contains("GIT_CONFIG_KEY_0="));
}

#[test]
fn cocugun_cikis_kodu_kopruden_doner() {
    let kopru_yolu = env!("CARGO_BIN_EXE_kalfa-kopru");
    let c = Command::new(kopru_yolu)
        .env("KALFA_KOPRU_UYGULAMA", r"C:\Windows\System32\cmd.exe")
        .env("KALFA_KOPRU_SATIR", r"cmd.exe /c exit 42")
        .env("KALFA_KOPRU_BORU", r"\\.\pipe\kalfa-test-gecersiz")
        .output()
        .expect("kopru calistirilamadi");

    assert_eq!(c.status.code(), Some(42));
}

#[tokio::test]
async fn cocuk_ve_kopru_ayni_iste_durur() {
    use std::os::windows::io::AsRawHandle;
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::CREATE_SUSPENDED;

    let is = kalfa_sandbox::windows::Is::yeni(false).unwrap();
    let kopru_yolu = env!("CARGO_BIN_EXE_kalfa-kopru");

    let mut cmd = std::process::Command::new(kopru_yolu);
    cmd.env("KALFA_KOPRU_UYGULAMA", r"C:\Windows\System32\ping.exe")
        .env("KALFA_KOPRU_SATIR", r"ping.exe -n 30 127.0.0.1")
        .env("KALFA_KOPRU_BORU", r"\\.\pipe\kalfa-test-gecersiz")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(CREATE_SUSPENDED);

    let cocuk = cmd.spawn().expect("kopru baslatilamadi");
    let pid = cocuk.id();
    is.ata(cocuk.as_raw_handle()).unwrap();
    kalfa_sandbox::windows::surdur(pid).unwrap();

    // Köprü ve çocuğun başlaması için kısa süre bekle
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let mut cocuk = cocuk;
    is.sonlandir(); // işi sonlandır -> çocuk ve köprü ölmeli

    let sonuc = cocuk.wait().expect("bekleme basarisiz");
    assert!(!sonuc.success());
}
