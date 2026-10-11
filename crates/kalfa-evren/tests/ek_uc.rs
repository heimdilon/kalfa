//! `EvrenIstemcisi::ek_uc` sahte sunucuyla (aşama 4 tasarımı §9.1, §9.2).
//! Gövde olduğu gibi gider, yanıt ham metin olarak döner; çözülen alanlar
//! yalnızca `model` ve kredi/istek kimliği başlıklarıdır.

mod ortak;

use std::str::FromStr;
use std::time::Duration;

use kalfa_evren::butce::JSON_BAYT_SINIRI;
use kalfa_evren::{ApiAnahtari, EkUc, EvrenError, EvrenIstemcisi};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ANAHTAR: &str = "evren_llm_test_anahtari";

#[tokio::test]
async fn gomme_istegi_yola_ve_anahtara_gider_yanit_ham_doner() {
    let (s, i) = sunucu_ve_istemci().await;
    let kayit = ortak::kayit_bayt("embeddings_qwen3_1024.json");
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .and(header(
            "authorization",
            format!("Bearer {ANAHTAR}").as_str(),
        ))
        .respond_with(
            json_yanit(200, kayit.clone())
                .insert_header("x-request-id", "gomme-1")
                .insert_header("x-evren-routed-model", "qwen/qwen3-embedding-8b")
                .insert_header("x-evren-credits-remaining", "1000.0000")
                .insert_header("x-evren-credits-held", "0.0000"),
        )
        .expect(1)
        .mount(&s)
        .await;

    let govde = gomme_govdesi();
    let yanit = i.ek_uc(EkUc::Gomme, &govde).await.unwrap();

    let gonderilen: Value =
        serde_json::from_slice(&s.received_requests().await.unwrap()[0].body).unwrap();
    assert_eq!(gonderilen, govde);
    assert_eq!(yanit.govde.as_bytes(), kayit.as_slice());
    assert_eq!(yanit.model.as_deref(), Some("qwen/qwen3-embedding-8b"));
    assert_eq!(yanit.meta.istek_id.as_deref(), Some("gomme-1"));
    assert_eq!(
        yanit.meta.yonlendirilen_model.as_deref(),
        Some("qwen/qwen3-embedding-8b")
    );
    let kredi = yanit
        .meta
        .kredi
        .expect("kredi başlıklarından gözlem üretilmeli");
    assert_eq!(kredi.kalan, Some(Decimal::from_str("1000").unwrap()));
    assert_eq!(kredi.tutulan, Some(Decimal::ZERO));
}

#[tokio::test]
async fn base64_gomme_yaniti_metin_olarak_kalir() {
    let (s, i) = sunucu_ve_istemci().await;
    let kayit = ortak::kayit_bayt("embeddings_qwen3_base64.json");
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(json_yanit(200, kayit.clone()))
        .expect(1)
        .mount(&s)
        .await;

    let govde = json!({
        "model": "qwen3-embedding-8b",
        "input": ["merhaba dünya"],
        "encoding_format": "base64"
    });
    let yanit = i.ek_uc(EkUc::Gomme, &govde).await.unwrap();

    assert_eq!(yanit.govde.as_bytes(), kayit.as_slice());
    assert_eq!(yanit.model.as_deref(), Some("qwen/qwen3-embedding-8b"));
    let cozulmus: Value = serde_json::from_str(&yanit.govde).unwrap();
    assert!(
        cozulmus["data"][0]["embedding"].is_string(),
        "base64 vektörü metin olarak kalmalı"
    );
}

#[tokio::test]
async fn siralama_rerank_yoluna_gider() {
    let (s, i) = sunucu_ve_istemci().await;
    let kayit = ortak::kayit_bayt("rerank_qwen3.json");
    Mock::given(method("POST"))
        .and(path("/v1/rerank"))
        .respond_with(json_yanit(200, kayit.clone()))
        .expect(1)
        .mount(&s)
        .await;

    let govde = json!({
        "model": "qwen3-reranker-8b",
        "query": "kurtarma",
        "documents": ["Kurtarma kaydı okur.", "Kabuk yalıtımda çalışır."],
        "top_n": 2
    });
    let yanit = i.ek_uc(EkUc::Siralama, &govde).await.unwrap();

    let gonderilen: Value =
        serde_json::from_slice(&s.received_requests().await.unwrap()[0].body).unwrap();
    assert_eq!(gonderilen, govde);
    assert_eq!(yanit.govde.as_bytes(), kayit.as_slice());
    assert_eq!(yanit.model.as_deref(), Some("qwen/qwen3-vl-reranker-8b"));
}

