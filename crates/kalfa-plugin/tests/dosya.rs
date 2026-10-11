//! Dosya çağrıları ve yetki (aşama 4 tasarımı §6.3, §7.3, §8.2, §8.3).
//! Eklenti dosyalara yalnızca host fonksiyonlarıyla, manifestin globları
//! içinde ve yerleşik okumanın yolundan erişir. Gizli dosya, `.git`,
//! bağlantı ve çalışma alanı dışı yol açılmaz.

mod ortak;

use ortak::*;
use serde_json::{Value, json};

const GIZLI: &str = "GIZLI_DEGER=dosya-testi-41c7";

async fn duzenek(globlar: &[&str]) -> Duzenek {
    let d = Duzenek::yeni(&manifest(globlar, &[], 0)).await;
    d.dosya("a.rs", b"fn a() {}\n")
        .dosya("docs/b.md", b"# B\n")
        .dosya(".env", GIZLI.as_bytes())
        .dosya("ayar/credentials.json", GIZLI.as_bytes())
        .dosya(".git/config", b"[core]\n")
        .dosya("ikili.rs", &[0xff, 0xfe, 0x00, 0x01]);
    d
}

fn oku(yol: &str) -> Value {
    json!({"islem": "oku", "yol": yol})
}

#[tokio::test]
async fn std_fs_on_acik_dizin_olmadigi_icin_hicbir_dosyayi_acamaz() {
    let d = duzenek(&["**/*"]).await;
    let mutlak = d.alan.path().join("a.rs").display().to_string();
    for yol in ["a.rs", "/a.rs", mutlak.as_str(), ".env"] {
        let c = d.cagir("dosya", json!({"islem": "std", "yol": yol})).await;
        assert!(c.icerik().starts_with("okunamadı"), "{yol}: {}", c.icerik());
    }
}

#[tokio::test]
async fn glob_icindeki_dosya_okunur() {
    let d = duzenek(&["**/*"]).await;
    assert_eq!(d.cagir("dosya", oku("a.rs")).await.icerik(), "fn a() {}\n");
    assert_eq!(d.cagir("dosya", oku("docs/b.md")).await.icerik(), "# B\n");
}

#[tokio::test]
async fn gizli_korumali_ve_disaridaki_yollar_okunamaz() {
    let d = duzenek(&["**/*"]).await;
    let mutlak = d.alan.path().join("a.rs").display().to_string();
    for (yol, beklenen) in [
        (".env", "izin-yok: gizli dosya"),
        ("ayar/credentials.json", "izin-yok: gizli dosya"),
        (".git/config", "izin-yok: korumalı yol"),
        ("../disari.rs", "izin-yok: `../disari.rs` geçersiz yol"),
        (mutlak.as_str(), "izin-yok"),
        ("yok.rs", "bulunamadi: yok.rs"),
        ("ikili.rs", "diger: `ikili.rs` metin değil"),
    ] {
        let c = d.cagir("dosya", oku(yol)).await;
        assert!(c.icerik().starts_with(beklenen), "{yol}: {}", c.icerik());
        assert!(!c.icerik().contains("dosya-testi-41c7"));
    }
}

#[tokio::test]
async fn glob_disindaki_dosya_okunamaz_ve_listelenmez() {
    let d = duzenek(&["**/*.rs"]).await;
    let c = d.cagir("dosya", oku("docs/b.md")).await;
    assert_eq!(
        c.icerik(),
        "izin-yok: `docs/b.md` manifestteki globlara uymuyor"
    );
    let l = d
        .cagir("dosya", json!({"islem": "listele", "desen": "**/*"}))
        .await;
    let yollar: Vec<&str> = l
        .icerik()
        .lines()
        .map(|s| s.split('\t').next().unwrap())
        .collect();
    assert_eq!(yollar, vec!["a.rs", "ikili.rs"]);
}

