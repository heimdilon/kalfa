//! Uzak MCP araç keşfi (`arac_ara`), çok sunuculu denetlenmiş katalog ve bağlam
//! bütçesi yönetimi (`1.1b` Görev 5).
//!
//! ## Sözleşme Notu (Görev 5)
//! - **Tek araç seçimi (`arac_ara`):** Model bağlamını şişirmemek için uzak MCP
//!   sunucularındaki araç şemaları başlangıçta modele doğrudan gönderilmez.
//!   Bunun yerine tek bir yerleşik keşif aracı (`arac_ara`) sunulur. `arac_ara`,
//!   `sorgu` (ve isteğe bağlı `sunucu`) ile denetlenmiş katalogda arama yapar ve
//!   isteğe bağlı `yukle` dizisiyle seçilen `mcp__<sunucu>__<arac>` tanımlarını
//!   oturumun uzak araç kaynağına (`UzakAracKaynagi`) yükler.
//! - **Bağlam bütçesi sınırları (`UzakBaglamButcesi`):**
//!   - Arama sonucunda en çok `8` aday (`AZAMI_UZAK_ADAY_ARAC_SINIRI = 8`).
//!   - Oturumda aynı anda en çok `16` yüklenmiş uzak araç (`AZAMI_UZAK_ETKIN_ARAC_SINIRI = 16`).
//!   - Yüklenmiş uzak araçların toplam şema boyutu en çok `64 KiB`
//!     (`AZAMI_UZAK_SEMA_BAYT_SINIRI = 65_536`).
//! - **Tur başı anlık görüntü ve sürüm bağlama:**
//!   - `arac_ara` ile yüklenen araçlar `UzakAracKaynagi::hazir_araclar()` yayımını
//!     günceller; mevcut model turunun `AracAnlikGoruntusu` değişmez olduğundan
//!     yeni yüklenen araç **ancak bir sonraki model turunun görüntüsünde** kullanılabilir.
//!   - Yüklü her uzak araç, yüklendiği andaki çalışma tarifine (`McpTarifi` — sunucu
//!     kimliği, normalize origin/uç, `tanim_revizyonu`, `kimlik_revizyonu`, protokol
//!     sürümü) ve araç tanım özetine (`McpAracTanimOzeti` — `arac_revizyonu`,
//!     `inputSchema`, `outputSchema`) bağlıdır. Sunucu/kimlik/şema revizyonu değiştiğinde
//!     eski sürümle hazırlık veya çağrı `AracHatasi::OnayEskidi` döner.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use kalfa_core::arac::{
    Arac, AracCiktisi, AracDenetimIsi, AracDenetimSahibi, AracHatasi, AracKaynagi, AracOrtami,
    Hazirlik, KaynakAraci, McpAgHostu, McpAgKarti, McpAracHazirligi, McpAracTanimOzeti,
    McpBaslatmaSayaci, McpErisim, McpKartEtiketi, McpTarifi, async_trait,
};
use kalfa_core::izin::{GizliAyarlari, Kip};
use kalfa_sandbox::tarama::CancellationToken;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use url::Url;

use crate::arac::{McpAracRpcHataDonusturucu, McpAracSonucDonusturucu, McpAracSunumu};
use crate::istemci::RpcHatasi;
use crate::kesif::arac_tanimlarini_denetle;
use crate::maske::GizliDeger;
use crate::tanim::{UzakBaglamButcesi, UzakMcpTanim};
use crate::tanim_denetimi::DenetlenmisAracTanimi;
use crate::tarif::{uzak_arac_tanim_ozeti, uzak_calisma_tarifi};
use crate::{CallToolResult, Tool};

/// Tekil uzak araç keşif ve yükleme aracının sabit adı.
pub const ARAC_ARA_ADI: &str = "arac_ara";

/// Uzak araç keşfi ve bağlam bütçesi hataları.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UzakKesifHatasi {
    AracBulunamadi(String),
    EtkinAracSiniriAsildi {
        sinir: usize,
    },
    SemaBaytSiniriAsildi {
        talep_edilen_toplam: usize,
        sinir: usize,
    },
    GecersizSema(String),
}

impl std::fmt::Display for UzakKesifHatasi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AracBulunamadi(ad) => {
                write!(f, "uzak araç denetlenmiş katalogda bulunamadı: {ad}")
            }
            Self::EtkinAracSiniriAsildi { sinir } => {
                write!(
                    f,
                    "oturumda en çok {sinir} uzak araç aynı anda yüklenebilir"
                )
            }
            Self::SemaBaytSiniriAsildi {
                talep_edilen_toplam,
                sinir,
            } => write!(
                f,
                "uzak araç şema bütçesi aşıldı (talep edilen: {talep_edilen_toplam} bayt, sınır: {sinir} bayt)"
            ),
            Self::GecersizSema(detay) => write!(f, "uzak araç şeması derlenemedi: {detay}"),
        }
    }
}

