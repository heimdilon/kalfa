//! Komutun oluşturduğu yeni `.git`'in denetimi (güvenlik analizi Bulgu 1).
//!
//! Yalıtımlı bir komut çalışma alanında yeni bir `.git` oluşturabilir.
//! Kullanıcı ya da IDE orada `git status` çalıştırınca git, o `.git`'in
//! yapılandırmasını ve kancalarını yalıtım dışında çalıştırır. Yeni `.git`
//! izin listesiyle denetlenir: listede olmayan her şey tehlikelidir.

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitDenetimi {
    Guvenli,
    Tehlikeli { nedenler: Vec<String> },
}

/// Alt bölümsüz izinli yapılandırma anahtarları (`bolum.ad`, küçük harfli).
const IZINLI_ANAHTARLAR: &[&str] = &[
    "core.repositoryformatversion",
    "core.filemode",
    "core.bare",
    "core.logallrefupdates",
    "core.ignorecase",
    "core.precomposeunicode",
    "core.symlinks",
    "core.autocrlf",
    "extensions.objectformat",
    "extensions.refstorage",
    "init.defaultbranch",
    "user.name",
    "user.email",
];

/// Alt bölümlü izinli anahtarlar: (`bolum`, `ad`), örn. `remote.<ad>.url`.
/// `submodule.<ad>.update` yoktur: `!komut` değeri kod çalıştırır.
const IZINLI_ALT_BOLUMLU: &[(&str, &str)] = &[
    ("remote", "url"),
    ("remote", "pushurl"),
    ("remote", "fetch"),
    ("branch", "remote"),
    ("branch", "merge"),
    ("submodule", "url"),
    ("submodule", "active"),
];

/// `yol` yeni bir `.git` girdisidir; `dizin` değilse (dosya ya da bağlantı)
/// git'i başka bir dizine yönlendirir ve tehlikelidir. Git bulunamazsa ya da
/// yapılandırma çözülemezse sonuç tehlikelidir.
pub fn yeni_git_denetle(yol: &Path, dizin: bool) -> GitDenetimi {
    denetle(yol, dizin, OsStr::new("git"))
}

