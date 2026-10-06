//! Ağ çıkış izin listesi ve host çözümlemesi (aşama 3 tasarımı §8.3, §8.4).
//!
//! Kurallar:
//! - Tam eşleşme (joker karakteri yoktur).
//! - Varsayılan liste sekiz kritik indirme/paket hostunu içerir (port 443).
//! - Kullanıcı listesi kalıcıdır; özel ağ işaretini taşıyabilir.
//! - Çağrıya özel liste yalnızca o çağrı için geçerlidir; özel ağ işareti taşıyamaz.
//! - Liste özeti profil özetine girer ve kararlıdır.

use std::collections::BTreeMap;
use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Varsayılan izin verilen hostlar (tasarım §8.3; port 443).
///
/// Paket kayıtları ve GitHub indirme uçlarıdır. `github.com` ve `upload.pypi.org`
/// gibi yükleme veya geniş etki uçları varsayılan listede yer almaz.
pub const VARSAYILAN_HOSTLAR: &[&str] = &[
    "index.crates.io",
    "static.crates.io",
    "registry.npmjs.org",
    "pypi.org",
    "files.pythonhosted.org",
    "codeload.github.com",
    "objects.githubusercontent.com",
    "raw.githubusercontent.com",
];

/// Host kuralının kaynağı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum HostKaynagi {
    Varsayilan = 0,
    Kullanici = 1,
    Cagri = 2,
}

/// İzin verilen bir host ve port kuralı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostKurali {
    pub host: String,
    pub port: u16,
    pub ozel_ag: bool,
    pub kaynak: HostKaynagi,
}

/// Host ayrıştırma ve doğrulama hataları.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListeHatasi {
    BosHost,
    GecersizKarakter(char),
    GecersizEtiket(String),
    CokUzunHost,
    GecersizPort(String),
    JokerKabulEdilmez,
    GecersizIpv6(String),
}

impl fmt::Display for ListeHatasi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BosHost => write!(f, "host adı boş olamaz"),
            Self::GecersizKarakter(c) => write!(f, "geçersiz karakter: '{c}'"),
            Self::GecersizEtiket(e) => write!(f, "geçersiz etiket: {e}"),
            Self::CokUzunHost => write!(f, "host adı veya etiketi izin verilen uzunluğu aşıyor"),
            Self::GecersizPort(p) => write!(f, "geçersiz port: '{p}' (1-65535 arası olmalı)"),
            Self::JokerKabulEdilmez => write!(f, "joker karakter (*) kabul edilmez"),
            Self::GecersizIpv6(e) => write!(f, "geçersiz IPv6 adresi: {e}"),
        }
    }
}

impl std::error::Error for ListeHatasi {}

/// `host` ya da `host:port` metnini doğrular ve ayrıştırır.
///
/// Port verilmezse varsayılan 443'tür. Host küçük harfe çevrilir.
/// IPv4 ve köşeli parantezli IPv6 (`[::1]` ya da `[::1]:8080`) kabul edilir.
pub fn ag_hostu_coz(s: &str) -> Result<(String, u16), ListeHatasi> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ListeHatasi::BosHost);
    }
    if s.contains('*') {
        return Err(ListeHatasi::JokerKabulEdilmez);
    }

    if s.starts_with('[') {
        // Köşeli parantezli IPv6
        let kapanis = s
            .find(']')
            .ok_or_else(|| ListeHatasi::GecersizIpv6("kapanmamış parantez".into()))?;
        let ip_metni = &s[1..kapanis];
        ip_metni
            .parse::<Ipv6Addr>()
            .map_err(|e| ListeHatasi::GecersizIpv6(e.to_string()))?;

        let kalan = &s[kapanis + 1..];
        let port = if kalan.is_empty() {
            443
        } else if let Some(port_str) = kalan.strip_prefix(':') {
            let p: u16 = port_str
                .parse()
                .map_err(|_| ListeHatasi::GecersizPort(port_str.into()))?;
            if p == 0 {
                return Err(ListeHatasi::GecersizPort(port_str.into()));
            }
            p
        } else {
            return Err(ListeHatasi::GecersizKarakter(
                kalan.chars().next().unwrap_or(' '),
            ));
        };

        Ok((format!("[{}]", ip_metni.to_lowercase()), port))
    } else {
        // Domain adı veya IPv4
        let (host_kismi, port) = if let Some((h, port_str)) = s.rsplit_once(':') {
            if h.contains(':') {
                // Parantezsiz IPv6 gibi çoklu iki nokta kabul edilmez
                return Err(ListeHatasi::GecersizKarakter(':'));
            }
            let p: u16 = port_str
                .parse()
                .map_err(|_| ListeHatasi::GecersizPort(port_str.into()))?;
            if p == 0 {
                return Err(ListeHatasi::GecersizPort(port_str.into()));
            }
            (h, p)
        } else {
            (s, 443)
        };

        let host_kismi = host_kismi.trim();
        if host_kismi.is_empty() {
            return Err(ListeHatasi::BosHost);
        }
        if host_kismi.len() > 253 {
            return Err(ListeHatasi::CokUzunHost);
        }

        // IPv4 adresi mi?
        if host_kismi.parse::<Ipv4Addr>().is_ok() {
            return Ok((host_kismi.to_string(), port));
        }

        // Alan adı denetimi
        let host_kucuk = host_kismi.to_lowercase();
        for c in host_kucuk.chars() {
            if !c.is_ascii_alphanumeric() && c != '-' && c != '.' {
                return Err(ListeHatasi::GecersizKarakter(c));
            }
        }

        if host_kucuk.starts_with('.') || host_kucuk.ends_with('.') {
            return Err(ListeHatasi::GecersizEtiket(
                "nokta ile başlayamaz veya bitemez".into(),
            ));
        }

        for etiket in host_kucuk.split('.') {
            if etiket.is_empty() {
                return Err(ListeHatasi::GecersizEtiket("boş etiket".into()));
            }
            if etiket.len() > 63 {
                return Err(ListeHatasi::CokUzunHost);
            }
            if etiket.starts_with('-') || etiket.ends_with('-') {
                return Err(ListeHatasi::GecersizEtiket(
                    "etiket tire ile başlayamaz veya bitemez".into(),
                ));
            }
        }

        Ok((host_kucuk, port))
    }
}

