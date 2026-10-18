//! Tur özeti (aşama 3 tasarımı §5.5): her tur, değişen dosyaları, çalışan
//! komutları ve EVREN çağrılarını gösteren bir özetle biter.

use serde::{Deserialize, Serialize};

use crate::arac::DosyaDegisiklikleri;
use crate::geri_alma::{DosyaYedegi, GeriAlinamazNedeni, OncekiHal};
use crate::kimlik::{CagriId, TurId};
use crate::protokol::AracDurumu;
use crate::yol::CalismaYolu;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurOzeti {
    pub tur: TurId,
    /// Dosya araçlarıyla değişen dosyalar; yalnızca başarılı ya da sonucu
    /// bilinmeyen yazmalar.
    pub dosyalar: Vec<DosyaOzeti>,
    pub komutlar: Vec<KomutOzeti>,
    pub istekler: IstekOzeti,
    pub guvenlik_uyarisi: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DosyaIslemi {
    Olusturuldu,
    Degisti,
}

/// Yazan bir çağrının bir dosyadaki değişikliği; araç kartında gösterilir
/// (masaüstü arayüzü tasarımı §4). "Başladı" kaydının yedeğinden üretilir;
/// diff oradaki gibi süzülmüştür: yalnız tek dosyalı yazmada, en çok
/// `DIFF_SINIRI` bayt, gizli dosyada hiç yok.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DosyaDegisikligi {
    pub yol: CalismaYolu,
    pub islem: DosyaIslemi,
    pub eklenen: u32,
    pub silinen: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
}

impl From<&DosyaYedegi> for DosyaDegisikligi {
    fn from(y: &DosyaYedegi) -> Self {
        Self {
            yol: y.yol.clone(),
            islem: match y.onceki {
                OncekiHal::Yoktu => DosyaIslemi::Olusturuldu,
                OncekiHal::Saklandi { .. } | OncekiHal::Buyuk { .. } => DosyaIslemi::Degisti,
            },
            eklenen: y.eklenen,
            silinen: y.silinen,
            diff: y.diff.clone(),
        }
    }
}

