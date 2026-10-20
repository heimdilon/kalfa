//! `web_getir` ağ katmanı: URL doğrulama, RFC 6890 / özel ağ / metadata IP
//! denetimi, DNS sabitleme (pinning), sistem vekili (proxy) kapatma,
//! yönlendirme sınırları ve akışlı gövde okuma.

use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use kalfa_sandbox::vekil::adres_izinli_mi;
pub use kalfa_sandbox::vekil::{Cozucu, SistemCozucu};
use reqwest::header::{
    ACCEPT, ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, LOCATION, USER_AGENT,
};
use thiserror::Error;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use url::{Host, Url};

use crate::web::izin::YonlendirmeIzni;
use crate::web::kaynak::{url_maskele, url_nesnesi_maskele};
use crate::web::sikistirma::{
    AZAMI_ACILMIS_GOVDE_BAYT, AkisAcici, SikistirmaHatasi, sikistirma_bicimi_bul,
};

/// En fazla takip edilecek HTTP yönlendirme (redirect) adımı.
pub const AZAMI_YONLENDIRME: usize = 5;

/// Tüm yönlendirmeler ve gövde okuması dahil toplam zaman aşımı (30 saniye).
pub const TOPLAM_ZAMAN_ASIMI: Duration = Duration::from_secs(30);

/// Tek bir TCP/TLS bağlantısı kurma zaman aşımı (10 saniye).
pub const BAGLANTI_ZAMAN_ASIMI: Duration = Duration::from_secs(10);

/// Kalfa web istemcisi User-Agent bilgisi.
pub const KALFA_USER_AGENT: &str = "Kalfa-WebGetir/0.1";

/// Bir IP adresinin reddedilme gerekçesi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdresRedGerekcesi {
    /// `127.0.0.0/8` veya `::1`.
    Loopback,
    /// `0.0.0.0/8` veya `::`.
    BelirsizAdres,
    /// Bulut metadata servisi (`169.254.169.254`, `fd00:ec2::254`, `100.100.100.200`).
    BulutMetadata,
    /// Link-local adres (`169.254.0.0/16`, `fe80::/10`).
    LinkLocal,
    /// Özel ağ (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `fc00::/7`).
    OzelAg,
    /// Paylaşılan CGNAT alanı (`100.64.0.0/10`, RFC 6598).
    PaylasilanAg,
    /// Çoklu yayın (multicast) veya broadcast (`224.0.0.0/4`, `255.255.255.255`, `ff00::/8`).
    CokluYayinVeyaBroadcast,
    /// Belgeleme, kıyaslama veya ayrılmış IETF aralığı.
    AyrilmisAralik,
    /// IPv6 içine gömülü (IPv4-mapped, NAT64, 6to4, Teredo) yasaklı IPv4 adresi.
    GomuluIpv4Yasakli,
}

impl std::fmt::Display for AdresRedGerekcesi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let metin = match self {
            Self::Loopback => "loopback adresi (127.0.0.0/8 veya ::1)",
            Self::BelirsizAdres => "belirsiz adres (0.0.0.0/8 veya ::)",
            Self::BulutMetadata => "bulut metadata adresi (169.254.169.254 / fd00:ec2::254)",
            Self::LinkLocal => "link-local adres (169.254.0.0/16 veya fe80::/10)",
            Self::OzelAg => "özel ağ adresi (RFC 1918 / RFC 4193 ULA)",
            Self::PaylasilanAg => "paylaşılan CGNAT ağı (100.64.0.0/10)",
            Self::CokluYayinVeyaBroadcast => "çoklu yayın (multicast) veya broadcast adresi",
            Self::AyrilmisAralik => "ayrılmış veya belgeleme amaçlı IP aralığı",
            Self::GomuluIpv4Yasakli => "IPv6 içine gömülü yasaklı IPv4 adresi",
        };
        f.write_str(metin)
    }
}

