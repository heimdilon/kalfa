//! MCP sunucu ayarları, opak sır kimliği bağlaması ve atomik kasa/ayarlar işlemi.
//!
//! Bu modül Görev 3 kapsamında şu garantileri sağlar:
//! - `ayarlar.json` içindeki MCP sunucu tanımlarında gizli ortam değişkenleri
//!   yalnızca [`GizliKimlik`] (`mcp-<uuid>`) taşır; düz metin sır diske yazılmaz.
//! - Çoklu sır yazımı sırasında kasa veya ayar kaydı başarısız olursa yeni
//!   yazılan tüm yetim sır kimlikleri kasadan geri alınır (rollback) ve eski
//!   ayarlar ile eski sırlar bozulmadan kalır.
//! - Sunucu veya ortam değişkeni yeniden adlandırıldığında mevcut [`GizliKimlik`]
//!   korunur; gizli ortam değişkeni silindiğinde, düz metne çevrildiğinde veya
//!   sunucu kaldırıldığında kimlik kasadan silinir.
//! - Kasadan silme hatası gizlenmez: silinemeyen kimlikler
//!   [`McpAyarlar::silinecek_gizliler`] listesinde kalıcı olarak tutulur ve
//!   sonraki işlemlerde yeniden denenir.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::mcp::kasa::{
    EN_AZ_GIZLI_KARAKTER, GizliDegerHatasi, GizliKasa, GizliKimlik, KasaHatasi, gizli_deger_dogrula,
};

/// MCP sunucu adı için izin verilen en fazla karakter sayısı.
pub const AZAMI_SUNUCU_ADI_UZUNLUGU: usize = 32;

/// Varsayılan başlatma zaman aşımı (saniye).
pub const VARSAYILAN_BASLATMA_ZAMAN_ASIMI_SN: u32 = 30;

/// Varsayılan araç çağrısı zaman aşımı (saniye).
pub const VARSAYILAN_CAGRI_ZAMAN_ASIMI_SN: u32 = 120;

const fn varsayilan_baslatma_zaman_asimi() -> u32 {
    VARSAYILAN_BASLATMA_ZAMAN_ASIMI_SN
}

const fn varsayilan_cagri_zaman_asimi() -> u32 {
    VARSAYILAN_CAGRI_ZAMAN_ASIMI_SN
}

/// Uygulama ayarlarında (`ayarlar.json`) saklanan tüm MCP yapılandırması.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpAyarlar {
    #[serde(default)]
    pub sunucular: Vec<McpSunucu>,
    /// Önceki bir silme/güncelleme işleminde kasadan silinmesi hata vermiş ve
    /// yeniden denenecek yetim sır kimlikleri.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub silinecek_gizliler: Vec<GizliKimlik>,
    /// Seçili yürütülebilir kimliklerinin arka uçtaki mutlak dosya yolları.
    /// Yalnız arka uç ayarlarında tutulur; arayüz DTO'larına hiçbir zaman
    /// ham yol olarak gönderilmez.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secili_yurutulebilirler: BTreeMap<String, PathBuf>,
}

impl McpAyarlar {
    pub fn sunucu(&self, ad: &str) -> Option<&McpSunucu> {
        self.sunucular.iter().find(|s| s.ad == ad)
    }

    /// Ayarlarda aktif olarak referans verilen tüm sır kimliklerini döner.
    pub fn aktif_gizli_kimlikler(&self) -> BTreeSet<GizliKimlik> {
        let mut kumeler = BTreeSet::new();
        for sunucu in &self.sunucular {
            for degisken in &sunucu.ortam {
                if let McpOrtamDegeri::Gizli { kimlik } = &degisken.deger {
                    kumeler.insert(kimlik.clone());
                }
            }
        }
        kumeler
    }

    /// Arayüze gönderilecek sır-güvenli özet listesini üretir.
    pub fn ozet_listesi(&self) -> Vec<McpSunucuOzetiDto> {
        self.sunucular.iter().map(McpSunucu::ozet_dto).collect()
    }
}

/// Tek bir MCP sunucusunun kalıcı ayarı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpSunucu {
    pub ad: String,
    #[serde(default)]
    pub etkin: bool,
    pub komut: McpKomut,
    #[serde(default)]
    pub ortam: Vec<McpOrtamDegiskeni>,
    #[serde(default)]
    pub kip: McpKipi,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ag_izinleri: Vec<McpAgKurali>,
    #[serde(default = "varsayilan_baslatma_zaman_asimi")]
    pub baslatma_zamani_asimi_sn: u32,
    #[serde(default = "varsayilan_cagri_zaman_asimi")]
    pub cagri_zamani_asimi_sn: u32,
}

impl McpSunucu {
    /// Bu sunucunun sahip olduğu gizli kimliklerin kümesini döner.
    pub fn gizli_kimlikler(&self) -> BTreeSet<GizliKimlik> {
        self.ortam
            .iter()
            .filter_map(|d| match &d.deger {
                McpOrtamDegeri::Gizli { kimlik } => Some(kimlik.clone()),
                McpOrtamDegeri::Duz { .. } => None,
            })
            .collect()
    }

    /// Arayüz için sır içermeyen özet DTO'su üretir.
    pub fn ozet_dto(&self) -> McpSunucuOzetiDto {
        McpSunucuOzetiDto {
            ad: self.ad.clone(),
            etkin: self.etkin,
            kip: self.kip,
            gizli_ortam_sayisi: self
                .ortam
                .iter()
                .filter(|d| matches!(d.deger, McpOrtamDegeri::Gizli { .. }))
                .count(),
        }
    }

    /// Arayüz düzenleme formu için sır içermeyen ayrıntı DTO'su üretir.
    pub fn ayrinti_dto(&self) -> McpSunucuAyrintiDto {
        McpSunucuAyrintiDto {
            ad: self.ad.clone(),
            etkin: self.etkin,
            komut: self.komut.clone(),
            ortam: self
                .ortam
                .iter()
                .map(|d| match &d.deger {
                    McpOrtamDegeri::Duz { deger } => McpOrtamDegiskeniDto {
                        ad: d.ad.clone(),
                        gizli: false,
                        deger_acik: Some(deger.clone()),
                        gizli_var: false,
                        gizli_kimlik: None,
                    },
                    McpOrtamDegeri::Gizli { kimlik } => McpOrtamDegiskeniDto {
                        ad: d.ad.clone(),
                        gizli: true,
                        deger_acik: None,
                        gizli_var: true,
                        gizli_kimlik: Some(kimlik.clone()),
                    },
                })
                .collect(),
            kip: self.kip,
            ag_izinleri: self.ag_izinleri.clone(),
            baslatma_zamani_asimi_sn: self.baslatma_zamani_asimi_sn,
            cagri_zamani_asimi_sn: self.cagri_zamani_asimi_sn,
        }
    }
}

