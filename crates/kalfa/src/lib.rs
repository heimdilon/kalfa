//! Kalfa masaüstü uygulamasının Tauri kabuğu (1c tasarımı §3, §6). İnce
//! tutulur: pencereler, komutlar, kanal bağdaştırıcısı, yerel iletişim
//! kutuları. Mantık `kalfa-uygulama`'dadır.

#[cfg(feature = "e2e")]
pub mod e2e;
mod iletisim;
mod kanal;
pub mod komut_adlari;
pub mod komutlar;
mod pencereler;

use std::error::Error;
use std::sync::Arc;

use kalfa_core::aktarim::AktarimAyarlari;
use kalfa_uygulama::anahtar::{AnahtarDeposu, AnahtarKasasi, SistemDeposu};
use kalfa_uygulama::dizin::{VERI_DIZINI_DEGISKENI, veri_dizini};
use kalfa_uygulama::yalitim::YalitimKaynagi;
use kalfa_uygulama::{Bilesenler, Uygulama};
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Builder, ExitRequestApi, Manager, RunEvent, Runtime, WindowEvent};

use crate::iletisim::TauriAcici;
use crate::pencereler::TauriPencereler;

/// `bellek` ise işletim sisteminin kimlik deposu kullanılmaz (uçtan uca
/// testler geliştiricinin gerçek anahtarına dokunmasın diye).
pub const ANAHTAR_DEPOSU_DEGISKENI: &str = "KALFA_ANAHTAR_DEPOSU";

pub fn calistir() {
    // Tek Kalfa süreci (1c tasarımı §5.2); ilk eklenti olmalı.
    let kurucu =
        tauri::Builder::default().plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            pencereler::one_getir(app);
        }));
    let kurucu = eklentiler(kurucu)
        .setup(|app| {
            let uygulama = kur(app.handle())?;
            app.manage(uygulama.clone());
            tauri::async_runtime::spawn(async move { uygulama.baslat().await });
            Ok(())
        })
        .on_page_load(|webview, yuk| {
            // Yeniden yükleme de bir sayfa yüklemesidir: ana pencereye bağlı
            // oturumlar kapanır, yeni sayfa son oturumu kayıttan açar (1c
            // tasarımı §5.6).
            if webview.label() == pencereler::ANA && yuk.event() == PageLoadEvent::Started {
                ana_pencere_gitti(webview.app_handle());
            }
        })
        .on_window_event(|pencere, olay| {
            if pencere.label() == pencereler::ANA && matches!(olay, WindowEvent::Destroyed) {
                ana_pencere_gitti(pencere.app_handle());
            }
        });
    let app = match komutlar::kur(kurucu).build(tauri::generate_context!()) {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Kalfa çalıştırılamadı: {e}");
            std::process::exit(1);
        }
    };
    app.run(|app, olay| {
        if let RunEvent::ExitRequested { api, .. } = olay {
            cikisi_ertele(app, api);
        }
    });
}

/// Ana pencere kapanınca oturumları kapanır; süren tur iptal edilip kayda
/// yazılır (1c tasarımı §5.6). Son pencere kapandığında süreç bunlar bitmeden
/// çıkmaz; yoksa kayıt yarım kalır.
fn cikisi_ertele<R: Runtime>(app: &AppHandle<R>, api: ExitRequestApi) {
    let Some(durum) = app.try_state::<Arc<Uygulama>>() else {
        return;
    };
    if !durum.kapanan_oturum_var() {
        return;
    }
    api.prevent_exit();
    let uygulama = durum.inner().clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        uygulama.kapananlari_bekle().await;
        app.exit(0);
    });
}

/// Webview'a açılmayan eklentiler: iletişim kutuları ve sistem tarayıcısı
/// yalnızca Rust'tan kullanılır. Hiçbir capability bu eklentilerin
/// komutlarına izin vermez (tests/izinler.rs denetler).
pub fn eklentiler<R: Runtime>(kurucu: Builder<R>) -> Builder<R> {
    kurucu
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
}

