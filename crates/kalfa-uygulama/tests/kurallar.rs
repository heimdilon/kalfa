use std::ffi::OsString;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use kalfa_core::izin::Kip;
use kalfa_core::kimlik::OturumId;
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::SahteSaglayici;
use kalfa_evren::EvrenError;
use kalfa_uygulama::anahtar::{AnahtarDeposu, AnahtarKasasi, BellekDeposu, DepoHatasi};
use kalfa_uygulama::dizin::{
    geri_alma_dizini, kalfa_araclari, kayit_yolu, kopru_ikilisi, oturum_dizini,
    oturum_geri_alma_dizini, veri_dizini, yalitim_dizini,
};
use kalfa_uygulama::evren::DegisirSaglayici;
use kalfa_uygulama::guvenlik::{
    acilabilir_adres, gezinme_izinli, kip_adi, yukseltme_aciklamasi, yukseltme_mi,
};
use kalfa_uygulama::modeller::{VARSAYILAN_MODEL, bilinen_model_mi, modeller};
use kalfa_uygulama::sozlesme::{DepoTuru, ModelProfili};
use url::Url;

fn url(s: &str) -> Url {
    Url::parse(s).unwrap()
}

/// Kalıcı depo gibi davranan ama `yazilabilir` kapalıyken yazılamayan depo
/// (Secret Service geçici olarak çalışmıyor).
#[derive(Clone, Default)]
struct AcilirDepo {
    yazilabilir: Arc<AtomicBool>,
    ic: Arc<BellekDeposu>,
}

impl AnahtarDeposu for AcilirDepo {
    fn oku(&self) -> Result<Option<String>, DepoHatasi> {
        self.ic.oku()
    }

    fn yaz(&self, anahtar: &str) -> Result<(), DepoHatasi> {
        if !self.yazilabilir.load(Ordering::SeqCst) {
            return Err(DepoHatasi("Secret Service yok".into()));
        }
        self.ic.yaz(anahtar)
    }
}

#[test]
fn pencereler_yalnizca_uygulamanin_sayfalarina_gider() {
    let gelistirme = url("http://localhost:5173");
    for izinli in [
        "tauri://localhost/index.html",
        "http://tauri.localhost/sartlar.html",
        "https://tauri.localhost/",
    ] {
        assert!(gezinme_izinli(&url(izinli), None), "{izinli}");
    }
    assert!(gezinme_izinli(
        &url("http://localhost:5173/index.html"),
        Some(&gelistirme)
    ));
    for yasak in [
        "http://localhost:5173/index.html",
        "https://evren.example/",
        "http://localhost:5174/",
        "file:///C:/Windows/win.ini",
        "data:text/html,x",
        "javascript:alert(1)",
        "tauri://baska/",
    ] {
        assert!(!gezinme_izinli(&url(yasak), None), "{yasak}");
    }
    assert!(!gezinme_izinli(
        &url("http://localhost:5174/"),
        Some(&gelistirme)
    ));
}

#[test]
fn yalnizca_http_ve_https_acilir() {
    assert_eq!(
        acilabilir_adres("https://ornek.com/yol?a=1").map(|u| u.to_string()),
        Some("https://ornek.com/yol?a=1".to_string())
    );
    assert!(acilabilir_adres("http://ornek.com").is_some());
    for kotu in [
        "javascript:alert(1)",
        "file:///etc/passwd",
        "data:text/html,x",
        "mailto:a@b.c",
        "/goreli",
        "ornek.com",
        "http://",
    ] {
        assert!(acilabilir_adres(kotu).is_none(), "{kotu}");
    }
}

#[test]
fn saga_gecis_yukseltmedir() {
    use Kip::*;
    assert!(yukseltme_mi(Inceleme, Siki));
    assert!(yukseltme_mi(Siki, Proje));
    assert!(yukseltme_mi(Siki, Serbest));
    assert!(yukseltme_mi(Serbest, Otomatik));
    assert!(!yukseltme_mi(Otomatik, Serbest));
    assert!(!yukseltme_mi(Serbest, Siki));
    assert!(!yukseltme_mi(Proje, Inceleme));
    assert!(!yukseltme_mi(Siki, Siki));
    assert_eq!(kip_adi(Otomatik), "Otomatik");
    let metin = yukseltme_aciklamasi(Otomatik);
    assert!(!metin.is_empty());
    assert!(metin.contains("Göndermeden önce göster açıkken otomatik kip çalışmaz"));
}