/// Sunucu çalıştırma kipi.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpKipi {
    #[default]
    Yalitimli,
    TamErisim,
}

/// Yalıtımlı sunucu için tek bir ağ izin kuralı.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpAgKurali {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub ozel_ag: bool,
}

/// Sunucu komutu: çıplak program adı ya da arka uçta seçilmiş yürütülebilir kimliği.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tur", rename_all = "snake_case", deny_unknown_fields)]
pub enum McpKomut {
    ProgramAdi {
        ad: String,
        #[serde(default)]
        argumanlar: Vec<String>,
    },
    SeciliYurutulebilir {
        kimlik: String,
        #[serde(default)]
        argumanlar: Vec<String>,
    },
}

/// Kalıcı ayardaki tek bir ortam değişkeni.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpOrtamDegiskeni {
    pub ad: String,
    pub deger: McpOrtamDegeri,
}

/// Kalıcı ayardaki ortam değişkeni değeri: düz metin ya da opak kasa kimliği.
///
/// Gizli değerler bu tipte **asla** düz metin olarak bulunmaz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tur", rename_all = "snake_case", deny_unknown_fields)]
pub enum McpOrtamDegeri {
    Duz { deger: String },
    Gizli { kimlik: GizliKimlik },
}

/// Arayüz listesi için sır taşımayan sunucu özeti DTO'su.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpSunucuOzetiDto {
    pub ad: String,
    pub etkin: bool,
    pub kip: McpKipi,
    pub gizli_ortam_sayisi: usize,
}

/// Arayüz ayrıntı/düzenleme ekranı için sır taşımayan sunucu DTO'su.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpSunucuAyrintiDto {
    pub ad: String,
    pub etkin: bool,
    pub komut: McpKomut,
    pub ortam: Vec<McpOrtamDegiskeniDto>,
    pub kip: McpKipi,
    pub ag_izinleri: Vec<McpAgKurali>,
    pub baslatma_zamani_asimi_sn: u32,
    pub cagri_zamani_asimi_sn: u32,
}

/// Arayüze gönderilen ortam değişkeni DTO'su.
///
/// `gizli == true` olduğunda `deger_acik` her zaman `None` olur; yalnızca
/// `gizli_var: true` ve opak `gizli_kimlik` gönderilir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpOrtamDegiskeniDto {
    pub ad: String,
    pub gizli: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deger_acik: Option<String>,
    pub gizli_var: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gizli_kimlik: Option<GizliKimlik>,
}

/// Yeni bir gizli ortam değeri girilirken kullanılan sarmalayıcı.
///
/// `Serialize` özellikle uygulanmaz; `Debug` çıktısı sabit `[gizli]` basar.
#[derive(Clone, PartialEq, Eq)]
pub struct YeniGizliDeger(String);

impl YeniGizliDeger {
    pub fn yeni(deger: impl Into<String>) -> Result<Self, GizliDegerHatasi> {
        let deger = deger.into();
        gizli_deger_dogrula(&deger)?;
        Ok(Self(deger))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for YeniGizliDeger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("YeniGizliDeger([gizli])")
    }
}

/// Sunucu ekleme/güncelleme taslağındaki tek bir ortam değişkeni girdisi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpOrtamGirdisi {
    /// Düz metin ortam değişkeni.
    Duz { ad: String, deger: String },
    /// Kasaya yeni yazılacak gizli ortam değişkeni.
    YeniGizli { ad: String, deger: YeniGizliDeger },
    /// Mevcut bir gizli ortam değişkeninin (örneğin sunucu veya değişken yeniden
    /// adlandırılırken) kasadaki kimliğini korur.
    KorunanGizli { ad: String, kimlik: GizliKimlik },
}

impl McpOrtamGirdisi {
    pub fn ad(&self) -> &str {
        match self {
            Self::Duz { ad, .. } | Self::YeniGizli { ad, .. } | Self::KorunanGizli { ad, .. } => ad,
        }
    }
}

/// Sunucu ekleme veya düzenleme taslağı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpSunucuTaslagi {
    pub ad: String,
    pub etkin: bool,
    pub komut: McpKomut,
    pub ortam: Vec<McpOrtamGirdisi>,
    pub kip: McpKipi,
    pub ag_izinleri: Vec<McpAgKurali>,
    pub baslatma_zamani_asimi_sn: u32,
    pub cagri_zamani_asimi_sn: u32,
}

/// Tek bir işlemde uygulanabilecek MCP ayar değişiklikleri.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpDegisikligi {
    /// Yeni sunucu ekler veya `eski_ad` verilmişse o sunucuyu günceller
    /// (yeniden adlandırma dahil).
    Kaydet {
        eski_ad: Option<String>,
        taslak: McpSunucuTaslagi,
    },
    /// Sunucuyu yalnız yeniden adlandırır; tüm `GizliKimlik` bağlarını korur.
    YenidenAdlandir { eski_ad: String, yeni_ad: String },
    /// Sunucuyu ve sahip olduğu tüm gizli kimlikleri kaldırır.
    Sil { ad: String },
    /// Sunucunun etkinliğini değiştirir.
    EtkinlikAyarla { ad: String, etkin: bool },
    /// Arka uçta seçilmiş bir yürütülebilir kimliğini mutlak dosya yoluyla kaydeder.
    SeciliYurutulebilirKaydet { kimlik: String, yol: PathBuf },
}

/// Ayarlar deposu için atomik okuma/yazma sözleşmesi.
///
/// `ayarlar.rs` entegrasyonunda `Ayarlar` bu trait'i uygular; testlerde hata
/// enjeksiyonu için sahte depo kullanılabilir.
pub trait McpAyarlarDeposu: Send + Sync {
    /// Mevcut MCP ayarlarını ve revizyon numarasını döner.
    fn mcp_ayarlarini_oku(&self) -> (McpAyarlar, u64);

    /// `beklenen_revizyon` eşleşiyorsa yeni MCP ayarlarını diske atomik olarak
    /// yazar ve yeni revizyonu döner.
    fn mcp_ayarlarini_kaydet(
        &self,
        yeni: &McpAyarlar,
        beklenen_revizyon: u64,
    ) -> Result<u64, McpDepoHatasi>;
}

