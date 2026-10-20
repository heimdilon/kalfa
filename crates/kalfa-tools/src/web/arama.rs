//! Arama sonucu doğrulama, `web_getir` ile sayfa açma ve kaynaklı yanıt
//! doğrulama akışı (Görev 4 ve Görev 5).
//!
//! Plan kısıtı uyarınca arama uzaktaki/yerel MCP arama sağlayıcısından gelir;
//! ikinci bir yerleşik arama aracı (`Arac`) tanımlanmaz. Bu modül MCP arama
//! çıktısını doğrular, seçilen sonuç URL'lerini `WebIstemci` / `web_getir` ile
//! okur ve üretilen iddiaların gerçekten okunan sayfa metinlerine dayandığını
//! denetler (W01, W03).

use std::net::IpAddr;

use kalfa_core::arac::{AracCiktisi, AracHatasi};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio_util::sync::CancellationToken;
use url::Host;

use crate::web::ag::{WebGetirHatasi, WebIstemci, adres_denetle, url_dogrula};
use crate::web::izin::YonlendirmeIzni;
use crate::web::kaynak::{WebSayfaOzeti, url_nesnesi_maskele};
use crate::web::metin::yaniti_metne_donustur;

/// MCP arama sağlayıcısından dönen tek bir arama sonucu öğesi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AramaSonucOgesi {
    /// Sonuç başlığı.
    #[serde(alias = "title")]
    pub baslik: String,
    /// Ham sonuç URL'si.
    pub url: String,
    /// Kısa özet parçacığı (snippet).
    #[serde(
        default,
        alias = "snippet",
        alias = "description",
        alias = "content",
        alias = "text",
        alias = "summary"
    )]
    pub ozet_parcacigi: String,
}

#[derive(Deserialize)]
struct HamAramaSonucOgesi {
    #[serde(alias = "title")]
    baslik: String,
    url: String,
    #[serde(
        default,
        alias = "snippet",
        alias = "description",
        alias = "content",
        alias = "text",
        alias = "summary"
    )]
    ozet_parcacigi: String,
    #[serde(default, alias = "highlights", alias = "extra_snippets")]
    ek_vurgular: Vec<String>,
}

/// Doğrulanmış ve maskelenmiş arama sonucu öğesi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DogrulanmisAramaSonucu {
    pub baslik: String,
    pub kanonik_url: String,
    pub maskeli_url: String,
    pub host: String,
    pub ozet_parcacigi: String,
}

/// MCP arama sağlayıcısının ham yanıt durumu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AramaSaglayiciYaniti {
    /// Başarılı arama sonuç listesi.
    Basarili(Vec<AramaSonucOgesi>),
    /// Sağlayıcı hız sınırına (HTTP 429) takıldı.
    HizSiniri { yeniden_dene_sn: Option<u64> },
    /// Sağlayıcı kimlik doğrulama veya bakiye/kota hatası (HTTP 401/402/403) döndürdü.
    YetersizBakiyeVeyaKimlik { detay: String },
}

/// Arama ve kaynak okuma akışı hataları.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AramaHatasi {
    #[error("arama işlemi iptal edildi")]
    IptalEdildi,

    #[error("arama sağlayıcısı hız sınırına takıldı (yeniden deneme: {yeniden_dene_sn:?} sn)")]
    HizSiniri { yeniden_dene_sn: Option<u64> },

    #[error("arama sağlayıcısı kimlik veya bakiye hatası: {detay}")]
    YetersizBakiyeVeyaKimlik { detay: String },

    #[error("`{sorgu}` sorgusu için arama sonucu bulunamadı")]
    BosSonuc { sorgu: String },

    #[error("arama sonucunda geçersiz veya yasaklı URL (`{url}`): {neden}")]
    GecersizSonucUrl { url: String, neden: String },

    #[error("arama çıktısı ayrıştırılamadı: {0}")]
    CiktiAyristirma(String),

    #[error("kaynak sayfa okunamadı: {0}")]
    SayfaGetirme(#[from] WebGetirHatasi),
}

