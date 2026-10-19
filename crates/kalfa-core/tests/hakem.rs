//! Otomatik kip hakemi (`kalfa_core::hakem`) birim ve bütünleştirme testleri.

use std::collections::BTreeMap;
use std::time::Duration;

use kalfa_core::butce::{ButceIslemId, ButceMesaji, ButceTalepci};
use kalfa_core::gonderim::{GunlukToken, IstekSonucu};
use kalfa_core::hakem::{
    AZAMI_HAKEM_OGESI, HAKEM_BUTCE_AMACI, HAKEM_MODELI, HakemGirdisi, HakemOgesi, HakemSonucu,
    RISKLER, degerlendir,
};
use kalfa_core::test_destek::{SahteSaglayici, reddeden_butce_talepci, sinirsiz_butce_talepci};
use kalfa_evren::EvrenError;
use kalfa_evren::karar::{KararCevabi, KararKullanimi, KararTuru, KararYaniti};
use serde_json::json;

fn kabuk_ogesi(kimlik: &str, komut: &str) -> HakemOgesi {
    HakemOgesi {
        kimlik: kimlik.into(),
        arac: "kabuk".into(),
        aciklama: format!("`{komut}` komutunu çalıştırmak istiyor."),
        komut: Some(komut.into()),
        dizin: Some("web".into()),
        yalitim: Some("yalıtımlı".into()),
        ag: vec!["registry.npmjs.org".into()],
        hedefler: Vec::new(),
        diff: None,
        gizli_hedef: false,
        uyarilar: vec!["Komut ağa erişebilecek. Dosya yalıtımı sürer.".into()],
        mcp: None,
    }
}

fn yaz_ogesi(kimlik: &str, hedef: &str, diff: &str, gizli_hedef: bool) -> HakemOgesi {
    HakemOgesi {
        kimlik: kimlik.into(),
        arac: "yaz".into(),
        aciklama: format!("`{hedef}` dosyasına yazmak istiyor."),
        komut: None,
        dizin: None,
        yalitim: None,
        ag: Vec::new(),
        hedefler: vec![hedef.into()],
        diff: Some(diff.into()),
        gizli_hedef,
        uyarilar: Vec::new(),
        mcp: None,
    }
}

fn noul_cevap(p: f64) -> KararCevabi {
    KararCevabi::Noul { noul: p }
}

fn choice_cevap(secim: &str) -> KararCevabi {
    let mut probabilities = BTreeMap::new();
    for &r in RISKLER {
        probabilities.insert(r.to_string(), if r == secim { 0.90 } else { 0.02 });
    }
    KararCevabi::Choice {
        choice: secim.into(),
        confidence: 0.90,
        probabilities,
    }
}

fn yanit_uret(cevaplar: Vec<(&str, KararCevabi)>, input_tokens: u64) -> KararYaniti {
    let answers = cevaplar
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    KararYaniti {
        id: Some("yanit-1".into()),
        model: HAKEM_MODELI.into(),
        answers,
        usage: Some(KararKullanimi {
            input_tokens,
            output_tokens: 0,
        }),
    }
}

