//! Onay mührü (spec §7.6): onay, kullanıcıya gösterilen işlemin kendisine
//! bağlanır. Yürütme anında yeniden hesaplanır; tutmazsa işlem yapılmaz.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::{self, Read};

use crate::arac::Hazirlik;
use crate::kimlik::CagriId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OnayMuhru(pub String);

pub fn ozet_hex(veri: &[u8]) -> String {
    hex::encode(Sha256::digest(veri))
}

#[derive(Debug, PartialEq, Eq)]
pub struct OzetliOkuma {
    pub ozet: String,
    pub bayt: u64,
    /// Sınır aşılınca bırakılır; kısmi eski içerik geri alma için kullanılmaz.
    pub icerik: Option<Vec<u8>>,
    pub gorunmez_icerik: bool,
}

/// Sabit tamponla özetler; içerik yalnız saklama sınırına kadar tutulur.
/// Çağıranın iptal/okuma denetimi her parçadan önce çalışır.
pub fn akistan_ozetle(
    okuyucu: &mut impl Read,
    saklama_siniri: usize,
    mut denetle: impl FnMut() -> io::Result<()>,
) -> io::Result<OzetliOkuma> {
    let mut h = Sha256::new();
    let mut bayt = 0_u64;
    let mut icerik = Some(Vec::new());
    let mut tampon = [0_u8; 64 * 1024 + 3];
    let mut onceki_adet = 0;
    let mut gorunmez_icerik = false;
    loop {
        denetle()?;
        let adet = okuyucu.read(&mut tampon[3..])?;
        if adet == 0 {
            gorunmez_icerik |= crate::gorunum::gorunmez_var_mi(&String::from_utf8_lossy(
                &tampon[3 - onceki_adet..3],
            ));
            break;
        }
        bayt = bayt
            .checked_add(adet as u64)
            .ok_or_else(|| io::Error::other("Dosya boyutu taşması."))?;
        h.update(&tampon[3..3 + adet]);
        if !gorunmez_icerik {
            // Parçada bölünen UTF-8 ve CRLF sonraki parçada tamamlanır.
            let son = 3 + adet;
            let tarama_sonu = if tampon[son - 1] == b'\r' {
                son - 1
            } else {
                son
            };
            gorunmez_icerik = crate::gorunum::gorunmez_var_mi(&String::from_utf8_lossy(
                &tampon[3 - onceki_adet..tarama_sonu],
            ));
        }
        if let Some(veri) = &mut icerik {
            if adet > saklama_siniri.saturating_sub(veri.len()) {
                icerik = None;
            } else {
                veri.extend_from_slice(&tampon[3..3 + adet]);
            }
        }
        onceki_adet = (onceki_adet + adet).min(3);
        tampon.copy_within(3 + adet - onceki_adet..3 + adet, 3 - onceki_adet);
    }
    Ok(OzetliOkuma {
        ozet: hex::encode(h.finalize()),
        bayt,
        icerik,
        gorunmez_icerik,
    })
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
    if hazirlik.gorunmez_icerik {
        alan(&mut h, b"gorunmez-icerik");
    }

    let mut okunan: Vec<&str> = hazirlik.okunan.iter().map(|y| y.as_str()).collect();
    okunan.sort_unstable();
    for yol in okunan {
        alan(&mut h, b"o");
        alan(&mut h, yol.as_bytes());
    }
    let mut sayilar: Vec<_> = hazirlik.okuma_baglanti_sayilari.iter().collect();
    sayilar.sort_unstable();
    for (yol, sayi) in sayilar {
        alan(&mut h, b"ob");
        alan(&mut h, yol.as_str().as_bytes());
        alan(&mut h, &sayi.to_le_bytes());
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
    // (aşama 2 tasarımı §3.3). Biri değişirse onay geçersizdir; yalnızca
    // tükenen tek seferlik koşullar onayla kapsanır
    // (`KabukTarifi::onayla_kapsanan`).
    if let Some(kabuk) = &hazirlik.kabuk {
        alan(&mut h, b"k");
        alan(
            &mut h,
            serde_json::to_string(kabuk).unwrap_or_default().as_bytes(),
        );
    }
    // Eklenti: sürüm, özet, globlar, uçlar ve ilk gönderim (aşama 4 tasarımı
    // §8.3). Dosya sayısı ve bayt yalnızca bilgi amaçlıdır: mühre sıfırlanmış
    // bir kopya girer; dosyalar onaydan sonra değişince onay bozulmaz.
    if let Some(eklenti) = &hazirlik.eklenti {
        let mut muhurlu = eklenti.clone();
        muhurlu.dosya_sayisi = 0;
        muhurlu.bayt = 0;
        alan(&mut h, b"e");
        alan(
            &mut h,
            serde_json::to_string(&muhurlu)
                .unwrap_or_default()
                .as_bytes(),
        );
    }
    // MCP: R çalışan süreç tarifini, D araç tanımını bağlar. Argümanlar
    // yukarıda ayrıca mühürlenmiştir; karta hiçbir ham süreç girdisi girmez.
    if let Some(mcp) = &hazirlik.mcp {
        let calisma = mcp.tarif.calisma().sha256();
        let arac_tanimi = mcp.arac_tanimi.sha256();
        alan(&mut h, b"mcp-v1");
        alan(&mut h, &calisma);
        alan(&mut h, &arac_tanimi);
        if mcp.erisim == crate::arac::McpErisim::UzakStreamableHttp {
            alan(&mut h, b"mcp-uzak-v1");
            if let crate::arac::McpAgKarti::UzakHedef {
                host,
                origin,
                kimlik_kapsami,
            } = &mcp.ag
            {
                alan(&mut h, host.gorunen().as_bytes());
                alan(&mut h, origin.as_bytes());
                alan(&mut h, kimlik_kapsami.as_bytes());
            }
        }
    }
    if let Some(web) = &hazirlik.web {
        alan(&mut h, b"web-v1");
        alan(&mut h, &crate::arac::web_muhur_ozeti(web));
    }
    if hazirlik.kilitsiz {
        alan(&mut h, b"kilitsiz");
    }
    OnayMuhru(hex::encode(h.finalize()))
}
