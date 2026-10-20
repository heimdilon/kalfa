//! Görev 5 (`M04`): Çok sunuculu denetlenmiş uzak MCP kataloğunda `arac_ara` ile
//! araç keşfi, bağlam bütçesi sınırları (`<= 8` aday, `<= 16` etkin uzak araç,
//! `<= 64 KiB` toplam şema baytı), sonraki model turunda görünürlük, sürüm/tarif
//! bağlama (`OnayEskidi`) ve sır/`outputSchema` denetimi testleri.

use std::sync::Arc;

use kalfa_core::arac::{
    Arac, AracHatasi, AracKayitDefteri, McpAracTanimOzeti, McpBaslatmaSayaci, McpTarifi,
    async_trait,
};
use kalfa_core::izin::{GizliAyarlari, Kip};
use kalfa_mcp::maske::GizliDeger;
use kalfa_mcp::tanim::{
    AZAMI_UZAK_ADAY_ARAC_SINIRI, AZAMI_UZAK_ETKIN_ARAC_SINIRI, AZAMI_UZAK_SEMA_BAYT_SINIRI,
    UzakMcpTanim, UzakYetkiTuru,
};
use kalfa_mcp::uzak::{
    ARAC_ARA_ADI, AracAraAraci, AracAraSonucu, UzakAracKatalogu, UzakAracKaynagi, UzakCagriHatasi,
    UzakKesifHatasi, UzakMcpCagriYurutucusu, UzakSunucuKatalogGirdisi,
};
use kalfa_mcp::{CallToolResult, Tool};
use kalfa_sandbox::tarama::CancellationToken;
use serde_json::{Map, Value, json};

const CANARY_SIR: &str = "CANARY-M04-SECRET-77889900";

struct SahteUzakYurutucu {
    katalog: Arc<UzakAracKatalogu>,
}

#[async_trait]
impl UzakMcpCagriYurutucusu for SahteUzakYurutucu {
    async fn uzak_arac_cagir(
        &self,
        sunucu_adi: &str,
        uzak_arac_adi: &str,
        argumanlar: Map<String, Value>,
        onayli_tarif: McpTarifi,
        _onayli_d: McpAracTanimOzeti,
        _iptal: CancellationToken,
    ) -> Result<(CallToolResult, bool), UzakCagriHatasi> {
        let Some(guncel_tarif) = self.katalog.sunucu_tarifi(sunucu_adi) else {
            return Err(UzakCagriHatasi::OnayEskidi);
        };
        if guncel_tarif != onayli_tarif {
            return Err(UzakCagriHatasi::OnayEskidi);
        }

        if uzak_arac_adi == "fatura_kes" {
            let tutar = argumanlar
                .get("tutar")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            let sonuc: CallToolResult = serde_json::from_value(json!({
                "content": [{
                    "type": "text",
                    "text": format!("Fatura kesildi (FAT-2026-001): {tutar} TL (iz={CANARY_SIR})")
                }],
                "structuredContent": {
                    "fatura_no": "FAT-2026-001",
                    "tutar": tutar,
                    "durum": "onaylandi"
                }
            }))
            .unwrap();
            return Ok((sonuc, true));
        }

        if uzak_arac_adi == "bozuk_sema_araci" {
            // outputSchema fatura_no: string beklerken integer dönüyoruz -> şema hatası!
            let sonuc: CallToolResult = serde_json::from_value(json!({
                "content": [{ "type": "text", "text": "yanıt" }],
                "structuredContent": {
                    "fatura_no": 12345
                }
            }))
            .unwrap();
            return Ok((sonuc, true));
        }

        let sonuc: CallToolResult = serde_json::from_value(json!({
            "content": [{ "type": "text", "text": "tamam" }]
        }))
        .unwrap();
        Ok((sonuc, true))
    }
}

