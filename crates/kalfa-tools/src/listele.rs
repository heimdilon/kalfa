//! `dosya_listele`: glob desenine uyan dosyalar.

use std::sync::Arc;

use async_trait::async_trait;
use globset::GlobBuilder;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, Hazirlik};
use kalfa_core::protokol::VeriCikisi;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::arguman;
use crate::calisma_alani::CalismaAlani;

const AZAMI_DOSYA: usize = 1000;

pub struct DosyaListele {
    alan: Arc<CalismaAlani>,
}

impl DosyaListele {
    pub fn yeni(alan: Arc<CalismaAlani>) -> Self {
        Self { alan }
    }
}

#[async_trait]
impl Arac for DosyaListele {
    fn ad(&self) -> &str {
        "dosya_listele"
    }

    fn aciklama(&self) -> &str {
        "Çalışma alanında bir glob desenine uyan dosyaları listeler (örn. `src/**/*.rs`). `.gitignore` kapsamındakiler listelenmez; `.git` dizini atlanır."
    }

    fn parametre_semasi(&self) -> Value {
        json!({
            "type": "object",
            "properties": {"desen": {"type": "string", "description": "Glob deseni, örn. `src/**/*.rs`"}},
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
            aciklama: format!("`{desen}` desenine uyan dosyaları listelemek istiyor."),
            veri_cikisi: Some(VeriCikisi::DosyaAdlari),
            ..Default::default()
        })
    }

    async fn calistir(
        &self,
        a: &Value,
        _h: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        let desen = arguman::metin(a, "desen")?;
        let eslestirici = GlobBuilder::new(desen)
            .literal_separator(true)
            .build()
            .map_err(|e| AracHatasi::Gecersiz(format!("Glob deseni geçersiz: {e}")))?
            .compile_matcher();
        let alan = self.alan.clone();
        let dosyalar = tokio::task::spawn_blocking(move || alan.dosyalar(&iptal))
            .await
            .map_err(|e| AracHatasi::GirisCikis(e.to_string()))??;

        let mut satirlar = Vec::new();
        let mut toplam = 0usize;
        for yol in dosyalar.iter().filter(|y| eslestirici.is_match(y.as_str())) {
            toplam += 1;
            if satirlar.len() < AZAMI_DOSYA {
                if self.alan.siniflandirici().gizli_mi(yol) {
                    satirlar.push(format!("{yol} (gizli: içeriği onay ister)"));
                } else {
                    satirlar.push(yol.to_string());
                }
            }
        }
        let mut icerik = if satirlar.is_empty() {
            "Desene uyan dosya yok.".to_string()
        } else {
            satirlar.join("\n")
        };
        if toplam > satirlar.len() {
            icerik.push_str(&format!(
                "\n[{toplam} dosyanın ilk {} tanesi gösterildi]",
                satirlar.len()
            ));
        }
        Ok(AracCiktisi {
            icerik,
            kaynak: format!("listeleme:{desen}"),
            guvenlik_uyarilari: Vec::new(),
            degisiklikler: None,
        })
    }
}
