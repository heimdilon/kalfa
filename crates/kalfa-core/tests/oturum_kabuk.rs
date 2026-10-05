//! Kabuk çağrısı oturumda (aşama 2 tasarımı §4.4, §4.5, §5): yerel onay,
//! ara çıktı, çıktı seli ve başarısızlık sayacı. Kabuk burada sahtedir;
//! gerçek komutlar `kalfa-tools` ve `kalfa-sandbox` testlerindedir.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, CiktiGonderici, Hazirlik};
use kalfa_core::izin::Kip;
use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
use kalfa_core::oturum::{OlayButcesi, OturumAyarlari};
use kalfa_core::protokol::{
    AracDurumu, BitisNedeni, CiktiAkisi, Komut, Olay, OnayKarari, OnayOgesi,
};
use kalfa_core::test_destek::*;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

struct SahteKabuk {
    yalitimli: bool,
    parca_sayisi: usize,
    parca_bayt: usize,
    basarisiz: bool,
    calisma: AtomicUsize,
    bitti: AtomicBool,
}

impl SahteKabuk {
    fn yeni(yalitimli: bool) -> Self {
        Self {
            yalitimli,
            parca_sayisi: 3,
            parca_bayt: 4,
            basarisiz: false,
            calisma: AtomicUsize::new(0),
            bitti: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl Arac for SahteKabuk {
    fn ad(&self) -> &str {
        "kabuk"
    }
    fn aciklama(&self) -> &str {
        "sahte kabuk"
    }
    fn parametre_semasi(&self) -> Value {
        json!({"type": "object"})
    }
    fn yazar_mi(&self) -> bool {
        true
    }
    async fn hazirla(&self, _a: &Value) -> Result<Hazirlik, AracHatasi> {
        let yalitim = if self.yalitimli {
            YalitimOzeti::Yalitimli {
                arka_uc: "sahte".into(),
                profil: "p".into(),
            }
        } else {
            YalitimOzeti::Yalitimsiz {
                talimat: "yalıtım yok".into(),
            }
        };
        Ok(Hazirlik {
            tum_calisma_alani: true,
            aciklama: "`test` komutunu çalıştırmak istiyor.".into(),
            kabuk: Some(KabukTarifi {
                komut: "test".into(),
                bicim: KabukBicimi::Program,
                dizin: ".".into(),
                ag: false,
                yalitim,
                ilk_cagri: false,
                alan_hazirligi: false,
            }),
            ..Default::default()
        })
    }
    async fn calistir(
        &self,
        a: &Value,
        h: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.calistir_akisli(a, h, iptal, CiktiGonderici::bos())
            .await
    }
    async fn calistir_akisli(
        &self,
        _a: &Value,
        _h: &Hazirlik,
        _iptal: CancellationToken,
        cikti: CiktiGonderici,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.calisma.fetch_add(1, Ordering::SeqCst);
        for i in 0..self.parca_sayisi {
            let akis = if i % 2 == 0 {
                CiktiAkisi::Standart
            } else {
                CiktiAkisi::Hata
            };
            cikti.gonder(akis, "x".repeat(self.parca_bayt));
            tokio::task::yield_now().await;
        }
        self.bitti.store(true, Ordering::SeqCst);
        let c = AracCiktisi {
            icerik: "Çıkış kodu: 1".into(),
            kaynak: "kabuk:test".into(),
            guvenlik_uyarilari: Vec::new(),
            degisiklikler: None,
            kalfa_uretimi: false,
        };
        if self.basarisiz {
            Err(AracHatasi::Basarisiz(c))
        } else {
            Ok(c)
        }
    }
}

fn surucu(kabuk: &Arc<SahteKabuk>, yanitlar: Vec<SahteYanit>, ayar: OturumAyarlari) -> Surucu {
    let s = SahteSaglayici::yeni(yanitlar);
    test_oturumu(
        test_baglami(
            s,
            vec![kabuk.clone() as Arc<dyn Arac>],
            sanal_siniflandirici(),
        ),
        ayar,
    )
}

fn ayarlar(kip: Kip) -> OturumAyarlari {
    let mut a = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    a.kip = kip;
    a
}

fn mesaj() -> Komut {
    Komut::KullaniciMesaji {
        metin: "çalıştır".into(),
    }
}

async fn onay_bekle(d: &mut Surucu) -> Vec<OnayOgesi> {
    match d
        .bekle(|o| matches!(o, Olay::OnayGerekiyor { .. }))
        .await
        .yuk
    {
        Olay::OnayGerekiyor { ogeler, .. } => ogeler,
        _ => unreachable!(),
    }
}

fn parcalar(d: &Surucu) -> Vec<(CiktiAkisi, String, u64)> {
    d.gecmis
        .iter()
        .filter_map(|z| match &z.yuk {
            Olay::AracCiktisiParcasi {
                akis,
                metin,
                atlanan_bayt,
                ..
            } => Some((*akis, metin.clone(), *atlanan_bayt)),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn yalitimsiz_kabuk_yalnizca_yerel_onayla_calisir() {
    let kabuk = Arc::new(SahteKabuk::yeni(false));
    let mut d = surucu(
        &kabuk,
        vec![
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::metin("bitti"),
        ],
        ayarlar(Kip::Serbest),
    );
    d.gonder(mesaj()).await;
    let ogeler = onay_bekle(&mut d).await;
    let oge = &ogeler[0];
    assert!(oge.yerel_onay, "Serbest kipte de yerel onay ister");
    assert_eq!(oge.kabuk.as_ref().map(|k| k.komut.as_str()), Some("test"));
    assert_eq!(
        d.tutamac.bekleyen_onay(oge.cagri).as_ref(),
        Some(oge),
        "yerel iletişim kutusu komutu çekirdekten okur"
    );

    // Webview'ın gönderebildiği onay yerel öğeyi onaylamaz.
    d.gonder(Komut::AracOnayi {
        cagri: oge.cagri,
        karar: OnayKarari::Onayla,
        muhur: oge.muhur.clone(),
    })
    .await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("yerel pencerede")))
        .await;
    assert_eq!(kabuk.calisma.load(Ordering::SeqCst), 0);

    d.gonder(Komut::YerelOnay {
        cagri: oge.cagri,
        muhur: oge.muhur.clone(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(kabuk.calisma.load(Ordering::SeqCst), 1);
    assert_eq!(
        d.tutamac.bekleyen_onay(oge.cagri),
        None,
        "onay bitince temizlenir"
    );
}

#[tokio::test]
async fn yerel_oge_reddedilebilir_ve_yerel_olmayan_yerel_onayla_onaylanmaz() {
    let kabuk = Arc::new(SahteKabuk::yeni(false));
    let mut d = surucu(
        &kabuk,
        vec![
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::metin("peki"),
        ],
        ayarlar(Kip::Siki),
    );
    d.gonder(mesaj()).await;
    let oge = onay_bekle(&mut d).await.remove(0);
    d.gonder(Komut::AracOnayi {
        cagri: oge.cagri,
        karar: OnayKarari::Reddet { gerekce: None },
        muhur: oge.muhur.clone(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(kabuk.calisma.load(Ordering::SeqCst), 0);

    let yalitimli = Arc::new(SahteKabuk::yeni(true));
    let mut d = surucu(
        &yalitimli,
        vec![
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::metin("peki"),
        ],
        ayarlar(Kip::Siki),
    );
    d.gonder(mesaj()).await;
    let oge = onay_bekle(&mut d).await.remove(0);
    assert!(!oge.yerel_onay);
    d.gonder(Komut::YerelOnay {
        cagri: oge.cagri,
        muhur: oge.muhur.clone(),
    })
    .await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("yerel onay gerektirmiyor")))
        .await;
    d.gonder(Komut::AracOnayi {
        cagri: oge.cagri,
        karar: OnayKarari::Onayla,
        muhur: oge.muhur.clone(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(yalitimli.calisma.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn yalitimli_agsiz_kabuk_serbest_kipte_onaysiz_calisir() {
    let kabuk = Arc::new(SahteKabuk::yeni(true));
    let mut d = surucu(
        &kabuk,
        vec![
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::metin("bitti"),
        ],
        ayarlar(Kip::Serbest),
    );
    d.gonder(mesaj()).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(kabuk.calisma.load(Ordering::SeqCst), 1);
    assert!(
        !d.gecmis
            .iter()
            .any(|z| matches!(z.yuk, Olay::OnayGerekiyor { .. }))
    );
}

#[tokio::test]
async fn ara_cikti_sonuctan_once_ve_akisiyla_gelir() {
    let kabuk = Arc::new(SahteKabuk::yeni(true));
    let mut d = surucu(
        &kabuk,
        vec![
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::metin("bitti"),
        ],
        ayarlar(Kip::Serbest),
    );
    d.gonder(mesaj()).await;
    d.bekle(|o| matches!(o, Olay::AracSonucu { .. })).await;
    assert_eq!(
        parcalar(&d),
        vec![
            (CiktiAkisi::Standart, "xxxx".into(), 0),
            (CiktiAkisi::Hata, "xxxx".into(), 0),
            (CiktiAkisi::Standart, "xxxx".into(), 0),
        ]
    );
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
}

/// Arayüz okumazken bile çıktı seli olay kuyruğunu doldurmaz ve turu iptal
/// ettirmez; atılan her bayt sayılır.
#[tokio::test]
async fn cikti_seli_turu_iptal_ettirmez_atilan_bayt_sayilir() {
    let kabuk = Arc::new(SahteKabuk {
        parca_sayisi: 2000,
        parca_bayt: 1024,
        ..SahteKabuk::yeni(true)
    });
    let mut ayar = ayarlar(Kip::Serbest);
    ayar.olay_butcesi = OlayButcesi::yeni(64, 64 * 1024).unwrap();
    let mut d = surucu(
        &kabuk,
        vec![
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::metin("bitti"),
        ],
        ayar,
    );
    d.gonder(mesaj()).await;
    // Arayüz takılmış gibi: araç bitene kadar olay okunmaz.
    tokio::time::timeout(Duration::from_secs(10), async {
        while !kabuk.bitti.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("araç bitmeli");
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert!(
        !d.gecmis
            .iter()
            .any(|z| matches!(z.yuk, Olay::AktarimDurdu { .. })),
        "kuyruk dolmamalı"
    );
    let p = parcalar(&d);
    let gelen: u64 = p.iter().map(|(_, m, _)| m.len() as u64).sum();
    let atlanan: u64 = p.iter().map(|(_, _, a)| a).sum();
    assert!(atlanan > 0, "ekran yetişemediği için parça atılmalı");
    assert_eq!(gelen + atlanan, 2000 * 1024, "atılan her bayt bildirilir");
}

/// Kabukta sıfırdan farklı çıkış kodu aynı çağrının başarısızlık sınırına
/// sayılmaz (aşama 2 tasarımı §4.5).
#[tokio::test]
async fn basarisiz_cikti_ayni_hata_sinirina_sayilmaz() {
    let kabuk = Arc::new(SahteKabuk {
        basarisiz: true,
        ..SahteKabuk::yeni(true)
    });
    let mut d = surucu(
        &kabuk,
        vec![
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::metin("düzeldi"),
        ],
        ayarlar(Kip::Serbest),
    );
    d.gonder(mesaj()).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    let sonuclar: Vec<(AracDurumu, String)> = d
        .gecmis
        .iter()
        .filter_map(|z| match &z.yuk {
            Olay::AracSonucu { durum, icerik, .. } => Some((*durum, icerik.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(sonuclar.len(), 4);
    for (durum, icerik) in sonuclar {
        assert_eq!(durum, AracDurumu::Basarisiz);
        let satirlar: Vec<&str> = icerik.lines().collect();
        assert!(
            satirlar[0].starts_with("<arac_sonucu kaynak=\"kabuk:test\" sinir=\""),
            "{icerik}"
        );
        assert_eq!(satirlar[1], "Çıkış kodu: 1");
    }
}

/// Windows'taki çalışma alanı hazırlığının sahtesi (aşama 2 tasarımı §9.4).
/// Hazırlık ve ilk çağrı tek seferliktir: onaylanan ilk komut çalışınca
/// tükenir. Hazırlanmamış alanda hazırlığı onaylanmamış komut başlamaz;
/// gerçek yalıtıcı da böyle davranır.
struct HazirlikliKabuk {
    hazir: AtomicBool,
    guvenildi: AtomicBool,
    hazirlama: AtomicUsize,
    calisma: AtomicUsize,
}

impl HazirlikliKabuk {
    fn yeni(hazir: bool, guvenildi: bool) -> Self {
        Self {
            hazir: AtomicBool::new(hazir),
            guvenildi: AtomicBool::new(guvenildi),
            hazirlama: AtomicUsize::new(0),
            calisma: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Arac for HazirlikliKabuk {
    fn ad(&self) -> &str {
        "kabuk"
    }
    fn aciklama(&self) -> &str {
        "hazırlıklı sahte kabuk"
    }
    fn parametre_semasi(&self) -> Value {
        json!({"type": "object"})
    }
    fn yazar_mi(&self) -> bool {
        true
    }
    async fn hazirla(&self, a: &Value) -> Result<Hazirlik, AracHatasi> {
        Ok(Hazirlik {
            tum_calisma_alani: true,
            aciklama: "`test` komutunu çalıştırmak istiyor.".into(),
            kabuk: Some(KabukTarifi {
                komut: a.to_string(),
                bicim: KabukBicimi::Program,
                dizin: ".".into(),
                ag: false,
                yalitim: YalitimOzeti::Yalitimli {
                    arka_uc: "sahte".into(),
                    profil: "p".into(),
                },
                ilk_cagri: !self.guvenildi.load(Ordering::SeqCst),
                alan_hazirligi: !self.hazir.load(Ordering::SeqCst),
            }),
            ..Default::default()
        })
    }
    async fn calistir(
        &self,
        a: &Value,
        h: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.calistir_akisli(a, h, iptal, CiktiGonderici::bos())
            .await
    }
    async fn calistir_akisli(
        &self,
        _a: &Value,
        h: &Hazirlik,
        _iptal: CancellationToken,
        _cikti: CiktiGonderici,
    ) -> Result<AracCiktisi, AracHatasi> {
        let onaylandi = h.kabuk.as_ref().is_some_and(|t| t.alan_hazirligi);
        if !self.hazir.load(Ordering::SeqCst) {
            if !onaylandi {
                return Err(AracHatasi::Gecersiz(
                    "hazırlanmamış ve hazırlık onaylanmamış".into(),
                ));
            }
            self.hazirlama.fetch_add(1, Ordering::SeqCst);
            self.hazir.store(true, Ordering::SeqCst);
        }
        self.guvenildi.store(true, Ordering::SeqCst);
        self.calisma.fetch_add(1, Ordering::SeqCst);
        Ok(AracCiktisi {
            icerik: "Çıkış kodu: 0".into(),
            kaynak: "kabuk:test".into(),
            guvenlik_uyarilari: Vec::new(),
            degisiklikler: None,
            kalfa_uretimi: false,
        })
    }
}

fn hazirlikli_surucu(kabuk: &Arc<HazirlikliKabuk>, yanitlar: Vec<SahteYanit>, kip: Kip) -> Surucu {
    let s = SahteSaglayici::yeni(yanitlar);
    test_oturumu(
        test_baglami(
            s,
            vec![kabuk.clone() as Arc<dyn Arac>],
            sanal_siniflandirici(),
        ),
        ayarlar(kip),
    )
}

fn sonuclar(d: &Surucu) -> Vec<(AracDurumu, String)> {
    d.gecmis
        .iter()
        .filter_map(|z| match &z.yuk {
            Olay::AracSonucu { durum, icerik, .. } => Some((*durum, icerik.clone())),
            _ => None,
        })
        .collect()
}

/// Aynı turda birden çok kabuk çağrısı hazırlanmamış çalışma alanında
/// onaylanır. İlki alanı hazırlar; ötekilerin taze tarifinde hazırlık ve ilk
/// çağrı artık yoktur. Onay bu tek seferlik koşulları kapsadığı için
/// hepsi çalışır; alan bir kez hazırlanır.
#[tokio::test]
async fn ayni_turdaki_kabuklar_tukenen_hazirlikla_calisir() {
    let kabuk = Arc::new(HazirlikliKabuk::yeni(false, false));
    let mut d = hazirlikli_surucu(
        &kabuk,
        vec![
            SahteYanit::araclar(vec![
                ("kabuk", json!({"n": 1})),
                ("kabuk", json!({"n": 2})),
                ("kabuk", json!({"n": 3})),
            ]),
            SahteYanit::metin("bitti"),
        ],
        Kip::Serbest,
    );
    d.gonder(mesaj()).await;
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(ogeler.len(), 3);
    for oge in &ogeler {
        let t = oge.kabuk.as_ref().unwrap();
        assert!(t.alan_hazirligi && t.ilk_cagri);
        assert!(oge.yerel_onay);
    }
    for oge in ogeler {
        d.gonder(Komut::YerelOnay {
            cagri: oge.cagri,
            muhur: oge.muhur,
        })
        .await;
    }
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    let sonuclar = sonuclar(&d);
    assert_eq!(sonuclar.len(), 3);
    for (durum, icerik) in &sonuclar {
        assert_eq!(*durum, AracDurumu::Basarili, "{icerik}");
    }
    assert_eq!(kabuk.calisma.load(Ordering::SeqCst), 3);
    assert_eq!(kabuk.hazirlama.load(Ordering::SeqCst), 1);
}

/// Tersi gevşemez: hazırlık onaylanmadan hazırlık gerekirse (onaydan sonra
/// hazırlık geri alındı) komut başlamaz ve ileti gerçek nedeni söyler.
#[tokio::test]
async fn onaylanmayan_hazirlik_belirirse_komut_baslamaz() {
    let kabuk = Arc::new(HazirlikliKabuk::yeni(true, true));
    let mut d = hazirlikli_surucu(
        &kabuk,
        vec![
            SahteYanit::arac("kabuk", json!({"n": 1})),
            SahteYanit::metin("peki"),
        ],
        Kip::Siki,
    );
    d.gonder(mesaj()).await;
    let oge = onay_bekle(&mut d).await.remove(0);
    let t = oge.kabuk.as_ref().unwrap();
    assert!(!t.alan_hazirligi && !t.ilk_cagri);
    assert!(!oge.yerel_onay);
    // Onaydan sonra hazırlık geri alındı.
    kabuk.hazir.store(false, Ordering::SeqCst);
    d.gonder(Komut::AracOnayi {
        cagri: oge.cagri,
        karar: OnayKarari::Onayla,
        muhur: oge.muhur.clone(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(kabuk.calisma.load(Ordering::SeqCst), 0);
    assert_eq!(kabuk.hazirlama.load(Ordering::SeqCst), 0);
    let sonuclar = sonuclar(&d);
    assert_eq!(sonuclar.len(), 1);
    assert_ne!(sonuclar[0].0, AracDurumu::Basarili);
    assert!(
        !sonuclar[0].1.contains("Dosya"),
        "kabukta dosya değişmedi: {}",
        sonuclar[0].1
    );
    assert!(
        sonuclar[0]
            .1
            .contains("yalıtım hazırlığı onaydan sonra geri alındı"),
        "{}",
        sonuclar[0].1
    );
}