fn arac_olustur(
    ad: &str,
    aciklama: &str,
    girdi_semasi: Value,
    cikti_semasi: Option<Value>,
) -> Tool {
    let girdi_map = girdi_semasi.as_object().cloned().unwrap_or_default();
    let mut tool = Tool::new(ad.to_owned(), aciklama.to_owned(), Arc::new(girdi_map));
    if let Some(cikti) = cikti_semasi
        && let Some(cikti_map) = cikti.as_object().cloned()
    {
        tool.output_schema = Some(Arc::new(cikti_map));
    }
    tool
}

fn sunucu_girdisi(
    ad: &str,
    port: u16,
    tanim_revizyonu: u64,
    kimlik_revizyonu: u64,
    arac_revizyonu: u64,
    ham_araclar: Vec<Tool>,
    sirlar: Vec<GizliDeger>,
) -> UzakSunucuKatalogGirdisi {
    let tanim = UzakMcpTanim::varsayilan_sinirlarla(
        ad.to_owned(),
        &format!("http://127.0.0.1:{port}/mcp"),
        true,
        UzakYetkiTuru::Yetkisiz,
    )
    .unwrap();
    UzakSunucuKatalogGirdisi {
        tanim,
        alan_kimligi: "alan-m04".to_owned(),
        gizli_ayarlari: GizliAyarlari::default(),
        tanim_revizyonu,
        kimlik_revizyonu,
        arac_revizyonu,
        protokol_surumu: "2025-06-18".to_owned(),
        baglanti_sayaci: McpBaslatmaSayaci::yeni(1),
        ham_araclar,
        sirlar,
    }
}

