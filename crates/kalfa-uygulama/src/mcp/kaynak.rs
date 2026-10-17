//! Çalışma alanı başına sürümlü MCP araç yayımı ve tanım denetimlerinin
//! sahiplenilmesi (Görev 8).
//!
//! Keşfi biten sunucunun denetimi `denetim_bekliyor` ile bekletilir. Denetim
//! yalnız aktif turu olan bir oturum `denetim_islerini_al` ile sahiplenince,
//! o oturumun bütçesiyle başlar. Aynı anahtar için aynı anda tek sahip olur.
//! Sahip turu biterse ya da iş sonuç vermezse kayıt bekleyene döner ve
//! sıradaki uygun oturum sahiplenir. Yayım değişmezdir; yeni sürüm bir sonraki
//! model turunun görüntüsüne girer.

use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, Weak};

use kalfa_core::arac::{AracDenetimIsi, AracDenetimSahibi, AracKaynagi, KaynakAraci};
use kalfa_core::kimlik::OturumId;

/// Sunucunun tanım denetimi. `None`: izin yok, iptal ya da hata; hiçbir şey
/// yayımlanmaz ve kayıt yeniden sahiplenilebilir.
pub type McpDenetimIsi = Arc<
    dyn Fn(AracDenetimSahibi) -> Pin<Box<dyn Future<Output = Option<Vec<KaynakAraci>>> + Send>>
        + Send
        + Sync,
>;

struct Bekleyen {
    nesil: u64,
    is: McpDenetimIsi,
    sahip: Option<OturumId>,
    /// Bu kaydı sahiplenen oturumlar, sırayla.
    denemeler: Vec<OturumId>,
}

#[derive(Default)]
struct Durum {
    yayim: BTreeMap<String, Vec<KaynakAraci>>,
    hazir: Option<Arc<[KaynakAraci]>>,
    surum: u64,
    bekleyenler: BTreeMap<String, Bekleyen>,
    nesil: u64,
}

impl Durum {
    fn yeniden_yayimla(&mut self) {
        let hazir: Vec<KaynakAraci> = self.yayim.values().flatten().cloned().collect();
        self.hazir = Some(hazir.into());
        self.surum += 1;
    }
}

pub struct McpAlanAracKaynagi {
    ben: Weak<Self>,
    durum: Mutex<Durum>,
}

impl McpAlanAracKaynagi {
    pub fn yeni() -> Arc<Self> {
        Arc::new_cyclic(|ben| Self {
            ben: ben.clone(),
            durum: Mutex::default(),
        })
    }

    fn durum(&self) -> MutexGuard<'_, Durum> {
        self.durum.lock().unwrap_or_else(|z| z.into_inner())
    }

    /// `anahtar` sunucusunun denetimini bekletir. Aynı anahtarın eski
    /// bekleyeni ve sürmekte olan işinin sonucu geçersiz olur.
    pub fn denetim_bekliyor(&self, anahtar: &str, is: McpDenetimIsi) {
        let mut d = self.durum();
        d.nesil += 1;
        let nesil = d.nesil;
        let denemeler = d
            .bekleyenler
            .remove(anahtar)
            .map(|b| b.denemeler)
            .unwrap_or_default();
        d.bekleyenler.insert(
            anahtar.to_string(),
            Bekleyen {
                nesil,
                is,
                sahip: None,
                denemeler,
            },
        );
    }

    /// Sunucunun yayımını ve bekleyen denetimini kaldırır.
    pub fn kaldir(&self, anahtar: &str) {
        let mut d = self.durum();
        d.bekleyenler.remove(anahtar);
        if d.yayim.remove(anahtar).is_some() {
            d.yeniden_yayimla();
        }
    }

    pub fn surum(&self) -> u64 {
        self.durum().surum
    }

    /// Bekleyen denetimi sahiplenen oturumlar.
    pub fn denemeler(&self, anahtar: &str) -> Vec<OturumId> {
        self.durum()
            .bekleyenler
            .get(anahtar)
            .map(|b| b.denemeler.clone())
            .unwrap_or_default()
    }

    /// İş bittiğinde ya da bırakıldığında sahipliği kapatır. Sonuç yalnız
    /// kayıt hâlâ aynı nesildeyse ve sahibin turu sürüyorsa yayımlanır.
    fn sahipligi_bitir(&self, anahtar: &str, nesil: u64, araclar: Option<Vec<KaynakAraci>>) {
        let mut d = self.durum();
        let Some(bekleyen) = d.bekleyenler.get_mut(anahtar) else {
            return;
        };
        if bekleyen.nesil != nesil {
            return;
        }
        match araclar {
            Some(araclar) => {
                d.bekleyenler.remove(anahtar);
                d.yayim.insert(anahtar.to_string(), araclar);
                d.yeniden_yayimla();
            }
            None => bekleyen.sahip = None,
        }
    }
}

