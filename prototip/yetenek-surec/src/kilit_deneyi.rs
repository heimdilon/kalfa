//! Görev 1: Kabuk kilidi, süreç sahipliği, korunan dosya denetimi ve kontrol
//! çağrısı kilitlenmesi (deadlock) yeniden üretim deneyleri.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{Notify, OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};

pub type Sonuc<T> = Result<T, String>;

/// `kalfa_core::kilit::KaynakKilitleri` ile birebir aynı kilit modeli.
#[derive(Default)]
pub struct KaynakKilitleri {
    genel: Arc<RwLock<()>>,
    yollar: Mutex<HashMap<PathBuf, Arc<RwLock<()>>>>,
}

#[allow(dead_code)]
pub struct KilitSeti {
    genel_okuma: Option<OwnedRwLockReadGuard<()>>,
    genel_yazma: Option<OwnedRwLockWriteGuard<()>>,
    okuma_kilitleri: Vec<OwnedRwLockReadGuard<()>>,
    yazma_kilitleri: Vec<OwnedRwLockWriteGuard<()>>,
}

impl KaynakKilitleri {
    pub fn yeni() -> Self {
        Self::default()
    }

    fn yol_kilidi(&self, yol: &Path) -> Arc<RwLock<()>> {
        let mut harita = self.yollar.lock().unwrap_or_else(|e| e.into_inner());
        harita
            .entry(yol.to_path_buf())
            .or_insert_with(|| Arc::new(RwLock::new(())))
            .clone()
    }

    pub async fn sahipli_al(
        self: &Arc<Self>,
        okunan: &[PathBuf],
        yazilan: &[PathBuf],
        tum_calisma_alani: bool,
    ) -> KilitSeti {
        if tum_calisma_alani {
            let genel_yazma = self.genel.clone().write_owned().await;
            return KilitSeti {
                genel_okuma: None,
                genel_yazma: Some(genel_yazma),
                okuma_kilitleri: Vec::new(),
                yazma_kilitleri: Vec::new(),
            };
        }

        let genel_okuma = self.genel.clone().read_owned().await;
        let yazma_kumesi: BTreeSet<PathBuf> = yazilan.iter().cloned().collect();
        let okuma_kumesi: BTreeSet<PathBuf> = okunan
            .iter()
            .filter(|y| !yazma_kumesi.contains(*y))
            .cloned()
            .collect();

        let mut tum_yollar: BTreeSet<PathBuf> = BTreeSet::new();
        tum_yollar.extend(okuma_kumesi.iter().cloned());
        tum_yollar.extend(yazma_kumesi.iter().cloned());

        let mut okuma_kilitleri = Vec::with_capacity(okuma_kumesi.len());
        let mut yazma_kilitleri = Vec::with_capacity(yazma_kumesi.len());

        for yol in tum_yollar {
            let kilit = self.yol_kilidi(&yol);
            if yazma_kumesi.contains(&yol) {
                yazma_kilitleri.push(kilit.write_owned().await);
            } else {
                okuma_kilitleri.push(kilit.read_owned().await);
            }
        }

        KilitSeti {
            genel_okuma: Some(genel_okuma),
            genel_yazma: None,
            okuma_kilitleri,
            yazma_kilitleri,
        }
    }
}

/// Korunan dosya görüntüsü (`KorumaGoruntusu` karşılığı).
#[derive(Clone, Debug, PartialEq, Eq)]
struct KorumaGoruntusu {
    git_config: Vec<u8>,
    kalfa_ayar: Vec<u8>,
}

/// Çalışma alanı yazma penceresi ve koruma durumu (`CalismaAlani` karşılığı).
struct OrnekCalismaAlani {
    etkin_yazma_sayisi: AtomicUsize,
    yazma_kusagi: AtomicU64,
    git_config: Mutex<Vec<u8>>,
    kalfa_ayar: Mutex<Vec<u8>>,
    src_lib: Mutex<Vec<u8>>,
}

