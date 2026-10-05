//! Uçtan uca test derlemesinin parçaları (1c tasarımı §11.4): ayarlar, sahte
//! sağlayıcının senaryoları, aktarım sayaçları ve uzun kayıt üreteci.
//! `cargo test -p kalfa-uygulama --features e2e` ile koşar.
#![cfg(feature = "e2e")]

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use kalfa_core::aktarim::{AktarimAyarlari, KanalMesaji};
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::protokol::{AktarimDurmaNedeni, BitisNedeni, Olay};
use kalfa_core::saglayici::Saglayici;
use kalfa_evren::tipler::{ChatMessage, ChatRequest, FunctionCall, ToolCall};
use kalfa_evren::{AkisOlayi, EvrenError};
use kalfa_sandbox::Yalitimsiz;
use kalfa_uygulama::anahtar::AnahtarKasasi;
use kalfa_uygulama::arayuz::Iletisim;
use kalfa_uygulama::dizin::{kayit_yolu, oturum_dizini};
use kalfa_uygulama::e2e::{
    AGSIZ_ANAHTAR, DAMGA_ONEKI, E2eAyarlari, GECERSIZ_ANAHTAR, SahteIletisim, SahteUretici,
    SartBaslangici, olcumleri_sifirla,
};
use kalfa_uygulama::evren::SaglayiciUretici;
use kalfa_uygulama::test_destek::{SahteAcici, SahteKanal, SahtePencereler};
use kalfa_uygulama::yalitim::YalitimKaynagi;
use kalfa_uygulama::{Bilesenler, Uygulama};
use serde_json::json;

fn ayarlar(deger: serde_json::Value) -> E2eAyarlari {
    serde_json::from_value(deger).unwrap()
}

fn saglayici(ayarlar: &E2eAyarlari) -> Arc<dyn Saglayici> {
    SahteUretici::yeni(ayarlar).uret("evren_llm_e2e").unwrap()
}

async fn olaylar(s: &dyn Saglayici, mesajlar: Vec<ChatMessage>) -> Vec<AkisOlayi> {
    let akis = s.akis(ChatRequest::new("glm-5.3", mesajlar)).await.unwrap();
    akis.olaylar.map(|o| o.unwrap()).collect().await
}

fn metin(olay: &AkisOlayi) -> &str {
    match olay {
        AkisOlayi::Metin(m) | AkisOlayi::Dusunce(m) => m,
        diger => panic!("metin beklenirdi: {diger:?}"),
    }
}

/// `[[e2e:<no>:<ms>]]` damgasının numarası ve zamanı.
fn damga(parca: &str) -> (u32, f64) {
    let ic = parca
        .strip_prefix(DAMGA_ONEKI)
        .and_then(|p| p.strip_suffix("]]"))
        .unwrap_or_else(|| panic!("damga değil: {parca}"));
    let (no, ms) = ic.split_once(':').unwrap();
    (no.parse().unwrap(), ms.parse().unwrap())
}

fn simdi_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
        * 1000.0
}

