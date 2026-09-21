//! Kalfa ajan çekirdeği (spec §4, §6, §7).
//!
//! Dış dünyaya tek açıklığı `Komut`/`Olay` sınırıdır (§4.1); arayüzden
//! bağımsızdır ve bir birim testinde baştan sona koşturulabilir (§4.2).

pub mod git_koruma;
pub mod izin;
pub mod kimlik;
pub mod yol;
