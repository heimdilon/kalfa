use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use async_trait::async_trait;
use kalfa_core::saglayici::{Saglayici, SaglayiciAkisi};
use kalfa_core::sart_kapisi::{
    GerekliSonucu, IlkDurum, KabulSonucu, KapiGorunumu, SartKapisi, SartKarari, SartMetni,
};
use kalfa_core::test_destek::SahteSaglayici;
use kalfa_evren::EvrenError;
use kalfa_evren::tipler::{ChatRequest, TermsStatus, TermsText};
use tokio::sync::oneshot;

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

async fn gerekli(k: &SartKapisi) -> (SartMetni, u64) {
    match k.gerekli(k.durum().nesil).await.unwrap() {
        GerekliSonucu::OnayGerekli { metin, nesil } => (metin, nesil),
        GerekliSonucu::KabulEdildi { .. } | GerekliSonucu::Ret => {
            panic!("bu çağrı yeni şart gösterimi bekliyordu")
        }
    }
}

struct Blok {
    basladi: oneshot::Sender<()>,
    surdur: oneshot::Receiver<()>,
}

struct KapiliSaglayici {
    surum: AtomicU32,
    kabul_edilen: Mutex<Vec<u32>>,
    kabul_cagrisi: AtomicUsize,
    durum_blok: Mutex<Option<Blok>>,
    kabul_blok: Mutex<Option<Blok>>,
}

impl KapiliSaglayici {
    fn yeni() -> Arc<Self> {
        Arc::new(Self {
            surum: AtomicU32::new(1),
            kabul_edilen: Mutex::new(Vec::new()),
            kabul_cagrisi: AtomicUsize::new(0),
            durum_blok: Mutex::new(None),
            kabul_blok: Mutex::new(None),
        })
    }

    fn blokla_durum(&self) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (basladi_tx, basladi_rx) = oneshot::channel();
        let (surdur_tx, surdur_rx) = oneshot::channel();
        *self.durum_blok.lock().unwrap() = Some(Blok {
            basladi: basladi_tx,
            surdur: surdur_rx,
        });
        (basladi_rx, surdur_tx)
    }

    fn blokla_kabul(&self) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (basladi_tx, basladi_rx) = oneshot::channel();
        let (surdur_tx, surdur_rx) = oneshot::channel();
        *self.kabul_blok.lock().unwrap() = Some(Blok {
            basladi: basladi_tx,
            surdur: surdur_rx,
        });
        (basladi_rx, surdur_tx)
    }

    async fn bekle(blok: &Mutex<Option<Blok>>) {
        let blok = blok.lock().unwrap().take();
        if let Some(blok) = blok {
            let _ = blok.basladi.send(());
            let _ = blok.surdur.await;
        }
    }
}

#[async_trait]
impl Saglayici for KapiliSaglayici {
    async fn akis(&self, _istek: ChatRequest) -> Result<SaglayiciAkisi, EvrenError> {
        Err(EvrenError::ag_baglanti(
            "test sağlayıcısında model akışı yok",
        ))
    }

    async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError> {
        Self::bekle(&self.durum_blok).await;
        let surum = self.surum.load(Ordering::SeqCst);
        Ok(TermsStatus {
            current_version: surum,
            is_material: true,
            accepted: self.kabul_edilen.lock().unwrap().contains(&surum),
            doc_path: None,
        })
    }

    async fn sart_metni(&self) -> Result<TermsText, EvrenError> {
        Ok(TermsText {
            version: self.surum.load(Ordering::SeqCst),
            content: "gated terms".into(),
            doc_path: None,
        })
    }

    async fn sart_kabul(&self, surum: u32) -> Result<(), EvrenError> {
        self.kabul_cagrisi.fetch_add(1, Ordering::SeqCst);
        Self::bekle(&self.kabul_blok).await;
        let guncel = self.surum.load(Ordering::SeqCst);
        if surum != guncel {
            return Err(EvrenError::SartKabulGerekli {
                surum: Some(guncel),
            });
        }
        self.kabul_edilen.lock().unwrap().push(surum);
        Ok(())
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
    let (m1, n1) = gerekli(&k).await;
    let (m2, n2) = gerekli(&k).await;
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
        KabulSonucu::Bosta,
        "gösterilmemiş metin kabul edilemez"
    );
    gerekli(&k).await;
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
    gerekli(&k).await;
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
    let (metin, nesil) = gerekli(&k).await;
    let bekleyen = k.karar_bekle(nesil);

    assert!(k.reddet());
    let (_, yeni_nesil) = gerekli(&k).await;
    assert_eq!(yeni_nesil, nesil + 1);
    assert_eq!(k.kabul_et(metin.surum).await.unwrap(), KabulSonucu::Kabul);

    assert_eq!(
        bekleyen.await,
        SartKarari::Ret,
        "eski tur ret kararını kaçırıp yeni kabulü kullanmamalı"
    );
    assert_eq!(
        k.gerekli(nesil).await.unwrap(),
        GerekliSonucu::Ret,
        "eski 403 ret ve yeniden açılmadan sonraki kabulü kullanmamalı"
    );
}

