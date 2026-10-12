//! İndeksin depodaki düzeni (aşama 4 tasarımı §11.2).
//!
//! - `meta` (`META`): sürüm, model, boyut, son indeksleme zamanı, sıradaki
//!   dilim numarası, silinecek dilimler ve dosya tablosu. Tablo dosya başına
//!   boyut, değişme zamanı ve parçaları; parça başına satır ve bayt aralığını,
//!   metin özetini ve vektörün yerini (dilim, sıra) tutar. **Metin
//!   saklanmaz** (karar 11).
//! - `vektor/<n>` (`dilim_anahtari`): bir toplu isteğin vektörleri, en çok
//!   256 × `BOYUT` küçük endian `f32`. `n` artarak verilir, yeniden
//!   kullanılmaz; `meta`'nın gösterdiği bir dilimin üzerine yazılmaz.

use std::collections::BTreeMap;

use kalfa_plugin_sdk::Hata;
use serde::{Deserialize, Serialize};

pub const META: &str = "meta";
/// Biçim değişirse artar; uymayan `meta` yok sayılır ve indeks baştan kurulur.
pub const SURUM: u32 = 1;

pub fn dilim_anahtari(n: u64) -> String {
    format!("vektor/{n}")
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Meta {
    pub surum: u32,
    pub model: String,
    pub boyut: u32,
    /// Son indekslemenin başladığı an (Unix ms). "Racy clean" kuralı buna
    /// göre işler.
    pub son_indeksleme: u64,
    /// Sıradaki toplu isteğin dilim numarası.
    pub siradaki_dilim: u64,
    /// Artık hiçbir parçanın göstermediği dilimler; bir sonraki `meta`
    /// yazmasında boşaltılır.
    pub silinecekler: Vec<u64>,
    /// Yola göre; yalnızca bütün parçaları gömülmüş dosyalar.
    pub dosyalar: BTreeMap<String, DosyaKaydi>,
    /// Bitmemiş dosyaların yazılmış parçaları. Eski indekslerde bu alan yoktur.
    /// Metin tutulmaz; sonraki çağrı yeniden okuduğu parçaları özetle eşler.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bekleyen: BTreeMap<String, DosyaKaydi>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DosyaKaydi {
    pub boyut: u64,
    /// Unix ms.
    pub degisme: u64,
    pub parcalar: Vec<ParcaKaydi>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ParcaKaydi {
    /// 1'den başlayan, iki uç dahil satır aralığı.
    pub satir_bas: u32,
    pub satir_son: u32,
    /// Dosya içeriğinde bayt aralığı `[bas_bayt, son_bayt)`. Arama metni
    /// buradan yeniden okur ve özetle doğrular.
    pub bas_bayt: u64,
    pub son_bayt: u64,
    /// Parça metninin SHA-256'sı, onaltılık.
    pub ozet: String,
    /// Vektörün yeri: `vektor/<dilim>` içindeki sırası.
    pub dilim: u64,
    pub sira: u32,
}

impl Meta {
    /// Boş indeks.
    pub fn yeni() -> Self {
        Self {
            surum: SURUM,
            model: crate::ortam::GOMME_MODELI.into(),
            boyut: crate::ortam::BOYUT as u32,
            ..Default::default()
        }
    }

    pub fn kodla(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Çözülemeyen ya da sürümü, modeli, boyutu uymayan `meta` `None` olur:
    /// indeks baştan kurulur.
    pub fn coz(veri: &[u8]) -> Option<Self> {
        let m: Self = serde_json::from_slice(veri).ok()?;
        (m.surum == SURUM
            && m.model == crate::ortam::GOMME_MODELI
            && m.boyut as usize == crate::ortam::BOYUT)
            .then_some(m)
    }
}

/// Vektörleri dilime kodlar: art arda küçük endian `f32`.
pub fn dilim_kodla(vektorler: &[Vec<f32>]) -> Vec<u8> {
    vektorler
        .iter()
        .flat_map(|v| v.iter().flat_map(|x| x.to_le_bytes()))
        .collect()
}

/// Dilimi `boyut` boyutlu vektörlere çözer.
pub fn dilim_coz(veri: &[u8], boyut: usize) -> Result<Vec<Vec<f32>>, Hata> {
    let satir = boyut * 4;
    if satir == 0 || !veri.len().is_multiple_of(satir) {
        return Err(Hata::Diger("vektör dilimi bozuk".into()));
    }
    Ok(veri
        .chunks_exact(satir)
        .map(|s| {
            s.as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b))
                .collect()
        })
        .collect())
}
