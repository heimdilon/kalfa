use std::path::Path;
use std::sync::Arc;

use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::{CagriId, IslemId, OturumId, TurId};
use kalfa_core::kurtarma::{ONIZLEME_KAPANISI, kurtar};
use kalfa_core::oturum::{OturumAyarlari, baslat};
use kalfa_core::protokol::{
    AracDurumu, BitisNedeni, Komut, Olay, OnizlemeNedeni, OnizlemeOgesi, SEMA_SURUMU, Zarf,
};
use kalfa_core::test_destek::{
    SahteArac, SahteSaglayici, SahteYanit, Surucu, sanal_siniflandirici, test_baglami,
};
use kalfa_evren::tipler::ToolCall;
use serde_json::json;

fn selam() -> Komut {
    Komut::KullaniciMesaji {
        metin: "selam".into(),
    }
}

fn kayitli(d: &Path) -> (OturumAyarlari, std::path::PathBuf) {
    let yol = d.join("oturum.jsonl");
    let mut a = OturumAyarlari::yeni("glm-5.3", d);
    a.kayit_yolu = Some(yol.clone());
    (a, yol)
}

fn test_oturumu(baglam: kalfa_core::oturum::OturumBaglami, ayar: OturumAyarlari) -> Surucu {
    let (t, o) = baslat(ayar, baglam).unwrap();
    Surucu::yeni(t, o)
}

fn a(arac: &Arc<SahteArac>) -> Arc<dyn kalfa_core::arac::Arac> {
    arac.clone()
}

fn yol_arg(y: &str) -> serde_json::Value {
    json!({ "yol": y })
}

async fn onizleme_bekle(surucu: &mut Surucu) -> (IslemId, Vec<OnizlemeOgesi>) {
    let zarf = surucu
        .bekle(|o| matches!(o, Olay::GonderimOnizlemesi { .. }))
        .await;
    match zarf.yuk {
        Olay::GonderimOnizlemesi { islem, ogeler, .. } => (islem, ogeler),
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn kapaliyken_bekletmez() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, _yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("oku", yol_arg("test.txt")),
        SahteYanit::metin("tamam"),
    ]);
    let arac = Arc::new(SahteArac::okuyan("oku"));
    let mut surucu = test_oturumu(
        test_baglami(s.clone(), vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    assert!(
        !surucu
            .olaylar()
            .iter()
            .any(|o| matches!(o, Olay::GonderimOnizlemesi { .. }))
    );
}

#[tokio::test]
async fn acikken_asama_sonunda_kart_gelir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, _yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("oku", yol_arg("test.txt")),
        SahteYanit::metin("tamam"),
    ]);
    let arac = Arc::new(SahteArac::okuyan("oku"));
    let mut surucu = test_oturumu(
        test_baglami(s.clone(), vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(Komut::OnizlemeAyarla { acik: true }).await;
    surucu.gonder(selam()).await;

    let (islem, ogeler) = onizleme_bekle(&mut surucu).await;
    assert_eq!(ogeler.len(), 1);
    assert_eq!(ogeler[0].kaynak, "sahte:oku");
    assert!(ogeler[0].bayt > 0);

    surucu
        .gonder(Komut::GonderimKarari {
            islem,
            gonderilmeyecekler: Vec::new(),
        })
        .await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
}

#[tokio::test]
async fn cikarilan_icerik_istekte_ve_kayitta_yok() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let gizli_icerik = "BU_COK_GIZLI_BIR_SIRDIR";
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("oku", yol_arg("gizli.txt")),
        SahteYanit::metin("tamam"),
    ]);
    let arac = Arc::new(SahteArac::okuyan("oku").yanit(gizli_icerik.into()));
    let mut surucu = test_oturumu(
        test_baglami(s.clone(), vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(Komut::OnizlemeAyarla { acik: true }).await;
    surucu.gonder(selam()).await;

    let (islem, ogeler) = onizleme_bekle(&mut surucu).await;
    assert_eq!(ogeler.len(), 1);

    surucu
        .gonder(Komut::GonderimKarari {
            islem,
            gonderilmeyecekler: vec![ogeler[0].cagri],
        })
        .await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    // 1. Sağlayıcının aldığı 2. istekte gizli içerik olmamalı, yer tutucu olmalı
    let istekler = s.istekler();
    assert_eq!(istekler.len(), 2);
    let tool_mesaji = istekler[1]
        .messages
        .iter()
        .find(|m| m.role == kalfa_evren::tipler::Role::Tool)
        .expect("tool mesajı bulunmalı");
    assert!(!tool_mesaji.content.as_ref().unwrap().contains(gizli_icerik));
    assert!(
        tool_mesaji
            .content
            .as_ref()
            .unwrap()
            .contains("[kullanıcı bu içeriği göndermedi]")
    );

    // 2. Kayıtta gizli içerik asla geçmemeli
    let ham = std::fs::read_to_string(&yol).unwrap();
    assert!(!ham.contains(gizli_icerik));
}

#[tokio::test]
async fn kullanici_mesaji_beklemez() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, _yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("merhaba")]);
    let mut surucu = test_oturumu(test_baglami(s, vec![], sanal_siniflandirici()), ayar);
    surucu.gonder(Komut::OnizlemeAyarla { acik: true }).await;
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    assert!(
        !surucu
            .olaylar()
            .iter()
            .any(|o| matches!(o, Olay::GonderimOnizlemesi { .. }))
    );
}

