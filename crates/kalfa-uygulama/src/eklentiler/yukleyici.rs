//! Eklentilerin diskten okunması, yüklenmesi ve oturuma bağlanması (aşama 4
//! tasarımı §7.2, §13). Bir `Motor` uygulama başına bir kez, ilk ihtiyaçta
//! kurulur; yüklenen eklentiler `(ad, sha256)` anahtarıyla önbelleğe alınır
//! (derleme ~100 ms).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use kalfa_core::arac::Arac;
use kalfa_core::kilit::KaynakKilitleri;
use kalfa_core::muhur::ozet_hex;
use kalfa_plugin::depo::dizin_boyutu;
use kalfa_plugin::manifest::{Manifest, eklenti_adi_gecerli};
use kalfa_plugin::{Eklenti, EklentiAraci, EvrenBaglami, Motor};
use kalfa_tools::CalismaAlani;

use super::kayit::{AlanIzni, AlanKaydi, Ayar, EklentiKaydi, Kayit, KuruluKaydi, kimlik_gecerli};
use crate::dizin;
use crate::hata::KomutHatasi;
use crate::sozlesme::{EklentiAlani, EklentiAraci as AracOzeti, EklentiDurumu, EklentiSatiri};

pub(crate) const MANIFEST_DOSYASI: &str = "kalfa.toml";
pub(crate) const WASM_DOSYASI: &str = "eklenti.wasm";

// ponytail: sabit sınırlar (§13.2). Gerçek eklentiler çok altında kalır
// (`kalfa-rag` birkaç MB); daha büyük bir bileşen gerekirse sınır ayara bağlanır.
const MANIFEST_AZAMI: u64 = 64 * 1024;
const WASM_AZAMI: u64 = 64 * 1024 * 1024;

const DEGISMIS: &str = "Eklentinin dosyası kurulumdan sonra değişmiş; eklentiyi yeniden kurun.";
const YERLESIK_ADI: &str =
    "Bu ad Kalfa'yla gelen bir eklentiye ait; bu kayıt kullanılmıyor. Kaldırabilirsiniz.";

/// Bir eklenti dizininin (ya da seçilen klasörün) doğrulanmış baytları.
/// Kurulan, doğrulanan baytların aynısıdır.
pub(crate) struct Dosyalar {
    pub manifest: String,
    pub wasm: Vec<u8>,
    /// `wasm`'ın SHA-256'sı (onaltılık).
    pub sha256: String,
}

/// En çok `azami` bayt okur; fazlası varsa `Ok(None)`. Okuma sınırlıdır, bu
/// yüzden okurken büyüyen dosya da belleği şişiremez.
fn sinirli_oku(yol: &Path, azami: u64) -> io::Result<Option<Vec<u8>>> {
    let mut veri = Vec::new();
    File::open(yol)?.take(azami + 1).read_to_end(&mut veri)?;
    Ok((u64::try_from(veri.len()).is_ok_and(|n| n <= azami)).then_some(veri))
}

/// Seçilen klasörde sıradan bir dosya okur; bağlantı izlenmez.
fn klasor_dosyasi(yol: &Path, ad: &str, azami: u64, azami_metin: &str) -> Result<Vec<u8>, String> {
    let meta = match std::fs::symlink_metadata(yol) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(format!(
                "Seçtiğiniz klasörde `{ad}` yok. Eklenti klasörü bir `{MANIFEST_DOSYASI}` ve tam bir `.wasm` dosyası içermelidir."
            ));
        }
        Err(e) => return Err(format!("`{ad}` okunamadı ({e}).")),
    };
    if !meta.is_file() {
        return Err(format!(
            "`{ad}` sıradan bir dosya olmalı; bağlantılar ve klasörler izlenmez."
        ));
    }
    match sinirli_oku(yol, azami) {
        Ok(Some(veri)) => Ok(veri),
        Ok(None) => Err(format!("`{ad}` en çok {azami_metin} olabilir.")),
        Err(e) => Err(format!("`{ad}` okunamadı ({e}).")),
    }
}

