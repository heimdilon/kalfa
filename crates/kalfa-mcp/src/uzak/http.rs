//! Sınırlı uzak MCP Streamable HTTP taşıması (`SinirliHttpTasima`) ve
//! DNS/IP/yönlendirme ağ politikası doğrulayıcısı.
//!
//! Güvenlik garantileri:
//! - Her HTTP isteği ve her yönlendirme adımı (`Location`) soket açılmadan önce
//!   URL şeması, kullanıcı bilgisi, fragment ve DNS/IP politikasına göre doğrulanır.
//! - Çözülen **tüm** IP adresleri `kalfa_sandbox::vekil::adres::adres_izinli_mi`
//!   kontrollerinden geçmek zorundadır; tek bir yasaklı IP (`127.0.0.1`, `::1`,
//!   `169.254.169.254`, `0.0.0.0`, özel ağ vb.) bile isteği reddeder.
//! - Doğrulanan `SocketAddr` listesi `reqwest::ClientBuilder::resolve_to_addrs`
//!   ile sabitlenir (`no_proxy` + `redirect::Policy::none()`); böylece DNS
//!   rebinding veya sistem vekili sızıntısı önlenir.
//! - Farklı origin'e yönlendirmede `Authorization`, gizli başlıklar veya
//!   `Mcp-Session-Id` asla taşınmaz; kimlikli bağlantıda cross-origin yönlendirme
//!   doğrudan reddedilir (`TasimaHatasi::AgPolitikasiReddi`).
//! - `tools/call` çağrılarında otomatik tekrar (`retry`) **kesinlikle** yapılmaz.
//!   Gönderimi başlamış bir `tools/call` yanıt tamamlanmadan kesilirse veya iptal
//!   bildirimi gönderilirse durum [`TasimaHatasi::BelirsizCagriSonucu`] olarak
//!   mühürlenir.

use std::collections::{BTreeMap, HashSet};
use std::future::Future;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::StreamExt;
use reqwest::header::{
    ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue, LOCATION,
    WWW_AUTHENTICATE,
};
use rmcp::RoleClient;
use rmcp::model::{
    ClientJsonRpcMessage, ClientNotification, ErrorCode, ErrorData, GetMeta, RequestId,
    ServerJsonRpcMessage, ServerNotification, ServerResult,
};
use rmcp::transport::Transport;
use serde_json::Value;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use url::Url;

use crate::maske::GizliDeger;
use crate::tanim::UzakMcpTanim;
use crate::tasima::{
    AbonelikKimlikleri, CERCEVE, KUYRUK, TasimaHatasi, TasimaKontrol, YenilemeDurumu,
    arac_cagrisi_yanitina_isaret_ekle,
};
use crate::uzak::oturum::{BelirsizCagriNedeni, UzakOturumDurumu};
use crate::uzak::sse::SseAkisAyristiricisi;

/// Tek bir HTTP isteğinde izin verilen azami güvenli yönlendirme sayısı.
pub const AZAMI_YONLENDIRME: usize = 5;

/// DNS çözümleme sözleşmesi; üretimde [`SistemDnsCozucu`], birim/entegrasyon
/// testlerinde DNS rebinding senaryoları için [`SabitDnsCozucu`] kullanılır.
pub trait UzakDnsCozucu: Send + Sync {
    fn coz<'a>(
        &'a self,
        host: &'a str,
        port: u16,
    ) -> Pin<Box<dyn Future<Output = io::Result<Vec<SocketAddr>>> + Send + 'a>>;
}

/// İşletim sistemi DNS çözümleyicisi (`tokio::net::lookup_host`).
#[derive(Debug, Default, Clone, Copy)]
pub struct SistemDnsCozucu;

impl UzakDnsCozucu for SistemDnsCozucu {
    fn coz<'a>(
        &'a self,
        host: &'a str,
        port: u16,
    ) -> Pin<Box<dyn Future<Output = io::Result<Vec<SocketAddr>>> + Send + 'a>> {
        Box::pin(async move {
            let adresler: Vec<SocketAddr> = tokio::net::lookup_host((host, port)).await?.collect();
            if adresler.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::AddrNotAvailable,
                    "DNS hiçbir adres döndürmedi",
                ));
            }
            Ok(adresler)
        })
    }
}

/// Testlerde belirli host adlarını sabit IP listelerine çözen DNS çözümleyici.
#[derive(Default, Clone)]
pub struct SabitDnsCozucu {
    kayitlar: Arc<Mutex<BTreeMap<String, Vec<IpAddr>>>>,
}

impl SabitDnsCozucu {
    pub fn yeni() -> Self {
        Self::default()
    }

    pub fn kayit_ekle(&self, host: impl Into<String>, ipler: Vec<IpAddr>) {
        if let Ok(mut g) = self.kayitlar.lock() {
            g.insert(host.into().to_ascii_lowercase(), ipler);
        }
    }
}

