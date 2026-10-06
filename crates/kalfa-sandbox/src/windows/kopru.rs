//! Windows adlandırılmış boru sunucusu (aşama 3d alt planı W1).
//!
//! Yalıtılmış kapta çalışan `kalfa-kopru.exe`, Kalfa'nın açtığı bu boruya
//! bağlanarak ağ trafiğini vekile aktarır.

use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::OwnedSemaphorePermit;
use tokio_util::sync::CancellationToken;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, LocalFree};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows_sys::Win32::System::JobObjects::IsProcessInJob;
use windows_sys::Win32::System::Pipes::GetNamedPipeClientProcessId;
use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

use crate::agac::Is;
use crate::vekil::Vekil;
use crate::windows::guvenlik::{self, Sid};

/// Adlandırılmış boru için SDDL dizgisi üretir.
///
/// DACL: `D:P(A;;GA;;;SY)(A;;GA;;;<geliştirici>)(A;;GRGW;;;<paket SID>)`
pub fn boru_sddl(paket: &Sid, gelistirici: &Sid) -> String {
    format!(
        "D:P(A;;GA;;;SY)(A;;GA;;;{})(A;;GRGW;;;{})",
        gelistirici.metin(),
        paket.metin()
    )
}

/// 128 bit rastgele boru adı üretir (`\\.\pipe\kalfa-ag-<onaltılık>`).
pub fn rastgele_boru_adi() -> io::Result<String> {
    let onaltilik = guvenlik::rastgele_onaltilik_128()?;
    Ok(format!(r"\\.\pipe\kalfa-ag-{onaltilik}"))
}

struct GuvenlikTanimi(PSECURITY_DESCRIPTOR);
unsafe impl Send for GuvenlikTanimi {}
unsafe impl Sync for GuvenlikTanimi {}

impl Drop for GuvenlikTanimi {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: tanım LocalFree ile bırakılmalıdır.
            unsafe { LocalFree(self.0 as _) };
        }
    }
}

fn sddl_coz(sddl: &str) -> io::Result<GuvenlikTanimi> {
    let sddl_genis: Vec<u16> = OsStr::new(sddl).encode_wide().chain(Some(0)).collect();
    let mut tanim: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    // SAFETY: sddl_genis NUL ile biten UTF-16 dizidir; tanım ConvertStringSecurityDescriptorToSecurityDescriptorW
    // tarafından ayrılır ve GuvenlikTanimi ile LocalFree yapılır.
    let tamam = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl_genis.as_ptr(),
            SDDL_REVISION_1,
            &mut tanim,
            std::ptr::null_mut(),
        )
    };
    if tamam == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(GuvenlikTanimi(tanim))
}

fn pipe_ornek_olustur(
    ad: &str,
    tanim: PSECURITY_DESCRIPTOR,
    ilk: bool,
    azami: usize,
) -> io::Result<NamedPipeServer> {
    let mut sa = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: tanim,
        bInheritHandle: 0,
    };
    // SAFETY: sa ve tanım çağrı boyunca geçerlidir.
    unsafe {
        ServerOptions::new()
            .first_pipe_instance(ilk)
            .reject_remote_clients(true)
            .max_instances(azami)
            .create_with_security_attributes_raw(ad, &mut sa as *mut _ as *mut std::ffi::c_void)
    }
}

fn istemci_iste_mi(boru: &NamedPipeServer, is_tutamac: HANDLE) -> Result<bool, String> {
    use std::os::windows::io::AsRawHandle;
    let tutamac = boru.as_raw_handle();
    let mut pid = 0u32;
    // SAFETY: boru geçerli bir adlandırılmış boru sunucusudur; pid işaretçisi geçerlidir.
    let basarili = unsafe { GetNamedPipeClientProcessId(tutamac as _, &mut pid) };
    if basarili == 0 {
        return Err(format!(
            "istemci süreç kimliği okunamadı: {}",
            io::Error::last_os_error()
        ));
    }

    // SAFETY: PROCESS_QUERY_LIMITED_INFORMATION ile süreci açarız.
    let surec_tutamac = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if surec_tutamac.is_null() {
        return Err(format!(
            "istemci süreci açılamadı: {}",
            io::Error::last_os_error()
        ));
    }

    let mut isin_icinde = 0i32;
    // SAFETY: surec_tutamac ve is_tutamac geçerli tanıtıcılardır; isin_icinde işaretçisi geçerlidir.
    let sonuc = unsafe { IsProcessInJob(surec_tutamac, is_tutamac, &mut isin_icinde) };
    let hata = if sonuc == 0 {
        Some(io::Error::last_os_error())
    } else {
        None
    };

    // SAFETY: surec_tutamac kapatılır.
    unsafe { CloseHandle(surec_tutamac) };

    if let Some(e) = hata {
        return Err(format!("IsProcessInJob başarısız: {e}"));
    }

    Ok(isin_icinde != 0)
}

