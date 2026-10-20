//! Görev 3 (G3b): Uzak MCP Streamable HTTP taşıması entegrasyon testleri (`M02`, `M05`, `M06`).

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use kalfa_mcp::istemci::{McpHatasi, McpIstemcisi};
use kalfa_mcp::kesif::arac_tanimlarini_denetle;
use kalfa_mcp::maske::GizliDeger;
use kalfa_mcp::tanim::{UzakMcpTanim, UzakYetkiTuru};
use kalfa_mcp::tasima::TasimaHatasi;
use kalfa_mcp::uzak::{
    BelirsizCagriNedeni, SabitDnsCozucu, SinirliHttpTasima, UzakGizliBaslik, UzakHttpSecenekleri,
    dogrulanmis_istemci_ve_url_hazirla,
};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SahteUzakKip {
    Modern2026JsonVeSse,
    Legacy20251125,
    Legacy20250618,
    Legacy20250326,
    YaridaKesilenSseCagri,
    YavasCagriIptal,
    DevSseCerceveAsimi,
    BozukJsonYaniti,
    Yetki401,
    YasakliIpYonlendirme,
    FarkliOriginYonlendirme,
}

struct SahteUzakSunucu {
    adres: SocketAddr,
    arac_cagrisi_sayaci: Arc<AtomicUsize>,
    gorulen_oturum_kimlikleri: Arc<Mutex<Vec<String>>>,
    kapat_tx: Option<tokio::sync::watch::Sender<bool>>,
    gorev: tokio::task::JoinHandle<()>,
}

impl SahteUzakSunucu {
    async fn baslat(kip: SahteUzakKip) -> Self {
        let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let adres = dinleyici.local_addr().unwrap();
        let arac_cagrisi_sayaci = Arc::new(AtomicUsize::new(0));
        let gorulen_oturum_kimlikleri = Arc::new(Mutex::new(Vec::new()));
        let (kapat_tx, mut kapat_rx) = tokio::sync::watch::channel(false);

        let cagri_sayaci = arac_cagrisi_sayaci.clone();
        let oturum_listesi = gorulen_oturum_kimlikleri.clone();

        let gorev = tokio::spawn(async move {
            loop {
                let kabul = tokio::select! {
                    biased;
                    _ = kapat_rx.changed() => break,
                    sonuc = dinleyici.accept() => sonuc,
                };
                let Ok((soket, _)) = kabul else { break };
                let cagri_sayaci = cagri_sayaci.clone();
                let oturum_listesi = oturum_listesi.clone();
                tokio::spawn(async move {
                    let _ = baglantiyi_isle(soket, kip, cagri_sayaci, oturum_listesi).await;
                });
            }
        });

        Self {
            adres,
            arac_cagrisi_sayaci,
            gorulen_oturum_kimlikleri,
            kapat_tx: Some(kapat_tx),
            gorev,
        }
    }

    fn url(&self) -> String {
        format!("http://{}/mcp", self.adres)
    }

    async fn kapat(mut self) {
        if let Some(tx) = self.kapat_tx.take() {
            let _ = tx.send(true);
        }
        let _ = tokio::time::timeout(Duration::from_secs(1), self.gorev).await;
    }
}

async fn http_istegi_oku(soket: &mut TcpStream) -> Option<(HashMap<String, String>, Vec<u8>)> {
    let mut tampon = Vec::new();
    let mut gecici = [0u8; 4096];
    let baslik_sonu = loop {
        let n = soket.read(&mut gecici).await.ok()?;
        if n == 0 {
            return None;
        }
        tampon.extend_from_slice(&gecici[..n]);
        if let Some(konum) = tampon.windows(4).position(|w| w == b"\r\n\r\n") {
            break konum + 4;
        }
    };

    let baslik_metni = String::from_utf8_lossy(&tampon[..baslik_sonu]);
    let mut basliklar = HashMap::new();
    for satir in baslik_metni.lines().skip(1) {
        if let Some((ad, deger)) = satir.split_once(':') {
            basliklar.insert(ad.trim().to_ascii_lowercase(), deger.trim().to_owned());
        }
    }
    let icerik_uzunlugu: usize = basliklar
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let mut govde = tampon[baslik_sonu..].to_vec();
    while govde.len() < icerik_uzunlugu {
        let n = soket.read(&mut gecici).await.ok()?;
        if n == 0 {
            break;
        }
        govde.extend_from_slice(&gecici[..n]);
    }
    govde.truncate(icerik_uzunlugu);
    Some((basliklar, govde))
}

