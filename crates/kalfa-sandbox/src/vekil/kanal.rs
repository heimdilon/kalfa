//! Köprü ile vekil arasındaki kanal (aşama 3 tasarımı §8.5). Köprü kabul ettiği
//! bağlantının tanımlayıcısını `SCM_RIGHTS` ile gönderir; vekil gelen
//! tanımlayıcıyı kullanmadan önce denetler. Kanal `SOCK_SEQPACKET`'tir: ileti
//! sınırları korunur, her ileti bir bayt ve bir tanımlayıcı taşır.

#![cfg(unix)]

use std::io;
use std::mem::{size_of, size_of_val, zeroed};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

/// Bir iletide alınabilen en çok tanımlayıcı. Sığmayanları çekirdek kapatır ve
/// `MSG_CTRUNC` verir; vekil tek tanımlayıcılı olmayan iletinin hepsini reddeder.
const EN_COK_TANIMLAYICI: usize = 8;

/// `SOCK_SEQPACKET` ve `SOCK_CLOEXEC` ile bir çift UNIX alan soketi açar.
pub fn kanal_cifti() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [-1; 2];
    // SAFETY: `fds` iki öğeliktir; başarıda çekirdek iki yeni tanımlayıcı yazar.
    let r = unsafe {
        libc::socketpair(
            libc::AF_UNIX,
            libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
            0,
            fds.as_mut_ptr(),
        )
    };
    if r != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: iki tanımlayıcı da yenidir ve yalnızca burada sahiplenilir.
    Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
}

