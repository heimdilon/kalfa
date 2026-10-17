//! MCP'nin gerçek yalıtımla kabulü (1.1a-2 Görev 12; tasarım §4.1, §4.3).
//! Tanım ürünün yolundan geçer: `McpTanim::hazirla` → `etkilesimli_istek` →
//! `Yalitici::baslat_etkilesimli` → sınırlı taşıma ve MCP istemcisi. Sunucu
//! depodaki test sunucusunun `sinama` kipidir; dosya ve ağ kararını işletim
//! sistemi verir. Yalıtım yoksa atlanmaz, başarısız olur:
//!
//! ```text
//! cargo build -p kalfa-sandbox --bin kalfa-kopru --locked
//! cargo test -p kalfa-mcp --features yalitim-testleri --test yalitim --locked -- --test-threads=1
//! ```
#![cfg(feature = "yalitim-testleri")]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kalfa_mcp::istemci::McpIstemcisi;
use kalfa_mcp::tanim::{
    AgIzinKurali, Komut, McpTanim, SeciliYurutulebilirKimligi, YazilabilirKokler,
};
use kalfa_mcp::tasima::SinirliTasima;
use kalfa_sandbox::vekil::Cozucu;
use kalfa_sandbox::{
    AlanBilgisi, CalistirmaKipi, EtkilesimliSurec, OzelDizinler, Yalitici, YalitimDurumu,
    alan_kimligi, tarama,
};
use serde_json::{Value, json};

const CANARY: &str = "SONRADAN-GELEN-GIZLI-7c41";

struct Kurgu {
    _gecici: tempfile::TempDir,
    taban: PathBuf,
    ev: PathBuf,
    kok: PathBuf,
    yalitim_dizini: PathBuf,
    exe: PathBuf,
    y: Arc<dyn Yalitici>,
}

struct Calisan {
    surec: EtkilesimliSurec,
    istemci: McpIstemcisi,
    stderr: Arc<Mutex<String>>,
}

/// Testin gizli kuralı: `.env` dosyaları.
fn gizli_mi(yol: &Path, dizin: bool) -> bool {
    !dizin && yol.file_name().and_then(|a| a.to_str()) == Some(".env")
}

/// `target/debug` altındaki yardımcı ikili (`kalfa-kopru`).
fn hedef_ikili(ad: &str) -> PathBuf {
    let ad = if cfg!(windows) {
        format!("{ad}.exe")
    } else {
        ad.to_owned()
    };
    let yol = std::env::current_exe()
        .unwrap()
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join(ad);
    assert!(
        yol.is_file(),
        "önce `cargo build -p kalfa-sandbox --bin kalfa-kopru` çalıştırın: {}",
        yol.display()
    );
    yol
}

fn yalitici(
    ev: &Path,
    yalitim_dizini: &Path,
    cozucu: Option<Arc<dyn Cozucu>>,
) -> Arc<dyn Yalitici> {
    let kopru = Some(hedef_ikili("kalfa-kopru"));
    let yerel_izin = cozucu.is_some();
    #[cfg(target_os = "linux")]
    let y: Arc<dyn Yalitici> = Arc::new(kalfa_sandbox::linux::LinuxYalitici::yeni(
        kalfa_sandbox::linux::LinuxAyari {
            ev: ev.to_path_buf(),
            yalitim_dizini: yalitim_dizini.to_path_buf(),
            ek_okunur: Vec::new(),
            kopru,
            git: Default::default(),
            vekil_cozucu: cozucu,
            vekil_yerel_izin: yerel_izin,
        },
        Box::new(kalfa_sandbox::linux::BwrapAlgilayici),
        Box::new(|_| {}),
    ));
    #[cfg(windows)]
    let y: Arc<dyn Yalitici> = Arc::new(kalfa_sandbox::windows::yalitici::WindowsYalitici::yeni(
        kalfa_sandbox::windows::yalitici::WindowsAyari {
            ev: ev.to_path_buf(),
            yalitim_dizini: yalitim_dizini.to_path_buf(),
            kopru,
            vekil_cozucu: cozucu,
            vekil_yerel_izin: yerel_izin,
            ..Default::default()
        },
        Box::new(|_| {}),
    ));
    match y.durum() {
        YalitimDurumu::Hazir { .. } => {}
        durum => panic!("yalıtım hazır değil: {durum:?}"),
    }
    y
}

