//! Kalfa ağ köprüsü (`kalfa-kopru`).
//!
//! Kabın içinde çalışır. Windows'ta 127.0.0.1'de bir dinleyici açar,
//! bağlantıları Kalfa'nın adlandırılmış borusuna aktarır, asıl komutu
//! çocuk süreç olarak çalıştırır ve onun çıkış koduyla biter.

#[cfg(windows)]
mod windows_kopru {
    use std::collections::BTreeMap;
    use std::ffi::OsStr;
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::AsRawHandle;
    use std::time::{Duration, Instant};

    use tokio::net::TcpListener;
    use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::Threading::{
        CREATE_UNICODE_ENVIRONMENT, CreateProcessW, GetExitCodeProcess, INFINITE,
        PROCESS_INFORMATION, STARTF_USESTDHANDLES, STARTUPINFOW, WaitForSingleObject,
    };

    pub async fn calis() {
        let uygulama = match std::env::var("KALFA_KOPRU_UYGULAMA") {
            Ok(u) if !u.is_empty() => u,
            _ => {
                eprintln!("[kalfa-kopru] KALFA_KOPRU_UYGULAMA eksik ya da boş");
                std::process::exit(125);
            }
        };

        let satir = match std::env::var("KALFA_KOPRU_SATIR") {
            Ok(s) => s,
            _ => {
                eprintln!("[kalfa-kopru] KALFA_KOPRU_SATIR eksik");
                std::process::exit(125);
            }
        };

        let boru = match std::env::var("KALFA_KOPRU_BORU") {
            Ok(b) if !b.is_empty() => b,
            _ => {
                eprintln!("[kalfa-kopru] KALFA_KOPRU_BORU eksik ya da boş");
                std::process::exit(125);
            }
        };

        // 127.0.0.1:0 üzerinde dinleyici aç
        let dinleyici = match TcpListener::bind("127.0.0.1:0").await {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[kalfa-kopru] Dinleyici açılamadı: {e}");
                std::process::exit(125);
            }
        };

        let port = match dinleyici.local_addr() {
            Ok(a) => a.port(),
            Err(e) => {
                eprintln!("[kalfa-kopru] Yerel adres okunamadı: {e}");
                std::process::exit(125);
            }
        };

        // Ortam blokunu hazırla: köprünün ortamı, KALFA_KOPRU_* silinir, vekil_ortami(port) eklenir
        let mut ortam: BTreeMap<String, String> = std::env::vars().collect();
        ortam.retain(|k, _| {
            !k.eq_ignore_ascii_case("KALFA_KOPRU_UYGULAMA")
                && !k.eq_ignore_ascii_case("KALFA_KOPRU_SATIR")
                && !k.eq_ignore_ascii_case("KALFA_KOPRU_BORU")
        });
        // Windows'ta ad büyük/küçük harf ayırmaz: kaptaki `http_proxy` gibi bir
        // yazım kalırsa ortamda aynı ad iki kez olur.
        let vekil_degiskenleri = kalfa_sandbox::vekil::ortam::vekil_ortami(port);
        ortam.retain(|k, _| {
            !vekil_degiskenleri
                .iter()
                .any(|(v, _)| v.eq_ignore_ascii_case(k))
        });
        for (k, v) in vekil_degiskenleri {
            ortam.insert(k, v);
        }

        let mut ortam_bloku: Vec<u16> = Vec::new();
        for (k, v) in ortam {
            let girdi = format!("{k}={v}");
            ortam_bloku.extend(OsStr::new(&girdi).encode_wide());
            ortam_bloku.push(0);
        }
        ortam_bloku.push(0); // son çift NUL

        let uygulama_utf16: Vec<u16> = OsStr::new(&uygulama).encode_wide().chain(Some(0)).collect();
        let mut satir_utf16: Vec<u16> = OsStr::new(&satir).encode_wide().chain(Some(0)).collect();

        let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        si.dwFlags = STARTF_USESTDHANDLES;
        si.hStdInput = io::stdin().as_raw_handle() as HANDLE;
        si.hStdOutput = io::stdout().as_raw_handle() as HANDLE;
        si.hStdError = io::stderr().as_raw_handle() as HANDLE;

        let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

        // SAFETY: uygulama_utf16 ve satir_utf16 NUL ile sonlanan UTF-16 dizileridir;
        // ortam_bloku çift NUL ile sonlandırılmıştır. si ve pi geçerli göstericilerdir.
        let basarili = unsafe {
            CreateProcessW(
                uygulama_utf16.as_ptr(),
                satir_utf16.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1, // bInheritHandles: TRUE
                CREATE_UNICODE_ENVIRONMENT,
                ortam_bloku.as_ptr() as *const std::ffi::c_void,
                std::ptr::null(),
                &si,
                &mut pi,
            )
        };

        if basarili == 0 {
            eprintln!(
                "[kalfa-kopru] Komut başlatılamadı: {}",
                io::Error::last_os_error()
            );
            std::process::exit(127);
        }

        // SAFETY: pi.hThread kapatılır, süreç tutamacı beklenene kadar açık kalır.
        unsafe { CloseHandle(pi.hThread) };
        let surec_tutamaci = pi.hProcess as usize;

