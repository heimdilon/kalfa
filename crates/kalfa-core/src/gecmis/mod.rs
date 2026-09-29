//! Kayıttan arayüz görünümü (1c tasarımı §5.5, §8.3, §9.3).
//!
//! Kayıttan açılan oturumun ekrandaki geçmişi kayıt öğelerinden üretilir. Her
//! öğe kayıttaki `sira`'yı taşır; canlı olaylar aynı sayaçtan devam ettiği için
//! arayüz ikisini tek sırada birleştirir.

mod liste;

pub use liste::{OturumDurumu, OturumOzeti, oturum_ozeti, oturumlari_listele};

use std::collections::HashMap;

use kalfa_evren::KrediGozlemi;
use kalfa_evren::tipler::Role;
use serde::{Deserialize, Serialize};

use crate::geri_alma::GeriAlmaSonucu;
use crate::izin::Kip;
use crate::kayit::KayitOgesi;
use crate::kimlik::{CagriId, TurId};
use crate::protokol::{AracDurumu, BitisNedeni, Zarf};
use crate::tur_ozeti::{TurOzeti, veri_silindi_isle};

/// Ekranda gösterilen bir geçmiş öğesi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum GorunumOgesi {
    KullaniciMesaji {
        sira: u64,
        tur: Option<TurId>,
        metin: String,
    },
    Yanit {
        sira: u64,
        tur: Option<TurId>,
        metin: Option<String>,
        dusunce: Option<String>,
    },
    AracCagrisi {
        sira: u64,
        tur: Option<TurId>,
        /// Modelin verdiği çağrı kimliği.
        model_cagri_id: String,
        arac: String,
        /// Modelin gönderdiği ham argüman (JSON metni).
        argumanlar: String,
        /// `None`: sonuç kaydı yok, çağrı hiç başlamadı.
        durum: Option<AracDurumu>,
        sonuc: Option<String>,
    },
    TurSonu {
        sira: u64,
        tur: Option<TurId>,
        neden: BitisNedeni,
    },
    GuvenlikUyarisi {
        sira: u64,
        tur: Option<TurId>,
        mesaj: String,
    },
    /// Bir turun geri alınması (aşama 3 tasarımı §5.6). `tur` kaydın kendi
    /// turudur (geri alma tur dışında yapılır); geri alınan tur
    /// `geri_alinan_tur`'dadır. `yarim`: `GeriAlmaBasladi` var, `GeriAlindi`
    /// yok.
    GeriAlma {
        sira: u64,
        tur: Option<TurId>,
        geri_alinan_tur: TurId,
        sonuc: Vec<GeriAlmaSonucu>,
        yarim: bool,
    },
    /// Geri almanın modele bildirimi; kullanıcı balonu değildir (§5.9).
    KalfaNotu {
        sira: u64,
        tur: Option<TurId>,
        metin: String,
    },
    /// Turun özeti (§5.5); turun son öğesidir.
    TurOzeti {
        sira: u64,
        tur: Option<TurId>,
        ozet: TurOzeti,
    },
}

impl GorunumOgesi {
    pub fn sira(&self) -> u64 {
        match self {
            Self::KullaniciMesaji { sira, .. }
            | Self::Yanit { sira, .. }
            | Self::AracCagrisi { sira, .. }
            | Self::TurSonu { sira, .. }
            | Self::GuvenlikUyarisi { sira, .. }
            | Self::GeriAlma { sira, .. }
            | Self::KalfaNotu { sira, .. }
            | Self::TurOzeti { sira, .. } => *sira,
        }
    }

    pub fn tur(&self) -> Option<TurId> {
        match self {
            Self::KullaniciMesaji { tur, .. }
            | Self::Yanit { tur, .. }
            | Self::AracCagrisi { tur, .. }
            | Self::TurSonu { tur, .. }
            | Self::GuvenlikUyarisi { tur, .. }
            | Self::GeriAlma { tur, .. }
            | Self::KalfaNotu { tur, .. }
            | Self::TurOzeti { tur, .. } => *tur,
        }
    }
}

