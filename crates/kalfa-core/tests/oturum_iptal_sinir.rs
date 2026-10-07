use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use kalfa_core::arac::Arac;
use kalfa_core::kimlik::TurId;
use kalfa_core::oturum::OturumAyarlari;
use kalfa_core::protokol::{AracDurumu, BitisNedeni, Komut, Olay, OnayKarari, OnayOgesi};
use kalfa_core::test_destek::*;
use kalfa_evren::tipler::Role;
use kalfa_evren::{AkisOlayi, EvrenError};
use serde_json::{Value, json};

fn ayarlar() -> OturumAyarlari {
    OturumAyarlari::yeni("glm-5.3", Path::new("calisma"))
}

fn a(arac: &Arc<SahteArac>) -> Arc<dyn Arac> {
    arac.clone()
}

fn oturum(s: &Arc<SahteSaglayici>, araclar: Vec<Arc<dyn Arac>>, ayar: OturumAyarlari) -> Surucu {
    test_oturumu(
        test_baglami(s.clone(), araclar, sanal_siniflandirici()),
        ayar,
    )
}

fn mesaj(m: &str) -> Komut {
    Komut::KullaniciMesaji { metin: m.into() }
}

fn yol(y: &str) -> Value {
    json!({ "yol": y })
}

async fn tur_basladi(d: &mut Surucu) -> TurId {
    d.bekle(|o| matches!(o, Olay::TurBasladi))
        .await
        .tur
        .expect("tur kimliği")
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

fn arac_mesajlari(s: &SahteSaglayici, istek: usize) -> Vec<String> {
    s.istekler()[istek]
        .messages
        .iter()
        .filter(|m| m.role == Role::Tool)
        .map(|m| m.content.clone().unwrap_or_default())
        .collect()
}

#[tokio::test]
async fn akis_sirasinda_iptal_yarim_metni_korur() {
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::Askida(vec![AkisOlayi::Metin("yarım".into())]),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = oturum(&s, vec![], ayarlar());
    d.gonder(mesaj("uzun anlat")).await;
    let tur = tur_basladi(&mut d).await;
    d.bekle(|o| matches!(o, Olay::TokenParcasi { .. })).await;
    d.gonder(Komut::Iptal { tur }).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);
    d.gonder(mesaj("kısa anlat")).await;
    d.tur_sonu().await;
    assert_eq!(
        s.istekler()[1].messages[2].content.as_deref(),
        Some("yarım")
    );
}

#[tokio::test]
async fn arayuz_olaylari_okumazken_de_iptal_isler() {
    // Çekirdek arayüzü beklemez: binlerce olay okunmadan birikirken iptal işler.
    let s = SahteSaglayici::yeni(vec![SahteYanit::Askida(
        (0..10_000)
            .map(|i| AkisOlayi::Metin(format!("{i} ")))
            .collect(),
    )]);
    let mut d = oturum(&s, vec![], ayarlar());
    d.gonder(mesaj("çok uzun anlat")).await;
    let tur = tur_basladi(&mut d).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    d.gonder(Komut::Iptal { tur }).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);
}

#[tokio::test]
async fn iptal_et_komut_kuyrugundan_bagimsiz_iptal_eder() {
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::Askida(vec![AkisOlayi::Metin("uzun".into())]),
        SahteYanit::arac("yaz", yol("a")),
    ]);
    let mut d = oturum(&s, vec![a(&yaz)], ayarlar());
    assert_eq!(
        d.tutamac.iptal_et(),
        None,
        "süren tur yokken iptal edilecek bir şey yok"
    );

    // Akış sırasında.
    d.gonder(mesaj("anlat")).await;
    let tur = tur_basladi(&mut d).await;
    d.bekle(|o| matches!(o, Olay::TokenParcasi { .. })).await;
    assert_eq!(d.tutamac.iptal_et(), Some(tur));
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);

    // Onay beklerken.
    d.gonder(mesaj("yaz")).await;
    onay_bekle(&mut d).await;
    assert!(d.tutamac.iptal_et().is_some());
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);
    assert_eq!(yaz.cagri_sayisi(), 0);
}

