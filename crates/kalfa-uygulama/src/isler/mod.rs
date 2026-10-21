//! Arka plan süreçleri ve etkileşimli terminal işleri (`G2b` / `G2c`).
//!
//! Tasarım ve güvenlik sınırları (`docs/superpowers/plans/2026-10-20-kalfa-yetenek-03-surec-terminal.md`,
//! `docs/superpowers/bulgular/yetenek-programi/surec-deneyi.md`):
//! - İş kimliğini (`IsId`) uygulama üretir (`is-<uuid>`). Ham PID, dosya yolu veya
//!   başka oturumun kimliği yetki sayılmaz; her işlem `(OturumId, IsId)` çiftini
//!   doğrular.
//! - Tek `IsKaydi` süreci (`EtkilesimliSurec`), çalışma alanı yazma kilidini
//!   (`KilitSeti`), imleçli çıktı tamponunu ve kapanış son denetimini (`SonDenetimKancasi`)
//!   sahiplenir. Araç çağrısının erken dönmesi işi veya kilidi sahipsiz bırakmaz.
//! - Süreç ağacı gerçekten kapanmadan ve son dosya denetimi (`komut_sonrasi_denetim`
//!   ile `yazma_bitisini_bekle`) tamamlanmadan `KilitSeti` serbest bırakılmaz.
//!   Belirsiz kapanış veya başarısız karantina durumunda durum `SonucuBilinmiyor`
//!   olur ve `KilitSeti` karantinada tutulur (yeni yazma işini sessizce açmaz).

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use kalfa_core::kilit::KilitSeti;
use kalfa_core::kimlik::OturumId;
use kalfa_sandbox::tarama::AdliGirdiler;
use kalfa_sandbox::{
    CalistirmaIstegi, EtkilesimliSurec, TerminalBoyutu, TerminalDenetimi, Yalitici,
};
use kalfa_tools::CalismaAlani;
use kalfa_tools::koruma::{self, SonDenetimSonucu};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::Notify;
use uuid::Uuid;

type Okuyucu = Box<dyn AsyncRead + Send + Unpin>;
type Yazici = Box<dyn AsyncWrite + Send + Unpin>;

/// Oturum başına aynı anda çalışabilecek azami etkin arka plan işi sayısı.
pub const AZAMI_ETKIN_IS_OTURUM_BASI: usize = 4;

/// İş başına bellekte tutulan azami çıktı baytı (`1 MiB`).
/// Sınır aşıldığında boru EOF'a kadar boşaltılmaya devam eder ve `tasti = true`
/// işaretlenir; çocuk süreç dolu boruda bloklanmaz.
pub const AZAMI_IS_CIKTISI_BAYT: usize = 1_048_576;

/// Tek `komut_girdi` çağrısında gönderilebilecek azami bayt (`64 KiB`).
pub const AZAMI_IS_GIRDI_BAYT: usize = 65_536;

/// Tek arka plan işinin azami toplam ömrü (`30 dk`).
pub const AZAMI_IS_OMRU: Duration = Duration::from_secs(1800);

/// Stdin okumayan çocuk süreçte `girdi` çağrısının bloklanabileceği azami süre.
pub const GIRDI_YAZMA_SURESI: Duration = Duration::from_secs(2);

/// Süreç kapandıktan sonra çıktı borularının EOF'a ulaşması için beklenen süre.
const CIKTI_BOSALMA_SURESI: Duration = Duration::from_millis(500);

/// Uygulamanın ürettiği opak iş kimliği (`is-<uuid>`).
/// Ham PID veya dosya yolu iş kimliği olarak kabul edilmez.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IsId(String);

impl IsId {
    pub fn yeni() -> Self {
        Self(format!("is-{}", Uuid::new_v4()))
    }

