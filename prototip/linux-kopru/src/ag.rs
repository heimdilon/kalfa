//! Ağ ad alanı yardımcıları.

use std::io;
use std::mem::zeroed;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

/// Yeni ağ ad alanında `lo` kapalı gelir. bwrap `--unshare-net`'te onu
/// kendisi açar; prototip `unshare` kullandığı için köprü `--lo-ac` ile açar.
/// Ad alanının kullanıcı ad alanında `CAP_NET_ADMIN` gerekir.
pub fn geri_donguyu_ac() -> io::Result<()> {
    // SAFETY: yeni bir soket açar; sahipliği hemen `OwnedFd`'ye geçer.
    let ham = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0) };
    if ham < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `ham` yenidir ve yalnızca burada sahiplenilir.
    let soket = unsafe { OwnedFd::from_raw_fd(ham) };
    // SAFETY: `ifreq` sıfırla başlatılabilen bir C yapısıdır.
    let mut ifr: libc::ifreq = unsafe { zeroed() };
    for (hedef, bayt) in ifr.ifr_name.iter_mut().zip(b"lo") {
        *hedef = *bayt as libc::c_char;
    }
    // SAFETY: `ifr` çağrı boyunca yaşar; çekirdek bayrakları ona yazar.
    if unsafe { libc::ioctl(soket.as_raw_fd(), libc::SIOCGIFFLAGS, &mut ifr) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: SIOCGIFFLAGS birliğin bayrak alanını doldurdu.
    unsafe {
        ifr.ifr_ifru.ifru_flags |= libc::IFF_UP as libc::c_short;
    }
    // SAFETY: `ifr` çağrı boyunca yaşar.
    if unsafe { libc::ioctl(soket.as_raw_fd(), libc::SIOCSIFFLAGS, &ifr) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub fn cloexec(fd: RawFd) -> io::Result<()> {
    // SAFETY: yalnızca tanımlayıcının bayrağını değiştirir.
    if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
