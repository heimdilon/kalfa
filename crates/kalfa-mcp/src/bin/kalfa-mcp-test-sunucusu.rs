use rmcp::model::{
    DiscoverResult, Implementation, ProtocolVersion, ServerCapabilities, SubscriptionsListenResult,
};
use serde_json::{Value, json};
use std::error::Error;
use std::io::{self, BufRead, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let kip = args.next().unwrap_or_else(|| "modern".to_owned());
    if kip == "--cocuk-yaz" {
        let kimlik = args
            .next()
            .ok_or_else(|| io::Error::other("yanıt kimliği verilmedi"))?;
        let kimlik: Value = serde_json::from_str(&kimlik)?;
        let isaret = args.next();
        let gecikme_ms = args
            .next()
            .and_then(|deger| deger.parse::<u64>().ok())
            .unwrap_or(200);
        thread::sleep(Duration::from_millis(gecikme_ms));
        if let Some(yol) = isaret {
            std::fs::write(yol, b"gecikmeli torun yazdi")?;
        }
        yazi(json!({
            "jsonrpc": "2.0",
            "id": kimlik,
            "result": {"content": [{"type": "text", "text": "gecikmeli çocuk yazdı"}], "isError": false}
        }))?;
        return Ok(());
    }
    let ek_argumanlar: Vec<String> = args.collect();

    let stdin = io::stdin();
    let mut bekleyen_istek = None;
    let mut cagri_sayisi = 0usize;
    let sinama_durumu = Arc::new(SinamaDurumu::default());
    for satir in stdin.lock().lines() {
        let ilet = satir?;
        let ileti: Value = serde_json::from_str(&ilet)?;
        let yontem = ileti.get("method").and_then(Value::as_str);
        let kimlik = ileti.get("id").cloned();
        match yontem {
            Some("server/discover") if matches!(kip.as_str(), "legacy" | "slow-initialize") => {
                if let Some(kimlik) = kimlik {
                    yazi(
                        json!({"jsonrpc":"2.0","id":kimlik,"error":{"code":-32601,"message":"eski uç"}}),
                    )?;
                }
            }
            Some("server/discover") => {
                if let Some(kimlik) = kimlik {
                    let mut yetenekler = ServerCapabilities::default();
                    yetenekler.tools = Some(Default::default());
                    let mut sonuc =
                        DiscoverResult::new(vec![ProtocolVersion::V_2026_07_28], yetenekler)
                            .with_server_info(Implementation::new("gizli-yansi", "1"));
                    sonuc.instructions = Some("gizli-yansi yönergesi".to_owned());
                    yazi(json!({
                        "jsonrpc": "2.0",
                        "id": kimlik,
                        "result": serde_json::to_value(sonuc)?
                    }))?;
                }
            }
            Some("initialize") => {
                if kip == "slow-initialize" {
                    thread::sleep(Duration::from_secs(26));
                }
                if ileti.pointer("/params/capabilities") != Some(&json!({})) {
                    return Err(io::Error::other("istemci yetenekleri boş olmalı").into());
                }
                if let Some(kimlik) = kimlik {
                    yazi(json!({
                        "jsonrpc":"2.0",
                        "id":kimlik,
                        "result":{
                            "protocolVersion":"2025-11-25",
                            "capabilities":{"tools":{"listChanged":true}},
                            "serverInfo":{"name":"gizli-yansi","version":"1"},
                            "instructions":"gizli-yansi yönergesi"
                        }
                    }))?;
                }
            }
            Some("notifications/initialized") => {}
            Some("subscriptions/listen") => {
                if let Some(kimlik) = kimlik {
                    if let Ok(mut a) = sinama_durumu.abonelik.lock() {
                        *a = Some(kimlik.clone());
                    }
                    let bildirimler = if kip == "empty-ack" {
                        json!({})
                    } else {
                        ileti
                            .pointer("/params/notifications")
                            .cloned()
                            .unwrap_or_else(|| json!({}))
                    };
                    let onay_kimligi = if kip == "wrong-ack" {
                        json!(99999)
                    } else {
                        kimlik.clone()
                    };
                    let onay = json!({
                        "jsonrpc":"2.0",
                        "method":"notifications/subscriptions/acknowledged",
                        "params":{
                            "notifications":bildirimler,
                            "_meta":{
                                "io.modelcontextprotocol/subscriptionId":onay_kimligi,
                                "fixture":"gizli-yansi"
                            }
                        }
                    });
                    yazi(onay.clone())?;
                    if kip == "double-ack" {
                        yazi(onay)?;
                    }
                    if kip == "subscription-end" {
                        yazi(json!({"jsonrpc":"2.0", "id":kimlik,
                            "result":SubscriptionsListenResult::complete(serde_json::from_value(kimlik.clone())?)}))?;
                    }
                    if kip == "tool-change" {
                        yazi(json!({
                            "jsonrpc":"2.0",
                            "method":"notifications/tools/list_changed",
                            "params":{"_meta":{"io.modelcontextprotocol/subscriptionId":kimlik}}
                        }))?;
                    }
                }
            }
            Some("tools/list") => {
                if let Some(kimlik) = kimlik {
                    stderr_satiri("LIST")?;
                    let sonuc = if kip == "sinama" {
                        json!({"tools":[sinama_araci(&sinama_durumu.aciklama())]})
                    } else {
                        arac_sayfasi(&kip, &ileti)?
                    };
                    yazi(json!({"jsonrpc":"2.0","id":kimlik,"result":sonuc}))?;
                    if kip == "server-requests" {
                        yazi(json!({"jsonrpc":"2.0","id":9001,"method":"ping","params":{}}))?;
                        yazi(json!({"jsonrpc":"2.0","id":9002,"method":"roots/list","params":{}}))?;
                        yazi(
                            json!({"jsonrpc":"2.0","id":9003,"method":"sampling/createMessage","params":{"messages":[],"maxTokens":1}}),
                        )?;
                        yazi(
                            json!({"jsonrpc":"2.0","id":9004,"method":"elicitation/create","params":{"message":"sinama","requestedSchema":{}}}),
                        )?;
                    }
                }
            }
            Some("tools/call") => {
                if let Some(kimlik) = kimlik {
                    cagri_sayisi += 1;
                    stderr_satiri(&format!("CALL {cagri_sayisi}"))?;
                    match kip.as_str() {
                        "late-response" | "ignore-cancel" | "silent-cancel" | "cancel-response" => {
                            bekleyen_istek = Some(kimlik)
                        }
                        "input-required" => yazi(json!({
                            "jsonrpc":"2.0", "id":kimlik,
                            "result":{"resultType":"input_required", "requestState":"gizli-yansi"}
                        }))?,
                        "delayed-child-write" => {
                            let kimlik = serde_json::to_string(&kimlik)?;
                            let mut komut = Command::new(std::env::current_exe()?);
                            komut
                                .arg("--cocuk-yaz")
                                .arg(kimlik)
                                .stdin(Stdio::null())
                                .stdout(Stdio::inherit())
                                .stderr(Stdio::null());
                            if let Some(isaret) = ek_argumanlar.first() {
                                komut
                                    .arg(isaret)
                                    .arg(ek_argumanlar.get(1).map(String::as_str).unwrap_or("200"));
                            }
                            komut.spawn()?;
                        }
                        "secret-error" => yazi(json!({
                            "jsonrpc":"2.0",
                            "id":kimlik,
                            "error":{"code":-32000,"message":"gizli-yansi","data":{"sır":"gizli-yansi"}}
                        }))?,
                        "sinama" => {
                            let metin = sinama(&ileti, &sinama_durumu);
                            yazi(json!({
                                "jsonrpc":"2.0",
                                "id":kimlik,
                                "result":{"content":[{"type":"text","text":metin}],"isError":false}
                            }))?
                        }
                        "secrets" => yazi(json!({
                            "jsonrpc":"2.0",
                            "id":kimlik,
                            "result":{"content":[{"type":"text","text":"gizli-yansi"}],"isError":false}
                        }))?,
                        _ => yazi(json!({
                            "jsonrpc":"2.0",
                            "id":kimlik,
                            "result":{"content":[{"type":"text","text":"tamam"}],"isError":false}
                        }))?,
                    }
                }
            }
            Some("notifications/cancelled") => {
                stderr_satiri("CANCEL")?;
                if let Some(bekleyen) = bekleyen_istek.take() {
                    if ileti.pointer("/params/requestId") == Some(&bekleyen)
                        && kip == "late-response"
                    {
                        thread::spawn(move || {
                            thread::sleep(Duration::from_millis(200));
                            let _ = yazi(json!({
                                "jsonrpc":"2.0",
                                "id":bekleyen,
                                "result":{"content":[{"type":"text","text":"geç yanıt"}],"isError":false}
                            }));
                        });
                    } else if kip == "cancel-response" {
                        yazi(json!({"jsonrpc":"2.0", "id":bekleyen,
                            "error":{"code":-32800,"message":"iptal edildi"}}))?;
                    } else if kip != "silent-cancel" {
                        bekleyen_istek = Some(bekleyen);
                    }
                }
            }
            None => {}
            _ => {
                if let Some(kimlik) = kimlik {
                    yazi(
                        json!({"jsonrpc":"2.0","id":kimlik,"error":{"code":-32601,"message":"Yöntem desteklenmiyor"}}),
                    )?;
                }
            }
        }
        if yontem.is_none() {
            if ileti.get("id") == Some(&json!(9001)) && ileti.get("result").is_some() {
                stderr_satiri("PING OK")?;
            } else if ileti.get("id") == Some(&json!(9002))
                && ileti.pointer("/error/code") == Some(&json!(-32601))
            {
                stderr_satiri("ROOTS RED")?;
            } else if ileti.get("id") == Some(&json!(9003))
                && ileti.pointer("/error/code") == Some(&json!(-32601))
            {
                stderr_satiri("SAMPLING RED")?;
            } else if ileti.get("id") == Some(&json!(9004))
                && ileti.pointer("/error/code") == Some(&json!(-32601))
            {
                stderr_satiri("ELICITATION RED")?;
            }
        }
    }
    Ok(())
}

