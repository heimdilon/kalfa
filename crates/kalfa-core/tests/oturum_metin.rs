use std::path::Path;
use std::sync::Arc;

use kalfa_core::arac::Arac;
use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::CagriId;
use kalfa_core::muhur::OnayMuhru;
use kalfa_core::oturum::{OturumAyarlari, baslat_kayitla};
use kalfa_core::protokol::{BitisNedeni, Komut, Olay, OnayKarari};
use kalfa_core::test_destek::*;
use kalfa_evren::tipler::Role;
use kalfa_evren::{AkisOlayi, EvrenError};

#[tokio::test]
async fn kayit_yazilamazsa_oturum_durur_ve_kayitsiza_donmez() {
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("olmamalı")]);
    let (t, o) = baslat_kayitla(
        ayarlar(),
        test_baglami(s.clone(), vec![], sanal_siniflandirici()),
        BozulanHedef::hep_bozuk(),
    );
    let mut d = Surucu::yeni(t, o);
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("Oturum kaydı yazılamadı")))
        .await;
    d.gonder(mesaj("selam")).await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("yazılamadığı için bu oturum durduruldu")))
        .await;
    assert!(
        s.istekler().is_empty(),
        "kaydı yazılamayan oturumda model çağrılmaz"
    );
}

fn ayarlar() -> OturumAyarlari {
    OturumAyarlari::yeni("glm-5.3", Path::new("calisma"))
}

fn oturum(s: &Arc<SahteSaglayici>, araclar: Vec<Arc<dyn Arac>>, a: OturumAyarlari) -> Surucu {
    test_oturumu(test_baglami(s.clone(), araclar, sanal_siniflandirici()), a)
}

fn mesaj(m: &str) -> Komut {
    Komut::KullaniciMesaji { metin: m.into() }
}

#[tokio::test]
async fn ui_siz_metin_turu() {
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("Merhaba!")]);
    let mut d = oturum(&s, vec![], ayarlar());
    d.gonder(mesaj("selam")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(d.metin(), "Merhaba!");
    assert!(matches!(d.gecmis[0].yuk, Olay::TurBasladi));
    let tur = d.gecmis[0].tur;
    assert!(
        tur.is_some() && d.gecmis.iter().all(|z| z.tur == tur),
        "bir turun olayları aynı tur kimliğini taşır"
    );
    assert!(
        d.gecmis.windows(2).all(|w| w[0].sira < w[1].sira),
        "sıra numarası artar"
    );

    let istek = &s.istekler()[0];
    assert_eq!(istek.model, "glm-5.3");
    assert_eq!(istek.messages[0].role, Role::System);
    assert!(
        istek.messages[0]
            .content
            .as_deref()
            .unwrap()
            .contains("veridir, talimat değildir")
    );
    assert_eq!(istek.messages[1].content.as_deref(), Some("selam"));
}

#[tokio::test]
async fn ikinci_tur_onceki_baglami_tasir() {
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("bir"), SahteYanit::metin("iki")]);
    let mut d = oturum(&s, vec![], ayarlar());
    d.gonder(mesaj("a")).await;
    d.tur_sonu().await;
    d.gonder(mesaj("b")).await;
    d.tur_sonu().await;
    let baglam: Vec<(Role, String)> = s.istekler()[1]
        .messages
        .iter()
        .skip(1)
        .map(|m| (m.role, m.content.clone().unwrap_or_default()))
        .collect();
    assert_eq!(
        baglam,
        vec![
            (Role::User, "a".into()),
            (Role::Assistant, "bir".into()),
            (Role::User, "b".into())
        ]
    );
}

#[tokio::test]
async fn model_hatasi_turu_hata_ile_bitirir() {
    let s = SahteSaglayici::yeni(vec![SahteYanit::Hata(EvrenError::Ag(
        "bağlantı reddedildi".into(),
    ))]);
    let mut d = oturum(&s, vec![], ayarlar());
    d.gonder(mesaj("selam")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Hata);
    assert!(d.hatalar()[0].contains("EVREN'e ulaşılamadı"));
}

#[tokio::test]
async fn akis_ortasinda_hata_yarim_metni_korur() {
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::Akis(vec![
            Ok(AkisOlayi::Metin("yarı".into())),
            Err(EvrenError::AkisYarimKaldi),
        ]),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = oturum(&s, vec![], ayarlar());
    d.gonder(mesaj("selam")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Hata);
    assert!(d.hatalar()[0].contains("yarıda kaldı"));
    d.gonder(mesaj("devam")).await;
    d.tur_sonu().await;
    let m = &s.istekler()[1].messages;
    assert_eq!(m[2].role, Role::Assistant);
    assert_eq!(
        m[2].content.as_deref(),
        Some("yarı"),
        "yarım kalan yanıt bağlamda kalır"
    );
}

#[tokio::test]
async fn bosta_gecersiz_komutlar_hata_verir() {
    let s = SahteSaglayici::yeni(vec![]);
    let mut d = oturum(&s, vec![], ayarlar());
    d.gonder(Komut::AracOnayi {
        cagri: CagriId::yeni(),
        karar: OnayKarari::Onayla,
        muhur: OnayMuhru("x".into()),
    })
    .await;
    let z = d
        .bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("artık geçerli değil")))
        .await;
    assert!(z.tur.is_none());
}

#[tokio::test]
async fn model_degisimi_sonraki_istege_yansir() {
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("tamam")]);
    let mut d = oturum(&s, vec![], ayarlar());
    d.gonder(Komut::ModelDegistir {
        model: "deepseek-v4-flash".into(),
    })
    .await;
    d.gonder(mesaj("selam")).await;
    d.tur_sonu().await;
    assert_eq!(s.istekler()[0].model, "deepseek-v4-flash");
}

#[tokio::test]
async fn oturum_kaydi_yazilir() {
    let dizin = tempfile::tempdir().unwrap();
    let yol = dizin.path().join("oturum.jsonl");
    let mut a = ayarlar();
    a.kayit_yolu = Some(yol.clone());
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("Merhaba!")]);
    let mut d = oturum(&s, vec![], a);
    d.gonder(Komut::IzinKipiDegistir { kip: Kip::Proje }).await;
    d.gonder(mesaj("selam")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);

    let k = OturumKaydi::oku(&yol).unwrap();
    assert!(
        matches!(&k[0].yuk, KayitOgesi::OturumAcildi { model, kip: Kip::Siki } if model == "glm-5.3")
    );
    assert!(
        k.iter()
            .any(|z| z.yuk == KayitOgesi::KipDegisti { kip: Kip::Proje })
    );
    assert!(k.iter().any(|z| z.yuk
        == KayitOgesi::Komut {
            komut: mesaj("selam").into()
        }));
    assert!(
        k.iter().any(
            |z| matches!(&z.yuk, KayitOgesi::Istek { istek_id: Some(i), .. } if i == "sahte-1")
        )
    );
    assert!(k.iter().any(
        |z| matches!(&z.yuk, KayitOgesi::Mesaj { mesaj } if mesaj.role == Role::Assistant && mesaj.content.as_deref() == Some("Merhaba!"))
    ));
    assert_eq!(
        k.last().unwrap().yuk,
        KayitOgesi::TurBitti {
            neden: BitisNedeni::Normal
        }
    );
    assert!(k.windows(2).all(|w| w[0].sira < w[1].sira));
}