impl std::error::Error for UzakKesifHatasi {}

fn ozet_hex(baytlar: &[u8; 32]) -> String {
    const TABLO: &[u8; 16] = b"0123456789abcdef";
    let mut cikti = String::with_capacity(64);
    for &b in baytlar {
        cikti.push(char::from(TABLO[usize::from(b >> 4)]));
        cikti.push(char::from(TABLO[usize::from(b & 0x0f)]));
    }
    cikti
}

/// Denetlenmiş katalogdaki tek bir uzak aracın arama adayı özeti.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UzakAracAdayi {
    pub birlesik_ad: String,
    pub sunucu: String,
    pub uzak_ad: String,
    pub aciklama: String,
    pub sema_bayt: usize,
    pub calisma_tarifi_ozeti: String,
    pub arac_tanimi_ozeti: String,
    pub tanim_revizyonu: u64,
    pub kimlik_revizyonu: u64,
    pub arac_revizyonu: u64,
}

/// Oturuma yüklenen bir uzak aracın sürüm ve bütçe özeti.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UzakYuklemeOzeti {
    pub birlesik_ad: String,
    pub sunucu: String,
    pub sema_bayt: usize,
    pub calisma_tarifi_ozeti: String,
    pub arac_tanimi_ozeti: String,
    pub sonraki_turda_etkin: bool,
}

/// `arac_ara` aracının JSON çıktısı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AracAraSonucu {
    pub adaylar: Vec<UzakAracAdayi>,
    pub yuklenenler: Vec<UzakYuklemeOzeti>,
    pub etkin_uzak_arac_sayisi: usize,
    pub kullanilan_sema_bayt: usize,
    pub azami_sema_bayt: usize,
}

/// Uzak MCP sunucusunun katalog kaydı için girdi.
pub struct UzakSunucuKatalogGirdisi {
    pub tanim: UzakMcpTanim,
    pub alan_kimligi: String,
    pub gizli_ayarlari: GizliAyarlari,
    pub tanim_revizyonu: u64,
    pub kimlik_revizyonu: u64,
    pub arac_revizyonu: u64,
    pub protokol_surumu: String,
    pub baglanti_sayaci: McpBaslatmaSayaci,
    pub ham_araclar: Vec<Tool>,
    pub sirlar: Vec<GizliDeger>,
}

#[derive(Clone)]
pub(crate) struct KatalogAracKaydi {
    pub sunucu_adi: String,
    pub origin_host: McpAgHostu,
    pub tarif: McpTarifi,
    pub uzak_d: McpAracTanimOzeti,
    pub tanim_revizyonu: u64,
    pub kimlik_revizyonu: u64,
    pub arac_revizyonu: u64,
    pub sema_bayt: usize,
    pub denetlenmis: DenetlenmisAracTanimi,
}

struct KatalogSunucuTablosu {
    tarif: McpTarifi,
    araclar: BTreeMap<String, KatalogAracKaydi>,
    reddedilen_siralar: Vec<usize>,
}

/// Uzak MCP aracının çağrı yürütücüsü.
///
/// Çağrı sırasında onaylanan `(McpTarifi, McpAracTanimOzeti)` ile güncel kataloğun
/// eşleştiği doğrulanır; ham `CallToolResult` veya `RpcHatasi` ise G7'nin
/// sır maskeli ve `outputSchema` doğrulamalı dönüştürücüsünden geçirilir.
#[async_trait]
pub trait UzakMcpCagriYurutucusu: Send + Sync {
    async fn uzak_arac_cagir(
        &self,
        sunucu_adi: &str,
        uzak_arac_adi: &str,
        argumanlar: Map<String, Value>,
        onayli_tarif: McpTarifi,
        onayli_d: McpAracTanimOzeti,
        iptal: CancellationToken,
    ) -> Result<(CallToolResult, bool), UzakCagriHatasi>;
}

/// Uzak çağrı yürütücüsünün dönebileceği hata türleri.
#[derive(Debug, Clone)]
pub enum UzakCagriHatasi {
    OnayEskidi,
    Rpc(RpcHatasi),
    Iletisim(String),
    SonucBelirsiz(String),
}

/// Çok sunuculu, sır ve şema denetiminden geçmiş uzak MCP araç kataloğu.
pub struct UzakAracKatalogu {
    butce: UzakBaglamButcesi,
    sunucular: Mutex<BTreeMap<String, KatalogSunucuTablosu>>,
}

