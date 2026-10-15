//! Komutun AppContainer'da başlatılması (aşama 2 tasarımı §9.3).
//!
//! - `CreateProcessW`'ye `STARTUPINFOEXW` verilir: güvenlik yetenekleri paket
//!   SID'ini ve yetenekleri, tutamaç listesi yalnızca üç standart tutamacı
//!   taşır.
//! - Süreç askıda başlar, Job Object'e atanır, sonra sürdürülür.
//! - Aracı süreç yoktur. Kabukta standart girdi `NUL`'dur; etkileşimli
//!   süreçte (1.1a §4.1) bir borudur.

use std::ffi::c_void;
use std::fs::File;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{
    GENERIC_READ, GetLastError, HANDLE, HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE,
    SetHandleInformation, WAIT_OBJECT_0,
};
use windows_sys::Win32::Security::{
    SECURITY_ATTRIBUTES, SECURITY_CAPABILITIES, SID_AND_ATTRIBUTES,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Diagnostics::Debug::{
    SEM_FAILCRITICALERRORS, SEM_NOGPFAULTERRORBOX, SEM_NOOPENFILEERRORBOX, SetErrorMode,
};
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows_sys::Win32::System::SystemServices::SE_GROUP_ENABLED;
use windows_sys::Win32::System::Threading::{
    CREATE_NO_WINDOW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW,
    DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess, INFINITE,
    InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
    PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
    PROCESS_INFORMATION, ResumeThread, STARTF_USESTDHANDLES, STARTUPINFOEXW, TerminateProcess,
    UpdateProcThreadAttribute, WaitForSingleObject,
};

use super::guvenlik::{Sid, genis};
use super::komut_satiri::KomutSatiri;
use crate::agac::Is;

pub(crate) struct KapIstegi<'a> {
    pub komut: &'a KomutSatiri,
    pub dizin: &'a Path,
    pub ortam: &'a [(String, String)],
    pub paket: &'a Sid,
    pub yetenekler: &'a [Sid],
}

/// Başlamış kap: süreç tutamacı ve iki çıktı borusunun okuma uçları.
pub(crate) struct Kap {
    pub surec: OwnedHandle,
    pub cikti: File,
    pub hata: File,
    /// Etkileşimli süreçte stdin'in yazma ucu; kabukta `None` (stdin `NUL`).
    pub girdi: Option<File>,
}

/// Yükleyici hata kutularını kapatır. Kap bu kipi devralır; yoksa eksik bir
/// DLL, UI kısıtlarına rağmen geliştiricinin masaüstünde iletişim kutusu
/// açar ve komut kutu kapanana kadar bekler (2c, P10).
pub(crate) fn hata_kutularini_kapat() {
    // SAFETY: yalnızca sürecin hata kipini değiştirir.
    unsafe {
        SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX);
    }
}

/// `System32`'nin yolu; toplu dosyaları çalıştıran cmd.exe oradadır.
pub(crate) fn sistem_dizini() -> PathBuf {
    let mut tampon = vec![0u16; 260];
    // SAFETY: tampon boyutuyla birlikte verilir.
    let n = unsafe { GetSystemDirectoryW(tampon.as_mut_ptr(), tampon.len() as u32) } as usize;
    if n == 0 || n >= tampon.len() {
        return PathBuf::from(r"C:\Windows\System32");
    }
    PathBuf::from(String::from_utf16_lossy(&tampon[..n]))
}

fn son_hata(adim: &str) -> io::Error {
    let e = io::Error::last_os_error();
    io::Error::new(e.kind(), format!("{adim}: {e}"))
}

/// Tutamaç; düşünce kapanır.
fn sahiplen(h: HANDLE) -> OwnedHandle {
    // SAFETY: tutamaç yeni açıldı ve yalnızca burada sahiplenilir.
    unsafe { OwnedHandle::from_raw_handle(h as _) }
}

/// Anonim boru: okuma ucu Kalfa'da kalır (miras alınmaz), yazma ucu çocuğa
/// geçer. Çocuğa yalnızca tutamaç listesindekiler geçer; yine de yazma ucu,
/// `CreateProcessW` ile kapatılışı arasında başka bir iş parçacığının
/// tutamaç listesiz başlattığı sürece geçebilir. Öyle olursa boru o süreç
/// bitene kadar açık kalır; `Surec::bekle` komut bittikten sonra en çok iki
/// saniye bekler.
fn boru() -> io::Result<(OwnedHandle, OwnedHandle)> {
    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: 1,
    };
    let mut okuma: HANDLE = null_mut();
    let mut yazma: HANDLE = null_mut();
    // SAFETY: çıktı tutamaçları hemen sahiplenilir.
    unsafe {
        if CreatePipe(&mut okuma, &mut yazma, &sa, 0) == 0 {
            return Err(son_hata("CreatePipe"));
        }
    }
    let (okuma, yazma) = (sahiplen(okuma), sahiplen(yazma));
    // SAFETY: tutamaç geçerlidir.
    if unsafe { SetHandleInformation(okuma.as_raw_handle() as HANDLE, HANDLE_FLAG_INHERIT, 0) } == 0
    {
        return Err(son_hata("SetHandleInformation"));
    }
    Ok((okuma, yazma))
}

