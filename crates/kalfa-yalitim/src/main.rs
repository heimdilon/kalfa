//! `kalfa-yalitim`: Kalfa'nın Windows yalıtımı için yönetici yardımcısı
//! (aşama 2 tasarımı §9.1). Bütün alt komutları yönetici olarak çalışır:
//!
//! - `kur [--izin-ver <dizin>]...`: sabit disklerin köküne araç zinciri
//!   yeteneği için listeleme izni verir (§9.2) ve kurulum kaydını yazar;
//!   ardından her dizin için `izin-ver`. Bir dizin reddedilirse kök izinleri
//!   kalır, çıkış kodu 3'tür ve dizin standart çıktıya yazılır.
//! - `kaldir`: kurulum kaydına göre her şeyi geri alır.
//! - `izin-ver <dizin>` / `izin-geri-al <dizin>`: geliştiricinin sahibi
//!   olmadığı bir araç zinciri dizinine yetenek için okuma izni verir ya da
//!   geri alır (§9.4).
//!
//! Komutu çalıştırmak için gerekmez: Kalfa kabı kendisi başlatır (§9.3).

use std::process::ExitCode;

#[cfg(windows)]
const KULLANIM: &str = "Kullanım: kalfa-yalitim kur [--izin-ver <dizin>]... | kaldir | \
                        izin-ver <dizin> | izin-geri-al <dizin>";

/// `kur`'da kök izinleri kuruldu ama bazı araç zinciri dizinleri açılamadı.
#[cfg(windows)]
const KISMI_KURULUM: u8 = 3;

/// Çıkış kodu ve iletisiyle hata. Düz ileti 1 koduyla çıkar.
struct Hata {
    kod: u8,
    ileti: String,
}

impl From<String> for Hata {
    fn from(ileti: String) -> Self {
        Hata { kod: 1, ileti }
    }
}

impl From<&str> for Hata {
    fn from(ileti: &str) -> Self {
        ileti.to_string().into()
    }
}

fn main() -> ExitCode {
    let argumanlar: Vec<String> = std::env::args().skip(1).collect();
    match calistir(&argumanlar) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", e.ileti);
            ExitCode::from(e.kod)
        }
    }
}

#[cfg(not(windows))]
fn calistir(_argumanlar: &[String]) -> Result<(), Hata> {
    Err(String::from("kalfa-yalitim yalnızca Windows'ta çalışır.").into())
}

#[cfg(windows)]
fn calistir(argumanlar: &[String]) -> Result<(), Hata> {
    windows::yap(ayristir(argumanlar)?)
}

/// Komut satırını işe çevirir. `kur`, ardından sıfır ya da daha çok
/// `--izin-ver <dizin>` alır (2d-3: "Onar" araç zinciri izinlerini de geri
/// getirir).
#[cfg(windows)]
fn ayristir(argumanlar: &[String]) -> Result<windows::Is, String> {
    use std::path::PathBuf;
    let Some((komut, kalan)) = argumanlar.split_first() else {
        return Err(KULLANIM.into());
    };
    let dizin = || -> Result<PathBuf, String> {
        match kalan {
            [d] => Ok(PathBuf::from(d)),
            _ => Err(KULLANIM.into()),
        }
    };
    Ok(match komut.as_str() {
        "kur" => {
            let mut izinli = Vec::new();
            let mut parcalar = kalan.iter();
            while let Some(parca) = parcalar.next() {
                match (parca.as_str(), parcalar.next()) {
                    ("--izin-ver", Some(d)) if !d.is_empty() => izinli.push(PathBuf::from(d)),
                    _ => return Err(KULLANIM.into()),
                }
            }
            windows::Is::Kur(izinli)
        }
        "kaldir" if kalan.is_empty() => windows::Is::Kaldir,
        "izin-ver" => windows::Is::IzinVer(dizin()?),
        "izin-geri-al" => windows::Is::IzinGeriAl(dizin()?),
        _ => return Err(KULLANIM.into()),
    })
}

#[cfg(windows)]
mod windows {
    use std::io;
    use std::path::{Path, PathBuf};

