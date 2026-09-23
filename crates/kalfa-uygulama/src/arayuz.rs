//! Uygulamanın dış dünyaya açılan uçları. `kalfa` crate'i bunları Tauri ile
//! uygular; testler sahteleriyle (`test_destek`).

use std::path::PathBuf;

use async_trait::async_trait;
use kalfa_core::aktarim::KanalMesaji;
use url::Url;

/// Yerel iletişim kutuları. Webview'daki bir betik bunları taklit edemez.
#[async_trait]
pub trait Iletisim: Send + Sync {
    /// Yerel klasör seçici; vazgeçilirse `None`.
    async fn klasor_sec(&self) -> Option<PathBuf>;
    /// Yerel onay kutusu; `tamam` düğmesi seçilirse `true`.
    async fn onayla(&self, baslik: &str, mesaj: &str, tamam: &str, vazgec: &str) -> bool;
}

/// Adresi sistem tarayıcısında açar.
pub trait Acici: Send + Sync {
    fn ac(&self, adres: &Url) -> Result<(), String>;
}

/// İki pencereyi yalnızca Rust açar (1c tasarımı §6).
pub trait Pencereler: Send + Sync {
    /// Ana pencereyi açar; açıksa öne getirir.
    fn ana_ac(&self);
    /// Şart penceresini açar; açıksa yeniden yükler ve öne getirir. Pencere
    /// açılınca durumunu `sart_penceresi_durumu` ile okur.
    fn sartlar_goster(&self);
    fn sartlar_kapat(&self);
}

/// Bir oturumun kanal mesajlarının yazıldığı yer (Tauri'de `Channel`).
pub trait KanalYazici: Send + Sync + 'static {
    fn gonder(&self, mesaj: KanalMesaji) -> Result<(), String>;
}
