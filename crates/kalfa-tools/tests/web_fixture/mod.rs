//! `web_getir` entegrasyon testleri için gerçek yerel HTTP ve `rustls` HTTPS
//! sunucuları ile sahte/rebinding DNS çözücüsü.

pub mod sertifikalar;

use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kalfa_tools::web::Cozucu;
use rustls::ServerConnection;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

use self::sertifikalar::{
    SAHTE_SUNUCU_ANAHTAR_DER, SAHTE_SUNUCU_SERTIFIKA_DER, TEST_KOK_CA_DER, TEST_SUNUCU_ANAHTAR_DER,
    TEST_SUNUCU_SERTIFIKA_DER,
};

/// Test sunucusunun bir HTTP yolu için üreteceği yanıt davranışı.
#[derive(Debug, Clone)]
pub enum SunucuYaniti {
    /// Tam ve geçerli HTTP yanıtı.
    Tam {
        durum: u16,
        basliklar: Vec<(String, String)>,
        govde: Vec<u8>,
    },
    /// Yönlendirme yanıtı (`Location` başlığıyla).
    Yonlendir { durum: u16, hedef: String },
    /// `Content-Length` değerinden daha az bayt gönderip bağlantıyı kapatan yanıt.
    EksikGovde {
        bildirilen_uzunluk: usize,
        gonderilen_govde: Vec<u8>,
    },
    /// Sürekli yavaş parça göndererek zaman aşımını tetikleyen akış.
    YavasSonsuzAkis { parca: Vec<u8>, aralik: Duration },
}

impl SunucuYaniti {
    pub fn html(icerik: &str) -> Self {
        Self::Tam {
            durum: 200,
            basliklar: vec![("Content-Type".into(), "text/html; charset=utf-8".into())],
            govde: icerik.as_bytes().to_vec(),
        }
    }

    pub fn metin(icerik: &str) -> Self {
        Self::Tam {
            durum: 200,
            basliklar: vec![("Content-Type".into(), "text/plain; charset=utf-8".into())],
            govde: icerik.as_bytes().to_vec(),
        }
    }
}

type RotaHaritasi = Arc<Mutex<HashMap<String, VecDeque<SunucuYaniti>>>>;

/// Yerel TCP üzerinde çalışan HTTP veya `rustls` HTTPS test sunucusu.
pub struct YerelWebSunucusu {
    adres: SocketAddr,
    rotalar: RotaHaritasi,
    istek_sayisi: Arc<AtomicUsize>,
    istek_yollari: Arc<Mutex<Vec<String>>>,
    gorev: JoinHandle<()>,
}

impl YerelWebSunucusu {
    /// Şifresiz yerel HTTP sunucusu başlatır.
    pub async fn http_baslat() -> Self {
        Self::ic_baslat(None).await
    }

    /// Test kök CA tarafından imzalanmış sertifikayla yerel HTTPS sunucusu başlatır.
    pub async fn https_baslat() -> Self {
        let tls = tls_sunucu_ayari(TEST_SUNUCU_SERTIFIKA_DER, TEST_SUNUCU_ANAHTAR_DER);
        Self::ic_baslat(Some(tls)).await
    }

    /// Güvenilmeyen (farklı kök) sertifikayla yerel HTTPS sunucusu başlatır.
    pub async fn guvenilmeyen_https_baslat() -> Self {
        let tls = tls_sunucu_ayari(SAHTE_SUNUCU_SERTIFIKA_DER, SAHTE_SUNUCU_ANAHTAR_DER);
        Self::ic_baslat(Some(tls)).await
    }

    /// Test kök CA sertifikasını (DER) döndürür.
    pub fn kok_sertifika_der() -> Vec<u8> {
        TEST_KOK_CA_DER.to_vec()
    }

