//! Ölçüm: 20 dosya yazan tek bir model yanıtının planlama ve yürütme süresi.
//! Git korumalarının çözümü (`git config`) bu süreye hâkimdir. Sonuçlar
//! `docs/superpowers/bulgular/2026-09-28-git-koruma-cozum-olcumu.md`'de.
//!
//! ```sh
//! cargo test -p kalfa-tools --test olcum_planlama -- --ignored --nocapture
//! ```

use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use kalfa_core::git_koruma;
use kalfa_core::izin::{Kip, Siniflandirici};
use kalfa_core::oturum::OturumAyarlari;
use kalfa_core::protokol::{Komut, Olay};
use kalfa_core::test_destek::*;
use kalfa_tools::{CalismaAlani, varsayilan_araclar};
use serde_json::json;

/// Oturum görevi engellerse olaylar gecikmesin diye çok iş parçacıklı.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "ölçüm; süre yazdırır"]
async fn yirmi_dosya_yazan_yanitin_planlamasi() {
    let alan = tempfile::tempdir().unwrap();
    let durum = Command::new("git")
        .arg("-C")
        .arg(alan.path())
        .args(["init", "-q"])
        .status()
        .expect("git kurulu olmalı");
    assert!(durum.success());

    let t = Instant::now();
    for _ in 0..5 {
        git_koruma::korumali_yollar(alan.path());
    }
    println!("korumali_yollar: {:?}/çağrı", t.elapsed() / 5);

    let siniflandirici = Arc::new(Siniflandirici::yeni(alan.path(), &[]).unwrap());
    let cagrilar: Vec<(&str, serde_json::Value)> = (0..20)
        .map(|i| {
            (
                "dosya_yaz",
                json!({"yol": format!("d{i}.txt"), "icerik": "x\n"}),
            )
        })
        .collect();
    let s = SahteSaglayici::yeni(vec![SahteYanit::araclar(cagrilar), SahteYanit::metin("ok")]);
    let calisma_alani = CalismaAlani::ac(siniflandirici.clone()).unwrap();
    let mut ayar = OturumAyarlari::yeni("glm-5.3", alan.path());
    ayar.kip = Kip::Serbest;
    let mut d = test_oturumu(
        test_baglami(s.clone(), varsayilan_araclar(calisma_alani), siniflandirici),
        ayar,
    )
    .zaman_asimi(Duration::from_secs(120));

    let t0 = Instant::now();
    d.gonder(Komut::KullaniciMesaji {
        metin: "yaz".into(),
    })
    .await;
    // Serbest çağrıların hepsi planlanınca yürütme başlar.
    d.bekle(|o| matches!(o, Olay::AracCagrisiBasladi { .. }))
        .await;
    let planlama = t0.elapsed();
    d.tur_sonu().await;
    let toplam = t0.elapsed();
    println!(
        "planlama: {planlama:?}, yürütme: {:?}, toplam: {toplam:?}",
        toplam - planlama
    );
    for i in 0..20 {
        assert!(alan.path().join(format!("d{i}.txt")).exists());
    }
}