    pub fn coz(metin: &str) -> Result<Self, IsHatasi> {
        let Some(kuyruk) = metin.strip_prefix("is-") else {
            return Err(IsHatasi::GecersizKimlik(metin.to_string()));
        };
        let Ok(uuid) = Uuid::parse_str(kuyruk) else {
            return Err(IsHatasi::GecersizKimlik(metin.to_string()));
        };
        Ok(Self(format!("is-{uuid}")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for IsId {
    fn default() -> Self {
        Self::yeni()
    }
}

impl fmt::Display for IsId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Arka plan işinin G/Ç kipi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tur", rename_all = "snake_case")]
pub enum IsKipi {
    Borulu,
    Terminal { boyut: TerminalBoyutu },
}

impl IsKipi {
    pub fn terminal_boyutu(&self) -> Option<TerminalBoyutu> {
        match self {
            Self::Borulu => None,
            Self::Terminal { boyut } => Some(*boyut),
        }
    }
}

/// İşin yaşam döngüsü durumu.
/// Başlatıldı (`Calisiyor`) kaydı bitiş (`Bitti`) yerine geçmez; belirsiz kapanış
/// `SonucuBilinmiyor` olarak ayrılır.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "durum", rename_all = "snake_case")]
pub enum IsDurumu {
    Calisiyor,
    Durduruluyor,
    Bitti {
        cikis_kodu: Option<i32>,
        sure_ms: u64,
        zaman_asimi: bool,
    },
    SonucuBilinmiyor {
        neden: String,
    },
}

impl IsDurumu {
    pub fn etkin_mi(&self) -> bool {
        matches!(self, Self::Calisiyor | Self::Durduruluyor)
    }

    pub fn bitti_mi(&self) -> bool {
        !self.etkin_mi()
    }

    pub fn etiket(&self) -> &'static str {
        match self {
            Self::Calisiyor => "çalışıyor",
            Self::Durduruluyor => "durduruluyor",
            Self::Bitti { .. } => "bitti",
            Self::SonucuBilinmiyor { .. } => "sonucu bilinmiyor",
        }
    }
}

/// İmleçli çıktı okuma sonucu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsOkumaSonucu {
    pub is_id: IsId,
    pub gosterim: String,
    pub kip: IsKipi,
    pub durum: IsDurumu,
    pub cikti: String,
    pub yeni_imlec: usize,
    pub toplam_bayt: u64,
    pub tasti: bool,
    pub uyarilar: Vec<String>,
}

/// İş özeti (kart ve durum sorguları için).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsOzeti {
    pub is_id: IsId,
    pub oturum: OturumId,
    pub gosterim: String,
    pub kip: IsKipi,
    pub durum: IsDurumu,
    pub toplam_bayt: u64,
    pub tasti: bool,
    pub uyarilar: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IsHatasi {
    #[error("Geçersiz iş kimliği: `{0}`.")]
    GecersizKimlik(String),
    #[error("İş bulunamadı: `{0}`.")]
    IsBulunamadi(IsId),
    #[error("Bu iş başka bir oturuma ait.")]
    YetkisizOturum,
    #[error("Oturum başına azami etkin iş sınırına ({sinir}) ulaşıldı.")]
    EtkinIsSiniriAsildi { sinir: usize },
    #[error("Oturum kapanıyor veya kapalı.")]
    OturumKapali,
    #[error("İş artık çalışmıyor veya durduruluyor.")]
    IsKapandi,
    #[error("Standart girdi çok büyük ({boyut} bayt > {azami} bayt).")]
    GirdiCokBuyuk { boyut: usize, azami: usize },
    #[error("Standart girdi kapalı.")]
    GirdiKapali,
    #[error("Standart girdiye yazma zaman aşımına uğradı.")]
    GirdiZamanAsimi,
    #[error("Standart girdiye yazılamadı: {0}")]
    GirdiHatasi(String),
    #[error("Bu iş sözde terminal (PTY/ConPTY) kipinde değil.")]
    TerminalDegil,
    #[error("Terminal boyutu değiştirilemedi: {0}")]
    BoyutHatasi(String),
    #[error("Süreç başlatılamadı: {0}")]
    Baslatma(String),
    #[error("İş temiz biçimde kapatılamadı: {0}")]
    KapanisBelirsiz(String),
}

/// İş kapandığında kilit bırakılmadan önce çalıştırılan son denetim kancası.
#[async_trait]
pub trait SonDenetimKancasi: Send + Sync {
    async fn denetle_ve_bitir(&self) -> Result<SonDenetimSonucu, String>;
}

/// Gerçek çalışma alanı için `.git`/yapılandırma son taramasını ve yazma kuşağı
/// bitişini yürüten kanca.
pub struct AlanSonDenetimi {
    alan: Arc<CalismaAlani>,
    onceki: Mutex<Option<AdliGirdiler>>,
    yazma_bildirildi: AtomicBool,
}

impl AlanSonDenetimi {
    /// Çalışma alanında yazma başladığını bildirerek son denetim kancasını kurar.
    pub fn baslat(alan: Arc<CalismaAlani>, onceki: AdliGirdiler) -> Arc<Self> {
        alan.yazma_basladi_bildir();
        Arc::new(Self {
            alan,
            onceki: Mutex::new(Some(onceki)),
            yazma_bildirildi: AtomicBool::new(true),
        })
    }

    /// Süreç başlatılamadığında yazma kuşağını hemen kapatır.
    pub async fn baslatma_hatasinda_kapat(&self) -> Result<(), String> {
        if self.yazma_bildirildi.swap(false, Ordering::SeqCst) {
            self.alan.yazma_bitti_bildir();
            self.alan
                .yazma_bitisini_bekle()
                .await
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

impl Drop for AlanSonDenetimi {
    fn drop(&mut self) {
        if self.yazma_bildirildi.swap(false, Ordering::SeqCst) {
            self.alan.yazma_bitti_bildir();
        }
    }
}

#[async_trait]
impl SonDenetimKancasi for AlanSonDenetimi {
    async fn denetle_ve_bitir(&self) -> Result<SonDenetimSonucu, String> {
        let onceki = self
            .onceki
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .take()
            .ok_or_else(|| "son denetim zaten çalıştırıldı".to_string())?;
        let sonuc = koruma::komut_sonrasi_denetim(self.alan.clone(), onceki).await;
        if self.yazma_bildirildi.swap(false, Ordering::SeqCst) {
            self.alan.yazma_bitti_bildir();
        }
        self.alan
            .yazma_bitisini_bekle()
            .await
            .map_err(|e| e.to_string())?;
        Ok(sonuc)
    }
}

/// Denetim gerektirmeyen birim testler için boş son denetim kancası.
pub struct BosSonDenetim;

#[async_trait]
impl SonDenetimKancasi for BosSonDenetim {
    async fn denetle_ve_bitir(&self) -> Result<SonDenetimSonucu, String> {
        Ok(SonDenetimSonucu {
            uyarilar: Vec::new(),
            karantina_basarisiz: false,
            tarama_tam: true,
            degisiklikler: None,
            guncel_adli_girdiler: None,
        })
    }
}

/// Yeni arka plan işi başlatma parametreleri.
pub struct IsBaslatma {
    pub oturum: OturumId,
    pub gosterim: String,
    pub dizin: String,
    pub ag: bool,
    pub ag_hostlari: Vec<String>,
    pub yalitimli: bool,
    pub kip: IsKipi,
    pub kilit: KilitSeti,
    pub son_denetim: Arc<dyn SonDenetimKancasi>,
    pub omur: Duration,
}

impl IsBaslatma {
    pub fn yeni(
        oturum: OturumId,
        gosterim: impl Into<String>,
        kip: IsKipi,
        kilit: KilitSeti,
        son_denetim: Arc<dyn SonDenetimKancasi>,
    ) -> Self {
        Self {
            oturum,
            gosterim: gosterim.into(),
            dizin: ".".to_string(),
            ag: false,
            ag_hostlari: Vec::new(),
            yalitimli: false,
            kip,
            kilit,
            son_denetim,
            omur: AZAMI_IS_OMRU,
        }
    }
}

struct CiktiTamponu {
    veri: Vec<u8>,
    toplam_bayt: u64,
    tasti: bool,
    akis_bitti: bool,
    sinir: usize,
}

impl CiktiTamponu {
    fn yeni(sinir: usize) -> Self {
        Self {
            veri: Vec::new(),
            toplam_bayt: 0,
            tasti: false,
            akis_bitti: false,
            sinir,
        }
    }

    fn ekle(&mut self, parca: &[u8]) {
        if parca.is_empty() {
            return;
        }
        self.toplam_bayt = self.toplam_bayt.saturating_add(parca.len() as u64);
        let kalan = self.sinir.saturating_sub(self.veri.len());
        if parca.len() <= kalan {
            self.veri.extend_from_slice(parca);
        } else {
            if kalan > 0 {
                self.veri.extend_from_slice(&parca[..kalan]);
            }
            self.tasti = true;
        }
    }

    fn oku(&self, imlec: usize) -> (String, usize, u64, bool) {
        let mut bas = imlec.min(self.veri.len());
        // Çağıran çok baytlı bir UTF-8 dizisinin ortasında bir ofset verdiyse
        // en yakın geçerli karakter sınırına geri çek.
        while bas > 0 && bas < self.veri.len() && (self.veri[bas] & 0b1100_0000) == 0b1000_0000 {
            bas -= 1;
        }
        let dilim = &self.veri[bas..];
        let okunabilir = utf8_okunabilir_uzunluk(dilim, self.akis_bitti);
        let yeni_imlec = bas + okunabilir;
        let metin = String::from_utf8_lossy(&dilim[..okunabilir]).into_owned();
        (metin, yeni_imlec, self.toplam_bayt, self.tasti)
    }
}

/// Akış henüz bitmemişken sondaki eksik (1–3 baytlık) UTF-8 karakter önekini
/// bir sonraki okumaya bırakır; böylece parça sınırına denk gelen Türkçe veya
/// çok baytlı karakterler `U+FFFD` bozulması üretmez.
fn utf8_okunabilir_uzunluk(dilim: &[u8], akis_bitti: bool) -> usize {
    if akis_bitti || dilim.is_empty() {
        return dilim.len();
    }
    let bas = dilim.len().saturating_sub(4);
    let mut alt = &dilim[bas..];
    while let Err(hata) = std::str::from_utf8(alt) {
        let gecerli = hata.valid_up_to();
        match hata.error_len() {
            Some(bozuk) => {
                alt = &alt[gecerli + bozuk..];
            }
            None => {
                let eksik = alt.len() - gecerli;
                return dilim.len() - eksik;
            }
        }
    }
    dilim.len()
}

struct IsDurumVerisi {
    durum: IsDurumu,
    kip: IsKipi,
    uyarilar: Vec<String>,
    /// Çalışma alanı genel yazma kilidi. Yalnız süreç ağacı doğrulamayla kapandıktan
    /// ve `SonDenetimKancasi` hatasız tamamlandıktan sonra `None` yapılarak düşürülür.
    kilit: Option<KilitSeti>,
}

struct IsKaydi {
    is_id: IsId,
    oturum: OturumId,
    gosterim: String,
    durum_verisi: Mutex<IsDurumVerisi>,
    cikti: Arc<Mutex<CiktiTamponu>>,
    girdi: Arc<tokio::sync::Mutex<Option<Yazici>>>,
    terminal: Option<TerminalDenetimi>,
    durdur_bildirimi: Notify,
    bitis_bildirimi: Notify,
    girdi_yazma_suresi: Duration,
    #[cfg(test)]
    zorla_kapanis_hatasi: AtomicBool,
}

impl IsKaydi {
    fn ozet(&self) -> IsOzeti {
        let (durum, kip, uyarilar) = {
            let d = self.durum_verisi.lock().unwrap_or_else(|z| z.into_inner());
            (d.durum.clone(), d.kip, d.uyarilar.clone())
        };
        let (toplam_bayt, tasti) = {
            let c = self.cikti.lock().unwrap_or_else(|z| z.into_inner());
            (c.toplam_bayt, c.tasti)
        };
        IsOzeti {
            is_id: self.is_id.clone(),
            oturum: self.oturum,
            gosterim: self.gosterim.clone(),
            kip,
            durum,
            toplam_bayt,
            tasti,
            uyarilar,
        }
    }

    fn oku(&self, imlec: usize) -> IsOkumaSonucu {
        let (durum, kip, uyarilar) = {
            let d = self.durum_verisi.lock().unwrap_or_else(|z| z.into_inner());
            (d.durum.clone(), d.kip, d.uyarilar.clone())
        };
        let (cikti, yeni_imlec, toplam_bayt, tasti) = {
            let c = self.cikti.lock().unwrap_or_else(|z| z.into_inner());
            c.oku(imlec)
        };
        IsOkumaSonucu {
            is_id: self.is_id.clone(),
            gosterim: self.gosterim.clone(),
            kip,
            durum,
            cikti,
            yeni_imlec,
            toplam_bayt,
            tasti,
            uyarilar,
        }
    }
}

/// Uygulama genelindeki arka plan işleri ve etkileşimli terminallerin yöneticisi.
pub struct IsYoneticisi {
    kayitlar: Mutex<HashMap<IsId, Arc<IsKaydi>>>,
    kurtarilanlar: Mutex<HashMap<IsId, IsOzeti>>,
    kapanan_oturumlar: Mutex<HashSet<OturumId>>,
    kayit_dizini: Option<std::path::PathBuf>,
    cikti_siniri: usize,
    girdi_yazma_suresi: Duration,
}

impl Default for IsYoneticisi {
    fn default() -> Self {
        Self::yeni()
    }
}

impl IsYoneticisi {
    pub fn yeni() -> Self {
        Self {
            kayitlar: Mutex::new(HashMap::new()),
            kurtarilanlar: Mutex::new(HashMap::new()),
            kapanan_oturumlar: Mutex::new(HashSet::new()),
            kayit_dizini: None,
            cikti_siniri: AZAMI_IS_CIKTISI_BAYT,
            girdi_yazma_suresi: GIRDI_YAZMA_SURESI,
        }
    }

    /// Oturum iş olaylarını (`<oturum>.isler.jsonl`) diske kaydeden ve
    /// yeniden açılışta çökme kurtarması yapan yönetici.
    pub fn yeni_kayitli(kayit_dizini: std::path::PathBuf) -> Self {
        Self {
            kayitlar: Mutex::new(HashMap::new()),
            kurtarilanlar: Mutex::new(HashMap::new()),
            kapanan_oturumlar: Mutex::new(HashSet::new()),
            kayit_dizini: Some(kayit_dizini),
            cikti_siniri: AZAMI_IS_CIKTISI_BAYT,
            girdi_yazma_suresi: GIRDI_YAZMA_SURESI,
        }
    }

    #[cfg(test)]
    fn yeni_sinirli(cikti_siniri: usize, girdi_yazma_suresi: Duration) -> Self {
        Self {
            kayitlar: Mutex::new(HashMap::new()),
            kurtarilanlar: Mutex::new(HashMap::new()),
            kapanan_oturumlar: Mutex::new(HashSet::new()),
            kayit_dizini: None,
            cikti_siniri,
            girdi_yazma_suresi,
        }
    }

    fn is_kayit_yolu(dizin: &std::path::Path, oturum: OturumId) -> std::path::PathBuf {
        dizin.join(format!("{oturum}.isler.jsonl"))
    }

    fn diske_kayit_olayi_yaz(dizin: &std::path::Path, oturum: OturumId, olay: &IsKayitOlayi) {
        use std::io::Write;
        let _ = std::fs::create_dir_all(dizin);
        let yol = Self::is_kayit_yolu(dizin, oturum);
        if let Ok(mut satir) = serde_json::to_vec(olay) {
            satir.push(b'\n');
            if let Ok(mut dosya) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(yol)
            {
                let _ = dosya.write_all(&satir);
                let _ = dosya.flush();
            }
        }
    }

    /// Oturum yeniden açıldığında kapanmış oturum işaretini kaldırır.
    pub fn oturumu_etkinlestir(&self, oturum: OturumId) {
        self.kapanan_oturumlar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .remove(&oturum);
    }

    /// Oturumun diskteki `<oturum>.isler.jsonl` kaydından geçmiş iş özetlerini
    /// kurtarır (`isler_kayittan_kurtar`). Kapanmamış işler eski PID'lere sinyal
    /// göndermeden `SonucuBilinmiyor` durumuna çekilir.
    pub fn oturumu_kurtar(&self, oturum: OturumId) -> Vec<IsOzeti> {
        let Some(dizin) = &self.kayit_dizini else {
            return Vec::new();
        };
        let yol = Self::is_kayit_yolu(dizin, oturum);
        let Ok(icerik) = std::fs::read_to_string(&yol) else {
            return Vec::new();
        };
        let satirlar: Vec<&str> = icerik.lines().collect();
        let kurtarilan = isler_kayittan_kurtar(oturum, satirlar);
        let etkin_kayitlar: HashSet<IsId> = self
            .kayitlar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .keys()
            .cloned()
            .collect();
        let mut tablo = self.kurtarilanlar.lock().unwrap_or_else(|z| z.into_inner());
        for ozet in &kurtarilan {
            if !etkin_kayitlar.contains(&ozet.is_id) {
                tablo.insert(ozet.is_id.clone(), ozet.clone());
            }
        }
        kurtarilan
    }

    /// Oturum silindiğinde iş kayıt dosyasını ve kurtarılmış özetleri temizler.
    pub fn oturum_kaydini_sil(&self, oturum: OturumId) {
        self.kayitlar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .retain(|_, k| k.oturum != oturum);
        self.kurtarilanlar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .retain(|_, o| o.oturum != oturum);
        if let Some(dizin) = &self.kayit_dizini {
            let _ = std::fs::remove_file(Self::is_kayit_yolu(dizin, oturum));
        }
    }

    /// Yalıtıcı üzerinden etkileşimli kabuk sürecini (borulu ya da PTY/ConPTY)
    /// başlatır ve sahipliği yeni bir `IsId` kaydına devreder.
    pub async fn yaliticidan_baslat(
        self: &Arc<Self>,
        yalitici: &dyn Yalitici,
        istek: &CalistirmaIstegi,
        baslatma: IsBaslatma,
    ) -> Result<IsId, IsHatasi> {
        self.baslatma_on_kontrol(baslatma.oturum)?;
        let surec = match yalitici.baslat_kabuk_etkilesimli(istek, baslatma.kip.terminal_boyutu()) {
            Ok(s) => s,
            Err(hata) => return Err(IsHatasi::Baslatma(hata.to_string())),
        };
        self.surecten_baslat(surec, baslatma)
    }

    /// Hazır bir `EtkilesimliSurec` örneğini iş tablosuna kaydeder ve yaşam döngüsü
    /// görevini başlatır.
    pub fn surecten_baslat(
        self: &Arc<Self>,
        mut surec: EtkilesimliSurec,
        baslatma: IsBaslatma,
    ) -> Result<IsId, IsHatasi> {
        let is_id = IsId::yeni();
        let girdi = surec.girdi_al();
        let cikti_okuyucu = surec.cikti_al();
        let hata_okuyucu = surec.hata_al();
        let terminal = surec.terminal_denetimi();
        let cikti_tamponu = Arc::new(Mutex::new(CiktiTamponu::yeni(self.cikti_siniri)));

        let baslat_olayi = IsKayitOlayi::IsBasladi {
            is_id: is_id.clone(),
            oturum: baslatma.oturum,
            komut: baslatma.gosterim.clone(),
            dizin: baslatma.dizin,
            ag: baslatma.ag,
            ag_hostlari: baslatma.ag_hostlari,
            yalitimli: baslatma.yalitimli,
            kip: baslatma.kip,
        };
        let kayit = Arc::new(IsKaydi {
            is_id: is_id.clone(),
            oturum: baslatma.oturum,
            gosterim: baslatma.gosterim,
            durum_verisi: Mutex::new(IsDurumVerisi {
                durum: IsDurumu::Calisiyor,
                kip: baslatma.kip,
                uyarilar: Vec::new(),
                kilit: Some(baslatma.kilit),
            }),
            cikti: cikti_tamponu.clone(),
            girdi: Arc::new(tokio::sync::Mutex::new(girdi)),
            terminal,
            durdur_bildirimi: Notify::new(),
            bitis_bildirimi: Notify::new(),
            girdi_yazma_suresi: self.girdi_yazma_suresi,
            #[cfg(test)]
            zorla_kapanis_hatasi: AtomicBool::new(false),
        });

        {
            let mut kilit_kapanan = self
                .kapanan_oturumlar
                .lock()
                .unwrap_or_else(|z| z.into_inner());
            if kilit_kapanan.contains(&baslatma.oturum) {
                let _ = &mut kilit_kapanan;
                return Err(IsHatasi::OturumKapali);
            }
            let mut tablo = self.kayitlar.lock().unwrap_or_else(|z| z.into_inner());
            let etkin = tablo
                .values()
                .filter(|k| {
                    k.oturum == baslatma.oturum
                        && k.durum_verisi
                            .lock()
                            .unwrap_or_else(|z| z.into_inner())
                            .durum
                            .etkin_mi()
                })
                .count();
            if etkin >= AZAMI_ETKIN_IS_OTURUM_BASI {
                return Err(IsHatasi::EtkinIsSiniriAsildi {
                    sinir: AZAMI_ETKIN_IS_OTURUM_BASI,
                });
            }
            tablo.insert(is_id.clone(), kayit.clone());
        }

        if let Some(dizin) = &self.kayit_dizini {
            Self::diske_kayit_olayi_yaz(dizin, baslatma.oturum, &baslat_olayi);
        }

        let cikti_gorevi = Self::cikti_okuma_baslat(cikti_okuyucu, hata_okuyucu, cikti_tamponu);
        Self::yasam_gorevi_baslat(
            kayit,
            surec,
            cikti_gorevi,
            baslatma.son_denetim,
            baslatma.omur,
            self.kayit_dizini.clone(),
        );
        Ok(is_id)
    }

    fn baslatma_on_kontrol(&self, oturum: OturumId) -> Result<(), IsHatasi> {
        if self
            .kapanan_oturumlar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .contains(&oturum)
        {
            return Err(IsHatasi::OturumKapali);
        }
        let tablo = self.kayitlar.lock().unwrap_or_else(|z| z.into_inner());
        let etkin = tablo
            .values()
            .filter(|k| {
                k.oturum == oturum
                    && k.durum_verisi
                        .lock()
                        .unwrap_or_else(|z| z.into_inner())
                        .durum
                        .etkin_mi()
            })
            .count();
        if etkin >= AZAMI_ETKIN_IS_OTURUM_BASI {
            return Err(IsHatasi::EtkinIsSiniriAsildi {
                sinir: AZAMI_ETKIN_IS_OTURUM_BASI,
            });
        }
        Ok(())
    }

    fn cikti_okuma_baslat(
        cikti: Option<Okuyucu>,
        hata: Option<Okuyucu>,
        tampon: Arc<Mutex<CiktiTamponu>>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let t1 = tampon.clone();
            let g1 = tokio::spawn(async move {
                if let Some(mut okuyucu) = cikti {
                    let mut buf = [0u8; 4096];
                    loop {
                        match okuyucu.read(&mut buf).await {
                            Ok(0) | Err(_) => break,
                            Ok(n) => {
                                t1.lock().unwrap_or_else(|z| z.into_inner()).ekle(&buf[..n]);
                            }
                        }
                    }
                }
            });
            let t2 = tampon.clone();
            let g2 = tokio::spawn(async move {
                if let Some(mut okuyucu) = hata {
                    let mut buf = [0u8; 4096];
                    loop {
                        match okuyucu.read(&mut buf).await {
                            Ok(0) | Err(_) => break,
                            Ok(n) => {
                                t2.lock().unwrap_or_else(|z| z.into_inner()).ekle(&buf[..n]);
                            }
                        }
                    }
                }
            });
            let _ = tokio::join!(g1, g2);
            tampon.lock().unwrap_or_else(|z| z.into_inner()).akis_bitti = true;
        })
    }

    fn yasam_gorevi_baslat(
        kayit: Arc<IsKaydi>,
        mut surec: EtkilesimliSurec,
        mut cikti_gorevi: tokio::task::JoinHandle<()>,
        son_denetim: Arc<dyn SonDenetimKancasi>,
        omur: Duration,
        kayit_dizini: Option<std::path::PathBuf>,
    ) {
        tokio::spawn(async move {
            let baslangic = Instant::now();
            let son_an = tokio::time::Instant::now() + omur;
            let (zaman_asimi, durduruldu) = tokio::select! {
                _ = surec.bitisi_bekle() => (false, false),
                _ = kayit.durdur_bildirimi.notified() => (false, true),
                _ = tokio::time::sleep_until(son_an) => (true, false),
            };

            {
                let mut d = kayit.durum_verisi.lock().unwrap_or_else(|z| z.into_inner());
                if matches!(d.durum, IsDurumu::Calisiyor) {
                    d.durum = IsDurumu::Durduruluyor;
                }
            }

            // Stdin yazıcısını hemen düşür: bekleyen/sonraki girdi reddedilsin.
            if let Ok(mut g) = kayit.girdi.try_lock() {
                let _ = g.take();
            }

            // Süreç ağacını zorunlu olarak sonlandır ve boşaldığını doğrula.
            let kapat_sonucu = surec.kapat_kadar(tokio::time::Instant::now()).await;
            #[cfg(test)]
            let kapat_sonucu = if kayit.zorla_kapanis_hatasi.load(Ordering::SeqCst) {
                Err(std::io::Error::other(
                    "test: süreç ağacı boşaltılamadı (benzetim)",
                ))
            } else {
                kapat_sonucu
            };

            // Çıktı borularının EOF'a ulaşmasını kısa süre bekle.
            if tokio::time::timeout(CIKTI_BOSALMA_SURESI, &mut cikti_gorevi)
                .await
                .is_err()
            {
                cikti_gorevi.abort();
                kayit
                    .cikti
                    .lock()
                    .unwrap_or_else(|z| z.into_inner())
                    .akis_bitti = true;
            }

            let sure_ms = u64::try_from(baslangic.elapsed().as_millis()).unwrap_or(u64::MAX);
            let kayit_olayi = match kapat_sonucu {
                Ok(kapanis) => match son_denetim.denetle_ve_bitir().await {
                    Ok(denetim) if denetim.tarama_tam && !denetim.karantina_basarisiz => {
                        let mut d = kayit.durum_verisi.lock().unwrap_or_else(|z| z.into_inner());
                        d.uyarilar = denetim.uyarilar;
                        d.durum = IsDurumu::Bitti {
                            cikis_kodu: kapanis.cikis,
                            sure_ms,
                            zaman_asimi,
                        };
                        // Yalnız ağaç doğrulandıktan ve son denetim temiz bittikten sonra
                        // çalışma alanı kilidi serbest bırakılır!
                        let _ = d.kilit.take();
                        if durduruldu && kapanis.cikis.is_none() {
                            IsKayitOlayi::IsIptalEdildi {
                                is_id: kayit.is_id.clone(),
                                sure_ms,
                            }
                        } else {
                            IsKayitOlayi::IsBitti {
                                is_id: kayit.is_id.clone(),
                                cikis_kodu: kapanis.cikis,
                                sure_ms,
                                zaman_asimi,
                            }
                        }
                    }
                    Ok(denetim) => {
                        let mut d = kayit.durum_verisi.lock().unwrap_or_else(|z| z.into_inner());
                        d.uyarilar = denetim.uyarilar.clone();
                        let neden = if denetim.uyarilar.is_empty() {
                            "Komut sonrası koruma taraması veya karantina tamamlanamadı."
                                .to_string()
                        } else {
                            denetim.uyarilar.join("; ")
                        };
                        d.durum = IsDurumu::SonucuBilinmiyor {
                            neden: neden.clone(),
                        };
                        // Karantina başarısızsa veya tarama eksikse kilit karantinada kalır.
                        IsKayitOlayi::IsSonucuBilinmiyor {
                            is_id: kayit.is_id.clone(),
                            neden,
                        }
                    }
                    Err(hata) => {
                        let mut d = kayit.durum_verisi.lock().unwrap_or_else(|z| z.into_inner());
                        d.durum = IsDurumu::SonucuBilinmiyor {
                            neden: hata.clone(),
                        };
                        IsKayitOlayi::IsSonucuBilinmiyor {
                            is_id: kayit.is_id.clone(),
                            neden: hata,
                        }
                    }
                },
                Err(hata) => {
                    // Süreç ağacı doğrulamayla kapanmadıysa KilitSeti serbest bırakılmaz!
                    let neden = format!("Süreç ağacı kapatılamadı: {hata}");
                    let mut d = kayit.durum_verisi.lock().unwrap_or_else(|z| z.into_inner());
                    d.durum = IsDurumu::SonucuBilinmiyor {
                        neden: neden.clone(),
                    };
                    IsKayitOlayi::IsSonucuBilinmiyor {
                        is_id: kayit.is_id.clone(),
                        neden,
                    }
                }
            };

            if let Some(dizin) = &kayit_dizini {
                Self::diske_kayit_olayi_yaz(dizin, kayit.oturum, &kayit_olayi);
            }

            kayit.bitis_bildirimi.notify_waiters();
        });
    }

    fn yetkili_kayit(&self, oturum: OturumId, is_id_metin: &str) -> Result<Arc<IsKaydi>, IsHatasi> {
        let is_id = IsId::coz(is_id_metin)?;
        if let Some(kayit) = self
            .kayitlar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .get(&is_id)
            .cloned()
        {
            if kayit.oturum != oturum {
                return Err(IsHatasi::YetkisizOturum);
            }
            return Ok(kayit);
        }
        if let Some(kurtarilan) = self
            .kurtarilanlar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .get(&is_id)
        {
            if kurtarilan.oturum != oturum {
                return Err(IsHatasi::YetkisizOturum);
            }
            return Err(IsHatasi::IsKapandi);
        }
        Err(IsHatasi::IsBulunamadi(is_id))
    }

    /// İmleçten itibaren yeni çıktıyı ve güncel iş durumunu okur.
    /// Çalışma alanı kilidini beklemez.
    pub fn oku(
        &self,
        oturum: OturumId,
        is_id_metin: &str,
        imlec: usize,
    ) -> Result<IsOkumaSonucu, IsHatasi> {
        let is_id = IsId::coz(is_id_metin)?;
        if let Some(kayit) = self
            .kayitlar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .get(&is_id)
            .cloned()
        {
            if kayit.oturum != oturum {
                return Err(IsHatasi::YetkisizOturum);
            }
            return Ok(kayit.oku(imlec));
        }
        if let Some(ozet) = self
            .kurtarilanlar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .get(&is_id)
            .cloned()
        {
            if ozet.oturum != oturum {
                return Err(IsHatasi::YetkisizOturum);
            }
            return Ok(IsOkumaSonucu {
                is_id: ozet.is_id,
                gosterim: ozet.gosterim,
                kip: ozet.kip,
                durum: ozet.durum,
                cikti: String::new(),
                yeni_imlec: 0,
                toplam_bayt: ozet.toplam_bayt,
                tasti: ozet.tasti,
                uyarilar: ozet.uyarilar,
            });
        }
        Err(IsHatasi::IsBulunamadi(is_id))
    }

    /// İşin standart girdisine veri yazar ve isteğe bağlı olarak EOF (`stdin` kapanışı)
    /// uygular. İş `Calisiyor` değilse reddedilir.
    pub async fn girdi(
        &self,
        oturum: OturumId,
        is_id_metin: &str,
        veri: &[u8],
        eof: bool,
    ) -> Result<IsOzeti, IsHatasi> {
        if veri.len() > AZAMI_IS_GIRDI_BAYT {
            return Err(IsHatasi::GirdiCokBuyuk {
                boyut: veri.len(),
                azami: AZAMI_IS_GIRDI_BAYT,
            });
        }
        let kayit = self.yetkili_kayit(oturum, is_id_metin)?;
        {
            let d = kayit.durum_verisi.lock().unwrap_or_else(|z| z.into_inner());
            if !matches!(d.durum, IsDurumu::Calisiyor) {
                return Err(IsHatasi::IsKapandi);
            }
        }

        let sure = kayit.girdi_yazma_suresi;
        let yazma = async {
            let mut kilit = kayit.girdi.lock().await;
            {
                let d = kayit.durum_verisi.lock().unwrap_or_else(|z| z.into_inner());
                if !matches!(d.durum, IsDurumu::Calisiyor) {
                    return Err(IsHatasi::IsKapandi);
                }
            }
            let Some(yazici) = kilit.as_mut() else {
                return Err(IsHatasi::GirdiKapali);
            };
            if eof
                && let Some(term) = &kayit.terminal
                && !term.kanonik_eof_destekleniyor_mu()
            {
                return Err(IsHatasi::GirdiHatasi(
                    "Terminal ham (kanonik olmayan) kipte olduğundan EOF gönderilemiyor; süreci sonlandırmak için durdurun veya uygulamaya uygun çıkış dizisi gönderin."
                        .to_string(),
                ));
            }
            if !veri.is_empty() {
                yazici
                    .write_all(veri)
                    .await
                    .map_err(|e| IsHatasi::GirdiHatasi(e.to_string()))?;
                yazici
                    .flush()
                    .await
                    .map_err(|e| IsHatasi::GirdiHatasi(e.to_string()))?;
            }
            if eof {
                let _ = kilit.take();
            }
            Ok(())
        };

        match tokio::time::timeout(sure, yazma).await {
            Ok(Ok(())) => Ok(kayit.ozet()),
            Ok(Err(hata)) => Err(hata),
            Err(_) => Err(IsHatasi::GirdiZamanAsimi),
        }
    }

    /// Sözde terminal (PTY/ConPTY) kipindeki işin pencere boyutunu günceller.
    pub fn boyutlandir(
        &self,
        oturum: OturumId,
        is_id_metin: &str,
        boyut: TerminalBoyutu,
    ) -> Result<IsOzeti, IsHatasi> {
        boyut
            .dogrula()
            .map_err(|e| IsHatasi::BoyutHatasi(e.to_string()))?;
        let kayit = self.yetkili_kayit(oturum, is_id_metin)?;
        let terminal = kayit.terminal.as_ref().ok_or(IsHatasi::TerminalDegil)?;
        {
            let d = kayit.durum_verisi.lock().unwrap_or_else(|z| z.into_inner());
            if !matches!(d.durum, IsDurumu::Calisiyor) {
                return Err(IsHatasi::IsKapandi);
            }
        }
        terminal
            .boyut_degistir(boyut)
            .map_err(|e| IsHatasi::BoyutHatasi(e.to_string()))?;
        {
            let mut d = kayit.durum_verisi.lock().unwrap_or_else(|z| z.into_inner());
            if matches!(d.kip, IsKipi::Terminal { .. }) {
                d.kip = IsKipi::Terminal { boyut };
            }
        }
        Ok(kayit.ozet())
    }

    /// İşi ve bütün süreç ağacını durdurur, son dosya denetimini bekler ve
    /// güncel durumu döner. Çift çağrılar tek kapanışı paylaşır.
    pub async fn durdur(&self, oturum: OturumId, is_id_metin: &str) -> Result<IsOzeti, IsHatasi> {
        let is_id = IsId::coz(is_id_metin)?;
        let kayit = self
            .kayitlar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .get(&is_id)
            .cloned();
        if let Some(kayit) = kayit {
            if kayit.oturum != oturum {
                return Err(IsHatasi::YetkisizOturum);
            }
            return Self::kaydi_durdur_ve_bekle(&kayit).await;
        }
        let kurtarilan = self
            .kurtarilanlar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .get(&is_id)
            .cloned();
        if let Some(ozet) = kurtarilan {
            if ozet.oturum != oturum {
                return Err(IsHatasi::YetkisizOturum);
            }
            return match &ozet.durum {
                IsDurumu::SonucuBilinmiyor { neden } => {
                    Err(IsHatasi::KapanisBelirsiz(neden.clone()))
                }
                _ => Ok(ozet),
            };
        }
        Err(IsHatasi::IsBulunamadi(is_id))
    }

    async fn kaydi_durdur_ve_bekle(kayit: &Arc<IsKaydi>) -> Result<IsOzeti, IsHatasi> {
        loop {
            let bildirim = kayit.bitis_bildirimi.notified();
            {
                let mut d = kayit.durum_verisi.lock().unwrap_or_else(|z| z.into_inner());
                match &d.durum {
                    IsDurumu::Calisiyor => {
                        d.durum = IsDurumu::Durduruluyor;
                        kayit.durdur_bildirimi.notify_one();
                    }
                    IsDurumu::Durduruluyor => {
                        kayit.durdur_bildirimi.notify_one();
                    }
                    IsDurumu::Bitti { .. } => {
                        drop(d);
                        return Ok(kayit.ozet());
                    }
                    IsDurumu::SonucuBilinmiyor { neden } => {
                        return Err(IsHatasi::KapanisBelirsiz(neden.clone()));
                    }
                }
            }
            bildirim.await;
        }
    }

    /// Sahip oturum kapanırken o oturuma ait tüm etkin işleri durdurur ve
    /// kapanışlarını bekler.
    pub async fn oturumu_kapat(&self, oturum: OturumId) {
        self.kapanan_oturumlar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .insert(oturum);
        let hedefler: Vec<Arc<IsKaydi>> = self
            .kayitlar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .values()
            .filter(|k| k.oturum == oturum)
            .cloned()
            .collect();
        for kayit in hedefler {
            let _ = Self::kaydi_durdur_ve_bekle(&kayit).await;
        }
    }

    /// Uygulama kapanırken bütün oturumların etkin işlerini durdurur ve bekler.
    pub async fn tumunu_kapat_ve_bekle(&self) {
        let hedefler: Vec<Arc<IsKaydi>> = self
            .kayitlar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .values()
            .cloned()
            .collect();
        for kayit in hedefler {
            let _ = Self::kaydi_durdur_ve_bekle(&kayit).await;
        }
    }

    /// Oturuma ait işlerin özetlerini döner.
    pub fn oturum_isleri(&self, oturum: OturumId) -> Vec<IsOzeti> {
        let tablo = self.kayitlar.lock().unwrap_or_else(|z| z.into_inner());
        let mut liste: Vec<IsOzeti> = tablo
            .values()
            .filter(|k| k.oturum == oturum)
            .map(|k| k.ozet())
            .collect();
        let kurtarilanlar = self.kurtarilanlar.lock().unwrap_or_else(|z| z.into_inner());
        for (id, ozet) in kurtarilanlar.iter() {
            if ozet.oturum == oturum && !tablo.contains_key(id) {
                liste.push(ozet.clone());
            }
        }
        liste.sort_by(|a, b| a.is_id.cmp(&b.is_id));
        liste
    }

    #[cfg(test)]
    fn zorla_kapanis_hatasi_ayarla(&self, is_id: &IsId) {
        if let Some(k) = self
            .kayitlar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .get(is_id)
        {
            k.zorla_kapanis_hatasi.store(true, Ordering::SeqCst);
        }
    }
}

fn varsayilan_dizin() -> String {
    ".".to_string()
}

/// Oturum kaydında (JSONL) arka plan işlerinin yaşam döngüsünü izleyen olaylar.
/// Kurtarma sırasında bitiş olayı olmayan bir `IsBasladi` hiçbir zaman `Calisiyor`
/// sayılmaz ve eski PID'lere sinyal gönderilmez; `SonucuBilinmiyor` olarak
/// işaretlenir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "is_olayi", rename_all = "snake_case")]
pub enum IsKayitOlayi {
    IsBasladi {
        is_id: IsId,
        oturum: OturumId,
        komut: String,
        #[serde(default = "varsayilan_dizin")]
        dizin: String,
        #[serde(default)]
        ag: bool,
        #[serde(default)]
        ag_hostlari: Vec<String>,
        #[serde(default)]
        yalitimli: bool,
        kip: IsKipi,
    },
    IsBitti {
        is_id: IsId,
        cikis_kodu: Option<i32>,
        sure_ms: u64,
        #[serde(default)]
        zaman_asimi: bool,
    },
    IsIptalEdildi {
        is_id: IsId,
        sure_ms: u64,
    },
    IsSonucuBilinmiyor {
        is_id: IsId,
        neden: String,
    },
}

/// JSONL kayıt satırlarından arka plan iş özetlerini kurtarır.
///
/// - Eski JSONL satırları (`is_olayi` alanı olmayanlar) sessizce atlanır; geri
///   uyumluluk korunur.
/// - `IsBasladi` olup kapanış olayı (`IsBitti`, `IsIptalEdildi`, `IsSonucuBilinmiyor`)
///   bulunmayan işler `IsDurumu::SonucuBilinmiyor` durumuna düşer; eski işletim
///   sistemi süreç kimliklerine kör sinyal gönderilmez.
pub fn isler_kayittan_kurtar<'a>(
    oturum: OturumId,
    satirlar: impl IntoIterator<Item = &'a str>,
) -> Vec<IsOzeti> {
    let mut sirali_id: Vec<IsId> = Vec::new();
    let mut tablo: HashMap<IsId, IsOzeti> = HashMap::new();

    for satir in satirlar {
        let satir = satir.trim();
        if satir.is_empty() {
            continue;
        }
        let Ok(deger) = serde_json::from_str::<serde_json::Value>(satir) else {
            continue;
        };
        if deger.get("is_olayi").is_none() {
            continue;
        }
        let Ok(olay) = serde_json::from_value::<IsKayitOlayi>(deger) else {
            continue;
        };
        match olay {
            IsKayitOlayi::IsBasladi {
                is_id,
                oturum: kayit_oturumu,
                komut,
                kip,
                ..
            } => {
                if kayit_oturumu != oturum {
                    continue;
                }
                if !tablo.contains_key(&is_id) {
                    sirali_id.push(is_id.clone());
                }
                tablo.insert(
                    is_id.clone(),
                    IsOzeti {
                        is_id,
                        oturum: kayit_oturumu,
                        gosterim: komut,
                        kip,
                        durum: IsDurumu::SonucuBilinmiyor {
                            neden: "Uygulama beklenmedik biçimde kapandı; eski süreç kimliklerine sinyal gönderilmedi.".to_string(),
                        },
                        toplam_bayt: 0,
                        tasti: false,
                        uyarilar: Vec::new(),
                    },
                );
            }
            IsKayitOlayi::IsBitti {
                is_id,
                cikis_kodu,
                sure_ms,
                zaman_asimi,
            } => {
                if let Some(ozet) = tablo.get_mut(&is_id) {
                    ozet.durum = IsDurumu::Bitti {
                        cikis_kodu,
                        sure_ms,
                        zaman_asimi,
                    };
                }
            }
            IsKayitOlayi::IsIptalEdildi { is_id, sure_ms } => {
                if let Some(ozet) = tablo.get_mut(&is_id) {
                    ozet.durum = IsDurumu::Bitti {
                        cikis_kodu: None,
                        sure_ms,
                        zaman_asimi: false,
                    };
                }
            }
            IsKayitOlayi::IsSonucuBilinmiyor { is_id, neden } => {
                if let Some(ozet) = tablo.get_mut(&is_id) {
                    ozet.durum = IsDurumu::SonucuBilinmiyor { neden };
                }
            }
        }
    }

    sirali_id
        .into_iter()
        .filter_map(|id| tablo.remove(&id))
        .collect()
}

#[derive(Debug, Clone)]
struct IsTarifMeta {
    dizin: String,
    ag: bool,
    ag_hostlari: Vec<String>,
    yalitimli: bool,
}

/// Belirli bir `OturumId` adına `kalfa-tools` arka plan iş araçlarını
/// (`kabuk`, `komut_oku`, `komut_girdi`, `komut_durdur`, `komut_boyutlandir`)
/// `IsYoneticisi`'ne bağlayan köprü.
pub struct OturumIsBaglantisi {
    yonetici: Arc<IsYoneticisi>,
    oturum: OturumId,
    tarifler: Mutex<HashMap<IsId, IsTarifMeta>>,
    kayit_olaylari: Mutex<Vec<IsKayitOlayi>>,
}

impl OturumIsBaglantisi {
    pub fn yeni(yonetici: Arc<IsYoneticisi>, oturum: OturumId) -> Arc<Self> {
        Arc::new(Self {
            yonetici,
            oturum,
            tarifler: Mutex::new(HashMap::new()),
            kayit_olaylari: Mutex::new(Vec::new()),
        })
    }

    pub fn oturum(&self) -> OturumId {
        self.oturum
    }

    /// Bu oturumda üretilen JSONL iş kayıt olaylarının kopyasını döner.
    pub fn kayit_olaylari(&self) -> Vec<IsKayitOlayi> {
        self.kayit_olaylari
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .clone()
    }
}

fn is_hatasini_arac_hatasina_cevir(hata: IsHatasi) -> kalfa_core::arac::AracHatasi {
    kalfa_core::arac::AracHatasi::Gecersiz(hata.to_string())
}

#[async_trait]
impl kalfa_tools::kabuk::isler::ArkaPlanIsBaglantisi for OturumIsBaglantisi {
    async fn is_baslat(
        &self,
        yalitici: &dyn Yalitici,
        istek: &CalistirmaIstegi,
        meta: kalfa_tools::kabuk::isler::ArkaPlanBaslatmaMeta,
        kilit: KilitSeti,
        alan: Arc<CalismaAlani>,
        onceki: AdliGirdiler,
    ) -> Result<String, kalfa_core::arac::AracHatasi> {
        let kip = match meta.terminal {
            Some(boyut) => IsKipi::Terminal { boyut },
            None => IsKipi::Borulu,
        };
        let son_denetim = AlanSonDenetimi::baslat(alan, onceki);
        let mut baslatma = IsBaslatma::yeni(
            self.oturum,
            meta.gosterim.clone(),
            kip,
            kilit,
            son_denetim.clone(),
        );
        baslatma.dizin = meta.dizin.clone();
        baslatma.ag = meta.ag;
        baslatma.ag_hostlari = meta.ag_hostlari.clone();
        baslatma.yalitimli = meta.yalitimli;
        baslatma.omur = meta.sure.min(AZAMI_IS_OMRU);
        match self
            .yonetici
            .yaliticidan_baslat(yalitici, istek, baslatma)
            .await
        {
            Ok(is_id) => {
                self.tarifler
                    .lock()
                    .unwrap_or_else(|z| z.into_inner())
                    .insert(
                        is_id.clone(),
                        IsTarifMeta {
                            dizin: meta.dizin.clone(),
                            ag: meta.ag,
                            ag_hostlari: meta.ag_hostlari.clone(),
                            yalitimli: meta.yalitimli,
                        },
                    );
                self.kayit_olaylari
                    .lock()
                    .unwrap_or_else(|z| z.into_inner())
                    .push(IsKayitOlayi::IsBasladi {
                        is_id: is_id.clone(),
                        oturum: self.oturum,
                        komut: meta.gosterim,
                        dizin: meta.dizin,
                        ag: meta.ag,
                        ag_hostlari: meta.ag_hostlari,
                        yalitimli: meta.yalitimli,
                        kip,
                    });
                Ok(is_id.as_str().to_string())
            }
            Err(hata) => {
                let _ = son_denetim.baslatma_hatasinda_kapat().await;
                Err(is_hatasini_arac_hatasina_cevir(hata))
            }
        }
    }

    fn is_bilgisi(
        &self,
        is_id: &str,
    ) -> Result<kalfa_tools::kabuk::isler::ArkaPlanIsBilgisi, kalfa_core::arac::AracHatasi> {
        let ozet = match self.yonetici.yetkili_kayit(self.oturum, is_id) {
            Ok(kayit) => kayit.ozet(),
            Err(IsHatasi::IsBulunamadi(kimlik)) => {
                let kurtarilan = self
                    .yonetici
                    .kurtarilanlar
                    .lock()
                    .unwrap_or_else(|z| z.into_inner())
                    .get(&kimlik)
                    .cloned();
                match kurtarilan {
                    Some(ozet) if ozet.oturum == self.oturum => ozet,
                    _ => {
                        return Err(is_hatasini_arac_hatasina_cevir(IsHatasi::IsBulunamadi(
                            kimlik,
                        )));
                    }
                }
            }
            Err(hata) => return Err(is_hatasini_arac_hatasina_cevir(hata)),
        };
        let tarif = self
            .tarifler
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .get(&ozet.is_id)
            .cloned()
            .unwrap_or(IsTarifMeta {
                dizin: ".".to_string(),
                ag: false,
                ag_hostlari: Vec::new(),
                yalitimli: false,
            });
        Ok(kalfa_tools::kabuk::isler::ArkaPlanIsBilgisi {
            is_id: ozet.is_id.as_str().to_string(),
            gosterim: ozet.gosterim,
            dizin: tarif.dizin,
            ag: tarif.ag,
            ag_hostlari: tarif.ag_hostlari,
            yalitimli: tarif.yalitimli,
            terminal: ozet.kip.terminal_boyutu(),
            calisiyor: matches!(ozet.durum, IsDurumu::Calisiyor),
        })
    }

    fn is_oku(
        &self,
        is_id: &str,
        imlec: usize,
    ) -> Result<kalfa_tools::kabuk::isler::ArkaPlanOkumaSonucu, kalfa_core::arac::AracHatasi> {
        let sonuc = self
            .yonetici
            .oku(self.oturum, is_id, imlec)
            .map_err(is_hatasini_arac_hatasina_cevir)?;
        Ok(kalfa_tools::kabuk::isler::ArkaPlanOkumaSonucu {
            is_id: sonuc.is_id.as_str().to_string(),
            durum_etiketi: sonuc.durum.etiket().to_string(),
            cikti: sonuc.cikti,
            yeni_imlec: sonuc.yeni_imlec,
            toplam_bayt: sonuc.toplam_bayt,
            tasti: sonuc.tasti,
            uyarilar: sonuc.uyarilar,
        })
    }

    async fn is_girdi(
        &self,
        is_id: &str,
        veri: &[u8],
        eof: bool,
    ) -> Result<String, kalfa_core::arac::AracHatasi> {
        let ozet = self
            .yonetici
            .girdi(self.oturum, is_id, veri, eof)
            .await
            .map_err(is_hatasini_arac_hatasina_cevir)?;
        Ok(ozet.durum.etiket().to_string())
    }

    fn is_boyutlandir(
        &self,
        is_id: &str,
        boyut: TerminalBoyutu,
    ) -> Result<String, kalfa_core::arac::AracHatasi> {
        let ozet = self
            .yonetici
            .boyutlandir(self.oturum, is_id, boyut)
            .map_err(is_hatasini_arac_hatasina_cevir)?;
        Ok(ozet.durum.etiket().to_string())
    }

    async fn is_durdur(
        &self,
        is_id: &str,
    ) -> Result<kalfa_tools::kabuk::isler::ArkaPlanDurdurmaSonucu, kalfa_core::arac::AracHatasi>
    {
        let ozet = self
            .yonetici
            .durdur(self.oturum, is_id)
            .await
            .map_err(is_hatasini_arac_hatasina_cevir)?;
        let (cikis_kodu, sure_ms, zaman_asimi) = match &ozet.durum {
            IsDurumu::Bitti {
                cikis_kodu,
                sure_ms,
                zaman_asimi,
            } => (*cikis_kodu, *sure_ms, *zaman_asimi),
            _ => (None, 0, false),
        };
        self.kayit_olaylari
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .push(IsKayitOlayi::IsIptalEdildi {
                is_id: ozet.is_id.clone(),
                sure_ms,
            });
        Ok(kalfa_tools::kabuk::isler::ArkaPlanDurdurmaSonucu {
            is_id: ozet.is_id.as_str().to_string(),
            durum_etiketi: ozet.durum.etiket().to_string(),
            cikis_kodu,
            sure_ms,
            zaman_asimi,
            uyarilar: ozet.uyarilar,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use kalfa_core::kilit::KilitYoneticisi;
    use kalfa_sandbox::{KomutBicimi, Yalitimsiz};

    use super::*;

    struct SayacliDenetim {
        sayac: Arc<AtomicUsize>,
        basarili: bool,
    }

    #[async_trait]
    impl SonDenetimKancasi for SayacliDenetim {
        async fn denetle_ve_bitir(&self) -> Result<SonDenetimSonucu, String> {
            self.sayac.fetch_add(1, Ordering::SeqCst);
            if self.basarili {
                Ok(SonDenetimSonucu {
                    uyarilar: Vec::new(),
                    karantina_basarisiz: false,
                    tarama_tam: true,
                    degisiklikler: None,
                    guncel_adli_girdiler: None,
                })
            } else {
                Ok(SonDenetimSonucu::denetlenemedi(
                    "tehlikeli .git karantinaya alınamadı",
                ))
            }
        }
    }

    fn test_istegi(kok: &std::path::Path, betik: &str) -> CalistirmaIstegi {
        CalistirmaIstegi {
            komut: KomutBicimi::Betik {
                metin: betik.to_string(),
            },
            dizin: kok.to_path_buf(),
            ortam: kalfa_sandbox::ortam::uret(std::env::vars()),
            ag: false,
            ag_listesi: None,
            alan: None,
        }
    }

    fn uyu_betigi() -> &'static str {
        #[cfg(unix)]
        {
            "echo HAZIR; while :; do sleep 0.1; done"
        }
        #[cfg(windows)]
        {
            "[Console]::Out.WriteLine('HAZIR'); [Console]::Out.Flush(); Start-Sleep -Seconds 60"
        }
    }

    #[tokio::test]
    async fn yetkisiz_oturum_ve_gecersiz_kimlik_reddedilir() {
        let d = tempfile::tempdir().unwrap();
        let kilit_y = KilitYoneticisi::yeni();
        let kilitler = kilit_y.al(d.path()).unwrap();
        let yonetici = Arc::new(IsYoneticisi::yeni());
        let oturum_a = OturumId::yeni();
        let oturum_b = OturumId::yeni();

        // Ham PID, dosya yolu veya rastgele metin iş kimliği olamaz.
        for gecersiz in ["12345", "../gizli", "is-degil", ""] {
            assert!(matches!(
                yonetici.oku(oturum_a, gecersiz, 0),
                Err(IsHatasi::GecersizKimlik(_))
            ));
        }

        // Var olmayan geçerli biçimli iş kimliği.
        let sahte_id = IsId::yeni();
        assert!(matches!(
            yonetici.oku(oturum_a, sahte_id.as_str(), 0),
            Err(IsHatasi::IsBulunamadi(_))
        ));

        let kilit = kilitler.sahipli_al(&[], &[], true).await;
        let is_id = yonetici
            .yaliticidan_baslat(
                &Yalitimsiz,
                &test_istegi(d.path(), uyu_betigi()),
                IsBaslatma::yeni(
                    oturum_a,
                    "uyu",
                    IsKipi::Borulu,
                    kilit,
                    Arc::new(BosSonDenetim),
                ),
            )
            .await
            .unwrap();

        // Başka oturumun kimliğiyle okuma, girdi, boyutlandırma ve durdurma reddedilir.
        assert_eq!(
            yonetici.oku(oturum_b, is_id.as_str(), 0),
            Err(IsHatasi::YetkisizOturum)
        );
        assert_eq!(
            yonetici
                .girdi(oturum_b, is_id.as_str(), b"x\n", false)
                .await,
            Err(IsHatasi::YetkisizOturum)
        );
        assert_eq!(
            yonetici.boyutlandir(oturum_b, is_id.as_str(), TerminalBoyutu::default()),
            Err(IsHatasi::YetkisizOturum)
        );
        assert_eq!(
            yonetici.durdur(oturum_b, is_id.as_str()).await,
            Err(IsHatasi::YetkisizOturum)
        );

        let ozet = yonetici.durdur(oturum_a, is_id.as_str()).await.unwrap();
        assert!(ozet.durum.bitti_mi());
    }

    #[tokio::test]
    async fn erken_arac_donusu_sonrasi_kilit_korunur_ve_cift_durdurma_tek_denetim_yapar() {
        let d = tempfile::tempdir().unwrap();
        let kilit_y = KilitYoneticisi::yeni();
        let kilitler = kilit_y.al(d.path()).unwrap();
        let yonetici = Arc::new(IsYoneticisi::yeni());
        let oturum = OturumId::yeni();
        let denetim_sayaci = Arc::new(AtomicUsize::new(0));

        // İş başlatma çağrısı kendi kapsamından erken döner; KilitSeti IsKaydi'na devredilir.
        let is_id = {
            let kilit = kilitler.sahipli_al(&[], &[], true).await;
            yonetici
                .yaliticidan_baslat(
                    &Yalitimsiz,
                    &test_istegi(d.path(), uyu_betigi()),
                    IsBaslatma::yeni(
                        oturum,
                        "arka-plan-sunucu",
                        IsKipi::Borulu,
                        kilit,
                        Arc::new(SayacliDenetim {
                            sayac: denetim_sayaci.clone(),
                            basarili: true,
                        }),
                    ),
                )
                .await
                .unwrap()
        };

        // İş çalışırken çalışma alanı genel yazma kilidi tutulmaya devam eder.
        let kilitler_ikinci = kilitler.clone();
        let mut bekleyen_yazma =
            tokio::spawn(async move { kilitler_ikinci.al(&[], &[], true).await });
        assert!(
            tokio::time::timeout(Duration::from_millis(80), &mut bekleyen_yazma)
                .await
                .is_err(),
            "arka plan işi çalışırken eşzamanlı yazma kilidi alınamamalı"
        );

        // Kontrol aracı (oku) genel yazma kilidini beklemeden anında yanıt verir.
        let okuma = yonetici.oku(oturum, is_id.as_str(), 0).unwrap();
        assert_eq!(okuma.durum, IsDurumu::Calisiyor);

        // Eşzamanlı çift durdurma tek kapanış ve tek son denetim çalıştırır.
        let y1 = yonetici.clone();
        let y2 = yonetici.clone();
        let id1 = is_id.as_str().to_string();
        let id2 = is_id.as_str().to_string();
        let (s1, s2) = tokio::join!(y1.durdur(oturum, &id1), y2.durdur(oturum, &id2));
        assert!(s1.unwrap().durum.bitti_mi());
        assert!(s2.unwrap().durum.bitti_mi());
        assert_eq!(
            denetim_sayaci.load(Ordering::SeqCst),
            1,
            "son denetim tam olarak bir kez çalışmalı"
        );

        // Kapanış ve son denetim tamamlandıktan sonra bekleyen yazma kilidi açılır.
        let _yeni_kilit = tokio::time::timeout(Duration::from_secs(2), bekleyen_yazma)
            .await
            .expect("iş kapandıktan sonra yazma kilidi serbest kalmalı")
            .unwrap();
    }

    #[tokio::test]
    async fn stdin_okumayan_cocukta_girdi_zaman_asimi_verir_ve_durdurma_kilitlenmez() {
        let d = tempfile::tempdir().unwrap();
        let kilit_y = KilitYoneticisi::yeni();
        let kilitler = kilit_y.al(d.path()).unwrap();
        let yonetici = Arc::new(IsYoneticisi::yeni_sinirli(
            AZAMI_IS_CIKTISI_BAYT,
            Duration::from_millis(150),
        ));
        let oturum = OturumId::yeni();
        let kilit = kilitler.sahipli_al(&[], &[], true).await;

        let is_id = yonetici
            .yaliticidan_baslat(
                &Yalitimsiz,
                &test_istegi(d.path(), uyu_betigi()),
                IsBaslatma::yeni(
                    oturum,
                    "okumayan",
                    IsKipi::Borulu,
                    kilit,
                    Arc::new(BosSonDenetim),
                ),
            )
            .await
            .unwrap();

        // 64 KiB üstü tek girdi anında reddedilir.
        let cok_buyuk = vec![b'A'; AZAMI_IS_GIRDI_BAYT + 1];
        assert!(matches!(
            yonetici
                .girdi(oturum, is_id.as_str(), &cok_buyuk, false)
                .await,
            Err(IsHatasi::GirdiCokBuyuk { .. })
        ));

        // Çocuk stdin okumadığı için boru tamponu dolunca yazma zaman aşımına düşer,
        // sonsuza dek bloklanmaz.
        let blok = vec![b'X'; AZAMI_IS_GIRDI_BAYT];
        let mut zaman_asimi_goruldu = false;
        for _ in 0..32 {
            match yonetici.girdi(oturum, is_id.as_str(), &blok, false).await {
                Ok(_) => {}
                Err(IsHatasi::GirdiZamanAsimi) => {
                    zaman_asimi_goruldu = true;
                    break;
                }
                Err(diger) => panic!("beklenmeyen girdi hatası: {diger:?}"),
            }
        }
        assert!(
            zaman_asimi_goruldu,
            "stdin okumayan çocukta girdi zaman aşımı dönmeli"
        );

        // Dolu stdin tamponuna rağmen durdurma kilitlenmeden tamamlanır.
        let ozet = tokio::time::timeout(
            Duration::from_secs(5),
            yonetici.durdur(oturum, is_id.as_str()),
        )
        .await
        .expect("durdurma kilitlenmemeli")
        .unwrap();
        assert!(ozet.durum.bitti_mi());

        // Kapanmış işe yeni girdi reddedilir.
        assert_eq!(
            yonetici
                .girdi(oturum, is_id.as_str(), b"sonra\n", false)
                .await,
            Err(IsHatasi::IsKapandi)
        );
    }

    #[tokio::test]
    async fn tasan_cikti_boruyu_tikamaz_ve_tasti_isaretler() {
        let d = tempfile::tempdir().unwrap();
        let kilit_y = KilitYoneticisi::yeni();
        let kilitler = kilit_y.al(d.path()).unwrap();
        // Küçük çıktı sınırı (4 KiB) ile 64 KiB çıktı üreten çocuğu sına.
        let yonetici = Arc::new(IsYoneticisi::yeni_sinirli(4096, Duration::from_secs(1)));
        let oturum = OturumId::yeni();
        let kilit = kilitler.sahipli_al(&[], &[], true).await;

        #[cfg(unix)]
        let betik = "dd if=/dev/zero bs=1024 count=64 2>/dev/null | tr '\\0' 'A'";
        #[cfg(windows)]
        let betik = "$s = 'A' * 1024; for ($i = 0; $i -lt 64; $i++) { [Console]::Out.Write($s) }; [Console]::Out.Flush()";

        let is_id = yonetici
            .yaliticidan_baslat(
                &Yalitimsiz,
                &test_istegi(d.path(), betik),
                IsBaslatma::yeni(
                    oturum,
                    "tasan-cikti",
                    IsKipi::Borulu,
                    kilit,
                    Arc::new(BosSonDenetim),
                ),
            )
            .await
            .unwrap();

        let son = Instant::now() + Duration::from_secs(10);
        let mut son_okuma = yonetici.oku(oturum, is_id.as_str(), 0).unwrap();
        while son_okuma.durum.etkin_mi() && Instant::now() < son {
            tokio::time::sleep(Duration::from_millis(50)).await;
            son_okuma = yonetici.oku(oturum, is_id.as_str(), 0).unwrap();
        }

        assert_eq!(
            son_okuma.durum,
            IsDurumu::Bitti {
                cikis_kodu: Some(0),
                sure_ms: match son_okuma.durum {
                    IsDurumu::Bitti { sure_ms, .. } => sure_ms,
                    _ => 0,
                },
                zaman_asimi: false,
            }
        );
        assert!(son_okuma.tasti, "sınır aşılınca tasti=true olmalı");
        assert_eq!(son_okuma.yeni_imlec, 4096);
        assert!(son_okuma.toplam_bayt >= 65_536);
    }

    #[test]
    fn utf8_cok_baytli_karakter_bolunmesi_bozuk_karakter_uretmez() {
        let mut tampon = CiktiTamponu::yeni(1024);
        let ornek = "Çalışma alanı: ğüşiöç 🦀 tamam";
        let baytlar = ornek.as_bytes();

        // Baytları tek tek ekleyip her adımda imleçle oku.
        let mut imlec = 0;
        let mut biriken = String::new();
        for &b in &baytlar[..baytlar.len() - 1] {
            tampon.ekle(&[b]);
            let (parca, yeni_imlec, _, _) = tampon.oku(imlec);
            assert!(
                !parca.contains('\u{FFFD}'),
                "bölünmüş UTF-8 baytında bozuk karakter üretilmemeli: {parca:?}"
            );
            biriken.push_str(&parca);
            imlec = yeni_imlec;
        }
        tampon.ekle(&baytlar[baytlar.len() - 1..]);
        tampon.akis_bitti = true;
        let (son_parca, yeni_imlec, _, _) = tampon.oku(imlec);
        biriken.push_str(&son_parca);
        assert_eq!(yeni_imlec, baytlar.len());
        assert_eq!(biriken, ornek);
    }

    #[tokio::test]
    async fn terminal_kipi_boyutlandirma_ansi_ve_kapanis_calisir() {
        let d = tempfile::tempdir().unwrap();
        let kilit_y = KilitYoneticisi::yeni();
        let kilitler = kilit_y.al(d.path()).unwrap();
        let yonetici = Arc::new(IsYoneticisi::yeni());
        let oturum = OturumId::yeni();
        let kilit = kilitler.sahipli_al(&[], &[], true).await;

        #[cfg(unix)]
        let betik = "printf '\\033[32mRENKLI\\033[0m\\n'; read satir; printf 'YANKI:%s\\n' \"$satir\"; while :; do sleep 0.1; done";
        #[cfg(windows)]
        let betik = "[Console]::Out.WriteLine(\"`e[32mRENKLI`e[0m\"); [Console]::Out.Flush(); $s = [Console]::In.ReadLine(); [Console]::Out.WriteLine(\"YANKI:$s\"); [Console]::Out.Flush(); Start-Sleep -Seconds 60";

        let boyut = TerminalBoyutu::yeni(90, 28).unwrap();
        let is_id = yonetici
            .yaliticidan_baslat(
                &Yalitimsiz,
                &test_istegi(d.path(), betik),
                IsBaslatma::yeni(
                    oturum,
                    "pty-is",
                    IsKipi::Terminal { boyut },
                    kilit,
                    Arc::new(BosSonDenetim),
                ),
            )
            .await
            .unwrap();

        let yeni_boyut = TerminalBoyutu::yeni(110, 36).unwrap();
        let ozet = yonetici
            .boyutlandir(oturum, is_id.as_str(), yeni_boyut)
            .unwrap();
        assert_eq!(ozet.kip, IsKipi::Terminal { boyut: yeni_boyut });

        // Geçersiz boyut reddedilir.
        assert!(
            yonetici
                .boyutlandir(
                    oturum,
                    is_id.as_str(),
                    TerminalBoyutu {
                        sutun: 0,
                        satir: 24
                    }
                )
                .is_err()
        );

        yonetici
            .girdi(oturum, is_id.as_str(), b"merhaba_pty\r\n", false)
            .await
            .unwrap();

        let son = Instant::now() + Duration::from_secs(10);
        let mut imlec = 0;
        let mut toplanan = String::new();
        while !toplanan.contains("YANKI:merhaba_pty") && Instant::now() < son {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let okuma = yonetici.oku(oturum, is_id.as_str(), imlec).unwrap();
            toplanan.push_str(&okuma.cikti);
            imlec = okuma.yeni_imlec;
        }
        assert!(toplanan.contains("RENKLI"), "{toplanan:?}");
        assert!(toplanan.contains("YANKI:merhaba_pty"), "{toplanan:?}");

        let bitis = yonetici.durdur(oturum, is_id.as_str()).await.unwrap();
        assert!(bitis.durum.bitti_mi());
    }

    #[tokio::test]
    async fn sahip_oturum_kapaninca_etkin_isler_durdurulur_ve_yeni_is_acilmaz() {
        let d = tempfile::tempdir().unwrap();
        let kilit_y = KilitYoneticisi::yeni();
        let kilitler = kilit_y.al(d.path()).unwrap();
        let yonetici = Arc::new(IsYoneticisi::yeni());
        let oturum = OturumId::yeni();

        let kilit = kilitler.sahipli_al(&[], &[], true).await;
        let is_id = yonetici
            .yaliticidan_baslat(
                &Yalitimsiz,
                &test_istegi(d.path(), uyu_betigi()),
                IsBaslatma::yeni(
                    oturum,
                    "oturum-isi",
                    IsKipi::Borulu,
                    kilit,
                    Arc::new(BosSonDenetim),
                ),
            )
            .await
            .unwrap();

        yonetici.oturumu_kapat(oturum).await;
        let okuma = yonetici.oku(oturum, is_id.as_str(), 0).unwrap();
        assert!(okuma.durum.bitti_mi());

        // Kapanan oturumda yeni iş başlatılamaz.
        let yeni_kilit = kilitler.sahipli_al(&[], &[], true).await;
        assert_eq!(
            yonetici
                .yaliticidan_baslat(
                    &Yalitimsiz,
                    &test_istegi(d.path(), uyu_betigi()),
                    IsBaslatma::yeni(
                        oturum,
                        "yeni-is",
                        IsKipi::Borulu,
                        yeni_kilit,
                        Arc::new(BosSonDenetim),
                    ),
                )
                .await,
            Err(IsHatasi::OturumKapali)
        );
    }

    #[tokio::test]
    async fn kapanis_hatasi_kilidi_serbest_birakmaz_ve_sonucu_bilinmiyor_yapar() {
        let d = tempfile::tempdir().unwrap();
        let kilit_y = KilitYoneticisi::yeni();
        let kilitler = kilit_y.al(d.path()).unwrap();
        let yonetici = Arc::new(IsYoneticisi::yeni());
        let oturum = OturumId::yeni();

        let kilit = kilitler.sahipli_al(&[], &[], true).await;
        let is_id = yonetici
            .yaliticidan_baslat(
                &Yalitimsiz,
                &test_istegi(d.path(), uyu_betigi()),
                IsBaslatma::yeni(
                    oturum,
                    "bozuk-kapanis",
                    IsKipi::Borulu,
                    kilit,
                    Arc::new(BosSonDenetim),
                ),
            )
            .await
            .unwrap();

        yonetici.zorla_kapanis_hatasi_ayarla(&is_id);
        let sonuc = yonetici.durdur(oturum, is_id.as_str()).await;
        assert!(matches!(sonuc, Err(IsHatasi::KapanisBelirsiz(_))));

        let okuma = yonetici.oku(oturum, is_id.as_str(), 0).unwrap();
        assert!(matches!(okuma.durum, IsDurumu::SonucuBilinmiyor { .. }));

        // Süreç ağacı doğrulamayla kapanamadığı için KilitSeti karantinada tutulur;
        // çalışma alanında yeni yazma kilidi sessizce açılmaz!
        let kilitler_ikinci = kilitler.clone();
        let mut bekleyen_yazma =
            tokio::spawn(async move { kilitler_ikinci.al(&[], &[], true).await });
        assert!(
            tokio::time::timeout(Duration::from_millis(80), &mut bekleyen_yazma)
                .await
                .is_err(),
            "belirsiz kapanışta çalışma alanı yazma kilidi serbest bırakılmamalı"
        );
        bekleyen_yazma.abort();
    }

    #[tokio::test]
    async fn oturum_basina_azami_etkin_is_siniri_uygulanir() {
        let d = tempfile::tempdir().unwrap();
        let kilit_y = KilitYoneticisi::yeni();
        let kilitler = kilit_y.al(d.path()).unwrap();
        let yonetici = Arc::new(IsYoneticisi::yeni());
        let oturum = OturumId::yeni();

        let mut isler = Vec::new();
        for i in 0..AZAMI_ETKIN_IS_OTURUM_BASI {
            let kilit = kilitler.al(&[], &[], false).await;
            let is_id = yonetici
                .yaliticidan_baslat(
                    &Yalitimsiz,
                    &test_istegi(d.path(), uyu_betigi()),
                    IsBaslatma::yeni(
                        oturum,
                        format!("is-{i}"),
                        IsKipi::Borulu,
                        kilit,
                        Arc::new(BosSonDenetim),
                    ),
                )
                .await
                .unwrap();
            isler.push(is_id);
        }

        // 5. eşzamanlı iş sınırı aşar.
        let fazladan_kilit = kilitler.al(&[], &[], false).await;
        assert_eq!(
            yonetici
                .yaliticidan_baslat(
                    &Yalitimsiz,
                    &test_istegi(d.path(), uyu_betigi()),
                    IsBaslatma::yeni(
                        oturum,
                        "is-fazla",
                        IsKipi::Borulu,
                        fazladan_kilit,
                        Arc::new(BosSonDenetim),
                    ),
                )
                .await,
            Err(IsHatasi::EtkinIsSiniriAsildi {
                sinir: AZAMI_ETKIN_IS_OTURUM_BASI
            })
        );

        for is_id in isler {
            let _ = yonetici.durdur(oturum, is_id.as_str()).await.unwrap();
        }
    }

    struct TestGuveni(AtomicBool);
    impl kalfa_tools::kabuk::KabukGuveni for TestGuveni {
        fn guvenildi_mi(&self) -> bool {
            self.0.load(Ordering::SeqCst)
        }
        fn guvenildi(&self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    fn is_id_cikart(icerik: &str) -> String {
        icerik
            .lines()
            .find_map(|s| s.strip_prefix("İş: "))
            .expect("çıktıda İş: satırı olmalı")
            .trim()
            .to_string()
    }

    #[tokio::test]
    async fn s01_arka_plan_kabuk_komut_oku_ve_durdur_uctan_uca() {
        use kalfa_core::arac::{Arac, AracOrtami};
        use kalfa_core::izin::Siniflandirici;
        use kalfa_core::yol::CalismaYolu;
        use kalfa_tools::kabuk::KabukAraci;
        use kalfa_tools::kabuk::isler::{CancellationToken, arka_plan_araclari};
        use serde_json::json;

        let d = tempfile::tempdir().unwrap();
        let sinif = Arc::new(Siniflandirici::yeni(d.path().to_path_buf(), &[]).unwrap());
        let alan = CalismaAlani::ac(sinif).unwrap();
        let yalitici: Arc<dyn Yalitici> = Arc::new(Yalitimsiz);
        let guven = Arc::new(TestGuveni(AtomicBool::new(false)));
        let izin = Arc::new(AtomicBool::new(true));
        let kullanici_ag: Arc<std::sync::RwLock<Vec<kalfa_sandbox::vekil::HostKurali>>> =
            Arc::default();

        let yonetici = Arc::new(IsYoneticisi::yeni());
        let oturum = OturumId::yeni();
        let baglanti = OturumIsBaglantisi::yeni(yonetici.clone(), oturum);

        let kabuk = KabukAraci::yeni(alan, yalitici.clone(), guven, izin.clone())
            .kullanici_ag_listesi(kullanici_ag.clone())
            .arka_plan_baglantisi(baglanti.clone());
        let kontrol = arka_plan_araclari(baglanti.clone(), yalitici, izin, kullanici_ag);
        let oku = &kontrol[0];
        let durdur = &kontrol[2];

        let kilit_y = KilitYoneticisi::yeni();
        let kilitler = kilit_y.al(d.path()).unwrap();
        let baslat_arg = json!({
            "betik": if cfg!(windows) {
                "[Console]::Out.WriteLine('S01-HAZIR'); [Console]::Out.Flush(); Start-Sleep -Seconds 30"
            } else {
                "echo S01-HAZIR; sleep 30"
            },
            "arka_plan": true
        });
        let h_baslat = kabuk.hazirla(&baslat_arg).await.unwrap();
        let kilit = kilitler
            .sahipli_al(&[], &[], h_baslat.tum_calisma_alani)
            .await;
        let c_baslat = kabuk
            .calistir_kilitli(
                &baslat_arg,
                &h_baslat,
                CancellationToken::new(),
                AracOrtami::bos(),
                Some(kilit),
            )
            .await
            .unwrap();
        let is_id = is_id_cikart(&c_baslat.icerik);

        // Arka plan işi sürerken eşzamanlı dosya yazma kilidi bekler.
        let yaz_yolu = [CalismaYolu::coz("a.txt").unwrap()];
        assert!(
            tokio::time::timeout(
                Duration::from_millis(50),
                kilitler.sahipli_al(&[], &yaz_yolu, false)
            )
            .await
            .is_err()
        );

        // `komut_oku` imleçle çıktıyı okur.
        let mut biriken = String::new();
        let mut imlec = 0u64;
        for _ in 0..80 {
            let oku_arg = json!({"is_id": is_id, "imlec": imlec});
            let h_oku = oku.hazirla(&oku_arg).await.unwrap();
            let c_oku = oku
                .calistir(&oku_arg, &h_oku, CancellationToken::new())
                .await
                .unwrap();
            biriken.push_str(&c_oku.icerik);
            if let Some(yeni) = c_oku
                .icerik
                .lines()
                .find_map(|s| s.strip_prefix("Yeni imleç: "))
                .and_then(|s| s.trim().parse::<u64>().ok())
            {
                imlec = yeni;
            }
            if biriken.contains("S01-HAZIR") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(biriken.contains("S01-HAZIR"), "okunan: {biriken}");

        // `komut_durdur` işi sonlandırır ve kilidi serbest bırakır.
        let durdur_arg = json!({"is_id": is_id});
        let h_durdur = durdur.hazirla(&durdur_arg).await.unwrap();
        let c_durdur = durdur
            .calistir(&durdur_arg, &h_durdur, CancellationToken::new())
            .await
            .unwrap();
        assert!(c_durdur.icerik.contains("Durum: bitti"));

        let _serbest_kilit = tokio::time::timeout(
            Duration::from_millis(100),
            kilitler.sahipli_al(&[], &yaz_yolu, false),
        )
        .await
        .expect("komut_durdur sonrasında yazma kilidi serbest kalmalı");
    }

    #[tokio::test]
    async fn s02_komut_girdi_stdin_ve_eof_ile_etkilesir() {
        use kalfa_core::arac::{Arac, AracOrtami};
        use kalfa_core::izin::Siniflandirici;
        use kalfa_tools::kabuk::KabukAraci;
        use kalfa_tools::kabuk::isler::{CancellationToken, arka_plan_araclari};
        use serde_json::json;

        let d = tempfile::tempdir().unwrap();
        let sinif = Arc::new(Siniflandirici::yeni(d.path().to_path_buf(), &[]).unwrap());
        let alan = CalismaAlani::ac(sinif).unwrap();
        let yalitici: Arc<dyn Yalitici> = Arc::new(Yalitimsiz);
        let guven = Arc::new(TestGuveni(AtomicBool::new(true)));
        let izin = Arc::new(AtomicBool::new(true));
        let kullanici_ag: Arc<std::sync::RwLock<Vec<kalfa_sandbox::vekil::HostKurali>>> =
            Arc::default();

        let yonetici = Arc::new(IsYoneticisi::yeni());
        let oturum = OturumId::yeni();
        let baglanti = OturumIsBaglantisi::yeni(yonetici.clone(), oturum);

        let kabuk = KabukAraci::yeni(alan, yalitici.clone(), guven, izin.clone())
            .arka_plan_baglantisi(baglanti.clone());
        let kontrol = arka_plan_araclari(baglanti, yalitici, izin, kullanici_ag);
        let oku = &kontrol[0];
        let girdi = &kontrol[1];
        let durdur = &kontrol[2];

        let kilit_y = KilitYoneticisi::yeni();
        let kilitler = kilit_y.al(d.path()).unwrap();
        let baslat_arg = json!({
            "betik": if cfg!(windows) {
                "$s = [Console]::In.ReadLine(); [Console]::Out.WriteLine(\"YANKI:$s\"); [Console]::Out.Flush()"
            } else {
                "IFS= read -r s; echo \"YANKI:$s\""
            },
            "arka_plan": true
        });
        let h_baslat = kabuk.hazirla(&baslat_arg).await.unwrap();
        let kilit = kilitler.sahipli_al(&[], &[], true).await;
        let c_baslat = kabuk
            .calistir_kilitli(
                &baslat_arg,
                &h_baslat,
                CancellationToken::new(),
                AracOrtami::bos(),
                Some(kilit),
            )
            .await
            .unwrap();
        let is_id = is_id_cikart(&c_baslat.icerik);

        let girdi_arg = json!({
            "is_id": is_id,
            "girdi": "merhaba-kalfa\n",
            "eof": true
        });
        let h_girdi = girdi.hazirla(&girdi_arg).await.unwrap();
        let _ = girdi
            .calistir(&girdi_arg, &h_girdi, CancellationToken::new())
            .await
            .unwrap();

        let mut biriken = String::new();
        for _ in 0..80 {
            let oku_arg = json!({"is_id": is_id, "imlec": 0});
            let h_oku = oku.hazirla(&oku_arg).await.unwrap();
            let c_oku = oku
                .calistir(&oku_arg, &h_oku, CancellationToken::new())
                .await
                .unwrap();
            biriken = c_oku.icerik;
            if biriken.contains("YANKI:merhaba-kalfa") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(biriken.contains("YANKI:merhaba-kalfa"), "çıktı: {biriken}");

        let durdur_arg = json!({"is_id": is_id});
        let h_durdur = durdur.hazirla(&durdur_arg).await.unwrap();
        let _ = durdur
            .calistir(&durdur_arg, &h_durdur, CancellationToken::new())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn s03_oturum_disi_is_id_ve_ham_pid_araclarca_reddedilir() {
        use kalfa_core::arac::{Arac, AracOrtami};
        use kalfa_core::izin::Siniflandirici;
        use kalfa_tools::kabuk::KabukAraci;
        use kalfa_tools::kabuk::isler::{CancellationToken, arka_plan_araclari};
        use serde_json::json;

        let d = tempfile::tempdir().unwrap();
        let sinif = Arc::new(Siniflandirici::yeni(d.path().to_path_buf(), &[]).unwrap());
        let alan = CalismaAlani::ac(sinif).unwrap();
        let yalitici: Arc<dyn Yalitici> = Arc::new(Yalitimsiz);
        let guven = Arc::new(TestGuveni(AtomicBool::new(true)));
        let izin = Arc::new(AtomicBool::new(true));
        let kullanici_ag: Arc<std::sync::RwLock<Vec<kalfa_sandbox::vekil::HostKurali>>> =
            Arc::default();

        let yonetici = Arc::new(IsYoneticisi::yeni());
        let oturum_a = OturumId::yeni();
        let oturum_b = OturumId::yeni();
        let baglanti_a = OturumIsBaglantisi::yeni(yonetici.clone(), oturum_a);
        let baglanti_b = OturumIsBaglantisi::yeni(yonetici.clone(), oturum_b);

        let kabuk_a = KabukAraci::yeni(alan, yalitici.clone(), guven, izin.clone())
            .arka_plan_baglantisi(baglanti_a.clone());
        let kontrol_b = arka_plan_araclari(baglanti_b, yalitici, izin, kullanici_ag);

        let kilit_y = KilitYoneticisi::yeni();
        let kilitler = kilit_y.al(d.path()).unwrap();
        let baslat_arg = json!({
            "betik": uyu_betigi(),
            "arka_plan": true,
            "terminal": true
        });
        let h_baslat = kabuk_a.hazirla(&baslat_arg).await.unwrap();
        let kilit = kilitler.sahipli_al(&[], &[], true).await;
        let c_baslat = kabuk_a
            .calistir_kilitli(
                &baslat_arg,
                &h_baslat,
                CancellationToken::new(),
                AracOrtami::bos(),
                Some(kilit),
            )
            .await
            .unwrap();
        let is_id = is_id_cikart(&c_baslat.icerik);

        for sahte_id in [is_id.as_str(), "12345", "/proc/12345"] {
            assert!(
                kontrol_b[0]
                    .hazirla(&json!({"is_id": sahte_id}))
                    .await
                    .is_err()
            );
            assert!(
                kontrol_b[1]
                    .hazirla(&json!({"is_id": sahte_id, "girdi": "x"}))
                    .await
                    .is_err()
            );
            assert!(
                kontrol_b[2]
                    .hazirla(&json!({"is_id": sahte_id}))
                    .await
                    .is_err()
            );
            assert!(
                kontrol_b[3]
                    .hazirla(&json!({"is_id": sahte_id, "sutun": 80, "satir": 24}))
                    .await
                    .is_err()
            );
        }

        let _ = yonetici.durdur(oturum_a, &is_id).await.unwrap();
    }

    #[test]
    fn s04_kurtarma_bitmemis_isi_sonucu_bilinmiyor_yapar_ve_eski_jsonl_bozulmaz() {
        let oturum = OturumId::yeni();
        let baska_oturum = OturumId::yeni();
        let is_acik = IsId::yeni();
        let is_bitmis = IsId::yeni();
        let is_baska = IsId::yeni();

        let eski_satir_1 = r#"{"tur":"kullanici","metin":"merhaba"}"#;
        let eski_satir_2 = r#"{"tur":"asistan","metin":"selam"}"#;

        let basladi_acik = serde_json::to_string(&IsKayitOlayi::IsBasladi {
            is_id: is_acik.clone(),
            oturum,
            komut: "cargo watch".to_string(),
            dizin: ".".to_string(),
            ag: false,
            ag_hostlari: Vec::new(),
            yalitimli: true,
            kip: IsKipi::Terminal {
                boyut: TerminalBoyutu::default(),
            },
        })
        .unwrap();

        let basladi_bitmis = serde_json::to_string(&IsKayitOlayi::IsBasladi {
            is_id: is_bitmis.clone(),
            oturum,
            komut: "cargo test".to_string(),
            dizin: ".".to_string(),
            ag: false,
            ag_hostlari: Vec::new(),
            yalitimli: true,
            kip: IsKipi::Borulu,
        })
        .unwrap();

        let bitti_olayi = serde_json::to_string(&IsKayitOlayi::IsBitti {
            is_id: is_bitmis.clone(),
            cikis_kodu: Some(0),
            sure_ms: 450,
            zaman_asimi: false,
        })
        .unwrap();

        let basladi_baska = serde_json::to_string(&IsKayitOlayi::IsBasladi {
            is_id: is_baska,
            oturum: baska_oturum,
            komut: "npm run dev".to_string(),
            dizin: ".".to_string(),
            ag: false,
            ag_hostlari: Vec::new(),
            yalitimli: true,
            kip: IsKipi::Borulu,
        })
        .unwrap();

        // Yalnız eski JSONL satırları varken kurtarma boş döner ve hata vermez.
        let yalniz_eski = isler_kayittan_kurtar(oturum, [eski_satir_1, eski_satir_2]);
        assert!(yalniz_eski.is_empty());

        let kurtarilan = isler_kayittan_kurtar(
            oturum,
            [
                eski_satir_1,
                basladi_acik.as_str(),
                basladi_bitmis.as_str(),
                basladi_baska.as_str(),
                eski_satir_2,
                bitti_olayi.as_str(),
            ],
        );
        assert_eq!(kurtarilan.len(), 2);
        assert_eq!(kurtarilan[0].is_id, is_acik);
        assert!(matches!(
            kurtarilan[0].durum,
            IsDurumu::SonucuBilinmiyor { .. }
        ));
        assert!(!kurtarilan[0].durum.etkin_mi());

        assert_eq!(kurtarilan[1].is_id, is_bitmis);
        assert_eq!(
            kurtarilan[1].durum,
            IsDurumu::Bitti {
                cikis_kodu: Some(0),
                sure_ms: 450,
                zaman_asimi: false,
            }
        );
    }

    #[tokio::test]
    async fn s05_terminal_kipi_ansi_ve_boyutlandirma_arac_uzerinden_calisir() {
        use kalfa_core::arac::{Arac, AracOrtami};
        use kalfa_core::izin::Siniflandirici;
        use kalfa_tools::kabuk::KabukAraci;
        use kalfa_tools::kabuk::isler::{CancellationToken, arka_plan_araclari};
        use serde_json::json;

        let d = tempfile::tempdir().unwrap();
        let sinif = Arc::new(Siniflandirici::yeni(d.path().to_path_buf(), &[]).unwrap());
        let alan = CalismaAlani::ac(sinif).unwrap();
        let yalitici: Arc<dyn Yalitici> = Arc::new(Yalitimsiz);
        let guven = Arc::new(TestGuveni(AtomicBool::new(true)));
        let izin = Arc::new(AtomicBool::new(true));
        let kullanici_ag: Arc<std::sync::RwLock<Vec<kalfa_sandbox::vekil::HostKurali>>> =
            Arc::default();

        let yonetici = Arc::new(IsYoneticisi::yeni());
        let oturum = OturumId::yeni();
        let baglanti = OturumIsBaglantisi::yeni(yonetici.clone(), oturum);

        let kabuk = KabukAraci::yeni(alan, yalitici.clone(), guven, izin.clone())
            .arka_plan_baglantisi(baglanti.clone());
        let kontrol = arka_plan_araclari(baglanti, yalitici, izin, kullanici_ag);
        let oku = &kontrol[0];
        let durdur = &kontrol[2];
        let boyut = &kontrol[3];

        let kilit_y = KilitYoneticisi::yeni();
        let kilitler = kilit_y.al(d.path()).unwrap();
        let baslat_arg = json!({
            "betik": if cfg!(windows) {
                "$e = [char]27; [Console]::Out.WriteLine(\"$e[32mYESIL-S05$e[0m\"); [Console]::Out.Flush(); Start-Sleep -Seconds 30"
            } else {
                "printf '\\033[32mYESIL-S05\\033[0m\\n'; sleep 30"
            },
            "arka_plan": true,
            "terminal": {"sutun": 80, "satir": 24}
        });
        let h_baslat = kabuk.hazirla(&baslat_arg).await.unwrap();
        let kilit = kilitler.sahipli_al(&[], &[], true).await;
        let c_baslat = kabuk
            .calistir_kilitli(
                &baslat_arg,
                &h_baslat,
                CancellationToken::new(),
                AracOrtami::bos(),
                Some(kilit),
            )
            .await
            .unwrap();
        let is_id = is_id_cikart(&c_baslat.icerik);

        let boyut_arg = json!({
            "is_id": is_id,
            "sutun": 110,
            "satir": 36
        });
        let h_boyut = boyut.hazirla(&boyut_arg).await.unwrap();
        let c_boyut = boyut
            .calistir(&boyut_arg, &h_boyut, CancellationToken::new())
            .await
            .unwrap();
        assert!(c_boyut.icerik.contains("110x36"));

        let mut biriken = String::new();
        for _ in 0..80 {
            let oku_arg = json!({"is_id": is_id, "imlec": 0});
            let h_oku = oku.hazirla(&oku_arg).await.unwrap();
            let c_oku = oku
                .calistir(&oku_arg, &h_oku, CancellationToken::new())
                .await
                .unwrap();
            biriken = c_oku.icerik;
            if biriken.contains("YESIL-S05") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(biriken.contains("YESIL-S05"), "terminal çıktısı: {biriken}");

        let durdur_arg = json!({"is_id": is_id});
        let h_durdur = durdur.hazirla(&durdur_arg).await.unwrap();
        let _ = durdur
            .calistir(&durdur_arg, &h_durdur, CancellationToken::new())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn s06_oturum_kapanisi_baglanti_uzerinden_baslatilan_isleri_temizler() {
        use kalfa_core::arac::{Arac, AracOrtami};
        use kalfa_core::izin::Siniflandirici;
        use kalfa_core::yol::CalismaYolu;
        use kalfa_tools::kabuk::KabukAraci;
        use kalfa_tools::kabuk::isler::CancellationToken;
        use serde_json::json;

        let d = tempfile::tempdir().unwrap();
        let sinif = Arc::new(Siniflandirici::yeni(d.path().to_path_buf(), &[]).unwrap());
        let alan = CalismaAlani::ac(sinif).unwrap();
        let yalitici: Arc<dyn Yalitici> = Arc::new(Yalitimsiz);
        let guven = Arc::new(TestGuveni(AtomicBool::new(true)));
        let izin = Arc::new(AtomicBool::new(true));

        let yonetici = Arc::new(IsYoneticisi::yeni());
        let oturum = OturumId::yeni();
        let baglanti = OturumIsBaglantisi::yeni(yonetici.clone(), oturum);

        let kabuk = KabukAraci::yeni(alan, yalitici, guven, izin).arka_plan_baglantisi(baglanti);
        let kilit_y = KilitYoneticisi::yeni();
        let kilitler = kilit_y.al(d.path()).unwrap();
        let baslat_arg = json!({
            "betik": uyu_betigi(),
            "arka_plan": true
        });
        let h_baslat = kabuk.hazirla(&baslat_arg).await.unwrap();
        let kilit = kilitler.sahipli_al(&[], &[], true).await;
        let _ = kabuk
            .calistir_kilitli(
                &baslat_arg,
                &h_baslat,
                CancellationToken::new(),
                AracOrtami::bos(),
                Some(kilit),
            )
            .await
            .unwrap();

        yonetici.oturumu_kapat(oturum).await;

        let yaz_yolu = [CalismaYolu::coz("temiz.txt").unwrap()];
        let _serbest = tokio::time::timeout(
            Duration::from_millis(100),
            kilitler.sahipli_al(&[], &yaz_yolu, false),
        )
        .await
        .expect("oturum kapanışı kilidi serbest bırakmalı");
    }

    fn pid_yasiyor_mu(pid: u32) -> bool {
        #[cfg(windows)]
        {
            std::process::Command::new("tasklist")
                .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).contains(&format!("\"{pid}\"")))
                .unwrap_or(false)
        }
        #[cfg(not(windows))]
        {
            std::fs::read_to_string(format!("/proc/{pid}/stat"))
                .map(|s| !s.contains(") Z ") && !s.contains(") X "))
                .unwrap_or(false)
        }
    }

    struct PidTemizlik(Arc<Mutex<Vec<u32>>>);

    impl Drop for PidTemizlik {
        fn drop(&mut self) {
            let pidler = self.0.lock().unwrap_or_else(|z| z.into_inner()).clone();
            for pid in pidler {
                #[cfg(windows)]
                let _ = std::process::Command::new("taskkill")
                    .args(["/F", "/PID", &pid.to_string()])
                    .output();
                #[cfg(not(windows))]
                let _ = std::process::Command::new("kill")
                    .args(["-9", &pid.to_string()])
                    .output();
            }
        }
    }

    struct SiraOlcenDenetim {
        yonetici: Arc<IsYoneticisi>,
        oturum: OturumId,
        is_id: Arc<Mutex<Option<String>>>,
        kilitler: Arc<kalfa_core::kilit::KaynakKilitleri>,
        izlenen_pidler: Arc<Mutex<Vec<u32>>>,
        adimlar: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl SonDenetimKancasi for SiraOlcenDenetim {
        async fn denetle_ve_bitir(&self) -> Result<SonDenetimSonucu, String> {
            let is_id = self
                .is_id
                .lock()
                .unwrap_or_else(|z| z.into_inner())
                .clone()
                .expect("denetim anında is_id atanmış olmalı");
            let okuma = self.yonetici.oku(self.oturum, &is_id, 0).unwrap();
            let pid: u32 = okuma
                .cikti
                .lines()
                .find_map(|s| s.strip_prefix("TORUN:"))
                .and_then(|s| s.trim().parse().ok())
                .expect("çıktıda TORUN:<pid> satırı bulunmalı");
            self.izlenen_pidler
                .lock()
                .unwrap_or_else(|z| z.into_inner())
                .push(pid);

            let torun_oldu = !pid_yasiyor_mu(pid);
            let cikti_tam = okuma.cikti.contains("SON-SATIR");
            // Son denetim çalışırken çalışma alanı kilidi henüz bırakılmamış olmalı.
            let kilit_tutuluyor = tokio::time::timeout(
                Duration::from_millis(25),
                self.kilitler.sahipli_al(&[], &[], true),
            )
            .await
            .is_err();

            let mut kayit = self.adimlar.lock().unwrap_or_else(|z| z.into_inner());
            if torun_oldu {
                kayit.push("torun_oldu".to_string());
            }
            if cikti_tam {
                kayit.push("cikti_bosaldi".to_string());
            }
            if kilit_tutuluyor {
                kayit.push("son_denetim_kilit_altinda".to_string());
            }
            Ok(SonDenetimSonucu {
                uyarilar: Vec::new(),
                karantina_basarisiz: false,
                tarama_tam: true,
                degisiklikler: None,
                guncel_adli_girdiler: None,
            })
        }
    }

    #[tokio::test]
    async fn dogal_bitis_iptal_zaman_asimi_ve_torun_cikti_son_denetim_sirasi_olculur() {
        let d = tempfile::tempdir().unwrap();
        let kilit_y = KilitYoneticisi::yeni();
        let kilitler = kilit_y.al(d.path()).unwrap();
        let yonetici = Arc::new(IsYoneticisi::yeni());
        let oturum = OturumId::yeni();
        let temizlik_listesi = Arc::new(Mutex::new(Vec::new()));
        let _temizlik = PidTemizlik(temizlik_listesi.clone());

        // 1) Doğal bitiş (Linux'ta setsid ile ayrılmış torun, Windows'ta arka plan çocuk)
        // 2) İptal (durdur)
        // 3) Zaman aşımı (omur)
        for kip in ["dogal", "iptal", "zaman_asimi"] {
            let adimlar = Arc::new(Mutex::new(Vec::new()));
            let is_id_kutu = Arc::new(Mutex::new(None));
            let denetim = Arc::new(SiraOlcenDenetim {
                yonetici: yonetici.clone(),
                oturum,
                is_id: is_id_kutu.clone(),
                kilitler: kilitler.clone(),
                izlenen_pidler: temizlik_listesi.clone(),
                adimlar: adimlar.clone(),
            });

            let betik = match (cfg!(windows), kip) {
                (false, "dogal") => {
                    "setsid sleep 30 </dev/null >/dev/null 2>&1 & echo \"TORUN:$!\"; echo SON-SATIR; sleep 0.2"
                }
                (false, _) => {
                    "setsid sleep 30 </dev/null >/dev/null 2>&1 & echo \"TORUN:$!\"; echo SON-SATIR; while :; do sleep 0.1; done"
                }
                (true, "dogal") => {
                    "$c = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\\timeout.exe') -ArgumentList @('/t','30','/nobreak') -PassThru -WindowStyle Hidden; [Console]::Out.WriteLine(\"TORUN:$($c.Id)\"); [Console]::Out.WriteLine('SON-SATIR'); [Console]::Out.Flush()"
                }
                (true, _) => {
                    "$c = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\\timeout.exe') -ArgumentList @('/t','30','/nobreak') -PassThru -WindowStyle Hidden; [Console]::Out.WriteLine(\"TORUN:$($c.Id)\"); [Console]::Out.WriteLine('SON-SATIR'); [Console]::Out.Flush(); Start-Sleep -Seconds 30"
                }
            };

            let kilit = kilitler.sahipli_al(&[], &[], true).await;
            let mut baslatma = IsBaslatma::yeni(oturum, kip, IsKipi::Borulu, kilit, denetim);
            if kip == "zaman_asimi" {
                baslatma.omur = if cfg!(windows) {
                    Duration::from_millis(1500)
                } else {
                    Duration::from_millis(400)
                };
            }
            let is_id = yonetici
                .yaliticidan_baslat(&Yalitimsiz, &test_istegi(d.path(), betik), baslatma)
                .await
                .unwrap();
            *is_id_kutu.lock().unwrap_or_else(|z| z.into_inner()) =
                Some(is_id.as_str().to_string());

            if kip == "iptal" {
                let son = Instant::now() + Duration::from_secs(5);
                while !yonetici
                    .oku(oturum, is_id.as_str(), 0)
                    .unwrap()
                    .cikti
                    .contains("SON-SATIR")
                    && Instant::now() < son
                {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                let ozet = yonetici.durdur(oturum, is_id.as_str()).await.unwrap();
                assert!(matches!(
                    ozet.durum,
                    IsDurumu::Bitti {
                        zaman_asimi: false,
                        ..
                    }
                ));
            } else {
                let son = Instant::now() + Duration::from_secs(8);
                while yonetici
                    .oku(oturum, is_id.as_str(), 0)
                    .unwrap()
                    .durum
                    .etkin_mi()
                    && Instant::now() < son
                {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                let son_durum = yonetici.oku(oturum, is_id.as_str(), 0).unwrap().durum;
                if kip == "dogal" {
                    assert!(
                        matches!(
                            son_durum,
                            IsDurumu::Bitti {
                                cikis_kodu: Some(0),
                                zaman_asimi: false,
                                ..
                            }
                        ),
                        "doğal bitiş durumu: {son_durum:?}"
                    );
                } else {
                    assert!(
                        matches!(
                            son_durum,
                            IsDurumu::Bitti {
                                zaman_asimi: true,
                                ..
                            }
                        ),
                        "zaman aşımı durumu: {son_durum:?}"
                    );
                }
            }

            // Bitişten hemen sonra kilit serbest bırakılmış olmalı.
            let _serbest = tokio::time::timeout(
                Duration::from_millis(100),
                kilitler.sahipli_al(&[], &[], true),
            )
            .await
            .expect("son denetim bittikten sonra kilit açılmalı");
            adimlar
                .lock()
                .unwrap_or_else(|z| z.into_inner())
                .push("kilit_birakildi".to_string());

            assert_eq!(
                *adimlar.lock().unwrap_or_else(|z| z.into_inner()),
                vec![
                    "torun_oldu".to_string(),
                    "cikti_bosaldi".to_string(),
                    "son_denetim_kilit_altinda".to_string(),
                    "kilit_birakildi".to_string(),
                ],
                "{kip} senaryosunda kapanış sırası bozuldu"
            );
        }
    }

    #[tokio::test]
    async fn terminal_kipinde_girdi_eof_programi_dogal_bitirir() {
        let d = tempfile::tempdir().unwrap();
        let kilit_y = KilitYoneticisi::yeni();
        let kilitler = kilit_y.al(d.path()).unwrap();
        let yonetici = Arc::new(IsYoneticisi::yeni());
        let oturum = OturumId::yeni();
        let kilit = kilitler.sahipli_al(&[], &[], true).await;

        let betik = if cfg!(windows) {
            "[Console]::In.ReadToEnd() > $null"
        } else {
            "cat >/dev/null"
        };

        let is_id = yonetici
            .yaliticidan_baslat(
                &Yalitimsiz,
                &test_istegi(d.path(), betik),
                IsBaslatma::yeni(
                    oturum,
                    "pty-eof",
                    IsKipi::Terminal {
                        boyut: TerminalBoyutu::default(),
                    },
                    kilit,
                    Arc::new(BosSonDenetim),
                ),
            )
            .await
            .unwrap();

        yonetici
            .girdi(oturum, is_id.as_str(), b"", true)
            .await
            .unwrap();

        let son = Instant::now() + Duration::from_secs(3);
        while yonetici
            .oku(oturum, is_id.as_str(), 0)
            .unwrap()
            .durum
            .etkin_mi()
            && Instant::now() < son
        {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let okuma = yonetici.oku(oturum, is_id.as_str(), 0).unwrap();
        assert!(
            matches!(
                okuma.durum,
                IsDurumu::Bitti {
                    cikis_kodu: Some(0),
                    zaman_asimi: false,
                    ..
                }
            ),
            "PTY eof=true sonrası iş doğal bitmeli: {:?}",
            okuma.durum
        );
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn terminal_kipinde_ham_eof_reddedilir_bekleyen_satir_boyut_ve_kapanis_dogrulanir() {
        let d = tempfile::tempdir().unwrap();
        let kilit_y = KilitYoneticisi::yeni();
        let kilitler = kilit_y.al(d.path()).unwrap();
        let yonetici = Arc::new(IsYoneticisi::yeni());
        let oturum = OturumId::yeni();

        // 1) Kanonik kipte satırda bekleyen veri (\n olmadan) + eof=true:
        let kilit1 = kilitler.sahipli_al(&[], &[], true).await;
        let is1 = yonetici
            .yaliticidan_baslat(
                &Yalitimsiz,
                &CalistirmaIstegi {
                    komut: KomutBicimi::Program {
                        program: "python3".into(),
                        argumanlar: vec![
                            "-c".into(),
                            "import sys\nv = sys.stdin.read()\nprint('BEKLEYEN:' + v, flush=True)"
                                .into(),
                        ],
                    },
                    dizin: d.path().to_path_buf(),
                    ortam: kalfa_sandbox::ortam::uret(std::env::vars()),
                    ag: false,
                    ag_listesi: None,
                    alan: None,
                },
                IsBaslatma::yeni(
                    oturum,
                    "pty-kanonik-bekleyen",
                    IsKipi::Terminal {
                        boyut: TerminalBoyutu::default(),
                    },
                    kilit1,
                    Arc::new(BosSonDenetim),
                ),
            )
            .await
            .unwrap();

        yonetici
            .girdi(oturum, is1.as_str(), b"yarim-satir", false)
            .await
            .unwrap();
        yonetici
            .girdi(oturum, is1.as_str(), b"", true)
            .await
            .unwrap();

        let son = Instant::now() + Duration::from_secs(3);
        while yonetici
            .oku(oturum, is1.as_str(), 0)
            .unwrap()
            .durum
            .etkin_mi()
            && Instant::now() < son
        {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let okuma1 = yonetici.oku(oturum, is1.as_str(), 0).unwrap();
        assert!(
            okuma1.cikti.contains("BEKLEYEN:yarim-satir"),
            "bekleyen satır verisi: {:?}",
            okuma1.cikti
        );
        assert!(matches!(
            okuma1.durum,
            IsDurumu::Bitti {
                cikis_kodu: Some(0),
                zaman_asimi: false,
                ..
            }
        ));

        // 2) Ham (tty.setraw) kipte eof=true açık hata ile reddedilir, 040404 üretmez,
        // kilit korunur, boyutlandırma ve durdurma düzgün çalışır:
        let kilit2 = kilitler.sahipli_al(&[], &[], true).await;
        let denetim_sayaci = Arc::new(AtomicUsize::new(0));
        let ham_betik = "import os, tty\ntty.setraw(0)\nprint('HAZIR', flush=True)\nb=b''\nwhile len(b)<3:\n b += os.read(0, 3-len(b))\nprint('ALINAN:'+b.hex(), flush=True)\n";
        let is2 = yonetici
            .yaliticidan_baslat(
                &Yalitimsiz,
                &CalistirmaIstegi {
                    komut: KomutBicimi::Program {
                        program: "python3".into(),
                        argumanlar: vec!["-c".into(), ham_betik.into()],
                    },
                    dizin: d.path().to_path_buf(),
                    ortam: kalfa_sandbox::ortam::uret(std::env::vars()),
                    ag: false,
                    ag_listesi: None,
                    alan: None,
                },
                IsBaslatma::yeni(
                    oturum,
                    "pty-ham-eof",
                    IsKipi::Terminal {
                        boyut: TerminalBoyutu::default(),
                    },
                    kilit2,
                    Arc::new(SayacliDenetim {
                        sayac: denetim_sayaci.clone(),
                        basarili: true,
                    }),
                ),
            )
            .await
            .unwrap();

        let son = Instant::now() + Duration::from_secs(3);
        while !yonetici
            .oku(oturum, is2.as_str(), 0)
            .unwrap()
            .cikti
            .contains("HAZIR")
            && Instant::now() < son
        {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        let hata = yonetici
            .girdi(oturum, is2.as_str(), b"", true)
            .await
            .expect_err("ham terminalde eof=true açık hata vermeli");
        assert!(
            matches!(hata, IsHatasi::GirdiHatasi(ref m) if m.contains("ham")),
            "beklenmeyen hata: {hata:?}"
        );

        tokio::time::sleep(Duration::from_millis(150)).await;
        let ara_okuma = yonetici.oku(oturum, is2.as_str(), 0).unwrap();
        assert!(
            !ara_okuma.cikti.contains("ALINAN:040404") && !ara_okuma.cikti.contains("ALINAN:"),
            "ham terminale sahte VEOF verisi enjekte edildi: {:?}",
            ara_okuma.cikti
        );
        assert!(matches!(ara_okuma.durum, IsDurumu::Calisiyor));

        // Çalışma alanı kilidi hâlâ tutuluyor olmalı.
        assert!(
            tokio::time::timeout(
                Duration::from_millis(40),
                kilitler.sahipli_al(&[], &[], true),
            )
            .await
            .is_err(),
            "ham terminal çalışırken çalışma alanı kilidi korunmalı"
        );

        // Boyutlandırma çalışırken başarılı olur.
        yonetici
            .boyutlandir(oturum, is2.as_str(), TerminalBoyutu::yeni(100, 30).unwrap())
            .unwrap();

        // Durdurma ağacı sonlandırır, son denetimi çalıştırır ve kilidi bırakır.
        let bitis = yonetici.durdur(oturum, is2.as_str()).await.unwrap();
        assert!(matches!(bitis.durum, IsDurumu::Bitti { .. }));
        assert_eq!(denetim_sayaci.load(Ordering::SeqCst), 1);
        assert!(
            yonetici
                .boyutlandir(oturum, is2.as_str(), TerminalBoyutu::default())
                .is_err()
        );
        let _serbest = tokio::time::timeout(
            Duration::from_millis(100),
            kilitler.sahipli_al(&[], &[], true),
        )
        .await
        .expect("durdur ve son denetim sonrasında çalışma alanı kilidi bırakılmalı");
    }
}
