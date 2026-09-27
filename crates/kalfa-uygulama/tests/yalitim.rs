//! Yalıtımın uygulama tarafı (aşama 2 tasarımı §8.3, §10.3–§10.5): durumun
//! ana pencereye yayılması, araç zinciri dizinleri.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier, Mutex};

use kalfa_core::test_destek::SahteSaglayici;
use kalfa_sandbox::{
    BaslatmaHatasi, CalistirmaIstegi, HazirAlan, Surec, Yalitici, YalitimDurumu,
    YalitimYoklukNedeni, Yalitimsiz,
};
use kalfa_uygulama::calisma_alanlari::CalismaAlaniKaydi;
use kalfa_uygulama::hata::HataKodu;
use kalfa_uygulama::sozlesme::{OnarimSonucu, YalitimCalismaAlani};
use kalfa_uygulama::test_destek::{SahteYalitimYazici, TestUygulamasi, test_uygulamasi_yalitimla};
use kalfa_uygulama::yalitim::{Degisince, YalitimKaynagi};

/// Durumu testin verdiği sıradaki değere geçen yalıtıcı.
struct DegisenYalitici {
    durum: Mutex<YalitimDurumu>,
    sonraki: Mutex<Option<YalitimDurumu>>,
    degisince: Mutex<Option<Degisince>>,
    dizinler: Mutex<Vec<PathBuf>>,
    hazir_alanlar: Mutex<Vec<HazirAlan>>,
    kaldirilan_alanlar: Mutex<Vec<String>>,
    kaldirma_esigi: Mutex<Option<Arc<(Barrier, Barrier)>>>,
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

    fn hazirlanmis_alanlar(&self) -> Vec<HazirAlan> {
        self.hazir_alanlar.lock().unwrap().clone()
    }

    fn alani_kaldir_kimlikle(&self, kimlik: &str) -> Result<(), String> {
        if let Some(esik) = self.kaldirma_esigi.lock().unwrap().clone() {
            esik.0.wait();
            esik.1.wait();
        }
        let mut alanlar = self.hazir_alanlar.lock().unwrap();
        let Some(sira) = alanlar.iter().position(|alan| alan.kimlik == kimlik) else {
            return Err("Hazırlanmış çalışma alanı bulunamadı".into());
        };
        alanlar.remove(sira);
        self.kaldirilan_alanlar
            .lock()
            .unwrap()
            .push(kimlik.to_string());
        Ok(())
    }
}

