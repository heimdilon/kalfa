//! Akış parçalarını (`ChatChunk`) Kalfa'nın kullandığı olaylara çevirir.
//!
//! Araç çağrıları `index` bazında biriktirilir ve yalnızca `finish_reason`
//! geldiğinde, tamamlanmış olarak yayılır: tamamlanmamış bir çağrı asla
//! çalıştırılmaz (spec §5.1).

use std::collections::BTreeMap;

use crate::butce::{YANIT_BAYT_SINIRI, YanitButcesi};
use crate::hata::EvrenError;
use crate::tipler::{ChatChunk, FunctionCall, ToolCall, Usage};

#[derive(Debug, Clone, PartialEq)]
pub enum AkisOlayi {
    Metin(String),
    Dusunce(String),
    AracCagrilari(Vec<ToolCall>),
    Kullanim(Usage),
    Bitti { neden: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum VeriSonucu {
    Olaylar(Vec<AkisOlayi>),
    Bitti,
}

#[derive(Debug, Default)]
struct KismiCagri {
    id: Option<String>,
    ad: Option<String>,
    argumanlar: String,
}

#[derive(Debug, Default)]
pub struct AkisBirlestirici {
    cagrilar: BTreeMap<u32, KismiCagri>,
    neden: Option<String>,
    bitirildi: bool,
    butce: YanitButcesi,
}

impl AkisBirlestirici {
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir SSE olayının `data` alanını işler.
    pub fn veri_isle(&mut self, veri: &str) -> Result<VeriSonucu, EvrenError> {
        self.butce.denetle()?;
        if veri.len() > YANIT_BAYT_SINIRI {
            self.butce.bayt_ekle(veri.len())?;
        }
        if veri.trim() == "[DONE]" {
            return Ok(VeriSonucu::Bitti);
        }
        let parca: ChatChunk = serde_json::from_str(veri)
            .map_err(|e| EvrenError::GecersizYanit(format!("akış parçası çözülemedi: {e}")))?;
        Ok(VeriSonucu::Olaylar(self.ekle(parca)?))
    }

    pub fn ekle(&mut self, parca: ChatChunk) -> Result<Vec<AkisOlayi>, EvrenError> {
        self.butce.girdi_ekle()?;
        let mut olaylar = Vec::new();
        for secim in parca.choices.into_iter().filter(|s| s.index == 0) {
            let delta = secim.delta;
            if let Some(r) = delta.reasoning.filter(|s| !s.is_empty()) {
                self.butce.bayt_ekle(r.len())?;
                self.butce.parca_ekle()?;
                olaylar.push(AkisOlayi::Dusunce(r));
            }
            if let Some(m) = delta.content.filter(|s| !s.is_empty()) {
                self.butce.bayt_ekle(m.len())?;
                self.butce.parca_ekle()?;
                olaylar.push(AkisOlayi::Metin(m));
            }
            for tc in delta.tool_calls.unwrap_or_default() {
                if !self.cagrilar.contains_key(&tc.index) {
                    self.butce.cagri_ekle(1)?;
                }
                let k = self.cagrilar.entry(tc.index).or_default();
                if let Some(id) = tc.id.filter(|s| !s.is_empty()) {
                    self.butce.bayt_ekle(id.len())?;
                    k.id = Some(id);
                }
                if let Some(f) = tc.function {
                    if let Some(ad) = f.name.filter(|s| !s.is_empty()) {
                        self.butce.bayt_ekle(ad.len())?;
                        k.ad = Some(ad);
                    }
                    if let Some(a) = f.arguments {
                        self.butce.bayt_ekle(a.len())?;
                        k.argumanlar.push_str(&a);
                    }
                }
            }
            if let Some(neden) = secim.finish_reason
                && self.neden.is_none()
            {
                self.butce.bayt_ekle(neden.len())?;
                let cagrilar = self.tamamla()?;
                if !cagrilar.is_empty() {
                    self.butce.parca_ekle()?;
                    olaylar.push(AkisOlayi::AracCagrilari(cagrilar));
                }
                self.neden = Some(neden);
            }
        }
        if let Some(u) = parca.usage {
            self.butce.parca_ekle()?;
            self.butce.bayt_ekle(
                u.evren
                    .as_ref()
                    .and_then(|e| e.routed_model.as_ref())
                    .map_or(0, String::len),
            )?;
            olaylar.push(AkisOlayi::Kullanim(u));
        }
        Ok(olaylar)
    }

    /// Akış bitti (`[DONE]` ya da bağlantı kapandı).
    pub fn bitir(&mut self) -> Result<Vec<AkisOlayi>, EvrenError> {
        self.butce.denetle()?;
        if self.bitirildi {
            return Ok(Vec::new());
        }
        self.bitirildi = true;
        match self.neden.clone() {
            Some(neden) => {
                self.butce.parca_ekle()?;
                Ok(vec![AkisOlayi::Bitti { neden }])
            }
            None => Err(EvrenError::AkisYarimKaldi),
        }
    }

    fn tamamla(&mut self) -> Result<Vec<ToolCall>, EvrenError> {
        std::mem::take(&mut self.cagrilar)
            .into_iter()
            .map(|(index, k)| {
                let id = k.id.ok_or_else(|| {
                    EvrenError::GecersizYanit(format!("{index}. araç çağrısının kimliği yok"))
                })?;
                let name = k.ad.ok_or_else(|| {
                    EvrenError::GecersizYanit(format!("{index}. araç çağrısının adı yok"))
                })?;
                Ok(ToolCall {
                    id,
                    kind: "function".to_string(),
                    function: FunctionCall {
                        name,
                        arguments: k.argumanlar,
                    },
                })
            })
            .collect()
    }
}
