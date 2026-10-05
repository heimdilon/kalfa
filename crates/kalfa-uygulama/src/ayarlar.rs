//! Uygulama ayarları (aşama 3 tasarımı §7.6; 3c Görev 7): veri dizinindeki
//! `ayarlar.json`. Bu görevde yalnızca denetçi kipi vardır; 3e gizli
//! kalıpları ve ağ listesini ekler. Dosya veri dizinindedir, yalıtımın içinde
//! görünmez; depo içindeki hiçbir dosya ayarı değiştiremez.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use kalfa_core::denetci::DenetimKipi;
use serde::{Deserialize, Serialize};

use crate::hata::KomutHatasi;
use crate::sozlesme::DenetciDurumu;
use crate::uygulama::Uygulama;

const DOSYA_ADI: &str = "ayarlar.json";

const DUSURME_ONAYI: &str = "Denetçi kipini düşürmek, dış kaynaklı araç sonuçlarının \
(dosya içeriği, komut çıktısı, arama sonuçları) modele daha az denetimle gitmesi demektir. \
Denetçi izinlerin yerine geçmez, ama içeriğe gömülü talimatları yakalayabilir. Devam etmek \
istiyor musunuz?";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Dosya {
    #[serde(default)]
    denetci_kipi: DenetimKipi,
}

pub struct Ayarlar {
    yol: PathBuf,
    dosya: Mutex<Dosya>,
    /// Dosya okunamadıysa nedeni; varsayılanlar kullanılır.
    hata: Mutex<Option<String>>,
    /// Açık oturumlar her turun başında buradan okur (Görev 7).
    denetci_kipi: Arc<RwLock<DenetimKipi>>,
}

impl Ayarlar {
    /// Dosya yoksa varsayılanlar sessizce kullanılır. Bozuksa da varsayılanlar
    /// kullanılır, ama neden Ayarlar'da görünür.
    pub fn oku(veri_dizini: &Path) -> Self {
        let yol = veri_dizini.join(DOSYA_ADI);
        let (dosya, hata) = match std::fs::read(&yol) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Dosya::default(), None),
            Err(e) => (
                Dosya::default(),
                Some(format!(
                    "ayarlar.json okunamadı ({e}); varsayılanlar kullanılıyor."
                )),
            ),
            Ok(veri) => match serde_json::from_slice(&veri) {
                Ok(d) => (d, None),
                Err(e) => (
                    Dosya::default(),
                    Some(format!(
                        "ayarlar.json bozuk ({e}); varsayılanlar kullanılıyor."
                    )),
                ),
            },
        };
        Self {
            yol,
            denetci_kipi: Arc::new(RwLock::new(dosya.denetci_kipi)),
            dosya: Mutex::new(dosya),
            hata: Mutex::new(hata),
        }
    }

    fn kilit(&self) -> std::sync::MutexGuard<'_, Dosya> {
        self.dosya.lock().unwrap_or_else(|z| z.into_inner())
    }

    /// Oturumların paylaştığı kip kaynağı.
    pub fn denetci_kipi_kaynagi(&self) -> Arc<RwLock<DenetimKipi>> {
        self.denetci_kipi.clone()
    }

    pub fn denetci_durumu(&self) -> DenetciDurumu {
        DenetciDurumu {
            kip: self.kilit().denetci_kipi,
            ayar_hatasi: self.hata.lock().unwrap_or_else(|z| z.into_inner()).clone(),
        }
    }

    /// Dosyayı atomik yazar (`yalitim.json` gibi); yazılamazsa eski hâl kalır.
    fn degistir(&self, is: impl FnOnce(&mut Dosya)) -> Result<(), KomutHatasi> {
        let mut dosya = self.kilit();
        let mut yeni = dosya.clone();
        is(&mut yeni);
        let metin = serde_json::to_vec_pretty(&yeni).map_err(|e| KomutHatasi::ic(e.to_string()))?;
        if let Some(ust) = self.yol.parent() {
            std::fs::create_dir_all(ust)?;
        }
        let gecici = self.yol.with_extension("json.yeni");
        std::fs::write(&gecici, metin)?;
        std::fs::rename(&gecici, &self.yol)?;
        *self.denetci_kipi.write().unwrap_or_else(|z| z.into_inner()) = yeni.denetci_kipi;
        *dosya = yeni;
        *self.hata.lock().unwrap_or_else(|z| z.into_inner()) = None;
        Ok(())
    }
}

/// Kipler sıkılık sırasıyla: kapalı < uyar < katı.
fn sikilik(kip: DenetimKipi) -> u8 {
    match kip {
        DenetimKipi::Kapali => 0,
        DenetimKipi::Uyar => 1,
        DenetimKipi::Kati => 2,
    }
}

impl Uygulama {
    pub fn denetci_kipi(&self) -> DenetciDurumu {
        self.ayarlar.denetci_durumu()
    }

    /// Kipi yükseltmek onaysızdır. Düşürmek yerel pencerede onaylanır;
    /// vazgeçilirse kip değişmez (kural `yalitimsizi_ayarla`'dakiyle aynı).
    pub async fn denetci_kipini_ayarla(
        &self,
        kip: DenetimKipi,
    ) -> Result<DenetciDurumu, KomutHatasi> {
        let simdiki = self.ayarlar.denetci_durumu().kip;
        if sikilik(kip) < sikilik(simdiki)
            && !self
                .iletisim
                .onayla("Denetçi kipini düşür", DUSURME_ONAYI, "Düşür", "Vazgeç")
                .await
        {
            return Ok(self.ayarlar.denetci_durumu());
        }
        self.ayarlar.degistir(|d| d.denetci_kipi = kip)?;
        Ok(self.ayarlar.denetci_durumu())
    }
}
