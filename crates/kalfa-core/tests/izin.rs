use std::process::Command;

use kalfa_core::git_koruma;
use kalfa_core::izin::*;
use kalfa_core::yol::CalismaYolu;

fn y(s: &str) -> CalismaYolu {
    CalismaYolu::coz(s).unwrap()
}

fn onayli(uyarilar: &[&str]) -> Karar {
    Karar::Onayli {
        uyarilar: uyarilar.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn okuma_kararlari() {
    for kip in [Kip::Inceleme, Kip::Siki, Kip::Proje, Kip::Serbest] {
        assert_eq!(
            karar_ver(kip, Islem::Okuma, YolSinifi::Normal, false),
            Karar::Serbest,
            "{kip:?}"
        );
        assert_eq!(
            karar_ver(kip, Islem::Okuma, YolSinifi::Korumali, false),
            Karar::Serbest,
            "{kip:?}"
        );
        assert_eq!(
            karar_ver(kip, Islem::Okuma, YolSinifi::Gizli, false),
            onayli(&[GIZLI_UYARISI]),
            "{kip:?}: gizli dosya her kipte onay ister"
        );
    }
}

#[test]
fn yazma_kararlari() {
    assert!(matches!(
        karar_ver(Kip::Inceleme, Islem::Yazma, YolSinifi::Normal, false),
        Karar::Yasak { .. }
    ));
    assert_eq!(
        karar_ver(Kip::Siki, Islem::Yazma, YolSinifi::Normal, false),
        onayli(&[])
    );
    assert_eq!(
        karar_ver(Kip::Siki, Islem::Yazma, YolSinifi::Korumali, false),
        onayli(&[KORUMALI_UYARISI])
    );
    for kip in [Kip::Proje, Kip::Serbest] {
        assert_eq!(
            karar_ver(kip, Islem::Yazma, YolSinifi::Normal, false),
            Karar::Serbest,
            "{kip:?}"
        );
        assert_eq!(
            karar_ver(kip, Islem::Yazma, YolSinifi::Gizli, false),
            Karar::Serbest,
            "{kip:?}"
        );
        assert_eq!(
            karar_ver(kip, Islem::Yazma, YolSinifi::Korumali, false),
            onayli(&[KORUMALI_UYARISI]),
            "{kip:?}: korumalı yol Serbest kipte de onay ister"
        );
        assert_eq!(
            karar_ver(kip, Islem::Yazma, YolSinifi::Normal, true),
            onayli(&[BAGLANTI_UYARISI])
        );
    }
}

#[test]
fn kararlar_birlesir() {
    assert_eq!(Karar::Serbest.birlestir(Karar::Serbest), Karar::Serbest);
    assert_eq!(Karar::Serbest.birlestir(onayli(&["a"])), onayli(&["a"]));
    assert_eq!(onayli(&["a"]).birlestir(Karar::Serbest), onayli(&["a"]));
    assert_eq!(
        onayli(&["a"]).birlestir(onayli(&["b"])),
        onayli(&["a", "b"])
    );
    assert_eq!(onayli(&["a"]).birlestir(onayli(&["a"])), onayli(&["a"]));
    let yasak = Karar::Yasak { neden: "x".into() };
    assert_eq!(onayli(&["a"]).birlestir(yasak.clone()), yasak);
    assert_eq!(yasak.clone().birlestir(Karar::Serbest), yasak);
}

#[test]
fn gizli_kaliplar() {
    let d = tempfile::tempdir().unwrap();
    let s = Siniflandirici::yeni(d.path(), &["*.sir".to_string()]).unwrap();
    for g in [
        ".env",
        "config/.env.local",
        "certs/sunucu.PEM",
        "id_rsa",
        "home/.ssh/id_ed25519.pub",
        ".npmrc",
        "credentials.toml",
        "a/b/anahtar.key",
        "veri.sir",
    ] {
        assert!(s.gizli_mi(&y(g)), "{g} gizli olmalı");
    }
    for n in [
        "src/main.rs",
        "env.txt",
        "README.md",
        "keys.rs",
        "src/environment.rs",
    ] {
        assert!(!s.gizli_mi(&y(n)), "{n} gizli olmamalı");
    }
}

#[test]
fn git_dizini_korunur() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join(".git/hooks")).unwrap();
    std::fs::write(d.path().join(".git/config"), "").unwrap();
    let s = Siniflandirici::yeni(d.path(), &[]).unwrap();
    for k in [
        ".git/hooks/pre-commit",
        ".GIT/Hooks/post-merge",
        ".git/config",
        ".git/hooks",
    ] {
        assert_eq!(s.sinifla(&y(k)), YolSinifi::Korumali, "{k}");
    }
    for n in [
        ".git/HEAD",
        ".git/objects/ab/cd",
        ".gitignore",
        "src/config",
    ] {
        assert_ne!(s.sinifla(&y(n)), YolSinifi::Korumali, "{n}");
    }
    assert_eq!(s.sinifla(&y(".env")), YolSinifi::Gizli);
}

#[test]
fn gitdir_dosyasi_gercek_dizini_gosterir() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("modul-git/hooks")).unwrap();
    std::fs::write(d.path().join(".git"), "gitdir: modul-git\n").unwrap();
    let s = Siniflandirici::yeni(d.path(), &[]).unwrap();
    assert_eq!(
        s.sinifla(&y("modul-git/hooks/pre-commit")),
        YolSinifi::Korumali
    );
    assert_eq!(s.sinifla(&y("modul-git/config")), YolSinifi::Korumali);

    // Çalışma alanı dışını gösteren gitdir burada korunmaz: orası zaten dışarıda.
    std::fs::write(d.path().join(".git"), "gitdir: ../baska\n").unwrap();
    assert!(
        git_koruma::korumali_yollar(d.path())
            .iter()
            .all(|p| !p.contains("baska"))
    );
}

#[test]
fn core_hookspath_hedefi_korunur() {
    let d = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let durum = Command::new("git")
            .arg("-C")
            .arg(d.path())
            .args(args)
            .status()
            .expect("git kurulu olmalı");
        assert!(durum.success(), "git {args:?}");
    };
    git(&["init", "-q"]);
    git(&["config", "core.hooksPath", ".husky"]);
    let s = Siniflandirici::yeni(d.path(), &[]).unwrap();
    assert_eq!(s.sinifla(&y(".husky/pre-commit")), YolSinifi::Korumali);
    assert_eq!(s.sinifla(&y(".HUSKY/pre-push")), YolSinifi::Korumali);
    assert_eq!(s.sinifla(&y("src/husky.rs")), YolSinifi::Normal);
}
