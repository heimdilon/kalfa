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
use kalfa_uygulama::dizin::{kayit_yolu, oturum_dizini, veri_dizini};
use kalfa_uygulama::evren::DegisirSaglayici;
use kalfa_uygulama::guvenlik::{acilabilir_adres, gezinme_izinli, yukseltme_mi};
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
    assert!(!yukseltme_mi(Serbest, Siki));
    assert!(!yukseltme_mi(Proje, Inceleme));
    assert!(!yukseltme_mi(Siki, Siki));
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
    let oturum = OturumId::yeni();
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
