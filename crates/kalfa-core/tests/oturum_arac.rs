use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use kalfa_core::arac::Arac;
use kalfa_core::gorunum::GORUNMEZ_UYARISI;
use kalfa_core::izin::{GIZLI_UYARISI, KORUMALI_UYARISI, Kip};
use kalfa_core::muhur::OnayMuhru;
use kalfa_core::oturum::{OturumAyarlari, baslat_kayitla};
use kalfa_core::protokol::{AracDurumu, BitisNedeni, Komut, Olay, OnayKarari, OnayOgesi};
use kalfa_core::test_destek::*;
use kalfa_evren::tipler::{ChatMessage, Role};
use serde_json::{Value, json};

#[tokio::test]
async fn ayni_calisma_alanindaki_iki_oturum_ayni_dosyaya_seri_yazar() {
    let yaz = Arc::new(SahteArac::yazan("yaz").gecikme(Duration::from_millis(200)));
    let s1 = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", yol("a.txt")),
        SahteYanit::metin("bir"),
    ]);
    let s2 = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", yol("a.txt")),
        SahteYanit::metin("iki"),
    ]);
    let b1 = test_baglami(s1.clone(), vec![a(&yaz)], sanal_siniflandirici());
    let mut b2 = b1.clone();
    b2.saglayici = s2.clone();
    let mut d1 = test_oturumu(b1, ayarlar(Kip::Proje));
    let mut d2 = test_oturumu(b2, ayarlar(Kip::Proje));
    d1.gonder(mesaj("yaz")).await;
    d2.gonder(mesaj("yaz")).await;
    assert_eq!(d1.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(d2.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(yaz.cagri_sayisi(), 2);
    assert_eq!(
        yaz.azami_eszamanli(),
        1,
        "kilitler çalışma alanı düzeyinde ortak"
    );
}

#[tokio::test]
async fn kip_degisince_bekleyen_onaylar_gecersizlesir() {
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", yol("b.txt")),
        SahteYanit::metin("peki"),
    ]);
    let mut d = oturum(&s, vec![a(&yaz)], ayarlar(Kip::Siki));
    d.gonder(mesaj("yaz")).await;
    let ogeler = onay_bekle(&mut d).await;
    d.gonder(Komut::IzinKipiDegistir { kip: Kip::Inceleme })
        .await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("İzin kipi değişti")))
        .await;
    assert_eq!(
        d.tur_sonu().await,
        BitisNedeni::Normal,
        "tur sürer; model yeni kipte yeniden planlayabilir"
    );
    assert!(arac_mesajlari(&s.istekler()[1].messages)[0].contains("izin kipi değişti"));

    d.gonder(onayla(&ogeler[0])).await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("artık geçerli değil")))
        .await;
    assert_eq!(yaz.cagri_sayisi(), 0, "eski kipte verilen onayla yazılmaz");
}

#[tokio::test]
async fn kip_degisimi_kilit_bekleyen_yurutmeyi_durdurur() {
    let yaz = Arc::new(SahteArac::yazan("yaz").gecikme(Duration::from_millis(300)));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![("yaz", yol("a.txt")), ("yaz", yol("a.txt"))]),
        SahteYanit::metin("peki"),
    ]);
    let mut d = oturum(&s, vec![a(&yaz)], ayarlar(Kip::Proje));
    d.gonder(mesaj("iki kez yaz")).await;
    d.bekle(|o| matches!(o, Olay::AracCagrisiBasladi { .. }))
        .await;
    d.bekle(|o| matches!(o, Olay::AracCagrisiBasladi { .. }))
        .await;
    d.gonder(Komut::IzinKipiDegistir { kip: Kip::Inceleme })
        .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert!(
        yaz.cagri_sayisi() < 2,
        "kilit bekleyen yazma, kip değişince yürütülmez"
    );
    assert!(
        arac_mesajlari(&s.istekler()[1].messages)
            .iter()
            .any(|m| m.contains("İzin kipi veya hedefin izin koşulları değişti"))
    );
}

