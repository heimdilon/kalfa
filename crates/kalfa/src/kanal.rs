//! Oturumun kanal mesajlarını webview'a taşıyan Tauri kanalı (1c tasarımı §4).

use kalfa_core::aktarim::KanalMesaji;
use kalfa_uygulama::arayuz::KanalYazici;
use tauri::ipc::Channel;

pub struct TauriKanal(pub Channel<KanalMesaji>);

impl KanalYazici for TauriKanal {
    fn gonder(&self, mesaj: KanalMesaji) -> Result<(), String> {
        self.0.send(mesaj).map_err(|e| e.to_string())
    }
}
