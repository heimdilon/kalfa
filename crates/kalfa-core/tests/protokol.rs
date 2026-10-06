use kalfa_core::izin::Kip;
use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
use kalfa_core::kimlik::{CagriId, IslemId, OturumId, TurId};
use kalfa_core::muhur::OnayMuhru;
use kalfa_core::protokol::*;
use kalfa_evren::KrediGozlemi;
use serde_json::json;

fn gidip_gelir<T>(deger: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let j = serde_json::to_string(deger).unwrap();
    assert_eq!(&serde_json::from_str::<T>(&j).unwrap(), deger, "{j}");
}

#[test]
fn komut_json_bicimi() {
    assert_eq!(
        serde_json::to_value(Komut::KullaniciMesaji {
            metin: "selam".into()
        })
        .unwrap(),
        json!({"tip": "KullaniciMesaji", "metin": "selam"})
    );
    let c = CagriId::yeni();
    let onay = Komut::AracOnayi {
        cagri: c,
        karar: OnayKarari::Reddet {
            gerekce: Some("gerek yok".into()),
        },
        muhur: OnayMuhru("ab".into()),
    };
    let j = serde_json::to_value(&onay).unwrap();
    assert_eq!(j["karar"], json!({"tip": "Reddet", "gerekce": "gerek yok"}));
    assert_eq!(j["cagri"], json!(c.to_string()));
    assert_eq!(j["muhur"], json!("ab"));
}

#[test]
fn butun_komutlar_gidip_gelir() {
    for k in [
        Komut::KullaniciMesaji { metin: "a".into() },
        Komut::Iptal { tur: TurId::yeni() },
        Komut::AracOnayi {
            cagri: CagriId::yeni(),
            karar: OnayKarari::Onayla,
            muhur: OnayMuhru("m".into()),
        },
        Komut::ModelDegistir {
            model: "glm-5.3".into(),
        },
        Komut::IzinKipiDegistir { kip: Kip::Proje },
        Komut::YerelOnay {
            cagri: CagriId::yeni(),
            muhur: OnayMuhru("m".into()),
        },
    ] {
        gidip_gelir(&k);
    }
}

#[test]
fn butun_olaylar_gidip_gelir() {
    let c = CagriId::yeni();
    assert_eq!(
        serde_json::to_value(Olay::TurBasladi).unwrap(),
        json!({"tip": "TurBasladi"})
    );
    for o in [
        Olay::TurBasladi,
        Olay::TokenParcasi {
            metin: "Mer".into(),
        },
        Olay::DusunceParcasi {
            metin: "düşün".into(),
        },
        Olay::AracCagrisiBasladi {
            cagri: c,
            arac: "dosya_oku".into(),
            aciklama: "okuyor".into(),
        },
        Olay::AracSonucu {
            cagri: c,
            arac: "dosya_oku".into(),
            durum: AracDurumu::SonucuBilinmiyor,
            icerik: "?".into(),
        },
        Olay::OnayGerekiyor {
            islem: IslemId::yeni(),
            ogeler: vec![OnayOgesi {
                cagri: c,
                arac: "dosya_yaz".into(),
                aciklama: "yazacak".into(),
                hedefler: vec!["a.txt".into()],
                diff: Some("+x".into()),
                uyarilar: vec!["dikkat".into()],
                muhur: OnayMuhru("m".into()),
                kabuk: None,
                yerel_onay: false,
                onaysiz_degisiklikler: None,
                geri_alinabilir: None,
                veri_cikisi: None,
            }],
        },
        Olay::OnayGerekiyor {
            islem: IslemId::yeni(),
            ogeler: vec![kabuk_ogesi(c)],
        },
        Olay::AracCiktisiParcasi {
            cagri: c,
            akis: CiktiAkisi::Hata,
            metin: "uyarı: ş\n".into(),
            atlanan_bayt: 12,
        },
        Olay::IstekBilgisi {
            istek_id: Some("r".into()),
            yonlendirilen_model: Some("glm-5.3".into()),
            amac: "ana".into(),
            basarili: true,
        },
        Olay::KrediGozlemi {
            gozlem: KrediGozlemi {
                kalan: None,
                tutulan: None,
                istek_id: Some("r".into()),
                zaman_ms: 1,
            },
        },
        Olay::TokenKullanimi {
            istem: 10,
            tamamlama: 5,
            dusunce: 2,
        },
        Olay::SartOnayiGerekiyor {
            surum: 1,
            metin: "# Şartlar".into(),
        },
        Olay::Hata {
            mesaj: "olmadı".into(),
        },
        Olay::Tamamlandi {
            neden: BitisNedeni::Sinir {
                aciklama: "çok".into(),
            },
        },
        Olay::Tamamlandi {
            neden: BitisNedeni::Normal,
        },
        Olay::AktarimDurdu {
            neden: AktarimDurmaNedeni::OlayCokBuyuk,
        },
    ] {
        gidip_gelir(&o);
    }
}

