//! 5a bağlam yönetimi: eşik, özetleme, yer tutucu, iptal, günlük ve kurtarma.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use kalfa_core::arac::Arac;
use kalfa_core::gonderim::{IstekSonucu, oge_metni};
use kalfa_core::kayit::{KayitOgesi, OturumKaydi, OzetYontemi};
use kalfa_core::kurtarma::kurtar;
use kalfa_core::oturum::OturumAyarlari;
use kalfa_core::protokol::{BitisNedeni, Komut, Olay};
use kalfa_core::test_destek::*;
use kalfa_evren::tipler::{ChatMessage, Role, Usage};
use kalfa_evren::{AkisOlayi, EvrenError};
use serde_json::json;

const OZETLEYICI: &str = "qwen3.8-flash-next";

/// Eşik ve hedef 1 token, son 1 araç çıktısı korunur.
fn ayarlar(dizin: &Path) -> (OturumAyarlari, PathBuf) {
    let yol = dizin.join("oturum.jsonl");
    let mut a = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    a.kayit_yolu = Some(yol.clone());
    a.sinirlar.baglam_esigi = 1;
    a.sinirlar.baglam_hedefi = 1;
    a.sinirlar.korunan_arac_ciktisi = 1;
    (a, yol)
}

fn oku() -> Arc<SahteArac> {
    Arc::new(SahteArac::okuyan("oku").yanit("x".repeat(3000)))
}

fn oku_cagrisi(yol: &str) -> SahteYanit {
    SahteYanit::arac("oku", json!({ "yol": yol }))
}

fn mesaj() -> Komut {
    Komut::KullaniciMesaji {
        metin: "iki dosyayı oku".into(),
    }
}

fn surucu(s: &Arc<SahteSaglayici>, ayar: OturumAyarlari) -> Surucu {
    let araclar: Vec<Arc<dyn Arac>> = vec![oku()];
    test_oturumu(
        test_baglami(s.clone(), araclar, sanal_siniflandirici()),
        ayar,
    )
}

/// İki okuma ve son yanıt. Üçüncü istekten önce ilk çıktı gönderilmiş,
/// ikincisi gönderilmemiş ve korunuyor.
fn iki_okuma() -> Arc<SahteSaglayici> {
    SahteSaglayici::yeni(vec![
        oku_cagrisi("a.txt"),
        oku_cagrisi("b.txt"),
        SahteYanit::metin("tamam"),
    ])
}

fn arac_mesajlari(m: &[ChatMessage]) -> Vec<&ChatMessage> {
    m.iter().filter(|x| x.role == Role::Tool).collect()
}

fn icerik(m: &ChatMessage) -> &str {
    m.content.as_deref().unwrap_or_default()
}

fn ozetlendi(d: &Surucu) -> Option<(u32, u32)> {
    d.olaylar().iter().find_map(|o| match o {
        Olay::BaglamOzetlendi {
            adet, yer_tutucu, ..
        } => Some((*adet, *yer_tutucu)),
        _ => None,
    })
}

async fn tur(s: &Arc<SahteSaglayici>, ayar: OturumAyarlari) -> (Surucu, BitisNedeni) {
    let mut d = surucu(s, ayar);
    d.gonder(mesaj()).await;
    let neden = d.tur_sonu().await;
    (d, neden)
}

#[tokio::test]
async fn esik_asilinca_en_eski_cikti_ozetlenir() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, _) = ayarlar(dizin.path());
    let s = iki_okuma();
    s.sohbet_ekle(SahteSohbet::Metin("a.txt: 3000 tane x".into()));
    let (d, neden) = tur(&s, ayar).await;
    assert_eq!(neden, BitisNedeni::Normal);

    let istekler = s.istekler();
    assert_eq!(istekler.len(), 3);
    let onceki = arac_mesajlari(&istekler[1].messages);
    let sonraki = arac_mesajlari(&istekler[2].messages);
    assert_eq!((onceki.len(), sonraki.len()), (1, 2));

    // Özetleyiciye yalnız ilk çıktı, bağlamdaki hâliyle gitti.
    let ozet = s.sohbet_istekleri();
    assert_eq!(ozet.len(), 1);
    assert_eq!(ozet[0].model, OZETLEYICI);
    assert_eq!(ozet[0].max_tokens, Some(2048));
    assert_eq!(icerik(&ozet[0].messages[1]), icerik(onceki[0]));

    // Özet araç rolünde, aynı çağrı kimliğiyle ve özet: etiketiyle.
    assert_eq!(sonraki[0].tool_call_id, onceki[0].tool_call_id);
    assert!(
        icerik(sonraki[0]).starts_with("<arac_sonucu kaynak=\"özet:sahte:oku\" sinir=\""),
        "{}",
        icerik(sonraki[0])
    );
    assert!(icerik(sonraki[0]).contains("a.txt: 3000 tane x"));
    assert!(!icerik(sonraki[0]).contains(&"x".repeat(3000)));
    // Korunan son çıktı ve araç dışı mesajlar değişmedi.
    assert!(icerik(sonraki[1]).contains(&"x".repeat(3000)));
    let diger = |m: &[ChatMessage]| -> Vec<ChatMessage> {
        m.iter().filter(|x| x.role != Role::Tool).cloned().collect()
    };
    let (d1, d2) = (diger(&istekler[1].messages), diger(&istekler[2].messages));
    assert_eq!(d2[..d1.len()], d1[..]);

    assert_eq!(ozetlendi(&d), Some((1, 0)));
}

