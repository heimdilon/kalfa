//! Kalfa masaüstü uygulamasının Tauri'siz mantığı (1c tasarımı §3.1, §3.2).
//!
//! `kalfa` crate'i bunu Tauri komutlarına ve pencerelerine bağlar. Pencereye,
//! yerel iletişim kutusuna, sistem tarayıcısına ve kanala trait'lerle erişilir
//! (`arayuz`); testler sahteleriyle koşar ve WebKitGTK istemez.

pub mod anahtar;
pub mod arayuz;
pub mod dizin;
pub mod evren;
pub mod guvenlik;
pub mod hata;
pub mod modeller;
pub mod sozlesme;
