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
        Kip::Otomatik => 4,
    }
}

/// İnceleme < Sıkı < Proje < Serbest < Otomatik; sağa her geçiş yükseltmedir ve
/// yerel iletişim kutusunda onaylanır. Kipi düşürmek onay istemez.
pub fn yukseltme_mi(eski: Kip, yeni: Kip) -> bool {
    sira(yeni) > sira(eski)
}

pub fn kip_adi(kip: Kip) -> &'static str {
    match kip {
        Kip::Inceleme => "İnceleme",
        Kip::Siki => "Sıkı",
        Kip::Proje => "Proje",
        Kip::Serbest => "Serbest",
        Kip::Otomatik => "Otomatik",
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
        Kip::Otomatik => {
            "Kalfa, Serbest kipin onay sormadan yaptıklarına ek olarak, onay isteyen yalıtımlı \
             komutları ve MCP çağrılarını bir hakem modele sorar. Hakem güvenli bulursa işlem onay \
             sorulmadan çalışır; riskli bulursa reddedilir; emin olamazsa size sorulur. Her hakem \
             kararı EVREN'e bir istek gönderir ve bütçeden düşer. Yalıtımsız komutlar, gizli \
             dosyalar, korumalı yollar ve geri alınamaz yazmalar yine size sorulur. Otomatik kip \
             yalnızca bu oturum boyunca geçerlidir. Göndermeden önce göster açıkken otomatik kip \
             çalışmaz; her işlem size sorulur."
        }
    }
}

fn yalitimsiz_var(ogeler: &[OnayOgesi]) -> bool {
    ogeler
        .iter()
        .any(|o| o.kabuk.as_ref().is_some_and(|k| !k.yalitim.yalitimli_mi()))
}

/// Eklentinin bu çalışma alanındaki ilk EVREN gönderimi (aşama 4 tasarımı
/// §8.4): öğe eklentinin tarifini taşır ve yerel onay ister.
fn eklenti_ogesi_mi(o: &OnayOgesi) -> bool {
    o.yerel_onay && o.eklenti.as_ref().is_some_and(|t| t.ilk_gonderim)
}

/// Yerel onay penceresinin başlığı: yalıtımsız komut varsa onu, yalnızca
/// eklentinin ilk gönderimi varsa onu, yoksa çalışma alanının hazırlığını
/// (Windows, aşama 2 tasarımı §9.4) söyler.
pub fn yerel_onay_basligi(ogeler: &[OnayOgesi]) -> &'static str {
    if yalitimsiz_var(ogeler) {
        "Komutu yalıtımsız çalıştır"
    } else if ogeler.iter().any(eklenti_ogesi_mi) && ogeler.iter().all(|o| o.kabuk.is_none()) {
        "Eklenti EVREN'e veri gönderecek"
    } else {
        "Çalışma alanını yalıtım için hazırla"
    }
}

