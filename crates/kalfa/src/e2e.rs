//! Uçtan uca test derlemesi (1c tasarımı §11.4). Yalnızca `e2e` özelliğiyle
//! derlenir: EVREN yerine senaryolu sahte sağlayıcı, yerel iletişim kutuları
//! yerine ayardaki cevaplar ve ana pencereye açılan test komutları.

use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

use kalfa_core::kimlik::OturumId;
use kalfa_uygulama::Uygulama;
use kalfa_uygulama::anahtar::AnahtarKasasi;
use kalfa_uygulama::arayuz::Iletisim;
use kalfa_uygulama::e2e::{E2eAyarlari, Olcumler, SahteIletisim, SahteUretici, olcumleri_sifirla};
use kalfa_uygulama::evren::SaglayiciUretici;
use kalfa_uygulama::hata::KomutHatasi;
use tauri::ipc::CapabilityBuilder;
use tauri::{AppHandle, Manager, Runtime, State};

use crate::komut_adlari::E2E_KOMUTLARI;
use crate::pencereler::ANA;

/// Test komutlarının yetkisi: yalnızca ana pencere. Capability dosyalarına
/// girmez; test derlemesi açılırken eklenir.
pub fn yetki() -> CapabilityBuilder {
    E2E_KOMUTLARI
        .iter()
        .fold(CapabilityBuilder::new("e2e").window(ANA), |yetki, komut| {
            yetki.permission(format!("allow-{}", komut.replace('_', "-")))
        })
}

pub(crate) struct E2eCalismaAlani(Option<PathBuf>);

/// EVREN'in ve yerel iletişim kutularının yerine geçen sahteler.
pub(crate) type Sahteler = (Arc<dyn SaglayiciUretici>, Arc<dyn Iletisim>);

/// Ayarları `KALFA_E2E`'den okur, yetkiyi ekler, anahtarı kasaya koyar ve
/// sahte sağlayıcıyla sahte iletişim kutularını döner.
pub(crate) fn kur<R: Runtime>(
    app: &AppHandle<R>,
    kasa: &AnahtarKasasi,
) -> Result<Sahteler, Box<dyn Error>> {
    let ayarlar = E2eAyarlari::ortamdan()?;
    if let Some(anahtar) = &ayarlar.anahtar {
        kasa.yaz(anahtar);
    }
    app.add_capability(yetki())?;
    app.manage(E2eCalismaAlani(ayarlar.calisma_alani.clone()));
    Ok((
        Arc::new(SahteUretici::yeni(&ayarlar)),
        Arc::new(SahteIletisim::yeni(&ayarlar)),
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn e2e_olcumler(u: State<'_, Arc<Uygulama>>) -> Result<Olcumler, KomutHatasi> {
    Ok(u.e2e_olcumler())
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn e2e_olcumleri_sifirla() -> Result<(), KomutHatasi> {
    olcumleri_sifirla();
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn e2e_kayit_uret(
    u: State<'_, Arc<Uygulama>>,
    alan: State<'_, E2eCalismaAlani>,
    tur: u32,
) -> Result<OturumId, KomutHatasi> {
    let kok = alan
        .0
        .clone()
        .ok_or_else(|| KomutHatasi::gecersiz("Ayarlarda çalışma alanı yok."))?;
    u.e2e_kayit_uret(kok, tur).await
}

/// Ana pencereyi kapatma düğmesi gibi kapatır (kabul kapısı 3): pencere
/// kapanma isteği ve yok olma olayları Kalfa'nın kapanış yolundan geçer.
/// WebDriver'ın pencere kapatması son pencerede süreci doğrudan sonlandırır;
/// o yolu sınamaz.
#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn e2e_ana_pencereyi_kapat<R: Runtime>(
    app: AppHandle<R>,
) -> Result<(), KomutHatasi> {
    if let Some(pencere) = app.get_webview_window(ANA) {
        pencere
            .close()
            .map_err(|e| KomutHatasi::ic(e.to_string()))?;
    }
    Ok(())
}
