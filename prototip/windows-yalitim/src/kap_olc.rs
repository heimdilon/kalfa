//! `kap`: AppContainer ek prototipi (2c). Kısıtlı belirteç yolunda açık kalan
//! P1 (Dal A), A8 (Schannel) ve A9 (geri döngü) AppContainer'da ölçülür; P2–P10
//! aynı senaryolarla yinelenir. Ayrı kullanıcı ve parola yoktur.
//!
//! Eşlemeler:
//! - Çalışma alanı SID'i → AppContainer'ın paket SID'i (alan başına bir profil).
//! - Ortak okuma SID'i → addan türetilen bir yetenek SID'i; araç zinciri
//!   dizinleri ona okuma izni verir.
//! - Ağ → `internetClient` (ve `privateNetworkClientServer`) yetenekleri; ağsız
//!   çalıştırmada yetenek verilmez.
//!
//! Git ve cmd, çalışma dizininin yolunu üst dizinleri listeleyerek
//! kanonikleştiriyor; AppContainer `C:\`'yi listeleyemediği için bu yolla
//! düşüyorlar. Senaryolar iki kipte koşar: yol (`C:\KalfaPrototip\…`) ve
//! sürücü (deneme ağacı ölçüm boyunca bir sürücü harfine bağlanır, `subst`
//! gibi).
//!
//!   kalfa-prototip kap hazirla     profilleri oluşturur, kap.json'u yazar
//!   kalfa-prototip kap olc [önek…] ölçümleri koşar
//!   kalfa-prototip kap sil         profilleri ve kap.json'u siler
//!   kalfa-prototip kap dene [--internet] [--lpac] [--arac-yok] [--b] [--dogrudan] [--dizin <d>] [--ortam A=D]… <komut>
//!                                  tek komutu A.nın (--b: B.nin) kabında koşar (hata ayıklama)
//!
//! Deneme ağacını ve araç zinciri izinlerini `kap-kur.ps1` kurar.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::kap::{self, KapIstek};
use crate::olc::{
    Denetim, KOK, PROJELER, arac_yolu, basarili, basarisiz, boru_ac, cmd, icermez, ilk_satir,
    olcum, ps, rastgele, yalitim_ortami,
};
use crate::ui;
use crate::win::{self, OturumSonucu, Sid, Sonuc};

const PROFIL_A: &str = "Kalfa.Prototip.AlanA";
const PROFIL_B: &str = "Kalfa.Prototip.AlanB";
const ARAC_YETENEGI: &str = "kalfaPrototipAracZinciri";
const INTERNET: &str = "S-1-15-3-1";
const OZEL_AG: &str = "S-1-15-3-3";

#[derive(Serialize, Deserialize)]
struct KapKaydi {
    alan_a_sid: String,
    alan_b_sid: String,
    arac_sid: String,
}

#[derive(Clone, Copy, PartialEq)]
enum Ag {
    Yok,
    Internet,
    InternetOzel,
}

struct Senaryo {
    kod: String,
    aciklama: String,
    beklenen: &'static str,
    alan_b: bool,
    ag: Ag,
    /// Araç zinciri yeteneği (ortak okuma).
    arac: bool,
    lpac: bool,
    ek_yetenekler: Vec<&'static str>,
    ui_kisit: bool,
    /// Çalışma dizini ve ortamdaki ağaç yolları sürücü harfinden verilir.
    surucu: bool,
    /// Ortama eklenen ya da oradakinin yerine geçen değişkenler.
    ek_ortam: Vec<(String, String)>,
    dizin: String,
    ozel: &'static str,
    cargo_home: Option<String>,
    komut: String,
    denetim: Denetim,
}

struct Olcer {
    kayit: KapKaydi,
    paket_a: Sid,
    paket_b: Sid,
    yerel: PathBuf,
    path: String,
    port: u16,
    boru: String,
    pencere: isize,
    surucu: Option<Surucu>,
    /// P9: bağlayıcı ve kütüphaneler için ortam.
    msvc: Vec<(String, String)>,
}

/// Deneme ağacına bağlanan sürücü harfi; düşünce kaldırılır.
struct Surucu {
    harf: String,
    hedef: String,
}

impl Surucu {
    fn bagla(hedef: &str) -> Sonuc<Surucu> {
        use windows_sys::Win32::Storage::FileSystem::{DefineDosDeviceW, GetLogicalDrives};
        // SAFETY: parametresiz sorgu.
        let dolu = unsafe { GetLogicalDrives() };
        let harf = (b'K'..=b'Y')
            .rev()
            .find(|h| dolu & (1 << (h - b'A')) == 0)
            .ok_or("boş sürücü harfi yok")?;
        let harf = format!("{}:", harf as char);
        // SAFETY: dizeler NUL ile biter.
        if unsafe { DefineDosDeviceW(0, win::genis(&harf).as_ptr(), win::genis(hedef).as_ptr()) }
            == 0
        {
            return Err(win::hata("DefineDosDeviceW"));
        }
        Ok(Surucu {
            harf,
            hedef: hedef.to_string(),
        })
    }
}