/// Onay kartının Markdown metninden kalın işaretlerini (`**`) kaldırır. Kod
/// aralığındaki (`` ` ``) `**/*.md` gibi kalıplara dokunmaz.
fn vurgusuz(metin: &str) -> String {
    let mut sonuc = String::with_capacity(metin.len());
    let mut kodda = false;
    let mut karakterler = metin.chars().peekable();
    while let Some(c) = karakterler.next() {
        match c {
            '`' => {
                kodda = !kodda;
                sonuc.push(c);
            }
            '*' if !kodda && karakterler.peek() == Some(&'*') => {
                karakterler.next();
            }
            _ => sonuc.push(c),
        }
    }
    sonuc
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
    // Eklentinin ilk gönderimi: neyi okuyup EVREN'e göndereceği kartın
    // metnindedir (§8.4). Metin eklentiden gelen bir manifestten türer;
    // görünmez karakterler işaretlenir.
    for o in ogeler.iter().filter(|o| eklenti_ogesi_mi(o)) {
        metin.push_str(&yerel_gosterim(&vurgusuz(&o.aciklama)));
        metin.push('\n');
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
                program: None,
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
            kisayol: None,
            eklenti: None,
            mcp: None,
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

    fn eklenti_oge() -> OnayOgesi {
        OnayOgesi {
            cagri: CagriId::yeni(),
            arac: "kalfa_rag__anlamsal_ara".into(),
            aciklama: "kalfa-rag eklentisinin `anlamsal_ara` aracını çalıştırmak istiyor.\n\n\
                **kalfa-rag** bu çalışma alanında ilk kez EVREN'e veri gönderecek.\n\n\
                Okuyabileceği dosyalar: `**/*.md`, `docs/**` ile eşleşen **412 dosya (6,0 MB)**. \
                Bu dosyalardan seçtiği parçalar gömme için EVREN'e gidecek. Gizli dosyalar \
                gönderilmez.\n\nİzin verirseniz bu çalışma alanında bir daha sorulmaz."
                .into(),
            hedefler: Vec::new(),
            diff: None,
            uyarilar: Vec::new(),
            muhur: OnayMuhru("m".into()),
            kabuk: None,
            yerel_onay: true,
            onaysiz_degisiklikler: None,
            geri_alinabilir: None,
            veri_cikisi: Some(kalfa_core::protokol::VeriCikisi::EklentiCiktisi),
            kisayol: None,
            eklenti: Some(kalfa_core::arac::EklentiTarifi {
                ad: "kalfa-rag".into(),
                ilk_gonderim: true,
                ..Default::default()
            }),
            mcp: None,
        }
    }

    #[test]
    fn vurgusuz_kalin_isaretini_kaldirir_kod_araligina_dokunmaz() {
        assert_eq!(
            vurgusuz("**ad** bu `**/*.md` ile **412 dosya** `a**b`"),
            "ad bu `**/*.md` ile 412 dosya `a**b`"
        );
        assert_eq!(vurgusuz("tek * yıldız ve ** açık"), "tek * yıldız ve  açık");
    }

    #[test]
    fn eklentinin_ilk_gonderimi_neyi_gonderecegini_soyler() {
        let ogeler = [eklenti_oge()];
        assert_eq!(
            yerel_onay_basligi(&ogeler),
            "Eklenti EVREN'e veri gönderecek"
        );
        let metin = yerel_onay_metni(&ogeler);
        assert!(
            metin.contains("kalfa-rag bu çalışma alanında ilk kez EVREN'e veri gönderecek."),
            "{metin}"
        );
        assert!(
            metin.contains("`**/*.md`, `docs/**` ile eşleşen 412 dosya (6,0 MB)"),
            "{metin}"
        );
        assert!(metin.contains("bir daha sorulmaz"), "{metin}");
        assert!(
            !metin.contains("**kalfa-rag**") && !metin.contains("**412"),
            "{metin}"
        );
        assert!(!metin.contains("yalıtım"), "kabukla ilgisi yok: {metin}");
    }

    #[test]
    fn eklenti_metni_gorunmez_karakterleri_isaretler() {
        let mut o = eklenti_oge();
        o.aciklama = format!("{}\u{202E}", o.aciklama);
        let metin = yerel_onay_metni(&[o]);
        assert!(
            metin.contains("⟨U+202E⟩") && !metin.contains('\u{202E}'),
            "{metin}"
        );
    }

    #[test]
    fn eklenti_ve_kabuk_birlikte_kabugun_basligini_alir() {
        let ogeler = [eklenti_oge(), oge(yalitimli(), true)];
        assert_eq!(
            yerel_onay_basligi(&ogeler),
            "Çalışma alanını yalıtım için hazırla"
        );
        let metin = yerel_onay_metni(&ogeler);
        assert!(metin.contains("ilk kez EVREN'e veri gönderecek"), "{metin}");
        assert!(metin.contains("Komut: cargo test"), "{metin}");
        let ogeler = [eklenti_oge(), oge(yalitimsiz(), false)];
        assert_eq!(yerel_onay_basligi(&ogeler), "Komutu yalıtımsız çalıştır");
        // İlk gönderimi olmayan eklenti öğesi başlığı değiştirmez.
        let mut baska = eklenti_oge();
        baska.eklenti = baska.eklenti.map(|t| kalfa_core::arac::EklentiTarifi {
            ilk_gonderim: false,
            ..t
        });
        assert_eq!(
            yerel_onay_basligi(&[baska]),
            "Çalışma alanını yalıtım için hazırla"
        );
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