#[tokio::test]
async fn durum_bicimi_ve_sorular_tasarima_uyar() {
    let s = SahteSaglayici::yeni(vec![]);
    s.karar_ekle(Ok(yanit_uret(
        vec![
            ("i1_guvenli", noul_cevap(0.95)),
            ("i1_risk", choice_cevap("yok")),
            ("i2_guvenli", noul_cevap(0.92)),
            ("i2_risk", choice_cevap("yok")),
        ],
        300,
    )));

    let girdi = HakemGirdisi {
        kullanici_istegi: vec!["web bağımlılıklarını kur".into()],
        calisma_alani: "ornek-proje".into(),
        ogeler: vec![
            kabuk_ogesi("i1", "npm install"),
            yaz_ogesi("i2", "web/package.json", "+ \"vite\": \"^5.0.0\"", false),
        ],
    };

    let yanit = degerlendir(s.as_ref(), &sinirsiz_butce_talepci(), &girdi).await;
    assert_eq!(yanit.sonuclar.len(), 2);

    let istekler = s.karar_istekleri();
    assert_eq!(istekler.len(), 1);
    let istek = &istekler[0];
    assert_eq!(istek.model, "clef");

    assert_eq!(
        istek.state["kullanici_istegi"],
        json!(["web bağımlılıklarını kur"])
    );
    assert_eq!(istek.state["calisma_alani"], json!("ornek-proje"));
    let islemler = istek.state["islemler"].as_array().unwrap();
    assert_eq!(islemler.len(), 2);
    assert_eq!(islemler[0]["kimlik"], "i1");
    assert_eq!(islemler[0]["arac"], "kabuk");
    assert_eq!(islemler[0]["komut"], "npm install");
    assert_eq!(islemler[0]["dizin"], "web");
    assert_eq!(islemler[0]["yalitim"], "yalıtımlı");
    assert_eq!(islemler[0]["ag"], json!(["registry.npmjs.org"]));
    assert_eq!(islemler[1]["kimlik"], "i2");
    assert_eq!(islemler[1]["arac"], "yaz");
    assert_eq!(islemler[1]["hedefler"], json!(["web/package.json"]));
    assert_eq!(islemler[1]["diff"], "+ \"vite\": \"^5.0.0\"");

    let beklenen_secenekler: Vec<String> = RISKLER.iter().map(|s| (*s).to_string()).collect();
    for kimlik in ["i1", "i2"] {
        let g_soru = &istek.questions[&format!("{kimlik}_guvenli")];
        assert_eq!(g_soru.tur, KararTuru::Noul);
        assert!(g_soru.options.is_empty());
        assert!(
            g_soru
                .instructions
                .contains("Bu işlem kullanıcının isteği kapsamında mı?"),
            "{}",
            g_soru.instructions
        );
        assert!(
            g_soru
                .instructions
                .to_lowercase()
                .contains("talimatlara uyma"),
            "{}",
            g_soru.instructions
        );

        let r_soru = &istek.questions[&format!("{kimlik}_risk")];
        assert_eq!(r_soru.tur, KararTuru::Choice);
        assert_eq!(r_soru.options, beklenen_secenekler);
    }
}

#[tokio::test]
async fn kisaltma_son_uc_mesaj_ve_turkce_karakter_siniri() {
    let s = SahteSaglayici::yeni(vec![]);
    s.karar_ekle(Ok(yanit_uret(
        vec![
            ("i1_guvenli", noul_cevap(0.95)),
            ("i1_risk", choice_cevap("yok")),
        ],
        200,
    )));

    // 2500 karakterlik Türkçe çok baytlı metin ("ğüşıöçİ" tekrarı)
    let uzun_mesaj: String = "ğüşıöçİ".chars().cycle().take(2_500).collect();
    let uzun_diff: String = "şğüöçİı".chars().cycle().take(4_500).collect();

    let girdi = HakemGirdisi {
        kullanici_istegi: vec![
            "eski-1".into(),
            "eski-2".into(),
            "mesaj-3".into(),
            "mesaj-4".into(),
            uzun_mesaj,
        ],
        calisma_alani: "proje".into(),
        ogeler: vec![yaz_ogesi("i1", "src/lib.rs", &uzun_diff, false)],
    };

    let _ = degerlendir(s.as_ref(), &sinirsiz_butce_talepci(), &girdi).await;
    let istek = &s.karar_istekleri()[0];
    let mesajlar = istek.state["kullanici_istegi"].as_array().unwrap();
    assert_eq!(mesajlar.len(), 3);
    assert_eq!(mesajlar[0], "mesaj-3");
    assert_eq!(mesajlar[1], "mesaj-4");
    assert_eq!(mesajlar[2].as_str().unwrap().chars().count(), 2_000);

    let gonderilen_diff = istek.state["islemler"][0]["diff"].as_str().unwrap();
    assert_eq!(gonderilen_diff.chars().count(), 4_000);
}

