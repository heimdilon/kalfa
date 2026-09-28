use std::path::Path;
use std::sync::Arc;

use kalfa_core::gecmis::{DurumOzeti, GorunumOgesi, durum_ozeti, gorunum_uret, sayfa};
use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::{CagriId, OturumId, TurId};
use kalfa_core::oturum::OturumAyarlari;
use kalfa_core::protokol::{AracDurumu, BitisNedeni, Komut, Olay, OnayKarari, SEMA_SURUMU, Zarf};
use kalfa_core::test_destek::*;
use kalfa_evren::KrediGozlemi;
use kalfa_evren::tipler::{ChatMessage, FunctionCall, ToolCall};
use serde_json::json;

struct Kayitci {
    oturum: OturumId,
    sira: u64,
    kayitlar: Vec<Zarf<KayitOgesi>>,
}

impl Kayitci {
    fn yeni() -> Self {
        Self {
            oturum: OturumId::yeni(),
            sira: 0,
            kayitlar: Vec::new(),
        }
    }

    fn ekle(&mut self, tur: Option<TurId>, yuk: KayitOgesi) -> u64 {
        self.sira += 1;
        self.kayitlar.push(Zarf {
            sema: SEMA_SURUMU,
            oturum: self.oturum,
            tur,
            sira: self.sira,
            yuk,
        });
        self.sira
    }

    fn mesaj(&mut self, tur: TurId, mesaj: ChatMessage) -> u64 {
        self.ekle(Some(tur), KayitOgesi::Mesaj { mesaj })
    }
}

fn cagri(id: &str, ad: &str) -> ToolCall {
    ToolCall {
        id: id.into(),
        kind: "function".into(),
        function: FunctionCall {
            name: ad.into(),
            arguments: "{\"yol\":\"a\"}".into(),
        },
    }
}

#[test]
fn metin_turunun_gorunumu() {
    let mut k = Kayitci::yeni();
    let tur = TurId::yeni();
    let s1 = k.mesaj(tur, ChatMessage::user("selam"));
    let mut yanit = ChatMessage::assistant(Some("merhaba".into()), vec![]);
    yanit.reasoning = Some("düşündüm".into());
    let s2 = k.mesaj(tur, yanit);
    let s3 = k.ekle(
        Some(tur),
        KayitOgesi::TurBitti {
            neden: BitisNedeni::Normal,
        },
    );
    assert_eq!(
        gorunum_uret(&k.kayitlar),
        vec![
            GorunumOgesi::KullaniciMesaji {
                sira: s1,
                tur: Some(tur),
                metin: "selam".into()
            },
            GorunumOgesi::Yanit {
                sira: s2,
                tur: Some(tur),
                metin: Some("merhaba".into()),
                dusunce: Some("düşündüm".into())
            },
            GorunumOgesi::TurSonu {
                sira: s3,
                tur: Some(tur),
                neden: BitisNedeni::Normal
            },
        ]
    );
}

