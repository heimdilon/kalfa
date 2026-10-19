//! Özetleyicinin oturum bütçesi: her özetleme denemesi kendi izni ve tek
//! bitişini alır. Kullanımı bilinmeyen bir bitiş sonraki adayı durdurmaz;
//! izin yoksa çıktı olduğu gibi kalır (yer tutucu konmaz).

use std::path::Path;
use std::sync::Arc;

use kalfa_core::arac::Arac;
use kalfa_core::butce::ButceIslemId;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi, OzetYontemi};
use kalfa_core::oturum::{OturumAyarlari, baslat_kayitla};
use kalfa_core::protokol::{BitisNedeni, Komut, Olay, Zarf};
use kalfa_core::test_destek::*;
use serde_json::{Value, json};

/// Eşik ve hedef 1 token, son çıktı korunur: her model turunda eski çıktılar özetlenir.
fn ayarlar(kayit_yolu: Option<&Path>) -> OturumAyarlari {
    let mut a = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    a.kayit_yolu = kayit_yolu.map(Path::to_path_buf);
    a.sinirlar.baglam_esigi = 1;
    a.sinirlar.baglam_hedefi = 1;
    a.sinirlar.korunan_arac_ciktisi = 1;
    a
}

fn oku() -> Arc<dyn Arac> {
    Arc::new(SahteArac::okuyan("oku").yanit("x".repeat(3000)))
}

fn mesaj() -> Komut {
    Komut::KullaniciMesaji {
        metin: "beş dosyayı oku".into(),
    }
}

fn yol(y: &str) -> Value {
    json!({ "yol": y })
}

fn ozetlendi(d: &Surucu) -> Option<(u32, u32)> {
    d.olaylar().iter().find_map(|o| match o {
        Olay::BaglamOzetlendi {
            adet, yer_tutucu, ..
        } => Some((*adet, *yer_tutucu)),
        _ => None,
    })
}

/// Beş çıktılı tek model yanıtı, bir çıktılı ikinci yanıt, sonra metin. Üçüncü
/// modelde ilk beş çıktı özetlenmeye adaydır; korunan son çıktı değildir.
fn besli_senaryo() -> Arc<SahteSaglayici> {
    SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![
            ("oku", yol("a")),
            ("oku", yol("b")),
            ("oku", yol("c")),
            ("oku", yol("d")),
            ("oku", yol("e")),
        ]),
        SahteYanit::arac("oku", yol("f")),
        SahteYanit::metin("tamam"),
    ])
}

fn besli_ozetler(s: &Arc<SahteSaglayici>) {
    // Sırayla: ilk özetleme kullanım bildirmez (bilinmeyen), diğerleri bildirir.
    s.sohbet_ekle(SahteSohbet::KullanimsizMetin("özet 1".into()));
    s.sohbet_ekle(SahteSohbet::Metin("özet 2".into()));
    s.sohbet_ekle(SahteSohbet::Metin("özet 3".into()));
    s.sohbet_ekle(SahteSohbet::Metin("özet 4".into()));
    s.sohbet_ekle(SahteSohbet::Metin("özet 5".into()));
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

/// Beş özetlemenin ilki kullanım vermeden biter; kalan dört aday kartsız
/// sürer ve her biri tek bitiş kaydı alır.
#[tokio::test]
async fn kullanimsiz_ozet_bitisi_sonraki_adaylari_durdurmaz() {
    let dizin = tempfile::tempdir().unwrap();
    let kayit_yolu = dizin.path().join("oturum.jsonl");
    let s = besli_senaryo();
    besli_ozetler(&s);
    let mut d = test_oturumu(
        test_baglami(s.clone(), vec![oku()], sanal_siniflandirici()),
        ayarlar(Some(&kayit_yolu)),
    );
    d.gonder(mesaj()).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(
        s.sohbet_istekleri().len(),
        5,
        "her aday tam bir kez özetlenir"
    );
    assert_eq!(ozetlendi(&d), Some((5, 0)));
    d.kapat().await;

    let kayitlar = OturumKaydi::oku(&kayit_yolu).unwrap();
    let bitisler = ozet_bitisleri(&kayitlar);
    assert_eq!(bitisler.len(), 5, "her özetleme denemesi tek bitiş alır");
    assert_eq!(
        bitisler.iter().filter(|(_, bilinen)| !bilinen).count(),
        1,
        "kullanımsız biten tek deneme bilinmeyendir"
    );
    assert!(
        !kayitlar.iter().any(|z| matches!(
            z.yuk,
            KayitOgesi::BaglamOzeti {
                yontem: OzetYontemi::YerTutucu,
                ..
            }
        )),
        "bilinmeyen tüketim yer tutucu üretmemeli"
    );
}

/// Kayıt hatası yüzünden özetleme izni verilemezse özetleme ağa çıkmaz ve
/// yer tutucu konmaz.
#[tokio::test]
async fn ozet_izni_yokken_cikti_aynen_kalir_yer_tutucu_konmaz() {
    let s = besli_senaryo();
    besli_ozetler(&s);
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
    assert_eq!(ozetlendi(&d), None);
}
