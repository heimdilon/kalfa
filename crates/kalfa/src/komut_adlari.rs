// Pencerelerin komutları (1c tasarımı §6.1). `build.rs` bu dosyayı
// `include!` ile okuyup uygulama manifest'ini üretir; capability dosyaları
// ve `komutlar::kur`'daki işleyici aynı listeyi taşır (tests/izinler.rs
// denetler).

/// Ana pencerenin komutları.
pub const ANA_PENCERE_KOMUTLARI: [&str; 17] = [
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
