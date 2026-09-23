//! Kimlik deposu (1c tasarımı §7.2). Anahtar işletim sisteminin deposunda
//! (Windows Credential Manager, Linux Secret Service) tutulur. Depo yoksa ya
//! da yazılamıyorsa anahtar yalnızca Kalfa açık kaldıkça bellekte durur ve
//! kullanıcıya söylenir. Anahtar hiçbir günlüğe, kayda ya da arayüz durumuna
//! yazılmaz.

use std::sync::Mutex;

use crate::sozlesme::DepoTuru;

#[derive(Debug, thiserror::Error)]
#[error("Kimlik deposu kullanılamadı: {0}")]
pub struct DepoHatasi(pub String);

/// Tek bir anahtarı saklayan depo.
pub trait AnahtarDeposu: Send + Sync {
    fn oku(&self) -> Result<Option<String>, DepoHatasi>;
    fn yaz(&self, anahtar: &str) -> Result<(), DepoHatasi>;
}

/// İşletim sisteminin kimlik deposu (`keyring` 4, `v1` özelliği).
pub struct SistemDeposu {
    giris: keyring::Entry,
}

impl SistemDeposu {
    pub const SERVIS: &str = "Kalfa";
    pub const KULLANICI: &str = "evren-api-anahtari";

    /// Depo kurulamazsa (ör. Linux'ta Secret Service yok) hata döner.
    pub fn ac() -> Result<Self, DepoHatasi> {
        keyring::Entry::new(Self::SERVIS, Self::KULLANICI)
            .map(|giris| Self { giris })
            .map_err(|e| DepoHatasi(e.to_string()))
    }
}

impl AnahtarDeposu for SistemDeposu {
    fn oku(&self) -> Result<Option<String>, DepoHatasi> {
        match self.giris.get_password() {
            Ok(anahtar) => Ok(Some(anahtar)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(DepoHatasi(e.to_string())),
        }
    }

    fn yaz(&self, anahtar: &str) -> Result<(), DepoHatasi> {
        self.giris
            .set_password(anahtar)
            .map_err(|e| DepoHatasi(e.to_string()))
    }
}

/// Bellekte tutan depo.
#[derive(Default)]
pub struct BellekDeposu {
    deger: Mutex<Option<String>>,
}

impl AnahtarDeposu for BellekDeposu {
    fn oku(&self) -> Result<Option<String>, DepoHatasi> {
        Ok(self.deger.lock().unwrap_or_else(|z| z.into_inner()).clone())
    }

    fn yaz(&self, anahtar: &str) -> Result<(), DepoHatasi> {
        *self.deger.lock().unwrap_or_else(|z| z.into_inner()) = Some(anahtar.to_string());
        Ok(())
    }
}

/// Uygulamanın kullandığı kasa: kalıcı depoyu dener, yazılamazsa belleğe düşer.
pub struct AnahtarKasasi {
    kalici: Option<Box<dyn AnahtarDeposu>>,
    bellek: BellekDeposu,
    tur: Mutex<DepoTuru>,
}

impl AnahtarKasasi {
    /// `kalici` yoksa (depo kurulamadı) anahtar yalnızca bellekte tutulur.
    pub fn yeni(kalici: Option<Box<dyn AnahtarDeposu>>) -> Self {
        let tur = if kalici.is_some() {
            DepoTuru::Kalici
        } else {
            DepoTuru::Bellekte
        };
        Self {
            kalici,
            bellek: BellekDeposu::default(),
            tur: Mutex::new(tur),
        }
    }

    /// Kayıtlı anahtar. Bellekteki (bu çalıştırmada girilen) önce gelir.
    /// Kalıcı depo okunamazsa anahtar yok sayılır; kullanıcı yeniden girer.
    pub fn oku(&self) -> Option<String> {
        if let Ok(Some(anahtar)) = self.bellek.oku() {
            return Some(anahtar);
        }
        self.kalici.as_ref().and_then(|d| d.oku().ok().flatten())
    }

    /// Anahtarı saklar; nereye saklandığını döner.
    pub fn yaz(&self, anahtar: &str) -> DepoTuru {
        let tur = match &self.kalici {
            Some(depo) if depo.yaz(anahtar).is_ok() => {
                // Önceki bir yazımdan bellekte kalan anahtar yenisini gölgelemesin.
                *self.bellek.deger.lock().unwrap_or_else(|z| z.into_inner()) = None;
                DepoTuru::Kalici
            }
            _ => {
                // BellekDeposu hata vermez.
                let _ = self.bellek.yaz(anahtar);
                DepoTuru::Bellekte
            }
        };
        *self.tur.lock().unwrap_or_else(|z| z.into_inner()) = tur;
        tur
    }

    pub fn tur(&self) -> DepoTuru {
        *self.tur.lock().unwrap_or_else(|z| z.into_inner())
    }
}
