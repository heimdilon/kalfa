//! Kalfa çıkış vekili ve ağ denetimi (aşama 3 tasarımı §8).
//!
//! Ağlı komutlar yalnızca izin listesindeki hostlara, bu vekil üzerinden çıkabilir.
//! Vekil hedefi sınırlar, içeriği değil (TLS açılmaz).

pub mod adres;
pub mod istek;
#[cfg(unix)]
pub mod kanal;
pub mod liste;
pub mod ortam;
pub mod sni;

use std::future::Future;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;

pub use adres::{AdresReddi, adres_izinli_mi};
pub use istek::{AZAMI_BASLIK_BOYUTU, BASLIK_ZAMAN_ASIMI, Istek, istek_ayristir};
pub use liste::{
    HostKaynagi, HostKurali, IzinListesi, ListeHatasi, VARSAYILAN_HOSTLAR, ag_hostu_coz,
};
pub use sni::{SniSonucu, sni_ayristir};

/// DNS / ad çözümleyici arayüzü.
pub trait Cozucu: Send + Sync {
    fn coz<'a>(
        &'a self,
        host: &'a str,
        port: u16,
    ) -> Pin<Box<dyn Future<Output = io::Result<Vec<SocketAddr>>> + Send + 'a>>;
}

/// Tokio'nun sistem çözümleyicisini kullanan varsayılan Cozucu.
#[derive(Debug, Default, Clone)]
pub struct SistemCozucu;

impl Cozucu for SistemCozucu {
    fn coz<'a>(
        &'a self,
        host: &'a str,
        port: u16,
    ) -> Pin<Box<dyn Future<Output = io::Result<Vec<SocketAddr>>> + Send + 'a>> {
        let hedef = format!("{host}:{port}");
        Box::pin(async move {
            let addrs = tokio::net::lookup_host(hedef).await?;
            Ok(addrs.collect())
        })
    }
}

/// Uçtan uca testlerin sahte ad çözümü: `KALFA_E2E_SAHTE_DNS`
/// (`host:port=ip:port;…`). Yalnızca `e2e` özelliğiyle derlenen uygulamada
/// okunur; sürüm derlemesinde değişken yok sayılır, sistem çözümleyicisi ve
/// adres denetimi kullanılır. İkinci değer `VekilAyari::yerel_izin`'dir.
pub fn e2e_cozucu_ve_yerel_izin() -> (Option<Arc<dyn Cozucu>>, bool) {
    sahte_dns(std::env::var("KALFA_E2E_SAHTE_DNS").ok())
}

#[cfg(not(feature = "e2e"))]
fn sahte_dns(_deger: Option<String>) -> (Option<Arc<dyn Cozucu>>, bool) {
    (None, false)
}

#[cfg(feature = "e2e")]
fn sahte_dns(deger: Option<String>) -> (Option<Arc<dyn Cozucu>>, bool) {
    let mut eslemeler = std::collections::HashMap::new();
    for pair in deger.as_deref().unwrap_or("").split(';') {
        let Some((src, dst)) = pair.trim().split_once('=') else {
            continue;
        };
        let (host, port) = match src.rsplit_once(':') {
            Some((h, p)) => (h.trim().to_lowercase(), p.parse().unwrap_or(443)),
            None => (src.trim().to_lowercase(), 443),
        };
        if let Ok(addr) = dst.trim().parse::<SocketAddr>() {
            eslemeler.insert((host, port), vec![addr]);
        }
    }
    if eslemeler.is_empty() {
        return (None, false);
    }
    (Some(Arc::new(E2eCozucu { eslemeler })), true)
}

#[cfg(feature = "e2e")]
struct E2eCozucu {
    eslemeler: std::collections::HashMap<(String, u16), Vec<SocketAddr>>,
}

#[cfg(feature = "e2e")]
impl Cozucu for E2eCozucu {
    fn coz<'a>(
        &'a self,
        host: &'a str,
        port: u16,
    ) -> Pin<Box<dyn Future<Output = io::Result<Vec<SocketAddr>>> + Send + 'a>> {
        let key = (host.to_lowercase(), port);
        let res = self.eslemeler.get(&key).cloned().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("{host}:{port} e2e sahte dns'te bulunamadı"),
            )
        });
        Box::pin(async move { res })
    }
}