/// `web_getir` ile okunmuş ve doğrulanmış bir kaynak sayfa kaydı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OkunanKaynakSayfa {
    /// Arama sonucundaki başlık.
    pub arama_basligi: String,
    /// Arama sonucundaki özet parçacığı.
    pub arama_ozeti: String,
    /// `web_getir` sayfa özeti.
    pub sayfa_ozeti: WebSayfaOzeti,
    /// Sayfadan çıkarılan düz metin.
    pub sayfa_metni: String,
}

/// Kaynaklı yanıt içindeki tek bir iddia ve dayandığı kaynak kanıtı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KaynakliIddia {
    /// Kullanıcıya sunulan teknik iddia veya bulgu.
    pub iddia: String,
    /// İddianın dayandığı okunan sayfa URL'si (`son_url` veya `istenen_url`).
    pub kaynak_url: String,
    /// Okunan sayfa metninde birebir yer alan kanıt alıntısı.
    pub alinti: String,
}

/// Kaynaklı yanıt doğrulama hataları.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KaynakliYanitHatasi {
    #[error("en az {beklenen} farklı kaynak okunmalıydı, okunan: {bulunan}")]
    YetersizKaynakSayisi { beklenen: usize, bulunan: usize },

    #[error(
        "iddia okunmamış bir kaynağa (`{kaynak_url}`) atıf yapıyor; yalnız arama özetine (snippet) dayanarak yanıt üretilemez"
    )]
    OkunmamisKaynak { kaynak_url: String },

    #[error("`{kaynak_url}` kaynağında belirtilen kanıt alıntısı (`{alinti}`) bulunamadı")]
    AlintiBulunamadi { kaynak_url: String, alinti: String },
}

fn oge_normallestir(mut oge: HamAramaSonucOgesi) -> AramaSonucOgesi {
    if oge.ozet_parcacigi.trim().is_empty() && !oge.ek_vurgular.is_empty() {
        oge.ozet_parcacigi = oge
            .ek_vurgular
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
    }
    AramaSonucOgesi {
        baslik: oge.baslik,
        url: oge.url,
        ozet_parcacigi: oge.ozet_parcacigi,
    }
}

/// Exa MCP ve Brave Search MCP gibi sunucuların `CallToolResult.content` metin
/// bloklarında ürettiği `Title:` / `URL:` / `Text:` (`Description:` / `Highlights:`)
/// biçimli düz metin çıktısını [`AramaSonucOgesi`] listesine çözümler.
fn mcp_arama_metin_bloklarini_coz(metin: &str) -> Option<Vec<AramaSonucOgesi>> {
    let mut sonuclar = Vec::new();
    let mut baslik: Option<String> = None;
    let mut url: Option<String> = None;
    let mut ozet_parcalari: Vec<String> = Vec::new();

    let kayit_tamamla = |baslik: &mut Option<String>,
                         url: &mut Option<String>,
                         ozet_parcalari: &mut Vec<String>,
                         sonuclar: &mut Vec<AramaSonucOgesi>| {
        if let (Some(b), Some(u)) = (baslik.take(), url.take()) {
            let b_temiz = b.trim().to_string();
            let u_temiz = u.trim().to_string();
            if !b_temiz.is_empty() && !u_temiz.is_empty() {
                sonuclar.push(AramaSonucOgesi {
                    baslik: b_temiz,
                    url: u_temiz,
                    ozet_parcacigi: ozet_parcalari.join(" ").trim().to_string(),
                });
            }
        }
        ozet_parcalari.clear();
    };

    for ham_satir in metin.lines() {
        let satir = ham_satir.trim();
        if satir.is_empty() {
            continue;
        }
        if let Some(deger) = satir
            .strip_prefix("Title:")
            .or_else(|| satir.strip_prefix("Başlık:"))
        {
            if baslik.is_some() && url.is_some() {
                kayit_tamamla(&mut baslik, &mut url, &mut ozet_parcalari, &mut sonuclar);
            }
            baslik = Some(deger.trim().to_string());
        } else if let Some(deger) = satir
            .strip_prefix("URL:")
            .or_else(|| satir.strip_prefix("Url:"))
        {
            url = Some(deger.trim().to_string());
        } else if let Some(deger) = satir
            .strip_prefix("Text:")
            .or_else(|| satir.strip_prefix("Description:"))
            .or_else(|| satir.strip_prefix("Snippet:"))
            .or_else(|| satir.strip_prefix("Highlights:"))
            .or_else(|| satir.strip_prefix("Content:"))
            .or_else(|| satir.strip_prefix("Summary:"))
            .or_else(|| satir.strip_prefix("Özet:"))
        {
            let parca = deger.trim();
            if !parca.is_empty() {
                ozet_parcalari.push(parca.to_string());
            }
        }
    }

    kayit_tamamla(&mut baslik, &mut url, &mut ozet_parcalari, &mut sonuclar);
    if sonuclar.is_empty() {
        None
    } else {
        Some(sonuclar)
    }
}

