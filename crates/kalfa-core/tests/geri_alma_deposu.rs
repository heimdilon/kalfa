//! Önceki hâllerin deposu (aşama 3 tasarımı §5.2).

use std::collections::HashSet;
use std::io;

use kalfa_core::geri_alma::{OncekiHalDeposu, ozet_gecerli_mi};
use kalfa_core::muhur::ozet_hex;

fn depo() -> (tempfile::TempDir, OncekiHalDeposu) {
    let d = tempfile::tempdir().unwrap();
    let depo = OncekiHalDeposu::ac(&d.path().join("geri-alma/oturum")).unwrap();
    (d, depo)
}

#[test]
fn saklanan_icerik_ozetiyle_geri_okunur() {
    let (_d, depo) = depo();
    let icerik = b"eski icerik\n";
    let ozet = ozet_hex(icerik);
    depo.sakla(&ozet, icerik).unwrap();
    assert!(depo.var_mi(&ozet));
    assert_eq!(depo.oku(&ozet).unwrap(), icerik);
}

#[test]
fn ozetle_uyusmayan_icerik_saklanmaz() {
    let (_d, depo) = depo();
    let ozet = ozet_hex(b"a");
    let hata = depo.sakla(&ozet, b"b").unwrap_err();
    assert_eq!(hata.kind(), io::ErrorKind::InvalidInput);
    assert!(!depo.var_mi(&ozet));
    assert_eq!(depo.boyut().unwrap(), 0);
}

#[test]
fn bozulan_dosya_okunmaz() {
    let (_d, depo) = depo();
    let icerik = b"saglam";
    let ozet = ozet_hex(icerik);
    depo.sakla(&ozet, icerik).unwrap();
    std::fs::write(depo.dizin().join(&ozet), b"bozuk").unwrap();
    let hata = depo.oku(&ozet).unwrap_err();
    assert_eq!(hata.kind(), io::ErrorKind::InvalidData);
    assert!(hata.to_string().contains("önceki hâl bozuk"), "{hata}");
}

#[test]
fn ayni_icerik_bir_kez_saklanir() {
    let (_d, depo) = depo();
    let icerik = b"tekrar";
    let ozet = ozet_hex(icerik);
    depo.sakla(&ozet, icerik).unwrap();
    depo.sakla(&ozet, icerik).unwrap();
    let dosyalar: Vec<_> = std::fs::read_dir(depo.dizin())
        .unwrap()
        .map(|g| g.unwrap().file_name())
        .collect();
    assert_eq!(dosyalar.len(), 1, "{dosyalar:?}");
    assert_eq!(depo.boyut().unwrap(), icerik.len() as u64);
}

#[test]
fn bos_icerik_de_saklanir() {
    let (_d, depo) = depo();
    let ozet = ozet_hex(b"");
    depo.sakla(&ozet, b"").unwrap();
    assert_eq!(depo.oku(&ozet).unwrap(), b"");
}

#[test]
fn gecersiz_ozet_adi_reddedilir() {
    let (_d, depo) = depo();
    let gecerli = ozet_hex(b"x");
    assert!(ozet_gecerli_mi(&gecerli));
    let gecersizler = [
        "../x".to_string(),
        gecerli.to_uppercase(),
        gecerli[..63].to_string(),
        format!("{}/x", &gecerli[..62]),
        format!("{gecerli}0"),
        String::new(),
    ];
    for ad in &gecersizler {
        assert!(!ozet_gecerli_mi(ad), "{ad}");
        assert_eq!(
            depo.sakla(ad, b"x").unwrap_err().kind(),
            io::ErrorKind::InvalidInput,
            "{ad}"
        );
        assert_eq!(
            depo.oku(ad).unwrap_err().kind(),
            io::ErrorKind::InvalidInput,
            "{ad}"
        );
        assert!(!depo.var_mi(ad), "{ad}");
        assert_eq!(
            depo.sil(ad).unwrap_err().kind(),
            io::ErrorKind::InvalidInput,
            "{ad}"
        );
    }
}

#[test]
fn kullanilmayan_ve_yarim_dosyalar_temizlenir() {
    let (_d, depo) = depo();
    let kalan = ozet_hex(b"kalan");
    let giden = ozet_hex(b"giden icerik");
    depo.sakla(&kalan, b"kalan").unwrap();
    depo.sakla(&giden, b"giden icerik").unwrap();
    let yarim = depo.dizin().join(format!(".{giden}.1234.yeni"));
    std::fs::write(&yarim, b"yarim").unwrap();
    let yabanci = depo.dizin().join("benioku.txt");
    std::fs::write(&yabanci, b"el surmez").unwrap();

    let kullanilan: HashSet<String> = [kalan.clone()].into_iter().collect();
    let silinen = depo.kullanilmayanlari_sil(&kullanilan).unwrap();
    assert_eq!(silinen, "giden icerik".len() as u64 + "yarim".len() as u64);
    assert!(depo.var_mi(&kalan));
    assert!(!depo.var_mi(&giden));
    assert!(!yarim.exists());
    assert!(yabanci.exists(), "tanınmayan dosyaya dokunulmaz");
}

#[test]
fn silinen_ozet_yoksa_hata_vermez() {
    let (_d, depo) = depo();
    let ozet = ozet_hex(b"z");
    depo.sil(&ozet).unwrap();
    depo.sakla(&ozet, b"z").unwrap();
    depo.sil(&ozet).unwrap();
    assert!(!depo.var_mi(&ozet));
}

#[test]
fn tumunu_sil_dizini_kaldirir() {
    let (_d, depo) = depo();
    let ozet = ozet_hex(b"q");
    depo.sakla(&ozet, b"q").unwrap();
    let dizin = depo.dizin().to_path_buf();
    depo.tumunu_sil().unwrap();
    assert!(!dizin.exists());
}

#[test]
fn dosyanin_yerindeki_depo_acilmaz_ve_yazilamaz() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("depo");
    std::fs::write(&yol, b"dosya").unwrap();
    assert!(OncekiHalDeposu::ac(&yol).is_err());

    // Açıldıktan sonra dizin bir dosyayla değişirse saklama hata verir.
    let yol = d.path().join("depo2");
    let depo = OncekiHalDeposu::ac(&yol).unwrap();
    std::fs::remove_dir(&yol).unwrap();
    std::fs::write(&yol, b"dosya").unwrap();
    let ozet = ozet_hex(b"w");
    assert!(depo.sakla(&ozet, b"w").is_err());
}

#[cfg(unix)]
#[test]
fn depo_izinleri_yalnizca_kullaniciya_acik() {
    use std::os::unix::fs::PermissionsExt;
    let (_d, depo) = depo();
    let ozet = ozet_hex(b"izin");
    depo.sakla(&ozet, b"izin").unwrap();
    let dizin = std::fs::metadata(depo.dizin())
        .unwrap()
        .permissions()
        .mode();
    let dosya = std::fs::metadata(depo.dizin().join(&ozet))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(dizin & 0o777, 0o700);
    assert_eq!(dosya & 0o777, 0o600);
}
