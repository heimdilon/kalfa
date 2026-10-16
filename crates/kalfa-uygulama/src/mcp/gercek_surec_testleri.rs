// Gerçek stdio süreçle Görev 6 kapanış, son denetim ve kuşak kabul testleri.

use super::*;
use async_trait::async_trait;
use kalfa_core::arac::{
    AracCiktisi, AracHatasi, McpAgKarti, McpAracHazirligi, McpAracTanimOzeti, McpErisim,
    McpKartEtiketi, McpTarifi,
};
use kalfa_core::izin::Kip;
use kalfa_core::kilit::{KaynakKilitleri, KilitYoneticisi};
use kalfa_core::kimlik::CagriId;
use kalfa_mcp::arac::{McpAracCagriIstegi, McpAracKimligi, McpAracYurutucusu};
use kalfa_sandbox::tarama::CancellationToken;
use kalfa_sandbox::{PlatformAyari, Yalitici};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::{Notify, RwLock};

const SUNUCU: &str = "g6-gercek";
const UZAK_ARAC: &str = "ornek";
const TEST_SANITIZER_CIKTISI: &str = "G7 test sanitizer'ından geçen sabit çıktı";

struct SahteG7 {
    /// Dikiş testinde güncelleme yazarı olmadığı için D sabittir. Kart yine de
    /// çalışan G5 kuşağının gerçek R'siyle kurulur; varsayılan izin yoktur.
    d_kilidi: Arc<RwLock<()>>,
}

impl SahteG7 {
    fn yeni() -> Arc<Self> {
        Arc::new(Self {
            d_kilidi: Arc::new(RwLock::new(())),
        })
    }

    fn kart(&self, kimlik: &McpAracKimligi, tarif: McpTarifi) -> McpG7AracKarti {
        let sunucu = McpKartEtiketi::yeni(kimlik.sunucu().gorunen()).unwrap();
        let arac = McpKartEtiketi::yeni(kimlik.arac().gorunen()).unwrap();
        let hazirlik = McpAracHazirligi {
            sunucu,
            arac,
            kip: Kip::Siki,
            erisim: McpErisim::TamErisim,
            ag: McpAgKarti::Kapali,
            ilk_cagri: false,
            tarif,
            arac_tanimi: McpAracTanimOzeti::ozetten([5; 32]),
        };
        let sonuc_donusturucu = Arc::new(|_ham_sonuc: CallToolResult, content_var: bool| {
            if !content_var {
                return Err(AracHatasi::SonucIsleme);
            }
            Ok(AracCiktisi {
                icerik: TEST_SANITIZER_CIKTISI.to_owned(),
                ..AracCiktisi::default()
            })
        });
        McpG7AracKarti {
            hazirlik,
            sunum: kalfa_mcp::arac::McpAracSunumu {
                ad: format!(
                    "mcp__{}__{}",
                    kimlik.sunucu().gorunen(),
                    kimlik.arac().gorunen()
                ),
                aciklama: "Gerçek süreç kabul testi için güvenli G7 sunumu.".to_owned(),
                parametre_semasi: serde_json::json!({"type":"object","properties":{}}),
            },
            // Ham RMCP yanıtı hiçbir zaman test çıktısına taşınmaz.
            sonuc_donusturucu,
        }
    }
}

#[async_trait]
impl McpG7AracKaynagi for SahteG7 {
    async fn cagri_korumasini_al(
        &self,
        _kimlik: &McpAracKimligi,
        _onayli: &McpAracHazirligi,
    ) -> Result<tokio::sync::OwnedRwLockReadGuard<()>, AracHatasi> {
        Ok(Arc::clone(&self.d_kilidi).read_owned().await)
    }

    async fn guncel_kart(
        &self,
        kimlik: &McpAracKimligi,
        aktif_tarif: McpTarifi,
    ) -> Result<McpG7AracKarti, AracHatasi> {
        Ok(self.kart(kimlik, aktif_tarif))
    }

    fn content_alani_var(&self, _cagri: CagriId) -> Option<bool> {
        // Yalnız `modern` başarı testinde kullanılır. İptal testleri başarı
        // sonucuna girmez ve bu sahte G7 değeri ham SDK yanıtı değildir.
        Some(true)
    }
}

struct SonDenetimBariyeri {
    ilkini_beklet: AtomicBool,
    basladi: Notify,
    devam: Notify,
}

