//! Komut sözleşmesinin uygulamaya ait tipleri (1c tasarımı §6.1). JSON
//! biçimleri arayüzdeki `ui/src/ipc/tipler.ts` ile aynıdır; biri değişirse
//! öbürü de aynı commit'te değişir.

use kalfa_core::gecmis::{DurumOzeti, GecmisSayfasi, OturumDurumu, OturumOzeti};
use kalfa_core::kimlik::{CagriId, OturumId};
use kalfa_core::muhur::OnayMuhru;
use kalfa_core::protokol::OnayKarari;
use kalfa_core::sart_kapisi::SartMetni;
use kalfa_sandbox::YalitimDurumu;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BaglantiDurumu {
    Var,
    /// Açılışta EVREN'e ulaşılamadı; ana pencere şerit gösterir (1c tasarımı §7.1).
    Yok,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DepoTuru {
    /// İşletim sisteminin kimlik deposu.
    Kalici,
    /// Depo yok; anahtar Kalfa açık kaldıkça bellekte (1c tasarımı §7.2).
    Bellekte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ModelProfili {
    Hizli,
    Dengeli,
    Derin,
    Otomatik,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModelSecenegi {
    pub kimlik: String,
    pub profil: ModelProfili,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UygulamaDurumu {
    pub baglanti: BaglantiDurumu,
    pub anahtar_deposu: DepoTuru,
    pub modeller: Vec<ModelSecenegi>,
    pub varsayilan_model: String,
    /// Bu çalıştırmada en son açılan ya da başlatılan oturum; yeniden
    /// yüklenen ana pencere onu kayıttan açar (1c tasarımı §5.6).
    pub son_oturum: Option<OturumId>,
    /// Kabuk komutlarının yalıtımı (aşama 2 tasarımı §10.3).
    pub yalitim: YalitimDurumu,
}

/// Oturum listesinin bir satırı (1c tasarımı §5.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OturumSatiri {
    /// Kayıt okunamadıysa `None`; o satır açılamaz.
    pub oturum: Option<OturumId>,
    /// Kayıt dosyasının adı.
    pub dosya: String,
    pub baslik: Option<String>,
    pub calisma_alani: Option<String>,
    pub son_degisiklik_ms: u64,
    pub durum: OturumDurumu,
}

impl From<OturumOzeti> for OturumSatiri {
    fn from(o: OturumOzeti) -> Self {
        Self {
            oturum: o.oturum,
            dosya: o
                .yol
                .file_name()
                .map(|ad| ad.to_string_lossy().into_owned())
                .unwrap_or_default(),
            baslik: o.baslik,
            calisma_alani: o.calisma_alani.map(|yol| yol.display().to_string()),
            son_degisiklik_ms: o.son_degisiklik_ms,
            durum: o.durum,
        }
    }
}

/// Yerel klasör seçiciden gelen çalışma alanı. Arayüz yol göndermez; bu
/// kimlikle başvurur. Kimlik yalnızca bu çalıştırmada geçerlidir (1c tasarımı
/// §5.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CalismaAlani {
    pub kimlik: u64,
    pub yol: String,
}

/// Kayıttan açılan oturumun neden yalnızca okunabildiği (1c tasarımı §10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum SaltOkunurNedeni {
    CalismaAlaniYok {
        yol: String,
    },
    /// Kayıt çalışma alanını yazmıyor (1c'den önceki kayıt).
    CalismaAlaniKayitsiz,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OturumAcilisi {
    pub oturum: OturumId,
    pub calisma_alani: Option<String>,
    pub gecmis: GecmisSayfasi,
    pub ozet: DurumOzeti,
    pub salt_okunur: Option<SaltOkunurNedeni>,
}

/// Onay sayfasındaki tek bir çağrının kararı.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OnayGirdisi {
    pub cagri: CagriId,
    pub muhur: OnayMuhru,
    pub karar: OnayKarari,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AnahtarNedeni {
    /// Kimlik deposunda anahtar yok (ilk açılış).
    Yok,
    /// Kayıtlı anahtar artık geçerli değil (401).
    KayitliGecersiz,
    /// Kullanıcı "Anahtarı değiştir"i seçti.
    Degistir,
}

/// Şart penceresinin göstereceği görünüm (1c tasarımı §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum SartPenceresiDurumu {
    AnahtarGerekli {
        neden: AnahtarNedeni,
    },
    SartGerekli {
        metin: SartMetni,
    },
    /// Kullanıcı şartları reddetti (1c tasarımı §7.4).
    Kilitli,
    /// İş bitti; pencere kapanır.
    Tamam,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum AnahtarSonucu {
    /// 401: anahtar saklanmadı.
    Gecersiz,
    /// EVREN'e ulaşılamadı: anahtar saklanmadı.
    Ulasilamadi,
    Kaydedildi {
        depo: DepoTuru,
        sonraki: SartPenceresiDurumu,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum KabulYaniti {
    Kabul,
    /// Sürüm gösterilenle aynı değildi ya da bu arada değişti.
    YeniMetin {
        metin: SartMetni,
    },
    /// Aynı anda "Kabul etmiyorum" da verildi; ret geçerli.
    Kilitli,
}
