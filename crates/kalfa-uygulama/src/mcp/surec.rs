//! MCP süreç durumu, saat soyutlaması, tanım çözümlemesi ve etkileşimli süreç sarmalayıcısı.
//!
//! Tasarım §4.1, §4.2, §4.3, §5.1.1, §5.5 ve §7 uyarınca:
//! - Her çalışan kuşak sabitlenmiş bir `R` (`McpTarifi`) ve artan `N`
//!   (`McpBaslatmaSayaci`) taşır.
//! - `t_K` (mutlak koruma denetimi yaşı, 10 dk) ve `son_cagri_ani` (boşta
//!   kapanma süresi, 10 dk) birbirinden bağımsızdır ve `SaatKaynagi` ile
//!   deterministik olarak ölçülebilir.
//! - Çökme sonrası yeniden başlatma en çok 3 kez (`1 sn`, `4 sn`, `16 sn`
//!   geri çekilmeyle) yapılır; iptal, boşta kapanma ve koruma yenilemesi çökme
//!   sayılmaz.
//! - `stderr` akışı `StderrHalkasi` üzerinden maskelenerek son 50 satır /
//!   200 KiB sınırında tutulur.

use std::collections::BTreeMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use kalfa_core::arac::{
    McpBaslatmaSayaci, McpKorumaOzeti, McpPolitikaOzeti, McpTanimOzeti, McpTarifi,
};
use kalfa_mcp::istemci::{McpAracCevabi, McpHatasi, McpIstemcisi};
use kalfa_mcp::maske::{GizliDeger, StderrHalkasi, maskele_metin};
use kalfa_mcp::ortam::OrtamDegeri;
use kalfa_mcp::tanim::{
    AgIzinKurali, CozulmusTanim, KodGuveni, Komut, McpTanim, SeciliYurutulebilirKimligi,
    TanimHatasi, YazilabilirKokler,
};
use kalfa_mcp::tasima::{SinirliTasima, TasimaHatasi, YenilemeDurumu};
use kalfa_mcp::{CallToolResult, JsonObject, Tool};
use kalfa_sandbox::{
    AlanBilgisi, BaslatmaHatasi, CalistirmaKipi, EtkilesimliIstek, EtkilesimliSurec, KapanisSonucu,
    OzelDizinler, Yalitici, sunucu_ozel,
};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use tokio::sync::Notify;

use crate::mcp::ayarlar::{McpKipi, McpKomut, McpOrtamDegeri, McpSunucu as McpSunucuKaydi};
use crate::mcp::kasa::{GizliKasa, GizliKimlik};
use crate::mcp::tanimlar::dosyayi_ozetle;

/// Mutlak koruma denetimi yaşı üst sınırı (`t_K`'dan itibaren 10 dk; §4.3).
pub const MUTLAK_KORUMA_YASI: Duration = Duration::from_secs(10 * 60);

/// Son çağrıdan itibaren boşta kapanma süresi (10 dk; §4.3).
pub const BOSTA_KAPANMA_SURESI: Duration = Duration::from_secs(10 * 60);

/// Çökme sonrası yeniden başlatma geri çekilme süreleri (`1 sn`, `4 sn`, `16 sn`; §7.4).
pub const COKME_GERI_CEKILME_SURELERI: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(4),
    Duration::from_secs(16),
];

/// Çökme sonrası en çok otomatik yeniden başlatma denemesi (§7.4).
pub const AZAMI_COKME_DENEMESI: u8 = 3;

/// Yönetici zamanlayıcıları (`t_K`, boşta kapanma ve çökme geri çekilmesi) için
/// saat kaynağı.
pub trait SaatKaynagi: Send + Sync {
    /// Başlangıçtan itibaren geçen süreyi milisaniye cinsinden döner.
    fn simdi_ms(&self) -> u64;

    /// Şimdiki zamanı `Duration` olarak döner.
    fn simdi(&self) -> Duration {
        Duration::from_millis(self.simdi_ms())
    }
}

/// Gerçek monotonik sistem saati.
pub struct SistemSaati {
    baslangic: Instant,
}

impl SistemSaati {
    pub fn yeni() -> Self {
        Self {
            baslangic: Instant::now(),
        }
    }
}

impl Default for SistemSaati {
    fn default() -> Self {
        Self::yeni()
    }
}

