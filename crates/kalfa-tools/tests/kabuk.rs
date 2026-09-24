//! `kabuk` aracı gerçek komutlarla, yalıtımsız arka uçta (aşama 2 tasarımı
//! §4, §12.1).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use kalfa_core::arac::{Arac, AracHatasi, AracKayitDefteri, CiktiGonderici, Hazirlik};
use kalfa_core::izin::Siniflandirici;
use kalfa_core::kabuk::{KabukBicimi, YalitimOzeti};
use kalfa_core::kimlik::CagriId;
use kalfa_core::protokol::CiktiAkisi;
use kalfa_sandbox::Yalitimsiz;
use kalfa_tools::{CalismaAlani, KabukAraci, KabukGuveni};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct SahteGuven {
    guvenildi: AtomicBool,
    kayit: AtomicUsize,
}

impl KabukGuveni for SahteGuven {
    fn guvenildi_mi(&self) -> bool {
        self.guvenildi.load(Ordering::SeqCst)
    }
    fn guvenildi(&self) {
        self.guvenildi.store(true, Ordering::SeqCst);
        self.kayit.fetch_add(1, Ordering::SeqCst);
    }
}

fn kur() -> (TempDir, KabukAraci, Arc<SahteGuven>) {
    let d = TempDir::new().unwrap();
    let s = Arc::new(Siniflandirici::yeni(d.path(), &[]).unwrap());
    let alan = CalismaAlani::ac(s).unwrap();
    let guven = Arc::new(SahteGuven::default());
    let arac = KabukAraci::yeni(alan, Arc::new(Yalitimsiz), guven.clone());
    (d, arac, guven)
}

/// Platforma göre betik: Linux'ta sh, Windows'ta PowerShell.
fn betik(sh: &str, ps: &str) -> Value {
    json!({ "betik": if cfg!(windows) { ps } else { sh } })
}

async fn calistir(
    arac: &KabukAraci,
    a: &Value,
) -> (Result<String, AracHatasi>, Vec<(CiktiAkisi, String)>) {
    let h = arac.hazirla(a).await.expect("hazırlık");
    let (tx, mut rx) = tokio::sync::mpsc::channel(1024);
    let sonuc = arac
        .calistir_akisli(
            a,
            &h,
            CancellationToken::new(),
            CiktiGonderici::yeni(CagriId::yeni(), tx),
        )
        .await
        .map(|c| format!("[{}] {}", c.kaynak, c.icerik));
    let mut parcalar = Vec::new();
    while let Ok(p) = rx.try_recv() {
        parcalar.push((p.akis, p.metin));
    }
    (sonuc, parcalar)
}

#[tokio::test]
async fn hazirlik_tarifi_uretir_ve_ilk_cagriyi_bilir() {
    let (d, arac, guven) = kur();
    std::fs::create_dir(d.path().join("alt")).unwrap();
    let a = json!({
        "program": "cargo",
        "argumanlar": ["test", "--features=e2e", "iki kelime"],
        "dizin": "alt",
        "ag": true
    });
    let h: Hazirlik = arac.hazirla(&a).await.unwrap();
    assert!(
        h.tum_calisma_alani,
        "kabuk çalışma alanının özel kilidini alır"
    );
    assert_eq!(
        h.aciklama,
        "`cargo test --features=e2e 'iki kelime'` komutunu çalıştırmak istiyor."
    );
    let t = h.kabuk.expect("kabuk tarifi");
    assert_eq!(t.komut, "cargo test --features=e2e 'iki kelime'");
    assert_eq!(t.bicim, KabukBicimi::Program);
    assert_eq!(t.dizin, "alt");
    assert!(t.ag);
    assert!(matches!(t.yalitim, YalitimOzeti::Yalitimsiz { .. }));
    assert!(t.ilk_cagri);

    guven.guvenildi.store(true, Ordering::SeqCst);
    let t = arac
        .hazirla(&json!({"betik": "echo a"}))
        .await
        .unwrap()
        .kabuk
        .unwrap();
    assert!(!t.ilk_cagri);
    assert!(matches!(t.bicim, KabukBicimi::Betik { .. }));
    assert_eq!(t.dizin, ".");
}

#[tokio::test]
async fn gecersiz_argumanlar_reddedilir() {
    let (d, arac, _) = kur();
    std::fs::write(d.path().join("dosya.txt"), "x").unwrap();
    let defter = AracKayitDefteri::yeni(vec![Arc::new(kur().1) as Arc<dyn Arac>]).unwrap();
    for metin in [
        r#"{"program": "a", "betik": "b"}"#,
        r#"{"dizin": "."}"#,
        r#"{"program": "a", "fazla": 1}"#,
        r#"{"program": "a", "zaman_asimi_sn": 5000}"#,
    ] {
        assert!(defter.dogrula("kabuk", metin).is_err(), "{metin}");
    }
    for a in [
        json!({"program": "/usr/bin/env"}),
        json!({"program": "C:\\Windows\\System32\\cmd.exe"}),
        json!({"program": "../disarida"}),
        json!({"program": "a", "dizin": ".."}),
        json!({"program": "a", "dizin": "yok"}),
        json!({"program": "a", "dizin": "dosya.txt"}),
        json!({"betik": "echo", "argumanlar": ["a"]}),
        json!({"betik": "   "}),
        json!({"program": ""}),
    ] {
        assert!(arac.hazirla(&a).await.is_err(), "{a}");
    }
}

