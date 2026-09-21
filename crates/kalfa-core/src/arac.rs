//! Araç sözleşmesi (spec §6, §7). Yerleşik araç ile eklenti aracı arasında
//! izin açısından fark yoktur: ikisi de bu trait'in arkasındadır.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use kalfa_evren::tipler::ToolDef;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::izin::Kip;
use crate::yol::{CalismaYolu, YolHatasi};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YazmaHedefi {
    pub yol: CalismaYolu,
    /// Dosyanın şu anki içeriğinin SHA-256 özeti; dosya yoksa `None`.
    pub onceki_ozet: Option<String>,
    /// Hard link sayısı; dosya yoksa 1.
    pub baglanti_sayisi: u64,
}

/// Yürütmeden önce aracın ne yapacağının tarifi: izin kararı, onay kartı ve
/// onay mührü bundan üretilir.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hazirlik {
    pub okunan: Vec<CalismaYolu>,
    pub yazilan: Vec<YazmaHedefi>,
    /// Hangi yollara dokunacağı önceden bilinemeyen iş (kabuk, aşama 2).
    pub tum_calisma_alani: bool,
    /// Sade Türkçe tek cümle: "`a.txt` dosyasını okumak istiyor."
    pub aciklama: String,
    pub diff: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AracCiktisi {
    pub icerik: String,
    /// Modele giden içeriğin kaynak etiketi (spec §9), örn. `dosya:src/a.rs:1-40`.
    pub kaynak: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AracHatasi {
    #[error("{0}")]
    Gecersiz(String),
    #[error(transparent)]
    Yol(#[from] YolHatasi),
    #[error("Dosya onaydan sonra değişti; işlem yapılmadı.")]
    OnayEskidi,
    #[error(
        "İzin kipi veya hedefin izin koşulları değişti; işlem yapılmadı. Gerekirse yeniden dene."
    )]
    PolitikaDegisti,
    #[error("İptal edildi.")]
    Iptal,
    #[error("{0}")]
    GirisCikis(String),
}

impl From<std::io::Error> for AracHatasi {
    fn from(hata: std::io::Error) -> Self {
        AracHatasi::GirisCikis(hata.to_string())
    }
}

#[async_trait]
pub trait Arac: Send + Sync {
    fn ad(&self) -> &str;
    fn aciklama(&self) -> &str;
    fn parametre_semasi(&self) -> Value;
    /// Dosya yazar ya da yan etkisi olan bir iş yapar mı? İnceleme kipinde
    /// yazan araçlar modele hiç sunulmaz (spec §7.5).
    fn yazar_mi(&self) -> bool;
    async fn hazirla(&self, argumanlar: &Value) -> Result<Hazirlik, AracHatasi>;
    async fn calistir(
        &self,
        argumanlar: &Value,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi>;
}

#[derive(Debug, thiserror::Error)]
pub enum KayitHatasi {
    #[error("`{0}` adlı araç iki kez kaydedildi.")]
    Yinelenen(String),
    #[error("`{arac}` aracının parametre şeması geçersiz: {hata}")]
    GecersizSema { arac: String, hata: String },
}

pub struct AracKayitDefteri {
    araclar: Vec<Arc<dyn Arac>>,
    semalar: HashMap<String, jsonschema::Validator>,
}

impl AracKayitDefteri {
    pub fn yeni(araclar: Vec<Arc<dyn Arac>>) -> Result<Self, KayitHatasi> {
        let mut semalar = HashMap::new();
        for arac in &araclar {
            if semalar.contains_key(arac.ad()) {
                return Err(KayitHatasi::Yinelenen(arac.ad().to_string()));
            }
            let dogrulayici = jsonschema::validator_for(&arac.parametre_semasi()).map_err(|e| {
                KayitHatasi::GecersizSema {
                    arac: arac.ad().to_string(),
                    hata: e.to_string(),
                }
            })?;
            semalar.insert(arac.ad().to_string(), dogrulayici);
        }
        Ok(Self { araclar, semalar })
    }

    pub fn bul(&self, ad: &str) -> Option<Arc<dyn Arac>> {
        self.araclar.iter().find(|a| a.ad() == ad).cloned()
    }

    pub fn tanimlar(&self, kip: Kip) -> Vec<ToolDef> {
        self.araclar
            .iter()
            .filter(|a| kip != Kip::Inceleme || !a.yazar_mi())
            .map(|a| ToolDef::function(a.ad(), a.aciklama(), a.parametre_semasi()))
            .collect()
    }

    /// Argüman metnini JSON olarak çözer ve aracın şemasına karşı doğrular.
    /// Hata metni modele gider; model argümanı düzeltip yeniden deneyebilir.
    pub fn dogrula(&self, ad: &str, argumanlar: &str) -> Result<Value, String> {
        let dogrulayici = self
            .semalar
            .get(ad)
            .ok_or_else(|| format!("`{ad}` adlı bir araç yok."))?;
        let metin = if argumanlar.trim().is_empty() {
            "{}"
        } else {
            argumanlar
        };
        let deger: Value = serde_json::from_str(metin)
            .map_err(|e| format!("Argümanlar geçerli JSON değil: {e}"))?;
        let hatalar: Vec<String> = dogrulayici
            .iter_errors(&deger)
            .map(|e| format!("{}: {}", e.instance_path(), e))
            .collect();
        if hatalar.is_empty() {
            Ok(deger)
        } else {
            Err(format!("Argümanlar şemaya uymuyor: {}", hatalar.join("; ")))
        }
    }
}
