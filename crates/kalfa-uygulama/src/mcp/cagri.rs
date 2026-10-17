//! MCP araç çağrısı için manager-owned kayıt, RPC worker ve son denetim kapısı.
//!
//! `McpCagriArkaUcu` süreç yöneticisine
//! bağlanırken `rpc_guncelse` R/D/Π/K/P/N'yi son kez doğrulayıp MCP RPC'sini
//! aynı çağrı yolunda başlatmalıdır. RPC future'ı bırakılınca SDK'nın
//! yanıt-alıcısı kapanır; cleanup bundan sonra G5'in gerçek süreç kapanışı ve
//! zorunlu son denetim sonucunu bekler.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kalfa_core::arac::{
    AracCiktisi, AracHatasi, McpAracHazirligi, McpAracTanimOzeti, McpBaslatmaSayaci, McpTarifi,
    async_trait,
};
use kalfa_core::kilit::KilitSeti;
use kalfa_core::kimlik::CagriId;
use kalfa_mcp::arac::{
    McpArac, McpAracCagriIstegi, McpAracKimligi, McpAracRpcHataDonusturucu,
    McpAracSonucDonusturucu, McpAracSunumu, McpAracYurutucusu,
};
use kalfa_sandbox::tarama::CancellationToken;
use tokio::sync::{oneshot, watch};
use tokio::time::Instant;

use crate::mcp::koruma::{McpAdliGirdiler, SonDenetimIstegi};

const GUVENLI_CAGRI_HATASI: &str = "MCP çağrısı güvenli biçimde tamamlanamadı.";
const GUVENLI_ZAMAN_ASIMI: &str = "MCP çağrısı zaman aşımına uğradı.";

/// Çağrı yaşam döngüsü. Hata durumları terminal engel durumudur: süreç
/// yöneticisi ve çağrı defteri bunları başarılı kapanış gibi yorumlamaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum McpCagriDurumu {
    Hazirlaniyor,
    RpcCalisiyor,
    DenetimBekliyor,
    SonDenetimSuruyor,
    IptalCleanupSuruyor,
    KapatmaHatasi,
    SonDenetimHatasi,
    Tamamlandi,
}

/// Manager'ın kapanış/denetim sınırındaki güvenli hata sınıfı. Ham manager,
/// süreç veya SDK hata metni kayda ve kullanıcı/model çıktısına taşınmaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum McpCagriTemizlikHatasi {
    Kapatma,
    SonDenetim,
}

/// Manager/RPC dikişinin güvenli hata sınıfı. G7 dönüşüm closure'ı yalnız
/// başarılı `CallToolResult` alır; bu enum ham SDK hatası taşımaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum McpCagriRpcHatasi {
    OnayEskidi,
    Iletisim,
}

/// G7'nin güncel, denetlenmiş araç kartı. D, sunum, sonuç sanitizer'ı ve
/// uzak JSON-RPC protokol hatası sanitizer'ı zorunludur; ham RMCP tanımı veya
/// varsayılan dönüştürme yolu yoktur.
pub struct McpG7AracKarti {
    pub hazirlik: McpAracHazirligi,
    pub sunum: McpAracSunumu,
    pub sonuc_donusturucu: McpAracSonucDonusturucu,
    pub rpc_hata_donusturucu: McpAracRpcHataDonusturucu,
}

/// G7 adapter'ı her kart ve sonuç için gerçek doğrulanmış kaynağı sağlar.
#[async_trait]
pub trait McpG7AracKaynagi: Send + Sync {
    /// D güncellemesi aynı kapının yazma kilidini kullanır. Guard, son
    /// doğrulama ile gerçek RPC arasındaki await'lerde D'yi sabit tutar.
    async fn cagri_korumasini_al(
        &self,
        kimlik: &McpAracKimligi,
        onayli: &McpAracHazirligi,
    ) -> Result<tokio::sync::OwnedRwLockReadGuard<()>, AracHatasi>;

    async fn guncel_kart(
        &self,
        kimlik: &McpAracKimligi,
        aktif_tarif: McpTarifi,
    ) -> Result<McpG7AracKarti, AracHatasi>;
}

/// G5 adapter'ının RPC sonucuyla birlikte verdiği aynı kuşağın tarama tabanı.
/// Bu değer bulunmadan normal çağrı son denetime geçemez.
pub(crate) struct McpCagriRpcCevabi {
    /// G7 sanitizer'ından geçmiş sonuç; ham SDK nesnesi çağrı kaydına girmez.
    pub sonuc: Result<AracCiktisi, AracHatasi>,
    pub adli_girdiler: McpAdliGirdiler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct McpCagriKapanisSonucu {
    pub surec_kapandi: bool,
    pub son_denetim_basarili: bool,
}

/// RPC ve cleanup görevleri bu kayda Arc ile bağlıdır. Caller future'ının
/// bırakılması `kilit` alanını düşürmez; map'teki sahiplik ancak doğrulanmış
/// cleanup callback'inde kaldırılabilir.
pub(crate) struct McpCagriKaydi {
    pub cagri: CagriId,
    pub alan_kimligi: String,
    pub kok: std::path::PathBuf,
    pub sunucu: String,
    pub arac: String,
    /// Süreç kuşağı N ve çalışma tarifi R.
    pub kusak: McpBaslatmaSayaci,
    pub r: McpTarifi,
    /// Araç tanım özeti D.
    pub d: McpAracTanimOzeti,
    pub cagri_zamani_asimi: Duration,
    pub iptal: CancellationToken,
    kilit: Mutex<Option<KilitSeti>>,
    durum: Mutex<McpCagriDurumu>,
    adli_girdiler: Mutex<Option<McpAdliGirdiler>>,
    kapanis_sonucu: watch::Sender<Option<McpCagriKapanisSonucu>>,
    /// Aynı çağrı için normal son denetim ve iptal cleanup'ı yarışsa bile tek
    /// manager-owned finalizer çalışır. Sonuç watch'te kaldığından bekleyen
    /// hook, task drop'undan sonra da final sonucu okuyabilir.
    finalizer_baslatildi: AtomicBool,
    finalizer_sonucu: watch::Sender<Option<bool>>,
    rpc_baslatildi: AtomicBool,
}

impl McpCagriKaydi {
    fn yeni(
        cagri: CagriId,
        kimlik: &McpAracKimligi,
        hazirlik: McpAracHazirligi,
        iptal: CancellationToken,
        kilit: KilitSeti,
        cagri_zamani_asimi: Duration,
    ) -> Self {
        let (finalizer_sonucu, ilk_alici) = watch::channel(None);
        drop(ilk_alici);
        let (kapanis_sonucu, ilk_kapanis_alici) = watch::channel(None);
        drop(ilk_kapanis_alici);
        Self {
            cagri,
            alan_kimligi: kimlik.alan_kimligi().to_owned(),
            kok: kimlik.kok().to_path_buf(),
            sunucu: kimlik.sunucu().gorunen().to_owned(),
            arac: kimlik.arac().gorunen().to_owned(),
            kusak: hazirlik.tarif.baslatma(),
            r: hazirlik.tarif,
            d: hazirlik.arac_tanimi,
            cagri_zamani_asimi,
            iptal,
            kilit: Mutex::new(Some(kilit)),
            durum: Mutex::new(McpCagriDurumu::Hazirlaniyor),
            adli_girdiler: Mutex::new(None),
            kapanis_sonucu,
            finalizer_baslatildi: AtomicBool::new(false),
            finalizer_sonucu,
            rpc_baslatildi: AtomicBool::new(false),
        }
    }

    pub(crate) fn durum(&self) -> McpCagriDurumu {
        *self.durum.lock().unwrap_or_else(|hata| hata.into_inner())
    }

    fn durumu_ayarla(&self, yeni: McpCagriDurumu) {
        *self.durum.lock().unwrap_or_else(|hata| hata.into_inner()) = yeni;
    }

    fn adli_girdileri_ayarla(&self, yeni: McpAdliGirdiler) {
        *self
            .adli_girdiler
            .lock()
            .unwrap_or_else(|hata| hata.into_inner()) = Some(yeni);
    }

    pub(crate) fn adli_girdileri_al(&self) -> Option<McpAdliGirdiler> {
        self.adli_girdiler
            .lock()
            .unwrap_or_else(|hata| hata.into_inner())
            .clone()
    }