    use kalfa_sandbox::windows::guvenlik::{
        Izin, MASKE_OKU, MIRAS, arac_zinciri_sidi, izin_ekle, izni_kaldir, kok_izni_kaldir,
        kok_izni_var_mi, kok_izni_ver, profiller_dizini, sabit_diskler, sade, yukseltilmis_mi,
    };
    use kalfa_sandbox::windows::kurulum::{KurulumKaydi, kayit_dizini};

    use super::{Hata, KISMI_KURULUM};

    #[derive(Debug, PartialEq, Eq)]
    pub enum Is {
        /// Kök izinleri, ardından verilen araç zinciri dizinlerine `izin-ver`.
        Kur(Vec<PathBuf>),
        Kaldir,
        IzinVer(PathBuf),
        IzinGeriAl(PathBuf),
    }

    enum KurulumDurumu {
        Okundu,
        Bulunamadi,
        Okunamadi(String),
    }

    enum YolDurumu {
        Var,
        Yok,
        Bilinmiyor(io::Error),
    }

    fn metin(e: impl std::fmt::Display) -> String {
        e.to_string()
    }

    pub fn yap(is: Is) -> Result<(), Hata> {
        if !yukseltilmis_mi().map_err(metin)? {
            return Err("Bu komut yönetici olarak çalıştırılmalıdır.".into());
        }
        let dizin = kayit_dizini().map_err(metin)?;
        // Bozuk kayıtla yalnızca `kaldir` sürer: kök izinlerini sürücülerden
        // bulur. Diğer komutlar kaydı ezmemek için durur.
        let (mut kayit, durum) = match KurulumKaydi::oku(&dizin) {
            Ok(Some(kayit)) => (kayit, KurulumDurumu::Okundu),
            Ok(None) => (KurulumKaydi::default(), KurulumDurumu::Bulunamadi),
            Err(e) if matches!(&is, Is::Kaldir) => {
                println!(
                    "{e}; kök izinleri taranacak; izin-ver dizinleri bilinmediğinden kaldırma başarısı doğrulanamayacak."
                );
                (
                    KurulumKaydi::default(),
                    KurulumDurumu::Okunamadi(e.to_string()),
                )
            }
            Err(e) => {
                return Err(format!("{e}. Kaydı inceleyin ya da önce kaldir.").into());
            }
        };
        match is {
            Is::Kur(izinli) => {
                kur(&dizin, &mut kayit)?;
                // Her dizin `izin-ver`'in denetimleriyle ayrı ayrı açılır;
                // biri reddedilse de ötekiler ve kök izinleri kalır.
                let mut reddedilen = 0;
                for d in &izinli {
                    if let Err(e) = izin_ver(&dizin, &mut kayit, d) {
                        eprintln!("{e}");
                        println!("reddedildi: {}", d.display());
                        reddedilen += 1;
                    }
                }
                if reddedilen > 0 {
                    return Err(Hata {
                        kod: KISMI_KURULUM,
                        ileti: format!("{reddedilen} araç zinciri dizini açılamadı."),
                    });
                }
            }
            Is::Kaldir => kaldir(&dizin, &kayit, durum)?,
            Is::IzinVer(d) => izin_ver(&dizin, &mut kayit, &d)?,
            Is::IzinGeriAl(d) => izin_geri_al(&dizin, &mut kayit, &d)?,
        }
        Ok(())
    }

    /// Kök izni. Kayıt izinden önce yazılır: izin yarıda kalsa da `kaldir`
    /// onu kaldırmayı dener.
    fn kur(dizin: &Path, kayit: &mut KurulumKaydi) -> Result<(), String> {
        let sid = arac_zinciri_sidi().map_err(metin)?;
        let diskler = sabit_diskler();
        if diskler.is_empty() {
            return Err("ACL taşıyan sabit disk bulunamadı.".into());
        }
        // Profil altındaki çalışma alanlarının yolu çözülürken kap profiller
        // dizinini de listeler (A11); dizin yöneticiye aittir.
        let mut hedefler = diskler;
        hedefler.push(profiller_dizini().map_err(metin)?);
        for kok in hedefler {
            if !kayit.kok_izinleri.contains(&kok) {
                kayit.kok_izinleri.push(kok.clone());
                kayit.yaz(dizin).map_err(metin)?;
            }
            if kok_izni_var_mi(&kok, &sid).map_err(metin)? {
                println!("{}: izin zaten var.", kok.display());
                continue;
            }
            kok_izni_ver(&kok, &sid).map_err(metin)?;
            println!("{}: listeleme izni verildi.", kok.display());
        }
        println!(
            "Kurulum tamam. Kayıt: {}",
            dizin.join("kurulum.json").display()
        );
        Ok(())
    }