fn denetle(yol: &Path, dizin: bool, git: &OsStr) -> GitDenetimi {
    if !dizin {
        return GitDenetimi::Tehlikeli {
            nedenler: vec!["git'i başka bir dizine yönlendiren bir `.git` dosyası".into()],
        };
    }
    let mut nedenler = Vec::new();
    for ad in ["commondir", "config.worktree", "gitdir"] {
        if std::fs::symlink_metadata(yol.join(ad)).is_ok() {
            nedenler.push(format!("`{ad}` dosyası"));
        }
    }
    kancalari_denetle(&yol.join("hooks"), &mut nedenler);
    let config = yol.join("config");
    match std::fs::symlink_metadata(&config) {
        Ok(bilgi) if bilgi.is_file() => match yapilandirma_anahtarlari(&config, git) {
            Ok(anahtarlar) => {
                for anahtar in anahtarlar {
                    if !izinli_mi(&anahtar) {
                        nedenler.push(format!("yapılandırma anahtarı `{anahtar}`"));
                    }
                }
            }
            Err(e) => nedenler.push(format!("yapılandırma okunamadı: {e}")),
        },
        Ok(_) => nedenler.push("`config` normal bir dosya değil".into()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => nedenler.push(format!("yapılandırma okunamadı: {e}")),
    }
    if nedenler.is_empty() {
        GitDenetimi::Guvenli
    } else {
        GitDenetimi::Tehlikeli { nedenler }
    }
}

/// `hooks/` altında yalnızca `.sample` ile biten normal dosyalar olabilir.
fn kancalari_denetle(hooks: &Path, nedenler: &mut Vec<String>) {
    let girdiler = match std::fs::symlink_metadata(hooks) {
        Ok(bilgi) if bilgi.is_dir() => match std::fs::read_dir(hooks) {
            Ok(g) => g,
            Err(e) => {
                nedenler.push(format!("kanca dizini okunamadı: {e}"));
                return;
            }
        },
        Ok(_) => {
            nedenler.push("`hooks` bir dizin değil".into());
            return;
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => return,
        Err(e) => {
            nedenler.push(format!("kanca dizini okunamadı: {e}"));
            return;
        }
    };
    for girdi in girdiler {
        let Ok(girdi) = girdi else {
            nedenler.push("kanca dizini okunamadı".into());
            return;
        };
        let ad = girdi.file_name().to_string_lossy().into_owned();
        let normal = girdi.file_type().is_ok_and(|t| t.is_file());
        if !normal || !ad.ends_with(".sample") {
            nedenler.push(format!("kanca `{ad}`"));
        }
    }
}

/// Anahtar izin listesinde mi? `bolum.ad` ve `bolum.altbolum.ad` biçimleri;
/// bölüm ve ad büyük-küçük harf duyarsızdır, alt bölüm önemsizdir.
fn izinli_mi(anahtar: &str) -> bool {
    let Some((bolum, kalan)) = anahtar.split_once('.') else {
        return false;
    };
    let bolum = bolum.to_ascii_lowercase();
    match kalan.rsplit_once('.') {
        None => {
            let tam = format!("{bolum}.{}", kalan.to_ascii_lowercase());
            IZINLI_ANAHTARLAR.contains(&tam.as_str())
        }
        Some((_, ad)) => {
            let ad = ad.to_ascii_lowercase();
            IZINLI_ALT_BOLUMLU
                .iter()
                .any(|(b, a)| *b == bolum && *a == ad)
        }
    }
}

/// `git config --file <config> --no-includes --list -z` ile anahtarlar. `--file`
/// ile git yalnızca o dosyayı okur ve hiçbir şey çalıştırmaz; yine de ortam
/// boş başlar, sistem ve kullanıcı yapılandırması kapatılır.
fn yapilandirma_anahtarlari(config: &Path, git: &OsStr) -> io::Result<Vec<String>> {
    let mut komut = Command::new(git);
    komut
        .arg("config")
        .arg("--file")
        .arg(duz_yol(config))
        .args(["--no-includes", "--list", "-z"])
        .env_clear()
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_PAGER", "cat")
        .current_dir(std::env::temp_dir());
    if let Some(path) = std::env::var_os("PATH") {
        komut.env("PATH", path);
    }
    #[cfg(windows)]
    if let Some(kok) = std::env::var_os("SystemRoot") {
        komut.env("SystemRoot", kok);
    }
    let cikti = komut.output()?;
    if !cikti.status.success() {
        return Err(io::Error::other(format!(
            "git config başarısız: {}",
            String::from_utf8_lossy(&cikti.stderr).trim()
        )));
    }
    let metin = String::from_utf8(cikti.stdout)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "git config çıktısı çözülemedi"))?;
    Ok(metin
        .split('\0')
        .filter(|g| !g.is_empty())
        .map(|g| g.split_once('\n').map_or(g, |(a, _)| a).to_string())
        .collect())
}

