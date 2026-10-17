//! Oturumun kanal mesajlarını webview'a taşıyan Tauri kanalı (1c tasarımı §4)
//! ve ana pencerenin yalıtım (aşama 2 tasarımı §10.5) ve MCP kanalları.

use kalfa_core::aktarim::KanalMesaji;
use kalfa_uygulama::arayuz::{KanalYazici, McpYazici, YalitimDurumu, YalitimYazici};
use kalfa_uygulama::mcp::komutlar::McpDurumuDto;
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

/// Ana pencerenin MCP durum kanalı (1.1a-2 Görev 10).
pub struct TauriMcpKanali(pub Channel<McpDurumuDto>);

impl McpYazici for TauriMcpKanali {
    fn gonder(&self, durum: &McpDurumuDto) -> Result<(), String> {
        self.0.send(durum.clone()).map_err(|e| e.to_string())
    }
}