/// Bir değişikliğin geri alınıp alınamayacağı. Bilgiyi çekirdek verir;
/// arayüz tahmin etmez.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum GeriAlinabilirlik {
    Evet,
    Hayir { neden: GeriAlinamazNedeni },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DosyaOzeti {
    pub yol: CalismaYolu,
    pub islem: DosyaIslemi,
    pub eklenen: u32,
    pub silinen: u32,
    /// Yazan çağrının sonucu bilinmiyor (Kalfa yazma sırasında kapandı).
    pub sonucu_bilinmiyor: bool,
    pub geri_alinabilir: GeriAlinabilirlik,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AgHostOzeti {
    pub host: String,
    pub port: u16,
    pub baglanti_sayisi: u32,
    pub giden_bayt: u64,
    pub gelen_bayt: u64,
    pub reddedilenler: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AgKomutOzeti {
    pub hostlar: Vec<AgHostOzeti>,
    pub toplam_giden: u64,
    pub toplam_gelen: u64,
    pub reddedilen_sayisi: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KomutOzeti {
    pub cagri: CagriId,
    pub komut: String,
    pub durum: AracDurumu,
    /// Komut çalışırken değişen dosyalar; geri alınamaz (K2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degisiklikler: Option<DosyaDegisiklikleri>,
    /// Komutun vekilden geçen ağ özeti (§8.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ag: Option<AgKomutOzeti>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IstekOzeti {
    pub sayi: u32,
    pub istem: u64,
    pub tamamlama: u64,
    pub dusunce: u64,
}

/// Yazan bir çağrının bir dosyası.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YazilanDosya {
    pub yol: CalismaYolu,
    /// Yazmadan önce dosya yoktu.
    pub olusturuldu: bool,
    pub eklenen: u32,
    pub silinen: u32,
    pub geri_alinabilir: GeriAlinabilirlik,
    pub diff: Option<String>,
}

#[derive(Debug)]
struct Yazma {
    cagri: CagriId,
    durum: AracDurumu,
    dosyalar: Vec<YazilanDosya>,
}

/// Bir `TurOzeti` olayındaki bütün diff'lerin JSON dizgisi olarak bayt üst sınırı.
/// Tekil `TurOzeti` olayının 8 MiB'lık olay aktarım bütçesini aşmasını önler.
pub const AZAMI_TUR_OZETI_DIFF_BAYT: usize = 2 * 1024 * 1024;

fn json_bayti(bayt: u8) -> usize {
    match bayt {
        b'"' | b'\\' | b'\n' | b'\r' | b'\t' | 8 | 12 => 2,
        0..=31 => 6,
        _ => 1,
    }
}

fn yama_basligini_atla(diff: &str) -> &str {
    if let Some(indeks) = diff.find("\n@@ ") {
        let bas = &diff[..indeks];
        if bas.starts_with("--- ") || bas.contains("\n--- ") {
            return &diff[indeks + 1..];
        }
    }
    diff
}

/// Tur boyunca özetin bilgisini toplar; turun başında sıfırlanır.
#[derive(Debug, Default)]
pub struct TurBiriktirici {
    yazmalar: Vec<Yazma>,
    komutlar: Vec<KomutOzeti>,
    istekler: IstekOzeti,
    guvenlik_uyarisi: u32,
}

impl TurBiriktirici {
    /// Yazan çağrı başladı; durumu `arac_bitti`'yle gelir.
    pub fn yazma_basladi(&mut self, cagri: CagriId, dosyalar: Vec<YazilanDosya>) {
        self.yazmalar.push(Yazma {
            cagri,
            durum: AracDurumu::SonucuBilinmiyor,
            dosyalar,
        });
    }

    pub fn komut_basladi(&mut self, cagri: CagriId, komut: String) {
        self.komutlar.push(KomutOzeti {
            cagri,
            komut,
            durum: AracDurumu::SonucuBilinmiyor,
            degisiklikler: None,
            ag: None,
        });
    }

    /// Kabuk çağrısının değiştirdiği dosyalar.
    pub fn komut_degisiklikleri(&mut self, cagri: CagriId, d: &DosyaDegisiklikleri) {
        for k in self.komutlar.iter_mut().filter(|k| k.cagri == cagri) {
            k.degisiklikler = Some(d.clone());
        }
    }

    /// Kabuk çağrısının vekilden geçen ağ özeti.
    pub fn komut_ag_ozeti(&mut self, cagri: CagriId, ag: &AgKomutOzeti) {
        for k in self.komutlar.iter_mut().filter(|k| k.cagri == cagri) {
            k.ag = Some(ag.clone());
        }
    }

    pub fn arac_bitti(&mut self, cagri: CagriId, durum: AracDurumu) {
        for y in self.yazmalar.iter_mut().filter(|y| y.cagri == cagri) {
            y.durum = durum;
        }
        for k in self.komutlar.iter_mut().filter(|k| k.cagri == cagri) {
            k.durum = durum;
        }
    }

    pub fn istek(&mut self) {
        self.istekler.sayi = self.istekler.sayi.saturating_add(1);
    }

    pub fn kullanim(&mut self, istem: u64, tamamlama: u64, dusunce: u64) {
        let i = &mut self.istekler;
        i.istem = i.istem.saturating_add(istem);
        i.tamamlama = i.tamamlama.saturating_add(tamamlama);
        i.dusunce = i.dusunce.saturating_add(dusunce);
    }

    pub fn guvenlik_uyarisi(&mut self, sayi: usize) {
        let sayi = u32::try_from(sayi).unwrap_or(u32::MAX);
        self.guvenlik_uyarisi = self.guvenlik_uyarisi.saturating_add(sayi);
    }

    /// Özeti üretir. `veri_silindi`: turun önceki hâlleri depo sınırı yüzünden
    /// silindi (aşama 3 tasarımı §5.2).
    pub fn bitir(self, tur: TurId, veri_silindi: bool) -> TurOzeti {
        let mut dosyalar: Vec<DosyaOzeti> = Vec::new();
        let yazilan = self
            .yazmalar
            .iter()
            .filter(|y| matches!(y.durum, AracDurumu::Basarili | AracDurumu::SonucuBilinmiyor));
        for yazma in yazilan {
            let bilinmiyor = yazma.durum == AracDurumu::SonucuBilinmiyor;
            for d in &yazma.dosyalar {
                match dosyalar.iter_mut().find(|o| o.yol == d.yol) {
                    Some(o) => {
                        o.eklenen = o.eklenen.saturating_add(d.eklenen);
                        o.silinen = o.silinen.saturating_add(d.silinen);
                        o.sonucu_bilinmiyor |= bilinmiyor;
                        if o.geri_alinabilir == GeriAlinabilirlik::Evet {
                            o.geri_alinabilir = d.geri_alinabilir;
                        }
                        if let Some(yeni_diff) = &d.diff {
                            match &mut o.diff {
                                Some(mevcut) => {
                                    if !mevcut.ends_with('\n') {
                                        mevcut.push('\n');
                                    }
                                    mevcut.push_str(yama_basligini_atla(yeni_diff));
                                }
                                None => o.diff = Some(yeni_diff.clone()),
                            }
                        }
                    }
                    None => dosyalar.push(DosyaOzeti {
                        yol: d.yol.clone(),
                        islem: if d.olusturuldu {
                            DosyaIslemi::Olusturuldu
                        } else {
                            DosyaIslemi::Degisti
                        },
                        eklenen: d.eklenen,
                        silinen: d.silinen,
                        sonucu_bilinmiyor: bilinmiyor,
                        geri_alinabilir: d.geri_alinabilir,
                        diff: d.diff.clone(),
                    }),
                }
            }
        }
        let mut kalan_diff = AZAMI_TUR_OZETI_DIFF_BAYT;
        for d in &mut dosyalar {
            if let Some(diff) = &mut d.diff {
                // JSON'un iki tırnağı ve kaçışları da bütçeye dahildir.
                // Yalnız tam satırlar tutulur; UTF-8 karakterleri bölünmez.
                let mut json_bayt = 2usize;
                let mut kesim = 0;
                for satir in diff.split_inclusive('\n') {
                    let satir_bayt = satir.bytes().map(json_bayti).sum::<usize>();
                    if json_bayt.saturating_add(satir_bayt) > kalan_diff {
                        break;
                    }
                    json_bayt += satir_bayt;
                    kesim += satir.len();
                }
                if kesim == diff.len() && json_bayt <= kalan_diff {
                    kalan_diff -= json_bayt;
                } else {
                    if kesim > 0 && diff[..kesim].contains("@@ ") {
                        diff.truncate(kesim);
                    } else {
                        d.diff = None;
                    }
                    kalan_diff = 0;
                }
            }
        }
        if veri_silindi {
            veri_silindi_isle(&mut dosyalar);
        }
        TurOzeti {
            tur,
            dosyalar,
            komutlar: self.komutlar,
            istekler: self.istekler,
            guvenlik_uyarisi: self.guvenlik_uyarisi,
        }
    }
}

/// Önceki hâlleri silinen turun değişen dosyaları artık geri alınamaz.
/// Turun oluşturduğu dosyalar içerik gerektirmeden silinebildiği için geri
/// alınabilir kalır (aşama 3 tasarımı §5.7).
pub fn veri_silindi_isle(dosyalar: &mut [DosyaOzeti]) {
    for d in dosyalar {
        if d.geri_alinabilir == GeriAlinabilirlik::Evet && d.islem == DosyaIslemi::Degisti {
            d.geri_alinabilir = GeriAlinabilirlik::Hayir {
                neden: GeriAlinamazNedeni::VeriSilindi,
            };
        }
    }
}
