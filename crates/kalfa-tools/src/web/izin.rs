//! `web_getir` aracı için izin tarifi, oturum içi host defteri ve mühür özeti.
//!
//! Üst tasarım ve Yetenek-05 planı uyarınca:
//! - `Kip::Inceleme` altında dış ağ isteği her zaman reddedilir (`Karar::Yasak`).
//! - `Kip::Serbest` (otomatik/serbest kip) dahil hiçbir kipte `web_getir`
//!   sessizce otomatik onaylanmaz; ilk host erişimi, şifresiz HTTP veya
//!   yönlendirme kapsamı genişlemesi yerel/kullanıcı onayı gerektirir.
//! - Mühür özeti (`web_muhur_ozeti`) yöntemi, şemayı, hostu, portu, kanonik
//!   URL'yi ve yönlendirme kapsamını birbirine bağlar.

use std::collections::HashSet;
use std::sync::Mutex;

pub use kalfa_core::arac::{WebTarifi, YonlendirmeIzni, web_muhur_ozeti};
#[cfg(test)]
use kalfa_core::izin::{Karar, Kip};
pub use kalfa_core::izin::{
    WEB_AG_UYARISI, WEB_ILK_HOST_UYARISI, WEB_SIFRESIZ_HTTP_UYARISI, WEB_YONLENDIRME_UYARISI,
    web_karari,
};

/// Oturum boyunca kullanıcı onayıyla erişilmiş `(host, port)` çiftlerini tutan defter.
#[derive(Debug, Default)]
pub struct WebIzinDefteri {
    onayli_hostlar: Mutex<HashSet<(String, u16)>>,
}

impl WebIzinDefteri {
    /// Boş bir oturum web izin defteri oluşturur.
    pub fn yeni() -> Self {
        Self {
            onayli_hostlar: Mutex::new(HashSet::new()),
        }
    }

    /// Verilen `(host, port)` çifti bu oturumda daha önce onaylandı mı?
    pub fn host_onayli_mi(&self, host: &str, port: u16) -> bool {
        let anahtar = (host.trim().to_ascii_lowercase(), port);
        match self.onayli_hostlar.lock() {
            Ok(kilit) => kilit.contains(&anahtar),
            Err(zehirli) => zehirli.into_inner().contains(&anahtar),
        }
    }

    /// Başarılı ve onaylı bir erişim sonrasında `(host, port)` çiftini kaydeder.
    pub fn host_onayla(&self, host: &str, port: u16) {
        let anahtar = (host.trim().to_ascii_lowercase(), port);
        match self.onayli_hostlar.lock() {
            Ok(mut kilit) => {
                kilit.insert(anahtar);
            }
            Err(zehirli) => {
                zehirli.into_inner().insert(anahtar);
            }
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn ornek_tarif() -> WebTarifi {
        WebTarifi {
            kanonik_url: "https://belge.ornek.test/kullanim".into(),
            maskeli_url: "https://belge.ornek.test/kullanim".into(),
            host: "belge.ornek.test".into(),
            port: 443,
            sema: "https".into(),
            yontem: "GET".into(),
            yonlendirme_izni: YonlendirmeIzni::AyniHost,
            oturumda_ilk_host: true,
        }
    }

    #[test]
    fn inceleme_kipi_reddeder_serbest_kip_otomatik_onaylamaz() {
        let tarif = ornek_tarif();
        assert!(matches!(
            web_karari(Kip::Inceleme, &tarif),
            Karar::Yasak { .. }
        ));

        let mut ikinci_ziyaret = tarif.clone();
        ikinci_ziyaret.oturumda_ilk_host = false;
        assert_ne!(
            web_karari(Kip::Serbest, &ikinci_ziyaret),
            Karar::Serbest,
            "Serbest kipte dahi web_getir otomatik onaylanmamalı"
        );
    }

    #[test]
    fn muhur_ozeti_host_port_url_ve_yonlendirme_degisince_farklilasir() {
        let t1 = ornek_tarif();
        let mut t2 = t1.clone();
        t2.kanonik_url = "https://belge.ornek.test/baska".into();
        assert_ne!(web_muhur_ozeti(&t1), web_muhur_ozeti(&t2));

        let mut t3 = t1.clone();
        t3.yonlendirme_izni = YonlendirmeIzni::IzinliHostlar(vec!["ikinci.ornek.test".into()]);
        assert_ne!(web_muhur_ozeti(&t1), web_muhur_ozeti(&t3));
    }
}
