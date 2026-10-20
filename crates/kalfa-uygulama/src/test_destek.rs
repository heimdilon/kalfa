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
use crate::arayuz::{Acici, Iletisim, KanalYazici, Pencereler, YalitimYazici, Yukseltici};
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
            AGSIZ_ANAHTAR => EvrenError::ag_baglanti("bağlantı reddedildi"),
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
    dosya: Mutex<Option<PathBuf>>,
    /// Son klasör seçicinin başlangıç dizini.
    baslangic: Mutex<Option<PathBuf>>,
    onay: AtomicBool,
    onay_bekleme: Mutex<Option<Arc<Semaphore>>>,
    sorulanlar: Mutex<Vec<String>>,
}

impl SahteIletisim {
    pub fn klasor_ver(&self, yol: &Path) {
        *self.klasor.lock().unwrap() = Some(yol.to_path_buf());
    }

    pub fn dosya_ver(&self, yol: &Path) {
        *self.dosya.lock().unwrap() = Some(yol.to_path_buf());
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

    /// Son açılan klasör seçicinin başlangıç dizini.
    pub fn son_baslangic(&self) -> Option<PathBuf> {
        self.baslangic.lock().unwrap().clone()
    }
}

#[async_trait]
impl Iletisim for SahteIletisim {
    async fn klasor_sec(&self, _baslik: &str, baslangic: Option<&Path>) -> Option<PathBuf> {
        *self.baslangic.lock().unwrap() = baslangic.map(Path::to_path_buf);
        self.klasor.lock().unwrap().clone()
    }

