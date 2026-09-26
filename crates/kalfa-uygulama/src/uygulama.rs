//! Uygulamanın paylaşılan durumu. Komutlar `Uygulama`'nın yöntemleridir;
//! `kalfa` crate'i her komutu ince bir Tauri sarmalayıcısıyla bunlara bağlar.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use kalfa_core::aktarim::AktarimAyarlari;
use kalfa_core::kilit::KilitYoneticisi;
use kalfa_core::kimlik::OturumId;
use kalfa_core::oturum::OturumTutamaci;
use kalfa_core::sart_kapisi::SartKapisi;
use kalfa_sandbox::Yalitici;

use crate::anahtar::AnahtarKasasi;
use crate::arayuz::{Acici, Iletisim, Pencereler};
use crate::calisma_alanlari::CalismaAlaniKaydi;
use crate::dizin;
use crate::evren::{DegisirSaglayici, SaglayiciUretici};
use crate::oturumlar::AcikOturum;
use crate::sozlesme::{AnahtarNedeni, BaglantiDurumu, SartPenceresiDurumu};
use crate::yalitim::{AracZinciriKaydi, Dinleyici, YalitimKaynagi, yalitici_kur};

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
    pub yalitim: YalitimKaynagi,
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
    pub(crate) aktarim: AktarimAyarlari,
    pub(crate) veri_dizini: PathBuf,
    pub(crate) ev_dizini: PathBuf,
    /// Kabuk komutlarını başlatan arka uç (aşama 2 tasarımı §3.2).
    pub(crate) yalitici: Arc<dyn Yalitici>,
    pub(crate) arac_zinciri: AracZinciriKaydi,
    pub(crate) yalitim_dinleyici: Dinleyici,
    pub(crate) alanlar: Arc<CalismaAlaniKaydi>,
    ic: Mutex<IcDurum>,
}

/// Kilitle korunan durum. Kilit hiçbir `await` boyunca tutulmaz.
pub(crate) struct IcDurum {
    pub baglanti: BaglantiDurumu,
    pub son_oturum: Option<OturumId>,
    pub oturumlar: HashMap<OturumId, AcikOturum>,
    /// Kapatılmış ama görevi henüz bitmemiş oturumlar; yeniden açılmadan önce
    /// beklenir (kayıt kilidi bırakılsın diye).
    pub kapaniyor: HashMap<OturumId, OturumTutamaci>,
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
        let arac_zinciri = AracZinciriKaydi::oku(&b.veri_dizini);
        let yalitim_dinleyici = Dinleyici::default();
        let yalitici = yalitici_kur(
            b.yalitim,
            &b.yalitim_dizini,
            &b.ev_dizini,
            &arac_zinciri,
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
            aktarim: b.aktarim,
            yalitici,
            arac_zinciri,
            yalitim_dinleyici,
            alanlar: Arc::new(CalismaAlaniKaydi::yeni(&b.veri_dizini)),
            veri_dizini: b.veri_dizini,
            ev_dizini: b.ev_dizini,
            ic: Mutex::new(IcDurum {
                baglanti: BaglantiDurumu::Var,
                son_oturum: None,
                oturumlar: HashMap::new(),
                kapaniyor: HashMap::new(),
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
