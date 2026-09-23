use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use kalfa_core::gecmis::{OturumDurumu, oturumlari_listele};
use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::{OturumId, TurId};
use kalfa_core::protokol::{BitisNedeni, SEMA_SURUMU, Zarf};
use kalfa_evren::tipler::ChatMessage;

fn kayit_yaz(yol: &Path, ogeler: Vec<KayitOgesi>, zaman_sn: u64) {
    let oturum = OturumId::yeni();
    let tur = TurId::yeni();
    let mut k = OturumKaydi::olustur(yol).unwrap();
    for (i, yuk) in (1u64..).zip(ogeler) {
        k.yaz(&Zarf {
            sema: SEMA_SURUMU,
            oturum,
            tur: Some(tur),
            sira: i,
            yuk,
        })
        .unwrap();
    }
    drop(k);
    fs::File::options()
        .write(true)
        .open(yol)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(zaman_sn))
        .unwrap();
}

fn acilis() -> KayitOgesi {
    KayitOgesi::OturumAcildi {
        model: "glm-5.3".into(),
        kip: Kip::Siki,
        calisma_alani: Some("C:/proje".into()),
    }
}

fn kullanici(m: &str) -> KayitOgesi {
    KayitOgesi::Mesaj {
        mesaj: ChatMessage::user(m),
    }
}

fn bitti(neden: BitisNedeni) -> KayitOgesi {
    KayitOgesi::TurBitti { neden }
}

#[test]
fn liste_en_yeni_once_baslik_ve_durumla_gelir() {
    let d = tempfile::tempdir().unwrap();
    kayit_yaz(
        &d.path().join("a.jsonl"),
        vec![
            acilis(),
            kullanici("  \nBirinci iş\nayrıntı"),
            bitti(BitisNedeni::Normal),
        ],
        1_000,
    );
    let uzun = "u".repeat(100);
    kayit_yaz(
        &d.path().join("b.jsonl"),
        vec![acilis(), kullanici(&uzun)],
        2_000,
    );
    let liste = oturumlari_listele(d.path()).unwrap();
    assert_eq!(liste.len(), 2, "kilit dosyaları listeye girmez");
    assert_eq!(liste[0].yol, d.path().join("b.jsonl"));
    assert_eq!(liste[0].durum, OturumDurumu::YarimKaldi);
    let kisa = format!("{}···", "u".repeat(80));
    assert_eq!(liste[0].baslik.as_deref(), Some(kisa.as_str()));
    assert_eq!(liste[1].baslik.as_deref(), Some("Birinci iş"));
    assert_eq!(liste[1].durum, OturumDurumu::Tamamlandi);
    assert_eq!(
        liste[1].calisma_alani.as_deref(),
        Some(Path::new("C:/proje"))
    );
    assert_eq!(liste[1].son_degisiklik_ms, 1_000_000);
}

