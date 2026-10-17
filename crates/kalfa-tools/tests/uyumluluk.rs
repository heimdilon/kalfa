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
use kalfa_core::izin::{GizliAyarlari, Siniflandirici};
#[cfg(windows)]
use kalfa_sandbox::GitKimligi;
#[cfg(target_os = "linux")]
use kalfa_sandbox::linux::{BwrapAlgilayici, LinuxAyari, LinuxYalitici};
use kalfa_sandbox::vekil::Cozucu;
#[cfg(windows)]
use kalfa_sandbox::windows::ace_kaydi::{AceKaydi, Girdi, ORTAK};
#[cfg(windows)]
use kalfa_sandbox::windows::yalitici::{WindowsAyari, WindowsYalitici};
#[cfg(windows)]
use kalfa_sandbox::{AlanBilgisi, OzelDizinler, Tarama, alan_kimligi};
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
                    kalfa_araclari: Vec::new(),
                    git: GitKimligi::default(),
                    ..WindowsAyari::default()
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
    kurgu_agli(kok, None)
}

#[cfg(target_os = "linux")]
fn kurgu_agli(kok: &Path, cozucu: Option<Arc<dyn Cozucu>>) -> Kurgu {
    kurgu_linux(kok, cozucu, Arc::default())
}

/// Kullanıcının gizli ayarlarıyla (3e, Görev 3).
#[cfg(target_os = "linux")]
fn kurgu_ayarli(kok: &Path, ayarlar: Arc<GizliAyarlari>) -> Kurgu {
    kurgu_linux(kok, None, ayarlar)
}

#[cfg(target_os = "linux")]
fn kurgu_linux(kok: &Path, cozucu: Option<Arc<dyn Cozucu>>, ayarlar: Arc<GizliAyarlari>) -> Kurgu {
    let veri = TempDir::new().unwrap();
    let yalitici = LinuxYalitici::yeni(
        LinuxAyari {
            ev: std::env::var_os("HOME").map(PathBuf::from).expect("HOME"),
            yalitim_dizini: veri.path().join("yalitim"),
            ek_okunur: node_koku().into_iter().collect(),
            kopru: kopru_ikilisi(),
            git: Default::default(),
            vekil_cozucu: cozucu,
            vekil_yerel_izin: true,
        },
        Box::new(BwrapAlgilayici),
        Box::new(|_| {}),
    );
    match yalitici.durum() {
        YalitimDurumu::Hazir { .. } => {}
        d => panic!("yalıtım hazır değil: {d:?}"),
    }
    let s = Arc::new(Siniflandirici::yeni_ayarla(kok, ayarlar).unwrap());
    let alan = CalismaAlani::ac(s).unwrap();
    Kurgu {
        _veri: veri,
        arac: KabukAraci::yeni(
            alan,
            Arc::new(yalitici),
            Arc::new(Guven::default()),
            Arc::new(AtomicBool::new(false)),
        ),
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
    kurgu_ile(kok, windows_arac_dizinleri(), Vec::new())
}

fn kopru_ikilisi() -> Option<PathBuf> {
    if let Ok(yol) = std::env::var("KALFA_KOPRU_YOLU") {
        let p = PathBuf::from(yol);
        if p.is_file() {
            return Some(p);
        }
    }
    let ad = format!("kalfa-kopru{}", std::env::consts::EXE_SUFFIX);
    if let Some(parent) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        let p = parent.join(&ad);
        if p.is_file() {
            return Some(p);
        }
        if let Some(grandparent) = parent.parent() {
            let p2 = grandparent.join(&ad);
            if p2.is_file() {
                return Some(p2);
            }
        }
    }
    if let Ok(hedef_dizin) = std::env::var("CARGO_TARGET_DIR") {
        let p = PathBuf::from(hedef_dizin).join("debug").join(&ad);
        if p.is_file() {
            return Some(p);
        }
    }
    let hedef = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("target").join("debug").join(&ad));
    if let Some(p) = hedef.filter(|p| p.is_file()) {
        return Some(p);
    }
    None
}

/// `ek_okunur`: kullanıcının Ayarlar'dan açtığı dizinler; `kalfa_araclari`:
/// kurulu Kalfa'nın getirdiği Python ve Node (2d-3).
#[cfg(windows)]
fn kurgu_ile(kok: &Path, ek_okunur: Vec<PathBuf>, kalfa_araclari: Vec<PathBuf>) -> Kurgu {
    kurgu_ile_ayarla(kok, ek_okunur, kalfa_araclari, None, false, Arc::default())
}

#[cfg(windows)]
fn kurgu_agli(kok: &Path, cozucu: Option<Arc<dyn Cozucu>>) -> Kurgu {
    kurgu_ile_ayarla(
        kok,
        windows_arac_dizinleri(),
        Vec::new(),
        cozucu,
        true,
        Arc::default(),
    )
}

/// Kullanıcının gizli ayarlarıyla (3e, Görev 3).
#[cfg(windows)]
fn kurgu_ayarli(kok: &Path, ayarlar: Arc<GizliAyarlari>) -> Kurgu {
    kurgu_ile_ayarla(
        kok,
        windows_arac_dizinleri(),
        Vec::new(),
        None,
        false,
        ayarlar,
    )
}

#[cfg(windows)]
fn kurgu_ile_ayarla(
    kok: &Path,
    ek_okunur: Vec<PathBuf>,
    kalfa_araclari: Vec<PathBuf>,
    cozucu: Option<Arc<dyn kalfa_sandbox::vekil::Cozucu>>,
    yerel_izin: bool,
    ayarlar: Arc<GizliAyarlari>,
) -> Kurgu {
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
            ek_okunur,
            kalfa_araclari,
            git: GitKimligi::default(),
            kopru: kopru_ikilisi(),
            vekil_cozucu: cozucu,
            vekil_yerel_izin: yerel_izin,
        },
        Box::new(|_| {}),
    ));
    temizlik.yalitici = Some(yalitici.clone());
    match yalitici.durum() {
        YalitimDurumu::Hazir { .. } => {}
        d => panic!("Windows yalıtımı hazır değil: {d:?}"),
    }
    let s = Arc::new(Siniflandirici::yeni_ayarla(kok, ayarlar).unwrap());
    let alan = CalismaAlani::ac(s).unwrap();
    Kurgu {
        arac: KabukAraci::yeni(
            alan,
            yalitici.clone(),
            Arc::new(Guven::default()),
            Arc::new(AtomicBool::new(false)),
        ),
        ilk_cagri: AtomicBool::new(true),
        _temizlik: temizlik,
    }
}