/// Ağ çıkışı için doğrulanmış ve birleştirilmiş izin listesi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IzinListesi {
    kurallar: Vec<HostKurali>,
}

impl IzinListesi {
    /// Varsayılan listeyi, kullanıcı listesini ve çağrıya özel hostları birleştirir.
    ///
    /// Çağrıya özel hostlarda `ozel_ag` her zaman `false`'tur.
    pub fn yeni(kullanici: &[HostKurali], cagri: &[(String, u16)]) -> Self {
        let mut harita: BTreeMap<(String, u16), HostKurali> = BTreeMap::new();

        // 1. Varsayılan hostlar
        for &host in VARSAYILAN_HOSTLAR {
            let h = host.to_lowercase();
            harita.insert(
                (h.clone(), 443),
                HostKurali {
                    host: h,
                    port: 443,
                    ozel_ag: false,
                    kaynak: HostKaynagi::Varsayilan,
                },
            );
        }

        // 2. Kullanıcı hostları (varsayılanı veya ozel_ag ayarını ezebilir)
        for k in kullanici {
            let h = k.host.to_lowercase();
            harita.insert(
                (h.clone(), k.port),
                HostKurali {
                    host: h,
                    port: k.port,
                    ozel_ag: k.ozel_ag,
                    kaynak: HostKaynagi::Kullanici,
                },
            );
        }

        // 3. Çağrıya özel hostlar (varsayılanda veya kullanıcıda yoksa eklenir; ozel_ag her zaman false)
        for (h, p) in cagri {
            let h = h.to_lowercase();
            let key = (h.clone(), *p);
            harita.entry(key).or_insert(HostKurali {
                host: h,
                port: *p,
                ozel_ag: false,
                kaynak: HostKaynagi::Cagri,
            });
        }

        Self {
            kurallar: harita.into_values().collect(),
        }
    }

    /// Host ve porta tam eşleşen kuralı arar.
    pub fn bul(&self, host: &str, port: u16) -> Option<&HostKurali> {
        let h = host.to_lowercase();
        self.kurallar.iter().find(|k| k.host == h && k.port == port)
    }

    /// Liste kurallarının dilimi.
    pub fn kurallar(&self) -> &[HostKurali] {
        &self.kurallar
    }

    /// Listenin kararlı SHA-256 özeti (ilk 16 onaltılık karakter).
    ///
    /// Profil özetine ve mühre girer. Kuralların girdi sırasından etkilenmez.
    pub fn ozet(&self) -> String {
        let mut h = Sha256::new();
        for k in &self.kurallar {
            h.update(k.host.as_bytes());
            h.update([0]);
            h.update(k.port.to_le_bytes());
            h.update([k.ozel_ag as u8]);
            h.update([k.kaynak as u8]);
            h.update([0]);
        }
        hex::encode(h.finalize())[..16].to_string()
    }
}