#[tokio::test]
async fn onay_beklerken_iptal_muhru_gecersiz_kilar() {
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", yol("a")),
        SahteYanit::metin("peki"),
    ]);
    let mut d = oturum(&s, vec![a(&yaz)], ayarlar());
    d.gonder(mesaj("yaz")).await;
    let tur = tur_basladi(&mut d).await;
    let ogeler = onay_bekle(&mut d).await;
    d.gonder(Komut::Iptal { tur }).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);
    assert!(d.gecmis.iter().any(|z| matches!(
        z.yuk,
        Olay::AracSonucu {
            durum: AracDurumu::IptalEdildi,
            ..
        }
    )));

    d.gonder(Komut::AracOnayi {
        cagri: ogeler[0].cagri,
        karar: OnayKarari::Onayla,
        muhur: ogeler[0].muhur.clone(),
        kisayol: false,
    })
    .await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("artık geçerli değil")))
        .await;
    assert_eq!(yaz.cagri_sayisi(), 0);

    d.gonder(mesaj("devam")).await;
    d.tur_sonu().await;
    assert_eq!(
        arac_mesajlari(&s, 1),
        vec!["[çalıştırılmadı: tur iptal edildi]"]
    );
}

#[tokio::test]
async fn arac_calisirken_iptal() {
    let oku = Arc::new(SahteArac::okuyan("oku").gecikme(Duration::from_secs(30)));
    let s = SahteSaglayici::yeni(vec![SahteYanit::arac("oku", yol("a"))]);
    let mut d = oturum(&s, vec![a(&oku)], ayarlar());
    d.gonder(mesaj("oku")).await;
    let tur = tur_basladi(&mut d).await;
    d.bekle(|o| matches!(o, Olay::AracCagrisiBasladi { .. }))
        .await;
    d.gonder(Komut::Iptal { tur }).await;
    d.bekle(|o| {
        matches!(
            o,
            Olay::AracSonucu {
                durum: AracDurumu::IptalEdildi,
                ..
            }
        )
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);
}

#[tokio::test]
async fn tur_sirasinda_yeni_mesaj_reddedilir() {
    let s = SahteSaglayici::yeni(vec![SahteYanit::Askida(vec![AkisOlayi::Metin("a".into())])]);
    let mut d = oturum(&s, vec![], ayarlar());
    d.gonder(mesaj("bir")).await;
    let tur = tur_basladi(&mut d).await;
    d.bekle(|o| matches!(o, Olay::TokenParcasi { .. })).await;
    d.gonder(mesaj("iki")).await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj == "Önceki tur sürüyor; bitmesini bekleyin ya da iptal edin."))
        .await;
    d.gonder(Komut::Iptal { tur }).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);
    assert_eq!(s.istekler().len(), 1, "ikinci mesaj yeni tur başlatmadı");
}

#[tokio::test]
async fn model_turu_siniri() {
    let oku = Arc::new(SahteArac::okuyan("oku"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("oku", yol("a")),
        SahteYanit::arac("oku", yol("b")),
    ]);
    let mut ayar = ayarlar();
    ayar.sinirlar.azami_model_turu = 2;
    let mut d = oturum(&s, vec![a(&oku)], ayar);
    d.gonder(mesaj("döngü")).await;
    assert!(
        matches!(d.tur_sonu().await, BitisNedeni::Sinir { aciklama } if aciklama.contains("2 model yanıtı"))
    );
    assert_eq!(s.istekler().len(), 2);
}

