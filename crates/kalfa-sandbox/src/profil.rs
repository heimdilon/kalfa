//! Yalıtım profilinin işletim sisteminden bağımsız parçaları (aşama 2
//! tasarımı §7): çalışma alanının yalıtıma giren bilgisi, çalışma alanına özel
//! dizinler ve araç yapılandırması, profil özeti.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::tarama::Tarama;

/// Bir çalıştırmanın çalışma alanı bilgisi. Araç her çalıştırmadan hemen önce
/// üretir; yalıtımsız arka uç kullanmaz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlanBilgisi {
    /// Çalışma alanının kanonik kökü; yalıtımda yazılabilir.
    pub kok: PathBuf,
    /// Okunamayacak dosyalar ve dizinler (§7.2).
    pub gizli: Tarama,
    /// Salt okunur kalacak yollar: `.git`'in kendisi ve `core.hooksPath`
    /// hedefi (§7.2).
    pub korumali: Vec<Korumali>,
    /// Çalışma alanı dışındaki, doğrulanmış git dizinleri (worktree'nin ortak
    /// git dizini, alt modülün git dizini); salt okunur açılır (§7.4).
    pub git_dizinleri: Vec<PathBuf>,
}

/// Salt okunur kalacak bir yol. Yol yoksa yalıtım onu boş olarak oluşturur;
/// yoksa komut, örneğin eksik `.git/hooks` dizinine kanca yazabilirdi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Korumali {
    pub yol: PathBuf,
    pub dizin: bool,
}

/// Çalışma alanının kimliği: kanonik yolunun SHA-256'sından 16 onaltılık
/// karakter (§7.3). Çalışma alanı kaydı da aynı kimliği kullanır.
pub fn alan_kimligi(kok: &Path) -> String {
    let kanonik = std::fs::canonicalize(kok).unwrap_or_else(|_| kok.to_path_buf());
    let ozet = Sha256::digest(kanonik.to_string_lossy().as_bytes());
    hex::encode(ozet)[..16].to_string()
}

/// Çalışma alanına özel, yalıtımda yazılabilir dizinler (§7.3). Ortak
/// yazılabilir durum yoktur: her çalışma alanının kendi ev, geçici ve
/// önbellek dizini vardır.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OzelDizinler {
    pub kok: PathBuf,
    pub ev: PathBuf,
    pub gecici: PathBuf,
    pub onbellek: PathBuf,
}

/// Kullanıcının git kimliği; yalıtımdaki `.gitconfig`'e yalnızca bu iki değer
/// girer (§7.4).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitKimligi {
    pub ad: Option<String>,
    pub eposta: Option<String>,
}

impl GitKimligi {
    /// Kullanıcının genel git yapılandırmasından okur. Depo yapılandırması
    /// yalıtımın içinde zaten okunur.
    pub fn oku() -> Self {
        let deger = |anahtar: &str| {
            let cikti = Command::new("git")
                .args(["config", "--global", "--get", anahtar])
                .output()
                .ok()?;
            let metin = String::from_utf8_lossy(&cikti.stdout).trim().to_string();
            (cikti.status.success() && !metin.is_empty()).then_some(metin)
        };
        Self {
            ad: deger("user.name"),
            eposta: deger("user.email"),
        }
    }

    fn gitconfig(&self) -> String {
        let mut metin = String::from("[user]\n");
        for (anahtar, deger) in [("name", &self.ad), ("email", &self.eposta)] {
            if let Some(d) = deger {
                metin.push_str(&format!("\t{anahtar} = {}\n", git_degeri(d)));
            }
        }
        metin
    }
}

/// Değeri git yapılandırmasında tırnaklı yazar; satır sonları atılır.
fn git_degeri(d: &str) -> String {
    let temiz: String = d
        .chars()
        .filter(|c| *c != '\n' && *c != '\r')
        .flat_map(|c| match c {
            '\\' => vec!['\\', '\\'],
            '"' => vec!['\\', '"'],
            c => vec![c],
        })
        .collect();
    format!("\"{temiz}\"")
}