/// Kayıttan ekran öğelerini üretir.
pub fn gorunum_uret(kayitlar: &[Zarf<KayitOgesi>]) -> Vec<GorunumOgesi> {
    let mut ogeler: Vec<GorunumOgesi> = Vec::new();
    // Modelin çağrı kimliği → öğenin yeri.
    let mut model_yeri: HashMap<String, usize> = HashMap::new();
    // Kalfa'nın çağrı kimliği → öğenin yeri (`AracBasladi` ile öğrenilir).
    let mut kalfa_yeri: HashMap<CagriId, usize> = HashMap::new();
    // Son `tool` mesajının öğesi: başlamamış çağrının `AracBitti`'si ona aittir.
    let mut son_sonuc: Option<usize> = None;
    for z in kayitlar {
        match &z.yuk {
            KayitOgesi::Mesaj { mesaj } => match mesaj.role {
                Role::User => ogeler.push(GorunumOgesi::KullaniciMesaji {
                    sira: z.sira,
                    tur: z.tur,
                    metin: mesaj.content.clone().unwrap_or_default(),
                }),
                Role::Assistant => {
                    if mesaj.content.is_some() || mesaj.reasoning.is_some() {
                        ogeler.push(GorunumOgesi::Yanit {
                            sira: z.sira,
                            tur: z.tur,
                            metin: mesaj.content.clone(),
                            dusunce: mesaj.reasoning.clone(),
                        });
                    }
                    for tc in mesaj.tool_calls.iter().flatten() {
                        model_yeri.insert(tc.id.clone(), ogeler.len());
                        ogeler.push(GorunumOgesi::AracCagrisi {
                            sira: z.sira,
                            tur: z.tur,
                            model_cagri_id: tc.id.clone(),
                            arac: tc.function.name.clone(),
                            argumanlar: tc.function.arguments.clone(),
                            durum: None,
                            sonuc: None,
                        });
                    }
                }
                Role::Tool => {
                    son_sonuc = mesaj
                        .tool_call_id
                        .as_ref()
                        .and_then(|id| model_yeri.get(id).copied());
                    if let Some(GorunumOgesi::AracCagrisi { sonuc, .. }) =
                        son_sonuc.and_then(|i| ogeler.get_mut(i))
                    {
                        *sonuc = mesaj.content.clone();
                    }
                }
                Role::System => {}
            },
            KayitOgesi::AracBasladi {
                cagri,
                model_cagri_id,
                ..
            } => {
                if let Some(&i) = model_yeri.get(model_cagri_id) {
                    kalfa_yeri.insert(*cagri, i);
                    if let Some(GorunumOgesi::AracCagrisi { durum, .. }) = ogeler.get_mut(i) {
                        // "Bitti" kaydı gelene kadar sonuç bilinmiyor.
                        *durum = Some(AracDurumu::SonucuBilinmiyor);
                    }
                }
            }
            KayitOgesi::AracBitti {
                cagri,
                durum: bitis,
            } => {
                let yer = kalfa_yeri.get(cagri).copied().or(son_sonuc);
                if let Some(GorunumOgesi::AracCagrisi { durum, .. }) =
                    yer.and_then(|i| ogeler.get_mut(i))
                {
                    *durum = Some(*bitis);
                }
                son_sonuc = None;
            }
            KayitOgesi::GuvenlikUyarisi { mesaj, .. } => {
                ogeler.push(GorunumOgesi::GuvenlikUyarisi {
                    sira: z.sira,
                    tur: z.tur,
                    mesaj: mesaj.clone(),
                })
            }
            KayitOgesi::TurBitti { neden } => ogeler.push(GorunumOgesi::TurSonu {
                sira: z.sira,
                tur: z.tur,
                neden: neden.clone(),
            }),
            KayitOgesi::GeriAlmaBasladi {
                geri_alinan_tur, ..
            } => ogeler.push(GorunumOgesi::GeriAlma {
                sira: z.sira,
                tur: z.tur,
                geri_alinan_tur: *geri_alinan_tur,
                sonuc: Vec::new(),
                yarim: true,
            }),
            KayitOgesi::GeriAlindi {
                geri_alinan_tur,
                sonuc: bitis,
            } => {
                let yarim_kalan = ogeler.iter_mut().rev().find_map(|o| match o {
                    GorunumOgesi::GeriAlma {
                        geri_alinan_tur: t,
                        sonuc,
                        yarim,
                        ..
                    } if t == geri_alinan_tur && *yarim => Some((sonuc, yarim)),
                    _ => None,
                });
                match yarim_kalan {
                    Some((sonuc, yarim)) => {
                        *sonuc = bitis.clone();
                        *yarim = false;
                    }
                    None => ogeler.push(GorunumOgesi::GeriAlma {
                        sira: z.sira,
                        tur: z.tur,
                        geri_alinan_tur: *geri_alinan_tur,
                        sonuc: bitis.clone(),
                        yarim: false,
                    }),
                }
            }
            KayitOgesi::KalfaNotu { metin } => ogeler.push(GorunumOgesi::KalfaNotu {
                sira: z.sira,
                tur: z.tur,
                metin: metin.clone(),
            }),
            KayitOgesi::TurOzeti { ozet } => ogeler.push(GorunumOgesi::TurOzeti {
                sira: z.sira,
                tur: z.tur,
                ozet: ozet.clone(),
            }),
            KayitOgesi::GeriAlmaVerisiSilindi { turlar } => {
                for oge in &mut ogeler {
                    if let GorunumOgesi::TurOzeti { ozet, .. } = oge
                        && turlar.contains(&ozet.tur)
                    {
                        veri_silindi_isle(&mut ozet.dosyalar);
                    }
                }
            }
            _ => {}
        }
    }
    ogeler
}

