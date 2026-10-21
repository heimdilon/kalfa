//! G5 v2: sabit K01–K05 girdileri, gerçek Kalfa araçları ve bağımsız
//! Rust/TypeScript/Svelte denetleyicileri. Geniş kabul bağımlılıkları gerektirir.
#![cfg(feature = "yalitim-testleri")]

mod ortak;
#[allow(dead_code)]
mod yetenek_senaryolari;

use yetenek_senaryolari::{
    CalistirmaOrtami, SenaryoSonucu, gorevi_kostur, yirmi_dort_gorevi_yukle,
};

async fn kodlama_kabulu(kimlik: &str) {
    let gorevler = yirmi_dort_gorevi_yukle().unwrap();
    let gorev = gorevler.iter().find(|g| g.kimlik == kimlik).unwrap();
    let kayit = gorevi_kostur(gorev, &CalistirmaOrtami::g0_cevrimdisi_taban()).await;
    assert_eq!(
        kayit.sonuc,
        SenaryoSonucu::Gecti,
        "{kimlik}: {}",
        kayit.neden
    );
    assert!(!kayit.canli_model_basarisi);
    assert!(kayit.arac_cagrisi_sayisi > 0);
    assert_eq!(kayit.dosya_sha256, gorev.dosya_sha256);
    println!("G5_GOREV:{kimlik}:{}", kayit.calisma_izi_ozeti);
}

#[tokio::test]
async fn g5_k01_rust_gercek_derleyici_ve_davranis_kabulu() {
    kodlama_kabulu("K01").await;
}
#[tokio::test]
async fn g5_k02_typescript_tip_ve_sifir_tutar_kabulu() {
    kodlama_kabulu("K02").await;
}
#[tokio::test]
async fn g5_k03_svelte_tiklama_sifirlama_ve_css_kabulu() {
    kodlama_kabulu("K03").await;
}
#[tokio::test]
async fn g5_k04_cok_dosyali_rust_sozlesme_kabulu() {
    kodlama_kabulu("K04").await;
}
#[tokio::test]
async fn g5_k05_gercek_uretim_fonksiyonu_ve_satir_kabulu() {
    kodlama_kabulu("K05").await;
}

#[tokio::test]
async fn g5_m01_gercek_yalitimli_stdio_mcp_sirlari_korur_izinleri_geri_alir() {
    if cfg!(windows) {
        assert!(
            std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
                && std::env::var("KALFA_G5_M01_CI").as_deref() == Ok("1"),
            "Windows M01 yalnız geçici kurucu CI kabulünde çalıştırılır"
        );
    }
    let gorevler = yirmi_dort_gorevi_yukle().unwrap();
    let gorev = gorevler.iter().find(|g| g.kimlik == "M01").unwrap();
    let mut ortam = CalistirmaOrtami::g0_cevrimdisi_taban();
    ortam.gercek_yalitim_acik = true;
    ortam.yerel_surec_acik = true;
    ortam.g5_windows_hazirligi = true;
    let kayit = gorevi_kostur(gorev, &ortam).await;
    assert_eq!(kayit.sonuc, SenaryoSonucu::Gecti, "M01: {}", kayit.neden);
    assert!(kayit.arac_cagrisi_sayisi >= 3);
    println!("G5_GOREV:M01:{}", kayit.calisma_izi_ozeti);
}