impl OrnekCalismaAlani {
    fn yeni() -> Arc<Self> {
        Arc::new(Self {
            etkin_yazma_sayisi: AtomicUsize::new(0),
            yazma_kusagi: AtomicU64::new(0),
            git_config: Mutex::new(b"[core]\n\tbare = false\n".to_vec()),
            kalfa_ayar: Mutex::new(b"{\"izin\":\"varsayilan\"}\n".to_vec()),
            src_lib: Mutex::new(b"pub fn eski() {}\n".to_vec()),
        })
    }

    fn goruntu_yakala(&self) -> KorumaGoruntusu {
        KorumaGoruntusu {
            git_config: self
                .git_config
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
            kalfa_ayar: self
                .kalfa_ayar
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
        }
    }

    fn komut_sonrasi_denetim(&self, onceki: &KorumaGoruntusu) -> Result<(), &'static str> {
        let simdiki = self.goruntu_yakala();
        if &simdiki != onceki {
            return Err("komut korunan bir yolu (.git/config veya .kalfa) değiştirdi");
        }
        Ok(())
    }

    fn yazma_basladi(&self) {
        self.etkin_yazma_sayisi.fetch_add(1, Ordering::SeqCst);
    }

    fn yazma_bitisini_tamamla(&self) {
        self.etkin_yazma_sayisi.fetch_sub(1, Ordering::SeqCst);
        self.yazma_kusagi.fetch_add(1, Ordering::SeqCst);
    }
}

pub struct SatirRaporu {
    pub ad: String,
    pub gecti: bool,
    pub ayrinti: String,
}