#[tokio::test]
async fn baslangic_kaydi_yazilamazsa_yan_etkili_arac_calismaz() {
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", yol("b.txt")),
        SahteYanit::metin("olmamalı"),
    ]);
    // Kalıcı yazımlar: istek başlangıcı ve istek bitişi geçer; araç "başladı"
    // kaydı düşer.
    let (t, o) = baslat_kayitla(
        ayarlar(Kip::Proje),
        test_baglami(s.clone(), vec![a(&yaz)], sanal_siniflandirici()),
        BozulanHedef::kalicidan_sonra_bozuk(2),
    )
    .unwrap();
    let mut d = Surucu::yeni(t, o);
    d.gonder(mesaj("yaz")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Hata);
    assert!(
        d.hatalar()
            .iter()
            .any(|h| h.contains("Oturum kaydı yazılamadı"))
    );
    assert_eq!(
        yaz.cagri_sayisi(),
        0,
        "başladı kaydı diske ulaşmadan yan etkili araç çalışmaz"
    );
    assert_eq!(s.istekler().len(), 1, "kayıt bozulunca tur ilerlemez");
    d.gonder(mesaj("tekrar")).await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("durduruldu")))
        .await;
}

#[tokio::test]
async fn grubun_ikinci_kaydi_bozulursa_ilk_arac_da_baslamaz() {
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let s = SahteSaglayici::yeni(vec![SahteYanit::araclar(vec![
        ("yaz", yol("a.txt")),
        ("yaz", yol("b.txt")),
    ])]);
    // Kalıcı yazımlar: istek başlangıcı, istek bitişi ve birinci araç kaydı
    // geçer; ikinci araç kaydı düşer.
    let (t, o) = baslat_kayitla(
        ayarlar(Kip::Proje),
        test_baglami(s.clone(), vec![a(&yaz)], sanal_siniflandirici()),
        BozulanHedef::kalicidan_sonra_bozuk(3),
    )
    .unwrap();
    let mut d = Surucu::yeni(t, o);
    d.gonder(mesaj("iki dosyaya yaz")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Hata);
    assert_eq!(yaz.cagri_sayisi(), 0);
    assert_eq!(sonuc_durumlari(&d), vec![AracDurumu::IptalEdildi; 2]);
    assert_eq!(s.istekler().len(), 1);
}

