mod ortak;

#[test]
fn tek_parcanin_cok_sayida_cikti_olayi_da_sinirlanir() {
    let secim = serde_json::json!({"index":0,"delta":{"content":"a","reasoning":"b"}});
    let veri = serde_json::json!({"choices":vec![secim;40_000]}).to_string();
    assert!(veri.len() < VARSAYILAN_AZAMI_OLAY);
    let mut b = AkisBirlestirici::yeni();
    assert_eq!(b.veri_isle(&veri), Err(EvrenError::YanitSiniri));
    assert_eq!(b.bitir(), Err(EvrenError::YanitSiniri));
}

#[test]
fn butce_bitis_nedeninden_sonra_sifirlanmaz_ve_karisik_alanlari_sayar() {
    use kalfa_evren::butce::{YANIT_BAYT_SINIRI, YANIT_OLAY_SINIRI, YanitButcesi};
    let mut b = AkisBirlestirici::yeni();
    b.ekle(parca(r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"c","function":{"name":"x","arguments":"{}"}}]},"finish_reason":"tool_calls"}]}"#)).unwrap();
    for index in 1..128 {
        b.ekle(serde_json::from_value(serde_json::json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":index}]}}]})).unwrap()).unwrap();
    }
    assert!(matches!(
        b.ekle(parca(
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":128}]}}]}"#
        )),
        Err(EvrenError::YanitSiniri)
    ));
    assert_eq!(b.bitir(), Err(EvrenError::YanitSiniri));
    let mut b = YanitButcesi::default();
    b.olay_ekle(&AkisOlayi::Metin("ü".repeat(YANIT_BAYT_SINIRI / 4)))
        .unwrap();
    b.olay_ekle(&AkisOlayi::Dusunce("a".repeat(YANIT_BAYT_SINIRI / 2)))
        .unwrap();
    assert_eq!(
        b.olay_ekle(&AkisOlayi::Metin("a".into())),
        Err(EvrenError::YanitSiniri)
    );
    let mut b = YanitButcesi::default();
    for _ in 0..YANIT_OLAY_SINIRI {
        b.olay_ekle(&AkisOlayi::Metin(String::new())).unwrap();
    }
    assert_eq!(
        b.olay_ekle(&AkisOlayi::Metin(String::new())),
        Err(EvrenError::YanitSiniri)
    );
}

#[test]
fn yanit_boyunca_cagri_sayisi_ve_arguman_toplami_sinirlanir() {
    let mut b = AkisBirlestirici::yeni();
    for index in 0..128 {
        b.ekle(serde_json::from_value(serde_json::json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":index,"id":format!("c{index}"),"function":{"name":"x","arguments":"{}"}}]}}]})).unwrap()).unwrap();
    }
    assert!(
        b.ekle(
            serde_json::from_value(
                serde_json::json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":128}]}}]})
            )
            .unwrap()
        )
        .is_err()
    );
    assert!(b.bitir().is_err());
    let mut b = AkisBirlestirici::yeni();
    let veri = serde_json::json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"c","function":{"name":"x","arguments":"a".repeat(1024*1024)}}]}}]}).to_string();
    for _ in 0..31 {
        b.veri_isle(&veri).unwrap();
    }
    assert!(b.veri_isle(&veri).is_err());
    assert!(b.bitir().is_err());
}

use kalfa_evren::akis::VeriSonucu;
use kalfa_evren::sse::{SseCozucu, VARSAYILAN_AZAMI_OLAY};
use kalfa_evren::tipler::ChatChunk;
use kalfa_evren::{AkisBirlestirici, AkisOlayi, EvrenError};

/// Fixture'ı SSE → birleştirici hattından geçirir; `[DONE]` gelince bitirir.
fn hattan_gecir(ham: &[u8]) -> Result<Vec<AkisOlayi>, EvrenError> {
    let mut sse = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    let mut b = AkisBirlestirici::yeni();
    let mut olaylar = Vec::new();
    for o in sse.besle(ham).unwrap() {
        match b.veri_isle(&o.veri)? {
            VeriSonucu::Olaylar(v) => olaylar.extend(v),
            VeriSonucu::Bitti => {
                olaylar.extend(b.bitir()?);
                return Ok(olaylar);
            }
        }
    }
    olaylar.extend(b.bitir()?);
    Ok(olaylar)
}

fn arac_cagrilari(olaylar: &[AkisOlayi]) -> Vec<(String, String)> {
    olaylar
        .iter()
        .filter_map(|o| match o {
            AkisOlayi::AracCagrilari(v) => Some(v.clone()),
            _ => None,
        })
        .flatten()
        .map(|c| (c.function.name, c.function.arguments))
        .collect()
}

