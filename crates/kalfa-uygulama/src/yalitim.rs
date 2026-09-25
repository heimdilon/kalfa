//! Yalıtımın uygulama tarafı (aşama 2 tasarımı §7.1, §8.3, §10.3–§10.5):
//! yalıtıcının kurulması, durum değişiminin ana pencereye iletilmesi ve
//! kullanıcının eklediği araç zinciri dizinleri.
//!
//! Durum, ana pencerenin `yalitimi_dinle` ile verdiği kanala yazılır
//! (`oturumu_dinle` gibi); pencereye Tauri'nin olay izinleri açılmaz.
//!
//! Dizinler uygulamanın veri dizinindeki `yalitim.json`'dadır; yalıtımın içinde
//! görünmez. Depo içindeki hiçbir dosya profili genişletemez.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use kalfa_sandbox::{PlatformAyari, Yalitici, YalitimDurumu};
use serde::{Deserialize, Serialize};

use crate::arayuz::YalitimYazici;
use crate::hata::KomutHatasi;
use crate::uygulama::Uygulama;

const DOSYA_ADI: &str = "yalitim.json";

/// Durum değişince çağrılan işlev.
pub type Degisince = Box<dyn Fn(&YalitimDurumu) + Send + Sync>;

/// Ana pencerenin kanalı; tek ana pencere vardır, yeni kanal eskisinin yerini
/// alır.
pub(crate) type Dinleyici = Arc<Mutex<Option<Box<dyn YalitimYazici>>>>;

/// Yalıtıcının nereden geldiği.
pub enum YalitimKaynagi {
    /// Platformun yalıtıcısı: Linux'ta bwrap, diğerlerinde şimdilik yalıtımsız.
    Platform,
    /// Testler: yalıtıcıyı durum değişince çağrılacak işlevle kurar.
    Kurucu(Box<dyn FnOnce(Degisince) -> Arc<dyn Yalitici> + Send>),
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Dosya {
    #[serde(default)]
    arac_zinciri_dizinleri: Vec<PathBuf>,
}

/// Kullanıcının Ayarlar'dan eklediği araç zinciri dizinleri.
pub struct AracZinciriKaydi {
    yol: PathBuf,
    dizinler: Mutex<Vec<PathBuf>>,
}

impl AracZinciriKaydi {
    /// Kaydı okur; okunamayan ya da bozuk kayıt boş sayılır.
    pub fn oku(veri_dizini: &Path) -> Self {
        let yol = veri_dizini.join(DOSYA_ADI);
        let dosya: Dosya = std::fs::read(&yol)
            .ok()
            .and_then(|v| serde_json::from_slice(&v).ok())
            .unwrap_or_default();
        Self {
            yol,
            dizinler: Mutex::new(dosya.arac_zinciri_dizinleri),
        }
    }

    pub fn dizinler(&self) -> Vec<PathBuf> {
        self.dizinler
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .clone()
    }

    /// Listeyi değiştirir ve yazar; yeni listeyi döner.
    fn degistir(&self, is: impl FnOnce(&mut Vec<PathBuf>)) -> Result<Vec<PathBuf>, KomutHatasi> {
        let mut dizinler = self.dizinler.lock().unwrap_or_else(|z| z.into_inner());
        let mut yeni = dizinler.clone();
        is(&mut yeni);
        let metin = serde_json::to_vec_pretty(&Dosya {
            arac_zinciri_dizinleri: yeni.clone(),
        })
        .map_err(|e| KomutHatasi::ic(e.to_string()))?;
        if let Some(ust) = self.yol.parent() {
            std::fs::create_dir_all(ust)?;
        }
        let gecici = self.yol.with_extension("json.yeni");
        std::fs::write(&gecici, metin)?;
        std::fs::rename(&gecici, &self.yol)?;
        *dizinler = yeni.clone();
        Ok(yeni)
    }
}

/// Yalıtıcıyı kurar; durum değişimi ana pencerenin kanalına yazılır.
pub(crate) fn yalitici_kur(
    kaynak: YalitimKaynagi,
    veri_dizini: &Path,
    ev: &Path,
    kayit: &AracZinciriKaydi,
    dinleyici: Dinleyici,
) -> Arc<dyn Yalitici> {
    let degisince: Degisince = Box::new(move |d| {
        if let Some(yazici) = dinleyici.lock().unwrap_or_else(|z| z.into_inner()).as_ref() {
            // Kapanmış pencerenin kanalı yazılamaz; yeni pencere yeniden verir.
            let _ = yazici.gonder(d);
        }
    });
    match kaynak {
        YalitimKaynagi::Platform => kalfa_sandbox::platform_yaliticisi(PlatformAyari {
            ev: ev.to_path_buf(),
            yalitim_dizini: veri_dizini.join("yalitim"),
            ek_okunur: kayit.dizinler(),
            degisince,
        }),
        YalitimKaynagi::Kurucu(kur) => kur(degisince),
    }
}

fn metinler(dizinler: &[PathBuf]) -> Vec<String> {
    dizinler.iter().map(|d| d.display().to_string()).collect()
}

impl Uygulama {
    /// Ana pencerenin yalıtım kanalı. Şimdiki durum hemen yazılır; sonraki
    /// değişimler de bu kanala gider.
    pub fn yalitimi_dinle(&self, yazici: Box<dyn YalitimYazici>) {
        let mut dinleyici = self
            .yalitim_dinleyici
            .lock()
            .unwrap_or_else(|z| z.into_inner());
        // Kilit tutulurken yazılır: araya giren bir değişim eski durumu ezmez.
        let _ = yazici.gonder(&self.yalitici.durum());
        *dinleyici = Some(yazici);
    }