#[test]
fn arac_cagrilarinin_durumlari() {
    let mut k = Kayitci::yeni();
    let tur = TurId::yeni();
    k.mesaj(tur, ChatMessage::user("üçünü yap"));
    let s = k.mesaj(
        tur,
        ChatMessage::assistant(
            None,
            vec![cagri("m1", "yaz"), cagri("m2", "yaz"), cagri("m3", "yaz")],
        ),
    );
    let (c1, c2, c3) = (CagriId::yeni(), CagriId::yeni(), CagriId::yeni());
    // m1 çalıştı ve bitti.
    k.ekle(
        Some(tur),
        KayitOgesi::AracBasladi {
            cagri: c1,
            model_cagri_id: "m1".into(),
            arac: "yaz".into(),
        },
    );
    k.mesaj(tur, ChatMessage::tool("m1", "yazıldı"));
    k.ekle(
        Some(tur),
        KayitOgesi::AracBitti {
            cagri: c1,
            durum: AracDurumu::Basarili,
        },
    );
    // m2 reddedildi: başladı kaydı yok; bitişi sonucun hemen ardından gelir.
    k.mesaj(
        tur,
        ChatMessage::tool("m2", "[kullanıcı reddetti] gerek yok"),
    );
    k.ekle(
        Some(tur),
        KayitOgesi::AracBitti {
            cagri: c2,
            durum: AracDurumu::Reddedildi,
        },
    );
    // m3 başladı, Kalfa sonuç gelmeden kapandı.
    k.ekle(
        Some(tur),
        KayitOgesi::AracBasladi {
            cagri: c3,
            model_cagri_id: "m3".into(),
            arac: "yaz".into(),
        },
    );
    let araclar: Vec<(String, Option<AracDurumu>, Option<String>, u64)> = gorunum_uret(&k.kayitlar)
        .into_iter()
        .filter_map(|o| match o {
            GorunumOgesi::AracCagrisi {
                model_cagri_id,
                durum,
                sonuc,
                sira,
                ..
            } => Some((model_cagri_id, durum, sonuc, sira)),
            _ => None,
        })
        .collect();
    assert_eq!(
        araclar,
        vec![
            (
                String::from("m1"),
                Some(AracDurumu::Basarili),
                Some(String::from("yazıldı")),
                s
            ),
            (
                String::from("m2"),
                Some(AracDurumu::Reddedildi),
                Some(String::from("[kullanıcı reddetti] gerek yok")),
                s
            ),
            (
                String::from("m3"),
                Some(AracDurumu::SonucuBilinmiyor),
                None,
                s
            ),
        ]
    );
}

#[tokio::test]
async fn gercek_oturum_kaydindan_gorunum() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("oturum.jsonl");
    let mut ayar = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    ayar.kayit_yolu = Some(yol.clone());
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", json!({"yol": "a"})),
        SahteYanit::metin("tamam"),
    ]);
    let mut surucu = test_oturumu(test_baglami(s, vec![yaz], sanal_siniflandirici()), ayar);
    surucu
        .gonder(Komut::KullaniciMesaji {
            metin: "yaz".into(),
        })
        .await;
    let onay = match surucu
        .bekle(|o| matches!(o, Olay::OnayGerekiyor { .. }))
        .await
        .yuk
    {
        Olay::OnayGerekiyor { ogeler, .. } => ogeler[0].clone(),
        _ => unreachable!(),
    };
    surucu
        .gonder(Komut::AracOnayi {
            cagri: onay.cagri,
            karar: OnayKarari::Reddet {
                gerekce: Some("gerek yok".into()),
            },
            muhur: onay.muhur,
        })
        .await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    surucu.kapat().await;

    let ogeler = gorunum_uret(&OturumKaydi::oku(&yol).unwrap());
    assert!(matches!(
        &ogeler[0],
        GorunumOgesi::KullaniciMesaji { metin, .. } if metin == "yaz"
    ));
    assert!(ogeler.iter().any(|o| matches!(
        o,
        GorunumOgesi::AracCagrisi { arac, durum: Some(AracDurumu::Reddedildi), sonuc: Some(s), .. }
            if arac == "yaz" && s.contains("gerek yok")
    )));
    assert!(ogeler.iter().any(|o| matches!(
        o,
        GorunumOgesi::Yanit { metin: Some(m), .. } if m == "tamam"
    )));
    assert!(matches!(
        ogeler.last(),
        Some(GorunumOgesi::TurSonu {
            neden: BitisNedeni::Normal,
            ..
        })
    ));
}

#[test]
fn sayfalama_tur_bazlidir() {
    let mut k = Kayitci::yeni();
    let mut turlar = Vec::new();
    for i in 0..3 {
        let tur = TurId::yeni();
        turlar.push(tur);
        k.mesaj(tur, ChatMessage::user(format!("soru {i}")));
        k.mesaj(
            tur,
            ChatMessage::assistant(Some(format!("yanıt {i}")), vec![]),
        );
        k.ekle(
            Some(tur),
            KayitOgesi::TurBitti {
                neden: BitisNedeni::Normal,
            },
        );
    }
    let ogeler = gorunum_uret(&k.kayitlar);
    let son = sayfa(&ogeler, None, 2);
    assert!(son.daha_eski_var);
    assert_eq!(son.ogeler.len(), 6);
    assert_eq!(son.ogeler[0].tur(), Some(turlar[1]));
    let once = sayfa(&ogeler, Some(son.ogeler[0].sira()), 2);
    assert!(!once.daha_eski_var);
    assert_eq!(once.ogeler.len(), 3);
    assert!(once.ogeler.iter().all(|o| o.tur() == Some(turlar[0])));
}

