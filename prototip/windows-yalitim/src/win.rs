//! Win32 yardımcıları: SID'ler, kısıtlı belirteç, Job Object, borular ve
//! `CreateProcessWithLogonW`. Prototip kodudur; hatalar metin olarak döner.

use std::ffi::c_void;
use std::fs::File;
use std::io::{Read, Write};
use std::os::windows::io::FromRawHandle;
use std::ptr::{null, null_mut};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, HANDLE, HLOCAL, INVALID_HANDLE_VALUE, LocalFree, WAIT_OBJECT_0,
    WAIT_TIMEOUT,
};
use windows_sys::Win32::Security::Authorization::{ConvertSidToStringSidW, ConvertStringSidToSidW};
use windows_sys::Win32::Security::Cryptography::{CRYPT_INTEGER_BLOB, CryptUnprotectData};
use windows_sys::Win32::Security::{
    ACL, ACL_REVISION, AddAccessAllowedAce, CreateRestrictedToken, DISABLE_MAX_PRIVILEGE,
    DeriveCapabilitySidsFromName, GetLengthSid, GetTokenInformation, InitializeAcl, PSID,
    SECURITY_ATTRIBUTES, SID_AND_ATTRIBUTES, SetTokenInformation, TOKEN_ADJUST_DEFAULT,
    TOKEN_ASSIGN_PRIMARY, TOKEN_DEFAULT_DACL, TOKEN_DUPLICATE, TOKEN_GROUPS, TOKEN_QUERY,
    TOKEN_USER, TokenDefaultDacl, TokenGroups, TokenUser,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
    JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_BASIC_UI_RESTRICTIONS, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectBasicUIRestrictions, JobObjectExtendedLimitInformation, SetInformationJobObject,
};
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::SystemServices::SE_GROUP_LOGON_ID;
use windows_sys::Win32::System::Threading::{
    CREATE_NO_WINDOW, CREATE_UNICODE_ENVIRONMENT, CreateProcessWithLogonW, GetCurrentProcess,
    GetExitCodeProcess, INFINITE, LOGON_WITH_PROFILE, OpenProcessToken, PROCESS_INFORMATION,
    STARTF_USESTDHANDLES, STARTUPINFOW, TerminateProcess, WaitForSingleObject,
};

pub type Sonuc<T> = Result<T, String>;

/// Son Win32 hatasıyla birlikte bir adım adı.
pub fn hata(adim: &str) -> String {
    format!("{adim}: Win32 hatası {}", unsafe { GetLastError() })
}

pub fn genis(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn genisten(p: *const u16) -> String {
    let mut n = 0;
    // SAFETY: p, NUL ile biten bir UTF-16 dizesidir.
    unsafe {
        while *p.add(n) != 0 {
            n += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
    }
}

/// Sahipli SID: `LocalFree` ile ya da kopya olarak tutulur.
pub struct Sid(Vec<u8>);

impl Sid {
    pub fn metinden(s: &str) -> Sonuc<Sid> {
        let mut p: PSID = null_mut();
        // SAFETY: dize NUL ile biter; p çağrıdan sonra LocalFree ile bırakılır.
        unsafe {
            if ConvertStringSidToSidW(genis(s).as_ptr(), &mut p) == 0 {
                return Err(hata(&format!("SID çözülemedi ({s})")));
            }
            let sid = Sid::kopyala(p);
            LocalFree(p as HLOCAL);
            Ok(sid)
        }
    }

    /// # Safety
    /// `p` geçerli bir SID'i göstermelidir.
    pub unsafe fn kopyala(p: PSID) -> Sid {
        unsafe {
            let n = GetLengthSid(p) as usize;
            Sid(std::slice::from_raw_parts(p as *const u8, n).to_vec())
        }
    }

    pub fn ptr(&self) -> PSID {
        self.0.as_ptr() as PSID
    }

    pub fn metin(&self) -> String {
        let mut p: *mut u16 = null_mut();
        // SAFETY: SID geçerlidir; dize LocalFree ile bırakılır.
        unsafe {
            if ConvertSidToStringSidW(self.ptr(), &mut p) == 0 {
                return "?".into();
            }
            let s = genisten(p);
            LocalFree(p as HLOCAL);
            s
        }
    }
}

/// `DeriveCapabilitySidsFromName` ile addan türetilen yetenek SID'i
/// (`S-1-15-3-…`).
pub fn yetenek_sidi(ad: &str) -> Sonuc<String> {
    let mut gruplar: *mut PSID = null_mut();
    let mut grup_sayisi = 0u32;
    let mut yetenekler: *mut PSID = null_mut();
    let mut yetenek_sayisi = 0u32;
    // SAFETY: çıktı dizileri ve SID'leri LocalFree ile bırakılır.
    unsafe {
        if DeriveCapabilitySidsFromName(
            genis(ad).as_ptr(),
            &mut gruplar,
            &mut grup_sayisi,
            &mut yetenekler,
            &mut yetenek_sayisi,
        ) == 0
        {
            return Err(hata("DeriveCapabilitySidsFromName"));
        }
        let sid = Sid::kopyala(*yetenekler).metin();
        for i in 0..grup_sayisi as usize {
            LocalFree(*gruplar.add(i) as HLOCAL);
        }
        for i in 0..yetenek_sayisi as usize {
            LocalFree(*yetenekler.add(i) as HLOCAL);
        }
        LocalFree(gruplar as HLOCAL);
        LocalFree(yetenekler as HLOCAL);
        Ok(sid)
    }
}

/// Kullanıcı kapsamlı DPAPI ile şifrelenmiş veriyi çözer.
pub fn dpapi_coz(veri: &[u8]) -> Sonuc<Vec<u8>> {
    let giris = CRYPT_INTEGER_BLOB {
        cbData: veri.len() as u32,
        pbData: veri.as_ptr() as *mut u8,
    };
    let mut cikis = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };
    // SAFETY: bloblar geçerlidir; çıktı LocalFree ile bırakılır.
    unsafe {
        if CryptUnprotectData(&giris, null_mut(), null(), null(), null(), 0, &mut cikis) == 0 {
            return Err(hata("CryptUnprotectData"));
        }
        let v = std::slice::from_raw_parts(cikis.pbData, cikis.cbData as usize).to_vec();
        LocalFree(cikis.pbData as HLOCAL);
        Ok(v)
    }
}

pub struct Tutamac(pub HANDLE);

impl Drop for Tutamac {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            // SAFETY: tutamaç bu yapıya aittir.
            unsafe { CloseHandle(self.0) };
        }
    }
}

