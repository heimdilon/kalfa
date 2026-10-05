mod ortak;

use std::str::FromStr;

use futures_util::StreamExt;
use kalfa_evren::tipler::{ChatMessage, ChatRequest};
use kalfa_evren::{AkisOlayi, ApiAnahtari, EvrenError, EvrenIstemcisi};
use rust_decimal::Decimal;
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ANAHTAR: &str = "evren_llm_test_anahtari";

#[tokio::test]
async fn json_ve_hata_govdesi_eklenmeden_once_sinirlanir() {
    for durum in [200, 500] {
        let (s, i) = sunucu_ve_istemci().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(json_yanit(
                durum,
                vec![b'a'; kalfa_evren::butce::JSON_BAYT_SINIRI + 1],
            ))
            .mount(&s)
            .await;
        assert!(matches!(i.modeller().await, Err(EvrenError::YanitSiniri)));
    }
}

#[tokio::test]
async fn bloklu_yanitta_cok_sayida_cagri_kabul_edilmez() {
    let (s, i) = sunucu_ve_istemci().await;
    let cagrilar: Vec<_> = (0..129).map(|n| json!({"id":format!("c{n}"),"type":"function","function":{"name":"x","arguments":"{}"}})).collect();
    Mock::given(method("POST")).and(path("/v1/chat/completions"))
        .respond_with(json_yanit(200, json!({"id":"r","model":"x","choices":[{"index":0,"message":{"role":"assistant","tool_calls":cagrilar}}]}).to_string().into_bytes()))
        .mount(&s).await;
    assert!(matches!(
        i.sohbet(&ChatRequest::new("x", vec![])).await,
        Err(EvrenError::YanitSiniri)
    ));
}

#[tokio::test]
async fn yalniz_kararli_acik_tutan_sse_de_toplam_bayt_sinirina_tabidir() {
    let (s, i) = sunucu_ve_istemci().await;
    // Tek olay sınırına takılmayan yorumlar da yanıtın ömrüne sayılır.
    let veri = ": canli\n".repeat(kalfa_evren::butce::YANIT_BAYT_SINIRI / 8 + 1);
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(veri, "text/event-stream"))
        .mount(&s)
        .await;
    let (_, mut akis) = i
        .sohbet_akisi(&ChatRequest::new("x", vec![]))
        .await
        .unwrap();
    assert!(matches!(
        akis.next().await,
        Some(Err(EvrenError::YanitSiniri))
    ));
    assert!(akis.next().await.is_none());
}

async fn sunucu_ve_istemci() -> (MockServer, EvrenIstemcisi) {
    let sunucu = MockServer::start().await;
    let istemci =
        EvrenIstemcisi::yeni(format!("{}/v1", sunucu.uri()), ApiAnahtari::yeni(ANAHTAR)).unwrap();
    (sunucu, istemci)
}

fn json_yanit(durum: u16, govde: Vec<u8>) -> ResponseTemplate {
    ResponseTemplate::new(durum).set_body_raw(govde, "application/json")
}

#[tokio::test]
async fn sart_durumu_bearer_ile_ister() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("GET"))
        .and(path("/v1/terms/status"))
        .and(header(
            "authorization",
            format!("Bearer {ANAHTAR}").as_str(),
        ))
        .respond_with(json_yanit(200, ortak::fixture_bayt("terms_status.json")))
        .expect(1)
        .mount(&s)
        .await;
    let d = i.sart_durumu().await.unwrap();
    assert!(d.accepted);
    assert_eq!(d.current_version, 1);
}

#[tokio::test]
async fn sart_metni_ve_kabul() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("GET"))
        .and(path("/v1/terms/text"))
        .respond_with(json_yanit(200, ortak::fixture_bayt("terms_text.json")))
        .mount(&s)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/terms/accept"))
        .and(body_json(json!({"version": 1})))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw("{\"accepted\":true}", "application/json"),
        )
        .expect(1)
        .mount(&s)
        .await;
    assert_eq!(i.sart_metni().await.unwrap().version, 1);
    i.sart_kabul(1).await.unwrap();
}

#[tokio::test]
async fn modeller() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(json_yanit(200, ortak::fixture_bayt("models.json")))
        .mount(&s)
        .await;
    let m = i.modeller().await.unwrap();
    assert_eq!(m.len(), 11);
    assert!(
        m.iter()
            .any(|x| x.id == "qwen3.8-flash-next" && x.task == "chat")
    );
}

