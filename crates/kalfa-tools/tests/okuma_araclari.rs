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
