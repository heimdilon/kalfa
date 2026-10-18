//! Sağ panelin salt okur çalışma alanı görünümü (masaüstü arayüzü tasarımı
//! §6.2, §6.3). Arayüz yol göndermez: dizinler ve dosyalar oturuma bağlı
//! kimliklerle anılır; kimlik tablosu oturumla birlikte silinir. Açma
//! işlerini oturumun açılışta aldığı çalışma alanı tutamacı yapar:
//! bileşenler bağlantı izlenmeden açılır, gizli ve başka adı olan dosyanın
//! içeriği dönmez.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use kalfa_core::gecmis::{self, GorunumOgesi};
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::OturumId;
use kalfa_core::yol::CalismaYolu;
use kalfa_tools::{CalismaAlani as AracAlani, GirdiTuru, Onizleme, baglantisiz_oku};
use serde::Serialize;

use crate::hata::KomutHatasi;
use crate::{Uygulama, dizin};

/// Bir dizinde listelenen en çok girdi.
pub const DIZIN_SINIRI: usize = 2000;
/// Önizlenen en çok bayt.
pub const ONIZLEME_SINIRI: usize = 256 * 1024;
/// `.git` dosyası ve `HEAD` için okunan en çok bayt.
const GIT_SINIRI: u64 = 4096;

/// Kimlikler bütün oturumlarda tektir: başka oturumun kimliği bu oturumun
/// tablosunda bulunmaz.
static SONRAKI_KIMLIK: AtomicU64 = AtomicU64::new(1);

pub(crate) struct DosyaGorunumu {
    /// Oturumun araçlarıyla aynı tutamaç: yol sonradan değiştirilse de
    /// görünüm seçilen çalışma alanında kalır.
    alan: Arc<AracAlani>,
    kimlikler: Mutex<Kimlikler>,
}

