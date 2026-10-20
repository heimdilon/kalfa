//! `web_getir` yerleşik aracı: izin hazırlığı (`Hazirlik`), mühür özeti ve
//! sınırlı HTTP/HTTPS metin okuma yürütmesi.

use std::net::IpAddr;
use std::sync::Arc;

use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, AracOrtami, Hazirlik};
use kalfa_core::protokol::VeriCikisi;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;
use url::Host;

use crate::arguman;
use crate::web::ag::{WebGetirHatasi, WebIstemci, adres_denetle, url_dogrula};
use crate::web::izin::{WebIzinDefteri, WebTarifi, YonlendirmeIzni, web_muhur_ozeti};
use crate::web::kaynak::{dis_icerik_zarfla, url_nesnesi_maskele};
use crate::web::metin::yaniti_metne_donustur;

/// Yerleşik web getirme aracının sabit adı.
pub const WEB_ARAC_ADI: &str = "web_getir";

const WEB_ARAC_ACIKLAMASI: &str = "Verilen http veya https adresindeki metin/HTML içeriğini sınırlı ve salt-okunur biçimde getirir. İkili dosyaları, özel ağları ve yerel adresleri reddeder; çıktıyı güvenilmez dış kaynak olarak işaretler.";

/// Ayrıştırılmış `web_getir` çağrısı argümanları.
#[derive(Debug, Clone)]
struct WebArgumanlari {
    tarif: WebTarifi,
    bolum: Option<String>,
    imlec: Option<String>,
    azami_bayt: Option<usize>,
}

/// Dış web sayfalarını sınırlı, yalnız metin/HTML olarak okuyan ve kaynak
/// bilgisini (`kaynak: "web:..."`, `kalfa_uretimi: false`) koruyan yerleşik araç.
pub struct WebGetirAraci {
    istemci: Arc<WebIstemci>,
    izin_defteri: Arc<WebIzinDefteri>,
}

impl std::fmt::Debug for WebGetirAraci {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebGetirAraci")
            .field("istemci", &self.istemci)
            .field("izin_defteri", &self.izin_defteri)
            .finish()
    }
}

impl Default for WebGetirAraci {
    fn default() -> Self {
        Self::yeni()
    }
}

impl WebGetirAraci {
    /// Üretim `WebIstemci` ve yeni bir oturum `WebIzinDefteri` ile araç kurar.
    pub fn yeni() -> Self {
        Self {
            istemci: Arc::new(WebIstemci::yeni()),
            izin_defteri: Arc::new(WebIzinDefteri::yeni()),
        }
    }

    /// Özel `WebIstemci` ve paylaşılan `WebIzinDefteri` ile araç kurar.
    pub fn ozel(istemci: Arc<WebIstemci>, izin_defteri: Arc<WebIzinDefteri>) -> Self {
        Self {
            istemci,
            izin_defteri,
        }
    }

    /// Oturumun web izin defterine erişim sağlar.
    pub fn izin_defteri(&self) -> &Arc<WebIzinDefteri> {
        &self.izin_defteri
    }

