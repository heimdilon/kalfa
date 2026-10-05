use std::path::{Path, PathBuf};
use std::time::Duration;

use kalfa_core::gonderim::{IstekAsamasi, IstekSonucu};
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kurtarma::kurtar;
use kalfa_core::oturum::OturumAyarlari;
use kalfa_core::protokol::{BitisNedeni, Komut, Zarf};
use kalfa_core::test_destek::*;
use kalfa_evren::EvrenError;
use serde_json::json;

fn kayitli(dizin: &Path) -> (OturumAyarlari, PathBuf) {
    let yol = dizin.join("oturum.jsonl");
    let mut ayar = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    ayar.kayit_yolu = Some(yol.clone());
    (ayar, yol)
}

fn selam() -> Komut {
    Komut::KullaniciMesaji {
        metin: "selam".into(),
    }
}

#[tokio::test]
async fn her_deneme_kaydedilir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::Hata(EvrenError::HizSiniri {
            tekrar_dene: Some(Duration::from_millis(10)),
        }),
        SahteYanit::metin("merhaba"),
    ]);
    let mut surucu = test_oturumu(test_baglami(s, vec![], sanal_siniflandirici()), ayar);
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    let istekler: Vec<_> = kayitlar
        .iter()
        .filter_map(|z| match &z.yuk {
            KayitOgesi::Istek {
                sonuc,
                onceki,
                yeni,
                ..
            } => Some((sonuc.clone(), *onceki, yeni.clone())),
            _ => None,
        })
        .collect();

    assert_eq!(istekler.len(), 2);
    // İlk deneme 429 ile başarısız (gövde gitti: Yanıt aşaması)
    assert!(matches!(
        istekler[0].0,
        IstekSonucu::Hata {
            asama: IstekAsamasi::Yanit,
            ..
        }
    ));
    assert_eq!(istekler[0].1, 0);
    // İlk denemede sistem istemi + araç tanımları + kullanıcı mesajı
    assert_eq!(istekler[0].2.len(), 3);

    // İkinci deneme başarılı; yeni boş, önceki 1
    assert_eq!(istekler[1].0, IstekSonucu::Basarili);
    assert_eq!(istekler[1].1, 1);
    assert!(istekler[1].2.is_empty());
}

#[tokio::test]
async fn baglanti_hatasi_govde_gitmedi_diye_kaydedilir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![SahteYanit::baglanti_hatasi("bağlantı reddedildi")]);
    let mut surucu = test_oturumu(test_baglami(s, vec![], sanal_siniflandirici()), ayar);
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Hata);

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    let istek = kayitlar
        .iter()
        .find_map(|z| match &z.yuk {
            KayitOgesi::Istek { sonuc, .. } => Some(sonuc),
            _ => None,
        })
        .expect("istek kaydı bulunmalı");

    assert!(matches!(
        istek,
        IstekSonucu::Hata {
            asama: IstekAsamasi::Baglanti,
            ..
        }
    ));
}

#[tokio::test]
async fn akis_kopunca_istek_koptu_yazilir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![SahteYanit::akis_kopmasi(
        &["ilk kısım "],
        EvrenError::AkisYarimKaldi,
    )]);
    let mut surucu = test_oturumu(test_baglami(s, vec![], sanal_siniflandirici()), ayar);
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Hata);

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    assert!(kayitlar.iter().any(|z| matches!(
        &z.yuk,
        KayitOgesi::Istek {
            sonuc: IstekSonucu::Basarili,
            ..
        }
    )));
    assert!(
        kayitlar
            .iter()
            .any(|z| matches!(&z.yuk, KayitOgesi::IstekKoptu { .. }))
    );
}

