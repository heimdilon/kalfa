//! HTTP yanıtlarının içerik türü (Content-Type), ikili veri (binary), karakter
//! kümesi (charset), bölüm (`bolum`) filtresi, 64 KiB çıktı sınırı ve içerik
//! özetine bağlı devam imleci (`imlec`) ile metne dönüştürülmesi.

use sha2::{Digest, Sha256};

use crate::web::ag::{WebGetirHatasi, WebHamYanit};
use crate::web::html::{HtmlBaglantisi, html_cozumle, html_meta_karakter_kumesi_bul};
use crate::web::kaynak::WebSayfaOzeti;

/// Model çıktısına tek çağrıda aktarılabilecek azami UTF-8 bayt sayısı (64 KiB).
pub const AZAMI_CIKTI_BAYT: usize = 64 * 1024;

/// Özel `azami_bayt` parametresi için alt sınır.
pub const EN_AZ_CIKTI_BAYT: usize = 256;

/// Desteklenen metinsel içerik biçimleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcerikBicimi {
    Html,
    Metin,
    Json,
    Xml,
}

/// Ayrıştırılmış devam imleci (`web1:<ozet>:<bayt_ofseti>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImlecDurumu {
    /// Sayfanın beklenen 16 haneli SHA-256 içerik özeti.
    pub beklenen_ozet: String,
    /// Okumaya başlanacak 0-tabanlı UTF-8 bayt ofseti.
    pub baslangic_bayti: usize,
}

/// İşlenmiş web yanıtı: kaynak özeti, düz metin gövdesi ve çıkarılan bağlantılar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IslenmisWebIcerigi {
    /// Sayfanın kaynak, durum ve kırpılma özeti.
    pub ozet: WebSayfaOzeti,
    /// Model ve arayüz için üretilen düz metin dilimi.
    pub govde: String,
    /// HTML sayfalarından çıkarılan bağlantılar.
    pub baglantilar: Vec<HtmlBaglantisi>,
}

/// `Content-Type` başlığını ayrıştırarak izin verilen metin biçimlerinden biri
/// olup olmadığını denetler. `(bicim, mime_turu, charset)` döndürür.
pub fn icerik_turu_kabul_edilir_mi(
    baslik: Option<&str>,
) -> Result<(IcerikBicimi, String, Option<String>), WebGetirHatasi> {
    let Some(ham) = baslik else {
        return Err(WebGetirHatasi::DesteklenmeyenIcerikTuru(
            "eksik Content-Type başlığı".into(),
        ));
    };
    let temiz = ham.trim();
    if temiz.is_empty() {
        return Err(WebGetirHatasi::DesteklenmeyenIcerikTuru(
            "boş Content-Type başlığı".into(),
        ));
    }

    let mut parcalar = temiz.split(';');
    let mime = parcalar.next().unwrap_or("").trim().to_ascii_lowercase();

    let mut charset: Option<String> = None;
    for p in parcalar {
        if let Some((k, v)) = p.trim().split_once('=')
            && k.trim().eq_ignore_ascii_case("charset")
        {
            let deger = v.trim().trim_matches('"').trim_matches('\'').trim();
            if !deger.is_empty() {
                charset = Some(deger.to_ascii_lowercase());
            }
        }
    }

    let bicim = match mime.as_str() {
        "text/html" | "application/xhtml+xml" => IcerikBicimi::Html,
        "text/plain"
        | "text/markdown"
        | "text/x-markdown"
        | "text/csv"
        | "text/tab-separated-values" => IcerikBicimi::Metin,
        "application/json" | "text/json" => IcerikBicimi::Json,
        "application/xml" | "text/xml" => IcerikBicimi::Xml,
        diger if diger.ends_with("+json") => IcerikBicimi::Json,
        diger if diger.ends_with("+xml") && diger != "image/svg+xml" => IcerikBicimi::Xml,
        diger => {
            return Err(WebGetirHatasi::DesteklenmeyenIcerikTuru(diger.to_string()));
        }
    };

    Ok((bicim, mime, charset))
}

