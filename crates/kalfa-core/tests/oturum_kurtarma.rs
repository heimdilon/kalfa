use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kalfa_core::arac::Arac;
use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::{CagriId, OturumId};
use kalfa_core::kurtarma::kurtar;
use kalfa_core::oturum::{OturumAyarlari, OturumHatasi, devam_et};
use kalfa_core::protokol::{AracDurumu, BitisNedeni, Komut, Olay, SEMA_SURUMU, Zarf};
use kalfa_core::test_destek::*;
use kalfa_evren::tipler::{ChatMessage, FunctionCall, Role, ToolCall};

fn ayarlar() -> OturumAyarlari {
    OturumAyarlari::yeni("bu-model-kullanilmaz", Path::new("calisma"))
}

/// Yarım kalmış bir yazma içeren kayıt: model `yaz` çağırdı, "başladı" yazıldı,
/// Kalfa sonuç gelmeden kapandı.
fn yarim_kayit(dizin: &Path) -> (PathBuf, CagriId) {
    let yol = dizin.join("oturum.jsonl");
    let oturum = OturumId::yeni();
    let z = |sira, yuk| Zarf {
        sema: SEMA_SURUMU,
        oturum,
        tur: None,
        sira,
        yuk,
    };
    let cagri = CagriId::yeni();
    let tc = ToolCall {
        id: "m1".into(),
        kind: "function".into(),
        function: FunctionCall {
            name: "yaz".into(),
            arguments: "{\"yol\":\"a.txt\"}".into(),
        },
    };
    let mut k = OturumKaydi::olustur(&yol).unwrap();
    k.yaz(&z(
        1,
        KayitOgesi::OturumAcildi {
            model: "glm-5.3".into(),
            kip: Kip::Serbest,
            calisma_alani: None,
        },
    ))
    .unwrap();
    k.yaz(&z(
        2,
        KayitOgesi::Mesaj {
            mesaj: ChatMessage::user("a.txt'ye yaz"),
            kaynak: None,
        },
    ))
    .unwrap();
    k.yaz(&z(
        3,
        KayitOgesi::Mesaj {
            mesaj: ChatMessage::assistant(None, vec![tc]),
            kaynak: None,
        },
    ))
    .unwrap();
    k.yaz(&z(
        4,
        KayitOgesi::AracBasladi {
            cagri,
            model_cagri_id: "m1".into(),
            arac: "yaz".into(),
            yedekler: Vec::new(),
        },
    ))
    .unwrap();
    (yol, cagri)
}

#[tokio::test]
async fn yarim_kalan_yazma_yeniden_calistirilmaz() {
    let dizin = tempfile::tempdir().unwrap();
    let (yol, cagri) = yarim_kayit(dizin.path());
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("devam ediyorum")]);
    let araclar: Vec<Arc<dyn Arac>> = vec![yaz.clone()];
    let (t, o, durum) = devam_et(
        &yol,
        ayarlar(),
        test_baglami(s.clone(), araclar, sanal_siniflandirici()),
    )
    .unwrap();
    assert_eq!(durum.kip, Kip::Siki, "Serbest kip geri gelmez");
    assert_eq!(durum.model, "glm-5.3");
    assert_eq!(durum.bilinmeyenler.len(), 1);

    let mut d = Surucu::yeni(t, o);
    let z = d
        .bekle(|o| {
            matches!(
                o,
                Olay::AracSonucu {
                    durum: AracDurumu::SonucuBilinmiyor,
                    ..
                }
            )
        })
        .await;
    assert!(matches!(
        &z.yuk,
        Olay::AracSonucu { cagri: c, .. } if *c == cagri
    ));
    assert!(z.sira > 4, "sıra kaydın devamından sürer");

    d.gonder(Komut::KullaniciMesaji {
        metin: "devam".into(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    let m = &s.istekler()[0].messages;
    assert_eq!(s.istekler()[0].model, "glm-5.3");
    assert_eq!(m[3].role, Role::Tool);
    assert_eq!(m[3].tool_call_id.as_deref(), Some("m1"));
    assert!(
        m[3].content
            .as_deref()
            .unwrap()
            .contains("sonucu bilinmiyor")
    );
    assert_eq!(m[4].content.as_deref(), Some("devam"));
    assert_eq!(
        yaz.cagri_sayisi(),
        0,
        "yarım kalan yazma yeniden çalıştırılmaz"
    );

    // İkinci kurtarma: tamamlayıcılar kayda yazıldığı için bağlam düzeni korunur.
    let ikinci = kurtar(&OturumKaydi::oku(&yol).unwrap()).unwrap();
    assert!(ikinci.bilinmeyenler.is_empty());
    assert!(ikinci.tamamlayicilar.is_empty());
    assert_eq!(ikinci.mesajlar[2].tool_call_id.as_deref(), Some("m1"));
    assert_eq!(ikinci.mesajlar[3].content.as_deref(), Some("devam"));
}

#[tokio::test]
async fn cokmus_kayittan_devam_edilir_ve_kayit_yeniden_acilabilir() {
    let dizin = tempfile::tempdir().unwrap();
    let (yol, _) = yarim_kayit(dizin.path());
    // Çökme: son satır yarım kaldı.
    let mut ham = std::fs::OpenOptions::new().append(true).open(&yol).unwrap();
    Write::write_all(&mut ham, b"{\"sema\":1,\"oturum\":\"").unwrap();
    drop(ham);

    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("tamam")]);
    let araclar: Vec<Arc<dyn Arac>> = vec![Arc::new(SahteArac::yazan("yaz"))];
    let (t, o, _) = devam_et(
        &yol,
        ayarlar(),
        test_baglami(s, araclar, sanal_siniflandirici()),
    )
    .unwrap();
    let mut d = Surucu::yeni(t, o);
    d.gonder(Komut::KullaniciMesaji {
        metin: "devam".into(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);

    // Yarım satır kesildiği için yeni kayıtlar ona yapışmadı; kayıt yeniden okunur.
    let ikinci = kurtar(&OturumKaydi::oku(&yol).expect("onarılmış kayıt okunabilmeli")).unwrap();
    assert!(
        ikinci
            .mesajlar
            .iter()
            .any(|m| m.content.as_deref() == Some("tamam"))
    );
}

#[tokio::test]
async fn olmayan_kayittan_devam_edilemez() {
    let dizin = tempfile::tempdir().unwrap();
    let s = SahteSaglayici::yeni(vec![]);
    let sonuc = devam_et(
        &dizin.path().join("yok.jsonl"),
        ayarlar(),
        test_baglami(s, vec![], sanal_siniflandirici()),
    );
    assert!(matches!(sonuc, Err(OturumHatasi::Kayit(_))));
}
