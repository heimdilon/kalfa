//! Yalıtımın uygulama tarafı (aşama 2 tasarımı §8.3, §10.3–§10.5): durumun
//! ana pencereye yayılması, araç zinciri dizinleri.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use kalfa_core::test_destek::SahteSaglayici;
use kalfa_sandbox::{
    BaslatmaHatasi, CalistirmaIstegi, Surec, Yalitici, YalitimDurumu, YalitimYoklukNedeni,
    Yalitimsiz,
};
use kalfa_uygulama::hata::HataKodu;
use kalfa_uygulama::test_destek::{SahteYalitimYazici, TestUygulamasi, test_uygulamasi_yalitimla};
use kalfa_uygulama::yalitim::{Degisince, YalitimKaynagi};

/// Durumu testin verdiği sıradaki değere geçen yalıtıcı.
struct DegisenYalitici {
    durum: Mutex<YalitimDurumu>,
    sonraki: Mutex<Option<YalitimDurumu>>,
    degisince: Mutex<Option<Degisince>>,
    dizinler: Mutex<Vec<PathBuf>>,
}

impl Yalitici for DegisenYalitici {
    fn durum(&self) -> YalitimDurumu {
        self.durum.lock().unwrap().clone()
    }

    fn baslat(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi> {
        Yalitimsiz.baslat(istek)
    }

    fn yeniden_algila(&self) -> YalitimDurumu {
        if let Some(yeni) = self.sonraki.lock().unwrap().take() {
            let degisti = *self.durum.lock().unwrap() != yeni;
            *self.durum.lock().unwrap() = yeni.clone();
            if degisti && let Some(f) = self.degisince.lock().unwrap().as_ref() {
                f(&yeni);
            }
        }
        self.durum()
    }

    fn okunur_dizinleri_ayarla(&self, dizinler: Vec<PathBuf>) {
        *self.dizinler.lock().unwrap() = dizinler;
    }
}

fn yok() -> YalitimDurumu {
    YalitimDurumu::Yok {
        neden: YalitimYoklukNedeni::BwrapYok,
        talimat: "bubblewrap kur".into(),
    }
}

fn hazir() -> YalitimDurumu {
    YalitimDurumu::Hazir {
        arka_uc: "bwrap".into(),
        surum: "0.9.0".into(),
    }
}

fn kur(dizin: &Path) -> (TestUygulamasi, Arc<DegisenYalitici>) {
    let sahte = Arc::new(DegisenYalitici {
        durum: Mutex::new(yok()),
        sonraki: Mutex::new(None),
        degisince: Mutex::new(None),
        dizinler: Mutex::new(Vec::new()),
    });
    let kurulan = sahte.clone();
    let t = test_uygulamasi_yalitimla(
        dizin,
        SahteSaglayici::yeni(Vec::new()),
        Some("evren_llm_a"),
        YalitimKaynagi::Kurucu(Box::new(move |degisince| {
            *kurulan.degisince.lock().unwrap() = Some(degisince);
            kurulan as Arc<dyn Yalitici>
        })),
    );
    (t, sahte)
}

#[tokio::test]
async fn yalitim_durumu_degisince_ana_pencerenin_kanalina_yazilir() {
    let d = tempfile::tempdir().unwrap();
    let (t, sahte) = kur(d.path());
    let durum = t.uygulama.uygulama_durumu();
    assert_eq!(durum.yalitim, yok());
    assert_eq!(durum.platform, std::env::consts::OS);

    // Kanal verilince şimdiki durum hemen yazılır.
    let kanal = SahteYalitimYazici::default();
    t.uygulama.yalitimi_dinle(Box::new(kanal.clone()));
    assert_eq!(kanal.yazilanlar(), vec![yok()]);

    // Değişmeyen durum yazılmaz.
    assert_eq!(t.uygulama.yalitimi_algila().await.unwrap(), yok());
    assert_eq!(kanal.yazilanlar().len(), 1);

    *sahte.sonraki.lock().unwrap() = Some(hazir());
    assert_eq!(t.uygulama.yalitimi_algila().await.unwrap(), hazir());
    assert_eq!(kanal.yazilanlar(), vec![yok(), hazir()]);
    assert_eq!(t.uygulama.uygulama_durumu().yalitim, hazir());

    // Yeniden yüklenen pencerenin kanalı eskisinin yerini alır.
    let yeni = SahteYalitimYazici::default();
    t.uygulama.yalitimi_dinle(Box::new(yeni.clone()));
    *sahte.sonraki.lock().unwrap() = Some(yok());
    t.uygulama.yalitimi_algila().await.unwrap();
    assert_eq!(yeni.yazilanlar(), vec![hazir(), yok()]);
    assert_eq!(kanal.yazilanlar().len(), 2);
}

#[tokio::test]
async fn arac_zinciri_dizini_klasor_seciciyle_eklenir_kalir_ve_kaldirilir() {
    let d = tempfile::tempdir().unwrap();
    let veri = d.path().join("veri");
    let arac = d.path().join("araclar/node");
    std::fs::create_dir_all(&arac).unwrap();
    let arac = arac.canonicalize().unwrap();
    let (t, sahte) = kur(&veri);
    assert!(t.uygulama.arac_zinciri_dizinleri().is_empty());

    // Vazgeçilirse liste değişmez.
    assert!(
        t.uygulama
            .arac_zinciri_dizini_ekle()
            .await
            .unwrap()
            .is_empty()
    );

    t.iletisim.klasor_ver(&arac);
    let beklenen = vec![arac.display().to_string()];
    assert_eq!(
        t.uygulama.arac_zinciri_dizini_ekle().await.unwrap(),
        beklenen
    );
    // Aynı dizin iki kez eklenmez.
    assert_eq!(
        t.uygulama.arac_zinciri_dizini_ekle().await.unwrap(),
        beklenen
    );
    assert_eq!(*sahte.dizinler.lock().unwrap(), vec![arac.clone()]);

    // Kayıt kalıcıdır: aynı veri diziniyle açılan uygulama okur.
    drop(t);
    let (t, _) = kur(&veri);
    assert_eq!(t.uygulama.arac_zinciri_dizinleri(), beklenen);

    let kalan = t
        .uygulama
        .arac_zinciri_dizini_kaldir(&arac.display().to_string())
        .unwrap();
    assert!(kalan.is_empty());
    assert!(
        t.uygulama
            .arac_zinciri_dizini_kaldir("/olmayan")
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn ev_ya_da_veri_dizinini_kapsayan_dizin_eklenemez() {
    let d = tempfile::tempdir().unwrap();
    // Ev dizini `<veri>/ev`dir (test_destek); veri dizini `d/veri`.
    let veri = d.path().join("veri");
    std::fs::create_dir_all(veri.join("ev")).unwrap();
    std::fs::create_dir_all(veri.join("yalitim/abc")).unwrap();
    let (t, sahte) = kur(&veri);
    for secilen in [
        d.path().to_path_buf(),
        veri.clone(),
        veri.join("ev"),
        veri.join("yalitim/abc"),
    ] {
        t.iletisim.klasor_ver(&secilen);
        let hata = t.uygulama.arac_zinciri_dizini_ekle().await.unwrap_err();
        assert_eq!(hata.kod, HataKodu::GecersizGirdi, "{}", secilen.display());
    }
    assert!(t.uygulama.arac_zinciri_dizinleri().is_empty());
    assert!(sahte.dizinler.lock().unwrap().is_empty());
}
