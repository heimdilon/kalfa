//! Prototipin seccomp filtresi (yalnızca x86_64). Kalfa'nın ağlı profildeki
//! soket kuralını (`socket(AF_UNIX, …)` → `EPERM`; kalfa-sandbox
//! `linux/seccomp.rs`) ve tasarımın önerdiği `pidfd_getfd` engelini taklit
//! eder. Köprü filtreyi komutu başlatmadan önce kurar; komut devralır.

use std::io;

const BPF_LD_W_ABS: u16 = 0x20;
const BPF_JMP_JEQ_K: u16 = 0x15;
const BPF_RET_K: u16 = 0x06;
const AUDIT_ARCH_X86_64: u32 = 62 | 0x8000_0000 | 0x4000_0000;

fn komut(code: u16, jt: u8, jf: u8, k: u32) -> libc::sock_filter {
    libc::sock_filter { code, jt, jf, k }
}

pub fn kur() -> io::Result<()> {
    let eperm = libc::SECCOMP_RET_ERRNO | libc::EPERM as u32;
    let program = [
        // 0: mimari
        komut(BPF_LD_W_ABS, 0, 0, 4),
        komut(BPF_JMP_JEQ_K, 1, 0, AUDIT_ARCH_X86_64),
        komut(BPF_RET_K, 0, 0, libc::SECCOMP_RET_KILL_PROCESS),
        // 3: çağrı numarası
        komut(BPF_LD_W_ABS, 0, 0, 0),
        komut(BPF_JMP_JEQ_K, 0, 1, libc::SYS_pidfd_getfd as u32),
        komut(BPF_RET_K, 0, 0, eperm),
        komut(BPF_JMP_JEQ_K, 0, 3, libc::SYS_socket as u32),
        // 7: socket'in ilk argümanı (adres ailesi, alt 32 bit)
        komut(BPF_LD_W_ABS, 0, 0, 16),
        komut(BPF_JMP_JEQ_K, 0, 1, libc::AF_UNIX as u32),
        komut(BPF_RET_K, 0, 0, eperm),
        komut(BPF_RET_K, 0, 0, libc::SECCOMP_RET_ALLOW),
    ];
    let prog = libc::sock_fprog {
        len: program.len() as u16,
        filter: program.as_ptr() as *mut libc::sock_filter,
    };
    // SAFETY: yalnızca bu sürecin ve çocuklarının ayrıcalık bayrağını değiştirir.
    if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `prog` ve gösterdiği dizi çağrı boyunca yaşar; çekirdek kopyalar.
    let r = unsafe {
        libc::syscall(
            libc::SYS_seccomp,
            libc::SECCOMP_SET_MODE_FILTER,
            0,
            &prog as *const libc::sock_fprog,
        )
    };
    if r != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
