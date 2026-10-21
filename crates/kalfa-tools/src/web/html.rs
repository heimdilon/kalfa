//! Durum makineli (state-machine) HTML sözcükbirimleyici ve okunabilir metin çıkarıcı.
//!
//! Plan kısıtı gereği genel HTML ayrıştırması için düzenli ifade (`regex`)
//! kullanılmaz. Etiket sınırları, tırnak içi `>` karakterleri, `<!-- ... -->`
//! yorumları ve `<script>`/`<style>`/`<noscript>`/`<template>`/`<svg>`/`<iframe>`
//! ham metin blokları durum makinesiyle işlenir.

use url::Url;

use crate::web::kaynak::url_nesnesi_maskele;

/// HTML belgesinden çıkarılan bir başlık/bölüm dilimi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlBolumu {
    /// Başlık metni (ör. `"Kurulum"`).
    pub baslik: String,
    /// Başlık veya kapsayıcı etiketin `id` özniteliği (varsa).
    pub kimlik: Option<String>,
    /// Başlık seviyesi (`1`..=`6`).
    pub seviye: u8,
    /// Bölümün başlığı ve alt içeriğini barındıran okunabilir düz metin.
    pub metin: String,
}

/// HTML belgesindeki bir köprü (`<a href="...">`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlBaglantisi {
    /// Köprü metni.
    pub metin: String,
    /// Taban URL ile çözümlenmiş ve hassas sorgu parametreleri maskelenmiş `http`/`https` adresi.
    pub url: String,
}

/// Çözümlenmiş HTML belgesi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlBelgesi {
    /// `<title>` etiketi içeriği (varsa).
    pub baslik: Option<String>,
    /// Okunabilir düz metin çıktısı.
    pub duz_metin: String,
    /// Başlıklara göre ayrılmış bölümler.
    pub bolumler: Vec<HtmlBolumu>,
    /// Belgedeki dış/iç bağlantılar.
    pub baglantilar: Vec<HtmlBaglantisi>,
}

#[derive(Debug, Clone)]
struct EtiketBilgisi {
    ad: String,
    kapanis_mi: bool,
    kendi_kapaniyor_mu: bool,
    ozellikler: Vec<(String, String)>,
}

impl EtiketBilgisi {
    fn ozellik_al(&self, aranan: &str) -> Option<&str> {
        self.ozellikler
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(aranan))
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone)]
enum Sozcuk {
    Metin(String),
    Etiket(EtiketBilgisi),
}

const GORUNMEZ_ETIKETLER: &[&str] = &[
    "script", "style", "noscript", "template", "svg", "iframe", "object", "embed", "canvas",
];

const BLOK_ETIKETLERI: &[&str] = &[
    "p",
    "div",
    "section",
    "article",
    "main",
    "header",
    "footer",
    "nav",
    "aside",
    "blockquote",
    "pre",
    "table",
    "tr",
    "ul",
    "ol",
    "dl",
    "dt",
    "dd",
    "figure",
    "figcaption",
    "hr",
    "br",
    "form",
    "fieldset",
    "details",
    "summary",
];

