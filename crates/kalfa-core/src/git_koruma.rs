//! Git'in kendiliğinden kod çalıştırdığı korumalı yollar (spec §7.3).
//!
//! Dosya araçları için `.git`'in tamamı korumalıdır, her derinlikte
//! (güvenlik analizi Bulgu 1): `commondir`, `modules/*/config` ya da iç içe
//! bir `.git` git'i başka bir yapılandırmaya ve kanca dizinine yönlendirir;
//! git de onları yalıtım dışında, kullanıcının yetkileriyle çalıştırır.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::yol::CalismaYolu;

/// Yalıtımın git yerleşimi (aşama 2 tasarımı §7.2, §7.4): korumalı yolların
/// mutlak ve gerçek yazımlı hâli, çalışma alanı dışındaki git dizinleri.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitYerlesimi {
    /// Yalıtımda salt okunur kalacak yollar: `.git`'in kendisi ve gerekirse
    /// `core.hooksPath` hedefi.
    pub korumali: Vec<KorumaliYol>,
    /// Çalışma alanı dışındaki, doğrulanmış git dizinleri (worktree'nin
    /// ortak git dizini, alt modülün git dizini). Yalıtımda salt okunur
    /// görünür.
    pub dis_dizinler: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KorumaliYol {
    pub yol: PathBuf,
    pub dizin: bool,
}

/// Çalışma alanının git yerleşimi. Git deposu değilse boştur.
///
/// - `.git` (dizin ya da worktree'nin dosyası) yalıtımda bütünüyle salt
///   okunurdur: yeniden adlandırılamaz, içine yazılamaz. Kancalar ve
///   yapılandırma onunla korunur. Yazılabilir olsaydı komut `.git`'i kenara
///   çekip kendi kancalarıyla yeni bir `.git` kurabilir ya da `commondir`
///   yazarak git'i başka bir kanca dizinine yönlendirebilirdi.
/// - `.git` dosyasının `gitdir:` satırı ve `commondir` çalışma alanında, yani
///   komutun yazabildiği bir yerdedir. Gösterdikleri dışarıdaki dizin yalnızca
///   geri başvurusu tutarsa kullanılır (worktree'de ortak dizinin
///   `worktrees/<ad>/gitdir` dosyası bu `.git`'i gösterir; alt modülde git
///   dizininin `core.worktree`'si bu çalışma alanını). Tutmazsa dışarıdaki
///   dizin açılmaz; komut rastgele bir dizini (örneğin ev dizinini)
///   açtıramaz.
/// - `core.hooksPath` hedefi yalıtımda görünen, `.git`'in ve dış git
///   dizinlerinin dışındaki bir yerdeyse ayrıca korunur.
/// - `.git` sembolik bağlantıysa korunmaz ve izlenmez (bilinen sınır).
pub fn git_yerlesimi(kok: &Path) -> GitYerlesimi {
    let Ok(kok) = kok.canonicalize() else {
        return GitYerlesimi::default();
    };
    let mut y = GitYerlesimi::default();
    let nokta_git = kok.join(".git");
    let gercek = std::fs::symlink_metadata(&nokta_git)
        .ok()
        .filter(|m| !m.file_type().is_symlink());
    if let Some(bilgi) = gercek {
        y.korumali.push(KorumaliYol {
            yol: nokta_git.clone(),
            dizin: bilgi.is_dir(),
        });
        if let Some((gitdir, ortak)) = git_dizinleri(&kok) {
            let disarida: Vec<PathBuf> = [ortak, gitdir]
                .into_iter()
                .filter(|d| !d.starts_with(&kok))
                .collect();
            // İç içe olanlardan dıştaki yeter (worktree'nin gitdir'i ortak
            // dizinin altındadır).
            for d in &disarida {
                let kapsanir = disarida.iter().any(|o| o != d && d.starts_with(o));
                if !kapsanir && !y.dis_dizinler.contains(d) {
                    y.dis_dizinler.push(d.clone());
                }
            }
        }
    }
    if let Some(hooks) = hooks_path_mutlak(&kok) {
        let gorunur =
            hooks.starts_with(&kok) || y.dis_dizinler.iter().any(|d| hooks.starts_with(d));
        let zaten =
            hooks.starts_with(&nokta_git) || y.dis_dizinler.iter().any(|d| hooks.starts_with(d));
        if gorunur && !zaten {
            y.korumali.push(KorumaliYol {
                yol: hooks,
                dizin: true,
            });
        }
    }
    y
}

