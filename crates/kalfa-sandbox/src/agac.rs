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

/// Etkileşimli sürecin kökü (1.1a §4.1). Linux'ta kök, kendi torunlarının
/// alt süreç toplayıcısı olur: aradaki bir süreç ölünce yetimler init'e değil
/// köke bağlanır ve kök yaşarken alt ağaç taramasında görünür. Kendi
/// oturumunu açan (`setsid`) torun da böylece kapsanır.
pub(crate) fn baslat_etkilesimli(komut: &mut Command) -> io::Result<(Child, Agac)> {
    // SAFETY: Kapanış `fork` ile `exec` arasında çocukta çalışır ve yalnızca
    // async-signal-safe `prctl`'ı çağırır. Ayar `exec`'ten sonra korunur.
    #[cfg(target_os = "linux")]
    unsafe {
        komut.pre_exec(|| {
            if libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    baslat(komut)
}

/// Yalıtımsız PTY kök süreci: `setsid()` ile yeni oturum ve süreç grubu
/// (`pgid = pid`) açar, standart girdideki PTY slave tanımlayıcısını
/// denetleyici terminal (`TIOCSCTTY`) yapar. `process_group(0)` ayrıca
/// çağrılmaz: `setpgid(0, 0)` önce çalışırsa `setsid()` `EPERM` verir.
#[cfg(unix)]
pub(crate) fn baslat_etkilesimli_pty(komut: &mut Command) -> io::Result<(Child, Agac)> {
    // SAFETY: `fork` ile `exec` arasında çocukta yalnızca async-signal-safe
    // `prctl`, `setsid` ve `ioctl` çağrılır.
    unsafe {
        komut.pre_exec(|| {
            #[cfg(target_os = "linux")]
            if libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) == -1 {
                return Err(io::Error::last_os_error());
            }
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            if libc::ioctl(0, libc::TIOCSCTTY as _, 0) == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let cocuk = komut.spawn()?;
    let grup = cocuk
        .id()
        .and_then(|p| i32::try_from(p).ok())
        .ok_or_else(|| io::Error::other("sürecin kimliği okunamadı"))?;
    Ok((cocuk, Agac { grup }))
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

/// Etkileşimli sürecin ağacını öldürme ve bitişini doğrulama (1.1a §4.1).
/// `EtkilesimliSurec` bunu kullanır; testler sahtesini verir.
pub(crate) trait AgacDenetimi {
    /// Linux'ta kökü durdurur, kökün yaşayan torunlarını dondurup öldürür.
    /// Torun bulduysa `true` döner; çağıran, `false` dönene kadar yineler.
    /// Tarama tam yapılamazsa hata döner: eksik tarama "torun yok" sayılmaz.
    /// Yalnız kök henüz toplanmamışken (`wait`) çağrılır. Windows'ta
    /// torunları iş nesnesi kapsar; hep `false` döner.
    fn torunlari_oldur(&self) -> io::Result<bool>;
    /// Ağacı öldürür ve hatayı bildirir. Linux'ta zaten boş grup hata değildir.
    fn oldur_denetimli(&self) -> io::Result<()>;
    /// Ağaçta yaşayan süreç kalmadı mı? Linux'ta süreç grubunda (zombiler
    /// dahil) hiçbir süreç yoksa, Windows'ta işin etkin süreç sayısı sıfırsa.
    fn bos_mu(&self) -> io::Result<bool>;
}

impl AgacDenetimi for Agac {
    fn torunlari_oldur(&self) -> io::Result<bool> {
        #[cfg(target_os = "linux")]
        {
            // SAFETY: `kill` yalnızca sayı ve sinyal alır. Kök toplanmamış
            // bir çocuktur; numarası ona aittir.
            unsafe {
                libc::kill(self.grup, libc::SIGSTOP);
            }
            torunlari_oldur_ile(
                &ProcFs {
                    kok: std::path::PathBuf::from("/proc"),
                },
                self.grup,
            )
        }
        #[cfg(not(target_os = "linux"))]
        Ok(false)
    }

    fn oldur_denetimli(&self) -> io::Result<()> {
        #[cfg(unix)]
        {
            // SAFETY: `durdur`'daki gibi.
            if unsafe { libc::killpg(self.grup, libc::SIGKILL) } == -1 {
                let e = io::Error::last_os_error();
                if e.raw_os_error() != Some(libc::ESRCH) {
                    return Err(e);
                }
            }
            Ok(())
        }
        #[cfg(windows)]
        self.is.sonlandir_denetimli()
    }

    fn bos_mu(&self) -> io::Result<bool> {
        #[cfg(unix)]
        {
            // SAFETY: 0 sinyali yalnızca varlığı sınar.
            if unsafe { libc::killpg(self.grup, 0) } == 0 {
                return Ok(false);
            }
            let e = io::Error::last_os_error();
            match e.raw_os_error() {
                Some(libc::ESRCH) => Ok(true),
                // Başka kullanıcının süreci gruptaysa (EPERM) grup boş değil.
                Some(libc::EPERM) => Ok(false),
                _ => Err(e),
            }
        }
        #[cfg(windows)]
        Ok(self.is.etkin_surec_sayisi()? == 0)
    }
}

/// `/proc/<pid>/stat`'tan okunan süreç kimliği.
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SurecBilgisi {
    pub pid: i32,
    pub ppid: i32,
    /// Açılıştan beri saat tıkı (`starttime`). Numara başka bir sürece
    /// verilirse değişir.
    pub baslangic: u64,
    pub zombi: bool,
}

/// Torun öldürmenin işletim sistemi kapısı; testler sahtesini verir.
#[cfg(target_os = "linux")]
pub(crate) trait SurecKaynagi {
    type Tutamac;
    /// Bütün süreçler. Tam okunamazsa hata döner.
    fn tara(&self) -> io::Result<Vec<SurecBilgisi>>;
    /// Süreci sabit bir başvuruyla açar; süreç yoksa `None`.
    fn ac(&self, pid: i32) -> io::Result<Option<Self::Tutamac>>;
    /// Açtıktan sonra yeniden okunan başlangıç; süreç yoksa `None`.
    fn baslangic(&self, pid: i32) -> io::Result<Option<u64>>;
    /// Açılan sürece sinyal; süreç bitmişse hata değildir.
    fn sinyal(&self, tutamac: &Self::Tutamac, sinyal: i32) -> io::Result<()>;
}

/// `kok`'ün zombi olmayan torunlarını bulur. Her adayın yaprağını pidfd
/// ile açıp başlangıcını, sonra köke kadar snapshot'taki her ebeveynin
/// başlangıcını doğrular. Değişmiş ya da kaybolmuş bir ara kimliğin alt dalı
/// bu taramayla sinyallenmez; sonraki tarama güncel ebeveynliği görür.
/// Kök toplanmadığı ve Linux'ta alt süreç toplayıcısı olduğu için doğrulama
/// sonrasında ara süreç çıkarsa yetimleri yine bu kökün altına bağlanır.
#[cfg(target_os = "linux")]
pub(crate) fn torunlari_oldur_ile<K: SurecKaynagi>(k: &K, kok: i32) -> io::Result<bool> {
    use std::collections::HashMap;

    let surecler = k.tara()?;
    let bilgiye_gore_pid: HashMap<i32, SurecBilgisi> =
        surecler.iter().map(|s| (s.pid, *s)).collect();
    let mut agac = vec![kok];
    let mut torunlar = Vec::new();
    let mut i = 0;
    while i < agac.len() {
        let ust = agac[i];
        for s in surecler.iter().filter(|s| s.ppid == ust) {
            if agac.contains(&s.pid) {
                continue;
            }
            agac.push(s.pid);
            if !s.zombi {
                torunlar.push(*s);
            }
        }
        i += 1;
    }
    let mut acik = Vec::new();
    for s in &torunlar {
        let Some(t) = k.ac(s.pid)? else {
            continue;
        };
        // Yaprak denetimi pidfd açıldıktan sonra sürer. Zincirin herhangi bir
        // eski halkası eşleşmezse aynı snapshot'taki alt dal hedef olamaz.
        let mut halka = s;
        loop {
            if k.baslangic(halka.pid)? != Some(halka.baslangic) {
                break;
            }
            if halka.ppid == kok {
                acik.push(t);
                break;
            }
            let Some(ebeveyn) = bilgiye_gore_pid.get(&halka.ppid) else {
                break;
            };
            halka = ebeveyn;
        }
    }
    for sinyal in [libc::SIGSTOP, libc::SIGKILL] {
        for t in &acik {
            k.sinyal(t, sinyal)?;
        }
    }
    Ok(!torunlar.is_empty())
}
/// Gerçek `/proc` ve `pidfd` (Linux 5.3+).
#[cfg(target_os = "linux")]
pub(crate) struct ProcFs {
    pub kok: std::path::PathBuf,
}

#[cfg(target_os = "linux")]
impl SurecKaynagi for ProcFs {
    type Tutamac = std::os::fd::OwnedFd;

    fn tara(&self) -> io::Result<Vec<SurecBilgisi>> {
        let mut surecler = Vec::new();
        for girdi in std::fs::read_dir(&self.kok)? {
            let girdi = girdi?;
            let Some(pid) = girdi.file_name().to_str().and_then(|a| a.parse().ok()) else {
                continue;
            };
            if let Some(s) = stat_oku(&self.kok, pid)? {
                surecler.push(s);
            }
        }
        Ok(surecler)
    }

    fn ac(&self, pid: i32) -> io::Result<Option<Self::Tutamac>> {
        use std::os::fd::FromRawFd;
        // SAFETY: `pidfd_open` yalnız sayı alır; dönen tanımlayıcı hemen
        // sahiplenilir.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
        if fd < 0 {
            let e = io::Error::last_os_error();
            return match e.raw_os_error() {
                Some(libc::ESRCH) => Ok(None),
                _ => Err(e),
            };
        }
        // SAFETY: `fd` az önce açıldı ve başka bir sahibi yok.
        Ok(Some(unsafe {
            std::os::fd::OwnedFd::from_raw_fd(fd as i32)
        }))
    }

    fn baslangic(&self, pid: i32) -> io::Result<Option<u64>> {
        Ok(stat_oku(&self.kok, pid)?.map(|s| s.baslangic))
    }

    fn sinyal(&self, tutamac: &Self::Tutamac, sinyal: i32) -> io::Result<()> {
        use std::os::fd::AsRawFd;
        // SAFETY: tanımlayıcı açık bir pidfd'dir; bilgi göstericisi boştur.
        let r = unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                tutamac.as_raw_fd(),
                sinyal,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            )
        };
        if r < 0 {
            let e = io::Error::last_os_error();
            if e.raw_os_error() != Some(libc::ESRCH) {
                return Err(e);
            }
        }
        Ok(())
    }
}

/// `<kok>/<pid>/stat`. Kayıt yoksa süreç toplanmıştır: `None`. Başka okuma
/// hatası ya da ayrıştırılamayan kayıt hatadır.
#[cfg(target_os = "linux")]
fn stat_oku(kok: &std::path::Path, pid: i32) -> io::Result<Option<SurecBilgisi>> {
    let metin = match std::fs::read_to_string(kok.join(pid.to_string()).join("stat")) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound || e.raw_os_error() == Some(libc::ESRCH) => {
            return Ok(None);
        }
        Err(e) => return Err(e),
    };
    stat_ayristir(pid, &metin).map(Some).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{pid} sürecinin stat kaydı ayrıştırılamadı"),
        )
    })
}

