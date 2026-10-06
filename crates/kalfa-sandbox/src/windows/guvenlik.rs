//! Windows güvenlik yardımcıları (aşama 2 tasarımı §9): SID'ler,
//! AppContainer profilleri, dosya izinleri (DACL) ve sürücü köklerindeki
//! listeleme izni.
//!
//! İzinler Win32 API'siyle verilir: `icacls` hiçbir hesaba karşılık gelmeyen
//! SID'leri (paket ve yetenek SID'leri) reddediyor (§9.4).

use std::ffi::{OsStr, c_void};
use std::fmt;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, GetLastError,
    HANDLE, HLOCAL, INVALID_HANDLE_VALUE, LocalFree,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertSidToStringSidW,
    ConvertStringSecurityDescriptorToSecurityDescriptorW, ConvertStringSidToSidW,
    EXPLICIT_ACCESS_W, GRANT_ACCESS, GetNamedSecurityInfoW, GetSecurityInfo, NO_MULTIPLE_TRUSTEE,
    REVOKE_ACCESS, SDDL_REVISION_1, SE_FILE_OBJECT, SetEntriesInAclW, SetNamedSecurityInfoW,
    TRUSTEE_IS_SID, TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
};
use windows_sys::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeleteAppContainerProfile, DeriveAppContainerSidFromAppContainerName,
};
use windows_sys::Win32::Security::{
    ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, ACL_SIZE_INFORMATION, AclSizeInformation, AddAce,
    DACL_SECURITY_INFORMATION, DeriveCapabilitySidsFromName, FreeSid, GetAce, GetAclInformation,
    GetFileSecurityW, GetLengthSid, GetSecurityDescriptorControl, GetSecurityDescriptorDacl,
    GetTokenInformation, INHERIT_ONLY_ACE, InitializeAcl, InitializeSecurityDescriptor,
    OWNER_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
    SE_DACL_AUTO_INHERIT_REQ, SE_DACL_AUTO_INHERITED, SE_DACL_PROTECTED, SECURITY_ATTRIBUTES,
    SECURITY_DESCRIPTOR, SetFileSecurityW, SetKernelObjectSecurity, SetSecurityDescriptorControl,
    SetSecurityDescriptorDacl, TOKEN_ELEVATION, TOKEN_QUERY, TOKEN_USER, TokenElevation, TokenUser,
    UNPROTECTED_DACL_SECURITY_INFORMATION,
};
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, CreateDirectoryW, CreateFileW, FILE_ATTRIBUTE_DIRECTORY,
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, GetDriveTypeW,
    GetFileInformationByHandle, GetLogicalDrives, GetVolumeInformationW, OPEN_EXISTING,
    READ_CONTROL, WRITE_DAC,
};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::SystemServices::SECURITY_DESCRIPTOR_REVISION;
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows_sys::Win32::UI::Shell::{
    FOLDERID_ProgramData, FOLDERID_ProgramFiles, FOLDERID_ProgramFilesX86, FOLDERID_UserProfiles,
    SHGetKnownFolderPath,
};

/// Kalfa'nın bütün kaplarının taşıdığı araç zinciri yeteneğinin adı (§9).
/// SID'i addan türetilir: `S-1-15-3-1024-…`.
pub const ARAC_ZINCIRI_YETENEGI: &str = "kalfaAracZinciri";
/// `internetClient` yeteneği; ağlı çalıştırmada verilir (§9.3).
pub const INTERNET_ISTEMCISI: &str = "S-1-15-3-1";
/// `ALL APPLICATION PACKAGES`: Program Files ve sistem dizinlerini okur.
pub const TUM_UYGULAMA_PAKETLERI: &str = "S-1-15-2-1";
/// Yöneticiler ve SYSTEM; mirası kesilen nesnelerde tam izin alırlar.
pub const YONETICILER: &str = "S-1-5-32-544";
pub const SISTEM: &str = "S-1-5-18";

/// `ListDirectory | ReadAttributes | Traverse | Synchronize` (`0x1000A1`):
/// kök ve üst dizinlerde adları listelemek için (§9.2).
pub const MASKE_LISTELE: u32 = 0x0010_00A1;
/// `ReadAndExecute`.
pub const MASKE_OKU: u32 = 0x0012_00A9;
/// `ReadAndExecute | Write`: silme ve yeniden adlandırma yok.
pub const MASKE_OKU_YAZ: u32 = 0x0012_01BF;
/// `Modify`: okuma, yazma, çalıştırma ve silme.
pub const MASKE_DEGISTIR: u32 = 0x0013_01BF;
/// `FullControl`.
pub const MASKE_TAM: u32 = 0x001F_01FF;

/// Miras bayrakları.
pub const MIRAS_YOK: u32 = 0;
/// `OI|CI`: nesnenin kendisi ve altındaki her şey.
pub const MIRAS: u32 = 0x3;
/// `OI|CI|IO`: yalnızca alttakiler, nesnenin kendisi değil.
pub const YALNIZ_ALTTAKILER: u32 = 0x3 | INHERIT_ONLY_ACE;

const HRESULT_ZATEN_VAR: i32 = 0x8007_00B7_u32 as i32;
const HRESULT_BULUNAMADI: i32 = 0x8007_0002_u32 as i32;
const DRIVE_FIXED: u32 = 3;
const FILE_PERSISTENT_ACLS: u32 = 0x8;

/// NUL ile biten UTF-16 dize.
pub(crate) fn genis(s: impl AsRef<OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(Some(0)).collect()
}

/// # Safety
/// `p`, NUL ile biten bir UTF-16 dizeyi göstermelidir.
unsafe fn genisten(p: *const u16) -> String {
    // SAFETY: çağıran dizenin NUL ile bittiğini garanti eder.
    unsafe {
        let mut n = 0;
        while *p.add(n) != 0 {
            n += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
    }
}

fn son_hata(adim: &str) -> io::Error {
    let e = io::Error::last_os_error();
    io::Error::new(e.kind(), format!("{adim}: {e}"))
}

fn win32_hatasi(adim: &str, kod: u32) -> io::Error {
    let e = io::Error::from_raw_os_error(kod as i32);
    io::Error::new(e.kind(), format!("{adim}: {e}"))
}

fn hresult_hatasi(adim: &str, hr: i32) -> io::Error {
    // HRESULT_FROM_WIN32: alt 16 bit Win32 hata kodudur.
    let kod = if (hr as u32) & 0xFFFF_0000 == 0x8007_0000 {
        (hr as u32) & 0xFFFF
    } else {
        hr as u32
    };
    win32_hatasi(adim, kod)
}

/// `LocalFree` ile bırakılan bellek.
struct Yerel(*mut c_void);

impl Drop for Yerel {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: işaretçi bir Win32 işlevinin LocalAlloc ile ayırdığı
            // bellektir ve yalnızca burada bırakılır.
            unsafe { LocalFree(self.0 as HLOCAL) };
        }
    }
}

/// Sahipli bir SID.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Sid(Vec<u8>);

impl Sid {
    /// `S-1-…` biçiminden.
    pub fn metinden(s: &str) -> io::Result<Sid> {
        let mut p: PSID = null_mut();
        // SAFETY: dize NUL ile biter; dönen SID LocalFree ile bırakılır.
        unsafe {
            if ConvertStringSidToSidW(genis(s).as_ptr(), &mut p) == 0 {
                return Err(son_hata(&format!("SID çözülemedi ({s})")));
            }
            let _bellek = Yerel(p);
            Ok(Sid::kopyala(p))
        }
    }

    /// # Safety
    /// `p` geçerli bir SID'i göstermelidir.
    unsafe fn kopyala(p: PSID) -> Sid {
        // SAFETY: çağıran SID'in geçerli olduğunu garanti eder.
        unsafe {
            let n = GetLengthSid(p) as usize;
            Sid(std::slice::from_raw_parts(p as *const u8, n).to_vec())
        }
    }

    pub(crate) fn ptr(&self) -> PSID {
        self.0.as_ptr() as PSID
    }

    pub fn metin(&self) -> String {
        let mut p: *mut u16 = null_mut();
        // SAFETY: SID geçerlidir; dize LocalFree ile bırakılır.
        unsafe {
            if ConvertSidToStringSidW(self.ptr(), &mut p) == 0 {
                return String::from("S-?");
            }
            let _bellek = Yerel(p as *mut c_void);
            genisten(p)
        }
    }
}

impl fmt::Debug for Sid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.metin())
    }
}