/// Ağ ve HTTP getirme katmanı hataları.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WebGetirHatasi {
    #[error("geçersiz URL `{url}`: {neden}")]
    GecersizUrl { url: String, neden: String },

    #[error("yalnızca http ve https şemaları desteklenir, verilen: `{sema}`")]
    GecersizSema { sema: String },

    #[error("URL içinde kullanıcı adı veya parola bilgisine izin verilmez")]
    UrlKimlikBilgisiYasak,

    #[error("`{host}` için DNS kaydı çözümlenemedi: {neden}")]
    DnsCozulemedi { host: String, neden: String },

    #[error("`{host}` hedefi yasaklı IP adresine ({ip}) çözümlendi: {gerekce}")]
    YasakliAdres {
        host: String,
        ip: IpAddr,
        gerekce: AdresRedGerekcesi,
    },

    #[error("gerçek bağlanılan soket adresi ({baglanan}) doğrulanmış DNS kümesinde değil")]
    BaglananSoketUyusmazligi { baglanan: SocketAddr },

    #[error("HTTPS adresinden şifresiz HTTP adresine yönlendirme engellendi: `{hedef_url}`")]
    HttpsHttpDususu { hedef_url: String },

    #[error(
        "`{kaynak_host}` adresinden onaylanmamış `{hedef_host}` hostuna yönlendirme engellendi"
    )]
    FarkliHostYonlendirmesi {
        kaynak_host: String,
        hedef_host: String,
    },

    #[error("azami yönlendirme sayısı ({sinir}) aşıldı")]
    YonlendirmeSiniriAsildi { sinir: usize },

    #[error("yönlendirme yanıtında ({durum_kodu}) geçerli Location başlığı yok")]
    YonlendirmeHedefiYok { durum_kodu: u16 },

    #[error("TLS/sertifika doğrulama veya bağlantı hatası: {0}")]
    TlsVeyaBaglantiHatasi(String),

    #[error("istek zaman aşımına uğradı ({saniye} sn sınırı)")]
    ZamanAsimi { saniye: u64 },

    #[error("istek iptal edildi")]
    IptalEdildi,

    #[error("HTTP hata durumu döndü: {durum_kodu}")]
    HttpDurumHatasi { durum_kodu: u16 },

    #[error("yanıt gövdesi eksik veya aktarım yarıda kesildi: {0}")]
    EksikGovde(String),

    #[error("sıkıştırma/gövde sınırı hatası: {0}")]
    Sikistirma(#[from] SikistirmaHatasi),

    #[error("desteklenmeyen içerik türü (Content-Type): `{0}`")]
    DesteklenmeyenIcerikTuru(String),

    #[error("desteklenmeyen karakter kümesi (charset): `{0}`")]
    DesteklenmeyenKarakterKumesi(String),

    #[error("yanıt ikili (binary) veri içerdiği için reddedildi")]
    IkiliIcerikReddedildi,

    #[error(
        "sayfa içeriği değiştiği için devam imleci geçersizleşti (beklenen özet: {beklenen_ozet}, güncel özet: {yeni_ozet})"
    )]
    KaynakDegisti {
        beklenen_ozet: String,
        yeni_ozet: String,
    },

    #[error("geçersiz devam imleci: `{0}`")]
    GecersizImlec(String),

    #[error("istenilen bölüm (`{0}`) sayfada bulunamadı")]
    BolumBulunamadi(String),
}

/// Verilen IPv4 adresini RFC 6890, özel ağ, loopback ve bulut metadata
/// kurallarına göre denetler.
fn ipv4_denetle(v4: Ipv4Addr) -> Result<(), AdresRedGerekcesi> {
    let o = v4.octets();
    if v4.is_loopback() {
        return Err(AdresRedGerekcesi::Loopback);
    }
    if o[0] == 0 {
        return Err(AdresRedGerekcesi::BelirsizAdres);
    }
    if o == [169, 254, 169, 254] || o == [100, 100, 100, 200] {
        return Err(AdresRedGerekcesi::BulutMetadata);
    }
    if v4.is_link_local() {
        return Err(AdresRedGerekcesi::LinkLocal);
    }
    if v4.is_private() {
        return Err(AdresRedGerekcesi::OzelAg);
    }
    if o[0] == 100 && (o[1] & 0b1100_0000) == 64 {
        return Err(AdresRedGerekcesi::PaylasilanAg);
    }
    if v4.is_multicast() || v4.is_broadcast() {
        return Err(AdresRedGerekcesi::CokluYayinVeyaBroadcast);
    }
    if (o[0] == 192 && o[1] == 0 && (o[2] == 0 || o[2] == 2))
        || (o[0] == 198 && (o[1] == 18 || o[1] == 19))
        || (o[0] == 198 && o[1] == 51 && o[2] == 100)
        || (o[0] == 203 && o[1] == 0 && o[2] == 113)
        || o[0] >= 240
    {
        return Err(AdresRedGerekcesi::AyrilmisAralik);
    }
    if adres_izinli_mi(IpAddr::V4(v4), false).is_err() {
        return Err(AdresRedGerekcesi::AyrilmisAralik);
    }
    Ok(())
}

