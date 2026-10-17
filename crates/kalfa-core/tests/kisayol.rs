//! "Bu oturumda izin ver" kısayolları (aşama 3 tasarımı §9.1; 3e planı
//! Görev 1).

use std::path::Path;
use std::sync::Arc;

use kalfa_core::arac::{
    Arac, Hazirlik, McpAgKarti, McpAracHazirligi, McpAracTanimOzeti, McpBaslatmaSayaci, McpErisim,
    McpKartEtiketi, McpKorumaOzeti, McpPolitikaOzeti, McpTanimOzeti, McpTarifi,
    McpYalitimProfiliOzeti, YazmaHedefi,
};
use kalfa_core::gorunum::GORUNMEZ_UYARISI;
use kalfa_core::izin::{
    AGLI_KABUK_UYARISI, ALAN_HAZIRLIGI_UYARISI, AYNI_ADIM_UYARISI, BAGLANTI_UYARISI,
    GERI_ALINAMAZ_UYARISI, GIZLI_UYARISI, ILK_KABUK_UYARISI, KORUMALI_UYARISI, Karar, Kip,
    YALITIMSIZ_UYARISI, YAPILANDIRMA_UYARISI, kabuk_karari,
};
use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
use kalfa_core::kisayol::{KisayolTeklifi, teklif};
use kalfa_core::oturum::{OturumAyarlari, devam_et};
use kalfa_core::protokol::{BitisNedeni, Komut, Olay, OnayKarari, OnayOgesi};
use kalfa_core::test_destek::*;
use kalfa_core::yol::CalismaYolu;
use serde_json::json;

fn ayarlar(kip: Kip) -> OturumAyarlari {
    let mut a = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    a.kip = kip;
    a
}

fn surucu(s: &Arc<SahteSaglayici>, araclar: Vec<Arc<dyn Arac>>, kip: Kip) -> Surucu {
    test_oturumu(
        test_baglami(s.clone(), araclar, sanal_siniflandirici()),
        ayarlar(kip),
    )
}

fn mesaj() -> Komut {
    Komut::KullaniciMesaji {
        metin: "yap".into(),
    }
}

async fn onay_bekle(d: &mut Surucu) -> Vec<OnayOgesi> {
    match d
        .bekle(|o| matches!(o, Olay::OnayGerekiyor { .. }))
        .await
        .yuk
    {
        Olay::OnayGerekiyor { ogeler, .. } => ogeler,
        _ => unreachable!(),
    }
}

fn onayla(o: &OnayOgesi, kisayol: bool) -> Komut {
    Komut::AracOnayi {
        cagri: o.cagri,
        karar: OnayKarari::Onayla,
        muhur: o.muhur.clone(),
        kisayol,
    }
}

/// Son `Kisayollar` olayındaki liste.
fn kisayollar(d: &Surucu) -> Option<Vec<KisayolTeklifi>> {
    d.gecmis.iter().rev().find_map(|z| match &z.yuk {
        Olay::Kisayollar { liste } => Some(liste.iter().map(|k| k.teklif.clone()).collect()),
        _ => None,
    })
}

fn dosya_hazirligi(yollar: &[&str]) -> Hazirlik {
    Hazirlik {
        yazilan: yollar
            .iter()
            .map(|y| YazmaHedefi::yeni(CalismaYolu::coz(y).unwrap(), None, 1))
            .collect(),
        ..Default::default()
    }
}

fn tarif(program: Option<&str>, dizin: &str) -> KabukTarifi {
    KabukTarifi {
        komut: "cargo test --all".into(),
        bicim: KabukBicimi::Program,
        program: program.map(str::to_string),
        dizin: dizin.into(),
        ag: false,
        ag_hostlari: Vec::new(),
        yalitim: YalitimOzeti::Yalitimli {
            arka_uc: "bwrap".into(),
            profil: "p".into(),
        },
        ilk_cagri: false,
        alan_hazirligi: false,
    }
}

fn kabuk_hazirligi(t: KabukTarifi) -> Hazirlik {
    Hazirlik {
        tum_calisma_alani: true,
        kabuk: Some(t),
        ..Default::default()
    }
}

