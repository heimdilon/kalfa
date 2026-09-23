use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use kalfa_core::aktarim::{
    AktarimAyarlari, IsledimHatasi, KanalMesaji, KrediTutamaci, OlayKaynagi, Paketleyici,
};
use kalfa_core::kimlik::{OturumId, TurId};
use kalfa_core::protokol::{BitisNedeni, Olay, SEMA_SURUMU, Zarf};
use tokio::sync::mpsc;
use tokio::time::Instant;

/// Kaç olay okunduğunu sayan test kaynağı.
struct TestKaynagi {
    rx: mpsc::UnboundedReceiver<Zarf<Olay>>,
    okunan: Arc<AtomicUsize>,
}

impl OlayKaynagi for TestKaynagi {
    fn recv(&mut self) -> impl Future<Output = Option<Zarf<Olay>>> + Send {
        let okunan = self.okunan.clone();
        let rx = &mut self.rx;
        async move {
            let zarf = rx.recv().await;
            if zarf.is_some() {
                okunan.fetch_add(1, Ordering::SeqCst);
            }
            zarf
        }
    }
}

struct Uretici {
    tx: mpsc::UnboundedSender<Zarf<Olay>>,
    oturum: OturumId,
    tur: TurId,
    sira: u64,
}

impl Uretici {
    fn gonder(&mut self, yuk: Olay) {
        self.sira += 1;
        self.tx
            .send(Zarf {
                sema: SEMA_SURUMU,
                oturum: self.oturum,
                tur: Some(self.tur),
                sira: self.sira,
                yuk,
            })
            .unwrap();
    }

    fn metin(&mut self, s: &str) {
        self.gonder(Olay::TokenParcasi { metin: s.into() });
    }

    fn dusunce(&mut self, s: &str) {
        self.gonder(Olay::DusunceParcasi { metin: s.into() });
    }

    fn hata(&mut self, s: &str) {
        self.gonder(Olay::Hata { mesaj: s.into() });
    }
}

fn kur(
    ayarlar: AktarimAyarlari,
) -> (
    Uretici,
    Paketleyici<TestKaynagi>,
    KrediTutamaci,
    Arc<AtomicUsize>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    let okunan = Arc::new(AtomicUsize::new(0));
    let kaynak = TestKaynagi {
        rx,
        okunan: okunan.clone(),
    };
    let (paketleyici, kredi) = Paketleyici::yeni(kaynak, ayarlar);
    let uretici = Uretici {
        tx,
        oturum: OturumId::yeni(),
        tur: TurId::yeni(),
        sira: 0,
    };
    (uretici, paketleyici, kredi, okunan)
}

fn olaylar(m: &KanalMesaji) -> Vec<Olay> {
    match m {
        KanalMesaji::Olaylar { zarflar, .. } => zarflar.iter().map(|z| z.yuk.clone()).collect(),
        KanalMesaji::Kapandi { .. } => Vec::new(),
    }
}

fn metin(s: &str) -> Olay {
    Olay::TokenParcasi { metin: s.into() }
}

#[tokio::test(start_paused = true)]
async fn pencere_icindeki_bitisik_metin_parcalari_birlesir() {
    let (mut u, mut p, _kredi, _) = kur(AktarimAyarlari::default());
    u.metin("Mer");
    u.metin("ha");
    u.metin("ba");
    let m = p.sonraki().await.unwrap();
    assert_eq!(m.no(), 1);
    assert_eq!(olaylar(&m), vec![metin("Merhaba")]);
    let KanalMesaji::Olaylar { zarflar, .. } = &m else {
        unreachable!()
    };
    assert_eq!(
        zarflar[0].sira, 3,
        "birleşen parça son parçanın sırasını taşır"
    );
}

#[tokio::test(start_paused = true)]
async fn pencereden_sonra_gelen_parca_sonraki_pakete_kalir() {
    let (mut u, mut p, kredi, _) = kur(AktarimAyarlari::default());
    u.metin("a");
    let uretici = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        u.metin("b");
        tokio::time::sleep(Duration::from_millis(20)).await;
        u.metin("c");
        u
    });
    let bir = p.sonraki().await.unwrap();
    assert_eq!(olaylar(&bir), vec![metin("ab")]);
    kredi.isledim(bir.no()).unwrap();
    let iki = p.sonraki().await.unwrap();
    assert_eq!(olaylar(&iki), vec![metin("c")]);
    drop(uretici.await.unwrap());
}

#[tokio::test(start_paused = true)]
async fn farkli_turden_olay_birlestirmeyi_keser() {
    let (mut u, mut p, _kredi, _) = kur(AktarimAyarlari::default());
    u.metin("a");
    u.dusunce("d");
    u.metin("b");
    let m = p.sonraki().await.unwrap();
    assert_eq!(
        olaylar(&m),
        vec![
            metin("a"),
            Olay::DusunceParcasi { metin: "d".into() },
            metin("b")
        ]
    );
}