    #[cfg(test)]
    fn kilit_tutuluyor(&self) -> bool {
        self.kilit
            .lock()
            .unwrap_or_else(|hata| hata.into_inner())
            .is_some()
    }
}

/// Süreç yuvasından ayrı duran çağrı tablosu. Bu tablo manager alanı olarak
/// yaşamalıdır; sunucu yuvası/map girdisi silinince çağrı guard'ı düşmez.
#[derive(Default)]
pub(crate) struct McpCagriDefteri {
    kayitlar: Mutex<HashMap<CagriId, Arc<McpCagriKaydi>>>,
    /// Yuvaların N sayacı azalmadığı için son başarılı kapanış, daha eski
    /// kuşakların kapanışını da kanıtlar. Sunucu başına tek kayıt tutulur.
    son_dogrulanmis_kapanis: Mutex<HashMap<(String, String), McpBaslatmaSayaci>>,
}

/// Worker'a verilen tek kullanımlık RPC girdisi. Argümanlar manager kaydında
/// veya hata durumunda tutulmaz; yalnız bağımsız RPC future'ı yaşarken vardır.
pub(crate) struct McpCagriRpcGirdisi {
    pub kimlik: Arc<McpAracKimligi>,
    pub hazirlik: McpAracHazirligi,
    pub argumanlar: serde_json::Map<String, serde_json::Value>,
}

impl McpCagriDefteri {
    fn kaydi_ekle(
        &self,
        istek: McpAracCagriIstegi,
        cagri_zamani_asimi: Duration,
    ) -> Result<(Arc<McpCagriKaydi>, McpCagriRpcGirdisi), &'static str> {
        let mut kayitlar = self
            .kayitlar
            .lock()
            .unwrap_or_else(|hata| hata.into_inner());
        if kayitlar.contains_key(&istek.cagri) {
            return Err("MCP çağrı kimliği zaten etkin.");
        }
        let McpAracCagriIstegi {
            cagri,
            kimlik,
            hazirlik,
            argumanlar,
            iptal,
            kilit,
        } = istek;
        let kilit = kilit.ok_or("MCP çağrısının sahipli kilidi yok.")?;
        let kayit = Arc::new(McpCagriKaydi::yeni(
            cagri,
            &kimlik,
            hazirlik.clone(),
            iptal.clone(),
            kilit,
            cagri_zamani_asimi,
        ));
        kayitlar.insert(cagri, Arc::clone(&kayit));
        Ok((
            kayit,
            McpCagriRpcGirdisi {
                kimlik,
                hazirlik,
                argumanlar,
            },
        ))
    }

    fn kaydi_bul(&self, cagri: CagriId) -> Option<Arc<McpCagriKaydi>> {
        self.kayitlar
            .lock()
            .unwrap_or_else(|hata| hata.into_inner())
            .get(&cagri)
            .cloned()
    }

    /// Yalnız zorunlu son denetim başarılı olduktan sonra guard ve kayıt
    /// birlikte çözülür. Önce map'ten ayırmak, uyandırılan ikinci yazmanın
    /// eski kayıtla yarışmasını önler.
    fn dogrulanmis_temizlikle_sil(&self, cagri: CagriId) -> bool {
        let mut kayitlar = self
            .kayitlar
            .lock()
            .unwrap_or_else(|hata| hata.into_inner());
        let Some(kayit) = kayitlar.get(&cagri).cloned() else {
            return false;
        };
        let mut kilit = kayit.kilit.lock().unwrap_or_else(|hata| hata.into_inner());
        let Some(sahipli_kilit) = kilit.take() else {
            return false;
        };
        kayit.durumu_ayarla(McpCagriDurumu::Tamamlandi);
        kayitlar.remove(&cagri);
        drop(kilit);
        drop(kayitlar);
        drop(sahipli_kilit);
        true
    }

    #[cfg(test)]
    fn etkin(&self) -> usize {
        self.kayitlar
            .lock()
            .unwrap_or_else(|hata| hata.into_inner())
            .len()
    }

    /// G5'teki explicit stop da yalnız aynı süreç kuşağı için çağrıyı iptal
    /// eder ve kapanış kanıtını kayda bırakır. Guard burada çözülmez.
    pub(crate) fn surec_kapanisini_bildir(
        &self,
        alan_kimligi: &str,
        sunucu: &str,
        kusak: McpBaslatmaSayaci,
        sonuc: McpCagriKapanisSonucu,
    ) {
        if sonuc.surec_kapandi && sonuc.son_denetim_basarili {
            let mut kanitlar = self
                .son_dogrulanmis_kapanis
                .lock()
                .unwrap_or_else(|h| h.into_inner());
            let son = kanitlar
                .entry((alan_kimligi.to_owned(), sunucu.to_owned()))
                .or_default();
            *son = (*son).max(kusak);
        }
        let kayitlar: Vec<_> = self
            .kayitlar
            .lock()
            .unwrap_or_else(|hata| hata.into_inner())
            .values()
            .filter(|kayit| {
                kayit.alan_kimligi == alan_kimligi && kayit.sunucu == sunucu && kayit.kusak == kusak
            })
            .cloned()
            .collect();
        for kayit in kayitlar {
            kayit.kapanis_sonucu.send_replace(Some(sonuc));
            kayit.iptal.cancel();
        }
    }

    fn kapanis_dogrulandi(&self, kayit: &McpCagriKaydi) -> bool {
        self.son_dogrulanmis_kapanis
            .lock()
            .unwrap_or_else(|h| h.into_inner())
            .get(&(kayit.alan_kimligi.clone(), kayit.sunucu.clone()))
            .is_some_and(|son| *son >= kayit.kusak)
    }

    pub(super) fn alan_engelli(&self, alan: &str) -> bool {
        self.kayitlar
            .lock()
            .unwrap_or_else(|h| h.into_inner())
            .values()
            .any(|k| {
                k.alan_kimligi == alan
                    && matches!(
                        k.durum(),
                        McpCagriDurumu::KapatmaHatasi | McpCagriDurumu::SonDenetimHatasi
                    )
            })
    }
}

/// App manager'a bağlanan zorunlu dikiş. `rpc_guncelse`, son R/D/Π/K/P/N
/// karşılaştırmasını ve G5 `McpYoneticisi::arac_cagir` çağrısını tek app
/// işleminde yapar: eşitlik bozulursa manager RPC'si hiç çağrılmaz. G5'in
/// süreç-stop callback'i aynı `(alan, sunucu, N)` kaydını uyandırmalıdır.
#[async_trait]
pub(crate) trait McpCagriArkaUcu: Send + Sync {
    async fn guncel_hazirlik(
        &self,
        kimlik: &McpAracKimligi,
    ) -> Result<McpAracHazirligi, AracHatasi>;

    async fn guncelligi_dogrula(
        &self,
        kimlik: &McpAracKimligi,
        onayli: &McpAracHazirligi,
    ) -> Result<(), AracHatasi>;

    /// Var olan `McpSunucuKaydi.cagri_zamani_asimi_sn` değerini döndürür;
    /// bulunmayan/geçersiz ayar fail-closed hatadır, varsayılan süre yoktur.
    fn cagri_zamani_asimi(&self, kimlik: &McpAracKimligi) -> Result<Duration, ()>;

    /// Son tazelik kontrolü bu metoda aittir. Uygulama D kaynağının kilidini
    /// doğrulama ile manager RPC dispatch'i boyunca koruyup, arada await
    /// bırakmadan mevcut G5 `arac_cagir` yoluna girmelidir.
    async fn rpc_guncelse(
        &self,
        kayit: Arc<McpCagriKaydi>,
        girdi: McpCagriRpcGirdisi,
    ) -> Result<McpCagriRpcCevabi, McpCagriRpcHatasi>;

    /// İptal/deadline/drop yolunda SDK çağrı future'ı bırakıldıktan sonra
    /// mevcut G5 generation-check'li kapanış ve son denetim tamamlanır.
    async fn iptal_kapat_ve_son_denetle(
        &self,
        kayit: Arc<McpCagriKaydi>,
    ) -> Result<(), McpCagriTemizlikHatasi>;

    /// Normal RPC'de core'un sonuç kancası geldikten sonra zorunlu son tarama.
    async fn son_denetim(&self, kayit: Arc<McpCagriKaydi>) -> Result<(), McpCagriTemizlikHatasi>;
}

/// G5 manager ile zorunlu G7 kaynağını birleştiren gerçek adapter.
/// `yonetici.rs` içinde child module olarak tutulduğu için yalnız mevcut
/// çalışan kuşağın özel, tipli durumunu okur; yeni public manager API'si açmaz.
pub(crate) struct McpCagriG5ArkaUcu {
    yonetici: Arc<super::McpYoneticisi>,
    g7: Arc<dyn McpG7AracKaynagi>,
}

impl McpCagriG5ArkaUcu {
    pub(crate) fn yeni(yonetici: Arc<super::McpYoneticisi>, g7: Arc<dyn McpG7AracKaynagi>) -> Self {
        Self { yonetici, g7 }
    }

