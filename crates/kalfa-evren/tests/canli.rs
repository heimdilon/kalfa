//! Canlı EVREN testleri. Varsayılan olarak atlanır.
//!
//! Çalıştırma (depo kökünde, `.env` içinde EVREN_API_KEY ve EVREN_BASE_URL):
//!   set -a && . ./.env && set +a && cargo test -p kalfa-evren --test canli -- --ignored
//!
//! Probe sırasında bütün modellerin fiyatı 0.0 CR idi (spec §3.5).

use futures_util::StreamExt;
use kalfa_evren::tipler::{ChatMessage, ChatRequest, ToolDef};
use kalfa_evren::{AkisOlayi, ApiAnahtari, EvrenIstemcisi, VARSAYILAN_TABAN};
use serde_json::json;

fn canli_istemci() -> EvrenIstemcisi {
    let anahtar = std::env::var("EVREN_API_KEY").expect("EVREN_API_KEY tanımlı olmalı");
    let taban = std::env::var("EVREN_BASE_URL").unwrap_or_else(|_| VARSAYILAN_TABAN.to_string());
    EvrenIstemcisi::yeni(taban, ApiAnahtari::yeni(anahtar)).unwrap()
}

#[tokio::test]
#[ignore = "canlı EVREN API; EVREN_API_KEY gerekir"]
async fn canli_sart_durumu_kabul_edilmis() {
    let d = canli_istemci().sart_durumu().await.unwrap();
    assert!(
        d.accepted,
        "hesapta şartlar kabul edilmiş olmalı (şart kabulü testle yapılmaz)"
    );
}

#[tokio::test]
#[ignore = "canlı EVREN API; EVREN_API_KEY gerekir"]
async fn canli_katalog_profil_modellerini_icerir() {
    let m = canli_istemci().modeller().await.unwrap();
    for id in ["glm-5.3", "deepseek-v4-flash", "qwen3.8-flash-next"] {
        assert!(
            m.iter().any(|x| x.id == id && x.task == "chat"),
            "{id} katalogda chat olarak olmalı"
        );
    }
}

#[tokio::test]
#[ignore = "canlı EVREN API; EVREN_API_KEY gerekir"]
async fn canli_akisli_arac_cagrisi() {
    let mut istek = ChatRequest::new(
        "qwen3.8-flash-next",
        vec![ChatMessage::user("İzmir'de hava nasıl?")],
    );
    istek.tools.push(ToolDef::function(
        "hava_durumu",
        "Bir şehrin güncel hava durumunu getirir",
        json!({"type": "object", "properties": {"sehir": {"type": "string"}}, "required": ["sehir"]}),
    ));
    let (meta, akis) = canli_istemci().sohbet_akisi(&istek).await.unwrap();
    assert!(meta.istek_id.is_some(), "X-Request-Id gelmeli");
    let olaylar: Vec<_> = akis.collect().await;
    let cagri = olaylar
        .iter()
        .find_map(|o| match o {
            Ok(AkisOlayi::AracCagrilari(v)) => Some(v[0].clone()),
            _ => None,
        })
        .expect("araç çağrısı gelmeli");
    assert_eq!(cagri.function.name, "hava_durumu");
    let arg: serde_json::Value = serde_json::from_str(&cagri.function.arguments).unwrap();
    assert!(arg["sehir"].as_str().unwrap().contains("zmir"));
}

/// Aşama 3 tasarımı §5.9 (G2): geri almanın Kalfa notu ile sonraki kullanıcı
/// mesajı art arda iki kullanıcı mesajıdır. Üç profil modeli bunu kabul edip
/// yanıt vermeli.
#[tokio::test]
#[ignore = "canlı EVREN API; EVREN_API_KEY gerekir"]
async fn canli_art_arda_iki_kullanici_mesaji_kabul_edilir() {
    for model in ["glm-5.3", "qwen3.8-flash-next", "deepseek-v4-flash"] {
        let istek = ChatRequest::new(
            model,
            vec![
                ChatMessage::user("Merhaba. a.txt dosyasına 'yeni' yaz."),
                ChatMessage::assistant(Some("a.txt dosyasına 'yeni' yazdım.".into()), Vec::new()),
                ChatMessage::user(
                    "[Kalfa] Kullanıcı \"Merhaba. a.txt dosyasına 'yeni' yaz.\" turunun dosya                      değişikliklerini geri aldı.
Önceki hâline dönen: a.txt.",
                ),
                ChatMessage::user("a.txt şimdi hangi hâlde? Tek cümleyle yanıtla."),
            ],
        );
        let (_, akis) = canli_istemci()
            .sohbet_akisi(&istek)
            .await
            .unwrap_or_else(|e| panic!("{model}: istek reddedildi: {e}"));
        let olaylar: Vec<_> = akis.collect().await;
        let metin: String = olaylar
            .iter()
            .filter_map(|o| match o {
                Ok(AkisOlayi::Metin(m)) => Some(m.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            olaylar.iter().all(Result::is_ok),
            "{model}: akış hatası: {olaylar:?}"
        );
        assert!(!metin.trim().is_empty(), "{model}: yanıt boş");
    }
}
