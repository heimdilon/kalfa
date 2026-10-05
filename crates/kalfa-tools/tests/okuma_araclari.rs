use std::sync::Arc;

use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi};
use kalfa_core::izin::Siniflandirici;
use kalfa_core::yol::{CalismaYolu, YolHatasi};
use kalfa_tools::{CalismaAlani, DosyaListele, DosyaOku, IcerikAra};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

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

async fn calistir(arac: &dyn Arac, a: Value) -> Result<AracCiktisi, AracHatasi> {
    let h = arac.hazirla(&a).await?;
    arac.calistir(&a, &h, CancellationToken::new()).await
}

#[tokio::test]
async fn dosya_oku_satir_numarali() {
    let (d, a) = alan();
    dosya(&d, "a.txt", "bir\niki\nüç\n".as_bytes());
    let oku = DosyaOku::yeni(a);
    let h = oku.hazirla(&json!({"yol": "a.txt"})).await.unwrap();
    assert_eq!(h.okunan, vec![CalismaYolu::coz("a.txt").unwrap()]);
    assert!(h.aciklama.contains("okumak"));
    let c = calistir(&oku, json!({"yol": "a.txt"})).await.unwrap();
    assert_eq!(c.icerik, "     1\tbir\n     2\tiki\n     3\tüç\n");
    assert_eq!(c.kaynak, "dosya:a.txt:1-3");
}

#[tokio::test]
async fn arama_coklu_baglantinin_icerigini_atlar_okuma_onayli_tarifle_calisir() {
    let (d, a) = alan();
    dosya(&d, ".env", b"SIR=aranan\n");
    std::fs::hard_link(d.path().join(".env"), d.path().join("normal.txt")).unwrap();
    dosya(&d, "tek.txt", b"aranan normal\n");
    let c = calistir(&IcerikAra::yeni(a.clone()), json!({"desen":"aranan"}))
        .await
        .unwrap();
    assert!(!c.icerik.contains("SIR="));
    assert!(!c.icerik.contains("normal.txt:"));
    assert!(c.icerik.contains("tek.txt:1:"));
    assert!(c.icerik.contains("1 dosya okunamadı; arama eksik"));

    dosya(&d, "kaynak.txt", b"mesru icerik\n");
    std::fs::hard_link(d.path().join("kaynak.txt"), d.path().join("kopya.txt")).unwrap();
    let oku = DosyaOku::yeni(a.clone());
    let args = json!({"yol":"kopya.txt"});
    let h = oku.hazirla(&args).await.unwrap();
    assert_eq!(h.okuma_baglanti_sayilari[0].1, 2);
    assert!(
        oku.calistir(&args, &h, CancellationToken::new())
            .await
            .unwrap()
            .icerik
            .contains("mesru icerik")
    );
    assert!(matches!(
        a.sinirli_oku(
            &CalismaYolu::coz("kopya.txt").unwrap(),
            &CancellationToken::new()
        ),
        Err(AracHatasi::OnayEskidi)
    ));
}

#[tokio::test]
async fn hazirliktan_sonra_baglanti_eklenen_okuma_icerik_dondurmez() {
    let (d, a) = alan();
    dosya(&d, "normal.txt", b"SIR=onaysiz\n");
    let oku = DosyaOku::yeni(a);
    let args = json!({"yol":"normal.txt"});
    let h = oku.hazirla(&args).await.unwrap();
    std::fs::hard_link(d.path().join("normal.txt"), d.path().join(".env")).unwrap();
    assert!(matches!(
        oku.calistir(&args, &h, CancellationToken::new()).await,
        Err(AracHatasi::OnayEskidi)
    ));
}