fn kurgu(cozucu: Option<Arc<dyn Cozucu>>) -> Kurgu {
    let gecici = tempfile::tempdir().unwrap();
    let taban = gecici.path().to_path_buf();
    let ev = taban.join("ev");
    let kok = taban.join("alan");
    let yalitim_dizini = taban.join("veri").join("yalitim");
    std::fs::create_dir_all(ev.join(".ssh")).unwrap();
    std::fs::write(ev.join(".ssh").join("id_rsa"), "GIZLI-SSH").unwrap();
    std::fs::create_dir_all(&kok).unwrap();
    std::fs::write(kok.join("okunur.txt"), "ALAN-ICERIGI").unwrap();
    // Sunucu ikilisi çalışma alanındadır: yalıtımda yalnız orası ve sistem
    // dizinleri görünür. `fs::copy` Unix çalıştırma iznini korur.
    let exe = kok.join(if cfg!(windows) {
        "sunucu.exe"
    } else {
        "sunucu"
    });
    std::fs::copy(env!("CARGO_BIN_EXE_kalfa-mcp-test-sunucusu"), &exe).unwrap();
    let y = yalitici(&ev, &yalitim_dizini, cozucu);
    Kurgu {
        _gecici: gecici,
        taban,
        ev,
        kok,
        yalitim_dizini,
        exe,
        y,
    }
}

impl Kurgu {
    /// Kalfa'nın her başlatmadan önce yaptığı gibi: gizli tarama ve hazırlık.
    fn alan(&self) -> AlanBilgisi {
        let alan = AlanBilgisi {
            kok: std::fs::canonicalize(&self.kok).unwrap(),
            gizli: tarama::tara(&self.kok, &gizli_mi),
            korumali: Vec::new(),
            git_dizinleri: Vec::new(),
        };
        self.y
            .alani_hazirla(&alan, true, &mut |_| {})
            .unwrap_or_else(|hata| panic!("çalışma alanı hazırlanamadı: {hata:?}"));
        alan
    }

    fn sistem_ortami(&self) -> Vec<(String, String)> {
        if cfg!(windows) {
            let sistem = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
            vec![("PATH".into(), format!(r"{sistem}\System32"))]
        } else {
            vec![("PATH".into(), "/usr/bin:/bin".into())]
        }
    }

