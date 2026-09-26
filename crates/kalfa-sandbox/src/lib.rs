//! Kabuk komutlarının çalıştırılması (aşama 2 tasarımı §3, §6).
//!
//! `Yalitici` bir komutu başlatır; `Surec` çıktısını okur, süre ve iptali
//! uygular, bitince bütün süreç ağacını sonlandırır. Süreç sınırları
//! yalıtımdan bağımsızdır: yalıtımsız arka uç da onları uygular. Crate
//! `kalfa-core`'a bağlı değildir.

mod agac;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod ortam;
pub mod profil;
pub mod program;
mod surec;
pub mod tarama;
#[cfg(windows)]
pub mod windows;
mod yalitimsiz;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;

pub use profil::{AlanBilgisi, GitKimligi, Korumali, OzelDizinler, alan_kimligi};
pub use surec::{Akis, Bitis, CalismaSonucu, Surec};
pub use tarama::Tarama;
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
    /// Yalıtımlı çalıştırmanın çalışma alanı bilgisi. Verilmişse komut
    /// yalnızca yalıtımda çalışır, verilmemişse yalnızca yalıtımsız: istek
    /// böylece onaylandığı kipte çalışır.
    pub alan: Option<AlanBilgisi>,
}

#[derive(Debug, thiserror::Error)]
pub enum BaslatmaHatasi {
    #[error("`{program}` bulunamadı. Komutun PATH'i: {path}")]
    ProgramYok { program: String, path: String },
    #[error("Komut başlatılamadı: {0}")]
    GirisCikis(#[from] std::io::Error),
    #[error("Yalıtım başlatılamadı: {0}")]
    Yalitim(String),
}

/// Bir komutu başlatan arka uç (aşama 2 tasarımı §3.2).
pub trait Yalitici: Send + Sync {
    fn durum(&self) -> YalitimDurumu;

    /// Yalıtım profilinin özeti (§7.1); mühre girer. Yalıtım yoksa `None`.
    fn profil_ozeti(&self, _ag: bool) -> Option<String> {
        None
    }

    fn baslat(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi>;

    /// Algılamayı yeniden çalıştırır (§8.3): Ayarlar'dan istenince ve
    /// yalıtım başlatılamayınca.
    fn yeniden_algila(&self) -> YalitimDurumu {
        self.durum()
    }

    /// Kullanıcının eklediği araç zinciri dizinleri (§7.1).
    fn okunur_dizinleri_ayarla(&self, _dizinler: Vec<PathBuf>) {}

    /// Çalışma alanı yalıtım için hazırlanmalı mı (§9.4)? Hazırlık çalışma
    /// alanının izinlerini değiştirir; onayı her kipte yerel pencerede
    /// sorulur. Linux'ta hazırlık yoktur.
    fn hazirlik_gerekli(&self, _kok: &Path) -> bool {
        false
    }

    /// Yalıtımlı her çalıştırmadan hemen önce çağrılır (§9.4).
    ///
    /// - Hazırlanmamış çalışma alanı yalnızca `hazirlik_onaylandi` ise
    ///   hazırlanır: onaylanan tarif hazırlığı içeriyordu. Değilse hata döner;
    ///   onaydan sonra hazırlık geri alındıysa komut başlamaz.
    /// - Hazırlanmış çalışma alanında yeni gizli dosyalar ve korumalı yollar
    ///   işlenir.
    /// - `ilerleme` araç kartına giden satırlardır.
    fn alani_hazirla(
        &self,
        _alan: &AlanBilgisi,
        _hazirlik_onaylandi: bool,
        _ilerleme: &mut dyn FnMut(&str),
    ) -> Result<(), BaslatmaHatasi> {
        Ok(())
    }
}

/// Platformun yalıtıcısının ayarları.
pub struct PlatformAyari {
    /// Kullanıcının ev dizini; araç zinciri dizinleri buna göre bulunur.
    pub ev: PathBuf,
    /// Çalışma alanına özel dizinlerin kökü: `<veri dizini>/yalitim`.
    pub yalitim_dizini: PathBuf,
    /// Kullanıcının eklediği araç zinciri dizinleri.
    pub ek_okunur: Vec<PathBuf>,
    /// Durum sonradan değişince çağrılır.
    pub degisince: Box<dyn Fn(&YalitimDurumu) + Send + Sync>,
}

/// Platformun yalıtıcısı. Linux'ta bwrap, Windows'ta AppContainer kullanılır;
/// desteklenmiyorsa durum bunu söyler.
pub fn platform_yaliticisi(ayar: PlatformAyari) -> Arc<dyn Yalitici> {
    #[cfg(target_os = "linux")]
    {
        Arc::new(linux::LinuxYalitici::yeni(
            linux::LinuxAyari {
                ev: ayar.ev,
                yalitim_dizini: ayar.yalitim_dizini,
                ek_okunur: ayar.ek_okunur,
                git: GitKimligi::oku(),
            },
            Box::new(linux::BwrapAlgilayici),
            ayar.degisince,
        ))
    }
    #[cfg(windows)]
    {
        Arc::new(windows::yalitici::WindowsYalitici::yeni(
            windows::yalitici::WindowsAyari {
                ev: ayar.ev,
                yalitim_dizini: ayar.yalitim_dizini,
                ek_okunur: ayar.ek_okunur,
                git: GitKimligi::oku(),
            },
            ayar.degisince,
        ))
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = ayar;
        Arc::new(Yalitimsiz)
    }
}