/// Kalfa'nın araç zinciri yalnızca Program Files'taki kurulumdan gelir;
/// `KALFA_ARAC_ZINCIRI` yalnızca `e2e` derlemesinde okunur (2d-3, Görev 4).
#[test]
fn kalfa_araclari_kurulum_ya_da_e2e_degiskeninden_gelir() {
    let d = tempfile::tempdir().unwrap();
    let exe = d.path().join("kalfa.exe");
    std::fs::write(&exe, b"").unwrap();
    let araclar = d.path().join("araclar");
    std::fs::create_dir_all(araclar.join("python")).unwrap();
    std::fs::create_dir_all(araclar.join("node")).unwrap();
    // Kurulu olmayan (Program Files dışındaki) exe'nin yanındaki dizin sayılmaz.
    assert!(kalfa_araclari(&exe, None).is_empty());
    let degisken = kalfa_araclari(&exe, Some(araclar.clone().into_os_string()));
    if cfg!(all(windows, feature = "e2e")) {
        assert_eq!(degisken, vec![araclar.join("python"), araclar.join("node")]);
    } else {
        assert!(degisken.is_empty(), "{degisken:?}");
    }
}

#[test]
fn kopru_ikilisi_kurulum_ve_gelistirme_dizininden_gelir() {
    let d = tempfile::tempdir().unwrap();
    let exe = d.path().join("kalfa.exe");
    std::fs::write(&exe, b"").unwrap();

    let ad = if cfg!(windows) {
        "kalfa-kopru.exe"
    } else {
        "kalfa-kopru"
    };

    // Henüz kalfa-kopru yokken None döner
    assert_eq!(kopru_ikilisi(&exe, None), None);

    // Ortam değişkeni geçerli dosya ise önceliklidir
    let ozel_kopru = d.path().join("ozel-kopru");
    std::fs::write(&ozel_kopru, b"").unwrap();
    assert_eq!(
        kopru_ikilisi(&exe, Some(ozel_kopru.clone().into_os_string())),
        Some(ozel_kopru)
    );

    // Kurulu sürümde (exe ile aynı dizinde kalfa-kopru)
    let ayni_dizin_kopru = d.path().join(ad);
    std::fs::write(&ayni_dizin_kopru, b"").unwrap();
    assert_eq!(kopru_ikilisi(&exe, None), Some(ayni_dizin_kopru.clone()));

    // Geliştirme derlemesinde (target/debug/deps/... altından üstteki kalfa-kopru)
    let alt_dizin = d.path().join("deps");
    std::fs::create_dir_all(&alt_dizin).unwrap();
    let alt_exe = alt_dizin.join("test.exe");
    std::fs::write(&alt_exe, b"").unwrap();
    assert_eq!(kopru_ikilisi(&alt_exe, None), Some(ayni_dizin_kopru));
}

#[test]
fn veri_dizini_platforma_gore_ve_ortamla_degisir() {
    let taban = Path::new("veri");
    let beklenen = if cfg!(windows) { "Kalfa" } else { "kalfa" };
    assert_eq!(veri_dizini(taban, None), taban.join(beklenen));
    assert_eq!(
        veri_dizini(taban, Some(OsString::new())),
        taban.join(beklenen)
    );
    assert_eq!(
        veri_dizini(taban, Some(OsString::from("baska"))),
        Path::new("baska")
    );
    // Yalıtım dizini Windows'ta yerel (dolaşmayan) veri dizinindedir.
    let yerel = Path::new("yerel");
    let yalitim = yalitim_dizini(Path::new("v"), yerel, None);
    if cfg!(windows) {
        assert_eq!(yalitim, yerel.join("Kalfa").join("yalitim"));
    } else {
        assert_eq!(yalitim, Path::new("v").join("yalitim"));
    }
    assert_eq!(
        yalitim_dizini(Path::new("v"), yerel, Some(OsString::from("v"))),
        Path::new("v").join("yalitim")
    );
    // Geri alma verisi de (aşama 3 tasarımı §5.2).
    let geri = geri_alma_dizini(Path::new("v"), yerel, None);
    if cfg!(windows) {
        assert_eq!(geri, yerel.join("Kalfa").join("geri-alma"));
    } else {
        assert_eq!(geri, Path::new("v").join("geri-alma"));
    }
    assert_eq!(
        geri_alma_dizini(Path::new("v"), yerel, Some(OsString::from("v"))),
        Path::new("v").join("geri-alma")
    );
    let oturum = OturumId::yeni();
    assert_eq!(
        oturum_geri_alma_dizini(&geri, oturum),
        geri.join(oturum.to_string())
    );
    assert_eq!(
        kayit_yolu(&oturum_dizini(Path::new("v")), oturum),
        Path::new("v")
            .join("oturumlar")
            .join(format!("{oturum}.jsonl"))
    );
}