fn mcp_hazirligi(
    tanim: u8,
    politika: u8,
    koruma: u8,
    baslatma: u64,
    arac_tanimi: u8,
    erisim: McpErisim,
) -> Hazirlik {
    Hazirlik {
        mcp: Some(McpAracHazirligi {
            sunucu: McpKartEtiketi::yeni("deneme").unwrap(),
            arac: McpKartEtiketi::yeni("dosya_oku").unwrap(),
            kip: Kip::Proje,
            erisim,
            ag: McpAgKarti::Kapali,
            ilk_cagri: false,
            tarif: McpTarifi::yeni(
                McpTanimOzeti::ozetten([tanim; 32]),
                McpPolitikaOzeti::ozetten([politika; 32]),
                McpKorumaOzeti::ozetten([koruma; 32]),
                McpYalitimProfiliOzeti::ozetten([4; 32]),
                McpBaslatmaSayaci::yeni(baslatma),
            ),
            arac_tanimi: McpAracTanimOzeti::ozetten([arac_tanimi; 32]),
        }),
        ..Default::default()
    }
}

fn onayli(uyarilar: &[&str]) -> Karar {
    Karar::Onayli {
        uyarilar: uyarilar.iter().map(|u| u.to_string()).collect(),
    }
}

/// Tasarım §9.1: uyarı taşıyan hiçbir öğe kısayol almaz; kabukta ek koşullar.
#[test]
fn uygunluk_tablosu() {
    let dosya = dosya_hazirligi(&["src/a.rs"]);
    assert_eq!(
        teklif("dosya_duzenle", &dosya, &onayli(&[])),
        Some(KisayolTeklifi::Dosya {
            arac: "dosya_duzenle".into(),
            dizin: "src".into(),
        })
    );
    assert_eq!(
        teklif("dosya_yaz", &dosya_hazirligi(&["a.rs"]), &onayli(&[])),
        Some(KisayolTeklifi::Dosya {
            arac: "dosya_yaz".into(),
            dizin: ".".into(),
        })
    );
    for uyari in [
        GIZLI_UYARISI,
        KORUMALI_UYARISI,
        YAPILANDIRMA_UYARISI,
        BAGLANTI_UYARISI,
        ILK_KABUK_UYARISI,
        ALAN_HAZIRLIGI_UYARISI,
        AGLI_KABUK_UYARISI,
        YALITIMSIZ_UYARISI,
        GORUNMEZ_UYARISI,
        AYNI_ADIM_UYARISI,
        GERI_ALINAMAZ_UYARISI,
    ] {
        assert_eq!(
            teklif("dosya_yaz", &dosya, &onayli(&[uyari])),
            None,
            "{uyari}"
        );
    }
    assert_eq!(teklif("dosya_yaz", &dosya, &Karar::Serbest), None);
    assert_eq!(
        teklif(
            "dosya_yaz",
            &dosya,
            &Karar::YerelOnayli {
                uyarilar: Vec::new()
            }
        ),
        None
    );
    // İki hedef ya da yazmayan araç teklif almaz.
    let iki = dosya_hazirligi(&["src/a.rs", "src/b.rs"]);
    assert_eq!(teklif("dosya_yaz", &iki, &onayli(&[])), None);
    assert_eq!(teklif("kabuk", &dosya, &onayli(&[])), None);

    // Kabuk: kararı izin motoru verir; yalnızca program biçimli, yalıtımlı,
    // ağsız ve ilk olmayan çağrı teklif alır.
    let uygun = tarif(Some("cargo"), ".");
    let karar = kabuk_karari(Kip::Proje, &uygun);
    assert_eq!(
        teklif("kabuk", &kabuk_hazirligi(uygun.clone()), &karar),
        Some(KisayolTeklifi::Kabuk {
            program: "cargo".into(),
            dizin: ".".into(),
        })
    );
    let degiskenler: Vec<(&str, KabukTarifi)> = vec![
        (
            "ilk çağrı",
            KabukTarifi {
                ilk_cagri: true,
                ..uygun.clone()
            },
        ),
        (
            "ağlı",
            KabukTarifi {
                ag: true,
                ..uygun.clone()
            },
        ),
        (
            "alan hazırlığı",
            KabukTarifi {
                alan_hazirligi: true,
                ..uygun.clone()
            },
        ),
        (
            "yalıtımsız",
            KabukTarifi {
                yalitim: YalitimOzeti::Yalitimsiz {
                    talimat: "t".into(),
                },
                ..uygun.clone()
            },
        ),
    ];
    for (ad, t) in degiskenler {
        let karar = kabuk_karari(Kip::Proje, &t);
        assert_eq!(teklif("kabuk", &kabuk_hazirligi(t), &karar), None, "{ad}");
    }
    // Betik ya da adı bilinmeyen program: karar uyarısız olsa da teklif yok.
    let betik = KabukTarifi {
        bicim: KabukBicimi::Betik {
            yorumlayici: "bash".into(),
        },
        program: None,
        ..uygun.clone()
    };
    assert_eq!(teklif("kabuk", &kabuk_hazirligi(betik), &onayli(&[])), None);
    assert_eq!(
        teklif("kabuk", &kabuk_hazirligi(tarif(None, ".")), &onayli(&[])),
        None
    );
}

