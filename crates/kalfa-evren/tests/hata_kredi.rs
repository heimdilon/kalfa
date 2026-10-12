mod ortak;

use std::str::FromStr;
use std::time::Duration;

use kalfa_evren::hata::hata_esle;
use kalfa_evren::kredi::basliklardan;
use kalfa_evren::{EvrenError, KrediGozlemi, YanitMeta};
use reqwest::header::{HeaderMap, HeaderValue};
use rust_decimal::Decimal;

#[test]
fn gecersiz_anahtar() {
    let h = hata_esle(401, None, &ortak::fixture_bayt("error_unauthorized.json"));
    assert_eq!(h, EvrenError::AnahtarGecersiz);
    assert!(
        h.to_string().starts_with("Anahtar geçersiz ya da eksik."),
        "sunucu mesajı değil kendi metnimiz (§5.4)"
    );
}

#[test]
fn anahtar_hatasi_kullaniciya_siz_diye_hitap_eder() {
    let m = EvrenError::AnahtarGecersiz.to_string();
    assert!(m.contains("anahtarınızı kontrol edip yeniden girin"), "{m}");
}

#[test]
fn sart_kabul_gerekli_sentetik() {
    let h = hata_esle(403, None, &ortak::sentetik_bayt("terms_not_accepted.json"));
    assert_eq!(h, EvrenError::SartKabulGerekli { surum: Some(1) });
}

#[test]
fn model_bulunamadi_onerilerle() {
    let h = hata_esle(
        404,
        None,
        &ortak::fixture_bayt("error_model_not_found.json"),
    );
    assert_eq!(
        h,
        EvrenError::ModelBulunamadi {
            mesaj: "model bulunamadı: guard".into(),
            oneriler: vec!["qwen3-guard-4b".into()]
        }
    );
}

#[test]
fn gorev_uyusmazligi_genel_sunucu_hatasi_olarak_gelir() {
    match hata_esle(400, None, &ortak::fixture_bayt("error_task_mismatch.json")) {
        EvrenError::Sunucu { durum, kod, mesaj } => {
            assert_eq!(durum, 400);
            assert_eq!(kod.as_deref(), Some("bad_request"));
            assert!(mesaj.contains("görev: guard"));
        }
        diger => panic!("beklenmeyen: {diger:?}"),
    }
}

#[test]
fn hiz_siniri_retry_after_ile_ve_onsuz() {
    assert_eq!(
        hata_esle(429, Some("12"), b"{}"),
        EvrenError::HizSiniri {
            tekrar_dene: Some(Duration::from_secs(12))
        }
    );
    assert_eq!(
        hata_esle(429, None, b""),
        EvrenError::HizSiniri { tekrar_dene: None }
    );
    assert_eq!(
        hata_esle(429, Some("Wed, 21 Oct 2026 07:28:00 GMT"), b""),
        EvrenError::HizSiniri { tekrar_dene: None },
        "tarih biçimli Retry-After süre olarak yorumlanmaz"
    );
}

/// Canlıda (2026-10-08) görülen yanıt: `Retry-After: 60` gelir, ama sınır
/// ancak `resets_at`'te açılır; bu yüzden hız sınırı sayılmaz.
const GUNLUK_SINIR: &[u8] = br#"{"error":{"message":"G\u00fcnl\u00fck token limitiniz a\u015f\u0131ld\u0131 (t\u00fcm API anahtarlar\u0131n\u0131z\u0131n toplam\u0131).","type":"rate_limit_error","param":null,"code":"daily_token_limit_exceeded","evren":{"resets_at":"2026-10-09T00:00:00Z","estimated_prompt_tokens":66744,"remaining_tokens":17761,"limit_tokens":15000000}}}"#;

#[test]
fn gunluk_token_siniri_hiz_sinirindan_ayrilir() {
    let h = hata_esle(429, Some("60"), GUNLUK_SINIR);
    assert_eq!(
        h,
        EvrenError::GunlukSinir {
            sifirlanma: Some("2026-10-09T00:00:00Z".into())
        }
    );
    assert_eq!(
        h.to_string(),
        "EVREN'in günlük token sınırına ulaşıldı (bütün anahtarlarınızın toplamı). \
         Sınır 2026-10-09T00:00:00Z itibarıyla sıfırlanır."
    );
}

