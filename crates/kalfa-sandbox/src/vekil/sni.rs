//! TLS ClientHello ayrıştırıcısı ve SNI denetimi (aşama 3 tasarımı §8.2, G7).
//!
//! Tünelin ilk kaydı bir TLS el sıkışmasıysa ClientHello'dan SNI (Server Name Indication)
//! çıkarılır (en çok 16 KiB).
//!
//! İlk kayıt TLS değilse akış olduğu gibi taşınır.

use std::fmt;

const EN_COK_BAYT: usize = 16 * 1024;
const KAYIT_EL_SIKISMA: u8 = 0x16;
const ILETI_CLIENT_HELLO: u8 = 0x01;
const UZANTI_SNI: u16 = 0x0000;

/// SNI denetiminin sonucu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SniSonucu {
    /// TLS ClientHello başarıyla ayrıştırıldı.
    Sni(Option<String>),
    /// İlk bayt TLS el sıkışma kaydı değil (düz TCP, HTTP vb.).
    TlsDegil,
    /// Bozuk ya da sınırı aşan kayıt.
    Bozuk(String),
}

impl fmt::Display for SniSonucu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sni(Some(s)) => write!(f, "SNI: {s}"),
            Self::Sni(None) => write!(f, "SNI yok"),
            Self::TlsDegil => write!(f, "TLS değil"),
            Self::Bozuk(h) => write!(f, "bozuk TLS: {h}"),
        }
    }
}

/// Verilen baytlardan ClientHello'yu ve SNI'yi ayrıştırır.
///
/// Başarılıysa `SniSonucu::Sni`, TLS değilse `SniSonucu::TlsDegil` döner.
pub fn sni_ayristir(kayit: &[u8]) -> SniSonucu {
    if kayit.is_empty() {
        return SniSonucu::Bozuk("boş veri".into());
    }

    // İlk bayt 0x16 (Handshake) değilse TLS sayılmaz
    if kayit[0] != KAYIT_EL_SIKISMA {
        return SniSonucu::TlsDegil;
    }

    if kayit.len() < 5 {
        return SniSonucu::Bozuk("eksik TLS başlığı".into());
    }

    let uzunluk = usize::from(u16::from_be_bytes([kayit[3], kayit[4]]));
    if uzunluk > EN_COK_BAYT || kayit.len() < 5 + uzunluk {
        if uzunluk > EN_COK_BAYT {
            return SniSonucu::Bozuk("ClientHello boyutu 16 KiB sınırını aşıyor".into());
        }
        return SniSonucu::Bozuk("eksik TLS gövdesi".into());
    }

    let govde = &kayit[5..5 + uzunluk];
    match client_hello_sni(govde) {
        Ok(sni) => SniSonucu::Sni(sni),
        Err(e) => SniSonucu::Bozuk(e),
    }
}

struct Imlec<'a> {
    veri: &'a [u8],
    konum: usize,
}

impl<'a> Imlec<'a> {
    fn al(&mut self, n: usize) -> Result<&'a [u8], String> {
        let son = self.konum.checked_add(n).ok_or("taşma")?;
        let parca = self.veri.get(self.konum..son).ok_or("kısa ileti")?;
        self.konum = son;
        Ok(parca)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.al(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, String> {
        let b = self.al(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn vektor8(&mut self) -> Result<&'a [u8], String> {
        let n = usize::from(self.u8()?);
        self.al(n)
    }

    fn vektor16(&mut self) -> Result<&'a [u8], String> {
        let n = usize::from(self.u16()?);
        self.al(n)
    }
}

fn client_hello_sni(ileti: &[u8]) -> Result<Option<String>, String> {
    let mut i = Imlec {
        veri: ileti,
        konum: 0,
    };
    if i.u8()? != ILETI_CLIENT_HELLO {
        return Err("ClientHello değil".into());
    }
    // 3 bayt handshake uzunluğu
    i.al(3)?;
    // legacy_version (2 bayt)
    i.al(2)?;
    // random (32 bayt)
    i.al(32)?;
    // legacy_session_id
    i.vektor8()?;
    // cipher_suites
    i.vektor16()?;
    // legacy_compression_methods
    i.vektor8()?;

    // Uzantılar var mı?
    if i.konum >= i.veri.len() {
        return Ok(None);
    }

    let uzantilar_bayt = i.vektor16()?;
    let mut uzantilar = Imlec {
        veri: uzantilar_bayt,
        konum: 0,
    };

    while uzantilar.konum < uzantilar.veri.len() {
        let tur = uzantilar.u16()?;
        let mut veri = Imlec {
            veri: uzantilar.vektor16()?,
            konum: 0,
        };
        if tur == UZANTI_SNI {
            let mut liste = Imlec {
                veri: veri.vektor16()?,
                konum: 0,
            };
            while liste.konum < liste.veri.len() {
                let ad_turu = liste.u8()?;
                let ad = liste.vektor16()?;
                if ad_turu == 0 {
                    let s = String::from_utf8_lossy(ad).trim().to_lowercase();
                    return Ok(Some(s));
                }
            }
        }
    }

    Ok(None)
}