/// HTML kaynağını durum makinesiyle çözümleyerek başlık, bölüm, bağlantı ve
/// okunabilir düz metin çıkarır.
pub fn html_cozumle(kaynak: &str, taban_url: Option<&Url>) -> HtmlBelgesi {
    let sozcukler = sozcuklere_ayir(kaynak);

    let mut belge_basligi: Option<String> = None;
    let mut baslik_icinde = false;
    let mut baslik_tamponu = String::new();

    let mut aktif_baslik_seviyesi: Option<u8> = None;
    let mut aktif_baslik_kimligi: Option<String> = None;
    let mut aktif_baslik_tamponu = String::new();

    let mut son_bolum_kimligi: Option<String> = None;

    let mut aktif_baglanti_href: Option<String> = None;
    let mut aktif_baglanti_metni = String::new();
    let mut baglantilar: Vec<HtmlBaglantisi> = Vec::new();

    // Ham bölüm kayıtları: (seviye, baslik, kimlik, baslangic_satir_indeksi)
    let mut ham_bolum_basliklari: Vec<(u8, String, Option<String>, usize)> = Vec::new();

    let mut satirlar: Vec<String> = Vec::new();
    let mut mevcut_satir = String::new();
    let mut on_bicimli_derinlik = 0usize;

    let satiri_kapat = |satirlar: &mut Vec<String>, mevcut_satir: &mut String, pre: bool| {
        if pre {
            satirlar.push(std::mem::take(mevcut_satir));
        } else {
            let temiz = bosluk_sadelestir(mevcut_satir);
            mevcut_satir.clear();
            if !temiz.is_empty() {
                satirlar.push(temiz);
            }
        }
    };

    for sozcuk in sozcukler {
        match sozcuk {
            Sozcuk::Metin(ham_metin) => {
                let cozulmus = html_varliklarini_coz(&ham_metin);
                if baslik_icinde {
                    baslik_tamponu.push_str(&cozulmus);
                    continue;
                }
                if aktif_baslik_seviyesi.is_some() {
                    aktif_baslik_tamponu.push_str(&cozulmus);
                }
                if aktif_baglanti_href.is_some() {
                    aktif_baglanti_metni.push_str(&cozulmus);
                }
                if on_bicimli_derinlik > 0 {
                    for (indeks, parca) in cozulmus.split('\n').enumerate() {
                        if indeks > 0 {
                            satirlar.push(std::mem::take(&mut mevcut_satir));
                        }
                        mevcut_satir.push_str(parca.trim_end_matches('\r'));
                    }
                } else {
                    mevcut_satir.push_str(&cozulmus);
                }
            }
            Sozcuk::Etiket(etiket) => {
                let ad = etiket.ad.as_str();
                if ad == "title" {
                    if !etiket.kapanis_mi && !etiket.kendi_kapaniyor_mu {
                        baslik_icinde = true;
                        baslik_tamponu.clear();
                    } else if etiket.kapanis_mi && baslik_icinde {
                        baslik_icinde = false;
                        let temiz = bosluk_sadelestir(&baslik_tamponu);
                        if !temiz.is_empty() && belge_basligi.is_none() {
                            belge_basligi = Some(temiz);
                        }
                    }
                    continue;
                }

                if ad == "pre" {
                    satiri_kapat(&mut satirlar, &mut mevcut_satir, on_bicimli_derinlik > 0);
                    if !etiket.kapanis_mi && !etiket.kendi_kapaniyor_mu {
                        on_bicimli_derinlik = on_bicimli_derinlik.saturating_add(1);
                    } else if etiket.kapanis_mi {
                        on_bicimli_derinlik = on_bicimli_derinlik.saturating_sub(1);
                    }
                    continue;
                }

                if let Some(seviye) = baslik_seviyesi(ad) {
                    if !etiket.kapanis_mi && !etiket.kendi_kapaniyor_mu {
                        satiri_kapat(&mut satirlar, &mut mevcut_satir, on_bicimli_derinlik > 0);
                        aktif_baslik_seviyesi = Some(seviye);
                        aktif_baslik_kimligi = etiket
                            .ozellik_al("id")
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(str::to_string)
                            .or_else(|| son_bolum_kimligi.take());
                        aktif_baslik_tamponu.clear();
                        let on_ek = "#".repeat(usize::from(seviye));
                        mevcut_satir.push_str(&on_ek);
                        mevcut_satir.push(' ');
                    } else if etiket.kapanis_mi {
                        satiri_kapat(&mut satirlar, &mut mevcut_satir, on_bicimli_derinlik > 0);
                        if let Some(sev) = aktif_baslik_seviyesi.take() {
                            let temiz_baslik = bosluk_sadelestir(&aktif_baslik_tamponu);
                            let kimlik = aktif_baslik_kimligi.take();
                            if !temiz_baslik.is_empty() {
                                let satir_indeksi = satirlar.len().saturating_sub(1);
                                ham_bolum_basliklari.push((
                                    sev,
                                    temiz_baslik,
                                    kimlik,
                                    satir_indeksi,
                                ));
                            }
                        }
                    }
                    continue;
                }

                if (ad == "section" || ad == "article" || ad == "main" || ad == "div")
                    && !etiket.kapanis_mi
                    && let Some(kimlik) = etiket.ozellik_al("id")
                {
                    let temiz = kimlik.trim();
                    if !temiz.is_empty() {
                        son_bolum_kimligi = Some(temiz.to_string());
                    }
                }

                if ad == "li" && !etiket.kapanis_mi {
                    satiri_kapat(&mut satirlar, &mut mevcut_satir, on_bicimli_derinlik > 0);
                    mevcut_satir.push_str("- ");
                    continue;
                }

                if ad == "td" || ad == "th" {
                    if !etiket.kapanis_mi && !mevcut_satir.is_empty() {
                        mevcut_satir.push_str(" | ");
                    }
                    continue;
                }

                if ad == "a" {
                    if !etiket.kapanis_mi && !etiket.kendi_kapaniyor_mu {
                        aktif_baglanti_href = etiket.ozellik_al("href").map(str::to_string);
                        aktif_baglanti_metni.clear();
                    } else if etiket.kapanis_mi
                        && let Some(ham_href) = aktif_baglanti_href.take()
                    {
                        let cozulmus_href = html_varliklarini_coz(&ham_href);
                        if let Some(cozulmus_url) = baglanti_url_coz(&cozulmus_href, taban_url) {
                            let metin = bosluk_sadelestir(&aktif_baglanti_metni);
                            baglantilar.push(HtmlBaglantisi {
                                metin: if metin.is_empty() {
                                    cozulmus_url.clone()
                                } else {
                                    metin
                                },
                                url: cozulmus_url,
                            });
                        }
                        aktif_baglanti_metni.clear();
                    }
                    continue;
                }

                if BLOK_ETIKETLERI.contains(&ad) {
                    satiri_kapat(&mut satirlar, &mut mevcut_satir, on_bicimli_derinlik > 0);
                }
            }
        }
    }

    satiri_kapat(&mut satirlar, &mut mevcut_satir, on_bicimli_derinlik > 0);

    let mut bolumler = Vec::with_capacity(ham_bolum_basliklari.len());
    for (indeks, (seviye, baslik, kimlik, baslangic)) in ham_bolum_basliklari.iter().enumerate() {
        let bitis = ham_bolum_basliklari
            .iter()
            .skip(indeks + 1)
            .find(|(sonraki_seviye, _, _, _)| *sonraki_seviye <= *seviye)
            .map(|(_, _, _, sonraki_baslangic)| *sonraki_baslangic)
            .unwrap_or(satirlar.len());

        let dilim = if *baslangic <= bitis && bitis <= satirlar.len() {
            satirlar[*baslangic..bitis].join("\n")
        } else {
            String::new()
        };

        bolumler.push(HtmlBolumu {
            baslik: baslik.clone(),
            kimlik: kimlik.clone(),
            seviye: *seviye,
            metin: dilim,
        });
    }

    let duz_metin = satirlar.join("\n");

    HtmlBelgesi {
        baslik: belge_basligi,
        duz_metin,
        bolumler,
        baglantilar,
    }
}