impl SaatKaynagi for SistemSaati {
    fn simdi_ms(&self) -> u64 {
        u64::try_from(self.baslangic.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

/// Testlerde mutlak koruma yaşını (`t_K`), boşta kapanmayı, 30 sn tarama
/// zaman aşımını ve `1/4/16 sn` çökme geri çekilmesini deterministik olarak
/// sınamak için kullanılan sahte saat.
#[derive(Default)]
pub struct SahteSaat {
    ms: AtomicU64,
    bildirim: Notify,
}

impl SahteSaat {
    pub fn yeni() -> Arc<Self> {
        Arc::new(Self {
            ms: AtomicU64::new(0),
            bildirim: Notify::new(),
        })
    }

    pub fn ilerlet(&self, sure: Duration) {
        let ek = u64::try_from(sure.as_millis()).unwrap_or(u64::MAX);
        self.ms.fetch_add(ek, Ordering::SeqCst);
        self.bildirim.notify_waiters();
    }

    pub fn ayarla(&self, sure: Duration) {
        let deger = u64::try_from(sure.as_millis()).unwrap_or(u64::MAX);
        self.ms.store(deger, Ordering::SeqCst);
        self.bildirim.notify_waiters();
    }

    pub fn bildirim(&self) -> &Notify {
        &self.bildirim
    }
}

impl SaatKaynagi for SahteSaat {
    fn simdi_ms(&self) -> u64 {
        self.ms.load(Ordering::SeqCst)
    }
}

/// MCP sunucusunun hata nedenleri (§10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tur", rename_all = "snake_case")]
pub enum McpHataNedeni {
    KomutBulunamadi { komut: String, path_ozeti: String },
    ElSikismaZamanAsimi,
    ProtokolSurumuUyusmuyor { surum: String },
    CerceveSiniri,
    IletiSeli,
    AracListesiSiniri,
    YalitimYok,
    KimlikDeposuKullanilamiyor,
    KorumaDogrulanamadi { detay: String },
    CokmeSiniriAsildi { deneme: u8, son_hata: String },
    Durdurulamadi { detay: String },
    BaslatmaHatasi { detay: String },
}

impl McpHataNedeni {
    pub fn kullanici_mesaji(&self) -> String {
        match self {
            Self::KomutBulunamadi { komut, path_ozeti } => {
                format!("`{komut}` bulunamadı; PATH: {path_ozeti}")
            }
            Self::ElSikismaZamanAsimi => "MCP el sıkışması zaman aşımına uğradı".to_owned(),
            Self::ProtokolSurumuUyusmuyor { surum } => {
                format!("MCP protokol sürümü uyuşmuyor ({surum})")
            }
            Self::CerceveSiniri => "çerçeve sınırı".to_owned(),
            Self::IletiSeli => "ileti seli".to_owned(),
            Self::AracListesiSiniri => "araç listesi çok uzun".to_owned(),
            Self::YalitimYok => {
                "sunucu yalıtımlı çalışmak üzere yapılandırıldı ancak yalıtım hazır değil"
                    .to_owned()
            }
            Self::KimlikDeposuKullanilamiyor => "kimlik deposu kullanılamıyor".to_owned(),
            Self::KorumaDogrulanamadi { detay } => format!("korumalar doğrulanamadı: {detay}"),
            Self::CokmeSiniriAsildi { deneme, son_hata } => {
                format!("sunucu {deneme} kez yeniden başlatıldı ancak çöktü: {son_hata}")
            }
            Self::Durdurulamadi { detay } => format!("sunucu durdurulamadı: {detay}"),
            Self::BaslatmaHatasi { detay } => detay.clone(),
        }
    }
}

/// Çalışma alanındaki bir MCP sunucusunun çalışma zamanı durumu (§9, Görev 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpSunucuDurumu {
    Durdu,
    Basliyor {
        kusak: McpBaslatmaSayaci,
        tarif: McpTarifi,
    },
    Calisiyor {
        kusak: McpBaslatmaSayaci,
        tarif: McpTarifi,
    },
    BostaKapandi,
    Hata {
        mesaj: String,
        neden: McpHataNedeni,
    },
}

impl McpSunucuDurumu {
    pub fn hata(neden: McpHataNedeni) -> Self {
        let mesaj = neden.kullanici_mesaji();
        Self::Hata { mesaj, neden }
    }

    pub fn tarif(&self) -> Option<&McpTarifi> {
        match self {
            Self::Basliyor { tarif, .. } | Self::Calisiyor { tarif, .. } => Some(tarif),
            _ => None,
        }
    }

    pub fn kusak(&self) -> Option<McpBaslatmaSayaci> {
        match self {
            Self::Basliyor { kusak, .. } | Self::Calisiyor { kusak, .. } => Some(*kusak),
            _ => None,
        }
    }
}

/// Kayıtlı bir sunucunun başlatma öncesi çözülmüş ve özetlenmiş hali.
pub struct HazirlanmisSunucu {
    pub ad: String,
    pub etkin: bool,
    pub kip: CalistirmaKipi,
    pub kod_guveni: KodGuveni,
    pub tanim_ozeti: McpTanimOzeti,
    pub cozulmus: CozulmusTanim,
    pub sirlar: Vec<GizliDeger>,
}

impl HazirlanmisSunucu {
    pub fn ag_ozeti(&self, alan_bilgisi: &AlanBilgisi) -> Result<Option<String>, TanimHatasi> {
        let istek = self.cozulmus.etkilesimli_istek(alan_bilgisi.clone())?;
        Ok(istek.ag_listesi.as_ref().map(|l| l.ozet()))
    }
}

/// Kayıtlı sunucu tanımını başlatma için çözer ve `T` (`McpTanimOzeti`) üretir.
pub fn kayitli_sunucuyu_hazirla_ve_ozetle(
    kayit: &McpSunucuKaydi,
    secili_yurutulebilirler: &BTreeMap<String, PathBuf>,
    kasa: &dyn GizliKasa,
    sistem_ortami: &[(String, String)],
    yalitim_dizini: &Path,
    proje_koku: &Path,
    alan_kimligi: &str,
) -> Result<HazirlanmisSunucu, McpHataNedeni> {
    let mut ortam_degerleri = Vec::with_capacity(kayit.ortam.len());
    let mut sirlar = Vec::new();

    for girdi in &kayit.ortam {
        match &girdi.deger {
            McpOrtamDegeri::Duz { deger } => {
                ortam_degerleri.push(OrtamDegeri::duz(girdi.ad.clone(), deger.clone()));
            }
            McpOrtamDegeri::Gizli { kimlik } => {
                let dogrulanmis = GizliKimlik::yeni_dogrulanmis(kimlik.as_str())
                    .map_err(|_| McpHataNedeni::KimlikDeposuKullanilamiyor)?;
                let deger = kasa
                    .oku(&dogrulanmis)
                    .map_err(|_| McpHataNedeni::KimlikDeposuKullanilamiyor)?
                    .ok_or(McpHataNedeni::KimlikDeposuKullanilamiyor)?;
                let sir = GizliDeger::yeni(&deger)
                    .map_err(|_| McpHataNedeni::KimlikDeposuKullanilamiyor)?;
                sirlar.push(sir);
                ortam_degerleri.push(OrtamDegeri::gizli(girdi.ad.clone(), deger));
            }
        }
    }

    let ham_komut_adi = match &kayit.komut {
        McpKomut::ProgramAdi { ad, .. } => ad.clone(),
        McpKomut::SeciliYurutulebilir { kimlik, .. } => kimlik.clone(),
    };

    let (komut, ic_esleme) = match &kayit.komut {
        McpKomut::ProgramAdi { ad, argumanlar } => (
            Komut::ProgramAdi {
                ad: ad.clone(),
                argumanlar: argumanlar.clone(),
            },
            None,
        ),
        McpKomut::SeciliYurutulebilir { kimlik, argumanlar } => {
            let Some(yol) = secili_yurutulebilirler.get(kimlik).cloned() else {
                return Err(McpHataNedeni::KomutBulunamadi {
                    komut: ham_komut_adi,
                    path_ozeti: "(seçili yürütülebilir kaydı yok)".to_owned(),
                });
            };
            let ic_kimlik = SeciliYurutulebilirKimligi::yeni(1);
            (
                Komut::SeciliYurutulebilir {
                    kimlik: ic_kimlik,
                    argumanlar: argumanlar.clone(),
                },
                Some((ic_kimlik, yol)),
            )
        }
    };

    let kip = match kayit.kip {
        McpKipi::Yalitimli => CalistirmaKipi::Yalitimli,
        McpKipi::TamErisim => CalistirmaKipi::TamErisim,
    };

    let ag = if kayit.ag_izinleri.is_empty() {
        None
    } else {
        let mut kurallar = Vec::with_capacity(kayit.ag_izinleri.len());
        for k in &kayit.ag_izinleri {
            let giris = format!("{}:{}", k.host, k.port);
            let kural = AgIzinKurali::yeni(&giris, k.ozel_ag).map_err(|h| {
                McpHataNedeni::BaslatmaHatasi {
                    detay: maskele_metin(&h.to_string(), &sirlar),
                }
            })?;
            kurallar.push(kural);
        }
        Some(kurallar)
    };

    let tanim = McpTanim::yeni(
        kayit.ad.clone(),
        komut,
        ortam_degerleri,
        kip,
        ag,
        kayit.baslatma_zamani_asimi_sn,
        kayit.cagri_zamani_asimi_sn,
    )
    .map_err(|h| McpHataNedeni::BaslatmaHatasi {
        detay: maskele_metin(&h.to_string(), &sirlar),
    })?;

    let ozel = sunucu_ozel(yalitim_dizini, alan_kimligi, &kayit.ad);
    let kabuk_ozel = OzelDizinler::yol(yalitim_dizini, alan_kimligi);
    // Eksik özel kökler yol çözümlemesinde temsil edilebilir. Oluşturma,
    // özellikle Windows mirassız izinleri için, yalnız yalıtıcıya aittir.

    let yazilabilir_kokler = YazilabilirKokler::yeni(proje_koku.to_path_buf(), kabuk_ozel);
    let path_gorunumu = sistem_ortami
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("PATH"))
        .map(|(_, v)| maskele_metin(v, &sirlar))
        .unwrap_or_else(|| "(boş)".to_owned());

    let cozulmus = tanim
        .hazirla(
            sistem_ortami,
            &ozel,
            &yazilabilir_kokler,
            &|aranan_kimlik| match &ic_esleme {
                Some((beklenen, yol)) if *beklenen == aranan_kimlik => Ok(yol.clone()),
                _ => Err(TanimHatasi::KomutCozulemedi),
            },
        )
        .map_err(|h| match h {
            TanimHatasi::KomutCozulemedi => McpHataNedeni::KomutBulunamadi {
                komut: maskele_metin(&ham_komut_adi, &sirlar),
                path_ozeti: path_gorunumu,
            },
            diger => McpHataNedeni::BaslatmaHatasi {
                detay: maskele_metin(&diger.to_string(), &sirlar),
            },
        })?;

    let mut dosyalar = Vec::with_capacity(cozulmus.dosya_adaylari().len());
    for aday in cozulmus.dosya_adaylari() {
        let ozet = dosyayi_ozetle(aday).map_err(|h| McpHataNedeni::BaslatmaHatasi {
            detay: maskele_metin(&h.to_string(), &sirlar),
        })?;
        dosyalar.push(ozet);
    }

    let tanim_ozeti = kalfa_mcp::tarif::tanim_ozeti(&tanim, &cozulmus, &dosyalar, alan_kimligi)
        .map_err(|h| McpHataNedeni::BaslatmaHatasi {
            detay: maskele_metin(&h.to_string(), &sirlar),
        })?;

    Ok(HazirlanmisSunucu {
        ad: kayit.ad.clone(),
        etkin: kayit.etkin,
        kip,
        kod_guveni: cozulmus.kod_guveni(),
        tanim_ozeti,
        cozulmus,
        sirlar,
    })
}

/// Çalışan bir MCP süreç örneğinin (veya test sürecinin) işlemleri.
pub trait McpSurecUcu: Send + Sync {
    /// Sürecin MCP el sıkışmasını ve istemci hazırlığını tamamlar.
    ///
    /// Sahte uçlar ek hazırlık gerektirmediğinden varsayılan olarak hazırdır.
    fn hazirlan<'a>(
        &'a self,
        _zaman_asimi: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<(), McpHataNedeni>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }

