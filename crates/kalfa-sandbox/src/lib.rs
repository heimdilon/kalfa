//! Kabuk komutlarının çalıştırılması (aşama 2 tasarımı §3, §6).
//!
//! `Yalitici` bir komutu başlatır; `Surec` çıktısını okur, süre ve iptali
//! uygular, bitince bütün süreç ağacını sonlandırır. Süreç sınırları
//! yalıtımdan bağımsızdır: yalıtımsız arka uç da onları uygular. Crate
//! `kalfa-core`'a bağlı değildir.

mod agac;
pub mod ortam;
pub mod program;
mod surec;
mod yalitimsiz;

use std::path::PathBuf;

use serde::Serialize;

pub use surec::{Akis, Bitis, CalismaSonucu, Surec};
pub use yalitimsiz::Yalitimsiz;

/// Süreç ağacının en çok süreç sayısı (aşama 2 tasarımı §6.3).
pub const AZAMI_SUREC: u32 = 256;

/// Yalıtımın kullanılabilirliği (aşama 2 tasarımı §3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum YalitimDurumu {
    Hazir {
        arka_uc: String,
        surum: String,
    },
    Yok {
        neden: YalitimYoklukNedeni,
        talimat: String,
    },
}

impl YalitimDurumu {
    pub fn hazir_mi(&self) -> bool {
        matches!(self, Self::Hazir { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum YalitimYoklukNedeni {
    BwrapYok,
    KullaniciAdAlaniKapali,
    KurulumGerekli,
    KurulumBozuk,
    Desteklenmiyor,
}

/// Çalıştırılacak komutun biçimi (aşama 2 tasarımı §4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KomutBicimi {
    /// Kabuk yorumlayıcısı olmadan: `program` çıplak bir ad ya da mutlak bir
    /// yoldur. Çıplak ad komutun ortamındaki `PATH`'te aranır.
    Program {
        program: String,
        argumanlar: Vec<String>,
    },
    /// Platformun kabuğunda: Linux'ta `/bin/sh -c`, Windows'ta PowerShell.
    Betik { metin: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalistirmaIstegi {
    pub komut: KomutBicimi,
    /// Mutlak çalışma dizini.
    pub dizin: PathBuf,
    /// Komutun ortamı; `ortam::uret` ile üretilir.
    pub ortam: Vec<(String, String)>,
    /// Komut ağa erişmek istiyor. Yalıtımsız arka uç ağı kapatamaz.
    pub ag: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum BaslatmaHatasi {
    #[error("`{program}` bulunamadı. Komutun PATH'i: {path}")]
    ProgramYok { program: String, path: String },
    #[error("Komut başlatılamadı: {0}")]
    GirisCikis(#[from] std::io::Error),
}

/// Bir komutu başlatan arka uç (aşama 2 tasarımı §3.2).
pub trait Yalitici: Send + Sync {
    fn durum(&self) -> YalitimDurumu;
    fn baslat(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi>;
}
