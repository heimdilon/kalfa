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
use kalfa_sandbox::{
    AlanBilgisi, BaslatmaHatasi, CalistirmaIstegi, Surec, Yalitici, YalitimDurumu,
    YalitimYoklukNedeni, Yalitimsiz,
};
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
    let arac = KabukAraci::yeni(
        alan,
        Arc::new(Yalitimsiz),
        guven.clone(),
        Arc::new(AtomicBool::new(true)),
    );
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

/// Yalıtım hazırmış gibi davranan yalıtıcı: isteği kaydeder, onaylanan kiple
/// şimdiki durumu gerçek arka uçlar gibi `kip_uyusmasi` ile karşılaştırır ve
/// komutu yalıtımsız çalıştırır. `basarisiz` ise yalıtım başlatılamaz; `yok`
/// ise durum "yalıtım yok" olur (çalışma anında değiştirilebilir).
#[derive(Default)]
struct KayitliYalitici {
    istekler: std::sync::Mutex<Vec<CalistirmaIstegi>>,
    algilama: AtomicUsize,
    basarisiz: AtomicBool,
    yok: AtomicBool,
    /// Çalışma alanı hazırlanmamış (Windows, §9.4).
    hazirlanmamis: AtomicBool,
    /// `alani_hazirla` çağrılarının `hazirlik_onaylandi` değerleri.
    hazirliklar: std::sync::Mutex<Vec<bool>>,
}

impl Yalitici for KayitliYalitici {
    fn durum(&self) -> YalitimDurumu {
        if self.yok.load(Ordering::SeqCst) {
            return YalitimDurumu::Yok {
                neden: YalitimYoklukNedeni::Desteklenmiyor,
                talimat: "yok".into(),
                ayrinti: "denetim=deneme".into(),
            };
        }
        YalitimDurumu::Hazir {
            arka_uc: "bwrap".into(),
            surum: "0.9.0".into(),
        }
    }

    fn profil_ozeti(&self, ag: bool) -> Option<String> {
        Some(format!("ozet-{ag}"))
    }

