//! Kalfa yetenek programı (Hat 01 — G0 ve G5) sabit görev şeması, 24 görev kümesi,
//! çevrimdışı `Senaryo` koşucusu ve ölçüm sözleşmesi doğrulama testleri.

mod ortak;
mod yetenek_senaryolari;

use serde_json::Value;
use yetenek_senaryolari::{
    CalistirmaOrtami, GorevAilesi, McpSinamaKipi, SahteAdimTanimi, SahteCagriTanimi, SenaryoSonucu,
    YIRMI_DORT_KIMLIK, dogrula_gorev_kumesi, dogrula_olcum_raporu_kaydi, gorev_tanimi_cozumle,
    gorevi_kostur, guvenli_goreli_yol_mu, tum_gorevleri_kostur, yirmi_dort_gorevi_yukle,
};

#[test]
fn denetleyicisi_olmayan_yinelenen_ve_guvensiz_yollu_tanimlar_reddedilir() {
    let gecerli_ham = include_str!("yetenek_senaryolari/tanimlar/K01.json").replace("\r\n", "\n");
    let k01 = gorev_tanimi_cozumle(&gecerli_ham).expect("K01 geçerli ayrışmalı");

    // 1. Bağımsız denetleyicisi null olan tanım reddedilmeli
    let mut eksik_denetleyici: Value = serde_json::from_str(&gecerli_ham).unwrap();
    eksik_denetleyici["bagimsiz_denetleyici"] = Value::Null;
    let hata = gorev_tanimi_cozumle(&eksik_denetleyici.to_string())
        .expect_err("denetleyicisiz tanım reddedilmeli");
    assert!(
        hata.contains("denetleyicisi olmayan fixture reddedilmeli"),
        "beklenmeyen hata: {hata}"
    );

    // 2. Hiçbir dosya/davranış/iz kontrolü içermeyen boş denetleyici reddedilmeli
    let mut bos_denetleyici: Value = serde_json::from_str(&gecerli_ham).unwrap();
    bos_denetleyici["bagimsiz_denetleyici"]["dosya_kontrolleri"] = serde_json::json!([]);
    bos_denetleyici["bagimsiz_denetleyici"]["davranis_kontrolleri"] = serde_json::json!([]);
    bos_denetleyici["bagimsiz_denetleyici"]["iz_kontrolleri"]["beklenen_arac_durumlari"] =
        serde_json::json!([]);
    bos_denetleyici["bagimsiz_denetleyici"]["iz_kontrolleri"]["isteklerde_bulunmali"] =
        serde_json::json!([]);
    let hata_bos = gorev_tanimi_cozumle(&bos_denetleyici.to_string())
        .expect_err("boş denetleyici reddedilmeli");
    assert!(
        hata_bos.contains("denetleyicisi olmayan fixture reddedilmeli"),
        "beklenmeyen hata: {hata_bos}"
    );

    // 3. Yinelenen görev kimliği reddedilmeli
    let yinelenen_hata = dogrula_gorev_kumesi(&[k01.clone(), k01.clone()])
        .expect_err("yinelenen kimlik reddedilmeli");
    assert!(yinelenen_hata.contains("yinelenen görev kimliği: `K01`"));

    // 4. Çalışma alanı dışı / güvensiz yollar reddedilmeli
    for guvensiz in [
        "",
        "../gizli.txt",
        "src/../../disari.rs",
        "/etc/passwd",
        "\\sunucu\\paylasim\\dosya.txt",
        "C:\\Windows\\win.ini",
        "alt\\ters_egik.rs",
    ] {
        assert!(
            !guvenli_goreli_yol_mu(guvensiz),
            "güvensiz yol reddedilmeliydi: `{guvensiz}`"
        );
    }
    let mut dis_yol_json: Value = serde_json::from_str(&gecerli_ham).unwrap();
    dis_yol_json["baslangic_dosyalari"][0]["yol"] = Value::String("../disari.rs".to_string());
    let hata_yol = gorev_tanimi_cozumle(&dis_yol_json.to_string())
        .expect_err("çalışma alanı dışı yol reddedilmeli");
    assert!(hata_yol.contains("çalışma alanı dışı/güvensiz yol"));

    // 5. Çevrimdışı tanımda `ag_erisim: true` reddedilmeli
    let mut ag_acik_json: Value = serde_json::from_str(&gecerli_ham).unwrap();
    ag_acik_json["platform_onkosulu"]["ag_erisim"] = Value::Bool(true);
    let hata_ag = gorev_tanimi_cozumle(&ag_acik_json.to_string())
        .expect_err("çevrimdışı tanımda ag_erisim=true reddedilmeli");
    assert!(hata_ag.contains("ag_erisim"));
}