impl UzakDnsCozucu for SabitDnsCozucu {
    fn coz<'a>(
        &'a self,
        host: &'a str,
        port: u16,
    ) -> Pin<Box<dyn Future<Output = io::Result<Vec<SocketAddr>>> + Send + 'a>> {
        Box::pin(async move {
            let anahtar = host
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_ascii_lowercase();
            if let Ok(ip) = anahtar.parse::<IpAddr>() {
                return Ok(vec![SocketAddr::new(ip, port)]);
            }
            let ipler = self
                .kayitlar
                .lock()
                .ok()
                .and_then(|g| g.get(&anahtar).cloned())
                .unwrap_or_default();
            if ipler.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "sabit DNS kaydı bulunamadı",
                ));
            }
            Ok(ipler
                .into_iter()
                .map(|ip| SocketAddr::new(ip, port))
                .collect())
        })
    }
}

/// Uzak HTTP isteğine eklenecek gizli başlık veya Bearer belirteci.
///
/// `Debug` uygulanmaz; ham değer yalnız soket başlığına yazılır ve maskeleme
/// listesine [`GizliDeger`] olarak eklenir.
#[derive(Clone)]
pub struct UzakGizliBaslik {
    baslik_adi: HeaderName,
    baslik_degeri: HeaderValue,
    maskeler: Vec<GizliDeger>,
}

impl UzakGizliBaslik {
    /// `Authorization: Bearer <token>` başlığı oluşturur.
    pub fn bearer(token: &str) -> Result<Self, TasimaHatasi> {
        let temiz = token.trim();
        if temiz.is_empty() || temiz.chars().any(char::is_control) {
            return Err(TasimaHatasi::YetkiGerekli);
        }
        let tam = format!("Bearer {temiz}");
        let baslik_degeri = HeaderValue::from_str(&tam).map_err(|_| TasimaHatasi::YetkiGerekli)?;
        let mut maskeler = Vec::new();
        if let Ok(m) = GizliDeger::yeni(temiz.to_owned()) {
            maskeler.push(m);
        }
        if let Ok(m) = GizliDeger::yeni(tam) {
            maskeler.push(m);
        }
        Ok(Self {
            baslik_adi: AUTHORIZATION,
            baslik_degeri,
            maskeler,
        })
    }

    /// Sabit gizli başlık (`X-Api-Key: <deger>` vb.) oluşturur.
    pub fn sabit_baslik(ad: &str, deger: &str) -> Result<Self, TasimaHatasi> {
        let temiz_ad = ad.trim().to_ascii_lowercase();
        if matches!(
            temiz_ad.as_str(),
            "host" | "content-length" | "transfer-encoding" | "connection" | "cookie"
        ) {
            return Err(TasimaHatasi::YetkiGerekli);
        }
        let baslik_adi =
            HeaderName::from_bytes(temiz_ad.as_bytes()).map_err(|_| TasimaHatasi::YetkiGerekli)?;
        let baslik_degeri = HeaderValue::from_str(deger).map_err(|_| TasimaHatasi::YetkiGerekli)?;
        let mut maskeler = Vec::new();
        if let Ok(m) = GizliDeger::yeni(deger.to_owned()) {
            maskeler.push(m);
        }
        Ok(Self {
            baslik_adi,
            baslik_degeri,
            maskeler,
        })
    }

    pub fn maskeler(&self) -> &[GizliDeger] {
        &self.maskeler
    }
}

/// Uzak Streamable HTTP taşıma yapılandırması.
#[derive(Clone)]
pub struct UzakHttpSecenekleri {
    pub uc_url: String,
    pub test_fiksturu_loopback: bool,
    pub ozel_ag_izinli: bool,
    pub istek_zamani_asimi: Duration,
    pub cerceve_siniri: usize,
    pub gizli_baslik: Option<UzakGizliBaslik>,
    pub ek_maskeler: Vec<GizliDeger>,
    pub dns_cozucu: Arc<dyn UzakDnsCozucu>,
}

impl UzakHttpSecenekleri {
    pub fn tanimdan(tanim: &UzakMcpTanim) -> Self {
        Self {
            uc_url: tanim.kanonik_uc_url().to_owned(),
            test_fiksturu_loopback: tanim.test_fiksturu_loopback(),
            ozel_ag_izinli: false,
            istek_zamani_asimi: tanim.cagri_zamani_asimi(),
            cerceve_siniri: CERCEVE,
            gizli_baslik: None,
            ek_maskeler: Vec::new(),
            dns_cozucu: Arc::new(SistemDnsCozucu),
        }
    }

    pub fn gizli_baslik_ile(mut self, gizli_baslik: Option<UzakGizliBaslik>) -> Self {
        self.gizli_baslik = gizli_baslik;
        self
    }

    pub fn dns_cozucu_ile(mut self, dns_cozucu: Arc<dyn UzakDnsCozucu>) -> Self {
        self.dns_cozucu = dns_cozucu;
        self
    }

    pub fn cerceve_siniri_ile(mut self, cerceve_siniri: usize) -> Self {
        self.cerceve_siniri = cerceve_siniri;
        self
    }

    pub(crate) fn tum_maskeler(&self) -> Vec<GizliDeger> {
        let mut sonuc = self.ek_maskeler.clone();
        if let Some(ref b) = self.gizli_baslik {
            sonuc.extend_from_slice(b.maskeler());
        }
        sonuc
    }
}