/// Geçmişin bir sayfası.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GecmisSayfasi {
    pub ogeler: Vec<GorunumOgesi>,
    /// Bu sayfadan daha eski öğe var mı.
    pub daha_eski_var: bool,
}

/// `once_sira`'dan (verilmezse sondan) geriye doğru en fazla `tur_sayisi`
/// turluk öğe. Tur, aynı `tur` kimliğini taşıyan ardışık öğelerdir.
pub fn sayfa(ogeler: &[GorunumOgesi], once_sira: Option<u64>, tur_sayisi: usize) -> GecmisSayfasi {
    let son = match once_sira {
        Some(s) => ogeler.partition_point(|o| o.sira() < s),
        None => ogeler.len(),
    };
    let mut bas = son;
    let mut sayilan = 0;
    while bas > 0 && sayilan < tur_sayisi {
        let tur = ogeler[bas - 1].tur();
        while bas > 0 && ogeler[bas - 1].tur() == tur {
            bas -= 1;
        }
        sayilan += 1;
    }
    GecmisSayfasi {
        ogeler: ogeler[bas..son].to_vec(),
        daha_eski_var: bas > 0,
    }
}

/// Durum çubuğunun kayıttan başlangıç değerleri (1c tasarımı §8.3).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DurumOzeti {
    pub model: Option<String>,
    pub kip: Option<Kip>,
    pub yonlendirilen_model: Option<String>,
    pub son_kredi: Option<KrediGozlemi>,
    pub istem: u64,
    pub tamamlama: u64,
    pub dusunce: u64,
}

pub fn durum_ozeti(kayitlar: &[Zarf<KayitOgesi>]) -> DurumOzeti {
    let mut o = DurumOzeti::default();
    for z in kayitlar {
        match &z.yuk {
            KayitOgesi::OturumAcildi { model, kip, .. } => {
                o.model = Some(model.clone());
                o.kip = Some(*kip);
            }
            KayitOgesi::ModelDegisti { model } => o.model = Some(model.clone()),
            KayitOgesi::KipDegisti { kip } => o.kip = Some(*kip),
            KayitOgesi::Istek {
                kredi,
                yonlendirilen_model,
                ..
            } => {
                if yonlendirilen_model.is_some() {
                    o.yonlendirilen_model = yonlendirilen_model.clone();
                }
                if kredi.is_some() {
                    o.son_kredi = kredi.clone();
                }
            }
            KayitOgesi::Kullanim {
                istem,
                tamamlama,
                dusunce,
            } => {
                o.istem = o.istem.saturating_add(*istem);
                o.tamamlama = o.tamamlama.saturating_add(*tamamlama);
                o.dusunce = o.dusunce.saturating_add(*dusunce);
            }
            _ => {}
        }
    }
    o
}
