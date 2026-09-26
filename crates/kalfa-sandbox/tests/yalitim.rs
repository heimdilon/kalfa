//! Gerçek yalıtımla testler (aşama 2 tasarımı §12.2, Linux sütunu). Yalıtım
//! yoksa atlanmaz, başarısız olur. Sıradan bir kullanıcıyla koşar:
//!
//! ```text
//! cargo test -p kalfa-sandbox --features yalitim-testleri
//! ```
//!
//! Komutlar `sh` ve `perl` (Debian/Ubuntu'da `perl-base`, her sistemde var)
//! ile yazılır.
#![cfg(all(target_os = "linux", feature = "yalitim-testleri"))]

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use kalfa_sandbox::linux::{self, BwrapAlgilayici, LinuxAyari, LinuxYalitici};
use kalfa_sandbox::{
    Akis, AlanBilgisi, Bitis, CalistirmaIstegi, KomutBicimi, Korumali, Yalitici, YalitimDurumu,
    tarama,
};
use tokio_util::sync::CancellationToken;

const UZUN: Duration = Duration::from_secs(60);

struct Kurgu {
    _gecici: tempfile::TempDir,
    taban: PathBuf,
    /// Sahte ev dizini: araç zinciri ve sırlar.
    ev: PathBuf,
    /// Çalışma alanı.
    kok: PathBuf,
    y: LinuxYalitici,
}

struct Sonuc {
    bitis: Bitis,
    standart: String,
    hata: String,
}

fn yaz(yol: &Path, icerik: &str) {
    if let Some(ust) = yol.parent() {
        std::fs::create_dir_all(ust).unwrap();
    }
    std::fs::write(yol, icerik).unwrap();
}

fn calistirilabilir_yaz(yol: &Path, icerik: &str) {
    use std::os::unix::fs::PermissionsExt;
    yaz(yol, icerik);
    std::fs::set_permissions(yol, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn kurgu() -> Kurgu {
    let gecici = tempfile::tempdir().unwrap();
    let taban = gecici.path().canonicalize().unwrap();
    let ev = taban.join("ev");
    let kok = taban.join("alan");
    std::fs::create_dir_all(&kok).unwrap();
    calistirilabilir_yaz(
        &ev.join(".cargo/bin/arac"),
        "#!/bin/sh\necho arac-calisti\n",
    );
    yaz(&ev.join(".cargo/credentials.toml"), "GIZLI-CARGO\n");
    yaz(&ev.join(".ssh/id_rsa"), "GIZLI-SSH\n");
    let y = LinuxYalitici::yeni(
        LinuxAyari {
            ev: ev.clone(),
            yalitim_dizini: taban.join("veri/yalitim"),
            ek_okunur: Vec::new(),
            git: Default::default(),
        },
        Box::new(BwrapAlgilayici),
        Box::new(|_| {}),
    );
    match y.durum() {
        YalitimDurumu::Hazir { .. } => {}
        d => panic!("yalıtım hazır değil: {d:?}"),
    }
    Kurgu {
        _gecici: gecici,
        taban,
        ev,
        kok,
        y,
    }
}

/// Testlerin gizli kuralı: `.env` ve `*.pem` dosyaları, `sirlar` dizinleri.
fn gizli_mi(yol: &Path, dizin: bool) -> bool {
    let ad = yol.file_name().and_then(|a| a.to_str()).unwrap_or("");
    if dizin {
        ad == "sirlar"
    } else {
        ad == ".env" || ad.ends_with(".pem")
    }
}

/// Git koruma çözücüsünün (kalfa-core) bu testlerdeki karşılığı: `.git`'in
/// tamamı ve `core.hooksPath` hedefi.
fn korumali(kok: &Path) -> Vec<Korumali> {
    let mut k = Vec::new();
    if let Ok(bilgi) = std::fs::symlink_metadata(kok.join(".git")) {
        k.push(Korumali {
            yol: kok.join(".git"),
            dizin: bilgi.is_dir(),
        });
    }
    if kok.join(".githooks").is_dir() {
        k.push(Korumali {
            yol: kok.join(".githooks"),
            dizin: true,
        });
    }
    k
}

impl Kurgu {
    fn istek(&self, kok: &Path, komut: KomutBicimi, ag: bool) -> CalistirmaIstegi {
        CalistirmaIstegi {
            komut,
            dizin: kok.to_path_buf(),
            ortam: vec![
                (
                    "PATH".into(),
                    format!("{}:/usr/bin:/bin", self.ev.join(".cargo/bin").display()),
                ),
                ("LANG".into(), "C.UTF-8".into()),
            ],
            ag,
            alan: Some(AlanBilgisi {
                kok: kok.to_path_buf(),
                gizli: tarama::tara(kok, &gizli_mi),
                korumali: korumali(kok),
                git_dizinleri: Vec::new(),
            }),
        }
    }

    async fn calistir_istek(&self, istek: CalistirmaIstegi, iptal: CancellationToken) -> Sonuc {
        let surec = self.y.baslat(&istek).expect("komut başlamalı");
        let mut standart = Vec::new();
        let mut hata = Vec::new();
        let sonuc = surec
            .bekle(UZUN, iptal, |akis, veri| match akis {
                Akis::Standart => standart.extend_from_slice(veri),
                Akis::Hata => hata.extend_from_slice(veri),
            })
            .await;
        Sonuc {
            bitis: sonuc.bitis,
            standart: String::from_utf8_lossy(&standart).into_owned(),
            hata: String::from_utf8_lossy(&hata).into_owned(),
        }
    }

    async fn betik(&self, kok: &Path, metin: &str, ag: bool) -> Sonuc {
        let istek = self.istek(
            kok,
            KomutBicimi::Betik {
                metin: metin.to_string(),
            },
            ag,
        );
        self.calistir_istek(istek, CancellationToken::new()).await
    }
}

/// Satırlardan `ad=değer` biçimindekinin değeri.
fn deger<'a>(metin: &'a str, ad: &str) -> &'a str {
    metin
        .lines()
        .find_map(|s| s.strip_prefix(&format!("{ad}=")))
        .unwrap_or_else(|| panic!("{ad} yok: {metin}"))
}