/// `pid (ad) durum ppid … starttime …`: ad parantez içerebilir; son `)`'den
/// sonrası okunur. `starttime` 22. alandır.
#[cfg(target_os = "linux")]
fn stat_ayristir(pid: i32, metin: &str) -> Option<SurecBilgisi> {
    let (_, geri) = metin.rsplit_once(')')?;
    let alanlar: Vec<&str> = geri.split_whitespace().collect();
    let durum = *alanlar.first()?;
    Some(SurecBilgisi {
        pid,
        ppid: alanlar.get(1)?.parse().ok()?,
        baslangic: alanlar.get(19)?.parse().ok()?,
        zombi: durum == "Z" || durum == "X",
    })
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
        JOB_OBJECT_UILIMIT_WRITECLIPBOARD, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
        JOBOBJECT_BASIC_UI_RESTRICTIONS, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JobObjectBasicAccountingInformation, JobObjectBasicUIRestrictions,
        JobObjectExtendedLimitInformation, QueryInformationJobObject, SetInformationJobObject,
        TerminateJobObject,
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

        /// `sonlandir` gibi; başarısızlığı bildirir. Sonlandırma eşzamansızdır:
        /// bitiş `etkin_surec_sayisi` ile ayrıca doğrulanır.
        pub fn sonlandir_denetimli(&self) -> io::Result<()> {
            // SAFETY: `self.0` bu nesnenin sahip olduğu açık iş tutamacıdır.
            if unsafe { TerminateJobObject(self.0, 1) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }

        /// İşte henüz bitmemiş süreçlerin sayısı.
        pub fn etkin_surec_sayisi(&self) -> io::Result<u32> {
            // SAFETY: Yapı yalnız sayısal alanlardan oluşur; sıfır geçerli bir değerdir.
            let mut bilgi: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = unsafe { std::mem::zeroed() };
            // SAFETY: Açık iş tutamacı ve yapının boyutuna uygun yazılabilir gösterici verilir.
            let tamam = unsafe {
                QueryInformationJobObject(
                    self.0,
                    JobObjectBasicAccountingInformation,
                    (&raw mut bilgi).cast(),
                    size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                    std::ptr::null_mut(),
                )
            };
            if tamam == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(bilgi.ActiveProcesses)
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

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::Path;

    use super::*;

    fn bilgi(pid: i32, ppid: i32, baslangic: u64, zombi: bool) -> SurecBilgisi {
        SurecBilgisi {
            pid,
            ppid,
            baslangic,
            zombi,
        }
    }

    /// Sahte süreç tablosu: taramadan sonra `simdiki` başlangıçlar okunur;
    /// gönderilen sinyaller kaydedilir.
    #[derive(Default)]
    struct Sahte {
        tablo: Vec<SurecBilgisi>,
        simdiki: HashMap<i32, u64>,
        tarama_hatasi: bool,
        acma_hatasi: bool,
        baslangic_hatasi: bool,
        sinyaller: RefCell<Vec<(i32, i32)>>,
    }

    impl SurecKaynagi for Sahte {
        type Tutamac = i32;

        fn tara(&self) -> io::Result<Vec<SurecBilgisi>> {
            if self.tarama_hatasi {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            Ok(self.tablo.clone())
        }

        fn ac(&self, pid: i32) -> io::Result<Option<i32>> {
            if self.acma_hatasi {
                return Err(io::Error::from(io::ErrorKind::Unsupported));
            }
            Ok(self.simdiki.contains_key(&pid).then_some(pid))
        }

        fn baslangic(&self, pid: i32) -> io::Result<Option<u64>> {
            if self.baslangic_hatasi {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            Ok(self.simdiki.get(&pid).copied())
        }
        fn sinyal(&self, t: &i32, sinyal: i32) -> io::Result<()> {
            self.sinyaller.borrow_mut().push((*t, sinyal));
            Ok(())
        }
    }

    /// Kök 10; 11 ve 12 torun (12 kendi oturumunda olabilir), 14 zombi
    /// torun, 13 ilgisiz.
    fn agac() -> Sahte {
        let tablo = vec![
            bilgi(11, 10, 100, false),
            bilgi(12, 11, 101, false),
            bilgi(13, 1, 102, false),
            bilgi(14, 11, 103, true),
        ];
        let simdiki = tablo.iter().map(|s| (s.pid, s.baslangic)).collect();
        Sahte {
            tablo,
            simdiki,
            ..Default::default()
        }
    }

    #[test]
    fn etkilesimli_torunlar_dondurulup_oldurulur_ilgisize_dokunulmaz() {
        let k = agac();
        assert!(torunlari_oldur_ile(&k, 10).unwrap());
        // Önce bütün torunlar durdurulur, sonra öldürülür.
        let beklenen = vec![
            (11, libc::SIGSTOP),
            (12, libc::SIGSTOP),
            (11, libc::SIGKILL),
            (12, libc::SIGKILL),
        ];
        assert_eq!(*k.sinyaller.borrow(), beklenen);
    }

    #[test]
    fn etkilesimli_numarasi_yeniden_kullanilan_toruna_sinyal_gitmez() {
        let mut k = agac();
        // 12 tarandıktan sonra bitti; numarası yeni bir sürece verildi.
        k.simdiki.insert(12, 999);
        assert!(torunlari_oldur_ile(&k, 10).unwrap());
        assert!(k.sinyaller.borrow().iter().all(|(p, _)| *p != 12));
        // 11 tarandıktan sonra toplandı: açılamaz, sinyal gitmez.
        let mut k = agac();
        k.simdiki.remove(&11);
        torunlari_oldur_ile(&k, 10).unwrap();
        assert!(k.sinyaller.borrow().iter().all(|(p, _)| *p != 11));
    }

    #[test]
    fn etkilesimli_yeniden_kullanilan_ara_ebeveynin_alt_dali_sinyal_almaz() {
        let mut k = agac();
        // 12'nin eski kimliği 11'in çocuğudur; yeni 12 ilgisiz 15'in ebeveynidir.
        k.tablo.push(bilgi(15, 12, 300, false));
        k.simdiki.insert(15, 300);
        k.simdiki.insert(12, 999);

        assert!(torunlari_oldur_ile(&k, 10).unwrap());
        let sinyaller = k.sinyaller.borrow();
        assert!(
            sinyaller.iter().all(|(pid, _)| *pid != 12 && *pid != 15),
            "yeniden kullanılan ebeveynin alt dalına sinyal gitti: {sinyaller:?}"
        );
        assert!(
            sinyaller.iter().any(|(pid, _)| *pid == 11),
            "geçerli kök çocuğu hâlâ kapsanmalı: {sinyaller:?}"
        );
    }

    #[test]
    fn etkilesimli_kaybolan_ara_ebeveyn_eski_dali_atlar_yeni_tarama_yetimi_kapsar() {
        let mut k = agac();
        // İlk taramadan sonra 11 kaybolur; 12 yeniden ebeveynlenince yeni
        // /proc taraması onu doğrudan kökün çocuğu olarak görür.
        k.simdiki.remove(&11);

        assert!(torunlari_oldur_ile(&k, 10).unwrap());
        assert!(
            k.sinyaller.borrow().is_empty(),
            "eski zincirden sinyal gitmemeli: {:?}",
            *k.sinyaller.borrow()
        );

        k.tablo = vec![bilgi(12, 10, 101, false)];
        assert!(torunlari_oldur_ile(&k, 10).unwrap());
        assert_eq!(
            *k.sinyaller.borrow(),
            vec![(12, libc::SIGSTOP), (12, libc::SIGKILL),]
        );
    }
    #[test]
    fn etkilesimli_tarama_acma_ve_baslangic_hatasi_basari_sayilmaz() {
        let mut k = agac();
        k.tarama_hatasi = true;
        assert!(torunlari_oldur_ile(&k, 10).is_err());
        let mut k = agac();
        k.acma_hatasi = true;
        assert!(torunlari_oldur_ile(&k, 10).is_err());
        assert!(k.sinyaller.borrow().is_empty());
        let mut k = agac();
        k.baslangic_hatasi = true;
        assert!(torunlari_oldur_ile(&k, 10).is_err());
        assert!(k.sinyaller.borrow().is_empty());
    }
    #[test]
    fn etkilesimli_torunu_olmayan_kok_false_doner() {
        let k = agac();
        assert!(!torunlari_oldur_ile(&k, 13).unwrap());
        assert!(k.sinyaller.borrow().is_empty());
    }

    fn stat_yaz(kok: &Path, pid: i32, metin: &str) {
        std::fs::create_dir_all(kok.join(pid.to_string())).unwrap();
        std::fs::write(kok.join(pid.to_string()).join("stat"), metin).unwrap();
    }

    fn stat_metni(pid: i32, ad: &str, durum: &str, ppid: i32, baslangic: u64) -> String {
        // 22. alan `starttime`; arada 17 alan.
        format!(
            "{pid} ({ad}) {durum} {ppid} {} {baslangic} 0 0\n",
            vec!["0"; 17].join(" ")
        )
    }

    #[test]
    fn etkilesimli_stat_parantezli_adla_ayristirilir() {
        let s = stat_ayristir(7, &stat_metni(7, "a) (b", "S", 3, 4242)).unwrap();
        assert_eq!(s, bilgi(7, 3, 4242, false));
        assert!(
            stat_ayristir(7, &stat_metni(7, "z", "Z", 3, 1))
                .unwrap()
                .zombi
        );
        assert!(stat_ayristir(7, "7 (kesik").is_none());
        assert!(stat_ayristir(7, "7 (a) S x 0").is_none());
    }

    #[test]
    fn etkilesimli_proc_okuma_hatalari_tarama_hatasidir() {
        let d = tempfile::tempdir().unwrap();
        let k = ProcFs {
            kok: d.path().to_path_buf(),
        };
        stat_yaz(d.path(), 11, &stat_metni(11, "a", "S", 10, 5));
        // Kaydı olmayan süreç toplanmıştır; atlanır.
        std::fs::create_dir_all(d.path().join("12")).unwrap();
        std::fs::create_dir_all(d.path().join("self")).unwrap();
        assert_eq!(k.tara().unwrap(), vec![bilgi(11, 10, 5, false)]);
        // Okunamayan kayıt (burada dizin) ve bozuk kayıt hatadır.
        std::fs::create_dir_all(d.path().join("13/stat")).unwrap();
        assert!(k.tara().is_err());
        std::fs::remove_dir_all(d.path().join("13")).unwrap();
        stat_yaz(d.path(), 14, "14 (bozuk");
        assert!(k.tara().is_err());
        // `/proc` açılamıyor.
        let yok = ProcFs {
            kok: d.path().join("yok"),
        };
        assert!(yok.tara().is_err());
    }
}
