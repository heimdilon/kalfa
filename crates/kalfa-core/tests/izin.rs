use std::process::Command;

use kalfa_core::git_koruma;
use kalfa_core::izin::*;
use kalfa_core::yol::CalismaYolu;

fn y(s: &str) -> CalismaYolu {
    CalismaYolu::coz(s).unwrap()
}

#[test]
fn gizli_dizinin_alt_yollari_ayni_okuma_sinirindadir() {
    let d = tempfile::tempdir().unwrap();
    let s = Siniflandirici::yeni(d.path(), &["ozel/kasa".into()]).unwrap();
    for yol in [
        "credentials/token.txt",
        "a/CREDENTIALS-prod/ic/token.txt",
        ".env/ic/normal.txt",
        "ozel/kasa/ic/normal.txt",
    ] {
        assert!(s.gizli_mi(&y(yol)), "{yol}");
    }
    for yol in [
        "normal/token.txt",
        "ozel/kasaba/token.txt",
        "src/secrets.rs",
        "node_modules/credentials/token.txt",
        "credentials/node_modules/token.txt",
    ] {
        assert!(!s.gizli_mi(&y(yol)), "{yol}");
    }
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

/// Güvenlik analizi Bulgu 4: yaygın sır dosyaları gizlidir.
#[test]
fn genis_gizli_liste() {
    let d = tempfile::tempdir().unwrap();
    let s = Siniflandirici::yeni(d.path(), &[]).unwrap();
    for g in [
        "config/prod.env",
        "infra/terraform.tfstate",
        "infra/terraform.tfstate.backup",
        "prod.tfvars",
        ".terraformrc",
        "terraform.rc",
        "config/secrets.yaml",
        "secret.json",
        "gcp/service-account-prod.json",
        "serviceaccount.json",
        ".pgpass",
        "web/.htpasswd",
        ".vault-token",
        ".s3cfg",
        ".boto",
        ".docker/config.json",
        ".dockercfg",
        "kubeconfig",
        ".kube/config",
        ".yarnrc.yml",
        "keys/app.jks",
        "vpn/ofis.ovpn",
        "putty.ppk",
        "krb5.keytab",
        "id_ecdsa",
        "id_dsa.pub",
    ] {
        assert!(s.gizli_mi(&y(g)), "{g} gizli olmalı");
    }
}

#[test]
fn gizli_olmayanlar() {
    let d = tempfile::tempdir().unwrap();
    let s = Siniflandirici::yeni(d.path(), &[]).unwrap();
    for n in [
        "src/secrets.rs",
        "src/env.rs",
        "environment.ts",
        "secrets_test.go",
        "docs/kubeconfig.md",
        ".docker/daemon.json",
        "src/service_account.rs",
    ] {
        assert!(!s.gizli_mi(&y(n)), "{n} gizli olmamalı");
    }
}

/// Güvenlik analizi Bulgu 2: kendiliğinden çalışan yapılandırmaya yazmak her
/// kipte onay ister; İnceleme'de yasaktır. Okuması serbesttir.
#[test]
fn yapilandirma_yazmasi_her_kipte_onay_ister() {
    let d = tempfile::tempdir().unwrap();
    let s = Siniflandirici::yeni(d.path(), &[]).unwrap();
    for k in [
        ".vscode/tasks.json",
        ".vscode/settings.json",
        ".idea/workspace.xml",
        ".devcontainer/devcontainer.json",
        ".cargo/config",
        ".cargo/config.toml",
        "rust-toolchain",
        "rust-toolchain.toml",
        "mise.toml",
        ".mise.toml",
        ".envrc",
        ".github/workflows/ci.yml",
        ".gitlab-ci.yml",
        ".pre-commit-config.yaml",
        ".husky/pre-commit",
        "lefthook.yml",
        ".lefthook.yml",
        ".yarnrc",
        ".pnpmfile.cjs",
        "alt/.envrc",
        "alt/.cargo/config.toml",
        ".VSCode/Tasks.json",
    ] {
        let sinif = s.sinifla(&y(k));
        assert_eq!(sinif, YolSinifi::Yapilandirma, "{k}");
        for kip in [Kip::Siki, Kip::Proje, Kip::Serbest] {
            assert_eq!(
                karar_ver(kip, Islem::Yazma, sinif, false),
                onayli(&[YAPILANDIRMA_UYARISI]),
                "{kip:?} {k}"
            );
        }
        assert!(matches!(
            karar_ver(Kip::Inceleme, Islem::Yazma, sinif, false),
            Karar::Yasak { .. }
        ));
        assert_eq!(
            karar_ver(Kip::Serbest, Islem::Okuma, sinif, false),
            Karar::Serbest,
            "{k}"
        );
    }
}

#[test]
fn yapilandirma_olmayanlar_normal() {
    let d = tempfile::tempdir().unwrap();
    let s = Siniflandirici::yeni(d.path(), &[]).unwrap();
    for n in [
        "src/vscode.rs",
        ".github/README.md",
        "docs/.idea.md",
        ".cargo/credentials.md",
        "src/envrc.rs",
    ] {
        assert!(!s.yapilandirma_mi(&y(n)), "{n}");
    }
    assert_eq!(s.sinifla(&y("src/vscode.rs")), YolSinifi::Normal);
}

/// Hem gizli hem yapılandırma olan dosya: yazması yapılandırma uyarısıyla,
/// okuması gizli dosya uyarısıyla onay ister.
#[test]
fn gizli_ve_yapilandirma_olan_dosyanin_okumasi_onay_ister() {
    let d = tempfile::tempdir().unwrap();
    let s = Siniflandirici::yeni(d.path(), &[]).unwrap();
    let yol = y(".vscode/.env");
    assert_eq!(s.sinifla(&yol), YolSinifi::Yapilandirma);
    assert!(s.gizli_mi(&yol));
}

/// Araçların ürettiği bağımlılık dizinlerinde gizli kalıplar uygulanmaz
/// (2d-3, kullanıcı kararı): pip'in `cacert.pem`'i ve npm paketlerinin
/// `credentials.js`'i kitaplık dosyasıdır.
#[test]
fn bagimlilik_dizininde_gizli_sayilmaz() {
    let d = tempfile::tempdir().unwrap();
    let s = Siniflandirici::yeni(d.path(), &["*.sir".to_string()]).unwrap();
    for n in [
        "node_modules/aws-sdk/lib/credentials.js",
        "Node_Modules/pkg/.env",
        "web/node_modules/pkg/sunucu.pem",
        ".venv/Lib/site-packages/pip/_vendor/certifi/cacert.pem",
        "venv/lib/python3.14/site-packages/certifi/cacert.pem",
        "vendor/dist-packages/x/anahtar.key",
        "node_modules/pkg/veri.sir",
    ] {
        assert!(!s.gizli_mi(&y(n)), "{n} gizli olmamalı");
    }
    for g in [
        ".env",
        "node_modules_degil/.env",
        "src/site-packages.pem",
        "paket/node_modulesx/credentials.json",
    ] {
        assert!(s.gizli_mi(&y(g)), "{g} gizli olmalı");
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
        ".git/HEAD",
        ".git/objects/ab/cd",
    ] {
        assert_eq!(s.sinifla(&y(k)), YolSinifi::Korumali, "{k}");
    }
    for n in [".gitignore", "src/config"] {
        assert_ne!(s.sinifla(&y(n)), YolSinifi::Korumali, "{n}");
    }
    assert_eq!(s.sinifla(&y(".env")), YolSinifi::Gizli);
}

/// Güvenlik analizi Bulgu 1: `.git`'in hiçbir yeri dosya araçlarıyla onaysız
/// yazılamaz (`commondir` git'i saldırganın yapılandırmasına yönlendirir).
#[test]
fn git_ic_yollari_her_kipte_onay_ister() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join(".git/hooks")).unwrap();
    let s = Siniflandirici::yeni(d.path(), &[]).unwrap();
    for k in [
        ".git/commondir",
        ".git/modules/x/config",
        ".git/modules/x/hooks/post-checkout",
        ".git/worktrees/w/commondir",
        ".git/info/attributes",
        ".git/HEAD",
        ".git",
        "alt/.git/config",
        "vendor/x/.git/commondir",
        ".GIT/commondir",
        ".git./commondir",
        ".git /config",
        "GIT~1/commondir",
    ] {
        let sinif = s.sinifla(&y(k));
        assert_eq!(sinif, YolSinifi::Korumali, "{k}");
        for kip in [Kip::Siki, Kip::Proje, Kip::Serbest] {
            assert_eq!(
                karar_ver(kip, Islem::Yazma, sinif, false),
                onayli(&[KORUMALI_UYARISI]),
                "{kip:?} {k}"
            );
        }
        assert!(matches!(
            karar_ver(Kip::Inceleme, Islem::Yazma, sinif, false),
            Karar::Yasak { .. }
        ));
    }
}

