//! `<veri>/eklentiler.json` (aşama 4 tasarımı §13.4): kurulu ve yerleşik
//! eklentilerin kaydı, çalışma alanı başına açık/kapalı durumu ve EVREN
//! gönderim izinleri. Dosya uygulamanın veri dizinindedir; çalışma alanındaki
//! ya da eklenti deposundaki bir dosya onu değiştiremez.
//!
//! Okuma toleranslıdır (3e'deki gizli kalıp dersi): her girdi ayrı çözülür,
//! bozuk bir girdi yalnızca o eklentiyi düşürür. Düşen girdi dosyadan
//! silinmez; ham hâliyle yazılır ve kullanıcı elle düzeltebilir.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use kalfa_plugin::GonderimIzni;
use kalfa_plugin::manifest::eklenti_adi_gecerli;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::hata::KomutHatasi;

const DOSYA_ADI: &str = "eklentiler.json";
const KAYIT_SURUMU: u32 = 1;

/// Çalışma alanı kimliği tam 16 küçük onaltılık karakterdir
/// (`CalismaAlaniKaydi::kimlik`). Kimlikten yol kuran her yer bunu doğrular.
pub(crate) fn kimlik_gecerli(kimlik: &str) -> bool {
    kimlik.len() == 16
        && kimlik
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Şimdiki zaman, Unix ms.
pub(crate) fn simdi_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|s| u64::try_from(s.as_millis()).ok())
        .unwrap_or(0)
}

fn evet() -> bool {
    true
}

/// Bir eklentinin bir çalışma alanındaki durumu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AlanKaydi {
    /// Gösterim için; arayüz geri göndermez.
    pub yol: String,
    pub acik: bool,
    /// EVREN'e gönderim izninin verildiği zaman (Unix ms).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gonderim_izni_ms: Option<u64>,
}

/// Kurulu ve yerleşik eklentilerin ortak ayarı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Ayar {
    /// Kapalıysa eklenti hiçbir çalışma alanında sunulmaz; ayarları kalır.
    #[serde(default = "evet")]
    pub etkin: bool,
    #[serde(default)]
    pub calisma_alanlari: BTreeMap<String, AlanKaydi>,
}

impl Default for Ayar {
    fn default() -> Self {
        Self {
            etkin: true,
            calisma_alanlari: BTreeMap::new(),
        }
    }
}

/// Klasörden kurulmuş eklenti.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct KuruluKaydi {
    pub ad: String,
    pub surum: String,
    /// Kurulan `.wasm`'ın SHA-256'sı (onaltılık).
    pub sha256: String,
    pub kaynak: String,
    pub kurulum_ms: u64,
    #[serde(flatten)]
    pub ayar: Ayar,
}

impl KuruluKaydi {
    /// Yol kurmakta kullanılan her değer (ad, çalışma alanı kimlikleri)
    /// geçerli olmalı; `../x` gibi bir değer girdiyi bozuk sayar.
    fn dogrula(&self) -> Result<(), String> {
        if !eklenti_adi_gecerli(&self.ad) {
            return Err("eklenti adı geçersiz".into());
        }
        if self
            .ayar
            .calisma_alanlari
            .keys()
            .any(|k| !kimlik_gecerli(k))
        {
            return Err("çalışma alanı kimliği geçersiz".into());
        }
        Ok(())
    }
}

/// Kalfa'yla gelen eklentinin kaydı. Dosyaları kayda girmez; yalnızca ayarı
/// ve son görülen manifesti tutulur.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct YerlesikKaydi {
    /// Son görülen `kalfa.toml` metni: yeni sürümün gönderim kapsamı bununla
    /// karşılaştırılır (§13.1).
    #[serde(default)]
    pub manifest: String,
    #[serde(flatten)]
    pub ayar: Ayar,
}