impl Kurgu {
    /// Kalfa'nın veri dizini gibi kullanılan geçici dizin: yalıtımın özel kökü
    /// (`yalitim`) bunun altındadır.
    fn veri(&self) -> &Path {
        #[cfg(target_os = "linux")]
        return self._veri.path();
        #[cfg(windows)]
        return self
            ._temizlik
            .veri
            .as_ref()
            .expect("veri dizini temizlikten önce var")
            .path();
    }

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

    /// Ağlı komutu çalıştırır; başarılı ya da başarısız çıktı ile ağ özetini döner.
    async fn calistir_agli(
        &self,
        komut: &str,
        ag_hostlari: &[&str],
    ) -> (
        Result<String, String>,
        Option<Box<kalfa_core::tur_ozeti::AgKomutOzeti>>,
    ) {
        let a = json!({
            "betik": komut,
            "ag": true,
            "ag_hostlari": ag_hostlari,
            "zaman_asimi_sn": 60,
        });
        let h = self.arac.hazirla(&a).await.expect("hazırlık");
        #[cfg(windows)]
        if self.ilk_cagri.swap(false, Ordering::SeqCst) {
            assert!(
                h.kabuk.as_ref().is_some_and(|t| t.alan_hazirligi),
                "ilk Windows çağrısında çalışma alanı hazırlığı yerel onay tarifine eklenmeli"
            );
        }
        match self
            .arac
            .calistir_akisli(&a, &h, CancellationToken::new(), CiktiGonderici::bos())
            .await
        {
            Ok(c) => (Ok(c.icerik), c.ag),
            Err(AracHatasi::Basarisiz(c)) => (Err(c.icerik), c.ag),
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

/// Ağsız cargo, geliştiricinin önceden indirdiği crate'lerle derler. Windows'ta
/// bu A15'in gelişmiş yoludur (2d-3): `CARGO_HOME` geliştiricinin salt okunur
/// `.cargo`'sudur.
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

/// A15 regresyonu: ağlı çağrının özel `CARGO_HOME`'u, yalnızca geliştiricinin
/// önceden önbelleğe aldığı arşiv ve crates.io indeks girdisiyle ağsız
/// doldurulup paketi gerçekten açabilmeli. Kaynak ağacını kopyalamıyoruz.
#[cfg(windows)]
#[tokio::test]
async fn cargo_ozel_registry_onbellekten_agdisi_paket_acar() {
    let d = TempDir::new().unwrap();
    std::fs::create_dir_all(d.path().join("src")).unwrap();
    std::fs::write(
        d.path().join("Cargo.toml"),
        "[package]\nname = \"a15-offline-regression\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nmemchr = \"=2.8.3\"\n",
    )
    .unwrap();
    std::fs::write(d.path().join("src/lib.rs"), "").unwrap();

    // Herhangi bir hata durumunda dış ağa çıkılmaması için boş çözücülü sahte
    // vekil kullanılır. İki hedef, yalnızca olası crates.io CONNECT girişimini
    // ölçmek için aynı mevcut Cargo uçları olarak izin listesinde kalır.
    let cozucu = Arc::new(SahteCozucu {
        eslemeler: std::collections::HashMap::new(),
    });
    let k = kurgu_agli(d.path(), Some(cozucu));

    // Mevcut test hazırlığı ilk çağrıda alan DACL'lerini ve özel önbellek
    // kökünü kurar. Arşiv ve indeks bundan sonra, test sürecinin yetkisiyle
    // özel CARGO_HOME'a kopyalanır.
    let ilk = k
        .dene(&json!({
            "program": "cmd",
            "argumanlar": ["/C", "exit 0"],
            "zaman_asimi_sn": 30
        }))
        .await
        .unwrap_or_else(|e| panic!("özel Cargo önbelleği hazırlığı başarısız: {e}"));
    assert!(ilk.contains("Çıkış kodu: 0"), "{ilk}");

    let cargo_home = OzelDizinler::yol(&k.veri().join("yalitim"), &alan_kimligi(d.path())).cargo();
    assert!(
        cargo_home.is_dir(),
        "özel CARGO_HOME hazırlanmalı: {cargo_home:?}"
    );
    let kaynak_dizin = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .into_iter()
        .chain(
            std::env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .map(|ev| ev.join(".cargo")),
        )
        .find_map(|host_cargo| {
            let registry = host_cargo.join("registry");
            let entries = std::fs::read_dir(registry.join("cache")).ok()?;
            entries.filter_map(Result::ok).find_map(|entry| {
                let kimlik = entry.file_name().to_string_lossy().into_owned();
                if !kimlik.starts_with("index.crates.io-") {
                    return None;
                }
                let indeks_koku = registry.join("index").join(&kimlik);
                let arsiv = entry.path().join("memchr-2.8.3.crate");
                let indeks = indeks_koku.join(".cache/me/mc/memchr");
                let ayar = indeks_koku.join("config.json");
                (arsiv.is_file() && indeks.is_file() && ayar.is_file())
                    .then_some((kimlik, arsiv, indeks, ayar))
            })
        })
        .expect("host Cargo önbelleğinde memchr 2.8.3 arşivi ve crates.io indeksi olmalı");
    let (registry_kimligi, arsiv, indeks, indeks_ayari) = kaynak_dizin;

    let kaynak = cargo_home
        .join("registry")
        .join("src")
        .join(&registry_kimligi)
        .join("memchr-2.8.3");
    assert!(
        !kaynak.exists(),
        "paket önceden açılmış olmamalı: {kaynak:?}"
    );

    let registry = cargo_home.join("registry");
    let arsiv_dizini = registry.join("cache").join(&registry_kimligi);
    let indeks_dizini = registry.join("index").join(&registry_kimligi);
    std::fs::create_dir_all(&arsiv_dizini).unwrap();
    std::fs::create_dir_all(indeks_dizini.join(".cache/me/mc")).unwrap();
    std::fs::copy(arsiv, arsiv_dizini.join("memchr-2.8.3.crate")).unwrap();
    std::fs::copy(indeks, indeks_dizini.join(".cache/me/mc/memchr")).unwrap();
    std::fs::copy(indeks_ayari, indeks_dizini.join("config.json")).unwrap();

    let (sonuc, ag) = k
        .calistir_agli(
            "cargo fetch --offline",
            &["index.crates.io:443", "static.crates.io:443"],
        )
        .await;
    let cikti = sonuc.unwrap_or_else(|e| panic!("önbellekten çevrimdışı fetch başarısız: {e}"));
    assert!(cikti.contains("Çıkış kodu: 0"), "{cikti}");
    // Bağlantı yokken kabuk ağ özeti üretmez; dolu özet varsa tek bir
    // CONNECT girişimi veya ret bile çevrimdışı koşulu bozar.
    assert!(
        ag.as_ref().is_none_or(|a| a.hostlar.is_empty()),
        "cargo fetch --offline CONNECT üretmemeli: {:?}",
        ag
    );

    let kilit = std::fs::read_to_string(d.path().join("Cargo.lock")).unwrap();
    assert!(
        kilit
            .replace("\r\n", "\n")
            .contains("name = \"memchr\"\nversion = \"2.8.3\""),
        "Cargo.lock tam önbellekteki sürümü çözmeli:\n{kilit}"
    );
    assert!(
        kaynak.join("Cargo.toml").is_file(),
        "Cargo arşivi özel registry/src altına açmalı: {kaynak:?}"
    );
    eprintln!("A15 çevrimdışı özel Cargo fetch çıktısı:\n{cikti}");
}

/// Cargo CONNECT isteği gerçek dış ağa çıkmadan vekile ulaşmalı. Boş sahte
/// DNS bütün çözümlemeleri reddeder; paket veya indeks indirilemez.
#[cfg(windows)]
#[tokio::test]
async fn cargo_connect_istegi_sahte_dns_ile_vekilde_gorulur() {
    let d = TempDir::new().unwrap();
    let k = kurgu_agli(
        d.path(),
        Some(Arc::new(SahteCozucu {
            eslemeler: std::collections::HashMap::new(),
        })),
    );
    let (_, ag) = k
        .calistir_agli(
            "cargo search memchr --limit 1 --config net.retry=0 --config http.timeout=2",
            &["crates.io:443", "index.crates.io:443"],
        )
        .await;
    let ag = ag.expect("Cargo CONNECT girişimi vekilde görülmeli");
    assert!(
        ag.hostlar
            .iter()
            .any(|h| { matches!(h.host.as_str(), "crates.io" | "index.crates.io") }),
        "Cargo isteği kaybolmamalı: {:?}",
        ag.hostlar
    );
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

/// Aynı turda iki komut hazırlanmamış çalışma alanında onaylanır; ikisinin
/// tarifi de hazırlığı içerir. İlki alanı hazırlar. İkincisinin taze tarifinde
/// hazırlık yoktur; çekirdek onu onaylanan tarifle çalıştırır. Gerçek
/// yalıtıcı hazırlanmış alanda hazırlık onayını kabul eder ve yeniden
/// hazırlamaz.
#[cfg(windows)]
#[tokio::test]
async fn ayni_turdaki_ikinci_komut_hazirlanmis_alanda_onaylanan_tarifle_calisir() {
    let d = TempDir::new().unwrap();
    let k = kurgu(d.path());
    k.ilk_cagri.store(false, Ordering::SeqCst);
    let a = betik("Write-Output bir");
    let b = betik("Write-Output iki");
    let ha = k.arac.hazirla(&a).await.expect("hazırlık");
    let hb = k.arac.hazirla(&b).await.expect("hazırlık");
    for h in [&ha, &hb] {
        assert!(h.kabuk.as_ref().is_some_and(|t| t.alan_hazirligi));
    }
    let bir = k
        .arac
        .calistir_akisli(&a, &ha, CancellationToken::new(), CiktiGonderici::bos())
        .await
        .expect("ilk komut");
    assert!(bir.icerik.contains("bir"), "{}", bir.icerik);
    let taze = k.arac.hazirla(&b).await.expect("hazırlık");
    assert!(
        taze.kabuk.as_ref().is_some_and(|t| !t.alan_hazirligi),
        "hazırlık tükenmeli"
    );
    let iki = k
        .arac
        .calistir_akisli(&b, &hb, CancellationToken::new(), CiktiGonderici::bos())
        .await
        .expect("ikinci komut");
    assert!(iki.icerik.contains("iki"), "{}", iki.icerik);
}

/// Bağımlılık dizinlerindeki `.pem` ve `credentials*` kitaplık dosyaları
/// kapta okunur (2d-3, Görev 2b); çalışma alanının kökündeki gizli dosyalar
/// okunamaz.
#[tokio::test]
async fn bagimlilik_dizinindeki_dosyalar_okunur_gizliler_okunamaz() {
    let d = TempDir::new().unwrap();
    let pem = d
        .path()
        .join(".venv/Lib/site-packages/pip/_vendor/certifi/cacert.pem");
    let js = d.path().join("node_modules/aws-sdk/lib/credentials.js");
    for (yol, icerik) in [(&pem, "pem-acik"), (&js, "js-acik")] {
        std::fs::create_dir_all(yol.parent().unwrap()).unwrap();
        std::fs::write(yol, icerik).unwrap();
    }
    std::fs::write(d.path().join(".env"), "env-sir").unwrap();
    std::fs::write(d.path().join("anahtar.pem"), "pem-sir").unwrap();
    let k = kurgu(d.path());
    let oku = |yol: &str| {
        if cfg!(windows) {
            format!(
                "try {{ Get-Content -Raw -ErrorAction Stop -LiteralPath '{yol}' }} catch {{ 'okunamadi' }}"
            )
        } else {
            format!("cat '{yol}' 2>/dev/null || echo okunamadi")
        }
    };
    let betik_metni = [
        ".venv/Lib/site-packages/pip/_vendor/certifi/cacert.pem",
        "node_modules/aws-sdk/lib/credentials.js",
        ".env",
        "anahtar.pem",
    ]
    .map(oku)
    .join("\n");
    let cikti = k.calistir(betik(&betik_metni)).await;
    assert!(cikti.contains("pem-acik"), "{cikti}");
    assert!(cikti.contains("js-acik"), "{cikti}");
    assert!(!cikti.contains("env-sir"), "{cikti}");
    assert!(!cikti.contains("pem-sir"), "{cikti}");
}

/// Kurulu Kalfa'nın getirdiği Python ve Node kapta, kullanıcının
/// bilgisayarındakilerden önce bulunur ve çalışır (2d-3, Görev 4).
/// `KALFA_TEST_ARAC_ZINCIRI`, kurucu derlemesinin `crates/kalfa/araclar`'ının
/// bir kopyasını gösterir: kopyanın üst dizini `C:\`'dir ve `ALL APPLICATION
/// PACKAGES` onu okur (Program Files'ın durumu). Değişken yoksa test atlanır.
#[cfg(windows)]
#[tokio::test]
async fn kalfa_python_ve_node_kapta_calisir() {
    let Some(kok) = std::env::var_os("KALFA_TEST_ARAC_ZINCIRI").map(PathBuf::from) else {
        eprintln!("araç zinciri verilmedi (KALFA_TEST_ARAC_ZINCIRI yok); test atlandı");
        return;
    };
    // Üründe olduğu gibi `\\?\` öneksiz gerçek yol.
    let kok = kalfa_sandbox::windows::guvenlik::sade(
        &kok.canonicalize().expect("KALFA_TEST_ARAC_ZINCIRI"),
    );
    let araclar = kalfa_sandbox::windows::sistem::arac_zinciri_dizinleri(&kok);
    assert!(kok.join(r"python\python.exe").is_file(), "{araclar:?}");
    assert!(kok.join(r"node\node.exe").is_file(), "{araclar:?}");
    let d = TempDir::new().unwrap();
    ornek_yaz(NODE_ORNEGI, d.path());
    std::fs::write(
        d.path().join("test_ornek.py"),
        "def test_toplama():\n    assert 1 + 1 == 2\n",
    )
    .unwrap();
    let k = kurgu_ile(d.path(), Vec::new(), araclar);
    let kok_metni = kok.display().to_string().to_lowercase();

    let yerler = k
        .calistir(betik(
            "where.exe python; where.exe node; \
             python -c \"import os; print('gercek=' + os.path.realpath('.'))\"",
        ))
        .await;
    let ilk = |ad: &str| {
        yerler
            .lines()
            .map(str::trim)
            .find(|s| s.to_lowercase().ends_with(ad))
            .unwrap_or_else(|| panic!("{ad} bulunamadı:\n{yerler}"))
            .to_lowercase()
    };
    assert!(ilk(r"\python.exe").starts_with(&kok_metni), "{yerler}");
    assert!(ilk(r"\node.exe").starts_with(&kok_metni), "{yerler}");
    let alan = d.path().canonicalize().unwrap();
    let alan = alan.display().to_string().to_lowercase();
    let alan = alan.trim_start_matches(r"\\?\");
    assert!(
        yerler.to_lowercase().contains(&format!("gercek={alan}")),
        "realpath yanlış:\n{yerler}"
    );

    k.calistir(json!({ "program": "python", "argumanlar": ["-m", "venv", ".venv"], "ag": false }))
        .await;
    // Kabuk aracı mutlak program yolunu kabul etmez; göreli yol alandadır.
    let venv_python = r".venv\Scripts\python.exe";
    let kur = k
        .calistir(json!({
            "program": venv_python,
            "argumanlar": ["-m", "pip", "install", "--disable-pip-version-check", "pytest"],
            "ag": true,
            "zaman_asimi_sn": 600
        }))
        .await;
    assert!(kur.contains("Successfully installed"), "{kur}");
    let test = k
        .calistir(json!({
            "program": venv_python,
            "argumanlar": ["-m", "pytest", "-q", "-p", "no:cacheprovider"],
            "ag": false
        }))
        .await;
    assert!(test.contains("1 passed"), "{test}");

    // pip'in kullanıcı kurulumu; betiği (`pyflakes.exe`) kabın PATH'indedir.
    let kullanici = k
        .calistir(json!({
            "program": "python",
            "argumanlar": ["-m", "pip", "install", "--user", "--disable-pip-version-check", "pyflakes"],
            "ag": true,
            "zaman_asimi_sn": 600
        }))
        .await;
    assert!(kullanici.contains("Successfully installed"), "{kullanici}");
    let pyflakes = k
        .calistir(betik("pyflakes --version; where.exe pyflakes"))
        .await;
    assert!(
        pyflakes.contains(r"Roaming\Python\Python314\Scripts\pyflakes.exe"),
        "{pyflakes}"
    );

    let ci = k
        .calistir(json!({
            "program": "npm",
            "argumanlar": ["ci", "--no-audit", "--no-fund"],
            "ag": true,
            "zaman_asimi_sn": 600
        }))
        .await;
    assert!(ci.contains("Çıkış kodu: 0"), "{ci}");
    let npm_test = k
        .calistir(json!({ "program": "npm", "argumanlar": ["test"], "ag": false }))
        .await;
    assert!(npm_test.contains("npm-test-ok"), "{npm_test}");
}

/// Kullanıcının kurduğu Git for Windows, hiçbir dizin açılmadan kapta
/// Program Files'tan bulunur ve okuyan komutlar çalışır (2d-3: Git
/// dağıtılmaz). Git kurulu değilse test atlanır.
#[cfg(windows)]
#[tokio::test]
async fn git_program_files_ten_calisir() {
    if !Path::new(r"C:\Program Files\Git\cmd\git.exe").is_file() {
        eprintln!("Git for Windows kurulu değil; test atlandı");
        return;
    }
    let d = TempDir::new().unwrap();
    git_host(d.path(), &["init", "-q", "-b", "ana"]);
    std::fs::write(d.path().join("benioku.md"), "ilk\n").unwrap();
    git_host(d.path(), &["add", "."]);
    git_host(d.path(), &["commit", "-q", "-m", "ilk"]);
    std::fs::write(d.path().join("benioku.md"), "ilk\nikinci\n").unwrap();
    let k = kurgu_ile(d.path(), Vec::new(), Vec::new());
    let cikti = k
        .calistir(betik(
            "$ErrorActionPreference='Stop'; \
             if (Get-Variable PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) { $PSNativeCommandUseErrorActionPreference=$false }; \
             where.exe git; \
             git status --short; Write-Output \"durum=$LASTEXITCODE\"; \
             git diff --stat; Write-Output \"fark=$LASTEXITCODE\"; \
             git log --oneline -1; Write-Output \"gunluk=$LASTEXITCODE\"",
        ))
        .await;
    assert!(
        cikti.to_lowercase().contains(r"program files\git"),
        "{cikti}"
    );
    assert!(cikti.contains("M benioku.md"), "{cikti}");
    for etiket in ["durum=", "fark=", "gunluk="] {
        assert_eq!(etiket_kodu(&cikti, etiket), 0, "{etiket}\n{cikti}");
    }
}

/// Bağımlılık dizinine konmuş, gizli bir dosyanın hard link'i yine gizlidir:
/// tarama o dizinlere de iner (2d-3, Görev 2b). Windows'ta ACL dosya
/// nesnesindedir; hard link zaten aynı izni taşır.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn bagimlilik_dizinindeki_hard_link_gizli_kalir() {
    let d = TempDir::new().unwrap();
    std::fs::write(d.path().join(".env"), "env-sir").unwrap();
    let dizin = d.path().join("node_modules/pkg");
    std::fs::create_dir_all(&dizin).unwrap();
    std::fs::hard_link(d.path().join(".env"), dizin.join("index.js")).unwrap();
    let k = kurgu(d.path());
    let cikti = k
        .calistir(betik(
            "cat node_modules/pkg/index.js 2>/dev/null; echo bitti",
        ))
        .await;
    assert!(cikti.contains("bitti"), "{cikti}");
    assert!(!cikti.contains("env-sir"), "{cikti}");
}

/// Güvenlik analizi Bulgu 1: yalıtımlı komutun oluşturduğu tehlikeli `.git`
/// kenara alınır; ardından yalıtım dışında çalışan `git status` kod
/// çalıştırmaz. Düz `git init` yerinde kalır.
#[tokio::test]
async fn yalitimda_tehlikeli_yeni_git_karantinaya_alinir() {
    let d = TempDir::new().unwrap();
    let k = kurgu(d.path());
    #[cfg(target_os = "linux")]
    let komut = "git init -q && git config core.fsmonitor 'touch KANIT' && git init -q duz";
    #[cfg(windows)]
    let komut = "git init -q; if ($LASTEXITCODE) { exit 1 }; \
                 git config core.fsmonitor 'touch KANIT'; if ($LASTEXITCODE) { exit 1 }; \
                 git init -q duz; exit $LASTEXITCODE";
    let a = betik(komut);
    let h = k.arac.hazirla(&a).await.expect("hazırlık");
    let c = k
        .arac
        .calistir_akisli(&a, &h, CancellationToken::new(), CiktiGonderici::bos())
        .await
        .expect("komut başarılı");
    assert!(!d.path().join(".git").exists());
    assert!(d.path().join("duz/.git").is_dir(), "düz depo yerinde kalır");
    let karantina: Vec<String> = std::fs::read_dir(d.path())
        .unwrap()
        .map(|g| g.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|ad| ad.starts_with(".git.kalfa-karantina-"))
        .collect();
    assert_eq!(karantina.len(), 1, "{karantina:?}");
    assert_eq!(c.guvenlik_uyarilari.len(), 1, "{:?}", c.guvenlik_uyarilari);
    assert!(c.guvenlik_uyarilari[0].contains("core.fsmonitor"));
    let _ = Command::new("git")
        .arg("-C")
        .arg(d.path())
        .arg("status")
        .output()
        .unwrap();
    assert!(!d.path().join("KANIT").exists());
}

/// Güvenlik analizi Bulgu 2: var olan kendiliğinden çalışan yapılandırma
/// yalıtımda salt okunurdur; `.vscode`'un içine yeni dosya da konamaz.
#[tokio::test]
async fn var_olan_vscode_ayari_yazilamaz() {
    let d = TempDir::new().unwrap();
    std::fs::create_dir_all(d.path().join(".vscode")).unwrap();
    std::fs::write(d.path().join(".vscode/settings.json"), "{}").unwrap();
    std::fs::create_dir_all(d.path().join("alt/.cargo")).unwrap();
    std::fs::write(d.path().join("alt/.cargo/config.toml"), "").unwrap();
    let k = kurgu(d.path());
    #[cfg(target_os = "linux")]
    let komut = "echo x >> .vscode/settings.json; echo x > .vscode/tasks.json; \
                 echo x >> alt/.cargo/config.toml; true";
    #[cfg(windows)]
    let komut = "Add-Content -LiteralPath .vscode/settings.json -Value x -ErrorAction SilentlyContinue; \
                 Set-Content -LiteralPath .vscode/tasks.json -Value x -ErrorAction SilentlyContinue; \
                 Add-Content -LiteralPath alt/.cargo/config.toml -Value x -ErrorAction SilentlyContinue; exit 0";
    k.calistir(betik(komut)).await;
    assert_eq!(
        std::fs::read_to_string(d.path().join(".vscode/settings.json")).unwrap(),
        "{}"
    );
    assert!(!d.path().join(".vscode/tasks.json").exists());
    assert_eq!(
        std::fs::read_to_string(d.path().join("alt/.cargo/config.toml")).unwrap(),
        ""
    );
}

/// Güvenlik analizi Bulgu 1: her derinlikteki `.git` yalıtımda salt okunurdur;
/// yalıtımlı bir komutun kendi oluşturduğu `.git` de sonraki komutta.
#[tokio::test]
async fn ic_ice_git_yalitimda_salt_okunur() {
    let d = TempDir::new().unwrap();
    std::fs::create_dir_all(d.path().join("alt2")).unwrap();
    std::fs::write(d.path().join("alt2/.git"), "gitdir: ../zz\n").unwrap();
    let k = kurgu(d.path());
    k.calistir(betik("git init -q alt")).await;
    let config = d.path().join("alt/.git/config");
    let onceki = std::fs::read(&config).unwrap();
    #[cfg(target_os = "linux")]
    let komut = "echo '[core] fsmonitor = x' >> alt/.git/config; \
                 echo 'gitdir: ../yy' > alt2/.git; \
                 mkdir -p alt/.git/hooks && echo x > alt/.git/hooks/pre-commit; true";
    #[cfg(windows)]
    let komut = "Add-Content -LiteralPath alt/.git/config -Value '[core] fsmonitor = x' -ErrorAction SilentlyContinue; \
                 Set-Content -LiteralPath alt2/.git -Value 'gitdir: ../yy' -ErrorAction SilentlyContinue; \
                 Set-Content -LiteralPath alt/.git/hooks/pre-commit -Value 'x' -ErrorAction SilentlyContinue; exit 0";
    k.calistir(betik(komut)).await;
    assert_eq!(std::fs::read(&config).unwrap(), onceki);
    assert_eq!(
        std::fs::read_to_string(d.path().join("alt2/.git")).unwrap(),
        "gitdir: ../zz\n"
    );
    assert!(!d.path().join("alt/.git/hooks/pre-commit").exists());
}

/// Aşama 3 tasarımı §5.4: yalıtımlı komutun değiştirdiği dosyalar gerçek
/// yalıtımda da bulunur. Gizli dosya ve `.git` yazılamaz; görüntüye de
/// girmezler.
#[tokio::test]
async fn yalitimli_kabugun_degisiklikleri_bulunur() {
    use kalfa_core::arac::DegisiklikTuru::{Degisti, Eklendi, Silindi};
    let d = TempDir::new().unwrap();
    std::fs::write(d.path().join("silinecek.txt"), "x").unwrap();
    std::fs::write(d.path().join("degisecek.txt"), "x").unwrap();
    std::fs::write(d.path().join("ayni.txt"), "x").unwrap();
    let k = kurgu(d.path());
    #[cfg(target_os = "linux")]
    let komut =
        "rm silinecek.txt && echo uzun-icerik > degisecek.txt && mkdir alt && echo y > alt/yeni.rs";
    #[cfg(windows)]
    let komut = "Remove-Item silinecek.txt; Set-Content degisecek.txt 'uzun-icerik'; \
                 New-Item -ItemType Directory alt | Out-Null; Set-Content alt/yeni.rs 'y'";
    let a = betik(komut);
    let h = k.arac.hazirla(&a).await.expect("hazırlık");
    let c = k
        .arac
        .calistir_akisli(&a, &h, CancellationToken::new(), CiktiGonderici::bos())
        .await
        .expect("komut başarılı");
    let g = c.degisiklikler.expect("kabuk değişiklikleri verir");
    let dosyalar: Vec<(&str, _)> = g.dosyalar.iter().map(|f| (f.yol.as_str(), f.tur)).collect();
    assert_eq!(
        dosyalar,
        vec![
            ("alt/yeni.rs", Eklendi),
            ("degisecek.txt", Degisti),
            ("silinecek.txt", Silindi),
        ]
    );
    assert!(g.tam_mi());
}

/// Aşama 3 tasarımı §5.2 (G10): geri alma deposu yalıtımın özel kökünün
/// kardeşidir; kap üst dizini listeleyebilse de deponun içini okuyamaz.
/// Windows'ta gerçek konum `%LOCALAPPDATA%\Kalfa\geri-alma`'dır; burada aynı
/// yerleşim geçici veri dizininde kurulur.
#[tokio::test]
async fn geri_alma_deposu_yalitimda_gorunmez() {
    let d = TempDir::new().unwrap();
    let k = kurgu(d.path());
    let depo = k.veri().join("geri-alma").join("oturum");
    std::fs::create_dir_all(&depo).unwrap();
    let dosya = depo.join("a".repeat(64));
    std::fs::write(&dosya, "onceki-hal-sir").unwrap();
    let yol = dosya.display().to_string();
    let komut = if cfg!(windows) {
        format!(
            "try {{ Get-Content -Raw -ErrorAction Stop -LiteralPath '{yol}' }} catch {{ 'okunamadi' }};              try {{ Get-ChildItem -Name -ErrorAction Stop -LiteralPath '{}' }} catch {{ 'listelenemedi' }}",
            depo.display()
        )
    } else {
        format!(
            "cat '{yol}' 2>/dev/null || echo okunamadi; ls '{}' 2>/dev/null || echo listelenemedi",
            depo.display()
        )
    };
    let cikti = k.calistir(betik(&komut)).await;
    assert!(!cikti.contains("onceki-hal-sir"), "{cikti}");
    assert!(!cikti.contains(&"a".repeat(64)), "{cikti}");
    assert!(cikti.contains("okunamadi"), "{cikti}");
    assert!(cikti.contains("listelenemedi"), "{cikti}");
}

/// Aşama 4 tasarımı §10, §13.4: eklenti deposu, eklenti kaydı
/// (`eklentiler.json`) ve kurulu eklentinin `.wasm`'ı uygulamanın veri
/// dizinindedir; kabuk komutu onları okuyamaz ve kayda yazıp kendine
/// gönderim izni veremez. Windows'ta depo `%LOCALAPPDATA%\Kalfa\eklenti-verisi`,
/// kayıt `%APPDATA%\Kalfa`'dadır; burada aynı yerleşim geçici veri dizininde
/// kurulur.
#[tokio::test]
async fn eklenti_deposu_ve_kaydi_yalitimda_gorunmez() {
    let d = TempDir::new().unwrap();
    let k = kurgu(d.path());
    let depo = k
        .veri()
        .join("eklenti-verisi")
        .join("kalfa-rag")
        .join("0123456789abcdef");
    std::fs::create_dir_all(&depo).unwrap();
    let dilim = depo.join("b".repeat(64));
    std::fs::write(&dilim, "indeks-sir").unwrap();
    let kayit = k.veri().join("eklentiler.json");
    std::fs::write(&kayit, "{\"surum\":1,\"kayit-sir\":true}").unwrap();
    let wasm = k
        .veri()
        .join("eklentiler")
        .join("ornek")
        .join("eklenti.wasm");
    std::fs::create_dir_all(wasm.parent().unwrap()).unwrap();
    std::fs::write(&wasm, "wasm-sir").unwrap();
    let komut = if cfg!(windows) {
        let oku = |yol: &Path| {
            format!(
                "try {{ Get-Content -Raw -ErrorAction Stop -LiteralPath '{}' }} catch {{ 'okunamadi' }};",
                yol.display()
            )
        };
        format!(
            "{} {} {} try {{ Get-ChildItem -Name -ErrorAction Stop -LiteralPath '{}' }} catch {{ 'listelenemedi' }}; try {{ Set-Content -ErrorAction Stop -LiteralPath '{}' -Value 'yazildi' }} catch {{ 'yazilamadi' }}",
            oku(&dilim),
            oku(&kayit),
            oku(&wasm),
            depo.display(),
            kayit.display()
        )
    } else {
        format!(
            "cat '{}' 2>/dev/null || echo okunamadi; cat '{}' 2>/dev/null || echo okunamadi; cat '{}' 2>/dev/null || echo okunamadi; ls '{}' 2>/dev/null || echo listelenemedi; (echo yazildi > '{}') 2>/dev/null || echo yazilamadi",
            dilim.display(),
            kayit.display(),
            wasm.display(),
            depo.display(),
            kayit.display()
        )
    };
    let cikti = k.calistir(betik(&komut)).await;
    for sir in ["indeks-sir", "kayit-sir", "wasm-sir", &"b".repeat(64)] {
        assert!(!cikti.contains(sir), "{sir} görünmemeli: {cikti}");
    }
    assert_eq!(cikti.matches("okunamadi").count(), 3, "{cikti}");
    assert!(cikti.contains("listelenemedi"), "{cikti}");
    // Windows'ta yazma reddedilir. Linux'ta bwrap yalıtım dizininin
    // üstlerini kabın içinde tmpfs'te kurar; yazma oraya gider ve kapla
    // biter. İki platformda da gerçek kayıt değişmez.
    if cfg!(windows) {
        assert!(cikti.contains("yazilamadi"), "{cikti}");
    }
    assert_eq!(
        std::fs::read_to_string(&kayit).unwrap(),
        "{\"surum\":1,\"kayit-sir\":true}",
        "kabuk eklenti kaydını değiştirmemeli"
    );
}

struct SahteCozucu {
    eslemeler: std::collections::HashMap<(String, u16), Vec<std::net::SocketAddr>>,
}

impl kalfa_sandbox::vekil::Cozucu for SahteCozucu {
    fn coz<'a>(
        &'a self,
        host: &'a str,
        port: u16,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = std::io::Result<Vec<std::net::SocketAddr>>>
                + Send
                + 'a,
        >,
    > {
        let key = (host.to_lowercase(), port);
        let res = self.eslemeler.get(&key).cloned().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{host}:{port} bulunamadı"),
            )
        });
        Box::pin(async move { res })
    }
}

/// curl, git, npm, Node (fetch) ve Python urllib yerel sahte vekile CONNECT
/// gönderiyor; Linux'ta cargo da. Paket indirilmez (3d planı Görev 4, Windows
/// alt planı Görev W4).
#[tokio::test]
async fn agli_curl_git_npm_node_python_vekillle_connect_gonderir() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let dinleyici = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = dinleyici.local_addr().unwrap().port();