#[tokio::test(start_paused = true)]
async fn kritik_olay_pencereyi_beklemeden_paketi_kapatir() {
    let (mut u, mut p, kredi, _) = kur(AktarimAyarlari::default());
    u.metin("a");
    u.gonder(Olay::Tamamlandi {
        neden: BitisNedeni::Normal,
    });
    u.metin("b");
    let bas = Instant::now();
    let bir = p.sonraki().await.unwrap();
    assert_eq!(
        bas.elapsed(),
        Duration::ZERO,
        "kritik olay pencereyi beklemez"
    );
    assert_eq!(
        olaylar(&bir),
        vec![
            metin("a"),
            Olay::Tamamlandi {
                neden: BitisNedeni::Normal
            }
        ]
    );
    kredi.isledim(bir.no()).unwrap();
    let iki = p.sonraki().await.unwrap();
    assert_eq!(olaylar(&iki), vec![metin("b")]);
}

#[tokio::test(start_paused = true)]
async fn ilk_olay_kritikse_tek_basina_hemen_gider() {
    let (mut u, mut p, _kredi, _) = kur(AktarimAyarlari::default());
    u.hata("bir");
    u.hata("iki");
    let bas = Instant::now();
    let m = p.sonraki().await.unwrap();
    assert_eq!(bas.elapsed(), Duration::ZERO);
    assert_eq!(
        olaylar(&m),
        vec![Olay::Hata {
            mesaj: "bir".into()
        }],
        "kritik olaylar birleştirilmez"
    );
}

#[tokio::test(start_paused = true)]
async fn paket_bayt_siniri_asilmaz_ve_olaylar_sirayla_gelir() {
    let ayar = AktarimAyarlari {
        azami_paket_bayt: 400,
        ..AktarimAyarlari::default()
    };
    let (mut u, mut p, kredi, _) = kur(ayar);
    for i in 0..10 {
        if i % 2 == 0 {
            u.metin(&format!("m{i}"));
        } else {
            u.dusunce(&format!("d{i}"));
        }
    }
    drop(u);
    let mut gelen = Vec::new();
    while let Some(m) = p.sonraki().await {
        kredi.isledim(m.no()).unwrap();
        if let KanalMesaji::Olaylar { zarflar, .. } = m {
            let toplam: usize = zarflar
                .iter()
                .map(|z| serde_json::to_vec(z).unwrap().len())
                .sum();
            assert!(zarflar.len() == 1 || toplam <= 400, "paket {toplam} bayt");
            gelen.extend(zarflar.into_iter().map(|z| z.sira));
        }
    }
    assert_eq!(gelen, (1..=10).collect::<Vec<u64>>());
}

#[tokio::test(start_paused = true)]
async fn siniri_asan_tek_olay_kendi_paketinde_gider() {
    let ayar = AktarimAyarlari {
        azami_paket_bayt: 100,
        ..AktarimAyarlari::default()
    };
    let (mut u, mut p, _kredi, _) = kur(ayar);
    let uzun = "x".repeat(500);
    u.metin(&uzun);
    u.dusunce("d");
    let m = p.sonraki().await.unwrap();
    assert_eq!(olaylar(&m), vec![metin(&uzun)]);
}

#[tokio::test(start_paused = true)]
async fn kredi_bitince_kaynaktan_okunmaz() {
    let ayar = AktarimAyarlari {
        kredi_paket: 2,
        ..AktarimAyarlari::default()
    };
    let (mut u, mut p, kredi, okunan) = kur(ayar);
    for i in 0..10 {
        u.hata(&format!("h{i}"));
    }
    let bir = p.sonraki().await.unwrap();
    let iki = p.sonraki().await.unwrap();
    assert_eq!((bir.no(), iki.no()), (1, 2));
    assert!(
        tokio::time::timeout(Duration::from_secs(1), p.sonraki())
            .await
            .is_err(),
        "kredi yokken paket gelmez"
    );
    assert_eq!(
        okunan.load(Ordering::SeqCst),
        2,
        "kredi yokken olay kuyruğundan okunmaz"
    );
    assert_eq!(kredi.yolda().0, 2);
    kredi.isledim(1).unwrap();
    let uc = p.sonraki().await.unwrap();
    assert_eq!(uc.no(), 3);
    assert_eq!(olaylar(&uc), vec![Olay::Hata { mesaj: "h2".into() }]);
}