/// Addan türetilen yetenek SID'i (`DeriveCapabilitySidsFromName`).
pub fn yetenek_sidi(ad: &str) -> io::Result<Sid> {
    let mut gruplar: *mut PSID = null_mut();
    let mut grup_sayisi = 0u32;
    let mut yetenekler: *mut PSID = null_mut();
    let mut yetenek_sayisi = 0u32;
    // SAFETY: çıktı dizileri ve içlerindeki SID'ler LocalFree ile bırakılır.
    unsafe {
        if DeriveCapabilitySidsFromName(
            genis(ad).as_ptr(),
            &mut gruplar,
            &mut grup_sayisi,
            &mut yetenekler,
            &mut yetenek_sayisi,
        ) == 0
        {
            return Err(son_hata("DeriveCapabilitySidsFromName"));
        }
        let sonuc = if yetenek_sayisi == 0 {
            Err(io::Error::other("yetenek SID'i türetilemedi"))
        } else {
            Ok(Sid::kopyala(*yetenekler))
        };
        for i in 0..grup_sayisi as usize {
            LocalFree(*gruplar.add(i) as HLOCAL);
        }
        for i in 0..yetenek_sayisi as usize {
            LocalFree(*yetenekler.add(i) as HLOCAL);
        }
        LocalFree(gruplar as HLOCAL);
        LocalFree(yetenekler as HLOCAL);
        sonuc
    }
}

/// Araç zinciri yeteneğinin SID'i.
pub fn arac_zinciri_sidi() -> io::Result<Sid> {
    yetenek_sidi(ARAC_ZINCIRI_YETENEGI)
}

/// 128 bitlik rastgele onaltılık dize (CoCreateGuid ile).
pub fn rastgele_onaltilik_128() -> io::Result<String> {
    let mut guid = std::mem::MaybeUninit::uninit();
    // SAFETY: CoCreateGuid geçerli bir GUID işaretçisi alır.
    let hr = unsafe { windows_sys::Win32::System::Com::CoCreateGuid(guid.as_mut_ptr()) };
    if hr != 0 {
        return Err(hresult_hatasi("CoCreateGuid", hr));
    }
    // SAFETY: hr == 0 olduğundan guid başarıyla doldurulmuştur.
    let guid = unsafe { guid.assume_init() };
    let mut baytlar = [0u8; 16];
    baytlar[0..4].copy_from_slice(&guid.data1.to_ne_bytes());
    baytlar[4..6].copy_from_slice(&guid.data2.to_ne_bytes());
    baytlar[6..8].copy_from_slice(&guid.data3.to_ne_bytes());
    baytlar[8..16].copy_from_slice(&guid.data4);
    Ok(hex::encode(baytlar))
}

/// AppContainer profilini oluşturur; varsa SID'ini addan türetir.
pub fn profil_olustur(ad: &str, aciklama: &str) -> io::Result<Sid> {
    let mut p: PSID = null_mut();
    let ad_g = genis(ad);
    // SAFETY: dizeler NUL ile biter; SID FreeSid ile bırakılır.
    unsafe {
        let hr = CreateAppContainerProfile(
            ad_g.as_ptr(),
            ad_g.as_ptr(),
            genis(aciklama).as_ptr(),
            null(),
            0,
            &mut p,
        );
        if hr == HRESULT_ZATEN_VAR {
            return profil_sidi(ad);
        }
        if hr != 0 {
            return Err(hresult_hatasi("CreateAppContainerProfile", hr));
        }
        let sid = Sid::kopyala(p);
        FreeSid(p);
        Ok(sid)
    }
}

/// Profilin SID'i addan türetilir; profil var olmasa da aynıdır.
pub fn profil_sidi(ad: &str) -> io::Result<Sid> {
    let mut p: PSID = null_mut();
    // SAFETY: dize NUL ile biter; SID FreeSid ile bırakılır.
    unsafe {
        let hr = DeriveAppContainerSidFromAppContainerName(genis(ad).as_ptr(), &mut p);
        if hr != 0 {
            return Err(hresult_hatasi(
                "DeriveAppContainerSidFromAppContainerName",
                hr,
            ));
        }
        let sid = Sid::kopyala(p);
        FreeSid(p);
        Ok(sid)
    }
}

/// Profili siler; profil yoksa başarılı sayılır. Yükseltilmemiş süreçten
/// çağrılmalıdır: 2c'de yükseltilmiş süreçten silinen profiller yerinde kaldı.
pub fn profil_sil(ad: &str) -> io::Result<()> {
    // SAFETY: dize NUL ile biter.
    let hr = unsafe { DeleteAppContainerProfile(genis(ad).as_ptr()) };
    if hr != 0 && hr != HRESULT_BULUNAMADI {
        return Err(hresult_hatasi("DeleteAppContainerProfile", hr));
    }
    Ok(())
}

/// Sürecin belirteci; düşünce kapanır.
struct Belirtec(HANDLE);

impl Belirtec {
    fn ac() -> io::Result<Self> {
        let mut h: HANDLE = null_mut();
        // SAFETY: sözde tutamaç ve çıktı tutamacı; tutamaç Drop'ta kapanır.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut h) } == 0 {
            return Err(son_hata("OpenProcessToken"));
        }
        Ok(Self(h))
    }

    fn bilgi(&self, sinif: i32) -> io::Result<Vec<u64>> {
        let mut gerekli = 0u32;
        // SAFETY: önce boyut sorulur, sonra 8 bayt hizalı, yeterli tampon verilir.
        unsafe {
            GetTokenInformation(self.0, sinif, null_mut(), 0, &mut gerekli);
            let mut tampon = vec![0u64; (gerekli as usize).div_ceil(8).max(1)];
            if GetTokenInformation(
                self.0,
                sinif,
                tampon.as_mut_ptr() as *mut c_void,
                // Bazı sınıflar (TokenElevation) tam boyut ister.
                gerekli,
                &mut gerekli,
            ) == 0
            {
                return Err(son_hata("GetTokenInformation"));
            }
            Ok(tampon)
        }
    }
}

impl Drop for Belirtec {
    fn drop(&mut self) {
        // SAFETY: tutamaç bu yapıya aittir.
        unsafe { CloseHandle(self.0) };
    }
}

/// Geliştiricinin (sürecin kullanıcısının) SID'i.
pub fn kullanici_sidi() -> io::Result<Sid> {
    let tampon = Belirtec::ac()?.bilgi(TokenUser)?;
    // SAFETY: tampon bir TOKEN_USER yapısıyla başlar; SID tamponun içindedir.
    unsafe {
        let kullanici = &*(tampon.as_ptr() as *const TOKEN_USER);
        Ok(Sid::kopyala(kullanici.User.Sid))
    }
}

/// Süreç yükseltilmiş (yönetici) mi?
pub fn yukseltilmis_mi() -> io::Result<bool> {
    let tampon = Belirtec::ac()?.bilgi(TokenElevation)?;
    // SAFETY: tampon bir TOKEN_ELEVATION yapısıdır.
    let yukseltme = unsafe { &*(tampon.as_ptr() as *const TOKEN_ELEVATION) };
    Ok(yukseltme.TokenIsElevated != 0)
}

/// Bir DACL girdisi: kime, hangi izin, hangi miras.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Izin {
    pub sid: Sid,
    pub maske: u32,
    pub miras: u32,
}

/// DACL'deki bir izin girdisi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ace {
    /// `ACCESS_ALLOWED_ACE_TYPE` (0) ya da `ACCESS_DENIED_ACE_TYPE` (1);
    /// diğer türlerin SID'i okunmaz.
    pub tur: u8,
    pub bayraklar: u8,
    pub maske: u32,
    pub sid: Option<Sid>,
}

impl Ace {
    /// Mirasla gelmiş girdi mi?
    pub fn miras_mi(&self) -> bool {
        u32::from(self.bayraklar) & 0x10 != 0
    }
}

/// Bir nesnenin güvenlik tanımlayıcısı (`GetNamedSecurityInfoW`).
struct Tanimlayici {
    sd: Yerel,
    dacl: *mut ACL,
}

