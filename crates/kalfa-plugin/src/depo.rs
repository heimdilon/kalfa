//! Eklentiye ve çalışma alanına özel anahtar-değer deposu (aşama 4 tasarımı
//! §10).
//!
//! Her anahtar bir dosyadır; adı anahtarın SHA-256'sıdır. Yazma atomiktir:
//! aynı dizinde geçici dosya, sonra yeniden adlandırma. Kota, dizindeki
//! dosyaların toplam boyutudur; çağrının başında bir kez sayılır ve yazdıkça
//! güncellenir. Bir dizinin aynı anda tek yazarı vardır (§7.1).

use std::io;
use std::path::{Path, PathBuf};

use crate::bag::kalfa::plugin::tipler::Hata;

pub const AZAMI_ANAHTAR: usize = 200;
pub const AZAMI_DEGER: usize = 64 * 1024 * 1024;

pub(crate) struct Depo {
    dizin: PathBuf,
    /// Bayt.
    kota: u64,
    kota_mb: u32,
    /// Dizindeki dosyaların toplam boyutu; ilk kullanımda sayılır.
    kullanilan: Option<u64>,
}

fn dosya_adi(anahtar: &str) -> Result<String, Hata> {
    if anahtar.is_empty() || anahtar.len() > AZAMI_ANAHTAR {
        return Err(Hata::Diger(format!(
            "depo anahtarı 1–{AZAMI_ANAHTAR} bayt olmalı"
        )));
    }
    Ok(kalfa_core::muhur::ozet_hex(anahtar.as_bytes()))
}

fn yazilamadi(e: io::Error) -> Hata {
    Hata::Diger(format!("depo yazılamadı: {e}"))
}

/// Görev çalışamadıysa (çalışma zamanı kapanıyor) hata.
fn gorev_hatasi(e: tokio::task::JoinError) -> Hata {
    Hata::Diger(format!("depo işlemi tamamlanamadı: {e}"))
}

impl Depo {
    pub(crate) fn yeni(dizin: PathBuf, kota_mb: u32) -> Self {
        Self {
            dizin,
            kota: u64::from(kota_mb) * 1024 * 1024,
            kota_mb,
            kullanilan: None,
        }
    }

    pub(crate) fn dizin(&self) -> &Path {
        &self.dizin
    }

    async fn kullanilan(&mut self) -> Result<u64, Hata> {
        if let Some(k) = self.kullanilan {
            return Ok(k);
        }
        let dizin = self.dizin.clone();
        let k = tokio::task::spawn_blocking(move || dizin_boyutu(&dizin))
            .await
            .map_err(gorev_hatasi)?
            .map_err(|e| Hata::Diger(format!("depo okunamadı: {e}")))?;
        self.kullanilan = Some(k);
        Ok(k)
    }

    pub(crate) async fn oku(&mut self, anahtar: &str) -> Result<Option<Vec<u8>>, Hata> {
        let yol = self.dizin.join(dosya_adi(anahtar)?);
        tokio::task::spawn_blocking(move || match std::fs::read(&yol) {
            Ok(v) => Ok(Some(v)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Hata::Diger(format!("depo okunamadı: {e}"))),
        })
        .await
        .map_err(gorev_hatasi)?
    }

    pub(crate) async fn yaz(&mut self, anahtar: &str, deger: Vec<u8>) -> Result<(), Hata> {
        let ad = dosya_adi(anahtar)?;
        if deger.len() > AZAMI_DEGER {
            return Err(Hata::SinirAsildi(
                "depo değeri en çok 64 MiB olabilir".into(),
            ));
        }
        let kullanilan = self.kullanilan().await?;
        let yol = self.dizin.join(&ad);
        let eski = boyut(&yol).await?;
        let yeni = kullanilan - eski.min(kullanilan) + deger.len() as u64;
        if yeni > self.kota {
            return Err(Hata::SinirAsildi(format!(
                "depo kotası ({} MB) doldu",
                self.kota_mb
            )));
        }
        let dizin = self.dizin.clone();
        tokio::task::spawn_blocking(move || atomik_yaz(&dizin, &ad, &deger))
            .await
            .map_err(gorev_hatasi)?
            .map_err(yazilamadi)?;
        self.kullanilan = Some(yeni);
        Ok(())
    }

    pub(crate) async fn sil(&mut self, anahtar: &str) -> Result<(), Hata> {
        let kullanilan = self.kullanilan().await?;
        let yol = self.dizin.join(dosya_adi(anahtar)?);
        let eski = boyut(&yol).await?;
        tokio::task::spawn_blocking(move || match std::fs::remove_file(&yol) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(yazilamadi(e)),
            _ => Ok(()),
        })
        .await
        .map_err(gorev_hatasi)??;
        self.kullanilan = Some(kullanilan - eski.min(kullanilan));
        Ok(())
    }
}

async fn boyut(yol: &Path) -> Result<u64, Hata> {
    let yol = yol.to_path_buf();
    tokio::task::spawn_blocking(move || match std::fs::metadata(&yol) {
        Ok(m) => Ok(m.len()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(Hata::Diger(format!("depo okunamadı: {e}"))),
    })
    .await
    .map_err(gorev_hatasi)?
}

/// Dizindeki dosyaların toplam boyutu; dizin yoksa 0.
pub fn dizin_boyutu(dizin: &Path) -> io::Result<u64> {
    let girdiler = match std::fs::read_dir(dizin) {
        Ok(g) => g,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e),
    };
    let mut toplam = 0;
    for girdi in girdiler {
        let girdi = girdi?;
        if girdi.file_type()?.is_file() {
            toplam += girdi.metadata()?.len();
        }
    }
    Ok(toplam)
}

fn atomik_yaz(dizin: &Path, ad: &str, deger: &[u8]) -> io::Result<()> {
    dizin_olustur(dizin)?;
    let gecici = dizin.join(format!("{ad}.yeni"));
    std::fs::write(&gecici, deger)?;
    std::fs::rename(&gecici, dizin.join(ad))
}

/// Dizin yalnızca kullanıcıya açıktır (Linux'ta 0700). Windows'ta yerel veri
/// dizininin izinlerini miras alır.
fn dizin_olustur(dizin: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dizin)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(dizin)
    }
}
