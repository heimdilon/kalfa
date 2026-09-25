//! Araç zinciri uyumluluğu (aşama 2 tasarımı §12.3): `git`, `cargo`, `npm` ve
//! `python` yalıtımın içinde, kabuk aracının bütün yolundan (gizli dosya
//! taraması, git yerleşimi, bwrap) geçerek çalışır. Kullanıcının gerçek ev
//! dizinindeki araç zincirini kullanır; yalıtım yoksa atlanmaz, başarısız olur:
//!
//! ```text
//! cargo test -p kalfa-tools --features yalitim-testleri --test uyumluluk
//! ```
#![cfg(all(target_os = "linux", feature = "yalitim-testleri"))]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use kalfa_core::arac::{Arac, AracHatasi, CiktiGonderici};
use kalfa_core::izin::Siniflandirici;
use kalfa_sandbox::linux::{BwrapAlgilayici, LinuxAyari, LinuxYalitici};
use kalfa_sandbox::{Yalitici, YalitimDurumu};
use kalfa_tools::{CalismaAlani, KabukAraci, KabukGuveni};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct Guven(AtomicBool);

impl KabukGuveni for Guven {
    fn guvenildi_mi(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
    fn guvenildi(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// Testlerin `node`'u sistem dizinlerinde değilse (örneğin /opt/node) yalıtıma
/// açılır; kullanıcı bunu Ayarlar'dan yapar.
fn node_koku() -> Option<PathBuf> {
    let cikti = Command::new("sh")
        .args(["-c", "command -v node"])
        .output()
        .ok()?;
    let node = PathBuf::from(String::from_utf8_lossy(&cikti.stdout).trim())
        .canonicalize()
        .ok()?;
    let kok = node.parent()?.parent()?.to_path_buf();
    (!kok.starts_with("/usr")).then_some(kok)
}

struct Kurgu {
    _veri: TempDir,
    arac: KabukAraci,
}

fn kurgu(kok: &Path) -> Kurgu {
    let veri = TempDir::new().unwrap();
    let yalitici = LinuxYalitici::yeni(
        LinuxAyari {
            ev: std::env::var_os("HOME").map(PathBuf::from).expect("HOME"),
            yalitim_dizini: veri.path().join("yalitim"),
            ek_okunur: node_koku().into_iter().collect(),
            git: Default::default(),
        },
        Box::new(BwrapAlgilayici),
        Box::new(|_| {}),
    );
    match yalitici.durum() {
        YalitimDurumu::Hazir { .. } => {}
        d => panic!("yalıtım hazır değil: {d:?}"),
    }
    let s = Arc::new(Siniflandirici::yeni(kok, &[]).unwrap());
    let alan = CalismaAlani::ac(s).unwrap();
    Kurgu {
        _veri: veri,
        arac: KabukAraci::yeni(alan, Arc::new(yalitici), Arc::new(Guven::default())),
    }
}

impl Kurgu {
    /// Komutu çalıştırır; başarılıysa çıktıyı, değilse hatayı panikle verir.
    async fn calistir(&self, a: Value) -> String {
        let h = self.arac.hazirla(&a).await.expect("hazırlık");
        match self
            .arac
            .calistir_akisli(&a, &h, CancellationToken::new(), CiktiGonderici::bos())
            .await
        {
            Ok(c) => c.icerik,
            Err(AracHatasi::Basarisiz(c)) => panic!("{a}\n{}", c.icerik),
            Err(e) => panic!("{a}\n{e:?}"),
        }
    }
}

fn betik(metin: &str) -> Value {
    json!({ "betik": metin, "zaman_asimi_sn": 600 })
}

fn git_host(dizin: &Path, args: &[&str]) {
    let durum = Command::new("git")
        .arg("-C")
        .arg(dizin)
        .args([
            "-c",
            "user.name=Kalfa",
            "-c",
            "user.email=kalfa@example.com",
        ])
        .args(args)
        .status()
        .unwrap();
    assert!(durum.success(), "git {args:?}");
}

/// Örnek projeler derlemeye gömülür: testi koşan kullanıcı kaynak ağacını
/// okuyamayabilir.
const RUST_ORNEGI: &[(&str, &str)] = &[
    ("Cargo.toml", include_str!("ornekler/rust/Cargo.toml")),
    ("Cargo.lock", include_str!("ornekler/rust/Cargo.lock")),
    ("src/lib.rs", include_str!("ornekler/rust/src/lib.rs")),
];
const NODE_ORNEGI: &[(&str, &str)] = &[
    ("package.json", include_str!("ornekler/node/package.json")),
    (
        "package-lock.json",
        include_str!("ornekler/node/package-lock.json"),
    ),
];

fn ornek_yaz(dosyalar: &[(&str, &str)], hedef: &Path) {
    for (ad, icerik) in dosyalar {
        let yol = hedef.join(ad);
        std::fs::create_dir_all(yol.parent().unwrap()).unwrap();
        std::fs::write(yol, icerik).unwrap();
    }
}

#[tokio::test]
async fn git_worktreede_durum_fark_ekleme_ve_commit() {
    let d = TempDir::new().unwrap();
    let ana = d.path().join("ana");
    std::fs::create_dir_all(&ana).unwrap();
    git_host(&ana, &["init", "-q", "-b", "ana"]);
    std::fs::write(ana.join("benioku.md"), "ilk\n").unwrap();
    git_host(&ana, &["add", "."]);
    git_host(&ana, &["commit", "-q", "-m", "ilk"]);
    git_host(&ana, &["worktree", "add", "-q", "-b", "dal", "../dal"]);
    let dal = d.path().join("dal");

    let k = kurgu(&dal);
    let cikti = k
        .calistir(betik(
            "echo ikinci >> benioku.md && git status --short && git diff --stat && \
             git add benioku.md && \
             git -c user.name=Kalfa -c user.email=kalfa@example.com commit -q -m yalitimdan && \
             git log --oneline -1",
        ))
        .await;
    assert!(cikti.contains("benioku.md"), "{cikti}");
    assert!(cikti.contains("yalitimdan"), "{cikti}");
    // Commit ana deponun nesnelerine yazıldı.
    let log = Command::new("git")
        .args(["-C"])
        .arg(&ana)
        .args(["log", "--oneline", "dal", "-1"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&log.stdout).contains("yalitimdan"));
}

#[tokio::test]
async fn cargo_agsiz_onceden_indirilmis_crate_ile_derler_ve_test_eder() {
    let d = TempDir::new().unwrap();
    ornek_yaz(RUST_ORNEGI, d.path());
    let k = kurgu(d.path());
    let cikti = k
        .calistir(json!({
            "program": "cargo",
            "argumanlar": ["test", "--offline", "--locked"],
            "zaman_asimi_sn": 600
        }))
        .await;
    assert!(cikti.contains("test result: ok. 1 passed"), "{cikti}");
}

#[tokio::test]
async fn npm_ci_agli_ve_npm_test_agsiz_calisir() {
    let d = TempDir::new().unwrap();
    ornek_yaz(NODE_ORNEGI, d.path());
    let k = kurgu(d.path());
    let ci = k
        .calistir(json!({
            "program": "npm",
            "argumanlar": ["ci", "--no-audit", "--no-fund"],
            "ag": true,
            "zaman_asimi_sn": 600
        }))
        .await;
    assert!(ci.contains("Çıkış kodu: 0"), "{ci}");
    let test = k
        .calistir(json!({ "program": "npm", "argumanlar": ["test"] }))
        .await;
    assert!(test.contains("npm-test-ok"), "{test}");
}

#[tokio::test]
async fn python_calisir() {
    let d = TempDir::new().unwrap();
    let k = kurgu(d.path());
    let cikti = k
        .calistir(json!({
            "program": "python3",
            "argumanlar": ["-c", "import json, sqlite3, ssl; print('py-ok')"]
        }))
        .await;
    assert!(cikti.contains("py-ok"), "{cikti}");
}