/// Verilen URL'yi ve DNS ile çözülen tüm IP adreslerini ağ politikasına göre
/// doğrular; DNS rebinding'i önlemek için IP'leri sabitlenmiş bir `reqwest::Client` döner.
pub async fn dogrulanmis_istemci_ve_url_hazirla(
    ham_url: &str,
    test_fiksturu_loopback: bool,
    ozel_ag_izinli: bool,
    zaman_asimi: Duration,
    dns_cozucu: &dyn UzakDnsCozucu,
) -> Result<(reqwest::Client, Url, Vec<SocketAddr>), TasimaHatasi> {
    let url = Url::parse(ham_url).map_err(|_| TasimaHatasi::AgPolitikasiReddi)?;
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return Err(TasimaHatasi::AgPolitikasiReddi);
    }
    let Some(host_str) = url.host_str() else {
        return Err(TasimaHatasi::AgPolitikasiReddi);
    };
    let Some(port) = url.port_or_known_default() else {
        return Err(TasimaHatasi::AgPolitikasiReddi);
    };

    let temiz_host = host_str.trim_start_matches('[').trim_end_matches(']');
    let loopback_host =
        matches!(temiz_host, "127.0.0.1" | "::1") || temiz_host.eq_ignore_ascii_case("localhost");

    match url.scheme() {
        "https" => {}
        "http" if test_fiksturu_loopback && loopback_host => {}
        _ => return Err(TasimaHatasi::AgPolitikasiReddi),
    }

    let adresler = if let Ok(ip) = temiz_host.parse::<IpAddr>() {
        vec![SocketAddr::new(ip, port)]
    } else {
        dns_cozucu
            .coz(temiz_host, port)
            .await
            .map_err(|_| TasimaHatasi::AgPolitikasiReddi)?
    };

    if adresler.is_empty() {
        return Err(TasimaHatasi::AgPolitikasiReddi);
    }

    for soket in &adresler {
        let ip = soket.ip();
        if test_fiksturu_loopback && loopback_host && ip_gercek_loopback_mi(ip) {
            continue;
        }
        kalfa_sandbox::vekil::adres::adres_izinli_mi(ip, ozel_ag_izinli)
            .map_err(|_| TasimaHatasi::AgPolitikasiReddi)?;
    }

    let tls_yapilandirma = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|_| TasimaHatasi::GirdiCikti)?
    .with_root_certificates(kok_sertifikalar())
    .with_no_client_auth();

    let mut olusturucu = reqwest::Client::builder()
        .use_preconfigured_tls(tls_yapilandirma)
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .timeout(zaman_asimi);

    if temiz_host.parse::<IpAddr>().is_err() {
        olusturucu = olusturucu.resolve_to_addrs(temiz_host, &adresler);
    }

    let istemci = olusturucu.build().map_err(|_| TasimaHatasi::GirdiCikti)?;
    Ok((istemci, url, adresler))
}

fn ip_gercek_loopback_mi(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => v6.is_loopback(),
    }
}

fn kok_sertifikalar() -> rustls::RootCertStore {
    rustls::RootCertStore::empty()
}

struct GidenHttpIleti {
    ileti: ClientJsonRpcMessage,
    yazildi: Option<oneshot::Sender<()>>,
}

/// Uzak MCP için sınırlı Streamable HTTP taşıması (`1.1b`).
pub struct SinirliHttpTasima {
    pub yenileme: Arc<YenilemeDurumu>,
    pub oturum: UzakOturumDurumu,
    gelen: mpsc::Receiver<ServerJsonRpcMessage>,
    giden: mpsc::Sender<GidenHttpIleti>,
    hata: watch::Receiver<Option<TasimaHatasi>>,
    hata_gonder: watch::Sender<Option<TasimaHatasi>>,
    kimlikler: Arc<Mutex<AbonelikKimlikleri>>,
    ucustaki_arac_cagrilari: Arc<Mutex<HashSet<RequestId>>>,
    kontrol: TasimaKontrol,
    gorev: Option<JoinHandle<()>>,
}

