//! Kabın komut satırı. Kap `CreateProcessW` ile başlar (§9.3); std'nin
//! `Command`'ı güvenlik yeteneklerini veremiyor. Argümanlar std'nin
//! kurallarıyla yazılır: program `CommandLineToArgvW` kurallarıyla, toplu
//! dosya (`.cmd`, `.bat`) ise cmd.exe'nin kurallarıyla. Toplu dosyada
//! `%DEGISKEN%` genişlemesi ve tırnak kaçışı engellenir (std'deki
//! "BatBadBut" düzeltmesinin karşılığı); satır sonu içeren argüman reddedilir.

use std::io;
use std::path::{Path, PathBuf};

/// Başlatılacak dosya ve komut satırı (NUL'suz UTF-16).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KomutSatiri {
    pub uygulama: PathBuf,
    pub satir: Vec<u16>,
}

fn gecersiz(metin: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, metin.to_string())
}

fn toplu_dosya_mi(program: &Path) -> bool {
    program
        .extension()
        .and_then(|u| u.to_str())
        .is_some_and(|u| u.eq_ignore_ascii_case("cmd") || u.eq_ignore_ascii_case("bat"))
}

/// `program` mutlak ve çözülmüş bir yoldur. `cmd`: toplu dosyayı çalıştıracak
/// cmd.exe'nin yolu.
pub(crate) fn uret(program: &Path, argumanlar: &[String], cmd: &Path) -> io::Result<KomutSatiri> {
    let ad: Vec<u16> = program.as_os_str().encode_wide_vec();
    if ad.contains(&u16::from(b'"')) || ad.contains(&0) {
        return Err(gecersiz("Program yolu tırnak ya da NUL içeremez."));
    }
    if argumanlar.iter().any(|a| a.contains('\0')) {
        return Err(gecersiz("Argüman NUL içeremez."));
    }
    if toplu_dosya_mi(program) {
        if ad.last() == Some(&u16::from(b'\\')) {
            return Err(gecersiz("Toplu dosya yolu `\\` ile bitemez."));
        }
        let mut satir: Vec<u16> = "cmd.exe /e:ON /v:OFF /d /c \"".encode_utf16().collect();
        satir.push(u16::from(b'"'));
        satir.extend_from_slice(&ad);
        satir.push(u16::from(b'"'));
        for a in argumanlar {
            if a.contains('\r') || a.contains('\n') {
                return Err(gecersiz(
                    "Toplu dosyaya verilen argüman satır sonu içeremez.",
                ));
            }
            satir.push(u16::from(b' '));
            toplu_arguman(&mut satir, a);
        }
        satir.push(u16::from(b'"'));
        return Ok(KomutSatiri {
            uygulama: cmd.to_path_buf(),
            satir,
        });
    }
    let mut satir = vec![u16::from(b'"')];
    satir.extend_from_slice(&ad);
    satir.push(u16::from(b'"'));
    for a in argumanlar {
        satir.push(u16::from(b' '));
        arguman(&mut satir, a);
    }
    Ok(KomutSatiri {
        uygulama: program.to_path_buf(),
        satir,
    })
}

trait GenisVec {
    fn encode_wide_vec(&self) -> Vec<u16>;
}

impl GenisVec for std::ffi::OsStr {
    fn encode_wide_vec(&self) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        self.encode_wide().collect()
    }
}

/// Argümanları boşlukla ayrılmış tek satıra `arguman`'ın kurallarıyla yazar
/// (`ShellExecuteExW`'nin parametreleri).
pub(crate) fn argumanlar_metni(argumanlar: &[&str]) -> String {
    let mut satir = Vec::new();
    for (i, a) in argumanlar.iter().enumerate() {
        if i > 0 {
            satir.push(u16::from(b' '));
        }
        arguman(&mut satir, a);
    }
    String::from_utf16_lossy(&satir)
}

/// `CommandLineToArgvW` kuralları: boşluk ya da sekme içeren (ya da boş)
/// argüman tırnaklanır; tırnaktan önceki ters bölüler ikilenir.
fn arguman(satir: &mut Vec<u16>, a: &str) {
    let tirnak = a.is_empty() || a.contains(' ') || a.contains('\t');
    if tirnak {
        satir.push(u16::from(b'"'));
    }
    let mut ters = 0usize;
    for c in a.encode_utf16() {
        if c == u16::from(b'\\') {
            ters += 1;
        } else {
            if c == u16::from(b'"') {
                satir.extend(std::iter::repeat_n(u16::from(b'\\'), ters + 1));
            }
            ters = 0;
        }
        satir.push(c);
    }
    if tirnak {
        satir.extend(std::iter::repeat_n(u16::from(b'\\'), ters));
        satir.push(u16::from(b'"'));
    }
}