#[tokio::test]
async fn kalfa_iletisi_beklemez() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, _yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("olmayan_arac", json!({})),
        SahteYanit::metin("anladım"),
    ]);
    let mut surucu = test_oturumu(test_baglami(s, vec![], sanal_siniflandirici()), ayar);
    surucu.gonder(Komut::OnizlemeAyarla { acik: true }).await;
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    assert!(
        !surucu
            .olaylar()
            .iter()
            .any(|o| matches!(o, Olay::GonderimOnizlemesi { .. }))
    );
}

#[tokio::test]
async fn beklerken_iptal() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, _yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("oku", yol_arg("test.txt")),
        SahteYanit::metin("tamam"),
    ]);
    let arac = Arc::new(SahteArac::okuyan("oku"));
    let mut surucu = test_oturumu(
        test_baglami(s.clone(), vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(Komut::OnizlemeAyarla { acik: true }).await;
    surucu.gonder(selam()).await;

    let (_islem, _ogeler) = onizleme_bekle(&mut surucu).await;
    surucu.tutamac.iptal_et();
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Iptal);

    // İptalden sonra yeni bir turda bağlama giren metin sınanır:
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    assert_eq!(s.istekler().len(), 2);
    let ikinci_istek = &s.istekler()[1];
    let arac_mesaji = ikinci_istek
        .messages
        .iter()
        .find(|m| m.role == kalfa_evren::tipler::Role::Tool)
        .expect("iptal edilen araç için tamamlayıcı mesaj olmalı");
    assert_eq!(
        arac_mesaji.content.as_deref(),
        Some("[sonuç gönderilmedi: tur, gönderim önizlemesi beklenirken durduruldu]")
    );
}

