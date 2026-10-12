//! EVREN hata yanıtlarının tipli karşılığı (spec §5.4).

use std::time::Duration;

use crate::tipler::ApiErrorBody;

const AZAMI_MESAJ: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EvrenError {
    #[error(
        "Anahtar geçersiz ya da eksik. EVREN portalından anahtarınızı kontrol edip yeniden girin."
    )]
    AnahtarGecersiz,
    #[error("EVREN kullanım şartlarının kabul edilmesi gerekiyor.")]
    SartKabulGerekli { surum: Option<u32> },
    #[error("Model bulunamadı: {mesaj}")]
    ModelBulunamadi {
        mesaj: String,
        oneriler: Vec<String>,
    },
    #[error("EVREN istek sınırına ulaşıldı; biraz sonra yeniden denenecek.")]
    HizSiniri { tekrar_dene: Option<Duration> },
    /// Günlük token sınırı (`429 daily_token_limit_exceeded`). EVREN bunda da
    /// `Retry-After: 60` gönderir, ama sınır ancak `resets_at`'te açılır; bu
    /// yüzden yeniden denenmez.
    #[error(
        "EVREN'in günlük token sınırına ulaşıldı (bütün anahtarlarınızın toplamı).{}",
        sifirlanma_eki(.sifirlanma)
    )]
    GunlukSinir { sifirlanma: Option<String> },
    #[error("Yanıt akışı yarıda kaldı; model yanıtı tamamlanmadı.")]
    AkisYarimKaldi,
    #[error("Sağlayıcı yanıtı boyut, olay veya araç çağrısı sınırını aştı; yanıt durduruldu.")]
    YanitSiniri,
    #[error("EVREN'den beklenmeyen yanıt: {0}")]
    GecersizYanit(String),
    #[error("EVREN'e ulaşılamadı: {mesaj}")]
    Ag { mesaj: String, govde_gitti: bool },
    #[error("EVREN hatası ({durum}): {mesaj}")]
    Sunucu {
        durum: u16,
        kod: Option<String>,
        mesaj: String,
    },
}

impl EvrenError {
    pub fn ag_baglanti(mesaj: impl Into<String>) -> Self {
        Self::Ag {
            mesaj: mesaj.into(),
            govde_gitti: false,
        }
    }

    pub fn ag_yanit(mesaj: impl Into<String>) -> Self {
        Self::Ag {
            mesaj: mesaj.into(),
            govde_gitti: true,
        }
    }

    pub fn ag(mesaj: impl Into<String>, govde_gitti: bool) -> Self {
        Self::Ag {
            mesaj: mesaj.into(),
            govde_gitti,
        }
    }
}

/// HTTP durum kodu, `Retry-After` başlığı ve gövdeden tipli hata üretir.
pub fn hata_esle(durum: u16, retry_after: Option<&str>, govde: &[u8]) -> EvrenError {
    if govde.len() > crate::butce::JSON_BAYT_SINIRI {
        return EvrenError::YanitSiniri;
    }
    let (kod, mesaj, evren) = match serde_json::from_slice::<ApiErrorBody>(govde) {
        Ok(g) => (g.error.code, g.error.message, g.error.evren),
        Err(_) => (None, kirp(&String::from_utf8_lossy(govde)), None),
    };

    if durum == 401 || kod.as_deref() == Some("unauthorized") {
        return EvrenError::AnahtarGecersiz;
    }
    if kod.as_deref() == Some("terms_not_accepted") {
        return EvrenError::SartKabulGerekli {
            surum: evren.and_then(|e| e.required_terms_version),
        };
    }
    if kod.as_deref() == Some("model_not_found") {
        return EvrenError::ModelBulunamadi {
            mesaj,
            oneriler: evren.map(|e| e.suggested_models).unwrap_or_default(),
        };
    }
    if kod.as_deref() == Some("daily_token_limit_exceeded") {
        return EvrenError::GunlukSinir {
            sifirlanma: evren
                .and_then(|e| e.resets_at)
                .and_then(|z| z.as_str().map(str::to_owned))
                .filter(|z| zaman_damgasi_mi(z)),
        };
    }
    if durum == 429 {
        let tekrar_dene = retry_after
            .and_then(|v| v.trim().parse::<u64>().ok())
            .map(Duration::from_secs);
        return EvrenError::HizSiniri { tekrar_dene };
    }
    EvrenError::Sunucu { durum, kod, mesaj }
}

/// Sunucudan gelen değer kullanıcıya gösterilir: yalnızca RFC 3339 zaman
/// damgasının karakterleri (izin listesi) ve makul bir uzunluk kabul edilir.
fn zaman_damgasi_mi(z: &str) -> bool {
    !z.is_empty()
        && z.len() <= 40
        && z.chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '-' | ':' | 'T' | 'Z' | '.' | '+'))
}

fn sifirlanma_eki(sifirlanma: &Option<String>) -> String {
    match sifirlanma {
        Some(z) => format!(" Sınır {z} itibarıyla sıfırlanır."),
        None => String::new(),
    }
}

fn kirp(metin: &str) -> String {
    let t = metin.trim();
    if t.chars().count() <= AZAMI_MESAJ {
        t.to_string()
    } else {
        let mut k: String = t.chars().take(AZAMI_MESAJ).collect();
        k.push('…');
        k
    }
}
