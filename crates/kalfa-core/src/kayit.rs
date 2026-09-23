//! Oturum kaydı: yalnızca sona eklenen JSONL (spec §6.6).
//!
//! Depo içinde değil, uygulama veri dizininde tutulur (yolu uygulama verir).
//! Yan etkili bir araçtan önce yazılan "başladı" kaydı diske ulaşmadan
//! dönülmez (`yaz_kalici`). Çökmede yarım kalan son satır, kayıt devam etmek
//! üzere açılırken kesilir; yoksa yeni kayıtlar o yarım satıra yapışır ve bir
//! sonraki açılışta kayıt ortasından bozuk görünürdü.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::Path;

use kalfa_evren::KrediGozlemi;
use kalfa_evren::tipler::ChatMessage;
use serde::{Deserialize, Serialize};

use crate::izin::Kip;
use crate::kimlik::CagriId;
use crate::protokol::{AracDurumu, BitisNedeni, Komut, Zarf};

/// Komut kaydının wire türü. Geçmişteki şart kabul komutu yalnızca kayıt
/// uyumluluğu için okunur; bu kayıt EVREN'in kabulü onayladığını göstermez.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KayitKomutu {
    Guncel(Komut),
    EskiSartKabulIstegi(EskiSartKabulKomutu),
}

/// Kayıtlarda tutulmuş eski şart kabul isteği; yeni `Komut` API'sinin parçası
/// değildir.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum EskiSartKabulKomutu {
    #[serde(rename = "SartlariKabulEt", skip_serializing)]
    SartKabulIstegi { surum: u32 },
}

impl From<Komut> for KayitKomutu {
    fn from(komut: Komut) -> Self {
        Self::Guncel(komut)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum KayitOgesi {
    OturumAcildi {
        model: String,
        kip: Kip,
    },
    Komut {
        komut: KayitKomutu,
    },
    Mesaj {
        mesaj: ChatMessage,
    },
    AracBasladi {
        cagri: CagriId,
        model_cagri_id: String,
        arac: String,
    },
    AracBitti {
        cagri: CagriId,
        durum: AracDurumu,
    },
    Istek {
        amac: String,
        istek_id: Option<String>,
        kredi: Option<KrediGozlemi>,
    },
    KipDegisti {
        kip: Kip,
    },
    ModelDegisti {
        model: String,
    },
    TurBitti {
        neden: BitisNedeni,
    },
    /// Oturum, kullanıcının şart penceresinde kabul ettiği sürümle sürdü.
    SartKabulEdildi {
        surum: u32,
    },
}

/// Kaydın yazıldığı yer. Dosya dışındaki hedefler, testlerde yazma hatası
/// benzetmek içindir.
pub trait KayitHedefi: Send {
    /// Satırı sona ekler ve işletim sistemine iletir.
    fn ekle(&mut self, satir: &[u8]) -> io::Result<()>;
    /// Şimdiye kadar eklenenleri kalıcı depolamaya yazar.
    fn kalici_yap(&mut self) -> io::Result<()>;
}

impl KayitHedefi for File {
    fn ekle(&mut self, satir: &[u8]) -> io::Result<()> {
        self.write_all(satir)?;
        self.flush()
    }

    fn kalici_yap(&mut self) -> io::Result<()> {
        self.sync_data()
    }
}

pub struct OturumKaydi {
    hedef: Box<dyn KayitHedefi>,
}

impl OturumKaydi {
    /// Yeni kayıt oluşturur; dosya zaten varsa hata döner.
    pub fn olustur(yol: &Path) -> io::Result<Self> {
        if let Some(ust) = yol.parent() {
            fs::create_dir_all(ust)?;
        }
        let mut secenek = OpenOptions::new();
        secenek.create_new(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            secenek.mode(0o600);
        }
        Ok(Self::hedefle(Box::new(secenek.open(yol)?)))
    }

    /// Var olan kaydı devam etmek üzere açar ve onarır: çökmede yarım kalan
    /// son satır kesilir, son geçerli satırın satır sonu eksikse eklenir.
    pub fn ac(yol: &Path) -> io::Result<Self> {
        let veri = fs::read(yol)?;
        let (_, gecerli) = coz(&veri)?;
        let mut dosya = OpenOptions::new().read(true).write(true).open(yol)?;
        dosya.set_len(gecerli as u64)?;
        dosya.seek(SeekFrom::End(0))?;
        if gecerli > 0 && veri[gecerli - 1] != b'\n' {
            dosya.write_all(b"\n")?;
        }
        dosya.sync_data()?;
        Ok(Self::hedefle(Box::new(dosya)))
    }

    pub fn hedefle(hedef: Box<dyn KayitHedefi>) -> Self {
        Self { hedef }
    }

    pub fn yaz(&mut self, zarf: &Zarf<KayitOgesi>) -> io::Result<()> {
        let mut satir = serde_json::to_vec(zarf).map_err(io::Error::other)?;
        satir.push(b'\n');
        self.hedef.ekle(&satir)
    }

    pub fn yaz_kalici(&mut self, zarf: &Zarf<KayitOgesi>) -> io::Result<()> {
        self.yaz(zarf)?;
        self.hedef.kalici_yap()
    }

    /// Kaydı okur. Çökme sırasında yarım yazılmış **son** satır atlanır;
    /// ortadaki bozuk bir satır kaydın güvenilmez olduğunu gösterir ve hatadır.
    pub fn oku(yol: &Path) -> io::Result<Vec<Zarf<KayitOgesi>>> {
        Ok(coz(&fs::read(yol)?)?.0)
    }
}

/// Kaydı çözer. İkinci değer, son geçerli satırın bittiği bayttır (satır sonu
/// dahil); `ac` dosyayı bu uzunluğa keser.
fn coz(veri: &[u8]) -> io::Result<(Vec<Zarf<KayitOgesi>>, usize)> {
    let mut kayitlar = Vec::new();
    let mut gecerli = 0usize;
    let mut bas = 0usize;
    let mut satir_no = 0usize;
    while bas < veri.len() {
        let (satir, sonraki) = match veri[bas..].iter().position(|b| *b == b'\n') {
            Some(i) => (&veri[bas..bas + i], bas + i + 1),
            None => (&veri[bas..], veri.len()),
        };
        if !satir.iter().all(u8::is_ascii_whitespace) {
            satir_no += 1;
            match serde_json::from_slice(satir) {
                Ok(zarf) => {
                    kayitlar.push(zarf);
                    gecerli = sonraki;
                }
                // Çökmede yarım kalan son satır: atlanır, kayıt burada biter.
                Err(_) if veri[sonraki..].iter().all(u8::is_ascii_whitespace) => break,
                Err(e) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("oturum kaydının {satir_no}. satırı bozuk: {e}"),
                    ));
                }
            }
        }
        bas = sonraki;
    }
    Ok((kayitlar, gecerli))
}
