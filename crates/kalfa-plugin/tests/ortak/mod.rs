//! `kalfa-plugin` testlerinin düzeneği: test eklentilerinin derlenmesi, bir
//! çalışma alanı ve depo, aracın çağrılması.
#![allow(dead_code)]

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, CiktiGonderici, CiktiParcasi};
use kalfa_core::izin::Siniflandirici;
use kalfa_core::kilit::KaynakKilitleri;
use kalfa_core::kimlik::CagriId;
use kalfa_core::protokol::CiktiAkisi;
use kalfa_core::saglayici::Saglayici;
use kalfa_core::sart_kapisi::SartKapisi;
use kalfa_core::test_destek::SahteSaglayici;
use kalfa_plugin::sinir::Sureler;
use kalfa_plugin::{BellekIzni, Eklenti, EklentiAraci, EvrenBaglami, GonderimIzni, Motor};
use kalfa_tools::CalismaAlani;
use serde_json::Value;
use tempfile::TempDir;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Test eklentilerini `wasm32-wasip2` için derler (aşama 4 tasarımı §17.1,
/// G7). Test ikilisi başına bir kez; derlenmişse cargo hiçbir şey yapmaz.
/// Testler `.wasm`'ları kendileri derlediği için `cargo test` tek başına
/// yeterlidir ve eski bir `.wasm`'la koşmaz.
fn derle() -> Result<PathBuf, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let cikti = Command::new(cargo)
        .args([
            "build",
            "--target",
            "wasm32-wasip2",
            "--locked",
            "--message-format=json-render-diagnostics",
            "-p",
            "kalfa-test-eklentisi",
            "-p",
            "kalfa-test-soket",
            "-p",
            "kalfa-test-yanlis-dunya",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("cargo çalıştırılamadı: {e}"))?;
    if !cikti.status.success() {
        return Err(format!(
            "Test eklentileri derlenemedi. `rustup target add wasm32-wasip2` gerekebilir. Durum: {}",
            cikti.status
        ));
    }
    String::from_utf8_lossy(&cikti.stdout)
        .lines()
        .filter_map(|s| serde_json::from_str::<Value>(s).ok())
        .filter(|m| m["reason"] == "compiler-artifact")
        .flat_map(|m| {
            m["filenames"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
        })
        .filter_map(|f| f.as_str().map(PathBuf::from))
        .find(|f| f.extension().is_some_and(|e| e == "wasm"))
        .and_then(|f| f.parent().map(PathBuf::from))
        .ok_or_else(|| "cargo çıktısında .wasm yok".into())
}

/// `crate_adi` test eklentisinin `.wasm`'ı.
pub fn wasm(crate_adi: &str) -> Vec<u8> {
    static DIZIN: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    let dizin = match DIZIN.get_or_init(derle) {
        Ok(d) => d,
        Err(e) => panic!("{e}"),
    };
    std::fs::read(dizin.join(format!("{}.wasm", crate_adi.replace('-', "_"))))
        .unwrap_or_else(|e| panic!("{crate_adi}.wasm okunamadı: {e}"))
}

/// `kalfa-test` adlı eklentinin manifesti.
pub fn manifest(dosya_oku: &[&str], evren: &[&str], depo_mb: u32) -> String {
    let liste = |l: &[&str]| {
        l.iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "[eklenti]\nad = \"kalfa-test\"\nsurum = \"0.1.0\"\nsozlesme = \"0.1\"\naciklama = \"Test eklentisi\"\n\n[yetenekler]\ndosya-oku = [{}]\nevren = [{}]\ndepo-mb = {depo_mb}\n",
        liste(dosya_oku),
        liste(evren)
    )
}

/// Testler için kısa süreler: CPU bütçesi 2 sn, toplam 20 sn.
pub fn motor() -> Arc<Motor> {
    motor_cpu(Duration::from_secs(2))
}

pub fn motor_cpu(cpu: Duration) -> Arc<Motor> {
    Motor::test_sureleri(Sureler {
        cpu,
        toplam: Duration::from_secs(20),
    })
    .unwrap()
}

/// Bir çağrının sonucu ve canlı çıktısı.
pub struct Cagri {
    pub sonuc: Result<AracCiktisi, AracHatasi>,
    pub cikti: Vec<(CiktiAkisi, String)>,
}

impl Cagri {
    pub fn icerik(&self) -> &str {
        match &self.sonuc {
            Ok(c) => &c.icerik,
            Err(e) => panic!("araç başarısız: {e}"),
        }
    }

    pub fn hata(&self) -> String {
        match &self.sonuc {
            Ok(c) => panic!("araç başarılı: {}", c.icerik),
            Err(e) => e.to_string(),
        }
    }

    pub fn cikti_metni(&self) -> String {
        self.cikti.iter().map(|(_, m)| m.as_str()).collect()
    }
}

pub struct Duzenek {
    pub alan: TempDir,
    pub depo: TempDir,
    pub motor: Arc<Motor>,
    pub eklenti: Arc<Eklenti>,
    pub calisma_alani: Arc<CalismaAlani>,
    pub kilitler: Arc<KaynakKilitleri>,
    /// EVREN bağlantısı; varsayılan yok.
    pub evren: Option<Arc<EvrenBaglami>>,
    /// Çalışma alanının gönderim izni; varsayılan yok.
    pub izin: Arc<dyn GonderimIzni>,
}

impl Duzenek {
    pub async fn yeni(manifest: &str) -> Self {
        Self::alanla(manifest, tempfile::tempdir().unwrap()).await
    }

    /// EVREN bağlantılı düzenek: `sahte` sağlayıcısı, onu kullanan şart kapısı
    /// ve verilen gönderim izni.
    pub async fn evrenli(
        manifest: &str,
        sahte: &Arc<SahteSaglayici>,
        izin: Arc<dyn GonderimIzni>,
    ) -> Self {
        let mut d = Self::yeni(manifest).await;
        let saglayici: Arc<dyn Saglayici> = sahte.clone();
        let kapi = Arc::new(SartKapisi::yeni(saglayici.clone()));
        d.evren = Some(EvrenBaglami::yeni(saglayici, kapi));
        d.izin = izin;
        d
    }

    pub async fn alanla(manifest: &str, alan: TempDir) -> Self {
        Self::kur(manifest, alan, motor()).await
    }

    /// CPU bütçesi verilen motorla.
    pub async fn cpu(manifest: &str, cpu: Duration) -> Self {
        Self::kur(manifest, tempfile::tempdir().unwrap(), motor_cpu(cpu)).await
    }

    async fn kur(manifest: &str, alan: TempDir, motor: Arc<Motor>) -> Self {
        let eklenti = Arc::new(
            motor
                .yukle(manifest, wasm("kalfa-test-eklentisi"))
                .await
                .unwrap(),
        );
        let siniflandirici = Arc::new(Siniflandirici::yeni(alan.path(), &[]).unwrap());
        Self {
            calisma_alani: CalismaAlani::ac(siniflandirici).unwrap(),
            alan,
            depo: tempfile::tempdir().unwrap(),
            motor,
            eklenti,
            kilitler: Arc::new(KaynakKilitleri::yeni()),
            evren: None,
            izin: BellekIzni::yeni(false),
        }
    }

    /// Çalışma alanına bir dosya koyar; üst dizinler oluşturulur.
    pub fn dosya(&self, yol: &str, icerik: &[u8]) -> &Self {
        let tam = self.alan.path().join(yol);
        std::fs::create_dir_all(tam.parent().unwrap()).unwrap();
        std::fs::write(tam, icerik).unwrap();
        self
    }

    /// Bu çalışma alanının, `depo_dizini`'ni kullanan araçları.
    pub fn araclar_depoyla(&self, depo_dizini: &std::path::Path) -> Vec<Arc<dyn Arac>> {
        EklentiAraci::hepsi(
            &self.motor,
            &self.eklenti,
            &self.calisma_alani,
            &self.kilitler,
            depo_dizini,
            self.evren.clone(),
            self.izin.clone(),
        )
    }

    pub fn arac(&self, ad: &str) -> Arc<dyn Arac> {
        self.arac_depoyla(ad, self.depo.path())
    }

    pub fn arac_depoyla(&self, ad: &str, depo: &std::path::Path) -> Arc<dyn Arac> {
        let model_adi = format!("kalfa_test__{ad}");
        self.araclar_depoyla(depo)
            .into_iter()
            .find(|a| a.ad() == model_adi)
            .unwrap_or_else(|| panic!("{model_adi} yok"))
    }

    pub async fn cagir(&self, ad: &str, argumanlar: Value) -> Cagri {
        self.cagir_iptalle(ad, argumanlar, CancellationToken::new())
            .await
    }

    pub async fn cagir_iptalle(
        &self,
        ad: &str,
        argumanlar: Value,
        iptal: CancellationToken,
    ) -> Cagri {
        cagir(&self.arac(ad), argumanlar, iptal).await
    }
}

pub async fn cagir(arac: &Arc<dyn Arac>, argumanlar: Value, iptal: CancellationToken) -> Cagri {
    let (tx, mut rx) = mpsc::channel::<CiktiParcasi>(1024);
    let hazirlik = arac.hazirla(&argumanlar).await.unwrap();
    let sonuc = arac
        .calistir_akisli(
            &argumanlar,
            &hazirlik,
            iptal,
            CiktiGonderici::yeni(CagriId::yeni(), tx),
        )
        .await;
    let mut cikti = Vec::new();
    while let Ok(p) = rx.try_recv() {
        cikti.push((p.akis, p.metin));
    }
    Cagri { sonuc, cikti }
}