    fn calisan_kusak_bul(
        &self,
        kimlik: &McpAracKimligi,
    ) -> Result<Arc<super::CalisanKusak>, AracHatasi> {
        let d = self.yonetici.durum();
        let Some(calisan) = d
            .yuvalar
            .get(&super::McpAnahtari::yeni(
                kimlik.alan_kimligi(),
                kimlik.sunucu().gorunen(),
            ))
            .and_then(|yuva| yuva.calisan.clone())
        else {
            return Err(AracHatasi::OnayEskidi);
        };
        if calisan.iptal.is_cancelled() {
            return Err(AracHatasi::OnayEskidi);
        }
        Ok(calisan)
    }

    async fn guncel_kart(&self, kimlik: &McpAracKimligi) -> Result<McpG7AracKarti, AracHatasi> {
        self.yonetici
            .sunucu_korumasini_bekleyerek_denetle(
                kimlik.alan_kimligi(),
                kimlik.sunucu().gorunen(),
                kimlik.kok(),
            )
            .await
            .map_err(|_| AracHatasi::OnayEskidi)?;
        let calisan = self.calisan_kusak_bul(kimlik)?;
        let kart = self.g7.guncel_kart(kimlik, calisan.tarif).await?;
        let son_calisan = self.calisan_kusak_bul(kimlik)?;
        if son_calisan.kusak != calisan.kusak || son_calisan.tarif != calisan.tarif {
            return Err(AracHatasi::OnayEskidi);
        }
        if kart.hazirlik.tarif != calisan.tarif
            || &kart.hazirlik.sunucu != kimlik.sunucu()
            || &kart.hazirlik.arac != kimlik.arac()
            || kart.sunum.ad
                != format!(
                    "mcp__{}__{}",
                    kimlik.sunucu().gorunen(),
                    kimlik.arac().gorunen()
                )
        {
            return Err(AracHatasi::OnayEskidi);
        }
        Ok(kart)
    }

    fn kapanis_hatasini_coz(sonuc: McpCagriKapanisSonucu) -> Result<(), McpCagriTemizlikHatasi> {
        if !sonuc.surec_kapandi {
            Err(McpCagriTemizlikHatasi::Kapatma)
        } else if !sonuc.son_denetim_basarili {
            Err(McpCagriTemizlikHatasi::SonDenetim)
        } else {
            Ok(())
        }
    }

    async fn kapanis_sonucunu_bekle(
        &self,
        kayit: &McpCagriKaydi,
    ) -> Result<(), McpCagriTemizlikHatasi> {
        let mut kapanis = kayit.kapanis_sonucu.subscribe();
        loop {
            if let Some(sonuc) = *kapanis.borrow_and_update() {
                return Self::kapanis_hatasini_coz(sonuc);
            }
            kapanis
                .changed()
                .await
                .map_err(|_| McpCagriTemizlikHatasi::Kapatma)?;
        }
    }
}

#[async_trait]
impl McpCagriArkaUcu for McpCagriG5ArkaUcu {
    async fn guncel_hazirlik(
        &self,
        kimlik: &McpAracKimligi,
    ) -> Result<McpAracHazirligi, AracHatasi> {
        // Yalnız hazırlık yeni N/R oluşturabilir. RPC'nin son doğrulaması
        // yeni süreç başlatmaz; eski mühür yeni kuşağa taşınamaz.
        self.yonetici
            .baslat_veya_dogrula(
                kimlik.alan_kimligi(),
                kimlik.kok(),
                kimlik.sunucu().gorunen(),
            )
            .await
            .map_err(|_| AracHatasi::OnayEskidi)?;
        Ok(self.guncel_kart(kimlik).await?.hazirlik)
    }

    async fn guncelligi_dogrula(
        &self,
        kimlik: &McpAracKimligi,
        onayli: &McpAracHazirligi,
    ) -> Result<(), AracHatasi> {
        let taze = self.guncel_kart(kimlik).await?;
        if &taze.hazirlik == onayli {
            Ok(())
        } else {
            Err(AracHatasi::OnayEskidi)
        }
    }

    fn cagri_zamani_asimi(&self, kimlik: &McpAracKimligi) -> Result<Duration, ()> {
        let ayarlar = self.yonetici.mcp_depo.mcp_ayarlarini_oku().0;
        let sure = ayarlar
            .sunucu(kimlik.sunucu().gorunen())
            .map(|sunucu| sunucu.cagri_zamani_asimi_sn)
            .filter(|sure| *sure > 0)
            .ok_or(())?;
        Ok(Duration::from_secs(u64::from(sure)))
    }

    async fn rpc_guncelse(
        &self,
        kayit: Arc<McpCagriKaydi>,
        girdi: McpCagriRpcGirdisi,
    ) -> Result<McpCagriRpcCevabi, McpCagriRpcHatasi> {
        let _tanim_koruma = self
            .g7
            .cagri_korumasini_al(&girdi.kimlik, &girdi.hazirlik)
            .await
            .map_err(|_| McpCagriRpcHatasi::OnayEskidi)?;
        let kart = self
            .guncel_kart(&girdi.kimlik)
            .await
            .map_err(|_| McpCagriRpcHatasi::OnayEskidi)?;
        if kart.hazirlik != girdi.hazirlik
            || kart.hazirlik.tarif != kayit.r
            || kart.hazirlik.arac_tanimi != kayit.d
            || kart.hazirlik.tarif.baslatma() != kayit.kusak
        {
            return Err(McpCagriRpcHatasi::OnayEskidi);
        }

        let calisan = self
            .yonetici
            .aktif_tarifli_kusak_bul(&kayit.alan_kimligi, &kayit.sunucu, &kayit.r)
            .map_err(|_| McpCagriRpcHatasi::OnayEskidi)?;
        // Adli karşılaştırmanın tabanı çağrıdan önce sabitlenir. Mutlak yaş
        // taraması RPC sırasında çalışsa da bu çağrının önceki hâlini ezemez.
        let adli_girdiler = calisan
            .adli_girdiler
            .lock()
            .map_err(|_| McpCagriRpcHatasi::Iletisim)?
            .clone();
        let sonuc = match self
            .yonetici
            .arac_cagir_cevabi(
                &kayit.alan_kimligi,
                &kayit.kok,
                &kayit.sunucu,
                &kayit.r,
                &kayit.arac,
                girdi.argumanlar,
            )
            .await
        {
            Ok(cevap) => (kart.sonuc_donusturucu)(cevap.sonuc, cevap.content_alani_var),
            Err(super::YoneticiHatasi::UzakRpc(rpc)) => Err((kart.rpc_hata_donusturucu)(&rpc)),
            Err(_) => return Err(McpCagriRpcHatasi::Iletisim),
        };
        Ok(McpCagriRpcCevabi {
            sonuc,
            adli_girdiler,
        })
    }

    async fn iptal_kapat_ve_son_denetle(
        &self,
        kayit: Arc<McpCagriKaydi>,
    ) -> Result<(), McpCagriTemizlikHatasi> {
        if self.yonetici.cagri_defteri.kapanis_dogrulandi(&kayit) {
            return Ok(());
        }
        let mut kapanis = kayit.kapanis_sonucu.subscribe();
        if let Some(sonuc) = *kapanis.borrow_and_update() {
            return Self::kapanis_hatasini_coz(sonuc);
        }
        // Eski çağrının cleanup'ı yeni kuşağı ayıramaz. Seçim ve ayırma
        // aynı yönetici kilidinde yapılır; bekleme bağımsız kapanış işindedir.
        let anahtar = super::McpAnahtari::yeni(&kayit.alan_kimligi, &kayit.sunucu);
        let cikarilan = {
            let mut d = self.yonetici.durum();
            if d.yuvalar
                .get(&anahtar)
                .and_then(|y| y.calisan.as_ref())
                .is_some_and(|c| c.kusak == kayit.kusak)
            {
                super::McpYoneticisi::yuvadan_sureci_ayir(
                    &mut d,
                    &anahtar,
                    super::YoneticiHatasi::SunucuDurduruldu,
                    super::McpSunucuDurumu::Durdu,
                )
            } else {
                None
            }
        };
        if let Some((anahtar, kusak, kok)) = cikarilan {
            let _ = self.yonetici.kapat_ve_denetle(anahtar, kusak, kok).await;
        }
        if self.yonetici.cagri_defteri.kapanis_dogrulandi(&kayit) {
            return Ok(());
        }
        if let Some(sonuc) = *kapanis.borrow_and_update() {
            return Self::kapanis_hatasini_coz(sonuc);
        }
        self.kapanis_sonucunu_bekle(&kayit).await
    }