/// Verilen IPv6 adresini ve olası gömülü IPv4 dönüşümlerini (IPv4-mapped,
/// IPv4-compatible, NAT64, 6to4, Teredo) denetler.
fn ipv6_denetle(v6: Ipv6Addr) -> Result<(), AdresRedGerekcesi> {
    if v6.is_loopback() {
        return Err(AdresRedGerekcesi::Loopback);
    }
    if v6.is_unspecified() {
        return Err(AdresRedGerekcesi::BelirsizAdres);
    }

    let s = v6.segments();
    // AWS IPv6 IMDS: fd00:ec2::254
    if s == [0xfd00, 0x0ec2, 0, 0, 0, 0, 0, 0x0254] {
        return Err(AdresRedGerekcesi::BulutMetadata);
    }

    // IPv4-mapped IPv6 (::ffff:a.b.c.d)
    if let Some(gomulu) = v6.to_ipv4_mapped() {
        return ipv4_denetle(gomulu).map_err(|_| AdresRedGerekcesi::GomuluIpv4Yasakli);
    }
    // IPv4-compatible IPv6 (::a.b.c.d)
    if let Some(gomulu) = v6.to_ipv4() {
        return ipv4_denetle(gomulu).map_err(|_| AdresRedGerekcesi::GomuluIpv4Yasakli);
    }

    // NAT64 (64:ff9b::/96 ve 64:ff9b:1::/48)
    if (s[0] == 0x0064 && s[1] == 0xff9b && s[2] == 0 && s[3] == 0 && s[4] == 0 && s[5] == 0)
        || (s[0] == 0x0064 && s[1] == 0xff9b && s[2] == 1)
    {
        let b6 = s[6].to_be_bytes();
        let b7 = s[7].to_be_bytes();
        let gomulu = Ipv4Addr::new(b6[0], b6[1], b7[0], b7[1]);
        ipv4_denetle(gomulu).map_err(|_| AdresRedGerekcesi::GomuluIpv4Yasakli)?;
    }

    // 6to4 (2002::/16): 2. ve 3. segmentlerde 32-bit IPv4 taşır.
    if s[0] == 0x2002 {
        let b1 = s[1].to_be_bytes();
        let b2 = s[2].to_be_bytes();
        let gomulu = Ipv4Addr::new(b1[0], b1[1], b2[0], b2[1]);
        ipv4_denetle(gomulu).map_err(|_| AdresRedGerekcesi::GomuluIpv4Yasakli)?;
    }

    // Teredo (2001:0000::/32): son 32 bit ters çevrilmiş istemci IPv4 adresidir.
    if s[0] == 0x2001 && s[1] == 0x0000 {
        let b6 = (!s[6]).to_be_bytes();
        let b7 = (!s[7]).to_be_bytes();
        let gomulu = Ipv4Addr::new(b6[0], b6[1], b7[0], b7[1]);
        ipv4_denetle(gomulu).map_err(|_| AdresRedGerekcesi::GomuluIpv4Yasakli)?;
    }

    if v6.is_unicast_link_local() {
        return Err(AdresRedGerekcesi::LinkLocal);
    }
    if v6.is_unique_local() {
        return Err(AdresRedGerekcesi::OzelAg);
    }
    if v6.is_multicast() {
        return Err(AdresRedGerekcesi::CokluYayinVeyaBroadcast);
    }
    if (s[0] == 0x2001 && s[1] == 0x0db8)
        || (s[0] == 0x2001 && s[1] == 0x0002 && s[2] == 0)
        || (s[0] == 0x0100 && s[1] == 0 && s[2] == 0 && s[3] == 0)
    {
        return Err(AdresRedGerekcesi::AyrilmisAralik);
    }
    if adres_izinli_mi(IpAddr::V6(v6), false).is_err() {
        return Err(AdresRedGerekcesi::AyrilmisAralik);
    }
    Ok(())
}

/// Bir IP adresinin `web_getir` tarafından dış ağ hedefi olarak kullanılıp
/// kullanılamayacağını denetler.
pub fn adres_denetle(ip: IpAddr) -> Result<(), AdresRedGerekcesi> {
    match ip {
        IpAddr::V4(v4) => ipv4_denetle(v4),
        IpAddr::V6(v6) => ipv6_denetle(v6),
    }
}

/// Ham URL dizgesini ayrıştırır, şemayı (`http`/`https`), URL içi kimlik
/// bilgilerini ve host varlığını doğrular; `(kanonik_url, host, port)` döndürür.
pub fn url_dogrula(ham_url: &str) -> Result<(Url, String, u16), WebGetirHatasi> {
    let temiz = ham_url.trim();
    if temiz.is_empty() {
        return Err(WebGetirHatasi::GecersizUrl {
            url: String::new(),
            neden: "boş URL".into(),
        });
    }

    let mut url = Url::parse(temiz).map_err(|hata| WebGetirHatasi::GecersizUrl {
        url: url_maskele(temiz),
        neden: hata.to_string(),
    })?;

    let sema = url.scheme().to_ascii_lowercase();
    if sema != "https" && sema != "http" {
        return Err(WebGetirHatasi::GecersizSema { sema });
    }

    if !url.username().is_empty() || url.password().is_some() {
        return Err(WebGetirHatasi::UrlKimlikBilgisiYasak);
    }

    let Some(host_nesnesi) = url.host() else {
        return Err(WebGetirHatasi::GecersizUrl {
            url: url_nesnesi_maskele(&url),
            neden: "host adı eksik".into(),
        });
    };

    let host_dizgesi = match host_nesnesi {
        Host::Domain(d) => {
            let kucuk = d.trim().to_ascii_lowercase();
            if kucuk.is_empty() {
                return Err(WebGetirHatasi::GecersizUrl {
                    url: url_nesnesi_maskele(&url),
                    neden: "boş alan adı".into(),
                });
            }
            kucuk
        }
        Host::Ipv4(v4) => v4.to_string(),
        Host::Ipv6(v6) => format!("[{v6}]"),
    };

    let Some(port) = url.port_or_known_default() else {
        return Err(WebGetirHatasi::GecersizUrl {
            url: url_nesnesi_maskele(&url),
            neden: "geçerli port belirlenemedi".into(),
        });
    };

    url.set_fragment(None);
    Ok((url, host_dizgesi, port))
}

