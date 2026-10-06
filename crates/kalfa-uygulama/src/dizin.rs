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

/// Yalıtımın çalışma alanına özel dizinlerinin kökü (aşama 2 tasarımı §7.3).
/// Windows'ta `%LOCALAPPDATA%\Kalfa\yalitim` (`yerel_veri`: Tauri'de
/// `local_data_dir()`): önbellekler dolaşan profile girmez. Linux'ta ve
/// `KALFA_VERI_DIZINI` verildiğinde veri dizinindeki `yalitim`.
pub fn yalitim_dizini(veri: &Path, yerel_veri: &Path, ortam: Option<OsString>) -> PathBuf {
    if cfg!(windows) && ortam.filter(|d| !d.is_empty()).is_none() {
        return yerel_veri.join("Kalfa").join("yalitim");
    }
    veri.join("yalitim")
}

/// Geri alma verisinin (önceki hâllerin deposu) kökü (aşama 3 tasarımı §5.2);
/// kural `yalitim_dizini`'ninkidir. Windows'ta `%LOCALAPPDATA%\Kalfa\geri-alma`:
/// büyük veri dolaşan profile girmez. Linux'ta ve `KALFA_VERI_DIZINI`
/// verildiğinde veri dizinindeki `geri-alma`.
pub fn geri_alma_dizini(veri: &Path, yerel_veri: &Path, ortam: Option<OsString>) -> PathBuf {
    if cfg!(windows) && ortam.filter(|d| !d.is_empty()).is_none() {
        return yerel_veri.join("Kalfa").join("geri-alma");
    }
    veri.join("geri-alma")
}

/// Bir oturumun geri alma deposu.
pub fn oturum_geri_alma_dizini(kok: &Path, oturum: OturumId) -> PathBuf {
    kok.join(oturum.to_string())
}

/// Uçtan uca test derlemesinde Kalfa'nın araç zinciri kökünü (`araclar\`)
/// verir. Üretim derlemesi bu değişkeni okumaz.
pub const ARAC_ZINCIRI_DEGISKENI: &str = "KALFA_ARAC_ZINCIRI";

/// Ağ köprüsü ikilisinin ortam değişkeni.
pub const KOPRU_YOLU_DEGISKENI: &str = "KALFA_KOPRU_YOLU";

/// Ağ köprüsü ikilisinin yolu (`kalfa-kopru.exe`).
/// Windows'ta kurulu sürümde `exe`'nin yanındaki dizinde, geliştirme
/// derlemesinde (`target\debug\...`) aynı `target` dizininde aranır.
/// `KALFA_KOPRU_YOLU` ortam değişkeni varsa ve geçerliyse önceliklidir.
pub fn kopru_ikilisi(exe: &Path, ortam: Option<OsString>) -> Option<PathBuf> {
    if let Some(yol) = ortam
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_file())
    {
        return Some(yol);
    }
    let ad = if cfg!(windows) {
        "kalfa-kopru.exe"
    } else {
        "kalfa-kopru"
    };
    if let Some(parent) = exe.parent() {
        let dogrudan = parent.join(ad);
        if dogrudan.is_file() {
            return Some(dogrudan);
        }
        if let Some(ust) = parent.parent() {
            let ustte = ust.join(ad);
            if ustte.is_file() {
                return Some(ustte);
            }
        }
    }
    None
}

/// Kurulu Kalfa'nın getirdiği Python ve Node dizinleri (2d-3): yalnızca
/// Windows'ta ve `exe` Program Files'taki kurulumdaysa. `ortam`:
/// `KALFA_ARAC_ZINCIRI`'nin değeri; yalnızca `e2e` özellikli derlemede
/// kullanılır.
pub fn kalfa_araclari(exe: &Path, ortam: Option<OsString>) -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        #[cfg(feature = "e2e")]
        if let Some(kok) = ortam.filter(|d| !d.is_empty()) {
            return kalfa_sandbox::windows::sistem::arac_zinciri_dizinleri(Path::new(&kok));
        }
        #[cfg(not(feature = "e2e"))]
        let _ = ortam;
        kalfa_sandbox::windows::sistem::kurulu_arac_zinciri(exe)
    }
    #[cfg(not(windows))]
    {
        let _ = (exe, ortam);
        Vec::new()
    }
}

pub fn oturum_dizini(veri_dizini: &Path) -> PathBuf {
    veri_dizini.join("oturumlar")
}

pub fn kayit_yolu(oturum_dizini: &Path, oturum: OturumId) -> PathBuf {
    oturum_dizini.join(format!("{oturum}.jsonl"))
}
