//! `olc`: G5 ve G7 ölçümleri (aşama 3 tasarımı §8.5, §13).
//!
//! Kalfa'nın bwrap'ı yerine `unshare --user --map-root-user --net` kullanılır.
//! Ağ ad alanı aynıdır; bwrap'ın açtığı geri döngüyü köprü `--lo-ac` ile
//! kendisi açar. Komutun ortamı bwrap'taki `--clearenv` gibi boşaltılır.
//!
//! Dışarıya hiçbir bağlantı kurulmaz. Vekilin sahtesi izinli hostta yalnızca
//! aracın ClientHello'sunu gözler; yalnızca `yerel.test` ölçümün yerel
//! sunucusuna aktarılır.

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::kanal;
use crate::tls::Okuma;
use crate::vekil::{Karar, Kayit, Kurallar, Vekil};

/// Ölçümlerde kullanılan, hiçbir zaman çözülmeyen ad (RFC 2606).
const IZINSIZ: &str = "izinsiz.example";
/// Belgelere ayrılmış, yönlendirilmeyen adres (RFC 5737); yapılandırma
/// önceliği ölçümünde "başka bir vekil" yerine geçer.
const BASKA_VEKIL: &str = "http://192.0.2.1:3128";

pub fn ana(a: &[String]) -> i32 {
    let mut cikti_dosyasi = None;
    let mut onekler = Vec::new();
    let mut i = 0;
    while i < a.len() {
        if a[i] == "--cikti" {
            i += 1;
            cikti_dosyasi = a.get(i).cloned();
        } else {
            onekler.push(a[i].clone());
        }
        i += 1;
    }
    let ortam = match Ortam::hazirla() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("olc: {e}");
            return 1;
        }
    };
    let rapor = rapor(&ortam, &onekler);
    let _ = std::fs::remove_dir_all(&ortam.gecici);
    print!("{rapor}");
    if let Some(dosya) = cikti_dosyasi
        && let Err(e) = std::fs::write(&dosya, &rapor)
    {
        eprintln!("{dosya}: {e}");
        return 1;
    }
    0
}

struct Ortam {
    exe: PathBuf,
    gecici: PathBuf,
    path: String,
    sunucu: SocketAddr,
    cargo: String,
    rustc: String,
}

impl Ortam {
    fn hazirla() -> Result<Self, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let gecici = std::env::temp_dir().join(format!("kalfa-kopru-olc-{}", std::process::id()));
        let yaz = |yol: &str, icerik: &[u8]| -> Result<(), String> {
            let tam = gecici.join(yol);
            if let Some(ust) = tam.parent() {
                std::fs::create_dir_all(ust).map_err(|e| format!("{}: {e}", ust.display()))?;
            }
            std::fs::write(&tam, icerik).map_err(|e| format!("{}: {e}", tam.display()))
        };
        for dizin in ["ev", "tmp", "npm", "pip", "go", "go-onbellek", "cargo-ev"] {
            std::fs::create_dir_all(gecici.join(dizin)).map_err(|e| e.to_string())?;
        }
        yaz(
            "cargo-proje/Cargo.toml",
            b"[package]\nname = \"ornek\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
              [dependencies]\nitoa = \"1\"\n",
        )?;
        yaz("cargo-proje/src/lib.rs", b"")?;
        yaz("npmrc", b"")?;
        yaz("npmrc-kuresel", b"")?;
        yaz("uv/req.in", b"idna\n")?;
        yaz("go-modul/go.mod", b"module ornek\n\ngo 1.24\n")?;
        yaz(
            "java/Istek.java",
            b"public class Istek {\n  public static void main(String[] a) throws Exception {\n    \
              var c = java.net.http.HttpClient.newHttpClient();\n    \
              var r = c.send(java.net.http.HttpRequest.newBuilder(java.net.URI.create(a[0])).build(),\n        \
              java.net.http.HttpResponse.BodyHandlers.discarding());\n    \
              System.out.println(r.statusCode());\n  }\n}\n",
        )?;
        yaz("yukleme.bin", &vec![b'y'; 1 << 20])?;
        let baska = format!("https-proxy={BASKA_VEKIL}\nproxy={BASKA_VEKIL}\n");
        yaz("npmrc-baska-vekil", baska.as_bytes())?;
        let baska = format!("[http]\nproxy = \"{BASKA_VEKIL}\"\n");
        yaz("cargo-baska/config.toml", baska.as_bytes())?;
        let baska = format!("[http]\n\tproxy = {BASKA_VEKIL}\n");
        yaz("gitconfig-baska", baska.as_bytes())?;
        let baska = format!("[global]\nproxy = {BASKA_VEKIL}\n");
        yaz("pip-baska.conf", baska.as_bytes())?;
        let sunucu = yerel_sunucu().map_err(|e| format!("yerel sunucu: {e}"))?;
        let bul = |ad: &str| {
            komut_ciktisi("rustup", &["which", ad])
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| ad.to_string())
        };
        Ok(Self {
            path: std::env::var("PATH").unwrap_or_default(),
            cargo: bul("cargo"),
            rustc: bul("rustc"),
            exe,
            gecici,
            sunucu,
        })
    }

    fn yol(&self, ad: &str) -> String {
        self.gecici.join(ad).display().to_string()
    }

    fn exe(&self) -> String {
        self.exe.display().to_string()
    }

    fn temel(&self) -> Vec<(String, String)> {
        vec![
            ("PATH".into(), self.path.clone()),
            ("HOME".into(), self.yol("ev")),
            ("LANG".into(), "C.UTF-8".into()),
            ("TERM".into(), "dumb".into()),
            ("TMPDIR".into(), self.yol("tmp")),
        ]
    }

    fn yerel_kurallar(&self) -> Kurallar {
        let mut k = Kurallar::varsayilan();
        k.yerel.push((("yerel.test".into(), 80), self.sunucu));
        k
    }
}

