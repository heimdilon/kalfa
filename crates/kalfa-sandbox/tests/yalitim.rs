//! Gerçek yalıtımla testler (aşama 2 tasarımı §12.2). Yalıtım yoksa atlanmaz,
//! başarısız olur. Windows'ta kurulumdan sonra, Linux'ta sıradan kullanıcıyla
//! koşar:
//!
//! ```text
//! cargo test -p kalfa-sandbox --features yalitim-testleri
//! ```
//!
//! Komutlar `sh` ve `perl` (Debian/Ubuntu'da `perl-base`, her sistemde var)
//! ile yazılır.
#![cfg(feature = "yalitim-testleri")]

#[cfg(target_os = "windows")]
#[path = "yalitim/windows.rs"]
mod windows;

#[cfg(target_os = "linux")]
macro_rules! linux_gercek_yalitim_testleri {
    ($($item:item)*) => {
        $($item)*
    };
}

#[cfg(target_os = "linux")]
linux_gercek_yalitim_testleri! {

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
    ag: Option<kalfa_sandbox::vekil::AgOzeti>,
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
            kopru: None,
            git: Default::default(),
            vekil_cozucu: None,
            vekil_yerel_izin: false,
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

fn kurgu_agli(
    cozucu: Option<std::sync::Arc<dyn kalfa_sandbox::vekil::Cozucu>>,
    yerel_izin: bool,
) -> Kurgu {
    let gecici = tempfile::tempdir().unwrap();
    let taban = gecici.path().canonicalize().unwrap();
    let ev = taban.join("ev");
    let kok = taban.join("alan");
    std::fs::create_dir_all(&kok).unwrap();
    calistirilabilir_yaz(
        &ev.join(".cargo/bin/arac"),
        "#!/bin/sh\necho arac-calisti\n",
    );
    let y = LinuxYalitici::yeni(
        LinuxAyari {
            ev: ev.clone(),
            yalitim_dizini: taban.join("veri/yalitim"),
            ek_okunur: Vec::new(),
            kopru: Some(PathBuf::from(env!("CARGO_BIN_EXE_kalfa-kopru"))),
            git: Default::default(),
            vekil_cozucu: cozucu,
            vekil_yerel_izin: yerel_izin,
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
            ag_listesi: None,
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
            ag: sonuc.ag,
        }
    }

    fn istek_listeli(
        &self,
        kok: &Path,
        komut: KomutBicimi,
        ag: bool,
        liste: Option<kalfa_sandbox::vekil::IzinListesi>,
    ) -> CalistirmaIstegi {
        let mut req = self.istek(kok, komut, ag);
        req.ag_listesi = liste;
        req
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

    async fn betik_listeli(
        &self,
        kok: &Path,
        metin: &str,
        ag: bool,
        liste: Option<kalfa_sandbox::vekil::IzinListesi>,
    ) -> Sonuc {
        let istek = self.istek_listeli(
            kok,
            KomutBicimi::Betik {
                metin: metin.to_string(),
            },
            ag,
            liste,
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
    let k = kurgu_agli(None, true);
    let s = k
        .betik(
            &k.kok,
            &format!(
                r#"{SOKETLER}; perl -MIO::Socket::INET -e '
my $l = IO::Socket::INET->new(LocalAddr => "127.0.0.1", LocalPort => 0, Listen => 1, Reuse => 1) or die "dinleyici=$!\n";
my $p = $l->sockport();
my $pid = fork();
if ($pid == 0) {{
    my $c = $l->accept();
    close($c);
    exit(0);
}}
use IO::Socket::INET;
my $s = IO::Socket::INET->new(PeerAddr => "127.0.0.1", PeerPort => $p, Proto => "tcp") or die "baglanti=$!\n";
print "tcp=baglandi\n";
waitpid($pid, 0);
'"#
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

struct TestCozucu {
    eslemeler: std::collections::HashMap<(String, u16), Vec<std::net::SocketAddr>>,
}

impl kalfa_sandbox::vekil::Cozucu for TestCozucu {
    fn coz<'a>(
        &'a self,
        host: &'a str,
        port: u16,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = std::io::Result<Vec<std::net::SocketAddr>>>
                + Send
                + 'a,
        >,
    > {
        let key = (host.to_lowercase(), port);
        let res = self.eslemeler.get(&key).cloned().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{host}:{port} bulunamadı"),
            )
        });
        Box::pin(async move { res })
    }
}

#[tokio::test]
async fn agli_komut_izinli_hosta_vekilden_ulasir() {
    let dinleyici = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = dinleyici.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut soket, _)) = dinleyici.accept().await {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 1024];
            let n = soket.read(&mut buf).await.unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]);
            if req.starts_with("CONNECT") {
                let _ = soket
                    .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                    .await;
            } else {
                let yanit =
                    b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nSELAM";
                let _ = soket.write_all(yanit).await;
            }
        }
    });

    let mut eslemeler = std::collections::HashMap::new();
    eslemeler.insert(
        ("fake.crates.io".to_string(), port),
        vec![std::net::SocketAddr::from(([127, 0, 0, 1], port))],
    );
    let cozucu = std::sync::Arc::new(TestCozucu { eslemeler });
    let k = kurgu_agli(Some(cozucu), true);

    let liste =
        kalfa_sandbox::vekil::IzinListesi::yeni(&[], &[("fake.crates.io".to_string(), port)]);

    let betik = format!("curl -p -s -S http://fake.crates.io:{port}/");
    let sonuc = k.betik_listeli(&k.kok, &betik, true, Some(liste)).await;
    assert_eq!(
        sonuc.bitis,
        Bitis::Cikti(Some(0)),
        "curl başarılı çıkmalı: {}",
        sonuc.hata
    );
    assert!(
        sonuc.standart.contains("SELAM"),
        "çıktı SELAM içermeli: {}",
        sonuc.standart
    );
    let ag = sonuc.ag.expect("ağ özeti dönmeli");
    assert!(
        ag.baglantilar.iter().any(
            |b| b.host == "fake.crates.io" && b.karar == kalfa_sandbox::vekil::AgKarari::Izinli
        ),
        "ağ özetinde fake.crates.io izinli olmalı: {:?}",
        ag.baglantilar
    );
}

