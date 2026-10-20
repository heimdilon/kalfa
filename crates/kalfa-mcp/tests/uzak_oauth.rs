//! Görev 4 (`M03`): Uzak MCP OAuth 2.0 (PKCE S256, `state`/oturum bağlama,
//! `issuer` ve `resource` doğrulaması, kimlikli Streamable HTTP çağrısı ve
//! sentetik canary sır maskelemesi) entegrasyon testleri.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kalfa_mcp::istemci::{McpHatasi, McpIstemcisi};
use kalfa_mcp::kesif::arac_tanimlarini_denetle;
use kalfa_mcp::maske::GizliDeger;
use kalfa_mcp::tanim::{UzakMcpTanim, UzakYetkiTuru};
use kalfa_mcp::tasima::TasimaHatasi;
use kalfa_mcp::uzak::{
    OAuthDurumBaglami, OAuthGeriCagrimGirdisi, OAuthHatasi, SabitDnsCozucu, SinirliHttpTasima,
    UzakDnsCozucu, UzakGizliBaslik, UzakHttpSecenekleri, geri_cagrim_dogrula, kod_ile_token_al,
    korunmus_kaynak_metadatasini_getir, yenileme_tokeni_ile_token_al,
    yetki_sunucusu_metadatasini_getir, yetkilendirme_urlsi_olustur,
};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const CANARY_ACCESS: &str = "CANARY-M03-ACCESS-TOKEN-88776655";
const CANARY_REFRESH: &str = "CANARY-M03-REFRESH-TOKEN-44332211";
const CANARY_REFRESHED: &str = "CANARY-M03-REFRESHED-TOKEN-99001122";

struct SahteM03Sunucusu {
    adres: SocketAddr,
    beklenen_challenge: Arc<Mutex<Option<String>>>,
    kapat_tx: Option<tokio::sync::watch::Sender<bool>>,
    gorev: tokio::task::JoinHandle<()>,
}

impl SahteM03Sunucusu {
    async fn baslat() -> Self {
        let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let adres = dinleyici.local_addr().unwrap();
        let beklenen_challenge = Arc::new(Mutex::new(None));
        let kabul_edilen_token = Arc::new(Mutex::new(CANARY_ACCESS.to_owned()));
        let (kapat_tx, mut kapat_rx) = tokio::sync::watch::channel(false);

        let challenge_kopyasi = beklenen_challenge.clone();
        let token_kopyasi = kabul_edilen_token.clone();

        let gorev = tokio::spawn(async move {
            loop {
                let kabul = tokio::select! {
                    biased;
                    _ = kapat_rx.changed() => break,
                    sonuc = dinleyici.accept() => sonuc,
                };
                let Ok((soket, _)) = kabul else { break };
                let ch = challenge_kopyasi.clone();
                let tk = token_kopyasi.clone();
                tokio::spawn(async move {
                    let _ = m03_baglantiyi_isle(soket, adres, ch, tk).await;
                });
            }
        });

        Self {
            adres,
            beklenen_challenge,
            kapat_tx: Some(kapat_tx),
            gorev,
        }
    }

    fn mcp_url(&self) -> String {
        format!("http://{}/mcp", self.adres)
    }

    fn resource_meta_url(&self) -> String {
        format!("http://{}/.well-known/oauth-protected-resource", self.adres)
    }

    fn auth_meta_url(&self) -> String {
        format!(
            "http://{}/.well-known/oauth-authorization-server",
            self.adres
        )
    }

    fn issuer_url(&self) -> String {
        format!("http://{}/issuer", self.adres)
    }

    async fn kapat(mut self) {
        if let Some(tx) = self.kapat_tx.take() {
            let _ = tx.send(true);
        }
        let _ = tokio::time::timeout(Duration::from_secs(1), self.gorev).await;
    }
}

async fn http_oku(soket: &mut TcpStream) -> Option<(String, HashMap<String, String>, Vec<u8>)> {
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
    let ilk_satir = baslik_metni.lines().next().unwrap_or_default().to_owned();
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
    Some((ilk_satir, basliklar, govde))
}