impl Drop for Surucu {
    fn drop(&mut self) {
        use windows_sys::Win32::Storage::FileSystem::{
            DDD_EXACT_MATCH_ON_REMOVE, DDD_REMOVE_DEFINITION, DefineDosDeviceW,
        };
        // SAFETY: dizeler NUL ile biter.
        unsafe {
            DefineDosDeviceW(
                DDD_REMOVE_DEFINITION | DDD_EXACT_MATCH_ON_REMOVE,
                win::genis(&self.harf).as_ptr(),
                win::genis(&self.hedef).as_ptr(),
            )
        };
    }
}

fn yerel() -> Sonuc<PathBuf> {
    Ok(
        PathBuf::from(std::env::var("LOCALAPPDATA").map_err(|e| e.to_string())?)
            .join("KalfaPrototip"),
    )
}

pub fn ana(argumanlar: &[String]) -> i32 {
    let r = match argumanlar.first().map(String::as_str) {
        Some("hazirla") => hazirla(),
        Some("sil") => sil(),
        Some("olc") => olc(&argumanlar[1..]),
        Some("dene") => dene(&argumanlar[1..]),
        _ => Err("kullanım: kalfa-prototip kap [hazirla | olc [önek…] | sil]".into()),
    };
    match r {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("hata: {e}");
            1
        }
    }
}

fn hazirla() -> Sonuc<()> {
    let kayit = KapKaydi {
        alan_a_sid: kap::profil(PROFIL_A)?.metin(),
        alan_b_sid: kap::profil(PROFIL_B)?.metin(),
        arac_sid: win::yetenek_sidi(ARAC_YETENEGI)?,
    };
    let y = yerel()?;
    std::fs::create_dir_all(&y).map_err(|e| e.to_string())?;
    std::fs::write(
        y.join("kap.json"),
        serde_json::to_vec_pretty(&kayit).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!("Alan A paket SID'i: {}", kayit.alan_a_sid);
    println!("Alan B paket SID'i: {}", kayit.alan_b_sid);
    println!("Araç zinciri yeteneği: {}", kayit.arac_sid);
    Ok(())
}

fn sil() -> Sonuc<()> {
    for ad in [PROFIL_A, PROFIL_B] {
        match kap::profil_sil(ad) {
            Ok(()) => println!("profil silindi: {ad}"),
            Err(e) => println!("{e}"),
        }
    }
    let _ = std::fs::remove_file(yerel()?.join("kap.json"));
    Ok(())
}

impl Olcer {
    fn kosu(&self, s: &Senaryo) -> Sonuc<OturumSonucu> {
        let mut yetenekler = Vec::new();
        if s.arac {
            yetenekler.push(Sid::metinden(&self.kayit.arac_sid)?);
        }
        if s.ag != Ag::Yok {
            yetenekler.push(Sid::metinden(INTERNET)?);
        }
        if s.ag == Ag::InternetOzel {
            yetenekler.push(Sid::metinden(OZEL_AG)?);
        }
        for ad in &s.ek_yetenekler {
            yetenekler.push(Sid::metinden(&win::yetenek_sidi(ad)?)?);
        }
        let esle = |m: &str| match (&self.surucu, s.surucu) {
            (Some(h), true) => m.replace(KOK, &h.harf),
            _ => m.to_string(),
        };
        // Ağaç kurulmadan da ağaçsız senaryolar koşabilsin.
        let dizin = esle(&s.dizin);
        let dizin = if Path::new(&dizin).is_dir() {
            dizin
        } else {
            r"C:\Windows".to_string()
        };
        let mut ortam: Vec<(String, String)> =
            yalitim_ortami(&self.path, s.ozel, s.cargo_home.as_deref())
                .into_iter()
                .map(|(a, d)| (a, esle(&d)))
                .collect();
        for (a, d) in &s.ek_ortam {
            ortam.retain(|(v, _)| !v.eq_ignore_ascii_case(a));
            ortam.push((a.clone(), esle(d)));
        }
        kap::kapta_calistir(
            &KapIstek {
                komut: &s.komut,
                dizin: &dizin,
                ortam: &ortam,
                paket: if s.alan_b {
                    &self.paket_b
                } else {
                    &self.paket_a
                },
                yetenekler: &yetenekler,
                lpac: s.lpac,
                ui_kisit: s.ui_kisit,
            },
            Duration::from_secs(180),
        )
    }
}

/// Geliştirici tarafında, yalıtımsız: ALL APPLICATION PACKAGES'a ya da ALL
/// RESTRICTED APPLICATION PACKAGES'a açıkça (miras olmadan) okuma veren
/// dizinler. Profil 3 düzey, C:\ kökü 1 düzey taranır.
fn aap_taramasi(yerel: &Path) -> Vec<String> {
    let betik = r#"$ErrorActionPreference = 'SilentlyContinue'
$aap = @('S-1-15-2-1', 'S-1-15-2-2')
function Tara([string]$kok, [int]$derinlik) {
    Get-ChildItem -LiteralPath $kok -Directory -Force -Recurse -Depth $derinlik |
        Where-Object { -not ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) } |
        ForEach-Object {
            $yol = $_.FullName
            $k = (Get-Acl -LiteralPath $yol).GetAccessRules($true, $false, [Security.Principal.SecurityIdentifier]) |
                Where-Object { $aap -contains $_.IdentityReference.Value -and $_.AccessControlType -eq 'Allow' -and (([int]$_.FileSystemRights -band 0x80000001) -ne 0) }
            if ($k) { '{0} ({1})' -f $yol, ((@($k) | ForEach-Object { $_.IdentityReference.Value } | Sort-Object -Unique) -join ', ') }
        }
}
Tara $env:USERPROFILE 3
Tara 'C:\' 0
"#;
    let dosya = yerel.join("aap-tarama.ps1");
    // PowerShell 5.1 BOM'suz betiği ANSI okur; Türkçe yollar için BOM.
    let mut icerik = vec![0xEF, 0xBB, 0xBF];
    icerik.extend_from_slice(betik.as_bytes());
    if std::fs::write(&dosya, icerik).is_err() {
        return vec!["tarama betiği yazılamadı".into()];
    }
    let cikti =
        std::process::Command::new(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&dosya)
            .output();
    let _ = std::fs::remove_file(&dosya);
    match cikti {
        Ok(c) => String::from_utf8_lossy(&c.stdout)
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect(),
        Err(e) => vec![format!("tarama çalışmadı: {e}")],
    }
}

