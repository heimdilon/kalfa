//! Windows arka ucu: AppContainer + ACL (aşama 2 tasarımı §9).

pub mod ace_kaydi;
pub mod guvenlik;
pub(crate) mod kap;
pub(crate) mod komut_satiri;
pub mod kopru;
pub mod kurulum;
pub mod msvc;
pub mod sistem;
pub mod yalitici;

pub use crate::agac::{Is, surdur};