impl Tanimlayici {
    fn oku(yol: &Path) -> io::Result<Self> {
        let mut dacl: *mut ACL = null_mut();
        let mut sd: PSECURITY_DESCRIPTOR = null_mut();
        // SAFETY: yol NUL ile biter; tanımlayıcı LocalFree ile bırakılır; DACL
        // tanımlayıcının içini gösterir.
        let kod = unsafe {
            GetNamedSecurityInfoW(
                genis(yol).as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                &mut dacl,
                null_mut(),
                &mut sd,
            )
        };
        if kod != ERROR_SUCCESS {
            return Err(win32_hatasi(
                &format!("GetNamedSecurityInfoW({})", yol.display()),
                kod,
            ));
        }
        Ok(Self {
            sd: Yerel(sd),
            dacl,
        })
    }

    fn denetim(&self) -> u16 {
        let mut denetim = 0u16;
        let mut surum = 0u32;
        // SAFETY: tanımlayıcı geçerlidir.
        unsafe { GetSecurityDescriptorControl(self.sd.0, &mut denetim, &mut surum) };
        denetim
    }

    fn aceler(&self) -> Vec<Ace> {
        // SAFETY: DACL geçerlidir ya da boştur (tam erişim); GetAce ile okunan
        // girdiler DACL'in içindedir.
        unsafe { aceler(self.dacl) }
    }
}

/// # Safety
/// `dacl` boş ya da geçerli bir ACL olmalıdır.
unsafe fn aceler(dacl: *const ACL) -> Vec<Ace> {
    if dacl.is_null() {
        return Vec::new();
    }
    // SAFETY: çağıran ACL'in geçerli olduğunu garanti eder.
    unsafe {
        let mut bilgi: ACL_SIZE_INFORMATION = std::mem::zeroed();
        if GetAclInformation(
            dacl,
            (&raw mut bilgi).cast(),
            size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        ) == 0
        {
            return Vec::new();
        }
        let mut sonuc = Vec::with_capacity(bilgi.AceCount as usize);
        for i in 0..bilgi.AceCount {
            let mut p: *mut c_void = null_mut();
            if GetAce(dacl, i, &mut p) == 0 {
                continue;
            }
            let baslik = &*(p as *const ACE_HEADER);
            // İzin ve ret girdilerinin gövdesi aynıdır: maske ve SID.
            let sid = if baslik.AceType <= 1 {
                let girdi = p as *const ACCESS_ALLOWED_ACE;
                Some(Sid::kopyala((&raw const (*girdi).SidStart) as PSID))
            } else {
                None
            };
            let maske = if baslik.AceType <= 1 {
                (*(p as *const ACCESS_ALLOWED_ACE)).Mask
            } else {
                0
            };
            sonuc.push(Ace {
                tur: baslik.AceType,
                bayraklar: baslik.AceFlags,
                maske,
                sid,
            });
        }
        sonuc
    }
}

/// Nesnenin DACL'indeki girdiler.
pub fn aceler_oku(yol: &Path) -> io::Result<Vec<Ace>> {
    Ok(Tanimlayici::oku(yol)?.aceler())
}

/// Nesnenin DACL'i mirası kesilmiş (korumalı) mi?
pub fn mirasi_kesik_mi(yol: &Path) -> io::Result<bool> {
    Ok(Tanimlayici::oku(yol)?.denetim() & SE_DACL_PROTECTED != 0)
}

/// Nesnenin DACL'i SDDL olarak (`D:…`); geri almada yazılır.
pub fn dacl_sddl(yol: &Path) -> io::Result<String> {
    let t = Tanimlayici::oku(yol)?;
    let mut metin: *mut u16 = null_mut();
    // SAFETY: tanımlayıcı geçerlidir; dize LocalFree ile bırakılır.
    unsafe {
        if ConvertSecurityDescriptorToStringSecurityDescriptorW(
            t.sd.0,
            SDDL_REVISION_1,
            DACL_SECURITY_INFORMATION,
            &mut metin,
            null_mut(),
        ) == 0
        {
            return Err(son_hata(
                "ConvertSecurityDescriptorToStringSecurityDescriptorW",
            ));
        }
        let _bellek = Yerel(metin as *mut c_void);
        Ok(genisten(metin))
    }
}

/// SDDL biçimindeki DACL'den pakete ait bütün ACE'leri çıkarır; diskteki
/// nesneye dokunmaz. İç içe alanın önceki DACL kaydını dış alan kaldırılırken
/// temizlemek için kullanılır.
pub fn dacl_sddl_sidini_kaldir(sddl: &str, sid: &Sid) -> io::Result<String> {
    let mut sd: PSECURITY_DESCRIPTOR = null_mut();
    // SAFETY: dize NUL ile biter; dönen tanımlayıcı LocalFree ile bırakılır.
    unsafe {
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            genis(sddl).as_ptr(),
            SDDL_REVISION_1,
            &mut sd,
            null_mut(),
        ) == 0
        {
            return Err(son_hata(
                "ConvertStringSecurityDescriptorToSecurityDescriptorW",
            ));
        }
    }
    let sd = Yerel(sd);
    let mut var = 0;
    let mut varsayilan = 0;
    let mut dacl: *mut ACL = null_mut();
    let mut denetim = 0u16;
    let mut surum = 0u32;
    // SAFETY: SDDL'den çözülen tanımlayıcı geçerlidir; DACL onun içini gösterir.
    unsafe {
        if GetSecurityDescriptorDacl(sd.0, &mut var, &mut dacl, &mut varsayilan) == 0 {
            return Err(son_hata("GetSecurityDescriptorDacl"));
        }
        if GetSecurityDescriptorControl(sd.0, &mut denetim, &mut surum) == 0 {
            return Err(son_hata("GetSecurityDescriptorControl"));
        }
    }
    if var == 0 || dacl.is_null() {
        return Ok(sddl.to_string());
    }

    // SAFETY: `ACL_SIZE_INFORMATION`, Win32'in çıktı olarak doldurduğu düz bir yapıdır.
    let mut bilgi: ACL_SIZE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: DACL, SDDL ayrıştırıcısının ürettiği geçerli ACL'dir.
    unsafe {
        if GetAclInformation(
            dacl,
            (&raw mut bilgi).cast(),
            size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        ) == 0
        {
            return Err(son_hata("GetAclInformation"));
        }
    }
    // Yeni ACL, eski girdilerin tümünü (eşleşen SID'e ait açık ve miraslı
    // izinler hariç) aynı sırada kopyalayacak kadar yer alır.
    // SAFETY: DACL geçerlidir; AclSize bütün ACE'leri içerir ve DWORD hizalıdır.
    let (_yeni_bellek, yeni) = unsafe {
        let boyut = u32::from((*dacl).AclSize);
        let mut bellek = vec![0u64; (boyut as usize).div_ceil(size_of::<u64>()).max(1)];
        let yeni = bellek.as_mut_ptr().cast::<ACL>();
        if InitializeAcl(yeni, boyut, u32::from((*dacl).AclRevision)) == 0 {
            return Err(son_hata("InitializeAcl"));
        }
        for sira in 0..bilgi.AceCount {
            let mut ace: *mut c_void = null_mut();
            if GetAce(dacl, sira, &mut ace) == 0 {
                return Err(son_hata("GetAce"));
            }
            let baslik = &*(ace as *const ACE_HEADER);
            let ayni_sid = if baslik.AceType <= 1 {
                let izin = ace as *const ACCESS_ALLOWED_ACE;
                // SAFETY: GetAce geçerli, türü izin veya ret olan ACE'i verir;
                // iki ACE biçiminin SID alanı SidStart'ta başlar.
                Sid::kopyala((&raw const (*izin).SidStart) as PSID) == *sid
            } else {
                false
            };
            if ayni_sid {
                continue;
            }
            if AddAce(
                yeni,
                u32::from((*dacl).AclRevision),
                u32::MAX,
                ace,
                u32::from(baslik.AceSize),
            ) == 0
            {
                return Err(son_hata("AddAce"));
            }
        }
        (bellek, yeni)
    };
    // SAFETY: `SECURITY_DESCRIPTOR` Win32 başlatıcısı tarafından doldurulacak düz bir yapıdır.
    let mut yeni_sd: SECURITY_DESCRIPTOR = unsafe { std::mem::zeroed() };
    // SAFETY: yeni_sd yığında, ACL `yeni_bellek` içinde yaşar; çağrılar boyunca geçerlidir.
    unsafe {
        let p = (&raw mut yeni_sd).cast::<c_void>();
        if InitializeSecurityDescriptor(p, SECURITY_DESCRIPTOR_REVISION) == 0 {
            return Err(son_hata("InitializeSecurityDescriptor"));
        }
        if SetSecurityDescriptorDacl(p, 1, yeni as *const ACL, varsayilan) == 0 {
            return Err(son_hata("SetSecurityDescriptorDacl"));
        }
        let ilgili = SE_DACL_AUTO_INHERIT_REQ | SE_DACL_AUTO_INHERITED | SE_DACL_PROTECTED;
        if SetSecurityDescriptorControl(p, ilgili, denetim & ilgili) == 0 {
            return Err(son_hata("SetSecurityDescriptorControl"));
        }
    }

    let mut metin: *mut u16 = null_mut();
    // SAFETY: tanımlayıcı geçerlidir; dönen dize LocalFree ile bırakılır.
    unsafe {
        if ConvertSecurityDescriptorToStringSecurityDescriptorW(
            (&raw mut yeni_sd).cast(),
            SDDL_REVISION_1,
            DACL_SECURITY_INFORMATION,
            &mut metin,
            null_mut(),
        ) == 0
        {
            return Err(son_hata(
                "ConvertSecurityDescriptorToStringSecurityDescriptorW",
            ));
        }
        let _bellek = Yerel(metin as *mut c_void);
        Ok(genisten(metin))
    }
}