#[tokio::test]
async fn esigin_altinda_ozetleme_istegi_yok() {
    let dizin = tempfile::tempdir().unwrap();
    let (mut ayar, _) = ayarlar(dizin.path());
    ayar.sinirlar = Default::default();
    let s = iki_okuma();
    let (d, neden) = tur(&s, ayar).await;
    assert_eq!(neden, BitisNedeni::Normal);
    assert!(s.sohbet_istekleri().is_empty());
    let istekler = s.istekler();
    assert_eq!(
        icerik(arac_mesajlari(&istekler[2].messages)[0]),
        icerik(arac_mesajlari(&istekler[1].messages)[0])
    );
    assert_eq!(ozetlendi(&d), None);
}

fn kullanimla(yanit: SahteYanit, istem: u64) -> SahteYanit {
    match yanit {
        SahteYanit::Akis(mut olaylar) => {
            // Sahte yanıtın varsayılan kullanımının yerine geçer: son kullanım olayı sayılır.
            olaylar.retain(|o| !matches!(o, Ok(AkisOlayi::Kullanim(_))));
            olaylar.insert(
                0,
                Ok(AkisOlayi::Kullanim(Usage {
                    prompt_tokens: istem,
                    completion_tokens: 1,
                    total_tokens: istem + 1,
                    completion_tokens_details: None,
                    evren: None,
                })),
            );
            SahteYanit::Akis(olaylar)
        }
        diger => diger,
    }
}

#[tokio::test]
async fn kullanim_orani_varsayilan_esigi_tetikler() {
    let dizin = tempfile::tempdir().unwrap();
    let (mut ayar, _) = ayarlar(dizin.path());
    ayar.sinirlar.baglam_esigi = 64_000;
    ayar.sinirlar.baglam_hedefi = 32_000;
    // İlk iki yanıt, birkaç yüz baytlık istemin 10 milyon token tuttuğunu söyler;
    // oran ikinci yanıtta da yüksek kalır ki eşik üçüncü istekten önce aşılsın.
    let s = SahteSaglayici::yeni(vec![
        kullanimla(oku_cagrisi("a.txt"), 10_000_000),
        kullanimla(oku_cagrisi("b.txt"), 10_000_000),
        SahteYanit::metin("tamam"),
    ]);
    s.sohbet_ekle(SahteSohbet::Metin("özet".into()));
    let (_, neden) = tur(&s, ayar).await;
    assert_eq!(neden, BitisNedeni::Normal);
    assert_eq!(s.sohbet_istekleri().len(), 1);
}

async fn yer_tutucu_bekle(yanit: SahteSohbet) {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, yol) = ayarlar(dizin.path());
    let s = iki_okuma();
    s.sohbet_ekle(yanit);
    let mut d = surucu(&s, ayar);
    d.gonder(mesaj()).await;
    let neden = d.tur_sonu().await;
    assert_eq!(neden, BitisNedeni::Normal);
    let istekler = s.istekler();
    let ozgun = icerik(arac_mesajlari(&istekler[1].messages)[0]).len();
    assert_eq!(
        icerik(arac_mesajlari(&istekler[2].messages)[0]),
        format!("[eski araç çıktısı kaldırıldı: {ozgun} bayt; gerekirse aracı yeniden çalıştırın]")
    );
    assert_eq!(ozetlendi(&d), Some((0, 1)));
    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    assert!(kayitlar.iter().any(|z| matches!(
        &z.yuk,
        KayitOgesi::BaglamOzeti {
            yontem: OzetYontemi::YerTutucu,
            ..
        }
    )));
}

