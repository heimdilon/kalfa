mod ortak;

use kalfa_evren::sse::{SseCozucu, SseHatasi, SseOlayi, VARSAYILAN_AZAMI_OLAY};

fn tek_seferde(veri: &[u8]) -> Vec<SseOlayi> {
    let mut c = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    c.besle(veri).expect("sınır aşılmamalı")
}

fn parcali(veri: &[u8], boyut: usize) -> Vec<SseOlayi> {
    let mut c = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    let mut hepsi = Vec::new();
    for parca in veri.chunks(boyut) {
        hepsi.extend(c.besle(parca).expect("sınır aşılmamalı"));
    }
    hepsi
}

fn veriler(olaylar: &[SseOlayi]) -> Vec<&str> {
    olaylar.iter().map(|o| o.veri.as_str()).collect()
}

#[test]
fn basit_olay() {
    let o = tek_seferde(b"data: merhaba\n\n");
    assert_eq!(
        o,
        vec![SseOlayi {
            olay: None,
            veri: "merhaba".into(),
            kimlik: None
        }]
    );
}

#[test]
fn yorum_satirlari_atlanir() {
    let o = tek_seferde(b": keep-alive\n\n: keep-alive\n\ndata: x\n\n");
    assert_eq!(veriler(&o), vec!["x"]);
}

#[test]
fn crlf_cr_ve_lf_satir_sonlari() {
    let o = tek_seferde(b"data: a\r\n\r\ndata: b\r\rdata: c\n\n");
    assert_eq!(veriler(&o), vec!["a", "b", "c"]);
}

#[test]
fn cok_satirli_data_birlesir() {
    let o = tek_seferde(b"data: bir\ndata: iki\n\n");
    assert_eq!(veriler(&o), vec!["bir\niki"]);
}

#[test]
fn degerdeki_bastaki_tek_bosluk_atilir() {
    let o = tek_seferde(b"data:  iki bosluk\n\ndata:bosluksuz\n\n");
    assert_eq!(veriler(&o), vec![" iki bosluk", "bosluksuz"]);
}

#[test]
fn event_ve_id_alanlari() {
    let o = tek_seferde(b"event: ping\nid: 7\ndata: {}\n\n");
    assert_eq!(
        o,
        vec![SseOlayi {
            olay: Some("ping".into()),
            veri: "{}".into(),
            kimlik: Some("7".into())
        }]
    );
}

#[test]
fn id_sonraki_olaylarda_kalici_olur() {
    let o = tek_seferde(
        b"id: 7\ndata: ilk\n\ndata: ikinci\n\nid: 9\ndata: ucuncu\n\ndata: dorduncu\n\n",
    );
    assert_eq!(o[0].kimlik.as_deref(), Some("7"));
    assert_eq!(
        o[1].kimlik.as_deref(),
        Some("7"),
        "id sonraki olayda da kalıcı olmalı"
    );
    assert_eq!(
        o[2].kimlik.as_deref(),
        Some("9"),
        "yeni id önceki değeri geçersiz kılmalı"
    );
    assert_eq!(
        o[3].kimlik.as_deref(),
        Some("9"),
        "yeni id de kalıcı olmalı"
    );
}

#[test]
fn bos_data_olay_uretir_ama_hic_data_yoksa_olay_yok() {
    let o = tek_seferde(b"data:\n\nevent: yalniz\n\n");
    assert_eq!(veriler(&o), vec![""]);
}

#[test]
fn eksik_son_olay_atilir_ve_bitir_bildirir() {
    let mut c = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    assert!(c.besle(b"data: tamam\n\ndata: yarim").unwrap().len() == 1);
    assert!(c.bitir(), "tamamlanmamış olay kaldığı bildirilmeli");

    let mut c = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    c.besle(b"data: tamam\n\n").unwrap();
    assert!(!c.bitir());
}

#[test]
fn bom_atilir() {
    let o = tek_seferde("\u{FEFF}data: x\n\n".as_bytes());
    assert_eq!(veriler(&o), vec!["x"]);
}

