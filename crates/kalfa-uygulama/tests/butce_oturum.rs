//! Oturum deneme defteri ve etkin bağlam penceresi uygulama katmanında: yeni
//! oturumun başlangıç sayaçları, kayıttan açılan oturumda sayaçların korunması,
//! güncel etkin pencerenin alınması ve eski kayıtlarda devir.

use std::path::{Path, PathBuf};
use std::time::Duration;

use kalfa_core::aktarim::KanalMesaji;
use kalfa_core::butce::ButceGorunumu;
use kalfa_core::gonderim::IstekSonucu;
use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::{OturumId, TurId};
use kalfa_core::protokol::{BitisNedeni, Olay, SEMA_SURUMU, Zarf};
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::{SahteSaglayici, SahteYanit};
use kalfa_uygulama::dizin::{kayit_yolu, oturum_dizini};
use kalfa_uygulama::test_destek::{SahteKanal, TestUygulamasi, test_uygulamasi};
use tempfile::TempDir;
use tokio::sync::mpsc::UnboundedReceiver;

struct Ortam {
    veri: TempDir,
    alan: TempDir,
    t: TestUygulamasi,
}

/// Anahtarı kayıtlı ve şartları kabul edilmiş, açılmış uygulama.
async fn ortam(senaryo: Vec<SahteYanit>) -> Ortam {
    let veri = tempfile::tempdir().unwrap();
    let alan = tempfile::tempdir().unwrap();
    let saglayici = SahteSaglayici::yeni(senaryo);
    saglayici.sart_kabul(1).await.unwrap();
    let t = test_uygulamasi(veri.path(), saglayici, Some("evren_llm_a"));
    t.uygulama.baslat().await;
    t.iletisim.klasor_ver(alan.path());
    Ortam { veri, alan, t }
}

impl Ortam {
    async fn yeni_oturum(&self) -> OturumId {
        let alan = self.t.uygulama.calisma_alani_sec().await.unwrap();
        self.t
            .uygulama
            .oturum_baslat(alan.kimlik, "glm-5.3".into())
            .await
            .unwrap()
            .oturum
    }

    fn dinle(&self, oturum: OturumId) -> UnboundedReceiver<KanalMesaji> {
        let (kanal, rx) = SahteKanal::yeni();
        self.t.uygulama.oturumu_dinle(oturum, kanal).unwrap();
        rx
    }

    fn kayit(&self, oturum: OturumId) -> PathBuf {
        kayit_yolu(&oturum_dizini(self.veri.path()), oturum)
    }
}

/// Paketleri işleyerek okur; koşula uyan olay gelince durur.
async fn olay_bekle(
    t: &TestUygulamasi,
    oturum: OturumId,
    rx: &mut UnboundedReceiver<KanalMesaji>,
    kosul: impl Fn(&Olay) -> bool,
) -> Vec<Olay> {
    let mut gorulen = Vec::new();
    loop {
        let mesaj = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("olay 5 saniyede gelmedi")
            .expect("kanal kapandı");
        let KanalMesaji::Olaylar { no, zarflar } = mesaj else {
            panic!("kanal beklenmeden kapandı");
        };
        t.uygulama.paketi_isledim(oturum, no).unwrap();
        for z in zarflar {
            let bulundu = kosul(&z.yuk);
            gorulen.push(z.yuk);
            if bulundu {
                return gorulen;
            }
        }
    }
}

fn tamamlandi(o: &Olay) -> bool {
    matches!(o, Olay::Tamamlandi { .. })
}

/// 5b öncesi kayıt: çalışma alanı var, iki sohbet isteği ve bütçe kaydı yok.
fn eski_istekli_kayit(yol: &Path, oturum: OturumId, alan: &Path) {
    let mut kayit = OturumKaydi::olustur(yol).unwrap();
    let mut sira = 0;
    let mut yaz = |tur: Option<TurId>, yuk: KayitOgesi| {
        sira += 1;
        kayit
            .yaz(&Zarf {
                sema: SEMA_SURUMU,
                oturum,
                tur,
                sira,
                yuk,
            })
            .unwrap();
    };
    yaz(
        None,
        KayitOgesi::OturumAcildi {
            model: "glm-5.3".into(),
            kip: Kip::Siki,
            calisma_alani: Some(alan.to_path_buf()),
        },
    );
    let tur = Some(TurId::yeni());
    for _ in 0..2 {
        yaz(
            tur,
            KayitOgesi::Istek {
                amac: "ana".into(),
                istek_id: None,
                kredi: None,
                yonlendirilen_model: None,
                model: Some("glm-5.3".into()),
                yeni: Vec::new(),
                onceki: 0,
                sonuc: IstekSonucu::Basarili,
                butce_islem: None,
            },
        );
    }
    yaz(
        tur,
        KayitOgesi::TurBitti {
            neden: BitisNedeni::Normal,
        },
    );
}

