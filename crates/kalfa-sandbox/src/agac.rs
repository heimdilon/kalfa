//! Süreç ağacı (aşama 2 tasarımı §6.3, §6.4). Linux'ta komut kendi süreç
//! grubunda başlar, sonlandırma gruba gider. Windows'ta komut askıda başlar,
//! bir Job Object'e atanır ve sonra sürdürülür; iş kapanınca ağacın tamamı
//! ölür.

use std::io;

use tokio::process::{Child, Command};

#[cfg(windows)]
pub use windows::{Is, surdur};

pub(crate) struct Agac {
    #[cfg(unix)]
    grup: i32,
    #[cfg(windows)]
    is: windows::Is,
}

/// Komutu ağacın kökü olarak başlatır.
pub(crate) fn baslat(komut: &mut Command) -> io::Result<(Child, Agac)> {
    #[cfg(unix)]
    {
        komut.process_group(0);
        let cocuk = komut.spawn()?;
        let grup = cocuk
            .id()
            .and_then(|p| i32::try_from(p).ok())
            .ok_or_else(|| io::Error::other("sürecin kimliği okunamadı"))?;
        Ok((cocuk, Agac { grup }))
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, CREATE_SUSPENDED};
        let is = windows::Is::yeni(false)?;
        komut.creation_flags(CREATE_SUSPENDED | CREATE_NO_WINDOW);
        let cocuk = komut.spawn()?;
        let hazir = (|| {
            let tutamac = cocuk
                .raw_handle()
                .ok_or_else(|| io::Error::other("sürecin tutamacı yok"))?;
            is.ata(tutamac)?;
            let kimlik = cocuk
                .id()
                .ok_or_else(|| io::Error::other("sürecin kimliği okunamadı"))?;
            windows::surdur(kimlik)
        })();
        if let Err(e) = hazir {
            // İşe atanamayan süreç askıda kalmasın.
            is.sonlandir();
            return Err(e);
        }
        Ok((cocuk, Agac { is }))
    }
}

impl Agac {
    /// Kalfa'nın kendisinin başlattığı ve işe atadığı ağaç (Windows kabı).
    #[cfg(windows)]
    pub(crate) fn isten(is: Is) -> Self {
        Self { is }
    }

    /// Ağaca kibarca durmasını söyler (Linux'ta `SIGTERM`). Windows'ta
    /// kibar sonlandırma yoktur; ağaç hemen ölür.
    pub(crate) fn durdur(&self) {
        // SAFETY: `killpg` yalnızca bir sayı ve sinyal alır; grup yoksa ESRCH
        // döner, bellek güvenliğini etkilemez.
        #[cfg(unix)]
        unsafe {
            libc::killpg(self.grup, libc::SIGTERM);
        }
        #[cfg(windows)]
        self.is.sonlandir();
    }

    /// Ağacı öldürür.
    pub(crate) fn oldur(&self) {
        // SAFETY: `durdur`'daki gibi.
        #[cfg(unix)]
        unsafe {
            libc::killpg(self.grup, libc::SIGKILL);
        }
        #[cfg(windows)]
        self.is.sonlandir();
    }
}

/// Ağaç bırakılınca (örneğin görev iptal edilince) içindeki süreçler de
/// ölür. Windows'ta bunu işin son tutamacının kapanması yapar.
#[cfg(unix)]
impl Drop for Agac {
    fn drop(&mut self) {
        self.oldur();
    }
}

