//! Yönetici kurulumunun kaydı (aşama 2 tasarımı §9.1, §9.2):
//! `%ProgramData%\Kalfa\yalitim\kurulum.json`. `kalfa-yalitim` yazar;
//! uygulama ve kaldırma okur. Dizini yalnızca Administrators ve SYSTEM
//! yazabilir; oturum açmış kullanıcılar okuyabilir.
//!
//! `%ProgramData%` altında standart kullanıcı klasör oluşturabilir ve
//! oluşturduğunun sahibi olur (güvenlik analizi Bulgu 6). Yönetici yardımcısı
//! her işten önce `kayit_dizini_hazirla` ile `Kalfa` ve `Kalfa\yalitim`'i
//! reparse noktası izlemeden açar, sahibinin yönetici olduğunu denetler ve
//! DACL'i tutamaçla yazar; aksi hâlde durur.

use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;

use super::guvenlik::{
    self, Izin, MASKE_OKU, MASKE_TAM, MIRAS, SISTEM, Sid, YONETICILER, dacl_yaz_tutamacla,
    korumali_dizin_olustur, sahip, yeniden_ayristirma_izlemeden_ac,
};

/// Oturum açmış kullanıcılar (Authenticated Users).
const OTURUM_ACMIS: &str = "S-1-5-11";

pub const KAYIT_SAHIBI_HATASI: &str = "Kayıt dizininin sahibi yönetici değil; güvenlik için \
durduruldu. Klasörü inceleyip silin, sonra yeniden kurun.";

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

/// `taban\Kalfa` ve `taban\Kalfa\yalitim`'i sırayla hazırlar ve kayıt
/// dizinini döner. Yönetici gerekir; üretimde `taban` `%ProgramData%`'dır.
///
/// - Yoksa dizin, oluştuğu anda mirassız DACL'le oluşturulur.
/// - Reparse noktası izlenmeden açılır; reparse noktasıysa durur.
/// - Sahibi Administrators ya da SYSTEM değilse durur.
/// - DACL tutamaçla yeniden yazılır: Administrators ve SYSTEM tam, oturum
///   açmış kullanıcılar okuma.
pub fn kayit_dizini_hazirla(taban: &Path) -> io::Result<PathBuf> {
    let kabul = [Sid::metinden(YONETICILER)?, Sid::metinden(SISTEM)?];
    kayit_dizini_hazirla_ile(taban, &kabul, &kayit_dizini_izinleri()?)
}

fn kayit_dizini_izinleri() -> io::Result<Vec<Izin>> {
    let izin = |sid: &str, maske| -> io::Result<Izin> {
        Ok(Izin {
            sid: Sid::metinden(sid)?,
            maske,
            miras: MIRAS,
        })
    };
    Ok(vec![
        izin(YONETICILER, MASKE_TAM)?,
        izin(SISTEM, MASKE_TAM)?,
        izin(OTURUM_ACMIS, MASKE_OKU)?,
    ])
}

