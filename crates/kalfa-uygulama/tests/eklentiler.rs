//! Eklentiler uygulamada (aşama 4 tasarımı §13, §14, §16.2): kayıt,
//! klasörden kurulum ve güncelleme, yerleşik eklenti, kaldırma, çalışma
//! alanında açma, gönderim izni ve oturuma bağlama. Gerçek bir eklenti
//! (`kalfa-test-eklentisi`) `wasm32-wasip2` için derlenir; manifestler testte
//! yazılır.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::Duration;

use kalfa_core::aktarim::KanalMesaji;
use kalfa_core::kimlik::OturumId;
use kalfa_core::muhur::ozet_hex;
use kalfa_core::protokol::{AracDurumu, Olay, OnayKarari, OnayOgesi, OnizlemeNedeni};
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::{SahteSaglayici, SahteYanit};
use kalfa_evren::EvrenError;
use kalfa_plugin::manifest::Manifest;
use kalfa_uygulama::calisma_alanlari::CalismaAlaniKaydi;
use kalfa_uygulama::hata::HataKodu;
use kalfa_uygulama::sozlesme::{
    AlanHedefi, EklentiDurumu, EklentiSatiri, GunlukSinirDurumu, KurulumSonucu, OnayGirdisi,
};
use kalfa_uygulama::test_destek::{SahteKanal, TestUygulamasi, test_uygulamasi_yerlesikli};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::sync::mpsc::UnboundedReceiver;

const KIMLIK: &str = "0123456789abcdef";
const DIGER: &str = "fedcba9876543210";

// --- Test eklentisi ---

/// `kalfa-test-eklentisi`'ni `wasm32-wasip2` için derler (`kalfa-plugin`
/// testlerinin yoluyla); test ikilisi başına bir kez.
fn derle() -> Result<Vec<u8>, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let cikti = Command::new(cargo)
        .args([
            "build",
            "--target",
            "wasm32-wasip2",
            "--locked",
            "--message-format=json-render-diagnostics",
            "-p",
            "kalfa-test-eklentisi",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("cargo çalıştırılamadı: {e}"))?;
    if !cikti.status.success() {
        return Err(format!(
            "Test eklentisi derlenemedi. `rustup target add wasm32-wasip2` gerekebilir. Durum: {}",
            cikti.status
        ));
    }
    let yol = String::from_utf8_lossy(&cikti.stdout)
        .lines()
        .filter_map(|s| serde_json::from_str::<Value>(s).ok())
        .filter(|m| m["reason"] == "compiler-artifact")
        .flat_map(|m| m["filenames"].as_array().cloned().unwrap_or_default())
        .filter_map(|f| f.as_str().map(PathBuf::from))
        .find(|f| {
            f.file_name()
                .is_some_and(|a| a == "kalfa_test_eklentisi.wasm")
        })
        .ok_or("cargo çıktısında kalfa_test_eklentisi.wasm yok")?;
    std::fs::read(&yol).map_err(|e| format!("{} okunamadı: {e}", yol.display()))
}

fn temel_wasm() -> &'static [u8] {
    static WASM: OnceLock<Result<Vec<u8>, String>> = OnceLock::new();
    match WASM.get_or_init(derle) {
        Ok(w) => w,
        Err(e) => panic!("{e}"),
    }
}

/// Aynı bileşen, sonuna bir özel bölüm eklenmiş: geçerli ama farklı SHA-256.
fn wasm_varyanti(n: u8) -> Vec<u8> {
    let mut w = temel_wasm().to_vec();
    w.extend_from_slice(&[0x00, 0x03, 0x01, b'x', n]);
    w
}

/// `ad` adlı eklentinin `kalfa.toml`'u.
fn manifest(ad: &str, surum: &str, dosya_oku: &[&str], evren: &[&str]) -> String {
    let liste = |l: &[&str]| {
        l.iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "[eklenti]\nad = \"{ad}\"\nsurum = \"{surum}\"\nsozlesme = \"0.1\"\naciklama = \"Test eklentisi\"\n\n[yetenekler]\ndosya-oku = [{}]\nevren = [{}]\ndepo-mb = 5\n",
        liste(dosya_oku),
        liste(evren)
    )
}

fn temiz_manifest(ad: &str, surum: &str) -> String {
    manifest(ad, surum, &["**/*.md"], &["embeddings"])
}

// --- Düzenek ---

async fn uygulama(
    veri: &Path,
    yerlesik: Option<PathBuf>,
    senaryo: Vec<SahteYanit>,
) -> TestUygulamasi {
    let saglayici = SahteSaglayici::yeni(senaryo);
    saglayici.sart_kabul(1).await.unwrap();
    let t = test_uygulamasi_yerlesikli(veri, saglayici, Some("evren_llm_a"), yerlesik);
    t.uygulama.baslat().await;
    t
}

struct Ortam {
    veri: TempDir,
    alan: TempDir,
    kaynak: TempDir,
    yerlesik: TempDir,
    yerlesik_var: bool,
    t: TestUygulamasi,
}

impl Ortam {
    async fn yeni() -> Self {
        Self::kur(false, Vec::new()).await
    }

    async fn yerlesikli() -> Self {
        Self::kur(true, Vec::new()).await
    }

    async fn senaryolu(senaryo: Vec<SahteYanit>) -> Self {
        Self::kur(false, senaryo).await
    }

    async fn kur(yerlesik_var: bool, senaryo: Vec<SahteYanit>) -> Self {
        let veri = tempfile::tempdir().unwrap();
        let yerlesik = tempfile::tempdir().unwrap();
        let t = uygulama(
            veri.path(),
            yerlesik_var.then(|| yerlesik.path().to_path_buf()),
            senaryo,
        )
        .await;
        Self {
            veri,
            alan: tempfile::tempdir().unwrap(),
            kaynak: tempfile::tempdir().unwrap(),
            yerlesik,
            yerlesik_var,
            t,
        }
    }

    /// Aynı veri dizininde yeni bir uygulama: süreç yeniden açılmış gibi.
    async fn yeniden(&mut self, senaryo: Vec<SahteYanit>) {
        self.t = uygulama(
            self.veri.path(),
            self.yerlesik_var
                .then(|| self.yerlesik.path().to_path_buf()),
            senaryo,
        )
        .await;
    }

    /// Kurulum klasörünün içeriğini değiştirir.
    fn klasor_yaz(&self, manifest: &str, wasm: &[u8]) {
        for g in std::fs::read_dir(self.kaynak.path()).unwrap() {
            let yol = g.unwrap().path();
            if yol.is_dir() {
                std::fs::remove_dir_all(yol).unwrap();
            } else {
                std::fs::remove_file(yol).unwrap();
            }
        }
        std::fs::write(self.kaynak.path().join("kalfa.toml"), manifest).unwrap();
        std::fs::write(self.kaynak.path().join("ornek.wasm"), wasm).unwrap();
    }

    fn yerlesik_yaz(&self, ad: &str, manifest: &str, wasm: &[u8]) {
        let d = self.yerlesik.path().join(ad);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("kalfa.toml"), manifest).unwrap();
        std::fs::write(d.join("eklenti.wasm"), wasm).unwrap();
    }

    /// "Klasörden kur"; yerel pencere `onay` cevabını verir.
    async fn kur_eklenti(&self, onay: bool) -> KurulumSonucu {
        self.t.iletisim.klasor_ver(self.kaynak.path());
        self.t.iletisim.onay_ver(onay);
        self.t.uygulama.eklenti_klasorden_kur().await.unwrap()
    }

    /// Onaylanan kurulum; başarısızsa test düşer.
    async fn kur_ve_dogrula(&self, ad: &str, surum: &str) {
        match self.kur_eklenti(true).await {
            KurulumSonucu::Kuruldu {
                ad: a, surum: s, ..
            } => assert_eq!((a.as_str(), s.as_str()), (ad, surum)),
            diger => panic!("kurulamadı: {diger:?}"),
        }
    }

    async fn liste(&self) -> Vec<EklentiSatiri> {
        self.t.uygulama.eklentileri_listele().await.unwrap()
    }

    async fn satir(&self, ad: &str) -> EklentiSatiri {
        self.liste()
            .await
            .into_iter()
            .find(|s| s.ad == ad)
            .unwrap_or_else(|| panic!("{ad} listede yok"))
    }

    fn son_soru(&self) -> String {
        self.t.iletisim.sorulanlar().pop().unwrap()
    }

    fn kayit(&self) -> Value {
        kayit_oku(self.veri.path())
    }

    fn kayit_degistir(&self, is: impl FnOnce(&mut Value)) {
        let mut k = self.kayit();
        is(&mut k);
        std::fs::write(
            self.veri.path().join("eklentiler.json"),
            serde_json::to_vec_pretty(&k).unwrap(),
        )
        .unwrap();
    }

    fn kurulu_dizin(&self, ad: &str) -> PathBuf {
        self.veri.path().join("eklentiler").join(ad)
    }

    fn depo_dizini(&self, ad: &str, kimlik: &str) -> PathBuf {
        self.veri
            .path()
            .join("eklenti-verisi")
            .join(ad)
            .join(kimlik)
    }

    /// Depoya bir değer koyar; boyutu 5 bayttır.
    fn depo_yaz(&self, ad: &str, kimlik: &str) {
        let d = self.depo_dizini(ad, kimlik);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("anahtar"), "deger").unwrap();
    }

    fn alan_kimligi(&self) -> String {
        CalismaAlaniKaydi::kimlik(self.alan.path())
    }

    async fn oturum(&self) -> (OturumId, UnboundedReceiver<KanalMesaji>) {
        self.t.iletisim.klasor_ver(self.alan.path());
        let alan = self.t.uygulama.calisma_alani_sec().await.unwrap();
        let oturum = self
            .t
            .uygulama
            .oturum_baslat(alan.kimlik, "glm-5.3".into())
            .await
            .unwrap()
            .oturum;
        let (kanal, rx) = SahteKanal::yeni();
        self.t.uygulama.oturumu_dinle(oturum, kanal).unwrap();
        (oturum, rx)
    }

    async fn bekle(
        &self,
        oturum: OturumId,
        rx: &mut UnboundedReceiver<KanalMesaji>,
        kosul: impl Fn(&Olay) -> bool,
    ) -> Vec<Olay> {
        let mut gorulen = Vec::new();
        loop {
            let mesaj = tokio::time::timeout(Duration::from_secs(20), rx.recv())
                .await
                .expect("olay gelmedi")
                .expect("kanal kapandı");
            let KanalMesaji::Olaylar { no, zarflar } = mesaj else {
                panic!("kanal kapandı");
            };
            self.t.uygulama.paketi_isledim(oturum, no).unwrap();
            for z in zarflar {
                let bulundu = kosul(&z.yuk);
                gorulen.push(z.yuk);
                if bulundu {
                    return gorulen;
                }
            }
        }
    }

    /// Bir mesaj gönderir ve turun bitmesini bekler.
    async fn tur(
        &self,
        oturum: OturumId,
        rx: &mut UnboundedReceiver<KanalMesaji>,
        metin: &str,
    ) -> Vec<Olay> {
        self.t
            .uygulama
            .mesaj_gonder(oturum, metin.into())
            .await
            .unwrap();
        self.bekle(oturum, rx, |o| matches!(o, Olay::Tamamlandi { .. }))
            .await
    }

    /// Sağlayıcıya giden `n`'inci (0'dan) isteğin araç adları.
    fn istek_araclari(&self, n: usize) -> Vec<String> {
        self.t.saglayici.istekler()[n]
            .tools
            .iter()
            .map(|t| t.function.name.clone())
            .collect()
    }
}

