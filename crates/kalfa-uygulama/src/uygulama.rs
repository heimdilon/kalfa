//! Uygulamanın paylaşılan durumu. Komutlar `Uygulama`'nın yöntemleridir;
//! `kalfa` crate'i her komutu ince bir Tauri sarmalayıcısıyla bunlara bağlar.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use kalfa_core::aktarim::AktarimAyarlari;
use kalfa_core::kilit::KilitYoneticisi;
use kalfa_core::kimlik::OturumId;
use kalfa_core::sart_kapisi::SartKapisi;
use kalfa_plugin::EvrenBaglami;
use kalfa_sandbox::Yalitici;

use crate::anahtar::AnahtarKasasi;
use crate::arayuz::{Acici, Iletisim, Pencereler, Yukseltici};
use crate::calisma_alanlari::CalismaAlaniKaydi;
use crate::dizin;
use crate::eklentiler::EklentiYoneticisi;
use crate::evren::{DegisirSaglayici, SaglayiciUretici};
use crate::modeller;
use crate::oturumlar::{AcikOturum, KapananOturum};
use crate::sozlesme::{AnahtarNedeni, BaglantiDurumu, ModelSecenegi, SartPenceresiDurumu};
use crate::yalitim::{Dinleyici, YalitimAyarlari, YalitimKaynagi, yalitici_kur};

/// Uygulamayı kurmak için gerekenler; Tauri katmanı ya da test verir.
pub struct Bilesenler {
    pub veri_dizini: PathBuf,
    pub kasa: AnahtarKasasi,
    pub uretici: Arc<dyn SaglayiciUretici>,
    pub iletisim: Arc<dyn Iletisim>,
    pub acici: Arc<dyn Acici>,
    pub pencereler: Arc<dyn Pencereler>,
    pub aktarim: AktarimAyarlari,
    /// Kullanıcının ev dizini: araç zinciri dizinleri buna göre bulunur.
    pub ev_dizini: PathBuf,
    /// Çalışma alanına özel dizinlerin kökü (`dizin::yalitim_dizini`).
    pub yalitim_dizini: PathBuf,
    /// Oturumların geri alma verisinin kökü (`dizin::geri_alma_dizini`).
    pub geri_alma_dizini: PathBuf,
    /// Eklenti depolarının kökü (`dizin::eklenti_verisi_dizini`).
    pub eklenti_verisi_dizini: PathBuf,
    /// Yerleşik eklentilerin dizini (`dizin::yerlesik_eklenti_dizini`); yoksa
    /// yerleşik eklenti gösterilmez.
    pub yerlesik_eklentiler: Option<PathBuf>,
    /// Kurulu Kalfa'nın getirdiği Python ve Node (`dizin::kalfa_araclari`).
    pub kalfa_araclari: Vec<PathBuf>,
    pub yalitim: YalitimKaynagi,
    /// "Onar": kurulu yardımcıyı UAC ile çalıştırır (aşama 2 tasarımı §9.8).
    pub yukseltici: Arc<dyn Yukseltici>,
}

pub struct Uygulama {
    pub(crate) oturum_dizini: PathBuf,
    pub(crate) kasa: AnahtarKasasi,
    pub(crate) uretici: Arc<dyn SaglayiciUretici>,
    pub(crate) evren: Arc<DegisirSaglayici>,
    pub(crate) kapi: Arc<SartKapisi>,
    pub(crate) kilitler: KilitYoneticisi,
    pub(crate) iletisim: Arc<dyn Iletisim>,
    pub(crate) acici: Arc<dyn Acici>,
    pub(crate) pencereler: Arc<dyn Pencereler>,
    pub(crate) yukseltici: Arc<dyn Yukseltici>,
    pub(crate) aktarim: AktarimAyarlari,
    pub(crate) veri_dizini: PathBuf,
    pub(crate) ev_dizini: PathBuf,
    pub(crate) geri_alma_dizini: PathBuf,
    /// Kabuk komutlarının başlatan arka uç (aşama 2 tasarımı §3.2).
    pub(crate) yalitici: Arc<dyn Yalitici>,
    pub(crate) yalitim_ayarlari: Arc<YalitimAyarlari>,
    /// `ayarlar.json` (3c Görev 7).
    pub(crate) ayarlar: Arc<crate::ayarlar::Ayarlar>,
    /// Bütün oturumların paylaştığı denetçi: eşzamanlı denetim sınırı uygulama
    /// geneldedir.
    pub(crate) denetci: Arc<dyn kalfa_core::denetci::GuvenlikDenetcisi>,
    pub(crate) yalitim_dinleyici: Dinleyici,
    pub(crate) alanlar: Arc<CalismaAlaniKaydi>,
    /// Eklentiler: `eklentiler.json`, motor, yükleme önbelleği (aşama 4
    /// tasarımı §13).
    pub(crate) eklentiler: EklentiYoneticisi,
    ic: Mutex<IcDurum>,
}

