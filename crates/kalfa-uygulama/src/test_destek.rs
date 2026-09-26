//! Testler için sahte bileşenler (`test-destek` özelliği). Uygulama koduna
//! girmez.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use kalfa_core::aktarim::{AktarimAyarlari, KanalMesaji};
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::SahteSaglayici;
use kalfa_evren::EvrenError;
use kalfa_sandbox::{YalitimDurumu, Yalitimsiz};
use tokio::sync::{Semaphore, mpsc};
use url::Url;

use crate::anahtar::{AnahtarDeposu, AnahtarKasasi, BellekDeposu};
use crate::arayuz::{Acici, Iletisim, KanalYazici, Pencereler, YalitimYazici};
use crate::evren::SaglayiciUretici;
use crate::yalitim::YalitimKaynagi;
use crate::{Bilesenler, Uygulama};

/// Bu anahtarla üretilen sağlayıcı her isteğe 401 döner.
pub const GECERSIZ_ANAHTAR: &str = "gecersiz";
/// Bu anahtarla üretilen sağlayıcı EVREN'e ulaşamaz.
pub const AGSIZ_ANAHTAR: &str = "agsiz";

/// Anahtara göre sağlayıcı: `GECERSIZ_ANAHTAR` ve `AGSIZ_ANAHTAR` dışındaki
/// her anahtar ortak sahte sağlayıcıyı verir.
pub struct SahteUretici {
    pub saglayici: Arc<SahteSaglayici>,
}

impl SaglayiciUretici for SahteUretici {
    fn uret(&self, anahtar: &str) -> Result<Arc<dyn Saglayici>, EvrenError> {
        let hata = match anahtar {
            GECERSIZ_ANAHTAR => EvrenError::AnahtarGecersiz,
            AGSIZ_ANAHTAR => EvrenError::Ag("bağlantı reddedildi".into()),
            _ => return Ok(self.saglayici.clone()),
        };
        let saglayici = SahteSaglayici::yeni(Vec::new());
        saglayici.sart_durumu_hatasi(Some(hata));
        Ok(saglayici)
    }
}

/// Yerel iletişim kutuları: klasör seçici verilen yolu döner, onay kutusu
/// verilen cevabı verir; sorulan başlık ve mesajlar kaydedilir.
#[derive(Default)]
pub struct SahteIletisim {
    klasor: Mutex<Option<PathBuf>>,
    onay: AtomicBool,
    onay_bekleme: Mutex<Option<Arc<Semaphore>>>,
    sorulanlar: Mutex<Vec<String>>,
}

impl SahteIletisim {
    pub fn klasor_ver(&self, yol: &Path) {
        *self.klasor.lock().unwrap() = Some(yol.to_path_buf());
    }

    pub fn onay_ver(&self, evet: bool) {
        self.onay.store(evet, Ordering::SeqCst);
    }

    /// Onay kutusunu testin açıkça serbest bırakmasına kadar bekletir.
    pub fn onayi_beklet(&self) -> Arc<Semaphore> {
        let izin = Arc::new(Semaphore::new(0));
        *self.onay_bekleme.lock().unwrap() = Some(izin.clone());
        izin
    }

    /// Sorulan onayların "başlık: mesaj" metinleri.
    pub fn sorulanlar(&self) -> Vec<String> {
        self.sorulanlar.lock().unwrap().clone()
    }
}

#[async_trait]
impl Iletisim for SahteIletisim {
    async fn klasor_sec(&self, _baslik: &str) -> Option<PathBuf> {
        self.klasor.lock().unwrap().clone()
    }

    async fn onayla(&self, baslik: &str, mesaj: &str, _tamam: &str, _vazgec: &str) -> bool {
        self.sorulanlar
            .lock()
            .unwrap()
            .push(format!("{baslik}: {mesaj}"));
        let bekleme = self.onay_bekleme.lock().unwrap().clone();
        if let Some(izin) = bekleme {
            let _ = izin.acquire().await;
        }
        self.onay.load(Ordering::SeqCst)
    }
}

#[derive(Default)]
pub struct SahteAcici {
    acilanlar: Mutex<Vec<String>>,
}

impl SahteAcici {
    pub fn acilanlar(&self) -> Vec<String> {
        self.acilanlar.lock().unwrap().clone()
    }
}

impl Acici for SahteAcici {
    fn ac(&self, adres: &Url) -> Result<(), String> {
        self.acilanlar.lock().unwrap().push(adres.to_string());
        Ok(())
    }
}

/// Pencere işlemlerini sırasıyla kaydeder: "ana_ac", "sartlar_goster",
/// "sartlar_kapat".
#[derive(Default)]
pub struct SahtePencereler {
    olaylar: Mutex<Vec<&'static str>>,
}

impl SahtePencereler {
    pub fn olaylar(&self) -> Vec<&'static str> {
        self.olaylar.lock().unwrap().clone()
    }

    pub fn temizle(&self) {
        self.olaylar.lock().unwrap().clear();
    }
}

impl Pencereler for SahtePencereler {
    fn ana_ac(&self) {
        self.olaylar.lock().unwrap().push("ana_ac");
    }

