//! Alan başına MCP keşif bağlantısı (Görev 9, tasarım §5.1, §7.1, §7.2).
//!
//! Her model turunun başında, aktif turu olan oturum alan kaynağından
//! denetimleri sahiplenirken planlayıcı etkin sunucuları okur. Her sunucu için
//! iş: başlat ya da doğrula (izin yoksa yönetici başlatma kartı bekletir,
//! araç gelmez), ilk keşifte ya da tarif değişince listeyi denetle, yoksa
//! yalnız gerekiyorsa yenile; denetimden geçen her araç için `McpArac` kur ve
//! yayımla. Başlamayan sunucu turu bekletmez; araçları o tur gelmez.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use kalfa_core::arac::{AracDenetimSahibi, KaynakAraci, McpBaslatmaSayaci, McpTarifi};
use kalfa_core::denetci::{DenetimKipi, GuvenlikDenetcisi};
use kalfa_mcp::arac::McpAracKimligi;

use crate::mcp::ayarlar::McpAyarlarDeposu;
use crate::mcp::denetim::{McpTanimDenetleyicisi, TanimButceSahibi};
use crate::mcp::kaynak::{McpAlanAracKaynagi, McpAlanPlani, McpAlanPlanlayici, McpDenetimIsi};
use crate::mcp::komutlar::McpArayuzKanali;
use crate::mcp::yonetici::{McpG7AracKaynagi, McpYoneticisi, YoneticiHatasi};

type BaslatmaGorevi =
    tokio::task::JoinHandle<Result<(McpBaslatmaSayaci, McpTarifi), YoneticiHatasi>>;
/// Planlamada başlayan görev; işi çağrılırsa bir kez alınır.
type Baslatma = Arc<Mutex<Option<BaslatmaGorevi>>>;

pub struct McpAlanBaglantisi {
    pub alan_kimligi: String,
    pub kok: PathBuf,
    pub yonetici: Arc<McpYoneticisi>,
    pub denetleyici: Arc<McpTanimDenetleyicisi>,
    pub mcp_depo: Arc<dyn McpAyarlarDeposu>,
    pub denetci: Arc<dyn GuvenlikDenetcisi>,
    pub denetci_kipi: Arc<RwLock<DenetimKipi>>,
    /// Başlatma kartı ya da yayım değişince arayüze görüntü yazılır (Görev 10).
    pub kanal: Option<Arc<McpArayuzKanali>>,
    /// Son yayımlanan listenin çalışma tarifi; değişirse liste yeniden denetlenir.
    tarifler: Mutex<BTreeMap<String, McpTarifi>>,
}