fn kayit_oku(veri: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(veri.join("eklentiler.json")).unwrap()).unwrap()
}

/// Bir eklenti klasörünün düzeni: (dosya adı, içerik) çiftleri.
type KlasorDuzeni<'a> = Vec<(&'a str, Vec<u8>)>;

fn basarisiz(s: KurulumSonucu) -> String {
    match s {
        KurulumSonucu::Basarisiz { mesaj } => mesaj,
        diger => panic!("başarısız olmalıydı: {diger:?}"),
    }
}

fn arac_adi(s: &EklentiSatiri, ad: &str) -> bool {
    s.araclar.iter().any(|a| a.ad == ad)
}

// --- Klasörden kurulum ---

#[tokio::test]
async fn klasor_secilmezse_vazgecilir() {
    let o = Ortam::yeni().await;
    assert_eq!(
        o.t.uygulama.eklenti_klasorden_kur().await.unwrap(),
        KurulumSonucu::Vazgecildi
    );
    assert!(o.t.iletisim.sorulanlar().is_empty());
    assert!(o.liste().await.is_empty());
}

#[tokio::test]
async fn kurulum_yerel_onayda_vazgecilirse_hicbir_iz_kalmaz() {
    let o = Ortam::yeni().await;
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());

    assert_eq!(o.kur_eklenti(false).await, KurulumSonucu::Vazgecildi);

    let sorular = o.t.iletisim.sorulanlar();
    assert_eq!(sorular.len(), 1, "{sorular:?}");
    assert!(sorular[0].starts_with("Eklentiyi kur: "), "{}", sorular[0]);
    assert!(!o.veri.path().join("eklentiler").exists());
    assert!(!o.veri.path().join("eklentiler.json").exists());
    assert!(o.liste().await.is_empty());
}

#[tokio::test]
async fn onaylanan_kurulum_dosyayi_kaydi_ve_listeyi_olusturur() {
    let o = Ortam::yeni().await;
    let metin = temiz_manifest("ornek", "0.1.0");
    o.klasor_yaz(&metin, temel_wasm());
    let sha = ozet_hex(temel_wasm());

    assert_eq!(
        o.kur_eklenti(true).await,
        KurulumSonucu::Kuruldu {
            ad: "ornek".into(),
            surum: "0.1.0".into(),
            guncelleme: false,
            izinler_silindi: false,
        }
    );

    // Onay penceresi: ad, SHA-256, yetenek cümlesi, kaynak, araçlar.
    let m = Manifest::coz(&metin).unwrap();
    let soru = o.son_soru();
    for beklenen in [
        "Ad: ornek",
        "Sürüm: 0.1.0\n",
        "Açıklama: Test eklentisi",
        &format!("SHA-256: {sha}"),
        &format!("Kaynak klasör: {}", o.kaynak.path().display()),
        &m.yetenek_cumlesi(),
        "ornek__yanki",
    ] {
        assert!(soru.contains(beklenen), "{beklenen:?} yok:\n{soru}");
    }
    assert!(!soru.contains("Yeni yetenek"), "{soru}");

    // Dosyalar: doğrulanan baytların aynısı; geçici dizin kalmaz.
    let kurulu = o.kurulu_dizin("ornek");
    assert_eq!(
        std::fs::read_to_string(kurulu.join("kalfa.toml")).unwrap(),
        metin
    );
    assert_eq!(
        std::fs::read(kurulu.join("eklenti.wasm")).unwrap(),
        temel_wasm()
    );
    assert!(!o.kurulu_dizin("ornek.yeni").exists());

    // Kayıt belgedeki biçimde.
    let kayit = o.kayit();
    assert_eq!(kayit["surum"], 1);
    let e = &kayit["eklentiler"][0];
    assert_eq!(kayit["eklentiler"].as_array().unwrap().len(), 1);
    assert_eq!(e["ad"], "ornek");
    assert_eq!(e["surum"], "0.1.0");
    assert_eq!(e["sha256"], sha.as_str());
    assert_eq!(e["kaynak"], o.kaynak.path().display().to_string());
    assert_eq!(e["etkin"], true);
    assert!(e["kurulum_ms"].as_u64().is_some_and(|ms| ms > 0));
    assert_eq!(e["calisma_alanlari"], json!({}));
    assert!(!o.veri.path().join("eklentiler.json.yeni").exists());

    // Liste.
    let liste = o.liste().await;
    assert_eq!(liste.len(), 1);
    let s = &liste[0];
    assert_eq!(s.ad, "ornek");
    assert_eq!(s.surum, "0.1.0");
    assert_eq!(s.aciklama, "Test eklentisi");
    assert!(!s.yerlesik);
    assert_eq!(s.durum, EklentiDurumu::Yuklendi);
    assert_eq!(s.sha256, sha);
    assert_eq!(s.kaynak.as_deref(), Some(o.kaynak.path().to_str().unwrap()));
    assert!(s.kurulum_ms.is_some_and(|ms| ms > 0));
    assert_eq!(s.yetenek.as_deref(), Some(m.yetenek_cumlesi().as_str()));
    assert!(arac_adi(s, "ornek__yanki") && arac_adi(s, "ornek__evren"));
    assert!(s.araclar.iter().all(|a| a.ad.starts_with("ornek__")));
    assert!(s.alanlar.is_empty());

    // Kurulu sürüm kaynak klasör sonradan değişse de değişmez.
    o.klasor_yaz("bozuk", b"bozuk");
    assert_eq!(o.satir("ornek").await.durum, EklentiDurumu::Yuklendi);
}

#[tokio::test]
async fn kurulum_yeniden_acilista_da_yuklenir() {
    let mut o = Ortam::yeni().await;
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("ornek", "0.1.0").await;
    o.yeniden(Vec::new()).await;
    let s = o.satir("ornek").await;
    assert_eq!(s.durum, EklentiDurumu::Yuklendi);
    assert_eq!(s.surum, "0.1.0");
}