/// Çözülemeyen kayıt girdisi.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BozukGirdi {
    /// Adı okunabildiyse (ve geçerliyse) listede görünür.
    pub ad: Option<String>,
    pub neden: String,
    /// Dosyaya olduğu gibi geri yazılır.
    pub ham: Value,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Kayit {
    pub kurulu: Vec<KuruluKaydi>,
    pub yerlesik: BTreeMap<String, YerlesikKaydi>,
    pub bozuk: Vec<BozukGirdi>,
}

impl Kayit {
    /// Girdileri tek tek çözer. Dosya bir JSON nesnesi bile değilse `Err`.
    pub fn coz(veri: &[u8]) -> Result<Self, String> {
        let kok: Value = serde_json::from_slice(veri).map_err(|e| e.to_string())?;
        let nesne = kok
            .as_object()
            .ok_or_else(|| "kök bir JSON nesnesi değil".to_string())?;
        let mut kayit = Self::default();
        for ham in nesne
            .get("eklentiler")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            kayit.girdi_ekle(ham);
        }
        for (ad, ham) in nesne
            .get("yerlesik")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
        {
            // Bozuk yerleşik girdi varsayılana döner: izinler ve açık
            // çalışma alanları kapalı kalır (güvenli yön).
            let girdi = serde_json::from_value::<YerlesikKaydi>(ham.clone())
                .ok()
                .filter(|y| y.ayar.calisma_alanlari.keys().all(|k| kimlik_gecerli(k)));
            if let Some(girdi) = girdi
                && eklenti_adi_gecerli(ad)
            {
                kayit.yerlesik.insert(ad.clone(), girdi);
            }
        }
        Ok(kayit)
    }

    fn girdi_ekle(&mut self, ham: &Value) {
        let sonuc = serde_json::from_value::<KuruluKaydi>(ham.clone())
            .map_err(|e| e.to_string())
            .and_then(|g| g.dogrula().map(|()| g));
        match sonuc {
            Ok(girdi) if self.kurulu(&girdi.ad).is_some() => self.bozuk.push(BozukGirdi {
                ad: Some(girdi.ad),
                neden: "Kayıtta aynı adlı ikinci bir girdi var; yalnızca ilki kullanılıyor.".into(),
                ham: ham.clone(),
            }),
            Ok(girdi) => self.kurulu.push(girdi),
            Err(ayrinti) => {
                let ad = ham
                    .get("ad")
                    .and_then(Value::as_str)
                    .filter(|a| eklenti_adi_gecerli(a))
                    .map(String::from);
                self.bozuk.push(BozukGirdi {
                    ad,
                    neden: format!(
                        "Eklentinin kayıt girdisi bozuk ({ayrinti}); eklentiyi kaldırıp yeniden kurun."
                    ),
                    ham: ham.clone(),
                });
            }
        }
    }

    /// Dosyaya yazılacak değer; bozuk girdiler ham hâlleriyle sona eklenir.
    pub fn degeri(&self) -> Result<Value, serde_json::Error> {
        let mut eklentiler = self
            .kurulu
            .iter()
            .map(serde_json::to_value)
            .collect::<Result<Vec<_>, _>>()?;
        eklentiler.extend(self.bozuk.iter().map(|b| b.ham.clone()));
        Ok(json!({
            "surum": KAYIT_SURUMU,
            "eklentiler": eklentiler,
            "yerlesik": serde_json::to_value(&self.yerlesik)?,
        }))
    }

    pub fn kurulu(&self, ad: &str) -> Option<&KuruluKaydi> {
        self.kurulu.iter().find(|e| e.ad == ad)
    }

    pub fn kurulu_mut(&mut self, ad: &str) -> Option<&mut KuruluKaydi> {
        self.kurulu.iter_mut().find(|e| e.ad == ad)
    }

    /// Eklentinin ayarı. Yerleşik eklentinin girdisi yoksa varsayılanla
    /// oluşturulur; kurulu eklentinin girdisi yoksa `None`.
    pub fn ayar_mut(&mut self, ad: &str, yerlesik: bool) -> Option<&mut Ayar> {
        if yerlesik {
            Some(&mut self.yerlesik.entry(ad.to_string()).or_default().ayar)
        } else {
            self.kurulu_mut(ad).map(|e| &mut e.ayar)
        }
    }

    /// Salt okunur ayar ve kurulu eklentinin kayıtlı kurulumu: `.wasm`'ın
    /// özeti ve kurulum zamanı (yerleşikte `None`).
    pub fn ayar(&self, ad: &str, yerlesik: bool) -> Option<(&Ayar, Option<(&str, u64)>)> {
        if yerlesik {
            self.yerlesik.get(ad).map(|y| (&y.ayar, None))
        } else {
            self.kurulu(ad)
                .map(|e| (&e.ayar, Some((e.sha256.as_str(), e.kurulum_ms))))
        }
    }

    /// Araç şu an sunulabilir mi (§8.1, §13.3): eklenti kayıtlı ve etkin, bu
    /// araç kayıttaki kurulumdan yüklenmiş (`kurulum`; yerleşikte `None`) ve
    /// çalışma alanında açık.
    fn kullanilabilir(
        &self,
        ad: &str,
        yerlesik: bool,
        kurulum: Option<(&str, u64)>,
        kimlik: &str,
    ) -> bool {
        let Some((ayar, kayitli)) = self.ayar(ad, yerlesik) else {
            return false;
        };
        ayar.etkin
            && kayitli == kurulum
            && ayar.calisma_alanlari.get(kimlik).is_some_and(|a| a.acik)
    }
}

