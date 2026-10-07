//! `kabuk` aracı gerçek komutlarla, yalıtımsız arka uçta (aşama 2 tasarımı
//! §4, §12.1).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, AracKayitDefteri, CiktiGonderici, Hazirlik};
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

async fn calistir_tam(arac: &KabukAraci, a: &Value) -> Result<AracCiktisi, AracHatasi> {
    let h = arac.hazirla(a).await.expect("hazırlık");
    arac.calistir_akisli(a, &h, CancellationToken::new(), CiktiGonderici::bos())
        .await
}

fn karantinadakiler(dizin: &std::path::Path) -> Vec<String> {
    let mut adlar: Vec<String> = std::fs::read_dir(dizin)
        .unwrap()
        .map(|g| g.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|ad| ad.starts_with(".git.kalfa-karantina-"))
        .collect();
    adlar.sort();
    adlar
}

/// Aşama 3 tasarımı §5.4: komutun çalışırken değiştirdiği dosyalar sonuca
/// eklenir. Önceden var olan, `.gitignore` kapsamındaki ve silinen dosyalar
/// dahildir; `.git`'in içi değildir. Başarısız çıkışta da eklenir.
#[tokio::test]
async fn kabugun_degistirdigi_dosyalar_sonuca_eklenir() {
    use kalfa_core::arac::DegisiklikTuru::{Degisti, Eklendi, Silindi};
    let (d, arac, _) = kur();
    std::fs::write(d.path().join("silinecek.txt"), "x").unwrap();
    std::fs::write(d.path().join("degisecek.txt"), "x").unwrap();
    std::fs::write(d.path().join("ayni.txt"), "x").unwrap();
    std::fs::write(d.path().join(".gitignore"), "*.log\n").unwrap();
    let a = betik(
        "git init -q && rm silinecek.txt && echo uzun-icerik > degisecek.txt && echo x > yeni.log && mkdir alt && echo y > alt/yeni.rs",
        "git init -q; Remove-Item silinecek.txt; Set-Content degisecek.txt 'uzun-icerik'; Set-Content yeni.log 'x'; New-Item -ItemType Directory alt | Out-Null; Set-Content alt/yeni.rs 'y'",
    );
    let c = calistir_tam(&arac, &a).await.unwrap();
    let g = c.degisiklikler.expect("kabuk değişiklikleri verir");
    let dosyalar: Vec<(&str, _)> = g.dosyalar.iter().map(|f| (f.yol.as_str(), f.tur)).collect();
    assert_eq!(
        dosyalar,
        vec![
            ("alt/yeni.rs", Eklendi),
            ("degisecek.txt", Degisti),
            ("silinecek.txt", Silindi),
            ("yeni.log", Eklendi),
        ]
    );
    assert_eq!(g.toplam, 4);
    assert!(g.tam_mi());
    assert_eq!(g.dizinler.len(), 1);
    assert_eq!(g.dizinler[0].dizin, "alt");
    assert_eq!(g.dizinler[0].eklenen, 1);

    let a = betik("echo z > b.txt; exit 3", "Set-Content b.txt 'z'; exit 3");
    let Err(AracHatasi::Basarisiz(c)) = calistir_tam(&arac, &a).await else {
        panic!("komut başarısız olmalıydı");
    };
    let g = c.degisiklikler.expect("başarısız çıkışta da");
    assert_eq!(g.dosyalar.len(), 1);
    assert_eq!(g.dosyalar[0].yol, "b.txt");
}

/// Güvenlik analizi Bulgu 1: düz `git init` yerinde kalır, uyarı yoktur.
#[tokio::test]
async fn kabuk_duz_git_init_korunur() {
    let (d, arac, _) = kur();
    let c = calistir_tam(
        &arac,
        &json!({"program": "git", "argumanlar": ["init", "-q"]}),
    )
    .await
    .unwrap();
    assert!(d.path().join(".git").is_dir());
    assert!(
        c.guvenlik_uyarilari.is_empty(),
        "{:?}",
        c.guvenlik_uyarilari
    );
    assert!(karantinadakiler(d.path()).is_empty());
    // Var olan `.git` ikinci komutta yeni sayılmaz.
    let c = calistir_tam(&arac, &betik("echo a", "echo a"))
        .await
        .unwrap();
    assert!(c.guvenlik_uyarilari.is_empty());
}

