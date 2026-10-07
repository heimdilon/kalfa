//! Gizli ayarları ve ağ izin listesi (aşama 3 tasarımı §8.3, §9.2; 3e planı
//! Görev 4): `ayarlar.json` ve komutlar.

use std::time::Duration;

use kalfa_core::aktarim::KanalMesaji;
use kalfa_core::kimlik::OturumId;
use kalfa_core::protokol::Olay;
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::{SahteSaglayici, SahteYanit};
use kalfa_uygulama::sozlesme::AgHostu;
use kalfa_uygulama::test_destek::{SahteKanal, TestUygulamasi, test_uygulamasi};
use serde_json::json;
use tokio::sync::mpsc::UnboundedReceiver;

fn uygulama(veri: &std::path::Path) -> TestUygulamasi {
    test_uygulamasi(veri, SahteSaglayici::yeni(vec![]), Some("anahtar"))
}

#[tokio::test]
async fn eklemek_ve_kaldirmak_onaysiz() {
    // Gizliliği artıranlar: ek kalıp eklemek ve istisna kaldırmak.
    let d = tempfile::tempdir().unwrap();
    std::fs::write(
        d.path().join("ayarlar.json"),
        r#"{"gizli_sayilmayanlar": [".env.example"]}"#,
    )
    .unwrap();
    let t = uygulama(d.path());
    t.iletisim.onay_ver(false);
    let g = t
        .uygulama
        .ek_gizli_kalip_ekle("*.sifre".into())
        .await
        .unwrap();
    assert_eq!(g.ek_kaliplar, vec!["*.sifre".to_string()]);
    let g = t
        .uygulama
        .gizli_sayilmayan_kaldir(".env.example".into())
        .await
        .unwrap();
    assert!(g.gizli_sayilmayanlar.is_empty());
    assert!(t.iletisim.sorulanlar().is_empty());
    assert!(g.varsayilanlar.contains(&".env".to_string()));
}

#[tokio::test]
async fn kaldirmak_yerel_onay_ister_vazgecilince_degismez() {
    let d = tempfile::tempdir().unwrap();
    let t = uygulama(d.path());
    t.uygulama
        .ek_gizli_kalip_ekle("*.sifre".into())
        .await
        .unwrap();

    t.iletisim.onay_ver(false);
    let g = t
        .uygulama
        .ek_gizli_kalip_kaldir("*.sifre".into())
        .await
        .unwrap();
    assert_eq!(g.ek_kaliplar, vec!["*.sifre".to_string()], "vazgeçildi");
    assert_eq!(t.iletisim.sorulanlar().len(), 1);

    t.iletisim.onay_ver(true);
    let g = t
        .uygulama
        .ek_gizli_kalip_kaldir("*.sifre".into())
        .await
        .unwrap();
    assert!(g.ek_kaliplar.is_empty());
}

#[tokio::test]
async fn istisna_eklemek_yerel_onay_ister() {
    let d = tempfile::tempdir().unwrap();
    let t = uygulama(d.path());
    t.iletisim.onay_ver(false);
    let g = t
        .uygulama
        .gizli_sayilmayan_ekle("src/credentials.rs".into())
        .await
        .unwrap();
    assert!(g.gizli_sayilmayanlar.is_empty(), "vazgeçildi");
    let soru = t.iletisim.sorulanlar().pop().unwrap();
    assert!(soru.contains("src/credentials.rs"), "{soru}");
    assert!(
        soru.contains("gizli sayılmayacak: onaysız okunup EVREN'e gidebilir"),
        "{soru}"
    );

    t.iletisim.onay_ver(true);
    let g = t
        .uygulama
        .gizli_sayilmayan_ekle("src/credentials.rs".into())
        .await
        .unwrap();
    assert_eq!(
        g.gizli_sayilmayanlar,
        vec!["src/credentials.rs".to_string()]
    );
}

#[tokio::test]
async fn gecersiz_kalip_reddedilir_ve_onay_sorulmaz() {
    let d = tempfile::tempdir().unwrap();
    let t = uygulama(d.path());
    t.iletisim.onay_ver(true);
    assert!(
        t.uygulama
            .ek_gizli_kalip_ekle("/etc/x".into())
            .await
            .is_err()
    );
    assert!(t.uygulama.gizli_sayilmayan_ekle("**".into()).await.is_err());
    assert!(t.iletisim.sorulanlar().is_empty());
    assert!(!d.path().join("ayarlar.json").exists());
}

#[tokio::test]
async fn ayar_dosyasi_atomik_yazilir_ve_yeniden_okunur() {
    let d = tempfile::tempdir().unwrap();
    let t = uygulama(d.path());
    t.iletisim.onay_ver(true);
    t.uygulama
        .ek_gizli_kalip_ekle("*.sifre".into())
        .await
        .unwrap();
    t.uygulama
        .gizli_sayilmayan_ekle(".env.example".into())
        .await
        .unwrap();
    t.uygulama
        .ag_hostu_ekle("git.sirket.local".into(), true)
        .await
        .unwrap();
    assert!(!d.path().join("ayarlar.json.yeni").exists());
    drop(t);

    let t = uygulama(d.path());
    let g = t.uygulama.gizli_ayarlari();
    assert_eq!(g.ek_kaliplar, vec!["*.sifre".to_string()]);
    assert_eq!(g.gizli_sayilmayanlar, vec![".env.example".to_string()]);
    assert_eq!(g.ayar_hatasi, None);
    assert_eq!(
        t.uygulama.ag_izin_listesi(),
        vec![AgHostu {
            host: "git.sirket.local".into(),
            port: 443,
            ozel_ag: true,
        }]
    );
}