impl SonDenetimBariyeri {
    fn yeni() -> Arc<Self> {
        Arc::new(Self {
            ilkini_beklet: AtomicBool::new(true),
            basladi: Notify::new(),
            devam: Notify::new(),
        })
    }

    async fn girisi_bekle(&self) {
        tokio::time::timeout(Duration::from_secs(8), self.basladi.notified())
            .await
            .expect("G5 süreç kapanışından sonra son denetim başlamalı");
    }

    fn devam_et(&self) {
        self.devam.notify_one();
    }
}

struct BariyerliKoruma {
    ic: Arc<KontrolluKoruma>,
    bariyer: Arc<SonDenetimBariyeri>,
}

impl KorumaDenetleyicisi for BariyerliKoruma {
    fn dogrula<'a>(
        &'a self,
        yalitici: &'a Arc<dyn Yalitici>,
        istek: KorumaDenetimIstegi,
    ) -> Pin<Box<dyn Future<Output = McpKorumaSonucu> + Send + 'a>> {
        self.ic.dogrula(yalitici, istek)
    }

    fn son_denetim<'a>(
        &'a self,
        istek: SonDenetimIstegi,
    ) -> Pin<Box<dyn Future<Output = SonDenetimSonucu> + Send + 'a>> {
        let ic = Arc::clone(&self.ic);
        let bariyer = Arc::clone(&self.bariyer);
        Box::pin(async move {
            if bariyer.ilkini_beklet.swap(false, Ordering::SeqCst) {
                // İptal cleanup'ında bu nokta gerçek G5 `kapat()` tamamlandıktan
                // sonra gelir; normal sonuçta core hook'unun son taramasıdır.
                bariyer.basladi.notify_one();
                bariyer.devam.notified().await;
            }
            ic.son_denetim(istek).await
        })
    }
}

struct GercekKurulum {
    test: TestOrtami,
    yonetici: Arc<McpYoneticisi>,
    yurutucu: Arc<dyn McpAracYurutucusu>,
    kimlik: Arc<McpAracKimligi>,
    kaynaklar: Arc<KaynakKilitleri>,
    bariyer: Arc<SonDenetimBariyeri>,
}

impl GercekKurulum {
    async fn yeni(
        sunucu_kipi: &str,
        ek_sunucu_argumanlari: Vec<String>,
        cagri_zamani_asimi_sn: u32,
    ) -> Self {
        let test = TestOrtami::kur();
        let sunucu_ikilisi = test_sunucusu_ikilisi();
        let secili_kimlik = "exe-g6-gercek".to_owned();
        test.sunucu_kaydet(
            McpSunucuKaydi {
                ad: SUNUCU.to_owned(),
                etkin: true,
                komut: McpKomut::SeciliYurutulebilir {
                    kimlik: secili_kimlik.clone(),
                    argumanlar: std::iter::once(sunucu_kipi.to_owned())
                        .chain(ek_sunucu_argumanlari)
                        .collect(),
                },
                ortam: Vec::new(),
                kip: McpKipi::TamErisim,
                ag_izinleri: Vec::new(),
                baslatma_zamani_asimi_sn: 20,
                cagri_zamani_asimi_sn,
            },
            Some((secili_kimlik, sunucu_ikilisi)),
        );

        let ev = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|| test.proje_koku.clone());
        let yalitici = kalfa_sandbox::platform_yaliticisi(PlatformAyari {
            ev,
            yalitim_dizini: test.yalitim_dizini.clone(),
            ek_okunur: Vec::new(),
            kalfa_araclari: Vec::new(),
            kopru: None,
            degisince: Box::new(|_| {}),
        });
        let bariyer = SonDenetimBariyeri::yeni();
        let koruma = Arc::new(BariyerliKoruma {
            ic: Arc::clone(&test.koruma),
            bariyer: Arc::clone(&bariyer),
        });
        let yonetici = McpYoneticisi::yeni(McpYoneticiBilesenleri {
            veri_dizini: test.veri_dizini.clone(),
            yalitim_dizini: test.yalitim_dizini.clone(),
            ayarlar: test.ayarlar.clone(),
            mcp_depo: test.ayarlar.clone(),
            kasa: test.kasa.clone(),
            iletisim: test.iletisim.clone(),
            yalitici,
            yalitim_ayarlari: Arc::clone(&test.yalitim_ayarlari),
            koruma_denetleyicisi: Some(koruma),
            // G5'nin gerçek süreç launcher'ı seçilsin.
            surec_baslatici: None,
            saat: Some(test.saat.clone()),
            // Gerçek süreç, test makinesinin gerekli sistem ortamını alır.
            sistem_ortami: None,
        });

