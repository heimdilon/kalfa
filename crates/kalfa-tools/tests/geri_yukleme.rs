//! Geri alma için çalışma alanına denetimli geri yazma (aşama 3 tasarımı
//! §5.6). Geri yazma dosya araçlarının yolundan geçer: bağlantı izlenmez,
//! `.git` takma adı reddedilir, hard link'li dosyaya yazılmaz, yazmadan hemen
//! önce şimdiki özet denetlenir.

use std::io;
use std::sync::Arc;

use kalfa_core::geri_alma::{DosyaDurumu, GeriYukleyici};
use kalfa_core::izin::Siniflandirici;
use kalfa_core::muhur::ozet_hex;
use kalfa_core::yol::CalismaYolu;
use kalfa_tools::CalismaAlani;
use tempfile::TempDir;

fn alan() -> (TempDir, Arc<CalismaAlani>) {
    let d = tempfile::tempdir().unwrap();
    let s = Arc::new(Siniflandirici::yeni(d.path(), &[]).unwrap());
    let a = CalismaAlani::ac(s).unwrap();
    (d, a)
}

fn y(s: &str) -> CalismaYolu {
    CalismaYolu::coz(s).unwrap()
}

fn oku(d: &TempDir, yol: &str) -> String {
    std::fs::read_to_string(d.path().join(yol)).unwrap()
}

#[test]
fn durum_ozeti_ve_baglanti_sayisini_verir() {
    let (d, a) = alan();
    std::fs::write(d.path().join("x.txt"), "icerik").unwrap();
    assert_eq!(
        a.durum(&y("x.txt")).unwrap(),
        Some(DosyaDurumu {
            ozet: ozet_hex(b"icerik"),
            baglanti_sayisi: 1,
        })
    );
    assert_eq!(a.durum(&y("yok.txt")).unwrap(), None);
    assert_eq!(a.durum(&y("yok/dizin/x.txt")).unwrap(), None);
    std::fs::create_dir(d.path().join("dizin")).unwrap();
    assert_eq!(
        a.durum(&y("dizin")).unwrap_err().kind(),
        io::ErrorKind::PermissionDenied,
        "normal dosya değil"
    );
}

#[test]
fn buyuk_dosyanin_durumu_ve_geri_yazmasi_akisli_ozetlenir() {
    let (d, a) = alan();
    let boyut = 16 * 1024 * 1024;
    let p = d.path().join("buyuk.txt");
    std::fs::File::create(&p).unwrap().set_len(boyut).unwrap();
    let ozet = ozet_hex(&vec![0; boyut as usize]);
    assert_eq!(a.durum(&y("buyuk.txt")).unwrap().unwrap().ozet, ozet);
    assert_eq!(a.icerik_ozeti(&y("buyuk.txt")).unwrap().unwrap(), ozet);
    assert!(
        a.geri_yaz(&y("buyuk.txt"), Some(b"eski"), &ozet_hex(b"yanlis"))
            .is_err()
    );
    assert_eq!(std::fs::metadata(&p).unwrap().len(), boyut);
    a.geri_yaz(&y("buyuk.txt"), Some(b"eski"), &ozet).unwrap();
    assert_eq!(std::fs::read(p).unwrap(), b"eski");
}

#[test]
fn geri_yaz_onceki_icerigi_koyar() {
    let (d, a) = alan();
    std::fs::write(d.path().join("x.txt"), "yeni ve daha uzun\n").unwrap();
    a.geri_yaz(
        &y("x.txt"),
        Some(b"eski\n"),
        &ozet_hex(b"yeni ve daha uzun\n"),
    )
    .unwrap();
    assert_eq!(oku(&d, "x.txt"), "eski\n");
}

