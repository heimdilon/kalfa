//! G6 ölçümü (aşama 3 tasarımı §8.6): Windows'ta `internetClient`'sız
//! AppContainer'da köprü ve vekil yolunun çalışıp çalışmadığı. Ürün kodu
//! değildir; ölçüm sonuçları bulgu raporuna yazılır.
//!
//! ```text
//! cargo test -p kalfa-tools --features yalitim-testleri --test g6_olcum -- --ignored --nocapture
//! ```
//!
//! - Ö1: aynı kaptaki iki süreç `127.0.0.1` üzerinden konuşur mu?
//! - Ö2: kap, paket SID'ine açılmış adlandırılmış boruya bağlanır mı?
//!   (bütünlük etiketiyle ve etiketsiz)
//! - Ö3: kaptaki köprü (Python) → boru → kabın dışındaki CONNECT vekili
//!   yolundan curl, git, cargo, npm, node ve pip çalışır mı? Vekilsiz curl
//!   başarısız olmalıdır (kap ağsız).
//! - Ö4: çalışma alanı dışındaki bir dizinden ve Program Files'tan ikili
//!   çalışır mı?
#![cfg(all(windows, feature = "yalitim-testleri"))]

use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::FromRawHandle;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use kalfa_core::arac::{Arac, AracHatasi, CiktiGonderici};
use kalfa_core::izin::Siniflandirici;
use kalfa_sandbox::windows::ace_kaydi::AceKaydi;
use kalfa_sandbox::windows::guvenlik;
use kalfa_sandbox::windows::yalitici::{WindowsAyari, WindowsYalitici};
use kalfa_sandbox::{AlanBilgisi, GitKimligi, Tarama, Yalitici, YalitimDurumu, alan_kimligi};
use kalfa_tools::{CalismaAlani, KabukAraci, KabukGuveni};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio_util::sync::CancellationToken;
use windows_sys::Win32::Foundation::{ERROR_PIPE_CONNECTED, GetLastError, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
    PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};

const PIPE_ACCESS_DUPLEX: u32 = 3;

#[derive(Default)]
struct Guven(AtomicBool);

