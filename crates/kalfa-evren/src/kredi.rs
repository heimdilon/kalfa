//! Yanıt başlıklarından kredi gözlemi (spec §3.4, §5.2).
//!
//! Kalfa kredi hakkında yalnızca **gözlem** bildirir: kesinleşmiş harcama
//! API'den gözlenemiyor.

use std::str::FromStr;

use reqwest::header::HeaderMap;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub const KALAN: &str = "x-evren-credits-remaining";
pub const TUTULAN: &str = "x-evren-credits-held";
pub const YONLENDIRILEN_MODEL: &str = "x-evren-routed-model";
pub const ISTEK_KIMLIGI: &str = "x-request-id";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KrediGozlemi {
    pub kalan: Option<Decimal>,
    pub tutulan: Option<Decimal>,
    pub istek_id: Option<String>,
    /// Gözlem zamanı, Unix epoch milisaniyesi.
    pub zaman_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct YanitMeta {
    pub istek_id: Option<String>,
    pub yonlendirilen_model: Option<String>,
    /// Kredi başlıklarından hiçbiri yoksa `None`.
    pub kredi: Option<KrediGozlemi>,
}

pub fn basliklardan(basliklar: &HeaderMap, zaman_ms: u64) -> YanitMeta {
    let metin = |ad: &str| {
        basliklar
            .get(ad)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };
    let ondalik = |ad: &str| metin(ad).and_then(|s| Decimal::from_str(&s).ok());

    let istek_id = metin(ISTEK_KIMLIGI);
    let kalan = ondalik(KALAN);
    let tutulan = ondalik(TUTULAN);
    let kredi = (kalan.is_some() || tutulan.is_some()).then(|| KrediGozlemi {
        kalan,
        tutulan,
        istek_id: istek_id.clone(),
        zaman_ms,
    });
    YanitMeta {
        istek_id,
        yonlendirilen_model: metin(YONLENDIRILEN_MODEL),
        kredi,
    }
}