fn komut_ciktisi(program: &str, argumanlar: &[&str]) -> Option<String> {
    let cikti = Command::new(program).args(argumanlar).output().ok()?;
    let mut metin = String::from_utf8_lossy(&cikti.stdout).into_owned();
    if metin.trim().is_empty() {
        metin = String::from_utf8_lossy(&cikti.stderr).into_owned();
    }
    Some(metin)
}

struct Is {
    komut: Vec<String>,
    ortam: Vec<(String, String)>,
    kurallar: Arc<Kurallar>,
    seccomp: bool,
    dizin: Option<String>,
    sinir: Duration,
}

impl Is {
    fn yeni(komut: &[&str]) -> Self {
        Self {
            komut: komut.iter().map(|s| (*s).to_string()).collect(),
            ortam: Vec::new(),
            kurallar: Arc::new(Kurallar::varsayilan()),
            seccomp: false,
            dizin: None,
            sinir: Duration::from_secs(90),
        }
    }

    fn sh(betik: &str) -> Self {
        Self::yeni(&["sh", "-c", betik])
    }

    fn ortam(mut self, ad: &str, deger: &str) -> Self {
        self.ortam.push((ad.into(), deger.into()));
        self
    }

    fn kurallar(mut self, k: Kurallar) -> Self {
        self.kurallar = Arc::new(k);
        self
    }

    fn seccomp(mut self) -> Self {
        self.seccomp = true;
        self
    }

    fn dizin(mut self, d: String) -> Self {
        self.dizin = Some(d);
        self
    }
}

struct Kosu {
    kod: Option<i32>,
    cikti: String,
    hata: String,
    sure: Duration,
    /// Komut bittikten sonra vekilin kanalı kapatıp bitmesi.
    bitis: Duration,
    kayitlar: Vec<Kayit>,
    zaman_asimi: bool,
}

impl Kosu {
    fn sonuc(&self) -> String {
        self.cikti
            .lines()
            .find_map(|s| s.strip_prefix("sonuc: "))
            .map(str::to_string)
            .unwrap_or_else(|| son_satir(&self.hata))
    }

    fn cikis(&self) -> String {
        match (self.zaman_asimi, self.kod) {
            (true, _) => "zaman aşımı".into(),
            (false, Some(k)) => k.to_string(),
            (false, None) => "sinyal".into(),
        }
    }
}

fn kos(o: &Ortam, is: Is) -> Kosu {
    match kos_(o, &is) {
        Ok(k) => k,
        Err(e) => Kosu {
            kod: None,
            cikti: String::new(),
            hata: format!("başlatılamadı: {e}"),
            sure: Duration::ZERO,
            bitis: Duration::ZERO,
            kayitlar: Vec::new(),
            zaman_asimi: false,
        },
    }
}

fn kos_(o: &Ortam, is: &Is) -> io::Result<Kosu> {
    let (vekil_ucu, kopru_ucu) = kanal::cift()?;
    let mut c = Command::new("unshare");
    c.args(["--user", "--map-root-user", "--net", "--"])
        .arg(&o.exe)
        .args(["kopru", "--kanal", "3", "--lo-ac"]);
    if is.seccomp {
        c.arg("--seccomp");
    }
    c.arg("--").args(&is.komut);
    c.env_clear().envs(o.temel()).envs(is.ortam.iter().cloned());
    c.current_dir(is.dizin.clone().unwrap_or_else(|| o.yol("tmp")));
    c.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let ham = kopru_ucu.as_raw_fd();
    // SAFETY: fork ile exec arasında yalnızca async-signal-safe `dup2` ve
    // `fcntl` çağrılır. Kanal ucu 3 numarasına `CLOEXEC`'siz yerleşir.
    unsafe {
        c.pre_exec(move || {
            let r = if ham == 3 {
                libc::fcntl(3, libc::F_SETFD, 0)
            } else {
                libc::dup2(ham, 3)
            };
            if r < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let baslangic = Instant::now();
    let mut cocuk = c.spawn()?;
    drop(kopru_ucu);
    let vekil = Vekil::baslat(vekil_ucu, Arc::clone(&is.kurallar));
    let cikti = oku_arka(cocuk.stdout.take());
    let hata = oku_arka(cocuk.stderr.take());
    let mut zaman_asimi = false;
    let durum = loop {
        if let Some(d) = cocuk.try_wait()? {
            break Some(d);
        }
        if baslangic.elapsed() > is.sinir {
            zaman_asimi = true;
            let _ = cocuk.kill();
            break cocuk.wait().ok();
        }
        thread::sleep(Duration::from_millis(10));
    };
    let sure = baslangic.elapsed();
    let kayitlar = vekil.bitir();
    let bitis = baslangic.elapsed() - sure;
    Ok(Kosu {
        kod: durum.and_then(|d| d.code()),
        cikti: cikti.join().unwrap_or_default(),
        hata: hata.join().unwrap_or_default(),
        sure,
        bitis,
        kayitlar,
        zaman_asimi,
    })
}

fn oku_arka(kaynak: Option<impl Read + Send + 'static>) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut s = Vec::new();
        if let Some(mut k) = kaynak {
            let _ = k.read_to_end(&mut s);
        }
        String::from_utf8_lossy(&s).into_owned()
    })
}

// --- yerel sunucu (dış ad alanında) ---

fn yerel_sunucu() -> io::Result<SocketAddr> {
    let dinleyici = TcpListener::bind("127.0.0.1:0")?;
    let adres = dinleyici.local_addr()?;
    thread::spawn(move || {
        for s in dinleyici.incoming().flatten() {
            thread::spawn(move || {
                let _ = sun(s);
            });
        }
    });
    Ok(adres)
}

fn istek_basi(s: &mut TcpStream) -> io::Result<String> {
    let mut bas = Vec::new();
    let mut bayt = [0u8; 1];
    while !bas.ends_with(b"\r\n\r\n") {
        if s.read(&mut bayt)? == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        bas.push(bayt[0]);
    }
    Ok(String::from_utf8_lossy(&bas).into_owned())
}

fn sun(mut s: TcpStream) -> io::Result<()> {
    s.set_read_timeout(Some(Duration::from_secs(10)))?;
    let bas = istek_basi(&mut s)?;
    let uzunluk: u64 = bas
        .lines()
        .find_map(|l| {
            let (ad, deger) = l.split_once(':')?;
            ad.eq_ignore_ascii_case("content-length")
                .then(|| deger.trim().parse().ok())?
        })
        .unwrap_or(0);
    let alinan = io::copy(&mut (&mut s).take(uzunluk), &mut io::sink())?;
    let ilk = bas.lines().next().unwrap_or_default();
    let govde = if ilk.starts_with("GET /buyuk") {
        vec![b'k'; 8 << 20]
    } else if ilk.starts_with("POST") {
        format!("alindi={alinan}\n").into_bytes()
    } else {
        b"kalfa-kopru-yaniti\n".to_vec()
    };
    let baslik = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        govde.len()
    );
    s.write_all(baslik.as_bytes())?;
    s.write_all(&govde)
}