    async fn ic_baslat(tls_ayari: Option<Arc<rustls::ServerConfig>>) -> Self {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let dinleyici = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("yerel test sunucusu port açabilmeli");
        let adres = dinleyici.local_addr().expect("yerel adres");
        let rotalar: RotaHaritasi = Arc::new(Mutex::new(HashMap::new()));
        let istek_sayisi = Arc::new(AtomicUsize::new(0));
        let istek_yollari = Arc::new(Mutex::new(Vec::new()));

        let r_kopya = rotalar.clone();
        let s_kopya = istek_sayisi.clone();
        let y_kopya = istek_yollari.clone();

        let gorev = tokio::spawn(async move {
            while let Ok((soket, _istemci_adresi)) = dinleyici.accept().await {
                let r = r_kopya.clone();
                let s = s_kopya.clone();
                let y = y_kopya.clone();
                let tls = tls_ayari.clone();
                tokio::spawn(async move {
                    if let Some(ayar) = tls {
                        let _ = https_baglanti_isle(soket, ayar, r, s, y).await;
                    } else {
                        let _ = http_baglanti_isle(soket, r, s, y).await;
                    }
                });
            }
        });

        Self {
            adres,
            rotalar,
            istek_sayisi,
            istek_yollari,
            gorev,
        }
    }

    /// Sunucunun dinlediği yerel `SocketAddr` (`127.0.0.1:<port>`).
    pub fn adres(&self) -> SocketAddr {
        self.adres
    }

    /// Belirli bir yol (ör. `"/dokuman"`) için yanıt tanımlar. Aynı yola birden
    /// fazla yanıt eklenirse sırayla tüketilir; son yanıt sabit kalır.
    pub fn rota_ekle(&self, yol: &str, yanit: SunucuYaniti) {
        let mut kilit = self.rotalar.lock().expect("rota kilidi");
        kilit.entry(yol.to_string()).or_default().push_back(yanit);
    }

    /// Sunucuya ulaşan toplam HTTP/HTTPS istek sayısı.
    pub fn istek_sayisi(&self) -> usize {
        self.istek_sayisi.load(Ordering::SeqCst)
    }

    /// Sunucuya ulaşan istek yollarının listesi.
    pub fn istek_yollari(&self) -> Vec<String> {
        self.istek_yollari.lock().expect("yol kilidi").clone()
    }
}

impl Drop for YerelWebSunucusu {
    fn drop(&mut self) {
        self.gorev.abort();
    }
}

fn tls_sunucu_ayari(
    sertifika_der: &'static [u8],
    anahtar_der: &'static [u8],
) -> Arc<rustls::ServerConfig> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let sertifika = CertificateDer::from(sertifika_der);
    let anahtar = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(anahtar_der));
    let ayar = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![sertifika], anahtar)
        .expect("geçerli test TLS sertifikası ve anahtarı");
    Arc::new(ayar)
}

fn rota_yaniti_sec(rotalar: &RotaHaritasi, yol: &str) -> SunucuYaniti {
    let mut kilit = rotalar.lock().expect("rota kilidi");
    let anahtar = if kilit.contains_key(yol) {
        Some(yol.to_string())
    } else {
        yol.split('?')
            .next()
            .filter(|sadece_yol| kilit.contains_key(*sadece_yol))
            .map(str::to_string)
    };
    if let Some(anahtar) = anahtar
        && let Some(kuyruk) = kilit.get_mut(&anahtar)
    {
        if kuyruk.len() > 1 {
            return kuyruk.pop_front().expect("kuyrukta öğe var");
        }
        if let Some(son) = kuyruk.front() {
            return son.clone();
        }
    }
    SunucuYaniti::Tam {
        durum: 404,
        basliklar: vec![("Content-Type".into(), "text/plain; charset=utf-8".into())],
        govde: b"Bulunamadi".to_vec(),
    }
}

fn istek_yolunu_coz(istek_tamponu: &[u8]) -> Option<String> {
    let metin = String::from_utf8_lossy(istek_tamponu);
    let ilk_satir = metin.lines().next()?;
    let mut parcalar = ilk_satir.split_whitespace();
    let _yontem = parcalar.next()?;
    let ham_hedef = parcalar.next()?;
    if (ham_hedef.starts_with("http://") || ham_hedef.starts_with("https://"))
        && let Ok(u) = url::Url::parse(ham_hedef)
    {
        return Some(match u.query() {
            Some(q) => format!("{}?{q}", u.path()),
            None => u.path().to_string(),
        });
    }
    Some(ham_hedef.to_string())
}