/// Windows'ta kabın `ev`'indeki `.curlrc`.
const CURLRC: &str = "ssl-revoke-best-effort\n";

impl OzelDizinler {
    /// `yalitim_dizini/<kimlik>/` altındaki dizinler; oluşturmaz.
    pub fn yol(yalitim_dizini: &Path, kimlik: &str) -> Self {
        let kok = yalitim_dizini.join(kimlik);
        Self {
            ev: kok.join("ev"),
            gecici: kok.join("gecici"),
            onbellek: kok.join("onbellek"),
            kok,
        }
    }

    pub fn cargo(&self) -> PathBuf {
        self.onbellek.join("cargo")
    }

    pub fn npm(&self) -> PathBuf {
        self.onbellek.join("npm")
    }

    pub fn pip(&self) -> PathBuf {
        self.onbellek.join("pip")
    }

    pub fn gitconfig(&self) -> PathBuf {
        self.ev.join(".gitconfig")
    }

    pub fn npmrc(&self) -> PathBuf {
        self.ev.join(".npmrc")
    }

    /// Dizinleri oluşturur ve en küçük araç yapılandırmasını yazar (§7.4).
    /// Her çalıştırmadan önce çağrılır; kullanıcının git kimliği değiştiyse
    /// yeni hâli yazılır. `.npmrc` boştur: kullanıcınınki token taşıyabilir.
    /// Windows'ta `.curlrc` de yazılır.
    ///
    /// Bu dizinlere yalıtımdaki komut yazabilir. Kalfa'nın yazması bu yüzden
    /// bağlantı izlemez: komut `ev/.gitconfig`'i ya da `ev`'i dışarıya bir
    /// bağlantıya (Windows'ta junction) çevirmiş olabilir; izlenseydi Kalfa
    /// geliştiricinin kendi dosyasının üzerine yazardı. Bağlantı silinir,
    /// yerine dizin ya da dosya konur.
    pub fn hazirla(&self, git: &GitKimligi) -> io::Result<()> {
        std::fs::create_dir_all(&self.kok)?;
        for goreli in [
            Path::new("ev"),
            Path::new("gecici"),
            Path::new("onbellek/cargo/registry"),
            Path::new("onbellek/npm"),
            Path::new("onbellek/pip"),
        ] {
            guvenli_dizin(&self.kok, goreli)?;
        }
        guvenli_yaz(&self.ev, ".gitconfig", git.gitconfig().as_bytes())?;
        guvenli_yaz(&self.ev, ".npmrc", b"")?;
        // Windows'ta curl Schannel'dır; iptal bilgisi kapta alınamadığında
        // bağlanır, bilgi varsa (stapling) iptal edilmiş sertifikayı yine
        // reddeder (3d iptal bulguları).
        if cfg!(windows) {
            guvenli_yaz(&self.ev, ".curlrc", CURLRC.as_bytes())?;
        }
        Ok(())
    }
}

/// Bağlantıyı (sembolik bağlantı ya da junction) siler; hedefine dokunmaz.
fn baglantiyi_sil(yol: &Path) -> io::Result<()> {
    // Windows'ta dizine giden bağlantı `remove_dir` ile, dosyaya giden
    // `remove_file` ile silinir; Unix'te ikisi de `remove_file`'dır.
    std::fs::remove_file(yol).or_else(|_| std::fs::remove_dir(yol))
}