#[tokio::test]
async fn sonuc_kaydi_bozulursa_kilit_bekleyen_arac_baslamaz() {
    use kalfa_core::kayit::{KayitHedefi, KayitOgesi, OturumKaydi};
    use kalfa_core::protokol::Zarf;

    struct SonuctaBoz;
    impl KayitHedefi for SonuctaBoz {
        fn ekle(&mut self, satir: &[u8]) -> std::io::Result<()> {
            let z: Zarf<KayitOgesi> = serde_json::from_slice(satir).unwrap();
            if matches!(z.yuk, KayitOgesi::AracBitti { .. }) {
                Err(std::io::Error::other("disk dolu"))
            } else {
                Ok(())
            }
        }

        fn kalici_yap(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let s = SahteSaglayici::yeni(vec![SahteYanit::araclar(vec![
        ("yaz", yol("a.txt")),
        ("yaz", yol("a.txt")),
    ])]);
    let (t, o) = baslat_kayitla(
        ayarlar(Kip::Proje),
        test_baglami(s, vec![a(&yaz)], sanal_siniflandirici()),
        OturumKaydi::hedefle(Box::new(SonuctaBoz)),
    )
    .unwrap();
    let mut d = Surucu::yeni(t, o);
    d.gonder(mesaj("iki kez yaz")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Hata);
    assert_eq!(
        yaz.cagri_sayisi(),
        1,
        "ilk yan etki geri alınmaz; bekleyen ikinci işlem başlamaz"
    );
}

#[tokio::test]
async fn ikinci_hazirlik_beklerken_kip_degisirse_calistirilmaz() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use kalfa_core::arac::{AracCiktisi, AracHatasi, Hazirlik};
    use tokio::sync::Notify;
    use tokio_util::sync::CancellationToken;

    struct Bekleyen {
        arac: Arc<SahteArac>,
        sayac: AtomicUsize,
        basladi: Notify,
        devam: Notify,
    }

    #[async_trait::async_trait]
    impl Arac for Bekleyen {
        fn ad(&self) -> &str {
            self.arac.ad()
        }
        fn aciklama(&self) -> &str {
            self.arac.aciklama()
        }
        fn parametre_semasi(&self) -> Value {
            self.arac.parametre_semasi()
        }
        fn yazar_mi(&self) -> bool {
            true
        }
        async fn hazirla(&self, a: &Value) -> Result<Hazirlik, AracHatasi> {
            if self.sayac.fetch_add(1, Ordering::SeqCst) == 1 {
                self.basladi.notify_one();
                self.devam.notified().await;
            }
            self.arac.hazirla(a).await
        }
        async fn calistir(
            &self,
            a: &Value,
            h: &Hazirlik,
            iptal: CancellationToken,
        ) -> Result<AracCiktisi, AracHatasi> {
            self.arac.calistir(a, h, iptal).await
        }
    }

    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let bekleyen = Arc::new(Bekleyen {
        arac: yaz.clone(),
        sayac: AtomicUsize::new(0),
        basladi: Notify::new(),
        devam: Notify::new(),
    });
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", yol("a.txt")),
        SahteYanit::metin("peki"),
    ]);
    let mut d = oturum(&s, vec![bekleyen.clone()], ayarlar(Kip::Proje));
    d.gonder(mesaj("yaz")).await;
    tokio::time::timeout(Duration::from_secs(5), bekleyen.basladi.notified())
        .await
        .unwrap();
    d.gonder(Komut::IzinKipiDegistir { kip: Kip::Inceleme })
        .await;
    // FIFO komut kuyruğundaki sonraki komutun yanıtı, kipin işlendiğine bariyer.
    d.gonder(Komut::AracOnayi {
        cagri: kalfa_core::kimlik::CagriId::yeni(),
        karar: OnayKarari::Onayla,
        muhur: OnayMuhru("eski".into()),
        kisayol: false,
    })
    .await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("artık geçerli değil")))
        .await;
    bekleyen.devam.notify_one();
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(yaz.cagri_sayisi(), 0);
    assert_eq!(sonuc_durumlari(&d), vec![AracDurumu::IptalEdildi]);
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

fn ayarlar(kip: Kip) -> OturumAyarlari {
    let mut a = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    a.kip = kip;
    a
}

fn mesaj(m: &str) -> Komut {
    Komut::KullaniciMesaji { metin: m.into() }
}