    fn araclari_listele<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Tool>, McpHatasi>> + Send + 'a>>;

    fn arac_cagir<'a>(
        &'a self,
        ad: String,
        argumanlar: JsonObject,
    ) -> Pin<Box<dyn Future<Output = Result<CallToolResult, McpHatasi>> + Send + 'a>>;

    /// Ham JSON-RPC yanıtındaki `content` alanı varlığıyla birlikte çağrı cevabını döner.
    /// Gerçek süreç ucu bu metodu doğrudan sınırlı taşımanın işaretiyle uygular.
    fn arac_cagir_cevabi<'a>(
        &'a self,
        ad: String,
        argumanlar: JsonObject,
    ) -> Pin<Box<dyn Future<Output = Result<McpAracCevabi, McpHatasi>> + Send + 'a>> {
        Box::pin(async move {
            let sonuc = self.arac_cagir(ad, argumanlar).await?;
            let content_alani_var = !sonuc.content.is_empty() || sonuc.structured_content.is_none();
            Ok(McpAracCevabi {
                sonuc,
                content_alani_var,
            })
        })
    }

    /// Sürecin araç listesi yenileme durumunu döner.
    fn yenileme_durumu(&self) -> Option<Arc<YenilemeDurumu>> {
        None
    }

    fn bitisi_bekle<'a>(&'a self) -> Pin<Box<dyn Future<Output = Option<i32>> + Send + 'a>>;

    /// İstemci/taşıma ve çocuk süreci kapatıp ağacın durduğunu doğrular.
    /// Uç paylaşılabildiğinden kapanış sahibi `self` değil içteki süreç
    /// yuvasıdır; yuva boşsa başarı uydurulmaz ve `Err` döner.
    fn kapat<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<KapanisSonucu>> + Send + 'a>>;
}

