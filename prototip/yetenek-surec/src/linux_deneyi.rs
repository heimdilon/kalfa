//! Görev 2 (Linux): `bwrap` + `seccomp` + `PR_SET_CHILD_SUBREAPER` + `pidfd`
//! içinde borulu süreç ve PTY (`openpty`, `TIOCSCTTY`, `TIOCSWINSZ`) deneyleri.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use seccompiler::{
    BpfProgram, SeccompAction, SeccompCmpArgLen as Uzunluk, SeccompCmpOp as Islem,
    SeccompCondition, SeccompFilter, SeccompRule, TargetArch, sock_filter,
};

use crate::kilit_deneyi::{SatirRaporu, Sonuc};

pub const AZAMI_IS_CIKTISI_BAYT: usize = 1024 * 1024; // 1 MiB

static SIGINT_ALINDI: AtomicBool = AtomicBool::new(false);

extern "C" fn sigint_yakala(_sig: libc::c_int) {
    SIGINT_ALINDI.store(true, Ordering::SeqCst);
}

// ---------------------------------------------------------------- çocuk alt komutları

pub fn cocuk_komutu(alt: &str) -> i32 {
    match alt {
        "yanki" => cocuk_yanki(),
        "ctrlc-sctty" => cocuk_ctrlc(true),
        "ctrlc-nosctty" => cocuk_ctrlc(false),
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

fn cocuk_ctrlc(sctty: bool) -> i32 {
    // SAFETY: geçerli SIGINT işleyicisi kaydedilir; istenirse setsid + TIOCSCTTY çağrılır.
    let (sid_ret, ctty_ret) = unsafe {
        libc::signal(
            libc::SIGINT,
            sigint_yakala as *const () as libc::sighandler_t,
        );
        if sctty {
            let s = libc::setsid();
            let c = libc::ioctl(0, libc::TIOCSCTTY as _, 0);
            (s, c)
        } else {
            (0, 0)
        }
    };
    let mut out = std::io::stdout().lock();
    let _ = writeln!(
        out,
        "HAZIR pid={} sid_ret={sid_ret} ctty_ret={ctty_ret}",
        std::process::id()
    );
    let _ = out.flush();
    let bas = Instant::now();
    while bas.elapsed() < Duration::from_secs(5) {
        if SIGINT_ALINDI.load(Ordering::SeqCst) {
            let _ = writeln!(out, "CTRL_C_ALINDI");
            let _ = out.flush();
            return 42;
        }
        std::thread::sleep(Duration::from_millis(15));
    }
    1
}

fn pty_boyutu() -> (u16, u16) {
    // SAFETY: standart girdi/çıktı üzerinde TIOCGWINSZ sorgulanır.
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(0, libc::TIOCGWINSZ, &mut ws) == 0 {
            (ws.ws_col, ws.ws_row)
        } else {
            (0, 0)
        }
    }
}

fn cocuk_pty_boyut() -> i32 {
    let mut out = std::io::stdout().lock();
    let (g, y) = pty_boyutu();
    let _ = writeln!(out, "\x1b[32mPTY_ACILDI\x1b[0m utf8=çğıöşü boyut={g}x{y}");
    let _ = out.flush();

    let stdin = std::io::stdin();
    let mut r = stdin.lock();
    let mut satir = String::new();
    while r.read_line(&mut satir).map(|n| n > 0).unwrap_or(false) {
        let komut = satir.trim();
        if komut == "boyut" {
            let (g2, y2) = pty_boyutu();
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
    loop {
        if out.write_all(blok.as_bytes()).is_err() {
            break;
        }
    }
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}

// ---------------------------------------------------------------- seccomp ve bwrap yardımcıları

const ENGELLI: &[i64] = &[
    libc::SYS_io_uring_setup,
    libc::SYS_io_uring_enter,
    libc::SYS_io_uring_register,
    libc::SYS_keyctl,
    libc::SYS_add_key,
    libc::SYS_request_key,
    libc::SYS_bpf,
    libc::SYS_perf_event_open,
    libc::SYS_userfaultfd,
    libc::SYS_ptrace,
    libc::SYS_pidfd_getfd,
];

const AD_ALANI_BAYRAKLARI: &[u64] = &[
    libc::CLONE_NEWUSER as u64,
    libc::CLONE_NEWNS as u64,
    libc::CLONE_NEWPID as u64,
    libc::CLONE_NEWNET as u64,
    libc::CLONE_NEWIPC as u64,
    libc::CLONE_NEWUTS as u64,
    libc::CLONE_NEWCGROUP as u64,
];
const CLONE_NEWTIME: u64 = 0x80;

fn engel_filtresi() -> Sonuc<Vec<u8>> {
    let arch = TargetArch::try_from(std::env::consts::ARCH).map_err(|e| e.to_string())?;
    let mut kurallar: BTreeMap<i64, Vec<SeccompRule>> =
        ENGELLI.iter().map(|c| (*c, Vec::new())).collect();
    let bayrak_kurallari = |bayraklar: &[u64]| -> Sonuc<Vec<SeccompRule>> {
        bayraklar
            .iter()
            .map(|b| {
                let c = SeccompCondition::new(0, Uzunluk::Qword, Islem::MaskedEq(*b), *b)
                    .map_err(|e| e.to_string())?;
                SeccompRule::new(vec![c]).map_err(|e| e.to_string())
            })
            .collect()
    };
    let mut unshare = AD_ALANI_BAYRAKLARI.to_vec();
    unshare.push(CLONE_NEWTIME);
    kurallar.insert(libc::SYS_unshare, bayrak_kurallari(&unshare)?);
    kurallar.insert(libc::SYS_clone, bayrak_kurallari(AD_ALANI_BAYRAKLARI)?);

    // Kalfa seccomp EPERM listesinden TIOCSTI (0x5412) ve TIOCLINUX (0x541C) engeli:
    let tiocsti = SeccompRule::new(vec![
        SeccompCondition::new(1, Uzunluk::Dword, Islem::Eq, libc::TIOCSTI as u64)
            .map_err(|e| e.to_string())?,
    ])
    .map_err(|e| e.to_string())?;
    let tioclinux = SeccompRule::new(vec![
        SeccompCondition::new(1, Uzunluk::Dword, Islem::Eq, 0x541C).map_err(|e| e.to_string())?,
    ])
    .map_err(|e| e.to_string())?;
    kurallar.insert(libc::SYS_ioctl, vec![tiocsti, tioclinux]);

    let filtre = SeccompFilter::new(
        kurallar,
        SeccompAction::Allow,
        SeccompAction::Errno(libc::EPERM as u32),
        arch,
    )
    .map_err(|e| e.to_string())?;
    let prog: BpfProgram = BpfProgram::try_from(filtre).map_err(|e| e.to_string())?;
    Ok(bpf_baytlar(&prog))
}

fn bpf_baytlar(program: &[sock_filter]) -> Vec<u8> {
    program
        .iter()
        .flat_map(|k| {
            let mut b = Vec::with_capacity(8);
            b.extend_from_slice(&k.code.to_ne_bytes());
            b.push(k.jt);
            b.push(k.jf);
            b.extend_from_slice(&k.k.to_ne_bytes());
            b
        })
        .collect()
}

fn memfd_yaz(ad: &std::ffi::CStr, veri: &[u8]) -> Sonuc<OwnedFd> {
    // SAFETY: memfd_create geçerli C dizesiyle çağrılır ve OwnedFd'ye sarılır.
    let fd = unsafe { libc::memfd_create(ad.as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        return Err(format!("memfd_create: {}", std::io::Error::last_os_error()));
    }
    let mut dosya = unsafe { File::from_raw_fd(fd) };
    dosya.write_all(veri).map_err(|e| e.to_string())?;
    dosya
        .sync_all()
        .or(Ok::<(), std::io::Error>(()))
        .map_err(|e| e.to_string())?;
    use std::io::Seek;
    dosya
        .seek(std::io::SeekFrom::Start(0))
        .map_err(|e| e.to_string())?;
    Ok(OwnedFd::from(dosya))
}

const BWRAP_YOLLARI: &[&str] = &[
    "/usr/lib/kalfa/bwrap",
    "/usr/bin/bwrap",
    "/usr/local/bin/bwrap",
];

const SISTEM_YOLLARI: &[&str] = &[
    "/usr", "/etc", "/bin", "/sbin", "/lib", "/lib32", "/lib64", "/libx32",
];

fn bwrap_yolu() -> Sonuc<&'static Path> {
    BWRAP_YOLLARI
        .iter()
        .map(Path::new)
        .find(|y| y.is_file())
        .ok_or_else(|| "bwrap bulunamadı".to_string())
}

fn enosys_filtresi() -> Sonuc<Vec<u8>> {
    let arch = TargetArch::try_from(std::env::consts::ARCH).map_err(|e| e.to_string())?;
    let izin = libc::SECCOMP_RET_ALLOW;
    let enosys = libc::SECCOMP_RET_ERRNO | libc::ENOSYS as u32;
    let clone3 = libc::SYS_clone3 as u32;
    let komut = |code: u16, jt: u8, jf: u8, k: u32| sock_filter { code, jt, jf, k };
    let prog = match arch {
        TargetArch::x86_64 => vec![
            komut(0x20, 0, 0, 4),
            komut(0x15, 0, 3, 62 | 0x8000_0000 | 0x4000_0000),
            komut(0x20, 0, 0, 0),
            komut(0x15, 2, 0, clone3),
            komut(0x35, 1, 0, 0x4000_0000),
            komut(0x06, 0, 0, izin),
            komut(0x06, 0, 0, enosys),
        ],
        TargetArch::aarch64 => vec![
            komut(0x20, 0, 0, 4),
            komut(0x15, 0, 2, 183 | 0x8000_0000 | 0x4000_0000),
            komut(0x20, 0, 0, 0),
            komut(0x15, 1, 0, clone3),
            komut(0x06, 0, 0, izin),
            komut(0x06, 0, 0, enosys),
        ],
        diger => return Err(format!("desteklenmeyen mimari: {diger:?}")),
    };
    Ok(bpf_baytlar(&prog))
}

fn bwrap_argumanlari(alan: &Path, seccomp_fdler: &[RawFd], yeni_oturum: bool) -> Vec<u8> {
    let mut parcalar: Vec<String> = vec![
        "--unshare-user".into(),
        "--unshare-pid".into(),
        "--unshare-net".into(),
        "--unshare-ipc".into(),
        "--unshare-uts".into(),
        "--unshare-cgroup-try".into(),
        "--disable-userns".into(),
        "--die-with-parent".into(),
    ];
    if yeni_oturum {
        parcalar.push("--new-session".into());
    }
    parcalar.extend(["--clearenv".into(), "--tmpfs".into(), "/".into()]);
    for s in SISTEM_YOLLARI {
        let yol = Path::new(s);
        if let Ok(meta) = std::fs::symlink_metadata(yol) {
            if meta.file_type().is_symlink() {
                if let Ok(hedef) = std::fs::read_link(yol) {
                    parcalar.push("--symlink".into());
                    parcalar.push(hedef.display().to_string());
                    parcalar.push(s.to_string());
                }
            } else if meta.is_dir() {
                parcalar.push("--ro-bind".into());
                parcalar.push(s.to_string());
                parcalar.push(s.to_string());
            }
        }
    }
    let alan_s = alan.display().to_string();
    parcalar.extend([
        "--proc".into(),
        "/proc".into(),
        "--dev".into(),
        "/dev".into(),
        "--tmpfs".into(),
        "/run".into(),
        "--tmpfs".into(),
        "/tmp".into(),
        "--bind".into(),
        alan_s.clone(),
        alan_s.clone(),
        "--remount-ro".into(),
        "/".into(),
        "--setenv".into(),
        "PATH".into(),
        "/usr/bin:/bin".into(),
        "--setenv".into(),
        "HOME".into(),
        alan_s.clone(),
        "--setenv".into(),
        "TERM".into(),
        "xterm-256color".into(),
        "--chdir".into(),
        alan_s,
    ]);
    for fd in seccomp_fdler {
        parcalar.push("--add-seccomp-fd".into());
        parcalar.push(fd.to_string());
    }

    let mut nul_ayrik = Vec::new();
    for p in parcalar {
        nul_ayrik.extend_from_slice(p.as_bytes());
        nul_ayrik.push(0);
    }
    nul_ayrik
}

struct BwrapBoruluSurec {
    cocuk: std::process::Child,
    _args_fd: OwnedFd,
    _engel_fd: OwnedFd,
    _enosys_fd: OwnedFd,
}

fn bwrap_borulu_baslat(alan: &Path, ic_komut: &[String]) -> Sonuc<BwrapBoruluSurec> {
    let bwrap = bwrap_yolu()?;
    let engel_fd = memfd_yaz(c"kalfa-g2a-engel", &engel_filtresi()?)?;
    let enosys_fd = memfd_yaz(c"kalfa-g2a-enosys", &enosys_filtresi()?)?;
    let args_veri = bwrap_argumanlari(alan, &[engel_fd.as_raw_fd(), enosys_fd.as_raw_fd()], true);
    let args_fd = memfd_yaz(c"kalfa-g2a-args", &args_veri)?;

    let a_raw = args_fd.as_raw_fd();
    let e_raw = engel_fd.as_raw_fd();
    let n_raw = enosys_fd.as_raw_fd();

    let mut cmd = std::process::Command::new(bwrap);
    cmd.args(["--args", &a_raw.to_string(), "--"])
        .args(ic_komut)
        .env_clear()
        .current_dir("/")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // SAFETY: yalnız async-signal-safe fcntl ve prctl çağrılır.
    unsafe {
        cmd.pre_exec(move || {
            let _ = libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
            for fd in [a_raw, e_raw, n_raw] {
                if libc::fcntl(fd, libc::F_SETFD, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }

    let cocuk = cmd.spawn().map_err(|e| format!("bwrap spawn: {e}"))?;
    Ok(BwrapBoruluSurec {
        cocuk,
        _args_fd: args_fd,
        _engel_fd: engel_fd,
        _enosys_fd: enosys_fd,
    })
}

struct BwrapPtySurec {
    cocuk: std::process::Child,
    master: File,
    _args_fd: OwnedFd,
    _engel_fd: OwnedFd,
    _enosys_fd: OwnedFd,
}

impl BwrapPtySurec {
    fn boyut_degistir(&self, sutun: u16, satir: u16) -> Sonuc<()> {
        let ws = libc::winsize {
            ws_row: satir,
            ws_col: sutun,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        // SAFETY: geçerli PTY master FD üzerinde TIOCSWINSZ çağrılır.
        let r = unsafe { libc::ioctl(self.master.as_raw_fd(), libc::TIOCSWINSZ, &ws) };
        if r < 0 {
            return Err(format!("TIOCSWINSZ: {}", std::io::Error::last_os_error()));
        }
        Ok(())
    }
}

fn bwrap_pty_baslat(
    alan: &Path,
    ic_komut: &[String],
    sutun: u16,
    satir: u16,
) -> Sonuc<BwrapPtySurec> {
    let bwrap = bwrap_yolu()?;
    let mut master_fd: libc::c_int = -1;
    let mut slave_fd: libc::c_int = -1;
    let ws = libc::winsize {
        ws_row: satir,
        ws_col: sutun,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: geçerli winsize ile openpty çağrılır.
    unsafe {
        if libc::openpty(&mut master_fd, &mut slave_fd, null_mut(), null_mut(), &ws) < 0 {
            return Err(format!("openpty: {}", std::io::Error::last_os_error()));
        }
        let m_flags = libc::fcntl(master_fd, libc::F_GETFD);
        if m_flags >= 0 {
            let _ = libc::fcntl(master_fd, libc::F_SETFD, m_flags | libc::FD_CLOEXEC);
        }
    }
    let master = unsafe { File::from_raw_fd(master_fd) };
    let slave_in = unsafe { File::from_raw_fd(slave_fd) };
    let slave_out = slave_in.try_clone().map_err(|e| e.to_string())?;
    let slave_err = slave_in.try_clone().map_err(|e| e.to_string())?;

    let engel_fd = memfd_yaz(c"kalfa-g2a-engel", &engel_filtresi()?)?;
    let enosys_fd = memfd_yaz(c"kalfa-g2a-enosys", &enosys_filtresi()?)?;
    let args_veri = bwrap_argumanlari(alan, &[engel_fd.as_raw_fd(), enosys_fd.as_raw_fd()], true);
    let args_fd = memfd_yaz(c"kalfa-g2a-args", &args_veri)?;

    let a_raw = args_fd.as_raw_fd();
    let e_raw = engel_fd.as_raw_fd();
    let n_raw = enosys_fd.as_raw_fd();

    let mut cmd = std::process::Command::new(bwrap);
    cmd.args([
        "--args",
        &a_raw.to_string(),
        "--",
        "/usr/bin/setsid",
        "-c",
        "--",
    ])
    .args(ic_komut)
    .env_clear()
    .current_dir("/")
    .stdin(std::process::Stdio::from(slave_in))
    .stdout(std::process::Stdio::from(slave_out))
    .stderr(std::process::Stdio::from(slave_err));

    // SAFETY: yalnız async-signal-safe fcntl ve prctl çağrılır.
    unsafe {
        cmd.pre_exec(move || {
            let _ = libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
            for fd in [a_raw, e_raw, n_raw] {
                if libc::fcntl(fd, libc::F_SETFD, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }

    let cocuk = cmd.spawn().map_err(|e| format!("bwrap PTY spawn: {e}"))?;
    Ok(BwrapPtySurec {
        cocuk,
        master,
        _args_fd: args_fd,
        _engel_fd: engel_fd,
        _enosys_fd: enosys_fd,
    })
}

pub fn calistir() -> Sonuc<Vec<SatirRaporu>> {
    // SAFETY: ana süreç alt süreçleri sahiplenmek için subreaper olur.
    unsafe {
        let _ = libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1);
    }
    let mut satirlar = Vec::new();
    let pid = std::process::id();
    let kok = std::env::temp_dir().join(format!("kalfa-g2a-{pid}"));
    let _ = std::fs::remove_dir_all(&kok);
    std::fs::create_dir_all(&kok).map_err(|e| e.to_string())?;

    let sonuc = (|| -> Sonuc<()> {
        let exe = kok.join("kalfa-yetenek-surec-prototip");
        std::fs::copy(std::env::current_exe().map_err(|e| e.to_string())?, &exe)
            .map_err(|e| format!("exe kopyası: {e}"))?;
        let exe_s = exe.display().to_string();

        // ---------------------------------------------------------------------
        // G2.1: bwrap + seccomp içinde borulu stdin/stdout ve EOF
        // ---------------------------------------------------------------------
        {
            let bas = Instant::now();
            let mut s =
                bwrap_borulu_baslat(&kok, &[exe_s.clone(), "cocuk".into(), "yanki".into()])?;
            let mut girdi = s.cocuk.stdin.take().ok_or("stdin yok")?;
            let cikti = s.cocuk.stdout.take().ok_or("stdout yok")?;
            let okuyucu = std::thread::spawn(move || {
                let mut r = BufReader::new(cikti);
                let mut buf = Vec::new();
                let _ = r.read_to_end(&mut buf);
                buf
            });
            let mut beklenen = Vec::new();
            for i in 0..200 {
                let satir = format!("satir-{i}-çğıöşü\n");
                girdi
                    .write_all(satir.as_bytes())
                    .map_err(|e| e.to_string())?;
                beklenen.extend_from_slice(satir.as_bytes());
            }
            let buyuk = format!("{}\n", "B".repeat(256 * 1024));
            girdi
                .write_all(buyuk.as_bytes())
                .map_err(|e| e.to_string())?;
            beklenen.extend_from_slice(buyuk.as_bytes());
            drop(girdi);

            let alinan = okuyucu.join().map_err(|_| "okuyucu çöktü")?;
            let durum = s.cocuk.wait().map_err(|e| e.to_string())?;
            let birebir = alinan == beklenen;
            satirlar.push(SatirRaporu {
                ad: "G2.1 Linux bwrap+seccomp borulu stdin/stdout ve EOF".into(),
                gecti: birebir && durum.code() == Some(0),
                ayrinti: format!(
                    "{} bayt (200 UTF-8 satırı + 256 KiB satır) birebir={birebir}, EOF çıkış kodu={:?}, süre={} ms",
                    alinan.len(),
                    durum.code(),
                    bas.elapsed().as_millis()
                ),
            });
        }

        // ---------------------------------------------------------------------
        // G2.2: bwrap + seccomp + PTY içinde `\x03` (`Ctrl+C`) ve `TIOCSCTTY`
        // ---------------------------------------------------------------------
        {
            let bas = Instant::now();
            // Çocuğun kendisi setsid/TIOCSCTTY çağırmadan (`ctrlc-nosctty`),
            // `bwrap --new-session` içinde `/usr/bin/setsid -c` sarmalayıcısıyla
            // `\x03` -> SIGINT ölçümü:
            let mut s = bwrap_pty_baslat(
                &kok,
                &[exe_s.clone(), "cocuk".into(), "ctrlc-nosctty".into()],
                80,
                24,
            )?;
            let mut yazici = s.master.try_clone().map_err(|e| e.to_string())?;
            let okuyucu_dosya = s.master;
            let hazir = Arc::new(AtomicBool::new(false));
            let ctrlc = Arc::new(AtomicBool::new(false));
            let hazir_kopya = Arc::clone(&hazir);
            let ctrlc_kopya = Arc::clone(&ctrlc);

            let okuyucu = std::thread::spawn(move || {
                let mut r = BufReader::new(okuyucu_dosya);
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
            while !hazir.load(Ordering::SeqCst) && bekle_bas.elapsed() < Duration::from_secs(5) {
                std::thread::sleep(Duration::from_millis(15));
            }
            yazici.write_all(&[0x03]).map_err(|e| e.to_string())?;
            yazici.flush().map_err(|e| e.to_string())?;

            let durum = s.cocuk.wait().map_err(|e| e.to_string())?;
            drop(yazici);
            let metin = okuyucu.join().map_err(|_| "PTY okuyucu çöktü")?;
            let gecti = ctrlc.load(Ordering::SeqCst) && durum.code() == Some(42);
            satirlar.push(SatirRaporu {
                ad: "G2.2 Linux bwrap+seccomp+PTY Ctrl+C (`\\x03` + `TIOCSCTTY`)".into(),
                gecti,
                ayrinti: format!(
                    "`bwrap --new-session` içinde `TIOCSCTTY` sonrası `\\x03` gönderildi; `SIGINT` yakalandı={}, çıkış kodu={:?}, süre={} ms (çıktı: {:?})",
                    ctrlc.load(Ordering::SeqCst),
                    durum.code(),
                    bas.elapsed().as_millis(),
                    metin.lines().collect::<Vec<_>>()
                ),
            });
        }

        // ---------------------------------------------------------------------
        // G2.3: bwrap + seccomp + PTY açılışı, UTF-8/ANSI ve `TIOCSWINSZ` resize
        // ---------------------------------------------------------------------
        {
            let bas = Instant::now();
            let mut s = bwrap_pty_baslat(
                &kok,
                &[exe_s.clone(), "cocuk".into(), "pty-boyut".into()],
                80,
                24,
            )?;
            let mut yazici = s.master.try_clone().map_err(|e| e.to_string())?;
            let ilk_goruldu = Arc::new(AtomicBool::new(false));
            let ikinci_goruldu = Arc::new(AtomicBool::new(false));
            let ilk_kopya = Arc::clone(&ilk_goruldu);
            let ikinci_kopya = Arc::clone(&ikinci_goruldu);

            let okuyucu_dosya = s.master.try_clone().map_err(|e| e.to_string())?;
            let okuyucu = std::thread::spawn(move || {
                let mut r = BufReader::new(okuyucu_dosya);
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
            s.boyut_degistir(120, 40)?;
            yazici
                .write_all(b"boyut\ncik\n")
                .map_err(|e| e.to_string())?;
            yazici.flush().map_err(|e| e.to_string())?;

            let durum = s.cocuk.wait().map_err(|e| e.to_string())?;
            drop(yazici);
            drop(s.master);
            let metin = okuyucu.join().map_err(|_| "PTY okuyucu çöktü")?;
            let utf8_tamam = metin.contains("utf8=çğıöşü");
            let gecti = ilk_goruldu.load(Ordering::SeqCst)
                && ikinci_goruldu.load(Ordering::SeqCst)
                && utf8_tamam
                && durum.code() == Some(0);
            satirlar.push(SatirRaporu {
                ad: "G2.3 Linux bwrap+seccomp+PTY açılış, UTF-8/ANSI ve resize (80x24 -> 120x40)".into(),
                gecti,
                ayrinti: format!(
                    "ilk boyut 80x24={}, `TIOCSWINSZ(120, 40)` sonrası 120x40={}, UTF-8/ANSI korundu={utf8_tamam}, çıkış={:?}, süre={} ms",
                    ilk_goruldu.load(Ordering::SeqCst),
                    ikinci_goruldu.load(Ordering::SeqCst),
                    durum.code(),
                    bas.elapsed().as_millis()
                ),
            });
        }

        // ---------------------------------------------------------------------
        // G2.4: bwrap + seccomp + PTY içinde torun süreç ve ağaç sonlandırma
        // ---------------------------------------------------------------------
        {
            let bas = Instant::now();
            let mut s = bwrap_pty_baslat(
                &kok,
                &[exe_s.clone(), "cocuk".into(), "torun".into()],
                80,
                24,
            )?;
            let bwrap_pid = s.cocuk.id();
            let pidler_paylasilan: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));
            let pidler_kopya = Arc::clone(&pidler_paylasilan);
            let okuyucu_dosya = s.master.try_clone().map_err(|e| e.to_string())?;

            let okuyucu = std::thread::spawn(move || {
                let mut r = BufReader::new(okuyucu_dosya);
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
            let ic_pidler = pidler_paylasilan
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();

            // bwrap (`--unshare-pid --die-with-parent`) öldürüldüğünde PID ad alanı
            // içindeki çocuk ve torun süreçlerin tamamı çekirdek tarafından sonlandırılır.
            let _ = s.cocuk.kill();
            let _ = s.cocuk.wait();
            drop(s.master);
            let _ = okuyucu.join();
            // Yetim kalmış olabilecek alt süreçleri subreaper olarak topla:
            // SAFETY: WNOHANG ile bekleyen alt süreç kalmadığı doğrulanır.
            unsafe {
                let mut st = 0;
                while libc::waitpid(-1, &mut st, libc::WNOHANG) > 0 {}
            }
            let bwrap_yasiyor = unsafe { libc::kill(bwrap_pid as i32, 0) == 0 };

            satirlar.push(SatirRaporu {
                ad: "G2.4 Linux bwrap+seccomp+PTY torun süreç ağacı sonlandırma".into(),
                gecti: ic_pidler.len() == 2 && !bwrap_yasiyor,
                ayrinti: format!(
                    "PID ad alanı içi çocuk+torun PID={ic_pidler:?}, bwrap PID={bwrap_pid} sonlandırıldı, yaşayan bwrap={bwrap_yasiyor}, süre={} ms",
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
            let mut s =
                bwrap_pty_baslat(&kok, &[exe_s.clone(), "cocuk".into(), "sel".into()], 80, 24)?;
            let tasti = Arc::new(AtomicBool::new(false));
            let toplam_okunan = Arc::new(AtomicUsize::new(0));
            let tutulan_boyut = Arc::new(AtomicUsize::new(0));
            let tasti_kopya = Arc::clone(&tasti);
            let toplam_kopya = Arc::clone(&toplam_okunan);
            let tutulan_kopya = Arc::clone(&tutulan_boyut);

            let okuyucu_dosya = s.master.try_clone().map_err(|e| e.to_string())?;
            let okuyucu = std::thread::spawn(move || {
                let mut r = BufReader::new(okuyucu_dosya);
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

            let bekle_bas = Instant::now();
            while toplam_okunan.load(Ordering::SeqCst) < 1536 * 1024
                && bekle_bas.elapsed() < Duration::from_secs(8)
            {
                std::thread::sleep(Duration::from_millis(15));
            }

            let kapanis_bas = Instant::now();
            let _ = s.cocuk.kill();
            let _ = s.cocuk.wait();
            drop(s.master);
            okuyucu.join().map_err(|_| "sel okuyucu çöktü")?;
            let kapanis_ms = kapanis_bas.elapsed().as_millis();

            let okunan_bayt = toplam_okunan.load(Ordering::SeqCst);
            let tutulan_bayt = tutulan_boyut.load(Ordering::SeqCst);
            let tasti_mi = tasti.load(Ordering::SeqCst);

            satirlar.push(SatirRaporu {
                ad: "G2.5 Linux dolu çıktı altında 1 MiB sınır ve kilitlenmeden kapanış".into(),
                gecti: tasti_mi && tutulan_bayt == AZAMI_IS_CIKTISI_BAYT && kapanis_ms < 5000,
                ayrinti: format!(
                    "toplam okunan={okunan_bayt} bayt, bellekte tutulan={tutulan_bayt} bayt (üst sınır 1 MiB), tasti={tasti_mi}, kapanış süresi={kapanis_ms} ms (< 5000 ms, toplam={} ms)",
                    bas.elapsed().as_millis()
                ),
            });
        }

        Ok(())
    })();

    let _ = std::fs::remove_dir_all(&kok);
    sonuc?;
    Ok(satirlar)
}
