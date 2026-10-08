//! Kalfa eklenti çalışma zamanı (aşama 4 tasarımı §5–§8).
//!
//! - [`manifest`]: `kalfa.toml` ve ad kuralları.
//! - [`Motor`]: wasmtime motoru; eklentiyi yükler, her çağrıya yeni örnek.
//! - [`EklentiAraci`]: eklentinin aracı, `kalfa-core`'un `Arac` trait'iyle.
//!
//! Eklenti Kalfa'nın sürecinde çalışır. Sınır, WebAssembly'nin bellek
//! yalıtımı ve yeteneksiz WASI'dir: ağ yok, ön açık dizin yok. Dosyalara
//! yalnızca host fonksiyonlarıyla, yerleşik okumanın yolundan erişir.

mod arac;
mod bag;
mod calistirici;
pub mod depo;
mod host;
pub mod manifest;
pub mod sinir;

pub use arac::EklentiAraci;
pub use calistirici::{AracBilgisi, Eklenti, Motor, YuklemeHatasi};
