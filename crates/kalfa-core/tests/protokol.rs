use kalfa_core::izin::Kip;
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
            }],
        },
        Olay::IstekBilgisi {
            istek_id: Some("r".into()),
            yonlendirilen_model: Some("glm-5.3".into()),
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