fn baslik_seviyesi(ad: &str) -> Option<u8> {
    match ad {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
        _ => None,
    }
}

fn baglanti_url_coz(ham_href: &str, taban_url: Option<&Url>) -> Option<String> {
    let temiz = ham_href.trim();
    if temiz.is_empty() || temiz.starts_with('#') {
        return None;
    }
    let url = match Url::parse(temiz) {
        Ok(u) => u,
        Err(_) => taban_url?.join(temiz).ok()?,
    };
    let sema = url.scheme();
    if !sema.eq_ignore_ascii_case("https") && !sema.eq_ignore_ascii_case("http") {
        return None;
    }
    if !url.username().is_empty() || url.password().is_some() {
        return None;
    }
    Some(url_nesnesi_maskele(&url))
}

fn bosluk_sadelestir(girdi: &str) -> String {
    let mut cikti = String::with_capacity(girdi.len());
    let mut onceki_bosluk = false;
    for kr in girdi.chars() {
        if kr.is_whitespace() {
            if !onceki_bosluk && !cikti.is_empty() {
                cikti.push(' ');
            }
            onceki_bosluk = true;
        } else {
            cikti.push(kr);
            onceki_bosluk = false;
        }
    }
    if cikti.ends_with(' ') {
        cikti.pop();
    }
    cikti
}

