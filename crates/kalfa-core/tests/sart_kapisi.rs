use std::sync::Arc;

use kalfa_core::saglayici::Saglayici;
use kalfa_core::sart_kapisi::{
    IlkDurum, KabulSonucu, KapiGorunumu, SartKapisi, SartKarari, SartMetni,
};
use kalfa_core::test_destek::SahteSaglayici;
use kalfa_evren::EvrenError;

fn kapi(s: &Arc<SahteSaglayici>) -> SartKapisi {
    SartKapisi::yeni(s.clone())
}

fn beklenen(surum: u32) -> SartMetni {
    SartMetni {
        surum,
        esasli: true,
        metin: "# Sahte kullanım şartları".into(),
    }
}

#[tokio::test]
async fn ilk_durum_kabul_edilmis() {
    let s = SahteSaglayici::yeni(vec![]);
    // Kullanıcı şartları daha önce kabul etmiş.
    s.sart_kabul(1).await.unwrap();
    let k = kapi(&s);
    assert_eq!(k.ilk_durum().await.unwrap(), IlkDurum::KabulEdilmis);
    assert_eq!(k.durum().gorunum, KapiGorunumu::Bosta);
}

#[tokio::test]
async fn ilk_durum_kabul_gerekliyse_metin_gosterilir() {
    let s = SahteSaglayici::yeni(vec![]);
    let k = kapi(&s);
    assert_eq!(
        k.ilk_durum().await.unwrap(),
        IlkDurum::KabulGerekli(beklenen(1))
    );
    assert_eq!(k.durum().gorunum, KapiGorunumu::Gosteriliyor(beklenen(1)));
}

#[tokio::test]
async fn ilk_durumda_anahtar_gecersizse_hata_doner() {
    let s = SahteSaglayici::yeni(vec![]);
    s.sart_durumu_hatasi(Some(EvrenError::AnahtarGecersiz));
    assert_eq!(kapi(&s).ilk_durum().await, Err(EvrenError::AnahtarGecersiz));
}

#[tokio::test]
async fn bekleyenler_tek_kabulle_uyanir() {
    let s = SahteSaglayici::yeni(vec![]);
    let k = Arc::new(kapi(&s));
    let (m1, n1) = k.gerekli().await.unwrap();
    let (m2, n2) = k.gerekli().await.unwrap();
    assert_eq!(
        (m1.clone(), n1),
        (m2, n2),
        "ikinci oturum gösterilen metne katılır"
    );
    let bekleyenler: Vec<_> = (0..2)
        .map(|_| {
            let k = k.clone();
            tokio::spawn(async move { k.karar_bekle(n1).await })
        })
        .collect();
    assert_eq!(k.kabul_et(m1.surum).await.unwrap(), KabulSonucu::Kabul);
    for b in bekleyenler {
        assert_eq!(b.await.unwrap(), SartKarari::Kabul { surum: 1 });
    }
    assert_eq!(
        s.kabul_edilen_surumler(),
        vec![1],
        "kabul EVREN'e bir kez gider"
    );
    assert_eq!(k.durum().gorunum, KapiGorunumu::Bosta);
}

#[tokio::test]
async fn gosterilmeyen_surum_kabul_edilmez() {
    let s = SahteSaglayici::yeni(vec![]);
    let k = kapi(&s);
    assert_eq!(
        k.kabul_et(1).await.unwrap(),
        KabulSonucu::YeniMetin(beklenen(1)),
        "gösterilmemiş metin kabul edilemez"
    );
    assert_eq!(
        k.kabul_et(2).await.unwrap(),
        KabulSonucu::YeniMetin(beklenen(1))
    );
    assert!(s.kabul_edilen_surumler().is_empty());
}

#[tokio::test]
async fn okurken_surum_degisirse_yeni_metin_gosterilir() {
    let s = SahteSaglayici::yeni(vec![]);
    let k = kapi(&s);
    k.gerekli().await.unwrap();
    s.sart_surumunu_degistir(2);
    assert_eq!(
        k.kabul_et(1).await.unwrap(),
        KabulSonucu::YeniMetin(beklenen(2))
    );
    assert_eq!(k.kabul_et(2).await.unwrap(), KabulSonucu::Kabul);
    assert_eq!(s.kabul_edilen_surumler(), vec![2]);
}

#[tokio::test]
async fn ret_yeniden_acilis_ve_kabul_eski_bekleyeni_ret_eder() {
    let s = SahteSaglayici::yeni(vec![]);
    let k = kapi(&s);
    let (metin, nesil) = k.gerekli().await.unwrap();
    let bekleyen = k.karar_bekle(nesil);

    k.reddet();
    let (_, yeni_nesil) = k.gerekli().await.unwrap();
    assert_eq!(yeni_nesil, nesil + 1);
    assert_eq!(k.kabul_et(metin.surum).await.unwrap(), KabulSonucu::Kabul);

    assert_eq!(
        bekleyen.await,
        SartKarari::Ret,
        "eski tur ret kararını kaçırıp yeni kabulü kullanmamalı"
    );
}

#[tokio::test]
async fn ret_bekleyenlere_iletilir_ve_sonraki_istekte_metin_yeniden_gelir() {
    let s = SahteSaglayici::yeni(vec![]);
    let k = Arc::new(kapi(&s));
    let (_, n) = k.gerekli().await.unwrap();
    let bekleyen = {
        let k = k.clone();
        tokio::spawn(async move { k.karar_bekle(n).await })
    };
    k.reddet();
    assert_eq!(bekleyen.await.unwrap(), SartKarari::Ret);
    assert_eq!(k.durum().gorunum, KapiGorunumu::Reddedildi);
    let (m, _) = k.gerekli().await.unwrap();
    assert_eq!(m, beklenen(1));
    assert_eq!(k.durum().gorunum, KapiGorunumu::Gosteriliyor(beklenen(1)));
}