/// Bellekteki tek kopya ve dosyası. Kilit hiçbir `await` boyunca tutulmaz.
pub(crate) struct EklentiKaydi {
    yol: PathBuf,
    ic: Mutex<Kayit>,
}

impl EklentiKaydi {
    /// Dosya yoksa ya da tamamen bozuksa kayıt boştur. Bozuk dosya bir sonraki
    /// yazmada ezilmesin diye yanına `.bozuk` olarak kopyalanır.
    pub fn oku(veri_dizini: &Path) -> Self {
        let yol = veri_dizini.join(DOSYA_ADI);
        let kayit = match std::fs::read(&yol) {
            Ok(veri) => Kayit::coz(&veri).unwrap_or_else(|_| {
                let _ = std::fs::copy(&yol, yol.with_extension("json.bozuk"));
                Kayit::default()
            }),
            Err(_) => Kayit::default(),
        };
        Self {
            yol,
            ic: Mutex::new(kayit),
        }
    }

    fn kilit(&self) -> MutexGuard<'_, Kayit> {
        self.ic.lock().unwrap_or_else(|z| z.into_inner())
    }

    pub fn anlik(&self) -> Kayit {
        self.kilit().clone()
    }

    pub fn bak<T>(&self, is: impl FnOnce(&Kayit) -> T) -> T {
        is(&self.kilit())
    }

    /// Değişikliği bir kopyaya uygular, dosyayı atomik yazar, sonra kopyayı
    /// asıl yapar. `is` hata dönerse ya da yazılamazsa bellek de dosya da
    /// eski hâlinde kalır. Değişiklik bir şey değiştirmediyse yazılmaz.
    pub fn degistir<T>(
        &self,
        is: impl FnOnce(&mut Kayit) -> Result<T, KomutHatasi>,
    ) -> Result<T, KomutHatasi> {
        let mut ic = self.kilit();
        let mut yeni = ic.clone();
        let sonuc = is(&mut yeni)?;
        if yeni != *ic {
            yaz(&self.yol, &yeni).map_err(|e| {
                KomutHatasi::ic(format!("Eklenti kaydı (eklentiler.json) yazılamadı: {e}"))
            })?;
            *ic = yeni;
        }
        Ok(sonuc)
    }

    /// Sıkılaştıran değişiklik: dosya yazılamasa da bellekte uygulanır, çünkü
    /// izinlerin eski hâle dönmesi güvenli yön değildir. Yazma hatası dönülür.
    pub fn sikilastir(&self, is: impl FnOnce(&mut Kayit)) -> Result<(), String> {
        let mut ic = self.kilit();
        let mut yeni = ic.clone();
        is(&mut yeni);
        if yeni == *ic {
            return Ok(());
        }
        let sonuc = yaz(&self.yol, &yeni).map_err(|e| e.to_string());
        *ic = yeni;
        sonuc
    }
}

