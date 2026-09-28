//! Uygulamanın dış dünyaya açılan uçları. `kalfa` crate'i bunları Tauri ile
//! uygular; testler sahteleriyle (`test_destek`).

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use kalfa_core::aktarim::KanalMesaji;
pub use kalfa_sandbox::YalitimDurumu;
use url::Url;

/// Yerel iletişim kutuları. Webview'daki bir betik bunları taklit edemez.
#[async_trait]
pub trait Iletisim: Send + Sync {
    /// Yerel klasör seçici; `baslangic` verilirse orada açılır. Vazgeçilirse
    /// `None`.
    async fn klasor_sec(&self, baslik: &str, baslangic: Option<&Path>) -> Option<PathBuf>;
    /// Yerel onay kutusu; `tamam` düğmesi seçilirse `true`.
    async fn onayla(&self, baslik: &str, mesaj: &str, tamam: &str, vazgec: &str) -> bool;
}

/// Yalıtımın kurulumunu yönetici olarak yineleyen uç, "Onar" (aşama 2
/// tasarımı §9.8). Gerçeği Windows'ta kurulu `kalfa-yalitim.exe`'yi UAC ile
/// başlatır.
#[async_trait]
pub trait Yukseltici: Send + Sync {
    /// Kalfa kurulu sürüm mü: yardımcı Program Files'ta, Kalfa'nın yanında mı?
    /// Değilse (geliştirme derlemesi, Linux) Onar yoktur.
    fn kurulu_mu(&self) -> bool;
    /// `kalfa-yalitim kur [--izin-ver <dizin>]...`'ü UAC ile çalıştırır ve
    /// bekler; çıkış kodunu döner (3: kök izinleri kuruldu, bir dizin
    /// reddedildi). Kullanıcı UAC'de vazgeçerse `Ok(None)`.
    async fn kur(&self, izinli: &[PathBuf]) -> Result<Option<u32>, String>;
    /// `kalfa-yalitim izin-ver <dizin>`'i UAC ile çalıştırır ve bekler.
    async fn izin_ver(&self, dizin: &Path) -> Result<Option<u32>, String>;
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

/// Yalıtım durumunun ana pencereye yazıldığı yer (Tauri'de `Channel`;
/// aşama 2 tasarımı §10.5).
pub trait YalitimYazici: Send + Sync + 'static {
    fn gonder(&self, durum: &YalitimDurumu) -> Result<(), String>;
}

/// Bir oturumun kanal mesajlarının yazıldığı yer (Tauri'de `Channel`).
pub trait KanalYazici: Send + Sync + 'static {
    fn gonder(&self, mesaj: KanalMesaji) -> Result<(), String>;
}
