//! Sınırlı MCP araç listesini tek tek güvenli tanım kartlarına dönüştürür.
//!
//! Sayfa/araç/bayt bütçesi istemci taşımasında zaten uygulanır. Bu modül o
//! ham listeyi tüketir; kabul edilmeyen kayıt için yalnız sıra numarası döner.

use std::collections::HashSet;

use serde_json::Value;

use crate::Tool;
use crate::maske::GizliDeger;
use crate::tanim_denetimi::{DenetlenmisAracTanimi, tanimi_denetle};

pub struct McpG7KesifSonucu {
    tanimlar: Vec<DenetlenmisAracTanimi>,
    reddedilen_siralar: Vec<usize>,
}

impl McpG7KesifSonucu {
    pub fn tanimlar(&self) -> &[DenetlenmisAracTanimi] {
        &self.tanimlar
    }

    pub fn parcalarina_ayir(self) -> (Vec<DenetlenmisAracTanimi>, Vec<usize>) {
        (self.tanimlar, self.reddedilen_siralar)
    }

    pub fn reddedilen_siralar(&self) -> &[usize] {
        &self.reddedilen_siralar
    }
}

/// Ham listeyi tüketir; ham Tool değerleri dönmez ve Debug/serde yüzeyi yoktur.
/// sunucu_model_adi, mcp__<sunucu>__<uzak-ad> birleşik adındaki model-safe ID'dir.
pub fn arac_tanimlarini_denetle(
    sunucu_model_adi: &str,
    ham_araclar: Vec<Tool>,
    sirlar: &[GizliDeger],
) -> McpG7KesifSonucu {
    let mut tanimlar = Vec::new();
    let mut reddedilen_siralar = Vec::new();
    let mut adlar = HashSet::new();

    for (indeks, ham) in ham_araclar.into_iter().enumerate() {
        let sira = indeks + 1;
        let girdi = Value::Object(ham.input_schema.as_ref().clone());
        let cikti = ham
            .output_schema
            .as_ref()
            .map(|sema| Value::Object(sema.as_ref().clone()));

        let sonuc = tanimi_denetle(
            sira,
            sunucu_model_adi,
            &ham.name,
            ham.description.as_deref(),
            &girdi,
            cikti.as_ref(),
            sirlar,
        );

        match sonuc {
            Ok(tanim) if adlar.insert(tanim.sunum().ad.clone()) => tanimlar.push(tanim),
            Ok(_) | Err(_) => reddedilen_siralar.push(sira),
        }
    }

    McpG7KesifSonucu {
        tanimlar,
        reddedilen_siralar,
    }
}

#[cfg(test)]
mod testler {
    use crate::Tool;
    use crate::maske::GizliDeger;

    use super::arac_tanimlarini_denetle;

    fn ham_arac(ad: &str) -> Tool {
        Tool::new(
            ad.to_owned(),
            "",
            std::sync::Arc::new(serde_json::Map::new()),
        )
    }

    #[test]
    fn yineleyen_birlesik_ad_sadece_sirasiyla_reddedilir() {
        let sonuc =
            arac_tanimlarini_denetle("yerel_srv", vec![ham_arac("ara"), ham_arac("ara")], &[]);

        assert_eq!(sonuc.tanimlar().len(), 1);
        assert_eq!(sonuc.tanimlar()[0].sunum().ad, "mcp__yerel_srv__ara");
        assert_eq!(sonuc.reddedilen_siralar(), &[2]);
    }

    #[test]
    fn sir_eslesen_ad_liste_donusunde_gorunmez() {
        let gizli = GizliDeger::yeni("SIR-TEST-123456".to_owned()).unwrap();
        let sonuc =
            arac_tanimlarini_denetle("yerel_srv", vec![ham_arac("SIR-TEST-123456")], &[gizli]);

        assert!(sonuc.tanimlar().is_empty());
        assert_eq!(sonuc.reddedilen_siralar(), &[1]);
    }
}
