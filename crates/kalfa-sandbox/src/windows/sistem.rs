//! İşletim sistemi bilgisi ve kurulu yardımcının yükseltilerek çalıştırılması
//! (aşama 2 tasarımı §9.8). BT için teknik raporda Windows'un sürümü, yapısı
//! ve baskısı yazılır; "Onar", kurulu `kalfa-yalitim.exe`'yi UAC ile başlatır.

use std::io;
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, WAIT_OBJECT_0};
use windows_sys::Win32::System::Registry::{
    HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW,
};
use windows_sys::Win32::System::Threading::{GetExitCodeProcess, INFINITE, WaitForSingleObject};
use windows_sys::Win32::UI::Shell::{
    SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

use super::guvenlik::{genis, program_files_dizinleri, sade};

/// Yardımcının dosya adı; kurulu sürümde Kalfa'nın yanındadır.
pub const YARDIMCI: &str = "kalfa-yalitim.exe";

/// Kurulu yardımcı: `exe`'nin dizini Program Files'ın altındaysa ve yanında
/// `kalfa-yalitim.exe` varsa onun yolu. Program Files'a yalnızca yönetici
/// yazabilir; başka yerdeki (ör. `target\debug`) bir yardımcı, yalıtımın
/// değiştirebileceği bir dosya olabilir ve UAC ile hiçbir zaman başlatılmaz.
pub fn kurulu_yardimci(exe: &Path) -> Option<PathBuf> {
    let dizin = sade(&exe.canonicalize().ok()?).parent()?.to_path_buf();
    let kurulu = program_files_dizinleri().into_iter().any(|pf| {
        pf.canonicalize()
            .map(|pf| dizin.starts_with(sade(&pf)))
            .unwrap_or(false)
    });
    let yardimci = dizin.join(YARDIMCI);
    (kurulu && yardimci.is_file()).then_some(yardimci)
}

/// `exe`'yi UAC ile yönetici olarak başlatır ve bitmesini bekler; çıkış
/// kodunu döner. Kullanıcı UAC'de vazgeçerse `Ok(None)`.
pub fn yonetici_olarak_calistir(exe: &Path, argumanlar: &str) -> io::Result<Option<u32>> {
    let fiil = genis("runas");
    let dosya = genis(exe);
    let parametreler = genis(argumanlar);
    // SAFETY: yapı sıfırla başlatılır; dizeler NUL ile sonlanır ve çağrı
    // boyunca yaşar.
    let mut bilgi: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    bilgi.cbSize = size_of::<SHELLEXECUTEINFOW>() as u32;
    bilgi.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
    bilgi.lpVerb = fiil.as_ptr();
    bilgi.lpFile = dosya.as_ptr();
    bilgi.lpParameters = parametreler.as_ptr();
    bilgi.lpDirectory = null();
    bilgi.nShow = SW_HIDE;
    // SAFETY: `bilgi` geçerli ve doldurulmuş; Windows süreç tutamacını
    // `hProcess`'e yazar.
    if unsafe { ShellExecuteExW(&mut bilgi) } == 0 {
        let hata = io::Error::last_os_error();
        if hata.raw_os_error() == Some(ERROR_CANCELLED as i32) {
            return Ok(None);
        }
        return Err(hata);
    }
    if bilgi.hProcess.is_null() {
        return Err(io::Error::other("yükseltilen sürecin tutamacı alınamadı"));
    }
    let surec = bilgi.hProcess;
    // SAFETY: tutamaç bu işleve aittir ve sonunda kapatılır.
    unsafe {
        let mut kod = 0u32;
        let beklendi = WaitForSingleObject(surec, INFINITE) == WAIT_OBJECT_0;
        let okundu = beklendi && GetExitCodeProcess(surec, &mut kod) != 0;
        let hata = (!okundu).then(io::Error::last_os_error);
        CloseHandle(surec);
        match hata {
            Some(e) => Err(e),
            None => Ok(Some(kod)),
        }
    }
}

const SURUM_ANAHTARI: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";

/// Windows 11'in ilk yapısı. `ProductName` Windows 11'de de "Windows 10" der.
const WINDOWS_11_YAPISI: u32 = 22000;

fn metin_degeri(ad: &str) -> Option<String> {
    let anahtar = genis(SURUM_ANAHTARI);
    let ad = genis(ad);
    let mut boyut: u32 = 0;
    // SAFETY: anahtar ve değer adları NUL ile sonlanır; veri işaretçisi boş
    // olduğu için Windows yalnızca gereken boyutu yazar.
    let sonuc = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            anahtar.as_ptr(),
            ad.as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            null_mut(),
            &mut boyut,
        )
    };
    if sonuc != 0 || boyut == 0 {
        return None;
    }
    let mut tampon = vec![0u16; (boyut as usize).div_ceil(2)];
    // SAFETY: tampon `boyut` bayt tutar; Windows en çok o kadar yazar ve
    // yazdığını `boyut`'a koyar.
    let sonuc = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            anahtar.as_ptr(),
            ad.as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            tampon.as_mut_ptr().cast(),
            &mut boyut,
        )
    };
    if sonuc != 0 {
        return None;
    }
    let son = tampon.iter().position(|&c| c == 0).unwrap_or(tampon.len());
    Some(String::from_utf16_lossy(&tampon[..son]))
}

