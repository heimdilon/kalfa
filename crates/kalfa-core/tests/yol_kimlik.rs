use std::path::PathBuf;

use kalfa_core::kimlik::TurId;
use kalfa_core::yol::{CalismaYolu, YolHatasi};

fn coz(s: &str) -> Result<CalismaYolu, YolHatasi> {
    CalismaYolu::coz(s)
}

#[test]
fn normallesir() {
    assert_eq!(coz("./src//main.rs").unwrap().as_str(), "src/main.rs");
    assert_eq!(coz("src\\alt\\a.txt").unwrap().as_str(), "src/alt/a.txt");
    assert_eq!(coz("a.txt").unwrap().to_string(), "a.txt");
}

#[test]
fn bos_yol() {
    assert_eq!(coz(""), Err(YolHatasi::Bos));
    assert_eq!(coz("./."), Err(YolHatasi::Bos));
}

#[test]
fn mutlak_yollar_reddedilir() {
    for y in [
        "/etc/passwd",
        "C:\\Windows",
        "c:dosya",
        "\\\\sunucu\\pay",
        "//sunucu/pay",
    ] {
        assert!(matches!(coz(y), Err(YolHatasi::Mutlak(_))), "{y}");
    }
}

#[test]
fn ust_dizin_reddedilir() {
    for y in ["../dis", "a/../../b", "a/.."] {
        assert!(matches!(coz(y), Err(YolHatasi::UstDizin(_))), "{y}");
    }
}

#[test]
fn alternatif_veri_akisi_reddedilir() {
    assert!(matches!(
        coz("not.txt:gizli"),
        Err(YolHatasi::AlternatifVeriAkisi(_))
    ));
}

#[test]
fn windows_aygit_adlari_reddedilir() {
    for y in ["CON", "nul.txt", "src/com1.rs", "LPT9", "aux "] {
        assert!(matches!(coz(y), Err(YolHatasi::AygitAdi(_))), "{y}");
    }
    assert!(coz("console.txt").is_ok());
    assert!(coz("CONFIG").is_ok());
}

#[test]
fn nul_karakteri_reddedilir() {
    assert!(matches!(coz("a\0b"), Err(YolHatasi::GecersizKarakter(_))));
}

#[test]
fn anahtar_buyuk_kucuk_harf_duyarsiz() {
    assert_eq!(coz("SRC/Main.rs").unwrap().anahtar(), "src/main.rs");
}

#[test]
fn ust_ve_std_yolu() {
    let y = coz("a/b/c.txt").unwrap();
    assert_eq!(y.ust().unwrap().as_str(), "a/b");
    assert!(coz("c.txt").unwrap().ust().is_none());
    assert_eq!(y.std_yolu(), PathBuf::from("a").join("b").join("c.txt"));
}

#[test]
fn serde_dogrulayarak_cozer() {
    let y: CalismaYolu = serde_json::from_str("\"src/a.rs\"").unwrap();
    assert_eq!(y.as_str(), "src/a.rs");
    assert!(serde_json::from_str::<CalismaYolu>("\"../x\"").is_err());
    assert_eq!(serde_json::to_string(&y).unwrap(), "\"src/a.rs\"");
}

#[test]
fn kimlikler_benzersiz_ve_serilesir() {
    let a = TurId::yeni();
    assert_ne!(a, TurId::yeni());
    let j = serde_json::to_string(&a).unwrap();
    assert_eq!(serde_json::from_str::<TurId>(&j).unwrap(), a);
}