/// cmd.exe kuralları: güvenli bilinmeyen her ASCII simgesi argümanı tırnaklatır;
/// tırnak ikilenerek kaçar; `%` ise `%%cd:~,%` olur: `cd` değişkeninin boş alt
/// dizesi hiçbir şeye genişler ve cmd'nin `%DEGISKEN%` genişletmesini böler.
fn toplu_arguman(satir: &mut Vec<u16>, a: &str) {
    const GUVENLI: &str = r"#$*+-./:?@\_";
    let mut tirnak = a.is_empty() || a.ends_with('\\');
    for c in a.chars() {
        let ascii_simge = c.is_ascii() && !(c.is_ascii_alphanumeric() || GUVENLI.contains(c));
        if ascii_simge || c.is_control() {
            tirnak = true;
        }
    }
    if tirnak {
        satir.push(u16::from(b'"'));
    }
    let mut ters = 0usize;
    for c in a.encode_utf16() {
        if c == u16::from(b'\\') {
            ters += 1;
        } else {
            if c == u16::from(b'"') {
                satir.extend(std::iter::repeat_n(u16::from(b'\\'), ters));
                satir.push(u16::from(b'"'));
            } else if c == u16::from(b'%') {
                satir.extend("%%cd:~,".encode_utf16());
            }
            ters = 0;
        }
        satir.push(c);
    }
    if tirnak {
        satir.extend(std::iter::repeat_n(u16::from(b'\\'), ters));
        satir.push(u16::from(b'"'));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metin(k: &KomutSatiri) -> String {
        String::from_utf16(&k.satir).unwrap()
    }

    fn argv(argumanlar: &[&str]) -> Vec<String> {
        argumanlar.iter().map(|a| a.to_string()).collect()
    }

    const CMD: &str = r"C:\Windows\System32\cmd.exe";

    #[test]
    fn program_tirnaklanir_argumanlar_msvc_kurallariyla() {
        let k = uret(
            Path::new(r"C:\Araclar\git.exe"),
            &argv(&[
                "status",
                "iki kelime",
                "",
                r#"a"b"#,
                r"son\",
                r#"x\"y"#,
                r"c:\d\",
            ]),
            Path::new(CMD),
        )
        .unwrap();
        assert_eq!(k.uygulama, PathBuf::from(r"C:\Araclar\git.exe"));
        assert_eq!(
            metin(&k),
            r#""C:\Araclar\git.exe" status "iki kelime" "" a\"b son\ x\\\"y c:\d\"#
        );
    }

    #[test]
    fn toplu_dosya_cmd_ile_ve_yuzde_kacisiyla() {
        let k = uret(
            Path::new(r"C:\Program Files\nodejs\npm.cmd"),
            &argv(&["run", "test", "%PATH%", "a&b", r#"x"y"#, "", r"d\"]),
            Path::new(CMD),
        )
        .unwrap();
        assert_eq!(k.uygulama, PathBuf::from(CMD));
        assert_eq!(
            metin(&k),
            r#"cmd.exe /e:ON /v:OFF /d /c ""C:\Program Files\nodejs\npm.cmd" run test "%%cd:~,%PATH%%cd:~,%" "a&b" "x""y" "" "d\\"""#
        );
    }

    #[test]
    fn toplu_dosyada_satir_sonu_reddedilir() {
        let hata = uret(Path::new(r"C:\a\b.bat"), &argv(&["a\nb"]), Path::new(CMD)).unwrap_err();
        assert_eq!(hata.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn nul_ve_tirnakli_program_reddedilir() {
        assert!(uret(Path::new(r"C:\a.exe"), &argv(&["a\0b"]), Path::new(CMD)).is_err());
        assert!(uret(Path::new(r#"C:\a".exe"#), &[], Path::new(CMD)).is_err());
    }

    #[test]
    fn uzanti_buyuk_harfli_toplu_dosya_da_taninir() {
        let k = uret(Path::new(r"C:\a\B.CMD"), &[], Path::new(CMD)).unwrap();
        assert_eq!(k.uygulama, PathBuf::from(CMD));
    }
}