fn yol(y: &str) -> Value {
    json!({ "yol": y })
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

fn onayla(o: &OnayOgesi) -> Komut {
    Komut::AracOnayi {
        cagri: o.cagri,
        karar: OnayKarari::Onayla,
        muhur: o.muhur.clone(),
        kisayol: false,
    }
}

fn arac_mesajlari(m: &[ChatMessage]) -> Vec<String> {
    m.iter()
        .filter(|x| x.role == Role::Tool)
        .map(|x| x.content.clone().unwrap_or_default())
        .collect()
}

fn sonuc_durumlari(d: &Surucu) -> Vec<AracDurumu> {
    d.gecmis
        .iter()
        .filter_map(|z| match &z.yuk {
            Olay::AracSonucu { durum, .. } => Some(*durum),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn serbest_okuma_onaysiz_calisir_ve_kaynakla_modele_doner() {
    let oku = Arc::new(SahteArac::okuyan("oku").yanit("dosya içeriği".into()));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("oku", yol("a.txt")),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = oturum(&s, vec![a(&oku)], ayarlar(Kip::Siki));
    d.gonder(mesaj("oku")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(sonuc_durumlari(&d), vec![AracDurumu::Basarili]);
    assert!(
        !d.gecmis
            .iter()
            .any(|z| matches!(z.yuk, Olay::OnayGerekiyor { .. }))
    );
    assert!(
        d.gecmis
            .iter()
            .any(|z| matches!(z.yuk, Olay::AracCagrisiBasladi { .. }))
    );
    let m = &s.istekler()[1].messages;
    let arac_mesaji = m.iter().find(|x| x.role == Role::Tool).unwrap();
    assert!(
        arac_mesaji
            .tool_call_id
            .as_deref()
            .unwrap()
            .starts_with("model-cagri-")
    );
    let icerik = arac_mesaji.content.as_deref().unwrap();
    let satirlar: Vec<&str> = icerik.lines().collect();
    assert!(
        satirlar[0].starts_with("<arac_sonucu kaynak=\"sahte:oku\" sinir=\""),
        "{icerik}"
    );
    assert_eq!(satirlar[1], "dosya içeriği");
    assert!(
        satirlar[2].starts_with("</arac_sonucu sinir=\""),
        "{icerik}"
    );
    assert_eq!(oku.cagri_sayisi(), 1);
}

#[tokio::test]
async fn siki_kipte_yazma_muhurle_onaylanir() {
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", yol("b.txt")),
        SahteYanit::metin("bitti"),
    ]);
    let mut d = oturum(&s, vec![a(&yaz)], ayarlar(Kip::Siki));
    d.gonder(mesaj("yaz")).await;
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(ogeler.len(), 1);
    assert_eq!(ogeler[0].arac, "yaz");
    assert_eq!(ogeler[0].hedefler, vec!["b.txt"]);
    assert_eq!(ogeler[0].diff.as_deref(), Some("+sahte yaz"));
    assert_eq!(ogeler[0].aciklama, "sahte yaz `b.txt`");
    assert!(ogeler[0].uyarilar.is_empty());
    assert_eq!(yaz.cagri_sayisi(), 0, "onaydan önce çalışmaz");
    d.gonder(onayla(&ogeler[0])).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(sonuc_durumlari(&d), vec![AracDurumu::Basarili]);
    assert_eq!(yaz.cagri_sayisi(), 1);
}

#[tokio::test]
async fn yanlis_muhur_reddedilir_dogrusu_kabul_edilir() {
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", yol("b.txt")),
        SahteYanit::metin("bitti"),
    ]);
    let mut d = oturum(&s, vec![a(&yaz)], ayarlar(Kip::Siki));
    d.gonder(mesaj("yaz")).await;
    let ogeler = onay_bekle(&mut d).await;
    d.gonder(Komut::AracOnayi {
        cagri: ogeler[0].cagri,
        karar: OnayKarari::Onayla,
        muhur: OnayMuhru("sahte".into()),
        kisayol: false,
    })
    .await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("mührü eşleşmedi")))
        .await;
    assert_eq!(yaz.cagri_sayisi(), 0);
    d.gonder(onayla(&ogeler[0])).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(yaz.cagri_sayisi(), 1);
}

#[tokio::test]
async fn reddetme_gerekcesi_modele_gider() {
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", yol("b.txt")),
        SahteYanit::metin("peki"),
    ]);
    let mut d = oturum(&s, vec![a(&yaz)], ayarlar(Kip::Siki));
    d.gonder(mesaj("yaz")).await;
    let ogeler = onay_bekle(&mut d).await;
    d.gonder(Komut::AracOnayi {
        cagri: ogeler[0].cagri,
        karar: OnayKarari::Reddet {
            gerekce: Some("gerek yok".into()),
        },
        muhur: ogeler[0].muhur.clone(),
        kisayol: false,
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(sonuc_durumlari(&d), vec![AracDurumu::Reddedildi]);
    assert_eq!(
        arac_mesajlari(&s.istekler()[1].messages),
        vec!["[kullanıcı reddetti] gerek yok"]
    );
    assert_eq!(yaz.cagri_sayisi(), 0);
}

#[tokio::test]
async fn proje_kipinde_yazma_onaysiz() {
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", yol("b.txt")),
        SahteYanit::metin("bitti"),
    ]);
    let mut d = oturum(&s, vec![a(&yaz)], ayarlar(Kip::Proje));
    d.gonder(mesaj("yaz")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert!(
        !d.gecmis
            .iter()
            .any(|z| matches!(z.yuk, Olay::OnayGerekiyor { .. }))
    );
    assert_eq!(yaz.cagri_sayisi(), 1);
}

