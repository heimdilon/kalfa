use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, Hazirlik};
use kalfa_core::izin::Kip;
use kalfa_core::oturum::OturumAyarlari;
use kalfa_core::protokol::{AracDurumu, BitisNedeni, Komut, Olay};
use kalfa_core::test_destek::*;
use kalfa_evren::tipler::Role;
use serde_json::{Value, json};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

struct BekleyenHazirlik {
    basladi: Notify,
    devam: Notify,
    arac: SahteArac,
}

#[async_trait]
impl Arac for BekleyenHazirlik {
    fn ad(&self) -> &str {
        "bekle"
    }
    fn aciklama(&self) -> &str {
        "Hazırlık iptal testi"
    }
    fn parametre_semasi(&self) -> Value {
        json!({"type": "object"})
    }
    fn yazar_mi(&self) -> bool {
        self.arac.yazar_mi()
    }
    async fn hazirla(&self, a: &Value) -> Result<Hazirlik, AracHatasi> {
        self.basladi.notify_one();
        self.devam.notified().await;
        self.arac.hazirla(a).await
    }
    async fn calistir(
        &self,
        _: &Value,
        _: &Hazirlik,
        _: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        panic!("İptal edilen hazırlık yürütülmemeli")
    }
}

async fn hazirlikta_iptal(dogrudan: bool) {
    let oku = Arc::new(SahteArac::okuyan("oku"));
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let bekle = Arc::new(BekleyenHazirlik {
        basladi: Notify::new(),
        devam: Notify::new(),
        arac: SahteArac::okuyan("bekle"),
    });
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![
            ("oku", json!({"yol": "ilk.txt"})),
            ("yaz", json!({"yol": "onay.txt"})),
            ("bekle", json!({"yol": "bekle.txt"})),
            ("oku", json!({"yol": "son.txt"})),
        ]),
        SahteYanit::metin("devam edildi"),
    ]);
    let mut d = test_oturumu(
        test_baglami(
            s.clone(),
            vec![oku.clone(), yaz.clone(), bekle.clone()],
            sanal_siniflandirici(),
        ),
        OturumAyarlari::yeni("sahte", Path::new("calisma")),
    )
    .zaman_asimi(Duration::from_secs(2));
    d.gonder(Komut::KullaniciMesaji {
        metin: "araçları hazırla".into(),
    })
    .await;
    let tur = d
        .bekle(|o| matches!(o, Olay::TurBasladi))
        .await
        .tur
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), bekle.basladi.notified())
        .await
        .unwrap();
    if dogrudan {
        assert_eq!(d.tutamac.iptal_et(), Some(tur));
    } else {
        d.gonder(Komut::Iptal { tur }).await;
    }
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);
    assert_eq!(oku.cagri_sayisi(), 0);
    assert_eq!(yaz.cagri_sayisi(), 0);
    assert!(
        !d.gecmis
            .iter()
            .any(|z| matches!(z.yuk, Olay::OnayGerekiyor { .. }))
    );
    assert_eq!(
        d.gecmis
            .iter()
            .filter(|z| matches!(
                z.yuk,
                Olay::AracSonucu {
                    durum: AracDurumu::IptalEdildi,
                    ..
                }
            ))
            .count(),
        4
    );
    d.gonder(Komut::KullaniciMesaji {
        metin: "devam".into(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    let istekler = s.istekler();
    let mesajlar = &istekler[1].messages;
    let cagrilar = mesajlar
        .iter()
        .flat_map(|m| m.tool_calls.iter().flatten())
        .collect::<Vec<_>>();
    assert_eq!(cagrilar.len(), 4);
    let yanitlar = mesajlar
        .iter()
        .filter(|m| m.role == Role::Tool)
        .collect::<Vec<_>>();
    assert_eq!(yanitlar.len(), 4);
    for cagri in cagrilar {
        assert_eq!(
            yanitlar
                .iter()
                .filter(|m| m.tool_call_id.as_deref() == Some(cagri.id.as_str()))
                .count(),
            1
        );
    }
}

#[tokio::test]
async fn hazirlik_sirasinda_komutla_iptal_butun_cagrilari_kapatir() {
    hazirlikta_iptal(false).await;
}

#[tokio::test]
async fn hazirlik_sirasinda_dogrudan_iptal_butun_cagrilari_kapatir() {
    hazirlikta_iptal(true).await;
}

async fn hazirlikta_kip_degisimi(kip: Kip) {
    let bekle = Arc::new(BekleyenHazirlik {
        basladi: Notify::new(),
        devam: Notify::new(),
        arac: SahteArac::yazan("bekle"),
    });
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("bekle", json!({"yol": "a.txt"})),
        SahteYanit::metin("yazma durduruldu"),
    ]);
    let mut ayarlar = OturumAyarlari::yeni("sahte", Path::new("calisma"));
    ayarlar.kip = kip;
    let mut d = test_oturumu(
        test_baglami(s.clone(), vec![bekle.clone()], sanal_siniflandirici()),
        ayarlar,
    )
    .zaman_asimi(Duration::from_secs(2));
    d.gonder(Komut::KullaniciMesaji {
        metin: "yaz".into(),
    })
    .await;
    tokio::time::timeout(Duration::from_secs(2), bekle.basladi.notified())
        .await
        .unwrap();
    d.gonder(Komut::IzinKipiDegistir { kip: Kip::Inceleme })
        .await;
    // Aynı kuyruktaki sonraki komutun yanıtı, kip değişiminin işlendiğini kanıtlar.
    d.gonder(Komut::KullaniciMesaji {
        metin: "ikinci mesaj".into(),
    })
    .await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("Önceki tur sürüyor")))
        .await;
    bekle.devam.notify_one();
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert!(
        !d.gecmis
            .iter()
            .any(|z| matches!(z.yuk, Olay::OnayGerekiyor { .. }))
    );
    assert!(d.gecmis.iter().any(|z| matches!(
        z.yuk,
        Olay::AracSonucu {
            durum: AracDurumu::IptalEdildi,
            ..
        }
    )));
    let istekler = s.istekler();
    assert!(istekler[1].tools.is_empty());
    let yanit = istekler[1]
        .messages
        .iter()
        .find(|m| m.role == Role::Tool)
        .unwrap();
    assert!(yanit.content.as_deref().unwrap().contains("İzin kipi"));
}

#[tokio::test]
async fn ilk_hazirlik_sirasinda_kip_degistiginde_eski_yetkiyle_yazilmaz() {
    hazirlikta_kip_degisimi(Kip::Proje).await;
}

#[tokio::test]
async fn ilk_hazirlik_sirasinda_kip_degistiginde_eski_onay_karti_yayilmaz() {
    hazirlikta_kip_degisimi(Kip::Siki).await;
}