#[test]
fn gunluk_sinirda_zaman_damgasi_olmayan_sifirlanma_gosterilmez() {
    for deger in ["\"yarın <b>sabah</b>\"", "\"\"", "12", "null"] {
        let govde = format!(
            r#"{{"error":{{"code":"daily_token_limit_exceeded","evren":{{"resets_at":{deger}}}}}}}"#
        );
        let h = hata_esle(429, None, govde.as_bytes());
        assert_eq!(h, EvrenError::GunlukSinir { sifirlanma: None }, "{deger}");
        assert!(h.to_string().ends_with("toplamı)."), "{h}");
    }
}

#[test]
fn json_olmayan_govde_kirpilarak_tasinir() {
    assert_eq!(
        hata_esle(502, None, b"  Bad Gateway  "),
        EvrenError::Sunucu {
            durum: 502,
            kod: None,
            mesaj: "Bad Gateway".into()
        }
    );
    let uzun = vec![b'x'; 2000];
    match hata_esle(500, None, &uzun) {
        EvrenError::Sunucu { mesaj, .. } => {
            assert_eq!(mesaj.chars().count(), 501, "500 karakter + üç nokta")
        }
        diger => panic!("{diger:?}"),
    }
}

#[test]
fn basliklardan_kredi_gozlemi() {
    let mut h = HeaderMap::new();
    h.insert(
        "x-evren-credits-remaining",
        HeaderValue::from_static("999.5000"),
    );
    h.insert("x-evren-credits-held", HeaderValue::from_static("0.5000"));
    h.insert("x-request-id", HeaderValue::from_static("f28c7cc1"));
    h.insert(
        "x-evren-routed-model",
        HeaderValue::from_static("qwen/qwen3.8-flash-next"),
    );
    let m = basliklardan(&h, 1_700_000_000_000);
    assert_eq!(
        m,
        YanitMeta {
            istek_id: Some("f28c7cc1".into()),
            yonlendirilen_model: Some("qwen/qwen3.8-flash-next".into()),
            kredi: Some(KrediGozlemi {
                kalan: Some(Decimal::from_str("999.5").unwrap()),
                tutulan: Some(Decimal::from_str("0.5").unwrap()),
                istek_id: Some("f28c7cc1".into()),
                zaman_ms: 1_700_000_000_000,
            }),
        }
    );
}

#[test]
fn kredi_basligi_yoksa_gozlem_yok_bozuksa_alan_bos() {
    let mut h = HeaderMap::new();
    h.insert("x-request-id", HeaderValue::from_static("r1"));
    let m = basliklardan(&h, 0);
    assert_eq!(m.kredi, None);
    assert_eq!(m.istek_id.as_deref(), Some("r1"));

    h.insert(
        "x-evren-credits-remaining",
        HeaderValue::from_static("bozuk"),
    );
    h.insert("x-evren-credits-held", HeaderValue::from_static("1.0000"));
    let k = basliklardan(&h, 0).kredi.expect("tutulan var");
    assert_eq!(k.kalan, None);
    assert_eq!(k.tutulan, Some(Decimal::ONE));
}

#[test]
fn kredi_gozlemi_serilestirilebilir() {
    let k = KrediGozlemi {
        kalan: Some(Decimal::from_str("1000.0000").unwrap()),
        tutulan: None,
        istek_id: None,
        zaman_ms: 5,
    };
    let geri: KrediGozlemi = serde_json::from_str(&serde_json::to_string(&k).unwrap()).unwrap();
    assert_eq!(geri, k);
}
#[test]
fn buyuk_hata_govdesi_dogrudan_cagrida_da_cozulmez() {
    let govde = vec![b'a'; kalfa_evren::butce::JSON_BAYT_SINIRI + 1];
    assert_eq!(
        kalfa_evren::hata::hata_esle(500, None, &govde),
        kalfa_evren::EvrenError::YanitSiniri
    );
}