/// Görev 1 kanıtı için: yalnızca URL ayrıştırma + tek seferlik ön DNS bakışı
/// yapan naif kontrolün, gerçek bağlanılan soketi doğrulamaktan neden farklı
/// olduğunu göstermek amacıyla kullanılır.
///
/// Bu fonksiyon:
/// 1. Yalnızca ilk döndürülen DNS kaydına bakar (çoklu A/AAAA kaydında ikincil
///    yasaklı adresi veya bağlantı anındaki DNS rebinding'i kaçırır),
/// 2. Sistem vekilini (`HTTP_PROXY`/`HTTPS_PROXY`) ve yönlendirmeleri denetlemez.
///
/// Üretimde [`WebIstemci`] bu fonksiyonu KULLANMAZ; her adımda tüm DNS
/// kayıtlarını doğrular, istemciye sabitler (`.resolve_to_addrs`), sistem
/// vekilini kapatır (`.no_proxy()`) ve bağlanılan `remote_addr()`'ı denetler.
pub async fn naif_url_on_kontrolu(
    ham_url: &str,
    cozucu: &dyn Cozucu,
) -> Result<(), WebGetirHatasi> {
    let (url, host, port) = url_dogrula(ham_url)?;
    match url.host() {
        Some(Host::Ipv4(v4)) => {
            if adres_izinli_mi(IpAddr::V4(v4), false).is_err() {
                return Err(WebGetirHatasi::YasakliAdres {
                    host,
                    ip: IpAddr::V4(v4),
                    gerekce: AdresRedGerekcesi::Loopback,
                });
            }
        }
        Some(Host::Ipv6(v6)) => {
            if adres_izinli_mi(IpAddr::V6(v6), false).is_err() {
                return Err(WebGetirHatasi::YasakliAdres {
                    host,
                    ip: IpAddr::V6(v6),
                    gerekce: AdresRedGerekcesi::Loopback,
                });
            }
        }
        Some(Host::Domain(alan_adi)) => {
            let adresler =
                cozucu
                    .coz(alan_adi, port)
                    .await
                    .map_err(|h| WebGetirHatasi::DnsCozulemedi {
                        host: host.clone(),
                        neden: h.to_string(),
                    })?;
            let Some(ilk) = adresler.first() else {
                return Err(WebGetirHatasi::DnsCozulemedi {
                    host,
                    neden: "boş adres listesi".into(),
                });
            };
            if adres_izinli_mi(ilk.ip(), false).is_err() {
                return Err(WebGetirHatasi::YasakliAdres {
                    host,
                    ip: ilk.ip(),
                    gerekce: AdresRedGerekcesi::Loopback,
                });
            }
        }
        None => {}
    }
    Ok(())
}

/// Yerel HTTP/HTTPS fixture testlerinde yalnızca açıkça tanımlanmış
/// `(alan_adi, SocketAddr)` çiftlerine ve test kök sertifikalarına izin veren yapı.
///
/// Üretimde `None` olarak kullanılır; testlerde bile `127.0.0.1` IP literaline
/// veya listede olmayan bir alan adına erişim reddedilmeye devam eder.
#[derive(Debug, Clone, Default)]
pub struct TestFixtureIzni {
    izinli_hedefler: HashSet<(String, SocketAddr)>,
    kok_sertifikalar_der: Vec<Vec<u8>>,
    http_izni: bool,
}

impl TestFixtureIzni {
    /// Yeni boş bir test fixture izni oluşturur.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Belirli bir alan adının belirli bir yerel soket adresine bağlanmasına izin verir.
    pub fn hedef_ekle(&mut self, alan_adi: &str, soket: SocketAddr) {
        self.izinli_hedefler
            .insert((alan_adi.trim().to_ascii_lowercase(), soket));
    }

    /// Test TLS sunucusunun kök CA sertifikasını (DER biçiminde) ekler.
    pub fn kok_sertifika_der_ekle(&mut self, der: Vec<u8>) {
        self.kok_sertifikalar_der.push(der);
    }

    /// Yalnızca yerel test fixture'larında şifresiz `http://` bağlantısına izin verir.
    /// Genel ürün yolunda bu bayrak kapalıdır ve yalnız `https://` kabul edilir.
    pub fn http_etkinlestir(&mut self) {
        self.http_izni = true;
    }