    async fn baslat(
        &self,
        alan: &AlanBilgisi,
        ad: &str,
        ag: Option<Vec<AgIzinKurali>>,
        iz: &str,
    ) -> Calisan {
        let tanim = McpTanim::yeni(
            ad.to_owned(),
            Komut::SeciliYurutulebilir {
                kimlik: SeciliYurutulebilirKimligi::yeni(1),
                argumanlar: vec!["sinama".into(), iz.into()],
            },
            Vec::new(),
            CalistirmaKipi::Yalitimli,
            ag,
            20,
            20,
        )
        .unwrap();
        let alan_k = alan_kimligi(&alan.kok);
        let ozel = kalfa_sandbox::sunucu_ozel(&self.yalitim_dizini, &alan_k, ad);
        let kokler = YazilabilirKokler::yeni(
            alan.kok.clone(),
            OzelDizinler::yol(&self.yalitim_dizini, &alan_k),
        );
        let exe = self.exe.clone();
        let cozulmus = tanim
            .hazirla(&self.sistem_ortami(), &ozel, &kokler, &move |_| {
                Ok(exe.clone())
            })
            .unwrap();
        let istek = cozulmus.etkilesimli_istek(alan.clone()).unwrap();
        let mut surec = self
            .y
            .baslat_etkilesimli(&istek)
            .unwrap_or_else(|hata| panic!("yalıtımlı MCP sunucusu başlamadı: {hata}"));
        let stderr = Arc::new(Mutex::new(String::new()));
        if let Some(mut okuyucu) = surec.hata_al() {
            let hedef = Arc::clone(&stderr);
            tokio::spawn(async move {
                let mut m = String::new();
                let _ = tokio::io::AsyncReadExt::read_to_string(&mut okuyucu, &mut m).await;
                hedef.lock().unwrap().push_str(&m);
            });
        }
        let (tasima, hata) =
            SinirliTasima::yeni(surec.cikti_al().unwrap(), surec.girdi_al().unwrap());
        let istemci =
            tokio::time::timeout(Duration::from_secs(20), McpIstemcisi::yeni(tasima, hata))
                .await
                .expect("yalıtımlı sunucuyla el sıkışma zaman aşımı")
                .unwrap_or_else(|hata| {
                    panic!("el sıkışma: {hata}; stderr: {}", stderr.lock().unwrap())
                });
        Calisan {
            surec,
            istemci,
            stderr,
        }
    }

    fn temizle(&self, alan: &AlanBilgisi, sunucular: &[&str]) {
        for ad in sunucular {
            let _ = self.y.sunucu_kokunu_kaldir(&alan.kok, ad);
        }
        let _ = self.y.alani_kaldir_kimlikle(&alan_kimligi(&alan.kok));
    }
}

impl Calisan {
    async fn dene(&self, argumanlar: Value) -> String {
        let Value::Object(argumanlar) = argumanlar else {
            panic!("argümanlar nesne olmalı");
        };
        let sonuc = tokio::time::timeout(
            Duration::from_secs(30),
            self.istemci.arac_cagir("dene", argumanlar),
        )
        .await
        .expect("araç çağrısı zaman aşımı")
        .unwrap();
        serde_json::to_value(&sonuc).unwrap()["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    }

    async fn aciklama(&self) -> String {
        let araclar = self.istemci.araclari_listele().await.unwrap();
        araclar[0].description.as_deref().unwrap_or("").to_owned()
    }

    /// İstemciyi kapatır, süreç ağacının durduğunu doğrular.
    async fn kapat(mut self, iz: &str) {
        self.istemci.kapat().await;
        tokio::time::timeout(Duration::from_secs(30), self.surec.kapat())
            .await
            .expect("yalıtımlı MCP süreci kapanmadı")
            .unwrap();
        assert!(
            !surec_kaldi(iz),
            "kapanıştan sonra izli süreç kaldı; stderr: {}",
            self.stderr.lock().unwrap()
        );
    }
}

/// İz argümanını taşıyan süreç hâlâ var mı? Linux'ta `/proc`, Windows'ta
/// süreç komut satırları.
fn surec_kaldi(iz: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_dir("/proc").unwrap().flatten().any(|g| {
            std::fs::read(g.path().join("cmdline"))
                .map(|c| String::from_utf8_lossy(&c).contains(iz))
                .unwrap_or(false)
        })
    }
    #[cfg(windows)]
    {
        let cikti = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "@(Get-CimInstance Win32_Process | Where-Object {{ $_.CommandLine -like '*{iz}*' -and $_.Name -ne 'powershell.exe' }}).Count"
                ),
            ])
            .output()
            .unwrap();
        String::from_utf8_lossy(&cikti.stdout).trim() != "0"
    }
}

