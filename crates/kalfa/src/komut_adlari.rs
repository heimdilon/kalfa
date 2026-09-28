// Pencerelerin komutları (1c tasarımı §6.1). `build.rs` bu dosyayı
// `include!` ile okuyup uygulama manifest'ini üretir; capability dosyaları
// ve `komutlar::kur`'daki işleyici aynı listeyi taşır (tests/izinler.rs
// denetler).

/// Ana pencerenin komutları.
pub const ANA_PENCERE_KOMUTLARI: [&str; 28] = [
    "uygulama_durumu",
    "oturumlari_listele",
    "calisma_alani_sec",
    "oturum_baslat",
    "oturum_ac",
    "oturumu_dinle",
    "paketi_isledim",
    "gecmis_sayfasi",
    "mesaj_gonder",
    "turu_iptal_et",
    "arac_onayi",
    "model_degistir",
    "izin_kipi_degistir",
    "oturumu_kapat",
    "baglanti_ac",
    "anahtar_penceresini_ac",
    "sart_penceresini_ac",
    "yalitimi_dinle",
    "yalitimi_algila",
    "arac_zinciri_dizinleri",
    "arac_zinciri_dizini_ekle",
    "arac_zinciri_dizini_kaldir",
    "arac_zinciri_onarim_gerekli",
    "yalitimsizi_ayarla",
    "yalitim_teknik_raporu",
    "yalitimi_onar",
    "yalitim_calisma_alanlari",
    "yalitim_calisma_alanini_kaldir",
];

/// Şart penceresinin komutları. Şart kabulüne giden tek yol buradaki
/// `sartlari_kabul_et`'tir.
pub const SART_PENCERESI_KOMUTLARI: [&str; 6] = [
    "sart_penceresi_durumu",
    "anahtari_kaydet",
    "sartlari_kabul_et",
    "sartlari_reddet",
    "sart_metnini_goster",
    "baglanti_ac",
];

/// Uçtan uca test derlemesinin komutları (`e2e` özelliği). Yalnızca o
/// derlemede kayıtlıdır; yetkisi açılışta ana pencereye verilir.
pub const E2E_KOMUTLARI: [&str; 4] = [
    "e2e_olcumler",
    "e2e_olcumleri_sifirla",
    "e2e_kayit_uret",
    "e2e_ana_pencereyi_kapat",
];
