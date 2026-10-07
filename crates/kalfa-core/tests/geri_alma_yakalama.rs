//! Yazmadan önce önceki hâlin yakalanması (aşama 3 tasarımı §5.3).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kalfa_core::arac::Arac;
use kalfa_core::gecmis::gorunum_uret;
use kalfa_core::geri_alma::{DOSYA_SINIRI, DosyaYedegi, OncekiHal, OncekiHalDeposu};
use kalfa_core::izin::{GERI_ALINAMAZ_UYARISI, Kip};
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::{CagriId, OturumId};
use kalfa_core::muhur::ozet_hex;
use kalfa_core::oturum::{OturumAyarlari, devam_et};
use kalfa_core::protokol::{AracDurumu, BitisNedeni, Komut, Olay, OnayKarari, SEMA_SURUMU, Zarf};
use kalfa_core::test_destek::*;
use serde_json::json;

struct Ortam {
    d: tempfile::TempDir,
}

impl Ortam {
    fn yeni() -> Self {
        Self {
            d: tempfile::tempdir().unwrap(),
        }
    }

    fn kayit(&self) -> PathBuf {
        self.d.path().join("kayitlar/oturum.jsonl")
    }

    fn depo(&self) -> PathBuf {
        self.d.path().join("geri-alma/oturum")
    }

    fn ayarlar(&self, kip: Kip, kayitli: bool) -> OturumAyarlari {
        let mut a = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
        a.kip = kip;
        if kayitli {
            a.kayit_yolu = Some(self.kayit());
        }
        a.geri_alma_dizini = Some(self.depo());
        a
    }

    fn surucu(&self, arac: Arc<SahteArac>, yol: &str, kip: Kip, kayitli: bool) -> Surucu {
        let s = SahteSaglayici::yeni(vec![
            SahteYanit::arac("yaz", json!({"yol": yol})),
            SahteYanit::metin("tamam"),
        ]);
        test_oturumu(
            test_baglami(s, vec![arac as Arc<dyn Arac>], sanal_siniflandirici()),
            self.ayarlar(kip, kayitli),
        )
    }

    fn yedekler(&self) -> Vec<DosyaYedegi> {
        OturumKaydi::oku(&self.kayit())
            .unwrap()
            .into_iter()
            .flat_map(|z| match z.yuk {
                KayitOgesi::AracBasladi { yedekler, .. } => yedekler,
                _ => Vec::new(),
            })
            .collect()
    }
}

fn mesaj() -> Komut {
    Komut::KullaniciMesaji {
        metin: "yaz".into(),
    }
}

fn arac_sonucu(d: &Surucu) -> (AracDurumu, String) {
    d.gecmis
        .iter()
        .find_map(|z| match &z.yuk {
            Olay::AracSonucu { durum, icerik, .. } => Some((*durum, icerik.clone())),
            _ => None,
        })
        .expect("araç sonucu")
}

#[tokio::test]
async fn yazmadan_once_onceki_hal_depoda_ve_kayitta() {
    let o = Ortam::yeni();
    let yaz = Arc::new(SahteArac::yazan("yaz").onceki_icerikli());
    let mut d = o.surucu(yaz.clone(), "a.txt", Kip::Proje, true);
    d.gonder(mesaj()).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    d.kapat().await;

    let ozet = ozet_hex(b"surum-0");
    let yedekler = o.yedekler();
    assert_eq!(yedekler.len(), 1, "{yedekler:?}");
    let y = &yedekler[0];
    assert_eq!(y.yol.as_str(), "a.txt");
    assert_eq!(y.onceki, OncekiHal::Saklandi { ozet: ozet.clone() });
    assert_eq!(
        y.sonraki.as_deref(),
        Some(ozet_hex(yaz.sonraki_icerik().as_bytes()).as_str())
    );
    assert_eq!((y.eklenen, y.silinen), (1, 0));
    assert_eq!(y.diff.as_deref(), Some("+sahte yaz"));

    let depo = OncekiHalDeposu::ac(&o.depo()).unwrap();
    assert_eq!(depo.oku(&ozet).unwrap(), b"surum-0");
    assert_eq!(yaz.cagri_sayisi(), 1);
}