#[cfg(windows)]
mod windows {
    use std::io;
    use std::os::windows::io::RawHandle;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
    };
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_UILIMIT_DESKTOP,
        JOB_OBJECT_UILIMIT_DISPLAYSETTINGS, JOB_OBJECT_UILIMIT_EXITWINDOWS,
        JOB_OBJECT_UILIMIT_GLOBALATOMS, JOB_OBJECT_UILIMIT_HANDLES,
        JOB_OBJECT_UILIMIT_READCLIPBOARD, JOB_OBJECT_UILIMIT_SYSTEMPARAMETERS,
        JOB_OBJECT_UILIMIT_WRITECLIPBOARD, JOBOBJECT_BASIC_UI_RESTRICTIONS,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectBasicUIRestrictions,
        JobObjectExtendedLimitInformation, SetInformationJobObject, TerminateJobObject,
    };
    use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

    use crate::AZAMI_SUREC;

    /// Kapanınca içindeki bütün süreçleri öldüren Job Object.
    pub struct Is(pub(crate) HANDLE);

    // Tutamaç yalnızca çekirdek nesnesine bir başvurudur; iş parçacıkları
    // arasında taşınabilir.
    unsafe impl Send for Is {}
    unsafe impl Sync for Is {}

    // SAFETY (bu `impl` ve `surdur` için): Windows API'lerine yalnızca açık
    // tutamaçlar ve sıfırla başlatılıp boyutu doğru verilmiş yapılar geçer.
    // `Is`'in tutamacı değer yaşadıkça açıktır; `Drop`'ta bir kez kapanır.
    impl Is {
        /// İş nesnesinin Win32 tutamacını döndürür.
        pub fn tutamac(&self) -> HANDLE {
            self.0
        }

        /// Kapanışta süreç öldürmeyen, kısıtsız boş iş nesnesi (testler için).
        pub fn yeni_ayarsiz() -> io::Result<Self> {
            let is = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if is.is_null() {
                return Err(io::Error::last_os_error());
            }
            Ok(Self(is))
        }

        /// `ui_kisiti`: yalıtılmış komutlarda pano, genel atomlar, masaüstü ve
        /// başka süreçlerin kullanıcı arayüzü tutamaçları kapatılır (§9.3).
        pub fn yeni(ui_kisiti: bool) -> io::Result<Self> {
            let is = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if is.is_null() {
                return Err(io::Error::last_os_error());
            }
            let is = Self(is);
            let mut bilgi: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
            bilgi.BasicLimitInformation.LimitFlags =
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
            bilgi.BasicLimitInformation.ActiveProcessLimit = AZAMI_SUREC;
            let tamam = unsafe {
                SetInformationJobObject(
                    is.0,
                    JobObjectExtendedLimitInformation,
                    (&raw const bilgi).cast(),
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            };
            if tamam == 0 {
                return Err(io::Error::last_os_error());
            }
            if ui_kisiti {
                let kisit = JOBOBJECT_BASIC_UI_RESTRICTIONS {
                    UIRestrictionsClass: JOB_OBJECT_UILIMIT_DESKTOP
                        | JOB_OBJECT_UILIMIT_DISPLAYSETTINGS
                        | JOB_OBJECT_UILIMIT_EXITWINDOWS
                        | JOB_OBJECT_UILIMIT_GLOBALATOMS
                        | JOB_OBJECT_UILIMIT_HANDLES
                        | JOB_OBJECT_UILIMIT_READCLIPBOARD
                        | JOB_OBJECT_UILIMIT_SYSTEMPARAMETERS
                        | JOB_OBJECT_UILIMIT_WRITECLIPBOARD,
                };
                let tamam = unsafe {
                    SetInformationJobObject(
                        is.0,
                        JobObjectBasicUIRestrictions,
                        (&raw const kisit).cast(),
                        size_of::<JOBOBJECT_BASIC_UI_RESTRICTIONS>() as u32,
                    )
                };
                if tamam == 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            Ok(is)
        }

        pub fn ata(&self, surec: RawHandle) -> io::Result<()> {
            if unsafe { AssignProcessToJobObject(self.0, surec as HANDLE) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }

        pub fn sonlandir(&self) {
            unsafe {
                TerminateJobObject(self.0, 1);
            }
        }
    }

    impl Drop for Is {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    /// Askıda başlatılan sürecin iş parçacıklarını sürdürür.
    pub fn surdur(surec: u32) -> io::Result<()> {
        let anlik = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
        if anlik == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let mut giris: THREADENTRY32 = unsafe { std::mem::zeroed() };
        giris.dwSize = size_of::<THREADENTRY32>() as u32;
        let mut surduruldu = false;
        let mut var = unsafe { Thread32First(anlik, &mut giris) } != 0;
        while var {
            if giris.th32OwnerProcessID == surec {
                let is_parcacigi =
                    unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, giris.th32ThreadID) };
                if !is_parcacigi.is_null() {
                    unsafe {
                        ResumeThread(is_parcacigi);
                        CloseHandle(is_parcacigi);
                    }
                    surduruldu = true;
                }
            }
            var = unsafe { Thread32Next(anlik, &mut giris) } != 0;
        }
        unsafe {
            CloseHandle(anlik);
        }
        if surduruldu {
            Ok(())
        } else {
            Err(io::Error::other("askıdaki sürecin iş parçacığı bulunamadı"))
        }
    }
}