/// Belirteç bilgisini bir tampona okur.
fn belirtec_bilgisi(belirtec: HANDLE, sinif: i32) -> Sonuc<Vec<u64>> {
    let mut gerekli = 0u32;
    // SAFETY: önce boyut sorulur, sonra yeterli (8 bayt hizalı) tampon verilir.
    unsafe {
        GetTokenInformation(belirtec, sinif, null_mut(), 0, &mut gerekli);
        let mut tampon = vec![0u64; (gerekli as usize).div_ceil(8)];
        if GetTokenInformation(
            belirtec,
            sinif,
            tampon.as_mut_ptr() as *mut c_void,
            gerekli,
            &mut gerekli,
        ) == 0
        {
            return Err(hata("GetTokenInformation"));
        }
        Ok(tampon)
    }
}

/// Sürecin kendi belirteci.
pub fn kendi_belirteci() -> Sonuc<Tutamac> {
    let mut h: HANDLE = null_mut();
    // SAFETY: çıktı tutamacı Tutamac'a geçer.
    unsafe {
        if OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_DUPLICATE | TOKEN_QUERY | TOKEN_ASSIGN_PRIMARY | TOKEN_ADJUST_DEFAULT,
            &mut h,
        ) == 0
        {
            return Err(hata("OpenProcessToken"));
        }
    }
    Ok(Tutamac(h))
}

pub fn kullanici_sidi(belirtec: HANDLE) -> Sonuc<Sid> {
    let t = belirtec_bilgisi(belirtec, TokenUser)?;
    // SAFETY: tampon TOKEN_USER içerir.
    unsafe { Ok(Sid::kopyala((*(t.as_ptr() as *const TOKEN_USER)).User.Sid)) }
}

/// Belirtecin oturum SID'i (`S-1-5-5-x-y`).
pub fn oturum_sidi(belirtec: HANDLE) -> Sonuc<Sid> {
    let t = belirtec_bilgisi(belirtec, TokenGroups)?;
    // SAFETY: tampon TOKEN_GROUPS içerir; dizi GroupCount öğelidir.
    unsafe {
        let g = &*(t.as_ptr() as *const TOKEN_GROUPS);
        let dizi = std::slice::from_raw_parts(g.Groups.as_ptr(), g.GroupCount as usize);
        dizi.iter()
            .find(|s| s.Attributes & SE_GROUP_LOGON_ID as u32 != 0)
            .map(|s| Sid::kopyala(s.Sid))
            .ok_or_else(|| "belirteçte oturum SID'i yok".to_string())
    }
}