impl Default for UzakAracKatalogu {
    fn default() -> Self {
        Self::yeni(UzakBaglamButcesi::default())
    }
}

impl UzakAracKatalogu {
    pub fn yeni(butce: UzakBaglamButcesi) -> Self {
        Self {
            butce,
            sunucular: Mutex::new(BTreeMap::new()),
        }
    }

    pub const fn butce(&self) -> UzakBaglamButcesi {
        self.butce
    }

    /// Bir uzak sunucunun ham araç listesini G7 sır ve şema kapılarından geçirerek
    /// kataloğa kaydeder. Kapıdan geçmeyen araçlar yalnız 1-tabanlı sıra numarasıyla
    /// döner ve kataloğa girmez.
    pub fn sunucu_katalogunu_guncelle(&self, girdi: UzakSunucuKatalogGirdisi) -> Vec<usize> {
        let sunucu_adi = girdi.tanim.ad().to_owned();
        let origin = girdi.tanim.origin();
        let origin_host = origin_host_cikar(&origin).unwrap_or_else(|| {
            McpAgHostu::yeni("uzak.invalid").expect("sabit yedek host geçerlidir")
        });
        let tarif = uzak_calisma_tarifi(
            &girdi.tanim,
            &girdi.alan_kimligi,
            &girdi.gizli_ayarlari,
            girdi.tanim_revizyonu,
            girdi.kimlik_revizyonu,
            &girdi.protokol_surumu,
            girdi.baglanti_sayaci,
        );

        let kesif = arac_tanimlarini_denetle(&sunucu_adi, girdi.ham_araclar, &girdi.sirlar);
        let (denetlenmisler, reddedilen_siralar) = kesif.parcalarina_ayir();

        let mut araclar = BTreeMap::new();
        for denetlenmis in denetlenmisler {
            let sunum = denetlenmis.sunum();
            let uzak_d = uzak_arac_tanim_ozeti(
                &sunucu_adi,
                &origin,
                girdi.arac_revizyonu,
                &sunum.ad,
                &sunum.aciklama,
                &sunum.parametre_semasi,
                denetlenmis.cikti_semasi_degeri(),
            );
            let sema_bayt = uzak_arac_sema_bayt_hesapla(sunum);
            araclar.insert(
                sunum.ad.clone(),
                KatalogAracKaydi {
                    sunucu_adi: sunucu_adi.clone(),
                    origin_host: origin_host.clone(),
                    tarif,
                    uzak_d,
                    tanim_revizyonu: girdi.tanim_revizyonu,
                    kimlik_revizyonu: girdi.kimlik_revizyonu,
                    arac_revizyonu: girdi.arac_revizyonu,
                    sema_bayt,
                    denetlenmis,
                },
            );
        }

        let mut sunucular = self.sunucular.lock().unwrap_or_else(|e| e.into_inner());
        sunucular.insert(
            sunucu_adi,
            KatalogSunucuTablosu {
                tarif,
                araclar,
                reddedilen_siralar: reddedilen_siralar.clone(),
            },
        );
        reddedilen_siralar
    }

    /// Sunucuyu katalogdan kaldırır; bu sunucuya ait önceden yüklenmiş araçlar
    /// bir sonraki hazırlık veya çağrıda `AracHatasi::OnayEskidi` ile reddedilir.
    pub fn sunucu_kaldir(&self, sunucu_adi: &str) {
        let mut sunucular = self.sunucular.lock().unwrap_or_else(|e| e.into_inner());
        sunucular.remove(sunucu_adi);
    }

    /// Sunucuda reddedilen araçların 1-tabanlı sıra numaralarını döner.
    pub fn reddedilen_siralar(&self, sunucu_adi: &str) -> Vec<usize> {
        let sunucular = self.sunucular.lock().unwrap_or_else(|e| e.into_inner());
        sunucular
            .get(sunucu_adi)
            .map(|t| t.reddedilen_siralar.clone())
            .unwrap_or_default()
    }

    /// Sunucunun güncel çalışma tarifini (`McpTarifi`) döner.
    pub fn sunucu_tarifi(&self, sunucu_adi: &str) -> Option<McpTarifi> {
        let sunucular = self.sunucular.lock().unwrap_or_else(|e| e.into_inner());
        sunucular.get(sunucu_adi).map(|t| t.tarif)
    }

    pub(crate) fn kayit_bul(&self, birlesik_ad: &str) -> Option<KatalogAracKaydi> {
        let sunucular = self.sunucular.lock().unwrap_or_else(|e| e.into_inner());
        sunucular
            .values()
            .find_map(|tablo| tablo.araclar.get(birlesik_ad).cloned())
    }