    fn argumanlari_coz(&self, a: &Value) -> Result<WebArgumanlari, AracHatasi> {
        let ham_url = arguman::metin(a, "url")?;
        let bolum = a
            .get("bolum")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let imlec = a
            .get("imlec")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let azami_bayt = arguman::sayi(a, "azami_bayt").map(|n| n as usize);

        let yonlendirme_izni = match a
            .get("izinli_yonlendirme_hostlari")
            .and_then(Value::as_array)
        {
            Some(dizi) => {
                let mut hostlar = Vec::new();
                for oge in dizi {
                    let Some(h) = oge.as_str() else {
                        return Err(AracHatasi::Gecersiz(
                            "`izinli_yonlendirme_hostlari` yalnızca metin öğeleri içermelidir."
                                .into(),
                        ));
                    };
                    let temiz = h.trim().to_ascii_lowercase();
                    if !temiz.is_empty() {
                        hostlar.push(temiz);
                    }
                }
                if hostlar.is_empty() {
                    YonlendirmeIzni::AyniHost
                } else {
                    YonlendirmeIzni::IzinliHostlar(hostlar)
                }
            }
            None => YonlendirmeIzni::AyniHost,
        };

        let (kanonik_url, host, port) =
            url_dogrula(ham_url).map_err(|h| AracHatasi::Gecersiz(h.to_string()))?;

        if kanonik_url.scheme() == "http" && !self.istemci.http_fixture_izni_var_mi() {
            return Err(AracHatasi::Gecersiz(
                WebGetirHatasi::GecersizSema {
                    sema: "http".into(),
                }
                .to_string(),
            ));
        }

        if let Some(sizinti_nedeni) = url_gizli_sizinti_var_mi(&kanonik_url) {
            return Err(AracHatasi::Gecersiz(sizinti_nedeni.to_string()));
        }

        // Doğrudan yasaklı bir IP literali (loopback, özel ağ, link-local,
        // metadata vb.) verilmişse hazırlık aşamasında reddet.
        match kanonik_url.host() {
            Some(Host::Ipv4(v4)) => {
                if let Err(gerekce) = adres_denetle(IpAddr::V4(v4)) {
                    return Err(AracHatasi::Gecersiz(
                        WebGetirHatasi::YasakliAdres {
                            host,
                            ip: IpAddr::V4(v4),
                            gerekce,
                        }
                        .to_string(),
                    ));
                }
            }
            Some(Host::Ipv6(v6)) => {
                if let Err(gerekce) = adres_denetle(IpAddr::V6(v6)) {
                    return Err(AracHatasi::Gecersiz(
                        WebGetirHatasi::YasakliAdres {
                            host,
                            ip: IpAddr::V6(v6),
                            gerekce,
                        }
                        .to_string(),
                    ));
                }
            }
            _ => {}
        }

        let maskeli_url = url_nesnesi_maskele(&kanonik_url);
        let sema = kanonik_url.scheme().to_ascii_lowercase();
        let oturumda_ilk_host = !self.izin_defteri.host_onayli_mi(&host, port);

        let tarif = WebTarifi {
            kanonik_url: kanonik_url.to_string(),
            maskeli_url,
            host,
            port,
            sema,
            yontem: "GET".into(),
            yonlendirme_izni,
            oturumda_ilk_host,
        };

        Ok(WebArgumanlari {
            tarif,
            bolum,
            imlec,
            azami_bayt,
        })
    }
}

/// URL yolu veya sorgu dizgesinde açık gizli anahtar / belirteç sızıntısı
/// (ör. prompt injection ile `.env` anahtarının dış URL'ye eklenmesi) var mı?
fn url_gizli_sizinti_var_mi(url: &url::Url) -> Option<&'static str> {
    let birlesik = match url.query() {
        Some(sorgu) => format!("{}?{}", url.path(), sorgu),
        None => url.path().to_string(),
    };
    let kucuk = birlesik.to_ascii_lowercase();
    if birlesik.contains("evr-") || kucuk.contains("evren_api_key") {
        return Some("URL içinde EVREN API anahtarı veya belirteci saptandı; istek engellendi.");
    }
    if birlesik.contains("sk-proj-")
        || birlesik.contains("sk-live-")
        || birlesik.contains("ghp_")
        || birlesik.contains("github_pat_")
    {
        return Some("URL içinde gizli API belirteci saptandı; istek engellendi.");
    }
    if kucuk.contains("-----begin ") && kucuk.contains("private key") {
        return Some("URL içinde özel anahtar (private key) materyali saptandı; istek engellendi.");
    }
    None
}

#[async_trait]
impl Arac for WebGetirAraci {
    fn ad(&self) -> &str {
        WEB_ARAC_ADI
    }