/// "Klasörden kur" için seçilen klasörü okur (§13.2): alt dizinlere inmeden
/// `kalfa.toml` ve tam bir `.wasm`. `Err` kullanıcıya gösterilecek iletidir.
pub(crate) fn klasoru_oku(klasor: &Path) -> Result<Dosyalar, String> {
    let okunamadi =
        |e: io::Error| format!("Seçtiğiniz klasör okunamadı ({e}). Başka bir klasör seçin.");
    let mut wasmlar = Vec::new();
    for girdi in std::fs::read_dir(klasor).map_err(okunamadi)? {
        let ad = girdi.map_err(okunamadi)?.file_name();
        if Path::new(&ad)
            .extension()
            .is_some_and(|u| u.eq_ignore_ascii_case("wasm"))
        {
            wasmlar.push(ad.to_string_lossy().into_owned());
        }
    }
    wasmlar.sort();
    let manifest = klasor_dosyasi(
        &klasor.join(MANIFEST_DOSYASI),
        MANIFEST_DOSYASI,
        MANIFEST_AZAMI,
        "64 KiB",
    )?;
    let manifest = String::from_utf8(manifest)
        .map_err(|_| format!("`{MANIFEST_DOSYASI}` UTF-8 bir metin olmalı."))?;
    let wasm = match wasmlar.as_slice() {
        [] => {
            return Err(format!(
                "Seçtiğiniz klasörde `.wasm` dosyası yok. Eklenti klasörü bir `{MANIFEST_DOSYASI}` ve tam bir `.wasm` dosyası içermelidir."
            ));
        }
        [ad] => klasor_dosyasi(&klasor.join(ad), ad, WASM_AZAMI, "64 MiB")?,
        birden_cok => {
            return Err(format!(
                "Seçtiğiniz klasörde {} `.wasm` dosyası var ({}). Eklenti klasöründe tam bir `.wasm` dosyası olmalı.",
                birden_cok.len(),
                birden_cok.join(", ")
            ));
        }
    };
    let sha256 = ozet_hex(&wasm);
    Ok(Dosyalar {
        manifest,
        wasm,
        sha256,
    })
}

/// Kurulu ya da yerleşik eklentinin dizini: `kalfa.toml` ve `eklenti.wasm`.
/// `oneri` okunamazsa kullanıcıya söylenecek çaredir.
fn dizin_dosyalari(dizin: &Path, oneri: &str) -> Result<Dosyalar, String> {
    let oku = |ad: &str, azami: u64| -> Result<Vec<u8>, String> {
        match sinirli_oku(&dizin.join(ad), azami) {
            Ok(Some(veri)) => Ok(veri),
            Ok(None) => Err(format!(
                "Eklentinin `{ad}` dosyası beklenenden büyük; {oneri}"
            )),
            Err(e) => Err(format!(
                "Eklentinin `{ad}` dosyası okunamadı ({e}); {oneri}"
            )),
        }
    };
    let manifest = String::from_utf8(oku(MANIFEST_DOSYASI, MANIFEST_AZAMI)?)
        .map_err(|_| format!("Eklentinin `{MANIFEST_DOSYASI}` dosyası UTF-8 değil; {oneri}"))?;
    let wasm = oku(WASM_DOSYASI, WASM_AZAMI)?;
    let sha256 = ozet_hex(&wasm);
    Ok(Dosyalar {
        manifest,
        wasm,
        sha256,
    })
}

const KURULU_ONERISI: &str = "eklentiyi yeniden kurun.";
const YERLESIK_ONERISI: &str = "Kalfa'yı yeniden kurun.";

/// Yerleşik eklentilerin adları: dizindeki geçerli adlı alt dizinler.
/// Bağlantılar izlenmez.
pub(crate) fn yerlesik_adlari(dizin: &Path) -> Vec<String> {
    let Ok(girdiler) = std::fs::read_dir(dizin) else {
        return Vec::new();
    };
    let mut adlar: Vec<String> = girdiler
        .filter_map(Result::ok)
        .filter(|g| g.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|g| g.file_name().into_string().ok())
        .filter(|a| eklenti_adi_gecerli(a))
        .collect();
    adlar.sort();
    adlar
}

/// Bir eklentinin kullanıcıya gösterilen hâli: yüklenmiş ya da yüklenememiş.
pub(crate) struct Bilinen {
    pub ad: String,
    pub yerlesik: bool,
    pub surum: String,
    pub aciklama: String,
    pub sha256: String,
    pub kaynak: Option<String>,
    pub kurulum_ms: Option<u64>,
    pub etkin: bool,
    pub alanlar: BTreeMap<String, AlanKaydi>,
    pub yetenek: Option<String>,
    pub yuklu: Result<Arc<Eklenti>, String>,
}

impl Bilinen {
    fn bozuk(ad: String, neden: String) -> Self {
        Self {
            ad,
            yerlesik: false,
            surum: String::new(),
            aciklama: String::new(),
            sha256: String::new(),
            kaynak: None,
            kurulum_ms: None,
            etkin: false,
            alanlar: BTreeMap::new(),
            yetenek: None,
            yuklu: Err(neden),
        }
    }