    /// Denetlenmiş katalogda `sorgu` ve isteğe bağlı `sunucu_filtresi` ile arama
    /// yapar; sonuçları alaka sırasına göre sıralar ve en çok `aday_arac_siniri`
    /// (`8`) aday döner.
    pub fn ara(&self, sorgu: &str, sunucu_filtresi: Option<&str>) -> Vec<UzakAracAdayi> {
        let temiz_sorgu = sorgu.trim().to_lowercase();
        let parcalar: Vec<&str> = temiz_sorgu.split_whitespace().collect();
        let filtre = sunucu_filtresi.map(str::trim).filter(|s| !s.is_empty());

        let sunucular = self.sunucular.lock().unwrap_or_else(|e| e.into_inner());
        let mut puanli: Vec<(u8, String, UzakAracAdayi)> = Vec::new();

        for (sunucu_adi, tablo) in sunucular.iter() {
            if let Some(beklenen_sunucu) = filtre
                && sunucu_adi != beklenen_sunucu
            {
                continue;
            }
            for kayit in tablo.araclar.values() {
                let birlesik = &kayit.denetlenmis.sunum().ad;
                let uzak_ad = kayit.denetlenmis.arac_etiketi().gorunen();
                let aciklama = &kayit.denetlenmis.sunum().aciklama;

                let Some(puan) = aday_eslesme_puani(
                    &temiz_sorgu,
                    &parcalar,
                    sunucu_adi,
                    birlesik,
                    uzak_ad,
                    aciklama,
                ) else {
                    continue;
                };

                let aday = UzakAracAdayi {
                    birlesik_ad: birlesik.clone(),
                    sunucu: sunucu_adi.clone(),
                    uzak_ad: uzak_ad.to_owned(),
                    aciklama: aciklama.clone(),
                    sema_bayt: kayit.sema_bayt,
                    calisma_tarifi_ozeti: ozet_hex(&kayit.tarif.calisma().sha256()),
                    arac_tanimi_ozeti: ozet_hex(&kayit.uzak_d.sha256()),
                    tanim_revizyonu: kayit.tanim_revizyonu,
                    kimlik_revizyonu: kayit.kimlik_revizyonu,
                    arac_revizyonu: kayit.arac_revizyonu,
                };
                puanli.push((puan, birlesik.clone(), aday));
            }
        }

        puanli.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        puanli
            .into_iter()
            .take(self.butce.aday_arac_siniri())
            .map(|(_, _, aday)| aday)
            .collect()
    }
}

fn aday_eslesme_puani(
    tam_sorgu: &str,
    parcalar: &[&str],
    sunucu_adi: &str,
    birlesik_ad: &str,
    uzak_ad: &str,
    aciklama: &str,
) -> Option<u8> {
    if tam_sorgu.is_empty() {
        return Some(5);
    }
    let birlesik_kucuk = birlesik_ad.to_lowercase();
    let uzak_kucuk = uzak_ad.to_lowercase();
    let sunucu_kucuk = sunucu_adi.to_lowercase();
    let aciklama_kucuk = aciklama.to_lowercase();

    if birlesik_kucuk == tam_sorgu || uzak_kucuk == tam_sorgu {
        return Some(0);
    }
    if uzak_kucuk.starts_with(tam_sorgu) || birlesik_kucuk.starts_with(tam_sorgu) {
        return Some(1);
    }
    if uzak_kucuk.contains(tam_sorgu) || birlesik_kucuk.contains(tam_sorgu) {
        return Some(2);
    }
    if aciklama_kucuk.contains(tam_sorgu) || sunucu_kucuk.contains(tam_sorgu) {
        return Some(3);
    }
    let birlesik_metin = format!("{sunucu_kucuk} {birlesik_kucuk} {uzak_kucuk} {aciklama_kucuk}");
    if !parcalar.is_empty() && parcalar.iter().all(|p| birlesik_metin.contains(p)) {
        return Some(4);
    }
    None
}

fn origin_host_cikar(origin: &str) -> Option<McpAgHostu> {
    let url = Url::parse(origin).ok()?;
    let host = url.host_str()?;
    let gorunen = match url.port() {
        Some(port) => {
            if host.contains(':') && !host.starts_with('[') {
                format!("[{host}]:{port}")
            } else {
                format!("{host}:{port}")
            }
        }
        None => {
            if host.contains(':') && !host.starts_with('[') {
                format!("[{host}]")
            } else {
                host.to_owned()
            }
        }
    };
    McpAgHostu::yeni(&gorunen).ok()
}