    async fn dosya_sec(&self, _baslik: &str, baslangic: Option<&Path>) -> Option<PathBuf> {
        *self.baslangic.lock().unwrap() = baslangic.map(Path::to_path_buf);
        self.dosya.lock().unwrap().clone()
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

/// "Onar"ın sahtesi: kurulu sürüm gibi davranır, `kur` ve `izin_ver` verilen
/// sonucu döner; çağrıları ve argümanlarını kaydeder.
pub struct SahteYukseltici {
    pub kurulu: AtomicBool,
    pub sonuc: Mutex<Result<Option<u32>, String>>,
    pub cagri: std::sync::atomic::AtomicUsize,
    /// `kur`'a verilen dizin listeleri, çağrı sırasıyla.
    pub kurulanlar: Mutex<Vec<Vec<PathBuf>>>,
    /// `izin_ver`'e verilen dizinler.
    pub izin_verilenler: Mutex<Vec<PathBuf>>,
}

impl Default for SahteYukseltici {
    fn default() -> Self {
        Self {
            kurulu: AtomicBool::new(true),
            sonuc: Mutex::new(Ok(Some(0))),
            cagri: Default::default(),
            kurulanlar: Default::default(),
            izin_verilenler: Default::default(),
        }
    }
}

impl SahteYukseltici {
    pub fn cagri_sayisi(&self) -> usize {
        self.cagri.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Yukseltici for SahteYukseltici {
    fn kurulu_mu(&self) -> bool {
        self.kurulu.load(Ordering::SeqCst)
    }

    async fn kur(&self, izinli: &[PathBuf]) -> Result<Option<u32>, String> {
        self.cagri.fetch_add(1, Ordering::SeqCst);
        self.kurulanlar.lock().unwrap().push(izinli.to_vec());
        self.sonuc.lock().unwrap().clone()
    }

    async fn izin_ver(&self, dizin: &Path) -> Result<Option<u32>, String> {
        self.cagri.fetch_add(1, Ordering::SeqCst);
        self.izin_verilenler
            .lock()
            .unwrap()
            .push(dizin.to_path_buf());
        self.sonuc.lock().unwrap().clone()
    }
}

#[derive(Default)]
pub struct SahteAcici {
    acilanlar: Mutex<Vec<String>>,
    #[allow(clippy::type_complexity)]
    kanca: Mutex<Option<Arc<dyn Fn(&Url) + Send + Sync>>>,
}

impl SahteAcici {
    pub fn acilanlar(&self) -> Vec<String> {
        self.acilanlar.lock().unwrap().clone()
    }

    pub fn kanca_ayarla(&self, f: impl Fn(&Url) + Send + Sync + 'static) {
        *self.kanca.lock().unwrap() = Some(Arc::new(f));
    }
}

impl Acici for SahteAcici {
    fn ac(&self, adres: &Url) -> Result<(), String> {
        self.acilanlar.lock().unwrap().push(adres.to_string());
        if let Some(kanca) = self.kanca.lock().unwrap().clone() {
            kanca(adres);
        }
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
    pub yukseltici: Arc<SahteYukseltici>,
}

/// `kayitli_anahtar` verilirse kalıcı depoda o anahtar kayıtlıymış gibi
/// başlar. Veri dizini `dizin`dir. Kabuk yalıtımsızdır ve yalıtımsız
/// çalıştırma anahtarı açık başlar (aşama 2 tasarımı §9.8); veri dizininde
/// `yalitim.json` varsa ona dokunulmaz.
pub fn test_uygulamasi(
    dizin: &Path,
    saglayici: Arc<SahteSaglayici>,
    kayitli_anahtar: Option<&str>,
) -> TestUygulamasi {
    test_uygulamasi_yerlesikli(dizin, saglayici, kayitli_anahtar, None)
}

/// `test_uygulamasi`, yerleşik eklentilerin dizini verilerek (aşama 4
/// tasarımı §13.1): her eklenti `<dizin>/<ad>/` altında `kalfa.toml` ve
/// `eklenti.wasm`'dır.
pub fn test_uygulamasi_yerlesikli(
    dizin: &Path,
    saglayici: Arc<SahteSaglayici>,
    kayitli_anahtar: Option<&str>,
    yerlesik_eklentiler: Option<PathBuf>,
) -> TestUygulamasi {
    let ayar = dizin.join("yalitim.json");
    if !ayar.exists() {
        std::fs::create_dir_all(dizin).unwrap();
        std::fs::write(&ayar, r#"{"yalitimsiz_izinli": true}"#).unwrap();
    }
    kur(
        dizin,
        saglayici,
        kayitli_anahtar,
        YalitimKaynagi::Kurucu(Box::new(|_| Arc::new(Yalitimsiz))),
        yerlesik_eklentiler,
    )
}

/// `test_uygulamasi`, verilen yalıtım kaynağıyla. Ev dizini `dizin/ev`dir.
pub fn test_uygulamasi_yalitimla(
    dizin: &Path,
    saglayici: Arc<SahteSaglayici>,
    kayitli_anahtar: Option<&str>,
    yalitim: YalitimKaynagi,
) -> TestUygulamasi {
    kur(dizin, saglayici, kayitli_anahtar, yalitim, None)
}

fn kur(
    dizin: &Path,
    saglayici: Arc<SahteSaglayici>,
    kayitli_anahtar: Option<&str>,
    yalitim: YalitimKaynagi,
    yerlesik_eklentiler: Option<PathBuf>,
) -> TestUygulamasi {
    let depo = Arc::new(BellekDeposu::default());
    if let Some(anahtar) = kayitli_anahtar {
        depo.yaz(anahtar).unwrap();
    }
    let iletisim = Arc::new(SahteIletisim::default());
    let acici = Arc::new(SahteAcici::default());
    let pencereler = Arc::new(SahtePencereler::default());
    let yukseltici = Arc::new(SahteYukseltici::default());
    let mcp_kasa: Arc<dyn crate::mcp::kasa::GizliKasa> =
        Arc::new(crate::mcp::kasa::test_destek::SahteGizliKasa::yeni());
    let uygulama = Uygulama::yeni_ozel_mcp_kasali(
        Bilesenler {
            veri_dizini: dizin.to_path_buf(),
            kasa: AnahtarKasasi::yeni(Some(Box::new(PaylasilanDepo(depo.clone())))),
            uretici: Arc::new(SahteUretici {
                saglayici: saglayici.clone(),
            }),
            iletisim: iletisim.clone(),
            acici: acici.clone(),
            pencereler: pencereler.clone(),
            yukseltici: yukseltici.clone(),
            aktarim: AktarimAyarlari::default(),
            yalitim_dizini: dizin.join("yalitim"),
            geri_alma_dizini: dizin.join("geri-alma"),
            eklenti_verisi_dizini: dizin.join("eklenti-verisi"),
            yerlesik_eklentiler,
            kalfa_araclari: Vec::new(),
            ev_dizini: dizin.join("ev"),
            yalitim,
        },
        mcp_kasa,
    );
    // MCP tanımı PATH'in her dizinini çözer; CI'da testleri koşan kullanıcı
    // yöneticinin PATH'ini taşır ve erişemediği dizin tanımı kapatır.
    uygulama.mcp.sistem_ortamini_degistir(vec![(
        "PATH".to_owned(),
        dizin.to_string_lossy().into_owned(),
    )]);
    TestUygulamasi {
        uygulama,
        saglayici,
        depo,
        iletisim,
        acici,
        pencereler,
        yukseltici,
    }
}