#[tokio::test]
async fn olcum_sozlesmesi_sahte_model_ve_bilinmeyen_tuketim_ihlallerini_reddeder() {
    let gorevler = yirmi_dort_gorevi_yukle().expect("24 görev yüklenmeli");
    let ortam = CalistirmaOrtami::g0_cevrimdisi_taban();
    let normal_kayit = gorevi_kostur(&gorevler[0], &ortam).await;
    dogrula_olcum_raporu_kaydi(&normal_kayit).expect("normal kayıt sözleşmeye uygun olmalı");

    // Kural 1: Sahte sağlayıcıyla geçen senaryo canlı model başarısı olarak işaretlenemez
    let mut sahte_canli_ihlali = normal_kayit.clone();
    sahte_canli_ihlali.canli_model_basarisi = true;
    let hata1 = dogrula_olcum_raporu_kaydi(&sahte_canli_ihlali)
        .expect_err("sahte sağlayıcı kaydı canlı model başarısı olamaz");
    assert!(hata1.contains("canlı model başarısı olarak raporlanamaz"));

    // Kural 2: Bilinmeyen jeton tüketimi 0 olarak yazılamaz (`None` / `bilinmiyor` olmalı)
    let mut sifir_tuketim_ihlali = normal_kayit;
    sifir_tuketim_ihlali.tuketim_biliniyor = false;
    sifir_tuketim_ihlali.girdi_jetonu = Some(0);
    let hata2 = dogrula_olcum_raporu_kaydi(&sifir_tuketim_ihlali)
        .expect_err("bilinmeyen tüketim 0 yazılamaz");
    assert!(hata2.contains("bilinmeyen jeton tüketimi 0 veya sayı olarak yazılamaz"));
}

#[test]
fn yirmi_dort_sabit_gorev_sema_ve_ozet_kontrollerini_gecer() {
    let gorevler = yirmi_dort_gorevi_yukle().expect("24 görev şema hatasız yüklenmeli");
    assert_eq!(gorevler.len(), 24, "toplam 24 sabit görev bulunmalı");

    let yuklenen_kimlikler: Vec<&str> = gorevler.iter().map(|g| g.kimlik.as_str()).collect();
    assert_eq!(yuklenen_kimlikler, YIRMI_DORT_KIMLIK);

    for aile in [
        GorevAilesi::YerelKodlama,
        GorevAilesi::SurecVeOturum,
        GorevAilesi::Mcp,
        GorevAilesi::Web,
    ] {
        let adet = gorevler.iter().filter(|g| g.aile == aile).count();
        assert_eq!(adet, 6, "{} ailesinde tam 6 görev olmalı", aile.kod());
    }

    for g in &gorevler {
        assert_eq!(
            g.dosya_sha256.len(),
            64,
            "{} SHA-256 özeti 64 hex olmalı",
            g.kimlik
        );
        assert!(!g.bagimsiz_denetleyici.aciklama.is_empty());
        assert!(!g.platform_onkosulu.ag_erisim);
    }
}