#[tokio::test]
async fn ozetleyici_hatasinda_yer_tutucu_konur() {
    yer_tutucu_bekle(SahteSohbet::Hata(EvrenError::Sunucu {
        durum: 500,
        kod: None,
        mesaj: "iç hata".into(),
    }))
    .await;
}

#[tokio::test]
async fn bos_ozet_yer_tutucu_olur() {
    yer_tutucu_bekle(SahteSohbet::Metin("  \n ".into())).await;
}

#[tokio::test]
async fn hiz_sinirinda_bir_kez_yeniden_denenir() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, _) = ayarlar(dizin.path());
    let s = iki_okuma();
    s.sohbet_ekle(SahteSohbet::Hata(EvrenError::HizSiniri {
        tekrar_dene: Some(Duration::from_millis(10)),
    }));
    s.sohbet_ekle(SahteSohbet::Metin("ikinci denemede özet".into()));
    let (d, neden) = tur(&s, ayar).await;
    assert_eq!(neden, BitisNedeni::Normal);
    assert_eq!(s.sohbet_istekleri().len(), 2);
    assert!(icerik(arac_mesajlari(&s.istekler()[2].messages)[0]).contains("ikinci denemede özet"));
    assert_eq!(ozetlendi(&d), Some((1, 0)));
}

#[tokio::test]
async fn ozetleme_iptal_edilebilir() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, _) = ayarlar(dizin.path());
    let s = iki_okuma();
    s.sohbet_ekle(SahteSohbet::Gecikme(Duration::from_secs(30)));
    let mut d = surucu(&s, ayar);
    d.gonder(mesaj()).await;
    for _ in 0..300 {
        if !s.sohbet_istekleri().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(s.sohbet_istekleri().len(), 1, "özetleme başlamadı");
    assert!(d.tutamac.iptal_et().is_some());
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);
    assert_eq!(s.istekler().len(), 2);
}

#[tokio::test]
async fn onizleme_acikken_ozetleme_onizleme_beklemez() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, _) = ayarlar(dizin.path());
    let s = iki_okuma();
    s.sohbet_ekle(SahteSohbet::Metin("özet".into()));
    let mut d = surucu(&s, ayar);
    d.gonder(Komut::OnizlemeAyarla { acik: true }).await;
    d.gonder(mesaj()).await;
    for _ in 0..2 {
        let islem = match d
            .bekle(|o| matches!(o, Olay::GonderimOnizlemesi { .. }))
            .await
            .yuk
        {
            Olay::GonderimOnizlemesi { islem, .. } => islem,
            diger => unreachable!("{diger:?}"),
        };
        d.gonder(Komut::GonderimKarari {
            islem,
            gonderilmeyecekler: Vec::new(),
            hep_gonderilecekler: Vec::new(),
        })
        .await;
    }
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(s.sohbet_istekleri().len(), 1);
    let onizlemeler = d
        .olaylar()
        .iter()
        .filter(|o| matches!(o, Olay::GonderimOnizlemesi { .. }))
        .count();
    assert_eq!(onizlemeler, 2);
}

#[tokio::test]
async fn gunluk_ve_kurtarma_ozeti_tasir() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, yol) = ayarlar(dizin.path());
    let s = iki_okuma();
    s.sohbet_ekle(SahteSohbet::Metin("özet".into()));
    let (d, neden) = tur(&s, ayar).await;
    assert_eq!(neden, BitisNedeni::Normal);
    d.kapat().await;

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    // Özetleme isteği günlükte kendi amacıyla, yeni öğesiz.
    let ozet_istegi = kayitlar
        .iter()
        .find_map(|z| match &z.yuk {
            KayitOgesi::Istek {
                amac,
                model,
                yeni,
                onceki,
                sonuc,
                ..
            } if amac == "özetleme" => Some((model.clone(), yeni.len(), *onceki, sonuc.clone())),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        ozet_istegi,
        (Some(OZETLEYICI.to_string()), 0, 1, IstekSonucu::Basarili)
    );
    // Özet, sonraki ana istekte yeni öğe olarak listelendi.
    let ozet_sira = kayitlar
        .iter()
        .find_map(|z| matches!(z.yuk, KayitOgesi::BaglamOzeti { .. }).then_some(z.sira))
        .unwrap();
    let son_ana = kayitlar
        .iter()
        .rev()
        .find_map(|z| match &z.yuk {
            KayitOgesi::Istek { amac, yeni, .. } if amac == "ana" => Some(yeni.clone()),
            _ => None,
        })
        .unwrap();
    assert!(
        son_ana
            .iter()
            .any(|o| o.kaynak == "özet:sahte:oku" && o.sira == Some(ozet_sira)),
        "{son_ana:?}"
    );
    // Kurtarılan bağlam, özetli son istekle bayt bayt aynı.
    let durum = kurtar(&kayitlar).unwrap();
    let son = &s.istekler()[2].messages;
    assert_eq!(durum.mesajlar[..son.len() - 1], son[1..]);
    assert_eq!(durum.gonderilen, son.len() - 1);
    assert!(durum.bekleyen_ozetler.is_empty());
    let ozet_icerigi = icerik(arac_mesajlari(son)[0]).to_string();
    assert_eq!(oge_metni(&kayitlar, ozet_sira), Some(ozet_icerigi));
}

