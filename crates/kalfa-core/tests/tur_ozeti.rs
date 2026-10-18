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
    AgHostOzeti, AgKomutOzeti, DosyaDegisikligi, DosyaIslemi, DosyaOzeti, GeriAlinabilirlik,
    IstekOzeti, TurOzeti,
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
                diff: Some("+sahte yaz".into()),
            },
            DosyaOzeti {
                yol: y("b.txt"),
                islem: DosyaIslemi::Olusturuldu,
                eklenen: 1,
                silinen: 0,
                sonucu_bilinmiyor: false,
                geri_alinabilir: GeriAlinabilirlik::Evet,
                diff: Some("+sahte yaz".into()),
            },
        ]
    );
    assert_eq!(ozet.komutlar.len(), 1);
    assert_eq!(ozet.komutlar[0].komut, "test");
    assert_eq!(ozet.komutlar[0].durum, AracDurumu::Basarili);
    // İki araç yanıtı sahte başarılı yanıtın varsayılan 10/5 kullanımını taşır;
    // son yanıt açık 100/20 kullanımdır.
    assert_eq!(
        ozet.istekler,
        IstekOzeti {
            sayi: 3,
            istem: 120,
            tamamlama: 30,
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
            diff: None,
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

#[test]
fn buyuk_coklu_duzenleme_tur_ozeti_olay_butcesine_sigar() {
    use kalfa_core::kimlik::CagriId;
    use kalfa_core::oturum::OlayButcesi;
    use kalfa_core::tur_ozeti::{AZAMI_TUR_OZETI_DIFF_BAYT, TurBiriktirici, YazilanDosya};

    let mut biriktirici = TurBiriktirici::default();
    let satir_eski = format!("-{}\n", "a".repeat(126));
    let satir_yeni = format!("+{}\n", "b".repeat(126));
    let tek_diff = format!(
        "--- a/dosya.rs\n+++ b/dosya.rs\n@@ -1,2048 +1,2048 @@\n{}{}",
        satir_eski.repeat(2048),
        satir_yeni.repeat(2048),
    );
    assert!(tek_diff.len() > 512 * 1024);

    for i in 0..17 {
        let cagri = CagriId::yeni();
        biriktirici.yazma_basladi(
            cagri,
            vec![YazilanDosya {
                yol: y(&format!("src/dosya_{i}.rs")),
                olusturuldu: false,
                eklenen: 2048,
                silinen: 2048,
                geri_alinabilir: GeriAlinabilirlik::Evet,
                diff: Some(tek_diff.clone()),
            }],
        );
        biriktirici.arac_bitti(cagri, AracDurumu::Basarili);
    }

    let ozet = biriktirici.bitir(TurId::yeni(), false);
    let toplam_diff: usize = ozet
        .dosyalar
        .iter()
        .filter_map(|d| d.diff.as_ref().map(String::len))
        .sum();
    assert!(toplam_diff <= AZAMI_TUR_OZETI_DIFF_BAYT);

    let serilestirilmis = serde_json::to_vec(&Olay::TurOzeti { ozet }).unwrap();
    assert!(
        serilestirilmis.len() < OlayButcesi::default().azami_bayt(),
        "TurOzeti olayı 8 MiB olay bütçesine sığmalı: {} bayt",
        serilestirilmis.len()
    );
}

#[test]
fn ayni_dosyanin_ardisik_yamalarinda_ara_dosya_basliklari_atlanir() {
    use kalfa_core::kimlik::CagriId;
    use kalfa_core::tur_ozeti::{TurBiriktirici, YazilanDosya};

    let mut biriktirici = TurBiriktirici::default();
    let yama1 = "--- a/x\n+++ b/x\n@@ -1,1 +1,1 @@\n-a\n+b\n";
    let yama2 = "--- a/x\n+++ b/x\n@@ -1,1 +1,1 @@\n-b\n+c\n";

    for yama in [yama1, yama2] {
        let cagri = CagriId::yeni();
        biriktirici.yazma_basladi(
            cagri,
            vec![YazilanDosya {
                yol: y("x"),
                olusturuldu: false,
                eklenen: 1,
                silinen: 1,
                geri_alinabilir: GeriAlinabilirlik::Evet,
                diff: Some(yama.into()),
            }],
        );
        biriktirici.arac_bitti(cagri, AracDurumu::Basarili);
    }

    let ozet = biriktirici.bitir(TurId::yeni(), false);
    let birlesik = ozet.dosyalar[0].diff.as_deref().unwrap();
    assert_eq!(birlesik.matches("--- a/x").count(), 1);
    assert_eq!(birlesik.matches("+++ b/x").count(), 1);
}

#[test]
fn kontrol_karakterli_diffler_json_butcesine_sigar() {
    use kalfa_core::kimlik::CagriId;
    use kalfa_core::oturum::OlayButcesi;
    use kalfa_core::tur_ozeti::{AZAMI_TUR_OZETI_DIFF_BAYT, TurBiriktirici, YazilanDosya};

    let satir_eski = format!("-{}\n", "\u{0001}".repeat(127));
    let satir_yeni = format!("+{}\n", "\u{0002}".repeat(127));
    let diff = format!(
        "--- a/x\n+++ b/x\n@@ -1,2048 +1,2048 @@\n{}{}",
        satir_eski.repeat(2048),
        satir_yeni.repeat(2048),
    );
    let mut biriktirici = TurBiriktirici::default();
    for i in 0..5 {
        let cagri = CagriId::yeni();
        biriktirici.yazma_basladi(
            cagri,
            vec![YazilanDosya {
                yol: y(&format!("kontrol-{i}.txt")),
                olusturuldu: false,
                eklenen: 2048,
                silinen: 2048,
                geri_alinabilir: GeriAlinabilirlik::Evet,
                diff: Some(diff.clone()),
            }],
        );
        biriktirici.arac_bitti(cagri, AracDurumu::Basarili);
    }

    let ozet = biriktirici.bitir(TurId::yeni(), false);
    let json_diff_bayt: usize = ozet
        .dosyalar
        .iter()
        .filter_map(|d| d.diff.as_ref())
        .map(|diff| serde_json::to_vec(diff).unwrap().len())
        .sum();
    assert!(json_diff_bayt <= AZAMI_TUR_OZETI_DIFF_BAYT);
    assert!(
        ozet.dosyalar[0]
            .diff
            .as_ref()
            .is_some_and(|diff| { diff.contains("@@ ") && diff.ends_with('\n') })
    );
    let olay_bayt = serde_json::to_vec(&Olay::TurOzeti { ozet }).unwrap().len();
    assert!(olay_bayt < OlayButcesi::default().azami_bayt());
}

/// Masaüstü arayüzü tasarımı §4: yazan çağrının dosya değişiklikleri
/// `AracCagrisiBasladi`'nın hemen ardından, sonuçtan önce yayımlanır; geçmiş
/// aynı bilgiyi "başladı" kaydının yedeklerinden üretir.
#[tokio::test]
async fn yazan_cagrinin_degisiklikleri_sonuctan_once_gelir_ve_gecmiste_gorunur() {
    let o = Ortam::yeni();
    o.dosyalar.yaz("a.txt", b"eski");
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", json!({"yol": "a.txt", "icerik": "yeni"})),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = o.surucu(s, Kip::Serbest, true);
    d.gonder(mesaj("değiştir")).await;
    let basladi = d
        .bekle(|o| matches!(o, Olay::AracCagrisiBasladi { .. }))
        .await;
    let Olay::AracCagrisiBasladi { cagri, .. } = basladi.yuk else {
        unreachable!()
    };
    let beklenen = vec![DosyaDegisikligi {
        yol: y("a.txt"),
        islem: DosyaIslemi::Degisti,
        eklenen: 1,
        silinen: 0,
        diff: Some("+sahte yaz".into()),
    }];
    assert_eq!(
        d.sonraki().await.yuk,
        Olay::DosyaDegisiklikleri {
            cagri,
            dosyalar: beklenen.clone(),
        },
        "değişiklikler sonuçtan önce gelir"
    );
    assert!(matches!(d.sonraki().await.yuk, Olay::AracSonucu { .. }));
    d.tur_sonu().await;
    d.kapat().await;

    let gorunum = gorunum_uret(&OturumKaydi::oku(&o.kayit()).unwrap());
    let degisiklikler = gorunum
        .iter()
        .find_map(|g| match g {
            GorunumOgesi::AracCagrisi { degisiklikler, .. } => Some(degisiklikler.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(degisiklikler, beklenen);
}

/// Yazmayan çağrı değişiklik olayı yayımlamaz.
#[tokio::test]
async fn yazmayan_cagri_degisiklik_olayi_yayimlamaz() {
    let o = Ortam::yeni();
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("kabuk", json!({"yol": "."})),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = o.surucu(s, Kip::Serbest, true);
    d.gonder(mesaj("çalıştır")).await;
    d.bekle(|o| matches!(o, Olay::AracCagrisiBasladi { .. }))
        .await;
    loop {
        let olay = d.sonraki().await.yuk;
        assert!(!matches!(olay, Olay::DosyaDegisiklikleri { .. }));
        if matches!(olay, Olay::Tamamlandi { .. }) {
            break;
        }
    }
}