#[test]
fn durum_ozeti_kayittan_cikarilir() {
    let mut k = Kayitci::yeni();
    k.ekle(
        None,
        KayitOgesi::OturumAcildi {
            model: "glm-5.3".into(),
            kip: Kip::Siki,
            calisma_alani: None,
        },
    );
    k.ekle(
        None,
        KayitOgesi::ModelDegisti {
            model: "auto".into(),
        },
    );
    let gozlem = KrediGozlemi {
        kalan: None,
        tutulan: None,
        istek_id: Some("r1".into()),
        zaman_ms: 7,
    };
    k.ekle(
        None,
        KayitOgesi::Istek {
            amac: "ana".into(),
            istek_id: Some("r1".into()),
            kredi: Some(gozlem.clone()),
            yonlendirilen_model: Some("deepseek-v4-flash".into()),
        },
    );
    k.ekle(
        None,
        KayitOgesi::Kullanim {
            istem: 10,
            tamamlama: 5,
            dusunce: 2,
        },
    );
    k.ekle(
        None,
        KayitOgesi::Kullanim {
            istem: 20,
            tamamlama: 1,
            dusunce: 0,
        },
    );
    k.ekle(None, KayitOgesi::KipDegisti { kip: Kip::Proje });
    assert_eq!(
        durum_ozeti(&k.kayitlar),
        DurumOzeti {
            model: Some("auto".into()),
            kip: Some(Kip::Proje),
            yonlendirilen_model: Some("deepseek-v4-flash".into()),
            son_kredi: Some(gozlem),
            istem: 30,
            tamamlama: 6,
            dusunce: 2,
        }
    );
}

/// Güvenlik analizi Bulgu 1: aracın güvenlik uyarısı olay olarak yayılır,
/// kayda yazılır, geçmişte görünür ve modele `[güvenlik]` satırıyla gider.
#[tokio::test]
async fn guvenlik_uyarisi_olay_ve_kayit_olur() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("oturum.jsonl");
    let mut ayar = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    ayar.kayit_yolu = Some(yol.clone());
    ayar.kip = Kip::Serbest;
    let oku = Arc::new(SahteArac::okuyan("oku").guvenlik_uyarisi("`.git` kenara alındı"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("oku", json!({"yol": "a"})),
        SahteYanit::metin("tamam"),
    ]);
    let mut surucu = test_oturumu(
        test_baglami(s.clone(), vec![oku], sanal_siniflandirici()),
        ayar,
    );
    surucu
        .gonder(Komut::KullaniciMesaji {
            metin: "oku".into(),
        })
        .await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let sonuc = surucu
        .gecmis
        .iter()
        .position(|z| matches!(z.yuk, Olay::AracSonucu { .. }))
        .unwrap();
    let uyari = surucu
        .gecmis
        .iter()
        .position(|z| matches!(&z.yuk, Olay::GuvenlikUyarisi { mesaj, .. } if mesaj == "`.git` kenara alındı"))
        .unwrap();
    assert!(sonuc < uyari, "uyarı sonuçtan sonra gelir");
    surucu.kapat().await;

    let model = s.istekler()[1]
        .messages
        .iter()
        .find_map(|m| m.tool_call_id.as_ref().and(m.content.clone()))
        .unwrap();
    assert!(
        model.ends_with("\n[güvenlik] `.git` kenara alındı"),
        "{model}"
    );

    let ogeler = gorunum_uret(&OturumKaydi::oku(&yol).unwrap());
    assert!(ogeler.iter().any(|o| matches!(
        o,
        GorunumOgesi::GuvenlikUyarisi { mesaj, .. } if mesaj == "`.git` kenara alındı"
    )));
}
