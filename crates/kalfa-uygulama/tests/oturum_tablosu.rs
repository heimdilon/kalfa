//! Oturum tablosu (1c tasarımı §5): başlatma, köprü, kayıttan açma, kapatma,
//! kopma, izin kipi ve bağlantı açma.

use std::path::{Path, PathBuf};
use std::time::Duration;

use kalfa_core::aktarim::KanalMesaji;
use kalfa_core::gecmis::GorunumOgesi;
use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::{IslemId, OturumId, TurId};
use kalfa_core::protokol::{BitisNedeni, Olay, SEMA_SURUMU, Zarf};
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::{SahteSaglayici, SahteYanit};
use kalfa_evren::AkisOlayi;
use kalfa_evren::tipler::ChatMessage;
use kalfa_uygulama::dizin::{kayit_yolu, oturum_dizini};
use kalfa_uygulama::hata::HataKodu;
use kalfa_uygulama::sozlesme::SaltOkunurNedeni;
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

/// Paketleri "işledim" diyerek okur; koşula uyan olay gelince durur.
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

#[tokio::test]
async fn calisma_alani_secilmeden_ya_da_bilinmeyen_modelle_baslamaz() {
    let o = ortam(Vec::new()).await;
    let hata =
        o.t.uygulama
            .oturum_baslat(99, "glm-5.3".into())
            .await
            .unwrap_err();
    assert_eq!(hata.kod, HataKodu::GecersizGirdi);
    let alan = o.t.uygulama.calisma_alani_sec().await.unwrap();
    let hata =
        o.t.uygulama
            .oturum_baslat(alan.kimlik, "gpt-5".into())
            .await
            .unwrap_err();
    assert_eq!(hata.kod, HataKodu::GecersizGirdi);
    assert_eq!(o.t.uygulama.acik_oturum_sayisi(), 0);
}

#[tokio::test]
async fn oturum_akar_kapatilir_ve_kayittan_acilir() {
    let o = ortam(vec![SahteYanit::metin("merhaba")]).await;
    let oturum = o.yeni_oturum().await;
    assert!(
        o.kayit(oturum).exists(),
        "kayıt dosyasının adı oturum kimliği"
    );
    let mut rx = o.dinle(oturum);
    o.t.uygulama
        .mesaj_gonder(oturum, "selam".into())
        .await
        .unwrap();
    let olaylar = olay_bekle(&o.t, oturum, &mut rx, tamamlandi).await;
    assert!(
        olaylar
            .iter()
            .any(|x| matches!(x, Olay::TokenParcasi { metin } if metin.contains("merhaba")))
    );
    assert_eq!(o.t.uygulama.uygulama_durumu().son_oturum, Some(oturum));

    o.t.uygulama.oturumu_kapat(oturum).await.unwrap();
    assert_eq!(o.t.uygulama.acik_oturum_sayisi(), 0);

    let acilis = o.t.uygulama.oturum_ac(oturum).await.unwrap();
    assert_eq!(acilis.salt_okunur, None);
    assert_eq!(
        acilis.calisma_alani,
        Some(o.alan.path().display().to_string())
    );
    assert!(
        acilis
            .gecmis
            .ogeler
            .iter()
            .any(|x| matches!(x, GorunumOgesi::KullaniciMesaji { metin, .. } if metin == "selam"))
    );
    assert!(
        acilis
            .gecmis
            .ogeler
            .iter()
            .any(|x| matches!(x, GorunumOgesi::Yanit { metin: Some(m), .. } if m == "merhaba"))
    );
    assert_eq!(acilis.ozet.model.as_deref(), Some("glm-5.3"));
    assert_eq!(o.t.uygulama.acik_oturum_sayisi(), 1);
}

#[tokio::test]
async fn acik_oturum_ikinci_kez_acilmaz_ve_ikinci_kez_dinlenmez() {
    let o = ortam(Vec::new()).await;
    let oturum = o.yeni_oturum().await;
    let _rx = o.dinle(oturum);
    o.t.uygulama.oturum_ac(oturum).await.unwrap();
    assert_eq!(o.t.uygulama.acik_oturum_sayisi(), 1);
    let (kanal, _rx2) = SahteKanal::yeni();
    let hata = o.t.uygulama.oturumu_dinle(oturum, kanal).unwrap_err();
    assert_eq!(hata.kod, HataKodu::GecersizGirdi);
}

