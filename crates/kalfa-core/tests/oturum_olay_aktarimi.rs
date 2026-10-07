use std::path::Path;
use std::time::Duration;

use kalfa_core::oturum::{OlayAktarimHatasi, OlayButcesi, OturumAyarlari, baslat};
use kalfa_core::protokol::{AktarimDurmaNedeni, Komut, Olay, OnayKarari};
use kalfa_core::test_destek::{SahteSaglayici, SahteYanit, sanal_siniflandirici, test_baglami};
use kalfa_evren::AkisOlayi;

fn ayarlar() -> OturumAyarlari {
    OturumAyarlari::yeni("glm-5.3", Path::new("calisma"))
}

fn mesaj() -> Komut {
    Komut::KullaniciMesaji {
        metin: "uzun akış".into(),
    }
}

#[tokio::test]
async fn yavas_alici_olay_sayisi_butcesini_asamaz() {
    let saglayici = SahteSaglayici::yeni(vec![SahteYanit::Askida(
        (0..100)
            .map(|i| AkisOlayi::Metin(format!("parça-{i} ş\\\"\n")))
            .collect(),
    )]);
    let mut ayar = ayarlar();
    ayar.olay_butcesi = OlayButcesi::yeni(8, 128 * 1024).expect("test bütçesi");
    let (tutamac, mut olaylar) = baslat(
        ayar.clone(),
        test_baglami(saglayici, vec![], sanal_siniflandirici()),
    )
    .expect("oturum başlatılmalı");

    tutamac.gonder(mesaj()).await.expect("komut gönderilmeli");
    let ilk = tokio::time::timeout(Duration::from_secs(2), olaylar.recv())
        .await
        .expect("ilk olay zaman aşımına uğramamalı")
        .expect("ilk olay gelmeli");
    assert!(matches!(ilk.yuk, Olay::TurBasladi));

    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if olaylar.transport_hatasi().is_some() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("yavaş alıcı bütçe hatasına ulaşmalı");

    assert!(olaylar.bekleyen_olay_sayisi() <= ayar.olay_butcesi.azami_olay());
    assert!(olaylar.bekleyen_bayt() <= ayar.olay_butcesi.azami_bayt());
    assert!(matches!(
        olaylar.transport_hatasi(),
        Some(OlayAktarimHatasi::OlaySayisiAsildi { .. })
    ));

    let bekleyen_bayt = olaylar.bekleyen_bayt();
    let mut kabul_edilen = vec![ilk];
    let mut terminal_goruldu = false;
    loop {
        let zarf = tokio::time::timeout(Duration::from_secs(2), olaylar.recv())
            .await
            .expect("taşıma hatasından sonra kanal kapanmalı");
        let Some(zarf) = zarf else { break };
        if matches!(
            zarf.yuk,
            Olay::AktarimDurdu {
                neden: AktarimDurmaNedeni::TuketiciYetismedi
            }
        ) {
            terminal_goruldu = true;
        }
        kabul_edilen.push(zarf);
    }
    assert!(
        terminal_goruldu,
        "taşıma hatası terminal olayla görünür olmalı"
    );
    assert_eq!(olaylar.bekleyen_olay_sayisi(), 0);
    assert_eq!(
        bekleyen_bayt,
        kabul_edilen[1..]
            .iter()
            .map(|zarf| serde_json::to_vec(zarf)
                .expect("zarf serileştirilmeli")
                .len())
            .sum::<usize>()
    );
    assert!(kabul_edilen.windows(2).all(|p| p[0].sira < p[1].sira));
}

#[tokio::test]
async fn yavas_alici_serilestirilmis_bayt_butcesini_asamaz() {
    let saglayici = SahteSaglayici::yeni(vec![SahteYanit::Askida(
        (0..100)
            .map(|i| AkisOlayi::Metin(format!("parça-{i} ş\\\"\n")))
            .collect(),
    )]);
    let mut ayar = ayarlar();
    ayar.olay_butcesi = OlayButcesi::yeni(128, 2 * 1024).expect("test bütçesi");
    let (tutamac, mut olaylar) = baslat(
        ayar.clone(),
        test_baglami(saglayici, vec![], sanal_siniflandirici()),
    )
    .expect("oturum başlatılmalı");
    tutamac.gonder(mesaj()).await.expect("komut gönderilmeli");
    let _ = tokio::time::timeout(Duration::from_secs(2), olaylar.recv())
        .await
        .expect("ilk olay zaman aşımına uğramamalı")
        .expect("ilk olay gelmeli");
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if olaylar.transport_hatasi().is_some() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("bayt bütçesi hatasına ulaşmalı");
    assert!(matches!(
        olaylar.transport_hatasi(),
        Some(OlayAktarimHatasi::BaytButcesiAsildi { .. })
    ));
    assert!(olaylar.bekleyen_olay_sayisi() <= ayar.olay_butcesi.azami_olay());
    assert!(olaylar.bekleyen_bayt() <= ayar.olay_butcesi.azami_bayt());
}

