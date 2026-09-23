//! Kayıt dizininden oturum listesi (1c tasarımı §5.4).
//!
//! Ayrı bir dizin dosyası tutulmaz; kaynak her zaman kayıttır. Her kaydın
//! başından çalışma alanı ve başlık, sonundan son durum okunur.

use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use kalfa_evren::tipler::Role;
use serde::{Deserialize, Serialize};

use crate::kayit::{KayitOgesi, coz};
use crate::kimlik::OturumId;
use crate::protokol::{BitisNedeni, Zarf};

/// Başlık ve çalışma alanı için okunan baş kısım.
const BAS_BAYT: u64 = 16 * 1024;
/// Son durum için okunan son kısım.
const SON_BAYT: u64 = 64 * 1024;
const AZAMI_BASLIK: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum OturumDurumu {
    /// Henüz kullanıcı mesajı yok.
    Bos,
    Tamamlandi,
    IptalEdildi,
    Hata,
    /// Son turun `TurBitti` kaydı yok: Kalfa tur sürerken kapanmış.
    YarimKaldi,
    Okunamadi {
        neden: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OturumOzeti {
    pub yol: PathBuf,
    pub oturum: Option<OturumId>,
    pub calisma_alani: Option<PathBuf>,
    pub baslik: Option<String>,
    /// Son değişiklik zamanı, Unix epoch milisaniyesi.
    pub son_degisiklik_ms: u64,
    pub durum: OturumDurumu,
}

/// Dizindeki `.jsonl` kayıtlarının özetleri, en yeni önce. Dizin yoksa boş
/// liste. Okunamayan kayıt listeden düşmez; `Okunamadi` olarak görünür.
pub fn oturumlari_listele(dizin: &Path) -> io::Result<Vec<OturumOzeti>> {
    let girisler = match fs::read_dir(dizin) {
        Ok(g) => g,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut ozetler = Vec::new();
    for giris in girisler {
        let yol = giris?.path();
        if yol.extension().and_then(|u| u.to_str()) == Some("jsonl") {
            ozetler.push(oturum_ozeti(&yol));
        }
    }
    ozetler.sort_by_key(|a| std::cmp::Reverse(a.son_degisiklik_ms));
    Ok(ozetler)
}

/// Tek kaydın özeti. Hata döndürmez; okunamayan kayıt `Okunamadi` olur.
pub fn oturum_ozeti(yol: &Path) -> OturumOzeti {
    let son_degisiklik_ms = degisiklik_ms(yol);
    match ozet_oku(yol) {
        Ok(o) => OturumOzeti {
            yol: yol.to_path_buf(),
            oturum: Some(o.oturum),
            calisma_alani: o.calisma_alani,
            baslik: o.baslik,
            son_degisiklik_ms,
            durum: o.durum,
        },
        Err(e) => OturumOzeti {
            yol: yol.to_path_buf(),
            oturum: None,
            calisma_alani: None,
            baslik: None,
            son_degisiklik_ms,
            durum: OturumDurumu::Okunamadi {
                neden: e.to_string(),
            },
        },
    }
}

struct Okunan {
    oturum: OturumId,
    calisma_alani: Option<PathBuf>,
    baslik: Option<String>,
    durum: OturumDurumu,
}

fn ozet_oku(yol: &Path) -> io::Result<Okunan> {
    let mut dosya = File::open(yol)?;
    let boyut = dosya.metadata()?.len();
    let (bas, durum) = if boyut <= BAS_BAYT + SON_BAYT {
        let hepsi = coz(&aralik_oku(&mut dosya, 0, boyut)?)?.0;
        let durum = durum_bul(&hepsi, true);
        (hepsi, durum)
    } else {
        let bas = baslangic_kayitlarini_oku(&mut dosya, boyut)?;
        let durum = sondan_durum_bul(&mut dosya, boyut)?;
        (bas, durum)
    };
    let Some(ilk) = bas.first() else {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Kayıt boş."));
    };
    let KayitOgesi::OturumAcildi { calisma_alani, .. } = &ilk.yuk else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Kayıt bir açılış kaydıyla başlamıyor.",
        ));
    };
    Ok(Okunan {
        oturum: ilk.oturum,
        calisma_alani: calisma_alani.clone(),
        baslik: baslik(&bas),
        durum,
    })
}

