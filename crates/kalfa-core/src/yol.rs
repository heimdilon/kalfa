//! Çalışma alanına göre göreli, normalleştirilmiş yol (spec §7.3).
//!
//! Metinsel kontrol ilk savunmadır; asıl sınırı `kalfa-tools`'taki `cap-std`
//! dizin tutamacı çizer. Burada reddedilenler: mutlak yollar (UNC ve sürücü
//! göreli dahil), herhangi bir `..` bileşeni, `:` içeren adlar (Windows
//! alternatif veri akışı), Windows aygıt adları ve NUL karakteri.

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const AYGIT_ADLARI: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum YolHatasi {
    #[error("Yol boş.")]
    Bos,
    #[error("Mutlak yol kullanılamaz; çalışma alanına göre göreli bir yol ver: {0}")]
    Mutlak(String),
    #[error("`..` ile üst dizine çıkılamaz: {0}")]
    UstDizin(String),
    #[error("Dosya adında `:` olamaz (alternatif veri akışı): {0}")]
    AlternatifVeriAkisi(String),
    #[error("Windows aygıt adı dosya adı olarak kullanılamaz: {0}")]
    AygitAdi(String),
    #[error("Yolda geçersiz karakter var: {0}")]
    GecersizKarakter(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CalismaYolu(String);

impl CalismaYolu {
    pub fn coz(girdi: &str) -> Result<Self, YolHatasi> {
        if girdi.contains('\0') {
            return Err(YolHatasi::GecersizKarakter(girdi.replace('\0', "\\0")));
        }
        let duz = girdi.replace('\\', "/");
        let bayt = duz.as_bytes();
        let surucu = bayt.len() >= 2 && bayt[0].is_ascii_alphabetic() && bayt[1] == b':';
        if duz.starts_with('/') || surucu {
            return Err(YolHatasi::Mutlak(girdi.to_string()));
        }
        let mut parcalar = Vec::new();
        for parca in duz.split('/') {
            match parca {
                "" | "." => continue,
                ".." => return Err(YolHatasi::UstDizin(girdi.to_string())),
                _ => {}
            }
            if parca.contains(':') {
                return Err(YolHatasi::AlternatifVeriAkisi(girdi.to_string()));
            }
            // Windows "con.txt" ve "aux " adlarını da aygıt sayar.
            let kok = parca
                .split('.')
                .next()
                .unwrap_or("")
                .trim_end()
                .to_ascii_uppercase();
            if AYGIT_ADLARI.contains(&kok.as_str()) {
                return Err(YolHatasi::AygitAdi(girdi.to_string()));
            }
            parcalar.push(parca);
        }
        if parcalar.is_empty() {
            return Err(YolHatasi::Bos);
        }
        Ok(Self(parcalar.join("/")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Karşılaştırma ve kilit anahtarı. Windows'ta büyük-küçük harf farklı iki
    /// yazım aynı dosyadır; güvenli tarafta kalmak için her platformda
    /// duyarsız karşılaştırılır.
    pub fn anahtar(&self) -> String {
        self.0.to_lowercase()
    }

    pub fn std_yolu(&self) -> PathBuf {
        self.0.split('/').collect()
    }

    pub fn ust(&self) -> Option<CalismaYolu> {
        self.0
            .rsplit_once('/')
            .map(|(ust, _)| Self(ust.to_string()))
    }
}

impl fmt::Display for CalismaYolu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for CalismaYolu {
    type Error = YolHatasi;

    fn try_from(deger: String) -> Result<Self, Self::Error> {
        Self::coz(&deger)
    }
}

impl From<CalismaYolu> for String {
    fn from(yol: CalismaYolu) -> String {
        yol.0
    }
}