impl<T: McpAyarlarDeposu + ?Sized> McpAyarlarDeposu for std::sync::Arc<T> {
    fn mcp_ayarlarini_oku(&self) -> (McpAyarlar, u64) {
        (**self).mcp_ayarlarini_oku()
    }

    fn mcp_ayarlarini_kaydet(
        &self,
        yeni: &McpAyarlar,
        beklenen_revizyon: u64,
    ) -> Result<u64, McpDepoHatasi> {
        (**self).mcp_ayarlarini_kaydet(yeni, beklenen_revizyon)
    }
}

/// Ayarlar deposu yazma hataları.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum McpDepoHatasi {
    #[error("ayarlar başka bir işlem tarafından değiştirildi")]
    EsZamanliDegisiklik,
    #[error("ayarlar dosyası yazılamadı: {0}")]
    Io(String),
}

/// MCP ayar doğrulama ve işlem hataları.
///
/// Hiçbir hata varyantı ham sır metnini taşımaz.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum McpAyarlarHatasi {
    #[error("geçersiz sunucu adı")]
    GecersizSunucuAdi,
    #[error("aynı adla bir MCP sunucusu zaten kayıtlı")]
    YinelenenSunucuAdi,
    #[error("MCP sunucusu bulunamadı")]
    SunucuBulunamadi,
    #[error("geçersiz MCP komutu")]
    GecersizKomut,
    #[error("geçersiz ortam değişkeni adı")]
    GecersizOrtamAdi,
    #[error("ortam değişkeni adı ayrılmış veya izinli değil")]
    YasakliOrtamAdi,
    #[error("aynı ortam değişkeni adı birden fazla kez tanımlanamaz")]
    YinelenenOrtamAdi,
    #[error("korunan gizli değer kimliği mevcut sunucuya ait değil")]
    BilinmeyenKorunanGizliKimlik,
    #[error("aynı gizli değer kimliği birden fazla ortam değişkeninde kullanılamaz")]
    PaylasilanGizliKimlik,
    #[error("tam erişim kipinde ağ izin listesi tanımlanamaz")]
    TamErisimdeAgListesi,
    #[error("zaman aşımı en az 1 saniye olmalıdır")]
    SifirZamanAsimi,
    #[error("gizli değer en az {EN_AZ_GIZLI_KARAKTER} karakter olmalıdır")]
    GizliDegerHatasi(#[from] GizliDegerHatasi),
    #[error("gizli değer kasası hatası: {0}")]
    Kasa(#[from] KasaHatasi),
    #[error("ayarlar deposu hatası: {0}")]
    Depo(#[from] McpDepoHatasi),
    #[error(
        "ayarlar kaydedildi ancak bazı eski gizli değerler kasadan silinemedi ({} kayıt tekrar denenecek)",
        kalanlar.len()
    )]
    TemizlikGerekiyor { kalanlar: Vec<GizliKimlik> },
}

impl McpAyarlarHatasi {
    /// Hata, ayarların diske zaten yazıldığı ancak kasa temizliğinin sonraki
    /// denemeye kaldığı durumu mu gösteriyor?
    pub const fn kayit_tamamlandi_mi(&self) -> bool {
        matches!(self, Self::TemizlikGerekiyor { .. })
    }
}

/// Atomik MCP ayarlar + sır kasası işlemi.
pub struct McpAyarlarIslemi {
    baz_ayarlar: McpAyarlar,
    baz_revizyon: u64,
    degisiklikler: Vec<McpDegisikligi>,
}

impl McpAyarlarIslemi {
    /// Depodan güncel anlık görüntüyü ve revizyonu okuyarak yeni bir işlem başlatır.
    pub fn baslat(depo: &dyn McpAyarlarDeposu) -> Self {
        let (baz_ayarlar, baz_revizyon) = depo.mcp_ayarlarini_oku();
        Self {
            baz_ayarlar,
            baz_revizyon,
            degisiklikler: Vec::new(),
        }
    }

    pub fn degisiklik_ekle(&mut self, degisiklik: McpDegisikligi) {
        self.degisiklikler.push(degisiklik);
    }

    pub fn baz_revizyon(&self) -> u64 {
        self.baz_revizyon
    }

