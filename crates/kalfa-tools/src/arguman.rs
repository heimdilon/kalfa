//! Araç argümanlarını okuma yardımcıları.
//!
//! Şema doğrulaması çekirdekte yapılır; yine de eksik ya da yanlış tipte
//! alanlarda panik yerine tipli hata döndürmek araçların doğrudan kullanımında
//! da önemlidir.

use kalfa_core::arac::AracHatasi;
use kalfa_core::yol::CalismaYolu;
use serde_json::Value;

pub(crate) fn metin<'a>(a: &'a Value, alan: &str) -> Result<&'a str, AracHatasi> {
    a.get(alan)
        .and_then(Value::as_str)
        .ok_or_else(|| AracHatasi::Gecersiz(format!("`{alan}` alanı gerekli.")))
}

pub(crate) fn yol(a: &Value, alan: &str) -> Result<CalismaYolu, AracHatasi> {
    Ok(CalismaYolu::coz(metin(a, alan)?)?)
}

pub(crate) fn sayi(a: &Value, alan: &str) -> Option<u64> {
    a.get(alan).and_then(Value::as_u64)
}

pub(crate) fn mantiksal(a: &Value, alan: &str) -> bool {
    a.get(alan).and_then(Value::as_bool).unwrap_or(false)
}
