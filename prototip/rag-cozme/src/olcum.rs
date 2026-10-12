use std::hint::black_box;
use std::time::{Duration, Instant};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use kalfa_plugin_sdk::gomme_yanitini_coz;
use serde_json::{Value, json};

const ORNEK: usize = 21;
const ISINMA: usize = 3;
const HEDEF: Duration = Duration::from_millis(10);

/// Tam SDK yolu JSON çözme, vektör üretme ve doğrulamayı içerir.
/// Ham yollar yalnızca base64 + küçük endian f32 dönüşümünü karşılaştırır.
pub fn olc(argumanlar: &str, ortam: &str) -> Result<String, String> {
    let a: Value = serde_json::from_str(argumanlar).map_err(|e| e.to_string())?;
    let adet = sayi(&a, "adet", &[1, 16, 256])?;
    let boyut = sayi(&a, "boyut", &[1024, 4096])?;
    let yol = a["yol"].as_str().ok_or("ölçüm yolu yok")?;
    if !["sdk-base64", "sdk-float", "ham-crate", "ham-elle"].contains(&yol) {
        return Err("ölçüm yolu geçersiz".into());
    }

    // İki gerçek EVREN kaydından biri; eşleştirilen biçimler aynı f32
    // bitlerini taşır. Sıralı döndürmeler topludaki vektörleri farklılaştırır.
    let kayit = if boyut == 1024 {
        include_str!("../../../crates/kalfa-evren/tests/kayitlar/embeddings_qwen3_1024.json")
    } else {
        include_str!("../../../crates/kalfa-evren/tests/kayitlar/embeddings_qwen3_base64.json")
    };
    let tek = gomme_yanitini_coz(kayit, 1, boyut)
        .map_err(|e| format!("kayıt: {e:?}"))?
        .into_iter()
        .next()
        .ok_or("kayıt boş")?;
    let beklenen: Vec<Vec<f32>> = (0..adet)
        .map(|i| (0..boyut).map(|j| tek[(i + j) % boyut]).collect())
        .collect();
    let metinler: Vec<String> = beklenen
        .iter()
        .map(|v| {
            let bayt: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
            STANDARD.encode(bayt)
        })
        .collect();
    let base64 = json!({"object":"list", "data": metinler.iter().enumerate().map(|(i, m)| {
        json!({"object":"embedding", "index":i, "embedding":m})
    }).collect::<Vec<_>>()})
    .to_string();
    let float = json!({"object":"list", "data": beklenen.iter().enumerate().map(|(i, v)| {
        json!({"object":"embedding", "index":i, "embedding":v})
    }).collect::<Vec<_>>()})
    .to_string();

    let coz = || -> Result<Vec<Vec<f32>>, String> {
        match yol {
            "sdk-base64" => {
                gomme_yanitini_coz(black_box(&base64), adet, boyut).map_err(|e| format!("{e:?}"))
            }
            "sdk-float" => {
                gomme_yanitini_coz(black_box(&float), adet, boyut).map_err(|e| format!("{e:?}"))
            }
            _ => metinler
                .iter()
                .map(|m| {
                    let bayt = if yol == "ham-crate" {
                        STANDARD.decode(black_box(m)).map_err(|e| e.to_string())?
                    } else {
                        elle_coz(black_box(m))?
                    };
                    let (dortlu, kalan) = bayt.as_chunks::<4>();
                    if !kalan.is_empty() {
                        return Err("bayt sayısı 4'ün katı değil".into());
                    }
                    Ok(dortlu.iter().map(|b| f32::from_le_bytes(*b)).collect())
                })
                .collect(),
        }
    };
    // Doğrulama ve yanıt üretimi süreye girmez; ölçülen her yol eşdeğerdir.
    let gercek = coz()?;
    if gercek.len() != beklenen.len()
        || !gercek.iter().zip(&beklenen).all(|(g, b)| {
            g.len() == b.len() && g.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
        })
    {
        return Err("çözülen vektörlerin bitleri uyuşmuyor".into());
    }
    for _ in 0..ISINMA {
        black_box(coz()?);
    }
    let mut tekrar = 1usize;
    loop {
        let bas = Instant::now();
        for _ in 0..tekrar {
            black_box(coz()?);
        }
        if bas.elapsed() >= HEDEF || tekrar >= 16_384 {
            break;
        }
        tekrar *= 2;
    }
    let mut sureler = Vec::with_capacity(ORNEK);
    for _ in 0..ORNEK {
        let bas = Instant::now();
        for _ in 0..tekrar {
            black_box(coz()?);
        }
        sureler.push(bas.elapsed().as_nanos() as f64 / tekrar as f64);
    }
    sureler.sort_by(f64::total_cmp);
    Ok(json!({"ortam":ortam,"adet":adet,"boyut":boyut,"yol":yol,
        "base64_bayt":base64.len(),"float_bayt":float.len(),
        "ornek":ORNEK,"tekrar":tekrar,"medyan_ns":sureler[ORNEK / 2],
        "p95_ns":sureler[19],"en_az_ns":sureler[0],"bitler_esit":true})
    .to_string())
}