#[tokio::test]
async fn yeni_oturum_sifir_butce_sayaciyla_baslar() {
    let o = ortam(vec![]).await;
    let alan = o.t.uygulama.calisma_alani_sec().await.unwrap();
    let acilis =
        o.t.uygulama
            .oturum_baslat(alan.kimlik, "glm-5.3".into())
            .await
            .unwrap();
    let butce = acilis.butce.expect("yeni oturumun bütçe görünümü olmalı");
    assert_eq!(
        butce,
        ButceGorunumu {
            baslatilan: 0,
            bilinen_token: 0,
            bilinmeyen: 0,
            aktif: 0,
        }
    );
}

#[tokio::test]
async fn kayittan_acilan_oturum_sayaclarini_ve_guncel_etkin_pencereyi_alir() {
    let o = ortam(vec![SahteYanit::metin("merhaba")]).await;
    let oturum = o.yeni_oturum().await;
    let mut rx = o.dinle(oturum);
    o.t.uygulama
        .mesaj_gonder(oturum, "selam".into())
        .await
        .unwrap();
    olay_bekle(&o.t, oturum, &mut rx, tamamlandi).await;
    o.t.uygulama.oturumu_kapat(oturum).await.unwrap();

    o.t.uygulama.etkin_pencereyi_ayarla(60_000).unwrap();
    let acilis = o.t.uygulama.oturum_ac(oturum).await.unwrap();
    let butce = acilis.butce.expect("kayıttan açılan oturumun bütçesi");
    assert_eq!(butce.baslatilan, 1, "sayaç kayıttan gelir, sıfırlanmaz");
    assert_eq!(butce.bilinen_token, 15);
    let sinirlar =
        o.t.uygulama
            .oturum_sinirlari(oturum)
            .expect("açılan oturumun sınırları");
    assert_eq!(sinirlar.etkin_pencere, 60_000);
}

/// Çalışma alanı silinmiş oturum salt okunur açılır; kayıtlı sayaçlar yine görünür.
#[tokio::test]
async fn salt_okunur_acilis_kayitli_butceyi_verir() {
    let o = ortam(vec![SahteYanit::metin("merhaba")]).await;
    let oturum = o.yeni_oturum().await;
    let mut rx = o.dinle(oturum);
    o.t.uygulama
        .mesaj_gonder(oturum, "selam".into())
        .await
        .unwrap();
    olay_bekle(&o.t, oturum, &mut rx, tamamlandi).await;
    o.t.uygulama.oturumu_kapat(oturum).await.unwrap();
    std::fs::remove_dir_all(o.alan.path()).unwrap();

    let acilis = o.t.uygulama.oturum_ac(oturum).await.unwrap();
    assert!(acilis.salt_okunur.is_some());
    let butce = acilis.butce.expect("salt okunur oturumun kayıtlı bütçesi");
    assert_eq!((butce.baslatilan, butce.bilinen_token), (1, 15));
}

#[tokio::test]
async fn eski_kayit_acilista_sayaclari_devralir_ve_karar_istemez() {
    let o = ortam(vec![SahteYanit::metin("merhaba")]).await;
    std::fs::create_dir_all(oturum_dizini(o.veri.path())).unwrap();
    let oturum = OturumId::yeni();
    eski_istekli_kayit(&o.kayit(oturum), oturum, o.alan.path());

    let acilis = o.t.uygulama.oturum_ac(oturum).await.unwrap();
    assert_eq!(
        acilis.butce,
        Some(ButceGorunumu {
            baslatilan: 2,
            bilinen_token: 0,
            bilinmeyen: 1,
            aktif: 0,
        })
    );
    let mut rx = o.dinle(oturum);
    o.t.uygulama
        .mesaj_gonder(oturum, "devam".into())
        .await
        .unwrap();
    let olaylar = olay_bekle(&o.t, oturum, &mut rx, tamamlandi).await;
    assert!(matches!(
        olaylar.last(),
        Some(Olay::Tamamlandi {
            neden: BitisNedeni::Normal
        })
    ));
}

#[tokio::test]
async fn oturum_ozetle_komutu_oturuma_iletir() {
    let o = ortam(vec![SahteYanit::metin("ilk yanıt")]).await;
    o.t.saglayici
        .sohbet_ekle(kalfa_core::test_destek::SahteSohbet::Metin(
            "kısa özet".into(),
        ));
    let oturum = o.yeni_oturum().await;
    let mut rx = o.dinle(oturum);
    o.t.uygulama
        .mesaj_gonder(oturum, "selam".into())
        .await
        .unwrap();
    olay_bekle(&o.t, oturum, &mut rx, tamamlandi).await;

    o.t.uygulama
        .oturum_ozetle(oturum, Some("odak: test".into()))
        .await
        .unwrap();
    let olaylar = olay_bekle(&o.t, oturum, &mut rx, tamamlandi).await;
    assert!(olaylar.iter().any(|o| matches!(
        o,
        Olay::OturumOzetlendi {
            tetik: kalfa_core::protokol::OzetTetigi::Elle,
            talimat: Some(t),
            ozet,
            ..
        } if t == "odak: test" && ozet == "kısa özet"
    )));
}