        let oturum = OturumId::yeni();
        yonetici.oturum_kipi_guncelle(
            &test.alan_kimligi,
            Some(&test.proje_koku),
            oturum,
            Some(Kip::Siki),
        );
        test.iletisim.onay_ver(true);
        match yonetici
            .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, SUNUCU)
            .await
        {
            Ok(_) => {}
            Err(YoneticiHatasi::BaslatmaIzniGerekli(kart)) => {
                assert!(
                    yonetici
                        .baslatma_izni_yanitla(
                            &kart.istek_kimligi,
                            &test.alan_kimligi,
                            &test.proje_koku,
                            true,
                        )
                        .await
                        .unwrap()
                );
                yonetici
                    .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, SUNUCU)
                    .await
                    .unwrap();
            }
            Err(hata) => panic!("gerçek G5 test sunucusu başlatılamadı: {hata:?}"),
        }

        let g7 = SahteG7::yeni();
        let yurutucu = yonetici.arac_yurutucusu(g7);
        let kimlik = Arc::new(McpAracKimligi::yeni(
            test.alan_kimligi.clone(),
            test.proje_koku.clone(),
            McpKartEtiketi::yeni(SUNUCU).unwrap(),
            McpKartEtiketi::yeni(UZAK_ARAC).unwrap(),
        ));
        let kilit_yoneticisi = KilitYoneticisi::yeni();
        let kaynaklar = kilit_yoneticisi.al(&test.proje_koku).unwrap();

        Self {
            test,
            yonetici,
            yurutucu,
            kimlik,
            kaynaklar,
            bariyer,
        }
    }

    async fn cagri_baslat(
        &self,
        cagri: CagriId,
        iptal: CancellationToken,
    ) -> tokio::sync::oneshot::Receiver<Result<AracCiktisi, AracHatasi>> {
        gercek_cagri_baslat(
            Arc::clone(&self.yurutucu),
            Arc::clone(&self.kimlik),
            Arc::clone(&self.kaynaklar),
            cagri,
            iptal,
            None,
        )
        .await
    }
}

async fn gercek_cagri_baslat(
    yurutucu: Arc<dyn McpAracYurutucusu>,
    kimlik: Arc<McpAracKimligi>,
    kaynaklar: Arc<KaynakKilitleri>,
    cagri: CagriId,
    iptal: CancellationToken,
    kilit_oncesi: Option<Arc<Notify>>,
) -> tokio::sync::oneshot::Receiver<Result<AracCiktisi, AracHatasi>> {
    // Core sırası: ilk kart → sahipli yazma kilidini eşzamanlı devret →
    // kilit map'e geçtiğinde taze kartı al → guard'sız RPC isteğini başlat.
    let ilk_hazirlik = yurutucu.guncel_hazirlik(&kimlik).await.unwrap();
    if let Some(bildir) = kilit_oncesi {
        bildir.notify_one();
    }
    let kilit = kaynaklar.sahipli_al(&[], &[], true).await;
    yurutucu
        .kilidi_devral(McpAracCagriIstegi {
            cagri,
            kimlik: Arc::clone(&kimlik),
            hazirlik: ilk_hazirlik,
            argumanlar: serde_json::Map::new(),
            iptal: iptal.clone(),
            kilit: Some(kilit),
        })
        .unwrap();
    let taze_hazirlik = yurutucu.guncel_hazirlik(&kimlik).await.unwrap();
    Arc::clone(&yurutucu)
        .cagriyi_baslat(McpAracCagriIstegi {
            cagri,
            kimlik,
            hazirlik: taze_hazirlik,
            argumanlar: serde_json::Map::new(),
            iptal,
            kilit: None,
        })
        .unwrap()
}

fn test_sunucusu_ikilisi() -> PathBuf {
    if let Some(yol) = std::env::var_os("CARGO_BIN_EXE_kalfa-mcp-test-sunucusu") {
        return PathBuf::from(yol);
    }
    #[cfg(windows)]
    let ad = "kalfa-mcp-test-sunucusu.exe";
    #[cfg(not(windows))]
    let ad = "kalfa-mcp-test-sunucusu";
    let yol = std::env::current_exe()
        .expect("test ikilisinin yolu")
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/debug üst dizini")
        .join(ad);
    assert!(
        yol.is_file(),
        "gerçek süreç kabul testi workspace testlerinde koşmalı; kalfa-mcp test sunucusu target/debug altında yok: {}",
        yol.display()
    );
    yol
}