    /// Bekleyen değişiklikleri doğrular, yeni sırları kasaya yazar, ayarları
    /// kaydeder ve emekliye ayrılan sır kimliklerini kasadan siler.
    ///
    /// - Çoklu sır yazımında veya ayar kaydında hata çıkarsa, bu işlemde kasaya
    ///   yazılmış tüm yeni kimlikler geri silinir (rollback) ve eski ayarlar
    ///   değişmeden kalır.
    /// - Ayarlar kaydedildikten sonra eski sırların kasadan silinmesi hata
    ///   verirse, silinemeyen kimlikler `silinecek_gizliler` içinde kalıcı
    ///   olarak saklanır ve [`McpAyarlarHatasi::TemizlikGerekiyor`] döner.
    pub fn kaydet(
        self,
        depo: &dyn McpAyarlarDeposu,
        kasa: &dyn GizliKasa,
    ) -> Result<McpAyarlar, McpAyarlarHatasi> {
        let mut aday = self.baz_ayarlar.clone();
        let mut yeni_yazilacaklar: Vec<(GizliKimlik, String)> = Vec::new();
        let mut emekli_kimlikler: BTreeSet<GizliKimlik> =
            aday.silinecek_gizliler.iter().cloned().collect();

        for degisiklik in self.degisiklikler {
            match degisiklik {
                McpDegisikligi::Kaydet { eski_ad, taslak } => {
                    sunucu_taslagini_dogrula(&taslak)?;

                    let mevcut_indis = match &eski_ad {
                        Some(ad) => Some(
                            aday.sunucular
                                .iter()
                                .position(|s| s.ad == *ad)
                                .ok_or(McpAyarlarHatasi::SunucuBulunamadi)?,
                        ),
                        None => None,
                    };

                    if aday
                        .sunucular
                        .iter()
                        .enumerate()
                        .any(|(i, s)| Some(i) != mevcut_indis && s.ad == taslak.ad)
                    {
                        return Err(McpAyarlarHatasi::YinelenenSunucuAdi);
                    }

                    let mevcut_kimlikler: BTreeSet<GizliKimlik> = mevcut_indis
                        .map(|i| aday.sunucular[i].gizli_kimlikler())
                        .unwrap_or_default();

                    let diger_sunucu_kimlikleri: BTreeSet<GizliKimlik> = aday
                        .sunucular
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| Some(*i) != mevcut_indis)
                        .flat_map(|(_, s)| s.gizli_kimlikler())
                        .collect();

                    let mut korunacaklar = BTreeSet::new();
                    let mut ortam = Vec::with_capacity(taslak.ortam.len());

                    for girdi in taslak.ortam {
                        match girdi {
                            McpOrtamGirdisi::Duz { ad, deger } => {
                                ortam.push(McpOrtamDegiskeni {
                                    ad,
                                    deger: McpOrtamDegeri::Duz { deger },
                                });
                            }
                            McpOrtamGirdisi::YeniGizli { ad, deger } => {
                                let kimlik = benzersiz_kimlik_uret(
                                    &mevcut_kimlikler,
                                    &diger_sunucu_kimlikleri,
                                    &emekli_kimlikler,
                                    &yeni_yazilacaklar,
                                );
                                yeni_yazilacaklar.push((kimlik.clone(), deger.as_str().to_owned()));
                                ortam.push(McpOrtamDegiskeni {
                                    ad,
                                    deger: McpOrtamDegeri::Gizli { kimlik },
                                });
                            }
                            McpOrtamGirdisi::KorunanGizli { ad, kimlik } => {
                                let _ = GizliKimlik::yeni_dogrulanmis(kimlik.as_str())?;
                                if !mevcut_kimlikler.contains(&kimlik) {
                                    return Err(McpAyarlarHatasi::BilinmeyenKorunanGizliKimlik);
                                }
                                if !korunacaklar.insert(kimlik.clone()) {
                                    return Err(McpAyarlarHatasi::PaylasilanGizliKimlik);
                                }
                                ortam.push(McpOrtamDegiskeni {
                                    ad,
                                    deger: McpOrtamDegeri::Gizli { kimlik },
                                });
                            }
                        }
                    }

                    for eski_kimlik in mevcut_kimlikler.difference(&korunacaklar) {
                        emekli_kimlikler.insert(eski_kimlik.clone());
                    }

                    let yeni_sunucu = McpSunucu {
                        ad: taslak.ad,
                        etkin: taslak.etkin,
                        komut: taslak.komut,
                        ortam,
                        kip: taslak.kip,
                        ag_izinleri: taslak.ag_izinleri,
                        baslatma_zamani_asimi_sn: taslak.baslatma_zamani_asimi_sn,
                        cagri_zamani_asimi_sn: taslak.cagri_zamani_asimi_sn,
                    };

                    if let Some(indis) = mevcut_indis {
                        aday.sunucular[indis] = yeni_sunucu;
                    } else {
                        aday.sunucular.push(yeni_sunucu);
                    }
                }
                McpDegisikligi::YenidenAdlandir { eski_ad, yeni_ad } => {
                    if !sunucu_adi_gecerli(&yeni_ad) {
                        return Err(McpAyarlarHatasi::GecersizSunucuAdi);
                    }
                    let indis = aday
                        .sunucular
                        .iter()
                        .position(|s| s.ad == eski_ad)
                        .ok_or(McpAyarlarHatasi::SunucuBulunamadi)?;
                    if aday
                        .sunucular
                        .iter()
                        .enumerate()
                        .any(|(i, s)| i != indis && s.ad == yeni_ad)
                    {
                        return Err(McpAyarlarHatasi::YinelenenSunucuAdi);
                    }
                    aday.sunucular[indis].ad = yeni_ad;
                }
                McpDegisikligi::Sil { ad } => {
                    let indis = aday
                        .sunucular
                        .iter()
                        .position(|s| s.ad == ad)
                        .ok_or(McpAyarlarHatasi::SunucuBulunamadi)?;
                    let silinen = aday.sunucular.remove(indis);
                    for kimlik in silinen.gizli_kimlikler() {
                        emekli_kimlikler.insert(kimlik);
                    }
                }
                McpDegisikligi::EtkinlikAyarla { ad, etkin } => {
                    let sunucu = aday
                        .sunucular
                        .iter_mut()
                        .find(|s| s.ad == ad)
                        .ok_or(McpAyarlarHatasi::SunucuBulunamadi)?;
                    sunucu.etkin = etkin;
                }
                McpDegisikligi::SeciliYurutulebilirKaydet { kimlik, yol } => {
                    if kimlik.trim().is_empty() || !yol.is_absolute() {
                        return Err(McpAyarlarHatasi::GecersizKomut);
                    }
                    aday.secili_yurutulebilirler.insert(kimlik, yol);
                }
            }
        }

        // Aktif bir sunucuda hâlâ kullanılan hiçbir kimlik silinecekler listesinde kalmamalı.
        let aktifler = aday.aktif_gizli_kimlikler();
        emekli_kimlikler.retain(|k| !aktifler.contains(k));
        aday.silinecek_gizliler = emekli_kimlikler.iter().cloned().collect();

        // Hiçbir sunucu tarafından referans verilmeyen seçili yürütülebilir yollarını temizle.
        let aktif_komut_kimlikleri: BTreeSet<&str> = aday
            .sunucular
            .iter()
            .filter_map(|s| match &s.komut {
                McpKomut::SeciliYurutulebilir { kimlik, .. } => Some(kimlik.as_str()),
                McpKomut::ProgramAdi { .. } => None,
            })
            .collect();
        aday.secili_yurutulebilirler
            .retain(|k, _| aktif_komut_kimlikleri.contains(k.as_str()));

        // 1. Aşama: Yeni sırları yeni kimliklere yaz. Hata olursa yazılanları geri al.
        let mut yazilan_yeni_kimlikler: Vec<GizliKimlik> =
            Vec::with_capacity(yeni_yazilacaklar.len());
        for (kimlik, deger) in &yeni_yazilacaklar {
            if let Err(hata) = kasa.yaz(kimlik, deger) {
                for geri_alinacak in yazilan_yeni_kimlikler.iter().rev() {
                    let _ = kasa.sil(geri_alinacak);
                }
                return Err(McpAyarlarHatasi::Kasa(hata));
            }
            yazilan_yeni_kimlikler.push(kimlik.clone());
        }

        // 2. Aşama: Ayarları (ve varsa `silinecek_gizliler` kuyruğunu) atomik olarak kaydet.
        let guncel_revizyon = match depo.mcp_ayarlarini_kaydet(&aday, self.baz_revizyon) {
            Ok(rev) => rev,
            Err(depo_hatasi) => {
                for geri_alinacak in yazilan_yeni_kimlikler.iter().rev() {
                    let _ = kasa.sil(geri_alinacak);
                }
                return Err(McpAyarlarHatasi::Depo(depo_hatasi));
            }
        };

