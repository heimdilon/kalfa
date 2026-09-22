//! Bir dosyanın hard link sayısı (spec §7.3).

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
