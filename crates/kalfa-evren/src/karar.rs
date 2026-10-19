//! EVREN `/v1/systemone` karar istemcisi tipleri (otomatik kip tasarımı §4).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// `/v1/systemone` karar sorusunun türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KararTuru {
    Noul,
    Choice,
}

/// `/v1/systemone` isteği.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KararIstegi {
    pub model: String,
    pub state: serde_json::Value,
    pub questions: BTreeMap<String, KararSorusu>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_len: Option<u32>,
}

/// `/v1/systemone` içindeki tek bir soru tanımı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KararSorusu {
    #[serde(rename = "type")]
    pub tur: KararTuru,
    pub instructions: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
}

/// `/v1/systemone` yanıtındaki tek bir sorunun cevabı.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum KararCevabi {
    Noul {
        noul: f64,
    },
    Choice {
        choice: String,
        confidence: f64,
        probabilities: BTreeMap<String, f64>,
    },
    #[serde(other)]
    Diger,
}

/// `/v1/systemone` yanıtı.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct KararYaniti {
    #[serde(default)]
    pub id: Option<String>,
    pub model: String,
    pub answers: BTreeMap<String, KararCevabi>,
    #[serde(default)]
    pub usage: Option<KararKullanimi>,
}

/// `/v1/systemone` kullanım bilgisi (`input_tokens` ve `output_tokens`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct KararKullanimi {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
}