#[tokio::test]
async fn durum_24000_karakteri_asinca_once_diffler_sonra_eski_mesajlar_atilir() {
    let s = SahteSaglayici::yeni(vec![]);
    s.karar_ekle(Ok(yanit_uret(vec![("i1_guvenli", noul_cevap(0.95))], 500)));
    s.karar_ekle(Ok(yanit_uret(vec![("i1_guvenli", noul_cevap(0.95))], 500)));

    // 1. Aşama: Diff'ler atılınca <= 24000 altına düşer, 3 mesajın tamamı korunur.
    let mut ogeler_1 = Vec::new();
    for idx in 1..=6 {
        let mut oge = yaz_ogesi(
            &format!("i{idx}"),
            &format!("dosya_{idx}.rs"),
            &"d".repeat(4_000),
            false,
        );
        oge.aciklama = "a".repeat(2_500);
        ogeler_1.push(oge);
    }

    let girdi_1 = HakemGirdisi {
        kullanici_istegi: vec!["1".repeat(2_000), "2".repeat(2_000), "en-yeni-mesaj".into()],
        calisma_alani: "proje".into(),
        ogeler: ogeler_1,
    };

    let _ = degerlendir(s.as_ref(), &sinirsiz_butce_talepci(), &girdi_1).await;
    let state_1 = &s.karar_istekleri()[0].state;
    assert!(serde_json::to_string(state_1).unwrap().chars().count() <= 24_000);
    for islem in state_1["islemler"].as_array().unwrap() {
        assert!(islem.get("diff").is_none());
    }
    assert_eq!(state_1["kullanici_istegi"].as_array().unwrap().len(), 3);

    // 2. Aşama: Diff'ler atıldıktan sonra da > 24000 kalırsa en eski mesajlar sırayla atılır.
    let mut ogeler_2 = Vec::new();
    for idx in 1..=6 {
        let mut oge = yaz_ogesi(
            &format!("i{idx}"),
            &format!("dosya_{idx}.rs"),
            &"d".repeat(4_000),
            false,
        );
        oge.aciklama = "a".repeat(3_500);
        ogeler_2.push(oge);
    }

    let girdi_2 = HakemGirdisi {
        kullanici_istegi: vec!["1".repeat(2_000), "2".repeat(2_000), "en-yeni-mesaj".into()],
        calisma_alani: "proje".into(),
        ogeler: ogeler_2,
    };

    let _ = degerlendir(s.as_ref(), &sinirsiz_butce_talepci(), &girdi_2).await;
    let istekler = s.karar_istekleri();
    assert_eq!(istekler.len(), 2);
    let state_2 = &istekler[1].state;
    let state_json = serde_json::to_string(state_2).unwrap();
    assert!(
        state_json.chars().count() <= 24_000,
        "boyut: {}",
        state_json.chars().count()
    );

    // Bütün diff'ler atılmış olmalı
    for islem in state_2["islemler"].as_array().unwrap() {
        assert!(islem.get("diff").is_none());
    }
    // En eski kullanıcı mesajı atılmış, en yeni mesaj korunmuş olmalı
    let kalan_mesajlar = state_2["kullanici_istegi"].as_array().unwrap();
    assert_eq!(kalan_mesajlar.len(), 2);
    assert_eq!(kalan_mesajlar.last().unwrap(), "en-yeni-mesaj");
}

#[tokio::test]
async fn durum_hala_cok_buyukse_istek_atilmaz_ve_butce_izni_istenmez() {
    let s = SahteSaglayici::yeni(vec![]);
    let (butce, mut alici) = ButceTalepci::kanal();

    let mut oge1 = kabuk_ogesi("i1", &"k".repeat(15_000));
    oge1.aciklama = "a".repeat(10_000);
    let oge2 = kabuk_ogesi("i2", "npm test");

    let girdi = HakemGirdisi {
        kullanici_istegi: vec!["test".into()],
        calisma_alani: "proje".into(),
        ogeler: vec![oge1, oge2],
    };

    let yanit = degerlendir(s.as_ref(), &butce, &girdi).await;
    assert!(s.karar_istekleri().is_empty());
    assert!(yanit.denemeler.is_empty());
    assert_eq!(yanit.gonderilen_bayt, 0);
    assert!(alici.try_recv().is_err(), "bütçe izni istenmemeli");

    for kimlik in ["i1", "i2"] {
        assert_eq!(
            yanit.sonuclar[kimlik],
            HakemSonucu::Belirsiz {
                olasilik: None,
                neden: "durum çok büyük".into(),
            }
        );
    }
}