#[test]
fn mcp_kisayolu_t_d_ve_politikayi_baglar_n_ve_k_degisimi_kapsami_korur() {
    let h = mcp_hazirligi(1, 2, 3, 4, 5, McpErisim::Yalitimli);
    let k = teklif("mcp__deneme__dosya_oku", &h, &onayli(&[])).unwrap();
    assert!(k.kapsar("mcp__deneme__dosya_oku", &h));
    assert!(!k.kapsar("mcp__diger__dosya_oku", &h));

    // N veya yalnız K değişirse R değişse de kapsam korunur.
    assert!(k.kapsar(
        "mcp__deneme__dosya_oku",
        &mcp_hazirligi(1, 2, 9, 40, 5, McpErisim::Yalitimli)
    ));
    assert!(k.kapsar(
        "mcp__deneme__dosya_oku",
        &mcp_hazirligi(1, 2, 3, 41, 5, McpErisim::Yalitimli)
    ));

    // T, D veya Π değişince kısayolun bağlandığı onay kapsamı değişir.
    for guncel in [
        mcp_hazirligi(8, 2, 3, 4, 5, McpErisim::Yalitimli),
        mcp_hazirligi(1, 2, 3, 4, 8, McpErisim::Yalitimli),
        mcp_hazirligi(1, 8, 3, 4, 5, McpErisim::Yalitimli),
    ] {
        assert!(!k.kapsar("mcp__deneme__dosya_oku", &guncel));
    }

    assert_eq!(
        teklif(
            "mcp__deneme__dosya_oku",
            &mcp_hazirligi(1, 2, 3, 4, 5, McpErisim::TamErisim),
            &onayli(&[])
        ),
        None,
        "tam erişimli MCP aracı kısayol almaz"
    );
}

#[test]
fn dosya_kisayolu_alt_dizini_kapsar_benzer_adi_kapsamaz() {
    let k = KisayolTeklifi::Dosya {
        arac: "dosya_yaz".into(),
        dizin: "src".into(),
    };
    assert!(k.kapsar("dosya_yaz", &dosya_hazirligi(&["src/b.rs"])));
    assert!(k.kapsar("dosya_yaz", &dosya_hazirligi(&["src/alt/c.rs"])));
    assert!(!k.kapsar("dosya_yaz", &dosya_hazirligi(&["srcx/c.rs"])));
    assert!(!k.kapsar("dosya_yaz", &dosya_hazirligi(&["c.rs"])));
    assert!(!k.kapsar("dosya_duzenle", &dosya_hazirligi(&["src/b.rs"])));
    assert!(!k.kapsar("dosya_yaz", &dosya_hazirligi(&["src/b.rs", "diger/c.rs"])));
}

#[test]
fn kabuk_kisayolu_argumanlari_serbest_birakir_dizini_birakmaz() {
    let k = KisayolTeklifi::Kabuk {
        program: "cargo".into(),
        dizin: ".".into(),
    };
    let baska_arguman = KabukTarifi {
        komut: "cargo build --release".into(),
        ..tarif(Some("cargo"), ".")
    };
    assert!(k.kapsar("kabuk", &kabuk_hazirligi(baska_arguman)));
    assert!(!k.kapsar("kabuk", &kabuk_hazirligi(tarif(Some("cargo"), "alt"))));
    assert!(!k.kapsar("kabuk", &kabuk_hazirligi(tarif(Some("npm"), "."))));
    let agli = KabukTarifi {
        ag: true,
        ..tarif(Some("cargo"), ".")
    };
    assert!(!k.kapsar("kabuk", &kabuk_hazirligi(agli)));
}