    fn sartlar_goster(&self) {
        self.olaylar.lock().unwrap().push("sartlar_goster");
    }

    fn sartlar_kapat(&self) {
        self.olaylar.lock().unwrap().push("sartlar_kapat");
    }
}

/// Ana pencerenin yalıtım kanalının yerine geçer; yazılan durumları kaydeder.
#[derive(Clone, Default)]
pub struct SahteYalitimYazici(Arc<Mutex<Vec<YalitimDurumu>>>);

impl SahteYalitimYazici {
    pub fn yazilanlar(&self) -> Vec<YalitimDurumu> {
        self.0.lock().unwrap().clone()
    }
}

impl YalitimYazici for SahteYalitimYazici {
    fn gonder(&self, durum: &YalitimDurumu) -> Result<(), String> {
        self.0.lock().unwrap().push(durum.clone());
        Ok(())
    }
}

/// Kanal mesajlarını bir `mpsc` kanalına aktarır.
pub struct SahteKanal {
    tx: mpsc::UnboundedSender<KanalMesaji>,
}

impl SahteKanal {
    pub fn yeni() -> (Box<Self>, mpsc::UnboundedReceiver<KanalMesaji>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Box::new(Self { tx }), rx)
    }
}

impl KanalYazici for SahteKanal {
    fn gonder(&self, mesaj: KanalMesaji) -> Result<(), String> {
        self.tx.send(mesaj).map_err(|e| e.to_string())
    }
}

/// Kalıcı depo gibi davranan ama yazılamayan depo (Secret Service çalışmıyor).
pub struct YazilamayanDepo;

impl AnahtarDeposu for YazilamayanDepo {
    fn oku(&self) -> Result<Option<String>, crate::anahtar::DepoHatasi> {
        Ok(None)
    }

    fn yaz(&self, _anahtar: &str) -> Result<(), crate::anahtar::DepoHatasi> {
        Err(crate::anahtar::DepoHatasi("Secret Service yok".into()))
    }
}

/// Testin de okuyabildiği kalıcı depo.
pub struct PaylasilanDepo(pub Arc<BellekDeposu>);

impl AnahtarDeposu for PaylasilanDepo {
    fn oku(&self) -> Result<Option<String>, crate::anahtar::DepoHatasi> {
        self.0.oku()
    }

    fn yaz(&self, anahtar: &str) -> Result<(), crate::anahtar::DepoHatasi> {
        self.0.yaz(anahtar)
    }
}

/// Sahteleriyle kurulmuş uygulama ve sahtelerine erişim.
pub struct TestUygulamasi {
    pub uygulama: Arc<Uygulama>,
    pub saglayici: Arc<SahteSaglayici>,
    /// Kalıcı depo gibi davranan depo.
    pub depo: Arc<BellekDeposu>,
    pub iletisim: Arc<SahteIletisim>,
    pub acici: Arc<SahteAcici>,
    pub pencereler: Arc<SahtePencereler>,
}

/// `kayitli_anahtar` verilirse kalıcı depoda o anahtar kayıtlıymış gibi
/// başlar. Veri dizini `dizin`dir. Kabuk yalıtımsızdır.
pub fn test_uygulamasi(
    dizin: &Path,
    saglayici: Arc<SahteSaglayici>,
    kayitli_anahtar: Option<&str>,
) -> TestUygulamasi {
    test_uygulamasi_yalitimla(
        dizin,
        saglayici,
        kayitli_anahtar,
        YalitimKaynagi::Kurucu(Box::new(|_| Arc::new(Yalitimsiz))),
    )
}

/// `test_uygulamasi`, verilen yalıtım kaynağıyla. Ev dizini `dizin/ev`dir.
pub fn test_uygulamasi_yalitimla(
    dizin: &Path,
    saglayici: Arc<SahteSaglayici>,
    kayitli_anahtar: Option<&str>,
    yalitim: YalitimKaynagi,
) -> TestUygulamasi {
    let depo = Arc::new(BellekDeposu::default());
    if let Some(anahtar) = kayitli_anahtar {
        depo.yaz(anahtar).unwrap();
    }
    let iletisim = Arc::new(SahteIletisim::default());
    let acici = Arc::new(SahteAcici::default());
    let pencereler = Arc::new(SahtePencereler::default());
    let uygulama = Uygulama::yeni(Bilesenler {
        veri_dizini: dizin.to_path_buf(),
        kasa: AnahtarKasasi::yeni(Some(Box::new(PaylasilanDepo(depo.clone())))),
        uretici: Arc::new(SahteUretici {
            saglayici: saglayici.clone(),
        }),
        iletisim: iletisim.clone(),
        acici: acici.clone(),
        pencereler: pencereler.clone(),
        aktarim: AktarimAyarlari::default(),
        yalitim_dizini: dizin.join("yalitim"),
        ev_dizini: dizin.join("ev"),
        yalitim,
    });
    TestUygulamasi {
        uygulama,
        saglayici,
        depo,
        iletisim,
        acici,
        pencereler,
    }
}