#[tokio::test]
async fn yeni_ve_onceki_hesabi() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("sahte_arac", json!({"yol": "a.txt"})),
        SahteYanit::arac("sahte_arac", json!({"yol": "b.txt"})),
        SahteYanit::metin("tamam"),
    ]);
    let arac = std::sync::Arc::new(SahteArac::okuyan("sahte_arac"));
    let mut surucu = test_oturumu(test_baglami(s, vec![arac], sanal_siniflandirici()), ayar);
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    let istekler: Vec<_> = kayitlar
        .iter()
        .filter_map(|z| match &z.yuk {
            KayitOgesi::Istek {
                onceki,
                yeni,
                sonuc,
                ..
            } if *sonuc == IstekSonucu::Basarili => Some((*onceki, yeni.clone())),
            _ => None,
        })
        .collect();

    assert_eq!(istekler.len(), 3);
    // 1. istek: sistem istemi + araç tanımları + kullanıcı mesajı yeni, önceki 0
    assert_eq!(istekler[0].0, 0);
    assert_eq!(istekler[0].1.len(), 3);

    // 2. istek: model yanıtı + araç sonucu yeni (2), önceki 1 (kullanıcı)
    assert_eq!(istekler[1].0, 1);
    assert_eq!(istekler[1].1.len(), 2);

    // 3. istek: ikinci model yanıtı + araç sonucu yeni (2), önceki 3 (1 + 2)
    assert_eq!(istekler[2].0, 3);
    assert_eq!(istekler[2].1.len(), 2);
}

#[tokio::test]
async fn ikinci_denemede_yeni_bos() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::Hata(EvrenError::HizSiniri {
            tekrar_dene: Some(Duration::from_millis(10)),
        }),
        SahteYanit::metin("sonunda geldi"),
    ]);
    let mut surucu = test_oturumu(test_baglami(s, vec![], sanal_siniflandirici()), ayar);
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    let istekler: Vec<_> = kayitlar
        .iter()
        .filter_map(|z| match &z.yuk {
            KayitOgesi::Istek { onceki, yeni, .. } => Some((*onceki, yeni.clone())),
            _ => None,
        })
        .collect();

    assert_eq!(istekler.len(), 2);
    assert_eq!(istekler[1].0, 1);
    assert!(istekler[1].1.is_empty());
}

#[test]
fn kurtarmada_gonderilen_sayaci_kayittan_kurulur() {
    let satir_acilis = r#"{"sema":1,"oturum":"6f1d0a4e-2b8c-4c1e-9a55-0d6b1f3e2a10","tur":null,"sira":1,"yuk":{"tip":"OturumAcildi","model":"glm-5.3","kip":"Siki"}}"#;
    let satir_mesaj = r#"{"sema":1,"oturum":"6f1d0a4e-2b8c-4c1e-9a55-0d6b1f3e2a10","tur":"00000000-0000-0000-0000-000000000001","sira":2,"yuk":{"tip":"Mesaj","mesaj":{"role":"user","content":"merhaba"}}}"#;
    let satir_istek = r#"{"sema":1,"oturum":"6f1d0a4e-2b8c-4c1e-9a55-0d6b1f3e2a10","tur":"00000000-0000-0000-0000-000000000001","sira":3,"yuk":{"tip":"Istek","amac":"ana","istek_id":"r-1","kredi":null,"yonlendirilen_model":"glm-5.3","model":"glm-5.3","yeni":[{"kaynak":"mesaj:kullanıcı","sira":2,"bayt":7}],"onceki":0,"sonuc":{"tip":"Basarili"}}}"#;

    let z1: Zarf<KayitOgesi> = serde_json::from_str(satir_acilis).unwrap();
    let z2: Zarf<KayitOgesi> = serde_json::from_str(satir_mesaj).unwrap();
    let z3: Zarf<KayitOgesi> = serde_json::from_str(satir_istek).unwrap();

    let durum = kurtar(&[z1, z2, z3]).unwrap();
    assert_eq!(durum.gonderilen, 1);
}

