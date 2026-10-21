//! Uzak MCP Streamable HTTP bağlantı/oturum durumu, protokol müzakeresi ve
//! belirsiz araç çağrısı (`BelirsizUzakCagri`) takibi.
//!
//! - SEP-2575 (`2026-07-28`) akışında oturum kimliği zorunlu değildir; her istek
//!   `_meta` içinde protokol sürümü ve istemci yeteneklerini taşır.
//! - Eski (`2025-11-25`, `2025-06-18`, `2025-03-26`) sunucularda `initialize`
//!   yanıtındaki `Mcp-Session-Id` başlığı doğrulanarak sonraki aynı-origin
//!   isteklerde `Mcp-Session-Id` ve `MCP-Protocol-Version` başlığıyla gönderilir.
//! - Gönderimi başlamış bir `tools/call` isteğinde yanıt tamamlanmadan akış
//!   koparsa veya iptal bildirimi gönderilirse çağrı sonucu **belirsiz** olarak
//!   kaydedilir ve otomatik tekrar yapılmaz.

use std::sync::{Arc, Mutex};

use rmcp::model::RequestId;

use crate::maske::{GizliDeger, maskele_metin};

/// Uzak bir `tools/call` çağrısının sonucunun belirsiz kalma nedeni.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BelirsizCagriNedeni {
    /// İstek sunucuya gönderildikten sonra HTTP/SSE yanıt akışı tamamlanmadan koptu.
    AkisKesildi,
    /// İstek sunucuya gönderildikten sonra iptal (`notifications/cancelled`) iletildi;
    /// iptal bildirimi sunucudaki yan etkinin geri alındığını kanıtlamaz.
    IptalGonderildiSonucBilinmiyor,
}

/// Sonucu belirsiz kalan tek bir uzak araç çağrısı kaydı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BelirsizUzakCagri {
    pub istek_kimligi: RequestId,
    pub neden: BelirsizCagriNedeni,
}

/// Uzak sunucunun döndürdüğü HTTP `401` / `403` yetkilendirme reddi özeti.
///
/// `www_authenticate` alanı kaydedilmeden önce bilinen tüm sırlarla maskelenir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UzakYetkiReddi {
    pub durum_kodu: u16,
    pub www_authenticate: Option<String>,
}

#[derive(Default)]
struct OturumIci {
    mcp_session_id: Option<String>,
    anlasilan_protokol: Option<String>,
    son_yetki_reddi: Option<UzakYetkiReddi>,
    belirsiz_cagrilar: Vec<BelirsizUzakCagri>,
    gonderilen_arac_cagrisi_sayisi: usize,
}

/// Uzak HTTP taşımasının oturum, protokol ve belirsiz çağrı durumunu paylaşan tutamak.
#[derive(Clone, Default)]
pub struct UzakOturumDurumu {
    ic: Arc<Mutex<OturumIci>>,
}

impl UzakOturumDurumu {
    pub fn yeni() -> Self {
        Self::default()
    }

    pub fn mcp_session_id(&self) -> Option<String> {
        self.ic.lock().ok().and_then(|g| g.mcp_session_id.clone())
    }

    pub(crate) fn mcp_session_id_kaydet(&self, aday: &str) -> bool {
        if !oturum_kimligi_gecerli(aday) {
            return false;
        }
        if let Ok(mut g) = self.ic.lock() {
            g.mcp_session_id = Some(aday.to_owned());
            true
        } else {
            false
        }
    }

    pub fn anlasilan_protokol(&self) -> Option<String> {
        self.ic
            .lock()
            .ok()
            .and_then(|g| g.anlasilan_protokol.clone())
    }

    pub(crate) fn anlasilan_protokol_kaydet(&self, surum: &str) {
        if let Ok(mut g) = self.ic.lock() {
            g.anlasilan_protokol = Some(surum.to_owned());
        }
    }

    pub fn son_yetki_reddi(&self) -> Option<UzakYetkiReddi> {
        self.ic.lock().ok().and_then(|g| g.son_yetki_reddi.clone())
    }

    pub(crate) fn yetki_reddi_kaydet(
        &self,
        durum_kodu: u16,
        ham_www_authenticate: Option<&str>,
        sirlar: &[GizliDeger],
    ) {
        let www_authenticate = ham_www_authenticate.map(|deger| {
            let maskeli = maskele_metin(deger, sirlar);
            maskeli.chars().take(512).collect()
        });
        if let Ok(mut g) = self.ic.lock() {
            g.son_yetki_reddi = Some(UzakYetkiReddi {
                durum_kodu,
                www_authenticate,
            });
        }
    }

    pub fn belirsiz_cagrilar(&self) -> Vec<BelirsizUzakCagri> {
        self.ic
            .lock()
            .map(|g| g.belirsiz_cagrilar.clone())
            .unwrap_or_default()
    }

    pub(crate) fn belirsiz_cagri_ekle(&self, istek_kimligi: RequestId, neden: BelirsizCagriNedeni) {
        if let Ok(mut g) = self.ic.lock()
            && !g
                .belirsiz_cagrilar
                .iter()
                .any(|k| k.istek_kimligi == istek_kimligi)
        {
            g.belirsiz_cagrilar.push(BelirsizUzakCagri {
                istek_kimligi,
                neden,
            });
        }
    }

    pub(crate) fn arac_cagrisi_gonderildi(&self) {
        if let Ok(mut g) = self.ic.lock() {
            g.gonderilen_arac_cagrisi_sayisi = g.gonderilen_arac_cagrisi_sayisi.saturating_add(1);
        }
    }

    pub fn gonderilen_arac_cagrisi_sayisi(&self) -> usize {
        self.ic
            .lock()
            .map(|g| g.gonderilen_arac_cagrisi_sayisi)
            .unwrap_or(0)
    }
}

pub fn oturum_kimligi_gecerli(kimlik: &str) -> bool {
    !kimlik.is_empty() && kimlik.len() <= 256 && kimlik.bytes().all(|b| (0x21..=0x7E).contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oturum_kimligi_dogrulanir_ve_yetki_reddi_sirlari_maskeler() {
        let durum = UzakOturumDurumu::yeni();
        assert!(durum.mcp_session_id_kaydet("sess-abc-123"));
        assert_eq!(durum.mcp_session_id().as_deref(), Some("sess-abc-123"));
        assert!(!durum.mcp_session_id_kaydet("gecersiz\r\nkimlik"));

        let sir = GizliDeger::yeni("gizli-bearer-token-999").unwrap();
        durum.yetki_reddi_kaydet(
            401,
            Some("Bearer error=\"invalid_token\", hint=\"gizli-bearer-token-999\""),
            &[sir],
        );
        let red = durum.son_yetki_reddi().unwrap();
        assert_eq!(red.durum_kodu, 401);
        assert!(
            !red.www_authenticate
                .as_deref()
                .unwrap_or_default()
                .contains("gizli-bearer-token-999")
        );
    }
}