    tokio::spawn(async move {
        while let Ok((mut soket, _)) = dinleyici.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = soket.read(&mut buf).await;
                let _ = soket
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK")
                    .await;
            });
        }
    });

    let mut eslemeler = std::collections::HashMap::new();
    eslemeler.insert(
        ("registry.npmjs.org".to_string(), 443),
        vec![std::net::SocketAddr::from(([127, 0, 0, 1], port))],
    );
    eslemeler.insert(
        ("github.com".to_string(), 443),
        vec![std::net::SocketAddr::from(([127, 0, 0, 1], port))],
    );
    eslemeler.insert(
        ("pypi.org".to_string(), 443),
        vec![std::net::SocketAddr::from(([127, 0, 0, 1], port))],
    );
    eslemeler.insert(
        ("index.crates.io".to_string(), 443),
        vec![std::net::SocketAddr::from(([127, 0, 0, 1], port))],
    );
    let cozucu = Arc::new(SahteCozucu { eslemeler });

    let d = TempDir::new().unwrap();
    let k = kurgu_agli(d.path(), Some(cozucu));

    // 1. curl CONNECT gönderir
    let curl_komut = if cfg!(windows) {
        "curl.exe -s -S --connect-timeout 5 https://registry.npmjs.org/"
    } else {
        "curl -s -S --connect-timeout 5 https://registry.npmjs.org/"
    };
    let (_, ag) = k
        .calistir_agli(curl_komut, &["registry.npmjs.org:443"])
        .await;
    let ag = ag.expect("curl ağ özeti üretmeli");
    assert!(
        ag.hostlar
            .iter()
            .any(|h| h.host == "registry.npmjs.org" && h.baglanti_sayisi > 0),
        "curl registry.npmjs.org'a CONNECT göndermeli: {:?}",
        ag.hostlar
    );

    // 2. git CONNECT gönderir
    let (_, ag) = k
        .calistir_agli(
            "git -c http.sslVerify=false ls-remote https://github.com/octocat/Hello-World HEAD",
            &["github.com:443"],
        )
        .await;
    let ag = ag.expect("git ağ özeti üretmeli");
    assert!(
        ag.hostlar
            .iter()
            .any(|h| h.host == "github.com" && h.baglanti_sayisi > 0),
        "git github.com'a CONNECT göndermeli: {:?}",
        ag.hostlar
    );

    // 3. npm CONNECT gönderir
    let npm_komut = if cfg!(windows) {
        "cmd /c npm view left-pad version"
    } else {
        "npm view left-pad version"
    };
    let (_, ag) = k
        .calistir_agli(npm_komut, &["registry.npmjs.org:443"])
        .await;
    let ag = ag.expect("npm ağ özeti üretmeli");
    assert!(
        ag.hostlar
            .iter()
            .any(|h| h.host == "registry.npmjs.org" && h.baglanti_sayisi > 0),
        "npm registry.npmjs.org'a CONNECT göndermeli: {:?}",
        ag.hostlar
    );

    // 4. node fetch CONNECT gönderir
    let (_, ag) = k
        .calistir_agli(
            "node -e \"fetch('https://registry.npmjs.org/left-pad').catch(() => {})\"",
            &["registry.npmjs.org:443"],
        )
        .await;
    let ag = ag.expect("node fetch ağ özeti üretmeli");
    assert!(
        ag.hostlar
            .iter()
            .any(|h| h.host == "registry.npmjs.org" && h.baglanti_sayisi > 0),
        "node fetch registry.npmjs.org'a CONNECT göndermeli: {:?}",
        ag.hostlar
    );

    // 5. Python urllib CONNECT gönderir; paket indirmez.
    let py_komut = if cfg!(windows) {
        "python -c \"import urllib.request; urllib.request.urlopen('https://pypi.org/').read()\""
    } else {
        "python3 -c \"import urllib.request; urllib.request.urlopen('https://pypi.org/').read()\""
    };
    let (_, ag) = k.calistir_agli(py_komut, &["pypi.org:443"]).await;
    let ag = ag.expect("python ağ özeti üretmeli");
    assert!(
        ag.hostlar
            .iter()
            .any(|h| h.host == "pypi.org" && h.baglanti_sayisi > 0),
        "Python urllib pypi.org'a CONNECT göndermeli: {:?}",
        ag.hostlar
    );

    // 6. cargo CONNECT gönderir. A15 testi çevrimdışı olduğundan bu pozitif
    // Cargo kontrolü Linux ile sınırlıdır.
    #[cfg(target_os = "linux")]
    {
        let (_, ag) = k
            .calistir_agli(
                "cargo search serde --limit 1 --config net.retry=0",
                &["index.crates.io:443"],
            )
            .await;
        let ag = ag.expect("cargo ağ özeti üretmeli");
        assert!(
            ag.hostlar.iter().any(|h| h.host == "index.crates.io"
                && h.baglanti_sayisi > 0
                && h.reddedilenler == 0),
            "cargo index.crates.io'ya vekilden CONNECT göndermeli: {:?}",
            ag.hostlar
        );
    }
}