#[test]
fn aktarim_durdu_json_bicimi() {
    assert_eq!(
        serde_json::to_value(Olay::AktarimDurdu {
            neden: AktarimDurmaNedeni::TuketiciYetismedi
        })
        .unwrap(),
        json!({"tip": "AktarimDurdu", "neden": "TuketiciYetismedi"})
    );
}

fn kabuk_ogesi(cagri: CagriId) -> OnayOgesi {
    OnayOgesi {
        cagri,
        arac: "kabuk".into(),
        aciklama: "`cargo test` komutunu çalıştırmak istiyor.".into(),
        hedefler: vec![],
        diff: None,
        uyarilar: vec!["Yalıtımsız".into()],
        muhur: OnayMuhru("m".into()),
        kabuk: Some(KabukTarifi {
            komut: "cargo test".into(),
            bicim: KabukBicimi::Program,
            dizin: ".".into(),
            ag: false,
            ag_hostlari: Vec::new(),
            yalitim: YalitimOzeti::Yalitimsiz {
                talimat: "yok".into(),
            },
            ilk_cagri: true,
            alan_hazirligi: false,
        }),
        yerel_onay: true,
        onaysiz_degisiklikler: None,
        geri_alinabilir: None,
        veri_cikisi: None,
    }
}

/// Arayüzün `tipler.ts`'teki tanımları bu biçime dayanır.
#[test]
fn kabuk_json_bicimi() {
    let c = CagriId::yeni();
    let j = serde_json::to_value(kabuk_ogesi(c)).unwrap();
    assert_eq!(j["yerel_onay"], json!(true));
    assert_eq!(
        j["kabuk"],
        json!({
            "komut": "cargo test",
            "bicim": {"tip": "Program"},
            "dizin": ".",
            "ag": false,
            "ag_hostlari": [],
            "yalitim": {"tip": "Yalitimsiz", "talimat": "yok"},
            "ilk_cagri": true,
            "alan_hazirligi": false
        })
    );
    let parca = serde_json::to_value(Olay::AracCiktisiParcasi {
        cagri: c,
        akis: CiktiAkisi::Standart,
        metin: "a".into(),
        atlanan_bayt: 0,
    })
    .unwrap();
    assert_eq!(
        parca,
        json!({"tip": "AracCiktisiParcasi", "cagri": c.to_string(), "akis": "Standart", "metin": "a", "atlanan_bayt": 0})
    );
    // Kabuk alanları olmayan eski biçim de okunur.
    let mut eski = serde_json::to_value(kabuk_ogesi(c)).unwrap();
    eski.as_object_mut().unwrap().remove("kabuk");
    eski.as_object_mut().unwrap().remove("yerel_onay");
    let oge: OnayOgesi = serde_json::from_value(eski).unwrap();
    assert_eq!(oge.kabuk, None);
    assert!(!oge.yerel_onay);
}

#[test]
fn zarf_bicimi() {
    let z = Zarf {
        sema: SEMA_SURUMU,
        oturum: OturumId::yeni(),
        tur: Some(TurId::yeni()),
        sira: 7,
        yuk: Olay::TurBasladi,
    };
    let j = serde_json::to_value(&z).unwrap();
    assert_eq!(j["sema"], 1);
    assert_eq!(j["sira"], 7);
    assert_eq!(j["yuk"], json!({"tip": "TurBasladi"}));
    gidip_gelir(&z);
}

#[test]
fn eski_onay_ogesi_veri_cikissiz_acilir() {
    let json = serde_json::json!({
        "cagri": CagriId::yeni().to_string(),
        "arac": "dosya_oku",
        "aciklama": "oku",
        "hedefler": ["a.txt"],
        "diff": null,
        "uyarilar": [],
        "muhur": "m",
        "kabuk": null,
        "yerel_onay": false
    });
    let oge: OnayOgesi = serde_json::from_value(json).unwrap();
    assert_eq!(oge.veri_cikisi, None);
}
