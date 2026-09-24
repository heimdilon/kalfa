//! Senaryolu sahte EVREN. Anahtara göre davranır; şart durumu uygulamanın
//! bütün sağlayıcılarında ortaktır.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use kalfa_core::saglayici::{Saglayici, SaglayiciAkisi};
use kalfa_evren::tipler::{ChatRequest, Role, TermsStatus, TermsText};
use kalfa_evren::{EvrenError, YanitMeta};

use super::senaryo::{self, Adim, Senaryo};
use super::{E2eAyarlari, SartBaslangici};
use crate::evren::SaglayiciUretici;

/// Bu anahtarla her istek 401 döner (arayüzün sahte arka ucundaki adla aynı).
pub const GECERSIZ_ANAHTAR: &str = "evren_llm_gecersiz";
/// Bu anahtarla EVREN'e ulaşılamaz.
pub const AGSIZ_ANAHTAR: &str = "evren_llm_agsiz";

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
            AGSIZ_ANAHTAR => Err(EvrenError::Ag("sahte EVREN'e ulaşılamıyor".into())),
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
}