    /// Yüklenemeyen eklenti etkin olsa da yüklenemedi görünür: kullanıcının
    /// çaresi (yeniden kurmak, kaldırmak) oradadır.
    pub fn durum(&self) -> EklentiDurumu {
        match (&self.yuklu, self.etkin) {
            (Err(neden), _) => EklentiDurumu::Yuklenemedi {
                neden: neden.clone(),
            },
            (Ok(_), false) => EklentiDurumu::DevreDisi,
            (Ok(_), true) => EklentiDurumu::Yuklendi,
        }
    }

    /// Eklentiler ekranının satırı; depo boyutları diskten sayılır.
    pub fn satir(self, eklenti_verisi: &Path) -> EklentiSatiri {
        let durum = self.durum();
        let araclar = match &self.yuklu {
            Ok(e) => e
                .araclar
                .iter()
                .map(|a| AracOzeti {
                    ad: e.manifest.model_adi(&a.ad),
                    aciklama: a.aciklama.clone(),
                })
                .collect(),
            Err(_) => Vec::new(),
        };
        let alanlar = self
            .alanlar
            .iter()
            .map(|(kimlik, a)| EklentiAlani {
                kimlik: kimlik.clone(),
                yol: a.yol.clone(),
                acik: a.acik,
                gonderim_izni_ms: a.gonderim_izni_ms,
                depo_bayt: dizin_boyutu(&eklenti_verisi.join(&self.ad).join(kimlik)).unwrap_or(0),
            })
            .collect();
        EklentiSatiri {
            ad: self.ad,
            surum: self.surum,
            aciklama: self.aciklama,
            yerlesik: self.yerlesik,
            durum,
            sha256: self.sha256,
            kaynak: self.kaynak,
            kurulum_ms: self.kurulum_ms,
            yetenek: self.yetenek,
            araclar,
            alanlar,
        }
    }
}

/// Bir ayarın hangi kayda ait olduğu: yerleşik ya da kurulu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Hedef {
    Yerlesik,
    Kurulu,
}

/// Yerleşik eklentinin görülen manifesti kayıttakinden farklıysa kayıt
/// güncellenir; yeni manifest gönderim kapsamını değiştirdiyse (ya da eskisi
/// çözülemiyorsa) bütün gönderim izinleri silinir (§13.1).
fn yerlesik_izle(kayit: &mut Kayit, gorulenler: &[(String, String, Manifest)]) {
    for (ad, metin, yeni) in gorulenler {
        let girdi = kayit.yerlesik.entry(ad.clone()).or_default();
        if girdi.manifest == *metin {
            continue;
        }
        let kapsam_degisti = match Manifest::coz(&girdi.manifest) {
            Ok(eski) => yeni.gonderim_kapsami_degisti(&eski),
            Err(_) => true,
        };
        if kapsam_degisti {
            for alan in girdi.ayar.calisma_alanlari.values_mut() {
                alan.gonderim_izni_ms = None;
            }
        }
        girdi.manifest = metin.clone();
    }
}

struct Onbellekli {
    /// Yüklendiği manifest metni: aynı `.wasm` başka bir manifestle
    /// yüklenirse önbellek kullanılmaz.
    manifest: String,
    eklenti: Arc<Eklenti>,
}

pub(crate) struct EklentiYoneticisi {
    pub kayit: Arc<EklentiKaydi>,
    pub evren: Arc<EvrenBaglami>,
    /// Klasörden kurulanların kökü (`<veri>/eklentiler`).
    pub kurulu_dizin: PathBuf,
    /// Eklenti depolarının kökü; bir depo `<kök>/<ad>/<çalışma alanı>/`'dır.
    pub veri_dizini: PathBuf,
    pub yerlesik_dizin: Option<PathBuf>,
    /// Kurulum, kaldırma ve veri silme sırayla çalışır: yerel pencere açıkken
    /// ikinci bir işlem aynı dizinlere dokunmasın.
    pub islem_kilidi: tokio::sync::Mutex<()>,
    motor: OnceLock<Result<Arc<Motor>, String>>,
    onbellek: Mutex<HashMap<(String, String), Onbellekli>>,
}

