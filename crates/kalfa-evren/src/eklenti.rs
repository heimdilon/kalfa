//! Eklentilere açık EVREN uçları: gömme ve yeniden sıralama (aşama 4
//! tasarımı §9.1, §9.2). Gövde eklentiden gelir ve host'ta izin listesiyle
//! doğrulanmıştır; istemci onu olduğu gibi gönderir ve yanıtı ham metin olarak
//! döndürür. Tipler yalnızca günlük için gereken alanı (`model`) çözer.

use serde::{Deserialize, Serialize};

use crate::kredi::YanitMeta;

/// Eklentinin çağırabileceği uç. Başka yol, yöntem ya da host yoktur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EkUc {
    Gomme,
    Siralama,
}

impl EkUc {
    /// Taban adrese (`…/v1`) göre yol.
    pub fn yol(self) -> &'static str {
        match self {
            Self::Gomme => "embeddings",
            Self::Siralama => "rerank",
        }
    }

    /// Model kataloğunda (`/v1/models`) bu uca uyan `task`.
    pub fn gorev(self) -> &'static str {
        match self {
            Self::Gomme => "embedding",
            Self::Siralama => "rerank",
        }
    }

    /// Kaynak etiketinin son parçası: `eklenti:<ad>:gomme`.
    pub fn etiket(self) -> &'static str {
        match self {
            Self::Gomme => "gomme",
            Self::Siralama => "siralama",
        }
    }
}

/// Uçtan dönen başarılı yanıt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HamYanit {
    /// Yanıt gövdesi, olduğu gibi (UTF-8).
    pub govde: String,
    /// Yanıttaki `model` alanı; yoksa `None`.
    pub model: Option<String>,
    pub meta: YanitMeta,
}
