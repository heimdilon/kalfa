use kalfa_core::gonderim::{
    GonderilenOge, GunlukSatiri, GunlukToken, IstekAsamasi, IstekSonucu, gunluk_sayfasi,
    gunluk_uret, oge_metni,
};
use kalfa_core::izin::Kip;
use kalfa_core::kayit::KayitOgesi;
use kalfa_core::kimlik::{OturumId, TurId};
use kalfa_core::protokol::{SEMA_SURUMU, Zarf};
use kalfa_evren::KrediGozlemi;
use kalfa_evren::tipler::{ChatMessage, ToolDef};

fn zarf(sira: u64, yuk: KayitOgesi) -> Zarf<KayitOgesi> {
    Zarf {
        sema: SEMA_SURUMU,
        oturum: OturumId::yeni(),
        tur: Some(TurId::yeni()),
        sira,
        yuk,
    }
}

#[test]
fn satir_istek_ve_kullanimi_birlestirir() {
    let istek_kredi = KrediGozlemi {
        kalan: None,
        tutulan: None,
        istek_id: Some("req-1".into()),
        zaman_ms: 1_700_000_000_123,
    };
    let kayitlar = vec![
        zarf(
            1,
            KayitOgesi::OturumAcildi {
                model: "glm-5.3".into(),
                kip: Kip::Siki,
                calisma_alani: None,
            },
        ),
        zarf(
            2,
            KayitOgesi::Istek {
                amac: "ana".into(),
                istek_id: Some("req-1".into()),
                kredi: Some(istek_kredi),
                yonlendirilen_model: Some("glm-5.3".into()),
                model: Some("glm-5.3".into()),
                yeni: vec![GonderilenOge {
                    kaynak: "mesaj:kullanıcı".into(),
                    sira: Some(1),
                    bayt: 10,
                }],
                onceki: 0,
                sonuc: IstekSonucu::Basarili,
            },
        ),
        zarf(
            3,
            KayitOgesi::Kullanim {
                istem: 120,
                tamamlama: 30,
                dusunce: 15,
            },
        ),
    ];

    let satirlar = gunluk_uret(&kayitlar);
    assert_eq!(satirlar.len(), 1);
    let s = &satirlar[0];
    assert_eq!(s.sira, 2);
    assert_eq!(s.zaman, Some(1_700_000_000_123));
    assert_eq!(s.amac, "ana");
    assert_eq!(s.model.as_deref(), Some("glm-5.3"));
    assert_eq!(s.yonlendirilen_model.as_deref(), Some("glm-5.3"));
    assert_eq!(s.istek_id.as_deref(), Some("req-1"));
    assert_eq!(
        s.token,
        Some(GunlukToken {
            istem: 120,
            tamamlama: 30,
            dusunce: 15,
        })
    );
    assert_eq!(s.sonuc, IstekSonucu::Basarili);
    assert_eq!(s.yeni.len(), 1);
    assert_eq!(s.onceki, 0);
}

#[test]
fn kopan_istek_satiri_hata_gosterir() {
    let kayitlar = vec![
        zarf(
            1,
            KayitOgesi::Istek {
                amac: "ana".into(),
                istek_id: Some("req-1".into()),
                kredi: None,
                yonlendirilen_model: Some("glm-5.3".into()),
                model: Some("glm-5.3".into()),
                yeni: vec![],
                onceki: 0,
                sonuc: IstekSonucu::Basarili,
            },
        ),
        zarf(
            2,
            KayitOgesi::IstekKoptu {
                mesaj: "bağlantı koptu: zaman aşımı".into(),
            },
        ),
    ];

    let satirlar = gunluk_uret(&kayitlar);
    assert_eq!(satirlar.len(), 1);
    assert_eq!(
        satirlar[0].sonuc,
        IstekSonucu::Hata {
            asama: IstekAsamasi::Yanit,
            mesaj: "bağlantı koptu: zaman aşımı".into(),
        }
    );
}

