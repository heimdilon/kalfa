//! Çalışma alanına bağlı MCP süreç yöneticisi, başlatma izinleri, yazma kuşağı
//! (`W`), mutlak koruma denetimi yaşı (`t_K`) ve çökme geri çekilmesi.
//!
//! Tasarım §4.1, §4.3, §5.1, §5.1.1, §5.3, §5.6, §7 ve §8 uyarınca:
//! - Tek sahip `(alan_kimligi, sunucu)` anahtarıyla süreç örneğini ve `N`
//!   (`McpBaslatmaSayaci`) sayacını tutar; `R` (`McpTarifi`) süreç başlarken
//!   sabitlenir.
//! - Kontrol kanalı (`sunucuyu_durdur`, `politika_degisti`, `oturum_kipi_guncelle`,
//!   `alan_kapanisini_baslat`, `zamanlayici_adimi`) `tools/call`, `tools/list`
//!   veya onay bekleme kuyruğunun arkasında beklemez.
//! - Süreç yalnız etkin tanım + İnceleme dışı açık oturum + geçerli başlatma
//!   izni + doğrulanmış güncel koruma (`K`) + `W` kod güveni koşulu birlikte
//!   sağlandığında oluşur.
//! - `Yalitimli` başlatma izni `(T, Π, alan_kimligi)` (+ `WdeKodlu` ise `W`)
//!   ile kalıcıdır; `TamErisim` başlatma izni yalnız uygulama ömründe bellekte
//!   tutulur ve yerel onay (`Iletisim::onayla`) gerektirir.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kalfa_core::arac::{
    McpBaslatmaSayaci, McpKorumaOzeti, McpPolitikaOzeti, McpTanimOzeti, McpTarifi, McpYazmaKusagi,
};
use kalfa_core::izin::Kip;
use kalfa_core::kimlik::OturumId;
use kalfa_mcp::istemci::McpHatasi;
use kalfa_mcp::maske::StderrHalkasi;
use kalfa_mcp::tanim::KodGuveni;
use kalfa_mcp::{CallToolResult, JsonObject, Tool};
use kalfa_sandbox::tarama::CancellationToken;
use kalfa_sandbox::{CalistirmaKipi, Yalitici, YalitimDurumu, mcp_kip_uyusmasi};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Notify;
use uuid::Uuid;

use crate::arayuz::Iletisim;
use crate::ayarlar::Ayarlar;
use crate::mcp::ayarlar::{McpAyarlarDeposu, McpSunucu as McpSunucuKaydi};
use crate::mcp::kasa::GizliKasa;
use crate::mcp::koruma::{
    GercekKorumaDenetleyicisi, KorumaDenetimIstegi, KorumaDenetleyicisi, McpAdliGirdiler,
    McpDogrulanmisKoruma, McpKorumaSonucu, SonDenetimIstegi, ozet_hex, politika_ozeti_hesapla,
    yalitim_profili_ozeti_hesapla,
};
use crate::mcp::surec::{
    AZAMI_COKME_DENEMESI, BOSTA_KAPANMA_SURESI, COKME_GERI_CEKILME_SURELERI,
    GercekMcpSurecBaslatici, HazirlanmisSunucu, MUTLAK_KORUMA_YASI, McpHataNedeni, McpSunucuDurumu,
    McpSurecBaslatici, McpSurecUcu, SaatKaynagi, SistemSaati, kayitli_sunucuyu_hazirla_ve_ozetle,
};
use crate::yalitim::YalitimAyarlari;

const IZIN_DOSYASI_ADI: &str = "mcp_izinleri.json";

#[path = "cagri.rs"]
mod cagri;
#[path = "zamanlayici.rs"]
mod zamanlayici;
use cagri::{McpCagriDefteri, McpCagriKapanisSonucu};
pub use cagri::{McpG7AracKarti, McpG7AracKaynagi};
const TAM_ERISIM_BASLATMA_BASLIGI: &str = "Tam sistem erişimli MCP sunucusunu başlat";

/// `(alan_kimligi, sunucu)` birleşik anahtarı.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct McpAnahtari {
    pub alan_kimligi: String,
    pub sunucu: String,
}

impl McpAnahtari {
    pub fn yeni(alan_kimligi: impl Into<String>, sunucu: impl Into<String>) -> Self {
        Self {
            alan_kimligi: alan_kimligi.into(),
            sunucu: sunucu.into(),
        }
    }
}

/// Yalıtımlı sunucu için kalıcı başlatma izni kaydı (§5.1, §5.1.1, §5.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KaliciYalitimliIzin {
    pub alan_kimligi: String,
    pub sunucu: String,
    pub tanim_ozeti_hex: String,
    pub politika_ozeti_hex: String,
    pub wde_kodlu: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub onayli_yazma_kusagi: Option<u64>,
}

/// Kalıcı izin dosyası (`mcp_izinleri.json`).
///
/// `TamErisim` izinleri bu dosyaya asla yazılmaz (§5.1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct KaliciIzinDosyasi {
    #[serde(default)]
    yalitimli_izinler: Vec<KaliciYalitimliIzin>,
    #[serde(default)]
    yazma_kusaklari: BTreeMap<String, u64>,
}

/// Yalnız bu uygulama ömründe bellekte yaşayan tam erişim başlatma izni (§5.1).
#[derive(Debug, Clone, PartialEq, Eq)]
struct GeciciTamErisimIzni {
    tanim_ozeti: McpTanimOzeti,
    kod_guveni: KodGuveni,
    onayli_yazma_kusagi: Option<McpYazmaKusagi>,
}

/// Model dışı başlatma izni kartı (§5.1, Görev 9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpBaslatmaKarti {
    pub istek_kimligi: String,
    pub alan_kimligi: String,
    pub sunucu: String,
    pub erisim_kipi: String,
    pub komut_ozeti: String,
    pub ag_listesi: Vec<String>,
    pub uyari: String,
    pub wde_kodlu: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BekleyenBaslatmaKarti {
    alan_kimligi: String,
    sunucu: String,
    kip: CalistirmaKipi,
    tanim_ozeti: McpTanimOzeti,
    politika_ozeti: McpPolitikaOzeti,
    kod_guveni: KodGuveni,
    yazma_kusagi: McpYazmaKusagi,
}

/// Yöneticinin hata tipleri.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum YoneticiHatasi {
    #[error("MCP sunucusu bulunamadı: {0}")]
    SunucuBulunamadi(String),
    #[error("MCP sunucusu devre dışı: {0}")]
    SunucuDevreDisi(String),
    #[error("çalışma alanı bulunamadı veya kaldırılıyor: {0}")]
    CalismaAlaniYok(String),
    #[error("çalışma alanında İnceleme dışı açık oturum yok")]
    IncelemeDisiOturumYok,
    #[error("MCP sunucusu için başlatma izni gerekiyor")]
    BaslatmaIzniGerekli(Box<McpBaslatmaKarti>),
    #[error("kullanıcı yerel onay penceresinde başlatmayı reddetti")]
    YerelOnayReddedildi,
    #[error("başlatma kartı geçersiz, süresi dolmuş veya çalışma alanı/yazma kuşağı değişmiş")]
    GecersizBaslatmaKarti,
    #[error("başlatma sırasında yazma kuşağı veya ayarlar değişti")]
    BaslatmaYarisi,
    #[error("çökme geri çekilme süresi henüz dolmadı ({kalan_ms} ms)")]
    CokmeGeriCekilmeBekleniyor { kalan_ms: u64 },
    #[error("korumalar değişti, sunucu durduruldu")]
    KorumalarDegisti,
    #[error("korumalar doğrulanamadı: {0}")]
    KorumalarDogrulanamadi(String),
    #[error("sunucu durduruldu")]
    SunucuDurduruldu,
    #[error("sunucu çöktü")]
    SunucuCoktu,
    #[error("sunucu durdurulamadı: {0}")]
    SunucuDurdurulamadi(String),
    #[error("uzak MCP araç hatası ({})", .0.kod())]
    UzakRpc(kalfa_mcp::istemci::RpcHatasi),
    #[error("{0}")]
    SunucuHatasi(String),
}

struct CalisanKusak {
    kusak: McpBaslatmaSayaci,
    tarif: McpTarifi,
    tanim_ozeti: McpTanimOzeti,
    politika_ozeti: McpPolitikaOzeti,
    koruma_ozeti: McpKorumaOzeti,
    kod_guveni: KodGuveni,
    kip: CalistirmaKipi,
    ag_ozeti: Option<String>,
    sirlar: Vec<kalfa_mcp::maske::GizliDeger>,
    t_k_ani_ms: AtomicU64,
    son_cagri_ani_ms: AtomicU64,
    etkin_cagri_sayisi: AtomicUsize,
    kesif_suruyor: AtomicBool,
    onay_bekliyor: AtomicBool,
    zamanlayici_bildirimi: Notify,
    koruma_kilidi: tokio::sync::Mutex<()>,
    iptal: CancellationToken,
    iptal_nedeni: Mutex<Option<YoneticiHatasi>>,
    adli_girdiler: Mutex<McpAdliGirdiler>,
    // RPC, çıkış izlemesi ve kapanış kuyruk kilidi paylaşmaz.
    ucu: Arc<dyn McpSurecUcu>,
}

impl CalisanKusak {
    fn iptal_et(&self, neden: YoneticiHatasi) {
        if let Ok(mut kilit) = self.iptal_nedeni.lock()
            && kilit.is_none()
        {
            *kilit = Some(neden);
        }
        self.iptal.cancel();
    }

    fn iptal_hatasi(&self) -> YoneticiHatasi {
        self.iptal_nedeni
            .lock()
            .ok()
            .and_then(|k| k.clone())
            .unwrap_or(YoneticiHatasi::SunucuDurduruldu)
    }
}

/// Gelecek bırakıldığında da etkinlik sayacını ve boşta zamanını kapatır.
struct EtkinlikKaydi {
    kusak: Arc<CalisanKusak>,
    saat: Arc<dyn SaatKaynagi>,
    cagri: bool,
}

struct KorumaBildirimi<'a>(&'a Notify);

impl Drop for KorumaBildirimi<'_> {
    fn drop(&mut self) {
        self.0.notify_one();
    }
}

impl EtkinlikKaydi {
    fn yeni(kusak: Arc<CalisanKusak>, saat: Arc<dyn SaatKaynagi>, cagri: bool) -> Self {
        kusak.etkin_cagri_sayisi.fetch_add(1, Ordering::SeqCst);
        if cagri {
            kusak
                .son_cagri_ani_ms
                .store(saat.simdi_ms(), Ordering::SeqCst);
        }
        kusak.zamanlayici_bildirimi.notify_one();
        Self { kusak, saat, cagri }
    }
}

impl Drop for EtkinlikKaydi {
    fn drop(&mut self) {
        if self.cagri {
            self.kusak
                .son_cagri_ani_ms
                .store(self.saat.simdi_ms(), Ordering::SeqCst);
        }
        self.kusak.etkin_cagri_sayisi.fetch_sub(1, Ordering::SeqCst);
        self.kusak.zamanlayici_bildirimi.notify_one();
    }
}

struct SunucuYuvasi {
    baslatma_revizyonu: u64,
    son_kusak: McpBaslatmaSayaci,
    durum: McpSunucuDurumu,
    calisan: Option<Arc<CalisanKusak>>,
    stderr_halkasi: Arc<Mutex<StderrHalkasi>>,
    cokme_sayaci: u8,
    sonraki_cokme_izni_ani_ms: Option<u64>,
    dogrulanamadi_hatasi: Option<String>,
    durdurulamadi_hatasi: Option<String>,
    aktif_kapanis_sayisi: usize,
    kapanis_bildirimi: Arc<Notify>,
}

/// Başlatma geleceğini bırakmak, doğrulanmış kapanış işini de bırakmaz.
struct BaslatmaTemizligi {
    yonetici: Arc<McpYoneticisi>,
    anahtar: McpAnahtari,
    kusak: McpBaslatmaSayaci,
    tamamlandi: bool,
}

impl Drop for BaslatmaTemizligi {
    fn drop(&mut self) {
        if self.tamamlandi {
            return;
        }
        let kapanacak = {
            let mut d = self.yonetici.durum();
            if !d
                .yuvalar
                .get(&self.anahtar)
                .and_then(|y| y.calisan.as_ref())
                .is_some_and(|c| c.kusak == self.kusak)
            {
                return;
            }
            McpYoneticisi::yuvadan_sureci_ayir(
                &mut d,
                &self.anahtar,
                YoneticiHatasi::SunucuDurduruldu,
                McpSunucuDurumu::Durdu,
            )
        };
        if let Some((anahtar, kusak, kok)) = kapanacak {
            self.yonetici.arka_planda_kapat(anahtar, kusak, kok);
        }
    }
}

impl SunucuYuvasi {
    fn yeni() -> Self {
        Self {
            baslatma_revizyonu: 0,
            son_kusak: McpBaslatmaSayaci::yeni(0),
            durum: McpSunucuDurumu::Durdu,
            calisan: None,
            stderr_halkasi: Arc::new(Mutex::new(StderrHalkasi::yeni(Vec::new()))),
            cokme_sayaci: 0,
            sonraki_cokme_izni_ani_ms: None,
            dogrulanamadi_hatasi: None,
            durdurulamadi_hatasi: None,
            aktif_kapanis_sayisi: 0,
            kapanis_bildirimi: Arc::new(Notify::new()),
        }
    }
}

#[derive(Default)]
struct AlanDurumu {
    kok: Option<PathBuf>,
    yazma_kusagi: McpYazmaKusagi,
    oturum_kipleri: BTreeMap<OturumId, Kip>,
    kaldiriliyor: bool,
}

impl AlanDurumu {
    fn inceleme_disi_oturum_var(&self) -> bool {
        self.oturum_kipleri
            .values()
            .any(|kip| *kip != Kip::Inceleme)
    }
}

struct YoneticiDurumu {
    alanlar: BTreeMap<String, AlanDurumu>,
    yuvalar: BTreeMap<McpAnahtari, SunucuYuvasi>,
    yalitimli_izinler: BTreeMap<McpAnahtari, KaliciYalitimliIzin>,
    gecici_tam_erisim_izinleri: BTreeMap<McpAnahtari, GeciciTamErisimIzni>,
    bekleyen_kartlar: BTreeMap<String, BekleyenBaslatmaKarti>,
    /// Bekleyen kartların arayüz görüntüsü; kimliği `bekleyen_kartlar`da
    /// kalmayan görüntü okunurken atılır (Görev 10).
    kart_gorunumleri: BTreeMap<String, McpBaslatmaKarti>,
    son_politika_ozeti: McpPolitikaOzeti,
    politika_revizyonu: u64,
    sistem_ortami: Vec<(String, String)>,
}

/// Çalışma alanına bağlı MCP süreçlerini yöneten ana yapı.
pub struct McpYoneticisi {
    veri_dizini: PathBuf,
    yalitim_dizini: PathBuf,
    ayarlar: Arc<Ayarlar>,
    mcp_depo: Arc<dyn McpAyarlarDeposu>,
    kasa: Arc<dyn GizliKasa>,
    iletisim: Arc<dyn Iletisim>,
    yalitici: Arc<dyn Yalitici>,
    yalitim_ayarlari: Arc<YalitimAyarlari>,
    koruma_denetleyicisi: Arc<dyn KorumaDenetleyicisi>,
    surec_baslatici: Arc<dyn McpSurecBaslatici>,
    saat: Arc<dyn SaatKaynagi>,
    cagri_defteri: Arc<McpCagriDefteri>,
    durum: Mutex<YoneticiDurumu>,
}

/// `McpYoneticisi` yapılandırma bileşenleri.
pub struct McpYoneticiBilesenleri {
    pub veri_dizini: PathBuf,
    pub yalitim_dizini: PathBuf,
    pub ayarlar: Arc<Ayarlar>,
    pub mcp_depo: Arc<dyn McpAyarlarDeposu>,
    pub kasa: Arc<dyn GizliKasa>,
    pub iletisim: Arc<dyn Iletisim>,
    pub yalitici: Arc<dyn Yalitici>,
    pub yalitim_ayarlari: Arc<YalitimAyarlari>,
    pub koruma_denetleyicisi: Option<Arc<dyn KorumaDenetleyicisi>>,
    pub surec_baslatici: Option<Arc<dyn McpSurecBaslatici>>,
    pub saat: Option<Arc<dyn SaatKaynagi>>,
    pub sistem_ortami: Option<Vec<(String, String)>>,
}

impl McpYoneticisi {
    pub fn yeni(b: McpYoneticiBilesenleri) -> Arc<Self> {
        let izin_dosyasi = izin_dosyasini_oku(&b.veri_dizini);
        let mut yalitimli_izinler = BTreeMap::new();
        for izin in izin_dosyasi.yalitimli_izinler {
            let anahtar = McpAnahtari::yeni(izin.alan_kimligi.clone(), izin.sunucu.clone());
            yalitimli_izinler.insert(anahtar, izin);
        }
        let mut alanlar = BTreeMap::new();
        for (alan_kimligi, kusak) in izin_dosyasi.yazma_kusaklari {
            alanlar.insert(
                alan_kimligi,
                AlanDurumu {
                    kok: None,
                    yazma_kusagi: McpYazmaKusagi::yeni(kusak),
                    oturum_kipleri: BTreeMap::new(),
                    kaldiriliyor: false,
                },
            );
        }

        let son_politika_ozeti = politika_ozeti_hesapla(
            &b.yalitici.durum(),
            &b.ayarlar.gizli_ayarlari(),
            &b.yalitim_ayarlari.dizinler(),
        );
        // Kayıtlı yalıtımlı izinlerden şimdiki Π ile uyuşmayanları düşür:
        let guncel_pi_hex = ozet_hex(&son_politika_ozeti.sha256());
        yalitimli_izinler.retain(|_, izin| izin.politika_ozeti_hex == guncel_pi_hex);

        let sistem_ortami = b
            .sistem_ortami
            .unwrap_or_else(|| std::env::vars().collect());

        Arc::new(Self {
            veri_dizini: b.veri_dizini,
            yalitim_dizini: b.yalitim_dizini,
            ayarlar: b.ayarlar,
            mcp_depo: b.mcp_depo,
            kasa: b.kasa,
            iletisim: b.iletisim,
            yalitici: b.yalitici,
            yalitim_ayarlari: b.yalitim_ayarlari,
            koruma_denetleyicisi: b
                .koruma_denetleyicisi
                .unwrap_or_else(|| Arc::new(GercekKorumaDenetleyicisi::yeni())),
            surec_baslatici: b
                .surec_baslatici
                .unwrap_or_else(|| Arc::new(GercekMcpSurecBaslatici::yeni())),
            saat: b.saat.unwrap_or_else(|| Arc::new(SistemSaati::yeni())),
            cagri_defteri: Arc::new(McpCagriDefteri::default()),
            durum: Mutex::new(YoneticiDurumu {
                alanlar,
                yuvalar: BTreeMap::new(),
                yalitimli_izinler,
                gecici_tam_erisim_izinleri: BTreeMap::new(),
                bekleyen_kartlar: BTreeMap::new(),
                kart_gorunumleri: BTreeMap::new(),
                son_politika_ozeti,
                politika_revizyonu: 1,
                sistem_ortami,
            }),
        })
    }

