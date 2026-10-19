//! Pencerelerin komutları (1c tasarımı §6.1). Her komut `Uygulama`'nın aynı
//! adlı yöntemine iner; hangi pencerenin hangi komutu çağırabileceğini
//! `capabilities/` belirler. Komutların hepsi async'tir: Tauri eşzamanlı
//! komutları ana iş parçacığında çalıştırır ve orada pencere kurmak Windows'ta
//! kilitlenir.

use std::sync::Arc;

use kalfa_core::aktarim::KanalMesaji;
use kalfa_core::denetci::DenetimKipi;
use kalfa_core::gecmis::GecmisSayfasi;
use kalfa_core::izin::Kip;
use kalfa_core::kimlik::{CagriId, IslemId, OturumId, TurId};
use kalfa_core::muhur::OnayMuhru;
use kalfa_uygulama::Uygulama;
use kalfa_uygulama::arayuz::YalitimDurumu;
use kalfa_uygulama::dosya_gorunumu::{CalismaAlaniOzeti, DizinIcerigi, DosyaIcerigi};
use kalfa_uygulama::hata::KomutHatasi;
use kalfa_uygulama::mcp::komutlar::{
    McpDurumuDto, McpSunucuAyrintisiDto, McpSunucuFormu, McpTaslakDto,
};
use kalfa_uygulama::mcp::tanimlar::{IceAktarmaOnizlemeDto, SeciliKomutBilgisi, TaslakKimligi};
use kalfa_uygulama::sozlesme::{
    AgHostu, AnahtarSonucu, CalismaAlani, DenetciDurumu, DusunceEforu, GizliAyarGorunumu,
    GunlukSayfasi, KabulYaniti, ModelSecenegi, OnarimSonucu, OnayGirdisi, OturumAcilisi,
    OturumSatiri, SartPenceresiDurumu, UygulamaDurumu, YalitimCalismaAlani,
};
use kalfa_uygulama::sozlesme::{
    AlanHedefi, EklentiSatiri, GunlukSinirDurumu, KurulumSonucu, OturumEklentisi,
};
use tauri::ipc::Channel;
use tauri::{Builder, Runtime, State};

use crate::kanal::{TauriKanal, TauriMcpKanali, TauriYalitimKanali};

type Sonuc<T> = Result<T, KomutHatasi>;

/// Komut işleyicisini kurar. Liste `komut_adlari.rs` ile aynıdır
/// (tests/izinler.rs denetler); uçtan uca test derlemesi test komutlarını
/// ekler.
pub fn kur<R: Runtime>(kurucu: Builder<R>) -> Builder<R> {
    kurucu.invoke_handler(tauri::generate_handler![
        uygulama_durumu,
        modelleri_yenile,
        oturumlari_listele,
        calisma_alani_sec,
        oturum_baslat,
        oturum_ac,
        oturumu_dinle,
        paketi_isledim,
        gecmis_sayfasi,
        mesaj_gonder,
        oturum_ozetle,
        turu_iptal_et,
        arac_onayi,
        model_degistir,
        efor_degistir,
        izin_kipi_degistir,
        oturumu_kapat,
        geri_alma_onizle,
        turu_geri_al,
        oturumu_sil,
        geri_alma_verisi_boyutu,
        baglanti_ac,
        anahtar_penceresini_ac,
        sart_penceresini_ac,
        yalitimi_dinle,
        yalitimi_algila,
        arac_zinciri_dizinleri,
        arac_zinciri_dizini_ekle,
        arac_zinciri_dizini_kaldir,
        arac_zinciri_onarim_gerekli,
        yalitimsizi_ayarla,
        yalitim_teknik_raporu,
        yalitimi_onar,
        yalitim_calisma_alanlari,
        yalitim_calisma_alanini_kaldir,
        denetci_kipi,
        denetci_kipini_ayarla,
        kisayolu_kaldir,
        gizli_ayarlari,
        ek_gizli_kalip_ekle,
        ek_gizli_kalip_kaldir,
        gizli_sayilmayan_ekle,
        gizli_sayilmayan_kaldir,
        ag_izin_listesi,
        ag_hostu_ekle,
        ag_hostu_kaldir,
        gonderim_gunlugu,
        gonderim_ogesi,
        onizleme_ayarla,
        gonderim_karari,
        etkin_pencere,
        etkin_pencereyi_ayarla,
        eklentileri_listele,
        eklenti_klasorden_kur,
        eklenti_kaldir,
        eklenti_etkin_ayarla,
        eklenti_alani_ayarla,
        gonderim_iznini_kaldir,
        eklenti_verisini_sil,
        oturum_eklentileri,
        eklenti_istegi,
        gunluk_sinir,
        mcp_dinle,
        mcp_durumu,
        mcp_sunucu_ayrintisi,
        mcp_komut_sec,
        mcp_taslak_olustur,
        mcp_taslak_guncelle,
        mcp_taslak_kaydet,
        mcp_ice_aktar_onizle,
        mcp_ice_aktar_gizlilik,
        mcp_ice_aktar_kaydet,
        mcp_etkin_ayarla,
        mcp_sil,
        mcp_yeniden_baslat,
        mcp_baslatma_karti_yanitla,
        mcp_baslatma_izni_geri_al,
        crate::pencereler::pencere_kucult,
        crate::pencereler::pencere_buyut_degistir,
        crate::pencereler::pencere_kapat,
        crate::pencereler::pencere_surukle,
        calisma_alani_ozeti,
        calisma_alani_dizini,
        calisma_alani_dosyasi,
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
        #[cfg(feature = "e2e")]
        crate::e2e::e2e_pencere_durumu,
    ])
}