/// Aynı dizinde geçici dosya, sonra yeniden adlandırma (`ayarlar.json` gibi).
fn yaz(yol: &Path, kayit: &Kayit) -> io::Result<()> {
    let metin = kayit.degeri().and_then(|d| serde_json::to_vec_pretty(&d))?;
    if let Some(ust) = yol.parent() {
        std::fs::create_dir_all(ust)?;
    }
    let gecici = yol.with_extension("json.yeni");
    std::fs::write(&gecici, metin)?;
    std::fs::rename(&gecici, yol).inspect_err(|_| {
        let _ = std::fs::remove_file(&gecici);
    })
}

/// Kayda bakan gönderim izni (`kalfa-plugin`): bir eklentinin bir çalışma
/// alanındaki durumu. Eklenti aracı her turda `acik_mi`'yi sorar; kaldırılan,
/// devre dışı bırakılan, güncellenen ya da kapatılan eklentinin eski araçları
/// açık oturumlarda böylece sunulmaz.
pub(crate) struct AlanIzni {
    pub kayit: Arc<EklentiKaydi>,
    pub ad: String,
    pub kimlik: String,
    pub yerlesik: bool,
    /// Aracın yüklendiği `.wasm`'ın özeti ve kurulum zamanı. Kurulu eklentide
    /// kayıttakilerle aynı kalmalıdır: güncelleme ya da yeniden kurulum, aynı
    /// `.wasm` ile yalnızca manifest değişse bile, eski araçları sunulmaz
    /// yapar. Yerleşikte karşılaştırılmaz (dosyalarına yalnızca yönetici
    /// yazabilir, §13.1).
    pub sha256: String,
    pub kurulum_ms: u64,
}

impl AlanIzni {
    fn kurulum(&self) -> Option<(&str, u64)> {
        (!self.yerlesik).then_some((self.sha256.as_str(), self.kurulum_ms))
    }
}

impl GonderimIzni for AlanIzni {
    fn var_mi(&self) -> bool {
        self.kayit.bak(|k| {
            k.ayar(&self.ad, self.yerlesik)
                .and_then(|(ayar, _)| ayar.calisma_alanlari.get(&self.kimlik))
                .is_some_and(|a| a.gonderim_izni_ms.is_some())
        })
    }

    /// İzin kalıcıdır; yazılamazsa araç çalışmaz. Eklenti bu arada kapatıldı,
    /// devre dışı bırakıldı ya da güncellendiyse izin de verilmez.
    fn ver(&self) -> Result<(), String> {
        self.kayit
            .degistir(|k| {
                if !k.kullanilabilir(&self.ad, self.yerlesik, self.kurulum(), &self.kimlik) {
                    return Err(KomutHatasi::gecersiz(
                        "Eklenti bu çalışma alanında artık kullanılamıyor.",
                    ));
                }
                k.ayar_mut(&self.ad, self.yerlesik)
                    .and_then(|ayar| ayar.calisma_alanlari.get_mut(&self.kimlik))
                    .ok_or_else(|| KomutHatasi::gecersiz("Çalışma alanı kayıtta yok."))?
                    .gonderim_izni_ms = Some(simdi_ms());
                Ok(())
            })
            .map_err(|e| e.mesaj)
    }

