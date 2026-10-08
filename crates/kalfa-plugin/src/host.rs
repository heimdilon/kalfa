//! Eklentinin çağırdığı host fonksiyonları ve bir çağrının `Store` durumu
//! (aşama 4 tasarımı §6.3, §7.3, §7.4, §8.3).

use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant, UNIX_EPOCH};

use globset::GlobBuilder;
use kalfa_core::arac::{AracHatasi, CiktiGonderici};
use kalfa_core::kilit::KaynakKilitleri;
use kalfa_core::protokol::CiktiAkisi;
use kalfa_core::yol::CalismaYolu;
use kalfa_tools::CalismaAlani;
use tokio_util::sync::CancellationToken;
use wasmtime::component::ResourceTable;
use wasmtime_wasi::cli::{IsTerminal, StdoutStream};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use crate::bag::kalfa::plugin::tipler::{DosyaBilgisi, EvrenUcu, Hata, Ilerleme, Seviye};
use crate::depo::Depo;
use crate::manifest::Manifest;
use crate::sinir;

/// Bir çağrının izi: tuzağın nedeni ve `stderr`'in sonu. `Store` atıldıktan
/// sonra da okunur.
#[derive(Default)]
pub(crate) struct Iz {
    pub bellek: AtomicBool,
    pub cpu: AtomicBool,
    pub cpu_tik: AtomicU64,
    pub stderr: SonBaytlar,
}

/// Bellek, tablo ve örnek sınırları (§7.4). Sınırı aşan büyüme tuzak kurar.
pub(crate) struct Sinirlayici {
    iz: Arc<Iz>,
}

impl wasmtime::ResourceLimiter for Sinirlayici {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        if desired > sinir::BELLEK {
            self.iz.bellek.store(true, Ordering::SeqCst);
            return Err(wasmtime::format_err!("bellek sınırı aşıldı"));
        }
        Ok(true)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        if desired > sinir::TABLO_OGESI {
            return Err(wasmtime::format_err!("tablo sınırı aşıldı"));
        }
        Ok(true)
    }

    fn instances(&self) -> usize {
        sinir::ORNEK
    }

    fn tables(&self) -> usize {
        sinir::TABLO
    }

    fn memories(&self) -> usize {
        sinir::BELLEK_SAYISI
    }
}

/// `stderr`'in son `STDERR` baytı (§7.3). Dolunca eskiler atılır; eklenti
/// yazarken tuzağa düşmez.
#[derive(Clone, Default)]
pub(crate) struct SonBaytlar(Arc<Mutex<VecDeque<u8>>>);

impl SonBaytlar {
    fn ekle(&self, bayt: &[u8]) {
        let mut ic = self.0.lock().unwrap_or_else(|z| z.into_inner());
        ic.extend(bayt);
        let fazla = ic.len().saturating_sub(sinir::STDERR);
        ic.drain(..fazla);
    }

    pub fn metin(&self) -> String {
        let ic = self.0.lock().unwrap_or_else(|z| z.into_inner());
        String::from_utf8_lossy(&ic.iter().copied().collect::<Vec<_>>()).into_owned()
    }
}

impl IsTerminal for SonBaytlar {
    fn is_terminal(&self) -> bool {
        false
    }
}

impl StdoutStream for SonBaytlar {
    fn async_stream(&self) -> Box<dyn tokio::io::AsyncWrite + Send + Sync> {
        Box::new(self.clone())
    }
}