impl SinirliHttpTasima {
    pub fn yeni(secenekler: UzakHttpSecenekleri) -> (Self, watch::Receiver<Option<TasimaHatasi>>) {
        let (gelen_gonder, gelen) = mpsc::channel(KUYRUK);
        let (giden, mut giden_al) = mpsc::channel::<GidenHttpIleti>(KUYRUK);
        let (hata_gonder, hata) = watch::channel(None);
        let (kapat_gonder, kapat_al) = watch::channel(false);
        let kimlikler = Arc::new(Mutex::new(AbonelikKimlikleri::yeni()));
        let ucustaki_arac_cagrilari = Arc::new(Mutex::new(HashSet::new()));
        let yenileme = Arc::new(YenilemeDurumu::yeni());
        let oturum = UzakOturumDurumu::yeni();
        let kontrol = TasimaKontrol::yeni(kapat_gonder);

        let isci_hata = hata_gonder.clone();
        let isci_kimlikler = kimlikler.clone();
        let isci_ucustaki = ucustaki_arac_cagrilari.clone();
        let isci_yenileme = yenileme.clone();
        let isci_oturum = oturum.clone();
        let mut isci_kapat = kapat_al;

        let gorev = tokio::spawn(async move {
            while let Some(GidenHttpIleti { ileti, yazildi }) = tokio::select! {
                biased;
                _ = kapat_bekle(&mut isci_kapat) => None,
                gelen_istek = giden_al.recv() => gelen_istek,
            } {
                let gonderim_sonucu = http_iletisini_gonder_ve_isle(
                    &secenekler,
                    ileti,
                    &gelen_gonder,
                    &isci_kimlikler,
                    &isci_ucustaki,
                    &isci_yenileme,
                    &isci_oturum,
                    &mut isci_kapat,
                )
                .await;

                if let Some(onay) = yazildi {
                    let _ = onay.send(());
                }

                if let Err(hata_turu) = gonderim_sonucu {
                    ilk_hatayi_yaz(&isci_hata, hata_turu);
                    break;
                }
            }
        });

        (
            Self {
                yenileme,
                oturum,
                gelen,
                giden,
                hata: hata.clone(),
                hata_gonder,
                kimlikler,
                ucustaki_arac_cagrilari,
                kontrol,
                gorev: Some(gorev),
            },
            hata,
        )
    }

    pub fn kontrol(&self) -> TasimaKontrol {
        self.kontrol.clone()
    }

    pub fn oturum_durumu(&self) -> UzakOturumDurumu {
        self.oturum.clone()
    }
}

impl Transport<RoleClient> for SinirliHttpTasima {
    type Error = io::Error;

    fn send(
        &mut self,
        item: ClientJsonRpcMessage,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        let giden = self.giden.clone();
        let hata = self.hata.clone();
        let hata_gonder = self.hata_gonder.clone();
        let kimlikler = self.kimlikler.clone();
        let ucustaki = self.ucustaki_arac_cagrilari.clone();
        let oturum = self.oturum.clone();
        let cerceve_siniri = CERCEVE;

        async move {
            if let Some(mevcut_hata) = *hata.borrow() {
                return Err(io::Error::other(mevcut_hata));
            }
            let govde = serde_json::to_vec(&item).map_err(|_| {
                ilk_hatayi_yaz(&hata_gonder, TasimaHatasi::BozukJson);
                io::Error::new(io::ErrorKind::InvalidData, TasimaHatasi::BozukJson)
            })?;
            if govde.len() > cerceve_siniri {
                ilk_hatayi_yaz(&hata_gonder, TasimaHatasi::Cerceve);
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    TasimaHatasi::Cerceve,
                ));
            }

            // Eğer uçuşta olan bir `tools/call` için `notifications/cancelled` gönderiliyorsa
            // uzaktaki yan etkinin durduğu kanıtlanamayacağı için çağrıyı belirsiz olarak işaretle.
            let iptal_bildirimi_mi = matches!(
                &item,
                ClientJsonRpcMessage::Notification(b)
                    if matches!(
                        &b.notification,
                        ClientNotification::CancelledNotification(_)
                    )
            );
            if let ClientJsonRpcMessage::Notification(ref b) = item
                && let ClientNotification::CancelledNotification(ref iptal) = b.notification
                && let Some(ref hedef_id) = iptal.params.request_id
                && ucustaki
                    .lock()
                    .map(|u| u.contains(hedef_id))
                    .unwrap_or(false)
            {
                oturum.belirsiz_cagri_ekle(
                    hedef_id.clone(),
                    BelirsizCagriNedeni::IptalGonderildiSonucBilinmiyor,
                );
            }

            {
                let mut k = kimlikler
                    .lock()
                    .map_err(|_| io::Error::other(TasimaHatasi::GirdiCikti))?;
                if let Err(hata_turu) = k.giden_istegi_ekle(&item) {
                    ilk_hatayi_yaz(&hata_gonder, hata_turu);
                    return Err(io::Error::other(hata_turu));
                }
            }

            let (yazildi_gonder, yazildi_al) = if iptal_bildirimi_mi {
                let (tx, rx) = oneshot::channel();
                (Some(tx), Some(rx))
            } else {
                (None, None)
            };

            giden
                .try_send(GidenHttpIleti {
                    ileti: item,
                    yazildi: yazildi_gonder,
                })
                .map_err(|err| match err {
                    mpsc::error::TrySendError::Full(_) => {
                        ilk_hatayi_yaz(&hata_gonder, TasimaHatasi::GidenIletiSeli);
                        io::Error::other(TasimaHatasi::GidenIletiSeli)
                    }
                    mpsc::error::TrySendError::Closed(_) => {
                        ilk_hatayi_yaz(&hata_gonder, TasimaHatasi::UzakKapandi);
                        io::Error::new(io::ErrorKind::BrokenPipe, TasimaHatasi::UzakKapandi)
                    }
                })?;