/// Etkileşimli MCP sürecini başlatan soyutlama.
///
/// `baslat` çağrısı süreci eşzamanlı olarak oluşturur ve süreç sahibini
/// döndürür. Çağıran bu metodu senkron rezervasyon kilidinin altında
/// çalıştırabilir; dönen sürecin `hazirlan` geleceğini kilit dışında bekler.
pub trait McpSurecBaslatici: Send + Sync {
    fn baslat(
        &self,
        yalitici: &Arc<dyn Yalitici>,
        istek: EtkilesimliIstek,
        stderr_halkasi: Arc<Mutex<StderrHalkasi>>,
    ) -> Result<Arc<dyn McpSurecUcu>, McpHataNedeni>;
}

/// Gerçek `EtkilesimliSurec` ve isteğe bağlı `McpIstemcisi` el sıkışması kullanan
/// süreç başlatıcısı.
pub struct GercekMcpSurecBaslatici {
    /// Üretimde `true` (`McpIstemcisi::yeni` ile el sıkışma yapar). Süreç
    /// yaşam döngüsünü ham komutlarla sınayan birim testlerinde `false`
    /// seçilebilir.
    el_sikisma: bool,
}

impl GercekMcpSurecBaslatici {
    pub fn yeni() -> Self {
        Self { el_sikisma: true }
    }