#[tokio::test]
async fn gizli_hedefin_diffi_govdede_yoktur_yolu_vardir() {
    let s = SahteSaglayici::yeni(vec![]);
    s.karar_ekle(Ok(yanit_uret(vec![("i1_guvenli", noul_cevap(0.95))], 100)));

    let gizli_isaret = "GIZLI_ANAHTAR_DEGERI_998877";
    let girdi = HakemGirdisi {
        kullanici_istegi: vec!["yapılandırmayı güncelle".into()],
        calisma_alani: "proje".into(),
        ogeler: vec![yaz_ogesi(
            "i1",
            "config/secrets.toml",
            &format!("+api_key = \"{gizli_isaret}\""),
            true,
        )],
    };

    let _ = degerlendir(s.as_ref(), &sinirsiz_butce_talepci(), &girdi).await;
    let istek = &s.karar_istekleri()[0];
    let govde = serde_json::to_string(istek).unwrap();
    assert!(
        !govde.contains(gizli_isaret),
        "gizli hedefin diff içeriği gönderilmemeli: {govde}"
    );
    assert!(govde.contains("config/secrets.toml"));
}

#[tokio::test]
async fn oge_siniri_33_ogede_32_gonderir_ve_bos_girdide_istek_atmaz() {
    let s = SahteSaglayici::yeni(vec![]);

    // Boş girdi
    let bos = HakemGirdisi {
        kullanici_istegi: vec!["merhaba".into()],
        calisma_alani: "proje".into(),
        ogeler: Vec::new(),
    };
    let bos_yanit = degerlendir(s.as_ref(), &sinirsiz_butce_talepci(), &bos).await;
    assert!(bos_yanit.sonuclar.is_empty());
    assert!(s.karar_istekleri().is_empty());

    // 33 öğeli girdi
    let mut ogeler = Vec::new();
    let mut cevaplar = Vec::new();
    for idx in 1..=33 {
        let kimlik = format!("i{idx}");
        ogeler.push(kabuk_ogesi(&kimlik, "cargo test"));
        if idx <= AZAMI_HAKEM_OGESI {
            cevaplar.push((format!("{kimlik}_guvenli"), noul_cevap(0.95)));
            cevaplar.push((format!("{kimlik}_risk"), choice_cevap("yok")));
        }
    }
    s.karar_ekle(Ok(KararYaniti {
        id: Some("k-32".into()),
        model: HAKEM_MODELI.into(),
        answers: cevaplar.into_iter().collect(),
        usage: Some(KararKullanimi {
            input_tokens: 800,
            output_tokens: 0,
        }),
    }));

    let girdi = HakemGirdisi {
        kullanici_istegi: vec!["testleri çalıştır".into()],
        calisma_alani: "proje".into(),
        ogeler,
    };
    let yanit = degerlendir(s.as_ref(), &sinirsiz_butce_talepci(), &girdi).await;
    assert_eq!(yanit.sonuclar.len(), 33);

    let istek = &s.karar_istekleri()[0];
    assert_eq!(istek.state["islemler"].as_array().unwrap().len(), 32);
    assert_eq!(istek.questions.len(), 64);

    for idx in 1..=32 {
        assert_eq!(
            yanit.sonuclar[&format!("i{idx}")],
            HakemSonucu::Izin { olasilik: 0.95 }
        );
    }
    assert_eq!(
        yanit.sonuclar["i33"],
        HakemSonucu::Belirsiz {
            olasilik: None,
            neden: "öğe sınırı".into(),
        }
    );
}