/// `taban/goreli`'yi bileşen bileşen, bağlantı izlemeden dizin olarak kurar.
/// `taban` yalıtımın değiştiremediği bir dizindir. Yolda bağlantı ya da dosya
/// varsa silinir.
pub(crate) fn guvenli_dizin(taban: &Path, goreli: &Path) -> io::Result<PathBuf> {
    let mut yol = taban.to_path_buf();
    for bilesen in goreli.components() {
        yol.push(bilesen);
        match std::fs::symlink_metadata(&yol) {
            Ok(m) if m.file_type().is_symlink() => {
                baglantiyi_sil(&yol)?;
                std::fs::create_dir(&yol)?;
            }
            Ok(m) if m.is_dir() => {}
            Ok(_) => {
                std::fs::remove_file(&yol)?;
                std::fs::create_dir(&yol)?;
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => std::fs::create_dir(&yol)?,
            Err(e) => return Err(e),
        }
    }
    Ok(yol)
}

/// `dizin/ad`'ı bağlantı izlemeden yazar: önce yeni bir geçici dosyaya, sonra
/// yerine taşır. Yerindeki bağlantı ya da dizin silinir.
pub(crate) fn guvenli_yaz(dizin: &Path, ad: &str, icerik: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let hedef = dizin.join(ad);
    let gecici = dizin.join(format!("{ad}.kalfa-yeni"));
    for yol in [&hedef, &gecici] {
        match std::fs::symlink_metadata(yol) {
            Ok(m) if m.file_type().is_symlink() => baglantiyi_sil(yol)?,
            Ok(m) if m.is_dir() => std::fs::remove_dir_all(yol)?,
            Ok(_) if yol == &gecici => std::fs::remove_file(yol)?,
            _ => {}
        }
    }
    let mut dosya = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&gecici)?;
    dosya.write_all(icerik)?;
    drop(dosya);
    std::fs::rename(&gecici, &hedef)
}

