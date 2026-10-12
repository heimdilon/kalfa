//! Senaryolu sahte EVREN. Anahtara göre davranır; şart durumu uygulamanın
//! bütün sağlayıcılarında ortaktır.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use kalfa_core::saglayici::{Saglayici, SaglayiciAkisi};
use kalfa_core::test_destek::{sahte_ek_yanit, sahte_modeller};
use kalfa_evren::tipler::{
    ChatMessage, ChatRequest, ChatResponse, Choice, ModelInfo, Role, TermsStatus, TermsText,
};
use kalfa_evren::{EkUc, EvrenError, HamYanit, YanitMeta};

use super::senaryo::{self, Adim, Senaryo};
use super::{E2eAyarlari, SartBaslangici};
use crate::evren::SaglayiciUretici;

/// Bu anahtarla her istek 401 döner (arayüzün sahte arka ucundaki adla aynı).
pub const GECERSIZ_ANAHTAR: &str = "evren_llm_gecersiz";
/// Bu anahtarla EVREN'e ulaşılamaz.
pub const AGSIZ_ANAHTAR: &str = "evren_llm_agsiz";
/// Denetçinin sahtesi bu işareti taşıyan içeriği şüpheli bulur (3c, e2e).
pub const SUPHELI_ISARETI: &str = "e2e-supheli";

const SART_SURUMU: u32 = 1;
const SART_METNI: &str =
    "# Sahte kullanım şartları\n\nBu metin uçtan uca test derlemesinin sahte EVREN'inden gelir.";

struct SahteEvren {
    senaryolar: HashMap<String, Senaryo>,
    sart_kabul: Mutex<bool>,
    istek_sayaci: AtomicU64,
}

impl SahteEvren {
    fn kabul(&self) -> MutexGuard<'_, bool> {
        self.sart_kabul.lock().unwrap_or_else(|z| z.into_inner())
    }
}

/// Anahtardan sahte sağlayıcı üretir.
pub struct SahteUretici(Arc<SahteEvren>);

impl SahteUretici {
    pub fn yeni(ayarlar: &E2eAyarlari) -> Self {
        Self(Arc::new(SahteEvren {
            senaryolar: ayarlar.senaryolar.clone(),
            sart_kabul: Mutex::new(ayarlar.sart == SartBaslangici::Kabul),
            istek_sayaci: AtomicU64::new(1),
        }))
    }
}

impl SaglayiciUretici for SahteUretici {
    fn uret(&self, anahtar: &str) -> Result<Arc<dyn Saglayici>, EvrenError> {
        Ok(Arc::new(SahteSaglayici {
            evren: self.0.clone(),
            anahtar: anahtar.to_string(),
        }))
    }
}

struct SahteSaglayici {
    evren: Arc<SahteEvren>,
    anahtar: String,
}

impl SahteSaglayici {
    fn denetle(&self) -> Result<(), EvrenError> {
        match self.anahtar.as_str() {
            GECERSIZ_ANAHTAR => Err(EvrenError::AnahtarGecersiz),
            AGSIZ_ANAHTAR => Err(EvrenError::ag_baglanti("sahte EVREN'e ulaşılamıyor")),
            _ => Ok(()),
        }
    }
}

/// İstekteki son kullanıcı mesajı, turdaki model isteğinin sırası (0'dan) ve
/// turun son araç sonucu.
fn istek_bilgisi(istek: &ChatRequest) -> (String, usize, Option<String>) {
    let Some(son) = istek.messages.iter().rposition(|m| m.role == Role::User) else {
        return (String::new(), 0, None);
    };
    let sonrasi = &istek.messages[son + 1..];
    let sira = sonrasi.iter().filter(|m| m.role == Role::Assistant).count();
    let arac = sonrasi
        .iter()
        .rev()
        .find(|m| m.role == Role::Tool)
        .and_then(|m| m.content.clone());
    let metin = istek.messages[son].content.clone().unwrap_or_default();
    (metin.trim().to_string(), sira, arac)
}

