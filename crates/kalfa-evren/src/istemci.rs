//! EVREN LLM Gateway HTTP istemcisi (spec §3, §5).

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::RETRY_AFTER;
use serde::de::DeserializeOwned;

use crate::hata::{EvrenError, hata_esle};
use crate::kredi::{YanitMeta, basliklardan};
use crate::tipler::{ChatRequest, ChatResponse, ModelInfo, ModelList, TermsStatus, TermsText};

pub const VARSAYILAN_TABAN: &str = "https://evren-llmapi.ssyz.org.tr/v1";

/// API anahtarı. Hiçbir `Debug`/`Display` çıktısında görünmez.
pub struct ApiAnahtari(String);

impl ApiAnahtari {
    pub fn yeni(deger: impl Into<String>) -> Self {
        Self(deger.into())
    }

    fn deger(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiAnahtari {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiAnahtari(***)")
    }
}

#[derive(Clone)]
pub struct EvrenIstemcisi {
    pub(crate) taban: String,
    pub(crate) anahtar: Arc<ApiAnahtari>,
    pub(crate) http: reqwest::Client,
}

impl fmt::Debug for EvrenIstemcisi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EvrenIstemcisi")
            .field("taban", &self.taban)
            .finish_non_exhaustive()
    }
}

impl EvrenIstemcisi {
    pub fn yeni(taban: impl Into<String>, anahtar: ApiAnahtari) -> Result<Self, EvrenError> {
        // reqwest `rustls-no-provider` ile derlendi; süreç genelinde ring'i kur.
        // Zaten kuruluysa hata döner; yok sayılır.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(180))
            .user_agent(concat!("kalfa-evren/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| EvrenError::Ag(e.to_string()))?;
        Ok(Self {
            taban: taban.into().trim_end_matches('/').to_string(),
            anahtar: Arc::new(anahtar),
            http,
        })
    }

    pub(crate) fn url(&self, yol: &str) -> String {
        format!("{}/{}", self.taban, yol)
    }

    /// İsteği kimlikle gönderir; başarısız durum kodunu tipli hataya çevirir.
    pub(crate) async fn gonder(
        &self,
        istek: reqwest::RequestBuilder,
    ) -> Result<(reqwest::Response, YanitMeta), EvrenError> {
        let yanit = istek
            .bearer_auth(self.anahtar.deger())
            .send()
            .await
            .map_err(ag_hatasi)?;
        let meta = basliklardan(yanit.headers(), simdi_ms());
        if yanit.status().is_success() {
            return Ok((yanit, meta));
        }
        let durum = yanit.status().as_u16();
        let retry_after = yanit
            .headers()
            .get(RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let govde = yanit.bytes().await.map_err(ag_hatasi)?;
        Err(hata_esle(durum, retry_after.as_deref(), &govde))
    }

    async fn json_al<T: DeserializeOwned>(&self, yol: &str) -> Result<T, EvrenError> {
        let (yanit, _) = self.gonder(self.http.get(self.url(yol))).await?;
        json_coz(yanit).await
    }

    pub async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError> {
        self.json_al("terms/status").await
    }

    pub async fn sart_metni(&self) -> Result<TermsText, EvrenError> {
        self.json_al("terms/text").await
    }

    pub async fn sart_kabul(&self, surum: u32) -> Result<(), EvrenError> {
        let istek = self
            .http
            .post(self.url("terms/accept"))
            .json(&serde_json::json!({ "version": surum }));
        self.gonder(istek).await.map(|_| ())
    }

    pub async fn modeller(&self) -> Result<Vec<ModelInfo>, EvrenError> {
        self.json_al::<ModelList>("models").await.map(|l| l.data)
    }

    pub async fn sohbet(
        &self,
        istek: &ChatRequest,
    ) -> Result<(ChatResponse, YanitMeta), EvrenError> {
        let mut govde = istek.clone();
        govde.stream = false;
        govde.stream_options = None;
        let (yanit, meta) = self
            .gonder(self.http.post(self.url("chat/completions")).json(&govde))
            .await?;
        Ok((json_coz(yanit).await?, meta))
    }
}

async fn json_coz<T: DeserializeOwned>(yanit: reqwest::Response) -> Result<T, EvrenError> {
    let govde = yanit.bytes().await.map_err(ag_hatasi)?;
    serde_json::from_slice(&govde)
        .map_err(|e| EvrenError::GecersizYanit(format!("JSON çözülemedi: {e}")))
}

pub(crate) fn ag_hatasi(e: reqwest::Error) -> EvrenError {
    EvrenError::Ag(e.to_string())
}

pub(crate) fn simdi_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