#[test]
fn geri_yaz_ozet_tutmazsa_yazmaz() {
    let (d, a) = alan();
    std::fs::write(d.path().join("x.txt"), "kullanici degistirdi").unwrap();
    let hata = a
        .geri_yaz(&y("x.txt"), Some(b"eski"), &ozet_hex(b"kalfa yazdi"))
        .unwrap_err();
    assert_eq!(hata.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(oku(&d, "x.txt"), "kullanici degistirdi");
    let hata = a
        .geri_yaz(&y("x.txt"), None, &ozet_hex(b"kalfa yazdi"))
        .unwrap_err();
    assert_eq!(hata.kind(), io::ErrorKind::PermissionDenied);
    assert!(d.path().join("x.txt").exists(), "silinmedi");
}

#[test]
fn geri_yaz_icerik_yoksa_dosyayi_siler() {
    let (d, a) = alan();
    std::fs::create_dir(d.path().join("u")).unwrap();
    std::fs::write(d.path().join("u/yeni.txt"), "tur olusturdu").unwrap();
    a.geri_yaz(&y("u/yeni.txt"), None, &ozet_hex(b"tur olusturdu"))
        .unwrap();
    assert!(!d.path().join("u/yeni.txt").exists());
}

#[test]
fn geri_yaz_eksik_dizini_olusturmaz() {
    let (d, a) = alan();
    let hata = a
        .geri_yaz(&y("yok/x.txt"), Some(b"eski"), &ozet_hex(b"x"))
        .unwrap_err();
    assert_eq!(hata.kind(), io::ErrorKind::NotFound);
    assert!(!d.path().join("yok").exists());
}

#[test]
fn hard_linkli_dosyaya_geri_yazilmaz() {
    let (d, a) = alan();
    std::fs::write(d.path().join("x.txt"), "yeni").unwrap();
    std::fs::hard_link(d.path().join("x.txt"), d.path().join("ikinci-ad.txt")).unwrap();
    let hata = a
        .geri_yaz(&y("x.txt"), Some(b"eski"), &ozet_hex(b"yeni"))
        .unwrap_err();
    assert_eq!(hata.kind(), io::ErrorKind::PermissionDenied);
    assert!(hata.to_string().contains("hard link"), "{hata}");
    assert_eq!(oku(&d, "ikinci-ad.txt"), "yeni");
    assert_eq!(a.durum(&y("x.txt")).unwrap().unwrap().baglanti_sayisi, 2);
}

#[cfg(unix)]
#[test]
fn baglantili_dosyaya_ve_ust_dizinden_geri_yazilamaz() {
    let (d, a) = alan();
    let dis = tempfile::tempdir().unwrap();
    std::fs::write(dis.path().join("hedef.txt"), "yeni").unwrap();
    std::os::unix::fs::symlink(dis.path(), d.path().join("bag")).unwrap();
    std::os::unix::fs::symlink(dis.path().join("hedef.txt"), d.path().join("son.txt")).unwrap();
    let beklenen = ozet_hex(b"yeni");
    assert!(
        a.geri_yaz(&y("bag/hedef.txt"), Some(b"eski"), &beklenen)
            .is_err()
    );
    assert!(a.geri_yaz(&y("son.txt"), Some(b"eski"), &beklenen).is_err());
    assert!(a.geri_yaz(&y("son.txt"), None, &beklenen).is_err());
    assert_eq!(
        std::fs::read_to_string(dis.path().join("hedef.txt")).unwrap(),
        "yeni"
    );
    assert!(d.path().join("son.txt").symlink_metadata().is_ok());
}

#[cfg(windows)]
#[test]
fn junction_uzerinden_geri_yazilamaz() {
    let (d, a) = alan();
    let hedef = d.path().join(".git");
    let takma_ad = d.path().join("normal");
    std::fs::create_dir_all(hedef.join("hooks")).unwrap();
    std::fs::write(hedef.join("hooks/pre-commit"), "yeni").unwrap();
    // Yalnızca bu testin geçici dizinindeki iki yol; kullanıcı girdisi yok.
    let cikti = std::process::Command::new("cmd.exe")
        .args(["/D", "/C", "mklink", "/J"])
        .arg(&takma_ad)
        .arg(&hedef)
        .output()
        .unwrap();
    assert!(cikti.status.success(), "junction kurulamadı: {cikti:?}");
    let yol = y("normal/hooks/pre-commit");
    assert!(a.geri_yaz(&yol, Some(b"eski"), &ozet_hex(b"yeni")).is_err());
    assert_eq!(oku(&d, ".git/hooks/pre-commit"), "yeni");
}

/// Git dizinine başka bir adla (8.3 kısa ad) geri yazılmaz (güvenlik analizi
/// Bulgu 1).
#[cfg(windows)]
#[test]
fn kisa_ad_ile_git_dizinine_geri_yazilamaz() {
    let (d, a) = alan();
    std::fs::create_dir(d.path().join(".git")).unwrap();
    std::fs::write(d.path().join(".git/config"), "yeni").unwrap();
    let cikti = std::process::Command::new("cmd")
        .args(["/c", "dir", "/x", "/a"])
        .current_dir(d.path())
        .output()
        .unwrap();
    let metin = String::from_utf8_lossy(&cikti.stdout);
    let kisa = metin.lines().find_map(|satir| {
        let parcalar: Vec<&str> = satir.split_whitespace().collect();
        match parcalar.as_slice() {
            [.., kisa, ".git"] if kisa.contains('~') => Some(kisa.to_string()),
            _ => None,
        }
    });
    let Some(kisa) = kisa else {
        eprintln!("8.3 kısa adlar bu sürücüde kapalı; test atlandı");
        return;
    };
    let hata = a
        .geri_yaz(
            &y(&format!("{kisa}/config")),
            Some(b"eski"),
            &ozet_hex(b"yeni"),
        )
        .unwrap_err();
    assert_eq!(hata.kind(), io::ErrorKind::PermissionDenied, "{kisa}");
    assert_eq!(oku(&d, ".git/config"), "yeni");
    // Kendi adıyla geri yazma (çekirdeğin planından sonra) çalışır.
    a.geri_yaz(&y(".git/config"), Some(b"eski"), &ozet_hex(b"yeni"))
        .unwrap();
    assert_eq!(oku(&d, ".git/config"), "eski");
}

#[test]
fn bos_dizin_kaldirilir_dolu_dizin_kalir() {
    let (d, a) = alan();
    std::fs::create_dir_all(d.path().join("a/b")).unwrap();
    std::fs::write(d.path().join("a/dosya.txt"), "x").unwrap();
    assert!(a.bos_dizini_kaldir(&y("a/b")).unwrap());
    assert!(!d.path().join("a/b").exists());
    assert!(!a.bos_dizini_kaldir(&y("a")).unwrap(), "dolu dizin kalır");
    assert!(d.path().join("a").exists());
    assert!(!a.bos_dizini_kaldir(&y("yok")).unwrap());
}

#[cfg(unix)]
#[test]
fn bos_dizin_yerine_konan_baglanti_kaldirilmaz() {
    let (d, a) = alan();
    let dis = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(dis.path(), d.path().join("bag")).unwrap();
    assert!(a.bos_dizini_kaldir(&y("bag")).is_err() || d.path().join("bag").exists());
    assert!(dis.path().exists());
}