fn erisim(izin: &Izin, kip: i32) -> EXPLICIT_ACCESS_W {
    EXPLICIT_ACCESS_W {
        grfAccessPermissions: izin.maske,
        grfAccessMode: kip,
        grfInheritance: izin.miras,
        Trustee: TRUSTEE_W {
            pMultipleTrustee: null_mut(),
            MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
            TrusteeForm: TRUSTEE_IS_SID,
            TrusteeType: TRUSTEE_IS_UNKNOWN,
            ptstrName: izin.sid.ptr() as *mut u16,
        },
    }
}

/// Girdileri `eski` DACL'e ekler ya da eski DACL'den çıkarır; yeni DACL
/// LocalFree ile bırakılır.
fn yeni_dacl(eski: *const ACL, girdiler: &[EXPLICIT_ACCESS_W]) -> io::Result<Yerel> {
    let mut yeni: *mut ACL = null_mut();
    // SAFETY: girdilerin SID'leri çağrı boyunca yaşar; eski DACL boş ya da
    // geçerlidir.
    let kod =
        unsafe { SetEntriesInAclW(girdiler.len() as u32, girdiler.as_ptr(), eski, &mut yeni) };
    if kod != ERROR_SUCCESS {
        return Err(win32_hatasi("SetEntriesInAclW", kod));
    }
    Ok(Yerel(yeni as *mut c_void))
}

fn dacl_yaz(yol: &Path, dacl: *const ACL, korumali: bool) -> io::Result<()> {
    let koruma = if korumali {
        PROTECTED_DACL_SECURITY_INFORMATION
    } else {
        UNPROTECTED_DACL_SECURITY_INFORMATION
    };
    // SAFETY: yol NUL ile biter; DACL geçerlidir. Değişiklik alttakilere
    // yayılır (mirasla gelen girdiler yeniden hesaplanır).
    let kod = unsafe {
        SetNamedSecurityInfoW(
            genis(yol).as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | koruma,
            null_mut(),
            null_mut(),
            dacl,
            null(),
        )
    };
    if kod != ERROR_SUCCESS {
        return Err(win32_hatasi(
            &format!("SetNamedSecurityInfoW({})", yol.display()),
            kod,
        ));
    }
    Ok(())
}

/// Reparse noktası izlenmeden açılmış bir dosya ya da dizin; sahibi ve DACL'i
/// okunup yazılabilir (güvenlik analizi Bulgu 6).
pub struct Tutamac(OwnedHandle);

impl Tutamac {
    fn ham(&self) -> HANDLE {
        self.0.as_raw_handle() as HANDLE
    }
}

/// `yol`'u reparse noktasını izlemeden açar (`FILE_FLAG_OPEN_REPARSE_POINT`).
/// Nesne bir reparse noktasıysa (junction, sembolik bağlantı) ya da türü
/// beklenen değilse hata döner. Tutamaç sahibi okumaya ve DACL yazmaya açıktır.
pub fn yeniden_ayristirma_izlemeden_ac(yol: &Path, dizin: bool) -> io::Result<Tutamac> {
    let mut bayraklar = FILE_FLAG_OPEN_REPARSE_POINT;
    if dizin {
        bayraklar |= FILE_FLAG_BACKUP_SEMANTICS;
    }
    // SAFETY: yol NUL ile biter; geçerli tutamaç hemen OwnedHandle'a geçer.
    let h = unsafe {
        CreateFileW(
            genis(yol).as_ptr(),
            READ_CONTROL | WRITE_DAC,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            OPEN_EXISTING,
            bayraklar,
            null_mut(),
        )
    };
    if h == INVALID_HANDLE_VALUE {
        return Err(son_hata(&format!("CreateFileW({})", yol.display())));
    }
    // SAFETY: h geçerli ve sahibi yalnızca bu değerdir.
    let t = Tutamac(unsafe { OwnedHandle::from_raw_handle(h as RawHandle) });
    // SAFETY: yapı yalnızca tamsayı alanlarından oluşur; sıfır geçerli bir
    // değerdir.
    let mut bilgi: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: tutamaç geçerlidir; yapı yığında ve yazılabilir.
    if unsafe { GetFileInformationByHandle(t.ham(), &mut bilgi) } == 0 {
        return Err(son_hata("GetFileInformationByHandle"));
    }
    if bilgi.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "{}: bağlantı (reparse noktası); güvenlik için izlenmez",
                yol.display()
            ),
        ));
    }
    if (bilgi.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0) != dizin {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: beklenen türde değil", yol.display()),
        ));
    }
    Ok(t)
}

