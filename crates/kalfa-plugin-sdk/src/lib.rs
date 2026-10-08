//! Kalfa eklenti SDK'sı (aşama 4 tasarımı §12).
//!
//! Eklenti yazarı wit-bindgen ile uğraşmaz: araçlarını [`Guest`] ile yazar ve
//! [`kalfa_eklentisi!`] ile dışa verir. Eklenti `wasm32-wasip2` için derlenir.
//!
//! ```ignore
//! use kalfa_plugin_sdk::{AracTanimi, Guest, Hata, Sonuc, host};
//!
//! struct Ornek;
//!
//! impl Guest for Ornek {
//!     fn araclari_listele() -> Vec<AracTanimi> { /* … */ }
//!     fn calistir(ad: String, argumanlar: String) -> Result<Sonuc, Hata> { /* … */ }
//! }
//!
//! kalfa_plugin_sdk::kalfa_eklentisi!(Ornek);
//! ```

// Üretilen kod ve dışa verme makrosu SDK'ya `kalfa_plugin_sdk` adıyla başvurur;
// SDK'nın kendi içinde de bu ad çözülsün.
extern crate self as kalfa_plugin_sdk;

wit_bindgen::generate!({
    world: "eklenti",
    path: "wit",
    pub_export_macro: true,
    export_macro_name: "kalfa_eklentisi",
    default_bindings_module: "kalfa_plugin_sdk",
    runtime_path: "kalfa_plugin_sdk::wit_bindgen::rt",
});

pub use exports::kalfa::plugin::arac::{AracTanimi, Guest, Sonuc};
pub use kalfa::plugin::host;
pub use kalfa::plugin::tipler::{DosyaBilgisi, EvrenUcu, Hata, Ilerleme, Seviye};
pub use {serde_json, wit_bindgen};

/// Aracın JSON argümanlarını çözer. Host argümanları aracın şemasına karşı
/// zaten doğrulamıştır; bu yine de bozuk girdiyi hataya çevirir.
pub fn argumanlar<T: serde::de::DeserializeOwned>(metin: &str) -> Result<T, Hata> {
    serde_json::from_str(metin).map_err(|e| Hata::Diger(format!("argümanlar çözülemedi: {e}")))
}