    /// Şifresiz `http://` test izninin açık olup olmadığını döndürür.
    pub fn http_izinli_mi(&self) -> bool {
        self.http_izni
    }

    fn hedef_izinli_mi(&self, host: &str, soket: SocketAddr) -> bool {
        self.izinli_hedefler
            .contains(&(host.trim().to_ascii_lowercase(), soket))
    }
}

/// Tek bir yönlendirme adımının kaydı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YonlendirmeKaydi {
    /// Yönlendiren durum kodu (301, 302, 303, 307, 308).
    pub durum_kodu: u16,
    /// Maskelenmiş kaynak URL.
    pub kaynak_url: String,
    /// Maskelenmiş hedef URL.
    pub hedef_url: String,
    /// Hedefin bağlanılan doğrulanmış soket adresi.
    pub hedef_soket: SocketAddr,
}

/// `WebIstemci::getir` tarafından döndürülen doğrulanmış ve açılmış ham yanıt.
#[derive(Debug, Clone)]
pub struct WebHamYanit {
    /// İlk istenen URL (maskelenmiş).
    pub istenen_url: String,
    /// Yönlendirmeler sonundaki son URL (maskelenmiş).
    pub son_url: String,
    /// Son URL nesnesi (bağlantı çözümlemesi için).
    pub son_url_nesnesi: Url,
    /// HTTP durum kodu.
    pub durum_kodu: u16,
    /// Ham `Content-Type` başlığı.
    pub icerik_turu_basligi: Option<String>,
    /// Açılmış (decompressed) yanıt gövdesi baytları (en çok 4 MiB).
    pub govde: Vec<u8>,
    /// Son adımda gerçek bağlanılan soket adresi.
    pub baglanan_adres: SocketAddr,
    /// Takip edilen yönlendirme zinciri.
    pub yonlendirmeler: Vec<YonlendirmeKaydi>,
}

/// Sınırlı, DNS-sabitli ve sistem vekili kapalı HTTP/HTTPS istemcisi.
pub struct WebIstemci {
    cozucu: Arc<dyn Cozucu>,
    test_izni: Option<TestFixtureIzni>,
    toplam_zaman_asimi: Duration,
    baglanti_zaman_asimi: Duration,
    azami_yonlendirme: usize,
    azami_acilmis_govde_bayt: usize,
}

impl std::fmt::Debug for WebIstemci {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebIstemci")
            .field("toplam_zaman_asimi", &self.toplam_zaman_asimi)
            .field("baglanti_zaman_asimi", &self.baglanti_zaman_asimi)
            .field("azami_yonlendirme", &self.azami_yonlendirme)
            .field("azami_acilmis_govde_bayt", &self.azami_acilmis_govde_bayt)
            .finish()
    }
}

impl Default for WebIstemci {
    fn default() -> Self {
        Self::yeni()
    }
}

impl WebIstemci {
    /// Üretim ayarlarıyla (`SistemCozucu`, 30 sn toplam süre, 5 yönlendirme, 4 MiB gövde)
    /// yeni bir `WebIstemci` oluşturur.
    pub fn yeni() -> Self {
        let _ = rustls::crypto::ring::default_provider().install_default();
        Self {
            cozucu: Arc::new(SistemCozucu),
            test_izni: None,
            toplam_zaman_asimi: TOPLAM_ZAMAN_ASIMI,
            baglanti_zaman_asimi: BAGLANTI_ZAMAN_ASIMI,
            azami_yonlendirme: AZAMI_YONLENDIRME,
            azami_acilmis_govde_bayt: AZAMI_ACILMIS_GOVDE_BAYT,
        }
    }

    /// Özel bir DNS çözücü ve isteğe bağlı yerel fixture izniyle istemci kurar.
    pub fn ozel_cozuculu(cozucu: Arc<dyn Cozucu>, test_izni: Option<TestFixtureIzni>) -> Self {
        let _ = rustls::crypto::ring::default_provider().install_default();
        Self {
            cozucu,
            test_izni,
            toplam_zaman_asimi: TOPLAM_ZAMAN_ASIMI,
            baglanti_zaman_asimi: BAGLANTI_ZAMAN_ASIMI,
            azami_yonlendirme: AZAMI_YONLENDIRME,
            azami_acilmis_govde_bayt: AZAMI_ACILMIS_GOVDE_BAYT,
        }
    }

    /// Testlerde toplam zaman aşımını özelleştirmek için kullanılır.
    pub fn toplam_zaman_asimi_ayarla(mut self, sure: Duration) -> Self {
        self.toplam_zaman_asimi = sure;
        self
    }

    /// Testlerde açılmış gövde bayt sınırını özelleştirmek için kullanılır.
    pub fn azami_acilmis_govde_ayarla(mut self, bayt: usize) -> Self {
        self.azami_acilmis_govde_bayt = bayt;
        self
    }

