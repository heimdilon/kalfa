//! Otomatik kip hakeminin canlı ölçümü (otomatik kip tasarımı §8; PR A
//! planı Görev 5). Varsayılan olarak atlanır.
//!
//! Çalıştırma (depo kökünde, `.env` içinde EVREN_API_KEY ve EVREN_BASE_URL):
//!   set -a && . ./.env && set +a && cargo test -p kalfa-core --test hakem_canli -- --ignored --nocapture
//!
//! Her öğe için bir JSON satırı basar; eşikler bu satırlardan çevrimdışı
//! seçilir. Test eşik denetlemez, ölçer.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_trait::async_trait;
use kalfa_core::hakem::{HakemGirdisi, HakemOgesi, degerlendir};
use kalfa_core::saglayici::{EvrenSaglayici, Saglayici, SaglayiciAkisi};
use kalfa_core::test_destek::sinirsiz_butce_talepci;
use kalfa_evren::karar::{KararCevabi, KararIstegi, KararYaniti};
use kalfa_evren::tipler::{ChatRequest, ChatResponse, TermsStatus, TermsText};
use kalfa_evren::{ApiAnahtari, EvrenError, EvrenIstemcisi, VARSAYILAN_TABAN, YanitMeta};
use serde::Deserialize;
use serde_json::json;

const SENARYOLAR: &str = include_str!("veri/hakem_senaryolari.json");

#[derive(Deserialize)]
struct Senaryo {
    ad: String,
    kullanici_istegi: Vec<String>,
    ogeler: Vec<EtiketliOge>,
}

#[derive(Deserialize)]
struct EtiketliOge {
    etiket: String,
    #[serde(flatten)]
    oge: HakemOgesi,
}

/// Ham karar yanıtını saklayan sarmalayıcı: hakemin sonucu risk seçimini
/// yalnız retlerde taşır, eşik analizi her öğenin ham cevabını ister.
struct Kaydeden {
    ic: EvrenSaglayici,
    son: Mutex<Option<(KararYaniti, YanitMeta)>>,
}

#[async_trait]
impl Saglayici for Kaydeden {
    async fn akis(&self, istek: ChatRequest) -> Result<SaglayiciAkisi, EvrenError> {
        self.ic.akis(istek).await
    }
    async fn sohbet(&self, istek: ChatRequest) -> Result<(ChatResponse, YanitMeta), EvrenError> {
        self.ic.sohbet(istek).await
    }
    async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError> {
        self.ic.sart_durumu().await
    }
    async fn sart_metni(&self) -> Result<TermsText, EvrenError> {
        self.ic.sart_metni().await
    }
    async fn sart_kabul(&self, _surum: u32) -> Result<(), EvrenError> {
        // Şart kabul ucu hiçbir koşulda çağrılmaz (AGENTS.md).
        Err(EvrenError::GecersizYanit(
            "ölçümde şart kabul edilmez".into(),
        ))
    }
    async fn karar(&self, istek: KararIstegi) -> Result<(KararYaniti, YanitMeta), EvrenError> {
        let sonuc = self.ic.karar(istek).await;
        if let Ok(y) = &sonuc {
            *self.son.lock().unwrap() = Some(y.clone());
        }
        sonuc
    }
}

fn canli_saglayici() -> Arc<Kaydeden> {
    let anahtar = std::env::var("EVREN_API_KEY").expect("EVREN_API_KEY tanımlı olmalı");
    let taban = std::env::var("EVREN_BASE_URL").unwrap_or_else(|_| VARSAYILAN_TABAN.to_string());
    Arc::new(Kaydeden {
        ic: EvrenSaglayici::yeni(EvrenIstemcisi::yeni(taban, ApiAnahtari::yeni(anahtar)).unwrap()),
        son: Mutex::new(None),
    })
}

#[test]
fn senaryo_kumesi_tasarimdaki_dagilima_uyar() {
    let senaryolar: Vec<Senaryo> = serde_json::from_str(SENARYOLAR).unwrap();
    assert!(senaryolar.len() >= 40, "en az 40 senaryo");
    let etiketler: Vec<&str> = senaryolar
        .iter()
        .flat_map(|s| s.ogeler.iter().map(|o| o.etiket.as_str()))
        .collect();
    assert!(etiketler.iter().all(|e| *e == "guvenli" || *e == "riskli"));
    let senaryo_riskli = |s: &Senaryo| s.ogeler.iter().any(|o| o.etiket == "riskli");
    assert_eq!(senaryolar.iter().filter(|s| senaryo_riskli(s)).count(), 20);
    assert_eq!(senaryolar.iter().filter(|s| !senaryo_riskli(s)).count(), 20);
}

#[tokio::test]
#[ignore = "canlı EVREN API; EVREN_API_KEY gerekir"]
async fn canli_hakem_senaryo_kumesi() {
    let senaryolar: Vec<Senaryo> = serde_json::from_str(SENARYOLAR).unwrap();
    let s = canli_saglayici();
    for senaryo in senaryolar {
        let girdi = HakemGirdisi {
            kullanici_istegi: senaryo.kullanici_istegi.clone(),
            calisma_alani: "deneme".into(),
            ogeler: senaryo.ogeler.iter().map(|o| o.oge.clone()).collect(),
        };
        *s.son.lock().unwrap() = None;
        let bas = Instant::now();
        let y = degerlendir(s.as_ref(), &sinirsiz_butce_talepci(), &girdi).await;
        let sure_ms = bas.elapsed().as_millis();
        let ham = s.son.lock().unwrap().clone();
        for o in &senaryo.ogeler {
            let k = &o.oge.kimlik;
            let cevap = |ek: &str| {
                ham.as_ref()
                    .and_then(|(y, _)| y.answers.get(&format!("{k}{ek}")).cloned())
            };
            let p = match cevap("_guvenli") {
                Some(KararCevabi::Noul { noul }) => Some(noul),
                _ => None,
            };
            let (risk, risk_olasiligi) = match cevap("_risk") {
                Some(KararCevabi::Choice {
                    choice, confidence, ..
                }) => (Some(choice), Some(confidence)),
                _ => (None, None),
            };
            println!(
                "{}",
                json!({
                    "ad": senaryo.ad,
                    "etiket": o.etiket,
                    "kimlik": k,
                    "P": p,
                    "risk": risk,
                    "risk_olasiligi": risk_olasiligi,
                    "sonuc": format!("{:?}", y.sonuclar.get(k)),
                    "sure_ms": sure_ms,
                    "istek_id": ham.as_ref().and_then(|(_, m)| m.istek_id.clone()),
                    "input_tokens": y.denemeler.last().and_then(|d| d.kullanim).map(|u| u.istem),
                })
            );
        }
    }
}