impl McpAlanBaglantisi {
    #[allow(clippy::too_many_arguments)]
    pub fn yeni(
        alan_kimligi: String,
        kok: PathBuf,
        yonetici: Arc<McpYoneticisi>,
        denetleyici: Arc<McpTanimDenetleyicisi>,
        mcp_depo: Arc<dyn McpAyarlarDeposu>,
        denetci: Arc<dyn GuvenlikDenetcisi>,
        denetci_kipi: Arc<RwLock<DenetimKipi>>,
        kanal: Option<Arc<McpArayuzKanali>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            alan_kimligi,
            kok,
            yonetici,
            denetleyici,
            mcp_depo,
            denetci,
            denetci_kipi,
            kanal,
            tarifler: Mutex::default(),
        })
    }

    pub fn planlayici(self: &Arc<Self>) -> McpAlanPlanlayici {
        let ben = Arc::clone(self);
        Arc::new(move |kaynak| ben.planla(kaynak))
    }

    fn planla(self: &Arc<Self>, kaynak: &McpAlanAracKaynagi) -> McpAlanPlani {
        let etkinler: std::collections::BTreeSet<String> = self
            .mcp_depo
            .mcp_ayarlarini_oku()
            .0
            .sunucular
            .into_iter()
            .filter(|s| s.etkin)
            .map(|s| s.ad)
            .collect();
        let isler = etkinler
            .iter()
            .map(|sunucu| {
                let ilk = !kaynak.yayimda_mi(sunucu);
                let baslatma = self.baslatmayi_baslat(sunucu);
                (sunucu.clone(), self.is(sunucu.clone(), ilk, baslatma))
            })
            .collect();
        McpAlanPlani { etkinler, isler }
    }

    /// Başlatma bütçe harcamaz ve tura bağlı değildir; planlama anında ayrı
    /// görevde başlar. Tur, iş hiç çalışmadan bitse de başlatma (ve izin yoksa
    /// başlatma kartı) tamamlanır. Yalnız bütçeli keşif turun iptaline bağlıdır.
    fn baslatmayi_baslat(self: &Arc<Self>, sunucu: &str) -> Baslatma {
        let ben = Arc::clone(self);
        let sunucu = sunucu.to_owned();
        let gorev = tokio::spawn(async move {
            let sonuc = ben
                .yonetici
                .baslat_veya_dogrula(&ben.alan_kimligi, &ben.kok, &sunucu)
                .await;
            // Başlatma kartı model dışında, arayüzde görünür (§5.1).
            if matches!(sonuc, Err(YoneticiHatasi::BaslatmaIzniGerekli(_))) {
                ben.bildir();
            }
            sonuc
        });
        Arc::new(Mutex::new(Some(gorev)))
    }

    fn is(self: &Arc<Self>, sunucu: String, ilk: bool, baslatma: Baslatma) -> McpDenetimIsi {
        let ben = Arc::clone(self);
        Arc::new(move |sahip| {
            let ben = Arc::clone(&ben);
            let sunucu = sunucu.clone();
            let gorev = baslatma.lock().unwrap_or_else(|z| z.into_inner()).take();
            Box::pin(async move { ben.hazirla(&sunucu, ilk, sahip, gorev?).await })
        })
    }

    /// `None`: değişiklik yok ya da keşif bu tur tamamlanamadı; önceki yayım kalır.
    async fn hazirla(
        &self,
        sunucu: &str,
        ilk: bool,
        sahip: AracDenetimSahibi,
        baslatma: BaslatmaGorevi,
    ) -> Option<Vec<KaynakAraci>> {
        // Başlamayan sunucunun (izin yok/geri alındı, alan kaldırılıyor, yalnız
        // İnceleme oturumu, çökme) araçları sunulmaz; sonraki başlatma listeyi
        // yeniden denetler.
        let Ok(Ok((_, tarif))) = baslatma.await else {
            self.tarifler
                .lock()
                .unwrap_or_else(|z| z.into_inner())
                .remove(sunucu);
            return Some(Vec::new());
        };
        let butce = TanimButceSahibi::yeni(sahip.oturum, sahip.butce, sahip.iptal);
        let kipi = *self.denetci_kipi.read().unwrap_or_else(|z| z.into_inner());
        let tarif_degisti = self
            .tarifler
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .get(sunucu)
            != Some(&tarif);
        let rapor = if ilk || tarif_degisti {
            self.denetleyici
                .sunucu_araclarini_kesfet_ve_denetle(
                    &self.alan_kimligi,
                    sunucu,
                    tarif,
                    kipi,
                    &self.denetci,
                    Some(&butce),
                )
                .await
                .ok()?
        } else {
            self.denetleyici
                .gerekiyorsa_yenile(
                    &self.alan_kimligi,
                    sunucu,
                    tarif,
                    kipi,
                    &self.denetci,
                    Some(&butce),
                )
                .await
                .ok()??
        };

        let g7: Arc<dyn McpG7AracKaynagi> = self.denetleyici.clone();
        let mut araclar = Vec::new();
        for tanim in &rapor.sunulan_araclar {
            let kimlik = McpAracKimligi::yeni(
                self.alan_kimligi.clone(),
                self.kok.clone(),
                tanim.sunucu_etiketi().clone(),
                tanim.arac_etiketi().clone(),
            );
            // Kurulamayan araç (ör. kuşak bu arada değişti) yalnız kendisi düşer.
            let Ok(arac) = self.yonetici.arac_olustur(kimlik, Arc::clone(&g7)).await else {
                continue;
            };
            if let Ok(girdi) = KaynakAraci::yeni(arac) {
                araclar.push(girdi);
            }
        }
        self.tarifler
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .insert(sunucu.to_owned(), tarif);
        self.bildir();
        Some(araclar)
    }

    fn bildir(&self) {
        if let Some(kanal) = &self.kanal {
            kanal.yay();
        }
    }
}