    async fn son_denetim(&self, kayit: Arc<McpCagriKaydi>) -> Result<(), McpCagriTemizlikHatasi> {
        let onceki = kayit
            .adli_girdileri_al()
            .ok_or(McpCagriTemizlikHatasi::SonDenetim)?;
        let sonuc = self
            .yonetici
            .koruma_denetleyicisi
            .son_denetim(SonDenetimIstegi {
                kok: kayit.kok.clone(),
                gizli_ayarlari: self.yonetici.ayarlar.gizli_ayarlari(),
                onceki,
            })
            .await;
        if !sonuc.mcp_icin_basarili_mi() {
            return Err(McpCagriTemizlikHatasi::SonDenetim);
        }
        let yeni_adli_girdiler = sonuc
            .guncel_adli_girdiler
            .ok_or(McpCagriTemizlikHatasi::SonDenetim)?;
        match self
            .yonetici
            .aktif_kusak_bul(&kayit.alan_kimligi, &kayit.sunucu, kayit.kusak)
        {
            Ok(calisan) => {
                let mut adli = calisan
                    .adli_girdiler
                    .lock()
                    .map_err(|_| McpCagriTemizlikHatasi::SonDenetim)?;
                *adli = yeni_adli_girdiler;
                Ok(())
            }
            Err(_) => self.kapanis_sonucunu_bekle(&kayit).await,
        }
    }
}

/// Uygulama aracı ile manager-owned çağrı defteri arasındaki worker.
pub(crate) struct McpCagriYurutucusu {
    defter: Arc<McpCagriDefteri>,
    arka_ucl: Arc<dyn McpCagriArkaUcu>,
}

impl McpCagriYurutucusu {
    pub(crate) fn yeni(defter: Arc<McpCagriDefteri>, arka_ucl: Arc<dyn McpCagriArkaUcu>) -> Self {
        Self { defter, arka_ucl }
    }

    async fn rpc_worker(
        yurutucu: Arc<Self>,
        kayit: Arc<McpCagriKaydi>,
        girdi: McpCagriRpcGirdisi,
        mut yanit: oneshot::Sender<Result<AracCiktisi, AracHatasi>>,
    ) {
        let McpCagriRpcGirdisi {
            kimlik, hazirlik, ..
        } = &girdi;
        let kimlik = Arc::clone(kimlik);
        let hazirlik = hazirlik.clone();
        let arka_ucl = Arc::clone(&yurutucu.arka_ucl);
        let kayit_rpc = Arc::clone(&kayit);
        let cagri_future = async move {
            arka_ucl
                .guncelligi_dogrula(&kimlik, &hazirlik)
                .await
                .map_err(|_| McpCagriRpcHatasi::OnayEskidi)?;
            arka_ucl.rpc_guncelse(kayit_rpc, girdi).await
        };
        let son_an = Instant::now() + kayit.cagri_zamani_asimi;
        let olay = tokio::select! {
            biased;
            _ = kayit.iptal.cancelled() => McpRpcOlayi::Iptal,
            _ = yanit.closed() => McpRpcOlayi::YanitAlaniKapandi,
            _ = tokio::time::sleep_until(son_an) => McpRpcOlayi::ZamanAsimi,
            sonuc = cagri_future => McpRpcOlayi::Rpc(sonuc.map(Box::new)),
        };
        // `select!` scope'u kapanınca kaybeden `cagri_future` düşer. RMCP
        // istemcisinde bu, yanıt oneshot alıcısını kapatıp cancel bildirimi
        // gönderen yoldur. Süreç kapanış kanıtı değildir; aşağıdaki cleanup
        // ayrıca gerçek G5 process stop ve son denetimini bekler.

        match olay {
            McpRpcOlayi::Rpc(Ok(cevap)) => {
                if yanit.is_closed() {
                    Self::iptal_cleanup_ve_bekle(Arc::clone(&yurutucu), Arc::clone(&kayit)).await;
                    return;
                }
                kayit.adli_girdileri_ayarla(cevap.adli_girdiler);
                kayit.durumu_ayarla(McpCagriDurumu::DenetimBekliyor);
                if yanit.send(cevap.sonuc).is_err() {
                    Self::iptal_cleanup_ve_bekle(yurutucu, kayit).await;
                }
            }
            olay => {
                kayit.iptal.cancel();
                Self::iptal_cleanup_ve_bekle(Arc::clone(&yurutucu), Arc::clone(&kayit)).await;
                let hata = match olay {
                    McpRpcOlayi::Iptal => AracHatasi::Iptal,
                    McpRpcOlayi::YanitAlaniKapandi => AracHatasi::Iptal,
                    McpRpcOlayi::ZamanAsimi => AracHatasi::Gecersiz(GUVENLI_ZAMAN_ASIMI.to_owned()),
                    McpRpcOlayi::Rpc(Err(McpCagriRpcHatasi::OnayEskidi)) => AracHatasi::OnayEskidi,
                    McpRpcOlayi::Rpc(Err(McpCagriRpcHatasi::Iletisim))
                    | McpRpcOlayi::Rpc(Ok(_)) => Self::guvenli_hata(),
                };
                let _ = yanit.send(Err(hata));
            }
        }
    }

    async fn iptal_cleanup_ve_bekle(yurutucu: Arc<Self>, kayit: Arc<McpCagriKaydi>) -> bool {
        kayit.iptal.cancel();
        Self::finalizeri_baslat(
            Arc::clone(&yurutucu.defter),
            Arc::clone(&yurutucu.arka_ucl),
            Arc::clone(&kayit),
            true,
        );
        Self::finalizer_sonucunu_bekle(kayit).await
    }

    fn finalizeri_baslat(
        defter: Arc<McpCagriDefteri>,
        arka_ucl: Arc<dyn McpCagriArkaUcu>,
        kayit: Arc<McpCagriKaydi>,
        zorunlu_kapatma: bool,
    ) {
        if !zorunlu_kapatma && kayit.durum() != McpCagriDurumu::DenetimBekliyor {
            return;
        }
        let handle = match tokio::runtime::Handle::try_current() {
            Ok(handle) => handle,
            Err(_) => {
                kayit.durumu_ayarla(McpCagriDurumu::KapatmaHatasi);
                kayit.finalizer_sonucu.send_replace(Some(false));
                return;
            }
        };
        if kayit
            .finalizer_baslatildi
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        kayit.durumu_ayarla(if zorunlu_kapatma {
            McpCagriDurumu::IptalCleanupSuruyor
        } else {
            McpCagriDurumu::SonDenetimSuruyor
        });
        handle.spawn(async move {
            let sonuc = if zorunlu_kapatma {
                Self::kapatma_ve_denetim(Arc::clone(&arka_ucl), Arc::clone(&kayit)).await
            } else {
                if kayit.adli_girdileri_al().is_none() {
                    Err(McpCagriTemizlikHatasi::SonDenetim)
                } else {
                    tokio::select! {
                        biased;
                        _ = kayit.iptal.cancelled() => {
                            kayit.durumu_ayarla(McpCagriDurumu::IptalCleanupSuruyor);
                            Self::kapatma_ve_denetim(Arc::clone(&arka_ucl), Arc::clone(&kayit)).await
                        }
                        sonuc = arka_ucl.son_denetim(Arc::clone(&kayit)) => sonuc,
                    }
                }
            };
            match sonuc {
                Ok(()) => {
                    if defter.dogrulanmis_temizlikle_sil(kayit.cagri) {
                        kayit.finalizer_sonucu.send_replace(Some(true));
                    } else {
                        kayit.durumu_ayarla(McpCagriDurumu::SonDenetimHatasi);
                        kayit.finalizer_sonucu.send_replace(Some(false));
                    }
                }
                Err(McpCagriTemizlikHatasi::Kapatma) => {
                    kayit.durumu_ayarla(McpCagriDurumu::KapatmaHatasi);
                    kayit.finalizer_sonucu.send_replace(Some(false));
                }
                Err(McpCagriTemizlikHatasi::SonDenetim) => {
                    kayit.durumu_ayarla(McpCagriDurumu::SonDenetimHatasi);
                    kayit.finalizer_sonucu.send_replace(Some(false));
                }
            }
        });
    }

    async fn kapatma_ve_denetim(
        arka_ucl: Arc<dyn McpCagriArkaUcu>,
        kayit: Arc<McpCagriKaydi>,
    ) -> Result<(), McpCagriTemizlikHatasi> {
        // RPC'nin kısa deadline'ı zorunlu kapanışı yarıda kesemez.
        // Süreç ağacı kapanışı ve taramanın kendi sınırları geçerlidir.
        arka_ucl.iptal_kapat_ve_son_denetle(kayit).await
    }

