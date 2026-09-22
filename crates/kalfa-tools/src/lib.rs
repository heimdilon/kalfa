//! Kalfa'nın yerleşik araçları (spec §7.3).
//!
//! Çalışma alanına yalnızca `cap-std` dizin tutamacı üzerinden erişilir.

mod arguman;

pub mod ara;
pub mod baglanti;
pub mod calisma_alani;
pub mod listele;
pub mod oku;

pub use ara::IcerikAra;
pub use calisma_alani::CalismaAlani;
pub use listele::DosyaListele;
pub use oku::DosyaOku;