/// Yanıt baytlarının ikili (binary) bir dosya imzası veya sıfır (`\0`) baytları
/// içerip içermediğini kontrol eder.
pub fn ikili_mi(veri: &[u8], karakter_kumesi: &str) -> bool {
    if veri.is_empty() {
        return false;
    }

    const SIHIRLI_IMZALAR: &[&[u8]] = &[
        b"%PDF-",
        b"\x89PNG\r\n\x1a\n",
        b"GIF87a",
        b"GIF89a",
        b"\xff\xd8\xff",
        b"PK\x03\x04",
        b"\x7fELF",
        b"\x00asm",
        b"Rar!\x1a\x07",
    ];
    for imza in SIHIRLI_IMZALAR {
        if veri.starts_with(imza) {
            return true;
        }
    }

    let kucuk = karakter_kumesi.trim().to_ascii_lowercase();
    if kucuk.starts_with("utf-16")
        || veri.starts_with(&[0xFF, 0xFE])
        || veri.starts_with(&[0xFE, 0xFF])
    {
        return false;
    }

    veri.contains(&0)
}

/// Ham baytları bildirilen veya algılanan karakter kümesine göre UTF-8 dizgeye çözer.
pub fn baytlari_metne_coz(
    veri: &[u8],
    bildirilen_charset: Option<&str>,
    bicim: IcerikBicimi,
) -> Result<(String, String), WebGetirHatasi> {
    // BOM kontrolü
    if let Some(govde) = veri.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        if ikili_mi(govde, "utf-8") {
            return Err(WebGetirHatasi::IkiliIcerikReddedildi);
        }
        return utf8_coz(govde).map(|m| (m, "utf-8".into()));
    }
    if let Some(govde) = veri.strip_prefix(&[0xFF, 0xFE]) {
        return utf16_coz(govde, true).map(|m| (m, "utf-16le".into()));
    }
    if let Some(govde) = veri.strip_prefix(&[0xFE, 0xFF]) {
        return utf16_coz(govde, false).map(|m| (m, "utf-16be".into()));
    }

    let aday_charset = match bildirilen_charset {
        Some(c) => c.trim().to_ascii_lowercase(),
        None => {
            if bicim == IcerikBicimi::Html {
                html_meta_karakter_kumesi_bul(veri).unwrap_or_else(|| "utf-8".into())
            } else {
                "utf-8".into()
            }
        }
    };

    if ikili_mi(veri, &aday_charset) {
        return Err(WebGetirHatasi::IkiliIcerikReddedildi);
    }

    match aday_charset.as_str() {
        "utf-8" | "utf8" | "us-ascii" | "ascii" => utf8_coz(veri).map(|m| (m, "utf-8".into())),
        "iso-8859-1" | "latin1" | "iso_8859-1" => {
            let metin: String = veri.iter().map(|&b| char::from(b)).collect();
            Ok((metin, "iso-8859-1".into()))
        }
        "windows-1252" | "cp1252" => {
            let metin: String = veri.iter().map(|&b| cp1252_bayt_coz(b)).collect();
            Ok((metin, "windows-1252".into()))
        }
        "iso-8859-9" | "latin5" | "windows-1254" | "cp1254" | "turkish" => {
            let metin: String = veri.iter().map(|&b| turkce_1254_bayt_coz(b)).collect();
            Ok((metin, "iso-8859-9".into()))
        }
        "utf-16le" | "utf-16" => utf16_coz(veri, true).map(|m| (m, "utf-16le".into())),
        "utf-16be" => utf16_coz(veri, false).map(|m| (m, "utf-16be".into())),
        diger => Err(WebGetirHatasi::DesteklenmeyenKarakterKumesi(
            diger.to_string(),
        )),
    }
}

fn utf8_coz(veri: &[u8]) -> Result<String, WebGetirHatasi> {
    if let Ok(tam) = std::str::from_utf8(veri) {
        return Ok(tam.to_string());
    }
    let kayipli = String::from_utf8_lossy(veri).into_owned();
    let bozuk_sayisi = kayipli.chars().filter(|&c| c == '\u{FFFD}').count();
    let toplam_karakter = kayipli.chars().count().max(1);
    if bozuk_sayisi * 10 > toplam_karakter {
        return Err(WebGetirHatasi::IkiliIcerikReddedildi);
    }
    Ok(kayipli)
}

fn utf16_coz(veri: &[u8], kucuk_sonlu: bool) -> Result<String, WebGetirHatasi> {
    if !veri.len().is_multiple_of(2) {
        return Err(WebGetirHatasi::EksikGovde(
            "UTF-16 gövdesi tek sayıda bayt içeriyor".into(),
        ));
    }
    let kod_birimleri: Vec<u16> = veri
        .as_chunks::<2>()
        .0
        .iter()
        .map(|cift| {
            if kucuk_sonlu {
                u16::from_le_bytes([cift[0], cift[1]])
            } else {
                u16::from_be_bytes([cift[0], cift[1]])
            }
        })
        .collect();
    if kod_birimleri.contains(&0) {
        return Err(WebGetirHatasi::IkiliIcerikReddedildi);
    }
    String::from_utf16(&kod_birimleri).map_err(|_| WebGetirHatasi::IkiliIcerikReddedildi)
}