/// Kısıtlı belirteç: bütün ayrıcalıklar (SeChangeNotify hariç) kaldırılır,
/// verilen SID'ler kısıtlayıcı olur. Varsayılan DACL kullanıcıya, SYSTEM'e ve
/// `dacl_ek`'teki SID'lere tam erişim verir. `dacl_ek` kısıtlayıcılardan biri
/// olmalıdır (süreç kendi nesnelerini açabilsin) ve Everyone ya da Users
/// olmamalıdır: aynı yalıtım kullanıcısıyla çalışan başka bir çalışma alanının
/// süreci de onları taşır ve bu sürecin nesnelerini açabilirdi.
pub fn kisitli_belirtec(
    kaynak: HANDLE,
    kisitlayicilar: &[Sid],
    dacl_ek: &[Sid],
    ayricaliklari_koru: bool,
) -> Sonuc<Tutamac> {
    let dizi: Vec<SID_AND_ATTRIBUTES> = kisitlayicilar
        .iter()
        .map(|s| SID_AND_ATTRIBUTES {
            Sid: s.ptr(),
            Attributes: 0,
        })
        .collect();
    let mut yeni: HANDLE = null_mut();
    // SAFETY: dizi ve SID'ler çağrı boyunca yaşar.
    unsafe {
        if CreateRestrictedToken(
            kaynak,
            if ayricaliklari_koru {
                0
            } else {
                DISABLE_MAX_PRIVILEGE
            },
            0,
            null(),
            0,
            null(),
            dizi.len() as u32,
            if dizi.is_empty() {
                null()
            } else {
                dizi.as_ptr()
            },
            &mut yeni,
        ) == 0
        {
            return Err(hata("CreateRestrictedToken"));
        }
    }
    let yeni = Tutamac(yeni);
    let mut sahipler = vec![kullanici_sidi(kaynak)?, Sid::metinden("S-1-5-18")?];
    for s in dacl_ek {
        sahipler.push(Sid(s.0.clone()));
    }
    let boyut: u32 = 8 + sahipler
        .iter()
        // SAFETY: SID'ler geçerlidir.
        .map(|s| 8 + unsafe { GetLengthSid(s.ptr()) })
        .sum::<u32>();
    let mut acl = vec![0u64; (boyut as usize).div_ceil(8)];
    let pacl = acl.as_mut_ptr() as *mut ACL;
    // SAFETY: ACL tamponu hesaplanan boyuttadır.
    unsafe {
        if InitializeAcl(pacl, boyut, ACL_REVISION) == 0 {
            return Err(hata("InitializeAcl"));
        }
        for s in &sahipler {
            // GENERIC_ALL
            if AddAccessAllowedAce(pacl, ACL_REVISION, 0x1000_0000, s.ptr()) == 0 {
                return Err(hata("AddAccessAllowedAce"));
            }
        }
        let dacl = TOKEN_DEFAULT_DACL { DefaultDacl: pacl };
        if SetTokenInformation(
            yeni.0,
            TokenDefaultDacl,
            &dacl as *const _ as *const c_void,
            size_of::<TOKEN_DEFAULT_DACL>() as u32,
        ) == 0
        {
            return Err(hata("SetTokenInformation(TokenDefaultDacl)"));
        }
    }
    Ok(yeni)
}

/// Job Object: tutamaç kapanınca ağaç ölür, en çok 256 süreç; `ui` ise bütün
/// UI kısıtları.
pub fn is_nesnesi(surec_siniri: bool, ui: bool) -> Sonuc<Tutamac> {
    // SAFETY: yapılar sıfırla başlatılır ve boyutlarıyla verilir.
    unsafe {
        let job = CreateJobObjectW(null(), null());
        if job.is_null() {
            return Err(hata("CreateJobObjectW"));
        }
        let job = Tutamac(job);
        let mut bilgi: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        bilgi.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
        if surec_siniri {
            bilgi.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
            bilgi.BasicLimitInformation.ActiveProcessLimit = 256;
        }
        if SetInformationJobObject(
            job.0,
            JobObjectExtendedLimitInformation,
            &bilgi as *const _ as *const c_void,
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        ) == 0
        {
            return Err(hata("SetInformationJobObject(limit)"));
        }
        if ui {
            let ui = JOBOBJECT_BASIC_UI_RESTRICTIONS {
                UIRestrictionsClass: 0xFF,
            };
            if SetInformationJobObject(
                job.0,
                JobObjectBasicUIRestrictions,
                &ui as *const _ as *const c_void,
                size_of::<JOBOBJECT_BASIC_UI_RESTRICTIONS>() as u32,
            ) == 0
            {
                return Err(hata("SetInformationJobObject(UI)"));
            }
        }
        Ok(job)
    }
}

pub fn ise_ata(job: HANDLE, surec: HANDLE) -> Sonuc<()> {
    // SAFETY: iki tutamaç da geçerlidir.
    if unsafe { AssignProcessToJobObject(job, surec) } == 0 {
        return Err(hata("AssignProcessToJobObject"));
    }
    Ok(())
}