#[test]
fn sayfalama_gecmis_gibi() {
    let satirlar: Vec<GunlukSatiri> = (1..=5)
        .map(|i| GunlukSatiri {
            sira: i * 10,
            tur: None,
            zaman: None,
            amac: "ana".into(),
            model: None,
            yonlendirilen_model: None,
            istek_id: None,
            token: None,
            sonuc: IstekSonucu::Basarili,
            yeni: vec![],
            onceki: 0,
        })
        .collect();

    // 1. sayfa: son 2 öğe (sira 40, 50)
    let sayfa1 = gunluk_sayfasi(&satirlar, None, 2);
    assert_eq!(sayfa1.satirlar.len(), 2);
    assert_eq!(sayfa1.satirlar[0].sira, 40);
    assert_eq!(sayfa1.satirlar[1].sira, 50);
    assert!(sayfa1.daha_eski_var);

    // 2. sayfa: sira 40'tan önceki 2 öğe (sira 20, 30)
    let sayfa2 = gunluk_sayfasi(&satirlar, Some(40), 2);
    assert_eq!(sayfa2.satirlar.len(), 2);
    assert_eq!(sayfa2.satirlar[0].sira, 20);
    assert_eq!(sayfa2.satirlar[1].sira, 30);
    assert!(sayfa2.daha_eski_var);

    // 3. sayfa: sira 20'den önceki 2 öğe (yalnızca sira 10 var)
    let sayfa3 = gunluk_sayfasi(&satirlar, Some(20), 2);
    assert_eq!(sayfa3.satirlar.len(), 1);
    assert_eq!(sayfa3.satirlar[0].sira, 10);
    assert!(!sayfa3.daha_eski_var);
}

#[test]
fn oge_metni_kayittan() {
    let kayitlar = vec![
        zarf(
            2,
            KayitOgesi::SistemIstemi {
                metin: "Sen Kalfa'sın".into(),
            },
        ),
        zarf(
            3,
            KayitOgesi::AracTanimlari {
                tanimlar: vec![ToolDef::function("oku", "dosya oku", serde_json::json!({}))],
            },
        ),
        zarf(
            4,
            KayitOgesi::Mesaj {
                mesaj: ChatMessage::user("merhaba"),
                kaynak: Some("mesaj:kullanıcı".into()),
            },
        ),
        zarf(
            5,
            KayitOgesi::KalfaNotu {
                metin: "Dosyalar geri alındı".into(),
            },
        ),
        zarf(
            6,
            KayitOgesi::Istek {
                amac: "ana".into(),
                istek_id: None,
                kredi: None,
                yonlendirilen_model: None,
                model: None,
                yeni: vec![],
                onceki: 0,
                sonuc: IstekSonucu::Basarili,
            },
        ),
    ];

    assert_eq!(oge_metni(&kayitlar, 2).as_deref(), Some("Sen Kalfa'sın"));
    let arac_metni = oge_metni(&kayitlar, 3).expect("araç tanımları metni olmalı");
    assert!(arac_metni.contains("oku"));
    assert_eq!(oge_metni(&kayitlar, 4).as_deref(), Some("merhaba"));
    assert_eq!(
        oge_metni(&kayitlar, 5).as_deref(),
        Some("Dosyalar geri alındı")
    );
    assert_eq!(oge_metni(&kayitlar, 6), None, "İstek metin öğesi değildir");
    assert_eq!(oge_metni(&kayitlar, 999), None, "Olmayan sıra None döner");
}

#[test]
fn eski_kayittan_gunluk_uretilir() {
    let satir = r#"{"sema":1,"oturum":"6f1d0a4e-2b8c-4c1e-9a55-0d6b1f3e2a10","tur":null,"sira":2,"yuk":{"tip":"Istek","amac":"ana","istek_id":"r-1","kredi":null,"yonlendirilen_model":"glm-5.3"}}"#;
    let zarf: Zarf<KayitOgesi> = serde_json::from_str(satir).unwrap();

    let satirlar = gunluk_uret(&[zarf]);
    assert_eq!(satirlar.len(), 1);
    let s = &satirlar[0];
    assert_eq!(s.sira, 2);
    assert_eq!(s.amac, "ana");
    assert_eq!(s.istek_id.as_deref(), Some("r-1"));
    assert_eq!(s.yonlendirilen_model.as_deref(), Some("glm-5.3"));
    assert_eq!(s.model, None);
    assert!(s.yeni.is_empty());
    assert_eq!(s.onceki, 0);
    assert_eq!(s.sonuc, IstekSonucu::Basarili);
    assert_eq!(s.token, None);
}