#[tokio::test]
async fn izinsiz_host_403_alir_dogrudan_baglanti_ve_ad_cozumu_yok() {
    let k = kurgu_agli(None, true);
    let liste = kalfa_sandbox::vekil::IzinListesi::yeni(&[], &[]);

    // 1. Vekil üzerinden izinsiz host 403 alır
    let betik = "curl -s -i http://evil.com/";
    let sonuc = k.betik_listeli(&k.kok, betik, true, Some(liste.clone())).await;
    assert!(
        sonuc.standart.contains("403 Forbidden"),
        "vekil 403 Forbidden dönmeli: {}",
        sonuc.standart
    );
    let ag = sonuc.ag.expect("ağ özeti dönmeli");
    assert!(
        ag.baglantilar
            .iter()
            .any(|b| b.host == "evil.com" && b.karar == kalfa_sandbox::vekil::AgKarari::ListedeYok),
        "ağ özetinde evil.com ListedeYok olmalı: {:?}",
        ag.baglantilar
    );

    // 2. Doğrudan bağlantı yok (bwrap --unshare-net yüzünden dış ağ erişimi yok)
    let betik_dogrudan =
        "curl --noproxy '*' --connect-timeout 2 http://1.1.1.1/ || echo 'BAGLANTI_YOK'";
    let sonuc_dogrudan = k
        .betik_listeli(&k.kok, betik_dogrudan, true, Some(liste.clone()))
        .await;
    assert!(
        sonuc_dogrudan.standart.contains("BAGLANTI_YOK"),
        "doğrudan bağlantı kurulamamalı: {}",
        sonuc_dogrudan.standart
    );

    // 3. Ad çözümü yok (resolv.conf bağlanmaz)
    let betik_dns = "getent hosts evil.com || echo 'COZULEMEDI'";
    let sonuc_dns = k.betik_listeli(&k.kok, betik_dns, true, Some(liste)).await;
    assert!(
        sonuc_dns.standart.contains("COZULEMEDI"),
        "kap içinde doğrudan ad çözümü olmamalı: {}",
        sonuc_dns.standart
    );
}

