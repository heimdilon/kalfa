//! Özetleyicinin canlı denemesi (5a tasarımı §14). Varsayılan olarak atlanır;
//! yalnız kullanıcının ayrı onayıyla koşar.
//!
//!   set -a && . ./.env && set +a && cargo test -p kalfa-core --test baglam_canli -- --ignored --nocapture

use std::sync::Arc;

use kalfa_core::oturum::baglam::ozet_istegi;
use kalfa_core::saglayici::{EvrenSaglayici, Saglayici};
use kalfa_evren::{ApiAnahtari, EvrenIstemcisi, VARSAYILAN_TABAN};

fn canli_saglayici() -> Arc<dyn Saglayici> {
    let anahtar = std::env::var("EVREN_API_KEY").expect("EVREN_API_KEY tanımlı olmalı");
    let taban = std::env::var("EVREN_BASE_URL").unwrap_or_else(|_| VARSAYILAN_TABAN.to_string());
    Arc::new(EvrenSaglayici::yeni(
        EvrenIstemcisi::yeni(taban, ApiAnahtari::yeni(anahtar)).unwrap(),
    ))
}

#[tokio::test]
#[ignore = "canlı EVREN API; EVREN_API_KEY gerekir"]
async fn canli_ozet_yolu_ve_hatayi_korur() {
    let mut cikti = String::from("<arac_sonucu kaynak=\"kabuk:cargo test\" sinir=\"s1\">\n");
    for i in 0..60 {
        cikti.push_str(&format!("test oturum::t{i} ... ok\n"));
    }
    cikti.push_str("error[E0308]: mismatched types\n --> crates/ornek/src/lib.rs:42:9\n");
    cikti.push_str("test result: FAILED. 60 passed; 1 failed\n</arac_sonucu sinir=\"s1\">");
    let (yanit, meta) = canli_saglayici().sohbet(ozet_istegi(&cikti)).await.unwrap();
    let ozet = yanit.choices[0].message.content.clone().unwrap_or_default();
    println!(
        "girdi={} bayt, özet={} bayt, kullanım={:?}, istek={:?}\n{ozet}",
        cikti.len(),
        ozet.len(),
        yanit.usage,
        meta.istek_id
    );
    assert!(!ozet.trim().is_empty());
    assert!(ozet.contains("lib.rs"), "{ozet}");
    assert!(ozet.contains("E0308"), "{ozet}");
    assert!(ozet.len() < cikti.len(), "{ozet}");
}