async fn json_yaniti_yaz(
    soket: &mut TcpStream,
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
    let _ = soket.write_all(baslik.as_bytes()).await;
    let _ = soket.write_all(&baytlar).await;
    let _ = soket.flush().await;
}

async fn baglantiyi_isle(
    mut soket: TcpStream,
    kip: SahteUzakKip,
    arac_cagrisi_sayaci: Arc<AtomicUsize>,
    gorulen_oturum_kimlikleri: Arc<Mutex<Vec<String>>>,
) -> Option<()> {
    let (basliklar, govde) = http_istegi_oku(&mut soket).await?;
    if let Some(sid) = basliklar.get("mcp-session-id") {
        gorulen_oturum_kimlikleri.lock().unwrap().push(sid.clone());
    }

    match kip {
        SahteUzakKip::Yetki401 => {
            let yanit = "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Bearer resource_metadata=\"https://mcp.example.com/.well-known/oauth-protected-resource\", scope=\"tools:read\", hint=\"gizli-canary-999\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let _ = soket.write_all(yanit.as_bytes()).await;
            return Some(());
        }
        SahteUzakKip::YasakliIpYonlendirme => {
            let yanit = "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://169.254.169.254/latest/meta-data\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let _ = soket.write_all(yanit.as_bytes()).await;
            return Some(());
        }
        SahteUzakKip::FarkliOriginYonlendirme => {
            let yanit = "HTTP/1.1 307 Temporary Redirect\r\nLocation: https://baska-origin.example.com/mcp\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let _ = soket.write_all(yanit.as_bytes()).await;
            return Some(());
        }
        _ => {}
    }

    let istek: Value = serde_json::from_slice(&govde).ok()?;
    let metod = istek
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let id = istek.get("id").cloned().unwrap_or(Value::Null);

    if metod == "notifications/cancelled" {
        let _ = soket
            .write_all(b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await;
        return Some(());
    }

    if metod == "notifications/initialized" {
        let _ = soket
            .write_all(b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await;
        return Some(());
    }

    if metod == "subscriptions/listen" {
        let _ = soket
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await;
        return Some(());
    }

    if metod == "server/discover" {
        match kip {
            SahteUzakKip::Legacy20251125
            | SahteUzakKip::Legacy20250618
            | SahteUzakKip::Legacy20250326 => {
                let _ = soket
                    .write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    )
                    .await;
                return Some(());
            }
            _ => {
                let mut yetenekler = rmcp::model::ServerCapabilities::default();
                yetenekler.tools = Some(Default::default());
                let sonuc = rmcp::model::DiscoverResult::new(
                    vec![rmcp::model::ProtocolVersion::V_2026_07_28],
                    yetenekler,
                )
                .with_server_info(rmcp::model::Implementation::new("sahte-uzak-2026", "1.0.0"));
                let yanit = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": sonuc
                });
                json_yaniti_yaz(&mut soket, "200 OK", &[], &yanit).await;
                return Some(());
            }
        }
    }

    if metod == "initialize" {
        let surum = match kip {
            SahteUzakKip::Legacy20250618 => "2025-06-18",
            SahteUzakKip::Legacy20250326 => "2025-03-26",
            _ => "2025-11-25",
        };
        let yanit = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": surum,
                "capabilities": {
                    "tools": { "listChanged": true }
                },
                "serverInfo": {
                    "name": "sahte-uzak-legacy",
                    "version": "1.0.0"
                }
            }
        });
        json_yaniti_yaz(
            &mut soket,
            "200 OK",
            &[("Mcp-Session-Id", "oturum-legacy-42")],
            &yanit,
        )
        .await;
        return Some(());
    }

    if metod == "subscriptions/listen" {
        let yanit = json!({
            "jsonrpc": "2.0",
            "method": "notifications/subscriptions/acknowledged",
            "params": {
                "_meta": {
                    "io.modelcontextprotocol/requestId": id
                },
                "acknowledged": {
                    "toolsListChanged": true
                }
            }
        });
        json_yaniti_yaz(&mut soket, "200 OK", &[], &yanit).await;
        return Some(());
    }

    if metod == "tools/list" {
        let yanit = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [
                    {
                        "name": "dokuman_ara",
                        "description": "Uzak belgelerde arama yapar",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "sorgu": { "type": "string" }
                            },
                            "required": ["sorgu"]
                        },
                        "outputSchema": {
                            "type": "object",
                            "properties": {
                                "bulunan": { "type": "integer" }
                            },
                            "required": ["bulunan"]
                        }
                    }
                ]
            }
        });
        json_yaniti_yaz(&mut soket, "200 OK", &[], &yanit).await;
        return Some(());
    }

    if metod == "tools/call" {
        arac_cagrisi_sayaci.fetch_add(1, Ordering::SeqCst);
        match kip {
            SahteUzakKip::YaridaKesilenSseCagri => {
                // SSE başlığını ve yarım bir data satırını yazıp soketi aniden kapatır.
                let yarim = b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {\"jsonrpc\":\"2.0\",\"id\":";
                let _ = soket.write_all(yarim).await;
                let _ = soket.flush().await;
                return Some(());
            }
            SahteUzakKip::YavasCagriIptal => {
                tokio::time::sleep(Duration::from_secs(5)).await;
                return Some(());
            }
            SahteUzakKip::DevSseCerceveAsimi => {
                let dev = "x".repeat(2048);
                let sse = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {dev}\n\n"
                );
                let _ = soket.write_all(sse.as_bytes()).await;
                let _ = soket.flush().await;
                return Some(());
            }
            SahteUzakKip::BozukJsonYaniti => {
                let bozuk = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\nConnection: close\r\n\r\n{bozuk-json";
                let _ = soket.write_all(bozuk).await;
                let _ = soket.flush().await;
                return Some(());
            }
            SahteUzakKip::Modern2026JsonVeSse => {
                // SSE üzerinden önce bildirim, ardından yapılandırılmış yanıt döner.
                let bildirim = json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/tools/list_changed"
                });
                let sonuc = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [
                            { "type": "text", "text": "{\"bulunan\":3}" }
                        ],
                        "structuredContent": {
                            "bulunan": 3
                        }
                    }
                });
                let sse_govde = format!(
                    "event: message\ndata: {}\n\nevent: message\ndata: {}\n\n",
                    serde_json::to_string(&bildirim).unwrap(),
                    serde_json::to_string(&sonuc).unwrap()
                );
                let yanit = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    sse_govde.len(),
                    sse_govde
                );
                let _ = soket.write_all(yanit.as_bytes()).await;
                let _ = soket.flush().await;
                return Some(());
            }
            _ => {
                let sonuc = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "structuredContent": {
                            "bulunan": 5
                        }
                    }
                });
                json_yaniti_yaz(&mut soket, "200 OK", &[], &sonuc).await;
                return Some(());
            }
        }
    }

    None
}

