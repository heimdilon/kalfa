mod ortak;

use std::str::FromStr;

use kalfa_evren::tipler::{ChatMessage, ChatRequest};
use kalfa_evren::{ApiAnahtari, EvrenError, EvrenIstemcisi};
use rust_decimal::Decimal;
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ANAHTAR: &str = "evren_llm_test_anahtari";

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
    assert!(matches!(i.sart_durumu().await, Err(EvrenError::Ag(_))));
}

#[test]
fn anahtar_hicbir_ciktida_gorunmez() {
    let a = ApiAnahtari::yeni(ANAHTAR);
    assert_eq!(format!("{a:?}"), "ApiAnahtari(***)");
    let i = EvrenIstemcisi::yeni("https://ornek/v1/", a).unwrap();
    assert!(!format!("{i:?}").contains(ANAHTAR));
}