#[test]
fn eski_istek_kaydi_acilir() {
    let satir = r#"{"sema":1,"oturum":"6f1d0a4e-2b8c-4c1e-9a55-0d6b1f3e2a10","tur":null,"sira":2,"yuk":{"tip":"Istek","amac":"ana","istek_id":"r-1","kredi":null,"yonlendirilen_model":"glm-5.3"}}"#;
    let zarf: Zarf<KayitOgesi> = serde_json::from_str(satir).unwrap();
    match zarf.yuk {
        KayitOgesi::Istek {
            model,
            yeni,
            onceki,
            sonuc,
            ..
        } => {
            assert_eq!(model, None);
            assert!(yeni.is_empty());
            assert_eq!(onceki, 0);
            assert_eq!(sonuc, IstekSonucu::Basarili);
        }
        _ => panic!("Istek bekleniyordu"),
    }
}

#[tokio::test]
async fn yanit_siniri_govde_gitti_diye_kaydedilir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![SahteYanit::Hata(EvrenError::YanitSiniri)]);
    let mut surucu = test_oturumu(test_baglami(s, vec![], sanal_siniflandirici()), ayar);
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Hata);

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    let istek = kayitlar
        .iter()
        .find_map(|z| match &z.yuk {
            KayitOgesi::Istek { sonuc, .. } => Some(sonuc),
            _ => None,
        })
        .expect("istek kaydı bulunmalı");

    assert!(matches!(
        istek,
        IstekSonucu::Hata {
            asama: IstekAsamasi::Yanit,
            ..
        }
    ));
}

fn yol_arg(y: &str) -> serde_json::Value {
    serde_json::json!({ "yol": y })
}

fn a(arac: &std::sync::Arc<SahteArac>) -> std::sync::Arc<dyn kalfa_core::arac::Arac> {
    arac.clone()
}

#[tokio::test]
async fn kullanici_mesaji_etiketlenir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("merhaba")]);
    let mut surucu = test_oturumu(test_baglami(s, vec![], sanal_siniflandirici()), ayar);
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    let mesaj_kaydi = kayitlar
        .iter()
        .find_map(|z| match &z.yuk {
            KayitOgesi::Mesaj { mesaj, kaynak }
                if mesaj.role == kalfa_evren::tipler::Role::User =>
            {
                Some(kaynak.clone())
            }
            _ => None,
        })
        .expect("kullanıcı mesajı bulunmalı");
    assert_eq!(mesaj_kaydi, Some("mesaj:kullanıcı".to_string()));

    let istek = kayitlar
        .iter()
        .find_map(|z| match &z.yuk {
            KayitOgesi::Istek { yeni, .. } => Some(yeni.clone()),
            _ => None,
        })
        .expect("istek bulunmalı");
    assert!(istek.iter().any(|o| o.kaynak == "mesaj:kullanıcı"));
}

#[tokio::test]
async fn arac_sonucu_aracin_kaynagini_tasir() {
    use std::sync::Arc;
    let d = tempfile::tempdir().unwrap();
    let (mut ayar, yol) = kayitli(d.path());
    ayar.kip = kalfa_core::izin::Kip::Serbest;
    let arac = Arc::new(SahteArac::okuyan("oku"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("oku", yol_arg("a.txt")),
        SahteYanit::metin("bitti"),
    ]);
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    let tool_mesaji = kayitlar
        .iter()
        .find_map(|z| match &z.yuk {
            KayitOgesi::Mesaj { mesaj, kaynak }
                if mesaj.role == kalfa_evren::tipler::Role::Tool =>
            {
                Some(kaynak.clone())
            }
            _ => None,
        })
        .expect("araç sonucu mesajı bulunmalı");
    assert_eq!(tool_mesaji, Some("sahte:oku".to_string()));

    let istekler: Vec<_> = kayitlar
        .iter()
        .filter_map(|z| match &z.yuk {
            KayitOgesi::Istek { yeni, .. } => Some(yeni.clone()),
            _ => None,
        })
        .collect();
    assert!(istekler.len() >= 2);
    assert!(istekler[1].iter().any(|o| o.kaynak == "sahte:oku"));
}

