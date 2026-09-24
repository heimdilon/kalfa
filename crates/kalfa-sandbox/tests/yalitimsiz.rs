//! Yalıtımsız arka uç gerçek süreçlerle (aşama 2 tasarımı §6, §12.1).

use std::time::Duration;

use kalfa_sandbox::{
    Akis, BaslatmaHatasi, Bitis, CalismaSonucu, CalistirmaIstegi, KomutBicimi, Yalitici,
    Yalitimsiz, ortam,
};
use tokio_util::sync::CancellationToken;

fn betik(metin: &str) -> KomutBicimi {
    KomutBicimi::Betik {
        metin: metin.to_string(),
    }
}

/// Platforma göre betik: Linux'ta sh, Windows'ta PowerShell.
fn sh_ya_da_ps(sh: &str, ps: &str) -> KomutBicimi {
    betik(if cfg!(windows) { ps } else { sh })
}

fn istek(komut: KomutBicimi) -> CalistirmaIstegi {
    CalistirmaIstegi {
        komut,
        dizin: std::env::temp_dir(),
        ortam: ortam::uret(std::env::vars()),
        ag: false,
    }
}

struct Toplanan {
    sonuc: CalismaSonucu,
    standart: String,
    hata: String,
}

async fn calistir(komut: KomutBicimi, sure: Duration, iptal: CancellationToken) -> Toplanan {
    let surec = Yalitimsiz.baslat(&istek(komut)).expect("komut başlamalı");
    let mut standart = Vec::new();
    let mut hata = Vec::new();
    let sonuc = surec
        .bekle(sure, iptal, |akis, veri| match akis {
            Akis::Standart => standart.extend_from_slice(veri),
            Akis::Hata => hata.extend_from_slice(veri),
        })
        .await;
    Toplanan {
        sonuc,
        standart: String::from_utf8_lossy(&standart).into_owned(),
        hata: String::from_utf8_lossy(&hata).into_owned(),
    }
}

const UZUN: Duration = Duration::from_secs(60);

#[tokio::test]
async fn cikis_kodu_ve_iki_akis_ayri_gelir() {
    let t = calistir(
        sh_ya_da_ps(
            "echo merhaba; echo sorun 1>&2; exit 3",
            "Write-Output merhaba; [Console]::Error.WriteLine('sorun'); exit 3",
        ),
        UZUN,
        CancellationToken::new(),
    )
    .await;
    assert_eq!(t.sonuc.bitis, Bitis::Cikti(Some(3)));
    assert_eq!(t.standart.trim(), "merhaba");
    assert_eq!(t.hata.trim(), "sorun");
}

#[tokio::test]
async fn program_bicimi_kabuksuz_calisir() {
    let komut = if cfg!(windows) {
        KomutBicimi::Program {
            program: "cmd".into(),
            argumanlar: vec!["/c".into(), "echo tamam".into()],
        }
    } else {
        KomutBicimi::Program {
            program: "sh".into(),
            argumanlar: vec!["-c".into(), "echo tamam".into()],
        }
    };
    let t = calistir(komut, UZUN, CancellationToken::new()).await;
    assert_eq!(t.sonuc.bitis, Bitis::Cikti(Some(0)));
    assert_eq!(t.standart.trim(), "tamam");
}

#[tokio::test]
async fn olmayan_program_baslatilmaz() {
    let hata = Yalitimsiz
        .baslat(&istek(KomutBicimi::Program {
            program: "kalfa-kesinlikle-olmayan-program".into(),
            argumanlar: vec![],
        }))
        .err()
        .expect("başlamamalı");
    assert!(matches!(hata, BaslatmaHatasi::ProgramYok { .. }), "{hata}");
}

#[tokio::test]
async fn izin_listesi_disindaki_ortam_gecmez() {
    // `cargo test` bu değişkeni test sürecine verir; izin listesinde yok.
    assert!(std::env::var("CARGO_MANIFEST_DIR").is_ok());
    let t = calistir(
        sh_ya_da_ps(
            "echo \"${CARGO_MANIFEST_DIR:-yok}|$TERM\"",
            "if ($env:CARGO_MANIFEST_DIR) { 'var|' + $env:TERM } else { 'yok|' + $env:TERM }",
        ),
        UZUN,
        CancellationToken::new(),
    )
    .await;
    assert_eq!(t.standart.trim(), "yok|dumb");
}

#[tokio::test]
async fn zaman_asimi_agaci_durdurur() {
    let t = calistir(
        sh_ya_da_ps("sleep 30", "Start-Sleep -Seconds 30"),
        Duration::from_millis(1500),
        CancellationToken::new(),
    )
    .await;
    assert_eq!(t.sonuc.bitis, Bitis::ZamanAsimi);
    assert!(t.sonuc.sure < Duration::from_secs(10), "{:?}", t.sonuc.sure);
}

#[tokio::test]
async fn iptal_torun_sureci_de_durdurur() {
    let iptal = CancellationToken::new();
    let tetik = iptal.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(2500)).await;
        tetik.cancel();
    });
    // Komut arka planda bir torun başlatır, onun kimliğini yazar ve bekler.
    let t = calistir(
        sh_ya_da_ps(
            "sleep 60 & echo $!; wait",
            "$p = Start-Process -PassThru -WindowStyle Hidden ping -ArgumentList '-n','60','127.0.0.1'; \
             Write-Output $p.Id; Start-Sleep -Seconds 60",
        ),
        UZUN,
        iptal,
    )
    .await;
    assert_eq!(t.sonuc.bitis, Bitis::Iptal);
    let torun: u32 = t
        .standart
        .trim()
        .parse()
        .expect("torunun kimliği yazılmalı");
    for _ in 0..30 {
        if !yasiyor_mu(torun) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("torun süreç {torun} hâlâ yaşıyor");
}

#[tokio::test]
async fn buyuk_cikti_eksiksiz_okunur() {
    let t = calistir(
        sh_ya_da_ps(
            "i=0; while [ $i -lt 20000 ]; do echo 0123456789012345678901234567890123456789012345678; i=$((i+1)); done",
            "$s = '0123456789012345678901234567890123456789012345678'; \
             [Console]::Out.Write((($s + \"`n\") * 20000))",
        ),
        UZUN,
        CancellationToken::new(),
    )
    .await;
    assert_eq!(t.sonuc.bitis, Bitis::Cikti(Some(0)));
    assert_eq!(t.standart.lines().count(), 20000);
}

#[cfg(unix)]
fn yasiyor_mu(pid: u32) -> bool {
    // Ölmüş ama biçilmemiş (zombi) süreç de ölü sayılır.
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .map(|stat| !stat.split(") ").nth(1).is_some_and(|s| s.starts_with('Z')))
        .unwrap_or(false)
}

#[cfg(windows)]
fn yasiyor_mu(pid: u32) -> bool {
    let cikti = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
        .output()
        .expect("tasklist çalışmalı");
    String::from_utf8_lossy(&cikti.stdout).contains(&format!("\"{pid}\""))
}