struct IzinliAkis<S> {
    ic: S,
    _izin: OwnedSemaphorePermit,
}

impl<S: AsyncRead + Unpin> AsyncRead for IzinliAkis<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.ic).poll_read(cx, buf)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for IzinliAkis<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.ic).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.ic).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.ic).poll_shutdown(cx)
    }
}

/// Kalfa vekiline giden adlandırılmış boru sunucusu.
pub struct BoruSunucusu {
    ad: String,
    iptal: CancellationToken,
    gorev: tokio::task::JoinHandle<()>,
}

impl BoruSunucusu {
    /// Rastgele adla boru sunucusunu açar ve kabul döngüsünü başlatır.
    pub fn ac(
        paket: &Sid,
        gelistirici: &Sid,
        is: &Is,
        vekil: Arc<Vekil>,
        azami: usize,
    ) -> io::Result<Self> {
        let ad = rastgele_boru_adi()?;
        let sddl = boru_sddl(paket, gelistirici);
        let tanim_arc = Arc::new(sddl_coz(&sddl)?);

        let semafor = Arc::new(tokio::sync::Semaphore::new(azami));
        let ilk_izin = semafor
            .clone()
            .try_acquire_owned()
            .map_err(|_| io::Error::other("azami boru örneği 0 olamaz"))?;

        let ilk_sunucu = pipe_ornek_olustur(&ad, tanim_arc.0, true, azami)?;

        let ad_dongu = ad.clone();
        let iptal = CancellationToken::new();
        let iptal_dongu = iptal.clone();
        let is_tutamac = is.tutamac() as usize;

        let gorev = tokio::spawn(async move {
            let mut sunucu = ilk_sunucu;
            let mut mevcut_izin = ilk_izin;

            loop {
                let baglanti_sonucu = tokio::select! {
                    _ = iptal_dongu.cancelled() => break,
                    res = sunucu.connect() => res,
                };

                if baglanti_sonucu.is_err() {
                    continue;
                }

                let bagli = sunucu;
                let su_anki_izin = mevcut_izin;

                // Sonraki dinleyici örneği için izin al
                let sonraki_izin = tokio::select! {
                    _ = iptal_dongu.cancelled() => break,
                    res = semafor.clone().acquire_owned() => {
                        match res {
                            Ok(p) => p,
                            Err(_) => break,
                        }
                    }
                };

                // Sonraki dinleyici örneğini oluştur
                match pipe_ornek_olustur(&ad_dongu, tanim_arc.0, false, azami) {
                    Ok(yeni) => {
                        sunucu = yeni;
                        mevcut_izin = sonraki_izin;
                    }
                    Err(_) => {
                        break;
                    }
                }

                // İstemciyi doğrula ve vekile ver
                let is_tutamac_val = is_tutamac;
                let vekil_klon = Arc::clone(&vekil);

                tokio::spawn(async move {
                    let is_tut = is_tutamac_val as HANDLE;
                    match istemci_iste_mi(&bagli, is_tut) {
                        Ok(true) => {
                            let akis = IzinliAkis {
                                ic: bagli,
                                _izin: su_anki_izin,
                            };
                            vekil_klon.baglanti_ver(akis);
                        }
                        Ok(false) => {
                            vekil_klon
                                .gecersiz_baglanti_kaydet("istemci komutun iş nesnesinde değil");
                            drop(bagli);
                            drop(su_anki_izin);
                        }
                        Err(e) => {
                            vekil_klon.gecersiz_baglanti_kaydet(format!(
                                "iş nesnesi denetimi başarısız: {e}"
                            ));
                            drop(bagli);
                            drop(su_anki_izin);
                        }
                    }
                });
            }
        });

        Ok(Self { ad, iptal, gorev })
    }

    /// Borunun tam adını döndürür (`\\.\pipe\kalfa-ag-...`).
    pub fn ad(&self) -> &str {
        &self.ad
    }

    /// Kabul döngüsünü kapatır.
    pub fn kapat(self) {
        self.iptal.cancel();
        self.gorev.abort();
    }
}