/// Profil özeti (§7.1): yalıtımın arka ucu, okunur yolların sıralı listesi,
/// ağ bayrağı ve ağ açıkken izin listesi özetinin SHA-256'sı. Mühre girer:
/// kullanıcı bir dizin eklerse veya izin listesi değişirse bekleyen
/// onaylar geçersizleşir. Gizli ve korumalı yollar her çalıştırmadan hemen önce
/// tarandığı için özete girmez; onları belirleyen kurallar sabittir.
pub fn ozet(arka_uc: &str, okunur: &[PathBuf], ag: bool, liste_ozeti: Option<&str>) -> String {
    let mut yollar: Vec<String> = okunur.iter().map(|y| y.display().to_string()).collect();
    yollar.sort();
    let mut h = Sha256::new();
    h.update(arka_uc.as_bytes());
    h.update([0]);
    for yol in &yollar {
        h.update(yol.as_bytes());
        h.update([0]);
    }
    h.update(if ag { "ag:acik" } else { "ag:kapali" }.as_bytes());
    if let (true, Some(lo)) = (ag, liste_ozeti) {
        h.update([0]);
        h.update(lo.as_bytes());
    }
    hex::encode(h.finalize())[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kimlik_kanonik_yoldan_turer_ve_16_karakterdir() {
        let d = tempfile::tempdir().unwrap();
        let k = alan_kimligi(d.path());
        assert_eq!(k.len(), 16);
        assert!(k.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(k, alan_kimligi(&d.path().join(".")));
    }

    #[test]
    fn ozel_dizinler_hazirlanir_ve_gitconfig_yalnizca_kimligi_tasir() {
        let d = tempfile::tempdir().unwrap();
        let o = OzelDizinler::yol(d.path(), "abc");
        o.hazirla(&GitKimligi {
            ad: Some("Ada \"Kalfa\"".into()),
            eposta: Some("ada@example.com\n[core]".into()),
        })
        .unwrap();
        for dizin in [
            &o.ev,
            &o.gecici,
            &o.cargo().join("registry"),
            &o.npm(),
            &o.pip(),
        ] {
            assert!(dizin.is_dir(), "{} yok", dizin.display());
        }
        let gitconfig = std::fs::read_to_string(o.gitconfig()).unwrap();
        assert_eq!(
            gitconfig,
            "[user]\n\tname = \"Ada \\\"Kalfa\\\"\"\n\temail = \"ada@example.com[core]\"\n"
        );
        assert_eq!(std::fs::read_to_string(o.npmrc()).unwrap(), "");
        let curlrc = std::fs::read_to_string(o.ev.join(".curlrc"));
        if cfg!(windows) {
            assert_eq!(curlrc.unwrap(), "ssl-revoke-best-effort\n");
        } else {
            assert!(curlrc.is_err());
        }
    }

    /// `baglanti`'yı `hedef` dizinine bağlar: Unix'te sembolik bağlantı,
    /// Windows'ta junction (yönetici gerektirmez).
    fn dizin_bagla(hedef: &Path, baglanti: &Path) {
        #[cfg(unix)]
        std::os::unix::fs::symlink(hedef, baglanti).unwrap();
        #[cfg(windows)]
        {
            let durum = Command::new("cmd")
                .args(["/d", "/c", "mklink", "/J"])
                .arg(baglanti)
                .arg(hedef)
                .output()
                .unwrap();
            assert!(durum.status.success(), "{durum:?}");
        }
    }

    #[test]
    fn hazirlik_yalitimin_kurdugu_baglantilari_izlemez() {
        let d = tempfile::tempdir().unwrap();
        let disari = d.path().join("gelistiricinin-evi");
        std::fs::create_dir(&disari).unwrap();
        std::fs::write(disari.join(".gitconfig"), "DOKUNMA").unwrap();
        std::fs::write(disari.join(".npmrc"), "//kayit/:_authToken=GIZLI").unwrap();
        let o = OzelDizinler::yol(&d.path().join("yalitim"), "abc");
        o.hazirla(&GitKimligi::default()).unwrap();

        // Yalıtımdaki komut `ev`'i ve `onbellek/cargo`'yu dışarıya bağlar.
        std::fs::remove_dir_all(&o.ev).unwrap();
        dizin_bagla(&disari, &o.ev);
        std::fs::remove_dir_all(o.cargo()).unwrap();
        dizin_bagla(&disari, &o.cargo());
        o.hazirla(&GitKimligi {
            ad: Some("Ada".into()),
            eposta: None,
        })
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(disari.join(".gitconfig")).unwrap(),
            "DOKUNMA"
        );
        assert_eq!(
            std::fs::read_to_string(disari.join(".npmrc")).unwrap(),
            "//kayit/:_authToken=GIZLI"
        );
        assert!(!disari.join("registry").exists());
        assert!(
            !std::fs::symlink_metadata(&o.ev)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(o.cargo().join("registry").is_dir());
        assert_eq!(
            std::fs::read_to_string(o.gitconfig()).unwrap(),
            "[user]\n\tname = \"Ada\"\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn hazirlik_dosya_baglantisini_izlemez() {
        let d = tempfile::tempdir().unwrap();
        let disari = d.path().join("bashrc");
        std::fs::write(&disari, "DOKUNMA").unwrap();
        let o = OzelDizinler::yol(&d.path().join("yalitim"), "abc");
        o.hazirla(&GitKimligi::default()).unwrap();
        for ad in [".gitconfig", ".npmrc", ".npmrc.kalfa-yeni"] {
            let _ = std::fs::remove_file(o.ev.join(ad));
            std::os::unix::fs::symlink(&disari, o.ev.join(ad)).unwrap();
        }
        o.hazirla(&GitKimligi::default()).unwrap();
        assert_eq!(std::fs::read_to_string(&disari).unwrap(), "DOKUNMA");
        assert_eq!(std::fs::read_to_string(o.npmrc()).unwrap(), "");
        assert!(
            !std::fs::symlink_metadata(o.npmrc())
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn ozet_sirasiz_listede_ayni_dizin_ve_agda_farkli() {
        let a = [PathBuf::from("/usr"), PathBuf::from("/opt/node")];
        let b = [PathBuf::from("/opt/node"), PathBuf::from("/usr")];
        assert_eq!(
            ozet("bwrap", &a, false, None),
            ozet("bwrap", &b, false, None)
        );
        assert_ne!(
            ozet("bwrap", &a, false, None),
            ozet("bwrap", &a, true, None)
        );
        assert_ne!(
            ozet("bwrap", &a, false, None),
            ozet("bwrap", &a[..1], false, None)
        );
        assert_ne!(
            ozet("bwrap", &a, true, Some("ozet1")),
            ozet("bwrap", &a, true, Some("ozet2"))
        );
        assert_eq!(
            ozet("bwrap", &a, false, Some("ozet1")),
            ozet("bwrap", &a, false, Some("ozet2"))
        );
    }
}