#[test]
fn ayarlar_dosyadan_okunur_bilinmeyen_alan_reddedilir() {
    let dizin = tempfile::tempdir().unwrap();
    let yol = dizin.path().join("e2e.json");
    let alan = dizin.path().join("alan");
    std::fs::write(
        &yol,
        json!({
            "sart": "kabul",
            "anahtar": "evren_llm_e2e",
            "calisma_alani": alan,
            "onay": true,
            "senaryolar": {
                "yaz": {"yanitlar": [[{"tur": "metin", "metin": "Tamam."}]]}
            }
        })
        .to_string(),
    )
    .unwrap();
    let a = E2eAyarlari::oku(&yol).unwrap();
    assert_eq!(a.sart, SartBaslangici::Kabul);
    assert_eq!(a.anahtar.as_deref(), Some("evren_llm_e2e"));
    assert_eq!(a.calisma_alani.as_deref(), Some(alan.as_path()));
    assert!(a.onay);
    assert_eq!(a.senaryolar["yaz"].yanitlar.len(), 1);

    std::fs::write(&yol, r#"{"sart": "kabul", "bilinmeyen": 1}"#).unwrap();
    assert!(E2eAyarlari::oku(&yol).is_err());

    let bos = E2eAyarlari::default();
    assert_eq!(bos.sart, SartBaslangici::Gerekli);
    assert!(!bos.onay);
}

#[tokio::test]
async fn sahte_iletisim_ayarlardaki_cevabi_verir() {
    let a = ayarlar(json!({"calisma_alani": "/tmp/alan", "onay": true}));
    let i = SahteIletisim::yeni(&a);
    assert_eq!(
        i.klasor_sec("x", None).await.as_deref(),
        Some(std::path::Path::new("/tmp/alan"))
    );
    assert!(i.onayla("b", "m", "Evet", "Hayır").await);
    let hayir = SahteIletisim::yeni(&E2eAyarlari::default());
    assert_eq!(hayir.klasor_sec("x", None).await, None);
    assert!(!hayir.onayla("b", "m", "Evet", "Hayır").await);
}

#[tokio::test]
async fn eslesmeyen_mesaja_yanit_doner() {
    let s = saglayici(&ayarlar(json!({"sart": "kabul"})));
    let o = olaylar(s.as_ref(), vec![ChatMessage::user("  merhaba  ")]).await;
    assert_eq!(
        o,
        vec![
            AkisOlayi::Metin("Yanıt: merhaba".into()),
            AkisOlayi::Bitti {
                neden: "stop".into()
            }
        ]
    );
}

#[tokio::test]
async fn senaryo_turun_isteklerine_sirayla_yanit_verir() {
    let a = ayarlar(json!({
        "sart": "kabul",
        "senaryolar": {"yaz": {"yanitlar": [
            [
                {"tur": "dusunce", "metin": "Düşünüyorum."},
                {"tur": "metin", "metin": "Yazıyorum."},
                {"tur": "arac", "ad": "dosya_yaz", "argumanlar": {"yol": "a.txt", "icerik": "x"}}
            ],
            [{"tur": "arac_sonucu"}]
        ]}}
    }));
    let s = saglayici(&a);

    let ilk = olaylar(s.as_ref(), vec![ChatMessage::user("yaz")]).await;
    assert_eq!(ilk[0], AkisOlayi::Dusunce("Düşünüyorum.".into()));
    assert_eq!(ilk[1], AkisOlayi::Metin("Yazıyorum.".into()));
    let AkisOlayi::AracCagrilari(cagrilar) = &ilk[2] else {
        panic!("araç çağrısı beklenirdi: {:?}", ilk[2]);
    };
    assert_eq!(cagrilar.len(), 1);
    assert_eq!(cagrilar[0].function.name, "dosya_yaz");
    let argumanlar: serde_json::Value =
        serde_json::from_str(&cagrilar[0].function.arguments).unwrap();
    assert_eq!(argumanlar, json!({"yol": "a.txt", "icerik": "x"}));
    assert_eq!(
        ilk[3],
        AkisOlayi::Bitti {
            neden: "tool_calls".into()
        }
    );

    let cagri = ToolCall {
        id: cagrilar[0].id.clone(),
        kind: "function".into(),
        function: FunctionCall {
            name: "dosya_yaz".into(),
            arguments: "{}".into(),
        },
    };
    let ikinci = olaylar(
        s.as_ref(),
        vec![
            ChatMessage::user("yaz"),
            ChatMessage::assistant(None, vec![cagri.clone()]),
            ChatMessage::tool(cagri.id.clone(), "Kullanıcı reddetti: başka ad kullan"),
        ],
    )
    .await;
    assert_eq!(
        ikinci,
        vec![
            AkisOlayi::Metin("Araç sonucu: Kullanıcı reddetti: başka ad kullan".into()),
            AkisOlayi::Bitti {
                neden: "stop".into()
            }
        ]
    );

    // Senaryonun yanıtları bitti: tur kapanır.
    let ucuncu = olaylar(
        s.as_ref(),
        vec![
            ChatMessage::user("yaz"),
            ChatMessage::assistant(None, vec![cagri.clone()]),
            ChatMessage::tool(cagri.id.clone(), "tamam"),
            ChatMessage::assistant(Some("Araç sonucu: tamam".into()), Vec::new()),
        ],
    )
    .await;
    assert_eq!(ucuncu[0], AkisOlayi::Metin("Tamamlandı.".into()));
}

#[tokio::test]
async fn damgali_desenler_sirali_ve_zamanli_parca_uretir() {
    let a = ayarlar(json!({
        "sart": "kabul",
        "senaryolar": {
            "duzenli": {"yanitlar": [[{"tur": "damgali", "sayi": 5,
                "desen": {"ad": "duzenli", "aralik_ms": 2}}]]},
            "yigin": {"yanitlar": [[{"tur": "damgali", "sayi": 12,
                "desen": {"ad": "yigin", "en_az": 5, "en_cok": 6, "aralik_ms": 2}}]]},
            "paralel": {"yanitlar": [[{"tur": "damgali", "sayi": 3,
                "desen": {"ad": "paralel", "aralik_ms": 2}}]]}
        }
    }));
    let s = saglayici(&a);
    let once = simdi_ms();

    let duzenli = olaylar(s.as_ref(), vec![ChatMessage::user("duzenli")]).await;
    assert_eq!(duzenli.len(), 6);
    let damgalar: Vec<(u32, f64)> = duzenli[..5].iter().map(|o| damga(metin(o))).collect();
    assert_eq!(
        damgalar.iter().map(|d| d.0).collect::<Vec<_>>(),
        [1, 2, 3, 4, 5]
    );
    assert!(damgalar.windows(2).all(|w| w[0].1 <= w[1].1));
    assert!(damgalar[0].1 >= once - 1.0 && damgalar[4].1 <= simdi_ms() + 1.0);

    let yigin = olaylar(s.as_ref(), vec![ChatMessage::user("yigin")]).await;
    assert_eq!(yigin.len(), 13, "12 parça ve bitiş");
    assert!(yigin[..12].iter().all(|o| matches!(o, AkisOlayi::Metin(_))));

    let paralel = olaylar(s.as_ref(), vec![ChatMessage::user("paralel")]).await;
    let turler: Vec<&str> = paralel[..6]
        .iter()
        .map(|o| match o {
            AkisOlayi::Dusunce(_) => "d",
            AkisOlayi::Metin(_) => "m",
            _ => "?",
        })
        .collect();
    assert_eq!(turler, ["d", "m", "d", "m", "d", "m"]);
}

#[tokio::test]
async fn sel_okundukca_akar_birakilinca_durur() {
    let a = ayarlar(json!({
        "sart": "kabul",
        "senaryolar": {"sel": {"yanitlar": [[{"tur": "sel", "parca_bayt": 1024}]]}}
    }));
    let s = saglayici(&a);
    let akis = s
        .akis(ChatRequest::new("glm-5.3", vec![ChatMessage::user("sel")]))
        .await
        .unwrap();
    let alinan: Vec<AkisOlayi> = akis.olaylar.take(500).map(|o| o.unwrap()).collect().await;
    assert_eq!(alinan.len(), 500);
    assert!(alinan.iter().all(|o| metin(o).len() == 1024));
}

#[tokio::test]
async fn anahtara_gore_hata_ve_ortak_sart_durumu() {
    let uretici = SahteUretici::yeni(&E2eAyarlari::default());
    let gecersiz = uretici.uret(GECERSIZ_ANAHTAR).unwrap();
    assert!(matches!(
        gecersiz.sart_durumu().await,
        Err(EvrenError::AnahtarGecersiz)
    ));
    let agsiz = uretici.uret(AGSIZ_ANAHTAR).unwrap();
    assert!(matches!(
        agsiz.sart_durumu().await,
        Err(EvrenError::Ag { .. })
    ));

    let a = uretici.uret("evren_llm_a").unwrap();
    let b = uretici.uret("evren_llm_b").unwrap();
    assert!(!a.sart_durumu().await.unwrap().accepted);
    let istek = ChatRequest::new("glm-5.3", vec![ChatMessage::user("merhaba")]);
    assert!(matches!(
        a.akis(istek.clone()).await,
        Err(EvrenError::SartKabulGerekli { surum: Some(1) })
    ));
    assert!(b.sart_kabul(2).await.is_err());
    b.sart_kabul(1).await.unwrap();
    assert!(a.sart_durumu().await.unwrap().accepted);
    assert!(a.akis(istek).await.is_ok());
}

/// Anahtarı kasada, şartları kabul edilmiş, açılmış uygulama.
async fn uygulama(a: &E2eAyarlari, veri: &std::path::Path) -> Arc<Uygulama> {
    let kasa = AnahtarKasasi::yeni(None);
    kasa.yaz("evren_llm_e2e");
    let u = Uygulama::yeni(Bilesenler {
        veri_dizini: veri.to_path_buf(),
        kasa,
        uretici: Arc::new(SahteUretici::yeni(a)),
        iletisim: Arc::new(SahteIletisim::yeni(a)),
        acici: Arc::new(SahteAcici::default()),
        pencereler: Arc::new(SahtePencereler::default()),
        yukseltici: Arc::new(kalfa_uygulama::test_destek::SahteYukseltici::default()),
        aktarim: AktarimAyarlari::default(),
        yalitim_dizini: veri.join("yalitim"),
        geri_alma_dizini: veri.join("geri-alma"),
        kalfa_araclari: Vec::new(),
        ev_dizini: std::env::temp_dir(),
        yalitim: YalitimKaynagi::Kurucu(Box::new(|_| Arc::new(Yalitimsiz))),
    });
    u.baslat().await;
    u
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn islenmeyen_paketler_krediyi_ve_kuyruk_butcesini_asmaz() {
    let veri = tempfile::tempdir().unwrap();
    let alan = tempfile::tempdir().unwrap();
    let a = ayarlar(json!({
        "sart": "kabul",
        "calisma_alani": alan.path(),
        "senaryolar": {"sel": {"yanitlar": [[{"tur": "sel", "parca_bayt": 4096}]]}}
    }));
    let u = uygulama(&a, veri.path()).await;
    let secim = u.calisma_alani_sec().await.unwrap();
    let oturum = u
        .oturum_baslat(secim.kimlik, "glm-5.3".into())
        .await
        .unwrap()
        .oturum;
    let (kanal, mut rx) = SahteKanal::yeni();
    olcumleri_sifirla();
    u.oturumu_dinle(oturum, kanal).unwrap();
    u.mesaj_gonder(oturum, "sel".into()).await.unwrap();

    // Arayüz paketleri işlemiyor: kredi dolar, çekirdek kuyruğu bütçeye
    // kadar dolar ve tur iptal edilir. Sonra arayüz işlemeye başlar; kuyruğun
    // son olayı `AktarimDurdu`dur.
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let mut durdu = None;
    loop {
        let mesaj = tokio::time::timeout(Duration::from_secs(20), rx.recv())
            .await
            .expect("kanal mesajı gelmedi")
            .expect("kanal kapandı");
        let KanalMesaji::Olaylar { no, zarflar } = mesaj else {
            break;
        };
        for z in zarflar {
            if let Olay::AktarimDurdu { neden } = z.yuk {
                durdu = Some(neden);
            }
        }
        u.paketi_isledim(oturum, no).unwrap();
    }
    assert_eq!(durdu, Some(AktarimDurmaNedeni::TuketiciYetismedi));
    let kayit = kayit_yolu(&oturum_dizini(veri.path()), oturum);
    let iptal = OturumKaydi::oku(&kayit).unwrap().iter().any(|z| {
        matches!(
            z.yuk,
            KayitOgesi::TurBitti {
                neden: BitisNedeni::Iptal
            }
        )
    });
    assert!(iptal, "kayıtta iptal edilen tur yok");

    let o = u.e2e_olcumler();
    assert!(o.yolda_paket_en_cok <= o.kredi_paket, "{o:?}");
    assert!(o.yolda_bayt_en_cok <= o.kredi_bayt, "{o:?}");
    assert!(o.kuyruk_olay_en_cok <= o.butce_olay, "{o:?}");
    assert!(o.kuyruk_bayt_en_cok <= o.butce_bayt, "{o:?}");
    assert!(
        o.kuyruk_bayt_en_cok > o.butce_bayt / 2,
        "kuyruk dolmadı: {o:?}"
    );
    assert_eq!(o.yolda_paket_en_cok, o.kredi_paket, "{o:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn kayit_ureteci_istenen_sayida_tur_yazar() {
    let veri = tempfile::tempdir().unwrap();
    let alan = tempfile::tempdir().unwrap();
    let a = ayarlar(json!({"sart": "kabul", "calisma_alani": alan.path()}));
    let u = uygulama(&a, veri.path()).await;

    let oturum = u
        .e2e_kayit_uret(alan.path().to_path_buf(), 25)
        .await
        .unwrap();
    let kayitlar = OturumKaydi::oku(&kayit_yolu(&oturum_dizini(veri.path()), oturum)).unwrap();
    let bitenler = kayitlar
        .iter()
        .filter(|z| {
            matches!(
                z.yuk,
                KayitOgesi::TurBitti {
                    neden: BitisNedeni::Normal
                }
            )
        })
        .count();
    assert_eq!(bitenler, 25);

    // Kayıt kilidi bırakıldı: oturum kayıttan açılır; son 20 tur gelir.
    let acilis = u.oturum_ac(oturum).await.unwrap();
    assert!(acilis.salt_okunur.is_none());
    assert!(acilis.gecmis.daha_eski_var);
}