#[tokio::test]
async fn baglam_siniri_asilinca_bad_request_sunucu_hatasi() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(json_yanit(
            400,
            ortak::kayit_bayt("error_embedding_context_length.json"),
        ))
        .mount(&s)
        .await;

    match i.ek_uc(EkUc::Gomme, &gomme_govdesi()).await {
        Err(EvrenError::Sunucu { durum, kod, mesaj }) => {
            assert_eq!(durum, 400);
            assert_eq!(kod.as_deref(), Some("bad_request"));
            assert!(mesaj.contains("maximum context length"), "{mesaj}");
        }
        diger => panic!("beklenmeyen sonuç: {diger:?}"),
    }
}

#[tokio::test]
async fn hiz_siniri_retry_after_ile_tekrar_dene_suresi_verir() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "7"))
        .mount(&s)
        .await;

    assert_eq!(
        i.ek_uc(EkUc::Gomme, &gomme_govdesi()).await.unwrap_err(),
        EvrenError::HizSiniri {
            tekrar_dene: Some(Duration::from_secs(7))
        }
    );
}

#[tokio::test]
async fn sart_kabul_edilmemisse_sart_kabul_gerekli_doner() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(json_yanit(
            403,
            ortak::sentetik_bayt("terms_not_accepted.json"),
        ))
        .mount(&s)
        .await;

    assert_eq!(
        i.ek_uc(EkUc::Gomme, &gomme_govdesi()).await.unwrap_err(),
        EvrenError::SartKabulGerekli { surum: Some(1) }
    );
}

#[tokio::test]
async fn gecersiz_anahtar_anahtar_gecersiz_doner() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(json_yanit(
            401,
            ortak::fixture_bayt("error_unauthorized.json"),
        ))
        .mount(&s)
        .await;

    assert_eq!(
        i.ek_uc(EkUc::Gomme, &gomme_govdesi()).await.unwrap_err(),
        EvrenError::AnahtarGecersiz
    );
}

#[tokio::test]
async fn json_olmayan_basarili_yanit_gecersiz_yanittir() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(json_yanit(200, b"<html>bakimda</html>".to_vec()))
        .mount(&s)
        .await;

    let hata = i.ek_uc(EkUc::Gomme, &gomme_govdesi()).await.unwrap_err();

    assert!(matches!(hata, EvrenError::GecersizYanit(_)), "{hata:?}");
}

#[tokio::test]
async fn model_alani_yoksa_model_none_olur() {
    let (s, i) = sunucu_ve_istemci().await;
    let govde: &[u8] = br#"{"object":"list","data":[]}"#;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(json_yanit(200, govde.to_vec()))
        .mount(&s)
        .await;

    let yanit = i.ek_uc(EkUc::Gomme, &gomme_govdesi()).await.unwrap();

    assert_eq!(yanit.model, None);
    assert_eq!(yanit.govde.as_bytes(), govde);
}

#[tokio::test]
async fn json_bayt_sinirini_asan_yanit_yanit_siniridir() {
    let (s, i) = sunucu_ve_istemci().await;
    // Gövde geçerli JSON: hata yalnızca boyuttan gelmeli, ayrıştırmadan değil.
    let dolgu = "a".repeat(JSON_BAYT_SINIRI);
    let govde = format!(r#"{{"model":"qwen/qwen3-embedding-8b","data":"{dolgu}"}}"#);
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(json_yanit(200, govde.into_bytes()))
        .mount(&s)
        .await;

    assert_eq!(
        i.ek_uc(EkUc::Gomme, &gomme_govdesi()).await.unwrap_err(),
        EvrenError::YanitSiniri
    );
}

#[tokio::test]
async fn yonlendirme_izlenmez() {
    // Belge §9.2: yönlendirme izlenmez. Hedef uca hiç istek almamalı.
    let (s, i) = sunucu_ve_istemci().await;
    let hedef = format!("{}/v1/hedef", s.uri());
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(307).insert_header("location", hedef.as_str()))
        .expect(1)
        .mount(&s)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/hedef"))
        .respond_with(json_yanit(200, b"{}".to_vec()))
        .expect(0)
        .mount(&s)
        .await;

    let hata = i.ek_uc(EkUc::Gomme, &gomme_govdesi()).await.unwrap_err();

    assert!(
        matches!(hata, EvrenError::Sunucu { durum: 307, .. }),
        "{hata:?}"
    );
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

fn gomme_govdesi() -> Value {
    json!({"model": "qwen3-embedding-8b", "input": ["merhaba dünya"], "dimensions": 1024})
}
