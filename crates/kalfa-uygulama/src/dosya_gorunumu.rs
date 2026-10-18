//! Sağ panelin salt okur çalışma alanı görünümü (masaüstü arayüzü tasarımı
//! §6.2, §6.3). Arayüz yol göndermez: dizinler ve dosyalar oturuma bağlı
//! kimliklerle anılır; kimlik tablosu oturumla birlikte silinir. Açma
//! işlerini araçların çalışma alanı yapar: bileşenler bağlantı izlenmeden
//! açılır, gizli ve başka adı olan dosyanın içeriği dönmez.

use std::collections::HashMap;
use std::io::{self, Read};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use kalfa_core::izin::Siniflandirici;
use kalfa_core::kimlik::OturumId;
use kalfa_core::yol::CalismaYolu;
use kalfa_tools::{CalismaAlani as AracAlani, GirdiTuru, Onizleme};
use serde::Serialize;

use crate::Uygulama;
use crate::hata::KomutHatasi;

/// Bir dizinde listelenen en çok girdi.
pub const DIZIN_SINIRI: usize = 2000;
/// Önizlenen en çok bayt.
pub const ONIZLEME_SINIRI: usize = 256 * 1024;

/// Kimlikler bütün oturumlarda tektir: başka oturumun kimliği bu oturumun
/// tablosunda bulunmaz.
static SONRAKI_KIMLIK: AtomicU64 = AtomicU64::new(1);

