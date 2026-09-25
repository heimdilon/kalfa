//! `olc`: P1–P10 ölçümleri (aşama 2 tasarımı §9.5). Yönetici gerektirmez;
//! `kur.ps1`'in oluşturduğu kullanıcıları, parolaları ve ortak SID'i kullanır.
//! Deneme ağacı `C:\KalfaPrototip` altındadır ve her koşuda yeniden kurulur.
//!
//!   kalfa-prototip olc [önek…]   ölçümleri koşar (önek verilirse yalnızca
//!                                kodu o önekle başlayanlar: p2, p5 …)
//!   kalfa-prototip olc temizle   deneme ağacını siler

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde::Deserialize;

use crate::calistir::Istek;
use crate::ui;
use crate::win::{self, OturumSonucu, Sonuc};

pub(crate) const KOK: &str = r"C:\KalfaPrototip";
pub(crate) const PROJELER: &str = r"C:\KalfaPrototipProjeler";
/// ALL APPLICATION PACKAGES: kısıtlayıcı SID olarak kabul edilmiyor (P1).
const AAP: &str = "S-1-15-2-1";
const RESTRICTED: &str = "S-1-5-12";
/// NUL aygıtına yazmak için gerekiyor.
const EVERYONE: &str = "S-1-1-0";
/// Sistem ve program dizinlerini okumak için gerekiyor (AAP kullanılamıyor).
const USERS: &str = "S-1-5-32-545";

#[derive(Deserialize)]
struct Kullanici {
    ad: String,
    sid: String,
}

#[derive(Deserialize)]
struct Agac {
    alan_a_sid: String,
    alan_b_sid: String,
}

#[derive(Deserialize)]
struct Kurulum {
    ortak_sid: String,
    agsiz: Kullanici,
    agli: Kullanici,
    users_grubunda: bool,
}

struct Ortam {
    kurulum: Kurulum,
    parolalar: std::collections::HashMap<String, String>,
    yerel: PathBuf,
    alan_a_sid: String,
    alan_b_sid: String,
    path: String,
    port: u16,
    boru: String,
    pencere: isize,
}

#[derive(Clone, Copy, PartialEq)]
enum Kim {
    Agsiz,
    Agli,
}

/// Beklenti: `Some(true)` geçti, `Some(false)` kaldı, `None` yalnızca ölçüm.
pub(crate) type Denetim = Box<dyn Fn(&OturumSonucu) -> Option<bool>>;

struct Senaryo {
    kod: String,
    aciklama: String,
    beklenen: &'static str,
    kim: Kim,
    kisitlayicilar: Option<Vec<String>>,
    ui_kisit: bool,
    ayricaliklari_koru: bool,
    dizin: String,
    ozel: &'static str,
    cargo_home: Option<String>,
    komut: String,
    denetim: Denetim,
}

pub(crate) fn basarili() -> Denetim {
    Box::new(|s| Some(s.cikis == Some(0)))
}

pub(crate) fn basarisiz() -> Denetim {
    Box::new(|s| Some(s.cikis.is_some_and(|k| k != 0) && !s.hata.contains("KALFA-PROTOTIP-HATA")))
}

pub(crate) fn icermez(metin: &'static str) -> Denetim {
    Box::new(move |s| Some(!s.standart.contains(metin) && !s.hata.contains("KALFA-PROTOTIP-HATA")))
}

pub(crate) fn olcum() -> Denetim {
    Box::new(|_| None)
}

pub(crate) fn cmd(komut: &str) -> String {
    format!(r"C:\Windows\System32\cmd.exe /d /c {komut}")
}

pub(crate) fn ps(betik: &str) -> String {
    format!(
        r#"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe -NoProfile -NonInteractive -Command "{betik}""#
    )
}

fn sil(yol: &str) {
    if Path::new(yol).exists() {
        let _ = std::fs::remove_dir_all(yol);
    }
}

pub fn ana(argumanlar: &[String]) -> i32 {
    if argumanlar.first().map(String::as_str) == Some("temizle") {
        sil(KOK);
        sil(PROJELER);
        println!("deneme ağacı silindi");
        return 0;
    }
    match olc(argumanlar) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("hata: {e}");
            1
        }
    }
}

fn yukle() -> Sonuc<(Kurulum, std::collections::HashMap<String, String>, PathBuf)> {
    let yerel = PathBuf::from(std::env::var("LOCALAPPDATA").map_err(|e| e.to_string())?)
        .join("KalfaPrototip");
    let ham = std::fs::read(yerel.join("kurulum.json"))
        .map_err(|e| format!("kurulum.json okunamadı ({e}); önce kur.ps1 çalıştırılmalı"))?;
    // PowerShell 5.1 UTF-8'i BOM'la yazar.
    let ham = ham.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&ham);
    let kurulum: Kurulum = serde_json::from_slice(ham).map_err(|e| format!("kurulum.json: {e}"))?;
    let sifreli = std::fs::read(yerel.join("parolalar.bin"))
        .map_err(|e| format!("parolalar.bin okunamadı: {e}"))?;
    let parolalar = serde_json::from_slice(&win::dpapi_coz(&sifreli)?)
        .map_err(|e| format!("parolalar: {e}"))?;
    Ok((kurulum, parolalar, yerel))
}