    pub fn el_sikismasiz() -> Self {
        Self { el_sikisma: false }
    }
}

impl Default for GercekMcpSurecBaslatici {
    fn default() -> Self {
        Self::yeni()
    }
}

struct GercekMcpSurecUcu {
    istemci: tokio::sync::Mutex<Option<McpIstemcisi>>,
    yenileme: Mutex<Option<Arc<YenilemeDurumu>>>,
    tasima: tokio::sync::Mutex<
        Option<(
            SinirliTasima,
            tokio::sync::watch::Receiver<Option<TasimaHatasi>>,
        )>,
    >,
    hazirlik_hatasi: Option<McpHataNedeni>,
    istemci_etkin: bool,
    surec: tokio::sync::Mutex<Option<EtkilesimliSurec>>,
    stderr_gorevi: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl McpSurecUcu for GercekMcpSurecUcu {
    fn hazirlan<'a>(
        &'a self,
        zaman_asimi: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<(), McpHataNedeni>> + Send + 'a>> {
        Box::pin(async move {
            if !self.istemci_etkin {
                return Ok(());
            }
            if let Some(hata) = &self.hazirlik_hatasi {
                return Err(hata.clone());
            }

            let tasima = self.tasima.lock().await.take();
            let Some((tasima, hata)) = tasima else {
                if self.istemci.lock().await.is_some() {
                    return Ok(());
                }
                return Err(McpHataNedeni::BaslatmaHatasi {
                    detay: "MCP istemci hazırlığı kullanılamıyor".to_owned(),
                });
            };

            let baglanti =
                tokio::time::timeout(zaman_asimi, McpIstemcisi::yeni(tasima, hata)).await;
            let istemci = match baglanti {
                Ok(Ok(istemci)) => istemci,
                Ok(Err(hata)) => return Err(mcp_hatasini_nedene_cevir(&hata)),
                Err(_) => return Err(McpHataNedeni::ElSikismaZamanAsimi),
            };

            *self
                .yenileme
                .lock()
                .unwrap_or_else(|hata| hata.into_inner()) = Some(istemci.yenileme_durumu());
            let mut kilit = self.istemci.lock().await;
            *kilit = Some(istemci);
            Ok(())
        })
    }