/// Schannel'lı curl, stapling yapmayan bir hosta (index.crates.io) kapta
/// gerçek TLS ile bağlanır: iptal bilgisi alınamaz, `.curlrc`'deki
/// `ssl-revoke-best-effort` bağlantıyı düşürmez (3d iptal bulguları).
#[cfg(windows)]
#[tokio::test]
async fn curl_kapta_iptal_bilgisi_olmadan_baglanir() {
    let d = TempDir::new().unwrap();
    let k = kurgu_agli(d.path(), None);
    let (sonuc, _) = k
        .calistir_agli(
            "curl.exe -sS -o NUL -w \"kod=%{http_code}\" https://index.crates.io/config.json",
            &["index.crates.io:443"],
        )
        .await;
    let cikti = sonuc.unwrap_or_else(|e| panic!("curl düştü:\n{e}"));
    assert!(cikti.contains("kod=200"), "{cikti}");
}

/// Sistem pip AC/Temp altında oluşturduğu dizinleri temizleyemiyor (A16).
/// Paket yokken `--no-index` ile ölçülür; genel paket-bulunamadı hatası bu
/// sınırın kanıtı değildir. İndirme veya kurulum yapılmaz.
#[cfg(windows)]
#[tokio::test]
async fn sistem_pip_a16_gecici_dizin_temizligi_siniri_beklenir() {
    let d = TempDir::new().unwrap();
    let k = kurgu_agli(d.path(), None);

    let (sonuc, _) = k
        .calistir_agli(
            "$env:PIP_CONFIG_FILE='nul'; python -m pip --isolated download six --no-index --no-deps --no-cache-dir --disable-pip-version-check",
            &["pypi.org:443", "files.pythonhosted.org:443"],
        )
        .await;
    // Beklenen sınır: A16 sistem pip AppContainer AC\\Temp erişim sınırı. Sınır kalkarsa kırmızıya döner.
    let cikti = match sonuc {
        Ok(c) => c,
        Err(e) => e,
    };
    assert!(
        cikti.contains("Failed to remove contents in a temporary directory")
            && cikti.contains(r"AC\Temp\pip-"),
        "A16 geçici dizin temizliği sınırı bekleniyordu, çıktı:\n{cikti}"
    );
    let kimlik = alan_kimligi(d.path());
    let gecici = k
        .veri()
        .join("yalitim")
        .join(&kimlik)
        .join("ev/Packages")
        .join(format!("Kalfa.Alan.{kimlik}"))
        .join("AC/Temp");
    assert!(
        std::fs::read_dir(gecici)
            .unwrap()
            .filter_map(Result::ok)
            .any(|g| { g.file_name().to_string_lossy().starts_with("pip-") && g.path().is_dir() }),
        "pip temizleyemediğini bildirdiği dizini gerçekten bırakmalı"
    );
}

