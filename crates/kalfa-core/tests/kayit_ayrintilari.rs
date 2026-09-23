use std::path::{Path, PathBuf};

use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::OturumId;
use kalfa_core::kurtarma::kurtar;
use kalfa_core::oturum::{OturumAyarlari, baslat_kimlikle};
use kalfa_core::protokol::{BitisNedeni, Komut, Olay, Zarf};
use kalfa_core::test_destek::*;
use kalfa_evren::AkisOlayi;
use kalfa_evren::tipler::{CompletionTokensDetails, Usage};

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
async fn acilis_kaydi_calisma_alanini_tasir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![]);
    let surucu = test_oturumu(test_baglami(s, vec![], sanal_siniflandirici()), ayar);
    surucu.kapat().await;
    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    assert!(matches!(
        &kayitlar[0].yuk,
        KayitOgesi::OturumAcildi { calisma_alani: Some(a), .. } if a == Path::new("calisma")
    ));
    assert_eq!(
        kurtar(&kayitlar).unwrap().calisma_alani,
        Some(PathBuf::from("calisma"))
    );
}

#[test]
fn calisma_alanisiz_eski_acilis_kaydi_okunur() {
    let satir = r#"{"sema":1,"oturum":"6f1d0a4e-2b8c-4c1e-9a55-0d6b1f3e2a10","tur":null,"sira":1,"yuk":{"tip":"OturumAcildi","model":"glm-5.3","kip":"Siki"}}"#;
    let zarf: Zarf<KayitOgesi> = serde_json::from_str(satir).unwrap();
    assert!(matches!(
        zarf.yuk,
        KayitOgesi::OturumAcildi {
            calisma_alani: None,
            ..
        }
    ));
    assert_eq!(
        kurtar(std::slice::from_ref(&zarf)).unwrap().calisma_alani,
        None
    );
}

#[test]
fn yonlendirilen_modelsiz_eski_istek_kaydi_okunur() {
    let satir = r#"{"sema":1,"oturum":"6f1d0a4e-2b8c-4c1e-9a55-0d6b1f3e2a10","tur":null,"sira":2,"yuk":{"tip":"Istek","amac":"ana","istek_id":null,"kredi":null}}"#;
    let zarf: Zarf<KayitOgesi> = serde_json::from_str(satir).unwrap();
    assert!(matches!(
        zarf.yuk,
        KayitOgesi::Istek {
            yonlendirilen_model: None,
            ..
        }
    ));
}

#[tokio::test]
async fn yonlendirilen_model_olay_olarak_yayilir_ve_kayda_yazilir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("tamam")]);
    let mut surucu = test_oturumu(test_baglami(s, vec![], sanal_siniflandirici()), ayar);
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert!(surucu.gecmis.iter().any(|z| matches!(
        &z.yuk,
        Olay::IstekBilgisi { yonlendirilen_model: Some(m), .. } if m == "glm-5.3"
    )));
    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    assert!(kayitlar.iter().any(|z| matches!(
        &z.yuk,
        KayitOgesi::Istek { yonlendirilen_model: Some(m), .. } if m == "glm-5.3"
    )));
}

#[tokio::test]
async fn token_kullanimi_kayda_yazilir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let kullanim = Usage {
        prompt_tokens: 10,
        completion_tokens: 5,
        total_tokens: 15,
        completion_tokens_details: Some(CompletionTokensDetails {
            reasoning_tokens: 2,
        }),
        evren: None,
    };
    let s = SahteSaglayici::yeni(vec![SahteYanit::Akis(vec![
        Ok(AkisOlayi::Metin("tamam".into())),
        Ok(AkisOlayi::Kullanim(kullanim)),
        Ok(AkisOlayi::Bitti {
            neden: "stop".into(),
        }),
    ])]);
    let mut surucu = test_oturumu(test_baglami(s, vec![], sanal_siniflandirici()), ayar);
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert!(surucu.gecmis.iter().any(|z| matches!(
        &z.yuk,
        Olay::TokenKullanimi {
            istem: 10,
            tamamlama: 5,
            dusunce: 2
        }
    )));
    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    assert!(kayitlar.iter().any(|z| z.yuk
        == KayitOgesi::Kullanim {
            istem: 10,
            tamamlama: 5,
            dusunce: 2
        }));
}

#[tokio::test]
async fn verilen_kimlikle_baslar() {
    let d = tempfile::tempdir().unwrap();
    let kimlik = OturumId::yeni();
    let yol = d.path().join(format!("{kimlik}.jsonl"));
    let mut ayar = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    ayar.kayit_yolu = Some(yol.clone());
    let s = SahteSaglayici::yeni(vec![]);
    let (tutamac, alici) = baslat_kimlikle(
        kimlik,
        ayar,
        test_baglami(s, vec![], sanal_siniflandirici()),
    )
    .unwrap();
    assert_eq!(tutamac.oturum_id(), kimlik);
    drop(alici);
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tutamac.kapanmasini_bekle(),
    )
    .await
    .expect("oturum kapanmalı");
    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    assert_eq!(kayitlar[0].oturum, kimlik);
}