#[tokio::test]
async fn sir_aliasi_dislama_kurali_olarak_da_okunmaz() {
    for ad in [
        ".gitignore",
        ".ignore",
        ".git/info/exclude",
        "alt/.gitignore",
        "alt/.ignore",
        "alt/.git/info/exclude",
    ] {
        let (d, a) = alan();
        dosya(&d, ".env", b"TOKEN=aday123\n");
        let p = d.path().join(ad);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::hard_link(d.path().join(".env"), &p).unwrap();
        dosya(&d, "TOKEN=aday123", b"aranan\n");
        dosya(&d, "TOKEN=diger", b"aranan\n");
        dosya(&d, "alt/TOKEN=aday123", b"aranan\n");
        for arac in [
            &DosyaListele::yeni(a.clone()) as &dyn Arac,
            &IcerikAra::yeni(a.clone()),
        ] {
            let args = if arac.ad() == "dosya_listele" {
                json!({"desen":"**/*"})
            } else {
                json!({"desen":"aranan"})
            };
            let hata = calistir(arac, args).await.unwrap_err();
            assert!(!hata.to_string().contains("aday123"), "{ad}: {hata}");
        }
    }
}

#[tokio::test]
async fn guvenli_dislama_ic_ice_negation_ve_tur_onceligini_korur() {
    let (d, a) = alan();
    dosya(
        &d,
        ".gitignore",
        "\u{feff}*.log\n/kok.txt\nkapali/\n*.tmp\n\\#isim\n".as_bytes(),
    );
    dosya(&d, ".ignore", b"*.yasak\n!ortak.log\n");
    dosya(&d, ".git/info/exclude", b"*.dis\n");
    dosya(&d, "alt/.gitignore", b"!tut.log\n!izin.yasak\n*.alt\n");
    dosya(&d, "alt/.ignore", b"!izin.yasak\nortak.log\n");
    dosya(&d, "kapali/.gitignore", b"!acik.txt\n");
    for ad in [
        "kok.txt",
        "alt/kok.txt",
        "sil.log",
        "alt/tut.log",
        "alt/ortak.log",
        "ortak.log",
        "alt/izin.yasak",
        "yasak.yasak",
        "kapali/acik.txt",
        "alt/sil.alt",
        "sil.dis",
        "#isim",
        "tek.txt",
    ] {
        dosya(&d, ad, b"aranan\n");
    }
    let yollar = a.dosyalar(&CancellationToken::new()).unwrap();
    for ad in [
        "alt/kok.txt",
        "alt/tut.log",
        "ortak.log",
        "alt/izin.yasak",
        "tek.txt",
    ] {
        assert!(yollar.iter().any(|y| y.as_str() == ad), "{ad}: {yollar:?}");
    }
    for ad in [
        "kok.txt",
        "sil.log",
        "alt/ortak.log",
        "yasak.yasak",
        "kapali/acik.txt",
        "alt/sil.alt",
        "sil.dis",
        "#isim",
    ] {
        assert!(!yollar.iter().any(|y| y.as_str() == ad), "{ad}: {yollar:?}");
    }
}

#[tokio::test]
async fn sabit_baglanti_alan_disinda_ve_bagimlilikta_da_onayli_tarif_ister() {
    let (d, a) = alan();
    let dis = tempfile::tempdir().unwrap();
    std::fs::write(dis.path().join(".env"), b"SIR=disarida\n").unwrap();
    std::fs::create_dir(d.path().join("node_modules")).unwrap();
    std::fs::hard_link(
        dis.path().join(".env"),
        d.path().join("node_modules/normal.txt"),
    )
    .unwrap();
    let oku = DosyaOku::yeni(a.clone());
    let args = json!({"yol":"node_modules/normal.txt"});
    let h = oku.hazirla(&args).await.unwrap();
    assert_eq!(h.okuma_baglanti_sayilari[0].1, 2);
    let c = calistir(&IcerikAra::yeni(a), json!({"desen":"SIR="}))
        .await
        .unwrap();
    assert!(!c.icerik.contains("SIR=disarida"));
    assert!(c.icerik.contains("1 dosya okunamadı"));
    std::fs::hard_link(dis.path().join(".env"), d.path().join("ek.txt")).unwrap();
    assert!(matches!(
        oku.calistir(&args, &h, CancellationToken::new()).await,
        Err(AracHatasi::OnayEskidi)
    ));
    std::fs::remove_file(d.path().join("ek.txt")).unwrap();
    std::fs::remove_file(dis.path().join(".env")).unwrap();
    assert!(matches!(
        oku.calistir(&args, &h, CancellationToken::new()).await,
        Err(AracHatasi::OnayEskidi)
    ));
}

