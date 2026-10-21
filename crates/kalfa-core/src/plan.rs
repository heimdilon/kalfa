use std::collections::HashSet;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::arac::{Arac, AracCiktisi, AracHatasi, AracOrtami, Hazirlik};
use crate::kimlik::TurId;

/// Bir oturum planında bulunabilecek azami adım sayısı.
pub const AZAMI_PLAN_ADIMI: usize = 32;

/// Bir plan adımının başlığında bulunabilecek azami karakter sayısı.
pub const AZAMI_ADIM_METNI_KARAKTER: usize = 200;

/// Bir plan adımının kimliğinde bulunabilecek azami karakter sayısı.
pub const AZAMI_ADIM_KIMLIGI_KARAKTER: usize = 64;

/// Plan adımının ilerleme durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanAdimDurumu {
    #[serde(alias = "bekliyor")]
    Bekliyor,
    #[serde(alias = "suruyor", alias = "sürüyor")]
    Suruyor,
    #[serde(alias = "tamam", alias = "tamamlandi", alias = "tamamlandı")]
    Tamam,
}

impl PlanAdimDurumu {
    pub fn etiket(&self) -> &'static str {
        match self {
            Self::Bekliyor => "Bekliyor",
            Self::Suruyor => "Sürüyor",
            Self::Tamam => "Tamam",
        }
    }
}

/// Tek bir oturum planı adımı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanAdimi {
    #[serde(alias = "id")]
    pub kimlik: String,
    #[serde(alias = "metin", alias = "adim")]
    pub baslik: String,
    pub durum: PlanAdimDurumu,
}

/// Oturumun geçerli çalışma planı.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OturumPlani {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guncellendigi_tur: Option<TurId>,
    pub adimlar: Vec<PlanAdimi>,
}

/// Plan doğrulama hataları.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanHatasi {
    #[error("plan en az 1 adım içermelidir")]
    BosPlan,
    #[error("plan en çok {sinir} adım içerebilir (gelen: {gercek})")]
    AdimSayisiAsildi { sinir: usize, gercek: usize },
    #[error("plan adım kimliği boş olamaz")]
    BosAdimKimligi,
    #[error("plan adım kimliği en çok {sinir} karakter olabilir: '{kimlik}'")]
    UzunAdimKimligi { kimlik: String, sinir: usize },
    #[error("yinelenen plan adım kimliği: '{kimlik}'")]
    YinelenenAdimKimligi { kimlik: String },
    #[error("'{kimlik}' adımının başlığı boş olamaz")]
    BosAdimBasligi { kimlik: String },
    #[error("'{kimlik}' adımının başlığı en çok {sinir} karakter olabilir (gelen: {gercek})")]
    AdimBasligiCokUzun {
        kimlik: String,
        sinir: usize,
        gercek: usize,
    },
    #[error("planda aynı anda en çok 1 'Suruyor' adımı bulunabilir (gelen: {sayisi})")]
    BirdenFazlaEtkinAdim { sayisi: usize },
}