#[tokio::test]
async fn kisayol_sonraki_cagriyi_onaysiz_yapar_ve_onaysiz_listeye_girer() {
    let yaz = Arc::new(SahteArac::yazan("dosya_yaz"));
    let kabuk = Arc::new(SahteArac::kabuk("kabuk"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("dosya_yaz", json!({"yol": "src/a.rs"})),
        SahteYanit::arac("dosya_yaz", json!({"yol": "src/alt/b.rs"})),
        SahteYanit::arac("kabuk", json!({"yol": "."})),
        SahteYanit::metin("bitti"),
    ]);
    let mut d = surucu(&s, vec![yaz.clone(), kabuk.clone()], Kip::Siki);
    d.gonder(mesaj()).await;
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(
        ogeler[0].kisayol,
        Some(KisayolTeklifi::Dosya {
            arac: "dosya_yaz".into(),
            dizin: "src".into(),
        })
    );
    d.gonder(onayla(&ogeler[0], true)).await;

    // İkinci yazma kısayolla onaysız; kabuk onayında onaysız listede görünür.
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(yaz.cagri_sayisi(), 2);
    assert_eq!(ogeler[0].arac, "kabuk");
    let onaysiz = ogeler[0].onaysiz_degisiklikler.clone().unwrap();
    assert_eq!(onaysiz.yollar, vec!["src/alt/b.rs".to_string()]);
    assert_eq!(
        kisayollar(&d),
        Some(vec![KisayolTeklifi::Dosya {
            arac: "dosya_yaz".into(),
            dizin: "src".into(),
        }])
    );
    d.gonder(onayla(&ogeler[0], false)).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(kabuk.cagri_sayisi(), 1);
}

#[tokio::test]
async fn teklif_edilmemis_kisayol_reddedilir() {
    // Ağlı kabuk uyarı taşır: kısayol önerilmez. `kisayol: true` ile gelen
    // onay geçmez, kart açık kalır.
    let kabuk = Arc::new(SahteArac::kabuk("kabuk"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("kabuk", json!({"yol": ".", "ag": true})),
        SahteYanit::metin("bitti"),
    ]);
    let mut d = surucu(&s, vec![kabuk.clone()], Kip::Proje);
    d.gonder(mesaj()).await;
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(ogeler[0].kisayol, None);
    d.gonder(onayla(&ogeler[0], true)).await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("kısayol önerilmedi")))
        .await;
    assert_eq!(kabuk.cagri_sayisi(), 0, "onay geçmemeli");
    d.gonder(onayla(&ogeler[0], false)).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(kabuk.cagri_sayisi(), 1);
    assert_eq!(kisayollar(&d), None);
}

#[tokio::test]
async fn kabuk_kisayolu_ayni_dizinde_gecer_baska_dizinde_sorar() {
    let kabuk = Arc::new(SahteArac::kabuk("kabuk"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("kabuk", json!({"yol": "."})),
        SahteYanit::arac("kabuk", json!({"yol": "."})),
        SahteYanit::arac("kabuk", json!({"yol": "alt"})),
        SahteYanit::metin("bitti"),
    ]);
    let mut d = surucu(&s, vec![kabuk.clone()], Kip::Proje);
    d.gonder(mesaj()).await;
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(
        ogeler[0].kisayol,
        Some(KisayolTeklifi::Kabuk {
            program: "test".into(),
            dizin: ".".into(),
        })
    );
    d.gonder(onayla(&ogeler[0], true)).await;
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(kabuk.cagri_sayisi(), 2, "aynı dizinde onaysız");
    d.gonder(onayla(&ogeler[0], false)).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(kabuk.cagri_sayisi(), 3);
}

#[tokio::test]
async fn kip_degisince_kisayollar_silinir() {
    let yaz = Arc::new(SahteArac::yazan("dosya_yaz"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("dosya_yaz", json!({"yol": "src/a.rs"})),
        SahteYanit::metin("bitti"),
        SahteYanit::arac("dosya_yaz", json!({"yol": "src/b.rs"})),
        SahteYanit::metin("bitti"),
    ]);
    let mut d = surucu(&s, vec![yaz.clone()], Kip::Siki);
    d.gonder(mesaj()).await;
    let ogeler = onay_bekle(&mut d).await;
    d.gonder(onayla(&ogeler[0], true)).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(kisayollar(&d).map(|l| l.len()), Some(1));

    d.gonder(Komut::IzinKipiDegistir { kip: Kip::Siki }).await;
    d.bekle(|o| matches!(o, Olay::Kisayollar { liste } if liste.is_empty()))
        .await;
    d.gonder(mesaj()).await;
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(yaz.cagri_sayisi(), 1, "kısayol silindi; yine onay sorulur");
    d.gonder(onayla(&ogeler[0], false)).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
}

