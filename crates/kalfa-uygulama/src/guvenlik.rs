//! Pencere sınırlarının Tauri'siz kuralları (1c tasarımı §6).

use kalfa_core::izin::Kip;
use url::Url;

/// Pencereler yalnızca uygulamanın kendi sayfalarına gidebilir. Derlenmiş
/// uygulamanın adresi Windows'ta `http://tauri.localhost`, Linux'ta
/// `tauri://localhost`'tur; geliştirmede Vite sunucusunun adresi de izinlidir.
pub fn gezinme_izinli(url: &Url, gelistirme_adresi: Option<&Url>) -> bool {
    match url.scheme() {
        "tauri" => url.host_str() == Some("localhost"),
        "http" | "https" => {
            url.host_str() == Some("tauri.localhost")
                || gelistirme_adresi.is_some_and(|g| g.origin() == url.origin())
        }
        _ => false,
    }
}

/// `baglanti_ac`'ın açabileceği adres: yalnızca mutlak `http` ve `https`
/// (1c tasarımı §6, §9.1). Arayüz de aynı denetimi yapar; bu ikinci katmandır.
pub fn acilabilir_adres(adres: &str) -> Option<Url> {
    let url = Url::parse(adres).ok()?;
    (matches!(url.scheme(), "http" | "https") && url.host().is_some()).then_some(url)
}

fn sira(kip: Kip) -> u8 {
    match kip {
        Kip::Inceleme => 0,
        Kip::Siki => 1,
        Kip::Proje => 2,
        Kip::Serbest => 3,
    }
}

/// İnceleme < Sıkı < Proje < Serbest; sağa her geçiş yükseltmedir ve yerel
/// iletişim kutusunda onaylanır. Kipi düşürmek onay istemez.
pub fn yukseltme_mi(eski: Kip, yeni: Kip) -> bool {
    sira(yeni) > sira(eski)
}

pub fn kip_adi(kip: Kip) -> &'static str {
    match kip {
        Kip::Inceleme => "İnceleme",
        Kip::Siki => "Sıkı",
        Kip::Proje => "Proje",
        Kip::Serbest => "Serbest",
    }
}

/// Yükseltme onayında hedef kipin ne getirdiği (üst belge §7.5). 1c'de kabuk
/// aracı yok; Proje ile Serbest dosya yazmada aynı davranır.
pub fn yukseltme_aciklamasi(yeni: Kip) -> &'static str {
    match yeni {
        Kip::Inceleme => "",
        Kip::Siki => {
            "Kalfa dosya yazmayı ve düzenlemeyi önerebilecek; her yazma onayınıza sunulur."
        }
        Kip::Proje => {
            "Kalfa çalışma alanı içindeki dosyaları onay sormadan yazabilecek. Korumalı yollar, \
             gizli dosyalar ve çalışma alanı dışındaki yollar yine onay ister."
        }
        Kip::Serbest => {
            "Kalfa çalışma alanı içindeki dosyaları onay sormadan yazabilecek. Serbest kip yalnızca \
             bu oturum boyunca geçerlidir. Korumalı yollar, gizli dosyalar ve çalışma alanı dışındaki \
             yollar yine onay ister."
        }
    }
}
