//! Araç zinciri uyumluluğu (aşama 2 tasarımı §12.3): `git`, `cargo`, `npm` ve
//! `python` yalıtımın içinde, kabuk aracının bütün yolundan (gizli dosya
//! taraması, git yerleşimi ve platform yalıtımı) geçerek çalışır. Kullanıcının
//! gerçek ev dizinindeki araç zincirini kullanır; yalıtım yoksa atlanmaz,
//! başarısız olur:
//!
//! ```text
//! cargo test -p kalfa-tools --features yalitim-testleri --test uyumluluk -- --show-output
//! ```
//!
//! Windows'ta A11, A12 ve A15'in ölçüm çıktıları `--show-output` ile görünür.
//! `node` ve `python` sistem dizinlerinde değilse ya da DACL'leri uygulama
//! paketlerine kapalıysa önce yönetici olarak `kalfa-yalitim izin-ver <dizin>`
//! çalıştırılır (§9.4).
#![cfg(all(any(target_os = "linux", windows), feature = "yalitim-testleri"))]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
use std::sync::{Mutex, MutexGuard};

use kalfa_core::arac::{Arac, AracHatasi, CiktiGonderici};
use kalfa_core::izin::Siniflandirici;
#[cfg(windows)]
use kalfa_sandbox::GitKimligi;
#[cfg(target_os = "linux")]
use kalfa_sandbox::linux::{BwrapAlgilayici, LinuxAyari, LinuxYalitici};
#[cfg(windows)]
use kalfa_sandbox::windows::ace_kaydi::{AceKaydi, Girdi, ORTAK};
#[cfg(windows)]
use kalfa_sandbox::windows::yalitici::{WindowsAyari, WindowsYalitici};
#[cfg(windows)]
use kalfa_sandbox::{AlanBilgisi, Tarama, alan_kimligi};
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
#[cfg(target_os = "linux")]
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
    #[cfg(target_os = "linux")]
    _veri: TempDir,
    arac: KabukAraci,
    #[cfg(windows)]
    ilk_cagri: AtomicBool,
    #[cfg(windows)]
    _temizlik: WindowsTemizlik,
}

#[cfg(windows)]
static WINDOWS_KURGU_KILIDI: Mutex<()> = Mutex::new(());

/// Gerçek kullanıcı araç zincirine eklenen ACE'leri günlük doğrulanana kadar
/// geri alır; testler arası ortak kilit paralel DACL yarışını önler.
#[cfg(windows)]
struct WindowsTemizlik {
    veri: Option<TempDir>,
    kok: PathBuf,
    yalitici: Option<Arc<WindowsYalitici>>,
    _kilit: MutexGuard<'static, ()>,
}

#[cfg(windows)]
impl Drop for WindowsTemizlik {
    fn drop(&mut self) {
        let Some(veri) = self.veri.take() else {
            return;
        };
        let yalitim_dizini = veri.path().join("yalitim");
        let ace_yolu = yalitim_dizini.join("ace-kaydi.jsonl");
        let kimlik = alan_kimligi(&self.kok);
        let ace = AceKaydi::oku(&ace_yolu);
        let mut temiz = !ace.bozuk_mu();
        if temiz && ace.girdiler().iter().any(|g| g.alan() == kimlik.as_str()) {
            if let Some(yalitici) = &self.yalitici {
                let alan = AlanBilgisi {
                    kok: self.kok.clone(),
                    gizli: Tarama::default(),
                    korumali: Vec::new(),
                    git_dizinleri: Vec::new(),
                };
                if let Err(hata) = yalitici.alani_kaldir(&alan) {
                    eprintln!("Windows uyumluluk testi alanı geri alınamadı: {hata}");
                    temiz = false;
                }
            } else {
                eprintln!("Windows uyumluluk testi alanı için yalitici oluşmadı");
                temiz = false;
            }
        }
        let ortak_izin_var = ace
            .girdiler()
            .iter()
            .any(|g| matches!(g, Girdi::Izin { alan, .. } if alan == ORTAK));
        if !ace.bozuk_mu() && ortak_izin_var {
            let bos_ev = veri.path().join("bos-ev");
            drop(WindowsYalitici::yeni(
                WindowsAyari {
                    ev: bos_ev,
                    yalitim_dizini: yalitim_dizini.clone(),
                    ek_okunur: Vec::new(),
                    git: GitKimligi::default(),
                },
                Box::new(|_| {}),
            ));
        }
        let son = AceKaydi::oku(&ace_yolu);
        temiz &= !son.bozuk_mu() && son.girdiler().is_empty();
        if temiz {
            drop(veri);
        } else {
            let yol = veri.keep();
            eprintln!(
                "Windows uyumluluk testi geri alma kaydı korundu: {}",
                yol.display()
            );
        }
    }
}