    /// Her adımı bağımsız dener. Kök izinlerini kayıttakilere ek olarak
    /// sürücülerde arar. Kayıt okunamıyorsa kök temizliği denenir ancak
    /// izin-ver dizinleri bilinmediği için işlem başarıyla tamamlanmış sayılamaz.
    fn kaldir(dizin: &Path, kayit: &KurulumKaydi, durum: KurulumDurumu) -> Result<(), String> {
        println!(
            "Kök izni bu makinedeki bütün hesapların Kalfa yalıtımı için ortaktır; kaldırınca başka hesaplardaki yalıtım da çalışmaz."
        );
        let sid = arac_zinciri_sidi().map_err(metin)?;
        let mut kokler = kayit.kok_izinleri.clone();
        let mut kalan = KurulumKaydi::default();
        for kok in sabit_diskler().into_iter().chain(profiller_dizini().ok()) {
            if !kokler.contains(&kok) {
                kok_taramasi_sonucunu_isle(
                    kok.clone(),
                    kok_izni_var_mi(&kok, &sid),
                    &mut kokler,
                    &mut kalan,
                );
            }
        }
        for kok in &kokler {
            match kok_izni_kaldir(kok, &sid) {
                Ok(()) => println!("{}: listeleme izni kaldırıldı.", kok.display()),
                Err(e) => {
                    println!("{}: kaldırılamadı: {e}", kok.display());
                    if !kalan.kok_izinleri.contains(kok) {
                        kalan.kok_izinleri.push(kok.clone());
                    }
                }
            }
        }
        for d in &kayit.izinli_dizinler {
            match izinli_dizin_durumunu_isle(d, d.try_exists(), &mut kalan) {
                YolDurumu::Yok => continue,
                YolDurumu::Bilinmiyor(e) => {
                    println!(
                        "{}: dizin durumu denetlenemedi, izin geri alınmış sayılamaz: {e}",
                        d.display()
                    );
                    continue;
                }
                YolDurumu::Var => {}
            }
            match izni_kaldir(d, &sid) {
                Ok(()) => println!("{}: okuma izni kaldırıldı.", d.display()),
                Err(e) => {
                    println!("{}: kaldırılamadı: {e}", d.display());
                    if !kalan.izinli_dizinler.iter().any(|yol| yol == d) {
                        kalan.izinli_dizinler.push(d.clone());
                    }
                }
            }
        }
        kaldirmayi_sonlandir(
            &durum,
            &kalan,
            || KurulumKaydi::sil(dizin),
            |kalan| kalan.yaz(dizin),
        )
    }

    fn kok_taramasi_sonucunu_isle(
        kok: PathBuf,
        sonuc: io::Result<bool>,
        kokler: &mut Vec<PathBuf>,
        kalan: &mut KurulumKaydi,
    ) {
        match sonuc {
            Ok(true) => {
                if !kokler.contains(&kok) {
                    kokler.push(kok);
                }
            }
            Ok(false) => {}
            Err(e) => {
                println!(
                    "{}: izin durumu okunamadı; kaldırıldığı doğrulanamadı: {e}",
                    kok.display()
                );
                if !kalan.kok_izinleri.contains(&kok) {
                    kalan.kok_izinleri.push(kok);
                }
            }
        }
    }

    fn izinli_dizin_durumunu_isle(
        d: &Path,
        sonuc: io::Result<bool>,
        kalan: &mut KurulumKaydi,
    ) -> YolDurumu {
        match sonuc {
            Ok(true) => YolDurumu::Var,
            Ok(false) => YolDurumu::Yok,
            Err(e) if e.kind() == io::ErrorKind::NotFound => YolDurumu::Yok,
            Err(e) => {
                if !kalan.izinli_dizinler.contains(&d.to_path_buf()) {
                    kalan.izinli_dizinler.push(d.to_path_buf());
                }
                YolDurumu::Bilinmiyor(e)
            }
        }
    }