#[test]
fn bozuk_ayar_dosyasi_varsayilanla_acilir() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(
        d.path().join("ayarlar.json"),
        r#"{"denetci_kipi": "Kati", "ek_gizli_kaliplar": ["/mutlak", "*.sifre"], "gizli_sayilmayanlar": ["*", ".env.example"]}"#,
    )
    .unwrap();
    let t = uygulama(d.path());
    let g = t.uygulama.gizli_ayarlari();
    // Yalnızca geçersiz kalıplar düşer; geçerli ek kalıp gizli saymaya devam
    // eder.
    assert_eq!(g.ek_kaliplar, vec!["*.sifre".to_string()]);
    assert_eq!(g.gizli_sayilmayanlar, vec![".env.example".to_string()]);
    assert!(
        g.ayar_hatasi
            .as_deref()
            .is_some_and(|h| h.contains("gizli kalıplar geçersiz")),
        "{g:?}"
    );
    // Dosyanın geri kalanı düşmez.
    assert_eq!(
        t.uygulama.denetci_kipi().kip,
        kalfa_core::denetci::DenetimKipi::Kati
    );
}

#[tokio::test]
async fn ag_hostu_eklemek_yerel_onay_ister() {
    let d = tempfile::tempdir().unwrap();
    let t = uygulama(d.path());
    t.iletisim.onay_ver(false);
    let l = t
        .uygulama
        .ag_hostu_ekle("pkg.sirket.local:8443".into(), true)
        .await
        .unwrap();
    assert!(l.is_empty(), "vazgeçildi");
    let soru = t.iletisim.sorulanlar().pop().unwrap();
    assert!(soru.contains("pkg.sirket.local:8443"), "{soru}");
    assert!(soru.contains("özel ağ"), "{soru}");

    t.iletisim.onay_ver(true);
    let l = t
        .uygulama
        .ag_hostu_ekle("pkg.sirket.local:8443".into(), false)
        .await
        .unwrap();
    assert_eq!(
        l,
        vec![AgHostu {
            host: "pkg.sirket.local".into(),
            port: 8443,
            ozel_ag: false,
        }]
    );
    let sorulan = t.iletisim.sorulanlar().len();
    let l = t
        .uygulama
        .ag_hostu_kaldir("pkg.sirket.local:8443".into())
        .await
        .unwrap();
    assert!(l.is_empty());
    assert_eq!(t.iletisim.sorulanlar().len(), sorulan, "kaldırmak onaysız");
    assert!(
        t.uygulama
            .ag_hostu_ekle("*.sirket.local".into(), false)
            .await
            .is_err(),
        "joker kabul edilmez"
    );
}

async fn bekle(
    t: &TestUygulamasi,
    oturum: OturumId,
    rx: &mut UnboundedReceiver<KanalMesaji>,
    kosul: impl Fn(&Olay) -> bool,
) -> Olay {
    loop {
        let mesaj = tokio::time::timeout(Duration::from_secs(20), rx.recv())
            .await
            .expect("olay gelmedi")
            .expect("kanal kapandı");
        let KanalMesaji::Olaylar { no, zarflar } = mesaj else {
            panic!("kanal kapandı");
        };
        t.uygulama.paketi_isledim(oturum, no).unwrap();
        if let Some(z) = zarflar.into_iter().find(|z| kosul(&z.yuk)) {
            return z.yuk;
        }
    }
}

/// Ek kalıp açık oturumda bir sonraki araç çağrısında geçerlidir: aynı
/// dosyanın okunması artık onaya çıkar.
#[tokio::test]
async fn degisiklik_acik_oturumda_sonraki_cagrida_gecerli() {
    let veri = tempfile::tempdir().unwrap();
    let alan = tempfile::tempdir().unwrap();
    std::fs::write(alan.path().join("notlar.txt"), "not").unwrap();
    let saglayici = SahteSaglayici::yeni(vec![
        SahteYanit::arac("dosya_oku", json!({"yol": "notlar.txt"})),
        SahteYanit::metin("okudum"),
        SahteYanit::arac("dosya_oku", json!({"yol": "notlar.txt"})),
        SahteYanit::metin("yine"),
    ]);
    saglayici.sart_kabul(1).await.unwrap();
    let t = test_uygulamasi(veri.path(), saglayici, Some("evren_llm_a"));
    t.uygulama.baslat().await;
    t.iletisim.klasor_ver(alan.path());
    let calisma = t.uygulama.calisma_alani_sec().await.unwrap();
    let oturum = t
        .uygulama
        .oturum_baslat(calisma.kimlik, "glm-5.3".into())
        .await
        .unwrap()
        .oturum;
    let (kanal, mut rx) = SahteKanal::yeni();
    t.uygulama.oturumu_dinle(oturum, kanal).unwrap();

    t.uygulama.mesaj_gonder(oturum, "oku".into()).await.unwrap();
    let olay = bekle(&t, oturum, &mut rx, |o| {
        matches!(o, Olay::Tamamlandi { .. } | Olay::OnayGerekiyor { .. })
    })
    .await;
    assert!(
        matches!(olay, Olay::Tamamlandi { .. }),
        "önce onaysız: {olay:?}"
    );

    t.uygulama
        .ek_gizli_kalip_ekle("notlar.txt".into())
        .await
        .unwrap();
    t.uygulama
        .mesaj_gonder(oturum, "yine oku".into())
        .await
        .unwrap();
    let olay = bekle(&t, oturum, &mut rx, |o| {
        matches!(o, Olay::Tamamlandi { .. } | Olay::OnayGerekiyor { .. })
    })
    .await;
    assert!(
        matches!(olay, Olay::OnayGerekiyor { .. }),
        "ek kalıptan sonra gizli: {olay:?}"
    );
}