const CP1252_UST_TABLO: [char; 32] = [
    '€', '\u{0081}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{008D}', 'Ž',
    '\u{008F}', '\u{0090}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{009D}',
    'ž', 'Ÿ',
];

fn cp1252_bayt_coz(bayt: u8) -> char {
    if (0x80..=0x9F).contains(&bayt) {
        CP1252_UST_TABLO[usize::from(bayt - 0x80)]
    } else {
        char::from(bayt)
    }
}

fn turkce_1254_bayt_coz(bayt: u8) -> char {
    match bayt {
        0xD0 => 'Ğ',
        0xDD => 'İ',
        0xDE => 'Ş',
        0xF0 => 'ğ',
        0xFD => 'ı',
        0xFE => 'ş',
        diger => cp1252_bayt_coz(diger),
    }
}

/// Normalize edilmiş tam metnin 16 haneli onaltılık SHA-256 özetini hesaplar.
pub fn icerik_ozeti_hesapla(metin: &str) -> String {
    let mut ozetleyici = Sha256::new();
    ozetleyici.update(metin.as_bytes());
    let tam = hex::encode(ozetleyici.finalize());
    tam.chars().take(16).collect()
}

/// İçerik özeti ve UTF-8 bayt ofsetinden devam imleci (`web1:<ozet>:<bayt_ofseti>`) üretir.
pub fn imlec_olustur(icerik_ozeti: &str, baslangic_bayti: usize) -> String {
    format!("web1:{icerik_ozeti}:{baslangic_bayti}")
}

/// Devam imlecini (`web1:<ozet>:<bayt_ofseti>`) ayrıştırır.
pub fn imlec_coz(imlec: &str) -> Result<ImlecDurumu, WebGetirHatasi> {
    let parcalar: Vec<&str> = imlec.trim().split(':').collect();
    if parcalar.len() != 3 || parcalar[0] != "web1" {
        return Err(WebGetirHatasi::GecersizImlec(imlec.to_string()));
    }
    let beklenen_ozet = parcalar[1].trim();
    if beklenen_ozet.len() != 16 || !beklenen_ozet.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(WebGetirHatasi::GecersizImlec(imlec.to_string()));
    }
    let baslangic_bayti = parcalar[2]
        .trim()
        .parse::<usize>()
        .map_err(|_| WebGetirHatasi::GecersizImlec(imlec.to_string()))?;

    Ok(ImlecDurumu {
        beklenen_ozet: beklenen_ozet.to_ascii_lowercase(),
        baslangic_bayti,
    })
}

