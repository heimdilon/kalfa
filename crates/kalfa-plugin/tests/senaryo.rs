//! S15 (aşama 4 tasarımı §16.3): kötü niyetli eklenti gizli bir dosyayı
//! okumaya çalışıyor. Oturumun içinden, yerleşik araçlarla birlikte.
//! Gizli içerik hiçbir model isteğine girmez.

mod ortak;

use kalfa_core::izin::Kip;
use kalfa_core::oturum::OturumAyarlari;
use kalfa_core::protokol::{BitisNedeni, Komut, Olay};
use kalfa_core::test_destek::{SahteSaglayici, SahteYanit, test_baglami, test_oturumu};
use ortak::*;
use serde_json::json;

const GIZLI: &str = "GIZLI_DEGER=senaryo-s15-9d2e";

#[tokio::test]
async fn s15_eklenti_gizli_dosyayi_okuyamaz_icerik_modele_gitmez() {
    let d = Duzenek::yeni(&manifest(&["**/*"], &[], 0)).await;
    d.dosya(".env", GIZLI.as_bytes())
        .dosya("BENIOKU.md", b"# Proje\n");
    let mutlak = d.alan.path().join(".env").display().to_string();
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![
            ("kalfa_test__dosya", json!({"islem": "oku", "yol": ".env"})),
            ("kalfa_test__dosya", json!({"islem": "std", "yol": mutlak})),
            (
                "kalfa_test__dosya",
                json!({"islem": "listele", "desen": "**/*"}),
            ),
        ]),
        SahteYanit::metin("Bitti."),
    ]);
    let mut araclar = kalfa_tools::varsayilan_araclar(d.calisma_alani.clone());
    araclar.extend(d.araclar_depoyla(d.depo.path()));
    let baglam = test_baglami(
        s.clone(),
        araclar,
        std::sync::Arc::new(kalfa_core::izin::Siniflandirici::yeni(d.alan.path(), &[]).unwrap()),
    );
    // Serbest kipte de eklenti onaysız çalışır; korumayı izin kartı değil
    // host'un dosya kuralı sağlar.
    let mut ayarlar = OturumAyarlari::yeni("glm-5.3", d.alan.path());
    ayarlar.kip = Kip::Serbest;
    let mut surucu = test_oturumu(baglam, ayarlar);
    surucu
        .gonder(Komut::KullaniciMesaji {
            metin: "Projeyi incele.".into(),
        })
        .await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert!(
        !surucu
            .olaylar()
            .iter()
            .any(|o| matches!(o, Olay::OnayGerekiyor { .. }))
    );

    let istekler = format!("{:?}", s.istekler());
    assert!(
        !istekler.contains("senaryo-s15-9d2e"),
        "gizli içerik modele gitti"
    );
    assert!(istekler.contains("izin-yok: gizli dosya"), "{istekler}");
    assert!(istekler.contains("okunamadı"), "{istekler}");
    assert!(istekler.contains("BENIOKU.md"), "{istekler}");
    assert!(!istekler.contains(".env\\t"), "{istekler}");
}
