//! Kayıttan oturum durumunu kurma (spec §6.6).
//!
//! Sonucu bilinmeyen yan etkili işlem **asla** yeniden çalıştırılmaz: yalnızca
//! "sonucu bilinmiyor" olarak modele ve kullanıcıya bildirilir.

use std::collections::{HashMap, HashSet};

use kalfa_evren::tipler::{ChatMessage, Role};

use crate::izin::Kip;
use crate::kayit::KayitOgesi;
use crate::kimlik::{CagriId, OturumId};
use crate::protokol::Zarf;

pub const BILINMEYEN_SONUC: &str = "[sonucu bilinmiyor: Kalfa bu araç çalışırken kapandı; işlem yapılmış da olabilir, yapılmamış da]";
pub const CALISTIRILMADI: &str = "[çalıştırılmadı: Kalfa bu çağrıdan önce kapandı]";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BilinmeyenCagri {
    pub cagri: CagriId,
    pub arac: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KurtarilmisDurum {
    pub oturum: OturumId,
    pub model: String,
    pub kip: Kip,
    /// Kayıtta bulunan bağlam mesajları.
    pub mesajlar: Vec<ChatMessage>,
    /// Yanıtı olmayan araç çağrıları için üretilen, henüz kayda yazılmamış
    /// `tool` mesajları; `mesajlar`ın sonuna eklenir.
    pub tamamlayicilar: Vec<ChatMessage>,
    pub bilinmeyenler: Vec<BilinmeyenCagri>,
    pub son_sira: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KurtarmaHatasi {
    #[error("Oturum kaydı boş.")]
    Bos,
    #[error("Oturum kaydı bir açılış kaydıyla başlamıyor.")]
    AcilisYok,
}

pub fn kurtar(kayitlar: &[Zarf<KayitOgesi>]) -> Result<KurtarilmisDurum, KurtarmaHatasi> {
    let ilk = kayitlar.first().ok_or(KurtarmaHatasi::Bos)?;
    let KayitOgesi::OturumAcildi { model, kip } = &ilk.yuk else {
        return Err(KurtarmaHatasi::AcilisYok);
    };
    let (mut model, mut kip) = (model.clone(), *kip);
    let mut mesajlar = Vec::new();
    let mut baslayan: HashMap<String, BilinmeyenCagri> = HashMap::new();
    let mut biten: HashSet<CagriId> = HashSet::new();

    for zarf in kayitlar {
        match &zarf.yuk {
            KayitOgesi::Mesaj { mesaj } => mesajlar.push(mesaj.clone()),
            KayitOgesi::KipDegisti { kip: k } => kip = *k,
            KayitOgesi::ModelDegisti { model: m } => model = m.clone(),
            KayitOgesi::AracBasladi {
                cagri,
                model_cagri_id,
                arac,
            } => {
                baslayan.insert(
                    model_cagri_id.clone(),
                    BilinmeyenCagri {
                        cagri: *cagri,
                        arac: arac.clone(),
                    },
                );
            }
            KayitOgesi::AracBitti { cagri, .. } => {
                biten.insert(*cagri);
            }
            _ => {}
        }
    }

    // Serbest kip yalnızca oturum boyunca geçerlidir; kurtarılan oturum Sıkı açılır.
    if kip == Kip::Serbest {
        kip = Kip::Siki;
    }

    // Her araç çağrısının bir yanıtı olmalı; yoksa bağlam geçersizdir.
    let yanitlanan: HashSet<&str> = mesajlar
        .iter()
        .filter(|m| m.role == Role::Tool)
        .filter_map(|m| m.tool_call_id.as_deref())
        .collect();
    let mut tamamlayicilar = Vec::new();
    let mut bilinmeyenler = Vec::new();
    for mesaj in mesajlar.iter().filter(|m| m.role == Role::Assistant) {
        for tc in mesaj.tool_calls.iter().flatten() {
            if yanitlanan.contains(tc.id.as_str()) {
                continue;
            }
            match baslayan.get(&tc.id) {
                Some(b) if !biten.contains(&b.cagri) => {
                    bilinmeyenler.push(b.clone());
                    tamamlayicilar.push(ChatMessage::tool(tc.id.clone(), BILINMEYEN_SONUC));
                }
                _ => tamamlayicilar.push(ChatMessage::tool(tc.id.clone(), CALISTIRILMADI)),
            }
        }
    }

    Ok(KurtarilmisDurum {
        oturum: ilk.oturum,
        model,
        kip,
        mesajlar,
        tamamlayicilar,
        bilinmeyenler,
        son_sira: kayitlar.last().map_or(0, |z| z.sira),
    })
}