/// Ham HTTP yanıtını içerik türü, karakter kümesi, HTML ayrıştırması, `bolum`
/// filtresi, `imlec` doğrulaması ve `azami_bayt` sınırıyla işler.
pub fn yaniti_metne_donustur(
    yanit: &WebHamYanit,
    bolum: Option<&str>,
    imlec: Option<&str>,
    istenen_azami_bayt: Option<usize>,
) -> Result<IslenmisWebIcerigi, WebGetirHatasi> {
    let (bicim, mime, http_charset) =
        icerik_turu_kabul_edilir_mi(yanit.icerik_turu_basligi.as_deref())?;
    let (cozulen_metin, etkin_charset) =
        baytlari_metne_coz(&yanit.govde, http_charset.as_deref(), bicim)?;

    let (secilen_metin, baslik, baglantilar) = match bicim {
        IcerikBicimi::Html => {
            let belge = html_cozumle(&cozulen_metin, Some(&yanit.son_url_nesnesi));
            if let Some(aranan_bolum) = bolum.map(str::trim).filter(|s| !s.is_empty()) {
                let aranan_temiz = aranan_bolum.trim_start_matches('#').to_ascii_lowercase();
                let eslesen = belge.bolumler.iter().find(|b| {
                    b.baslik.to_ascii_lowercase().contains(&aranan_temiz)
                        || b.kimlik
                            .as_ref()
                            .is_some_and(|k| k.eq_ignore_ascii_case(&aranan_temiz))
                });
                let Some(bulunan) = eslesen else {
                    return Err(WebGetirHatasi::BolumBulunamadi(aranan_bolum.to_string()));
                };
                let bolum_basligi = match &belge.baslik {
                    Some(ana) => Some(format!("{ana} — {}", bulunan.baslik)),
                    None => Some(bulunan.baslik.clone()),
                };
                (bulunan.metin.clone(), bolum_basligi, belge.baglantilar)
            } else {
                (belge.duz_metin, belge.baslik, belge.baglantilar)
            }
        }
        IcerikBicimi::Json => {
            let bicimlendirilmis = match serde_json::from_str::<serde_json::Value>(&cozulen_metin) {
                Ok(deger) => {
                    serde_json::to_string_pretty(&deger).unwrap_or_else(|_| cozulen_metin.clone())
                }
                Err(_) => cozulen_metin,
            };
            let duzeltilmis = satir_sonlarini_normallestir(&bicimlendirilmis);
            (
                bolum_filtrele_duz_metin(&duzeltilmis, bolum)?,
                None,
                Vec::new(),
            )
        }
        IcerikBicimi::Metin | IcerikBicimi::Xml => {
            let duzeltilmis = satir_sonlarini_normallestir(&cozulen_metin);
            (
                bolum_filtrele_duz_metin(&duzeltilmis, bolum)?,
                None,
                Vec::new(),
            )
        }
    };

    let icerik_ozeti = icerik_ozeti_hesapla(&secilen_metin);
    let baslangic_bayti = match imlec.map(str::trim).filter(|s| !s.is_empty()) {
        Some(imlec_dizgesi) => {
            let durum = imlec_coz(imlec_dizgesi)?;
            if durum.beklenen_ozet != icerik_ozeti {
                return Err(WebGetirHatasi::KaynakDegisti {
                    beklenen_ozet: durum.beklenen_ozet,
                    yeni_ozet: icerik_ozeti,
                });
            }
            durum.baslangic_bayti
        }
        None => 0,
    };

    let azami_bayt = istenen_azami_bayt
        .unwrap_or(AZAMI_CIKTI_BAYT)
        .clamp(EN_AZ_CIKTI_BAYT, AZAMI_CIKTI_BAYT);

    let (govde, kirpildi, devam_imleci) =
        metin_araligi_kes(&secilen_metin, baslangic_bayti, azami_bayt, &icerik_ozeti)?;

    let ozet = WebSayfaOzeti {
        istenen_url: yanit.istenen_url.clone(),
        son_url: yanit.son_url.clone(),
        durum_kodu: yanit.durum_kodu,
        icerik_turu: mime,
        karakter_kumesi: etkin_charset,
        baslik,
        okunan_bayt: govde.len(),
        kirpildi,
        devam_imleci,
        icerik_ozeti,
        guvenilmez_dis_icerik: true,
    };

    Ok(IslenmisWebIcerigi {
        ozet,
        govde,
        baglantilar,
    })
}

fn satir_sonlarini_normallestir(girdi: &str) -> String {
    girdi.replace("\r\n", "\n").replace('\r', "\n")
}

fn bolum_filtrele_duz_metin(metin: &str, bolum: Option<&str>) -> Result<String, WebGetirHatasi> {
    let Some(aranan) = bolum.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(metin.to_string());
    };
    let kucuk_aranan = aranan.trim_start_matches('#').to_ascii_lowercase();
    let satirlar: Vec<&str> = metin.lines().collect();

    let mut baslangic: Option<usize> = None;
    for (indeks, satir) in satirlar.iter().enumerate() {
        if satir.to_ascii_lowercase().contains(&kucuk_aranan) {
            baslangic = Some(indeks);
            break;
        }
    }

    let Some(bas) = baslangic else {
        return Err(WebGetirHatasi::BolumBulunamadi(aranan.to_string()));
    };

    let bitis = (bas + 80).min(satirlar.len());
    Ok(satirlar[bas..bitis].join("\n"))
}