    fn durum(&self) -> std::sync::MutexGuard<'_, YoneticiDurumu> {
        self.durum.lock().unwrap_or_else(|z| z.into_inner())
    }

    /// Şimdiki politika özetini (`Π`) hesaplar.
    pub fn guncel_politika_ozeti(&self) -> McpPolitikaOzeti {
        politika_ozeti_hesapla(
            &self.yalitici.durum(),
            &self.ayarlar.gizli_ayarlari(),
            &self.yalitim_ayarlari.dizinler(),
        )
    }

    /// Politika revizyon sayacını döner (testler ve kısayol geçersizleştirme için).
    pub fn politika_revizyonu(&self) -> u64 {
        self.durum().politika_revizyonu
    }

    /// Çalışma alanının kök yolunu kaydeder.
    pub fn calisma_alanini_kaydet(&self, alan_kimligi: &str, kok: &Path) {
        let mut d = self.durum();
        let alan = d.alanlar.entry(alan_kimligi.to_owned()).or_default();
        alan.kok = Some(kok.to_path_buf());
    }

    /// Oturum açıldığında, kipi değiştiğinde (`Some(kip)`) veya kapandığında (`None`)
    /// çalışma alanındaki açık oturum tablosunu günceller.
    ///
    /// Aynı alanda kalan son `İnceleme` dışı oturum kapanır ya da `İnceleme`
    /// kipine dönerse o alandaki bütün çalışan/başlayan MCP süreçleri hemen
    /// durdurulur (§5.1, §7.1). Sekme değişimi oturum kapanışı değildir.
    pub fn oturum_kipi_guncelle(
        self: &Arc<Self>,
        alan_kimligi: &str,
        kok: Option<&Path>,
        oturum: OturumId,
        kip: Option<Kip>,
    ) {
        let durdurulacaklar = {
            let mut d = self.durum();
            let alan = d.alanlar.entry(alan_kimligi.to_owned()).or_default();
            if let Some(k) = kok {
                alan.kok = Some(k.to_path_buf());
            }
            match kip {
                Some(k) => {
                    alan.oturum_kipleri.insert(oturum, k);
                }
                None => {
                    alan.oturum_kipleri.remove(&oturum);
                }
            }
            if alan.inceleme_disi_oturum_var() {
                Vec::new()
            } else {
                Self::alandaki_tum_surecleri_ayir(
                    &mut d,
                    alan_kimligi,
                    YoneticiHatasi::IncelemeDisiOturumYok,
                    McpSunucuDurumu::Durdu,
                )
            }
        };

        for (anahtar, kusak, kok_yolu) in durdurulacaklar {
            self.arka_planda_kapat(anahtar, kusak, kok_yolu);
        }
    }

    /// Çalışma alanının yazma kuşağını (`W`) döner.
    pub fn yazma_kusagi(&self, alan_kimligi: &str) -> McpYazmaKusagi {
        self.durum()
            .alanlar
            .get(alan_kimligi)
            .map(|a| a.yazma_kusagi)
            .unwrap_or_default()
    }

    /// Çalışma alanında yan etkili bir işlem (dosya yazma/düzenleme, kabuk komutu
    /// veya MCP çağrısı) başlamak üzereyken yazma kuşağını (`W`) atomik ve seri
    /// olarak artırır (§5.1.1, Görev 5.6).
    pub fn yazma_kusagini_artir(&self, alan_kimligi: &str) -> McpYazmaKusagi {
        let mut d = self.durum();
        let alan = d.alanlar.entry(alan_kimligi.to_owned()).or_default();
        alan.yazma_kusagi = alan.yazma_kusagi.ileri();
        let yeni = alan.yazma_kusagi;
        // Bu alanda bekleyen W'de kodlu başlatma kartlarını düşür:
        d.bekleyen_kartlar.retain(|_, kart| {
            !(kart.alan_kimligi == alan_kimligi && kart.kod_guveni == KodGuveni::WdeKodlu)
        });
        self.izin_dosyasini_kaydet_kilitli(&d);
        yeni
    }

    /// Kalfa'nın kendi yazma aracı veya kabuk komutu bittikten sonra o çalışma
    /// alanındaki çalışan MCP süreçlerinin koruma özetini (`K`) denetler (§4.3).
    pub async fn kalfa_yazmasi_sonrasi_denetle(self: &Arc<Self>, alan_kimligi: &str, kok: &Path) {
        self.alan_korumasini_denetle(alan_kimligi, kok).await;
    }

    /// Belirtilen çalışma alanındaki bütün çalışan MCP süreçleri için koruma
    /// doğrulaması (`K`) ve son denetim yapar (§4.3).
    pub async fn alan_korumasini_denetle(self: &Arc<Self>, alan_kimligi: &str, kok: &Path) {
        let anahtarlar: Vec<McpAnahtari> = {
            let d = self.durum();
            d.yuvalar
                .iter()
                .filter(|(k, y)| k.alan_kimligi == alan_kimligi && y.calisan.is_some())
                .map(|(k, _)| k.clone())
                .collect()
        };
        let mut gorevler = tokio::task::JoinSet::new();
        for anahtar in anahtarlar {
            let yonetici = self.clone();
            let kok = kok.to_path_buf();
            gorevler.spawn(async move {
                loop {
                    let calisan = yonetici
                        .durum()
                        .yuvalar
                        .get(&anahtar)
                        .and_then(|y| y.calisan.clone());
                    let sonuc = yonetici
                        .sunucu_korumasini_denetle(&anahtar.alan_kimligi, &anahtar.sunucu, &kok)
                        .await;
                    if !matches!(sonuc, Err(YoneticiHatasi::BaslatmaYarisi)) {
                        break;
                    }
                    if let Some(calisan) = calisan {
                        // Daha önce başlayan tarama yazının yeni korumalarını
                        // görmemiş olabilir. Bitişini bekleyip yeniden tararız.
                        let _tarama = calisan.koruma_kilidi.lock().await;
                    }
                }
            });
        }
        while gorevler.join_next().await.is_some() {}
    }

    /// Ayarlar veya yalıtım politikası (`Π`: gizli kalıplar, istisnalar, okunur
    /// dizinler, yalıtım arka ucu) değiştiğinde çalışan yalıtımlı sunucuları
    /// hemen kapatır ve `Π`'ye bağlı başlatma izinlerini düşürür (§4.3, §5.3).
    pub fn politika_degisti(self: &Arc<Self>) {
        let yeni_pi = self.guncel_politika_ozeti();
        let yeni_pi_hex = ozet_hex(&yeni_pi.sha256());
        let durdurulacaklar = {
            let mut d = self.durum();
            d.son_politika_ozeti = yeni_pi;
            d.politika_revizyonu = d.politika_revizyonu.saturating_add(1);
            d.yalitimli_izinler
                .retain(|_, izin| izin.politika_ozeti_hex == yeni_pi_hex);
            d.bekleyen_kartlar
                .retain(|_, kart| kart.politika_ozeti == yeni_pi);
            self.izin_dosyasini_kaydet_kilitli(&d);

            let yalitim_hazir = matches!(self.yalitici.durum(), YalitimDurumu::Hazir { .. });
            let mut liste = Vec::new();
            let anahtarlar: Vec<McpAnahtari> = d.yuvalar.keys().cloned().collect();
            for anahtar in anahtarlar {
                let yalitimli_mi = d
                    .yuvalar
                    .get(&anahtar)
                    .and_then(|y| y.calisan.as_ref())
                    .is_some_and(|c| c.kip == CalistirmaKipi::Yalitimli);
                if yalitimli_mi {
                    let hedef_durum = if yalitim_hazir {
                        McpSunucuDurumu::Durdu
                    } else {
                        McpSunucuDurumu::hata(McpHataNedeni::YalitimYok)
                    };
                    if let Some(cikarilan) = Self::yuvadan_sureci_ayir(
                        &mut d,
                        &anahtar,
                        YoneticiHatasi::KorumalarDegisti,
                        hedef_durum,
                    ) {
                        liste.push(cikarilan);
                    }
                }
            }
            liste
        };

        for (anahtar, kusak, kok_yolu) in durdurulacaklar {
            self.arka_planda_kapat(anahtar, kusak, kok_yolu);
        }
    }

    /// Sunucu tanımı düzenlendiğinde, kapatıldığında veya silindiğinde o
    /// sunucunun bütün çalışma alanlarındaki süreçlerini durdurur ve `stderr`
    /// halkasını temizler (§5.5, §7.1).
    pub fn sunucu_tanimi_degisti(self: &Arc<Self>, sunucu: &str) {
        let durdurulacaklar = {
            let mut d = self.durum();
            d.bekleyen_kartlar.retain(|_, kart| kart.sunucu != sunucu);
            let anahtarlar: Vec<McpAnahtari> = d
                .yuvalar
                .keys()
                .filter(|k| k.sunucu == sunucu)
                .cloned()
                .collect();
            let mut liste = Vec::new();
            for anahtar in anahtarlar {
                if let Some(yuva) = d.yuvalar.get_mut(&anahtar) {
                    if let Ok(mut halka) = yuva.stderr_halkasi.lock() {
                        halka.temizle();
                    }
                    yuva.cokme_sayaci = 0;
                    yuva.sonraki_cokme_izni_ani_ms = None;
                }
                if let Some(cikarilan) = Self::yuvadan_sureci_ayir(
                    &mut d,
                    &anahtar,
                    YoneticiHatasi::SunucuDurduruldu,
                    McpSunucuDurumu::Durdu,
                ) {
                    liste.push(cikarilan);
                }
            }
            liste
        };

        for (anahtar, kusak, kok_yolu) in durdurulacaklar {
            self.arka_planda_kapat(anahtar, kusak, kok_yolu);
        }
    }

    /// Belirtilen `(alan_kimligi, sunucu)` için başlatma iznini geri alır ve
    /// çalışan süreci hemen durdurur (§5.1, §9).
    pub fn baslatma_izni_geri_al(self: &Arc<Self>, alan_kimligi: &str, sunucu: &str) {
        let anahtar = McpAnahtari::yeni(alan_kimligi, sunucu);
        let cikarilan = {
            let mut d = self.durum();
            d.yalitimli_izinler.remove(&anahtar);
            d.gecici_tam_erisim_izinleri.remove(&anahtar);
            d.bekleyen_kartlar
                .retain(|_, k| !(k.alan_kimligi == alan_kimligi && k.sunucu == sunucu));
            self.izin_dosyasini_kaydet_kilitli(&d);
            Self::yuvadan_sureci_ayir(
                &mut d,
                &anahtar,
                YoneticiHatasi::SunucuDurduruldu,
                McpSunucuDurumu::Durdu,
            )
        };
        if let Some((anahtar, kusak, kok_yolu)) = cikarilan {
            self.arka_planda_kapat(anahtar, kusak, kok_yolu);
        }
    }

    /// `(alan_kimligi, sunucu)` için geçerli bir başlatma izni olup olmadığını
    /// denetler.
    pub fn baslatma_izni_gecerli_mi(
        &self,
        alan_kimligi: &str,
        sunucu: &str,
        kip: CalistirmaKipi,
        tanim_ozeti: &McpTanimOzeti,
        politika_ozeti: &McpPolitikaOzeti,
        kod_guveni: KodGuveni,
    ) -> bool {
        let d = self.durum();
        Self::izin_gecerli_kilitli(
            &d,
            alan_kimligi,
            sunucu,
            kip,
            tanim_ozeti,
            politika_ozeti,
            kod_guveni,
        )
    }

    fn izin_gecerli_kilitli(
        d: &YoneticiDurumu,
        alan_kimligi: &str,
        sunucu: &str,
        kip: CalistirmaKipi,
        tanim_ozeti: &McpTanimOzeti,
        politika_ozeti: &McpPolitikaOzeti,
        kod_guveni: KodGuveni,
    ) -> bool {
        let anahtar = McpAnahtari::yeni(alan_kimligi, sunucu);
        let guncel_w = d
            .alanlar
            .get(alan_kimligi)
            .map(|a| a.yazma_kusagi)
            .unwrap_or_default();

        match kip {
            CalistirmaKipi::Yalitimli => {
                let Some(izin) = d.yalitimli_izinler.get(&anahtar) else {
                    return false;
                };
                if izin.tanim_ozeti_hex != ozet_hex(&tanim_ozeti.sha256())
                    || izin.politika_ozeti_hex != ozet_hex(&politika_ozeti.sha256())
                {
                    return false;
                }
                if kod_guveni == KodGuveni::WdeKodlu {
                    izin.wde_kodlu && izin.onayli_yazma_kusagi == Some(guncel_w.deger())
                } else {
                    true
                }
            }
            CalistirmaKipi::TamErisim => {
                let Some(izin) = d.gecici_tam_erisim_izinleri.get(&anahtar) else {
                    return false;
                };
                if &izin.tanim_ozeti != tanim_ozeti {
                    return false;
                }
                if kod_guveni == KodGuveni::WdeKodlu {
                    izin.kod_guveni == KodGuveni::WdeKodlu
                        && izin.onayli_yazma_kusagi == Some(guncel_w)
                } else {
                    true
                }
            }
        }
    }

    /// Otomatik başlatma her turda yeniden denenir (Görev 9). Aynı koşullarla
    /// bekleyen kart varsa onun kimliği yeniden verilir; yoksa aynı (alan,
    /// sunucu) için eskimiş kartlar düşer ve yenisi eklenir. Kartlar birikmez.
    fn karti_bekletmeye_al(d: &mut YoneticiDurumu, kart: BekleyenBaslatmaKarti) -> String {
        if let Some((kimlik, _)) = d.bekleyen_kartlar.iter().find(|(_, k)| **k == kart) {
            return kimlik.clone();
        }
        d.bekleyen_kartlar
            .retain(|_, k| !(k.alan_kimligi == kart.alan_kimligi && k.sunucu == kart.sunucu));
        let kimlik = format!("mcp-baslat-{}", Uuid::new_v4());
        d.bekleyen_kartlar.insert(kimlik.clone(), kart);
        kimlik
    }

    /// Model dışı başlatma izni kartı oluşturur (§5.1).
    pub fn baslatma_karti_olustur(
        &self,
        alan_kimligi: &str,
        kok: &Path,
        sunucu: &str,
    ) -> Result<McpBaslatmaKarti, YoneticiHatasi> {
        let hazir = self.sunucuyu_hazirla(alan_kimligi, kok, sunucu)?;
        let politika_ozeti = self.guncel_politika_ozeti();
        let istek_kimligi = format!("mcp-baslat-{}", Uuid::new_v4());

        let mut d = self.durum();
        let alan = d.alanlar.entry(alan_kimligi.to_owned()).or_default();
        alan.kok = Some(kok.to_path_buf());
        let yazma_kusagi = alan.yazma_kusagi;

        d.bekleyen_kartlar.insert(
            istek_kimligi.clone(),
            BekleyenBaslatmaKarti {
                alan_kimligi: alan_kimligi.to_owned(),
                sunucu: sunucu.to_owned(),
                kip: hazir.kip,
                tanim_ozeti: hazir.tanim_ozeti,
                politika_ozeti,
                kod_guveni: hazir.kod_guveni,
                yazma_kusagi,
            },
        );

        Ok(Self::kart_gorunumunu_kaydet(
            &mut d,
            Self::baslatma_karti_dto(istek_kimligi, alan_kimligi, &hazir),
        ))
    }

    fn kart_gorunumunu_kaydet(d: &mut YoneticiDurumu, kart: McpBaslatmaKarti) -> McpBaslatmaKarti {
        let YoneticiDurumu {
            bekleyen_kartlar,
            kart_gorunumleri,
            ..
        } = d;
        kart_gorunumleri.retain(|kimlik, _| bekleyen_kartlar.contains_key(kimlik));
        kart_gorunumleri.insert(kart.istek_kimligi.clone(), kart.clone());
        kart
    }

    /// Yanıt bekleyen başlatma kartları (arayüzün sohbet ve Ayarlar görüntüsü).
    pub fn bekleyen_baslatma_kartlari(&self) -> Vec<McpBaslatmaKarti> {
        let d = self.durum();
        d.kart_gorunumleri
            .iter()
            .filter(|(kimlik, _)| d.bekleyen_kartlar.contains_key(*kimlik))
            .map(|(_, kart)| kart.clone())
            .collect()
    }

    /// Alan kaydındaki kök; arayüz yol göndermediği için komutlar kökü buradan alır.
    pub fn alan_koku(&self, alan_kimligi: &str) -> Option<PathBuf> {
        self.durum()
            .alanlar
            .get(alan_kimligi)
            .and_then(|a| a.kok.clone())
    }

    /// Sunucunun yuvası bulunan alanlar ve çalışma zamanı durumları.
    pub fn sunucu_alanlari(&self, sunucu: &str) -> Vec<(String, McpSunucuDurumu)> {
        self.durum()
            .yuvalar
            .iter()
            .filter(|(k, _)| k.sunucu == sunucu)
            .map(|(k, y)| (k.alan_kimligi.clone(), y.durum.clone()))
            .collect()
    }

    pub fn sistem_ortami(&self) -> Vec<(String, String)> {
        self.durum().sistem_ortami.clone()
    }

    /// Test düzeneği için: süreç ortamı yerine sabit ortam.
    pub(crate) fn sistem_ortamini_degistir(&self, ortam: Vec<(String, String)>) {
        self.durum().sistem_ortami = ortam;
    }

    pub fn yalitim_dizini(&self) -> &Path {
        &self.yalitim_dizini
    }

    fn baslatma_karti_dto(
        istek_kimligi: String,
        alan_kimligi: &str,
        hazir: &HazirlanmisSunucu,
    ) -> McpBaslatmaKarti {
        let erisim_kipi = match hazir.kip {
            CalistirmaKipi::Yalitimli => "Yalıtılmış süreç".to_owned(),
            CalistirmaKipi::TamErisim => "Tam sistem erişimi".to_owned(),
        };
        let uyari = match hazir.kip {
            CalistirmaKipi::Yalitimli => {
                "Sunucu başladığı andan kapanana kadar, araç çağrılmasa da çalışır; çalışma alanına yazabilir ve izinli ağa çıkabilir.".to_owned()
            }
            CalistirmaKipi::TamErisim => {
                "Sunucu başladığı andan kapanana kadar, araç çağrılmasa da çalışır. Hesabınızın erişebildiği her şeye erişir. Sunucu kendiliğinden kapansa da geride çalışan süreçler kalabilir.".to_owned()
            }
        };
        let ag_listesi = if hazir.kip == CalistirmaKipi::Yalitimli {
            hazir
                .cozulmus
                .ag_kurallari()
                .iter()
                .map(|k| format!("{}:{}", k.host(), k.port()))
                .collect()
        } else {
            Vec::new()
        };
        let komut_ozeti = hazir.cozulmus.maskeli_yerel_onay_metni(&hazir.sirlar);

        McpBaslatmaKarti {
            istek_kimligi,
            alan_kimligi: alan_kimligi.to_owned(),
            sunucu: hazir.ad.clone(),
            erisim_kipi,
            komut_ozeti,
            ag_listesi,
            uyari,
            wde_kodlu: hazir.kod_guveni == KodGuveni::WdeKodlu,
        }
    }

    /// Başlatma kartına verilen kullanıcı yanıtını işler (§5.1, Görev 9).
    ///
    /// - Tek kullanımlık `istek_kimligi` tüketilir; başka alanın veya eski
    ///   kuşağın yanıtı reddedilir.
    /// - `TamErisim` kipinde yerel onay kutusu (`Iletisim::onayla`) zorunludur;
    ///   izin yalnız bellekte saklanır.
    /// - `Yalitimli` kipinde izin `(T, Π, alan_kimligi)` (+ `WdeKodlu` ise `W`)
    ///   ile kalıcı kaydedilir.
    pub async fn baslatma_izni_yanitla(
        &self,
        istek_kimligi: &str,
        alan_kimligi: &str,
        kok: &Path,
        onaylandi: bool,
    ) -> Result<bool, YoneticiHatasi> {
        let kart = {
            let mut d = self.durum();
            let Some(mevcut) = d.bekleyen_kartlar.remove(istek_kimligi) else {
                return Err(YoneticiHatasi::GecersizBaslatmaKarti);
            };
            if mevcut.alan_kimligi != alan_kimligi {
                return Err(YoneticiHatasi::GecersizBaslatmaKarti);
            }
            mevcut
        };

        if !onaylandi {
            return Ok(false);
        }

        let hazir = self.sunucuyu_hazirla(alan_kimligi, kok, &kart.sunucu)?;
        let guncel_pi = self.guncel_politika_ozeti();
        if hazir.tanim_ozeti != kart.tanim_ozeti
            || guncel_pi != kart.politika_ozeti
            || hazir.kip != kart.kip
            || hazir.kod_guveni != kart.kod_guveni
        {
            return Err(YoneticiHatasi::GecersizBaslatmaKarti);
        }

        if kart.kip == CalistirmaKipi::TamErisim {
            let onay_metni = hazir.cozulmus.maskeli_yerel_onay_metni(&hazir.sirlar);
            if !self
                .iletisim
                .onayla(TAM_ERISIM_BASLATMA_BASLIGI, &onay_metni, "Başlat", "Vazgeç")
                .await
            {
                return Err(YoneticiHatasi::YerelOnayReddedildi);
            }
            // Yerel onay beklenirken T veya W değişmiş olabilir:
            let tekrar = self.sunucuyu_hazirla(alan_kimligi, kok, &kart.sunucu)?;
            if tekrar.tanim_ozeti != kart.tanim_ozeti || tekrar.kod_guveni != kart.kod_guveni {
                return Err(YoneticiHatasi::GecersizBaslatmaKarti);
            }
        }

        let mut d = self.durum();
        let guncel_pi_kilitli = self.guncel_politika_ozeti();
        if guncel_pi_kilitli != kart.politika_ozeti {
            return Err(YoneticiHatasi::GecersizBaslatmaKarti);
        }
        let guncel_w = d
            .alanlar
            .get(alan_kimligi)
            .map(|a| a.yazma_kusagi)
            .unwrap_or_default();
        if kart.kod_guveni == KodGuveni::WdeKodlu && guncel_w != kart.yazma_kusagi {
            return Err(YoneticiHatasi::GecersizBaslatmaKarti);
        }

        let anahtar = McpAnahtari::yeni(alan_kimligi, &kart.sunucu);
        match kart.kip {
            CalistirmaKipi::Yalitimli => {
                d.yalitimli_izinler.insert(
                    anahtar,
                    KaliciYalitimliIzin {
                        alan_kimligi: alan_kimligi.to_owned(),
                        sunucu: kart.sunucu,
                        tanim_ozeti_hex: ozet_hex(&kart.tanim_ozeti.sha256()),
                        politika_ozeti_hex: ozet_hex(&kart.politika_ozeti.sha256()),
                        wde_kodlu: kart.kod_guveni == KodGuveni::WdeKodlu,
                        onayli_yazma_kusagi: (kart.kod_guveni == KodGuveni::WdeKodlu)
                            .then_some(guncel_w.deger()),
                    },
                );
                self.izin_dosyasini_kaydet_kilitli(&d);
            }
            CalistirmaKipi::TamErisim => {
                d.gecici_tam_erisim_izinleri.insert(
                    anahtar,
                    GeciciTamErisimIzni {
                        tanim_ozeti: kart.tanim_ozeti,
                        kod_guveni: kart.kod_guveni,
                        onayli_yazma_kusagi: (kart.kod_guveni == KodGuveni::WdeKodlu)
                            .then_some(guncel_w),
                    },
                );
            }
        }

        Ok(true)
    }

    /// `(alan_kimligi, sunucu)` için sunucu durumunu döner.
    pub fn sunucu_durumu(&self, alan_kimligi: &str, sunucu: &str) -> McpSunucuDurumu {
        if self.cagri_defteri.alan_engelli(alan_kimligi) {
            return McpSunucuDurumu::hata(McpHataNedeni::Durdurulamadi {
                detay: "MCP çağrısının kapanışı veya son denetimi doğrulanamadı; çalışma alanı engelli.".into(),
            });
        }
        let d = self.durum();
        d.yuvalar
            .get(&McpAnahtari::yeni(alan_kimligi, sunucu))
            .map(|y| y.durum.clone())
            .unwrap_or(McpSunucuDurumu::Durdu)
    }

    /// `(alan_kimligi, sunucu)` için son kuşak sayacını (`N`) döner.
    pub fn son_kusak(&self, alan_kimligi: &str, sunucu: &str) -> McpBaslatmaSayaci {
        let d = self.durum();
        d.yuvalar
            .get(&McpAnahtari::yeni(alan_kimligi, sunucu))
            .map(|y| y.son_kusak)
            .unwrap_or_else(|| McpBaslatmaSayaci::yeni(0))
    }

    /// `(alan_kimligi, sunucu)` için maskelenmiş `stderr` satırlarını döner.
    pub fn stderr_satirlari(&self, alan_kimligi: &str, sunucu: &str) -> Vec<String> {
        let halka = {
            let d = self.durum();
            d.yuvalar
                .get(&McpAnahtari::yeni(alan_kimligi, sunucu))
                .map(|y| y.stderr_halkasi.clone())
        };
        let Some(halka) = halka else {
            return Vec::new();
        };
        let Ok(kilit) = halka.lock() else {
            return Vec::new();
        };
        kilit.satirlar().map(str::to_owned).collect()
    }

    /// Sunucuyu çalıştırır veya zaten güncel `R` ile çalışıyorsa doğrular.
    ///
    /// - Başlatma izni yoksa veya `W` kuşağı ilerlediği için geçersizleştiyse
    ///   `Err(YoneticiHatasi::BaslatmaIzniGerekli(kart))` döner; hiçbir süreç
    ///   başlatılmaz.
    /// - Çalışan sürecin `K`'sı değiştiyse eski süreç durdurulur, son denetim
    ///   yapılır ve izin geçerliyse yeni kuşak (`N + 1`) başlatılır.
    pub async fn baslat_veya_dogrula(
        self: &Arc<Self>,
        alan_kimligi: &str,
        kok: &Path,
        sunucu: &str,
    ) -> Result<(McpBaslatmaSayaci, McpTarifi), YoneticiHatasi> {
        self.ic_baslat_veya_dogrula(alan_kimligi, kok, sunucu, false)
            .await
    }

    /// Ayarlar'dan kullanıcının açık "Yeniden başlat" komutu (§7.4, §9):
    /// çökme sayacını sıfırlar, mevcut süreci durdurur ve izin geçerliyse
    /// yeni kuşak başlatır.
    pub async fn kullanici_yeniden_baslat(
        self: &Arc<Self>,
        alan_kimligi: &str,
        kok: &Path,
        sunucu: &str,
    ) -> Result<(McpBaslatmaSayaci, McpTarifi), YoneticiHatasi> {
        self.sunucuyu_durdur_ve_bekle(alan_kimligi, sunucu).await?;
        {
            let mut d = self.durum();
            if let Some(yuva) = d.yuvalar.get_mut(&McpAnahtari::yeni(alan_kimligi, sunucu)) {
                yuva.cokme_sayaci = 0;
                yuva.sonraki_cokme_izni_ani_ms = None;
            }
        }
        self.ic_baslat_veya_dogrula(alan_kimligi, kok, sunucu, true)
            .await
    }

    async fn ic_baslat_veya_dogrula(
        self: &Arc<Self>,
        alan_kimligi: &str,
        kok: &Path,
        sunucu: &str,
        kullanici_istegi: bool,
    ) -> Result<(McpBaslatmaSayaci, McpTarifi), YoneticiHatasi> {
        let anahtar = McpAnahtari::yeni(alan_kimligi, sunucu);
        if self.cagri_defteri.alan_engelli(alan_kimligi) {
            return Err(YoneticiHatasi::SunucuDurdurulamadi(
                "MCP çağrısının kapanışı veya son denetimi doğrulanamadı; çalışma alanı engelli."
                    .into(),
            ));
        }
        let baslatma_revizyonu = self
            .durum()
            .yuvalar
            .get(&anahtar)
            .map(|y| y.baslatma_revizyonu)
            .unwrap_or(0);

        // 1. Ön kontroller: alan açık mı, İnceleme dışı oturum var mı, durdurulamadı engeli var mı?
        {
            let mut d = self.durum();
            let alan = d.alanlar.entry(alan_kimligi.to_owned()).or_default();
            alan.kok = Some(kok.to_path_buf());
            if alan.kaldiriliyor {
                return Err(YoneticiHatasi::CalismaAlaniYok(alan_kimligi.to_owned()));
            }
            if !alan.inceleme_disi_oturum_var() {
                return Err(YoneticiHatasi::IncelemeDisiOturumYok);
            }
            if let Some(yuva) = d.yuvalar.get(&anahtar)
                && let Some(detay) = &yuva.durdurulamadi_hatasi
            {
                return Err(YoneticiHatasi::SunucuDurdurulamadi(detay.clone()));
            }
        }

        // 2. Kayıtlı sunucu tanımını çöz ve T özetini al:
        let hazir = match self.sunucuyu_hazirla(alan_kimligi, kok, sunucu) {
            Ok(h) => h,
            Err(hata) => {
                return Err(hata);
            }
        };
        if !hazir.etkin {
            return Err(YoneticiHatasi::SunucuDevreDisi(sunucu.to_owned()));
        }

        // 3. Yalıtım kipi uyumu (§4.1):
        let yalitim_hazir = matches!(self.yalitici.durum(), YalitimDurumu::Hazir { .. });
        if mcp_kip_uyusmasi(hazir.kip, yalitim_hazir).is_err() {
            let mut d = self.durum();
            let yuva = d.yuvalar.entry(anahtar).or_insert_with(SunucuYuvasi::yeni);
            yuva.durum = McpSunucuDurumu::hata(McpHataNedeni::YalitimYok);
            return Err(YoneticiHatasi::SunucuHatasi(
                McpHataNedeni::YalitimYok.kullanici_mesaji(),
            ));
        }

        // 4. Çalışan süreç varsa korumasını ve tarifini denetle:
        let mevcut_calisan = {
            let d = self.durum();
            d.yuvalar.get(&anahtar).and_then(|y| y.calisan.clone())
        };
        if let Some(calisan) = mevcut_calisan {
            if calisan.kesif_suruyor.load(Ordering::SeqCst) {
                return Err(YoneticiHatasi::BaslatmaYarisi);
            }
            let guncel_pi = self.guncel_politika_ozeti();
            if calisan.tanim_ozeti == hazir.tanim_ozeti && calisan.politika_ozeti == guncel_pi {
                match self
                    .sunucu_korumasini_denetle(alan_kimligi, sunucu, kok)
                    .await
                {
                    Ok(()) => {
                        let d = self.durum();
                        if let Some(yuva) = d.yuvalar.get(&anahtar)
                            && let Some(aktif) = &yuva.calisan
                            && aktif.kusak == calisan.kusak
                        {
                            return Ok((aktif.kusak, aktif.tarif));
                        }
                    }
                    Err(hata) => {
                        if matches!(
                            hata,
                            YoneticiHatasi::KorumalarDogrulanamadi(_)
                                | YoneticiHatasi::SunucuDurdurulamadi(_)
                        ) {
                            return Err(hata);
                        }
                        // Korumalar değiştiyse eski süreç durduruldu; aşağıda yeni kuşak başlatılır.
                    }
                }
            } else {
                self.sunucuyu_durdur_ve_bekle(alan_kimligi, sunucu).await?;
            }
        }

        // 5. Önceki kapanışların tamamlanmasını bekle:
        self.yuva_kapanislarini_bekle(&anahtar).await;

        // 6. Başlatmadan ÖNCE izin ve çökme sınırı ön kontrolü:
        //    İzin yoksa veya W ilerlediyse burada hemen BaslatmaIzniGerekli döner!
        let on_politika_ozeti = self.guncel_politika_ozeti();
        let baslangic_w = {
            let mut d = self.durum();
            let alan = d.alanlar.entry(alan_kimligi.to_owned()).or_default();
            if alan.kaldiriliyor {
                return Err(YoneticiHatasi::CalismaAlaniYok(alan_kimligi.to_owned()));
            }
            if !alan.inceleme_disi_oturum_var() {
                return Err(YoneticiHatasi::IncelemeDisiOturumYok);
            }
            let simdi_ms = self.saat.simdi_ms();
            let guncel_w = alan.yazma_kusagi;

            if let Some(yuva) = d.yuvalar.get(&anahtar) {
                if let Some(detay) = &yuva.durdurulamadi_hatasi {
                    return Err(YoneticiHatasi::SunucuDurdurulamadi(detay.clone()));
                }
                if !kullanici_istegi {
                    if yuva.cokme_sayaci > AZAMI_COKME_DENEMESI {
                        return Err(YoneticiHatasi::SunucuHatasi(
                            McpHataNedeni::CokmeSiniriAsildi {
                                deneme: AZAMI_COKME_DENEMESI,
                                son_hata: "azami yeniden deneme sınırı aşıldı".to_owned(),
                            }
                            .kullanici_mesaji(),
                        ));
                    }
                    if let Some(izin_ani) = yuva.sonraki_cokme_izni_ani_ms
                        && simdi_ms < izin_ani
                    {
                        return Err(YoneticiHatasi::CokmeGeriCekilmeBekleniyor {
                            kalan_ms: izin_ani - simdi_ms,
                        });
                    }
                }
            }

            if !Self::izin_gecerli_kilitli(
                &d,
                alan_kimligi,
                sunucu,
                hazir.kip,
                &hazir.tanim_ozeti,
                &on_politika_ozeti,
                hazir.kod_guveni,
            ) {
                let istek_kimligi = Self::karti_bekletmeye_al(
                    &mut d,
                    BekleyenBaslatmaKarti {
                        alan_kimligi: alan_kimligi.to_owned(),
                        sunucu: sunucu.to_owned(),
                        kip: hazir.kip,
                        tanim_ozeti: hazir.tanim_ozeti,
                        politika_ozeti: on_politika_ozeti,
                        kod_guveni: hazir.kod_guveni,
                        yazma_kusagi: guncel_w,
                    },
                );
                return Err(YoneticiHatasi::BaslatmaIzniGerekli(Box::new(
                    Self::kart_gorunumunu_kaydet(
                        &mut d,
                        Self::baslatma_karti_dto(istek_kimligi, alan_kimligi, &hazir),
                    ),
                )));
            }
            guncel_w
        };

        // 7. Koruma doğrulaması (K):
        //    Tarama doğrulanamazsa süreç başlatılmaz ve yuva Doğrulanamadı hatasına geçer.
        //    Tarama doğrulanırsa eski dogrulanamadi_hatasi temizlenir.
        let ag_ozeti = hazir.cozulmus.ag_kurallari().first().map(|_| {
            let kurallar: Vec<kalfa_sandbox::vekil::HostKurali> = hazir
                .cozulmus
                .ag_kurallari()
                .iter()
                .map(|k| kalfa_sandbox::vekil::HostKurali {
                    host: k.host().to_owned(),
                    port: k.port(),
                    ozel_ag: k.ozel_ag(),
                    kaynak: kalfa_sandbox::vekil::HostKaynagi::Kullanici,
                })
                .collect();
            kalfa_sandbox::vekil::IzinListesi::dogrudan(&kurallar).ozet()
        });

        let koruma_sonucu = self
            .koruma_denetleyicisi
            .dogrula(
                &self.yalitici,
                KorumaDenetimIstegi {
                    kok: kok.to_path_buf(),
                    gizli_ayarlari: self.ayarlar.gizli_ayarlari(),
                    okunur_dizinler: self.yalitim_ayarlari.dizinler(),
                    ag_ozeti: ag_ozeti.clone(),
                    yalitimli: hazir.kip == CalistirmaKipi::Yalitimli,
                    alan_hazirla: hazir.kip == CalistirmaKipi::Yalitimli,
                    onceki_ozet: None,
                    iptal: None,
                },
            )
            .await;

        let dogrulanmis_koruma: McpDogrulanmisKoruma = match koruma_sonucu {
            McpKorumaSonucu::Ayni(d) | McpKorumaSonucu::Degisti(d) => d,
            McpKorumaSonucu::Dogrulanamadi(hata) => {
                let detay = hata.kullanici_mesaji();
                let mut d = self.durum();
                let yuva = d.yuvalar.entry(anahtar).or_insert_with(SunucuYuvasi::yeni);
                yuva.dogrulanamadi_hatasi = Some(detay.clone());
                yuva.durum = McpSunucuDurumu::hata(McpHataNedeni::KorumaDogrulanamadi {
                    detay: detay.clone(),
                });
                return Err(YoneticiHatasi::KorumalarDogrulanamadi(detay));
            }
        };

        // 8. ATOMİK BAŞLATMA REZERVASYONU (Görev 5.6):
        //    W kuşağı, etkin tanım, İnceleme dışı oturum, Π ve başlatma izni
        //    aynı senkron kilit adımında yeniden doğrulanır; N artırılır ve R sabitlenir!
        let (kusak_kaydi, baslatma_zamani_asimi) = {
            let mut d = self.durum();
            if let Some(yuva) = d.yuvalar.get(&anahtar)
                && (yuva.calisan.is_some() || yuva.baslatma_revizyonu != baslatma_revizyonu)
            {
                return Err(YoneticiHatasi::BaslatmaYarisi);
            }
            let guncel_pi = self.guncel_politika_ozeti();
            if guncel_pi != on_politika_ozeti {
                return Err(YoneticiHatasi::BaslatmaYarisi);
            }
            let Some(alan) = d.alanlar.get(alan_kimligi) else {
                return Err(YoneticiHatasi::CalismaAlaniYok(alan_kimligi.to_owned()));
            };
            if alan.kaldiriliyor {
                return Err(YoneticiHatasi::CalismaAlaniYok(alan_kimligi.to_owned()));
            }
            if !alan.inceleme_disi_oturum_var() {
                return Err(YoneticiHatasi::IncelemeDisiOturumYok);
            }
            let guncel_w = alan.yazma_kusagi;
            if guncel_w != baslangic_w {
                // Koruma taraması sürerken başka bir yazma araya girdi!
                if hazir.kod_guveni == KodGuveni::WdeKodlu {
                    let istek_kimligi = Self::karti_bekletmeye_al(
                        &mut d,
                        BekleyenBaslatmaKarti {
                            alan_kimligi: alan_kimligi.to_owned(),
                            sunucu: sunucu.to_owned(),
                            kip: hazir.kip,
                            tanim_ozeti: hazir.tanim_ozeti,
                            politika_ozeti: guncel_pi,
                            kod_guveni: hazir.kod_guveni,
                            yazma_kusagi: guncel_w,
                        },
                    );
                    return Err(YoneticiHatasi::BaslatmaIzniGerekli(Box::new(
                        Self::kart_gorunumunu_kaydet(
                            &mut d,
                            Self::baslatma_karti_dto(istek_kimligi, alan_kimligi, &hazir),
                        ),
                    )));
                }
                return Err(YoneticiHatasi::BaslatmaYarisi);
            }
            if !Self::izin_gecerli_kilitli(
                &d,
                alan_kimligi,
                sunucu,
                hazir.kip,
                &hazir.tanim_ozeti,
                &guncel_pi,
                hazir.kod_guveni,
            ) {
                let istek_kimligi = Self::karti_bekletmeye_al(
                    &mut d,
                    BekleyenBaslatmaKarti {
                        alan_kimligi: alan_kimligi.to_owned(),
                        sunucu: sunucu.to_owned(),
                        kip: hazir.kip,
                        tanim_ozeti: hazir.tanim_ozeti,
                        politika_ozeti: guncel_pi,
                        kod_guveni: hazir.kod_guveni,
                        yazma_kusagi: guncel_w,
                    },
                );
                return Err(YoneticiHatasi::BaslatmaIzniGerekli(Box::new(
                    Self::kart_gorunumunu_kaydet(
                        &mut d,
                        Self::baslatma_karti_dto(istek_kimligi, alan_kimligi, &hazir),
                    ),
                )));
            }

            let yalitici_profili = yalitim_profili_ozeti_hesapla(
                self.yalitici.as_ref(),
                hazir.kip,
                &dogrulanmis_koruma.alan_bilgisi,
                ag_ozeti.as_deref(),
            );

            let yuva = d
                .yuvalar
                .entry(anahtar.clone())
                .or_insert_with(SunucuYuvasi::yeni);
            if let Some(detay) = &yuva.durdurulamadi_hatasi {
                return Err(YoneticiHatasi::SunucuDurdurulamadi(detay.clone()));
            }
            yuva.dogrulanamadi_hatasi = None;
            yuva.son_kusak = yuva.son_kusak.ileri();
            let kusak = yuva.son_kusak;
            let tarif = McpTarifi::yeni(
                hazir.tanim_ozeti,
                guncel_pi,
                dogrulanmis_koruma.ozet,
                yalitici_profili,
                kusak,
            );

            if let Ok(mut halka) = yuva.stderr_halkasi.lock() {
                halka.sirlari_guncelle(hazir.sirlar.clone());
            }
            // Son W kontrolü ve gerçek OS süreç oluşturma aynı seri adımdadır.
            // Dönen uç el sıkışması beklerken de süreç ağacının sahibidir.
            let istek = hazir
                .cozulmus
                .etkilesimli_istek(dogrulanmis_koruma.alan_bilgisi)
                .map_err(|h| YoneticiHatasi::SunucuHatasi(h.to_string()))?;
            let ucu = match self.surec_baslatici.baslat(
                &self.yalitici,
                istek,
                yuva.stderr_halkasi.clone(),
            ) {
                Ok(ucu) => ucu,
                Err(neden) => {
                    yuva.durum = McpSunucuDurumu::hata(neden.clone());
                    return Err(YoneticiHatasi::SunucuHatasi(neden.kullanici_mesaji()));
                }
            };
            let simdi_ms = self.saat.simdi_ms();
            let calisan = Arc::new(CalisanKusak {
                kusak,
                tarif,
                tanim_ozeti: hazir.tanim_ozeti,
                politika_ozeti: guncel_pi,
                koruma_ozeti: dogrulanmis_koruma.ozet,
                kod_guveni: hazir.kod_guveni,
                kip: hazir.kip,
                ag_ozeti,
                sirlar: hazir.sirlar.clone(),
                t_k_ani_ms: AtomicU64::new(simdi_ms),
                son_cagri_ani_ms: AtomicU64::new(simdi_ms),
                etkin_cagri_sayisi: AtomicUsize::new(0),
                kesif_suruyor: AtomicBool::new(true),
                onay_bekliyor: AtomicBool::new(false),
                zamanlayici_bildirimi: Notify::new(),
                koruma_kilidi: tokio::sync::Mutex::new(()),
                iptal: CancellationToken::new(),
                iptal_nedeni: Mutex::new(None),
                adli_girdiler: Mutex::new(dogrulanmis_koruma.adli_girdiler),
                ucu,
            });
            yuva.durum = McpSunucuDurumu::Basliyor { kusak, tarif };
            yuva.calisan = Some(Arc::clone(&calisan));

            (calisan, hazir.cozulmus.baslatma_zamani_asimi())
        };

        let mut temizlik = BaslatmaTemizligi {
            yonetici: self.clone(),
            anahtar: anahtar.clone(),
            kusak: kusak_kaydi.kusak,
            tamamlandi: false,
        };
        self.kusak_zamanlayicisi_baslat(anahtar.clone(), kusak_kaydi.clone(), kok.to_path_buf());
        let hazirlik = tokio::select! {
            biased;
            _ = kusak_kaydi.iptal.cancelled() => Err(kusak_kaydi.iptal_hatasi()),
            sonuc = async {
                kusak_kaydi.ucu.hazirlan(baslatma_zamani_asimi).await.map_err(|neden| YoneticiHatasi::SunucuHatasi(neden.kullanici_mesaji()))
            } => sonuc,
        };
        if let Err(hata) = hazirlik {
            // Yalnız bu başlatmanın kuşağını ayırır; geciken eski hata yeni
            // kuşağın sürecini durduramaz.
            drop(temizlik);
            self.yuva_kapanislarini_bekle(&anahtar).await;
            return Err(hata);
        }
        {
            let mut d = self.durum();
            let yuva = d
                .yuvalar
                .get_mut(&anahtar)
                .ok_or(YoneticiHatasi::SunucuDurduruldu)?;
            if !yuva
                .calisan
                .as_ref()
                .is_some_and(|c| c.kusak == kusak_kaydi.kusak)
            {
                return Err(kusak_kaydi.iptal_hatasi());
            }
            yuva.durum = McpSunucuDurumu::Calisiyor {
                kusak: kusak_kaydi.kusak,
                tarif: kusak_kaydi.tarif,
            };
            yuva.sonraki_cokme_izni_ani_ms = None;
            kusak_kaydi.kesif_suruyor.store(false, Ordering::SeqCst);
            kusak_kaydi.zamanlayici_bildirimi.notify_one();
        }
        temizlik.tamamlandi = true;

        // Arka planda sürecin kendiliğinden ölmesini (çökme) izle:
        self.cikis_izleyicisi_baslat(anahtar, Arc::clone(&kusak_kaydi), kok.to_path_buf());

        Ok((kusak_kaydi.kusak, kusak_kaydi.tarif))
    }

    fn cikis_izleyicisi_baslat(
        self: &Arc<Self>,
        anahtar: McpAnahtari,
        kusak: Arc<CalisanKusak>,
        kok: PathBuf,
    ) {
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let yonetici = Arc::clone(self);
        handle.spawn(async move {
            // Ucu kilitlemeden yalnız iptal veya süreç çıkışını gözlemle:
            loop {
                tokio::select! {
                    biased;
                    _ = kusak.iptal.cancelled() => break,
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {
                        let cikis = {
                            tokio::time::timeout(Duration::from_millis(5), kusak.ucu.bitisi_bekle()).await
                        };
                        if let Ok(_kod) = cikis {
                            if !kusak.iptal.is_cancelled() {
                                yonetici.beklenmeyen_cikis_bildir(&anahtar.alan_kimligi, &anahtar.sunucu, kusak.kusak, &kok);
                            }
                            break;
                        }
                    }
                }
            }
        });
    }

    /// Sürecin kendiliğinden öldüğünü (çökme) bildirir (§7.4, Görev 5.7).
    ///
    /// - Çökme sayacını artırır (`1 sn`, `4 sn`, `16 sn` geri çekilme).
    /// - 3 yeniden başlatma denemesi de çökerse sunucu `Hata(CokmeSiniriAsildi)`
    ///   durumuna geçer.
    pub fn beklenmeyen_cikis_bildir(
        self: &Arc<Self>,
        alan_kimligi: &str,
        sunucu: &str,
        beklenen_kusak: McpBaslatmaSayaci,
        kok: &Path,
    ) {
        let anahtar = McpAnahtari::yeni(alan_kimligi, sunucu);
        let cikarilan = {
            let mut d = self.durum();
            let Some(yuva) = d.yuvalar.get_mut(&anahtar) else {
                return;
            };
            let Some(calisan) = &yuva.calisan else {
                return;
            };
            if calisan.kusak != beklenen_kusak || calisan.iptal.is_cancelled() {
                return;
            }

            let simdi_ms = self.saat.simdi_ms();
            let indeks = usize::from(yuva.cokme_sayaci);
            let yeni_durum = if let Some(bekleme) = COKME_GERI_CEKILME_SURELERI.get(indeks) {
                yuva.cokme_sayaci = yuva.cokme_sayaci.saturating_add(1);
                let ek_ms = u64::try_from(bekleme.as_millis()).unwrap_or(1_000);
                yuva.sonraki_cokme_izni_ani_ms = Some(simdi_ms.saturating_add(ek_ms));
                McpSunucuDurumu::Durdu
            } else {
                let deneme = yuva.cokme_sayaci;
                yuva.cokme_sayaci = yuva.cokme_sayaci.saturating_add(1);
                yuva.sonraki_cokme_izni_ani_ms = None;
                McpSunucuDurumu::hata(McpHataNedeni::CokmeSiniriAsildi {
                    deneme,
                    son_hata: "süreç beklenmedik biçimde sonlandı".to_owned(),
                })
            };

            Self::yuvadan_sureci_ayir(&mut d, &anahtar, YoneticiHatasi::SunucuCoktu, yeni_durum)
        };

        if let Some((anahtar, kusak, kok_yolu)) = cikarilan {
            let kok_yolu = kok_yolu.or_else(|| Some(kok.to_path_buf()));
            let tekrar = {
                let d = self.durum();
                d.yuvalar.get(&anahtar).and_then(|y| {
                    y.sonraki_cokme_izni_ani_ms
                        .map(|an| (an, y.baslatma_revizyonu))
                })
            };
            self.arka_planda_kapat(anahtar.clone(), kusak, kok_yolu);
            if let Some((an, revizyon)) = tekrar {
                self.yeniden_baslatmayi_zamanla(
                    anahtar,
                    beklenen_kusak,
                    revizyon,
                    an,
                    kok.to_path_buf(),
                );
            }
        }
    }

    /// Çalışan kuşağın keşif (`Basliyor`) veya onay bekleme durumunu işaretler
    /// (testler ve Görev 6/8 için).
    pub fn onay_bekliyor_ayarla(&self, alan_kimligi: &str, sunucu: &str, bekliyor: bool) {
        let d = self.durum();
        if let Some(yuva) = d.yuvalar.get(&McpAnahtari::yeni(alan_kimligi, sunucu))
            && let Some(calisan) = &yuva.calisan
        {
            calisan.onay_bekliyor.store(bekliyor, Ordering::SeqCst);
            calisan.zamanlayici_bildirimi.notify_one();
        }
    }

    /// Çalışan süreçte `tools/list` çağrısı yapar.
    pub async fn araclari_listele(
        self: &Arc<Self>,
        alan_kimligi: &str,
        sunucu: &str,
        beklenen_kusak: McpBaslatmaSayaci,
    ) -> Result<Vec<Tool>, YoneticiHatasi> {
        let calisan = self.aktif_kusak_bul(alan_kimligi, sunucu, beklenen_kusak)?;
        let _etkinlik = EtkinlikKaydi::yeni(calisan.clone(), self.saat.clone(), false);
        let sonuc = tokio::select! {
            biased;
            _ = calisan.iptal.cancelled() => {
                Err(calisan.iptal_hatasi())
            }
            res = async {
                calisan.ucu.araclari_listele().await.map_err(|h| YoneticiHatasi::SunucuHatasi(h.to_string()))
            } => res
        };
        sonuc
    }

    /// Çalışan süreçte `tools/list` çağrısı yapıp ham listeyi aynı adımda G7
    /// sır ve biçim kapılarından geçirir.
    pub async fn denetlenmis_araclari_listele(
        self: &Arc<Self>,
        alan_kimligi: &str,
        sunucu: &str,
        beklenen_kusak: McpBaslatmaSayaci,
    ) -> Result<kalfa_mcp::kesif::McpG7KesifSonucu, YoneticiHatasi> {
        let calisan = self.aktif_kusak_bul(alan_kimligi, sunucu, beklenen_kusak)?;
        let ham = self
            .araclari_listele(alan_kimligi, sunucu, beklenen_kusak)
            .await?;
        Ok(kalfa_mcp::kesif::arac_tanimlarini_denetle(
            sunucu,
            ham,
            &calisan.sirlar,
        ))
    }

    /// Çalışan kuşağın etkin sırlarını döner.
    pub fn aktif_kusagin_sirlari(
        &self,
        alan_kimligi: &str,
        sunucu: &str,
        beklenen_kusak: McpBaslatmaSayaci,
    ) -> Result<Vec<kalfa_mcp::maske::GizliDeger>, YoneticiHatasi> {
        let calisan = self.aktif_kusak_bul(alan_kimligi, sunucu, beklenen_kusak)?;
        Ok(calisan.sirlar.clone())
    }

    /// Çalışan kuşağın araç listesi yenileme durumunu döner.
    pub fn aktif_kusagin_yenileme_durumu(
        &self,
        alan_kimligi: &str,
        sunucu: &str,
        beklenen_kusak: McpBaslatmaSayaci,
    ) -> Option<Arc<kalfa_mcp::tasima::YenilemeDurumu>> {
        self.aktif_kusak_bul(alan_kimligi, sunucu, beklenen_kusak)
            .ok()
            .and_then(|calisan| calisan.ucu.yenileme_durumu())
    }

    /// Çalışan süreçte `tools/call` çağrısı yapar (§5.1.1, §7.3).
    ///
    /// - Çağrıdan önce çalışma alanı koruma özetini (`K`) doğrular.
    /// - Yan etki başlayacağı sınırda çalışma alanının yazma kuşağını (`W`) artırır.
    /// - Çağrı sırasında `t_K` mutlak yaş denetimi veya kontrol kanalı süreci
    ///   durdurursa çağrı hemen ilgili hatayla (`KorumalarDegisti`,
    ///   `KorumalarDogrulanamadi`, `SunucuDurduruldu`) kesilir.
    pub async fn arac_cagir(
        self: &Arc<Self>,
        alan_kimligi: &str,
        kok: &Path,
        sunucu: &str,
        beklenen_tarif: &McpTarifi,
        arac_adi: &str,
        argumanlar: JsonObject,
    ) -> Result<CallToolResult, YoneticiHatasi> {
        self.arac_cagir_cevabi(
            alan_kimligi,
            kok,
            sunucu,
            beklenen_tarif,
            arac_adi,
            argumanlar,
        )
        .await
        .map(|cevap| cevap.sonuc)
    }

    /// Çalışan süreçte `tools/call` çağrısı yapar ve taşımanın ham `content`
    /// alanı varlık bilgisiyle birlikte cevabı döner. Uzak JSON-RPC protokol
    /// hataları `YoneticiHatasi::UzakRpc` olarak korunur.
    pub async fn arac_cagir_cevabi(
        self: &Arc<Self>,
        alan_kimligi: &str,
        kok: &Path,
        sunucu: &str,
        beklenen_tarif: &McpTarifi,
        arac_adi: &str,
        argumanlar: JsonObject,
    ) -> Result<kalfa_mcp::istemci::McpAracCevabi, YoneticiHatasi> {
        let _ = self.aktif_tarifli_kusak_bul(alan_kimligi, sunucu, beklenen_tarif)?;
        self.sunucu_korumasini_denetle(alan_kimligi, sunucu, kok)
            .await?;
        let calisan = self.aktif_tarifli_kusak_bul(alan_kimligi, sunucu, beklenen_tarif)?;
        // Her MCP çağrısı yan etki sınırında W yazma kuşağını artırır (§5.1.1):
        self.yazma_kusagini_artir(alan_kimligi);

        let _etkinlik = EtkinlikKaydi::yeni(calisan.clone(), self.saat.clone(), true);

        let ad = arac_adi.to_owned();
        let sonuc = tokio::select! {
            biased;
            _ = calisan.iptal.cancelled() => {
                Err(calisan.iptal_hatasi())
            }
            res = async {
                calisan.ucu.arac_cagir_cevabi(ad, argumanlar).await
            } => {
                match res {
                    Ok(cikti) => Ok(cikti),
                    Err(McpHatasi::UzakRpc(rpc)) => Err(YoneticiHatasi::UzakRpc(rpc)),
                    Err(McpHatasi::Baglanti | McpHatasi::Kapali) => {
                        if !calisan.iptal.is_cancelled() {
                            self.beklenmeyen_cikis_bildir(alan_kimligi, sunucu, calisan.kusak, kok);
                            Err(YoneticiHatasi::SunucuCoktu)
                        } else {
                            Err(calisan.iptal_hatasi())
                        }
                    }
                    Err(hata) => Err(YoneticiHatasi::SunucuHatasi(hata.to_string())),
                }
            }
        };

        sonuc
    }

    fn aktif_kusak_bul(
        &self,
        alan_kimligi: &str,
        sunucu: &str,
        beklenen_kusak: McpBaslatmaSayaci,
    ) -> Result<Arc<CalisanKusak>, YoneticiHatasi> {
        let d = self.durum();
        let Some(yuva) = d.yuvalar.get(&McpAnahtari::yeni(alan_kimligi, sunucu)) else {
            return Err(YoneticiHatasi::SunucuDurduruldu);
        };
        let Some(calisan) = &yuva.calisan else {
            return Err(YoneticiHatasi::SunucuDurduruldu);
        };
        if calisan.kusak != beklenen_kusak || calisan.iptal.is_cancelled() {
            return Err(calisan.iptal_hatasi());
        }
        Ok(Arc::clone(calisan))
    }

    fn aktif_tarifli_kusak_bul(
        &self,
        alan_kimligi: &str,
        sunucu: &str,
        beklenen_tarif: &McpTarifi,
    ) -> Result<Arc<CalisanKusak>, YoneticiHatasi> {
        let d = self.durum();
        let Some(yuva) = d.yuvalar.get(&McpAnahtari::yeni(alan_kimligi, sunucu)) else {
            return Err(YoneticiHatasi::SunucuDurduruldu);
        };
        let Some(calisan) = &yuva.calisan else {
            return Err(YoneticiHatasi::SunucuDurduruldu);
        };
        if &calisan.tarif != beklenen_tarif || calisan.iptal.is_cancelled() {
            return Err(calisan.iptal_hatasi());
        }
        Ok(Arc::clone(calisan))
    }

    /// Belirtilen sunucunun koruma özetini (`K`) kilit almadan doğrular (§4.3).
    ///
    /// - `Aynı`: yalnız bu durumda `t_K` yenilenir.
    /// - `Değişti`: süreç ağacı hemen durdurulur, etkin çağrılar
    ///   `"korumalar değişti, sunucu durduruldu"` hatasıyla biter.
    /// - `Doğrulanamadı`: `t_K` yenilenmez, süreç durdurulur, başarılı bir
    ///   tarama gelene kadar yeniden başlatma engellenir.
    pub async fn sunucu_korumasini_denetle(
        self: &Arc<Self>,
        alan_kimligi: &str,
        sunucu: &str,
        kok: &Path,
    ) -> Result<(), YoneticiHatasi> {
        let anahtar = McpAnahtari::yeni(alan_kimligi, sunucu);
        let calisan = {
            let d = self.durum();
            d.yuvalar.get(&anahtar).and_then(|y| y.calisan.clone())
        };
        let Some(calisan) = calisan else {
            return Ok(());
        };

        // Aynı kuşaktaki eşzamanlı taramalar yaş bilgisini geriye taşıyamaz.
        // Etkin taramanın sonucu gelmeden çağrının hazırlığı başarılı sayılmaz.
        let Ok(_koruma) = calisan.koruma_kilidi.try_lock() else {
            return Err(YoneticiHatasi::BaslatmaYarisi);
        };
        let _bildirim = KorumaBildirimi(&calisan.zamanlayici_bildirimi);

        let sonuc = self
            .koruma_denetleyicisi
            .dogrula(
                &self.yalitici,
                KorumaDenetimIstegi {
                    kok: kok.to_path_buf(),
                    gizli_ayarlari: self.ayarlar.gizli_ayarlari(),
                    okunur_dizinler: self.yalitim_ayarlari.dizinler(),
                    ag_ozeti: calisan.ag_ozeti.clone(),
                    yalitimli: calisan.kip == CalistirmaKipi::Yalitimli,
                    alan_hazirla: calisan.kip == CalistirmaKipi::Yalitimli,
                    onceki_ozet: Some(calisan.koruma_ozeti),
                    iptal: None,
                },
            )
            .await;

        match sonuc {
            McpKorumaSonucu::Ayni(dogrulanmis) => {
                let mut d = self.durum();
                let Some(yuva) = d.yuvalar.get_mut(&anahtar) else {
                    return Err(calisan.iptal_hatasi());
                };
                if !yuva
                    .calisan
                    .as_ref()
                    .is_some_and(|c| c.kusak == calisan.kusak)
                {
                    return Err(calisan.iptal_hatasi());
                }
                let simdi_ms = self.saat.simdi_ms();
                calisan.t_k_ani_ms.store(simdi_ms, Ordering::SeqCst);
                calisan.zamanlayici_bildirimi.notify_one();
                if let Ok(mut adli) = calisan.adli_girdiler.lock() {
                    *adli = dogrulanmis.adli_girdiler;
                }
                yuva.dogrulanamadi_hatasi = None;
                Ok(())
            }
            McpKorumaSonucu::Degisti(_) => {
                let cikarilan = {
                    let mut d = self.durum();
                    if d.yuvalar
                        .get(&anahtar)
                        .and_then(|y| y.calisan.as_ref())
                        .is_some_and(|c| c.kusak == calisan.kusak)
                    {
                        Self::yuvadan_sureci_ayir(
                            &mut d,
                            &anahtar,
                            YoneticiHatasi::KorumalarDegisti,
                            McpSunucuDurumu::Durdu,
                        )
                    } else {
                        None
                    }
                };
                if let Some((anahtar, kusak, kok_yolu)) = cikarilan {
                    let kok_yolu = kok_yolu.or_else(|| Some(kok.to_path_buf()));
                    self.kapat_ve_denetle(anahtar, kusak, kok_yolu).await?;
                }
                Err(YoneticiHatasi::KorumalarDegisti)
            }
            McpKorumaSonucu::Dogrulanamadi(hata) => {
                // t_K YENİLENMEZ! Süreç hemen durdurulur ve başarılı taramaya kadar bloklanır.
                let detay = hata.kullanici_mesaji();
                let cikarilan = {
                    let mut d = self.durum();
                    if let Some(yuva) = d.yuvalar.get_mut(&anahtar)
                        && yuva
                            .calisan
                            .as_ref()
                            .is_some_and(|c| c.kusak == calisan.kusak)
                    {
                        yuva.dogrulanamadi_hatasi = Some(detay.clone());
                    }
                    if d.yuvalar
                        .get(&anahtar)
                        .and_then(|y| y.calisan.as_ref())
                        .is_some_and(|c| c.kusak == calisan.kusak)
                    {
                        Self::yuvadan_sureci_ayir(
                            &mut d,
                            &anahtar,
                            YoneticiHatasi::KorumalarDogrulanamadi(detay.clone()),
                            McpSunucuDurumu::hata(McpHataNedeni::KorumaDogrulanamadi {
                                detay: detay.clone(),
                            }),
                        )
                    } else {
                        None
                    }
                };
                if let Some((anahtar, kusak, kok_yolu)) = cikarilan {
                    let kok_yolu = kok_yolu.or_else(|| Some(kok.to_path_buf()));
                    self.kapat_ve_denetle(anahtar, kusak, kok_yolu).await?;
                }
                Err(YoneticiHatasi::KorumalarDogrulanamadi(detay))
            }
        }
    }

    /// Zamanlayıcı adımı (§4.3, Görev 5.4):
    /// - Son çağrıdan itibaren 10 dk (`BOSTA_KAPANMA_SURESI`) geçen ve etkin
    ///   çağrı, keşif veya onay beklemeyen boşta süreçleri kapatıp `BostaKapandi`
    ///   durumuna geçirir.
    /// - `t_K`'dan itibaren 10 dk (`MUTLAK_KORUMA_YASI`) geçen bütün çalışan/başlayan
    ///   süreçler için (keşif, onay bekleme ve uzun çağrı sürerken dahi) koruma
    ///   denetimi çalıştırır.
    pub async fn zamanlayici_adimi(self: &Arc<Self>) {
        let simdi_ms = self.saat.simdi_ms();
        let mutlak_yas_ms = u64::try_from(MUTLAK_KORUMA_YASI.as_millis()).unwrap_or(600_000);
        let bosta_sure_ms = u64::try_from(BOSTA_KAPANMA_SURESI.as_millis()).unwrap_or(600_000);

        let mut koruma_kontrol_listesi = Vec::new();
        let mut bosta_kapatilacaklar = Vec::new();

        {
            let mut d = self.durum();
            let anahtarlar: Vec<McpAnahtari> = d.yuvalar.keys().cloned().collect();
            for anahtar in anahtarlar {
                let Some(calisan) = d.yuvalar.get(&anahtar).and_then(|y| y.calisan.clone()) else {
                    continue;
                };
                let etkin_cagri = calisan.etkin_cagri_sayisi.load(Ordering::SeqCst);
                let kesif = calisan.kesif_suruyor.load(Ordering::SeqCst);
                let onay = calisan.onay_bekliyor.load(Ordering::SeqCst);
                let son_cagri_ms = calisan.son_cagri_ani_ms.load(Ordering::SeqCst);
                if etkin_cagri == 0
                    && !kesif
                    && !onay
                    && simdi_ms.saturating_sub(son_cagri_ms) >= bosta_sure_ms
                    && let Some(cikarilan) = Self::yuvadan_sureci_ayir(
                        &mut d,
                        &anahtar,
                        YoneticiHatasi::SunucuDurduruldu,
                        McpSunucuDurumu::BostaKapandi,
                    )
                {
                    bosta_kapatilacaklar.push(cikarilan);
                    continue;
                }

                let kok = d
                    .alanlar
                    .get(&anahtar.alan_kimligi)
                    .and_then(|a| a.kok.clone());
                let t_k_ms = calisan.t_k_ani_ms.load(Ordering::SeqCst);
                if simdi_ms.saturating_sub(t_k_ms) >= mutlak_yas_ms
                    && let Some(kok_yolu) = kok
                {
                    koruma_kontrol_listesi.push((anahtar.clone(), kok_yolu));
                }
            }
        }

        let mut gorevler = tokio::task::JoinSet::new();
        for (anahtar, kusak, kok_yolu) in bosta_kapatilacaklar {
            let yonetici = self.clone();
            gorevler.spawn(async move {
                let _ = yonetici.kapat_ve_denetle(anahtar, kusak, kok_yolu).await;
            });
        }

        for (anahtar, kok_yolu) in koruma_kontrol_listesi {
            let yonetici = self.clone();
            gorevler.spawn(async move {
                let calisan = yonetici
                    .durum()
                    .yuvalar
                    .get(&anahtar)
                    .and_then(|y| y.calisan.clone());
                let sonuc = yonetici
                    .sunucu_korumasini_denetle(&anahtar.alan_kimligi, &anahtar.sunucu, &kok_yolu)
                    .await;
                // Elle beklenen adım, aynı kuşakta daha önce başlamış olan
                // zamanlayıcı taramasının bitişini de kapsar.
                if matches!(sonuc, Err(YoneticiHatasi::BaslatmaYarisi))
                    && let Some(calisan) = calisan
                {
                    let _tarama = calisan.koruma_kilidi.lock().await;
                }
            });
        }
        while gorevler.join_next().await.is_some() {}
    }

    /// Belirtilen `(alan_kimligi, sunucu)` sürecini hemen durdurmaya başlar
    /// (senkron, beklemeyen kontrol kanalı; Görev 5.1).
    pub fn sunucuyu_durdur(self: &Arc<Self>, alan_kimligi: &str, sunucu: &str) {
        let anahtar = McpAnahtari::yeni(alan_kimligi, sunucu);
        let cikarilan = {
            let mut d = self.durum();
            Self::yuvadan_sureci_ayir(
                &mut d,
                &anahtar,
                YoneticiHatasi::SunucuDurduruldu,
                McpSunucuDurumu::Durdu,
            )
        };
        if let Some((anahtar, kusak, kok_yolu)) = cikarilan {
            self.arka_planda_kapat(anahtar, kusak, kok_yolu);
        }
    }

    /// Belirtilen `(alan_kimligi, sunucu)` sürecini hemen durdurmaya başlar ve
    /// kapanışın doğrulanmasını bekler.
    pub async fn sunucuyu_durdur_ve_bekle(
        self: &Arc<Self>,
        alan_kimligi: &str,
        sunucu: &str,
    ) -> Result<(), YoneticiHatasi> {
        let anahtar = McpAnahtari::yeni(alan_kimligi, sunucu);
        let cikarilan = {
            let mut d = self.durum();
            Self::yuvadan_sureci_ayir(
                &mut d,
                &anahtar,
                YoneticiHatasi::SunucuDurduruldu,
                McpSunucuDurumu::Durdu,
            )
        };
        if let Some((anahtar, kusak, kok_yolu)) = cikarilan {
            self.kapat_ve_denetle(anahtar.clone(), kusak, kok_yolu)
                .await?;
        }
        self.yuva_kapanislarini_bekle(&anahtar).await;
        let d = self.durum();
        if let Some(yuva) = d.yuvalar.get(&anahtar)
            && let Some(detay) = &yuva.durdurulamadi_hatasi
        {
            return Err(YoneticiHatasi::SunucuDurdurulamadi(detay.clone()));
        }
        Ok(())
    }

    /// Sunucunun özel kökünü (`sunucu_kokunu_kaldir`) yalnız süreç kapanışı
    /// doğrulandıktan sonra siler (§5.6, Görev 5.8).
    ///
    /// Ortak Windows çalışma alanı profiline veya paket SID ACE'lerine dokunmaz.
    pub async fn sunucu_kokunu_temizle(
        self: &Arc<Self>,
        kok: &Path,
        alan_kimligi: &str,
        sunucu: &str,
    ) -> Result<(), YoneticiHatasi> {
        self.sunucuyu_durdur_ve_bekle(alan_kimligi, sunucu).await?;
        self.yalitici
            .sunucu_kokunu_kaldir(kok, sunucu)
            .map_err(|h| YoneticiHatasi::SunucuHatasi(h.to_string()))
    }

    /// Çalışma alanı kaldırılırken yeni MCP başlatmalarını engeller, alandaki
    /// tüm MCP süreçlerini durdurur ve doğrulanmış kapanışlarını bekler (§5.6, Görev 5.8).
    pub async fn alan_kapanisini_baslat_ve_bekle(
        self: &Arc<Self>,
        alan_kimligi: &str,
    ) -> Result<(), YoneticiHatasi> {
        let (durdurulacaklar, anahtarlar) = {
            let mut d = self.durum();
            let alan = d.alanlar.entry(alan_kimligi.to_owned()).or_default();
            alan.kaldiriliyor = true;
            d.bekleyen_kartlar
                .retain(|_, k| k.alan_kimligi != alan_kimligi);
            let durdurulacaklar = Self::alandaki_tum_surecleri_ayir(
                &mut d,
                alan_kimligi,
                YoneticiHatasi::SunucuDurduruldu,
                McpSunucuDurumu::Durdu,
            );
            let anahtarlar: Vec<McpAnahtari> = d
                .yuvalar
                .keys()
                .filter(|k| k.alan_kimligi == alan_kimligi)
                .cloned()
                .collect();
            (durdurulacaklar, anahtarlar)
        };

        for (anahtar, kusak, kok_yolu) in durdurulacaklar {
            self.kapat_ve_denetle(anahtar, kusak, kok_yolu).await?;
        }
        for anahtar in &anahtarlar {
            self.yuva_kapanislarini_bekle(anahtar).await;
            let d = self.durum();
            if let Some(yuva) = d.yuvalar.get(anahtar)
                && let Some(detay) = &yuva.durdurulamadi_hatasi
            {
                return Err(YoneticiHatasi::SunucuDurdurulamadi(detay.clone()));
            }
        }
        Ok(())
    }

    /// Çalışma alanı kaldırma rezervasyonunu serbest bırakır.
    pub fn alan_rezervini_kaldir(&self, alan_kimligi: &str) {
        let mut d = self.durum();
        if let Some(alan) = d.alanlar.get_mut(alan_kimligi) {
            alan.kaldiriliyor = false;
        }
    }

    /// Uygulama çıkarken bütün çalışma alanlarındaki MCP süreçlerini kapatır ve
    /// doğrulanmış kapanışlarını bekler (§7.1, Görev 5.8).
    pub async fn tumunu_kapat_ve_bekle(self: &Arc<Self>) {
        let (durdurulacaklar, anahtarlar) = {
            let mut d = self.durum();
            let anahtarlar: Vec<McpAnahtari> = d.yuvalar.keys().cloned().collect();
            let mut liste = Vec::new();
            for anahtar in &anahtarlar {
                if let Some(cikarilan) = Self::yuvadan_sureci_ayir(
                    &mut d,
                    anahtar,
                    YoneticiHatasi::SunucuDurduruldu,
                    McpSunucuDurumu::Durdu,
                ) {
                    liste.push(cikarilan);
                }
            }
            (liste, anahtarlar)
        };

        for (anahtar, kusak, kok_yolu) in durdurulacaklar {
            let _ = self.kapat_ve_denetle(anahtar, kusak, kok_yolu).await;
        }
        for anahtar in anahtarlar {
            self.yuva_kapanislarini_bekle(&anahtar).await;
        }
    }

    #[allow(clippy::type_complexity)]
    fn alandaki_tum_surecleri_ayir(
        d: &mut YoneticiDurumu,
        alan_kimligi: &str,
        iptal_nedeni: YoneticiHatasi,
        hedef_durum: McpSunucuDurumu,
    ) -> Vec<(McpAnahtari, Arc<CalisanKusak>, Option<PathBuf>)> {
        let anahtarlar: Vec<McpAnahtari> = d
            .yuvalar
            .keys()
            .filter(|k| k.alan_kimligi == alan_kimligi)
            .cloned()
            .collect();
        let mut liste = Vec::new();
        for anahtar in anahtarlar {
            if let Some(cikarilan) =
                Self::yuvadan_sureci_ayir(d, &anahtar, iptal_nedeni.clone(), hedef_durum.clone())
            {
                liste.push(cikarilan);
            }
        }
        liste
    }

    #[allow(clippy::type_complexity)]
    fn yuvadan_sureci_ayir(
        d: &mut YoneticiDurumu,
        anahtar: &McpAnahtari,
        iptal_nedeni: YoneticiHatasi,
        hedef_durum: McpSunucuDurumu,
    ) -> Option<(McpAnahtari, Arc<CalisanKusak>, Option<PathBuf>)> {
        let kok = d
            .alanlar
            .get(&anahtar.alan_kimligi)
            .and_then(|a| a.kok.clone());
        let yuva = d
            .yuvalar
            .entry(anahtar.clone())
            .or_insert_with(SunucuYuvasi::yeni);
        yuva.baslatma_revizyonu = yuva.baslatma_revizyonu.saturating_add(1);
        let kusak = yuva.calisan.take()?;
        kusak.iptal_et(iptal_nedeni);
        yuva.durum = hedef_durum;
        yuva.aktif_kapanis_sayisi = yuva.aktif_kapanis_sayisi.saturating_add(1);
        Some((anahtar.clone(), kusak, kok))
    }

    fn arka_planda_kapat(
        self: &Arc<Self>,
        anahtar: McpAnahtari,
        kusak: Arc<CalisanKusak>,
        kok: Option<PathBuf>,
    ) {
        let yonetici = Arc::clone(self);
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let _ = yonetici.kapat_ve_denetle(anahtar, kusak, kok).await;
            });
        }
    }

    async fn kapat_ve_denetle(
        self: &Arc<Self>,
        anahtar: McpAnahtari,
        kusak: Arc<CalisanKusak>,
        kok: Option<PathBuf>,
    ) -> Result<(), YoneticiHatasi> {
        let yonetici = self.clone();
        // Çağıranın geleceği bırakılsa da kapanış işi süreç sahipliğini korur.
        tokio::spawn(async move { yonetici.kapanisi_tamamla(anahtar, kusak, kok).await })
            .await
            .map_err(|_| {
                YoneticiHatasi::SunucuDurdurulamadi("kapanış görevi tamamlanamadı".to_owned())
            })?
    }

    async fn kapanisi_tamamla(
        self: &Arc<Self>,
        anahtar: McpAnahtari,
        kusak: Arc<CalisanKusak>,
        kok: Option<PathBuf>,
    ) -> Result<(), YoneticiHatasi> {
        let kapat_sonucu = kusak.ucu.kapat().await;
        let surec_kapandi = kapat_sonucu.is_ok();

        let mut son_denetim_basarili = true;
        if let Some(kok_yolu) = kok {
            let onceki = kusak
                .adli_girdiler
                .lock()
                .map(|g| g.clone())
                .unwrap_or_default();
            let denetim = self
                .koruma_denetleyicisi
                .son_denetim(SonDenetimIstegi {
                    kok: kok_yolu,
                    gizli_ayarlari: self.ayarlar.gizli_ayarlari(),
                    onceki,
                })
                .await;
            son_denetim_basarili = denetim.mcp_icin_basarili_mi();
        }

        let sonuc = {
            let mut d = self.durum();
            if let Some(yuva) = d.yuvalar.get_mut(&anahtar) {
                yuva.aktif_kapanis_sayisi = yuva.aktif_kapanis_sayisi.saturating_sub(1);
                let bildirim = Arc::clone(&yuva.kapanis_bildirimi);
                let sonuc = match (kapat_sonucu, son_denetim_basarili) {
                    (Err(hata), _) => {
                        let detay = hata.to_string();
                        yuva.durdurulamadi_hatasi = Some(detay.clone());
                        yuva.durum = McpSunucuDurumu::hata(McpHataNedeni::Durdurulamadi {
                            detay: detay.clone(),
                        });
                        Err(YoneticiHatasi::SunucuDurdurulamadi(detay))
                    }
                    (Ok(_), false) => {
                        let detay = "son denetim tamamlanamadı veya karantina başarısız".to_owned();
                        yuva.durdurulamadi_hatasi = Some(detay.clone());
                        yuva.durum = McpSunucuDurumu::hata(McpHataNedeni::Durdurulamadi {
                            detay: detay.clone(),
                        });
                        Err(YoneticiHatasi::SunucuDurdurulamadi(detay))
                    }
                    (Ok(_), true) => Ok(()),
                };
                drop(d);
                bildirim.notify_waiters();
                sonuc
            } else {
                Ok(())
            }
        };
        self.cagri_defteri.surec_kapanisini_bildir(
            &anahtar.alan_kimligi,
            &anahtar.sunucu,
            kusak.kusak,
            McpCagriKapanisSonucu {
                surec_kapandi,
                son_denetim_basarili,
            },
        );
        sonuc
    }

    async fn yuva_kapanislarini_bekle(&self, anahtar: &McpAnahtari) {
        let bildirim = {
            let d = self.durum();
            let Some(yuva) = d.yuvalar.get(anahtar) else {
                return;
            };
            yuva.kapanis_bildirimi.clone()
        };
        loop {
            let uyandirma = bildirim.notified();
            tokio::pin!(uyandirma);
            uyandirma.as_mut().enable();
            {
                let d = self.durum();
                let Some(yuva) = d.yuvalar.get(anahtar) else {
                    return;
                };
                if yuva.aktif_kapanis_sayisi == 0 {
                    return;
                }
            }
            uyandirma.await;
        }
    }

    fn sunucuyu_hazirla(
        &self,
        alan_kimligi: &str,
        kok: &Path,
        sunucu: &str,
    ) -> Result<HazirlanmisSunucu, YoneticiHatasi> {
        let (mcp_ayarlar, _) = self.mcp_depo.mcp_ayarlarini_oku();
        let Some(kayit): Option<&McpSunucuKaydi> =
            mcp_ayarlar.sunucular.iter().find(|s| s.ad == sunucu)
        else {
            return Err(YoneticiHatasi::SunucuBulunamadi(sunucu.to_owned()));
        };
        let sistem_ortami = self.durum().sistem_ortami.clone();
        match kayitli_sunucuyu_hazirla_ve_ozetle(
            kayit,
            &mcp_ayarlar.secili_yurutulebilirler,
            self.kasa.as_ref(),
            &sistem_ortami,
            &self.yalitim_dizini,
            kok,
            alan_kimligi,
        ) {
            Ok(hazir) => Ok(hazir),
            Err(neden) => {
                let mut d = self.durum();
                let yuva = d
                    .yuvalar
                    .entry(McpAnahtari::yeni(alan_kimligi, sunucu))
                    .or_insert_with(SunucuYuvasi::yeni);
                yuva.durum = McpSunucuDurumu::hata(neden.clone());
                Err(YoneticiHatasi::SunucuHatasi(neden.kullanici_mesaji()))
            }
        }
    }

    fn izin_dosyasini_kaydet_kilitli(&self, d: &YoneticiDurumu) {
        let dosya = KaliciIzinDosyasi {
            yalitimli_izinler: d.yalitimli_izinler.values().cloned().collect(),
            yazma_kusaklari: d
                .alanlar
                .iter()
                .filter(|(_, a)| a.yazma_kusagi.deger() > 0)
                .map(|(k, a)| (k.clone(), a.yazma_kusagi.deger()))
                .collect(),
        };
        let yol = self.veri_dizini.join(IZIN_DOSYASI_ADI);
        let _ = izin_dosyasini_yaz(&yol, &dosya);
    }

    /// Çalışma alanı için kayıtlı bütün başlatma izinlerini listeler (Ayarlar için).
    pub fn kayitli_baslatma_izinleri(&self) -> Vec<(String, String, CalistirmaKipi)> {
        let d = self.durum();
        let mut sonuc: BTreeSet<(String, String, u8)> = BTreeSet::new();
        for anahtar in d.yalitimli_izinler.keys() {
            sonuc.insert((anahtar.alan_kimligi.clone(), anahtar.sunucu.clone(), 1));
        }
        for anahtar in d.gecici_tam_erisim_izinleri.keys() {
            sonuc.insert((anahtar.alan_kimligi.clone(), anahtar.sunucu.clone(), 2));
        }
        sonuc
            .into_iter()
            .map(|(alan, sunucu, kip)| {
                (
                    alan,
                    sunucu,
                    if kip == 1 {
                        CalistirmaKipi::Yalitimli
                    } else {
                        CalistirmaKipi::TamErisim
                    },
                )
            })
            .collect()
    }
}

