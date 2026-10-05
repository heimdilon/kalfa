use std::time::Duration;

use futures_util::StreamExt;
use kalfa_core::arac::{Arac, AracHatasi};
use kalfa_core::saglayici::{EvrenSaglayici, Saglayici};
use kalfa_core::test_destek::{SahteArac, SahteSaglayici, SahteYanit};
use kalfa_evren::tipler::{ChatMessage, ChatRequest};
use kalfa_evren::{AkisOlayi, ApiAnahtari, EvrenError, EvrenIstemcisi};
use serde_json::json;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn istek() -> ChatRequest {
    ChatRequest::new("glm-5.3", vec![ChatMessage::user("selam")])
}

#[tokio::test]
async fn sahte_saglayici_senaryoyu_sirayla_verir() {
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::metin("a"),
        SahteYanit::Hata(EvrenError::ag_baglanti("yok")),
    ]);
    let akis = s.akis(istek()).await.unwrap();
    assert_eq!(akis.meta.istek_id.as_deref(), Some("sahte-1"));
    let olaylar: Vec<_> = akis.olaylar.collect().await;
    assert_eq!(
        olaylar,
        vec![
            Ok(AkisOlayi::Metin("a".into())),
            Ok(AkisOlayi::Bitti {
                neden: "stop".into()
            })
        ]
    );
    assert!(matches!(s.akis(istek()).await, Err(EvrenError::Ag { .. })));
    assert_eq!(s.istekler().len(), 2);
}

#[tokio::test]
async fn model_cagri_kimlikleri_benzersiz() {
    let kimlik = |y: SahteYanit| match y {
        SahteYanit::Akis(o) => match &o[0] {
            Ok(AkisOlayi::AracCagrilari(v)) => v[0].id.clone(),
            diger => panic!("{diger:?}"),
        },
        _ => panic!("akış bekleniyordu"),
    };
    let a = kimlik(SahteYanit::arac("oku", json!({"yol": "a"})));
    let b = kimlik(SahteYanit::arac("oku", json!({"yol": "a"})));
    assert_ne!(a, b);
}

#[tokio::test]
async fn sahte_sart_surumu() {
    let s = SahteSaglayici::yeni(vec![]);
    assert_eq!(s.sart_metni().await.unwrap().version, 1);
    assert_eq!(
        s.sart_kabul(2).await,
        Err(EvrenError::SartKabulGerekli { surum: Some(1) })
    );
    s.sart_kabul(1).await.unwrap();
    s.sart_surumunu_degistir(3);
    assert!(s.sart_kabul(1).await.is_err());
    assert_eq!(s.kabul_edilen_surumler(), vec![1]);
}

#[tokio::test]
async fn sahte_arac_hazirlik_calisma_ve_iptal() {
    let okuyan = SahteArac::okuyan("oku").yanit("içerik".into());
    let h = okuyan.hazirla(&json!({"yol": "a.txt"})).await.unwrap();
    assert_eq!(h.okunan[0].as_str(), "a.txt");
    let c = okuyan
        .calistir(&json!({"yol": "a.txt"}), &h, CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(
        (c.icerik.as_str(), c.kaynak.as_str()),
        ("içerik", "sahte:oku")
    );
    assert_eq!(okuyan.cagri_sayisi(), 1);

    let yazan = SahteArac::yazan("yaz");
    let h = yazan.hazirla(&json!({"yol": "b.txt"})).await.unwrap();
    assert_eq!(h.yazilan[0].onceki_ozet.as_deref(), Some("surum-0"));
    yazan.dosyayi_degistir();
    let h2 = yazan.hazirla(&json!({"yol": "b.txt"})).await.unwrap();
    assert_eq!(h2.yazilan[0].onceki_ozet.as_deref(), Some("surum-1"));

    let hatali = SahteArac::okuyan("oku").hata("okunamadı".into());
    let h = hatali.hazirla(&json!({"yol": "a"})).await.unwrap();
    assert_eq!(
        hatali
            .calistir(&json!({"yol": "a"}), &h, CancellationToken::new())
            .await,
        Err(AracHatasi::Gecersiz("okunamadı".into()))
    );

    let yavas = SahteArac::okuyan("oku").gecikme(Duration::from_secs(30));
    let iptal = CancellationToken::new();
    iptal.cancel();
    let h = yavas.hazirla(&json!({"yol": "a"})).await.unwrap();
    assert_eq!(
        yavas.calistir(&json!({"yol": "a"}), &h, iptal).await,
        Err(AracHatasi::Iptal)
    );
}

#[tokio::test]
async fn evren_saglayici_istemciyi_sarar() {
    let sunucu = MockServer::start().await;
    let akis = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../research/evren-probe/fixtures/chat_tool_calls_stream_qwen_flash.sse"),
    )
    .unwrap();
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(akis, "text/event-stream")
                .insert_header("x-request-id", "q1"),
        )
        .mount(&sunucu)
        .await;
    let istemci =
        EvrenIstemcisi::yeni(format!("{}/v1", sunucu.uri()), ApiAnahtari::yeni("k")).unwrap();
    let s = EvrenSaglayici::yeni(istemci);
    let a = s.akis(istek()).await.unwrap();
    assert_eq!(a.meta.istek_id.as_deref(), Some("q1"));
    let olaylar: Vec<_> = a.olaylar.collect().await;
    assert!(olaylar.iter().any(|o| matches!(o, Ok(AkisOlayi::AracCagrilari(v)) if v[0].function.arguments == "{\"sehir\": \"İzmir\"}")));
}
