//! Açılış ve şart penceresi akışı (1c tasarımı §7). Hiçbir akışta şart
//! kendiliğinden kabul edilmez: sahte sağlayıcının kabul listesi yalnızca
//! `sartlari_kabul_et` çağrılınca dolar.

use std::sync::Arc;
use std::time::Duration;

use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::SahteSaglayici;
use kalfa_sandbox::Yalitimsiz;
use kalfa_uygulama::anahtar::{AnahtarDeposu, AnahtarKasasi};
use kalfa_uygulama::sozlesme::{
    AnahtarNedeni, AnahtarSonucu, BaglantiDurumu, DepoTuru, KabulYaniti, SartPenceresiDurumu,
};
use kalfa_uygulama::test_destek::{
    AGSIZ_ANAHTAR, GECERSIZ_ANAHTAR, SahteAcici, SahteIletisim, SahtePencereler, SahteUretici,
    TestUygulamasi, YazilamayanDepo, test_uygulamasi,
};
use kalfa_uygulama::yalitim::YalitimKaynagi;
use kalfa_uygulama::{Bilesenler, Uygulama};

async fn bekle(mut kosul: impl FnMut() -> bool) {
    for _ in 0..200 {
        if kosul() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("koşul 2 saniyede gerçekleşmedi");
}

fn gerekli_surum(durum: &SartPenceresiDurumu) -> Option<u32> {
    match durum {
        SartPenceresiDurumu::SartGerekli { metin } => Some(metin.surum),
        _ => None,
    }
}

async fn acilmis(
    dizin: &std::path::Path,
    anahtar: Option<&str>,
    kabul_edilmis: bool,
) -> TestUygulamasi {
    let saglayici = SahteSaglayici::yeni(Vec::new());
    if kabul_edilmis {
        saglayici.sart_kabul(1).await.unwrap();
    }
    let t = test_uygulamasi(dizin, saglayici, anahtar);
    t.uygulama.baslat().await;
    t
}

#[tokio::test]
async fn anahtar_yoksa_anahtar_formu_acilir() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), None, false).await;
    assert_eq!(
        t.uygulama.sart_penceresi_durumu(),
        SartPenceresiDurumu::AnahtarGerekli {
            neden: AnahtarNedeni::Yok
        }
    );
    assert_eq!(t.pencereler.olaylar(), ["sartlar_goster"]);
}

#[tokio::test]
async fn sartlar_kabul_edilmisse_ana_pencere_acilir() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), true).await;
    assert_eq!(t.pencereler.olaylar(), ["ana_ac"]);
    assert_eq!(t.uygulama.uygulama_durumu().baglanti, BaglantiDurumu::Var);
    assert_eq!(
        t.saglayici.kabul_edilen_surumler(),
        [1],
        "yalnızca kurulumdaki kabul"
    );
}

#[tokio::test]
async fn kabul_gerekiyorsa_sart_penceresi_acilir_kabul_gonderilmez() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), false).await;
    bekle(|| t.pencereler.olaylar().contains(&"sartlar_goster")).await;
    assert_eq!(gerekli_surum(&t.uygulama.sart_penceresi_durumu()), Some(1));
    assert!(!t.pencereler.olaylar().contains(&"ana_ac"));
    assert!(t.saglayici.kabul_edilen_surumler().is_empty());
}

#[tokio::test]
async fn kayitli_anahtar_gecersizse_anahtar_formu() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some(GECERSIZ_ANAHTAR), false).await;
    assert_eq!(
        t.uygulama.sart_penceresi_durumu(),
        SartPenceresiDurumu::AnahtarGerekli {
            neden: AnahtarNedeni::KayitliGecersiz
        }
    );
    assert_eq!(t.pencereler.olaylar(), ["sartlar_goster"]);
}

#[tokio::test]
async fn evrene_ulasilamazsa_ana_pencere_baglantisiz_acilir() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some(AGSIZ_ANAHTAR), false).await;
    assert_eq!(t.pencereler.olaylar(), ["ana_ac"]);
    assert_eq!(t.uygulama.uygulama_durumu().baglanti, BaglantiDurumu::Yok);
}

#[tokio::test]
async fn gecersiz_ya_da_ulasilamayan_anahtar_saklanmaz() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), None, false).await;
    assert_eq!(
        t.uygulama
            .anahtari_kaydet(GECERSIZ_ANAHTAR.into())
            .await
            .unwrap(),
        AnahtarSonucu::Gecersiz
    );
    assert_eq!(
        t.uygulama
            .anahtari_kaydet(AGSIZ_ANAHTAR.into())
            .await
            .unwrap(),
        AnahtarSonucu::Ulasilamadi
    );
    assert_eq!(
        t.uygulama.anahtari_kaydet("   ".into()).await.unwrap(),
        AnahtarSonucu::Gecersiz
    );
    assert_eq!(t.depo.oku().unwrap(), None);
}