#[tokio::test]
async fn kaldirilan_kisayol_bekleyen_calismayi_durdurur() {
    // Aynı adımda kısayolla serbest bir yazma ve onay bekleyen gizli bir
    // okuma. Okumanın onayı beklenirken kısayol kaldırılır: yazma planlandığı
    // kısayolla yürütülmez.
    let yaz = Arc::new(SahteArac::yazan("dosya_yaz"));
    let oku = Arc::new(SahteArac::okuyan("oku"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("dosya_yaz", json!({"yol": "src/a.rs"})),
        SahteYanit::metin("bitti"),
        SahteYanit::araclar(vec![
            ("dosya_yaz", json!({"yol": "src/b.rs"})),
            ("oku", json!({"yol": ".env"})),
        ]),
        SahteYanit::metin("bitti"),
    ]);
    let mut d = surucu(&s, vec![yaz.clone(), oku.clone()], Kip::Siki);
    d.gonder(mesaj()).await;
    let ogeler = onay_bekle(&mut d).await;
    d.gonder(onayla(&ogeler[0], true)).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    let kimlik = d
        .gecmis
        .iter()
        .rev()
        .find_map(|z| match &z.yuk {
            Olay::Kisayollar { liste } => liste.first().map(|k| k.kimlik),
            _ => None,
        })
        .unwrap();

    d.gonder(mesaj()).await;
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(ogeler.len(), 1, "yazma kısayolla karta gelmez");
    assert_eq!(ogeler[0].arac, "oku");
    d.gonder(Komut::KisayoluKaldir { kimlik }).await;
    d.bekle(|o| matches!(o, Olay::Kisayollar { liste } if liste.is_empty()))
        .await;
    d.gonder(onayla(&ogeler[0], false)).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(oku.cagri_sayisi(), 1);
    assert_eq!(yaz.cagri_sayisi(), 1, "kaldırılan kısayolla yazılmaz");
}

#[tokio::test]
async fn kisayol_kayda_yazilmaz_kurtarmada_yok() {
    let dizin = tempfile::tempdir().unwrap();
    let yol = dizin.path().join("oturum.jsonl");
    {
        let yaz = Arc::new(SahteArac::yazan("dosya_yaz"));
        let s = SahteSaglayici::yeni(vec![
            SahteYanit::arac("dosya_yaz", json!({"yol": "src/a.rs"})),
            SahteYanit::metin("bitti"),
        ]);
        let mut a = ayarlar(Kip::Siki);
        a.kayit_yolu = Some(yol.clone());
        let mut d = test_oturumu(test_baglami(s, vec![yaz], sanal_siniflandirici()), a);
        d.gonder(mesaj()).await;
        let ogeler = onay_bekle(&mut d).await;
        d.gonder(onayla(&ogeler[0], true)).await;
        assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
        assert!(kisayollar(&d).is_some());
        d.kapat().await;
    }
    let kayit = std::fs::read_to_string(&yol).unwrap();
    assert!(!kayit.contains("Kisayol"), "{kayit}");

    let yaz = Arc::new(SahteArac::yazan("dosya_yaz"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("dosya_yaz", json!({"yol": "src/b.rs"})),
        SahteYanit::metin("bitti"),
    ]);
    let araclar: Vec<Arc<dyn Arac>> = vec![yaz.clone()];
    let (t, o, _) = devam_et(
        &yol,
        ayarlar(Kip::Siki),
        test_baglami(s, araclar, sanal_siniflandirici()),
    )
    .unwrap();
    let mut d = Surucu::yeni(t, o);
    d.gonder(mesaj()).await;
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(yaz.cagri_sayisi(), 0, "kurtarılan oturumda kısayol yok");
    d.gonder(onayla(&ogeler[0], false)).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
}