fn durum_metni(durum: u16) -> &'static str {
    match durum {
        200 => "OK",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        401 => "Unauthorized",
        402 => "Payment Required",
        403 => "Forbidden",
        404 => "Not Found",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

fn yanit_baytlari_olustur(yanit: &SunucuYaniti) -> Vec<u8> {
    match yanit {
        SunucuYaniti::Tam {
            durum,
            basliklar,
            govde,
        } => {
            let mut bas = format!(
                "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
                durum,
                durum_metni(*durum),
                govde.len()
            );
            for (k, v) in basliklar {
                bas.push_str(&format!("{k}: {v}\r\n"));
            }
            bas.push_str("\r\n");
            let mut cikti = bas.into_bytes();
            cikti.extend_from_slice(govde);
            cikti
        }
        SunucuYaniti::Yonlendir { durum, hedef } => format!(
            "HTTP/1.1 {} {}\r\nLocation: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            durum,
            durum_metni(*durum),
            hedef
        )
        .into_bytes(),
        SunucuYaniti::EksikGovde {
            bildirilen_uzunluk,
            gonderilen_govde,
        } => {
            let bas = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {bildirilen_uzunluk}\r\nConnection: close\r\n\r\n"
            );
            let mut cikti = bas.into_bytes();
            cikti.extend_from_slice(gonderilen_govde);
            cikti
        }
        SunucuYaniti::YavasSonsuzAkis { .. } => {
            b"HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nConnection: close\r\n\r\n"
                .to_vec()
        }
    }
}

async fn http_baglanti_isle(
    mut soket: TcpStream,
    rotalar: RotaHaritasi,
    istek_sayisi: Arc<AtomicUsize>,
    istek_yollari: Arc<Mutex<Vec<String>>>,
) -> io::Result<()> {
    let mut tampon = Vec::with_capacity(4096);
    let mut parca = [0u8; 2048];
    while !tampon.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = soket.read(&mut parca).await?;
        if n == 0 {
            break;
        }
        tampon.extend_from_slice(&parca[..n]);
        if tampon.len() > 65_536 {
            break;
        }
    }

    let Some(yol) = istek_yolunu_coz(&tampon) else {
        return Ok(());
    };
    istek_sayisi.fetch_add(1, Ordering::SeqCst);
    istek_yollari.lock().expect("yol kilidi").push(yol.clone());

    let yanit = rota_yaniti_sec(&rotalar, &yol);
    let ilk_baytlar = yanit_baytlari_olustur(&yanit);
    soket.write_all(&ilk_baytlar).await?;
    soket.flush().await?;

    if let SunucuYaniti::YavasSonsuzAkis { parca, aralik } = yanit {
        loop {
            tokio::time::sleep(aralik).await;
            if soket.write_all(&parca).await.is_err() {
                break;
            }
            let _ = soket.flush().await;
        }
    }

    let _ = soket.shutdown().await;
    Ok(())
}

