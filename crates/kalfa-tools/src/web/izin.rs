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

use kalfa_core::izin::{Karar, Kip};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Dış ağ erişimi uyarısı.
pub const WEB_AG_UYARISI: &str = "Bu işlem dış bir web adresine HTTP/HTTPS isteği gönderecek.";

/// Oturumda ilk kez erişilen web hostu uyarısı.
pub const WEB_ILK_HOST_UYARISI: &str = "Bu web hostuna bu oturumda ilk kez erişilecek.";

/// Farklı bir hosta yönlendirme izni uyarısı.
pub const WEB_YONLENDIRME_UYARISI: &str = "Bu istek farklı bir hosta yönlendirmeye izin veriyor.";

/// Şifresiz `http://` bağlantısı uyarısı.
pub const WEB_SIFRESIZ_HTTP_UYARISI: &str =
    "Bağlantı şifresiz HTTP kullanıyor; ağ trafiği üçüncü taraflarca görülebilir.";

/// Yönlendirme sırasında hangi hedef hostlara geçiş yapılabileceğini belirler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum YonlendirmeIzni {
    /// Yalnızca ilk istekteki aynı hosta yönlendirmeye izin verilir (varsayılan).
    AyniHost,
    /// İlk hosta ek olarak açıkça onaylanmış hedef host listesine izin verilir.
    IzinliHostlar(Vec<String>),
}

impl YonlendirmeIzni {
    /// Verilen `hedef_host`, `baslangic_host` veya onaylı listede mi?
    pub fn host_izinli_mi(&self, baslangic_host: &str, hedef_host: &str) -> bool {
        let baslangic = baslangic_host.trim().to_ascii_lowercase();
        let hedef = hedef_host.trim().to_ascii_lowercase();
        if baslangic == hedef {
            return true;
        }
        match self {
            Self::AyniHost => false,
            Self::IzinliHostlar(liste) => liste
                .iter()
                .any(|izinli| izinli.trim().eq_ignore_ascii_case(&hedef)),
        }
    }
}

/// Bir `web_getir` çağrısının onay ve mühürlemeye esas tarifi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebTarifi {
    /// Fragment (`#...`) arındırılmış kanonik istek URL'si.
    pub kanonik_url: String,
    /// Günlük ve onay penceresi için hassas sorgu parametreleri maskelenmiş URL.
    pub maskeli_url: String,
    /// Küçük harfe normalize edilmiş hedef host adı.
    pub host: String,
    /// Etkin hedef port (ör. HTTPS için 443, HTTP için 80).
    pub port: u16,
    /// URL şeması (`"https"` veya `"http"`).
    pub sema: String,
    /// HTTP yöntemi (`"GET"`).
    pub yontem: String,
    /// Yönlendirme kapsamı.
    pub yonlendirme_izni: YonlendirmeIzni,
    /// Bu `(host, port)` çiftine mevcut oturumda ilk kez mi çıkılıyor?
    pub oturumda_ilk_host: bool,
}

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

/// Oturum kipi ve [`WebTarifi`] için izin kararını hesaplar.
///
/// - `Kip::Inceleme`: her zaman `Karar::Yasak`.
/// - `Kip::Siki`, `Kip::Proje`, `Kip::Serbest`: `web_getir` hiçbir kipte
///   otomatik (`Karar::Serbest`) çalışmaz; her zaman onay (`Karar::Onayli` veya
///   ilk host / şifresiz HTTP / yönlendirme genişlemesinde `Karar::YerelOnayli`)
///   ister.
pub fn web_karari(kip: Kip, tarif: &WebTarifi) -> Karar {
    if kip == Kip::Inceleme {
        return Karar::Yasak {
            neden: "İnceleme kipinde dış ağ isteği (`web_getir`) çalıştırılamaz.".into(),
        };
    }

    if tarif.sema != "https" && tarif.sema != "http" {
        return Karar::Yasak {
            neden: format!("Desteklenmeyen web şeması: {}", tarif.sema),
        };
    }

    if tarif.yontem != "GET" {
        return Karar::Yasak {
            neden: format!("Yalnızca GET yöntemine izin verilir: {}", tarif.yontem),
        };
    }

    let mut uyarilar = vec![WEB_AG_UYARISI.to_string()];
    let mut yerel_onay_gerekli = false;

    if tarif.oturumda_ilk_host {
        uyarilar.push(WEB_ILK_HOST_UYARISI.to_string());
        yerel_onay_gerekli = true;
    }
    if tarif.sema == "http" {
        uyarilar.push(WEB_SIFRESIZ_HTTP_UYARISI.to_string());
        yerel_onay_gerekli = true;
    }
    if let YonlendirmeIzni::IzinliHostlar(ek_hostlar) = &tarif.yonlendirme_izni
        && !ek_hostlar.is_empty()
    {
        uyarilar.push(WEB_YONLENDIRME_UYARISI.to_string());
        yerel_onay_gerekli = true;
    }

    if yerel_onay_gerekli || kip == Kip::Serbest {
        Karar::YerelOnayli { uyarilar }
    } else {
        Karar::Onayli { uyarilar }
    }
}

fn uzunluklu_yaz(ozetleyici: &mut Sha256, alan: &[u8]) {
    ozetleyici.update((alan.len() as u64).to_le_bytes());
    ozetleyici.update(alan);
}

/// [`WebTarifi`] için tek yönlü, alan sınırları ayrılmış SHA-256 mühür özeti üretir.
pub fn web_muhur_ozeti(tarif: &WebTarifi) -> [u8; 32] {
    let mut ozetleyici = Sha256::new();
    uzunluklu_yaz(&mut ozetleyici, b"kalfa-web-muhur-v1");
    uzunluklu_yaz(&mut ozetleyici, tarif.yontem.as_bytes());
    uzunluklu_yaz(&mut ozetleyici, tarif.sema.as_bytes());
    uzunluklu_yaz(&mut ozetleyici, tarif.host.as_bytes());
    ozetleyici.update(tarif.port.to_le_bytes());
    uzunluklu_yaz(&mut ozetleyici, tarif.kanonik_url.as_bytes());
    match &tarif.yonlendirme_izni {
        YonlendirmeIzni::AyniHost => {
            uzunluklu_yaz(&mut ozetleyici, b"ayni-host");
        }
        YonlendirmeIzni::IzinliHostlar(hostlar) => {
            uzunluklu_yaz(&mut ozetleyici, b"izinli-hostlar");
            let mut sirali: Vec<String> = hostlar
                .iter()
                .map(|h| h.trim().to_ascii_lowercase())
                .collect();
            sirali.sort();
            sirali.dedup();
            ozetleyici.update((sirali.len() as u64).to_le_bytes());
            for h in &sirali {
                uzunluklu_yaz(&mut ozetleyici, h.as_bytes());
            }
        }
    }
    ozetleyici.finalize().into()
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