impl EklentiYoneticisi {
    pub fn yeni(
        veri_dizini: &Path,
        eklenti_verisi: PathBuf,
        yerlesik_dizin: Option<PathBuf>,
        evren: Arc<EvrenBaglami>,
    ) -> Self {
        Self {
            kayit: Arc::new(EklentiKaydi::oku(veri_dizini)),
            evren,
            kurulu_dizin: dizin::kurulu_eklenti_dizini(veri_dizini),
            veri_dizini: eklenti_verisi,
            yerlesik_dizin,
            islem_kilidi: tokio::sync::Mutex::new(()),
            motor: OnceLock::new(),
            onbellek: Mutex::new(HashMap::new()),
        }
    }

    /// Uygulamanın tek motoru; ilk ihtiyaçta kurulur. Kurulamazsa hata
    /// kalıcıdır ve her eklenti bu iletiyle yüklenemedi olur.
    pub fn motor(&self) -> Result<Arc<Motor>, String> {
        self.motor
            .get_or_init(|| Motor::yeni().map_err(|e| e.to_string()))
            .clone()
    }

    pub fn yerlesik_adlari(&self) -> Vec<String> {
        self.yerlesik_dizin
            .as_deref()
            .map(yerlesik_adlari)
            .unwrap_or_default()
    }

    /// Ayarı değiştirilecek eklentinin kaydı (§13.3). Yerleşik ad önce
    /// aranır. Kaydı bozuk ya da bilinmeyen eklenti `gecersiz` alır.
    pub fn hedef_coz(&self, ad: &str) -> Result<Hedef, KomutHatasi> {
        if self.yerlesik_adlari().iter().any(|a| a == ad) {
            return Ok(Hedef::Yerlesik);
        }
        let (kurulu, bozuk) = self.kayit.bak(|k| {
            (
                k.kurulu(ad).is_some(),
                k.bozuk.iter().any(|b| b.ad.as_deref() == Some(ad)),
            )
        });
        if kurulu {
            Ok(Hedef::Kurulu)
        } else if bozuk {
            Err(KomutHatasi::gecersiz(format!(
                "`{ad}` eklentisinin kaydı bozuk; eklentiyi kaldırıp yeniden kurun."
            )))
        } else {
            Err(KomutHatasi::gecersiz(format!(
                "`{ad}` adlı bir eklenti yok."
            )))
        }
    }