#[test]
fn boyut_siniri_asilinca_hata() {
    let mut c = SseCozucu::yeni(16);
    let hata = c.besle(&[b'a'; 32]).unwrap_err();
    assert_eq!(hata, SseHatasi::OlayCokBuyuk { azami: 16 });

    let mut c = SseCozucu::yeni(16);
    let hata = c
        .besle(b"data: 0123456789\ndata: 0123456789\n")
        .unwrap_err();
    assert_eq!(hata, SseHatasi::OlayCokBuyuk { azami: 16 });
}

#[test]
fn turkce_cok_bayti_karakter_her_bayt_sinirinda_bozulmaz() {
    let akis =
        "data: {\"metin\":\"İğüşçö ÇĞİÖŞÜ ı\"}\r\n\r\n: yorum\r\ndata: ikinci\r\n\r\n".as_bytes();
    let beklenen = tek_seferde(akis);
    assert_eq!(
        veriler(&beklenen),
        vec!["{\"metin\":\"İğüşçö ÇĞİÖŞÜ ı\"}", "ikinci"]
    );
    for boyut in 1..=akis.len() {
        assert_eq!(parcali(akis, boyut), beklenen, "parça boyutu {boyut}");
    }
    // CRLF'in CR ve LF'si ayrı beslemelerde gelse de tek satır sonu sayılır.
    for kesim in 0..=akis.len() {
        let mut c = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
        let mut o = c.besle(&akis[..kesim]).unwrap();
        o.extend(c.besle(&akis[kesim..]).unwrap());
        assert_eq!(o, beklenen, "kesim noktası {kesim}");
    }
}

fn fixture_dogrula(ad: &str) {
    let ham = ortak::fixture_bayt(ad);
    let metin = ortak::fixture_metin(ad);
    let beklenen_sayi = metin.lines().filter(|s| s.starts_with("data:")).count();

    let tum = tek_seferde(&ham);
    assert_eq!(
        tum.len(),
        beklenen_sayi,
        "{ad}: her data satırı bir olay olmalı"
    );
    assert_eq!(
        tum.last().map(|o| o.veri.as_str()),
        Some("[DONE]"),
        "{ad}: son olay [DONE]"
    );
    for boyut in [1, 2, 3, 7, 64, 1024] {
        assert_eq!(parcali(&ham, boyut), tum, "{ad}: parça boyutu {boyut}");
    }
}

#[test]
fn fixture_glm_akisi() {
    fixture_dogrula("chat_tool_calls_stream.sse");
}

#[test]
fn fixture_qwen_akisi() {
    fixture_dogrula("chat_tool_calls_stream_qwen_flash.sse");
}

#[test]
fn fixture_deepseek_akisi() {
    fixture_dogrula("chat_tool_calls_stream_deepseek_flash.sse");
}
#[test]
fn kalici_kimlik_tek_parcada_sinirsiz_kopyalanamaz() {
    let mut c = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    let ham = format!(
        "id: {}\n{}",
        "a".repeat(1024 * 1024),
        "data: {}\n\n".repeat(33)
    );
    assert!(c.besle(ham.as_bytes()).is_err());
}

#[test]
fn normal_buyuk_yanit_ag_parcalarina_bagli_degildir() {
    let ham = format!("data: {}\n\n", "a".repeat(1024 * 1024)).repeat(13);
    let mut c = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    assert_eq!(c.besle(ham.as_bytes()).unwrap().len(), 13);
    let mut c = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    let mut toplam = 0;
    for p in ham.as_bytes().chunks(23_777) {
        toplam += c.besle(p).unwrap().len();
    }
    assert_eq!(toplam, 13);
    let mut c = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    let adet = c
        .olaylar(ham.as_bytes())
        .try_fold(0, |adet, o| o.map(|_| adet + 1))
        .unwrap();
    assert_eq!(adet, 13);
}

#[test]
fn kimlikli_olaylar_teker_teker_birikmeden_tuketilir() {
    let ham = format!(
        "id: {}\n{}",
        "a".repeat(1024 * 1024),
        "data: {}\n\n".repeat(33)
    );
    let mut c = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    let mut adet = 0;
    for olay in c.olaylar(ham.as_bytes()) {
        let olay = olay.unwrap();
        assert_eq!(olay.kimlik.as_deref().unwrap().len(), 1024 * 1024);
        adet += 1;
    }
    assert_eq!(adet, 33);
}
