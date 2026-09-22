//! `icerik_ara`: düzenli ifadeyle içerik arama.
//!
//! Gizli dosyaların içeriği aranmaz (spec §7.4); yalnızca kaç tane atlandığı
//! bildirilir.

use std::sync::Arc;

use async_trait::async_trait;
use globset::{GlobBuilder, GlobMatcher};
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, Hazirlik};
use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::arguman;
use crate::calisma_alani::{CalismaAlani, ikili_mi};

const AZAMI_ESLESME: usize = 200;
const AZAMI_SATIR_UZUNLUGU: usize = 300;

pub struct IcerikAra {
    alan: Arc<CalismaAlani>,
}

impl IcerikAra {
    pub fn yeni(alan: Arc<CalismaAlani>) -> Self {
        Self { alan }
    }
}

struct AramaSonucu {
    satirlar: Vec<String>,
    kesildi: bool,
    gizli_atlanan: usize,
}

#[async_trait]
impl Arac for IcerikAra {
    fn ad(&self) -> &str {
        "icerik_ara"
    }

    fn aciklama(&self) -> &str {
        "Çalışma alanındaki metin dosyalarında düzenli ifade arar; `yol:satır: metin` biçiminde döner. `dosya_deseni` ile aranacak dosyaları daralt."
    }

    fn parametre_semasi(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "desen": {"type": "string", "description": "Düzenli ifade (Rust regex sözdizimi)"},
                "dosya_deseni": {"type": "string", "description": "İsteğe bağlı glob, örn. `**/*.rs`"},
                "buyuk_kucuk_harf_duyarsiz": {"type": "boolean"}
            },
            "required": ["desen"],
            "additionalProperties": false
        })
    }

    fn yazar_mi(&self) -> bool {
        false
    }

    async fn hazirla(&self, a: &Value) -> Result<Hazirlik, AracHatasi> {
        let desen = arguman::metin(a, "desen")?;
        Ok(Hazirlik {
            aciklama: format!("`{desen}` ifadesini çalışma alanında aramak istiyor."),
            ..Default::default()
        })
    }

    async fn calistir(
        &self,
        a: &Value,
        _h: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        let desen = arguman::metin(a, "desen")?.to_string();
        let ifade = RegexBuilder::new(&desen)
            .case_insensitive(arguman::mantiksal(a, "buyuk_kucuk_harf_duyarsiz"))
            .size_limit(1 << 20)
            .build()
            .map_err(|e| AracHatasi::Gecersiz(format!("Düzenli ifade geçersiz: {e}")))?;
        let dosya_eslestirici = match a.get("dosya_deseni").and_then(Value::as_str) {
            Some(d) => Some(
                GlobBuilder::new(d)
                    .literal_separator(true)
                    .build()
                    .map_err(|e| AracHatasi::Gecersiz(format!("Glob deseni geçersiz: {e}")))?
                    .compile_matcher(),
            ),
            None => None,
        };
        let alan = self.alan.clone();
        let s = tokio::task::spawn_blocking(move || {
            ara(&alan, &ifade, dosya_eslestirici.as_ref(), &iptal)
        })
        .await
        .map_err(|e| AracHatasi::GirisCikis(e.to_string()))??;

        let mut icerik = if s.satirlar.is_empty() {
            "Eşleşme yok.".to_string()
        } else {
            s.satirlar.join("\n")
        };
        if s.kesildi {
            icerik.push_str(&format!("\n[ilk {AZAMI_ESLESME} eşleşme gösterildi]"));
        }
        if s.gizli_atlanan > 0 {
            icerik.push_str(&format!(
                "\n[{} gizli dosya aranmadı: içeriklerini okumak onay ister]",
                s.gizli_atlanan
            ));
        }
        Ok(AracCiktisi {
            icerik,
            kaynak: format!("arama:{desen}"),
        })
    }
}

fn ara(
    alan: &CalismaAlani,
    ifade: &Regex,
    dosya_eslestirici: Option<&GlobMatcher>,
    iptal: &CancellationToken,
) -> Result<AramaSonucu, AracHatasi> {
    let mut s = AramaSonucu {
        satirlar: Vec::new(),
        kesildi: false,
        gizli_atlanan: 0,
    };
    'dosyalar: for yol in alan.dosyalar(iptal)? {
        if iptal.is_cancelled() {
            return Err(AracHatasi::Iptal);
        }
        if dosya_eslestirici.is_some_and(|e| !e.is_match(yol.as_str())) {
            continue;
        }
        if alan.siniflandirici().gizli_mi(&yol) {
            s.gizli_atlanan += 1;
            continue;
        }
        let Ok(veri) = alan.oku(&yol) else { continue };
        if ikili_mi(&veri) {
            continue;
        }
        let metin = String::from_utf8_lossy(&veri);
        for (no, satir) in metin.lines().enumerate() {
            if ifade.is_match(satir) {
                if s.satirlar.len() >= AZAMI_ESLESME {
                    s.kesildi = true;
                    break 'dosyalar;
                }
                s.satirlar
                    .push(format!("{yol}:{}: {}", no + 1, kisalt(satir)));
            }
        }
    }
    Ok(s)
}

fn kisalt(satir: &str) -> String {
    if satir.chars().count() <= AZAMI_SATIR_UZUNLUGU {
        satir.to_string()
    } else {
        let mut k: String = satir.chars().take(AZAMI_SATIR_UZUNLUGU).collect();
        k.push('…');
        k
    }
}