    fn onbellek(&self) -> std::sync::MutexGuard<'_, HashMap<(String, String), Onbellekli>> {
        self.onbellek.lock().unwrap_or_else(|z| z.into_inner())
    }

    fn onbellekten(&self, ad: &str, sha256: &str, manifest: &str) -> Option<Arc<Eklenti>> {
        self.onbellek()
            .get(&(ad.to_string(), sha256.to_string()))
            .filter(|o| o.manifest == manifest)
            .map(|o| o.eklenti.clone())
    }

    /// Aynı adın eski sürümleri önbellekten atılır.
    pub fn onbellege_koy(&self, ad: &str, sha256: &str, manifest: &str, eklenti: Arc<Eklenti>) {
        let mut onbellek = self.onbellek();
        onbellek.retain(|(a, _), _| a != ad);
        onbellek.insert(
            (ad.to_string(), sha256.to_string()),
            Onbellekli {
                manifest: manifest.to_string(),
                eklenti,
            },
        );
    }

    pub fn onbellegi_temizle(&self, ad: &str) {
        self.onbellek().retain(|(a, _), _| a != ad);
    }

    /// Eklentiyi yükler (§7.2). Aynı `(ad, sha256, manifest)` önbellekten gelir.
    async fn yukle(
        &self,
        ad: &str,
        manifest: &str,
        wasm: Vec<u8>,
        sha256: &str,
    ) -> Result<Arc<Eklenti>, String> {
        if let Some(eklenti) = self.onbellekten(ad, sha256, manifest) {
            return Ok(eklenti);
        }
        let eklenti = self
            .motor()?
            .yukle(manifest, wasm)
            .await
            .map_err(|e| e.to_string())?;
        if eklenti.manifest.ad != ad {
            return Err(format!(
                "Eklentinin manifestindeki ad (`{}`) kayıtlı adla (`{ad}`) uyuşmuyor; eklentiyi yeniden kurun.",
                eklenti.manifest.ad
            ));
        }
        let eklenti = Arc::new(eklenti);
        self.onbellege_koy(ad, sha256, manifest, eklenti.clone());
        Ok(eklenti)
    }

    /// Bilinen her eklenti (yerleşikler önce, sonra kurulular ada göre),
    /// yüklenebildiği kadarıyla. Yerleşik eklentinin yeni manifesti gönderim
    /// kapsamını değiştirdiyse izinler burada silinir.
    pub async fn bilinenler(&self) -> Vec<Bilinen> {
        let mut kayit = self.kayit.anlik();
        let kurulu_dizin = self.kurulu_dizin.clone();
        let yerlesik_dizin = self.yerlesik_dizin.clone();
        let kurulu_adlari: Vec<String> = kayit.kurulu.iter().map(|k| k.ad.clone()).collect();
        // ponytail: her listelemede ve oturum açılışında `.wasm` okunup özeti
        // alınır (birkaç MB, milisaniyeler); ölçülürse (yol, boyut, değişme
        // zamanı) ile önbelleğe alınır. Kurulumun dizini yeniden adlandırdığı
        // an oturum açılırsa o oturum eklentiyi görmeyebilir; dizin kilidi
        // gerekirse eklenir.
        let okunan = tokio::task::spawn_blocking(move || {
            let yerlesik: Vec<(String, Result<Dosyalar, String>)> = yerlesik_dizin
                .map(|d| {
                    yerlesik_adlari(&d)
                        .into_iter()
                        .map(|ad| {
                            let dosyalar = dizin_dosyalari(&d.join(&ad), YERLESIK_ONERISI);
                            (ad, dosyalar)
                        })
                        .collect()
                })
                .unwrap_or_default();
            let kurulu: Vec<(String, Result<Dosyalar, String>)> = kurulu_adlari
                .into_iter()
                .map(|ad| {
                    let dosyalar = dizin_dosyalari(&kurulu_dizin.join(&ad), KURULU_ONERISI);
                    (ad, dosyalar)
                })
                .collect();
            (yerlesik, kurulu)
        })
        .await;
        let Ok((yerlesikler, kurulular)) = okunan else {
            return Vec::new();
        };

        let gorulenler: Vec<(String, String, Manifest)> = yerlesikler
            .iter()
            .filter_map(|(ad, dosyalar)| {
                let d = dosyalar.as_ref().ok()?;
                let manifest = Manifest::coz(&d.manifest).ok().filter(|m| m.ad == *ad)?;
                Some((ad.clone(), d.manifest.clone(), manifest))
            })
            .collect();
        yerlesik_izle(&mut kayit, &gorulenler);
        // Yazılamasa da bellekte uygulanır; sonraki değişiklikte yeniden yazılır.
        let _ = self.kayit.sikilastir(|k| yerlesik_izle(k, &gorulenler));

        let yerlesik_adlari: HashSet<String> =
            yerlesikler.iter().map(|(ad, _)| ad.clone()).collect();
        let mut sonuc = Vec::new();
        for (ad, dosyalar) in yerlesikler {
            let ayar = kayit
                .yerlesik
                .get(&ad)
                .map(|y| y.ayar.clone())
                .unwrap_or_default();
            sonuc.push(self.olustur(ad, None, dosyalar, ayar).await);
        }

        let mut kuruluya_ait = Vec::new();
        for (ad, dosyalar) in kurulular {
            let Some(kurulu) = kayit.kurulu(&ad) else {
                continue;
            };
            kuruluya_ait.push(if yerlesik_adlari.contains(&ad) {
                Bilinen::bozuk(ad, YERLESIK_ADI.to_string())
            } else {
                self.olustur(ad, Some(kurulu), dosyalar, kurulu.ayar.clone())
                    .await
            });
        }
        let mut gorulen = HashSet::new();
        for bozuk in &kayit.bozuk {
            if let Some(ad) = &bozuk.ad
                && kayit.kurulu(ad).is_none()
                && gorulen.insert(ad.clone())
            {
                kuruluya_ait.push(Bilinen::bozuk(ad.clone(), bozuk.neden.clone()));
            }
        }
        kuruluya_ait.sort_by(|a, b| a.ad.cmp(&b.ad));
        sonuc.extend(kuruluya_ait);
        sonuc
    }

    /// `kurulu` verilirse kayıttaki özet ve sürüm esastır (§7.2).
    async fn olustur(
        &self,
        ad: String,
        kurulu: Option<&KuruluKaydi>,
        dosyalar: Result<Dosyalar, String>,
        ayar: Ayar,
    ) -> Bilinen {
        let kayitli_sha = kurulu.map(|k| k.sha256.clone());
        let (yuklu, manifest, sha256) = match dosyalar {
            Err(neden) => (Err(neden), None, kayitli_sha.unwrap_or_default()),
            Ok(d) => {
                let manifest = Manifest::coz(&d.manifest).ok();
                let yuklu = match &kayitli_sha {
                    Some(sha) if *sha != d.sha256 => Err(DEGISMIS.to_string()),
                    _ => self.yukle(&ad, &d.manifest, d.wasm, &d.sha256).await,
                };
                (yuklu, manifest, kayitli_sha.unwrap_or(d.sha256))
            }
        };
        Bilinen {
            yerlesik: kurulu.is_none(),
            surum: kurulu
                .map(|k| k.surum.clone())
                .or_else(|| manifest.as_ref().map(|m| m.surum.to_string()))
                .unwrap_or_default(),
            aciklama: manifest
                .as_ref()
                .map(|m| m.aciklama.clone())
                .unwrap_or_default(),
            sha256,
            kaynak: kurulu.map(|k| k.kaynak.clone()),
            kurulum_ms: kurulu.map(|k| k.kurulum_ms),
            etkin: ayar.etkin,
            alanlar: ayar.calisma_alanlari,
            yetenek: manifest.as_ref().map(Manifest::yetenek_cumlesi),
            yuklu,
            ad,
        }
    }

    /// Yüklenebilen her eklentinin bu çalışma alanındaki araçları (§13.3).
    /// Etkin ya da açık olmasından bağımsızdır: kapalı eklentinin araçları
    /// `sunulur_mu` ile gizlenir, böylece anahtar açık oturumda canlı çalışır.
    /// Yüklenemeyen eklenti yalnızca araçsız kalır.
    pub async fn araclar(
        &self,
        alan: &Arc<CalismaAlani>,
        kilitler: &Arc<KaynakKilitleri>,
        kimlik: &str,
    ) -> Vec<Arc<dyn Arac>> {
        if !kimlik_gecerli(kimlik) {
            return Vec::new();
        }
        let Ok(motor) = self.motor() else {
            return Vec::new();
        };
        let mut araclar = Vec::new();
        for bilinen in self.bilinenler().await {
            let Ok(eklenti) = &bilinen.yuklu else {
                continue;
            };
            let izin = Arc::new(AlanIzni {
                kayit: self.kayit.clone(),
                ad: bilinen.ad.clone(),
                kimlik: kimlik.to_string(),
                yerlesik: bilinen.yerlesik,
                sha256: bilinen.sha256.clone(),
                kurulum_ms: bilinen.kurulum_ms.unwrap_or(0),
            });
            let depo = self.veri_dizini.join(&bilinen.ad).join(kimlik);
            araclar.extend(EklentiAraci::hepsi(
                &motor,
                eklenti,
                alan,
                kilitler,
                &depo,
                Some(self.evren.clone()),
                izin,
            ));
        }
        araclar
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaz(dizin: &Path, ad: &str, icerik: &[u8]) {
        std::fs::write(dizin.join(ad), icerik).unwrap();
    }

    #[test]
    fn sinirli_oku_siniri_asan_dosyayi_bellege_almaz() {
        let d = tempfile::tempdir().unwrap();
        yaz(d.path(), "tam", &[7; 10]);
        yaz(d.path(), "fazla", &[7; 11]);
        yaz(d.path(), "bos", b"");
        assert_eq!(
            sinirli_oku(&d.path().join("tam"), 10).unwrap(),
            Some(vec![7; 10])
        );
        assert_eq!(sinirli_oku(&d.path().join("fazla"), 10).unwrap(), None);
        assert_eq!(
            sinirli_oku(&d.path().join("bos"), 0).unwrap(),
            Some(Vec::new())
        );
        assert!(sinirli_oku(&d.path().join("yok"), 10).is_err());
    }

    #[test]
    fn klasor_kalfa_toml_ve_tam_bir_wasm_ister() {
        let d = tempfile::tempdir().unwrap();
        let hata = |d: &Path| klasoru_oku(d).err().unwrap();

        assert!(hata(d.path()).contains("`kalfa.toml` yok"), "boş klasör");
        yaz(d.path(), "kalfa.toml", b"[eklenti]");
        assert!(hata(d.path()).contains("`.wasm` dosyası yok"));

        yaz(d.path(), "a.wasm", b"\0asm");
        let ok = klasoru_oku(d.path()).unwrap();
        assert_eq!(ok.manifest, "[eklenti]");
        assert_eq!(ok.wasm, b"\0asm");
        assert_eq!(ok.sha256, ozet_hex(b"\0asm"));

        yaz(d.path(), "b.WASM", b"\0asm");
        let iki = hata(d.path());
        assert!(
            iki.contains("2 `.wasm`") && iki.contains("a.wasm") && iki.contains("b.WASM"),
            "{iki}"
        );
    }

    #[test]
    fn klasor_alt_dizinlere_inmez_ve_sinirlari_uygular() {
        let d = tempfile::tempdir().unwrap();
        yaz(d.path(), "kalfa.toml", b"[eklenti]");
        yaz(d.path(), "a.wasm", b"\0asm");
        let alt = d.path().join("alt");
        std::fs::create_dir(&alt).unwrap();
        yaz(&alt, "b.wasm", b"\0asm");
        assert!(
            klasoru_oku(d.path()).is_ok(),
            "alt dizindeki .wasm sayılmaz"
        );

        yaz(d.path(), "kalfa.toml", &vec![b'x'; 64 * 1024]);
        assert!(klasoru_oku(d.path()).is_ok(), "tam 64 KiB kabul");
        yaz(d.path(), "kalfa.toml", &vec![b'x'; 64 * 1024 + 1]);
        let buyuk = klasoru_oku(d.path()).err().unwrap();
        assert!(buyuk.contains("64 KiB"), "{buyuk}");

        yaz(d.path(), "kalfa.toml", &[0xff, 0xfe]);
        assert!(klasoru_oku(d.path()).err().unwrap().contains("UTF-8"));
    }

    #[test]
    fn klasor_sirada_dosya_olmayan_girdileri_reddeder() {
        let d = tempfile::tempdir().unwrap();
        yaz(d.path(), "kalfa.toml", b"[eklenti]");
        // `.wasm` adlı bir dizin de sayılır ve sıradan dosya olmadığı için
        // reddedilir.
        std::fs::create_dir(d.path().join("a.wasm")).unwrap();
        let hata = klasoru_oku(d.path()).err().unwrap();
        assert!(hata.contains("sıradan bir dosya"), "{hata}");

        let d2 = tempfile::tempdir().unwrap();
        yaz(d2.path(), "a.wasm", b"\0asm");
        std::fs::create_dir(d2.path().join("kalfa.toml")).unwrap();
        assert!(
            klasoru_oku(d2.path())
                .err()
                .unwrap()
                .contains("sıradan bir dosya")
        );

        let yok = d.path().join("yok");
        assert!(klasoru_oku(&yok).err().unwrap().contains("okunamadı"));
    }

    #[cfg(unix)]
    #[test]
    fn klasor_baglantilari_izlemez() {
        let d = tempfile::tempdir().unwrap();
        let hedef = tempfile::tempdir().unwrap();
        yaz(hedef.path(), "gercek.toml", b"[eklenti]");
        yaz(hedef.path(), "gercek.wasm", b"\0asm");
        std::os::unix::fs::symlink(
            hedef.path().join("gercek.toml"),
            d.path().join("kalfa.toml"),
        )
        .unwrap();
        yaz(d.path(), "a.wasm", b"\0asm");
        let hata = klasoru_oku(d.path()).err().unwrap();
        assert!(hata.contains("sıradan bir dosya"), "{hata}");

        let d2 = tempfile::tempdir().unwrap();
        yaz(d2.path(), "kalfa.toml", b"[eklenti]");
        std::os::unix::fs::symlink(hedef.path().join("gercek.wasm"), d2.path().join("a.wasm"))
            .unwrap();
        let hata = klasoru_oku(d2.path()).err().unwrap();
        assert!(hata.contains("sıradan bir dosya"), "{hata}");
    }

    #[test]
    fn dizin_dosyalari_ozeti_hesaplar_ve_eksigi_cozumle_soyler() {
        let d = tempfile::tempdir().unwrap();
        let hata = dizin_dosyalari(d.path(), KURULU_ONERISI).err().unwrap();
        assert!(
            hata.contains("kalfa.toml") && hata.ends_with("eklentiyi yeniden kurun."),
            "{hata}"
        );
        yaz(d.path(), MANIFEST_DOSYASI, b"[eklenti]");
        yaz(d.path(), WASM_DOSYASI, b"abc");
        let ok = dizin_dosyalari(d.path(), KURULU_ONERISI).unwrap();
        assert_eq!(ok.sha256, ozet_hex(b"abc"));
        std::fs::remove_file(d.path().join(WASM_DOSYASI)).unwrap();
        let hata = dizin_dosyalari(d.path(), YERLESIK_ONERISI).err().unwrap();
        assert!(hata.ends_with("Kalfa'yı yeniden kurun."), "{hata}");
    }

    #[test]
    fn yerlesik_adlari_gecerli_alt_dizinlerdir() {
        let d = tempfile::tempdir().unwrap();
        for ad in ["kalfa-rag", "ornek", "Buyuk", "a", "iki--tire", "x.yeni"] {
            std::fs::create_dir(d.path().join(ad)).unwrap();
        }
        yaz(d.path(), "dosya", b"");
        assert_eq!(yerlesik_adlari(d.path()), vec!["kalfa-rag", "ornek"]);
        assert!(yerlesik_adlari(&d.path().join("yok")).is_empty());
    }

    fn manifest(globlar: &str, evren: &str) -> String {
        format!(
            "[eklenti]\nad = \"kalfa-rag\"\nsurum = \"0.1.0\"\nsozlesme = \"0.1\"\naciklama = \"x\"\n\n[yetenekler]\ndosya-oku = [{globlar}]\nevren = [{evren}]\ndepo-mb = 10\n"
        )
    }

    fn izleme(kayit: &mut Kayit, metin: &str) {
        let m = Manifest::coz(metin).unwrap();
        yerlesik_izle(kayit, &[("kalfa-rag".to_string(), metin.to_string(), m)]);
    }

    fn izinli(manifest: &str) -> Kayit {
        let mut k = Kayit::default();
        let y = k.yerlesik.entry("kalfa-rag".into()).or_default();
        y.manifest = manifest.into();
        y.ayar.calisma_alanlari.insert(
            "0123456789abcdef".into(),
            AlanKaydi {
                yol: "C:\\a".into(),
                acik: true,
                gonderim_izni_ms: Some(5),
            },
        );
        k
    }

    fn izin_var(k: &Kayit) -> bool {
        k.yerlesik["kalfa-rag"].ayar.calisma_alanlari["0123456789abcdef"]
            .gonderim_izni_ms
            .is_some()
    }

    #[test]
    fn yerlesik_kapsami_degisince_izinler_silinir_degismezse_kalir() {
        let eski = manifest("\"**/*.md\"", "\"embeddings\"");
        // Aynı kapsam, farklı metin (sürüm ve açıklama): izin kalır, metin güncellenir.
        let ayni = eski.replace("0.1.0", "0.2.0");
        let mut k = izinli(&eski);
        izleme(&mut k, &ayni);
        assert!(izin_var(&k));
        assert_eq!(k.yerlesik["kalfa-rag"].manifest, ayni);
        assert!(k.yerlesik["kalfa-rag"].ayar.calisma_alanlari["0123456789abcdef"].acik);

        // Yeni glob: izin silinir, açık kalır.
        let mut k = izinli(&eski);
        let yeni = manifest("\"**/*.md\", \"**/*.rs\"", "\"embeddings\"");
        izleme(&mut k, &yeni);
        assert!(!izin_var(&k));
        assert!(k.yerlesik["kalfa-rag"].ayar.calisma_alanlari["0123456789abcdef"].acik);
        assert_eq!(k.yerlesik["kalfa-rag"].manifest, yeni);

        // Yeni EVREN ucu.
        let mut k = izinli(&eski);
        izleme(
            &mut k,
            &manifest("\"**/*.md\"", "\"embeddings\", \"rerank\""),
        );
        assert!(!izin_var(&k));

        // Daralma da kapsamı değiştirir (uç kümesi değişti).
        let mut k = izinli(&manifest("\"**/*.md\"", "\"embeddings\", \"rerank\""));
        izleme(&mut k, &eski);
        assert!(!izin_var(&k));
    }

    #[test]
    fn yerlesik_ilk_gorulunce_ya_da_eski_cozulemezse_izin_silinir() {
        let m = manifest("\"**/*.md\"", "\"embeddings\"");
        // Hiç kayıt yok: girdi varsayılanla oluşur, manifest saklanır.
        let mut k = Kayit::default();
        izleme(&mut k, &m);
        assert_eq!(k.yerlesik["kalfa-rag"].manifest, m);
        assert!(k.yerlesik["kalfa-rag"].ayar.etkin);
        assert!(k.yerlesik["kalfa-rag"].ayar.calisma_alanlari.is_empty());

        // Saklanan metin bozuksa karşılaştırılamaz: güvenli yön izni siler.
        let mut k = izinli("bozuk metin");
        izleme(&mut k, &m);
        assert!(!izin_var(&k));
        assert_eq!(k.yerlesik["kalfa-rag"].manifest, m);

        // Aynı metin: hiçbir şey değişmez.
        let mut k = izinli(&m);
        let once = k.clone();
        izleme(&mut k, &m);
        assert_eq!(k, once);
    }
}
