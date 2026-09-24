//! Aktarım sayaçları (1c tasarımı §11.4, kabul kapısı 2): arayüzün "işledim"
//! demediği veri ve çekirdek olay kuyruğu en çok ne kadar büyüdü?
//!
//! Kuyruk yalnızca paketleyici okuyunca küçülür; bu yüzden her okumadan önce
//! bakılan değer iki okuma arasındaki en yüksek değerdir. Yoldaki veri her
//! paket gönderilirken ölçülür.

use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};

use kalfa_core::aktarim::{KanalMesaji, KrediTutamaci, OlayKaynagi};
use kalfa_core::oturum::{OlayAlici, OlayButcesi};
use kalfa_core::protokol::{Olay, Zarf};
use serde::Serialize;

use crate::Uygulama;
use crate::arayuz::KanalYazici;

static YOLDA_PAKET: AtomicUsize = AtomicUsize::new(0);
static YOLDA_BAYT: AtomicUsize = AtomicUsize::new(0);
static KUYRUK_OLAY: AtomicUsize = AtomicUsize::new(0);
static KUYRUK_BAYT: AtomicUsize = AtomicUsize::new(0);

/// Süreç başladığından (ya da son sıfırlamadan) beri görülen en yüksek
/// değerler ve karşılaştırılacakları sınırlar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Olcumler {
    pub yolda_paket_en_cok: usize,
    pub yolda_bayt_en_cok: usize,
    pub kuyruk_olay_en_cok: usize,
    pub kuyruk_bayt_en_cok: usize,
    pub kredi_paket: usize,
    pub kredi_bayt: usize,
    pub butce_olay: usize,
    pub butce_bayt: usize,
}

pub fn olcumleri_sifirla() {
    for sayac in [&YOLDA_PAKET, &YOLDA_BAYT, &KUYRUK_OLAY, &KUYRUK_BAYT] {
        sayac.store(0, Ordering::SeqCst);
    }
}

impl Uygulama {
    pub fn e2e_olcumler(&self) -> Olcumler {
        let butce = OlayButcesi::default();
        Olcumler {
            yolda_paket_en_cok: YOLDA_PAKET.load(Ordering::SeqCst),
            yolda_bayt_en_cok: YOLDA_BAYT.load(Ordering::SeqCst),
            kuyruk_olay_en_cok: KUYRUK_OLAY.load(Ordering::SeqCst),
            kuyruk_bayt_en_cok: KUYRUK_BAYT.load(Ordering::SeqCst),
            kredi_paket: self.aktarim.kredi_paket,
            kredi_bayt: self.aktarim.kredi_bayt,
            butce_olay: butce.azami_olay(),
            butce_bayt: butce.azami_bayt(),
        }
    }
}

/// Olay kuyruğunu okumadan önce doluluğunu kaydeder.
pub struct OlcumluKaynak(pub OlayAlici);

impl OlayKaynagi for OlcumluKaynak {
    fn recv(&mut self) -> impl Future<Output = Option<Zarf<Olay>>> + Send {
        KUYRUK_OLAY.fetch_max(self.0.bekleyen_olay_sayisi(), Ordering::SeqCst);
        KUYRUK_BAYT.fetch_max(self.0.bekleyen_bayt(), Ordering::SeqCst);
        self.0.recv()
    }
}

/// Paketi kanala yazmadan önce yoldaki veriyi kaydeder. Paketleyici paketi
/// yola, `sonraki` dönmeden ekler.
pub struct OlcumluYazici {
    ic: Box<dyn KanalYazici>,
    kredi: KrediTutamaci,
}

impl OlcumluYazici {
    pub fn yeni(ic: Box<dyn KanalYazici>, kredi: KrediTutamaci) -> Self {
        Self { ic, kredi }
    }
}

impl KanalYazici for OlcumluYazici {
    fn gonder(&self, mesaj: KanalMesaji) -> Result<(), String> {
        let (paket, bayt) = self.kredi.yolda();
        YOLDA_PAKET.fetch_max(paket, Ordering::SeqCst);
        YOLDA_BAYT.fetch_max(bayt, Ordering::SeqCst);
        self.ic.gonder(mesaj)
    }
}
