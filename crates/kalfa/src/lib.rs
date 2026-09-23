//! Kalfa masaüstü uygulamasının Tauri kabuğu (1c tasarımı §3, §6). İnce
//! tutulur: pencereler, komutlar, kanal bağdaştırıcısı, yerel iletişim
//! kutuları. Mantık `kalfa-uygulama`'dadır.

mod iletisim;
mod kanal;
pub mod komut_adlari;
pub mod komutlar;
mod pencereler;

use std::error::Error;
use std::sync::Arc;

use kalfa_core::aktarim::AktarimAyarlari;
use kalfa_evren::VARSAYILAN_TABAN;
use kalfa_uygulama::anahtar::{AnahtarDeposu, AnahtarKasasi, SistemDeposu};
use kalfa_uygulama::dizin::{VERI_DIZINI_DEGISKENI, veri_dizini};
use kalfa_uygulama::evren::{EvrenUretici, TABAN_DEGISKENI};
use kalfa_uygulama::{Bilesenler, Uygulama};
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Builder, Manager, Runtime, WindowEvent};

use crate::iletisim::{TauriAcici, TauriIletisim};
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
    if let Err(e) = komutlar::kur(kurucu).run(tauri::generate_context!()) {
        eprintln!("Kalfa çalıştırılamadı: {e}");
        std::process::exit(1);
    }
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
    let veri = veri_dizini(
        &app.path().data_dir()?,
        std::env::var_os(VERI_DIZINI_DEGISKENI),
    );
    let taban = std::env::var(TABAN_DEGISKENI)
        .ok()
        .filter(|_| cfg!(debug_assertions))
        .unwrap_or_else(|| VARSAYILAN_TABAN.to_string());
    let kalici: Option<Box<dyn AnahtarDeposu>> =
        if std::env::var(ANAHTAR_DEPOSU_DEGISKENI).as_deref() == Ok("bellek") {
            None
        } else {
            SistemDeposu::ac()
                .ok()
                .map(|depo| Box::new(depo) as Box<dyn AnahtarDeposu>)
        };
    Ok(Uygulama::yeni(Bilesenler {
        veri_dizini: veri,
        kasa: AnahtarKasasi::yeni(kalici),
        uretici: Arc::new(EvrenUretici { taban }),
        iletisim: Arc::new(TauriIletisim(app.clone())),
        acici: Arc::new(TauriAcici(app.clone())),
        pencereler: Arc::new(TauriPencereler(app.clone())),
        aktarim: AktarimAyarlari::default(),
    }))
}
