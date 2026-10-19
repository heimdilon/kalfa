//! `EvrenIstemcisi::karar` (`/v1/systemone`) wiremock testleri.

mod ortak;

use std::collections::BTreeMap;
use std::str::FromStr;

use kalfa_evren::butce::JSON_BAYT_SINIRI;
use kalfa_evren::karar::{KararCevabi, KararIstegi, KararKullanimi, KararSorusu, KararTuru};
use kalfa_evren::{ApiAnahtari, EvrenError, EvrenIstemcisi};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ANAHTAR: &str = "evren_llm_test_anahtari";

fn ornek_istek() -> KararIstegi {
    let mut questions = BTreeMap::new();
    questions.insert(
        "i1_guvenli".into(),
        KararSorusu {
            tur: KararTuru::Noul,
            instructions: "İşlem güvenli mi?".into(),
            options: Vec::new(),
        },
    );
    questions.insert(
        "i2_risk".into(),
        KararSorusu {
            tur: KararTuru::Choice,
            instructions: "Risk türü nedir?".into(),
            options: vec![
                "yok".into(),
                "yikici".into(),
                "veri_sizdirma".into(),
                "kapsam_disi".into(),
                "guvenlik_zayiflatma".into(),
                "supheli_kaynak".into(),
            ],
        },
    );
    KararIstegi {
        model: "clef".into(),
        state: json!({
            "kullanici_istegi": ["bağımlılıkları kur"],
            "calisma_alani": "proje",
            "islemler": [{"kimlik": "i1", "arac": "kabuk", "komut": "npm install"}]
        }),
        questions,
        max_len: None,
    }
}

#[tokio::test]
async fn karar_istegi_systemone_yoluna_gider_ve_gercek_kayit_cozulur() {
    let (s, i) = sunucu_ve_istemci().await;
    let kayit = ortak::kayit_bayt("systemone_clef.json");
    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .and(header(
            "authorization",
            format!("Bearer {ANAHTAR}").as_str(),
        ))
        .respond_with(
            json_yanit(200, kayit)
                .insert_header("x-request-id", "karar-1")
                .insert_header("x-evren-routed-model", "Cloudflare/clef")
                .insert_header("x-evren-credits-remaining", "1070.0000")
                .insert_header("x-evren-credits-held", "0.0000"),
        )
        .expect(1)
        .mount(&s)
        .await;

    let istek = ornek_istek();
    let (yanit, meta) = i.karar(&istek).await.unwrap();

    let gonderilen: Value =
        serde_json::from_slice(&s.received_requests().await.unwrap()[0].body).unwrap();
    let beklenen = serde_json::to_value(&istek).unwrap();
    assert_eq!(gonderilen, beklenen);
    assert!(
        gonderilen["questions"]["i1_guvenli"]
            .get("options")
            .is_none()
    );
    assert!(gonderilen.get("max_len").is_none());

    assert_eq!(
        yanit.id.as_deref(),
        Some("e6576a67-c884-4ec7-b5e7-662a3d009422")
    );
    assert_eq!(yanit.model, "clef");
    assert_eq!(
        yanit.usage,
        Some(KararKullanimi {
            input_tokens: 632,
            output_tokens: 0,
        })
    );

    match &yanit.answers["i1_guvenli"] {
        KararCevabi::Noul { noul } => assert!((*noul - 0.8152324893520001).abs() < 1e-12),
        diger => panic!("i1_guvenli Noul bekleniyordu: {diger:?}"),
    }
    match &yanit.answers["i2_guvenli"] {
        KararCevabi::Noul { noul } => assert!((*noul - 0.01648199798672662).abs() < 1e-12),
        diger => panic!("i2_guvenli Noul bekleniyordu: {diger:?}"),
    }
    match &yanit.answers["i2_risk"] {
        KararCevabi::Choice {
            choice,
            confidence,
            probabilities,
        } => {
            assert_eq!(choice, "veri_sizdirma");
            assert!((*confidence - 0.9236427837447773).abs() < 1e-12);
            assert_eq!(probabilities.len(), 6);
            assert!((probabilities["veri_sizdirma"] - 0.936368986453981).abs() < 1e-12);
        }
        diger => panic!("i2_risk Choice bekleniyordu: {diger:?}"),
    }

    assert_eq!(meta.istek_id.as_deref(), Some("karar-1"));
    assert_eq!(meta.yonlendirilen_model.as_deref(), Some("Cloudflare/clef"));
    let kredi = meta.kredi.expect("kredi başlıklarından gözlem üretilmeli");
    assert_eq!(kredi.kalan, Some(Decimal::from_str("1070").unwrap()));
    assert_eq!(kredi.tutulan, Some(Decimal::ZERO));
}