#[test]
fn depo_olmayan_alanda_git_olusturmak_onay_ister() {
    let d = tempfile::tempdir().unwrap();
    let s = Siniflandirici::yeni(d.path(), &[]).unwrap();
    let sinif = s.sinifla(&y(".git/config"));
    assert_eq!(sinif, YolSinifi::Korumali);
    assert_eq!(
        karar_ver(Kip::Proje, Islem::Yazma, sinif, false),
        onayli(&[KORUMALI_UYARISI])
    );
}

#[test]
fn git_benzeri_ama_git_olmayanlar_normal() {
    let d = tempfile::tempdir().unwrap();
    let s = Siniflandirici::yeni(d.path(), &[]).unwrap();
    for n in [
        ".github/ci.yml",
        ".gitignore",
        ".gitmodules",
        "a.git/x",
        "git~x/y",
        "gitx/y",
        "git~/y",
    ] {
        assert_eq!(s.sinifla(&y(n)), YolSinifi::Normal, "{n}");
    }
}

#[test]
fn git_bileseni_kurali() {
    for e in [
        ".git", ".GIT", ".git.", ".git ", ".git. .", "git~1", "GIT~12",
    ] {
        assert!(git_koruma::git_bileseni_mi(e), "{e}");
    }
    for h in [
        ".github", "git", ".gitx", "git~", "git~a", "x.git", ".git~1",
    ] {
        assert!(!git_koruma::git_bileseni_mi(h), "{h}");
    }
}

