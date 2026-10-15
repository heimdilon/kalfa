//! AppContainer yardımcıları (2c ek prototipi): profil oluşturma ve silme,
//! komutu bir AppContainer'da Job Object içinde başlatma. Kısıtlı belirteç
//! yolundan farkı: ayrı kullanıcı, parola ve oturum yoktur; komut
//! geliştiricinin kendi hesabıyla, AppContainer belirteciyle çalışır.

use std::ffi::c_void;
use std::fs::File;
use std::io::Read;
use std::os::windows::io::FromRawHandle;
use std::ptr::{null, null_mut};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    GENERIC_READ, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeleteAppContainerProfile, DeriveAppContainerSidFromAppContainerName,
};
use windows_sys::Win32::Security::{
    FreeSid, PSID, SECURITY_ATTRIBUTES, SECURITY_CAPABILITIES, SID_AND_ATTRIBUTES,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::JobObjects::TerminateJobObject;
use windows_sys::Win32::System::SystemServices::SE_GROUP_ENABLED;
use windows_sys::Win32::System::Threading::{
    CREATE_NO_WINDOW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW,
    DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess,
    InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
    PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
    PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, PROCESS_INFORMATION, ResumeThread,
    STARTF_USESTDHANDLES, STARTUPINFOEXW, UpdateProcThreadAttribute, WaitForSingleObject,
};

use crate::win::{self, OturumSonucu, Sid, Sonuc, Tutamac};

/// `HRESULT_FROM_WIN32(ERROR_ALREADY_EXISTS)`.
const ZATEN_VAR: i32 = 0x8007_00B7_u32 as i32;
/// `PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT`: LPAC.
const AAP_DISI: u32 = 1;

/// AppContainer profilini oluşturur; varsa SID'ini addan türetir.
pub fn profil(ad: &str) -> Sonuc<Sid> {
    let mut p: PSID = null_mut();
    // SAFETY: dizeler NUL ile biter; SID FreeSid ile bırakılır.
    unsafe {
        let hr = CreateAppContainerProfile(
            win::genis(ad).as_ptr(),
            win::genis(ad).as_ptr(),
            win::genis("Kalfa 2c prototipi (geçici)").as_ptr(),
            null(),
            0,
            &mut p,
        );
        if hr == ZATEN_VAR {
            let hr = DeriveAppContainerSidFromAppContainerName(win::genis(ad).as_ptr(), &mut p);
            if hr != 0 {
                return Err(format!(
                    "DeriveAppContainerSidFromAppContainerName: {hr:#010x}"
                ));
            }
        } else if hr != 0 {
            return Err(format!("CreateAppContainerProfile: {hr:#010x}"));
        }
        let sid = Sid::kopyala(p);
        FreeSid(p);
        Ok(sid)
    }
}

pub fn profil_sil(ad: &str) -> Sonuc<()> {
    // SAFETY: dize NUL ile biter.
    let hr = unsafe { DeleteAppContainerProfile(win::genis(ad).as_ptr()) };
    if hr != 0 {
        return Err(format!("DeleteAppContainerProfile({ad}): {hr:#010x}"));
    }
    Ok(())
}

pub struct KapIstek<'a> {
    pub komut: &'a str,
    pub dizin: &'a str,
    pub ortam: &'a [(String, String)],
    /// AppContainer'ın (paketin) SID'i: çalışma alanı SID'inin karşılığı.
    pub paket: &'a Sid,
    pub yetenekler: &'a [Sid],
    /// ALL APPLICATION PACKAGES dışında kalan AppContainer (LPAC).
    pub lpac: bool,
    pub ui_kisit: bool,
}

/// Süreç öznitelik listesi; düşünce silinir.
pub(crate) struct Oznitelikler(Vec<u64>);

impl Oznitelikler {
    pub(crate) fn yeni(sayi: u32) -> Sonuc<Oznitelikler> {
        let mut boyut = 0usize;
        // SAFETY: önce boyut sorulur, sonra yeterli (8 bayt hizalı) tampon verilir.
        unsafe {
            InitializeProcThreadAttributeList(null_mut(), sayi, 0, &mut boyut);
            let mut v = vec![0u64; boyut.div_ceil(8)];
            if InitializeProcThreadAttributeList(v.as_mut_ptr() as _, sayi, 0, &mut boyut) == 0 {
                return Err(win::hata("InitializeProcThreadAttributeList"));
            }
            Ok(Oznitelikler(v))
        }
    }

    pub(crate) fn ptr(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.0.as_mut_ptr() as _
    }

