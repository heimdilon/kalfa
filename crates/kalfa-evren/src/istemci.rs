//! EVREN LLM Gateway HTTP istemcisi (spec §3, §5).

use std::fmt;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{Stream, StreamExt};
use reqwest::header::RETRY_AFTER;
use serde::de::DeserializeOwned;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::akis::{AkisBirlestirici, AkisOlayi, VeriSonucu};
use crate::butce::{JSON_BAYT_SINIRI, YANIT_CAGRI_SINIRI, YanitButcesi};
use crate::eklenti::{EkUc, HamYanit};
use crate::hata::{EvrenError, hata_esle};
use crate::kredi::{YanitMeta, basliklardan};
use crate::sse::{SseCozucu, VARSAYILAN_AZAMI_OLAY};
use crate::tipler::{
    ChatRequest, ChatResponse, ModelInfo, ModelList, StreamOptions, TermsStatus, TermsText,
};

pub const VARSAYILAN_TABAN: &str = "https://evren-llmapi.ssyz.org.tr/v1";

/// Akış olayları kanalı. Bırakılırsa (drop) bağlantı kapanır.
pub type AkisKanali = Pin<Box<dyn Stream<Item = Result<AkisOlayi, EvrenError>> + Send>>;

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
        // Belge §9.2: yönlendirme izlenmez; anahtar ve gövde başka hedefe gitmez.
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(180))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("kalfa-evren/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| EvrenError::ag_baglanti(e.to_string()))?;
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
        let govde = sinirli_govde(yanit).await?;
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
        let cevap: ChatResponse = json_coz(yanit).await?;
        let mut adet = 0usize;
        for secim in &cevap.choices {
            adet = adet
                .checked_add(secim.message.tool_calls.as_ref().map_or(0, Vec::len))
                .filter(|n| *n <= YANIT_CAGRI_SINIRI)
                .ok_or(EvrenError::YanitSiniri)?;
        }
        Ok((cevap, meta))
    }

    /// Eklentinin gömme ya da yeniden sıralama isteği (aşama 4 tasarımı §9.1).
    /// Gövde host'ta doğrulanmıştır ve olduğu gibi gider. Yanıt JSON olmalı
    /// ve `JSON_BAYT_SINIRI`'ni aşmamalıdır; ham metin olarak döner.
    pub async fn ek_uc(&self, uc: EkUc, govde: &serde_json::Value) -> Result<HamYanit, EvrenError> {
        let (yanit, meta) = self
            .gonder(self.http.post(self.url(uc.yol())).json(govde))
            .await?;
        let govde = sinirli_govde(yanit).await?;
        let deger: serde_json::Value = serde_json::from_slice(&govde)
            .map_err(|e| EvrenError::GecersizYanit(format!("JSON çözülemedi: {e}")))?;
        let model = deger
            .get("model")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        let govde = String::from_utf8(govde)
            .map_err(|_| EvrenError::GecersizYanit("yanıt UTF-8 değil".into()))?;
        Ok(HamYanit { govde, model, meta })
    }

    /// Akışlı sohbet. HTTP hatası akış başlamadan `Err` döner; akış sırasındaki
    /// hata kanalda son öğe olarak gelir.
    pub async fn sohbet_akisi(
        &self,
        istek: &ChatRequest,
    ) -> Result<(YanitMeta, AkisKanali), EvrenError> {
        let mut govde = istek.clone();
        govde.stream = true;
        govde.stream_options = Some(StreamOptions {
            include_usage: true,
        });
        let (yanit, meta) = self
            .gonder(self.http.post(self.url("chat/completions")).json(&govde))
            .await?;
        let (tx, rx) = mpsc::channel(64);
        tokio::spawn(akisi_isle(yanit, tx));
        Ok((meta, Box::pin(ReceiverStream::new(rx))))
    }
}

type Gonderici = mpsc::Sender<Result<AkisOlayi, EvrenError>>;

/// Yanıt gövdesini okur, SSE → birleştirici hattından geçirip kanala yazar.
/// Alıcı bırakılınca görev **hemen** biter ve yanıt (bağlantı) bırakılır:
/// yalnızca gönderim denemesine güvenilseydi, sessiz ya da yalnızca
/// `: keep-alive` gönderen bir sunucuda bağlantı açık kalırdı.
async fn akisi_isle(yanit: reqwest::Response, tx: Gonderici) {
    let mut baytlar = yanit.bytes_stream();
    let mut cozucu = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    let mut birlestirici = AkisBirlestirici::yeni();
    let mut butce = YanitButcesi::default();

    loop {
        let parca = tokio::select! {
            biased;
            _ = tx.closed() => return,
            parca = baytlar.next() => parca,
        };
        let Some(parca) = parca else { break };
        let parca = match parca {
            Ok(p) => p,
            Err(e) => {
                let _ = tx.send(Err(ag_hatasi(e))).await;
                return;
            }
        };
        if let Err(e) = butce.bayt_ekle(parca.len()) {
            let _ = tx.send(Err(e)).await;
            return;
        }
        for sse in cozucu.veri_olaylari(&parca) {
            let sse = match sse {
                Ok(o) => o,
                Err(e) => {
                    let _ = tx.send(Err(EvrenError::GecersizYanit(e.to_string()))).await;
                    return;
                }
            };
            match birlestirici.veri_isle(&sse.veri) {
                Ok(VeriSonucu::Olaylar(olaylar)) => {
                    for o in olaylar {
                        if tx.send(Ok(o)).await.is_err() {
                            return;
                        }
                    }
                }
                Ok(VeriSonucu::Bitti) => {
                    son_olaylari_gonder(&mut birlestirici, &tx).await;
                    return;
                }
                Err(e) => {
                    let _ = tx.send(Err(e)).await;
                    return;
                }
            }
        }
    }
    // Bağlantı `[DONE]` gelmeden kapandı: finish_reason görüldüyse akış tamamdır,
    // görülmediyse yarıda kalmıştır.
    son_olaylari_gonder(&mut birlestirici, &tx).await;
}

async fn son_olaylari_gonder(birlestirici: &mut AkisBirlestirici, tx: &Gonderici) {
    match birlestirici.bitir() {
        Ok(son) => {
            for o in son {
                let _ = tx.send(Ok(o)).await;
            }
        }
        Err(e) => {
            let _ = tx.send(Err(e)).await;
        }
    }
}

async fn json_coz<T: DeserializeOwned>(yanit: reqwest::Response) -> Result<T, EvrenError> {
    let govde = sinirli_govde(yanit).await?;
    serde_json::from_slice(&govde)
        .map_err(|e| EvrenError::GecersizYanit(format!("JSON çözülemedi: {e}")))
}

async fn sinirli_govde(yanit: reqwest::Response) -> Result<Vec<u8>, EvrenError> {
    let mut akis = yanit.bytes_stream();
    let mut govde = Vec::new();
    while let Some(parca) = akis.next().await {
        let parca = parca.map_err(ag_hatasi)?;
        if parca.len() > JSON_BAYT_SINIRI - govde.len() {
            return Err(EvrenError::YanitSiniri);
        }
        govde.extend_from_slice(&parca);
    }
    Ok(govde)
}

pub(crate) fn ag_hatasi(e: reqwest::Error) -> EvrenError {
    EvrenError::Ag {
        mesaj: e.to_string(),
        govde_gitti: !e.is_connect(),
    }
}

pub(crate) fn simdi_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