// --- rapor ---

struct Satir {
    kod: &'static str,
    deneme: String,
    beklenen: String,
    gozlenen: String,
    gecti: Option<bool>,
}

fn satir(
    kod: &'static str,
    deneme: &str,
    beklenen: &str,
    gozlenen: String,
    gecti: Option<bool>,
) -> Satir {
    Satir {
        kod,
        deneme: deneme.into(),
        beklenen: beklenen.into(),
        gozlenen,
        gecti,
    }
}

fn hucre(s: &str) -> String {
    let tek: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let kisa: String = tek.chars().take(160).collect();
    let kisa = if tek.chars().count() > 160 {
        format!("{kisa}…")
    } else {
        kisa
    };
    kisa.replace('|', "\\|")
}

fn son_satir(metin: &str) -> String {
    metin
        .lines()
        .rev()
        .find(|s| !s.trim().is_empty())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn tablo(satirlar: &[Satir]) -> String {
    let mut s =
        String::from("| Kod | Deneme | Beklenen | Gözlenen | Sonuç |\n|---|---|---|---|---|\n");
    for r in satirlar {
        let sonuc = match r.gecti {
            Some(true) => "geçti",
            Some(false) => "**kaldı**",
            None => "ölçüm",
        };
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            r.kod,
            hucre(&r.deneme),
            hucre(&r.beklenen),
            hucre(&r.gozlenen),
            sonuc
        ));
    }
    s
}

