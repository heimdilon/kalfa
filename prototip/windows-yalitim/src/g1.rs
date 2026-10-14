//! MCP 1.1a G1 ölçümü (1.1a tasarımı §4.1): AppContainer içinde borulu,
//! uzun yaşayan süreç ve sunucuya özel yetenek SID'i ayrımı.
//!
//!   kalfa-prototip g1 olc [kök]   üç ölçümü koşar, kökü kurar ve temizler
//!   kalfa-prototip g1 yanki       (kapta) stdin satırlarını stdout'a yankılar
//!   kalfa-prototip g1 uzun        (kapta) bir torun başlatır, ping'e yanıt verir
//!   kalfa-prototip g1 uyku        (kapta) sonsuza dek uyur
//!   kalfa-prototip g1 dene <yol>  (kapta) dosyayı okur ve yazar, sonucu basar
//!
//! Yönetici gerekmez. Profiller, ACE'ler ve kök ölçüm sonunda silinir.

use std::ffi::c_void;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::windows::io::FromRawHandle;
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    ERROR_SUCCESS, HANDLE, HLOCAL, LocalFree, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::Security::Authorization::{
    EXPLICIT_ACCESS_W, GRANT_ACCESS, NO_MULTIPLE_TRUSTEE, SE_FILE_OBJECT, SetEntriesInAclW,
    SetNamedSecurityInfoW, TRUSTEE_IS_SID, TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
};
use windows_sys::Win32::Security::{
    ACL, DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, SECURITY_CAPABILITIES,
    SID_AND_ATTRIBUTES, SUB_CONTAINERS_AND_OBJECTS_INHERIT,
};
use windows_sys::Win32::System::JobObjects::TerminateJobObject;
use windows_sys::Win32::System::SystemServices::SE_GROUP_ENABLED;
use windows_sys::Win32::System::Threading::{
    CREATE_NO_WINDOW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW,
    EXTENDED_STARTUPINFO_PRESENT, OpenProcess, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
    PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, PROCESS_INFORMATION,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, ResumeThread, STARTF_USESTDHANDLES,
    STARTUPINFOEXW, WaitForSingleObject,
};

use crate::kap::{self, Oznitelikler};
use crate::win::{self, Sid, Sonuc, Tutamac};

const TAM: u32 = 0x001F_01FF; // FILE_ALL_ACCESS
const DEGISTIR: u32 = 0x0013_01BF; // okuma + yazma + yürütme + silme
const OKU_YURUT: u32 = 0x0012_00A9; // FILE_GENERIC_READ | FILE_GENERIC_EXECUTE

pub fn ana(a: &[String]) -> i32 {
    match a.first().map(String::as_str) {
        Some("yanki") => yanki(),
        Some("uzun") => uzun(),
        Some("uyku") => loop {
            std::thread::sleep(Duration::from_secs(3600));
        },
        Some("dene") => dene(a.get(1).map(String::as_str).unwrap_or("")),
        Some("olc") => match olc(a.get(1).map(PathBuf::from)) {
            Ok(basarili) => {
                if basarili {
                    0
                } else {
                    1
                }
            }
            Err(e) => {
                eprintln!("G1 HATA: {e}");
                2
            }
        },
        _ => {
            eprintln!("kullanım: kalfa-prototip g1 olc [kök]");
            2
        }
    }
}

// ---------------------------------------------------------------- kaptaki taraf

fn yanki() -> i32 {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    let mut satir = String::new();
    let mut r = stdin.lock();
    loop {
        satir.clear();
        match r.read_line(&mut satir) {
            Ok(0) => return 0, // EOF: kibar kapanma
            Ok(_) => {
                let _ = out.write_all(satir.as_bytes());
                let _ = out.flush();
            }
            Err(_) => return 3,
        }
    }
}

