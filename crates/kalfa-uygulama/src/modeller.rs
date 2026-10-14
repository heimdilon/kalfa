//! Model seçici (üst belge §6.4). EVREN `/v1/models` kataloğundaki `chat`
//! modellerini ve efor seçeneklerini dinamik olarak çözer; çevrimdışı açılışta
//! varsayılan katalog kullanılır.

use kalfa_evren::tipler::{DusunceEforu, ModelInfo};

use crate::sozlesme::{ModelProfili, ModelSecenegi};

pub const VARSAYILAN_MODEL: &str = "glm-5.3";
pub const VARSAYILAN_EFOR: DusunceEforu = DusunceEforu::Orta;

const MODELLER: [(&str, ModelProfili); 4] = [
    ("qwen3.8-flash-next", ModelProfili::Hizli),
    ("deepseek-v4-flash", ModelProfili::Dengeli),
    ("glm-5.3", ModelProfili::Derin),
    ("auto", ModelProfili::Otomatik),
];

pub fn modeller() -> Vec<ModelSecenegi> {
    MODELLER
        .iter()
        .map(|(kimlik, profil)| ModelSecenegi {
            kimlik: (*kimlik).to_string(),
            profil: *profil,
            eforlar: DusunceEforu::tumu(),
        })
        .collect()
}

pub fn katalogdan_modeller(katalog: &[ModelInfo]) -> Vec<ModelSecenegi> {
    let mut sonuc: Vec<ModelSecenegi> = katalog
        .iter()
        .filter(|m| m.task == "chat" && !m.id.trim().is_empty())
        .map(|m| ModelSecenegi {
            kimlik: m.id.clone(),
            profil: profil_tahmin_et(&m.id),
            eforlar: if m.eforlar.is_empty() {
                DusunceEforu::tumu()
            } else {
                m.eforlar.clone()
            },
        })
        .collect();
    if sonuc.is_empty() {
        return modeller();
    }
    if !sonuc.iter().any(|m| m.kimlik == "auto") {
        sonuc.push(ModelSecenegi {
            kimlik: "auto".to_string(),
            profil: ModelProfili::Otomatik,
            eforlar: DusunceEforu::tumu(),
        });
    }
    sonuc
}

fn profil_tahmin_et(kimlik: &str) -> ModelProfili {
    if let Some((_, p)) = MODELLER.iter().find(|(k, _)| *k == kimlik) {
        return *p;
    }
    let kucuk = kimlik.to_ascii_lowercase();
    if kucuk == "auto" {
        ModelProfili::Otomatik
    } else if kucuk.contains("flash") || kucuk.contains("mini") || kucuk.contains("fast") {
        ModelProfili::Hizli
    } else if kucuk.contains("glm") || kucuk.contains("pro") || kucuk.contains("reasoner") {
        ModelProfili::Derin
    } else {
        ModelProfili::Dengeli
    }
}

pub fn bilinen_model_mi(model: &str) -> bool {
    MODELLER.iter().any(|(kimlik, _)| *kimlik == model)
}

pub fn varsayilan_model(liste: &[ModelSecenegi]) -> String {
    if liste.iter().any(|m| m.kimlik == VARSAYILAN_MODEL) {
        VARSAYILAN_MODEL.to_string()
    } else {
        liste
            .first()
            .map(|m| m.kimlik.clone())
            .unwrap_or_else(|| VARSAYILAN_MODEL.to_string())
    }
}

pub fn model_gecerli_mi(liste: &[ModelSecenegi], model: &str) -> bool {
    if liste.is_empty() {
        bilinen_model_mi(model)
    } else {
        liste.iter().any(|m| m.kimlik == model)
    }
}

pub fn uygun_efor(liste: &[ModelSecenegi], model: &str, mevcut: DusunceEforu) -> DusunceEforu {
    let Some(secenek) = liste.iter().find(|m| m.kimlik == model) else {
        return mevcut;
    };
    if secenek.eforlar.contains(&mevcut) {
        mevcut
    } else if secenek.eforlar.contains(&VARSAYILAN_EFOR) {
        VARSAYILAN_EFOR
    } else {
        secenek.eforlar.first().copied().unwrap_or(VARSAYILAN_EFOR)
    }
}