/// Vekilin bir bağlantı isteği hakkındaki kararı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgKarari {
    Izinli,
    ListedeYok,
    IpIleIstek,
    YasakAdres(AdresReddi),
    SniUyusmuyor { sni: Option<String> },
    Cozulemedi(String),
    Baglanamadi(String),
    Gecersiz(String),
}

/// Vekilden geçen ya da reddedilen bir bağlantının kaydı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgBaglantisi {
    pub host: String,
    pub port: u16,
    pub karar: AgKarari,
    pub giden: u64,
    pub gelen: u64,
}

/// Bir çalıştırmanın vekil üzerinden geçen ağ özeti.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AgOzeti {
    pub baglantilar: Vec<AgBaglantisi>,
}

/// Vekilin çalışma yapılandırması.
#[derive(Clone)]
pub struct VekilAyari {
    pub liste: IzinListesi,
    pub cozucu: Arc<dyn Cozucu>,
    /// En çok eşzamanlı bağlantı sayısı (varsayılan 64).
    pub azami_baglanti: usize,
    /// Boşta kalma zaman aşımı süresi (varsayılan 5 dakika).
    pub bosta_kalma: Duration,
    /// Testlerde yerel sahte sunucuya (127.0.0.1) bağlanmaya izin verir (varsayılan false).
    pub yerel_izin: bool,
}

impl Default for VekilAyari {
    fn default() -> Self {
        Self {
            liste: IzinListesi::yeni(&[], &[]),
            cozucu: Arc::new(SistemCozucu),
            azami_baglanti: 64,
            bosta_kalma: Duration::from_secs(300),
            yerel_izin: false,
        }
    }
}

