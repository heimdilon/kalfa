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

    // Yalıtımın yerleşiminde de mutlak ve gerçek yazımıyla bulunur.
    let kok = d.path().canonicalize().unwrap();
    let yerlesim = git_koruma::git_yerlesimi(d.path());
    assert!(yerlesim.korumali.contains(&git_koruma::KorumaliYol {
        yol: kok.join(".husky"),
        dizin: true,
    }));
}

#[test]
fn git_yerlesimi_depoda_git_dizininin_tamamini_korur() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join(".git/hooks")).unwrap();
    std::fs::write(d.path().join(".git/config"), "").unwrap();
    let kok = d.path().canonicalize().unwrap();
    let yerlesim = git_koruma::git_yerlesimi(d.path());
    assert_eq!(
        yerlesim.korumali,
        vec![git_koruma::KorumaliYol {
            yol: kok.join(".git"),
            dizin: true,
        }]
    );
    assert!(yerlesim.dis_dizinler.is_empty());

    // Depo olmayan dizinde yerleşim boştur.
    let bos = tempfile::tempdir().unwrap();
    assert_eq!(
        git_koruma::git_yerlesimi(bos.path()),
        git_koruma::GitYerlesimi::default()
    );
}

#[test]
fn git_yerlesimi_worktreede_ortak_git_dizinini_bulur() {
    let d = tempfile::tempdir().unwrap();
    let ana = d.path().join("ana/.git");
    std::fs::create_dir_all(ana.join("hooks")).unwrap();
    std::fs::create_dir_all(ana.join("worktrees/dal")).unwrap();
    std::fs::write(ana.join("config"), "").unwrap();
    std::fs::write(ana.join("worktrees/dal/commondir"), "../..\n").unwrap();
    std::fs::write(ana.join("worktrees/dal/config.worktree"), "").unwrap();
    let alan = d.path().join("dal");
    std::fs::create_dir_all(&alan).unwrap();
    std::fs::write(alan.join(".git"), "gitdir: ../ana/.git/worktrees/dal\n").unwrap();

    std::fs::write(
        ana.join("worktrees/dal/gitdir"),
        format!("{}\n", alan.join(".git").display()),
    )
    .unwrap();

    let ana = ana.canonicalize().unwrap();
    let alan_k = alan.canonicalize().unwrap();
    let yerlesim = git_koruma::git_yerlesimi(&alan);
    assert_eq!(yerlesim.dis_dizinler, vec![ana.clone()]);
    assert_eq!(
        yerlesim.korumali,
        vec![git_koruma::KorumaliYol {
            yol: alan_k.join(".git"),
            dizin: false,
        }]
    );
}

/// `.git` dosyası ve `commondir` çalışma alanındadır: yalıtımdaki komut
/// yazabilir. Geri başvurusu tutmayan dış dizin açılmaz.
#[test]
fn git_yerlesimi_dogrulanmayan_dis_dizini_acmaz() {
    let d = tempfile::tempdir().unwrap();
    let ev = d.path().join("ev");
    std::fs::create_dir_all(ev.join(".ssh")).unwrap();

    // `.git` dosyası ev dizinini gösteriyor.
    let alan = d.path().join("alan");
    std::fs::create_dir_all(&alan).unwrap();
    std::fs::write(alan.join(".git"), format!("gitdir: {}\n", ev.display())).unwrap();
    let yerlesim = git_koruma::git_yerlesimi(&alan);
    assert!(yerlesim.dis_dizinler.is_empty(), "{yerlesim:?}");
    assert_eq!(yerlesim.korumali.len(), 1);

    // Sahte worktree: ortak dizinin `worktrees/` altında ama `gitdir` dosyası
    // yok.
    std::fs::create_dir_all(ev.join("worktrees/x")).unwrap();
    std::fs::write(ev.join("worktrees/x/commondir"), "../..\n").unwrap();
    std::fs::write(
        alan.join(".git"),
        format!("gitdir: {}\n", ev.join("worktrees/x").display()),
    )
    .unwrap();
    assert!(git_koruma::git_yerlesimi(&alan).dis_dizinler.is_empty());

    // `.git` dizinindeki `commondir` dışarıyı gösteriyor.
    let depo = d.path().join("depo");
    std::fs::create_dir_all(depo.join(".git/hooks")).unwrap();
    std::fs::write(depo.join(".git/commondir"), format!("{}\n", ev.display())).unwrap();
    let yerlesim = git_koruma::git_yerlesimi(&depo);
    assert!(yerlesim.dis_dizinler.is_empty(), "{yerlesim:?}");
    assert_eq!(
        yerlesim.korumali,
        vec![git_koruma::KorumaliYol {
            yol: depo.canonicalize().unwrap().join(".git"),
            dizin: true,
        }]
    );
}

