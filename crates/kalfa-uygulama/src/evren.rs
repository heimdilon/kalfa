//! EVREN sağlayıcısı. Anahtar değişince açık oturumlar da yenisini kullanır:
//! oturumlar sağlayıcıya, anahtarı çalışırken değiştirilebilen paylaşılan bir
//! sarmalayıcı üzerinden erişir (1c tasarımı §7.2).

use std::sync::{Arc, Mutex, RwLock};

use async_trait::async_trait;
use kalfa_core::saglayici::{EvrenSaglayici, Saglayici, SaglayiciAkisi};
use kalfa_evren::tipler::{ChatRequest, TermsStatus, TermsText};
use kalfa_evren::{ApiAnahtari, EvrenError, EvrenIstemcisi};

use crate::sozlesme::GunlukSinirDurumu;

/// EVREN'in taban adresini değiştirir (yerel denemeler). Yalnızca hata
/// ayıklama derlemesinde okunur: sürümde anahtar başka bir adrese gidemez.
pub const TABAN_DEGISKENI: &str = "KALFA_EVREN_TABAN";

/// Anahtardan sağlayıcı üretir. Uygulamada gerçek EVREN istemcisi; testlerde
/// anahtara göre davranan sahte sağlayıcı.
pub trait SaglayiciUretici: Send + Sync {
    fn uret(&self, anahtar: &str) -> Result<Arc<dyn Saglayici>, EvrenError>;
}

pub struct EvrenUretici {
    pub taban: String,
}

impl SaglayiciUretici for EvrenUretici {
    fn uret(&self, anahtar: &str) -> Result<Arc<dyn Saglayici>, EvrenError> {
        let istemci = EvrenIstemcisi::yeni(self.taban.clone(), ApiAnahtari::yeni(anahtar))?;
        Ok(Arc::new(EvrenSaglayici::yeni(istemci)))
    }
}

/// Anahtarı çalışırken değiştirilebilen sağlayıcı. Anahtar yokken her istek
/// `AnahtarGecersiz` döner.
#[derive(Default)]
pub struct DegisirSaglayici {
    ic: RwLock<Option<Arc<dyn Saglayici>>>,
    /// EVREN'in günlük token sınırı doldu (aşama 4 tasarımı §9.5). Sınır hesap
    /// geneldir: sohbet, denetçi ve eklenti isteklerinden biri ona çarpınca
    /// saklanır, biri başarılı olunca kalkar.
    gunluk_sinir: Mutex<Option<GunlukSinirDurumu>>,
}

impl DegisirSaglayici {
    pub fn ayarla(&self, saglayici: Arc<dyn Saglayici>) {
        *self.ic.write().unwrap_or_else(|z| z.into_inner()) = Some(saglayici);
    }

    fn simdiki(&self) -> Result<Arc<dyn Saglayici>, EvrenError> {
        self.ic
            .read()
            .unwrap_or_else(|z| z.into_inner())
            .clone()
            .ok_or(EvrenError::AnahtarGecersiz)
    }

    /// Günlük sınır dolduysa durumu; dolmadıysa `None`.
    pub fn gunluk_sinir(&self) -> Option<GunlukSinirDurumu> {
        self.gunluk_sinir
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .clone()
    }

    /// `akis`, `sohbet` ve `ek_uc` sonucunu izler; sonucu değiştirmez. Akışın
    /// içindeki hatalar kapsam dışıdır.
    fn izle<T>(&self, sonuc: Result<T, EvrenError>) -> Result<T, EvrenError> {
        let mut durum = self.gunluk_sinir.lock().unwrap_or_else(|z| z.into_inner());
        match &sonuc {
            Ok(_) => *durum = None,
            Err(EvrenError::GunlukSinir { sifirlanma }) => {
                *durum = Some(GunlukSinirDurumu {
                    sifirlanma: sifirlanma.clone(),
                });
            }
            Err(_) => {}
        }
        sonuc
    }
}

#[async_trait]
impl Saglayici for DegisirSaglayici {
    async fn akis(&self, istek: ChatRequest) -> Result<SaglayiciAkisi, EvrenError> {
        self.izle(self.simdiki()?.akis(istek).await)
    }

    async fn sohbet(
        &self,
        istek: ChatRequest,
    ) -> Result<(kalfa_evren::tipler::ChatResponse, kalfa_evren::YanitMeta), EvrenError> {
        self.izle(self.simdiki()?.sohbet(istek).await)
    }

    async fn sart_metni(&self) -> Result<TermsText, EvrenError> {
        self.simdiki()?.sart_metni().await
    }

    async fn sart_kabul(&self, surum: u32) -> Result<(), EvrenError> {
        self.simdiki()?.sart_kabul(surum).await
    }

    async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError> {
        self.simdiki()?.sart_durumu().await
    }

    async fn modeller(&self) -> Result<Vec<kalfa_evren::tipler::ModelInfo>, EvrenError> {
        self.simdiki()?.modeller().await
    }

    async fn ek_uc(
        &self,
        uc: kalfa_evren::EkUc,
        govde: &serde_json::Value,
    ) -> Result<kalfa_evren::HamYanit, EvrenError> {
        self.izle(self.simdiki()?.ek_uc(uc, govde).await)
    }
}
