//! 1.1a-2 taşıma ön doğrulaması; ürün kodu değildir.
use rmcp::RoleClient;
use rmcp::model::{
    ClientJsonRpcMessage, ClientResult, ErrorCode, ErrorData, ServerJsonRpcMessage,
    ServerNotification, ServerRequest,
};
use rmcp::transport::Transport;
use std::io;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

pub const CERCEVE: usize = 4 * 1024 * 1024;
pub const KUYRUK: usize = 64;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kesinti {
    Cerceve,
    YarimSatir,
    Json,
    IletiSeli,
    GirdiCikti,
}

/// Ayrıştırmadan önce en çok sınır kadar veri biriktirir.
pub async fn satir<R: AsyncBufRead + Unpin>(
    r: &mut R,
    sinir: usize,
) -> Result<Option<Vec<u8>>, Kesinti> {
    let mut veri = Vec::new();
    loop {
        let gelen = r.fill_buf().await.map_err(|_| Kesinti::GirdiCikti)?;
        if gelen.is_empty() {
            return if veri.is_empty() {
                Ok(None)
            } else {
                Err(Kesinti::YarimSatir)
            };
        }
        let son = gelen.iter().position(|b| *b == b'\n');
        let n = son.unwrap_or(gelen.len());
        if n > sinir.saturating_sub(veri.len()) {
            return Err(Kesinti::Cerceve);
        }
        veri.extend_from_slice(&gelen[..n]);
        r.consume(n + usize::from(son.is_some()));
        if son.is_some() {
            return Ok(Some(veri));
        }
    }
}

