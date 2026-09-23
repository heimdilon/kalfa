//! Komutların hata tipi (1c tasarımı §6.1). `mesaj` kullanıcıya gösterilir
//! ("siz" dili); `kod` arayüzün ne yapacağını seçer.

use std::io;

use kalfa_core::oturum::OturumHatasi;
use kalfa_evren::EvrenError;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum HataKodu {
    OturumYok,
    /// Kayıt başka bir süreçte açık.
    KayitMesgul,
    SaltOkunur,
    GecersizGirdi,
    Evren,
    Ic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[error("{mesaj}")]
pub struct KomutHatasi {
    pub kod: HataKodu,
    pub mesaj: String,
}

impl KomutHatasi {
    pub fn yeni(kod: HataKodu, mesaj: impl Into<String>) -> Self {
        Self {
            kod,
            mesaj: mesaj.into(),
        }
    }

    pub fn oturum_yok() -> Self {
        Self::yeni(HataKodu::OturumYok, "Bu oturum açık değil.")
    }

    pub fn salt_okunur() -> Self {
        Self::yeni(
            HataKodu::SaltOkunur,
            "Bu oturum yalnızca okunabilir; çalışma alanı bulunamadı.",
        )
    }

    pub fn gecersiz(mesaj: impl Into<String>) -> Self {
        Self::yeni(HataKodu::GecersizGirdi, mesaj)
    }

    pub fn ic(mesaj: impl Into<String>) -> Self {
        Self::yeni(HataKodu::Ic, mesaj)
    }

    /// Oturum kaydı okunurken çıkan hata; kaydın olmaması `OturumYok`tur.
    pub fn kayit(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::NotFound {
            return Self::yeni(HataKodu::OturumYok, "Bu oturumun kaydı bulunamadı.");
        }
        e.into()
    }
}

impl From<io::Error> for KomutHatasi {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::ResourceBusy {
            return Self::yeni(
                HataKodu::KayitMesgul,
                "Bu oturum başka bir Kalfa sürecinde açık.",
            );
        }
        Self::ic(e.to_string())
    }
}

impl From<OturumHatasi> for KomutHatasi {
    fn from(e: OturumHatasi) -> Self {
        match e {
            OturumHatasi::Kayit(io) => io.into(),
            OturumHatasi::Kapali => Self::oturum_yok(),
            diger => Self::ic(diger.to_string()),
        }
    }
}

impl From<EvrenError> for KomutHatasi {
    fn from(e: EvrenError) -> Self {
        Self::yeni(HataKodu::Evren, e.to_string())
    }
}