/// Araç zincirinin dizinleri: PATH'te ilk bulunan yerler.
pub(crate) fn arac_yolu() -> String {
    let mut dizinler: Vec<String> = [
        r"C:\Windows\System32",
        r"C:\Windows",
        r"C:\Windows\System32\Wbem",
        r"C:\Windows\System32\WindowsPowerShell\v1.0",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let path = std::env::var("PATH").unwrap_or_default();
    for program in ["git.exe", "node.exe", "python.exe", "cargo.exe"] {
        if let Some(d) = std::env::split_paths(&path).find(|d| d.join(program).is_file()) {
            let d = d.display().to_string();
            if !dizinler.contains(&d) {
                dizinler.push(d);
            }
        }
    }
    dizinler.join(";")
}

/// Yalıtımın ortam değişkenleri: çalışma alanına özel ev, geçici ve önbellek
/// dizinleri.
pub(crate) fn yalitim_ortami(
    path: &str,
    ozel: &str,
    cargo_home: Option<&str>,
) -> Vec<(String, String)> {
    let oz = format!(r"{KOK}\ozel-{ozel}");
    let ev = format!(r"{oz}\ev");
    let mut v: Vec<(String, String)> = [
        ("SystemRoot", r"C:\Windows"),
        ("windir", r"C:\Windows"),
        ("SystemDrive", "C:"),
        ("ComSpec", r"C:\Windows\System32\cmd.exe"),
        ("PATHEXT", ".COM;.EXE;.BAT;.CMD;.VBS;.JS;.WS;.MSC;.PS1"),
        ("OS", "Windows_NT"),
        ("ProgramFiles", r"C:\Program Files"),
        ("ProgramFiles(x86)", r"C:\Program Files (x86)"),
        ("ProgramW6432", r"C:\Program Files"),
        ("CommonProgramFiles", r"C:\Program Files\Common Files"),
        ("ProgramData", r"C:\ProgramData"),
    ]
    .iter()
    .map(|(a, d)| (a.to_string(), d.to_string()))
    .collect();
    for a in ["PROCESSOR_ARCHITECTURE", "NUMBER_OF_PROCESSORS", "USERNAME"] {
        if let Ok(d) = std::env::var(a) {
            v.push((a.into(), d));
        }
    }
    let rustup = format!(
        r"{}\.rustup",
        std::env::var("USERPROFILE").unwrap_or_default()
    );
    v.extend([
        ("PATH".into(), path.to_string()),
        ("TEMP".into(), format!(r"{oz}\gecici")),
        ("TMP".into(), format!(r"{oz}\gecici")),
        ("USERPROFILE".into(), ev.clone()),
        ("HOME".into(), ev.clone()),
        ("APPDATA".into(), format!(r"{ev}\AppData\Roaming")),
        ("LOCALAPPDATA".into(), format!(r"{ev}\AppData\Local")),
        (
            "CARGO_HOME".into(),
            cargo_home.map_or(format!(r"{oz}\cargo"), str::to_string),
        ),
        ("RUSTUP_HOME".into(), rustup),
        ("npm_config_cache".into(), format!(r"{oz}\npm")),
        ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
    ]);
    v
}

impl Ortam {
    fn ortam(&self, ozel: &str, cargo_home: Option<&str>) -> Vec<(String, String)> {
        yalitim_ortami(&self.path, ozel, cargo_home)
    }

    fn tam(&self, alan_sid: &str) -> Vec<String> {
        vec![
            alan_sid.to_string(),
            self.kurulum.ortak_sid.clone(),
            EVERYONE.into(),
            USERS.into(),
            RESTRICTED.into(),
            "OTURUM".into(),
        ]
    }

    fn kosu(&self, s: &Senaryo) -> Sonuc<OturumSonucu> {
        let k = match s.kim {
            Kim::Agsiz => &self.kurulum.agsiz,
            Kim::Agli => &self.kurulum.agli,
        };
        let parola = self
            .parolalar
            .get(&k.ad)
            .ok_or_else(|| format!("{} için parola yok", k.ad))?;
        let istek = Istek {
            komut: s.komut.clone(),
            dizin: s.dizin.clone(),
            ortam: self.ortam(s.ozel, s.cargo_home.as_deref()),
            kisitlayicilar: s.kisitlayicilar.clone().unwrap_or_default(),
            // Çalışma alanı SID'i kümenin ilk öğesidir.
            dacl_ek: s
                .kisitlayicilar
                .as_ref()
                .and_then(|k| k.first().cloned())
                .into_iter()
                .collect(),
            kisitsiz: s.kisitlayicilar.is_none(),
            ui_kisit: s.ui_kisit,
            ayricaliklari_koru: s.ayricaliklari_koru,
        };
        let yardimci = format!(r"{KOK}\bin\kalfa-prototip.exe");
        win::oturumla_calistir(
            &k.ad,
            parola,
            &yardimci,
            &format!("\"{yardimci}\" calistir"),
            &format!(r"{KOK}\bin"),
            &serde_json::to_vec(&istek).map_err(|e| e.to_string())?,
            Duration::from_secs(180),
        )
    }
}

pub(crate) fn ilk_satir(s: &OturumSonucu) -> String {
    let metin = if s.standart.trim().is_empty() {
        &s.hata
    } else {
        &s.standart
    };
    let satir: String = metin
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .take(3)
        .collect::<Vec<_>>()
        .join(" / ")
        .chars()
        .filter(|c| !c.is_control())
        .take(160)
        .collect();
    satir.replace('|', "/")
}

fn olc(argumanlar: &[String]) -> Sonuc<()> {
    let (kurulum, parolalar, yerel) = yukle()?;
    // Deneme ağacını ve çalışma alanı SID'lerini agac.ps1 kurar (A7: S-1-9 ve
    // S-1-5-21 biçimleri).
    let ham = std::fs::read(yerel.join("agac.json"))
        .map_err(|e| format!("agac.json okunamadı ({e}); önce agac.ps1 çalıştırılmalı"))?;
    let agac: Agac = serde_json::from_slice(ham.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&ham))
        .map_err(|e| format!("agac.json: {e}"))?;
    let kopya = Path::new(KOK).join("bin").join("kalfa-prototip.exe");
    if !kopya.is_file() {
        return Err("deneme ağacı yok; önce agac.ps1 çalıştırılmalı".into());
    }
    // Yardımcının kopyası bu sürümle aynı olsun; dosyanın izinleri korunur.
    std::fs::copy(std::env::current_exe().map_err(|e| e.to_string())?, &kopya)
        .map_err(|e| format!("yardımcı kopyalanamadı: {e}"))?;
    let alan_a_sid = agac.alan_a_sid;
    let alan_b_sid = agac.alan_b_sid;
    let dinleyici = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = dinleyici.local_addr().map_err(|e| e.to_string())?.port();
    std::thread::spawn(move || {
        for mut b in dinleyici.incoming().flatten() {
            let mut tampon = [0u8; 1024];
            let _ = b.read(&mut tampon);
            let _ = b.write_all(b"HTTP/1.0 200 OK\r\nContent-Length: 13\r\n\r\nGERI-DONGU-OK");
        }
    });
    let o = Ortam {
        kurulum,
        parolalar,
        yerel,
        alan_a_sid,
        alan_b_sid,
        path: arac_yolu(),
        port,
        pencere: ui::pencere_ac(),
        boru: boru_ac(),
    };
    let profil_vardi = Path::new(r"C:\Users").join(&o.kurulum.agsiz.ad).exists();

    let secili: Vec<&str> = argumanlar.iter().map(String::as_str).collect();
    let mut satirlar = Vec::new();
    let mut ek = Vec::new();
    ek.push(format!(
        "- Yalıtım kullanıcıları Users grubunda: {}",
        o.kurulum.users_grubunda
    ));
    ek.push(format!("- Alan A SID'i (S-1-9): `{}`", o.alan_a_sid));
    ek.push(format!("- Alan B SID'i (S-1-5-21): `{}`", o.alan_b_sid));
    ek.push(format!("- Ortak okuma SID'i: `{}`", o.kurulum.ortak_sid));
    ek.push(format!("- Yalıtımın PATH'i: `{}`", o.path));
    ek.push(format!(
        "- Ağsız kullanıcı `{}` ({}), ağlı `{}` ({})",
        o.kurulum.agsiz.ad, o.kurulum.agsiz.sid, o.kurulum.agli.ad, o.kurulum.agli.sid
    ));

    // P6 önce: ilk çağrı profil oluşturabilir.
    if secili.is_empty()
        || secili
            .iter()
            .any(|s| "p6".starts_with(s) || s.starts_with("p6"))
    {
        let mut sureler = Vec::new();
        let mut atama = String::new();
        for i in 0..11 {
            let s = senaryo(
                &o,
                "p6",
                "cmd /c exit 0",
                Kim::Agsiz,
                Some(o.tam(&o.alan_a_sid)),
                cmd("exit 0"),
                basarili(),
            );
            let r = o.kosu(&s)?;
            if i == 0 {
                ek.push(format!(
                    "- P6 ilk çağrı: {} ms (profil önceden {})",
                    r.sure.as_millis(),
                    if profil_vardi { "vardı" } else { "yoktu" }
                ));
                atama = match &r.ise_atama {
                    Ok(()) => "başarılı".into(),
                    Err(e) => e.clone(),
                };
            } else {
                sureler.push(r.sure.as_millis());
            }
            if r.cikis != Some(0) {
                ek.push(format!(
                    "- P6 çağrı {i} başarısız: {} {}",
                    ilk_satir(&r),
                    r.hata.trim()
                ));
                break;
            }
        }
        sureler.sort();
        if !sureler.is_empty() {
            ek.push(format!(
                "- P6 sonraki {} çağrı: en az {} / ortanca {} / en çok {} ms",
                sureler.len(),
                sureler[0],
                sureler[sureler.len() / 2],
                sureler[sureler.len() - 1]
            ));
        }
        ek.push(format!(
            "- P6 dönen tutamaçla Kalfa'nın işine atama: {atama}"
        ));
    }

    let once = ui::GELEN.load(Ordering::SeqCst);
    for s in senaryolar(&o) {
        if !secili.is_empty() && !secili.iter().any(|p| s.kod.starts_with(p)) {
            continue;
        }
        let r = o.kosu(&s);
        let (sonuc, durum) = match r {
            Ok(r) => {
                let d = match (s.denetim)(&r) {
                    Some(true) => "✔",
                    Some(false) => "✖",
                    None => "·",
                };
                let kod = r.cikis.map_or("zaman aşımı".into(), |k| k.to_string());
                (
                    format!("{kod}: {} ({} ms)", ilk_satir(&r), r.sure.as_millis()),
                    d,
                )
            }
            Err(e) => (e, "✖"),
        };
        println!("{durum} {} {} → {sonuc}", s.kod, s.aciklama);
        satirlar.push(format!(
            "| {} | {} | {} | {} | {} |",
            s.kod, s.aciklama, s.beklenen, sonuc, durum
        ));
    }
    ek.push(format!(
        "- P10 geliştirici penceresine gelen ileti sayısı: {}",
        ui::GELEN.load(Ordering::SeqCst) - once
    ));
    // P7'nin son durumu: dosyalar yerinde mi?
    let a = Path::new(KOK).join("alan-a");
    ek.push(format!(
        "- P7 sonrası: .git/config \"{}\", pre-commit {}, hooks {}",
        std::fs::read_to_string(a.join(".git").join("config"))
            .unwrap_or_default()
            .trim(),
        if a.join(".git").join("hooks").join("pre-commit").exists() {
            "yerinde"
        } else {
            "SİLİNDİ"
        },
        if a.join(".git").join("hooks").is_dir() {
            "yerinde"
        } else {
            "TAŞINDI"
        },
    ));

    let rapor = format!(
        "# 2c prototip ölçümü\n\n{}\n\n| Kod | Senaryo | Beklenen | Sonuç | |\n| --- | --- | --- | --- | --- |\n{}\n",
        ek.join("\n"),
        satirlar.join("\n")
    );
    let dosya = Path::new(KOK).join("sonuc.md");
    std::fs::write(&dosya, &rapor).map_err(|e| e.to_string())?;
    println!("\n{}", ek.join("\n"));
    println!("\nrapor: {}", dosya.display());
    Ok(())
}

