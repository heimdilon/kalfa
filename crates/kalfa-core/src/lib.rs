//! Kalfa ajan çekirdeği (spec §4, §6, §7).
//!
//! Dış dünyaya tek açıklığı `Komut`/`Olay` sınırıdır (§4.1); arayüzden
//! bağımsızdır ve bir birim testinde baştan sona koşturulabilir (§4.2).

pub mod aktarim;
pub mod arac;
pub mod butce;
pub mod denetci;
pub mod gecmis;
pub mod geri_alma;
pub mod git_koruma;
pub mod gonderim;
pub mod gorunum;
pub mod hakem;
pub mod izin;
pub mod kabuk;
pub mod kayit;
pub mod kilit;
pub mod kimlik;
pub mod kisayol;
pub mod kurtarma;
pub mod muhur;
pub mod oturum;
pub mod protokol;
pub mod saglayici;
pub mod sart_kapisi;
pub mod tur_ozeti;
pub mod yol;

#[cfg(feature = "test-destek")]
pub mod test_destek;
