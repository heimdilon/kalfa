//! Tur özeti (aşama 3 tasarımı §5.5).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kalfa_core::arac::Arac;
use kalfa_core::gecmis::{GorunumOgesi, OturumDurumu, gorunum_uret, oturum_ozeti};
use kalfa_core::geri_alma::GeriAlinamazNedeni;
use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::{OturumId, TurId};
use kalfa_core::oturum::OturumAyarlari;
use kalfa_core::protokol::{AracDurumu, BitisNedeni, Komut, Olay, SEMA_SURUMU, Zarf};
use kalfa_core::test_destek::*;
use kalfa_core::tur_ozeti::{
    AgHostOzeti, AgKomutOzeti, DosyaIslemi, DosyaOzeti, GeriAlinabilirlik, IstekOzeti, TurOzeti,
};
use kalfa_core::yol::CalismaYolu;
use kalfa_evren::AkisOlayi;
use kalfa_evren::tipler::Usage;
use serde_json::json;

struct Ortam {
    d: tempfile::TempDir,
    dosyalar: Arc<SahteGeriYukleyici>,
}

impl Ortam {
    fn yeni() -> Self {
        Self {
            d: tempfile::tempdir().unwrap(),
            dosyalar: SahteGeriYukleyici::yeni(),
        }
    }

    fn kayit(&self) -> PathBuf {
        self.d.path().join("oturum.jsonl")
    }

    fn surucu(&self, s: Arc<SahteSaglayici>, kip: Kip, kayitli: bool) -> Surucu {
        let yaz = Arc::new(SahteArac::yazan("yaz").dosyalar(self.dosyalar.clone()));
        let bozuk = Arc::new(
            SahteArac::yazan("bozuk")
                .dosyalar(self.dosyalar.clone())
                .hata("yazılamadı".into()),
        );
        let kabuk = Arc::new(SahteArac::kabuk("kabuk"));
        let mut baglam = test_baglami(
            s,
            vec![
                yaz as Arc<dyn Arac>,
                bozuk as Arc<dyn Arac>,
                kabuk as Arc<dyn Arac>,
            ],
            sanal_siniflandirici(),
        );
        baglam.geri_yukleyici = Some(self.dosyalar.clone());
        let mut a = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
        a.kip = kip;
        if kayitli {
            a.kayit_yolu = Some(self.kayit());
            a.geri_alma_dizini = Some(self.d.path().join("geri-alma"));
        }
        test_oturumu(baglam, a)
    }

    fn surucu_kabuk(&self, s: Arc<SahteSaglayici>, kabuk: Arc<SahteArac>) -> Surucu {
        let mut baglam = test_baglami(s, vec![kabuk as Arc<dyn Arac>], sanal_siniflandirici());
        baglam.geri_yukleyici = Some(self.dosyalar.clone());
        let mut a = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
        a.kip = Kip::Serbest;
        a.kayit_yolu = Some(self.kayit());
        a.geri_alma_dizini = Some(self.d.path().join("geri-alma"));
        test_oturumu(baglam, a)
    }
}

fn mesaj(m: &str) -> Komut {
    Komut::KullaniciMesaji { metin: m.into() }
}

fn kullanimli_metin(metin: &str, istem: u64, tamamlama: u64) -> SahteYanit {
    SahteYanit::Akis(vec![
        Ok(AkisOlayi::Metin(metin.into())),
        Ok(AkisOlayi::Kullanim(Usage {
            prompt_tokens: istem,
            completion_tokens: tamamlama,
            total_tokens: istem + tamamlama,
            completion_tokens_details: None,
            evren: None,
        })),
        Ok(AkisOlayi::Bitti {
            neden: "stop".into(),
        }),
    ])
}

/// Turun `Tamamlandi`'sından sonra, turun zarfıyla gelen özet.
async fn ozet_bekle(d: &mut Surucu) -> TurOzeti {
    let zarf = d.bekle(|o| matches!(o, Olay::TurOzeti { .. })).await;
    match zarf.yuk {
        Olay::TurOzeti { ozet } => {
            assert_eq!(zarf.tur, Some(ozet.tur), "özet turun zarfıyla gelir");
            ozet
        }
        _ => unreachable!(),
    }
}

fn y(s: &str) -> CalismaYolu {
    CalismaYolu::coz(s).unwrap()
}