fn kayit_ozeti(kayitlar: &[Kayit]) -> String {
    if kayitlar.is_empty() {
        return "vekile istek yok".into();
    }
    kayitlar
        .iter()
        .map(|k| {
            let karar = match &k.karar {
                Karar::TanimlayiciReddi(n) => format!("tanımlayıcı reddedildi: {n}"),
                Karar::Gecersiz(n) => format!("400: {n}"),
                Karar::Yasak => "403".into(),
                Karar::Izinli(Okuma::Tls(ch)) => {
                    format!("200, SNI {}", ch.sni.as_deref().unwrap_or("yok"))
                }
                Karar::Izinli(Okuma::TlsDegil(b)) => format!("200, TLS değil (ilk bayt {b:#04x})"),
                Karar::Izinli(Okuma::Hata(e)) => format!("200, ClientHello okunamadı: {e}"),
                Karar::Aktarildi { giden, gelen } => {
                    format!("aktarıldı (giden {giden} B, gelen {gelen} B)")
                }
            };
            format!("`{}` → {karar}", k.istek)
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn rapor(o: &Ortam, onekler: &[String]) -> String {
    let secili =
        |kod: &str| onekler.is_empty() || onekler.iter().any(|p| kod.starts_with(p.as_str()));
    let mut r = String::from("# G5 ölçümü: Linux ağ köprüsü prototipi\n\n");
    r.push_str(&surumler(o));
    type Grup = fn(&Ortam) -> (String, String);
    let gruplar: [(&str, Grup); 10] = [
        ("o1", o1),
        ("o2", o2),
        ("o3", o3),
        ("o4", o4),
        ("o5", o5),
        ("o6", o6),
        ("o7", o7),
        ("o8", o8),
        ("o9", o9),
        ("o10", o10),
    ];
    for (kod, grup) in gruplar {
        if !secili(kod) {
            continue;
        }
        eprintln!("olc: {kod}");
        let (baslik, govde) = grup(o);
        r.push_str(&format!("\n## {kod}: {baslik}\n\n{govde}"));
    }
    r
}

fn surumler(o: &Ortam) -> String {
    let mut s = String::from("| Bileşen | Sürüm |\n|---|---|\n");
    let cekirdek = komut_ciktisi("uname", &["-sr"]).unwrap_or_default();
    s.push_str(&format!("| Çekirdek | {} |\n", hucre(cekirdek.trim())));
    let araclar: [(&str, &str, &[&str]); 12] = [
        ("unshare", "unshare", &["--version"]),
        ("curl", "curl", &["--version"]),
        ("wget", "wget", &["--version"]),
        ("git", "git", &["--version"]),
        ("cargo", &o.cargo, &["--version"]),
        ("pip", "python3", &["-m", "pip", "--version"]),
        ("uv", "uv", &["--version"]),
        ("npm", "npm", &["--version"]),
        ("node", "node", &["--version"]),
        ("python", "python3", &["--version"]),
        ("go", "go", &["version"]),
        ("java", "java", &["--version"]),
    ];
    for (ad, program, argumanlar) in araclar {
        let surum = komut_ciktisi(program, argumanlar)
            .and_then(|m| {
                m.lines()
                    .find(|l| !l.starts_with("Picked up"))
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "yok".into());
        s.push_str(&format!("| {ad} | {} |\n", hucre(surum.trim())));
    }
    s
}

// --- ölçümler ---

fn o1(o: &Ortam) -> (String, String) {
    let exe = o.exe();
    let dogrudan = kos(o, Is::yeni(&[&exe, "sina", "dogrudan"]));
    let dns = kos(o, Is::yeni(&[&exe, "sina", "dns", "index.crates.io"]));
    let port = o.sunucu.port().to_string();
    let geri = kos(o, Is::yeni(&[&exe, "sina", "geri-donus", &port]));
    let satirlar = [
        satir(
            "o1.1",
            "Doğrudan bağlantı (192.0.2.1:443)",
            "ağa ulaşılamaz",
            dogrudan.sonuc(),
            Some(dogrudan.sonuc().contains("unreachable")),
        ),
        satir(
            "o1.2",
            "Ad çözümü (index.crates.io)",
            "başarısız",
            dns.sonuc(),
            Some(dns.sonuc().starts_with("hata")),
        ),
        satir(
            "o1.3",
            "Dış ad alanının geri döngüsündeki sunucu",
            "bağlantı reddedilir",
            geri.sonuc(),
            Some(geri.sonuc().contains("refused")),
        ),
    ];
    ("Ağ ad alanı".into(), tablo(&satirlar))
}

fn o2(o: &Ortam) -> (String, String) {
    let kurallar = || o.yerel_kurallar();
    let kucuk = kos(
        o,
        Is::yeni(&[
            "curl",
            "-sS",
            "-p",
            "--max-time",
            "15",
            "http://yerel.test/",
        ])
        .kurallar(kurallar()),
    );
    let buyuk = kos(
        o,
        Is::yeni(&[
            "curl",
            "-sS",
            "-p",
            "--max-time",
            "30",
            "-o",
            "/dev/null",
            "-w",
            "%{size_download} %{speed_download}",
            "http://yerel.test/buyuk",
        ])
        .kurallar(kurallar()),
    );
    let dosya = format!("@{}", o.yol("yukleme.bin"));
    let yukleme = kos(
        o,
        Is::yeni(&[
            "curl",
            "-sS",
            "-p",
            "--max-time",
            "30",
            "-H",
            "Expect:",
            "--data-binary",
            &dosya,
            "http://yerel.test/yukle",
        ])
        .kurallar(kurallar()),
    );
    let hiz = |k: &Kosu| {
        let mut p = k.cikti.split_whitespace();
        let bayt: f64 = p.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let hiz: f64 = p.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
        format!(
            "{bayt:.0} B, {:.0} MB/sn; {}",
            hiz / 1e6,
            kayit_ozeti(&k.kayitlar)
        )
    };
    let satirlar = [
        satir(
            "o2.1",
            "curl `-p` ile `yerel.test` (CONNECT, tanımlayıcı kanaldan vekile)",
            "yerel sunucunun yanıtı",
            format!("{}; {}", kucuk.cikti.trim(), kayit_ozeti(&kucuk.kayitlar)),
            Some(kucuk.cikti.contains("kalfa-kopru-yaniti")),
        ),
        satir(
            "o2.2",
            "8 MiB indirme",
            "8388608 B",
            hiz(&buyuk),
            Some(buyuk.cikti.starts_with("8388608")),
        ),
        satir(
            "o2.3",
            "1 MiB yükleme",
            "alindi=1048576",
            format!(
                "{}; {}",
                yukleme.cikti.trim(),
                kayit_ozeti(&yukleme.kayitlar)
            ),
            Some(yukleme.cikti.contains("alindi=1048576")),
        ),
    ];
    ("Tanımlayıcı taşıma ve aktarım".into(), tablo(&satirlar))
}

fn o3(o: &Ortam) -> (String, String) {
    let exe = o.exe();
    let fd = kos(o, Is::yeni(&[&exe, "sina", "fd"]));
    let satirlar = [satir(
        "o3.1",
        "Komutun açık tanımlayıcıları",
        "kanal ve dinleyici yok (yalnızca 0, 1, 2 ve listelemenin kendisi)",
        fd.sonuc(),
        Some(fd.sonuc().starts_with("0=") && !fd.sonuc().contains("socket:")),
    )];
    ("Kanal komuta geçmez".into(), tablo(&satirlar))
}

fn o4(o: &Ortam) -> (String, String) {
    let yedi = kos(o, Is::sh("exit 7"));
    let sinyal = kos(o, Is::sh("kill -TERM $$"));
    let dosya = o.yol("tmp/arka.txt");
    let betik =
        format!("(sleep 1; curl -sS -p --max-time 5 http://yerel.test/ > {dosya} 2>&1) & exit 0");
    let arka = kos(o, Is::sh(&betik).kurallar(o.yerel_kurallar()));
    thread::sleep(Duration::from_millis(2500));
    let arka_sonuc = std::fs::read_to_string(&dosya).unwrap_or_else(|e| format!("okunamadı: {e}"));
    let satirlar = [
        satir(
            "o4.1",
            "Komut 7 ile çıkar",
            "köprü 7 ile çıkar",
            yedi.cikis(),
            Some(yedi.kod == Some(7)),
        ),
        satir(
            "o4.2",
            "Komut SIGTERM ile ölür",
            "köprü 143 ile çıkar",
            sinyal.cikis(),
            Some(sinyal.kod == Some(143)),
        ),
        satir(
            "o4.3",
            "Köprü çıkınca vekilin kanalı",
            "kanal hemen kapanır, vekil biter",
            format!("{} ms", yedi.bitis.as_millis()),
            Some(yedi.bitis < Duration::from_millis(500)),
        ),
        satir(
            "o4.4",
            "Komutun arka plandaki çocuğu köprüden sonra vekile bağlanır",
            "bağlanamaz (dinleyici köprüyle kapandı)",
            format!(
                "{}; {}",
                son_satir(&arka_sonuc),
                kayit_ozeti(&arka.kayitlar)
            ),
            Some(arka.kayitlar.is_empty() && !arka_sonuc.contains("kalfa-kopru-yaniti")),
        ),
    ];
    ("Çıkış kodu ve yaşam süresi".into(), tablo(&satirlar))
}

fn o5(o: &Ortam) -> (String, String) {
    let exe = o.exe();
    let curl = kos(
        o,
        Is::yeni(&[
            "curl",
            "-sS",
            "-p",
            "--max-time",
            "15",
            "http://yerel.test/",
        ])
        .kurallar(o.yerel_kurallar())
        .seccomp(),
    );
    let filtreli = kos(o, Is::yeni(&[&exe, "sina", "unix"]).seccomp());
    let filtresiz = kos(o, Is::yeni(&[&exe, "sina", "unix"]));
    let satirlar = [
        satir(
            "o5.1",
            "Filtre altında curl `-p` ile `yerel.test`",
            "yerel sunucunun yanıtı",
            format!("{}; {}", curl.cikti.trim(), kayit_ozeti(&curl.kayitlar)),
            Some(curl.cikti.contains("kalfa-kopru-yaniti")),
        ),
        satir(
            "o5.2",
            "Filtre altında `socket(AF_UNIX)`",
            "EPERM",
            filtreli.sonuc(),
            Some(filtreli.sonuc().contains("not permitted")),
        ),
        satir(
            "o5.3",
            "Filtresiz `socket(AF_UNIX)` (karşılaştırma)",
            "açılır",
            filtresiz.sonuc(),
            Some(filtresiz.sonuc() == "AÇILDI"),
        ),
    ];
    (
        "Ağlı profilin soket kuralı altında köprü".into(),
        tablo(&satirlar),
    )
}

fn o6(o: &Ortam) -> (String, String) {
    let sunucu = "python3 -m http.server --bind 127.0.0.1 18080 >/dev/null 2>&1 & p=$!; sleep 1.5;";
    let curl = kos(
        o,
        Is::sh(&format!(
            "{sunucu} curl -sS --max-time 5 -o /dev/null -w '%{{http_code}} ' http://127.0.0.1:18080/; \
             curl -sS --max-time 5 -o /dev/null -w '%{{http_code}}' http://localhost:18080/; kill $p"
        )),
    );
    let wget = format!(
        "{sunucu} wget -q -O /dev/null --timeout=5 --tries=1 http://127.0.0.1:18080/ && echo tamam || echo hata; kill $p"
    );
    let wget_buyuk = kos(o, Is::sh(&wget));
    let wget_kucuk = kos(
        o,
        Is::sh(&wget).ortam("no_proxy", "localhost,127.0.0.1,::1"),
    );
    let satirlar = [
        satir(
            "o6.1",
            "Kabın içindeki yerel sunucu, curl (`127.0.0.1` ve `localhost`)",
            "vekilsiz 200 200",
            format!("{}; {}", curl.cikti.trim(), kayit_ozeti(&curl.kayitlar)),
            Some(curl.cikti.trim() == "200 200" && curl.kayitlar.is_empty()),
        ),
        satir(
            "o6.2",
            "Aynısı wget, yalnızca `NO_PROXY` ile",
            "ölçüm",
            format!(
                "{}; {}",
                wget_buyuk.cikti.trim(),
                kayit_ozeti(&wget_buyuk.kayitlar)
            ),
            None,
        ),
        satir(
            "o6.3",
            "Aynısı wget, `no_proxy` da verilince",
            "vekilsiz",
            format!(
                "{}; {}",
                wget_kucuk.cikti.trim(),
                kayit_ozeti(&wget_kucuk.kayitlar)
            ),
            Some(wget_kucuk.cikti.contains("tamam") && wget_kucuk.kayitlar.is_empty()),
        ),
    ];
    (
        "NO_PROXY ve kabın içindeki sunucular".into(),
        tablo(&satirlar),
    )
}

struct AracSatiri {
    kod: &'static str,
    arac: &'static str,
    kosu: Kosu,
    /// Son satır yerine `403` geçen son satır gösterilir.
    dort_yuz_uc: bool,
}

const FETCH: &str = "fetch('https://registry.npmjs.org/left-pad').then(r=>console.log(r.status),\
                     e=>{console.error(String(e.cause||e));process.exit(1)})";
const HTTPS_GET: &str = "require('https').get('https://registry.npmjs.org/left-pad',\
                         r=>console.log(r.statusCode)).on('error',e=>{console.error(String(e));process.exit(1)})";
const URLLIB: &str =
    "import urllib.request as u; u.urlopen('https://pypi.org/simple/', timeout=15)";

fn npm(o: &Ortam, is: Is) -> Is {
    is.ortam("npm_config_userconfig", &o.yol("npmrc"))
        .ortam("npm_config_globalconfig", &o.yol("npmrc-kuresel"))
        .ortam("npm_config_cache", &o.yol("npm"))
        .ortam("npm_config_fetch_retries", "0")
        .ortam("npm_config_update_notifier", "false")
}

fn cargo(o: &Ortam, is: Is, cargo_home: &str) -> Is {
    is.ortam("CARGO_HOME", cargo_home)
        .ortam("CARGO_NET_RETRY", "0")
        .ortam("CARGO_HTTP_TIMEOUT", "15")
        .ortam("RUSTC", &o.rustc)
        .dizin(o.yol("cargo-proje"))
}

fn pip(o: &Ortam, is: Is, yapilandirma: &str) -> Is {
    is.ortam("PIP_CONFIG_FILE", yapilandirma)
        .ortam("PIP_DISABLE_PIP_VERSION_CHECK", "1")
        .ortam("PIP_NO_INPUT", "1")
        .dizin(o.yol("pip"))
}

const PIP: &[&str] = &[
    "python3",
    "-m",
    "pip",
    "download",
    "--no-deps",
    "--no-cache-dir",
    "--retries",
    "0",
    "--timeout",
    "15",
    "-d",
    ".",
    "idna",
];

fn git(is: Is) -> Is {
    is.ortam("GIT_TERMINAL_PROMPT", "0")
        .ortam("GIT_CONFIG_NOSYSTEM", "1")
}

fn o7(o: &Ortam) -> (String, String) {
    let cargo_home = o.yol("cargo-ev");
    let java = o.yol("java/Istek.java");
    let maven = "https://repo.maven.apache.org/maven2/";
    let isler: Vec<(&'static str, &'static str, Is)> = vec![
        (
            "a1",
            "curl (izinli host)",
            Is::yeni(&[
                "curl",
                "-sS",
                "-o",
                "/dev/null",
                "--max-time",
                "15",
                "https://index.crates.io/config.json",
            ]),
        ),
        (
            "a2",
            "curl (listede olmayan host)",
            Is::yeni(&[
                "curl",
                "-sS",
                "-o",
                "/dev/null",
                "--max-time",
                "15",
                &format!("https://{IZINSIZ}/"),
            ]),
        ),
        (
            "a3",
            "curl (düz HTTP)",
            Is::yeni(&[
                "curl",
                "-sS",
                "-o",
                "/dev/null",
                "--max-time",
                "15",
                &format!("http://{IZINSIZ}/"),
            ]),
        ),
        (
            "a4",
            "wget",
            Is::yeni(&[
                "wget",
                "-q",
                "-O",
                "/dev/null",
                "--timeout=15",
                "--tries=1",
                "https://pypi.org/simple/",
            ]),
        ),
        (
            "a5",
            "git ls-remote (github.com listede yok)",
            git(Is::yeni(&[
                "git",
                "ls-remote",
                "https://github.com/rust-lang/cargo",
            ])),
        ),
        (
            "a6",
            "git ls-remote (github.com eklendi)",
            git(Is::yeni(&[
                "git",
                "ls-remote",
                "https://github.com/rust-lang/cargo",
            ]))
            .kurallar(Kurallar::varsayilan().ekle("github.com", 443)),
        ),
        (
            "a7",
            "cargo fetch",
            cargo(o, Is::yeni(&[&o.cargo, "fetch"]), &cargo_home),
        ),
        ("a8", "pip download", pip(o, Is::yeni(PIP), "/dev/null")),
        (
            "a9",
            "uv pip compile",
            Is::yeni(&[
                "uv",
                "pip",
                "compile",
                "--no-config",
                "--index-url",
                "https://pypi.org/simple",
                &o.yol("uv/req.in"),
            ])
            .ortam("UV_CACHE_DIR", &o.yol("uv/onbellek"))
            .ortam("UV_PYTHON_DOWNLOADS", "never")
            .ortam("UV_HTTP_TIMEOUT", "15")
            .ortam("UV_HTTP_RETRIES", "0"),
        ),
        (
            "a10",
            "npm view",
            npm(o, Is::yeni(&["npm", "view", "left-pad", "version"])),
        ),
        ("a11", "node fetch", Is::yeni(&["node", "-e", FETCH])),
        (
            "a12",
            "node fetch, NODE_USE_ENV_PROXY=1",
            Is::yeni(&["node", "-e", FETCH]).ortam("NODE_USE_ENV_PROXY", "1"),
        ),
        (
            "a13",
            "node https.get",
            Is::yeni(&["node", "-e", HTTPS_GET]),
        ),
        (
            "a14",
            "node https.get, NODE_USE_ENV_PROXY=1",
            Is::yeni(&["node", "-e", HTTPS_GET]).ortam("NODE_USE_ENV_PROXY", "1"),
        ),
        ("a15", "python urllib", Is::yeni(&["python3", "-c", URLLIB])),
        (
            "a16",
            "go mod download (proxy.golang.org eklendi)",
            Is::yeni(&[
                "go",
                "mod",
                "download",
                "-json",
                "golang.org/x/text@v0.14.0",
            ])
            .ortam("GOPATH", &o.yol("go"))
            .ortam("GOMODCACHE", &o.yol("go/mod"))
            .ortam("GOCACHE", &o.yol("go-onbellek"))
            .ortam("GOTOOLCHAIN", "local")
            .ortam("GOENV", "off")
            .ortam("GOFLAGS", "-mod=mod")
            .dizin(o.yol("go-modul"))
            .kurallar(Kurallar::varsayilan().ekle("proxy.golang.org", 443)),
        ),
        ("a17", "java HttpClient", Is::yeni(&["java", &java, maven])),
        (
            "a18",
            "java HttpClient, JAVA_TOOL_OPTIONS ile vekil",
            Is::sh(&format!(
                "JAVA_TOOL_OPTIONS=\"-Dhttps.proxyHost=127.0.0.1 -Dhttps.proxyPort=${{HTTPS_PROXY##*:}}\" java {java} {maven}"
            )),
        ),
    ];
    let satirlar: Vec<AracSatiri> = isler
        .into_iter()
        .map(|(kod, arac, is)| {
            eprintln!("olc: {kod} {arac}");
            AracSatiri {
                kod,
                arac,
                kosu: kos(o, is),
                dort_yuz_uc: false,
            }
        })
        .collect();
    (
        "Araçların vekile uyumu (G5) ve ClientHello (G7)".into(),
        arac_tablosu(&satirlar),
    )
}

fn arac_tablosu(satirlar: &[AracSatiri]) -> String {
    let mut s = String::from(
        "| Kod | Araç | Vekile gelen | SNI | ECH | ALPN | ClientHello | Çıkış | Süre | Son satır |\n\
         |---|---|---|---|---|---|---|---|---|---|\n",
    );
    for r in satirlar {
        let istekler = if r.kosu.kayitlar.is_empty() {
            "istek yok".to_string()
        } else {
            r.kosu
                .kayitlar
                .iter()
                .map(|k| {
                    let karar = match &k.karar {
                        Karar::Yasak => "403",
                        Karar::Izinli(_) => "200",
                        Karar::Aktarildi { .. } => "aktarıldı",
                        Karar::Gecersiz(_) => "400",
                        Karar::TanimlayiciReddi(_) => "ret",
                    };
                    format!("`{}` → {karar}", k.istek)
                })
                .collect::<Vec<_>>()
                .join("; ")
        };
        let ch = r
            .kosu
            .kayitlar
            .iter()
            .find_map(|k| match (&k.karar, &k.host) {
                (Karar::Izinli(Okuma::Tls(ch)), Some((host, _))) => {
                    Some((ch.clone(), host.clone()))
                }
                _ => None,
            });
        let boyut = ch
            .as_ref()
            .map(|(ch, _)| format!("{} B", ch.bayt))
            .unwrap_or_else(|| "—".into());
        let (sni, ech, alpn) = match &ch {
            Some((ch, host)) => (
                match &ch.sni {
                    Some(sni) if sni == host => format!("{sni} (= host)"),
                    Some(sni) => format!("{sni} (≠ {host})"),
                    None => "yok".into(),
                },
                if ch.ech { "var" } else { "yok" }.to_string(),
                if ch.alpn.is_empty() {
                    "—".into()
                } else {
                    ch.alpn.join(", ")
                },
            ),
            None => ("—".into(), "—".into(), "—".into()),
        };
        let son = {
            let h = son_satir(&r.kosu.hata);
            if h.is_empty() {
                son_satir(&r.kosu.cikti)
            } else {
                h
            }
        };
        let son = if r.dort_yuz_uc {
            format!("{}{}", r.kosu.hata, r.kosu.cikti)
                .lines()
                .rev()
                .find(|l| l.contains("403"))
                .map(|l| l.trim().to_string())
                .unwrap_or(son)
        } else {
            son
        };
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {:.1} sn | {} |\n",
            r.kod,
            hucre(r.arac),
            hucre(&istekler),
            hucre(&sni),
            ech,
            hucre(&alpn),
            boyut,
            r.kosu.cikis(),
            r.kosu.sure.as_secs_f64(),
            hucre(&son)
        ));
    }
    s
}

fn o8(o: &Ortam) -> (String, String) {
    let cargo_baska = o.yol("cargo-baska");
    let isler: Vec<(&'static str, &'static str, Is)> =
        vec![
        (
            "c1",
            "npm, küresel npmrc'de başka vekil",
            npm(o, Is::yeni(&["npm", "view", "left-pad", "version"]))
                .ortam("npm_config_globalconfig", &o.yol("npmrc-baska-vekil")),
        ),
        (
            "c2",
            "aynısı + `npm_config_https_proxy`",
            npm(o, Is::sh("npm_config_https_proxy=$HTTPS_PROXY npm view left-pad version"))
                .ortam("npm_config_globalconfig", &o.yol("npmrc-baska-vekil")),
        ),
        (
            "c3",
            "cargo, yapılandırmada başka vekil",
            cargo(o, Is::yeni(&[&o.cargo, "fetch"]), &cargo_baska),
        ),
        (
            "c4",
            "aynısı + `CARGO_HTTP_PROXY`",
            cargo(
                o,
                Is::sh(&format!("CARGO_HTTP_PROXY=$HTTPS_PROXY {} fetch", o.cargo)),
                &cargo_baska,
            ),
        ),
        (
            "c5",
            "git, gitconfig'de başka vekil",
            git(Is::yeni(&["git", "ls-remote", "https://github.com/rust-lang/cargo"]))
                .ortam("GIT_CONFIG_GLOBAL", &o.yol("gitconfig-baska")),
        ),
        (
            "c6",
            "aynısı + `GIT_CONFIG_*` ile `http.proxy`",
            git(Is::sh(
                "GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=http.proxy GIT_CONFIG_VALUE_0=$HTTPS_PROXY \
                 git ls-remote https://github.com/rust-lang/cargo",
            ))
            .ortam("GIT_CONFIG_GLOBAL", &o.yol("gitconfig-baska")),
        ),
        (
            "c7",
            "pip, pip.conf'ta başka vekil",
            pip(o, Is::yeni(PIP), &o.yol("pip-baska.conf")),
        ),
        (
            "c8",
            "aynısı + `PIP_PROXY`",
            pip(
                o,
                Is::sh(&format!("PIP_PROXY=$HTTPS_PROXY {}", PIP.join(" "))),
                &o.yol("pip-baska.conf"),
            ),
        ),
    ];
    let satirlar: Vec<AracSatiri> = isler
        .into_iter()
        .map(|(kod, arac, is)| {
            eprintln!("olc: {kod} {arac}");
            AracSatiri {
                kod,
                arac,
                kosu: kos(o, is),
                dort_yuz_uc: false,
            }
        })
        .collect();
    (
        "Yapılandırma dosyasındaki vekil ortamı ezer mi".into(),
        arac_tablosu(&satirlar),
    )
}

fn o9(o: &Ortam) -> (String, String) {
    let exe = o.exe();
    let sirali = kos(
        o,
        Is::yeni(&[&exe, "sina", "yuk", "200", "1"]).kurallar(o.yerel_kurallar()),
    );
    let paralel = kos(
        o,
        Is::yeni(&[&exe, "sina", "yuk", "256", "32"]).kurallar(o.yerel_kurallar()),
    );
    let dogrudan = dogrudan_yuk(o.sunucu, 200);
    let aktarilan = |k: &Kosu| {
        k.kayitlar
            .iter()
            .filter(|k| matches!(k.karar, Karar::Aktarildi { .. }))
            .count()
    };
    let satirlar = [
        satir(
            "o9.1",
            "200 sıralı bağlantı, köprü ve vekil üzerinden",
            "ölçüm",
            format!("{}; vekilde {} aktarım", sirali.sonuc(), aktarilan(&sirali)),
            None,
        ),
        satir(
            "o9.2",
            "256 bağlantı, 32 paralel",
            "ölçüm",
            format!(
                "{}; vekilde {} aktarım",
                paralel.sonuc(),
                aktarilan(&paralel)
            ),
            None,
        ),
        satir(
            "o9.3",
            "200 sıralı bağlantı, doğrudan (karşılaştırma)",
            "ölçüm",
            dogrudan,
            None,
        ),
    ];
    ("Bağlantı kurma maliyeti".into(), tablo(&satirlar))
}

fn dogrudan_yuk(sunucu: SocketAddr, n: usize) -> String {
    let mut sureler = Vec::with_capacity(n);
    for _ in 0..n {
        let t = Instant::now();
        let tek = || -> io::Result<()> {
            let mut s = TcpStream::connect(sunucu)?;
            s.write_all(b"GET / HTTP/1.1\r\nHost: yerel\r\nConnection: close\r\n\r\n")?;
            let mut yanit = Vec::new();
            s.read_to_end(&mut yanit)?;
            Ok(())
        };
        if let Err(e) = tek() {
            return format!("hata: {e}");
        }
        sureler.push(t.elapsed());
    }
    sureler.sort();
    let ortalama = sureler.iter().sum::<Duration>() / n.max(1) as u32;
    let medyan = sureler.get(n / 2).copied().unwrap_or_default();
    format!(
        "{n} istek, ortalama {:.2} ms, medyan {:.2} ms",
        ortalama.as_secs_f64() * 1000.0,
        medyan.as_secs_f64() * 1000.0
    )
}

/// Liste boşken araçlar `403`'ü nasıl gösteriyor (§8.4'teki `[ağ]` satırı
/// için).
fn o10(o: &Ortam) -> (String, String) {
    let cargo_home = o.yol("cargo-ev");
    let bos = || Kurallar::default();
    let isler: Vec<(&'static str, &'static str, Is)> = vec![
        (
            "e1",
            "cargo fetch",
            cargo(o, Is::yeni(&[&o.cargo, "fetch"]), &cargo_home).kurallar(bos()),
        ),
        (
            "e2",
            "npm view",
            npm(o, Is::yeni(&["npm", "view", "left-pad", "version"])).kurallar(bos()),
        ),
        (
            "e3",
            "pip download",
            pip(o, Is::yeni(PIP), "/dev/null").kurallar(bos()),
        ),
        (
            "e4",
            "uv pip compile",
            Is::yeni(&[
                "uv",
                "pip",
                "compile",
                "--no-config",
                "--index-url",
                "https://pypi.org/simple",
                &o.yol("uv/req.in"),
            ])
            .ortam("UV_CACHE_DIR", &o.yol("uv/onbellek"))
            .ortam("UV_PYTHON_DOWNLOADS", "never")
            .ortam("UV_HTTP_TIMEOUT", "15")
            .ortam("UV_HTTP_RETRIES", "0")
            .kurallar(bos()),
        ),
        (
            "e5",
            "python urllib",
            Is::yeni(&["python3", "-c", URLLIB]).kurallar(bos()),
        ),
        (
            "e6",
            "node fetch, NODE_USE_ENV_PROXY=1",
            Is::yeni(&["node", "-e", FETCH])
                .ortam("NODE_USE_ENV_PROXY", "1")
                .kurallar(bos()),
        ),
        (
            "e7",
            "go mod download",
            Is::yeni(&[
                "go",
                "mod",
                "download",
                "-json",
                "golang.org/x/text@v0.14.0",
            ])
            .ortam("GOPATH", &o.yol("go"))
            .ortam("GOMODCACHE", &o.yol("go/mod"))
            .ortam("GOCACHE", &o.yol("go-onbellek"))
            .ortam("GOTOOLCHAIN", "local")
            .ortam("GOENV", "off")
            .ortam("GOFLAGS", "-mod=mod")
            .dizin(o.yol("go-modul"))
            .kurallar(bos()),
        ),
    ];
    let satirlar: Vec<AracSatiri> = isler
        .into_iter()
        .map(|(kod, arac, is)| {
            eprintln!("olc: {kod} {arac}");
            let kosu = kos(o, is);
            AracSatiri {
                kod,
                arac,
                kosu,
                dort_yuz_uc: true,
            }
        })
        .collect();
    let mut govde = arac_tablosu(&satirlar);
    govde.push_str("\n| Kod | Çıktıda `403` geçiyor mu |\n|---|---|\n");
    for r in &satirlar {
        let var = r.kosu.hata.contains("403") || r.kosu.cikti.contains("403");
        govde.push_str(&format!(
            "| {} | {} |\n",
            r.kod,
            if var { "evet" } else { "hayır" }
        ));
    }
    ("Engellenen hostta araçların iletisi".into(), govde)
}
