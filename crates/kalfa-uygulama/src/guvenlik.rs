//! Pencere sınırlarının Tauri'siz kuralları (1c tasarımı §6).

use kalfa_core::izin::Kip;
use kalfa_core::kabuk::YalitimOzeti;
use kalfa_core::protokol::OnayOgesi;
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

/// Yükseltme onayında hedef kipin ne getirdiği (üst belge §7.5, aşama 2
/// tasarımı §5.1). Yalıtımsız komut her kipte yerel pencerede onay ister.
pub fn yukseltme_aciklamasi(yeni: Kip) -> &'static str {
    match yeni {
        Kip::Inceleme => "",
        Kip::Siki => {
            "Kalfa dosya yazmayı, düzenlemeyi ve komut çalıştırmayı önerebilecek; her biri onayınıza \
             sunulur."
        }
        Kip::Proje => {
            "Kalfa çalışma alanı içindeki dosyaları onay sormadan yazabilecek. Korumalı yollar, \
             gizli dosyalar, çalışma alanı dışındaki yollar ve komutlar yine onay ister."
        }
        Kip::Serbest => {
            "Kalfa çalışma alanı içindeki dosyaları onay sormadan yazabilecek; yalıtım varsa \
             yalıtımlı ve ağsız komutları da onay sormadan çalıştırabilecek. Serbest kip yalnızca bu \
             oturum boyunca geçerlidir. Korumalı yollar, gizli dosyalar, çalışma alanı dışındaki \
             yollar, ağlı ve yalıtımsız komutlar yine onay ister."
        }
    }
}

/// Yalıtımsız komutun yerel onay metni (aşama 2 tasarımı §5.2). Komut
/// çekirdeğin bekleyen onayından gelir.
pub fn yerel_onay_metni(ogeler: &[OnayOgesi]) -> String {
    let mut metin = String::from(
        "Aşağıdaki komut yalıtımsız çalışacak: senin bütün yetkilerinle çalışır, ev dizinindeki \
         dosyaları okuyabilir ve ağa erişebilir.\n",
    );
    let mut talimat = None;
    for o in ogeler {
        let Some(k) = &o.kabuk else { continue };
        metin.push_str(&format!("\nKomut: {}\nDizin: {}\n", k.komut, k.dizin));
        if k.ag {
            metin.push_str("Model ağ erişimi istedi.\n");
        }
        if let YalitimOzeti::Yalitimsiz { talimat: t } = &k.yalitim {
            talimat.get_or_insert(t.clone());
        }
    }
    if let Some(t) = talimat {
        metin.push_str(&format!("\n{t}"));
    }
    metin
}
