//! "Onar"ın gerçeği (aşama 2 tasarımı §9.8): kurulu sürümde Program
//! Files'taki `kalfa-yalitim.exe kur`'u UAC ile çalıştırır. Geliştirme
//! derlemesinde ve Linux'ta kurulu yardımcı yoktur; Onar gösterilmez.

use crate::arayuz::Yukseltici;
use async_trait::async_trait;

pub struct KuruluYukseltici;

#[cfg(windows)]
fn yardimci() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    kalfa_sandbox::windows::sistem::kurulu_yardimci(&exe)
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

    async fn kur(&self) -> Result<Option<u32>, String> {
        #[cfg(windows)]
        {
            // Yol her çağrıda yeniden doğrulanır; önceden bulunan yola
            // güvenilmez.
            let yardimci = yardimci().ok_or("Kurulu yardımcı bulunamadı.")?;
            tokio::task::spawn_blocking(move || {
                kalfa_sandbox::windows::sistem::yonetici_olarak_calistir(&yardimci, "kur")
                    .map_err(|e| format!("{} başlatılamadı: {e}", yardimci.display()))
            })
            .await
            .map_err(|e| e.to_string())?
        }
        #[cfg(not(windows))]
        {
            Err("Onar yalnızca Windows'ta kurulu sürümde çalışır.".into())
        }
    }
}