#[tokio::test]
async fn g0_cevrimdisi_kosucu_iki_kosuda_ayni_sonucu_ve_dort_durumu_uretir() {
    let gorevler = yirmi_dort_gorevi_yukle().expect("24 görev yüklenmeli");
    let ortam = CalistirmaOrtami::g0_cevrimdisi_taban();

    let ilk_ozet = tum_gorevleri_kostur(&gorevler, &ortam, "4041a39")
        .await
        .expect("ilk koşu başarılı olmalı");
    let ikinci_ozet = tum_gorevleri_kostur(&gorevler, &ortam, "4041a39")
        .await
        .expect("ikinci koşu başarılı olmalı");

    // İki ardışık koşu birebir aynı sonucu ve deterministik çalışma izi özetini üretmeli
    assert_eq!(ilk_ozet, ikinci_ozet, "iki çevrimdışı koşu özdeş olmalı");

    // 4 durum dağılımı: K01-K05 (5 Geçti), M01 (1 Çalıştırılmadı), diğer 18 görev (Desteklenmiyor), 0 Kaldı
    assert_eq!(ilk_ozet.sayim(SenaryoSonucu::Gecti), 5);
    assert_eq!(ilk_ozet.sayim(SenaryoSonucu::Kaldi), 0);
    assert_eq!(ilk_ozet.sayim(SenaryoSonucu::Desteklenmiyor), 18);
    assert_eq!(ilk_ozet.sayim(SenaryoSonucu::Calistirilmadi), 1);
    assert_eq!(
        ilk_ozet.sayim(SenaryoSonucu::Gecti)
            + ilk_ozet.sayim(SenaryoSonucu::Kaldi)
            + ilk_ozet.sayim(SenaryoSonucu::Desteklenmiyor)
            + ilk_ozet.sayim(SenaryoSonucu::Calistirilmadi),
        24
    );

    // Hiçbir kayıt canlı model başarısı olarak işaretlenmemeli
    for kayit in &ilk_ozet.kayitlar {
        assert!(!kayit.canli_model_basarisi);
        assert_eq!(kayit.saglayici_turu, "sahte_saglayici");
    }

    // Bağımsız denetleyici ihlal edildiğinde `Kaldı` (`Kaldi`) durumu üretilmeli
    let mut bozuk_k01 = gorevler[0].clone();
    bozuk_k01.sahte_model_adimlari = vec![SahteAdimTanimi::Metin(
        "Dosyayı düzenlemeden bitirdim.".to_string(),
    )];
    let bozuk_kayit = gorevi_kostur(&bozuk_k01, &ortam).await;
    assert_eq!(bozuk_kayit.sonuc, SenaryoSonucu::Kaldi);
    assert!(bozuk_kayit.neden.contains("Bağımsız denetleyici hatası"));

    // JSON ve Markdown çıktı biçimleri sözleşme alanlarını içermeli
    let json_cikti = ilk_ozet.json_degeri();
    assert_eq!(json_cikti["toplam_gorev"], 24);
    assert_eq!(json_cikti["ozet_sayilar"]["gecti"], 5);
    assert_eq!(json_cikti["ozet_sayilar"]["desteklenmiyor"], 18);
    assert_eq!(json_cikti["ozet_sayilar"]["calistirilmadi"], 1);

    let md_tablo = ilk_ozet.markdown_ozet_tablosu();
    assert!(md_tablo.contains("| `K01` |"));
    assert!(md_tablo.contains("| `W06` |"));
    assert!(md_tablo.contains("Hayır (`sahte_saglayici`)"));
}