/// Güvenlik analizi Bulgu 1: komutun oluşturduğu tehlikeli `.git` (yeni
/// yapılandırma anahtarı, `.git` dosyası) kenara alınır; ardından çalışan
/// `git status` kod çalıştırmaz.
#[tokio::test]
async fn kabuk_tehlikeli_yeni_git_karantinaya_alinir() {
    let (d, arac, _) = kur();
    let a = betik(
        "git init -q && git config core.fsmonitor 'touch KANIT' && mkdir alt && echo 'gitdir: ../zz' > alt/.git",
        "git init -q; git config core.fsmonitor 'touch KANIT'; mkdir alt | Out-Null; Set-Content alt/.git 'gitdir: ../zz'",
    );
    let c = calistir_tam(&arac, &a).await.unwrap();
    assert!(!d.path().join(".git").exists());
    assert!(!d.path().join("alt/.git").exists());
    assert_eq!(karantinadakiler(d.path()).len(), 1);
    assert_eq!(karantinadakiler(&d.path().join("alt")).len(), 1);
    assert_eq!(c.guvenlik_uyarilari.len(), 2, "{:?}", c.guvenlik_uyarilari);
    assert!(
        c.guvenlik_uyarilari.iter().any(|u| u.contains("`.git`")
            && u.contains("core.fsmonitor")
            && u.contains("kenara alındı")),
        "{:?}",
        c.guvenlik_uyarilari
    );
    let durum = std::process::Command::new("git")
        .arg("-C")
        .arg(d.path())
        .arg("status")
        .output()
        .unwrap();
    assert!(!durum.status.success(), "depo artık yok");
    assert!(!d.path().join("KANIT").exists());
}

