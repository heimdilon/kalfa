//! G5'in gerçek TCP/JSON-RPC sunucusu. Ürün istemcisi taklit edilmez.
use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub struct McpSunucusu {
    pub port: u16,
    pub sayac: Arc<AtomicUsize>,
    pub cagrilar: Arc<Mutex<Vec<Value>>>,
    gorev: tokio::task::JoinHandle<()>,
}

impl Drop for McpSunucusu {
    fn drop(&mut self) {
        self.gorev.abort();
    }
}

pub fn arac(ad: &str, salt_okunur: bool) -> Value {
    json!({"name":ad,"description":format!("G5 {ad}"),"inputSchema":{"type":"object","properties":{"kayit":{"type":"string"},"durum":{"type":"string"},"sorgu":{"type":"string"}}},"annotations":{"readOnlyHint":salt_okunur}})
}

pub async fn baslat(araclar: Vec<Value>, metin: &str) -> McpSunucusu {
    let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = dinleyici.local_addr().unwrap().port();
    let sayac = Arc::new(AtomicUsize::new(0));
    let cagrilar = Arc::new(Mutex::new(Vec::new()));
    let durum = Arc::new(Mutex::new(metin.to_owned()));
    let araclar = Arc::new(araclar);
    let sayac_kopya = sayac.clone();
    let cagri_kopya = cagrilar.clone();
    let gorev = tokio::spawn(async move {
        loop {
            let Ok((mut soket, _)) = dinleyici.accept().await else {
                break;
            };
            let sayac = sayac_kopya.clone();
            let cagrilar = cagri_kopya.clone();
            let araclar = araclar.clone();
            let durum = durum.clone();
            tokio::spawn(async move {
                let mut veri = Vec::new();
                let mut parca = [0; 4096];
                let baslik_sonu = loop {
                    let n = soket.read(&mut parca).await.unwrap();
                    if n == 0 || veri.len() > 1_048_576 {
                        return;
                    }
                    veri.extend_from_slice(&parca[..n]);
                    if let Some(n) = veri.windows(4).position(|s| s == b"\r\n\r\n") {
                        break n + 4;
                    }
                };
                let uzunluk: usize = String::from_utf8_lossy(&veri[..baslik_sonu])
                    .lines()
                    .find_map(|s| {
                        let (ad, deger) = s.split_once(':')?;
                        ad.eq_ignore_ascii_case("content-length")
                            .then(|| deger.trim().parse().ok())
                            .flatten()
                    })
                    .unwrap_or(0);
                if uzunluk > 1_048_576 {
                    return;
                }
                while veri.len() < baslik_sonu + uzunluk {
                    let n = soket.read(&mut parca).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    veri.extend_from_slice(&parca[..n]);
                }
                let istek: Value =
                    serde_json::from_slice(&veri[baslik_sonu..baslik_sonu + uzunluk])
                        .unwrap_or(Value::Null);
                let sonuc = match istek["method"].as_str().unwrap_or_default() {
                    "initialize" => {
                        json!({"protocolVersion":"2025-11-25","capabilities":{"tools":{}},"serverInfo":{"name":"g5","version":"2"}})
                    }
                    "tools/list" => json!({"tools":*araclar}),
                    "tools/call" => {
                        let ad = istek["params"]["name"].as_str().unwrap();
                        assert!(araclar.iter().any(|a| a["name"] == ad));
                        sayac.fetch_add(1, Ordering::SeqCst);
                        cagrilar.lock().unwrap().push(istek["params"].clone());
                        if ad == "kayit_guncelle" {
                            assert_eq!(istek["params"]["arguments"]["kayit"], "gorev-42");
                            *durum.lock().unwrap() = istek["params"]["arguments"]["durum"]
                                .as_str()
                                .unwrap()
                                .to_owned();
                        }
                        let metin = if ad.starts_with("surum_") {
                            format!("KLF-M05-{ad}")
                        } else {
                            durum.lock().unwrap().clone()
                        };
                        json!({"content":[{"type":"text","text":metin}]})
                    }
                    _ => Value::Null,
                };
                let (durum, govde) = if istek.get("id").is_none() {
                    ("202 Accepted", String::new())
                } else if sonuc.is_null() {
                    ("200 OK",json!({"jsonrpc":"2.0","id":istek["id"],"error":{"code":-32601,"message":"Metod desteklenmiyor"}}).to_string())
                } else {
                    (
                        "200 OK",
                        json!({"jsonrpc":"2.0","id":istek["id"],"result":sonuc}).to_string(),
                    )
                };
                let yanit = format!(
                    "HTTP/1.1 {durum}\r\nContent-Type: application/json\r\nMcp-Session-Id: g5-oturum\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{govde}",
                    govde.len()
                );
                let _ = soket.write_all(yanit.as_bytes()).await;
            });
        }
    });
    McpSunucusu {
        port,
        sayac,
        cagrilar,
        gorev,
    }
}