#[async_trait]
impl Saglayici for SahteSaglayici {
    async fn akis(&self, istek: ChatRequest) -> Result<SaglayiciAkisi, EvrenError> {
        self.denetle()?;
        if !*self.evren.kabul() {
            return Err(EvrenError::SartKabulGerekli {
                surum: Some(SART_SURUMU),
            });
        }
        let (metin, sira, arac) = istek_bilgisi(&istek);
        let adimlar = match self.evren.senaryolar.get(&metin) {
            Some(senaryo) => senaryo.yanitlar.get(sira).cloned().unwrap_or_else(|| {
                vec![Adim::Metin {
                    metin: "Tamamlandı.".into(),
                }]
            }),
            None => vec![Adim::Metin {
                metin: format!("Yanıt: {metin}"),
            }],
        };
        let no = self.evren.istek_sayaci.fetch_add(1, Ordering::Relaxed);
        Ok(SaglayiciAkisi {
            meta: YanitMeta {
                istek_id: Some(format!("e2e-istek-{no}")),
                yonlendirilen_model: Some(istek.model.clone()),
                kredi: None,
            },
            olaylar: senaryo::akis(adimlar, arac),
        })
    }

    async fn sohbet(&self, istek: ChatRequest) -> Result<(ChatResponse, YanitMeta), EvrenError> {
        self.denetle()?;
        let no = self.evren.istek_sayaci.fetch_add(1, Ordering::Relaxed);
        // Denetçinin sahtesi: denetlenen içerik işareti taşıyorsa şüpheli bulur.
        let supheli = istek.messages.iter().any(|m| {
            m.content
                .as_deref()
                .is_some_and(|c| c.contains(SUPHELI_ISARETI))
        });
        let govde = if supheli {
            r#"{"guvenli":false,"kategori":"talimat_enjeksiyonu","gerekce":"e2e sahte"}"#
        } else {
            r#"{"guvenli":true,"kategori":"yok","gerekce":"e2e sahte"}"#
        };
        let yanit = ChatResponse {
            id: format!("e2e-sohbet-{no}"),
            model: istek.model.clone(),
            choices: vec![Choice {
                index: 0,
                message: ChatMessage::assistant(Some(govde.into()), vec![]),
                finish_reason: Some("stop".into()),
            }],
            usage: None,
        };
        Ok((
            yanit,
            YanitMeta {
                istek_id: Some(format!("e2e-sohbet-{no}")),
                yonlendirilen_model: Some(istek.model),
                kredi: None,
            },
        ))
    }

    async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError> {
        self.denetle()?;
        Ok(TermsStatus {
            current_version: SART_SURUMU,
            is_material: true,
            accepted: *self.evren.kabul(),
            doc_path: None,
        })
    }

    async fn sart_metni(&self) -> Result<TermsText, EvrenError> {
        self.denetle()?;
        Ok(TermsText {
            version: SART_SURUMU,
            content: SART_METNI.into(),
            doc_path: None,
        })
    }

    async fn sart_kabul(&self, surum: u32) -> Result<(), EvrenError> {
        self.denetle()?;
        if surum != SART_SURUMU {
            return Err(EvrenError::SartKabulGerekli {
                surum: Some(SART_SURUMU),
            });
        }
        *self.evren.kabul() = true;
        Ok(())
    }

    /// Bir sohbet, bir gömme ve bir yeniden sıralama modeli (aşama 4).
    async fn modeller(&self) -> Result<Vec<ModelInfo>, EvrenError> {
        self.denetle()?;
        Ok(sahte_modeller())
    }

    /// Eklentinin gömme ve yeniden sıralama istekleri: metnin özetinden
    /// belirlenimli vektör ve kelime örtüşmesine göre puan (`sahte_ek_yanit`).
    async fn ek_uc(&self, uc: EkUc, govde: &serde_json::Value) -> Result<HamYanit, EvrenError> {
        self.denetle()?;
        if !*self.evren.kabul() {
            return Err(EvrenError::SartKabulGerekli {
                surum: Some(SART_SURUMU),
            });
        }
        let no = self.evren.istek_sayaci.fetch_add(1, Ordering::Relaxed);
        let model = format!("qwen/{}", govde["model"].as_str().unwrap_or_default());
        Ok(HamYanit {
            govde: sahte_ek_yanit(uc, govde).to_string(),
            model: Some(model.clone()),
            meta: YanitMeta {
                istek_id: Some(format!("e2e-ek-{no}")),
                yonlendirilen_model: Some(model),
                kredi: None,
            },
        })
    }
}