#[tokio::test]
async fn tur_ozeti_dosyalari_komutlari_ve_istekleri_tasir() {
    let o = Ortam::yeni();
    o.dosyalar.yaz("a.txt", b"eski");
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![
            ("yaz", json!({"yol": "a.txt", "icerik": "yeni"})),
            ("yaz", json!({"yol": "b.txt", "icerik": "b"})),
        ]),
        SahteYanit::arac("kabuk", json!({"yol": "."})),
        kullanimli_metin("bitti", 100, 20),
    ]);
    let mut d = o.surucu(s, Kip::Serbest, true);
    d.gonder(mesaj("değiştir")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    let ozet = ozet_bekle(&mut d).await;

    assert_eq!(
        ozet.dosyalar,
        vec![
            DosyaOzeti {
                yol: y("a.txt"),
                islem: DosyaIslemi::Degisti,
                eklenen: 1,
                silinen: 0,
                sonucu_bilinmiyor: false,
                geri_alinabilir: GeriAlinabilirlik::Evet,
            },
            DosyaOzeti {
                yol: y("b.txt"),
                islem: DosyaIslemi::Olusturuldu,
                eklenen: 1,
                silinen: 0,
                sonucu_bilinmiyor: false,
                geri_alinabilir: GeriAlinabilirlik::Evet,
            },
        ]
    );
    assert_eq!(ozet.komutlar.len(), 1);
    assert_eq!(ozet.komutlar[0].komut, "test");
    assert_eq!(ozet.komutlar[0].durum, AracDurumu::Basarili);
    assert_eq!(
        ozet.istekler,
        IstekOzeti {
            sayi: 3,
            istem: 100,
            tamamlama: 20,
            dusunce: 0,
        }
    );
    d.kapat().await;
    let kayit = OturumKaydi::oku(&o.kayit()).unwrap();
    assert!(
        kayit
            .iter()
            .any(|z| matches!(&z.yuk, KayitOgesi::TurOzeti { ozet: k } if *k == ozet))
    );
}

#[tokio::test]
async fn basarisiz_yazma_ozette_yok() {
    let o = Ortam::yeni();
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("bozuk", json!({"yol": "a.txt", "icerik": "x"})),
        SahteYanit::metin("olmadı"),
    ]);
    let mut d = o.surucu(s, Kip::Proje, true);
    d.gonder(mesaj("yaz")).await;
    d.tur_sonu().await;
    let ozet = ozet_bekle(&mut d).await;
    assert!(ozet.dosyalar.is_empty(), "{ozet:?}");
    d.kapat().await;
}

#[tokio::test]
async fn iptal_edilen_turun_ozeti_kaydedilir() {
    let o = Ortam::yeni();
    let s = SahteSaglayici::yeni(vec![SahteYanit::arac(
        "yaz",
        json!({"yol": "a.txt", "icerik": "x"}),
    )]);
    let mut d = o.surucu(s, Kip::Siki, true);
    d.gonder(mesaj("yaz")).await;
    d.bekle(|o| matches!(o, Olay::OnayGerekiyor { .. })).await;
    let tur = d.tutamac.aktif_tur().unwrap();
    d.gonder(Komut::Iptal { tur }).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);
    let ozet = ozet_bekle(&mut d).await;
    assert_eq!(ozet.tur, tur);
    assert!(ozet.dosyalar.is_empty());
    d.kapat().await;
}

#[tokio::test]
async fn bos_tur_ozeti_kaydedilir() {
    let o = Ortam::yeni();
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("merhaba")]);
    let mut d = o.surucu(s, Kip::Siki, true);
    d.gonder(mesaj("selam")).await;
    d.tur_sonu().await;
    let ozet = ozet_bekle(&mut d).await;
    assert!(ozet.dosyalar.is_empty() && ozet.komutlar.is_empty());
    assert_eq!(ozet.istekler.sayi, 1);
    d.kapat().await;
}

#[tokio::test]
async fn kayitsiz_oturumun_ozeti_geri_alinamaz_der() {
    let o = Ortam::yeni();
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", json!({"yol": "a.txt", "icerik": "x"})),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = o.surucu(s, Kip::Proje, false);
    d.gonder(mesaj("yaz")).await;
    d.tur_sonu().await;
    let ozet = ozet_bekle(&mut d).await;
    assert_eq!(
        ozet.dosyalar[0].geri_alinabilir,
        GeriAlinabilirlik::Hayir {
            neden: GeriAlinamazNedeni::Kayitsiz
        }
    );
    d.kapat().await;
}

