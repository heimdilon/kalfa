//! Ağ ad alanının içinden denemeler. Her deneme sonucunu tek satırda yazar
//! (`sonuc: …`) ve 0 ile çıkar; yorumu `olc` yapar.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

pub fn ana(a: &[String]) -> i32 {
    let arg = |i: usize| a.get(i).map(String::as_str).unwrap_or("");
    let sonuc = match arg(0) {
        "dogrudan" => dogrudan(),
        "dns" => dns(arg(1)),
        "geri-donus" => geri_donus(arg(1)),
        "fd" => fd_listesi(),
        "unix" => unix_soketi(),
        "yuk" => yuk(arg(1), arg(2)),
        diger => {
            eprintln!("bilinmeyen deneme: {diger}");
            return 2;
        }
    };
    println!("sonuc: {sonuc}");
    0
}

/// Belgelere ayrılmış bir adrese (RFC 5737, TEST-NET-1) doğrudan bağlantı.
/// Ad alanında yol olmadığı için hemen başarısız olmalı.
fn dogrudan() -> String {
    let adres: SocketAddr = ([192, 0, 2, 1], 443).into();
    let baslangic = Instant::now();
    match TcpStream::connect_timeout(&adres, Duration::from_secs(3)) {
        Ok(_) => "BAĞLANDI".into(),
        Err(e) => format!("hata: {e} ({} ms)", baslangic.elapsed().as_millis()),
    }
}

fn dns(ad: &str) -> String {
    let baslangic = Instant::now();
    match (ad, 443).to_socket_addrs() {
        Ok(adresler) => format!("ÇÖZÜLDÜ: {:?}", adresler.collect::<Vec<_>>()),
        Err(e) => format!("hata: {e} ({} ms)", baslangic.elapsed().as_millis()),
    }
}

/// Dış ad alanındaki bir sunucunun portu; kabın geri döngüsü ayrı olduğu için
/// ulaşılamamalı.
fn geri_donus(port: &str) -> String {
    let Ok(port) = port.parse::<u16>() else {
        return "hata: port yok".into();
    };
    let adres: SocketAddr = ([127, 0, 0, 1], port).into();
    match TcpStream::connect_timeout(&adres, Duration::from_secs(3)) {
        Ok(_) => "BAĞLANDI".into(),
        Err(e) => format!("hata: {e}"),
    }
}

fn fd_listesi() -> String {
    let Ok(girdiler) = std::fs::read_dir("/proc/self/fd") else {
        return "hata: /proc/self/fd okunamadı".into();
    };
    let mut satirlar: Vec<String> = girdiler
        .flatten()
        .map(|g| {
            let hedef = std::fs::read_link(g.path())
                .map(|h| h.display().to_string())
                .unwrap_or_else(|e| e.to_string());
            format!("{}={hedef}", g.file_name().to_string_lossy())
        })
        .collect();
    satirlar.sort();
    satirlar.join(" ")
}

fn unix_soketi() -> String {
    // SAFETY: yeni bir soket açar; başarıda hemen kapatılır.
    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0) };
    if fd < 0 {
        return format!("hata: {}", std::io::Error::last_os_error());
    }
    // SAFETY: `fd` bu işlevde açıldı ve başka yerde kullanılmıyor.
    unsafe { libc::close(fd) };
    "AÇILDI".into()
}

/// Vekil üzerinden `yerel.test:80`'e `n` bağlantı (`paralel` iş parçacığıyla);
/// bağlantı başına ortalama süre.
fn yuk(n: &str, paralel: &str) -> String {
    let (Ok(n), Ok(paralel)) = (n.parse::<usize>(), paralel.parse::<usize>()) else {
        return "hata: sayı yok".into();
    };
    let Some(vekil) = vekil_adresi() else {
        return "hata: HTTPS_PROXY yok".into();
    };
    let baslangic = Instant::now();
    let isler: Vec<_> = (0..paralel)
        .map(|i| {
            let adet = n / paralel + usize::from(i < n % paralel);
            std::thread::spawn(move || {
                let mut sureler = Vec::with_capacity(adet);
                for _ in 0..adet {
                    let t = Instant::now();
                    tek_istek(vekil).ok()?;
                    sureler.push(t.elapsed());
                }
                Some(sureler)
            })
        })
        .collect();
    let mut sureler = Vec::new();
    for is in isler {
        match is.join() {
            Ok(Some(s)) => sureler.extend(s),
            _ => return "hata: bir istek başarısız".into(),
        }
    }
    let toplam = baslangic.elapsed();
    sureler.sort();
    let ortalama = sureler.iter().sum::<Duration>() / sureler.len().max(1) as u32;
    let medyan = sureler.get(sureler.len() / 2).copied().unwrap_or_default();
    format!(
        "{} istek, ortalama {:.2} ms, medyan {:.2} ms, toplam {} ms",
        sureler.len(),
        ortalama.as_secs_f64() * 1000.0,
        medyan.as_secs_f64() * 1000.0,
        toplam.as_millis()
    )
}

fn vekil_adresi() -> Option<SocketAddr> {
    let deger = std::env::var("HTTPS_PROXY").ok()?;
    deger.strip_prefix("http://")?.parse().ok()
}

fn tek_istek(vekil: SocketAddr) -> std::io::Result<()> {
    let mut s = TcpStream::connect(vekil)?;
    s.write_all(b"CONNECT yerel.test:80 HTTP/1.1\r\nHost: yerel.test:80\r\n\r\n")?;
    let mut bas = Vec::new();
    let mut bayt = [0u8; 1];
    while !bas.ends_with(b"\r\n\r\n") {
        if s.read(&mut bayt)? == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        bas.push(bayt[0]);
    }
    s.write_all(b"GET / HTTP/1.1\r\nHost: yerel.test\r\nConnection: close\r\n\r\n")?;
    let mut yanit = Vec::new();
    s.read_to_end(&mut yanit)?;
    if !yanit.starts_with(b"HTTP/1.1 200") {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    Ok(())
}
