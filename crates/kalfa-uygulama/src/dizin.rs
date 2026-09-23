//! Kalfa'nın veri dizini (üst belge §6.6, 1c tasarımı §5.1): Windows'ta
//! `%APPDATA%\Kalfa`, Linux'ta `$XDG_DATA_HOME/kalfa`. Oturum kayıtları onun
//! altındaki `oturumlar` dizinindedir; dosya adı oturum kimliğidir.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use kalfa_core::kimlik::OturumId;

/// Testlerde ve uçtan uca denemelerde veri dizinini değiştirir.
pub const VERI_DIZINI_DEGISKENI: &str = "KALFA_VERI_DIZINI";

/// `platform_veri_dizini`: işletim sisteminin kullanıcı veri dizini (Tauri'de
/// `path().data_dir()`). `ortam`: `KALFA_VERI_DIZINI`'nin değeri.
pub fn veri_dizini(platform_veri_dizini: &Path, ortam: Option<OsString>) -> PathBuf {
    if let Some(dizin) = ortam.filter(|d| !d.is_empty()) {
        return PathBuf::from(dizin);
    }
    platform_veri_dizini.join(if cfg!(windows) { "Kalfa" } else { "kalfa" })
}

pub fn oturum_dizini(veri_dizini: &Path) -> PathBuf {
    veri_dizini.join("oturumlar")
}

pub fn kayit_yolu(oturum_dizini: &Path, oturum: OturumId) -> PathBuf {
    oturum_dizini.join(format!("{oturum}.jsonl"))
}