    async fn finalizer_sonucunu_bekle(kayit: Arc<McpCagriKaydi>) -> bool {
        let mut sonuc = kayit.finalizer_sonucu.subscribe();
        loop {
            if let Some(basarili) = *sonuc.borrow_and_update() {
                return basarili;
            }
            if sonuc.changed().await.is_err() {
                return false;
            }
        }
    }

    fn guvenli_hata() -> AracHatasi {
        AracHatasi::Gecersiz(GUVENLI_CAGRI_HATASI.to_owned())
    }
}

enum McpRpcOlayi {
    Rpc(Result<Box<McpCagriRpcCevabi>, McpCagriRpcHatasi>),
    Iptal,
    YanitAlaniKapandi,
    ZamanAsimi,
}

#[async_trait]
impl McpAracYurutucusu for McpCagriYurutucusu {
    fn kilidi_devral(&self, istek: McpAracCagriIstegi) -> Result<(), AracHatasi> {
        let zaman = self
            .arka_ucl
            .cagri_zamani_asimi(&istek.kimlik)
            .map_err(|_| Self::guvenli_hata())?;
        if zaman.is_zero() {
            return Err(Self::guvenli_hata());
        }
        self.defter
            .kaydi_ekle(istek, zaman)
            .map_err(|_| Self::guvenli_hata())?;
        Ok(())
    }
    async fn guncel_hazirlik(
        &self,
        kimlik: &McpAracKimligi,
    ) -> Result<McpAracHazirligi, AracHatasi> {
        self.arka_ucl.guncel_hazirlik(kimlik).await
    }

    async fn guncelligi_dogrula(
        &self,
        kimlik: &McpAracKimligi,
        onayli: &McpAracHazirligi,
    ) -> Result<(), AracHatasi> {
        self.arka_ucl.guncelligi_dogrula(kimlik, onayli).await
    }

    fn cagriyi_baslat(
        self: Arc<Self>,
        istek: McpAracCagriIstegi,
    ) -> Result<oneshot::Receiver<Result<AracCiktisi, AracHatasi>>, AracHatasi> {
        let handle = tokio::runtime::Handle::try_current().map_err(|_| Self::guvenli_hata())?;
        let cagri_zamani_asimi = self
            .arka_ucl
            .cagri_zamani_asimi(&istek.kimlik)
            .map_err(|_| Self::guvenli_hata())?;
        if cagri_zamani_asimi.is_zero() {
            return Err(Self::guvenli_hata());
        }
        let (kayit, girdi) = if istek.kilit.is_some() {
            self.defter
                .kaydi_ekle(istek, cagri_zamani_asimi)
                .map_err(|_| Self::guvenli_hata())?
        } else {
            let kayit = self
                .defter
                .kaydi_bul(istek.cagri)
                .ok_or_else(Self::guvenli_hata)?;
            if kayit.r != istek.hazirlik.tarif
                || kayit.d != istek.hazirlik.arac_tanimi
                || kayit.alan_kimligi != istek.kimlik.alan_kimligi()
                || kayit.kok != istek.kimlik.kok()
                || kayit.sunucu != istek.kimlik.sunucu().gorunen()
                || kayit.arac != istek.kimlik.arac().gorunen()
            {
                return Err(AracHatasi::OnayEskidi);
            }
            let girdi = McpCagriRpcGirdisi {
                kimlik: istek.kimlik,
                hazirlik: istek.hazirlik,
                argumanlar: istek.argumanlar,
            };
            (kayit, girdi)
        };
        if kayit.rpc_baslatildi.swap(true, Ordering::AcqRel) || kayit.iptal.is_cancelled() {
            return Err(Self::guvenli_hata());
        }
        kayit.durumu_ayarla(McpCagriDurumu::RpcCalisiyor);
        let (yanit, alici) = oneshot::channel();
        handle.spawn(async move {
            Self::rpc_worker(self, kayit, girdi, yanit).await;
        });
        Ok(alici)
    }

    async fn sonuc_isleme_bitti(&self, cagri: CagriId) -> Result<(), AracHatasi> {
        let Some(kayit) = self.defter.kaydi_bul(cagri) else {
            // İptal/taşıma cleanup'ı başarılı biçimde guard'ı daha önce
            // çözdüyse core aynı çağrının tamamlanma kancasını çağırabilir.
            return Ok(());
        };
        match kayit.durum() {
            McpCagriDurumu::DenetimBekliyor => Self::finalizeri_baslat(
                Arc::clone(&self.defter),
                Arc::clone(&self.arka_ucl),
                Arc::clone(&kayit),
                false,
            ),
            McpCagriDurumu::SonDenetimSuruyor | McpCagriDurumu::IptalCleanupSuruyor => {}
            McpCagriDurumu::Tamamlandi => return Ok(()),
            McpCagriDurumu::KapatmaHatasi | McpCagriDurumu::SonDenetimHatasi => {
                return Err(AracHatasi::SonucIsleme);
            }
            McpCagriDurumu::Hazirlaniyor | McpCagriDurumu::RpcCalisiyor => {
                // Taze hazırlık generic hata verdiyse henüz RPC yanıtı yoktur.
                // Guard kayıtlıdır; bu terminal çıkış da doğrulanmış cleanup ister.
                kayit.iptal.cancel();
                Self::finalizeri_baslat(
                    Arc::clone(&self.defter),
                    Arc::clone(&self.arka_ucl),
                    Arc::clone(&kayit),
                    true,
                );
            }
        }
        if Self::finalizer_sonucunu_bekle(kayit).await {
            Ok(())
        } else {
            Err(AracHatasi::SonucIsleme)
        }
    }

    fn sonuc_isleme_iptal(&self, cagri: CagriId) {
        let Some(kayit) = self.defter.kaydi_bul(cagri) else {
            return;
        };
        kayit.iptal.cancel();
        Self::finalizeri_baslat(
            Arc::clone(&self.defter),
            Arc::clone(&self.arka_ucl),
            kayit,
            true,
        );
    }

    async fn sonuc_isleme_iptal_ve_bekle(&self, cagri: CagriId) -> Result<(), AracHatasi> {
        let Some(kayit) = self.defter.kaydi_bul(cagri) else {
            return Ok(());
        };
        kayit.iptal.cancel();
        Self::finalizeri_baslat(
            Arc::clone(&self.defter),
            Arc::clone(&self.arka_ucl),
            Arc::clone(&kayit),
            true,
        );
        if Self::finalizer_sonucunu_bekle(kayit).await {
            Ok(())
        } else {
            Err(AracHatasi::SonucIsleme)
        }
    }
}

impl super::McpYoneticisi {
    /// G6 çağrı manager'ını, bu manager'ın bağımsız çağrı defteriyle kurar.
    pub fn arac_yurutucusu(
        self: &Arc<Self>,
        g7: Arc<dyn McpG7AracKaynagi>,
    ) -> Arc<dyn McpAracYurutucusu> {
        let arka_uc = Arc::new(McpCagriG5ArkaUcu::yeni(Arc::clone(self), g7));
        Arc::new(McpCagriYurutucusu::yeni(
            Arc::clone(&self.cagri_defteri),
            arka_uc,
        ))
    }

    /// Zorunlu G7 kaynağından güncel D, güvenli sunum ve sanitizer alarak
    /// çağrılabilir core aracını kurar. Varsayılan/ham sonuç yolu yoktur.
    pub async fn arac_olustur(
        self: &Arc<Self>,
        kimlik: McpAracKimligi,
        g7: Arc<dyn McpG7AracKaynagi>,
    ) -> Result<Arc<dyn kalfa_core::arac::Arac>, AracHatasi> {
        let adapter = McpCagriG5ArkaUcu::yeni(Arc::clone(self), Arc::clone(&g7));
        let kart = adapter.guncel_kart(&kimlik).await?;
        let yurutucu = self.arac_yurutucusu(g7);
        Ok(Arc::new(McpArac::yeni(kimlik, kart.sunum, yurutucu)))
    }
}

#[cfg(test)]
mod worker_kabul_testleri {
    use super::*;
    use kalfa_core::arac::{
        McpAgKarti, McpAracTanimOzeti, McpBaslatmaSayaci, McpErisim, McpKartEtiketi,
        McpKorumaOzeti, McpPolitikaOzeti, McpTanimOzeti, McpYalitimProfiliOzeti,
    };
    use kalfa_core::izin::Kip;
    use kalfa_core::kilit::{KaynakKilitleri, KilitYoneticisi};
    use kalfa_mcp::arac::{McpAracCagriIstegi, McpAracKimligi, McpAracYurutucusu};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use tokio::sync::{Notify, oneshot};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct SahteSurum {
        r: u64,
        d: u64,
        pi: u64,
    }

    #[derive(Clone, Copy)]
    enum DegisenBilesen {
        R,
        D,
        Pi,
    }