#[cfg(unix)]
#[tokio::test]
async fn bekleyen_fifo_hazirligi_olay_dongusunu_durdurmaz() {
    let (d, a) = alan();
    let p = d.path().join("boru");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&p)
            .status()
            .unwrap()
            .success()
    );
    let serbest_birakan = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        std::fs::OpenOptions::new().write(true).open(p).unwrap();
    });
    let oku = DosyaOku::yeni(a);
    let sonuc = tokio::time::timeout(
        std::time::Duration::from_millis(50),
        oku.hazirla(&json!({"yol":"boru"})),
    )
    .await;
    serbest_birakan.join().unwrap();
    assert!(
        sonuc.is_err(),
        "hazırlık, olay döngüsünün zaman aşımını engellememeli"
    );
}

#[tokio::test]
async fn buyuk_dosya_tek_satir_istenince_de_okunmaz() {
    let (d, a) = alan();
    // Süreci tüketmeden eski sınırsız okuma yolunu gösterir.
    let p = d.path().join("buyuk.txt");
    let mut veri = vec![b'a'; 8 * 1024 * 1024 + 1];
    veri[..10].copy_from_slice(b"bulunacak\n");
    std::fs::write(&p, veri).unwrap();
    let sonuc = calistir(
        &DosyaOku::yeni(a.clone()),
        json!({"yol":"buyuk.txt", "satir_sayisi":1}),
    )
    .await;
    assert!(
        sonuc.is_err(),
        "boyut sınırı çıktı sınırından önce uygulanmalı"
    );
    dosya(&d, "normal.txt", b"bulunacak\n");
    let sonuc = calistir(&IcerikAra::yeni(a), json!({"desen":"bulunacak"}))
        .await
        .unwrap();
    assert!(!sonuc.icerik.contains("buyuk.txt:"));
    assert!(sonuc.icerik.contains("normal.txt:1:"));
    assert!(sonuc.icerik.contains("1 dosya okunamadı; arama eksik"));
}

#[tokio::test]
async fn okuma_tam_bayt_sinirini_ve_tek_uzun_satiri_kabul_eder() {
    use kalfa_tools::calisma_alani::OKUMA_BAYT_SINIRI;
    let (d, a) = alan();
    dosya(&d, "sinir.txt", &vec![b'a'; OKUMA_BAYT_SINIRI]);
    let c = calistir(
        &DosyaOku::yeni(a),
        json!({"yol":"sinir.txt", "satir_sayisi":1}),
    )
    .await
    .unwrap();
    assert_eq!(c.kaynak, "dosya:sinir.txt:1-1");
    assert_eq!(c.icerik.len(), OKUMA_BAYT_SINIRI + 8);
    assert!(c.icerik.starts_with("     1\ta"));
}

#[tokio::test]
async fn seyrek_buyuk_dosyada_arama_eksikligini_bildirir() {
    let (d, a) = alan();
    let p = d.path().join("seyrek.txt");
    std::fs::write(&p, b"aranan\n").unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(p)
        .unwrap()
        .set_len(1024 * 1024 * 1024)
        .unwrap();
    let c = calistir(&IcerikAra::yeni(a), json!({"desen":"aranan"}))
        .await
        .unwrap();
    assert!(!c.icerik.contains("seyrek.txt:"));
    assert!(c.icerik.contains("1 dosya okunamadı; arama eksik"));
}

#[tokio::test]
async fn iptal_edilmis_okuma_ve_arama_dosya_acmadan_biter() {
    let (_d, a) = alan();
    let oku = DosyaOku::yeni(a.clone());
    let arg = json!({"yol":"olmayan.txt"});
    let h = oku.hazirla(&arg).await.unwrap();
    let iptal = CancellationToken::new();
    iptal.cancel();
    assert!(matches!(
        oku.calistir(&arg, &h, iptal.clone()).await,
        Err(AracHatasi::Iptal)
    ));
    let ara = IcerikAra::yeni(a);
    let arg = json!({"desen":"x"});
    let h = ara.hazirla(&arg).await.unwrap();
    assert!(matches!(
        ara.calistir(&arg, &h, iptal).await,
        Err(AracHatasi::Iptal)
    ));
}