#[cfg(target_os = "linux")]
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

/// Windows'ta `node` ve `python`'un dizinleri yalıtıma ek okunur olarak
/// açılır; kullanıcı bunu Ayarlar'dan yapar. WindowsApps'teki `python.exe`
/// Mağaza'nın takma adıdır, yorumlayıcı değildir. Dizinin DACL'i uygulama
/// paketlerine kapalıysa (Node ve python.org kurulumları böyledir) hazırlık
/// onu açamaz: önce yönetici olarak `kalfa-yalitim izin-ver <dizin>`.
#[cfg(windows)]
fn windows_arac_dizinleri() -> Vec<PathBuf> {
    ["node", "python"]
        .iter()
        .filter_map(|ad| {
            let cikti = Command::new("where.exe").arg(ad).output().ok()?;
            String::from_utf8_lossy(&cikti.stdout)
                .lines()
                .map(|satir| PathBuf::from(satir.trim()))
                .find(|yol| {
                    !yol.components().any(|c| {
                        c.as_os_str()
                            .to_string_lossy()
                            .eq_ignore_ascii_case("WindowsApps")
                    })
                })
                .and_then(|yol| yol.parent().map(Path::to_path_buf))
        })
        .collect()
}

#[cfg(windows)]
fn kurgu(kok: &Path) -> Kurgu {
    let kilit = WINDOWS_KURGU_KILIDI
        .lock()
        .unwrap_or_else(|zehir| zehir.into_inner());
    let veri = TempDir::new().unwrap();
    let ev = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .expect("USERPROFILE");
    let yalitim_dizini = veri.path().join("yalitim");
    let mut temizlik = WindowsTemizlik {
        veri: Some(veri),
        kok: kok.to_path_buf(),
        yalitici: None,
        _kilit: kilit,
    };
    let yalitici = Arc::new(WindowsYalitici::yeni(
        WindowsAyari {
            ev,
            yalitim_dizini,
            ek_okunur: windows_arac_dizinleri(),
            git: GitKimligi::default(),
        },
        Box::new(|_| {}),
    ));
    temizlik.yalitici = Some(yalitici.clone());
    match yalitici.durum() {
        YalitimDurumu::Hazir { .. } => {}
        d => panic!("Windows yalıtımı hazır değil: {d:?}"),
    }
    let s = Arc::new(Siniflandirici::yeni(kok, &[]).unwrap());
    let alan = CalismaAlani::ac(s).unwrap();
    Kurgu {
        arac: KabukAraci::yeni(alan, yalitici.clone(), Arc::new(Guven::default())),
        ilk_cagri: AtomicBool::new(true),
        _temizlik: temizlik,
    }
}

impl Kurgu {
    /// Komutu çalıştırır; başarılıysa çıktıyı, değilse hatayı panikle verir.
    async fn calistir(&self, a: Value) -> String {
        match self.dene(&a).await {
            Ok(icerik) => icerik,
            Err(icerik) => panic!("{a}\n{icerik}"),
        }
    }

