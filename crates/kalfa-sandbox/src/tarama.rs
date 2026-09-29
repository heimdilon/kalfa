//! Gizli dosya taraması (aşama 2 tasarımı §7.2). Her çalıştırmadan hemen önce
//! bütün çalışma alanı taranır; `.gitignore` kapsamındakiler dahil, çünkü
//! `.env` genellikle yok sayılan bir dosyadır. Sembolik bağlantılar izlenmez.
//!
//! Linux'ta bağlantı sayısı birden büyük dosyaların (aygıt, inode) çifti gizli
//! dosyalarınkiyle karşılaştırılır: `.env`'nin hard link'i de gizlidir.
//!
//! Tarama her derinlikteki `.git` girdilerini de toplar (güvenlik analizi
//! Bulgu 1); `.git` dizinlerinin içine inilmez. Kendiliğinden çalışan
//! yapılandırma yollarını da toplar (Bulgu 2). Adın `.git` ya da yolun
//! yapılandırma sayılıp sayılmadığına çağıran karar verir (çekirdekteki
//! kurallar).
//!
//! Aynı taramalar komuttan önce ve sonra bir meta veri görüntüsü de toplar
//! (aşama 3 tasarımı §5.4): kabuğun değiştirdiği dosyalar ikisinin farkıdır.
//! İçerik okunmaz; yeni bir tarama açılmaz.

use std::collections::{BTreeSet, HashMap};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant, UNIX_EPOCH};

use ignore::{WalkBuilder, WalkState};

/// Görüntüye girebilecek en çok girdi; aşılırsa görüntü eksiktir.
pub const GORUNTU_AZAMI_GIRDI: usize = 500_000;
/// Görüntünün toplanabileceği en uzun süre; aşılırsa görüntü eksiktir. Gizli
/// dosya taraması sürer, yalnızca görüntü toplanmaz.
pub const GORUNTU_AZAMI_SURE: Duration = Duration::from_secs(5);

/// Görüntünün sınırları; testler küçültür.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoruntuSiniri {
    pub girdi: usize,
    pub sure: Duration,
}

impl Default for GoruntuSiniri {
    fn default() -> Self {
        Self {
            girdi: GORUNTU_AZAMI_GIRDI,
            sure: GORUNTU_AZAMI_SURE,
        }
    }
}