#[tokio::test]
async fn beklerken_kip_degisikligi() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("oku", yol_arg("test.txt")),
        SahteYanit::metin("tamam"),
    ]);
    let arac = Arc::new(SahteArac::okuyan("oku"));
    let mut surucu = test_oturumu(
        test_baglami(s.clone(), vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(Komut::OnizlemeAyarla { acik: true }).await;
    surucu.gonder(selam()).await;

    let (islem, ogeler) = onizleme_bekle(&mut surucu).await;
    assert_eq!(ogeler.len(), 1);

    // Kip değişir: bekleme sürmeli, sağlayıcıya hemen istek gitmemeli.
    surucu
        .gonder(Komut::IzinKipiDegistir { kip: Kip::Inceleme })
        .await;

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    assert_eq!(
        s.istekler().len(),
        1,
        "kip değişikliğinden sonra otomatik istek gitmemeli"
    );

    // Kullanıcı çıkarma kararı verir:
    surucu
        .gonder(Komut::GonderimKarari {
            islem,
            gonderilmeyecekler: vec![ogeler[0].cagri],
        })
        .await;

    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    assert_eq!(s.istekler().len(), 2);
    let ikinci_istek = &s.istekler()[1];
    let arac_mesaji = ikinci_istek
        .messages
        .iter()
        .find(|m| m.role == kalfa_evren::tipler::Role::Tool)
        .expect("araç yanıtı olmalı");
    assert_eq!(
        arac_mesaji.content.as_deref(),
        Some("[kullanıcı bu içeriği göndermedi]")
    );

    let kayit_metni = std::fs::read_to_string(&yol).unwrap();
    assert!(!kayit_metni.contains("oku:test.txt"));
    assert!(kayit_metni.contains("[kullanıcı bu içeriği göndermedi]"));
}

#[test]
fn kurtarmada_yer_tutucu() {
    let oturum_id = OturumId::yeni();
    let tur_id = TurId::yeni();
    let cagri = CagriId::yeni();
    let kayitlar = vec![
        Zarf {
            sema: SEMA_SURUMU,
            oturum: oturum_id,
            tur: None,
            sira: 1,
            yuk: KayitOgesi::OturumAcildi {
                model: "glm-5.3".into(),
                kip: Kip::Siki,
                calisma_alani: None,
            },
        },
        Zarf {
            sema: SEMA_SURUMU,
            oturum: oturum_id,
            tur: Some(tur_id),
            sira: 2,
            yuk: KayitOgesi::Mesaj {
                mesaj: kalfa_evren::tipler::ChatMessage::assistant(
                    None,
                    vec![ToolCall {
                        id: "call_1".into(),
                        kind: "function".into(),
                        function: kalfa_evren::tipler::FunctionCall {
                            name: "oku".into(),
                            arguments: "{}".into(),
                        },
                    }],
                ),
                kaynak: Some("model".into()),
            },
        },
        Zarf {
            sema: SEMA_SURUMU,
            oturum: oturum_id,
            tur: Some(tur_id),
            sira: 3,
            yuk: KayitOgesi::AracBasladi {
                cagri,
                model_cagri_id: "call_1".into(),
                arac: "oku".into(),
                yedekler: vec![],
            },
        },
        Zarf {
            sema: SEMA_SURUMU,
            oturum: oturum_id,
            tur: Some(tur_id),
            sira: 4,
            yuk: KayitOgesi::AracBitti {
                cagri,
                durum: AracDurumu::Basarili,
            },
        },
    ];

    let durum = kurtar(&kayitlar).unwrap();
    assert_eq!(durum.tamamlayicilar.len(), 1);
    assert_eq!(
        durum.tamamlayicilar[0].content.as_deref(),
        Some(ONIZLEME_KAPANISI)
    );
}

#[tokio::test]
async fn ayar_kayittan_geri_gelir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("merhaba")]);
    let mut surucu = test_oturumu(
        test_baglami(s.clone(), vec![], sanal_siniflandirici()),
        ayar.clone(),
    );
    surucu.gonder(Komut::OnizlemeAyarla { acik: true }).await;
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    surucu.kapat().await;

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    let durum = kurtar(&kayitlar).unwrap();
    assert!(durum.onizleme_acik, "kurtarmada onizleme_acik true olmalı");
}

