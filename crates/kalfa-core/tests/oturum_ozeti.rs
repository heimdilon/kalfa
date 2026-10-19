//! Oturum özeti (compact): eşik, kapsam, zorunlu compact, hata durumları ve kurtarma.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kalfa_core::arac::Arac;
use kalfa_core::gonderim::oge_metni;
use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi, OzetYontemi};
use kalfa_core::kimlik::OturumId;
use kalfa_core::kurtarma::kurtar;
use kalfa_core::oturum::compact::OZET_KAYNAGI;
use kalfa_core::oturum::{OturumAyarlari, devam_et};
use kalfa_core::protokol::{BitisNedeni, Komut, Olay, OzetTetigi, SEMA_SURUMU, Zarf};
use kalfa_core::test_destek::*;
use kalfa_evren::tipler::{ChatMessage, FunctionCall, Role, ToolCall, Usage};
use kalfa_evren::{AkisOlayi, EvrenError};
use serde_json::json;

fn ayarlar(dizin: &Path) -> (OturumAyarlari, PathBuf) {
    let yol = dizin.join("oturum.jsonl");
    let mut a = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    a.kayit_yolu = Some(yol.clone());
    a.sinirlar.etkin_pencere = 32_000;
    (a, yol)
}

fn oku(icerik: &str) -> Arc<SahteArac> {
    Arc::new(SahteArac::okuyan("oku").yanit(icerik.to_string()))
}

fn oku_cagrisi(yol: &str) -> SahteYanit {
    SahteYanit::arac("oku", json!({ "yol": yol }))
}

fn kullanimla(yanit: SahteYanit, istem: u64) -> SahteYanit {
    match yanit {
        SahteYanit::Akis(mut olaylar) => {
            olaylar.retain(|o| !matches!(o, Ok(AkisOlayi::Kullanim(_))));
            olaylar.insert(
                0,
                Ok(AkisOlayi::Kullanim(Usage {
                    prompt_tokens: istem,
                    completion_tokens: 1,
                    total_tokens: istem + 1,
                    completion_tokens_details: None,
                    evren: None,
                })),
            );
            SahteYanit::Akis(olaylar)
        }
        diger => diger,
    }
}

fn surucu(s: &Arc<SahteSaglayici>, arac: Arc<SahteArac>, ayar: OturumAyarlari) -> Surucu {
    let araclar: Vec<Arc<dyn Arac>> = vec![arac];
    test_oturumu(
        test_baglami(s.clone(), araclar, sanal_siniflandirici()),
        ayar,
    )
}

fn icerik(m: &ChatMessage) -> &str {
    m.content.as_deref().unwrap_or_default()
}