/// Nesnenin sahibi.
pub fn sahip(t: &Tutamac) -> io::Result<Sid> {
    let mut sahip: PSID = null_mut();
    let mut sd: PSECURITY_DESCRIPTOR = null_mut();
    // SAFETY: tutamaç READ_CONTROL ile açık; tanımlayıcı LocalFree ile
    // bırakılır ve `sahip` onun içini gösterir, kopyalanana kadar yaşar.
    unsafe {
        let kod = GetSecurityInfo(
            t.ham(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            &mut sahip,
            null_mut(),
            null_mut(),
            null_mut(),
            &mut sd,
        );
        if kod != ERROR_SUCCESS {
            return Err(win32_hatasi("GetSecurityInfo", kod));
        }
        let _bellek = Yerel(sd);
        if sahip.is_null() {
            return Err(io::Error::other("nesnenin sahibi yok"));
        }
        Ok(Sid::kopyala(sahip))
    }
}

/// Mirassız (korumalı) DACL'i tutamaç üzerinden, yalnızca nesnenin kendisine
/// yazar (`SetKernelObjectSecurity`). `SetNamedSecurityInfoW` yola göre
/// çalışır ve reparse noktasını izler; `SetSecurityInfo` da mirası alt ağaca
/// yayarken araya konmuş bir bağlantıyı izleyebilirdi. Burada hiçbir yol
/// izlenmez; alt nesnelerin mirası yeniden hesaplanmaz.
pub fn dacl_yaz_tutamacla(t: &Tutamac, izinler: &[Izin]) -> io::Result<()> {
    let girdiler: Vec<EXPLICIT_ACCESS_W> =
        izinler.iter().map(|i| erisim(i, GRANT_ACCESS)).collect();
    let yeni = yeni_dacl(null(), &girdiler)?;
    // SAFETY: tanımlayıcı yığındadır ve geçerli ACL'i gösterir; ikisi de
    // çağrı dönene kadar yaşar. Tutamaç WRITE_DAC ile açıktır.
    unsafe {
        let mut sd: SECURITY_DESCRIPTOR = std::mem::zeroed();
        let p = (&raw mut sd).cast::<c_void>();
        if InitializeSecurityDescriptor(p, SECURITY_DESCRIPTOR_REVISION) == 0 {
            return Err(son_hata("InitializeSecurityDescriptor"));
        }
        if SetSecurityDescriptorDacl(p, 1, yeni.0 as *const ACL, 0) == 0 {
            return Err(son_hata("SetSecurityDescriptorDacl"));
        }
        let bitler = SE_DACL_AUTO_INHERIT_REQ | SE_DACL_PROTECTED;
        if SetSecurityDescriptorControl(p, bitler, bitler) == 0 {
            return Err(son_hata("SetSecurityDescriptorControl"));
        }
        if SetKernelObjectSecurity(t.ham(), DACL_SECURITY_INFORMATION, p) == 0 {
            return Err(son_hata("SetKernelObjectSecurity"));
        }
    }
    Ok(())
}

/// Dizini, oluştuğu anda mirassız ve yalnızca verilen izinlerle oluşturur:
/// arada başka bir kullanıcının içine bir şey koyabileceği an olmaz. Dizin
/// zaten varsa dokunmaz ve `AlreadyExists` döner.
pub fn korumali_dizin_olustur(yol: &Path, izinler: &[Izin]) -> io::Result<()> {
    let girdiler: Vec<EXPLICIT_ACCESS_W> =
        izinler.iter().map(|i| erisim(i, GRANT_ACCESS)).collect();
    let dacl = yeni_dacl(null(), &girdiler)?;
    // SAFETY: tanımlayıcı ve ACL CreateDirectoryW dönene kadar yaşar; yol NUL
    // ile biter.
    unsafe {
        let mut sd: SECURITY_DESCRIPTOR = std::mem::zeroed();
        let p = (&raw mut sd).cast::<c_void>();
        if InitializeSecurityDescriptor(p, SECURITY_DESCRIPTOR_REVISION) == 0 {
            return Err(son_hata("InitializeSecurityDescriptor"));
        }
        if SetSecurityDescriptorDacl(p, 1, dacl.0 as *const ACL, 0) == 0 {
            return Err(son_hata("SetSecurityDescriptorDacl"));
        }
        if SetSecurityDescriptorControl(p, SE_DACL_PROTECTED, SE_DACL_PROTECTED) == 0 {
            return Err(son_hata("SetSecurityDescriptorControl"));
        }
        let nitelik = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: p,
            bInheritHandle: 0,
        };
        if CreateDirectoryW(genis(yol).as_ptr(), &nitelik) == 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

/// Nesneye izin ekler. Mirası olan izin bütün alt ağaca yayılır; mirası
/// kesilmiş alt nesneler onu almaz. Mirassız izin yalnızca nesneye yazılır.
pub fn izin_ekle(yol: &Path, izin: &Izin) -> io::Result<()> {
    if izin.miras == MIRAS_YOK {
        return kok_dacl_degistir(yol, erisim(izin, GRANT_ACCESS));
    }
    let t = Tanimlayici::oku(yol)?;
    let yeni = yeni_dacl(t.dacl, &[erisim(izin, GRANT_ACCESS)])?;
    dacl_yaz(
        yol,
        yeni.0 as *const ACL,
        t.denetim() & SE_DACL_PROTECTED != 0,
    )
}

/// SID'in nesnedeki açık izinlerini siler; mirasla gelenler kaynağından
/// silinince kalkar.
pub fn izni_kaldir(yol: &Path, sid: &Sid) -> io::Result<()> {
    let t = Tanimlayici::oku(yol)?;
    let var = t
        .aceler()
        .iter()
        .any(|a| !a.miras_mi() && a.sid.as_ref() == Some(sid));
    if !var {
        return Ok(());
    }
    let girdi = Izin {
        sid: sid.clone(),
        maske: 0,
        miras: MIRAS_YOK,
    };
    let yeni = yeni_dacl(t.dacl, &[erisim(&girdi, REVOKE_ACCESS)])?;
    dacl_yaz(
        yol,
        yeni.0 as *const ACL,
        t.denetim() & SE_DACL_PROTECTED != 0,
    )
}

/// Nesnenin mirasını keser; yeni DACL yalnızca verilen izinlerden oluşur.
pub fn mirasi_kes(yol: &Path, izinler: &[Izin]) -> io::Result<()> {
    let girdiler: Vec<EXPLICIT_ACCESS_W> =
        izinler.iter().map(|i| erisim(i, GRANT_ACCESS)).collect();
    let yeni = yeni_dacl(null(), &girdiler)?;
    dacl_yaz(yol, yeni.0 as *const ACL, true)
}

/// SDDL'den okunan DACL'i nesneye yazar. Tanımlayıcı mirası kesilmiş değilse
/// miras yeniden açılır ve mirasla gelen girdiler üstten yeniden hesaplanır.
pub fn dacl_geri_yaz(yol: &Path, sddl: &str) -> io::Result<()> {
    let mut sd: PSECURITY_DESCRIPTOR = null_mut();
    // SAFETY: dize NUL ile biter; tanımlayıcı LocalFree ile bırakılır.
    unsafe {
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            genis(sddl).as_ptr(),
            SDDL_REVISION_1,
            &mut sd,
            null_mut(),
        ) == 0
        {
            return Err(son_hata(
                "ConvertStringSecurityDescriptorToSecurityDescriptorW",
            ));
        }
    }
    let sd = Yerel(sd);
    let mut var = 0;
    let mut varsayilan = 0;
    let mut dacl: *mut ACL = null_mut();
    let mut denetim = 0u16;
    let mut surum = 0u32;
    // SAFETY: tanımlayıcı geçerlidir; DACL onun içini gösterir.
    unsafe {
        if GetSecurityDescriptorDacl(sd.0, &mut var, &mut dacl, &mut varsayilan) == 0 {
            return Err(son_hata("GetSecurityDescriptorDacl"));
        }
        GetSecurityDescriptorControl(sd.0, &mut denetim, &mut surum);
    }
    dacl_yaz(yol, dacl, denetim & SE_DACL_PROTECTED != 0)
}

/// Sağlam ACE kaydındaki tek bir izinle eşleşen açık ACE'i kaldırır.
/// Aynı SID, maske ve miras bayraklarıyla birden çok ACE varsa kaynağı
/// ayırt edilemediğinden hiçbirini kaldırmaz.
pub fn kayitli_izni_kaldir(yol: &Path, sid: &Sid, maske: u32, miras: u32) -> io::Result<bool> {
    const ACE_MIRAS_BAYRAKLARI: u32 = 0x0F;

    let t = Tanimlayici::oku(yol)?;
    if t.dacl.is_null() {
        return Ok(false);
    }
    let girdiler = t.aceler();
    let eslesen: Vec<usize> = girdiler
        .iter()
        .enumerate()
        .filter_map(|(indeks, ace)| {
            (ace.tur == 0
                && !ace.miras_mi()
                && ace.maske == maske
                && u32::from(ace.bayraklar) & ACE_MIRAS_BAYRAKLARI == miras
                && ace.sid.as_ref() == Some(sid))
            .then_some(indeks)
        })
        .collect();
    let Some(&silinecek) = eslesen.first() else {
        if girdiler.iter().any(|ace| ace.sid.as_ref() == Some(sid)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "kayıtlı ACE yok ama aynı SID için başka ACL girdisi var; geri alma belirsiz",
            ));
        }
        return Ok(false);
    };
    if eslesen.len() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "birden çok aynı ACE bulundu; belirsiz izin geri alınmadı",
        ));
    }

    // SAFETY: ACL_SIZE_INFORMATION düz bir çıktı yapısıdır ve tüm bitlerinin
    // sıfır olması geçerli başlangıç değeridir; Win32 çağrısı alanları doldurur.
    let mut bilgi: ACL_SIZE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: `t.dacl` geçerli ACL'dir; `bilgi` yeterli boyutta çıktı yapısıdır.
    unsafe {
        if GetAclInformation(
            t.dacl,
            (&raw mut bilgi).cast(),
            size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        ) == 0
        {
            return Err(son_hata("GetAclInformation"));
        }
    }
    // SAFETY: `AclSize` bütün ACE'leri kapsar; ACL boyutu DWORD hizalıdır.
    let (_yeni_bellek, yeni) = unsafe {
        let boyut = u32::from((*t.dacl).AclSize);
        let mut bellek = vec![0u64; (boyut as usize).div_ceil(size_of::<u64>()).max(1)];
        let yeni = bellek.as_mut_ptr().cast::<ACL>();
        if InitializeAcl(yeni, boyut, u32::from((*t.dacl).AclRevision)) == 0 {
            return Err(son_hata("InitializeAcl"));
        }
        for sira in 0..bilgi.AceCount {
            if sira as usize == silinecek {
                continue;
            }
            let mut ace: *mut c_void = null_mut();
            if GetAce(t.dacl, sira, &mut ace) == 0 {
                return Err(son_hata("GetAce"));
            }
            let baslik = &*(ace as *const ACE_HEADER);
            if AddAce(
                yeni,
                u32::from((*t.dacl).AclRevision),
                u32::MAX,
                ace,
                u32::from(baslik.AceSize),
            ) == 0
            {
                return Err(son_hata("AddAce"));
            }
        }
        (bellek, yeni)
    };
    if miras == MIRAS_YOK {
        // Mirassız ACE'nin silinmesi alt nesneleri etkilemez.
        dacl_yaymadan_yaz(yol, yeni, t.denetim())?;
    } else {
        dacl_yaz(yol, yeni, t.denetim() & SE_DACL_PROTECTED != 0)?;
    }
    Ok(true)
}

