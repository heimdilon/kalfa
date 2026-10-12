//! `kalfa-rag`'ın bütünleşme düzeneği: eklentiyi `wasm32-wasip2` için derler,
//! gerçek host'la (`kalfa-plugin`) bir çalışma alanında yükler ve aracını
//! çağırır.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, OnceLock};

use kalfa_core::arac::{Arac, AracOrtami};
use kalfa_core::izin::Siniflandirici;
use kalfa_core::kilit::KaynakKilitleri;
use kalfa_core::saglayici::Saglayici;
use kalfa_core::sart_kapisi::SartKapisi;
use kalfa_plugin::{BellekIzni, EklentiAraci, EvrenBaglami, Motor};
use kalfa_tools::CalismaAlani;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

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
            "kalfa-rag",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("cargo çalıştırılamadı: {e}"))?;
    if !cikti.status.success() {
        return Err(format!(
            "kalfa-rag derlenemedi. `rustup target add wasm32-wasip2` gerekebilir. Durum: {}",
            cikti.status
        ));
    }
    String::from_utf8_lossy(&cikti.stdout)
        .lines()
        .filter_map(|s| serde_json::from_str::<Value>(s).ok())
        .filter(|m| m["reason"] == "compiler-artifact")
        .flat_map(|m| m["filenames"].as_array().cloned().unwrap_or_default())
        .filter_map(|f| f.as_str().map(PathBuf::from))
        .find(|f| f.extension().is_some_and(|e| e == "wasm"))
        .ok_or_else(|| "cargo çıktısında .wasm yok".into())
}

/// Derlenmiş `kalfa_rag.wasm`; test ikilisi başına bir kez derlenir.
pub fn wasm() -> Vec<u8> {
    static YOL: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    match YOL.get_or_init(derle) {
        Ok(y) => std::fs::read(y).unwrap(),
        Err(e) => panic!("{e}"),
    }
}

pub fn manifest() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("kalfa.toml")).unwrap()
}

/// Bir çalışma alanında, gönderim izni verilmiş `kalfa_rag__anlamsal_ara`.
pub async fn arac(alan: &Path, depo: &Path, saglayici: Arc<dyn Saglayici>) -> Arc<dyn Arac> {
    let motor = Motor::yeni().unwrap();
    let eklenti = Arc::new(motor.yukle(&manifest(), wasm()).await.unwrap());
    let siniflandirici = Arc::new(Siniflandirici::yeni(alan, &[]).unwrap());
    let kapi = Arc::new(SartKapisi::yeni(saglayici.clone()));
    EklentiAraci::hepsi(
        &motor,
        &eklenti,
        &CalismaAlani::ac(siniflandirici).unwrap(),
        &Arc::new(KaynakKilitleri::yeni()),
        depo,
        Some(EvrenBaglami::yeni(saglayici, kapi)),
        BellekIzni::yeni(true),
    )
    .into_iter()
    .find(|a| a.ad() == "kalfa_rag__anlamsal_ara")
    .unwrap()
}

/// Aracı oturumsuz ortamla çağırır (gönderim talepleri kendiliğinden geçer).
pub async fn ara(arac: &Arc<dyn Arac>, sorgu: &str, sonuc_sayisi: u32) -> Result<String, String> {
    let a = json!({"sorgu": sorgu, "sonuc_sayisi": sonuc_sayisi});
    let hazirlik = arac.hazirla(&a).await.unwrap();
    arac.calistir_ortamli(&a, &hazirlik, CancellationToken::new(), AracOrtami::bos())
        .await
        .map(|c| c.icerik)
        .map_err(|e| e.to_string())
}

/// Sonuçtaki `[i] dosya:<yol>:<a>-<b>` başlıklarının yolları, sırayla.
pub fn sonuc_yollari(cikti: &str) -> Vec<String> {
    cikti
        .lines()
        .filter_map(|s| s.split_once("] dosya:"))
        .filter_map(|(_, kalan)| kalan.rsplit_once(" (puan").map(|(y, _)| y))
        .filter_map(|y| y.rsplit_once(':').map(|(yol, _)| yol.to_string()))
        .collect()
}