/// Güvenlik analizi Bulgu 2: komutun oluşturduğu kendiliğinden çalışan
/// yapılandırma bildirilir; dosya yerinde kalır. Var olan yapılandırmanın
/// içindekiler yeniden bildirilmez.
#[tokio::test]
async fn yeni_envrc_bildirilir() {
    let (d, arac, _) = kur();
    std::fs::create_dir(d.path().join(".vscode")).unwrap();
    let c = calistir_tam(
        &arac,
        &betik(
            "echo x > .envrc && echo '{}' > .vscode/tasks.json",
            "Set-Content .envrc x; Set-Content .vscode/tasks.json '{}'",
        ),
    )
    .await
    .unwrap();
    assert!(d.path().join(".envrc").exists());
    assert_eq!(c.guvenlik_uyarilari.len(), 1, "{:?}", c.guvenlik_uyarilari);
    assert!(
        c.guvenlik_uyarilari[0].contains("`.envrc`")
            && c.guvenlik_uyarilari[0].contains("yapılandırma"),
        "{:?}",
        c.guvenlik_uyarilari
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
    sahte_ag: std::sync::Mutex<Option<kalfa_sandbox::vekil::AgOzeti>>,
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

    fn profil_ozeti(&self, ag: bool, liste_ozeti: Option<&str>) -> Option<String> {
        match liste_ozeti {
            Some(lo) => Some(format!("ozet-{ag}-{lo}")),
            None => Some(format!("ozet-{ag}")),
        }
    }

    fn baslat(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi> {
        let kip = kalfa_sandbox::kip_uyusmasi(istek.alan.is_some(), self.durum().hazir_mi());
        self.istekler.lock().unwrap().push(istek.clone());
        kip?;
        if self.basarisiz.load(Ordering::SeqCst) {
            return Err(BaslatmaHatasi::Yalitim("deneme".into()));
        }
        let mut surec = Yalitimsiz.baslat(&CalistirmaIstegi {
            dizin: std::env::temp_dir(),
            alan: None,
            ..istek.clone()
        })?;
        if let Some(ozet) = self.sahte_ag.lock().unwrap().clone() {
            surec.ag_kapat_ekle(Box::new(|| Box::pin(async move { ozet })));
        }
        Ok(surec)
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

/// Aynı turda birden çok kabuk çağrısı hazırlanmamış çalışma alanında yerel
/// pencerede onaylanır (paralel araç çağrıları). İlki alanı hazırlar;
/// ötekiler "onaydan sonra değişti" ile düşmez, onaylanan tarifle çalışır.
/// Alan bir kez hazırlanır.
#[tokio::test]
async fn ayni_turdaki_kabuklar_hazirliktan_sonra_da_calisir() {
    use kalfa_core::izin::Kip;
    use kalfa_core::oturum::OturumAyarlari;
    use kalfa_core::protokol::{AracDurumu, BitisNedeni, Komut, Olay};
    use kalfa_core::test_destek as td;

    let (d, arac, yalitici) = kur_yalitimli();
    yalitici.hazirlanmamis.store(true, Ordering::SeqCst);
    let s = td::SahteSaglayici::yeni(vec![
        td::SahteYanit::araclar(vec![
            ("kabuk", yazdir()),
            ("kabuk", yazdir()),
            ("kabuk", yazdir()),
        ]),
        td::SahteYanit::metin("bitti"),
    ]);
    let mut ayar = OturumAyarlari::yeni("glm-5.3", d.path());
    ayar.kip = Kip::Serbest;
    let siniflandirici = Arc::new(Siniflandirici::yeni(d.path(), &[]).unwrap());
    let mut surucu = td::test_oturumu(
        td::test_baglami(s, vec![Arc::new(arac) as Arc<dyn Arac>], siniflandirici),
        ayar,
    );
    surucu
        .gonder(Komut::KullaniciMesaji {
            metin: "çalıştır".into(),
        })
        .await;
    let ogeler = match surucu
        .bekle(|o| matches!(o, Olay::OnayGerekiyor { .. }))
        .await
        .yuk
    {
        Olay::OnayGerekiyor { ogeler, .. } => ogeler,
        _ => unreachable!(),
    };
    assert_eq!(ogeler.len(), 3);
    for oge in ogeler {
        assert!(oge.yerel_onay);
        assert!(oge.kabuk.as_ref().unwrap().alan_hazirligi);
        surucu
            .gonder(Komut::YerelOnay {
                cagri: oge.cagri,
                muhur: oge.muhur,
            })
            .await;
    }
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    let sonuclar: Vec<(AracDurumu, String)> = surucu
        .gecmis
        .iter()
        .filter_map(|z| match &z.yuk {
            Olay::AracSonucu { durum, icerik, .. } => Some((*durum, icerik.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(sonuclar.len(), 3);
    for (durum, icerik) in &sonuclar {
        assert_eq!(*durum, AracDurumu::Basarili, "{icerik}");
        assert!(icerik.contains("tamam"), "{icerik}");
    }
    // Araç her çalıştırmada onaylanan tarifle yalıtıcıya sorar; alan yalnızca
    // ilkinde hazırlanır.
    assert_eq!(*yalitici.hazirliklar.lock().unwrap(), vec![true; 3]);
    let hazirlama = surucu
        .gecmis
        .iter()
        .filter(|z| {
            matches!(&z.yuk, Olay::AracCiktisiParcasi { metin, .. } if metin.contains("İzinler hazırlanıyor"))
        })
        .count();
    assert_eq!(hazirlama, 1);
    assert_eq!(yalitici.istekler.lock().unwrap().len(), 3);
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
    assert!(
        matches!(t.yalitim, YalitimOzeti::Yalitimli { profil, .. } if profil.starts_with("ozet-true"))
    );

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

#[tokio::test]
async fn ag_hostlari_ag_olmadan_gecersiz() {
    let (_d, arac, _guven) = kur();
    let a = json!({
        "program": "cargo",
        "argumanlar": ["check"],
        "ag": false,
        "ag_hostlari": ["github.com"]
    });
    match arac.hazirla(&a).await {
        Err(AracHatasi::Gecersiz(m)) => {
            assert!(m.contains("ag: true") || m.contains("ağ"), "{m}");
        }
        diger => panic!("ag olmadan ag_hostlari hata vermeliydi: {diger:?}"),
    }
}

#[tokio::test]
async fn on_altidan_fazla_host_gecersiz() {
    let (_d, arac, _guven) = kur();
    let hostlar: Vec<String> = (0..17).map(|i| format!("host{i}.com")).collect();
    let a = json!({
        "program": "cargo",
        "argumanlar": ["check"],
        "ag": true,
        "ag_hostlari": hostlar
    });
    match arac.hazirla(&a).await {
        Err(AracHatasi::Gecersiz(m)) => {
            assert!(m.contains("16"), "{m}");
        }
        diger => panic!("16'dan fazla host hata vermeliydi: {diger:?}"),
    }
}

#[tokio::test]
async fn joker_host_gecersiz() {
    let (_d, arac, _guven) = kur();
    let a = json!({
        "program": "cargo",
        "argumanlar": ["check"],
        "ag": true,
        "ag_hostlari": ["*.github.com"]
    });
    match arac.hazirla(&a).await {
        Err(AracHatasi::Gecersiz(m)) => {
            assert!(m.contains("joker") || m.contains('*'), "{m}");
        }
        diger => panic!("joker host hata vermeliydi: {diger:?}"),
    }
}

#[tokio::test]
async fn tekrarlanan_host_bir_kez() {
    let (_d, arac, _guven) = kur();
    let a = json!({
        "program": "cargo",
        "argumanlar": ["check"],
        "ag": true,
        "ag_hostlari": ["github.com", "GITHUB.COM", "github.com:443"]
    });
    let h = arac.hazirla(&a).await.expect("hazırlık başarılı olmalı");
    let t = h.kabuk.expect("kabuk tarifi olmalı");
    assert_eq!(t.ag_hostlari, vec!["github.com:443"]);
}

#[tokio::test]
async fn reddedilen_baglanti_sonuca_satir_ekler() {
    let (_d, arac, yalitici) = kur_yalitimli();
    yalitici
        .sahte_ag
        .lock()
        .unwrap()
        .replace(kalfa_sandbox::vekil::AgOzeti {
            baglantilar: vec![
                kalfa_sandbox::vekil::AgBaglantisi {
                    host: "index.crates.io".into(),
                    port: 443,
                    karar: kalfa_sandbox::vekil::AgKarari::Izinli,
                    giden: 100,
                    gelen: 200,
                },
                kalfa_sandbox::vekil::AgBaglantisi {
                    host: "evil.com".into(),
                    port: 443,
                    karar: kalfa_sandbox::vekil::AgKarari::ListedeYok,
                    giden: 0,
                    gelen: 0,
                },
                kalfa_sandbox::vekil::AgBaglantisi {
                    host: "yerel.test".into(),
                    port: 80,
                    karar: kalfa_sandbox::vekil::AgKarari::YasakAdres(
                        kalfa_sandbox::vekil::AdresReddi::Yerel,
                    ),
                    giden: 0,
                    gelen: 0,
                },
                kalfa_sandbox::vekil::AgBaglantisi {
                    host: "sni.test".into(),
                    port: 443,
                    karar: kalfa_sandbox::vekil::AgKarari::SniUyusmuyor {
                        sni: Some("baska.test".into()),
                    },
                    giden: 0,
                    gelen: 0,
                },
            ],
        });
    let mut a = yazdir();
    a["ag"] = json!(true);
    let c = calistir_tam(&arac, &a).await.unwrap();
    assert!(
        c.icerik
            .contains("[ağ] `evil.com:443` engellendi (izin listesinde yok)"),
        "{}",
        c.icerik
    );
    assert!(
        c.icerik
            .contains("[ağ] `yerel.test:80` engellendi: özel ya da yerel bir adrese çözüldü"),
        "{}",
        c.icerik
    );
    assert!(
        c.icerik
            .contains("[ağ] `sni.test:443` engellendi: SNI farklı"),
        "{}",
        c.icerik
    );
    assert!(
        !c.icerik.contains("index.crates.io"),
        "izinli host ret satırı almaz"
    );
    let ag = c.ag.expect("ağ komut özeti");
    assert_eq!(ag.reddedilen_sayisi, 3);
    assert_eq!(ag.toplam_giden, 100);
    assert_eq!(ag.toplam_gelen, 200);
}

#[tokio::test]
async fn ayni_ret_tek_satir() {
    let (_d, arac, yalitici) = kur_yalitimli();
    yalitici
        .sahte_ag
        .lock()
        .unwrap()
        .replace(kalfa_sandbox::vekil::AgOzeti {
            baglantilar: vec![
                kalfa_sandbox::vekil::AgBaglantisi {
                    host: "evil.com".into(),
                    port: 443,
                    karar: kalfa_sandbox::vekil::AgKarari::ListedeYok,
                    giden: 0,
                    gelen: 0,
                },
                kalfa_sandbox::vekil::AgBaglantisi {
                    host: "evil.com".into(),
                    port: 443,
                    karar: kalfa_sandbox::vekil::AgKarari::ListedeYok,
                    giden: 0,
                    gelen: 0,
                },
                kalfa_sandbox::vekil::AgBaglantisi {
                    host: "evil.com".into(),
                    port: 443,
                    karar: kalfa_sandbox::vekil::AgKarari::ListedeYok,
                    giden: 0,
                    gelen: 0,
                },
            ],
        });
    let mut a = yazdir();
    a["ag"] = json!(true);
    let c = calistir_tam(&arac, &a).await.unwrap();
    assert!(
        c.icerik
            .contains("[ağ] `evil.com:443` engellendi (izin listesinde yok) (3)"),
        "{}",
        c.icerik
    );
    assert_eq!(
        c.icerik.matches("evil.com").count(),
        1,
        "aynı ret tek satırdır"
    );
}

#[tokio::test]
async fn kurumsal_vekil_notu() {
    let (_d, arac, yalitici) = kur_yalitimli();
    yalitici
        .sahte_ag
        .lock()
        .unwrap()
        .replace(kalfa_sandbox::vekil::AgOzeti {
            baglantilar: vec![
                kalfa_sandbox::vekil::AgBaglantisi {
                    host: "cozulemeyen.com".into(),
                    port: 443,
                    karar: kalfa_sandbox::vekil::AgKarari::Cozulemedi("ad bulunamadı".into()),
                    giden: 0,
                    gelen: 0,
                },
                kalfa_sandbox::vekil::AgBaglantisi {
                    host: "baglanilamayan.com".into(),
                    port: 443,
                    karar: kalfa_sandbox::vekil::AgKarari::Baglanamadi(
                        "bağlantı reddedildi".into(),
                    ),
                    giden: 0,
                    gelen: 0,
                },
            ],
        });
    let mut a = yazdir();
    a["ag"] = json!(true);

    unsafe {
        std::env::remove_var("HTTPS_PROXY");
        std::env::remove_var("https_proxy");
    }
    let c1 = calistir_tam(&arac, &a).await.unwrap();
    assert!(
        c1.icerik.contains("[ağ] `cozulemeyen.com:443` çözülemedi"),
        "{}",
        c1.icerik
    );
    assert!(
        c1.icerik
            .contains("[ağ] `baglanilamayan.com:443` bağlanılamadı"),
        "{}",
        c1.icerik
    );
    assert!(
        !c1.icerik
            .contains("kurumsal vekil bu sürümde desteklenmiyor"),
        "{}",
        c1.icerik
    );

    unsafe {
        std::env::set_var("HTTPS_PROXY", "http://127.0.0.1:8080");
    }
    let c2 = calistir_tam(&arac, &a).await.unwrap();
    unsafe {
        std::env::remove_var("HTTPS_PROXY");
    }
    assert!(
        c2.icerik.contains(
            "[ağ] `cozulemeyen.com:443` çözülemedi (kurumsal vekil bu sürümde desteklenmiyor)"
        ),
        "{}",
        c2.icerik
    );
    assert!(
        c2.icerik.contains(
            "[ağ] `baglanilamayan.com:443` bağlanılamadı (kurumsal vekil bu sürümde desteklenmiyor)"
        ),
        "{}",
        c2.icerik
    );
}

/// 3e Görev 4: kullanıcının ağ izin listesi profil özetine ve vekilin
/// listesine girer. Liste değişince özet de değişir; bekleyen onayın mührü
/// tutmaz (aşama 3 tasarımı §8.3).
#[tokio::test]
async fn kullanici_ag_listesi_profil_ozetine_ve_vekile_girer() {
    use kalfa_sandbox::vekil::{HostKaynagi, HostKurali};
    let d = TempDir::new().unwrap();
    let s = Arc::new(Siniflandirici::yeni(d.path(), &[]).unwrap());
    let alan = CalismaAlani::ac(s).unwrap();
    let yalitici = Arc::new(KayitliYalitici::default());
    let liste = Arc::new(std::sync::RwLock::new(Vec::<HostKurali>::new()));
    let arac = KabukAraci::yeni(
        alan,
        yalitici.clone(),
        Arc::new(SahteGuven::default()),
        Arc::new(AtomicBool::new(true)),
    )
    .kullanici_ag_listesi(liste.clone());
    let agli = if cfg!(windows) {
        json!({"betik": "Write-Output tamam", "ag": true})
    } else {
        json!({"betik": "echo tamam", "ag": true})
    };
    let profil = |h: Hazirlik| match h.kabuk.unwrap().yalitim {
        YalitimOzeti::Yalitimli { profil, .. } => profil,
        diger => panic!("{diger:?}"),
    };
    let once = profil(arac.hazirla(&agli).await.unwrap());
    liste.write().unwrap().push(HostKurali {
        host: "pkg.sirket.local".into(),
        port: 443,
        ozel_ag: true,
        kaynak: HostKaynagi::Kullanici,
    });
    let sonra = profil(arac.hazirla(&agli).await.unwrap());
    assert_ne!(once, sonra, "liste değişince profil özeti değişir");

    calistir_tam(&arac, &agli).await.unwrap();
    let istekler = yalitici.istekler.lock().unwrap();
    let kural = istekler
        .last()
        .and_then(|i| i.ag_listesi.as_ref())
        .and_then(|l| l.bul("pkg.sirket.local", 443).cloned())
        .expect("kullanıcı hostu vekilin listesinde");
    assert!(kural.ozel_ag);
}
