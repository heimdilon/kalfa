//! Çalışma alanlarının kaydı (aşama 2 tasarımı §4.2): kabuğun ilk çağrısı
//! onaylanmış çalışma alanları. Kayıt uygulamanın veri dizinindedir; kabuk
//! komutları onu göremez ve değiştiremez.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use kalfa_tools::KabukGuveni;
use serde::{Deserialize, Serialize};

const DOSYA_ADI: &str = "calisma_alanlari.json";

#[derive(Debug, Default, Serialize, Deserialize)]
struct Dosya {
    #[serde(default)]
    kabuk_onaylanan: BTreeSet<String>,
}

pub struct CalismaAlaniKaydi {
    yol: PathBuf,
    /// İlk okumada dosyadan yüklenir.
    icerik: Mutex<Option<Dosya>>,
}

impl CalismaAlaniKaydi {
    pub fn yeni(veri_dizini: &Path) -> Self {
        Self {
            yol: veri_dizini.join(DOSYA_ADI),
            icerik: Mutex::new(None),
        }
    }

    /// Çalışma alanının kimliği: kanonik yolunun SHA-256'sından 16 onaltılık
    /// karakter.
    /// Yalıtımın çalışma alanına özel dizinleri de bu kimliği kullanır.
    pub fn kimlik(kok: &Path) -> String {
        kalfa_sandbox::alan_kimligi(kok)
    }

    fn ile<T>(&self, is: impl FnOnce(&mut Dosya) -> T) -> T {
        let mut icerik = self.icerik.lock().unwrap_or_else(|z| z.into_inner());
        let dosya = icerik.get_or_insert_with(|| {
            // Okunamayan ya da bozuk kayıt boş sayılır: en kötü durumda ilk
            // çağrı uyarısı yeniden gösterilir.
            std::fs::read(&self.yol)
                .ok()
                .and_then(|v| serde_json::from_slice(&v).ok())
                .unwrap_or_default()
        });
        is(dosya)
    }

    pub fn kabuk_onaylandi_mi(&self, kimlik: &str) -> bool {
        self.ile(|d| d.kabuk_onaylanan.contains(kimlik))
    }

    /// Kaydı yazar. Yazılamazsa kayıt bellekte kalır; sonraki açılışta ilk
    /// çağrı uyarısı yeniden gösterilir.
    pub fn kabuk_onayla(&self, kimlik: &str) {
        let metin = self.ile(|d| {
            d.kabuk_onaylanan.insert(kimlik.to_string());
            serde_json::to_vec_pretty(d)
        });
        let Ok(metin) = metin else { return };
        if let Some(ust) = self.yol.parent() {
            let _ = std::fs::create_dir_all(ust);
        }
        let gecici = self.yol.with_extension("json.yeni");
        if std::fs::write(&gecici, metin).is_ok() {
            let _ = std::fs::rename(&gecici, &self.yol);
        }
    }
}

/// Bir çalışma alanının kabuk güveni; kabuk aracına verilir.
pub(crate) struct AlanGuveni {
    pub kayit: Arc<CalismaAlaniKaydi>,
    pub kimlik: String,
}

impl KabukGuveni for AlanGuveni {
    fn guvenildi_mi(&self) -> bool {
        self.kayit.kabuk_onaylandi_mi(&self.kimlik)
    }

    fn guvenildi(&self) {
        if !self.guvenildi_mi() {
            self.kayit.kabuk_onayla(&self.kimlik);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn onay_kalici_ve_calisma_alanina_ozeldir() {
        let veri = tempfile::tempdir().unwrap();
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let (ka, kb) = (
            CalismaAlaniKaydi::kimlik(a.path()),
            CalismaAlaniKaydi::kimlik(b.path()),
        );
        assert_ne!(ka, kb);
        assert_eq!(ka.len(), 16);

        let kayit = CalismaAlaniKaydi::yeni(veri.path());
        assert!(!kayit.kabuk_onaylandi_mi(&ka));
        kayit.kabuk_onayla(&ka);
        assert!(kayit.kabuk_onaylandi_mi(&ka));
        assert!(!kayit.kabuk_onaylandi_mi(&kb));

        let yeniden = CalismaAlaniKaydi::yeni(veri.path());
        assert!(
            yeniden.kabuk_onaylandi_mi(&ka),
            "kayıt yeniden açılışta okunur"
        );
    }

    #[test]
    fn bozuk_kayit_bos_sayilir() {
        let veri = tempfile::tempdir().unwrap();
        std::fs::write(veri.path().join(DOSYA_ADI), "{bozuk").unwrap();
        let kayit = CalismaAlaniKaydi::yeni(veri.path());
        assert!(!kayit.kabuk_onaylandi_mi("abc"));
        kayit.kabuk_onayla("abc");
        assert!(CalismaAlaniKaydi::yeni(veri.path()).kabuk_onaylandi_mi("abc"));
    }
}
