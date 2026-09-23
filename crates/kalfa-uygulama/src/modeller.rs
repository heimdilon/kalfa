//! Model seçici (üst belge §6.4). Yalnızca bu kimlikler kabul edilir.

use crate::sozlesme::{ModelProfili, ModelSecenegi};

pub const VARSAYILAN_MODEL: &str = "glm-5.3";

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
        })
        .collect()
}

pub fn bilinen_model_mi(model: &str) -> bool {
    MODELLER.iter().any(|(kimlik, _)| *kimlik == model)
}