#[tokio::test(start_paused = true)]
async fn kredi_bayt_siniri_kanal_mesaji_zarf_ekini_hesaba_katar() {
    let (tx, rx) = mpsc::unbounded_channel();
    let okunan = Arc::new(AtomicUsize::new(0));
    let mut u = Uretici {
        tx,
        oturum: OturumId::yeni(),
        tur: TurId::yeni(),
        sira: 0,
    };
    let zarf = |sira, mesaj: &str| Zarf {
        sema: SEMA_SURUMU,
        oturum: u.oturum,
        tur: Some(u.tur),
        sira,
        yuk: Olay::Hata {
            mesaj: mesaj.into(),
        },
    };
    let birinci_zarf = zarf(1, "bir");
    let ikinci_zarf = zarf(2, "iki");
    let kanal_bayti = |no, zarf| {
        serde_json::to_vec(&KanalMesaji::Olaylar {
            no,
            zarflar: vec![zarf],
        })
        .unwrap()
        .len()
    };
    let birinci_bayt = kanal_bayti(1, birinci_zarf.clone());
    let ikinci_bayt = kanal_bayti(2, ikinci_zarf.clone());
    let ikinci_zarf_bayt = serde_json::to_vec(&ikinci_zarf).unwrap().len();
    let kredi_bayt = birinci_bayt + ikinci_bayt - 1;
    assert!(
        birinci_bayt + ikinci_zarf_bayt <= kredi_bayt,
        "regresyon eski Zarf-only kontrolünü tetiklemeli"
    );
    assert!(
        birinci_bayt + ikinci_bayt > kredi_bayt,
        "tam kanal mesajı kredi sınırını aşmalı"
    );

    let ayar = AktarimAyarlari {
        kredi_bayt,
        ..AktarimAyarlari::default()
    };
    let kaynak = TestKaynagi {
        rx,
        okunan: okunan.clone(),
    };
    let (mut p, kredi) = Paketleyici::yeni(kaynak, ayar);
    u.hata("bir");
    u.hata("iki");

    let bir = p.sonraki().await.unwrap();
    assert_eq!(serde_json::to_vec(&bir).unwrap().len(), birinci_bayt);
    assert!(
        tokio::time::timeout(Duration::from_millis(1), p.sonraki())
            .await
            .is_err(),
        "tam kanal mesajı kalan krediye sığmayınca beklemeli"
    );
    assert_eq!(kredi.yolda(), (1, birinci_bayt));

    kredi.isledim(bir.no()).unwrap();
    let iki = p.sonraki().await.unwrap();
    assert_eq!(serde_json::to_vec(&iki).unwrap().len(), ikinci_bayt);
}

#[tokio::test(start_paused = true)]
async fn krediden_buyuk_olay_yolda_paket_yokken_gider() {
    let ayar = AktarimAyarlari {
        kredi_bayt: 1000,
        ..AktarimAyarlari::default()
    };
    let (mut u, mut p, kredi, _) = kur(ayar);
    let buyuk = "b".repeat(2000);
    u.hata("küçük");
    u.hata(&buyuk);
    let bir = p.sonraki().await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(1), p.sonraki())
            .await
            .is_err(),
        "büyük olay yolda paket varken gitmez"
    );
    kredi.isledim(bir.no()).unwrap();
    let iki = p.sonraki().await.unwrap();
    assert_eq!(olaylar(&iki), vec![Olay::Hata { mesaj: buyuk }]);
    assert_eq!(kredi.yolda().0, 1);
}

#[tokio::test]
async fn isledim_gonderilmemis_paketi_reddeder_ve_tekrari_zararsizdir() {
    let (mut u, mut p, kredi, _) = kur(AktarimAyarlari::default());
    u.hata("h");
    let m = p.sonraki().await.unwrap();
    assert_eq!(
        kredi.isledim(5),
        Err(IsledimHatasi::GonderilmemisPaket { no: 5, son: 1 })
    );
    kredi.isledim(m.no()).unwrap();
    kredi.isledim(m.no()).unwrap();
    assert_eq!(kredi.yolda(), (0, 0));
}

#[tokio::test(start_paused = true)]
async fn kaynak_kapaninca_kapandi_gelir_ve_numaralar_bosluksuzdur() {
    let (mut u, mut p, kredi, _) = kur(AktarimAyarlari::default());
    u.hata("a");
    u.hata("b");
    drop(u);
    let mut numaralar = Vec::new();
    let mut son = None;
    while let Some(m) = p.sonraki().await {
        numaralar.push(m.no());
        kredi.isledim(m.no()).unwrap();
        son = Some(m);
    }
    assert_eq!(numaralar, vec![1, 2, 3]);
    assert_eq!(son, Some(KanalMesaji::Kapandi { no: 3 }));
}

#[test]
fn kanal_mesaji_json_bicimi() {
    let z = Zarf {
        sema: SEMA_SURUMU,
        oturum: OturumId::yeni(),
        tur: None,
        sira: 4,
        yuk: Olay::TurBasladi,
    };
    let j = serde_json::to_value(KanalMesaji::Olaylar {
        no: 1,
        zarflar: vec![z],
    })
    .unwrap();
    assert_eq!(j["tip"], "Olaylar");
    assert_eq!(j["no"], 1);
    assert_eq!(j["zarflar"][0]["sira"], 4);
    assert_eq!(
        serde_json::to_value(KanalMesaji::Kapandi { no: 2 }).unwrap(),
        serde_json::json!({"tip": "Kapandi", "no": 2})
    );
}
