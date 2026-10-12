//! Eklentinin çağırdığı host fonksiyonları ve bir çağrının `Store` durumu
//! (aşama 4 tasarımı §6.3, §7.3, §7.4, §8.3, §9.2).

use std::collections::{HashSet, VecDeque};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use globset::{GlobBuilder, GlobMatcher};
use kalfa_core::arac::{AracHatasi, AracOrtami, CiktiGonderici, GonderimTalebi, GonderimYaniti};
use kalfa_core::butce::hata_kullanimi;
use kalfa_core::gonderim::{EklentiIstekOzeti, IstekAsamasi, IstekSonucu, OKUNAN_SINIRI};
use kalfa_core::kilit::KaynakKilitleri;
use kalfa_core::protokol::CiktiAkisi;
use kalfa_core::yol::CalismaYolu;
use kalfa_evren::{EvrenError, HamYanit, YanitMeta};
use kalfa_tools::CalismaAlani;
use tokio_util::sync::CancellationToken;
use wasmtime::component::ResourceTable;
use wasmtime_wasi::cli::{IsTerminal, StdoutStream};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use crate::bag::kalfa::plugin::tipler::{DosyaBilgisi, EvrenUcu, Hata, Ilerleme, Seviye};
use crate::depo::Depo;
use crate::evren::{
    BellekIzni, EvrenBaglami, GonderimIzni, ek_uc, eklenti_hatasi, govde_dogrula, ham_kullanim,
    kayit_hatasi,
};
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

/// Eklentinin çalışma alanında açabildiği dosyalar (§6.3, §8.3): `desen`
/// (yoksa hepsi) ile eşleşen, manifest globuna uyan, gizli olmayan ve korumalı
/// olmayan dosyalar; yola göre sıralı. Boyut ve değişme zamanıyla. `dosya-listele`
/// ile ilk gönderim kartı aynı listeyi kullanır (§8.1).
pub(crate) fn acik_dosyalar(
    manifest: &Manifest,
    alan: &CalismaAlani,
    korumali: &[String],
    desen: Option<&GlobMatcher>,
    iptal: &CancellationToken,
) -> Result<Vec<(CalismaYolu, u64, SystemTime)>, AracHatasi> {
    let mut acik = Vec::new();
    for yol in alan.dosyalar(iptal)? {
        let desene_uyar = desen.is_none_or(|d| d.is_match(yol.as_str()));
        if !desene_uyar
            || !manifest.okuyabilir(yol.as_str())
            || alan.siniflandirici().gizli_mi(&yol)
            || kalfa_core::izin::korumali_listede(&yol, korumali)
        {
            continue;
        }
        // Tarama ile bilgi arasında silinen ya da değişen dosya atlanır.
        if let Ok((boyut, zaman)) = alan.dosya_bilgisi(&yol) {
            acik.push((yol, boyut, zaman));
        }
    }
    Ok(acik)
}

fn hata_cevir(hata: AracHatasi) -> Hata {
    match hata {
        AracHatasi::Iptal => Hata::IptalEdildi,
        diger => Hata::Diger(diger.to_string()),
    }
}

/// Bir EVREN isteğinin ağdaki sonucu (§9.2, §9.5).
enum Cevap {
    Yanit(HamYanit),
    Hata(EvrenError),
    ZamanAsimi,
    Iptal,
}