/// `CalismaAlani` ve `KabukAraci` tarafından yazma/komut sınırında çağrılan
/// bildirim köprüsü (§5.1.1, §4.3).
pub struct AlanYazmaBildirimi {
    yonetici: Arc<McpYoneticisi>,
    alan_kimligi: String,
    kok: PathBuf,
}

impl AlanYazmaBildirimi {
    pub fn yeni(
        yonetici: Arc<McpYoneticisi>,
        alan_kimligi: impl Into<String>,
        kok: PathBuf,
    ) -> Arc<Self> {
        Arc::new(Self {
            yonetici,
            alan_kimligi: alan_kimligi.into(),
            kok,
        })
    }

    pub fn yazma_basladi(&self) {
        self.yonetici.yazma_kusagini_artir(&self.alan_kimligi);
    }

    pub async fn yazma_bitti(&self) {
        self.yonetici
            .kalfa_yazmasi_sonrasi_denetle(&self.alan_kimligi, &self.kok)
            .await;
    }

    pub fn bildirim_cifti(
        self: &Arc<Self>,
    ) -> (
        kalfa_tools::YazmaBildirimi,
        kalfa_tools::YazmaBitisBildirimi,
    ) {
        let basladi_hedef = self.clone();
        let bitti_hedef = self.clone();
        (
            Arc::new(move || basladi_hedef.yazma_basladi()),
            Arc::new(move || {
                let bitti_hedef = bitti_hedef.clone();
                Box::pin(async move { bitti_hedef.yazma_bitti().await })
            }),
        )
    }
}

