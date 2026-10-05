//! Bir dosyanın hard link sayısı (spec §7.3) ve Windows'ta açılan tutamacın
//! gerçek adı.

use std::fs::File;
use std::io;

#[cfg(unix)]
pub fn sayi(dosya: &File) -> io::Result<u64> {
    use std::os::unix::fs::MetadataExt;

    Ok(dosya.metadata()?.nlink())
}

#[cfg(windows)]
pub fn sayi(dosya: &File) -> io::Result<u64> {
    use std::os::windows::io::AsRawHandle;

    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };

    // SAFETY: `dosya` yaşadığı sürece ham tutamaç geçerlidir; Windows API
    // yapıyı doldurur ve sıfırlarla başlatılmış değer geçerli bir başlangıçtır.
    let mut bilgi: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    let tamam = unsafe { GetFileInformationByHandle(dosya.as_raw_handle(), &mut bilgi) };
    if tamam == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(u64::from(bilgi.nNumberOfLinks))
}

/// Tutamacın açıldığı dizin girdisinin uzun adı. Normalleştirilmiş yol 8.3
/// kısa adları uzun adlara çevirir ve harfleri diskteki hâliyle verir; hard
/// link'te açılan bağlantının adı döner. Dizini taramadan tek çağrıdır.
#[cfg(windows)]
pub fn gercek_ad(dosya: &File) -> io::Result<std::ffi::OsString> {
    use std::os::windows::ffi::OsStringExt;
    use std::os::windows::io::AsRawHandle;

    use windows_sys::Win32::Storage::FileSystem::{
        FILE_NAME_NORMALIZED, GetFinalPathNameByHandleW, VOLUME_NAME_NONE,
    };

    let mut tampon = vec![0_u16; 512];
    loop {
        let boyut = u32::try_from(tampon.len()).map_err(io::Error::other)?;
        // SAFETY: `dosya` yaşadığı sürece ham tutamaç geçerlidir; tampon
        // `boyut` kadar u16 yazılabilir yer taşır ve çağrı süresince yaşar.
        let adet = unsafe {
            GetFinalPathNameByHandleW(
                dosya.as_raw_handle(),
                tampon.as_mut_ptr(),
                boyut,
                FILE_NAME_NORMALIZED | VOLUME_NAME_NONE,
            )
        } as usize;
        if adet == 0 {
            return Err(io::Error::last_os_error());
        }
        // Tampon yetmezse dönüş, sondaki NUL dahil gereken boyuttur.
        if adet >= tampon.len() {
            tampon.resize(adet + 1, 0);
            continue;
        }
        let yol = std::path::PathBuf::from(std::ffi::OsString::from_wide(&tampon[..adet]));
        return yol
            .file_name()
            .map(std::ffi::OsStr::to_os_string)
            .ok_or_else(|| io::Error::other("Tutamacın adı çözülemedi."));
    }
}