#[tokio::test]
async fn sp02_yorum_icinde_alt_dizge_ile_yanlis_kod_k01_k02_k03_kabulunu_gecemez() {
    let gorevler = yirmi_dort_gorevi_yukle().expect("24 görev yüklenmeli");
    let ortam = CalistirmaOrtami::g0_cevrimdisi_taban();

    // 1. K01: Yanlış kodu koruyup yoruma beklenen düzeltme dizgesini eklemek Kaldi üretmeli
    let mut sahte_k01 = gorevler.iter().find(|g| g.kimlik == "K01").unwrap().clone();
    sahte_k01.sahte_model_adimlari = vec![
        SahteAdimTanimi::Araclar(vec![
            SahteCagriTanimi {
                ad: "dosya_oku".to_string(),
                argumanlar: serde_json::json!({ "yol": "src/dilim.rs" }),
            },
            SahteCagriTanimi {
                ad: "dosya_oku".to_string(),
                argumanlar: serde_json::json!({ "yol": "tests/sinir_testi.rs" }),
            },
        ]),
        SahteAdimTanimi::Arac(SahteCagriTanimi {
            ad: "dosya_duzenle".to_string(),
            argumanlar: serde_json::json!({
                "yol": "src/dilim.rs",
                "eski_metin": "    let baslangic = dilim.len() - n - 1;",
                "yeni_metin": "    let baslangic = dilim.len() - n;\n    // dilim.len().saturating_sub(n)"
            }),
        }),
        SahteAdimTanimi::Metin("Sözde düzeltildi.".to_string()),
    ];
    let k01_sonuc = gorevi_kostur(&sahte_k01, &ortam).await;
    assert_eq!(
        k01_sonuc.sonuc,
        SenaryoSonucu::Kaldi,
        "SP-02 regresyonu: K01 yorum içine eklenen alt dizgeyle veya bozuk sınır hesabıyla geçmemeli: {:?}",
        k01_sonuc
    );

    // 2. K02: Yanlış kodu koruyup yoruma beklenen düzeltme dizgesini eklemek Kaldi üretmeli
    let mut sahte_k02 = gorevler.iter().find(|g| g.kimlik == "K02").unwrap().clone();
    sahte_k02.sahte_model_adimlari = vec![
        SahteAdimTanimi::Araclar(vec![
            SahteCagriTanimi {
                ad: "dosya_oku".to_string(),
                argumanlar: serde_json::json!({ "yol": "src/tipler.ts" }),
            },
            SahteCagriTanimi {
                ad: "dosya_oku".to_string(),
                argumanlar: serde_json::json!({ "yol": "src/donustur.ts" }),
            },
        ]),
        SahteAdimTanimi::Arac(SahteCagriTanimi {
            ad: "dosya_duzenle".to_string(),
            argumanlar: serde_json::json!({
                "yol": "src/donustur.ts",
                "eski_metin": "  const gecerliler = kayitlar.filter((k) => !k.iptal && k.tutarKurus > 0);\n  const toplam = gecerliler.reduce((acc, k) => acc + k.tutarKurus, 0);\n  return {\n    gecerliSayisi: gecerliler.length,\n    toplamKurus: String(toplam) as unknown as number,\n  };",
                "yeni_metin": "  const gecerliler = kayitlar.filter((k) => !k.iptal && k.tutarKurus > 0);\n  // !k.iptal && k.tutarKurus >= 0\n  // toplamKurus: toplam\n  const toplam = gecerliler.reduce((acc, k) => acc + k.tutarKurus, 0);\n  return {\n    gecerliSayisi: gecerliler.length,\n    toplamKurus: toplam,\n  };"
            }),
        }),
        SahteAdimTanimi::Metin("Sözde düzeltildi.".to_string()),
    ];
    let k02_sonuc = gorevi_kostur(&sahte_k02, &ortam).await;
    assert_eq!(
        k02_sonuc.sonuc,
        SenaryoSonucu::Kaldi,
        "SP-02 regresyonu: K02 yorum içine eklenen alt dizgeyle ve bozuk sıfır filtresiyle geçmemeli: {:?}",
        k02_sonuc
    );

    // 3. K03: Yanlış kodu koruyup yoruma beklenen düzeltme dizgesini eklemek Kaldi üretmeli
    let mut sahte_k03 = gorevler.iter().find(|g| g.kimlik == "K03").unwrap().clone();
    sahte_k03.sahte_model_adimlari = vec![
        SahteAdimTanimi::Arac(SahteCagriTanimi {
            ad: "dosya_oku".to_string(),
            argumanlar: serde_json::json!({ "yol": "src/SayacPaneli.svelte" }),
        }),
        SahteAdimTanimi::Arac(SahteCagriTanimi {
            ad: "dosya_duzenle".to_string(),
            argumanlar: serde_json::json!({
                "yol": "src/SayacPaneli.svelte",
                "eski_metin": "  function sifirla() {\n    sayac = 1;\n  }",
                "yeni_metin": "  function sifirla() {\n    sayac = 0;\n    // seciliEtiket = null;\n  }"
            }),
        }),
        SahteAdimTanimi::Metin("Sözde düzeltildi.".to_string()),
    ];
    let k03_sonuc = gorevi_kostur(&sahte_k03, &ortam).await;
    assert_eq!(
        k03_sonuc.sonuc,
        SenaryoSonucu::Kaldi,
        "SP-02 regresyonu: K03 yorum içine eklenen alt dizgeyle ve eksik durum sıfırlamasıyla geçmemeli: {:?}",
        k03_sonuc
    );
}