#[tokio::test]
async fn ret_bekleyenlere_iletilir_ve_sonraki_istekte_metin_yeniden_gelir() {
    let s = SahteSaglayici::yeni(vec![]);
    let k = Arc::new(kapi(&s));
    let (_, n) = gerekli(&k).await;
    let bekleyen = {
        let k = k.clone();
        tokio::spawn(async move { k.karar_bekle(n).await })
    };
    assert!(k.reddet());
    assert_eq!(bekleyen.await.unwrap(), SartKarari::Ret);
    assert_eq!(k.durum().gorunum, KapiGorunumu::Reddedildi);
    let (m, _) = gerekli(&k).await;
    assert_eq!(m, beklenen(1));
    assert_eq!(k.durum().gorunum, KapiGorunumu::Gosteriliyor(beklenen(1)));
}

#[tokio::test]
async fn istekten_sonra_gelen_kabul_gec_403u_karsilar_yeni_istegi_karsilamaz() {
    let s = KapiliSaglayici::yeni();
    let k = SartKapisi::yeni(s.clone());
    let istek_nesli = k.durum().nesil;
    let (metin, _) = gerekli(&k).await;
    assert_eq!(k.kabul_et(metin.surum).await.unwrap(), KabulSonucu::Kabul);

    assert_eq!(
        k.gerekli(istek_nesli).await.unwrap(),
        GerekliSonucu::KabulEdildi { surum: 1 },
        "önceki istekten geç dönen 403 aynı başarılı kabulü kullanır"
    );

    s.surum.store(2, Ordering::SeqCst);
    let yeni_istek_nesli = k.durum().nesil;
    assert_eq!(
        k.gerekli(yeni_istek_nesli).await.unwrap(),
        GerekliSonucu::OnayGerekli {
            metin: SartMetni {
                surum: 2,
                esasli: true,
                metin: "gated terms".into(),
            },
            nesil: yeni_istek_nesli,
        },
        "kabulden sonra başlayan istek yeni sürümü göstermeli"
    );
}

#[tokio::test]
async fn kabul_sirasinda_ikinci_403_ayni_karara_katilir_ve_cift_kabul_gondermez() {
    let s = KapiliSaglayici::yeni();
    let k = Arc::new(SartKapisi::yeni(s.clone()));
    let eski_istek_nesli = k.durum().nesil;
    let (metin, nesil) = gerekli(&k).await;
    let (basladi, surdur) = s.blokla_kabul();

    let kabul = {
        let k = k.clone();
        tokio::spawn(async move { k.kabul_et(metin.surum).await.unwrap() })
    };
    basladi.await.unwrap();

    let ikinci = k.gerekli(eski_istek_nesli).await.unwrap();
    assert!(matches!(ikinci, GerekliSonucu::OnayGerekli { nesil: n, .. } if n == nesil));
    let ikinci_kabul = {
        let k = k.clone();
        tokio::spawn(async move { k.kabul_et(1).await.unwrap() })
    };

    surdur.send(()).unwrap();
    assert_eq!(kabul.await.unwrap(), KabulSonucu::Kabul);
    assert_eq!(ikinci_kabul.await.unwrap(), KabulSonucu::Kabul);
    assert_eq!(s.kabul_cagrisi.load(Ordering::SeqCst), 1);
    assert_eq!(k.karar_bekle(nesil).await, SartKarari::Kabul { surum: 1 });
}

#[tokio::test]
async fn eski_metin_yenilemesi_ret_kararini_atomik_olarak_korur() {
    let s = KapiliSaglayici::yeni();
    let k = Arc::new(SartKapisi::yeni(s.clone()));
    gerekli(&k).await;
    s.surum.store(2, Ordering::SeqCst);
    let (basladi, surdur) = s.blokla_durum();

    let yenile = {
        let k = k.clone();
        tokio::spawn(async move { k.kabul_et(1).await.unwrap() })
    };
    basladi.await.unwrap();
    assert!(k.reddet());
    surdur.send(()).unwrap();

    assert_eq!(yenile.await.unwrap(), KabulSonucu::Ret);
    assert_eq!(k.durum().gorunum, KapiGorunumu::Reddedildi);
    assert_eq!(k.durum().son_karar, Some(SartKarari::Ret));
}

#[tokio::test]
async fn ret_kabul_postu_surerken_gelse_de_kapida_ustun_kalir() {
    let s = KapiliSaglayici::yeni();
    let k = Arc::new(SartKapisi::yeni(s.clone()));
    let (metin, _) = gerekli(&k).await;
    let (basladi, surdur) = s.blokla_kabul();

    let kabul = {
        let k = k.clone();
        tokio::spawn(async move { k.kabul_et(metin.surum).await.unwrap() })
    };
    basladi.await.unwrap();
    assert!(k.reddet());
    surdur.send(()).unwrap();

    assert_eq!(kabul.await.unwrap(), KabulSonucu::Ret);
    assert_eq!(k.durum().gorunum, KapiGorunumu::Reddedildi);
    assert_eq!(k.durum().son_karar, Some(SartKarari::Ret));
    assert_eq!(s.kabul_cagrisi.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn gecikmis_ret_bosta_kapiyi_kilitlemez() {
    let s = KapiliSaglayici::yeni();
    let k = SartKapisi::yeni(s);
    let (metin, nesil) = gerekli(&k).await;
    assert_eq!(k.kabul_et(metin.surum).await.unwrap(), KabulSonucu::Kabul);

    assert!(
        !k.reddet(),
        "kabulden sonraki eski ret tıklaması no-op olmalı"
    );
    assert_eq!(k.durum().gorunum, KapiGorunumu::Bosta);
    assert_eq!(
        k.durum().son_karar,
        Some(SartKarari::Kabul { surum: metin.surum })
    );
    assert_eq!(k.karar_bekle(nesil).await, SartKarari::Kabul { surum: 1 });
}