impl tokio::io::AsyncWrite for SonBaytlar {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.ekle(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// Bir araç çağrısının host tarafı: çalışma alanı, depo, iptal ve canlı çıktı.
pub(crate) struct CagriOrtami {
    pub manifest: Arc<Manifest>,
    pub alan: Arc<CalismaAlani>,
    pub kilitler: Arc<KaynakKilitleri>,
    pub depo: Depo,
    pub iptal: CancellationToken,
    pub cikti: CiktiGonderici,
    /// Git'in korumalı yolları; ilk dosya çağrısında bir kez çözülür.
    korumali: Option<Vec<String>>,
    okunan_bayt: u64,
    gunluk_satiri: u32,
    gunluk_atilan: u32,
    son_ilerleme: Option<Instant>,
}

impl CagriOrtami {
    pub fn yeni(
        manifest: Arc<Manifest>,
        alan: Arc<CalismaAlani>,
        kilitler: Arc<KaynakKilitleri>,
        depo: Depo,
        iptal: CancellationToken,
        cikti: CiktiGonderici,
    ) -> Self {
        Self {
            manifest,
            alan,
            kilitler,
            depo,
            iptal,
            cikti,
            korumali: None,
            okunan_bayt: 0,
            gunluk_satiri: 0,
            gunluk_atilan: 0,
            son_ilerleme: None,
        }
    }

    /// Çağrı bitti: atılan günlük satırları bildirilir.
    pub fn bitir(&self) {
        if self.gunluk_atilan > 0 {
            self.cikti.gonder(
                CiktiAkisi::Standart,
                format!(
                    "[günlük] {} satır sınırı aştığı için atıldı\n",
                    self.gunluk_atilan
                ),
            );
        }
    }

    async fn korumali(&mut self) -> Result<&[String], Hata> {
        if self.korumali.is_none() {
            let kok = self.alan.kok().to_path_buf();
            let k =
                tokio::task::spawn_blocking(move || kalfa_core::git_koruma::korumali_yollar(&kok))
                    .await
                    .map_err(|e| Hata::Diger(format!("git korumaları çözülemedi: {e}")))?;
            self.korumali = Some(k);
        }
        Ok(self.korumali.as_deref().unwrap_or_default())
    }

    fn dosya_yetenegi(&self) -> Result<(), Hata> {
        if self.manifest.dosya_oku.is_empty() {
            return Err(Hata::IzinYok("dosya-oku yeteneği manifestte yok".into()));
        }
        if self.iptal.is_cancelled() {
            return Err(Hata::IptalEdildi);
        }
        Ok(())
    }

    /// Eklentiye açık mı: manifest globu, gizli değil, `.git` ya da korumalı
    /// değil.
    async fn acik_mi(&mut self, yol: &CalismaYolu) -> Result<Result<(), Hata>, Hata> {
        if !self.manifest.okuyabilir(yol.as_str()) {
            return Ok(Err(Hata::IzinYok(format!(
                "`{yol}` manifestteki globlara uymuyor"
            ))));
        }
        if self.alan.siniflandirici().gizli_mi(yol) {
            return Ok(Err(Hata::IzinYok("gizli dosya".into())));
        }
        if kalfa_core::izin::korumali_listede(yol, self.korumali().await?) {
            return Ok(Err(Hata::IzinYok("korumalı yol".into())));
        }
        Ok(Ok(()))
    }

    async fn dosya_listele(&mut self, desen: String) -> Result<Vec<DosyaBilgisi>, Hata> {
        self.dosya_yetenegi()?;
        let eslestirici = GlobBuilder::new(&desen)
            .literal_separator(true)
            .build()
            .map_err(|e| Hata::Diger(format!("desen geçersiz: {e}")))?
            .compile_matcher();
        let alan = self.alan.clone();
        let iptal = self.iptal.clone();
        let dosyalar = tokio::task::spawn_blocking(move || alan.dosyalar(&iptal))
            .await
            .map_err(|e| Hata::Diger(e.to_string()))?
            .map_err(|e| match e {
                AracHatasi::Iptal => Hata::IptalEdildi,
                e => Hata::Diger(e.to_string()),
            })?;
        let mut acik = Vec::new();
        for yol in dosyalar {
            if eslestirici.is_match(yol.as_str()) && self.acik_mi(&yol).await?.is_ok() {
                if acik.len() == sinir::LISTE {
                    return Err(Hata::SinirAsildi(format!(
                        "dosya listesi {} kaydı aşıyor; deseni daraltın",
                        sinir::LISTE
                    )));
                }
                acik.push(yol);
            }
        }
        let alan = self.alan.clone();
        tokio::task::spawn_blocking(move || {
            acik.into_iter()
                // Tarama ile bilgi arasında silinen ya da değişen dosya atlanır.
                .filter_map(|yol| {
                    let (boyut, zaman) = alan.dosya_bilgisi(&yol).ok()?;
                    let degisme = zaman
                        .duration_since(UNIX_EPOCH)
                        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
                    Some(DosyaBilgisi {
                        yol: yol.as_str().to_string(),
                        boyut,
                        degisme,
                    })
                })
                .collect()
        })
        .await
        .map_err(|e| Hata::Diger(e.to_string()))
    }

    async fn dosya_oku(&mut self, yol: String) -> Result<String, Hata> {
        self.dosya_yetenegi()?;
        let yol = CalismaYolu::coz(&yol)
            .map_err(|e| Hata::IzinYok(format!("`{yol}` geçersiz yol: {e}")))?;
        self.acik_mi(&yol).await??;
        // Okuma süresince o yolun okuma kilidi ve genel okuma kilidi (§8.3).
        let _kilit = tokio::select! {
            k = self.kilitler.al(std::slice::from_ref(&yol), &[], false) => k,
            _ = self.iptal.cancelled() => return Err(Hata::IptalEdildi),
        };
        let alan = self.alan.clone();
        let iptal = self.iptal.clone();
        let okuma_yolu = yol.clone();
        let veri = tokio::task::spawn_blocking(move || {
            let (boyut, _) = alan.dosya_bilgisi(&okuma_yolu).map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Hata::Bulunamadi(okuma_yolu.to_string())
                } else {
                    Hata::IzinYok(format!("`{okuma_yolu}` okunamadı: {e}"))
                }
            })?;
            if boyut > sinir::DOSYA {
                return Err(Hata::SinirAsildi(format!(
                    "`{okuma_yolu}` 4 MiB okuma sınırını aşıyor"
                )));
            }
            alan.sinirli_oku(&okuma_yolu, &iptal).map_err(|e| match e {
                AracHatasi::Iptal => Hata::IptalEdildi,
                // Hard link'li dosya: içeriği başka bir yolla da paylaşılıyor.
                AracHatasi::OnayEskidi => Hata::IzinYok(format!(
                    "`{okuma_yolu}` başka bir yerde de adı olan (hard link) bir dosya"
                )),
                e => Hata::IzinYok(format!("`{okuma_yolu}` okunamadı: {e}")),
            })
        })
        .await
        .map_err(|e| Hata::Diger(e.to_string()))??;
        if veri.len() as u64 > sinir::DOSYA {
            return Err(Hata::SinirAsildi(format!(
                "`{yol}` 4 MiB okuma sınırını aşıyor"
            )));
        }
        self.okunan_bayt += veri.len() as u64;
        if self.okunan_bayt > sinir::CAGRI_OKUMASI {
            return Err(Hata::SinirAsildi(
                "bu çağrıda okunan dosyalar 1 GiB sınırını aştı".into(),
            ));
        }
        String::from_utf8(veri).map_err(|_| Hata::Diger(format!("`{yol}` metin değil")))
    }

    fn depo_yetenegi(&self) -> Result<(), Hata> {
        if self.manifest.depo_mb == 0 {
            return Err(Hata::IzinYok("depo yeteneği manifestte yok".into()));
        }
        if self.iptal.is_cancelled() {
            return Err(Hata::IptalEdildi);
        }
        Ok(())
    }

    fn evren_cagir(&mut self, uc: EvrenUcu) -> Result<String, Hata> {
        let uc = match uc {
            EvrenUcu::Embeddings => crate::manifest::EvrenUcu::Embeddings,
            EvrenUcu::Rerank => crate::manifest::EvrenUcu::Rerank,
        };
        if !self.manifest.evren.contains(&uc) {
            return Err(Hata::IzinYok(format!(
                "evren yeteneği manifestte yok: `{}`",
                uc.ad()
            )));
        }
        if self.iptal.is_cancelled() {
            return Err(Hata::IptalEdildi);
        }
        // ponytail: EVREN yolu aşama 4b'de bağlanır (tasarım §20).
        Err(Hata::IzinYok(
            "EVREN çağrısı bu Kalfa sürümünde henüz yok (aşama 4b)".into(),
        ))
    }

    fn gunluk(&mut self, duzey: Seviye, mesaj: String) {
        if self.gunluk_satiri >= sinir::GUNLUK_SATIRI {
            self.gunluk_atilan += 1;
            return;
        }
        self.gunluk_satiri += 1;
        let duzey = match duzey {
            Seviye::Hata => "hata",
            Seviye::Uyari => "uyarı",
            Seviye::Bilgi => "bilgi",
            Seviye::Ayrinti => "ayrıntı",
        };
        let mesaj = kalfa_core::arac::kirp(&mesaj, sinir::GUNLUK_BAYTI);
        self.cikti
            .gonder(CiktiAkisi::Standart, format!("[{duzey}] {mesaj}\n"));
    }

    /// Saniyede en çok 10 olay; arada gelenler atılır, sonraki olay güncel
    /// durumu taşır.
    fn ilerleme(&mut self, d: Ilerleme) {
        let simdi = Instant::now();
        if self
            .son_ilerleme
            .is_some_and(|s| simdi.duration_since(s) < Duration::from_millis(100))
        {
            return;
        }
        self.son_ilerleme = Some(simdi);
        let sayi = match d.toplam {
            Some(t) => format!("{}/{t}", d.tamamlanan),
            None => d.tamamlanan.to_string(),
        };
        let metin = match d.mesaj {
            Some(m) => format!("[ilerleme] {sayi} · {}", kalfa_core::arac::kirp(&m, 512)),
            None => format!("[ilerleme] {sayi}"),
        };
        self.cikti
            .gonder(CiktiAkisi::Standart, format!("{metin}\n"));
    }
}

