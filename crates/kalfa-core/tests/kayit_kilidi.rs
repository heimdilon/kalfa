use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;

use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi, kilit_yolu};
use kalfa_core::kimlik::OturumId;
use kalfa_core::oturum::{OturumAyarlari, OturumHatasi, devam_et};
use kalfa_core::protokol::{BitisNedeni, Komut, SEMA_SURUMU, Zarf};
use kalfa_core::test_destek::*;

const COCUK_DEGISKENI: &str = "KALFA_KAYIT_KILIDI_COCUK";

fn kip_kaydi() -> Zarf<KayitOgesi> {
    Zarf {
        sema: SEMA_SURUMU,
        oturum: OturumId::yeni(),
        tur: None,
        sira: 1,
        yuk: KayitOgesi::KipDegisti { kip: Kip::Proje },
    }
}

#[test]
fn ayni_surecte_ikinci_yazici_acilamaz() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("o.jsonl");
    let ilk = OturumKaydi::olustur(&yol).unwrap();
    let hata = OturumKaydi::ac(&yol)
        .err()
        .expect("ikinci yazıcı açılmamalı");
    assert_eq!(hata.kind(), ErrorKind::ResourceBusy);
    drop(ilk);
    OturumKaydi::ac(&yol).expect("ilk yazıcı kapanınca açılabilmeli");
}

#[test]
fn kilit_tutulurken_kayit_okunabilir() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("o.jsonl");
    let mut k = OturumKaydi::olustur(&yol).unwrap();
    k.yaz(&kip_kaydi()).unwrap();
    assert_eq!(OturumKaydi::oku(&yol).unwrap().len(), 1);
}

#[test]
fn olmayan_kayit_icin_kilit_dosyasi_birakilmaz() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("yok.jsonl");
    let hata = OturumKaydi::devam_icin_ac(&yol)
        .err()
        .expect("olmayan kayıt açılmamalı");
    assert_eq!(hata.kind(), ErrorKind::NotFound);
    assert!(!kilit_yolu(&yol).exists());
}

#[test]
fn baska_surecte_ikinci_yazici_acilamaz() {
    if let Ok(yol) = std::env::var(COCUK_DEGISKENI) {
        // Çocuk süreç: kilit ebeveynde olduğu için açılamamalı.
        let kod = match OturumKaydi::ac(Path::new(&yol)) {
            Err(e) if e.kind() == ErrorKind::ResourceBusy => 0,
            _ => 1,
        };
        std::process::exit(kod);
    }
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("o.jsonl");
    let _kayit = OturumKaydi::olustur(&yol).unwrap();
    let durum = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "baska_surecte_ikinci_yazici_acilamaz",
            "--test-threads=1",
        ])
        .env(COCUK_DEGISKENI, &yol)
        .status()
        .unwrap();
    assert!(
        durum.success(),
        "başka süreçteki ikinci yazıcı reddedilmeli"
    );
}

#[cfg(unix)]
#[test]
fn sembolik_baglanti_ayni_sidecari_kullanir() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("o.jsonl");
    let ilk = OturumKaydi::olustur(&yol).unwrap();
    let bag = d.path().join("bag.jsonl");
    std::os::unix::fs::symlink(&yol, &bag).unwrap();

    assert_eq!(kilit_yolu(&yol), kilit_yolu(&bag));
    assert_eq!(
        OturumKaydi::ac(&bag).err().unwrap().kind(),
        ErrorKind::ResourceBusy
    );
    drop(ilk);
}

#[cfg(unix)]
#[test]
fn sembolik_baglanti_kilit_uzantili_kaydi_atlayamaz() {
    let d = tempfile::tempdir().unwrap();
    let kilit_uzantili = d.path().join("asıl.jsonl.kilit");
    std::fs::write(&kilit_uzantili, b"not a record").unwrap();
    let bag = d.path().join("bag.jsonl");
    std::os::unix::fs::symlink(&kilit_uzantili, &bag).unwrap();

    let hata = OturumKaydi::ac(&bag).err().unwrap();
    assert_eq!(hata.kind(), ErrorKind::InvalidInput);
    assert!(!d.path().join("asıl.jsonl.kilit.kilit").exists());
}

#[cfg(windows)]
#[test]
fn junction_ebeveyn_yolu_ayni_sidecari_kullanir() {
    let d = tempfile::tempdir().unwrap();
    let hedef = d.path().join("hedef");
    std::fs::create_dir(&hedef).unwrap();
    let bag = d.path().join("bag");
    let cikti = std::process::Command::new("cmd.exe")
        .args(["/D", "/C", "mklink", "/J"])
        .arg(&bag)
        .arg(&hedef)
        .output()
        .unwrap();
    assert!(cikti.status.success(), "junction kurulamadı: {cikti:?}");

    let yol = hedef.join("o.jsonl");
    let bag_yolu = bag.join("o.jsonl");
    let ilk = OturumKaydi::olustur(&yol).unwrap();
    assert_eq!(kilit_yolu(&yol), kilit_yolu(&bag_yolu));
    assert_eq!(
        OturumKaydi::ac(&bag_yolu).err().unwrap().kind(),
        ErrorKind::ResourceBusy
    );
    drop(ilk);
}

#[test]
fn hardlink_kaydi_acik_hata_ile_reddedilir() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("o.jsonl");
    let ilk = OturumKaydi::olustur(&yol).unwrap();
    let bag = d.path().join("bag.jsonl");
    std::fs::hard_link(&yol, &bag).unwrap();

    let hata = OturumKaydi::ac(&bag).err().unwrap();
    assert_eq!(hata.kind(), ErrorKind::Unsupported);
    drop(ilk);
}

#[test]
fn kilit_uzantili_kayit_adi_reddedilir() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("o.jsonl.kilit");
    let hata = OturumKaydi::olustur(&yol).err().unwrap();
    assert_eq!(hata.kind(), ErrorKind::InvalidInput);
    assert!(!yol.exists());
}

#[tokio::test]
async fn oturum_kapaninca_kayit_yeniden_acilabilir() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("oturum.jsonl");
    let mut ayar = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    ayar.kayit_yolu = Some(yol.clone());
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("tamam")]);
    let mut surucu = test_oturumu(
        test_baglami(s.clone(), vec![], sanal_siniflandirici()),
        ayar.clone(),
    );
    surucu
        .gonder(Komut::KullaniciMesaji {
            metin: "selam".into(),
        })
        .await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let acikken = devam_et(
        &yol,
        ayar.clone(),
        test_baglami(s.clone(), vec![], sanal_siniflandirici()),
    );
    assert!(
        matches!(acikken, Err(OturumHatasi::Kayit(e)) if e.kind() == ErrorKind::ResourceBusy),
        "açık oturumun kaydına ikinci yazıcı açılmaz"
    );

    surucu.kapat().await;
    let (_tutamac, _alici, durum) =
        devam_et(&yol, ayar, test_baglami(s, vec![], sanal_siniflandirici()))
            .expect("kapanan oturumun kaydı yeniden açılmalı");
    assert_eq!(durum.model, "glm-5.3");
}
