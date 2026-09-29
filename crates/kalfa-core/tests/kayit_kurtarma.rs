use std::io::Write;

use kalfa_core::izin::Kip;
use kalfa_core::kayit::{EskiSartKabulKomutu, KayitKomutu, KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::{CagriId, OturumId};
use kalfa_core::kurtarma::*;
use kalfa_core::protokol::{AracDurumu, Komut, SEMA_SURUMU, Zarf};
use kalfa_evren::tipler::{ChatMessage, FunctionCall, Role, ToolCall};

fn z(oturum: OturumId, sira: u64, yuk: KayitOgesi) -> Zarf<KayitOgesi> {
    Zarf {
        sema: SEMA_SURUMU,
        oturum,
        tur: None,
        sira,
        yuk,
    }
}

fn cagri(id: &str, ad: &str) -> ToolCall {
    ToolCall {
        id: id.into(),
        kind: "function".into(),
        function: FunctionCall {
            name: ad.into(),
            arguments: "{}".into(),
        },
    }
}

fn acilis(kip: Kip) -> KayitOgesi {
    KayitOgesi::OturumAcildi {
        model: "glm-5.3".into(),
        kip,
        calisma_alani: None,
    }
}

#[test]
fn kayit_yazilir_okunur_ve_ikinci_olusturma_reddedilir() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("alt/oturum.jsonl");
    let o = OturumId::yeni();
    let mut k = OturumKaydi::olustur(&yol).unwrap();
    k.yaz(&z(o, 1, acilis(Kip::Siki))).unwrap();
    k.yaz_kalici(&z(
        o,
        2,
        KayitOgesi::Komut {
            komut: Komut::KullaniciMesaji {
                metin: "selam".into(),
            }
            .into(),
        },
    ))
    .unwrap();
    drop(k);
    let okunan = OturumKaydi::oku(&yol).unwrap();
    assert_eq!(okunan.len(), 2);
    assert_eq!(
        okunan[1].yuk,
        KayitOgesi::Komut {
            komut: Komut::KullaniciMesaji {
                metin: "selam".into()
            }
            .into()
        }
    );
    assert!(
        OturumKaydi::olustur(&yol).is_err(),
        "var olan kaydın üzerine yazılmaz"
    );

    let mut k = OturumKaydi::ac(&yol).unwrap();
    k.yaz(&z(o, 3, KayitOgesi::KipDegisti { kip: Kip::Proje }))
        .unwrap();
    assert_eq!(OturumKaydi::oku(&yol).unwrap().len(), 3);
}

fn eski_sart_satiri(oturum: OturumId, sira: u64) -> String {
    format!(
        r#"{{"sema":{},"oturum":"{}","tur":null,"sira":{},"yuk":{{"tip":"Komut","komut":{{"tip":"SartlariKabulEt","surum":1}}}}}}"#,
        SEMA_SURUMU, oturum, sira
    )
}

#[test]
fn eski_sart_komutu_kayit_turunde_kabul_edilir_canli_komutta_edilmez() {
    let eski = r#"{"tip":"SartlariKabulEt","surum":1}"#;
    assert!(serde_json::from_str::<Komut>(eski).is_err());
    let kayit_komutu = serde_json::from_str::<KayitKomutu>(eski).unwrap();
    assert_eq!(
        kayit_komutu,
        KayitKomutu::EskiSartKabulIstegi(EskiSartKabulKomutu::SartKabulIstegi { surum: 1 })
    );
    assert!(serde_json::to_string(&kayit_komutu).is_err());
}

#[test]
fn ortadaki_eski_sart_komutu_ve_sonraki_kayitlar_okunur() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("oturum.jsonl");
    let o = OturumId::yeni();
    let mut k = OturumKaydi::olustur(&yol).unwrap();
    k.yaz(&z(o, 1, acilis(Kip::Siki))).unwrap();
    drop(k);

    let mut dosya = std::fs::OpenOptions::new().append(true).open(&yol).unwrap();
    writeln!(dosya, "{}", eski_sart_satiri(o, 2)).unwrap();
    writeln!(
        dosya,
        "{}",
        serde_json::to_string(&z(
            o,
            3,
            KayitOgesi::ModelDegisti {
                model: "deepseek-v4-flash".into(),
            },
        ))
        .unwrap()
    )
    .unwrap();
    drop(dosya);

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    assert_eq!(kayitlar.len(), 3);
    assert!(matches!(
        &kayitlar[1].yuk,
        KayitOgesi::Komut {
            komut: KayitKomutu::EskiSartKabulIstegi(EskiSartKabulKomutu::SartKabulIstegi {
                surum: 1
            })
        }
    ));
    assert_eq!(
        kayitlar[2].yuk,
        KayitOgesi::ModelDegisti {
            model: "deepseek-v4-flash".into(),
        }
    );
    assert!(
        !kayitlar
            .iter()
            .any(|z| matches!(z.yuk, KayitOgesi::SartKabulEdildi { .. }))
    );

    let durum = kurtar(&kayitlar).unwrap();
    assert_eq!(durum.model, "deepseek-v4-flash");
    assert_eq!(durum.son_sira, 3);
}

