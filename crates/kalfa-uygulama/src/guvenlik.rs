//! Pencere sınırlarının Tauri'siz kuralları (1c tasarımı §6).

use kalfa_core::gorunum::gorunur_yap;
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

fn yalitimsiz_var(ogeler: &[OnayOgesi]) -> bool {
    ogeler
        .iter()
        .any(|o| o.kabuk.as_ref().is_some_and(|k| !k.yalitim.yalitimli_mi()))
}

/// Yerel onay penceresinin başlığı: yalıtımsız komut varsa onu, yoksa
/// çalışma alanının hazırlığını (Windows, aşama 2 tasarımı §9.4) söyler.
pub fn yerel_onay_basligi(ogeler: &[OnayOgesi]) -> &'static str {
    if yalitimsiz_var(ogeler) {
        "Komutu yalıtımsız çalıştır"
    } else {
        "Çalışma alanını yalıtım için hazırla"
    }
}

/// Yerel kutuda gösterilen komutun sınırı; fazlası onay sayfasındadır.
const KOMUT_AZAMI_KARAKTER: usize = 2000;
const KOMUT_AZAMI_SATIR: usize = 30;

/// Komut ve dizin yerel kutuda görünmez karakterler kaçışlanarak ve uzunsa
/// kısaltılarak gösterilir (güvenlik analizi Bulgu 7).
fn yerel_gosterim(metin: &str) -> String {
    let gorunur = gorunur_yap(metin);
    let satirlar: Vec<&str> = gorunur.lines().collect();
    if gorunur.chars().count() <= KOMUT_AZAMI_KARAKTER && satirlar.len() <= KOMUT_AZAMI_SATIR {
        return gorunur;
    }
    let mut kisa: String = satirlar[..satirlar.len().min(KOMUT_AZAMI_SATIR)]
        .join("\n")
        .chars()
        .take(KOMUT_AZAMI_KARAKTER)
        .collect();
    kisa.push_str("\n…(kısaltıldı; tamamı onay sayfasında)");
    kisa
}

/// Yerel onay metni (aşama 2 tasarımı §5.2, §9.4). Komut çekirdeğin bekleyen
/// onayından gelir.
pub fn yerel_onay_metni(ogeler: &[OnayOgesi]) -> String {
    let mut metin = String::new();
    if yalitimsiz_var(ogeler) {
        metin.push_str(
            "Aşağıdaki komut yalıtımsız çalışacak: senin bütün yetkilerinle çalışır, ev \
             dizinindeki dosyaları okuyabilir ve ağa erişebilir.\n",
        );
    }
    let hazirlik = ogeler
        .iter()
        .any(|o| o.kabuk.as_ref().is_some_and(|k| k.alan_hazirligi));
    if hazirlik {
        metin.push_str(
            "Kalfa bu çalışma alanını yalıtım için hazırlayacak: klasörün izinlerine yalıtım için \
             giriş ekler, gizli dosyaları ve .git'i yalıtıma kapatır. Klasörü ayarlardan \
             kaldırınca geri alınır. Projenin betikleri (build.rs, package.json, Makefile) kod \
             olarak çalışır.\n",
        );
    }
    let mut talimat = None;
    for o in ogeler {
        let Some(k) = &o.kabuk else { continue };
        metin.push_str(&format!(
            "\nKomut: {}\nDizin: {}\n",
            yerel_gosterim(&k.komut),
            yerel_gosterim(&k.dizin)
        ));
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

#[cfg(test)]
mod tests {
    use kalfa_core::kabuk::{KabukBicimi, KabukTarifi};
    use kalfa_core::kimlik::CagriId;
    use kalfa_core::muhur::OnayMuhru;

    use super::*;

    fn oge(yalitim: YalitimOzeti, alan_hazirligi: bool) -> OnayOgesi {
        OnayOgesi {
            cagri: CagriId::yeni(),
            arac: "kabuk".into(),
            aciklama: String::new(),
            hedefler: Vec::new(),
            diff: None,
            uyarilar: Vec::new(),
            muhur: OnayMuhru("m".into()),
            kabuk: Some(KabukTarifi {
                komut: "cargo test".into(),
                bicim: KabukBicimi::Program,
                dizin: ".".into(),
                ag: false,
                ag_hostlari: Vec::new(),
                yalitim,
                ilk_cagri: true,
                alan_hazirligi,
            }),
            yerel_onay: true,
            onaysiz_degisiklikler: None,
            geri_alinabilir: None,
            veri_cikisi: None,
        }
    }

    fn yalitimli() -> YalitimOzeti {
        YalitimOzeti::Yalitimli {
            arka_uc: "windows".into(),
            profil: "p".into(),
        }
    }

    fn yalitimsiz() -> YalitimOzeti {
        YalitimOzeti::Yalitimsiz {
            talimat: "Yalıtımı kurun.".into(),
        }
    }

    #[test]
    fn hazirlik_penceresi_izin_degisikligini_soyler() {
        let ogeler = [oge(yalitimli(), true)];
        assert_eq!(
            yerel_onay_basligi(&ogeler),
            "Çalışma alanını yalıtım için hazırla"
        );
        let metin = yerel_onay_metni(&ogeler);
        assert!(
            metin.contains("izinlerine yalıtım için giriş ekler"),
            "{metin}"
        );
        assert!(metin.contains("Komut: cargo test"), "{metin}");
        assert!(!metin.contains("yalıtımsız"), "{metin}");
    }

    #[test]
    fn yerel_onay_metni_kacislar_ve_kisaltir() {
        let mut o = oge(yalitimsiz(), false);
        if let Some(k) = o.kabuk.as_mut() {
            k.komut = "echo \u{202E}txt.exe".into();
            k.dizin = "alt\u{200B}".into();
        }
        let metin = yerel_onay_metni(std::slice::from_ref(&o));
        assert!(metin.contains("Komut: echo ⟨U+202E⟩txt.exe"), "{metin}");
        assert!(metin.contains("Dizin: alt⟨U+200B⟩"), "{metin}");
        assert!(!metin.contains('\u{202E}'));

        if let Some(k) = o.kabuk.as_mut() {
            k.komut = (0..40)
                .map(|i| format!("echo {i}"))
                .collect::<Vec<_>>()
                .join("\n");
        }
        let metin = yerel_onay_metni(std::slice::from_ref(&o));
        assert!(
            metin.contains("echo 29\n…(kısaltıldı; tamamı onay sayfasında)"),
            "{metin}"
        );
        assert!(!metin.contains("echo 30"), "{metin}");

        if let Some(k) = o.kabuk.as_mut() {
            k.komut = "x".repeat(5000);
        }
        let metin = yerel_onay_metni(std::slice::from_ref(&o));
        assert!(!metin.contains(&"x".repeat(2001)));
        assert!(metin.contains("kısaltıldı"));
    }

    #[test]
    fn yalitimsiz_komut_basligi_ve_talimati() {
        let ogeler = [oge(yalitimsiz(), false), oge(yalitimli(), true)];
        assert_eq!(yerel_onay_basligi(&ogeler), "Komutu yalıtımsız çalıştır");
        let metin = yerel_onay_metni(&ogeler);
        assert!(
            metin.starts_with("Aşağıdaki komut yalıtımsız çalışacak"),
            "{metin}"
        );
        assert!(
            metin.contains("izinlerine yalıtım için giriş ekler"),
            "{metin}"
        );
        assert!(metin.ends_with("Yalıtımı kurun."), "{metin}");
    }
}
