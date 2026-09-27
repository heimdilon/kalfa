//! Pencerelerin komutları (1c tasarımı §6.1). Her komut `Uygulama`'nın aynı
//! adlı yöntemine iner; hangi pencerenin hangi komutu çağırabileceğini
//! `capabilities/` belirler. Komutların hepsi async'tir: Tauri eşzamanlı
//! komutları ana iş parçacığında çalıştırır ve orada pencere kurmak Windows'ta
//! kilitlenir.

use std::sync::Arc;

use kalfa_core::aktarim::KanalMesaji;
use kalfa_core::gecmis::GecmisSayfasi;
use kalfa_core::izin::Kip;
use kalfa_core::kimlik::OturumId;
use kalfa_uygulama::Uygulama;
use kalfa_uygulama::arayuz::YalitimDurumu;
use kalfa_uygulama::hata::KomutHatasi;
use kalfa_uygulama::sozlesme::{
    AnahtarSonucu, CalismaAlani, KabulYaniti, OnarimSonucu, OnayGirdisi, OturumAcilisi,
    OturumSatiri, SartPenceresiDurumu, UygulamaDurumu, YalitimCalismaAlani,
};
use tauri::ipc::Channel;
use tauri::{Builder, Runtime, State};

use crate::kanal::{TauriKanal, TauriYalitimKanali};

type Sonuc<T> = Result<T, KomutHatasi>;

/// Komut işleyicisini kurar. Liste `komut_adlari.rs` ile aynıdır
/// (tests/izinler.rs denetler); uçtan uca test derlemesi test komutlarını
/// ekler.
pub fn kur<R: Runtime>(kurucu: Builder<R>) -> Builder<R> {
    kurucu.invoke_handler(tauri::generate_handler![
        uygulama_durumu,
        oturumlari_listele,
        calisma_alani_sec,
        oturum_baslat,
        oturum_ac,
        oturumu_dinle,
        paketi_isledim,
        gecmis_sayfasi,
        mesaj_gonder,
        turu_iptal_et,
        arac_onayi,
        model_degistir,
        izin_kipi_degistir,
        oturumu_kapat,
        baglanti_ac,
        anahtar_penceresini_ac,
        sart_penceresini_ac,
        yalitimi_dinle,
        yalitimi_algila,
        arac_zinciri_dizinleri,
        arac_zinciri_dizini_ekle,
        arac_zinciri_dizini_kaldir,
        yalitimsizi_ayarla,
        yalitim_teknik_raporu,
        yalitimi_onar,
        yalitim_calisma_alanlari,
        yalitim_calisma_alanini_kaldir,
        sart_penceresi_durumu,
        anahtari_kaydet,
        sartlari_kabul_et,
        sartlari_reddet,
        sart_metnini_goster,
        #[cfg(feature = "e2e")]
        crate::e2e::e2e_olcumler,
        #[cfg(feature = "e2e")]
        crate::e2e::e2e_olcumleri_sifirla,
        #[cfg(feature = "e2e")]
        crate::e2e::e2e_kayit_uret,
        #[cfg(feature = "e2e")]
        crate::e2e::e2e_ana_pencereyi_kapat,
    ])
}

// --- Ana pencere ---