/// Git for Windows `\\?\` önekli yolları okuyamıyor; kanonik yol öneksiz
/// verilir.
fn duz_yol(yol: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let metin = yol.to_string_lossy();
        if let Some(kalan) = metin.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{kalan}"));
        }
        if let Some(kalan) = metin.strip_prefix(r"\\?\") {
            return PathBuf::from(kalan);
        }
    }
    yol.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(dizin: &Path, args: &[&str]) {
        let durum = Command::new("git")
            .arg("-C")
            .arg(dizin)
            .args(args)
            .status()
            .expect("git kurulu olmalı");
        assert!(durum.success(), "git {args:?}");
    }

    fn depo() -> (tempfile::TempDir, std::path::PathBuf) {
        let d = tempfile::tempdir().unwrap();
        git(d.path(), &["init", "-q"]);
        let g = d.path().join(".git");
        (d, g)
    }

    fn tehlikeli(sonuc: &GitDenetimi, parca: &str) -> bool {
        matches!(sonuc, GitDenetimi::Tehlikeli { nedenler }
            if nedenler.iter().any(|n| n.contains(parca)))
    }

    #[test]
    fn duz_git_init_guvenli() {
        let (_d, g) = depo();
        assert_eq!(yeni_git_denetle(&g, true), GitDenetimi::Guvenli);
        // Kabuk aracı kanonik yol verir; Windows'ta `\\?\` öneklidir.
        let kanonik = g.canonicalize().unwrap();
        assert_eq!(yeni_git_denetle(&kanonik, true), GitDenetimi::Guvenli);
    }

    #[test]
    fn uzak_dal_ve_alt_modul_anahtarlari_guvenli() {
        let (d, g) = depo();
        git(
            d.path(),
            &["remote", "add", "Origin", "https://ornek.invalid/x.git"],
        );
        git(d.path(), &["config", "branch.main.remote", "Origin"]);
        git(
            d.path(),
            &["config", "branch.main.merge", "refs/heads/main"],
        );
        git(
            d.path(),
            &["config", "submodule.alt.url", "https://ornek.invalid/y.git"],
        );
        git(d.path(), &["config", "submodule.alt.active", "true"]);
        git(d.path(), &["config", "user.name", "Ad"]);
        assert_eq!(yeni_git_denetle(&g, true), GitDenetimi::Guvenli);
    }

    #[test]
    fn fsmonitor_tehlikeli() {
        let (d, g) = depo();
        git(d.path(), &["config", "core.fsmonitor", "touch KANIT"]);
        assert!(tehlikeli(&yeni_git_denetle(&g, true), "core.fsmonitor"));
    }

    #[test]
    fn alt_modul_update_komutu_tehlikeli() {
        let (d, g) = depo();
        git(
            d.path(),
            &["config", "submodule.alt.update", "!touch KANIT"],
        );
        assert!(tehlikeli(
            &yeni_git_denetle(&g, true),
            "submodule.alt.update"
        ));
    }

    #[test]
    fn include_path_tehlikeli() {
        let (d, g) = depo();
        git(d.path(), &["config", "include.path", "../baska"]);
        assert!(tehlikeli(&yeni_git_denetle(&g, true), "include.path"));
    }

    #[test]
    fn kanca_tehlikeli_ornek_kanca_degil() {
        let (_d, g) = depo();
        std::fs::create_dir_all(g.join("hooks")).unwrap();
        std::fs::write(g.join("hooks/pre-commit.sample"), "#!/bin/sh\n").unwrap();
        assert_eq!(yeni_git_denetle(&g, true), GitDenetimi::Guvenli);
        std::fs::write(g.join("hooks/pre-commit"), "#!/bin/sh\n").unwrap();
        assert!(tehlikeli(&yeni_git_denetle(&g, true), "pre-commit"));
    }

    #[test]
    fn commondir_ve_git_dosyasi_tehlikeli() {
        let (_d, g) = depo();
        std::fs::write(g.join("commondir"), "../zz\n").unwrap();
        assert!(tehlikeli(&yeni_git_denetle(&g, true), "commondir"));
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join(".git"), "gitdir: ../zz\n").unwrap();
        assert!(matches!(
            yeni_git_denetle(&d.path().join(".git"), false),
            GitDenetimi::Tehlikeli { .. }
        ));
    }

    #[test]
    fn git_bulunamazsa_tehlikeli() {
        let (_d, g) = depo();
        let sonuc = denetle(&g, true, OsStr::new("kalfa-olmayan-git-programi"));
        assert!(tehlikeli(&sonuc, "yapılandırma okunamadı"), "{sonuc:?}");
    }

    #[test]
    fn anahtar_kurali() {
        for a in [
            "core.bare",
            "CORE.Bare",
            "remote.Origin.url",
            "remote.a.b.url",
            "branch.feature/x.merge",
        ] {
            assert!(izinli_mi(a), "{a}");
        }
        for a in [
            "core.fsmonitor",
            "core.hookspath",
            "core.sshcommand",
            "remote.origin.uploadpack",
            "user.x.name",
            "bos",
            "alias.st",
        ] {
            assert!(!izinli_mi(a), "{a}");
        }
    }
}
