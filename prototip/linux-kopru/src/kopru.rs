//! Köprü (aşama 3 tasarımı §8.5): ağ ad alanının içinde `127.0.0.1`'de dinler,
//! komutu vekil ortamıyla başlatır ve kabul ettiği her bağlantının
//! tanımlayıcısını kanaldan vekile verir. Baytları kendisi taşımaz.

use std::net::TcpListener;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

use crate::{ag, kanal, seccomp};

pub fn ana(a: &[String]) -> i32 {
    match calistir(a) {
        Ok(kod) => kod,
        Err(e) => {
            eprintln!("kopru: {e}");
            125
        }
    }
}

struct Secenekler {
    kanal: RawFd,
    lo_ac: bool,
    seccomp: bool,
}

fn secenekler(a: &[String]) -> Result<Secenekler, String> {
    let mut kanal = None;
    let mut s = Secenekler {
        kanal: -1,
        lo_ac: false,
        seccomp: false,
    };
    let mut i = 0;
    while i < a.len() {
        match a[i].as_str() {
            "--kanal" => {
                i += 1;
                let deger = a.get(i).ok_or("--kanal değersiz")?;
                kanal = Some(deger.parse().map_err(|e| format!("--kanal: {e}"))?);
            }
            "--lo-ac" => s.lo_ac = true,
            "--seccomp" => s.seccomp = true,
            diger => return Err(format!("bilinmeyen seçenek: {diger}")),
        }
        i += 1;
    }
    s.kanal = kanal.ok_or("--kanal gerekli")?;
    Ok(s)
}

fn calistir(a: &[String]) -> Result<i32, String> {
    let ayrac = a.iter().position(|s| s == "--").ok_or("-- yok")?;
    let s = secenekler(&a[..ayrac])?;
    let (program, argumanlar) = a[ayrac + 1..].split_first().ok_or("komut yok")?;
    if s.lo_ac {
        ag::geri_donguyu_ac().map_err(|e| format!("lo açılamadı: {e}"))?;
    }
    // Kanal komuta geçmez.
    ag::cloexec(s.kanal).map_err(|e| format!("kanal: {e}"))?;
    let dinleyici = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("dinleme: {e}"))?;
    let port = dinleyici.local_addr().map_err(|e| e.to_string())?.port();
    if s.seccomp {
        seccomp::kur().map_err(|e| format!("seccomp: {e}"))?;
    }
    let mut cocuk = Command::new(program)
        .args(argumanlar)
        .envs(ortam(&format!("http://127.0.0.1:{port}")))
        .spawn()
        .map_err(|e| format!("{program}: {e}"))?;
    let kanal = s.kanal;
    std::thread::spawn(move || {
        for baglanti in dinleyici.incoming() {
            let Ok(baglanti) = baglanti else { continue };
            // Vekil yoksa bağlantı kapanır; komut hatayı kendisi görür.
            let _ = kanal::gonder(kanal, b"b", &[baglanti.as_raw_fd()]);
        }
    });
    let durum = cocuk.wait().map_err(|e| e.to_string())?;
    Ok(durum
        .code()
        .unwrap_or_else(|| 128 + durum.signal().unwrap_or(0)))
}

/// Tasarımın §8.5'teki ortamı: köprü bunları komutun ortamına ekler.
pub fn ortam(vekil: &str) -> Vec<(&'static str, String)> {
    let mut o: Vec<(&'static str, String)> = [
        "HTTPS_PROXY",
        "HTTP_PROXY",
        "https_proxy",
        "http_proxy",
        "ALL_PROXY",
    ]
    .into_iter()
    .map(|ad| (ad, vekil.to_string()))
    .collect();
    o.push(("NO_PROXY", "localhost,127.0.0.1,::1".into()));
    o
}
