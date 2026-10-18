//! İki pencere yalnızca buradan açılır (1c tasarımı §6). Webview başka bir
//! adrese gidemez, yeni pencere açamaz.

use kalfa_uygulama::arayuz::Pencereler;
use kalfa_uygulama::guvenlik::gezinme_izinli;
use kalfa_uygulama::hata::KomutHatasi;
use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const ANA: &str = "ana";
pub const SARTLAR: &str = "sartlar";

struct PencereTanimi {
    etiket: &'static str,
    sayfa: &'static str,
    baslik: &'static str,
    boyut: (f64, f64),
    en_kucuk: (f64, f64),
}

const ANA_PENCERE: PencereTanimi = PencereTanimi {
    etiket: ANA,
    sayfa: "index.html",
    baslik: "Kalfa",
    boyut: (1280.0, 800.0),
    en_kucuk: (960.0, 600.0),
};

const SART_PENCERESI: PencereTanimi = PencereTanimi {
    etiket: SARTLAR,
    sayfa: "sartlar.html",
    baslik: "Kalfa — EVREN hesabı",
    boyut: (760.0, 820.0),
    en_kucuk: (560.0, 600.0),
};

pub struct TauriPencereler<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> TauriPencereler<R> {
    /// Pencere açıksa öne getirir, değilse kurar.
    fn ac(&self, tanim: &PencereTanimi) -> Option<WebviewWindow<R>> {
        if let Some(pencere) = self.0.get_webview_window(tanim.etiket) {
            one_al(&pencere);
            return Some(pencere);
        }
        match kur(&self.0, tanim) {
            Ok(pencere) => Some(pencere),
            Err(e) => {
                eprintln!("Kalfa: {} penceresi açılamadı: {e}", tanim.etiket);
                None
            }
        }
    }
}

impl<R: Runtime> Pencereler for TauriPencereler<R> {
    fn ana_ac(&self) {
        self.ac(&ANA_PENCERE);
    }

    fn sartlar_goster(&self) {
        // Açık pencere yeniden yüklenir; yeni durumu `sart_penceresi_durumu`
        // ile okur. İkinci bir şart penceresi açılmaz.
        if let Some(pencere) = self.0.get_webview_window(SARTLAR) {
            if let Err(e) = pencere.reload() {
                eprintln!("Kalfa: şart penceresi yenilenemedi: {e}");
            }
            one_al(&pencere);
            return;
        }
        self.ac(&SART_PENCERESI);
    }

    fn sartlar_kapat(&self) {
        if let Some(pencere) = self.0.get_webview_window(SARTLAR)
            && let Err(e) = pencere.close()
        {
            eprintln!("Kalfa: şart penceresi kapatılamadı: {e}");
        }
    }
}

/// İkinci Kalfa süreci açılmak istendiğinde var olan pencere öne gelir.
pub fn one_getir<R: Runtime>(app: &AppHandle<R>) {
    let pencere = app
        .get_webview_window(ANA)
        .or_else(|| app.get_webview_window(SARTLAR));
    if let Some(pencere) = pencere {
        one_al(&pencere);
    }
}

fn one_al<R: Runtime>(pencere: &WebviewWindow<R>) {
    let _ = pencere.unminimize();
    let _ = pencere.show();
    let _ = pencere.set_focus();
}

fn kur<R: Runtime>(app: &AppHandle<R>, tanim: &PencereTanimi) -> tauri::Result<WebviewWindow<R>> {
    // Geliştirmede sayfalar Vite sunucusundan gelir; o köken yalnızca dev
    // derlemesinde izinlidir.
    let gelistirme = if tauri::is_dev() {
        app.config().build.dev_url.clone()
    } else {
        None
    };
    WebviewWindowBuilder::new(app, tanim.etiket, WebviewUrl::App(tanim.sayfa.into()))
        .title(tanim.baslik)
        .inner_size(tanim.boyut.0, tanim.boyut.1)
        .min_inner_size(tanim.en_kucuk.0, tanim.en_kucuk.1)
        .center()
        // Windows'ta ana pencerenin başlık çubuğunu arayüz çizer (masaüstü
        // arayüzü tasarımı §5); Linux'ta ve öbür pencerelerde yerel çerçeve.
        .decorations(!(cfg!(windows) && tanim.etiket == ANA))
        .on_navigation(move |adres| gezinme_izinli(adres, gelistirme.as_ref()))
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .build()
}

// Başlık çubuğunun düğmeleri (masaüstü arayüzü tasarımı §5.2). Tauri'nin
// `core:window` izinleri pencerelere verilmez; bu komutlar yalnız ana
// pencereye açıktır ve yalnız çağıran pencereye uygulanır.

fn ic(e: tauri::Error) -> KomutHatasi {
    KomutHatasi::ic(e.to_string())
}

#[tauri::command]
pub(crate) async fn pencere_kucult<R: Runtime>(
    pencere: WebviewWindow<R>,
) -> Result<(), KomutHatasi> {
    pencere.minimize().map_err(ic)
}

/// Büyükse geri yükler, değilse büyütür; pencerenin yeni hâlini (büyük mü) döner.
#[tauri::command]
pub(crate) async fn pencere_buyut_degistir<R: Runtime>(
    pencere: WebviewWindow<R>,
) -> Result<bool, KomutHatasi> {
    if pencere.is_maximized().map_err(ic)? {
        pencere.unmaximize().map_err(ic)?;
        Ok(false)
    } else {
        pencere.maximize().map_err(ic)?;
        Ok(true)
    }
}

/// Kapatma düğmesiyle aynı yol: kapanma isteği ve yok olma olayları
/// oturumları kapatır (lib.rs).
#[tauri::command]
pub(crate) async fn pencere_kapat<R: Runtime>(
    pencere: WebviewWindow<R>,
) -> Result<(), KomutHatasi> {
    pencere.close().map_err(ic)
}

#[tauri::command]
pub(crate) async fn pencere_surukle<R: Runtime>(
    pencere: WebviewWindow<R>,
) -> Result<(), KomutHatasi> {
    pencere.start_dragging().map_err(ic)
}
