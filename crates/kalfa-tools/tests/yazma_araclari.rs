use std::sync::Arc;

use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, AracKayitDefteri, Hazirlik};
use kalfa_core::izin::Siniflandirici;
use kalfa_core::muhur::ozet_hex;
use kalfa_tools::{CalismaAlani, DosyaDuzenle, DosyaYaz, varsayilan_araclar};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

fn alan() -> (TempDir, Arc<CalismaAlani>) {
    let d = tempfile::tempdir().unwrap();
    let s = Arc::new(Siniflandirici::yeni(d.path(), &[]).unwrap());
    let a = CalismaAlani::ac(s).unwrap();
    (d, a)
}

fn oku(d: &TempDir, yol: &str) -> String {
    std::fs::read_to_string(d.path().join(yol)).unwrap()
}

async fn calistir(arac: &dyn Arac, a: &Value, h: &Hazirlik) -> Result<AracCiktisi, AracHatasi> {
    arac.calistir(a, h, CancellationToken::new()).await
}

#[tokio::test]
async fn dosya_yaz_yeni_dosya_ve_ust_dizinler() {
    let (d, a) = alan();
    let yaz = DosyaYaz::yeni(a);
    let arg = json!({"yol": "yeni/alt/d.txt", "icerik": "a\nb\n"});
    let h = yaz.hazirla(&arg).await.unwrap();
    assert!(h.aciklama.contains("oluşturmak"), "{}", h.aciklama);
    assert_eq!(h.yazilan[0].onceki_ozet, None);
    assert_eq!(h.yazilan[0].baglanti_sayisi, 1);
    let diff = h.diff.as_deref().unwrap();
    assert!(
        diff.contains("+++ b/yeni/alt/d.txt") && diff.contains("+a") && diff.contains("+b"),
        "{diff}"
    );
    let c = calistir(&yaz, &arg, &h).await.unwrap();
    assert!(c.icerik.contains("yazıldı"));
    assert_eq!(c.kaynak, "dosya:yeni/alt/d.txt");
    assert_eq!(oku(&d, "yeni/alt/d.txt"), "a\nb\n");
}

#[tokio::test]
async fn dosya_yaz_var_olani_yeniden_yazar() {
    let (d, a) = alan();
    std::fs::write(d.path().join("x.txt"), "eski\n").unwrap();
    let yaz = DosyaYaz::yeni(a);
    let arg = json!({"yol": "x.txt", "icerik": "yeni\n"});
    let h = yaz.hazirla(&arg).await.unwrap();
    assert!(h.aciklama.contains("yeniden yazmak"));
    assert_eq!(
        h.yazilan[0].onceki_ozet.as_deref(),
        Some(ozet_hex(b"eski\n").as_str())
    );
    let diff = h.diff.as_deref().unwrap();
    assert!(diff.contains("-eski") && diff.contains("+yeni"), "{diff}");
    calistir(&yaz, &arg, &h).await.unwrap();
    assert_eq!(oku(&d, "x.txt"), "yeni\n");
}

#[tokio::test]
async fn dosya_yaz_onaydan_sonra_degisirse_yazmaz() {
    let (d, a) = alan();
    std::fs::write(d.path().join("x.txt"), "eski\n").unwrap();
    let yaz = DosyaYaz::yeni(a);
    let arg = json!({"yol": "x.txt", "icerik": "yeni\n"});
    let h = yaz.hazirla(&arg).await.unwrap();
    std::fs::write(d.path().join("x.txt"), "başkası\n").unwrap();
    assert_eq!(calistir(&yaz, &arg, &h).await, Err(AracHatasi::OnayEskidi));
    assert_eq!(oku(&d, "x.txt"), "başkası\n");
}