        // Boru aktarım döngüsü arka planda koşar
        let boru_yolu = boru.clone();
        tokio::spawn(async move {
            loop {
                let (mut tcp, _) = match dinleyici.accept().await {
                    Ok(b) => b,
                    Err(_) => break,
                };

                let b_yol = boru_yolu.clone();
                tokio::spawn(async move {
                    let mut pipe = match boru_ac(&b_yol).await {
                        Ok(p) => p,
                        Err(e) => {
                            eprintln!("[kalfa-kopru] Boruya bağlanılamadı: {e}");
                            return;
                        }
                    };
                    let _ = tokio::io::copy_bidirectional(&mut tcp, &mut pipe).await;
                });
            }
        });

        // Çocuğun bitmesini bekle
        let bekleme = tokio::task::spawn_blocking(move || {
            let h = surec_tutamaci as HANDLE;
            // SAFETY: h geçerli bir süreç tutamacıdır.
            unsafe {
                WaitForSingleObject(h, INFINITE);
                let mut kod = 0u32;
                GetExitCodeProcess(h, &mut kod);
                CloseHandle(h);
                kod
            }
        });

        let kod = match bekleme.await {
            Ok(k) => k as i32,
            Err(_) => 1,
        };

        std::process::exit(kod);
    }

    async fn boru_ac(boru: &str) -> io::Result<NamedPipeClient> {
        let baslangic = Instant::now();
        let azami = Duration::from_secs(5);
        loop {
            match ClientOptions::new().open(boru) {
                Ok(c) => return Ok(c),
                Err(e) if e.raw_os_error() == Some(231) => {
                    if baslangic.elapsed() >= azami {
                        return Err(e);
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

#[cfg(unix)]
mod linux_kopru {
    use std::net::TcpListener;
    use std::os::fd::{AsRawFd, RawFd};
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    use kalfa_sandbox::vekil::kanal;
    use kalfa_sandbox::vekil::ortam::vekil_ortami;

    pub async fn calis() {
        let args: Vec<String> = std::env::args().collect();
        let kod = match calistir(&args) {
            Ok(k) => k,
            Err(e) => {
                eprintln!("[kalfa-kopru] {e}");
                125
            }
        };
        std::process::exit(kod);
    }

    fn calistir(args: &[String]) -> Result<i32, String> {
        let ayrac = args
            .iter()
            .position(|s| s == "--")
            .ok_or("-- ayracı bulunamadı")?;
        let s = secenekler(&args[1..ayrac])?;
        let (program, argumanlar) = args[ayrac + 1..].split_first().ok_or("komut bulunamadı")?;

        // Kanalı CLOEXEC yap
        cloexec(s.kanal)?;

        // Dinleyiciyi 127.0.0.1:0'da aç ve CLOEXEC yap
        let dinleyici =
            TcpListener::bind("127.0.0.1:0").map_err(|e| format!("Dinleyici açılamadı: {e}"))?;
        cloexec(dinleyici.as_raw_fd())?;

        let port = dinleyici
            .local_addr()
            .map_err(|e| format!("Yerel adres okunamadı: {e}"))?
            .port();

        // Ortam: vekil_ortami(port)
        let mut cmd = Command::new(program);
        cmd.args(argumanlar);
        for (k, v) in vekil_ortami(port) {
            cmd.env(k, v);
        }

        let mut cocuk = cmd
            .spawn()
            .map_err(|e| format!("Komut başlatılamadı: {e}"))?;

        let kanal = s.kanal;
        std::thread::spawn(move || {
            for baglanti in dinleyici.incoming() {
                let Ok(baglanti) = baglanti else { continue };
                let _ = kanal::gonder(kanal, b"b", &[baglanti.as_raw_fd()]);
            }
        });

        let durum = cocuk
            .wait()
            .map_err(|e| format!("Komut beklenemedi: {e}"))?;
        Ok(durum
            .code()
            .unwrap_or_else(|| 128 + durum.signal().unwrap_or(0)))
    }

    struct Secenekler {
        kanal: RawFd,
    }

    fn secenekler(a: &[String]) -> Result<Secenekler, String> {
        let mut kanal = None;
        let mut i = 0;
        while i < a.len() {
            match a[i].as_str() {
                "--kanal" => {
                    i += 1;
                    let deger = a.get(i).ok_or("--kanal değersiz")?;
                    kanal = Some(
                        deger
                            .parse()
                            .map_err(|e| format!("--kanal geçersiz: {e}"))?,
                    );
                }
                diger => return Err(format!("Bilinmeyen seçenek: {diger}")),
            }
            i += 1;
        }
        let kanal = kanal.ok_or("--kanal gerekli")?;
        Ok(Secenekler { kanal })
    }

    fn cloexec(fd: RawFd) -> Result<(), String> {
        // SAFETY: `fcntl` yalnızca bayrakları okur ve yazar; geçersiz bir
        // tanımlayıcıda -1 döner, bellek güvenliğini etkilemez.
        let bayrak = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        if bayrak < 0 {
            return Err(format!(
                "fcntl(F_GETFD): {}",
                std::io::Error::last_os_error()
            ));
        }
        // SAFETY: yukarıdakiyle aynı.
        if unsafe { libc::fcntl(fd, libc::F_SETFD, bayrak | libc::FD_CLOEXEC) } < 0 {
            return Err(format!(
                "fcntl(F_SETFD): {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    #[cfg(windows)]
    windows_kopru::calis().await;

    #[cfg(unix)]
    linux_kopru::calis().await;
}