#[tokio::test]
async fn ret_ve_hata_kalfa_sonucu() {
    use kalfa_core::protokol::{Olay, OnayKarari};
    use std::sync::Arc;
    let d = tempfile::tempdir().unwrap();
    let (mut ayar, yol) = kayitli(d.path());
    ayar.kip = kalfa_core::izin::Kip::Siki;
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", yol_arg("a.txt")),
        SahteYanit::metin("anlaşıldı"),
    ]);
    let mut surucu = test_oturumu(test_baglami(s, vec![a(&yaz)], sanal_siniflandirici()), ayar);
    surucu.gonder(selam()).await;

    let onay_olayi = surucu
        .bekle(|o| matches!(o, Olay::OnayGerekiyor { .. }))
        .await;
    let (cagri, muhur) = match onay_olayi.yuk {
        Olay::OnayGerekiyor { ogeler, .. } => (ogeler[0].cagri, ogeler[0].muhur.clone()),
        _ => unreachable!(),
    };
    surucu
        .gonder(Komut::AracOnayi {
            cagri,
            karar: OnayKarari::Reddet {
                gerekce: Some("gerek yok".into()),
            },
            muhur,
        })
        .await;

    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    let ret_mesaji = kayitlar
        .iter()
        .find_map(|z| match &z.yuk {
            KayitOgesi::Mesaj { mesaj, kaynak }
                if mesaj.role == kalfa_evren::tipler::Role::Tool =>
            {
                Some(kaynak.clone())
            }
            _ => None,
        })
        .expect("ret mesajı bulunmalı");
    assert_eq!(ret_mesaji, Some("kalfa:sonuç".to_string()));

    let istekler: Vec<_> = kayitlar
        .iter()
        .filter_map(|z| match &z.yuk {
            KayitOgesi::Istek { yeni, .. } => Some(yeni.clone()),
            _ => None,
        })
        .collect();
    assert!(istekler.len() >= 2);
    assert!(istekler[1].iter().any(|o| o.kaynak == "kalfa:sonuç"));
}

#[tokio::test]
async fn kalfa_notu_etiketlenir() {
    use kalfa_core::arac::Arac;
    use kalfa_core::kimlik::TurId;
    use kalfa_core::protokol::Olay;
    use std::sync::Arc;

    let d = tempfile::tempdir().unwrap();
    let kayit_yolu = d.path().join("kayitlar/oturum.jsonl");
    let depo_yolu = d.path().join("geri-alma/oturum");
    let dosyalar = SahteGeriYukleyici::yeni();
    dosyalar.yaz("a.txt", b"v0");

    let mut ayar = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    ayar.kip = kalfa_core::izin::Kip::Proje;
    ayar.kayit_yolu = Some(kayit_yolu.clone());
    ayar.geri_alma_dizini = Some(depo_yolu);

    let yaz = Arc::new(SahteArac::yazan("yaz").dosyalar(dosyalar.clone()));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![("yaz", json!({"yol": "a.txt", "icerik": "v1"}))]),
        SahteYanit::metin("yazdım"),
        SahteYanit::metin("ikinci yanıt"),
    ]);

    let mut baglam = test_baglami(
        s.clone(),
        vec![yaz as Arc<dyn Arac>],
        sanal_siniflandirici(),
    );
    baglam.geri_yukleyici = Some(dosyalar.clone());

    let mut surucu = test_oturumu(baglam, ayar);
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let turlar: Vec<TurId> = surucu
        .gecmis
        .iter()
        .filter(|z| matches!(z.yuk, Olay::TurBasladi))
        .filter_map(|z| z.tur)
        .collect();
    let tur = turlar[0];

    surucu.gonder(Komut::GeriAlmaOnizle { tur }).await;
    let muhur = match surucu
        .bekle(|o| matches!(o, Olay::GeriAlmaPlani { .. }))
        .await
        .yuk
    {
        Olay::GeriAlmaPlani { muhur, .. } => muhur,
        _ => unreachable!(),
    };
    surucu.gonder(Komut::TuruGeriAl { tur, muhur }).await;
    surucu
        .bekle(|o| matches!(o, Olay::TurGeriAlindi { .. }))
        .await;

    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let kayitlar = OturumKaydi::oku(&kayit_yolu).unwrap();
    let istekler: Vec<_> = kayitlar
        .iter()
        .filter_map(|z| match &z.yuk {
            KayitOgesi::Istek { yeni, .. } => Some(yeni.clone()),
            _ => None,
        })
        .collect();
    assert!(istekler.len() >= 3);
    assert!(
        istekler[2].iter().any(|o| o.kaynak == "kalfa:not"),
        "İkinci tur isteğinde kalfa:not kaynağı bulunmalı: {:?}",
        istekler[2]
    );
}