    struct SahteArkaUc {
        onayli: SahteSurum,
        guncel: Mutex<SahteSurum>,
        sonraki_tazelik_degisim: Mutex<Option<DegisenBilesen>>,
        zaman_asimi_ms: AtomicU64,
        rpc_sayisi: AtomicUsize,
        rpc_beklet: AtomicBool,
        rpc_basladi: Notify,
        rpc_devam: Notify,
        cleanup_sayisi: AtomicUsize,
        cleanup_beklet: AtomicBool,
        cleanup_basladi: Notify,
        cleanup_devam: Notify,
        kapatma_hatasi: AtomicBool,
        cleanup_denetim_hatasi: AtomicBool,
        son_denetim_sayisi: AtomicUsize,
        son_denetim_beklet: AtomicBool,
        son_denetim_basladi: Notify,
        son_denetim_devam: Notify,
        son_denetim_hatasi: AtomicBool,
    }

    impl SahteArkaUc {
        fn yeni() -> Arc<Self> {
            let onayli = SahteSurum { r: 1, d: 1, pi: 1 };
            Arc::new(Self {
                onayli,
                guncel: Mutex::new(onayli),
                sonraki_tazelik_degisim: Mutex::new(None),
                zaman_asimi_ms: AtomicU64::new(1_000),
                rpc_sayisi: AtomicUsize::new(0),
                rpc_beklet: AtomicBool::new(false),
                rpc_basladi: Notify::new(),
                rpc_devam: Notify::new(),
                cleanup_sayisi: AtomicUsize::new(0),
                cleanup_beklet: AtomicBool::new(false),
                cleanup_basladi: Notify::new(),
                cleanup_devam: Notify::new(),
                kapatma_hatasi: AtomicBool::new(false),
                cleanup_denetim_hatasi: AtomicBool::new(false),
                son_denetim_sayisi: AtomicUsize::new(0),
                son_denetim_beklet: AtomicBool::new(false),
                son_denetim_basladi: Notify::new(),
                son_denetim_devam: Notify::new(),
                son_denetim_hatasi: AtomicBool::new(false),
            })
        }

        fn sonraki_kapida_degistir(&self, bilesen: DegisenBilesen) {
            *self
                .sonraki_tazelik_degisim
                .lock()
                .unwrap_or_else(|hata| hata.into_inner()) = Some(bilesen);
        }
    }

    #[async_trait]
    impl McpCagriArkaUcu for SahteArkaUc {
        async fn guncel_hazirlik(
            &self,
            _kimlik: &McpAracKimligi,
        ) -> Result<McpAracHazirligi, AracHatasi> {
            Ok(hazirlik())
        }

        async fn guncelligi_dogrula(
            &self,
            _kimlik: &McpAracKimligi,
            _onayli: &McpAracHazirligi,
        ) -> Result<(), AracHatasi> {
            let mut guncel = self.guncel.lock().unwrap_or_else(|hata| hata.into_inner());
            if *guncel != self.onayli {
                return Err(AracHatasi::OnayEskidi);
            }
            let degisim = self
                .sonraki_tazelik_degisim
                .lock()
                .unwrap_or_else(|hata| hata.into_inner())
                .take();
            match degisim {
                Some(DegisenBilesen::R) => guncel.r = guncel.r.saturating_add(1),
                Some(DegisenBilesen::D) => guncel.d = guncel.d.saturating_add(1),
                Some(DegisenBilesen::Pi) => guncel.pi = guncel.pi.saturating_add(1),
                None => {}
            }
            Ok(())
        }

        fn cagri_zamani_asimi(&self, _kimlik: &McpAracKimligi) -> Result<Duration, ()> {
            Ok(Duration::from_millis(
                self.zaman_asimi_ms.load(Ordering::SeqCst),
            ))
        }

        async fn rpc_guncelse(
            &self,
            _kayit: Arc<McpCagriKaydi>,
            _girdi: McpCagriRpcGirdisi,
        ) -> Result<McpCagriRpcCevabi, McpCagriRpcHatasi> {
            // R, D ve Π için son senkron sayaç kontrolü RPC sayacından önce gelir.
            if *self.guncel.lock().unwrap_or_else(|hata| hata.into_inner()) != self.onayli {
                return Err(McpCagriRpcHatasi::OnayEskidi);
            }
            self.rpc_sayisi.fetch_add(1, Ordering::SeqCst);
            self.rpc_basladi.notify_one();
            if self.rpc_beklet.load(Ordering::SeqCst) {
                self.rpc_devam.notified().await;
            }
            Ok(McpCagriRpcCevabi {
                sonuc: Ok(AracCiktisi {
                    icerik: "önceden maskelenmiş test sonucu".to_owned(),
                    ..AracCiktisi::default()
                }),
                adli_girdiler: McpAdliGirdiler::default(),
            })
        }

        async fn iptal_kapat_ve_son_denetle(
            &self,
            _kayit: Arc<McpCagriKaydi>,
        ) -> Result<(), McpCagriTemizlikHatasi> {
            self.cleanup_sayisi.fetch_add(1, Ordering::SeqCst);
            self.cleanup_basladi.notify_one();
            if self.cleanup_beklet.load(Ordering::SeqCst) {
                self.cleanup_devam.notified().await;
            }
            if self.kapatma_hatasi.load(Ordering::SeqCst) {
                return Err(McpCagriTemizlikHatasi::Kapatma);
            }
            if self.cleanup_denetim_hatasi.load(Ordering::SeqCst) {
                return Err(McpCagriTemizlikHatasi::SonDenetim);
            }
            Ok(())
        }

        async fn son_denetim(
            &self,
            _kayit: Arc<McpCagriKaydi>,
        ) -> Result<(), McpCagriTemizlikHatasi> {
            self.son_denetim_sayisi.fetch_add(1, Ordering::SeqCst);
            self.son_denetim_basladi.notify_one();
            if self.son_denetim_beklet.load(Ordering::SeqCst) {
                self.son_denetim_devam.notified().await;
            }
            if self.son_denetim_hatasi.load(Ordering::SeqCst) {
                Err(McpCagriTemizlikHatasi::SonDenetim)
            } else {
                Ok(())
            }
        }
    }

    struct Kurulum {
        yurutucu: Arc<McpCagriYurutucusu>,
        defter: Arc<McpCagriDefteri>,
        kaynaklar: Arc<KaynakKilitleri>,
        kimlik: Arc<McpAracKimligi>,
        hazirlik: McpAracHazirligi,
    }

    impl Kurulum {
        fn yeni(gecici: &tempfile::TempDir, arka_uc: Arc<SahteArkaUc>) -> Self {
            let yonetici = KilitYoneticisi::yeni();
            let kaynaklar = yonetici.al(gecici.path()).unwrap();
            let defter = Arc::new(McpCagriDefteri::default());
            let yurutucu = Arc::new(McpCagriYurutucusu::yeni(
                Arc::clone(&defter),
                arka_uc.clone(),
            ));
            Self {
                yurutucu,
                defter,
                kaynaklar,
                kimlik: Arc::new(McpAracKimligi::yeni(
                    "alan-1".to_owned(),
                    PathBuf::from(gecici.path()),
                    McpKartEtiketi::yeni("fake-sunucu").unwrap(),
                    McpKartEtiketi::yeni("fake-arac").unwrap(),
                )),
                hazirlik: hazirlik(),
            }
        }

        async fn baslat(
            &self,
            cagri: CagriId,
            iptal: CancellationToken,
        ) -> oneshot::Receiver<Result<AracCiktisi, AracHatasi>> {
            let kilit = self.kaynaklar.sahipli_al(&[], &[], true).await;
            Arc::clone(&self.yurutucu)
                .cagriyi_baslat(McpAracCagriIstegi {
                    cagri,
                    kimlik: Arc::clone(&self.kimlik),
                    hazirlik: self.hazirlik.clone(),
                    argumanlar: serde_json::Map::new(),
                    iptal,
                    kilit: Some(kilit),
                })
                .unwrap()
        }
    }

    fn hazirlik() -> McpAracHazirligi {
        let politika = McpPolitikaOzeti::ozetten([2; 32]);
        let koruma = McpKorumaOzeti::ozetten([3; 32]);
        let yalitim = McpYalitimProfiliOzeti::ozetten([4; 32]);
        McpAracHazirligi {
            sunucu: McpKartEtiketi::yeni("fake-sunucu").unwrap(),
            arac: McpKartEtiketi::yeni("fake-arac").unwrap(),
            kip: Kip::Siki,
            erisim: McpErisim::Yalitimli,
            ag: McpAgKarti::Kapali,
            ilk_cagri: false,
            tarif: McpTarifi::yeni(
                McpTanimOzeti::ozetten([1; 32]),
                politika,
                koruma,
                yalitim,
                McpBaslatmaSayaci::yeni(1),
            ),
            arac_tanimi: McpAracTanimOzeti::ozetten([5; 32]),
        }
    }

