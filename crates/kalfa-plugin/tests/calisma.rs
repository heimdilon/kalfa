//! Çağrı ve kaynak sınırları (aşama 4 tasarımı §7.1, §7.4–§7.6).

mod ortak;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use kalfa_core::arac::AracHatasi;
use kalfa_core::protokol::CiktiAkisi;
use ortak::*;
use serde_json::json;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn temel_yol_sonuc_gunluk_ve_ilerleme() {
    let d = Duzenek::yeni(&manifest(&[], &[], 0)).await;
    let c = d.cagir("yanki", json!({"metin": "merhaba"})).await;
    assert_eq!(c.icerik(), "merhaba");
    let Ok(cikti) = &c.sonuc else { unreachable!() };
    assert_eq!(cikti.kaynak, "eklenti:kalfa-test:yanki");
    assert!(!cikti.kalfa_uretimi);
    let canli = c.cikti_metni();
    assert!(canli.contains("[bilgi] yanki çağrıldı"), "{canli}");
    assert!(canli.contains("[ilerleme] 1/1 · bitti"), "{canli}");
}

#[tokio::test]
async fn hazirlik_eklentiyi_tarif_eder_ve_kilitsizdir() {
    let d = Duzenek::yeni(&manifest(&["**/*.rs"], &["embeddings"], 0)).await;
    let h = d.arac("yanki").hazirla(&json!({})).await.unwrap();
    assert!(h.kilitsiz);
    assert!(h.okunan.is_empty() && h.yazilan.is_empty() && !h.tum_calisma_alani);
    let t = h.eklenti.unwrap();
    assert_eq!(t.ad, "kalfa-test");
    assert_eq!(t.ozet, d.eklenti.ozet);
    assert_eq!(t.globlar, vec!["**/*.rs"]);
    assert_eq!(t.evren_uclari, vec!["embeddings"]);
    assert!(
        h.aciklama
            .contains("kalfa-test eklentisinin `yanki` aracını")
    );
}

/// CPU bütçesinde durur (§7.4, G3). Tek iş parçacıklı çalışma zamanında
/// başka bir görev ilerlemeye devam eder: eklenti her tikte yer verir.
#[tokio::test(flavor = "current_thread")]
async fn sonsuz_dongu_cpu_butcesinde_durur_ve_yer_verir() {
    let d = Duzenek::yeni(&manifest(&[], &[], 0)).await;
    let sayac = Arc::new(AtomicU64::new(0));
    let s = sayac.clone();
    let gorev = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(10)).await;
            s.fetch_add(1, Ordering::SeqCst);
        }
    });
    let bas = Instant::now();
    let c = d.cagir("dongu", json!({})).await;
    let sure = bas.elapsed();
    gorev.abort();
    assert!(c.hata().contains("işlem süresi sınırı"), "{}", c.hata());
    // Bütçe 2 sn (test süreleri).
    assert!(
        sure >= Duration::from_millis(1800) && sure < Duration::from_secs(10),
        "{sure:?}"
    );
    let ilerleme = sayac.load(Ordering::SeqCst);
    assert!(ilerleme > 50, "öteki görev ilerlemedi: {ilerleme}");
    // Sonraki çağrı temiz bir örnekle çalışır.
    assert_eq!(d.cagir("yanki", json!({"metin": "x"})).await.icerik(), "x");
}

#[tokio::test]
async fn bellek_siniri_cagriyi_durdurur_sonraki_calisir() {
    let d = Duzenek::yeni(&manifest(&[], &[], 0)).await;
    let c = d.cagir("bellek", json!({})).await;
    assert!(c.hata().contains("bellek sınırı (512 MiB)"), "{}", c.hata());
    assert_eq!(d.cagir("yanki", json!({"metin": "x"})).await.icerik(), "x");
}

#[tokio::test]
async fn sisen_sonuc_kirpilir_gunluk_sinirlanir() {
    let d = Duzenek::yeni(&manifest(&[], &[], 0)).await;
    let c = d.cagir("sisirme", json!({})).await;
    let icerik = c.icerik();
    assert!(icerik.len() < 64 * 1024 + 200, "{}", icerik.len());
    assert!(icerik.contains("bayta kırpıldı"));
    let canli = c.cikti_metni();
    assert_eq!(canli.matches("[ayrıntı]").count(), 200);
    assert!(canli.contains("[günlük] 100 satır"), "{canli}");
}

