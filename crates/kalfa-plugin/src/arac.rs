//! Eklenti aracı (aşama 4 tasarımı §8.1): yerleşik araçlarla aynı izin
//! motorundan geçer.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use kalfa_core::arac::{
    Arac, AracCiktisi, AracHatasi, CiktiGonderici, EklentiTarifi, Hazirlik, kirp,
};
use kalfa_core::kilit::KaynakKilitleri;
use kalfa_core::protokol::{CiktiAkisi, VeriCikisi};
use kalfa_tools::CalismaAlani;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::bag::kalfa::plugin::tipler::Hata;
use crate::calistirici::{CagriHatasi, Eklenti, Motor};
use crate::depo::Depo;
use crate::host::CagriOrtami;
use crate::sinir;

pub struct EklentiAraci {
    motor: Arc<Motor>,
    eklenti: Arc<Eklenti>,
    /// `eklenti.araclar` içindeki sırası.
    sira: usize,
    model_adi: String,
    alan: Arc<CalismaAlani>,
    kilitler: Arc<KaynakKilitleri>,
    /// Bu eklentinin bu çalışma alanındaki deposu (§10).
    depo_dizini: PathBuf,
}

impl EklentiAraci {
    /// Eklentinin bütün araçları; bir çalışma alanının oturumu için.
    /// `depo_dizini` eklentiye ve çalışma alanına özeldir.
    pub fn hepsi(
        motor: &Arc<Motor>,
        eklenti: &Arc<Eklenti>,
        alan: &Arc<CalismaAlani>,
        kilitler: &Arc<KaynakKilitleri>,
        depo_dizini: &Path,
    ) -> Vec<Arc<dyn Arac>> {
        (0..eklenti.araclar.len())
            .map(|sira| {
                Arc::new(Self {
                    motor: motor.clone(),
                    eklenti: eklenti.clone(),
                    sira,
                    model_adi: eklenti.manifest.model_adi(&eklenti.araclar[sira].ad),
                    alan: alan.clone(),
                    kilitler: kilitler.clone(),
                    depo_dizini: depo_dizini.to_path_buf(),
                }) as Arc<dyn Arac>
            })
            .collect()
    }

    fn arac_adi(&self) -> &str {
        &self.eklenti.araclar[self.sira].ad
    }

    fn tarif(&self) -> EklentiTarifi {
        let m = &self.eklenti.manifest;
        EklentiTarifi {
            ad: m.ad.clone(),
            surum: m.surum.to_string(),
            ozet: self.eklenti.ozet.clone(),
            globlar: m.dosya_oku.clone(),
            evren_uclari: m.evren.iter().map(|u| u.ad().to_string()).collect(),
        }
    }

    fn hata_iletisi(&self, h: &CagriHatasi) -> String {
        let ad = &self.eklenti.manifest.ad;
        let ileti = match h {
            CagriHatasi::Eklenti(h) => {
                let (tur, ileti) = match h {
                    Hata::IzinYok(m) => ("izin-yok", m.as_str()),
                    Hata::Bulunamadi(m) => ("bulunamadi", m.as_str()),
                    Hata::IptalEdildi => ("iptal-edildi", ""),
                    Hata::SinirAsildi(m) => ("sinir-asildi", m.as_str()),
                    Hata::Evren(m) => ("evren", m.as_str()),
                    Hata::Diger(m) => ("diger", m.as_str()),
                };
                format!("Eklenti hatası ({ad}): {tur}: {ileti}")
            }
            CagriHatasi::Bellek => format!(
                "Eklenti durduruldu ({ad}): bellek sınırı ({} MiB) aşıldı.",
                sinir::BELLEK / (1024 * 1024)
            ),
            CagriHatasi::Cpu => format!("Eklenti durduruldu ({ad}): işlem süresi sınırı aşıldı."),
            CagriHatasi::Sure => format!("Eklenti durduruldu ({ad}): süre sınırı aşıldı."),
            CagriHatasi::Cokme => format!("Eklenti çalışırken çöktü ({ad})."),
            CagriHatasi::Iptal => "İptal edildi.".into(),
        };
        kirp(&ileti, sinir::HATA_ILETISI)
    }
}

#[async_trait]
impl Arac for EklentiAraci {
    fn ad(&self) -> &str {
        &self.model_adi
    }

    fn aciklama(&self) -> &str {
        &self.eklenti.araclar[self.sira].aciklama
    }

    fn parametre_semasi(&self) -> Value {
        self.eklenti.araclar[self.sira].sema.clone()
    }

    /// v1 eklentileri yazamaz; İnceleme kipinde de sunulur.
    fn yazar_mi(&self) -> bool {
        false
    }

    async fn hazirla(&self, _argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
        Ok(Hazirlik {
            aciklama: format!(
                "{} eklentisinin `{}` aracını çalıştırmak istiyor.",
                self.eklenti.manifest.ad,
                self.arac_adi()
            ),
            veri_cikisi: Some(VeriCikisi::EklentiCiktisi),
            eklenti: Some(self.tarif()),
            kilitsiz: true,
            ..Default::default()
        })
    }

    async fn calistir(
        &self,
        argumanlar: &Value,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.calistir_akisli(argumanlar, hazirlik, iptal, CiktiGonderici::bos())
            .await
    }

    async fn calistir_akisli(
        &self,
        argumanlar: &Value,
        _hazirlik: &Hazirlik,
        iptal: CancellationToken,
        cikti: CiktiGonderici,
    ) -> Result<AracCiktisi, AracHatasi> {
        let ortam = CagriOrtami::yeni(
            self.eklenti.manifest.clone(),
            self.alan.clone(),
            self.kilitler.clone(),
            Depo::yeni(self.depo_dizini.clone(), self.eklenti.manifest.depo_mb),
            iptal,
            cikti.clone(),
        );
        let (sonuc, stderr) = self
            .eklenti
            .calistir(&self.motor, self.arac_adi(), &argumanlar.to_string(), ortam)
            .await;
        match sonuc {
            Ok(icerik) => Ok(AracCiktisi {
                icerik: kirp(&icerik, sinir::SONUC),
                kaynak: format!("eklenti:{}:{}", self.eklenti.manifest.ad, self.arac_adi()),
                ..Default::default()
            }),
            Err(CagriHatasi::Iptal) => Err(AracHatasi::Iptal),
            Err(h) => {
                // Tuzakta `stderr`'in sonu (panik iletisi) yalnızca canlı
                // çıktıya gider; modele gitmez (§7.3).
                if !matches!(h, CagriHatasi::Eklenti(_)) && !stderr.is_empty() {
                    cikti.gonder(CiktiAkisi::Hata, stderr);
                }
                Err(AracHatasi::Gecersiz(self.hata_iletisi(&h)))
            }
        }
    }
}