fn sozcuklere_ayir(kaynak: &str) -> Vec<Sozcuk> {
    let karakterler: Vec<char> = kaynak.chars().collect();
    let n = karakterler.len();
    let mut i = 0usize;
    let mut sozcukler = Vec::new();
    let mut metin_tamponu = String::new();
    let mut gorunmez_etiket: Option<String> = None;

    while i < n {
        if let Some(ref kapali_ad) = gorunmez_etiket {
            if karakterler[i] == '<'
                && i + 1 < n
                && karakterler[i + 1] == '/'
                && let Some((etiket, yeni_i)) = etiket_oku(&karakterler, i)
                && etiket.kapanis_mi
                && etiket.ad == *kapali_ad
            {
                gorunmez_etiket = None;
                i = yeni_i;
                continue;
            }
            i += 1;
            continue;
        }

        if karakterler[i] == '<' {
            // HTML yorumu: <!-- ... -->
            if i + 3 < n
                && karakterler[i + 1] == '!'
                && karakterler[i + 2] == '-'
                && karakterler[i + 3] == '-'
            {
                i += 4;
                while i + 2 < n
                    && !(karakterler[i] == '-'
                        && karakterler[i + 1] == '-'
                        && karakterler[i + 2] == '>')
                {
                    i += 1;
                }
                i = (i + 3).min(n);
                continue;
            }

            // <!DOCTYPE ...> veya <![CDATA[...]]> bildirimleri
            if i + 1 < n && (karakterler[i + 1] == '!' || karakterler[i + 1] == '?') {
                i += 2;
                while i < n && karakterler[i] != '>' {
                    i += 1;
                }
                if i < n {
                    i += 1;
                }
                continue;
            }

            if let Some((etiket, yeni_i)) = etiket_oku(&karakterler, i) {
                if !metin_tamponu.is_empty() {
                    sozcukler.push(Sozcuk::Metin(std::mem::take(&mut metin_tamponu)));
                }
                if !etiket.kapanis_mi
                    && !etiket.kendi_kapaniyor_mu
                    && GORUNMEZ_ETIKETLER.contains(&etiket.ad.as_str())
                {
                    gorunmez_etiket = Some(etiket.ad.clone());
                }
                sozcukler.push(Sozcuk::Etiket(etiket));
                i = yeni_i;
                continue;
            }
        }

        metin_tamponu.push(karakterler[i]);
        i += 1;
    }

    if !metin_tamponu.is_empty() {
        sozcukler.push(Sozcuk::Metin(metin_tamponu));
    }

    sozcukler
}