/// Bir araç sunumunun bağlam bütçesinde kapladığı UTF-8 şema bayt boyutunu hesaplar.
pub fn uzak_arac_sema_bayt_hesapla(sunum: &McpAracSunumu) -> usize {
    let deger = json!({
        "name": &sunum.ad,
        "description": &sunum.aciklama,
        "parameters": &sunum.parametre_semasi,
    });
    serde_json::to_vec(&deger)
        .map(|v| v.len())
        .unwrap_or_else(|_| sunum.ad.len() + sunum.aciklama.len() + 64)
}

#[derive(Clone)]
struct YukluUzakArac {
    yuklenen_tarif: McpTarifi,
    yuklenen_d: McpAracTanimOzeti,
    sema_bayt: usize,
    kaynak_araci: KaynakAraci,
}

#[derive(Default)]
struct SecimDurumu {
    yuklenenler: BTreeMap<String, YukluUzakArac>,
    hazir: Option<Arc<[KaynakAraci]>>,
    kullanilan_sema_bayt: usize,
    ilk_cagri_yapilan_sunucular: BTreeSet<String>,
}

/// Oturum başına seçilmiş uzak MCP araçlarını bağlam bütçesi (`<= 16` araç,
/// `<= 64 KiB` şema) içinde tutan ve `AracKaynagi` üzerinden bir sonraki
/// model turunun görüntüsüne yayımlayan bileşen.
pub struct UzakAracKaynagi {
    katalog: Arc<UzakAracKatalogu>,
    yurutucu: Arc<dyn UzakMcpCagriYurutucusu>,
    oturum_kipi: Mutex<Kip>,
    durum: Mutex<SecimDurumu>,
}

impl UzakAracKaynagi {
    pub fn yeni(
        katalog: Arc<UzakAracKatalogu>,
        yurutucu: Arc<dyn UzakMcpCagriYurutucusu>,
    ) -> Arc<Self> {
        Arc::new(Self {
            katalog,
            yurutucu,
            oturum_kipi: Mutex::new(Kip::Proje),
            durum: Mutex::new(SecimDurumu::default()),
        })
    }

    pub fn oturum_kipini_ayarla(&self, kip: Kip) {
        *self.oturum_kipi.lock().unwrap_or_else(|e| e.into_inner()) = kip;
    }

    pub fn etkin_arac_sayisi(&self) -> usize {
        self.durum
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .yuklenenler
            .len()
    }

    pub fn kullanilan_sema_bayt(&self) -> usize {
        self.durum
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .kullanilan_sema_bayt
    }

    /// Seçilen birleşik araç adlarını (`mcp__<sunucu>__<arac>`) bütçe sınırlarını
    /// (`<= 16` araç ve `<= 64 KiB` toplam şema baytı) doğrulayarak oturuma yükler.
    ///
    /// Herhangi bir araç bulunamazsa veya bütçeyi aşarsa atomik olarak hata döner
    /// ve mevcut yüklü araç kümesi değişmez.
    pub fn araclari_yukle(
        self: &Arc<Self>,
        birlesik_adlar: &[String],
    ) -> Result<Vec<UzakYuklemeOzeti>, UzakKesifHatasi> {
        let butce = self.katalog.butce();
        let mut d = self.durum.lock().unwrap_or_else(|e| e.into_inner());

        let mut aday_harita = d.yuklenenler.clone();
        let mut aday_toplam_bayt = d.kullanilan_sema_bayt;
        let mut ozetler = Vec::with_capacity(birlesik_adlar.len());

        for ad in birlesik_adlar {
            let kayit = self
                .katalog
                .kayit_bul(ad)
                .ok_or_else(|| UzakKesifHatasi::AracBulunamadi(ad.clone()))?;

            let eski_bayt = aday_harita.get(ad).map(|y| y.sema_bayt).unwrap_or(0);
            let yeni_toplam_bayt = aday_toplam_bayt
                .saturating_sub(eski_bayt)
                .saturating_add(kayit.sema_bayt);

            if yeni_toplam_bayt > butce.sema_bayt_siniri() {
                return Err(UzakKesifHatasi::SemaBaytSiniriAsildi {
                    talep_edilen_toplam: yeni_toplam_bayt,
                    sinir: butce.sema_bayt_siniri(),
                });
            }

            let yeni_sayi = if aday_harita.contains_key(ad) {
                aday_harita.len()
            } else {
                aday_harita.len() + 1
            };
            if yeni_sayi > butce.etkin_arac_siniri() {
                return Err(UzakKesifHatasi::EtkinAracSiniriAsildi {
                    sinir: butce.etkin_arac_siniri(),
                });
            }

            let uzak_arac = Arc::new(UzakMcpArac {
                sunucu_adi: kayit.sunucu_adi.clone(),
                origin_host: kayit.origin_host.clone(),
                sunucu_etiketi: kayit.denetlenmis.sunucu_etiketi().clone(),
                arac_etiketi: kayit.denetlenmis.arac_etiketi().clone(),
                sunum: kayit.denetlenmis.sunum_kopyasi(),
                yuklenen_tarif: kayit.tarif,
                yuklenen_d: kayit.uzak_d,
                sonuc_donusturucu: kayit.denetlenmis.sonuc_donusturucusu(),
                rpc_hata_donusturucu: kayit.denetlenmis.rpc_hata_donusturucusu(),
                katalog: Arc::clone(&self.katalog),
                kaynak: Arc::clone(self),
                yurutucu: Arc::clone(&self.yurutucu),
            });
            let kaynak_araci = KaynakAraci::yeni(uzak_arac)
                .map_err(|e| UzakKesifHatasi::GecersizSema(e.to_string()))?;

            aday_harita.insert(
                ad.clone(),
                YukluUzakArac {
                    yuklenen_tarif: kayit.tarif,
                    yuklenen_d: kayit.uzak_d,
                    sema_bayt: kayit.sema_bayt,
                    kaynak_araci,
                },
            );
            aday_toplam_bayt = yeni_toplam_bayt;

            ozetler.push(UzakYuklemeOzeti {
                birlesik_ad: ad.clone(),
                sunucu: kayit.sunucu_adi,
                sema_bayt: kayit.sema_bayt,
                calisma_tarifi_ozeti: ozet_hex(&kayit.tarif.calisma().sha256()),
                arac_tanimi_ozeti: ozet_hex(&kayit.uzak_d.sha256()),
                sonraki_turda_etkin: true,
            });
        }

        let hazir: Vec<KaynakAraci> = aday_harita
            .values()
            .map(|y| y.kaynak_araci.clone())
            .collect();
        d.yuklenenler = aday_harita;
        d.kullanilan_sema_bayt = aday_toplam_bayt;
        d.hazir = Some(hazir.into());

        Ok(ozetler)
    }