/// Testler yönetici olmadan koşar: kabul edilen sahipler ve DACL verilir.
fn kayit_dizini_hazirla_ile(taban: &Path, kabul: &[Sid], izinler: &[Izin]) -> io::Result<PathBuf> {
    let kalfa = taban.join("Kalfa");
    let dizin = kalfa.join("yalitim");
    for yol in [&kalfa, &dizin] {
        match korumali_dizin_olustur(yol, izinler) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
        let t = yeniden_ayristirma_izlemeden_ac(yol, true)?;
        let s = sahip(&t)?;
        if !kabul.contains(&s) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("{KAYIT_SAHIBI_HATASI} ({}, sahip {s:?})", yol.display()),
            ));
        }
        dacl_yaz_tutamacla(&t, izinler)?;
    }
    Ok(dizin)
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

    /// Kaydı yazar. Dizin önce `kayit_dizini_hazirla` ile hazırlanmış
    /// olmalıdır. Geçici dosya reparse noktası izlenmeden ve yalnızca yeni
    /// olarak açılır; önceden konmuş bir bağlantının hedefine yazılmaz.
    pub fn yaz(&self, dizin: &Path) -> io::Result<()> {
        let metin = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let gecici = dizin.join("kurulum.json.yeni");
        // Kalmış geçici dosya (ya da bağlantı) silinir; bağlantının kendisi
        // silinir, hedefi değil.
        match std::fs::remove_file(&gecici) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        let mut dosya = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&gecici)?;
        io::Write::write_all(&mut dosya, &metin)?;
        dosya.sync_all()?;
        drop(dosya);
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
    use std::process::Command;

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

    fn ben() -> Sid {
        guvenlik::kullanici_sidi().unwrap()
    }

    /// Üretim DACL'i ve testi koşan kullanıcıya tam izin: testler yönetici
    /// olmadan koşar.
    fn test_izinleri() -> Vec<Izin> {
        let mut izinler = kayit_dizini_izinleri().unwrap();
        izinler.push(Izin {
            sid: ben(),
            maske: MASKE_TAM,
            miras: MIRAS,
        });
        izinler
    }

    fn yonetici_olmayan() -> bool {
        !guvenlik::yukseltilmis_mi().unwrap()
    }

    #[test]
    fn yeni_kayit_dizini_mirassiz_ve_yazilabilir() {
        let d = tempfile::tempdir().unwrap();
        let dizin = kayit_dizini_hazirla_ile(d.path(), &[ben()], &test_izinleri()).unwrap();
        assert_eq!(dizin, d.path().join("Kalfa").join("yalitim"));
        for yol in [d.path().join("Kalfa"), dizin.clone()] {
            assert!(
                guvenlik::mirasi_kesik_mi(&yol).unwrap(),
                "{}",
                yol.display()
            );
            let sddl = guvenlik::dacl_sddl(&yol).unwrap();
            assert!(sddl.starts_with("D:P"), "{sddl}");
            assert!(
                sddl.contains(";;;BA)") && sddl.contains(";;;SY)") && sddl.contains(";;;AU)"),
                "{sddl}"
            );
        }
        // İkinci hazırlık da geçer; sahip değişmedi.
        kayit_dizini_hazirla_ile(d.path(), &[ben()], &test_izinleri()).unwrap();
    }

    #[test]
    fn kayit_dizini_sahibi_kabul_edilmiyorsa_reddedilir() {
        if !yonetici_olmayan() {
            eprintln!("yükseltilmiş süreçte oluşturulan dizinin sahibi yöneticidir; test atlandı");
            return;
        }
        let d = tempfile::tempdir().unwrap();
        // Standart kullanıcı `Kalfa`'yı önceden oluşturmuş: sahibi odur.
        std::fs::create_dir(d.path().join("Kalfa")).unwrap();
        let kabul = [
            Sid::metinden(YONETICILER).unwrap(),
            Sid::metinden(SISTEM).unwrap(),
        ];
        let hata = kayit_dizini_hazirla_ile(d.path(), &kabul, &test_izinleri()).unwrap_err();
        assert_eq!(hata.kind(), io::ErrorKind::PermissionDenied);
        assert!(hata.to_string().contains(KAYIT_SAHIBI_HATASI), "{hata}");
        assert!(!d.path().join("Kalfa").join("yalitim").exists());
        assert!(!guvenlik::mirasi_kesik_mi(&d.path().join("Kalfa")).unwrap());
    }

    #[test]
    fn kayit_dizini_junction_ise_reddedilir() {
        let d = tempfile::tempdir().unwrap();
        let hedef = d.path().join("hedef");
        std::fs::create_dir(&hedef).unwrap();
        std::fs::create_dir(d.path().join("Kalfa")).unwrap();
        let durum = Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(d.path().join("Kalfa").join("yalitim"))
            .arg(&hedef)
            .output()
            .unwrap();
        assert!(durum.status.success(), "{durum:?}");
        let onceki = guvenlik::dacl_sddl(&hedef).unwrap();
        let hata = kayit_dizini_hazirla_ile(d.path(), &[ben()], &test_izinleri()).unwrap_err();
        assert_eq!(hata.kind(), io::ErrorKind::PermissionDenied, "{hata}");
        assert_eq!(guvenlik::dacl_sddl(&hedef).unwrap(), onceki);
        assert!(!hedef.join("kurulum.json").exists());
        std::fs::remove_dir(d.path().join("Kalfa").join("yalitim")).unwrap();
    }

    #[test]
    fn kayit_yazmasi_onceden_konmus_baglantiyi_izlemez() {
        let d = tempfile::tempdir().unwrap();
        let hedef = d.path().join("hedef.txt");
        std::fs::write(&hedef, "dokunulmaz").unwrap();
        let dizin = d.path().join("kayit");
        std::fs::create_dir(&dizin).unwrap();
        // Geçici dosyanın yerine bırakılmış bir hard link: yazılırsa hedef değişirdi.
        std::fs::hard_link(&hedef, dizin.join("kurulum.json.yeni")).unwrap();
        KurulumKaydi {
            kok_izinleri: vec![PathBuf::from(r"C:\")],
            ..Default::default()
        }
        .yaz(&dizin)
        .unwrap();
        assert_eq!(std::fs::read_to_string(&hedef).unwrap(), "dokunulmaz");
        assert_eq!(
            KurulumKaydi::oku(&dizin).unwrap().unwrap().kok_izinleri,
            vec![PathBuf::from(r"C:\")]
        );
    }
}
