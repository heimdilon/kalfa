//! Sağ panelin dosya görünümü komutları (masaüstü arayüzü tasarımı §6.3):
//! kimlikler oturuma bağlıdır, başka oturumun ya da kapanmış oturumun
//! kimliği geçersizdir, gizli dosyanın içeriği dönmez, git dalı okunur.

use kalfa_core::kimlik::OturumId;
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::SahteSaglayici;
use kalfa_uygulama::dosya_gorunumu::{DosyaIcerigi, GitDurumu};
use kalfa_uygulama::hata::HataKodu;
use kalfa_uygulama::test_destek::{TestUygulamasi, test_uygulamasi};
use tempfile::TempDir;

struct Ortam {
    _veri: TempDir,
    alan: TempDir,
    t: TestUygulamasi,
}

async fn ortam() -> Ortam {
    let veri = tempfile::tempdir().unwrap();
    let alan = tempfile::tempdir().unwrap();
    let saglayici = SahteSaglayici::yeni(vec![]);
    saglayici.sart_kabul(1).await.unwrap();
    let t = test_uygulamasi(veri.path(), saglayici, Some("evren_llm_a"));
    t.uygulama.baslat().await;
    Ortam {
        _veri: veri,
        alan,
        t,
    }
}

impl Ortam {
    async fn oturum(&self, kok: &std::path::Path) -> OturumId {
        self.t.iletisim.klasor_ver(kok);
        let alan = self.t.uygulama.calisma_alani_sec().await.unwrap();
        self.t
            .uygulama
            .oturum_baslat(alan.kimlik, "glm-5.3".into())
            .await
            .unwrap()
            .oturum
    }

    async fn kimlik(&self, oturum: OturumId, dizin: Option<u64>, ad: &str) -> u64 {
        self.t
            .uygulama
            .calisma_alani_dizini(oturum, dizin)
            .await
            .unwrap()
            .girdiler
            .into_iter()
            .find(|g| g.ad == ad)
            .unwrap_or_else(|| panic!("{ad} listede yok"))
            .kimlik
    }
}

#[tokio::test]
async fn dizin_ve_dosya_kimlikle_okunur_gizli_dosya_icerigi_donmez() {
    let o = ortam().await;
    std::fs::create_dir(o.alan.path().join("src")).unwrap();
    std::fs::write(
        o.alan.path().join("src/gorevler.py"),
        "def tamamla():\n    pass\n",
    )
    .unwrap();
    std::fs::write(o.alan.path().join(".env"), "ANAHTAR=gizli").unwrap();
    let oturum = o.oturum(o.alan.path()).await;

    let src = o.kimlik(oturum, None, "src").await;
    let py = o.kimlik(oturum, Some(src), "gorevler.py").await;
    assert_eq!(
        o.t.uygulama
            .calisma_alani_dosyasi(oturum, py)
            .await
            .unwrap(),
        DosyaIcerigi::Metin {
            metin: "def tamamla():\n    pass\n".into(),
            kirpildi: false
        }
    );
    // Aynı yol aynı kimliği korur.
    assert_eq!(o.kimlik(oturum, Some(src), "gorevler.py").await, py);

    let env = o.kimlik(oturum, None, ".env").await;
    assert_eq!(
        o.t.uygulama
            .calisma_alani_dosyasi(oturum, env)
            .await
            .unwrap(),
        DosyaIcerigi::Gizli
    );
}

#[tokio::test]
async fn baska_oturumun_ve_bilinmeyen_kimlik_gecersiz() {
    let o = ortam().await;
    let ikinci_alan = tempfile::tempdir().unwrap();
    std::fs::write(o.alan.path().join("bir.txt"), "bir").unwrap();
    std::fs::write(ikinci_alan.path().join("iki.txt"), "iki").unwrap();
    let birinci = o.oturum(o.alan.path()).await;
    let ikinci = o.oturum(ikinci_alan.path()).await;

    let kimlik = o.kimlik(birinci, None, "bir.txt").await;
    let hata =
        o.t.uygulama
            .calisma_alani_dosyasi(ikinci, kimlik)
            .await
            .unwrap_err();
    assert_eq!(hata.kod, HataKodu::GecersizGirdi, "{hata:?}");
    assert!(
        o.t.uygulama
            .calisma_alani_dizini(ikinci, Some(kimlik))
            .await
            .is_err()
    );
    assert!(
        o.t.uygulama
            .calisma_alani_dosyasi(birinci, 0)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn kapanan_oturumun_kimlikleri_silinir() {
    let o = ortam().await;
    std::fs::write(o.alan.path().join("a.txt"), "a").unwrap();
    let oturum = o.oturum(o.alan.path()).await;
    let kimlik = o.kimlik(oturum, None, "a.txt").await;
    o.t.uygulama.oturumu_kapat(oturum).await.unwrap();
    let hata =
        o.t.uygulama
            .calisma_alani_dosyasi(oturum, kimlik)
            .await
            .unwrap_err();
    assert_eq!(hata.kod, HataKodu::OturumYok);
}

#[tokio::test]
async fn ozet_git_dalini_okur() {
    let o = ortam().await;
    std::fs::create_dir(o.alan.path().join(".git")).unwrap();
    std::fs::write(o.alan.path().join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    let oturum = o.oturum(o.alan.path()).await;
    let ozet = o.t.uygulama.calisma_alani_ozeti(oturum).await.unwrap();
    assert_eq!(ozet.git, GitDurumu::Dal { ad: "main".into() });
    // `.git` listede görünmez.
    let dizin =
        o.t.uygulama
            .calisma_alani_dizini(oturum, None)
            .await
            .unwrap();
    assert!(dizin.girdiler.iter().all(|g| g.ad != ".git"));
}

/// Görünüm oturumun açılışta aldığı tutamacı kullanır: çalışma alanının
/// yolu sonradan başka bir yere bağlansa da seçilen klasörde kalır. (Linux:
/// Windows açık tutamaçlı dizinin adının değişmesine izin vermez.)
#[cfg(unix)]
#[tokio::test]
async fn calisma_alani_yolu_degistirilse_de_secilen_klasor_gorunur() {
    let o = ortam().await;
    let kok = o.alan.path().join("proje");
    std::fs::create_dir(&kok).unwrap();
    std::fs::write(kok.join("ic.txt"), "ic").unwrap();
    let dis = tempfile::tempdir().unwrap();
    std::fs::write(dis.path().join("dis.txt"), "dis").unwrap();
    let oturum = o.oturum(&kok).await;

    std::fs::rename(&kok, o.alan.path().join("eski-proje")).unwrap();
    std::os::unix::fs::symlink(dis.path(), &kok).unwrap();

    let adlar: Vec<String> =
        o.t.uygulama
            .calisma_alani_dizini(oturum, None)
            .await
            .unwrap()
            .girdiler
            .into_iter()
            .map(|g| g.ad)
            .collect();
    assert_eq!(adlar, vec!["ic.txt".to_string()]);
}