/// Güvenlik analizi Bulgu 1: `.git/commondir` git'i saldırganın
/// yapılandırmasına yönlendirir; Proje kipinde de onaysız yazılamaz.
#[tokio::test]
async fn proje_kipinde_commondir_yazmasi_onaya_cikar() {
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", yol(".git/commondir")),
        SahteYanit::metin("peki"),
    ]);
    let mut d = oturum(&s, vec![a(&yaz)], ayarlar(Kip::Proje));
    d.gonder(mesaj("yaz")).await;
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(ogeler[0].uyarilar, vec![KORUMALI_UYARISI]);
    d.gonder(Komut::AracOnayi {
        cagri: ogeler[0].cagri,
        karar: OnayKarari::Reddet { gerekce: None },
        muhur: ogeler[0].muhur.clone(),
        kisayol: false,
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(sonuc_durumlari(&d), vec![AracDurumu::Reddedildi]);
    assert_eq!(yaz.cagri_sayisi(), 0);
}

/// Güvenlik analizi Bulgu 7: yolunda yön değiştiren karakter olan yazma
/// Serbest kipte de onay ister.
#[tokio::test]
async fn gorunmez_karakterli_yol_serbest_kipte_de_onay_ister() {
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", yol("belge\u{202E}txt.exe")),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = oturum(&s, vec![a(&yaz)], ayarlar(Kip::Serbest));
    d.gonder(mesaj("yaz")).await;
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(ogeler[0].uyarilar, vec![GORUNMEZ_UYARISI]);
    d.gonder(onayla(&ogeler[0])).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(yaz.cagri_sayisi(), 1, "onaydan sonra çalışır");
}

#[tokio::test]
async fn gizli_dosya_okumasi_serbest_kipte_de_onay_ister() {
    let oku = Arc::new(SahteArac::okuyan("oku"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("oku", yol("config/.env")),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = oturum(&s, vec![a(&oku)], ayarlar(Kip::Serbest));
    d.gonder(mesaj("oku")).await;
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(ogeler[0].uyarilar, vec![GIZLI_UYARISI]);
    d.gonder(onayla(&ogeler[0])).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(oku.cagri_sayisi(), 1);
}

#[tokio::test]
async fn inceleme_kipinde_yazan_arac_sunulmaz_ve_calismaz() {
    let (oku, yaz) = (
        Arc::new(SahteArac::okuyan("oku")),
        Arc::new(SahteArac::yazan("yaz")),
    );
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", yol("b.txt")),
        SahteYanit::metin("peki"),
    ]);
    let mut d = oturum(&s, vec![a(&oku), a(&yaz)], ayarlar(Kip::Inceleme));
    d.gonder(mesaj("yaz")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    let sunulan: Vec<String> = s.istekler()[0]
        .tools
        .iter()
        .map(|t| t.function.name.clone())
        .collect();
    assert_eq!(sunulan, vec!["oku"]);
    assert_eq!(sonuc_durumlari(&d), vec![AracDurumu::Basarisiz]);
    assert!(arac_mesajlari(&s.istekler()[1].messages)[0].contains("İnceleme kipinde"));
    assert_eq!(yaz.cagri_sayisi(), 0);
}

#[tokio::test]
async fn sema_hatasi_ve_bilinmeyen_arac_modele_doner_her_cagriya_tek_yanit() {
    let oku = Arc::new(SahteArac::okuyan("oku"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![("oku", json!({})), ("yok", json!({}))]),
        SahteYanit::metin("düzeltirim"),
    ]);
    let mut d = oturum(&s, vec![a(&oku)], ayarlar(Kip::Siki));
    d.gonder(mesaj("dene")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    let yanitlar = arac_mesajlari(&s.istekler()[1].messages);
    assert_eq!(yanitlar.len(), 2, "her araç çağrısına tam bir yanıt");
    assert!(yanitlar.iter().any(|y| y.contains("şemaya uymuyor")));
    assert!(yanitlar.iter().any(|y| y.contains("adlı bir araç yok")));
    assert_eq!(oku.cagri_sayisi(), 0);
}

#[tokio::test]
async fn onaydan_sonra_dosya_degisirse_yazilmaz() {
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", yol("b.txt")),
        SahteYanit::metin("yeniden denerim"),
    ]);
    let mut d = oturum(&s, vec![a(&yaz)], ayarlar(Kip::Siki));
    d.gonder(mesaj("yaz")).await;
    let ogeler = onay_bekle(&mut d).await;
    yaz.dosyayi_degistir();
    d.gonder(onayla(&ogeler[0])).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(sonuc_durumlari(&d), vec![AracDurumu::Basarisiz]);
    assert!(arac_mesajlari(&s.istekler()[1].messages)[0].contains("onaydan sonra değişti"));
    assert_eq!(yaz.cagri_sayisi(), 0, "eski onayla yazılmaz");
}