    fn acik_mi(&self) -> bool {
        self.kayit
            .bak(|k| k.kullanilabilir(&self.ad, self.yerlesik, self.kurulum(), &self.kimlik))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KIMLIK: &str = "0123456789abcdef";
    const DIGER: &str = "fedcba9876543210";

    fn kurulu(ad: &str, sha: &str) -> KuruluKaydi {
        KuruluKaydi {
            ad: ad.into(),
            surum: "0.2.0".into(),
            sha256: sha.into(),
            kaynak: "C:\\kaynak".into(),
            kurulum_ms: 1_791_700_000_000,
            ayar: Ayar::default(),
        }
    }

    fn alan(acik: bool, izin: Option<u64>) -> AlanKaydi {
        AlanKaydi {
            yol: "C:\\proje".into(),
            acik,
            gonderim_izni_ms: izin,
        }
    }

    fn dosyali(dizin: &Path, kayit: &Kayit) -> Arc<EklentiKaydi> {
        yaz(&dizin.join(DOSYA_ADI), kayit).unwrap();
        Arc::new(EklentiKaydi::oku(dizin))
    }

    #[test]
    fn kimlik_tam_16_kucuk_onaltilik() {
        assert!(kimlik_gecerli(KIMLIK));
        for kotu in [
            "",
            "0123456789abcde",
            "0123456789abcdef0",
            "0123456789ABCDEF",
            "../0123456789abc",
            "0123456789abcdeg",
            "a/b",
            "..",
        ] {
            assert!(!kimlik_gecerli(kotu), "{kotu:?}");
        }
    }

    #[test]
    fn kayit_belgedeki_bicimde_yazilir_ve_okunur() {
        let mut k = Kayit::default();
        let mut e = kurulu("ornek", "abc");
        e.ayar
            .calisma_alanlari
            .insert(KIMLIK.into(), alan(true, Some(1_791_700_000_000)));
        k.kurulu.push(e);
        let y = k.yerlesik.entry("kalfa-rag".into()).or_default();
        y.manifest = "[eklenti]".into();
        y.ayar
            .calisma_alanlari
            .insert(DIGER.into(), alan(false, None));

        let deger = k.degeri().unwrap();
        assert_eq!(deger["surum"], 1);
        let e0 = &deger["eklentiler"][0];
        assert_eq!(e0["ad"], "ornek");
        assert_eq!(e0["surum"], "0.2.0");
        assert_eq!(e0["sha256"], "abc");
        assert_eq!(e0["kaynak"], "C:\\kaynak");
        assert_eq!(e0["kurulum_ms"], 1_791_700_000_000u64);
        assert_eq!(e0["etkin"], true);
        let a = &e0["calisma_alanlari"][KIMLIK];
        assert_eq!(a["yol"], "C:\\proje");
        assert_eq!(a["acik"], true);
        assert_eq!(a["gonderim_izni_ms"], 1_791_700_000_000u64);
        let r = &deger["yerlesik"]["kalfa-rag"];
        assert_eq!(r["etkin"], true);
        assert_eq!(r["manifest"], "[eklenti]");
        assert!(
            r["calisma_alanlari"][DIGER]
                .as_object()
                .is_some_and(|o| !o.contains_key("gonderim_izni_ms")),
            "izin yoksa alan yazılmaz"
        );

        let geri = Kayit::coz(&serde_json::to_vec(&deger).unwrap()).unwrap();
        assert_eq!(geri, k);
    }

    #[test]
    fn bozuk_girdi_yalnizca_kendini_dusurur_ve_ham_yazilir() {
        let iyi = serde_json::to_value(kurulu("iyi", "abc")).unwrap();
        let ham = json!({
            "surum": 1,
            "eklentiler": [
                iyi,
                {"ad": "sha-yok", "surum": "1.0.0"},
                {"ad": "../x", "surum": "1.0.0", "sha256": "a", "kaynak": "k", "kurulum_ms": 1},
                {"ad": "kotu-kimlik", "surum": "1.0.0", "sha256": "a", "kaynak": "k", "kurulum_ms": 1,
                 "calisma_alanlari": {"../x": {"yol": "y", "acik": true}}},
                "metin",
                {"ad": "iyi", "surum": "9.9.9", "sha256": "a", "kaynak": "k", "kurulum_ms": 1}
            ],
            "yerlesik": {
                "kalfa-rag": {"etkin": false, "manifest": "m"},
                "bozuk-yerlesik": {"etkin": "belki"},
                "../x": {"etkin": true}
            }
        });
        let k = Kayit::coz(&serde_json::to_vec(&ham).unwrap()).unwrap();

        assert_eq!(k.kurulu.len(), 1);
        assert_eq!(k.kurulu[0].ad, "iyi");
        assert_eq!(k.kurulu[0].surum, "0.2.0", "yinelenen girdi ilkini ezmez");
        let adlar: Vec<Option<&str>> = k.bozuk.iter().map(|b| b.ad.as_deref()).collect();
        assert_eq!(
            adlar,
            vec![
                Some("sha-yok"),
                None,
                Some("kotu-kimlik"),
                None,
                Some("iyi")
            ]
        );
        assert!(k.bozuk[0].neden.contains("sha256"), "{}", k.bozuk[0].neden);
        assert!(k.bozuk[2].neden.contains("kimliği"), "{}", k.bozuk[2].neden);
        assert!(!k.yerlesik["kalfa-rag"].ayar.etkin);
        assert_eq!(k.yerlesik.len(), 1, "bozuk yerleşik girdiler düşer");

        let yazilan = k.degeri().unwrap();
        let eklentiler = yazilan["eklentiler"].as_array().unwrap();
        assert_eq!(eklentiler.len(), 6, "bozuk girdiler ham hâliyle korunur");
        assert_eq!(eklentiler[2], ham["eklentiler"][2]);
        assert_eq!(eklentiler[5], ham["eklentiler"][5]);
    }

    #[test]
    fn dosya_yoksa_ya_da_tamamen_bozuksa_bos_kayit() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(EklentiKaydi::oku(d.path()).anlik(), Kayit::default());

        std::fs::write(d.path().join(DOSYA_ADI), "{bozuk").unwrap();
        assert_eq!(EklentiKaydi::oku(d.path()).anlik(), Kayit::default());
        assert_eq!(
            std::fs::read_to_string(d.path().join("eklentiler.json.bozuk")).unwrap(),
            "{bozuk",
            "bozuk dosya ezilmeden önce yedeklenir"
        );

        std::fs::write(d.path().join(DOSYA_ADI), "[1,2]").unwrap();
        assert_eq!(EklentiKaydi::oku(d.path()).anlik(), Kayit::default());
    }