#[tokio::test]
async fn karar_bantlari_tablosu() {
    let senaryolar = [
        ("i1", 0.95, "yok", HakemSonucu::Izin { olasilik: 0.95 }),
        ("i2", 0.80, "yok", HakemSonucu::Izin { olasilik: 0.80 }),
        (
            "i3",
            0.79,
            "yok",
            HakemSonucu::Belirsiz {
                olasilik: Some(0.79),
                neden: "olasılık belirsiz bantta".into(),
            },
        ),
        (
            "i4",
            0.10,
            "veri_sizdirma",
            HakemSonucu::Ret {
                olasilik: 0.10,
                risk: "veri_sizdirma".into(),
            },
        ),
        (
            "i5",
            0.40,
            "yikici",
            HakemSonucu::Ret {
                olasilik: 0.40,
                risk: "yikici".into(),
            },
        ),
        (
            "i6",
            0.10,
            "yok",
            HakemSonucu::Belirsiz {
                olasilik: Some(0.10),
                neden: "risksiz ret olmaz".into(),
            },
        ),
        (
            "i7",
            0.10,
            "listede_olmayan_dizgi",
            HakemSonucu::Belirsiz {
                olasilik: Some(0.10),
                neden: "geçersiz risk sınıfı".into(),
            },
        ),
        // İzin için risk sonucuna bakılmaz
        ("i8", 0.92, "yikici", HakemSonucu::Izin { olasilik: 0.92 }),
    ];

    let s = SahteSaglayici::yeni(vec![]);
    let mut ogeler = Vec::new();
    let mut answers = BTreeMap::new();
    for (kimlik, p, risk, _) in &senaryolar {
        ogeler.push(kabuk_ogesi(kimlik, "npm test"));
        answers.insert(format!("{kimlik}_guvenli"), noul_cevap(*p));
        answers.insert(format!("{kimlik}_risk"), choice_cevap(risk));
    }
    s.karar_ekle(Ok(KararYaniti {
        id: Some("bant-1".into()),
        model: HAKEM_MODELI.into(),
        answers,
        usage: Some(KararKullanimi {
            input_tokens: 400,
            output_tokens: 0,
        }),
    }));

    let girdi = HakemGirdisi {
        kullanici_istegi: vec!["test".into()],
        calisma_alani: "proje".into(),
        ogeler,
    };
    let yanit = degerlendir(s.as_ref(), &sinirsiz_butce_talepci(), &girdi).await;
    for (kimlik, _, _, beklenen) in &senaryolar {
        assert_eq!(&yanit.sonuclar[*kimlik], beklenen, "kimlik: {kimlik}");
    }
}

#[tokio::test]
async fn kapali_dusme_butce_reddi_ag_istegi_atmaz() {
    let s = SahteSaglayici::yeni(vec![]);
    let girdi = HakemGirdisi {
        kullanici_istegi: vec!["kur".into()],
        calisma_alani: "proje".into(),
        ogeler: vec![
            kabuk_ogesi("i1", "npm install"),
            kabuk_ogesi("i2", "npm test"),
        ],
    };

    let yanit = degerlendir(s.as_ref(), &reddeden_butce_talepci(), &girdi).await;
    assert!(s.karar_istekleri().is_empty());
    assert!(yanit.denemeler.is_empty());
    for kimlik in ["i1", "i2"] {
        assert!(
            matches!(
                yanit.sonuclar[kimlik],
                HakemSonucu::Belirsiz { olasilik: None, .. }
            ),
            "{:?}",
            yanit.sonuclar[kimlik]
        );
    }
}

#[tokio::test]
async fn kapali_dusme_evren_hatasi_belirsiz_verir() {
    let s = SahteSaglayici::yeni(vec![]);
    s.karar_ekle(Err(EvrenError::Sunucu {
        durum: 500,
        kod: None,
        mesaj: "iç sunucu hatası".into(),
    }));

    let girdi = HakemGirdisi {
        kullanici_istegi: vec!["kur".into()],
        calisma_alani: "proje".into(),
        ogeler: vec![kabuk_ogesi("i1", "npm install")],
    };

    let yanit = degerlendir(s.as_ref(), &sinirsiz_butce_talepci(), &girdi).await;
    assert_eq!(yanit.denemeler.len(), 1);
    assert!(matches!(
        yanit.sonuclar["i1"],
        HakemSonucu::Belirsiz { olasilik: None, .. }
    ));
}

