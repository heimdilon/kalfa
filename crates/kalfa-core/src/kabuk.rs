//! Kabuk çağrısının çekirdekteki tarifi (aşama 2 tasarımı §3.3). İzin kararı,
//! onay kartı ve mühür bundan üretilir; komutu çalıştıran kod
//! `kalfa-sandbox`'tadır.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KabukTarifi {
    /// Onay kartında gösterilen tam komut: program ve argümanları ya da
    /// betiğin bütün metni.
    pub komut: String,
    pub bicim: KabukBicimi,
    /// Çalışma alanına göre çalışma dizini; kök için `.`.
    pub dizin: String,
    /// Model ağ erişimi istedi. Yalıtımsız çalıştırmada ağ kapatılamaz; bu
    /// yalnızca modelin niyetidir.
    pub ag: bool,
    pub yalitim: YalitimOzeti,
    /// Bu çalışma alanındaki ilk kabuk çağrısı (aşama 2 tasarımı §4.2).
    pub ilk_cagri: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum KabukBicimi {
    Program,
    Betik { yorumlayici: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum YalitimOzeti {
    /// `profil`: yalıtım profilinin özeti; profil değişirse bekleyen onaylar
    /// geçersizleşir.
    Yalitimli { arka_uc: String, profil: String },
    /// `talimat`: yalıtımın neden olmadığı ve nasıl açılacağı.
    Yalitimsiz { talimat: String },
}

impl YalitimOzeti {
    pub fn yalitimli_mi(&self) -> bool {
        matches!(self, Self::Yalitimli { .. })
    }
}