/// Yalıtımda dosya okuyan betik satırı; okunamazsa `okunamadi` yazar.
fn okuma_satiri(yol: &str) -> String {
    if cfg!(windows) {
        format!(
            "try {{ Get-Content -Raw -ErrorAction Stop -LiteralPath '{yol}' }} catch {{ 'okunamadi' }}"
        )
    } else {
        format!("cat '{yol}' 2>/dev/null || echo okunamadi")
    }
}

fn gizli_ayarlari(ek: &[&str], istisna: &[&str]) -> Arc<GizliAyarlari> {
    let l = |k: &[&str]| k.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    Arc::new(GizliAyarlari::yeni(&l(ek), &l(istisna)).unwrap())
}

/// 3e Görev 3: gizli sayılmayan dosya yalıtımda okunur; istisnaya uymayan
/// gizli dosya okunamaz (aşama 3 tasarımı §9.2, §11.3).
#[tokio::test]
async fn istisna_edilen_dosya_yalitimda_okunur() {
    let d = TempDir::new().unwrap();
    std::fs::create_dir_all(d.path().join("src")).unwrap();
    std::fs::write(d.path().join(".env.example"), "ornek-acik").unwrap();
    std::fs::write(d.path().join("src/credentials.rs"), "kaynak-acik").unwrap();
    std::fs::write(d.path().join(".env"), "env-sir").unwrap();
    let k = kurgu_ayarli(
        d.path(),
        gizli_ayarlari(&[], &[".env.example", "src/credentials.rs"]),
    );
    let metin = [".env.example", "src/credentials.rs", ".env"]
        .map(okuma_satiri)
        .join("\n");
    let cikti = k.calistir(betik(&metin)).await;
    assert!(cikti.contains("ornek-acik"), "{cikti}");
    assert!(cikti.contains("kaynak-acik"), "{cikti}");
    assert!(!cikti.contains("env-sir"), "{cikti}");
}