fn arac_sayfasi(kip: &str, ileti: &Value) -> Result<Value, Box<dyn Error>> {
    let cursor = ileti.pointer("/params/cursor").and_then(Value::as_str);
    if kip == "many-pages" || kip == "pagination" {
        let sayfa = cursor.and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
        let arac = arac_tanimi(&format!("arac-{sayfa}"), "");
        let devam = if kip == "many-pages" {
            (sayfa < 10).then(|| (sayfa + 1).to_string())
        } else {
            (sayfa == 0).then(|| "1".to_owned())
        };
        let mut sonuc = json!({"tools":[arac]});
        if let Some(devam) = devam {
            sonuc["nextCursor"] = json!(devam);
        }
        return Ok(sonuc);
    }
    if kip == "129-tools" {
        return Ok(
            json!({"tools":(0..129).map(|i| arac_tanimi(&format!("arac-{i}"), "")).collect::<Vec<_>>()}),
        );
    }
    if kip == "big-definition" {
        return Ok(json!({"tools":[arac_tanimi("ornek", &"x".repeat(256 * 1024))]}));
    }
    let aciklama = if kip == "secret-definition" || kip == "secrets" {
        "gizli-yansi"
    } else {
        ""
    };
    Ok(json!({
        "tools":[arac_tanimi("ornek", aciklama)],
        "ttlMs":60000
    }))
}