    fn araclari_listele<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Tool>, McpHatasi>> + Send + 'a>> {
        Box::pin(async move {
            let istek = {
                let kilit = self.istemci.lock().await;
                let Some(istemci) = kilit.as_ref() else {
                    return Ok(Vec::new());
                };
                istemci.araclari_listele()
            };
            istek.await
        })
    }

    fn arac_cagir<'a>(
        &'a self,
        ad: String,
        argumanlar: JsonObject,
    ) -> Pin<Box<dyn Future<Output = Result<CallToolResult, McpHatasi>> + Send + 'a>> {
        Box::pin(async move {
            let istek = {
                let kilit = self.istemci.lock().await;
                let Some(istemci) = kilit.as_ref() else {
                    return Err(McpHatasi::Kapali);
                };
                istemci.arac_cagir(ad, argumanlar)
            };
            istek.await
        })
    }

    fn arac_cagir_cevabi<'a>(
        &'a self,
        ad: String,
        argumanlar: JsonObject,
    ) -> Pin<Box<dyn Future<Output = Result<McpAracCevabi, McpHatasi>> + Send + 'a>> {
        Box::pin(async move {
            let istek = {
                let kilit = self.istemci.lock().await;
                let Some(istemci) = kilit.as_ref() else {
                    return Err(McpHatasi::Kapali);
                };
                istemci.arac_cagir_cevabi(ad, argumanlar)
            };
            istek.await
        })
    }

    fn yenileme_durumu(&self) -> Option<Arc<YenilemeDurumu>> {
        self.yenileme
            .lock()
            .unwrap_or_else(|hata| hata.into_inner())
            .clone()
    }

    fn bitisi_bekle<'a>(&'a self) -> Pin<Box<dyn Future<Output = Option<i32>> + Send + 'a>> {
        Box::pin(async move {
            let mut kilit = self.surec.lock().await;
            let surec = kilit.as_mut()?;
            surec.bitisi_bekle().await
        })
    }

    fn kapat<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<KapanisSonucu>> + Send + 'a>> {
        Box::pin(async move {
            // İptal/flush ve stdin sonrası bekleme birlikte en çok iki saniyedir.
            let son = tokio::time::Instant::now() + kalfa_sandbox::etkilesimli::KAPANMA_PAYI;
            *self
                .yenileme
                .lock()
                .unwrap_or_else(|hata| hata.into_inner()) = None;
            let istemci = self.istemci.lock().await.take();
            if let Some(mut istemci) = istemci {
                istemci.kapat().await;
            }
            let tasima = self.tasima.lock().await.take();
            drop(tasima);
            let surec = self.surec.lock().await.take();
            let sonuc = match surec {
                Some(surec) => surec.kapat_kadar(son).await,
                None => Err(std::io::Error::other(
                    "süreç sahibi olmadan kapanış doğrulanamaz",
                )),
            };
            if let Some(gorev) = self.stderr_gorevi.lock().await.take() {
                let _ = tokio::time::timeout(Duration::from_millis(200), gorev).await;
            }
            sonuc
        })
    }
}

