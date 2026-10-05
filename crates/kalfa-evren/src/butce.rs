//! Bir yanıtın ömrü boyunca, biriktirmeden önce uygulanan kaynak sınırları.
use crate::{AkisOlayi, EvrenError};

pub const YANIT_BAYT_SINIRI: usize = 32 * 1024 * 1024;
pub const JSON_BAYT_SINIRI: usize = 8 * 1024 * 1024;
pub const YANIT_CAGRI_SINIRI: usize = 128;
pub const YANIT_OLAY_SINIRI: usize = 65_536;

#[derive(Debug, Default)]
pub struct YanitButcesi {
    bayt: usize,
    cagri: usize,
    olay: usize,
    girdi: usize,
    asildi: bool,
}

impl YanitButcesi {
    pub fn denetle(&self) -> Result<(), EvrenError> {
        if self.asildi {
            Err(EvrenError::YanitSiniri)
        } else {
            Ok(())
        }
    }

    fn ekle(
        sayac: &mut usize,
        adet: usize,
        azami: usize,
        asildi: &mut bool,
    ) -> Result<(), EvrenError> {
        if !*asildi && let Some(toplam) = sayac.checked_add(adet).filter(|n| *n <= azami) {
            *sayac = toplam;
            return Ok(());
        }
        *asildi = true;
        Err(EvrenError::YanitSiniri)
    }

    pub fn bayt_ekle(&mut self, adet: usize) -> Result<(), EvrenError> {
        Self::ekle(&mut self.bayt, adet, YANIT_BAYT_SINIRI, &mut self.asildi)
    }
    pub fn cagri_ekle(&mut self, adet: usize) -> Result<(), EvrenError> {
        Self::ekle(&mut self.cagri, adet, YANIT_CAGRI_SINIRI, &mut self.asildi)
    }
    pub fn parca_ekle(&mut self) -> Result<(), EvrenError> {
        Self::ekle(&mut self.olay, 1, YANIT_OLAY_SINIRI, &mut self.asildi)
    }
    pub fn girdi_ekle(&mut self) -> Result<(), EvrenError> {
        Self::ekle(&mut self.girdi, 1, YANIT_OLAY_SINIRI, &mut self.asildi)
    }

    /// EVREN dışında bir sağlayıcı da çekirdeğin biriktirme sınırını paylaşır.
    pub fn olay_ekle(&mut self, olay: &AkisOlayi) -> Result<(), EvrenError> {
        self.parca_ekle()?;
        match olay {
            AkisOlayi::Metin(s) | AkisOlayi::Dusunce(s) => self.bayt_ekle(s.len()),
            AkisOlayi::AracCagrilari(v) => {
                self.cagri_ekle(v.len())?;
                for c in v {
                    for alan in [&c.id, &c.kind, &c.function.name, &c.function.arguments] {
                        self.bayt_ekle(alan.len())?;
                    }
                }
                Ok(())
            }
            AkisOlayi::Bitti { neden } => self.bayt_ekle(neden.len()),
            AkisOlayi::Kullanim(u) => self.bayt_ekle(
                u.evren
                    .as_ref()
                    .and_then(|e| e.routed_model.as_ref())
                    .map_or(0, String::len),
            ),
        }
    }
}
