//! `kalfa-rag`'ın bütünleşme testleri (aşama 4 tasarımı §16.4): derlenmiş
//! eklenti, gerçek host (`kalfa-plugin`) ve dosyalar; EVREN yerine metnin
//! özetinden belirlenimli vektör üreten sahte sağlayıcı.

mod ortak;

use std::time::Duration;

use kalfa_core::test_destek::SahteSaglayici;
use kalfa_evren::{EkUc, EvrenError};
use ortak::*;

fn yaz(kok: &std::path::Path, yol: &str, icerik: &str) {
    let tam = kok.join(yol);
    std::fs::create_dir_all(tam.parent().unwrap()).unwrap();
    std::fs::write(tam, icerik).unwrap();
}

/// Bir gömme isteğinin girdi sayısı.
fn girdi_sayisi(govde: &serde_json::Value) -> usize {
    govde["input"].as_array().map_or(1, Vec::len)
}

fn gommeler(s: &SahteSaglayici) -> Vec<serde_json::Value> {
    s.ek_istekleri()
        .into_iter()
        .filter(|(uc, _)| *uc == EkUc::Gomme)
        .map(|(_, g)| g)
        .collect()
}

#[tokio::test]
async fn tek_dosyanin_299_parcasi_iki_toplu_istekle_indekslenir() {
    let alan = tempfile::tempdir().unwrap();
    let depo = tempfile::tempdir().unwrap();
    let metin: String = (0..300)
        .map(|i| format!("{i:04}{}\n", "a".repeat(844)))
        .collect();
    assert!(metin.len() < 256 * 1024);
    yaz(alan.path(), "uzun.md", &metin);
    let s = SahteSaglayici::yeni(vec![]);
    let arac = arac(alan.path(), depo.path(), s.clone()).await;
    let cikti = ara(&arac, "uzun belge", 2).await.unwrap();
    assert!(!cikti.contains("İndeks eksik"), "{cikti}");
    assert!(cikti.contains("indeks: 1 dosya"), "{cikti}");
    assert_eq!(sonuc_yollari(&cikti), ["uzun.md", "uzun.md"]);
    assert_eq!(
        gommeler(&s).iter().map(girdi_sayisi).collect::<Vec<_>>(),
        [1, 256, 43]
    );
}

#[tokio::test]
async fn suren_hiz_ve_gunluk_sinirda_kaynakli_kismi_sonuclar_doner() {
    for hata in [
        EvrenError::HizSiniri {
            tekrar_dene: Some(Duration::from_secs(9)),
        },
        EvrenError::GunlukSinir { sifirlanma: None },
    ] {
        let alan = tempfile::tempdir().unwrap();
        let depo = tempfile::tempdir().unwrap();
        for i in 0..3 {
            let metin: String = (0..10_000)
                .map(|n| format!("dosya{i} satır {n}\n"))
                .collect();
            yaz(alan.path(), &format!("d{i}.md"), &metin);
        }
        let s = SahteSaglayici::yeni(vec![]);
        let arac = arac(alan.path(), depo.path(), s.clone()).await;
        // Sorgu (1) ve ilk dosya (2) gömülür. Sonraki indeksleme (3)
        // ve yeniden sıralama (4) aynı devam eden sınıra çarpar.
        s.ek_hata_sirada(3, hata.clone());
        s.ek_hata_sirada(4, hata);
        let cikti = ara(&arac, "satır", 3).await.unwrap();
        assert!(
            cikti.contains("İndeks eksik: 3 dosyanın 1 tanesi indekslendi."),
            "{cikti}"
        );
        assert!(
            cikti.contains("sonuçlar kosinüs benzerliğiyle sıralandı"),
            "{cikti}"
        );
        assert!(!sonuc_yollari(&cikti).is_empty(), "{cikti}");
        assert!(sonuc_yollari(&cikti).iter().all(|y| y == "d0.md"));
        assert_eq!(s.ek_istekleri().len(), 4, "sınıra yeniden deneme yapılmaz");
    }
}