/// İstisna yalnızca adın eşleşmesini kaldırır: gizli dosyanın hard link'i,
/// adı istisnaya uysa da yalıtımda gizli kalır (Linux'ta aygıt ve inode,
/// Windows'ta dosya nesnesinin ACL'i).
#[tokio::test]
async fn gizli_dosyanin_hard_linki_istisnaya_uysa_da_okunamaz() {
    let d = TempDir::new().unwrap();
    std::fs::create_dir_all(d.path().join("src")).unwrap();
    std::fs::write(d.path().join(".env"), "env-sir").unwrap();
    std::fs::hard_link(d.path().join(".env"), d.path().join("src/credentials.rs")).unwrap();
    let k = kurgu_ayarli(d.path(), gizli_ayarlari(&[], &["src/credentials.rs"]));
    let metin = [okuma_satiri("src/credentials.rs"), "echo bitti".to_string()].join("\n");
    let cikti = k.calistir(betik(&metin)).await;
    assert!(cikti.contains("bitti"), "{cikti}");
    assert!(!cikti.contains("env-sir"), "{cikti}");
}

/// Gizli dizindeki dosyaya verilen istisna dizini açmaz.
#[tokio::test]
async fn gizli_dizindeki_dosyaya_istisna_dizini_acmaz() {
    let d = TempDir::new().unwrap();
    std::fs::create_dir_all(d.path().join("gizli")).unwrap();
    std::fs::write(d.path().join("gizli/ornek.txt"), "dizin-sir").unwrap();
    let k = kurgu_ayarli(d.path(), gizli_ayarlari(&["gizli"], &["gizli/ornek.txt"]));
    let metin = [okuma_satiri("gizli/ornek.txt"), "echo bitti".to_string()].join("\n");
    let cikti = k.calistir(betik(&metin)).await;
    assert!(cikti.contains("bitti"), "{cikti}");
    assert!(!cikti.contains("dizin-sir"), "{cikti}");
}
