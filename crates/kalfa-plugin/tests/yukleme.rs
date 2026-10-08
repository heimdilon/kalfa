//! Yükleme (aşama 4 tasarımı §7.2, §7.3): sözleşmeye uyan bileşen yüklenir,
//! uymayan ve izinsiz import eden yüklenmez.

mod ortak;

use std::io::Read;
use std::time::Duration;

use kalfa_plugin::YuklemeHatasi;
use kalfa_plugin::manifest::ManifestHatasi;
use ortak::*;
use serde_json::json;

#[tokio::test]
async fn sozlesmeye_uyan_eklenti_yuklenir_ve_araclari_sunulur() {
    let d = Duzenek::yeni(&manifest(&[], &[], 0)).await;
    let adlar: Vec<&str> = d.eklenti.araclar.iter().map(|a| a.ad.as_str()).collect();
    assert!(
        adlar.contains(&"yanki") && adlar.contains(&"dongu"),
        "{adlar:?}"
    );
    assert_eq!(d.arac("yanki").ad(), "kalfa_test__yanki");
    assert_eq!(d.eklenti.ozet.len(), 64);
}

#[tokio::test]
async fn soket_import_eden_bilesen_yuklenmez() {
    let h = motor()
        .yukle(&manifest(&[], &[], 0), wasm("kalfa-test-soket"))
        .await
        .unwrap_err();
    assert!(
        matches!(&h, YuklemeHatasi::Import(ad) if ad.starts_with("wasi:sockets/")),
        "{h:?}"
    );
    let ileti = h.to_string();
    assert!(
        ileti.contains("ağ erişimi") && ileti.contains("evren-cagir"),
        "{ileti}"
    );
}

/// İkinci katman (§7.3): import denetimi atlansa bile `WasiCtx` ağsızdır;
/// eklenti bu makinede dinleyen bir sokete bağlanamaz.
#[tokio::test]
async fn import_denetimi_atlansa_da_soket_baglanamaz() {
    let dinleyici = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    dinleyici.set_nonblocking(true).unwrap();
    let adres = dinleyici.local_addr().unwrap().to_string();
    let motor = motor();
    let eklenti = std::sync::Arc::new(
        motor
            .yukle_import_denetimsiz(&manifest(&[], &[], 0), wasm("kalfa-test-soket"))
            .await
            .unwrap(),
    );
    let alan = tempfile::tempdir().unwrap();
    let siniflandirici =
        std::sync::Arc::new(kalfa_core::izin::Siniflandirici::yeni(alan.path(), &[]).unwrap());
    let araclar = kalfa_plugin::EklentiAraci::hepsi(
        &motor,
        &eklenti,
        &kalfa_tools::CalismaAlani::ac(siniflandirici).unwrap(),
        &std::sync::Arc::new(kalfa_core::kilit::KaynakKilitleri::yeni()),
        alan.path(),
    );
    let c = cagir(
        &araclar[0],
        json!({"adres": adres}),
        tokio_util::sync::CancellationToken::new(),
    )
    .await;
    assert!(c.icerik().starts_with("bağlanamadı"), "{}", c.icerik());
    tokio::time::sleep(Duration::from_millis(100)).await;
    match dinleyici.accept() {
        Ok((mut s, _)) => {
            let mut b = Vec::new();
            let _ = s.read_to_end(&mut b);
            panic!("eklenti bağlandı");
        }
        Err(e) => assert_eq!(e.kind(), std::io::ErrorKind::WouldBlock),
    }
}

#[tokio::test]
async fn sozlesmeye_uymayan_dunya_yuklenmez() {
    let h = motor()
        .yukle(&manifest(&[], &[], 0), wasm("kalfa-test-yanlis-dunya"))
        .await
        .unwrap_err();
    assert!(matches!(h, YuklemeHatasi::Dunya(_)), "{h:?}");
    assert!(h.to_string().contains("kalfa:plugin@0.1.0"));
}

#[tokio::test]
async fn cekirdek_modul_ve_bozuk_dosya_yuklenmez() {
    let m = manifest(&[], &[], 0);
    let h = motor()
        .yukle(&m, b"\0asm\x01\0\0\0".to_vec())
        .await
        .unwrap_err();
    assert!(h.to_string().contains("çekirdek modül"), "{h}");
    let h = motor().yukle(&m, b"bozuk".to_vec()).await.unwrap_err();
    assert!(matches!(h, YuklemeHatasi::BilesenDegil(_)), "{h:?}");
}

#[tokio::test]
async fn baska_sozlesme_surumunun_manifesti_yuklenmez() {
    let m = manifest(&[], &[], 0).replace("sozlesme = \"0.1\"", "sozlesme = \"0.2\"");
    let h = motor()
        .yukle(&m, wasm("kalfa-test-eklentisi"))
        .await
        .unwrap_err();
    assert!(
        matches!(&h, YuklemeHatasi::Manifest(ManifestHatasi::Sozlesme(s)) if s == "0.2"),
        "{h:?}"
    );
}