    /// Yalnızca yerel test fixture'ında şifresiz `http://` şemasının açıkça
    /// etkinleştirilip etkinleştirilmediğini döndürür.
    pub fn http_fixture_izni_var_mi(&self) -> bool {
        self.test_izni
            .as_ref()
            .is_some_and(TestFixtureIzni::http_izinli_mi)
    }

    fn soket_denetle(&self, host: &str, soket: SocketAddr) -> Result<(), WebGetirHatasi> {
        if let Some(izin) = &self.test_izni
            && izin.hedef_izinli_mi(host, soket)
        {
            return Ok(());
        }
        adres_denetle(soket.ip()).map_err(|gerekce| WebGetirHatasi::YasakliAdres {
            host: host.to_string(),
            ip: soket.ip(),
            gerekce,
        })
    }

    async fn hedef_soketleri_coz_ve_dogrula(
        &self,
        url: &Url,
        host: &str,
        port: u16,
    ) -> Result<Vec<SocketAddr>, WebGetirHatasi> {
        match url.host() {
            Some(Host::Ipv4(v4)) => {
                let soket = SocketAddr::new(IpAddr::V4(v4), port);
                self.soket_denetle(host, soket)?;
                Ok(vec![soket])
            }
            Some(Host::Ipv6(v6)) => {
                let soket = SocketAddr::new(IpAddr::V6(v6), port);
                self.soket_denetle(host, soket)?;
                Ok(vec![soket])
            }
            Some(Host::Domain(alan_adi)) => {
                let adaylar = self.cozucu.coz(alan_adi, port).await.map_err(|h| {
                    WebGetirHatasi::DnsCozulemedi {
                        host: host.to_string(),
                        neden: h.to_string(),
                    }
                })?;
                if adaylar.is_empty() {
                    return Err(WebGetirHatasi::DnsCozulemedi {
                        host: host.to_string(),
                        neden: "DNS yanıtı boş döndü".into(),
                    });
                }
                for &soket in &adaylar {
                    self.soket_denetle(host, soket)?;
                }
                Ok(adaylar)
            }
            None => Err(WebGetirHatasi::GecersizUrl {
                url: url_nesnesi_maskele(url),
                neden: "host bulunamadı".into(),
            }),
        }
    }

    fn adim_istemcisi_kur(
        &self,
        url: &Url,
        dogrulanmis_soketler: &[SocketAddr],
        kalan_sure: Duration,
    ) -> Result<reqwest::Client, WebGetirHatasi> {
        let mut kurucu = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(self.baglanti_zaman_asimi.min(kalan_sure))
            .timeout(kalan_sure);

        if let Some(Host::Domain(alan_adi)) = url.host() {
            kurucu = kurucu.resolve_to_addrs(alan_adi, dogrulanmis_soketler);
        }

        if let Some(izin) = &self.test_izni
            && !izin.kok_sertifikalar_der.is_empty()
        {
            let mut kok_depo = rustls::RootCertStore::empty();
            for der in &izin.kok_sertifikalar_der {
                let sertifika = rustls::pki_types::CertificateDer::from(der.clone());
                kok_depo.add(sertifika).map_err(|h| {
                    WebGetirHatasi::TlsVeyaBaglantiHatasi(format!(
                        "test kök sertifikası eklenemedi: {h}"
                    ))
                })?;
            }
            let tls_ayari = rustls::ClientConfig::builder()
                .with_root_certificates(kok_depo)
                .with_no_client_auth();
            kurucu = kurucu.use_preconfigured_tls(tls_ayari);
        }

        kurucu
            .build()
            .map_err(|h| WebGetirHatasi::TlsVeyaBaglantiHatasi(h.to_string()))
    }