#[tokio::test]
async fn yeni_dosya_yoktu_olarak_kaydedilir() {
    let o = Ortam::yeni();
    let yaz = Arc::new(SahteArac::yazan("yaz").onceki_yok());
    let mut d = o.surucu(yaz, "yeni.txt", Kip::Proje, true);
    d.gonder(mesaj()).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    d.kapat().await;

    let yedekler = o.yedekler();
    assert_eq!(yedekler[0].onceki, OncekiHal::Yoktu);
    assert!(yedekler[0].sonraki.is_some());
    let depo = OncekiHalDeposu::ac(&o.depo()).unwrap();
    assert_eq!(depo.boyut().unwrap(), 0);
}

#[tokio::test]
async fn depo_yazilamazsa_arac_calismaz() {
    let o = Ortam::yeni();
    let yaz = Arc::new(SahteArac::yazan("yaz").onceki_icerikli());
    let mut d = o.surucu(yaz.clone(), "a.txt", Kip::Proje, true);
    // Oturum açılırken kurulan depo dizini bir dosyayla değişir.
    std::fs::remove_dir(o.depo()).unwrap();
    std::fs::write(o.depo(), b"dosya").unwrap();
    d.gonder(mesaj()).await;
    d.tur_sonu().await;
    let (durum, icerik) = arac_sonucu(&d);
    assert_eq!(durum, AracDurumu::Basarisiz);
    assert!(
        icerik.contains("[çalıştırılmadı: önceki hâl saklanamadı"),
        "{icerik}"
    );
    assert_eq!(yaz.cagri_sayisi(), 0, "araç çalışmadı");
    d.kapat().await;
    assert!(o.yedekler().is_empty(), "başladı kaydı yazılmadı");
}