#[tokio::test]
async fn yazma_hazirligindan_sonra_konulan_baglanti_izlenmez() {
    let (d, a) = alan();
    std::fs::write(d.path().join("x.txt"), "eski").unwrap();
    std::fs::write(d.path().join(".env"), "eski").unwrap();
    let yaz = DosyaYaz::yeni(a.clone());
    let arg = json!({"yol": "x.txt", "icerik": "yeni"});
    let h = yaz.hazirla(&arg).await.unwrap();
    std::fs::remove_file(d.path().join("x.txt")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(".env", d.path().join("x.txt")).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(".env", d.path().join("x.txt")).unwrap();
    // İçerik özeti aynı; yalnızca hash denetimi bu değişikliği yakalamaz.
    assert!(calistir(&yaz, &arg, &h).await.is_err());
    assert!(
        a.yaz(
            &kalfa_core::yol::CalismaYolu::coz("x.txt").unwrap(),
            b"yeni"
        )
        .is_err()
    );
    assert_eq!(oku(&d, ".env"), "eski");
}

#[tokio::test]
async fn baglantili_ust_dizinden_git_hook_yazilamaz() {
    let (d, a) = alan();
    std::fs::create_dir_all(d.path().join(".git/hooks")).unwrap();
    std::fs::write(d.path().join(".git/hooks/pre-commit"), "eski").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(".git", d.path().join("normal")).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(".git", d.path().join("normal")).unwrap();
    let yol = kalfa_core::yol::CalismaYolu::coz("normal/hooks/pre-commit").unwrap();
    assert!(a.yaz(&yol, b"yeni").is_err());
    assert!(
        DosyaYaz::yeni(a.clone())
            .hazirla(&json!({"yol": yol.as_str(), "icerik": "yeni"}))
            .await
            .is_err()
    );
    assert!(
        DosyaDuzenle::yeni(a)
            .hazirla(&json!({
                "yol": yol.as_str(),
                "eski_metin": "eski",
                "yeni_metin": "yeni",
            }))
            .await
            .is_err()
    );
    assert_eq!(oku(&d, ".git/hooks/pre-commit"), "eski");
}

#[cfg(windows)]
#[tokio::test]
async fn junction_uzerinden_git_hook_yazilamaz() {
    let (d, a) = alan();
    let hedef = d.path().join(".git");
    let takma_ad = d.path().join("normal");
    std::fs::create_dir_all(hedef.join("hooks")).unwrap();
    std::fs::write(hedef.join("hooks/pre-commit"), "eski").unwrap();
    // Yalnızca bu testin geçici dizinindeki iki yol; kullanıcı girdisi yok.
    let cikti = std::process::Command::new("cmd.exe")
        .args(["/D", "/C", "mklink", "/J"])
        .arg(&takma_ad)
        .arg(&hedef)
        .output()
        .unwrap();
    assert!(cikti.status.success(), "junction kurulamadı: {cikti:?}");
    let yol = kalfa_core::yol::CalismaYolu::coz("normal/hooks/pre-commit").unwrap();
    assert!(a.oku(&yol).is_err());
    assert!(a.yaz(&yol, b"yeni").is_err());
    assert_eq!(oku(&d, ".git/hooks/pre-commit"), "eski");
}

#[tokio::test]
async fn dosya_duzenle_tek_eslesme() {
    let (d, a) = alan();
    std::fs::write(d.path().join("s.txt"), "merhaba\ndünya\n").unwrap();
    let duzenle = DosyaDuzenle::yeni(a);
    let arg = json!({"yol": "s.txt", "eski_metin": "merhaba", "yeni_metin": "selam"});
    let h = duzenle.hazirla(&arg).await.unwrap();
    assert!(h.aciklama.contains("1 değişiklik"));
    let diff = h.diff.as_deref().unwrap();
    assert!(
        diff.contains("-merhaba") && diff.contains("+selam"),
        "{diff}"
    );
    calistir(&duzenle, &arg, &h).await.unwrap();
    assert_eq!(oku(&d, "s.txt"), "selam\ndünya\n");
}

#[tokio::test]
async fn dosya_duzenle_coklu_eslesme_ve_tumu() {
    let (d, a) = alan();
    std::fs::write(d.path().join("s.txt"), "x x x").unwrap();
    let duzenle = DosyaDuzenle::yeni(a);
    let hata = duzenle
        .hazirla(&json!({"yol": "s.txt", "eski_metin": "x", "yeni_metin": "y"}))
        .await
        .unwrap_err();
    assert!(hata.to_string().contains("3 kez"), "{hata}");
    let arg = json!({"yol": "s.txt", "eski_metin": "x", "yeni_metin": "y", "tumu": true});
    let h = duzenle.hazirla(&arg).await.unwrap();
    assert!(h.aciklama.contains("3 değişiklik"));
    calistir(&duzenle, &arg, &h).await.unwrap();
    assert_eq!(oku(&d, "s.txt"), "y y y");
}

#[tokio::test]
async fn dosya_duzenle_gecersiz_durumlar() {
    let (d, a) = alan();
    std::fs::write(d.path().join("s.txt"), "merhaba").unwrap();
    std::fs::write(d.path().join("b.bin"), [0u8, 1, 2]).unwrap();
    let duzenle = DosyaDuzenle::yeni(a);
    let dene = |arg: Value| {
        let duzenle = &duzenle;
        async move { duzenle.hazirla(&arg).await.unwrap_err().to_string() }
    };
    assert!(
        dene(json!({"yol": "s.txt", "eski_metin": "yok", "yeni_metin": "x"}))
            .await
            .contains("bulunamadı")
    );
    assert!(
        dene(json!({"yol": "s.txt", "eski_metin": "", "yeni_metin": "x"}))
            .await
            .contains("boş olamaz")
    );
    assert!(
        dene(json!({"yol": "olmayan.txt", "eski_metin": "a", "yeni_metin": "b"}))
            .await
            .contains("dosya_yaz")
    );
    assert!(
        dene(json!({"yol": "b.bin", "eski_metin": "a", "yeni_metin": "b"}))
            .await
            .contains("ikili")
    );
}

#[tokio::test]
async fn dosya_duzenle_onaydan_sonra_degisirse_yazmaz() {
    let (d, a) = alan();
    std::fs::write(d.path().join("s.txt"), "merhaba\n").unwrap();
    let duzenle = DosyaDuzenle::yeni(a);
    let arg = json!({"yol": "s.txt", "eski_metin": "merhaba", "yeni_metin": "selam"});
    let h = duzenle.hazirla(&arg).await.unwrap();
    std::fs::write(d.path().join("s.txt"), "merhaba\nekleme\n").unwrap();
    assert_eq!(
        calistir(&duzenle, &arg, &h).await,
        Err(AracHatasi::OnayEskidi)
    );
    assert_eq!(oku(&d, "s.txt"), "merhaba\nekleme\n");
}

#[tokio::test]
async fn hard_link_hedefi_hazirlikta_bildirilir() {
    let (d, a) = alan();
    std::fs::write(d.path().join("a.txt"), "x").unwrap();
    std::fs::hard_link(d.path().join("a.txt"), d.path().join("b.txt")).unwrap();
    let h = DosyaYaz::yeni(a)
        .hazirla(&json!({"yol": "a.txt", "icerik": "y"}))
        .await
        .unwrap();
    assert_eq!(h.yazilan[0].baglanti_sayisi, 2);
}

#[tokio::test]
async fn hard_link_onaydan_sonra_eklenirse_dosya_yazilmaz() {
    let (d, a) = alan();
    std::fs::write(d.path().join("x.txt"), "eski").unwrap();
    let yaz = DosyaYaz::yeni(a);
    let arg = json!({"yol": "x.txt", "icerik": "yeni"});
    let h = yaz.hazirla(&arg).await.unwrap();
    std::fs::hard_link(d.path().join("x.txt"), d.path().join("alias.txt")).unwrap();

    assert_eq!(calistir(&yaz, &arg, &h).await, Err(AracHatasi::OnayEskidi));
    assert_eq!(oku(&d, "x.txt"), "eski");
    assert_eq!(oku(&d, "alias.txt"), "eski");
}

#[tokio::test]
async fn hard_link_onaydan_sonra_eklenirse_duzenleme_yapilmaz() {
    let (d, a) = alan();
    std::fs::write(d.path().join("x.txt"), "eski").unwrap();
    let duzenle = DosyaDuzenle::yeni(a);
    let arg = json!({"yol": "x.txt", "eski_metin": "eski", "yeni_metin": "yeni"});
    let h = duzenle.hazirla(&arg).await.unwrap();
    std::fs::hard_link(d.path().join("x.txt"), d.path().join("alias.txt")).unwrap();

    assert_eq!(
        calistir(&duzenle, &arg, &h).await,
        Err(AracHatasi::OnayEskidi)
    );
    assert_eq!(oku(&d, "x.txt"), "eski");
    assert_eq!(oku(&d, "alias.txt"), "eski");
}

#[test]
fn varsayilan_araclar_kaydedilebilir() {
    let (_d, a) = alan();
    let araclar = varsayilan_araclar(a);
    let adlar: Vec<&str> = araclar.iter().map(|x| x.ad()).collect();
    assert_eq!(
        adlar,
        vec![
            "dosya_oku",
            "dosya_listele",
            "icerik_ara",
            "dosya_yaz",
            "dosya_duzenle"
        ]
    );
    let yazanlar: Vec<&str> = araclar
        .iter()
        .filter(|x| x.yazar_mi())
        .map(|x| x.ad())
        .collect();
    assert_eq!(yazanlar, vec!["dosya_yaz", "dosya_duzenle"]);
    AracKayitDefteri::yeni(araclar).expect("bütün şemalar geçerli olmalı");
}

/// Güvenlik analizi Bulgu 1: çekirdeğin metin kuralı `.git`'i adıyla tanır;
/// araç, git dizinine başka bir adla (8.3 kısa ad) yazmaz.
#[cfg(windows)]
#[test]
fn kisa_ad_ile_git_dizinine_yazilamaz() {
    use kalfa_core::yol::CalismaYolu;

    let (d, a) = alan();
    std::fs::create_dir(d.path().join(".git")).unwrap();
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
    let yol = CalismaYolu::coz(&format!("{kisa}/commondir")).unwrap();
    let hata = a.yaz(&yol, b"../zz").unwrap_err();
    assert_eq!(hata.kind(), std::io::ErrorKind::PermissionDenied, "{kisa}");
    // cap-std sondaki noktayı Win32 gibi atmaz; yol hiç çözülmez.
    assert!(
        a.yaz(&CalismaYolu::coz(".git./commondir").unwrap(), b"../zz")
            .is_err()
    );
    assert!(!d.path().join(".git/commondir").exists());
    // Kendi adıyla yazma (çekirdeğin onayından sonra) çalışır.
    a.yaz(&CalismaYolu::coz(".git/description").unwrap(), b"x")
        .unwrap();
    assert!(d.path().join(".git/description").exists());
}

// Aşama 3a: hazırlık geri alma için önceki içeriği ve sonraki özeti taşır
// (aşama 3 tasarımı §5.3).

#[tokio::test]
async fn yazma_hazirligi_onceki_icerigi_ve_sonraki_ozeti_tasir() {
    let (d, a) = alan();
    std::fs::write(d.path().join("x.txt"), "eski\n").unwrap();
    let yaz = DosyaYaz::yeni(a);
    let arg = json!({"yol": "x.txt", "icerik": "yeni\n"});
    let h = yaz.hazirla(&arg).await.unwrap();
    let hedef = &h.yazilan[0];
    assert_eq!(hedef.onceki_icerik.as_deref(), Some(&b"eski\n"[..]));
    assert_eq!(hedef.onceki_bayt, 5);
    assert!(!hedef.buyuk);
    assert_eq!(
        hedef.sonraki_ozet.as_deref(),
        Some(ozet_hex(b"yeni\n").as_str())
    );
    assert!(hedef.olusacak_dizinler.is_empty());
    assert!(hedef.geri_alinabilir());
}

#[tokio::test]
async fn yeni_dosyanin_hazirligi_olusacak_dizinleri_bildirir() {
    let (d, a) = alan();
    let yaz = DosyaYaz::yeni(a);
    let arg = json!({"yol": "a/b/c.txt", "icerik": "x"});
    let h = yaz.hazirla(&arg).await.unwrap();
    let hedef = &h.yazilan[0];
    let dizinler: Vec<&str> = hedef.olusacak_dizinler.iter().map(|y| y.as_str()).collect();
    assert_eq!(dizinler, ["a", "a/b"]);
    assert_eq!(hedef.onceki_icerik, None);
    assert_eq!(hedef.onceki_ozet, None);
    assert!(
        hedef.geri_alinabilir(),
        "yok olan dosya silinerek geri alınır"
    );

    std::fs::create_dir(d.path().join("a")).unwrap();
    let h = yaz.hazirla(&arg).await.unwrap();
    let dizinler: Vec<&str> = h.yazilan[0]
        .olusacak_dizinler
        .iter()
        .map(|y| y.as_str())
        .collect();
    assert_eq!(dizinler, ["a/b"]);
}

#[tokio::test]
async fn duzenleme_hazirligi_sonraki_ozeti_tasir() {
    let (d, a) = alan();
    std::fs::write(d.path().join("k.rs"), "fn a() {}\n").unwrap();
    let duzenle = DosyaDuzenle::yeni(a);
    let arg = json!({"yol": "k.rs", "eski_metin": "a()", "yeni_metin": "b()"});
    let h = duzenle.hazirla(&arg).await.unwrap();
    let hedef = &h.yazilan[0];
    assert_eq!(hedef.onceki_icerik.as_deref(), Some(&b"fn a() {}\n"[..]));
    assert_eq!(
        hedef.sonraki_ozet.as_deref(),
        Some(ozet_hex(b"fn b() {}\n").as_str())
    );
    assert!(hedef.olusacak_dizinler.is_empty());
    calistir(&duzenle, &arg, &h).await.unwrap();
    assert_eq!(
        ozet_hex(oku(&d, "k.rs").as_bytes()),
        hedef.sonraki_ozet.clone().unwrap()
    );
}

#[tokio::test]
async fn buyuk_dosyanin_onceki_icerigi_tasinmaz() {
    let (d, a) = alan();
    let boyut = kalfa_core::geri_alma::DOSYA_SINIRI as usize + 1;
    let veri = vec![b'a'; boyut];
    std::fs::write(d.path().join("buyuk.txt"), &veri).unwrap();
    let yaz = DosyaYaz::yeni(a.clone());
    let h = yaz
        .hazirla(&json!({"yol": "buyuk.txt", "icerik": "kisa"}))
        .await
        .unwrap();
    let hedef = &h.yazilan[0];
    assert!(hedef.buyuk);
    assert_eq!(hedef.onceki_icerik, None);
    assert_eq!(hedef.onceki_ozet.as_deref(), Some(ozet_hex(&veri).as_str()));
    assert_eq!(hedef.onceki_bayt, boyut as u64);
    assert!(!hedef.geri_alinabilir());

    // Sınırdaki dosya saklanır.
    let tam = vec![b'b'; kalfa_core::geri_alma::DOSYA_SINIRI as usize];
    std::fs::write(d.path().join("tam.txt"), &tam).unwrap();
    let h = yaz
        .hazirla(&json!({"yol": "tam.txt", "icerik": "kisa"}))
        .await
        .unwrap();
    assert!(!h.yazilan[0].buyuk);
    assert_eq!(
        h.yazilan[0].onceki_icerik.as_deref().map(<[u8]>::len),
        Some(tam.len())
    );
}

#[cfg(unix)]
#[tokio::test]
async fn baglantili_ust_dizinde_olusacak_dizin_hesaplanmaz() {
    let (d, a) = alan();
    let dis = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(dis.path(), d.path().join("bag")).unwrap();
    assert!(
        a.olusacak_dizinler(&kalfa_core::yol::CalismaYolu::coz("bag/yeni/x.txt").unwrap())
            .is_err()
    );
}
