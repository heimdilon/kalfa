//! Vekil için çözülen IP adreslerinin denetimi (aşama 3 tasarımı §8.2).
//!
//! Her zaman yasak olanlar:
//! - Geri döngü (loopback: 127.0.0.0/8, ::1)
//! - Bağlantı yerel (link-local: 169.254.0.0/16, fe80::/10; bulut meta verisi dahil)
//! - Çok noktaya yayın (multicast: 224.0.0.0/4, ff00::/8)
//! - Belirtilmemiş (unspecified: 0.0.0.0, ::)
//! - Yayın (broadcast: 255.255.255.255)
//!
//! Özel ağ adresleri (10/8, 172.16/12, 192.168/16, 100.64/10, fc00::/7)
//! yalnızca `ozel_ag: true` işaretiyle izinlidir.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use serde::{Deserialize, Serialize};

/// Bir IP adresinin reddedilme nedeni.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdresReddi {
    /// Geri döngü, bağlantı yerel, çok noktaya yayın ya da belirtilmemiş adres.
    Yerel,
    /// Özel ağ adresi (işaretsiz host için yasak).
    OzelAg,
}

impl fmt::Display for AdresReddi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yerel => write!(f, "yerel ya da yasak bir adrese çözüldü"),
            Self::OzelAg => write!(f, "özel ağ adresine çözüldü"),
        }
    }
}

impl std::error::Error for AdresReddi {}

/// Verilen IP adresinin çıkış vekilinden geçmesine izin var mı denetler.
pub fn adres_izinli_mi(ip: IpAddr, ozel_ag: bool) -> Result<(), AdresReddi> {
    match ip {
        IpAddr::V4(v4) => ipv4_izinli_mi(v4, ozel_ag),
        IpAddr::V6(v6) => {
            // IPv4'e eşlenmiş IPv6 (::ffff:a.b.c.d) IPv4 olarak denetlenir.
            if let Some(v4) = v6.to_ipv4_mapped() {
                return ipv4_izinli_mi(v4, ozel_ag);
            }
            // IPv4 uyumlu IPv6 (::a.b.c.d, kullanımdan kalkan RFC 4291)
            if let Some(v4) = v6
                .to_ipv4()
                .filter(|_| !v6.is_loopback() && !v6.is_unspecified())
            {
                return ipv4_izinli_mi(v4, ozel_ag);
            }
            ipv6_izinli_mi(v6, ozel_ag)
        }
    }
}

fn ipv4_izinli_mi(ip: Ipv4Addr, ozel_ag: bool) -> Result<(), AdresReddi> {
    let o = ip.octets();

    // 1. Her zaman yasak olanlar:
    // 0.0.0.0/8 (Bu ağ / RFC 1122)
    if o[0] == 0 {
        return Err(AdresReddi::Yerel);
    }
    // Geri döngü (127.0.0.0/8)
    if ip.is_loopback() {
        return Err(AdresReddi::Yerel);
    }
    // Bağlantı yerel (169.254.0.0/16; 169.254.169.254 bulut meta verisi dahil)
    if ip.is_link_local() {
        return Err(AdresReddi::Yerel);
    }
    // Çok noktaya yayın (224.0.0.0/4)
    if ip.is_multicast() {
        return Err(AdresReddi::Yerel);
    }
    // Yayın (255.255.255.255)
    if ip.is_broadcast() {
        return Err(AdresReddi::Yerel);
    }

    // 2. Özel ağ adresleri:
    // 10.0.0.0/8
    let is_10 = o[0] == 10;
    // 172.16.0.0/12 (172.16.0.0 - 172.31.255.255)
    let is_172 = o[0] == 172 && (o[1] >= 16 && o[1] <= 31);
    // 192.168.0.0/16
    let is_192 = o[0] == 192 && o[1] == 168;
    // 100.64.0.0/10 (CGNAT / RFC 6598: 100.64.0.0 - 100.127.255.255)
    let is_100_64 = o[0] == 100 && (o[1] >= 64 && o[1] <= 127);

    if is_10 || is_172 || is_192 || is_100_64 {
        if !ozel_ag {
            return Err(AdresReddi::OzelAg);
        }
        return Ok(());
    }

    Ok(())
}

fn ipv6_izinli_mi(ip: Ipv6Addr, ozel_ag: bool) -> Result<(), AdresReddi> {
    let s = ip.segments();

    // 1. Her zaman yasak olanlar:
    // Belirtilmemiş (::)
    if ip.is_unspecified() {
        return Err(AdresReddi::Yerel);
    }
    // Geri döngü (::1)
    if ip.is_loopback() {
        return Err(AdresReddi::Yerel);
    }
    // Çok noktaya yayın (ff00::/8)
    if ip.is_multicast() {
        return Err(AdresReddi::Yerel);
    }
    // Bağlantı yerel (fe80::/10)
    if ip.is_unicast_link_local() || (s[0] & 0xffc0) == 0xfe80 {
        return Err(AdresReddi::Yerel);
    }

    // 2. Özel ağ adresleri:
    // Benzersiz yerel adresler (fc00::/7 -> fc00:: ve fd00::)
    let is_ula = (s[0] & 0xfe00) == 0xfc00;
    if is_ula {
        if !ozel_ag {
            return Err(AdresReddi::OzelAg);
        }
        return Ok(());
    }

    Ok(())
}