#[tokio::test]
async fn ayni_dosyaya_yazmalar_seri_okumalar_paralel() {
    let yaz = Arc::new(SahteArac::yazan("yaz").gecikme(Duration::from_millis(100)));
    let oku = Arc::new(SahteArac::okuyan("oku").gecikme(Duration::from_millis(200)));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![("yaz", yol("a.txt")), ("yaz", yol("A.TXT"))]),
        SahteYanit::araclar(vec![("oku", yol("a.txt")), ("oku", yol("a.txt"))]),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = oturum(&s, vec![a(&yaz), a(&oku)], ayarlar(Kip::Proje));
    d.gonder(mesaj("paralel")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(yaz.cagri_sayisi(), 2);
    assert_eq!(
        yaz.azami_eszamanli(),
        1,
        "aynı dosyaya iki yazma seri çalışır"
    );
    assert_eq!(
        oku.azami_eszamanli(),
        2,
        "aynı dosyayı iki okuma paralel okur"
    );
}

/// Aşama 2 tasarımı §9.8: şu an sunulmayan araç (yalıtım yokken kabuk) modele
/// tanımlanmaz; model yine de çağırırsa çağrı çalışmadan başarısız olur.
#[tokio::test]
async fn sunulmayan_arac_tanimlanmaz_ve_cagrisi_calismaz() {
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    yaz.sunulur_ayarla(false);
    let oku = Arc::new(SahteArac::okuyan("oku"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", yol("a.txt")),
        SahteYanit::metin("peki"),
    ]);
    let mut d = oturum(&s, vec![a(&yaz), a(&oku)], ayarlar(Kip::Serbest));
    d.gonder(mesaj("yaz")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    let adlar: Vec<String> = s.istekler()[0]
        .tools
        .iter()
        .map(|t| t.function.name.clone())
        .collect();
    assert_eq!(adlar, vec!["oku"]);
    assert_eq!(yaz.cagri_sayisi(), 0);
    assert_eq!(sonuc_durumlari(&d), vec![AracDurumu::Basarisiz]);
    assert!(
        d.gecmis.iter().any(|z| matches!(
            &z.yuk,
            Olay::AracSonucu { icerik, .. } if icerik.contains("şu an kullanılamıyor")
        )),
        "sonuç modele nedeni söylemeli"
    );
}

#[tokio::test]
async fn bilgi_vermeyen_arac_icin_kart_arac_sonucu_yazar() {
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", yol("b.txt")),
        SahteYanit::metin("peki"),
    ]);
    let mut d = oturum(&s, vec![a(&yaz)], ayarlar(Kip::Siki));
    d.gonder(mesaj("yaz")).await;
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(ogeler[0].veri_cikisi, None);
}