#[tokio::test]
async fn sp01_m01_dosya_oku_taklidi_kabul_edilmez() {
    let gorevler = yirmi_dort_gorevi_yukle().expect("24 görev yüklenmeli");
    let m01 = gorevler.iter().find(|g| g.kimlik == "M01").unwrap();

    // 1. M01 adımlarında `dosya_oku` yerine gerçek yerel MCP aracı (`mcp__yerel_sinama__dene`) çağrılmalı
    let mcp_cagrisi_var = m01.sahte_model_adimlari.iter().any(|adim| match adim {
        SahteAdimTanimi::Arac(c) => c.ad == "mcp__yerel_sinama__dene",
        SahteAdimTanimi::Araclar(liste) => liste.iter().any(|c| c.ad == "mcp__yerel_sinama__dene"),
        SahteAdimTanimi::Metin(_) => false,
    });
    assert!(
        mcp_cagrisi_var,
        "SP-01 regresyonu: M01 yalnız dosya_oku çağırmamalı, gerçek MCP aracını çağırmalı"
    );

    // 2. Yetenek desteklenmediğinde `Desteklenmiyor` dönmeli
    let mut yeteneksiz_ortam = CalistirmaOrtami::g0_cevrimdisi_taban();
    yeteneksiz_ortam
        .desteklenen_yetenekler
        .remove("yerel_mcp_stdio");
    let yeteneksiz_kayit = gorevi_kostur(m01, &yeteneksiz_ortam).await;
    assert_eq!(yeteneksiz_kayit.sonuc, SenaryoSonucu::Desteklenmiyor);

    // 3. Önkoşul kapalıyken `Çalıştırılmadı` dönmeli
    let kapali_ortam = CalistirmaOrtami::g0_cevrimdisi_taban();
    let kapali_kayit = gorevi_kostur(m01, &kapali_ortam).await;
    assert_eq!(kapali_kayit.sonuc, SenaryoSonucu::Calistirilmadi);

    // 4. MCP stdio bağlantısı / el sıkışması bozuk olduğunda denetleyici `Kaldı` dönmeli
    let mut bozuk_baglanti_ortami = CalistirmaOrtami::g0_cevrimdisi_taban();
    bozuk_baglanti_ortami.gercek_yalitim_acik = true;
    bozuk_baglanti_ortami.yerel_surec_acik = true;
    bozuk_baglanti_ortami.mcp_sinama_kipi = McpSinamaKipi::BozukBaglanti;
    let bozuk_baglanti_kaydi = gorevi_kostur(m01, &bozuk_baglanti_ortami).await;
    assert_eq!(
        bozuk_baglanti_kaydi.sonuc,
        SenaryoSonucu::Kaldi,
        "SP-01 regresyonu: bozuk MCP bağlantısı Kaldı üretmeli: {:?}",
        bozuk_baglanti_kaydi
    );

    // 5. MCP yalıtımı/koruması bozuk olduğunda (`.env` ve alan dışı sır sızdığında) `Kaldı` dönmeli
    let mut kacak_ortam = CalistirmaOrtami::g0_cevrimdisi_taban();
    kacak_ortam.gercek_yalitim_acik = true;
    kacak_ortam.yerel_surec_acik = true;
    kacak_ortam.mcp_sinama_kipi = McpSinamaKipi::YalitimsizKacak;
    let kacak_kaydi = gorevi_kostur(m01, &kacak_ortam).await;
    assert_eq!(
        kacak_kaydi.sonuc,
        SenaryoSonucu::Kaldi,
        "SP-01 regresyonu: yalıtımsız/korumasız MCP sunucusu Kaldı üretmeli: {:?}",
        kacak_kaydi
    );
    assert!(
        kacak_kaydi.neden.contains("GIZLI_ENV=KORUNDU")
            || kacak_kaydi.neden.contains("gizli-m01-degeri")
            || kacak_kaydi.neden.contains("DIS_ALAN_SIRRI_M01"),
        "sır sızıntısı denetleyici hatasında görünmeli: {}",
        kacak_kaydi.neden
    );

    // 6. Eski SP-01 hatasının birebir tekrarı: M01 adımlarını yalnız `dosya_oku` ile taklit etmek
    // hiçbir kipte `Geçti` döndüremez (`Kaldı` veya ortam kapalıysa `Çalıştırılmadı` döner).
    let mut taklit_m01 = m01.clone();
    taklit_m01.sahte_model_adimlari = vec![
        SahteAdimTanimi::Arac(SahteCagriTanimi {
            ad: "dosya_oku".to_string(),
            argumanlar: serde_json::json!({ "yol": "mcp/yerel_durum.json" }),
        }),
        SahteAdimTanimi::Metin(
            "Yerel stdio MCP durumu: KLF-M01-STDIO-OK; .env erişimi reddedildi.".to_string(),
        ),
    ];
    let taklit_kaydi = gorevi_kostur(&taklit_m01, &kacak_ortam).await;
    assert_eq!(
        taklit_kaydi.sonuc,
        SenaryoSonucu::Kaldi,
        "SP-01 regresyonu: yalnız dosya_oku çağıran taklit M01 senaryosu Kaldı üretmeli: {:?}",
        taklit_kaydi
    );

    // 7. Gerçek yalıtım kipi: yalnız iki ortam bayrağını açmak sahte başarı üretmez;
    // gerçek `kalfa-mcp-test-sunucusu` + `kalfa-kopru` + platform yalıtımı hazırsa `Geçti`,
    // hazır değilse `Çalıştırılmadı` döner.
    let mut gercek_ortam = CalistirmaOrtami::g0_cevrimdisi_taban();
    gercek_ortam.gercek_yalitim_acik = true;
    gercek_ortam.yerel_surec_acik = true;
    gercek_ortam.mcp_sinama_kipi = McpSinamaKipi::GercekYalitim;
    let gercek_kayit = gorevi_kostur(m01, &gercek_ortam).await;
    println!(
        "M01 GercekYalitim sonucu: {:?} — {}",
        gercek_kayit.sonuc, gercek_kayit.neden
    );
    assert!(
        matches!(
            gercek_kayit.sonuc,
            SenaryoSonucu::Gecti | SenaryoSonucu::Calistirilmadi
        ),
        "beklenmeyen M01 gerçek yalıtım sonucu: {:?}",
        gercek_kayit
    );
}

