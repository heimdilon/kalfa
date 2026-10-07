//! "Bu oturumda izin ver" kısayolları (aşama 3 tasarımı §9.1).
//!
//! Teklifi çekirdek yapar; webview yalnızca kabul edebilir. Kısayol yalnızca
//! kararı izin kipi yüzünden onaylı olan (uyarısız) öğeye verilir. Oturum
//! boyunca yaşar, izin kipi değişince silinir, kayda yazılmaz. Bir
//! kolaylıktır, güvenlik sınırı değildir (üst belge §7.2).

use serde::{Deserialize, Serialize};

use crate::arac::Hazirlik;
use crate::izin::Karar;
use crate::kabuk::{KabukBicimi, YalitimOzeti};

/// Dosya kısayolu alabilen araçlar.
const DOSYA_ARACLARI: &[&str] = &["dosya_yaz", "dosya_duzenle"];

/// Kısayolun kapsamı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum KisayolTeklifi {
    /// `arac` ile `dizin` ve altındaki dosyalara yazma; kökte `dizin` `.`'dır.
    Dosya { arac: String, dizin: String },
    /// `program`'ı `dizin`'de yalıtımlı ve ağsız çalıştırma; argümanlar serbest.
    Kabuk { program: String, dizin: String },
}

impl KisayolTeklifi {
    /// Bu kısayol `arac`'ın `hazirlik`'ını kapsıyor mu? Yalnızca kararı
    /// uyarısız onaylı olan çağrıda sorulur (`Oturum::planla`).
    pub fn kapsar(&self, arac: &str, h: &Hazirlik) -> bool {
        match self {
            KisayolTeklifi::Dosya { arac: a, dizin } => {
                a == arac
                    && h.kabuk.is_none()
                    && !h.yazilan.is_empty()
                    && h.yazilan.iter().all(|y| dizinde(y.yol.as_str(), dizin))
            }
            KisayolTeklifi::Kabuk { program, dizin } => h.kabuk.as_ref().is_some_and(|k| {
                k.bicim == KabukBicimi::Program
                    && k.program.as_deref() == Some(program.as_str())
                    && &k.dizin == dizin
                    && !k.ag
                    && matches!(k.yalitim, YalitimOzeti::Yalitimli { .. })
            }),
        }
    }
}

/// `yol` `dizin`'in içinde mi? Yol bileşenleriyle karşılaştırılır: `src`
/// `srcx/a`'yı kapsamaz.
fn dizinde(yol: &str, dizin: &str) -> bool {
    dizin == "." || yol.strip_prefix(dizin).is_some_and(|k| k.starts_with('/'))
}

/// Onay kartında önerilecek kısayol. Yalnızca karar `Onayli` ve uyarı listesi
/// boşsa yapılır; uyarı taşıyan hiçbir öğe kısayol almaz (§9.1).
pub fn teklif(arac: &str, h: &Hazirlik, karar: &Karar) -> Option<KisayolTeklifi> {
    if !matches!(karar, Karar::Onayli { uyarilar } if uyarilar.is_empty()) {
        return None;
    }
    if let Some(k) = &h.kabuk {
        let uygun = k.bicim == KabukBicimi::Program
            && matches!(k.yalitim, YalitimOzeti::Yalitimli { .. })
            && !k.ag
            && !k.ilk_cagri
            && !k.alan_hazirligi;
        if !uygun {
            return None;
        }
        return k.program.clone().map(|program| KisayolTeklifi::Kabuk {
            program,
            dizin: k.dizin.clone(),
        });
    }
    if !DOSYA_ARACLARI.contains(&arac) || h.yazilan.len() != 1 {
        return None;
    }
    let yol = h.yazilan[0].yol.as_str();
    let dizin = yol.rsplit_once('/').map_or(".", |(d, _)| d);
    Some(KisayolTeklifi::Dosya {
        arac: arac.to_string(),
        dizin: dizin.to_string(),
    })
}

/// Oturumun listesindeki bir kısayol. Cümlesini arayüz kurar (`tr.ts`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kisayol {
    pub kimlik: u64,
    pub teklif: KisayolTeklifi,
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn dizin_bilesenlerle_karsilastirilir() {
        assert!(dizinde("src/a.rs", "src"));
        assert!(dizinde("src/alt/a.rs", "src"));
        assert!(!dizinde("srcx/a.rs", "src"));
        assert!(!dizinde("src", "src"));
        assert!(dizinde("a.rs", "."));
    }
}
