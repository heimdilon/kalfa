//! Eklenti deposu (aşama 4 tasarımı §10): kota, atomik yazma, eklenti ×
//! çalışma alanı ayrımı.

mod ortak;

use ortak::*;
use serde_json::json;

async fn yaz(d: &Duzenek, anahtar: &str, deger: &str) -> String {
    d.cagir(
        "depo",
        json!({"islem": "yaz", "anahtar": anahtar, "deger": deger}),
    )
    .await
    .icerik()
    .to_string()
}

async fn oku(d: &Duzenek, anahtar: &str) -> String {
    d.cagir("depo", json!({"islem": "oku", "anahtar": anahtar}))
        .await
        .icerik()
        .to_string()
}

#[tokio::test]
async fn yazilan_okunur_silinen_gider() {
    let d = Duzenek::yeni(&manifest(&[], &[], 1)).await;
    assert_eq!(oku(&d, "meta").await, "yok");
    assert_eq!(yaz(&d, "meta", "bir").await, "yazıldı");
    assert_eq!(yaz(&d, "meta", "iki").await, "yazıldı");
    assert_eq!(oku(&d, "meta").await, "değer: iki");
    let c = d
        .cagir("depo", json!({"islem": "sil", "anahtar": "meta"}))
        .await;
    assert_eq!(c.icerik(), "silindi");
    assert_eq!(oku(&d, "meta").await, "yok");
}

#[tokio::test]
async fn dosya_adi_anahtarin_ozeti_gecici_dosya_kalmaz() {
    let d = Duzenek::yeni(&manifest(&[], &[], 1)).await;
    yaz(&d, "vektor/1", "x").await;
    let adlar: Vec<String> = std::fs::read_dir(d.depo.path())
        .unwrap()
        .map(|g| g.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        adlar,
        vec![kalfa_core::muhur::ozet_hex(b"vektor/1")],
        "geçici dosya kalmamalı"
    );
}

#[tokio::test]
async fn kota_asilinca_yazma_reddedilir_eski_deger_kalir() {
    let d = Duzenek::yeni(&manifest(&[], &[], 1)).await;
    let yaz_boyut = |anahtar: &'static str, n: u64| {
        d.cagir(
            "depo",
            json!({"islem": "yaz", "anahtar": anahtar, "boyut": n}),
        )
    };
    assert_eq!(yaz_boyut("a", 600_000).await.icerik(), "yazıldı");
    assert_eq!(
        yaz_boyut("b", 600_000).await.icerik(),
        "sinir-asildi: depo kotası (1 MB) doldu"
    );
    // Üzerine yazmada eski değerin yeri sayılmaz.
    assert_eq!(yaz_boyut("a", 900_000).await.icerik(), "yazıldı");
    assert_eq!(oku(&d, "b").await, "yok");
    // Kota çağrılar arasında diskteki dosyalardan yeniden sayılır.
    assert!(
        yaz_boyut("c", 200_000)
            .await
            .icerik()
            .starts_with("sinir-asildi")
    );
}

#[tokio::test]
async fn gecersiz_anahtar_reddedilir() {
    let d = Duzenek::yeni(&manifest(&[], &[], 1)).await;
    assert!(yaz(&d, "", "x").await.starts_with("diger: depo anahtarı"));
    assert!(
        yaz(&d, &"a".repeat(201), "x")
            .await
            .starts_with("diger: depo anahtarı")
    );
}

/// Depo dizini eklenti × çalışma alanı başınadır; çağıran onu ayrı verir.
#[tokio::test]
async fn ayri_depo_dizinleri_birbirini_gormez() {
    let d = Duzenek::yeni(&manifest(&[], &[], 1)).await;
    let obur = tempfile::tempdir().unwrap();
    yaz(&d, "meta", "birinci alan").await;
    let c = cagir(
        &d.arac_depoyla("depo", obur.path()),
        json!({"islem": "oku", "anahtar": "meta"}),
        tokio_util::sync::CancellationToken::new(),
    )
    .await;
    assert_eq!(c.icerik(), "yok");
}