    #[test]
    fn degistir_atomik_yazar_ve_degismeyeni_yazmaz() {
        let d = tempfile::tempdir().unwrap();
        let kayit = EklentiKaydi::oku(d.path());
        kayit
            .degistir(|k| {
                k.kurulu.push(kurulu("ornek", "abc"));
                Ok(())
            })
            .unwrap();
        let yol = d.path().join(DOSYA_ADI);
        assert!(yol.is_file());
        assert!(!d.path().join("eklentiler.json.yeni").exists());
        assert_eq!(EklentiKaydi::oku(d.path()).anlik(), kayit.anlik());

        std::fs::remove_file(&yol).unwrap();
        kayit.degistir(|_| Ok(())).unwrap();
        assert!(!yol.exists(), "değişmeyen kayıt yeniden yazılmaz");
    }

    #[test]
    fn yazma_hatasinda_bellek_eski_halinde_kalir() {
        let d = tempfile::tempdir().unwrap();
        // Hedef bir dizin olduğu için yeniden adlandırma başarısız olur.
        std::fs::create_dir(d.path().join(DOSYA_ADI)).unwrap();
        let kayit = EklentiKaydi::oku(d.path());
        let hata = kayit
            .degistir(|k| {
                k.kurulu.push(kurulu("ornek", "abc"));
                Ok(())
            })
            .unwrap_err();
        assert!(hata.mesaj.contains("eklentiler.json"), "{}", hata.mesaj);
        assert!(kayit.anlik().kurulu.is_empty());
        assert!(
            !d.path().join("eklentiler.json.yeni").exists(),
            "geçici dosya bırakılmaz"
        );

        // Sıkılaştırma yazılamasa da bellekte uygulanır.
        let sonuc = kayit.sikilastir(|k| k.kurulu.push(kurulu("ornek", "abc")));
        assert!(sonuc.is_err());
        assert_eq!(kayit.anlik().kurulu.len(), 1);
    }

