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
    #[error("Yanıt akışı yarıda kaldı; model yanıtı tamamlanmadı.")]
    AkisYarimKaldi,
    #[error("Sağlayıcı yanıtı boyut, olay veya araç çağrısı sınırını aştı; yanıt durduruldu.")]
    YanitSiniri,
    #[error("EVREN'den beklenmeyen yanıt: {0}")]
    GecersizYanit(String),
    #[error("EVREN'e ulaşılamadı: {0}")]
    Ag(String),
    #[error("EVREN hatası ({durum}): {mesaj}")]
    Sunucu {
        durum: u16,
        kod: Option<String>,
        mesaj: String,
    },
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
    if durum == 429 {
        let tekrar_dene = retry_after
            .and_then(|v| v.trim().parse::<u64>().ok())
            .map(Duration::from_secs);
        return EvrenError::HizSiniri { tekrar_dene };
    }
    EvrenError::Sunucu { durum, kod, mesaj }
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
