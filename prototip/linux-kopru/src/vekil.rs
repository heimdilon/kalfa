//! Vekilin ölçüm için sahtesi (Kalfa tarafı; aşama 3 tasarımı §8.2). Kanaldan
//! gelen tanımlayıcıyı denetler, isteği okur ve izin listesini uygular.
//!
//! Hiçbir bağlantı dışarı kurulmaz:
//! - İzinli hostta `200` döner, aracın ClientHello'sunu kaydeder ve kapatır.
//! - Yalnızca `yerel` eşlemesindeki hostlar ölçümün yerel sunucusuna aktarılır.
//! - Gerisi `403` alır.

use std::io::{self, Read, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpStream};
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::kanal;
use crate::tls::{self, Okuma};

const BAS_SINIRI: usize = 8 * 1024;

#[derive(Default)]
pub struct Kurallar {
    pub izinli: Vec<(String, u16)>,
    /// Ölçümün yerel sunucusuna aktarılan hostlar.
    pub yerel: Vec<((String, u16), SocketAddr)>,
}

impl Kurallar {
    pub fn varsayilan() -> Self {
        Self {
            izinli: VARSAYILAN_HOSTLAR
                .iter()
                .map(|h| ((*h).to_string(), 443))
                .collect(),
            yerel: Vec::new(),
        }
    }

    pub fn ekle(mut self, host: &str, port: u16) -> Self {
        self.izinli.push((host.to_string(), port));
        self
    }

    fn izinli_mi(&self, host: &str, port: u16) -> bool {
        self.izinli.iter().any(|(h, p)| h == host && *p == port)
    }

    fn yerel_adres(&self, host: &str, port: u16) -> Option<SocketAddr> {
        self.yerel
            .iter()
            .find(|((h, p), _)| h == host && *p == port)
            .map(|(_, a)| *a)
    }
}

/// Tasarımın §8.3'teki varsayılan liste.
pub const VARSAYILAN_HOSTLAR: &[&str] = &[
    "index.crates.io",
    "static.crates.io",
    "registry.npmjs.org",
    "pypi.org",
    "files.pythonhosted.org",
    "codeload.github.com",
    "objects.githubusercontent.com",
    "raw.githubusercontent.com",
];

#[derive(Debug, Clone)]
pub enum Karar {
    /// Kanaldan gelen ileti ya da tanımlayıcı reddedildi.
    TanimlayiciReddi(String),
    /// İstek okunamadı ya da anlaşılmadı (`400`).
    Gecersiz(String),
    /// Listede yok (`403`).
    Yasak,
    /// İzinli; tünel açıldı, aracın ilk kaydı gözlendi, bağlantı kapatıldı.
    Izinli(Okuma),
    /// Yerel sunucuya aktarıldı.
    Aktarildi { giden: u64, gelen: u64 },
}

#[derive(Debug, Clone)]
pub struct Kayit {
    pub istek: String,
    pub host: Option<(String, u16)>,
    pub karar: Karar,
}

pub struct Vekil {
    kayitlar: Arc<Mutex<Vec<Kayit>>>,
    alici: JoinHandle<Vec<JoinHandle<()>>>,
}

impl Vekil {
    pub fn baslat(kanal_ucu: OwnedFd, kurallar: Arc<Kurallar>) -> Self {
        let kayitlar = Arc::new(Mutex::new(Vec::new()));
        let k = Arc::clone(&kayitlar);
        let alici = std::thread::spawn(move || alici(kanal_ucu, kurallar, k));
        Self { kayitlar, alici }
    }

    /// Kanal kapanana ve açık bağlantılar bitene kadar bekler.
    pub fn bitir(self) -> Vec<Kayit> {
        if let Ok(isler) = self.alici.join() {
            for is in isler {
                let _ = is.join();
            }
        }
        self.kayitlar.lock().map(|k| k.clone()).unwrap_or_default()
    }
}

fn kaydet(kayitlar: &Mutex<Vec<Kayit>>, kayit: Kayit) {
    if let Ok(mut k) = kayitlar.lock() {
        k.push(kayit);
    }
}

fn alici(
    kanal_ucu: OwnedFd,
    kurallar: Arc<Kurallar>,
    kayitlar: Arc<Mutex<Vec<Kayit>>>,
) -> Vec<JoinHandle<()>> {
    let mut isler = Vec::new();
    loop {
        let ileti = match kanal::al(kanal_ucu.as_raw_fd()) {
            Ok(Some(ileti)) => ileti,
            Ok(None) => break,
            Err(e) => {
                kaydet(&kayitlar, reddi(format!("kanal: {e}")));
                break;
            }
        };
        if ileti.kesildi || ileti.fds.len() != 1 {
            let neden = format!(
                "{} tanımlayıcı{}",
                ileti.fds.len(),
                if ileti.kesildi { ", kesik ileti" } else { "" }
            );
            kaydet(&kayitlar, reddi(neden));
            continue;
        }
        let Some(fd) = ileti.fds.into_iter().next() else {
            continue;
        };
        if let Err(neden) = kanal::dogrula(&fd) {
            kaydet(&kayitlar, reddi(neden));
            continue;
        }
        let kurallar = Arc::clone(&kurallar);
        let kayitlar = Arc::clone(&kayitlar);
        isler.push(std::thread::spawn(move || {
            let kayit = baglanti(TcpStream::from(fd), &kurallar);
            kaydet(&kayitlar, kayit);
        }));
    }
    isler
}

