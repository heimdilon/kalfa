//! MSVC ortamı (aşama 2 tasarımı §9.3). rustc, Visual Studio'nun
//! `ProgramData`'daki kurulum kaydını kaptan okuyamadığı için `link.exe`'yi
//! bulamaz. Kalfa, geliştirici tarafında `vswhere` ile son MSVC araç takımını
//! ve Windows SDK'yı bulur; `PATH` eki, `LIB` ve `INCLUDE` kaba verilir. Bu
//! dizinler Program Files (x86) altındadır ve kaba zaten açıktır.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MsvcOrtami {
    /// `PATH`'e eklenecek dizin: `…\bin\Hostx64\x64`.
    pub bin: Option<PathBuf>,
    pub lib: Vec<PathBuf>,
    pub include: Vec<PathBuf>,
}

impl MsvcOrtami {
    /// Makinedeki kurulumdan bulur; bulunamazsa boştur.
    pub fn bul() -> Self {
        let Some(x86) = std::env::var_os("ProgramFiles(x86)").map(PathBuf::from) else {
            return Self::default();
        };
        let vswhere = x86.join(r"Microsoft Visual Studio\Installer\vswhere.exe");
        let kurulum = std::process::Command::new(vswhere)
            .args([
                "-latest",
                "-products",
                "*",
                "-requires",
                "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                "-property",
                "installationPath",
            ])
            .output()
            .ok()
            .filter(|c| c.status.success())
            .map(|c| String::from_utf8_lossy(&c.stdout).trim().to_string())
            .unwrap_or_default();
        if kurulum.is_empty() {
            return Self::default();
        }
        Self::coz(Path::new(&kurulum), &x86.join(r"Windows Kits\10"))
    }

    /// `vs`: Visual Studio kurulumu; `sdk`: `Windows Kits\10`.
    pub fn coz(vs: &Path, sdk: &Path) -> Self {
        let Some(msvc) = son_surum(&vs.join(r"VC\Tools\MSVC"), |d| {
            d.join(r"bin\Hostx64\x64\link.exe").is_file()
        }) else {
            return Self::default();
        };
        let mut o = Self {
            bin: Some(msvc.join(r"bin\Hostx64\x64")),
            lib: vec![msvc.join(r"lib\x64")],
            include: vec![msvc.join("include")],
        };
        if let Some(lib) = son_surum(&sdk.join("Lib"), |d| d.join(r"um\x64").is_dir()) {
            o.lib.push(lib.join(r"um\x64"));
            o.lib.push(lib.join(r"ucrt\x64"));
        }
        if let Some(inc) = son_surum(&sdk.join("Include"), |d| d.join("um").is_dir()) {
            for alt in ["ucrt", "um", "shared", "winrt", "cppwinrt"] {
                o.include.push(inc.join(alt));
            }
        }
        o
    }

    /// Kabın ortamına eklenenler: `PATH`'in başına bağlayıcının dizini ile
    /// `LIB` ve `INCLUDE`. rustc kabın içinde Visual Studio'yu COM ile
    /// bulamaz ve `PATH`'teki ilk `link.exe`'yi çalıştırır; sonda kalsaydı bu
    /// Git'in `usr\bin\link.exe`'si olurdu (MSYS ikilisi, AppContainer'da
    /// 0xC0000142 ile başlayamaz).
    pub fn uygula(&self, ortam: &mut Vec<(String, String)>) {
        let Some(bin) = &self.bin else {
            return;
        };
        let birlestir = |yollar: &[PathBuf]| {
            yollar
                .iter()
                .map(|y| y.display().to_string())
                .collect::<Vec<_>>()
                .join(";")
        };
        match ortam
            .iter_mut()
            .find(|(ad, _)| ad.eq_ignore_ascii_case("PATH"))
        {
            Some((_, deger)) => *deger = format!("{};{deger}", bin.display()),
            None => ortam.push(("PATH".into(), bin.display().to_string())),
        }
        ortam.retain(|(ad, _)| {
            !ad.eq_ignore_ascii_case("LIB") && !ad.eq_ignore_ascii_case("INCLUDE")
        });
        ortam.push(("LIB".into(), birlestir(&self.lib)));
        ortam.push(("INCLUDE".into(), birlestir(&self.include)));
    }
}

/// `dizin`'in koşulu sağlayan, sürüm numarasına göre en büyük alt dizini.
fn son_surum(dizin: &Path, kosul: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    let surum = |d: &Path| -> Vec<u64> {
        d.file_name()
            .and_then(|a| a.to_str())
            .map(|a| a.split('.').map(|p| p.parse().unwrap_or(0)).collect())
            .unwrap_or_default()
    };
    std::fs::read_dir(dizin)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|d| d.is_dir() && kosul(d))
        .max_by_key(|d| surum(d))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaz(yol: &Path) {
        std::fs::create_dir_all(yol.parent().unwrap()).unwrap();
        std::fs::write(yol, "").unwrap();
    }

    #[test]
    fn son_surum_secilir_ve_ortama_eklenir() {
        let d = tempfile::tempdir().unwrap();
        let vs = d.path().join("vs");
        let sdk = d.path().join("sdk");
        yaz(&vs.join(r"VC\Tools\MSVC\14.9.1\bin\Hostx64\x64\link.exe"));
        yaz(&vs.join(r"VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe"));
        // Bağlayıcısı olmayan sürüm seçilmez.
        std::fs::create_dir_all(vs.join(r"VC\Tools\MSVC\14.50.1\bin")).unwrap();
        std::fs::create_dir_all(sdk.join(r"Lib\10.0.22621.0\um\x64")).unwrap();
        std::fs::create_dir_all(sdk.join(r"Lib\10.0.26100.0\um\x64")).unwrap();
        std::fs::create_dir_all(sdk.join(r"Include\10.0.26100.0\um")).unwrap();
        let o = MsvcOrtami::coz(&vs, &sdk);
        let msvc = vs.join(r"VC\Tools\MSVC\14.44.35207");
        assert_eq!(o.bin, Some(msvc.join(r"bin\Hostx64\x64")));
        assert_eq!(
            o.lib,
            vec![
                msvc.join(r"lib\x64"),
                sdk.join(r"Lib\10.0.26100.0\um\x64"),
                sdk.join(r"Lib\10.0.26100.0\ucrt\x64"),
            ]
        );
        assert_eq!(o.include[0], msvc.join("include"));
        assert_eq!(o.include[2], sdk.join(r"Include\10.0.26100.0\um"));

        let mut ortam = vec![
            ("Path".to_string(), r"C:\a".to_string()),
            ("LIB".to_string(), r"C:\eski".to_string()),
        ];
        o.uygula(&mut ortam);
        assert_eq!(
            ortam[0].1,
            format!(r"{};C:\a", msvc.join(r"bin\Hostx64\x64").display())
        );
        assert!(ortam.iter().all(|(_, d)| d != r"C:\eski"));
        assert!(ortam.iter().any(|(a, _)| a == "INCLUDE"));
    }

    #[test]
    fn kurulum_yoksa_bos() {
        let d = tempfile::tempdir().unwrap();
        let o = MsvcOrtami::coz(d.path(), d.path());
        assert_eq!(o, MsvcOrtami::default());
        let mut ortam = vec![("PATH".to_string(), "x".to_string())];
        o.uygula(&mut ortam);
        assert_eq!(ortam, vec![("PATH".to_string(), "x".to_string())]);
    }
}