/// Nesnenin mevcut DACL'ini koruyup üst dizinden mirası yeniden açar.
/// Bozuk ACE kaydının taramalı geri almasında kullanılır (§9.4).
pub fn mirasi_ac(yol: &Path) -> io::Result<()> {
    let t = Tanimlayici::oku(yol)?;
    dacl_yaz(yol, t.dacl, false)
}

/// Sürücü kökünün DACL'ini okur (`GetFileSecurityW`).
fn kok_tanimlayicisi(kok: &Path) -> io::Result<Vec<u64>> {
    let yol = genis(kok);
    let mut gerekli = 0u32;
    // SAFETY: önce boyut sorulur, sonra 8 bayt hizalı, yeterli tampon verilir.
    unsafe {
        GetFileSecurityW(
            yol.as_ptr(),
            DACL_SECURITY_INFORMATION,
            null_mut(),
            0,
            &mut gerekli,
        );
        if GetLastError() != ERROR_INSUFFICIENT_BUFFER {
            return Err(son_hata("GetFileSecurityW"));
        }
        let mut tampon = vec![0u64; (gerekli as usize).div_ceil(8)];
        if GetFileSecurityW(
            yol.as_ptr(),
            DACL_SECURITY_INFORMATION,
            tampon.as_mut_ptr() as PSECURITY_DESCRIPTOR,
            (tampon.len() * 8) as u32,
            &mut gerekli,
        ) == 0
        {
            return Err(son_hata("GetFileSecurityW"));
        }
        Ok(tampon)
    }
}

/// Kökte SID'in listeleme izni var mı?
pub fn kok_izni_var_mi(kok: &Path, sid: &Sid) -> io::Result<bool> {
    let tampon = kok_tanimlayicisi(kok)?;
    let mut var = 0;
    let mut varsayilan = 0;
    let mut dacl: *mut ACL = null_mut();
    // SAFETY: tampon GetFileSecurityW'nin yazdığı tanımlayıcıdır.
    let girdiler = unsafe {
        if GetSecurityDescriptorDacl(
            tampon.as_ptr() as PSECURITY_DESCRIPTOR,
            &mut var,
            &mut dacl,
            &mut varsayilan,
        ) == 0
        {
            return Err(son_hata("GetSecurityDescriptorDacl"));
        }
        aceler(dacl)
    };
    Ok(girdiler.iter().any(|a| {
        a.tur == 0
            && u32::from(a.bayraklar) & INHERIT_ONLY_ACE == 0
            && a.maske & MASKE_LISTELE == MASKE_LISTELE
            && a.sid.as_ref() == Some(sid)
    }))
}

/// Dizinin DACL'inde, verilen SID'lerden birine nesnenin kendisinde geçerli
/// okuma ve çalıştırma izni var mı?
pub fn okuma_izni_var_mi(yol: &Path, sidler: &[Sid]) -> io::Result<bool> {
    Ok(aceler_oku(yol)?.iter().any(|a| {
        a.tur == 0
            && u32::from(a.bayraklar) & INHERIT_ONLY_ACE == 0
            && a.maske & MASKE_OKU == MASKE_OKU
            && a.sid.as_ref().is_some_and(|s| sidler.contains(s))
    }))
}

/// Geçerli kullanıcı nesnenin DACL'ini değiştirebilir mi? Yalnızca
/// `WRITE_DAC` ile açmayı dener; hiçbir şey yazılmaz.
pub fn dacl_yazilabilir_mi(yol: &Path) -> io::Result<bool> {
    // SAFETY: yol NUL ile biter; geçerli tutamaç hemen kapatılır.
    let h = unsafe {
        CreateFileW(
            genis(yol).as_ptr(),
            WRITE_DAC,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            null_mut(),
        )
    };
    if h == INVALID_HANDLE_VALUE {
        let hata = io::Error::last_os_error();
        return if hata.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32) {
            Ok(false)
        } else {
            Err(hata)
        };
    }
    // SAFETY: h geçerli ve yalnızca burada kapatılır.
    unsafe { CloseHandle(h) };
    Ok(true)
}

/// Nesnenin DACL'ine tek girdi uygular ve yalnızca nesnenin kendisini yazar
/// (`dacl_yaymadan_yaz`). Kök izni ve mirassız izinler için kullanılır.
fn kok_dacl_degistir(kok: &Path, girdi: EXPLICIT_ACCESS_W) -> io::Result<()> {
    let tampon = kok_tanimlayicisi(kok)?;
    let eski_sd = tampon.as_ptr() as PSECURITY_DESCRIPTOR;
    let mut var = 0;
    let mut varsayilan = 0;
    let mut dacl: *mut ACL = null_mut();
    let mut denetim = 0u16;
    let mut surum = 0u32;
    // SAFETY: tampon GetFileSecurityW'nin yazdığı tanımlayıcıdır.
    unsafe {
        if GetSecurityDescriptorDacl(eski_sd, &mut var, &mut dacl, &mut varsayilan) == 0 {
            return Err(son_hata("GetSecurityDescriptorDacl"));
        }
        GetSecurityDescriptorControl(eski_sd, &mut denetim, &mut surum);
    }
    let yeni = yeni_dacl(dacl, &[girdi])?;
    dacl_yaymadan_yaz(kok, yeni.0 as *const ACL, denetim)
}

/// DACL'i yalnızca nesnenin kendisine yazar (`SetFileSecurityW`).
/// `SetNamedSecurityInfoW` her yazımda mirası bütün alt ağaca yeniden yayar:
/// kökte bütün sürücü, `C:\Users` gibi bir üst dizinde bütün kullanıcı ağacı
/// gezilir. Mirassız bir ACE'nin eklenmesi ya da silinmesi alt nesneleri
/// etkilemediğinden yayma gerekmez. `SE_DACL_AUTO_INHERIT_REQ` verilmezse
/// `SE_DACL_AUTO_INHERITED` bayrağı düşer (2c'de `D:PAI` → `D:P`).
fn dacl_yaymadan_yaz(yol: &Path, dacl: *const ACL, denetim: u16) -> io::Result<()> {
    // SAFETY: yeni tanımlayıcı yığındadır ve verilen geçerli ACL'i gösterir;
    // ikisi de SetFileSecurityW dönene kadar yaşar.
    unsafe {
        let mut sd: SECURITY_DESCRIPTOR = std::mem::zeroed();
        let p = (&raw mut sd).cast::<c_void>();
        if InitializeSecurityDescriptor(p, SECURITY_DESCRIPTOR_REVISION) == 0 {
            return Err(son_hata("InitializeSecurityDescriptor"));
        }
        if SetSecurityDescriptorDacl(p, 1, dacl, 0) == 0 {
            return Err(son_hata("SetSecurityDescriptorDacl"));
        }
        let ilgili = SE_DACL_AUTO_INHERIT_REQ | SE_DACL_AUTO_INHERITED | SE_DACL_PROTECTED;
        let bitler =
            SE_DACL_AUTO_INHERIT_REQ | (denetim & (SE_DACL_AUTO_INHERITED | SE_DACL_PROTECTED));
        if SetSecurityDescriptorControl(p, ilgili, bitler) == 0 {
            return Err(son_hata("SetSecurityDescriptorControl"));
        }
        if SetFileSecurityW(genis(yol).as_ptr(), DACL_SECURITY_INFORMATION, p) == 0 {
            return Err(son_hata(&format!("SetFileSecurityW({})", yol.display())));
        }
    }
    Ok(())
}

/// Köke SID için listeleme izni verir; miras yok (§9.2). Yönetici gerekir.
pub fn kok_izni_ver(kok: &Path, sid: &Sid) -> io::Result<()> {
    let izin = Izin {
        sid: sid.clone(),
        maske: MASKE_LISTELE,
        miras: MIRAS_YOK,
    };
    kok_dacl_degistir(kok, erisim(&izin, GRANT_ACCESS))
}