fn uzun() -> i32 {
    let exe = std::env::current_exe().unwrap_or_default();
    let torun = std::process::Command::new(exe)
        .args(["g1", "uyku"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    let mut out = std::io::stdout().lock();
    match &torun {
        Ok(t) => {
            let _ = writeln!(out, "pid {} torun {}", std::process::id(), t.id());
        }
        Err(e) => {
            let _ = writeln!(out, "pid {} torun-hata {e}", std::process::id());
        }
    }
    let _ = out.flush();
    let stdin = std::io::stdin();
    let mut satir = String::new();
    let mut r = stdin.lock();
    loop {
        satir.clear();
        match r.read_line(&mut satir) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let _ = writeln!(out, "pong {}", satir.trim());
                let _ = out.flush();
            }
        }
    }
    // EOF'u bilerek yok sayar: kapanışı Job Object yapmalı.
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}

fn dene(yol: &str) -> i32 {
    let p = Path::new(yol);
    let yaz = std::fs::write(p.join("deneme.txt"), b"g1");
    let oku = std::fs::read_dir(p).map(|d| d.count());
    let kod = |r: &std::io::Error| r.raw_os_error().unwrap_or(-1);
    println!(
        "yaz={} oku={}",
        match &yaz {
            Ok(()) => "TAMAM".to_string(),
            Err(e) => format!("HATA{}", kod(e)),
        },
        match &oku {
            Ok(n) => format!("TAMAM({n})"),
            Err(e) => format!("HATA{}", kod(e)),
        }
    );
    0
}

// ---------------------------------------------------------------- başlatıcı

/// Kapta, borulu ve bir Job Object içinde başlamış süreç.
struct Kapli {
    surec: Tutamac,
    is: Option<Tutamac>,
    stdin: Option<File>,
    stdout: Option<File>,
    stderr: Option<File>,
}

fn kapta_baslat(
    paket: &Sid,
    yetenekler: &[&Sid],
    komut: &str,
    dizin: &Path,
) -> Sonuc<Kapli> {
    let (girdi_o, girdi_y) = win::boru(true)?; // okuma ucu çocuğa geçer
    let (cikti_o, cikti_y) = win::boru(false)?;
    let (hata_o, hata_y) = win::boru(false)?;
    let mut yet: Vec<SID_AND_ATTRIBUTES> = yetenekler
        .iter()
        .map(|s| SID_AND_ATTRIBUTES {
            Sid: s.ptr(),
            Attributes: SE_GROUP_ENABLED as u32,
        })
        .collect();
    let bilgi = SECURITY_CAPABILITIES {
        AppContainerSid: paket.ptr(),
        Capabilities: if yet.is_empty() {
            null_mut()
        } else {
            yet.as_mut_ptr()
        },
        CapabilityCount: yet.len() as u32,
        Reserved: 0,
    };
    let tutamaclar: [HANDLE; 3] = [girdi_o.0, cikti_y.0, hata_y.0];
    let mut liste = Oznitelikler::yeni(2)?;
    // SAFETY: değerler bu işlevin sonuna dek yaşar.
    unsafe {
        liste.ekle(
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
            &bilgi as *const _ as *const c_void,
            size_of::<SECURITY_CAPABILITIES>(),
        )?;
        liste.ekle(
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
            tutamaclar.as_ptr() as *const c_void,
            size_of_val(&tutamaclar),
        )?;
    }
    let is = win::is_nesnesi(true, false)?;
    let mut si: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    si.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    si.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    si.StartupInfo.hStdInput = girdi_o.0;
    si.StartupInfo.hStdOutput = cikti_y.0;
    si.StartupInfo.hStdError = hata_y.0;
    si.lpAttributeList = liste.ptr();
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let mut satir = win::genis(komut);
    let sistem = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    let d = dizin.display().to_string();
    // AppContainer başlatması profil değişkenlerini ister (yoksa 203).
    let ortam = win::ortam_blogu(&[
        ("SystemRoot".into(), sistem.clone()),
        ("windir".into(), sistem.clone()),
        ("SystemDrive".into(), "C:".into()),
        ("PATH".into(), format!(r"{sistem}\System32")),
        ("TEMP".into(), d.clone()),
        ("TMP".into(), d.clone()),
        ("USERPROFILE".into(), d.clone()),
        ("HOME".into(), d.clone()),
        ("APPDATA".into(), d.clone()),
        ("LOCALAPPDATA".into(), d),
    ]);
    // SAFETY: dizeler NUL ile biter ve çağrı boyunca yaşar.
    let tamam = unsafe {
        CreateProcessW(
            null(),
            satir.as_mut_ptr(),
            null(),
            null(),
            1,
            EXTENDED_STARTUPINFO_PRESENT
                | CREATE_SUSPENDED
                | CREATE_UNICODE_ENVIRONMENT
                | CREATE_NO_WINDOW,
            ortam.as_ptr() as *const _,
            win::genis(&dizin.display().to_string()).as_ptr(),
            &si.StartupInfo,
            &mut pi,
        )
    };
    if tamam == 0 {
        return Err(win::hata("CreateProcessW(AppContainer, borulu)"));
    }
    let surec = Tutamac(pi.hProcess);
    let ip = Tutamac(pi.hThread);
    win::ise_ata(is.0, surec.0)?;
    // SAFETY: iş parçacığı askıda başladı.
    unsafe { ResumeThread(ip.0) };
    drop((girdi_o, cikti_y, hata_y, liste));
    // SAFETY: uçlar bu işleve aittir; File'a devredilir.
    let (g, c, h) = unsafe {
        let g = File::from_raw_handle(girdi_y.0 as _);
        let c = File::from_raw_handle(cikti_o.0 as _);
        let h = File::from_raw_handle(hata_o.0 as _);
        std::mem::forget((girdi_y, cikti_o, hata_o));
        (g, c, h)
    };
    Ok(Kapli {
        surec,
        is: Some(is),
        stdin: Some(g),
        stdout: Some(c),
        stderr: Some(h),
    })
}