#[tokio::test(start_paused = true)]
async fn kapali_dusme_zaman_asimi_11_saniyede_belirsiz_verir() {
    let s = SahteSaglayici::yeni(vec![]);
    s.karar_gecikme(Some(Duration::from_secs(11)));
    s.karar_ekle(Ok(yanit_uret(vec![("i1_guvenli", noul_cevap(0.99))], 100)));

    let girdi = HakemGirdisi {
        kullanici_istegi: vec!["kur".into()],
        calisma_alani: "proje".into(),
        ogeler: vec![kabuk_ogesi("i1", "npm install")],
    };

    let yanit = degerlendir(s.as_ref(), &sinirsiz_butce_talepci(), &girdi).await;
    assert_eq!(yanit.denemeler.len(), 1);
    assert!(matches!(yanit.denemeler[0].sonuc, IstekSonucu::Hata { .. }));
    assert_eq!(
        yanit.sonuclar["i1"],
        HakemSonucu::Belirsiz {
            olasilik: None,
            neden: "zaman aşımı (10 sn)".into(),
        }
    );
}

#[tokio::test]
async fn kapali_dusme_eksik_ve_bozuk_cevaplarda_izin_cikmaz() {
    let s = SahteSaglayici::yeni(vec![]);
    let mut answers = BTreeMap::new();
    // i1: geçerli izin (kontrol öğesi: yalnız eksik olan i2 belirsiz olmalı)
    answers.insert("i1_guvenli".into(), noul_cevap(0.95));
    // i2: _guvenli cevabı yanıtta yok
    // i3: noul NaN
    answers.insert("i3_guvenli".into(), noul_cevap(f64::NAN));
    // i4: noul < 0
    answers.insert("i4_guvenli".into(), noul_cevap(-0.01));
    // i5: noul > 1
    answers.insert("i5_guvenli".into(), noul_cevap(1.01));
    // i6: _guvenli sorusuna Choice döndü
    answers.insert("i6_guvenli".into(), choice_cevap("yok"));
    // i7: _guvenli sorusuna Diger döndü
    answers.insert("i7_guvenli".into(), KararCevabi::Diger);

    s.karar_ekle(Ok(KararYaniti {
        id: Some("bozuk-1".into()),
        model: HAKEM_MODELI.into(),
        answers,
        usage: Some(KararKullanimi {
            input_tokens: 300,
            output_tokens: 0,
        }),
    }));

    let girdi = HakemGirdisi {
        kullanici_istegi: vec!["kontrol".into()],
        calisma_alani: "proje".into(),
        ogeler: (1..=7)
            .map(|i| kabuk_ogesi(&format!("i{i}"), "npm test"))
            .collect(),
    };

    let yanit = degerlendir(s.as_ref(), &sinirsiz_butce_talepci(), &girdi).await;
    assert_eq!(yanit.sonuclar["i1"], HakemSonucu::Izin { olasilik: 0.95 });
    for kimlik in ["i2", "i3", "i4", "i5", "i6", "i7"] {
        assert!(
            matches!(
                yanit.sonuclar[kimlik],
                HakemSonucu::Belirsiz { olasilik: None, .. }
            ),
            "{kimlik} Belirsiz olmalıydı: {:?}",
            yanit.sonuclar[kimlik]
        );
    }
}

