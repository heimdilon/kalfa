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
        assert_eq!(icerik, "[kaynak: kabuk:test]\nÇıkış kodu: 1");
    }
}