/// `veri`'yi ve `fds`'yi tek iletide gönderir.
pub fn gonder(kanal: RawFd, veri: &[u8], fds: &[RawFd]) -> io::Result<()> {
    let fd_bayt = size_of_val(fds);
    // SAFETY: yalnızca boyut hesabı yapar.
    let alan = unsafe { libc::CMSG_SPACE(fd_bayt as u32) } as usize;
    // u64: denetim alanı `cmsghdr`'a hizalı olmalı.
    let mut kontrol = vec![0u64; alan.div_ceil(8)];
    let mut iov = libc::iovec {
        iov_base: veri.as_ptr() as *mut libc::c_void,
        iov_len: veri.len(),
    };
    // SAFETY: `msghdr` sıfırla başlatılabilen bir C yapısıdır.
    let mut msg: libc::msghdr = unsafe { zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    if !fds.is_empty() {
        msg.msg_control = kontrol.as_mut_ptr().cast();
        msg.msg_controllen = alan as _;
        // SAFETY: denetim alanı `CMSG_SPACE(fd_bayt)` bayt ve hizalıdır; ilk
        // başlık ve verisi bu alanın içindedir.
        unsafe {
            let c = libc::CMSG_FIRSTHDR(&msg);
            (*c).cmsg_level = libc::SOL_SOCKET;
            (*c).cmsg_type = libc::SCM_RIGHTS;
            (*c).cmsg_len = libc::CMSG_LEN(fd_bayt as u32) as _;
            std::ptr::copy_nonoverlapping(fds.as_ptr().cast::<u8>(), libc::CMSG_DATA(c), fd_bayt);
        }
    }
    // SAFETY: `msg`'nin gösterdiği tamponlar çağrı boyunca yaşar.
    let n = unsafe { libc::sendmsg(kanal, &msg, libc::MSG_NOSIGNAL) };
    if n < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Bir ileti okur; kanal kapandıysa `None`.
/// Tek tanımlayıcılı olmayan ya da kesik ileti reddedilir (`InvalidData` hatası).
pub fn al(kanal: RawFd) -> io::Result<Option<OwnedFd>> {
    let mut veri = [0u8; 64];
    // SAFETY: yalnızca boyut hesabı yapar.
    let alan =
        unsafe { libc::CMSG_SPACE((EN_COK_TANIMLAYICI * size_of::<RawFd>()) as u32) } as usize;
    let mut kontrol = vec![0u64; alan.div_ceil(8)];
    let mut iov = libc::iovec {
        iov_base: veri.as_mut_ptr().cast(),
        iov_len: veri.len(),
    };
    // SAFETY: `msghdr` sıfırla başlatılabilen bir C yapısıdır.
    let mut msg: libc::msghdr = unsafe { zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = kontrol.as_mut_ptr().cast();
    msg.msg_controllen = alan as _;
    let n = loop {
        // SAFETY: tamponlar çağrı boyunca yaşar.
        let n = unsafe { libc::recvmsg(kanal, &mut msg, libc::MSG_CMSG_CLOEXEC) };
        if n >= 0 {
            break n as usize;
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    };
    let mut fds = Vec::new();
    // SAFETY: çekirdeğin doldurduğu denetim alanında yürünür; her tanımlayıcı
    // yenidir ve bir kez sahiplenilir.
    unsafe {
        let mut c = libc::CMSG_FIRSTHDR(&msg);
        while !c.is_null() {
            if (*c).cmsg_level == libc::SOL_SOCKET && (*c).cmsg_type == libc::SCM_RIGHTS {
                let bayt = (*c).cmsg_len as usize - libc::CMSG_LEN(0) as usize;
                let p = libc::CMSG_DATA(c).cast::<RawFd>();
                for i in 0..bayt / size_of::<RawFd>() {
                    fds.push(OwnedFd::from_raw_fd(p.add(i).read_unaligned()));
                }
            }
            c = libc::CMSG_NXTHDR(&msg, c);
        }
    }
    if n == 0 && fds.is_empty() {
        return Ok(None);
    }
    let kesildi = msg.msg_flags & (libc::MSG_CTRUNC | libc::MSG_TRUNC) != 0;
    if kesildi || fds.len() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "tek tanımlayıcılı olmayan ya da kesik ileti",
        ));
    }
    Ok(Some(fds.remove(0)))
}

/// Gelen tanımlayıcı bağlı bir TCP soketi mi (§8.5)? Değilse nedeni.
pub fn dogrula(fd: &OwnedFd) -> Result<(), String> {
    let ham = fd.as_raw_fd();
    // SAFETY: `stat` sıfırla başlatılabilir; `fstat` yalnızca ona yazar.
    let mut st: libc::stat = unsafe { zeroed() };
    // SAFETY: `ham` bu çağrı boyunca açıktır.
    if unsafe { libc::fstat(ham, &mut st) } != 0 {
        return Err(format!("fstat: {}", io::Error::last_os_error()));
    }
    let tur = st.st_mode & libc::S_IFMT;
    if tur != libc::S_IFSOCK {
        return Err(format!("soket değil ({})", dosya_turu(tur)));
    }
    let aile = secenek(ham, libc::SO_DOMAIN)?;
    if aile != libc::AF_INET && aile != libc::AF_INET6 {
        return Err(format!("adres ailesi {}", aile_adi(aile)));
    }
    let soket_turu = secenek(ham, libc::SO_TYPE)?;
    if soket_turu != libc::SOCK_STREAM {
        return Err(format!("soket türü {soket_turu} (SOCK_STREAM değil)"));
    }
    let protokol = secenek(ham, libc::SO_PROTOCOL)?;
    if protokol != libc::IPPROTO_TCP {
        return Err(format!("protokol {protokol} (TCP değil)"));
    }
    if secenek(ham, libc::SO_ACCEPTCONN)? != 0 {
        return Err("dinleyen soket".into());
    }
    // SAFETY: `sockaddr_storage` sıfırla başlatılabilir; uzunluk ona göredir.
    let mut adres: libc::sockaddr_storage = unsafe { zeroed() };
    let mut uzunluk = size_of::<libc::sockaddr_storage>() as libc::socklen_t;
    // SAFETY: adres ve uzunluk çağrı boyunca yaşar.
    let r = unsafe {
        libc::getpeername(
            ham,
            (&mut adres as *mut libc::sockaddr_storage).cast(),
            &mut uzunluk,
        )
    };
    if r != 0 {
        return Err(format!("bağlı değil ({})", io::Error::last_os_error()));
    }
    Ok(())
}

fn secenek(fd: RawFd, ad: libc::c_int) -> Result<libc::c_int, String> {
    let mut deger: libc::c_int = 0;
    let mut uzunluk = size_of::<libc::c_int>() as libc::socklen_t;
    // SAFETY: `deger` bir `c_int`'tir; uzunluk ona göre verilir.
    let r = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            ad,
            (&mut deger as *mut libc::c_int).cast(),
            &mut uzunluk,
        )
    };
    if r != 0 {
        return Err(format!("getsockopt({ad}): {}", io::Error::last_os_error()));
    }
    Ok(deger)
}

