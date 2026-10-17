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

use kalfa_core::arac::{AracDenetimSahibi, KaynakAraci, McpTarifi};
use kalfa_core::denetci::{DenetimKipi, GuvenlikDenetcisi};
use kalfa_mcp::arac::McpAracKimligi;

use crate::mcp::ayarlar::McpAyarlarDeposu;
use crate::mcp::denetim::{McpTanimDenetleyicisi, TanimButceSahibi};
use crate::mcp::kaynak::{McpAlanAracKaynagi, McpAlanPlani, McpAlanPlanlayici, McpDenetimIsi};
use crate::mcp::yonetici::{McpG7AracKaynagi, McpYoneticisi};

pub struct McpAlanBaglantisi {
    pub alan_kimligi: String,
    pub kok: PathBuf,
    pub yonetici: Arc<McpYoneticisi>,
    pub denetleyici: Arc<McpTanimDenetleyicisi>,
    pub mcp_depo: Arc<dyn McpAyarlarDeposu>,
    pub denetci: Arc<dyn GuvenlikDenetcisi>,
    pub denetci_kipi: Arc<RwLock<DenetimKipi>>,
    /// Son yayımlanan listenin çalışma tarifi; değişirse liste yeniden denetlenir.
    tarifler: Mutex<BTreeMap<String, McpTarifi>>,
}

impl McpAlanBaglantisi {
    pub fn yeni(
        alan_kimligi: String,
        kok: PathBuf,
        yonetici: Arc<McpYoneticisi>,
        denetleyici: Arc<McpTanimDenetleyicisi>,
        mcp_depo: Arc<dyn McpAyarlarDeposu>,
        denetci: Arc<dyn GuvenlikDenetcisi>,
        denetci_kipi: Arc<RwLock<DenetimKipi>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            alan_kimligi,
            kok,
            yonetici,
            denetleyici,
            mcp_depo,
            denetci,
            denetci_kipi,
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
                (sunucu.clone(), self.is(sunucu.clone(), ilk))
            })
            .collect();
        McpAlanPlani { etkinler, isler }
    }

    fn is(self: &Arc<Self>, sunucu: String, ilk: bool) -> McpDenetimIsi {
        let ben = Arc::clone(self);
        Arc::new(move |sahip| {
            let ben = Arc::clone(&ben);
            let sunucu = sunucu.clone();
            Box::pin(async move { ben.hazirla(&sunucu, ilk, sahip).await })
        })
    }

    /// `None`: değişiklik yok ya da keşif bu tur tamamlanamadı; önceki yayım kalır.
    async fn hazirla(
        &self,
        sunucu: &str,
        ilk: bool,
        sahip: AracDenetimSahibi,
    ) -> Option<Vec<KaynakAraci>> {
        // Başlamayan sunucunun (izin yok/geri alındı, alan kaldırılıyor, yalnız
        // İnceleme oturumu, çökme) araçları sunulmaz; sonraki başlatma listeyi
        // yeniden denetler.
        let Ok((_, tarif)) = self
            .yonetici
            .baslat_veya_dogrula(&self.alan_kimligi, &self.kok, sunucu)
            .await
        else {
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
        Some(araclar)
    }
}