/// Geliştirici tarafında varsayılan güvenlik tanımlayıcısıyla bir adlandırılmış
/// boru açar (P5). Varsayılan DACL Everyone'a okuma verir; kısıtlı kümede
/// Everyone olduğu için yalıtımın bağlanıp bağlanamadığı ölçülür. Bağlanan
/// her istemciye "BORU-OK" yazılır.
pub(crate) fn boru_ac() -> String {
    use windows_sys::Win32::Storage::FileSystem::{PIPE_ACCESS_DUPLEX, WriteFile};
    use windows_sys::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_TYPE_BYTE, PIPE_WAIT,
    };
    let ad = format!("kalfa-prototip-{}", rastgele());
    let tam = format!(r"\\.\pipe\{ad}");
    std::thread::spawn(move || {
        loop {
            // SAFETY: boru tutamacı bu döngüye aittir.
            unsafe {
                let h = CreateNamedPipeW(
                    win::genis(&tam).as_ptr(),
                    PIPE_ACCESS_DUPLEX,
                    PIPE_TYPE_BYTE | PIPE_WAIT,
                    1,
                    512,
                    512,
                    0,
                    std::ptr::null(),
                );
                if h.is_null() || h == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
                    return;
                }
                let h = win::Tutamac(h);
                if ConnectNamedPipe(h.0, std::ptr::null_mut()) != 0
                    || windows_sys::Win32::Foundation::GetLastError() == 535
                {
                    let veri = b"BORU-OK\r\n";
                    let mut yazilan = 0u32;
                    WriteFile(
                        h.0,
                        veri.as_ptr(),
                        veri.len() as u32,
                        &mut yazilan,
                        std::ptr::null_mut(),
                    );
                    DisconnectNamedPipe(h.0);
                }
            }
        }
    });
    ad
}