fn sayi_degeri(ad: &str) -> Option<u32> {
    let anahtar = genis(SURUM_ANAHTARI);
    let ad = genis(ad);
    let mut deger: u32 = 0;
    let mut boyut = size_of::<u32>() as u32;
    // SAFETY: `deger` dört bayttır ve `boyut` bunu söyler.
    let sonuc = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            anahtar.as_ptr(),
            ad.as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            (&mut deger as *mut u32).cast(),
            &mut boyut,
        )
    };
    (sonuc == 0).then_some(deger)
}

/// Örneğin "Windows 11 Home 25H2 (yapı 26200.9457, Core)".
pub fn surum() -> String {
    let yapi = metin_degeri("CurrentBuild").unwrap_or_default();
    let mut urun = metin_degeri("ProductName").unwrap_or_else(|| "Windows".into());
    if yapi.parse::<u32>().is_ok_and(|y| y >= WINDOWS_11_YAPISI) {
        urun = urun.replacen("Windows 10", "Windows 11", 1);
    }
    let gorunen = metin_degeri("DisplayVersion").unwrap_or_default();
    let ubr = sayi_degeri("UBR")
        .map(|u| format!(".{u}"))
        .unwrap_or_default();
    let baski = metin_degeri("EditionID").unwrap_or_default();
    format!("{urun} {gorunen} (yapı {yapi}{ubr}, {baski})")
}

/// BT için gereksinim paragrafı (§9.8).
pub fn gereksinim() -> String {
    let sid = super::guvenlik::arac_zinciri_sidi()
        .map(|s| s.metin())
        .unwrap_or_else(|e| format!("(türetilemedi: {e})"));
    format!(
        "Kalfa, yapay zekânın önerdiği komutları kullanıcının kendi hesabıyla, çalışma \
         alanına özel bir Windows AppContainer'ında (düşük bütünlük, Job Object) çalıştırır; \
         yönetici hakkı istemez. Kurulum bir kez yönetici olarak `kalfa-yalitim.exe kur` \
         çalıştırır: sabit disklerin köklerine ve C:\\Users dizinine, `kalfaAracZinciri` \
         yetenek SID'i ({sid}) için mirassız listeleme izni (0x1000A1) ekler ve kaydı \
         %ProgramData%\\Kalfa\\yalitim\\kurulum.json dosyasına yazar. Uygulama her \
         açılışta `Kalfa.Deneme` adlı bir AppContainer profilinde cmd.exe'yi deneyerek \
         kurulumu doğrular. Engelleyebilecekler: uygulama denetimi ilkeleri (AppLocker, \
         WDAC), kullanıcıların AppContainer profili oluşturmasını kısıtlayan yapılandırma \
         ve sürücü köklerinin izinlerini yeniden yazan yapılandırma yönetimi. Onarım: \
         Kalfa'nın kurulum dizinindeki `kalfa-yalitim.exe kur` komutunu yönetici olarak \
         çalıştırın ya da Kalfa'da Ayarlar > Onar'ı kullanın."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Güvenlik sınırı (§9.8): Program Files dışındaki yardımcı UAC ile
    /// başlatılmaz. Test ikilisi `target\debug\deps`'tedir; yanına konan bir
    /// `kalfa-yalitim.exe` da kurulu sayılmaz.
    #[test]
    fn program_files_disindaki_yardimci_kurulu_sayilmaz() {
        let d = tempfile::tempdir().unwrap();
        let exe = d.path().join("kalfa.exe");
        std::fs::write(&exe, b"").unwrap();
        std::fs::write(d.path().join(YARDIMCI), b"").unwrap();
        assert_eq!(kurulu_yardimci(&exe), None);
        let test_ikilisi = std::env::current_exe().unwrap();
        assert_eq!(kurulu_yardimci(&test_ikilisi), None);
    }

    #[test]
    fn program_files_dizinleri_bulunur() {
        let dizinler = program_files_dizinleri();
        assert!(!dizinler.is_empty());
        assert!(dizinler.iter().all(|d| d.is_dir()), "{dizinler:?}");
    }

    #[test]
    fn surum_yapiyi_ve_urunu_icerir() {
        let s = surum();
        assert!(s.starts_with("Windows"), "{s}");
        assert!(s.contains("yapı "), "{s}");
        assert!(!s.contains("yapı ,"), "yapı numarası okunmalı: {s}");
    }
}
