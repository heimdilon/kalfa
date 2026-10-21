//! 1.1b G3a — `rmcp 3.5.1` Streamable HTTP taşıma ve protokol ön doğrulaması; ürün kodu değildir.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use bytes::Bytes;
use futures_util::StreamExt;
use rmcp::RoleClient;
use rmcp::model::{
    ClientJsonRpcMessage, ClientNotification, ClientRequest, ClientResult, ErrorCode, ErrorData,
    GetMeta, ProtocolVersion, RequestId, ServerJsonRpcMessage, ServerNotification, ServerRequest,
    ServerResult,
};
use rmcp::transport::Transport;
use serde_json::{Value, json};
use std::io;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tokio::sync::{Mutex, mpsc, watch};
use tokio::task::JoinHandle;
use url::Url;

pub const CERCEVE_SINIRI: usize = 4 * 1024 * 1024;
pub const KUYRUK_SINIRI: usize = 64;
pub const IC_ISARET_CONTENT_ALANI_VAR: &str = "__kalfa_internal_v1_content_alani_var";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HttpKesinti {
    CerceveCokBuyuk,
    JsonHatasi,
    IletiSeli,
    DesteklenmeyenIcerikTipi(String),
    YetkiGerekli {
        durum: u16,
        www_authenticate: Option<String>,
    },
    HttpDurumHatasi(u16),
    BaglantiKoptu,
    BelirsizCagriSonucu {
        istek_id: String,
        arac_adi: String,
    },
    ZamanAsimi,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseOlayi {
    pub olay: Option<String>,
    pub veri: String,
    pub kimlik: Option<String>,
}

/// WHATWG uyumlu sınırlı SSE ayrıştırıcısı; satır bölündükten sonra UTF-8 çözer.
#[derive(Debug)]
pub struct SinirliSseCozucu {
    azami: usize,
    satir: Vec<u8>,
    cr_sonrasi: bool,
    ilk_satir: bool,
    veri: String,
    veri_var: bool,
    olay: Option<String>,
    kimlik: Option<String>,
}

impl SinirliSseCozucu {
    pub fn yeni(azami: usize) -> Self {
        Self {
            azami,
            satir: Vec::new(),
            cr_sonrasi: false,
            ilk_satir: true,
            veri: String::new(),
            veri_var: false,
            olay: None,
            kimlik: None,
        }
    }

    pub fn besle(&mut self, parca: &[u8]) -> Result<Vec<SseOlayi>, HttpKesinti> {
        let mut olaylar = Vec::new();
        for &bayt in parca {
            if self.cr_sonrasi {
                self.cr_sonrasi = false;
                if bayt == b'\n' {
                    continue;
                }
            }
            match bayt {
                b'\r' => {
                    self.cr_sonrasi = true;
                    if let Some(olay) = self.satiri_isle()? {
                        olaylar.push(olay);
                    }
                }
                b'\n' => {
                    if let Some(olay) = self.satiri_isle()? {
                        olaylar.push(olay);
                    }
                }
                _ => {
                    if self.satir.len() >= self.azami {
                        return Err(HttpKesinti::CerceveCokBuyuk);
                    }
                    self.satir.push(bayt);
                }
            }
        }
        Ok(olaylar)
    }

    fn satiri_isle(&mut self) -> Result<Option<SseOlayi>, HttpKesinti> {
        let ham = std::mem::take(&mut self.satir);
        let mut satir = String::from_utf8_lossy(&ham).into_owned();
        if self.ilk_satir {
            self.ilk_satir = false;
            if let Some(bomsuz) = satir.strip_prefix('\u{FEFF}') {
                satir = bomsuz.to_string();
            }
        }
        if satir.is_empty() {
            if !self.veri_var {
                self.olay = None;
                return Ok(None);
            }
            let mut veri = std::mem::take(&mut self.veri);
            if veri.ends_with('\n') {
                veri.pop();
            }
            self.veri_var = false;
            return Ok(Some(SseOlayi {
                olay: self.olay.take(),
                veri,
                kimlik: self.kimlik.clone(),
            }));
        }
        if satir.starts_with(':') {
            return Ok(None);
        }
        let (alan, deger) = match satir.split_once(':') {
            Some((a, d)) => (a, d.strip_prefix(' ').unwrap_or(d)),
            None => (satir.as_str(), ""),
        };
        match alan {
            "data" => {
                let eklenecek = deger.len().saturating_add(1);
                if self.veri.len().saturating_add(eklenecek) > self.azami {
                    return Err(HttpKesinti::CerceveCokBuyuk);
                }
                self.veri.push_str(deger);
                self.veri.push('\n');
                self.veri_var = true;
            }
            "event" => {
                if deger.len() > self.azami {
                    return Err(HttpKesinti::CerceveCokBuyuk);
                }
                self.olay = Some(deger.to_string());
            }
            "id" if !deger.contains('\0') => {
                if deger.len() > self.azami {
                    return Err(HttpKesinti::CerceveCokBuyuk);
                }
                self.kimlik = Some(deger.to_string());
            }
            _ => {}
        }
        Ok(None)
    }
}

fn cevaba_ic_isaret_ekle(mut ham: Value) -> Result<ServerJsonRpcMessage, serde_json::Error> {
    if let Some(nesne) = ham.as_object_mut()
        && nesne.contains_key("id")
        && let Some(Value::Object(sonuc)) = nesne.get_mut("result")
        && sonuc.contains_key("content")
    {
        let meta = sonuc.entry("_meta").or_insert_with(|| json!({}));
        if let Value::Object(meta) = meta {
            meta.insert(IC_ISARET_CONTENT_ALANI_VAR.to_string(), Value::Bool(true));
        }
    }
    serde_json::from_value(ham)
}

fn baslik_degeri_kodla(deger: &str) -> String {
    let baytlar = deger.as_bytes();
    let bosluk_kenari = matches!(baytlar.first(), Some(b' ' | b'\t'))
        || matches!(baytlar.last(), Some(b' ' | b'\t'));
    let ascii_disi = deger
        .chars()
        .any(|c| (c as u32) < 0x20 || (c as u32) > 0x7E);
    let zaten_sarmal = deger.starts_with("=?base64?") && deger.ends_with("?=");
    if !deger.is_empty() && (bosluk_kenari || ascii_disi || zaten_sarmal) {
        format!("=?base64?{}?=", BASE64_STANDARD.encode(deger))
    } else {
        deger.to_owned()
    }
}

#[derive(Clone, Debug)]
struct OturumDurumu {
    oturum_id: Option<String>,
    protokol_surumu: Option<ProtocolVersion>,
    modern_kip: bool,
}

pub struct SinirliHttpTasima {
    pub yenileme: Arc<AtomicBool>,
    gelen: mpsc::Receiver<ServerJsonRpcMessage>,
    giden: mpsc::Sender<ClientJsonRpcMessage>,
    hata: watch::Receiver<Option<HttpKesinti>>,
    gorev: JoinHandle<()>,
}

impl SinirliHttpTasima {
    pub fn yeni(
        istemci: reqwest::Client,
        hedef: Url,
        yetki_basligi: Option<String>,
        istek_zaman_asimi: Duration,
    ) -> (Self, watch::Receiver<Option<HttpKesinti>>) {
        let (gelen_tx, gelen_rx) = mpsc::channel(KUYRUK_SINIRI);
        let (giden_tx, mut giden_rx) = mpsc::channel::<ClientJsonRpcMessage>(KUYRUK_SINIRI);
        let (hata_tx, hata_rx) = watch::channel(None);
        let yenileme = Arc::new(AtomicBool::new(false));
        let yenileme_ic = yenileme.clone();
        let kontrol_tx = giden_tx.clone();
        let durum = Arc::new(Mutex::new(OturumDurumu {
            oturum_id: None,
            protokol_surumu: None,
            modern_kip: false,
        }));

        let gorev = tokio::spawn(async move {
            let mut abonelik_onayi = false;
            while let Some(ileti) = giden_rx.recv().await {
                if let Err(kesinti) = Self::ilet_ve_isle(
                    &istemci,
                    &hedef,
                    yetki_basligi.as_deref(),
                    istek_zaman_asimi,
                    &durum,
                    &yenileme_ic,
                    &mut abonelik_onayi,
                    &gelen_tx,
                    &kontrol_tx,
                    ileti,
                )
                .await
                {
                    let _ = hata_tx.send(Some(kesinti));
                    break;
                }
            }
        });

        (
            Self {
                yenileme,
                gelen: gelen_rx,
                giden: giden_tx,
                hata: hata_rx.clone(),
                gorev,
            },
            hata_rx,
        )
    }

    #[allow(clippy::too_many_arguments)]
    async fn ilet_ve_isle(
        istemci: &reqwest::Client,
        hedef: &Url,
        yetki_basligi: Option<&str>,
        istek_zaman_asimi: Duration,
        durum: &Arc<Mutex<OturumDurumu>>,
        yenileme: &Arc<AtomicBool>,
        abonelik_onayi: &mut bool,
        gelen_tx: &mpsc::Sender<ServerJsonRpcMessage>,
        kontrol_tx: &mpsc::Sender<ClientJsonRpcMessage>,
        ileti: ClientJsonRpcMessage,
    ) -> Result<(), HttpKesinti> {
        let is_discover = matches!(
            &ileti,
            ClientJsonRpcMessage::Request(r) if matches!(&r.request, ClientRequest::DiscoverRequest(_))
        );
        let is_initialize = matches!(
            &ileti,
            ClientJsonRpcMessage::Request(r) if matches!(&r.request, ClientRequest::InitializeRequest(_))
        );
        let is_cancelled_notif = matches!(
            &ileti,
            ClientJsonRpcMessage::Notification(n)
                if matches!(&n.notification, ClientNotification::CancelledNotification(_))
        );
        let cagri_bilgisi: Option<(RequestId, String)> = match &ileti {
            ClientJsonRpcMessage::Request(r) => match &r.request {
                ClientRequest::CallToolRequest(c) => {
                    Some((r.id.clone(), c.params.name.to_string()))
                }
                _ => None,
            },
            _ => None,
        };
        let beklenen_istek_id: Option<RequestId> = match &ileti {
            ClientJsonRpcMessage::Request(r) => Some(r.id.clone()),
            _ => None,
        };

        let anlik = durum.lock().await.clone();
        // Modern (2026-07-28) kipte iptal akışın düşürülmesiyle yapılır; ayrı POST gerekmez.
        if is_cancelled_notif && anlik.modern_kip {
            return Ok(());
        }

        let govde_degeri = serde_json::to_value(&ileti).map_err(|_| HttpKesinti::JsonHatasi)?;
        let govde = serde_json::to_vec(&govde_degeri).map_err(|_| HttpKesinti::JsonHatasi)?;
        if govde.len() > CERCEVE_SINIRI {
            return Err(HttpKesinti::CerceveCokBuyuk);
        }

        let mut istek = istemci
            .post(hedef.clone())
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");

        if let Some(yetki) = yetki_basligi {
            istek = istek.header("Authorization", yetki);
        }

        let istek_surumu = match &ileti {
            ClientJsonRpcMessage::Request(r) => r
                .request
                .get_meta()
                .protocol_version()
                .or_else(|| anlik.protokol_surumu.clone()),
            _ => anlik.protokol_surumu.clone(),
        };

        if !is_initialize {
            if let Some(ref surum) = istek_surumu {
                istek = istek.header("MCP-Protocol-Version", surum.as_str());
                if *surum >= ProtocolVersion::STANDARD_HEADERS
                    && let Some(yontem) = govde_degeri.get("method").and_then(Value::as_str)
                {
                    istek = istek.header("Mcp-Method", yontem);
                    if yontem == "tools/call"
                        && let Some(ad) = govde_degeri
                            .get("params")
                            .and_then(|p| p.get("name"))
                            .and_then(Value::as_str)
                    {
                        istek = istek.header("Mcp-Name", baslik_degeri_kodla(ad));
                    }
                }
            }
            if !is_discover && let Some(ref sid) = anlik.oturum_id {
                istek = istek.header("Mcp-Session-Id", sid.as_str());
            }
        }

        let yanit = match tokio::time::timeout(istek_zaman_asimi, istek.body(govde).send()).await {
            Ok(Ok(y)) => y,
            Ok(Err(_)) => {
                if let Some((id, arac)) = cagri_bilgisi {
                    return Err(HttpKesinti::BelirsizCagriSonucu {
                        istek_id: id.to_string(),
                        arac_adi: arac,
                    });
                }
                return Err(HttpKesinti::BaglantiKoptu);
            }
            Err(_) => {
                if let Some((id, arac)) = cagri_bilgisi {
                    return Err(HttpKesinti::BelirsizCagriSonucu {
                        istek_id: id.to_string(),
                        arac_adi: arac,
                    });
                }
                return Err(HttpKesinti::ZamanAsimi);
            }
        };

        let durum_kodu = yanit.status().as_u16();
        if matches!(durum_kodu, 401 | 403) {
            let www_auth = yanit
                .headers()
                .get("www-authenticate")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned);
            return Err(HttpKesinti::YetkiGerekli {
                durum: durum_kodu,
                www_authenticate: www_auth,
            });
        }

        // Eski (2025-*) sunucular oturumsuz `server/discover` çağrısını 4xx ile reddedebilir.
        if is_discover && (400..500).contains(&durum_kodu) {
            let metin = Self::sinirli_govde_oku(yanit, CERCEVE_SINIRI)
                .await
                .unwrap_or_default();
            let hata_verisi = match serde_json::from_slice::<ServerJsonRpcMessage>(&metin) {
                Ok(ServerJsonRpcMessage::Error(e)) => e.error,
                _ => ErrorData::new(
                    ErrorCode::METHOD_NOT_FOUND,
                    "eski sunucu server/discover yöntemini reddetti",
                    None,
                ),
            };
            if let Some(id) = beklenen_istek_id {
                gelen_tx
                    .try_send(ServerJsonRpcMessage::error(hata_verisi, Some(id)))
                    .map_err(|_| HttpKesinti::IletiSeli)?;
            }
            return Ok(());
        }

        if beklenen_istek_id.is_none() {
            if matches!(durum_kodu, 200 | 202 | 204) {
                return Ok(());
            }
            return Err(HttpKesinti::HttpDurumHatasi(durum_kodu));
        }

        if durum_kodu != 200 {
            return Err(HttpKesinti::HttpDurumHatasi(durum_kodu));
        }

        let yeni_oturum_id = yanit
            .headers()
            .get("mcp-session-id")
            .and_then(|v| v.to_str().ok())
            .filter(|s| !s.is_empty() && s.bytes().all(|b| (0x21..=0x7E).contains(&b)))
            .map(str::to_owned);

        let icerik_tipi = yanit
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();

        match icerik_tipi.as_str() {
            "application/json" => {
                let tam = match tokio::time::timeout(
                    istek_zaman_asimi,
                    Self::sinirli_govde_oku(yanit, CERCEVE_SINIRI),
                )
                .await
                {
                    Ok(Ok(b)) => b,
                    Ok(Err(HttpKesinti::BaglantiKoptu)) => {
                        if let Some((id, arac)) = cagri_bilgisi {
                            return Err(HttpKesinti::BelirsizCagriSonucu {
                                istek_id: id.to_string(),
                                arac_adi: arac,
                            });
                        }
                        return Err(HttpKesinti::BaglantiKoptu);
                    }
                    Ok(Err(e)) => return Err(e),
                    Err(_) => {
                        if let Some((id, arac)) = cagri_bilgisi {
                            return Err(HttpKesinti::BelirsizCagriSonucu {
                                istek_id: id.to_string(),
                                arac_adi: arac,
                            });
                        }
                        return Err(HttpKesinti::ZamanAsimi);
                    }
                };
                Self::json_ileti_isle(
                    &tam,
                    is_discover,
                    is_initialize,
                    beklenen_istek_id.as_ref(),
                    yeni_oturum_id,
                    durum,
                    yenileme,
                    abonelik_onayi,
                    gelen_tx,
                    kontrol_tx,
                )
                .await?;
            }
            "text/event-stream" => {
                let yanit_geldi = match tokio::time::timeout(
                    istek_zaman_asimi,
                    Self::sse_akisi_isle(
                        yanit,
                        is_discover,
                        is_initialize,
                        beklenen_istek_id.as_ref(),
                        yeni_oturum_id,
                        durum,
                        yenileme,
                        abonelik_onayi,
                        gelen_tx,
                        kontrol_tx,
                    ),
                )
                .await
                {
                    Ok(Ok(geldi)) => geldi,
                    Ok(Err(HttpKesinti::BaglantiKoptu)) => {
                        if let Some((id, arac)) = cagri_bilgisi {
                            return Err(HttpKesinti::BelirsizCagriSonucu {
                                istek_id: id.to_string(),
                                arac_adi: arac,
                            });
                        }
                        return Err(HttpKesinti::BaglantiKoptu);
                    }
                    Ok(Err(e)) => return Err(e),
                    Err(_) => {
                        if let Some((id, arac)) = cagri_bilgisi {
                            return Err(HttpKesinti::BelirsizCagriSonucu {
                                istek_id: id.to_string(),
                                arac_adi: arac,
                            });
                        }
                        return Err(HttpKesinti::ZamanAsimi);
                    }
                };
                if !yanit_geldi {
                    if let Some((id, arac)) = cagri_bilgisi {
                        return Err(HttpKesinti::BelirsizCagriSonucu {
                            istek_id: id.to_string(),
                            arac_adi: arac,
                        });
                    }
                    return Err(HttpKesinti::BaglantiKoptu);
                }
            }
            diger => {
                return Err(HttpKesinti::DesteklenmeyenIcerikTipi(diger.to_string()));
            }
        }

        Ok(())
    }

    async fn sinirli_govde_oku(
        yanit: reqwest::Response,
        sinir: usize,
    ) -> Result<Vec<u8>, HttpKesinti> {
        if yanit
            .content_length()
            .is_some_and(|uzunluk| uzunluk > sinir as u64)
        {
            return Err(HttpKesinti::CerceveCokBuyuk);
        }
        let mut akis = yanit.bytes_stream();
        let mut biriken = Vec::new();
        while let Some(parca) = akis.next().await {
            let parca: Bytes = parca.map_err(|_| HttpKesinti::BaglantiKoptu)?;
            if biriken.len().saturating_add(parca.len()) > sinir {
                return Err(HttpKesinti::CerceveCokBuyuk);
            }
            biriken.extend_from_slice(&parca);
        }
        Ok(biriken)
    }

    #[allow(clippy::too_many_arguments)]
    async fn sse_akisi_isle(
        yanit: reqwest::Response,
        is_discover: bool,
        is_initialize: bool,
        beklenen_istek_id: Option<&RequestId>,
        yeni_oturum_id: Option<String>,
        durum: &Arc<Mutex<OturumDurumu>>,
        yenileme: &Arc<AtomicBool>,
        abonelik_onayi: &mut bool,
        gelen_tx: &mpsc::Sender<ServerJsonRpcMessage>,
        kontrol_tx: &mpsc::Sender<ClientJsonRpcMessage>,
    ) -> Result<bool, HttpKesinti> {
        let mut akis = yanit.bytes_stream();
        let mut cozucu = SinirliSseCozucu::yeni(CERCEVE_SINIRI);
        let mut yanit_geldi = false;
        let mut oturum_islendi = yeni_oturum_id;

        while let Some(parca) = akis.next().await {
            let parca: Bytes = parca.map_err(|_| HttpKesinti::BaglantiKoptu)?;
            let olaylar = cozucu.besle(&parca)?;
            for olay in olaylar {
                if let Some(olay_adi) = olay.olay.as_deref()
                    && !olay_adi.is_empty()
                    && olay_adi != "message"
                {
                    continue;
                }
                let son_yanit = Self::json_ileti_isle(
                    olay.veri.as_bytes(),
                    is_discover,
                    is_initialize,
                    beklenen_istek_id,
                    oturum_islendi.take(),
                    durum,
                    yenileme,
                    abonelik_onayi,
                    gelen_tx,
                    kontrol_tx,
                )
                .await?;
                if son_yanit {
                    yanit_geldi = true;
                }
            }
            if yanit_geldi {
                break;
            }
        }
        Ok(yanit_geldi)
    }

    #[allow(clippy::too_many_arguments)]
    async fn json_ileti_isle(
        ham: &[u8],
        is_discover: bool,
        is_initialize: bool,
        beklenen_istek_id: Option<&RequestId>,
        yeni_oturum_id: Option<String>,
        durum: &Arc<Mutex<OturumDurumu>>,
        yenileme: &Arc<AtomicBool>,
        abonelik_onayi: &mut bool,
        gelen_tx: &mpsc::Sender<ServerJsonRpcMessage>,
        kontrol_tx: &mpsc::Sender<ClientJsonRpcMessage>,
    ) -> Result<bool, HttpKesinti> {
        let deger: Value = serde_json::from_slice(ham).map_err(|_| HttpKesinti::JsonHatasi)?;
        let mut ileti = cevaba_ic_isaret_ekle(deger).map_err(|_| HttpKesinti::JsonHatasi)?;

        if is_discover {
            match &mut ileti {
                ServerJsonRpcMessage::Response(yanit) => {
                    if let ServerResult::DiscoverResult(sonuc) = &yanit.result {
                        let mut kilit = durum.lock().await;
                        kilit.modern_kip = true;
                        kilit.oturum_id = None;
                        kilit.protokol_surumu = sonuc
                            .supported_versions
                            .iter()
                            .find(|s| **s == ProtocolVersion::V_2026_07_28)
                            .cloned()
                            .or_else(|| sonuc.supported_versions.first().cloned());
                    }
                }
                ServerJsonRpcMessage::Error(hata) => {
                    if let Some(id) = beklenen_istek_id {
                        hata.id = Some(id.clone());
                    }
                }
                _ => {}
            }
        } else if is_initialize
            && let ServerJsonRpcMessage::Response(yanit) = &ileti
            && let ServerResult::InitializeResult(sonuc) = &yanit.result
        {
            let mut kilit = durum.lock().await;
            kilit.modern_kip = false;
            kilit.protokol_surumu = Some(sonuc.protocol_version.clone());
            if yeni_oturum_id.is_some() {
                kilit.oturum_id = yeni_oturum_id;
            }
        }

        if let ServerJsonRpcMessage::Notification(ref n) = ileti {
            match &n.notification {
                ServerNotification::ToolListChangedNotification(_) => {
                    yenileme.store(true, Ordering::SeqCst);
                    return Ok(false);
                }
                ServerNotification::SubscriptionsAcknowledgedNotification(_)
                    if !*abonelik_onayi =>
                {
                    *abonelik_onayi = true;
                }
                ServerNotification::SubscriptionsAcknowledgedNotification(_) => {
                    return Err(HttpKesinti::IletiSeli);
                }
                _ => return Ok(false),
            }
        }

        if let ServerJsonRpcMessage::Request(ref istek) = ileti {
            let yanit_iletisi = if matches!(istek.request, ServerRequest::PingRequest(_)) {
                ClientJsonRpcMessage::response(ClientResult::empty(()), istek.id.clone())
            } else {
                ClientJsonRpcMessage::error(
                    ErrorData::new(ErrorCode::METHOD_NOT_FOUND, "Yöntem desteklenmiyor", None),
                    Some(istek.id.clone()),
                )
            };
            kontrol_tx
                .try_send(yanit_iletisi)
                .map_err(|_| HttpKesinti::IletiSeli)?;
            return Ok(false);
        }

        let son_yanit_mi = matches!(
            ileti,
            ServerJsonRpcMessage::Response(_) | ServerJsonRpcMessage::Error(_)
        );
        gelen_tx
            .try_send(ileti)
            .map_err(|_| HttpKesinti::IletiSeli)?;
        Ok(son_yanit_mi)
    }
}