#[tokio::test]
async fn hatali_klasor_basarisiz_olur_ve_iz_birakmaz() {
    let o = Ortam::yeni().await;
    let gecerli = temiz_manifest("ornek", "0.1.0");
    let cekirdek_modul = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

    // (durum, klasördeki dosyalar, iletide aranan metin)
    let durumlar: Vec<(&str, KlasorDuzeni<'_>, &str)> = vec![
        ("boş klasör", vec![], "`kalfa.toml` yok"),
        (
            "manifestsiz",
            vec![("ornek.wasm", temel_wasm().to_vec())],
            "`kalfa.toml` yok",
        ),
        (
            "wasmsız",
            vec![("kalfa.toml", gecerli.clone().into_bytes())],
            "`.wasm` dosyası yok",
        ),
        (
            "iki wasm",
            vec![
                ("kalfa.toml", gecerli.clone().into_bytes()),
                ("a.wasm", temel_wasm().to_vec()),
                ("b.wasm", temel_wasm().to_vec()),
            ],
            "2 `.wasm`",
        ),
        (
            "bozuk manifest",
            vec![
                ("kalfa.toml", b"[eklenti".to_vec()),
                ("a.wasm", temel_wasm().to_vec()),
            ],
            "kalfa.toml okunamadı",
        ),
        (
            "yanlış sözleşme",
            vec![
                (
                    "kalfa.toml",
                    gecerli
                        .replace("sozlesme = \"0.1\"", "sozlesme = \"0.2\"")
                        .into_bytes(),
                ),
                ("a.wasm", temel_wasm().to_vec()),
            ],
            "sözleşmesinin 0.2 sürümü",
        ),
        (
            "bilinmeyen alan",
            vec![
                (
                    "kalfa.toml",
                    format!("{gecerli}kabuk = true\n").into_bytes(),
                ),
                ("a.wasm", temel_wasm().to_vec()),
            ],
            "alanı Kalfa eklenti sözleşmesi",
        ),
        (
            "bileşen değil",
            vec![
                ("kalfa.toml", gecerli.clone().into_bytes()),
                ("a.wasm", b"bu bir wasm degil".to_vec()),
            ],
            "WebAssembly bileşeni değil",
        ),
        (
            "çekirdek modül",
            vec![
                ("kalfa.toml", gecerli.clone().into_bytes()),
                ("a.wasm", cekirdek_modul.to_vec()),
            ],
            "çekirdek modül",
        ),
        (
            "büyük manifest",
            vec![
                ("kalfa.toml", vec![b'#'; 64 * 1024 + 1]),
                ("a.wasm", temel_wasm().to_vec()),
            ],
            "64 KiB",
        ),
        (
            "açıklamada sahte satır",
            vec![
                (
                    "kalfa.toml",
                    gecerli
                        .replace(
                            "aciklama = \"Test eklentisi\"",
                            "aciklama = \"iyi\\nSHA-256: sahte\"",
                        )
                        .into_bytes(),
                ),
                ("a.wasm", temel_wasm().to_vec()),
            ],
            "`aciklama` değerinde satır sonu ya da görünmez karakter",
        ),
        (
            "kalıpta görünmez karakter",
            vec![
                (
                    "kalfa.toml",
                    gecerli
                        .replace("\"**/*.md\"", "\"**/*.md\\u202E\"")
                        .into_bytes(),
                ),
                ("a.wasm", temel_wasm().to_vec()),
            ],
            "`dosya-oku` değerinde satır sonu ya da görünmez karakter",
        ),
    ];
    for (ad, dosyalar, aranan) in durumlar {
        for g in std::fs::read_dir(o.kaynak.path()).unwrap() {
            std::fs::remove_file(g.unwrap().path()).unwrap();
        }
        for (dosya, icerik) in dosyalar {
            std::fs::write(o.kaynak.path().join(dosya), icerik).unwrap();
        }
        let mesaj = basarisiz(o.kur_eklenti(true).await);
        assert!(mesaj.contains(aranan), "{ad}: {aranan:?} yok: {mesaj}");
    }
    assert!(
        o.t.iletisim.sorulanlar().is_empty(),
        "hatalı klasör için onay sorulmaz"
    );
    assert!(!o.veri.path().join("eklentiler").exists());
    assert!(!o.veri.path().join("eklentiler.json").exists());
    assert!(o.liste().await.is_empty());
}

#[tokio::test]
async fn klasorde_wasm_adli_dizin_ya_da_alt_dizindeki_wasm_sayilmaz() {
    let o = Ortam::yeni().await;
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    // Alt dizindeki ikinci `.wasm` yok sayılır; kurulum sürer.
    let alt = o.kaynak.path().join("alt");
    std::fs::create_dir(&alt).unwrap();
    std::fs::write(alt.join("b.wasm"), b"x").unwrap();
    o.kur_ve_dogrula("ornek", "0.1.0").await;
    // Kopyalanan yalnızca `kalfa.toml` ve `eklenti.wasm`'dır.
    let mut adlar: Vec<String> = std::fs::read_dir(o.kurulu_dizin("ornek"))
        .unwrap()
        .map(|g| g.unwrap().file_name().into_string().unwrap())
        .collect();
    adlar.sort();
    assert_eq!(adlar, vec!["eklenti.wasm", "kalfa.toml"]);

    // `.wasm` adlı bir dizin sıradan dosya değildir.
    let o2 = Ortam::yeni().await;
    std::fs::write(
        o2.kaynak.path().join("kalfa.toml"),
        temiz_manifest("ornek", "0.1.0"),
    )
    .unwrap();
    std::fs::create_dir(o2.kaynak.path().join("a.wasm")).unwrap();
    let mesaj = basarisiz(o2.kur_eklenti(true).await);
    assert!(mesaj.contains("sıradan bir dosya"), "{mesaj}");
}

// --- Güncelleme ---

#[tokio::test]
async fn guncelleme_izinleri_yalnizca_kapsam_degisince_siler() {
    let mut o = Ortam::yeni().await;
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("ornek", "0.1.0").await;
    let ilk_kurulum = o.kayit()["eklentiler"][0]["kurulum_ms"].as_u64().unwrap();
    o.kayit_degistir(|k| {
        k["eklentiler"][0]["etkin"] = json!(false);
        k["eklentiler"][0]["calisma_alanlari"] = json!({
            KIMLIK: {"yol": "C:\\a", "acik": true, "gonderim_izni_ms": 123},
            DIGER: {"yol": "C:\\b", "acik": false},
        });
    });
    o.depo_yaz("ornek", KIMLIK);
    o.yeniden(Vec::new()).await;
    assert_eq!(o.satir("ornek").await.durum, EklentiDurumu::DevreDisi);

    // 1) Aynı kapsam, yeni sürüm ve farklı bileşen: izinler korunur.
    o.klasor_yaz(&temiz_manifest("ornek", "0.2.0"), &wasm_varyanti(1));
    assert_eq!(
        o.kur_eklenti(true).await,
        KurulumSonucu::Kuruldu {
            ad: "ornek".into(),
            surum: "0.2.0".into(),
            guncelleme: true,
            izinler_silindi: false,
        }
    );
    let soru = o.son_soru();
    assert!(soru.starts_with("Eklentiyi güncelle: "), "{soru}");
    assert!(soru.contains("Sürüm: 0.1.0 → 0.2.0\n"), "{soru}");
    assert!(soru.contains("Yeni yetenek yok."), "{soru}");
    assert!(!soru.contains("silinecek"), "{soru}");
    let e = &o.kayit()["eklentiler"][0];
    assert_eq!(e["surum"], "0.2.0");
    assert_eq!(e["sha256"], ozet_hex(&wasm_varyanti(1)).as_str());
    assert_eq!(e["etkin"], false, "devre dışı ayarı korunur");
    assert!(e["kurulum_ms"].as_u64().unwrap() >= ilk_kurulum);
    assert_eq!(e["calisma_alanlari"][KIMLIK]["gonderim_izni_ms"], 123);
    assert_eq!(e["calisma_alanlari"][KIMLIK]["acik"], true);
    assert_eq!(e["calisma_alanlari"][DIGER]["acik"], false);
    assert!(o.depo_dizini("ornek", KIMLIK).join("anahtar").is_file());
    assert_eq!(
        std::fs::read(o.kurulu_dizin("ornek").join("eklenti.wasm")).unwrap(),
        wasm_varyanti(1)
    );
    let s = o.satir("ornek").await;
    assert_eq!(s.surum, "0.2.0");
    assert_eq!(s.durum, EklentiDurumu::DevreDisi);
    assert_eq!(s.alanlar.len(), 2);
    assert_eq!(s.alanlar[0].kimlik, KIMLIK);
    assert_eq!(s.alanlar[0].gonderim_izni_ms, Some(123));
    assert_eq!(s.alanlar[0].depo_bayt, 5, "depo boyutu diskten sayılır");
    assert_eq!(s.alanlar[1].depo_bayt, 0);

    // 2) Yeni glob: yeni yetenek listelenir, bütün izinler silinir; `acik` ve
    // depo korunur.
    o.klasor_yaz(
        &manifest("ornek", "0.3.0", &["**/*.md", "**/*.py"], &["embeddings"]),
        &wasm_varyanti(2),
    );
    assert_eq!(
        o.kur_eklenti(true).await,
        KurulumSonucu::Kuruldu {
            ad: "ornek".into(),
            surum: "0.3.0".into(),
            guncelleme: true,
            izinler_silindi: true,
        }
    );
    let soru = o.son_soru();
    assert!(
        soru.contains("Yeni yetenekler:\n- `**/*.py` ile eşleşen dosyaları okuma"),
        "{soru}"
    );
    assert!(
        soru.contains("Bütün çalışma alanlarındaki EVREN gönderim izinleri silinecek."),
        "{soru}"
    );
    let e = &o.kayit()["eklentiler"][0];
    assert_eq!(e["surum"], "0.3.0");
    assert!(
        e["calisma_alanlari"][KIMLIK]
            .get("gonderim_izni_ms")
            .is_none()
    );
    assert_eq!(e["calisma_alanlari"][KIMLIK]["acik"], true);
    assert!(o.depo_dizini("ornek", KIMLIK).join("anahtar").is_file());

    // 3) Eski sürüme dönüş (kurulu kapsamla aynı): "eski sürüm" işaretlenir.
    o.klasor_yaz(
        &manifest("ornek", "0.2.0", &["**/*.md", "**/*.py"], &["embeddings"]),
        &wasm_varyanti(1),
    );
    assert_eq!(
        o.kur_eklenti(true).await,
        KurulumSonucu::Kuruldu {
            ad: "ornek".into(),
            surum: "0.2.0".into(),
            guncelleme: true,
            izinler_silindi: false,
        }
    );
    let soru = o.son_soru();
    assert!(soru.contains("Sürüm: 0.3.0 → 0.2.0 (eski sürüm)"), "{soru}");
}