/// Windows: hazırlık kaldırılınca çalışma alanında AppContainer izni kalmaz.
fn artik_yok(alan: &AlanBilgisi) {
    #[cfg(windows)]
    {
        let sddl = kalfa_sandbox::windows::guvenlik::dacl_sddl(&alan.kok).unwrap();
        assert!(
            !sddl.contains("S-1-15-2"),
            "çalışma alanında AppContainer izni kaldı: {sddl}"
        );
    }
    #[cfg(not(windows))]
    let _ = alan;
}

fn iz() -> String {
    format!("kalfa-mcp-iz-{}", std::process::id())
}

/// Plan 1: ağsız yalıtımlı sunucu çalışma alanını okur ve yazar; evdeki
/// sırrı okuyamaz, alan dışına yazamaz, doğrudan ağa çıkamaz. Kapanışta
/// süreç ağacı ve hazırlık izleri kalmaz.
#[tokio::test]
async fn agsiz_sunucu_alan_disina_ve_aga_cikamaz_kapanista_iz_birakmaz() {
    let k = kurgu(None);
    let dinleyici = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = dinleyici.local_addr().unwrap().port();
    let alan = k.alan();
    let iz = iz();
    let s = k.baslat(&alan, "agsiz", None, &iz).await;

    let ic = k.kok.join("okunur.txt");
    assert_eq!(
        s.dene(json!({"islem":"oku","yol":ic})).await,
        "TAMAM:ALAN-ICERIGI"
    );
    let yeni = k.kok.join("yeni.txt");
    assert_eq!(
        s.dene(json!({"islem":"yaz","yol":yeni,"deger":"x"})).await,
        "TAMAM:"
    );
    let sir = k.ev.join(".ssh").join("id_rsa");
    let okuma = s.dene(json!({"islem":"oku","yol":sir})).await;
    assert!(okuma.starts_with("RED:"), "evdeki sır okundu: {okuma}");
    // Sınır ana makinededir: Windows yazmayı reddeder; Linux'ta `/tmp`
    // sunucunun özel geçici dizinidir, yazma oraya düşer ve dışarı çıkmaz.
    let dis = k.taban.join("dis.txt");
    let yazma = s.dene(json!({"islem":"yaz","yol":dis,"deger":"x"})).await;
    assert!(!dis.exists(), "alan dışına yazıldı: {yazma}");
    if cfg!(windows) {
        assert!(yazma.starts_with("RED:"), "{yazma}");
    }
    let ag = s
        .dene(json!({"islem":"dogrudan","hedef":format!("127.0.0.1:{port}")}))
        .await;
    assert!(
        ag.starts_with("RED:"),
        "ağsız sunucu doğrudan bağlandı: {ag}"
    );

    s.kapat(&iz).await;
    k.temizle(&alan, &["agsiz"]);
    artik_yok(&alan);
}

/// Codex 2 ve plan 1: ağ listeli sunucu izinli hedefe vekilden ulaşır;
/// listede olmayan hedef 403 alır; vekili atlayan doğrudan bağlantı kurulamaz.
#[tokio::test]
async fn agli_sunucu_izinli_hedefe_ulasir_yasakli_403_alir() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let dinleyici = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = dinleyici.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut soket, _)) = dinleyici.accept().await {
            // Başlıkların sonuna kadar oku: okunmamış veriyle kapanan
            // soket Windows'ta yanıtı sıfırlar.
            let mut istek = Vec::new();
            let mut tampon = [0_u8; 1024];
            while !istek.windows(4).any(|w| w == b"\r\n\r\n") {
                match soket.read(&mut tampon).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => istek.extend_from_slice(&tampon[..n]),
                }
            }
            let _ = soket
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nSELAM",
                )
                .await;
        }
    });
    let mut eslemeler = std::collections::HashMap::new();
    eslemeler.insert(
        ("izinli.ornek".to_owned(), port),
        vec![std::net::SocketAddr::from(([127, 0, 0, 1], port))],
    );
    let k = kurgu(Some(Arc::new(TestCozucu { eslemeler })));
    let alan = k.alan();
    let iz = iz();
    let kural = AgIzinKurali::yeni(&format!("izinli.ornek:{port}"), false).unwrap();
    let s = k.baslat(&alan, "agli", Some(vec![kural]), &iz).await;

    let izinli = s
        .dene(json!({"islem":"vekil","hedef":format!("izinli.ornek:{port}")}))
        .await;
    assert_eq!(
        izinli,
        "TAMAM:SELAM",
        "stderr: {}",
        s.stderr.lock().unwrap()
    );
    let yasak = s
        .dene(json!({"islem":"vekil","hedef":"yasak.ornek:443"}))
        .await;
    assert!(
        yasak.starts_with("TAMAM:") && yasak.contains("403"),
        "{yasak}"
    );
    let dogrudan = s
        .dene(json!({"islem":"dogrudan","hedef":format!("127.0.0.1:{port}")}))
        .await;
    assert!(dogrudan.starts_with("RED:"), "vekil atlandı: {dogrudan}");

    s.kapat(&iz).await;
    k.temizle(&alan, &["agli"]);
    artik_yok(&alan);
}

