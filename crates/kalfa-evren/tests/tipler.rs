mod ortak;

use std::str::FromStr;

use kalfa_evren::tipler::*;
use rust_decimal::Decimal;
use serde_json::json;

fn yanit(ad: &str) -> ChatResponse {
    serde_json::from_slice(&ortak::fixture_bayt(ad)).unwrap_or_else(|e| panic!("{ad}: {e}"))
}

#[test]
fn glm_bloklu_arac_cagrisi_ve_evren_alanlari() {
    let y = yanit("chat_tool_calls_blocking.json");
    let m = &y.choices[0].message;
    assert_eq!(m.role, Role::Assistant);
    let cagrilar = m.tool_calls.as_ref().expect("araç çağrısı olmalı");
    assert_eq!(cagrilar.len(), 2, "paralel iki çağrı");
    assert_eq!(cagrilar[0].kind, "function");
    assert_eq!(cagrilar[0].function.name, "hava_durumu");
    let arg: serde_json::Value = serde_json::from_str(&cagrilar[0].function.arguments).unwrap();
    assert_eq!(arg["sehir"], "Ankara");
    assert!(
        m.reasoning.as_deref().is_some_and(|r| !r.is_empty()),
        "reasoning alanı"
    );

    let u = y.usage.expect("usage");
    assert_eq!(u.reasoning_tokens(), 32);
    assert!(
        u.reasoning_tokens() <= u.completion_tokens,
        "reasoning completion'ın içinde"
    );
    let e = u.evren.expect("usage.evren");
    assert_eq!(e.credits_remaining_cr, Decimal::from_str("1000").unwrap());
    assert_eq!(e.credits_held_cr, Decimal::ZERO);
    assert_eq!(e.routed_model.as_deref(), Some("zai/glm-5.3-fp8"));
}

#[test]
fn qwen_ve_deepseek_bloklu_paralel_arac_cagrisi() {
    for ad in [
        "chat_tool_calls_qwen_flash.json",
        "chat_tool_calls_deepseek_flash.json",
    ] {
        let y = yanit(ad);
        let cagrilar = y.choices[0].message.tool_calls.clone().expect(ad);
        let sehirler: Vec<String> = cagrilar
            .iter()
            .map(|c| {
                let v: serde_json::Value = serde_json::from_str(&c.function.arguments).unwrap();
                v["sehir"].as_str().unwrap().to_string()
            })
            .collect();
        assert_eq!(sehirler, vec!["İstanbul", "Ankara"], "{ad}");
    }
}

#[test]
fn json_schema_yanitlari() {
    let y = yanit("chat_json_schema_qwen_flash.json");
    let icerik: serde_json::Value =
        serde_json::from_str(y.choices[0].message.content.as_deref().unwrap()).unwrap();
    assert_eq!(icerik["guvenli"], false);
    assert_eq!(icerik["kategori"], "talimat_enjeksiyonu");

    let y = yanit("chat_json_schema.json");
    let icerik: serde_json::Value =
        serde_json::from_str(y.choices[0].message.content.as_deref().unwrap()).unwrap();
    assert!(icerik.is_object());
}

#[test]
fn model_katalogu() {
    let k: ModelList = serde_json::from_slice(&ortak::fixture_bayt("models.json")).unwrap();
    assert_eq!(k.data.len(), 11);
    let glm = k.data.iter().find(|m| m.id == "glm-5.3").expect("glm-5.3");
    assert_eq!(glm.task, "chat");
    let p = glm.pricing.as_ref().expect("pricing");
    assert_eq!(p.currency, "CR");
    assert_eq!(p.prompt_token_price, Decimal::ZERO);
    let gomme = k
        .data
        .iter()
        .find(|m| m.id == "qwen3-embedding-8b")
        .unwrap();
    assert_eq!(gomme.task, "embedding");
}

#[test]
fn sart_durumu_ve_metni() {
    let d: TermsStatus = serde_json::from_slice(&ortak::fixture_bayt("terms_status.json")).unwrap();
    assert_eq!(
        d,
        TermsStatus {
            current_version: 1,
            is_material: true,
            accepted: true,
            doc_path: Some("docs/legal/LLM_TERMS_OF_SERVICE_v1.md".into())
        }
    );
    let m: TermsText = serde_json::from_slice(&ortak::fixture_bayt("terms_text.json")).unwrap();
    assert_eq!(m.version, 1);
    assert!(m.content.starts_with("# EVREN LLM Gateway"));
}

#[test]
fn hata_govdeleri() {
    let g: ApiErrorBody =
        serde_json::from_slice(&ortak::fixture_bayt("error_model_not_found.json")).unwrap();
    assert_eq!(g.error.code.as_deref(), Some("model_not_found"));
    assert_eq!(
        g.error.evren.unwrap().suggested_models,
        vec!["qwen3-guard-4b"]
    );

    let g: ApiErrorBody =
        serde_json::from_slice(&ortak::fixture_bayt("error_task_mismatch.json")).unwrap();
    assert_eq!(g.error.code.as_deref(), Some("bad_request"));
    assert!(g.error.evren.is_none(), "error.evren koşullu: burada yok");

    let g: ApiErrorBody =
        serde_json::from_slice(&ortak::fixture_bayt("error_unauthorized.json")).unwrap();
    assert_eq!(g.error.code.as_deref(), Some("unauthorized"));
    assert_eq!(g.error.kind.as_deref(), Some("authentication_error"));
}