impl KabukGuveni for Guven {
    fn guvenildi_mi(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
    fn guvenildi(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

const BETIK: &str = r#"
import os, socket, subprocess, sys, threading, time

def o1():
    s = socket.socket(); s.bind(("127.0.0.1", 0)); s.listen(1)
    port = s.getsockname()[1]
    kod = "import socket,sys; c=socket.create_connection(('127.0.0.1',%d),5); c.sendall(b'selam'); print(c.recv(16).decode())" % port
    try:
        cocuk = subprocess.Popen([sys.executable, "-c", kod], stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    except Exception as e:
        print("O1 COCUK BASLAMADI", repr(e)); return
    s.settimeout(10)
    try:
        b, _ = s.accept(); veri = b.recv(16); b.sendall(b"merhaba"); b.close()
        cikti = cocuk.communicate(timeout=10)[0].decode(errors="replace").strip()
        print("O1 SONUC dinleyen aldi=%r cocuk aldi=%r" % (veri, cikti))
    except Exception as e:
        cocuk.kill()
        print("O1 HATA", repr(e), cocuk.communicate()[0].decode(errors="replace").strip())

def o2(boru):
    try:
        with open(boru, "r+b", buffering=0) as f:
            f.write(b"selam\n"); print("O2 SONUC", boru, f.read(16))
    except Exception as e:
        print("O2 HATA", boru, repr(e))

def kopru(boru):
    import asyncio
    hazir = threading.Event(); kutu = {}
    def calis():
        async def ana():
            loop = asyncio.get_running_loop()
            async def aktar(r, w):
                try:
                    while True:
                        v = await r.read(65536)
                        if not v: break
                        w.write(v); await w.drain()
                except Exception:
                    pass
            async def baglanti(cr, cw):
                try:
                    pr = asyncio.StreamReader()
                    proto = asyncio.StreamReaderProtocol(pr)
                    tr, _ = await loop.create_pipe_connection(lambda: proto, boru)
                    pw = asyncio.StreamWriter(tr, proto, pr, loop)
                except Exception as e:
                    print("KOPRU BORU HATASI", repr(e), flush=True); cw.close(); return
                gorevler = [asyncio.ensure_future(aktar(cr, pw)), asyncio.ensure_future(aktar(pr, cw))]
                await asyncio.wait(gorevler, return_when=asyncio.FIRST_COMPLETED)
                for g in gorevler: g.cancel()
                pw.close(); cw.close()
            sunucu = await asyncio.start_server(baglanti, "127.0.0.1", 0)
            kutu["port"] = sunucu.sockets[0].getsockname()[1]
            hazir.set()
            await sunucu.serve_forever()
        asyncio.run(ana())
    threading.Thread(target=calis, daemon=True).start()
    hazir.wait(10)
    return kutu["port"]

def calistir(ad, komut, ortam, zaman=120):
    bas = time.time()
    try:
        r = subprocess.run(komut, env=ortam, capture_output=True, timeout=zaman, cwd=os.getcwd())
        cikti = (r.stdout + r.stderr).decode(errors="replace").strip().splitlines()
        print("O3 %-12s kod=%s sure=%.1fs | %s" % (ad, r.returncode, time.time() - bas, " / ".join([l for l in cikti if "rror" in l or "denied" in l or "engellendi" in l][:6] or cikti[-3:])[:900]), flush=True)
    except Exception as e:
        print("O3 %-12s HATA %r" % (ad, e), flush=True)

def o3(boru):
    port = kopru(boru)
    vekil = "http://127.0.0.1:%d" % port
    alan = os.getcwd()
    taban = dict(os.environ)
    ortam = dict(taban)
    cevrimdisi = [k for k in ortam if "OFFLINE" in k.upper()]
    for k in cevrimdisi: del ortam[k]
    print("O3 ortamdan kaldırılan:", cevrimdisi, flush=True)
    for k in ("HTTPS_PROXY", "HTTP_PROXY", "https_proxy", "http_proxy", "ALL_PROXY",
              "npm_config_https_proxy", "npm_config_proxy", "CARGO_HTTP_PROXY", "PIP_PROXY"):
        ortam[k] = vekil
    ortam["NO_PROXY"] = ortam["no_proxy"] = "localhost,127.0.0.1,::1"
    ortam["NODE_USE_ENV_PROXY"] = "1"
    ortam["CARGO_HOME"] = os.path.join(alan, ".g6-cargo")
    ortam["npm_config_cache"] = os.path.join(alan, ".g6-npm")
    ortam["GIT_CONFIG_COUNT"] = "1"; ortam["GIT_CONFIG_KEY_0"] = "http.proxy"; ortam["GIT_CONFIG_VALUE_0"] = vekil
    os.makedirs(os.path.join(alan, "g6c", "src"), exist_ok=True)
    open(os.path.join(alan, "g6c", "Cargo.toml"), "w").write('[package]\nname="g6c"\nversion="0.1.0"\nedition="2021"\n[dependencies]\nitoa="1"\n')
    open(os.path.join(alan, "g6c", "src", "lib.rs"), "w").write("")
    calistir("curl-vekilsiz", ["curl.exe", "-sS", "-m", "15", "-o", "NUL", "-w", "%{http_code}", "https://index.crates.io/config.json"], taban, 30)
    calistir("curl", ["curl.exe", "-sS", "-m", "30", "-o", "NUL", "-w", "%{http_code}", "https://index.crates.io/config.json"], ortam)
    calistir("curl-norevoke", ["curl.exe", "-sS", "--ssl-no-revoke", "-m", "30", "-o", "NUL", "-w", "%{http_code}", "https://index.crates.io/config.json"], ortam)
    calistir("git", ["git", "ls-remote", "https://github.com/octocat/Hello-World", "HEAD"], ortam)
    calistir("cargo", ["cargo", "fetch", "--manifest-path", os.path.join(alan, "g6c", "Cargo.toml")], ortam)
    o2 = dict(ortam); o2["CARGO_HTTP_CHECK_REVOKE"] = "false"
    calistir("cargo-norev", ["cargo", "fetch", "--manifest-path", os.path.join(alan, "g6c", "Cargo.toml")], o2)
    calistir("npm", ["cmd", "/c", "npm", "view", "left-pad", "version"], ortam)
    calistir("node-fetch", ["node", "-e", "fetch('https://registry.npmjs.org/left-pad').then(r=>console.log(r.status)).catch(e=>{console.log(String(e.cause||e));process.exit(1)})"], ortam)
    calistir("pip", [sys.executable, "-m", "pip", "download", "six", "--no-deps", "--no-cache-dir", "-d", os.path.join(alan, ".g6-pip")], ortam)

def o4(yol):
    try:
        r = subprocess.run([yol, "--version"], capture_output=True, timeout=30)
        print("O4 SONUC", yol, "kod=", r.returncode, (r.stdout + r.stderr).decode(errors="replace").strip()[:120])
    except Exception as e:
        print("O4 HATA", yol, repr(e))

kip = sys.argv[1]
if kip == "o1": o1()
elif kip == "o2":
    for b in sys.argv[2:]: o2(b)
elif kip == "o3": o3(sys.argv[2])
elif kip == "o5":
    alan = os.getcwd()
    ortam = {k: v for k, v in os.environ.items() if "OFFLINE" not in k.upper()}
    ortam["CARGO_HOME"] = os.path.join(alan, ".g6-cargo5")
    calistir("cargo-dogrudan", ["cargo", "fetch", "--manifest-path", os.path.join(alan, "g6c", "Cargo.toml")], ortam)
    calistir("pip-dogrudan", [sys.executable, "-m", "pip", "download", "six", "--no-deps", "--no-cache-dir", "-d", os.path.join(alan, ".g6-pip5")], ortam)
elif kip == "o4":
    for y in sys.argv[2:]: o4(y)
sys.stdout.flush()
"#;

/// Kabın dışında, verilen SDDL'li bir adlandırılmış boru sunucusu. Her bağlantı
/// `isle` ile ayrı bir iş parçacığında işlenir; sunucu süreç bitince ölür.
fn boru_sunucusu(ad: String, sddl: String, isle: fn(File, Arc<Mutex<Vec<String>>>), gunluk: Arc<Mutex<Vec<String>>>) {
    let genis: Vec<u16> = OsStr::new(&ad).encode_wide().chain([0]).collect();
    let sddl_genis: Vec<u16> = OsStr::new(&sddl).encode_wide().chain([0]).collect();
    let mut tanim: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    // SAFETY: sddl_genis NUL ile biten geçerli bir UTF-16 dizidir; tanım
    // Windows'un ayırdığı bellektir ve süreç boyunca serbest bırakılmaz.
    let tamam = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl_genis.as_ptr(), 1, &mut tanim, std::ptr::null_mut())
    };
    assert!(tamam != 0, "SDDL çözülemedi: {}", io::Error::last_os_error());
    let tanim_adresi = tanim as usize;
    thread::spawn(move || {
        loop {
            let sa = SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: tanim_adresi as PSECURITY_DESCRIPTOR,
                bInheritHandle: 0,
            };
            // SAFETY: ad NUL ile biten UTF-16; sa ve içindeki tanım çağrı boyunca geçerli.
            let h = unsafe {
                CreateNamedPipeW(
                    genis.as_ptr(),
                    PIPE_ACCESS_DUPLEX,
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                    PIPE_UNLIMITED_INSTANCES,
                    65536,
                    65536,
                    0,
                    &sa,
                )
            };
            if h == INVALID_HANDLE_VALUE {
                gunluk.lock().unwrap().push(format!("boru oluşturulamadı: {}", io::Error::last_os_error()));
                return;
            }
            // SAFETY: h geçerli bir boru tutamacıdır.
            let bagli = unsafe { ConnectNamedPipe(h, std::ptr::null_mut()) } != 0
                || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED;
            // SAFETY: h'nin sahibi artık File'dır; başka yerde kapatılmaz.
            let dosya = unsafe { File::from_raw_handle(h as _) };
            if bagli {
                let g = gunluk.clone();
                thread::spawn(move || isle(dosya, g));
            }
        }
    });
}

fn yanki(mut boru: File, gunluk: Arc<Mutex<Vec<String>>>) {
    let mut b = [0u8; 16];
    let n = boru.read(&mut b).unwrap_or(0);
    gunluk.lock().unwrap().push(format!("yankı aldı {:?}", String::from_utf8_lossy(&b[..n])));
    let _ = boru.write_all(b"merhaba\n");
}

fn sddl_coz(sddl: &str) -> usize {
    let sddl_genis: Vec<u16> = OsStr::new(sddl).encode_wide().chain([0]).collect();
    let mut tanim: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    // SAFETY: sddl_genis NUL ile biten UTF-16; tanım süreç boyunca serbest bırakılmaz.
    let tamam = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl_genis.as_ptr(), 1, &mut tanim, std::ptr::null_mut())
    };
    assert!(tamam != 0, "SDDL çözülemedi: {}", io::Error::last_os_error());
    tanim as usize
}