#[tokio::test]
async fn bloklu_sohbet_yanit_ve_meta() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            json_yanit(200, ortak::fixture_bayt("chat_tool_calls_blocking.json"))
                .insert_header("x-evren-credits-remaining", "1000.0000")
                .insert_header("x-evren-credits-held", "0.0000")
                .insert_header("x-request-id", "istek-1")
                .insert_header("x-evren-routed-model", "zai/glm-5.3-fp8"),
        )
        .mount(&s)
        .await;
    let mut istek = ChatRequest::new("glm-5.3", vec![ChatMessage::user("hava?")]);
    istek.stream = true; // sohbet() bunu false'a çekmeli
    let (y, meta) = i.sohbet(&istek).await.unwrap();
    assert_eq!(y.choices[0].message.tool_calls.as_ref().unwrap().len(), 2);
    assert_eq!(meta.istek_id.as_deref(), Some("istek-1"));
    assert_eq!(meta.yonlendirilen_model.as_deref(), Some("zai/glm-5.3-fp8"));
    assert_eq!(
        meta.kredi.unwrap().kalan,
        Some(Decimal::from_str("1000").unwrap())
    );

    let gonderilen: serde_json::Value =
        serde_json::from_slice(&s.received_requests().await.unwrap()[0].body).unwrap();
    assert_eq!(gonderilen["stream"], false);
    assert!(gonderilen.get("stream_options").is_none());
}

#[tokio::test]
async fn hata_durumlari_tipli_doner() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("GET"))
        .and(path("/v1/terms/status"))
        .respond_with(json_yanit(
            401,
            ortak::fixture_bayt("error_unauthorized.json"),
        ))
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(json_yanit(
            403,
            ortak::sentetik_bayt("terms_not_accepted.json"),
        ))
        .mount(&s)
        .await;
    assert_eq!(
        i.sart_durumu().await.unwrap_err(),
        EvrenError::AnahtarGecersiz
    );
    assert_eq!(
        i.modeller().await.unwrap_err(),
        EvrenError::SartKabulGerekli { surum: Some(1) }
    );
}

#[tokio::test]
async fn ulasilamayan_sunucu_ag_hatasi() {
    let i = EvrenIstemcisi::yeni("http://127.0.0.1:1/v1", ApiAnahtari::yeni(ANAHTAR)).unwrap();
    assert!(matches!(
        i.sart_durumu().await,
        Err(EvrenError::Ag {
            govde_gitti: false,
            ..
        })
    ));
}

#[test]
fn anahtar_hicbir_ciktida_gorunmez() {
    let a = ApiAnahtari::yeni(ANAHTAR);
    assert_eq!(format!("{a:?}"), "ApiAnahtari(***)");
    let i = EvrenIstemcisi::yeni("https://ornek/v1/", a).unwrap();
    assert!(!format!("{i:?}").contains(ANAHTAR));
}

fn akis_yaniti(govde: Vec<u8>) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_raw(govde, "text/event-stream")
        .insert_header("x-request-id", "akis-1")
        .insert_header("x-evren-credits-remaining", "1000.0000")
}

#[tokio::test]
async fn akisli_sohbet_fixture_uzerinden() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(akis_yaniti(ortak::fixture_bayt(
            "chat_tool_calls_stream.sse",
        )))
        .mount(&s)
        .await;
    let istek = ChatRequest::new("glm-5.3", vec![ChatMessage::user("İstanbul'da hava?")]);
    let (meta, akis) = i.sohbet_akisi(&istek).await.unwrap();
    assert_eq!(meta.istek_id.as_deref(), Some("akis-1"));

    let olaylar: Vec<_> = akis.collect().await;
    assert!(olaylar.iter().all(|o| o.is_ok()), "{olaylar:?}");
    let olaylar: Vec<AkisOlayi> = olaylar.into_iter().map(Result::unwrap).collect();
    assert!(olaylar.iter().any(|o| matches!(o, AkisOlayi::AracCagrilari(v) if v[0].function.arguments == "{\"sehir\": \"Istanbul\"}")));
    assert_eq!(
        olaylar.last(),
        Some(&AkisOlayi::Bitti {
            neden: "tool_calls".into()
        })
    );

    let gonderilen: serde_json::Value =
        serde_json::from_slice(&s.received_requests().await.unwrap()[0].body).unwrap();
    assert_eq!(gonderilen["stream"], true);
    assert_eq!(gonderilen["stream_options"]["include_usage"], true);
}

