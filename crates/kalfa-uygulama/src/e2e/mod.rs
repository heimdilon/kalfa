//! Uçtan uca test derlemesinin parçaları (1c tasarımı §11.4): senaryolu sahte
//! sağlayıcı, sahte yerel iletişim kutuları, aktarım sayaçları ve uzun kayıt
//! üreteci. Yalnızca `e2e` özelliğiyle derlenir; sürüm derlemesine girmez.
//!
//! Test koşucusu `KALFA_E2E` ortam değişkeniyle bir JSON ayar dosyası verir.

mod kayit;
mod olcum;
mod saglayici;
mod senaryo;

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::Deserialize;

use crate::arayuz::Iletisim;

pub use olcum::{Olcumler, OlcumluKaynak, OlcumluYazici, olcumleri_sifirla};
pub use saglayici::{AGSIZ_ANAHTAR, GECERSIZ_ANAHTAR, SahteUretici};
pub use senaryo::{Adim, DAMGA_ONEKI, Desen, Senaryo};

/// Ayar dosyasının yolunu taşıyan ortam değişkeni.
pub const AYAR_DEGISKENI: &str = "KALFA_E2E";

/// Test koşucusunun verdiği ayarlar.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct E2eAyarlari {
    /// Sahte EVREN'in açılıştaki şart durumu.
    #[serde(default)]
    pub sart: SartBaslangici,
    /// Açılışta kasaya konan anahtar; yoksa şart penceresi anahtar ister.
    #[serde(default)]
    pub anahtar: Option<String>,
    /// Klasör seçicinin döneceği çalışma alanı; yoksa seçimden vazgeçilir.
    #[serde(default)]
    pub calisma_alani: Option<PathBuf>,
    /// Yerel onay kutularının cevabı (izin kipi yükseltme, bağlantı açma).
    #[serde(default)]
    pub onay: bool,
    /// Kullanıcı mesajının metniyle seçilen senaryolar. Eşleşmeyen mesaja
    /// "Yanıt: <mesaj>" döner.
    #[serde(default)]
    pub senaryolar: HashMap<String, Senaryo>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SartBaslangici {
    /// Şartlar kabul edilmemiş; ilk açılışta metin gösterilir.
    #[default]
    Gerekli,
    /// Şartlar daha önce kabul edilmiş.
    Kabul,
}

impl E2eAyarlari {
    pub fn oku(yol: &Path) -> io::Result<Self> {
        let metin = std::fs::read(yol)?;
        serde_json::from_slice(&metin).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// `KALFA_E2E` tanımlıysa ayar dosyasını okur, değilse varsayılanları
    /// döner.
    pub fn ortamdan() -> io::Result<Self> {
        match std::env::var_os(AYAR_DEGISKENI) {
            Some(yol) => Self::oku(Path::new(&yol)),
            None => Ok(Self::default()),
        }
    }
}

/// Yerel iletişim kutularının yerine geçer: WebDriver işletim sisteminin
/// kutularını süremez.
pub struct SahteIletisim {
    calisma_alani: Option<PathBuf>,
    onay: bool,
}

impl SahteIletisim {
    pub fn yeni(ayarlar: &E2eAyarlari) -> Self {
        Self {
            calisma_alani: ayarlar.calisma_alani.clone(),
            onay: ayarlar.onay,
        }
    }
}

#[async_trait]
impl Iletisim for SahteIletisim {
    async fn klasor_sec(&self) -> Option<PathBuf> {
        self.calisma_alani.clone()
    }

    async fn onayla(&self, _baslik: &str, _mesaj: &str, _tamam: &str, _vazgec: &str) -> bool {
        self.onay
    }
}