/// Ham adım listesini doğrular, kırpar ve geçerli bir `OturumPlani` üretir.
pub fn plan_adimlarini_dogrula(
    tur: Option<TurId>,
    ham_adimlar: Vec<PlanAdimi>,
) -> Result<OturumPlani, PlanHatasi> {
    if ham_adimlar.is_empty() {
        return Err(PlanHatasi::BosPlan);
    }
    if ham_adimlar.len() > AZAMI_PLAN_ADIMI {
        return Err(PlanHatasi::AdimSayisiAsildi {
            sinir: AZAMI_PLAN_ADIMI,
            gercek: ham_adimlar.len(),
        });
    }

    let mut gorulen_kimlikler = HashSet::with_capacity(ham_adimlar.len());
    let mut suruyor_sayisi = 0usize;
    let mut temiz_adimlar = Vec::with_capacity(ham_adimlar.len());

    for adim in ham_adimlar {
        let kimlik = adim.kimlik.trim().to_string();
        if kimlik.is_empty() {
            return Err(PlanHatasi::BosAdimKimligi);
        }
        if kimlik.chars().count() > AZAMI_ADIM_KIMLIGI_KARAKTER {
            return Err(PlanHatasi::UzunAdimKimligi {
                kimlik,
                sinir: AZAMI_ADIM_KIMLIGI_KARAKTER,
            });
        }
        if !gorulen_kimlikler.insert(kimlik.clone()) {
            return Err(PlanHatasi::YinelenenAdimKimligi { kimlik });
        }

        let baslik = adim.baslik.trim().to_string();
        if baslik.is_empty() {
            return Err(PlanHatasi::BosAdimBasligi { kimlik });
        }
        let karakter_sayisi = baslik.chars().count();
        if karakter_sayisi > AZAMI_ADIM_METNI_KARAKTER {
            return Err(PlanHatasi::AdimBasligiCokUzun {
                kimlik,
                sinir: AZAMI_ADIM_METNI_KARAKTER,
                gercek: karakter_sayisi,
            });
        }

        if adim.durum == PlanAdimDurumu::Suruyor {
            suruyor_sayisi += 1;
        }

        temiz_adimlar.push(PlanAdimi {
            kimlik,
            baslik,
            durum: adim.durum,
        });
    }

    if suruyor_sayisi > 1 {
        return Err(PlanHatasi::BirdenFazlaEtkinAdim {
            sayisi: suruyor_sayisi,
        });
    }

    Ok(OturumPlani {
        guncellendigi_tur: tur,
        adimlar: temiz_adimlar,
    })
}

#[derive(Debug, Deserialize)]
struct PlanGuncelleGirdisi {
    adimlar: Vec<PlanAdimi>,
}

/// Oturumun çalışma planını bütünüyle değiştiren çekirdek aracı (`plan_guncelle`).
/// Disk veya kabuk yetkisi vermez; çalışma alanı kilidi tutmaz.
#[derive(Debug, Default)]
pub struct PlanGuncelleAraci;

impl PlanGuncelleAraci {
    pub fn yeni() -> Self {
        Self
    }
}

#[async_trait]
impl Arac for PlanGuncelleAraci {
    fn ad(&self) -> &str {
        "plan_guncelle"
    }

    fn aciklama(&self) -> &str {
        "Oturumun çalışma planını bütünüyle günceller. En çok 32 benzersiz kimlikli adım, \
         adım başına en çok 200 karakter ve aynı anda en çok 1 'Suruyor' adımı içerebilir. \
         Disk veya kabuk yetkisi oluşturmaz."
    }