async fn https_baglanti_isle(
    mut soket: TcpStream,
    tls_ayari: Arc<rustls::ServerConfig>,
    rotalar: RotaHaritasi,
    istek_sayisi: Arc<AtomicUsize>,
    istek_yollari: Arc<Mutex<Vec<String>>>,
) -> io::Result<()> {
    let mut baglanti =
        ServerConnection::new(tls_ayari).map_err(|e| io::Error::other(e.to_string()))?;
    let mut duz_istek = Vec::with_capacity(4096);
    let mut ag_parca = [0u8; 4096];

    while !duz_istek.windows(4).any(|w| w == b"\r\n\r\n") {
        tls_cikisini_bosalt(&mut baglanti, &mut soket).await?;

        if baglanti.wants_read() {
            let n = soket.read(&mut ag_parca).await?;
            if n == 0 {
                break;
            }
            let mut imlec = io::Cursor::new(&ag_parca[..n]);
            baglanti.read_tls(&mut imlec)?;
            baglanti
                .process_new_packets()
                .map_err(|e| io::Error::other(e.to_string()))?;
        }

        let mut okuma_parca = [0u8; 2048];
        while let Ok(n) = baglanti.reader().read(&mut okuma_parca) {
            if n == 0 {
                break;
            }
            duz_istek.extend_from_slice(&okuma_parca[..n]);
        }
    }

    tls_cikisini_bosalt(&mut baglanti, &mut soket).await?;

    let Some(yol) = istek_yolunu_coz(&duz_istek) else {
        return Ok(());
    };
    istek_sayisi.fetch_add(1, Ordering::SeqCst);
    istek_yollari.lock().expect("yol kilidi").push(yol.clone());

    let yanit = rota_yaniti_sec(&rotalar, &yol);
    let ilk_baytlar = yanit_baytlari_olustur(&yanit);
    baglanti.writer().write_all(&ilk_baytlar)?;
    tls_cikisini_bosalt(&mut baglanti, &mut soket).await?;

    match yanit {
        SunucuYaniti::YavasSonsuzAkis { parca, aralik } => loop {
            tokio::time::sleep(aralik).await;
            if baglanti.writer().write_all(&parca).is_err() {
                break;
            }
            if tls_cikisini_bosalt(&mut baglanti, &mut soket)
                .await
                .is_err()
            {
                break;
            }
        },
        SunucuYaniti::EksikGovde { .. } => {
            // Eksik gövdede close_notify göndermeden TCP bağlantısını kapatıyoruz.
        }
        _ => {
            baglanti.send_close_notify();
            let _ = tls_cikisini_bosalt(&mut baglanti, &mut soket).await;
        }
    }

    let _ = soket.shutdown().await;
    Ok(())
}

async fn tls_cikisini_bosalt(
    baglanti: &mut ServerConnection,
    soket: &mut TcpStream,
) -> io::Result<()> {
    while baglanti.wants_write() {
        let mut tls_cikti = Vec::new();
        baglanti.write_tls(&mut tls_cikti)?;
        if !tls_cikti.is_empty() {
            soket.write_all(&tls_cikti).await?;
        }
    }
    soket.flush().await
}

/// DNS rebinding ve çoklu A/AAAA kaydı senaryolarını deterministik olarak
/// simüle eden test çözücüsü.
#[derive(Debug, Default)]
pub struct TestCozucu {
    kayitlar: Mutex<HashMap<String, VecDeque<Vec<SocketAddr>>>>,
}

impl TestCozucu {
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Belirli bir alan adı için sabit tek bir `SocketAddr` tanımlar.
    pub fn sabit_ekle(&self, host: &str, soket: SocketAddr) {
        self.coklu_yanit_ekle(host, vec![soket]);
    }

    /// Belirli bir alan adı için sıradaki DNS sorgusunda dönecek `Vec<SocketAddr>`
    /// yanıtını kuyruğa ekler. Kuyrukta tek öğe kalırsa sonraki sorgularda
    /// tekrar kullanılır; birden fazla öğe varsa her sorguda sıradaki tüketilir
    /// (DNS rebinding simülasyonu).
    pub fn coklu_yanit_ekle(&self, host: &str, adresler: Vec<SocketAddr>) {
        let mut kilit = self.kayitlar.lock().expect("dns kilidi");
        kilit
            .entry(host.trim().to_ascii_lowercase())
            .or_default()
            .push_back(adresler);
    }
}

impl Cozucu for TestCozucu {
    fn coz(
        &self,
        host: &str,
        _port: u16,
    ) -> Pin<Box<dyn Future<Output = io::Result<Vec<SocketAddr>>> + Send + '_>> {
        let anahtar = host.trim().to_ascii_lowercase();
        let sonuc = {
            let mut kilit = self.kayitlar.lock().expect("dns kilidi");
            match kilit.get_mut(&anahtar) {
                Some(kuyruk) => {
                    if kuyruk.len() > 1 {
                        Ok(kuyruk.pop_front().expect("öğe var"))
                    } else if let Some(son) = kuyruk.front() {
                        Ok(son.clone())
                    } else {
                        Err(io::Error::new(
                            io::ErrorKind::NotFound,
                            format!("`{anahtar}` için DNS kaydı yok"),
                        ))
                    }
                }
                None => Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("`{anahtar}` için DNS kaydı bulunamadı"),
                )),
            }
        };
        Box::pin(async move { sonuc })
    }
}