fn asenkron_boru(ad: &str, tanim: usize, ilk: bool) -> io::Result<NamedPipeServer> {
    let mut sa = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: tanim as PSECURITY_DESCRIPTOR,
        bInheritHandle: 0,
    };
    // SAFETY: sa ve içindeki tanım çağrı boyunca geçerli.
    unsafe {
        ServerOptions::new()
            .first_pipe_instance(ilk)
            .create_with_security_attributes_raw(ad, &mut sa as *mut _ as *mut std::ffi::c_void)
    }
}

/// CONNECT vekili, asenkron (overlapped) boruyla: izin listesi yok (ölçüm);
/// her hedef günlüğe yazılır.
fn vekil_sunucusu(ad: String, sddl: String, gunluk: Arc<Mutex<Vec<String>>>) {
    let tanim = sddl_coz(&sddl);
    let mut sunucu = asenkron_boru(&ad, tanim, true).unwrap();
    tokio::spawn(async move {
        loop {
            if sunucu.connect().await.is_err() {
                continue;
            }
            let bagli = sunucu;
            sunucu = asenkron_boru(&ad, tanim, false).unwrap();
            tokio::spawn(vekil_isle(bagli, gunluk.clone()));
        }
    });
}

async fn vekil_isle(boru: NamedPipeServer, gunluk: Arc<Mutex<Vec<String>>>) {
    let mut okuyucu = tokio::io::BufReader::new(boru);
    let mut ilk = String::new();
    if okuyucu.read_line(&mut ilk).await.is_err() {
        return;
    }
    loop {
        let mut satir = String::new();
        if okuyucu.read_line(&mut satir).await.unwrap_or(0) == 0 || satir.trim().is_empty() {
            break;
        }
    }
    let mut boru = okuyucu.into_inner();
    let parcalar: Vec<&str> = ilk.split_whitespace().collect();
    if parcalar.first() != Some(&"CONNECT") || parcalar.len() < 2 {
        gunluk.lock().unwrap().push(format!("CONNECT değil: {:?}", ilk.trim()));
        let _ = boru.write_all(b"HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\n\r\n").await;
        return;
    }
    let hedef = parcalar[1].to_string();
    let mut tcp = match tokio::net::TcpStream::connect(&hedef).await {
        Ok(t) => t,
        Err(e) => {
            gunluk.lock().unwrap().push(format!("{hedef}: bağlanamadı {e}"));
            let _ = boru.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n").await;
            return;
        }
    };
    let _ = boru.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await;
    let sonuc = tokio::io::copy_bidirectional(&mut boru, &mut tcp).await;
    gunluk.lock().unwrap().push(format!("{hedef}: tünel {sonuc:?}"));
}