    /// Belirtilen uzak aracı oturumun etkin kümesinden çıkarır ve şema bütçesini boşaltır.
    pub fn arac_kaldir(&self, birlesik_ad: &str) -> bool {
        let mut d = self.durum.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cikarilan) = d.yuklenenler.remove(birlesik_ad) {
            d.kullanilan_sema_bayt = d.kullanilan_sema_bayt.saturating_sub(cikarilan.sema_bayt);
            let hazir: Vec<KaynakAraci> = d
                .yuklenenler
                .values()
                .map(|y| y.kaynak_araci.clone())
                .collect();
            d.hazir = Some(hazir.into());
            true
        } else {
            false
        }
    }

    fn ilk_cagri_mi(&self, sunucu_adi: &str) -> bool {
        !self
            .durum
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .ilk_cagri_yapilan_sunucular
            .contains(sunucu_adi)
    }

    fn ilk_cagri_yapildi_isaretle(&self, sunucu_adi: &str) {
        self.durum
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .ilk_cagri_yapilan_sunucular
            .insert(sunucu_adi.to_owned());
    }
}

impl AracKaynagi for UzakAracKaynagi {
    fn hazir_araclar(&self) -> Arc<[KaynakAraci]> {
        let mut d = self.durum.lock().unwrap_or_else(|e| e.into_inner());
        let onceki_uzunluk = d.yuklenenler.len();
        d.yuklenenler.retain(|ad, yuklu| {
            self.katalog.kayit_bul(ad).is_some_and(|guncel| {
                guncel.tarif == yuklu.yuklenen_tarif && guncel.uzak_d == yuklu.yuklenen_d
            })
        });
        if d.yuklenenler.len() != onceki_uzunluk {
            d.kullanilan_sema_bayt = d.yuklenenler.values().map(|y| y.sema_bayt).sum();
            let hazir: Vec<KaynakAraci> = d
                .yuklenenler
                .values()
                .map(|y| y.kaynak_araci.clone())
                .collect();
            d.hazir = Some(hazir.into());
        }
        d.hazir.clone().unwrap_or_else(|| Arc::from([]))
    }

    fn denetim_islerini_al(&self, _sahip: AracDenetimSahibi) -> Vec<AracDenetimIsi> {
        Vec::new()
    }
}