#[tokio::test]
async fn kayit_baska_yerde_aciksa_mesgul_hatasi() {
    let o = ortam(Vec::new()).await;
    let oturum = o.yeni_oturum().await;
    o.t.uygulama.oturumu_kapat(oturum).await.unwrap();
    let _kilit = OturumKaydi::ac(&o.kayit(oturum)).unwrap();
    let hata = o.t.uygulama.oturum_ac(oturum).await.unwrap_err();
    assert_eq!(hata.kod, HataKodu::KayitMesgul);
    assert_eq!(hata.mesaj, "Bu oturum başka bir Kalfa sürecinde açık.");
}

#[tokio::test]
async fn eszamanli_iki_acma_ayni_oturumu_verir() {
    let o = ortam(Vec::new()).await;
    let oturum = o.yeni_oturum().await;
    o.t.uygulama.oturumu_kapat(oturum).await.unwrap();
    let (a, b) = tokio::join!(
        o.t.uygulama.oturum_ac(oturum),
        o.t.uygulama.oturum_ac(oturum)
    );
    assert_eq!(a.unwrap().oturum, oturum);
    assert_eq!(b.unwrap().oturum, oturum);
    assert_eq!(o.t.uygulama.acik_oturum_sayisi(), 1);
}

#[tokio::test]
async fn kaydi_olmayan_oturum_acilmaz() {
    let o = ortam(Vec::new()).await;
    let yok = OturumId::yeni();
    let hata = o.t.uygulama.oturum_ac(yok).await.unwrap_err();
    assert_eq!(hata.kod, HataKodu::OturumYok);
    assert_eq!(hata.mesaj, "Bu oturumun kaydı bulunamadı.");
    let hata = o.t.uygulama.gecmis_sayfasi(yok, 10).await.unwrap_err();
    assert_eq!(hata.kod, HataKodu::OturumYok);
}

#[tokio::test]
async fn calisma_alani_yoksa_yalnizca_gecmis_gosterilir() {
    let mut o = ortam(Vec::new()).await;
    let oturum = o.yeni_oturum().await;
    o.t.uygulama.oturumu_kapat(oturum).await.unwrap();
    let alan_yolu = o.alan.path().display().to_string();
    std::mem::replace(&mut o.alan, tempfile::tempdir().unwrap())
        .close()
        .unwrap();
    let acilis = o.t.uygulama.oturum_ac(oturum).await.unwrap();
    assert_eq!(
        acilis.salt_okunur,
        Some(SaltOkunurNedeni::CalismaAlaniYok { yol: alan_yolu })
    );
    assert_eq!(o.t.uygulama.acik_oturum_sayisi(), 0);
    let hata =
        o.t.uygulama
            .mesaj_gonder(oturum, "x".into())
            .await
            .unwrap_err();
    assert_eq!(hata.kod, HataKodu::OturumYok);
}

/// Çalışma alanını yazmayan (1c öncesi) kayıt: 21 tur, her birinde bir
/// kullanıcı mesajı ve bir yanıt.
fn eski_kayit(yol: &Path, oturum: OturumId) {
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
            calisma_alani: None,
        },
    );
    for i in 0..21 {
        let tur = Some(TurId::yeni());
        yaz(
            tur,
            KayitOgesi::Mesaj {
                mesaj: ChatMessage::user(format!("soru {i}")),
                kaynak: None,
            },
        );
        yaz(
            tur,
            KayitOgesi::Mesaj {
                mesaj: ChatMessage::assistant(Some(format!("yanıt {i}")), Vec::new()),
                kaynak: None,
            },
        );
        yaz(
            tur,
            KayitOgesi::TurBitti {
                neden: BitisNedeni::Normal,
            },
        );
    }
}