/// Kilitle korunan durum. Kilit hiçbir `await` boyunca tutulmaz.
pub(crate) struct IcDurum {
    pub baglanti: BaglantiDurumu,
    pub modeller: Vec<ModelSecenegi>,
    pub son_oturum: Option<OturumId>,
    pub oturumlar: HashMap<OturumId, AcikOturum>,
    /// Kapatılmış ama görevi henüz bitmemiş oturumlar; yeniden açılmadan önce
    /// beklenir (kayıt kilidi bırakılsın diye).
    pub kapaniyor: HashMap<OturumId, KapananOturum>,
    /// Kaldırma onaylanıp yalıtıcı işlemi bitene kadar oturum açılmasını önler.
    pub yalitim_kaldirma_rezervleri: HashSet<String>,
    pub calisma_alanlari: HashMap<u64, PathBuf>,
    pub son_alan: u64,
    pub sart_penceresi: SartPenceresiDurumu,
    /// Süren şart penceresi komutu sayısı. Bu sırada kapı izleyicisi pencereyi
    /// yeniden yüklemez; komut yeni durumu kendisi döner.
    pub pencere_komutu: u32,
}

impl Uygulama {
    pub fn yeni(b: Bilesenler) -> Arc<Self> {
        let evren = Arc::new(DegisirSaglayici::default());
        let kapi = Arc::new(SartKapisi::yeni(evren.clone()));
        let yalitim_ayarlari = Arc::new(YalitimAyarlari::oku(&b.veri_dizini));
        let ayarlar = Arc::new(crate::ayarlar::Ayarlar::oku(&b.veri_dizini));
        let denetci: Arc<dyn kalfa_core::denetci::GuvenlikDenetcisi> = Arc::new(
            kalfa_core::denetci::YapilandirilmisSiniflandirici::yeni(evren.clone()),
        );
        let yalitim_dinleyici = Dinleyici::default();
        // Uygulama başına bir kez: eklentilerin EVREN yolu, oturumların
        // paylaştığı sağlayıcıyı ve şart kapısını kullanır.
        let eklentiler = EklentiYoneticisi::yeni(
            &b.veri_dizini,
            b.eklenti_verisi_dizini,
            b.yerlesik_eklentiler,
            EvrenBaglami::yeni(evren.clone(), kapi.clone()),
        );
        let yalitici = yalitici_kur(
            b.yalitim,
            &b.yalitim_dizini,
            &b.ev_dizini,
            b.kalfa_araclari,
            &yalitim_ayarlari,
            yalitim_dinleyici.clone(),
        );
        Arc::new(Self {
            oturum_dizini: dizin::oturum_dizini(&b.veri_dizini),
            kasa: b.kasa,
            uretici: b.uretici,
            evren,
            kapi,
            kilitler: KilitYoneticisi::yeni(),
            iletisim: b.iletisim,
            acici: b.acici,
            pencereler: b.pencereler,
            yukseltici: b.yukseltici,
            aktarim: b.aktarim,
            yalitici,
            yalitim_ayarlari,
            ayarlar,
            denetci,
            yalitim_dinleyici,
            alanlar: Arc::new(CalismaAlaniKaydi::yeni(&b.veri_dizini)),
            eklentiler,
            veri_dizini: b.veri_dizini,
            ev_dizini: b.ev_dizini,
            geri_alma_dizini: b.geri_alma_dizini,
            ic: Mutex::new(IcDurum {
                baglanti: BaglantiDurumu::Var,
                modeller: modeller::modeller(),
                son_oturum: None,
                oturumlar: HashMap::new(),
                kapaniyor: HashMap::new(),
                yalitim_kaldirma_rezervleri: HashSet::new(),
                calisma_alanlari: HashMap::new(),
                son_alan: 0,
                sart_penceresi: SartPenceresiDurumu::AnahtarGerekli {
                    neden: AnahtarNedeni::Yok,
                },
                pencere_komutu: 0,
            }),
        })
    }

    pub(crate) fn ic(&self) -> MutexGuard<'_, IcDurum> {
        self.ic.lock().unwrap_or_else(|z| z.into_inner())
    }

    /// Uygulama genelindeki tek şart kapısı.
    pub fn sart_kapisi(&self) -> &Arc<SartKapisi> {
        &self.kapi
    }

    /// Açık oturum sayısı (testler ve tanılama için).
    pub fn acik_oturum_sayisi(&self) -> usize {
        self.ic().oturumlar.len()
    }
}