#[test]
fn eski_mesaj_etiketsiz_acilir() {
    let satir = r#"{"sema":1,"oturum":"6f1d0a4e-2b8c-4c1e-9a55-0d6b1f3e2a10","tur":null,"sira":2,"yuk":{"tip":"Mesaj","mesaj":{"role":"user","content":"eski"}}}"#;
    let zarf: Zarf<KayitOgesi> = serde_json::from_str(satir).unwrap();
    match zarf.yuk {
        KayitOgesi::Mesaj { mesaj, kaynak } => {
            assert_eq!(mesaj.content.as_deref(), Some("eski"));
            assert_eq!(kaynak, None);
        }
        _ => panic!("Mesaj bekleniyordu"),
    }
}

#[tokio::test]
async fn ilk_istekte_sistem_ve_araclar_yazilir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("merhaba")]);
    let arac = std::sync::Arc::new(SahteArac::okuyan("oku"));
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
        ayar.clone(),
    );
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    let sistem_kaydi = kayitlar.iter().find_map(|z| match &z.yuk {
        KayitOgesi::SistemIstemi { metin } => Some((z.sira, metin.clone())),
        _ => None,
    });
    assert!(sistem_kaydi.is_some(), "SistemIstemi kaydı bulunmalı");
    let (sistem_sira, sistem_metin) = sistem_kaydi.unwrap();
    assert_eq!(sistem_metin, ayar.sistem_mesaji);

    let arac_kaydi = kayitlar.iter().find_map(|z| match &z.yuk {
        KayitOgesi::AracTanimlari { tanimlar } => Some((z.sira, tanimlar.clone())),
        _ => None,
    });
    assert!(arac_kaydi.is_some(), "AracTanimlari kaydı bulunmalı");
    let (arac_sira, tanimlar) = arac_kaydi.unwrap();
    assert!(tanimlar.iter().any(|t| t.function.name == "oku"));

    let istek = kayitlar
        .iter()
        .find_map(|z| match &z.yuk {
            KayitOgesi::Istek { yeni, .. } => Some(yeni.clone()),
            _ => None,
        })
        .expect("istek kaydı bulunmalı");

    let sistem_oge = istek
        .iter()
        .find(|o| o.kaynak == "kalfa:sistem")
        .expect("yeni içinde kalfa:sistem bulunmalı");
    assert_eq!(sistem_oge.sira, Some(sistem_sira));
    assert_eq!(sistem_oge.bayt, sistem_metin.len() as u64);

    let arac_oge = istek
        .iter()
        .find(|o| o.kaynak == "kalfa:araçlar")
        .expect("yeni içinde kalfa:araçlar bulunmalı");
    assert_eq!(arac_oge.sira, Some(arac_sira));
    assert!(arac_oge.bayt > 0);
}

