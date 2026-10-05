//! Gönderim günlüğü: gönderilen öğeler, istek sonucu ve günlük türetimi (spec §6).

use serde::{Deserialize, Serialize};

use crate::kayit::KayitOgesi;
use crate::kimlik::TurId;
use crate::protokol::Zarf;

/// Bir istekte gönderilen bir öğe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GonderilenOge {
    pub kaynak: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sira: Option<u64>,
    pub bayt: u64,
}

/// İstek denemesinin hata aşaması.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IstekAsamasi {
    Baglanti,
    Yanit,
}

/// Bir istek denemesinin sonucu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "tip")]
pub enum IstekSonucu {
    #[default]
    Basarili,
    Hata {
        asama: IstekAsamasi,
        mesaj: String,
    },
}

/// Bir istekte kullanılan token miktarı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GunlukToken {
    pub istem: u64,
    pub tamamlama: u64,
    pub dusunce: u64,
}

impl GunlukToken {
    pub fn toplam(&self) -> u64 {
        self.istem.saturating_add(self.tamamlama)
    }
}

/// Gönderim günlüğünün bir satırı (spec §6.2, plan Task 4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GunlukSatiri {
    pub sira: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tur: Option<TurId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zaman: Option<u64>,
    pub amac: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yonlendirilen_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub istek_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<GunlukToken>,
    pub sonuc: IstekSonucu,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub yeni: Vec<GonderilenOge>,
    #[serde(default)]
    pub onceki: u32,
}

/// Gönderim günlüğünün bir sayfası (spec §6.5, plan Task 4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GunlukSayfasi {
    pub satirlar: Vec<GunlukSatiri>,
    pub daha_eski_var: bool,
}

/// Kayıtlardan gönderim günlüğü satırlarını üretir.
pub fn gunluk_uret(kayitlar: &[Zarf<KayitOgesi>]) -> Vec<GunlukSatiri> {
    let mut satirlar: Vec<GunlukSatiri> = Vec::new();

    for zarf in kayitlar {
        match &zarf.yuk {
            KayitOgesi::Istek {
                amac,
                istek_id,
                kredi,
                yonlendirilen_model,
                model,
                yeni,
                onceki,
                sonuc,
            } => {
                let zaman = kredi.as_ref().map(|k| k.zaman_ms);
                satirlar.push(GunlukSatiri {
                    sira: zarf.sira,
                    tur: zarf.tur,
                    zaman,
                    amac: amac.clone(),
                    model: model.clone(),
                    yonlendirilen_model: yonlendirilen_model.clone(),
                    istek_id: istek_id.clone(),
                    token: None,
                    sonuc: sonuc.clone(),
                    yeni: yeni.clone(),
                    onceki: *onceki,
                });
            }
            KayitOgesi::IstekKoptu { mesaj } => {
                if let Some(son) = satirlar.last_mut() {
                    son.sonuc = IstekSonucu::Hata {
                        asama: IstekAsamasi::Yanit,
                        mesaj: mesaj.clone(),
                    };
                }
            }
            KayitOgesi::Kullanim {
                istem,
                tamamlama,
                dusunce,
            } => {
                if let Some(son) = satirlar.last_mut() {
                    son.token = Some(GunlukToken {
                        istem: *istem,
                        tamamlama: *tamamlama,
                        dusunce: *dusunce,
                    });
                }
            }
            _ => {}
        }
    }

    satirlar
}

/// Günlük satırlarını sayfalar (`gecmis::sayfa` gibi).
pub fn gunluk_sayfasi(
    satirlar: &[GunlukSatiri],
    once_sira: Option<u64>,
    adet: usize,
) -> GunlukSayfasi {
    let son = match once_sira {
        Some(s) => satirlar.partition_point(|satir| satir.sira < s),
        None => satirlar.len(),
    };
    let bas = son.saturating_sub(adet);
    GunlukSayfasi {
        satirlar: satirlar[bas..son].to_vec(),
        daha_eski_var: bas > 0,
    }
}

/// Kayıtlardaki bir sıranın tam metnini döndürür.
pub fn oge_metni(kayitlar: &[Zarf<KayitOgesi>], sira: u64) -> Option<String> {
    let zarf = kayitlar.iter().find(|z| z.sira == sira)?;
    match &zarf.yuk {
        KayitOgesi::Mesaj { mesaj, .. } => {
            if let Some(ref c) = mesaj.content {
                Some(c.clone())
            } else if let Some(ref r) = mesaj.reasoning {
                Some(r.clone())
            } else if let Some(ref tc) = mesaj.tool_calls {
                serde_json::to_string_pretty(tc).ok()
            } else {
                Some(String::new())
            }
        }
        KayitOgesi::SistemIstemi { metin } => Some(metin.clone()),
        KayitOgesi::AracTanimlari { tanimlar } => serde_json::to_string_pretty(tanimlar).ok(),
        KayitOgesi::KalfaNotu { metin } => Some(metin.clone()),
        _ => None,
    }
}
