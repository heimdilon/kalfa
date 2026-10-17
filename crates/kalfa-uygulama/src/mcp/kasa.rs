//! MCP sunucu sırları için anahtarlı işletim sistemi gizli değer kasası.
//!
//! Mevcut EVREN `AnahtarKasasi` (`anahtar.rs`) tek sabit `(Kalfa, evren_api_key)`
//! girişine bağlıdır ve değiştirilmez. Bu modül MCP sırları için `Kalfa`
//! servisi altında `mcp-<uuid>` hesap anahtarlarıyla çalışan bağımsız
//! [`GizliKasa`] soyutlamasını ve [`SistemGizliKasasi`] uygulamasını sağlar.
//!
//! Güvenlik kuralları:
//! - Bellek içi ya da düz metin dosya geri dönüşü (fallback) **yoktur**.
//! - İşletim sistemi gizli deposu kullanılamıyorsa işlem açıkça hata döner.
//! - Gizli değerler en az 8 UTF-8 karakter (`chars().count() >= 8`) olmalı ve
//!   `\0` baytı içermemelidir.
//! - Hata türleri ham `keyring::Error` metnini veya aday sır değerini taşımaz.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;
use uuid::Uuid;

/// İşletim sistemi anahtarlığında kullanılan sabit servis adı.
pub const SERVIS_ADI: &str = "Kalfa";

/// MCP sır kimliklerinin sabit öneki.
pub const KIMLIK_ONEKI: &str = "mcp-";

/// Gizli değerler için en az UTF-8 karakter sayısı.
pub const EN_AZ_GIZLI_KARAKTER: usize = 8;

/// Ayarlarda (`ayarlar.json`) saklanan opak sır kimliği (`mcp-<uuid>`).
///
/// Bu kimlik gizli değerin kendisini taşımaz; yalnızca işletim sistemi
/// anahtarlığındaki hesap adını temsil eder.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GizliKimlik(String);

impl GizliKimlik {
    /// Yeni, rastgele bir `mcp-<uuid>` kimliği üretir.
    pub fn yeni() -> Self {
        Self(format!("{KIMLIK_ONEKI}{}", Uuid::new_v4()))
    }

    /// Var olan bir kimlik dizgesini doğrular (`mcp-` öneki ve geçerli,
    /// kanonik küçük harfli UUID v4).
    pub fn yeni_dogrulanmis(deger: impl Into<String>) -> Result<Self, KasaHatasi> {
        let deger = deger.into();
        let Some(uuid_kismi) = deger.strip_prefix(KIMLIK_ONEKI) else {
            return Err(KasaHatasi::GecersizKimlik);
        };
        let Ok(uuid) = Uuid::try_parse(uuid_kismi) else {
            return Err(KasaHatasi::GecersizKimlik);
        };
        if uuid.get_version() != Some(uuid::Version::Random)
            || uuid.get_variant() != uuid::Variant::RFC4122
        {
            return Err(KasaHatasi::GecersizKimlik);
        }
        // Yalnızca kanonik küçük harfli tireli gösterimi kabul et.
        let kanonik = format!("{KIMLIK_ONEKI}{}", uuid.hyphenated());
        if deger != kanonik {
            return Err(KasaHatasi::GecersizKimlik);
        }
        Ok(Self(kanonik))
    }

    /// İşletim sistemi anahtarlığında kullanılan hesap adını (`mcp-<uuid>`) döner.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for GizliKimlik {
    fn default() -> Self {
        Self::yeni()
    }
}

impl fmt::Display for GizliKimlik {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for GizliKimlik {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for GizliKimlik {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let ham = String::deserialize(deserializer)?;
        Self::yeni_dogrulanmis(ham).map_err(serde::de::Error::custom)
    }
}

/// Gizli değer doğrulama hatası. Aday sır metnini asla taşımaz.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GizliDegerHatasi {
    #[error("gizli değer en az {EN_AZ_GIZLI_KARAKTER} karakter olmalıdır")]
    CokKisa,
    #[error("gizli değer NUL karakteri içeremez")]
    NulKarakteri,
}

