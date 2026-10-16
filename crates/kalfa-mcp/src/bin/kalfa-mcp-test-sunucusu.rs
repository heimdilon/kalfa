use rmcp::model::{
    DiscoverResult, Implementation, ProtocolVersion, ServerCapabilities, SubscriptionsListenResult,
};
use serde_json::{Value, json};
use std::error::Error;
use std::io::{self, BufRead, Write};
use std::process::{Command, Stdio};
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
        thread::sleep(Duration::from_millis(200));
        yazi(json!({
            "jsonrpc": "2.0",
            "id": kimlik,
            "result": {"content": [{"type": "text", "text": "gecikmeli çocuk yazdı"}], "isError": false}
        }))?;
        return Ok(());
    }

    let stdin = io::stdin();
    let mut bekleyen_istek = None;
    let mut cagri_sayisi = 0usize;
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
                    let sonuc = arac_sayfasi(&kip, &ileti)?;
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
                            Command::new(std::env::current_exe()?)
                                .arg("--cocuk-yaz")
                                .arg(kimlik)
                                .stdin(Stdio::null())
                                .stdout(Stdio::inherit())
                                .stderr(Stdio::null())
                                .spawn()?;
                        }
                        "secret-error" => yazi(json!({
                            "jsonrpc":"2.0",
                            "id":kimlik,
                            "error":{"code":-32000,"message":"gizli-yansi","data":{"sır":"gizli-yansi"}}
                        }))?,
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