#[derive(Default)]
struct Kimlikler {
    yollar: HashMap<u64, CalismaYolu>,
    kimlikler: HashMap<CalismaYolu, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DosyaGirdisi {
    pub kimlik: u64,
    pub ad: String,
    pub tur: &'static str,
    pub boyut: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DizinIcerigi {
    pub girdiler: Vec<DosyaGirdisi>,
    /// Dizinde `DIZIN_SINIRI`'ndan çok girdi var.
    pub kirpildi: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum DosyaIcerigi {
    Metin { metin: String, kirpildi: bool },
    Ikili,
    Gizli,
    CokAdli,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum GitDurumu {
    Dal { ad: String },
    AyrikHead,
    GitDegil,
}

/// Oturumda değişen bir dosya: geri alınmamış turların toplamı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CiktiDosyasi {
    pub yol: String,
    pub eklenen: u32,
    pub silinen: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CalismaAlaniOzeti {
    /// Ev dizini `~` ile kısaltılmış yol.
    pub kisa_yol: String,
    pub git: GitDurumu,
    /// Bütün kayıttan, yol sırasıyla; arayüzün yüklediği geçmiş sayfasından
    /// bağımsızdır.
    pub ciktilar: Vec<CiktiDosyasi>,
    /// Bağlam yönetiminin özetlediği araç çıktısı sayısı (5a).
    pub ozetlenen: u32,
}

fn ic(e: impl std::fmt::Display) -> KomutHatasi {
    KomutHatasi::ic(e.to_string())
}

fn okuma_hatasi(e: &io::Error) -> KomutHatasi {
    KomutHatasi::gecersiz(format!("Okunamadı: {e}"))
}

impl Uygulama {
    /// Oturumun çalışma alanı ve görünümü; açık değilse `OturumYok`.
    fn gorunum(&self, oturum: OturumId) -> Result<(PathBuf, Arc<DosyaGorunumu>), KomutHatasi> {
        self.ic()
            .oturumlar
            .get(&oturum)
            .map(|a| (a.calisma_alani.clone(), a.gorunum.clone()))
            .ok_or_else(KomutHatasi::oturum_yok)
    }

    pub async fn calisma_alani_ozeti(
        &self,
        oturum: OturumId,
    ) -> Result<CalismaAlaniOzeti, KomutHatasi> {
        let (kok, gorunum) = self.gorunum(oturum)?;
        let ev = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" });
        let kayit = dizin::kayit_yolu(&self.oturum_dizini, oturum);
        tokio::task::spawn_blocking(move || -> io::Result<CalismaAlaniOzeti> {
            let (ciktilar, ozetlenen) = kayit_toplamlari(&OturumKaydi::oku(&kayit)?);
            Ok(CalismaAlaniOzeti {
                kisa_yol: kisa_yol(&kok, ev.as_deref().map(Path::new)),
                git: git_durumu(&gorunum.alan, &kok),
                ciktilar,
                ozetlenen,
            })
        })
        .await
        .map_err(ic)?
        .map_err(KomutHatasi::kayit)
    }

    /// Dizinin girdileri; `dizin` yoksa çalışma alanının kökü.
    pub async fn calisma_alani_dizini(
        &self,
        oturum: OturumId,
        dizin: Option<u64>,
    ) -> Result<DizinIcerigi, KomutHatasi> {
        let (_, gorunum) = self.gorunum(oturum)?;
        let ust = dizin.map(|k| gorunum.yol(k)).transpose()?;
        let alan = gorunum.alan.clone();
        let (ust, girdiler, kirpildi) = tokio::task::spawn_blocking(move || {
            alan.dizini_listele(ust.as_ref(), DIZIN_SINIRI)
                .map(|(g, k)| (ust, g, k))
        })
        .await
        .map_err(ic)?
        .map_err(|e| okuma_hatasi(&e))?;
        Ok(DizinIcerigi {
            girdiler: gorunum.kaydet(ust.as_ref(), girdiler),
            kirpildi,
        })
    }

    pub async fn calisma_alani_dosyasi(
        &self,
        oturum: OturumId,
        kimlik: u64,
    ) -> Result<DosyaIcerigi, KomutHatasi> {
        let (_, gorunum) = self.gorunum(oturum)?;
        let yol = gorunum.yol(kimlik)?;
        let alan = gorunum.alan.clone();
        let onizleme = tokio::task::spawn_blocking(move || alan.onizle(&yol, ONIZLEME_SINIRI))
            .await
            .map_err(ic)?
            .map_err(|e| okuma_hatasi(&e))?;
        Ok(match onizleme {
            Onizleme::Metin { metin, kirpildi } => DosyaIcerigi::Metin { metin, kirpildi },
            Onizleme::Ikili => DosyaIcerigi::Ikili,
            Onizleme::Gizli => DosyaIcerigi::Gizli,
            Onizleme::CokAdli => DosyaIcerigi::CokAdli,
        })
    }
}

impl DosyaGorunumu {
    pub(crate) fn yeni(alan: Arc<AracAlani>) -> Self {
        Self {
            alan,
            kimlikler: Mutex::default(),
        }
    }

    fn yol(&self, kimlik: u64) -> Result<CalismaYolu, KomutHatasi> {
        self.kimlikler
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .yollar
            .get(&kimlik)
            .cloned()
            .ok_or_else(|| KomutHatasi::gecersiz("Bu dosya bu oturumda bilinmiyor."))
    }

    /// Girdilere kimlik verir; aynı yol aynı kimliği korur.
    fn kaydet(
        &self,
        ust: Option<&CalismaYolu>,
        girdiler: Vec<kalfa_tools::DizinGirdisi>,
    ) -> Vec<DosyaGirdisi> {
        let mut tablo = self.kimlikler.lock().unwrap_or_else(|z| z.into_inner());
        girdiler
            .into_iter()
            .filter_map(|g| {
                let goreli =
                    ust.map_or_else(|| g.ad.clone(), |u| format!("{}/{}", u.as_str(), g.ad));
                let yol = CalismaYolu::coz(&goreli).ok()?;
                let kimlik = match tablo.kimlikler.get(&yol) {
                    Some(k) => *k,
                    None => {
                        let k = SONRAKI_KIMLIK.fetch_add(1, Ordering::Relaxed);
                        tablo.kimlikler.insert(yol.clone(), k);
                        tablo.yollar.insert(k, yol);
                        k
                    }
                };
                Some(DosyaGirdisi {
                    kimlik,
                    ad: g.ad,
                    tur: match g.tur {
                        GirdiTuru::Dizin => "Dizin",
                        GirdiTuru::Dosya => "Dosya",
                        GirdiTuru::Baglanti => "Baglanti",
                    },
                    boyut: g.boyut,
                })
            })
            .collect()
    }
}

/// Bütün kayıttan değişen dosyalar (geri alınmamış turlar) ve özetlenen araç
/// çıktısı sayısı.
fn kayit_toplamlari(
    kayitlar: &[kalfa_core::protokol::Zarf<KayitOgesi>],
) -> (Vec<CiktiDosyasi>, u32) {
    let ogeler = gecmis::gorunum_uret(kayitlar);
    let geri_alinan: HashSet<_> = ogeler
        .iter()
        .filter_map(|o| match o {
            GorunumOgesi::GeriAlma {
                geri_alinan_tur,
                yarim: false,
                ..
            } => Some(*geri_alinan_tur),
            _ => None,
        })
        .collect();
    let mut dosyalar: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    for o in &ogeler {
        if let GorunumOgesi::TurOzeti { ozet, .. } = o
            && !geri_alinan.contains(&ozet.tur)
        {
            for d in &ozet.dosyalar {
                let t = dosyalar.entry(d.yol.as_str().to_string()).or_default();
                *t = (t.0 + d.eklenen, t.1 + d.silinen);
            }
        }
    }
    let ozetlenen: HashSet<u64> = kayitlar
        .iter()
        .filter_map(|z| match &z.yuk {
            KayitOgesi::BaglamOzeti { mesaj_sira, .. } => Some(*mesaj_sira),
            _ => None,
        })
        .collect();
    let ciktilar = dosyalar
        .into_iter()
        .map(|(yol, (eklenen, silinen))| CiktiDosyasi {
            yol,
            eklenen,
            silinen,
        })
        .collect();
    (ciktilar, u32::try_from(ozetlenen.len()).unwrap_or(u32::MAX))
}

/// Ev dizininin altındaysa `~` ile başlar.
fn kisa_yol(kok: &Path, ev: Option<&Path>) -> String {
    match ev.and_then(|e| kok.strip_prefix(e).ok()) {
        Some(alt) if alt.as_os_str().is_empty() => "~".into(),
        Some(alt) => format!("~{}{}", std::path::MAIN_SEPARATOR, alt.display()),
        None => kok.display().to_string(),
    }
}

/// `.git/HEAD`'den dal; git süreci başlatılmaz. Çalışma alanının içindeki
/// `.git` oturumun tutamacından okunur. `.git` dosyaysa (worktree,
/// alt modül) `gitdir:`'in gösterdiği HEAD kökten başlayarak tutamaçlarla,
/// bağlantı izlenmeden okunur; Windows'ta ağ yolu açılmaz (`.git`'i
/// çalışma alanına kim yazdıysa onu seçer).
pub(crate) fn git_durumu(alan: &AracAlani, kok: &Path) -> GitDurumu {
    let yol = |s: &str| CalismaYolu::coz(s).ok();
    let head = match yol(".git/HEAD").and_then(|y| alan.oku(&y).ok()) {
        Some(head) => head,
        None => {
            let Some(git) = yol(".git").and_then(|y| alan.oku(&y).ok()) else {
                return GitDurumu::GitDegil;
            };
            let git =
                String::from_utf8_lossy(&git[..git.len().min(GIT_SINIRI as usize)]).into_owned();
            let Some(gitdir) = git
                .trim()
                .strip_prefix("gitdir:")
                .map(|d| Path::new(d.trim()))
            else {
                return GitDurumu::GitDegil;
            };
            let Some(head) = sozcuksel(&kok.join(gitdir).join("HEAD")) else {
                return GitDurumu::GitDegil;
            };
            match baglantisiz_oku(&head, GIT_SINIRI) {
                Ok(head) => head,
                Err(_) => return GitDurumu::GitDegil,
            }
        }
    };
    let head = String::from_utf8_lossy(&head);
    match head.trim().strip_prefix("ref:") {
        Some(r) => {
            let r = r.trim();
            GitDurumu::Dal {
                ad: r.strip_prefix("refs/heads/").unwrap_or(r).to_string(),
            }
        }
        None => GitDurumu::AyrikHead,
    }
}

/// `.` ve `..` sözcüksel olarak çözülmüş yol; kökün üstüne çıkarsa `None`.
/// Bağlantılar zaten izlenmediği için sözcüksel çözüm fiziksel olanla aynıdır.
fn sozcuksel(yol: &Path) -> Option<PathBuf> {
    let mut sonuc = PathBuf::new();
    for parca in yol.components() {
        match parca {
            Component::CurDir => {}
            // Kökte `pop` başarısız olur: kökün üstü yoktur.
            Component::ParentDir => {
                if !sonuc.pop() {
                    return None;
                }
            }
            _ => sonuc.push(parca),
        }
    }
    Some(sonuc)
}

#[cfg(test)]
mod testler {
    use super::*;
    use kalfa_core::izin::Siniflandirici;

    fn alan(kok: &Path) -> Arc<AracAlani> {
        AracAlani::ac(Arc::new(Siniflandirici::yeni(kok, &[]).unwrap())).unwrap()
    }

    fn durum(kok: &Path) -> GitDurumu {
        git_durumu(&alan(kok), kok)
    }

    #[test]
    fn git_dali_ayrik_head_ve_worktree() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(durum(d.path()), GitDurumu::GitDegil);

        std::fs::create_dir(d.path().join(".git")).unwrap();
        std::fs::write(d.path().join(".git/HEAD"), "ref: refs/heads/ui/sag-panel\n").unwrap();
        assert_eq!(
            durum(d.path()),
            GitDurumu::Dal {
                ad: "ui/sag-panel".into()
            }
        );
        std::fs::write(
            d.path().join(".git/HEAD"),
            "90e729d0123456789abcdef0123456789abcdef0\n",
        )
        .unwrap();
        assert_eq!(durum(d.path()), GitDurumu::AyrikHead);

        let w = tempfile::tempdir().unwrap();
        let gitdir = d.path().join(".git");
        std::fs::write(d.path().join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(
            w.path().join(".git"),
            format!("gitdir: {}\n", gitdir.display()),
        )
        .unwrap();
        assert_eq!(durum(w.path()), GitDurumu::Dal { ad: "main".into() });

        // Göreli gitdir de okunur.
        std::fs::create_dir_all(w.path().join("modul/.git-dizin")).unwrap();
        std::fs::write(
            w.path().join("modul/.git-dizin/HEAD"),
            "ref: refs/heads/alt\n",
        )
        .unwrap();
        std::fs::write(w.path().join(".git"), "gitdir: modul/.git-dizin\n").unwrap();
        assert_eq!(durum(w.path()), GitDurumu::Dal { ad: "alt".into() });
    }

    /// Aynı yerel gitdir'e `\\?\` ya da UNC biçimiyle gidilmez: Windows'ta
    /// bu yolları açmak ağa kimlik bilgisi gönderebilir.
    #[cfg(windows)]
    #[test]
    fn ag_ve_ozel_onekli_gitdir_acilmaz() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir(d.path().join("gd")).unwrap();
        std::fs::write(d.path().join("gd/HEAD"), "ref: refs/heads/main\n").unwrap();
        let w = tempfile::tempdir().unwrap();
        let yerel = d.path().join("gd");
        std::fs::write(
            w.path().join(".git"),
            format!("gitdir: {}\n", yerel.display()),
        )
        .unwrap();
        assert_eq!(durum(w.path()), GitDurumu::Dal { ad: "main".into() });

        let ozel = format!(r"\\?\{}", yerel.display());
        std::fs::write(w.path().join(".git"), format!("gitdir: {ozel}\n")).unwrap();
        assert!(Path::new(&ozel).join("HEAD").exists(), "{ozel}");
        assert_eq!(durum(w.path()), GitDurumu::GitDegil);

        for ag in [r"\\127.0.0.1\pay\gd", "//127.0.0.1/pay/gd"] {
            std::fs::write(w.path().join(".git"), format!("gitdir: {ag}\n")).unwrap();
            assert_eq!(durum(w.path()), GitDurumu::GitDegil, "{ag}");
        }
    }

    /// gitdir yolundaki bağlantı ve bağlantı HEAD izlenmez: hedefi ağ yolu
    /// olabilir. Hedef aynı dizinde: izlenseydi okuma başarırdı.
    #[test]
    fn gitdir_yolundaki_baglanti_izlenmez() {
        let w = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(w.path().join("gercek/gd")).unwrap();
        std::fs::write(w.path().join("gercek/gd/HEAD"), "ref: refs/heads/dis\n").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("gercek", w.path().join("bag")).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir("gercek", w.path().join("bag"))
            .expect("Windows test ajanında symlink yetkisi gerekir");
        assert!(w.path().join("bag/gd/HEAD").exists());
        std::fs::write(w.path().join(".git"), "gitdir: bag/gd\n").unwrap();
        assert_eq!(durum(w.path()), GitDurumu::GitDegil);
        std::fs::write(w.path().join(".git"), "gitdir: gercek/gd\n").unwrap();
        assert_eq!(durum(w.path()), GitDurumu::Dal { ad: "dis".into() });
    }

    /// Toplamlar bütün kayıttandır: geri alınan turun dosyası düşer, aynı
    /// dosya turlar boyunca toplanır, aynı mesajın özeti bir kez sayılır.
    #[test]
    fn kayit_toplamlari_geri_alinani_dusurur() {
        use kalfa_core::kayit::OzetYontemi;
        use kalfa_core::kimlik::TurId;
        use kalfa_core::protokol::{SEMA_SURUMU, Zarf};
        use kalfa_core::tur_ozeti::{
            DosyaIslemi, DosyaOzeti, GeriAlinabilirlik, IstekOzeti, TurOzeti,
        };

        let oturum = OturumId::yeni();
        let zarf = |sira: u64, yuk: KayitOgesi| Zarf {
            sema: SEMA_SURUMU,
            oturum,
            tur: None,
            sira,
            yuk,
        };
        let ozet = |tur: TurId, dosyalar: &[(&str, u32, u32)]| KayitOgesi::TurOzeti {
            ozet: TurOzeti {
                tur,
                dosyalar: dosyalar
                    .iter()
                    .map(|(yol, eklenen, silinen)| DosyaOzeti {
                        yol: CalismaYolu::coz(yol).unwrap(),
                        islem: DosyaIslemi::Degisti,
                        eklenen: *eklenen,
                        silinen: *silinen,
                        sonucu_bilinmiyor: false,
                        geri_alinabilir: GeriAlinabilirlik::Evet,
                        diff: None,
                    })
                    .collect(),
                komutlar: vec![],
                istekler: IstekOzeti {
                    sayi: 1,
                    istem: 1,
                    tamamlama: 1,
                    dusunce: 0,
                },
                guvenlik_uyarisi: 0,
            },
        };
        let baglam_ozeti = |mesaj_sira| KayitOgesi::BaglamOzeti {
            mesaj_sira,
            kaynak: "dosya_oku".into(),
            icerik: "özet".into(),
            yontem: OzetYontemi::Ozet,
        };
        let (t1, t2, t3) = (TurId::yeni(), TurId::yeni(), TurId::yeni());
        let kayitlar = vec![
            zarf(1, ozet(t1, &[("a.py", 2, 1), ("b.py", 1, 0)])),
            zarf(2, ozet(t2, &[("a.py", 3, 0)])),
            zarf(3, ozet(t3, &[("c.py", 5, 0)])),
            zarf(
                4,
                KayitOgesi::GeriAlindi {
                    geri_alinan_tur: t3,
                    sonuc: vec![],
                },
            ),
            zarf(5, baglam_ozeti(7)),
            zarf(6, baglam_ozeti(7)),
            zarf(7, baglam_ozeti(9)),
        ];
        let (ciktilar, ozetlenen) = kayit_toplamlari(&kayitlar);
        assert_eq!(
            ciktilar,
            vec![
                CiktiDosyasi {
                    yol: "a.py".into(),
                    eklenen: 5,
                    silinen: 1
                },
                CiktiDosyasi {
                    yol: "b.py".into(),
                    eklenen: 1,
                    silinen: 0
                },
            ]
        );
        assert_eq!(ozetlenen, 2);
    }

    #[test]
    fn sozcuksel_cozum() {
        let kok = if cfg!(windows) { r"C:\a" } else { "/a" };
        let k = Path::new(kok);
        assert_eq!(
            sozcuksel(&k.join("b/../c/./d")),
            Some(k.join("c").join("d"))
        );
        assert_eq!(sozcuksel(&k.join("../../..")), None);
    }

    #[test]
    fn ev_dizini_kisaltilir() {
        let ev = Path::new("/ev/orkun");
        assert_eq!(kisa_yol(Path::new("/ev/orkun"), Some(ev)), "~");
        assert_eq!(
            kisa_yol(Path::new("/ev/orkun/proje"), Some(ev)),
            format!("~{}proje", std::path::MAIN_SEPARATOR)
        );
        assert_eq!(
            kisa_yol(Path::new("/baska/proje"), Some(ev)),
            "/baska/proje"
        );
        assert_eq!(kisa_yol(Path::new("/baska/proje"), None), "/baska/proje");
    }
}