/// Geliştirici tarafında bulunan MSVC bağlayıcısı ve Windows SDK
/// kütüphaneleri. AppContainer Visual Studio'nun kurulum kaydını (ProgramData)
/// okuyamıyor; rustc link.exe'yi bulamıyor. Bulunamazsa boş.
fn msvc_ortami(path: &str) -> Vec<(String, String)> {
    let son_alt_dizin = |d: &Path, kosul: &dyn Fn(&Path) -> bool| {
        let mut v: Vec<PathBuf> = std::fs::read_dir(d)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .filter(|p| kosul(p))
            .collect();
        v.sort();
        v.pop()
    };
    let kurulum = std::process::Command::new(
        r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe",
    )
    .args(["-latest", "-products", "*", "-property", "installationPath"])
    .output()
    .ok()
    .map(|c| String::from_utf8_lossy(&c.stdout).trim().to_string())
    .unwrap_or_default();
    if kurulum.is_empty() {
        return Vec::new();
    }
    let Some(msvc) = son_alt_dizin(&Path::new(&kurulum).join(r"VC\Tools\MSVC"), &|p| {
        p.join(r"bin\Hostx64\x64\link.exe").is_file()
    }) else {
        return Vec::new();
    };
    let Some(sdk) = son_alt_dizin(
        Path::new(r"C:\Program Files (x86)\Windows Kits\10\Lib"),
        &|p| p.join(r"um\x64").is_dir(),
    ) else {
        return Vec::new();
    };
    vec![
        (
            "PATH".into(),
            format!(r"{path};{}\bin\Hostx64\x64", msvc.display()),
        ),
        (
            "LIB".into(),
            format!(
                r"{}\lib\x64;{s}\um\x64;{s}\ucrt\x64",
                msvc.display(),
                s = sdk.display()
            ),
        ),
    ]
}

fn kaydi_oku(yerel: &Path) -> Sonuc<KapKaydi> {
    let ham = std::fs::read(yerel.join("kap.json"))
        .map_err(|e| format!("kap.json okunamadı ({e}); önce: kalfa-prototip kap hazirla"))?;
    serde_json::from_slice(ham.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&ham))
        .map_err(|e| format!("kap.json: {e}"))
}

fn dene(argumanlar: &[String]) -> Sonuc<()> {
    let yerel = yerel()?;
    let kayit = kaydi_oku(&yerel)?;
    let bayrak = |b: &str| argumanlar.iter().any(|a| a == b);
    let mut dizin = None;
    let mut ortam = Vec::new();
    let mut parcalar = Vec::new();
    let mut i = argumanlar.iter();
    while let Some(a) = i.next() {
        if a == "--dizin" {
            dizin = i.next().cloned();
        } else if a == "--ortam" {
            if let Some((ad, deger)) = i.next().and_then(|o| o.split_once('=')) {
                ortam.push((ad.to_string(), deger.to_string()));
            }
        } else if !a.starts_with("--") {
            parcalar.push(a.clone());
        }
    }
    // --dogrudan: komut satırı cmd'siz, olduğu gibi başlatılır.
    let satir = parcalar.join(" ");
    let komut = if bayrak("--dogrudan") {
        satir
    } else {
        cmd(&satir)
    };
    let mut s = senaryo("dene", "", komut, olcum());
    if let Some(d) = dizin {
        s.dizin = d;
    }
    s.ek_ortam = ortam;
    s.arac = !bayrak("--arac-yok");
    s.lpac = bayrak("--lpac");
    s.alan_b = bayrak("--b");
    if bayrak("--internet") {
        s.ag = Ag::Internet;
    }
    let o = Olcer {
        paket_a: Sid::metinden(&kayit.alan_a_sid)?,
        paket_b: Sid::metinden(&kayit.alan_b_sid)?,
        kayit,
        yerel,
        path: arac_yolu(),
        port: 0,
        boru: String::new(),
        pencere: 0,
        surucu: None,
        msvc: Vec::new(),
    };
    let r = o.kosu(&s)?;
    print!("{}", r.standart);
    eprint!("{}", r.hata);
    println!("[çıkış: {:?}, {} ms]", r.cikis, r.sure.as_millis());
    Ok(())
}

