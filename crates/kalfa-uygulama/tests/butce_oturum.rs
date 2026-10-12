//! Oturum bütçesi uygulama katmanında (5b tasarımı §7–§9): yeni oturumun
//! varsayılanı, kayıtlı limitlerin kalması, eski kayıtta açılış kararı ve karar
//! yönlendirmesi.

use std::path::{Path, PathBuf};
use std::time::Duration;

use kalfa_core::aktarim::KanalMesaji;
use kalfa_core::butce::{ButceEngeli, ButceSinirlari};
use kalfa_core::gonderim::IstekSonucu;
use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::{OturumId, TurId};
use kalfa_core::protokol::{BitisNedeni, ButceKarari, Olay, SEMA_SURUMU, Zarf};
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::{SahteSaglayici, SahteYanit};
use kalfa_uygulama::dizin::{kayit_yolu, oturum_dizini};
use kalfa_uygulama::hata::HataKodu;
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

fn sinir(istek: u64, token: u64) -> ButceSinirlari {
    ButceSinirlari { istek, token }
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
async fn yeni_oturum_ayardaki_varsayilani_alir() {
    let o = ortam(vec![]).await;
    o.t.uygulama.oturum_butcesini_ayarla(4, 2000).unwrap();
    let alan = o.t.uygulama.calisma_alani_sec().await.unwrap();
    let acilis =
        o.t.uygulama
            .oturum_baslat(alan.kimlik, "glm-5.3".into())
            .await
            .unwrap();
    let butce = acilis.butce.expect("yeni oturumun bütçe görünümü olmalı");
    assert_eq!(butce.sinirlar, sinir(4, 2000));
    assert_eq!(
        (butce.baslatilan, butce.bilinen_token, butce.bilinmeyen),
        (0, 0, 0)
    );
    assert_eq!(butce.engel, None);
}

#[tokio::test]
async fn genel_ayar_degisince_kayitli_oturumun_limiti_degismez() {
    let o = ortam(vec![SahteYanit::metin("merhaba")]).await;
    let oturum = o.yeni_oturum().await;
    let mut rx = o.dinle(oturum);
    o.t.uygulama
        .mesaj_gonder(oturum, "selam".into())
        .await
        .unwrap();
    olay_bekle(&o.t, oturum, &mut rx, tamamlandi).await;
    o.t.uygulama.oturumu_kapat(oturum).await.unwrap();

    o.t.uygulama.oturum_butcesini_ayarla(9, 9000).unwrap();
    let acilis = o.t.uygulama.oturum_ac(oturum).await.unwrap();
    let butce = acilis.butce.expect("kayıttan açılan oturumun bütçesi");
    assert_eq!(butce.sinirlar, sinir(200, 5_000_000));
    assert_eq!(butce.baslatilan, 1, "sayaç kayıttan gelir, sıfırlanmaz");
    assert_eq!(butce.bilinen_token, 15);
    assert_eq!(butce.engel, None);
    assert_eq!(o.t.uygulama.oturum_butcesi(), sinir(9, 9000));
}

#[tokio::test]
async fn butce_karari_bilinmeyen_oturumda_reddedilir() {
    let o = ortam(vec![]).await;
    let hata =
        o.t.uygulama
            .butce_karari(OturumId::yeni(), ButceKarari::Durdur)
            .await
            .unwrap_err();
    assert_eq!(hata.kod, HataKodu::OturumYok);
}

#[tokio::test]
async fn eski_kayit_ilk_istekten_once_karar_ister_ve_karar_dogrulanir() {
    let o = ortam(vec![SahteYanit::metin("merhaba")]).await;
    std::fs::create_dir_all(oturum_dizini(o.veri.path())).unwrap();
    let oturum = OturumId::yeni();
    eski_istekli_kayit(&o.kayit(oturum), oturum, o.alan.path());

    // Kayıttan açılış bütçe kaydı olmayan oturumda sayaç görünümü vermez; karar
    // açılışta gelir, ilk istek ağa çıkmadan önce.
    let acilis = o.t.uygulama.oturum_ac(oturum).await.unwrap();
    assert_eq!(acilis.butce, None);
    let mut rx = o.dinle(oturum);
    let olaylar = olay_bekle(&o.t, oturum, &mut rx, |y| {
        matches!(y, Olay::ButceKarariGerekli { .. })
    })
    .await;
    let durum = olaylar
        .iter()
        .find_map(|y| match y {
            Olay::ButceKarariGerekli { durum } => Some(*durum),
            _ => None,
        })
        .expect("açılışta karar kartı");
    assert_eq!(durum.engel, Some(ButceEngeli::Bilinmeyen));
    assert_eq!((durum.baslatilan, durum.bilinmeyen), (2, 1));
    assert!(
        o.t.saglayici.sohbet_istekleri().is_empty(),
        "karar verilmeden ağa çıkılmamalı"
    );

    // Yanlış gözlem reddedilir; kart güncel sayaçlarla yeniden yayılır.
    o.t.uygulama
        .butce_karari(
            oturum,
            ButceKarari::DevamEt {
                sinirlar: sinir(10, 5_000_000),
                gozlenen_baslatilan: 1,
                gozlenen_bilinmeyen: 1,
            },
        )
        .await
        .unwrap();
    let olaylar = olay_bekle(&o.t, oturum, &mut rx, |y| {
        matches!(y, Olay::ButceKarariGerekli { .. })
    })
    .await;
    assert!(
        olaylar.iter().any(|y| matches!(y, Olay::Hata { .. })),
        "yanlış gözlem hata vermeli"
    );

    // Doğru gözlem kabul edilir; sayaçlar sıfırlanmaz, engel kalkar.
    o.t.uygulama
        .butce_karari(
            oturum,
            ButceKarari::DevamEt {
                sinirlar: sinir(10, 5_000_000),
                gozlenen_baslatilan: 2,
                gozlenen_bilinmeyen: 1,
            },
        )
        .await
        .unwrap();
    let olaylar = olay_bekle(&o.t, oturum, &mut rx, |y| {
        matches!(y, Olay::ButceDurumu { .. })
    })
    .await;
    let Some(Olay::ButceDurumu { durum }) = olaylar.last() else {
        panic!("kabul edilen kararın bütçe durumu gelmeli: {olaylar:?}");
    };
    assert_eq!(durum.engel, None);
    assert_eq!(durum.sinirlar, sinir(10, 5_000_000));
    assert_eq!((durum.baslatilan, durum.onaylanan_bilinmeyen), (2, 1));

    // Kayıt yeniden açılınca onaylanan bilinmeyen geri gelir; eksik yeniden sorulur.
    o.t.uygulama.oturumu_kapat(oturum).await.unwrap();
    let acilis = o.t.uygulama.oturum_ac(oturum).await.unwrap();
    let butce = acilis.butce.expect("kurtarılan oturumun bütçesi");
    assert_eq!((butce.baslatilan, butce.onaylanan_bilinmeyen), (2, 1));
    assert_eq!(butce.engel, None);
}