fn reddi(neden: String) -> Kayit {
    Kayit {
        istek: String::new(),
        host: None,
        karar: Karar::TanimlayiciReddi(neden),
    }
}

fn baglanti(mut akis: TcpStream, kurallar: &Kurallar) -> Kayit {
    let _ = akis.set_read_timeout(Some(Duration::from_secs(10)));
    let _ = akis.set_write_timeout(Some(Duration::from_secs(10)));
    let bas = match istek_basi(&mut akis) {
        Ok(b) => b,
        Err(e) => {
            return Kayit {
                istek: String::new(),
                host: None,
                karar: Karar::Gecersiz(e),
            };
        }
    };
    let ilk = bas.lines().next().unwrap_or_default().to_string();
    let mut parca = ilk.split_whitespace();
    let (yontem, hedef) = (parca.next().unwrap_or(""), parca.next().unwrap_or(""));
    if yontem != "CONNECT" {
        // Mutlak biçimli HTTP isteği: varsayılan listede 80 portu yok (§8.2).
        let host = mutlak_host(hedef);
        let karar = if host.is_some() {
            let _ = yasak_yaz(&mut akis, hedef);
            Karar::Yasak
        } else {
            let _ = akis.write_all(b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\n");
            Karar::Gecersiz("CONNECT ya da mutlak biçim değil".into())
        };
        return Kayit {
            istek: ilk,
            host,
            karar,
        };
    }
    let Some((host, port)) = host_port(hedef) else {
        let _ = akis.write_all(b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\n");
        return Kayit {
            istek: ilk,
            host: None,
            karar: Karar::Gecersiz("hedef ayrıştırılamadı".into()),
        };
    };
    let karar = if let Some(adres) = kurallar.yerel_adres(&host, port) {
        aktar(akis, adres)
    } else if kurallar.izinli_mi(&host, port) && host.parse::<IpAddr>().is_err() {
        match akis.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n") {
            Ok(()) => Karar::Izinli(tls::oku(&mut akis)),
            Err(e) => Karar::Gecersiz(format!("yanıt yazılamadı: {e}")),
        }
    } else {
        let _ = yasak_yaz(&mut akis, &format!("{host}:{port}"));
        Karar::Yasak
    };
    Kayit {
        istek: ilk,
        host: Some((host, port)),
        karar,
    }
}

fn istek_basi(akis: &mut TcpStream) -> Result<String, String> {
    let mut bas = Vec::new();
    let mut bayt = [0u8; 1];
    while !bas.ends_with(b"\r\n\r\n") {
        if bas.len() >= BAS_SINIRI {
            return Err("istek başı çok uzun".into());
        }
        match akis.read(&mut bayt) {
            Ok(0) => return Err("istek bitmeden kapandı".into()),
            Ok(_) => bas.push(bayt[0]),
            Err(e) => return Err(format!("okuma: {e}")),
        }
    }
    Ok(String::from_utf8_lossy(&bas).into_owned())
}

fn yasak_yaz(akis: &mut TcpStream, hedef: &str) -> io::Result<()> {
    let govde = format!("Kalfa: {hedef} izin listesinde yok.\n");
    let yanit = format!(
        "HTTP/1.1 403 Forbidden\r\nContent-Type: text/plain; charset=utf-8\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{govde}",
        govde.len()
    );
    akis.write_all(yanit.as_bytes())
}

/// `host:port` ya da `[v6]:port`.
fn host_port(hedef: &str) -> Option<(String, u16)> {
    let (host, port) = if let Some(kalan) = hedef.strip_prefix('[') {
        let (host, port) = kalan.split_once("]:")?;
        (host, port)
    } else {
        hedef.rsplit_once(':')?
    };
    if host.is_empty() {
        return None;
    }
    Some((host.to_ascii_lowercase(), port.parse().ok()?))
}

fn mutlak_host(hedef: &str) -> Option<(String, u16)> {
    let kalan = hedef.strip_prefix("http://")?;
    let yetki = kalan.split('/').next()?;
    host_port(yetki).or_else(|| Some((yetki.to_ascii_lowercase(), 80)))
}

fn aktar(mut istemci: TcpStream, adres: SocketAddr) -> Karar {
    let sunucu = match TcpStream::connect_timeout(&adres, Duration::from_secs(5)) {
        Ok(s) => s,
        Err(e) => return Karar::Gecersiz(format!("yerel sunucu: {e}")),
    };
    if let Err(e) = istemci.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n") {
        return Karar::Gecersiz(format!("yanıt yazılamadı: {e}"));
    }
    let _ = istemci.set_read_timeout(Some(Duration::from_secs(30)));
    let (Ok(mut istemci_2), Ok(mut sunucu_2)) = (istemci.try_clone(), sunucu.try_clone()) else {
        return Karar::Gecersiz("tanımlayıcı çoğaltılamadı".into());
    };
    let mut sunucu = sunucu;
    let giden = std::thread::spawn(move || {
        let n = io::copy(&mut istemci_2, &mut sunucu_2).unwrap_or(0);
        let _ = sunucu_2.shutdown(Shutdown::Write);
        n
    });
    let gelen = io::copy(&mut sunucu, &mut istemci).unwrap_or(0);
    let _ = istemci.shutdown(Shutdown::Write);
    Karar::Aktarildi {
        giden: giden.join().unwrap_or(0),
        gelen,
    }
}