fn dusunce(olaylar: &[AkisOlayi]) -> String {
    olaylar
        .iter()
        .filter_map(|o| {
            if let AkisOlayi::Dusunce(s) = o {
                Some(s.as_str())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn glm_akisi_istanbul_ascii() {
    let o = hattan_gecir(&ortak::fixture_bayt("chat_tool_calls_stream.sse")).unwrap();
    assert_eq!(
        arac_cagrilari(&o),
        vec![(
            "hava_durumu".to_string(),
            "{\"sehir\": \"Istanbul\"}".to_string()
        )]
    );
    assert!(
        !dusunce(&o).is_empty(),
        "delta.reasoning ayrı akış olarak gelmeli"
    );
    let kullanim = o
        .iter()
        .find_map(|x| {
            if let AkisOlayi::Kullanim(u) = x {
                Some(u.clone())
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(kullanim.reasoning_tokens(), 51);
    assert!(kullanim.evren.is_none(), "usage.evren akışta gelmez (§3.3)");
    assert_eq!(
        o.last(),
        Some(&AkisOlayi::Bitti {
            neden: "tool_calls".into()
        })
    );
    let arac_sirasi = o
        .iter()
        .position(|x| matches!(x, AkisOlayi::AracCagrilari(_)))
        .unwrap();
    let kullanim_sirasi = o
        .iter()
        .position(|x| matches!(x, AkisOlayi::Kullanim(_)))
        .unwrap();
    assert!(
        arac_sirasi < kullanim_sirasi,
        "araç çağrıları finish_reason ile, usage'dan önce"
    );
}

#[test]
fn qwen_akisi_turkce_karakter_token_sinirinda() {
    let o = hattan_gecir(&ortak::fixture_bayt(
        "chat_tool_calls_stream_qwen_flash.sse",
    ))
    .unwrap();
    assert_eq!(
        arac_cagrilari(&o),
        vec![(
            "hava_durumu".to_string(),
            "{\"sehir\": \"İzmir\"}".to_string()
        )]
    );
    assert_eq!(
        o.last(),
        Some(&AkisOlayi::Bitti {
            neden: "tool_calls".into()
        })
    );
}

#[test]
fn deepseek_akisi() {
    let o = hattan_gecir(&ortak::fixture_bayt(
        "chat_tool_calls_stream_deepseek_flash.sse",
    ))
    .unwrap();
    assert_eq!(
        arac_cagrilari(&o),
        vec![(
            "hava_durumu".to_string(),
            "{\"sehir\": \"İzmir\"}".to_string()
        )]
    );
}

#[test]
fn yarim_kalan_akis_hata_verir_ve_arac_cagrisi_yaymaz() {
    let metin = ortak::fixture_metin("chat_tool_calls_stream.sse");
    let bitis = metin
        .find("\"finish_reason\": \"tool_calls\"")
        .expect("fixture'da finish satırı");
    let kes = metin[..bitis].rfind("data:").unwrap();
    let mut sse = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    let mut b = AkisBirlestirici::yeni();
    let mut olaylar = Vec::new();
    for o in sse.besle(&metin.as_bytes()[..kes]).unwrap() {
        if let VeriSonucu::Olaylar(v) = b.veri_isle(&o.veri).unwrap() {
            olaylar.extend(v);
        }
    }
    assert!(
        !olaylar
            .iter()
            .any(|o| matches!(o, AkisOlayi::AracCagrilari(_))),
        "tamamlanmamış çağrı yayılmaz"
    );
    assert_eq!(b.bitir(), Err(EvrenError::AkisYarimKaldi));
}

fn parca(json: &str) -> ChatChunk {
    serde_json::from_str(json).unwrap()
}

#[test]
fn yalnizca_metin_ve_bos_parcalar() {
    let mut b = AkisBirlestirici::yeni();
    let mut o = Vec::new();
    o.extend(
        b.ekle(parca(
            r#"{"choices":[{"index":0,"delta":{"role":"assistant","content":""}}]}"#,
        ))
        .unwrap(),
    );
    o.extend(
        b.ekle(parca(
            r#"{"choices":[{"index":0,"delta":{"content":"Mer"}}]}"#,
        ))
        .unwrap(),
    );
    o.extend(
        b.ekle(parca(
            r#"{"choices":[{"index":0,"delta":{"content":"haba"},"finish_reason":"stop"}]}"#,
        ))
        .unwrap(),
    );
    o.extend(b.bitir().unwrap());
    assert_eq!(
        o,
        vec![
            AkisOlayi::Metin("Mer".into()),
            AkisOlayi::Metin("haba".into()),
            AkisOlayi::Bitti {
                neden: "stop".into()
            }
        ]
    );
    assert_eq!(b.bitir().unwrap(), vec![], "ikinci bitir boş döner");
}

#[test]
fn adi_hic_gelmeyen_arac_cagrisi_gecersiz_yanittir() {
    let mut b = AkisBirlestirici::yeni();
    b.ekle(parca(r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"c1","function":{"arguments":"{}"}}]}}]}"#))
        .unwrap();
    let hata = b
        .ekle(parca(
            r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#,
        ))
        .unwrap_err();
    assert!(matches!(hata, EvrenError::GecersizYanit(_)));
}

#[test]
fn paralel_iki_cagri_index_sirasiyla() {
    let mut b = AkisBirlestirici::yeni();
    b.ekle(parca(r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":1,"id":"b","function":{"name":"y","arguments":"{\"a\":"}}]}}]}"#)).unwrap();
    b.ekle(parca(r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"a","function":{"name":"x","arguments":"{}"}}]}}]}"#)).unwrap();
    b.ekle(parca(r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":1,"function":{"arguments":"1}"}}]}}]}"#)).unwrap();
    let o = b
        .ekle(parca(
            r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#,
        ))
        .unwrap();
    assert_eq!(
        arac_cagrilari(&o),
        vec![("x".into(), "{}".into()), ("y".into(), "{\"a\":1}".into())]
    );
}

#[test]
fn bozuk_parca_gecersiz_yanit() {
    let mut b = AkisBirlestirici::yeni();
    assert!(matches!(
        b.veri_isle("{bozuk"),
        Err(EvrenError::GecersizYanit(_))
    ));
    assert_eq!(b.veri_isle(" [DONE] ").unwrap(), VeriSonucu::Bitti);
}