        // 3. Aşama: Emekliye ayrılan kimlikleri kasadan sil.
        if emekli_kimlikler.is_empty() {
            return Ok(aday);
        }

        let mut kalan_silinecekler = Vec::new();
        for kimlik in &emekli_kimlikler {
            if kasa.sil(kimlik).is_err() {
                kalan_silinecekler.push(kimlik.clone());
            }
        }

        if kalan_silinecekler != aday.silinecek_gizliler {
            aday.silinecek_gizliler = kalan_silinecekler.clone();
            // Kuyruk güncellemesini kaydetmeyi dene; eşzamanlı değişiklik olduysa
            // silme durumu bir sonraki işlemde tekrar temizlenir.
            let _ = depo.mcp_ayarlarini_kaydet(&aday, guncel_revizyon);
        }

        if !kalan_silinecekler.is_empty() {
            return Err(McpAyarlarHatasi::TemizlikGerekiyor {
                kalanlar: kalan_silinecekler,
            });
        }

        Ok(aday)
    }
}

fn benzersiz_kimlik_uret(
    mevcut: &BTreeSet<GizliKimlik>,
    diger: &BTreeSet<GizliKimlik>,
    emekli: &BTreeSet<GizliKimlik>,
    yeni_yazilacaklar: &[(GizliKimlik, String)],
) -> GizliKimlik {
    loop {
        let aday = GizliKimlik::yeni();
        if !mevcut.contains(&aday)
            && !diger.contains(&aday)
            && !emekli.contains(&aday)
            && !yeni_yazilacaklar.iter().any(|(k, _)| k == &aday)
        {
            return aday;
        }
    }
}

pub(crate) fn sunucu_taslagini_dogrula(taslak: &McpSunucuTaslagi) -> Result<(), McpAyarlarHatasi> {
    use kalfa_mcp::ortam::OrtamDegeri;
    use kalfa_mcp::tanim::{
        AgIzinKurali, Komut, McpTanim, SeciliYurutulebilirKimligi, TanimHatasi,
    };
    use kalfa_sandbox::CalistirmaKipi;

    let komut = match &taslak.komut {
        McpKomut::ProgramAdi { ad, argumanlar } => Komut::ProgramAdi {
            ad: ad.clone(),
            argumanlar: argumanlar.clone(),
        },
        McpKomut::SeciliYurutulebilir { kimlik, argumanlar } => {
            if kimlik.trim().is_empty() || kimlik.contains('\0') {
                return Err(McpAyarlarHatasi::GecersizKomut);
            }
            Komut::SeciliYurutulebilir {
                kimlik: SeciliYurutulebilirKimligi::yeni(1),
                argumanlar: argumanlar.clone(),
            }
        }
    };

    let mut ortam = Vec::with_capacity(taslak.ortam.len());
    for girdi in &taslak.ortam {
        match girdi {
            McpOrtamGirdisi::Duz { ad, deger } => {
                ortam.push(OrtamDegeri::duz(ad.clone(), deger.clone()));
            }
            McpOrtamGirdisi::YeniGizli { ad, deger } => {
                gizli_deger_dogrula(deger.as_str())?;
                ortam.push(OrtamDegeri::gizli(ad.clone(), deger.as_str()));
            }
            McpOrtamGirdisi::KorunanGizli { ad, .. } => {
                ortam.push(OrtamDegeri::gizli(ad.clone(), "korunan-gizli-yer-tutucu"));
            }
        }
    }

    let kip = match taslak.kip {
        McpKipi::Yalitimli => CalistirmaKipi::Yalitimli,
        McpKipi::TamErisim => CalistirmaKipi::TamErisim,
    };

    let ag = if taslak.ag_izinleri.is_empty() {
        None
    } else {
        let mut kurallar = Vec::with_capacity(taslak.ag_izinleri.len());
        for kural in &taslak.ag_izinleri {
            if kural.host.trim().is_empty() || kural.port == 0 {
                return Err(McpAyarlarHatasi::GecersizKomut);
            }
            let birlesik = format!("{}:{}", kural.host, kural.port);
            let k = AgIzinKurali::yeni(&birlesik, kural.ozel_ag)
                .map_err(|_| McpAyarlarHatasi::GecersizKomut)?;
            kurallar.push(k);
        }
        Some(kurallar)
    };

    McpTanim::yeni(
        taslak.ad.clone(),
        komut,
        ortam,
        kip,
        ag,
        taslak.baslatma_zamani_asimi_sn,
        taslak.cagri_zamani_asimi_sn,
    )
    .map(|_| ())
    .map_err(|hata| match hata {
        TanimHatasi::GecersizAd => McpAyarlarHatasi::GecersizSunucuAdi,
        TanimHatasi::GecersizKomut | TanimHatasi::GoreliIkiliYolu => {
            McpAyarlarHatasi::GecersizKomut
        }
        TanimHatasi::YinelenenOrtamAdi => McpAyarlarHatasi::YinelenenOrtamAdi,
        TanimHatasi::GecersizOrtamAdi | TanimHatasi::GecersizOrtamDegeri => {
            McpAyarlarHatasi::GecersizOrtamAdi
        }
        TanimHatasi::AyrilmisOrtamAdi
        | TanimHatasi::SabitOrtamAdi
        | TanimHatasi::OzelKokOrtamAdi => McpAyarlarHatasi::YasakliOrtamAdi,
        TanimHatasi::TamErisimAgListesi => McpAyarlarHatasi::TamErisimdeAgListesi,
        TanimHatasi::SifirZamanAsimi => McpAyarlarHatasi::SifirZamanAsimi,
        TanimHatasi::GecersizAgKurali
        | TanimHatasi::KomutCozulemedi
        | TanimHatasi::YolCozumlenemedi
        | TanimHatasi::CiktiYoluDegisti
        | TanimHatasi::DosyaOkunamadi
        | TanimHatasi::ProfilKuraliEksik => McpAyarlarHatasi::GecersizKomut,
    })
}

