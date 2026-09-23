use std::path::Path;
use std::sync::Arc;

use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::oturum::OturumAyarlari;
use kalfa_core::protokol::{BitisNedeni, Komut, Olay};
use kalfa_core::sart_kapisi::{KabulSonucu, SartKapisi};
use kalfa_core::test_destek::*;
use kalfa_evren::EvrenError;

fn ayarlar() -> OturumAyarlari {
    OturumAyarlari::yeni("glm-5.3", Path::new("calisma"))
}

fn oturum(s: &Arc<SahteSaglayici>, ayar: OturumAyarlari) -> (Surucu, Arc<SartKapisi>) {
    let baglam = test_baglami(s.clone(), vec![], sanal_siniflandirici());
    let kapi = baglam.sart_kapisi.clone();
    (test_oturumu(baglam, ayar), kapi)
}

fn senaryo() -> Arc<SahteSaglayici> {
    SahteSaglayici::yeni(vec![
        SahteYanit::Hata(EvrenError::SartKabulGerekli { surum: Some(1) }),
        SahteYanit::metin("tamam"),
    ])
}

fn mesaj() -> Komut {
    Komut::KullaniciMesaji {
        metin: "selam".into(),
    }
}

async fn sart_bekle(d: &mut Surucu, beklenen: u32) {
    let z = d
        .bekle(|o| matches!(o, Olay::SartOnayiGerekiyor { .. }))
        .await;
    match z.yuk {
        Olay::SartOnayiGerekiyor { surum, metin } => {
            assert_eq!(surum, beklenen);
            assert_eq!(metin, "# Sahte kullanım şartları");
        }
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn sart_kabul_edilince_istek_yeniden_gonderilir() {
    let s = senaryo();
    let (mut d, kapi) = oturum(&s, ayarlar());
    d.gonder(mesaj()).await;
    sart_bekle(&mut d, 1).await;
    assert!(
        s.kabul_edilen_surumler().is_empty(),
        "kullanıcı onaylamadan kabul gönderilmez"
    );
    assert_eq!(kapi.kabul_et(1).await.unwrap(), KabulSonucu::Kabul);
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(d.metin(), "tamam");
    assert_eq!(s.kabul_edilen_surumler(), vec![1]);
    assert_eq!(s.istekler().len(), 2);
}

#[tokio::test]
async fn gosterilenden_farkli_surum_kabul_edilmez() {
    let s = senaryo();
    let (mut d, kapi) = oturum(&s, ayarlar());
    d.gonder(mesaj()).await;
    sart_bekle(&mut d, 1).await;
    assert!(matches!(
        kapi.kabul_et(2).await.unwrap(),
        KabulSonucu::YeniMetin(m) if m.surum == 1
    ));
    assert!(s.kabul_edilen_surumler().is_empty());
    assert_eq!(kapi.kabul_et(1).await.unwrap(), KabulSonucu::Kabul);
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(s.kabul_edilen_surumler(), vec![1]);
}

#[tokio::test]
async fn surum_okurken_degisirse_yeni_metin_gosterilir() {
    let s = senaryo();
    let (mut d, kapi) = oturum(&s, ayarlar());
    d.gonder(mesaj()).await;
    sart_bekle(&mut d, 1).await;
    s.sart_surumunu_degistir(2);
    assert!(matches!(
        kapi.kabul_et(1).await.unwrap(),
        KabulSonucu::YeniMetin(m) if m.surum == 2
    ));
    assert_eq!(kapi.kabul_et(2).await.unwrap(), KabulSonucu::Kabul);
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(
        s.kabul_edilen_surumler(),
        vec![2],
        "eski sürüm kabul edilmiş sayılmaz"
    );
}

#[tokio::test]
async fn sart_beklerken_iptal() {
    let s = senaryo();
    let (mut d, _kapi) = oturum(&s, ayarlar());
    d.gonder(mesaj()).await;
    let tur = d
        .bekle(|o| matches!(o, Olay::TurBasladi))
        .await
        .tur
        .unwrap();
    sart_bekle(&mut d, 1).await;
    d.gonder(Komut::Iptal { tur }).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);
    assert!(s.kabul_edilen_surumler().is_empty());
    assert_eq!(s.istekler().len(), 1);
}

#[tokio::test]
async fn ret_bekleyen_turu_iptal_eder() {
    let s = senaryo();
    let (mut d, kapi) = oturum(&s, ayarlar());
    d.gonder(mesaj()).await;
    sart_bekle(&mut d, 1).await;
    kapi.reddet();
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);
    assert!(s.kabul_edilen_surumler().is_empty());
}

#[tokio::test]
async fn iki_oturum_tek_kabulle_devam_eder() {
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::Hata(EvrenError::SartKabulGerekli { surum: Some(1) }),
        SahteYanit::Hata(EvrenError::SartKabulGerekli { surum: Some(1) }),
        SahteYanit::metin("bir"),
        SahteYanit::metin("iki"),
    ]);
    let baglam = test_baglami(s.clone(), vec![], sanal_siniflandirici());
    let kapi = baglam.sart_kapisi.clone();
    let mut a = test_oturumu(baglam.clone(), ayarlar());
    let mut b = test_oturumu(baglam, ayarlar());
    a.gonder(mesaj()).await;
    sart_bekle(&mut a, 1).await;
    b.gonder(mesaj()).await;
    sart_bekle(&mut b, 1).await;
    assert_eq!(kapi.kabul_et(1).await.unwrap(), KabulSonucu::Kabul);
    assert_eq!(a.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(b.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(
        s.kabul_edilen_surumler(),
        vec![1],
        "kabul EVREN'e bir kez gider"
    );
}

#[tokio::test]
async fn kabul_edilen_surum_kayda_yazilir() {
    let dizin = tempfile::tempdir().unwrap();
    let yol = dizin.path().join("oturum.jsonl");
    let mut ayar = ayarlar();
    ayar.kayit_yolu = Some(yol.clone());
    let s = senaryo();
    let (mut d, kapi) = oturum(&s, ayar);
    d.gonder(mesaj()).await;
    sart_bekle(&mut d, 1).await;
    kapi.kabul_et(1).await.unwrap();
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    assert!(
        kayitlar
            .iter()
            .any(|z| z.yuk == KayitOgesi::SartKabulEdildi { surum: 1 })
    );
}
