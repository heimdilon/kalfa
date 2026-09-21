#![allow(dead_code)]

use std::path::PathBuf;

/// `research/evren-probe/fixtures/` altındaki gerçek EVREN yanıtının yolu.
pub fn fixture_yolu(ad: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/evren-probe/fixtures")
        .join(ad)
}

pub fn fixture_bayt(ad: &str) -> Vec<u8> {
    std::fs::read(fixture_yolu(ad)).unwrap_or_else(|e| panic!("fixture {ad} okunamadı: {e}"))
}

pub fn fixture_metin(ad: &str) -> String {
    String::from_utf8(fixture_bayt(ad)).expect("fixture UTF-8 olmalı")
}

pub fn sentetik_bayt(ad: &str) -> Vec<u8> {
    let yol = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/sentetik")
        .join(ad);
    std::fs::read(&yol).unwrap_or_else(|e| panic!("sentetik veri {ad} okunamadı: {e}"))
}