/// stdin borusu: okuma ucu çocuğa geçer (tutamaç listesinde), yazma ucu
/// Kalfa'da kalır ve miras alınmaz.
fn girdi_borusu() -> io::Result<(OwnedHandle, OwnedHandle)> {
    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: 1,
    };
    let mut okuma: HANDLE = null_mut();
    let mut yazma: HANDLE = null_mut();
    // SAFETY: çıktı tutamaçları hemen sahiplenilir.
    unsafe {
        if CreatePipe(&mut okuma, &mut yazma, &sa, 0) == 0 {
            return Err(son_hata("CreatePipe(girdi)"));
        }
    }
    let (okuma, yazma) = (sahiplen(okuma), sahiplen(yazma));
    // SAFETY: tutamaç geçerlidir.
    if unsafe { SetHandleInformation(yazma.as_raw_handle() as HANDLE, HANDLE_FLAG_INHERIT, 0) } == 0
    {
        return Err(son_hata("SetHandleInformation(girdi)"));
    }
    Ok((okuma, yazma))
}

fn nul() -> io::Result<OwnedHandle> {
    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: 1,
    };
    // SAFETY: dize NUL ile biter; tutamaç hemen sahiplenilir.
    let h = unsafe {
        CreateFileW(
            genis("NUL").as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            &sa,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            null_mut(),
        )
    };
    if h == INVALID_HANDLE_VALUE {
        return Err(son_hata("CreateFileW(NUL)"));
    }
    Ok(sahiplen(h))
}

/// Ortam bloğu: `AD=DEĞER\0 … \0`, adlar büyük/küçük harf ayırmadan sıralı.
fn ortam_blogu(ortam: &[(String, String)]) -> Vec<u16> {
    let mut sirali: Vec<&(String, String)> = ortam.iter().collect();
    sirali.sort_by_key(|(ad, _)| ad.to_uppercase());
    let mut blok = Vec::new();
    for (ad, deger) in sirali {
        blok.extend(format!("{ad}={deger}").encode_utf16());
        blok.push(0);
    }
    if blok.is_empty() {
        blok.push(0);
    }
    blok.push(0);
    blok
}

/// Süreç öznitelik listesi; düşünce silinir.
struct Oznitelikler(Vec<u64>);

impl Oznitelikler {
    fn yeni(sayi: u32) -> io::Result<Self> {
        let mut boyut = 0usize;
        // SAFETY: önce boyut sorulur, sonra 8 bayt hizalı, yeterli tampon verilir.
        unsafe {
            InitializeProcThreadAttributeList(null_mut(), sayi, 0, &mut boyut);
            let mut v = vec![0u64; boyut.div_ceil(8)];
            if InitializeProcThreadAttributeList(v.as_mut_ptr() as _, sayi, 0, &mut boyut) == 0 {
                return Err(son_hata("InitializeProcThreadAttributeList"));
            }
            Ok(Self(v))
        }
    }

    fn ptr(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.0.as_mut_ptr() as _
    }

    /// # Safety
    /// `deger`, liste kullanıldığı sürece yaşamalıdır.
    unsafe fn ekle(
        &mut self,
        oznitelik: usize,
        deger: *const c_void,
        boyut: usize,
    ) -> io::Result<()> {
        // SAFETY: çağıran değerin ömrünü garanti eder.
        let tamam = unsafe {
            UpdateProcThreadAttribute(self.ptr(), 0, oznitelik, deger, boyut, null_mut(), null())
        };
        if tamam == 0 {
            return Err(son_hata("UpdateProcThreadAttribute"));
        }
        Ok(())
    }
}

impl Drop for Oznitelikler {
    fn drop(&mut self) {
        // SAFETY: liste InitializeProcThreadAttributeList ile başlatıldı.
        unsafe { DeleteProcThreadAttributeList(self.ptr()) };
    }
}

/// Komutu kapta başlatır ve işe atar. `is` düşünce ağaç ölür.
pub(crate) fn baslat(istek: &KapIstegi, is: &Is) -> io::Result<Kap> {
    baslat_ortak(istek, is, false)
}

/// Etkileşimli süreç: `baslat` gibi, stdin bir borudur (1.1a §4.1).
pub(crate) fn baslat_etkilesimli(istek: &KapIstegi, is: &Is) -> io::Result<Kap> {
    baslat_ortak(istek, is, true)
}

