//! Kalfa yetenek programı (Hat 01 — G0 ve G5) sabit görev şeması, 24 görev kümesi,
//! çevrimdışı `Senaryo` koşucusu ve ölçüm sözleşmesi doğrulama testleri.

mod ortak;
mod yetenek_senaryolari;

use serde_json::Value;
use yetenek_senaryolari::{
    CalistirmaOrtami, GorevAilesi, SahteAdimTanimi, SenaryoSonucu, YIRMI_DORT_KIMLIK,
    dogrula_gorev_kumesi, dogrula_olcum_raporu_kaydi, gorev_tanimi_cozumle, gorevi_kostur,
    guvenli_goreli_yol_mu, tum_gorevleri_kostur, yirmi_dort_gorevi_yukle,
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

    // 2. Hiçbir dosya/iz kontrolü içermeyen boş denetleyici reddedilmeli
    let mut bos_denetleyici: Value = serde_json::from_str(&gecerli_ham).unwrap();
    bos_denetleyici["bagimsiz_denetleyici"]["dosya_kontrolleri"] = serde_json::json!([]);
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

    // M01 yerel süreç ve yalıtım önkoşulu açıldığında çevrimdışı sözleşme kontrolünü geçmeli
    let m01 = gorevler.iter().find(|g| g.kimlik == "M01").unwrap();
    let mut m01_ortam = CalistirmaOrtami::g0_cevrimdisi_taban();
    m01_ortam.gercek_yalitim_acik = true;
    m01_ortam.yerel_surec_acik = true;
    let m01_kayit = gorevi_kostur(m01, &m01_ortam).await;
    assert_eq!(
        m01_kayit.sonuc,
        SenaryoSonucu::Gecti,
        "M01 önkoşul açıkken geçmeli: {}",
        m01_kayit.neden
    );

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
