//! Yönetici kurulumunun kaydı (aşama 2 tasarımı §9.1, §9.2):
//! `%ProgramData%\Kalfa\yalitim\kurulum.json`. `kalfa-yalitim` yazar;
//! uygulama ve kaldırma okur. Dizini yalnızca Administrators ve SYSTEM
//! yazabilir; oturum açmış kullanıcılar okuyabilir.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::guvenlik::{
    self, Izin, MASKE_OKU, MASKE_TAM, MIRAS, SISTEM, Sid, YONETICILER, mirasi_kes,
};

/// Oturum açmış kullanıcılar (Authenticated Users).
const OTURUM_ACMIS: &str = "S-1-5-11";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KurulumKaydi {
    /// Araç zinciri yeteneğine listeleme izni verilmiş sürücü kökleri.
    #[serde(default)]
    pub kok_izinleri: Vec<PathBuf>,
    /// `izin-ver` ile araç zinciri yeteneğine okuma izni verilmiş dizinler.
    #[serde(default)]
    pub izinli_dizinler: Vec<PathBuf>,
}

/// Kaydın dizini: `%ProgramData%\Kalfa\yalitim`.
pub fn kayit_dizini() -> io::Result<PathBuf> {
    Ok(guvenlik::program_data()?.join("Kalfa").join("yalitim"))
}

impl KurulumKaydi {
    /// Kaydı okur; yoksa `None`. Okunamayan ya da çözülemeyen kayıt hatadır:
    /// sessizce yok sayılsaydı `kaldir` kaydedilen izinleri yerinde bırakır,
    /// `kur` da `izin-ver` kayıtlarını ezerdi.
    pub fn oku(dizin: &Path) -> io::Result<Option<Self>> {
        let yol = dizin.join("kurulum.json");
        let metin = match std::fs::read(&yol) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        serde_json::from_slice(&metin).map(Some).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("kurulum kaydı bozuk ({}): {e}", yol.display()),
            )
        })
    }

    /// Kaydı yazar; dizinin izinleri her yazışta yeniden konur. Yönetici
    /// gerekir.
    pub fn yaz(&self, dizin: &Path) -> io::Result<()> {
        std::fs::create_dir_all(dizin)?;
        let izin = |sid: &str, maske| -> io::Result<Izin> {
            Ok(Izin {
                sid: Sid::metinden(sid)?,
                maske,
                miras: MIRAS,
            })
        };
        mirasi_kes(
            dizin,
            &[
                izin(YONETICILER, MASKE_TAM)?,
                izin(SISTEM, MASKE_TAM)?,
                izin(OTURUM_ACMIS, MASKE_OKU)?,
            ],
        )?;
        let metin = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let gecici = dizin.join("kurulum.json.yeni");
        std::fs::write(&gecici, metin)?;
        std::fs::rename(&gecici, dizin.join("kurulum.json"))
    }

    /// Kaydı siler; dizin boşaldıysa onu da.
    pub fn sil(dizin: &Path) -> io::Result<()> {
        match std::fs::remove_file(dizin.join("kurulum.json")) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        let _ = std::fs::remove_dir(dizin);
        if let Some(ust) = dizin.parent() {
            let _ = std::fs::remove_dir(ust);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kayit_okunur_ve_bozuk_kayit_hata_verir() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(KurulumKaydi::oku(d.path()).unwrap(), None);
        std::fs::write(
            d.path().join("kurulum.json"),
            r#"{"kok_izinleri":["C:\\"],"izinli_dizinler":["C:\\Python314"]}"#,
        )
        .unwrap();
        let k = KurulumKaydi::oku(d.path()).unwrap().unwrap();
        assert_eq!(k.kok_izinleri, vec![PathBuf::from(r"C:\")]);
        assert_eq!(k.izinli_dizinler, vec![PathBuf::from(r"C:\Python314")]);
        std::fs::write(d.path().join("kurulum.json"), "{bozuk").unwrap();
        let hata = KurulumKaydi::oku(d.path()).unwrap_err();
        assert_eq!(hata.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn kayit_dizini_program_data_altinda() {
        let d = kayit_dizini().unwrap();
        assert!(d.ends_with(r"Kalfa\yalitim"), "{}", d.display());
    }
}