async fn gunluk_satiri_bekle(kurulum: &GercekKurulum, satir: &str) {
    tokio::time::timeout(Duration::from_secs(6), async {
        loop {
            if kurulum
                .yonetici
                .stderr_satirlari(&kurulum.test.alan_kimligi, SUNUCU)
                .iter()
                .any(|gelen| gelen == satir)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("gerçek MCP sunucusunda {satir} gözlenmedi"));
}

async fn yazma_kilidi_hala_tutuluyor(kaynaklar: &Arc<KaynakKilitleri>) -> bool {
    tokio::time::timeout(
        Duration::from_millis(60),
        kaynaklar.sahipli_al(&[], &[], true),
    )
    .await
    .is_err()
}

#[derive(Clone, Copy)]
enum KesmeTuru {
    AcikIptal,
    Deadline,
}

#[tokio::test]
async fn gercek_g5_iptal_ve_deadline_uc_server_davranisinda_kilidi_son_denetimden_once_cozmez() {
    for sunucu_kipi in ["cancel-response", "silent-cancel", "ignore-cancel"] {
        for kesme in [KesmeTuru::AcikIptal, KesmeTuru::Deadline] {
            let kurulum = GercekKurulum::yeni(
                sunucu_kipi,
                Vec::new(),
                if matches!(kesme, KesmeTuru::Deadline) {
                    1
                } else {
                    20
                },
            )
            .await;
            let cagri = CagriId::yeni();
            let iptal = CancellationToken::new();
            let mut yanit = kurulum.cagri_baslat(cagri, iptal.clone()).await;
            gunluk_satiri_bekle(&kurulum, "CALL 1").await;

            if matches!(kesme, KesmeTuru::AcikIptal) {
                iptal.cancel();
            }
            // RMCP'nin düşen gerçek çağrı future'ı best-effort cancelled
            // bildirimi gönderir; G6 ayrıca süreç ağacını kapatmayı bekler.
            gunluk_satiri_bekle(&kurulum, "CANCEL").await;
            kurulum.bariyer.girisi_bekle().await;

            assert!(yazma_kilidi_hala_tutuluyor(&kurulum.kaynaklar).await);
            assert!(
                tokio::time::timeout(Duration::from_millis(60), &mut yanit)
                    .await
                    .is_err(),
                "yanıt, G5 süreç kapanışı ve zorunlu son denetim tamamlanmadan gelmemeli"
            );

            kurulum.bariyer.devam_et();
            let sonuc = tokio::time::timeout(Duration::from_secs(8), yanit)
                .await
                .expect("iptal cleanup sonlu sürede tamamlanmalı")
                .unwrap();
            match kesme {
                KesmeTuru::AcikIptal => assert!(matches!(sonuc, Err(AracHatasi::Iptal))),
                KesmeTuru::Deadline => {
                    assert!(matches!(sonuc, Err(AracHatasi::Gecersiz(_))))
                }
            }
            let _yeni_kilit = tokio::time::timeout(
                Duration::from_secs(2),
                kurulum.kaynaklar.sahipli_al(&[], &[], true),
            )
            .await
            .expect("son denetim başarılı olduktan sonra kilit çözülsün");
        }
    }
}

#[tokio::test]
async fn gecikmeli_torun_surec_kapanisi_sonrasinda_dosya_yazamaz() {
    let gecici = tempfile::tempdir().unwrap();
    let isaret = gecici.path().join("gecikmeli-torun-yazdi");
    std::fs::write(&isaret, b"").unwrap();
    let kurulum = GercekKurulum::yeni(
        "delayed-child-write",
        vec![isaret.to_string_lossy().into_owned(), "5000".to_owned()],
        20,
    )
    .await;
    let cagri = CagriId::yeni();
    let iptal = CancellationToken::new();
    let mut yanit = kurulum.cagri_baslat(cagri, iptal.clone()).await;
    gunluk_satiri_bekle(&kurulum, "CALL 1").await;
    iptal.cancel();
    gunluk_satiri_bekle(&kurulum, "CANCEL").await;
    kurulum.bariyer.girisi_bekle().await;

    tokio::time::sleep(Duration::from_millis(5500)).await;
    assert!(
        std::fs::read(&isaret).unwrap().is_empty(),
        "G5 süreç ağacı kapanışı doğrulandıktan sonra gecikmeli MCP torunu yazmamalı"
    );
    assert!(yazma_kilidi_hala_tutuluyor(&kurulum.kaynaklar).await);
    assert!(
        tokio::time::timeout(Duration::from_millis(60), &mut yanit)
            .await
            .is_err()
    );

    kurulum.bariyer.devam_et();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(8), yanit)
            .await
            .unwrap()
            .unwrap(),
        Err(AracHatasi::Iptal)
    ));
    let _yeni_kilit = tokio::time::timeout(
        Duration::from_secs(2),
        kurulum.kaynaklar.sahipli_al(&[], &[], true),
    )
    .await
    .expect("torun yanıtı iptal temizliğini atlamamalı");
}

