//! Gizli dosya taraması (aşama 2 tasarımı §7.2). Her çalıştırmadan hemen önce
//! bütün çalışma alanı taranır; `.gitignore` kapsamındakiler dahil, çünkü
//! `.env` genellikle yok sayılan bir dosyadır. Sembolik bağlantılar izlenmez.
//!
//! Linux'ta bağlantı sayısı birden büyük dosyaların (aygıt, inode) çifti gizli
//! dosyalarınkiyle karşılaştırılır: `.env`'nin hard link'i de gizlidir.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ignore::{WalkBuilder, WalkState};

/// Taramanın sonucu; yollar mutlaktır ve sıralıdır.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tarama {
    /// Okunamayacak dosyalar; gizli bir dizinin içindekiler bu listede yoktur,
    /// dizinin kendisi örtülür.
    pub dosyalar: Vec<PathBuf>,
    /// İçi boş görünecek dizinler.
    pub dizinler: Vec<PathBuf>,
}

impl Tarama {
    pub fn sayi(&self) -> usize {
        self.dosyalar.len() + self.dizinler.len()
    }
}

#[derive(Default)]
struct Toplanan {
    dosyalar: BTreeSet<PathBuf>,
    dizinler: BTreeSet<PathBuf>,
    /// Gizli dosyaların (aygıt, inode) çiftleri; yalnızca bağlantı sayısı
    /// birden büyük olanlar.
    gizli_inode: BTreeSet<(u64, u64)>,
    /// Bağlantı sayısı birden büyük bütün normal dosyalar.
    bagli: Vec<((u64, u64), PathBuf)>,
}

/// Çalışma alanını tarar. `gizli_mi` çalışma alanına göre yolu ve yolun dizin
/// olup olmadığını alır.
pub fn tara(kok: &Path, gizli_mi: &(dyn Fn(&Path, bool) -> bool + Sync)) -> Tarama {
    let toplanan = Mutex::new(Toplanan::default());
    WalkBuilder::new(kok)
        .standard_filters(false)
        .follow_links(false)
        .build_parallel()
        .run(|| {
            let toplanan = &toplanan;
            Box::new(move |giris| {
                let Ok(giris) = giris else {
                    return WalkState::Continue;
                };
                if giris.depth() == 0 {
                    return WalkState::Continue;
                }
                let Some(tur) = giris.file_type() else {
                    return WalkState::Continue;
                };
                let Ok(goreli) = giris.path().strip_prefix(kok) else {
                    return WalkState::Continue;
                };
                // Bağlantı izlenmez; hedefi çalışma alanındaysa kendisi taranır.
                if tur.is_symlink() || !(tur.is_dir() || tur.is_file()) {
                    return WalkState::Continue;
                }
                let gizli = gizli_mi(goreli, tur.is_dir());
                let inode = if tur.is_file() {
                    inode(giris.path())
                } else {
                    None
                };
                let mut t = toplanan.lock().unwrap_or_else(|z| z.into_inner());
                if gizli {
                    if tur.is_dir() {
                        t.dizinler.insert(giris.path().to_path_buf());
                    } else {
                        t.dosyalar.insert(giris.path().to_path_buf());
                    }
                }
                if let Some((cift, bag)) = inode
                    && bag > 1
                {
                    if gizli {
                        t.gizli_inode.insert(cift);
                    }
                    t.bagli.push((cift, giris.path().to_path_buf()));
                }
                // Gizli dizinin içi de gezilir: içindeki bir dosyanın dışarıdaki
                // hard link'i bulunabilsin.
                WalkState::Continue
            })
        });
    let t = toplanan.into_inner().unwrap_or_else(|z| z.into_inner());
    let mut dosyalar = t.dosyalar;
    let gizli_dizin_altinda =
        |yol: &Path| yol.ancestors().skip(1).any(|ata| t.dizinler.contains(ata));
    // Gizli dizindeki dosyaların hard link'leri de gizlidir.
    let mut gizli_inode = t.gizli_inode;
    for (cift, yol) in &t.bagli {
        if gizli_dizin_altinda(yol) {
            gizli_inode.insert(*cift);
        }
    }
    for (cift, yol) in t.bagli {
        if gizli_inode.contains(&cift) {
            dosyalar.insert(yol);
        }
    }
    let dizinler: Vec<PathBuf> = t
        .dizinler
        .iter()
        .filter(|d| !gizli_dizin_altinda(d))
        .cloned()
        .collect();
    Tarama {
        dosyalar: dosyalar
            .into_iter()
            .filter(|d| !gizli_dizin_altinda(d))
            .collect(),
        dizinler,
    }
}

