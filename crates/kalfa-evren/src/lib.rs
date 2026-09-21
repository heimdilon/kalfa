//! EVREN LLM Gateway için Rust istemcisi.
//!
//! Tasarım: `docs/superpowers/specs/2026-09-21-kalfa-design.md` §3 ve §5.

pub mod akis;
pub mod hata;
pub mod istemci;
pub mod kredi;
pub mod sse;
pub mod tipler;

pub use akis::{AkisBirlestirici, AkisOlayi};
pub use hata::EvrenError;
pub use istemci::{AkisKanali, ApiAnahtari, EvrenIstemcisi, VARSAYILAN_TABAN};
pub use kredi::{KrediGozlemi, YanitMeta};