    /// Algılamayı yeniden çalıştırır (§8.3); durum değiştiyse kanala da yazılır.
    pub async fn yalitimi_algila(&self) -> Result<YalitimDurumu, KomutHatasi> {
        let yalitici = self.yalitici.clone();
        tokio::task::spawn_blocking(move || yalitici.yeniden_algila())
            .await
            .map_err(|e| KomutHatasi::ic(e.to_string()))
    }

    pub fn arac_zinciri_dizinleri(&self) -> Vec<String> {
        metinler(&self.arac_zinciri.dizinler())
    }

    /// Yerel klasör seçiciyle bir dizin ekler; arayüz yol gönderemez (§10.5).
    /// Vazgeçilirse liste değişmez.
    pub async fn arac_zinciri_dizini_ekle(&self) -> Result<Vec<String>, KomutHatasi> {
        let Some(secilen) = self
            .iletisim
            .klasor_sec("Yalıtıma açılacak araç zinciri dizinini seçin")
            .await
        else {
            return Ok(self.arac_zinciri_dizinleri());
        };
        let dizin = self.eklenebilir_mi(&secilen)?;
        let yeni = self.arac_zinciri.degistir(|l| {
            if !l.contains(&dizin) {
                l.push(dizin);
            }
        })?;
        self.yalitici.okunur_dizinleri_ayarla(yeni.clone());
        Ok(metinler(&yeni))
    }

    /// Listeden çıkarır. Yalnızca çıkarma yapar; listede olmayan yol yok
    /// sayılır.
    pub fn arac_zinciri_dizini_kaldir(&self, yol: &str) -> Result<Vec<String>, KomutHatasi> {
        let yeni = self
            .arac_zinciri
            .degistir(|l| l.retain(|d| d.as_os_str() != yol))?;
        self.yalitici.okunur_dizinleri_ayarla(yeni.clone());
        Ok(metinler(&yeni))
    }

    /// Ev dizinini ya da Kalfa'nın veri dizinini kapsayan, onların içinde
    /// kalan dizin eklenemez: sırlar ve oturum kayıtları yalıtıma açılırdı.
    fn eklenebilir_mi(&self, secilen: &Path) -> Result<PathBuf, KomutHatasi> {
        let dizin = secilen
            .canonicalize()
            .ok()
            .filter(|d| d.is_dir())
            .ok_or_else(|| KomutHatasi::gecersiz("Seçilen dizin bulunamadı."))?;
        let kanonik = |y: &Path| y.canonicalize().unwrap_or_else(|_| y.to_path_buf());
        let ev = kanonik(&self.ev_dizini);
        let veri = kanonik(&self.veri_dizini);
        if ev.starts_with(&dizin) || veri.starts_with(&dizin) {
            return Err(KomutHatasi::gecersiz(
                "Bu dizin ev dizinini ya da Kalfa'nın veri dizinini kapsıyor; yalıtıma açılamaz.",
            ));
        }
        if dizin.starts_with(&veri) {
            return Err(KomutHatasi::gecersiz(
                "Kalfa'nın veri dizinindeki bir dizin yalıtıma açılamaz.",
            ));
        }
        Ok(dizin)
    }
}