    fn parametre_semasi(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "adimlar": {
                    "type": "array",
                    "description": "Oturumun güncel plan adımları (1-32 adım).",
                    "items": {
                        "type": "object",
                        "properties": {
                            "kimlik": {
                                "type": "string",
                                "description": "Benzersiz adım kimliği (ör. 'adim-1')."
                            },
                            "baslik": {
                                "type": "string",
                                "description": "Kısa adım açıklaması (en çok 200 karakter)."
                            },
                            "durum": {
                                "type": "string",
                                "enum": ["Bekliyor", "Suruyor", "Tamam"],
                                "description": "Adım durumu; aynı anda en çok bir adım 'Suruyor' olabilir."
                            }
                        },
                        "required": ["kimlik", "baslik", "durum"]
                    }
                }
            },
            "required": ["adimlar"]
        })
    }

    fn yazar_mi(&self) -> bool {
        false
    }

    async fn hazirla(&self, argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
        let ayristirilan: PlanGuncelleGirdisi = serde_json::from_value(argumanlar.clone())
            .map_err(|e| AracHatasi::Gecersiz(format!("plan_guncelle girdisi geçersiz: {e}")))?;
        let dogrulanan = plan_adimlarini_dogrula(None, ayristirilan.adimlar)
            .map_err(|e| AracHatasi::Gecersiz(e.to_string()))?;

        Ok(Hazirlik {
            aciklama: format!("Plan güncellenecek ({} adım).", dogrulanan.adimlar.len()),
            kilitsiz: true,
            ..Hazirlik::default()
        })
    }

    async fn calistir(
        &self,
        _argumanlar: &Value,
        _hazirlik: &Hazirlik,
        _iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        Err(AracHatasi::Gecersiz(
            "plan_guncelle yalnız oturum ortamında çalıştırılabilir.".to_string(),
        ))
    }

    async fn calistir_ortamli(
        &self,
        argumanlar: &Value,
        _hazirlik: &Hazirlik,
        iptal: CancellationToken,
        ortam: AracOrtami,
    ) -> Result<AracCiktisi, AracHatasi> {
        if iptal.is_cancelled() {
            return Err(AracHatasi::Iptal);
        }
        let ayristirilan: PlanGuncelleGirdisi = serde_json::from_value(argumanlar.clone())
            .map_err(|e| AracHatasi::Gecersiz(format!("plan_guncelle girdisi geçersiz: {e}")))?;
        let dogrulanan = plan_adimlarini_dogrula(None, ayristirilan.adimlar)
            .map_err(|e| AracHatasi::Gecersiz(e.to_string()))?;

        let plan = ortam.plan_guncelle(dogrulanan.adimlar, iptal).await?;

        let tamam_sayisi = plan
            .adimlar
            .iter()
            .filter(|a| a.durum == PlanAdimDurumu::Tamam)
            .count();
        let etkin = plan
            .adimlar
            .iter()
            .find(|a| a.durum == PlanAdimDurumu::Suruyor)
            .map(|a| format!(" Etkin adım: {} ({}).", a.kimlik, a.baslik))
            .unwrap_or_default();

        Ok(AracCiktisi {
            icerik: format!(
                "Oturum planı güncellendi: toplam {} adım, {} tamamlandı.{}",
                plan.adimlar.len(),
                tamam_sayisi,
                etkin
            ),
            kaynak: "kalfa:plan".to_string(),
            kalfa_uretimi: true,
            ..AracCiktisi::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iki_suruyor_adimi_reddedilir() {
        let sonuc = plan_adimlarini_dogrula(
            None,
            vec![
                PlanAdimi {
                    kimlik: "a1".to_string(),
                    baslik: "İlk adım".to_string(),
                    durum: PlanAdimDurumu::Suruyor,
                },
                PlanAdimi {
                    kimlik: "a2".to_string(),
                    baslik: "İkinci adım".to_string(),
                    durum: PlanAdimDurumu::Suruyor,
                },
            ],
        );
        assert_eq!(sonuc, Err(PlanHatasi::BirdenFazlaEtkinAdim { sayisi: 2 }));
    }

    #[test]
    fn yinelenen_kimlik_ve_asiri_uzun_liste_reddedilir() {
        let yinelenen = plan_adimlarini_dogrula(
            None,
            vec![
                PlanAdimi {
                    kimlik: "ayni".to_string(),
                    baslik: "Bir".to_string(),
                    durum: PlanAdimDurumu::Bekliyor,
                },
                PlanAdimi {
                    kimlik: "ayni".to_string(),
                    baslik: "İki".to_string(),
                    durum: PlanAdimDurumu::Bekliyor,
                },
            ],
        );
        assert_eq!(
            yinelenen,
            Err(PlanHatasi::YinelenenAdimKimligi {
                kimlik: "ayni".to_string()
            })
        );

        let cok_fazla: Vec<PlanAdimi> = (0..33)
            .map(|i| PlanAdimi {
                kimlik: format!("adim-{i}"),
                baslik: "Adım".to_string(),
                durum: PlanAdimDurumu::Bekliyor,
            })
            .collect();
        assert_eq!(
            plan_adimlarini_dogrula(None, cok_fazla),
            Err(PlanHatasi::AdimSayisiAsildi {
                sinir: 32,
                gercek: 33
            })
        );
    }
}