#[test]
fn istek_serilestirme() {
    let mut r = ChatRequest::new(
        "glm-5.3",
        vec![ChatMessage::system("s"), ChatMessage::user("u")],
    );
    let j = serde_json::to_value(&r).unwrap();
    assert_eq!(j["model"], "glm-5.3");
    assert_eq!(j["stream"], false);
    assert!(j.get("tools").is_none(), "boş araç listesi gönderilmez");
    assert!(j.get("stream_options").is_none());
    assert_eq!(j["messages"][1], json!({"role": "user", "content": "u"}));

    r.tools.push(ToolDef::function(
        "hava_durumu",
        "Hava durumunu getirir",
        json!({"type": "object"}),
    ));
    r.stream = true;
    r.stream_options = Some(StreamOptions {
        include_usage: true,
    });
    let j = serde_json::to_value(&r).unwrap();
    assert_eq!(j["tools"][0]["type"], "function");
    assert_eq!(j["tools"][0]["function"]["name"], "hava_durumu");
    assert_eq!(j["stream_options"]["include_usage"], true);
}

#[test]
fn mesaj_serilestirme_reasoning_gondermez() {
    let mut a = ChatMessage::assistant(
        Some("bakıyorum".into()),
        vec![ToolCall {
            id: "c1".into(),
            kind: "function".into(),
            function: FunctionCall {
                name: "dosya_oku".into(),
                arguments: "{\"yol\":\"a\"}".into(),
            },
        }],
    );
    a.reasoning = Some("gizli düşünce".into());
    let j = serde_json::to_value(&a).unwrap();
    assert!(j.get("reasoning").is_none());
    assert_eq!(j["tool_calls"][0]["type"], "function");

    let t = serde_json::to_value(ChatMessage::tool("c1", "içerik")).unwrap();
    assert_eq!(
        t,
        json!({"role": "tool", "content": "içerik", "tool_call_id": "c1"})
    );

    let bos = serde_json::to_value(ChatMessage::assistant(Some("x".into()), vec![])).unwrap();
    assert!(bos.get("tool_calls").is_none());
}

#[test]
fn max_tokens_verilmezse_govdede_yok() {
    let mut req = ChatRequest::new("glm-5.3", vec![ChatMessage::user("merhaba")]);
    req.max_tokens = None;
    let json = serde_json::to_string(&req).unwrap();
    assert!(!json.contains("max_tokens"));
}

#[test]
fn max_tokens_govdeye_yazilir() {
    let mut req = ChatRequest::new("glm-5.3", vec![ChatMessage::user("merhaba")]);
    req.max_tokens = Some(2048);
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains(r#""max_tokens":2048"#));
}

/// `tests/kayitlar/` altındaki gerçek bir kaydın `usage` nesnesi.
fn kayit_kullanimi(ad: &str) -> Usage {
    let v: serde_json::Value = serde_json::from_slice(&ortak::kayit_bayt(ad)).unwrap();
    serde_json::from_value(v["usage"].clone()).unwrap_or_else(|e| panic!("{ad} usage: {e}"))
}

/// Akış fixture'ının son `usage` nesnesi (SSE `data:` satırlarından).
fn akis_son_kullanimi(ad: &str) -> Usage {
    ortak::fixture_metin(ad)
        .lines()
        .filter_map(|satir| satir.strip_prefix("data: "))
        .filter(|veri| *veri != "[DONE]")
        .map(|veri| serde_json::from_str::<ChatChunk>(veri).unwrap_or_else(|e| panic!("{ad}: {e}")))
        .filter_map(|parca| parca.usage)
        .next_back()
        .expect("akışta usage olmalı")
}

fn kullanim(json: &str) -> Usage {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("usage çözülemedi: {e}"))
}

#[test]
fn butce_kullanimi_sohbet_yanitlari() {
    let u = yanit("chat_tool_calls_blocking.json").usage.expect("usage");
    assert_eq!(u.butce_kullanimi(), Some((186, 78)));
    assert_eq!(u.reasoning_tokens(), 32, "düşünce tamamlamanın dökümü");
    assert_eq!(
        akis_son_kullanimi("chat_tool_calls_stream.sse").butce_kullanimi(),
        Some((169, 67))
    );
}

#[test]
fn butce_kullanimi_gomme_ve_siralama_kayitlari() {
    assert_eq!(
        kayit_kullanimi("embeddings_qwen3_1024.json").butce_kullanimi(),
        Some((5, 0))
    );
    assert_eq!(
        kayit_kullanimi("rerank_qwen3.json").butce_kullanimi(),
        Some((279, 0))
    );
}

#[test]
fn butce_kullanimi_eksik_veya_tutarsiz_alan_bilinmeyendir() {
    assert_eq!(kullanim("{}").butce_kullanimi(), None);
    assert_eq!(kullanim(r#"{"prompt_tokens":5}"#).butce_kullanimi(), None);
    assert_eq!(
        kullanim(r#"{"prompt_tokens":0,"total_tokens":0}"#).butce_kullanimi(),
        None
    );
    assert_eq!(
        kullanim(r#"{"prompt_tokens":5,"completion_tokens":2,"total_tokens":9}"#).butce_kullanimi(),
        None
    );
    // Taşan toplam 0'a sarılır; bu eşleşme kabul edilmez.
    let tasan = format!(
        r#"{{"prompt_tokens":{},"completion_tokens":1,"total_tokens":0}}"#,
        u64::MAX
    );
    assert_eq!(kullanim(&tasan).butce_kullanimi(), None);
}

#[test]
fn negatif_kullanim_serde_de_reddedilir() {
    assert!(serde_json::from_str::<Usage>(r#"{"prompt_tokens":-1,"total_tokens":-1}"#).is_err());
}

#[test]
fn dusunce_tokenlari_tamamlamaya_eklenmez() {
    let u = kullanim(
        r#"{"prompt_tokens":10,"completion_tokens":20,"total_tokens":30,"completion_tokens_details":{"reasoning_tokens":15}}"#,
    );
    assert_eq!(u.butce_kullanimi(), Some((10, 20)));
}