#[test]
fn eof_eski_sart_komutu_acip_yazarken_korunur() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("oturum.jsonl");
    let o = OturumId::yeni();
    let mut k = OturumKaydi::olustur(&yol).unwrap();
    k.yaz(&z(o, 1, acilis(Kip::Siki))).unwrap();
    drop(k);

    let mut dosya = std::fs::OpenOptions::new().append(true).open(&yol).unwrap();
    writeln!(dosya, "{}", eski_sart_satiri(o, 2)).unwrap();
    drop(dosya);
    let eski_baytlar = std::fs::read(&yol).unwrap();

    let mut k = OturumKaydi::ac(&yol).unwrap();
    k.yaz(&z(
        o,
        3,
        KayitOgesi::ModelDegisti {
            model: "deepseek-v4-flash".into(),
        },
    ))
    .unwrap();
    drop(k);

    let yeni_baytlar = std::fs::read(&yol).unwrap();
    assert!(yeni_baytlar.starts_with(&eski_baytlar));
    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    assert_eq!(kayitlar.len(), 3);
    assert!(matches!(
        &kayitlar[1].yuk,
        KayitOgesi::Komut {
            komut: KayitKomutu::EskiSartKabulIstegi(EskiSartKabulKomutu::SartKabulIstegi {
                surum: 1
            })
        }
    ));
    assert!(
        !kayitlar
            .iter()
            .any(|z| matches!(z.yuk, KayitOgesi::SartKabulEdildi { .. }))
    );
    let durum = kurtar(&kayitlar).unwrap();
    assert_eq!(durum.model, "deepseek-v4-flash");
    assert_eq!(durum.son_sira, 3);
}

#[test]
fn yarim_son_satir_atlanir_ortadaki_bozuk_satir_hatadir() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("o.jsonl");
    let o = OturumId::yeni();
    let mut k = OturumKaydi::olustur(&yol).unwrap();
    k.yaz(&z(o, 1, acilis(Kip::Siki))).unwrap();
    k.yaz(&z(o, 2, KayitOgesi::KipDegisti { kip: Kip::Proje }))
        .unwrap();
    drop(k);
    let mut ham = std::fs::OpenOptions::new().append(true).open(&yol).unwrap();
    ham.write_all(b"{\"sema\":1,\"otu").unwrap();
    drop(ham);
    assert_eq!(
        OturumKaydi::oku(&yol).unwrap().len(),
        2,
        "çökmede yarım kalan son satır atlanır"
    );

    let bozuk = d.path().join("bozuk.jsonl");
    let gecerli = serde_json::to_string(&z(o, 1, acilis(Kip::Siki))).unwrap();
    std::fs::write(&bozuk, format!("{gecerli}\nbozuk satır\n{gecerli}\n")).unwrap();
    let hata = OturumKaydi::oku(&bozuk).unwrap_err();
    assert_eq!(hata.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn cokmus_kayit_onarilir_ve_yeniden_acilabilir() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("o.jsonl");
    let o = OturumId::yeni();
    let mut k = OturumKaydi::olustur(&yol).unwrap();
    k.yaz(&z(o, 1, acilis(Kip::Siki))).unwrap();
    drop(k);

    // Çökme: son satır yarım kaldı. Kurtar → yaz → tekrar aç.
    let mut ham = std::fs::OpenOptions::new().append(true).open(&yol).unwrap();
    ham.write_all(b"{\"sema\":1,\"otu").unwrap();
    drop(ham);
    let mut k = OturumKaydi::ac(&yol).unwrap();
    k.yaz(&z(o, 2, KayitOgesi::KipDegisti { kip: Kip::Proje }))
        .unwrap();
    drop(k);
    let okunan = OturumKaydi::oku(&yol).unwrap();
    assert_eq!(
        okunan.len(),
        2,
        "yarım satır kesildi; yeni kayıt ona yapışmadı"
    );
    assert_eq!(okunan[1].yuk, KayitOgesi::KipDegisti { kip: Kip::Proje });

    // Tam yazılmış ama satır sonu eksik son satır da onarılır.
    let tam = serde_json::to_vec(&z(o, 3, KayitOgesi::KipDegisti { kip: Kip::Siki })).unwrap();
    let mut ham = std::fs::OpenOptions::new().append(true).open(&yol).unwrap();
    ham.write_all(&tam).unwrap();
    drop(ham);
    let mut k = OturumKaydi::ac(&yol).unwrap();
    k.yaz(&z(o, 4, KayitOgesi::KipDegisti { kip: Kip::Proje }))
        .unwrap();
    drop(k);
    assert_eq!(OturumKaydi::oku(&yol).unwrap().len(), 4);
}

