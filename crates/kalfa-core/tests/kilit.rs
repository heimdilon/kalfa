use std::time::Duration;

use kalfa_core::kilit::KaynakKilitleri;
use kalfa_core::yol::CalismaYolu;
use tokio::time::timeout;

const KISA: Duration = Duration::from_millis(100);

fn y(s: &str) -> CalismaYolu {
    CalismaYolu::coz(s).unwrap()
}

#[tokio::test]
async fn okumalar_birbirini_beklemez() {
    let k = KaynakKilitleri::yeni();
    let _a = k.al(&[y("a")], &[], false).await;
    assert!(timeout(KISA, k.al(&[y("a")], &[], false)).await.is_ok());
}

#[tokio::test]
async fn yazma_ayni_yoldaki_okumayi_bekletir() {
    let k = KaynakKilitleri::yeni();
    let yazma = k.al(&[], &[y("a")], false).await;
    assert!(timeout(KISA, k.al(&[y("a")], &[], false)).await.is_err());
    drop(yazma);
    assert!(timeout(KISA, k.al(&[y("a")], &[], false)).await.is_ok());
}

#[tokio::test]
async fn farkli_yollar_bagimsiz() {
    let k = KaynakKilitleri::yeni();
    let _w = k.al(&[], &[y("a")], false).await;
    assert!(timeout(KISA, k.al(&[], &[y("b")], false)).await.is_ok());
}

#[tokio::test]
async fn buyuk_kucuk_harf_ayni_kilit() {
    let k = KaynakKilitleri::yeni();
    let _w = k.al(&[], &[y("A.txt")], false).await;
    assert!(
        timeout(KISA, k.al(&[y("a.TXT")], &[], false))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn ayni_istekte_okuma_ve_yazma_kendini_kilitlemez() {
    let k = KaynakKilitleri::yeni();
    assert!(
        timeout(KISA, k.al(&[y("a")], &[y("a")], false))
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn tum_calisma_alani_ozel_kilittir() {
    let k = KaynakKilitleri::yeni();
    let tum = k.al(&[], &[], true).await;
    assert!(timeout(KISA, k.al(&[y("x")], &[], false)).await.is_err());
    drop(tum);
    let _o = k.al(&[y("x")], &[], false).await;
    assert!(
        timeout(KISA, k.al(&[], &[], true)).await.is_err(),
        "okuma sürerken özel kilit bekler"
    );
}