/// Kökteki izni kaldırır. Yönetici gerekir.
pub fn kok_izni_kaldir(kok: &Path, sid: &Sid) -> io::Result<()> {
    let izin = Izin {
        sid: sid.clone(),
        maske: 0,
        miras: MIRAS_YOK,
    };
    kok_dacl_degistir(kok, erisim(&izin, REVOKE_ACCESS))
}

/// ACL taşıyan sabit disklerin kökleri (`C:\`, `D:\` …); FAT32 ve exFAT
/// sayılmaz (§9.6).
pub fn sabit_diskler() -> Vec<PathBuf> {
    // SAFETY: bağımsız çağrılar; dizeler NUL ile biter.
    let maske = unsafe { GetLogicalDrives() };
    (0..26u8)
        .filter(|i| maske & (1 << i) != 0)
        .map(|i| PathBuf::from(format!("{}:\\", char::from(b'A' + i))))
        .filter(|kok| {
            let g = genis(kok);
            // SAFETY: dize NUL ile biter; çıktılar yerel değişkenlerdir.
            unsafe {
                if GetDriveTypeW(g.as_ptr()) != DRIVE_FIXED {
                    return false;
                }
                let mut bayraklar = 0u32;
                GetVolumeInformationW(
                    g.as_ptr(),
                    null_mut(),
                    0,
                    null_mut(),
                    null_mut(),
                    &mut bayraklar,
                    null_mut(),
                    0,
                ) != 0
                    && bayraklar & FILE_PERSISTENT_ACLS != 0
            }
        })
        .collect()
}

/// `%ProgramData%`; ortam değişkeninden değil kabuğun bilinen dizininden
/// okunur: yükseltilmiş yardımcının ortamı çağıranın elindedir.
pub fn program_data() -> io::Result<PathBuf> {
    bilinen_dizin(&FOLDERID_ProgramData)
}

/// Kullanıcı profillerinin dizini (`C:\Users`). Profil altındaki bir çalışma
/// alanının yolu çözülürken kap onu listeler (A11); dizin yöneticiye ait
/// olduğundan izni kurulum verir (§9.2).
pub fn profiller_dizini() -> io::Result<PathBuf> {
    bilinen_dizin(&FOLDERID_UserProfiles)
}

/// Program Files dizinleri (64 ve 32 bit); bulunabilenler. Yalnızca
/// yöneticinin yazabildiği yerlerdir (aşama 2 tasarımı §9.8).
pub fn program_files_dizinleri() -> Vec<PathBuf> {
    [&FOLDERID_ProgramFiles, &FOLDERID_ProgramFilesX86]
        .into_iter()
        .filter_map(|k| bilinen_dizin(k).ok())
        .collect()
}

fn bilinen_dizin(kimlik: &windows_sys::core::GUID) -> io::Result<PathBuf> {
    let mut p: *mut u16 = null_mut();
    // SAFETY: dönen dize CoTaskMemFree ile bırakılır.
    unsafe {
        let hr = SHGetKnownFolderPath(kimlik, 0, null_mut(), &mut p);
        if hr != 0 {
            if !p.is_null() {
                CoTaskMemFree(p as *const c_void);
            }
            return Err(hresult_hatasi("SHGetKnownFolderPath", hr));
        }
        let yol = PathBuf::from(genisten(p));
        CoTaskMemFree(p as *const c_void);
        Ok(yol)
    }
}

/// `\\?\C:\…` biçimindeki yolu `C:\…`'ye, `\\?\UNC\…`'yu `\\…`'ya çevirir.
/// `canonicalize` Windows'ta uzun yol önekini ekler; cmd.exe bu önekli
/// çalışma dizinini UNC sanıp reddediyor.
pub fn sade(yol: &Path) -> PathBuf {
    let metin = yol.as_os_str().to_string_lossy();
    if let Some(kalan) = metin.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{kalan}"));
    }
    match metin.strip_prefix(r"\\?\") {
        Some(kalan) if kalan.as_bytes().get(1) == Some(&b':') => PathBuf::from(kalan),
        _ => yol.to_path_buf(),
    }
}