    #[test]
    fn degistir_hata_donerse_hicbir_sey_degismez() {
        let d = tempfile::tempdir().unwrap();
        let kayit = EklentiKaydi::oku(d.path());
        let hata = kayit
            .degistir(|k| -> Result<(), KomutHatasi> {
                k.kurulu.push(kurulu("ornek", "abc"));
                Err(KomutHatasi::gecersiz("olmaz"))
            })
            .unwrap_err();
        assert_eq!(hata.mesaj, "olmaz");
        assert!(kayit.anlik().kurulu.is_empty());
        assert!(!d.path().join(DOSYA_ADI).exists());
    }

    fn izinli_kayit(dizin: &Path, etkin: bool, acik: bool) -> Arc<EklentiKaydi> {
        let mut k = Kayit::default();
        let mut e = kurulu("ornek", "sha-1");
        e.ayar.etkin = etkin;
        e.ayar
            .calisma_alanlari
            .insert(KIMLIK.into(), alan(acik, None));
        k.kurulu.push(e);
        dosyali(dizin, &k)
    }

    /// `kurulu()`'nun kurulum zamanı.
    const KURULUM_MS: u64 = 1_791_700_000_000;

    fn izin(kayit: &Arc<EklentiKaydi>, sha: &str, kimlik: &str) -> AlanIzni {
        AlanIzni {
            kayit: kayit.clone(),
            ad: "ornek".into(),
            kimlik: kimlik.into(),
            yerlesik: false,
            sha256: sha.into(),
            kurulum_ms: KURULUM_MS,
        }
    }

    #[test]
    fn izin_ver_kalici_yazar_var_mi_gorur() {
        let d = tempfile::tempdir().unwrap();
        let kayit = izinli_kayit(d.path(), true, true);
        let i = izin(&kayit, "sha-1", KIMLIK);
        assert!(i.acik_mi());
        assert!(!i.var_mi());
        i.ver().unwrap();
        assert!(i.var_mi());
        assert!(
            EklentiKaydi::oku(d.path())
                .anlik()
                .kurulu("ornek")
                .and_then(|e| e.ayar.calisma_alanlari.get(KIMLIK))
                .is_some_and(|a| a.gonderim_izni_ms.is_some_and(|ms| ms > 0)),
            "izin dosyaya yazıldı"
        );
        assert!(
            !izin(&kayit, "sha-1", DIGER).var_mi(),
            "başka çalışma alanı"
        );
    }