fn asim() -> SahteYanit {
    SahteYanit::Hata(EvrenError::BaglamAsimi {
        mesaj: "pencere aşıldı".into(),
    })
}

const ASIM_ILETISI: &str = "Bağlam modelin penceresini aşıyor";

#[tokio::test]
async fn baglam_asiminda_siki_ozetleyip_bir_kez_yeniden_dener() {
    let dizin = tempfile::tempdir().unwrap();
    let (mut ayar, _) = ayarlar(dizin.path());
    ayar.sinirlar = Default::default();
    let s = SahteSaglayici::yeni(vec![
        oku_cagrisi("a.txt"),
        oku_cagrisi("b.txt"),
        asim(),
        SahteYanit::metin("tamam"),
    ]);
    s.sohbet_ekle(SahteSohbet::Metin("özet a".into()));
    s.sohbet_ekle(SahteSohbet::Metin("özet b".into()));
    let (d, neden) = tur(&s, ayar).await;
    assert_eq!(neden, BitisNedeni::Normal);
    let istekler = s.istekler();
    assert_eq!(istekler.len(), 4);
    // Varsayılan eşikte üçüncü istekten önce özetleme yoktu; aşımdan sonra
    // korunan çıktılar dahil ikisi de özetlendi.
    assert_eq!(s.sohbet_istekleri().len(), 2);
    for m in arac_mesajlari(&istekler[3].messages) {
        assert!(
            icerik(m).starts_with("<arac_sonucu kaynak=\"özet:"),
            "{}",
            icerik(m)
        );
    }
    assert_eq!(ozetlendi(&d), Some((2, 0)));
}

#[tokio::test]
async fn ikinci_asimda_tur_hatayla_biter() {
    let dizin = tempfile::tempdir().unwrap();
    let (mut ayar, _) = ayarlar(dizin.path());
    ayar.sinirlar = Default::default();
    let s = SahteSaglayici::yeni(vec![oku_cagrisi("a.txt"), asim(), asim()]);
    s.sohbet_ekle(SahteSohbet::Metin("özet".into()));
    let (d, neden) = tur(&s, ayar).await;
    assert_eq!(neden, BitisNedeni::Hata);
    assert_eq!(s.istekler().len(), 3);
    assert_eq!(s.sohbet_istekleri().len(), 1);
    assert!(
        d.hatalar().iter().any(|h| h.starts_with(ASIM_ILETISI)),
        "{:?}",
        d.hatalar()
    );
}

#[tokio::test]
async fn aday_yoksa_asim_hemen_hata() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, _) = ayarlar(dizin.path());
    let s = SahteSaglayici::yeni(vec![asim()]);
    let (d, neden) = tur(&s, ayar).await;
    assert_eq!(neden, BitisNedeni::Hata);
    assert_eq!(s.istekler().len(), 1);
    assert!(s.sohbet_istekleri().is_empty());
    assert!(
        d.hatalar().iter().any(|h| h.starts_with(ASIM_ILETISI)),
        "{:?}",
        d.hatalar()
    );
}

// İnceleme düzeltmeleri (5a son inceleme).

#[tokio::test]
async fn ozet_kalfa_uyarilarini_korur() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, _) = ayarlar(dizin.path());
    let s = iki_okuma();
    s.sohbet_ekle(SahteSohbet::Metin("özet".into()));
    let arac = Arc::new(
        SahteArac::okuyan("oku")
            .yanit("x".repeat(3000))
            .guvenlik_uyarisi("görünmez karakter var"),
    );
    let araclar: Vec<Arc<dyn Arac>> = vec![arac];
    let mut d = test_oturumu(
        test_baglami(s.clone(), araclar, sanal_siniflandirici()),
        ayar,
    );
    d.gonder(mesaj()).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    let ozet = arac_mesajlari(&s.istekler()[2].messages)[0].clone();
    assert!(
        icerik(&ozet).starts_with("<arac_sonucu kaynak=\"özet:"),
        "{}",
        icerik(&ozet)
    );
    assert!(
        icerik(&ozet).ends_with("\n[güvenlik] görünmez karakter var"),
        "{}",
        icerik(&ozet)
    );
}