#[tokio::test]
async fn eşik_altında_özetleyici_çağrılmaz() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, _) = ayarlar(dizin.path());
    let s = SahteSaglayici::yeni(vec![
        oku_cagrisi("a.txt"),
        oku_cagrisi("b.txt"),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = surucu(&s, oku("küçük çıktı"), ayar);
    d.gonder(Komut::KullaniciMesaji {
        metin: "iki dosyayı oku".into(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert!(s.sohbet_istekleri().is_empty());
    assert_eq!(s.istekler().len(), 3);
    assert!(
        !d.olaylar()
            .iter()
            .any(|o| matches!(o, Olay::OturumOzetlendi { .. }))
    );
}

#[tokio::test]
async fn eşik_üstünde_ana_istekten_önce_bir_kez_özetler() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, _) = ayarlar(dizin.path());
    let s = SahteSaglayici::yeni(vec![
        oku_cagrisi("a.txt"),
        kullanimla(SahteYanit::metin("ilk tur tamam"), 30_000),
        SahteYanit::metin("ikinci tur tamam"),
    ]);
    s.sohbet_ekle(SahteSohbet::Metin("ilk turun özeti".into()));
    let mut d = surucu(&s, oku(&"x".repeat(30_000)), ayar);

    d.gonder(Komut::KullaniciMesaji {
        metin: "a.txt'yi oku".into(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert!(s.sohbet_istekleri().is_empty());

    d.gonder(Komut::KullaniciMesaji {
        metin: "devam et".into(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);

    let ozet_istekleri = s.sohbet_istekleri();
    assert_eq!(ozet_istekleri.len(), 1);
    assert_eq!(ozet_istekleri[0].model, "glm-5.3");
    assert_eq!(ozet_istekleri[0].max_tokens, Some(8_192));

    let istekler = s.istekler();
    assert_eq!(istekler.len(), 3);
    let son_ana = &istekler[2];
    assert_eq!(son_ana.messages[0].role, Role::System);
    assert!(
        icerik(&son_ana.messages[1]).contains("<oturum_ozeti"),
        "{}",
        icerik(&son_ana.messages[1])
    );
    assert!(icerik(&son_ana.messages[1]).contains("ilk turun özeti"));
    assert_eq!(icerik(&son_ana.messages[2]), "devam et");

    let ozet_olaylari: Vec<_> = d
        .olaylar()
        .into_iter()
        .filter(|o| {
            matches!(
                o,
                Olay::OturumOzetlendi {
                    tetik: OzetTetigi::Otomatik,
                    ..
                }
            )
        })
        .collect();
    assert_eq!(ozet_olaylari.len(), 1);
}

#[tokio::test]
async fn özetleyici_isteği_gönderilmemiş_içeriği_taşımaz() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, _) = ayarlar(dizin.path());
    let s = SahteSaglayici::yeni(vec![
        kullanimla(SahteYanit::arac("oku_a", json!({ "yol": "a.txt" })), 30_000),
        kullanimla(SahteYanit::arac("oku_b", json!({ "yol": "b.txt" })), 30_000),
        SahteYanit::metin("tamam"),
    ]);
    s.sohbet_ekle(SahteSohbet::Metin("ara özet".into()));
    let arac_a: Arc<dyn Arac> = Arc::new(
        SahteArac::okuyan("oku_a").yanit(format!("GONDERILMIS_ISARET_{}", "a".repeat(20_000))),
    );
    let arac_b: Arc<dyn Arac> = Arc::new(SahteArac::okuyan("oku_b").yanit(format!(
        "GONDERILMEMIS_BENZERSIZ_ISARET_{}",
        "b".repeat(20_000)
    )));
    let mut d = test_oturumu(
        test_baglami(s.clone(), vec![arac_a, arac_b], sanal_siniflandirici()),
        ayar,
    );
    d.gonder(Komut::KullaniciMesaji {
        metin: "iki dosyayı sırayla oku".into(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);

    let ozet_istekleri = s.sohbet_istekleri();
    assert_eq!(ozet_istekleri.len(), 1);
    let ozet_girdisi = icerik(&ozet_istekleri[0].messages[1]);
    assert!(ozet_girdisi.contains("GONDERILMIS_ISARET_"));
    assert!(
        !ozet_girdisi.contains("GONDERILMEMIS_BENZERSIZ_ISARET_"),
        "gönderilmemiş araç çıktısı özetleyiciye gitmemeli"
    );

    let son_ana = &s.istekler()[2];
    assert!(
        son_ana
            .messages
            .iter()
            .any(|m| icerik(m).contains("GONDERILMEMIS_BENZERSIZ_ISARET_")),
        "gönderilmemiş araç çıktısı ana istekte yer almalı"
    );
}

#[tokio::test]
async fn bağlam_aşımında_tek_zorunlu_compact() {
    let asim = || {
        SahteYanit::Hata(EvrenError::BaglamAsimi {
            mesaj: "pencere aşıldı".into(),
        })
    };

    // 1. BaglamAsimi -> özet -> başarı
    let dizin = tempfile::tempdir().unwrap();
    let (mut ayar, _) = ayarlar(dizin.path());
    ayar.sinirlar.etkin_pencere = 200_000;
    let s = SahteSaglayici::yeni(vec![
        oku_cagrisi("a.txt"),
        asim(),
        SahteYanit::metin("kurtarıldı"),
    ]);
    s.sohbet_ekle(SahteSohbet::Metin("zorunlu özet".into()));
    let mut d = surucu(&s, oku("orta boy çıktı"), ayar);
    d.gonder(Komut::KullaniciMesaji {
        metin: "oku".into(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(s.sohbet_istekleri().len(), 1);
    assert!(d.olaylar().iter().any(|o| matches!(
        o,
        Olay::OturumOzetlendi {
            tetik: OzetTetigi::Zorunlu,
            ..
        }
    )));

    // 2. BaglamAsimi -> özet -> BaglamAsimi -> hata, özetleyici yalnız bir kez çağrılır
    let dizin2 = tempfile::tempdir().unwrap();
    let (mut ayar2, _) = ayarlar(dizin2.path());
    ayar2.sinirlar.etkin_pencere = 200_000;
    let s2 = SahteSaglayici::yeni(vec![oku_cagrisi("a.txt"), asim(), asim()]);
    s2.sohbet_ekle(SahteSohbet::Metin("zorunlu özet".into()));
    s2.sohbet_ekle(SahteSohbet::Metin("ikinci kez çağrılmamalı".into()));
    let mut d2 = surucu(&s2, oku("orta boy çıktı"), ayar2);
    d2.gonder(Komut::KullaniciMesaji {
        metin: "oku".into(),
    })
    .await;
    assert_eq!(d2.tur_sonu().await, BitisNedeni::Hata);
    assert_eq!(s2.sohbet_istekleri().len(), 1);
    assert!(
        d2.hatalar()
            .iter()
            .any(|h| h.starts_with("Bağlam modelin penceresini aşıyor")),
        "{:?}",
        d2.hatalar()
    );
}

#[tokio::test]
async fn özetleyici_hatasında_bağlam_değişmez_ve_tur_sürer() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, _) = ayarlar(dizin.path());
    let s = SahteSaglayici::yeni(vec![
        oku_cagrisi("a.txt"),
        kullanimla(SahteYanit::metin("ilk yanıt"), 30_000),
        SahteYanit::metin("ikinci yanıt"),
    ]);
    s.sohbet_ekle(SahteSohbet::Hata(EvrenError::Sunucu {
        durum: 500,
        kod: None,
        mesaj: "iç sunucu hatası".into(),
    }));
    let mut d = surucu(&s, oku(&"x".repeat(30_000)), ayar);

    d.gonder(Komut::KullaniciMesaji {
        metin: "ilk mesaj".into(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);

    d.gonder(Komut::KullaniciMesaji {
        metin: "ikinci mesaj".into(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);

    assert_eq!(s.sohbet_istekleri().len(), 1);
    assert!(
        d.hatalar()
            .iter()
            .any(|h| h.starts_with("Bağlam özetlenemedi")),
        "{:?}",
        d.hatalar()
    );
    assert!(
        !d.olaylar()
            .iter()
            .any(|o| matches!(o, Olay::OturumOzetlendi { .. }))
    );
    let son_ana = &s.istekler()[2];
    assert!(
        !son_ana
            .messages
            .iter()
            .skip(1)
            .any(|m| icerik(m).contains("<oturum_ozeti"))
    );
    assert_eq!(icerik(&son_ana.messages[1]), "ilk mesaj");
}

#[tokio::test]
async fn günlük_sınırda_compact_durur_ve_tur_biter() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, _) = ayarlar(dizin.path());
    let s = SahteSaglayici::yeni(vec![
        oku_cagrisi("a.txt"),
        kullanimla(SahteYanit::metin("ilk yanıt"), 30_000),
        SahteYanit::metin("buraya ulaşılmamalı"),
    ]);
    s.sohbet_ekle(SahteSohbet::Hata(EvrenError::GunlukSinir {
        sifirlanma: Some("2026-10-20T00:00:00Z".into()),
    }));
    let mut d = surucu(&s, oku(&"x".repeat(30_000)), ayar);

    d.gonder(Komut::KullaniciMesaji {
        metin: "ilk".into(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);

    d.gonder(Komut::KullaniciMesaji {
        metin: "ikinci".into(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Hata);

    assert_eq!(s.istekler().len(), 2, "ana istek gönderilmemeli");
    assert!(
        d.hatalar()
            .iter()
            .any(|h| h.contains("2026-10-20T00:00:00Z")),
        "{:?}",
        d.hatalar()
    );
}

#[tokio::test]
async fn compact_sonrası_gönderilmeden_kurtarma_aynı_bağlamı_verir() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, yol) = ayarlar(dizin.path());
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("oku_a", json!({ "yol": "a.txt" })),
        kullanimla(SahteYanit::arac("oku_b", json!({ "yol": "b.txt" })), 30_000),
        SahteYanit::baglanti_hatasi("kopuk"),
        SahteYanit::metin("canlı devam"),
    ]);
    s.sohbet_ekle(SahteSohbet::Metin("kapsamlı özet".into()));
    let arac_a: Arc<dyn Arac> = Arc::new(SahteArac::okuyan("oku_a").yanit("x".repeat(90_000)));
    let arac_b: Arc<dyn Arac> = Arc::new(SahteArac::okuyan("oku_b").yanit("küçük".to_string()));
    let mut d = test_oturumu(
        test_baglami(
            s.clone(),
            vec![arac_a.clone(), arac_b.clone()],
            sanal_siniflandirici(),
        ),
        ayar.clone(),
    );

    d.gonder(Komut::KullaniciMesaji {
        metin: "iki dosyayı oku".into(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Hata);
    assert_eq!(s.sohbet_istekleri().len(), 1);

    // Bağlantı hatasından hemen sonraki kaydı kurtarma testi için kopyala.
    let kurtarma_yolu = dizin.path().join("kurtarma.jsonl");
    std::fs::copy(&yol, &kurtarma_yolu).unwrap();

    // Canlı oturumda bir sonraki ana istek:
    d.gonder(Komut::KullaniciMesaji {
        metin: "devam".into(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    let canli_mesajlar = s.istekler().last().unwrap().messages.clone();
    d.kapat().await;

    // Kayıttan açılan oturumda aynı sonraki ana istek:
    let s2 = SahteSaglayici::yeni(vec![SahteYanit::metin("kurtarılan devam")]);
    let (tutamac, olaylar, _) = devam_et(
        &kurtarma_yolu,
        ayar,
        test_baglami(s2.clone(), vec![arac_a, arac_b], sanal_siniflandirici()),
    )
    .unwrap();
    let mut d2 = Surucu::yeni(tutamac, olaylar);
    d2.gonder(Komut::KullaniciMesaji {
        metin: "devam".into(),
    })
    .await;
    assert_eq!(d2.tur_sonu().await, BitisNedeni::Normal);
    let kurtarilan_mesajlar = s2.istekler().last().unwrap().messages.clone();
    d2.kapat().await;

    assert_eq!(kurtarilan_mesajlar, canli_mesajlar);

    let kayitlar = OturumKaydi::oku(&kurtarma_yolu).unwrap();
    let ozet_sira = kayitlar
        .iter()
        .find_map(|z| matches!(z.yuk, KayitOgesi::OturumOzetlendi { .. }).then_some(z.sira))
        .unwrap();
    let son_ana_yeni = kayitlar
        .iter()
        .rev()
        .find_map(|z| match &z.yuk {
            KayitOgesi::Istek { amac, yeni, .. } if amac == "ana" => Some(yeni.clone()),
            _ => None,
        })
        .unwrap();
    assert!(
        son_ana_yeni
            .iter()
            .any(|o| o.kaynak == OZET_KAYNAGI && o.sira == Some(ozet_sira)),
        "{son_ana_yeni:?}"
    );
    let ozet_metni = oge_metni(&kayitlar, ozet_sira).unwrap();
    assert!(ozet_metni.contains("kapsamlı özet"));
}

#[test]
fn eski_5a_özetli_kayıt_açılır() {
    let dizin = tempfile::tempdir().unwrap();
    let yol = dizin.path().join("eski.jsonl");
    let oturum = OturumId::yeni();
    let z = |sira, yuk| Zarf {
        sema: SEMA_SURUMU,
        oturum,
        tur: None,
        sira,
        yuk,
    };
    let tc = ToolCall {
        id: "c1".into(),
        kind: "function".into(),
        function: FunctionCall {
            name: "oku".into(),
            arguments: "{\"yol\":\"a.txt\"}".into(),
        },
    };
    let mut k = OturumKaydi::olustur(&yol).unwrap();
    k.yaz(&z(
        1,
        KayitOgesi::OturumAcildi {
            model: "glm-5.3".into(),
            kip: Kip::Siki,
            calisma_alani: None,
        },
    ))
    .unwrap();
    k.yaz(&z(
        2,
        KayitOgesi::Mesaj {
            mesaj: ChatMessage::user("oku"),
            kaynak: Some("mesaj:kullanıcı".into()),
        },
    ))
    .unwrap();
    k.yaz(&z(
        3,
        KayitOgesi::Mesaj {
            mesaj: ChatMessage::assistant(None, vec![tc]),
            kaynak: Some("model".into()),
        },
    ))
    .unwrap();
    k.yaz(&z(
        4,
        KayitOgesi::Mesaj {
            mesaj: ChatMessage::tool("c1", "uzun özgün çıktı"),
            kaynak: Some("sahte:oku".into()),
        },
    ))
    .unwrap();
    k.yaz(&z(
        5,
        KayitOgesi::BaglamOzeti {
            mesaj_sira: 4,
            kaynak: "özet:sahte:oku".into(),
            icerik: "<arac_sonucu kaynak=\"özet:sahte:oku\" sinir=\"s\">\neski 5a özeti\n</arac_sonucu sinir=\"s\">".into(),
            yontem: OzetYontemi::Ozet,
        },
    ))
    .unwrap();
    drop(k);

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    let durum = kurtar(&kayitlar).unwrap();
    assert_eq!(durum.mesajlar.len(), 3);
    assert!(
        durum.mesajlar[2]
            .content
            .as_deref()
            .unwrap()
            .contains("eski 5a özeti")
    );
    assert_eq!(
        durum.mesaj_bilgileri[2],
        (Some(5), Some("özet:sahte:oku".to_string()))
    );
    assert!(durum.bekleyen_ozetler.contains(&2));
}

#[tokio::test]
async fn elle_compact_talimatı_özetleyiciye_iletir() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, _) = ayarlar(dizin.path());
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("ilk yanıt")]);
    s.sohbet_ekle(SahteSohbet::Metin("odaklı özet".into()));
    let mut d = surucu(&s, oku("küçük"), ayar);

    d.gonder(Komut::KullaniciMesaji {
        metin: "merhaba".into(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);

    d.gonder(Komut::Ozetle {
        talimat: Some("odak: testler".into()),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);

    let ozet_istekleri = s.sohbet_istekleri();
    assert_eq!(ozet_istekleri.len(), 1);
    let kullanici_girdisi = icerik(&ozet_istekleri[0].messages[1]);
    assert!(
        kullanici_girdisi.ends_with("Kullanıcının özet talimatı: odak: testler"),
        "{kullanici_girdisi}"
    );
    assert!(d.olaylar().iter().any(|o| matches!(
        o,
        Olay::OturumOzetlendi {
            tetik: OzetTetigi::Elle,
            talimat: Some(t),
            ozet,
            ..
        } if t == "odak: testler" && ozet == "odaklı özet"
    )));
}

#[tokio::test]
async fn elle_compact_boş_oturumda_istek_atmaz() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, _) = ayarlar(dizin.path());
    let s = SahteSaglayici::yeni(vec![]);
    let mut d = surucu(&s, oku("küçük"), ayar);

    d.gonder(Komut::Ozetle { talimat: None }).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);

    assert!(s.istekler().is_empty());
    assert!(s.sohbet_istekleri().is_empty());
    assert_eq!(d.hatalar(), vec!["Özetlenecek bir şey yok.".to_string()]);
}

#[tokio::test]
async fn elle_compact_tur_sürerken_reddedilir() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, _) = ayarlar(dizin.path());
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", json!({ "yol": "a.txt" })),
        SahteYanit::metin("tamam"),
    ]);
    let yaz: Arc<dyn Arac> = Arc::new(SahteArac::yazan("yaz"));
    let mut d = test_oturumu(
        test_baglami(s.clone(), vec![yaz], sanal_siniflandirici()),
        ayar,
    );

    d.gonder(Komut::KullaniciMesaji {
        metin: "a.txt yaz".into(),
    })
    .await;
    let (cagri, muhur) = match d
        .bekle(|o| matches!(o, Olay::OnayGerekiyor { .. }))
        .await
        .yuk
    {
        Olay::OnayGerekiyor { ogeler, .. } => (ogeler[0].cagri, ogeler[0].muhur.clone()),
        diger => unreachable!("{diger:?}"),
    };

    // Tur onay beklerken elle compact reddedilir:
    d.gonder(Komut::Ozetle { talimat: None }).await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("Önceki tur sürüyor")))
        .await;

    d.gonder(Komut::AracOnayi {
        cagri,
        karar: kalfa_core::protokol::OnayKarari::Onayla,
        muhur,
        kisayol: false,
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(s.istekler().len(), 2);
    assert!(s.sohbet_istekleri().is_empty());
}

#[tokio::test]
async fn elle_compact_sonrası_bağlam_yalnız_özet() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, _) = ayarlar(dizin.path());
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::metin("ilk yanıt"),
        SahteYanit::metin("ikinci yanıt"),
    ]);
    s.sohbet_ekle(SahteSohbet::Metin("birinci turun özeti".into()));
    let mut d = surucu(&s, oku("küçük"), ayar);

    d.gonder(Komut::KullaniciMesaji {
        metin: "ilk soru".into(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);

    d.gonder(Komut::Ozetle { talimat: None }).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);

    d.gonder(Komut::KullaniciMesaji {
        metin: "yeni kullanıcı mesajı".into(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);

    let istekler = s.istekler();
    assert_eq!(istekler.len(), 2);
    let m = &istekler[1].messages;
    assert_eq!(m.len(), 3);
    assert_eq!(m[0].role, Role::System);
    assert_eq!(m[1].role, Role::User);
    assert!(icerik(&m[1]).contains("<oturum_ozeti"));
    assert!(icerik(&m[1]).contains("birinci turun özeti"));
    assert_eq!(m[2].role, Role::User);
    assert_eq!(icerik(&m[2]), "yeni kullanıcı mesajı");
}

#[tokio::test]
async fn bağlam_durumu_kullanımsız_yanıtta_ve_model_değişince_yayılır() {
    let dizin = tempfile::tempdir().unwrap();
    let (mut ayar, _) = ayarlar(dizin.path());
    ayar.sinirlar.etkin_pencere = 1_000_000;
    // Kullanım bildirmeyen geçerli yanıt.
    let s = SahteSaglayici::yeni(vec![SahteYanit::Akis(vec![
        Ok(AkisOlayi::Metin("tamam".into())),
        Ok(AkisOlayi::Bitti {
            neden: "stop".into(),
        }),
    ])]);
    let mut d = surucu(&s, oku("x"), ayar);
    let esikler = |d: &Surucu| -> Vec<u64> {
        d.olaylar()
            .iter()
            .filter_map(|o| match o {
                Olay::BaglamDurumu { esik, .. } => Some(*esik),
                _ => None,
            })
            .collect()
    };

    d.gonder(Komut::KullaniciMesaji {
        metin: "selam".into(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    // glm-5.3: min(524 288, 1 000 000) × 0,9.
    assert_eq!(esikler(&d).last(), Some(&471_859));

    d.gonder(Komut::ModelDegistir {
        model: "qwen3.8-flash-next".into(),
    })
    .await;
    d.bekle(|o| matches!(o, Olay::BaglamDurumu { esik: 235_929, .. }))
        .await;
}