/// Kalfa çıkış vekili.
pub struct Vekil {
    ayar: Arc<VekilAyari>,
    kayitlar: Arc<Mutex<Vec<AgBaglantisi>>>,
    aktif_baglantilar: Arc<AtomicUsize>,
    iptal: CancellationToken,
    gorevler: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

impl Vekil {
    /// Yapılandırmayla vekili başlatır.
    pub fn baslat(ayar: VekilAyari) -> Self {
        Self {
            ayar: Arc::new(ayar),
            kayitlar: Arc::new(Mutex::new(Vec::new())),
            aktif_baglantilar: Arc::new(AtomicUsize::new(0)),
            iptal: CancellationToken::new(),
            gorevler: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Yeni bir akışı vekilin işlemesi için verir.
    ///
    /// Linux'ta köprünün devrettiği TCP soketidir; Windows'ta adlandırılmış borudur.
    pub fn baglanti_ver(&self, akis: impl AsyncRead + AsyncWrite + Unpin + Send + 'static) {
        let aktif = self.aktif_baglantilar.fetch_add(1, Ordering::SeqCst);
        if aktif >= self.ayar.azami_baglanti {
            self.aktif_baglantilar.fetch_sub(1, Ordering::SeqCst);
            return;
        }

        let ayar = Arc::clone(&self.ayar);
        let kayitlar = Arc::clone(&self.kayitlar);
        let aktif_baglantilar = Arc::clone(&self.aktif_baglantilar);
        let iptal = self.iptal.child_token();

        let handle = tokio::spawn(async move {
            baglanti_isle(akis, ayar, kayitlar, iptal).await;
            aktif_baglantilar.fetch_sub(1, Ordering::SeqCst);
        });

        if let Ok(mut g) = self.gorevler.lock() {
            g.push(handle);
        }
    }

    /// Açık bağlantıları sonlandırır ve komutun ağ özetini döndürür.
    pub async fn bitir(&self) -> AgOzeti {
        self.iptal.cancel();

        let gorevler = if let Ok(mut g) = self.gorevler.lock() {
            std::mem::take(&mut *g)
        } else {
            Vec::new()
        };

        for g in gorevler {
            let _ = g.await;
        }

        let baglantilar = if let Ok(mut k) = self.kayitlar.lock() {
            std::mem::take(&mut *k)
        } else {
            Vec::new()
        };

        AgOzeti { baglantilar }
    }

    /// Geçersiz bir bağlantı girişimini (örneğin iş nesnesinde olmayan istemci) kaydeder.
    pub fn gecersiz_baglanti_kaydet(&self, neden: impl Into<String>) {
        kaydet(
            &self.kayitlar,
            AgBaglantisi {
                host: "-".into(),
                port: 0,
                karar: AgKarari::Gecersiz(neden.into()),
                giden: 0,
                gelen: 0,
            },
        );
    }
}

fn kaydet(kayitlar: &Mutex<Vec<AgBaglantisi>>, b: AgBaglantisi) {
    if let Ok(mut k) = kayitlar.lock() {
        k.push(b);
    }
}

async fn baglanti_isle<S>(
    mut akis: S,
    ayar: Arc<VekilAyari>,
    kayitlar: Arc<Mutex<Vec<AgBaglantisi>>>,
    iptal: CancellationToken,
) where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    // 1. İstek başlığını oku (en çok 8 KiB, 10 sn)
    let mut baslik_tamponu = Vec::with_capacity(1024);
    let mut gecici = [0u8; 1024];

    let ayrisan_istek = loop {
        let okuma = tokio::select! {
            _ = iptal.cancelled() => return,
            res = tokio::time::timeout(BASLIK_ZAMAN_ASIMI, akis.read(&mut gecici)) => res,
        };

        match okuma {
            Ok(Ok(0)) => return,
            Ok(Ok(n)) => {
                baslik_tamponu.extend_from_slice(&gecici[..n]);
                if baslik_tamponu.windows(4).any(|w| w == b"\r\n\r\n")
                    || baslik_tamponu.windows(2).any(|w| w == b"\n\n")
                {
                    break istek_ayristir(&baslik_tamponu);
                }
                if baslik_tamponu.len() >= AZAMI_BASLIK_BOYUTU {
                    break Istek::Gecersiz("başlık boyutu 8 KiB sınırını aştı".into());
                }
            }
            _ => return,
        }
    };

    match ayrisan_istek {
        Istek::Gecersiz(neden) => {
            let _ = akis
                .write_all(
                    b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await;
            kaydet(
                &kayitlar,
                AgBaglantisi {
                    host: "-".into(),
                    port: 0,
                    karar: AgKarari::Gecersiz(neden),
                    giden: 0,
                    gelen: 0,
                },
            );
        }
        Istek::Connect { host, port, http10 } => {
            connect_isle(akis, host, port, http10, ayar, kayitlar, iptal).await;
        }
        Istek::MutlakHttp {
            metot: _,
            host,
            port,
            hedef_istek,
        } => {
            mutlak_http_isle(akis, host, port, hedef_istek, ayar, kayitlar, iptal).await;
        }
    }
}

async fn connect_isle<S>(
    mut akis: S,
    host: String,
    port: u16,
    http10: bool,
    ayar: Arc<VekilAyari>,
    kayitlar: Arc<Mutex<Vec<AgBaglantisi>>>,
    iptal: CancellationToken,
) where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let temiz_host = host.trim_matches(['[', ']']);
    let is_ip = temiz_host.parse::<IpAddr>().is_ok();

    if is_ip {
        let kural = ayar.liste.bul(&host, port);
        if kural.is_none() {
            let govde =
                format!("Ağ çıkışı engellendi: '{host}:{port}' IP adresi izin listesinde yok.\n");
            let yanit = format!(
                "HTTP/1.1 403 Forbidden\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{govde}",
                govde.len()
            );
            let _ = akis.write_all(yanit.as_bytes()).await;
            kaydet(
                &kayitlar,
                AgBaglantisi {
                    host,
                    port,
                    karar: AgKarari::IpIleIstek,
                    giden: 0,
                    gelen: 0,
                },
            );
            return;
        }
    }

    let kural = match ayar.liste.bul(&host, port) {
        Some(k) => k,
        None => {
            let govde = format!("Ağ çıkışı engellendi: '{host}:{port}' izin listesinde yok.\n");
            let yanit = format!(
                "HTTP/1.1 403 Forbidden\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{govde}",
                govde.len()
            );
            let _ = akis.write_all(yanit.as_bytes()).await;
            kaydet(
                &kayitlar,
                AgBaglantisi {
                    host,
                    port,
                    karar: AgKarari::ListedeYok,
                    giden: 0,
                    gelen: 0,
                },
            );
            return;
        }
    };

    let ozel_ag = kural.ozel_ag;

    let adresler = match ayar.cozucu.coz(&host, port).await {
        Ok(a) => a,
        Err(e) => {
            let govde = format!("Ağ çıkışı engellendi: '{host}:{port}' adresi çözülemedi: {e}\n");
            let yanit = format!(
                "HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{govde}",
                govde.len()
            );
            let _ = akis.write_all(yanit.as_bytes()).await;
            kaydet(
                &kayitlar,
                AgBaglantisi {
                    host,
                    port,
                    karar: AgKarari::Cozulemedi(e.to_string()),
                    giden: 0,
                    gelen: 0,
                },
            );
            return;
        }
    };

    if adresler.is_empty() {
        let govde = format!("Ağ çıkışı engellendi: '{host}:{port}' için adres bulunamadı.\n");
        let yanit = format!(
            "HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{govde}",
            govde.len()
        );
        let _ = akis.write_all(yanit.as_bytes()).await;
        kaydet(
            &kayitlar,
            AgBaglantisi {
                host,
                port,
                karar: AgKarari::Cozulemedi("adres bulunamadı".into()),
                giden: 0,
                gelen: 0,
            },
        );
        return;
    }

    if !ayar.yerel_izin {
        for addr in &adresler {
            if let Err(red) = adres_izinli_mi(addr.ip(), ozel_ag) {
                let govde = format!(
                    "Ağ çıkışı engellendi: '{host}:{port}' çözülen adresi ({}) yasak: {red}\n",
                    addr.ip()
                );
                let yanit = format!(
                    "HTTP/1.1 403 Forbidden\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{govde}",
                    govde.len()
                );
                let _ = akis.write_all(yanit.as_bytes()).await;
                kaydet(
                    &kayitlar,
                    AgBaglantisi {
                        host,
                        port,
                        karar: AgKarari::YasakAdres(red),
                        giden: 0,
                        gelen: 0,
                    },
                );
                return;
            }
        }
    }

    let mut hedef = match TcpStream::connect(adresler[0]).await {
        Ok(s) => s,
        Err(e) => {
            let govde =
                format!("Ağ çıkışı engellendi: '{host}:{port}' sunucusuna bağlanılamadı: {e}\n");
            let yanit = format!(
                "HTTP/1.1 504 Gateway Timeout\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{govde}",
                govde.len()
            );
            let _ = akis.write_all(yanit.as_bytes()).await;
            kaydet(
                &kayitlar,
                AgBaglantisi {
                    host,
                    port,
                    karar: AgKarari::Baglanamadi(e.to_string()),
                    giden: 0,
                    gelen: 0,
                },
            );
            return;
        }
    };

    let yanit_200 = if http10 {
        b"HTTP/1.0 200 Connection Established\r\n\r\n".as_slice()
    } else {
        b"HTTP/1.1 200 Connection Established\r\n\r\n".as_slice()
    };
    if akis.write_all(yanit_200).await.is_err() {
        return;
    }
    let _ = akis.flush().await;

    let mut ilk_kayit = Vec::new();
    let mut baslik = [0u8; 5];

    let okuma_sonucu = tokio::select! {
        _ = iptal.cancelled() => {
            kaydet(
                &kayitlar,
                AgBaglantisi {
                    host,
                    port,
                    karar: AgKarari::Izinli,
                    giden: 0,
                    gelen: 0,
                },
            );
            return;
        }
        res = tokio::time::timeout(ayar.bosta_kalma, akis.read(&mut baslik)) => res,
    };

    let n = match okuma_sonucu {
        Ok(Ok(n)) if n > 0 => n,
        _ => {
            // Zaman aşımı, EOF veya okuma hatası: 200 verildiği için izinli bağlantı kaydedilir
            kaydet(
                &kayitlar,
                AgBaglantisi {
                    host,
                    port,
                    karar: AgKarari::Izinli,
                    giden: 0,
                    gelen: 0,
                },
            );
            return;
        }
    };

    ilk_kayit.extend_from_slice(&baslik[..n]);

    if ilk_kayit[0] == 0x16 {
        while ilk_kayit.len() < 5 {
            let mut k = [0u8; 1];
            if akis.read_exact(&mut k).await.is_err() {
                return;
            }
            ilk_kayit.push(k[0]);
        }
        let uzunluk = usize::from(u16::from_be_bytes([ilk_kayit[3], ilk_kayit[4]]));
        if uzunluk > 16 * 1024 {
            kaydet(
                &kayitlar,
                AgBaglantisi {
                    host,
                    port,
                    karar: AgKarari::Gecersiz("ClientHello 16 KiB sınırını aşıyor".into()),
                    giden: 0,
                    gelen: 0,
                },
            );
            return;
        }
        while ilk_kayit.len() < 5 + uzunluk {
            let gereken = (5 + uzunluk) - ilk_kayit.len();
            let mut parca = vec![0u8; gereken.min(1024)];
            let okunan = match akis.read(&mut parca).await {
                Ok(n) if n > 0 => n,
                _ => return,
            };
            ilk_kayit.extend_from_slice(&parca[..okunan]);
        }

        match sni_ayristir(&ilk_kayit) {
            SniSonucu::Sni(sni_opt) => {
                let temiz_hedef = host.trim_matches(['[', ']']);
                let eslesti = sni_opt.as_deref().map(|s| s.to_lowercase())
                    == Some(temiz_hedef.to_lowercase());
                if !eslesti {
                    kaydet(
                        &kayitlar,
                        AgBaglantisi {
                            host,
                            port,
                            karar: AgKarari::SniUyusmuyor { sni: sni_opt },
                            giden: 0,
                            gelen: 0,
                        },
                    );
                    return;
                }
            }
            SniSonucu::Bozuk(e) => {
                kaydet(
                    &kayitlar,
                    AgBaglantisi {
                        host,
                        port,
                        karar: AgKarari::Gecersiz(e),
                        giden: 0,
                        gelen: 0,
                    },
                );
                return;
            }
            SniSonucu::TlsDegil => {}
        }
    }

    if hedef.write_all(&ilk_kayit).await.is_err() {
        return;
    }
    let _ = hedef.flush().await;
    let ilk_giden = ilk_kayit.len() as u64;

    let (giden_ek, gelen) =
        aktar_bosta_kalma(&mut akis, &mut hedef, ayar.bosta_kalma, &iptal).await;
    kaydet(
        &kayitlar,
        AgBaglantisi {
            host,
            port,
            karar: AgKarari::Izinli,
            giden: ilk_giden + giden_ek,
            gelen,
        },
    );
}

async fn mutlak_http_isle<S>(
    mut akis: S,
    host: String,
    port: u16,
    hedef_istek: Vec<u8>,
    ayar: Arc<VekilAyari>,
    kayitlar: Arc<Mutex<Vec<AgBaglantisi>>>,
    iptal: CancellationToken,
) where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    if port != 80 || ayar.liste.bul(&host, 80).is_none() {
        let govde = format!("Ağ çıkışı engellendi: '{host}:{port}' izin listesinde yok.\n");
        let yanit = format!(
            "HTTP/1.1 403 Forbidden\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{govde}",
            govde.len()
        );
        let _ = akis.write_all(yanit.as_bytes()).await;
        kaydet(
            &kayitlar,
            AgBaglantisi {
                host,
                port,
                karar: AgKarari::ListedeYok,
                giden: 0,
                gelen: 0,
            },
        );
        return;
    }