/// Bırakılan (panik, çalışma zamanı kapanışı) iş de sahipliği geri verir.
struct Sahiplik {
    kaynak: Weak<McpAlanAracKaynagi>,
    anahtar: String,
    nesil: u64,
    bitti: bool,
}

impl Sahiplik {
    fn bitir(mut self, araclar: Option<Vec<KaynakAraci>>) {
        self.bitti = true;
        if let Some(kaynak) = self.kaynak.upgrade() {
            kaynak.sahipligi_bitir(&self.anahtar, self.nesil, araclar);
        }
    }
}

impl Drop for Sahiplik {
    fn drop(&mut self) {
        if !self.bitti
            && let Some(kaynak) = self.kaynak.upgrade()
        {
            kaynak.sahipligi_bitir(&self.anahtar, self.nesil, None);
        }
    }
}

impl AracKaynagi for McpAlanAracKaynagi {
    fn hazir_araclar(&self) -> Arc<[KaynakAraci]> {
        self.durum().hazir.clone().unwrap_or_else(|| Arc::from([]))
    }

    fn denetim_islerini_al(&self, sahip: AracDenetimSahibi) -> Vec<AracDenetimIsi> {
        let mut d = self.durum();
        let mut isler: Vec<AracDenetimIsi> = Vec::new();
        for (anahtar, bekleyen) in d.bekleyenler.iter_mut() {
            if bekleyen.sahip.is_some() {
                continue;
            }
            bekleyen.sahip = Some(sahip.oturum);
            bekleyen.denemeler.push(sahip.oturum);
            let sahiplik = Sahiplik {
                kaynak: self.ben.clone(),
                anahtar: anahtar.clone(),
                nesil: bekleyen.nesil,
                bitti: false,
            };
            let is = bekleyen.is.clone();
            let sahip = sahip.clone();
            isler.push(Box::pin(async move {
                let iptal = sahip.iptal.clone();
                let araclar = tokio::select! {
                    biased;
                    _ = iptal.cancelled() => None,
                    araclar = is(sahip) => araclar,
                };
                // İş biterken tur kapandıysa sonuç yayımlanmaz.
                sahiplik.bitir(araclar.filter(|_| !iptal.is_cancelled()));
            }));
        }
        isler
    }
}

/// Uygulama genelinde alan kimliği başına tek kaynak.
#[derive(Default)]
pub struct McpAracKaynaklari {
    alanlar: Mutex<HashMap<String, Arc<McpAlanAracKaynagi>>>,
}

impl McpAracKaynaklari {
    pub fn al(&self, alan_kimligi: &str) -> Arc<McpAlanAracKaynagi> {
        self.alanlar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .entry(alan_kimligi.to_string())
            .or_insert_with(McpAlanAracKaynagi::yeni)
            .clone()
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, Hazirlik, async_trait};
    use kalfa_core::butce::ButceTalepci;
    use kalfa_sandbox::tarama::CancellationToken;
    use serde_json::{Value, json};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Notify;