fn aralik_oku(dosya: &mut File, bas: u64, uzunluk: u64) -> io::Result<Vec<u8>> {
    dosya.seek(SeekFrom::Start(bas))?;
    let mut veri = Vec::new();
    dosya.by_ref().take(uzunluk).read_to_end(&mut veri)?;
    Ok(veri)
}

/// Başlangıç penceresini 16 KiB'da başlatır. İlk açılış JSONL satırı daha
/// uzunsa, onu tamamlayacak newline veya EOF'a dek bloklar halinde genişletir;
/// böylece uzun çalışma alanı/model alanı geçerli oturumu okunamaz yapmaz.
fn baslangic_kayitlarini_oku(dosya: &mut File, boyut: u64) -> io::Result<Vec<Zarf<KayitOgesi>>> {
    let mut veri = aralik_oku(dosya, 0, boyut.min(BAS_BAYT))?;
    let mut okunan = veri.len() as u64;
    while !veri.contains(&b'\n') && okunan < boyut {
        let uzunluk = BAS_BAYT.min(boyut - okunan);
        veri.extend(aralik_oku(dosya, okunan, uzunluk)?);
        okunan += uzunluk;
    }
    satirlari_coz(&veri, okunan == boyut)
}

/// Bir aralıktaki satırları çözer. EOF penceresinde `kayit::coz` ile aynı
/// çökme-kuyruğu kuralı uygulanır; diğer pencerelerde yalnız tam satırlar
/// çözümlenir.
fn satirlari_coz(veri: &[u8], dosya_sonu: bool) -> io::Result<Vec<Zarf<KayitOgesi>>> {
    if dosya_sonu {
        return coz(veri).map(|(kayitlar, _)| kayitlar);
    }

    let mut satirlar: Vec<&[u8]> = veri.split(|b| *b == b'\n').collect();
    // Yeni satır varsa son boş parçayı düşürürüz; yoksa aralık ortasında
    // kesilen satır parçasını düşürürüz.
    let _ = satirlar.pop();
    let mut kayitlar = Vec::new();
    for satir in satirlar {
        if satir.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let zarf = serde_json::from_slice(satir)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        kayitlar.push(zarf);
    }
    Ok(kayitlar)
}

/// Geriye doğru pencereyi satır sınırında başlatır. Böylece iki komşu pencere
/// arasında kalan uzun bir JSONL kaydı kaybolmaz.
fn onceki_satir_baslangici(dosya: &mut File, hedef: u64) -> io::Result<u64> {
    let mut ara = hedef;
    while ara > 0 {
        let bas = ara.saturating_sub(4096);
        let veri = aralik_oku(dosya, bas, ara - bas)?;
        if let Some(i) = veri.iter().rposition(|b| *b == b'\n') {
            return Ok(bas + i as u64 + 1);
        }
        ara = bas;
    }
    Ok(0)
}

/// Büyük kayıtta sondan başlayıp en yeni kullanıcı mesajı veya tur bitişini
/// bulur. Pencereler arasında ilgili satır kaybolmaması için her pencere
/// önceki satır sınırından başlar.
fn sondan_durum_bul(dosya: &mut File, boyut: u64) -> io::Result<OturumDurumu> {
    let mut pencere_sonu = boyut;
    loop {
        let hedef = pencere_sonu.saturating_sub(SON_BAYT);
        let pencere_basi = if hedef == 0 {
            0
        } else {
            onceki_satir_baslangici(dosya, hedef)?
        };
        let veri = aralik_oku(dosya, pencere_basi, pencere_sonu - pencere_basi)?;
        let kayitlar = satirlari_coz(&veri, pencere_sonu == boyut)?;
        for zarf in kayitlar.iter().rev() {
            match &zarf.yuk {
                KayitOgesi::TurBitti { neden } => return Ok(bitisten(neden)),
                KayitOgesi::Mesaj { mesaj } if mesaj.role == Role::User => {
                    return Ok(OturumDurumu::YarimKaldi);
                }
                _ => {}
            }
        }
        if pencere_basi == 0 {
            return Ok(OturumDurumu::Bos);
        }
        pencere_sonu = pencere_basi;
    }
}