#[tokio::test]
async fn calisma_dizini_baglanti_olamaz() {
    let (d, arac, _) = kur();
    let dis = TempDir::new().unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(dis.path(), d.path().join("baglanti")).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(dis.path(), d.path().join("baglanti"))
        .expect("Windows test ajanında symlink yetkisi gerekir");
    assert!(
        arac.hazirla(&json!({"program": "a", "dizin": "baglanti"}))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn basarili_komut_ciktisi_modele_ve_canli_akisa_gider() {
    let (d, arac, guven) = kur();
    std::fs::create_dir(d.path().join("alt")).unwrap();
    let mut a = betik(
        "echo merhaba; pwd; echo uyari 1>&2",
        "Write-Output merhaba; (Get-Location).Path; [Console]::Error.WriteLine('uyari')",
    );
    a["dizin"] = json!("alt");
    let (sonuc, parcalar) = calistir(&arac, &a).await;
    let metin = sonuc.expect("çıkış kodu 0");
    let kaynak = if cfg!(windows) {
        "kabuk:PowerShell"
    } else {
        "kabuk:sh"
    };
    assert!(
        metin.starts_with(&format!("[{kaynak}] Çıkış kodu: 0\nSüre: ")),
        "{metin}"
    );
    assert!(metin.contains("--- çıktı ---\nmerhaba"), "{metin}");
    assert!(
        metin.contains("alt"),
        "komut `alt` dizininde çalışmalı: {metin}"
    );
    assert!(
        parcalar
            .iter()
            .any(|(a, m)| *a == CiktiAkisi::Standart && m.contains("merhaba"))
    );
    assert!(
        parcalar
            .iter()
            .any(|(a, m)| *a == CiktiAkisi::Hata && m.contains("uyari"))
    );
    assert_eq!(guven.kayit.load(Ordering::SeqCst), 1, "ilk onay kaydedilir");
}

#[tokio::test]
async fn program_bicimi_calisir() {
    let (_d, arac, _) = kur();
    let a = if cfg!(windows) {
        json!({"program": "cmd", "argumanlar": ["/c", "echo tamam"]})
    } else {
        json!({"program": "sh", "argumanlar": ["-c", "echo tamam"]})
    };
    let (sonuc, _) = calistir(&arac, &a).await;
    let metin = sonuc.unwrap();
    assert!(metin.contains("tamam"), "{metin}");
    let ad = if cfg!(windows) {
        "kabuk:cmd"
    } else {
        "kabuk:sh"
    };
    assert!(metin.starts_with(&format!("[{ad}]")), "{metin}");
}

#[tokio::test]
async fn sifirdan_farkli_cikis_basarisiz_ama_ciktisiyla_doner() {
    let (_d, arac, _) = kur();
    let (sonuc, _) = calistir(
        &arac,
        &betik("echo bozuk; exit 3", "Write-Output bozuk; exit 3"),
    )
    .await;
    match sonuc {
        Err(AracHatasi::Basarisiz(c)) => {
            assert!(c.icerik.starts_with("Çıkış kodu: 3\n"), "{}", c.icerik);
            assert!(c.icerik.contains("bozuk"));
        }
        diger => panic!("{diger:?}"),
    }
}

#[tokio::test]
async fn zaman_asimi_basarisiz_doner() {
    let (_d, arac, _) = kur();
    let mut a = betik("sleep 30", "Start-Sleep -Seconds 30");
    a["zaman_asimi_sn"] = json!(1);
    let (sonuc, _) = calistir(&arac, &a).await;
    match sonuc {
        Err(AracHatasi::Basarisiz(c)) => {
            assert!(c.icerik.starts_with("Zaman aşımı (1 sn)"), "{}", c.icerik)
        }
        diger => panic!("{diger:?}"),
    }
}

#[tokio::test]
async fn iptal_komutu_durdurur() {
    let (_d, arac, _) = kur();
    let a = betik("sleep 30", "Start-Sleep -Seconds 30");
    let h = arac.hazirla(&a).await.unwrap();
    let iptal = CancellationToken::new();
    let tetik = iptal.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(1500)).await;
        tetik.cancel();
    });
    let baslangic = std::time::Instant::now();
    let sonuc = arac
        .calistir_akisli(&a, &h, iptal, CiktiGonderici::bos())
        .await;
    assert!(matches!(sonuc, Err(AracHatasi::Iptal)), "{sonuc:?}");
    assert!(baslangic.elapsed() < Duration::from_secs(10));
}

#[tokio::test]
async fn olmayan_program_hata_verir() {
    let (_d, arac, guven) = kur();
    let (sonuc, _) = calistir(&arac, &json!({"program": "kalfa-kesinlikle-olmayan"})).await;
    match sonuc {
        Err(AracHatasi::Gecersiz(m)) => assert!(m.contains("bulunamadı"), "{m}"),
        diger => panic!("{diger:?}"),
    }
    assert_eq!(
        guven.kayit.load(Ordering::SeqCst),
        0,
        "başlamayan komut güven kaydetmez"
    );
}