#[tokio::test]
async fn ilk_arama_indeksler_ve_kaynakli_sonuc_verir() {
    let alan = tempfile::tempdir().unwrap();
    let depo = tempfile::tempdir().unwrap();
    yaz(
        alan.path(),
        "src/kurtarma.rs",
        "// Kayıttan oturum kurtarma.\nfn kurtar() {}\n",
    );
    yaz(
        alan.path(),
        "src/kabuk.rs",
        "// Kabuk aracı yalıtımda çalışır.\nfn calistir() {}\n",
    );
    yaz(alan.path(), "BENIOKU.md", "# Proje\nKalfa bir ajan.\n");
    yaz(alan.path(), ".env", "GIZLI=rag-sizinti-7f3a\n");
    yaz(alan.path(), "Cargo.lock", "kilit dosyası\n");
    let s = SahteSaglayici::yeni(vec![]);
    let arac = arac(alan.path(), depo.path(), s.clone()).await;

    let cikti = ara(&arac, "oturum kurtarma", 2).await.unwrap();
    assert!(cikti.starts_with("\"oturum kurtarma\" için"), "{cikti}");
    assert_eq!(sonuc_yollari(&cikti)[0], "src/kurtarma.rs", "{cikti}");
    assert!(cikti.contains("dosya:src/kurtarma.rs:1-2"), "{cikti}");

    // Sorgu gömmesi + toplu indeksleme; ardından yeniden sıralama.
    let istekler = s.ek_istekleri();
    let g = gommeler(&s);
    assert_eq!(g.len(), 2, "{istekler:?}");
    assert_eq!(girdi_sayisi(&g[1]), 3, "üç dosya, birer parça: {:?}", g[1]);
    assert_eq!(g[0]["encoding_format"], "base64");
    assert_eq!(g[0]["dimensions"], 1024);
    assert!(g[0]["input"][0].as_str().unwrap().starts_with("Instruct:"));
    assert_eq!(istekler.last().unwrap().0, EkUc::Siralama);
    // Gizli dosya ve kilit dosyası EVREN'e hiç gitmez.
    let hepsi = format!("{istekler:?}");
    assert!(!hepsi.contains("rag-sizinti-7f3a"));
    assert!(!hepsi.contains("kilit dosyası"));
}

#[tokio::test]
async fn degisen_dosya_yeniden_silinen_dosya_sonuclardan_duser() {
    let alan = tempfile::tempdir().unwrap();
    let depo = tempfile::tempdir().unwrap();
    yaz(alan.path(), "a.md", "elma armut\n");
    yaz(alan.path(), "b.md", "kiraz vişne\n");
    yaz(alan.path(), "c.md", "muz ananas\n");
    let s = SahteSaglayici::yeni(vec![]);
    let arac = arac(alan.path(), depo.path(), s.clone()).await;
    ara(&arac, "kiraz", 3).await.unwrap();
    assert_eq!(girdi_sayisi(&gommeler(&s)[1]), 3);

    // Değişiklik yokken yalnızca sorgu gömülür.
    ara(&arac, "kiraz", 3).await.unwrap();
    assert_eq!(gommeler(&s).len(), 3);

    // "Racy clean" penceresini geçmek için zaman damgası belirgin değişsin.
    tokio::time::sleep(Duration::from_millis(2_100)).await;
    yaz(alan.path(), "b.md", "kiraz vişne çilek dut\n");
    std::fs::remove_file(alan.path().join("c.md")).unwrap();
    let cikti = ara(&arac, "çilek", 3).await.unwrap();
    let g = gommeler(&s);
    assert_eq!(g.len(), 5, "değişen dosya için bir toplu istek");
    assert_eq!(girdi_sayisi(&g[4]), 1, "yalnızca değişen dosya: {:?}", g[4]);
    assert!(g[4]["input"][0].as_str().unwrap().contains("çilek"));
    let yollar = sonuc_yollari(&cikti);
    assert_eq!(yollar[0], "b.md", "{cikti}");
    assert!(!yollar.contains(&"c.md".to_string()), "{cikti}");
}

#[tokio::test]
async fn hiz_sinirinda_eldeki_indeksle_arar_sonraki_cagri_tamamlar() {
    let alan = tempfile::tempdir().unwrap();
    let depo = tempfile::tempdir().unwrap();
    // Beş ~200 KB'lık dosya: toplu istek en çok 256 parça olduğu için birkaç
    // isteğe bölünür.
    for i in 0..5 {
        let satir = format!(
            "dosya{i} {}
",
            "kelime ".repeat(40)
        );
        yaz(alan.path(), &format!("d{i}.md"), &satir.repeat(700));
    }
    let s = SahteSaglayici::yeni(vec![]);
    let arac = arac(alan.path(), depo.path(), s.clone()).await;
    // İkinci toplu istek 429 alır: indeksleme durur, eldekiyle arar.
    s.ek_hata_sirada(
        3,
        EvrenError::HizSiniri {
            tekrar_dene: Some(Duration::from_secs(9)),
        },
    );
    let cikti = ara(&arac, "kelime", 3).await.unwrap();
    assert!(cikti.contains("İndeks eksik"), "{cikti}");
    assert!(cikti.contains("9 sn"), "{cikti}");
    assert!(!sonuc_yollari(&cikti).is_empty(), "{cikti}");
    let ilk_tur = gommeler(&s).len();

    // Sonraki çağrı kaldığı yerden sürer: biten toplu istek tekrarlanmaz.
    let cikti = ara(&arac, "kelime", 3).await.unwrap();
    assert!(!cikti.contains("İndeks eksik"), "{cikti}");
    let ikinci: Vec<_> = gommeler(&s)[ilk_tur..].to_vec();
    let ikinci_parca: usize = ikinci.iter().map(girdi_sayisi).sum();
    let ilk_parca = girdi_sayisi(&gommeler(&s)[1]);
    assert!(ilk_parca > 1);
    let toplam_parca: usize = gommeler(&s).iter().map(girdi_sayisi).sum();
    assert!(
        ikinci_parca < toplam_parca - 1,
        "ilk toplu istek yeniden gömülmemeli"
    );
}