fn baslik(kayitlar: &[Zarf<KayitOgesi>]) -> Option<String> {
    let metin = kayitlar.iter().find_map(|z| match &z.yuk {
        KayitOgesi::Mesaj { mesaj } if mesaj.role == Role::User => mesaj.content.clone(),
        _ => None,
    })?;
    let satir = metin.lines().map(str::trim).find(|s| !s.is_empty())?;
    if satir.chars().count() <= AZAMI_BASLIK {
        return Some(satir.to_string());
    }
    let mut kisa: String = satir.chars().take(AZAMI_BASLIK).collect();
    kisa.push_str("···");
    Some(kisa)
}

fn durum_bul(kayitlar: &[Zarf<KayitOgesi>], tamami: bool) -> OturumDurumu {
    let son_mesaj = kayitlar
        .iter()
        .rev()
        .find(|z| matches!(&z.yuk, KayitOgesi::Mesaj { mesaj } if mesaj.role == Role::User))
        .map(|z| z.sira);
    let son_bitis = kayitlar.iter().rev().find_map(|z| match &z.yuk {
        KayitOgesi::TurBitti { neden } => Some((z.sira, neden)),
        _ => None,
    });
    match (son_mesaj, son_bitis) {
        (Some(mesaj), Some((bitis, neden))) if bitis > mesaj => bitisten(neden),
        (None, Some((_, neden))) => bitisten(neden),
        (None, None) if tamami => OturumDurumu::Bos,
        _ => OturumDurumu::YarimKaldi,
    }
}

fn bitisten(neden: &BitisNedeni) -> OturumDurumu {
    match neden {
        BitisNedeni::Normal | BitisNedeni::Sinir { .. } => OturumDurumu::Tamamlandi,
        BitisNedeni::Iptal => OturumDurumu::IptalEdildi,
        BitisNedeni::Hata => OturumDurumu::Hata,
    }
}

fn degisiklik_ms(yol: &Path) -> u64 {
    fs::metadata(yol)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;

    use crate::izin::Kip;
    use crate::kimlik::OturumId;
    use crate::protokol::SEMA_SURUMU;

    fn satir(sira: u64) -> Vec<u8> {
        serde_json::to_vec(&Zarf {
            sema: SEMA_SURUMU,
            oturum: OturumId::yeni(),
            tur: None,
            sira,
            yuk: KayitOgesi::KipDegisti { kip: Kip::Proje },
        })
        .unwrap()
    }

    #[test]
    fn pencere_satir_sinirindaysa_siniri_korur() {
        let ilk = satir(1);
        let ikinci = satir(2);
        let mut veri = ilk;
        veri.push(b'\n');
        let bas = veri.len() as u64;
        veri.extend_from_slice(&ikinci);
        veri.push(b'\n');

        let mut dosya = tempfile::tempfile().unwrap();
        dosya.write_all(&veri).unwrap();
        assert_eq!(onceki_satir_baslangici(&mut dosya, bas).unwrap(), bas);
        let kayitlar = satirlari_coz(&veri[bas as usize..], false).unwrap();
        assert_eq!(kayitlar.len(), 1);
        assert_eq!(kayitlar[0].sira, 2);
    }

    #[test]
    fn pencere_satir_ortasindaysa_satir_baslangicina_geri_gider() {
        let ilk = satir(1);
        let ikinci = satir(2);
        let ilk_uzunluk = ilk.len() as u64;
        let bas = ilk_uzunluk + 1 + (ikinci.len() / 2) as u64;
        let mut veri = ilk;
        veri.push(b'\n');
        veri.extend_from_slice(&ikinci);
        veri.push(b'\n');

        let mut dosya = tempfile::tempfile().unwrap();
        dosya.write_all(&veri).unwrap();
        let baslangic = onceki_satir_baslangici(&mut dosya, bas).unwrap();
        assert_eq!(baslangic, ilk_uzunluk + 1);
        let kayitlar = satirlari_coz(&veri[baslangic as usize..], false).unwrap();
        assert_eq!(kayitlar.len(), 1);
        assert_eq!(kayitlar[0].sira, 2);
    }
}
