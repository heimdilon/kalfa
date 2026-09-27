//! Yalıtımın uygulama tarafı (aşama 2 tasarımı §7.1, §8.3, §10.3–§10.5):
//! yalıtıcının kurulması, durum değişiminin ana pencereye iletilmesi ve
//! kullanıcının eklediği araç zinciri dizinleri.
//!
//! Durum, ana pencerenin `yalitimi_dinle` ile verdiği kanala yazılır
//! (`oturumu_dinle` gibi); pencereye Tauri'nin olay izinleri açılmaz.
//!
//! Dizinler ve yalıtımsız çalıştırma anahtarı (§9.8) uygulamanın veri
//! dizinindeki `yalitim.json`'dadır; yalıtımın içinde görünmez. Depo içindeki
//! hiçbir dosya profili genişletemez.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use kalfa_sandbox::{PlatformAyari, Yalitici, YalitimDurumu};
use kalfa_tools::YalitimsizIzni;
use serde::{Deserialize, Serialize};

use crate::arayuz::YalitimYazici;
use crate::hata::KomutHatasi;
use crate::sozlesme::{OnarimSonucu, YalitimCalismaAlani};
use crate::uygulama::{IcDurum, Uygulama};

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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Dosya {
    #[serde(default)]
    arac_zinciri_dizinleri: Vec<PathBuf>,
    /// Yalıtım yokken komutların yalıtımsız çalışmasına izin (§9.8).
    #[serde(default)]
    yalitimsiz_izinli: bool,
}

/// Kullanıcının Ayarlar'dan eklediği araç zinciri dizinleri ve yalıtımsız
/// çalıştırma anahtarı.
pub struct YalitimAyarlari {
    yol: PathBuf,
    dosya: Mutex<Dosya>,
}

impl YalitimAyarlari {
    /// Kaydı okur; okunamayan ya da bozuk kayıt varsayılan sayılır (anahtar
    /// kapalı).
    pub fn oku(veri_dizini: &Path) -> Self {
        let yol = veri_dizini.join(DOSYA_ADI);
        let dosya: Dosya = std::fs::read(&yol)
            .ok()
            .and_then(|v| serde_json::from_slice(&v).ok())
            .unwrap_or_default();
        Self {
            yol,
            dosya: Mutex::new(dosya),
        }
    }

    fn kilit(&self) -> std::sync::MutexGuard<'_, Dosya> {
        self.dosya.lock().unwrap_or_else(|z| z.into_inner())
    }

    pub fn dizinler(&self) -> Vec<PathBuf> {
        self.kilit().arac_zinciri_dizinleri.clone()
    }

    /// Kaydı değiştirir ve yazar; yazılamazsa bellekteki kayıt değişmez.
    fn degistir(&self, is: impl FnOnce(&mut Dosya)) -> Result<Dosya, KomutHatasi> {
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
        *dosya = yeni.clone();
        Ok(yeni)
    }

    fn dizinleri_degistir(
        &self,
        is: impl FnOnce(&mut Vec<PathBuf>),
    ) -> Result<Vec<PathBuf>, KomutHatasi> {
        self.degistir(|d| is(&mut d.arac_zinciri_dizinleri))
            .map(|d| d.arac_zinciri_dizinleri)
    }
}

impl YalitimsizIzni for YalitimAyarlari {
    fn izinli_mi(&self) -> bool {
        self.kilit().yalitimsiz_izinli
    }
}

const ONARIM_ONAYI: &str = "Kalfa, komutları güvenle çalıştırmak için gereken kurulumu \
yeniden yapacak. Windows yönetici izni isteyecek. Kurulum, sürücülerin köklerine Kalfa'nın \
yalıtımı için bir listeleme izni ekler; Kalfa'yı kaldırınca geri alınır.";

const ONARIM_TALIMATI: &str = "Bu Kalfa kurulu bir sürüm değil (Program Files'ta değil); \
Onar yalnızca kurulu sürümde çalışır. Yalıtımı kurmak için `kalfa-yalitim.exe kur` \
komutunu yönetici olarak çalıştırın.";

const YALITIMSIZ_ONAYI: &str = "Yalıtım bu bilgisayarda kullanılamadığında Kalfa'nın komutları \
yalıtımsız çalıştırmasına izin verilecek. Yalıtımsız bir komut sizin bütün yetkilerinizle \
çalışır: ev dizininizdeki dosyaları okuyabilir, değiştirebilir ve ağa erişebilir. Her komut \
yine ayrıca onayınızı ister.\n\nBunu yalnızca ne yaptığınızı biliyorsanız açın.";

