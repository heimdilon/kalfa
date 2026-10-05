use std::process::Command;

use kalfa_core::izin::{Islem, Karar, Kip, Siniflandirici, YolSinifi, karar_ver};
use kalfa_core::yol::CalismaYolu;

fn depo(kancalar: &str) -> tempfile::TempDir {
    let depo = tempfile::tempdir().unwrap();
    for argumanlar in [
        vec!["init", "-q"],
        vec!["config", "core.hooksPath", kancalar],
    ] {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(depo.path())
                .args(argumanlar)
                .status()
                .unwrap()
                .success()
        );
    }
    depo
}

fn korumayi_sina(depo: &std::path::Path, hedef: &str) {
    let siniflandirici = Siniflandirici::yeni(depo, &[]).unwrap();
    let yol = CalismaYolu::coz(hedef).unwrap();
    assert_eq!(siniflandirici.sinifla(&yol), YolSinifi::Korumali);
    assert_eq!(siniflandirici.toplu().sinifla(&yol), YolSinifi::Korumali);
    for kip in [Kip::Proje, Kip::Serbest] {
        assert!(matches!(
            karar_ver(kip, Islem::Yazma, siniflandirici.sinifla(&yol), false),
            Karar::Onayli { .. }
        ));
    }
}

#[test]
fn ust_bilesenli_kanca_yolu_onay_ister() {
    let depo = depo("build/../hooks");
    std::fs::create_dir(depo.path().join("build")).unwrap();
    std::fs::create_dir(depo.path().join("hooks")).unwrap();
    korumayi_sina(depo.path(), "hooks/pre-commit");
}

#[test]
fn henuz_olmayan_kanca_dizini_korunur() {
    let depo = depo(".husky");
    korumayi_sina(depo.path(), ".husky/pre-commit");
}

#[test]
fn kanca_dizinindeki_bosluklar_korunur() {
    let depo = depo(" hooks");
    std::fs::create_dir(depo.path().join(" hooks")).unwrap();
    korumayi_sina(depo.path(), " hooks/pre-commit");
    let s = Siniflandirici::yeni(depo.path(), &[]).unwrap();
    assert_eq!(
        s.sinifla(&CalismaYolu::coz("hooks/pre-commit").unwrap()),
        YolSinifi::Normal
    );
}

#[test]
fn kokteki_kancalar_butun_yazmalari_korur() {
    let depo = depo(".");
    korumayi_sina(depo.path(), "pre-commit");
    korumayi_sina(depo.path(), "src/normal.rs");
}

#[cfg(unix)]
#[test]
fn henuz_olmayan_baglanti_hedefi_korunur() {
    let depo = depo("takma");
    std::fs::create_dir(depo.path().join("gercek")).unwrap();
    std::os::unix::fs::symlink("gercek/hooks", depo.path().join("takma")).unwrap();
    korumayi_sina(depo.path(), "gercek/hooks/pre-commit");
}

#[cfg(unix)]
#[test]
fn kanca_baglantisinin_gercek_hedefi_korunur() {
    let depo = depo("takma/../hooks");
    std::fs::create_dir_all(depo.path().join("gercek/alt")).unwrap();
    std::fs::create_dir(depo.path().join("gercek/hooks")).unwrap();
    std::os::unix::fs::symlink("gercek/alt", depo.path().join("takma")).unwrap();
    korumayi_sina(depo.path(), "gercek/hooks/pre-commit");
}