/// Ortam bloğu: `ad=değer\0…\0\0`, ada göre sıralı.
pub fn ortam_blogu(ortam: &[(String, String)]) -> Vec<u16> {
    let mut v: Vec<&(String, String)> = ortam.iter().collect();
    v.sort_by_key(|(a, _)| a.to_uppercase());
    let mut blok = Vec::new();
    for (a, d) in v {
        blok.extend(format!("{a}={d}").encode_utf16());
        blok.push(0);
    }
    blok.push(0);
    blok
}

pub fn boru(miras_okuma: bool) -> Sonuc<(Tutamac, Tutamac)> {
    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: 1,
    };
    let (mut o, mut y): (HANDLE, HANDLE) = (null_mut(), null_mut());
    // SAFETY: çıktı tutamaçları Tutamac'a geçer.
    unsafe {
        if CreatePipe(&mut o, &mut y, &sa, 0) == 0 {
            return Err(hata("CreatePipe"));
        }
        // Yalnızca çocuğa geçecek uç miras alınabilir kalır.
        use windows_sys::Win32::Foundation::{HANDLE_FLAG_INHERIT, SetHandleInformation};
        let kalan = if miras_okuma { y } else { o };
        SetHandleInformation(kalan, HANDLE_FLAG_INHERIT, 0);
    }
    Ok((Tutamac(o), Tutamac(y)))
}

pub struct OturumSonucu {
    pub cikis: Option<u32>,
    pub standart: String,
    pub hata: String,
    pub sure: Duration,
    /// Dönen süreç tutamacıyla Kalfa'nın işine atama (P6).
    pub ise_atama: Sonuc<()>,
}

/// `CreateProcessWithLogonW` ile verilen kullanıcı olarak `program` +
/// `arguman`'ı başlatır; `girdi`'yi standart girdiye yazıp kapatır, iki çıktıyı
/// toplar. Süreç, kapanınca ağacı öldüren bir işe atanmaya çalışılır.
pub fn oturumla_calistir(
    kullanici: &str,
    parola: &str,
    program: &str,
    komut_satiri: &str,
    dizin: &str,
    girdi: &[u8],
    zaman_asimi: Duration,
) -> Sonuc<OturumSonucu> {
    let (girdi_o, girdi_y) = boru(true)?;
    let (cikti_o, cikti_y) = boru(false)?;
    let (hata_o, hata_y) = boru(false)?;
    let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
    si.cb = size_of::<STARTUPINFOW>() as u32;
    si.dwFlags = STARTF_USESTDHANDLES;
    si.hStdInput = girdi_o.0;
    si.hStdOutput = cikti_y.0;
    si.hStdError = hata_y.0;
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let mut satir = genis(komut_satiri);
    let bas = Instant::now();
    // SAFETY: bütün dizeler NUL ile biter ve çağrı boyunca yaşar.
    let tamam = unsafe {
        CreateProcessWithLogonW(
            genis(kullanici).as_ptr(),
            genis(".").as_ptr(),
            genis(parola).as_ptr(),
            LOGON_WITH_PROFILE,
            genis(program).as_ptr(),
            satir.as_mut_ptr(),
            CREATE_NO_WINDOW | CREATE_UNICODE_ENVIRONMENT,
            null(),
            genis(dizin).as_ptr(),
            &si,
            &mut pi,
        )
    };
    if tamam == 0 {
        return Err(hata("CreateProcessWithLogonW"));
    }
    let surec = Tutamac(pi.hProcess);
    drop(Tutamac(pi.hThread));
    drop((girdi_o, cikti_y, hata_y));
    let is = is_nesnesi(false, false)?;
    let ise_atama = ise_ata(is.0, surec.0);
    // SAFETY: uçlar bu işleve aittir; File'a devredilir.
    let (mut yaz, mut oku_c, mut oku_h) = unsafe {
        let y = File::from_raw_handle(girdi_y.0 as _);
        let c = File::from_raw_handle(cikti_o.0 as _);
        let h = File::from_raw_handle(hata_o.0 as _);
        std::mem::forget((girdi_y, cikti_o, hata_o));
        (y, c, h)
    };
    let _ = yaz.write_all(girdi);
    drop(yaz);
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
    // SAFETY: süreç tutamacı geçerlidir.
    let cikis = unsafe {
        match WaitForSingleObject(surec.0, if ms == 0 { INFINITE } else { ms }) {
            WAIT_OBJECT_0 => {
                let mut kod = 0u32;
                GetExitCodeProcess(surec.0, &mut kod);
                Some(kod)
            }
            WAIT_TIMEOUT => {
                TerminateProcess(surec.0, 1);
                None
            }
            _ => None,
        }
    };
    let sure = bas.elapsed();
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