#[cfg(unix)]
fn inode(yol: &Path) -> Option<((u64, u64), u64)> {
    use std::os::unix::fs::MetadataExt;
    let bilgi = std::fs::symlink_metadata(yol).ok()?;
    Some(((bilgi.dev(), bilgi.ino()), bilgi.nlink()))
}

/// Windows'ta gerek yoktur: ACL dosya nesnesindedir, hard link aynı nesneyi
/// paylaşır.
#[cfg(not(unix))]
fn inode(_yol: &Path) -> Option<((u64, u64), u64)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_ya_da_gizli_dizin(yol: &Path, dizin: bool) -> bool {
        let ad = yol.file_name().and_then(|a| a.to_str()).unwrap_or("");
        if dizin {
            ad == "gizli"
        } else {
            ad == ".env" || ad.ends_with(".pem")
        }
    }

    #[test]
    fn gizli_dosya_ve_dizin_bulunur_gizli_dizinin_ici_listelenmez() {
        let d = tempfile::tempdir().unwrap();
        let k = d.path();
        std::fs::create_dir_all(k.join("a/b")).unwrap();
        std::fs::create_dir_all(k.join("gizli/ic")).unwrap();
        std::fs::write(k.join(".env"), "x").unwrap();
        std::fs::write(k.join("a/b/anahtar.pem"), "x").unwrap();
        std::fs::write(k.join("a/b/normal.txt"), "x").unwrap();
        std::fs::write(k.join("gizli/ic/.env"), "x").unwrap();
        // .gitignore kapsamındakiler de taranır.
        std::fs::write(k.join(".gitignore"), ".env\n").unwrap();
        let t = tara(k, &env_ya_da_gizli_dizin);
        assert_eq!(t.dosyalar, vec![k.join(".env"), k.join("a/b/anahtar.pem")]);
        assert_eq!(t.dizinler, vec![k.join("gizli")]);
        assert_eq!(t.sayi(), 3);
    }

    #[cfg(unix)]
    #[test]
    fn hard_link_ve_sembolik_baglanti() {
        let d = tempfile::tempdir().unwrap();
        let k = d.path();
        std::fs::create_dir_all(k.join("gizli")).unwrap();
        std::fs::write(k.join(".env"), "x").unwrap();
        std::fs::hard_link(k.join(".env"), k.join("a.txt")).unwrap();
        std::fs::write(k.join("gizli/sir"), "x").unwrap();
        std::fs::hard_link(k.join("gizli/sir"), k.join("b.txt")).unwrap();
        std::fs::write(k.join("c.txt"), "x").unwrap();
        std::fs::hard_link(k.join("c.txt"), k.join("d.txt")).unwrap();
        // Bağlantı izlenmez ve kendisi listelenmez.
        std::os::unix::fs::symlink(k.join(".env"), k.join("bag.env")).unwrap();
        let t = tara(k, &env_ya_da_gizli_dizin);
        assert_eq!(
            t.dosyalar,
            vec![k.join(".env"), k.join("a.txt"), k.join("b.txt")]
        );
        assert_eq!(t.dizinler, vec![k.join("gizli")]);
    }
}