#[tokio::test]
async fn eski_kusak_cleanup_yeni_kusagi_durdurmaz_ve_eski_guard_serbest_kalir() {
    let kurulum = GercekKurulum::yeni("modern", Vec::new(), 20).await;
    let eski_hazirlik = kurulum
        .yurutucu
        .guncel_hazirlik(&kurulum.kimlik)
        .await
        .unwrap();
    let eski_kusak = eski_hazirlik.tarif.baslatma();

    // Eski N sürecinin G5 kapanışında son denetim bariyerine geldiğinde
    // beklemeden geçmesi için önceden permit bırak. Böylece N+1, eski kapanış
    // kanıtı manager kaydına yazıldıktan sonra başlatılabilir.
    kurulum.bariyer.devam_et();
    kurulum
        .yonetici
        .sunucuyu_durdur_ve_bekle(&kurulum.test.alan_kimligi, SUNUCU)
        .await
        .unwrap();
    let (yeni_kusak, _) = kurulum
        .yonetici
        .baslat_veya_dogrula(&kurulum.test.alan_kimligi, &kurulum.test.proje_koku, SUNUCU)
        .await
        .unwrap();
    assert_ne!(eski_kusak, yeni_kusak);
    assert!(matches!(
        kurulum
            .yonetici
            .sunucu_durumu(&kurulum.test.alan_kimligi, SUNUCU),
        McpSunucuDurumu::Calisiyor { kusak, .. } if kusak == yeni_kusak
    ));

    // N+1 çalışırken eski N kartı ve yeni alınmış sahipli guard ile kayıt
    // eklenir. Worker eski N hazırlığıyla başladığında N+1'i çağırmamalı.
    let cagri = CagriId::yeni();
    let iptal = CancellationToken::new();
    let kilit = kurulum.kaynaklar.sahipli_al(&[], &[], true).await;
    kurulum
        .yurutucu
        .kilidi_devral(McpAracCagriIstegi {
            cagri,
            kimlik: Arc::clone(&kurulum.kimlik),
            hazirlik: eski_hazirlik.clone(),
            argumanlar: serde_json::Map::new(),
            iptal: iptal.clone(),
            kilit: Some(kilit),
        })
        .unwrap();
    let yanit = Arc::clone(&kurulum.yurutucu)
        .cagriyi_baslat(McpAracCagriIstegi {
            cagri,
            kimlik: Arc::clone(&kurulum.kimlik),
            hazirlik: eski_hazirlik,
            argumanlar: serde_json::Map::new(),
            iptal,
            kilit: None,
        })
        .unwrap();
    let eski_cagri_reddedildi = matches!(
        tokio::time::timeout(Duration::from_secs(6), yanit).await,
        Ok(Ok(Err(AracHatasi::OnayEskidi)))
    );
    let yeni_kusak_calisiyor = matches!(
        kurulum
            .yonetici
            .sunucu_durumu(&kurulum.test.alan_kimligi, SUNUCU),
        McpSunucuDurumu::Calisiyor { kusak, .. } if kusak == yeni_kusak
    );
    let yeni_rpc_baslamadi = !kurulum
        .yonetici
        .stderr_satirlari(&kurulum.test.alan_kimligi, SUNUCU)
        .iter()
        .any(|satir| satir == "CALL 1");
    let eski_guard_cozuldu = !yazma_kilidi_hala_tutuluyor(&kurulum.kaynaklar).await;
    let ikinci_guard_alinabiliyor = if eski_guard_cozuldu {
        match tokio::time::timeout(
            Duration::from_secs(1),
            kurulum.kaynaklar.sahipli_al(&[], &[], true),
        )
        .await
        {
            Ok(guard) => {
                drop(guard);
                true
            }
            Err(_) => false,
        }
    } else {
        false
    };

    // Başarısız bir kırmızı varyant da N+1 süreç bırakmasın.
    let n2_kapatildi = kurulum
        .yonetici
        .sunucuyu_durdur_ve_bekle(&kurulum.test.alan_kimligi, SUNUCU)
        .await
        .is_ok();

    assert!(
        eski_cagri_reddedildi,
        "eski N hazırlığı OnayEskidi ile sonuçlanmalı"
    );
    assert!(yeni_kusak_calisiyor, "eski cleanup N+1'i durdurmamalı");
    assert!(yeni_rpc_baslamadi, "eski N çağrısı N+1 RPC'sine ulaşmamalı");
    assert!(eski_guard_cozuldu, "eski N kapanış kanıtı guard'ı çözmeli");
    assert!(
        ikinci_guard_alinabiliyor,
        "N+1 çalışırken ikinci yazma guard'ı alınabilmeli"
    );
    assert!(n2_kapatildi, "kabul testi sonunda N+1 sunucusu kapatılmalı");
}