#[tokio::test]
async fn dosya_oku_bolum_ve_sinirlar() {
    let (d, a) = alan();
    dosya(&d, "b.txt", b"1\n2\n3\n4\n5\n");
    dosya(&d, "bos.txt", b"");
    let oku = DosyaOku::yeni(a);
    let c = calistir(
        &oku,
        json!({"yol": "b.txt", "baslangic_satiri": 2, "satir_sayisi": 2}),
    )
    .await
    .unwrap();
    assert_eq!(
        c.icerik,
        "     2\t2\n     3\t3\n[devamı var: 5 satırın 2-3 arasındaki gösterildi]\n"
    );
    let hata = calistir(&oku, json!({"yol": "b.txt", "baslangic_satiri": 9}))
        .await
        .unwrap_err();
    assert!(hata.to_string().contains("5 satır"), "{hata}");
    assert_eq!(
        calistir(&oku, json!({"yol": "bos.txt"}))
            .await
            .unwrap()
            .icerik,
        "[dosya boş]"
    );
}

#[tokio::test]
async fn ikili_dosya_okunmaz() {
    let (d, a) = alan();
    dosya(&d, "resim.bin", &[0, 1, 2, 3]);
    let hata = calistir(&DosyaOku::yeni(a), json!({"yol": "resim.bin"}))
        .await
        .unwrap_err();
    assert!(matches!(hata, AracHatasi::Gecersiz(m) if m.contains("ikili")));
}

#[tokio::test]
async fn calisma_alani_disina_metinle_cikilamaz() {
    let (_d, a) = alan();
    let oku = DosyaOku::yeni(a);
    assert!(matches!(
        oku.hazirla(&json!({"yol": "../dis.txt"})).await,
        Err(AracHatasi::Yol(YolHatasi::UstDizin(_)))
    ));
    assert!(matches!(
        oku.hazirla(&json!({"yol": "/etc/passwd"})).await,
        Err(AracHatasi::Yol(YolHatasi::Mutlak(_)))
    ));
}

#[tokio::test]
async fn sembolik_baglanti_ile_disari_cikilamaz() {
    let (d, a) = alan();
    let dis = tempfile::tempdir().unwrap();
    std::fs::write(dis.path().join("sir.txt"), "gizli").unwrap();
    dosya_bagla(&dis.path().join("sir.txt"), &d.path().join("kacis.txt"));
    let sonuc = calistir(&DosyaOku::yeni(a), json!({"yol": "kacis.txt"})).await;
    assert!(
        sonuc.is_err(),
        "cap-std çalışma alanı dışını açmamalı: {sonuc:?}"
    );
}

fn dosya_bagla(hedef: &std::path::Path, ad: &std::path::Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(hedef, ad).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(hedef, ad)
        .expect("Windows test ajanında symlink yetkisi gerekir");
}

#[tokio::test]
async fn ic_baglanti_gizli_dosyayi_okutamaz_ve_aramaya_sokamaz() {
    let (d, a) = alan();
    dosya(&d, ".env", b"OZEL_SIR=123");
    dosya_bagla(std::path::Path::new(".env"), &d.path().join("normal.txt"));
    assert!(
        calistir(&DosyaOku::yeni(a.clone()), json!({"yol": "normal.txt"}))
            .await
            .is_err()
    );
    let sonuc = calistir(&IcerikAra::yeni(a), json!({"desen": "OZEL_SIR"}))
        .await
        .unwrap();
    assert!(!sonuc.icerik.contains("123"));
    assert!(!sonuc.icerik.contains("normal.txt:"));
}