/// Çalışma alanının dosyalarının ve bağlantılarının meta verisi. Anahtar köke
/// göre yoldur, `/` ayraçlı. `.git`'in içi ve `.git` girdileri görüntüye
/// girmez. Dizinler girmez: değişiklik, içlerindeki dosyalarda görünür.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Goruntu {
    pub girdiler: HashMap<String, Girdi>,
    /// Sınır aşıldı ya da bir girdi okunamadı; bazı girdiler görüntüde yok.
    pub eksik: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Girdi {
    pub tur: GirdiTuru,
    pub boyut: u64,
    /// Son değişiklik zamanı, Unix döneminden bu yana ns.
    pub zaman_ns: u128,
    /// Unix'te `(aygıt, inode)`. Windows'ta dizin girdisinde yoktur; almak
    /// için dosyayı açmak gerekir.
    pub kimlik: Option<(u64, u64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GirdiTuru {
    Dosya,
    Baglanti,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DegisiklikTuru {
    Eklendi,
    Degisti,
    Silindi,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Degisiklik {
    /// Köke göre yol, `/` ayraçlı.
    pub yol: String,
    pub tur: DegisiklikTuru,
}

/// İki görüntünün farkı, yola göre sıralı. Türü, boyutu, zamanı ya da kimliği
/// değişen girdi "değişti" sayılır. Aynı zaman damgası içinde boyutu
/// değişmeden yapılan değişiklik kaçabilir (bilinen sınır, üst belge §6.7).
pub fn karsilastir(once: &Goruntu, sonra: &Goruntu) -> Vec<Degisiklik> {
    let mut sonuc = Vec::new();
    for (yol, s) in &sonra.girdiler {
        let tur = match once.girdiler.get(yol) {
            None => DegisiklikTuru::Eklendi,
            Some(o) if o != s => DegisiklikTuru::Degisti,
            Some(_) => continue,
        };
        sonuc.push(Degisiklik {
            yol: yol.clone(),
            tur,
        });
    }
    for yol in once.girdiler.keys() {
        if !sonra.girdiler.contains_key(yol) {
            sonuc.push(Degisiklik {
                yol: yol.clone(),
                tur: DegisiklikTuru::Silindi,
            });
        }
    }
    sonuc.sort();
    sonuc
}

/// Paralel taramada görüntüyü toplar. Her iş parçacığı kendi parçasını
/// biriktirir (`YerelGoruntu`); ortak kilit yalnızca parça bitince alınır.
struct GoruntuToplayici<'a> {
    kok: &'a Path,
    sinir: GoruntuSiniri,
    baslangic: Instant,
    sayi: AtomicUsize,
    eksik: AtomicBool,
    parcalar: Mutex<Vec<Vec<(String, Girdi)>>>,
}

impl<'a> GoruntuToplayici<'a> {
    fn yeni(kok: &'a Path, sinir: GoruntuSiniri) -> Self {
        Self {
            kok,
            sinir,
            baslangic: Instant::now(),
            sayi: AtomicUsize::new(0),
            eksik: AtomicBool::new(false),
            parcalar: Mutex::new(Vec::new()),
        }
    }

    fn yerel(&self) -> YerelGoruntu<'_, 'a> {
        YerelGoruntu {
            toplayici: self,
            girdiler: Vec::new(),
        }
    }

    fn eksik_isaretle(&self) {
        self.eksik.store(true, Ordering::Relaxed);
    }

    fn bitir(self) -> Goruntu {
        let parcalar = self
            .parcalar
            .into_inner()
            .unwrap_or_else(|z| z.into_inner());
        Goruntu {
            girdiler: parcalar.into_iter().flatten().collect(),
            eksik: self.eksik.into_inner(),
        }
    }
}

/// Bir iş parçacığının görüntü parçası; bırakılınca ortak listeye eklenir.
struct YerelGoruntu<'t, 'a> {
    toplayici: &'t GoruntuToplayici<'a>,
    girdiler: Vec<(String, Girdi)>,
}

impl YerelGoruntu<'_, '_> {
    /// Dosya ya da bağlantıysa görüntüye ekler. Sınır aşıldıysa eklemez ve
    /// görüntüyü eksik işaretler.
    fn ekle(&mut self, giris: &ignore::DirEntry) {
        let t = self.toplayici;
        let Some(tur) = giris.file_type() else {
            return;
        };
        let tur = if tur.is_symlink() {
            GirdiTuru::Baglanti
        } else if tur.is_file() {
            GirdiTuru::Dosya
        } else {
            return;
        };
        if t.eksik.load(Ordering::Relaxed) {
            return;
        }
        if t.sayi.fetch_add(1, Ordering::Relaxed) >= t.sinir.girdi
            || t.baslangic.elapsed() > t.sinir.sure
        {
            t.eksik_isaretle();
            return;
        }
        let (Ok(goreli), Ok(bilgi)) = (giris.path().strip_prefix(t.kok), giris.metadata()) else {
            // Okunamayan girdi farkı gizleyebilir.
            t.eksik_isaretle();
            return;
        };
        let zaman_ns = bilgi
            .modified()
            .ok()
            .and_then(|z| z.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |s| s.as_nanos());
        let girdi = Girdi {
            tur,
            boyut: bilgi.len(),
            zaman_ns,
            kimlik: kimlik(&bilgi),
        };
        self.girdiler.push((anahtar(goreli), girdi));
    }
}

impl Drop for YerelGoruntu<'_, '_> {
    fn drop(&mut self) {
        if !self.girdiler.is_empty() {
            self.toplayici
                .parcalar
                .lock()
                .unwrap_or_else(|z| z.into_inner())
                .push(std::mem::take(&mut self.girdiler));
        }
    }
}

/// Köke göre yolun `/` ayraçlı metni.
fn anahtar(goreli: &Path) -> String {
    let metin = goreli.to_string_lossy();
    if cfg!(windows) {
        metin.replace('\\', "/")
    } else {
        metin.into_owned()
    }
}

#[cfg(unix)]
fn kimlik(bilgi: &std::fs::Metadata) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    Some((bilgi.dev(), bilgi.ino()))
}

#[cfg(not(unix))]
fn kimlik(_bilgi: &std::fs::Metadata) -> Option<(u64, u64)> {
    None
}

/// Taramanın sonucu; yollar mutlaktır ve sıralıdır.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tarama {
    /// Okunamayacak dosyalar; gizli bir dizinin içindekiler bu listede yoktur,
    /// dizinin kendisi örtülür.
    pub dosyalar: Vec<PathBuf>,
    /// İçi boş görünecek dizinler.
    pub dizinler: Vec<PathBuf>,
    /// Her derinlikteki `.git` girdileri (dizin, dosya ya da bağlantı).
    pub git: Vec<GitGirdisi>,
    /// Kendiliğinden çalışan yapılandırma yolları; en üsttekiler (eşleşen
    /// bir dizinin içindekiler listelenmez).
    pub yapilandirma: Vec<Yapilandirma>,
    /// Komuttan önceki meta veri görüntüsü (aşama 3 tasarımı §5.4).
    pub goruntu: Goruntu,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GitGirdisi {
    pub yol: PathBuf,
    pub dizin: bool,
    /// Sembolik bağlantı; git onu izler, Kalfa izlemez.
    pub baglanti: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Yapilandirma {
    pub yol: PathBuf,
    pub dizin: bool,
}

/// Git girdisi taraması için ad kuralı.
pub type GitMi<'a> = &'a (dyn Fn(&OsStr) -> bool + Sync);
/// Yol kuralı: çalışma alanına göre yol ve yolun dizin olup olmadığı.
pub type YolKurali<'a> = &'a (dyn Fn(&Path, bool) -> bool + Sync);

/// Komuttan sonra aranan adlı girdiler.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdliGirdiler {
    pub git: Vec<GitGirdisi>,
    pub yapilandirma: Vec<Yapilandirma>,
    pub goruntu: Goruntu,
}

impl Tarama {
    pub fn sayi(&self) -> usize {
        self.dosyalar.len() + self.dizinler.len()
    }
}

#[derive(Default)]
struct Toplanan {
    dosyalar: BTreeSet<PathBuf>,
    dizinler: BTreeSet<PathBuf>,
    /// Gizli dosyaların (aygıt, inode) çiftleri; yalnızca bağlantı sayısı
    /// birden büyük olanlar.
    gizli_inode: BTreeSet<(u64, u64)>,
    /// Bağlantı sayısı birden büyük bütün normal dosyalar.
    bagli: Vec<((u64, u64), PathBuf)>,
    git: BTreeSet<GitGirdisi>,
    yapilandirma: BTreeSet<Yapilandirma>,
}

/// Çalışma alanını tarar. `gizli_mi` çalışma alanına göre yolu ve yolun dizin
/// olup olmadığını alır.
pub fn tara(kok: &Path, gizli_mi: YolKurali<'_>) -> Tarama {
    tara_ayrintili(kok, gizli_mi, &|_| false, &|_, _| false)
}

/// `tara` gibi; ayrıca adı `git_mi` olan girdileri ve `yapilandirma_mi`
/// yollarını toplar. `.git` dizinlerinin içine inmez.
pub fn tara_ayrintili(
    kok: &Path,
    gizli_mi: YolKurali<'_>,
    git_mi: GitMi<'_>,
    yapilandirma_mi: YolKurali<'_>,
) -> Tarama {
    let goruntu = GoruntuToplayici::yeni(kok, GoruntuSiniri::default());
    let toplanan = Mutex::new(Toplanan::default());
    WalkBuilder::new(kok)
        .standard_filters(false)
        .follow_links(false)
        .build_parallel()
        .run(|| {
            let toplanan = &toplanan;
            let mut goruntu = goruntu.yerel();
            Box::new(move |giris| {
                let Ok(giris) = giris else {
                    // Okunamayan dizin farkı gizleyebilir.
                    goruntu.toplayici.eksik_isaretle();
                    return WalkState::Continue;
                };
                if giris.depth() == 0 {
                    return WalkState::Continue;
                }
                let Some(tur) = giris.file_type() else {
                    return WalkState::Continue;
                };
                let Ok(goreli) = giris.path().strip_prefix(kok) else {
                    return WalkState::Continue;
                };
                if let Some(girdi) = git_girdisi(&giris, git_mi) {
                    let dizin = girdi.dizin;
                    toplanan
                        .lock()
                        .unwrap_or_else(|z| z.into_inner())
                        .git
                        .insert(girdi);
                    if dizin {
                        return WalkState::Skip;
                    }
                } else {
                    goruntu.ekle(&giris);
                }
                // Bağlantı izlenmez; hedefi çalışma alanındaysa kendisi taranır.
                if tur.is_symlink() || !(tur.is_dir() || tur.is_file()) {
                    return WalkState::Continue;
                }
                let gizli = gizli_mi(goreli, tur.is_dir());
                let yapilandirma = yapilandirma_mi(goreli, tur.is_dir());
                let inode = if tur.is_file() {
                    inode(giris.path())
                } else {
                    None
                };
                let mut t = toplanan.lock().unwrap_or_else(|z| z.into_inner());
                if yapilandirma {
                    t.yapilandirma.insert(Yapilandirma {
                        yol: giris.path().to_path_buf(),
                        dizin: tur.is_dir(),
                    });
                }
                if gizli {
                    if tur.is_dir() {
                        t.dizinler.insert(giris.path().to_path_buf());
                    } else {
                        t.dosyalar.insert(giris.path().to_path_buf());
                    }
                }
                if let Some((cift, bag)) = inode
                    && bag > 1
                {
                    if gizli {
                        t.gizli_inode.insert(cift);
                    }
                    t.bagli.push((cift, giris.path().to_path_buf()));
                }
                // Gizli dizinin içi de gezilir: içindeki bir dosyanın dışarıdaki
                // hard link'i bulunabilsin.
                WalkState::Continue
            })
        });
    let t = toplanan.into_inner().unwrap_or_else(|z| z.into_inner());
    let mut dosyalar = t.dosyalar;
    let gizli_dizin_altinda =
        |yol: &Path| yol.ancestors().skip(1).any(|ata| t.dizinler.contains(ata));
    // Gizli dizindeki dosyaların hard link'leri de gizlidir.
    let mut gizli_inode = t.gizli_inode;
    for (cift, yol) in &t.bagli {
        if gizli_dizin_altinda(yol) {
            gizli_inode.insert(*cift);
        }
    }
    for (cift, yol) in t.bagli {
        if gizli_inode.contains(&cift) {
            dosyalar.insert(yol);
        }
    }
    let dizinler: Vec<PathBuf> = t
        .dizinler
        .iter()
        .filter(|d| !gizli_dizin_altinda(d))
        .cloned()
        .collect();
    Tarama {
        dosyalar: dosyalar
            .into_iter()
            .filter(|d| !gizli_dizin_altinda(d))
            .collect(),
        dizinler,
        git: t.git.into_iter().collect(),
        yapilandirma: en_ustteki(t.yapilandirma),
        goruntu: goruntu.bitir(),
    }
}

/// Eşleşen bir dizinin altındakileri atar.
fn en_ustteki(kume: BTreeSet<Yapilandirma>) -> Vec<Yapilandirma> {
    let dizinler: Vec<PathBuf> = kume
        .iter()
        .filter(|y| y.dizin)
        .map(|y| y.yol.clone())
        .collect();
    kume.into_iter()
        .filter(|y| {
            !y.yol
                .ancestors()
                .skip(1)
                .any(|a| dizinler.iter().any(|d| d == a))
        })
        .collect()
}

/// `.git` girdileri, yapılandırma yolları ve meta veri görüntüsü; komuttan
/// önce (yalıtımsız) ve sonra. Gizli tarama ve inode yapılmaz.
pub fn adli_girdiler(
    kok: &Path,
    git_mi: GitMi<'_>,
    yapilandirma_mi: YolKurali<'_>,
) -> AdliGirdiler {
    adli_girdiler_sinirli(kok, git_mi, yapilandirma_mi, GoruntuSiniri::default())
}

/// `adli_girdiler`, görüntünün sınırı verilerek; testler için.
pub fn adli_girdiler_sinirli(
    kok: &Path,
    git_mi: GitMi<'_>,
    yapilandirma_mi: YolKurali<'_>,
    sinir: GoruntuSiniri,
) -> AdliGirdiler {
    let goruntu = GoruntuToplayici::yeni(kok, sinir);
    let toplanan = Mutex::new((BTreeSet::new(), BTreeSet::new()));
    WalkBuilder::new(kok)
        .standard_filters(false)
        .follow_links(false)
        .build_parallel()
        .run(|| {
            let toplanan = &toplanan;
            let mut goruntu = goruntu.yerel();
            Box::new(move |giris| {
                let Ok(giris) = giris else {
                    goruntu.toplayici.eksik_isaretle();
                    return WalkState::Continue;
                };
                if let Some(girdi) = git_girdisi(&giris, git_mi) {
                    let dizin = girdi.dizin;
                    toplanan
                        .lock()
                        .unwrap_or_else(|z| z.into_inner())
                        .0
                        .insert(girdi);
                    return if dizin {
                        WalkState::Skip
                    } else {
                        WalkState::Continue
                    };
                }
                if giris.depth() > 0 {
                    goruntu.ekle(&giris);
                }
                let (Some(tur), Ok(goreli)) = (giris.file_type(), giris.path().strip_prefix(kok))
                else {
                    return WalkState::Continue;
                };
                if giris.depth() > 0
                    && !tur.is_symlink()
                    && (tur.is_dir() || tur.is_file())
                    && yapilandirma_mi(goreli, tur.is_dir())
                {
                    toplanan
                        .lock()
                        .unwrap_or_else(|z| z.into_inner())
                        .1
                        .insert(Yapilandirma {
                            yol: giris.path().to_path_buf(),
                            dizin: tur.is_dir(),
                        });
                }
                WalkState::Continue
            })
        });
    let (git, yapilandirma) = toplanan.into_inner().unwrap_or_else(|z| z.into_inner());
    AdliGirdiler {
        git: git.into_iter().collect(),
        yapilandirma: en_ustteki(yapilandirma),
        goruntu: goruntu.bitir(),
    }
}

fn git_girdisi(giris: &ignore::DirEntry, git_mi: GitMi<'_>) -> Option<GitGirdisi> {
    if giris.depth() == 0 || !git_mi(giris.file_name()) {
        return None;
    }
    let tur = giris.file_type()?;
    Some(GitGirdisi {
        yol: giris.path().to_path_buf(),
        dizin: tur.is_dir(),
        baglanti: tur.is_symlink(),
    })
}

#[cfg(unix)]
fn inode(yol: &Path) -> Option<((u64, u64), u64)> {
    use std::os::unix::fs::MetadataExt;
    let bilgi = std::fs::symlink_metadata(yol).ok()?;
    Some(((bilgi.dev(), bilgi.ino()), bilgi.nlink()))
}

/// Windows'ta gerek yoktur: ACL dosya nesnesindedir, hard link aynı nesneyi
/// paylaşır.
#[cfg(not(unix))]
fn inode(_yol: &Path) -> Option<((u64, u64), u64)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_ya_da_gizli_dizin(yol: &Path, dizin: bool) -> bool {
        let ad = yol.file_name().and_then(|a| a.to_str()).unwrap_or("");
        if dizin {
            ad == "gizli"
        } else {
            ad == ".env" || ad.ends_with(".pem")
        }
    }

    #[test]
    fn gizli_dosya_ve_dizin_bulunur_gizli_dizinin_ici_listelenmez() {
        let d = tempfile::tempdir().unwrap();
        let k = d.path();
        std::fs::create_dir_all(k.join("a/b")).unwrap();
        std::fs::create_dir_all(k.join("gizli/ic")).unwrap();
        std::fs::write(k.join(".env"), "x").unwrap();
        std::fs::write(k.join("a/b/anahtar.pem"), "x").unwrap();
        std::fs::write(k.join("a/b/normal.txt"), "x").unwrap();
        std::fs::write(k.join("gizli/ic/.env"), "x").unwrap();
        // .gitignore kapsamındakiler de taranır.
        std::fs::write(k.join(".gitignore"), ".env\n").unwrap();
        let t = tara(k, &env_ya_da_gizli_dizin);
        assert_eq!(t.dosyalar, vec![k.join(".env"), k.join("a/b/anahtar.pem")]);
        assert_eq!(t.dizinler, vec![k.join("gizli")]);
        assert_eq!(t.sayi(), 3);
    }

    fn git_adi(ad: &OsStr) -> bool {
        ad.to_string_lossy().eq_ignore_ascii_case(".git")
    }

    #[test]
    fn tarama_her_derinlikte_git_girdilerini_toplar_icine_inmez() {
        let d = tempfile::tempdir().unwrap();
        let k = d.path();
        std::fs::create_dir_all(k.join(".git/objects/ab")).unwrap();
        std::fs::write(k.join(".git/objects/ab/cd"), "x").unwrap();
        std::fs::create_dir_all(k.join("alt")).unwrap();
        std::fs::write(k.join("alt/.git"), "gitdir: ../zz").unwrap();
        std::fs::create_dir_all(k.join("a/b/.GIT/hooks")).unwrap();
        // `.git`'in içine inilmez: içindeki gizli adlı dosya listelenmez.
        std::fs::write(k.join(".git/.env"), "x").unwrap();
        let beklenen = vec![
            GitGirdisi {
                yol: k.join(".git"),
                dizin: true,
                baglanti: false,
            },
            GitGirdisi {
                yol: k.join("a/b/.GIT"),
                dizin: true,
                baglanti: false,
            },
            GitGirdisi {
                yol: k.join("alt/.git"),
                dizin: false,
                baglanti: false,
            },
        ];
        let t = tara_ayrintili(k, &env_ya_da_gizli_dizin, &git_adi, &|_, _| false);
        assert_eq!(t.git, beklenen);
        assert!(t.dosyalar.is_empty(), "{:?}", t.dosyalar);
        assert_eq!(adli_girdiler(k, &git_adi, &|_, _| false).git, beklenen);
        // Git kuralı verilmezse eski davranış.
        assert!(tara(k, &env_ya_da_gizli_dizin).git.is_empty());
    }

    fn vscode_ya_da_envrc(yol: &Path, dizin: bool) -> bool {
        let ad = yol.file_name().and_then(|a| a.to_str()).unwrap_or("");
        if dizin {
            ad == ".vscode"
        } else {
            ad == ".envrc"
        }
    }

    #[test]
    fn yapilandirma_yollari_en_ustten_toplanir() {
        let d = tempfile::tempdir().unwrap();
        let k = d.path();
        std::fs::create_dir_all(k.join(".vscode/alt")).unwrap();
        std::fs::write(k.join(".vscode/tasks.json"), "{}").unwrap();
        std::fs::write(k.join(".vscode/alt/.envrc"), "x").unwrap();
        std::fs::create_dir_all(k.join("alt")).unwrap();
        std::fs::write(k.join("alt/.envrc"), "x").unwrap();
        // `.vscode/.env` gizlidir; yapılandırma dizininin içi de gizli
        // taraması için gezilir.
        std::fs::write(k.join(".vscode/.env"), "x").unwrap();
        let beklenen = vec![
            Yapilandirma {
                yol: k.join(".vscode"),
                dizin: true,
            },
            Yapilandirma {
                yol: k.join("alt/.envrc"),
                dizin: false,
            },
        ];
        let t = tara_ayrintili(k, &env_ya_da_gizli_dizin, &git_adi, &vscode_ya_da_envrc);
        assert_eq!(t.yapilandirma, beklenen);
        assert_eq!(t.dosyalar, vec![k.join(".vscode/.env")]);
        assert_eq!(
            adli_girdiler(k, &git_adi, &vscode_ya_da_envrc).yapilandirma,
            beklenen
        );
    }

    fn goruntu(k: &Path) -> Goruntu {
        adli_girdiler(k, &git_adi, &|_, _| false).goruntu
    }

    /// Zaman damgası çözünürlüğüne takılmamak için değişiklik zamanı açıkça
    /// ileri alınır.
    fn yaz_ileri(yol: &Path, icerik: &str, sn: u64) {
        std::fs::write(yol, icerik).unwrap();
        let f = std::fs::OpenOptions::new().write(true).open(yol).unwrap();
        f.set_modified(std::time::SystemTime::now() + Duration::from_secs(sn))
            .unwrap();
    }

    fn degisiklikler(once: &Goruntu, sonra: &Goruntu) -> Vec<(String, DegisiklikTuru)> {
        karsilastir(once, sonra)
            .into_iter()
            .map(|d| (d.yol, d.tur))
            .collect()
    }

    #[test]
    fn goruntu_eklenen_silinen_degisen_bulur() {
        let d = tempfile::tempdir().unwrap();
        let k = d.path();
        std::fs::create_dir_all(k.join("src")).unwrap();
        std::fs::write(k.join("src/a.rs"), "a").unwrap();
        std::fs::write(k.join("src/b.rs"), "b").unwrap();
        std::fs::write(k.join("ayni.txt"), "x").unwrap();
        let once = goruntu(k);
        assert!(!once.eksik);
        assert_eq!(once.girdiler.len(), 3, "{:?}", once.girdiler);
        yaz_ileri(&k.join("src/a.rs"), "aa", 10);
        std::fs::remove_file(k.join("src/b.rs")).unwrap();
        std::fs::write(k.join("src/c.rs"), "c").unwrap();
        let sonra = goruntu(k);
        assert_eq!(
            degisiklikler(&once, &sonra),
            vec![
                ("src/a.rs".into(), DegisiklikTuru::Degisti),
                ("src/b.rs".into(), DegisiklikTuru::Silindi),
                ("src/c.rs".into(), DegisiklikTuru::Eklendi),
            ]
        );
        // Değişmeyen görüntünün farkı boştur.
        assert!(karsilastir(&sonra, &goruntu(k)).is_empty());
    }

    /// Git'e göre önceden değişmiş (`M`) dosyanın yeniden değişmesi de bulunur:
    /// karşılaştırma git'e değil komuttan önceki görüntüye göredir.
    #[test]
    fn onceden_degismis_dosyanin_yeniden_degismesi_bulunur() {
        let d = tempfile::tempdir().unwrap();
        let k = d.path();
        std::fs::create_dir(k.join(".git")).unwrap();
        yaz_ileri(&k.join("a.rs"), "kullanicinin degisikligi", 5);
        let once = goruntu(k);
        // Boyut aynı, yalnızca zaman değişiyor.
        yaz_ileri(&k.join("a.rs"), "komutun degisikligi!!!!!", 20);
        assert_eq!(
            degisiklikler(&once, &goruntu(k)),
            vec![("a.rs".into(), DegisiklikTuru::Degisti)]
        );
    }

    #[test]
    fn gitignore_kapsamindaki_dosya_bulunur() {
        let d = tempfile::tempdir().unwrap();
        let k = d.path();
        std::fs::write(k.join(".gitignore"), "target/\n*.log\n").unwrap();
        let once = goruntu(k);
        std::fs::create_dir_all(k.join("target/debug")).unwrap();
        std::fs::write(k.join("target/debug/x"), "x").unwrap();
        std::fs::write(k.join("derleme.log"), "x").unwrap();
        assert_eq!(
            degisiklikler(&once, &goruntu(k)),
            vec![
                ("derleme.log".into(), DegisiklikTuru::Eklendi),
                ("target/debug/x".into(), DegisiklikTuru::Eklendi),
            ]
        );
    }

    #[test]
    fn git_ici_goruntuye_girmez() {
        let d = tempfile::tempdir().unwrap();
        let k = d.path();
        std::fs::create_dir_all(k.join(".git/objects")).unwrap();
        std::fs::create_dir_all(k.join("alt")).unwrap();
        std::fs::write(k.join("alt/.git"), "gitdir: ../zz").unwrap();
        std::fs::write(k.join("a.txt"), "x").unwrap();
        let once = goruntu(k);
        std::fs::write(k.join(".git/objects/ab"), "x").unwrap();
        std::fs::write(k.join(".git/index"), "x").unwrap();
        let sonra = goruntu(k);
        assert_eq!(sonra.girdiler.keys().collect::<Vec<_>>(), vec!["a.txt"]);
        assert!(karsilastir(&once, &sonra).is_empty());
        // Ayrıntılı tarama da aynı görüntüyü toplar.
        let t = tara_ayrintili(k, &env_ya_da_gizli_dizin, &git_adi, &|_, _| false);
        assert_eq!(t.goruntu, sonra);
    }

    #[test]
    fn sinir_asilinca_goruntu_eksik() {
        let d = tempfile::tempdir().unwrap();
        let k = d.path();
        for i in 0..10 {
            std::fs::write(k.join(format!("{i}.txt")), "x").unwrap();
        }
        let sinirli = |girdi, sure| {
            adli_girdiler_sinirli(k, &git_adi, &|_, _| false, GoruntuSiniri { girdi, sure }).goruntu
        };
        let g = sinirli(4, GORUNTU_AZAMI_SURE);
        assert!(g.eksik);
        assert!(g.girdiler.len() <= 4, "{}", g.girdiler.len());
        assert!(sinirli(4, Duration::ZERO).eksik);
        let tam = sinirli(10, GORUNTU_AZAMI_SURE);
        assert!(!tam.eksik);
        assert_eq!(tam.girdiler.len(), 10);
    }

    #[cfg(unix)]
    #[test]
    fn git_baglantisi_toplanir() {
        let d = tempfile::tempdir().unwrap();
        let k = d.path();
        std::fs::create_dir_all(k.join("zz")).unwrap();
        std::os::unix::fs::symlink(k.join("zz"), k.join(".git")).unwrap();
        assert_eq!(
            adli_girdiler(k, &git_adi, &|_, _| false).git,
            vec![GitGirdisi {
                yol: k.join(".git"),
                dizin: false,
                baglanti: true,
            }]
        );
    }

    #[cfg(unix)]
    #[test]
    fn hard_link_ve_sembolik_baglanti() {
        let d = tempfile::tempdir().unwrap();
        let k = d.path();
        std::fs::create_dir_all(k.join("gizli")).unwrap();
        std::fs::write(k.join(".env"), "x").unwrap();
        std::fs::hard_link(k.join(".env"), k.join("a.txt")).unwrap();
        std::fs::write(k.join("gizli/sir"), "x").unwrap();
        std::fs::hard_link(k.join("gizli/sir"), k.join("b.txt")).unwrap();
        std::fs::write(k.join("c.txt"), "x").unwrap();
        std::fs::hard_link(k.join("c.txt"), k.join("d.txt")).unwrap();
        // Bağlantı izlenmez ve kendisi listelenmez.
        std::os::unix::fs::symlink(k.join(".env"), k.join("bag.env")).unwrap();
        let t = tara(k, &env_ya_da_gizli_dizin);
        assert_eq!(
            t.dosyalar,
            vec![k.join(".env"), k.join("a.txt"), k.join("b.txt")]
        );
        assert_eq!(t.dizinler, vec![k.join("gizli")]);
    }
}
