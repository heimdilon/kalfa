//! Görev 8: model turunun tek araç görüntüsü ve tanım denetimlerinin bütçe
//! olay akışı.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kalfa_core::arac::{
    Arac, AracCiktisi, AracDenetimIsi, AracDenetimSahibi, AracHatasi, AracKayitDefteri,
    AracKaynagi, Hazirlik, KaynakAraci,
};
use kalfa_core::gonderim::GunlukToken;
use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::OturumId;
use kalfa_core::oturum::OturumAyarlari;
use kalfa_core::protokol::{BitisNedeni, Komut};
use kalfa_core::test_destek::*;
use kalfa_evren::EvrenError;
use kalfa_evren::tipler::{ChatRequest, Role, ToolDef};
use serde_json::{Value, json};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

const MCP: &str = "mcp__s__a";

/// Çağrıları sayan, tanımı sabit sarmalayıcı.
struct Sayan {
    ad: &'static str,
    aciklama: &'static str,
    sema: Value,
    sayac: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl Arac for Sayan {
    fn ad(&self) -> &str {
        self.ad
    }
    fn aciklama(&self) -> &str {
        self.aciklama
    }
    fn parametre_semasi(&self) -> Value {
        self.sema.clone()
    }
    fn yazar_mi(&self) -> bool {
        false
    }
    async fn hazirla(&self, _: &Value) -> Result<Hazirlik, AracHatasi> {
        Ok(Hazirlik::default())
    }
    async fn calistir(
        &self,
        _: &Value,
        _: &Hazirlik,
        _: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.sayac.fetch_add(1, Ordering::SeqCst);
        Ok(AracCiktisi {
            icerik: format!("{} çalıştı", self.aciklama),
            ..Default::default()
        })
    }
}

fn surum(aciklama: &'static str, alan: &str) -> (KaynakAraci, Arc<AtomicUsize>) {
    let sayac = Arc::new(AtomicUsize::new(0));
    let arac = Sayan {
        ad: MCP,
        aciklama,
        sema: json!({
            "type": "object",
            "properties": {alan: {"type": "string"}},
            "required": [alan],
            "additionalProperties": false,
        }),
        sayac: sayac.clone(),
    };
    (KaynakAraci::yeni(Arc::new(arac)).unwrap(), sayac)
}

type IsUretici = Arc<dyn Fn(AracDenetimSahibi) -> AracDenetimIsi + Send + Sync>;

/// Her okumada sıradaki sürümü veren kaynak: görüntü kaynağı yeniden
/// okursa test farklı sürümü görür.
#[derive(Default)]
struct TestKaynagi {
    surumler: Mutex<VecDeque<Vec<KaynakAraci>>>,
    son: Mutex<Vec<KaynakAraci>>,
    okuma: AtomicUsize,
    is: Mutex<Option<IsUretici>>,
    sahipli: AtomicBool,
    sahipler: Mutex<Vec<OturumId>>,
}

impl TestKaynagi {
    fn sonraki_surum(&self, araclar: Vec<KaynakAraci>) {
        self.surumler.lock().unwrap().push_back(araclar);
    }

    fn sahipler(&self) -> Vec<OturumId> {
        self.sahipler.lock().unwrap().clone()
    }
}

impl AracKaynagi for TestKaynagi {
    fn hazir_araclar(&self) -> Arc<[KaynakAraci]> {
        self.okuma.fetch_add(1, Ordering::SeqCst);
        let mut son = self.son.lock().unwrap();
        if let Some(yeni) = self.surumler.lock().unwrap().pop_front() {
            *son = yeni;
        }
        son.clone().into()
    }