#[tauri::command(rename_all = "snake_case")]
pub async fn uygulama_durumu(u: State<'_, Arc<Uygulama>>) -> Sonuc<UygulamaDurumu> {
    Ok(u.uygulama_durumu())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn oturumlari_listele(u: State<'_, Arc<Uygulama>>) -> Sonuc<Vec<OturumSatiri>> {
    u.oturumlari_listele().await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn calisma_alani_sec(u: State<'_, Arc<Uygulama>>) -> Sonuc<Option<CalismaAlani>> {
    Ok(u.calisma_alani_sec().await)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn oturum_baslat(
    u: State<'_, Arc<Uygulama>>,
    calisma_alani: u64,
    model: String,
) -> Sonuc<OturumAcilisi> {
    u.oturum_baslat(calisma_alani, model).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn oturum_ac(u: State<'_, Arc<Uygulama>>, oturum: OturumId) -> Sonuc<OturumAcilisi> {
    u.oturum_ac(oturum).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn oturumu_dinle(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    kanal: Channel<KanalMesaji>,
) -> Sonuc<()> {
    u.oturumu_dinle(oturum, Box::new(TauriKanal(kanal)))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn paketi_isledim(u: State<'_, Arc<Uygulama>>, oturum: OturumId, no: u64) -> Sonuc<()> {
    u.paketi_isledim(oturum, no)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn gecmis_sayfasi(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    once_sira: u64,
) -> Sonuc<GecmisSayfasi> {
    u.gecmis_sayfasi(oturum, once_sira).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mesaj_gonder(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    metin: String,
) -> Sonuc<()> {
    u.mesaj_gonder(oturum, metin).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn turu_iptal_et(u: State<'_, Arc<Uygulama>>, oturum: OturumId) -> Sonuc<()> {
    u.turu_iptal_et(oturum)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn arac_onayi(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    kararlar: Vec<OnayGirdisi>,
) -> Sonuc<()> {
    u.arac_onayi(oturum, kararlar).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_degistir(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    model: String,
) -> Sonuc<()> {
    u.model_degistir(oturum, model).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn izin_kipi_degistir(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    kip: Kip,
) -> Sonuc<Kip> {
    u.izin_kipi_degistir(oturum, kip).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn oturumu_kapat(u: State<'_, Arc<Uygulama>>, oturum: OturumId) -> Sonuc<()> {
    u.oturumu_kapat(oturum).await
}

/// İki pencerede de var: şart metnindeki bağlantılar da yerel onayla açılır.
#[tauri::command(rename_all = "snake_case")]
pub async fn baglanti_ac(u: State<'_, Arc<Uygulama>>, adres: String) -> Sonuc<()> {
    u.baglanti_ac(adres).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn anahtar_penceresini_ac(u: State<'_, Arc<Uygulama>>) -> Sonuc<()> {
    u.anahtar_penceresini_ac();
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn sart_penceresini_ac(u: State<'_, Arc<Uygulama>>) -> Sonuc<()> {
    u.sart_penceresini_ac();
    Ok(())
}

/// Ana pencere açılınca bir kez çağırır; durum değişimi bu kanala yazılır.
#[tauri::command(rename_all = "snake_case")]
pub async fn yalitimi_dinle(
    u: State<'_, Arc<Uygulama>>,
    kanal: Channel<YalitimDurumu>,
) -> Sonuc<()> {
    u.yalitimi_dinle(Box::new(TauriYalitimKanali(kanal)));
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn yalitimi_algila(u: State<'_, Arc<Uygulama>>) -> Sonuc<YalitimDurumu> {
    u.yalitimi_algila().await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn arac_zinciri_dizinleri(u: State<'_, Arc<Uygulama>>) -> Sonuc<Vec<String>> {
    Ok(u.arac_zinciri_dizinleri())
}

/// Dizin yerel klasör seçiciyle seçilir; arayüz yol gönderemez.
#[tauri::command(rename_all = "snake_case")]
pub async fn arac_zinciri_dizini_ekle(u: State<'_, Arc<Uygulama>>) -> Sonuc<Vec<String>> {
    u.arac_zinciri_dizini_ekle().await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn arac_zinciri_dizini_kaldir(
    u: State<'_, Arc<Uygulama>>,
    yol: String,
) -> Sonuc<Vec<String>> {
    u.arac_zinciri_dizini_kaldir(&yol)
}

/// Yalıtımsız çalıştırma anahtarı (aşama 2 tasarımı §9.8); açmak yerel
/// pencerede onaylanır. Yeni değeri döner.
#[tauri::command(rename_all = "snake_case")]
pub async fn yalitimsizi_ayarla(u: State<'_, Arc<Uygulama>>, acik: bool) -> Sonuc<bool> {
    u.yalitimsizi_ayarla(acik).await
}

/// BT için teknik rapor (aşama 2 tasarımı §9.8); arayüz panoya kopyalar.
#[tauri::command(rename_all = "snake_case")]
pub async fn yalitim_teknik_raporu(u: State<'_, Arc<Uygulama>>) -> Sonuc<String> {
    Ok(u.yalitim_teknik_raporu())
}

/// "Onar" (aşama 2 tasarımı §9.8): yerel onay, sonra kurulu yardımcı UAC ile.
#[tauri::command(rename_all = "snake_case")]
pub async fn yalitimi_onar(u: State<'_, Arc<Uygulama>>) -> Sonuc<OnarimSonucu> {
    u.yalitimi_onar().await
}

/// Yalıtım için hazırlanmış çalışma alanlarını kimlik ve gösterim yoluyla
/// listeler. Yol, arayüzün arka uca gönderebildiği bir girdi değildir.
#[tauri::command(rename_all = "snake_case")]
pub async fn yalitim_calisma_alanlari(
    u: State<'_, Arc<Uygulama>>,
) -> Sonuc<Vec<YalitimCalismaAlani>> {
    Ok(u.yalitim_calisma_alanlari())
}

/// İzinleri yerel onaydan sonra geri alır; açık oturumu olan alan reddedilir.
#[tauri::command(rename_all = "snake_case")]
pub async fn yalitim_calisma_alanini_kaldir(
    u: State<'_, Arc<Uygulama>>,
    kimlik: String,
) -> Sonuc<bool> {
    u.yalitim_calisma_alanini_kaldir(&kimlik).await
}

// --- Şart penceresi ---

#[tauri::command(rename_all = "snake_case")]
pub async fn sart_penceresi_durumu(u: State<'_, Arc<Uygulama>>) -> Sonuc<SartPenceresiDurumu> {
    Ok(u.sart_penceresi_durumu())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn anahtari_kaydet(u: State<'_, Arc<Uygulama>>, anahtar: String) -> Sonuc<AnahtarSonucu> {
    u.inner().anahtari_kaydet(anahtar).await
}

/// Şart kabulüne giden tek yol (üst belge §11.8).
#[tauri::command(rename_all = "snake_case")]
pub async fn sartlari_kabul_et(u: State<'_, Arc<Uygulama>>, surum: u32) -> Sonuc<KabulYaniti> {
    u.sartlari_kabul_et(surum).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn sartlari_reddet(u: State<'_, Arc<Uygulama>>) -> Sonuc<()> {
    u.sartlari_reddet();
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn sart_metnini_goster(u: State<'_, Arc<Uygulama>>) -> Sonuc<SartPenceresiDurumu> {
    u.sart_metnini_goster().await
}