/// Kanonik ve doğrulanmış (gitdir, ortak git dizini). `.git` bir dizinse
/// gitdir odur; `commondir`'i doğrulanamazsa yok sayılır. `.git` bir dosyaysa
/// gösterdiği dizin doğrulanamazsa yerleşim yoktur.
fn git_dizinleri(kok: &Path) -> Option<(PathBuf, PathBuf)> {
    let nokta_git = kok.join(".git");
    let dosyadan = !nokta_git.is_dir();
    let gitdir = if dosyadan {
        let icerik = std::fs::read_to_string(&nokta_git).ok()?;
        let hedef = icerik
            .lines()
            .find_map(|s| s.trim().strip_prefix("gitdir:"))?
            .trim()
            .to_string();
        kok.join(hedef).canonicalize().ok()?
    } else {
        nokta_git.canonicalize().ok()?
    };
    let ortak = std::fs::read_to_string(gitdir.join("commondir"))
        .ok()
        .and_then(|m| gitdir.join(m.trim()).canonicalize().ok())
        .filter(|o| o != &gitdir);
    match ortak {
        Some(o) if worktree_mu(kok, &gitdir, &o) => Some((gitdir, o)),
        // Doğrulanmamış `commondir` yok sayılır.
        _ if !dosyadan || alt_modul_mu(kok, &gitdir) => Some((gitdir.clone(), gitdir)),
        _ => None,
    }
}