#[derive(Default)]
pub(crate) struct DosyaGorunumu {
    alan: Mutex<Option<Arc<AracAlani>>>,
    kimlikler: Mutex<HashMap<u64, CalismaYolu>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DosyaGirdisi {
    pub kimlik: u64,
    pub ad: String,
    pub tur: &'static str,
    pub boyut: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DizinIcerigi {
    pub girdiler: Vec<DosyaGirdisi>,
    /// Dizinde `DIZIN_SINIRI`'ndan çok girdi var.
    pub kirpildi: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum DosyaIcerigi {
    Metin { metin: String, kirpildi: bool },
    Ikili,
    Gizli,
    CokAdli,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum GitDurumu {
    Dal { ad: String },
    AyrikHead,
    GitDegil,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CalismaAlaniOzeti {
    /// Ev dizini `~` ile kısaltılmış yol.
    pub kisa_yol: String,
    pub git: GitDurumu,
}

fn ic(e: impl std::fmt::Display) -> KomutHatasi {
    KomutHatasi::ic(e.to_string())
}

fn okuma_hatasi(e: &io::Error) -> KomutHatasi {
    KomutHatasi::gecersiz(format!("Okunamadı: {e}"))
}

impl Uygulama {
    /// Oturumun çalışma alanı ve görünümü; açık değilse `OturumYok`.
    fn gorunum(
        &self,
        oturum: OturumId,
    ) -> Result<(std::path::PathBuf, Arc<DosyaGorunumu>), KomutHatasi> {
        self.ic()
            .oturumlar
            .get(&oturum)
            .map(|a| (a.calisma_alani.clone(), a.gorunum.clone()))
            .ok_or_else(KomutHatasi::oturum_yok)
    }

    /// Görünümün çalışma alanı tutamacı ilk kullanımda açılır; gizli dosya
    /// ayarları canlı paylaşılır.
    fn gorunum_alani(
        &self,
        kok: &Path,
        gorunum: &DosyaGorunumu,
    ) -> Result<Arc<AracAlani>, KomutHatasi> {
        let mut alan = gorunum.alan.lock().unwrap_or_else(|z| z.into_inner());
        if let Some(a) = alan.as_ref() {
            return Ok(a.clone());
        }
        let siniflandirici =
            Siniflandirici::yeni_ayarla(kok, self.ayarlar.gizli_ayarlari()).map_err(ic)?;
        let yeni = AracAlani::ac(Arc::new(siniflandirici)).map_err(|e| okuma_hatasi(&e))?;
        *alan = Some(yeni.clone());
        Ok(yeni)
    }

    pub async fn calisma_alani_ozeti(
        &self,
        oturum: OturumId,
    ) -> Result<CalismaAlaniOzeti, KomutHatasi> {
        let (kok, _) = self.gorunum(oturum)?;
        let ev = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" });
        tokio::task::spawn_blocking(move || CalismaAlaniOzeti {
            kisa_yol: kisa_yol(&kok, ev.as_deref().map(Path::new)),
            git: git_durumu(&kok),
        })
        .await
        .map_err(ic)
    }

    /// Dizinin girdileri; `dizin` yoksa çalışma alanının kökü.
    pub async fn calisma_alani_dizini(
        &self,
        oturum: OturumId,
        dizin: Option<u64>,
    ) -> Result<DizinIcerigi, KomutHatasi> {
        let (kok, gorunum) = self.gorunum(oturum)?;
        let alan = self.gorunum_alani(&kok, &gorunum)?;
        let ust = dizin.map(|k| gorunum.yol(k)).transpose()?;
        let (girdiler, kirpildi) = tokio::task::spawn_blocking(move || {
            alan.dizini_listele(ust.as_ref(), DIZIN_SINIRI)
                .map(|(g, k)| (ust, g, k))
        })
        .await
        .map_err(ic)?
        .map(|(ust, girdiler, kirpildi)| (gorunum.kaydet(ust.as_ref(), girdiler), kirpildi))
        .map_err(|e| okuma_hatasi(&e))?;
        Ok(DizinIcerigi { girdiler, kirpildi })
    }

    pub async fn calisma_alani_dosyasi(
        &self,
        oturum: OturumId,
        kimlik: u64,
    ) -> Result<DosyaIcerigi, KomutHatasi> {
        let (kok, gorunum) = self.gorunum(oturum)?;
        let alan = self.gorunum_alani(&kok, &gorunum)?;
        let yol = gorunum.yol(kimlik)?;
        let onizleme = tokio::task::spawn_blocking(move || alan.onizle(&yol, ONIZLEME_SINIRI))
            .await
            .map_err(ic)?
            .map_err(|e| okuma_hatasi(&e))?;
        Ok(match onizleme {
            Onizleme::Metin { metin, kirpildi } => DosyaIcerigi::Metin { metin, kirpildi },
            Onizleme::Ikili => DosyaIcerigi::Ikili,
            Onizleme::Gizli => DosyaIcerigi::Gizli,
            Onizleme::CokAdli => DosyaIcerigi::CokAdli,
        })
    }
}

impl DosyaGorunumu {
    fn yol(&self, kimlik: u64) -> Result<CalismaYolu, KomutHatasi> {
        self.kimlikler
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .get(&kimlik)
            .cloned()
            .ok_or_else(|| KomutHatasi::gecersiz("Bu dosya bu oturumda bilinmiyor."))
    }

    /// Girdilere kimlik verir; aynı yol aynı kimliği korur.
    fn kaydet(
        &self,
        ust: Option<&CalismaYolu>,
        girdiler: Vec<kalfa_tools::DizinGirdisi>,
    ) -> Vec<DosyaGirdisi> {
        let mut tablo = self.kimlikler.lock().unwrap_or_else(|z| z.into_inner());
        girdiler
            .into_iter()
            .filter_map(|g| {
                let goreli =
                    ust.map_or_else(|| g.ad.clone(), |u| format!("{}/{}", u.as_str(), g.ad));
                let yol = CalismaYolu::coz(&goreli).ok()?;
                let kimlik = match tablo.iter().find(|(_, y)| **y == yol) {
                    Some((k, _)) => *k,
                    None => {
                        let k = SONRAKI_KIMLIK.fetch_add(1, Ordering::Relaxed);
                        tablo.insert(k, yol);
                        k
                    }
                };
                Some(DosyaGirdisi {
                    kimlik,
                    ad: g.ad,
                    tur: match g.tur {
                        GirdiTuru::Dizin => "Dizin",
                        GirdiTuru::Dosya => "Dosya",
                        GirdiTuru::Baglanti => "Baglanti",
                    },
                    boyut: g.boyut,
                })
            })
            .collect()
    }
}

/// Ev dizininin altındaysa `~` ile başlar.
fn kisa_yol(kok: &Path, ev: Option<&Path>) -> String {
    match ev.and_then(|e| kok.strip_prefix(e).ok()) {
        Some(alt) if alt.as_os_str().is_empty() => "~".into(),
        Some(alt) => format!("~{}{}", std::path::MAIN_SEPARATOR, alt.display()),
        None => kok.display().to_string(),
    }
}

/// `.git/HEAD`'den dal; git süreci başlatılmaz. `.git` dosyaysa (worktree)
/// `gitdir:` satırının gösterdiği dizine bakılır. Bağlantı olan `.git`
/// izlenmez.
pub(crate) fn git_durumu(kok: &Path) -> GitDurumu {
    let git = kok.join(".git");
    let Ok(meta) = std::fs::symlink_metadata(&git) else {
        return GitDurumu::GitDegil;
    };
    let git_dizini = if meta.is_dir() {
        git
    } else if meta.is_file() {
        match kisa_oku(&git)
            .as_deref()
            .and_then(|s| s.trim().strip_prefix("gitdir:"))
            .map(|d| Path::new(d.trim()))
        {
            // `.git` dosyasını çalışma alanına kim yazdıysa (klonlanan depo,
            // model) seçer: ağ yolu açılmaz.
            Some(d) if yerel_yol_mu(d) => kok.join(d),
            _ => return GitDurumu::GitDegil,
        }
    } else {
        return GitDurumu::GitDegil;
    };
    let head = git_dizini.join("HEAD");
    // gitdir ve HEAD bağlantı olamaz: bağlantı ağ yoluna çıkabilir.
    let gercek = |yol: &Path, dizin: bool| {
        std::fs::symlink_metadata(yol).is_ok_and(|m| if dizin { m.is_dir() } else { m.is_file() })
    };
    if !gercek(&git_dizini, true) || !gercek(&head, false) {
        return GitDurumu::GitDegil;
    }
    match kisa_oku(&head) {
        Some(head) => match head.trim().strip_prefix("ref:") {
            Some(r) => {
                let r = r.trim();
                GitDurumu::Dal {
                    ad: r.strip_prefix("refs/heads/").unwrap_or(r).to_string(),
                }
            }
            None => GitDurumu::AyrikHead,
        },
        None => GitDurumu::GitDegil,
    }
}

/// Göreli ya da sürücü harfli yerel yol mu? Windows'ta UNC (`\\sunucu\pay`),
/// `\\?\` ve aygıt yollarını açmak ağa kimlik bilgisi gönderebilir.
fn yerel_yol_mu(yol: &Path) -> bool {
    use std::path::{Component, Prefix};
    match yol.components().next() {
        Some(Component::Prefix(p)) => matches!(p.kind(), Prefix::Disk(_)),
        _ => true,
    }
}

/// En çok 4 KiB.
fn kisa_oku(yol: &Path) -> Option<String> {
    let mut metin = String::new();
    std::fs::File::open(yol)
        .ok()?
        .take(4096)
        .read_to_string(&mut metin)
        .ok()?;
    Some(metin)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn git_dali_ayrik_head_ve_worktree() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(git_durumu(d.path()), GitDurumu::GitDegil);

        std::fs::create_dir(d.path().join(".git")).unwrap();
        std::fs::write(d.path().join(".git/HEAD"), "ref: refs/heads/ui/sag-panel\n").unwrap();
        assert_eq!(
            git_durumu(d.path()),
            GitDurumu::Dal {
                ad: "ui/sag-panel".into()
            }
        );
        std::fs::write(
            d.path().join(".git/HEAD"),
            "90e729d0123456789abcdef0123456789abcdef0\n",
        )
        .unwrap();
        assert_eq!(git_durumu(d.path()), GitDurumu::AyrikHead);

        let w = tempfile::tempdir().unwrap();
        let gitdir = d.path().join(".git");
        std::fs::write(d.path().join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(
            w.path().join(".git"),
            format!("gitdir: {}\n", gitdir.display()),
        )
        .unwrap();
        assert_eq!(git_durumu(w.path()), GitDurumu::Dal { ad: "main".into() });

        // Göreli gitdir de okunur.
        std::fs::create_dir_all(w.path().join("modul/.git-dizin")).unwrap();
        std::fs::write(
            w.path().join("modul/.git-dizin/HEAD"),
            "ref: refs/heads/alt\n",
        )
        .unwrap();
        std::fs::write(w.path().join(".git"), "gitdir: modul/.git-dizin\n").unwrap();
        assert_eq!(git_durumu(w.path()), GitDurumu::Dal { ad: "alt".into() });
    }

    /// Aynı yerel gitdir'e `\\?\` ya da UNC biçimiyle gidilmez: Windows'ta
    /// bu yolları açmak ağa kimlik bilgisi gönderebilir.
    #[cfg(windows)]
    #[test]
    fn ag_ve_ozel_onekli_gitdir_acilmaz() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir(d.path().join("gd")).unwrap();
        std::fs::write(d.path().join("gd/HEAD"), "ref: refs/heads/main\n").unwrap();
        let w = tempfile::tempdir().unwrap();
        let yerel = d.path().join("gd");
        std::fs::write(
            w.path().join(".git"),
            format!("gitdir: {}\n", yerel.display()),
        )
        .unwrap();
        assert_eq!(git_durumu(w.path()), GitDurumu::Dal { ad: "main".into() });

        let ozel = format!(r"\\?\{}", yerel.display());
        std::fs::write(w.path().join(".git"), format!("gitdir: {ozel}\n")).unwrap();
        assert!(Path::new(&ozel).join("HEAD").exists(), "{ozel}");
        assert_eq!(git_durumu(w.path()), GitDurumu::GitDegil);

        for ag in [r"\\127.0.0.1\pay\gd", "//127.0.0.1/pay/gd"] {
            std::fs::write(w.path().join(".git"), format!("gitdir: {ag}\n")).unwrap();
            assert_eq!(git_durumu(w.path()), GitDurumu::GitDegil, "{ag}");
            assert!(!yerel_yol_mu(Path::new(ag)), "{ag}");
        }
    }

    /// HEAD bağlantıysa izlenmez.
    #[cfg(unix)]
    #[test]
    fn baglanti_head_izlenmez() {
        let d = tempfile::tempdir().unwrap();
        let dis = tempfile::tempdir().unwrap();
        std::fs::write(dis.path().join("HEAD"), "ref: refs/heads/dis\n").unwrap();
        std::fs::create_dir(d.path().join(".git")).unwrap();
        std::os::unix::fs::symlink(dis.path().join("HEAD"), d.path().join(".git/HEAD")).unwrap();
        assert_eq!(git_durumu(d.path()), GitDurumu::GitDegil);
    }

    #[test]
    fn ev_dizini_kisaltilir() {
        let ev = Path::new("/ev/orkun");
        assert_eq!(kisa_yol(Path::new("/ev/orkun"), Some(ev)), "~");
        assert_eq!(
            kisa_yol(Path::new("/ev/orkun/proje"), Some(ev)),
            format!("~{}proje", std::path::MAIN_SEPARATOR)
        );
        assert_eq!(
            kisa_yol(Path::new("/baska/proje"), Some(ev)),
            "/baska/proje"
        );
        assert_eq!(kisa_yol(Path::new("/baska/proje"), None), "/baska/proje");
    }
}