/// Oturuma yüklenmiş ve belirli bir `(McpTarifi, McpAracTanimOzeti)` sürümüne
/// sabitlenmiş uzak MCP aracı.
pub struct UzakMcpArac {
    sunucu_adi: String,
    origin_host: McpAgHostu,
    sunucu_etiketi: McpKartEtiketi,
    arac_etiketi: McpKartEtiketi,
    sunum: McpAracSunumu,
    yuklenen_tarif: McpTarifi,
    yuklenen_d: McpAracTanimOzeti,
    sonuc_donusturucu: McpAracSonucDonusturucu,
    rpc_hata_donusturucu: McpAracRpcHataDonusturucu,
    katalog: Arc<UzakAracKatalogu>,
    kaynak: Arc<UzakAracKaynagi>,
    yurutucu: Arc<dyn UzakMcpCagriYurutucusu>,
}

impl UzakMcpArac {
    fn guncelligi_dogrula(&self) -> Result<(), AracHatasi> {
        let Some(guncel) = self.katalog.kayit_bul(&self.sunum.ad) else {
            return Err(AracHatasi::OnayEskidi);
        };
        if guncel.tarif != self.yuklenen_tarif || guncel.uzak_d != self.yuklenen_d {
            return Err(AracHatasi::OnayEskidi);
        }
        Ok(())
    }
}

#[async_trait]
impl Arac for UzakMcpArac {
    fn ad(&self) -> &str {
        &self.sunum.ad
    }

    fn aciklama(&self) -> &str {
        &self.sunum.aciklama
    }

    fn parametre_semasi(&self) -> Value {
        self.sunum.parametre_semasi.clone()
    }

    /// Uzak MCP araçlarının yan etkisizlik iddiasına yalnız sunucu annotation'ıyla
    /// güvenilmez; her uzak çağrı yazan/yan etkili araç olarak sınıflandırılır.
    fn yazar_mi(&self) -> bool {
        true
    }

    async fn hazirla(&self, _argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
        self.guncelligi_dogrula()?;
        let kip = *self
            .kaynak
            .oturum_kipi
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let ilk_cagri = self.kaynak.ilk_cagri_mi(&self.sunucu_adi);
        let mcp = McpAracHazirligi {
            sunucu: self.sunucu_etiketi.clone(),
            arac: self.arac_etiketi.clone(),
            kip,
            erisim: McpErisim::Yalitimli,
            ag: McpAgKarti::Sinirli {
                hostlar: vec![self.origin_host.clone()],
            },
            ilk_cagri,
            tarif: self.yuklenen_tarif,
            arac_tanimi: self.yuklenen_d,
        };
        Ok(Hazirlik {
            aciklama: format!(
                "`{}` uzak MCP sunucusundaki `{}` aracını çağırmak istiyor.",
                self.sunucu_etiketi.gorunen(),
                self.arac_etiketi.gorunen()
            ),
            tum_calisma_alani: false,
            kilitsiz: true,
            mcp: Some(mcp),
            ..Hazirlik::default()
        })
    }

    async fn calistir(
        &self,
        argumanlar: &Value,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.guncelligi_dogrula()?;
        let Some(mcp_hazirlik) = &hazirlik.mcp else {
            return Err(AracHatasi::OnayEskidi);
        };
        if mcp_hazirlik.tarif != self.yuklenen_tarif || mcp_hazirlik.arac_tanimi != self.yuklenen_d
        {
            return Err(AracHatasi::OnayEskidi);
        }
        let arg_nesnesi = argumanlar
            .as_object()
            .cloned()
            .ok_or_else(|| AracHatasi::Gecersiz("MCP araç girdisi nesne olmalıdır.".to_owned()))?;

        match self
            .yurutucu
            .uzak_arac_cagir(
                &self.sunucu_adi,
                self.arac_etiketi.gorunen(),
                arg_nesnesi,
                self.yuklenen_tarif,
                self.yuklenen_d,
                iptal,
            )
            .await
        {
            Ok((ham_sonuc, content_alani_var)) => {
                self.kaynak.ilk_cagri_yapildi_isaretle(&self.sunucu_adi);
                (self.sonuc_donusturucu)(ham_sonuc, content_alani_var)
            }
            Err(UzakCagriHatasi::OnayEskidi) => Err(AracHatasi::OnayEskidi),
            Err(UzakCagriHatasi::Rpc(rpc)) => Err((self.rpc_hata_donusturucu)(&rpc)),
            Err(UzakCagriHatasi::SonucBelirsiz(detay)) => Err(AracHatasi::Basarisiz(AracCiktisi {
                icerik: format!(
                    "Uzak MCP çağrısı sırasında bağlantı kesildi; sunucudaki yan etkinin tamamlanıp tamamlanmadığı bilinmiyor (otomatik tekrar yapılmadı): {detay}"
                ),
                kaynak: format!("mcp:{}", self.sunum.ad),
                kalfa_uretimi: true,
                ..AracCiktisi::default()
            })),
            Err(UzakCagriHatasi::Iletisim(detay)) => Err(AracHatasi::Gecersiz(format!(
                "Uzak MCP çağrısı tamamlanamadı: {detay}"
            ))),
        }
    }