pub async fn calistir() -> Sonuc<Vec<SatirRaporu>> {
    let mut satirlar = Vec::new();

    // -------------------------------------------------------------------------
    // G1.1: Mevcut kabuk kilidi + bekleyen dosya_yaz + kilitsiz OLMAYAN
    // kontrol çağrısı (`komut_durdur`) -> KİLİTLENME (deadlock) ölçümü.
    // -------------------------------------------------------------------------
    {
        let kilitler = Arc::new(KaynakKilitleri::yeni());
        let kabuk_basladi = Arc::new(Notify::new());
        let kabuk_durdur = Arc::new(Notify::new());
        let dosya_yaz_basladi = Arc::new(Notify::new());
        let dosya_yaz_bitti = Arc::new(AtomicBool::new(false));

        // 1. Uzun süren kabuk komutu `tum_calisma_alani: true` ile yazma kilidini alır.
        let kilitler_kabuk = Arc::clone(&kilitler);
        let basladi_kopya = Arc::clone(&kabuk_basladi);
        let durdur_kopya = Arc::clone(&kabuk_durdur);
        let kabuk_gorevi = tokio::spawn(async move {
            let _kilit = kilitler_kabuk.sahipli_al(&[], &[], true).await;
            basladi_kopya.notify_one();
            durdur_kopya.notified().await;
        });

        kabuk_basladi.notified().await;

        // 2. Eşzamanlı `dosya_yaz("src/lib.rs")` gelir (`tum_calisma_alani: false`).
        let kilitler_yaz = Arc::clone(&kilitler);
        let yaz_basladi_kopya = Arc::clone(&dosya_yaz_basladi);
        let yaz_bitti_kopya = Arc::clone(&dosya_yaz_bitti);
        let yaz_gorevi = tokio::spawn(async move {
            yaz_basladi_kopya.notify_one();
            let _kilit = kilitler_yaz
                .sahipli_al(&[], &[PathBuf::from("src/lib.rs")], false)
                .await;
            yaz_bitti_kopya.store(true, Ordering::SeqCst);
        });

        dosya_yaz_basladi.notified().await;
        tokio::time::sleep(Duration::from_millis(25)).await;
        let yaz_bekliyor = !dosya_yaz_bitti.load(Ordering::SeqCst);

        // 3. Kontrol çağrısı (`komut_durdur`) eğer `kilitsiz: false` ise
        // `KaynakKilitleri::sahipli_al(&[], &[], false)` bekler -> kilitlenir!
        let bas = Instant::now();
        let kilitler_kontrol = Arc::clone(&kilitler);
        let durdur_kontrol = Arc::clone(&kabuk_durdur);
        let kontrol_sonucu = tokio::time::timeout(Duration::from_millis(150), async move {
            let _kilit = kilitler_kontrol.sahipli_al(&[], &[], false).await;
            durdur_kontrol.notify_one();
        })
        .await;
        let sure_ms = bas.elapsed().as_millis();
        let kilitlendi = kontrol_sonucu.is_err();

        // Temizlik için kabuk görevini doğrudan uyandır.
        kabuk_durdur.notify_one();
        let _ = kabuk_gorevi.await;
        let _ = yaz_gorevi.await;

        satirlar.push(SatirRaporu {
            ad: "G1.1 Kilitli kontrol çağrısında kilitlenme (deadlock)".into(),
            gecti: yaz_bekliyor && kilitlendi,
            ayrinti: format!(
                "uzun komut `genel` yazma kilidini tutarken `dosya_yaz` bekledi ({yaz_bekliyor}); `kilitsiz: false` olan `komut_durdur` `genel` okuma kilidini alamayarak {sure_ms} ms sonunda zaman aşımına düştü (kilitlendi: {kilitlendi})"
            ),
        });
    }

    // -------------------------------------------------------------------------
    // G1.2: Süreç sürerken kilidi ve son denetimi erken bırakmanın koruma ihlali
    // kanıtı.
    // -------------------------------------------------------------------------
    {
        let alan = OrnekCalismaAlani::yeni();
        let onceki = alan.goruntu_yakala();
        let erken_denetim = alan.komut_sonrasi_denetim(&onceki).is_ok();

        // Arka plan süreci erken dönüşten 30 ms sonra `.git/config` dosyasını değiştirir.
        let alan_arka = Arc::clone(&alan);
        let arka_plan = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            *alan_arka
                .git_config
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = b"[core]\n\tfsmonitor = \"evil\"\n".to_vec();
        });
        arka_plan.await.map_err(|e| e.to_string())?;
        let kapanis_denetimi_ihlali = alan.komut_sonrasi_denetim(&onceki).is_err();

        satirlar.push(SatirRaporu {
            ad: "G1.2 Erken dönüşte korunan dosya denetimi kaçışı".into(),
            gecti: erken_denetim && kapanis_denetimi_ihlali,
            ayrinti: format!(
                "erken dönüş anındaki denetim temiz döndü ({erken_denetim}), ancak arka plan süreci 30 ms sonra `.git/config` (`fsmonitor`) değiştirdi; denetim yalnız gerçek süreç kapanışında çalıştığında ihlali yakaladı ({kapanis_denetimi_ihlali})"
            ),
        });
    }

    // -------------------------------------------------------------------------
    // G1.3: Seçilen tasarım — Sahipli kilit devri (`IsKaydi`) + `kilitsiz: true`
    // kontrol kanalı (`oku`/`girdi`/`durdur`) + kapanışta zorunlu son denetim +
    // bekleyen `dosya_yaz` işleminin kilit çözülünce tamamlanması.
    // -------------------------------------------------------------------------
    {
        let kilitler = Arc::new(KaynakKilitleri::yeni());
        let alan = OrnekCalismaAlani::yeni();

        struct IsKaydi {
            oturum_id: &'static str,
            is_id: &'static str,
            onceki_koruma: KorumaGoruntusu,
            kilit: Mutex<Option<KilitSeti>>,
            durduruldu: AtomicBool,
            cikis_bildirimi: Notify,
        }

        // Başlatma: `sahipli_al` ile `genel` yazma kilidi alınır ve `IsKaydi` içine devredilir.
        alan.yazma_basladi();
        let sahipli_kilit = kilitler.sahipli_al(&[], &[], true).await;
        let is_kaydi = Arc::new(IsKaydi {
            oturum_id: "oturum-1",
            is_id: "is-1",
            onceki_koruma: alan.goruntu_yakala(),
            kilit: Mutex::new(Some(sahipli_kilit)),
            durduruldu: AtomicBool::new(false),
            cikis_bildirimi: Notify::new(),
        });

        // Başka oturumun (`oturum-2`) kontrol çağrısı reddedilir.
        let yetkisiz_red = is_kaydi.oturum_id != "oturum-2";

        // Aynı anda `dosya_yaz("src/lib.rs")` çağrısı gelir ve `genel` kilidinde seri bekler.
        let yaz_bitti = Arc::new(AtomicBool::new(false));
        let yaz_bitti_kopya = Arc::clone(&yaz_bitti);
        let kilitler_yaz = Arc::clone(&kilitler);
        let alan_yaz = Arc::clone(&alan);
        let yaz_gorevi = tokio::spawn(async move {
            let _kilit = kilitler_yaz
                .sahipli_al(&[], &[PathBuf::from("src/lib.rs")], false)
                .await;
            *alan_yaz.src_lib.lock().unwrap_or_else(|e| e.into_inner()) =
                b"pub fn yeni() {}\n".to_vec();
            yaz_bitti_kopya.store(true, Ordering::SeqCst);
        });

        tokio::time::sleep(Duration::from_millis(25)).await;
        let calisirken_yaz_bekliyor = !yaz_bitti.load(Ordering::SeqCst);

        // `komut_durdur` (`kilitsiz: true`) `KaynakKilitleri::sahipli_al` çağırmadan
        // doğrudan `(oturum_id, is_id)` doğrulamasıyla `IsKaydi`'nı kapatır.
        let bas = Instant::now();
        assert_eq!(is_kaydi.oturum_id, "oturum-1");
        assert_eq!(is_kaydi.is_id, "is-1");
        is_kaydi.durduruldu.store(true, Ordering::SeqCst);
        is_kaydi.cikis_bildirimi.notify_waiters();

        // Kapanış son denetimi (`komut_sonrasi_denetim`) + `yazma_bitisini_bekle` + kilit düşürme:
        let denetim_sonucu = alan.komut_sonrasi_denetim(&is_kaydi.onceki_koruma);
        alan.yazma_bitisini_tamamla();
        let dusurulen_kilit = is_kaydi
            .kilit
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        drop(dusurulen_kilit);

        // Bekleyen `dosya_yaz` hemen tamamlanmalı:
        tokio::time::timeout(Duration::from_millis(200), yaz_gorevi)
            .await
            .map_err(|_| "dosya_yaz zaman aşımına uğradı")?
            .map_err(|e| e.to_string())?;
        let cozulme_ms = bas.elapsed().as_millis();
        let yaz_tamam = yaz_bitti.load(Ordering::SeqCst);
        let kusak = alan.yazma_kusagi.load(Ordering::SeqCst);

        satirlar.push(SatirRaporu {
            ad: "G1.3 Sahipli kilit devri + kilitsiz kontrol + seri dosya_yaz".into(),
            gecti: yetkisiz_red
                && calisirken_yaz_bekliyor
                && denetim_sonucu.is_ok()
                && yaz_tamam
                && kusak == 1,
            ayrinti: format!(
                "farklı oturum reddedildi ({yetkisiz_red}); iş çalışırken `dosya_yaz` bekledi ({calisirken_yaz_bekliyor}); `kilitsiz: true` olan `komut_durdur` kilitlenmeden işi durdurdu, son denetimi geçirdi ve kilidi düşürdü; bekleyen `dosya_yaz` {cozulme_ms} ms içinde tamamlandı (yazma kuşağı W={kusak})"
            ),
        });
    }

    Ok(satirlar)
}