    fn aciklama(&self) -> &str {
        WEB_ARAC_ACIKLAMASI
    }

    fn parametre_semasi(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "Okunacak http veya https adresi."
                },
                "bolum": {
                    "type": "string",
                    "description": "Yalnızca başlığı veya kimliği eşleşen bölümü okumak için isteğe bağlı filtre."
                },
                "imlec": {
                    "type": "string",
                    "description": "Kırpılmış çıktının devamını okumak için önceki çağrının döndürdüğü web1:... imleci."
                },
                "azami_bayt": {
                    "type": "integer",
                    "minimum": 256,
                    "maximum": 65536,
                    "description": "Model çıktısı için azami UTF-8 bayt sınırı (256..=65536)."
                },
                "izinli_yonlendirme_hostlari": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Farklı bir hosta yönlendirmeye izin verilecek ek host adları."
                }
            },
            "required": ["url"],
            "additionalProperties": false
        })
    }

    /// `Kip::Inceleme` altında dış ağ isteğinin çekirdek tarafından da
    /// sunulmaması ve çalıştırılmaması için `true` döner.
    fn yazar_mi(&self) -> bool {
        true
    }

    async fn hazirla(&self, argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
        let cozulmus = self.argumanlari_coz(argumanlar)?;
        let aciklama = format!(
            "`{}` ({}:{}) web adresini GET ile okumak istiyor.",
            cozulmus.tarif.maskeli_url, cozulmus.tarif.host, cozulmus.tarif.port
        );

        Ok(Hazirlik {
            aciklama,
            veri_cikisi: Some(VeriCikisi::WebSayfasi),
            web: Some(cozulmus.tarif),
            kilitsiz: true,
            ..Default::default()
        })
    }

    async fn calistir(
        &self,
        argumanlar: &Value,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        if iptal.is_cancelled() {
            return Err(AracHatasi::Iptal);
        }

        let cozulmus = self.argumanlari_coz(argumanlar)?;
        let beklenen_ozet = web_muhur_ozeti(&cozulmus.tarif);
        let hazirlik_web = hazirlik.web.as_ref().ok_or(AracHatasi::OnayEskidi)?;
        if web_muhur_ozeti(hazirlik_web) != beklenen_ozet {
            return Err(AracHatasi::OnayEskidi);
        }

        let ham_yanit = match self
            .istemci
            .getir(
                &cozulmus.tarif.kanonik_url,
                &cozulmus.tarif.yonlendirme_izni,
                iptal,
            )
            .await
        {
            Ok(y) => y,
            Err(WebGetirHatasi::IptalEdildi) => return Err(AracHatasi::Iptal),
            Err(hata) => return Err(AracHatasi::GirisCikis(hata.to_string())),
        };

        let islenmis = yaniti_metne_donustur(
            &ham_yanit,
            cozulmus.bolum.as_deref(),
            cozulmus.imlec.as_deref(),
            cozulmus.azami_bayt,
        )
        .map_err(|h| AracHatasi::GirisCikis(h.to_string()))?;

        // Başarılı ve onaylı erişimden sonra hostu oturum defterine kaydet.
        self.izin_defteri
            .host_onayla(&cozulmus.tarif.host, cozulmus.tarif.port);

        let zarfli_icerik = dis_icerik_zarfla(&islenmis.ozet, &islenmis.govde);
        let kaynak_etiketi = format!("web:{}", islenmis.ozet.son_url);

        Ok(AracCiktisi {
            icerik: zarfli_icerik,
            kaynak: kaynak_etiketi,
            guvenlik_uyarilari: Vec::new(),
            degisiklikler: None,
            kalfa_uretimi: false,
            ag: None,
        })
    }

    async fn calistir_ortamli(
        &self,
        argumanlar: &Value,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
        ortam: AracOrtami,
    ) -> Result<AracCiktisi, AracHatasi> {
        let _ = ortam;
        self.calistir(argumanlar, hazirlik, iptal).await
    }
}
