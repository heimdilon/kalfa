//! Araç sözleşmesi (spec §6, §7). Yerleşik araç ile eklenti aracı arasında
//! izin açısından fark yoktur: ikisi de bu trait'in arkasındadır.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use kalfa_evren::tipler::ToolDef;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::izin::Kip;
use crate::kabuk::KabukTarifi;
use crate::kimlik::CagriId;
use crate::protokol::CiktiAkisi;
use crate::yol::{CalismaYolu, YolHatasi};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YazmaHedefi {
    pub yol: CalismaYolu,
    /// Dosyanın şu anki içeriğinin SHA-256 özeti; dosya yoksa `None`.
    pub onceki_ozet: Option<String>,
    /// Hard link sayısı; dosya yoksa 1.
    pub baglanti_sayisi: u64,
}

/// Yürütmeden önce aracın ne yapacağının tarifi: izin kararı, onay kartı ve
/// onay mührü bundan üretilir.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hazirlik {
    pub okunan: Vec<CalismaYolu>,
    pub yazilan: Vec<YazmaHedefi>,
    /// Hangi yollara dokunacağı önceden bilinemeyen iş (kabuk, aşama 2).
    pub tum_calisma_alani: bool,
    /// Sade Türkçe tek cümle: "`a.txt` dosyasını okumak istiyor."
    pub aciklama: String,
    pub diff: Option<String>,
    /// Kabuk çağrısının tarifi (aşama 2 tasarımı §3.3).
    pub kabuk: Option<KabukTarifi>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AracCiktisi {
    pub icerik: String,
    /// Modele giden içeriğin kaynak etiketi (spec §9), örn. `dosya:src/a.rs:1-40`.
    pub kaynak: String,
    /// Kullanıcıya ayrıca gösterilen güvenlik uyarıları (güvenlik analizi
    /// Bulgu 1): örneğin komutun oluşturduğu tehlikeli `.git`'in karantinası.
    /// Modele de `[güvenlik] …` satırıyla gider.
    pub guvenlik_uyarilari: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AracHatasi {
    #[error("{0}")]
    Gecersiz(String),
    #[error(transparent)]
    Yol(#[from] YolHatasi),
    #[error("Dosya onaydan sonra değişti; işlem yapılmadı.")]
    OnayEskidi,
    /// Kabuk tarifi onaydan sonra değişti; ileti nedeni söyler.
    #[error("{0}; komut çalıştırılmadı. Gerekirse yeniden dene.")]
    KomutKosuluDegisti(String),
    #[error(
        "İzin kipi veya hedefin izin koşulları değişti; işlem yapılmadı. Gerekirse yeniden dene."
    )]
    PolitikaDegisti,
    #[error("İptal edildi.")]
    Iptal,
    #[error("{0}")]
    GirisCikis(String),
    /// Araç çalıştı ama başarısız oldu; çıktı yine de modele gider. Kabukta
    /// sıfırdan farklı çıkış kodu ve zaman aşımı böyledir. Aynı çağrının
    /// üst üste başarısızlık sınırına sayılmaz (aşama 2 tasarımı §4.5):
    /// düzeltmeden sonra aynı testi yeniden koşmak olağandır.
    #[error("{}", .0.icerik)]
    Basarisiz(AracCiktisi),
}

/// Süren bir aracın ara çıktı parçası.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiktiParcasi {
    pub cagri: CagriId,
    pub akis: CiktiAkisi,
    pub metin: String,
    /// Bu parçadan önce atılan baytların toplamı.
    pub atlanan_bayt: u64,
}

/// Aracın ara çıktısını oturuma taşır (aşama 2 tasarımı §4.4). Göndermek
/// hiç beklemez: kanal doluysa parça atılır ve atılan bayt sayısı sonraki
/// parçaya yazılır. Aracın modele verdiği sonuç bundan etkilenmez.
#[derive(Debug, Clone)]
pub struct CiktiGonderici {
    ic: Option<(CagriId, mpsc::Sender<CiktiParcasi>)>,
    atlanan: Arc<AtomicU64>,
}

impl CiktiGonderici {
    /// Hiçbir yere göndermeyen gönderici.
    pub fn bos() -> Self {
        Self {
            ic: None,
            atlanan: Arc::default(),
        }
    }

    /// `cagri`'nın parçalarını `tx`'e gönderen gönderici.
    pub fn yeni(cagri: CagriId, tx: mpsc::Sender<CiktiParcasi>) -> Self {
        Self {
            ic: Some((cagri, tx)),
            atlanan: Arc::default(),
        }
    }

    /// Oturumun, araç bittikten sonra atılan baytları okuması için.
    pub(crate) fn atlanan_sayaci(&self) -> Arc<AtomicU64> {
        self.atlanan.clone()
    }

