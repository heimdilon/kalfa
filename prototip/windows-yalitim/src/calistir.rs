//! `calistir`: yalıtım kullanıcısı olarak çalışır (aşama 2 tasarımı §9.3).
//! İsteği standart girdiden okur, kısıtlı belirteci türetir, komutu Job
//! Object içinde başlatır ve çıkış kodunu döndürür. Komutun standart girdisi
//! NUL'dur; çıktıları bu sürecinkilere yazılır.

use std::io::Read;
use std::ptr::{null, null_mut};

use serde::{Deserialize, Serialize};
use windows_sys::Win32::Foundation::{
    DUPLICATE_SAME_ACCESS, DuplicateHandle, GENERIC_READ, HANDLE, WAIT_OBJECT_0,
};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Console::{GetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};
use windows_sys::Win32::System::Threading::{
    CREATE_NO_WINDOW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessAsUserW,
    GetCurrentProcess, GetExitCodeProcess, INFINITE, PROCESS_INFORMATION, ResumeThread,
    STARTF_USESTDHANDLES, STARTUPINFOW, WaitForSingleObject,
};

use crate::win::{self, Sid, Sonuc, Tutamac};

/// `olc`'un yardımcıya gönderdiği istek.
#[derive(Debug, Serialize, Deserialize)]
pub struct Istek {
    /// Tam komut satırı.
    pub komut: String,
    pub dizin: String,
    pub ortam: Vec<(String, String)>,
    /// Kısıtlayıcı SID'ler. `OTURUM` ve `KULLANICI` bu sürecin belirtecinden
    /// çözülür.
    pub kisitlayicilar: Vec<String>,
    /// Varsayılan DACL'e kullanıcı ve SYSTEM dışında eklenecek SID'ler.
    pub dacl_ek: Vec<String>,
    /// Kısıtlı belirteç kullanılmasın (karşılaştırma için).
    pub kisitsiz: bool,
    pub ui_kisit: bool,
    /// Ayrıcalıklar kaldırılmasın (karşılaştırma için).
    #[serde(default)]
    pub ayricaliklari_koru: bool,
}

const ONEK: &str = "KALFA-PROTOTIP";

pub fn ana() -> i32 {
    match calistir() {
        Ok(kod) => kod as i32,
        Err(e) => {
            eprintln!("{ONEK}-HATA: {e}");
            250
        }
    }
}

fn miras_kopyasi(h: HANDLE) -> Sonuc<Tutamac> {
    let mut kopya: HANDLE = null_mut();
    // SAFETY: kaynak tutamaç bu sürece aittir.
    unsafe {
        let s = GetCurrentProcess();
        if DuplicateHandle(s, h, s, &mut kopya, 0, 1, DUPLICATE_SAME_ACCESS) == 0 {
            return Err(win::hata("DuplicateHandle"));
        }
    }
    Ok(Tutamac(kopya))
}

fn calistir() -> Sonuc<u32> {
    let mut metin = String::new();
    std::io::stdin()
        .read_to_string(&mut metin)
        .map_err(|e| format!("istek okunamadı: {e}"))?;
    let istek: Istek =
        serde_json::from_str(&metin).map_err(|e| format!("istek çözülemedi: {e}"))?;

    // Yükleyici ve çökme hataları geliştiricinin masaüstünde iletişim kutusu
    // açmasın (P10: UI kısıtları bunları engellemiyor). Çocuk süreç bu kipi
    // devralır.
    // SAFETY: yalnızca bu sürecin hata kipini değiştirir.
    unsafe {
        use windows_sys::Win32::System::Diagnostics::Debug::{
            SEM_FAILCRITICALERRORS, SEM_NOGPFAULTERRORBOX, SEM_NOOPENFILEERRORBOX, SetErrorMode,
        };
        SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX);
    }
    let kendi = win::kendi_belirteci()?;
    let belirtec = if istek.kisitsiz {
        None
    } else {
        let coz = |liste: &[String]| -> Sonuc<Vec<Sid>> {
            liste
                .iter()
                .map(|k| match k.as_str() {
                    "OTURUM" => win::oturum_sidi(kendi.0),
                    "KULLANICI" => win::kullanici_sidi(kendi.0),
                    s => Sid::metinden(s),
                })
                .collect()
        };
        let sidler = coz(&istek.kisitlayicilar)?;
        let dacl = coz(&istek.dacl_ek)?;
        Some(win::kisitli_belirtec(
            kendi.0,
            &sidler,
            &dacl,
            istek.ayricaliklari_koru,
        )?)
    };
    let is = win::is_nesnesi(true, istek.ui_kisit)?;

    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: 1,
    };
    // SAFETY: tutamaçlar Tutamac'a geçer; yapılar çağrı boyunca yaşar.
    unsafe {
        let nul = CreateFileW(
            win::genis("NUL").as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            &sa,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            null_mut(),
        );
        let nul = Tutamac(nul);
        let cikti = miras_kopyasi(GetStdHandle(STD_OUTPUT_HANDLE))?;
        let hata = miras_kopyasi(GetStdHandle(STD_ERROR_HANDLE))?;
        let mut si: STARTUPINFOW = std::mem::zeroed();
        si.cb = size_of::<STARTUPINFOW>() as u32;
        si.dwFlags = STARTF_USESTDHANDLES;
        si.hStdInput = nul.0;
        si.hStdOutput = cikti.0;
        si.hStdError = hata.0;
        let mut pi: PROCESS_INFORMATION = std::mem::zeroed();
        let mut satir = win::genis(&istek.komut);
        let ortam = win::ortam_blogu(&istek.ortam);
        let bayraklar = CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW;
        let tamam = match &belirtec {
            Some(b) => CreateProcessAsUserW(
                b.0,
                null(),
                satir.as_mut_ptr(),
                null(),
                null(),
                1,
                bayraklar,
                ortam.as_ptr() as *const _,
                win::genis(&istek.dizin).as_ptr(),
                &si,
                &mut pi,
            ),
            None => windows_sys::Win32::System::Threading::CreateProcessW(
                null(),
                satir.as_mut_ptr(),
                null(),
                null(),
                1,
                bayraklar,
                ortam.as_ptr() as *const _,
                win::genis(&istek.dizin).as_ptr(),
                &si,
                &mut pi,
            ),
        };
        if tamam == 0 {
            return Err(win::hata("CreateProcessAsUserW"));
        }
        let surec = Tutamac(pi.hProcess);
        let is_parcacigi = Tutamac(pi.hThread);
        win::ise_ata(is.0, surec.0)?;
        ResumeThread(is_parcacigi.0);
        drop((nul, cikti, hata));
        if WaitForSingleObject(surec.0, INFINITE) != WAIT_OBJECT_0 {
            return Err(win::hata("WaitForSingleObject"));
        }
        let mut kod = 0u32;
        GetExitCodeProcess(surec.0, &mut kod);
        Ok(kod)
    }
}