    fn kaldirmayi_sonlandir<Fsil, Fyaz>(
        durum: &KurulumDurumu,
        kalan: &KurulumKaydi,
        sil: Fsil,
        yaz: Fyaz,
    ) -> Result<(), String>
    where
        Fsil: FnOnce() -> io::Result<()>,
        Fyaz: FnOnce(&KurulumKaydi) -> io::Result<()>,
    {
        match durum {
            KurulumDurumu::Okundu if kalan == &KurulumKaydi::default() => {
                sil().map_err(metin)?;
                println!("Kaldırma tamam.");
                Ok(())
            }
            KurulumDurumu::Okundu => {
                yaz(kalan).map_err(metin)?;
                Err(
                    "Bazı izinler kaldırılamadı; kayıt yalnızca geri alınamayanları tutuyor."
                        .into(),
                )
            }
            KurulumDurumu::Bulunamadi => {
                if kalan != &KurulumKaydi::default() {
                    yaz(kalan).map_err(metin)?;
                }
                Err(
                    "Kurulum kaydı bulunamadı; izin-ver dizinleri bilinmediğinden kaldırmanın \
                     tamamlandığı doğrulanamadı."
                        .into(),
                )
            }
            KurulumDurumu::Okunamadi(e) => Err(format!(
                "Kurulum kaydı okunamadı ({e}); izin-ver dizinleri bilinmiyor. \
                 Kök izinleri tarandı; kayıt korunuyor, kaldırma tamamlanmış sayılamaz."
            )),
        }
    }

    fn dizin_coz(d: &Path) -> Result<PathBuf, String> {
        let kanonik = d
            .canonicalize()
            .map_err(|e| format!("{}: {e}", d.display()))?;
        if !kanonik.is_dir() {
            return Err(format!("{} bir dizin değil.", d.display()));
        }
        Ok(sade(&kanonik))
    }

    fn izin_ver(dizin: &Path, kayit: &mut KurulumKaydi, d: &Path) -> Result<(), String> {
        let d = dizin_coz(d)?;
        let sid = arac_zinciri_sidi().map_err(metin)?;
        if !kayit.izinli_dizinler.contains(&d) {
            kayit.izinli_dizinler.push(d.clone());
            kayit.yaz(dizin).map_err(metin)?;
        }
        izin_ekle(
            &d,
            &Izin {
                sid,
                maske: MASKE_OKU,
                miras: MIRAS,
            },
        )
        .map_err(metin)?;
        println!("{}: okuma izni verildi.", d.display());
        Ok(())
    }

    fn izin_geri_al(dizin: &Path, kayit: &mut KurulumKaydi, d: &Path) -> Result<(), String> {
        let d = dizin_coz(d)?;
        izin_geri_al_kayitta_mi(kayit, &d)?;
        let sid = arac_zinciri_sidi().map_err(metin)?;
        izni_kaldir(&d, &sid).map_err(metin)?;
        kayit.izinli_dizinler.retain(|x| x != &d);
        kayit.yaz(dizin).map_err(metin)?;
        println!("{}: okuma izni kaldırıldı.", d.display());
        Ok(())
    }