#[tokio::test]
async fn guncellemede_vazgecilirse_kurulu_surum_aynen_kalir() {
    let o = Ortam::yeni().await;
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("ornek", "0.1.0").await;
    let once = o.kayit();

    o.klasor_yaz(&temiz_manifest("ornek", "0.2.0"), &wasm_varyanti(1));
    assert_eq!(o.kur_eklenti(false).await, KurulumSonucu::Vazgecildi);
    assert_eq!(o.kayit(), once);
    assert_eq!(
        std::fs::read(o.kurulu_dizin("ornek").join("eklenti.wasm")).unwrap(),
        temel_wasm()
    );
    assert_eq!(o.satir("ornek").await.surum, "0.1.0");
}

#[tokio::test]
async fn kurulumdan_sonra_degisen_wasm_yuklenemez_yeniden_kurulunca_yuklenir() {
    let o = Ortam::yeni().await;
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("ornek", "0.1.0").await;

    let yol = o.kurulu_dizin("ornek").join("eklenti.wasm");
    std::fs::write(&yol, wasm_varyanti(9)).unwrap();
    let s = o.satir("ornek").await;
    assert_eq!(
        s.durum,
        EklentiDurumu::Yuklenemedi {
            neden: "Eklentinin dosyası kurulumdan sonra değişmiş; eklentiyi yeniden kurun.".into()
        }
    );
    assert_eq!(
        s.sha256,
        ozet_hex(temel_wasm()),
        "onaylanan özet gösterilir"
    );
    assert!(s.araclar.is_empty());
    assert!(s.yetenek.is_some(), "manifest okunabildi");

    // Dosya kaybolursa da yüklenemedi görünür; kayıt kalır.
    std::fs::remove_file(&yol).unwrap();
    let EklentiDurumu::Yuklenemedi { neden } = o.satir("ornek").await.durum else {
        panic!("yüklenmemeliydi");
    };
    assert!(
        neden.contains("okunamadı") && neden.contains("yeniden kurun"),
        "{neden}"
    );

    // Aynı klasörle yeniden kurmak onarır.
    assert_eq!(
        o.kur_eklenti(true).await,
        KurulumSonucu::Kuruldu {
            ad: "ornek".into(),
            surum: "0.1.0".into(),
            guncelleme: true,
            izinler_silindi: false,
        }
    );
    assert_eq!(o.satir("ornek").await.durum, EklentiDurumu::Yuklendi);
}

#[tokio::test]
async fn kayit_yazilamazsa_yeni_kurulumun_dosyalari_birakilmaz() {
    let o = Ortam::yeni().await;
    // `eklentiler.json` bir dizin: yeniden adlandırma başarısız olur.
    std::fs::create_dir(o.veri.path().join("eklentiler.json")).unwrap();
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.t.iletisim.klasor_ver(o.kaynak.path());
    o.t.iletisim.onay_ver(true);
    let hata = o.t.uygulama.eklenti_klasorden_kur().await.unwrap_err();
    assert_eq!(hata.kod, HataKodu::Ic);
    assert!(hata.mesaj.contains("eklentiler.json"), "{}", hata.mesaj);
    assert!(!o.kurulu_dizin("ornek").exists());
    assert!(o.liste().await.is_empty());
}

// --- Yerleşik eklenti ---

#[tokio::test]
async fn yerlesik_eklenti_listelenir_kaldirilamaz_ve_adi_alinamaz() {
    let o = Ortam::yerlesikli().await;
    o.yerlesik_yaz(
        "kalfa-rag",
        &temiz_manifest("kalfa-rag", "0.1.0"),
        temel_wasm(),
    );
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("ornek", "0.1.0").await;

    let liste = o.liste().await;
    let adlar: Vec<&str> = liste.iter().map(|s| s.ad.as_str()).collect();
    assert_eq!(adlar, vec!["kalfa-rag", "ornek"], "yerleşikler önce");
    let y = &liste[0];
    assert!(y.yerlesik && !liste[1].yerlesik);
    assert_eq!(y.durum, EklentiDurumu::Yuklendi);
    assert_eq!(y.kaynak, None);
    assert_eq!(y.kurulum_ms, None);
    assert_eq!(y.surum, "0.1.0");
    assert_eq!(y.sha256, ozet_hex(temel_wasm()));
    assert!(
        y.alanlar.is_empty(),
        "varsayılan olarak hiçbir alanda açık değil"
    );
    assert!(arac_adi(y, "kalfa_rag__yanki"));

    // Kaldırılamaz; yerel pencere açılmaz.
    let once = o.t.iletisim.sorulanlar().len();
    let hata =
        o.t.uygulama
            .eklenti_kaldir("kalfa-rag".into())
            .await
            .unwrap_err();
    assert_eq!(hata.kod, HataKodu::GecersizGirdi);
    assert_eq!(
        hata.mesaj,
        "Kalfa'yla gelen eklenti kaldırılamaz; devre dışı bırakabilirsiniz."
    );
    assert_eq!(o.t.iletisim.sorulanlar().len(), once);

    // Aynı adla kurulum reddedilir; onay sorulmaz.
    o.klasor_yaz(&temiz_manifest("kalfa-rag", "9.9.9"), &wasm_varyanti(3));
    let mesaj = basarisiz(o.kur_eklenti(true).await);
    assert!(
        mesaj.contains("Kalfa'yla gelen bir eklentiyle aynı adı taşıyor"),
        "{mesaj}"
    );
    assert_eq!(o.t.iletisim.sorulanlar().len(), once);
    assert!(!o.kurulu_dizin("kalfa-rag").exists());

    // Devre dışı bırakılabilir ve geri açılabilir.
    o.t.uygulama
        .eklenti_etkin_ayarla("kalfa-rag".into(), false)
        .await
        .unwrap();
    assert_eq!(o.satir("kalfa-rag").await.durum, EklentiDurumu::DevreDisi);
    o.t.uygulama
        .eklenti_etkin_ayarla("kalfa-rag".into(), true)
        .await
        .unwrap();
    assert_eq!(o.satir("kalfa-rag").await.durum, EklentiDurumu::Yuklendi);
}

#[tokio::test]
async fn yerlesik_manifest_kapsami_degisince_izinler_silinir_degismezse_kalir() {
    let mut o = Ortam::yerlesikli().await;
    let m1 = temiz_manifest("kalfa-rag", "0.1.0");
    o.yerlesik_yaz("kalfa-rag", &m1, temel_wasm());
    o.liste().await;
    assert_eq!(
        o.kayit()["yerlesik"]["kalfa-rag"]["manifest"],
        m1.as_str(),
        "görülen manifest saklanır"
    );
    assert_eq!(o.kayit()["yerlesik"]["kalfa-rag"]["etkin"], true);

    o.kayit_degistir(|k| {
        k["yerlesik"]["kalfa-rag"]["calisma_alanlari"] = json!({
            KIMLIK: {"yol": "C:\\a", "acik": true, "gonderim_izni_ms": 77},
        });
    });
    o.yeniden(Vec::new()).await;
    assert_eq!(
        o.satir("kalfa-rag").await.alanlar[0].gonderim_izni_ms,
        Some(77)
    );

    // Aynı kapsam, yeni sürüm: izin kalır, saklanan metin güncellenir.
    let m2 = temiz_manifest("kalfa-rag", "0.2.0");
    o.yerlesik_yaz("kalfa-rag", &m2, temel_wasm());
    let s = o.satir("kalfa-rag").await;
    assert_eq!(s.surum, "0.2.0");
    assert_eq!(s.alanlar[0].gonderim_izni_ms, Some(77));
    assert_eq!(o.kayit()["yerlesik"]["kalfa-rag"]["manifest"], m2.as_str());

    // Yeni EVREN ucu: bütün gönderim izinleri silinir, çalışma alanı açık kalır.
    let m3 = manifest(
        "kalfa-rag",
        "0.3.0",
        &["**/*.md"],
        &["embeddings", "rerank"],
    );
    o.yerlesik_yaz("kalfa-rag", &m3, temel_wasm());
    let s = o.satir("kalfa-rag").await;
    assert_eq!(s.surum, "0.3.0");
    assert_eq!(s.alanlar[0].gonderim_izni_ms, None);
    assert!(s.alanlar[0].acik);
    let alan = &o.kayit()["yerlesik"]["kalfa-rag"]["calisma_alanlari"][KIMLIK];
    assert!(alan.get("gonderim_izni_ms").is_none());
    assert_eq!(o.kayit()["yerlesik"]["kalfa-rag"]["manifest"], m3.as_str());

    // Değişiklik yeniden açılışta da kalıcıdır.
    o.yeniden(Vec::new()).await;
    assert_eq!(o.satir("kalfa-rag").await.alanlar[0].gonderim_izni_ms, None);
}

#[tokio::test]
async fn bozuk_yerlesik_eklenti_yuklenemedi_gorunur() {
    let o = Ortam::yerlesikli().await;
    let d = o.yerlesik.path().join("kalfa-rag");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("kalfa.toml"), temiz_manifest("kalfa-rag", "0.1.0")).unwrap();
    let EklentiDurumu::Yuklenemedi { neden } = o.satir("kalfa-rag").await.durum else {
        panic!("yüklenmemeliydi");
    };
    assert!(
        neden.contains("eklenti.wasm") && neden.contains("Kalfa'yı yeniden kurun"),
        "{neden}"
    );

    // Dizin adıyla manifestteki ad uyuşmuyorsa da yüklenmez.
    o.yerlesik_yaz(
        "kalfa-rag",
        &temiz_manifest("baska-ad", "0.1.0"),
        temel_wasm(),
    );
    let EklentiDurumu::Yuklenemedi { neden } = o.satir("kalfa-rag").await.durum else {
        panic!("yüklenmemeliydi");
    };
    assert!(neden.contains("uyuşmuyor"), "{neden}");
}