#[tokio::test]
async fn normal_gercek_rpc_sonucu_core_hook_ve_son_denetim_bitisine_kadar_kilidi_tutar() {
    let kurulum = GercekKurulum::yeni("modern", Vec::new(), 20).await;
    let cagri = CagriId::yeni();
    let yanit = kurulum.cagri_baslat(cagri, CancellationToken::new()).await;
    gunluk_satiri_bekle(&kurulum, "CALL 1").await;
    let sonuc = tokio::time::timeout(Duration::from_secs(4), yanit)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(sonuc.icerik, TEST_SANITIZER_CIKTISI);
    assert!(yazma_kilidi_hala_tutuluyor(&kurulum.kaynaklar).await);

    let yurutucu = Arc::clone(&kurulum.yurutucu);
    let hook = tokio::spawn(async move { yurutucu.sonuc_isleme_bitti(cagri).await });
    kurulum.bariyer.girisi_bekle().await;
    assert!(yazma_kilidi_hala_tutuluyor(&kurulum.kaynaklar).await);

    // İkinci araç çağrısı ilk son denetim bariyerinde kilide ulaşır ama RPC'ye
    // geçemez. CALL 2 ancak son denetim onaylanıp guard bırakılınca görünür.
    let ikinci_cagri = CagriId::yeni();
    let kilit_oncesi = Arc::new(Notify::new());
    let kilit_oncesi_bekle = Arc::clone(&kilit_oncesi);
    let ikinci_yurutucu = Arc::clone(&kurulum.yurutucu);
    let ikinci_kimlik = Arc::clone(&kurulum.kimlik);
    let ikinci_kaynaklar = Arc::clone(&kurulum.kaynaklar);
    let ikinci = tokio::spawn(async move {
        gercek_cagri_baslat(
            ikinci_yurutucu,
            ikinci_kimlik,
            ikinci_kaynaklar,
            ikinci_cagri,
            CancellationToken::new(),
            Some(kilit_oncesi_bekle),
        )
        .await
        .await
    });
    tokio::time::timeout(Duration::from_secs(4), kilit_oncesi.notified())
        .await
        .expect("ikinci çağrı testte aynı yazma kilidine ulaşmalı");
    assert!(
        tokio::time::timeout(
            Duration::from_millis(100),
            gunluk_satiri_bekle(&kurulum, "CALL 2")
        )
        .await
        .is_err(),
        "ikinci MCP RPC son denetim tamamlanmadan başlamamalı"
    );

    kurulum.bariyer.devam_et();
    tokio::time::timeout(Duration::from_secs(8), hook)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    gunluk_satiri_bekle(&kurulum, "CALL 2").await;
    let ikinci_sonuc = tokio::time::timeout(Duration::from_secs(4), ikinci)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let ikinci_sonuc = ikinci_sonuc.unwrap();
    assert_eq!(ikinci_sonuc.icerik, TEST_SANITIZER_CIKTISI);
    Arc::clone(&kurulum.yurutucu)
        .sonuc_isleme_bitti(ikinci_cagri)
        .await
        .unwrap();
    let _yeni_kilit = tokio::time::timeout(
        Duration::from_secs(2),
        kurulum.kaynaklar.sahipli_al(&[], &[], true),
    )
    .await
    .expect("core sonuç hook'u ve başarılı son denetimden sonra kilit açılsın");
    kurulum
        .yonetici
        .sunucuyu_durdur_ve_bekle(&kurulum.test.alan_kimligi, SUNUCU)
        .await
        .unwrap();
}