impl McpYoneticisi {
    /// Çalışan kuşağın kod güveni sınıfını (`Harici` veya `WdeKodlu`) döner.
    pub fn calisan_kod_guveni(&self, alan_kimligi: &str, sunucu: &str) -> Option<KodGuveni> {
        let d = self.durum();
        d.yuvalar
            .get(&McpAnahtari::yeni(alan_kimligi, sunucu))
            .and_then(|y| y.calisan.as_ref())
            .map(|c| c.kod_guveni)
    }
}

fn izin_dosyasini_oku(veri_dizini: &Path) -> KaliciIzinDosyasi {
    let yol = veri_dizini.join(IZIN_DOSYASI_ADI);
    std::fs::read_to_string(yol)
        .ok()
        .and_then(|metin| serde_json::from_str(&metin).ok())
        .unwrap_or_default()
}

fn izin_dosyasini_yaz(yol: &Path, dosya: &KaliciIzinDosyasi) -> std::io::Result<()> {
    let metin = serde_json::to_vec_pretty(dosya).map_err(std::io::Error::other)?;
    if let Some(ust) = yol.parent() {
        std::fs::create_dir_all(ust)?;
    }
    let gecici = yol.with_extension("json.yeni");
    std::fs::write(&gecici, metin)?;
    std::fs::rename(&gecici, yol)
}

