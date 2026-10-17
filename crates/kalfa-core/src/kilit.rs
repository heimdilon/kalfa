//! Yol bazında okuma/yazma kilitleri ve çalışma alanı başına kilit yöneticisi
//! (spec §6.1, 1c tasarımı §5.3).
//!
//! Önce genel kilit (paylaşımlı ya da özel), sonra yol kilitleri anahtar
//! sırasıyla alınır. Sıralı edinim kilitlenmeyi (deadlock) önler.

use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::Path;
use std::sync::Weak;
use std::sync::{Arc, Mutex};

use same_file::Handle;
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};

use crate::yol::CalismaYolu;

type KokTablosu = Mutex<Vec<(Handle, Weak<KaynakKilitleri>)>>;

#[derive(Default)]
pub struct KaynakKilitleri {
    genel: Arc<RwLock<()>>,
    yollar: Mutex<HashMap<String, Arc<RwLock<()>>>>,
    kok_tablosu: Option<Weak<KokTablosu>>,
}

/// Tutulan kilitler; bırakılınca (drop) çözülür.
pub struct KilitSeti {
    _genel: Tutucu,
    _yollar: Vec<Tutucu>,
    /// MCP cleanup kaydı yaşarken kök yöneticisinin Weak tablosu düşmesin.
    /// Sıradan araçların `al()` yolu bu alanı boş bırakır.
    _sahip: Option<Arc<KaynakKilitleri>>,
}

impl std::fmt::Debug for KilitSeti {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KilitSeti")
            .field("genel", &self._genel)
            .field("yollar", &self._yollar)
            .field("sahip_tutuldu", &self._sahip.is_some())
            .finish()
    }
}

/// Kilit tutucular yalnızca bırakılmak için saklanır.
#[derive(Debug)]
#[allow(dead_code)]
enum Tutucu {
    Okuma(OwnedRwLockReadGuard<()>),
    Yazma(OwnedRwLockWriteGuard<()>),
}

impl KaynakKilitleri {
    pub fn yeni() -> Self {
        Self::default()
    }

    fn kilit(&self, anahtar: &str) -> Arc<RwLock<()>> {
        let mut harita = self.yollar.lock().unwrap_or_else(|z| z.into_inner());
        // Kimsenin tutmadığı ve beklemediği kilitler atılır; tablo büyümez.
        harita.retain(|_, kilit| Arc::strong_count(kilit) > 1);
        harita.entry(anahtar.to_string()).or_default().clone()
    }

    /// Yol kilidi tablosundaki girdi sayısı (testler için).
    #[cfg(feature = "test-destek")]
    pub fn yol_tablosu_boyutu(&self) -> usize {
        self.yollar.lock().unwrap_or_else(|z| z.into_inner()).len()
    }

    pub async fn al(
        &self,
        okunan: &[CalismaYolu],
        yazilan: &[CalismaYolu],
        tum_calisma_alani: bool,
    ) -> KilitSeti {
        let genel = if tum_calisma_alani {
            Tutucu::Yazma(self.genel.clone().write_owned().await)
        } else {
            Tutucu::Okuma(self.genel.clone().read_owned().await)
        };
        let mut istek: BTreeMap<String, bool> = BTreeMap::new();
        for yol in okunan {
            istek.entry(yol.anahtar()).or_insert(false);
        }
        for yol in yazilan {
            istek.insert(yol.anahtar(), true);
        }
        let mut yollar = Vec::with_capacity(istek.len());
        for (anahtar, yazma) in istek {
            let kilit = self.kilit(&anahtar);
            yollar.push(if yazma {
                Tutucu::Yazma(kilit.write_owned().await)
            } else {
                Tutucu::Okuma(kilit.read_owned().await)
            });
        }
        KilitSeti {
            _genel: genel,
            _yollar: yollar,
            _sahip: None,
        }
    }

    /// Kilitlerle birlikte bu çalışma alanının kaynak yöneticisini de yaşatır.
    /// MCP manager çağrı kaydı kilidi devraldığında kökün Weak tablosu
    /// düşmemeli; sıradan araçlar mevcut `al()` yolunu kullanmaya devam eder.
    pub async fn sahipli_al(
        self: &Arc<Self>,
        okunan: &[CalismaYolu],
        yazilan: &[CalismaYolu],
        tum_calisma_alani: bool,
    ) -> KilitSeti {
        let mut kilit = self.al(okunan, yazilan, tum_calisma_alani).await;
        kilit._sahip = Some(self.clone());
        kilit
    }
}