/// Bir araç çağrısının host tarafı: çalışma alanı, depo, iptal, canlı çıktı ve
/// EVREN (§7.4, §8.3, §9.2).
pub(crate) struct CagriOrtami {
    pub manifest: Arc<Manifest>,
    pub alan: Arc<CalismaAlani>,
    pub kilitler: Arc<KaynakKilitleri>,
    pub depo: Depo,
    pub iptal: CancellationToken,
    /// Canlı çıktı ve oturumla konuşma kanalı (§8.7).
    ortam: AracOrtami,
    evren: Option<Arc<EvrenBaglami>>,
    izin: Arc<dyn GonderimIzni>,
    /// Bu çağrı ilk gönderim onayıyla çalışıyor (§8.4).
    ilk_onayli: bool,
    /// Git'in korumalı yolları; ilk dosya çağrısında bir kez çözülür.
    korumali: Option<Vec<String>>,
    okunan_bayt: u64,
    /// Bu çağrıda okunan dosyalar, okunma sırasıyla, tekrarsız (§9.3).
    okunan: Vec<String>,
    okunan_kume: HashSet<String>,
    evren_istegi: u32,
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
        ortam: AracOrtami,
    ) -> Self {
        Self {
            manifest,
            alan,
            kilitler,
            depo,
            iptal,
            ortam,
            evren: None,
            izin: BellekIzni::yeni(false),
            ilk_onayli: false,
            korumali: None,
            okunan_bayt: 0,
            okunan: Vec::new(),
            okunan_kume: HashSet::new(),
            evren_istegi: 0,
            gunluk_satiri: 0,
            gunluk_atilan: 0,
            son_ilerleme: None,
        }
    }

    /// EVREN bağlantısı, gönderim izni ve bu çağrının ilk gönderim onayı.
    pub fn evrenli(
        mut self,
        evren: Option<Arc<EvrenBaglami>>,
        izin: Arc<dyn GonderimIzni>,
        ilk_onayli: bool,
    ) -> Self {
        self.evren = evren;
        self.izin = izin;
        self.ilk_onayli = ilk_onayli;
        self
    }

    fn cikti(&self) -> CiktiGonderici {
        self.ortam.cikti()
    }

    /// Çağrı bitti: atılan günlük satırları bildirilir.
    pub fn bitir(&self) {
        if self.gunluk_atilan > 0 {
            self.cikti().gonder(
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
        let korumali = self.korumali().await?.to_vec();
        let manifest = self.manifest.clone();
        let alan = self.alan.clone();
        let iptal = self.iptal.clone();
        let liste = tokio::task::spawn_blocking(move || {
            acik_dosyalar(&manifest, &alan, &korumali, Some(&eslestirici), &iptal)
        })
        .await
        .map_err(|e| Hata::Diger(e.to_string()))?
        .map_err(hata_cevir)?;
        if liste.len() > sinir::LISTE {
            return Err(Hata::SinirAsildi(format!(
                "dosya listesi {} kaydı aşıyor; deseni daraltın",
                sinir::LISTE
            )));
        }
        Ok(liste
            .into_iter()
            .map(|(yol, boyut, zaman)| {
                let degisme = zaman
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
                DosyaBilgisi {
                    yol: yol.as_str().to_string(),
                    boyut,
                    degisme,
                }
            })
            .collect())
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
        let metin =
            String::from_utf8(veri).map_err(|_| Hata::Diger(format!("`{yol}` metin değil")))?;
        self.okundu(yol.as_str());
        Ok(metin)
    }

    fn okundu(&mut self, yol: &str) {
        if self.okunan_kume.insert(yol.to_string()) {
            self.okunan.push(yol.to_string());
        }
    }

    /// Bu çağrıda okunan dosyalar (en çok `OKUNAN_SINIRI`) ve fazlasının sayısı
    /// (§9.3).
    fn okunan_ozeti(&self) -> (Vec<String>, u32) {
        let dosyalar = self.okunan.iter().take(OKUNAN_SINIRI).cloned().collect();
        let fazla =
            u32::try_from(self.okunan.len().saturating_sub(OKUNAN_SINIRI)).unwrap_or(u32::MAX);
        (dosyalar, fazla)
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

    /// `evren-cagir` (§9.2): yetenek ve bağlantı denetimi, gövde doğrulama, izin,
    /// sınırlar, model ve şart, gönderim izni (önizleme), bütçe izni (5b), istek,
    /// kayıt, yanıt boyutu. Hiçbir hata yolunda istek iki kez gönderilmez.
    async fn evren_cagir(&mut self, uc: EvrenUcu, govde: String) -> Result<String, Hata> {
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
        let Some(evren) = self.evren.clone() else {
            return Err(Hata::Evren("EVREN bağlantısı bu oturumda yok".into()));
        };
        let uc = ek_uc(uc);
        let bayt = govde.len() as u64;
        let dogrulanmis = govde_dogrula(uc, &govde)?;
        if !self.ilk_onayli && !self.izin.var_mi() {
            return Err(Hata::IzinYok(
                "bu çalışma alanında EVREN'e gönderim izni yok".into(),
            ));
        }
        if self.evren_istegi >= sinir::CAGRI_EVREN_ISTEGI {
            return Err(Hata::SinirAsildi(format!(
                "bu çağrıda EVREN istek sınırı ({}) doldu",
                sinir::CAGRI_EVREN_ISTEGI
            )));
        }
        self.evren_istegi += 1;
        // Şart beklenirken model kataloğu için de ağa çıkılmaz.
        evren.sart_denetle()?;
        evren.model_denetle(&dogrulanmis.model, uc).await?;
        let (dosyalar, dosya_fazlasi) = self.okunan_ozeti();
        let ozet = EklentiIstekOzeti {
            eklenti: self.manifest.ad.clone(),
            uc,
            model: dogrulanmis.model.clone(),
            metin_sayisi: dogrulanmis.metin_sayisi,
            bayt,
            ozet: dogrulanmis.ozet.clone(),
            dosyalar,
            dosya_fazlasi,
        };
        let talep = GonderimTalebi {
            ozet: ozet.clone(),
            govde: Arc::from(govde),
        };
        let karar = tokio::select! {
            karar = self.ortam.gonderim_izni(talep) => karar,
            _ = self.iptal.cancelled() => return Err(Hata::IptalEdildi),
        };
        match karar {
            GonderimYaniti::Gonder => {}
            GonderimYaniti::Gonderme => {
                return Err(Hata::IzinYok("kullanıcı göndermedi".into()));
            }
            GonderimYaniti::SinirAsildi(ileti) => return Err(Hata::SinirAsildi(ileti)),
            GonderimYaniti::Iptal => return Err(Hata::IptalEdildi),
        }
        // Bütçe izni ağdan hemen önce (5b tasarımı §7). Reddedilen ya da iptal
        // edilen çağrı ağa çıkmaz ve eklentiye bütçe hatası olarak gitmez: RAG'ın
        // toplu küçültmesi bu yüzden tetiklenmez. Çağrı aynı future'da bekler.
        let amac = format!("eklenti:{}", self.manifest.ad);
        let butce_islem = tokio::select! {
            izin = self.ortam.butce_izni(&amac) => izin,
            _ = self.iptal.cancelled() => return Err(Hata::IptalEdildi),
        };
        let butce_islem = butce_islem.map_err(|()| Hata::IptalEdildi)?;
        let cevap = tokio::select! {
            sonuc = tokio::time::timeout(
                sinir::EVREN_SURESI,
                evren.ek_istegi(uc, &dogrulanmis.deger),
            ) => match sonuc {
                Ok(Ok(yanit)) => Cevap::Yanit(yanit),
                Ok(Err(hata)) => Cevap::Hata(hata),
                Err(_) => Cevap::ZamanAsimi,
            },
            _ = self.iptal.cancelled() => Cevap::Iptal,
        };
        let (meta, kayit): (YanitMeta, IstekSonucu) = match &cevap {
            Cevap::Yanit(yanit) => (yanit.meta.clone(), IstekSonucu::Basarili),
            Cevap::Hata(hata) => (YanitMeta::default(), kayit_hatasi(hata)),
            Cevap::ZamanAsimi => (
                YanitMeta::default(),
                IstekSonucu::Hata {
                    asama: IstekAsamasi::Yanit,
                    mesaj: "zaman aşımı".into(),
                },
            ),
            Cevap::Iptal => (
                YanitMeta::default(),
                IstekSonucu::Hata {
                    asama: IstekAsamasi::Yanit,
                    mesaj: "iptal edildi".into(),
                },
            ),
        };
        // Kullanım yalnız ağa çıkmış denemeden gelir: başarılı yanıtın gövdesinden,
        // hatada hata sınıfına göre; zaman aşımı ve iptal bilinmeyendir (5b §2, §6).
        let kullanim = match &cevap {
            Cevap::Yanit(yanit) => ham_kullanim(&yanit.govde),
            Cevap::Hata(hata) => hata_kullanimi(hata),
            Cevap::ZamanAsimi | Cevap::Iptal => None,
        };
        self.ortam
            .istek_bitti(ozet, meta, kayit, butce_islem, kullanim)
            .await;
        match cevap {
            Cevap::Yanit(yanit) if yanit.govde.len() > sinir::EVREN_YANITI => {
                Err(Hata::SinirAsildi(format!(
                    "EVREN yanıtı en çok {} MiB olabilir",
                    sinir::EVREN_YANITI / (1024 * 1024)
                )))
            }
            Cevap::Yanit(yanit) => Ok(yanit.govde),
            Cevap::Hata(hata) => Err(eklenti_hatasi(hata)),
            Cevap::ZamanAsimi => Err(Hata::Evren("zaman aşımı".into())),
            Cevap::Iptal => Err(Hata::IptalEdildi),
        }
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
        self.cikti()
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
        self.cikti()
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

    async fn evren_cagir(&mut self, uc: EvrenUcu, govde: String) -> Result<String, Hata> {
        self.cagri()?.evren_cagir(uc, govde).await
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
