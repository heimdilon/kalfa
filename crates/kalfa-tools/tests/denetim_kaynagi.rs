use std::sync::Arc;

use kalfa_core::arac::{
    Arac, AracCiktisi, AracHatasi, DenetimGereksinimi, Hazirlik, denetim_durumu,
};
use kalfa_core::izin::{GIZLI_UYARISI, Siniflandirici};
use kalfa_tools::{CalismaAlani, DosyaDuzenle, DosyaListele, DosyaOku, DosyaYaz, IcerikAra};
use serde_json::json;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

fn alan() -> (TempDir, Arc<CalismaAlani>) {
    let d = tempfile::tempdir().unwrap();
    let s = Arc::new(Siniflandirici::yeni(d.path(), &[]).unwrap());
    let a = CalismaAlani::ac(s).unwrap();
    (d, a)
}

fn dosya(d: &TempDir, yol: &str, icerik: &[u8]) {
    let p = d.path().join(yol);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, icerik).unwrap();
}

async fn calistir_oku(arac: &dyn Arac, args: serde_json::Value) -> Result<AracCiktisi, AracHatasi> {
    let h = arac.hazirla(&args).await?;
    arac.calistir(&args, &h, CancellationToken::new()).await
}

async fn calistir_yaz(
    arac: &dyn Arac,
    args: serde_json::Value,
) -> Result<(AracCiktisi, Hazirlik), AracHatasi> {
    let h = arac.hazirla(&args).await?;
    let c = arac.calistir(&args, &h, CancellationToken::new()).await?;
    Ok((c, h))
}

#[tokio::test]
async fn yazma_iletisi_kalfa_uretimi() {
    let (_d, a) = alan();

    // 1. dosya_yaz
    let yaz = DosyaYaz::yeni(a.clone());
    let (c_yaz, _) = calistir_yaz(
        &yaz,
        json!({
            "yol": "yeni.txt",
            "icerik": "deneme"
        }),
    )
    .await
    .unwrap();
    assert!(c_yaz.kalfa_uretimi);
    assert_eq!(
        denetim_durumu(&c_yaz, &[], 65536),
        DenetimGereksinimi::KalfaUretimi
    );

    // 2. dosya_duzenle
    let duzenle = DosyaDuzenle::yeni(a.clone());
    let (c_duzenle, _) = calistir_yaz(
        &duzenle,
        json!({
            "yol": "yeni.txt",
            "eski_metin": "deneme",
            "yeni_metin": "degisti"
        }),
    )
    .await
    .unwrap();
    assert!(c_duzenle.kalfa_uretimi);
    assert_eq!(
        denetim_durumu(&c_duzenle, &[], 65536),
        DenetimGereksinimi::KalfaUretimi
    );
}

#[tokio::test]
async fn okuma_ve_kabuk_dis_kaynakli() {
    let (d, a) = alan();
    dosya(&d, "ornek.txt", b"bir icerik");

    // dosya_oku
    let oku = DosyaOku::yeni(a.clone());
    let c_oku = calistir_oku(&oku, json!({"yol": "ornek.txt"}))
        .await
        .unwrap();
    assert!(!c_oku.kalfa_uretimi);
    assert!(matches!(
        denetim_durumu(&c_oku, &[], 65536),
        DenetimGereksinimi::Denetlenecek(_)
    ));

    // icerik_ara
    let ara = IcerikAra::yeni(a.clone());
    let c_ara = calistir_oku(&ara, json!({"desen": "icerik"}))
        .await
        .unwrap();
    assert!(!c_ara.kalfa_uretimi);
    assert!(matches!(
        denetim_durumu(&c_ara, &[], 65536),
        DenetimGereksinimi::Denetlenecek(_)
    ));

    // dosya_listele
    let listele = DosyaListele::yeni(a.clone());
    let c_listele = calistir_oku(&listele, json!({"desen": "*"})).await.unwrap();
    assert!(!c_listele.kalfa_uretimi);
    assert!(matches!(
        denetim_durumu(&c_listele, &[], 65536),
        DenetimGereksinimi::Denetlenecek(_)
    ));

    // kabuk çıktısı temsili
    let c_kabuk = AracCiktisi {
        icerik: "komut ciktisi\n".into(),
        kaynak: "kabuk:echo".into(),
        guvenlik_uyarilari: vec![],
        degisiklikler: None,
        kalfa_uretimi: false,
        ag: None,
    };
    assert!(!c_kabuk.kalfa_uretimi);
    assert!(matches!(
        denetim_durumu(&c_kabuk, &[], 65536),
        DenetimGereksinimi::Denetlenecek(_)
    ));
}

#[tokio::test]
async fn onaylanan_gizli_okuma_denetciye_gitmez() {
    let cikti = AracCiktisi {
        icerik: "API_KEY=gizli123".into(),
        kaynak: "dosya:.env:1-1".into(),
        guvenlik_uyarilari: vec![],
        degisiklikler: None,
        kalfa_uretimi: false,
        ag: None,
    };

    let uyarilar = vec![GIZLI_UYARISI.to_string()];
    let durum = denetim_durumu(&cikti, &uyarilar, 65536);
    assert_eq!(durum, DenetimGereksinimi::GizliDosya);
}

#[tokio::test]
async fn denetlenen_metin_kirpilmis_hal() {
    let uzun_icerik = "A".repeat(100_000);
    let cikti = AracCiktisi {
        icerik: uzun_icerik,
        kaynak: "dosya:buyuk.txt:1-1000".into(),
        guvenlik_uyarilari: vec![],
        degisiklikler: None,
        kalfa_uretimi: false,
        ag: None,
    };

    let durum = denetim_durumu(&cikti, &[], 1000);
    match durum {
        DenetimGereksinimi::Denetlenecek(girdi) => {
            assert_eq!(girdi.kaynak, "dosya:buyuk.txt:1-1000");
            assert!(
                girdi
                    .icerik
                    .contains("[çıktı 100000 bayttan 1000 bayta kırpıldı]")
            );
            // Sarmal etiketleri denetlenen metinde olmamalıdır (sarmaldan önceki hâl)
            assert!(!girdi.icerik.contains("<arac_sonucu"));
        }
        diger => panic!("Denetlenecek bekleniyordu, geldi: {diger:?}"),
    }
}
