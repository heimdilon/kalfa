use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use kalfa_core::arac::{
    Arac, AracCiktisi, AracHatasi, AracOrtami, Hazirlik, McpAgKarti, McpAracHazirligi,
    McpAracTanimOzeti, McpBaslatmaSayaci, McpErisim, McpKartEtiketi, McpKorumaOzeti,
    McpPolitikaOzeti, McpTanimOzeti, McpTarifi, McpYalitimProfiliOzeti,
};
use kalfa_core::butce::ButceSinirlari;
use kalfa_core::izin::Kip;
use kalfa_core::oturum::OturumAyarlari;
use kalfa_core::protokol::{AracDurumu, BitisNedeni, Komut, Olay, OnayKarari, OnayOgesi};
use kalfa_core::test_destek::*;
use kalfa_evren::tipler::{ChatMessage, Role};
use serde_json::{Value, json};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

const MCP_ARAC_ADI: &str = "mcp__deneme__oku";
const MCP_TAZE_ONAY: &str = "MCP hazırlığı onaydan sonra değişti; güncel kartı yeniden onaylayın.";

struct McpDurumu {
    hazirlama: AtomicUsize,
    rpc: AtomicUsize,
    temizleme: AtomicUsize,
    tamamlama: AtomicUsize,
    finalizasyon_basladi: Notify,
    finalizasyon_bitti: AtomicBool,
}

struct McpAraci {
    durumu: Arc<McpDurumu>,
    bir_kez_eskisin: bool,
    gizli_okuma: bool,
    finalizasyon_izni: Option<Arc<Notify>>,
}

impl McpAraci {
    fn yeni(
        bir_kez_eskisin: bool,
        gizli_okuma: bool,
        finalizasyon_izni: Option<Arc<Notify>>,
    ) -> (Self, Arc<McpDurumu>) {
        let durumu = Arc::new(McpDurumu {
            hazirlama: AtomicUsize::new(0),
            rpc: AtomicUsize::new(0),
            temizleme: AtomicUsize::new(0),
            tamamlama: AtomicUsize::new(0),
            finalizasyon_basladi: Notify::new(),
            finalizasyon_bitti: AtomicBool::new(false),
        });
        (
            Self {
                durumu: durumu.clone(),
                bir_kez_eskisin,
                gizli_okuma,
                finalizasyon_izni,
            },
            durumu,
        )
    }

    fn hazirlik(&self, surum: u8) -> Hazirlik {
        let tarif = McpTarifi::yeni(
            McpTanimOzeti::ozetten([1; 32]),
            McpPolitikaOzeti::ozetten([2; 32]),
            McpKorumaOzeti::ozetten([3; 32]),
            McpYalitimProfiliOzeti::ozetten([4; 32]),
            McpBaslatmaSayaci::yeni(5),
        );
        let mut hazirlik = Hazirlik {
            mcp: Some(McpAracHazirligi {
                sunucu: McpKartEtiketi::yeni("deneme-sunucusu").unwrap(),
                arac: McpKartEtiketi::yeni("oku").unwrap(),
                kip: Kip::Serbest,
                erisim: McpErisim::Yalitimli,
                ag: McpAgKarti::Kapali,
                ilk_cagri: false,
                tarif,
                arac_tanimi: McpAracTanimOzeti::ozetten([surum; 32]),
            }),
            aciklama: "Deneme MCP aracını çağırmak istiyor.".into(),
            ..Default::default()
        };
        if self.gizli_okuma {
            hazirlik
                .okunan
                .push(kalfa_core::yol::CalismaYolu::coz(".env").unwrap());
        }
        hazirlik
    }
}

#[async_trait::async_trait]
impl Arac for McpAraci {
    fn ad(&self) -> &str {
        MCP_ARAC_ADI
    }

    fn aciklama(&self) -> &str {
        "MCP actor test aracı"
    }

    fn parametre_semasi(&self) -> Value {
        json!({"type": "object"})
    }

    fn yazar_mi(&self) -> bool {
        false
    }

    async fn hazirla(&self, _argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
        let sira = self.durumu.hazirlama.fetch_add(1, Ordering::SeqCst);
        let surum = if self.bir_kez_eskisin && sira == 0 {
            1
        } else {
            2
        };
        Ok(self.hazirlik(surum))
    }

    async fn calistir(
        &self,
        _argumanlar: &Value,
        _hazirlik: &Hazirlik,
        _iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.durumu.rpc.fetch_add(1, Ordering::SeqCst);
        Ok(AracCiktisi {
            icerik: "MCP deneme sonucu".into(),
            kaynak: "mcp:deneme".into(),
            ..Default::default()
        })
    }

