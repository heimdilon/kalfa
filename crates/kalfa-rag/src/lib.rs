//! `kalfa-rag`: çalışma alanında Türkçe anlamsal arama (aşama 4 tasarımı
//! §11). Kalfa'yla birlikte gelir; sözleşmenin üzerinden yazılan ilk eklentidir.
//!
//! - [`parca`]: satır pencereleri.
//! - [`indeks`]: artımlı güncelleme, kesintiye dayanıklı yazma sırası.
//! - [`arama`]: kosinüs, yeniden sıralama, kaynaklı sonuç.
//!
//! İndeks ve arama yalnızca [`ortam::Ortam`]'ı görür; eklentide onu host
//! çağrıları karşılar ([`HostOrtami`]).

pub mod arama;
pub mod indeks;
pub mod meta;
pub mod ortam;
pub mod parca;

#[cfg(test)]
mod test_ortami;

use kalfa_plugin_sdk::{AracTanimi, DosyaBilgisi, Guest, Hata, Ilerleme, Sonuc, host};

use crate::ortam::{BOYUT, GOMME_MODELI, Ortam, SIRALAMA_MODELI};

pub const ARAC: &str = "anlamsal_ara";

const ACIKLAMA: &str = "Çalışma alanındaki kodda ve belgelerde anlamca arar; sonuçları dosya yolu ve satır aralığıyla, kaynaklı verir. Aranan şeyin adını ya da tam kelimesini bilmediğinizde, \"bu nerede yapılıyor\" türü sorularda ve Türkçe bir soruyla İngilizce koddaki karşılığını ararken `icerik_ara`'dan iyidir. Tam bir ad ya da metin arıyorsanız `icerik_ara` daha hızlı ve kesindir. İlk çağrıda çalışma alanını indeksler; sonraki çağrılar yalnızca değişen dosyaları günceller.";

const SEMA: &str = r#"{
  "type": "object",
  "properties": {
    "sorgu": { "type": "string", "minLength": 1, "maxLength": 2000,
               "description": "Ne aradığınızı doğal dille yazın; Türkçe ya da İngilizce." },
    "sonuc_sayisi": { "type": "integer", "minimum": 1, "maximum": 20, "default": 8 }
  },
  "required": ["sorgu"],
  "additionalProperties": false
}"#;

/// Host çağrılarıyla karşılanan ortam.
pub struct HostOrtami;

impl Ortam for HostOrtami {
    fn dosya_listele(&self, desen: &str) -> Result<Vec<DosyaBilgisi>, Hata> {
        host::dosya_listele(desen)
    }

    fn dosya_oku(&self, yol: &str) -> Result<String, Hata> {
        host::dosya_oku(yol)
    }

    fn gomme(&self, metinler: &[String]) -> Result<Vec<Vec<f32>>, Hata> {
        kalfa_plugin_sdk::gomme(GOMME_MODELI, metinler, BOYUT as u32)
    }

    fn sirala(
        &self,
        sorgu: &str,
        belgeler: &[String],
        ilk_n: u32,
    ) -> Result<Vec<(usize, f32)>, Hata> {
        kalfa_plugin_sdk::yeniden_sirala(SIRALAMA_MODELI, sorgu, belgeler, ilk_n)
    }

    fn depo_oku(&self, anahtar: &str) -> Result<Option<Vec<u8>>, Hata> {
        host::depo_oku(anahtar)
    }

    fn depo_yaz(&self, anahtar: &str, deger: &[u8]) -> Result<(), Hata> {
        host::depo_yaz(anahtar, deger)
    }

    fn depo_sil(&self, anahtar: &str) -> Result<(), Hata> {
        host::depo_sil(anahtar)
    }

    fn ilerleme(&self, tamamlanan: u64, toplam: Option<u64>, mesaj: Option<String>) {
        host::ilerleme_bildir(&Ilerleme {
            tamamlanan,
            toplam,
            mesaj,
        });
    }

    fn iptal_istendi(&self) -> bool {
        host::iptal_istendi()
    }

    fn simdi_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
    }
}

struct KalfaRag;

impl Guest for KalfaRag {
    fn araclari_listele() -> Vec<AracTanimi> {
        vec![AracTanimi {
            ad: ARAC.into(),
            aciklama: ACIKLAMA.into(),
            parametre_semasi: SEMA.into(),
        }]
    }

    fn calistir(ad: String, argumanlar: String) -> Result<Sonuc, Hata> {
        if ad != ARAC {
            return Err(Hata::Bulunamadi(format!("`{ad}` adlı bir araç yok")));
        }
        let a: arama::Argumanlar = kalfa_plugin_sdk::argumanlar(&argumanlar)?;
        Ok(Sonuc {
            icerik: arama::ara(&HostOrtami, &a)?,
        })
    }
}

kalfa_plugin_sdk::kalfa_eklentisi!(KalfaRag);