/// Gizli kasa işlemleri sırasında oluşabilecek hatalar.
///
/// Ham `keyring::Error` metinleri veya gizli değerler hata mesajına yazılmaz.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KasaHatasi {
    #[error("geçersiz gizli değer kimliği")]
    GecersizKimlik,
    #[error("işletim sistemi gizli değer deposu kullanılamıyor")]
    SistemDeposuYok,
    #[error("gizli değer kasasına erişilemedi")]
    ErisimHatasi,
    #[error("gizli değer kasasındaki kayıt bozuk")]
    BozukVeri,
    #[error(transparent)]
    GecersizDeger(#[from] GizliDegerHatasi),
}

/// Bir gizli değerin uzunluk ve NUL kısıtlarını doğrular.
pub fn gizli_deger_dogrula(deger: &str) -> Result<(), GizliDegerHatasi> {
    if deger.contains('\0') {
        return Err(GizliDegerHatasi::NulKarakteri);
    }
    if deger.chars().count() < EN_AZ_GIZLI_KARAKTER {
        return Err(GizliDegerHatasi::CokKisa);
    }
    Ok(())
}

/// Anahtarlı gizli değer kasası.
pub trait GizliKasa: Send + Sync {
    /// Verilen kimliğe karşılık gelen sırrı okur; kayıt yoksa `Ok(None)` döner.
    fn oku(&self, kimlik: &GizliKimlik) -> Result<Option<String>, KasaHatasi>;

    /// Verilen kimliğe sırrı yazar (önce [`gizli_deger_dogrula`] ile doğrular).
    fn yaz(&self, kimlik: &GizliKimlik, deger: &str) -> Result<(), KasaHatasi>;

    /// Verilen kimliği kasadan siler; kayıt zaten yoksa `Ok(())` döner.
    fn sil(&self, kimlik: &GizliKimlik) -> Result<(), KasaHatasi>;
}

/// İşletim sisteminin kimlik deposunu (`keyring` crate'i: Windows Credential
/// Manager, Linux Secret Service, macOS Keychain) kullanan gerçek kasa.
///
/// Bellek içi veya dosya tabanlı hiçbir yedek mekanizma içermez.
#[derive(Default)]
pub struct SistemGizliKasasi;

impl SistemGizliKasasi {
    pub fn yeni() -> Self {
        Self
    }

    fn giris(kimlik: &GizliKimlik) -> Result<keyring::Entry, KasaHatasi> {
        let dogrulanmis = GizliKimlik::yeni_dogrulanmis(kimlik.as_str())?;
        keyring::Entry::new(SERVIS_ADI, dogrulanmis.as_str()).map_err(keyring_hatasini_cevir)
    }
}

fn keyring_hatasini_cevir(hata: keyring::Error) -> KasaHatasi {
    match hata {
        keyring::Error::NoStorageAccess(_) | keyring::Error::PlatformFailure(_) => {
            KasaHatasi::SistemDeposuYok
        }
        keyring::Error::BadEncoding(_) => KasaHatasi::BozukVeri,
        keyring::Error::Invalid(_, _) | keyring::Error::TooLong(_, _) => KasaHatasi::GecersizKimlik,
        keyring::Error::NoEntry | keyring::Error::Ambiguous(_) | _ => KasaHatasi::ErisimHatasi,
    }
}

impl GizliKasa for SistemGizliKasasi {
    fn oku(&self, kimlik: &GizliKimlik) -> Result<Option<String>, KasaHatasi> {
        let giris = Self::giris(kimlik)?;
        match giris.get_password() {
            Ok(sifre) => Ok(Some(sifre)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(hata) => Err(keyring_hatasini_cevir(hata)),
        }
    }

    fn yaz(&self, kimlik: &GizliKimlik, deger: &str) -> Result<(), KasaHatasi> {
        gizli_deger_dogrula(deger)?;
        let giris = Self::giris(kimlik)?;
        giris.set_password(deger).map_err(keyring_hatasini_cevir)
    }

    fn sil(&self, kimlik: &GizliKimlik) -> Result<(), KasaHatasi> {
        let giris = Self::giris(kimlik)?;
        match giris.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(hata) => Err(keyring_hatasini_cevir(hata)),
        }
    }
}