#[test]
fn git_yerlesimi_alt_modulun_git_dizinini_dogrular() {
    let d = tempfile::tempdir().unwrap();
    let ust = d.path().join("ust");
    let modul = ust.join(".git/modules/alt");
    std::fs::create_dir_all(&modul).unwrap();
    std::fs::write(modul.join("config"), "[core]\n\tworktree = ../../../alt\n").unwrap();
    let alan = ust.join("alt");
    std::fs::create_dir_all(&alan).unwrap();
    std::fs::write(alan.join(".git"), "gitdir: ../.git/modules/alt\n").unwrap();
    // Çalışma alanı alt modülün kendisi; git dizini onun dışında.
    let yerlesim = git_koruma::git_yerlesimi(&alan);
    assert_eq!(yerlesim.dis_dizinler, vec![modul.canonicalize().unwrap()]);

    // `core.worktree` başka yeri gösterirse açılmaz.
    std::fs::write(modul.join("config"), "[core]\n\tworktree = ../../..\n").unwrap();
    assert!(git_koruma::git_yerlesimi(&alan).dis_dizinler.is_empty());
}

fn kabuk(yalitimli: bool, ag: bool, ilk_cagri: bool) -> kalfa_core::kabuk::KabukTarifi {
    use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
    KabukTarifi {
        komut: "cargo test".into(),
        bicim: KabukBicimi::Program,
        dizin: ".".into(),
        ag,
        yalitim: if yalitimli {
            YalitimOzeti::Yalitimli {
                arka_uc: "bwrap".into(),
                profil: "p".into(),
            }
        } else {
            YalitimOzeti::Yalitimsiz {
                talimat: "yalıtım yok".into(),
            }
        },
        ilk_cagri,
    }
}

/// Aşama 2 tasarımı §5.1'in tablosu.
#[test]
fn kabuk_kararlari() {
    for kip in [Kip::Siki, Kip::Proje, Kip::Serbest] {
        for ag in [false, true] {
            assert_eq!(
                kabuk_karari(kip, &kabuk(false, ag, false)),
                Karar::YerelOnayli {
                    uyarilar: vec![YALITIMSIZ_UYARISI.into(), "yalıtım yok".into()]
                },
                "yalıtımsız çağrı her kipte yerel onay ister ({kip:?}, ağ: {ag})"
            );
        }
        let beklenen = if kip == Kip::Serbest {
            Karar::Serbest
        } else {
            onayli(&[])
        };
        assert_eq!(
            kabuk_karari(kip, &kabuk(true, false, false)),
            beklenen,
            "{kip:?}"
        );
        assert_eq!(
            kabuk_karari(kip, &kabuk(true, true, false)),
            onayli(&[AGLI_KABUK_UYARISI]),
            "ağlı çağrı her kipte onay ister ({kip:?})"
        );
        assert_eq!(
            kabuk_karari(kip, &kabuk(true, false, true)),
            onayli(&[ILK_KABUK_UYARISI]),
            "çalışma alanındaki ilk çağrı Serbest kipte de onay ister ({kip:?})"
        );
    }
    assert!(matches!(
        kabuk_karari(Kip::Inceleme, &kabuk(true, false, false)),
        Karar::Yasak { .. }
    ));
    assert_eq!(
        kabuk_karari(Kip::Serbest, &kabuk(false, false, true)),
        Karar::YerelOnayli {
            uyarilar: vec![
                ILK_KABUK_UYARISI.into(),
                YALITIMSIZ_UYARISI.into(),
                "yalıtım yok".into()
            ]
        }
    );
}

#[test]
fn yerel_onay_birlesmede_onayliyi_ezer() {
    let yerel = |u: &[&str]| Karar::YerelOnayli {
        uyarilar: u.iter().map(|s| s.to_string()).collect(),
    };
    assert_eq!(onayli(&["a"]).birlestir(yerel(&["b"])), yerel(&["a", "b"]));
    assert_eq!(yerel(&["b"]).birlestir(onayli(&["a"])), yerel(&["b", "a"]));
    assert_eq!(yerel(&["a"]).birlestir(Karar::Serbest), yerel(&["a"]));
    assert_eq!(Karar::Serbest.birlestir(yerel(&["a"])), yerel(&["a"]));
    let yasak = Karar::Yasak { neden: "x".into() };
    assert_eq!(yerel(&["a"]).birlestir(yasak.clone()), yasak);
}