fn yok() -> YalitimDurumu {
    YalitimDurumu::Yok {
        neden: YalitimYoklukNedeni::BwrapYok,
        talimat: "bubblewrap kur".into(),
        ayrinti: "denetim=bwrap-yolu".into(),
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
        hazir_alanlar: Mutex::new(Vec::new()),
        kaldirilan_alanlar: Mutex::new(Vec::new()),
        kaldirma_esigi: Mutex::new(None),
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

fn hazir_alan_ekle(yalitici: &DegisenYalitici, kok: &Path) -> String {
    let kok = kok.canonicalize().unwrap();
    let kimlik = CalismaAlaniKaydi::kimlik(&kok);
    *yalitici.hazir_alanlar.lock().unwrap() = vec![HazirAlan {
        kimlik: kimlik.clone(),
        kok,
    }];
    kimlik
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

/// Aşama 2 tasarımı §9.8: yalıtımsız çalıştırma varsayılan kapalıdır,
/// açılınca kalıcıdır; kapatmak onay istemez.
#[tokio::test]
async fn yalitimsiz_anahtari_varsayilan_kapali_ve_kalici() {
    let d = tempfile::tempdir().unwrap();
    {
        let (t, _) = kur(d.path());
        assert!(!t.uygulama.uygulama_durumu().yalitimsiz_izinli);
        t.iletisim.onay_ver(true);
        assert!(t.uygulama.yalitimsizi_ayarla(true).await.unwrap());
        assert!(t.uygulama.uygulama_durumu().yalitimsiz_izinli);
    }
    let (t, _) = kur(d.path());
    assert!(t.uygulama.uygulama_durumu().yalitimsiz_izinli);
    t.iletisim.onay_ver(false);
    let sorulan = t.iletisim.sorulanlar().len();
    assert!(!t.uygulama.yalitimsizi_ayarla(false).await.unwrap());
    assert_eq!(
        t.iletisim.sorulanlar().len(),
        sorulan,
        "kapatmak onay istemez"
    );
    assert!(!t.uygulama.uygulama_durumu().yalitimsiz_izinli);
}

#[tokio::test]
async fn yalitimsiz_acmak_yerel_onay_ister_vazgecince_kapali_kalir() {
    let d = tempfile::tempdir().unwrap();
    let (t, _) = kur(d.path());
    t.iletisim.onay_ver(false);
    assert!(!t.uygulama.yalitimsizi_ayarla(true).await.unwrap());
    assert!(!t.uygulama.uygulama_durumu().yalitimsiz_izinli);
    let sorulan = t.iletisim.sorulanlar().pop().unwrap();
    assert!(
        sorulan.starts_with("Yalıtımsız çalıştırmayı aç: "),
        "{sorulan}"
    );
}

/// Anahtarı değiştirmek araç zinciri dizinlerini silmez (aynı dosya).
#[tokio::test]
async fn yalitimsiz_anahtari_arac_zinciri_dizinlerini_korur() {
    let d = tempfile::tempdir().unwrap();
    let veri = d.path().join("veri");
    let arac = d.path().join("arac");
    std::fs::create_dir(&arac).unwrap();
    let (t, _) = kur(&veri);
    t.iletisim.klasor_ver(&arac);
    t.uygulama.arac_zinciri_dizini_ekle().await.unwrap();
    t.iletisim.onay_ver(true);
    t.uygulama.yalitimsizi_ayarla(true).await.unwrap();
    drop(t);
    let (t, _) = kur(&veri);
    assert_eq!(t.uygulama.arac_zinciri_dizinleri().len(), 1);
    assert!(t.uygulama.uygulama_durumu().yalitimsiz_izinli);
}

/// BT için teknik rapor (aşama 2 tasarımı §9.8): sürümler, durum, düşen
/// denetim, talimat ve platformun gereksinimi.
#[tokio::test]
async fn teknik_rapor_surum_durum_ve_gereksinimi_icerir() {
    let d = tempfile::tempdir().unwrap();
    let (t, _) = kur(d.path());
    let rapor = t.uygulama.yalitim_teknik_raporu();
    for parca in [
        "Kalfa sürümü: ",
        "İşletim sistemi: ",
        "Durum: Yok (BwrapYok)",
        "Ayrıntı: denetim=bwrap-yolu",
        "Talimat: bubblewrap kur",
        "Yalıtımsız çalıştırma: kapalı",
        "Gereksinim:",
    ] {
        assert!(rapor.contains(parca), "{parca} yok:\n{rapor}");
    }
    assert!(
        rapor.contains(&kalfa_sandbox::sistem_surumu()),
        "işletim sistemi sürümü yazılmalı:\n{rapor}"
    );
    assert!(rapor.contains(&kalfa_sandbox::gereksinim()), "{rapor}");
}

#[tokio::test]
async fn yalitim_calisma_alanlari_kimlik_ve_yol_doner() {
    let d = tempfile::tempdir().unwrap();
    let kok = d.path().join("proje");
    std::fs::create_dir(&kok).unwrap();
    let kok = kok.canonicalize().unwrap();
    let (t, sahte) = kur(d.path());
    let kimlik = hazir_alan_ekle(&sahte, &kok);

    assert_eq!(
        t.uygulama.yalitim_calisma_alanlari(),
        vec![YalitimCalismaAlani {
            kimlik,
            yol: kok.display().to_string(),
        }]
    );
}

#[tokio::test]
async fn alan_kaldirma_yerel_onay_ister_ve_vazgecilince_izinleri_tutar() {
    let d = tempfile::tempdir().unwrap();
    let kok = d.path().join("proje");
    std::fs::create_dir(&kok).unwrap();
    let (t, sahte) = kur(d.path());
    let kimlik = hazir_alan_ekle(&sahte, &kok);

    t.iletisim.onay_ver(false);
    assert!(
        !t.uygulama
            .yalitim_calisma_alanini_kaldir(&kimlik)
            .await
            .unwrap()
    );
    assert!(sahte.kaldirilan_alanlar.lock().unwrap().is_empty());
    assert_eq!(t.uygulama.yalitim_calisma_alanlari().len(), 1);
    assert!(t.iletisim.sorulanlar()[0].starts_with("Çalışma alanını kaldır: "));

    t.iletisim.onay_ver(true);
    assert!(
        t.uygulama
            .yalitim_calisma_alanini_kaldir(&kimlik)
            .await
            .unwrap()
    );
    assert_eq!(*sahte.kaldirilan_alanlar.lock().unwrap(), vec![kimlik]);
    assert!(t.uygulama.yalitim_calisma_alanlari().is_empty());
}

#[tokio::test]
async fn alan_kaldirma_acik_oturumu_olan_calisma_alanini_reddeder() {
    let d = tempfile::tempdir().unwrap();
    let kok = d.path().join("proje");
    std::fs::create_dir(&kok).unwrap();
    let (t, sahte) = kur(d.path());
    let kimlik = hazir_alan_ekle(&sahte, &kok);

    t.iletisim.klasor_ver(&kok);
    let secilen = t.uygulama.calisma_alani_sec().await.unwrap();
    let acilis = t
        .uygulama
        .oturum_baslat(
            secilen.kimlik,
            kalfa_uygulama::modeller::VARSAYILAN_MODEL.to_string(),
        )
        .await
        .unwrap();

    let hata = t
        .uygulama
        .yalitim_calisma_alanini_kaldir(&kimlik)
        .await
        .unwrap_err();
    assert_eq!(hata.kod, HataKodu::GecersizGirdi);
    assert!(hata.mesaj.contains("açık oturum"));
    assert!(
        t.iletisim.sorulanlar().is_empty(),
        "onay penceresi açılmamalı"
    );
    assert!(sahte.kaldirilan_alanlar.lock().unwrap().is_empty());

    t.uygulama.oturumu_kapat(acilis.oturum).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn alan_kaldirma_rezervasyonu_eszamanli_oturum_baslatmayi_durdurur() {
    let d = tempfile::tempdir().unwrap();
    let kok = d.path().join("proje");
    std::fs::create_dir(&kok).unwrap();
    let (t, sahte) = kur(d.path());
    let kimlik = hazir_alan_ekle(&sahte, &kok);
    let esik = Arc::new((Barrier::new(2), Barrier::new(2)));
    *sahte.kaldirma_esigi.lock().unwrap() = Some(esik.clone());
    t.iletisim.onay_ver(true);

    let uygulama = t.uygulama.clone();
    let silme_kimligi = kimlik.clone();
    let kaldirma = tokio::spawn(async move {
        uygulama
            .yalitim_calisma_alanini_kaldir(&silme_kimligi)
            .await
    });
    // Kaldırma, uygulama durumunda rezervasyon aldıktan sonra sahte yalıtıcıya
    // ulaşır. İkinci bariyer tamamlanana kadar profil silinmez.
    esik.0.wait();

    t.iletisim.klasor_ver(&kok);
    let secilen = t.uygulama.calisma_alani_sec().await.unwrap();
    let hata = t
        .uygulama
        .oturum_baslat(
            secilen.kimlik,
            kalfa_uygulama::modeller::VARSAYILAN_MODEL.to_string(),
        )
        .await
        .unwrap_err();
    assert_eq!(hata.kod, HataKodu::GecersizGirdi);
    assert!(hata.mesaj.contains("kaldırılıyor"));
    assert_eq!(t.uygulama.acik_oturum_sayisi(), 0);

    esik.1.wait();
    assert!(kaldirma.await.unwrap().unwrap());
    assert!(t.uygulama.yalitim_calisma_alanlari().is_empty());
}

// --- Onar (aşama 2 tasarımı §9.8) ---

#[tokio::test]
async fn onar_kurulu_surum_degilse_talimat_doner_ve_kur_cagrilmaz() {
    let d = tempfile::tempdir().unwrap();
    let (t, _) = kur(d.path());
    t.yukseltici
        .kurulu
        .store(false, std::sync::atomic::Ordering::SeqCst);
    t.iletisim.onay_ver(true);
    match t.uygulama.yalitimi_onar().await.unwrap() {
        OnarimSonucu::KuruluDegil { talimat } => {
            assert!(talimat.contains("kalfa-yalitim.exe kur"), "{talimat}")
        }
        d => panic!("{d:?}"),
    }
    assert_eq!(t.yukseltici.cagri_sayisi(), 0);
    assert!(t.iletisim.sorulanlar().is_empty(), "onay sorulmaz");
}

#[tokio::test]
async fn onar_yerel_pencerede_vazgecilirse_kur_calismaz() {
    let d = tempfile::tempdir().unwrap();
    let (t, _) = kur(d.path());
    t.iletisim.onay_ver(false);
    assert_eq!(
        t.uygulama.yalitimi_onar().await.unwrap(),
        OnarimSonucu::Vazgecildi
    );
    assert_eq!(t.yukseltici.cagri_sayisi(), 0);
    assert!(
        t.iletisim.sorulanlar()[0].starts_with("Yalıtımı onar: "),
        "{:?}",
        t.iletisim.sorulanlar()
    );
}

#[tokio::test]
async fn onar_uacde_vazgecilirse_durum_degismez() {
    let d = tempfile::tempdir().unwrap();
    let (t, sahte) = kur(d.path());
    *sahte.sonraki.lock().unwrap() = Some(hazir());
    *t.yukseltici.sonuc.lock().unwrap() = Ok(None);
    t.iletisim.onay_ver(true);
    assert_eq!(
        t.uygulama.yalitimi_onar().await.unwrap(),
        OnarimSonucu::Vazgecildi
    );
    assert_eq!(t.yukseltici.cagri_sayisi(), 1);
    assert_eq!(t.uygulama.uygulama_durumu().yalitim, yok());
}

#[tokio::test]
async fn onar_basarida_yeniden_algilar_ve_yeni_durumu_doner() {
    let d = tempfile::tempdir().unwrap();
    let (t, sahte) = kur(d.path());
    *sahte.sonraki.lock().unwrap() = Some(hazir());
    t.iletisim.onay_ver(true);
    assert_eq!(
        t.uygulama.yalitimi_onar().await.unwrap(),
        OnarimSonucu::Tamam { yalitim: hazir() }
    );
    assert_eq!(t.uygulama.uygulama_durumu().yalitim, hazir());
}

#[tokio::test]
async fn onar_kur_hata_koduyla_biterse_basarisiz_doner() {
    let d = tempfile::tempdir().unwrap();
    let (t, _) = kur(d.path());
    *t.yukseltici.sonuc.lock().unwrap() = Ok(Some(1));
    t.iletisim.onay_ver(true);
    match t.uygulama.yalitimi_onar().await.unwrap() {
        OnarimSonucu::Basarisiz { mesaj } => assert!(mesaj.contains("1 koduyla"), "{mesaj}"),
        d => panic!("{d:?}"),
    }
}