#[test]
fn olcum_belgeleri_yirmi_dort_gorev_sha256_ozetleriyle_uyusur() {
    let gorevler = yirmi_dort_gorevi_yukle().expect("24 görev yüklenmeli");
    let depo_koku = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("depo kökü bulunmalı");
    let sozlesme_yolu = depo_koku
        .join("docs")
        .join("superpowers")
        .join("bulgular")
        .join("yetenek-programi")
        .join("olcum-sozlesmesi.md");
    let g0_yolu = depo_koku
        .join("docs")
        .join("superpowers")
        .join("bulgular")
        .join("yetenek-programi")
        .join("olcum-g0-baslangic.md");

    let sozlesme = std::fs::read_to_string(&sozlesme_yolu).expect("olcum-sozlesmesi.md okunmalı");
    let g0_rapor = std::fs::read_to_string(&g0_yolu).expect("olcum-g0-baslangic.md okunmalı");

    for g in &gorevler {
        assert!(
            sozlesme.contains(&g.dosya_sha256),
            "olcum-sozlesmesi.md içinde `{}` görevinin tam SHA-256 özeti (`{}`) bulunmalı",
            g.kimlik,
            g.dosya_sha256
        );
        let kisa_sha = &g.dosya_sha256[..16];
        assert!(
            g0_rapor.contains(kisa_sha),
            "olcum-g0-baslangic.md içinde `{}` görevinin kısa SHA-256 özeti (`{}`) bulunmalı",
            g.kimlik,
            kisa_sha
        );
    }
}