    let kural = match ayar.liste.bul(&host, 80) {
        Some(k) => k,
        None => return,
    };
    let ozel_ag = kural.ozel_ag;

    let adresler = match ayar.cozucu.coz(&host, 80).await {
        Ok(a) if !a.is_empty() => a,
        Ok(_) => {
            kaydet(
                &kayitlar,
                AgBaglantisi {
                    host,
                    port,
                    karar: AgKarari::Cozulemedi("adres bulunamadı".into()),
                    giden: 0,
                    gelen: 0,
                },
            );
            return;
        }
        Err(e) => {
            kaydet(
                &kayitlar,
                AgBaglantisi {
                    host,
                    port,
                    karar: AgKarari::Cozulemedi(e.to_string()),
                    giden: 0,
                    gelen: 0,
                },
            );
            return;
        }
    };

    if !ayar.yerel_izin {
        for addr in &adresler {
            if let Err(red) = adres_izinli_mi(addr.ip(), ozel_ag) {
                let govde = format!(
                    "Ağ çıkışı engellendi: '{host}:{port}' çözülen adresi ({}) yasak: {red}\n",
                    addr.ip()
                );
                let yanit = format!(
                    "HTTP/1.1 403 Forbidden\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{govde}",
                    govde.len()
                );
                let _ = akis.write_all(yanit.as_bytes()).await;
                kaydet(
                    &kayitlar,
                    AgBaglantisi {
                        host,
                        port,
                        karar: AgKarari::YasakAdres(red),
                        giden: 0,
                        gelen: 0,
                    },
                );
                return;
            }
        }
    }