    fn izin_geri_al_kayitta_mi(kayit: &KurulumKaydi, d: &Path) -> Result<(), String> {
        if kayit.izinli_dizinler.iter().any(|yol| yol == d) {
            Ok(())
        } else {
            Err(format!(
                "{} izin-ver kaydında yok; mevcut ACL izinlerine dokunulmadı.",
                d.display()
            ))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn bozuk_kayit_kaldirma_kaydini_silmez_ve_yazmaz() {
            let mut silindi = false;
            let mut yazildi = false;
            let sonuc = kaldirmayi_sonlandir(
                &KurulumDurumu::Okunamadi("bozuk kurulum.json".into()),
                &KurulumKaydi::default(),
                || {
                    silindi = true;
                    Ok(())
                },
                |_| {
                    yazildi = true;
                    Ok(())
                },
            );
            assert!(sonuc.is_err());
            assert!(!silindi);
            assert!(!yazildi);
        }

        #[test]
        fn kok_acl_okuma_hatasi_kok_kalanlara_eklenir() {
            let kok = PathBuf::from(r"C:\");
            let mut kokler = Vec::new();
            let mut kalan = KurulumKaydi::default();
            kok_taramasi_sonucunu_isle(
                kok.clone(),
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "ACL okunamıyor",
                )),
                &mut kokler,
                &mut kalan,
            );
            assert!(kokler.is_empty());
            assert_eq!(kalan.kok_izinleri, vec![kok]);
        }

        #[test]
        fn izinli_dizin_not_found_ise_kalanlara_girmez() {
            let d = PathBuf::from(r"C:\olmayan");
            let mut kalan = KurulumKaydi::default();
            let durum = izinli_dizin_durumunu_isle(
                &d,
                Err(io::Error::from(io::ErrorKind::NotFound)),
                &mut kalan,
            );
            assert!(matches!(durum, YolDurumu::Yok));
            assert!(kalan.izinli_dizinler.is_empty());
        }

        #[test]
        fn izinli_dizin_metadata_hatasi_kalanlarda_tutulur() {
            let d = PathBuf::from(r"C:\erisilemeyen");
            let mut kalan = KurulumKaydi::default();
            let durum = izinli_dizin_durumunu_isle(
                &d,
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "metadata okunamıyor",
                )),
                &mut kalan,
            );
            match durum {
                YolDurumu::Bilinmiyor(e) => {
                    assert_eq!(e.kind(), io::ErrorKind::PermissionDenied);
                }
                _ => panic!("Erişim hatası belirsiz olarak sınıflandırılmalı."),
            }
            assert_eq!(kalan.izinli_dizinler, vec![d]);
        }

        #[test]
        fn izin_geri_al_kayit_disi_dizini_reddeder() {
            let kayit = KurulumKaydi::default();
            let d = PathBuf::from(r"C:\araclar");
            assert!(izin_geri_al_kayitta_mi(&kayit, &d).is_err());
        }

        #[test]
        fn izin_geri_al_kayitli_dizini_kabul_eder() {
            let d = PathBuf::from(r"C:\araclar");
            let kayit = KurulumKaydi {
                izinli_dizinler: vec![d.clone()],
                ..KurulumKaydi::default()
            };
            assert!(izin_geri_al_kayitta_mi(&kayit, &d).is_ok());
        }

        #[test]
        fn kalan_izin_kaydi_silinmeden_ve_korunarak_yazilir() {
            let kalan = KurulumKaydi {
                izinli_dizinler: vec![PathBuf::from(r"C:\araclar")],
                ..KurulumKaydi::default()
            };
            let mut silindi = false;
            let mut yazilan = None;
            let sonuc = kaldirmayi_sonlandir(
                &KurulumDurumu::Okundu,
                &kalan,
                || {
                    silindi = true;
                    Ok(())
                },
                |kayit| {
                    yazilan = Some(kayit.clone());
                    Ok(())
                },
            );
            assert!(sonuc.is_err());
            assert!(!silindi);
            assert_eq!(yazilan, Some(kalan));
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn arg(parcalar: &[&str]) -> Vec<String> {
        parcalar.iter().map(|p| p.to_string()).collect()
    }

    #[test]
    fn kur_izin_ver_argumanlari_ayrisir() {
        assert_eq!(
            ayristir(&arg(&["kur"])).ok(),
            Some(windows::Is::Kur(Vec::new()))
        );
        assert_eq!(
            ayristir(&arg(&[
                "kur",
                "--izin-ver",
                r"C:\a",
                "--izin-ver",
                r"C:\b c"
            ]))
            .ok(),
            Some(windows::Is::Kur(vec![
                PathBuf::from(r"C:\a"),
                PathBuf::from(r"C:\b c")
            ]))
        );
        for yanlis in [
            &["kur", r"C:\a"][..],
            &["kur", "--izin-ver"],
            &["kur", "--izin-ver", ""],
            &["kur", "--izin-ver", r"C:\a", "fazla"],
            &["kur", "--baska", r"C:\a"],
            &["kaldir", "--izin-ver", r"C:\a"],
        ] {
            assert!(ayristir(&arg(yanlis)).is_err(), "{yanlis:?}");
        }
    }
}