const ALAN_KALDIRMA_ONAYI: &str = "Bu çalışma alanı için eklenen yalıtım izinleri geri alınacak \
ve çalışma alanına özel yalıtım profili kaldırılacak. Devam etmek istiyor musunuz?";

struct YalitimKaldirmaRezervi<'a> {
    uygulama: &'a Uygulama,
    kimlik: String,
}

impl Drop for YalitimKaldirmaRezervi<'_> {
    fn drop(&mut self) {
        self.uygulama
            .ic()
            .yalitim_kaldirma_rezervleri
            .remove(&self.kimlik);
    }
}

/// Yalıtıcıyı kurar; durum değişimi ana pencerenin kanalına yazılır.
pub(crate) fn yalitici_kur(
    kaynak: YalitimKaynagi,
    yalitim_dizini: &Path,
    ev: &Path,
    kayit: &YalitimAyarlari,
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
            yalitim_dizini: yalitim_dizini.to_path_buf(),
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
        metinler(&self.yalitim_ayarlari.dizinler())
    }

    pub fn yalitimsiz_izinli(&self) -> bool {
        self.yalitim_ayarlari.izinli_mi()
    }

    /// Yalıtım için hazırlanmış çalışma alanları. Arayüze yol değil, yalıtıcı
    /// kaydının verdiği kimlik ve gösterim yolu çıkar (§9.8).
    pub fn yalitim_calisma_alanlari(&self) -> Vec<YalitimCalismaAlani> {
        self.yalitici
            .hazirlanmis_alanlar()
            .into_iter()
            .map(|alan| YalitimCalismaAlani {
                kimlik: alan.kimlik,
                yol: alan.kok.display().to_string(),
            })
            .collect()
    }

    /// Hazırlanmış çalışma alanının izinlerini yerel onayla geri alır. Açık
    /// oturumu olan çalışma alanı, onay penceresi açılmadan reddedilir.
    pub async fn yalitim_calisma_alanini_kaldir(&self, kimlik: &str) -> Result<bool, KomutHatasi> {
        let alan = self
            .yalitici
            .hazirlanmis_alanlar()
            .into_iter()
            .find(|alan| alan.kimlik == kimlik)
            .ok_or_else(|| KomutHatasi::gecersiz("Hazırlanmış çalışma alanı bulunamadı."))?;

        if self.yalitim_calisma_alani_acik_mi(&alan.kimlik) {
            return Err(KomutHatasi::gecersiz(
                "Bu çalışma alanında açık oturum var. Önce oturumu kapatın.",
            ));
        }
        let mesaj = format!("{ALAN_KALDIRMA_ONAYI}\n\n{}", alan.kok.display());
        if !self
            .iletisim
            .onayla(
                "Çalışma alanını kaldır",
                &mesaj,
                "İzinleri geri al ve kaldır",
                "Vazgeç",
            )
            .await
        {
            return Ok(false);
        }
        // Onay penceresi sürerken oturum açılmış olabilir. Oturum tablosu ile
        // rezervasyon aynı kilitle korunur; tabloya ekleme bundan sonra bu
        // çalışma alanını reddeder.
        {
            let mut ic = self.ic();
            if Self::yalitim_calisma_alani_acik(&ic, &alan.kimlik) {
                return Err(KomutHatasi::gecersiz(
                    "Bu çalışma alanında açık oturum var. Önce oturumu kapatın.",
                ));
            }
            if !ic.yalitim_kaldirma_rezervleri.insert(alan.kimlik.clone()) {
                return Err(KomutHatasi::gecersiz(
                    "Bu çalışma alanı için başka bir kaldırma işlemi sürüyor.",
                ));
            }
        }
        let _rezerv = YalitimKaldirmaRezervi {
            uygulama: self,
            kimlik: alan.kimlik.clone(),
        };
        self.yalitici
            .alani_kaldir_kimlikle(&alan.kimlik)
            .map_err(|e| KomutHatasi::ic(format!("Çalışma alanı kaldırılamadı: {e}")))?;
        Ok(true)
    }

    fn yalitim_calisma_alani_acik_mi(&self, kimlik: &str) -> bool {
        Self::yalitim_calisma_alani_acik(&self.ic(), kimlik)
    }

    fn yalitim_calisma_alani_acik(ic: &IcDurum, kimlik: &str) -> bool {
        ic.oturumlar
            .values()
            .any(|oturum| oturum.calisma_alani_kimligi == kimlik)
            || ic
                .kapaniyor
                .values()
                .any(|oturum| oturum.calisma_alani_kimligi == kimlik)
    }

    /// "Onar" (§9.8): yerel pencerede ne yapılacağını söyler, onaylanırsa
    /// kurulu yardımcıyla `kalfa-yalitim kur`'u UAC ile çalıştırır ve
    /// algılamayı yeniden yapar. `kur` yinelenince zararsızdır.
    pub async fn yalitimi_onar(&self) -> Result<OnarimSonucu, KomutHatasi> {
        if !self.yukseltici.kurulu_mu() {
            return Ok(OnarimSonucu::KuruluDegil {
                talimat: ONARIM_TALIMATI.to_string(),
            });
        }
        if !self
            .iletisim
            .onayla("Yalıtımı onar", ONARIM_ONAYI, "Onar", "Vazgeç")
            .await
        {
            return Ok(OnarimSonucu::Vazgecildi);
        }
        match self.yukseltici.kur().await {
            Ok(None) => Ok(OnarimSonucu::Vazgecildi),
            Ok(Some(0)) => Ok(OnarimSonucu::Tamam {
                yalitim: self.yalitimi_algila().await?,
            }),
            Ok(Some(kod)) => {
                // Yarım kalan kurulum durumu değiştirmiş olabilir.
                self.yalitimi_algila().await?;
                Ok(OnarimSonucu::Basarisiz {
                    mesaj: format!("kalfa-yalitim kur {kod} koduyla bitti."),
                })
            }
            Err(mesaj) => Ok(OnarimSonucu::Basarisiz { mesaj }),
        }
    }

    /// BT için teknik rapor (§9.8), düz metin; arayüz panoya kopyalar.
    pub fn yalitim_teknik_raporu(&self) -> String {
        let (durum, ayrinti, talimat) = match self.yalitici.durum() {
            YalitimDurumu::Hazir { arka_uc, surum } => {
                (format!("Hazır ({arka_uc} {surum})"), "-".into(), "-".into())
            }
            YalitimDurumu::Yok {
                neden,
                talimat,
                ayrinti,
            } => (format!("Yok ({neden:?})"), ayrinti, talimat),
        };
        let yalitimsiz = if self.yalitimsiz_izinli() {
            "açık"
        } else {
            "kapalı"
        };
        format!(
            "Kalfa yalıtım raporu\n\
             Kalfa sürümü: {}\n\
             İşletim sistemi: {}\n\
             Durum: {durum}\n\
             Ayrıntı: {ayrinti}\n\
             Talimat: {talimat}\n\
             Yalıtımsız çalıştırma: {yalitimsiz}\n\
             \n\
             Gereksinim:\n{}\n",
            env!("CARGO_PKG_VERSION"),
            kalfa_sandbox::sistem_surumu(),
            kalfa_sandbox::gereksinim(),
        )
    }

    /// Yalıtımsız çalıştırma anahtarı (§9.8). Açarken yerel pencere riski
    /// anlatır; vazgeçilirse kapalı kalır. Kapatmak onay istemez. Yeni değeri
    /// döner.
    pub async fn yalitimsizi_ayarla(&self, acik: bool) -> Result<bool, KomutHatasi> {
        if acik
            && !self.yalitimsiz_izinli()
            && !self
                .iletisim
                .onayla(
                    "Yalıtımsız çalıştırmayı aç",
                    YALITIMSIZ_ONAYI,
                    "Aç",
                    "Vazgeç",
                )
                .await
        {
            return Ok(false);
        }
        Ok(self
            .yalitim_ayarlari
            .degistir(|d| d.yalitimsiz_izinli = acik)?
            .yalitimsiz_izinli)
    }

    /// Yerel klasör seçiciyle bir dizin ekler; arayüz yol gönderemez (§10.5).
    /// Vazgeçilirse liste değişmez.
    pub async fn arac_zinciri_dizini_ekle(&self) -> Result<Vec<String>, KomutHatasi> {
        let Some(secilen) = self
            .iletisim
            .klasor_sec("Yalıtıma açılacak araç zinciri dizinini seçin", None)
            .await
        else {
            return Ok(self.arac_zinciri_dizinleri());
        };
        let dizin = self.eklenebilir_mi(&secilen)?;
        let yeni = self.yalitim_ayarlari.dizinleri_degistir(|l| {
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
            .yalitim_ayarlari
            .dizinleri_degistir(|l| l.retain(|d| d.as_os_str() != yol))?;
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
