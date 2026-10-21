//! Görev 2 (Windows): AppContainer + Job Object içinde borulu süreç ve ConPTY
//! (`CreatePseudoConsole`, `ResizePseudoConsole`, `ClosePseudoConsole`) deneyleri.

use std::ffi::c_void;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::windows::io::FromRawHandle;
use std::path::Path;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_SUCCESS, GetLastError, HANDLE, HLOCAL, INVALID_HANDLE_VALUE, LocalFree,
    WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSidToSidW, EXPLICIT_ACCESS_W, GRANT_ACCESS,
    NO_MULTIPLE_TRUSTEE, SE_FILE_OBJECT, SetEntriesInAclW, SetNamedSecurityInfoW, TRUSTEE_IS_SID,
    TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
};
use windows_sys::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeleteAppContainerProfile, DeriveAppContainerSidFromAppContainerName,
};
use windows_sys::Win32::Security::{
    ACL, DACL_SECURITY_INFORMATION, FreeSid, GetLengthSid, GetTokenInformation,
    PROTECTED_DACL_SECURITY_INFORMATION, PSID, SECURITY_ATTRIBUTES, SECURITY_CAPABILITIES,
    SUB_CONTAINERS_AND_OBJECTS_INHERIT, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows_sys::Win32::System::Console::{
    CONSOLE_SCREEN_BUFFER_INFO, COORD, CTRL_C_EVENT, ClosePseudoConsole, CreatePseudoConsole,
    GetConsoleScreenBufferInfo, GetStdHandle, HPCON, ResizePseudoConsole, STD_OUTPUT_HANDLE,
    SetConsoleCtrlHandler,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
    JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOB_OBJECT_UILIMIT_DESKTOP, JOB_OBJECT_UILIMIT_DISPLAYSETTINGS, JOB_OBJECT_UILIMIT_EXITWINDOWS,
    JOB_OBJECT_UILIMIT_GLOBALATOMS, JOB_OBJECT_UILIMIT_HANDLES, JOB_OBJECT_UILIMIT_READCLIPBOARD,
    JOB_OBJECT_UILIMIT_SYSTEMPARAMETERS, JOB_OBJECT_UILIMIT_WRITECLIPBOARD,
    JOBOBJECT_BASIC_UI_RESTRICTIONS, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectBasicUIRestrictions, JobObjectExtendedLimitInformation, SetInformationJobObject,
    TerminateJobObject,
};
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::Threading::{
    CREATE_NO_WINDOW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW,
    DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT, GetCurrentProcess,
    GetExitCodeProcess, InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
    OpenProcess, OpenProcessToken, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
    PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
    PROCESS_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, ResumeThread,
    STARTF_USESTDHANDLES, STARTUPINFOEXW, UpdateProcThreadAttribute, WaitForSingleObject,
};

use crate::kilit_deneyi::{SatirRaporu, Sonuc};

const ZATEN_VAR: i32 = 0x8007_00B7_u32 as i32;
const TAM: u32 = 0x001F_01FF;
const DEGISTIR: u32 = 0x0013_01BF;
const OKU_YURUT: u32 = 0x0012_00A9;
pub const AZAMI_IS_CIKTISI_BAYT: usize = 1024 * 1024; // 1 MiB

static CTRL_C_ALINDI: AtomicBool = AtomicBool::new(false);

unsafe extern "system" fn ctrl_c_yakala(olay: u32) -> i32 {
    if olay == CTRL_C_EVENT {
        CTRL_C_ALINDI.store(true, Ordering::SeqCst);
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------- çocuk alt komutları

pub fn cocuk_komutu(alt: &str) -> i32 {
    match alt {
        "yanki" => cocuk_yanki(),
        "ctrlc" => cocuk_ctrlc(),
        "pty-boyut" => cocuk_pty_boyut(),
        "torun" => cocuk_torun(),
        "uyku" => loop {
            std::thread::sleep(Duration::from_secs(3600));
        },
        "sel" => cocuk_sel(),
        _ => 2,
    }
}

fn cocuk_yanki() -> i32 {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    let mut satir = String::new();
    let mut r = stdin.lock();
    loop {
        satir.clear();
        match r.read_line(&mut satir) {
            Ok(0) => return 0,
            Ok(_) => {
                let _ = out.write_all(satir.as_bytes());
                let _ = out.flush();
            }
            Err(_) => return 3,
        }
    }
}

fn cocuk_ctrlc() -> i32 {
    // SAFETY: geçerli konsol sinyali işleyicisi kaydedilir.
    unsafe {
        SetConsoleCtrlHandler(Some(ctrl_c_yakala), 1);
    }
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "HAZIR pid={}", std::process::id());
    let _ = out.flush();
    let bas = Instant::now();
    while bas.elapsed() < Duration::from_secs(15) {
        if CTRL_C_ALINDI.load(Ordering::SeqCst) {
            let _ = writeln!(out, "CTRL_C_ALINDI");
            let _ = out.flush();
            return 42;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    1
}

fn konsol_boyutu() -> (i16, i16) {
    // SAFETY: standart çıktı konsol tutamacı sorgulanır.
    unsafe {
        let h = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut csbi: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
        if GetConsoleScreenBufferInfo(h, &mut csbi) != 0 {
            let genislik = csbi.srWindow.Right - csbi.srWindow.Left + 1;
            let yukseklik = csbi.srWindow.Bottom - csbi.srWindow.Top + 1;
            (genislik, yukseklik)
        } else {
            (0, 0)
        }
    }
}

fn cocuk_pty_boyut() -> i32 {
    let mut out = std::io::stdout().lock();
    let (g, y) = konsol_boyutu();
    let _ = writeln!(out, "\x1b[32mPTY_ACILDI\x1b[0m utf8=çğıöşü boyut={g}x{y}");
    let _ = out.flush();

    let stdin = std::io::stdin();
    let mut r = stdin.lock();
    let mut satir = String::new();
    while r.read_line(&mut satir).map(|n| n > 0).unwrap_or(false) {
        let komut = satir.trim();
        if komut == "boyut" {
            let (g2, y2) = konsol_boyutu();
            let _ = writeln!(out, "YENI_BOYUT={g2}x{y2}");
            let _ = out.flush();
        } else if komut == "cik" {
            return 0;
        }
        satir.clear();
    }
    0
}

fn cocuk_torun() -> i32 {
    let exe = std::env::current_exe().unwrap_or_default();
    let torun = std::process::Command::new(exe)
        .args(["cocuk", "uyku"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    let mut out = std::io::stdout().lock();
    match &torun {
        Ok(t) => {
            let _ = writeln!(out, "PIDLER {} {}", std::process::id(), t.id());
        }
        Err(e) => {
            let _ = writeln!(out, "TORUN_HATA {e}");
            return 2;
        }
    }
    let _ = out.flush();
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}

fn cocuk_sel() -> i32 {
    let exe = std::env::current_exe().unwrap_or_default();
    let torun = std::process::Command::new(exe)
        .args(["cocuk", "uyku"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    let torun_pid = torun.as_ref().map(|t| t.id()).unwrap_or(0);
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "SEL_PIDLER {} {}", std::process::id(), torun_pid);
    let _ = out.flush();
    let blok = format!("{}\n", "D".repeat(4096));
    for _ in 0..512 {
        if out.write_all(blok.as_bytes()).is_err() {
            break;
        }
    }
    let _ = out.flush();
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}

// ---------------------------------------------------------------- Win32 yardımcıları

struct Tutamac(HANDLE);
unsafe impl Send for Tutamac {}
unsafe impl Sync for Tutamac {}

impl Drop for Tutamac {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            // SAFETY: geçerli tutamaç kapatılır.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

struct Sid(Vec<u8>);

impl Sid {
    fn metinden(s: &str) -> Sonuc<Sid> {
        let mut p: PSID = null_mut();
        // SAFETY: NUL sonlu UTF-16 dize; çıktı LocalFree ile bırakılır.
        unsafe {
            if ConvertStringSidToSidW(genis(s).as_ptr(), &mut p) == 0 {
                return Err(win_hata(&format!("ConvertStringSidToSidW({s})")));
            }
            let sid = Sid::kopyala(p);
            LocalFree(p as HLOCAL);
            Ok(sid)
        }
    }

    unsafe fn kopyala(p: PSID) -> Sid {
        // SAFETY: çağıran geçerli SID verir.
        unsafe {
            let n = GetLengthSid(p) as usize;
            Sid(std::slice::from_raw_parts(p as *const u8, n).to_vec())
        }
    }

    fn ptr(&self) -> PSID {
        self.0.as_ptr() as PSID
    }

    #[allow(dead_code)]
    fn metin(&self) -> String {
        let mut p: *mut u16 = null_mut();
        // SAFETY: SID geçerlidir; çıktı LocalFree ile bırakılır.
        unsafe {
            if ConvertSidToStringSidW(self.ptr(), &mut p) == 0 {
                return "?".into();
            }
            let mut n = 0usize;
            while *p.add(n) != 0 {
                n += 1;
            }
            let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, n));
            LocalFree(p as HLOCAL);
            s
        }
    }
}

fn win_hata(adim: &str) -> String {
    // SAFETY: iş parçacığı son hata kodu okunur.
    format!("{adim}: Win32 hatası {}", unsafe { GetLastError() })
}

fn genis(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn ortam_blogu(ciftler: &[(String, String)]) -> Vec<u16> {
    let mut b = Vec::new();
    for (k, v) in ciftler {
        b.extend(format!("{k}={v}").encode_utf16());
        b.push(0);
    }
    b.push(0);
    b
}

fn kendi_kullanici_sidi() -> Sonuc<Sid> {
    let mut token: HANDLE = null_mut();
    // SAFETY: geçerli sürecin belirteci açılır.
    unsafe {
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(win_hata("OpenProcessToken"));
        }
        let _t = Tutamac(token);
        let mut n = 0u32;
        GetTokenInformation(token, TokenUser, null_mut(), 0, &mut n);
        let mut buf = vec![0u8; n as usize];
        if GetTokenInformation(token, TokenUser, buf.as_mut_ptr() as _, n, &mut n) == 0 {
            return Err(win_hata("GetTokenInformation(TokenUser)"));
        }
        let tu = &*(buf.as_ptr() as *const TOKEN_USER);
        Ok(Sid::kopyala(tu.User.Sid))
    }
}

fn profil_olustur(ad: &str) -> Sonuc<Sid> {
    let mut p: PSID = null_mut();
    // SAFETY: dizeler NUL sonludur; dönen SID FreeSid ile bırakılır.
    unsafe {
        let hr = CreateAppContainerProfile(
            genis(ad).as_ptr(),
            genis(ad).as_ptr(),
            genis("Kalfa G2a süreç/terminal prototipi (geçici)").as_ptr(),
            null(),
            0,
            &mut p,
        );
        if hr == ZATEN_VAR {
            let hr = DeriveAppContainerSidFromAppContainerName(genis(ad).as_ptr(), &mut p);
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

fn profil_sil(ad: &str) -> Sonuc<()> {
    // SAFETY: NUL sonlu UTF-16 dize.
    let hr = unsafe { DeleteAppContainerProfile(genis(ad).as_ptr()) };
    if hr != 0 {
        return Err(format!("DeleteAppContainerProfile({ad}): {hr:#010x}"));
    }
    Ok(())
}

struct Erisim<'a> {
    sid: &'a Sid,
    maske: u32,
}

fn dacl_yaz(yol: &Path, erisimler: &[Erisim]) -> Sonuc<()> {
    let girdiler: Vec<EXPLICIT_ACCESS_W> = erisimler
        .iter()
        .map(|e| EXPLICIT_ACCESS_W {
            grfAccessPermissions: e.maske,
            grfAccessMode: GRANT_ACCESS,
            grfInheritance: SUB_CONTAINERS_AND_OBJECTS_INHERIT,
            Trustee: TRUSTEE_W {
                pMultipleTrustee: null_mut(),
                MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                TrusteeForm: TRUSTEE_IS_SID,
                TrusteeType: TRUSTEE_IS_UNKNOWN,
                ptstrName: e.sid.ptr() as *mut u16,
            },
        })
        .collect();
    let mut acl: *mut ACL = null_mut();
    // SAFETY: girdiler çağrı boyunca yaşar; ACL LocalFree ile bırakılır.
    unsafe {
        let r = SetEntriesInAclW(girdiler.len() as u32, girdiler.as_ptr(), null(), &mut acl);
        if r != ERROR_SUCCESS {
            return Err(format!("SetEntriesInAclW: {r}"));
        }
        let ad = genis(&yol.display().to_string());
        let r = SetNamedSecurityInfoW(
            ad.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            acl,
            null(),
        );
        LocalFree(acl as HLOCAL);
        if r != ERROR_SUCCESS {
            return Err(format!("SetNamedSecurityInfoW({}): {r}", yol.display()));
        }
    }
    Ok(())
}

fn boru(okuma_kalitsin: bool) -> Sonuc<(Tutamac, Tutamac)> {
    let mut oku: HANDLE = null_mut();
    let mut yaz: HANDLE = null_mut();
    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: 1,
    };
    // SAFETY: geçerli SECURITY_ATTRIBUTES ile anonim boru açılır.
    unsafe {
        if CreatePipe(&mut oku, &mut yaz, &sa, 0) == 0 {
            return Err(win_hata("CreatePipe"));
        }
        let kalitilmayacak = if okuma_kalitsin { yaz } else { oku };
        windows_sys::Win32::Foundation::SetHandleInformation(
            kalitilmayacak,
            windows_sys::Win32::Foundation::HANDLE_FLAG_INHERIT,
            0,
        );
    }
    Ok((Tutamac(oku), Tutamac(yaz)))
}

fn is_nesnesi_olustur() -> Sonuc<Tutamac> {
    // SAFETY: yeni Job Object oluşturulur ve süreç/UI sınırları atanır.
    unsafe {
        let is = CreateJobObjectW(null(), null());
        if is.is_null() {
            return Err(win_hata("CreateJobObjectW"));
        }
        let is = Tutamac(is);
        let mut bilgi: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        bilgi.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
            | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
        bilgi.BasicLimitInformation.ActiveProcessLimit = 256;
        if SetInformationJobObject(
            is.0,
            JobObjectExtendedLimitInformation,
            &bilgi as *const _ as *const c_void,
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        ) == 0
        {
            return Err(win_hata("SetInformationJobObject(ExtendedLimit)"));
        }
        let ui = JOBOBJECT_BASIC_UI_RESTRICTIONS {
            UIRestrictionsClass: JOB_OBJECT_UILIMIT_HANDLES
                | JOB_OBJECT_UILIMIT_READCLIPBOARD
                | JOB_OBJECT_UILIMIT_WRITECLIPBOARD
                | JOB_OBJECT_UILIMIT_SYSTEMPARAMETERS
                | JOB_OBJECT_UILIMIT_DISPLAYSETTINGS
                | JOB_OBJECT_UILIMIT_GLOBALATOMS
                | JOB_OBJECT_UILIMIT_DESKTOP
                | JOB_OBJECT_UILIMIT_EXITWINDOWS,
        };
        if SetInformationJobObject(
            is.0,
            JobObjectBasicUIRestrictions,
            &ui as *const _ as *const c_void,
            size_of::<JOBOBJECT_BASIC_UI_RESTRICTIONS>() as u32,
        ) == 0
        {
            return Err(win_hata("SetInformationJobObject(BasicUIRestrictions)"));
        }
        Ok(is)
    }
}

struct Oznitelikler(Vec<u64>);

impl Oznitelikler {
    fn yeni(sayi: u32) -> Sonuc<Oznitelikler> {
        let mut boyut = 0usize;
        // SAFETY: önce boyut sorgulanır, sonra 8 bayt hizalı tampon başlatılır.
        unsafe {
            InitializeProcThreadAttributeList(null_mut(), sayi, 0, &mut boyut);
            let mut v = vec![0u64; boyut.div_ceil(8)];
            if InitializeProcThreadAttributeList(v.as_mut_ptr() as _, sayi, 0, &mut boyut) == 0 {
                return Err(win_hata("InitializeProcThreadAttributeList"));
            }
            Ok(Oznitelikler(v))
        }
    }

    fn ptr(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.0.as_mut_ptr() as _
    }

    unsafe fn ekle(&mut self, oznitelik: u32, deger: *const c_void, boyut: usize) -> Sonuc<()> {
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
            return Err(win_hata(&format!(
                "UpdateProcThreadAttribute({oznitelik:#x})"
            )));
        }
        Ok(())
    }
}

impl Drop for Oznitelikler {
    fn drop(&mut self) {
        // SAFETY: InitializeProcThreadAttributeList ile başlatıldı.
        unsafe { DeleteProcThreadAttributeList(self.ptr()) };
    }
}

fn standart_ortam(dizin: &Path) -> Vec<u16> {
    let sistem = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    let d = dizin.display().to_string();
    ortam_blogu(&[
        ("SystemRoot".into(), sistem.clone()),
        ("windir".into(), sistem.clone()),
        ("SystemDrive".into(), "C:".into()),
        ("PATH".into(), format!(r"{sistem}\System32")),
        ("TEMP".into(), d.clone()),
        ("TMP".into(), d.clone()),
        ("USERPROFILE".into(), d.clone()),
        ("HOME".into(), d.clone()),
        ("APPDATA".into(), d.clone()),
        ("LOCALAPPDATA".into(), d),
        ("TERM".into(), "xterm-256color".into()),
    ])
}

struct BoruluKapSureci {
    surec: Tutamac,
    #[allow(dead_code)]
    is: Tutamac,
    stdin: Option<File>,
    stdout: Option<File>,
}

fn kapta_borulu_baslat(paket: &Sid, komut: &str, dizin: &Path) -> Sonuc<BoruluKapSureci> {
    let (girdi_o, girdi_y) = boru(true)?;
    let (cikti_o, cikti_y) = boru(false)?;
    let (hata_o, hata_y) = boru(false)?;
    let bilgi = SECURITY_CAPABILITIES {
        AppContainerSid: paket.ptr(),
        Capabilities: null_mut(),
        CapabilityCount: 0,
        Reserved: 0,
    };
    let tutamaclar: [HANDLE; 3] = [girdi_o.0, cikti_y.0, hata_y.0];
    let mut liste = Oznitelikler::yeni(2)?;
    // SAFETY: bilgi ve tutamaclar CreateProcessW bitene kadar yaşar.
    unsafe {
        liste.ekle(
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
            &bilgi as *const _ as *const c_void,
            size_of::<SECURITY_CAPABILITIES>(),
        )?;
        liste.ekle(
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
            tutamaclar.as_ptr() as *const c_void,
            size_of_val(&tutamaclar),
        )?;
    }
    let is = is_nesnesi_olustur()?;
    let mut si: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    si.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    si.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    si.StartupInfo.hStdInput = girdi_o.0;
    si.StartupInfo.hStdOutput = cikti_y.0;
    si.StartupInfo.hStdError = hata_y.0;
    si.lpAttributeList = liste.ptr();

    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let mut satir = genis(komut);
    let ortam = standart_ortam(dizin);
    let cwd = genis(&dizin.display().to_string());
    // SAFETY: geçerli yapılarla CreateProcessW çağrılır.
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
            cwd.as_ptr(),
            &si.StartupInfo,
            &mut pi,
        )
    };
    if tamam == 0 {
        return Err(win_hata("CreateProcessW(AppContainer, borulu)"));
    }
    let surec = Tutamac(pi.hProcess);
    let ip = Tutamac(pi.hThread);
    // SAFETY: askıdaki süreç Job Object'e atanıp devam ettirilir.
    unsafe {
        if AssignProcessToJobObject(is.0, surec.0) == 0 {
            let _ = TerminateJobObject(is.0, 1);
            return Err(win_hata("AssignProcessToJobObject"));
        }
        ResumeThread(ip.0);
    }
    drop((girdi_o, cikti_y, hata_y, hata_o, liste));
    // SAFETY: tutamaç sahipliği File'a devredilir.
    let (stdin, stdout) = unsafe {
        let g = File::from_raw_handle(girdi_y.0 as _);
        let c = File::from_raw_handle(cikti_o.0 as _);
        std::mem::forget((girdi_y, cikti_o));
        (g, c)
    };
    Ok(BoruluKapSureci {
        surec,
        is,
        stdin: Some(stdin),
        stdout: Some(stdout),
    })
}

struct ConPtyTutamaci(HPCON);
unsafe impl Send for ConPtyTutamaci {}
unsafe impl Sync for ConPtyTutamaci {}

impl Drop for ConPtyTutamaci {
    fn drop(&mut self) {
        if self.0 != 0 {
            // SAFETY: geçerli HPCON kapatılır.
            unsafe {
                ClosePseudoConsole(self.0);
            }
            self.0 = 0;
        }
    }
}

struct ConPtyKapSureci {
    surec: Tutamac,
    is: Tutamac,
    hpc: ConPtyTutamaci,
    girdi: Option<File>,
    cikti: Option<File>,
}

impl ConPtyKapSureci {
    fn boyut_degistir(&self, sutun: i16, satir: i16) -> Sonuc<()> {
        // SAFETY: geçerli HPCON üzerinde yeni konsol boyutu ayarlanır.
        let hr = unsafe { ResizePseudoConsole(self.hpc.0, COORD { X: sutun, Y: satir }) };
        if hr != 0 {
            return Err(format!("ResizePseudoConsole({sutun}x{satir}): {hr:#010x}"));
        }
        Ok(())
    }
}

fn kapta_conpty_baslat(
    paket: &Sid,
    komut: &str,
    dizin: &Path,
    sutun: i16,
    satir: i16,
) -> Sonuc<ConPtyKapSureci> {
    let (pty_girdi_oku, pty_girdi_yaz) = boru(true)?;
    let (pty_cikti_oku, pty_cikti_yaz) = boru(false)?;

    let mut hpc: HPCON = 0;
    // SAFETY: boru uçları CreatePseudoConsole'a verilir.
    let hr = unsafe {
        CreatePseudoConsole(
            COORD { X: sutun, Y: satir },
            pty_girdi_oku.0,
            pty_cikti_yaz.0,
            0,
            &mut hpc,
        )
    };
    if hr != 0 {
        return Err(format!("CreatePseudoConsole: {hr:#010x}"));
    }
    let hpc = ConPtyTutamaci(hpc);
    // conhost kendi kopyalarını aldığı için çocuk tarafındaki boru uçları kapatılabilir.
    drop((pty_girdi_oku, pty_cikti_yaz));

    let bilgi = SECURITY_CAPABILITIES {
        AppContainerSid: paket.ptr(),
        Capabilities: null_mut(),
        CapabilityCount: 0,
        Reserved: 0,
    };
    let mut liste = Oznitelikler::yeni(2)?;
    // SAFETY: bilgi ve hpc CreateProcessW bitene kadar yaşar.
    unsafe {
        liste.ekle(
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
            &bilgi as *const _ as *const c_void,
            size_of::<SECURITY_CAPABILITIES>(),
        )?;
        liste.ekle(
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
            hpc.0 as *const c_void,
            size_of::<HPCON>(),
        )?;
    }

    let is = is_nesnesi_olustur()?;
    let mut si: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    si.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    si.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    si.StartupInfo.hStdInput = null_mut();
    si.StartupInfo.hStdOutput = null_mut();
    si.StartupInfo.hStdError = null_mut();
    si.lpAttributeList = liste.ptr();

    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let mut komut_satiri = genis(komut);
    let ortam = standart_ortam(dizin);
    let cwd = genis(&dizin.display().to_string());
    // SAFETY: ConPTY + AppContainer öznitelikleriyle süreç başlatılır.
    let tamam = unsafe {
        CreateProcessW(
            null(),
            komut_satiri.as_mut_ptr(),
            null(),
            null(),
            0,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
            ortam.as_ptr() as *const _,
            cwd.as_ptr(),
            &si.StartupInfo,
            &mut pi,
        )
    };
    if tamam == 0 {
        return Err(win_hata("CreateProcessW(AppContainer + ConPTY)"));
    }
    let surec = Tutamac(pi.hProcess);
    let ip = Tutamac(pi.hThread);
    // SAFETY: askıdaki süreç Job Object'e atanıp devam ettirilir.
    unsafe {
        if AssignProcessToJobObject(is.0, surec.0) == 0 {
            let _ = TerminateJobObject(is.0, 1);
            return Err(win_hata("AssignProcessToJobObject(ConPTY)"));
        }
        ResumeThread(ip.0);
    }
    drop(liste);
    // SAFETY: boru uçları File'a devredilir.
    let (girdi, cikti) = unsafe {
        let g = File::from_raw_handle(pty_girdi_yaz.0 as _);
        let c = File::from_raw_handle(pty_cikti_oku.0 as _);
        std::mem::forget((pty_girdi_yaz, pty_cikti_oku));
        (g, c)
    };
    Ok(ConPtyKapSureci {
        surec,
        is,
        hpc,
        girdi: Some(girdi),
        cikti: Some(cikti),
    })
}

fn bekle(h: HANDLE, ms: u32) -> Option<u32> {
    // SAFETY: geçerli süreç tutamacı beklenir.
    unsafe {
        if WaitForSingleObject(h, ms) != WAIT_OBJECT_0 {
            return None;
        }
        let mut kod = 0u32;
        GetExitCodeProcess(h, &mut kod);
        Some(kod)
    }
}

fn yasiyor(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    // SAFETY: süreç durumu sorgulanır ve tutamaç kapatılır.
    unsafe {
        let h = OpenProcess(
            PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid,
        );
        if h.is_null() {
            return false;
        }
        let t = Tutamac(h);
        WaitForSingleObject(t.0, 0) == WAIT_TIMEOUT
    }
}

pub fn calistir() -> Sonuc<Vec<SatirRaporu>> {
    let mut satirlar = Vec::new();
    let pid = std::process::id();
    let kok = std::env::temp_dir().join(format!("kalfa-g2a-{pid}"));
    let profil_adi = format!("kalfa-g2a-{pid}-kap");

    let sonuc = (|| -> Sonuc<()> {
        let _ = std::fs::remove_dir_all(&kok);
        let bin = kok.join("bin");
        let alan = kok.join("alan");
        std::fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&alan).map_err(|e| e.to_string())?;

        let exe = bin.join("kalfa-yetenek-surec-prototip.exe");
        std::fs::copy(std::env::current_exe().map_err(|e| e.to_string())?, &exe)
            .map_err(|e| format!("exe kopyası: {e}"))?;

        let paket = profil_olustur(&profil_adi)?;
        let ben = kendi_kullanici_sidi()?;
        let sistem = Sid::metinden("S-1-5-18")?;

        dacl_yaz(
            &kok,
            &[
                Erisim {
                    sid: &ben,
                    maske: TAM,
                },
                Erisim {
                    sid: &sistem,
                    maske: TAM,
                },
                Erisim {
                    sid: &paket,
                    maske: OKU_YURUT,
                },
            ],
        )?;
        dacl_yaz(
            &alan,
            &[
                Erisim {
                    sid: &ben,
                    maske: TAM,
                },
                Erisim {
                    sid: &sistem,
                    maske: TAM,
                },
                Erisim {
                    sid: &paket,
                    maske: DEGISTIR,
                },
            ],
        )?;

        let exe_s = exe.display().to_string();

        // ---------------------------------------------------------------------
        // G2.1: AppContainer + Job Object içinde borulu stdin/stdout ve imleçli okuma
        // ---------------------------------------------------------------------
        {
            let bas = Instant::now();
            let mut k = kapta_borulu_baslat(&paket, &format!("\"{exe_s}\" cocuk yanki"), &alan)?;
            let mut girdi = k.stdin.take().ok_or("stdin yok")?;
            let cikti = k.stdout.take().ok_or("stdout yok")?;
            let okuyucu = std::thread::spawn(move || {
                let mut r = BufReader::new(cikti);
                let mut buf = Vec::new();
                let _ = r.read_to_end(&mut buf);
                buf
            });
            let mut beklenen = Vec::new();
            for i in 0..200 {
                let s = format!("satir-{i}-çğıöşü\n");
                girdi.write_all(s.as_bytes()).map_err(|e| e.to_string())?;
                beklenen.extend_from_slice(s.as_bytes());
            }
            let buyuk = format!("{}\n", "B".repeat(256 * 1024));
            girdi
                .write_all(buyuk.as_bytes())
                .map_err(|e| e.to_string())?;
            beklenen.extend_from_slice(buyuk.as_bytes());
            drop(girdi); // EOF

            let alinan = okuyucu.join().map_err(|_| "okuyucu çöktü")?;
            let cikis = bekle(k.surec.0, 5000);
            let birebir = alinan == beklenen;
            satirlar.push(SatirRaporu {
                ad: "G2.1 Windows AppContainer+Job borulu stdin/stdout ve EOF".into(),
                gecti: birebir && cikis == Some(0),
                ayrinti: format!(
                    "{} bayt (200 UTF-8 satırı + 256 KiB satır) birebir={birebir}, EOF çıkış kodu={cikis:?}, süre={} ms",
                    alinan.len(),
                    bas.elapsed().as_millis()
                ),
            });
        }

        // ---------------------------------------------------------------------
        // G2.2: AppContainer + Job Object + ConPTY içinde `\x03` (`Ctrl+C`)
        // ---------------------------------------------------------------------
        {
            let bas = Instant::now();
            let mut k =
                kapta_conpty_baslat(&paket, &format!("\"{exe_s}\" cocuk ctrlc"), &alan, 80, 24)?;
            let mut girdi = k.girdi.take().ok_or("ConPTY girdi yok")?;
            let cikti = k.cikti.take().ok_or("ConPTY çıktı yok")?;
            let hazir_bildirim = Arc::new(AtomicBool::new(false));
            let ctrlc_bildirim = Arc::new(AtomicBool::new(false));
            let hazir_kopya = Arc::clone(&hazir_bildirim);
            let ctrlc_kopya = Arc::clone(&ctrlc_bildirim);

            let okuyucu = std::thread::spawn(move || {
                let mut r = BufReader::new(cikti);
                let mut toplanan = String::new();
                let mut buf = [0u8; 1024];
                while let Ok(n) = r.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    toplanan.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if toplanan.contains("HAZIR") {
                        hazir_kopya.store(true, Ordering::SeqCst);
                    }
                    if toplanan.contains("CTRL_C_ALINDI") {
                        ctrlc_kopya.store(true, Ordering::SeqCst);
                        break;
                    }
                }
                toplanan
            });

            let bekle_bas = Instant::now();
            while !hazir_bildirim.load(Ordering::SeqCst)
                && bekle_bas.elapsed() < Duration::from_secs(5)
            {
                std::thread::sleep(Duration::from_millis(15));
            }
            // ConPTY girdi borusuna `\x03` (ETX / Ctrl+C) gönder.
            girdi.write_all(&[0x03]).map_err(|e| e.to_string())?;
            girdi.flush().map_err(|e| e.to_string())?;

            let cikis = bekle(k.surec.0, 5000);
            drop(k.hpc);
            drop(girdi);
            let metin = okuyucu.join().map_err(|_| "ConPTY okuyucu çöktü")?;
            let gecti = ctrlc_bildirim.load(Ordering::SeqCst) && cikis == Some(42);
            satirlar.push(SatirRaporu {
                ad: "G2.2 Windows AppContainer+ConPTY Ctrl+C (`\\x03`) sinyali".into(),
                gecti,
                ayrinti: format!(
                    "ConPTY üzerinden `\\x03` gönderildi; `CTRL_C_EVENT` yakalandı={}, çıkış kodu={cikis:?}, süre={} ms (çıktı özeti: {:?})",
                    ctrlc_bildirim.load(Ordering::SeqCst),
                    bas.elapsed().as_millis(),
                    metin.lines().collect::<Vec<_>>()
                ),
            });
        }

        // ---------------------------------------------------------------------
        // G2.3: AppContainer + Job Object + ConPTY açılışı, UTF-8/ANSI ve Resize
        // ---------------------------------------------------------------------
        {
            let bas = Instant::now();
            let mut k = kapta_conpty_baslat(
                &paket,
                &format!("\"{exe_s}\" cocuk pty-boyut"),
                &alan,
                80,
                24,
            )?;
            let mut girdi = k.girdi.take().ok_or("ConPTY girdi yok")?;
            let cikti = k.cikti.take().ok_or("ConPTY çıktı yok")?;
            let ilk_goruldu = Arc::new(AtomicBool::new(false));
            let ikinci_goruldu = Arc::new(AtomicBool::new(false));
            let ilk_kopya = Arc::clone(&ilk_goruldu);
            let ikinci_kopya = Arc::clone(&ikinci_goruldu);

            let okuyucu = std::thread::spawn(move || {
                let mut r = BufReader::new(cikti);
                let mut toplanan = String::new();
                let mut buf = [0u8; 1024];
                while let Ok(n) = r.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    toplanan.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if toplanan.contains("boyut=80x24") {
                        ilk_kopya.store(true, Ordering::SeqCst);
                    }
                    if toplanan.contains("YENI_BOYUT=120x40") {
                        ikinci_kopya.store(true, Ordering::SeqCst);
                        break;
                    }
                }
                toplanan
            });

            let bekle_bas = Instant::now();
            while !ilk_goruldu.load(Ordering::SeqCst)
                && bekle_bas.elapsed() < Duration::from_secs(5)
            {
                std::thread::sleep(Duration::from_millis(15));
            }
            k.boyut_degistir(120, 40)?;
            girdi
                .write_all(b"boyut\r\ncik\r\n")
                .map_err(|e| e.to_string())?;
            girdi.flush().map_err(|e| e.to_string())?;

            let cikis = bekle(k.surec.0, 5000);
            drop(k.hpc);
            drop(girdi);
            let metin = okuyucu.join().map_err(|_| "ConPTY okuyucu çöktü")?;
            let utf8_tamam = metin.contains("utf8=çğıöşü");
            let gecti = ilk_goruldu.load(Ordering::SeqCst)
                && ikinci_goruldu.load(Ordering::SeqCst)
                && utf8_tamam;
            satirlar.push(SatirRaporu {
                ad: "G2.3 Windows AppContainer+ConPTY açılış, UTF-8/ANSI ve resize (80x24 -> 120x40)".into(),
                gecti,
                ayrinti: format!(
                    "ilk boyut 80x24={}, `ResizePseudoConsole(120, 40)` sonrası 120x40={}, UTF-8/ANSI korundu={utf8_tamam}, çıkış={cikis:?}, süre={} ms",
                    ilk_goruldu.load(Ordering::SeqCst),
                    ikinci_goruldu.load(Ordering::SeqCst),
                    bas.elapsed().as_millis()
                ),
            });
        }

        // ---------------------------------------------------------------------
        // G2.4: AppContainer + Job Object + ConPTY içinde torun süreç ve ağaç
        // sonlandırma
        // ---------------------------------------------------------------------
        {
            let bas = Instant::now();
            let mut k =
                kapta_conpty_baslat(&paket, &format!("\"{exe_s}\" cocuk torun"), &alan, 80, 24)?;
            let cikti = k.cikti.take().ok_or("ConPTY çıktı yok")?;
            let pidler_paylasilan: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));
            let pidler_kopya = Arc::clone(&pidler_paylasilan);

            let okuyucu = std::thread::spawn(move || {
                let mut r = BufReader::new(cikti);
                let mut toplanan = String::new();
                let mut buf = [0u8; 1024];
                while let Ok(n) = r.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    toplanan.push_str(&String::from_utf8_lossy(&buf[..n]));
                    for satir in toplanan.lines() {
                        if let Some(kalan) = satir.find("PIDLER ") {
                            let parcalar: Vec<u32> = satir[kalan + 7..]
                                .split_whitespace()
                                .filter_map(|p| {
                                    p.trim_matches(|c: char| !c.is_ascii_digit()).parse().ok()
                                })
                                .collect();
                            if parcalar.len() >= 2 {
                                *pidler_kopya.lock().unwrap_or_else(|e| e.into_inner()) =
                                    parcalar[..2].to_vec();
                            }
                        }
                    }
                }
            });

            let bekle_bas = Instant::now();
            while pidler_paylasilan
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .len()
                < 2
                && bekle_bas.elapsed() < Duration::from_secs(5)
            {
                std::thread::sleep(Duration::from_millis(15));
            }
            let pidler = pidler_paylasilan
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            let yasayan_once = pidler.iter().filter(|p| yasiyor(**p)).count();

            // Önce Job Object sonlandırılır, ardından ConPTY kapatılır (okuyucu boşaltırken).
            // SAFETY: geçerli Job Object tutamacı sonlandırılır.
            unsafe {
                TerminateJobObject(k.is.0, 1);
            }
            let _ = bekle(k.surec.0, 5000);
            drop(k.hpc);
            drop(k.girdi.take());
            let _ = okuyucu.join();
            std::thread::sleep(Duration::from_millis(150));
            let yasayan_sonra: Vec<u32> = pidler.iter().copied().filter(|p| yasiyor(*p)).collect();

            satirlar.push(SatirRaporu {
                ad: "G2.4 Windows AppContainer+Job+ConPTY torun süreç ağacı sonlandırma".into(),
                gecti: pidler.len() == 2 && yasayan_once == 2 && yasayan_sonra.is_empty(),
                ayrinti: format!(
                    "çocuk+torun PID={pidler:?}, kapanış öncesi yaşayan={yasayan_once}, `TerminateJobObject` + `ClosePseudoConsole` sonrası yaşayan={yasayan_sonra:?}, süre={} ms",
                    bas.elapsed().as_millis()
                ),
            });
        }

        // ---------------------------------------------------------------------
        // G2.5: Dolu çıktı (`full output`) varken taşma sınırı (1 MiB) ve
        // kilitlenmeden kapanış
        // ---------------------------------------------------------------------
        {
            let bas = Instant::now();
            let mut k =
                kapta_conpty_baslat(&paket, &format!("\"{exe_s}\" cocuk sel"), &alan, 80, 24)?;
            let cikti = k.cikti.take().ok_or("ConPTY çıktı yok")?;
            let tasti = Arc::new(AtomicBool::new(false));
            let toplam_okunan = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let tutulan_boyut = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let tasti_kopya = Arc::clone(&tasti);
            let toplam_kopya = Arc::clone(&toplam_okunan);
            let tutulan_kopya = Arc::clone(&tutulan_boyut);

            let okuyucu = std::thread::spawn(move || {
                let mut r = BufReader::new(cikti);
                let mut tampon = Vec::with_capacity(AZAMI_IS_CIKTISI_BAYT);
                let mut buf = [0u8; 8192];
                while let Ok(n) = r.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    toplam_kopya.fetch_add(n, Ordering::SeqCst);
                    if tampon.len() < AZAMI_IS_CIKTISI_BAYT {
                        let kalan = AZAMI_IS_CIKTISI_BAYT - tampon.len();
                        let eklenecek = kalan.min(n);
                        tampon.extend_from_slice(&buf[..eklenecek]);
                        tutulan_kopya.store(tampon.len(), Ordering::SeqCst);
                        if eklenecek < n {
                            tasti_kopya.store(true, Ordering::SeqCst);
                        }
                    } else {
                        tasti_kopya.store(true, Ordering::SeqCst);
                    }
                }
            });

            // Çıktı 1.5 MiB'ı aşana kadar bekle (taşma kesinleşsin).
            let bekle_bas = Instant::now();
            while toplam_okunan.load(Ordering::SeqCst) < 1536 * 1024
                && bekle_bas.elapsed() < Duration::from_secs(8)
            {
                std::thread::sleep(Duration::from_millis(15));
            }

            let kapanis_bas = Instant::now();
            // SAFETY: geçerli Job Object tutamacı sonlandırılır.
            unsafe {
                TerminateJobObject(k.is.0, 1);
            }
            let cikis_tamam = bekle(k.surec.0, 5000).is_some();
            drop(k.hpc);
            drop(k.girdi.take());
            okuyucu.join().map_err(|_| "sel okuyucu çöktü")?;
            let kapanis_ms = kapanis_bas.elapsed().as_millis();

            let okunan_bayt = toplam_okunan.load(Ordering::SeqCst);
            let tutulan_bayt = tutulan_boyut.load(Ordering::SeqCst);
            let tasti_mi = tasti.load(Ordering::SeqCst);

            satirlar.push(SatirRaporu {
                ad: "G2.5 Windows dolu çıktı altında 1 MiB sınır ve kilitlenmeden kapanış".into(),
                gecti: cikis_tamam
                    && tasti_mi
                    && tutulan_bayt == AZAMI_IS_CIKTISI_BAYT
                    && kapanis_ms < 5000,
                ayrinti: format!(
                    "toplam okunan={okunan_bayt} bayt, bellekte tutulan={tutulan_bayt} bayt (üst sınır 1 MiB), tasti={tasti_mi}, kapanış süresi={kapanis_ms} ms (< 5000 ms, toplam={} ms)",
                    bas.elapsed().as_millis()
                ),
            });
        }

        Ok(())
    })();

    let profil_temiz = profil_sil(&profil_adi);
    let dizin_temiz = std::fs::remove_dir_all(&kok);
    sonuc?;
    profil_temiz?;
    dizin_temiz.map_err(|e| format!("geçici dizin silinemedi: {e}"))?;

    satirlar.push(SatirRaporu {
        ad: "G2.6 Windows AppContainer profil ve geçici kök temizliği".into(),
        gecti: !kok.exists(),
        ayrinti: format!(
            "AppContainer profili `{profil_adi}` (`DeleteAppContainerProfile`) ve geçici kök `{}` silindi",
            kok.display()
        ),
    });

    Ok(satirlar)
}