    pub fn gonder(&self, akis: CiktiAkisi, metin: String) {
        let Some((cagri, tx)) = &self.ic else {
            return;
        };
        let atlanan = self.atlanan.swap(0, Ordering::SeqCst);
        let bayt = metin.len() as u64;
        let parca = CiktiParcasi {
            cagri: *cagri,
            akis,
            metin,
            atlanan_bayt: atlanan,
        };
        if tx.try_send(parca).is_err() {
            self.atlanan.fetch_add(atlanan + bayt, Ordering::SeqCst);
        }
    }
}

impl From<std::io::Error> for AracHatasi {
    fn from(hata: std::io::Error) -> Self {
        AracHatasi::GirisCikis(hata.to_string())
    }
}

#[async_trait]
pub trait Arac: Send + Sync {
    fn ad(&self) -> &str;
    fn aciklama(&self) -> &str;
    fn parametre_semasi(&self) -> Value;
    /// Dosya yazar ya da yan etkisi olan bir iş yapar mı? İnceleme kipinde
    /// yazan araçlar modele hiç sunulmaz (spec §7.5).
    fn yazar_mi(&self) -> bool;
    /// Araç şu an modele sunulur mu? Her turda sorulur: sunulmayan araç
    /// tanımlanmaz ve çağrılırsa çalışmaz. Örneğin yalıtım yokken ve
    /// yalıtımsız çalıştırma kapalıyken kabuk (aşama 2 tasarımı §9.8).
    fn sunulur_mu(&self) -> bool {
        true
    }
    async fn hazirla(&self, argumanlar: &Value) -> Result<Hazirlik, AracHatasi>;
    async fn calistir(
        &self,
        argumanlar: &Value,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi>;

    /// Ara çıktı üreten araçlar bunu uygular; oturum araçları bununla
    /// çalıştırır. Varsayılanı `calistir`'dır ve ara çıktı üretmez.
    async fn calistir_akisli(
        &self,
        argumanlar: &Value,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
        _cikti: CiktiGonderici,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.calistir(argumanlar, hazirlik, iptal).await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KayitHatasi {
    #[error("`{0}` adlı araç iki kez kaydedildi.")]
    Yinelenen(String),
    #[error("`{arac}` aracının parametre şeması geçersiz: {hata}")]
    GecersizSema { arac: String, hata: String },
}

pub struct AracKayitDefteri {
    araclar: Vec<Arc<dyn Arac>>,
    semalar: HashMap<String, jsonschema::Validator>,
}

impl AracKayitDefteri {
    pub fn yeni(araclar: Vec<Arc<dyn Arac>>) -> Result<Self, KayitHatasi> {
        let mut semalar = HashMap::new();
        for arac in &araclar {
            if semalar.contains_key(arac.ad()) {
                return Err(KayitHatasi::Yinelenen(arac.ad().to_string()));
            }
            let dogrulayici = jsonschema::validator_for(&arac.parametre_semasi()).map_err(|e| {
                KayitHatasi::GecersizSema {
                    arac: arac.ad().to_string(),
                    hata: e.to_string(),
                }
            })?;
            semalar.insert(arac.ad().to_string(), dogrulayici);
        }
        Ok(Self { araclar, semalar })
    }

    pub fn bul(&self, ad: &str) -> Option<Arc<dyn Arac>> {
        self.araclar.iter().find(|a| a.ad() == ad).cloned()
    }

    pub fn tanimlar(&self, kip: Kip) -> Vec<ToolDef> {
        self.araclar
            .iter()
            .filter(|a| a.sunulur_mu() && (kip != Kip::Inceleme || !a.yazar_mi()))
            .map(|a| ToolDef::function(a.ad(), a.aciklama(), a.parametre_semasi()))
            .collect()
    }

    /// Argüman metnini JSON olarak çözer ve aracın şemasına karşı doğrular.
    /// Hata metni modele gider; model argümanı düzeltip yeniden deneyebilir.
    pub fn dogrula(&self, ad: &str, argumanlar: &str) -> Result<Value, String> {
        let dogrulayici = self
            .semalar
            .get(ad)
            .ok_or_else(|| format!("`{ad}` adlı bir araç yok."))?;
        let metin = if argumanlar.trim().is_empty() {
            "{}"
        } else {
            argumanlar
        };
        let deger: Value = serde_json::from_str(metin)
            .map_err(|e| format!("Argümanlar geçerli JSON değil: {e}"))?;
        let hatalar: Vec<String> = dogrulayici
            .iter_errors(&deger)
            .map(|e| format!("{}: {}", e.instance_path(), e))
            .collect();
        if hatalar.is_empty() {
            Ok(deger)
        } else {
            Err(format!("Argümanlar şemaya uymuyor: {}", hatalar.join("; ")))
        }
    }
}