#[tokio::test]
async fn tur_ozeti_turun_son_ogesidir_ve_liste_durumunu_degistirmez() {
    let o = Ortam::yeni();
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", json!({"yol": "a.txt", "icerik": "x"})),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = o.surucu(s, Kip::Proje, true);
    d.gonder(mesaj("yaz")).await;
    d.tur_sonu().await;
    ozet_bekle(&mut d).await;
    d.kapat().await;

    let gorunum = gorunum_uret(&OturumKaydi::oku(&o.kayit()).unwrap());
    let tipler: Vec<&str> = gorunum
        .iter()
        .map(|g| match g {
            GorunumOgesi::KullaniciMesaji { .. } => "mesaj",
            GorunumOgesi::Yanit { .. } => "yanit",
            GorunumOgesi::AracCagrisi { .. } => "arac",
            GorunumOgesi::TurSonu { .. } => "son",
            GorunumOgesi::TurOzeti { .. } => "ozet",
            _ => "diger",
        })
        .collect();
    assert_eq!(tipler, ["mesaj", "arac", "yanit", "son", "ozet"]);
    let turlar: Vec<Option<TurId>> = gorunum.iter().map(GorunumOgesi::tur).collect();
    assert!(
        turlar.windows(2).all(|w| w[0] == w[1]),
        "özet turun içindedir"
    );
    assert_eq!(oturum_ozeti(&o.kayit()).durum, OturumDurumu::Tamamlandi);
}

fn zarf(oturum: OturumId, tur: Option<TurId>, sira: u64, yuk: KayitOgesi) -> Zarf<KayitOgesi> {
    Zarf {
        sema: SEMA_SURUMU,
        oturum,
        tur,
        sira,
        yuk,
    }
}

#[test]
fn veri_silinen_turun_ozeti_gecmiste_geri_alinamaz() {
    let (oturum, tur) = (OturumId::yeni(), TurId::yeni());
    let ozet = TurOzeti {
        tur,
        dosyalar: vec![DosyaOzeti {
            yol: y("a.txt"),
            islem: DosyaIslemi::Degisti,
            eklenen: 1,
            silinen: 1,
            sonucu_bilinmiyor: false,
            geri_alinabilir: GeriAlinabilirlik::Evet,
        }],
        komutlar: Vec::new(),
        istekler: IstekOzeti::default(),
        guvenlik_uyarisi: 0,
    };
    let kayitlar = vec![
        zarf(oturum, Some(tur), 1, KayitOgesi::TurOzeti { ozet }),
        zarf(
            oturum,
            None,
            2,
            KayitOgesi::GeriAlmaVerisiSilindi { turlar: vec![tur] },
        ),
    ];
    let gorunum = gorunum_uret(&kayitlar);
    let GorunumOgesi::TurOzeti { ozet, .. } = &gorunum[0] else {
        panic!("{gorunum:?}");
    };
    assert_eq!(
        ozet.dosyalar[0].geri_alinabilir,
        GeriAlinabilirlik::Hayir {
            neden: GeriAlinamazNedeni::VeriSilindi
        }
    );
}

#[test]
fn eski_kayit_ozetsiz_acilir() {
    let (oturum, tur) = (OturumId::yeni(), TurId::yeni());
    let kayitlar = vec![zarf(
        oturum,
        Some(tur),
        1,
        KayitOgesi::TurBitti {
            neden: BitisNedeni::Normal,
        },
    )];
    let gorunum = gorunum_uret(&kayitlar);
    assert!(
        !gorunum
            .iter()
            .any(|g| matches!(g, GorunumOgesi::TurOzeti { .. }))
    );
}

#[tokio::test]
async fn tur_ozetinde_komutun_ag_ozeti() {
    let o = Ortam::yeni();
    let ag = AgKomutOzeti {
        hostlar: vec![
            AgHostOzeti {
                host: "index.crates.io".into(),
                port: 443,
                baglanti_sayisi: 2,
                giden_bayt: 120,
                gelen_bayt: 4500,
                reddedilenler: 0,
            },
            AgHostOzeti {
                host: "evil.com".into(),
                port: 443,
                baglanti_sayisi: 1,
                giden_bayt: 0,
                gelen_bayt: 0,
                reddedilenler: 1,
            },
        ],
        toplam_giden: 120,
        toplam_gelen: 4500,
        reddedilen_sayisi: 1,
    };
    let kabuk = Arc::new(SahteArac::kabuk("kabuk").ag_ozeti(ag.clone()));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("kabuk", json!({"yol": "."})),
        kullanimli_metin("bitti", 100, 20),
    ]);
    let mut d = o.surucu_kabuk(s, kabuk);

    d.gonder(mesaj("derle")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    let ozet = ozet_bekle(&mut d).await;
    assert_eq!(ozet.komutlar.len(), 1);
    assert_eq!(ozet.komutlar[0].ag, Some(ag));
    d.kapat().await;
}