fn metin_araligi_kes(
    tam_metin: &str,
    baslangic_bayti: usize,
    azami_bayt: usize,
    icerik_ozeti: &str,
) -> Result<(String, bool, Option<String>), WebGetirHatasi> {
    if tam_metin.is_empty() {
        if baslangic_bayti > 0 {
            return Err(WebGetirHatasi::GecersizImlec(format!(
                "bayt ofseti ({baslangic_bayti}) boş içerik sınırını aşıyor"
            )));
        }
        return Ok((String::new(), false, None));
    }

    if baslangic_bayti >= tam_metin.len() || !tam_metin.is_char_boundary(baslangic_bayti) {
        return Err(WebGetirHatasi::GecersizImlec(format!(
            "bayt ofseti ({baslangic_bayti}) geçerli bir UTF-8 sınırı değil veya toplam uzunluğu ({}) aşıyor",
            tam_metin.len()
        )));
    }

    let kalan = &tam_metin[baslangic_bayti..];
    if kalan.len() <= azami_bayt {
        return Ok((kalan.to_string(), false, None));
    }

    let utf8_dilim = utf8_sinirinda_kes(kalan, azami_bayt);
    if utf8_dilim.is_empty() {
        return Err(WebGetirHatasi::GecersizImlec(
            "azami_bayt sınırı tek bir UTF-8 karakterini almaya yetmiyor".into(),
        ));
    }

    let kesme_uzunlugu = match utf8_dilim.rfind('\n') {
        Some(son_nl) => son_nl + 1,
        None => utf8_dilim.len(),
    };

    let parca = &kalan[..kesme_uzunlugu];
    let sonraki_bayt = baslangic_bayti + kesme_uzunlugu;
    let imlec = imlec_olustur(icerik_ozeti, sonraki_bayt);
    Ok((parca.to_string(), true, Some(imlec)))
}

fn utf8_sinirinda_kes(metin: &str, azami_bayt: usize) -> &str {
    if metin.len() <= azami_bayt {
        return metin;
    }
    let mut son = azami_bayt;
    while son > 0 && !metin.is_char_boundary(son) {
        son -= 1;
    }
    &metin[..son]
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use url::Url;

    fn sahte_yanit(icerik_turu: &str, govde: Vec<u8>) -> WebHamYanit {
        let url = Url::parse("https://belge.ornek.test/sayfa").expect("url");
        WebHamYanit {
            istenen_url: url.to_string(),
            son_url: url.to_string(),
            son_url_nesnesi: url,
            durum_kodu: 200,
            icerik_turu_basligi: Some(icerik_turu.to_string()),
            govde,
            baglanan_adres: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)), 443),
            yonlendirmeler: Vec::new(),
        }
    }

    #[test]
    fn turkce_iso_8859_9_ve_meta_charset_dogru_cozulur() {
        // "Çalışma" in ISO-8859-9: Ç=0xC7, a=0x61, l=0x6C, ı=0xFD, ş=0xFE, m=0x6D, a=0x61
        let baytlar = b"<html><head><meta charset=\"iso-8859-9\"><title>Ba\xfe\x6c\xfdk</title></head><body><p>\xc7al\xfd\xfema</p></body></html>".to_vec();
        let yanit = sahte_yanit("text/html", baytlar);
        let islenmis = yaniti_metne_donustur(&yanit, None, None, None).expect("işlenmeli");
        assert_eq!(islenmis.ozet.karakter_kumesi, "iso-8859-9");
        assert_eq!(islenmis.ozet.baslik.as_deref(), Some("Başlık"));
        assert!(islenmis.govde.contains("Çalışma"));
    }

    #[test]
    fn imlec_sayfa_degistiginde_hata_verir() {
        let satirlar: String = (1..=40)
            .map(|i| format!("Satır numarası {i:02} — örnek içerik metni"))
            .collect::<Vec<_>>()
            .join("\n");
        let yanit1 = sahte_yanit("text/plain; charset=utf-8", satirlar.into_bytes());
        let ilk_parca =
            yaniti_metne_donustur(&yanit1, None, None, Some(300)).expect("ilk parça okunmalı");
        assert!(ilk_parca.ozet.kirpildi);
        let imlec = ilk_parca
            .ozet
            .devam_imleci
            .clone()
            .expect("devam imleci olmalı");

        // Aynı sayfada imleç kaldığı yerden devam eder.
        let ikinci_parca = yaniti_metne_donustur(&yanit1, None, Some(&imlec), Some(300))
            .expect("ikinci parça okunmalı");
        assert!(!ikinci_parca.govde.contains("Satır numarası 01"));

        // Sayfa değiştiğinde eski imleç reddedilir.
        let degismis = sahte_yanit(
            "text/plain; charset=utf-8",
            b"Tamamen farkli yeni sayfa icerigi\nSatir 2".to_vec(),
        );
        let hata = yaniti_metne_donustur(&degismis, None, Some(&imlec), Some(300))
            .expect_err("değişen sayfada eski imleç reddedilmeli");
        assert!(matches!(hata, WebGetirHatasi::KaynakDegisti { .. }));
    }
}
