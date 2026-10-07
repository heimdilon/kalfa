//! Vekil ortam değişkenleri (aşama 3 tasarımı §8.5).
//!
//! Ağlı komutların vekili kullanması için gereken standart ortam değişkenleri.
//! Hem Linux hem de Windows köprüsü tarafından kullanılır.

/// Verilen yerel vekil portu için ortam değişkenlerini döndürür.
///
/// Linux'ta küçük harfli yazımlar da verilir (curl `http_proxy`'yi yalnızca
/// küçük harfle okur). Windows'ta ortam adları büyük/küçük harf ayırmaz;
/// iki yazım aynı adı iki kez tanımlar ve PowerShell gibi ortamı sözlüğe
/// okuyan araçları bozar. Orada yalnızca büyük harfli yazım verilir.
pub fn vekil_ortami(port: u16) -> Vec<(String, String)> {
    let vekil = format!("http://127.0.0.1:{port}");
    let yerel = "localhost,127.0.0.1,::1";
    let mut ortam: Vec<(String, String)> = vec![
        ("HTTPS_PROXY".into(), vekil.clone()),
        ("HTTP_PROXY".into(), vekil.clone()),
        ("ALL_PROXY".into(), vekil.clone()),
        ("NO_PROXY".into(), yerel.into()),
    ];
    if !cfg!(windows) {
        ortam.extend([
            ("https_proxy".into(), vekil.clone()),
            ("http_proxy".into(), vekil.clone()),
            ("no_proxy".into(), yerel.into()),
        ]);
    }
    ortam.extend::<[(String, String); 8]>([
        ("NODE_USE_ENV_PROXY".into(), "1".into()),
        ("npm_config_https_proxy".into(), vekil.clone()),
        ("npm_config_proxy".into(), vekil.clone()),
        ("CARGO_HTTP_PROXY".into(), vekil.clone()),
        ("PIP_PROXY".into(), vekil.clone()),
        ("GIT_CONFIG_COUNT".into(), "1".into()),
        ("GIT_CONFIG_KEY_0".into(), "http.proxy".into()),
        ("GIT_CONFIG_VALUE_0".into(), vekil),
    ]);
    // Schannel'ın iptal denetimi (CryptNet) vekile uymaz; `internetClient`'sız
    // kapta CRL ve OCSP uçlarına ulaşamaz, stapling yapmayan hostlarda
    // (index.crates.io) bağlantı `CRYPT_E_REVOCATION_OFFLINE` ile düşer.
    // cargo Windows dışında zaten denetlemez (3d iptal bulguları).
    if cfg!(windows) {
        ortam.push(("CARGO_HTTP_CHECK_REVOKE".into(), "false".into()));
    }
    ortam
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vekil_ortami_tasarimdaki_degiskenler() {
        let ortam = vekil_ortami(8080);
        let vekil = "http://127.0.0.1:8080";

        let bul = |k: &str| -> Option<&str> {
            ortam
                .iter()
                .find(|(anahtar, _)| anahtar == k)
                .map(|(_, d)| d.as_str())
        };

        // Standart vekil değişkenleri (§8.5)
        assert_eq!(bul("HTTPS_PROXY"), Some(vekil));
        assert_eq!(bul("HTTP_PROXY"), Some(vekil));
        assert_eq!(bul("ALL_PROXY"), Some(vekil));
        if cfg!(windows) {
            assert_eq!(bul("https_proxy"), None);
        } else {
            assert_eq!(bul("https_proxy"), Some(vekil));
            assert_eq!(bul("http_proxy"), Some(vekil));
            assert_eq!(bul("no_proxy"), Some("localhost,127.0.0.1,::1"));
        }

        // Yerel sunucu istisnaları
        assert_eq!(bul("NO_PROXY"), Some("localhost,127.0.0.1,::1"));

        // Node.js
        assert_eq!(bul("NODE_USE_ENV_PROXY"), Some("1"));

        // Araçlara özel değişkenler
        assert_eq!(bul("npm_config_https_proxy"), Some(vekil));
        assert_eq!(bul("npm_config_proxy"), Some(vekil));
        assert_eq!(bul("CARGO_HTTP_PROXY"), Some(vekil));
        assert_eq!(bul("PIP_PROXY"), Some(vekil));
        if cfg!(windows) {
            assert_eq!(bul("CARGO_HTTP_CHECK_REVOKE"), Some("false"));
        } else {
            assert_eq!(bul("CARGO_HTTP_CHECK_REVOKE"), None);
        }

        // Git değişkenleri
        assert_eq!(bul("GIT_CONFIG_COUNT"), Some("1"));
        assert_eq!(bul("GIT_CONFIG_KEY_0"), Some("http.proxy"));
        assert_eq!(bul("GIT_CONFIG_VALUE_0"), Some(vekil));
    }

    /// Windows'ta aynı ad iki kez tanımlanmaz (ad büyük/küçük harf ayırmaz).
    #[test]
    fn windowsta_ayni_ad_iki_kez_yok() {
        let ortam = vekil_ortami(8080);
        for (i, (a, _)) in ortam.iter().enumerate() {
            for (b, _) in &ortam[i + 1..] {
                if cfg!(windows) {
                    assert!(!a.eq_ignore_ascii_case(b), "{a} ile {b}");
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }
}