    #[test]
    fn eklenti_kapali_devre_disi_kaldirilmis_ya_da_guncellenmisse_sunulmaz() {
        let d = tempfile::tempdir().unwrap();
        let acik = izinli_kayit(d.path(), true, true);
        assert!(izin(&acik, "sha-1", KIMLIK).acik_mi());
        assert!(
            !izin(&acik, "sha-2", KIMLIK).acik_mi(),
            "güncellenmiş: eski sürümün araçları sunulmaz"
        );
        assert!(
            !izin(&acik, "sha-1", DIGER).acik_mi(),
            "çalışma alanı kayıtta yok"
        );
        // Aynı `.wasm` ile yalnızca manifest değişen güncelleme de kurulum
        // zamanını yeniler; eski manifestle yüklenmiş araçlar sunulmaz.
        let eski_kurulum = AlanIzni {
            kurulum_ms: KURULUM_MS - 1,
            ..izin(&acik, "sha-1", KIMLIK)
        };
        assert!(!eski_kurulum.acik_mi());
        assert!(eski_kurulum.ver().is_err());

        let d2 = tempfile::tempdir().unwrap();
        let kapali = izinli_kayit(d2.path(), true, false);
        assert!(!izin(&kapali, "sha-1", KIMLIK).acik_mi());

        let d3 = tempfile::tempdir().unwrap();
        let devre_disi = izinli_kayit(d3.path(), false, true);
        assert!(!izin(&devre_disi, "sha-1", KIMLIK).acik_mi());

        // Anahtar canlı çalışır: aynı kayıt değişince bir sonraki soru yeni
        // durumu görür.
        let i = izin(&acik, "sha-1", KIMLIK);
        acik.degistir(|k| {
            k.kurulu_mut("ornek").unwrap().ayar.etkin = false;
            Ok(())
        })
        .unwrap();
        assert!(!i.acik_mi());
        acik.degistir(|k| {
            k.kurulu_mut("ornek").unwrap().ayar.etkin = true;
            Ok(())
        })
        .unwrap();
        assert!(i.acik_mi());
        acik.degistir(|k| {
            k.kurulu.clear();
            Ok(())
        })
        .unwrap();
        assert!(!i.acik_mi(), "kaldırılmış");
    }

    #[test]
    fn kullanilamayan_eklentiye_izin_verilmez() {
        let d = tempfile::tempdir().unwrap();
        let kayit = izinli_kayit(d.path(), true, true);
        let i = izin(&kayit, "sha-1", KIMLIK);
        kayit
            .degistir(|k| {
                k.kurulu_mut("ornek").unwrap().ayar.etkin = false;
                Ok(())
            })
            .unwrap();
        assert!(i.ver().unwrap_err().contains("kullanılamıyor"));
        assert!(!i.var_mi());
        assert!(izin(&kayit, "sha-2", KIMLIK).ver().is_err());
    }

    #[test]
    fn izin_yazilamazsa_ver_hata_doner_ve_izin_olusmaz() {
        let d = tempfile::tempdir().unwrap();
        let kayit = izinli_kayit(d.path(), true, true);
        // Bundan sonra dosya yazılamaz: geçici dosyanın yerine dizin konur.
        std::fs::create_dir(d.path().join("eklentiler.json.yeni")).unwrap();
        let i = izin(&kayit, "sha-1", KIMLIK);
        let hata = i.ver().unwrap_err();
        assert!(hata.contains("yazılamadı"), "{hata}");
        assert!(!i.var_mi(), "kaydedilemeyen izin bellekte de yok");
    }

    #[test]
    fn yerlesik_eklenti_sha_karsilastirmaz_ve_varsayilan_kapali() {
        let d = tempfile::tempdir().unwrap();
        let kayit = Arc::new(EklentiKaydi::oku(d.path()));
        let yerlesik = |sha: &str| AlanIzni {
            kayit: kayit.clone(),
            ad: "kalfa-rag".into(),
            kimlik: KIMLIK.into(),
            yerlesik: true,
            sha256: sha.into(),
            kurulum_ms: 0,
        };
        assert!(
            !yerlesik("x").acik_mi(),
            "kayıt yok: hiçbir alanda açık değil"
        );
        kayit
            .degistir(|k| {
                let y = k.yerlesik.entry("kalfa-rag".into()).or_default();
                y.ayar
                    .calisma_alanlari
                    .insert(KIMLIK.into(), alan(true, None));
                Ok(())
            })
            .unwrap();
        assert!(yerlesik("x").acik_mi());
        assert!(yerlesik("baska").acik_mi());
        yerlesik("x").ver().unwrap();
        assert!(yerlesik("x").var_mi());
    }
}