#[tokio::test]
async fn proje_kipinde_buyuk_dosya_yazmasi_onay_ister() {
    let o = Ortam::yeni();
    let yaz = Arc::new(SahteArac::yazan("yaz").onceki_buyuk());
    let mut d = o.surucu(yaz.clone(), "buyuk.bin", Kip::Proje, true);
    d.gonder(mesaj()).await;
    let ogeler = match d
        .bekle(|o| matches!(o, Olay::OnayGerekiyor { .. }))
        .await
        .yuk
    {
        Olay::OnayGerekiyor { ogeler, .. } => ogeler,
        _ => unreachable!(),
    };
    assert!(
        ogeler[0]
            .uyarilar
            .iter()
            .any(|u| u == GERI_ALINAMAZ_UYARISI),
        "{:?}",
        ogeler[0].uyarilar
    );
    d.gonder(Komut::AracOnayi {
        cagri: ogeler[0].cagri,
        karar: OnayKarari::Onayla,
        muhur: ogeler[0].muhur.clone(),
        kisayol: false,
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    d.kapat().await;
    assert_eq!(yaz.cagri_sayisi(), 1);
    let yedekler = o.yedekler();
    assert!(
        matches!(&yedekler[0].onceki, OncekiHal::Buyuk { bayt, .. } if *bayt == DOSYA_SINIRI + 1),
        "{yedekler:?}"
    );
}

#[tokio::test]
async fn serbest_kipte_de_buyuk_dosya_onay_ister() {
    let o = Ortam::yeni();
    let yaz = Arc::new(SahteArac::yazan("yaz").onceki_buyuk());
    let mut d = o.surucu(yaz.clone(), "buyuk.bin", Kip::Serbest, true);
    d.gonder(mesaj()).await;
    d.bekle(|o| matches!(o, Olay::OnayGerekiyor { .. })).await;
    d.gonder(Komut::Iptal {
        tur: d.tutamac.aktif_tur().unwrap(),
    })
    .await;
    d.tur_sonu().await;
    assert_eq!(yaz.cagri_sayisi(), 0);
}

#[tokio::test]
async fn kayitsiz_oturumda_yedek_yok_ve_buyuk_dosya_onaysiz() {
    let o = Ortam::yeni();
    let yaz = Arc::new(SahteArac::yazan("yaz").onceki_buyuk());
    let mut d = o.surucu(yaz.clone(), "buyuk.bin", Kip::Proje, false);
    d.gonder(mesaj()).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert!(
        !d.gecmis
            .iter()
            .any(|z| matches!(z.yuk, Olay::OnayGerekiyor { .. })),
        "kayıtsız oturumda geri alma yok; büyük dosya kuralı işlemez"
    );
    assert_eq!(yaz.cagri_sayisi(), 1);
    d.kapat().await;
    assert!(!o.depo().exists(), "kayıtsız oturumda depo açılmaz");
}

#[tokio::test]
async fn gizli_dosyanin_diffi_kayda_yazilmaz() {
    let o = Ortam::yeni();
    let yaz = Arc::new(SahteArac::yazan("yaz").onceki_icerikli());
    let mut d = o.surucu(yaz, ".env", Kip::Proje, true);
    d.gonder(mesaj()).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    d.kapat().await;
    let yedekler = o.yedekler();
    assert_eq!(yedekler[0].yol.as_str(), ".env");
    assert_eq!(yedekler[0].diff, None, "gizli dosyanın diff'i kayda girmez");
    let ozet = ozet_hex(b"surum-0");
    assert_eq!(
        yedekler[0].onceki,
        OncekiHal::Saklandi { ozet: ozet.clone() }
    );
    let depo = OncekiHalDeposu::ac(&o.depo()).unwrap();
    assert!(depo.var_mi(&ozet), "önceki hâli yine saklanır (K8)");
}

fn zarf(oturum: OturumId, sira: u64, yuk: KayitOgesi) -> Zarf<KayitOgesi> {
    Zarf {
        sema: SEMA_SURUMU,
        oturum,
        tur: None,
        sira,
        yuk,
    }
}

#[tokio::test]
async fn devam_ederken_kaydi_olmayan_depo_dosyalari_silinir() {
    let o = Ortam::yeni();
    let oturum = OturumId::yeni();
    let kalan = ozet_hex(b"kalan");
    let giden = ozet_hex(b"giden");
    {
        std::fs::create_dir_all(o.kayit().parent().unwrap()).unwrap();
        let mut k = OturumKaydi::olustur(&o.kayit()).unwrap();
        k.yaz(&zarf(
            oturum,
            1,
            KayitOgesi::OturumAcildi {
                model: "glm-5.3".into(),
                kip: Kip::Proje,
                calisma_alani: None,
            },
        ))
        .unwrap();
        let cagri = CagriId::yeni();
        k.yaz(&zarf(
            oturum,
            2,
            KayitOgesi::AracBasladi {
                cagri,
                model_cagri_id: "m1".into(),
                arac: "yaz".into(),
                yedekler: vec![DosyaYedegi {
                    yol: kalfa_core::yol::CalismaYolu::coz("a.txt").unwrap(),
                    onceki: OncekiHal::Saklandi {
                        ozet: kalan.clone(),
                    },
                    sonraki: None,
                    olusan_dizinler: Vec::new(),
                    eklenen: 0,
                    silinen: 0,
                    diff: None,
                }],
            },
        ))
        .unwrap();
        k.yaz(&zarf(
            oturum,
            3,
            KayitOgesi::AracBitti {
                cagri,
                durum: AracDurumu::Basarili,
            },
        ))
        .unwrap();
    }
    let depo = OncekiHalDeposu::ac(&o.depo()).unwrap();
    depo.sakla(&kalan, b"kalan").unwrap();
    depo.sakla(&giden, b"giden").unwrap();

    let s = SahteSaglayici::yeni(vec![]);
    let (t, alici, _) = devam_et(
        &o.kayit(),
        o.ayarlar(Kip::Proje, true),
        test_baglami(s, vec![], sanal_siniflandirici()),
    )
    .unwrap();
    assert!(depo.var_mi(&kalan));
    assert!(!depo.var_mi(&giden), "kayıtta adı geçmeyen içerik silinir");
    Surucu::yeni(t, alici).kapat().await;
}

#[tokio::test]
async fn eski_kayit_yedeksiz_acilir() {
    // `yedekler` alanı olmayan (aşama 3'ten önceki) başladı kaydı.
    let satir = r#"{"sema":1,"oturum":"00000000-0000-0000-0000-000000000001","tur":null,"sira":2,"yuk":{"tip":"AracBasladi","cagri":"00000000-0000-0000-0000-000000000002","model_cagri_id":"m1","arac":"yaz"}}"#;
    let zarf: Zarf<KayitOgesi> = serde_json::from_str(satir).unwrap();
    match &zarf.yuk {
        KayitOgesi::AracBasladi { yedekler, .. } => assert!(yedekler.is_empty()),
        diger => panic!("{diger:?}"),
    }
    // Boş yedek listesi yazılmaz: eski okuyucular da açar.
    let geri = serde_json::to_string(&zarf).unwrap();
    assert!(!geri.contains("yedekler"), "{geri}");
    let _ = gorunum_uret(&[zarf]);
}

/// Onay kartının "Geri alınabilir mi" alanı (aşama 3 tasarımı §5.11, 3a
/// Task 8): bilgiyi çekirdek verir.
#[tokio::test]
async fn onay_ogesi_geri_alinabilirligi() {
    use kalfa_core::geri_alma::GeriAlinamazNedeni;
    use kalfa_core::tur_ozeti::GeriAlinabilirlik;

    async fn ogeler(o: &Ortam, kayitli: bool) -> Vec<kalfa_core::protokol::OnayOgesi> {
        let yaz = Arc::new(SahteArac::yazan("yaz").onceki_icerikli());
        let buyuk = Arc::new(SahteArac::yazan("buyuk").onceki_buyuk());
        let kabuk = Arc::new(SahteArac::kabuk("kabuk"));
        let oku = Arc::new(SahteArac::okuyan("oku"));
        let s = SahteSaglayici::yeni(vec![SahteYanit::araclar(vec![
            ("yaz", json!({"yol": "a.txt"})),
            ("buyuk", json!({"yol": "b.bin"})),
            ("kabuk", json!({"yol": "."})),
            ("oku", json!({"yol": ".env"})),
        ])]);
        let mut d = test_oturumu(
            test_baglami(
                s,
                vec![
                    yaz as Arc<dyn Arac>,
                    buyuk as Arc<dyn Arac>,
                    kabuk as Arc<dyn Arac>,
                    oku as Arc<dyn Arac>,
                ],
                sanal_siniflandirici(),
            ),
            o.ayarlar(Kip::Siki, kayitli),
        );
        d.gonder(mesaj()).await;
        let ogeler = match d
            .bekle(|o| matches!(o, Olay::OnayGerekiyor { .. }))
            .await
            .yuk
        {
            Olay::OnayGerekiyor { ogeler, .. } => ogeler,
            _ => unreachable!(),
        };
        let tur = d.tutamac.aktif_tur().unwrap();
        d.gonder(Komut::Iptal { tur }).await;
        d.tur_sonu().await;
        d.kapat().await;
        ogeler
    }
    let bul = |ogeler: &[kalfa_core::protokol::OnayOgesi], arac: &str| {
        ogeler
            .iter()
            .find(|o| o.arac == arac)
            .unwrap()
            .geri_alinabilir
    };
    let hayir = |neden| Some(GeriAlinabilirlik::Hayir { neden });

    let o = Ortam::yeni();
    let kayitli = ogeler(&o, true).await;
    assert_eq!(bul(&kayitli, "yaz"), Some(GeriAlinabilirlik::Evet));
    assert_eq!(bul(&kayitli, "buyuk"), hayir(GeriAlinamazNedeni::Buyuk));
    assert_eq!(bul(&kayitli, "kabuk"), hayir(GeriAlinamazNedeni::Kabuk));
    assert_eq!(bul(&kayitli, "oku"), None, "yalnızca okuyan öğe");

    let o = Ortam::yeni();
    let kayitsiz = ogeler(&o, false).await;
    assert_eq!(bul(&kayitsiz, "yaz"), hayir(GeriAlinamazNedeni::Kayitsiz));
    assert_eq!(bul(&kayitsiz, "buyuk"), hayir(GeriAlinamazNedeni::Kayitsiz));
}