#[tokio::test]
async fn yarim_kalan_akis_kanalda_hata_olarak_biter() {
    let (s, i) = sunucu_ve_istemci().await;
    let metin = ortak::fixture_metin("chat_tool_calls_stream.sse");
    let bitis = metin.find("\"finish_reason\": \"tool_calls\"").unwrap();
    let kes = metin[..bitis].rfind("data:").unwrap();
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(akis_yaniti(metin.as_bytes()[..kes].to_vec()))
        .mount(&s)
        .await;
    let (_, akis) = i
        .sohbet_akisi(&ChatRequest::new("glm-5.3", vec![]))
        .await
        .unwrap();
    let olaylar: Vec<_> = akis.collect().await;
    assert_eq!(olaylar.last(), Some(&Err(EvrenError::AkisYarimKaldi)));
    assert!(
        !olaylar
            .iter()
            .any(|o| matches!(o, Ok(AkisOlayi::AracCagrilari(_))))
    );
}

#[tokio::test]
async fn akis_oncesi_http_hatasi_err_doner() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(json_yanit(
            401,
            ortak::fixture_bayt("error_unauthorized.json"),
        ))
        .mount(&s)
        .await;
    let sonuc = i.sohbet_akisi(&ChatRequest::new("glm-5.3", vec![])).await;
    assert_eq!(sonuc.err(), Some(EvrenError::AnahtarGecersiz));
}

#[tokio::test]
async fn akis_birakilinca_sessiz_sunucuda_da_baglanti_kapanir() {
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let dinleyici = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adres = dinleyici.local_addr().unwrap();
    let sunucu = tokio::spawn(async move {
        let (mut soket, _) = dinleyici.accept().await.unwrap();
        // TCP read bir HTTP isteğine denk gelmez. Başlık ve gövdeyi tamamen
        // tüket; aksi halde kalan istek baytları kapanma testini yanıltır.
        let mut istek = Vec::new();
        let mut parca = [0u8; 1024];
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let n = soket.read(&mut parca).await.unwrap();
                assert!(n > 0, "istek tamamlanmadan EOF");
                istek.extend_from_slice(&parca[..n]);
                assert!(
                    istek.len() <= 64 * 1024,
                    "test isteği beklenmedik kadar büyük"
                );
                if let Some(son) = istek.windows(4).position(|w| w == b"\r\n\r\n") {
                    let baslik = std::str::from_utf8(&istek[..son]).unwrap();
                    let boyut: usize = baslik
                        .lines()
                        .filter_map(|l| l.split_once(':'))
                        .find(|(ad, _)| ad.eq_ignore_ascii_case("content-length"))
                        .expect("test istemcisi Content-Length göndermeli")
                        .1
                        .trim()
                        .parse()
                        .unwrap();
                    if istek.len() >= son + 4 + boyut {
                        break;
                    }
                }
            }
        })
        .await
        .expect("test HTTP isteği tamamlanmalı");
        soket
            .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\n")
            .await
            .unwrap();
        // Tek parça: ": keep-alive\n\n" (14 = 0xe bayt). Sonra sessizlik.
        soket.write_all(b"e\r\n: keep-alive\n\n\r\n").await.unwrap();
        let mut tampon = [0u8; 64];
        tokio::time::timeout(Duration::from_secs(5), soket.read(&mut tampon)).await
    });

    let i = EvrenIstemcisi::yeni(format!("http://{adres}/v1"), ApiAnahtari::yeni(ANAHTAR)).unwrap();
    let (_, akis) = i
        .sohbet_akisi(&ChatRequest::new("glm-5.3", vec![]))
        .await
        .unwrap();
    drop(akis);

    let okuma = sunucu
        .await
        .unwrap()
        .expect("istemci akışı bırakınca bağlantı 5 sn içinde kapanmalı");
    assert!(matches!(okuma, Ok(0) | Err(_)), "{okuma:?}");
}