fn sayi(a: &Value, ad: &str, izinli: &[usize]) -> Result<usize, String> {
    let n = a[ad]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| format!("{ad} yok"))?;
    izinli
        .contains(&n)
        .then_some(n)
        .ok_or_else(|| format!("{ad} geçersiz"))
}

/// Ölçüm için basit, skaler çözücü; ürün SDK'sında kullanılmaz.
fn elle_coz(metin: &str) -> Result<Vec<u8>, String> {
    if !metin.len().is_multiple_of(4) {
        return Err("base64 uzunluğu geçersiz".into());
    }
    let mut bayt = Vec::with_capacity(metin.len() / 4 * 3);
    for (i, grup) in metin.as_bytes().as_chunks::<4>().0.iter().enumerate() {
        let mut deger = 0u32;
        let mut dolgu = 0;
        for &c in grup {
            let n = match c {
                b'A'..=b'Z' => u32::from(c - b'A'),
                b'a'..=b'z' => u32::from(c - b'a' + 26),
                b'0'..=b'9' => u32::from(c - b'0' + 52),
                b'+' => 62,
                b'/' => 63,
                b'=' => {
                    dolgu += 1;
                    0
                }
                _ => return Err("base64 karakteri geçersiz".into()),
            };
            if dolgu > 0 && c != b'=' {
                return Err("base64 dolgusu geçersiz".into());
            }
            deger = (deger << 6) | n;
        }
        if dolgu > 2
            || (dolgu > 0 && i + 1 != metin.len() / 4)
            || (dolgu == 1 && deger & 0xff != 0)
            || (dolgu == 2 && deger & 0xffff != 0)
        {
            return Err("base64 dolgusu geçersiz".into());
        }
        bayt.push((deger >> 16) as u8);
        if dolgu < 2 {
            bayt.push((deger >> 8) as u8);
        }
        if dolgu == 0 {
            bayt.push(deger as u8);
        }
    }
    Ok(bayt)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn elle_cozucu_crate_ile_ayni_baytlari_verir() {
        for n in [0, 1, 2, 3, 4, 1024, 4096] {
            let bayt: Vec<u8> = (0..n).map(|i| (i % 256) as u8).collect();
            assert_eq!(elle_coz(&STANDARD.encode(&bayt)).unwrap(), bayt);
        }
    }

    #[test]
    fn elle_cozucu_bozuk_dolguyu_reddeder() {
        for m in ["A", "====", "AA=A", "AA==AAAA", "AB==", "AAB=", "AA?="] {
            assert!(elle_coz(m).is_err(), "{m}");
        }
    }

    #[test]
    fn olcum_argumanlari_sinirli() {
        assert!(olc(r#"{"adet":257,"boyut":1024,"yol":"sdk-base64"}"#, "test").is_err());
        assert!(olc(r#"{"adet":1,"boyut":1024,"yol":"bilinmeyen"}"#, "test").is_err());
    }
}
