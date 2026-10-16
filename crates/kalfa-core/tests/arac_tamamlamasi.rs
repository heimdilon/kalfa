use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, AracTamamlama, Hazirlik};
use kalfa_core::kimlik::CagriId;
use serde_json::Value;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

struct Kayit {
    kilitli: AtomicBool,
    tamamlandi: AtomicUsize,
    iptal_edildi: AtomicUsize,
    basladi: AtomicBool,
    hata: bool,
    bekle: Option<Arc<Notify>>,
    cleanup_bekle: Option<Arc<Notify>>,
    cleanup_bitti: AtomicBool,
}

struct TamamlamaAraci(Arc<Kayit>);

#[async_trait]
impl Arac for TamamlamaAraci {
    fn ad(&self) -> &str {
        "mcp__deneme__arac"
    }

    fn aciklama(&self) -> &str {
        "tamamlama kancası testi"
    }

    fn parametre_semasi(&self) -> Value {
        serde_json::json!({"type": "object"})
    }

    fn yazar_mi(&self) -> bool {
        true
    }

    async fn hazirla(&self, _argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
        Ok(Hazirlik::default())
    }

    async fn calistir(
        &self,
        _argumanlar: &Value,
        _hazirlik: &Hazirlik,
        _iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        Ok(AracCiktisi::default())
    }

    async fn sonuc_isleme_bitti(&self, _cagri: CagriId) -> Result<(), AracHatasi> {
        self.0.basladi.store(true, Ordering::SeqCst);
        if let Some(bekle) = &self.0.bekle {
            bekle.notified().await;
        }
        if self.0.hata {
            return Err(AracHatasi::Gecersiz("gizli manager ayrıntısı".into()));
        }
        self.0.tamamlandi.fetch_add(1, Ordering::SeqCst);
        self.0.kilitli.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn sonuc_isleme_iptal(&self, _cagri: CagriId) {
        self.0.iptal_edildi.fetch_add(1, Ordering::SeqCst);
    }

    async fn sonuc_isleme_iptal_ve_bekle(&self, _cagri: CagriId) -> Result<(), AracHatasi> {
        self.0.iptal_edildi.fetch_add(1, Ordering::SeqCst);
        self.0.basladi.store(true, Ordering::SeqCst);
        if let Some(bekle) = &self.0.cleanup_bekle {
            bekle.notified().await;
        }
        self.0.cleanup_bitti.store(true, Ordering::SeqCst);
        self.0.kilitli.store(false, Ordering::SeqCst);
        Ok(())
    }
}

fn kayit(hata: bool, bekle: Option<Arc<Notify>>) -> Arc<Kayit> {
    Arc::new(Kayit {
        kilitli: AtomicBool::new(true),
        tamamlandi: AtomicUsize::new(0),
        iptal_edildi: AtomicUsize::new(0),
        basladi: AtomicBool::new(false),
        hata,
        bekle,
        cleanup_bekle: None,
        cleanup_bitti: AtomicBool::new(false),
    })
}

#[tokio::test]
async fn eskimis_onayda_yeniden_planlama_cleanup_bitene_kadar_bekler() {
    let bekle = Arc::new(Notify::new());
    let mut k = kayit(false, None);
    Arc::get_mut(&mut k).unwrap().cleanup_bekle = Some(bekle.clone());
    let gorev = tokio::spawn(guard(&k).iptal_ve_bekle());

    while !k.basladi.load(Ordering::SeqCst) {
        tokio::task::yield_now().await;
    }
    assert!(!k.cleanup_bitti.load(Ordering::SeqCst));
    assert!(k.kilitli.load(Ordering::SeqCst));
    assert_eq!(k.iptal_edildi.load(Ordering::SeqCst), 1);

    bekle.notify_one();
    gorev.await.unwrap().unwrap();
    assert!(k.cleanup_bitti.load(Ordering::SeqCst));
    assert!(!k.kilitli.load(Ordering::SeqCst));
}

fn guard(kayit: &Arc<Kayit>) -> AracTamamlama {
    let arac: Arc<dyn Arac> = Arc::new(TamamlamaAraci(kayit.clone()));
    AracTamamlama::yeni(arac, CagriId::yeni())
}

#[tokio::test]
async fn tamamlanan_cagri_son_denetime_gore_kilidi_cozer_drop_iptal_hooku_cagirir() {
    let k = kayit(false, None);
    guard(&k).tamamla().await.unwrap();
    assert_eq!(k.tamamlandi.load(Ordering::SeqCst), 1);
    assert_eq!(k.iptal_edildi.load(Ordering::SeqCst), 0);
    assert!(!k.kilitli.load(Ordering::SeqCst));

    let k = kayit(false, None);
    drop(guard(&k));
    assert_eq!(k.iptal_edildi.load(Ordering::SeqCst), 1);
    assert!(k.kilitli.load(Ordering::SeqCst));
}

#[tokio::test]
async fn tamamlanma_futurei_birakilinca_cleanup_hooku_cagrilir() {
    let k = kayit(true, Some(Arc::new(Notify::new())));
    let gorev = tokio::spawn(guard(&k).tamamla());
    while !k.basladi.load(Ordering::SeqCst) {
        tokio::task::yield_now().await;
    }

    gorev.abort();
    let _ = gorev.await;
    assert_eq!(k.iptal_edildi.load(Ordering::SeqCst), 1);
    assert_eq!(k.tamamlandi.load(Ordering::SeqCst), 0);
    assert!(k.kilitli.load(Ordering::SeqCst));
}

#[tokio::test]
async fn tamamlanma_hatasi_ayrintiyi_sizdirmaz_ve_kilit_kaydi_korunur() {
    let k = kayit(true, None);
    let hata = guard(&k).tamamla().await.unwrap_err();

    assert_eq!(hata, AracHatasi::SonucIsleme);
    assert!(!hata.to_string().contains("gizli manager ayrıntısı"));
    assert_eq!(k.iptal_edildi.load(Ordering::SeqCst), 0);
    assert!(k.kilitli.load(Ordering::SeqCst));
}