#[test]
fn kurtarma_kip_model_ve_sira() {
    let o = OturumId::yeni();
    let kayitlar = vec![
        z(o, 1, acilis(Kip::Serbest)),
        z(
            o,
            2,
            KayitOgesi::ModelDegisti {
                model: "deepseek-v4-flash".into(),
            },
        ),
        z(o, 3, KayitOgesi::KipDegisti { kip: Kip::Proje }),
        z(o, 4, KayitOgesi::KipDegisti { kip: Kip::Serbest }),
        z(
            o,
            9,
            KayitOgesi::Mesaj {
                mesaj: ChatMessage::user("a"),
            },
        ),
    ];
    let d = kurtar(&kayitlar).unwrap();
    assert_eq!(d.oturum, o);
    assert_eq!(d.model, "deepseek-v4-flash");
    assert_eq!(d.kip, Kip::Siki, "Serbest kip kurtarmada geri gelmez");
    assert_eq!(d.son_sira, 9);
    assert_eq!(d.mesajlar, vec![ChatMessage::user("a")]);
    assert!(d.tamamlayicilar.is_empty() && d.bilinmeyenler.is_empty());

    let d = kurtar(&[z(o, 1, acilis(Kip::Proje))]).unwrap();
    assert_eq!(d.kip, Kip::Proje, "Proje kipi korunur");
}

#[test]
fn eksik_arac_yanitlari_tamamlanir() {
    let o = OturumId::yeni();
    let (c1, c3) = (CagriId::yeni(), CagriId::yeni());
    let kayitlar = vec![
        z(o, 1, acilis(Kip::Siki)),
        z(
            o,
            2,
            KayitOgesi::Mesaj {
                mesaj: ChatMessage::user("üçünü yap"),
            },
        ),
        z(
            o,
            3,
            KayitOgesi::Mesaj {
                mesaj: ChatMessage::assistant(
                    None,
                    vec![cagri("m1", "yaz"), cagri("m2", "oku"), cagri("m3", "oku")],
                ),
            },
        ),
        z(
            o,
            4,
            KayitOgesi::AracBasladi {
                cagri: c1,
                model_cagri_id: "m1".into(),
                arac: "yaz".into(),
                yedekler: Vec::new(),
            },
        ),
        z(
            o,
            5,
            KayitOgesi::AracBasladi {
                cagri: c3,
                model_cagri_id: "m3".into(),
                arac: "oku".into(),
                yedekler: Vec::new(),
            },
        ),
        z(
            o,
            6,
            KayitOgesi::Mesaj {
                mesaj: ChatMessage::tool("m3", "içerik"),
            },
        ),
        z(
            o,
            7,
            KayitOgesi::AracBitti {
                cagri: c3,
                durum: AracDurumu::Basarili,
            },
        ),
    ];
    let d = kurtar(&kayitlar).unwrap();
    assert_eq!(d.mesajlar.len(), 3);
    assert_eq!(
        d.tamamlayicilar,
        vec![
            ChatMessage::tool("m1", BILINMEYEN_SONUC),
            ChatMessage::tool("m2", CALISTIRILMADI)
        ]
    );
    assert_eq!(
        d.bilinmeyenler,
        vec![BilinmeyenCagri {
            cagri: c1,
            arac: "yaz".into()
        }]
    );
    assert!(d.tamamlayicilar.iter().all(|m| m.role == Role::Tool));
}

#[test]
fn bos_ve_acilissiz_kayit() {
    assert_eq!(kurtar(&[]), Err(KurtarmaHatasi::Bos));
    let o = OturumId::yeni();
    assert_eq!(
        kurtar(&[z(o, 1, KayitOgesi::KipDegisti { kip: Kip::Siki })]),
        Err(KurtarmaHatasi::AcilisYok)
    );
}

/// Aşama 3 tasarımı §5.10: kayıt, kilit altında silinir; başka yerde açık
/// kayıt silinmez; `once` başarısızsa kayıt kalır.
#[test]
fn kayit_kilit_altinda_silinir() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("oturum.jsonl");
    let kilit = kalfa_core::kayit::kilit_yolu(&yol);
    let acik = OturumKaydi::olustur(&yol).unwrap();
    let hata = OturumKaydi::sil(&yol, || panic!("kilitsiz çalışmamalı")).unwrap_err();
    assert_eq!(hata.kind(), std::io::ErrorKind::ResourceBusy);
    assert!(yol.exists());
    drop(acik);

    let hata =
        OturumKaydi::sil(&yol, || Err(std::io::Error::other("veri silinemedi"))).unwrap_err();
    assert_eq!(hata.to_string(), "veri silinemedi");
    assert!(yol.exists(), "önceki adım başarısızsa kayıt kalır");

    let mut calisti = false;
    OturumKaydi::sil(&yol, || {
        calisti = true;
        Ok(())
    })
    .unwrap();
    assert!(calisti);
    assert!(!yol.exists());
    assert!(!kilit.exists(), "kilit dosyası da silinir");
    let hata = OturumKaydi::sil(&yol, || Ok(())).unwrap_err();
    assert_eq!(hata.kind(), std::io::ErrorKind::NotFound);
}