#[tokio::test]
async fn liste_gizli_ve_git_dosyalarini_gostermez() {
    let d = duzenek(&["**/*"]).await;
    let l = d
        .cagir("dosya", json!({"islem": "listele", "desen": "**/*"}))
        .await;
    let satirlar: Vec<Vec<&str>> = l
        .icerik()
        .lines()
        .map(|s| s.split('\t').collect())
        .collect();
    let yollar: Vec<&str> = satirlar.iter().map(|s| s[0]).collect();
    assert_eq!(yollar, vec!["a.rs", "docs/b.md", "ikili.rs"]);
    // Boyut ve değişme zamanı (ms).
    assert_eq!(satirlar[0][1], "10");
    let zaman: u64 = satirlar[0][2].parse().unwrap();
    assert!(zaman > 1_700_000_000_000, "{zaman}");
    // Desen manifestle kesişir.
    let l = d
        .cagir("dosya", json!({"islem": "listele", "desen": "docs/**"}))
        .await;
    assert_eq!(l.icerik().split('\t').next(), Some("docs/b.md"));
}

#[tokio::test]
async fn buyuk_dosya_sinirda_durur() {
    let d = duzenek(&["**/*"]).await;
    d.dosya("buyuk.rs", &vec![b'a'; 4 * 1024 * 1024 + 1]);
    let c = d.cagir("dosya", oku("buyuk.rs")).await;
    assert!(c.icerik().starts_with("sinir-asildi"), "{}", c.icerik());
}

#[tokio::test]
async fn sembolik_baglanti_izlenmez() {
    let d = duzenek(&["**/*"]).await;
    let hedef = d.alan.path().join(".env");
    let baglanti = d.alan.path().join("bag.rs");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&hedef, &baglanti).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&hedef, &baglanti).unwrap();
    let c = d.cagir("dosya", oku("bag.rs")).await;
    assert!(c.icerik().starts_with("izin-yok"), "{}", c.icerik());
    assert!(!c.icerik().contains("dosya-testi-41c7"));
    let l = d
        .cagir("dosya", json!({"islem": "listele", "desen": "**/*"}))
        .await;
    assert!(!l.icerik().contains("bag.rs"), "{}", l.icerik());
}

#[tokio::test]
async fn hard_linkli_dosya_okunmaz() {
    let d = duzenek(&["**/*"]).await;
    d.dosya("c.rs", GIZLI.as_bytes());
    std::fs::hard_link(d.alan.path().join("c.rs"), d.alan.path().join("c2.rs")).unwrap();
    let c = d.cagir("dosya", oku("c2.rs")).await;
    assert!(c.icerik().starts_with("izin-yok"), "{}", c.icerik());
    assert!(c.icerik().contains("hard link"), "{}", c.icerik());
}

#[tokio::test]
async fn manifestte_olmayan_yetenek_izin_yok_alir() {
    let d = duzenek(&[]).await;
    assert_eq!(
        d.cagir("dosya", oku("a.rs")).await.icerik(),
        "izin-yok: dosya-oku yeteneği manifestte yok"
    );
    let c = d.cagir("yetkisiz", json!({})).await;
    assert_eq!(
        c.icerik(),
        "evren: izin-yok: evren yeteneği manifestte yok: `embeddings`\ndepo: izin-yok: depo yeteneği manifestte yok"
    );
}

/// Manifestte `evren` var ama bağlantı yok: istek ağa çıkmaz (4a yer tutucusunun
/// yerini alır; 4b'de bağlantısız oturumun iletisi budur).
#[tokio::test]
async fn evren_yetenegi_olsa_da_baglanti_yoksa_cagri_yok() {
    let d = Duzenek::yeni(&manifest(&[], &["embeddings"], 0)).await;
    let c = d.cagir("yetkisiz", json!({})).await;
    assert!(
        c.icerik().contains("EVREN bağlantısı bu oturumda yok"),
        "{}",
        c.icerik()
    );
}