#[tokio::test]
async fn okuma_hazirligindan_sonra_baglanti_konursa_acilmaz() {
    let (d, a) = alan();
    dosya(&d, "normal.txt", b"normal");
    dosya(&d, ".env", b"OZEL_SIR=123");
    let oku = DosyaOku::yeni(a);
    let arg = json!({"yol": "normal.txt"});
    let h = oku.hazirla(&arg).await.unwrap();
    std::fs::remove_file(d.path().join("normal.txt")).unwrap();
    dosya_bagla(std::path::Path::new(".env"), &d.path().join("normal.txt"));
    assert!(
        oku.calistir(&arg, &h, CancellationToken::new())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn listele_desen_gitignore_ve_gizli() {
    let (d, a) = alan();
    dosya(&d, "src/a.rs", b"");
    dosya(&d, "src/b.txt", b"");
    dosya(&d, "target/x.rs", b"");
    dosya(&d, ".gitignore", b"target/\n");
    dosya(&d, ".env", b"X=1");
    let l = DosyaListele::yeni(a);
    assert_eq!(
        calistir(&l, json!({"desen": "**/*.rs"}))
            .await
            .unwrap()
            .icerik,
        "src/a.rs"
    );
    assert_eq!(
        calistir(&l, json!({"desen": ".env"})).await.unwrap().icerik,
        ".env (gizli: içeriği onay ister)"
    );
    assert_eq!(
        calistir(&l, json!({"desen": "**/*.md"}))
            .await
            .unwrap()
            .icerik,
        "Desene uyan dosya yok."
    );
    assert!(matches!(
        calistir(&l, json!({"desen": "a["})).await,
        Err(AracHatasi::Gecersiz(_))
    ));
}

#[tokio::test]
async fn icerik_ara_satir_numarasi_ve_gizli_dosya_atlama() {
    let (d, a) = alan();
    dosya(&d, "src/a.rs", b"fn main() {}\nlet anahtar = 1;\n");
    dosya(&d, ".env", b"anahtar=gizli");
    let ara = IcerikAra::yeni(a);
    let c = calistir(&ara, json!({"desen": "anahtar"})).await.unwrap();
    assert!(
        c.icerik.contains("src/a.rs:2: let anahtar = 1;"),
        "{}",
        c.icerik
    );
    assert!(
        !c.icerik.contains("anahtar=gizli"),
        "gizli dosyanın içeriği modele gitmez"
    );
    assert!(c.icerik.contains("1 gizli dosya aranmadı"));
    assert_eq!(c.kaynak, "arama:anahtar");

    let c = calistir(
        &ara,
        json!({"desen": "anahtar", "dosya_deseni": "**/*.txt"}),
    )
    .await
    .unwrap();
    assert_eq!(c.icerik, "Eşleşme yok.");
    let c = calistir(
        &ara,
        json!({"desen": "ANAHTAR", "buyuk_kucuk_harf_duyarsiz": true}),
    )
    .await
    .unwrap();
    assert!(c.icerik.contains("src/a.rs:2:"));
    assert!(matches!(
        calistir(&ara, json!({"desen": "("})).await,
        Err(AracHatasi::Gecersiz(_))
    ));
}

#[tokio::test]
async fn hard_link_sayisi() {
    let (d, a) = alan();
    dosya(&d, "a.txt", b"x");
    let y = CalismaYolu::coz("a.txt").unwrap();
    assert_eq!(a.baglanti_sayisi(&y).unwrap(), 1);
    std::fs::hard_link(d.path().join("a.txt"), d.path().join("b.txt")).unwrap();
    assert_eq!(a.baglanti_sayisi(&y).unwrap(), 2);
    assert_eq!(
        a.baglanti_sayisi(&CalismaYolu::coz("yok.txt").unwrap())
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn gizli_dizin_cocugu_aramaya_ve_normal_listeye_giremez() {
    let (d, a) = alan();
    dosya(&d, "credentials/token.txt", b"BENZERSIZ_SIR=123");
    dosya(&d, "normal/token.txt", b"BENZERSIZ_SIR=normal");
    let c = calistir(
        &IcerikAra::yeni(a.clone()),
        json!({"desen":"BENZERSIZ_SIR"}),
    )
    .await
    .unwrap();
    assert!(!c.icerik.contains("123"));
    assert!(c.icerik.contains("normal/token.txt:1:"));
    assert!(c.icerik.contains("1 gizli dosya aranmadı"));
    let c = calistir(&DosyaListele::yeni(a), json!({"desen":"**/token.txt"}))
        .await
        .unwrap();
    assert!(
        c.icerik
            .contains("credentials/token.txt (gizli: içeriği onay ister)")
    );
}