/// Worktree: gitdir, ortak dizinin `worktrees/` altındadır ve `gitdir`
/// dosyası bu çalışma alanının `.git`'ini gösterir.
fn worktree_mu(kok: &Path, gitdir: &Path, ortak: &Path) -> bool {
    let Ok(worktrees) = ortak.join("worktrees").canonicalize() else {
        return false;
    };
    if gitdir.parent() != Some(worktrees.as_path()) {
        return false;
    }
    let Ok(geri) = std::fs::read_to_string(gitdir.join("gitdir")) else {
        return false;
    };
    let geri = gitdir.join(geri.trim());
    match (geri.canonicalize(), kok.join(".git").canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Alt modül: git dizininin `core.worktree`'si bu çalışma alanını gösterir.
/// Yapılandırma burada okunur, git çalıştırılmaz. Basit okuma yeterlidir:
/// doğrulanan dizin, yalıtımın yazamadığı bir yerde bu değeri taşımak
/// zorundadır.
fn alt_modul_mu(kok: &Path, gitdir: &Path) -> bool {
    let Ok(metin) = std::fs::read_to_string(gitdir.join("config")) else {
        return false;
    };
    let mut core = false;
    let deger = metin.lines().find_map(|satir| {
        let satir = satir.trim();
        if satir.starts_with('[') {
            core = satir
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .eq_ignore_ascii_case("core");
            return None;
        }
        let (ad, deger) = satir.split_once('=')?;
        (core && ad.trim().eq_ignore_ascii_case("worktree"))
            .then(|| deger.trim().trim_matches('"').to_string())
    });
    deger.is_some_and(|d| gitdir.join(d).canonicalize().is_ok_and(|w| w == kok))
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

/// Yol bileşeni git'in dizin adı mı: `.git`, büyük-küçük harf duyarsız ve
/// sondaki nokta ile boşluklar atılarak (Windows bunları yok sayar), ya da
/// 8.3 kısa adı `git~<rakamlar>`. git'in `is_ntfs_dotgit` denetimine denktir
/// ve her platformda uygulanır.
pub fn git_bileseni_mi(parca: &str) -> bool {
    let ad = parca.trim_end_matches(['.', ' ']).to_ascii_lowercase();
    ad == ".git"
        || ad
            .strip_prefix("git~")
            .is_some_and(|r| !r.is_empty() && r.bytes().all(|b| b.is_ascii_digit()))
}

/// Yolun herhangi bir bileşeni git'in dizin adı mı; derinlikten bağımsız.
pub fn git_ic_yolu_mu(yol: &CalismaYolu) -> bool {
    yol.as_str().split('/').any(git_bileseni_mi)
}

/// Çalışma alanındaki korumalı yollar; her öğe küçük harfli, göreli yoldur.
/// Her derinlikteki `.git` ayrıca `git_ic_yolu_mu` ile korunur; burada adı
/// `.git` olmayan git dizinleri de listelenir:
/// - `.git` bir dosyaysa gösterdiği dizinin tamamı,
/// - git dizininin `commondir`'i,
/// - `core.hooksPath`.
///
/// Çalışma alanı dışına düşen hedefler listelenmez: onlara zaten çalışma
/// alanı dışı kuralı uygulanır.
pub fn korumali_yollar(kok: &Path) -> Vec<String> {
    let mut yollar = vec![".git".to_string()];
    if let Some(gitdir) = git_dizini(kok) {
        if let Some(ortak) = ortak_dizin(kok, &gitdir) {
            yollar.push(ortak);
        }
        yollar.push(gitdir);
    }
    if let Some(hooks) = hooks_path(kok) {
        yollar.push(hooks);
    }
    yollar.into_iter().map(|y| y.to_lowercase()).collect()
}

/// Bir `.git` girdisinin git'i yönlendirdiği, çalışma alanı içindeki dizinler
/// (yalıtımda salt okunur kalmaları için): `.git` bir dosyaysa `gitdir:`
/// hedefi, ve git dizininin `commondir` hedefi. `kok` kanoniktir; dışarıya
/// düşen ya da çözülemeyen hedefler atlanır. Değerler doğrulanmaz: korumak
/// yalnızca kısıtlar.
pub fn yonlendirilen_dizinler(nokta_git: &Path, dizin: bool, kok: &Path) -> Vec<PathBuf> {
    let Some(ust) = nokta_git.parent() else {
        return Vec::new();
    };
    let gitdir = if dizin {
        nokta_git.to_path_buf()
    } else {
        let Some(hedef) = std::fs::read_to_string(nokta_git).ok().and_then(|m| {
            m.lines().find_map(|s| {
                s.trim()
                    .strip_prefix("gitdir:")
                    .map(|h| h.trim().to_string())
            })
        }) else {
            return Vec::new();
        };
        match ust.join(hedef).canonicalize() {
            Ok(g) => g,
            Err(_) => return Vec::new(),
        }
    };
    let ortak = std::fs::read_to_string(gitdir.join("commondir"))
        .ok()
        .and_then(|m| gitdir.join(m.trim()).canonicalize().ok());
    let mut dizinler = Vec::new();
    if !dizin {
        dizinler.push(gitdir);
    }
    dizinler.extend(ortak);
    // Kökün kendisi salt okunur yapılamaz; o zaman bütün alan yazılamazdı.
    dizinler.retain(|d| d.starts_with(kok) && d != kok);
    dizinler.sort();
    dizinler.dedup();
    dizinler
}

/// Git dizininin `commondir`'inin çalışma alanındaki göreli yolu. Değer
/// doğrulanmaz: korumak yalnızca kısıtlar.
fn ortak_dizin(kok: &Path, gitdir: &str) -> Option<String> {
    let gitdir = kok.join(gitdir);
    let deger = std::fs::read_to_string(gitdir.join("commondir")).ok()?;
    let deger = deger.trim();
    if deger.is_empty() {
        return None;
    }
    goreli_yap(kok, &gitdir.join(deger))
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