    struct Okuyan(&'static str);

    #[async_trait]
    impl Arac for Okuyan {
        fn ad(&self) -> &str {
            self.0
        }
        fn aciklama(&self) -> &str {
            "okur"
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
            _: CancellationToken,
        ) -> Result<AracCiktisi, AracHatasi> {
            Ok(AracCiktisi::default())
        }
    }

    fn sahip() -> AracDenetimSahibi {
        let (butce, _alici) = ButceTalepci::kanal();
        AracDenetimSahibi {
            oturum: OturumId::yeni(),
            butce,
            iptal: CancellationToken::new(),
        }
    }

    fn kaynak_araci(ad: &'static str) -> KaynakAraci {
        KaynakAraci::yeni(Arc::new(Okuyan(ad))).unwrap()
    }

    /// Her çağrıda sayaç artırır; `kapi` açılana kadar bekler, sonra `sonuc`'u verir.
    fn is(
        sayac: Arc<AtomicUsize>,
        kapi: Arc<Notify>,
        sonuc: Option<&'static str>,
    ) -> McpDenetimIsi {
        Arc::new(move |_sahip| {
            let sayac = sayac.clone();
            let kapi = kapi.clone();
            Box::pin(async move {
                sayac.fetch_add(1, Ordering::SeqCst);
                kapi.notified().await;
                sonuc.map(|ad| vec![kaynak_araci(ad)])
            })
        })
    }

    fn adlar(kaynak: &McpAlanAracKaynagi) -> Vec<String> {
        kaynak
            .hazir_araclar()
            .iter()
            .map(|a| a.ad().to_string())
            .collect()
    }

    #[tokio::test]
    async fn ayni_anahtar_icin_tek_sahip_ve_tamamlaninca_yayim() {
        let kaynak = McpAlanAracKaynagi::yeni();
        let sayac = Arc::new(AtomicUsize::new(0));
        let kapi = Arc::new(Notify::new());
        kaynak.denetim_bekliyor("s", is(sayac.clone(), kapi.clone(), Some("mcp__s__a")));

        let (a, b) = (sahip(), sahip());
        let isler_a = kaynak.denetim_islerini_al(a.clone());
        let isler_b = kaynak.denetim_islerini_al(b);
        assert_eq!(isler_a.len(), 1);
        assert!(isler_b.is_empty(), "ikinci oturum aynı denetimi açmaz");
        assert_eq!(kaynak.denemeler("s"), vec![a.oturum]);

        let gorev = tokio::spawn(isler_a.into_iter().next().unwrap());
        while sayac.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
        assert!(adlar(&kaynak).is_empty(), "denetim bitmeden yayım yok");
        let surum = kaynak.surum();
        kapi.notify_one();
        gorev.await.unwrap();
        assert_eq!(adlar(&kaynak), vec!["mcp__s__a"]);
        assert_eq!(kaynak.surum(), surum + 1);
        assert!(kaynak.denetim_islerini_al(sahip()).is_empty());
    }

    #[tokio::test]
    async fn sahibin_turu_biterse_yayim_yok_ve_siradaki_sahiplenir() {
        let kaynak = McpAlanAracKaynagi::yeni();
        let sayac = Arc::new(AtomicUsize::new(0));
        let kapi = Arc::new(Notify::new());
        kaynak.denetim_bekliyor("s", is(sayac.clone(), kapi.clone(), Some("mcp__s__a")));

        let a = sahip();
        let is_a = kaynak.denetim_islerini_al(a.clone()).pop().unwrap();
        let gorev = tokio::spawn(is_a);
        while sayac.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
        a.iptal.cancel();
        gorev.await.unwrap();
        assert!(
            adlar(&kaynak).is_empty(),
            "kapanan sahibin sonucu yayımlanmaz"
        );

        let b = sahip();
        let is_b = kaynak.denetim_islerini_al(b.clone()).pop().unwrap();
        assert_eq!(kaynak.denemeler("s"), vec![a.oturum, b.oturum]);
        let gorev = tokio::spawn(is_b);
        while sayac.load(Ordering::SeqCst) < 2 {
            tokio::task::yield_now().await;
        }
        kapi.notify_one();
        gorev.await.unwrap();
        assert_eq!(adlar(&kaynak), vec!["mcp__s__a"]);
    }

    #[tokio::test]
    async fn sonucsuz_ya_da_birakilan_is_kaydi_geri_verir() {
        let kaynak = McpAlanAracKaynagi::yeni();
        let sayac = Arc::new(AtomicUsize::new(0));
        let kapi = Arc::new(Notify::new());
        kaynak.denetim_bekliyor("s", is(sayac.clone(), kapi.clone(), None));

        // Bütçe reddi gibi sonuçsuz iş.
        let is_a = kaynak.denetim_islerini_al(sahip()).pop().unwrap();
        kapi.notify_one();
        is_a.await;
        assert!(adlar(&kaynak).is_empty());

        // Hiç çalışmadan bırakılan iş de sahipliği geri verir.
        drop(kaynak.denetim_islerini_al(sahip()));
        assert_eq!(kaynak.denetim_islerini_al(sahip()).len(), 1);
    }

    #[tokio::test]
    async fn yeni_nesil_ve_kaldirma_eski_sonucu_gecersiz_kilar() {
        let kaynak = McpAlanAracKaynagi::yeni();
        let sayac = Arc::new(AtomicUsize::new(0));
        let kapi = Arc::new(Notify::new());
        kaynak.denetim_bekliyor("s", is(sayac.clone(), kapi.clone(), Some("mcp__s__eski")));
        let eski = tokio::spawn(kaynak.denetim_islerini_al(sahip()).pop().unwrap());
        while sayac.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
        kaynak.denetim_bekliyor("s", is(sayac.clone(), kapi.clone(), Some("mcp__s__yeni")));
        kapi.notify_one();
        eski.await.unwrap();
        assert!(adlar(&kaynak).is_empty(), "eski nesil yayımlanmaz");

        let yeni = tokio::spawn(kaynak.denetim_islerini_al(sahip()).pop().unwrap());
        while sayac.load(Ordering::SeqCst) < 2 {
            tokio::task::yield_now().await;
        }
        kapi.notify_one();
        yeni.await.unwrap();
        assert_eq!(adlar(&kaynak), vec!["mcp__s__yeni"]);

        kaynak.kaldir("s");
        assert!(adlar(&kaynak).is_empty());
    }

    #[test]
    fn alan_basina_tek_kaynak() {
        let kaynaklar = McpAracKaynaklari::default();
        assert!(Arc::ptr_eq(&kaynaklar.al("a"), &kaynaklar.al("a")));
        assert!(!Arc::ptr_eq(&kaynaklar.al("a"), &kaynaklar.al("b")));
    }
}