async fn json_yaz(soket: &mut TcpStream, durum: &str, govde: &Value) {
    let baytlar = serde_json::to_vec(govde).unwrap();
    let baslik = format!(
        "HTTP/1.1 {durum}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        baytlar.len()
    );
    let _ = soket.write_all(baslik.as_bytes()).await;
    let _ = soket.write_all(&baytlar).await;
    let _ = soket.flush().await;
}

async fn m03_baglantiyi_isle(
    mut soket: TcpStream,
    adres: SocketAddr,
    beklenen_challenge: Arc<Mutex<Option<String>>>,
    kabul_edilen_token: Arc<Mutex<String>>,
) -> Option<()> {
    let (ilk_satir, basliklar, govde) = http_oku(&mut soket).await?;
    let resource_url = format!("http://{adres}/mcp");
    let issuer_url = format!("http://{adres}/issuer");

    if ilk_satir.contains("/.well-known/oauth-protected-resource") {
        json_yaz(
            &mut soket,
            "200 OK",
            &json!({
                "resource": resource_url,
                "authorization_servers": [issuer_url],
                "scopes_supported": ["tools:read", "tools:call"]
            }),
        )
        .await;
        return Some(());
    }

    if ilk_satir.contains("/.well-known/oauth-authorization-server") {
        json_yaz(
            &mut soket,
            "200 OK",
            &json!({
                "issuer": issuer_url,
                "authorization_endpoint": format!("http://{adres}/authorize"),
                "token_endpoint": format!("http://{adres}/token"),
                "code_challenge_methods_supported": ["S256"]
            }),
        )
        .await;
        return Some(());
    }

    if ilk_satir.starts_with("POST /token") {
        let form_metni = String::from_utf8_lossy(&govde);
        let form: HashMap<String, String> = url::form_urlencoded::parse(form_metni.as_bytes())
            .into_owned()
            .collect();
        let grant_type = form
            .get("grant_type")
            .map(String::as_str)
            .unwrap_or_default();
        if grant_type == "authorization_code" {
            let verifier = form.get("code_verifier").cloned().unwrap_or_default();
            let pkce = kalfa_mcp::uzak::PkceCifti::dogrulayicidan(verifier).ok()?;
            let beklenen = beklenen_challenge.lock().unwrap().clone();
            if beklenen.as_deref() != Some(pkce.code_challenge()) {
                json_yaz(
                    &mut soket,
                    "400 Bad Request",
                    &json!({"error": "invalid_grant"}),
                )
                .await;
                return Some(());
            }
            json_yaz(
                &mut soket,
                "200 OK",
                &json!({
                    "access_token": CANARY_ACCESS,
                    "refresh_token": CANARY_REFRESH,
                    "token_type": "Bearer",
                    "expires_in": 3600,
                    "scope": "tools:read tools:call",
                    "resource": resource_url
                }),
            )
            .await;
            return Some(());
        } else if grant_type == "refresh_token" {
            if form.get("refresh_token").map(String::as_str) != Some(CANARY_REFRESH) {
                json_yaz(
                    &mut soket,
                    "400 Bad Request",
                    &json!({"error": "invalid_grant"}),
                )
                .await;
                return Some(());
            }
            *kabul_edilen_token.lock().unwrap() = CANARY_REFRESHED.to_owned();
            json_yaz(
                &mut soket,
                "200 OK",
                &json!({
                    "access_token": CANARY_REFRESHED,
                    "refresh_token": CANARY_REFRESH,
                    "token_type": "Bearer",
                    "expires_in": 3600,
                    "scope": "tools:read tools:call",
                    "resource": resource_url
                }),
            )
            .await;
            return Some(());
        }
    }

    if ilk_satir.starts_with("POST /mcp") {
        let beklenen_auth = format!("Bearer {}", kabul_edilen_token.lock().unwrap());
        if basliklar.get("authorization").map(String::as_str) != Some(beklenen_auth.as_str()) {
            let yanit = format!(
                "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Bearer resource_metadata=\"http://{adres}/.well-known/oauth-protected-resource\", scope=\"tools:read tools:call\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            let _ = soket.write_all(yanit.as_bytes()).await;
            return Some(());
        }

        let istek: Value = serde_json::from_slice(&govde).ok()?;
        let metod = istek
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let id = istek.get("id").cloned().unwrap_or(Value::Null);

        match metod {
            "server/discover" => {
                let mut yetenekler = rmcp::model::ServerCapabilities::default();
                yetenekler.tools = Some(Default::default());
                let sonuc = rmcp::model::DiscoverResult::new(
                    vec![rmcp::model::ProtocolVersion::V_2026_07_28],
                    yetenekler,
                )
                .with_server_info(rmcp::model::Implementation::new("sahte-m03", "1.0.0"));
                json_yaz(
                    &mut soket,
                    "200 OK",
                    &json!({"jsonrpc": "2.0", "id": id, "result": sonuc}),
                )
                .await;
            }
            "subscriptions/listen" => {
                let _ = soket
                    .write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    )
                    .await;
            }
            "tools/list" => {
                json_yaz(
                    &mut soket,
                    "200 OK",
                    &json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "tools": [{
                                "name": "hesap_ozeti",
                                "description": "Kimlikli hesap özetini getirir",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {}
                                }
                            }]
                        }
                    }),
                )
                .await;
            }
            "tools/call" => {
                // Yanıt içinde yanlışlıkla sızabilecek canary token varsa maskeleme bunu yakalamalıdır.
                json_yaz(
                    &mut soket,
                    "200 OK",
                    &json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{
                                "type": "text",
                                "text": format!("hesap=aktif, sızıntı_denemesi={CANARY_ACCESS} ve {CANARY_REFRESH}")
                            }]
                        }
                    }),
                )
                .await;
            }
            _ => {}
        }
    }

    Some(())
}