impl Drop for BoruSunucusu {
    fn drop(&mut self) {
        self.iptal.cancel();
        self.gorev.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vekil::VekilAyari;
    use crate::windows::guvenlik::{self, Sid};
    use tokio::io::AsyncReadExt;

    #[test]
    fn sddl_yalnizca_sistem_gelistirici_ve_paket() {
        let paket = Sid::metinden("S-1-15-2-1-2-3-4").unwrap();
        let gelistirici = guvenlik::kullanici_sidi().unwrap();
        let sddl = boru_sddl(&paket, &gelistirici);

        assert!(sddl.starts_with("D:P"));
        assert!(sddl.contains("(A;;GA;;;SY)"));
        assert!(sddl.contains(&format!("(A;;GA;;;{})", gelistirici.metin())));
        assert!(sddl.contains(&format!("(A;;GRGW;;;{})", paket.metin())));

        // SDDL'in Win32 tarafından çözülebildiğini doğrula
        let tanim = sddl_coz(&sddl);
        assert!(tanim.is_ok());
    }

    #[test]
    fn ad_her_cagrida_farkli_ve_128_bit() {
        let ad1 = rastgele_boru_adi().unwrap();
        let ad2 = rastgele_boru_adi().unwrap();

        assert!(ad1.starts_with(r"\\.\pipe\kalfa-ag-"));
        assert!(ad2.starts_with(r"\\.\pipe\kalfa-ag-"));
        assert_ne!(ad1, ad2);

        let sonek1 = ad1.strip_prefix(r"\\.\pipe\kalfa-ag-").unwrap();
        assert_eq!(sonek1.len(), 32);
        assert!(sonek1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn ayni_adla_ikinci_sunucu_acilamaz_birim() {
        let paket = Sid::metinden("S-1-15-2-1-2-3-4").unwrap();
        let gelistirici = guvenlik::kullanici_sidi().unwrap();
        let is = Is::yeni(false).unwrap();
        let vekil = Arc::new(Vekil::baslat(VekilAyari::default()));
        let sunucu = BoruSunucusu::ac(&paket, &gelistirici, &is, vekil, 64).unwrap();

        let ikinci = tokio::net::windows::named_pipe::ServerOptions::new()
            .first_pipe_instance(true)
            .create(sunucu.ad());
        assert!(
            ikinci.is_err(),
            "aynı adla ilk örnek olarak ikinci sunucu açılamamalı"
        );
    }

    #[tokio::test]
    async fn isin_disindaki_istemci_reddedilir_birim() {
        let paket = Sid::metinden("S-1-15-2-1-2-3-4").unwrap();
        let gelistirici = guvenlik::kullanici_sidi().unwrap();
        let is = Is::yeni(false).unwrap();
        let vekil = Arc::new(Vekil::baslat(VekilAyari::default()));
        let sunucu = BoruSunucusu::ac(&paket, &gelistirici, &is, vekil.clone(), 64).unwrap();

        let mut istemci = tokio::net::windows::named_pipe::ClientOptions::new()
            .open(sunucu.ad())
            .unwrap();

        let mut tampon = [0u8; 16];
        let okunan = istemci.read(&mut tampon).await;
        assert!(matches!(okunan, Ok(0) | Err(_)));

        let ozet = vekil.bitir().await;
        assert!(
            ozet.baglantilar
                .iter()
                .any(|b| matches!(b.karar, crate::vekil::AgKarari::Gecersiz(_))),
            "iş dışındaki istemci için karar Gecersiz olmalı"
        );
    }

    async fn boru_baglan(
        ad: &str,
        bekle: bool,
    ) -> io::Result<tokio::net::windows::named_pipe::NamedPipeClient> {
        let baslangic = std::time::Instant::now();
        loop {
            match tokio::net::windows::named_pipe::ClientOptions::new().open(ad) {
                Ok(c) => return Ok(c),
                Err(e) if e.raw_os_error() == Some(231) && bekle => {
                    if baslangic.elapsed() > std::time::Duration::from_millis(500) {
                        return Err(e);
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    #[tokio::test]
    async fn azami_ornek_asilinca_yeni_baglanti_reddedilir_birim() {
        let paket = Sid::metinden("S-1-15-2-1-2-3-4").unwrap();
        let gelistirici = guvenlik::kullanici_sidi().unwrap();
        let is = Is::yeni_ayarsiz().unwrap();
        // Test sürecini iş nesnesine ata; böylece bağlantılar kabul edilir ve açık kalır.
        let atandi = unsafe {
            windows_sys::Win32::System::JobObjects::AssignProcessToJobObject(
                is.tutamac(),
                windows_sys::Win32::System::Threading::GetCurrentProcess(),
            )
        };
        assert_ne!(atandi, 0, "test süreci iş nesnesine atanabilmeli");

        let vekil = Arc::new(Vekil::baslat(VekilAyari::default()));
        let sunucu = BoruSunucusu::ac(&paket, &gelistirici, &is, vekil, 2).unwrap();

        let baglanti1 = boru_baglan(sunucu.ad(), true).await.unwrap();
        let baglanti2 = boru_baglan(sunucu.ad(), true).await.unwrap();

        let baglanti3 = boru_baglan(sunucu.ad(), true).await;
        assert!(
            baglanti3.is_err(),
            "azami sınır aşıldığında yeni bağlantı reddedilmeli"
        );

        drop(baglanti1);
        drop(baglanti2);
    }
}