/// Yolun bulunduğu sürücünün kökü (`C:\`).
pub fn surucu_koku(yol: &Path) -> Option<PathBuf> {
    use std::path::{Component, Prefix};
    match yol.components().next()? {
        Component::Prefix(p) => match p.kind() {
            Prefix::Disk(h) | Prefix::VerbatimDisk(h) => {
                Some(PathBuf::from(format!("{}:\\", char::from(h))))
            }
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yetenek_sidi_addan_ayni_turer() {
        let a = arac_zinciri_sidi().unwrap();
        let b = yetenek_sidi(ARAC_ZINCIRI_YETENEGI).unwrap();
        assert_eq!(a, b);
        assert!(a.metin().starts_with("S-1-15-3-1024-"), "{a:?}");
        assert_ne!(a, yetenek_sidi("baskaYetenek").unwrap());
    }

    #[test]
    fn kullanici_sidi_ve_yukseltme_okunur() {
        assert!(kullanici_sidi().unwrap().metin().starts_with("S-1-5-"));
        yukseltilmis_mi().unwrap();
    }

    #[test]
    fn sid_metinden_ve_metne() {
        let s = Sid::metinden(YONETICILER).unwrap();
        assert_eq!(s.metin(), YONETICILER);
        assert!(Sid::metinden("S-x").is_err());
    }

    #[test]
    fn profil_sidi_addan_turer_ve_olusturmadan_ayni() {
        let ad = format!("Kalfa.Test.{}", std::process::id());
        let turetilen = profil_sidi(&ad).unwrap();
        let olusan = profil_olustur(&ad, "Kalfa testi").unwrap();
        assert_eq!(turetilen, olusan);
        assert_eq!(profil_olustur(&ad, "Kalfa testi").unwrap(), olusan);
        assert!(olusan.metin().starts_with("S-1-15-2-"));
        profil_sil(&ad).unwrap();
        profil_sil(&ad).unwrap();
    }

    #[test]
    fn izin_eklenir_miras_kesilir_ve_geri_yazilir() {
        let d = tempfile::tempdir().unwrap();
        let alt = d.path().join("alt");
        std::fs::create_dir(&alt).unwrap();
        let dosya = alt.join("a.txt");
        std::fs::write(&dosya, "a").unwrap();
        let sid = yetenek_sidi("kalfaTestYetenegi").unwrap();
        let once = dacl_sddl(&dosya).unwrap();

        izin_ekle(
            d.path(),
            &Izin {
                sid: sid.clone(),
                maske: MASKE_DEGISTIR,
                miras: MIRAS,
            },
        )
        .unwrap();
        let miras = aceler_oku(&dosya).unwrap();
        assert!(
            miras
                .iter()
                .any(|a| a.miras_mi() && a.sid.as_ref() == Some(&sid)),
            "{miras:?}"
        );

        mirasi_kes(
            &dosya,
            &[Izin {
                sid: kullanici_sidi().unwrap(),
                maske: MASKE_TAM,
                miras: MIRAS_YOK,
            }],
        )
        .unwrap();
        assert!(mirasi_kesik_mi(&dosya).unwrap());
        assert!(
            !aceler_oku(&dosya)
                .unwrap()
                .iter()
                .any(|a| a.sid.as_ref() == Some(&sid))
        );
        std::fs::read(&dosya).unwrap();

        dacl_geri_yaz(&dosya, &once).unwrap();
        assert!(!mirasi_kesik_mi(&dosya).unwrap());
        assert!(
            aceler_oku(&dosya)
                .unwrap()
                .iter()
                .any(|a| a.miras_mi() && a.sid.as_ref() == Some(&sid))
        );

        izni_kaldir(d.path(), &sid).unwrap();
        assert!(
            !aceler_oku(&dosya)
                .unwrap()
                .iter()
                .any(|a| a.sid.as_ref() == Some(&sid))
        );
        assert_eq!(dacl_sddl(&dosya).unwrap(), once);
    }

    #[test]
    fn sddl_sid_temizligi_acik_ve_mirasli_ace_girdilerini_kaldirir() {
        let paket = Sid::metinden("S-1-15-2-1").unwrap();
        let baska = Sid::metinden(SISTEM).unwrap();
        let sddl = format!(
            "D:AI(A;OICI;FA;;;{})(A;ID;FA;;;{})(A;;FA;;;{})",
            paket.metin(),
            paket.metin(),
            baska.metin()
        );

        let temiz = dacl_sddl_sidini_kaldir(&sddl, &paket).unwrap();
        let mut sd: PSECURITY_DESCRIPTOR = null_mut();
        // SAFETY: SDDL NUL ile biter; dönüşteki tanımlayıcı Yerel ile bırakılır.
        unsafe {
            assert_ne!(
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    genis(&temiz).as_ptr(),
                    SDDL_REVISION_1,
                    &mut sd,
                    null_mut(),
                ),
                0
            );
        }
        let sd = Yerel(sd);
        let mut var = 0;
        let mut varsayilan = 0;
        let mut dacl: *mut ACL = null_mut();
        // SAFETY: SDDL ayrıştırıcısının döndürdüğü tanımlayıcı geçerlidir.
        unsafe {
            assert_ne!(
                GetSecurityDescriptorDacl(sd.0, &mut var, &mut dacl, &mut varsayilan),
                0
            );
            assert_ne!(var, 0);
        }
        // SAFETY: DACL yukarıda ayrıştırılan güvenlik tanımlayıcısının içindedir.
        let girdiler = unsafe { aceler(dacl) };
        assert_eq!(girdiler.len(), 1, "{girdiler:?}; {temiz}");
        assert_eq!(girdiler[0].sid.as_ref(), Some(&baska));
    }

    #[test]
    fn profiller_dizini_kullanici_profilinin_ustudur() {
        let profiller = profiller_dizini().unwrap();
        let ev = PathBuf::from(std::env::var_os("USERPROFILE").unwrap());
        let ust = ev.parent().unwrap();
        assert!(
            ust.display()
                .to_string()
                .eq_ignore_ascii_case(&profiller.display().to_string()),
            "{} / {}",
            profiller.display(),
            ev.display()
        );
    }

    /// Mirassız izin yalnızca nesneye yazılır: alt nesnelerin DACL'i ve
    /// nesnenin denetim bayrakları (`D:AI`) değişmez; kaldırma ilk hâle
    /// döndürür.
    #[test]
    fn mirassiz_izin_yalniz_nesneye_yazilir_ve_geri_alinir() {
        let d = tempfile::tempdir().unwrap();
        let ust = d.path().join("ust");
        let alt = ust.join("alt");
        std::fs::create_dir_all(&alt).unwrap();
        std::fs::write(alt.join("dosya.txt"), "x").unwrap();
        let ust_onceki = dacl_sddl(&ust).unwrap();
        let alt_onceki = dacl_sddl(&alt).unwrap();
        let sid = Sid::metinden("S-1-15-2-98").unwrap();
        izin_ekle(
            &ust,
            &Izin {
                sid: sid.clone(),
                maske: MASKE_LISTELE,
                miras: MIRAS_YOK,
            },
        )
        .unwrap();
        let ust_sonra = dacl_sddl(&ust).unwrap();
        assert!(
            ust_sonra.contains("(A;;0x1000a1;;;S-1-15-2-98)"),
            "{ust_sonra}"
        );
        assert_eq!(
            ust_sonra.split('(').next(),
            ust_onceki.split('(').next(),
            "denetim bayrakları değişti"
        );
        assert_eq!(dacl_sddl(&alt).unwrap(), alt_onceki);

        assert!(kayitli_izni_kaldir(&ust, &sid, MASKE_LISTELE, MIRAS_YOK).unwrap());
        assert_eq!(dacl_sddl(&ust).unwrap(), ust_onceki);
        assert_eq!(dacl_sddl(&alt).unwrap(), alt_onceki);
    }

    #[test]
    fn kayitli_izin_kaldirma_maskeyi_ve_mirasi_eslestirir() {
        let d = tempfile::tempdir().unwrap();
        let yol = d.path().join("izin.txt");
        let sid = Sid::metinden("S-1-15-2-99").unwrap();
        izin_ekle(
            d.path(),
            &Izin {
                sid: sid.clone(),
                maske: MASKE_TAM,
                miras: MIRAS,
            },
        )
        .unwrap();
        std::fs::write(&yol, "izin").unwrap();
        izin_ekle(
            &yol,
            &Izin {
                sid: sid.clone(),
                maske: MASKE_DEGISTIR,
                miras: MIRAS_YOK,
            },
        )
        .unwrap();
        let onceki = aceler_oku(&yol).unwrap();
        assert_eq!(
            onceki
                .iter()
                .filter(|ace| ace.sid.as_ref() == Some(&sid))
                .count(),
            2,
            "{onceki:?}"
        );

        assert!(kayitli_izni_kaldir(&yol, &sid, MASKE_DEGISTIR, MIRAS_YOK).unwrap());
        let kalan = aceler_oku(&yol).unwrap();
        let sid_izinleri: Vec<&Ace> = kalan
            .iter()
            .filter(|ace| ace.sid.as_ref() == Some(&sid))
            .collect();
        assert_eq!(
            sid_izinleri.len(),
            1,
            "yalnızca eşleşen açık ACE kaldırılmalı"
        );
        assert_eq!(sid_izinleri[0].maske, MASKE_TAM);
        assert!(sid_izinleri.iter().any(|ace| ace.miras_mi()));
    }

    #[test]
    fn kayitli_izin_kaldirma_farkli_ayni_sid_ace_varsa_belirsiz_hata_verir() {
        let d = tempfile::tempdir().unwrap();
        let yol = d.path().join("izin.txt");
        let sid = Sid::metinden("S-1-15-2-100").unwrap();
        std::fs::write(&yol, "izin").unwrap();
        izin_ekle(
            &yol,
            &Izin {
                sid: sid.clone(),
                maske: MASKE_OKU,
                miras: MIRAS_YOK,
            },
        )
        .unwrap();
        let onceki = dacl_sddl(&yol).unwrap();

        let sonuc = kayitli_izni_kaldir(&yol, &sid, MASKE_TAM, MIRAS);
        assert_eq!(sonuc.unwrap_err().kind(), io::ErrorKind::InvalidData);
        assert_eq!(dacl_sddl(&yol).unwrap(), onceki);
    }

    #[test]
    fn kayitli_izin_kaldirma_sid_ace_yoksa_idempotenttir() {
        let d = tempfile::tempdir().unwrap();
        let yol = d.path().join("izin.txt");
        std::fs::write(&yol, "izin").unwrap();
        let sid = Sid::metinden("S-1-15-2-101").unwrap();

        assert!(!kayitli_izni_kaldir(&yol, &sid, MASKE_TAM, MIRAS).unwrap());
    }

    #[test]
    fn sade_uzun_yol_onekini_kaldirir() {
        assert_eq!(sade(Path::new(r"\\?\C:\a\b")), PathBuf::from(r"C:\a\b"));
        assert_eq!(
            sade(Path::new(r"\\?\UNC\sunucu\pay")),
            PathBuf::from(r"\\sunucu\pay")
        );
        assert_eq!(sade(Path::new(r"C:\a")), PathBuf::from(r"C:\a"));
        assert_eq!(
            sade(Path::new(r"\\?\Volume{x}\a")),
            PathBuf::from(r"\\?\Volume{x}\a")
        );
    }

    /// `canonicalize`'ın gerçek çıktısı: önek kalkar, yol aynı dizini gösterir.
    #[test]
    fn sade_canonicalize_ciktisini_sadelestirir() {
        let d = tempfile::tempdir().unwrap();
        let kanonik = d.path().canonicalize().unwrap();
        assert!(kanonik.as_os_str().to_string_lossy().starts_with(r"\\?\"));
        let s = sade(&kanonik);
        assert!(!s.as_os_str().to_string_lossy().starts_with(r"\\"), "{s:?}");
        assert_eq!(s.canonicalize().unwrap(), kanonik);
    }

    #[test]
    fn surucu_koku_ayrilir() {
        assert_eq!(
            surucu_koku(Path::new(r"C:\Users\a")),
            Some(PathBuf::from(r"C:\"))
        );
        assert_eq!(
            surucu_koku(Path::new(r"\\?\D:\x")),
            Some(PathBuf::from(r"D:\"))
        );
        assert_eq!(surucu_koku(Path::new(r"\\sunucu\pay\x")), None);
    }

    #[test]
    fn sistem_surucusu_sabit_diskler_arasinda() {
        let sistem = std::env::var("SystemDrive").unwrap();
        let diskler = sabit_diskler();
        assert!(
            diskler.contains(&PathBuf::from(format!("{sistem}\\"))),
            "{diskler:?}"
        );
    }
}