#[tokio::test]
async fn gecerli_anahtar_saklanir_ve_sart_metni_doner() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), None, false).await;
    let sonuc = t
        .uygulama
        .anahtari_kaydet(" evren_llm_a ".into())
        .await
        .unwrap();
    let AnahtarSonucu::Kaydedildi { depo, sonraki } = sonuc else {
        panic!("anahtar kaydedilmeliydi: {sonuc:?}");
    };
    assert_eq!(depo, DepoTuru::Kalici);
    assert_eq!(gerekli_surum(&sonraki), Some(1));
    assert_eq!(t.depo.oku().unwrap().as_deref(), Some("evren_llm_a"));
    tokio::time::sleep(Duration::from_millis(50)).await;
    // Pencere komutu sürerken kapı izleyicisi pencereyi yeniden yüklemez.
    assert_eq!(t.pencereler.olaylar(), ["sartlar_goster"]);
    assert!(t.saglayici.kabul_edilen_surumler().is_empty());
}

#[tokio::test]
async fn kabul_edilmis_hesabin_anahtari_ana_pencereyi_acar() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), None, false).await;
    t.saglayici.sart_kabul(1).await.unwrap();
    t.pencereler.temizle();
    let sonuc = t
        .uygulama
        .anahtari_kaydet("evren_llm_a".into())
        .await
        .unwrap();
    assert!(matches!(
        sonuc,
        AnahtarSonucu::Kaydedildi {
            sonraki: SartPenceresiDurumu::Tamam,
            ..
        }
    ));
    assert_eq!(t.pencereler.olaylar(), ["ana_ac", "sartlar_kapat"]);
}

#[tokio::test]
async fn kimlik_deposu_yazilamazsa_anahtar_bellekte_tutulur() {
    let d = tempfile::tempdir().unwrap();
    let saglayici = SahteSaglayici::yeni(Vec::new());
    let uygulama = Uygulama::yeni(Bilesenler {
        veri_dizini: d.path().to_path_buf(),
        kasa: AnahtarKasasi::yeni(Some(Box::new(YazilamayanDepo))),
        uretici: Arc::new(SahteUretici { saglayici }),
        iletisim: Arc::new(SahteIletisim::default()),
        acici: Arc::new(SahteAcici::default()),
        pencereler: Arc::new(SahtePencereler::default()),
        yukseltici: Arc::new(kalfa_uygulama::test_destek::SahteYukseltici::default()),
        aktarim: Default::default(),
        yalitim_dizini: d.path().join("yalitim"),
        ev_dizini: std::env::temp_dir(),
        yalitim: YalitimKaynagi::Kurucu(Box::new(|_| Arc::new(Yalitimsiz))),
    });
    uygulama.baslat().await;
    let sonuc = uygulama
        .anahtari_kaydet("evren_llm_a".into())
        .await
        .unwrap();
    assert!(matches!(
        sonuc,
        AnahtarSonucu::Kaydedildi {
            depo: DepoTuru::Bellekte,
            ..
        }
    ));
    assert_eq!(
        uygulama.uygulama_durumu().anahtar_deposu,
        DepoTuru::Bellekte
    );
}

#[tokio::test]
async fn kabul_bir_kez_gider_ana_pencere_acilir_sart_penceresi_kapanir() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), false).await;
    bekle(|| t.pencereler.olaylar().contains(&"sartlar_goster")).await;
    assert_eq!(
        t.uygulama.sartlari_kabul_et(1).await.unwrap(),
        KabulYaniti::Kabul
    );
    assert_eq!(t.saglayici.kabul_edilen_surumler(), [1]);
    assert_eq!(
        t.uygulama.sart_penceresi_durumu(),
        SartPenceresiDurumu::Tamam
    );
    let olaylar = t.pencereler.olaylar();
    assert_eq!(&olaylar[olaylar.len() - 2..], ["ana_ac", "sartlar_kapat"]);
}

#[tokio::test]
async fn ikinci_tiklama_kabulu_yinelemez_sonraki_ret_kilitlemez() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), false).await;
    bekle(|| t.pencereler.olaylar().contains(&"sartlar_goster")).await;
    let (a, b) = tokio::join!(
        t.uygulama.sartlari_kabul_et(1),
        t.uygulama.sartlari_kabul_et(1)
    );
    assert_eq!(a.unwrap(), KabulYaniti::Kabul);
    assert_eq!(b.unwrap(), KabulYaniti::Kabul);
    assert_eq!(t.saglayici.kabul_edilen_surumler(), [1]);
    // Kabulden sonra gelen eski "Kabul etmiyorum" tıklaması bir şey değiştirmez.
    t.uygulama.sartlari_reddet();
    assert_eq!(
        t.uygulama.sart_penceresi_durumu(),
        SartPenceresiDurumu::Tamam
    );
}