fn ana_pencere_gitti<R: Runtime>(app: &AppHandle<R>) {
    let Some(durum) = app.try_state::<Arc<Uygulama>>() else {
        return;
    };
    let uygulama = durum.inner().clone();
    let kapananlar = uygulama.ana_pencere_gitti();
    if !kapananlar.is_empty() {
        tauri::async_runtime::spawn(async move { uygulama.kapanislari_bekle(kapananlar).await });
    }
}

fn kur<R: Runtime>(app: &AppHandle<R>) -> Result<Arc<Uygulama>, Box<dyn Error>> {
    let platform_veri = app
        .path()
        .data_dir()
        .map_err(|e| std::io::Error::other(format!("Kullanıcı veri dizini bulunamadı: {e}")))?;
    let platform_yerel_veri = app.path().local_data_dir().map_err(|e| {
        std::io::Error::other(format!("Yerel kullanıcı veri dizini bulunamadı: {e}"))
    })?;
    let veri = veri_dizini(&platform_veri, std::env::var_os(VERI_DIZINI_DEGISKENI));
    // Windows test yalıtımında evin işletim sistemi kaydı görünmeyebilir.
    // E2e derlemesi evini kendi geçici veri dizininde tutar.
    #[cfg(all(feature = "e2e", windows))]
    let ev_dizini = veri.join("ev");
    #[cfg(not(all(feature = "e2e", windows)))]
    let ev_dizini = app
        .path()
        .home_dir()
        .map_err(|e| std::io::Error::other(format!("Kullanıcı ev dizini bulunamadı: {e}")))?;
    let yalitim = kalfa_uygulama::dizin::yalitim_dizini(
        &veri,
        &platform_yerel_veri,
        std::env::var_os(VERI_DIZINI_DEGISKENI),
    );
    let kalici: Option<Box<dyn AnahtarDeposu>> =
        if std::env::var(ANAHTAR_DEPOSU_DEGISKENI).as_deref() == Ok("bellek") {
            None
        } else {
            SistemDeposu::ac()
                .ok()
                .map(|depo| Box::new(depo) as Box<dyn AnahtarDeposu>)
        };
    let kasa = AnahtarKasasi::yeni(kalici);
    #[cfg(not(feature = "e2e"))]
    let (uretici, iletisim) = evren_bilesenleri(app);
    #[cfg(feature = "e2e")]
    let (uretici, iletisim) = e2e::kur(app, &kasa)?;
    Ok(Uygulama::yeni(Bilesenler {
        veri_dizini: veri,
        kasa,
        uretici,
        iletisim,
        acici: Arc::new(TauriAcici(app.clone())),
        pencereler: Arc::new(TauriPencereler(app.clone())),
        yukseltici: Arc::new(kalfa_uygulama::yukseltici::KuruluYukseltici),
        aktarim: AktarimAyarlari::default(),
        ev_dizini,
        yalitim_dizini: yalitim,
        yalitim: YalitimKaynagi::Platform,
    }))
}

/// EVREN istemcisi ve işletim sisteminin iletişim kutuları. Uçtan uca test
/// derlemesi bunların yerine `e2e::kur`'un sahtelerini kullanır.
#[cfg(not(feature = "e2e"))]
fn evren_bilesenleri<R: Runtime>(
    app: &AppHandle<R>,
) -> (
    Arc<dyn kalfa_uygulama::evren::SaglayiciUretici>,
    Arc<dyn kalfa_uygulama::arayuz::Iletisim>,
) {
    let taban = std::env::var(kalfa_uygulama::evren::TABAN_DEGISKENI)
        .ok()
        .filter(|_| cfg!(debug_assertions))
        .unwrap_or_else(|| kalfa_evren::VARSAYILAN_TABAN.to_string());
    (
        Arc::new(kalfa_uygulama::evren::EvrenUretici { taban }),
        Arc::new(iletisim::TauriIletisim(app.clone())),
    )
}