pub(crate) fn rastgele() -> u32 {
    let mut b = [0u8; 4];
    // SAFETY: tampon 4 bayttır.
    unsafe {
        windows_sys::Win32::Security::Cryptography::BCryptGenRandom(
            std::ptr::null_mut(),
            b.as_mut_ptr(),
            4,
            windows_sys::Win32::Security::Cryptography::BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        );
    }
    u32::from_le_bytes(b)
}

fn senaryo(
    o: &Ortam,
    kod: &str,
    aciklama: &str,
    kim: Kim,
    kisitlayicilar: Option<Vec<String>>,
    komut: String,
    denetim: Denetim,
) -> Senaryo {
    let _ = o;
    Senaryo {
        kod: kod.into(),
        aciklama: aciklama.into(),
        beklenen: "",
        kim,
        kisitlayicilar,
        ui_kisit: true,
        ayricaliklari_koru: false,
        dizin: format!(r"{KOK}\alan-a"),
        ozel: "a",
        cargo_home: None,
        komut,
        denetim,
    }
}

#[allow(clippy::too_many_arguments)]
fn ekle<'a>(
    v: &'a mut Vec<Senaryo>,
    o: &Ortam,
    kod: &str,
    aciklama: &str,
    beklenen: &'static str,
    kim: Kim,
    kisitlayicilar: Option<Vec<String>>,
    komut: String,
    denetim: Denetim,
) -> &'a mut Senaryo {
    let mut s = senaryo(o, kod, aciklama, kim, kisitlayicilar, komut, denetim);
    s.beklenen = beklenen;
    v.push(s);
    v.last_mut().expect("az önce eklendi")
}

