//! Onay mührü (spec §7.6): onay, kullanıcıya gösterilen işlemin kendisine
//! bağlanır. Yürütme anında yeniden hesaplanır; tutmazsa işlem yapılmaz.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::arac::Hazirlik;
use crate::kimlik::CagriId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OnayMuhru(pub String);

pub fn ozet_hex(veri: &[u8]) -> String {
    hex::encode(Sha256::digest(veri))
}

fn alan(h: &mut Sha256, veri: &[u8]) {
    h.update((veri.len() as u64).to_le_bytes());
    h.update(veri);
}

pub fn muhur_hesapla(
    cagri: CagriId,
    arac: &str,
    argumanlar: &Value,
    hazirlik: &Hazirlik,
) -> OnayMuhru {
    let mut h = Sha256::new();
    alan(&mut h, b"kalfa-muhur-v1");
    alan(&mut h, cagri.to_string().as_bytes());
    alan(&mut h, arac.as_bytes());
    alan(&mut h, argumanlar.to_string().as_bytes());

    let mut okunan: Vec<&str> = hazirlik.okunan.iter().map(|y| y.as_str()).collect();
    okunan.sort_unstable();
    for yol in okunan {
        alan(&mut h, b"o");
        alan(&mut h, yol.as_bytes());
    }
    let mut yazilan: Vec<_> = hazirlik.yazilan.iter().collect();
    yazilan.sort_by(|a, b| a.yol.cmp(&b.yol));
    for hedef in yazilan {
        alan(&mut h, b"y");
        alan(&mut h, hedef.yol.as_str().as_bytes());
        alan(
            &mut h,
            hedef.onceki_ozet.as_deref().unwrap_or("-").as_bytes(),
        );
        alan(&mut h, &hedef.baglanti_sayisi.to_le_bytes());
    }
    alan(
        &mut h,
        if hazirlik.tum_calisma_alani {
            b"1"
        } else {
            b"0"
        },
    );
    // Kabuk: komut, dizin, ağ, yalıtım durumu ve profil özeti, ilk çağrı
    // (aşama 2 tasarımı §3.3). Biri değişirse onay geçersizdir.
    if let Some(kabuk) = &hazirlik.kabuk {
        alan(&mut h, b"k");
        alan(
            &mut h,
            serde_json::to_string(kabuk).unwrap_or_default().as_bytes(),
        );
    }
    OnayMuhru(hex::encode(h.finalize()))
}