/// Komut satırında `parca` geçen süreçler (bütün ad alanlarından).
fn surecler(parca: &str) -> Vec<u32> {
    let Ok(dizin) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    dizin
        .filter_map(|g| g.ok())
        .filter_map(|g| g.file_name().to_str()?.parse::<u32>().ok())
        .filter(|pid| {
            std::fs::read(format!("/proc/{pid}/cmdline"))
                .map(|c| {
                    String::from_utf8_lossy(&c)
                        .replace('\0', " ")
                        .contains(parca)
                })
                .unwrap_or(false)
        })
        .collect()
}

fn kalmadi_mi(parca: &str, sure: Duration) -> bool {
    let son = Instant::now() + sure;
    while Instant::now() < son {
        if surecler(parca).is_empty() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

#[tokio::test]
async fn ev_dizinindeki_sirlar_okunamaz_arac_zinciri_calisir() {
    let k = kurgu();
    let ev = k.ev.display();
    let s = k
        .betik(
            &k.kok,
            &format!(
                "arac; cat {ev}/.ssh/id_rsa; cat {ev}/.cargo/credentials.toml; \
                 echo \"cargo=$(ls {ev}/.cargo | tr '\\n' ' ')\"; echo \"ev=$HOME\""
            ),
            false,
        )
        .await;
    assert!(s.standart.contains("arac-calisti"), "{}", s.standart);
    assert!(!s.standart.contains("GIZLI"), "{}", s.standart);
    assert_eq!(deger(&s.standart, "cargo").trim(), "bin");
    assert!(deger(&s.standart, "ev").ends_with("/ev"));
    assert_ne!(deger(&s.standart, "ev"), ev.to_string());
}

#[tokio::test]
async fn secilmeyen_yollar_ve_baska_calisma_alani_gorunmez() {
    let k = kurgu();
    let baska = k.taban.join("baska");
    yaz(&baska.join("dosya"), "BASKA-ICERIK\n");
    let s = k
        .betik(
            &k.kok,
            &format!(
                "for y in /srv /mnt /home /root /opt /media /sys {b}; do \
                 test -e \"$y\" && echo \"gorunur:$y\"; done; cat {b}/dosya; echo son",
                b = baska.display()
            ),
            false,
        )
        .await;
    assert!(s.standart.contains("son"), "{}{}", s.standart, s.hata);
    assert!(!s.standart.contains("gorunur:"), "{}", s.standart);
    assert!(!s.standart.contains("BASKA-ICERIK"));
}

#[tokio::test]
async fn ayni_anda_iki_calisma_alani_birbirini_okuyamaz_ve_yazamaz() {
    let k = kurgu();
    let b = k.taban.join("b");
    yaz(&k.kok.join("dosya"), "A-ICERIK\n");
    yaz(&b.join("dosya"), "B-ICERIK\n");
    let a_betik = format!(
        "cat {b}/dosya; mkdir -p {b} && echo x > {b}/a-yazdi; sleep 1",
        b = b.display()
    );
    let b_betik = format!(
        "cat {a}/dosya; mkdir -p {a} && echo x > {a}/b-yazdi; sleep 1",
        a = k.kok.display()
    );
    let (sa, sb) = tokio::join!(
        k.betik(&k.kok, &a_betik, false),
        k.betik(&b, &b_betik, false)
    );
    assert!(!sa.standart.contains("B-ICERIK"));
    assert!(!sb.standart.contains("A-ICERIK"));
    assert!(!b.join("a-yazdi").exists());
    assert!(!k.kok.join("b-yazdi").exists());
}

#[tokio::test]
async fn calisma_alani_disina_yazilamaz() {
    let k = kurgu();
    let s = k
        .betik(
            &k.kok,
            "echo x > /etc/kalfa-deneme; echo \"etc=$?\"; \
             echo x > /kalfa-deneme; echo \"kok=$?\"; \
             echo x > /tmp/kalfa-gecici; echo \"tmp=$?\"; \
             echo x > icerde.txt; echo \"alan=$?\"",
            false,
        )
        .await;
    assert_ne!(deger(&s.standart, "etc"), "0");
    assert_ne!(deger(&s.standart, "kok"), "0");
    assert_eq!(deger(&s.standart, "tmp"), "0");
    assert_eq!(deger(&s.standart, "alan"), "0");
    assert!(k.kok.join("icerde.txt").exists());
    // Geçici dizin çalışma alanına özeldir; sistemin /tmp'si değildir.
    assert!(!Path::new("/tmp/kalfa-gecici").exists());
}

#[tokio::test]
async fn git_dizini_ve_hooks_yolu_salt_okunur() {
    let k = kurgu();
    yaz(&k.kok.join(".git/config"), "[core]\n\tbare = false\n");
    yaz(&k.kok.join(".git/HEAD"), "ref: refs/heads/ana\n");
    yaz(&k.kok.join(".git/hooks/pre-commit.sample"), "#!/bin/sh\n");
    yaz(&k.kok.join(".githooks/.gitkeep"), "");
    let s = k
        .betik(
            &k.kok,
            "echo x > .git/hooks/pre-commit; echo \"kanca=$?\"; \
             rm -f .git/config; echo \"sil=$?\"; \
             echo x > .git/HEAD; echo \"head=$?\"; \
             echo x > .git/yeni; echo \"yeni=$?\"; \
             mv .git .git-2; echo \"ad=$?\"; \
             echo x > .githooks/pre-push; echo \"yol=$?\"",
            false,
        )
        .await;
    for ad in ["kanca", "sil", "head", "yeni", "ad", "yol"] {
        assert_ne!(deger(&s.standart, ad), "0", "{ad}: {}", s.standart);
    }
    assert_eq!(
        std::fs::read_to_string(k.kok.join(".git/config")).unwrap(),
        "[core]\n\tbare = false\n"
    );
    assert_eq!(
        std::fs::read_to_string(k.kok.join(".git/HEAD")).unwrap(),
        "ref: refs/heads/ana\n"
    );
    assert!(!k.kok.join(".git-2").exists());
    assert!(!k.kok.join(".git/yeni").exists());
    assert!(!k.kok.join(".git/hooks/pre-commit").exists());
    assert!(!k.kok.join(".githooks/pre-push").exists());
}

/// Worktree'nin `.git` dosyası da salt okunurdur.
#[tokio::test]
async fn git_dosyasi_salt_okunur() {
    let k = kurgu();
    yaz(&k.kok.join(".git"), "gitdir: ../baska\n");
    let s = k
        .betik(
            &k.kok,
            "echo x > .git; echo \"yaz=$?\"; mv .git .git-2; echo \"ad=$?\"",
            false,
        )
        .await;
    for ad in ["yaz", "ad"] {
        assert_ne!(deger(&s.standart, ad), "0", "{ad}: {}", s.standart);
    }
    assert_eq!(
        std::fs::read_to_string(k.kok.join(".git")).unwrap(),
        "gitdir: ../baska\n"
    );
}

#[tokio::test]
async fn env_ve_hard_linki_ve_gizli_dizin_okunamaz() {
    let k = kurgu();
    yaz(&k.kok.join(".env"), "GIZLI-ENV\n");
    std::fs::hard_link(k.kok.join(".env"), k.kok.join("a.txt")).unwrap();
    yaz(&k.kok.join("sirlar/anahtar"), "GIZLI-DIZIN\n");
    yaz(&k.kok.join("alt/sunucu.pem"), "GIZLI-PEM\n");
    yaz(&k.kok.join("normal.txt"), "ACIK\n");
    let s = k
        .betik(
            &k.kok,
            "cat .env; cat a.txt; cat sirlar/anahtar; cat alt/sunucu.pem; \
             echo \"liste=$(ls sirlar)\"; cat normal.txt; \
             echo x > sirlar/yeni; echo \"yaz=$?\"",
            false,
        )
        .await;
    assert!(!s.standart.contains("GIZLI"), "{}", s.standart);
    assert!(s.standart.contains("ACIK"));
    assert_eq!(deger(&s.standart, "liste"), "");
    assert_ne!(deger(&s.standart, "yaz"), "0");
    // Dosyalar yerinde durur.
    assert_eq!(
        std::fs::read_to_string(k.kok.join(".env")).unwrap(),
        "GIZLI-ENV\n"
    );
}

const SOKETLER: &str = r#"perl -MSocket -e '
for my $a ([PF_INET, "inet"], [PF_INET6, "inet6"], [PF_UNIX, "unix"], [40, "vsock"]) {
  if (socket(my $s, $a->[0], SOCK_STREAM, 0)) { print "$a->[1]=acildi\n" }
  else { print "$a->[1]=$!\n" }
}
if (socketpair(my $x, my $y, AF_UNIX, SOCK_STREAM, PF_UNSPEC)) { print "cift=acildi\n" }
else { print "cift=$!\n" }'"#;

#[tokio::test]
async fn agsiz_profilde_tcp_ve_unix_soketi_acilamaz() {
    let k = kurgu();
    let s = k.betik(&k.kok, SOKETLER, false).await;
    for aile in ["inet", "inet6", "unix", "vsock"] {
        assert_eq!(
            deger(&s.standart, aile),
            "Operation not permitted",
            "{}{}",
            s.standart,
            s.hata
        );
    }
    assert_eq!(deger(&s.standart, "cift"), "acildi");
}

#[tokio::test]
async fn agli_profilde_tcp_acilir_unix_soketi_acilamaz() {
    let k = kurgu();
    let dinleyici = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = dinleyici.local_addr().unwrap().port();
    let kabul = std::thread::spawn(move || dinleyici.accept().is_ok());
    let s = k
        .betik(
            &k.kok,
            &format!(
                r#"{SOKETLER}; perl -MSocket -e '
socket(my $s, PF_INET, SOCK_STREAM, 0) or die "soket=$!\n";
connect($s, pack_sockaddr_in({port}, inet_aton("127.0.0.1"))) or die "baglanti=$!\n";
print "tcp=baglandi\n"'"#
            ),
            true,
        )
        .await;
    assert_eq!(
        deger(&s.standart, "inet"),
        "acildi",
        "{}{}",
        s.standart,
        s.hata
    );
    assert_eq!(deger(&s.standart, "unix"), "Operation not permitted");
    assert_eq!(deger(&s.standart, "vsock"), "Operation not permitted");
    assert_eq!(deger(&s.standart, "tcp"), "baglandi", "{}", s.hata);
    assert!(kabul.join().unwrap());
}

#[tokio::test]
async fn seccomp_engelleri() {
    let k = kurgu();
    let engelli = [
        ("io_uring_setup", libc::SYS_io_uring_setup, "1, 0"),
        ("keyctl", libc::SYS_keyctl, "0, 0, 0, 0, 0"),
        ("bpf", libc::SYS_bpf, "0, 0, 0"),
        ("ptrace", libc::SYS_ptrace, "0, 0, 0, 0"),
        (
            "perf_event_open",
            libc::SYS_perf_event_open,
            "0, 0, 0, 0, 0",
        ),
        ("userfaultfd", libc::SYS_userfaultfd, "0"),
        (
            "unshare",
            libc::SYS_unshare,
            &*format!("{}", libc::CLONE_NEWUSER),
        ),
    ];
    let mut betik = String::from("perl -e '");
    for (ad, nr, arg) in &engelli {
        betik.push_str(&format!(
            "$r = syscall({nr}, {arg}); print \"{ad}=\" . ($r == -1 ? $! : \"izin\") . \"\\n\"; "
        ));
    }
    betik.push_str(&format!(
        "$r = syscall({}, 0, 0); print \"clone3=\" . ($r == -1 ? $! : \"izin\") . \"\\n\"; ",
        libc::SYS_clone3
    ));
    if cfg!(target_arch = "x86_64") {
        betik.push_str(&format!(
            "$r = syscall({}); print \"x32=\" . ($r == -1 ? $! : \"izin\") . \"\\n\"; ",
            0x4000_0000 | libc::SYS_getpid
        ));
    }
    betik.push('\'');
    let s = k.betik(&k.kok, &betik, false).await;
    for (ad, _, _) in &engelli {
        assert_eq!(
            deger(&s.standart, ad),
            "Operation not permitted",
            "{}{}",
            s.standart,
            s.hata
        );
    }
    assert_eq!(deger(&s.standart, "clone3"), "Function not implemented");
    if cfg!(target_arch = "x86_64") {
        assert_eq!(deger(&s.standart, "x32"), "Function not implemented");
    }
}

#[tokio::test]
async fn iptal_agaci_torun_ve_setsid_kacagi_dahil_durdurur() {
    let k = kurgu();
    let istek = k.istek(
        &k.kok,
        KomutBicimi::Betik {
            metin: "sleep 7771 & setsid sleep 7772 & echo basladi; wait".into(),
        },
        false,
    );
    let iptal = CancellationToken::new();
    let surec = k.y.baslat(&istek).unwrap();
    let tetik = iptal.clone();
    let sonuc = surec
        .bekle(UZUN, iptal, move |_, veri| {
            if String::from_utf8_lossy(veri).contains("basladi") {
                tetik.cancel();
            }
        })
        .await;
    assert_eq!(sonuc.bitis, Bitis::Iptal);
    assert!(kalmadi_mi("sleep 7771", Duration::from_secs(5)));
    assert!(kalmadi_mi("sleep 7772", Duration::from_secs(5)));
}

/// Kalfa gibi davranan yardımcı: ortam değişkeniyle çağrılınca yalıtımda
/// uzun bir komut başlatır, "hazir" yazar ve bekler. Doğrudan koşunca bir şey
/// yapmaz.
#[tokio::test]
async fn yardimci_kalfa_gibi() {
    if std::env::var_os("KALFA_YARDIMCI").is_none() {
        return;
    }
    let k = kurgu();
    let istek = k.istek(
        &k.kok,
        KomutBicimi::Betik {
            metin: "sleep 7781 & echo basladi; wait".into(),
        },
        false,
    );
    let surec = k.y.baslat(&istek).unwrap();
    surec
        .bekle(UZUN, CancellationToken::new(), |_, veri| {
            if String::from_utf8_lossy(veri).contains("basladi") {
                println!("hazir");
            }
        })
        .await;
}

#[test]
fn kalfanin_olumu_agaci_durdurur() {
    let mut yardimci = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "yardimci_kalfa_gibi",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("KALFA_YARDIMCI", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let cikti = BufReader::new(yardimci.stdout.take().unwrap());
    for satir in cikti.lines() {
        if satir.unwrap().contains("hazir") {
            break;
        }
    }
    assert!(!surecler("sleep 7781").is_empty());
    yardimci.kill().unwrap();
    yardimci.wait().unwrap();
    assert!(kalmadi_mi("sleep 7781", Duration::from_secs(5)));
}

#[tokio::test]
async fn surec_sayisi_siniri_yalnizca_icerdekileri_sayar() {
    let k = kurgu();
    // Dışarıda 260 süreç varken içeride 100 süreç açılabilir.
    let mut disari: Vec<_> = (0..260)
        .map(|_| Command::new("sleep").arg("30").spawn().unwrap())
        .collect();
    let s = k
        .betik(
            &k.kok,
            "i=0; while [ $i -lt 100 ]; do sleep 3 & i=$((i+1)); done; echo yuz=tamam; wait",
            false,
        )
        .await;
    for c in &mut disari {
        let _ = c.kill();
        let _ = c.wait();
    }
    assert_eq!(deger(&s.standart, "yuz"), "tamam", "{}", s.hata);
    // İçeride 300 süreç açılamaz.
    let s = k
        .betik(
            &k.kok,
            "i=0; while [ $i -lt 300 ]; do sleep 3 & i=$((i+1)); done; echo uc_yuz=tamam; wait",
            false,
        )
        .await;
    assert!(!s.standart.contains("uc_yuz=tamam"));
    assert!(s.hata.to_lowercase().contains("fork"), "{}", s.hata);
}

#[tokio::test]
async fn sahte_bwrap_secilmez() {
    let k = kurgu();
    let isaret = k.taban.join("sahte-bwrap-calisti");
    calistirilabilir_yaz(
        &k.kok.join("bin/bwrap"),
        &format!("#!/bin/sh\ntouch {}\n", isaret.display()),
    );
    assert!(linux::bwrap_yolu().is_some_and(|y| y.starts_with("/usr")));
    let mut istek = k.istek(
        &k.kok,
        KomutBicimi::Program {
            program: "true".into(),
            argumanlar: Vec::new(),
        },
        false,
    );
    istek.ortam[0].1 = format!("{}:/usr/bin:/bin", k.kok.join("bin").display());
    let s = k.calistir_istek(istek, CancellationToken::new()).await;
    assert_eq!(s.bitis, Bitis::Cikti(Some(0)), "{}", s.hata);
    k.y.yeniden_algila();
    assert!(!isaret.exists());
}

#[tokio::test]
async fn program_yalitimin_icindeki_pathte_aranir() {
    let k = kurgu();
    let istek = k.istek(
        &k.kok,
        KomutBicimi::Program {
            program: "arac".into(),
            argumanlar: Vec::new(),
        },
        false,
    );
    let s = k.calistir_istek(istek, CancellationToken::new()).await;
    assert_eq!(s.standart.trim(), "arac-calisti");
    let istek = k.istek(
        &k.kok,
        KomutBicimi::Program {
            program: "kesinlikle-olmayan".into(),
            argumanlar: Vec::new(),
        },
        false,
    );
    assert!(matches!(
        k.y.baslat(&istek),
        Err(kalfa_sandbox::BaslatmaHatasi::ProgramYok { .. })
    ));
}

#[test]
fn tarama_yuz_bin_dosyada_olculur() {
    let d = tempfile::tempdir().unwrap();
    for i in 0..1000 {
        let dizin = d.path().join(format!("d{i}"));
        std::fs::create_dir(&dizin).unwrap();
        for j in 0..100 {
            std::fs::write(dizin.join(format!("f{j}.txt")), "").unwrap();
        }
    }
    yaz(&d.path().join("d7/.env"), "x");
    let bas = Instant::now();
    let t = tarama::tara(d.path(), &gizli_mi);
    let sure = bas.elapsed();
    println!("tarama: 100 000 dosya, {} ms", sure.as_millis());
    assert_eq!(t.dosyalar, vec![d.path().join("d7/.env")]);
}