#[tokio::test]
async fn m02_kimliksiz_streamable_http_2026_ve_legacy_surumlerde_listeler_ve_cagirir() {
    for (kip, beklenen_protokol, beklenen_bulunan) in [
        (SahteUzakKip::Modern2026JsonVeSse, "2026-07-28", 3),
        (SahteUzakKip::Legacy20251125, "2025-11-25", 5),
        (SahteUzakKip::Legacy20250618, "2025-06-18", 5),
        (SahteUzakKip::Legacy20250326, "2025-03-26", 5),
    ] {
        let sunucu = SahteUzakSunucu::baslat(kip).await;
        let tanim = UzakMcpTanim::varsayilan_sinirlarla(
            "uzak-m02".into(),
            &sunucu.url(),
            true,
            UzakYetkiTuru::Yetkisiz,
        )
        .unwrap();

        let (tasima, hata) = SinirliHttpTasima::yeni(UzakHttpSecenekleri::tanimdan(&tanim));
        let mut istemci = McpIstemcisi::yeni_uzak(tasima, hata).await.unwrap();

        let oturum = istemci.uzak_oturum_durumu().unwrap().clone();
        assert_eq!(
            oturum.anlasilan_protokol().as_deref(),
            Some(beklenen_protokol)
        );

        let araclar = istemci.araclari_listele().await.unwrap();
        assert_eq!(araclar.len(), 1);
        assert_eq!(araclar[0].name, "dokuman_ara");
        assert!(araclar[0].output_schema.is_some());

        let mut argumanlar = rmcp::model::JsonObject::new();
        argumanlar.insert("sorgu".into(), Value::String("kalfa".into()));
        let cevap = istemci
            .arac_cagir_cevabi("dokuman_ara", argumanlar)
            .await
            .unwrap();

        let denetlenmis = arac_tanimlarini_denetle("uzak_m02", araclar, &[]);
        assert_eq!(denetlenmis.tanimlar().len(), 1);
        let donusturucu = denetlenmis.tanimlar()[0].sonuc_donusturucusu();
        let ciktisi = donusturucu(cevap.sonuc, cevap.content_alani_var).unwrap();
        assert!(
            ciktisi
                .icerik
                .contains(&format!("\"bulunan\":{beklenen_bulunan}"))
        );

        if kip != SahteUzakKip::Modern2026JsonVeSse {
            assert_eq!(oturum.mcp_session_id().as_deref(), Some("oturum-legacy-42"));
            assert!(
                sunucu
                    .gorulen_oturum_kimlikleri
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|s| s == "oturum-legacy-42")
            );
        }

        istemci.kapat().await;
        sunucu.kapat().await;
    }
}

