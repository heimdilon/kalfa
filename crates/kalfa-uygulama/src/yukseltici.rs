//! "Onar"ın gerçeği (aşama 2 tasarımı §9.8): kurulu sürümde Program
//! Files'taki `kalfa-yalitim.exe kur`'u UAC ile çalıştırır. Geliştirme
//! derlemesinde ve Linux'ta kurulu yardımcı yoktur; Onar gösterilmez.

use std::path::{Path, PathBuf};

use crate::arayuz::Yukseltici;
use async_trait::async_trait;

pub struct KuruluYukseltici;

#[cfg(windows)]
fn yardimci() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    kalfa_sandbox::windows::sistem::kurulu_yardimci(&exe)
}

/// `kur` ve her dizin için `--izin-ver <dizin>`.
pub fn kur_argumanlari(izinli: &[PathBuf]) -> Vec<std::ffi::OsString> {
    let mut argumanlar = vec!["kur".into()];
    for dizin in izinli {
        argumanlar.push("--izin-ver".into());
        argumanlar.push(dizin.clone().into_os_string());
    }
    argumanlar
}

/// Kurulu yardımcıyı verilen argümanlarla UAC ile çalıştırır.
#[cfg(windows)]
async fn yardimciyi_calistir(argumanlar: Vec<std::ffi::OsString>) -> Result<Option<u32>, String> {
    // Yol her çağrıda yeniden doğrulanır; önceden bulunan yola güvenilmez.
    let yardimci = yardimci().ok_or("Kurulu yardımcı bulunamadı.")?;
    tokio::task::spawn_blocking(move || {
        let parcalar: Vec<&std::ffi::OsStr> = argumanlar.iter().map(|a| a.as_os_str()).collect();
        kalfa_sandbox::windows::sistem::yonetici_olarak_calistir(&yardimci, &parcalar)
            .map_err(|e| format!("{} başlatılamadı: {e}", yardimci.display()))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[async_trait]
impl Yukseltici for KuruluYukseltici {
    fn kurulu_mu(&self) -> bool {
        #[cfg(windows)]
        {
            yardimci().is_some()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    async fn kur(&self, izinli: &[PathBuf]) -> Result<Option<u32>, String> {
        #[cfg(windows)]
        {
            yardimciyi_calistir(kur_argumanlari(izinli)).await
        }
        #[cfg(not(windows))]
        {
            let _ = izinli;
            Err("Onar yalnızca Windows'ta kurulu sürümde çalışır.".into())
        }
    }

    async fn izin_ver(&self, dizin: &Path) -> Result<Option<u32>, String> {
        #[cfg(windows)]
        {
            yardimciyi_calistir(vec!["izin-ver".into(), dizin.as_os_str().to_owned()]).await
        }
        #[cfg(not(windows))]
        {
            let _ = dizin;
            Err("Yönetici izni yalnızca Windows'ta kurulu sürümde istenir.".into())
        }
    }
}
