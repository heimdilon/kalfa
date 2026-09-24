//! Programın bulunması ve betiğin yorumlayıcısı (aşama 2 tasarımı §4.1).
//!
//! Çıplak ad komutun kendi ortamındaki `PATH`'te aranır, Kalfa'nınkinde değil.
//! Windows'ta `PATHEXT` uzantıları da denenir: `npm` gibi araçlar `npm.cmd`'dir.

use std::path::{Path, PathBuf};

use crate::ortam;
use crate::{BaslatmaHatasi, KomutBicimi};

/// Betiğin çalıştırılacağı yorumlayıcının gösterim adı.
pub fn yorumlayici_adi() -> &'static str {
    if cfg!(windows) { "PowerShell" } else { "sh" }
}

/// Windows'ta PowerShell'in çıktıyı UTF-8 yazması ve ilerleme kaydı basmaması
/// için betiğin önüne eklenen satır.
const POWERSHELL_ONEKI: &str = "$ProgressPreference='SilentlyContinue';\
[Console]::OutputEncoding=[System.Text.Encoding]::UTF8;\
$OutputEncoding=[System.Text.Encoding]::UTF8;\n";

/// Komutu çalıştırılacak programa ve argümanlarına çevirir.
pub fn coz(
    komut: &KomutBicimi,
    ortam: &[(String, String)],
) -> Result<(PathBuf, Vec<String>), BaslatmaHatasi> {
    match komut {
        KomutBicimi::Program {
            program,
            argumanlar,
        } => Ok((bul(program, ortam)?, argumanlar.clone())),
        KomutBicimi::Betik { metin } => {
            if cfg!(windows) {
                let kodlu = base64(&utf16le(&format!("{POWERSHELL_ONEKI}{metin}")));
                let args = [
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-EncodedCommand",
                    &kodlu,
                ];
                Ok((
                    bul("powershell", ortam)?,
                    args.iter().map(|a| a.to_string()).collect(),
                ))
            } else {
                Ok((PathBuf::from("/bin/sh"), vec!["-c".into(), metin.clone()]))
            }
        }
    }
}

/// Programı bulur. Yol ayırıcısı içeren ad olduğu gibi kullanılır.
pub fn bul(program: &str, ortam: &[(String, String)]) -> Result<PathBuf, BaslatmaHatasi> {
    let path = ortam::deger(ortam, "PATH").unwrap_or("");
    let yok = || BaslatmaHatasi::ProgramYok {
        program: program.to_string(),
        path: path.to_string(),
    };
    if program.contains('/') || program.contains('\\') {
        let yol = PathBuf::from(program);
        return if calistirilabilir(&yol) {
            Ok(yol)
        } else {
            Err(yok())
        };
    }
    for dizin in std::env::split_paths(path) {
        if dizin.as_os_str().is_empty() {
            continue;
        }
        for aday in adaylar(&dizin, program, ortam) {
            if calistirilabilir(&aday) {
                return Ok(aday);
            }
        }
    }
    Err(yok())
}

fn adaylar(dizin: &Path, program: &str, ortam: &[(String, String)]) -> Vec<PathBuf> {
    let mut adaylar = vec![dizin.join(program)];
    if cfg!(windows) {
        let pathext = ortam::deger(ortam, "PATHEXT").unwrap_or(".COM;.EXE;.BAT;.CMD");
        let uzantili = Path::new(program).extension().is_some();
        let ekler = pathext
            .split(';')
            .filter(|u| !u.is_empty())
            .map(|u| dizin.join(format!("{program}{}", u.to_ascii_lowercase())));
        if uzantili {
            adaylar.extend(ekler);
        } else {
            // Uzantısız dosya Windows'ta çalıştırılamaz.
            adaylar = ekler.collect();
        }
    }
    adaylar
}

fn calistirilabilir(yol: &Path) -> bool {
    let Ok(bilgi) = std::fs::metadata(yol) else {
        return false;
    };
    if !bilgi.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        bilgi.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn utf16le(metin: &str) -> Vec<u8> {
    metin.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

fn base64(veri: &[u8]) -> String {
    const ABECE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut cikti = String::with_capacity(veri.len().div_ceil(3) * 4);
    for parca in veri.chunks(3) {
        let b = [
            parca[0],
            parca.get(1).copied().unwrap_or(0),
            parca.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= parca.len() {
                cikti.push(char::from(ABECE[(n >> (18 - 6 * i)) as usize & 63]));
            } else {
                cikti.push('=');
            }
        }
    }
    cikti
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_rfc4648_orneklerini_uretir() {
        for (girdi, beklenen) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(girdi.as_bytes()), beklenen);
        }
    }

    #[test]
    fn path_disindaki_program_bulunamaz() {
        let bos = tempfile::tempdir().unwrap();
        let ortam = vec![("PATH".to_string(), bos.path().display().to_string())];
        let hata = bul("kesinlikle-olmayan-program", &ortam).unwrap_err();
        assert!(matches!(hata, BaslatmaHatasi::ProgramYok { .. }));
    }

    #[cfg(windows)]
    #[test]
    fn windows_pathext_ile_cmd_bulunur() {
        let dizin = tempfile::tempdir().unwrap();
        std::fs::write(dizin.path().join("arac.cmd"), "@echo off\r\n").unwrap();
        let ortam = vec![
            ("PATH".to_string(), dizin.path().display().to_string()),
            ("PATHEXT".to_string(), ".EXE;.CMD".to_string()),
        ];
        assert_eq!(bul("arac", &ortam).unwrap(), dizin.path().join("arac.cmd"));
    }

    #[cfg(unix)]
    #[test]
    fn unix_calistirilamayan_dosya_atlanir() {
        use std::os::unix::fs::PermissionsExt;
        let dizin = tempfile::tempdir().unwrap();
        let yol = dizin.path().join("arac");
        std::fs::write(&yol, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&yol, std::fs::Permissions::from_mode(0o644)).unwrap();
        let ortam = vec![("PATH".to_string(), dizin.path().display().to_string())];
        assert!(bul("arac", &ortam).is_err());
        std::fs::set_permissions(&yol, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(bul("arac", &ortam).unwrap(), yol);
    }
}
