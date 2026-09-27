//! Kalfa'nın yerleşik araçları (spec §7.3).
//!
//! Çalışma alanına yalnızca `cap-std` dizin tutamacı üzerinden erişilir.

mod arguman;

pub mod ara;
pub mod baglanti;
pub mod calisma_alani;
pub mod diff;
pub mod duzenle;
pub mod kabuk;
pub mod listele;
pub mod oku;
pub mod yaz;

pub use ara::IcerikAra;
pub use calisma_alani::CalismaAlani;
pub use duzenle::DosyaDuzenle;
pub use kabuk::{KabukAraci, KabukGuveni, YalitimsizIzni};
pub use listele::DosyaListele;
pub use oku::DosyaOku;
pub use yaz::DosyaYaz;

use std::sync::Arc;

use kalfa_core::arac::Arac;

/// Yerleşik dosya araçları (spec §12.1). Kabuk aracı ayrıca kurulur
/// (`KabukAraci::yeni`): yalıtıcı ve çalışma alanının güven kaydını ister.
pub fn varsayilan_araclar(alan: Arc<CalismaAlani>) -> Vec<Arc<dyn Arac>> {
    vec![
        Arc::new(DosyaOku::yeni(alan.clone())) as Arc<dyn Arac>,
        Arc::new(DosyaListele::yeni(alan.clone())) as Arc<dyn Arac>,
        Arc::new(IcerikAra::yeni(alan.clone())) as Arc<dyn Arac>,
        Arc::new(DosyaYaz::yeni(alan.clone())) as Arc<dyn Arac>,
        Arc::new(DosyaDuzenle::yeni(alan)) as Arc<dyn Arac>,
    ]
}
