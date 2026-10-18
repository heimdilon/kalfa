//! Salt okur dosya görünümü (masaüstü arayüzü tasarımı §6.3): bağlantı
//! izlenmez, `.git` gösterilmez, gizli ve başka adlı dosyanın içeriği dönmez,
//! sınırlar uygulanır.

use std::sync::Arc;

use kalfa_core::izin::Siniflandirici;
use kalfa_core::yol::CalismaYolu;
use kalfa_tools::{CalismaAlani, DizinGirdisi, GirdiTuru, Onizleme};
use tempfile::TempDir;

fn alan() -> (TempDir, Arc<CalismaAlani>) {
    let d = tempfile::tempdir().unwrap();
    let s = Arc::new(Siniflandirici::yeni(d.path(), &[]).unwrap());
    let a = CalismaAlani::ac(s).unwrap();
    (d, a)
}

fn dosya(d: &TempDir, yol: &str, icerik: &[u8]) {
    let p = d.path().join(yol);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, icerik).unwrap();
}

fn yol(s: &str) -> CalismaYolu {
    CalismaYolu::coz(s).unwrap()
}

#[cfg(unix)]
fn dizin_baglantisi(hedef: &std::path::Path, ad: &std::path::Path) {
    std::os::unix::fs::symlink(hedef, ad).unwrap();
}

#[cfg(windows)]
fn dizin_baglantisi(hedef: &std::path::Path, ad: &std::path::Path) {
    std::os::windows::fs::symlink_dir(hedef, ad)
        .expect("Windows test ajanında symlink yetkisi gerekir");
}

#[cfg(unix)]
fn dosya_baglantisi(hedef: &std::path::Path, ad: &std::path::Path) {
    std::os::unix::fs::symlink(hedef, ad).unwrap();
}

#[cfg(windows)]
fn dosya_baglantisi(hedef: &std::path::Path, ad: &std::path::Path) {
    std::os::windows::fs::symlink_file(hedef, ad)
        .expect("Windows test ajanında symlink yetkisi gerekir");
}

#[test]
fn kok_dizinler_once_git_gizli() {
    let (d, a) = alan();
    dosya(&d, "b.txt", b"12345");
    dosya(&d, "a.py", b"x");
    dosya(&d, "src/main.rs", b"fn main() {}");
    dosya(&d, ".git/HEAD", b"ref: refs/heads/main\n");
    let (girdiler, kirpildi) = a.dizini_listele(None, 2000).unwrap();
    assert!(!kirpildi);
    assert_eq!(
        girdiler,
        vec![
            DizinGirdisi {
                ad: "src".into(),
                tur: GirdiTuru::Dizin,
                boyut: None
            },
            DizinGirdisi {
                ad: "a.py".into(),
                tur: GirdiTuru::Dosya,
                boyut: Some(1)
            },
            DizinGirdisi {
                ad: "b.txt".into(),
                tur: GirdiTuru::Dosya,
                boyut: Some(5)
            },
        ]
    );
    let (alt, _) = a.dizini_listele(Some(&yol("src")), 2000).unwrap();
    assert_eq!(alt[0].ad, "main.rs");
}

#[test]
fn dizin_basina_sinir() {
    let (d, a) = alan();
    for i in 0..5 {
        dosya(&d, &format!("d/{i}.txt"), b"");
    }
    let (girdiler, kirpildi) = a.dizini_listele(Some(&yol("d")), 3).unwrap();
    assert_eq!(girdiler.len(), 3);
    assert!(kirpildi);
}

#[test]
fn baglanti_gosterilir_ama_izlenmez() {
    let (d, a) = alan();
    let dis = tempfile::tempdir().unwrap();
    std::fs::write(dis.path().join("gizem.txt"), b"alan disi").unwrap();
    dizin_baglantisi(dis.path(), &d.path().join("dis"));
    dosya_baglantisi(&dis.path().join("gizem.txt"), &d.path().join("kisayol.txt"));

    let (girdiler, _) = a.dizini_listele(None, 2000).unwrap();
    let turler: Vec<_> = girdiler.iter().map(|g| (g.ad.as_str(), g.tur)).collect();
    assert!(turler.contains(&("dis", GirdiTuru::Baglanti)), "{turler:?}");
    assert!(
        turler.contains(&("kisayol.txt", GirdiTuru::Baglanti)),
        "{turler:?}"
    );

    assert!(a.dizini_listele(Some(&yol("dis")), 2000).is_err());
    assert!(a.onizle(&yol("dis/gizem.txt"), 1024).is_err());
    assert!(a.onizle(&yol("kisayol.txt"), 1024).is_err());
}

#[test]
fn gizli_dosyanin_icerigi_donmez() {
    let (d, a) = alan();
    dosya(&d, ".env", b"ANAHTAR=gizli");
    dosya(&d, "ayar/.env.local", b"ANAHTAR=gizli");
    assert_eq!(a.onizle(&yol(".env"), 1024).unwrap(), Onizleme::Gizli);
    assert_eq!(
        a.onizle(&yol("ayar/.env.local"), 1024).unwrap(),
        Onizleme::Gizli
    );
    // Gizli dosya listede adıyla görünür.
    let (girdiler, _) = a.dizini_listele(None, 2000).unwrap();
    assert!(girdiler.iter().any(|g| g.ad == ".env"));
}

#[test]
fn baska_adi_olan_dosyanin_icerigi_donmez() {
    let (d, a) = alan();
    dosya(&d, ".env", b"ANAHTAR=gizli");
    std::fs::hard_link(d.path().join(".env"), d.path().join("notlar.txt")).unwrap();
    assert_eq!(
        a.onizle(&yol("notlar.txt"), 1024).unwrap(),
        Onizleme::CokAdli
    );
}

#[test]
fn metin_ikili_ve_kirpma() {
    let (d, a) = alan();
    dosya(&d, "kisa.txt", "merhaba dünya".as_bytes());
    dosya(&d, "uzun.txt", &[b'a'; 300]);
    dosya(&d, "resim.bin", b"PNG\0\0veri");
    assert_eq!(
        a.onizle(&yol("kisa.txt"), 256).unwrap(),
        Onizleme::Metin {
            metin: "merhaba dünya".into(),
            kirpildi: false
        }
    );
    assert_eq!(
        a.onizle(&yol("uzun.txt"), 256).unwrap(),
        Onizleme::Metin {
            metin: "a".repeat(256),
            kirpildi: true
        }
    );
    assert_eq!(a.onizle(&yol("resim.bin"), 256).unwrap(), Onizleme::Ikili);
    // Dizin dosya gibi açılmaz.
    dosya(&d, "klasor/x", b"");
    assert!(a.onizle(&yol("klasor"), 256).is_err());
}