impl Transport<RoleClient> for SinirliHttpTasima {
    type Error = io::Error;

    fn send(
        &mut self,
        item: ClientJsonRpcMessage,
    ) -> impl Future<Output = io::Result<()>> + Send + 'static {
        let tx = self.giden.clone();
        async move {
            tx.send(item)
                .await
                .map_err(|_| io::Error::other("HTTP taşıma kapandı"))
        }
    }

    async fn receive(&mut self) -> Option<ServerJsonRpcMessage> {
        if self.hata.borrow().is_some() {
            return None;
        }
        tokio::select! {
            biased;
            _ = self.hata.changed() => None,
            ileti = self.gelen.recv() => ileti,
        }
    }

    async fn close(&mut self) -> io::Result<()> {
        self.gorev.abort();
        Ok(())
    }
}

impl Drop for SinirliHttpTasima {
    fn drop(&mut self) {
        self.gorev.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::{
        CallToolRequestParams, DiscoverResult, Implementation, ServerCapabilities, Tool,
    };
    use rmcp::{ClientHandler, ClientLifecycleMode, ClientServiceExt};
    use std::collections::HashMap;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    use tokio::net::{TcpListener, TcpStream};

    #[derive(Clone, Default)]
    struct SahteIstemci;
    impl ClientHandler for SahteIstemci {}

    #[derive(Clone, Debug)]
    struct KaydedilenHttpIstek {
        yontem: String,
        yol: String,
        basliklar: HashMap<String, String>,
        govde: Value,
    }

    async fn http_istegi_oku(akis: &mut TcpStream) -> Option<KaydedilenHttpIstek> {
        let mut okuyucu = BufReader::new(akis);
        let mut ilk_satir = String::new();
        if okuyucu.read_line(&mut ilk_satir).await.ok()? == 0 {
            return None;
        }
        let parcalar: Vec<&str> = ilk_satir.split_whitespace().collect();
        if parcalar.len() < 2 {
            return None;
        }
        let yontem = parcalar[0].to_string();
        let yol = parcalar[1].to_string();
        let mut basliklar = HashMap::new();
        let mut icerik_uzunlugu = 0usize;
        loop {
            let mut satir = String::new();
            if okuyucu.read_line(&mut satir).await.ok()? == 0 {
                break;
            }
            let temiz = satir.trim();
            if temiz.is_empty() {
                break;
            }
            if let Some((ad, deger)) = temiz.split_once(':') {
                let kucuk = ad.trim().to_ascii_lowercase();
                let d = deger.trim().to_string();
                if kucuk == "content-length" {
                    icerik_uzunlugu = d.parse().unwrap_or(0);
                }
                basliklar.insert(kucuk, d);
            }
        }
        let mut ham = vec![0u8; icerik_uzunlugu];
        if icerik_uzunlugu > 0 {
            okuyucu.read_exact(&mut ham).await.ok()?;
        }
        let govde = if ham.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&ham).unwrap_or(Value::Null)
        };
        Some(KaydedilenHttpIstek {
            yontem,
            yol,
            basliklar,
            govde,
        })
    }

    async fn http_json_yaniti_yaz(
        akis: &mut TcpStream,
        durum: &str,
        ek_basliklar: &[(&str, &str)],
        govde: &Value,
    ) {
        let baytlar = serde_json::to_vec(govde).unwrap();
        let mut baslik = format!(
            "HTTP/1.1 {durum}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
            baytlar.len()
        );
        for (k, v) in ek_basliklar {
            baslik.push_str(&format!("{k}: {v}\r\n"));
        }
        baslik.push_str("\r\n");
        akis.write_all(baslik.as_bytes()).await.unwrap();
        akis.write_all(&baytlar).await.unwrap();
        akis.flush().await.unwrap();
    }

    fn ornek_arac(ad: &str) -> Tool {
        let sema: serde_json::Map<String, Value> = serde_json::from_value(json!({
            "type": "object",
            "properties": {
                "girdi": { "type": "string" }
            }
        }))
        .unwrap();
        Tool::new(ad.to_string(), "Örnek uzak araç", Arc::new(sema))
    }

    fn test_http_istemcisi() -> reqwest::Client {
        let _ = rustls::crypto::ring::default_provider().install_default();
        reqwest::Client::builder().no_proxy().build().unwrap()
    }

    #[tokio::test]
    async fn protokol_2026_07_28_discover_ve_standart_basliklar() {
        let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let adres = dinleyici.local_addr().unwrap();
        let kayitlar = Arc::new(Mutex::new(Vec::<KaydedilenHttpIstek>::new()));
        let kayitlar_sunucu = kayitlar.clone();

        let sunucu = tokio::spawn(async move {
            while let Ok((mut akis, _)) = dinleyici.accept().await {
                let Some(istek) = http_istegi_oku(&mut akis).await else {
                    continue;
                };
                kayitlar_sunucu.lock().await.push(istek.clone());
                let id = istek.govde["id"].clone();
                match istek.govde["method"].as_str() {
                    Some("server/discover") => {
                        let mut yetenekler = ServerCapabilities::default();
                        yetenekler.tools = Some(Default::default());
                        let sonuc =
                            DiscoverResult::new(vec![ProtocolVersion::V_2026_07_28], yetenekler)
                                .with_server_info(Implementation::new("modern-http", "1.0"));
                        http_json_yaniti_yaz(
                            &mut akis,
                            "200 OK",
                            &[],
                            &json!({"jsonrpc": "2.0", "id": id, "result": sonuc}),
                        )
                        .await;
                    }
                    Some("tools/list") => {
                        let arac = ornek_arac("uzak_sorgu_ğüşıöç");
                        http_json_yaniti_yaz(
                            &mut akis,
                            "200 OK",
                            &[],
                            &json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": { "tools": [arac], "ttlMs": 60000 }
                            }),
                        )
                        .await;
                    }
                    Some("tools/call") => {
                        http_json_yaniti_yaz(
                            &mut akis,
                            "200 OK",
                            &[],
                            &json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "content": [{ "type": "text", "text": "merhaba 2026" }],
                                    "isError": false
                                }
                            }),
                        )
                        .await;
                    }
                    _ => {}
                }
            }
        });

        let hedef = Url::parse(&format!("http://{adres}/mcp")).unwrap();
        let (tasima, _hata) = SinirliHttpTasima::yeni(
            test_http_istemcisi(),
            hedef,
            Some("Bearer test-jeton".to_string()),
            Duration::from_secs(3),
        );

        let hizmet = SahteIstemci
            .serve_with_lifecycle(
                tasima,
                ClientLifecycleMode::Auto {
                    preferred_versions: vec![ProtocolVersion::V_2026_07_28],
                    legacy_version: Some(ProtocolVersion::V_2025_11_25),
                },
            )
            .await
            .unwrap();
        hizmet
            .peer()
            .set_response_cache_config(rmcp::ClientCacheConfig::disabled())
            .await;

        let liste_1 = hizmet.list_tools(None).await.unwrap();
        let liste_2 = hizmet.list_tools(None).await.unwrap();
        assert_eq!(liste_1.tools.len(), 1);
        assert_eq!(liste_2.tools.len(), 1);

        let cagri = hizmet
            .call_tool(CallToolRequestParams::new("uzak_sorgu_ğüşıöç"))
            .await
            .unwrap();
        assert_eq!(
            cagri
                .meta
                .as_ref()
                .and_then(|m| m.get(IC_ISARET_CONTENT_ALANI_VAR)),
            Some(&Value::Bool(true))
        );

        let kaydedilen = kayitlar.lock().await.clone();
        assert_eq!(kaydedilen.len(), 4);
        assert!(!kaydedilen.iter().any(|i| i.govde["method"] == "initialize"
            || i.govde["method"] == "notifications/initialized"));
        for istek in &kaydedilen {
            assert_eq!(istek.yontem, "POST");
            assert_eq!(istek.yol, "/mcp");
            assert_eq!(
                istek
                    .basliklar
                    .get("mcp-protocol-version")
                    .map(String::as_str),
                Some("2026-07-28")
            );
            assert_eq!(
                istek.govde["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"],
                json!("2026-07-28")
            );
            assert_eq!(
                istek.govde["params"]["_meta"]["io.modelcontextprotocol/clientCapabilities"],
                json!({})
            );
        }
        let cagri_istegi = kaydedilen.last().unwrap();
        assert_eq!(
            cagri_istegi.basliklar.get("mcp-method").map(String::as_str),
            Some("tools/call")
        );
        assert!(
            cagri_istegi
                .basliklar
                .get("mcp-name")
                .is_some_and(|v| v.starts_with("=?base64?") && v.ends_with("?="))
        );

        hizmet.cancel().await.unwrap();
        sunucu.abort();
    }

    async fn eski_donem_senaryosu_calistir(surum: ProtocolVersion, discover_http_400: bool) {
        let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let adres = dinleyici.local_addr().unwrap();
        let kayitlar = Arc::new(Mutex::new(Vec::<KaydedilenHttpIstek>::new()));
        let kayitlar_sunucu = kayitlar.clone();
        let surum_dizge = surum.as_str().to_string();
        let beklenen_oturum = format!("oturum-{surum_dizge}");
        let beklenen_oturum_sunucu = beklenen_oturum.clone();

        let sunucu = tokio::spawn(async move {
            while let Ok((mut akis, _)) = dinleyici.accept().await {
                let Some(istek) = http_istegi_oku(&mut akis).await else {
                    continue;
                };
                kayitlar_sunucu.lock().await.push(istek.clone());
                let id = istek.govde["id"].clone();
                match istek.govde["method"].as_str() {
                    Some("server/discover") => {
                        if discover_http_400 {
                            http_json_yaniti_yaz(
                                &mut akis,
                                "400 Bad Request",
                                &[],
                                &json!({"error": "oturum baslatilmadi"}),
                            )
                            .await;
                        } else {
                            http_json_yaniti_yaz(
                                &mut akis,
                                "200 OK",
                                &[],
                                &json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "error": { "code": -32601, "message": "Method not found" }
                                }),
                            )
                            .await;
                        }
                    }
                    Some("initialize") => {
                        http_json_yaniti_yaz(
                            &mut akis,
                            "200 OK",
                            &[("Mcp-Session-Id", beklenen_oturum_sunucu.as_str())],
                            &json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "protocolVersion": surum_dizge,
                                    "capabilities": { "tools": { "listChanged": true } },
                                    "serverInfo": { "name": "eski-http", "version": "1.0" }
                                }
                            }),
                        )
                        .await;
                    }
                    Some("notifications/initialized") => {
                        let yanit = "HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                        akis.write_all(yanit.as_bytes()).await.unwrap();
                        akis.flush().await.unwrap();
                    }
                    Some("tools/list") => {
                        let arac = ornek_arac("eski_arac");
                        http_json_yaniti_yaz(
                            &mut akis,
                            "200 OK",
                            &[],
                            &json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": { "tools": [arac] }
                            }),
                        )
                        .await;
                    }
                    Some("tools/call") => {
                        http_json_yaniti_yaz(
                            &mut akis,
                            "200 OK",
                            &[],
                            &json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "content": [],
                                    "isError": false
                                }
                            }),
                        )
                        .await;
                    }
                    _ => {}
                }
            }
        });

        let hedef = Url::parse(&format!("http://{adres}/mcp")).unwrap();
        let (tasima, _hata) =
            SinirliHttpTasima::yeni(test_http_istemcisi(), hedef, None, Duration::from_secs(3));

        let hizmet = SahteIstemci
            .serve_with_lifecycle(
                tasima,
                ClientLifecycleMode::Auto {
                    preferred_versions: vec![ProtocolVersion::V_2026_07_28],
                    legacy_version: Some(ProtocolVersion::V_2025_11_25),
                },
            )
            .await
            .unwrap();
        hizmet
            .peer()
            .set_response_cache_config(rmcp::ClientCacheConfig::disabled())
            .await;

        let liste = hizmet.list_tools(None).await.unwrap();
        assert_eq!(liste.tools.len(), 1);
        let cagri = hizmet
            .call_tool(CallToolRequestParams::new("eski_arac"))
            .await
            .unwrap();
        assert!(cagri.content.is_empty());
        assert_eq!(
            cagri
                .meta
                .as_ref()
                .and_then(|m| m.get(IC_ISARET_CONTENT_ALANI_VAR)),
            Some(&Value::Bool(true))
        );

        let kaydedilen = kayitlar.lock().await.clone();
        assert_eq!(kaydedilen[0].govde["method"], "server/discover");
        assert_eq!(kaydedilen[1].govde["method"], "initialize");
        assert!(!kaydedilen[1].basliklar.contains_key("mcp-session-id"));
        assert_eq!(kaydedilen[2].govde["method"], "notifications/initialized");
        for istek in &kaydedilen[2..] {
            assert_eq!(
                istek.basliklar.get("mcp-session-id").map(String::as_str),
                Some(beklenen_oturum.as_str())
            );
            assert_eq!(
                istek
                    .basliklar
                    .get("mcp-protocol-version")
                    .map(String::as_str),
                Some(surum.as_str())
            );
        }

        hizmet.cancel().await.unwrap();
        sunucu.abort();
    }

    #[tokio::test]
    async fn eski_donem_surum_matrisi_2025_11_25_2025_06_18_2025_03_26() {
        eski_donem_senaryosu_calistir(ProtocolVersion::V_2025_11_25, true).await;
        eski_donem_senaryosu_calistir(ProtocolVersion::V_2025_06_18, false).await;
        eski_donem_senaryosu_calistir(ProtocolVersion::V_2025_03_26, true).await;
    }

    #[tokio::test]
    async fn sse_cok_parcali_turkce_utf8_bildirim_seli_ve_sunucu_istek_reddi() {
        let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let adres = dinleyici.local_addr().unwrap();
        let kontrol_kayitlari = Arc::new(Mutex::new(Vec::<Value>::new()));
        let kontrol_sunucu = kontrol_kayitlari.clone();

        let sunucu = tokio::spawn(async move {
            while let Ok((mut akis, _)) = dinleyici.accept().await {
                let Some(istek) = http_istegi_oku(&mut akis).await else {
                    continue;
                };
                if istek.govde.get("method").is_none() {
                    kontrol_sunucu.lock().await.push(istek.govde.clone());
                    let yanit =
                        "HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                    akis.write_all(yanit.as_bytes()).await.unwrap();
                    continue;
                }
                let id = istek.govde["id"].clone();
                match istek.govde["method"].as_str() {
                    Some("server/discover") => {
                        let mut yetenekler = ServerCapabilities::default();
                        yetenekler.tools = Some(Default::default());
                        let sonuc =
                            DiscoverResult::new(vec![ProtocolVersion::V_2026_07_28], yetenekler)
                                .with_server_info(Implementation::new("sse-sunucu", "1.0"));
                        http_json_yaniti_yaz(
                            &mut akis,
                            "200 OK",
                            &[],
                            &json!({"jsonrpc": "2.0", "id": id, "result": sonuc}),
                        )
                        .await;
                    }
                    Some("tools/call") => {
                        let baslik = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nConnection: close\r\n\r\n";
                        akis.write_all(baslik.as_bytes()).await.unwrap();
                        akis.write_all(b": canli-baglanti\n\n").await.unwrap();
                        for _ in 0..300 {
                            let bildirim = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/tools/list_changed\"}\n\n";
                            akis.write_all(bildirim.as_bytes()).await.unwrap();
                        }
                        let ornekleme = "data: {\"jsonrpc\":\"2.0\",\"id\":701,\"method\":\"sampling/createMessage\",\"params\":{\"messages\":[],\"maxTokens\":1}}\n\n";
                        let ping = "data: {\"jsonrpc\":\"2.0\",\"id\":702,\"method\":\"ping\"}\n\n";
                        akis.write_all(ornekleme.as_bytes()).await.unwrap();
                        akis.write_all(ping.as_bytes()).await.unwrap();

                        let son_olay = format!(
                            "id: olay-9\nevent: message\ndata: {}\n\n",
                            json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "content": [{ "type": "text", "text": "İğüşöç-SSE-Yanıtı" }],
                                    "isError": false
                                }
                            })
                        );
                        // Çok baytlı UTF-8 dizisini tek tek baytlarla göndererek parça sınırını sına.
                        for &b in son_olay.as_bytes() {
                            akis.write_all(&[b]).await.unwrap();
                        }
                        akis.flush().await.unwrap();
                    }
                    _ => {}
                }
            }
        });

        let hedef = Url::parse(&format!("http://{adres}/mcp")).unwrap();
        let (tasima, hata) =
            SinirliHttpTasima::yeni(test_http_istemcisi(), hedef, None, Duration::from_secs(5));
        let yenileme_bayragi = tasima.yenileme.clone();

        let hizmet = SahteIstemci
            .serve_with_lifecycle(
                tasima,
                ClientLifecycleMode::Auto {
                    preferred_versions: vec![ProtocolVersion::V_2026_07_28],
                    legacy_version: Some(ProtocolVersion::V_2025_11_25),
                },
            )
            .await
            .unwrap();

        let sonuc = hizmet
            .call_tool(CallToolRequestParams::new("sse_arac"))
            .await
            .unwrap();
        assert!(yenileme_bayragi.load(Ordering::SeqCst));
        assert_eq!(*hata.borrow(), None);
        let metin = serde_json::to_string(&sonuc.content).unwrap();
        assert!(metin.contains("İğüşöç-SSE-Yanıtı"));

        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let k = kontrol_kayitlari.lock().await;
                let ornekleme_red = k
                    .iter()
                    .any(|v| v["id"] == 701 && v["error"]["code"] == -32601);
                let ping_yanit = k
                    .iter()
                    .any(|v| v["id"] == 702 && v.get("result").is_some());
                if ornekleme_red && ping_yanit {
                    break;
                }
                drop(k);
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();

        hizmet.cancel().await.unwrap();
        sunucu.abort();
    }

    #[tokio::test]
    async fn bozuk_json_asiri_buyuk_govde_ve_beklenmeyen_icerik_tipi_reddedilir() {
        // 1. Beklenmeyen içerik tipi (`text/html`)
        {
            let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let adres = dinleyici.local_addr().unwrap();
            tokio::spawn(async move {
                if let Ok((mut akis, _)) = dinleyici.accept().await {
                    let _ = http_istegi_oku(&mut akis).await;
                    let yanit = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 12\r\nConnection: close\r\n\r\n<html></html>";
                    let _ = akis.write_all(yanit.as_bytes()).await;
                }
            });
            let (tasima, hata) = SinirliHttpTasima::yeni(
                test_http_istemcisi(),
                Url::parse(&format!("http://{adres}/mcp")).unwrap(),
                None,
                Duration::from_secs(2),
            );
            let sonuc = SahteIstemci
                .serve_with_lifecycle(
                    tasima,
                    ClientLifecycleMode::Auto {
                        preferred_versions: vec![ProtocolVersion::V_2026_07_28],
                        legacy_version: Some(ProtocolVersion::V_2025_11_25),
                    },
                )
                .await;
            assert!(sonuc.is_err());
            assert_eq!(
                *hata.borrow(),
                Some(HttpKesinti::DesteklenmeyenIcerikTipi(
                    "text/html".to_string()
                ))
            );
        }

        // 2. Bozuk JSON
        {
            let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let adres = dinleyici.local_addr().unwrap();
            tokio::spawn(async move {
                if let Ok((mut akis, _)) = dinleyici.accept().await {
                    let _ = http_istegi_oku(&mut akis).await;
                    let yanit = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 19\r\nConnection: close\r\n\r\n{bozuk-gizli-json!!";
                    let _ = akis.write_all(yanit.as_bytes()).await;
                }
            });
            let (tasima, hata) = SinirliHttpTasima::yeni(
                test_http_istemcisi(),
                Url::parse(&format!("http://{adres}/mcp")).unwrap(),
                None,
                Duration::from_secs(2),
            );
            let sonuc = SahteIstemci
                .serve_with_lifecycle(
                    tasima,
                    ClientLifecycleMode::Auto {
                        preferred_versions: vec![ProtocolVersion::V_2026_07_28],
                        legacy_version: Some(ProtocolVersion::V_2025_11_25),
                    },
                )
                .await;
            assert!(sonuc.is_err());
            assert_eq!(*hata.borrow(), Some(HttpKesinti::JsonHatasi));
        }

        // 3. Aşırı büyük SSE olayı (> 4 MiB)
        {
            let mut cozucu = SinirliSseCozucu::yeni(CERCEVE_SINIRI);
            let mut dev_satir = b"data: ".to_vec();
            dev_satir.extend(std::iter::repeat_n(b'a', CERCEVE_SINIRI));
            assert_eq!(cozucu.besle(&dev_satir), Err(HttpKesinti::CerceveCokBuyuk));
        }
    }

    #[tokio::test]
    async fn akis_ortasinda_baglanti_kopmasi_belirsiz_sonuc_verir_ve_otomatik_tekrar_yapmaz() {
        let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let adres = dinleyici.local_addr().unwrap();
        let cagri_sayaci = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let cagri_sayaci_sunucu = cagri_sayaci.clone();

        let sunucu = tokio::spawn(async move {
            while let Ok((mut akis, _)) = dinleyici.accept().await {
                let Some(istek) = http_istegi_oku(&mut akis).await else {
                    continue;
                };
                let id = istek.govde["id"].clone();
                match istek.govde["method"].as_str() {
                    Some("server/discover") => {
                        let mut yetenekler = ServerCapabilities::default();
                        yetenekler.tools = Some(Default::default());
                        let sonuc =
                            DiscoverResult::new(vec![ProtocolVersion::V_2026_07_28], yetenekler)
                                .with_server_info(Implementation::new("kopan-sunucu", "1.0"));
                        http_json_yaniti_yaz(
                            &mut akis,
                            "200 OK",
                            &[],
                            &json!({"jsonrpc": "2.0", "id": id, "result": sonuc}),
                        )
                        .await;
                    }
                    Some("tools/call") => {
                        cagri_sayaci_sunucu.fetch_add(1, Ordering::SeqCst);
                        let baslik = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n: islem-basladi\n\n";
                        akis.write_all(baslik.as_bytes()).await.unwrap();
                        akis.flush().await.unwrap();
                        // Son JSON-RPC yanıtı yazılmadan bağlantıyı kapat.
                        drop(akis);
                    }
                    _ => {}
                }
            }
        });

        let hedef = Url::parse(&format!("http://{adres}/mcp")).unwrap();
        let (tasima, hata) =
            SinirliHttpTasima::yeni(test_http_istemcisi(), hedef, None, Duration::from_secs(3));

        let hizmet = SahteIstemci
            .serve_with_lifecycle(
                tasima,
                ClientLifecycleMode::Auto {
                    preferred_versions: vec![ProtocolVersion::V_2026_07_28],
                    legacy_version: Some(ProtocolVersion::V_2025_11_25),
                },
            )
            .await
            .unwrap();

        let cagri_sonucu = hizmet
            .call_tool(CallToolRequestParams::new("yazma_araci"))
            .await;
        assert!(cagri_sonucu.is_err());
        assert_eq!(
            cagri_sayaci.load(Ordering::SeqCst),
            1,
            "bağlantı koptuğunda yan etkili araç çağrısı otomatik tekrar edilmemelidir"
        );
        assert!(matches!(
            &*hata.borrow(),
            Some(HttpKesinti::BelirsizCagriSonucu { arac_adi, .. }) if arac_adi == "yazma_araci"
        ));

        sunucu.abort();
    }

    #[tokio::test]
    async fn zaman_asimi_ve_401_yetki_hatasi_ayristirilir() {
        // 1. 401 WWW-Authenticate algılama
        {
            let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let adres = dinleyici.local_addr().unwrap();
            tokio::spawn(async move {
                if let Ok((mut akis, _)) = dinleyici.accept().await {
                    let _ = http_istegi_oku(&mut akis).await;
                    let yanit = "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Bearer resource_metadata=\"https://ornek.com/.well-known/oauth-protected-resource\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                    let _ = akis.write_all(yanit.as_bytes()).await;
                }
            });
            let (tasima, hata) = SinirliHttpTasima::yeni(
                test_http_istemcisi(),
                Url::parse(&format!("http://{adres}/mcp")).unwrap(),
                None,
                Duration::from_secs(2),
            );
            let sonuc = SahteIstemci
                .serve_with_lifecycle(
                    tasima,
                    ClientLifecycleMode::Auto {
                        preferred_versions: vec![ProtocolVersion::V_2026_07_28],
                        legacy_version: Some(ProtocolVersion::V_2025_11_25),
                    },
                )
                .await;
            assert!(sonuc.is_err());
            assert_eq!(
                *hata.borrow(),
                Some(HttpKesinti::YetkiGerekli {
                    durum: 401,
                    www_authenticate: Some(
                        "Bearer resource_metadata=\"https://ornek.com/.well-known/oauth-protected-resource\""
                            .to_string()
                    )
                })
            );
        }

        // 2. Araç çağrısında zaman aşımı -> BelirsizCagriSonucu ve tek deneme
        {
            let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let adres = dinleyici.local_addr().unwrap();
            let cagri_sayaci = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let cagri_sayaci_sunucu = cagri_sayaci.clone();
            let sunucu = tokio::spawn(async move {
                while let Ok((mut akis, _)) = dinleyici.accept().await {
                    let Some(istek) = http_istegi_oku(&mut akis).await else {
                        continue;
                    };
                    let id = istek.govde["id"].clone();
                    match istek.govde["method"].as_str() {
                        Some("server/discover") => {
                            let mut yetenekler = ServerCapabilities::default();
                            yetenekler.tools = Some(Default::default());
                            let sonuc = DiscoverResult::new(
                                vec![ProtocolVersion::V_2026_07_28],
                                yetenekler,
                            )
                            .with_server_info(Implementation::new("yavas-sunucu", "1.0"));
                            http_json_yaniti_yaz(
                                &mut akis,
                                "200 OK",
                                &[],
                                &json!({"jsonrpc": "2.0", "id": id, "result": sonuc}),
                            )
                            .await;
                        }
                        Some("tools/call") => {
                            cagri_sayaci_sunucu.fetch_add(1, Ordering::SeqCst);
                            tokio::time::sleep(Duration::from_secs(2)).await;
                        }
                        _ => {}
                    }
                }
            });

            let (tasima, hata) = SinirliHttpTasima::yeni(
                test_http_istemcisi(),
                Url::parse(&format!("http://{adres}/mcp")).unwrap(),
                None,
                Duration::from_millis(200),
            );
            let hizmet = SahteIstemci
                .serve_with_lifecycle(
                    tasima,
                    ClientLifecycleMode::Auto {
                        preferred_versions: vec![ProtocolVersion::V_2026_07_28],
                        legacy_version: Some(ProtocolVersion::V_2025_11_25),
                    },
                )
                .await
                .unwrap();
            let sonuc = hizmet
                .call_tool(CallToolRequestParams::new("yavas_yazma"))
                .await;
            assert!(sonuc.is_err());
            assert_eq!(cagri_sayaci.load(Ordering::SeqCst), 1);
            assert!(matches!(
                &*hata.borrow(),
                Some(HttpKesinti::BelirsizCagriSonucu { arac_adi, .. }) if arac_adi == "yavas_yazma"
            ));
            sunucu.abort();
        }
    }
}