    /// Komutu çalıştırır; sıfırdan farklı çıkış kodunda çıktıyı `Err` verir.
    async fn dene(&self, a: &Value) -> Result<String, String> {
        let h = self.arac.hazirla(a).await.expect("hazırlık");
        #[cfg(windows)]
        if self.ilk_cagri.swap(false, Ordering::SeqCst) {
            assert!(
                h.kabuk.as_ref().is_some_and(|t| t.alan_hazirligi),
                "ilk Windows çağrısında çalışma alanı hazırlığı yerel onay tarifine eklenmeli"
            );
        }
        match self
            .arac
            .calistir_akisli(a, &h, CancellationToken::new(), CiktiGonderici::bos())
            .await
        {
            Ok(c) => Ok(c.icerik),
            Err(AracHatasi::Basarisiz(c)) => Err(c.icerik),
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

fn git_cikti(dizin: &Path, args: &[&str]) -> String {
    let cikti = Command::new("git")
        .arg("-C")
        .arg(dizin)
        .args(args)
        .output()
        .unwrap();
    assert!(
        cikti.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&cikti.stderr)
    );
    String::from_utf8_lossy(&cikti.stdout).into_owned()
}

fn etiket_kodu(cikti: &str, etiket: &str) -> i32 {
    cikti
        .lines()
        .find_map(|satir| satir.trim().strip_prefix(etiket))
        .and_then(|kod| kod.parse::<i32>().ok())
        .unwrap_or_else(|| panic!("{etiket} çıkış kodu çıktıda yok:\n{cikti}"))
}

/// Örnek projeler derlemeye gömülür: testi koşan kullanıcı kaynak ağacını
/// okuyamayabilir.
const RUST_ORNEGI: &[(&str, &str)] = &[
    ("Cargo.toml", include_str!("ornekler/rust/Cargo.toml")),
    ("Cargo.lock", include_str!("ornekler/rust/Cargo.lock")),
    ("src/lib.rs", include_str!("ornekler/rust/src/lib.rs")),
];

/// Windows'ta A12 örneği: dış crate yok, C derleyen `build.rs` var.
#[cfg(windows)]
const WINDOWS_RUST_ORNEGI: &[(&str, &str)] = &[
    (
        "Cargo.toml",
        "[package]\nname = \"kalfa-ornek\"\nversion = \"0.1.0\"\nedition = \"2024\"\nbuild = \"build.rs\"\npublish = false\n\n[workspace]\n",
    ),
    (
        "Cargo.lock",
        "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n\n[[package]]\nname = \"kalfa-ornek\"\nversion = \"0.1.0\"\n",
    ),
    ("build.rs", RUST_BUILD_RS),
    ("src/native.c", RUST_NATIVE_C),
    (
        "src/lib.rs",
        "#[cfg(test)]\nmod tests {\n    #[test]\n    fn ornek_crate_calisir() { assert_eq!(2 + 2, 4); }\n}\n",
    ),
];

/// Windows'ta A12: build.rs, MSVC'nin `cl.exe` ve `INCLUDE` ortamıyla
/// standart C başlığını bulup küçük bir nesne dosyası derler.
#[cfg(windows)]
const RUST_BUILD_RS: &str = r#"
use std::path::PathBuf;
use std::process::Command;

fn main() {
    if !cfg!(windows) {
        return;
    }
    let kaynak = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
        .join("src/native.c");
    let nesne = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR")).join("native.obj");
    let durum = Command::new("cl.exe")
        .args(["/nologo", "/c"])
        .arg(kaynak)
        .arg(format!("/Fo{}", nesne.display()))
        .status()
        .expect("cl.exe başlatılamadı");
    assert!(durum.success(), "cl.exe C örneğini derleyemedi");
    println!("cargo:warning=A12 cl.exe ve INCLUDE ile C örneği derlendi");
}
"#;

#[cfg(windows)]
const RUST_NATIVE_C: &str =
    "#include <stdlib.h>\nint kalfa_ornek_deger(void) { return EXIT_SUCCESS + 42; }\n";
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

/// `.git` ve worktree'nin ortak git dizini yalıtımda salt okunurdur
/// (aşama 2 tasarımı §7.4): okuyan komutlar çalışır, yazanlar çalışmaz.
#[tokio::test]
async fn git_worktreede_durum_fark_log_calisir_ekleme_ve_commit_calismaz() {
    let d = TempDir::new().unwrap();
    let ana = d.path().join("ana");
    std::fs::create_dir_all(&ana).unwrap();
    git_host(&ana, &["init", "-q", "-b", "ana"]);
    std::fs::write(ana.join("benioku.md"), "ilk\n").unwrap();
    git_host(&ana, &["add", "."]);
    git_host(&ana, &["commit", "-q", "-m", "ilk"]);
    git_host(&ana, &["config", "user.name", "Kalfa"]);
    git_host(&ana, &["config", "user.email", "kalfa@example.com"]);
    git_host(&ana, &["worktree", "add", "-q", "-b", "dal", "../dal"]);
    let dal = d.path().join("dal");
    std::fs::write(dal.join("bekleyen.md"), "ana depoda bekleyen değişiklik\n").unwrap();
    git_host(&dal, &["add", "bekleyen.md"]);
    let onceki_kayit = git_cikti(&dal, &["rev-parse", "HEAD"]);

    let k = kurgu(&dal);
    #[cfg(target_os = "linux")]
    let komut = "echo ikinci >> benioku.md; \
                 git status --short; echo \"durum=$?\"; \
                 git diff --stat; echo \"fark=$?\"; \
                 git log --oneline -1; echo \"gunluk=$?\"; \
                 git add benioku.md; echo \"ekle=$?\"; \
                 git commit -m 'yalıtım testi'; echo \"commit=$?\"";
    #[cfg(windows)]
    let komut = "$ErrorActionPreference='Stop'; \
                 if (Get-Variable PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) { $PSNativeCommandUseErrorActionPreference=$false }; \
                 Add-Content -LiteralPath 'benioku.md' -Value 'ikinci'; \
                 git status --short; $durum=$LASTEXITCODE; Write-Output \"durum=$durum\"; \
                 git diff --stat; $fark=$LASTEXITCODE; Write-Output \"fark=$fark\"; \
                 git log --oneline -1; $gunluk=$LASTEXITCODE; Write-Output \"gunluk=$gunluk\"; \
                 git add benioku.md; $ekle=$LASTEXITCODE; Write-Output \"ekle=$ekle\"; \
                 git commit -m 'yalıtım testi'; $commit=$LASTEXITCODE; Write-Output \"commit=$commit\"";
    let cikti = k.calistir(betik(komut)).await;
    assert!(cikti.contains("M benioku.md"), "{cikti}");
    assert!(cikti.contains("ilk"), "{cikti}");
    assert!(
        cikti.contains("benioku.md | 1 +"),
        "diff --stat değişikliği göstermedi:\n{cikti}"
    );
    for etiket in ["durum=", "fark=", "gunluk="] {
        assert_eq!(
            etiket_kodu(&cikti, etiket),
            0,
            "{etiket} okuma komutu başarısız:\n{cikti}"
        );
    }
    for etiket in ["ekle=", "commit="] {
        let kod = etiket_kodu(&cikti, etiket);
        assert_ne!(kod, 0, "{etiket} beklenmedik biçimde başarılı:\n{cikti}");
    }
    // Worktree'nin ayrı indeksi ve ortak HEAD'i yalıtımlı komutça değişmedi.
    assert_eq!(git_cikti(&dal, &["rev-parse", "HEAD"]), onceki_kayit);
    assert_eq!(
        git_cikti(&dal, &["diff", "--cached", "--name-only"]).trim(),
        "bekleyen.md"
    );
}

/// A11: PowerShell çalışma alanında başlar; alanın üst dizinleri sürücü
/// köküne kadar listelenebilir (Git for Windows ve `Set-Location` bunu
/// ister). Her düzeyin sonucu çıktıya yazılır.
#[cfg(windows)]
#[tokio::test]
async fn ust_dizin_listeleme_sinirini_olcer() {
    use kalfa_sandbox::windows::guvenlik::sade;

    let d = TempDir::new().unwrap();
    let k = kurgu(d.path());
    let alan = sade(&d.path().canonicalize().unwrap());
    let beklenen_kok = alan.ancestors().last().expect("sürücü kökü");
    let kok_yolu = beklenen_kok.display().to_string();
    let komut = r#"
$ErrorActionPreference='Stop'
$yol=(Get-Location).Path
Write-Output "A11_KONUM=$yol"
$kok=[System.IO.Path]::GetPathRoot($yol)
$yol=Split-Path -Parent $yol
while ($yol) {
  try {
    Get-ChildItem -LiteralPath $yol -Force -ErrorAction Stop | Select-Object -First 1 | Out-Null
    Write-Output "A11_LISTE_OK=$yol"
  } catch {
    Write-Output "A11_LISTE_REDDI=$yol"
  }
  if ([System.IO.Path]::GetFullPath($yol).TrimEnd('\') -ieq $kok.TrimEnd('\')) {
    Write-Output "A11_SURUCU_KOKU_DENENDI=$kok"
    break
  }
  $ust=Split-Path -Parent $yol
  if (-not $ust -or $ust -eq $yol) { break }
  $yol=$ust
}
"#;
    let cikti = k.calistir(betik(komut)).await;
    let kok_satiri = format!("A11_SURUCU_KOKU_DENENDI={kok_yolu}");
    assert!(
        cikti
            .lines()
            .any(|satir| satir.trim().eq_ignore_ascii_case(&kok_satiri)),
        "sürücü köküne ulaşıldığı doğrulanamadı ({kok_satiri}):\n{cikti}"
    );
    let satir_var = |beklenen: &str| {
        cikti
            .lines()
            .any(|satir| satir.trim().eq_ignore_ascii_case(beklenen))
    };
    assert!(
        satir_var(&format!("A11_KONUM={}", alan.display())),
        "PowerShell çalışma alanında başlamadı:\n{cikti}"
    );
    for ust in alan.ancestors().skip(1) {
        let ust = ust.display().to_string();
        assert!(
            satir_var(&format!("A11_LISTE_OK={ust}")),
            "{ust} listelenemedi:\n{cikti}"
        );
    }
    assert!(!cikti.contains("A11_LISTE_REDDI="), "{cikti}");
    eprintln!("A11 üst dizin listeleme ölçümü:\n{cikti}");
}

#[cfg(target_os = "linux")]
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

/// A15 (bilinen sınır, aşama 2 tasarımı §14): Windows'ta registry
/// bağlanamaz; çalışma alanına özel `CARGO_HOME` boş başlar ve kapta
/// doldurulamaz. cargo paketi açarken hedefi kanonikleştirir; bu,
/// `GetFinalPathNameByHandleW`'nin DOS yol adını ister, o da Mount Manager'a
/// erişim ister ve AppContainer'da "Erişim engellendi" verir. Sınır kalkarsa
/// bu test düşer; §14 ve §7.4 güncellenir.
#[cfg(windows)]
#[tokio::test]
async fn cargo_ozel_registry_kapta_doldurulamaz() {
    let d = TempDir::new().unwrap();
    ornek_yaz(RUST_ORNEGI, d.path());
    let k = kurgu(d.path());
    let bos = k
        .dene(&json!({
            "program": "cargo",
            "argumanlar": ["build", "--offline", "--locked"],
            "ag": false,
            "zaman_asimi_sn": 600
        }))
        .await
        .expect_err("boş registry'yle ağsız derleme başarılı olmamalı");
    let fetch = k
        .dene(&json!({
            "program": "cargo",
            "argumanlar": ["fetch", "--locked"],
            "ag": true,
            "zaman_asimi_sn": 600
        }))
        .await
        .expect_err("A15 sınırı kalktı: §14 ve §7.4'ü güncelleyin");
    assert!(fetch.contains("Downloaded hex"), "{fetch}");
    assert!(fetch.contains("canonicaliz"), "{fetch}");
    eprintln!("A15 boş registry, ağsız:\n{bos}\nA15 ağlı cargo fetch:\n{fetch}");
}

/// A12: `build.rs` MSVC'nin `cl.exe` ve `INCLUDE` ortamıyla C derler; bağlama
/// da kapta çalışır.
#[cfg(windows)]
#[tokio::test]
async fn cargo_ag_olmadan_msvc_build_rs_ile_derler_ve_test_eder() {
    let d = TempDir::new().unwrap();
    ornek_yaz(WINDOWS_RUST_ORNEGI, d.path());
    let k = kurgu(d.path());
    let derleme = k
        .calistir(json!({
            "program": "cargo",
            "argumanlar": ["build", "--offline", "--locked"],
            "ag": false,
            "zaman_asimi_sn": 600
        }))
        .await;
    assert!(derleme.contains("Çıkış kodu: 0"), "{derleme}");
    eprintln!("A12 cl.exe/INCLUDE ölçüm çıktısı:\n{derleme}");
    let test = k
        .calistir(json!({
            "program": "cargo",
            "argumanlar": ["test", "--offline", "--locked"],
            "ag": false,
            "zaman_asimi_sn": 600
        }))
        .await;
    assert!(test.contains("test result: ok. 1 passed"), "{test}");
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
        .calistir(json!({ "program": "npm", "argumanlar": ["test"], "ag": false }))
        .await;
    assert!(test.contains("npm-test-ok"), "{test}");
}

#[tokio::test]
async fn python_calisir() {
    let d = TempDir::new().unwrap();
    let k = kurgu(d.path());
    let program = if cfg!(windows) { "python" } else { "python3" };
    let cikti = k
        .calistir(json!({
            "program": program,
            "argumanlar": ["-c", "import json, sqlite3, ssl; print('py-ok')"],
            "ag": false
        }))
        .await;
    assert!(cikti.contains("py-ok"), "{cikti}");
}