fn dosya_turu(tur: libc::mode_t) -> &'static str {
    match tur {
        libc::S_IFREG => "normal dosya",
        libc::S_IFDIR => "dizin",
        libc::S_IFIFO => "boru",
        libc::S_IFCHR => "karakter aygıtı",
        libc::S_IFBLK => "blok aygıtı",
        libc::S_IFLNK => "bağlantı",
        _ => "bilinmeyen tür",
    }
}

fn aile_adi(aile: libc::c_int) -> String {
    match aile {
        libc::AF_UNIX => "AF_UNIX".into(),
        libc::AF_NETLINK => "AF_NETLINK".into(),
        libc::AF_PACKET => "AF_PACKET".into(),
        _ => aile.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};

    #[test]
    fn boru_ve_dosya_tanimlayicisi_reddedilir() {
        let dosya = tempfile::tempfile().unwrap();
        let err = dogrula(&OwnedFd::from(dosya)).unwrap_err();
        assert!(err.contains("soket değil"), "{err}");

        let mut boru = [-1; 2];
        unsafe {
            assert_eq!(libc::pipe(boru.as_mut_ptr()), 0);
            let r = OwnedFd::from_raw_fd(boru[0]);
            let _w = OwnedFd::from_raw_fd(boru[1]);
            let err = dogrula(&r).unwrap_err();
            assert!(err.contains("soket değil"), "{err}");
        }
    }

    #[test]
    fn dinleyen_ve_bagli_olmayan_soket_reddedilir() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let err = dogrula(&OwnedFd::from(listener)).unwrap_err();
        assert!(err.contains("dinleyen soket"), "{err}");

        // Bağlı olmayan TCP soketi
        unsafe {
            let s = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
            assert!(s >= 0);
            let fd = OwnedFd::from_raw_fd(s);
            let err = dogrula(&fd).unwrap_err();
            assert!(err.contains("bağlı değil"), "{err}");
        }
    }

    #[test]
    fn iki_tanimlayicili_ileti_reddedilir() {
        let (k1, k2) = kanal_cifti().unwrap();
        let f1 = tempfile::tempfile().unwrap();
        let f2 = tempfile::tempfile().unwrap();
        let fds = [f1.as_raw_fd(), f2.as_raw_fd()];
        gonder(k1.as_raw_fd(), b"x", &fds).unwrap();
        let res = al(k2.as_raw_fd());
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("tek tanımlayıcılı olmayan")
        );
    }

    #[test]
    fn bagli_tcp_soketi_kabul_edilir() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let client = TcpStream::connect(addr).unwrap();
        let (server, _) = listener.accept().unwrap();

        assert!(dogrula(&OwnedFd::from(client)).is_ok());
        assert!(dogrula(&OwnedFd::from(server)).is_ok());

        // Kanal üzerinden aktarım
        let (k1, k2) = kanal_cifti().unwrap();
        let test_client = TcpStream::connect(addr).unwrap();
        gonder(k1.as_raw_fd(), b"b", &[test_client.as_raw_fd()]).unwrap();
        let alinan = al(k2.as_raw_fd()).unwrap().expect("tanımlayıcı gelmeli");
        assert!(dogrula(&alinan).is_ok());
    }
}
