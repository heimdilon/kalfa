use std::path::Path;
use std::time::Duration;

use kalfa_core::aktarim::{AktarimAyarlari, KanalMesaji, Paketleyici};
use kalfa_core::oturum::{OlayButcesi, OturumAyarlari, baslat};
use kalfa_core::protokol::{AktarimDurmaNedeni, Komut, Olay};
use kalfa_core::test_destek::{SahteSaglayici, SahteYanit, sanal_siniflandirici, test_baglami};
use kalfa_evren::AkisOlayi;

fn durma_nedeni(m: &KanalMesaji) -> Option<AktarimDurmaNedeni> {
    match m {
        KanalMesaji::Olaylar { zarflar, .. } => zarflar.iter().find_map(|z| match z.yuk {
            Olay::AktarimDurdu { neden } => Some(neden),
            _ => None,
        }),
        KanalMesaji::Kapandi { .. } => None,
    }
}

#[tokio::test]
async fn islemeyen_arayuzde_bellek_sinirli_kalir_tur_iptal_edilir_ve_neden_bildirilir() {
    let saglayici = SahteSaglayici::yeni(vec![SahteYanit::Askida(
        (0..200)
            .map(|i| AkisOlayi::Metin(format!("parça-{i} ")))
            .collect(),
    )]);
    let mut ayar = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    ayar.olay_butcesi = OlayButcesi::yeni(16, 1024 * 1024).unwrap();
    let (tutamac, alici) = baslat(
        ayar,
        test_baglami(saglayici, vec![], sanal_siniflandirici()),
    )
    .unwrap();
    let aktarim = AktarimAyarlari {
        azami_paket_bayt: 1024,
        kredi_paket: 1,
        ..AktarimAyarlari::default()
    };
    let (mut p, kredi) = Paketleyici::yeni(alici, aktarim);
    tutamac
        .gonder(Komut::KullaniciMesaji {
            metin: "uzun akış".into(),
        })
        .await
        .unwrap();

    // İlk paket gelir; arayüz "işledim" demez.
    let ilk = tokio::time::timeout(Duration::from_secs(2), p.sonraki())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ilk.no(), 1);
    let mut neden = durma_nedeni(&ilk);

    // Paketleyici kuyruktan okumayı bıraktığı için çekirdek bütçesi dolar,
    // tur iptal edilir.
    tokio::time::timeout(Duration::from_secs(2), async {
        while tutamac.aktif_tur().is_some() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("tur iptal edilmeli");
    assert_eq!(kredi.yolda().0, 1, "yolda en çok kredi kadar paket olur");

    // Arayüz yeniden işlemeye başlayınca neden görünür, sonra kanal kapanır.
    kredi.isledim(ilk.no()).unwrap();
    let mut son = None;
    while let Some(m) = tokio::time::timeout(Duration::from_secs(2), p.sonraki())
        .await
        .unwrap()
    {
        assert!(kredi.yolda().0 <= 1);
        kredi.isledim(m.no()).unwrap();
        neden = neden.or(durma_nedeni(&m));
        son = Some(m);
    }
    assert_eq!(neden, Some(AktarimDurmaNedeni::TuketiciYetismedi));
    assert!(matches!(son, Some(KanalMesaji::Kapandi { .. })));
}