/// Bir `Store`'un durumu. `cagri` yükleme sırasında (`araclari-listele`)
/// yoktur; o sırada host çağrıları reddedilir.
pub(crate) struct Durum {
    wasi: WasiCtx,
    tablo: ResourceTable,
    pub sinirlayici: Sinirlayici,
    pub iz: Arc<Iz>,
    pub cagri: Option<CagriOrtami>,
}

impl Durum {
    pub fn yeni(cagri: Option<CagriOrtami>) -> Self {
        let iz = Arc::new(Iz::default());
        // Ortam değişkeni, argüman ve ön açık dizin yok; ağ kapalı. `stdout`
        // atılır, `stderr`'in sonu tutulur (§7.3).
        let wasi = WasiCtx::builder()
            .stderr(iz.stderr.clone())
            .allow_tcp(false)
            .allow_udp(false)
            .allow_ip_name_lookup(false)
            .build();
        Self {
            wasi,
            tablo: ResourceTable::new(),
            sinirlayici: Sinirlayici { iz: iz.clone() },
            iz,
            cagri,
        }
    }

    fn cagri(&mut self) -> Result<&mut CagriOrtami, Hata> {
        self.cagri
            .as_mut()
            .ok_or_else(|| Hata::IzinYok("araç listesi alınırken host çağrısı yapılamaz".into()))
    }
}

