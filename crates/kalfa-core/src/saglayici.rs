//! Model sağlayıcı soyutlaması. Çekirdek EVREN'e bu trait üzerinden konuşur;
//! testler sahte bir sağlayıcıyla koşar (spec §4.2).

use async_trait::async_trait;
use kalfa_evren::tipler::{ChatRequest, TermsStatus, TermsText};
use kalfa_evren::{AkisKanali, EvrenError, EvrenIstemcisi, YanitMeta};

pub struct SaglayiciAkisi {
    pub meta: YanitMeta,
    pub olaylar: AkisKanali,
}

#[async_trait]
pub trait Saglayici: Send + Sync {
    async fn akis(&self, istek: ChatRequest) -> Result<SaglayiciAkisi, EvrenError>;
    async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError>;
    async fn sart_metni(&self) -> Result<TermsText, EvrenError>;
    async fn sart_kabul(&self, surum: u32) -> Result<(), EvrenError>;
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

    async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError> {
        self.istemci.sart_durumu().await
    }

    async fn sart_metni(&self) -> Result<TermsText, EvrenError> {
        self.istemci.sart_metni().await
    }

    async fn sart_kabul(&self, surum: u32) -> Result<(), EvrenError> {
        self.istemci.sart_kabul(surum).await
    }
}