fn bekle(h: HANDLE, ms: u32) -> bool {
    // SAFETY: tutamaç geçerlidir.
    unsafe { WaitForSingleObject(h, ms) == WAIT_OBJECT_0 }
}

/// PID'li süreç hâlâ yaşıyor mu?
fn yasiyor(pid: u32) -> bool {
    // SAFETY: tutamaç kapatılır.
    unsafe {
        let h = OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return false;
        }
        let t = Tutamac(h);
        WaitForSingleObject(t.0, 0) == WAIT_TIMEOUT
    }
}

// ---------------------------------------------------------------- ACL

struct Erisim<'a> {
    sid: &'a Sid,
    maske: u32,
}

/// Dizine korumalı (mirası kesilmiş) yeni bir DACL yazar; alt öğelere mirasla
/// yayılır. Yalnız ölçümün kendi kurduğu dizinlerde kullanılır.
fn dacl_yaz(yol: &Path, erisimler: &[Erisim]) -> Sonuc<()> {
    let girdiler: Vec<EXPLICIT_ACCESS_W> = erisimler
        .iter()
        .map(|e| EXPLICIT_ACCESS_W {
            grfAccessPermissions: e.maske,
            grfAccessMode: GRANT_ACCESS,
            grfInheritance: SUB_CONTAINERS_AND_OBJECTS_INHERIT,
            Trustee: TRUSTEE_W {
                pMultipleTrustee: null_mut(),
                MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                TrusteeForm: TRUSTEE_IS_SID,
                TrusteeType: TRUSTEE_IS_UNKNOWN,
                ptstrName: e.sid.ptr() as *mut u16,
            },
        })
        .collect();
    let mut acl: *mut ACL = null_mut();
    // SAFETY: girdiler çağrı boyunca yaşar; ACL LocalFree ile bırakılır.
    unsafe {
        let r = SetEntriesInAclW(girdiler.len() as u32, girdiler.as_ptr(), null(), &mut acl);
        if r != ERROR_SUCCESS {
            return Err(format!("SetEntriesInAclW: {r}"));
        }
        let ad = win::genis(&yol.display().to_string());
        let r = SetNamedSecurityInfoW(
            ad.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            acl,
            null(),
        );
        LocalFree(acl as HLOCAL);
        if r != ERROR_SUCCESS {
            return Err(format!("SetNamedSecurityInfoW({}): {r}", yol.display()));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- ölçüm

struct Rapor {
    satirlar: Vec<String>,
    basarili: bool,
}

impl Rapor {
    fn sonuc(&mut self, ad: &str, gecti: bool, ayrinti: String) {
        self.basarili &= gecti;
        let s = format!("| {ad} | {} | {ayrinti} |", if gecti { "GEÇTİ" } else { "KALDI" });
        println!("{s}");
        self.satirlar.push(s);
    }
}

fn olc(kok: Option<PathBuf>) -> Sonuc<bool> {
    let kok = kok.unwrap_or_else(|| std::env::temp_dir().join("kalfa-g1"));
    let ek = std::process::id();
    let ad_a = format!("kalfa-g1-{ek}-a");
    let ad_b = format!("kalfa-g1-{ek}-b");
    let yetenek_adi = format!("kalfa-g1-{ek}-sunucu");
    let mut rapor = Rapor {
        satirlar: Vec::new(),
        basarili: true,
    };
    println!("kök: {}", kok.display());
    let sonuc = (|| -> Sonuc<()> {
        let _ = std::fs::remove_dir_all(&kok);
        let bin = kok.join("bin");
        let sunucu_kok = kok.join("sunucu-kok");
        let kabuk_kok = kok.join("kabuk-kok");
        for d in [&bin, &sunucu_kok, &kabuk_kok] {
            std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
        let exe = bin.join("kalfa-prototip.exe");
        std::fs::copy(std::env::current_exe().map_err(|e| e.to_string())?, &exe)
            .map_err(|e| format!("exe kopyası: {e}"))?;

        let a = kap::profil(&ad_a)?;
        let b = kap::profil(&ad_b)?;
        let yetenek = Sid::metinden(&win::yetenek_sidi(&yetenek_adi)?)?;
        let ben = win::kullanici_sidi(win::kendi_belirteci()?.0)?;
        let sistem = Sid::metinden("S-1-5-18")?;
        println!(
            "paket A {} · paket B {} · yetenek {}",
            a.metin(),
            b.metin(),
            yetenek.metin()
        );

        // Kök: kullanıcı ve SYSTEM tam; iki paket okuyup yürütebilir.
        dacl_yaz(
            &kok,
            &[
                Erisim { sid: &ben, maske: TAM },
                Erisim { sid: &sistem, maske: TAM },
                Erisim { sid: &a, maske: OKU_YURUT },
                Erisim { sid: &b, maske: OKU_YURUT },
            ],
        )?;
        // Sunucunun özel kökü: paket SID'ine hiçbir şey yok; yalnız yetenek.
        dacl_yaz(
            &sunucu_kok,
            &[
                Erisim { sid: &ben, maske: TAM },
                Erisim { sid: &sistem, maske: TAM },
                Erisim { sid: &yetenek, maske: DEGISTIR },
            ],
        )?;
        // Kabuğun özel kökü: paket A değiştirebilir (bugünkü 2d karşılığı).
        dacl_yaz(
            &kabuk_kok,
            &[
                Erisim { sid: &ben, maske: TAM },
                Erisim { sid: &sistem, maske: TAM },
                Erisim { sid: &a, maske: DEGISTIR },
            ],
        )?;
        let exe_s = exe.display().to_string();

        // ---------------- G1.1 borular
        {
            let mut k = kapta_baslat(&a, &[&yetenek], &format!("\"{exe_s}\" g1 yanki"), &sunucu_kok)?;
            let mut girdi = k.stdin.take().ok_or("stdin yok")?;
            let cikti = k.stdout.take().ok_or("stdout yok")?;
            let okuyucu = std::thread::spawn(move || {
                let mut r = BufReader::new(cikti);
                let mut satirlar = Vec::new();
                let mut s = String::new();
                while r.read_line(&mut s).map(|n| n > 0).unwrap_or(false) {
                    satirlar.push(std::mem::take(&mut s));
                }
                satirlar
            });
            let mut gonderilen = Vec::new();
            let bas = Instant::now();
            for i in 0..1000 {
                gonderilen.push(format!("{{\"jsonrpc\":\"2.0\",\"id\":{i},\"method\":\"ping\"}}\n"));
            }
            gonderilen.push(format!("{}\n", "x".repeat(1024 * 1024))); // 1 MiB satır
            gonderilen.push("{\"son\":true,\"metin\":\"çğıöşü\"}\n".into());
            for s in &gonderilen {
                girdi.write_all(s.as_bytes()).map_err(|e| format!("stdin yaz: {e}"))?;
            }
            drop(girdi); // EOF
            let alinan = okuyucu.join().map_err(|_| "okuyucu çöktü")?;
            let sure = bas.elapsed();
            let cikti_tamam = bekle(k.surec.0, 5000);
            let ayni = alinan == gonderilen;
            rapor.sonuc(
                "G1.1 stdin/stdout boruları",
                ayni && cikti_tamam,
                format!(
                    "{} satır gönderildi (1 MiB'lık satır ve UTF-8 dahil), {} geri geldi, birebir: {ayni}; EOF'ta kendiliğinden çıktı: {cikti_tamam}; {:.0} ms",
                    gonderilen.len(),
                    alinan.len(),
                    sure.as_secs_f64() * 1000.0
                ),
            );
        }

        // ---------------- G1.2 uzun yaşam ve Job Object kapanışı
        {
            let mut k = kapta_baslat(&a, &[&yetenek], &format!("\"{exe_s}\" g1 uzun"), &sunucu_kok)?;
            let mut girdi = k.stdin.take().ok_or("stdin yok")?;
            let mut r = BufReader::new(k.stdout.take().ok_or("stdout yok")?);
            let mut ilk = String::new();
            r.read_line(&mut ilk).map_err(|e| e.to_string())?;
            let pidler: Vec<u32> = ilk
                .split_whitespace()
                .filter_map(|p| p.parse().ok())
                .collect();
            let bas = Instant::now();
            let mut pong = 0;
            let sure = Duration::from_secs(
                std::env::var("KALFA_G1_SURE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(60),
            );
            let mut i = 0;
            while bas.elapsed() < sure {
                i += 1;
                writeln!(girdi, "{i}").map_err(|e| e.to_string())?;
                girdi.flush().map_err(|e| e.to_string())?;
                let mut s = String::new();
                r.read_line(&mut s).map_err(|e| e.to_string())?;
                if s.trim() == format!("pong {i}") {
                    pong += 1;
                }
                std::thread::sleep(Duration::from_secs(5));
            }
            let yasayan_once = pidler.iter().filter(|p| yasiyor(**p)).count();
            // Kapanış: stdin kapanır, 2 sn pay, sonra Job Object.
            drop(girdi);
            let kendiliginden = bekle(k.surec.0, 2000);
            let kapanis = Instant::now();
            // SAFETY: iş tutamacı geçerlidir.
            unsafe { TerminateJobObject(k.is.as_ref().ok_or("iş yok")?.0, 1) };
            let ana_bitti = bekle(k.surec.0, 5000);
            let olum = kapanis.elapsed();
            std::thread::sleep(Duration::from_millis(200));
            let yasayan_sonra: Vec<u32> = pidler.iter().copied().filter(|p| yasiyor(*p)).collect();
            drop(k.is.take());
            rapor.sonuc(
                "G1.2 uzun yaşam + Job Object kapanışı",
                pidler.len() == 2
                    && yasayan_once == 2
                    && pong == i
                    && !kendiliginden
                    && ana_bitti
                    && yasayan_sonra.is_empty(),
                format!(
                    "{} sn yaşadı, {pong}/{i} ping yanıtlandı; çocuk+torun PID {pidler:?} (kapanıştan önce yaşayan {yasayan_once}); stdin kapanınca kendiliğinden çıkmadı (beklenen): {}; TerminateJobObject sonrası yaşayan: {yasayan_sonra:?}; ölüm {:.0} ms",
                    bas.elapsed().as_secs(),
                    !kendiliginden,
                    olum.as_secs_f64() * 1000.0
                ),
            );
        }

        // ---------------- G1.3 yetenek SID'i ayrımı
        {
            let calistir = |paket: &Sid, yet: &[&Sid], hedef: &Path, cwd: &Path| -> Sonuc<String> {
                let mut k = kapta_baslat(
                    paket,
                    yet,
                    &format!("\"{exe_s}\" g1 dene \"{}\"", hedef.display()),
                    cwd,
                )?;
                drop(k.stdin.take());
                let mut s = String::new();
                k.stdout
                    .take()
                    .ok_or("stdout yok")?
                    .read_to_string(&mut s)
                    .map_err(|e| e.to_string())?;
                let mut h = String::new();
                let _ = k.stderr.take().ok_or("stderr yok")?.read_to_string(&mut h);
                bekle(k.surec.0, 5000);
                Ok(format!("{}{}", s.trim(), if h.is_empty() { String::new() } else { format!(" stderr={}", h.trim()) }))
            };
            let sunucu = calistir(&a, &[&yetenek], &sunucu_kok, &bin)?;
            let kabuk_ayni_paket = calistir(&a, &[], &sunucu_kok, &bin)?;
            let baska_paket = calistir(&b, &[], &sunucu_kok, &bin)?;
            let kabuk_kendi = calistir(&a, &[], &kabuk_kok, &bin)?;
            let sunucu_kabuga = calistir(&a, &[&yetenek], &kabuk_kok, &bin)?;
            let red = |s: &str| s.contains("yaz=HATA5") && s.contains("oku=HATA5");
            rapor.sonuc(
                "G1.3a yetenekli sunucu kendi köküne",
                sunucu.starts_with("yaz=TAMAM oku=TAMAM"),
                sunucu,
            );
            rapor.sonuc(
                "G1.3b aynı paket, yeteneksiz (kabuk) sunucu köküne",
                red(&kabuk_ayni_paket),
                kabuk_ayni_paket,
            );
            rapor.sonuc(
                "G1.3c başka paket, yeteneksiz sunucu köküne",
                red(&baska_paket),
                baska_paket,
            );
            rapor.sonuc(
                "G1.3d kabuk kendi köküne (denetim)",
                kabuk_kendi.starts_with("yaz=TAMAM"),
                kabuk_kendi,
            );
            rapor.sonuc(
                "G1.3e sunucu kabuk köküne (aynı paket SID'i; tasarımdaki W)",
                true,
                format!("{sunucu_kabuga} — bilgi: paket SID'i ortak olduğu için beklenen TAMAM"),
            );
        }
        Ok(())
    })();

    // ---------------- temizlik
    let mut temizlik = Vec::new();
    for ad in [&ad_a, &ad_b] {
        temizlik.push(match kap::profil_sil(ad) {
            Ok(()) => format!("profil {ad} silindi"),
            Err(e) => format!("profil {ad}: {e}"),
        });
    }
    let dizin = std::fs::remove_dir_all(&kok);
    temizlik.push(format!(
        "kök {}: {}",
        kok.display(),
        match &dizin {
            Ok(()) => "silindi".to_string(),
            Err(e) => format!("silinemedi: {e}"),
        }
    ));
    for t in &temizlik {
        println!("temizlik: {t}");
    }
    sonuc?;
    Ok(rapor.basarili && dizin.is_ok())
}