#[tokio::test]
async fn vekil_ozel_ag_adresine_baglanmaz() {
    let dinleyici = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = dinleyici.local_addr().unwrap().port();

    let mut eslemeler = std::collections::HashMap::new();
    eslemeler.insert(
        ("yerel.test".to_string(), port),
        vec![std::net::SocketAddr::from(([127, 0, 0, 1], port))],
    );
    let cozucu = std::sync::Arc::new(TestCozucu { eslemeler });

    // yerel_izin = false olduğunda özel ağ adresine çözülen istek reddedilmeli
    let k = kurgu_agli(Some(cozucu), false);
    let liste = kalfa_sandbox::vekil::IzinListesi::yeni(&[], &[("yerel.test".to_string(), port)]);

    let betik = format!("curl -p -s -i http://yerel.test:{port}/");
    let sonuc = k.betik_listeli(&k.kok, &betik, true, Some(liste)).await;
    assert!(
        sonuc.standart.contains("403 Forbidden"),
        "özel ağa çözülen host 403 almalı: {}",
        sonuc.standart
    );
    let ag = sonuc.ag.expect("ağ özeti dönmeli");
    assert!(
        ag.baglantilar.iter().any(|b| {
            b.host == "yerel.test"
                && matches!(b.karar, kalfa_sandbox::vekil::AgKarari::YasakAdres(_))
        }),
        "ağ özetinde YasakAdres kararı olmalı: {:?}",
        ag.baglantilar
    );
}

#[tokio::test]
async fn kanal_ve_dinleyici_komuta_gecmez() {
    let k = kurgu_agli(None, true);
    let liste = kalfa_sandbox::vekil::IzinListesi::yeni(&[], &[]);

    let betik = "ls /proc/self/fd";
    let sonuc = k.betik_listeli(&k.kok, betik, true, Some(liste)).await;
    assert_eq!(sonuc.bitis, Bitis::Cikti(Some(0)));
    let fds: Vec<&str> = sonuc.standart.split_whitespace().collect();
    for fd in &fds {
        let n: i32 = fd.parse().unwrap_or(-1);
        assert!(n <= 3, "beklenmeyen açık fd komuta geçmiş: {fd}, tümü: {fds:?}");
    }
}

#[tokio::test]
async fn kopru_oldurulunce_komut_aga_cikamaz() {
    let k = kurgu_agli(None, true);
    let liste = kalfa_sandbox::vekil::IzinListesi::yeni(&[], &[]);

    let betik = "pkill -9 -f kalfa-kopru 2>/dev/null; sleep 0.1; curl -s -S --connect-timeout 2 http://fake.crates.io/";
    let sonuc = k.betik_listeli(&k.kok, betik, true, Some(liste)).await;
    assert_ne!(
        sonuc.bitis,
        Bitis::Cikti(Some(0)),
        "köprü öldürülünce komut ağa çıkamamalı: {}",
        sonuc.standart
    );
}

#[tokio::test]
async fn kabin_icindeki_sunucu_vekilsiz_calisir() {
    let k = kurgu_agli(None, true);
    let liste = kalfa_sandbox::vekil::IzinListesi::yeni(&[], &[]);

    let betik = r#"perl -MIO::Socket::INET -e '
    my $server = IO::Socket::INET->new(LocalAddr => "127.0.0.1", LocalPort => 0, Listen => 1, Reuse => 1) or die;
    my $port = $server->sockport();
    my $pid = fork();
    if ($pid == 0) {
        my $client = $server->accept();
        print $client "YEREL_CEVAP\n";
        close($client);
        exit(0);
    }
    my $sock = IO::Socket::INET->new(PeerAddr => "127.0.0.1", PeerPort => $port, Proto => "tcp") or die;
    my $line = <$sock>;
    print $line;
    waitpid($pid, 0);
'"#;
    let sonuc = k.betik_listeli(&k.kok, betik, true, Some(liste)).await;
    assert_eq!(
        sonuc.bitis,
        Bitis::Cikti(Some(0)),
        "kabin içindeki sunucuya bağlanabilmeli: {}",
        sonuc.hata
    );
    assert!(
        sonuc.standart.contains("YEREL_CEVAP"),
        "çıktıda YEREL_CEVAP olmalı: {}",
        sonuc.standart
    );
    let ag = sonuc.ag.expect("ağ özeti dönmeli");
    assert!(
        ag.baglantilar.is_empty(),
        "kabin içindeki yerel bağlantı vekile gitmemeli: {:?}",
        ag.baglantilar
    );
}

