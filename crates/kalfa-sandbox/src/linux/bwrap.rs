//! bwrap seçenekleri (aşama 2 tasarımı §8.1). Seçenekler bwrap'a bir bellek
//! dosyasından (`--args FD`) verilir; yol sayısı komut satırı sınırına takılmaz.
//!
//! Sıra önemlidir: sonra gelen bağlama öncekinin üstüne biner. `/tmp` en başta
//! bağlanır; altındaki yollar (örneğin geçici dizindeki bir çalışma alanı) onun
//! üstüne gelir. Korumalı ve gizli yollar en sonda, çalışma alanının üstüne
//! bağlanır.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::profil::{AlanBilgisi, OzelDizinler};

/// Sistemin salt okunur kökleri: dizin ya da (birleşik `/usr`'da) bağlantı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Sistem {
    Dizin(PathBuf),
    Bag { yol: PathBuf, hedef: PathBuf },
}

impl Sistem {
    pub(crate) fn yol(&self) -> &Path {
        match self {
            Self::Dizin(y) | Self::Bag { yol: y, .. } => y,
        }
    }
}

pub(crate) struct Kurulum<'a> {
    pub sistem: &'a [Sistem],
    /// Araç zinciri ve kullanıcının eklediği dizinler; yalnızca var olanlar.
    pub okunur: &'a [PathBuf],
    /// Kullanıcının `~/.cargo/registry`'si; çalışma alanının cargo
    /// önbelleğine salt okunur bağlanır; ne zaman bağlanacağını
    /// `registry_secimi` belirler (§7.4).
    pub registry: Option<&'a Path>,
    pub alan: &'a AlanBilgisi,
    pub ozel: &'a OzelDizinler,
    pub dizin: &'a Path,
    pub ortam: &'a [(String, String)],
    pub ag: bool,
    /// Ağlı profilde `/etc/resolv.conf`'un gösterdiği dosya; `/etc` dışında
    /// olabilir (`/run/...`, WSL'de `/mnt/wsl/...`).
    pub resolv: Option<&'a Path>,
    pub seccomp: &'a [i32],
}

struct Secenekler(Vec<OsString>);

impl Secenekler {
    fn ekle(&mut self, parcalar: &[&str]) {
        self.0.extend(parcalar.iter().map(OsString::from));
    }

    fn yolla(&mut self, secenek: &str, yollar: &[&Path]) {
        self.0.push(secenek.into());
        self.0
            .extend(yollar.iter().map(|y| y.as_os_str().to_os_string()));
    }

    fn ayni(&mut self, secenek: &str, yol: &Path) {
        self.yolla(secenek, &[yol, yol]);
    }
}

pub(crate) fn secenekler(k: &Kurulum) -> Vec<OsString> {
    let mut s = Secenekler(Vec::new());
    s.ekle(&[
        "--unshare-user",
        "--unshare-pid",
        "--unshare-ipc",
        "--unshare-uts",
        "--unshare-cgroup-try",
    ]);
    if !k.ag {
        s.ekle(&["--unshare-net"]);
    }
    s.ekle(&[
        "--disable-userns",
        "--die-with-parent",
        "--new-session",
        "--clearenv",
        "--tmpfs",
        "/",
    ]);
    for sistem in k.sistem {
        match sistem {
            Sistem::Dizin(yol) => s.ayni("--ro-bind", yol),
            Sistem::Bag { yol, hedef } => s.yolla("--symlink", &[hedef, yol]),
        }
    }
    s.ekle(&["--proc", "/proc", "--dev", "/dev", "--tmpfs", "/run"]);
    if k.ag
        && let Some(resolv) = k.resolv
    {
        s.ayni("--ro-bind", resolv);
    }
    s.yolla("--bind", &[&k.ozel.gecici, Path::new("/tmp")]);
    for yol in k.okunur {
        s.ayni("--ro-bind", yol);
    }
    s.ayni("--bind", &k.alan.kok);
    s.ayni("--bind", &k.ozel.ev);
    s.ayni("--bind", &k.ozel.onbellek);
    if let Some(registry) = k.registry {
        s.yolla("--ro-bind", &[registry, &k.ozel.cargo().join("registry")]);
    }
    for git in &k.alan.git_dizinleri {
        s.ayni("--bind", git);
    }
    for korumali in &k.alan.korumali {
        s.ayni("--ro-bind", &korumali.yol);
    }
    for dosya in &k.alan.gizli.dosyalar {
        s.yolla("--ro-bind", &[Path::new("/dev/null"), dosya]);
    }
    for dizin in &k.alan.gizli.dizinler {
        s.yolla("--tmpfs", &[dizin]);
        s.yolla("--remount-ro", &[dizin]);
    }
    // Boş kök de salt okunur olur: çalışma alanı dışına yazma denemesi
    // sessizce bellekte kalmak yerine hata verir.
    s.ekle(&["--remount-ro", "/"]);
    for (ad, deger) in k.ortam {
        s.ekle(&["--setenv", ad, deger]);
    }
    s.yolla("--chdir", &[k.dizin]);
    for fd in k.seccomp {
        s.ekle(&["--add-seccomp-fd", &fd.to_string()]);
    }
    s.0
}