            if let Some(rx) = yazildi_al {
                let _ = tokio::time::timeout(Duration::from_millis(250), rx).await;
            }
            Ok(())
        }
    }

    fn receive(&mut self) -> impl Future<Output = Option<ServerJsonRpcMessage>> + Send {
        self.gelen.recv()
    }

    fn close(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.kontrol.kapat();
        let gorev = self.gorev.take();
        async move {
            if let Some(g) = gorev {
                let _ = g.await;
            }
            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn http_iletisini_gonder_ve_isle(
    secenekler: &UzakHttpSecenekleri,
    ileti: ClientJsonRpcMessage,
    gelen_gonder: &mpsc::Sender<ServerJsonRpcMessage>,
    kimlikler: &Arc<Mutex<AbonelikKimlikleri>>,
    ucustaki_arac_cagrilari: &Arc<Mutex<HashSet<RequestId>>>,
    yenileme: &Arc<YenilemeDurumu>,
    oturum: &UzakOturumDurumu,
    kapat: &mut watch::Receiver<bool>,
) -> Result<(), TasimaHatasi> {
    let mut istek_kimligi: Option<RequestId> = None;
    let mut istek_metodu: Option<String> = None;
    let mut meta_protokol: Option<String> = None;
    let mut cagri_arac_adi: Option<String> = None;

    match ileti {
        ClientJsonRpcMessage::Request(ref istek) => {
            istek_kimligi = Some(istek.id.clone());
            istek_metodu = Some(istek.request.method().to_owned());
            meta_protokol = istek
                .request
                .get_meta()
                .protocol_version()
                .map(|p| p.as_str().to_owned());
            if let rmcp::model::ClientRequest::CallToolRequest(ref c) = istek.request {
                cagri_arac_adi = Some(c.params.name.to_string());
            }
        }
        ClientJsonRpcMessage::Notification(ref bildirim)
            if matches!(
                bildirim.notification,
                ClientNotification::CancelledNotification(_)
            ) && oturum.anlasilan_protokol().as_deref() == Some("2026-07-28") =>
        {
            return Ok(());
        }
        _ => {}
    }

    let arac_cagrisi_id = if istek_metodu.as_deref() == Some("tools/call") {
        istek_kimligi.clone()
    } else {
        None
    };

    if let Some(ref id) = arac_cagrisi_id {
        if let Ok(mut u) = ucustaki_arac_cagrilari.lock() {
            u.insert(id.clone());
        }
        oturum.arac_cagrisi_gonderildi();
    }

    let sonuc = http_post_ve_yanit_akisi(
        secenekler,
        &ileti,
        istek_kimligi.as_ref(),
        istek_metodu.as_deref(),
        cagri_arac_adi.as_deref(),
        meta_protokol.as_deref(),
        arac_cagrisi_id.as_ref(),
        gelen_gonder,
        kimlikler,
        yenileme,
        oturum,
        kapat,
    )
    .await;

    if let Some(ref id) = arac_cagrisi_id
        && let Ok(mut u) = ucustaki_arac_cagrilari.lock()
    {
        u.remove(id);
    }

    sonuc
}

#[allow(clippy::too_many_arguments)]
async fn http_post_ve_yanit_akisi(
    secenekler: &UzakHttpSecenekleri,
    ileti: &ClientJsonRpcMessage,
    istek_kimligi: Option<&RequestId>,
    istek_metodu: Option<&str>,
    cagri_arac_adi: Option<&str>,
    meta_protokol: Option<&str>,
    arac_cagrisi_id: Option<&RequestId>,
    gelen_gonder: &mpsc::Sender<ServerJsonRpcMessage>,
    kimlikler: &Arc<Mutex<AbonelikKimlikleri>>,
    yenileme: &Arc<YenilemeDurumu>,
    oturum: &UzakOturumDurumu,
    kapat: &mut watch::Receiver<bool>,
) -> Result<(), TasimaHatasi> {
    let govde = serde_json::to_vec(ileti).map_err(|_| TasimaHatasi::BozukJson)?;
    if govde.len() > secenekler.cerceve_siniri {
        return Err(TasimaHatasi::Cerceve);
    }

    let baslangic_url =
        Url::parse(&secenekler.uc_url).map_err(|_| TasimaHatasi::AgPolitikasiReddi)?;
    let baslangic_origin = baslangic_url.origin().ascii_serialization();
    let mut mevcut_url_metni = secenekler.uc_url.clone();

    let mut bulunana_yanit = None;
    for adim in 0..=AZAMI_YONLENDIRME {
        let (istemci, dogrulanmis_url, _) = dogrulanmis_istemci_ve_url_hazirla(
            &mevcut_url_metni,
            secenekler.test_fiksturu_loopback,
            secenekler.ozel_ag_izinli,
            secenekler.istek_zamani_asimi,
            secenekler.dns_cozucu.as_ref(),
        )
        .await?;

        let ayni_origin = dogrulanmis_url.origin().ascii_serialization() == baslangic_origin;
        if !ayni_origin && (secenekler.gizli_baslik.is_some() || oturum.mcp_session_id().is_some())
        {
            return Err(TasimaHatasi::AgPolitikasiReddi);
        }

        let mut basliklar = HeaderMap::new();
        basliklar.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        basliklar.insert(
            ACCEPT,
            HeaderValue::from_static("application/json, text/event-stream"),
        );

        let gonderilecek_protokol = meta_protokol
            .map(str::to_owned)
            .or_else(|| oturum.anlasilan_protokol());
        if istek_metodu != Some("initialize")
            && let Some(ref surum) = gonderilecek_protokol
            && let Ok(hv) = HeaderValue::from_str(surum)
        {
            basliklar.insert(HeaderName::from_static("mcp-protocol-version"), hv);
            if surum == "2026-07-28"
                && let Some(metod) = istek_metodu
                && let Ok(mhv) = HeaderValue::from_str(metod)
            {
                basliklar.insert(HeaderName::from_static("mcp-method"), mhv);
                if let Some(arac_adi) = cagri_arac_adi
                    && let Ok(nhv) = HeaderValue::from_str(arac_adi)
                {
                    basliklar.insert(HeaderName::from_static("mcp-name"), nhv);
                }
            }
        }

        if ayni_origin
            && istek_metodu != Some("server/discover")
            && istek_metodu != Some("initialize")
            && let Some(ref sid) = oturum.mcp_session_id()
            && let Ok(hv) = HeaderValue::from_str(sid)
        {
            basliklar.insert(HeaderName::from_static("mcp-session-id"), hv);
        }
        if ayni_origin && let Some(ref gizli) = secenekler.gizli_baslik {
            basliklar.insert(gizli.baslik_adi.clone(), gizli.baslik_degeri.clone());
        }

        let gonderim = istemci
            .post(dogrulanmis_url.clone())
            .headers(basliklar)
            .body(govde.clone())
            .send();

        let yanit = tokio::select! {
            biased;
            _ = kapat_bekle(kapat) => {
                if let Some(id) = arac_cagrisi_id {
                    oturum.belirsiz_cagri_ekle(
                        id.clone(),
                        BelirsizCagriNedeni::AkisKesildi,
                    );
                    return Err(TasimaHatasi::BelirsizCagriSonucu);
                }
                return Err(TasimaHatasi::UzakKapandi);
            }
            sonuc = gonderim => match sonuc {
                Ok(y) => y,
                Err(_) => {
                    if let Some(id) = arac_cagrisi_id {
                        oturum.belirsiz_cagri_ekle(
                            id.clone(),
                            BelirsizCagriNedeni::AkisKesildi,
                        );
                        return Err(TasimaHatasi::BelirsizCagriSonucu);
                    }
                    return Err(TasimaHatasi::GirdiCikti);
                }
            }
        };

        let durum = yanit.status();
        if durum.is_redirection() {
            // `tools/call` için yönlendirme peşinden tekrar POST yapılmaz; ancak
            // başlangıç el sıkışması veya salt-okunur katalog keşfi güvenli yönlendirmeyi
            // sınırlar dahilinde izleyebilir.
            if arac_cagrisi_id.is_some() || adim == AZAMI_YONLENDIRME {
                return Err(TasimaHatasi::AgPolitikasiReddi);
            }
            let Some(konum) = yanit.headers().get(LOCATION).and_then(|v| v.to_str().ok()) else {
                return Err(TasimaHatasi::AgPolitikasiReddi);
            };
            let yeni_url = dogrulanmis_url
                .join(konum)
                .map_err(|_| TasimaHatasi::AgPolitikasiReddi)?;
            if dogrulanmis_url.scheme() == "https" && yeni_url.scheme() != "https" {
                return Err(TasimaHatasi::AgPolitikasiReddi);
            }
            mevcut_url_metni = yeni_url.into();
            continue;
        }

        bulunana_yanit = Some(yanit);
        break;
    }
    let Some(yanit) = bulunana_yanit else {
        return Err(TasimaHatasi::AgPolitikasiReddi);
    };

    let durum_kodu = yanit.status().as_u16();
    if durum_kodu == 401 || durum_kodu == 403 {
        let www_auth = yanit
            .headers()
            .get(WWW_AUTHENTICATE)
            .and_then(|v| v.to_str().ok());
        let maskeler = secenekler.tum_maskeler();
        oturum.yetki_reddi_kaydet(durum_kodu, www_auth, &maskeler);
        return Err(TasimaHatasi::YetkiGerekli);
    }

    if let Some(sid) = yanit
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
    {
        let _ = oturum.mcp_session_id_kaydet(sid);
    }

    // Legacy (`2025-11-25` vb.) sunucular `server/discover` metodunu tanımayıp
    // HTTP 4xx döndürdüğünde rmcp'nin `ClientLifecycleMode::Auto` mekanizmasının
    // `initialize` akışına düşebilmesi için, ayrıca durumsuz 2026 HTTP sunucuları
    // `subscriptions/listen` isteğini reddettiğinde yoklama kipine düşülebilmesi için
    // ilişkili `METHOD_NOT_FOUND` yanıtı üretilir.
    if !yanit.status().is_success() {
        if matches!(
            istek_metodu,
            Some("server/discover") | Some("subscriptions/listen")
        ) && let Some(id) = istek_kimligi
        {
            if let Ok(mut k) = kimlikler.lock() {
                k.yanit_alindi(id);
            }
            let geri_dusme = ServerJsonRpcMessage::error(
                ErrorData::new(ErrorCode::METHOD_NOT_FOUND, "metod desteklenmiyor", None),
                Some(id.clone()),
            );
            gelen_gonder
                .send(geri_dusme)
                .await
                .map_err(|_| TasimaHatasi::UzakKapandi)?;
            return Ok(());
        }
        if let Some(id) = arac_cagrisi_id {
            oturum.belirsiz_cagri_ekle(id.clone(), BelirsizCagriNedeni::AkisKesildi);
            return Err(TasimaHatasi::BelirsizCagriSonucu);
        }
        return Err(TasimaHatasi::GirdiCikti);
    }

    if durum_kodu == 202 || durum_kodu == 204 {
        if let Some(id) = arac_cagrisi_id {
            oturum.belirsiz_cagri_ekle(id.clone(), BelirsizCagriNedeni::AkisKesildi);
            return Err(TasimaHatasi::BelirsizCagriSonucu);
        }
        return Ok(());
    }

    let icerik_turu = yanit
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut beklenen_yanit_alindi = istek_kimligi.is_none();

    if icerik_turu.starts_with("text/event-stream") {
        let mut akis = yanit.bytes_stream();
        let mut sse = SseAkisAyristiricisi::yeni(secenekler.cerceve_siniri);
        loop {
            let parca = tokio::select! {
                biased;
                _ = kapat_bekle(kapat) => {
                    if let Some(id) = arac_cagrisi_id
                        && !beklenen_yanit_alindi
                    {
                        oturum.belirsiz_cagri_ekle(id.clone(), BelirsizCagriNedeni::AkisKesildi);
                        return Err(TasimaHatasi::BelirsizCagriSonucu);
                    }
                    return Err(TasimaHatasi::UzakKapandi);
                }
                sonraki = akis.next() => sonraki,
            };
            match parca {
                Some(Ok(baytlar)) => {
                    let olaylar = match sse.parca_besle(&baytlar) {
                        Ok(o) => o,
                        Err(hata) => {
                            if let Some(id) = arac_cagrisi_id
                                && !beklenen_yanit_alindi
                            {
                                oturum.belirsiz_cagri_ekle(
                                    id.clone(),
                                    BelirsizCagriNedeni::AkisKesildi,
                                );
                            }
                            return Err(hata);
                        }
                    };
                    for olay in olaylar {
                        if (olay.olay_turu == "message" || olay.olay_turu.is_empty())
                            && ham_json_iletisini_isle(
                                olay.veri.as_bytes(),
                                istek_kimligi,
                                gelen_gonder,
                                kimlikler,
                                yenileme,
                                oturum,
                            )
                            .await?
                        {
                            beklenen_yanit_alindi = true;
                        }
                    }
                }
                Some(Err(_)) => {
                    if let Some(id) = arac_cagrisi_id
                        && !beklenen_yanit_alindi
                    {
                        oturum.belirsiz_cagri_ekle(id.clone(), BelirsizCagriNedeni::AkisKesildi);
                        return Err(TasimaHatasi::BelirsizCagriSonucu);
                    }
                    return Err(TasimaHatasi::YarimSatir);
                }
                None => {
                    if let Err(hata) = sse.bitir() {
                        if let Some(id) = arac_cagrisi_id
                            && !beklenen_yanit_alindi
                        {
                            oturum
                                .belirsiz_cagri_ekle(id.clone(), BelirsizCagriNedeni::AkisKesildi);
                            return Err(TasimaHatasi::BelirsizCagriSonucu);
                        }
                        return Err(hata);
                    }
                    break;
                }
            }
        }
    } else {
        let mut akis = yanit.bytes_stream();
        let mut govde_tamponu = Vec::new();
        loop {
            let parca = tokio::select! {
                biased;
                _ = kapat_bekle(kapat) => {
                    if let Some(id) = arac_cagrisi_id {
                        oturum.belirsiz_cagri_ekle(id.clone(), BelirsizCagriNedeni::AkisKesildi);
                        return Err(TasimaHatasi::BelirsizCagriSonucu);
                    }
                    return Err(TasimaHatasi::UzakKapandi);
                }
                sonraki = akis.next() => sonraki,
            };
            match parca {
                Some(Ok(baytlar)) => {
                    if govde_tamponu
                        .len()
                        .checked_add(baytlar.len())
                        .is_none_or(|toplam| toplam > secenekler.cerceve_siniri)
                    {
                        return Err(TasimaHatasi::Cerceve);
                    }
                    govde_tamponu.extend_from_slice(&baytlar);
                }
                Some(Err(_)) => {
                    if let Some(id) = arac_cagrisi_id {
                        oturum.belirsiz_cagri_ekle(id.clone(), BelirsizCagriNedeni::AkisKesildi);
                        return Err(TasimaHatasi::BelirsizCagriSonucu);
                    }
                    return Err(TasimaHatasi::YarimSatir);
                }
                None => break,
            }
        }

        if govde_tamponu.iter().all(u8::is_ascii_whitespace) {
            if istek_kimligi.is_none() {
                return Ok(());
            }
            if let Some(id) = arac_cagrisi_id {
                oturum.belirsiz_cagri_ekle(id.clone(), BelirsizCagriNedeni::AkisKesildi);
                return Err(TasimaHatasi::BelirsizCagriSonucu);
            }
            return Err(TasimaHatasi::YarimSatir);
        }

        if ham_json_iletisini_isle(
            &govde_tamponu,
            istek_kimligi,
            gelen_gonder,
            kimlikler,
            yenileme,
            oturum,
        )
        .await?
        {
            beklenen_yanit_alindi = true;
        }
    }

    if !beklenen_yanit_alindi {
        if let Some(id) = arac_cagrisi_id {
            oturum.belirsiz_cagri_ekle(id.clone(), BelirsizCagriNedeni::AkisKesildi);
            return Err(TasimaHatasi::BelirsizCagriSonucu);
        }
        return Err(TasimaHatasi::YarimSatir);
    }

    Ok(())
}

async fn ham_json_iletisini_isle(
    ham: &[u8],
    beklenen_id: Option<&RequestId>,
    gelen_gonder: &mpsc::Sender<ServerJsonRpcMessage>,
    kimlikler: &Arc<Mutex<AbonelikKimlikleri>>,
    yenileme: &Arc<YenilemeDurumu>,
    oturum: &UzakOturumDurumu,
) -> Result<bool, TasimaHatasi> {
    let mut ham_json: Value = serde_json::from_slice(ham).map_err(|_| TasimaHatasi::BozukJson)?;
    if let Ok(k) = kimlikler.lock() {
        let _ = arac_cagrisi_yanitina_isaret_ekle(&mut ham_json, &k);
    }
    let mut ileti: ServerJsonRpcMessage =
        serde_json::from_value(ham_json).map_err(|_| TasimaHatasi::BozukJson)?;

    match &mut ileti {
        ServerJsonRpcMessage::Response(yanit) => {
            if let ServerResult::InitializeResult(ref init) = yanit.result {
                oturum.anlasilan_protokol_kaydet(init.protocol_version.as_str());
            } else if let ServerResult::DiscoverResult(ref disc) = yanit.result
                && let Some(surum) = disc.supported_versions.first()
            {
                oturum.anlasilan_protokol_kaydet(surum.as_str());
            }
            let eslesti = beklenen_id.is_some_and(|b| b == &yanit.id);
            let biliniyor = kimlikler
                .lock()
                .map(|mut k| k.yanit_alindi(&yanit.id))
                .unwrap_or(false);
            if !biliniyor && !eslesti {
                return Err(TasimaHatasi::BozukJson);
            }
            gelen_gonder
                .try_send(ileti)
                .map_err(|_| TasimaHatasi::IletiSeli)?;
            Ok(eslesti)
        }
        ServerJsonRpcMessage::Error(hata) => {
            let eslesti = beklenen_id.is_some_and(|b| hata.id.as_ref() == Some(b));
            if let Some(ref hata_id) = hata.id {
                let _ = kimlikler.lock().map(|mut k| k.yanit_alindi(hata_id));
            }
            gelen_gonder
                .try_send(ileti)
                .map_err(|_| TasimaHatasi::IletiSeli)?;
            Ok(eslesti)
        }
        ServerJsonRpcMessage::Notification(bildirim) => match &mut bildirim.notification {
            ServerNotification::ToolListChangedNotification(_) => {
                yenileme.degisim_bildirimi();
                Ok(false)
            }
            ServerNotification::SubscriptionsAcknowledgedNotification(_) => {
                let abonelik_id = bildirim.notification.get_meta().subscription_id();
                {
                    let mut k = kimlikler.lock().map_err(|_| TasimaHatasi::GirdiCikti)?;
                    k.abonelik_onayini_dogrula(abonelik_id)?;
                }
                gelen_gonder
                    .try_send(ileti)
                    .map_err(|_| TasimaHatasi::IletiSeli)?;
                Ok(false)
            }
            ServerNotification::CancelledNotification(iptal) => {
                iptal.params.reason = None;
                let abonelik_iptali = kimlikler
                    .lock()
                    .map(|mut k| k.abonelik_iptalini_dogrula(iptal.params.request_id.as_ref()))
                    .unwrap_or(false);
                if abonelik_iptali {
                    yenileme.abonelik_sonlandi();
                    gelen_gonder
                        .try_send(ileti)
                        .map_err(|_| TasimaHatasi::IletiSeli)?;
                }
                Ok(false)
            }
            _ => Ok(false),
        },
        ServerJsonRpcMessage::Request(_) => {
            // İlk teslimat yalnız `tools` kapsamındadır; uzak sunucudan gelen
            // sampling / elicitation / roots istekleri reddedilir.
            Err(TasimaHatasi::BozukJson)
        }
    }
}

fn ilk_hatayi_yaz(hedef: &watch::Sender<Option<TasimaHatasi>>, hata: TasimaHatasi) {
    hedef.send_if_modified(|mevcut| {
        if mevcut.is_none() {
            *mevcut = Some(hata);
            true
        } else {
            false
        }
    });
}

async fn kapat_bekle(kapat: &mut watch::Receiver<bool>) {
    while !*kapat.borrow_and_update() {
        if kapat.changed().await.is_err() {
            return;
        }
    }
}
