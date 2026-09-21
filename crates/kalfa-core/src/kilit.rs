//! Yol bazında okuma/yazma kilitleri (spec §6.1).
//!
//! Önce genel kilit (paylaşımlı ya da özel), sonra yol kilitleri anahtar
//! sırasıyla alınır. Sıralı edinim kilitlenmeyi (deadlock) önler.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};

use crate::yol::CalismaYolu;

#[derive(Debug, Default)]
pub struct KaynakKilitleri {
    genel: Arc<RwLock<()>>,
    yollar: Mutex<HashMap<String, Arc<RwLock<()>>>>,
}

/// Tutulan kilitler; bırakılınca (drop) çözülür.
#[derive(Debug)]
pub struct KilitSeti {
    _genel: Tutucu,
    _yollar: Vec<Tutucu>,
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
        harita.entry(anahtar.to_string()).or_default().clone()
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
        }
    }
}