#[cfg(any(test, feature = "test-destek"))]
pub mod test_destek {
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Mutex;

    use super::{GizliKasa, GizliKimlik, KasaHatasi, gizli_deger_dogrula};

    /// Yalnızca testlerde hata enjeksiyonu ve çoklu sır izolasyonunu
    /// doğrulamak için kullanılan sahte kasa. Üretim kodunda derlenmez.
    #[derive(Default)]
    pub struct SahteGizliKasa {
        kayitlar: Mutex<BTreeMap<GizliKimlik, String>>,
        yaz_sayaci: Mutex<usize>,
        sil_sayaci: Mutex<usize>,
        yaz_hatasi_adimi: Mutex<Option<(usize, KasaHatasi)>>,
        sil_hatasi_kimlikleri: Mutex<BTreeSet<GizliKimlik>>,
        sistem_deposu_yok: Mutex<bool>,
    }

    impl SahteGizliKasa {
        pub fn yeni() -> Self {
            Self::default()
        }

        pub fn sistem_deposu_kapali(&self, kapali: bool) {
            *self.sistem_deposu_yok.lock().unwrap() = kapali;
        }

        /// `kacinci` (1-tabanlı) `yaz` çağrısında hata döndürür.
        pub fn yaz_hatasi_ayarla(&self, kacinci: usize, hata: KasaHatasi) {
            *self.yaz_hatasi_adimi.lock().unwrap() = Some((kacinci, hata));
        }

        pub fn sil_hatasi_ekle(&self, kimlik: GizliKimlik) {
            self.sil_hatasi_kimlikleri.lock().unwrap().insert(kimlik);
        }

        pub fn sil_hatalarini_temizle(&self) {
            self.sil_hatasi_kimlikleri.lock().unwrap().clear();
        }

        pub fn kayit_sayisi(&self) -> usize {
            self.kayitlar.lock().unwrap().len()
        }

        pub fn ham_oku(&self, kimlik: &GizliKimlik) -> Option<String> {
            self.kayitlar.lock().unwrap().get(kimlik).cloned()
        }

        pub fn tum_kimlikler(&self) -> Vec<GizliKimlik> {
            self.kayitlar.lock().unwrap().keys().cloned().collect()
        }
    }

    impl GizliKasa for SahteGizliKasa {
        fn oku(&self, kimlik: &GizliKimlik) -> Result<Option<String>, KasaHatasi> {
            let _ = GizliKimlik::yeni_dogrulanmis(kimlik.as_str())?;
            if *self.sistem_deposu_yok.lock().unwrap() {
                return Err(KasaHatasi::SistemDeposuYok);
            }
            Ok(self.kayitlar.lock().unwrap().get(kimlik).cloned())
        }

        fn yaz(&self, kimlik: &GizliKimlik, deger: &str) -> Result<(), KasaHatasi> {
            let dogrulanmis = GizliKimlik::yeni_dogrulanmis(kimlik.as_str())?;
            gizli_deger_dogrula(deger)?;
            if *self.sistem_deposu_yok.lock().unwrap() {
                return Err(KasaHatasi::SistemDeposuYok);
            }
            let mut sayac = self.yaz_sayaci.lock().unwrap();
            *sayac += 1;
            if let Some((hedef, hata)) = self.yaz_hatasi_adimi.lock().unwrap().clone()
                && *sayac == hedef
            {
                return Err(hata);
            }
            self.kayitlar
                .lock()
                .unwrap()
                .insert(dogrulanmis, deger.to_owned());
            Ok(())
        }