fn mcp_hatasini_nedene_cevir(hata: &McpHatasi) -> McpHataNedeni {
    match hata {
        McpHatasi::Tasima(TasimaHatasi::Cerceve) => McpHataNedeni::CerceveSiniri,
        McpHatasi::Tasima(TasimaHatasi::IletiSeli | TasimaHatasi::GidenIletiSeli) => {
            McpHataNedeni::IletiSeli
        }
        McpHatasi::AracListesiUzun | McpHatasi::AracSayisi | McpHatasi::TanimBoyutu => {
            McpHataNedeni::AracListesiSiniri
        }
        McpHatasi::UzakRpc(_) => McpHataNedeni::ProtokolSurumuUyusmuyor {
            surum: "desteklenmeyen protokol".to_owned(),
        },
        _ => McpHataNedeni::BaslatmaHatasi {
            detay: hata.to_string(),
        },
    }
}

fn stderr_okuyucu_baslat(
    mut okuyucu: Box<dyn tokio::io::AsyncRead + Send + Unpin>,
    halka: Arc<Mutex<StderrHalkasi>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut tampon = vec![0_u8; 4096];
        loop {
            match okuyucu.read(&mut tampon).await {
                Ok(0) | Err(_) => {
                    if let Ok(mut kilit) = halka.lock() {
                        kilit.akisi_bitir();
                    }
                    break;
                }
                Ok(n) => {
                    if let Ok(mut kilit) = halka.lock() {
                        kilit.besle(&tampon[..n]);
                    }
                }
            }
        }
    })
}

impl McpSurecBaslatici for GercekMcpSurecBaslatici {
    fn baslat(
        &self,
        yalitici: &Arc<dyn Yalitici>,
        istek: EtkilesimliIstek,
        stderr_halkasi: Arc<Mutex<StderrHalkasi>>,
    ) -> Result<Arc<dyn McpSurecUcu>, McpHataNedeni> {
        let el_sikisma = self.el_sikisma;
        let mut surec = yalitici
            .baslat_etkilesimli(&istek)
            .map_err(|hata| match hata {
                BaslatmaHatasi::ProgramYok { program, path } => McpHataNedeni::KomutBulunamadi {
                    komut: program,
                    path_ozeti: path,
                },
                BaslatmaHatasi::Yalitim(detay) => McpHataNedeni::BaslatmaHatasi { detay },
                diger => McpHataNedeni::BaslatmaHatasi {
                    detay: diger.to_string(),
                },
            })?;

        let stderr_gorevi = surec
            .hata_al()
            .map(|okuyucu| stderr_okuyucu_baslat(okuyucu, stderr_halkasi));

        let (tasima, hazirlik_hatasi) = if el_sikisma {
            match (surec.girdi_al(), surec.cikti_al()) {
                (Some(girdi), Some(cikti)) => {
                    let (tasima, hata) = SinirliTasima::yeni(cikti, girdi);
                    (Some((tasima, hata)), None)
                }
                (None, _) => (
                    None,
                    Some(McpHataNedeni::BaslatmaHatasi {
                        detay: "MCP sunucusu stdin borusu alınamadı".to_owned(),
                    }),
                ),
                (_, None) => (
                    None,
                    Some(McpHataNedeni::BaslatmaHatasi {
                        detay: "MCP sunucusu stdout borusu alınamadı".to_owned(),
                    }),
                ),
            }
        } else {
            (None, None)
        };

        Ok(Arc::new(GercekMcpSurecUcu {
            istemci: tokio::sync::Mutex::new(None),
            yenileme: Mutex::new(None),
            tasima: tokio::sync::Mutex::new(tasima),
            hazirlik_hatasi,
            istemci_etkin: el_sikisma,
            surec: tokio::sync::Mutex::new(Some(surec)),
            stderr_gorevi: tokio::sync::Mutex::new(stderr_gorevi),
        }))
    }
}

/// Çalışan veya başlamakta olan tek bir MCP süreç kuşağının (`N`) değişmez
/// üst verisi.
#[derive(Debug, Clone)]
pub struct CalisanKusakBilgisi {
    pub alan_kimligi: String,
    pub sunucu: String,
    pub kusak: McpBaslatmaSayaci,
    pub tarif: McpTarifi,
    pub tanim_ozeti: McpTanimOzeti,
    pub politika_ozeti: McpPolitikaOzeti,
    pub koruma_ozeti: McpKorumaOzeti,
    pub kod_guveni: KodGuveni,
    pub kip: CalistirmaKipi,
}