fn etiket_oku(karakterler: &[char], baslangic: usize) -> Option<(EtiketBilgisi, usize)> {
    let n = karakterler.len();
    if baslangic + 1 >= n || karakterler[baslangic] != '<' {
        return None;
    }
    let mut i = baslangic + 1;
    let mut kapanis_mi = false;
    if karakterler[i] == '/' {
        kapanis_mi = true;
        i += 1;
    }
    if i >= n || !karakterler[i].is_ascii_alphabetic() {
        return None;
    }

    let mut ad = String::new();
    while i < n
        && (karakterler[i].is_ascii_alphanumeric()
            || karakterler[i] == '-'
            || karakterler[i] == ':'
            || karakterler[i] == '_')
    {
        ad.push(karakterler[i].to_ascii_lowercase());
        i += 1;
    }

    let mut ozellikler = Vec::new();
    let mut kendi_kapaniyor_mu = false;

    while i < n {
        while i < n && karakterler[i].is_whitespace() {
            i += 1;
        }
        if i >= n {
            break;
        }
        if karakterler[i] == '>' {
            i += 1;
            return Some((
                EtiketBilgisi {
                    ad,
                    kapanis_mi,
                    kendi_kapaniyor_mu,
                    ozellikler,
                },
                i,
            ));
        }
        if karakterler[i] == '/' && i + 1 < n && karakterler[i + 1] == '>' {
            kendi_kapaniyor_mu = true;
            i += 2;
            return Some((
                EtiketBilgisi {
                    ad,
                    kapanis_mi,
                    kendi_kapaniyor_mu,
                    ozellikler,
                },
                i,
            ));
        }

        // Öznitelik adını oku.
        let mut oz_ad = String::new();
        while i < n
            && !karakterler[i].is_whitespace()
            && karakterler[i] != '='
            && karakterler[i] != '>'
            && karakterler[i] != '/'
        {
            oz_ad.push(karakterler[i].to_ascii_lowercase());
            i += 1;
        }
        if oz_ad.is_empty() {
            i += 1;
            continue;
        }

        while i < n && karakterler[i].is_whitespace() {
            i += 1;
        }

        let mut oz_deger = String::new();
        if i < n && karakterler[i] == '=' {
            i += 1;
            while i < n && karakterler[i].is_whitespace() {
                i += 1;
            }
            if i < n && (karakterler[i] == '"' || karakterler[i] == '\'') {
                let tirnak = karakterler[i];
                i += 1;
                while i < n && karakterler[i] != tirnak {
                    oz_deger.push(karakterler[i]);
                    i += 1;
                }
                if i < n && karakterler[i] == tirnak {
                    i += 1;
                }
            } else {
                while i < n && !karakterler[i].is_whitespace() && karakterler[i] != '>' {
                    oz_deger.push(karakterler[i]);
                    i += 1;
                }
            }
        }

        ozellikler.push((oz_ad, oz_deger));
    }

    None
}

/// HTML ilk 2048 baytında `<meta charset="...">` veya
/// `<meta http-equiv="content-type" content="...charset=...">` bildirimi varsa bulur.
pub fn html_meta_karakter_kumesi_bul(ilk_baytlar: &[u8]) -> Option<String> {
    let sinir = ilk_baytlar.len().min(2048);
    let kesit = String::from_utf8_lossy(&ilk_baytlar[..sinir]);
    for sozcuk in sozcuklere_ayir(&kesit) {
        if let Sozcuk::Etiket(etiket) = sozcuk
            && etiket.ad == "meta"
        {
            if let Some(charset) = etiket.ozellik_al("charset") {
                let temiz = charset.trim().trim_matches('"').trim_matches('\'').trim();
                if !temiz.is_empty() {
                    return Some(temiz.to_ascii_lowercase());
                }
            }
            if let Some(icerik) = etiket.ozellik_al("content") {
                for parca in icerik.split(';') {
                    let p = parca.trim();
                    if let Some((anahtar, deger)) = p.split_once('=')
                        && anahtar.trim().eq_ignore_ascii_case("charset")
                    {
                        let temiz = deger.trim().trim_matches('"').trim_matches('\'').trim();
                        if !temiz.is_empty() {
                            return Some(temiz.to_ascii_lowercase());
                        }
                    }
                }
            }
        }
    }
    None
}