#[cfg(test)]
mod tests {
    mod gercek_surec_testleri {
        include!("gercek_surec_testleri.rs");
    }
    use super::*;
    use std::future::Future;
    use std::pin::Pin;

    use kalfa_core::izin::Siniflandirici;
    use kalfa_core::yol::CalismaYolu;
    use kalfa_sandbox::tarama::KorumaTaramaHatasi;
    use kalfa_sandbox::{BaslatmaHatasi, CalistirmaIstegi, EtkilesimliIstek, KapanisSonucu, Surec};
    use kalfa_tools::koruma::{KorumaDogrulamaHatasi, SonDenetimSonucu};

    use crate::calisma_alanlari::CalismaAlaniKaydi;
    use crate::mcp::ayarlar::{McpKipi, McpKomut};
    use crate::mcp::kasa::test_destek::SahteGizliKasa;
    use crate::mcp::surec::SahteSaat;
    use crate::test_destek::SahteIletisim;

    struct TestYaliticisi {
        durum: Mutex<YalitimDurumu>,
        etkin_surec_sayisi: Arc<AtomicUsize>,
        silinen_sunucu_kokleri: Mutex<Vec<(PathBuf, String)>>,
        kapanis_bitmeden_kok_silindi: AtomicBool,
    }

    impl TestYaliticisi {
        fn yeni(etkin_surec_sayisi: Arc<AtomicUsize>) -> Arc<Self> {
            Arc::new(Self {
                durum: Mutex::new(YalitimDurumu::Hazir {
                    arka_uc: "test-yalitim".into(),
                    surum: "1.0".into(),
                }),
                etkin_surec_sayisi,
                silinen_sunucu_kokleri: Mutex::new(Vec::new()),
                kapanis_bitmeden_kok_silindi: AtomicBool::new(false),
            })
        }
    }

    impl Yalitici for TestYaliticisi {
        fn durum(&self) -> YalitimDurumu {
            self.durum.lock().unwrap().clone()
        }

        fn profil_ozeti(&self, ag: bool, liste_ozeti: Option<&str>) -> Option<String> {
            match self.durum() {
                YalitimDurumu::Hazir { arka_uc, surum } => Some(format!(
                    "{arka_uc}:{surum}:{ag}:{}",
                    liste_ozeti.unwrap_or("-")
                )),
                YalitimDurumu::Yok { .. } => None,
            }
        }

        fn baslat(&self, _istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi> {
            Err(BaslatmaHatasi::Yalitim("test yalıtıcısı".into()))
        }

        fn sunucu_kokunu_kaldir(&self, alan_koku: &Path, sunucu: &str) -> Result<(), String> {
            if self.etkin_surec_sayisi.load(Ordering::SeqCst) > 0 {
                self.kapanis_bitmeden_kok_silindi
                    .store(true, Ordering::SeqCst);
            }
            self.silinen_sunucu_kokleri
                .lock()
                .unwrap()
                .push((alan_koku.to_path_buf(), sunucu.to_owned()));
            Ok(())
        }
    }

    struct SahteSurecUcu {
        etkin_surec_sayisi: Arc<AtomicUsize>,
        cikis_bildirimi: Arc<Notify>,
        hazirlan_beklet: Option<Arc<Notify>>,
        hazirlan_basladi: Option<Arc<Notify>>,
        cagri_beklet: Option<Arc<Notify>>,
        cagri_basladi: Option<Arc<Notify>>,
        kapat_beklet: Option<Arc<Notify>>,
        kapat_basladi: Option<Arc<Notify>>,
        cagri_hatasi: Option<McpHatasi>,
        kapanis_sonucu: KapanisSonucu,
        kapandi: AtomicBool,
        araclar: Vec<Tool>,
    }

    impl McpSurecUcu for SahteSurecUcu {
        fn hazirlan<'a>(
            &'a self,
            _zaman_asimi: Duration,
        ) -> Pin<Box<dyn Future<Output = Result<(), McpHataNedeni>> + Send + 'a>> {
            Box::pin(async move {
                if let Some(b) = &self.hazirlan_basladi {
                    b.notify_one();
                }
                if let Some(b) = &self.hazirlan_beklet {
                    b.notified().await;
                }
                Ok(())
            })
        }
        fn araclari_listele<'a>(
            &'a self,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<Tool>, McpHatasi>> + Send + 'a>> {
            let cagri_beklet = self.cagri_beklet.clone();
            let cagri_basladi = self.cagri_basladi.clone();
            let cagri_hatasi = self.cagri_hatasi.clone();
            let araclar = self.araclar.clone();
            Box::pin(async move {
                if let Some(b) = cagri_basladi {
                    b.notify_one();
                }
                if let Some(beklet) = cagri_beklet {
                    beklet.notified().await;
                }
                if let Some(hata) = cagri_hatasi {
                    return Err(hata);
                }
                Ok(araclar)
            })
        }