#[tokio::test]
async fn m04_cok_sunuculu_katalogda_arac_ara_sakli_araci_bulur_sonraki_turda_yansitir_ve_cagirir() {
    let katalog = Arc::new(UzakAracKatalogu::default());
    let yurutucu = Arc::new(SahteUzakYurutucu {
        katalog: Arc::clone(&katalog),
    });
    let kaynak = UzakAracKaynagi::yeni(Arc::clone(&katalog), yurutucu);
    let arac_ara = AracAraAraci::yeni(Arc::clone(&katalog), Arc::clone(&kaynak));

    let gizli = GizliDeger::yeni(CANARY_SIR.to_owned()).unwrap();

    // 3 ayrı uzak sunucu kataloğa kaydedilir.
    katalog.sunucu_katalogunu_guncelle(sunucu_girdisi(
        "analitik",
        19101,
        1,
        1,
        1,
        vec![
            arac_olust(
                "rapor_ozeti",
                "Günlük satış analiz raporunu getirir",
                json!({"type": "object", "properties": {"gun": {"type": "string"}}}),
                None,
            ),
            arac_olust(
                "metrik_sorgula",
                "Canlı performans metriklerini listeler",
                json!({"type": "object"}),
                None,
            ),
        ],
        vec![gizli.clone()],
    ));

    katalog.sunucu_katalogunu_guncelle(sunucu_girdisi(
        "muhasebe",
        19102,
        1,
        1,
        1,
        vec![
            arac_olust(
                "fatura_kes",
                "Müşteri için resmi e-fatura oluşturur ve onaylar",
                json!({
                    "type": "object",
                    "properties": { "tutar": { "type": "integer" } },
                    "required": ["tutar"]
                }),
                Some(json!({
                    "type": "object",
                    "properties": {
                        "fatura_no": { "type": "string" },
                        "tutar": { "type": "integer" },
                        "durum": { "type": "string" }
                    },
                    "required": ["fatura_no", "tutar", "durum"]
                })),
            ),
            arac_olust(
                "bozuk_sema_araci",
                "Çıktı şeması doğrulamasını sınayan araç",
                json!({"type": "object"}),
                Some(json!({
                    "type": "object",
                    "properties": { "fatura_no": { "type": "string" } },
                    "required": ["fatura_no"]
                })),
            ),
        ],
        vec![gizli.clone()],
    ));

    katalog.sunucu_katalogunu_guncelle(sunucu_girdisi(
        "depolama",
        19103,
        1,
        1,
        1,
        vec![arac_olust(
            "arsiv_listele",
            "Yedek arşiv paketlerini listeler",
            json!({"type": "object"}),
            None,
        )],
        vec![gizli],
    ));

    let defter = AracKayitDefteri::yeni_kaynakli(vec![arac_ara.clone()], kaynak.clone()).unwrap();

    // 1. Model Turu: Başlangıç görüntüsünde yalnız `arac_ara` vardır; uzak araçlar bağlamı şişirmez.
    let goruntu_tur_1 = defter.anlik_goruntu(Kip::Proje);
    assert_eq!(goruntu_tur_1.tanimlar().len(), 1);
    assert_eq!(goruntu_tur_1.tanimlar()[0].function.name, ARAC_ARA_ADI);
    assert!(goruntu_tur_1.bul("mcp__muhasebe__fatura_kes").is_none());

    // Model 1. turda `arac_ara` ile "fatura" araması yapar ve bulunan aracı yükler.
    let arama_arg = goruntu_tur_1
        .dogrula(
            ARAC_ARA_ADI,
            r#"{"sorgu": "fatura", "yukle": ["mcp__muhasebe__fatura_kes", "mcp__muhasebe__bozuk_sema_araci"]}"#,
        )
        .unwrap();
    let arama_hazirlik = arac_ara.hazirla(&arama_arg).await.unwrap();
    let arama_cikti = arac_ara
        .calistir(&arama_arg, &arama_hazirlik, CancellationToken::new())
        .await
        .unwrap();
    let arama_sonucu: AracAraSonucu = serde_json::from_str(&arama_cikti.icerik).unwrap();
    assert!(!arama_sonucu.adaylar.is_empty());
    assert_eq!(
        arama_sonucu.adaylar[0].birlesik_ad,
        "mcp__muhasebe__fatura_kes"
    );
    assert_eq!(arama_sonucu.yuklenenler.len(), 2);
    assert_eq!(arama_sonucu.etkin_uzak_arac_sayisi, 2);

    // ÖNEMLİ: 1. model turunun mevcut görüntüsünde (`goruntu_tur_1`) yeni yüklenen araç HÂLÂ YOKTUR!
    assert!(
        goruntu_tur_1.bul("mcp__muhasebe__fatura_kes").is_none(),
        "yeni yüklenen uzak araç aynı model turunun görüntüsünde kullanılamaz"
    );

    // 2. Model Turu: Yeni tur başı görüntüsünde (`goruntu_tur_2`) yüklenen uzak araçlar görünür olur.
    let goruntu_tur_2 = defter.anlik_goruntu(Kip::Proje);
    assert_eq!(goruntu_tur_2.tanimlar().len(), 3);
    let fatura_araci = goruntu_tur_2
        .bul("mcp__muhasebe__fatura_kes")
        .expect("2. tur görüntüsünde yüklü uzak araç bulunmalı");

    // 2. turda aracı hazırla ve çalıştır; çıktı şeması doğrulanır ve CANARY sır maskelenir.
    let cagri_arg = goruntu_tur_2
        .dogrula("mcp__muhasebe__fatura_kes", r#"{"tutar": 2500}"#)
        .unwrap();
    let cagri_hazirlik = fatura_araci.hazirla(&cagri_arg).await.unwrap();
    assert!(cagri_hazirlik.mcp.as_ref().unwrap().ilk_cagri);
    let cagri_cikti = fatura_araci
        .calistir(&cagri_arg, &cagri_hazirlik, CancellationToken::new())
        .await
        .unwrap();
    assert!(cagri_cikti.icerik.contains("FAT-2026-001"));
    assert!(
        !cagri_cikti.icerik.contains(CANARY_SIR),
        "araç çıktısındaki bilinen sır maskelenmelidir"
    );
    assert!(cagri_cikti.icerik.contains("[gizli]"));

    // Çıktı şemasına uymayan yanıt (`bozuk_sema_araci`) güvenli biçimde reddedilir.
    let bozuk_arac = goruntu_tur_2
        .bul("mcp__muhasebe__bozuk_sema_araci")
        .unwrap();
    let bozuk_hazirlik = bozuk_arac.hazirla(&json!({})).await.unwrap();
    let bozuk_sonuc = bozuk_arac
        .calistir(&json!({}), &bozuk_hazirlik, CancellationToken::new())
        .await;
    assert!(bozuk_sonuc.is_err(), "outputSchema ihlali reddedilmelidir");
}

#[tokio::test]
async fn m04_aday_arac_ve_sema_bayt_butce_sinirlari_korunur() {
    let katalog = Arc::new(UzakAracKatalogu::default());
    let yurutucu = Arc::new(SahteUzakYurutucu {
        katalog: Arc::clone(&katalog),
    });
    let kaynak = UzakAracKaynagi::yeni(Arc::clone(&katalog), yurutucu);

    // 25 araçlı şişkin bir sunucu kataloğu oluştur.
    let mut cok_arac = Vec::new();
    for i in 1..=25 {
        cok_arac.push(arac_olust(
            &format!("arac_{i:02}"),
            "ortak arama kelimesi içeren uzak test aracı",
            json!({
                "type": "object",
                "properties": { "deger": { "type": "string" } }
            }),
            None,
        ));
    }
    katalog.sunucu_katalogunu_guncelle(sunucu_girdisi("buyuk", 19201, 1, 1, 1, cok_arac, vec![]));

    // 1. Arama sonucu en çok 8 aday (`AZAMI_UZAK_ADAY_ARAC_SINIRI`) döner.
    let adaylar = katalog.ara("ortak", None);
    assert_eq!(adaylar.len(), AZAMI_UZAK_ADAY_ARAC_SINIRI);

    // 2. Oturuma en çok 16 uzak araç (`AZAMI_UZAK_ETKIN_ARAC_SINIRI`) yüklenebilir.
    let ilk_16: Vec<String> = (1..=16)
        .map(|i| format!("mcp__buyuk__arac_{i:02}"))
        .collect();
    let yuklenen = kaynak.araclari_yukle(&ilk_16).unwrap();
    assert_eq!(yuklenen.len(), AZAMI_UZAK_ETKIN_ARAC_SINIRI);
    assert_eq!(kaynak.etkin_arac_sayisi(), AZAMI_UZAK_ETKIN_ARAC_SINIRI);

    // 17. aracı yükleme denemesi EtkinAracSiniriAsildi ile reddedilir.
    let on_yedinci = vec!["mcp__buyuk__arac_17".to_owned()];
    assert_eq!(
        kaynak.araclari_yukle(&on_yedinci),
        Err(UzakKesifHatasi::EtkinAracSiniriAsildi {
            sinir: AZAMI_UZAK_ETKIN_ARAC_SINIRI
        })
    );
    assert_eq!(kaynak.etkin_arac_sayisi(), AZAMI_UZAK_ETKIN_ARAC_SINIRI);

    // 3. 64 KiB (`AZAMI_UZAK_SEMA_BAYT_SINIRI`) şema bütçesi sınırı korunur.
    let katalog_2 = Arc::new(UzakAracKatalogu::default());
    let yurutucu_2 = Arc::new(SahteUzakYurutucu {
        katalog: Arc::clone(&katalog_2),
    });
    let kaynak_2 = UzakAracKaynagi::yeni(Arc::clone(&katalog_2), yurutucu_2);

    let buyuk_metin = "x".repeat(12_000);
    let mut iri_araclar = Vec::new();
    for i in 1..=6 {
        iri_araclar.push(arac_olust(
            &format!("iri_{i}"),
            "büyük şemalı araç",
            json!({
                "type": "object",
                "properties": {
                    "alan": {
                        "type": "string",
                        "description": buyuk_metin
                    }
                }
            }),
            None,
        ));
    }
    katalog_2.sunucu_katalogunu_guncelle(sunucu_girdisi(
        "sema",
        19202,
        1,
        1,
        1,
        iri_araclar,
        vec![],
    ));

    // İlk 5 araç (~60 KiB) 64 KiB bütçesine sığar; 6. araç (~72 KiB toplam) SemaBaytSiniriAsildi döner!
    let ilk_5: Vec<String> = (1..=5).map(|i| format!("mcp__sema__iri_{i}")).collect();
    assert!(kaynak_2.araclari_yukle(&ilk_5).is_ok());
    assert!(kaynak_2.kullanilan_sema_bayt() <= AZAMI_UZAK_SEMA_BAYT_SINIRI);

    let altinci = vec!["mcp__sema__iri_6".to_owned()];
    let hata = kaynak_2.araclari_yukle(&altinci).unwrap_err();
    assert!(matches!(
        hata,
        UzakKesifHatasi::SemaBaytSiniriAsildi {
            sinir: AZAMI_UZAK_SEMA_BAYT_SINIRI,
            ..
        }
    ));
    assert!(kaynak_2.kullanilan_sema_bayt() <= AZAMI_UZAK_SEMA_BAYT_SINIRI);
}

#[tokio::test]
async fn m04_sir_iceren_tanim_kataloga_girmez_ve_eski_surum_cagrisi_onay_eskidi_doner() {
    let katalog = Arc::new(UzakAracKatalogu::default());
    let yurutucu = Arc::new(SahteUzakYurutucu {
        katalog: Arc::clone(&katalog),
    });
    let kaynak = UzakAracKaynagi::yeni(Arc::clone(&katalog), yurutucu);
    let gizli = GizliDeger::yeni(CANARY_SIR.to_owned()).unwrap();

    // 1. Sır içeren araç tanımı G7 kapısında reddedilir ve kataloğa girmez.
    let reddedilenler = katalog.sunucu_katalogunu_guncelle(sunucu_girdisi(
        "guvenlik",
        19301,
        1,
        1,
        1,
        vec![
            arac_olust(
                "temiz_arac",
                "Güvenli araç",
                json!({"type": "object"}),
                None,
            ),
            arac_olust(
                "sizinti_araci",
                &format!("Açıklamada sır sızdı: {CANARY_SIR}"),
                json!({"type": "object"}),
                None,
            ),
        ],
        vec![gizli],
    ));
    assert_eq!(reddedilenler, vec![2]);
    assert!(katalog.ara("sizinti", None).is_empty());
    assert_eq!(
        kaynak.araclari_yukle(&["mcp__guvenlik__sizinti_araci".to_owned()]),
        Err(UzakKesifHatasi::AracBulunamadi(
            "mcp__guvenlik__sizinti_araci".to_owned()
        ))
    );

    // 2. Temiz aracı yükle ve anlık görüntüye al.
    kaynak
        .araclari_yukle(&["mcp__guvenlik__temiz_arac".to_owned()])
        .unwrap();
    let defter = AracKayitDefteri::yeni_kaynakli(vec![], kaynak.clone()).unwrap();
    let goruntu = defter.anlik_goruntu(Kip::Proje);
    let temiz_arac = goruntu.bul("mcp__guvenlik__temiz_arac").unwrap();
    let ilk_hazirlik = temiz_arac.hazirla(&json!({})).await.unwrap();

    // Sunucunun kimlik_revizyonu (veya araç şeması) değiştiğinde eski görüntüdeki
    // araç hazırlığı ve çağrısı OnayEskidi ile reddedilir!
    katalog.sunucu_katalogunu_guncelle(sunucu_girdisi(
        "guvenlik",
        19301,
        1,
        2, // kimlik_revizyonu 1 -> 2 oldu
        1,
        vec![arac_olust(
            "temiz_arac",
            "Güvenli araç",
            json!({"type": "object"}),
            None,
        )],
        vec![],
    ));

    assert_eq!(
        temiz_arac.hazirla(&json!({})).await,
        Err(AracHatasi::OnayEskidi)
    );
    assert_eq!(
        temiz_arac
            .calistir(&json!({}), &ilk_hazirlik, CancellationToken::new())
            .await,
        Err(AracHatasi::OnayEskidi)
    );
}

fn arac_olust(ad: &str, aciklama: &str, girdi_semasi: Value, cikti_semasi: Option<Value>) -> Tool {
    arac_olustur(ad, aciklama, girdi_semasi, cikti_semasi)
}

#[tokio::test]
async fn m05_uzak_mcp_arac_hazirligi_yerel_yalitimli_degildir_ve_uzak_baglam_tasir() {
    let katalog = Arc::new(UzakAracKatalogu::default());
    let yurutucu = Arc::new(SahteUzakYurutucu {
        katalog: Arc::clone(&katalog),
    });
    let kaynak = UzakAracKaynagi::yeni(Arc::clone(&katalog), yurutucu);
    katalog.sunucu_katalogunu_guncelle(sunucu_girdisi(
        "github",
        19401,
        1,
        1,
        1,
        vec![arac_olust(
            "issue_ara",
            "Issue arar",
            json!({"type": "object"}),
            None,
        )],
        vec![],
    ));
    kaynak
        .araclari_yukle(&["mcp__github__issue_ara".to_owned()])
        .unwrap();
    let defter = AracKayitDefteri::yeni_kaynakli(vec![], kaynak).unwrap();
    let goruntu = defter.anlik_goruntu(Kip::Proje);
    let arac = goruntu.bul("mcp__github__issue_ara").unwrap();
    let hazirlik = arac.hazirla(&json!({"q": "kalfa"})).await.unwrap();
    assert_eq!(
        hazirlik.veri_cikisi,
        Some(kalfa_core::protokol::VeriCikisi::UzakMcpCagrisi)
    );
    let mcp = hazirlik.mcp.as_ref().expect("Mcp hazırlığı beklendi");
    assert_ne!(
        mcp.erisim,
        kalfa_core::arac::McpErisim::Yalitimli,
        "Uzak MCP aracı yerel yalıtımlı (AppContainer/bwrap) olarak sınıflandırılamaz"
    );
    assert_eq!(mcp.erisim, kalfa_core::arac::McpErisim::UzakStreamableHttp);
    assert_eq!(mcp.ag.uzak_origin(), Some("http://127.0.0.1:19401"));
    assert_eq!(mcp.ag.kimlik_kapsami(), Some("Yetkisiz"));

    let onay_karti = kalfa_core::protokol::McpOnayKarti::yeni(mcp);
    assert_eq!(
        onay_karti.erisim,
        kalfa_core::arac::McpErisim::UzakStreamableHttp
    );
    assert_eq!(
        onay_karti.uzak_origin.as_deref(),
        Some("http://127.0.0.1:19401")
    );
    assert_eq!(onay_karti.kimlik_kapsami.as_deref(), Some("Yetkisiz"));

    for kip in [Kip::Siki, Kip::Proje, Kip::Serbest] {
        let karar = kalfa_core::izin::mcp_karari(kip, mcp.erisim, false, &mcp.ag);
        assert_eq!(
            karar,
            kalfa_core::izin::Karar::Onayli {
                uyarilar: vec![kalfa_core::izin::MCP_UZAK_VERI_CIKISI_UYARISI.to_string()],
            },
            "uzak MCP çağrısı {kip:?} kipinde dahi veri çıkışı uyarısıyla onay istemelidir"
        );
        assert_eq!(
            kalfa_core::kisayol::teklif(arac.ad(), &hazirlik, &karar),
            None,
            "uzak MCP çağrısına oturum kısayolu önerilemez"
        );
    }
}