    /// # Safety
    /// `deger` liste kullanıldığı sürece yaşamalıdır.
    pub(crate) unsafe fn ekle(&mut self, oznitelik: u32, deger: *const c_void, boyut: usize) -> Sonuc<()> {
        // SAFETY: çağıran değerin ömrünü garanti eder.
        let tamam = unsafe {
            UpdateProcThreadAttribute(
                self.ptr(),
                0,
                oznitelik as usize,
                deger,
                boyut,
                null_mut(),
                null(),
            )
        };
        if tamam == 0 {
            return Err(win::hata(&format!(
                "UpdateProcThreadAttribute({oznitelik:#x})"
            )));
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

/// Komutu AppContainer'da, kapanınca ağacı öldüren bir Job Object içinde
/// başlatır. Standart girdi NUL'dur; çocuğa yalnızca üç standart tutamaç
/// geçer (tutamaç listesi). İki çıktı toplanır.
pub fn kapta_calistir(i: &KapIstek, zaman_asimi: Duration) -> Sonuc<OturumSonucu> {
    let (cikti_o, cikti_y) = win::boru(false)?;
    let (hata_o, hata_y) = win::boru(false)?;
    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: 1,
    };
    // SAFETY: tutamaç Tutamac'a geçer.
    let nul = Tutamac(unsafe {
        CreateFileW(
            win::genis("NUL").as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            &sa,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            null_mut(),
        )
    });
    if nul.0 == INVALID_HANDLE_VALUE {
        return Err(win::hata("CreateFileW(NUL)"));
    }

    let mut yetenekler: Vec<SID_AND_ATTRIBUTES> = i
        .yetenekler
        .iter()
        .map(|s| SID_AND_ATTRIBUTES {
            Sid: s.ptr(),
            Attributes: SE_GROUP_ENABLED as u32,
        })
        .collect();
    let yetenek_bilgisi = SECURITY_CAPABILITIES {
        AppContainerSid: i.paket.ptr(),
        Capabilities: if yetenekler.is_empty() {
            null_mut()
        } else {
            yetenekler.as_mut_ptr()
        },
        CapabilityCount: yetenekler.len() as u32,
        Reserved: 0,
    };
    let tutamaclar: [HANDLE; 3] = [nul.0, cikti_y.0, hata_y.0];
    let aap_disi = AAP_DISI;
    let mut liste = Oznitelikler::yeni(if i.lpac { 3 } else { 2 })?;
    // SAFETY: değerler bu işlevin sonuna dek yaşar; liste de öyle.
    unsafe {
        liste.ekle(
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
            &yetenek_bilgisi as *const _ as *const c_void,
            size_of::<SECURITY_CAPABILITIES>(),
        )?;
        liste.ekle(
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
            tutamaclar.as_ptr() as *const c_void,
            size_of_val(&tutamaclar),
        )?;
        if i.lpac {
            liste.ekle(
                PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY,
                &aap_disi as *const _ as *const c_void,
                size_of::<u32>(),
            )?;
        }
    }

    let is = win::is_nesnesi(true, i.ui_kisit)?;
    let mut si: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    si.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    si.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    si.StartupInfo.hStdInput = nul.0;
    si.StartupInfo.hStdOutput = cikti_y.0;
    si.StartupInfo.hStdError = hata_y.0;
    si.lpAttributeList = liste.ptr();
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let mut satir = win::genis(i.komut);
    let ortam = win::ortam_blogu(i.ortam);
    let bas = Instant::now();
    // SAFETY: bütün dizeler NUL ile biter ve çağrı boyunca yaşar.
    let tamam = unsafe {
        CreateProcessW(
            null(),
            satir.as_mut_ptr(),
            null(),
            null(),
            1,
            EXTENDED_STARTUPINFO_PRESENT
                | CREATE_SUSPENDED
                | CREATE_UNICODE_ENVIRONMENT
                | CREATE_NO_WINDOW,
            ortam.as_ptr() as *const _,
            win::genis(i.dizin).as_ptr(),
            &si.StartupInfo,
            &mut pi,
        )
    };
    if tamam == 0 {
        return Err(win::hata("CreateProcessW(AppContainer)"));
    }
    let surec = Tutamac(pi.hProcess);
    let is_parcacigi = Tutamac(pi.hThread);
    let ise_atama = win::ise_ata(is.0, surec.0);
    if let Err(e) = &ise_atama {
        // SAFETY: süreç askıda başladı ve bu işleve aittir.
        unsafe { windows_sys::Win32::System::Threading::TerminateProcess(surec.0, 1) };
        return Err(e.clone());
    }
    // SAFETY: iş parçacığı askıda başladı.
    unsafe { ResumeThread(is_parcacigi.0) };
    drop((nul, cikti_y, hata_y, liste));

    // SAFETY: okuma uçları bu işleve aittir; File'a devredilir.
    let (mut oku_c, mut oku_h) = unsafe {
        let c = File::from_raw_handle(cikti_o.0 as _);
        let h = File::from_raw_handle(hata_o.0 as _);
        std::mem::forget((cikti_o, hata_o));
        (c, h)
    };
    let c = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = oku_c.read_to_end(&mut v);
        v
    });
    let h = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = oku_h.read_to_end(&mut v);
        v
    });
    let ms = zaman_asimi.as_millis().min(u32::MAX as u128) as u32;
    // SAFETY: süreç ve iş tutamaçları geçerlidir.
    let cikis = unsafe {
        match WaitForSingleObject(surec.0, ms) {
            WAIT_OBJECT_0 => {
                let mut kod = 0u32;
                GetExitCodeProcess(surec.0, &mut kod);
                Some(kod)
            }
            WAIT_TIMEOUT => None,
            _ => None,
        }
    };
    let sure = bas.elapsed();
    // Ağaçta kalan torunlar da ölür; borular kapanır.
    // SAFETY: iş tutamacı geçerlidir.
    unsafe { TerminateJobObject(is.0, 1) };
    drop(is);
    let standart = String::from_utf8_lossy(&c.join().unwrap_or_default()).into_owned();
    let hata = String::from_utf8_lossy(&h.join().unwrap_or_default()).into_owned();
    Ok(OturumSonucu {
        cikis,
        standart,
        hata,
        sure,
        ise_atama,
    })
}
