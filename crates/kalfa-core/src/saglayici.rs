//! Model sağlayıcı soyutlaması. Çekirdek EVREN'e bu trait üzerinden konuşur;
//! testler sahte bir sağlayıcıyla koşar (spec §4.2).

use async_trait::async_trait;
use kalfa_evren::tipler::{ChatRequest, ChatResponse, ModelInfo, TermsStatus, TermsText};
use kalfa_evren::{AkisKanali, EkUc, EvrenError, EvrenIstemcisi, HamYanit, YanitMeta};

pub struct SaglayiciAkisi {
    pub meta: YanitMeta,
    pub olaylar: AkisKanali,
}

#[async_trait]
pub trait Saglayici: Send + Sync {
    async fn akis(&self, istek: ChatRequest) -> Result<SaglayiciAkisi, EvrenError>;
    async fn sohbet(&self, istek: ChatRequest) -> Result<(ChatResponse, YanitMeta), EvrenError>;
    async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError>;
    async fn sart_metni(&self) -> Result<TermsText, EvrenError>;
    async fn sart_kabul(&self, surum: u32) -> Result<(), EvrenError>;
    /// Model kataloğu (`/v1/models`). Eklentinin istediği modelin uca
    /// uyduğu bununla denetlenir (aşama 4 tasarımı §9.2).
    async fn modeller(&self) -> Result<Vec<ModelInfo>, EvrenError> {
        Err(EvrenError::GecersizYanit(
            "bu sağlayıcıda model kataloğu yok".into(),
        ))
    }
    /// Eklentinin gömme ya da yeniden sıralama isteği (aşama 4 tasarımı §9.1).
    async fn ek_uc(&self, _uc: EkUc, _govde: &serde_json::Value) -> Result<HamYanit, EvrenError> {
        Err(EvrenError::GecersizYanit(
            "bu sağlayıcı eklenti uçlarını desteklemiyor".into(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct EvrenSaglayici {
    istemci: EvrenIstemcisi,
}

impl EvrenSaglayici {
    pub fn yeni(istemci: EvrenIstemcisi) -> Self {
        Self { istemci }
    }
}

#[async_trait]
impl Saglayici for EvrenSaglayici {
    async fn akis(&self, istek: ChatRequest) -> Result<SaglayiciAkisi, EvrenError> {
        let (meta, olaylar) = self.istemci.sohbet_akisi(&istek).await?;
        Ok(SaglayiciAkisi { meta, olaylar })
    }

    async fn sohbet(&self, istek: ChatRequest) -> Result<(ChatResponse, YanitMeta), EvrenError> {
        self.istemci.sohbet(&istek).await
    }

    async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError> {
        self.istemci.sart_durumu().await
    }

    async fn sart_metni(&self) -> Result<TermsText, EvrenError> {
        self.istemci.sart_metni().await
    }

    async fn sart_kabul(&self, surum: u32) -> Result<(), EvrenError> {
        self.istemci.sart_kabul(surum).await
    }

    async fn modeller(&self) -> Result<Vec<ModelInfo>, EvrenError> {
        self.istemci.modeller().await
    }

    async fn ek_uc(&self, uc: EkUc, govde: &serde_json::Value) -> Result<HamYanit, EvrenError> {
        self.istemci.ek_uc(uc, govde).await
    }
}