#[test]
fn model_listesi() {
    let liste = modeller();
    assert_eq!(liste.len(), 4);
    assert!(
        liste
            .iter()
            .any(|m| m.kimlik == "glm-5.3" && m.profil == ModelProfili::Derin)
    );
    assert!(bilinen_model_mi(VARSAYILAN_MODEL));
    assert!(bilinen_model_mi("auto"));
    assert!(!bilinen_model_mi("gpt-5"));
}

#[test]
fn kasa_kalici_depoya_yazar() {
    let kasa = AnahtarKasasi::yeni(Some(Box::new(BellekDeposu::default())));
    assert_eq!(kasa.tur(), DepoTuru::Kalici);
    assert_eq!(kasa.oku(), None);
    assert_eq!(kasa.yaz("evren_llm_a"), DepoTuru::Kalici);
    assert_eq!(kasa.oku().as_deref(), Some("evren_llm_a"));
}

#[test]
fn kasa_depo_yazilamazsa_bellekte_tutar() {
    let kasa = AnahtarKasasi::yeni(Some(Box::new(AcilirDepo::default())));
    assert_eq!(kasa.yaz("evren_llm_a"), DepoTuru::Bellekte);
    assert_eq!(kasa.tur(), DepoTuru::Bellekte);
    assert_eq!(kasa.oku().as_deref(), Some("evren_llm_a"));
}

#[test]
fn kalici_depoya_yazilan_anahtari_bellekteki_eskisi_golgelemez() {
    let depo = AcilirDepo::default();
    let kasa = AnahtarKasasi::yeni(Some(Box::new(depo.clone())));
    assert_eq!(kasa.yaz("evren_llm_eski"), DepoTuru::Bellekte);
    depo.yazilabilir.store(true, Ordering::SeqCst);
    assert_eq!(kasa.yaz("evren_llm_yeni"), DepoTuru::Kalici);
    assert_eq!(kasa.oku().as_deref(), Some("evren_llm_yeni"));
}

#[test]
fn kasa_depo_yoksa_bellekte_tutar() {
    let kasa = AnahtarKasasi::yeni(None);
    assert_eq!(kasa.tur(), DepoTuru::Bellekte);
    assert_eq!(kasa.yaz("evren_llm_a"), DepoTuru::Bellekte);
    assert_eq!(kasa.oku().as_deref(), Some("evren_llm_a"));
}

#[test]
fn bellek_deposu_okur_ve_yazar() {
    let depo = BellekDeposu::default();
    assert_eq!(depo.oku().unwrap(), None);
    depo.yaz("x").unwrap();
    assert_eq!(depo.oku().unwrap().as_deref(), Some("x"));
}

#[tokio::test]
async fn degisir_saglayici_anahtarsiz_401_doner_sonra_yenisini_kullanir() {
    let degisir = DegisirSaglayici::default();
    assert!(matches!(
        degisir.sart_durumu().await,
        Err(EvrenError::AnahtarGecersiz)
    ));
    let ilk = SahteSaglayici::yeni(Vec::new());
    degisir.ayarla(ilk.clone());
    assert_eq!(degisir.sart_metni().await.unwrap().version, 1);
    let ikinci = SahteSaglayici::yeni(Vec::new());
    ikinci.sart_surumunu_degistir(7);
    degisir.ayarla(ikinci as Arc<dyn Saglayici>);
    assert_eq!(degisir.sart_metni().await.unwrap().version, 7);
    assert!(ilk.kabul_edilen_surumler().is_empty());
}