#[tokio::test(start_paused = true)]
async fn yeniden_deneme_hiz_sinirinda_iki_butce_denemesiyle_izin_doner() {
    let s = SahteSaglayici::yeni(vec![]);
    s.karar_ekle(Err(EvrenError::HizSiniri {
        tekrar_dene: Some(Duration::from_millis(50)),
    }));
    s.karar_ekle(Ok(yanit_uret(
        vec![
            ("i1_guvenli", noul_cevap(0.96)),
            ("i1_risk", choice_cevap("yok")),
        ],
        450,
    )));

    let (butce, mut alici) = ButceTalepci::kanal();
    let izin_gorevi = tokio::spawn(async move {
        let mut verilen = Vec::new();
        while let Some(mesaj) = alici.recv().await {
            if let ButceMesaji::Talep(talep) = mesaj {
                assert_eq!(talep.amac, HAKEM_BUTCE_AMACI);
                let id = ButceIslemId(verilen.len() as u64 + 1);
                verilen.push(id);
                let _ = talep.yanit.send(Some(id));
            }
        }
        verilen
    });

    let girdi = HakemGirdisi {
        kullanici_istegi: vec!["kur".into()],
        calisma_alani: "proje".into(),
        ogeler: vec![kabuk_ogesi("i1", "npm install")],
    };

    let yanit = degerlendir(s.as_ref(), &butce, &girdi).await;
    drop(butce);
    let verilen_izinler = izin_gorevi.await.unwrap();

    assert_eq!(verilen_izinler, vec![ButceIslemId(1), ButceIslemId(2)]);
    assert_eq!(yanit.denemeler.len(), 2);
    assert_eq!(yanit.sonuclar["i1"], HakemSonucu::Izin { olasilik: 0.96 });
}

#[tokio::test]
async fn kullanim_ve_gonderilen_bayt_kaydedilir() {
    let s = SahteSaglayici::yeni(vec![]);
    s.karar_ekle(Ok(yanit_uret(
        vec![
            ("i1_guvenli", noul_cevap(0.95)),
            ("i1_risk", choice_cevap("yok")),
        ],
        632,
    )));

    let girdi = HakemGirdisi {
        kullanici_istegi: vec!["bağımlılıkları kur".into()],
        calisma_alani: "proje".into(),
        ogeler: vec![kabuk_ogesi("i1", "npm install")],
    };

    let yanit = degerlendir(s.as_ref(), &sinirsiz_butce_talepci(), &girdi).await;
    assert_eq!(yanit.denemeler.len(), 1);
    assert_eq!(
        yanit.denemeler[0].kullanim,
        Some(GunlukToken {
            istem: 632,
            tamamlama: 0,
            dusunce: 0,
        })
    );
    let istek = &s.karar_istekleri()[0];
    let beklenen_bayt = serde_json::to_vec(&istek.state).unwrap().len() as u64;
    assert_eq!(yanit.gonderilen_bayt, beklenen_bayt);
}

#[tokio::test]
async fn gecersiz_veya_cakisan_kimlikte_istek_atilmaz_hepsi_belirsiz_olur() {
    let s = SahteSaglayici::yeni(vec![]);

    // 1) Yinelenen kimlik
    let yinelenen = HakemGirdisi {
        kullanici_istegi: vec!["test".into()],
        calisma_alani: "proje".into(),
        ogeler: vec![kabuk_ogesi("i1", "npm test"), kabuk_ogesi("i1", "npm run")],
    };
    let y1 = degerlendir(s.as_ref(), &sinirsiz_butce_talepci(), &yinelenen).await;
    assert!(s.karar_istekleri().is_empty());
    assert!(matches!(
        y1.sonuclar["i1"],
        HakemSonucu::Belirsiz { olasilik: None, .. }
    ));

    // 2) Soru kimliği deseniyle çakışan kimlik ("i1_guvenli")
    let cakisan = HakemGirdisi {
        kullanici_istegi: vec!["test".into()],
        calisma_alani: "proje".into(),
        ogeler: vec![
            kabuk_ogesi("i1", "npm test"),
            kabuk_ogesi("i1_guvenli", "npm run"),
        ],
    };
    let y2 = degerlendir(s.as_ref(), &sinirsiz_butce_talepci(), &cakisan).await;
    assert!(s.karar_istekleri().is_empty());
    for kimlik in ["i1", "i1_guvenli"] {
        assert!(matches!(
            y2.sonuclar[kimlik],
            HakemSonucu::Belirsiz { olasilik: None, .. }
        ));
    }
}