        fn sil(&self, kimlik: &GizliKimlik) -> Result<(), KasaHatasi> {
            let dogrulanmis = GizliKimlik::yeni_dogrulanmis(kimlik.as_str())?;
            if *self.sistem_deposu_yok.lock().unwrap() {
                return Err(KasaHatasi::SistemDeposuYok);
            }
            let mut sayac = self.sil_sayaci.lock().unwrap();
            *sayac += 1;
            if self
                .sil_hatasi_kimlikleri
                .lock()
                .unwrap()
                .contains(&dogrulanmis)
            {
                return Err(KasaHatasi::ErisimHatasi);
            }
            self.kayitlar.lock().unwrap().remove(&dogrulanmis);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_destek::SahteGizliKasa;
    use super::*;

    #[test]
    fn gizli_kimlik_yalnizca_kanonik_mcp_uuid_kabul_eder() {
        let kimlik = GizliKimlik::yeni();
        assert!(kimlik.as_str().starts_with("mcp-"));
        assert_eq!(
            GizliKimlik::yeni_dogrulanmis(kimlik.as_str()).unwrap(),
            kimlik
        );

        for gecersiz in [
            "",
            "evren_api_key",
            "mcp-",
            "mcp-degil-uuid",
            "MCP-550e8400-e29b-41d4-a716-446655440000",
            "mcp-550E8400-E29B-41D4-A716-446655440000",
            // nil ve v1 UUID'ler reddedilmeli (yalnızca RFC4122 v4 kabul edilir)
            "mcp-00000000-0000-0000-0000-000000000000",
            "mcp-550e8400-e29b-11d4-a716-446655440000",
        ] {
            assert_eq!(
                GizliKimlik::yeni_dogrulanmis(gecersiz),
                Err(KasaHatasi::GecersizKimlik),
                "geçersiz kimlik reddedilmeli: {gecersiz}"
            );
            let json = serde_json::to_string(gecersiz).unwrap();
            assert!(
                serde_json::from_str::<GizliKimlik>(&json).is_err(),
                "serde geçersiz kimliği reddetmeli: {gecersiz}"
            );
        }
    }

    #[test]
    fn gizli_deger_uzunluk_ve_nul_kisitlarini_uygular() {
        let kasa = SahteGizliKasa::yeni();
        let kimlik = GizliKimlik::yeni();

        for kisa in ["", "q", "1234567", "ğüşiöç7"] {
            assert_eq!(
                kasa.yaz(&kimlik, kisa),
                Err(KasaHatasi::GecersizDeger(GizliDegerHatasi::CokKisa))
            );
        }

        let nul_deger = "cok-gizli\0deger";
        let hata = kasa.yaz(&kimlik, nul_deger).unwrap_err();
        assert_eq!(
            hata,
            KasaHatasi::GecersizDeger(GizliDegerHatasi::NulKarakteri)
        );
        assert!(!hata.to_string().contains("cok-gizli"));
        assert!(!format!("{hata:?}").contains("cok-gizli"));

        // 8 çok baytlı UTF-8 karakter kabul edilmeli.
        kasa.yaz(&kimlik, "ğüşiöç12").unwrap();
        assert_eq!(kasa.oku(&kimlik).unwrap().as_deref(), Some("ğüşiöç12"));
    }

    #[test]
    fn sistem_deposu_yokken_bellek_veya_dosya_yedegine_dusmez() {
        let kasa = SahteGizliKasa::yeni();
        let kimlik = GizliKimlik::yeni();
        kasa.sistem_deposu_kapali(true);

        assert_eq!(
            kasa.yaz(&kimlik, "sekiz-karakterli-sir"),
            Err(KasaHatasi::SistemDeposuYok)
        );
        assert_eq!(kasa.oku(&kimlik), Err(KasaHatasi::SistemDeposuYok));
        assert_eq!(kasa.sil(&kimlik), Err(KasaHatasi::SistemDeposuYok));
        assert_eq!(kasa.kayit_sayisi(), 0);
    }

    #[test]
    fn keyring_hata_cevirisi_ham_detay_sizdirmaz() {
        let hata = keyring_hatasini_cevir(keyring::Error::NoStorageAccess(
            "gizli-detay-12345678".into(),
        ));
        assert_eq!(hata, KasaHatasi::SistemDeposuYok);
        assert!(!hata.to_string().contains("gizli-detay"));
        assert!(!format!("{hata:?}").contains("gizli-detay"));
    }
}