#[tokio::test]
async fn gunluk_sinirda_ozgun_cikti_kalir() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, yol) = ayarlar(dizin.path());
    let s = iki_okuma();
    s.sohbet_ekle(SahteSohbet::Hata(EvrenError::GunlukSinir {
        sifirlanma: None,
    }));
    let (d, neden) = tur(&s, ayar).await;
    assert_eq!(neden, BitisNedeni::Normal);
    let istekler = s.istekler();
    assert_eq!(
        icerik(arac_mesajlari(&istekler[2].messages)[0]),
        icerik(arac_mesajlari(&istekler[1].messages)[0])
    );
    assert_eq!(ozetlendi(&d), None);
    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    assert!(
        !kayitlar
            .iter()
            .any(|z| matches!(z.yuk, KayitOgesi::BaglamOzeti { .. }))
    );
}

#[tokio::test]
async fn sart_gerekirse_ozetleme_durur_ve_sart_kabul_edilmez() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, _) = ayarlar(dizin.path());
    let s = iki_okuma();
    s.sohbet_ekle(SahteSohbet::Hata(EvrenError::SartKabulGerekli {
        surum: Some(1),
    }));
    let (d, neden) = tur(&s, ayar).await;
    assert_eq!(neden, BitisNedeni::Normal);
    let istekler = s.istekler();
    assert_eq!(
        icerik(arac_mesajlari(&istekler[2].messages)[0]),
        icerik(arac_mesajlari(&istekler[1].messages)[0])
    );
    assert!(s.kabul_edilen_surumler().is_empty());
    assert_eq!(ozetlendi(&d), None);
}

#[tokio::test(start_paused = true)]
async fn zaman_asiminda_yer_tutucu_konur() {
    let dizin = tempfile::tempdir().unwrap();
    let (ayar, _) = ayarlar(dizin.path());
    let s = iki_okuma();
    s.sohbet_ekle(SahteSohbet::Gecikme(Duration::from_secs(60)));
    let mut d = surucu(&s, ayar).zaman_asimi(Duration::from_secs(120));
    d.gonder(mesaj()).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert!(
        icerik(arac_mesajlari(&s.istekler()[2].messages)[0])
            .starts_with("[eski araç çıktısı kaldırıldı:")
    );
    assert_eq!(ozetlendi(&d), Some((0, 1)));
}

/// Codex incelemesi (#2): durdurucu bir hata geldiğinde başlamış özetleme
/// istekleri bırakılmaz, kayda yazılır; durduktan sonra başarısızlık yer
/// tutucuya dönmez.
#[tokio::test]
async fn durdurucu_hatada_baslamis_istekler_kaydedilir() {
    let dizin = tempfile::tempdir().unwrap();
    let (mut ayar, yol) = ayarlar(dizin.path());
    ayar.sinirlar.baglam_esigi = 64_000;
    ayar.sinirlar.baglam_hedefi = 32_000;
    // Eşik ancak üçüncü yanıtın kullanımıyla aşılır.
    let s = SahteSaglayici::yeni(vec![
        oku_cagrisi("a.txt"),
        oku_cagrisi("b.txt"),
        kullanimla(oku_cagrisi("c.txt"), 10_000_000),
        SahteYanit::metin("tamam"),
    ]);
    // Dördüncü istekten önce iki aday (a, b) aynı anda özetlenir: a'nın
    // isteği yavaş (başlamış, sürüyor), b'ninki hemen günlük sınıra çarpar.
    s.sohbet_ekle(SahteSohbet::Gecikme(Duration::from_millis(50)));
    s.sohbet_ekle(SahteSohbet::Hata(EvrenError::GunlukSinir {
        sifirlanma: None,
    }));
    let (_, neden) = tur(&s, ayar).await;
    assert_eq!(neden, BitisNedeni::Normal);
    assert_eq!(s.sohbet_istekleri().len(), 2);
    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    let ozetleme = kayitlar
        .iter()
        .filter(|z| matches!(&z.yuk, KayitOgesi::Istek { amac, .. } if amac == "özetleme"))
        .count();
    assert_eq!(ozetleme, 2, "başlamış her özetleme isteği günlükte olmalı");
    assert!(
        !kayitlar
            .iter()
            .any(|z| matches!(z.yuk, KayitOgesi::BaglamOzeti { .. }))
    );
}