#[tokio::test]
async fn m05_yarida_kesilen_ve_iptal_edilen_arac_cagrisi_belirsiz_sayilir_ve_tekrar_edilmez() {
    // 1) Yarıda kesilen SSE yanıtı: tek bir POST gönderilmeli (otomatik retry yok)
    // ve hata BelirsizCagriSonucu olmalı.
    let sunucu = SahteUzakSunucu::baslat(SahteUzakKip::YaridaKesilenSseCagri).await;
    let tanim = UzakMcpTanim::varsayilan_sinirlarla(
        "uzak-m05".into(),
        &sunucu.url(),
        true,
        UzakYetkiTuru::Yetkisiz,
    )
    .unwrap();
    let (tasima, hata) = SinirliHttpTasima::yeni(UzakHttpSecenekleri::tanimdan(&tanim));
    let mut istemci = McpIstemcisi::yeni_uzak(tasima, hata).await.unwrap();
    let oturum = istemci.uzak_oturum_durumu().unwrap().clone();

    let sonuc = istemci
        .arac_cagir_cevabi("dokuman_ara", rmcp::model::JsonObject::new())
        .await;
    let err = sonuc.unwrap_err();
    assert!(
        err.belirsiz_cagri_sonucu_mu(),
        "yarıda kesilen tools/call BelirsizCagriSonucu dönmeli, alınan: {err:?}"
    );
    assert_eq!(
        sunucu.arac_cagrisi_sayaci.load(Ordering::SeqCst),
        1,
        "yan etkili olabilecek tools/call otomatik tekrar edilmemeli"
    );
    assert_eq!(oturum.gonderilen_arac_cagrisi_sayisi(), 1);
    let belirsizler = oturum.belirsiz_cagrilar();
    assert_eq!(belirsizler.len(), 1);
    assert_eq!(belirsizler[0].neden, BelirsizCagriNedeni::AkisKesildi);

    istemci.kapat().await;
    sunucu.kapat().await;

    // 2) Yavaş araç çağrısı çağıran tarafından iptal edildiğinde sunucuya iptal
    // bildirimi gider ancak sonuç yine de belirsiz (`IptalGonderildiSonucBilinmiyor`) kaydedilir.
    let yavas_sunucu = SahteUzakSunucu::baslat(SahteUzakKip::YavasCagriIptal).await;
    let yavas_tanim = UzakMcpTanim::varsayilan_sinirlarla(
        "uzak-iptal".into(),
        &yavas_sunucu.url(),
        true,
        UzakYetkiTuru::Yetkisiz,
    )
    .unwrap();
    let (tasima, hata) = SinirliHttpTasima::yeni(UzakHttpSecenekleri::tanimdan(&yavas_tanim));
    let mut istemci = McpIstemcisi::yeni_uzak(tasima, hata).await.unwrap();
    let oturum = istemci.uzak_oturum_durumu().unwrap().clone();

    let cagri = istemci.arac_cagir_cevabi("dokuman_ara", rmcp::model::JsonObject::new());
    let zaman_asimi = tokio::time::timeout(Duration::from_millis(120), cagri).await;
    assert!(zaman_asimi.is_err());

    // İptal bildiriminin işlenmesi için kısa bekleme.
    tokio::time::sleep(Duration::from_millis(100)).await;
    let belirsizler = oturum.belirsiz_cagrilar();
    assert_eq!(belirsizler.len(), 1);
    assert!(matches!(
        belirsizler[0].neden,
        BelirsizCagriNedeni::IptalGonderildiSonucBilinmiyor | BelirsizCagriNedeni::AkisKesildi
    ));
    assert_eq!(yavas_sunucu.arac_cagrisi_sayaci.load(Ordering::SeqCst), 1);

    istemci.kapat().await;
    yavas_sunucu.kapat().await;
}