#[test]
fn g8_onizleme_karti_olay_butcesine_sigar() {
    let islem = IslemId::yeni();
    let ogeler: Vec<OnizlemeOgesi> = (0..50)
        .map(|i| OnizlemeOgesi {
            cagri: CagriId::yeni(),
            kaynak: format!("dosya:src/bilesenler/uzun_yol_adi_{i}.rs:1-120"),
            bayt: 64 * 1024,
            denetim: None,
        })
        .collect();
    let olay = Olay::GonderimOnizlemesi {
        islem,
        ogeler,
        neden: OnizlemeNedeni::Onizleme,
    };
    let zarf = Zarf {
        sema: SEMA_SURUMU,
        oturum: OturumId::yeni(),
        tur: Some(TurId::yeni()),
        sira: 1,
        yuk: olay,
    };

    let bayt = serde_json::to_vec(&zarf).unwrap().len();
    // 256 KiB bütçenin çok altında kalmalı (yaklaşık 5-10 KiB)
    assert!(
        bayt < 32 * 1024,
        "50 öğeli önizleme kartı {bayt} bayt; 32 KiB altında olmalı"
    );
}

#[tokio::test]
async fn ayni_hata_esiginde_onizleme_calisir_cikarilan_icerik_baglama_girmez() {
    let d = tempfile::tempdir().unwrap();
    let (mut ayar, yol) = kayitli(d.path());
    ayar.sinirlar.ayni_hata_esigi = 1;

    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![
            ("oku", yol_arg("gizli.txt")),
            ("hata_veren", yol_arg("sorunlu.txt")),
        ]),
        SahteYanit::metin("sonraki cevap"),
    ]);
    let okuyan = Arc::new(SahteArac::okuyan("oku").yanit("gizli-veri-9c3f".into()));
    let hata_veren = Arc::new(SahteArac::okuyan("hata_veren").hata("hata oluştu".into()));
    let mut surucu = test_oturumu(
        test_baglami(
            s.clone(),
            vec![a(&okuyan), a(&hata_veren)],
            sanal_siniflandirici(),
        ),
        ayar,
    );
    surucu.gonder(Komut::OnizlemeAyarla { acik: true }).await;
    surucu.gonder(selam()).await;

    // Hata eşiği aşılsa da bekleyen dış kaynaklı sonuç için önizleme kartı gelmeli:
    let (islem, ogeler) = onizleme_bekle(&mut surucu).await;
    assert_eq!(ogeler.len(), 1);

    // Kullanıcı gizli veriyi göndermeme kararı verir:
    surucu
        .gonder(Komut::GonderimKarari {
            islem,
            gonderilmeyecekler: vec![ogeler[0].cagri],
        })
        .await;

    // Tur sınırla bitmeli:
    assert!(matches!(surucu.tur_sonu().await, BitisNedeni::Sinir { .. }));

    // Kullanıcı yeni bir tur başlatır:
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    // İkinci turda sağlayıcıya giden mesajlarda gizli içerik OLMAMALI, yer tutucu olmalı:
    assert_eq!(s.istekler().len(), 2);
    let ikinci_istek = &s.istekler()[1];
    let arac_mesaji = ikinci_istek
        .messages
        .iter()
        .find(|m| {
            m.role == kalfa_evren::tipler::Role::Tool
                && m.content.as_deref() == Some("[kullanıcı bu içeriği göndermedi]")
        })
        .expect("çıkarılan araç yanıtı olmalı");
    assert_eq!(
        arac_mesaji.content.as_deref(),
        Some("[kullanıcı bu içeriği göndermedi]")
    );

    let butun_istekler = serde_json::to_string(&s.istekler()).unwrap();
    assert!(!butun_istekler.contains("gizli-veri-9c3f"));

    // Kayıtta da gizli içerik olmamalı:
    let kayit_metni = std::fs::read_to_string(&yol).unwrap();
    assert!(!kayit_metni.contains("gizli-veri-9c3f"));
    assert!(kayit_metni.contains("[kullanıcı bu içeriği göndermedi]"));
}