#[tokio::test]
async fn buyuk_onay_olayi_yazma_baslamadan_turu_kapatir() {
    let yaz = std::sync::Arc::new(
        kalfa_core::test_destek::SahteArac::yazan("yaz").yanit("yazıldı".into()),
    );
    let saglayici = SahteSaglayici::yeni(vec![SahteYanit::arac(
        "yaz",
        serde_json::json!({
            "yol": "a"
        }),
    )]);
    let mut ayar = ayarlar();
    ayar.olay_butcesi = OlayButcesi::yeni(8, 900).expect("test bütçesi");
    let (tutamac, mut olaylar) = baslat(
        ayar,
        test_baglami(saglayici, vec![yaz.clone()], sanal_siniflandirici()),
    )
    .expect("oturum başlatılmalı");

    tutamac.gonder(mesaj()).await.expect("komut gönderilmeli");
    let mut terminal_goruldu = false;
    for _ in 0..20 {
        let Some(zarf) = tokio::time::timeout(Duration::from_secs(2), olaylar.recv())
            .await
            .expect("olay zaman aşımına uğramamalı")
        else {
            break;
        };
        if matches!(zarf.yuk, Olay::AktarimDurdu { .. }) {
            terminal_goruldu = true;
            break;
        }
    }
    assert!(
        terminal_goruldu,
        "aşırı büyük onay terminal hatayla görünür olmalı"
    );
    assert_eq!(
        yaz.cagri_sayisi(),
        0,
        "gösterilemeyen onay yazmayı başlatmamalı"
    );
    assert!(matches!(
        olaylar.transport_hatasi(),
        Some(OlayAktarimHatasi::BaytButcesiAsildi { .. })
    ));
}

#[tokio::test]
async fn alici_birakilinca_sessiz_saglayici_yeni_olay_gerekmeden_iptal_edilir() {
    let saglayici = SahteSaglayici::yeni(vec![SahteYanit::Askida(vec![])]);
    let (tutamac, mut olaylar) = baslat(
        ayarlar(),
        test_baglami(saglayici.clone(), vec![], sanal_siniflandirici()),
    )
    .expect("oturum başlatılmalı");
    tutamac.gonder(mesaj()).await.expect("komut gönderilmeli");
    let _ = tokio::time::timeout(Duration::from_secs(2), olaylar.recv())
        .await
        .expect("tur başlangıcı gelmeli");
    drop(olaylar);

    tokio::time::timeout(Duration::from_secs(2), async {
        while tutamac.aktif_tur().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("alıcı kapanınca aktif tur yeni olayı beklemeden bitmeli");
    assert_eq!(
        saglayici.istekler().len(),
        1,
        "sağlayıcı çağrısı başlatılmış olmalı"
    );
}

#[tokio::test]
async fn kabul_edilen_kritik_olaylar_sira_ve_icerikle_korunur() {
    let yaz = std::sync::Arc::new(kalfa_core::test_destek::SahteArac::yazan("yaz"));
    let saglayici = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", serde_json::json!({ "yol": "a" })),
        SahteYanit::metin("tamam"),
    ]);
    let mut ayar = ayarlar();
    ayar.olay_butcesi = OlayButcesi::yeni(32, 64 * 1024).expect("test bütçesi");
    let (tutamac, mut olaylar) = baslat(
        ayar,
        test_baglami(saglayici, vec![yaz.clone()], sanal_siniflandirici()),
    )
    .expect("oturum başlatılmalı");
    tutamac.gonder(mesaj()).await.expect("komut gönderilmeli");

    let mut gelen = Vec::new();
    loop {
        let zarf = tokio::time::timeout(Duration::from_secs(2), olaylar.recv())
            .await
            .expect("olay beklerken zaman aşımı")
            .expect("normal turda kanal kapanmamalı");
        let tamamlandi = matches!(zarf.yuk, Olay::Tamamlandi { .. });
        let onay = match &zarf.yuk {
            Olay::OnayGerekiyor { ogeler, .. } => ogeler.first().cloned(),
            _ => None,
        };
        gelen.push(zarf);
        if let Some(onay) = onay {
            tutamac
                .gonder(Komut::AracOnayi {
                    cagri: onay.cagri,
                    karar: OnayKarari::Onayla,
                    muhur: onay.muhur,
                    kisayol: false,
                })
                .await
                .expect("onay gönderilmeli");
        }
        if tamamlandi {
            break;
        }
    }
    assert!(gelen.iter().any(|z| matches!(z.yuk, Olay::TurBasladi)));
    assert!(
        gelen
            .iter()
            .any(|z| matches!(z.yuk, Olay::OnayGerekiyor { .. }))
    );
    assert!(
        gelen
            .iter()
            .any(|z| matches!(z.yuk, Olay::AracCagrisiBasladi { .. }))
    );
    assert!(
        gelen
            .iter()
            .any(|z| matches!(z.yuk, Olay::AracSonucu { .. }))
    );
    assert!(
        gelen
            .iter()
            .any(|z| matches!(z.yuk, Olay::Tamamlandi { .. }))
    );
    assert!(gelen.windows(2).all(|p| p[0].sira < p[1].sira));
    assert!(olaylar.transport_hatasi().is_none());
    assert_eq!(yaz.cagri_sayisi(), 1);
}

#[tokio::test]
async fn alici_kapatilinca_bos_oturum_yeni_tur_baslatmaz() {
    let saglayici = SahteSaglayici::yeni(vec![SahteYanit::metin("tamam")]);
    let (tutamac, mut olaylar) = baslat(
        ayarlar(),
        test_baglami(saglayici, vec![], sanal_siniflandirici()),
    )
    .expect("oturum başlatılmalı");
    tutamac.gonder(mesaj()).await.expect("komut gönderilmeli");
    loop {
        let zarf = tokio::time::timeout(Duration::from_secs(2), olaylar.recv())
            .await
            .expect("normal tur tamamlanmalı")
            .expect("olay gelmeli");
        if matches!(zarf.yuk, Olay::Tamamlandi { .. }) {
            break;
        }
    }
    olaylar.close();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if tutamac.gonder(mesaj()).await.is_err() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("alıcı kapanınca komut kanalı da kapanmalı");
}

#[test]
fn olay_butcesi_tokio_kanal_ust_sinirini_reddeder() {
    assert!(matches!(
        OlayButcesi::yeni(tokio::sync::Semaphore::MAX_PERMITS + 1, usize::MAX),
        Err(kalfa_core::oturum::OlayButcesiHatasi::FazlaOlay)
    ));
}