#[tokio::test]
async fn eski_kayit_salt_okunur_acilir_ve_sayfa_sayfa_okunur() {
    let o = ortam(Vec::new()).await;
    let oturum = OturumId::yeni();
    std::fs::create_dir_all(oturum_dizini(o.veri.path())).unwrap();
    eski_kayit(&o.kayit(oturum), oturum);

    let acilis = o.t.uygulama.oturum_ac(oturum).await.unwrap();
    assert_eq!(
        acilis.salt_okunur,
        Some(SaltOkunurNedeni::CalismaAlaniKayitsiz)
    );
    assert!(acilis.gecmis.daha_eski_var);
    let ilk_sorular: Vec<&str> = acilis
        .gecmis
        .ogeler
        .iter()
        .filter_map(|x| match x {
            GorunumOgesi::KullaniciMesaji { metin, .. } => Some(metin.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(ilk_sorular.len(), 20);
    assert_eq!(ilk_sorular[0], "soru 1");

    let en_eski = acilis.gecmis.ogeler[0].sira();
    let sayfa = o.t.uygulama.gecmis_sayfasi(oturum, en_eski).await.unwrap();
    assert!(!sayfa.daha_eski_var);
    assert!(matches!(
        &sayfa.ogeler[0],
        GorunumOgesi::KullaniciMesaji { metin, .. } if metin == "soru 0"
    ));
}

#[tokio::test]
async fn oturum_listesi_en_yeni_once() {
    let o = ortam(vec![SahteYanit::metin("a"), SahteYanit::metin("b")]).await;
    for mesaj in ["ilk oturum", "ikinci oturum"] {
        let oturum = o.yeni_oturum().await;
        let mut rx = o.dinle(oturum);
        o.t.uygulama
            .mesaj_gonder(oturum, mesaj.into())
            .await
            .unwrap();
        olay_bekle(&o.t, oturum, &mut rx, tamamlandi).await;
        o.t.uygulama.oturumu_kapat(oturum).await.unwrap();
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    let liste = o.t.uygulama.oturumlari_listele().await.unwrap();
    let basliklar: Vec<_> = liste.iter().map(|s| s.baslik.as_deref()).collect();
    assert_eq!(basliklar, [Some("ikinci oturum"), Some("ilk oturum")]);
    assert!(
        liste
            .iter()
            .all(|s| s.dosya.ends_with(".jsonl") && s.oturum.is_some())
    );
}

#[tokio::test]
async fn izin_kipi_yukseltmesi_yerel_onay_ister() {
    let o = ortam(Vec::new()).await;
    let oturum = o.yeni_oturum().await;
    let u = &o.t.uygulama;
    o.t.iletisim.onay_ver(false);
    assert_eq!(
        u.izin_kipi_degistir(oturum, Kip::Proje).await.unwrap(),
        Kip::Siki
    );
    assert_eq!(o.t.iletisim.sorulanlar().len(), 1);
    assert!(o.t.iletisim.sorulanlar()[0].contains("Sıkı yerine Proje"));
    o.t.iletisim.onay_ver(true);
    assert_eq!(
        u.izin_kipi_degistir(oturum, Kip::Serbest).await.unwrap(),
        Kip::Serbest
    );
    assert_eq!(o.t.iletisim.sorulanlar().len(), 2);
    assert_eq!(
        u.izin_kipi_degistir(oturum, Kip::Inceleme).await.unwrap(),
        Kip::Inceleme
    );
    assert_eq!(
        o.t.iletisim.sorulanlar().len(),
        2,
        "kip düşürmek onay istemez"
    );
}

#[tokio::test]
async fn eszamanli_kip_degisimleri_ayni_oturumda_sirayla_islenir() {
    let o = ortam(Vec::new()).await;
    let oturum = o.yeni_oturum().await;
    o.t.iletisim.onay_ver(true);
    let onay_izni = o.t.iletisim.onayi_beklet();

    let uygulama = o.t.uygulama.clone();
    let ilk = tokio::spawn(async move { uygulama.izin_kipi_degistir(oturum, Kip::Proje).await });
    tokio::time::timeout(Duration::from_secs(5), async {
        while o.t.iletisim.sorulanlar().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("ilk yerel onay sorulmadı");

    let uygulama = o.t.uygulama.clone();
    let (basladi_tx, basladi_rx) = tokio::sync::oneshot::channel();
    let mut ikinci = tokio::spawn(async move {
        let _ = basladi_tx.send(());
        uygulama.izin_kipi_degistir(oturum, Kip::Inceleme).await
    });
    basladi_rx.await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut ikinci)
            .await
            .is_err(),
        "ikinci değişiklik ilk yerel onay sürerken tamamlanmamalı"
    );

    onay_izni.add_permits(1);
    assert_eq!(ilk.await.unwrap().unwrap(), Kip::Proje);
    assert_eq!(ikinci.await.unwrap().unwrap(), Kip::Inceleme);
    assert_eq!(o.t.iletisim.sorulanlar().len(), 1);
}

#[tokio::test]
async fn baglanti_yalnizca_http_https_ve_onayla_acilir() {
    let o = ortam(Vec::new()).await;
    let u = &o.t.uygulama;
    for kotu in ["javascript:alert(1)", "file:///etc/passwd", "ornek.com"] {
        let hata = u.baglanti_ac(kotu.into()).await.unwrap_err();
        assert_eq!(hata.kod, HataKodu::GecersizGirdi, "{kotu}");
    }
    assert!(o.t.iletisim.sorulanlar().is_empty());
    o.t.iletisim.onay_ver(false);
    u.baglanti_ac("https://ornek.com".into()).await.unwrap();
    assert!(o.t.acici.acilanlar().is_empty());
    assert!(o.t.iletisim.sorulanlar()[0].contains("https://ornek.com/"));
    o.t.iletisim.onay_ver(true);
    u.baglanti_ac("https://ornek.com".into()).await.unwrap();
    assert_eq!(o.t.acici.acilanlar(), ["https://ornek.com/"]);
}

#[tokio::test]
async fn ana_pencere_gidince_tur_iptal_edilir_kilit_birakilir() {
    let o = ortam(vec![SahteYanit::Askida(vec![AkisOlayi::Metin(
        "yarım".into(),
    )])])
    .await;
    let oturum = o.yeni_oturum().await;
    let mut rx = o.dinle(oturum);
    o.t.uygulama
        .mesaj_gonder(oturum, "uzun iş".into())
        .await
        .unwrap();
    olay_bekle(&o.t, oturum, &mut rx, |x| {
        matches!(x, Olay::TokenParcasi { .. })
    })
    .await;

    let kapananlar = o.t.uygulama.ana_pencere_gitti();
    assert_eq!(kapananlar.len(), 1);
    assert_eq!(o.t.uygulama.acik_oturum_sayisi(), 0);
    let u = o.t.uygulama.clone();
    tokio::spawn(async move { u.kapanislari_bekle(kapananlar).await });

    // Yeniden yüklenen sayfa aynı oturumu açar: önce kapanış beklenir.
    let acilis = o.t.uygulama.oturum_ac(oturum).await.unwrap();
    assert!(acilis.gecmis.ogeler.iter().any(|x| matches!(
        x,
        GorunumOgesi::TurSonu {
            neden: BitisNedeni::Iptal,
            ..
        }
    )));
}

#[tokio::test]
async fn cikista_kapanan_oturumlarin_kaydi_tamamlanana_kadar_beklenir() {
    let o = ortam(vec![SahteYanit::Askida(vec![AkisOlayi::Metin(
        "yarım".into(),
    )])])
    .await;
    let oturum = o.yeni_oturum().await;
    let mut rx = o.dinle(oturum);
    o.t.uygulama
        .mesaj_gonder(oturum, "uzun iş".into())
        .await
        .unwrap();
    olay_bekle(&o.t, oturum, &mut rx, |x| {
        matches!(x, Olay::TokenParcasi { .. })
    })
    .await;
    assert!(!o.t.uygulama.kapanan_oturum_var());

    // Son pencere kapandı: süreç çıkmadan önce kapanışlar beklenir (1c
    // tasarımı §5.6); iptal edilen tur kayda yazılmış olur.
    let _ = o.t.uygulama.ana_pencere_gitti();
    assert!(o.t.uygulama.kapanan_oturum_var());
    o.t.uygulama.kapananlari_bekle().await;
    assert!(!o.t.uygulama.kapanan_oturum_var());
    let kayitlar = OturumKaydi::oku(&o.kayit(oturum)).unwrap();
    assert!(kayitlar.iter().any(|z| matches!(
        z.yuk,
        KayitOgesi::TurBitti {
            neden: BitisNedeni::Iptal
        }
    )));
}

#[tokio::test]
async fn turu_iptal_etmek_tamamlandi_iptal_uretir() {
    let o = ortam(vec![SahteYanit::Askida(vec![AkisOlayi::Metin(
        "yarım".into(),
    )])])
    .await;
    let oturum = o.yeni_oturum().await;
    let mut rx = o.dinle(oturum);
    o.t.uygulama
        .mesaj_gonder(oturum, "uzun iş".into())
        .await
        .unwrap();
    olay_bekle(&o.t, oturum, &mut rx, |x| {
        matches!(x, Olay::TokenParcasi { .. })
    })
    .await;
    o.t.uygulama.turu_iptal_et(oturum).unwrap();
    let olaylar = olay_bekle(&o.t, oturum, &mut rx, tamamlandi).await;
    assert!(matches!(
        olaylar.last(),
        Some(Olay::Tamamlandi {
            neden: BitisNedeni::Iptal
        })
    ));
}

/// Aşama 2 tasarımı §9.8: çalışma alanı seçici ev dizinindeki "Kalfa
/// Projeleri"nde açılır; klasör yoksa oluşturulur. Araç zinciri seçicisi
/// başlangıç vermez.
#[tokio::test]
async fn calisma_alani_secici_kalfa_projeleri_ile_acilir() {
    let o = ortam(Vec::new()).await;
    let projeler = o.veri.path().join("ev").join("Kalfa Projeleri");
    assert!(!projeler.exists());
    o.t.uygulama.calisma_alani_sec().await.unwrap();
    assert!(projeler.is_dir(), "klasör oluşturulmalı");
    assert_eq!(o.t.iletisim.son_baslangic(), Some(projeler));

    o.t.uygulama.arac_zinciri_dizini_ekle().await.ok();
    assert_eq!(o.t.iletisim.son_baslangic(), None);
}

// --- Geri alma ve oturumu silme (aşama 3 tasarımı §5.6, §5.10) ---------------

fn geri_alma_deposu(o: &Ortam, oturum: OturumId) -> PathBuf {
    kalfa_uygulama::dizin::oturum_geri_alma_dizini(&o.veri.path().join("geri-alma"), oturum)
}

/// Proje kipinde bir tur `a.txt`'yi değiştirir ve `yeni/b.txt`'yi oluşturur.
async fn yazan_tur(o: &Ortam) -> (OturumId, UnboundedReceiver<KanalMesaji>, TurId) {
    std::fs::write(o.alan.path().join("a.txt"), "eski").unwrap();
    let oturum = o.yeni_oturum().await;
    let mut rx = o.dinle(oturum);
    o.t.iletisim.onay_ver(true);
    o.t.uygulama
        .izin_kipi_degistir(oturum, Kip::Proje)
        .await
        .unwrap();
    o.t.uygulama
        .mesaj_gonder(oturum, "iki dosya".into())
        .await
        .unwrap();
    let olaylar = olay_bekle(&o.t, oturum, &mut rx, |x| {
        matches!(x, Olay::TurOzeti { .. })
    })
    .await;
    let tur = olaylar
        .iter()
        .find_map(|x| match x {
            Olay::TurOzeti { ozet } => Some(ozet.tur),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(o.alan.path().join("a.txt")).unwrap(),
        "yeni"
    );
    (oturum, rx, tur)
}

fn yazan_senaryo() -> Vec<SahteYanit> {
    vec![
        SahteYanit::araclar(vec![
            (
                "dosya_yaz",
                serde_json::json!({"yol": "a.txt", "icerik": "yeni"}),
            ),
            (
                "dosya_yaz",
                serde_json::json!({"yol": "yeni/b.txt", "icerik": "b"}),
            ),
        ]),
        SahteYanit::metin("tamam"),
    ]
}

#[tokio::test]
async fn uygulamada_tur_onizlenip_geri_alinir() {
    let o = ortam(yazan_senaryo()).await;
    let (oturum, mut rx, tur) = yazan_tur(&o).await;
    assert!(
        geri_alma_deposu(&o, oturum).is_dir(),
        "önceki hâl oturumun deposunda"
    );
    o.t.uygulama.geri_alma_onizle(oturum, tur).await.unwrap();
    let olaylar = olay_bekle(&o.t, oturum, &mut rx, |x| {
        matches!(x, Olay::GeriAlmaPlani { .. })
    })
    .await;
    let Some(Olay::GeriAlmaPlani { muhur, plan, .. }) = olaylar.into_iter().last() else {
        unreachable!()
    };
    assert_eq!(plan.ogeler.len(), 2);
    o.t.uygulama.turu_geri_al(oturum, tur, muhur).await.unwrap();
    olay_bekle(&o.t, oturum, &mut rx, |x| {
        matches!(x, Olay::TurGeriAlindi { .. })
    })
    .await;
    assert_eq!(
        std::fs::read_to_string(o.alan.path().join("a.txt")).unwrap(),
        "eski"
    );
    assert!(!o.alan.path().join("yeni").exists(), "boş dizin de kalktı");
    assert!(o.t.uygulama.geri_alma_verisi_boyutu().await.unwrap() > 0);
}

#[tokio::test]
async fn oturumu_sil_kaydi_ve_geri_alma_verisini_siler() {
    let o = ortam(yazan_senaryo()).await;
    let (oturum, _rx, _) = yazan_tur(&o).await;
    let depo = geri_alma_deposu(&o, oturum);
    assert!(depo.is_dir());
    let sorulan = o.t.iletisim.sorulanlar().len();
    assert!(o.t.uygulama.oturumu_sil(oturum).await.unwrap());
    let sorulanlar = o.t.iletisim.sorulanlar();
    assert_eq!(sorulanlar.len(), sorulan + 1, "yerel pencere soruldu");
    assert!(
        sorulanlar
            .last()
            .unwrap()
            .contains("geri alma verisi kalıcı olarak silinecek"),
        "{sorulanlar:?}"
    );
    // Açık oturum önce kapatıldı.
    assert_eq!(o.t.uygulama.acik_oturum_sayisi(), 0);
    assert!(!o.kayit(oturum).exists());
    assert!(!kalfa_core::kayit::kilit_yolu(&o.kayit(oturum)).exists());
    assert!(!depo.exists());
    assert_eq!(o.t.uygulama.geri_alma_verisi_boyutu().await.unwrap(), 0);
    assert_eq!(o.t.uygulama.uygulama_durumu().son_oturum, None);
    assert!(o.t.uygulama.oturumlari_listele().await.unwrap().is_empty());
    let hata = o.t.uygulama.oturum_ac(oturum).await.unwrap_err();
    assert_eq!(hata.kod, HataKodu::OturumYok);
}

#[tokio::test]
async fn yerel_onay_reddedilirse_oturum_silinmez() {
    let o = ortam(Vec::new()).await;
    let oturum = o.yeni_oturum().await;
    o.t.iletisim.onay_ver(false);
    assert!(!o.t.uygulama.oturumu_sil(oturum).await.unwrap());
    assert!(o.kayit(oturum).exists());
    assert_eq!(o.t.uygulama.acik_oturum_sayisi(), 1, "oturum açık kaldı");
}

#[tokio::test]
async fn baska_yerde_acik_kayit_silinmez() {
    let o = ortam(Vec::new()).await;
    let oturum = o.yeni_oturum().await;
    o.t.uygulama.oturumu_kapat(oturum).await.unwrap();
    let _kilit = OturumKaydi::ac(&o.kayit(oturum)).unwrap();
    o.t.iletisim.onay_ver(true);
    let hata = o.t.uygulama.oturumu_sil(oturum).await.unwrap_err();
    assert_eq!(hata.kod, HataKodu::KayitMesgul);
    assert!(o.kayit(oturum).exists());
}

#[tokio::test]
async fn kaydi_olmayan_oturum_silinmez_ve_sorulmaz() {
    let o = ortam(Vec::new()).await;
    let hata =
        o.t.uygulama
            .oturumu_sil(OturumId::yeni())
            .await
            .unwrap_err();
    assert_eq!(hata.kod, HataKodu::OturumYok);
    assert!(o.t.iletisim.sorulanlar().is_empty());
}

#[tokio::test]
async fn gunluk_sayfasi_kayittan() {
    let o = ortam(vec![SahteYanit::metin("merhaba")]).await;
    let oturum = o.yeni_oturum().await;
    let mut rx = o.dinle(oturum);
    o.t.uygulama
        .mesaj_gonder(oturum, "selam".into())
        .await
        .unwrap();
    loop {
        let msg = rx.recv().await.unwrap();
        if let KanalMesaji::Olaylar { zarflar, .. } = msg
            && zarflar
                .iter()
                .any(|z| matches!(z.yuk, Olay::Tamamlandi { .. }))
        {
            break;
        }
    }

    let sayfa = o.t.uygulama.gonderim_gunlugu(oturum, None).await.unwrap();
    assert_eq!(sayfa.satirlar.len(), 1);
    assert_eq!(sayfa.satirlar[0].model.as_deref(), Some("glm-5.3"));
    assert!(!sayfa.satirlar[0].yeni.is_empty());
}

#[tokio::test]
async fn bilinmeyen_oturum_hata() {
    let o = ortam(Vec::new()).await;
    let bilinmeyen = OturumId::yeni();
    let islem = IslemId::yeni();

    let err1 =
        o.t.uygulama
            .gonderim_gunlugu(bilinmeyen, None)
            .await
            .unwrap_err();
    assert_eq!(err1.kod, HataKodu::OturumYok);

    let err2 =
        o.t.uygulama
            .gonderim_ogesi(bilinmeyen, 1)
            .await
            .unwrap_err();
    assert_eq!(err2.kod, HataKodu::OturumYok);

    let err3 =
        o.t.uygulama
            .onizleme_ayarla(bilinmeyen, true)
            .await
            .unwrap_err();
    assert_eq!(err3.kod, HataKodu::OturumYok);

    let err4 =
        o.t.uygulama
            .gonderim_karari(bilinmeyen, islem, Vec::new(), Vec::new())
            .await
            .unwrap_err();
    assert_eq!(err4.kod, HataKodu::OturumYok);
}

#[tokio::test]
async fn oge_metni_yalnizca_kayittaki_sira() {
    let o = ortam(vec![SahteYanit::metin("cevabim")]).await;
    let oturum = o.yeni_oturum().await;
    let mut rx = o.dinle(oturum);
    o.t.uygulama
        .mesaj_gonder(oturum, "ozel_mesaj_12345".into())
        .await
        .unwrap();
    loop {
        let msg = rx.recv().await.unwrap();
        if let KanalMesaji::Olaylar { zarflar, .. } = msg
            && zarflar
                .iter()
                .any(|z| matches!(z.yuk, Olay::Tamamlandi { .. }))
        {
            break;
        }
    }

    let sayfa = o.t.uygulama.gonderim_gunlugu(oturum, None).await.unwrap();
    assert_eq!(sayfa.satirlar.len(), 1);
    let sira = sayfa.satirlar[0]
        .yeni
        .iter()
        .find(|o| o.kaynak == "mesaj:kullanıcı")
        .unwrap()
        .sira
        .unwrap();

    let metin = o.t.uygulama.gonderim_ogesi(oturum, sira).await.unwrap();
    assert_eq!(metin, "ozel_mesaj_12345");

    let err =
        o.t.uygulama
            .gonderim_ogesi(oturum, 999_999)
            .await
            .unwrap_err();
    assert_eq!(err.kod, HataKodu::GecersizGirdi);
}
