//! TLS ClientHello okuyucusu (aşama 3 tasarımı §8.2, G7). Araçların tünele
//! gönderdiği ilk el sıkışma iletisinden SNI'yi, ECH uzantısını ve ALPN'yi
//! çıkarır. Yalnızca gözlem içindir; hiçbir şey yanıtlanmaz.

use std::io::Read;

/// Birleştirilen el sıkışma baytlarının üst sınırı.
const EN_COK: usize = 16 * 1024;
const EL_SIKISMA: u8 = 0x16;
const CLIENT_HELLO: u8 = 0x01;
const UZANTI_SNI: u16 = 0x0000;
const UZANTI_ALPN: u16 = 0x0010;
const UZANTI_SURUMLER: u16 = 0x002b;
const UZANTI_ECH: u16 = 0xfe0d;

#[derive(Debug, Clone, Default)]
pub struct ClientHello {
    pub sni: Option<String>,
    pub ech: bool,
    pub alpn: Vec<String>,
    pub tls13: bool,
    pub bayt: usize,
}

#[derive(Debug, Clone)]
pub enum Okuma {
    Tls(ClientHello),
    /// İlk bayt bir TLS el sıkışma kaydı değil.
    TlsDegil(u8),
    Hata(String),
}

pub fn oku(r: &mut impl Read) -> Okuma {
    let mut el_sikisma = Vec::new();
    loop {
        let mut baslik = [0u8; 5];
        if let Err(e) = r.read_exact(&mut baslik) {
            return Okuma::Hata(format!("kayıt başlığı: {e}"));
        }
        if baslik[0] != EL_SIKISMA {
            return Okuma::TlsDegil(baslik[0]);
        }
        let uzunluk = usize::from(u16::from_be_bytes([baslik[3], baslik[4]]));
        if el_sikisma.len() + uzunluk > EN_COK {
            return Okuma::Hata("ClientHello çok büyük".into());
        }
        let mut govde = vec![0u8; uzunluk];
        if let Err(e) = r.read_exact(&mut govde) {
            return Okuma::Hata(format!("kayıt gövdesi: {e}"));
        }
        el_sikisma.extend_from_slice(&govde);
        if el_sikisma.len() >= 4 {
            let ileti = 4
                + (usize::from(el_sikisma[1]) << 16
                    | usize::from(el_sikisma[2]) << 8
                    | usize::from(el_sikisma[3]));
            if el_sikisma.len() >= ileti {
                return match ayristir(&el_sikisma[..ileti]) {
                    Ok(ch) => Okuma::Tls(ch),
                    Err(e) => Okuma::Hata(e),
                };
            }
        }
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

fn ayristir(ileti: &[u8]) -> Result<ClientHello, String> {
    let mut i = Imlec {
        veri: ileti,
        konum: 0,
    };
    if i.u8()? != CLIENT_HELLO {
        return Err("ClientHello değil".into());
    }
    i.al(3)?; // uzunluk
    i.al(2)?; // legacy_version
    i.al(32)?; // random
    i.vektor8()?; // legacy_session_id
    i.vektor16()?; // cipher_suites
    i.vektor8()?; // legacy_compression_methods
    let mut ch = ClientHello {
        bayt: ileti.len(),
        ..ClientHello::default()
    };
    let mut uzantilar = Imlec {
        veri: i.vektor16()?,
        konum: 0,
    };
    while uzantilar.konum < uzantilar.veri.len() {
        let tur = uzantilar.u16()?;
        let mut veri = Imlec {
            veri: uzantilar.vektor16()?,
            konum: 0,
        };
        match tur {
            UZANTI_SNI => {
                let mut liste = Imlec {
                    veri: veri.vektor16()?,
                    konum: 0,
                };
                while liste.konum < liste.veri.len() {
                    let ad_turu = liste.u8()?;
                    let ad = liste.vektor16()?;
                    if ad_turu == 0 {
                        ch.sni = Some(String::from_utf8_lossy(ad).into_owned());
                    }
                }
            }
            UZANTI_ALPN => {
                let mut liste = Imlec {
                    veri: veri.vektor16()?,
                    konum: 0,
                };
                while liste.konum < liste.veri.len() {
                    ch.alpn
                        .push(String::from_utf8_lossy(liste.vektor8()?).into_owned());
                }
            }
            UZANTI_SURUMLER => {
                let surumler = veri.vektor8()?;
                ch.tls13 = surumler.chunks(2).any(|s| s == [0x03, 0x04]);
            }
            UZANTI_ECH => ch.ech = true,
            _ => {}
        }
    }
    Ok(ch)
}