struct Kurgu {
    arac: KabukAraci,
    yalitici: Arc<WindowsYalitici>,
    kok: PathBuf,
    veri: TempDir,
}

impl Drop for Kurgu {
    fn drop(&mut self) {
        let alan = AlanBilgisi {
            kok: self.kok.clone(),
            gizli: Tarama::default(),
            korumali: Vec::new(),
            git_dizinleri: Vec::new(),
        };
        if let Err(e) = self.yalitici.alani_kaldir(&alan) {
            eprintln!("alan kaldırılamadı: {e}");
        }
        drop(WindowsYalitici::yeni(
            WindowsAyari {
                ev: self.veri.path().join("bos-ev"),
                yalitim_dizini: self.veri.path().join("yalitim"),
                ek_okunur: Vec::new(),
                kalfa_araclari: Vec::new(),
                git: GitKimligi::default(),
            },
            Box::new(|_| {}),
        ));
        let kayit = AceKaydi::oku(&self.veri.path().join("yalitim").join("ace-kaydi.jsonl"));
        eprintln!("ACE kaydı temiz: {}", !kayit.bozuk_mu() && kayit.girdiler().is_empty());
    }
}

fn arac_dizinleri() -> Vec<PathBuf> {
    ["node", "python"]
        .iter()
        .filter_map(|ad| {
            let cikti = Command::new("where.exe").arg(ad).output().ok()?;
            String::from_utf8_lossy(&cikti.stdout)
                .lines()
                .map(|s| PathBuf::from(s.trim()))
                .find(|y| !y.to_string_lossy().contains("WindowsApps"))
                .and_then(|y| y.parent().map(Path::to_path_buf))
        })
        .collect()
}