/// `--args` biçimi: her seçenek NUL ile biter.
pub(crate) fn nul_ile(secenekler: &[OsString]) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    let mut veri = Vec::new();
    for s in secenekler {
        veri.extend_from_slice(s.as_bytes());
        veri.push(0);
    }
    veri
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profil::Korumali;
    use crate::tarama::Tarama;

    fn metin(s: &[OsString]) -> Vec<String> {
        s.iter().map(|p| p.to_string_lossy().into_owned()).collect()
    }

    /// `dizi` içinde `parca`'nın ardışık geçtiği konum.
    fn konum(dizi: &[String], parca: &[&str]) -> usize {
        dizi.windows(parca.len())
            .position(|w| w.iter().zip(parca).all(|(a, b)| a == b))
            .unwrap_or_else(|| panic!("{parca:?} yok: {dizi:?}"))
    }

    fn kurulum_ile<T>(ag: bool, is: impl FnOnce(&Kurulum) -> T) -> T {
        let alan = AlanBilgisi {
            kok: "/tmp/alan".into(),
            gizli: Tarama {
                dosyalar: vec!["/tmp/alan/.env".into()],
                dizinler: vec!["/tmp/alan/gizli".into()],
            },
            korumali: vec![Korumali {
                yol: "/tmp/alan/.git/hooks".into(),
                dizin: true,
            }],
            git_dizinleri: vec!["/depo/.git".into()],
        };
        let ozel = OzelDizinler::yol(Path::new("/veri/yalitim"), "abc");
        let sistem = [
            Sistem::Dizin("/usr".into()),
            Sistem::Bag {
                yol: "/bin".into(),
                hedef: "usr/bin".into(),
            },
        ];
        let okunur = [PathBuf::from("/ev/.cargo/bin")];
        let ortam = [("HOME".to_string(), "/veri/yalitim/abc/ev".to_string())];
        let k = Kurulum {
            sistem: &sistem,
            okunur: &okunur,
            registry: Some(Path::new("/ev/.cargo/registry")),
            alan: &alan,
            ozel: &ozel,
            dizin: Path::new("/tmp/alan/alt"),
            ortam: &ortam,
            ag,
            resolv: Some(Path::new("/run/resolv.conf")),
            seccomp: &[5, 6],
        };
        is(&k)
    }

    #[test]
    fn agsiz_profilde_ag_ayrilir_ve_siralama_dogrudur() {
        let s = kurulum_ile(false, |k| metin(&secenekler(k)));
        assert!(s.contains(&"--unshare-net".to_string()));
        assert!(!s.contains(&"/run/resolv.conf".to_string()));
        for bayrak in [
            "--disable-userns",
            "--die-with-parent",
            "--new-session",
            "--clearenv",
        ] {
            assert!(s.contains(&bayrak.to_string()), "{bayrak} yok");
        }
        let kok = konum(&s, &["--tmpfs", "/"]);
        let usr = konum(&s, &["--ro-bind", "/usr", "/usr"]);
        let bin = konum(&s, &["--symlink", "usr/bin", "/bin"]);
        let tmp = konum(&s, &["--bind", "/veri/yalitim/abc/gecici", "/tmp"]);
        let alan = konum(&s, &["--bind", "/tmp/alan", "/tmp/alan"]);
        let hooks = konum(
            &s,
            &["--ro-bind", "/tmp/alan/.git/hooks", "/tmp/alan/.git/hooks"],
        );
        let env = konum(&s, &["--ro-bind", "/dev/null", "/tmp/alan/.env"]);
        let gizli = konum(
            &s,
            &[
                "--tmpfs",
                "/tmp/alan/gizli",
                "--remount-ro",
                "/tmp/alan/gizli",
            ],
        );
        assert!(kok < usr && usr < bin && bin < tmp && tmp < alan);
        assert!(alan < hooks && alan < env && alan < gizli);
        assert!(gizli < konum(&s, &["--remount-ro", "/"]));
        konum(
            &s,
            &[
                "--ro-bind",
                "/ev/.cargo/registry",
                "/veri/yalitim/abc/onbellek/cargo/registry",
            ],
        );
        konum(&s, &["--bind", "/depo/.git", "/depo/.git"]);
        konum(&s, &["--setenv", "HOME", "/veri/yalitim/abc/ev"]);
        konum(
            &s,
            &[
                "--chdir",
                "/tmp/alan/alt",
                "--add-seccomp-fd",
                "5",
                "--add-seccomp-fd",
                "6",
            ],
        );
    }

    #[test]
    fn agli_profilde_ag_ayrilmaz_resolv_baglanir() {
        let s = kurulum_ile(true, |k| metin(&secenekler(k)));
        assert!(!s.contains(&"--unshare-net".to_string()));
        konum(&s, &["--ro-bind", "/run/resolv.conf", "/run/resolv.conf"]);
    }

    #[test]
    fn args_bicimi_nul_ile_biter() {
        let veri = nul_ile(&["--a".into(), "b c".into()]);
        assert_eq!(veri, b"--a\0b c\0");
    }
}