    async fn ikinci_yazma_bekliyor(kaynaklar: &Arc<KaynakKilitleri>) -> bool {
        tokio::time::timeout(
            Duration::from_millis(30),
            kaynaklar.sahipli_al(&[], &[], true),
        )
        .await
        .is_err()
    }

    async fn cleanup_basladi(arka_uc: &SahteArkaUc) {
        tokio::time::timeout(Duration::from_secs(2), arka_uc.cleanup_basladi.notified())
            .await
            .unwrap();
    }

    async fn kayit_silindi(defter: &McpCagriDefteri) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while defter.etkin() != 0 {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn rpc_oncesi_hazirlik_hatasi_normal_kancada_dogrulanmis_cleanup_yapar() {
        let gecici = tempfile::tempdir().unwrap();
        let arka_uc = SahteArkaUc::yeni();
        let kurulum = Kurulum::yeni(&gecici, arka_uc.clone());
        let cagri = CagriId::yeni();
        let kilit = kurulum.kaynaklar.sahipli_al(&[], &[], true).await;
        kurulum
            .yurutucu
            .kilidi_devral(McpAracCagriIstegi {
                cagri,
                kimlik: kurulum.kimlik.clone(),
                hazirlik: kurulum.hazirlik.clone(),
                argumanlar: serde_json::Map::new(),
                iptal: CancellationToken::new(),
                kilit: Some(kilit),
            })
            .unwrap();
        assert_eq!(arka_uc.rpc_sayisi.load(Ordering::SeqCst), 0);
        assert!(
            kurulum.yurutucu.sonuc_isleme_bitti(cagri).await.is_ok(),
            "RPC öncesi hata kalan reservation'ı doğrulanmış kapanışla temizlemeli"
        );
        assert_eq!(arka_uc.cleanup_sayisi.load(Ordering::SeqCst), 1);
        assert!(kurulum.defter.kaydi_bul(cagri).is_none());
        let _ikinci = tokio::time::timeout(
            Duration::from_secs(1),
            kurulum.kaynaklar.sahipli_al(&[], &[], true),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn acik_iptal_cleanup_tamamlanana_kadar_guard_tutar() {
        let gecici = tempfile::tempdir().unwrap();
        let arka_uc = SahteArkaUc::yeni();
        arka_uc.rpc_beklet.store(true, Ordering::SeqCst);
        arka_uc.cleanup_beklet.store(true, Ordering::SeqCst);
        let kurulum = Kurulum::yeni(&gecici, Arc::clone(&arka_uc));
        let cagri = CagriId::yeni();
        let iptal = CancellationToken::new();
        let mut yanit = kurulum.baslat(cagri, iptal.clone()).await;
        tokio::time::timeout(Duration::from_secs(2), arka_uc.rpc_basladi.notified())
            .await
            .unwrap();

        iptal.cancel();
        cleanup_basladi(&arka_uc).await;
        assert!(kurulum.defter.kaydi_bul(cagri).unwrap().kilit_tutuluyor());
        assert!(ikinci_yazma_bekliyor(&kurulum.kaynaklar).await);
        assert!(
            tokio::time::timeout(Duration::from_millis(30), &mut yanit)
                .await
                .is_err()
        );

        arka_uc.cleanup_devam.notify_one();
        assert!(matches!(yanit.await.unwrap(), Err(AracHatasi::Iptal)));
        kayit_silindi(&kurulum.defter).await;
        let _yeni_kilit = tokio::time::timeout(
            Duration::from_secs(1),
            kurulum.kaynaklar.sahipli_al(&[], &[], true),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn hazirlik_rezervasyonu_rpc_baslamadan_cleanup_boyunca_kilidi_tutar() {
        let gecici = tempfile::tempdir().unwrap();
        let arka_uc = SahteArkaUc::yeni();
        arka_uc.cleanup_beklet.store(true, Ordering::SeqCst);
        let kurulum = Kurulum::yeni(&gecici, arka_uc.clone());
        let cagri = CagriId::yeni();
        let kilit = kurulum.kaynaklar.sahipli_al(&[], &[], true).await;
        kurulum
            .yurutucu
            .kilidi_devral(McpAracCagriIstegi {
                cagri,
                kimlik: kurulum.kimlik.clone(),
                hazirlik: kurulum.hazirlik.clone(),
                argumanlar: serde_json::Map::new(),
                iptal: CancellationToken::new(),
                kilit: Some(kilit),
            })
            .unwrap();
        assert_eq!(arka_uc.rpc_sayisi.load(Ordering::SeqCst), 0);
        assert!(ikinci_yazma_bekliyor(&kurulum.kaynaklar).await);
        kurulum.yurutucu.sonuc_isleme_iptal(cagri);
        cleanup_basladi(&arka_uc).await;
        assert!(ikinci_yazma_bekliyor(&kurulum.kaynaklar).await);
        arka_uc.cleanup_devam.notify_one();
        kayit_silindi(&kurulum.defter).await;
    }

    #[tokio::test]
    async fn kayittan_onceki_kapanis_kaniti_yeni_kusaga_tasinmaz() {
        let gecici = tempfile::tempdir().unwrap();
        let kurulum = Kurulum::yeni(&gecici, SahteArkaUc::yeni());
        let eski = kurulum.hazirlik.tarif.baslatma();
        kurulum.defter.surec_kapanisini_bildir(
            "alan-1",
            "fake-sunucu",
            eski,
            McpCagriKapanisSonucu {
                surec_kapandi: true,
                son_denetim_basarili: true,
            },
        );
        let cagri = CagriId::yeni();
        let kilit = kurulum.kaynaklar.sahipli_al(&[], &[], true).await;
        kurulum
            .yurutucu
            .kilidi_devral(McpAracCagriIstegi {
                cagri,
                kimlik: kurulum.kimlik.clone(),
                hazirlik: kurulum.hazirlik.clone(),
                argumanlar: serde_json::Map::new(),
                iptal: CancellationToken::new(),
                kilit: Some(kilit),
            })
            .unwrap();
        let kayit = kurulum.defter.kaydi_bul(cagri).unwrap();
        assert!(kurulum.defter.kapanis_dogrulandi(&kayit));
        assert!(
            kayit.kapanis_sonucu.borrow().is_none(),
            "kanıt eski watch bildirimine bağlı değildir"
        );
        assert!(
            !kurulum
                .defter
                .son_dogrulanmis_kapanis
                .lock()
                .unwrap()
                .get(&("alan-1".into(), "fake-sunucu".into()))
                .is_some_and(|n| *n >= eski.ileri())
        );
        kurulum
            .yurutucu
            .sonuc_isleme_iptal_ve_bekle(cagri)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn yanit_alicisi_drop_edilince_bagimsiz_cleanup_guardi_tutar() {
        let gecici = tempfile::tempdir().unwrap();
        let arka_uc = SahteArkaUc::yeni();
        arka_uc.rpc_beklet.store(true, Ordering::SeqCst);
        arka_uc.cleanup_beklet.store(true, Ordering::SeqCst);
        let kurulum = Kurulum::yeni(&gecici, Arc::clone(&arka_uc));
        let cagri = CagriId::yeni();
        let yanit = kurulum.baslat(cagri, CancellationToken::new()).await;
        tokio::time::timeout(Duration::from_secs(2), arka_uc.rpc_basladi.notified())
            .await
            .unwrap();

        drop(yanit);
        cleanup_basladi(&arka_uc).await;
        assert!(kurulum.defter.kaydi_bul(cagri).unwrap().kilit_tutuluyor());
        assert!(ikinci_yazma_bekliyor(&kurulum.kaynaklar).await);

        arka_uc.cleanup_devam.notify_one();
        kayit_silindi(&kurulum.defter).await;
        let _yeni_kilit = tokio::time::timeout(
            Duration::from_secs(1),
            kurulum.kaynaklar.sahipli_al(&[], &[], true),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn deadline_rpc_futureini_kapatip_cleanup_beklerken_guardi_tutar() {
        let gecici = tempfile::tempdir().unwrap();
        let arka_uc = SahteArkaUc::yeni();
        arka_uc.zaman_asimi_ms.store(40, Ordering::SeqCst);
        arka_uc.rpc_beklet.store(true, Ordering::SeqCst);
        arka_uc.cleanup_beklet.store(true, Ordering::SeqCst);
        let kurulum = Kurulum::yeni(&gecici, Arc::clone(&arka_uc));
        let cagri = CagriId::yeni();
        let mut yanit = kurulum.baslat(cagri, CancellationToken::new()).await;
        tokio::time::timeout(Duration::from_secs(2), arka_uc.rpc_basladi.notified())
            .await
            .unwrap();
        cleanup_basladi(&arka_uc).await;
        assert!(kurulum.defter.kaydi_bul(cagri).unwrap().kilit_tutuluyor());
        assert!(ikinci_yazma_bekliyor(&kurulum.kaynaklar).await);
        assert!(
            tokio::time::timeout(Duration::from_millis(30), &mut yanit)
                .await
                .is_err()
        );

        arka_uc.cleanup_devam.notify_one();
        let sonuc = tokio::time::timeout(Duration::from_secs(2), yanit)
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(sonuc, Err(AracHatasi::Gecersiz(_))));
        kayit_silindi(&kurulum.defter).await;
    }

    #[tokio::test]
    async fn normal_sonuc_core_kancasina_kadar_guard_tutar() {
        let gecici = tempfile::tempdir().unwrap();
        let arka_uc = SahteArkaUc::yeni();
        arka_uc.son_denetim_beklet.store(true, Ordering::SeqCst);
        let kurulum = Kurulum::yeni(&gecici, Arc::clone(&arka_uc));
        let cagri = CagriId::yeni();
        let sonuc = kurulum
            .baslat(cagri, CancellationToken::new())
            .await
            .await
            .unwrap()
            .unwrap();
        assert_eq!(sonuc.icerik, "önceden maskelenmiş test sonucu");
        let kayit = kurulum.defter.kaydi_bul(cagri).unwrap();
        assert_eq!(kayit.durum(), McpCagriDurumu::DenetimBekliyor);
        assert!(kayit.kilit_tutuluyor());
        assert_eq!(kurulum.defter.etkin(), 1);

        let hook_yurutucu = Arc::clone(&kurulum.yurutucu);
        let hook = tokio::spawn(async move { hook_yurutucu.sonuc_isleme_bitti(cagri).await });
        tokio::time::timeout(
            Duration::from_secs(2),
            arka_uc.son_denetim_basladi.notified(),
        )
        .await
        .unwrap();
        assert_eq!(kayit.durum(), McpCagriDurumu::SonDenetimSuruyor);
        assert!(kayit.kilit_tutuluyor());
        assert!(ikinci_yazma_bekliyor(&kurulum.kaynaklar).await);

        arka_uc.son_denetim_devam.notify_one();
        hook.await.unwrap().unwrap();
        kayit_silindi(&kurulum.defter).await;
        let _yeni_kilit = tokio::time::timeout(
            Duration::from_secs(1),
            kurulum.kaynaklar.sahipli_al(&[], &[], true),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn cleanup_bekleyicisi_drop_edilse_de_bagimsiz_gorev_guardi_cozer() {
        let gecici = tempfile::tempdir().unwrap();
        let arka_uc = SahteArkaUc::yeni();
        arka_uc.cleanup_beklet.store(true, Ordering::SeqCst);
        let kurulum = Kurulum::yeni(&gecici, Arc::clone(&arka_uc));
        let cagri = CagriId::yeni();
        kurulum
            .baslat(cagri, CancellationToken::new())
            .await
            .await
            .unwrap()
            .unwrap();

        let hook_yurutucu = Arc::clone(&kurulum.yurutucu);
        let bekleyici =
            tokio::spawn(async move { hook_yurutucu.sonuc_isleme_iptal_ve_bekle(cagri).await });
        cleanup_basladi(&arka_uc).await;
        assert!(kurulum.defter.kaydi_bul(cagri).unwrap().kilit_tutuluyor());
        bekleyici.abort();
        assert!(ikinci_yazma_bekliyor(&kurulum.kaynaklar).await);

        arka_uc.cleanup_devam.notify_one();
        kayit_silindi(&kurulum.defter).await;
        let _yeni_kilit = tokio::time::timeout(
            Duration::from_secs(1),
            kurulum.kaynaklar.sahipli_al(&[], &[], true),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn kapatma_hatasi_kaydi_ve_guardi_engelli_tutar() {
        let gecici = tempfile::tempdir().unwrap();
        let arka_uc = SahteArkaUc::yeni();
        arka_uc.rpc_beklet.store(true, Ordering::SeqCst);
        arka_uc.kapatma_hatasi.store(true, Ordering::SeqCst);
        let kurulum = Kurulum::yeni(&gecici, Arc::clone(&arka_uc));
        let cagri = CagriId::yeni();
        let iptal = CancellationToken::new();
        let mut yanit = kurulum.baslat(cagri, iptal.clone()).await;
        tokio::time::timeout(Duration::from_secs(2), arka_uc.rpc_basladi.notified())
            .await
            .unwrap();

        iptal.cancel();
        let sonuc = tokio::time::timeout(Duration::from_secs(2), &mut yanit)
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(sonuc, Err(AracHatasi::Iptal)));
        let kayit = kurulum.defter.kaydi_bul(cagri).unwrap();
        assert_eq!(kayit.durum(), McpCagriDurumu::KapatmaHatasi);
        assert!(kayit.kilit_tutuluyor());
        assert_eq!(kurulum.defter.etkin(), 1);
        assert!(ikinci_yazma_bekliyor(&kurulum.kaynaklar).await);
    }

    #[tokio::test]
    async fn kapanis_son_denetim_hatasi_kaydi_ve_guardi_engelli_tutar() {
        let gecici = tempfile::tempdir().unwrap();
        let arka_uc = SahteArkaUc::yeni();
        arka_uc.rpc_beklet.store(true, Ordering::SeqCst);
        arka_uc.cleanup_denetim_hatasi.store(true, Ordering::SeqCst);
        let kurulum = Kurulum::yeni(&gecici, Arc::clone(&arka_uc));
        let cagri = CagriId::yeni();
        let iptal = CancellationToken::new();
        let mut yanit = kurulum.baslat(cagri, iptal.clone()).await;
        tokio::time::timeout(Duration::from_secs(2), arka_uc.rpc_basladi.notified())
            .await
            .unwrap();

        iptal.cancel();
        let sonuc = tokio::time::timeout(Duration::from_secs(2), &mut yanit)
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(sonuc, Err(AracHatasi::Iptal)));
        let kayit = kurulum.defter.kaydi_bul(cagri).unwrap();
        assert_eq!(kayit.durum(), McpCagriDurumu::SonDenetimHatasi);
        assert!(kayit.kilit_tutuluyor());
        assert_eq!(kurulum.defter.etkin(), 1);
        assert!(ikinci_yazma_bekliyor(&kurulum.kaynaklar).await);
    }

    #[tokio::test]
    async fn son_denetim_hatasi_kaydi_ve_guardi_engelli_tutar() {
        let gecici = tempfile::tempdir().unwrap();
        let arka_uc = SahteArkaUc::yeni();
        arka_uc.son_denetim_hatasi.store(true, Ordering::SeqCst);
        let kurulum = Kurulum::yeni(&gecici, Arc::clone(&arka_uc));
        let cagri = CagriId::yeni();
        kurulum
            .baslat(cagri, CancellationToken::new())
            .await
            .await
            .unwrap()
            .unwrap();

        assert!(matches!(
            kurulum.yurutucu.sonuc_isleme_bitti(cagri).await,
            Err(AracHatasi::SonucIsleme)
        ));
        let kayit = kurulum.defter.kaydi_bul(cagri).unwrap();
        assert_eq!(kayit.durum(), McpCagriDurumu::SonDenetimHatasi);
        assert!(kayit.kilit_tutuluyor());
        assert_eq!(kurulum.defter.etkin(), 1);
        assert!(ikinci_yazma_bekliyor(&kurulum.kaynaklar).await);
    }

    #[tokio::test]
    async fn r_d_ve_pi_son_kapi_yarisinda_rpc_sayaci_sifir_kalir() {
        for bilesen in [DegisenBilesen::R, DegisenBilesen::D, DegisenBilesen::Pi] {
            let gecici = tempfile::tempdir().unwrap();
            let arka_uc = SahteArkaUc::yeni();
            arka_uc.sonraki_kapida_degistir(bilesen);
            let kurulum = Kurulum::yeni(&gecici, Arc::clone(&arka_uc));
            let cagri = CagriId::yeni();
            let sonuc = kurulum
                .baslat(cagri, CancellationToken::new())
                .await
                .await
                .unwrap();
            assert!(matches!(sonuc, Err(AracHatasi::OnayEskidi)));
            assert_eq!(arka_uc.rpc_sayisi.load(Ordering::SeqCst), 0);
            kayit_silindi(&kurulum.defter).await;
        }
    }
}