#[tokio::test]
async fn degismeyince_yeniden_yazilmaz() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::metin("yanıt 1"),
        SahteYanit::metin("yanıt 2"),
    ]);
    let arac = std::sync::Arc::new(SahteArac::okuyan("oku"));
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    let sistem_sayisi = kayitlar
        .iter()
        .filter(|z| matches!(&z.yuk, KayitOgesi::SistemIstemi { .. }))
        .count();
    let arac_sayisi = kayitlar
        .iter()
        .filter(|z| matches!(&z.yuk, KayitOgesi::AracTanimlari { .. }))
        .count();
    assert_eq!(sistem_sayisi, 1, "SistemIstemi yalnızca bir kez yazılmalı");
    assert_eq!(arac_sayisi, 1, "AracTanimlari yalnızca bir kez yazılmalı");

    let istekler: Vec<_> = kayitlar
        .iter()
        .filter_map(|z| match &z.yuk {
            KayitOgesi::Istek { yeni, .. } => Some(yeni.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(istekler.len(), 2);
    // İkinci istekte sistem ve araçlar yeniden gönderilmez
    assert!(!istekler[1].iter().any(|o| o.kaynak == "kalfa:sistem"));
    assert!(!istekler[1].iter().any(|o| o.kaynak == "kalfa:araçlar"));
}

#[tokio::test]
async fn kabuk_sunulunca_araclar_yeniden_yazilir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::metin("yanıt 1"),
        SahteYanit::metin("yanıt 2"),
    ]);
    let arac = std::sync::Arc::new(SahteArac::okuyan("oku"));
    arac.sunulur_ayarla(false);
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    // Araç artık sunulur (örneğin kabuk yetkisi geldi)
    arac.sunulur_ayarla(true);
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    let arac_tanimlari: Vec<_> = kayitlar
        .iter()
        .filter_map(|z| match &z.yuk {
            KayitOgesi::AracTanimlari { tanimlar } => Some((z.sira, tanimlar.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        arac_tanimlari.len(),
        2,
        "Tanımlar değişince tekrar yazılmalı"
    );
    assert!(!arac_tanimlari[0].1.iter().any(|t| t.function.name == "oku"));
    assert!(arac_tanimlari[1].1.iter().any(|t| t.function.name == "oku"));

    let istekler: Vec<_> = kayitlar
        .iter()
        .filter_map(|z| match &z.yuk {
            KayitOgesi::Istek { yeni, .. } => Some(yeni.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(istekler.len(), 2);
    let ikinci_arac = istekler[1]
        .iter()
        .find(|o| o.kaynak == "kalfa:araçlar")
        .expect("ikinci istekte kalfa:araçlar bulunmalı");
    assert_eq!(ikinci_arac.sira, Some(arac_tanimlari[1].0));
}

#[tokio::test]
async fn kurtarmada_son_yazilan_bilinir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::metin("yanıt 1"),
        SahteYanit::metin("yanıt 2"),
    ]);
    let arac = std::sync::Arc::new(SahteArac::okuyan("oku"));
    let mut surucu = test_oturumu(
        test_baglami(s.clone(), vec![a(&arac)], sanal_siniflandirici()),
        ayar.clone(),
    );
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    surucu.kapat().await;

    // Kayıttan kurtarma durumunu oku
    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    let durum = kurtar(&kayitlar).unwrap();
    assert!(
        durum.son_sistem_istemi.is_some(),
        "son sistem istemi kurtarılmalı"
    );
    assert!(
        durum.son_arac_tanimlari.is_some(),
        "son araç tanımları kurtarılmalı"
    );

    // Oturumu kurtarılmış durumdan sürdür
    let (t, o, _) = kalfa_core::oturum::devam_et(
        &yol,
        ayar,
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
    )
    .unwrap();
    let mut devam_surucu = Surucu::yeni(t, o);
    devam_surucu.gonder(selam()).await;
    assert_eq!(devam_surucu.tur_sonu().await, BitisNedeni::Normal);

    let son_kayitlar = OturumKaydi::oku(&yol).unwrap();
    let sistem_sayisi = son_kayitlar
        .iter()
        .filter(|z| matches!(&z.yuk, KayitOgesi::SistemIstemi { .. }))
        .count();
    let arac_sayisi = son_kayitlar
        .iter()
        .filter(|z| matches!(&z.yuk, KayitOgesi::AracTanimlari { .. }))
        .count();
    assert_eq!(
        sistem_sayisi, 1,
        "Kurtarmadan sonra aynı sistem istemi tekrar yazılmamalı"
    );
    assert_eq!(
        arac_sayisi, 1,
        "Kurtarmadan sonra aynı araç tanımları tekrar yazılmamalı"
    );
}

#[tokio::test]
async fn baglanti_hatasindan_sonra_ogeler_yeniden_yeni_sayilir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::baglanti_hatasi("ağa erişilemedi"),
        SahteYanit::metin("tamamlandı"),
    ]);
    let arac = std::sync::Arc::new(SahteArac::okuyan("oku"));
    let mut surucu = test_oturumu(
        test_baglami(s.clone(), vec![a(&arac)], sanal_siniflandirici()),
        ayar.clone(),
    );
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Hata);

    // İkinci mesajı aynı oturumda gönder
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    let istekler: Vec<_> = kayitlar
        .iter()
        .filter_map(|z| match &z.yuk {
            KayitOgesi::Istek {
                onceki,
                yeni,
                sonuc,
                ..
            } => Some((*onceki, yeni.clone(), sonuc.clone())),
            _ => None,
        })
        .collect();

    assert_eq!(istekler.len(), 2);
    // 1. istek (bağlantı hatası): govde_gitti = false
    assert_eq!(istekler[0].0, 0);
    assert!(matches!(
        istekler[0].2,
        IstekSonucu::Hata {
            asama: IstekAsamasi::Baglanti,
            ..
        }
    ));

    // 2. istek: ilk istek gitmediği için önceki hâlâ 0 olmalı ve öğeler yeniden yeni sayılmalı
    assert_eq!(
        istekler[1].0, 0,
        "Bağlantı hatasından sonra önceki 0 kalmalı"
    );
    assert!(matches!(istekler[1].2, IstekSonucu::Basarili));
    assert!(
        istekler[1].1.iter().any(|o| o.kaynak == "kalfa:sistem"),
        "Sistem istemi yeniden yeni sayılmalı"
    );
    assert!(
        istekler[1].1.iter().any(|o| o.kaynak == "kalfa:araçlar"),
        "Araç tanımları yeniden yeni sayılmalı"
    );
    assert_eq!(
        istekler[1]
            .1
            .iter()
            .filter(|o| o.kaynak == "mesaj:kullanıcı")
            .count(),
        2,
        "İki kullanıcı mesajı da yeni sayılmalı"
    );

    // Kurtarma denetimi: ilk istekten sonra kurtarılsaydı gonderilen 0 olmalı
    let ilk_istek_idx = kayitlar
        .iter()
        .position(|z| {
            matches!(
                &z.yuk,
                KayitOgesi::Istek {
                    sonuc: IstekSonucu::Hata { .. },
                    ..
                }
            )
        })
        .unwrap();
    let durum = kurtar(&kayitlar[..=ilk_istek_idx]).unwrap();
    assert_eq!(
        durum.gonderilen, 0,
        "Bağlantı hatası gonderilen sayacını ilerletmemeli"
    );
    assert!(
        durum.son_sistem_istemi.is_none(),
        "Bağlantı hatasında son sistem istemi ilerlememeli"
    );
    assert!(
        durum.son_arac_tanimlari.is_none(),
        "Bağlantı hatasında son araç tanımları ilerlememeli"
    );
}