/// Plan 3 ve §4.3'ün kabul edilmiş kalan riski: süreç başladıktan sonra
/// gelen gizli dosya çalışan süreçte görünür; içerik hem araç sonucuna hem
/// araç açıklamasına girer ve tanım denetiminden geçer (sır kasasında
/// değildir). Koruma yenilenip yeni süreç başlayınca aynı dosya okunamaz.
/// Kasadaki bilinen sır değil, sonradan gelen dosya içeriği kullanılır.
#[tokio::test]
async fn sonradan_gelen_gizli_dosya_eski_surecte_gorunur_yeni_surecte_gorunmez() {
    let k = kurgu(None);
    let alan = k.alan();
    let iz = iz();
    let s = k.baslat(&alan, "risk", None, &iz).await;
    let env = k.kok.join(".env");
    std::fs::write(&env, CANARY).unwrap();

    // Sonuç yolu: içerik araç sonucuna girer.
    assert_eq!(
        s.dene(json!({"islem":"oku","yol":env})).await,
        format!("TAMAM:{CANARY}")
    );
    // Tanım yolu: içerik açıklamaya girer ve denetimden geçer.
    s.dene(json!({"islem":"aciklamaya_koy","yol":env})).await;
    assert_eq!(s.aciklama().await, CANARY);
    let araclar = s.istemci.araclari_listele().await.unwrap();
    let denetim = kalfa_mcp::kesif::arac_tanimlarini_denetle("risk", araclar, &[]);
    assert_eq!(denetim.tanimlar().len(), 1);
    assert!(denetim.tanimlar()[0].sunum().aciklama.contains(CANARY));
    s.kapat(&iz).await;

    // Yaş dolunca Kalfa korumayı yeniden tarar ve yeni süreç başlatır.
    let yeni_alan = k.alan();
    let s = k.baslat(&yeni_alan, "risk", None, &iz).await;
    let okuma = s.dene(json!({"islem":"oku","yol":env})).await;
    assert!(
        okuma.starts_with("RED:"),
        "korunan dosya yeni süreçte okundu: {okuma}"
    );
    let aciklama = s.dene(json!({"islem":"aciklamaya_koy","yol":env})).await;
    assert!(aciklama.starts_with("RED:"), "{aciklama}");
    assert_eq!(s.aciklama().await, "");
    s.kapat(&iz).await;
    k.temizle(&yeni_alan, &["risk"]);
    artik_yok(&yeni_alan);
}

struct TestCozucu {
    eslemeler: std::collections::HashMap<(String, u16), Vec<std::net::SocketAddr>>,
}

impl Cozucu for TestCozucu {
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
        let sonuc = self
            .eslemeler
            .get(&(host.to_lowercase(), port))
            .cloned()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "bulunamadı"));
        Box::pin(async move { sonuc })
    }
}