impl WasiView for Durum {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.tablo,
        }
    }
}

impl crate::bag::kalfa::plugin::tipler::Host for Durum {}

impl crate::bag::kalfa::plugin::host::Host for Durum {
    async fn dosya_listele(&mut self, desen: String) -> Result<Vec<DosyaBilgisi>, Hata> {
        self.cagri()?.dosya_listele(desen).await
    }

    async fn dosya_oku(&mut self, yol: String) -> Result<String, Hata> {
        self.cagri()?.dosya_oku(yol).await
    }

    async fn evren_cagir(&mut self, uc: EvrenUcu, _govde: String) -> Result<String, Hata> {
        self.cagri()?.evren_cagir(uc)
    }

    async fn depo_oku(&mut self, anahtar: String) -> Result<Option<Vec<u8>>, Hata> {
        let c = self.cagri()?;
        c.depo_yetenegi()?;
        c.depo.oku(&anahtar).await
    }

    async fn depo_yaz(&mut self, anahtar: String, deger: Vec<u8>) -> Result<(), Hata> {
        let c = self.cagri()?;
        c.depo_yetenegi()?;
        c.depo.yaz(&anahtar, deger).await
    }

    async fn depo_sil(&mut self, anahtar: String) -> Result<(), Hata> {
        let c = self.cagri()?;
        c.depo_yetenegi()?;
        c.depo.sil(&anahtar).await
    }

    async fn ilerleme_bildir(&mut self, durum: Ilerleme) {
        if let Some(c) = self.cagri.as_mut() {
            c.ilerleme(durum);
        }
    }

    async fn iptal_istendi(&mut self) -> bool {
        self.cagri.as_ref().is_some_and(|c| c.iptal.is_cancelled())
    }

    async fn gunluk(&mut self, duzey: Seviye, mesaj: String) {
        if let Some(c) = self.cagri.as_mut() {
            c.gunluk(duzey, mesaj);
        }
    }
}