    fn baslat(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi> {
        let kip = kalfa_sandbox::kip_uyusmasi(istek.alan.is_some(), self.durum().hazir_mi());
        self.istekler.lock().unwrap().push(istek.clone());
        kip?;
        if self.basarisiz.load(Ordering::SeqCst) {
            return Err(BaslatmaHatasi::Yalitim("deneme".into()));
        }
        Yalitimsiz.baslat(&CalistirmaIstegi {
            dizin: std::env::temp_dir(),
            alan: None,
            ..istek.clone()
        })
    }

    fn yeniden_algila(&self) -> YalitimDurumu {
        self.algilama.fetch_add(1, Ordering::SeqCst);
        self.durum()
    }

    fn hazirlik_gerekli(&self, _kok: &std::path::Path) -> bool {
        self.hazirlanmamis.load(Ordering::SeqCst)
    }

    fn alani_hazirla(
        &self,
        _alan: &AlanBilgisi,
        hazirlik_onaylandi: bool,
        ilerleme: &mut dyn FnMut(&str),
    ) -> Result<(), BaslatmaHatasi> {
        self.hazirliklar.lock().unwrap().push(hazirlik_onaylandi);
        if self.hazirlanmamis.load(Ordering::SeqCst) {
            if !hazirlik_onaylandi {
                return Err(BaslatmaHatasi::Yalitim(
                    "çalışma alanı hazırlanmamış".into(),
                ));
            }
            ilerleme("İzinler hazırlanıyor…");
            self.hazirlanmamis.store(false, Ordering::SeqCst);
        }
        Ok(())
    }
}

fn kur_yalitimli() -> (TempDir, KabukAraci, Arc<KayitliYalitici>) {
    let d = TempDir::new().unwrap();
    let s = Arc::new(Siniflandirici::yeni(d.path(), &[]).unwrap());
    let alan = CalismaAlani::ac(s).unwrap();
    let yalitici = Arc::new(KayitliYalitici::default());
    let arac = KabukAraci::yeni(
        alan,
        yalitici.clone(),
        Arc::new(SahteGuven::default()),
        Arc::new(AtomicBool::new(true)),
    );
    (d, arac, yalitici)
}

fn yazdir() -> Value {
    if cfg!(windows) {
        json!({"betik": "Write-Output tamam"})
    } else {
        json!({"betik": "echo tamam"})
    }
}

/// Windows'ta çalışma alanı ilk yalıtımlı çağrıda hazırlanır (§9.4): hazırlık
/// tarife girer (yerel onay), komuttan önce yapılır, ilerlemesi araç kartında
/// görünür.
#[tokio::test]
async fn hazirlik_tarife_girer_ve_komuttan_once_yapilir() {
    let (_d, arac, yalitici) = kur_yalitimli();
    yalitici.hazirlanmamis.store(true, Ordering::SeqCst);
    let t = arac.hazirla(&yazdir()).await.unwrap().kabuk.unwrap();
    assert!(t.alan_hazirligi);

    let (sonuc, parcalar) = calistir(&arac, &yazdir()).await;
    assert!(sonuc.unwrap().contains("tamam"));
    assert_eq!(*yalitici.hazirliklar.lock().unwrap(), vec![true]);
    assert!(
        parcalar
            .iter()
            .any(|(_, m)| m.contains("İzinler hazırlanıyor")),
        "{parcalar:?}"
    );
    assert_eq!(yalitici.istekler.lock().unwrap().len(), 1);

    // Hazırlanmış çalışma alanında sonraki çağrı hazırlık istemez; araç yine
    // her çalıştırmadan önce yalıtıcıya sorar (yeni gizli dosyalar).
    let t = arac.hazirla(&yazdir()).await.unwrap().kabuk.unwrap();
    assert!(!t.alan_hazirligi);
    let (sonuc, _) = calistir(&arac, &yazdir()).await;
    sonuc.unwrap();
    assert_eq!(*yalitici.hazirliklar.lock().unwrap(), vec![true, false]);
}

/// Onaylanan tarif hazırlık içermiyorsa (onaydan sonra hazırlık geri
/// alındıysa) komut başlamaz.
#[tokio::test]
async fn hazirlik_onaylanmadiysa_komut_baslamaz() {
    let (_d, arac, yalitici) = kur_yalitimli();
    let h = arac.hazirla(&yazdir()).await.unwrap();
    assert!(!h.kabuk.as_ref().unwrap().alan_hazirligi);
    yalitici.hazirlanmamis.store(true, Ordering::SeqCst);
    let sonuc = arac.calistir(&yazdir(), &h, CancellationToken::new()).await;
    assert!(sonuc.is_err());
    assert_eq!(*yalitici.hazirliklar.lock().unwrap(), vec![false]);
    assert!(yalitici.istekler.lock().unwrap().is_empty());
}

#[tokio::test]
async fn yalitimsizken_hazirlik_tarife_girmez() {
    let (_d, arac, yalitici) = kur_yalitimli();
    yalitici.yok.store(true, Ordering::SeqCst);
    yalitici.hazirlanmamis.store(true, Ordering::SeqCst);
    let t = arac.hazirla(&yazdir()).await.unwrap().kabuk.unwrap();
    assert!(!t.alan_hazirligi);
}

#[tokio::test]
async fn yalitim_hazirsa_tarif_profil_ozetini_tasir_ve_alan_bilgisi_verilir() {
    let (d, arac, yalitici) = kur_yalitimli();
    std::fs::create_dir_all(d.path().join(".git/hooks")).unwrap();
    std::fs::write(d.path().join(".git/config"), "").unwrap();
    std::fs::create_dir_all(d.path().join("alt")).unwrap();
    std::fs::write(d.path().join(".env"), "x").unwrap();
    std::fs::write(d.path().join("alt/sunucu.pem"), "x").unwrap();

    let t = arac
        .hazirla(&json!({"betik": "echo a"}))
        .await
        .unwrap()
        .kabuk
        .unwrap();
    assert_eq!(
        t.yalitim,
        YalitimOzeti::Yalitimli {
            arka_uc: "bwrap".into(),
            profil: "ozet-false".into(),
        }
    );
    let t = arac
        .hazirla(&json!({"betik": "echo a", "ag": true}))
        .await
        .unwrap()
        .kabuk
        .unwrap();
    assert!(matches!(t.yalitim, YalitimOzeti::Yalitimli { profil, .. } if profil == "ozet-true"));

    let yazdir = if cfg!(windows) {
        "Write-Output tamam"
    } else {
        "echo tamam"
    };
    let (sonuc, _) = calistir(&arac, &json!({"betik": yazdir, "dizin": "alt"})).await;
    assert!(sonuc.unwrap().contains("tamam"));
    let istek = yalitici.istekler.lock().unwrap().pop().unwrap();
    let kok = d.path().canonicalize().unwrap();
    let alan = istek.alan.expect("yalıtımlı istek alan bilgisi taşır");
    assert_eq!(alan.kok, kok);
    assert_eq!(istek.dizin, kok.join("alt"));
    assert_eq!(
        alan.gizli.dosyalar,
        vec![kok.join(".env"), kok.join("alt").join("sunucu.pem")]
    );
    let korumali: Vec<_> = alan.korumali.iter().map(|k| k.yol.clone()).collect();
    assert_eq!(korumali, vec![kok.join(".git")]);
    assert!(alan.git_dizinleri.is_empty());
}

/// İstek, onaylanan hazırlıktaki kiple gider (§6.3): mühür denetimiyle
/// çalıştırma arasında durum değişse de kip şimdiki durumdan çıkarılmaz;
/// uyuşmazlığı arka uç reddeder.
#[tokio::test]
async fn istek_onaylanan_kiple_gider() {
    let (_d, arac, yalitici) = kur_yalitimli();
    let a = json!({"betik": "echo a"});
    let son_istek = || yalitici.istekler.lock().unwrap().pop().unwrap();

    let h = arac.hazirla(&a).await.unwrap();
    yalitici.yok.store(true, Ordering::SeqCst);
    let _ = arac
        .calistir_akisli(&a, &h, CancellationToken::new(), CiktiGonderici::bos())
        .await;
    assert!(
        son_istek().alan.is_some(),
        "yalıtımlı onay yalıtımlı istenir"
    );

    let h = arac.hazirla(&a).await.unwrap();
    yalitici.yok.store(false, Ordering::SeqCst);
    let _ = arac
        .calistir_akisli(&a, &h, CancellationToken::new(), CiktiGonderici::bos())
        .await;
    assert!(
        son_istek().alan.is_none(),
        "yalıtımsız onay yalıtımsız istenir"
    );
}

#[tokio::test]
async fn yalitim_baslatilamazsa_algilama_yeniden_calisir() {
    let (_d, arac, yalitici) = kur_yalitimli();
    yalitici.basarisiz.store(true, Ordering::SeqCst);
    let (sonuc, _) = calistir(&arac, &json!({"betik": "echo a"})).await;
    match sonuc {
        Err(AracHatasi::Gecersiz(m)) => assert!(m.contains("Yalıtım başlatılamadı"), "{m}"),
        diger => panic!("{diger:?}"),
    }
    assert_eq!(yalitici.algilama.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn bwrap_kurulum_hatasi_algilamayi_yeniden_calistirir() {
    let (_d, arac, yalitici) = kur_yalitimli();
    let (sonuc, _) = calistir(
        &arac,
        &betik(
            "echo 'bwrap: Can not mount tmpfs' >&2; exit 1",
            "[Console]::Error.WriteLine('bwrap: Can not mount tmpfs'); exit 1",
        ),
    )
    .await;
    assert!(matches!(sonuc, Err(AracHatasi::Basarisiz(_))), "{sonuc:?}");
    assert_eq!(yalitici.algilama.load(Ordering::SeqCst), 1);
    // Çıkış kodu 1 ama bwrap öneki yoksa algılama çalışmaz.
    let _ = calistir(
        &arac,
        &betik(
            "echo hata >&2; exit 1",
            "[Console]::Error.WriteLine('hata'); exit 1",
        ),
    )
    .await;
    assert_eq!(yalitici.algilama.load(Ordering::SeqCst), 1);
}

/// Aşama 2 tasarımı §9.8: yalıtım yokken kabuk ancak yalıtımsız çalıştırma
/// anahtarı açıksa sunulur.
fn kur_izinle(
    yalitici: Arc<dyn Yalitici>,
    izin: bool,
) -> (TempDir, KabukAraci, Arc<AtomicBool>, Arc<SahteGuven>) {
    let d = TempDir::new().unwrap();
    let s = Arc::new(Siniflandirici::yeni(d.path(), &[]).unwrap());
    let alan = CalismaAlani::ac(s).unwrap();
    let izin = Arc::new(AtomicBool::new(izin));
    let guven = Arc::new(SahteGuven::default());
    let arac = KabukAraci::yeni(alan, yalitici, guven.clone(), izin.clone());
    (d, arac, izin, guven)
}

#[tokio::test]
async fn kabuk_yalitim_yokken_ve_anahtar_kapaliyken_sunulmaz() {
    let (_d, arac, izin, _guven) = kur_izinle(Arc::new(Yalitimsiz), false);
    assert!(!arac.sunulur_mu());
    match arac.hazirla(&json!({"betik": "echo a"})).await {
        Err(AracHatasi::Gecersiz(m)) => assert!(m.contains("yalıtımsız çalıştırma kapalı"), "{m}"),
        diger => panic!("{diger:?}"),
    }
    izin.store(true, Ordering::SeqCst);
    assert!(arac.sunulur_mu());
    let t = arac
        .hazirla(&json!({"betik": "echo a"}))
        .await
        .unwrap()
        .kabuk
        .unwrap();
    assert!(matches!(t.yalitim, YalitimOzeti::Yalitimsiz { .. }));
}

#[tokio::test]
async fn kabuk_yalitim_hazirsa_anahtar_kapaliyken_de_sunulur() {
    let (_d, arac, _izin, _guven) = kur_izinle(Arc::new(KayitliYalitici::default()), false);
    assert!(arac.sunulur_mu());
    let t = arac.hazirla(&yazdir()).await.unwrap().kabuk.unwrap();
    assert!(matches!(t.yalitim, YalitimOzeti::Yalitimli { .. }));
}

/// Onaydan sonra anahtar kapatılırsa yalıtımsız onaylanan komut başlamaz:
/// araç onaylanan tarife göre ve çalışma anındaki izinle çalışır.
#[tokio::test]
async fn yalitimsiz_onaylanan_komut_anahtar_kapaninca_baslamaz() {
    let (d, arac, izin, _guven) = kur_izinle(Arc::new(Yalitimsiz), true);
    let a = betik(
        "echo x > isaret.txt",
        "Set-Content -Path isaret.txt -Value x",
    );
    let h = arac.hazirla(&a).await.unwrap();
    izin.store(false, Ordering::SeqCst);
    let sonuc = arac
        .calistir_akisli(&a, &h, CancellationToken::new(), CiktiGonderici::bos())
        .await;
    match sonuc {
        Err(AracHatasi::Gecersiz(m)) => assert!(m.contains("yalıtımsız çalıştırma kapalı"), "{m}"),
        diger => panic!("{diger:?}"),
    }
    assert!(!d.path().join("isaret.txt").exists(), "komut başlamamalı");
}

/// Onaylanan tarif ile çalışma anının sınır matrisi (2d-3 planı, Görev 2):
/// onaylanan kip × çalışma anında yalıtım × yalıtımsız anahtarı × ağ. Hazırlık
/// onay anındaki durumla yapılır; durum ve anahtar çalıştırmadan önce
/// değiştirilir. Süreç yalnızca (yalıtımlı ∧ hazır) ya da (yalıtımsız ∧ yok ∧
/// anahtar açık) iken başlar ve istekte onaylanan `ag` gider. Başlamayan
/// hücrede komutun işaret dosyası oluşmaz ve güven kaydedilmez.
#[tokio::test]
async fn sinir_matrisi() {
    let mut hucreler = Vec::new();
    for onay_yalitimli in [true, false] {
        for calisma_hazir in [true, false] {
            for anahtar in [true, false] {
                for ag in [true, false] {
                    hucreler.push((onay_yalitimli, calisma_hazir, anahtar, ag));
                }
            }
        }
    }
    assert_eq!(hucreler.len(), 16);

    for (onay_yalitimli, calisma_hazir, anahtar, ag) in hucreler {
        let hucre = format!(
            "onay_yalitimli={onay_yalitimli} calisma_hazir={calisma_hazir} \
             anahtar={anahtar} ag={ag}"
        );
        let yalitici = Arc::new(KayitliYalitici::default());
        // Yalıtımsız onay ancak anahtar açıkken verilebilir; anahtar
        // çalıştırmadan önce hücrenin değerine getirilir.
        let (d, arac, izin, guven) = kur_izinle(yalitici.clone(), true);
        let isaret = d.path().join("isaret.txt");
        let metin = if cfg!(windows) {
            format!("Set-Content -LiteralPath '{}' -Value x", isaret.display())
        } else {
            format!("echo x > '{}'", isaret.display())
        };
        let a = json!({ "betik": metin, "ag": ag });

        // Onay anı.
        yalitici.yok.store(!onay_yalitimli, Ordering::SeqCst);
        let h = arac.hazirla(&a).await.unwrap();
        let tarif = h.kabuk.as_ref().unwrap();
        assert_eq!(tarif.yalitim.yalitimli_mi(), onay_yalitimli, "{hucre}");
        assert_eq!(tarif.ag, ag, "{hucre}");

        // Çalışma anı.
        yalitici.yok.store(!calisma_hazir, Ordering::SeqCst);
        izin.store(anahtar, Ordering::SeqCst);
        let sonuc = arac
            .calistir_akisli(&a, &h, CancellationToken::new(), CiktiGonderici::bos())
            .await;

        let baslamali =
            (onay_yalitimli && calisma_hazir) || (!onay_yalitimli && !calisma_hazir && anahtar);
        if baslamali {
            // Sıfırdan farklı çıkış `Err(Basarisiz)` döner; `Ok` komutun
            // başarıyla bittiğini söyler.
            sonuc.unwrap_or_else(|e| panic!("{hucre}: {e:?}"));
            assert!(isaret.exists(), "{hucre}: komut başlamalıydı");
            assert_eq!(guven.kayit.load(Ordering::SeqCst), 1, "{hucre}");
            let istek = yalitici.istekler.lock().unwrap().pop().unwrap();
            assert_eq!(istek.ag, ag, "{hucre}: onaylanan ağ gitmeli");
            assert_eq!(istek.alan.is_some(), onay_yalitimli, "{hucre}");
        } else {
            let beklenen = if onay_yalitimli {
                "artık kullanılamıyor"
            } else if !anahtar {
                "yalıtımsız çalıştırma kapalı"
            } else {
                "artık kullanılabiliyor"
            };
            match sonuc {
                Err(AracHatasi::Gecersiz(m)) => assert!(m.contains(beklenen), "{hucre}: {m}"),
                diger => panic!("{hucre}: {diger:?}"),
            }
            assert!(!isaret.exists(), "{hucre}: komut başlamamalıydı");
            assert_eq!(guven.kayit.load(Ordering::SeqCst), 0, "{hucre}");
        }
    }
}