#[test]
fn profil_ozeti_liste_degisince_degisir() {
    let k = kurgu();
    let ozet_agsiz = k.y.profil_ozeti(false, None).expect("profil özeti olmalı");
    let ozet1 = k.y.profil_ozeti(true, Some("ozet-1")).expect("profil özeti olmalı");
    let ozet2 = k.y.profil_ozeti(true, Some("ozet-2")).expect("profil özeti olmalı");

    assert_ne!(ozet_agsiz, ozet1, "ağ açıkken özet değişmeli");
    assert_ne!(ozet1, ozet2, "liste değişince özet değişmeli");
}

#[tokio::test]
async fn devredilmeyen_tanimlayici_kaba_gecmez() {
    let k = kurgu();
    let fd = unsafe { libc::open(c"/dev/null".as_ptr(), libc::O_RDONLY) };
    assert!(fd >= 3, "açılan fd 3'ten büyük olmalı: {fd}");

    let sonuc = k.betik(&k.kok, "ls /proc/self/fd", false).await;
    unsafe {
        libc::close(fd);
    }
    assert_eq!(sonuc.bitis, Bitis::Cikti(Some(0)));
    let fds: Vec<&str> = sonuc.standart.split_whitespace().collect();
    let fd_str = fd.to_string();
    assert!(
        !fds.contains(&fd_str.as_str()),
        "CLOEXEC'siz açılan fd={fd} kaba geçmiş olmamalı! Mevcut: {fds:?}"
    );
}

#[tokio::test]
async fn kopru_yoksa_agli_komut_baslamaz() {
    let k = kurgu();
    let istek = k.istek(
        &k.kok,
        KomutBicimi::Program {
            program: "true".into(),
            argumanlar: Vec::new(),
        },
        true,
    );
    let hata = k
        .y
        .baslat(&istek)
        .err()
        .expect("köprü yokken ağlı komut başlamamalı");
    match hata {
        kalfa_sandbox::BaslatmaHatasi::Yalitim(msg) => {
            assert!(
                msg.contains("Ağ köprüsü ikilisi (kalfa-kopru) ayarlanmamış"),
                "hata iletisi bekleneni söylemeli: {msg}"
            );
        }
        diger => panic!("beklenmeyen hata türü: {diger:?}"),
    }
}

// --- 1.1a-1: etkileşimli süreç (G1'in ürün başlatıcısıyla karşılıkları) ---

fn etkilesimli_istek(
    k: &Kurgu,
    program: &str,
    arg: &[&str],
    sunucu: &str,
) -> kalfa_sandbox::EtkilesimliIstek {
    kalfa_sandbox::EtkilesimliIstek {
        program: program.into(),
        argumanlar: arg.iter().map(|s| s.to_string()).collect(),
        ortam: vec![
            (
                "PATH".into(),
                format!("{}:/usr/bin:/bin", k.ev.join(".cargo/bin").display()),
            ),
            ("LANG".into(), "C.UTF-8".into()),
        ],
        kip: kalfa_sandbox::CalistirmaKipi::Yalitimli,
        sunucu: sunucu.into(),
        alan: AlanBilgisi {
            kok: k.kok.clone(),
            gizli: tarama::tara(&k.kok, &gizli_mi),
            korumali: korumali(&k.kok),
            git_dizinleri: Vec::new(),
        },
        ag_listesi: None,
    }
}

async fn tum_cikti(s: &mut kalfa_sandbox::EtkilesimliSurec) -> String {
    let mut m = String::new();
    tokio::io::AsyncReadExt::read_to_string(&mut s.cikti_al().unwrap(), &mut m)
        .await
        .unwrap();
    m
}

