//! Git'in kendiliğinden kod çalıştırdığı korumalı yollar (spec §7.3).

use std::path::Path;
use std::process::Command;

use crate::yol::CalismaYolu;

/// Çalışma alanındaki korumalı yollar; her öğe küçük harfli, göreli yoldur.
/// Çalışma alanı dışına düşen hedefler listelenmez: onlara zaten çalışma
/// alanı dışı kuralı uygulanır.
pub fn korumali_yollar(kok: &Path) -> Vec<String> {
    let mut yollar = Vec::new();
    if let Some(gitdir) = git_dizini(kok) {
        yollar.push(format!("{gitdir}/hooks"));
        yollar.push(format!("{gitdir}/config"));
    }
    if let Some(hooks) = hooks_path(kok) {
        yollar.push(hooks);
    }
    yollar.into_iter().map(|y| y.to_lowercase()).collect()
}

/// `.git` bir dizinse `.git`; `gitdir:` satırı taşıyan bir dosyaysa (worktree,
/// submodule) gösterdiği dizin.
fn git_dizini(kok: &Path) -> Option<String> {
    let nokta_git = kok.join(".git");
    if nokta_git.is_dir() {
        return Some(".git".to_string());
    }
    let icerik = std::fs::read_to_string(&nokta_git).ok()?;
    let hedef = icerik
        .lines()
        .find_map(|s| s.trim().strip_prefix("gitdir:"))?
        .trim()
        .to_string();
    goreli_yap(kok, Path::new(&hedef))
}

/// `core.hooksPath`'in etkin değeri (depo + kullanıcı yapılandırması).
/// `git config` yapılandırmayı yalnızca okur; kanca ya da fsmonitor çalıştırmaz.
fn hooks_path(kok: &Path) -> Option<String> {
    let cikti = Command::new("git")
        .arg("-C")
        .arg(kok)
        .args(["config", "--get", "core.hooksPath"])
        .output()
        .ok()?;
    if !cikti.status.success() {
        return None;
    }
    let deger = String::from_utf8_lossy(&cikti.stdout).trim().to_string();
    if deger.is_empty() || deger.starts_with('~') {
        return None;
    }
    goreli_yap(kok, Path::new(&deger))
}

fn goreli_yap(kok: &Path, hedef: &Path) -> Option<String> {
    if hedef.is_relative() {
        return CalismaYolu::coz(&hedef.to_string_lossy())
            .ok()
            .map(|y| y.as_str().to_string());
    }
    let kok = kok.canonicalize().ok()?;
    let hedef = hedef.canonicalize().ok()?;
    let goreli = hedef.strip_prefix(&kok).ok()?;
    CalismaYolu::coz(&goreli.to_string_lossy())
        .ok()
        .map(|y| y.as_str().to_string())
}