#[tokio::test]
async fn panik_tuzak_olur_stderr_yalnizca_canli_ciktida() {
    let d = Duzenek::yeni(&manifest(&[], &[], 0)).await;
    let c = d.cagir("panik", json!({})).await;
    let hata = c.hata();
    assert!(hata.contains("çalışırken çöktü"), "{hata}");
    assert!(!hata.contains("bilerek panik"), "{hata}");
    let stderr: String = c
        .cikti
        .iter()
        .filter(|(a, _)| *a == CiktiAkisi::Hata)
        .map(|(_, m)| m.as_str())
        .collect();
    assert!(stderr.contains("bilerek panik"), "{stderr}");
    assert!(stderr.contains("stderr satırı"), "{stderr}");
}

#[tokio::test]
async fn eklenti_hatasi_tur_ve_iletiyle_doner() {
    let d = Duzenek::yeni(&manifest(&[], &[], 0)).await;
    let c = d.cagir("dosya", json!({"islem": "yok"})).await;
    assert_eq!(
        c.hata(),
        "Eklenti hatası (kalfa-test): diger: bilinmeyen işlem: yok"
    );
}

#[tokio::test]
async fn iptale_uyan_eklenti_hemen_cikar() {
    let d = Duzenek::yeni(&manifest(&[], &[], 0)).await;
    let iptal = CancellationToken::new();
    let i = iptal.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        i.cancel();
    });
    let bas = Instant::now();
    let c = d.cagir_iptalle("iptal", json!({}), iptal).await;
    assert!(matches!(c.sonuc, Err(AracHatasi::Iptal)), "{:?}", c.sonuc);
    assert!(
        bas.elapsed() < Duration::from_millis(1500),
        "{:?}",
        bas.elapsed()
    );
}

/// İptali yok sayan eklenti 2 sn sonra bırakılır (§7.5).
#[tokio::test]
async fn iptali_yok_sayan_eklenti_iki_saniyede_birakilir() {
    let d = Duzenek::cpu(&manifest(&[], &[], 0), Duration::from_secs(20)).await;
    let iptal = CancellationToken::new();
    let i = iptal.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        i.cancel();
    });
    let bas = Instant::now();
    let c = d
        .cagir_iptalle("iptal", json!({"inatci": true}), iptal)
        .await;
    let sure = bas.elapsed();
    assert!(matches!(c.sonuc, Err(AracHatasi::Iptal)), "{:?}", c.sonuc);
    assert!(
        sure >= Duration::from_millis(2100) && sure < Duration::from_millis(2900),
        "{sure:?}"
    );
}

/// G3: host çağrısında beklenen süre CPU bütçesine sayılmaz. Dosyanın yazma
/// kilidi bütçeden (2 sn) uzun tutulur; eklentinin okuması kilidi bekler ve
/// yine de başarılı olur. Kilit bekleyen okuma kabukla eklentinin aynı
/// dosyada birbirinin ortasına girmediğini de gösterir (§8.3).
#[tokio::test]
async fn host_cagrisinda_beklenen_sure_cpu_butcesine_sayilmaz() {
    let d = Duzenek::yeni(&manifest(&["**/*"], &[], 0)).await;
    d.dosya("a.rs", b"fn a() {}\n");
    let yol = kalfa_core::yol::CalismaYolu::coz("a.rs").unwrap();
    let kilit = d.kilitler.al(&[], &[yol], false).await;
    let birak = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(3)).await;
        drop(kilit);
    });
    let bas = Instant::now();
    let c = d
        .cagir("dosya", json!({"islem": "oku", "yol": "a.rs"}))
        .await;
    assert_eq!(c.icerik(), "fn a() {}\n");
    assert!(
        bas.elapsed() >= Duration::from_millis(2900),
        "{:?}",
        bas.elapsed()
    );
    birak.await.unwrap();
}