/// G1.1 (ürün başlatıcısıyla): 1 000 JSON-RPC satırı, 1 MiB'lık satır ve
/// UTF-8 birebir; EOF'ta çıkış.
#[tokio::test]
async fn etkilesimli_borular_birebir_yankilanir() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let k = kurgu();
    let mut s =
        k.y.baslat_etkilesimli(&etkilesimli_istek(&k, "cat", &[], "yanki"))
            .unwrap();
    let mut girdi = s.girdi_al().unwrap();
    let mut cikti = BufReader::new(s.cikti_al().unwrap()).lines();
    let mut gonderilen: Vec<String> = (0..1000)
        .map(|i| format!("{{\"jsonrpc\":\"2.0\",\"id\":{i},\"method\":\"ping\"}}"))
        .collect();
    gonderilen.push("x".repeat(1024 * 1024));
    gonderilen.push("{\"metin\":\"çğıöşü\"}".into());
    let g = gonderilen.clone();
    let yazici = tokio::spawn(async move {
        for s in g {
            girdi.write_all(s.as_bytes()).await.unwrap();
            girdi.write_all(b"\n").await.unwrap();
        }
    });
    let mut alinan = Vec::new();
    while let Some(s) = cikti.next_line().await.unwrap() {
        alinan.push(s);
    }
    yazici.await.unwrap();
    assert_eq!(alinan, gonderilen);
    assert_eq!(s.bitisi_bekle().await, Some(0));
    assert!(s.kapat().await.unwrap().kendiliginden);
}