async fn calistir(arac: &KabukAraci, a: Value) -> String {
    let h = arac.hazirla(&a).await.expect("hazırlık");
    match arac
        .calistir_akisli(&a, &h, CancellationToken::new(), CiktiGonderici::bos())
        .await
    {
        Ok(c) => c.icerik,
        Err(AracHatasi::Basarisiz(c)) => c.icerik,
        Err(e) => format!("ARAÇ HATASI {e:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "G6 ölçümü; gerçek yalıtım ve internet ister"]
async fn g6_olcum() {
    let alan_dizini = TempDir::new().unwrap();
    let kok = alan_dizini.path().canonicalize().unwrap();
    std::fs::write(kok.join("g6.py"), BETIK).unwrap();
    let veri = TempDir::new().unwrap();
    let ev = PathBuf::from(std::env::var_os("USERPROFILE").unwrap());
    let yalitici = Arc::new(WindowsYalitici::yeni(
        WindowsAyari {
            ev,
            yalitim_dizini: veri.path().join("yalitim"),
            ek_okunur: arac_dizinleri(),
            kalfa_araclari: Vec::new(),
            git: GitKimligi::default(),
        },
        Box::new(|_| {}),
    ));
    match yalitici.durum() {
        YalitimDurumu::Hazir { .. } => {}
        d => panic!("Windows yalıtımı hazır değil: {d:?}"),
    }
    let s = Arc::new(Siniflandirici::yeni(&kok, &[]).unwrap());
    let k = Kurgu {
        arac: KabukAraci::yeni(
            CalismaAlani::ac(s).unwrap(),
            yalitici.clone(),
            Arc::new(Guven::default()),
            Arc::new(AtomicBool::new(false)),
        ),
        yalitici,
        kok: kok.clone(),
        veri,
    };

    let py = |args: &[&str]| {
        let argumanlar = [&["g6.py"][..], args].concat();
        json!({ "program": "python", "argumanlar": argumanlar, "ag": false, "zaman_asimi_sn": 900 })
    };

    // Alanı hazırlar ve paket SID'ini kurar; yalnızca Python'un kapta çalıştığını gösterir.
    println!("--- hazırlık\n{}", calistir(&k.arac, json!({ "program": "python", "argumanlar": ["--version"], "ag": false })).await);

    println!("--- Ö1\n{}", calistir(&k.arac, py(&["o1"])).await);

    let paket = guvenlik::profil_sidi(&format!("Kalfa.Alan.{}", alan_kimligi(&kok))).unwrap().metin();
    let kullanici = guvenlik::kullanici_sidi().unwrap().metin();
    let gunluk = Arc::new(Mutex::new(Vec::new()));
    let ek = std::process::id();
    let etiketli = format!(r"\\.\pipe\kalfa-g6-etiketli-{ek}");
    let etiketsiz = format!(r"\\.\pipe\kalfa-g6-etiketsiz-{ek}");
    let yalniz_kullanici = format!(r"\\.\pipe\kalfa-g6-kullanici-{ek}");
    boru_sunucusu(etiketli.clone(), format!("D:P(A;;GA;;;SY)(A;;GA;;;{kullanici})(A;;GRGW;;;{paket})S:(ML;;NW;;;LW)"), yanki, gunluk.clone());
    boru_sunucusu(etiketsiz.clone(), format!("D:P(A;;GA;;;SY)(A;;GA;;;{kullanici})(A;;GRGW;;;{paket})"), yanki, gunluk.clone());
    boru_sunucusu(yalniz_kullanici.clone(), format!("D:P(A;;GA;;;SY)(A;;GA;;;{kullanici})S:(ML;;NW;;;LW)"), yanki, gunluk.clone());
    println!("--- Ö2 (paket {paket})\n{}", calistir(&k.arac, py(&["o2", &etiketli, &etiketsiz, &yalniz_kullanici])).await);

    let vekil_borusu = format!(r"\\.\pipe\kalfa-g6-vekil-{ek}");
    vekil_sunucusu(vekil_borusu.clone(), format!("D:P(A;;GA;;;SY)(A;;GA;;;{kullanici})(A;;GRGW;;;{paket})"), gunluk.clone());
    println!("--- Ö3\n{}", calistir(&k.arac, py(&["o3", &vekil_borusu])).await);

    let disarida = TempDir::new().unwrap();
    let kopya = disarida.path().join("where.exe");
    std::fs::copy(r"C:\Windows\System32\where.exe", &kopya).unwrap();
    let program_files = r"C:\Program Files\Git\cmd\git.exe";
    println!(
        "--- Ö4\n{}",
        calistir(&k.arac, py(&["o4", &kopya.to_string_lossy(), program_files])).await
    );

    println!(
        "--- Ö5 (ag: true, vekilsiz)
{}",
        calistir(&k.arac, json!({ "program": "python", "argumanlar": ["g6.py", "o5"], "ag": true, "zaman_asimi_sn": 600 })).await
    );

    println!("--- vekil ve boru günlüğü");
    for satir in gunluk.lock().unwrap().iter() {
        println!("{satir}");
    }
}