/// Ham AsyncRead rmcp'ye verilmez. Hata kanalı süreç sahibine de verilir.
pub struct SinirliTasima {
    pub yenileme: Arc<AtomicBool>,
    gelen: mpsc::Receiver<ServerJsonRpcMessage>,
    giden: mpsc::Sender<ClientJsonRpcMessage>,
    hata: watch::Receiver<Option<Kesinti>>,
    gorevler: Vec<JoinHandle<()>>,
}
impl SinirliTasima {
    pub fn yeni<R, W>(r: R, mut w: W) -> (Self, watch::Receiver<Option<Kesinti>>)
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (tx, gelen) = mpsc::channel(KUYRUK);
        let (giden, mut rx) = mpsc::channel::<ClientJsonRpcMessage>(KUYRUK);
        let (hata_tx, hata) = watch::channel(None);
        let cevap = giden.clone();
        let yenileme = Arc::new(AtomicBool::new(false));
        let yenileme_okuma = yenileme.clone();
        let okuma_hatasi = hata_tx.clone();
        let okuma = tokio::spawn(async move {
            let mut r = BufReader::new(r);
            let mut abonelik_onayi = false;
            loop {
                let ham = match satir(&mut r, CERCEVE).await {
                    Ok(Some(h)) => h,
                    Ok(None) => break,
                    Err(e) => {
                        let _ = okuma_hatasi.send(Some(e));
                        break;
                    }
                };
                let ileti: ServerJsonRpcMessage = match serde_json::from_slice(&ham) {
                    Ok(i) => i,
                    Err(_) => {
                        let _ = okuma_hatasi.send(Some(Kesinti::Json));
                        break;
                    }
                };
                // SDK'ye sınırsız işleyici görevleri üretebilen bildirim/istek akışı verilmez.
                if let ServerJsonRpcMessage::Notification(ref n) = ileti {
                    match &n.notification {
                        ServerNotification::ToolListChangedNotification(_) => {
                            yenileme_okuma.store(true, Ordering::SeqCst);
                            continue;
                        }
                        ServerNotification::SubscriptionsAcknowledgedNotification(_)
                            if !abonelik_onayi =>
                        {
                            abonelik_onayi = true;
                        }
                        ServerNotification::SubscriptionsAcknowledgedNotification(_) => {
                            let _ = okuma_hatasi.send(Some(Kesinti::IletiSeli));
                            break;
                        }
                        _ => continue,
                    }
                }
                if let ServerJsonRpcMessage::Request(ref istek) = ileti {
                    let i = if matches!(istek.request, ServerRequest::PingRequest(_)) {
                        ClientJsonRpcMessage::response(ClientResult::empty(()), istek.id.clone())
                    } else {
                        ClientJsonRpcMessage::error(
                            ErrorData::new(
                                ErrorCode::METHOD_NOT_FOUND,
                                "Yöntem desteklenmiyor",
                                None,
                            ),
                            Some(istek.id.clone()),
                        )
                    };
                    if cevap.try_send(i).is_err() {
                        let _ = okuma_hatasi.send(Some(Kesinti::IletiSeli));
                        break;
                    }
                    continue;
                }
                if tx.try_send(ileti).is_err() {
                    let _ = okuma_hatasi.send(Some(Kesinti::IletiSeli));
                    break;
                }
            }
        });
        let yazma = tokio::spawn(async move {
            while let Some(i) = rx.recv().await {
                let mut ham = match serde_json::to_vec(&i) {
                    Ok(h) => h,
                    Err(_) => {
                        let _ = hata_tx.send(Some(Kesinti::Json));
                        break;
                    }
                };
                ham.push(b'\n');
                if w.write_all(&ham).await.is_err() || w.flush().await.is_err() {
                    let _ = hata_tx.send(Some(Kesinti::GirdiCikti));
                    break;
                }
            }
        });
        (
            Self {
                yenileme,
                gelen,
                giden,
                hata: hata.clone(),
                gorevler: vec![okuma, yazma],
            },
            hata,
        )
    }
}
impl Transport<RoleClient> for SinirliTasima {
    type Error = io::Error;
    fn send(
        &mut self,
        item: ClientJsonRpcMessage,
    ) -> impl Future<Output = io::Result<()>> + Send + 'static {
        let tx = self.giden.clone();
        async move {
            tx.send(item)
                .await
                .map_err(|_| io::Error::other("taşıma kapandı"))
        }
    }
    async fn receive(&mut self) -> Option<ServerJsonRpcMessage> {
        if self.hata.borrow().is_some() {
            return None;
        }
        tokio::select! { biased; _ = self.hata.changed() => None, i = self.gelen.recv() => i }
    }
    async fn close(&mut self) -> io::Result<()> {
        for g in &self.gorevler {
            g.abort();
        }
        Ok(())
    }
}
impl Drop for SinirliTasima {
    fn drop(&mut self) {
        for g in &self.gorevler {
            g.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::{
        DiscoverResult, GetMeta, Implementation, ProtocolVersion, ServerCapabilities,
        ServerNotification, SubscriptionFilter, SubscriptionsAcknowledgedNotification,
        SubscriptionsAcknowledgedNotificationParams, ToolListChangedNotification,
    };
    use rmcp::service::NotificationContext;
    use rmcp::{ClientHandler, ClientLifecycleMode, ClientServiceExt};
    use serde_json::{Value, json};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    #[derive(Clone, Default)]
    struct Istemci(Arc<std::sync::atomic::AtomicBool>);
    impl ClientHandler for Istemci {
        async fn on_tool_list_changed(&self, _: NotificationContext<RoleClient>) {
            self.0.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }
    async fn yaz<W: AsyncWrite + Unpin>(w: &mut W, v: Value) {
        let mut h = serde_json::to_vec(&v).unwrap();
        h.push(b'\n');
        w.write_all(&h).await.unwrap();
    }
    #[tokio::test]
    async fn sinirda_gecer_fazla_bayt_yeni_satirsiz_reddedilir() {
        let h = [vec![b'x'; CERCEVE], vec![b'\n']].concat();
        assert_eq!(
            satir(&mut &h[..], CERCEVE).await.unwrap().unwrap().len(),
            CERCEVE
        );
        let h = vec![b'x'; CERCEVE + 1];
        assert_eq!(satir(&mut &h[..], CERCEVE).await, Err(Kesinti::Cerceve));
    }
    #[tokio::test]
    async fn bolunmus_utf8_ve_cok_satir_birebir_yarim_satir_hata() {
        let (mut w, r) = tokio::io::duplex(1);
        let g = tokio::spawn(async move {
            for b in "İğüşöç\nson\n".as_bytes() {
                w.write_all(&[*b]).await.unwrap();
            }
        });
        let mut r = BufReader::new(r);
        assert_eq!(
            satir(&mut r, 50).await.unwrap().unwrap(),
            "İğüşöç".as_bytes()
        );
        assert_eq!(satir(&mut r, 50).await.unwrap().unwrap(), b"son");
        assert_eq!(satir(&mut r, 50).await.unwrap(), None);
        g.await.unwrap();
        assert_eq!(
            satir(&mut &b"yarim"[..], 50).await,
            Err(Kesinti::YarimSatir)
        );
    }
    #[tokio::test]
    async fn altmis_besinci_ileti_kuyrugu_buyutmez() {
        let (mut w, r) = tokio::io::duplex(16384);
        let (r, y) = tokio::io::split(r);
        let (_t, mut h) = SinirliTasima::yeni(r, y);
        for _ in 0..65 {
            yaz(&mut w, json!({"jsonrpc":"2.0","id":0,"result":{}})).await;
        }
        tokio::time::timeout(Duration::from_secs(2), h.changed())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(*h.borrow(), Some(Kesinti::IletiSeli));
    }
    #[tokio::test]
    async fn bozuk_json_ham_metinsiz_kesinti_verir() {
        let (mut w, r) = tokio::io::duplex(256);
        let (r, y) = tokio::io::split(r);
        let (mut t, mut h) = SinirliTasima::yeni(r, y);
        w.write_all(b"gizli-ornek-bozuk-json\n").await.unwrap();
        h.changed().await.unwrap();
        assert_eq!(*h.borrow(), Some(Kesinti::Json));
        assert!(t.receive().await.is_none());
    }
    #[tokio::test]
    async fn bin_liste_bildirimi_tek_bayrak_olur_sdk_kuyruguna_girmez() {
        let (mut w, r) = tokio::io::duplex(16_384);
        let (r, y) = tokio::io::split(r);
        let (t, h) = SinirliTasima::yeni(r, y);
        for _ in 0..1000 {
            yaz(
                &mut w,
                json!({"jsonrpc":"2.0","method":"notifications/tools/list_changed"}),
            )
            .await;
        }
        w.shutdown().await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if t.gorevler[0].is_finished() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(t.yenileme.load(Ordering::SeqCst));
        assert!(t.gelen.is_empty());
        assert_eq!(*h.borrow(), None);
    }
    async fn donem(yeni_donem: bool) {
        let (sunucu, istemci) = tokio::io::duplex(65536);
        let (r, w) = tokio::io::split(istemci);
        let (t, _h) = SinirliTasima::yeni(r, w);
        let kayit = Arc::new(Mutex::new(Vec::<Value>::new()));
        let k2 = kayit.clone();
        let g = tokio::spawn(async move {
            let (r, mut w) = tokio::io::split(sunucu);
            let mut r = BufReader::new(r);
            while let Some(ham) = satir(&mut r, CERCEVE).await.unwrap() {
                let v: Value = serde_json::from_slice(&ham).unwrap();
                k2.lock().unwrap().push(v.clone());
                let id = v["id"].clone();
                match v["method"].as_str() {
                    Some("server/discover") if yeni_donem => {
                        let sonuc = DiscoverResult::new(vec![ProtocolVersion::V_2026_07_28], ServerCapabilities::builder().enable_tools().build()).with_server_info(Implementation::new("sinama", "0"));
                        yaz(&mut w, json!({"jsonrpc":"2.0","id":id,"result":sonuc})).await;
                    }
                    Some("server/discover") => yaz(&mut w, json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"eski sunucu"}})).await,
                    Some("initialize") => {
                        assert!(!yeni_donem); assert_eq!(v["params"]["capabilities"], json!({}));
                        yaz(&mut w, json!({"jsonrpc":"2.0","id":id,"result":{"protocolVersion":"2025-11-25","capabilities":{"tools":{"listChanged":true}},"serverInfo":{"name":"sinama","version":"0"}}})).await;
                    }
                    Some("tools/list") => {
                        yaz(&mut w, json!({"jsonrpc":"2.0","id":id,"result":{"tools":[],"ttlMs":60000}})).await;
                        if !yeni_donem { yaz(&mut w, json!({"jsonrpc":"2.0","method":"notifications/tools/list_changed"})).await; }
                        for (id, method) in [(90, "roots/list"), (91, "sampling/createMessage")] {
                            let params = if id == 91 { json!({"messages":[],"maxTokens":1}) } else { json!({}) };
                            yaz(&mut w, json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})).await;
                        }
                        yaz(&mut w, json!({"jsonrpc":"2.0","id":92,"method":"ping"})).await;
                    }
                    Some("subscriptions/listen") => {
                        let filter: SubscriptionFilter = serde_json::from_value(v["params"]["notifications"].clone()).unwrap();
                        let mut ack: ServerNotification = SubscriptionsAcknowledgedNotification::new(SubscriptionsAcknowledgedNotificationParams::new(filter)).into();
                        ack.get_meta_mut().set_subscription_id(serde_json::from_value(id.clone()).unwrap()); yaz(&mut w, serde_json::to_value(ServerJsonRpcMessage::notification(ack)).unwrap()).await;
                        let mut n = ServerNotification::ToolListChangedNotification(ToolListChangedNotification { method: Default::default(), extensions: Default::default() });
                        n.get_meta_mut().set_subscription_id(serde_json::from_value(id).unwrap()); yaz(&mut w, serde_json::to_value(ServerJsonRpcMessage::notification(n)).unwrap()).await;
                    }
                    _ => {}
                }
            }
        });
        let h = Istemci::default();
        let bayrak = t.yenileme.clone();
        let istemci_hizmeti = tokio::time::timeout(
            Duration::from_secs(3),
            h.serve_with_lifecycle(
                t,
                ClientLifecycleMode::Auto {
                    preferred_versions: vec![ProtocolVersion::V_2026_07_28],
                    legacy_version: Some(ProtocolVersion::V_2025_11_25),
                },
            ),
        )
        .await
        .unwrap()
        .unwrap();
        istemci_hizmeti
            .peer()
            .set_response_cache_config(rmcp::ClientCacheConfig::disabled())
            .await;
        assert!(
            istemci_hizmeti
                .list_tools(None)
                .await
                .unwrap()
                .tools
                .is_empty()
        );
        assert!(
            istemci_hizmeti
                .list_tools(None)
                .await
                .unwrap()
                .tools
                .is_empty()
        );
        let abonelik = if yeni_donem {
            Some(
                istemci_hizmeti
                    .listen_with_capacity(
                        SubscriptionFilter::builder().tools_list_changed().build(),
                        std::num::NonZeroUsize::new(2).unwrap(),
                    )
                    .await
                    .unwrap(),
            )
        } else {
            None
        };
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let tamam = {
                    let k = kayit.lock().unwrap();
                    [90, 91].into_iter().all(|id| {
                        k.iter()
                            .any(|v| v["id"] == id && v["error"]["code"] == -32601)
                    }) && k.iter().any(|v| v["id"] == 92 && v.get("result").is_some())
                        && bayrak.load(std::sync::atomic::Ordering::SeqCst)
                };
                if tamam {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        {
            let k = kayit.lock().unwrap();
            assert_eq!(
                k.iter().filter(|v| v["method"] == "tools/list").count(),
                2,
                "pozitif TTL ham tanımı SDK önbelleğine sokmamalı"
            );
            assert_eq!(
                k.iter().any(|v| v["method"] == "notifications/initialized"),
                !yeni_donem
            );
        }
        drop(abonelik);
        istemci_hizmeti.cancel().await.unwrap();
        g.abort();
    }
    #[tokio::test]
    async fn eski_donem_auto_geri_donus_bos_yetenek_liste_ve_red() {
        donem(false).await;
    }
    #[tokio::test]
    async fn yeni_donem_auto_abonelik_liste_ve_red() {
        donem(true).await;
    }
}