    /// Verilen URL'yi yönlendirme, DNS sabitleme, IP politikası, zaman aşımı
    /// ve açılmış gövde sınırlarıyla getirir.
    pub async fn getir(
        &self,
        ham_url: &str,
        yonlendirme_izni: &YonlendirmeIzni,
        iptal: CancellationToken,
    ) -> Result<WebHamYanit, WebGetirHatasi> {
        let (ilk_url, ilk_host, _ilk_port) = url_dogrula(ham_url)?;
        let istenen_url_maskeli = url_nesnesi_maskele(&ilk_url);
        let son_tarih = Instant::now() + self.toplam_zaman_asimi;
        let zaman_asimi_sn = self.toplam_zaman_asimi.as_secs().max(1);

        let mut su_anki_url = ilk_url;
        let mut yonlendirmeler = Vec::new();

        loop {
            if iptal.is_cancelled() {
                return Err(WebGetirHatasi::IptalEdildi);
            }

            let simdi = Instant::now();
            if simdi >= son_tarih {
                return Err(WebGetirHatasi::ZamanAsimi {
                    saniye: zaman_asimi_sn,
                });
            }
            let kalan_sure = son_tarih.duration_since(simdi);

            let (kanonik_url, host, port) = url_dogrula(su_anki_url.as_str())?;
            let dogrulanmis_soketler = tokio::select! {
                _ = iptal.cancelled() => return Err(WebGetirHatasi::IptalEdildi),
                sonuc = tokio::time::timeout_at(
                    son_tarih,
                    self.hedef_soketleri_coz_ve_dogrula(&kanonik_url, &host, port),
                ) => {
                    match sonuc {
                        Ok(res) => res?,
                        Err(_) => return Err(WebGetirHatasi::ZamanAsimi { saniye: zaman_asimi_sn }),
                    }
                }
            };
            if kanonik_url.scheme() == "http" && !self.http_fixture_izni_var_mi() {
                return Err(WebGetirHatasi::GecersizSema {
                    sema: "http".into(),
                });
            }
            let istemci =
                self.adim_istemcisi_kur(&kanonik_url, &dogrulanmis_soketler, kalan_sure)?;

            let istek = istemci
                .get(kanonik_url.clone())
                .header(USER_AGENT, KALFA_USER_AGENT)
                .header(
                    ACCEPT,
                    "text/html,application/xhtml+xml,text/plain,text/markdown,application/json,application/xml,text/xml;q=0.9",
                )
                .header(ACCEPT_ENCODING, "gzip, deflate, identity");

            let yanit = tokio::select! {
                _ = iptal.cancelled() => return Err(WebGetirHatasi::IptalEdildi),
                sonuc = tokio::time::timeout_at(son_tarih, istek.send()) => {
                    match sonuc {
                        Ok(Ok(y)) => y,
                        Ok(Err(hata)) => {
                            if hata.is_timeout() {
                                return Err(WebGetirHatasi::ZamanAsimi { saniye: zaman_asimi_sn });
                            }
                            return Err(WebGetirHatasi::TlsVeyaBaglantiHatasi(
                                hata.without_url().to_string(),
                            ));
                        }
                        Err(_) => return Err(WebGetirHatasi::ZamanAsimi { saniye: zaman_asimi_sn }),
                    }
                }
            };

            let baglanan_adres = match yanit.remote_addr() {
                Some(soket) => {
                    self.soket_denetle(&host, soket)?;
                    if !dogrulanmis_soketler.contains(&soket) {
                        return Err(WebGetirHatasi::BaglananSoketUyusmazligi { baglanan: soket });
                    }
                    soket
                }
                None => dogrulanmis_soketler[0],
            };

            let durum = yanit.status();
            let durum_kodu = durum.as_u16();

            if durum.is_redirection() {
                if yonlendirmeler.len() >= self.azami_yonlendirme {
                    return Err(WebGetirHatasi::YonlendirmeSiniriAsildi {
                        sinir: self.azami_yonlendirme,
                    });
                }

                let Some(konum_degeri) = yanit.headers().get(LOCATION) else {
                    return Err(WebGetirHatasi::YonlendirmeHedefiYok { durum_kodu });
                };
                let konum_metni = konum_degeri
                    .to_str()
                    .map_err(|_| WebGetirHatasi::YonlendirmeHedefiYok { durum_kodu })?
                    .trim();
                if konum_metni.is_empty() {
                    return Err(WebGetirHatasi::YonlendirmeHedefiYok { durum_kodu });
                }

                let ham_hedef =
                    kanonik_url
                        .join(konum_metni)
                        .map_err(|h| WebGetirHatasi::GecersizUrl {
                            url: url_maskele(konum_metni),
                            neden: h.to_string(),
                        })?;

                let (hedef_url, hedef_host, _hedef_port) = url_dogrula(ham_hedef.as_str())?;

                if kanonik_url.scheme() == "https" && hedef_url.scheme() == "http" {
                    return Err(WebGetirHatasi::HttpsHttpDususu {
                        hedef_url: url_nesnesi_maskele(&hedef_url),
                    });
                }

                if !yonlendirme_izni.host_izinli_mi(&ilk_host, &hedef_host) {
                    return Err(WebGetirHatasi::FarkliHostYonlendirmesi {
                        kaynak_host: host,
                        hedef_host,
                    });
                }

                yonlendirmeler.push(YonlendirmeKaydi {
                    durum_kodu,
                    kaynak_url: url_nesnesi_maskele(&kanonik_url),
                    hedef_url: url_nesnesi_maskele(&hedef_url),
                    hedef_soket: baglanan_adres,
                });
                su_anki_url = hedef_url;
                continue;
            }

            if !durum.is_success() {
                return Err(WebGetirHatasi::HttpDurumHatasi { durum_kodu });
            }

            let icerik_turu_basligi = yanit
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);

            let kodlama_basligi = yanit
                .headers()
                .get(CONTENT_ENCODING)
                .and_then(|v| v.to_str().ok());
            let sikistirma_bicimi = sikistirma_bicimi_bul(kodlama_basligi)?;

            let beklenen_tel_uzunlugu = match yanit.headers().get(CONTENT_LENGTH) {
                Some(deger) => {
                    let metin = deger.to_str().map_err(|_| {
                        WebGetirHatasi::EksikGovde("geçersiz Content-Length başlığı".into())
                    })?;
                    let uz = metin.trim().parse::<usize>().map_err(|_| {
                        WebGetirHatasi::EksikGovde("sayısal olmayan Content-Length".into())
                    })?;
                    if uz > self.azami_acilmis_govde_bayt {
                        return Err(WebGetirHatasi::Sikistirma(
                            SikistirmaHatasi::AcilmisBoyutAsildi {
                                sinir: self.azami_acilmis_govde_bayt,
                            },
                        ));
                    }
                    Some(uz)
                }
                None => None,
            };

            let mut acici = AkisAcici::yeni(sikistirma_bicimi, self.azami_acilmis_govde_bayt);
            let mut okunan_tel_bayt = 0usize;
            let mut akis = yanit.bytes_stream();

            loop {
                let sonraki_parca = tokio::select! {
                    _ = iptal.cancelled() => return Err(WebGetirHatasi::IptalEdildi),
                    sonuc = tokio::time::timeout_at(son_tarih, akis.next()) => {
                        match sonuc {
                            Ok(parca) => parca,
                            Err(_) => return Err(WebGetirHatasi::ZamanAsimi { saniye: zaman_asimi_sn }),
                        }
                    }
                };

                let Some(parca_sonucu) = sonraki_parca else {
                    break;
                };

                let parca = match parca_sonucu {
                    Ok(p) => p,
                    Err(h) if h.is_timeout() => {
                        return Err(WebGetirHatasi::ZamanAsimi {
                            saniye: zaman_asimi_sn,
                        });
                    }
                    Err(h) => {
                        let temiz_hata = h.without_url();
                        return Err(WebGetirHatasi::EksikGovde(format!(
                            "akış okuma hatası: {temiz_hata}"
                        )));
                    }
                };
                okunan_tel_bayt = okunan_tel_bayt.saturating_add(parca.len());
                acici.besle(&parca)?;
            }

            if let Some(beklenen) = beklenen_tel_uzunlugu
                && okunan_tel_bayt != beklenen
            {
                return Err(WebGetirHatasi::EksikGovde(format!(
                    "Content-Length ({beklenen}) ile okunan bayt ({okunan_tel_bayt}) uyuşmuyor"
                )));
            }

            let govde = acici.bitir()?;

            return Ok(WebHamYanit {
                istenen_url: istenen_url_maskeli,
                son_url: url_nesnesi_maskele(&kanonik_url),
                son_url_nesnesi: kanonik_url,
                durum_kodu,
                icerik_turu_basligi,
                govde,
                baglanan_adres,
                yonlendirmeler,
            });
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn adres_denetle_ozel_loopback_metadata_ve_gomulu_ipv6_reddeder() {
        let yasaklilar = [
            "127.0.0.1",
            "127.255.255.254",
            "0.0.0.0",
            "10.0.0.1",
            "172.16.0.1",
            "172.31.255.254",
            "192.168.1.1",
            "169.254.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "100.100.100.200",
            "224.0.0.1",
            "255.255.255.255",
            "::1",
            "::",
            "fe80::1",
            "fc00::1",
            "fd00:ec2::254",
            "::ffff:127.0.0.1",
            "::ffff:169.254.169.254",
            "::ffff:10.0.0.1",
            "64:ff9b::127.0.0.1",
            "64:ff9b::169.254.169.254",
            "2002:7f00:0001::1",
        ];

        for dizge in yasaklilar {
            let ip: IpAddr = dizge.parse().expect("geçerli IP");
            assert!(adres_denetle(ip).is_err(), "{dizge} adresi reddedilmeliydi");
        }

        let izinliler = [
            "93.184.216.34",
            "1.1.1.1",
            "2606:2800:220:1:248:1893:25c8:1946",
        ];
        for dizge in izinliler {
            let ip: IpAddr = dizge.parse().expect("geçerli genel IP");
            assert_eq!(adres_denetle(ip), Ok(()), "{dizge} adresi izinli olmalıydı");
        }
    }

    #[test]
    fn url_dogrula_sema_ve_kimlik_bilgisini_denetler() {
        assert!(matches!(
            url_dogrula("file:///etc/passwd"),
            Err(WebGetirHatasi::GecersizSema { .. })
        ));
        assert!(matches!(
            url_dogrula("javascript:alert(1)"),
            Err(WebGetirHatasi::GecersizSema { .. })
        ));
        assert!(matches!(
            url_dogrula("https://kullanici:parola@belge.ornek.test/yol"),
            Err(WebGetirHatasi::UrlKimlikBilgisiYasak)
        ));
        let (url, host, port) =
            url_dogrula("https://Belge.Ornek.Test/kullanim#bolum1").expect("geçerli URL");
        assert_eq!(host, "belge.ornek.test");
        assert_eq!(port, 443);
        assert_eq!(url.as_str(), "https://belge.ornek.test/kullanim");
    }
}
