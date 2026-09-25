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
    /// Salt okunur kalacak yollar: `.git/hooks`, `.git/config`,
    /// `core.hooksPath` (§7.2).
    pub korumali: Vec<Korumali>,
    /// Çalışma alanı dışındaki git dizinleri (worktree, submodule);
    /// yazılabilir bağlanır (§7.4).
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
    pub fn hazirla(&self, git: &GitKimligi) -> io::Result<()> {
        for dizin in [
            &self.ev,
            &self.gecici,
            &self.cargo().join("registry"),
            &self.npm(),
            &self.pip(),
        ] {
            std::fs::create_dir_all(dizin)?;
        }
        std::fs::write(self.gitconfig(), git.gitconfig())?;
        std::fs::write(self.npmrc(), "")?;
        Ok(())
    }
}

/// Profil özeti (§7.1): yalıtımın arka ucu, okunur yolların sıralı listesi ve
/// ağ bayrağının SHA-256'sı. Mühre girer: kullanıcı bir dizin eklerse bekleyen
/// onaylar geçersizleşir. Gizli ve korumalı yollar her çalıştırmadan hemen önce
/// tarandığı için özete girmez; onları belirleyen kurallar sabittir.
pub fn ozet(arka_uc: &str, okunur: &[PathBuf], ag: bool) -> String {
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
    }

    #[test]
    fn ozet_sirasiz_listede_ayni_dizin_ve_agda_farkli() {
        let a = [PathBuf::from("/usr"), PathBuf::from("/opt/node")];
        let b = [PathBuf::from("/opt/node"), PathBuf::from("/usr")];
        assert_eq!(ozet("bwrap", &a, false), ozet("bwrap", &b, false));
        assert_ne!(ozet("bwrap", &a, false), ozet("bwrap", &a, true));
        assert_ne!(ozet("bwrap", &a, false), ozet("bwrap", &a[..1], false));
    }
}