    async fn calistir_kilitli(
        &self,
        argumanlar: &Value,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
        _ortam: AracOrtami,
        _kilit: Option<kalfa_core::kilit::KilitSeti>,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.calistir(argumanlar, hazirlik, iptal).await
    }
}

/// Denetlenmiş uzak MCP kataloğunda arama yapan ve seçilen araçları bir sonraki
/// model turunun görüntüsüne yükleyen yerleşik `arac_ara` aracı.
pub struct AracAraAraci {
    katalog: Arc<UzakAracKatalogu>,
    kaynak: Arc<UzakAracKaynagi>,
}

impl AracAraAraci {
    pub fn yeni(katalog: Arc<UzakAracKatalogu>, kaynak: Arc<UzakAracKaynagi>) -> Arc<Self> {
        Arc::new(Self { katalog, kaynak })
    }
}

#[async_trait]
impl Arac for AracAraAraci {
    fn ad(&self) -> &str {
        ARAC_ARA_ADI
    }

    fn aciklama(&self) -> &str {
        "Denetlenmiş uzak MCP araç kataloğunda arama yapar ve seçilen araçları sonraki model turu için yükler."
    }

    fn parametre_semasi(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "sorgu": {
                    "type": "string",
                    "description": "Aranacak araç adı, sunucu veya açıklama anahtar sözcüğü."
                },
                "sunucu": {
                    "type": "string",
                    "description": "İsteğe bağlı: yalnız belirtilen uzak MCP sunucusunda ara."
                },
                "yukle": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "İsteğe bağlı: sonraki model turu için yüklenecek birleşik araç adları (`mcp__<sunucu>__<arac>`)."
                }
            },
            "additionalProperties": false
        })
    }

    fn yazar_mi(&self) -> bool {
        false
    }

    async fn hazirla(&self, _argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
        Ok(Hazirlik {
            aciklama: "Uzak MCP araç kataloğunda arama yapmak istiyor.".to_owned(),
            kilitsiz: true,
            ..Hazirlik::default()
        })
    }

    async fn calistir(
        &self,
        argumanlar: &Value,
        _hazirlik: &Hazirlik,
        _iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        let sorgu = argumanlar
            .get("sorgu")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let sunucu = argumanlar.get("sunucu").and_then(Value::as_str);
        let yuklenecekler: Vec<String> = match argumanlar.get("yukle") {
            Some(Value::Array(dizi)) => {
                let mut adlar = Vec::with_capacity(dizi.len());
                for oge in dizi {
                    let Some(ad) = oge.as_str() else {
                        return Err(AracHatasi::Gecersiz(
                            "`yukle` dizisindeki her öğe metin olmalıdır.".to_owned(),
                        ));
                    };
                    adlar.push(ad.to_owned());
                }
                adlar
            }
            Some(_) => {
                return Err(AracHatasi::Gecersiz(
                    "`yukle` alanı metin dizisi olmalıdır.".to_owned(),
                ));
            }
            None => Vec::new(),
        };

        if sorgu.trim().is_empty() && sunucu.is_none() && yuklenecekler.is_empty() {
            return Err(AracHatasi::Gecersiz(
                "`arac_ara` için en az `sorgu`, `sunucu` veya `yukle` belirtilmelidir.".to_owned(),
            ));
        }

        let adaylar = if !sorgu.trim().is_empty() || sunucu.is_some() {
            self.katalog.ara(sorgu, sunucu)
        } else {
            Vec::new()
        };

        let yuklenenler = if !yuklenecekler.is_empty() {
            self.kaynak
                .araclari_yukle(&yuklenecekler)
                .map_err(|e| AracHatasi::Gecersiz(e.to_string()))?
        } else {
            Vec::new()
        };

        let butce = self.katalog.butce();
        let sonuc = AracAraSonucu {
            adaylar,
            yuklenenler,
            etkin_uzak_arac_sayisi: self.kaynak.etkin_arac_sayisi(),
            kullanilan_sema_bayt: self.kaynak.kullanilan_sema_bayt(),
            azami_sema_bayt: butce.sema_bayt_siniri(),
        };
        let icerik = serde_json::to_string(&sonuc)
            .map_err(|e| AracHatasi::Gecersiz(format!("arama sonucu serileştirilemedi: {e}")))?;

        Ok(AracCiktisi {
            icerik,
            kaynak: format!("kalfa:{ARAC_ARA_ADI}"),
            kalfa_uretimi: true,
            ..AracCiktisi::default()
        })
    }
}
