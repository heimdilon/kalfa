//! Vekile gelen HTTP ve CONNECT isteklerinin ayrıştırılması (aşama 3 tasarımı §8.2).
//!
//! Desteklenen biçimler:
//! - `CONNECT host:port HTTP/1.0` ya da `HTTP/1.1`
//! - `GET http://host[:port]/path HTTP/1.0` ya da `HTTP/1.1` (mutlak HTTP)
//!
//! Başlık sınırı en çok 8 KiB, zaman aşımı 10 saniyedir. Başka yöntem ya da
//! mutlak olmayan URI `400` alır.

use std::time::Duration;

/// İstek başlığının azami boyutu (8 KiB).
pub const AZAMI_BASLIK_BOYUTU: usize = 8 * 1024;

/// Başlığın okunması için azami süre (10 saniye).
pub const BASLIK_ZAMAN_ASIMI: Duration = Duration::from_secs(10);

/// Ayrıştırılan vekil isteği.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Istek {
    /// HTTPS için CONNECT tüneli.
    Connect {
        host: String,
        port: u16,
        http10: bool,
    },
    /// Düz HTTP için mutlak biçimli istek.
    MutlakHttp {
        metot: String,
        host: String,
        port: u16,
        hedef_istek: Vec<u8>,
    },
    /// Desteklenmeyen yöntem, biçim ya da bozuk başlık (400).
    Gecersiz(String),
}

/// Okunan başlık baytlarını ayrıştırır.
pub fn istek_ayristir(baslik: &[u8]) -> Istek {
    let metin = match std::str::from_utf8(baslik) {
        Ok(m) => m,
        Err(_) => return Istek::Gecersiz("başlık geçerli UTF-8 değil".into()),
    };

    let ilk_satir = match metin.lines().next() {
        Some(s) => s.trim_end_matches('\r'),
        None => return Istek::Gecersiz("boş istek".into()),
    };

    let parcalar: Vec<&str> = ilk_satir.split_whitespace().collect();
    if parcalar.len() != 3 {
        return Istek::Gecersiz("geçersiz istek satırı biçimi".into());
    }

    let metot = parcalar[0];
    let uri = parcalar[1];
    let surum = parcalar[2];

    if surum != "HTTP/1.0" && surum != "HTTP/1.1" {
        return Istek::Gecersiz(format!("desteklenmeyen HTTP sürümü: {surum}"));
    }

    if metot.eq_ignore_ascii_case("CONNECT") {
        let (host, port) = match ayristir_host_port(uri, 443) {
            Ok(hp) => hp,
            Err(e) => return Istek::Gecersiz(e),
        };
        return Istek::Connect {
            host,
            port,
            http10: surum == "HTTP/1.0",
        };
    }

    if let Some(kalan) = uri.strip_prefix("http://") {
        let (host_port, yol) = match kalan.find(['/', '?']) {
            Some(idx) => (&kalan[..idx], &kalan[idx..]),
            None => (kalan, "/"),
        };

        let (host, port) = match ayristir_host_port(host_port, 80) {
            Ok(hp) => hp,
            Err(e) => return Istek::Gecersiz(e),
        };

        // Hedef sunucu için mutlak URI'yi göreli URI'ye çevir:
        // "GET http://host/path HTTP/1.1" -> "GET /path HTTP/1.1"
        let mut yeni_istek = Vec::new();
        yeni_istek.extend_from_slice(format!("{metot} {yol} {surum}\r\n").as_bytes());

        // İlk satırdan sonraki başlıkları ve gövdeyi aynen ekle
        if let Some(idx) = baslik.windows(2).position(|w| w == b"\r\n") {
            yeni_istek.extend_from_slice(&baslik[idx + 2..]);
        } else if let Some(idx) = baslik.iter().position(|&b| b == b'\n') {
            yeni_istek.extend_from_slice(&baslik[idx + 1..]);
        }

        return Istek::MutlakHttp {
            metot: metot.to_string(),
            host,
            port,
            hedef_istek: yeni_istek,
        };
    }

    Istek::Gecersiz(format!(
        "yalnızca CONNECT ya da mutlak http:// istekleri kabul edilir; alınan: {metot} {uri}"
    ))
}

fn ayristir_host_port(s: &str, varsayilan_port: u16) -> Result<(String, u16), String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("boş host".into());
    }

    if s.starts_with('[') {
        let kapanis = s.find(']').ok_or("kapanmamış IPv6 parantezi")?;
        let host = &s[1..kapanis];
        let kalan = &s[kapanis + 1..];
        let port = if kalan.is_empty() {
            varsayilan_port
        } else if let Some(p_str) = kalan.strip_prefix(':') {
            let p: u16 = p_str.parse().map_err(|_| "geçersiz port")?;
            if p == 0 {
                return Err("port 0 olamaz".into());
            }
            p
        } else {
            return Err("geçersiz karakter".into());
        };
        Ok((format!("[{}]", host.to_lowercase()), port))
    } else if let Some((h, p_str)) = s.rsplit_once(':') {
        if h.contains(':') {
            return Err("parantezsiz IPv6 kabul edilmez".into());
        }
        let p: u16 = p_str.parse().map_err(|_| "geçersiz port")?;
        if p == 0 {
            return Err("port 0 olamaz".into());
        }
        Ok((h.to_lowercase(), p))
    } else {
        Ok((s.to_lowercase(), varsayilan_port))
    }
}
