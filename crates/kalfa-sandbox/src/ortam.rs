//! Komutun ortam değişkenleri (aşama 2 tasarımı §6.1): ortam boş başlar,
//! yalnızca izin listesindekiler geçer. EVREN anahtarı, token'lar ve ajan
//! soketleri hiçbir koşulda aktarılmaz.

/// Her platformda geçen değişkenler.
const ORTAK: &[&str] = &[
    "PATH",
    "HOME",
    "USER",
    "LOGNAME",
    "LANG",
    "LANGUAGE",
    "TZ",
    "TMPDIR",
    "TEMP",
    "TMP",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
    "GOPATH",
    "GOROOT",
    "JAVA_HOME",
];

/// Windows'ta sistem araçlarının çalışması için gerekenler.
const WINDOWS: &[&str] = &[
    "SystemRoot",
    "windir",
    "ComSpec",
    "PATHEXT",
    "SystemDrive",
    "ProgramFiles",
    "ProgramFiles(x86)",
    "ProgramW6432",
    "ProgramData",
    "CommonProgramFiles",
    "CommonProgramFiles(x86)",
    "CommonProgramW6432",
    "NUMBER_OF_PROCESSORS",
    "PROCESSOR_ARCHITECTURE",
    "PROCESSOR_IDENTIFIER",
    "PROCESSOR_LEVEL",
    "PROCESSOR_REVISION",
    "OS",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "HOMEDRIVE",
    "HOMEPATH",
    "USERNAME",
    // Windows PowerShell'in modül çözümleme önbelleğinin yeri. Bazı imajlar
    // (GitHub'ın Windows makineleri) önbelleği önceden ısıtıp buraya koyar;
    // değişken geçmezse her komutun ilk cmdlet'i bütün modülleri baştan
    // çözümler (windows-2025'te komut başına ~25 sn).
    "PSModuleAnalysisCachePath",
];

/// Bu önekle başlayanlar geçer.
const ONEKLER: &[&str] = &["LC_"];

/// Adında bunlardan biri geçen değişken, izin listesinde olsa da geçmez.
const YASAK_PARCALAR: &[&str] = &[
    "KEY",
    "TOKEN",
    "SECRET",
    "PASSWORD",
    "PASSWD",
    "CREDENTIAL",
    "AUTH",
];

/// Etkileşim bekleyen araçlar için sabit değerler (aşama 2 tasarımı §4.3).
pub const SABIT: &[(&str, &str)] = &[
    ("TERM", "dumb"),
    ("GIT_TERMINAL_PROMPT", "0"),
    ("GIT_PAGER", "cat"),
    ("PAGER", "cat"),
    ("PYTHONIOENCODING", "utf-8"),
    ("PYTHONUTF8", "1"),
];

fn ayni_ad(a: &str, b: &str) -> bool {
    // Windows'ta ortam değişkeni adları büyük/küçük harf ayırmaz.
    if cfg!(windows) {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

/// Değişken komuta geçebilir mi?
pub fn izinli_mi(ad: &str) -> bool {
    let buyuk = ad.to_ascii_uppercase();
    if YASAK_PARCALAR.iter().any(|p| buyuk.contains(p)) {
        return false;
    }
    let listede = ORTAK
        .iter()
        .chain(if cfg!(windows) { WINDOWS } else { &[] })
        .any(|izinli| ayni_ad(izinli, ad));
    listede || ONEKLER.iter().any(|o| ad.starts_with(o))
}

/// Kaynak ortamdan komutun ortamını üretir: izin listesindekiler ve sabit
/// değerler. Sabit değerler kaynaktakileri ezer.
pub fn uret(kaynak: impl IntoIterator<Item = (String, String)>) -> Vec<(String, String)> {
    let mut ortam: Vec<(String, String)> = kaynak
        .into_iter()
        .filter(|(ad, _)| izinli_mi(ad) && !SABIT.iter().any(|(s, _)| ayni_ad(s, ad)))
        .collect();
    ortam.extend(
        SABIT
            .iter()
            .map(|(ad, deger)| (ad.to_string(), deger.to_string())),
    );
    ortam
}

/// Ortamdaki bir değişkenin değeri.
pub fn deger<'a>(ortam: &'a [(String, String)], ad: &str) -> Option<&'a str> {
    ortam
        .iter()
        .find(|(a, _)| ayni_ad(a, ad))
        .map(|(_, d)| d.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cift(ad: &str, deger: &str) -> (String, String) {
        (ad.to_string(), deger.to_string())
    }

    #[test]
    fn izin_listesi_disindakiler_ve_sirlar_gecmez() {
        let ortam = uret([
            cift("PATH", "/usr/bin"),
            cift("LC_ALL", "tr_TR.UTF-8"),
            cift("EVREN_API_KEY", "evren_llm_x"),
            cift("GITHUB_TOKEN", "ghp_x"),
            cift("SSH_AUTH_SOCK", "/tmp/ssh"),
            cift("AWS_SECRET_ACCESS_KEY", "x"),
            cift("DISPLAY", ":0"),
            cift("RASTGELE", "1"),
        ]);
        let adlar: Vec<&str> = ortam.iter().map(|(a, _)| a.as_str()).collect();
        assert!(adlar.contains(&"PATH"));
        assert!(adlar.contains(&"LC_ALL"));
        for yasak in [
            "EVREN_API_KEY",
            "GITHUB_TOKEN",
            "SSH_AUTH_SOCK",
            "AWS_SECRET_ACCESS_KEY",
            "DISPLAY",
            "RASTGELE",
        ] {
            assert!(!adlar.contains(&yasak), "{yasak} geçmemeliydi");
        }
    }

    #[test]
    fn sabit_degerler_kaynaktakini_ezer() {
        let ortam = uret([cift("TERM", "xterm-256color")]);
        assert_eq!(deger(&ortam, "TERM"), Some("dumb"));
        assert_eq!(ortam.iter().filter(|(a, _)| a == "TERM").count(), 1);
        assert_eq!(deger(&ortam, "GIT_TERMINAL_PROMPT"), Some("0"));
    }

    #[cfg(windows)]
    #[test]
    fn powershell_modul_onbellegi_gecer() {
        let ortam = uret([cift(
            "PSModuleAnalysisCachePath",
            "C:\\PSModuleAnalysisCachePath\\ModuleAnalysisCache",
        )]);
        assert_eq!(
            deger(&ortam, "PSModuleAnalysisCachePath"),
            Some("C:\\PSModuleAnalysisCachePath\\ModuleAnalysisCache")
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_adlari_buyuk_kucuk_harf_ayirmaz() {
        let ortam = uret([
            cift("Path", "C:\\Windows"),
            cift("SYSTEMROOT", "C:\\Windows"),
        ]);
        assert_eq!(deger(&ortam, "PATH"), Some("C:\\Windows"));
        assert_eq!(deger(&ortam, "SystemRoot"), Some("C:\\Windows"));
    }
}