/// HTML karakter referanslarını (`&amp;`, `&#39;`, `&#x1F600;` vb.) çözer.
pub fn html_varliklarini_coz(girdi: &str) -> String {
    if !girdi.contains('&') {
        return girdi.to_string();
    }
    let mut cikti = String::with_capacity(girdi.len());
    let karakterler: Vec<char> = girdi.chars().collect();
    let n = karakterler.len();
    let mut i = 0usize;

    while i < n {
        if karakterler[i] == '&' {
            let mut j = i + 1;
            while j < n && j - i <= 12 && karakterler[j] != ';' && karakterler[j] != '&' {
                j += 1;
            }
            if j < n && karakterler[j] == ';' && j > i + 1 {
                let ad: String = karakterler[i + 1..j].iter().collect();
                if let Some(cozulmus) = tek_varlik_coz(&ad) {
                    cikti.push_str(&cozulmus);
                    i = j + 1;
                    continue;
                }
            }
        }
        cikti.push(karakterler[i]);
        i += 1;
    }

    cikti
}

fn tek_varlik_coz(ad: &str) -> Option<String> {
    if let Some(sayi_kismi) = ad.strip_prefix('#') {
        let kod = if let Some(onaltilik) = sayi_kismi
            .strip_prefix('x')
            .or_else(|| sayi_kismi.strip_prefix('X'))
        {
            u32::from_str_radix(onaltilik, 16).ok()?
        } else {
            sayi_kismi.parse::<u32>().ok()?
        };
        if kod == 0 {
            return None;
        }
        let kr = char::from_u32(kod)?;
        return Some(kr.to_string());
    }

    let karsilik = match ad {
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "quot" => "\"",
        "apos" | "#39" => "'",
        "nbsp" => " ",
        "copy" => "©",
        "reg" => "®",
        "trade" => "™",
        "mdash" => "—",
        "ndash" => "–",
        "hellip" => "…",
        "euro" => "€",
        "pound" => "£",
        "laquo" => "«",
        "raquo" => "»",
        _ => return None,
    };
    Some(karsilik.to_string())
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn html_cozumle_gorunmez_bloklari_atar_ve_tirnak_ici_buyuktur_isaretini_korur() {
        let html = r#"<!DOCTYPE html>
<html>
<head>
  <title>Kalfa &amp; Web Kılavuzu</title>
  <style>body { display: none; }</style>
  <script>if (1 < 2 && 3 > 1) { console.log("gizli betik"); }</script>
</head>
<body>
  <!-- <script>yorum içi</script> -->
  <div data-test="a > b" id="giris">
    <h1>Giriş</h1>
    <p>İlk paragraf &lt;güvenli&gt; metin.</p>
  </div>
  <section id="kurulum">
    <h2>Kurulum Adımları</h2>
    <ul>
      <li>Adım bir</li>
      <li>Adım iki: <a href="/indir?token=gizli123&amp;surum=1">İndir</a></li>
    </ul>
  </section>
  <noscript>Tarayıcı betiği kapalı</noscript>
</body>
</html>"#;

        let taban = Url::parse("https://belge.ornek.test/dokuman/").expect("taban url");
        let belge = html_cozumle(html, Some(&taban));

        assert_eq!(belge.baslik.as_deref(), Some("Kalfa & Web Kılavuzu"));
        assert!(!belge.duz_metin.contains("gizli betik"));
        assert!(!belge.duz_metin.contains("display: none"));
        assert!(!belge.duz_metin.contains("Tarayıcı betiği kapalı"));
        assert!(belge.duz_metin.contains("# Giriş"));
        assert!(belge.duz_metin.contains("İlk paragraf <güvenli> metin."));
        assert!(belge.duz_metin.contains("## Kurulum Adımları"));
        assert!(belge.duz_metin.contains("- Adım bir"));

        assert_eq!(belge.bolumler.len(), 2);
        assert_eq!(belge.bolumler[1].baslik, "Kurulum Adımları");
        assert_eq!(belge.bolumler[1].kimlik.as_deref(), Some("kurulum"));

        assert_eq!(belge.baglantilar.len(), 1);
        assert_eq!(
            belge.baglantilar[0].url,
            "https://belge.ornek.test/indir?token=[MASKELENDI]&surum=1"
        );
    }
}