    let mut hedef = match TcpStream::connect(adresler[0]).await {
        Ok(s) => s,
        Err(e) => {
            kaydet(
                &kayitlar,
                AgBaglantisi {
                    host,
                    port,
                    karar: AgKarari::Baglanamadi(e.to_string()),
                    giden: 0,
                    gelen: 0,
                },
            );
            return;
        }
    };

    if hedef.write_all(&hedef_istek).await.is_err() {
        return;
    }
    let _ = hedef.flush().await;
    let ilk_giden = hedef_istek.len() as u64;

    let (giden_ek, gelen) =
        aktar_bosta_kalma(&mut akis, &mut hedef, ayar.bosta_kalma, &iptal).await;
    kaydet(
        &kayitlar,
        AgBaglantisi {
            host,
            port,
            karar: AgKarari::Izinli,
            giden: ilk_giden + giden_ek,
            gelen,
        },
    );
}

async fn aktar_bosta_kalma<R, W>(
    istemci: &mut R,
    hedef: &mut W,
    bosta_kalma: Duration,
    iptal: &CancellationToken,
) -> (u64, u64)
where
    R: AsyncRead + AsyncWrite + Unpin,
    W: AsyncRead + AsyncWrite + Unpin,
{
    let (mut cr, mut cw) = tokio::io::split(istemci);
    let (mut hr, mut hw) = tokio::io::split(hedef);

    let mut buf_ch = [0u8; 8192];
    let mut buf_hc = [0u8; 8192];

    let mut giden: u64 = 0;
    let mut gelen: u64 = 0;

    let mut ch_kapali = false;
    let mut hc_kapali = false;

    loop {
        if ch_kapali && hc_kapali {
            break;
        }

        tokio::select! {
            _ = iptal.cancelled() => {
                break;
            }
            res = tokio::time::timeout(bosta_kalma, cr.read(&mut buf_ch)), if !ch_kapali => {
                match res {
                    Ok(Ok(0)) => {
                        ch_kapali = true;
                        let _ = hw.shutdown().await;
                    }
                    Ok(Ok(n)) => {
                        if hw.write_all(&buf_ch[..n]).await.is_err() {
                            break;
                        }
                        let _ = hw.flush().await;
                        giden += n as u64;
                    }
                    _ => {
                        break;
                    }
                }
            }
            res = tokio::time::timeout(bosta_kalma, hr.read(&mut buf_hc)), if !hc_kapali => {
                match res {
                    Ok(Ok(0)) => {
                        hc_kapali = true;
                        let _ = cw.shutdown().await;
                    }
                    Ok(Ok(n)) => {
                        if cw.write_all(&buf_hc[..n]).await.is_err() {
                            break;
                        }
                        let _ = cw.flush().await;
                        gelen += n as u64;
                    }
                    _ => {
                        break;
                    }
                }
            }
        }
    }

    (giden, gelen)
}

#[cfg(all(test, not(feature = "e2e")))]
mod testler {
    /// Sürüm derlemesinde `KALFA_E2E_SAHTE_DNS` vekilin çözümünü ve adres
    /// denetimini değiştiremez.
    #[test]
    fn surum_derlemesinde_sahte_dns_yok_sayilir() {
        let (cozucu, yerel_izin) = super::sahte_dns(Some(
            "evil.test:443=127.0.0.1:1;169.254.169.254=10.0.0.1:80".into(),
        ));
        assert!(cozucu.is_none());
        assert!(!yerel_izin);
    }
}
