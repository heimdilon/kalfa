//! Git'in kendiliğinden kod çalıştırdığı korumalı yollar (spec §7.3).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::yol::CalismaYolu;

/// Yalıtımın git yerleşimi (aşama 2 tasarımı §7.2, §7.4): korumalı yolların
/// mutlak ve gerçek yazımlı hâli, çalışma alanı dışındaki git dizinleri.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitYerlesimi {
    /// Yalıtımda salt okunur kalacak yollar.
    pub korumali: Vec<KorumaliYol>,
    /// Çalışma alanı dışındaki git dizinleri (worktree, submodule). Yalıtımda
    /// yazılabilir bağlanır: yazılan yer çalışma alanının kendi deposudur.
    pub dis_dizinler: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KorumaliYol {
    pub yol: PathBuf,
    pub dizin: bool,
}

/// Çalışma alanının git yerleşimi. Git deposu değilse boştur.
///
/// - Kancalar ve yapılandırma ortak git dizinindedir (worktree'de
///   `commondir`'in gösterdiği yer); worktree'nin kendi `config.worktree`'si
///   varsa o da korunur.
/// - `core.hooksPath` hedefi yalıtımda görünen bir yerdeyse korunur.
pub fn git_yerlesimi(kok: &Path) -> GitYerlesimi {
    let Ok(kok) = kok.canonicalize() else {
        return GitYerlesimi::default();
    };
    let mut y = GitYerlesimi::default();
    if let Some((gitdir, ortak)) = git_dizinleri(&kok) {
        y.korumali.push(KorumaliYol {
            yol: ortak.join("hooks"),
            dizin: true,
        });
        y.korumali.push(KorumaliYol {
            yol: ortak.join("config"),
            dizin: false,
        });
        let calisma_agaci = gitdir.join("config.worktree");
        if gitdir != ortak && calisma_agaci.is_file() {
            y.korumali.push(KorumaliYol {
                yol: calisma_agaci,
                dizin: false,
            });
        }
        let disarida: Vec<PathBuf> = [ortak, gitdir]
            .into_iter()
            .filter(|d| !d.starts_with(&kok))
            .collect();
        // İç içe olanlardan dıştaki yeter (worktree'nin gitdir'i ortak dizinin
        // altındadır).
        for d in &disarida {
            let kapsanir = disarida.iter().any(|o| o != d && d.starts_with(o));
            if !kapsanir && !y.dis_dizinler.contains(d) {
                y.dis_dizinler.push(d.clone());
            }
        }
    }
    if let Some(hooks) = hooks_path_mutlak(&kok) {
        let gorunur =
            hooks.starts_with(&kok) || y.dis_dizinler.iter().any(|d| hooks.starts_with(d));
        if gorunur && !y.korumali.iter().any(|k| k.yol == hooks) {
            y.korumali.push(KorumaliYol {
                yol: hooks,
                dizin: true,
            });
        }
    }
    y
}

/// Kanonik (gitdir, ortak git dizini).
fn git_dizinleri(kok: &Path) -> Option<(PathBuf, PathBuf)> {
    let nokta_git = kok.join(".git");
    let gitdir = if nokta_git.is_dir() {
        nokta_git
    } else {
        let icerik = std::fs::read_to_string(&nokta_git).ok()?;
        let hedef = icerik
            .lines()
            .find_map(|s| s.trim().strip_prefix("gitdir:"))?
            .trim()
            .to_string();
        kok.join(hedef).canonicalize().ok()?
    };
    let ortak = std::fs::read_to_string(gitdir.join("commondir"))
        .ok()
        .and_then(|m| gitdir.join(m.trim()).canonicalize().ok())
        .unwrap_or_else(|| gitdir.clone());
    Some((gitdir, ortak))
}

/// `core.hooksPath`'in mutlak hâli; göreli değer çalışma ağacının köküne
/// göredir. `~` ile başlayan değer ev dizinindedir ve yalıtımda görünmez.
fn hooks_path_mutlak(kok: &Path) -> Option<PathBuf> {
    let deger = hooks_path_degeri(kok)?;
    if deger.starts_with('~') {
        return None;
    }
    let yol = kok.join(&deger);
    Some(yol.canonicalize().unwrap_or(yol))
}

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

/// `core.hooksPath`'in çalışma alanındaki göreli yolu.
fn hooks_path(kok: &Path) -> Option<String> {
    let deger = hooks_path_degeri(kok)?;
    if deger.starts_with('~') {
        return None;
    }
    goreli_yap(kok, Path::new(&deger))
}

/// `core.hooksPath`'in etkin değeri (depo + kullanıcı yapılandırması).
/// `git config` yapılandırmayı yalnızca okur; kanca ya da fsmonitor çalıştırmaz.
fn hooks_path_degeri(kok: &Path) -> Option<String> {
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
    (!deger.is_empty()).then_some(deger)
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