pub fn sunucu_adi_gecerli(ad: &str) -> bool {
    !ad.is_empty()
        && ad.len() <= AZAMI_SUNUCU_ADI_UZUNLUGU
        && ad
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Çalıştırma anında sunucunun ortam değişkenlerini kasadan çözer.
///
/// Bir gizli kimlik kasada bulunamazsa ya da kasa erişilemezse sunucu
/// başlatılmaz (`Err` döner).
pub fn sunucu_ortamini_coz(
    sunucu: &McpSunucu,
    kasa: &dyn GizliKasa,
) -> Result<Vec<(String, String, bool)>, KasaHatasi> {
    let mut cozulmus = Vec::with_capacity(sunucu.ortam.len());
    for degisken in &sunucu.ortam {
        match &degisken.deger {
            McpOrtamDegeri::Duz { deger } => {
                cozulmus.push((degisken.ad.clone(), deger.clone(), false));
            }
            McpOrtamDegeri::Gizli { kimlik } => {
                let Some(gizli_deger) = kasa.oku(kimlik)? else {
                    return Err(KasaHatasi::ErisimHatasi);
                };
                gizli_deger_dogrula(&gizli_deger)?;
                cozulmus.push((degisken.ad.clone(), gizli_deger, true));
            }
        }
    }
    Ok(cozulmus)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use crate::mcp::kasa::test_destek::SahteGizliKasa;

    use super::*;

    #[derive(Default)]
    struct SahteMcpDeposu {
        durum: Mutex<(McpAyarlar, u64)>,
        kaydet_hatasi: Mutex<Option<McpDepoHatasi>>,
    }

    impl SahteMcpDeposu {
        fn yeni() -> Self {
            Self::default()
        }

        fn kaydet_hatasi_ayarla(&self, hata: Option<McpDepoHatasi>) {
            *self.kaydet_hatasi.lock().unwrap() = hata;
        }
    }

    impl McpAyarlarDeposu for SahteMcpDeposu {
        fn mcp_ayarlarini_oku(&self) -> (McpAyarlar, u64) {
            self.durum.lock().unwrap().clone()
        }

        fn mcp_ayarlarini_kaydet(
            &self,
            yeni: &McpAyarlar,
            beklenen_revizyon: u64,
        ) -> Result<u64, McpDepoHatasi> {
            if let Some(hata) = self.kaydet_hatasi.lock().unwrap().clone() {
                return Err(hata);
            }
            let mut durum = self.durum.lock().unwrap();
            if durum.1 != beklenen_revizyon {
                return Err(McpDepoHatasi::EsZamanliDegisiklik);
            }
            durum.0 = yeni.clone();
            durum.1 += 1;
            Ok(durum.1)
        }
    }

    fn ornek_taslak(ad: &str, ortam: Vec<McpOrtamGirdisi>) -> McpSunucuTaslagi {
        McpSunucuTaslagi {
            ad: ad.to_owned(),
            etkin: true,
            komut: McpKomut::ProgramAdi {
                ad: "npx".to_owned(),
                argumanlar: vec!["-y".to_owned(), "ornek-paket@1.0.0".to_owned()],
            },
            ortam,
            kip: McpKipi::Yalitimli,
            ag_izinleri: Vec::new(),
            baslatma_zamani_asimi_sn: 30,
            cagri_zamani_asimi_sn: 120,
        }
    }

    #[test]
    fn iki_sunucu_ve_birden_cok_gizli_deger_birbirine_karismaz() {
        let depo = SahteMcpDeposu::yeni();
        let kasa = SahteGizliKasa::yeni();

        let mut islem = McpAyarlarIslemi::baslat(&depo);
        islem.degisiklik_ekle(McpDegisikligi::Kaydet {
            eski_ad: None,
            taslak: ornek_taslak(
                "sunucu-a",
                vec![
                    McpOrtamGirdisi::YeniGizli {
                        ad: "GITHUB_TOKEN".into(),
                        deger: YeniGizliDeger::yeni("ghp_sunucu_a_token_111").unwrap(),
                    },
                    McpOrtamGirdisi::YeniGizli {
                        ad: "API_KEY".into(),
                        deger: YeniGizliDeger::yeni("api_sunucu_a_key_222").unwrap(),
                    },
                    McpOrtamGirdisi::Duz {
                        ad: "LOG_LEVEL".into(),
                        deger: "info".into(),
                    },
                ],
            ),
        });
        islem.degisiklik_ekle(McpDegisikligi::Kaydet {
            eski_ad: None,
            taslak: ornek_taslak(
                "sunucu-b",
                vec![
                    McpOrtamGirdisi::YeniGizli {
                        ad: "GITHUB_TOKEN".into(),
                        deger: YeniGizliDeger::yeni("ghp_sunucu_b_token_333").unwrap(),
                    },
                    McpOrtamGirdisi::YeniGizli {
                        ad: "API_KEY".into(),
                        deger: YeniGizliDeger::yeni("api_sunucu_b_key_444").unwrap(),
                    },
                ],
            ),
        });

        let kaydedilen = islem.kaydet(&depo, &kasa).unwrap();
        assert_eq!(kaydedilen.sunucular.len(), 2);
        assert_eq!(kasa.kayit_sayisi(), 4);

        let a_kimlikleri = kaydedilen.sunucu("sunucu-a").unwrap().gizli_kimlikler();
        let b_kimlikleri = kaydedilen.sunucu("sunucu-b").unwrap().gizli_kimlikler();
        assert_eq!(a_kimlikleri.len(), 2);
        assert_eq!(b_kimlikleri.len(), 2);
        assert!(a_kimlikleri.is_disjoint(&b_kimlikleri));

        let a_cozulmus =
            sunucu_ortamini_coz(kaydedilen.sunucu("sunucu-a").unwrap(), &kasa).unwrap();
        let b_cozulmus =
            sunucu_ortamini_coz(kaydedilen.sunucu("sunucu-b").unwrap(), &kasa).unwrap();

        assert_eq!(
            a_cozulmus,
            vec![
                ("GITHUB_TOKEN".into(), "ghp_sunucu_a_token_111".into(), true),
                ("API_KEY".into(), "api_sunucu_a_key_222".into(), true),
                ("LOG_LEVEL".into(), "info".into(), false),
            ]
        );
        assert_eq!(
            b_cozulmus,
            vec![
                ("GITHUB_TOKEN".into(), "ghp_sunucu_b_token_333".into(), true),
                ("API_KEY".into(), "api_sunucu_b_key_444".into(), true),
            ]
        );
    }

    #[test]
    fn ayarlar_json_serilestirmesi_ve_dto_hicbir_zaman_ham_sir_tasimaz() {
        let depo = SahteMcpDeposu::yeni();
        let kasa = SahteGizliKasa::yeni();
        let gizli_metin = "cok-gizli-token-degeri-999";

        let yeni_gizli = YeniGizliDeger::yeni(gizli_metin).unwrap();
        assert!(!format!("{yeni_gizli:?}").contains(gizli_metin));

        let mut islem = McpAyarlarIslemi::baslat(&depo);
        islem.degisiklik_ekle(McpDegisikligi::Kaydet {
            eski_ad: None,
            taslak: ornek_taslak(
                "ornek",
                vec![McpOrtamGirdisi::YeniGizli {
                    ad: "SECRET_TOKEN".into(),
                    deger: yeni_gizli,
                }],
            ),
        });
        let ayarlar = islem.kaydet(&depo, &kasa).unwrap();

        let json = serde_json::to_string_pretty(&ayarlar).unwrap();
        assert!(!json.contains(gizli_metin));
        assert!(json.contains("\"mcp-"));

        let ayrinti = ayarlar.sunucu("ornek").unwrap().ayrinti_dto();
        let dto_json = serde_json::to_string(&ayrinti).unwrap();
        assert!(!dto_json.contains(gizli_metin));
        assert_eq!(ayrinti.ortam[0].deger_acik, None);
        assert!(ayrinti.ortam[0].gizli_var);
        assert!(ayrinti.ortam[0].gizli_kimlik.is_some());
    }

    #[test]
    fn yeniden_adlandirma_kimligi_korur_ve_silme_kasadan_kaldirir() {
        let depo = SahteMcpDeposu::yeni();
        let kasa = SahteGizliKasa::yeni();

        let mut islem = McpAyarlarIslemi::baslat(&depo);
        islem.degisiklik_ekle(McpDegisikligi::Kaydet {
            eski_ad: None,
            taslak: ornek_taslak(
                "ilk-ad",
                vec![
                    McpOrtamGirdisi::YeniGizli {
                        ad: "TOKEN_BIR".into(),
                        deger: YeniGizliDeger::yeni("token-bir-degeri-123").unwrap(),
                    },
                    McpOrtamGirdisi::YeniGizli {
                        ad: "TOKEN_IKI".into(),
                        deger: YeniGizliDeger::yeni("token-iki-degeri-456").unwrap(),
                    },
                ],
            ),
        });
        let ilk = islem.kaydet(&depo, &kasa).unwrap();
        let ilk_sunucu = ilk.sunucu("ilk-ad").unwrap();
        let McpOrtamDegeri::Gizli { kimlik: kimlik_1 } = &ilk_sunucu.ortam[0].deger else {
            panic!("gizli bekleniyordu");
        };
        let McpOrtamDegeri::Gizli { kimlik: kimlik_2 } = &ilk_sunucu.ortam[1].deger else {
            panic!("gizli bekleniyordu");
        };
        let kimlik_1 = kimlik_1.clone();
        let kimlik_2 = kimlik_2.clone();

        // 1) Doğrudan sunucu yeniden adlandırma: her iki kimlik de korunmalı.
        let mut islem = McpAyarlarIslemi::baslat(&depo);
        islem.degisiklik_ekle(McpDegisikligi::YenidenAdlandir {
            eski_ad: "ilk-ad".into(),
            yeni_ad: "ikinci-ad".into(),
        });
        let ikinci = islem.kaydet(&depo, &kasa).unwrap();
        assert_eq!(
            ikinci.sunucu("ikinci-ad").unwrap().gizli_kimlikler(),
            BTreeSet::from([kimlik_1.clone(), kimlik_2.clone()])
        );
        assert_eq!(kasa.kayit_sayisi(), 2);

        // 2) Düzenleme ile: TOKEN_BIR yeniden adlandırılıp korunuyor, TOKEN_IKI
        // düz metne çevriliyor (kasadan silinmeli).
        let mut islem = McpAyarlarIslemi::baslat(&depo);
        islem.degisiklik_ekle(McpDegisikligi::Kaydet {
            eski_ad: Some("ikinci-ad".into()),
            taslak: ornek_taslak(
                "ucuncu-ad",
                vec![
                    McpOrtamGirdisi::KorunanGizli {
                        ad: "YENI_TOKEN_BIR".into(),
                        kimlik: kimlik_1.clone(),
                    },
                    McpOrtamGirdisi::Duz {
                        ad: "TOKEN_IKI".into(),
                        deger: "artik-duz-deger".into(),
                    },
                ],
            ),
        });
        let ucuncu = islem.kaydet(&depo, &kasa).unwrap();
        assert_eq!(
            ucuncu.sunucu("ucuncu-ad").unwrap().gizli_kimlikler(),
            BTreeSet::from([kimlik_1.clone()])
        );
        assert_eq!(
            kasa.ham_oku(&kimlik_1).as_deref(),
            Some("token-bir-degeri-123")
        );
        assert_eq!(kasa.ham_oku(&kimlik_2), None);
        assert_eq!(kasa.kayit_sayisi(), 1);

        // 3) Sunucuyu tamamen silme: kalan kimlik de kasadan kaldırılmalı.
        let mut islem = McpAyarlarIslemi::baslat(&depo);
        islem.degisiklik_ekle(McpDegisikligi::Sil {
            ad: "ucuncu-ad".into(),
        });
        let son = islem.kaydet(&depo, &kasa).unwrap();
        assert!(son.sunucular.is_empty());
        assert_eq!(kasa.kayit_sayisi(), 0);
    }

    #[test]
    fn coklu_sir_yazma_hatasinda_yetim_kimlikler_geri_alinir_ve_eski_ayar_korunur() {
        let depo = SahteMcpDeposu::yeni();
        let kasa = SahteGizliKasa::yeni();

        // Önce geçerli bir sunucu ve sır kaydet.
        let mut ilk_islem = McpAyarlarIslemi::baslat(&depo);
        ilk_islem.degisiklik_ekle(McpDegisikligi::Kaydet {
            eski_ad: None,
            taslak: ornek_taslak(
                "mevcut",
                vec![McpOrtamGirdisi::YeniGizli {
                    ad: "ESKI_SIR".into(),
                    deger: YeniGizliDeger::yeni("eski-sir-degeri-korunmali").unwrap(),
                }],
            ),
        });
        let baslangic_ayarlari = ilk_islem.kaydet(&depo, &kasa).unwrap();
        let baslangic_kimlikleri = kasa.tum_kimlikler();
        assert_eq!(baslangic_kimlikleri.len(), 1);

        // Şimdi mevcut sunucuyu güncelleyen ve 2 yeni sır yazan bir işlem başlat;
        // 2. yeni sır yazımında (toplamda 3. `yaz` çağrısı) kasa hatası enjekte et.
        kasa.yaz_hatasi_ayarla(3, KasaHatasi::ErisimHatasi);

        let mut basarisiz_islem = McpAyarlarIslemi::baslat(&depo);
        basarisiz_islem.degisiklik_ekle(McpDegisikligi::Kaydet {
            eski_ad: Some("mevcut".into()),
            taslak: ornek_taslak(
                "mevcut",
                vec![
                    McpOrtamGirdisi::YeniGizli {
                        ad: "YENI_SIR_1".into(),
                        deger: YeniGizliDeger::yeni("yeni-sir-bir-12345").unwrap(),
                    },
                    McpOrtamGirdisi::YeniGizli {
                        ad: "YENI_SIR_2".into(),
                        deger: YeniGizliDeger::yeni("yeni-sir-iki-67890").unwrap(),
                    },
                ],
            ),
        });

        let sonuc = basarisiz_islem.kaydet(&depo, &kasa);
        assert_eq!(sonuc, Err(McpAyarlarHatasi::Kasa(KasaHatasi::ErisimHatasi)));

        // 1. yeni sır geri alınmış (rollback) olmalı; eski sır ve eski ayarlar
        // olduğu gibi durmalı.
        assert_eq!(kasa.tum_kimlikler(), baslangic_kimlikleri);
        assert_eq!(
            kasa.ham_oku(&baslangic_kimlikleri[0]).as_deref(),
            Some("eski-sir-degeri-korunmali")
        );
        assert_eq!(depo.mcp_ayarlarini_oku().0, baslangic_ayarlari);
    }

    #[test]
    fn depo_kayit_hatasinda_yeni_yazilan_sirlar_kasadan_geri_alinir() {
        let depo = SahteMcpDeposu::yeni();
        let kasa = SahteGizliKasa::yeni();
        depo.kaydet_hatasi_ayarla(Some(McpDepoHatasi::Io("disk dolu".into())));

        let mut islem = McpAyarlarIslemi::baslat(&depo);
        islem.degisiklik_ekle(McpDegisikligi::Kaydet {
            eski_ad: None,
            taslak: ornek_taslak(
                "sunucu",
                vec![McpOrtamGirdisi::YeniGizli {
                    ad: "TOKEN".into(),
                    deger: YeniGizliDeger::yeni("geri-alinacak-token-123").unwrap(),
                }],
            ),
        });

        let hata = islem.kaydet(&depo, &kasa).unwrap_err();
        assert!(matches!(hata, McpAyarlarHatasi::Depo(_)));
        assert_eq!(kasa.kayit_sayisi(), 0);
        assert!(depo.mcp_ayarlarini_oku().0.sunucular.is_empty());
    }

    #[test]
    fn kasa_silme_hatasi_gizlenmez_ve_sonraki_islemde_yeniden_denenir() {
        let depo = SahteMcpDeposu::yeni();
        let kasa = SahteGizliKasa::yeni();

        let mut kurulum = McpAyarlarIslemi::baslat(&depo);
        kurulum.degisiklik_ekle(McpDegisikligi::Kaydet {
            eski_ad: None,
            taslak: ornek_taslak(
                "gecici",
                vec![McpOrtamGirdisi::YeniGizli {
                    ad: "SILINECEK_TOKEN".into(),
                    deger: YeniGizliDeger::yeni("silinecek-token-degeri").unwrap(),
                }],
            ),
        });
        let ayarlar = kurulum.kaydet(&depo, &kasa).unwrap();
        let kimlik = ayarlar
            .sunucu("gecici")
            .unwrap()
            .gizli_kimlikler()
            .into_iter()
            .next()
            .unwrap();

        // Kasada bu kimliğin silinmesine hata enjekte et.
        kasa.sil_hatasi_ekle(kimlik.clone());

        let mut silme_islemi = McpAyarlarIslemi::baslat(&depo);
        silme_islemi.degisiklik_ekle(McpDegisikligi::Sil {
            ad: "gecici".into(),
        });
        let hata = silme_islemi.kaydet(&depo, &kasa).unwrap_err();
        assert_eq!(
            hata,
            McpAyarlarHatasi::TemizlikGerekiyor {
                kalanlar: vec![kimlik.clone()],
            }
        );

        // Sunucu ayarlardan kalkmış, ancak silinemeyen kimlik `silinecek_gizliler`
        // listesinde kalıcı olarak kaydedilmiş olmalı.
        let (kayitli, _) = depo.mcp_ayarlarini_oku();
        assert!(kayitli.sunucular.is_empty());
        assert_eq!(kayitli.silinecek_gizliler, vec![kimlik.clone()]);
        assert_eq!(kasa.kayit_sayisi(), 1);

        // Kasa hatası kalktığında bir sonraki işlem yetim kimliği temizlemeli.
        kasa.sil_hatalarini_temizle();
        let yeniden_dene = McpAyarlarIslemi::baslat(&depo);
        let temiz_ayarlar = yeniden_dene.kaydet(&depo, &kasa).unwrap();
        assert!(temiz_ayarlar.silinecek_gizliler.is_empty());
        assert_eq!(kasa.kayit_sayisi(), 0);
    }

    #[test]
    fn baska_sunucunun_gizli_kimligini_korunan_kimlik_olarak_calmak_reddedilir() {
        let depo = SahteMcpDeposu::yeni();
        let kasa = SahteGizliKasa::yeni();

        let mut kurulum = McpAyarlarIslemi::baslat(&depo);
        kurulum.degisiklik_ekle(McpDegisikligi::Kaydet {
            eski_ad: None,
            taslak: ornek_taslak(
                "hedef-sunucu",
                vec![McpOrtamGirdisi::YeniGizli {
                    ad: "GIZLI_TOKEN".into(),
                    deger: YeniGizliDeger::yeni("hedef-sunucu-ozel-sirri").unwrap(),
                }],
            ),
        });
        let ayarlar = kurulum.kaydet(&depo, &kasa).unwrap();
        let hedef_kimlik = ayarlar
            .sunucu("hedef-sunucu")
            .unwrap()
            .gizli_kimlikler()
            .into_iter()
            .next()
            .unwrap();

        // Başka bir sunucu `hedef_kimlik` değerini `KorunanGizli` olarak bağlamaya çalışırsa reddedilmeli.
        let mut saldiri = McpAyarlarIslemi::baslat(&depo);
        saldiri.degisiklik_ekle(McpDegisikligi::Kaydet {
            eski_ad: None,
            taslak: ornek_taslak(
                "saldirgan-sunucu",
                vec![McpOrtamGirdisi::KorunanGizli {
                    ad: "CALINAN_TOKEN".into(),
                    kimlik: hedef_kimlik,
                }],
            ),
        });
        assert_eq!(
            saldiri.kaydet(&depo, &kasa),
            Err(McpAyarlarHatasi::BilinmeyenKorunanGizliKimlik)
        );
    }
}