/// Gerçek yalıtım kabulünün yoklama aracı (Görev 12). Sonuç `TAMAM:...` ya
/// da `RED:...` metnidir; işletim sisteminin verdiği kararı aynen bildirir.
fn sinama_araci(aciklama: &str) -> Value {
    json!({
        "name":"dene",
        "description":aciklama,
        "inputSchema":{"type":"object","properties":{
            "islem":{"type":"string"},
            "yol":{"type":"string"},
            "deger":{"type":"string"},
            "hedef":{"type":"string"}
        }}
    })
}

/// `sinama` kipinin paylaşılan durumu: izleme iş parçacığı da açıklamayı
/// değiştirir ve liste değişikliğini bildirir.
#[derive(Default)]
struct SinamaDurumu {
    aciklama: Mutex<String>,
    abonelik: Mutex<Option<Value>>,
}

impl SinamaDurumu {
    fn aciklama(&self) -> String {
        self.aciklama.lock().map(|a| a.clone()).unwrap_or_default()
    }

    /// Açıklamayı değiştirir; abonelik varsa istemci listeyi yenilesin.
    fn aciklamayi_koy(&self, icerik: &str) -> io::Result<()> {
        if let Ok(mut a) = self.aciklama.lock() {
            icerik.clone_into(&mut a);
        }
        let abonelik = self.abonelik.lock().ok().and_then(|a| a.clone());
        if let Some(abonelik) = abonelik {
            yazi(json!({
                "jsonrpc":"2.0",
                "method":"notifications/tools/list_changed",
                "params":{"_meta":{"io.modelcontextprotocol/subscriptionId":abonelik}}
            }))?;
        }
        Ok(())
    }
}

/// Dosya görünene kadar (en çok 20 sn) bekler: içerik çağrının ya da
/// izlemenin sınama penceresinde gelir.
fn gorunene_kadar_oku(yol: &str) -> io::Result<String> {
    for _ in 0..200 {
        if let Ok(icerik) = std::fs::read_to_string(yol) {
            return Ok(icerik);
        }
        thread::sleep(Duration::from_millis(100));
    }
    std::fs::read_to_string(yol)
}