// --- Ana pencere ---

#[tauri::command(rename_all = "snake_case")]
pub async fn uygulama_durumu(u: State<'_, Arc<Uygulama>>) -> Sonuc<UygulamaDurumu> {
    Ok(u.uygulama_durumu())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn modelleri_yenile(u: State<'_, Arc<Uygulama>>) -> Sonuc<Vec<ModelSecenegi>> {
    u.modelleri_yenile().await
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
pub async fn oturum_ozetle(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    talimat: Option<String>,
) -> Sonuc<()> {
    u.oturum_ozetle(oturum, talimat).await
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
pub async fn efor_degistir(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    efor: DusunceEforu,
) -> Sonuc<()> {
    u.efor_degistir(oturum, efor).await
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

/// Turun geri alma önizlemesi (aşama 3 tasarımı §5.6); plan olay olarak gelir.
#[tauri::command(rename_all = "snake_case")]
pub async fn geri_alma_onizle(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    tur: TurId,
) -> Sonuc<()> {
    u.geri_alma_onizle(oturum, tur).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn turu_geri_al(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    tur: TurId,
    muhur: OnayMuhru,
) -> Sonuc<()> {
    u.turu_geri_al(oturum, tur, muhur).await
}

/// Yerel pencerede onaylanır (aşama 3 tasarımı §5.10); vazgeçilirse `false`.
#[tauri::command(rename_all = "snake_case")]
pub async fn oturumu_sil(u: State<'_, Arc<Uygulama>>, oturum: OturumId) -> Sonuc<bool> {
    u.oturumu_sil(oturum).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn geri_alma_verisi_boyutu(u: State<'_, Arc<Uygulama>>) -> Sonuc<u64> {
    u.geri_alma_verisi_boyutu().await
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

/// Kurulu sürümde kayıtlı bir araç zinciri dizini kapta okunamıyor mu (2d-3);
/// öyleyse Ayarlar Onar'ı gösterir.
#[tauri::command(rename_all = "snake_case")]
pub async fn arac_zinciri_onarim_gerekli(u: State<'_, Arc<Uygulama>>) -> Sonuc<bool> {
    Ok(u.arac_zinciri_onarim_gerekli().await)
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

#[tauri::command(rename_all = "snake_case")]
pub async fn gonderim_gunlugu(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    once_sira: Option<u64>,
) -> Sonuc<GunlukSayfasi> {
    u.gonderim_gunlugu(oturum, once_sira).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn gonderim_ogesi(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    sira: u64,
) -> Sonuc<String> {
    u.gonderim_ogesi(oturum, sira).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn onizleme_ayarla(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    acik: bool,
) -> Sonuc<()> {
    u.onizleme_ayarla(oturum, acik).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn gonderim_karari(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    islem: IslemId,
    gonderilmeyecekler: Vec<CagriId>,
    hep_gonderilecekler: Vec<CagriId>,
) -> Sonuc<()> {
    u.gonderim_karari(oturum, islem, gonderilmeyecekler, hep_gonderilecekler)
        .await
}

// --- Etkin bağlam penceresi ---

/// Yeni ve açılan oturumların etkin bağlam penceresi (Ayarlar).
#[tauri::command(rename_all = "snake_case")]
pub async fn etkin_pencere(u: State<'_, Arc<Uygulama>>) -> Sonuc<u64> {
    Ok(u.etkin_pencere())
}

/// Etkin bağlam penceresini değiştirir.
#[tauri::command(rename_all = "snake_case")]
pub async fn etkin_pencereyi_ayarla(u: State<'_, Arc<Uygulama>>, deger: u64) -> Sonuc<u64> {
    u.etkin_pencereyi_ayarla(deger)
}

// --- Eklentiler (aşama 4 tasarımı §13, §14) ---

#[tauri::command(rename_all = "snake_case")]
pub async fn eklentileri_listele(u: State<'_, Arc<Uygulama>>) -> Sonuc<Vec<EklentiSatiri>> {
    u.eklentileri_listele().await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn eklenti_klasorden_kur(u: State<'_, Arc<Uygulama>>) -> Sonuc<KurulumSonucu> {
    u.eklenti_klasorden_kur().await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn eklenti_kaldir(u: State<'_, Arc<Uygulama>>, ad: String) -> Sonuc<bool> {
    u.eklenti_kaldir(ad).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn eklenti_etkin_ayarla(
    u: State<'_, Arc<Uygulama>>,
    ad: String,
    etkin: bool,
) -> Sonuc<()> {
    u.eklenti_etkin_ayarla(ad, etkin).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn eklenti_alani_ayarla(
    u: State<'_, Arc<Uygulama>>,
    ad: String,
    hedef: AlanHedefi,
    acik: bool,
) -> Sonuc<()> {
    u.eklenti_alani_ayarla(ad, hedef, acik).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn gonderim_iznini_kaldir(
    u: State<'_, Arc<Uygulama>>,
    ad: String,
    kimlik: String,
) -> Sonuc<()> {
    u.gonderim_iznini_kaldir(ad, kimlik).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn eklenti_verisini_sil(
    u: State<'_, Arc<Uygulama>>,
    ad: String,
    kimlik: String,
) -> Sonuc<bool> {
    u.eklenti_verisini_sil(ad, kimlik).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn oturum_eklentileri(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
) -> Sonuc<Vec<OturumEklentisi>> {
    u.oturum_eklentileri(oturum).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn eklenti_istegi(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    islem: IslemId,
) -> Sonuc<String> {
    u.eklenti_istegi(oturum, islem).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn gunluk_sinir(u: State<'_, Arc<Uygulama>>) -> Sonuc<Option<GunlukSinirDurumu>> {
    Ok(u.gunluk_sinir())
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

/// Denetçi kipi ve ayar dosyasının durumu (3c Görev 7).
#[tauri::command(rename_all = "snake_case")]
pub async fn denetci_kipi(u: State<'_, Arc<Uygulama>>) -> Sonuc<DenetciDurumu> {
    Ok(u.denetci_kipi())
}

/// Denetçi kipini değiştirir; düşürmek yerel pencerede onaylanır.
#[tauri::command(rename_all = "snake_case")]
pub async fn denetci_kipini_ayarla(
    u: State<'_, Arc<Uygulama>>,
    kip: DenetimKipi,
) -> Sonuc<DenetciDurumu> {
    u.denetci_kipini_ayarla(kip).await
}

/// Oturumdaki bir kısayolu kaldırır (aşama 3 tasarımı §9.1).
#[tauri::command(rename_all = "snake_case")]
pub async fn kisayolu_kaldir(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    kimlik: u64,
) -> Sonuc<()> {
    u.kisayolu_kaldir(oturum, kimlik).await
}

/// Ayarlar > "Gizli dosyalar" (3e Görev 4).
#[tauri::command(rename_all = "snake_case")]
pub async fn gizli_ayarlari(u: State<'_, Arc<Uygulama>>) -> Sonuc<GizliAyarGorunumu> {
    Ok(u.gizli_ayarlari())
}

/// Ek gizli kalıp ekler; onaysızdır.
#[tauri::command(rename_all = "snake_case")]
pub async fn ek_gizli_kalip_ekle(
    u: State<'_, Arc<Uygulama>>,
    kalip: String,
) -> Sonuc<GizliAyarGorunumu> {
    u.ek_gizli_kalip_ekle(kalip).await
}

/// Ek gizli kalıbı kaldırır; yerel pencerede onaylanır.
#[tauri::command(rename_all = "snake_case")]
pub async fn ek_gizli_kalip_kaldir(
    u: State<'_, Arc<Uygulama>>,
    kalip: String,
) -> Sonuc<GizliAyarGorunumu> {
    u.ek_gizli_kalip_kaldir(kalip).await
}

/// Gizli sayılmayanlara ekler; yerel pencerede onaylanır.
#[tauri::command(rename_all = "snake_case")]
pub async fn gizli_sayilmayan_ekle(
    u: State<'_, Arc<Uygulama>>,
    kalip: String,
) -> Sonuc<GizliAyarGorunumu> {
    u.gizli_sayilmayan_ekle(kalip).await
}

/// Gizli sayılmayanlardan kaldırır; onaysızdır.
#[tauri::command(rename_all = "snake_case")]
pub async fn gizli_sayilmayan_kaldir(
    u: State<'_, Arc<Uygulama>>,
    kalip: String,
) -> Sonuc<GizliAyarGorunumu> {
    u.gizli_sayilmayan_kaldir(kalip).await
}

/// Kullanıcının ağ izin listesi (aşama 3 tasarımı §8.3).
#[tauri::command(rename_all = "snake_case")]
pub async fn ag_izin_listesi(u: State<'_, Arc<Uygulama>>) -> Sonuc<Vec<AgHostu>> {
    Ok(u.ag_izin_listesi())
}

/// Ağ izin listesine host ekler; yerel pencerede onaylanır.
#[tauri::command(rename_all = "snake_case")]
pub async fn ag_hostu_ekle(
    u: State<'_, Arc<Uygulama>>,
    host: String,
    ozel_ag: bool,
) -> Sonuc<Vec<AgHostu>> {
    u.ag_hostu_ekle(host, ozel_ag).await
}

/// Ağ izin listesinden host kaldırır; onaysızdır.
#[tauri::command(rename_all = "snake_case")]
pub async fn ag_hostu_kaldir(u: State<'_, Arc<Uygulama>>, host: String) -> Sonuc<Vec<AgHostu>> {
    u.ag_hostu_kaldir(host).await
}

// MCP sunucuları (1.1a-2 Görev 10). Arayüz yol göndermez; kayıt, silme ve
// tam erişimli başlatma arka uçta yerel pencereyle onaylanır.

/// Ana pencere açılınca bir kez çağırır; MCP durum görüntüsü bu kanala yazılır.
#[tauri::command(rename_all = "snake_case")]
pub async fn mcp_dinle(u: State<'_, Arc<Uygulama>>, kanal: Channel<McpDurumuDto>) -> Sonuc<()> {
    u.mcp_dinle(Box::new(TauriMcpKanali(kanal)));
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mcp_durumu(u: State<'_, Arc<Uygulama>>) -> Sonuc<McpDurumuDto> {
    Ok(u.mcp_durumu())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mcp_sunucu_ayrintisi(
    u: State<'_, Arc<Uygulama>>,
    ad: String,
) -> Sonuc<McpSunucuAyrintisiDto> {
    u.mcp_sunucu_ayrintisi(&ad)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mcp_komut_sec(u: State<'_, Arc<Uygulama>>) -> Sonuc<Option<SeciliKomutBilgisi>> {
    u.mcp_komut_sec().await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mcp_taslak_olustur(
    u: State<'_, Arc<Uygulama>>,
    eski_ad: Option<String>,
    form: McpSunucuFormu,
) -> Sonuc<McpTaslakDto> {
    u.mcp_taslak_olustur(eski_ad, form)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mcp_taslak_guncelle(
    u: State<'_, Arc<Uygulama>>,
    taslak_kimligi: TaslakKimligi,
    eski_ad: Option<String>,
    form: McpSunucuFormu,
) -> Sonuc<u64> {
    u.mcp_taslak_guncelle(taslak_kimligi, eski_ad, form)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mcp_taslak_kaydet(
    u: State<'_, Arc<Uygulama>>,
    taslak_kimligi: TaslakKimligi,
    kusak: u64,
    oturum: Option<OturumId>,
) -> Sonuc<bool> {
    u.mcp_taslak_kaydet(taslak_kimligi, kusak, oturum).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mcp_ice_aktar_onizle(
    u: State<'_, Arc<Uygulama>>,
    json: String,
) -> Sonuc<IceAktarmaOnizlemeDto> {
    u.mcp_ice_aktar_onizle(&json)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mcp_ice_aktar_gizlilik(
    u: State<'_, Arc<Uygulama>>,
    taslak_kimligi: TaslakKimligi,
    sunucu: String,
    ortam: String,
    gizli: bool,
) -> Sonuc<u64> {
    u.mcp_ice_aktar_gizlilik(taslak_kimligi, &sunucu, &ortam, gizli)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mcp_ice_aktar_kaydet(
    u: State<'_, Arc<Uygulama>>,
    taslak_kimligi: TaslakKimligi,
    kusak: u64,
    oturum: Option<OturumId>,
) -> Sonuc<bool> {
    u.mcp_ice_aktar_kaydet(taslak_kimligi, kusak, oturum).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mcp_etkin_ayarla(u: State<'_, Arc<Uygulama>>, ad: String, etkin: bool) -> Sonuc<()> {
    u.mcp_etkin_ayarla(ad, etkin)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mcp_sil(u: State<'_, Arc<Uygulama>>, ad: String) -> Sonuc<bool> {
    u.mcp_sil(ad).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mcp_yeniden_baslat(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    ad: String,
) -> Sonuc<()> {
    u.mcp_yeniden_baslat(oturum, ad).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mcp_baslatma_karti_yanitla(
    u: State<'_, Arc<Uygulama>>,
    istek_kimligi: String,
    kabul: bool,
) -> Sonuc<bool> {
    u.mcp_baslatma_karti_yanitla(istek_kimligi, kabul).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mcp_baslatma_izni_geri_al(
    u: State<'_, Arc<Uygulama>>,
    alan_kimligi: String,
    ad: String,
) -> Sonuc<()> {
    u.mcp_baslatma_izni_geri_al(alan_kimligi, ad)
}

// Sağ panelin salt okur görünümü (masaüstü arayüzü tasarımı §6.2, §6.3):
// arayüz yol göndermez, dizin ve dosyalar oturuma bağlı kimlikle anılır.

#[tauri::command(rename_all = "snake_case")]
pub async fn calisma_alani_ozeti(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
) -> Sonuc<CalismaAlaniOzeti> {
    u.calisma_alani_ozeti(oturum).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn calisma_alani_dizini(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    dizin: Option<u64>,
) -> Sonuc<DizinIcerigi> {
    u.calisma_alani_dizini(oturum, dizin).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn calisma_alani_dosyasi(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    kimlik: u64,
) -> Sonuc<DosyaIcerigi> {
    u.calisma_alani_dosyasi(oturum, kimlik).await
}