fn baslat_ortak(istek: &KapIstegi, is: &Is, borulu: bool) -> io::Result<Kap> {
    let (cikti_o, cikti_y) = boru()?;
    let (hata_o, hata_y) = boru()?;
    let (girdi, girdi_yazma) = if borulu {
        let (okuma, yazma) = girdi_borusu()?;
        (okuma, Some(yazma))
    } else {
        (nul()?, None)
    };

    let mut yetenekler: Vec<SID_AND_ATTRIBUTES> = istek
        .yetenekler
        .iter()
        .map(|s| SID_AND_ATTRIBUTES {
            Sid: s.ptr(),
            Attributes: SE_GROUP_ENABLED as u32,
        })
        .collect();
    let guvenlik = SECURITY_CAPABILITIES {
        AppContainerSid: istek.paket.ptr(),
        Capabilities: if yetenekler.is_empty() {
            null_mut()
        } else {
            yetenekler.as_mut_ptr()
        },
        CapabilityCount: yetenekler.len() as u32,
        Reserved: 0,
    };
    let tutamaclar: [HANDLE; 3] = [
        girdi.as_raw_handle() as HANDLE,
        cikti_y.as_raw_handle() as HANDLE,
        hata_y.as_raw_handle() as HANDLE,
    ];
    let mut liste = Oznitelikler::yeni(2)?;
    // SAFETY: değerler bu işlevin sonuna kadar yaşar; liste de öyle.
    unsafe {
        liste.ekle(
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
            (&raw const guvenlik).cast(),
            size_of::<SECURITY_CAPABILITIES>(),
        )?;
        liste.ekle(
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
            tutamaclar.as_ptr().cast(),
            size_of_val(&tutamaclar),
        )?;
    }

    // SAFETY: sıfırla başlatılan yapı; alanlar aşağıda doldurulur.
    let mut si: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    si.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    si.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    si.StartupInfo.hStdInput = tutamaclar[0];
    si.StartupInfo.hStdOutput = tutamaclar[1];
    si.StartupInfo.hStdError = tutamaclar[2];
    si.lpAttributeList = liste.ptr();
    // SAFETY: sıfırla başlatılan çıktı yapısı.
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let uygulama = genis(&istek.komut.uygulama);
    let mut satir = istek.komut.satir.clone();
    satir.push(0);
    let ortam = ortam_blogu(istek.ortam);
    let dizin = genis(istek.dizin);
    // SAFETY: bütün dizeler NUL ile biter ve çağrı boyunca yaşar; öznitelik
    // listesinin gösterdiği değerler de öyle.
    let tamam = unsafe {
        CreateProcessW(
            uygulama.as_ptr(),
            satir.as_mut_ptr(),
            null(),
            null(),
            1,
            EXTENDED_STARTUPINFO_PRESENT
                | CREATE_SUSPENDED
                | CREATE_UNICODE_ENVIRONMENT
                | CREATE_NO_WINDOW,
            ortam.as_ptr().cast(),
            dizin.as_ptr(),
            &si.StartupInfo,
            &mut pi,
        )
    };
    if tamam == 0 {
        // SAFETY: yalnızca son hatayı okur.
        let kod = unsafe { GetLastError() };
        let e = io::Error::from_raw_os_error(kod as i32);
        return Err(io::Error::new(e.kind(), format!("kap başlatılamadı: {e}")));
    }
    let surec = sahiplen(pi.hProcess);
    let is_parcacigi = sahiplen(pi.hThread);
    if let Err(e) = is.ata(surec.as_raw_handle()) {
        // SAFETY: süreç askıda başladı ve bu işleve aittir.
        unsafe { TerminateProcess(surec.as_raw_handle() as HANDLE, 1) };
        return Err(e);
    }
    // SAFETY: iş parçacığı askıda başladı ve bu işleve aittir.
    unsafe { ResumeThread(is_parcacigi.as_raw_handle() as HANDLE) };
    drop((girdi, cikti_y, hata_y, liste, is_parcacigi));
    Ok(Kap {
        surec,
        cikti: File::from(cikti_o),
        hata: File::from(hata_o),
        girdi: girdi_yazma.map(File::from),
    })
}

/// Süreç bitene kadar bekler ve çıkış kodunu döner.
pub(crate) fn bekle(surec: &OwnedHandle) -> Option<i32> {
    let h = surec.as_raw_handle() as HANDLE;
    // SAFETY: tutamaç geçerlidir; çıktı yerel bir değişkendir.
    unsafe {
        if WaitForSingleObject(h, INFINITE) != WAIT_OBJECT_0 {
            return None;
        }
        let mut kod = 0u32;
        if GetExitCodeProcess(h, &mut kod) == 0 {
            return None;
        }
        Some(kod as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ortam_blogu_sirali_ve_cift_nul_ile_biter() {
        let blok = ortam_blogu(&[
            ("b".into(), "2".into()),
            ("A".into(), "1".into()),
            ("PATH".into(), r"C:\x".into()),
        ]);
        let metin = String::from_utf16(&blok).unwrap();
        assert_eq!(metin, "A=1\0b=2\0PATH=C:\\x\0\0");
        assert_eq!(ortam_blogu(&[]), vec![0, 0]);
    }
}