/// MCP arama aracından gelen JSON veya yapılandırılmış metin çıktısını
/// [`AramaSonucOgesi`] listesine çözümler.
pub fn mcp_arama_json_coz(json_metni: &str) -> Result<Vec<AramaSonucOgesi>, AramaHatasi> {
    let temiz = json_metni.trim();
    let deger: serde_json::Value = match serde_json::from_str(temiz) {
        Ok(d) => d,
        Err(json_hatasi) => {
            if let Some(bloklar) = mcp_arama_metin_bloklarini_coz(temiz) {
                return Ok(bloklar);
            }
            return Err(AramaHatasi::CiktiAyristirma(json_hatasi.to_string()));
        }
    };

    if let Some(hata_kodu) = deger
        .get("hata_kodu")
        .or_else(|| deger.get("status"))
        .or_else(|| deger.get("code"))
        .and_then(|v| v.as_u64())
    {
        match hata_kodu {
            429 => {
                let sn = deger
                    .get("yeniden_dene_sn")
                    .or_else(|| deger.get("retry_after"))
                    .and_then(|v| v.as_u64());
                return Err(AramaHatasi::HizSiniri {
                    yeniden_dene_sn: sn,
                });
            }
            401..=403 => {
                let detay = deger
                    .get("mesaj")
                    .or_else(|| deger.get("message"))
                    .or_else(|| deger.get("error"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("yetkisiz veya yetersiz kredi")
                    .to_string();
                return Err(AramaHatasi::YetersizBakiyeVeyaKimlik { detay });
            }
            _ => {}
        }
    }

    let dizi_degeri = if deger.is_array() {
        Some(&deger)
    } else {
        deger
            .get("sonuclar")
            .or_else(|| deger.get("results"))
            .or_else(|| {
                deger.get("web").and_then(|w| {
                    if w.is_array() {
                        Some(w)
                    } else {
                        w.get("results").or_else(|| w.get("sonuclar"))
                    }
                })
            })
    };

    let Some(dizi) = dizi_degeri else {
        return Err(AramaHatasi::CiktiAyristirma(
            "JSON içinde sonuç dizisi (`sonuclar`, `results` veya `web.results`) bulunamadı".into(),
        ));
    };

    let ogeler = serde_json::from_value::<Vec<HamAramaSonucOgesi>>(dizi.clone())
        .map_err(|h| AramaHatasi::CiktiAyristirma(h.to_string()))?;
    Ok(ogeler.into_iter().map(oge_normallestir).collect())
}

fn hata_metnini_sinifla(icerik: &str) -> Option<AramaSaglayiciYaniti> {
    match mcp_arama_json_coz(icerik) {
        Err(AramaHatasi::HizSiniri { yeniden_dene_sn }) => {
            return Some(AramaSaglayiciYaniti::HizSiniri { yeniden_dene_sn });
        }
        Err(AramaHatasi::YetersizBakiyeVeyaKimlik { detay }) => {
            return Some(AramaSaglayiciYaniti::YetersizBakiyeVeyaKimlik { detay });
        }
        _ => {}
    }

    let kucuk = icerik.to_ascii_lowercase();
    if kucuk.contains("429")
        || kucuk.contains("rate limit")
        || kucuk.contains("too many requests")
        || icerik.contains("hız sınırı")
    {
        return Some(AramaSaglayiciYaniti::HizSiniri {
            yeniden_dene_sn: None,
        });
    }
    if kucuk.contains("401")
        || kucuk.contains("402")
        || kucuk.contains("403")
        || kucuk.contains("unauthorized")
        || kucuk.contains("payment required")
        || kucuk.contains("insufficient")
        || kucuk.contains("quota")
        || icerik.contains("yetkisiz")
        || icerik.contains("bakiye")
    {
        return Some(AramaSaglayiciYaniti::YetersizBakiyeVeyaKimlik {
            detay: icerik.trim().to_string(),
        });
    }
    None
}

/// `kalfa-mcp` araç yürütücüsünün (`McpArac` / `sonucu_donustur`) ürettiği
/// `Result<AracCiktisi, AracHatasi>` sözleşmesini [`AramaSaglayiciYaniti`]'na
/// dönüştürür.
pub fn mcp_arac_sonucunu_coz(
    sonuc: &Result<AracCiktisi, AracHatasi>,
) -> Result<AramaSaglayiciYaniti, AramaHatasi> {
    match sonuc {
        Ok(cikti) => {
            if !cikti.kaynak.starts_with("mcp:") {
                return Err(AramaHatasi::CiktiAyristirma(format!(
                    "beklenen `mcp:` kaynak etiketi, gelen: `{}`",
                    cikti.kaynak
                )));
            }
            match mcp_arama_json_coz(&cikti.icerik) {
                Ok(liste) => Ok(AramaSaglayiciYaniti::Basarili(liste)),
                Err(AramaHatasi::HizSiniri { yeniden_dene_sn }) => {
                    Ok(AramaSaglayiciYaniti::HizSiniri { yeniden_dene_sn })
                }
                Err(AramaHatasi::YetersizBakiyeVeyaKimlik { detay }) => {
                    Ok(AramaSaglayiciYaniti::YetersizBakiyeVeyaKimlik { detay })
                }
                Err(diger) => Err(diger),
            }
        }
        Err(AracHatasi::Iptal) => Err(AramaHatasi::IptalEdildi),
        Err(AracHatasi::Basarisiz(cikti)) => {
            if !cikti.kaynak.is_empty() && !cikti.kaynak.starts_with("mcp:") {
                return Err(AramaHatasi::CiktiAyristirma(format!(
                    "beklenen `mcp:` kaynak etiketi, gelen: `{}`",
                    cikti.kaynak
                )));
            }
            if let Some(yanit) = hata_metnini_sinifla(&cikti.icerik) {
                return Ok(yanit);
            }
            Err(AramaHatasi::CiktiAyristirma(cikti.icerik.clone()))
        }
        Err(diger) => Err(AramaHatasi::CiktiAyristirma(diger.to_string())),
    }
}

/// Arama sağlayıcısı yanıtını ve içindeki her URL'yi şema, kimlik bilgisi ve
/// özel ağ/loopback kurallarına göre doğrular.
pub fn arama_sonuclarini_dogrula(
    sorgu: &str,
    yanit: AramaSaglayiciYaniti,
    iptal: &CancellationToken,
) -> Result<Vec<DogrulanmisAramaSonucu>, AramaHatasi> {
    if iptal.is_cancelled() {
        return Err(AramaHatasi::IptalEdildi);
    }

    let ham_liste = match yanit {
        AramaSaglayiciYaniti::Basarili(liste) => liste,
        AramaSaglayiciYaniti::HizSiniri { yeniden_dene_sn } => {
            return Err(AramaHatasi::HizSiniri { yeniden_dene_sn });
        }
        AramaSaglayiciYaniti::YetersizBakiyeVeyaKimlik { detay } => {
            return Err(AramaHatasi::YetersizBakiyeVeyaKimlik { detay });
        }
    };

    if ham_liste.is_empty() {
        return Err(AramaHatasi::BosSonuc {
            sorgu: sorgu.trim().to_string(),
        });
    }

    let mut dogrulanmis = Vec::with_capacity(ham_liste.len());
    for oge in ham_liste {
        let (kanonik_url, host, _port) =
            url_dogrula(&oge.url).map_err(|h| AramaHatasi::GecersizSonucUrl {
                url: oge.url.clone(),
                neden: h.to_string(),
            })?;

        match kanonik_url.host() {
            Some(Host::Ipv4(v4)) => {
                if let Err(gerekce) = adres_denetle(IpAddr::V4(v4)) {
                    return Err(AramaHatasi::GecersizSonucUrl {
                        url: oge.url,
                        neden: gerekce.to_string(),
                    });
                }
            }
            Some(Host::Ipv6(v6)) => {
                if let Err(gerekce) = adres_denetle(IpAddr::V6(v6)) {
                    return Err(AramaHatasi::GecersizSonucUrl {
                        url: oge.url,
                        neden: gerekce.to_string(),
                    });
                }
            }
            _ => {}
        }

        dogrulanmis.push(DogrulanmisAramaSonucu {
            baslik: oge.baslik.trim().to_string(),
            kanonik_url: kanonik_url.to_string(),
            maskeli_url: url_nesnesi_maskele(&kanonik_url),
            host,
            ozet_parcacigi: oge.ozet_parcacigi.trim().to_string(),
        });
    }

    Ok(dogrulanmis)
}

/// Doğrulanmış arama sonuçlarından ilk `okunacak_adet` sayfayı `WebIstemci`
/// üzerinden okuyarak [`OkunanKaynakSayfa`] listesi döndürür.
pub async fn arama_kaynaklarini_oku(
    istemci: &WebIstemci,
    sonuclar: &[DogrulanmisAramaSonucu],
    okunacak_adet: usize,
    bolum: Option<&str>,
    azami_bayt: Option<usize>,
    iptal: CancellationToken,
) -> Result<Vec<OkunanKaynakSayfa>, AramaHatasi> {
    if iptal.is_cancelled() {
        return Err(AramaHatasi::IptalEdildi);
    }

    let mut okunanlar = Vec::new();
    for sonuc in sonuclar.iter().take(okunacak_adet) {
        if iptal.is_cancelled() {
            return Err(AramaHatasi::IptalEdildi);
        }
        let ham_yanit = istemci
            .getir(
                &sonuc.kanonik_url,
                &YonlendirmeIzni::AyniHost,
                iptal.clone(),
            )
            .await?;
        let islenmis = yaniti_metne_donustur(&ham_yanit, bolum, None, azami_bayt)?;
        okunanlar.push(OkunanKaynakSayfa {
            arama_basligi: sonuc.baslik.clone(),
            arama_ozeti: sonuc.ozet_parcacigi.clone(),
            sayfa_ozeti: islenmis.ozet,
            sayfa_metni: islenmis.govde,
        });
    }

    Ok(okunanlar)
}

fn metin_normallestir(girdi: &str) -> String {
    girdi
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Üretilen kaynaklı iddiaların gerçekten `web_getir` ile okunmuş sayfalara
/// dayandığını ve her iddianın kanıt alıntısının ilgili sayfa metninde yer
/// aldığını doğrular.
pub fn kaynakli_yanit_dogrula(
    okunan_sayfalar: &[OkunanKaynakSayfa],
    iddialar: &[KaynakliIddia],
    en_az_farkli_kaynak: usize,
) -> Result<(), KaynakliYanitHatasi> {
    let mut kullanilan_kaynaklar = Vec::new();

    for iddia in iddialar {
        let aranan_url = iddia.kaynak_url.trim();
        let eslesen_sayfa = okunan_sayfalar.iter().find(|s| {
            s.sayfa_ozeti.son_url == aranan_url || s.sayfa_ozeti.istenen_url == aranan_url
        });

        let Some(sayfa) = eslesen_sayfa else {
            return Err(KaynakliYanitHatasi::OkunmamisKaynak {
                kaynak_url: aranan_url.to_string(),
            });
        };

        let norm_sayfa = metin_normallestir(&sayfa.sayfa_metni);
        let norm_alinti = metin_normallestir(&iddia.alinti);
        if norm_alinti.is_empty() || !norm_sayfa.contains(&norm_alinti) {
            return Err(KaynakliYanitHatasi::AlintiBulunamadi {
                kaynak_url: aranan_url.to_string(),
                alinti: iddia.alinti.clone(),
            });
        }

        if !kullanilan_kaynaklar
            .iter()
            .any(|k: &String| k == &sayfa.sayfa_ozeti.son_url)
        {
            kullanilan_kaynaklar.push(sayfa.sayfa_ozeti.son_url.clone());
        }
    }

    if kullanilan_kaynaklar.len() < en_az_farkli_kaynak {
        return Err(KaynakliYanitHatasi::YetersizKaynakSayisi {
            beklenen: en_az_farkli_kaynak,
            bulunan: kullanilan_kaynaklar.len(),
        });
    }

    Ok(())
}