#[test]
fn bos_bozuk_ve_olmayan_dizin() {
    let d = tempfile::tempdir().unwrap();
    kayit_yaz(&d.path().join("bos.jsonl"), vec![acilis()], 1);
    fs::write(d.path().join("bozuk.jsonl"), "bu bir kayıt değil\n").unwrap();
    let liste = oturumlari_listele(d.path()).unwrap();
    let durum = |ad: &str| {
        liste
            .iter()
            .find(|o| o.yol.ends_with(ad))
            .unwrap()
            .durum
            .clone()
    };
    assert_eq!(durum("bos.jsonl"), OturumDurumu::Bos);
    assert!(matches!(
        durum("bozuk.jsonl"),
        OturumDurumu::Okunamadi { .. }
    ));
    assert!(
        oturumlari_listele(&d.path().join("yok"))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn eski_calisma_alani_olmayan_kayit_listeye_girer() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("eski.jsonl");
    fs::write(
        &yol,
        concat!(
            "{\"sema\":1,\"oturum\":\"6f1d0a4e-2b8c-4c1e-9a55-0d6b1f3e2a10\",\"tur\":null,\"sira\":1,\"yuk\":{\"tip\":\"OturumAcildi\",\"model\":\"glm-5.3\",\"kip\":\"Siki\"}}\n",
            "{\"sema\":1,\"oturum\":\"6f1d0a4e-2b8c-4c1e-9a55-0d6b1f3e2a10\",\"tur\":null,\"sira\":2,\"yuk\":{\"tip\":\"Komut\",\"komut\":{\"tip\":\"SartlariKabulEt\",\"surum\":1}}}\n"
        ),
    )
    .unwrap();

    let liste = oturumlari_listele(d.path()).unwrap();
    assert_eq!(liste.len(), 1);
    assert!(liste[0].oturum.is_some());
    assert_eq!(liste[0].calisma_alani, None);
    assert_eq!(liste[0].durum, OturumDurumu::Bos);
}

#[test]
fn buyuk_kayitta_son_durum_sondan_okunur() {
    let d = tempfile::tempdir().unwrap();
    let mut ogeler = vec![acilis(), kullanici("uzun iş")];
    ogeler.extend((0..3000).map(|_| KayitOgesi::KipDegisti { kip: Kip::Proje }));
    ogeler.push(bitti(BitisNedeni::Iptal));
    let yol = d.path().join("buyuk.jsonl");
    kayit_yaz(&yol, ogeler, 5);
    assert!(
        fs::metadata(&yol).unwrap().len() > 80 * 1024,
        "test kaydı baş ve son pencereden büyük olmalı"
    );
    let liste = oturumlari_listele(d.path()).unwrap();
    assert_eq!(liste[0].baslik.as_deref(), Some("uzun iş"));
    assert_eq!(liste[0].durum, OturumDurumu::IptalEdildi);
}

#[test]
fn buyuk_kayitta_eof_newline_olmayan_gecerli_son_satir_korunur() {
    let d = tempfile::tempdir().unwrap();
    let mut ogeler = vec![acilis(), kullanici("uzun iş")];
    ogeler.extend((0..3000).map(|_| KayitOgesi::KipDegisti { kip: Kip::Proje }));
    ogeler.push(bitti(BitisNedeni::Normal));
    let yol = d.path().join("newline-yok.jsonl");
    kayit_yaz(&yol, ogeler, 5);
    let mut veri = fs::read(&yol).unwrap();
    assert_eq!(veri.pop(), Some(b'\n'));
    fs::write(&yol, veri).unwrap();

    assert!(fs::metadata(&yol).unwrap().len() > 80 * 1024);
    let liste = oturumlari_listele(d.path()).unwrap();
    assert_eq!(liste[0].durum, OturumDurumu::Tamamlandi);
}

#[test]
fn buyuk_kayitta_eski_tur_bitisini_sondan_bloklayarak_bulur() {
    let d = tempfile::tempdir().unwrap();
    let mut ogeler = vec![acilis(), kullanici("uzun iş"), bitti(BitisNedeni::Normal)];
    ogeler.extend((0..3000).map(|_| KayitOgesi::KipDegisti { kip: Kip::Proje }));
    let yol = d.path().join("son-durum-eski.jsonl");
    kayit_yaz(&yol, ogeler, 5);

    assert!(fs::metadata(&yol).unwrap().len() > 80 * 1024);
    let liste = oturumlari_listele(d.path()).unwrap();
    assert_eq!(liste[0].durum, OturumDurumu::Tamamlandi);
}

#[test]
fn buyuk_kayitta_sondaki_bozuk_cokme_satiri_yoksayilir() {
    use std::io::Write;

    let d = tempfile::tempdir().unwrap();
    let mut ogeler = vec![acilis(), kullanici("uzun iş"), bitti(BitisNedeni::Normal)];
    ogeler.extend((0..3000).map(|_| KayitOgesi::KipDegisti { kip: Kip::Proje }));
    let yol = d.path().join("bozuk-son.jsonl");
    kayit_yaz(&yol, ogeler, 5);
    let mut dosya = fs::OpenOptions::new().append(true).open(&yol).unwrap();
    dosya.write_all(b"{yarim json\n  \n").unwrap();

    assert!(fs::metadata(&yol).unwrap().len() > 80 * 1024);
    let liste = oturumlari_listele(d.path()).unwrap();
    assert_eq!(liste[0].durum, OturumDurumu::Tamamlandi);
}

#[test]
fn buyuk_acilis_satiri_16_kib_penceresini_genisletir() {
    let d = tempfile::tempdir().unwrap();
    let acilis_uzun = KayitOgesi::OturumAcildi {
        model: "m".repeat(20 * 1024),
        kip: Kip::Siki,
        calisma_alani: Some("C:/proje".into()),
    };
    let mut ogeler = vec![acilis_uzun, kullanici("başlık"), bitti(BitisNedeni::Normal)];
    ogeler.extend((0..3000).map(|_| KayitOgesi::KipDegisti { kip: Kip::Proje }));
    let yol = d.path().join("uzun-acilis.jsonl");
    kayit_yaz(&yol, ogeler, 5);

    assert!(fs::metadata(&yol).unwrap().len() > 80 * 1024);
    let liste = oturumlari_listele(d.path()).unwrap();
    assert_eq!(
        liste[0].calisma_alani.as_deref(),
        Some(Path::new("C:/proje"))
    );
    assert_eq!(liste[0].baslik.as_deref(), Some("başlık"));
    assert_eq!(liste[0].durum, OturumDurumu::Tamamlandi);
}