#[tokio::test]
async fn arac_cagrisi_siniri_asilinca_cagrilar_yurutulmeden_kapatilir() {
    let oku = Arc::new(SahteArac::okuyan("oku"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![("oku", yol("a")), ("oku", yol("b"))]),
        SahteYanit::metin("peki"),
    ]);
    let mut ayar = ayarlar();
    ayar.sinirlar.azami_arac_cagrisi = 1;
    let mut d = oturum(&s, vec![a(&oku)], ayar);
    d.gonder(mesaj("iki dosya")).await;
    assert!(matches!(d.tur_sonu().await, BitisNedeni::Sinir { .. }));
    assert_eq!(oku.cagri_sayisi(), 0);
    d.gonder(mesaj("devam")).await;
    d.tur_sonu().await;
    let yanitlar = arac_mesajlari(&s, 1);
    assert_eq!(
        yanitlar.len(),
        2,
        "bağlam geçerli kalır: her çağrıya bir yanıt"
    );
    assert!(yanitlar.iter().all(|y| y.contains("sınırı aşıldı")));
}

#[tokio::test]
async fn ayni_cagri_uc_kez_basarisiz_olunca_dongu_durur() {
    let oku = Arc::new(SahteArac::okuyan("oku").hata("okunamadı".into()));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("oku", yol("a")),
        SahteYanit::arac("oku", yol("a")),
        SahteYanit::arac("oku", yol("a")),
    ]);
    let mut d = oturum(&s, vec![a(&oku)], ayarlar());
    d.gonder(mesaj("oku")).await;
    assert!(
        matches!(d.tur_sonu().await, BitisNedeni::Sinir { aciklama } if aciklama.contains("üst üste 3 kez"))
    );
    assert_eq!(s.istekler().len(), 3);
}

#[tokio::test]
async fn buyuk_cikti_kirpilir_ve_utf8_bozulmaz() {
    let oku = Arc::new(SahteArac::okuyan("oku").yanit("ş".repeat(1000)));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("oku", yol("a")),
        SahteYanit::metin("tamam"),
    ]);
    let mut ayar = ayarlar();
    ayar.sinirlar.azami_arac_ciktisi = 101;
    let mut d = oturum(&s, vec![a(&oku)], ayar);
    d.gonder(mesaj("oku")).await;
    d.tur_sonu().await;
    let yanit = &arac_mesajlari(&s, 1)[0];
    assert!(yanit.starts_with("<arac_sonucu kaynak=\"sahte:oku\" sinir=\""));
    assert!(yanit.contains("kırpıldı"));
    // Sınır içeriğe uygulanır; sarmalın etiketleri (~150 bayt) ona girmez.
    assert!(yanit.len() < 350, "{}", yanit.len());
}

#[tokio::test]
async fn hiz_siniri_bir_kez_yeniden_denenir() {
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::Hata(EvrenError::HizSiniri {
            tekrar_dene: Some(Duration::ZERO),
        }),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = oturum(&s, vec![], ayarlar());
    d.gonder(mesaj("selam")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(d.metin(), "tamam");
    assert_eq!(s.istekler().len(), 2);
    assert!(
        d.hatalar()
            .iter()
            .any(|h| h.contains("istek sınırına ulaşıldı"))
    );
}

#[tokio::test]
async fn ikinci_hiz_siniri_turu_bitirir() {
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::Hata(EvrenError::HizSiniri {
            tekrar_dene: Some(Duration::ZERO),
        }),
        SahteYanit::Hata(EvrenError::HizSiniri {
            tekrar_dene: Some(Duration::ZERO),
        }),
    ]);
    let mut d = oturum(&s, vec![], ayarlar());
    d.gonder(mesaj("selam")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Hata);
    assert_eq!(s.istekler().len(), 2);
}

#[tokio::test]
async fn hiz_siniri_beklemesi_iptal_edilebilir() {
    let s = SahteSaglayici::yeni(vec![SahteYanit::Hata(EvrenError::HizSiniri {
        tekrar_dene: Some(Duration::from_secs(600)),
    })]);
    let mut d = oturum(&s, vec![], ayarlar());
    d.gonder(mesaj("selam")).await;
    let tur = tur_basladi(&mut d).await;
    d.bekle(|o| matches!(o, Olay::Hata { .. })).await;
    d.gonder(Komut::Iptal { tur }).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);
}
