//! Gönderim günlüğü: gönderilen öğeler, istek sonucu ve günlük türetimi (spec §6).

use std::collections::HashMap;

use kalfa_evren::EkUc;
use serde::{Deserialize, Serialize};

use crate::butce::ButceIslemId;
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
    /// İstek ağa çıkmadı: kullanıcı önizlemede göndermedi ya da oturumun
    /// eklenti sınırı doldu (aşama 4 tasarımı §9.4, §9.5).
    Gonderilmedi {
        neden: String,
    },
}

/// Bir eklenti isteğinde listelenen okunan dosyaların sınırı; fazlası
/// `dosya_fazlasi` olarak sayılır (aşama 4 tasarımı §9.3).
pub const OKUNAN_SINIRI: usize = 1_000;

/// Bir eklentinin bir oturumdaki EVREN istek sayısı üst sınırı (aşama 4
/// tasarımı §9.5). Sınırı kaldırmanın ya da değiştirmenin bir ayarı yoktur.
pub const EKLENTI_ISTEK_SINIRI: u64 = 5_000;
/// Bir eklentinin bir oturumda gönderebileceği gövdelerin toplam bayt sınırı
/// (aşama 4 tasarımı §9.5).
pub const EKLENTI_BAYT_SINIRI: u64 = 256 * 1024 * 1024;

/// Bir eklentinin bir oturumdaki EVREN kullanımı: ağa çıkmasına izin verilen
/// istek sayısı ve gövde baytı (aşama 4 tasarımı §9.5). Oturumla sıfırlanır;
/// kurtarılan oturumda kayıttaki `EklentiIstegi` öğelerinden yeniden kurulur.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EklentiKullanimi {
    pub istek: u64,
    pub bayt: u64,
}

impl EklentiKullanimi {
    /// `bayt` büyüklüğünde yeni bir istek bu kullanıma eklenirse sınırlardan
    /// biri aşılır mı? Sınır isteği ağa çıkmadan keser.
    pub fn sinir_asilir_mi(&self, bayt: u64) -> bool {
        self.istek.saturating_add(1) > EKLENTI_ISTEK_SINIRI
            || self.bayt.saturating_add(bayt) > EKLENTI_BAYT_SINIRI
    }

    /// Ağa çıkmasına izin verilen bir isteği sayar.
    pub fn say(&mut self, bayt: u64) {
        self.istek = self.istek.saturating_add(1);
        self.bayt = self.bayt.saturating_add(bayt);
    }
}

/// Eklentinin EVREN isteğinin kayda, günlüğe ve önizlemeye giren özeti (aşama
/// 4 tasarımı §9.3). Metnin kendisi kayda yazılmaz (karar 8): içeriğin kaynağı
/// o çağrıda okunan dosyaların listesiyle anlatılır.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EklentiIstekOzeti {
    pub eklenti: String,
    pub uc: EkUc,
    pub model: String,
    /// `input` ya da `documents` içindeki metin sayısı (sıralamada sorgu
    /// dahil değil).
    pub metin_sayisi: u32,
    /// Gönderilen gövdenin bayt sayısı.
    pub bayt: u64,
    /// Metinlerin SHA-256 özeti, onaltılık.
    pub ozet: String,
    /// O araç çağrısında o ana kadar okunan dosyalar, en çok `OKUNAN_SINIRI`.
    pub dosyalar: Vec<String>,
    #[serde(default)]
    pub dosya_fazlasi: u32,
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
    /// Eklenti isteğinin özeti (aşama 4 tasarımı §9.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eklenti: Option<EklentiIstekOzeti>,
}

/// Gönderim günlüğünün bir sayfası (spec §6.5, plan Task 4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GunlukSayfasi {
    pub satirlar: Vec<GunlukSatiri>,
    pub daha_eski_var: bool,
}

/// Kayıtlardan gönderim günlüğü satırlarını üretir.
///
/// Bütçe kaydı olan (`butce_islem`'li) satırın tokenı yalnızca kendi denemesinin
/// `ButceIstegiBitti` kaydından gelir (5b tasarımı §6); komşu bir bitişi ya da
/// `Kullanim` kaydını almaz. Bütçesiz eski sohbet satırına, eski kayıt gibi,
/// izleyen `Kullanim` kaydı iliştirilir.
pub fn gunluk_uret(kayitlar: &[Zarf<KayitOgesi>]) -> Vec<GunlukSatiri> {
    let butce_bitisleri: HashMap<ButceIslemId, Option<GunlukToken>> = kayitlar
        .iter()
        .filter_map(|zarf| match &zarf.yuk {
            KayitOgesi::ButceIstegiBitti { islem, kullanim } => Some((*islem, *kullanim)),
            _ => None,
        })
        .collect();
    let butce_tokeni = |islem: Option<ButceIslemId>| {
        islem.and_then(|islem| butce_bitisleri.get(&islem).copied().flatten())
    };

    let mut satirlar: Vec<GunlukSatiri> = Vec::new();
    // Son satır bütçesiz bir sohbet satırıysa eski `Kullanim` ona iliştirilir.
    let mut eski_sohbet_satiri = false;

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
                butce_islem,
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
                    token: butce_tokeni(*butce_islem),
                    sonuc: sonuc.clone(),
                    yeni: yeni.clone(),
                    onceki: *onceki,
                    eklenti: None,
                });
                eski_sohbet_satiri = butce_islem.is_none();
            }
            KayitOgesi::EklentiIstegi {
                istek,
                istek_id,
                kredi,
                yonlendirilen_model,
                sonuc,
                butce_islem,
                ..
            } => {
                satirlar.push(GunlukSatiri {
                    sira: zarf.sira,
                    tur: zarf.tur,
                    zaman: kredi.as_ref().map(|k| k.zaman_ms),
                    amac: format!("eklenti:{}", istek.eklenti),
                    model: Some(istek.model.clone()),
                    yonlendirilen_model: yonlendirilen_model.clone(),
                    istek_id: istek_id.clone(),
                    token: butce_tokeni(*butce_islem),
                    sonuc: sonuc.clone(),
                    yeni: vec![GonderilenOge {
                        kaynak: format!("eklenti:{}:{}", istek.eklenti, istek.uc.etiket()),
                        sira: None,
                        bayt: istek.bayt,
                    }],
                    onceki: 0,
                    eklenti: Some(istek.clone()),
                });
                eski_sohbet_satiri = false;
            }
            // Sonucu ve token kullanımı yalnızca sohbet satırına iliştirilir;
            // eklenti satırı sohbet isteği değildir.
            KayitOgesi::IstekKoptu { mesaj } => {
                if let Some(son) = satirlar.last_mut().filter(|s| s.eklenti.is_none()) {
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
                if eski_sohbet_satiri && let Some(son) = satirlar.last_mut() {
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
        KayitOgesi::BaglamOzeti { icerik, .. } => Some(icerik.clone()),
        _ => None,
    }
}
