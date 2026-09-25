//! Yerel iletişim kutuları ve sistem tarayıcısı (1c tasarımı §6). Onay
//! gerektiren işler webview'daki bir düğmeyle değil, işletim sisteminin
//! kutusuyla doğrulanır.
//!
//! Uçtan uca test derlemesi yerel kutuların yerine ayardaki cevapları kullanır
//! (`e2e`); orada `TauriIletisim` kullanılmaz.
#![cfg_attr(feature = "e2e", allow(dead_code))]

use std::path::PathBuf;

use async_trait::async_trait;
use kalfa_uygulama::arayuz::{Acici, Iletisim};
use tauri::{AppHandle, Manager, Runtime, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::oneshot;
use url::Url;

pub struct TauriIletisim<R: Runtime>(pub AppHandle<R>);

#[async_trait]
impl<R: Runtime> Iletisim for TauriIletisim<R> {
    async fn klasor_sec(&self, baslik: &str) -> Option<PathBuf> {
        let (gonder, al) = oneshot::channel();
        let mut kutu = self.0.dialog().file().set_title(baslik);
        if let Some(pencere) = odakli_pencere(&self.0) {
            kutu = kutu.set_parent(&pencere);
        }
        kutu.pick_folder(move |secilen| {
            let _ = gonder.send(secilen);
        });
        al.await
            .ok()
            .flatten()
            .and_then(|yol| yol.simplified().into_path().ok())
    }

    async fn onayla(&self, baslik: &str, mesaj: &str, tamam: &str, vazgec: &str) -> bool {
        let (gonder, al) = oneshot::channel();
        let mut kutu = self
            .0
            .dialog()
            .message(mesaj)
            .title(baslik)
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom(
                tamam.to_string(),
                vazgec.to_string(),
            ));
        if let Some(pencere) = odakli_pencere(&self.0) {
            kutu = kutu.parent(&pencere);
        }
        kutu.show(move |tamam_mi| {
            let _ = gonder.send(tamam_mi);
        });
        al.await.unwrap_or(false)
    }
}

pub struct TauriAcici<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> Acici for TauriAcici<R> {
    fn ac(&self, adres: &Url) -> Result<(), String> {
        self.0
            .opener()
            .open_url(adres.as_str(), None::<&str>)
            .map_err(|e| e.to_string())
    }
}

/// Kutunun ait olduğu pencere; kutu onun önünde açılır.
fn odakli_pencere<R: Runtime>(app: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    app.webview_windows()
        .into_values()
        .find(|p| p.is_focused().unwrap_or(false))
}
