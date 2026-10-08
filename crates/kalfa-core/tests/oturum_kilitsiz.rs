//! Kilitsiz araç (aşama 4 tasarımı §8.3, karar 5): eklenti aracı çağrı
//! boyunca kilit tutmaz; aynı adımdaki, bütün çalışma alanını kilitleyen bir
//! iş (kabuk) onu beklemeden çalışır.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, Hazirlik};
use kalfa_core::izin::Kip;
use kalfa_core::oturum::OturumAyarlari;
use kalfa_core::protokol::{BitisNedeni, Komut};
use kalfa_core::test_destek::{
    SahteSaglayici, SahteYanit, sanal_siniflandirici, test_baglami, test_oturumu,
};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

/// Öteki aracın çalışmasını 3 sn'ye kadar bekler.
struct Bekleyen {
    kilitsiz: bool,
    isaret: Arc<AtomicBool>,
}

/// Bütün çalışma alanını kilitler (kabuk gibi) ve işareti koyar.
struct Genel {
    isaret: Arc<AtomicBool>,
}

#[async_trait]
impl Arac for Bekleyen {
    fn ad(&self) -> &str {
        "bekleyen"
    }
    fn aciklama(&self) -> &str {
        "bekler"
    }
    fn parametre_semasi(&self) -> Value {
        json!({"type": "object"})
    }
    fn yazar_mi(&self) -> bool {
        false
    }
    async fn hazirla(&self, _: &Value) -> Result<Hazirlik, AracHatasi> {
        Ok(Hazirlik {
            kilitsiz: self.kilitsiz,
            ..Default::default()
        })
    }
    async fn calistir(
        &self,
        _: &Value,
        _: &Hazirlik,
        _: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        for _ in 0..300 {
            if self.isaret.load(Ordering::SeqCst) {
                return Ok(AracCiktisi {
                    icerik: "öteki araç çalıştı".into(),
                    ..Default::default()
                });
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Err(AracHatasi::Gecersiz("öteki araç beklendi".into()))
    }
}

#[async_trait]
impl Arac for Genel {
    fn ad(&self) -> &str {
        "genel"
    }
    fn aciklama(&self) -> &str {
        "bütün alanı kilitler"
    }
    fn parametre_semasi(&self) -> Value {
        json!({"type": "object"})
    }
    fn yazar_mi(&self) -> bool {
        false
    }
    async fn hazirla(&self, _: &Value) -> Result<Hazirlik, AracHatasi> {
        Ok(Hazirlik {
            tum_calisma_alani: true,
            ..Default::default()
        })
    }
    async fn calistir(
        &self,
        _: &Value,
        _: &Hazirlik,
        _: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.isaret.store(true, Ordering::SeqCst);
        Ok(AracCiktisi::default())
    }
}

async fn calistir(kilitsiz: bool) -> String {
    let isaret = Arc::new(AtomicBool::new(false));
    let araclar: Vec<Arc<dyn Arac>> = vec![
        Arc::new(Bekleyen {
            kilitsiz,
            isaret: isaret.clone(),
        }),
        Arc::new(Genel { isaret }),
    ];
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![("bekleyen", json!({})), ("genel", json!({}))]),
        SahteYanit::metin("tamam"),
    ]);
    let mut ayarlar = OturumAyarlari::yeni("glm-5.3", &std::env::temp_dir());
    ayarlar.kip = Kip::Serbest;
    let mut d = test_oturumu(
        test_baglami(s.clone(), araclar, sanal_siniflandirici()),
        ayarlar,
    )
    .zaman_asimi(Duration::from_secs(10));
    d.gonder(Komut::KullaniciMesaji { metin: "x".into() }).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    format!("{:?}", s.istekler())
}

#[tokio::test]
async fn kilitsiz_arac_tum_alani_kilitleyeni_bekletmez() {
    let istekler = calistir(true).await;
    assert!(istekler.contains("öteki araç çalıştı"), "{istekler}");
}

/// Karşılaştırma: kilit tutan araç genel okuma kilidini çağrı boyunca
/// tutar; bütün alanı kilitleyen iş onun bitmesini bekler.
#[tokio::test]
async fn kilitli_arac_tum_alani_kilitleyeni_bekletir() {
    let istekler = calistir(false).await;
    assert!(istekler.contains("öteki araç beklendi"), "{istekler}");
}