impl Drop for KaynakKilitleri {
    fn drop(&mut self) {
        let Some(kok_tablosu) = self.kok_tablosu.as_ref().and_then(Weak::upgrade) else {
            return;
        };
        let mut kokler = kok_tablosu.lock().unwrap_or_else(|z| z.into_inner());
        // Son oturum bırakıldığında Weak kaydı ve dosya sistemi Handle'ı da
        // bırakılır; sonraki bir kök erişimini beklemez.
        kokler.retain(|(_, kilitler)| kilitler.strong_count() > 0);
    }
}

/// Çalışma alanı kökünü dosya sistemi kimliğiyle tanır ve aynı köke aynı
/// kaynak kilitlerini verir. Kilitler süreç içidir; iç içe çalışma alanları
/// ayrı yönetici alır (bilinen sınır).
#[derive(Default)]
pub struct KilitYoneticisi {
    kokler: Arc<KokTablosu>,
}

impl KilitYoneticisi {
    pub fn yeni() -> Self {
        Self::default()
    }

    /// `kok` dizininin kilitleri. Aynı kökü açan bütün oturumlar aynı
    /// kilitleri paylaşır; son oturum bırakınca kökün kaydı silinir.
    pub fn al(&self, kok: &Path) -> io::Result<Arc<KaynakKilitleri>> {
        let kimlik = Handle::from_path(kok)?;
        let mut kokler = self.kokler.lock().unwrap_or_else(|z| z.into_inner());
        kokler.retain(|(_, kilitler)| kilitler.strong_count() > 0);
        let mevcut = kokler
            .iter()
            .find(|(h, _)| *h == kimlik)
            .and_then(|(_, kilitler)| kilitler.upgrade());
        if let Some(kilitler) = mevcut {
            return Ok(kilitler);
        }
        let mut yonetilen = KaynakKilitleri::default();
        yonetilen.kok_tablosu = Some(Arc::downgrade(&self.kokler));
        let kilitler = Arc::new(yonetilen);
        kokler.push((kimlik, Arc::downgrade(&kilitler)));
        Ok(kilitler)
    }

    /// Hâlâ kullanılan kök sayısı; aynı anda ölü kayıtları da temizler.
    pub fn acik_kok_sayisi(&self) -> usize {
        let mut kokler = self.kokler.lock().unwrap_or_else(|z| z.into_inner());
        kokler.retain(|(_, kilitler)| kilitler.strong_count() > 0);
        kokler.len()
    }
}

#[cfg(test)]
mod tests {
    use super::KilitYoneticisi;

    #[test]
    fn son_arc_dropu_registry_kaydini_yontem_cagrisi_olmadan_siler() {
        let d = tempfile::tempdir().unwrap();
        let y = KilitYoneticisi::yeni();
        let kilitler = y.al(d.path()).unwrap();
        assert_eq!(y.kokler.lock().unwrap().len(), 1);

        drop(kilitler);

        assert_eq!(
            y.kokler.lock().unwrap().len(),
            0,
            "temizlik son Arc drop'unda yapılmalı"
        );
    }

    #[tokio::test]
    async fn sahipli_guard_son_oturumdan_sonra_ayni_kok_kilitlerini_yasatir() {
        let d = tempfile::tempdir().unwrap();
        let y = KilitYoneticisi::yeni();
        let kaynaklar = y.al(d.path()).unwrap();
        let guard = kaynaklar.sahipli_al(&[], &[], true).await;
        drop(kaynaklar);

        // Guard'ın tuttuğu Arc sayesinde Weak kayıt yükselir; yeni oturum
        // ayrı bir kilit tablosu kuramaz ve aynı genel yazma kilidinde bekler.
        let yeni_oturum = y.al(d.path()).unwrap();
        let mut ikinci = tokio::spawn(async move { yeni_oturum.al(&[], &[], true).await });
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(25), &mut ikinci)
                .await
                .is_err(),
            "cleanup guard'ı yaşarken aynı kökte yeni yazma başlamamalı"
        );

        drop(guard);
        let _ikinci = ikinci.await.unwrap();
    }
}
