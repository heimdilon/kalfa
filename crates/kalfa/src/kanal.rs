//! Oturumun kanal mesajlarını webview'a taşıyan Tauri kanalı (1c tasarımı §4)
//! ve ana pencerenin yalıtım kanalı (aşama 2 tasarımı §10.5).

use kalfa_core::aktarim::KanalMesaji;
use kalfa_uygulama::arayuz::{KanalYazici, YalitimDurumu, YalitimYazici};
use tauri::ipc::Channel;

pub struct TauriKanal(pub Channel<KanalMesaji>);

impl KanalYazici for TauriKanal {
    fn gonder(&self, mesaj: KanalMesaji) -> Result<(), String> {
        self.0.send(mesaj).map_err(|e| e.to_string())
    }
}

pub struct TauriYalitimKanali(pub Channel<YalitimDurumu>);

impl YalitimYazici for TauriYalitimKanali {
    fn gonder(&self, durum: &YalitimDurumu) -> Result<(), String> {
        self.0.send(durum.clone()).map_err(|e| e.to_string())
    }
}