#[tokio::test]
async fn m06_ag_politikasi_dns_yonlendirme_cerceve_ve_yetki_maskeleme_sinirlari() {
    // 1) DNS rebinding / özel ağ / bulut metadata IP'leri (karma kayıt dahil) reddedilir.
    let dns = SabitDnsCozucu::yeni();
    dns.kayit_ekle(
        "rebinding.example.com",
        vec![
            IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        ],
    );
    dns.kayit_ekle(
        "metadata.example.com",
        vec![IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))],
    );
    dns.kayit_ekle(
        "ozel-ag.example.com",
        vec![IpAddr::V4(Ipv4Addr::new(10, 0, 0, 15))],
    );

    for yasak_url in [
        "https://rebinding.example.com/mcp",
        "https://metadata.example.com/mcp",
        "https://ozel-ag.example.com/mcp",
        "http://127.0.0.1:8080/mcp",
    ] {
        let sonuc = dogrulanmis_istemci_ve_url_hazirla(
            yasak_url,
            false,
            false,
            Duration::from_secs(5),
            &dns,
        )
        .await;
        assert_eq!(
            sonuc.err(),
            Some(TasimaHatasi::AgPolitikasiReddi),
            "{yasak_url} ağ politikası tarafından reddedilmeli"
        );
    }

    // 2) Yasaklı IP'ye yönlendirme ve kimlikli bağlantıda farklı origin'e yönlendirme reddedilir.
    for kip in [
        SahteUzakKip::YasakliIpYonlendirme,
        SahteUzakKip::FarkliOriginYonlendirme,
    ] {
        let sunucu = SahteUzakSunucu::baslat(kip).await;
        let tanim = UzakMcpTanim::varsayilan_sinirlarla(
            "uzak-yonlendirme".into(),
            &sunucu.url(),
            true,
            UzakYetkiTuru::Yetkisiz,
        )
        .unwrap();
        let gizli = UzakGizliBaslik::bearer("gizli-bearer-token-12345").unwrap();
        let secenekler = UzakHttpSecenekleri::tanimdan(&tanim).gizli_baslik_ile(Some(gizli));
        let (tasima, hata) = SinirliHttpTasima::yeni(secenekler);
        let sonuc = McpIstemcisi::yeni_uzak(tasima, hata).await;
        assert!(matches!(
            sonuc,
            Err(McpHatasi::Tasima(TasimaHatasi::AgPolitikasiReddi))
        ));
        sunucu.kapat().await;
    }

    // 3) 401 Yetki Gerekli yanıtı YetkiGerekli hatası üretir ve WWW-Authenticate içindeki sırrı maskeler.
    let yetki_sunucu = SahteUzakSunucu::baslat(SahteUzakKip::Yetki401).await;
    let yetki_tanim = UzakMcpTanim::varsayilan_sinirlarla(
        "uzak-401".into(),
        &yetki_sunucu.url(),
        true,
        UzakYetkiTuru::Yetkisiz,
    )
    .unwrap();
    let mut secenekler = UzakHttpSecenekleri::tanimdan(&yetki_tanim);
    secenekler
        .ek_maskeler
        .push(GizliDeger::yeni("gizli-canary-999").unwrap());
    let (tasima, hata) = SinirliHttpTasima::yeni(secenekler);
    let oturum = tasima.oturum_durumu();
    let sonuc = McpIstemcisi::yeni_uzak(tasima, hata).await;
    assert!(matches!(
        sonuc,
        Err(McpHatasi::Tasima(TasimaHatasi::YetkiGerekli))
    ));
    let red = oturum.son_yetki_reddi().unwrap();
    assert_eq!(red.durum_kodu, 401);
    assert!(
        !red.www_authenticate
            .as_deref()
            .unwrap_or_default()
            .contains("gizli-canary-999")
    );
    yetki_sunucu.kapat().await;

    // 4) Çerçeve sınırı aşımı (`Cerceve`) ve bozuk JSON (`BozukJson`) kapalı düşer.
    let dev_sunucu = SahteUzakSunucu::baslat(SahteUzakKip::DevSseCerceveAsimi).await;
    let dev_tanim = UzakMcpTanim::varsayilan_sinirlarla(
        "uzak-dev".into(),
        &dev_sunucu.url(),
        true,
        UzakYetkiTuru::Yetkisiz,
    )
    .unwrap();
    let (tasima, hata) =
        SinirliHttpTasima::yeni(UzakHttpSecenekleri::tanimdan(&dev_tanim).cerceve_siniri_ile(1024));
    let mut istemci = McpIstemcisi::yeni_uzak(tasima, hata).await.unwrap();
    let dev_sonuc = istemci
        .arac_cagir_cevabi("dokuman_ara", rmcp::model::JsonObject::new())
        .await;
    assert!(matches!(
        dev_sonuc,
        Err(McpHatasi::Tasima(TasimaHatasi::Cerceve))
    ));
    istemci.kapat().await;
    dev_sunucu.kapat().await;

    let bozuk_sunucu = SahteUzakSunucu::baslat(SahteUzakKip::BozukJsonYaniti).await;
    let bozuk_tanim = UzakMcpTanim::varsayilan_sinirlarla(
        "uzak-bozuk".into(),
        &bozuk_sunucu.url(),
        true,
        UzakYetkiTuru::Yetkisiz,
    )
    .unwrap();
    let (tasima, hata) = SinirliHttpTasima::yeni(UzakHttpSecenekleri::tanimdan(&bozuk_tanim));
    let mut istemci = McpIstemcisi::yeni_uzak(tasima, hata).await.unwrap();
    let bozuk_sonuc = istemci
        .arac_cagir_cevabi("dokuman_ara", rmcp::model::JsonObject::new())
        .await;
    assert!(matches!(
        bozuk_sonuc,
        Err(McpHatasi::Tasima(TasimaHatasi::BozukJson))
    ));
    istemci.kapat().await;
    bozuk_sunucu.kapat().await;
}
