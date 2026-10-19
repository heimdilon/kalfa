//! Özetleyicinin deneme defteri: her özetleme denemesi kendi izni ve tek
//! bitişini alır. Kullanımı bilinmeyen bir bitiş sonraki özetlemeyi durdurmaz;
//! izin yoksa ağa çıkılmaz.

use std::path::Path;
use std::sync::Arc;

use kalfa_core::arac::Arac;
use kalfa_core::butce::ButceIslemId;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::oturum::{OturumAyarlari, baslat_kayitla};
use kalfa_core::protokol::{BitisNedeni, Komut, Olay, Zarf};
use kalfa_core::test_destek::*;
use kalfa_evren::AkisOlayi;
use kalfa_evren::tipler::Usage;
use serde_json::{Value, json};

fn ayarlar(kayit_yolu: Option<&Path>) -> OturumAyarlari {
    let mut a = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    a.kayit_yolu = kayit_yolu.map(Path::to_path_buf);
    a.sinirlar.etkin_pencere = 32_000;
    a
}

fn oku() -> Arc<dyn Arac> {
    Arc::new(SahteArac::okuyan("oku").yanit("x".repeat(30_000)))
}

fn mesaj() -> Komut {
    Komut::KullaniciMesaji {
        metin: "dosyaları oku".into(),
    }
}

fn yol(y: &str) -> Value {
    json!({ "yol": y })
}

fn kullanimla(yanit: SahteYanit, istem: u64) -> SahteYanit {
    match yanit {
        SahteYanit::Akis(mut olaylar) => {
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

fn ozetlendi_sayisi(d: &Surucu) -> usize {
    d.olaylar()
        .iter()
        .filter(|o| matches!(o, Olay::OturumOzetlendi { .. }))
        .count()
}

fn ozet_bitisleri(kayitlar: &[Zarf<KayitOgesi>]) -> Vec<(ButceIslemId, bool)> {
    let ozet: Vec<ButceIslemId> = kayitlar
        .iter()
        .filter_map(|z| match &z.yuk {
            KayitOgesi::ButceIstegiBasladi { islem, amac } if amac == "özetleme" => Some(*islem),
            _ => None,
        })
        .collect();
    kayitlar
        .iter()
        .filter_map(|z| match &z.yuk {
            KayitOgesi::ButceIstegiBitti { islem, kullanim } if ozet.contains(islem) => {
                Some((*islem, kullanim.is_some()))
            }
            _ => None,
        })
        .collect()
}

/// İlk compact kullanım vermeden biter; sonraki compact kartsız sürer ve her
/// biri tek bitiş kaydı alır.
#[tokio::test]
async fn kullanimsiz_ozet_bitisi_sonraki_adaylari_durdurmaz() {
    let dizin = tempfile::tempdir().unwrap();
    let kayit_yolu = dizin.path().join("oturum.jsonl");
    let s = SahteSaglayici::yeni(vec![
        kullanimla(SahteYanit::arac("oku", yol("a")), 30_000),
        kullanimla(SahteYanit::arac("oku", yol("b")), 30_000),
        kullanimla(SahteYanit::arac("oku", yol("c")), 30_000),
        SahteYanit::metin("tamam"),
    ]);
    s.sohbet_ekle(SahteSohbet::KullanimsizMetin("özet 1".into()));
    s.sohbet_ekle(SahteSohbet::Metin("özet 2".into()));
    let mut d = test_oturumu(
        test_baglami(s.clone(), vec![oku()], sanal_siniflandirici()),
        ayarlar(Some(&kayit_yolu)),
    );
    d.gonder(mesaj()).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(s.sohbet_istekleri().len(), 2);
    assert_eq!(ozetlendi_sayisi(&d), 2);
    d.kapat().await;

    let kayitlar = OturumKaydi::oku(&kayit_yolu).unwrap();
    let bitisler = ozet_bitisleri(&kayitlar);
    assert_eq!(bitisler.len(), 2, "her özetleme denemesi tek bitiş alır");
    assert_eq!(
        bitisler.iter().filter(|(_, bilinen)| !bilinen).count(),
        1,
        "kullanımsız biten tek deneme bilinmeyendir"
    );
}

/// Kayıt hatası yüzünden özetleme izni verilemezse özetleme ağa çıkmaz.
#[tokio::test]
async fn ozet_izni_yokken_cikti_aynen_kalir_yer_tutucu_konmaz() {
    let s = SahteSaglayici::yeni(vec![
        kullanimla(SahteYanit::arac("oku", yol("a")), 30_000),
        kullanimla(SahteYanit::arac("oku", yol("b")), 30_000),
        SahteYanit::metin("tamam"),
    ]);
    s.sohbet_ekle(SahteSohbet::Metin("özet".into()));
    // İki ana isteğin başlangıç ve bitişi (4 kalıcı yazım) geçer; özetlemenin
    // ilk başlangıç yazımı düşer ve izin verilmez.
    let kayit = BozulanHedef::kalicidan_sonra_bozuk(4);
    let (tutamac, olaylar) = baslat_kayitla(
        ayarlar(None),
        test_baglami(s.clone(), vec![oku()], sanal_siniflandirici()),
        kayit,
    )
    .unwrap();
    let mut d = Surucu::yeni(tutamac, olaylar);
    d.gonder(mesaj()).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Hata);
    assert!(
        s.sohbet_istekleri().is_empty(),
        "izin yokken özetleme ağa çıkmamalı"
    );
    assert_eq!(ozetlendi_sayisi(&d), 0);
}
