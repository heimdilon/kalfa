use std::path::Path;
use std::sync::Arc;

use kalfa_core::oturum::OturumAyarlari;
use kalfa_core::protokol::{BitisNedeni, Komut, Olay};
use kalfa_core::test_destek::*;
use kalfa_evren::EvrenError;

fn oturum(s: &Arc<SahteSaglayici>) -> Surucu {
    test_oturumu(
        test_baglami(s.clone(), vec![], sanal_siniflandirici()),
        OturumAyarlari::yeni("glm-5.3", Path::new("calisma")),
    )
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
    let mut d = oturum(&s);
    d.gonder(mesaj()).await;
    sart_bekle(&mut d, 1).await;
    assert!(
        s.kabul_edilen_surumler().is_empty(),
        "kullanıcı onaylamadan kabul gönderilmez"
    );
    d.gonder(Komut::SartlariKabulEt { surum: 1 }).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(d.metin(), "tamam");
    assert_eq!(s.kabul_edilen_surumler(), vec![1]);
    assert_eq!(s.istekler().len(), 2);
}

#[tokio::test]
async fn gosterilenden_farkli_surum_kabul_edilmez() {
    let s = senaryo();
    let mut d = oturum(&s);
    d.gonder(mesaj()).await;
    sart_bekle(&mut d, 1).await;
    d.gonder(Komut::SartlariKabulEt { surum: 2 }).await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("aynı değil")))
        .await;
    sart_bekle(&mut d, 1).await;
    d.gonder(Komut::SartlariKabulEt { surum: 1 }).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(s.kabul_edilen_surumler(), vec![1]);
}

#[tokio::test]
async fn surum_okurken_degisirse_yeni_metin_gosterilir() {
    let s = senaryo();
    let mut d = oturum(&s);
    d.gonder(mesaj()).await;
    sart_bekle(&mut d, 1).await;
    s.sart_surumunu_degistir(2);
    d.gonder(Komut::SartlariKabulEt { surum: 1 }).await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("güncellendi")))
        .await;
    sart_bekle(&mut d, 2).await;
    d.gonder(Komut::SartlariKabulEt { surum: 2 }).await;
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
    let mut d = oturum(&s);
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