    async fn sonuc_isleme_bitti(
        &self,
        _cagri: kalfa_core::kimlik::CagriId,
    ) -> Result<(), AracHatasi> {
        self.durumu.finalizasyon_basladi.notify_one();
        if let Some(izin) = &self.finalizasyon_izni {
            izin.notified().await;
        }
        self.durumu.tamamlama.fetch_add(1, Ordering::SeqCst);
        self.durumu.finalizasyon_bitti.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn sonuc_isleme_iptal(&self, _cagri: kalfa_core::kimlik::CagriId) {
        self.durumu.temizleme.fetch_add(1, Ordering::SeqCst);
    }

    async fn sonuc_isleme_iptal_ve_bekle(
        &self,
        _cagri: kalfa_core::kimlik::CagriId,
    ) -> Result<(), AracHatasi> {
        self.durumu.temizleme.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

struct ButceBekleyen {
    basladi: Notify,
    bitti: Notify,
}

#[async_trait::async_trait]
impl Arac for ButceBekleyen {
    fn ad(&self) -> &str {
        "butce_izni"
    }

    fn aciklama(&self) -> &str {
        "Bütçe izni bekler"
    }

    fn parametre_semasi(&self) -> Value {
        json!({"type": "object"})
    }

    fn yazar_mi(&self) -> bool {
        false
    }

    async fn hazirla(&self, _argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
        Ok(Hazirlik::default())
    }

    async fn calistir(
        &self,
        _argumanlar: &Value,
        _hazirlik: &Hazirlik,
        _iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        Ok(AracCiktisi::default())
    }

    async fn calistir_ortamli(
        &self,
        _argumanlar: &Value,
        _hazirlik: &Hazirlik,
        _iptal: CancellationToken,
        ortam: AracOrtami,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.basladi.notify_one();
        let izin = ortam.butce_izni("MCP actor testi").await;
        self.bitti.notify_one();
        izin.map(|_| AracCiktisi::default())
            .map_err(|()| AracHatasi::Iptal)
    }
}

fn ayarlar(kip: Kip) -> OturumAyarlari {
    let mut ayar = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    ayar.kip = kip;
    ayar
}

fn mesaj() -> Komut {
    Komut::KullaniciMesaji {
        metin: "MCP aracını çağır".into(),
    }
}

async fn onay_bekle(driver: &mut Surucu) -> Vec<OnayOgesi> {
    match driver
        .bekle(|olay| matches!(olay, Olay::OnayGerekiyor { .. }))
        .await
        .yuk
    {
        Olay::OnayGerekiyor { ogeler, .. } => ogeler,
        diger => unreachable!("{diger:?}"),
    }
}

fn onayla(oge: &OnayOgesi) -> Komut {
    Komut::AracOnayi {
        cagri: oge.cagri,
        karar: OnayKarari::Onayla,
        muhur: oge.muhur.clone(),
        kisayol: false,
    }
}

fn tool_yanitlari(istek: &[ChatMessage]) -> Vec<&ChatMessage> {
    istek
        .iter()
        .filter(|mesaj| mesaj.role == Role::Tool)
        .collect()
}

fn sonuc_durumlari(driver: &Surucu) -> Vec<AracDurumu> {
    driver
        .olaylar()
        .into_iter()
        .filter_map(|olay| match olay {
            Olay::AracSonucu { durum, .. } => Some(durum),
            _ => None,
        })
        .collect()
}

fn oturum(saglayici: Arc<SahteSaglayici>, araclar: Vec<Arc<dyn Arac>>, kip: Kip) -> Surucu {
    test_oturumu(
        test_baglami(saglayici, araclar, sanal_siniflandirici()),
        ayarlar(kip),
    )
}

#[tokio::test]
async fn taze_onay_uyarisi_baz_karardan_ayridir_ve_rpc_ile_model_yaniti_bir_kezdir() {
    let (arac, durum) = McpAraci::yeni(true, true, None);
    let saglayici = SahteSaglayici::yeni(vec![
        SahteYanit::arac(MCP_ARAC_ADI, json!({})),
        SahteYanit::metin("tamam"),
    ]);
    let arac: Arc<dyn Arac> = Arc::new(arac);
    let mut driver = oturum(saglayici.clone(), vec![arac], Kip::Serbest);

    driver.gonder(mesaj()).await;
    let ilk = onay_bekle(&mut driver).await;
    assert_eq!(ilk.len(), 1);
    driver.gonder(onayla(&ilk[0])).await;

    let taze = onay_bekle(&mut driver).await;
    assert_eq!(taze.len(), 1);
    assert!(taze[0].uyarilar.iter().any(|uyari| uyari == MCP_TAZE_ONAY));
    assert_eq!(
        durum.rpc.load(Ordering::SeqCst),
        0,
        "taze kart onaylanmadan RPC yok"
    );
    driver.gonder(onayla(&taze[0])).await;

    assert_eq!(driver.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(durum.rpc.load(Ordering::SeqCst), 1);
    assert_eq!(durum.temizleme.load(Ordering::SeqCst), 1);
    assert_eq!(durum.tamamlama.load(Ordering::SeqCst), 1);
    assert_eq!(
        sonuc_durumlari(&driver),
        vec![AracDurumu::IptalEdildi, AracDurumu::Basarili],
        "eski çağrı yeniden onay gerektirir, taze çağrı tek RPC yapar"
    );
    let istekler = saglayici.istekler();
    assert_eq!(istekler.len(), 2);
    let yanitlar = tool_yanitlari(&istekler[1].messages);
    assert_eq!(
        yanitlar.len(),
        1,
        "model bağlamında tek MCP tool yanıtı olmalı"
    );
    assert!(
        yanitlar[0]
            .content
            .as_deref()
            .unwrap()
            .contains("MCP deneme sonucu")
    );
}

#[tokio::test]
async fn serbest_mcp_taze_karari_yalniz_kartta_onaya_yukseltilir() {
    let (arac, durum) = McpAraci::yeni(true, false, None);
    let saglayici = SahteSaglayici::yeni(vec![
        SahteYanit::arac(MCP_ARAC_ADI, json!({})),
        SahteYanit::metin("tamam"),
    ]);
    let arac: Arc<dyn Arac> = Arc::new(arac);
    let mut driver = oturum(saglayici.clone(), vec![arac], Kip::Serbest);

    driver.gonder(mesaj()).await;
    let taze = onay_bekle(&mut driver).await;
    assert_eq!(taze.len(), 1);
    assert_eq!(taze[0].uyarilar, vec![MCP_TAZE_ONAY]);
    driver.gonder(onayla(&taze[0])).await;

    assert_eq!(driver.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(durum.rpc.load(Ordering::SeqCst), 1);
    let istekler = saglayici.istekler();
    let yanitlar = tool_yanitlari(&istekler[1].messages);
    assert_eq!(yanitlar.len(), 1);
}

#[tokio::test]
async fn yavas_mcp_finalizasyonunda_butce_istegi_ve_iptal_islenir() {
    let finalizasyon_izni = Arc::new(Notify::new());
    let (arac, durum) = McpAraci::yeni(false, false, Some(finalizasyon_izni.clone()));
    let butce = Arc::new(ButceBekleyen {
        basladi: Notify::new(),
        bitti: Notify::new(),
    });
    let saglayici = SahteSaglayici::yeni(vec![SahteYanit::araclar(vec![
        (MCP_ARAC_ADI, json!({})),
        ("butce_izni", json!({})),
    ])]);
    let arac: Arc<dyn Arac> = Arc::new(arac);
    let butce_araci: Arc<dyn Arac> = butce.clone();
    let mut ayar = ayarlar(Kip::Serbest);
    ayar.butce = ButceSinirlari {
        istek: 1,
        token: 1_000_000,
    };
    let mut driver = test_oturumu(
        test_baglami(saglayici, vec![arac, butce_araci], sanal_siniflandirici()),
        ayar,
    );

    driver.gonder(mesaj()).await;
    let tur = driver
        .bekle(|olay| matches!(olay, Olay::TurBasladi))
        .await
        .tur
        .unwrap();
    tokio::time::timeout(
        Duration::from_secs(2),
        durum.finalizasyon_basladi.notified(),
    )
    .await
    .expect("MCP finalizasyonu başlamalı");
    tokio::time::timeout(Duration::from_secs(2), butce.basladi.notified())
        .await
        .expect("bütçe aracı başlamalı");
    driver
        .bekle(|olay| matches!(olay, Olay::ButceKarariGerekli { .. }))
        .await;

    driver.gonder(Komut::Iptal { tur }).await;
    tokio::time::timeout(Duration::from_secs(2), butce.bitti.notified())
        .await
        .expect("iptal bekleyen bütçe iznini çözmeli");
    assert!(
        !durum.finalizasyon_bitti.load(Ordering::SeqCst),
        "actor diğer işleri işlerken MCP finalizasyonu sürmeli"
    );

    finalizasyon_izni.notify_one();
    assert_eq!(driver.tur_sonu().await, BitisNedeni::Iptal);
    assert!(durum.finalizasyon_bitti.load(Ordering::SeqCst));
    assert_eq!(
        durum.tamamlama.load(Ordering::SeqCst),
        1,
        "tamamlama guard'ı yalnızca son denetim tamamlanınca kapanmalı"
    );
    assert!(
        !driver.olaylar().iter().any(|olay| matches!(
            olay,
            Olay::AracSonucu { icerik, .. } if icerik.contains("MCP deneme sonucu")
        )),
        "iptalden sonra doğrulanmamış ham MCP sonucu yayımlanmamalı"
    );
}

struct ErisimAraci {
    ic: McpAraci,
    erisim: McpErisim,
    ilk: bool,
    ag: McpAgKarti,
}

#[async_trait::async_trait]
impl Arac for ErisimAraci {
    fn ad(&self) -> &str {
        self.ic.ad()
    }
    fn aciklama(&self) -> &str {
        self.ic.aciklama()
    }
    fn parametre_semasi(&self) -> Value {
        self.ic.parametre_semasi()
    }
    fn yazar_mi(&self) -> bool {
        true
    }
    async fn hazirla(&self, argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
        let mut h = self.ic.hazirla(argumanlar).await?;
        let mcp = h.mcp.as_mut().unwrap();
        mcp.erisim = self.erisim;
        mcp.ilk_cagri = self.ilk;
        mcp.ag = self.ag.clone();
        h.tum_calisma_alani = true;
        Ok(h)
    }
    async fn calistir(
        &self,
        argumanlar: &Value,
        h: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.ic.calistir(argumanlar, h, iptal).await
    }
}

#[tokio::test]
async fn mcp_cagri_izinleri_gercek_actor_yolunda_dort_kip_iki_erisim_uygular() {
    for erisim in [McpErisim::Yalitimli, McpErisim::TamErisim] {
        for kip in [Kip::Siki, Kip::Proje, Kip::Serbest, Kip::Inceleme] {
            let (ic, durum) = McpAraci::yeni(false, false, None);
            let saglayici = SahteSaglayici::yeni(vec![
                SahteYanit::arac(MCP_ARAC_ADI, json!({})),
                SahteYanit::metin("tamam"),
                SahteYanit::arac(MCP_ARAC_ADI, json!({})),
                SahteYanit::metin("ikinci"),
            ]);
            let arac: Arc<dyn Arac> = Arc::new(ErisimAraci {
                ic,
                erisim,
                ilk: false,
                ag: McpAgKarti::Kapali,
            });
            let mut d = oturum(saglayici, vec![arac], kip);
            let tekrar = if erisim == McpErisim::TamErisim && kip != Kip::Inceleme {
                2
            } else {
                1
            };
            for sira in 0..tekrar {
                d.gonder(mesaj()).await;
                if kip != Kip::Inceleme && (erisim == McpErisim::TamErisim || kip != Kip::Serbest) {
                    let ogeler = tokio::time::timeout(Duration::from_secs(2), onay_bekle(&mut d))
                        .await
                        .expect("MCP izni gerçek actor yolunda onay istemeli");
                    assert_eq!(
                        durum.rpc.load(Ordering::SeqCst),
                        sira,
                        "onaydan önce RPC yok"
                    );
                    assert_eq!(ogeler[0].yerel_onay, erisim == McpErisim::TamErisim);
                    if erisim == McpErisim::TamErisim {
                        assert!(ogeler[0].kisayol.is_none());
                        d.gonder(Komut::YerelOnay {
                            cagri: ogeler[0].cagri,
                            muhur: ogeler[0].muhur.clone(),
                        })
                        .await;
                    } else {
                        d.gonder(onayla(&ogeler[0])).await;
                    }
                }
                assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
            }
            assert_eq!(
                durum.rpc.load(Ordering::SeqCst),
                if kip == Kip::Inceleme { 0 } else { tekrar }
            );
        }
    }
}

#[tokio::test]
async fn serbest_mcp_ilk_cagri_ve_ag_gercek_actor_yolunda_onay_ister() {
    for (ilk, ag) in [(true, McpAgKarti::Kapali), (false, McpAgKarti::Sinirsiz)] {
        let (ic, durum) = McpAraci::yeni(false, false, None);
        let s = SahteSaglayici::yeni(vec![
            SahteYanit::arac(MCP_ARAC_ADI, json!({})),
            SahteYanit::metin("tamam"),
        ]);
        let a: Arc<dyn Arac> = Arc::new(ErisimAraci {
            ic,
            erisim: McpErisim::Yalitimli,
            ilk,
            ag,
        });
        let mut d = oturum(s, vec![a], Kip::Serbest);
        d.gonder(mesaj()).await;
        let ogeler = tokio::time::timeout(Duration::from_secs(2), onay_bekle(&mut d))
            .await
            .expect("ilk çağrı ve ağ gerçek actor yolunda onay istemeli");
        assert_eq!(durum.rpc.load(Ordering::SeqCst), 0);
        assert!(!ogeler[0].uyarilar.is_empty());
        assert!(ogeler[0].kisayol.is_none());
        d.gonder(onayla(&ogeler[0])).await;
        assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
        assert_eq!(durum.rpc.load(Ordering::SeqCst), 1);
    }
}