    fn denetim_islerini_al(&self, sahip: AracDenetimSahibi) -> Vec<AracDenetimIsi> {
        let Some(is) = self.is.lock().unwrap().clone() else {
            return Vec::new();
        };
        if self.sahipli.swap(true, Ordering::SeqCst) {
            return Vec::new();
        }
        self.sahipler.lock().unwrap().push(sahip.oturum);
        vec![is(sahip)]
    }
}

fn ayarlar(kip: Kip, kayit: Option<PathBuf>) -> OturumAyarlari {
    let mut ayar = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    ayar.kip = kip;
    ayar.kayit_yolu = kayit;
    ayar
}

fn oturum(
    saglayici: Arc<SahteSaglayici>,
    statik: Vec<Arc<dyn Arac>>,
    kaynak: Arc<TestKaynagi>,
    ayar: OturumAyarlari,
) -> Surucu {
    let mut baglam = test_baglami(saglayici, Vec::new(), sanal_siniflandirici());
    baglam.araclar = Arc::new(AracKayitDefteri::yeni_kaynakli(statik, kaynak).unwrap());
    test_oturumu(baglam, ayar)
}

fn mesaj() -> Komut {
    Komut::KullaniciMesaji {
        metin: "aracı kullan".into(),
    }
}

fn aciklamalar(istek: &ChatRequest) -> Vec<String> {
    istek
        .tools
        .iter()
        .filter(|t| t.function.name == MCP)
        .map(|t| t.function.description.clone())
        .collect()
}

fn arac_yanitlari(istek: &ChatRequest) -> Vec<String> {
    istek
        .messages
        .iter()
        .filter(|m| m.role == Role::Tool)
        .filter_map(|m| m.content.clone())
        .collect()
}

#[tokio::test]
async fn tur_ortasinda_degisen_kaynak_cagriyi_eski_goruntuden_ayirmaz() {
    let dizin = tempfile::tempdir().unwrap();
    let kayit = dizin.path().join("oturum.jsonl");
    let (v1, a) = surum("v1", "x");
    let (v2, b) = surum("v2", "y");
    let kaynak = Arc::new(TestKaynagi::default());
    kaynak.sonraki_surum(vec![v1]);
    kaynak.sonraki_surum(vec![v2]);
    let saglayici = SahteSaglayici::yeni(vec![
        SahteYanit::arac(MCP, json!({"x": "a"})),
        SahteYanit::metin("tamam"),
    ]);
    let mut surucu = oturum(
        saglayici.clone(),
        Vec::new(),
        kaynak.clone(),
        ayarlar(Kip::Serbest, Some(kayit.clone())),
    );
    surucu.gonder(mesaj()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    // Kaynak her model turunda bir kez okunur: planlama yeniden okusaydı v2'nin
    // `y` şeması `x` argümanını reddeder ya da B sarmalayıcısı çalışırdı.
    assert_eq!(kaynak.okuma.load(Ordering::SeqCst), 2);
    assert_eq!((a.load(Ordering::SeqCst), b.load(Ordering::SeqCst)), (1, 0));
    let istekler = saglayici.istekler();
    assert_eq!(aciklamalar(&istekler[0]), vec!["v1"]);
    assert!(arac_yanitlari(&istekler[1])[0].contains("v1 çalıştı"));
    assert_eq!(
        aciklamalar(&istekler[1]),
        vec!["v2"],
        "sonraki tur yeni sürümü görür"
    );

    // Gönderim günlüğü her istekte modele giden tanımla aynıdır.
    let gunluk: Vec<Vec<ToolDef>> = OturumKaydi::oku(&kayit)
        .unwrap()
        .into_iter()
        .filter_map(|z| match z.yuk {
            KayitOgesi::AracTanimlari { tanimlar } => Some(tanimlar),
            _ => None,
        })
        .collect();
    assert_eq!(
        gunluk,
        vec![istekler[0].tools.clone(), istekler[1].tools.clone()]
    );
}

#[tokio::test]
async fn hiz_siniri_tekrari_ayni_goruntuyu_kullanir() {
    let (v1, _) = surum("v1", "x");
    let (v2, _) = surum("v2", "y");
    let kaynak = Arc::new(TestKaynagi::default());
    kaynak.sonraki_surum(vec![v1]);
    kaynak.sonraki_surum(vec![v2]);
    let saglayici = SahteSaglayici::yeni(vec![
        SahteYanit::Hata(EvrenError::HizSiniri {
            tekrar_dene: Some(Duration::from_millis(1)),
        }),
        SahteYanit::metin("tamam"),
    ]);
    let mut surucu = oturum(
        saglayici.clone(),
        Vec::new(),
        kaynak.clone(),
        ayarlar(Kip::Serbest, None),
    );
    surucu.gonder(mesaj()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let istekler = saglayici.istekler();
    assert_eq!(istekler.len(), 2);
    assert_eq!(aciklamalar(&istekler[0]), vec!["v1"]);
    assert_eq!(istekler[0].tools, istekler[1].tools);
    assert_eq!(kaynak.okuma.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn inceleme_kipinde_kaynak_araci_ne_sunulur_ne_calisir_ne_denetlenir() {
    let (v1, a) = surum("v1", "x");
    let kaynak = Arc::new(TestKaynagi::default());
    kaynak.sonraki_surum(vec![v1]);
    let is_sayaci = Arc::new(AtomicUsize::new(0));
    let sayac = is_sayaci.clone();
    *kaynak.is.lock().unwrap() = Some(Arc::new(move |_| {
        sayac.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {})
    }));
    let saglayici = SahteSaglayici::yeni(vec![
        SahteYanit::arac(MCP, json!({"x": "a"})),
        SahteYanit::metin("tamam"),
    ]);
    let mut surucu = oturum(
        saglayici.clone(),
        Vec::new(),
        kaynak.clone(),
        ayarlar(Kip::Inceleme, None),
    );
    surucu.gonder(mesaj()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let istekler = saglayici.istekler();
    assert!(aciklamalar(&istekler[0]).is_empty());
    assert_eq!(a.load(Ordering::SeqCst), 0);
    assert!(arac_yanitlari(&istekler[1])[0].contains("adlı bir araç yok"));
    assert_eq!(kaynak.okuma.load(Ordering::SeqCst), 0);
    assert_eq!(is_sayaci.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn bitmeyen_denetim_turu_bekletmez_ve_bostayken_sahiplenilmez() {
    let kaynak = Arc::new(TestKaynagi::default());
    let iptal_goruldu = Arc::new(Notify::new());
    let goruldu = iptal_goruldu.clone();
    *kaynak.is.lock().unwrap() = Some(Arc::new(move |sahip: AracDenetimSahibi| {
        let goruldu = goruldu.clone();
        Box::pin(async move {
            // Keşif hiç bitmez; yalnız turun iptalini görür.
            sahip.iptal.cancelled().await;
            goruldu.notify_one();
        })
    }));
    let saglayici = SahteSaglayici::yeni(vec![SahteYanit::metin("tamam")]);
    let mut surucu = oturum(
        saglayici.clone(),
        Vec::new(),
        kaynak.clone(),
        ayarlar(Kip::Serbest, None),
    );
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(
        kaynak.sahipler().is_empty(),
        "boştaki oturum denetim başlatmaz"
    );

    surucu.gonder(mesaj()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(kaynak.sahipler(), vec![surucu.tutamac.oturum_id()]);
    assert!(aciklamalar(&saglayici.istekler()[0]).is_empty());
    tokio::time::timeout(Duration::from_secs(5), iptal_goruldu.notified())
        .await
        .expect("tur bitince denetimin belirteci iptal edilmeli");
}

/// Denetim işi: oturumun bütçesinden izin alır, denemeyi bitirir ve yalnız
/// izin verilmişse `araclar`'ı yayımlar.
fn butceli_is(
    kaynak: &Arc<TestKaynagi>,
    araclar: Vec<KaynakAraci>,
    bitti: Arc<Notify>,
) -> IsUretici {
    let kaynak = Arc::downgrade(kaynak);
    Arc::new(move |sahip: AracDenetimSahibi| {
        let (kaynak, araclar, bitti) = (kaynak.clone(), araclar.clone(), bitti.clone());
        Box::pin(async move {
            let Some(kaynak) = kaynak.upgrade() else {
                return;
            };
            match sahip.butce.izin("mcp-tanim").await {
                Some(islem) => {
                    sahip.butce.bitir(islem, Some(GunlukToken::default())).await;
                    kaynak.sonraki_surum(araclar);
                }
                None => kaynak.sahipli.store(false, Ordering::SeqCst),
            }
            bitti.notify_one();
        })
    })
}

/// Denetim yayımı bitene kadar bekleyen statik araç.
struct Bekle(Arc<Notify>);

#[async_trait::async_trait]
impl Arac for Bekle {
    fn ad(&self) -> &str {
        "bekle"
    }
    fn aciklama(&self) -> &str {
        "denetimi bekler"
    }
    fn parametre_semasi(&self) -> Value {
        json!({"type": "object"})
    }
    fn yazar_mi(&self) -> bool {
        false
    }
    async fn hazirla(&self, _: &Value) -> Result<Hazirlik, AracHatasi> {
        Ok(Hazirlik::default())
    }
    async fn calistir(
        &self,
        _: &Value,
        _: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        tokio::select! {
            _ = iptal.cancelled() => Err(AracHatasi::Iptal),
            _ = tokio::time::sleep(Duration::from_secs(5)) => Err(AracHatasi::Iptal),
            _ = self.0.notified() => Ok(AracCiktisi::default()),
        }
    }
}

#[tokio::test]
async fn denetim_butcesi_tur_icinde_islenir_ve_arac_sonraki_model_turunda_gorunur() {
    let (v1, a) = surum("v1", "x");
    let kaynak = Arc::new(TestKaynagi::default());
    let bitti = Arc::new(Notify::new());
    *kaynak.is.lock().unwrap() = Some(butceli_is(&kaynak, vec![v1], bitti.clone()));
    let saglayici = SahteSaglayici::yeni(vec![
        SahteYanit::arac("bekle", json!({})),
        SahteYanit::arac(MCP, json!({"x": "a"})),
        SahteYanit::metin("tamam"),
    ]);
    let ayar = ayarlar(Kip::Serbest, None);
    let mut surucu = oturum(
        saglayici.clone(),
        vec![Arc::new(Bekle(bitti))],
        kaynak.clone(),
        ayar,
    );
    surucu.gonder(mesaj()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let istekler = saglayici.istekler();
    assert!(
        aciklamalar(&istekler[0]).is_empty(),
        "ilk istek denetimi beklemez"
    );
    assert_eq!(aciklamalar(&istekler[1]), vec!["v1"]);
    assert_eq!(a.load(Ordering::SeqCst), 1);
    assert_eq!(kaynak.sahipler(), vec![surucu.tutamac.oturum_id()]);
}