fn sinama(ileti: &Value, durum: &Arc<SinamaDurumu>) -> String {
    let arg = |ad: &str| {
        ileti
            .pointer(&format!("/params/arguments/{ad}"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    let sonuc = match arg("islem").as_str() {
        "pid" => Ok(std::process::id().to_string()),
        "oku" => std::fs::read_to_string(arg("yol")),
        "bekleyip_oku" => gorunene_kadar_oku(&arg("yol")),
        "yaz" => std::fs::write(arg("yol"), arg("deger")).map(|()| String::new()),
        "aciklamaya_koy" => std::fs::read_to_string(arg("yol")).and_then(|icerik| {
            durum.aciklamayi_koy(&icerik)?;
            Ok(icerik)
        }),
        // Çağrı hemen döner; dosya sonradan gelince açıklamaya konur.
        "izle_aciklamaya_koy" => {
            let (yol, durum) = (arg("yol"), Arc::clone(durum));
            thread::spawn(move || {
                if let Ok(icerik) = gorunene_kadar_oku(&yol) {
                    let _ = durum.aciklamayi_koy(&icerik);
                }
            });
            Ok(String::new())
        }
        "dogrudan" => dogrudan_baglan(&arg("hedef")).map(|()| String::new()),
        "vekil" => vekilden_al(&arg("hedef")),
        diger => Err(io::Error::other(format!("bilinmeyen işlem {diger}"))),
    };
    match sonuc {
        Ok(metin) => format!("TAMAM:{metin}"),
        Err(hata) => format!("RED:{:?}", hata.kind()),
    }
}

fn dogrudan_baglan(hedef: &str) -> io::Result<()> {
    use std::net::ToSocketAddrs;
    let adres = hedef
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::other("adres yok"))?;
    std::net::TcpStream::connect_timeout(&adres, Duration::from_secs(3)).map(drop)
}

/// `HTTP_PROXY` üzerinden `CONNECT` tüneli ve `GET /`. Vekilin yanıt satırı
/// (izinsizde 403) ya da hedefin gövdesi döner.
fn vekilden_al(hedef: &str) -> io::Result<String> {
    use std::io::Read;
    let vekil = ["HTTP_PROXY", "http_proxy"]
        .iter()
        .find_map(|ad| std::env::var(ad).ok())
        .ok_or_else(|| io::Error::other("HTTP_PROXY yok"))?;
    let vekil = vekil.trim_start_matches("http://").trim_end_matches('/');
    let mut soket = std::net::TcpStream::connect(vekil)?;
    soket.set_read_timeout(Some(Duration::from_secs(10)))?;
    // İstek tek parçada gider; `write!` biçim parçalarını ayrı ayrı yazar.
    soket.write_all(format!("CONNECT {hedef} HTTP/1.1\r\nHost: {hedef}\r\n\r\n").as_bytes())?;
    let mut tampon = [0_u8; 1024];
    let n = soket.read(&mut tampon)?;
    let yanit = String::from_utf8_lossy(&tampon[..n]).into_owned();
    let durum = yanit.lines().next().unwrap_or("").to_owned();
    if !durum.contains(" 200") {
        return Ok(durum);
    }
    let host = hedef.rsplit_once(':').map_or(hedef, |(h, _)| h);
    soket.write_all(
        format!("GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n").as_bytes(),
    )?;
    // Tünel karşı tarafın kapanışını hemen iletmeyebilir: gövde
    // `Content-Length` kadar okunur, EOF beklenmez.
    let mut ham = Vec::new();
    loop {
        let metin = String::from_utf8_lossy(&ham).into_owned();
        if let Some((baslik, govde)) = metin.split_once("\r\n\r\n") {
            let uzunluk = baslik
                .lines()
                .find_map(|s| s.strip_prefix("Content-Length: "))
                .and_then(|s| s.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if govde.len() >= uzunluk {
                return Ok(govde[..uzunluk].to_owned());
            }
        }
        let n = soket.read(&mut tampon)?;
        if n == 0 {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        ham.extend_from_slice(&tampon[..n]);
    }
}

fn arac_tanimi(ad: &str, aciklama: &str) -> Value {
    json!({
        "name":ad,
        "description":aciklama,
        "inputSchema":{"type":"object","properties":{}}
    })
}

fn yazi(ileti: Value) -> io::Result<()> {
    let stdout = io::stdout();
    let mut kilit = stdout.lock();
    serde_json::to_writer(&mut kilit, &ileti).map_err(io::Error::other)?;
    kilit.write_all(b"\n")?;
    kilit.flush()
}

fn stderr_satiri(metin: &str) -> io::Result<()> {
    let stderr = io::stderr();
    let mut kilit = stderr.lock();
    writeln!(kilit, "{metin}")?;
    kilit.flush()
}
