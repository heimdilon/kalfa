//! Tur özeti (aşama 3 tasarımı §5.5): her tur, değişen dosyaları, çalışan
//! komutları ve EVREN çağrılarını gösteren bir özetle biter.

use serde::{Deserialize, Serialize};

use crate::arac::DosyaDegisiklikleri;
use crate::geri_alma::GeriAlinamazNedeni;
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KomutOzeti {
    pub cagri: CagriId,
    pub komut: String,
    pub durum: AracDurumu,
    /// Komut çalışırken değişen dosyalar; geri alınamaz (K2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degisiklikler: Option<DosyaDegisiklikleri>,
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
}

#[derive(Debug)]
struct Yazma {
    cagri: CagriId,
    durum: AracDurumu,
    dosyalar: Vec<YazilanDosya>,
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
        });
    }

    /// Kabuk çağrısının değiştirdiği dosyalar.
    pub fn komut_degisiklikleri(&mut self, cagri: CagriId, d: &DosyaDegisiklikleri) {
        for k in self.komutlar.iter_mut().filter(|k| k.cagri == cagri) {
            k.degisiklikler = Some(d.clone());
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
                    }),
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