#[test]
fn worktree_gitdir_calisma_alani_icindeyse_tamami_korumali() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("ic/gd")).unwrap();
    std::fs::write(d.path().join(".git"), "gitdir: ic/gd\n").unwrap();
    let s = Siniflandirici::yeni(d.path(), &[]).unwrap();
    for k in ["ic/gd/commondir", "ic/gd/HEAD", "ic/gd"] {
        assert_eq!(s.sinifla(&y(k)), YolSinifi::Korumali, "{k}");
    }
    assert_eq!(s.sinifla(&y("ic/baska")), YolSinifi::Normal);
}

#[test]
fn commondir_hedefi_calisma_alani_icindeyse_korumali() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join(".git")).unwrap();
    std::fs::create_dir_all(d.path().join("zz")).unwrap();
    std::fs::write(d.path().join(".git/commondir"), "../zz\n").unwrap();
    let s = Siniflandirici::yeni(d.path(), &[]).unwrap();
    assert_eq!(s.sinifla(&y("zz/config")), YolSinifi::Korumali);
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

    // Toplu sınıflandırıcı korumaları bir kez çözer; sonucu aynıdır.
    let toplu = s.toplu();
    for yol in [
        ".husky/pre-commit",
        ".git/config",
        "alt/.GIT/hooks/x",
        ".envrc",
        ".env",
        "src/husky.rs",
    ] {
        assert_eq!(toplu.sinifla(&y(yol)), s.sinifla(&y(yol)), "{yol}");
    }

    // Yalıtımın yerleşiminde de mutlak ve gerçek yazımıyla bulunur.
    let kok = d.path().canonicalize().unwrap();
    let yerlesim = git_koruma::git_yerlesimi(d.path());
    assert!(yerlesim.korumali.contains(&git_koruma::KorumaliYol {
        yol: kok.join(".husky"),
        dizin: true,
    }));
}