// --- Kayıt dosyası ---

#[tokio::test]
async fn bozuk_kayit_girdisi_ote_eklentileri_dusurmez() {
    let mut o = Ortam::yeni().await;
    o.klasor_yaz(&temiz_manifest("iyi", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("iyi", "0.1.0").await;
    o.kayit_degistir(|k| {
        let eklentiler = k["eklentiler"].as_array_mut().unwrap();
        eklentiler.push(json!({"ad": "eksik-alan", "surum": "1.0.0"}));
        eklentiler.push(
            json!({"ad": "../x", "surum": "1.0.0", "sha256": "a", "kaynak": "k", "kurulum_ms": 1}),
        );
        eklentiler.push(json!("metin"));
        k["yerlesik"] = json!({"bozuk-yerlesik": {"etkin": "belki"}});
    });
    o.yeniden(Vec::new()).await;

    let liste = o.liste().await;
    let adlar: Vec<&str> = liste.iter().map(|s| s.ad.as_str()).collect();
    assert_eq!(
        adlar,
        vec!["eksik-alan", "iyi"],
        "adı okunamayanlar görünmez"
    );
    assert_eq!(liste[1].durum, EklentiDurumu::Yuklendi);
    let EklentiDurumu::Yuklenemedi { neden } = &liste[0].durum else {
        panic!("yüklenmemeliydi");
    };
    assert!(
        neden.contains("kayıt girdisi bozuk") && neden.contains("yeniden kurun"),
        "{neden}"
    );
    assert!(!liste[0].yerlesik);

    // Ayar değişikliği bozuk girdileri dosyadan silmez.
    o.t.uygulama
        .eklenti_etkin_ayarla("iyi".into(), false)
        .await
        .unwrap();
    let kayit = o.kayit();
    let eklentiler = kayit["eklentiler"].as_array().unwrap();
    assert_eq!(eklentiler.len(), 4, "{eklentiler:?}");
    assert_eq!(eklentiler[0]["etkin"], false);
    assert_eq!(eklentiler[3], json!("metin"));

    // Bozuk girdinin ayarı değiştirilemez; kaldırılabilir.
    let hata =
        o.t.uygulama
            .eklenti_etkin_ayarla("eksik-alan".into(), true)
            .await
            .unwrap_err();
    assert!(hata.mesaj.contains("kaydı bozuk"), "{}", hata.mesaj);
    o.t.iletisim.onay_ver(true);
    assert!(
        o.t.uygulama
            .eklenti_kaldir("eksik-alan".into())
            .await
            .unwrap()
    );
    assert_eq!(o.kayit()["eklentiler"].as_array().unwrap().len(), 3);
    assert_eq!(o.liste().await.len(), 1);
}

#[tokio::test]
async fn bozuk_girdinin_adiyla_kurulum_girdiyi_degistirir() {
    let mut o = Ortam::yeni().await;
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("ornek", "0.1.0").await;
    o.kayit_degistir(|k| k["eklentiler"][0]["sha256"] = json!(42));
    o.yeniden(Vec::new()).await;
    assert!(matches!(
        o.satir("ornek").await.durum,
        EklentiDurumu::Yuklenemedi { .. }
    ));

    // Onarım: aynı klasörle yeniden kurmak; bozuk girdi yerine yenisi gelir.
    assert_eq!(
        o.kur_eklenti(true).await,
        KurulumSonucu::Kuruldu {
            ad: "ornek".into(),
            surum: "0.1.0".into(),
            guncelleme: false,
            izinler_silindi: false,
        }
    );
    assert_eq!(o.kayit()["eklentiler"].as_array().unwrap().len(), 1);
    assert_eq!(o.satir("ornek").await.durum, EklentiDurumu::Yuklendi);
}

#[tokio::test]
async fn tamamen_bozuk_kayit_bos_sayilir_ve_yedeklenir() {
    let mut o = Ortam::yeni().await;
    std::fs::write(o.veri.path().join("eklentiler.json"), "{bozuk").unwrap();
    o.yeniden(Vec::new()).await;
    assert!(o.liste().await.is_empty());
    assert_eq!(
        std::fs::read_to_string(o.veri.path().join("eklentiler.json.bozuk")).unwrap(),
        "{bozuk"
    );
    // Kurulum yeni bir kayıt yazar.
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("ornek", "0.1.0").await;
    assert_eq!(o.kayit()["eklentiler"][0]["ad"], "ornek");
}

// --- Çalışma alanında açma, gönderim izni ---

#[tokio::test]
async fn alan_acma_kapama_oturum_ve_kimlik_hedefiyle_calisir() {
    let mut o = Ortam::yeni().await;
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("ornek", "0.1.0").await;
    let (oturum, _rx) = o.oturum().await;
    let kimlik = o.alan_kimligi();

    // Oturumun çalışma alanı açılır; yol kayda yazılır.
    o.t.uygulama
        .eklenti_alani_ayarla("ornek".into(), AlanHedefi::Oturum { oturum }, true)
        .await
        .unwrap();
    let alan = &o.kayit()["eklentiler"][0]["calisma_alanlari"][kimlik.as_str()];
    assert_eq!(alan["acik"], true);
    assert_eq!(alan["yol"], o.alan.path().display().to_string());
    let s = o.satir("ornek").await;
    assert_eq!(s.alanlar.len(), 1);
    assert_eq!(s.alanlar[0].kimlik, kimlik);
    assert!(s.alanlar[0].acik);

    // Kapatmak gönderim iznini de siler (izin elle tohumlanır).
    o.kayit_degistir(|k| {
        k["eklentiler"][0]["calisma_alanlari"][kimlik.as_str()]["gonderim_izni_ms"] = json!(5);
    });
    o.yeniden(Vec::new()).await;
    assert_eq!(o.satir("ornek").await.alanlar[0].gonderim_izni_ms, Some(5));
    o.t.uygulama
        .eklenti_alani_ayarla(
            "ornek".into(),
            AlanHedefi::Kimlik {
                kimlik: kimlik.clone(),
            },
            false,
        )
        .await
        .unwrap();
    let s = o.satir("ornek").await;
    assert!(!s.alanlar[0].acik);
    assert_eq!(s.alanlar[0].gonderim_izni_ms, None);
    let alan = &o.kayit()["eklentiler"][0]["calisma_alanlari"][kimlik.as_str()];
    assert!(alan.get("gonderim_izni_ms").is_none());

    // Kimlik hedefiyle yeniden açılabilir.
    o.t.uygulama
        .eklenti_alani_ayarla(
            "ornek".into(),
            AlanHedefi::Kimlik {
                kimlik: kimlik.clone(),
            },
            true,
        )
        .await
        .unwrap();
    assert!(o.satir("ornek").await.alanlar[0].acik);
}

#[tokio::test]
async fn alan_hedefi_dogrulanir() {
    let o = Ortam::yeni().await;
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("ornek", "0.1.0").await;
    let once = o.kayit();

    let kimlik = |k: &str| AlanHedefi::Kimlik { kimlik: k.into() };
    for (ad, hedef, kod) in [
        ("ornek", kimlik("../x"), HataKodu::GecersizGirdi),
        ("ornek", kimlik("a/b"), HataKodu::GecersizGirdi),
        ("ornek", kimlik("0123456789abcde"), HataKodu::GecersizGirdi),
        ("ornek", kimlik(DIGER), HataKodu::GecersizGirdi), // kayıtta yok
        ("../x", kimlik(KIMLIK), HataKodu::GecersizGirdi),
        ("yok-boyle", kimlik(KIMLIK), HataKodu::GecersizGirdi),
        (
            "ornek",
            AlanHedefi::Oturum {
                oturum: OturumId::yeni(),
            },
            HataKodu::OturumYok,
        ),
    ] {
        let hata =
            o.t.uygulama
                .eklenti_alani_ayarla(ad.into(), hedef.clone(), true)
                .await
                .unwrap_err();
        assert_eq!(hata.kod, kod, "{ad} {hedef:?}: {}", hata.mesaj);
    }
    assert_eq!(o.kayit(), once, "reddedilen çağrılar kaydı değiştirmez");

    // Etkin ayarı da adı doğrular.
    for ad in ["../x", "a/b", "yok-boyle"] {
        let hata =
            o.t.uygulama
                .eklenti_etkin_ayarla(ad.into(), false)
                .await
                .unwrap_err();
        assert_eq!(hata.kod, HataKodu::GecersizGirdi, "{ad}");
    }
    assert_eq!(o.kayit(), once);
}

#[tokio::test]
async fn gonderim_izni_kaldirilir_ve_dogrulanir() {
    let mut o = Ortam::yeni().await;
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("ornek", "0.1.0").await;
    o.kayit_degistir(|k| {
        k["eklentiler"][0]["calisma_alanlari"] = json!({
            KIMLIK: {"yol": "C:\\a", "acik": true, "gonderim_izni_ms": 9},
            DIGER: {"yol": "C:\\b", "acik": true, "gonderim_izni_ms": 8},
        });
    });
    o.yeniden(Vec::new()).await;

    // Geri almak sıkılaştırmadır: onay sorulmaz.
    o.t.uygulama
        .gonderim_iznini_kaldir("ornek".into(), KIMLIK.into())
        .await
        .unwrap();
    assert!(o.t.iletisim.sorulanlar().is_empty());
    let s = o.satir("ornek").await;
    assert_eq!(s.alanlar[0].kimlik, KIMLIK);
    assert_eq!(s.alanlar[0].gonderim_izni_ms, None);
    assert!(s.alanlar[0].acik, "çalışma alanı açık kalır");
    assert_eq!(
        s.alanlar[1].gonderim_izni_ms,
        Some(8),
        "başka alan etkilenmez"
    );

    for (ad, kimlik) in [
        ("ornek", "../x"),
        ("ornek", "0123456789abcde"),
        ("../x", KIMLIK),
        ("yok-boyle", KIMLIK),
        ("ornek", "aaaaaaaaaaaaaaaa"), // kayıtta yok
    ] {
        let hata =
            o.t.uygulama
                .gonderim_iznini_kaldir(ad.into(), kimlik.into())
                .await
                .unwrap_err();
        assert_eq!(hata.kod, HataKodu::GecersizGirdi, "{ad} {kimlik}");
    }
}

// --- Kaldırma ve veri silme ---

#[tokio::test]
async fn kaldirma_kaydi_dosyalari_ve_depoyu_siler_vazgecilirse_hicbirini() {
    let mut o = Ortam::yeni().await;
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("ornek", "0.1.0").await;
    o.klasor_yaz(&temiz_manifest("diger", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("diger", "0.1.0").await;
    o.kayit_degistir(|k| {
        k["eklentiler"][0]["calisma_alanlari"] = json!({KIMLIK: {"yol": "C:\\a", "acik": true}});
    });
    o.yeniden(Vec::new()).await;
    o.depo_yaz("ornek", KIMLIK);
    o.depo_yaz("ornek", DIGER);
    o.depo_yaz("diger", KIMLIK);
    let once = o.kayit();

    // Vazgeçilirse hiçbir şey silinmez.
    o.t.iletisim.onay_ver(false);
    assert!(!o.t.uygulama.eklenti_kaldir("ornek".into()).await.unwrap());
    assert_eq!(
        o.son_soru(),
        "Eklentiyi kaldır: ornek eklentisi ve bütün çalışma alanlarındaki verisi kalıcı olarak silinecek. Bu geri alınamaz."
    );
    assert_eq!(o.kayit(), once);
    assert!(o.kurulu_dizin("ornek").is_dir());
    assert!(o.depo_dizini("ornek", KIMLIK).is_dir());

    // Onaylanırsa kayıt, kurulu dizin ve bütün depolar silinir; öteki eklenti kalır.
    o.t.iletisim.onay_ver(true);
    assert!(o.t.uygulama.eklenti_kaldir("ornek".into()).await.unwrap());
    let adlar: Vec<String> = o.kayit()["eklentiler"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["ad"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(adlar, vec!["diger"]);
    assert!(!o.kurulu_dizin("ornek").exists());
    assert!(!o.veri.path().join("eklenti-verisi").join("ornek").exists());
    assert!(o.kurulu_dizin("diger").is_dir());
    assert!(o.depo_dizini("diger", KIMLIK).is_dir());
    let liste = o.liste().await;
    assert_eq!(liste.len(), 1);
    assert_eq!(liste[0].ad, "diger");

    // Bilinmeyen ve geçersiz adlar.
    for ad in [
        "ornek",
        "yok-boyle",
        "../eklentiler",
        "a/b",
        "",
        "diger.yeni",
    ] {
        let hata = o.t.uygulama.eklenti_kaldir(ad.into()).await.unwrap_err();
        assert_eq!(hata.kod, HataKodu::GecersizGirdi, "{ad:?}");
    }
    assert!(
        o.kurulu_dizin("diger").is_dir(),
        "yol geçişi bir şey silmedi"
    );
}

#[tokio::test]
async fn veri_silme_yerel_onayla_yalnizca_o_deposunu_siler() {
    let mut o = Ortam::yeni().await;
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("ornek", "0.1.0").await;
    o.kayit_degistir(|k| {
        k["eklentiler"][0]["calisma_alanlari"] =
            json!({KIMLIK: {"yol": "C:\\proje", "acik": true}});
    });
    o.yeniden(Vec::new()).await;
    o.depo_yaz("ornek", KIMLIK);
    o.depo_yaz("ornek", DIGER);
    // Yol geçişi denemesi için deponun dışında bir dizin.
    let dis = o.veri.path().join("dis-dizin");
    std::fs::create_dir(&dis).unwrap();
    std::fs::write(dis.join("dosya"), "x").unwrap();

    o.t.iletisim.onay_ver(false);
    assert!(
        !o.t.uygulama
            .eklenti_verisini_sil("ornek".into(), KIMLIK.into())
            .await
            .unwrap()
    );
    let soru = o.son_soru();
    assert!(soru.starts_with("Eklenti verisini sil: ornek eklentisinin bu çalışma alanındaki verisi (ör. arama indeksi) silinecek. Sonraki kullanımda yeniden oluşturulur."), "{soru}");
    assert!(soru.contains("Çalışma alanı: C:\\proje"), "{soru}");
    assert!(o.depo_dizini("ornek", KIMLIK).is_dir());

    o.t.iletisim.onay_ver(true);
    assert!(
        o.t.uygulama
            .eklenti_verisini_sil("ornek".into(), KIMLIK.into())
            .await
            .unwrap()
    );
    assert!(!o.depo_dizini("ornek", KIMLIK).exists());
    assert!(
        o.depo_dizini("ornek", DIGER).is_dir(),
        "başka alanın deposu kalır"
    );
    // Eklenti ve kaydı yerinde.
    assert_eq!(o.satir("ornek").await.durum, EklentiDurumu::Yuklendi);

    // Doğrulama: yol geçişi, ayraç, kısa kimlik; onay bile sorulmaz.
    let sorulan = o.t.iletisim.sorulanlar().len();
    for (ad, kimlik) in [
        ("ornek", "../../dis-dizin"),
        ("ornek", "../x"),
        ("ornek", "a/b"),
        ("ornek", "0123456789abcde"),
        ("ornek", "0123456789ABCDEF"),
        ("../ornek", KIMLIK),
        ("a/b", KIMLIK),
        ("yok-boyle", KIMLIK),
    ] {
        let hata =
            o.t.uygulama
                .eklenti_verisini_sil(ad.into(), kimlik.into())
                .await
                .unwrap_err();
        assert_eq!(hata.kod, HataKodu::GecersizGirdi, "{ad} {kimlik}");
    }
    assert_eq!(o.t.iletisim.sorulanlar().len(), sorulan);
    assert!(dis.join("dosya").is_file());
    assert!(o.depo_dizini("ornek", DIGER).is_dir());
}

// --- Oturum ---

#[tokio::test]
async fn oturum_eklentileri_alandaki_durumu_gosterir() {
    let o = Ortam::yerlesikli().await;
    o.yerlesik_yaz(
        "kalfa-rag",
        &temiz_manifest("kalfa-rag", "0.1.0"),
        temel_wasm(),
    );
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("ornek", "0.1.0").await;
    let (oturum, _rx) = o.oturum().await;

    let liste = o.t.uygulama.oturum_eklentileri(oturum).await.unwrap();
    assert_eq!(liste.len(), 2);
    assert_eq!(liste[0].ad, "kalfa-rag");
    assert!(liste[0].yerlesik);
    assert_eq!(liste[1].ad, "ornek");
    assert!(!liste[1].yerlesik);
    for e in &liste {
        assert!(
            !e.acik && !e.kullanilamaz && e.gonderim_izni_ms.is_none(),
            "{e:?}"
        );
    }

    o.t.uygulama
        .eklenti_alani_ayarla("ornek".into(), AlanHedefi::Oturum { oturum }, true)
        .await
        .unwrap();
    o.t.uygulama
        .eklenti_alani_ayarla("kalfa-rag".into(), AlanHedefi::Oturum { oturum }, true)
        .await
        .unwrap();
    o.t.uygulama
        .eklenti_etkin_ayarla("kalfa-rag".into(), false)
        .await
        .unwrap();
    let liste = o.t.uygulama.oturum_eklentileri(oturum).await.unwrap();
    assert!(liste[1].acik && !liste[1].kullanilamaz);
    assert!(
        liste[0].acik && liste[0].kullanilamaz,
        "devre dışı: anahtar değişmez"
    );

    // Yüklenemeyen eklenti de kullanılamaz görünür.
    std::fs::write(o.kurulu_dizin("ornek").join("eklenti.wasm"), b"x").unwrap();
    let liste = o.t.uygulama.oturum_eklentileri(oturum).await.unwrap();
    assert!(liste[1].kullanilamaz);

    let hata =
        o.t.uygulama
            .oturum_eklentileri(OturumId::yeni())
            .await
            .unwrap_err();
    assert_eq!(hata.kod, HataKodu::OturumYok);
}

#[tokio::test]
async fn eklenti_istegi_bekleyen_yokken_hata_doner() {
    let o = Ortam::yeni().await;
    let (oturum, _rx) = o.oturum().await;
    let hata =
        o.t.uygulama
            .eklenti_istegi(oturum, kalfa_core::kimlik::IslemId::yeni())
            .await
            .unwrap_err();
    assert_eq!(hata.kod, HataKodu::GecersizGirdi);
    assert_eq!(hata.mesaj, "Bu istek artık önizlemede beklemiyor.");

    let hata =
        o.t.uygulama
            .eklenti_istegi(OturumId::yeni(), kalfa_core::kimlik::IslemId::yeni())
            .await
            .unwrap_err();
    assert_eq!(hata.kod, HataKodu::OturumYok);
}

#[tokio::test]
async fn gunluk_sinir_sinirdan_sonra_var_basarili_istekten_sonra_yok() {
    let sifirlanma = "2026-10-12T00:00:00Z".to_string();
    let o = Ortam::senaryolu(vec![
        SahteYanit::metin("bir"),
        SahteYanit::Hata(EvrenError::GunlukSinir {
            sifirlanma: Some(sifirlanma.clone()),
        }),
        SahteYanit::Hata(EvrenError::GunlukSinir { sifirlanma: None }),
        SahteYanit::baglanti_hatasi("ağ yok"),
        SahteYanit::metin("iki"),
    ])
    .await;
    let (oturum, mut rx) = o.oturum().await;
    assert_eq!(o.t.uygulama.gunluk_sinir(), None);

    o.tur(oturum, &mut rx, "a").await;
    assert_eq!(o.t.uygulama.gunluk_sinir(), None);

    o.tur(oturum, &mut rx, "b").await;
    assert_eq!(
        o.t.uygulama.gunluk_sinir(),
        Some(GunlukSinirDurumu {
            sifirlanma: Some(sifirlanma)
        })
    );

    // Sıfırlanma anı bilinmiyorsa da sınır durumu görünür.
    o.tur(oturum, &mut rx, "c").await;
    assert_eq!(
        o.t.uygulama.gunluk_sinir(),
        Some(GunlukSinirDurumu { sifirlanma: None })
    );

    // Başka türden bir hata durumu değiştirmez.
    o.tur(oturum, &mut rx, "d").await;
    assert_eq!(
        o.t.uygulama.gunluk_sinir(),
        Some(GunlukSinirDurumu { sifirlanma: None })
    );

    // Başarılı bir istek sınırı kaldırır.
    o.tur(oturum, &mut rx, "e").await;
    assert_eq!(o.t.uygulama.gunluk_sinir(), None);
}

// --- Oturuma bağlama ---

#[tokio::test]
async fn eklenti_araci_kapaliyken_sunulmaz_acilinca_sunulur() {
    let o = Ortam::senaryolu(vec![
        SahteYanit::metin("1"),
        SahteYanit::metin("2"),
        SahteYanit::metin("3"),
        SahteYanit::metin("4"),
        SahteYanit::metin("5"),
        SahteYanit::metin("6"),
    ])
    .await;
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("ornek", "0.1.0").await;
    let (oturum, mut rx) = o.oturum().await;
    let var = |n: usize| o.istek_araclari(n).contains(&"ornek__yanki".to_string());

    // Kurulu ama bu çalışma alanında kapalı: modele sunulmaz.
    o.tur(oturum, &mut rx, "a").await;
    assert!(!var(0));
    assert!(
        o.istek_araclari(0).iter().any(|a| a == "dosya_oku"),
        "yerleşik araçlar yerinde: {:?}",
        o.istek_araclari(0)
    );

    // Oturum sürerken açmak hemen geçerlidir.
    o.t.uygulama
        .eklenti_alani_ayarla("ornek".into(), AlanHedefi::Oturum { oturum }, true)
        .await
        .unwrap();
    o.tur(oturum, &mut rx, "b").await;
    assert!(var(1));
    assert!(o.istek_araclari(1).contains(&"ornek__evren".to_string()));

    // Devre dışı bırakmak da.
    o.t.uygulama
        .eklenti_etkin_ayarla("ornek".into(), false)
        .await
        .unwrap();
    o.tur(oturum, &mut rx, "c").await;
    assert!(!var(2));
    o.t.uygulama
        .eklenti_etkin_ayarla("ornek".into(), true)
        .await
        .unwrap();
    o.tur(oturum, &mut rx, "d").await;
    assert!(var(3));

    // Çalışma alanında kapatmak.
    o.t.uygulama
        .eklenti_alani_ayarla("ornek".into(), AlanHedefi::Oturum { oturum }, false)
        .await
        .unwrap();
    o.tur(oturum, &mut rx, "e").await;
    assert!(!var(4));

    // Kaldırılan eklentinin eski araçları açık oturumda sunulmaz.
    o.t.uygulama
        .eklenti_alani_ayarla("ornek".into(), AlanHedefi::Oturum { oturum }, true)
        .await
        .unwrap();
    o.t.iletisim.onay_ver(true);
    assert!(o.t.uygulama.eklenti_kaldir("ornek".into()).await.unwrap());
    o.tur(oturum, &mut rx, "f").await;
    assert!(!var(5));
}

#[tokio::test]
async fn guncellenen_eklentinin_eski_araclari_acik_oturumda_sunulmaz() {
    let o = Ortam::senaryolu(vec![
        SahteYanit::metin("1"),
        SahteYanit::metin("2"),
        SahteYanit::metin("3"),
        SahteYanit::metin("4"),
    ])
    .await;
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("ornek", "0.1.0").await;
    let (oturum, mut rx) = o.oturum().await;
    o.t.uygulama
        .eklenti_alani_ayarla("ornek".into(), AlanHedefi::Oturum { oturum }, true)
        .await
        .unwrap();
    let var = |n: usize| o.istek_araclari(n).contains(&"ornek__yanki".to_string());
    o.tur(oturum, &mut rx, "a").await;
    assert!(var(0));

    // Farklı bileşenli güncelleme: eski sürümün araçları sunulmaz.
    o.klasor_yaz(&temiz_manifest("ornek", "0.2.0"), &wasm_varyanti(1));
    o.kur_ve_dogrula("ornek", "0.2.0").await;
    o.tur(oturum, &mut rx, "b").await;
    assert!(
        !var(1),
        "güncellemeden önce yüklenen araçlar sunulmaz: {:?}",
        o.istek_araclari(1)
    );

    // Oturum yeniden açılınca yeni sürümün araçları bağlanır.
    o.t.uygulama.oturumu_kapat(oturum).await.unwrap();
    o.t.uygulama.oturum_ac(oturum).await.unwrap();
    let (kanal, mut rx) = SahteKanal::yeni();
    o.t.uygulama.oturumu_dinle(oturum, kanal).unwrap();
    o.tur(oturum, &mut rx, "c").await;
    assert!(var(2));

    // Aynı bileşenle yalnızca manifesti değişen güncelleme de eski araçları
    // (eski manifestle yüklenmişlerdi) sunulmaz yapar.
    o.klasor_yaz(
        &manifest("ornek", "0.3.0", &["**/*.md"], &[]),
        &wasm_varyanti(1),
    );
    o.kur_ve_dogrula("ornek", "0.3.0").await;
    o.tur(oturum, &mut rx, "d").await;
    assert!(!var(3), "{:?}", o.istek_araclari(3));
}

#[tokio::test]
async fn eklenti_araci_oturum_acilisinda_da_baglanir_yuklenemeyen_eklenti_oturumu_dusurmez() {
    let o = Ortam::senaryolu(vec![
        SahteYanit::metin("1"),
        SahteYanit::metin("2"),
        SahteYanit::metin("3"),
    ])
    .await;
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("ornek", "0.1.0").await;
    let (oturum, mut rx) = o.oturum().await;
    o.t.uygulama
        .eklenti_alani_ayarla("ornek".into(), AlanHedefi::Oturum { oturum }, true)
        .await
        .unwrap();
    o.tur(oturum, &mut rx, "a").await;
    o.t.uygulama.oturumu_kapat(oturum).await.unwrap();

    // Kayıttan açılan oturum eklenti araçlarını yeniden bağlar.
    o.t.uygulama.oturum_ac(oturum).await.unwrap();
    let (kanal, mut rx) = SahteKanal::yeni();
    o.t.uygulama.oturumu_dinle(oturum, kanal).unwrap();
    o.tur(oturum, &mut rx, "b").await;
    assert!(o.istek_araclari(1).contains(&"ornek__yanki".to_string()));
    o.t.uygulama.oturumu_kapat(oturum).await.unwrap();

    // Bileşeni bozulan eklenti oturum açılmasını engellemez; araçları sunulmaz.
    std::fs::write(o.kurulu_dizin("ornek").join("eklenti.wasm"), b"bozuk").unwrap();
    let (yeni, mut rx) = o.oturum().await;
    assert_ne!(yeni, oturum);
    assert!(matches!(
        o.satir("ornek").await.durum,
        EklentiDurumu::Yuklenemedi { .. }
    ));
    o.tur(yeni, &mut rx, "c").await;
    assert!(!o.istek_araclari(2).contains(&"ornek__yanki".to_string()));
    assert!(
        o.istek_araclari(2).iter().any(|a| a == "dosya_oku"),
        "yerleşik araçlar yerinde"
    );
}

// --- İlk gönderim (aşama 4 tasarımı §8.4, §9.4) ---

const GOMME: &str = r#"{"model":"qwen3-embedding-8b","input":"merhaba","dimensions":32}"#;

fn evren_cagrisi() -> SahteYanit {
    SahteYanit::arac("ornek__evren", json!({"uc": "embeddings", "govde": GOMME}))
}

fn onayla(o: &OnayOgesi) -> OnayGirdisi {
    OnayGirdisi {
        cagri: o.cagri,
        muhur: o.muhur.clone(),
        karar: OnayKarari::Onayla,
        kisayol: false,
    }
}

fn arac_sonucu(olaylar: &[Olay], arac: &str) -> (AracDurumu, String) {
    olaylar
        .iter()
        .find_map(|o| match o {
            Olay::AracSonucu {
                arac: a,
                durum,
                icerik,
                ..
            } if a == arac => Some((*durum, icerik.clone())),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{arac} sonucu yok: {olaylar:?}"))
}

impl Ortam {
    async fn onay_ogeleri(
        &self,
        oturum: OturumId,
        rx: &mut UnboundedReceiver<KanalMesaji>,
    ) -> Vec<OnayOgesi> {
        match self
            .bekle(oturum, rx, |o| matches!(o, Olay::OnayGerekiyor { .. }))
            .await
            .pop()
        {
            Some(Olay::OnayGerekiyor { ogeler, .. }) => ogeler,
            _ => unreachable!(),
        }
    }

    fn izin_ms(&self) -> Option<u64> {
        self.kayit()["eklentiler"][0]["calisma_alanlari"][self.alan_kimligi().as_str()]
            ["gonderim_izni_ms"]
            .as_u64()
    }
}

#[tokio::test]
async fn ilk_gonderim_yerel_pencerede_sorulur_izin_kalici_olur_geri_alinir() {
    let o = Ortam::senaryolu(vec![
        evren_cagrisi(),
        SahteYanit::metin("tamam"),
        evren_cagrisi(),
        SahteYanit::metin("bitti"),
        evren_cagrisi(),
        SahteYanit::metin("reddedildi"),
    ])
    .await;
    std::fs::write(o.alan.path().join("not.md"), "# not").unwrap();
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("ornek", "0.1.0").await;
    let (oturum, mut rx) = o.oturum().await;
    o.t.uygulama
        .eklenti_alani_ayarla("ornek".into(), AlanHedefi::Oturum { oturum }, true)
        .await
        .unwrap();
    assert_eq!(o.izin_ms(), None);

    // 1) İlk gönderim: kart yerel onay ister, yerel pencere neyin gideceğini söyler.
    o.t.uygulama.mesaj_gonder(oturum, "a".into()).await.unwrap();
    let ogeler = o.onay_ogeleri(oturum, &mut rx).await;
    assert_eq!(ogeler.len(), 1);
    assert!(ogeler[0].yerel_onay);
    assert_eq!(ogeler[0].arac, "ornek__evren");
    assert!(
        ogeler[0]
            .aciklama
            .contains("ilk kez EVREN'e veri gönderecek")
            && ogeler[0].aciklama.contains("1 dosya"),
        "{}",
        ogeler[0].aciklama
    );
    assert!(o.t.saglayici.ek_istekleri().is_empty());
    o.t.iletisim.onay_ver(true);
    o.t.uygulama
        .arac_onayi(oturum, vec![onayla(&ogeler[0])])
        .await
        .unwrap();
    let soru = o.son_soru();
    assert!(
        soru.starts_with(
            "Eklenti EVREN'e veri gönderecek: ornek eklentisinin `evren` aracını çalıştırmak istiyor."
        ),
        "{soru}"
    );
    assert!(
        soru.contains("ornek bu çalışma alanında ilk kez EVREN'e veri gönderecek."),
        "{soru}"
    );
    assert!(soru.contains("`**/*.md` ile eşleşen 1 dosya"), "{soru}");
    assert!(
        !soru.contains("**ornek**") && !soru.contains("yalıtım"),
        "{soru}"
    );
    let olaylar = o
        .bekle(oturum, &mut rx, |e| matches!(e, Olay::Tamamlandi { .. }))
        .await;
    let (durum, icerik) = arac_sonucu(&olaylar, "ornek__evren");
    assert_eq!(durum, AracDurumu::Basarili, "{icerik}");
    assert_eq!(o.t.saglayici.ek_istekleri().len(), 1);

    // İzin kalıcı yazıldı.
    assert!(o.izin_ms().is_some_and(|ms| ms > 0));
    let s = o.satir("ornek").await;
    assert_eq!(s.alanlar.len(), 1);
    assert!(s.alanlar[0].gonderim_izni_ms.is_some());
    let liste = o.t.uygulama.oturum_eklentileri(oturum).await.unwrap();
    assert!(liste[0].acik && liste[0].gonderim_izni_ms.is_some());

    // 2) İzin varken sorulmaz; "Göndermeden önce göster" açıksa gövde "Metni
    // göster" ile okunur ve karar verilince bırakılır.
    o.t.uygulama.onizleme_ayarla(oturum, true).await.unwrap();
    o.t.uygulama.mesaj_gonder(oturum, "b".into()).await.unwrap();
    let mut nedenler = Vec::new();
    let mut gorulen_govde = None;
    let mut eklenti_islemi = None;
    loop {
        let olaylar = o
            .bekle(oturum, &mut rx, |e| {
                matches!(
                    e,
                    Olay::GonderimOnizlemesi { .. }
                        | Olay::Tamamlandi { .. }
                        | Olay::OnayGerekiyor { .. }
                )
            })
            .await;
        match olaylar.last() {
            Some(Olay::GonderimOnizlemesi {
                islem,
                neden,
                ogeler,
            }) => {
                nedenler.push(*neden);
                if *neden == OnizlemeNedeni::Eklenti {
                    assert_eq!(ogeler.len(), 1);
                    assert!(ogeler[0].eklenti.is_some());
                    // Karar gelmeden sahte EVREN istek almaz.
                    assert_eq!(o.t.saglayici.ek_istekleri().len(), 1);
                    gorulen_govde =
                        Some(o.t.uygulama.eklenti_istegi(oturum, *islem).await.unwrap());
                    eklenti_islemi = Some(*islem);
                }
                o.t.uygulama
                    .gonderim_karari(oturum, *islem, Vec::new(), Vec::new())
                    .await
                    .unwrap();
            }
            Some(Olay::Tamamlandi { .. }) => break,
            diger => panic!("izin varken onay sorulmamalıydı: {diger:?}"),
        }
    }
    assert_eq!(nedenler[0], OnizlemeNedeni::Eklenti, "{nedenler:?}");
    let govde = gorulen_govde.unwrap();
    assert!(
        govde.contains("\"model\": \"qwen3-embedding-8b\""),
        "{govde}"
    );
    assert!(govde.lines().count() > 3, "gövde okunur biçimde: {govde}");
    assert_eq!(o.t.saglayici.ek_istekleri().len(), 2);
    let hata =
        o.t.uygulama
            .eklenti_istegi(oturum, eklenti_islemi.unwrap())
            .await
            .unwrap_err();
    assert_eq!(hata.mesaj, "Bu istek artık önizlemede beklemiyor.");
    o.t.uygulama.onizleme_ayarla(oturum, false).await.unwrap();

    // 3) İzni kaldırmak sıkılaştırmadır; sonraki ilk gönderim yeniden sorulur.
    o.t.uygulama
        .gonderim_iznini_kaldir("ornek".into(), o.alan_kimligi())
        .await
        .unwrap();
    assert_eq!(o.izin_ms(), None);
    o.t.uygulama.mesaj_gonder(oturum, "c".into()).await.unwrap();
    let ogeler = o.onay_ogeleri(oturum, &mut rx).await;
    assert!(ogeler[0].yerel_onay);
    o.t.iletisim.onay_ver(false);
    o.t.uygulama
        .arac_onayi(oturum, vec![onayla(&ogeler[0])])
        .await
        .unwrap();
    let olaylar = o
        .bekle(oturum, &mut rx, |e| matches!(e, Olay::Tamamlandi { .. }))
        .await;
    assert_eq!(
        arac_sonucu(&olaylar, "ornek__evren").0,
        AracDurumu::Reddedildi
    );
    assert_eq!(
        o.t.saglayici.ek_istekleri().len(),
        2,
        "reddedilen çağrı ağa çıkmaz"
    );
    assert_eq!(o.izin_ms(), None, "reddedilince izin yazılmaz");
}

#[tokio::test]
async fn izin_yazilamazsa_ilk_gonderim_calismaz() {
    let o = Ortam::senaryolu(vec![evren_cagrisi(), SahteYanit::metin("tamam")]).await;
    std::fs::write(o.alan.path().join("not.md"), "# not").unwrap();
    o.klasor_yaz(&temiz_manifest("ornek", "0.1.0"), temel_wasm());
    o.kur_ve_dogrula("ornek", "0.1.0").await;
    let (oturum, mut rx) = o.oturum().await;
    o.t.uygulama
        .eklenti_alani_ayarla("ornek".into(), AlanHedefi::Oturum { oturum }, true)
        .await
        .unwrap();
    // Bundan sonra `eklentiler.json` yazılamaz.
    std::fs::create_dir(o.veri.path().join("eklentiler.json.yeni")).unwrap();

    o.t.uygulama.mesaj_gonder(oturum, "a".into()).await.unwrap();
    let ogeler = o.onay_ogeleri(oturum, &mut rx).await;
    o.t.iletisim.onay_ver(true);
    o.t.uygulama
        .arac_onayi(oturum, vec![onayla(&ogeler[0])])
        .await
        .unwrap();
    let olaylar = o
        .bekle(oturum, &mut rx, |e| matches!(e, Olay::Tamamlandi { .. }))
        .await;
    let (durum, icerik) = arac_sonucu(&olaylar, "ornek__evren");
    assert_eq!(durum, AracDurumu::Basarisiz);
    assert!(icerik.contains("gönderim izni kaydedilemedi"), "{icerik}");
    assert!(o.t.saglayici.ek_istekleri().is_empty(), "istek ağa çıkmadı");
    assert_eq!(
        o.t.uygulama.oturum_eklentileri(oturum).await.unwrap()[0].gonderim_izni_ms,
        None
    );
}