fn senaryolar(o: &Ortam) -> Vec<Senaryo> {
    let tam_a = o.tam(&o.alan_a_sid);
    let tam_b = o.tam(&o.alan_b_sid);
    let mut v: Vec<Senaryo> = Vec::new();
    // P1: sistem ve program dizinleri okunur, kullanıcı klasörleri okunmaz.
    ekle(
        &mut v,
        o,
        "p1-sistem",
        "hosts dosyasını okuma",
        "okunur",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd(r"type C:\Windows\System32\drivers\etc\hosts >NUL && echo TAMAM"),
        basarili(),
    );
    ekle(
        &mut v,
        o,
        "p1-program",
        "Program Files listeleme",
        "okunur",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd(r#"dir "C:\Program Files" >NUL && echo TAMAM"#),
        basarili(),
    );
    ekle(
        &mut v,
        o,
        "p1-projeler-oku",
        r"C:\projeler benzeri klasörde dosya okuma",
        "okunmaz",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd(&format!(r"type {PROJELER}\gizli.txt")),
        icermez("PROJE-GIZLI"),
    );
    ekle(
        &mut v,
        o,
        "p1-projeler-liste",
        r"C:\projeler benzeri klasörü listeleme",
        "listelenmez",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd(&format!(r"dir /b {PROJELER}")),
        icermez("gizli.txt"),
    );
    ekle(
        &mut v,
        o,
        "p1-projeler-kisitsiz",
        "aynısı, kısıtsız belirteçle (karşılaştırma)",
        "okunur (kısıtsız)",
        Kim::Agsiz,
        None,
        cmd(&format!(r"type {PROJELER}\gizli.txt")),
        olcum(),
    );
    ekle(
        &mut v,
        o,
        "p1-aap",
        "ALL APPLICATION PACKAGES kısıtlayıcı SID olarak",
        "kabul edilir (Dal A varsayımı)",
        Kim::Agsiz,
        Some(vec![AAP.into(), EVERYONE.into(), RESTRICTED.into()]),
        cmd("ver"),
        basarili(),
    );
    ekle(
        &mut v,
        o,
        "p1-yetenek",
        "addan türetilen yetenek SID'i (S-1-15-3) kısıtlayıcı olarak",
        "ölçüm (A7)",
        Kim::Agsiz,
        Some(vec![
            win::yetenek_sidi("KalfaCalismaAlani_deneme").unwrap_or_default(),
            EVERYONE.into(),
            RESTRICTED.into(),
        ]),
        cmd("ver"),
        olcum(),
    );
    ekle(
        &mut v,
        o,
        "p1-alan",
        "kendi çalışma alanında okuma ve yazma",
        "çalışır",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd("type normal.txt && echo x> yeni.txt && echo TAMAM"),
        basarili(),
    );

    // P2: araç zincirleri; kısıtlayıcı küme daraltılarak.
    let araclar: [(&str, String); 9] = [
        ("cmd", cmd("ver")),
        ("powershell", ps("$PSVersionTable.PSVersion.Major")),
        ("git", cmd("git --version")),
        (
            "git-depo",
            cmd(
                "git init -q depo && cd depo && echo a> a.txt && git add a.txt && git -c user.name=k -c user.email=k@k commit -qm ilk && git log --oneline",
            ),
        ),
        ("node", cmd("node -v")),
        ("npm", cmd("npm -v")),
        (
            "python",
            cmd("python -c \"import sys, ssl; print(sys.version.split()[0])\""),
        ),
        ("cargo", cmd("cargo -V")),
        ("rustc", cmd("rustc -V")),
    ];
    let kumeler: Vec<(&str, Option<Vec<String>>)> = vec![
        ("tam", Some(tam_a.clone())),
        (
            "everyone-yok",
            Some(tam_a.iter().filter(|s| *s != EVERYONE).cloned().collect()),
        ),
        (
            "users-yok",
            Some(tam_a.iter().filter(|s| *s != USERS).cloned().collect()),
        ),
        (
            "restricted-yok",
            Some(tam_a.iter().filter(|s| *s != RESTRICTED).cloned().collect()),
        ),
        (
            "oturum-yok",
            Some(tam_a.iter().filter(|s| *s != "OTURUM").cloned().collect()),
        ),
        (
            "ortak-yok",
            Some(
                tam_a
                    .iter()
                    .filter(|s| **s != o.kurulum.ortak_sid)
                    .cloned()
                    .collect(),
            ),
        ),
    ];
    let ek = rastgele();
    for (kume, kis) in &kumeler {
        for (ad, komut) in &araclar {
            // Her küme kendi deposunu açar; ağaç koşular arasında korunur.
            let komut = komut.replace("depo", &format!("depo-{kume}-{ek}"));
            ekle(
                &mut v,
                o,
                &format!("p2-{kume}-{ad}"),
                &format!("{ad} ({kume})"),
                "çalışır",
                Kim::Agsiz,
                kis.clone(),
                komut.clone(),
                basarili(),
            );
        }
    }

    // P3: A'nın SID'iyle B'ye, B'ninkiyle A'ya erişilmez; ikisi kendi alanında çalışır.
    ekle(
        &mut v,
        o,
        "p3-a-b-oku",
        "A'nın SID'iyle B'yi okuma",
        "okunmaz",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd(&format!(r"type {KOK}\alan-b\normal.txt")),
        icermez("ALAN-b"),
    );
    ekle(
        &mut v,
        o,
        "p3-a-b-yaz",
        "A'nın SID'iyle B'ye yazma",
        "yazılmaz",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd(&format!(r"echo x> {KOK}\alan-b\sizinti.txt")),
        basarisiz(),
    );
    let s = ekle(
        &mut v,
        o,
        "p3-b-kendi",
        "B'nin (S-1-5-21) SID'iyle kendi alanında okuma ve yazma",
        "çalışır",
        Kim::Agsiz,
        Some(tam_b.clone()),
        cmd("type normal.txt && echo x> yeni.txt && echo TAMAM"),
        basarili(),
    );
    s.dizin = format!(r"{KOK}\alan-b");
    s.ozel = "b";
    let s = ekle(
        &mut v,
        o,
        "p3-b-a-oku",
        "B'nin SID'iyle A'yı okuma",
        "okunmaz",
        Kim::Agsiz,
        Some(tam_b.clone()),
        cmd(&format!(r"type {KOK}\alan-a\normal.txt")),
        icermez("ALAN-a"),
    );
    s.dizin = format!(r"{KOK}\alan-b");
    s.ozel = "b";

    // P4: geliştiricinin profili.
    let profil = std::env::var("USERPROFILE").unwrap_or_default();
    ekle(
        &mut v,
        o,
        "p4-profil",
        "geliştiricinin profilini listeleme",
        "listelenmez",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd(&format!(r#"dir /b "{profil}""#)),
        basarisiz(),
    );
    ekle(
        &mut v,
        o,
        "p4-kanarya",
        "profildeki dosyayı okuma",
        "okunmaz",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd(&format!(r#"type "{}\kanarya.txt""#, o.yerel.display())),
        icermez("PROFIL-KANARYA"),
    );
    ekle(
        &mut v,
        o,
        "p4-kimlik",
        "geliştiricinin Credentials dizini",
        "listelenmez",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd(&format!(
            r#"dir /b "{profil}\AppData\Local\Microsoft\Credentials""#
        )),
        basarisiz(),
    );
    ekle(
        &mut v,
        o,
        "p4-kanarya-kisitsiz",
        "profildeki dosya, kısıtsız belirteçle",
        "okunmaz",
        Kim::Agsiz,
        None,
        cmd(&format!(r#"type "{}\kanarya.txt""#, o.yerel.display())),
        icermez("PROFIL-KANARYA"),
    );

    // P5: ağ.
    let curl = r"C:\Windows\System32\curl.exe";
    ekle(
        &mut v,
        o,
        "p5-agsiz-dis",
        "ağsız kullanıcı, dış HTTPS",
        "bağlanmaz",
        Kim::Agsiz,
        Some(tam_a.clone()),
        format!(r#"{curl} -sS -m 8 -o NUL -w "%{{http_code}}" https://example.com"#),
        basarisiz(),
    );
    ekle(
        &mut v,
        o,
        "p5-agsiz-geri-dongu",
        "ağsız kullanıcı, 127.0.0.1",
        "ölçüm",
        Kim::Agsiz,
        Some(tam_a.clone()),
        format!("{curl} -sS -m 5 http://127.0.0.1:{}/", o.port),
        olcum(),
    );
    for boru in ["openssh-ssh-agent", "docker_engine"] {
        ekle(
            &mut v,
            o,
            &format!("p5-boru-{boru}"),
            &format!(r"\\.\pipe\{boru}"),
            "ölçüm",
            Kim::Agsiz,
            Some(tam_a.clone()),
            ps(&format!(
                "$p = New-Object IO.Pipes.NamedPipeClientStream('.', '{boru}'); try {{ $p.Connect(500); 'baglandi' }} catch {{ $_.Exception.GetType().Name + ': ' + $_.Exception.Message }}"
            )),
            olcum(),
        );
    }
    ekle(
        &mut v,
        o,
        "p5-agli-dis",
        "ağlı kullanıcı, dış HTTPS",
        "bağlanır",
        Kim::Agli,
        Some(tam_a.clone()),
        format!(r#"{curl} -sS -m 8 -o NUL -w "%{{http_code}}" https://example.com"#),
        basarili(),
    );
    for (kim, ad) in [(Kim::Agsiz, "agsiz"), (Kim::Agli, "agli")] {
        ekle(
            &mut v,
            o,
            &format!("p5-{ad}-http"),
            &format!("{ad} kullanıcı, dış HTTP (TLS'siz)"),
            if kim == Kim::Agli {
                "bağlanır"
            } else {
                "bağlanmaz"
            },
            kim,
            Some(tam_a.clone()),
            format!(r#"{curl} -sS -m 8 -o NUL -w "%{{http_code}}" http://example.com"#),
            if kim == Kim::Agli {
                basarili()
            } else {
                basarisiz()
            },
        );
    }
    ekle(
        &mut v,
        o,
        "p5-agli-https-iptalsiz",
        "ağlı kullanıcı, HTTPS, sertifika iptal denetimi kapalı",
        "ölçüm",
        Kim::Agli,
        Some(tam_a.clone()),
        format!(
            r#"{curl} --ssl-no-revoke -sS -m 8 -o NUL -w "%{{http_code}}" https://example.com"#
        ),
        olcum(),
    );
    ekle(
        &mut v,
        o,
        "p5-agli-https-kisitsiz",
        "ağlı kullanıcı, HTTPS, kısıtsız belirteçle (karşılaştırma)",
        "ölçüm",
        Kim::Agli,
        None,
        format!(r#"{curl} -sS -m 8 -o NUL -w "%{{http_code}}" https://example.com"#),
        olcum(),
    );
    ekle(
        &mut v,
        o,
        "p5-agli-python-https",
        "ağlı kullanıcı, python ile HTTPS",
        "ölçüm",
        Kim::Agli,
        Some(tam_a.clone()),
        cmd(
            "python -c \"import urllib.request as u; print(u.urlopen('https://example.com', timeout=8).status)\"",
        ),
        olcum(),
    );
    ekle(
        &mut v,
        o,
        "p5-schannel-ayrinti",
        "ağlı kullanıcı, HTTPS, curl'ün schannel iletileri",
        "ölçüm",
        Kim::Agli,
        Some(tam_a.clone()),
        cmd(&format!(
            r#"{curl} -v -m 8 -o NUL https://example.com 2>&1 | findstr /i "schannel SEC_E""#
        )),
        olcum(),
    );
    // Schannel'in araçlara yansıması: git'in iki TLS arka ucu ve cargo.
    for (arka, ad) in [("openssl", "git-openssl"), ("schannel", "git-schannel")] {
        ekle(
            &mut v,
            o,
            &format!("p5-{ad}"),
            &format!("ağlı kullanıcı, git ls-remote HTTPS ({arka})"),
            "ölçüm",
            Kim::Agli,
            Some(tam_a.clone()),
            cmd(&format!(
                "git -c http.sslBackend={arka} ls-remote https://github.com/rust-lang/cfg-if HEAD"
            )),
            olcum(),
        );
    }
    ekle(
        &mut v,
        o,
        "p5-cargo-ag",
        "ağlı kullanıcı, cargo search (crates.io)",
        "ölçüm",
        Kim::Agli,
        Some(tam_a.clone()),
        cmd("cargo search cfg-if --limit 1"),
        olcum(),
    );
    ekle(
        &mut v,
        o,
        "p5-schannel-ayricalikli",
        "ağlı kullanıcı, HTTPS, ayrıcalıklar kaldırılmadan",
        "ölçüm",
        Kim::Agli,
        Some(tam_a.clone()),
        format!(r#"{curl} -sS -m 8 -o NUL -w "%{{http_code}}" https://example.com"#),
        olcum(),
    )
    .ayricaliklari_koru = true;
    // Güvenlik duvarının kullanıcı koşulu kısıtlı belirteçte tutuyor mu?
    ekle(
        &mut v,
        o,
        "p5-agsiz-http-kisitsiz",
        "ağsız kullanıcı, HTTP, kısıtsız belirteçle (karşılaştırma)",
        "bağlanmaz",
        Kim::Agsiz,
        None,
        format!(r#"{curl} -sS -m 8 -o NUL -w "%{{http_code}}" http://example.com"#),
        basarisiz(),
    );
    // Schannel: kümeye hangi SID eklenince çalışıyor?
    for (ek, ad) in [
        ("S-1-5-11", "authusers"),
        ("S-1-5-4", "interactive"),
        ("S-1-2-1", "konsol"),
        ("S-1-2-0", "local"),
        ("S-1-5-113", "yerel-hesap"),
        ("OTURUM", "oturum"),
        ("KULLANICI", "kullanici"),
    ] {
        let mut kume = tam_a.clone();
        kume.push(ek.into());
        ekle(
            &mut v,
            o,
            &format!("p5-schannel-{ad}"),
            &format!("ağlı kullanıcı, HTTPS, kümeye {ek} eklenince"),
            "ölçüm",
            Kim::Agli,
            Some(kume),
            format!(r#"{curl} -sS -m 8 -o NUL -w "%{{http_code}}" https://example.com"#),
            olcum(),
        );
    }
    for (yon, ad) in [("In", "oku"), ("InOut", "oku-yaz")] {
        ekle(
            &mut v,
            o,
            &format!("p5-boru-varsayilan-{ad}"),
            &format!("geliştiricinin varsayılan izinli borusu ({yon})"),
            "ölçüm",
            Kim::Agsiz,
            Some(tam_a.clone()),
            ps(&format!(
                "$p = New-Object IO.Pipes.NamedPipeClientStream('.', '{}', '{yon}'); try {{ $p.Connect(2000); (New-Object IO.StreamReader($p)).ReadLine() }} catch {{ $_.Exception.GetType().Name + ': ' + $_.Exception.Message }}",
                o.boru
            )),
            olcum(),
        );
    }

    // P7: gizli dosya ve korumalı yollar.
    ekle(
        &mut v,
        o,
        "p7-gizli",
        ".env okuma",
        "okunmaz",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd("type .env"),
        icermez("GIZLI-ENV"),
    );
    ekle(
        &mut v,
        o,
        "p7-config-yaz",
        ".git/config'e ekleme",
        "yazılmaz",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd(r"echo x>> .git\config"),
        basarisiz(),
    );
    ekle(
        &mut v,
        o,
        "p7-config-sil",
        ".git/config'i silme",
        "silinmez",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd(r"del /q .git\config & if exist .git\config (exit 1) else (exit 0)"),
        basarisiz(),
    );
    ekle(
        &mut v,
        o,
        "p7-config-uzerine",
        ".git/config'in üzerine taşıma",
        "taşınmaz",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd(r"echo y> k.txt && move /y k.txt .git\config"),
        basarisiz(),
    );
    ekle(
        &mut v,
        o,
        "p7-hook-yeni",
        ".git/hooks'a dosya ekleme",
        "eklenmez",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd(r"echo x> .git\hooks\yeni"),
        basarisiz(),
    );
    ekle(
        &mut v,
        o,
        "p7-hook-sil",
        "pre-commit'i silme",
        "silinmez",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd(
            r"del /q .git\hooks\pre-commit & if exist .git\hooks\pre-commit (exit 1) else (exit 0)",
        ),
        basarisiz(),
    );
    ekle(
        &mut v,
        o,
        "p7-hooks-tasi",
        ".git/hooks dizinini yeniden adlandırma",
        "taşınmaz",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd(r"ren .git\hooks eski-hooks"),
        basarisiz(),
    );

    // P9: cargo derlemesi.
    let s = ekle(
        &mut v,
        o,
        "p9-kutu",
        "bağımlılıksız crate, çalışma alanına özel CARGO_HOME, ağsız",
        "derlenir",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd("cargo build --offline -q && echo TAMAM"),
        basarili(),
    );
    s.dizin = format!(r"{KOK}\alan-a\kutu");
    let s = ekle(
        &mut v,
        o,
        "p9-bagimli-ozel",
        "bağımlı crate, çalışma alanına özel CARGO_HOME, ağsız",
        "derlenmez (önbellek yok)",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd("cargo build --offline -q && echo TAMAM"),
        olcum(),
    );
    s.dizin = format!(r"{KOK}\alan-a\bagimli");
    let s = ekle(
        &mut v,
        o,
        "p9-bagimli-gelistirici",
        "bağımlı crate, geliştiricinin CARGO_HOME'u salt okunur, ağsız",
        "ölçüm",
        Kim::Agsiz,
        Some(tam_a.clone()),
        cmd("cargo build --offline -q && echo TAMAM"),
        olcum(),
    );
    s.dizin = format!(r"{KOK}\alan-a\bagimli");
    s.cargo_home = Some(format!(
        r"{}\.cargo",
        std::env::var("USERPROFILE").unwrap_or_default()
    ));

    // P10: UI kısıtları.
    let yardimci = format!(r"{KOK}\bin\kalfa-prototip.exe ui-dene {}", o.pencere);
    ekle(
        &mut v,
        o,
        "p10-kisitli",
        "UI kısıtlarıyla ileti, pano, pencere bulma",
        "üçü de engellenir",
        Kim::Agsiz,
        Some(tam_a.clone()),
        yardimci.clone(),
        olcum(),
    );
    let s = ekle(
        &mut v,
        o,
        "p10-kisitsiz",
        "UI kısıtları olmadan (karşılaştırma)",
        "ölçüm",
        Kim::Agsiz,
        Some(tam_a.clone()),
        yardimci,
        olcum(),
    );
    s.ui_kisit = false;
    v
}