/// Korumaları bir kez çözen sınıflandırıcı (engelleyen ve engellemeyen hâli)
/// yol başına çözenle aynı sınıfı verir.
#[tokio::test]
async fn toplu_siniflandirici_tekliyle_ayni_sinifi_verir() {
    let d = tempfile::tempdir().unwrap();
    for args in [
        &["init", "-q"][..],
        &["config", "core.hooksPath", ".husky"][..],
    ] {
        let durum = Command::new("git")
            .arg("-C")
            .arg(d.path())
            .args(args)
            .status()
            .expect("git kurulu olmalı");
        assert!(durum.success(), "git {args:?}");
    }
    let s = Siniflandirici::yeni(d.path(), &[]).unwrap();
    let toplu = s.toplu();
    let engellemeden = s.toplu_engellemeden().await;
    for yol in [
        ".husky/pre-commit",
        ".HUSKY/pre-push",
        ".git/config",
        "alt/.git/hooks/pre-commit",
        ".vscode/settings.json",
        ".env",
        "node_modules/paket/.env",
        "src/husky.rs",
    ] {
        let yol = y(yol);
        assert_eq!(toplu.sinifla(&yol), s.sinifla(&yol), "{yol}");
        assert_eq!(engellemeden.sinifla(&yol), s.sinifla(&yol), "{yol}");
        assert_eq!(toplu.gizli_mi(&yol), s.gizli_mi(&yol), "{yol}");
        assert_eq!(engellemeden.gizli_mi(&yol), s.gizli_mi(&yol), "{yol}");
    }
    assert_eq!(toplu.sinifla(&y(".husky/pre-commit")), YolSinifi::Korumali);
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
        program: None,
        dizin: ".".into(),
        ag,
        ag_hostlari: Vec::new(),
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
        alan_hazirligi: false,
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
    // Windows'ta çalışma alanının hazırlığı her kipte yerel onay ister.
    for kip in [Kip::Siki, Kip::Proje, Kip::Serbest] {
        let tarif = kalfa_core::kabuk::KabukTarifi {
            alan_hazirligi: true,
            ..kabuk(true, false, true)
        };
        assert_eq!(
            kabuk_karari(kip, &tarif),
            Karar::YerelOnayli {
                uyarilar: vec![ILK_KABUK_UYARISI.into(), ALAN_HAZIRLIGI_UYARISI.into()]
            },
            "{kip:?}"
        );
    }
    assert!(matches!(
        kabuk_karari(
            Kip::Inceleme,
            &kalfa_core::kabuk::KabukTarifi {
                alan_hazirligi: true,
                ..kabuk(true, false, false)
            }
        ),
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

#[test]
fn eski_kayittaki_tarif_ag_hostlari_olmadan_acilir() {
    let json_str = r#"{
        "komut": "cargo test",
        "bicim": {"tip": "Program"},
        "dizin": ".",
        "ag": true,
        "yalitim": {"tip": "Yalitimli", "arka_uc": "bwrap", "profil": "p"},
        "ilk_cagri": false,
        "alan_hazirligi": false
    }"#;
    let tarif: kalfa_core::kabuk::KabukTarifi = serde_json::from_str(json_str).unwrap();
    assert!(tarif.ag_hostlari.is_empty());
}

#[test]
fn kartta_her_ek_host_ayri_satir() {
    let tarif = kalfa_core::kabuk::KabukTarifi {
        komut: "curl https://github.com".into(),
        bicim: kalfa_core::kabuk::KabukBicimi::Program,
        program: None,
        dizin: ".".into(),
        ag: true,
        ag_hostlari: vec![
            "github.com:443".into(),
            "raw.githubusercontent.com:443".into(),
        ],
        yalitim: kalfa_core::kabuk::YalitimOzeti::Yalitimli {
            arka_uc: "bwrap".into(),
            profil: "p1".into(),
        },
        ilk_cagri: false,
        alan_hazirligi: false,
    };
    let kullanici_hostlari = [("internal.registry.corp:443", true)];
    let satirlar = kart_ag_satirlari(&tarif, &kullanici_hostlari);

    assert_eq!(
        satirlar[0],
        "Ağ: açık; paket kayıtları (crates.io, npm, PyPI), GitHub indirmeleri ve şu adresler:"
    );
    assert_eq!(satirlar[1], "internal.registry.corp (özel ağ)");
    assert_eq!(satirlar[2], "github.com");
    assert_eq!(satirlar[3], "raw.githubusercontent.com");
    assert_eq!(satirlar.len(), 4, "her ek host ayrı bir satırdır");

    // Yalıtımsızda "Ağ: sınırsız (yalıtımsız)"
    let yalitimsiz_tarif = kalfa_core::kabuk::KabukTarifi {
        yalitim: kalfa_core::kabuk::YalitimOzeti::Yalitimsiz {
            talimat: "talimat".into(),
        },
        ..tarif.clone()
    };
    assert_eq!(
        kart_ag_satirlari(&yalitimsiz_tarif, &kullanici_hostlari),
        vec!["Ağ: sınırsız (yalıtımsız)"]
    );

    // Ağ kapalıysa "Ağ: kapalı"
    let kapali_tarif = kalfa_core::kabuk::KabukTarifi { ag: false, ..tarif };
    assert_eq!(kart_ag_satirlari(&kapali_tarif, &[]), vec!["Ağ: kapalı"]);
}

// --- 3e Görev 2: ek gizli kalıplar ve gizli sayılmayanlar (tasarım §9.2) ---

mod gizli_ayarlari {
    use std::sync::Arc;

    use kalfa_core::izin::{
        AZAMI_KALIP, GIZLI_UYARISI, GizliAyarlari, Islem, KalipHatasi, Karar, Kip, Siniflandirici,
        YolSinifi, kalip_dogrula, karar_ver,
    };
    use kalfa_core::yol::CalismaYolu;

    fn y(s: &str) -> CalismaYolu {
        CalismaYolu::coz(s).unwrap()
    }

    fn l(k: &[&str]) -> Vec<String> {
        k.iter().map(|s| s.to_string()).collect()
    }

    fn sinif(ek: &[&str], istisna: &[&str]) -> (Siniflandirici, Arc<GizliAyarlari>) {
        let a = Arc::new(GizliAyarlari::yeni(&l(ek), &l(istisna)).unwrap());
        let s = Siniflandirici::yeni_ayarla("/kok", a.clone()).unwrap();
        (s, a)
    }

    #[test]
    fn ek_kalip_gizli_yapar() {
        let (s, _) = sinif(&["*.sifre"], &[]);
        assert!(s.gizli_mi(&y("a/b.sifre")));
        assert!(!s.gizli_mi(&y("a/b.txt")));
    }

    #[test]
    fn istisna_varsayilan_kalibi_kaldirir() {
        let (s, _) = sinif(&[], &["src/credentials.rs", ".env.example"]);
        assert!(!s.gizli_mi(&y("src/credentials.rs")));
        assert!(!s.gizli_mi(&y(".env.example")));
        assert!(
            !s.gizli_mi(&y("alt/.env.example")),
            "kalıp her derinlikte eşleşir"
        );
        // İstisna yalnızca adı geçeni açar.
        assert!(s.gizli_mi(&y("credentials.json")));
        assert!(s.gizli_mi(&y(".env")));
        assert!(s.gizli_mi(&y(".env.local")));
    }

    #[test]
    fn istisna_ek_kalibi_kaldirir() {
        let (s, _) = sinif(&["*.sifre"], &["ornek.sifre"]);
        assert!(!s.gizli_mi(&y("ornek.sifre")));
        assert!(s.gizli_mi(&y("gercek.sifre")));
    }

    #[test]
    fn bagimlilik_dizini_kurali_surer() {
        let (s, _) = sinif(&["*.sifre"], &[]);
        assert!(!s.gizli_mi(&y("node_modules/paket/.env")));
        assert!(!s.gizli_mi(&y("node_modules/paket/a.sifre")));
    }

    #[test]
    fn gizli_dizindeki_dosyaya_istisna_dizini_acmaz() {
        // `.ssh/` varsayılan gizli değil; ek kalıpla gizli bir dizin kurulur.
        let (s, _) = sinif(&["gizli"], &["gizli/ornek.txt"]);
        assert!(s.gizli_mi(&y("gizli/ornek.txt")), "üst dizin gizli kalır");
        assert!(s.gizli_mi(&y("gizli/baska.txt")));
    }

    #[test]
    fn dizine_istisna_altindakileri_acar_ek_kalip_kapatir() {
        let (s, _) = sinif(&["gizli", "gizli/anahtar.txt"], &["gizli"]);
        assert!(
            !s.gizli_mi(&y("gizli/notlar.txt")),
            "dizin istisnayla açılır"
        );
        assert!(
            s.gizli_mi(&y("gizli/anahtar.txt")),
            "dosyanın kendi kalıbı kapatır"
        );
        assert!(s.gizli_mi(&y("gizli/.env")), "varsayılan kalıp da kapatır");
    }

    #[test]
    fn istisna_cok_baglantili_okumanin_onayini_kaldirmaz() {
        // İstisna yalnızca adın eşleşmesini kaldırır: hard link'i olan dosya
        // yine onaya çıkar (güvenlik serisi #41).
        let (s, _) = sinif(&[], &["src/credentials.rs"]);
        let yol = y("src/credentials.rs");
        let sinif = if s.gizli_mi(&yol) {
            YolSinifi::Gizli
        } else {
            YolSinifi::Normal
        };
        assert_eq!(sinif, YolSinifi::Normal);
        assert_eq!(
            karar_ver(Kip::Serbest, Islem::Okuma, sinif, true),
            Karar::Onayli {
                uyarilar: vec![GIZLI_UYARISI.into()]
            }
        );
    }

    #[test]
    fn yalniz_joker_istisna_reddedilir() {
        for k in ["*", "**", "*.*", "**/*", "?"] {
            assert_eq!(
                kalip_dogrula(k, true),
                Err(KalipHatasi::YalnizJoker(k.into())),
                "{k}"
            );
        }
        // Ek kalıpta joker serbesttir: gizliliği artırır.
        assert_eq!(kalip_dogrula("*", false), Ok(()));
    }

    #[test]
    fn gecersiz_kaliplar_reddedilir_eski_hal_kalir() {
        assert_eq!(kalip_dogrula("  ", false), Err(KalipHatasi::Bos));
        assert!(matches!(
            kalip_dogrula("/etc/x", false),
            Err(KalipHatasi::Mutlak(_))
        ));
        assert!(matches!(
            kalip_dogrula("C:\\x", false),
            Err(KalipHatasi::Mutlak(_))
        ));
        assert!(matches!(
            kalip_dogrula("a/../b", false),
            Err(KalipHatasi::UstDizin(_))
        ));
        assert!(matches!(
            kalip_dogrula(&"a".repeat(257), false),
            Err(KalipHatasi::Uzun(_))
        ));
        assert!(matches!(
            kalip_dogrula("a[", false),
            Err(KalipHatasi::Gecersiz(..))
        ));

        let (s, a) = sinif(&["*.sifre"], &["ornek.sifre"]);
        assert!(a.degistir(&l(&["*.sifre", "/mutlak"]), &[]).is_err());
        assert_eq!(a.ek_kaliplar(), l(&["*.sifre"]));
        assert_eq!(a.istisnalar(), l(&["ornek.sifre"]));
        assert!(!s.gizli_mi(&y("ornek.sifre")), "eski istisna geçerli");
    }

    #[test]
    fn iki_yuzden_fazla_kalip() {
        let cok: Vec<String> = (0..=AZAMI_KALIP).map(|i| format!("k{i}.txt")).collect();
        assert_eq!(
            GizliAyarlari::yeni(&cok, &[]).err(),
            Some(KalipHatasi::CokKalip)
        );
        assert!(GizliAyarlari::yeni(&cok[..AZAMI_KALIP], &[]).is_ok());
    }

    #[test]
    fn degisiklik_acik_siniflandiricida_hemen_gecerli() {
        let (s, a) = sinif(&[], &[]);
        assert!(s.gizli_mi(&y("src/credentials.rs")));
        a.degistir(&l(&["*.sifre"]), &l(&["src/credentials.rs"]))
            .unwrap();
        assert!(!s.gizli_mi(&y("src/credentials.rs")));
        assert!(s.gizli_mi(&y("a.sifre")));
    }

    /// G9: paylaşılan ayarın her sınıflandırmada okunmasının maliyeti.
    /// `cargo test -p kalfa-core --test izin g9 -- --ignored --nocapture`
    #[test]
    #[ignore = "ölçüm"]
    fn g9_gizli_mi_olcumu() {
        let yollar: Vec<CalismaYolu> = (0..100_000)
            .map(|i| y(&format!("src/modul{}/alt{}/dosya{i}.rs", i % 97, i % 13)))
            .collect();
        let ek: Vec<String> = (0..50).map(|i| format!("*.ozel{i}")).collect();
        let istisna: Vec<String> = (0..50).map(|i| format!("ornek{i}.env")).collect();
        let duz = Siniflandirici::yeni("/kok", &[]).unwrap();
        let (ayarli, _) = {
            let a = Arc::new(GizliAyarlari::yeni(&ek, &istisna).unwrap());
            (Siniflandirici::yeni_ayarla("/kok", a.clone()).unwrap(), a)
        };
        for (ad, s) in [("ayarsız", &duz), ("50 ek + 50 istisna", &ayarli)] {
            let bas = std::time::Instant::now();
            let n = yollar.iter().filter(|p| s.gizli_mi(p)).count();
            println!("[G9] {ad}: 100 000 yol {:?} ({n} gizli)", bas.elapsed());
        }
    }
}