        fn arac_cagir<'a>(
            &'a self,
            _ad: String,
            _argumanlar: JsonObject,
        ) -> Pin<Box<dyn Future<Output = Result<CallToolResult, McpHatasi>> + Send + 'a>> {
            let cagri_beklet = self.cagri_beklet.clone();
            let cagri_basladi = self.cagri_basladi.clone();
            let cagri_hatasi = self.cagri_hatasi.clone();
            Box::pin(async move {
                if let Some(b) = cagri_basladi {
                    b.notify_one();
                }
                if let Some(beklet) = cagri_beklet {
                    beklet.notified().await;
                }
                if let Some(hata) = cagri_hatasi {
                    return Err(hata);
                }
                Ok(CallToolResult::success(Vec::new()))
            })
        }

        fn bitisi_bekle<'a>(&'a self) -> Pin<Box<dyn Future<Output = Option<i32>> + Send + 'a>> {
            Box::pin(async move {
                self.cikis_bildirimi.notified().await;
                Some(1)
            })
        }

        fn kapat<'a>(
            &'a self,
        ) -> Pin<Box<dyn Future<Output = std::io::Result<KapanisSonucu>> + Send + 'a>> {
            Box::pin(async move {
                if let Some(b) = &self.kapat_basladi {
                    b.notify_one();
                }
                if let Some(beklet) = &self.kapat_beklet {
                    beklet.notified().await;
                }
                if !self.kapandi.swap(true, Ordering::SeqCst) {
                    self.etkin_surec_sayisi.fetch_sub(1, Ordering::SeqCst);
                }
                Ok(self.kapanis_sonucu.clone())
            })
        }
    }

    struct SahteSurecBaslatici {
        baslatma_sayisi: AtomicUsize,
        cikis_bildirimi: Arc<Notify>,
        hazirlan_beklet: Mutex<Option<Arc<Notify>>>,
        hazirlan_basladi: Mutex<Option<Arc<Notify>>>,
        etkin_surec_sayisi: Arc<AtomicUsize>,
        isaret_dosyasi: Mutex<Option<PathBuf>>,
        cagri_beklet: Mutex<Option<Arc<Notify>>>,
        cagri_basladi: Mutex<Option<Arc<Notify>>>,
        kapat_beklet: Mutex<Option<Arc<Notify>>>,
        kapat_basladi: Mutex<Option<Arc<Notify>>>,
        cagri_hatasi: Mutex<Option<McpHatasi>>,
        baslatma_hatalari_kalan: AtomicUsize,
        kapanis_sonucu: Mutex<KapanisSonucu>,
        /// Sahte sürecin `tools/list` yanıtı.
        araclar: Mutex<Vec<Tool>>,
    }

    impl SahteSurecBaslatici {
        fn yeni(etkin_surec_sayisi: Arc<AtomicUsize>) -> Arc<Self> {
            Arc::new(Self {
                baslatma_sayisi: AtomicUsize::new(0),
                cikis_bildirimi: Arc::new(Notify::new()),
                hazirlan_beklet: Mutex::new(None),
                hazirlan_basladi: Mutex::new(None),
                etkin_surec_sayisi,
                isaret_dosyasi: Mutex::new(None),
                cagri_beklet: Mutex::new(None),
                cagri_basladi: Mutex::new(None),
                kapat_beklet: Mutex::new(None),
                kapat_basladi: Mutex::new(None),
                cagri_hatasi: Mutex::new(None),
                baslatma_hatalari_kalan: AtomicUsize::new(0),
                araclar: Mutex::new(Vec::new()),
                kapanis_sonucu: Mutex::new(KapanisSonucu {
                    kendiliginden: true,
                    cikis: Some(0),
                    sure: Duration::ZERO,
                    ag: None,
                }),
            })
        }

        fn sayac(&self) -> usize {
            self.baslatma_sayisi.load(Ordering::SeqCst)
        }
    }

    impl McpSurecBaslatici for SahteSurecBaslatici {
        fn baslat(
            &self,
            _yalitici: &Arc<dyn Yalitici>,
            _istek: EtkilesimliIstek,
            stderr_halkasi: Arc<Mutex<StderrHalkasi>>,
        ) -> Result<Arc<dyn McpSurecUcu>, McpHataNedeni> {
            if let Some(yol) = self.isaret_dosyasi.lock().unwrap().clone() {
                let _ = std::fs::write(yol, b"basladi");
            }
            self.baslatma_sayisi.fetch_add(1, Ordering::SeqCst);
            if let Ok(mut h) = stderr_halkasi.lock() {
                h.besle(b"sunucu baslatildi\n");
            }
            let kalan = self.baslatma_hatalari_kalan.load(Ordering::SeqCst);
            if kalan > 0 {
                self.baslatma_hatalari_kalan.fetch_sub(1, Ordering::SeqCst);
                return Err(McpHataNedeni::ElSikismaZamanAsimi);
            }
            self.etkin_surec_sayisi.fetch_add(1, Ordering::SeqCst);
            Ok(Arc::new(SahteSurecUcu {
                etkin_surec_sayisi: self.etkin_surec_sayisi.clone(),
                cikis_bildirimi: self.cikis_bildirimi.clone(),
                hazirlan_beklet: self.hazirlan_beklet.lock().unwrap().clone(),
                hazirlan_basladi: self.hazirlan_basladi.lock().unwrap().clone(),
                cagri_beklet: self.cagri_beklet.lock().unwrap().clone(),
                cagri_basladi: self.cagri_basladi.lock().unwrap().clone(),
                kapat_beklet: self.kapat_beklet.lock().unwrap().clone(),
                kapat_basladi: self.kapat_basladi.lock().unwrap().clone(),
                cagri_hatasi: self.cagri_hatasi.lock().unwrap().clone(),
                kapanis_sonucu: self.kapanis_sonucu.lock().unwrap().clone(),
                kapandi: AtomicBool::new(false),
                araclar: self.araclar.lock().unwrap().clone(),
            }) as Arc<dyn McpSurecUcu>)
        }
    }

    struct KontrolluKoruma {
        ic: GercekKorumaDenetleyicisi,
        dogrula_sayisi: AtomicUsize,
        son_denetim_sayisi: AtomicUsize,
        zorla_dogrulanamadi: AtomicBool,
        dogrula_giris_bildirimi: Mutex<Option<Arc<Notify>>>,
        dogrula_devam_bildirimi: Mutex<Option<Arc<Notify>>>,
    }

    impl KontrolluKoruma {
        fn yeni(saat: Arc<SahteSaat>) -> Arc<Self> {
            Arc::new(Self {
                ic: GercekKorumaDenetleyicisi::saatli(saat),
                dogrula_sayisi: AtomicUsize::new(0),
                son_denetim_sayisi: AtomicUsize::new(0),
                zorla_dogrulanamadi: AtomicBool::new(false),
                dogrula_giris_bildirimi: Mutex::new(None),
                dogrula_devam_bildirimi: Mutex::new(None),
            })
        }
    }

    impl KorumaDenetleyicisi for KontrolluKoruma {
        fn dogrula<'a>(
            &'a self,
            yalitici: &'a Arc<dyn Yalitici>,
            istek: KorumaDenetimIstegi,
        ) -> Pin<Box<dyn Future<Output = McpKorumaSonucu> + Send + 'a>> {
            self.dogrula_sayisi.fetch_add(1, Ordering::SeqCst);
            let giris = self.dogrula_giris_bildirimi.lock().unwrap().clone();
            let devam = self.dogrula_devam_bildirimi.lock().unwrap().clone();
            let zorla = self.zorla_dogrulanamadi.load(Ordering::SeqCst);
            Box::pin(async move {
                if let Some(g) = giris {
                    g.notify_one();
                }
                if let Some(d) = devam {
                    d.notified().await;
                }
                if zorla {
                    return McpKorumaSonucu::Dogrulanamadi(KorumaDogrulamaHatasi::TaramaEksik(
                        KorumaTaramaHatasi::ZamanAsimi,
                    ));
                }
                self.ic.dogrula(yalitici, istek).await
            })
        }

        fn son_denetim<'a>(
            &'a self,
            istek: SonDenetimIstegi,
        ) -> Pin<Box<dyn Future<Output = SonDenetimSonucu> + Send + 'a>> {
            self.son_denetim_sayisi.fetch_add(1, Ordering::SeqCst);
            self.ic.son_denetim(istek)
        }
    }

    struct TestOrtami {
        _gecici: tempfile::TempDir,
        veri_dizini: PathBuf,
        yalitim_dizini: PathBuf,
        proje_koku: PathBuf,
        alan_kimligi: String,
        ayarlar: Arc<Ayarlar>,
        yalitim_ayarlari: Arc<YalitimAyarlari>,
        kasa: Arc<SahteGizliKasa>,
        iletisim: Arc<SahteIletisim>,
        yalitici: Arc<TestYaliticisi>,
        koruma: Arc<KontrolluKoruma>,
        baslatici: Arc<SahteSurecBaslatici>,
        saat: Arc<SahteSaat>,
        sistem_ortami: Vec<(String, String)>,
        yonetici: Arc<McpYoneticisi>,
    }

    impl TestOrtami {
        fn kur() -> Self {
            let gecici = tempfile::tempdir().unwrap();
            let veri_dizini = gecici.path().join("veri");
            let yalitim_dizini = gecici.path().join("yalitim");
            let proje_koku = gecici.path().join("proje");
            let harici_bin_dizini = gecici.path().join("harici-bin");
            std::fs::create_dir_all(&veri_dizini).unwrap();
            std::fs::create_dir_all(&yalitim_dizini).unwrap();
            std::fs::create_dir_all(&proje_koku).unwrap();
            std::fs::create_dir_all(&harici_bin_dizini).unwrap();

            // Harici yürütülebilir ve proje içi yürütülebilir oluştur:
            let harici_exe = Self::test_programi_yaz(&harici_bin_dizini, "mcp-harici");
            let _ = harici_exe;
            let proje_bin = proje_koku.join("bin");
            std::fs::create_dir_all(&proje_bin).unwrap();
            let _proje_exe = Self::test_programi_yaz(&proje_bin, "mcp-yerel");

            let alan_kimligi = CalismaAlaniKaydi::kimlik(&proje_koku);
            let ayarlar = Arc::new(Ayarlar::oku(&veri_dizini));
            let yalitim_ayarlari = Arc::new(YalitimAyarlari::oku(&veri_dizini));
            let kasa = Arc::new(SahteGizliKasa::default());
            let iletisim = Arc::new(SahteIletisim::default());
            let etkin_surec_sayisi = Arc::new(AtomicUsize::new(0));
            let yalitici = TestYaliticisi::yeni(etkin_surec_sayisi.clone());
            let saat = SahteSaat::yeni();
            let koruma = KontrolluKoruma::yeni(saat.clone());
            let baslatici = SahteSurecBaslatici::yeni(etkin_surec_sayisi);

            let path_degeri = harici_bin_dizini.to_string_lossy().into_owned();
            let sistem_ortami = vec![("PATH".to_owned(), path_degeri)];

            let yonetici = McpYoneticisi::yeni(McpYoneticiBilesenleri {
                veri_dizini: veri_dizini.clone(),
                yalitim_dizini: yalitim_dizini.clone(),
                ayarlar: ayarlar.clone(),
                mcp_depo: ayarlar.clone(),
                kasa: kasa.clone(),
                iletisim: iletisim.clone(),
                yalitici: yalitici.clone(),
                yalitim_ayarlari: yalitim_ayarlari.clone(),
                koruma_denetleyicisi: Some(koruma.clone()),
                surec_baslatici: Some(baslatici.clone()),
                saat: Some(saat.clone()),
                sistem_ortami: Some(sistem_ortami.clone()),
            });

            Self {
                _gecici: gecici,
                veri_dizini,
                yalitim_dizini,
                proje_koku,
                alan_kimligi,
                ayarlar,
                yalitim_ayarlari,
                kasa,
                iletisim,
                yalitici,
                koruma,
                baslatici,
                saat,
                sistem_ortami,
                yonetici,
            }
        }

        fn test_programi_yaz(dizin: &Path, ad: &str) -> PathBuf {
            #[cfg(windows)]
            let dosya_adi = format!("{ad}.exe");
            #[cfg(not(windows))]
            let dosya_adi = ad.to_owned();

            let yol = dizin.join(dosya_adi);
            std::fs::write(&yol, format!("sahte-ikili-{ad}").as_bytes()).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut izin = std::fs::metadata(&yol).unwrap().permissions();
                izin.set_mode(0o755);
                std::fs::set_permissions(&yol, izin).unwrap();
            }
            yol
        }

        fn sunucu_kaydet(&self, sunucu: McpSunucuKaydi, secili: Option<(String, PathBuf)>) {
            let (mut mcp_ayar, rev) = self.ayarlar.mcp_ayarlarini_oku();
            mcp_ayar.sunucular.retain(|s| s.ad != sunucu.ad);
            mcp_ayar.sunucular.push(sunucu);
            if let Some((kimlik, yol)) = secili {
                mcp_ayar.secili_yurutulebilirler.insert(kimlik, yol);
            }
            self.ayarlar.mcp_ayarlarini_kaydet(&mcp_ayar, rev).unwrap();
        }

        fn harici_sunucu_ekle(&self, ad: &str, kip: McpKipi) {
            self.sunucu_kaydet(
                McpSunucuKaydi {
                    ad: ad.to_owned(),
                    etkin: true,
                    komut: McpKomut::ProgramAdi {
                        ad: "mcp-harici".to_owned(),
                        argumanlar: vec!["--stdio".to_owned()],
                    },
                    ortam: Vec::new(),
                    kip,
                    ag_izinleri: Vec::new(),
                    baslatma_zamani_asimi_sn: 30,
                    cagri_zamani_asimi_sn: 120,
                },
                None,
            );
        }

        fn wde_kodlu_sunucu_ekle(&self, ad: &str, kip: McpKipi) {
            let proje_exe = self.proje_koku.join("bin").join(if cfg!(windows) {
                "mcp-yerel.exe"
            } else {
                "mcp-yerel"
            });
            let secili_kimlik = format!("exe-{ad}");
            self.sunucu_kaydet(
                McpSunucuKaydi {
                    ad: ad.to_owned(),
                    etkin: true,
                    komut: McpKomut::SeciliYurutulebilir {
                        kimlik: secili_kimlik.clone(),
                        argumanlar: Vec::new(),
                    },
                    ortam: Vec::new(),
                    kip,
                    ag_izinleri: Vec::new(),
                    baslatma_zamani_asimi_sn: 30,
                    cagri_zamani_asimi_sn: 120,
                },
                Some((secili_kimlik, proje_exe)),
            );
        }

        fn siki_oturum_ac(&self) -> OturumId {
            let oturum = OturumId::yeni();
            self.yonetici.oturum_kipi_guncelle(
                &self.alan_kimligi,
                Some(&self.proje_koku),
                oturum,
                Some(Kip::Siki),
            );
            oturum
        }

        async fn onayla_ve_baslat(&self, sunucu: &str) -> (McpBaslatmaSayaci, McpTarifi) {
            match self
                .yonetici
                .baslat_veya_dogrula(&self.alan_kimligi, &self.proje_koku, sunucu)
                .await
            {
                Ok(sonuc) => sonuc,
                Err(YoneticiHatasi::BaslatmaIzniGerekli(kart)) => {
                    let onay = self
                        .yonetici
                        .baslatma_izni_yanitla(
                            &kart.istek_kimligi,
                            &self.alan_kimligi,
                            &self.proje_koku,
                            true,
                        )
                        .await
                        .unwrap();
                    assert!(onay);
                    self.yonetici
                        .baslat_veya_dogrula(&self.alan_kimligi, &self.proje_koku, sunucu)
                        .await
                        .unwrap()
                }
                Err(diger) => panic!("beklenmeyen hata: {diger:?}"),
            }
        }
    }

    #[tokio::test]
    async fn baslatma_izni_yok_veya_reddedildi_ise_isaret_dosyasi_olusmaz() {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();

        let isaret = test.proje_koku.join("basladi.marker");
        *test.baslatici.isaret_dosyasi.lock().unwrap() = Some(isaret.clone());

        // 1. İzin yokken başlatma girişimi BaslatmaIzniGerekli döner, süreç hiç başlamaz:
        let Err(YoneticiHatasi::BaslatmaIzniGerekli(kart)) = test
            .yonetici
            .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "harici")
            .await
        else {
            panic!("başlatma izni kartı bekleniyordu");
        };
        assert_eq!(test.baslatici.sayac(), 0);
        assert!(!isaret.exists(), "izin yokken işaret dosyası oluşmamalı");

        // 2. Kullanıcı kartı reddettiğinde de süreç başlamaz:
        let sonuc = test
            .yonetici
            .baslatma_izni_yanitla(
                &kart.istek_kimligi,
                &test.alan_kimligi,
                &test.proje_koku,
                false,
            )
            .await
            .unwrap();
        assert!(!sonuc);
        assert!(matches!(
            test.yonetici
                .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "harici")
                .await,
            Err(YoneticiHatasi::BaslatmaIzniGerekli(_))
        ));
        assert_eq!(test.baslatici.sayac(), 0);
        assert!(
            !isaret.exists(),
            "reddedilen izinde işaret dosyası oluşmamalı"
        );
    }

    #[tokio::test]
    async fn yalniz_inceleme_oturumunda_surec_baslamaz_ve_son_oturum_incelemeye_gecince_kapanir() {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
        let isaret = test.proje_koku.join("inceleme.marker");
        *test.baslatici.isaret_dosyasi.lock().unwrap() = Some(isaret.clone());

        let oturum_1 = OturumId::yeni();
        test.yonetici.oturum_kipi_guncelle(
            &test.alan_kimligi,
            Some(&test.proje_koku),
            oturum_1,
            Some(Kip::Inceleme),
        );

        // Yalnız İnceleme oturumu varken süreç başlamaz:
        assert_eq!(
            test.yonetici
                .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "harici")
                .await,
            Err(YoneticiHatasi::IncelemeDisiOturumYok)
        );
        assert!(!isaret.exists());

        // İkinci oturum Sıkı kiple açılır, sunucu onaylanıp başlatılır:
        let oturum_2 = test.siki_oturum_ac();
        let (kusak, _) = test.onayla_ve_baslat("harici").await;
        assert_eq!(kusak, McpBaslatmaSayaci::yeni(1));
        assert!(isaret.exists());
        assert_eq!(
            test.yonetici
                .calisan_kod_guveni(&test.alan_kimligi, "harici"),
            Some(KodGuveni::WDisindaKodlu)
        );

        // İkinci oturum da İnceleme kipine geçince alanda İnceleme dışı oturum kalmaz;
        // çalışan MCP süreci hemen kapatılır:
        test.yonetici.oturum_kipi_guncelle(
            &test.alan_kimligi,
            Some(&test.proje_koku),
            oturum_2,
            Some(Kip::Inceleme),
        );
        test.yonetici
            .sunucuyu_durdur_ve_bekle(&test.alan_kimligi, "harici")
            .await
            .unwrap();
        assert_eq!(
            test.yonetici.sunucu_durumu(&test.alan_kimligi, "harici"),
            McpSunucuDurumu::Durdu
        );
        assert_eq!(test.yalitici.etkin_surec_sayisi.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn tam_erisim_izni_diske_yazilmaz_yalitimli_kalici_izin_yeniden_kullanilir() {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("yalitimli-s", McpKipi::Yalitimli);
        test.harici_sunucu_ekle("tam-s", McpKipi::TamErisim);
        let _oturum = test.siki_oturum_ac();

        // 1. Yalıtımlı sunucu onaylanır ve başlatılır:
        let (k1, _) = test.onayla_ve_baslat("yalitimli-s").await;
        assert_eq!(k1, McpBaslatmaSayaci::yeni(1));

        // 2. Tam erişimli sunucu: yerel onay reddedilirse başlamaz:
        test.iletisim.onay_ver(false);
        let Err(YoneticiHatasi::BaslatmaIzniGerekli(kart_red)) = test
            .yonetici
            .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "tam-s")
            .await
        else {
            panic!("başlatma kartı bekleniyordu");
        };
        assert_eq!(
            test.yonetici
                .baslatma_izni_yanitla(
                    &kart_red.istek_kimligi,
                    &test.alan_kimligi,
                    &test.proje_koku,
                    true,
                )
                .await,
            Err(YoneticiHatasi::YerelOnayReddedildi)
        );
        assert_eq!(test.baslatici.sayac(), 1);

        // 3. Tam erişimli sunucu: yerel onay kabul edilirse başlar:
        test.iletisim.onay_ver(true);
        let (k_tam, _) = test.onayla_ve_baslat("tam-s").await;
        assert_eq!(k_tam, McpBaslatmaSayaci::yeni(1));

        // mcp_izinleri.json içinde yalnız yalitimli-s bulunmalı, tam-s asla bulunmamalı:
        let izin_metni = std::fs::read_to_string(test.veri_dizini.join(IZIN_DOSYASI_ADI)).unwrap();
        assert!(izin_metni.contains("yalitimli-s"));
        assert!(!izin_metni.contains("tam-s"));

        // 4. Uygulama yeniden başlatıldığında (yeni McpYoneticisi):
        //    - yalitimli-s (Harici) aynı (T, Π, alan) ile yeniden onay istemeden başlar,
        //    - tam-s ise yeniden başlatma izni ve yerel onay ister!
        let yeni_yonetici = McpYoneticisi::yeni(McpYoneticiBilesenleri {
            veri_dizini: test.veri_dizini.clone(),
            yalitim_dizini: test.yalitim_dizini.clone(),
            ayarlar: test.ayarlar.clone(),
            mcp_depo: test.ayarlar.clone(),
            kasa: test.kasa.clone(),
            iletisim: test.iletisim.clone(),
            yalitici: test.yalitici.clone(),
            yalitim_ayarlari: test.yalitim_ayarlari.clone(),
            koruma_denetleyicisi: Some(test.koruma.clone()),
            surec_baslatici: Some(test.baslatici.clone()),
            saat: Some(test.saat.clone()),
            sistem_ortami: Some(test.sistem_ortami.clone()),
        });
        yeni_yonetici.oturum_kipi_guncelle(
            &test.alan_kimligi,
            Some(&test.proje_koku),
            OturumId::yeni(),
            Some(Kip::Siki),
        );

        let (k_yeniden, _) = yeni_yonetici
            .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "yalitimli-s")
            .await
            .unwrap();
        assert_eq!(k_yeniden, McpBaslatmaSayaci::yeni(1));

        assert!(matches!(
            yeni_yonetici
                .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "tam-s")
                .await,
            Err(YoneticiHatasi::BaslatmaIzniGerekli(_))
        ));
    }

    #[tokio::test]
    async fn aktif_cagri_bosta_suresini_sifirlarken_mutlak_koruma_yasi_dolunca_denetim_calisir() {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();

        let (_kusak, tarif) = test.onayla_ve_baslat("harici").await;
        let baslangic_dogrula = test.koruma.dogrula_sayisi.load(Ordering::SeqCst);

        // 5. dakikada ve 9. dakikada yapılan çağrılar boşta kalma sayacını sıfırlar,
        // ancak çağrı sırasında koruma özeti aynı kaldığı için t_K güncellenir.
        // Şimdi t_K'yı doğrudan 10 dk yaşlandıralım ama son_cagri_ani'nı yeni tutalım:
        test.saat.ilerlet(Duration::from_secs(5 * 60));
        test.yonetici
            .arac_cagir(
                &test.alan_kimligi,
                &test.proje_koku,
                "harici",
                &tarif,
                "ping",
                JsonObject::new(),
            )
            .await
            .unwrap();

        // Şimdi çağrı yapmadan 10 dk ilerletelim, ama son_cagri_ani'nı 2 dk önceye çekelim
        // (böylece boşta süresi 2 dk < 10 dk, fakat t_K yaşı 10 dk = MUTLAK_KORUMA_YASI):
        test.saat.ilerlet(Duration::from_secs(10 * 60));
        {
            let d = test.yonetici.durum();
            let calisan = d
                .yuvalar
                .get(&McpAnahtari::yeni(&test.alan_kimligi, "harici"))
                .unwrap()
                .calisan
                .as_ref()
                .unwrap();
            calisan
                .son_cagri_ani_ms
                .store(test.saat.simdi_ms() - 120_000, Ordering::SeqCst);
        }

        let onceki = test.koruma.dogrula_sayisi.load(Ordering::SeqCst);
        assert!(onceki > baslangic_dogrula);

        // Zamanlayıcı adımı çalıştığında boşta kapanmaz (son çağrı 2 dk önce),
        // ama t_K (10 dk) dolduğu için zorunlu koruma denetimi çalışır!
        test.yonetici.zamanlayici_adimi().await;
        assert_eq!(
            test.koruma.dogrula_sayisi.load(Ordering::SeqCst),
            onceki + 1
        );
        assert!(matches!(
            test.yonetici.sunucu_durumu(&test.alan_kimligi, "harici"),
            McpSunucuDurumu::Calisiyor { .. }
        ));

        // Şimdi gizli bir dosya ekleyip t_K'yı tekrar 10 dk yaşlandıralım:
        std::fs::write(test.proje_koku.join(".env"), "SECRET=12345678\n").unwrap();
        test.saat.ilerlet(Duration::from_secs(10 * 60));
        {
            let d = test.yonetici.durum();
            let calisan = d
                .yuvalar
                .get(&McpAnahtari::yeni(&test.alan_kimligi, "harici"))
                .unwrap()
                .calisan
                .as_ref()
                .unwrap();
            calisan
                .son_cagri_ani_ms
                .store(test.saat.simdi_ms() - 60_000, Ordering::SeqCst);
        }

        // Mutlak yaş denetimi K değişimini yakalar ve süreci hemen durdurur:
        test.yonetici.zamanlayici_adimi().await;
        assert_eq!(
            test.yonetici.sunucu_durumu(&test.alan_kimligi, "harici"),
            McpSunucuDurumu::Durdu
        );
    }

    #[tokio::test]
    async fn etkin_cagri_kesif_veya_onay_beklerken_bosta_kapanmaz_ama_mutlak_koruma_yasi_calisir() {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();

        let (_kusak, _) = test.onayla_ve_baslat("harici").await;

        // Onay bekleniyor bayrağı açıkken 15 dk geçse bile boşta kapanmaz,
        // ancak mutlak koruma yaşı (t_K >= 10 dk) denetimi yine çalışır!
        test.yonetici
            .onay_bekliyor_ayarla(&test.alan_kimligi, "harici", true);
        test.saat.ilerlet(Duration::from_secs(15 * 60));

        let onceki_dogrula = test.koruma.dogrula_sayisi.load(Ordering::SeqCst);
        test.yonetici.zamanlayici_adimi().await;
        assert_eq!(
            test.koruma.dogrula_sayisi.load(Ordering::SeqCst),
            onceki_dogrula + 1,
            "onay beklerken de mutlak koruma yaşı denetimi çalışmalı"
        );
        assert!(
            matches!(
                test.yonetici.sunucu_durumu(&test.alan_kimligi, "harici"),
                McpSunucuDurumu::Calisiyor { .. }
            ),
            "onay beklerken boşta kapanmamalı"
        );

        // Onay bekleme bittiğinde ve 10 dk daha geçtiğinde BostaKapandi durumuna geçer:
        test.yonetici
            .onay_bekliyor_ayarla(&test.alan_kimligi, "harici", false);
        test.saat.ilerlet(Duration::from_secs(11 * 60));
        test.yonetici.zamanlayici_adimi().await;
        assert_eq!(
            test.yonetici.sunucu_durumu(&test.alan_kimligi, "harici"),
            McpSunucuDurumu::BostaKapandi
        );
    }

    #[tokio::test]
    async fn koruma_degisimi_ve_dogrulanamadi_durumu_sureci_durdurur_ve_yeniden_baslatmayi_engeller()
     {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();

        let (_kusak, tarif) = test.onayla_ve_baslat("harici").await;

        // 1. İki çağrı arasında yeni `.env` eklendiğinde sonraki tools/call
        //    KorumalarDegisti döner ve süreci durdurur:
        std::fs::write(test.proje_koku.join(".env"), "API_TOKEN=supersecret\n").unwrap();
        let sonuc = test
            .yonetici
            .arac_cagir(
                &test.alan_kimligi,
                &test.proje_koku,
                "harici",
                &tarif,
                "oku",
                JsonObject::new(),
            )
            .await;
        assert_eq!(sonuc, Err(YoneticiHatasi::KorumalarDegisti));
        assert_eq!(
            test.yonetici.sunucu_durumu(&test.alan_kimligi, "harici"),
            McpSunucuDurumu::Durdu
        );

        // 2. Şimdi tarama hatası / 30 sn zaman aşımı (Doğrulanamadı) enjekte edelim:
        test.koruma
            .zorla_dogrulanamadi
            .store(true, Ordering::SeqCst);
        let baslat_err = test
            .yonetici
            .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "harici")
            .await
            .unwrap_err();
        assert!(matches!(
            baslat_err,
            YoneticiHatasi::KorumalarDogrulanamadi(_)
        ));
        assert!(matches!(
            test.yonetici.sunucu_durumu(&test.alan_kimligi, "harici"),
            McpSunucuDurumu::Hata {
                neden: McpHataNedeni::KorumaDogrulanamadi { .. },
                ..
            }
        ));

        // 3. Tarama tekrar sağlıklı olduğunda yeni kuşak (N = 2) güncel K ile başlar:
        test.koruma
            .zorla_dogrulanamadi
            .store(false, Ordering::SeqCst);
        let (yeni_kusak, _) = test
            .yonetici
            .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "harici")
            .await
            .unwrap();
        assert_eq!(yeni_kusak, McpBaslatmaSayaci::yeni(2));
    }

    #[tokio::test]
    async fn baslatma_sirasinda_yazma_kusagi_degisirse_wde_kodlu_sunucu_eski_izinle_baslamaz() {
        let test = TestOrtami::kur();
        test.wde_kodlu_sunucu_ekle("yerel", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();

        // Başlatma iznini W = 0 kuşağında onayla:
        let Err(YoneticiHatasi::BaslatmaIzniGerekli(kart)) = test
            .yonetici
            .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "yerel")
            .await
        else {
            panic!("başlatma kartı bekleniyordu");
        };
        assert!(kart.wde_kodlu);
        test.yonetici
            .baslatma_izni_yanitla(
                &kart.istek_kimligi,
                &test.alan_kimligi,
                &test.proje_koku,
                true,
            )
            .await
            .unwrap();

        // Şimdi baslat_veya_dogrula koruma taramasındayken (adım 7) araya bir
        // yazma girip W kuşağını 0 -> 1 artırsın:
        let giris = Arc::new(Notify::new());
        let devam = Arc::new(Notify::new());
        *test.koruma.dogrula_giris_bildirimi.lock().unwrap() = Some(giris.clone());
        *test.koruma.dogrula_devam_bildirimi.lock().unwrap() = Some(devam.clone());

        let yonetici_kopya = test.yonetici.clone();
        let alan_kopya = test.alan_kimligi.clone();
        let kok_kopya = test.proje_koku.clone();
        let baslat_gorevi = tokio::spawn(async move {
            yonetici_kopya
                .baslat_veya_dogrula(&alan_kopya, &kok_kopya, "yerel")
                .await
        });

        giris.notified().await;
        // Koruma doğrulaması sürerken W artar:
        assert_eq!(
            test.yonetici.yazma_kusagini_artir(&test.alan_kimligi),
            McpYazmaKusagi::yeni(1)
        );
        devam.notify_one();

        // Atomik rezervasyon adımı (adım 8) W'nin değiştiğini yakalar ve eski W=0
        // izniyle süreci başlatmayı reddedip yeni BaslatmaIzniGerekli döner!
        let sonuc = baslat_gorevi.await.unwrap();
        assert!(
            matches!(sonuc, Err(YoneticiHatasi::BaslatmaIzniGerekli(_))),
            "arada W arttığında eski izinle süreç başlamamalı: {sonuc:?}"
        );
        assert_eq!(test.baslatici.sayac(), 0);
    }

    #[tokio::test]
    async fn yazma_sonucu_koruma_taramasinin_bitisini_bekler() {
        use kalfa_core::arac::Arac;
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();
        test.onayla_ve_baslat("harici").await;
        let bildirim = AlanYazmaBildirimi::yeni(
            test.yonetici.clone(),
            test.alan_kimligi.clone(),
            test.proje_koku.clone(),
        );
        let (basladi, bitti) = bildirim.bildirim_cifti();
        let alan = kalfa_tools::CalismaAlani::ac_bildirimli(
            Arc::new(
                Siniflandirici::yeni_ayarla(&test.proje_koku, test.ayarlar.gizli_ayarlari())
                    .unwrap(),
            ),
            Some(basladi),
            Some(bitti),
        )
        .unwrap();
        let arac = kalfa_tools::DosyaYaz::yeni(alan);
        let argumanlar = serde_json::json!({"yol":"not.txt", "icerik":"yeni"});
        let hazirlik = arac.hazirla(&argumanlar).await.unwrap();
        let giris = Arc::new(Notify::new());
        let devam = Arc::new(Notify::new());
        *test.koruma.dogrula_giris_bildirimi.lock().unwrap() = Some(giris.clone());
        *test.koruma.dogrula_devam_bildirimi.lock().unwrap() = Some(devam.clone());
        let yazma = tokio::spawn(async move {
            arac.calistir(&argumanlar, &hazirlik, CancellationToken::new())
                .await
        });
        giris.notified().await;
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert!(
            !yazma.is_finished(),
            "araç sonucu koruma denetimini beklemeli"
        );
        devam.notify_one();
        yazma.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn kalfa_yazma_araclari_ve_mcp_cagrisi_w_kusagini_artirir_salt_okuma_artirmaz() {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
        test.wde_kodlu_sunucu_ekle("yerel", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();

        // Çalışma alanını AlanYazmaBildirimi ile aç:
        let siniflandirici = Arc::new(
            Siniflandirici::yeni_ayarla(&test.proje_koku, test.ayarlar.gizli_ayarlari()).unwrap(),
        );
        let bildirim = AlanYazmaBildirimi::yeni(
            test.yonetici.clone(),
            test.alan_kimligi.clone(),
            test.proje_koku.clone(),
        );
        let (yazma_basladi, yazma_bitti) = bildirim.bildirim_cifti();
        let calisma_alani = kalfa_tools::CalismaAlani::ac_bildirimli(
            siniflandirici,
            Some(yazma_basladi),
            Some(yazma_bitti),
        )
        .unwrap();

        assert_eq!(
            test.yonetici.yazma_kusagi(&test.alan_kimligi),
            McpYazmaKusagi::yeni(0)
        );

        // 1. Yazma işlemi W'yi 0 -> 1 artırır:
        let yol = CalismaYolu::coz("not.txt").unwrap();
        calisma_alani.yaz(&yol, b"merhaba\n").unwrap();
        assert_eq!(
            test.yonetici.yazma_kusagi(&test.alan_kimligi),
            McpYazmaKusagi::yeni(1)
        );

        // 2. Salt okuma (oku / sinirli_oku) W'yi artırmaz:
        let _ = calisma_alani.oku(&yol).unwrap();
        let _ = calisma_alani
            .sinirli_oku(&yol, &CancellationToken::new())
            .unwrap();
        assert_eq!(
            test.yonetici.yazma_kusagi(&test.alan_kimligi),
            McpYazmaKusagi::yeni(1)
        );

        // 3. WdeKodlu sunucuyu W = 1 kuşağında onayla ve başlat:
        let (yerel_kusak, _) = test.onayla_ve_baslat("yerel").await;
        assert_eq!(yerel_kusak, McpBaslatmaSayaci::yeni(1));

        // 4. Harici sunucuyu da onayla ve başlat:
        let (harici_kusak, harici_tarif) = test.onayla_ve_baslat("harici").await;
        assert_eq!(harici_kusak, McpBaslatmaSayaci::yeni(1));

        // 5. Harici sunucu üzerinden bir MCP çağrısı yapıldığında W (1 -> 2) artar:
        test.yonetici
            .arac_cagir(
                &test.alan_kimligi,
                &test.proje_koku,
                "harici",
                &harici_tarif,
                "islem",
                JsonObject::new(),
            )
            .await
            .unwrap();
        assert_eq!(
            test.yonetici.yazma_kusagi(&test.alan_kimligi),
            McpYazmaKusagi::yeni(2)
        );

        // 6. W arttığı için WdeKodlu sunucu durdurulup yeniden başlatılmak istendiğinde
        //    eski W=1 izni geçersizleşmiştir; Harici sunucu ise yeni izin istemez!
        test.yonetici
            .sunucuyu_durdur_ve_bekle(&test.alan_kimligi, "yerel")
            .await
            .unwrap();
        assert!(matches!(
            test.yonetici
                .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "yerel")
                .await,
            Err(YoneticiHatasi::BaslatmaIzniGerekli(_))
        ));

        test.yonetici
            .sunucuyu_durdur_ve_bekle(&test.alan_kimligi, "harici")
            .await
            .unwrap();
        let (harici_kusak_2, _) = test
            .yonetici
            .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "harici")
            .await
            .unwrap();
        assert_eq!(harici_kusak_2, McpBaslatmaSayaci::yeni(2));
    }

    #[tokio::test]
    async fn coken_wde_kodlu_sunucu_w_veya_k_degisince_otomatik_yeniden_baslatilmaz() {
        let test = TestOrtami::kur();
        test.wde_kodlu_sunucu_ekle("yerel", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();

        let (kusak, _) = test.onayla_ve_baslat("yerel").await;
        assert_eq!(kusak, McpBaslatmaSayaci::yeni(1));

        // Sunucu çökerken çalışma alanında bir yazma olmuş (W artmış) olsun:
        test.yonetici.yazma_kusagini_artir(&test.alan_kimligi);
        test.yonetici.beklenmeyen_cikis_bildir(
            &test.alan_kimligi,
            "yerel",
            kusak,
            &test.proje_koku,
        );

        // 1 sn geri çekilme dolsa bile W arttığı için otomatik yeniden başlatma
        // eski izinle süreci başlatamaz; BaslatmaIzniGerekli döner!
        test.saat.ilerlet(Duration::from_secs(2));
        assert!(matches!(
            test.yonetici
                .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "yerel")
                .await,
            Err(YoneticiHatasi::BaslatmaIzniGerekli(_))
        ));
        assert_eq!(test.baslatici.sayac(), 1);
    }

    #[tokio::test]
    async fn cokme_sonrasi_1_4_16_sn_geri_cekilmeyle_en_cok_3_kez_yeniden_baslatilir() {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();

        let (kusak_1, _) = test.onayla_ve_baslat("harici").await;
        assert_eq!(kusak_1, McpBaslatmaSayaci::yeni(1));

        // 1. çökme -> 1 sn geri çekilme:
        test.yonetici.beklenmeyen_cikis_bildir(
            &test.alan_kimligi,
            "harici",
            kusak_1,
            &test.proje_koku,
        );
        assert!(matches!(
            test.yonetici
                .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "harici")
                .await,
            Err(YoneticiHatasi::CokmeGeriCekilmeBekleniyor { kalan_ms: 1000 })
        ));
        test.saat.ilerlet(Duration::from_secs(1));
        let (kusak_2, _) = test
            .yonetici
            .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "harici")
            .await
            .unwrap();
        assert_eq!(kusak_2, McpBaslatmaSayaci::yeni(2));

        // 2. çökme -> 4 sn geri çekilme:
        test.yonetici.beklenmeyen_cikis_bildir(
            &test.alan_kimligi,
            "harici",
            kusak_2,
            &test.proje_koku,
        );
        assert!(matches!(
            test.yonetici
                .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "harici")
                .await,
            Err(YoneticiHatasi::CokmeGeriCekilmeBekleniyor { kalan_ms: 4000 })
        ));
        test.saat.ilerlet(Duration::from_secs(4));
        let (kusak_3, _) = test
            .yonetici
            .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "harici")
            .await
            .unwrap();
        assert_eq!(kusak_3, McpBaslatmaSayaci::yeni(3));

        // 3. çökme -> 16 sn geri çekilme (3. ve son otomatik yeniden başlatma hakkı):
        test.yonetici.beklenmeyen_cikis_bildir(
            &test.alan_kimligi,
            "harici",
            kusak_3,
            &test.proje_koku,
        );
        assert!(matches!(
            test.yonetici
                .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "harici")
                .await,
            Err(YoneticiHatasi::CokmeGeriCekilmeBekleniyor { kalan_ms: 16000 })
        ));
        test.saat.ilerlet(Duration::from_secs(16));
        let (kusak_4, _) = test
            .yonetici
            .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "harici")
            .await
            .unwrap();
        assert_eq!(kusak_4, McpBaslatmaSayaci::yeni(4));

        // 4. çökme -> 3 yeniden başlatma denemesi tükendiği için CokmeSiniriAsildi hatasına geçer:
        test.yonetici.beklenmeyen_cikis_bildir(
            &test.alan_kimligi,
            "harici",
            kusak_4,
            &test.proje_koku,
        );
        assert!(matches!(
            test.yonetici.sunucu_durumu(&test.alan_kimligi, "harici"),
            McpSunucuDurumu::Hata {
                neden: McpHataNedeni::CokmeSiniriAsildi { deneme: 3, .. },
                ..
            }
        ));
        assert!(matches!(
            test.yonetici
                .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "harici")
                .await,
            Err(YoneticiHatasi::SunucuHatasi(_))
        ));

        // Kullanıcının açık "Yeniden başlat" komutu sayacı sıfırlar ve tekrar başlatır:
        let (kusak_5, _) = test
            .yonetici
            .kullanici_yeniden_baslat(&test.alan_kimligi, &test.proje_koku, "harici")
            .await
            .unwrap();
        assert_eq!(kusak_5, McpBaslatmaSayaci::yeni(5));
    }

    #[tokio::test]
    async fn uzun_cagri_sirasinda_kontrol_kanali_bloklanmaz_ve_dogrulanan_kapanistan_once_kok_silinmez()
     {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();

        let cagri_beklet = Arc::new(Notify::new());
        let cagri_basladi = Arc::new(Notify::new());
        let kapat_beklet = Arc::new(Notify::new());
        let kapat_basladi = Arc::new(Notify::new());
        *test.baslatici.cagri_beklet.lock().unwrap() = Some(cagri_beklet.clone());
        *test.baslatici.cagri_basladi.lock().unwrap() = Some(cagri_basladi.clone());
        *test.baslatici.kapat_beklet.lock().unwrap() = Some(kapat_beklet.clone());
        *test.baslatici.kapat_basladi.lock().unwrap() = Some(kapat_basladi.clone());

        let (_kusak, tarif) = test.onayla_ve_baslat("harici").await;

        let yonetici_cagri = test.yonetici.clone();
        let alan_cagri = test.alan_kimligi.clone();
        let kok_cagri = test.proje_koku.clone();
        let cagri_gorevi = tokio::spawn(async move {
            yonetici_cagri
                .arac_cagir(
                    &alan_cagri,
                    &kok_cagri,
                    "harici",
                    &tarif,
                    "uzun_is",
                    JsonObject::new(),
                )
                .await
        });

        cagri_basladi.notified().await;

        // cagri_beklet hiç tetiklenmediği halde kontrol kanalı (sunucuyu_durdur)
        // bloklanmadan hemen döner ve çağrıyı iptal eder!
        test.yonetici.sunucuyu_durdur(&test.alan_kimligi, "harici");
        let cagri_sonucu = tokio::time::timeout(Duration::from_secs(2), cagri_gorevi)
            .await
            .expect("kontrol kanalı uzun çağrıyı hemen iptal etmeli")
            .unwrap();
        assert_eq!(cagri_sonucu, Err(YoneticiHatasi::SunucuDurduruldu));

        // Süreç kapatma aşamasında bekliyor: bu sırada sunucu_kokunu_temizle
        // çağrıldığında kapat_beklet serbest bırakılana kadar asla kök silinmez!
        kapat_basladi.notified().await;
        let yonetici_sil = test.yonetici.clone();
        let alan_sil = test.alan_kimligi.clone();
        let kok_sil = test.proje_koku.clone();
        let sil_gorevi = tokio::spawn(async move {
            yonetici_sil
                .sunucu_kokunu_temizle(&kok_sil, &alan_sil, "harici")
                .await
        });

        tokio::task::yield_now().await;
        assert!(
            test.yalitici
                .silinen_sunucu_kokleri
                .lock()
                .unwrap()
                .is_empty(),
            "kapanış doğrulanmadan sunucu kökü silinmemeli"
        );

        kapat_beklet.notify_one();
        sil_gorevi.await.unwrap().unwrap();
        assert!(
            !test
                .yalitici
                .kapanis_bitmeden_kok_silindi
                .load(Ordering::SeqCst),
            "etkin süreç varken sunucu kökü silinmemeli"
        );
        assert_eq!(
            test.yalitici.silinen_sunucu_kokleri.lock().unwrap().len(),
            1
        );
    }

    #[tokio::test]
    async fn uzun_rpc_cikis_izleyicisini_bekletmez() {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();
        let cagri_basladi = Arc::new(Notify::new());
        *test.baslatici.cagri_basladi.lock().unwrap() = Some(cagri_basladi.clone());
        *test.baslatici.cagri_beklet.lock().unwrap() = Some(Arc::new(Notify::new()));
        let (_, tarif) = test.onayla_ve_baslat("harici").await;
        let yonetici = test.yonetici.clone();
        let alan = test.alan_kimligi.clone();
        let kok = test.proje_koku.clone();
        let cagri = tokio::spawn(async move {
            yonetici
                .arac_cagir(&alan, &kok, "harici", &tarif, "uzun", JsonObject::new())
                .await
        });
        cagri_basladi.notified().await;
        test.baslatici.cikis_bildirimi.notify_one();
        let sonuc = tokio::time::timeout(Duration::from_secs(2), cagri).await;
        assert_eq!(
            sonuc
                .expect("çıkış izleyicisi RPC kilidini beklememeli")
                .unwrap(),
            Err(YoneticiHatasi::SunucuCoktu)
        );
        test.yonetici
            .sunucuyu_durdur_ve_bekle(&test.alan_kimligi, "harici")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn birakilan_cagri_bosta_kapanmayi_engellemez() {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();
        let cagri_basladi = Arc::new(Notify::new());
        *test.baslatici.cagri_basladi.lock().unwrap() = Some(cagri_basladi.clone());
        *test.baslatici.cagri_beklet.lock().unwrap() = Some(Arc::new(Notify::new()));
        let (_, tarif) = test.onayla_ve_baslat("harici").await;
        let yonetici = test.yonetici.clone();
        let alan = test.alan_kimligi.clone();
        let kok = test.proje_koku.clone();
        let cagri = tokio::spawn(async move {
            yonetici
                .arac_cagir(&alan, &kok, "harici", &tarif, "uzun", JsonObject::new())
                .await
        });
        cagri_basladi.notified().await;
        cagri.abort();
        assert!(cagri.await.unwrap_err().is_cancelled());
        test.saat.ilerlet(BOSTA_KAPANMA_SURESI);
        test.yonetici.zamanlayici_adimi().await;
        assert_eq!(
            test.yonetici.sunucu_durumu(&test.alan_kimligi, "harici"),
            McpSunucuDurumu::BostaKapandi
        );
    }

    #[tokio::test(start_paused = true)]
    async fn mutlak_denetim_kendiliginden_calisir() {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();
        test.onayla_ve_baslat("harici").await;
        test.yonetici
            .onay_bekliyor_ayarla(&test.alan_kimligi, "harici", true);
        let giris = Arc::new(Notify::new());
        *test.koruma.dogrula_giris_bildirimi.lock().unwrap() = Some(giris.clone());
        tokio::task::yield_now().await;
        test.saat.ilerlet(MUTLAK_KORUMA_YASI);
        tokio::time::advance(MUTLAK_KORUMA_YASI).await;
        tokio::time::timeout(Duration::from_secs(1), giris.notified())
            .await
            .expect("üretim zamanlayıcısı çağrıdan bağımsız denetlemeli");
        test.yonetici
            .sunucuyu_durdur_ve_bekle(&test.alan_kimligi, "harici")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn el_sikisma_gelecegi_birakilinca_kok_dogrulanan_kapanisi_bekler() {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();
        let kart = test
            .yonetici
            .baslatma_karti_olustur(&test.alan_kimligi, &test.proje_koku, "harici")
            .unwrap();
        test.yonetici
            .baslatma_izni_yanitla(
                &kart.istek_kimligi,
                &test.alan_kimligi,
                &test.proje_koku,
                true,
            )
            .await
            .unwrap();
        let hazirlan = Arc::new(Notify::new());
        let kapanis = Arc::new(Notify::new());
        let kapanis_basladi = Arc::new(Notify::new());
        *test.baslatici.hazirlan_basladi.lock().unwrap() = Some(hazirlan.clone());
        *test.baslatici.hazirlan_beklet.lock().unwrap() = Some(Arc::new(Notify::new()));
        *test.baslatici.kapat_beklet.lock().unwrap() = Some(kapanis.clone());
        *test.baslatici.kapat_basladi.lock().unwrap() = Some(kapanis_basladi.clone());
        let y = test.yonetici.clone();
        let a = test.alan_kimligi.clone();
        let k = test.proje_koku.clone();
        let baslat = tokio::spawn(async move { y.baslat_veya_dogrula(&a, &k, "harici").await });
        hazirlan.notified().await;
        assert_eq!(test.baslatici.etkin_surec_sayisi.load(Ordering::SeqCst), 1);
        assert_eq!(
            test.yonetici
                .baslat_veya_dogrula(&test.alan_kimligi, &test.proje_koku, "harici")
                .await,
            Err(YoneticiHatasi::BaslatmaYarisi)
        );
        baslat.abort();
        assert!(baslat.await.unwrap_err().is_cancelled());
        tokio::time::timeout(Duration::from_secs(2), kapanis_basladi.notified())
            .await
            .unwrap();
        let y = test.yonetici.clone();
        let a = test.alan_kimligi.clone();
        let k = test.proje_koku.clone();
        let sil = tokio::spawn(async move { y.sunucu_kokunu_temizle(&k, &a, "harici").await });
        tokio::task::yield_now().await;
        assert!(!sil.is_finished());
        assert!(
            test.yalitici
                .silinen_sunucu_kokleri
                .lock()
                .unwrap()
                .is_empty()
        );
        kapanis.notify_one();
        tokio::time::timeout(Duration::from_secs(2), sil)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(test.baslatici.etkin_surec_sayisi.load(Ordering::SeqCst), 0);
        assert!(
            !test
                .yalitici
                .kapanis_bitmeden_kok_silindi
                .load(Ordering::SeqCst)
        );
    }

    #[tokio::test]
    async fn taramada_durdurma_baslatma_rezervasyonunu_gecersiz_kilar() {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();
        let kart = test
            .yonetici
            .baslatma_karti_olustur(&test.alan_kimligi, &test.proje_koku, "harici")
            .unwrap();
        test.yonetici
            .baslatma_izni_yanitla(
                &kart.istek_kimligi,
                &test.alan_kimligi,
                &test.proje_koku,
                true,
            )
            .await
            .unwrap();
        let giris = Arc::new(Notify::new());
        let devam = Arc::new(Notify::new());
        *test.koruma.dogrula_giris_bildirimi.lock().unwrap() = Some(giris.clone());
        *test.koruma.dogrula_devam_bildirimi.lock().unwrap() = Some(devam.clone());
        let y = test.yonetici.clone();
        let a = test.alan_kimligi.clone();
        let k = test.proje_koku.clone();
        let baslat = tokio::spawn(async move { y.baslat_veya_dogrula(&a, &k, "harici").await });
        giris.notified().await;
        test.yonetici.sunucuyu_durdur(&test.alan_kimligi, "harici");
        devam.notify_one();
        assert_eq!(baslat.await.unwrap(), Err(YoneticiHatasi::BaslatmaYarisi));
        assert_eq!(test.baslatici.sayac(), 0);
    }

    #[tokio::test]
    async fn eszamanli_baslatmalar_tek_surec_olusturur() {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();
        let kart = test
            .yonetici
            .baslatma_karti_olustur(&test.alan_kimligi, &test.proje_koku, "harici")
            .unwrap();
        test.yonetici
            .baslatma_izni_yanitla(
                &kart.istek_kimligi,
                &test.alan_kimligi,
                &test.proje_koku,
                true,
            )
            .await
            .unwrap();
        let devam = Arc::new(Notify::new());
        *test.koruma.dogrula_devam_bildirimi.lock().unwrap() = Some(devam.clone());
        let mut gorevler = Vec::new();
        for _ in 0..2 {
            let y = test.yonetici.clone();
            let a = test.alan_kimligi.clone();
            let k = test.proje_koku.clone();
            gorevler.push(tokio::spawn(async move {
                y.baslat_veya_dogrula(&a, &k, "harici").await
            }));
        }
        tokio::time::timeout(Duration::from_secs(2), async {
            while test.koruma.dogrula_sayisi.load(Ordering::SeqCst) != 2 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        devam.notify_waiters();
        let mut basarili = 0;
        for gorev in gorevler {
            if gorev.await.unwrap().is_ok() {
                basarili += 1;
            }
        }
        assert_eq!(basarili, 1);
        assert_eq!(test.baslatici.sayac(), 1);
        test.yonetici
            .sunucuyu_durdur_ve_bekle(&test.alan_kimligi, "harici")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn hazirlama_korumali_sunucu_kokunu_olusturmaz() {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
        let ozel = kalfa_sandbox::sunucu_ozel(&test.yalitim_dizini, &test.alan_kimligi, "harici");
        assert!(!ozel.kok.exists());
        test.yonetici
            .baslatma_karti_olustur(&test.alan_kimligi, &test.proje_koku, "harici")
            .unwrap();
        assert!(
            !ozel.kok.exists(),
            "Windows kökünü yalnız yalıtım arka ucu mirassız kurabilir"
        );
    }

    #[tokio::test]
    async fn eski_baslatmanin_geciken_iptali_yeni_kusagi_durdurmaz() {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();
        let kart = test
            .yonetici
            .baslatma_karti_olustur(&test.alan_kimligi, &test.proje_koku, "harici")
            .unwrap();
        test.yonetici
            .baslatma_izni_yanitla(
                &kart.istek_kimligi,
                &test.alan_kimligi,
                &test.proje_koku,
                true,
            )
            .await
            .unwrap();
        *test.baslatici.hazirlan_beklet.lock().unwrap() = Some(Arc::new(Notify::new()));
        let mut eski = Box::pin(test.yonetici.baslat_veya_dogrula(
            &test.alan_kimligi,
            &test.proje_koku,
            "harici",
        ));
        std::future::poll_fn(|cx| {
            assert!(eski.as_mut().poll(cx).is_pending());
            if test.baslatici.etkin_surec_sayisi.load(Ordering::SeqCst) == 1 {
                std::task::Poll::Ready(())
            } else {
                std::task::Poll::Pending
            }
        })
        .await;
        // Eski hazırlık geleceği yeniden poll edilmeden gerçek kapanış ilerler.
        test.yonetici
            .sunucuyu_durdur_ve_bekle(&test.alan_kimligi, "harici")
            .await
            .unwrap();
        *test.baslatici.hazirlan_beklet.lock().unwrap() = None;
        let (yeni, _) = test.onayla_ve_baslat("harici").await;
        assert_eq!(yeni, McpBaslatmaSayaci::yeni(2));
        assert_eq!(eski.await, Err(YoneticiHatasi::SunucuDurduruldu));
        assert!(
            matches!(test.yonetici.sunucu_durumu(&test.alan_kimligi, "harici"), McpSunucuDurumu::Calisiyor { kusak, .. } if kusak == yeni)
        );
        assert_eq!(test.baslatici.etkin_surec_sayisi.load(Ordering::SeqCst), 1);
        test.yonetici
            .sunucuyu_durdur_ve_bekle(&test.alan_kimligi, "harici")
            .await
            .unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn cokme_sonrasi_yeniden_baslatma_yeni_istek_beklemez() {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();
        let (kusak, _) = test.onayla_ve_baslat("harici").await;
        test.yonetici.beklenmeyen_cikis_bildir(
            &test.alan_kimligi,
            "harici",
            kusak,
            &test.proje_koku,
        );
        test.yonetici
            .yuva_kapanislarini_bekle(&McpAnahtari::yeni(&test.alan_kimligi, "harici"))
            .await;
        test.saat.ilerlet(Duration::from_secs(1));
        tokio::time::advance(Duration::from_secs(1)).await;
        tokio::time::timeout(Duration::from_secs(1), async {
            while test.baslatici.sayac() != 2 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("çökme sonrası yeniden başlatma kendiliğinden yürümeli");
        test.yonetici
            .sunucuyu_durdur_ve_bekle(&test.alan_kimligi, "harici")
            .await
            .unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn takilan_kapanis_diger_sunucunun_mutlak_denetimini_bekletmez() {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("a", McpKipi::Yalitimli);
        test.harici_sunucu_ekle("b", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();
        let kapanis = Arc::new(Notify::new());
        let kapanis_basladi = Arc::new(Notify::new());
        *test.baslatici.kapat_beklet.lock().unwrap() = Some(kapanis.clone());
        *test.baslatici.kapat_basladi.lock().unwrap() = Some(kapanis_basladi.clone());
        test.onayla_ve_baslat("a").await;
        *test.baslatici.kapat_beklet.lock().unwrap() = None;
        *test.baslatici.kapat_basladi.lock().unwrap() = None;
        test.onayla_ve_baslat("b").await;
        test.yonetici
            .onay_bekliyor_ayarla(&test.alan_kimligi, "b", true);
        let giris = Arc::new(Notify::new());
        *test.koruma.dogrula_giris_bildirimi.lock().unwrap() = Some(giris.clone());
        tokio::task::yield_now().await;
        test.saat.ilerlet(MUTLAK_KORUMA_YASI);
        tokio::time::advance(MUTLAK_KORUMA_YASI).await;
        tokio::time::timeout(Duration::from_secs(1), kapanis_basladi.notified())
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(1), giris.notified())
            .await
            .expect("öteki sunucu aynı mutlak yaşta denetlenmeli");
        assert_eq!(test.baslatici.etkin_surec_sayisi.load(Ordering::SeqCst), 2);
        kapanis.notify_one();
        test.yonetici
            .sunucuyu_durdur_ve_bekle(&test.alan_kimligi, "a")
            .await
            .unwrap();
        test.yonetici
            .sunucuyu_durdur_ve_bekle(&test.alan_kimligi, "b")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn ayni_calisma_alaninda_iki_sunucu_ve_kabuk_birbirini_bozmaz() {
        let test = TestOrtami::kur();
        test.harici_sunucu_ekle("sunucu-a", McpKipi::Yalitimli);
        test.harici_sunucu_ekle("sunucu-b", McpKipi::Yalitimli);
        let _oturum = test.siki_oturum_ac();

        let (kusak_a, tarif_a) = test.onayla_ve_baslat("sunucu-a").await;
        let (kusak_b, tarif_b) = test.onayla_ve_baslat("sunucu-b").await;
        assert_eq!(kusak_a, McpBaslatmaSayaci::yeni(1));
        assert_eq!(kusak_b, McpBaslatmaSayaci::yeni(1));
        assert_ne!(tarif_a, tarif_b);

        // sunucu-a durdurulduğunda sunucu-b çalışmaya ve çağrı kabul etmeye devam eder:
        test.yonetici
            .sunucuyu_durdur_ve_bekle(&test.alan_kimligi, "sunucu-a")
            .await
            .unwrap();
        assert_eq!(
            test.yonetici.sunucu_durumu(&test.alan_kimligi, "sunucu-a"),
            McpSunucuDurumu::Durdu
        );
        assert!(matches!(
            test.yonetici.sunucu_durumu(&test.alan_kimligi, "sunucu-b"),
            McpSunucuDurumu::Calisiyor { .. }
        ));
        test.yonetici
            .arac_cagir(
                &test.alan_kimligi,
                &test.proje_koku,
                "sunucu-b",
                &tarif_b,
                "kontrol",
                JsonObject::new(),
            )
            .await
            .unwrap();
    }

    /// Görev 9: alan planlayıcısının başlat → keşif → `McpArac` → yayım zinciri.
    mod alan_baglantisi {
        use super::*;
        use crate::mcp::baglanti::McpAlanBaglantisi;
        use crate::mcp::denetim::McpTanimDenetleyicisi;
        use crate::mcp::kaynak::McpAlanAracKaynagi;
        use kalfa_core::arac::{AracDenetimSahibi, AracKaynagi};
        use kalfa_core::butce::ButceTalepci;
        use kalfa_core::denetci::DenetimKipi;
        use kalfa_sandbox::tarama::CancellationToken;

        fn tool(ad: &str) -> Tool {
            let sema = serde_json::json!({"type": "object"});
            Tool::new(
                ad.to_owned(),
                "okur".to_owned(),
                Arc::new(sema.as_object().cloned().unwrap()),
            )
        }

        fn kaynak_kur(test: &TestOrtami) -> Arc<McpAlanAracKaynagi> {
            kaynak_kur_alan(test, &test.alan_kimligi, &test.proje_koku)
        }

        fn kaynak_kur_alan(
            test: &TestOrtami,
            alan_kimligi: &str,
            kok: &Path,
        ) -> Arc<McpAlanAracKaynagi> {
            let denetleyici = McpTanimDenetleyicisi::yeni(
                test.veri_dizini.clone(),
                test.ayarlar.clone(),
                test.yonetici.clone(),
            );
            let baglanti = McpAlanBaglantisi::yeni(
                alan_kimligi.to_owned(),
                kok.to_path_buf(),
                test.yonetici.clone(),
                denetleyici,
                test.ayarlar.clone(),
                Arc::new(kalfa_core::denetci::Yok),
                Arc::new(std::sync::RwLock::new(DenetimKipi::Kapali)),
                None,
            );
            let kaynak = McpAlanAracKaynagi::yeni();
            kaynak.planlayici_yoksa_ayarla(baglanti.planlayici());
            kaynak
        }

        /// Bir model turunun başı: sahiplen ve işleri bitir.
        async fn tur(kaynak: &McpAlanAracKaynagi, oturum: OturumId) -> Vec<String> {
            let (butce, _alici) = ButceTalepci::kanal();
            let sahip = AracDenetimSahibi {
                oturum,
                butce,
                iptal: CancellationToken::new(),
            };
            for is in kaynak.denetim_islerini_al(sahip) {
                is.await;
            }
            kaynak
                .hazir_araclar()
                .iter()
                .map(|a| a.ad().to_owned())
                .collect()
        }

        fn etkinlik(test: &TestOrtami, sunucu: &str, etkin: bool) {
            let (mut ayar, rev) = test.ayarlar.mcp_ayarlarini_oku();
            for s in &mut ayar.sunucular {
                if s.ad == sunucu {
                    s.etkin = etkin;
                }
            }
            test.ayarlar.mcp_ayarlarini_kaydet(&ayar, rev).unwrap();
        }

        #[tokio::test]
        async fn izinli_sunucu_turda_baslar_kesfedilir_ve_yayimlanir() {
            let test = TestOrtami::kur();
            test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
            *test.baslatici.araclar.lock().unwrap() = vec![tool("oku")];
            let oturum = test.siki_oturum_ac();
            // Başlatma izni (yerel kart) verilir; süreç sonra durdurulur ki
            // planlayıcı onu kendisi başlatsın.
            test.onayla_ve_baslat("harici").await;
            test.yonetici
                .sunucuyu_durdur_ve_bekle(&test.alan_kimligi, "harici")
                .await
                .unwrap();
            let kaynak = kaynak_kur(&test);

            assert_eq!(tur(&kaynak, oturum).await, vec!["mcp__harici__oku"]);
            assert_eq!(test.baslatici.sayac(), 2, "planlayıcı süreci başlattı");

            // Değişiklik yoksa aynı yayım kalır; süreç yeniden başlamaz.
            let surum = kaynak.surum();
            assert_eq!(tur(&kaynak, oturum).await, vec!["mcp__harici__oku"]);
            assert_eq!(kaynak.surum(), surum);
            assert_eq!(test.baslatici.sayac(), 2);

            // Kapatılan sunucunun araçları sonraki turda düşer.
            etkinlik(&test, "harici", false);
            assert!(tur(&kaynak, oturum).await.is_empty());
        }

        /// Başlatma izni geri alınınca ya da alan kaldırılırken sunucu
        /// başlamaz ve önceki yayımdaki araçları sonraki turda düşer.
        #[tokio::test]
        async fn izin_geri_alininca_ve_alan_kaldirilirken_araclar_duser() {
            let test = TestOrtami::kur();
            test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
            *test.baslatici.araclar.lock().unwrap() = vec![tool("oku")];
            let oturum = test.siki_oturum_ac();
            test.onayla_ve_baslat("harici").await;
            let kaynak = kaynak_kur(&test);
            assert_eq!(tur(&kaynak, oturum).await, vec!["mcp__harici__oku"]);

            test.yonetici
                .baslatma_izni_geri_al(&test.alan_kimligi, "harici");
            assert!(tur(&kaynak, oturum).await.is_empty());
            assert_eq!(test.baslatici.sayac(), 1, "izinsiz yeniden başlamaz");

            // İzin yeniden verilir; alan kaldırılırken yeni başlatma olmaz.
            test.onayla_ve_baslat("harici").await;
            assert_eq!(tur(&kaynak, oturum).await, vec!["mcp__harici__oku"]);
            let sayac = test.baslatici.sayac();
            test.yonetici
                .alan_kapanisini_baslat_ve_bekle(&test.alan_kimligi)
                .await
                .unwrap();
            assert!(tur(&kaynak, oturum).await.is_empty());
            assert_eq!(test.baslatici.sayac(), sayac);
        }

        /// Aynı tanım iki alanda ayrı süreçtir; bir alanın izni öbürüne geçmez.
        #[tokio::test]
        async fn iki_alan_ayri_izin_ve_ayri_yayim_tutar() {
            let test = TestOrtami::kur();
            test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
            *test.baslatici.araclar.lock().unwrap() = vec![tool("oku")];
            let oturum = test.siki_oturum_ac();
            test.onayla_ve_baslat("harici").await;
            let ikinci_kok = test.proje_koku.with_file_name("proje-iki");
            std::fs::create_dir_all(&ikinci_kok).unwrap();
            let ikinci_alan = CalismaAlaniKaydi::kimlik(&ikinci_kok);
            let ikinci_oturum = OturumId::yeni();
            test.yonetici.oturum_kipi_guncelle(
                &ikinci_alan,
                Some(&ikinci_kok),
                ikinci_oturum,
                Some(Kip::Siki),
            );

            let birinci = kaynak_kur(&test);
            let ikinci = kaynak_kur_alan(&test, &ikinci_alan, &ikinci_kok);
            assert_eq!(tur(&birinci, oturum).await, vec!["mcp__harici__oku"]);
            assert!(
                tur(&ikinci, ikinci_oturum).await.is_empty(),
                "ikinci alan birincinin iznini kullanamaz"
            );
            assert_eq!(test.baslatici.sayac(), 1);
        }

        #[tokio::test]
        async fn izinsiz_sunucu_baslamaz_araci_gelmez_ve_turu_bekletmez() {
            let test = TestOrtami::kur();
            test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
            *test.baslatici.araclar.lock().unwrap() = vec![tool("oku")];
            let oturum = test.siki_oturum_ac();
            let kaynak = kaynak_kur(&test);

            let araclar = tokio::time::timeout(Duration::from_secs(5), tur(&kaynak, oturum))
                .await
                .expect("izinsiz sunucu turu bekletmemeli");
            assert!(araclar.is_empty());
            assert_eq!(test.baslatici.sayac(), 0, "izin yokken süreç başlamaz");

            // Her tur yeniden denenen başlatma tek bekleyen kartı yeniden kullanır.
            let ilk_kart: Vec<String> = test
                .yonetici
                .durum()
                .bekleyen_kartlar
                .keys()
                .cloned()
                .collect();
            tur(&kaynak, oturum).await;
            tur(&kaynak, oturum).await;
            let son_kart: Vec<String> = test
                .yonetici
                .durum()
                .bekleyen_kartlar
                .keys()
                .cloned()
                .collect();
            assert_eq!(ilk_kart.len(), 1);
            assert_eq!(son_kart, ilk_kart, "kartlar birikmez, kimlik korunur");
        }

        #[tokio::test]
        async fn yalniz_inceleme_oturumu_kalinca_sunucu_baslatilmaz() {
            let test = TestOrtami::kur();
            test.harici_sunucu_ekle("harici", McpKipi::Yalitimli);
            *test.baslatici.araclar.lock().unwrap() = vec![tool("oku")];
            let oturum = test.siki_oturum_ac();
            test.onayla_ve_baslat("harici").await;
            test.yonetici.oturum_kipi_guncelle(
                &test.alan_kimligi,
                Some(&test.proje_koku),
                oturum,
                Some(Kip::Inceleme),
            );
            let kaynak = kaynak_kur(&test);

            assert!(tur(&kaynak, oturum).await.is_empty());
            assert_eq!(test.baslatici.sayac(), 1, "yalnız izin anındaki başlatma");
        }
    }
}