#[tokio::test]
async fn m03_kimliksiz_401_sonrasi_oauth_pkce_akisi_kimlikli_cagri_ve_canary_maskelemesi() {
    let sunucu = SahteM03Sunucusu::baslat().await;
    let tanim = UzakMcpTanim::varsayilan_sinirlarla(
        "m03-sunucu".into(),
        &sunucu.mcp_url(),
        true,
        UzakYetkiTuru::OAuth {
            istemci_kimligi: Some("kalfa-m03".into()),
            kapsamlar: vec!["tools:read".into(), "tools:call".into()],
            kaynak_url: Some(sunucu.issuer_url()),
            yetki_kimligi: None,
        },
    )
    .unwrap();

    // 1. Kimliksiz bağlantı 401 YetkiGerekli ile reddedilir.
    let (tasima_401, hata_401) = SinirliHttpTasima::yeni(UzakHttpSecenekleri::tanimdan(&tanim));
    let sonuc_401 = McpIstemcisi::yeni_uzak(tasima_401, hata_401).await;
    assert!(matches!(
        sonuc_401,
        Err(McpHatasi::Tasima(TasimaHatasi::YetkiGerekli))
    ));

    // 2. OAuth metadata keşfi ve PKCE S256 yetkilendirme akışı.
    let dns: Arc<dyn UzakDnsCozucu> = Arc::new(SabitDnsCozucu::yeni());
    let kaynak_meta = korunmus_kaynak_metadatasini_getir(
        &sunucu.resource_meta_url(),
        tanim.kanonik_uc_url(),
        true,
        false,
        Duration::from_secs(5),
        &dns,
    )
    .await
    .unwrap();
    let yetki_meta = yetki_sunucusu_metadatasini_getir(
        &sunucu.auth_meta_url(),
        &sunucu.issuer_url(),
        true,
        false,
        Duration::from_secs(5),
        &dns,
    )
    .await
    .unwrap();

    let mut akis = OAuthDurumBaglami::yeni(
        "m03-sunucu",
        Some("oturum-m03".into()),
        yetki_meta.issuer.clone(),
        kaynak_meta.resource.clone(),
        "http://127.0.0.1:49152/callback",
        vec!["tools:read".into(), "tools:call".into()],
        1_000,
    );
    *sunucu.beklenen_challenge.lock().unwrap() = Some(akis.pkce.code_challenge().to_owned());

    let yetki_url = yetkilendirme_urlsi_olustur(&yetki_meta, "kalfa-m03", &akis).unwrap();
    assert!(yetki_url.as_str().contains("code_challenge_method=S256"));

    // Yanlış issuer callback'i reddedilir.
    let mut yanlis_iss_akis = akis.clone();
    assert_eq!(
        geri_cagrim_dogrula(
            &mut yanlis_iss_akis,
            &OAuthGeriCagrimGirdisi {
                gelen_state: &akis.state,
                gelen_oturum: Some("oturum-m03"),
                gelen_issuer: Some("http://127.0.0.1:19999/sahte-issuer"),
                gelen_kod: Some("gecerli-kod"),
                gelen_hata: None,
                simdi_sn: 1_005,
                azami_yas_sn: 300,
                test_fiksturu_loopback: true,
            }
        ),
        Err(OAuthHatasi::YanlisIssuer)
    );

    let state = akis.state.clone();
    let kod = geri_cagrim_dogrula(
        &mut akis,
        &OAuthGeriCagrimGirdisi {
            gelen_state: &state,
            gelen_oturum: Some("oturum-m03"),
            gelen_issuer: Some(&sunucu.issuer_url()),
            gelen_kod: Some("gecerli-kod"),
            gelen_hata: None,
            simdi_sn: 1_005,
            azami_yas_sn: 300,
            test_fiksturu_loopback: true,
        },
    )
    .unwrap();

    let paket = kod_ile_token_al(
        &yetki_meta.token_endpoint,
        "kalfa-m03",
        &kod,
        &akis,
        true,
        false,
        Duration::from_secs(5),
        &dns,
    )
    .await
    .unwrap();
    assert_eq!(paket.access_token(), CANARY_ACCESS);
    assert_eq!(paket.refresh_token(), Some(CANARY_REFRESH));

    // 3. Kimlikli Streamable HTTP bağlantısı ve canary sır maskelemesi.
    let sirlar = vec![
        GizliDeger::yeni(CANARY_ACCESS).unwrap(),
        GizliDeger::yeni(CANARY_REFRESH).unwrap(),
    ];
    let gizli_baslik = UzakGizliBaslik::bearer(paket.access_token()).unwrap();
    let secenekler = UzakHttpSecenekleri::tanimdan(&tanim)
        .gizli_baslik_ile(Some(gizli_baslik))
        .ek_maskeler_ile(sirlar.clone());
    let (tasima, hata) = SinirliHttpTasima::yeni(secenekler);
    let mut istemci = McpIstemcisi::yeni_uzak(tasima, hata).await.unwrap();

    let araclar = istemci.araclari_listele().await.unwrap();
    let denetlenmis = arac_tanimlarini_denetle("m03_sunucu", araclar, &sirlar);
    assert_eq!(denetlenmis.tanimlar().len(), 1);

    let cevap = istemci
        .arac_cagir_cevabi("hesap_ozeti", rmcp::model::JsonObject::new())
        .await
        .unwrap();
    let donusturucu = denetlenmis.tanimlar()[0].sonuc_donusturucusu();
    let ciktisi = donusturucu(cevap.sonuc, cevap.content_alani_var).unwrap();
    assert!(ciktisi.icerik.contains("hesap=aktif"));
    assert!(
        !ciktisi.icerik.contains(CANARY_ACCESS),
        "araç çıktısında OAuth access token maskelenmelidir"
    );
    assert!(
        !ciktisi.icerik.contains(CANARY_REFRESH),
        "araç çıktısında OAuth refresh token maskelenmelidir"
    );
    istemci.kapat().await;

    // 4. Token yenileme sonrası yeni access_token ile bağlantı.
    let yenilenmis = yenileme_tokeni_ile_token_al(
        &yetki_meta.token_endpoint,
        "kalfa-m03",
        CANARY_REFRESH,
        tanim.kanonik_uc_url(),
        &["tools:read".into(), "tools:call".into()],
        true,
        false,
        Duration::from_secs(5),
        &dns,
    )
    .await
    .unwrap();
    assert_eq!(yenilenmis.access_token(), CANARY_REFRESHED);

    sunucu.kapat().await;
}
