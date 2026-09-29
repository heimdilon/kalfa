//! Kabuk onayında oturumdaki onaysız değişiklikler ve aynı adım kuralı
//! (güvenlik analizi Bulgu 3).

use std::path::Path;
use std::sync::Arc;

use kalfa_core::arac::Arac;
use kalfa_core::izin::{AYNI_ADIM_UYARISI, Kip};
use kalfa_core::oturum::{OturumAyarlari, devam_et};
use kalfa_core::protokol::{
    AracDurumu, BitisNedeni, Komut, Olay, OnayKarari, OnayOgesi, OnaysizDegisiklikler,
};
use kalfa_core::test_destek::*;
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
        metin: "çalıştır".into(),
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

fn karar(o: &OnayOgesi, onay: bool) -> Komut {
    Komut::AracOnayi {
        cagri: o.cagri,
        karar: if onay {
            OnayKarari::Onayla
        } else {
            OnayKarari::Reddet { gerekce: None }
        },
        muhur: o.muhur.clone(),
    }
}

fn sonuclar(d: &Surucu) -> Vec<(String, AracDurumu)> {
    d.gecmis
        .iter()
        .filter_map(|z| match &z.yuk {
            Olay::AracSonucu { arac, durum, .. } => Some((arac.clone(), *durum)),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn proje_kipinde_onaysiz_yazilan_dosya_kabuk_onayinda_listelenir_ve_sifirlanir() {
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let kabuk = Arc::new(SahteArac::kabuk("kabuk"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", json!({"yol": "build.rs"})),
        SahteYanit::arac("yaz", json!({"yol": "src/a.rs"})),
        SahteYanit::arac("kabuk", json!({"yol": "."})),
        SahteYanit::metin("bitti"),
        SahteYanit::arac("kabuk", json!({"yol": "."})),
        SahteYanit::metin("yine bitti"),
    ]);
    let mut d = surucu(&s, vec![yaz.clone(), kabuk.clone()], Kip::Proje);
    d.gonder(mesaj()).await;
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(yaz.cagri_sayisi(), 2, "Proje kipinde yazmalar onaysız");
    assert_eq!(
        ogeler[0].onaysiz_degisiklikler,
        Some(OnaysizDegisiklikler {
            yollar: vec!["build.rs".into(), "src/a.rs".into()],
            komut_calisti: false,
            derleme_dosyalari: vec!["build.rs".into()],
        })
    );
    d.gonder(karar(&ogeler[0], true)).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);

    // Kabuk onaylandı: liste sıfırlandı.
    d.gonder(mesaj()).await;
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(
        ogeler[0].onaysiz_degisiklikler,
        Some(OnaysizDegisiklikler::default())
    );
    d.gonder(karar(&ogeler[0], true)).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(kabuk.cagri_sayisi(), 2);
}

#[tokio::test]
async fn serbest_kipte_onaysiz_komut_isaretlenir() {
    let kabuk = Arc::new(SahteArac::kabuk("kabuk"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("kabuk", json!({"yol": "."})),
        SahteYanit::arac("kabuk", json!({"yol": ".", "ag": true})),
        SahteYanit::metin("bitti"),
    ]);
    let mut d = surucu(&s, vec![kabuk.clone()], Kip::Serbest);
    d.gonder(mesaj()).await;
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(
        kabuk.cagri_sayisi(),
        1,
        "ağsız yalıtımlı komut onaysız çalıştı"
    );
    let onaysiz = ogeler[0].onaysiz_degisiklikler.as_ref().unwrap();
    assert!(onaysiz.komut_calisti);
    d.gonder(karar(&ogeler[0], true)).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
}

#[tokio::test]
async fn ayni_adimdaki_yazma_kabukla_birlikte_onaya_cikar_ve_calisir() {
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let kabuk = Arc::new(SahteArac::kabuk("kabuk"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![
            ("yaz", json!({"yol": "a.rs"})),
            ("kabuk", json!({"yol": "."})),
        ]),
        SahteYanit::metin("bitti"),
        SahteYanit::araclar(vec![
            ("yaz", json!({"yol": "b.rs"})),
            ("kabuk", json!({"yol": "."})),
        ]),
        SahteYanit::metin("yine bitti"),
    ]);
    let mut d = surucu(&s, vec![yaz.clone(), kabuk.clone()], Kip::Proje);
    d.gonder(mesaj()).await;
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(ogeler.len(), 2, "yazma da onaya çıktı");
    let yazma = ogeler.iter().find(|o| o.arac == "yaz").unwrap();
    assert_eq!(yazma.uyarilar, vec![AYNI_ADIM_UYARISI]);
    assert_eq!(yaz.cagri_sayisi(), 0, "onaydan önce yazılmadı");
    for o in &ogeler {
        d.gonder(karar(o, true)).await;
    }
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(yaz.cagri_sayisi(), 1);
    assert_eq!(kabuk.cagri_sayisi(), 1);
    assert!(
        sonuclar(&d)
            .iter()
            .all(|(_, durum)| *durum == AracDurumu::Basarili),
        "{:?}",
        sonuclar(&d)
    );

    // Yazma reddedilse de kabuk onaylanabilir.
    d.gonder(mesaj()).await;
    let ogeler = onay_bekle(&mut d).await;
    for o in &ogeler {
        d.gonder(karar(o, o.arac == "kabuk")).await;
    }
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(yaz.cagri_sayisi(), 1);
    assert_eq!(kabuk.cagri_sayisi(), 2);
}

#[tokio::test]
async fn kurtarilan_oturumda_onaysiz_komut_varsayilir() {
    let dizin = tempfile::tempdir().unwrap();
    let yol = dizin.path().join("oturum.jsonl");
    // Kayıtlı bir oturum: bir tur konuşma.
    {
        let s = SahteSaglayici::yeni(vec![SahteYanit::metin("merhaba")]);
        let mut a = ayarlar(Kip::Siki);
        a.kayit_yolu = Some(yol.clone());
        let mut d = test_oturumu(test_baglami(s, vec![], sanal_siniflandirici()), a);
        d.gonder(mesaj()).await;
        assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
        d.kapat().await;
    }
    let kabuk = Arc::new(SahteArac::kabuk("kabuk"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("kabuk", json!({"yol": "."})),
        SahteYanit::metin("bitti"),
    ]);
    let araclar: Vec<Arc<dyn Arac>> = vec![kabuk];
    let (t, o, _) = devam_et(
        &yol,
        ayarlar(Kip::Siki),
        test_baglami(s, araclar, sanal_siniflandirici()),
    )
    .unwrap();
    let mut d = Surucu::yeni(t, o);
    d.gonder(mesaj()).await;
    let ogeler = onay_bekle(&mut d).await;
    assert!(
        ogeler[0]
            .onaysiz_degisiklikler
            .as_ref()
            .is_some_and(|o| o.komut_calisti),
        "önceki süreçte onaysız ne olduğu bilinmez"
    );
    d.gonder(karar(&ogeler[0], true)).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
}

/// Aşama 3 tasarımı §5.4: onaysız çalışan kabuğun değiştirdiği dosyalar tam
/// biliniyorsa sonraki kabuk onayında listelenir; "onaysız bir komut
/// çalıştı" notu gerekmez. Bilinmiyorsa not kalır.
#[tokio::test]
async fn onaysiz_kabugun_degistirdigi_dosyalar_sonraki_kabuk_onayinda_listelenir() {
    let bilinen = Arc::new(SahteArac::kabuk("kabuk").degistirdigi(&["build.rs", "src/a.rs"]));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("kabuk", json!({"yol": "."})),
        SahteYanit::arac("kabuk", json!({"yol": ".", "ag": true})),
        SahteYanit::metin("bitti"),
    ]);
    let mut d = surucu(&s, vec![bilinen.clone()], Kip::Serbest);
    d.gonder(mesaj()).await;
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(bilinen.cagri_sayisi(), 1);
    assert_eq!(
        ogeler[0].onaysiz_degisiklikler,
        Some(OnaysizDegisiklikler {
            yollar: vec!["build.rs".into(), "src/a.rs".into()],
            komut_calisti: false,
            derleme_dosyalari: vec!["build.rs".into()],
        })
    );
    d.gonder(karar(&ogeler[0], true)).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    // Kayıtta ve tur özetinde de görünür.
    let Olay::TurOzeti { ozet } = d.bekle(|o| matches!(o, Olay::TurOzeti { .. })).await.yuk else {
        unreachable!()
    };
    let ilk = ozet.komutlar[0].degisiklikler.as_ref().unwrap();
    assert_eq!(ilk.toplam, 2);
    assert!(ilk.tam_mi());
}