fn olc(argumanlar: &[String]) -> Sonuc<()> {
    let yerel = yerel()?;
    let kayit = kaydi_oku(&yerel)?;
    if !Path::new(KOK).join("alan-a").is_dir() {
        println!("uyarı: deneme ağacı yok (kap-kur.ps1); ağaca bağlı senaryolar düşer\n");
    }
    // P10 yardımcısının kopyası bu sürümle aynı olsun; dosyanın izinleri korunur.
    let kopya = Path::new(KOK).join("bin").join("kalfa-prototip.exe");
    if kopya.is_file() {
        std::fs::copy(std::env::current_exe().map_err(|e| e.to_string())?, &kopya)
            .map_err(|e| format!("yardımcı kopyalanamadı: {e}"))?;
    }
    // Yükleyici hata kutuları geliştiricinin masaüstünde açılmasın; çocuklar
    // bu kipi devralır.
    // SAFETY: yalnızca bu sürecin hata kipini değiştirir.
    unsafe {
        use windows_sys::Win32::System::Diagnostics::Debug::{
            SEM_FAILCRITICALERRORS, SEM_NOGPFAULTERRORBOX, SEM_NOOPENFILEERRORBOX, SetErrorMode,
        };
        SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX);
    }
    let dinleyici = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = dinleyici.local_addr().map_err(|e| e.to_string())?.port();
    std::thread::spawn(move || {
        for mut b in dinleyici.incoming().flatten() {
            let mut tampon = [0u8; 1024];
            let _ = b.read(&mut tampon);
            let _ = b.write_all(b"HTTP/1.0 200 OK\r\nContent-Length: 13\r\n\r\nGERI-DONGU-OK");
        }
    });
    let o = Olcer {
        paket_a: Sid::metinden(&kayit.alan_a_sid)?,
        paket_b: Sid::metinden(&kayit.alan_b_sid)?,
        kayit,
        yerel,
        path: arac_yolu(),
        port,
        boru: boru_ac(),
        pencere: ui::pencere_ac(),
        surucu: if Path::new(KOK).join("alan-a").is_dir() {
            Some(Surucu::bagla(KOK)?)
        } else {
            None
        },
        msvc: msvc_ortami(&arac_yolu()),
    };
    let secili: Vec<&str> = argumanlar.iter().map(String::as_str).collect();
    let secildi = |kod: &str| secili.is_empty() || secili.iter().any(|p| kod.starts_with(p));

    let mut ek = vec![
        format!("- Alan A paket SID'i: `{}`", o.kayit.alan_a_sid),
        format!("- Alan B paket SID'i: `{}`", o.kayit.alan_b_sid),
        format!("- Araç zinciri yeteneği: `{}`", o.kayit.arac_sid),
        format!("- Yalıtımın PATH'i: `{}`", o.path),
        format!(
            "- Sürücü kipi: {}",
            o.surucu
                .as_ref()
                .map_or("ağaç yok".to_string(), |h| format!(
                    "`{}` → `{KOK}`",
                    h.harf
                ))
        ),
        format!(
            "- P9 MSVC ortamı: {}",
            if o.msvc.is_empty() {
                "bulunamadı".to_string()
            } else {
                o.msvc
                    .iter()
                    .map(|(a, d)| format!("`{a}={d}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        ),
    ];

    if secildi("p6") {
        let mut sureler = Vec::new();
        for i in 0..11 {
            let mut s = senaryo("p6", "cmd /c exit 0", cmd("exit 0"), basarili());
            s.arac = false;
            let r = o.kosu(&s)?;
            if r.cikis != Some(0) {
                ek.push(format!("- P6 çağrı {i} başarısız: {}", ilk_satir(&r)));
                break;
            }
            if i == 0 {
                ek.push(format!("- P6 ilk çağrı: {} ms", r.sure.as_millis()));
            } else {
                sureler.push(r.sure.as_millis());
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
    }

    let mut satirlar = Vec::new();
    let once = ui::GELEN.load(Ordering::SeqCst);
    for s in senaryolar(&o) {
        if !secildi(&s.kod) {
            continue;
        }
        let (sonuc, durum) = match o.kosu(&s) {
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
    if secildi("p10") {
        ek.push(format!(
            "- P10 geliştirici penceresine gelen ileti sayısı: {}",
            ui::GELEN.load(Ordering::SeqCst) - once
        ));
    }
    if secildi("p7") {
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
    }
    let tarama = if secildi("aap") {
        let t = aap_taramasi(&o.yerel);
        println!("\nAAP taraması: {} dizin", t.len());
        format!(
            "\n\n## ALL APPLICATION PACKAGES'a açıkça okuma veren dizinler\n\nProfil 3 düzey, `C:\\` 1 düzey; miras alınan izinler sayılmaz.\n\n{}\n",
            if t.is_empty() {
                "(yok)".to_string()
            } else {
                t.iter()
                    .map(|l| format!("- `{l}`"))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        )
    } else {
        String::new()
    };

    let rapor = format!(
        "# 2c AppContainer ölçümü\n\n{}\n\n| Kod | Senaryo | Beklenen | Sonuç | |\n| --- | --- | --- | --- | --- |\n{}\n{tarama}",
        ek.join("\n"),
        satirlar.join("\n")
    );
    let dosya = o.yerel.join("sonuc-kap.md");
    std::fs::write(&dosya, &rapor).map_err(|e| e.to_string())?;
    println!("\n{}", ek.join("\n"));
    println!("\nrapor: {}", dosya.display());
    Ok(())
}

fn senaryo(kod: &str, aciklama: &str, komut: String, denetim: Denetim) -> Senaryo {
    Senaryo {
        kod: kod.into(),
        aciklama: aciklama.into(),
        beklenen: "",
        alan_b: false,
        ag: Ag::Yok,
        arac: true,
        lpac: false,
        ek_yetenekler: Vec::new(),
        ui_kisit: true,
        surucu: false,
        ek_ortam: Vec::new(),
        dizin: format!(r"{KOK}\alan-a"),
        ozel: "a",
        cargo_home: None,
        komut,
        denetim,
    }
}

fn ekle<'a>(
    v: &'a mut Vec<Senaryo>,
    kod: &str,
    aciklama: &str,
    beklenen: &'static str,
    komut: String,
    denetim: Denetim,
) -> &'a mut Senaryo {
    let mut s = senaryo(kod, aciklama, komut, denetim);
    s.beklenen = beklenen;
    v.push(s);
    v.last_mut().expect("az önce eklendi")
}

fn curl(ek: &str, url: &str) -> String {
    format!(r#"C:\Windows\System32\curl.exe {ek} -sS -m 8 -o NUL -w "%{{http_code}}" {url}"#)
}

fn senaryolar(o: &Olcer) -> Vec<Senaryo> {
    let mut v: Vec<Senaryo> = Vec::new();
    let profil = std::env::var("USERPROFILE").unwrap_or_default();

    // P1: sistem ve program dizinleri okunur; ALL APPLICATION PACKAGES'a izin
    // vermeyen her yer (kullanıcı klasörleri, C:\ altındaki klasörler) okunmaz.
    ekle(
        &mut v,
        "p1-sistem",
        "hosts dosyasını okuma",
        "okunur",
        cmd(r"type C:\Windows\System32\drivers\etc\hosts >NUL && echo TAMAM"),
        basarili(),
    );
    ekle(
        &mut v,
        "p1-program",
        "Program Files listeleme",
        "okunur",
        cmd(r#"dir "C:\Program Files" >NUL && echo TAMAM"#),
        basarili(),
    );
    ekle(
        &mut v,
        "p1-projeler-oku",
        r"C:\projeler benzeri klasörde dosya okuma",
        "okunmaz",
        cmd(&format!(r"type {PROJELER}\gizli.txt")),
        icermez("PROJE-GIZLI"),
    );
    ekle(
        &mut v,
        "p1-projeler-liste",
        r"C:\projeler benzeri klasörü listeleme",
        "listelenmez",
        cmd(&format!(r"dir /b {PROJELER}")),
        icermez("gizli.txt"),
    );
    for (ad, yol) in [
        ("kok", r"C:\"),
        ("kullanicilar", r"C:\Users"),
        ("public", r"C:\Users\Public"),
        ("programdata", r"C:\ProgramData"),
    ] {
        ekle(
            &mut v,
            &format!("p1-liste-{ad}"),
            &format!("{yol} listeleme"),
            "ölçüm",
            cmd(&format!(r#"dir /b "{yol}""#)),
            olcum(),
        );
    }
    ekle(
        &mut v,
        "p1-alan",
        "kendi çalışma alanında okuma ve yazma (Low etiketli)",
        "çalışır",
        cmd("type normal.txt && echo x> yeni.txt && echo TAMAM"),
        basarili(),
    );
    let s = ekle(
        &mut v,
        "p1-etiketsiz-oku",
        "izinli ama bütünlük etiketsiz dizinde okuma",
        "ölçüm",
        cmd("type normal.txt"),
        olcum(),
    );
    s.dizin = format!(r"{KOK}\etiketsiz-a");
    let s = ekle(
        &mut v,
        "p1-etiketsiz-yaz",
        "izinli ama bütünlük etiketsiz dizine yazma",
        "ölçüm (Low bütünlük)",
        cmd("echo x> yeni.txt && echo TAMAM"),
        olcum(),
    );
    s.dizin = format!(r"{KOK}\etiketsiz-a");
    for (ad, komut) in [
        (
            "sistem",
            cmd(r"type C:\Windows\System32\drivers\etc\hosts >NUL && echo TAMAM"),
        ),
        (
            "program",
            cmd(r#"dir "C:\Program Files" >NUL && echo TAMAM"#),
        ),
    ] {
        ekle(
            &mut v,
            &format!("p1-lpac-{ad}"),
            &format!("LPAC: {ad}"),
            "ölçüm",
            komut,
            olcum(),
        )
        .lpac = true;
    }

    // P2: araç zincirleri; yetenek kümeleri değiştirilerek.
    let araclar: [(&str, String); 12] = [
        ("cmd", cmd("ver")),
        ("cmd-dir", cmd("dir /b")),
        ("cmd-cd", cmd("mkdir d-cd 2>NUL & cd d-cd && cd")),
        ("powershell", ps("$PSVersionTable.PSVersion.Major")),
        (
            "powershell-dil",
            ps("$ExecutionContext.SessionState.LanguageMode"),
        ),
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
    // (küme, araç yeteneği, LPAC, ek yetenekler, sürücü kipi)
    let kumeler: [(&str, bool, bool, &[&'static str], bool); 5] = [
        ("yol", true, false, &[], false),
        ("surucu", true, false, &[], true),
        ("arac-yok", false, false, &[], true),
        ("lpac", true, true, &[], true),
        ("lpac-kayit", true, true, &["registryRead", "lpacCom"], true),
    ];
    let ek = rastgele();
    for (kume, arac, lpac, yetenekler, surucu) in kumeler {
        for (ad, komut) in &araclar {
            let komut = komut.replace("depo", &format!("depo-{kume}-{ek}"));
            let s = ekle(
                &mut v,
                &format!("p2-{kume}-{ad}"),
                &format!("{ad} ({kume})"),
                "çalışır",
                komut,
                basarili(),
            );
            s.arac = arac;
            s.lpac = lpac;
            s.ek_yetenekler = yetenekler.to_vec();
            s.surucu = surucu;
            if *ad == "powershell-dil" {
                s.beklenen = "FullLanguage";
                s.denetim = Box::new(|r| Some(r.standart.contains("FullLanguage")));
            }
        }
    }

    // P3: iki AppContainer birbirinin alanına erişemez; geliştiricinin
    // süreçlerini açamaz.
    ekle(
        &mut v,
        "p3-a-b-oku",
        "A'nın kabından B'yi okuma",
        "okunmaz",
        cmd(&format!(r"type {KOK}\alan-b\normal.txt")),
        icermez("ALAN-b"),
    );
    ekle(
        &mut v,
        "p3-a-b-yaz",
        "A'nın kabından B'ye yazma",
        "yazılmaz",
        cmd(&format!(r"echo x> {KOK}\alan-b\sizinti.txt")),
        basarisiz(),
    );
    let s = ekle(
        &mut v,
        "p3-b-kendi",
        "B'nin kabından kendi alanında okuma ve yazma",
        "çalışır",
        cmd("type normal.txt && echo x> yeni.txt && echo TAMAM"),
        basarili(),
    );
    s.alan_b = true;
    s.dizin = format!(r"{KOK}\alan-b");
    s.ozel = "b";
    let s = ekle(
        &mut v,
        "p3-b-a-oku",
        "B'nin kabından A'yı okuma",
        "okunmaz",
        cmd(&format!(r"type {KOK}\alan-a\normal.txt")),
        icermez("ALAN-a"),
    );
    s.alan_b = true;
    s.dizin = format!(r"{KOK}\alan-b");
    s.ozel = "b";
    ekle(
        &mut v,
        "p3-gelistirici-sureci",
        "geliştiricinin sürecinin yolunu okuma",
        "okunmaz",
        ps(&format!("(Get-Process -Id {}).Path", std::process::id())),
        icermez("kalfa-prototip"),
    );

    // P4: geliştiricinin profili.
    ekle(
        &mut v,
        "p4-profil",
        "geliştiricinin profilini listeleme",
        "listelenmez",
        cmd(&format!(r#"dir /b "{profil}""#)),
        basarisiz(),
    );
    ekle(
        &mut v,
        "p4-kanarya",
        "profildeki dosyayı okuma",
        "okunmaz",
        cmd(&format!(r#"type "{}\kanarya.txt""#, o.yerel.display())),
        icermez("PROFIL-KANARYA"),
    );
    ekle(
        &mut v,
        "p4-kimlik",
        "geliştiricinin Credentials dizini",
        "listelenmez",
        cmd(&format!(
            r#"dir /b "{profil}\AppData\Local\Microsoft\Credentials""#
        )),
        basarisiz(),
    );
    for (ad, yol) in [
        ("ssh", r".ssh"),
        ("gitconfig", r".gitconfig"),
        ("cargo", r".cargo"),
        ("cargo-kimlik", r".cargo\credentials.toml"),
        ("claude", r".claude"),
        ("roaming", r"AppData\Roaming"),
        ("locallow", r"AppData\LocalLow"),
        ("paketler", r"AppData\Local\Packages"),
    ] {
        ekle(
            &mut v,
            &format!("p4-{ad}"),
            &format!(r"profil\{yol} listeleme"),
            "ölçüm",
            cmd(&format!(r#"dir /b "{profil}\{yol}""#)),
            olcum(),
        );
    }

    // P5: ağ. Yetenek yoksa ağ yok; geri döngü AppContainer'da varsayılan
    // olarak kapalı.
    ekle(
        &mut v,
        "p5-agsiz-https",
        "yeteneksiz, dış HTTPS",
        "bağlanmaz",
        curl("", "https://example.com"),
        basarisiz(),
    );
    ekle(
        &mut v,
        "p5-agsiz-http",
        "yeteneksiz, dış HTTP",
        "bağlanmaz",
        curl("", "http://example.com"),
        basarisiz(),
    );
    ekle(
        &mut v,
        "p5-agsiz-geri-dongu",
        "yeteneksiz, 127.0.0.1",
        "bağlanmaz (A9)",
        curl("", &format!("http://127.0.0.1:{}/", o.port)),
        basarisiz(),
    );
    ekle(
        &mut v,
        "p5-agsiz-dns",
        "yeteneksiz, ad çözme",
        "ölçüm",
        cmd("nslookup example.com"),
        olcum(),
    );
    for (ag, ad) in [(Ag::Internet, "internet"), (Ag::InternetOzel, "ozel")] {
        ekle(
            &mut v,
            &format!("p5-{ad}-https"),
            &format!("{ad}: dış HTTPS (curl, Schannel)"),
            "bağlanır (A8)",
            curl("", "https://example.com"),
            basarili(),
        )
        .ag = ag;
        ekle(
            &mut v,
            &format!("p5-{ad}-geri-dongu"),
            &format!("{ad}: 127.0.0.1"),
            "ölçüm (A9)",
            curl("", &format!("http://127.0.0.1:{}/", o.port)),
            olcum(),
        )
        .ag = ag;
    }
    ekle(
        &mut v,
        "p5-internet-http",
        "internet: dış HTTP",
        "bağlanır",
        curl("", "http://example.com"),
        basarili(),
    )
    .ag = Ag::Internet;
    let s = ekle(
        &mut v,
        "p5-internet-python-https",
        "internet: python ile HTTPS",
        "bağlanır",
        cmd(
            "python -c \"import urllib.request as u; print(u.urlopen('https://example.com', timeout=8).status)\"",
        ),
        basarili(),
    );
    s.ag = Ag::Internet;
    s.surucu = true;
    for (arka, ad) in [("openssl", "git-openssl"), ("schannel", "git-schannel")] {
        let s = ekle(
            &mut v,
            &format!("p5-internet-{ad}"),
            &format!("internet: git ls-remote HTTPS ({arka})"),
            "bağlanır",
            cmd(&format!(
                "git -c http.sslBackend={arka} ls-remote https://github.com/rust-lang/cfg-if HEAD"
            )),
            basarili(),
        );
        s.ag = Ag::Internet;
        s.surucu = true;
    }
    let s = ekle(
        &mut v,
        "p5-internet-cargo",
        "internet: cargo search (crates.io)",
        "bağlanır (A8)",
        cmd("cargo search cfg-if --limit 1"),
        basarili(),
    );
    s.ag = Ag::Internet;
    s.surucu = true;
    let s = ekle(
        &mut v,
        "p5-lpac-internet-https",
        "LPAC + internet: dış HTTPS",
        "ölçüm",
        curl("", "https://example.com"),
        olcum(),
    );
    s.ag = Ag::Internet;
    s.lpac = true;
    for boru in ["openssh-ssh-agent", "docker_engine"] {
        ekle(
            &mut v,
            &format!("p5-boru-{boru}"),
            &format!(r"\\.\pipe\{boru}"),
            "ölçüm",
            ps(&format!(
                "$p = New-Object IO.Pipes.NamedPipeClientStream('.', '{boru}'); try {{ $p.Connect(500); 'baglandi' }} catch {{ $_.Exception.GetType().Name + ': ' + $_.Exception.Message }}"
            )),
            olcum(),
        );
    }
    for (yon, ad) in [("In", "oku"), ("InOut", "oku-yaz")] {
        ekle(
            &mut v,
            &format!("p5-boru-varsayilan-{ad}"),
            &format!("geliştiricinin varsayılan izinli borusu ({yon})"),
            "açılmaz (A10)",
            ps(&format!(
                "$p = New-Object IO.Pipes.NamedPipeClientStream('.', '{}', '{yon}'); try {{ $p.Connect(2000); (New-Object IO.StreamReader($p)).ReadLine() }} catch {{ $_.Exception.GetType().Name + ': ' + $_.Exception.Message }}",
                o.boru
            )),
            icermez("BORU-OK"),
        );
    }

    // P7: gizli dosya ve korumalı yollar; yalnızca izinle (ret izinleri paket
    // SID'i için işlemiyor). Sürücü kipinde:
    // araçlar orada çalışıyor; ret başka bir nedenden gelmesin.
    for (kod, aciklama, beklenen, komut, denetim) in [
        (
            "p7-kontrol",
            "normal.txt'ye ekleme (karşılaştırma)",
            "yazılır",
            cmd("echo x>> normal.txt"),
            basarili(),
        ),
        (
            "p7-gizli",
            ".env okuma",
            "okunmaz",
            cmd("type .env"),
            icermez("GIZLI-ENV"),
        ),
        (
            "p7-config-yaz",
            ".git/config'e ekleme",
            "yazılmaz",
            cmd(r"echo x>> .git\config"),
            basarisiz(),
        ),
        (
            "p7-config-sil",
            ".git/config'i silme",
            "silinmez",
            cmd(r"del /q .git\config & if exist .git\config (exit 1) else (exit 0)"),
            basarisiz(),
        ),
        (
            "p7-config-uzerine",
            ".git/config'in üzerine taşıma",
            "taşınmaz",
            cmd(r"echo y> k.txt && move /y k.txt .git\config"),
            basarisiz(),
        ),
        (
            "p7-hook-yeni",
            ".git/hooks'a dosya ekleme",
            "eklenmez",
            cmd(r"echo x> .git\hooks\yeni"),
            basarisiz(),
        ),
        (
            "p7-hook-sil",
            "pre-commit'i silme",
            "silinmez",
            cmd(
                r"del /q .git\hooks\pre-commit & if exist .git\hooks\pre-commit (exit 1) else (exit 0)",
            ),
            basarisiz(),
        ),
        (
            "p7-hooks-tasi",
            ".git/hooks dizinini yeniden adlandırma",
            "taşınmaz",
            cmd(r"ren .git\hooks eski-hooks"),
            basarisiz(),
        ),
        (
            "p7-git-tasi",
            ".git dizinini yeniden adlandırma",
            "taşınmaz",
            cmd("ren .git eski-git"),
            basarisiz(),
        ),
        (
            "p7-git-ic",
            ".git içinde dosya oluşturma, yeniden adlandırma, silme",
            "çalışır",
            cmd(r"echo x> .git\d1 && ren .git\d1 d2 && del .git\d2 && echo TAMAM"),
            basarili(),
        ),
        (
            "p7-ret-deneme",
            "paket SID'ine okuma reddi (eski yöntem)",
            "ölçüm",
            cmd("type ret-deneme.txt"),
            olcum(),
        ),
    ] {
        ekle(&mut v, kod, aciklama, beklenen, komut, denetim).surucu = true;
    }

    // P9: cargo derlemesi, ağsız, MSVC ortamı geliştirici tarafında çözülerek;
    // sürücü kipinde (yol kipi ve MSVC'siz karşılaştırma).
    let s = ekle(
        &mut v,
        "p9-kutu-msvcsiz",
        "bağımlılıksız crate, MSVC ortamı verilmeden",
        "ölçüm",
        cmd("cargo build --offline -q && echo TAMAM"),
        olcum(),
    );
    s.dizin = format!(r"{KOK}\alan-a\kutu");
    s.surucu = true;
    let s = ekle(
        &mut v,
        "p9-kutu-yol",
        "bağımlılıksız crate, yol kipinde",
        "ölçüm",
        cmd("cargo build --offline -q && echo TAMAM"),
        olcum(),
    );
    s.dizin = format!(r"{KOK}\alan-a\kutu");
    s.ek_ortam = o.msvc.clone();
    let s = ekle(
        &mut v,
        "p9-kutu",
        "bağımlılıksız crate, çalışma alanına özel CARGO_HOME",
        "derlenir",
        cmd("cargo build --offline -q && echo TAMAM"),
        basarili(),
    );
    s.dizin = format!(r"{KOK}\alan-a\kutu");
    s.surucu = true;
    s.ek_ortam = o.msvc.clone();
    let s = ekle(
        &mut v,
        "p9-bagimli-ozel",
        "bağımlı crate, çalışma alanına özel CARGO_HOME",
        "derlenmez (önbellek yok)",
        cmd("cargo build --offline -q && echo TAMAM"),
        olcum(),
    );
    s.dizin = format!(r"{KOK}\alan-a\bagimli");
    s.surucu = true;
    s.ek_ortam = o.msvc.clone();
    let s = ekle(
        &mut v,
        "p9-bagimli-gelistirici",
        "bağımlı crate, geliştiricinin CARGO_HOME'u salt okunur",
        "ölçüm",
        cmd("cargo build --offline -q && echo TAMAM"),
        olcum(),
    );
    s.dizin = format!(r"{KOK}\alan-a\bagimli");
    s.cargo_home = Some(format!(r"{profil}\.cargo"));
    s.surucu = true;
    s.ek_ortam = o.msvc.clone();

    // P10: UI kısıtları.
    let yardimci = format!(r"{KOK}\bin\kalfa-prototip.exe ui-dene {}", o.pencere);
    ekle(
        &mut v,
        "p10-kisitli",
        "UI kısıtlarıyla ileti, pano, pencere bulma",
        "üçü de engellenir",
        yardimci.clone(),
        olcum(),
    );
    ekle(
        &mut v,
        "p10-kisitsiz",
        "UI kısıtları olmadan (karşılaştırma)",
        "ölçüm",
        yardimci,
        olcum(),
    )
    .ui_kisit = false;
    v
}