#[tokio::test]
async fn bilinmeyen_cevap_turu_diger_olarak_cozulur() {
    let (s, i) = sunucu_ve_istemci().await;
    let govde = json!({
        "model": "clef",
        "answers": {
            "i1_guvenli": {"type": "score", "score": 0.75},
            "i2_guvenli": {"type": "noul", "noul": 0.91}
        }
    });
    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(json_yanit(200, serde_json::to_vec(&govde).unwrap()))
        .expect(1)
        .mount(&s)
        .await;

    let (yanit, _) = i.karar(&ornek_istek()).await.unwrap();
    assert_eq!(yanit.answers["i1_guvenli"], KararCevabi::Diger);
    assert_eq!(
        yanit.answers["i2_guvenli"],
        KararCevabi::Noul { noul: 0.91 }
    );
}

#[tokio::test]
async fn sunucu_400_ve_422_hatalari_mevcut_eslemeyle_doner() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(json_yanit(
            422,
            r#"{"error":{"code":"validation_error","message":"soru sınırı aşıldı"}}"#
                .as_bytes()
                .to_vec(),
        ))
        .up_to_n_times(1)
        .mount(&s)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(json_yanit(
            400,
            r#"{"error":{"code":"bad_request","message":"geçersiz durum"}}"#
                .as_bytes()
                .to_vec(),
        ))
        .mount(&s)
        .await;

    match i.karar(&ornek_istek()).await {
        Err(EvrenError::Sunucu { durum, kod, mesaj }) => {
            assert_eq!(durum, 422);
            assert_eq!(kod.as_deref(), Some("validation_error"));
            assert!(mesaj.contains("soru sınırı"), "{mesaj}");
        }
        diger => panic!("422 Sunucu hatası bekleniyordu: {diger:?}"),
    }

    match i.karar(&ornek_istek()).await {
        Err(EvrenError::Sunucu { durum, kod, mesaj }) => {
            assert_eq!(durum, 400);
            assert_eq!(kod.as_deref(), Some("bad_request"));
            assert!(mesaj.contains("geçersiz durum"), "{mesaj}");
        }
        diger => panic!("400 Sunucu hatası bekleniyordu: {diger:?}"),
    }
}

#[tokio::test]
async fn bozuk_json_gecersiz_yanit_doner() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(json_yanit(200, b"{bozuk json".to_vec()))
        .mount(&s)
        .await;

    let hata = i.karar(&ornek_istek()).await.unwrap_err();
    assert!(matches!(hata, EvrenError::GecersizYanit(_)), "{hata:?}");
}

#[tokio::test]
async fn boyut_sinirini_asan_govde_yanit_siniri_doner() {
    let (s, i) = sunucu_ve_istemci().await;
    let dolgu = "a".repeat(JSON_BAYT_SINIRI);
    let govde = format!(r#"{{"model":"clef","answers":{{}},"extra":"{dolgu}"}}"#);
    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(json_yanit(200, govde.into_bytes()))
        .mount(&s)
        .await;

    assert_eq!(
        i.karar(&ornek_istek()).await.unwrap_err(),
        EvrenError::YanitSiniri
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