/// G1.2 (ürün başlatıcısıyla): kök ve kendi oturumundaki torun yaşar,
/// ping'lere yanıt verilir, EOF yok sayılır; `kapat` döndüğünde ikisi de
/// yaşamaz.
#[tokio::test]
async fn etkilesimli_uzun_yasar_kapat_agaci_durdurur() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let k = kurgu();
    let isaret = format!("kalfa-g1-{}", std::process::id());
    // İşaretler `$0`'dan kurulur; betiğin metninde (bwrap'ın komut
    // satırında) geçmez.
    let betik = "setsid perl -e 'sleep 600' \"$0-torun\" </dev/null >/dev/null 2>&1 & \
                 trap '' TERM HUP; echo hazir; \
                 while read l; do echo \"pong $l\"; done; \
                 exec perl -e '$SIG{TERM}=q(IGNORE); sleep 600' \"$0-kok\"";
    let mut s =
        k.y.baslat_etkilesimli(&etkilesimli_istek(&k, "sh", &["-c", betik, &isaret], "uzun"))
            .unwrap();
    let mut girdi = s.girdi_al().unwrap();
    let mut cikti = BufReader::new(s.cikti_al().unwrap()).lines();
    assert_eq!(cikti.next_line().await.unwrap().as_deref(), Some("hazir"));
    for i in 1..=5 {
        girdi.write_all(format!("{i}\n").as_bytes()).await.unwrap();
        assert_eq!(cikti.next_line().await.unwrap(), Some(format!("pong {i}")));
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    drop(girdi);
    let torun = format!("{isaret}-torun");
    let kok = format!("{isaret}-kok");
    let son = Instant::now() + Duration::from_secs(5);
    while surecler(&kok).is_empty() && Instant::now() < son {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(surecler(&torun).len(), 1, "torun yaşamalı");
    assert_eq!(surecler(&kok).len(), 1, "kök yaşamalı");
    let bas = Instant::now();
    let kapanis = s.kapat().await.unwrap();
    assert!(!kapanis.kendiliginden, "EOF'u yok sayan süreç kendiliğinden çıkmamalı");
    assert!(bas.elapsed() < Duration::from_secs(5), "{:?}", bas.elapsed());
    assert!(surecler(&torun).is_empty() && surecler(&kok).is_empty());
}

/// G1.3 ve §3: sunucunun özel kökü kabuğa görünmez; kabuğun özel ev'i
/// sunucuya görünmez; sunucu kendi köküne yazar. Aynı alanda iki sunucu
/// birbirini göremez (Review Focus 4). Kaldırma kökü siler.
#[tokio::test]
async fn etkilesimli_sunucu_ozel_koku_kabuktan_ve_obur_sunucudan_ayri() {
    let k = kurgu();
    let kimlik = kalfa_sandbox::alan_kimligi(&k.kok);
    let ydir = k.taban.join("veri/yalitim");
    let a = kalfa_sandbox::sunucu_ozel(&ydir, &kimlik, "a");
    let b = kalfa_sandbox::sunucu_ozel(&ydir, &kimlik, "b");
    let kabuk_ev = kalfa_sandbox::OzelDizinler::yol(&ydir, &kimlik).ev;
    let s =
        k.y.baslat_etkilesimli(&etkilesimli_istek(&k, "true", &[], "b"))
            .unwrap();
    s.kapat().await.unwrap();
    // Kabuğun özel dizinleri kurulsun.
    k.betik(&k.kok, "true", false).await;
    assert!(b.ev.is_dir() && kabuk_ev.is_dir());
    let betik = format!(
        "touch \"$HOME/yazildi\" && echo kendi=TAMAM; \
         test -e '{}' && echo kabuk=GORUNUR || echo kabuk=YOK; \
         test -e '{}' && echo obur=GORUNUR || echo obur=YOK",
        kabuk_ev.display(),
        b.kok.display()
    );
    let mut s =
        k.y.baslat_etkilesimli(&etkilesimli_istek(&k, "sh", &["-c", &betik], "a"))
            .unwrap();
    let metin = tum_cikti(&mut s).await;
    s.kapat().await.unwrap();
    assert!(metin.contains("kendi=TAMAM"), "{metin}");
    assert!(metin.contains("kabuk=YOK"), "{metin}");
    assert!(metin.contains("obur=YOK"), "{metin}");
    assert!(a.ev.join("yazildi").exists());
    let sonuc = k
        .betik(
            &k.kok,
            &format!("test -e '{}' && echo GORUNUR || echo YOK", a.kok.display()),
            false,
        )
        .await;
    assert!(sonuc.standart.contains("YOK"), "{}", sonuc.standart);
    k.y.sunucu_kokunu_kaldir(&k.kok, "a").unwrap();
    assert!(!a.kok.exists() && b.kok.exists());
}

/// G1 bulgusu ve §3: ortam özel köke yönelir; `EVREN_`/`KALFA_` adları
/// geçmez, yapılandırılmış belirteç geçer.
#[tokio::test]
async fn etkilesimli_ortam_ozel_koke_yonelir() {
    let k = kurgu();
    let mut i = etkilesimli_istek(&k, "env", &[], "ortam");
    i.ortam.push(("EVREN_API_KEY".into(), "evren_llm_x".into()));
    i.ortam.push(("KALFA_E2E".into(), "1".into()));
    i.ortam.push(("GITHUB_TOKEN".into(), "ghp_deneme".into()));
    let mut s = k.y.baslat_etkilesimli(&i).unwrap();
    let metin = tum_cikti(&mut s).await;
    s.kapat().await.unwrap();
    let ozel = kalfa_sandbox::sunucu_ozel(
        &k.taban.join("veri/yalitim"),
        &kalfa_sandbox::alan_kimligi(&k.kok),
        "ortam",
    );
    assert_eq!(deger(&metin, "HOME"), ozel.ev.display().to_string());
    assert_eq!(
        deger(&metin, "NPM_CONFIG_CACHE"),
        ozel.npm().display().to_string()
    );
    assert_eq!(
        deger(&metin, "UV_CACHE_DIR"),
        ozel.onbellek.join("uv").display().to_string()
    );
    assert_eq!(deger(&metin, "GITHUB_TOKEN"), "ghp_deneme");
    assert!(
        !metin
            .lines()
            .any(|s| s.starts_with("EVREN_") || s.starts_with("KALFA_")),
        "{metin}"
    );
}

/// §4.1: tam erişim, yalıtım hazırken yalıtımsız başlar (ev'deki sırrı
/// okur); yalıtımlı aynı sırrı okuyamaz.
#[tokio::test]
async fn etkilesimli_tam_erisim_yalitim_hazirken_yalitimsiz_baslar() {
    let k = kurgu();
    let sir = k.ev.join(".ssh/id_rsa");
    for (kip, gorunur) in [
        (kalfa_sandbox::CalistirmaKipi::TamErisim, true),
        (kalfa_sandbox::CalistirmaKipi::Yalitimli, false),
    ] {
        let mut i = etkilesimli_istek(&k, "cat", &[sir.to_str().unwrap()], "kip");
        i.kip = kip;
        let mut s = k.y.baslat_etkilesimli(&i).unwrap();
        let metin = tum_cikti(&mut s).await;
        s.kapat().await.unwrap();
        assert_eq!(metin.contains("GIZLI-SSH"), gorunur, "{kip:?}: {metin}");
    }
}

/// Codex 2: ağlı sunucu köprüyle başlar. stdin/stdout köprüden geçer;
/// izinli hedefe vekilden ulaşılır, listede olmayan reddedilir. `kapat`
/// döndüğünde köprü dahil süreç kalmaz ve ağ özeti gelir.
#[tokio::test]
async fn etkilesimli_agli_sunucu_vekilden_cikar_ve_kapanista_temizlenir() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let dinleyici = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = dinleyici.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut soket, _)) = dinleyici.accept().await {
            use tokio::io::AsyncReadExt;
            let mut buf = [0u8; 1024];
            let n = soket.read(&mut buf).await.unwrap_or(0);
            if String::from_utf8_lossy(&buf[..n]).starts_with("CONNECT") {
                let _ = soket
                    .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                    .await;
            } else {
                let _ = soket
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nSELAM",
                    )
                    .await;
            }
        }
    });
    let mut eslemeler = std::collections::HashMap::new();
    eslemeler.insert(
        ("fake.crates.io".to_string(), port),
        vec![std::net::SocketAddr::from(([127, 0, 0, 1], port))],
    );
    let k = kurgu_agli(Some(std::sync::Arc::new(TestCozucu { eslemeler })), true);
    let isaret = format!("kalfa-agli-{}", std::process::id());
    let betik = format!(
        "echo \"ayrilmis=$(env | grep -c -e ^KALFA_ -e ^EVREN_)\"; \
         while read l; do case \"$l\" in \
           izinli) curl -p -s -S http://fake.crates.io:{port}/; echo ;; \
           yasak) curl -s -i http://evil.com/ | head -n 1 ;; \
           *) echo \"pong $l\" ;; esac; done"
    );
    let mut i = etkilesimli_istek(&k, "sh", &["-c", &betik, &isaret], "agli");
    i.ag_listesi = Some(kalfa_sandbox::vekil::IzinListesi::yeni(
        &[],
        &[("fake.crates.io".to_string(), port)],
    ));
    let mut s = k.y.baslat_etkilesimli(&i).unwrap();
    let mut girdi = s.girdi_al().unwrap();
    let mut cikti = BufReader::new(s.cikti_al().unwrap()).lines();
    assert_eq!(
        cikti.next_line().await.unwrap().as_deref(),
        Some("ayrilmis=0")
    );
    let mut sor = async |satir: &str| {
        girdi
            .write_all(format!("{satir}\n").as_bytes())
            .await
            .unwrap();
        cikti.next_line().await.unwrap().unwrap()
    };
    assert_eq!(sor("1").await, "pong 1");
    assert!(sor("izinli").await.contains("SELAM"));
    assert!(sor("yasak").await.contains("403"));
    assert!(!surecler(&isaret).is_empty(), "köprü ve sunucu yaşamalı");
    let kapanis = s.kapat().await.unwrap();
    assert!(surecler(&isaret).is_empty(), "kapanıştan sonra süreç kaldı");
    let ag = kapanis.ag.expect("ağ özeti dönmeli");
    let karar = |host: &str, k: kalfa_sandbox::vekil::AgKarari| {
        ag.baglantilar.iter().any(|b| b.host == host && b.karar == k)
    };
    assert!(
        karar("fake.crates.io", kalfa_sandbox::vekil::AgKarari::Izinli),
        "{:?}",
        ag.baglantilar
    );
    assert!(
        karar("evil.com", kalfa_sandbox::vekil::AgKarari::ListedeYok),
        "{:?}",
        ag.baglantilar
    );
}
}