#[tokio::test]
async fn retten_sonra_eski_kabul_tiklamasi_kabul_gondermez() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), false).await;
    bekle(|| t.pencereler.olaylar().contains(&"sartlar_goster")).await;
    t.uygulama.sartlari_reddet();
    assert_eq!(
        t.uygulama.sartlari_kabul_et(1).await.unwrap(),
        KabulYaniti::Kilitli
    );
    assert!(t.saglayici.kabul_edilen_surumler().is_empty());
    assert_eq!(
        t.uygulama.sart_penceresi_durumu(),
        SartPenceresiDurumu::Kilitli
    );
}

#[tokio::test]
async fn gosterilenden_farkli_surum_kabul_edilmez() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), false).await;
    bekle(|| t.pencereler.olaylar().contains(&"sartlar_goster")).await;
    let yanit = t.uygulama.sartlari_kabul_et(2).await.unwrap();
    assert!(matches!(yanit, KabulYaniti::YeniMetin { metin } if metin.surum == 1));
    assert!(t.saglayici.kabul_edilen_surumler().is_empty());
}

#[tokio::test]
async fn okurken_surum_degisirse_yeni_metin_gelir() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), false).await;
    bekle(|| t.pencereler.olaylar().contains(&"sartlar_goster")).await;
    t.saglayici.sart_surumunu_degistir(2);
    let yanit = t.uygulama.sartlari_kabul_et(1).await.unwrap();
    assert!(matches!(yanit, KabulYaniti::YeniMetin { metin } if metin.surum == 2));
    assert!(t.saglayici.kabul_edilen_surumler().is_empty());
    assert_eq!(gerekli_surum(&t.uygulama.sart_penceresi_durumu()), Some(2));
}

#[tokio::test]
async fn ret_kilitli_gorunume_gecer_metin_yeniden_gosterilebilir() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), false).await;
    bekle(|| t.pencereler.olaylar().contains(&"sartlar_goster")).await;
    t.uygulama.sartlari_reddet();
    assert_eq!(
        t.uygulama.sart_penceresi_durumu(),
        SartPenceresiDurumu::Kilitli
    );
    let durum = t.uygulama.sart_metnini_goster().await.unwrap();
    assert_eq!(gerekli_surum(&durum), Some(1));
    assert_eq!(gerekli_surum(&t.uygulama.sart_penceresi_durumu()), Some(1));
    assert!(t.saglayici.kabul_edilen_surumler().is_empty());
}

#[tokio::test]
async fn anahtar_penceresi_sart_beklerken_sart_metnini_gosterir() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), false).await;
    bekle(|| t.pencereler.olaylar().contains(&"sartlar_goster")).await;
    t.uygulama.anahtar_penceresini_ac();
    assert_eq!(gerekli_surum(&t.uygulama.sart_penceresi_durumu()), Some(1));

    let d2 = tempfile::tempdir().unwrap();
    let t2 = acilmis(d2.path(), Some("evren_llm_a"), true).await;
    t2.uygulama.anahtar_penceresini_ac();
    assert_eq!(
        t2.uygulama.sart_penceresi_durumu(),
        SartPenceresiDurumu::AnahtarGerekli {
            neden: AnahtarNedeni::Degistir
        }
    );
    assert_eq!(t2.pencereler.olaylar(), ["ana_ac", "sartlar_goster"]);
}

#[tokio::test]
async fn sart_penceresi_yalnizca_bekleyen_ya_da_kilitli_durumda_yeniden_acilir() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), true).await;
    t.uygulama.sart_penceresini_ac();
    assert_eq!(t.pencereler.olaylar(), ["ana_ac"]);
    // Bekleyen metin yokken verilen ret kilitlemez.
    t.uygulama.sartlari_reddet();
    t.uygulama.sart_penceresini_ac();
    assert_eq!(t.pencereler.olaylar(), ["ana_ac"]);

    let d2 = tempfile::tempdir().unwrap();
    let t2 = acilmis(d2.path(), Some("evren_llm_a"), false).await;
    bekle(|| t2.pencereler.olaylar().contains(&"sartlar_goster")).await;
    t2.uygulama.sartlari_reddet();
    t2.pencereler.temizle();
    t2.uygulama.sart_penceresini_ac();
    assert_eq!(t2.pencereler.olaylar(), ["sartlar_goster"]);
}
