//! Tur özeti ve geri alma (aşama 3 tasarımı §5; üst belge §6.7).
//!
//! Dosya araçlarının yazdığı dosyaların önceki hâlleri kaydın dışındaki bir
//! depoda durur: kayıt her geçmiş açılışında tümüyle okunduğu için içerik
//! oraya gömülmez. Depodaki dosyanın adı içeriğinin SHA-256 özetidir; kayda
//! yalnızca özet girer.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use kalfa_evren::tipler::Role;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::izin::{KORUMALI_UYARISI, Siniflandirici, YAPILANDIRMA_UYARISI, YolSinifi};
use crate::kayit::KayitOgesi;
use crate::kimlik::{CagriId, TurId};
use crate::muhur::{OnayMuhru, ozet_hex};
use crate::protokol::{AracDurumu, Zarf};
use crate::tur_ozeti::KomutOzeti;
use crate::yol::CalismaYolu;

/// Önceki hâli bundan büyük dosya saklanmaz; değişikliği geri alınamaz olur.
pub const DOSYA_SINIRI: u64 = 8 * 1024 * 1024;
/// Bir oturumun deposu tur sonunda bunu aşmışsa en eski turların verisi silinir.
pub const OTURUM_SINIRI: u64 = 512 * 1024 * 1024;
/// Silme, depo bu sınırın altına inene kadar sürer.
pub const OTURUM_ALT_SINIRI: u64 = 448 * 1024 * 1024;
/// Kayda yazılan diff'in bayt sınırı; aşan diff kayda girmez.
pub const DIFF_SINIRI: usize = 64 * 1024;

/// Bir yazmadan önceki hâl (aşama 3 tasarımı §5.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum OncekiHal {
    /// Dosya yoktu; geri alma onu siler.
    Yoktu,
    /// İçerik depoda bu özetin adıyla saklandı.
    Saklandi { ozet: String },
    /// İçerik saklanamayacak kadar büyüktü; değişiklik geri alınamaz.
    Buyuk { ozet: String, bayt: u64 },
}

/// Yazan bir çağrının dosya başına geri alma bilgisi. "Başladı" kaydına
/// (`AracBasladi.yedekler`) yazılır; içerik değil, özetler girer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DosyaYedegi {
    pub yol: CalismaYolu,
    pub onceki: OncekiHal,
    /// Yazılacak içeriğin özeti.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sonraki: Option<String>,
    /// Yazarken oluşan üst dizinler, sığdan derine.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub olusan_dizinler: Vec<CalismaYolu>,
    #[serde(default)]
    pub eklenen: u32,
    #[serde(default)]
    pub silinen: u32,
    /// En çok `DIFF_SINIRI` bayt; gizli dosyada hiç yoktur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
}

/// Bir dosyanın şimdiki durumu: içeriğinin özeti ve hard link sayısı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DosyaDurumu {
    pub ozet: String,
    pub baglanti_sayisi: u64,
}

/// Geri almanın çalışma alanına yazması (aşama 3 tasarımı §5.6). Çekirdek
/// `kalfa-tools`'a bağlı değildir; `kalfa_tools::CalismaAlani` bunu dosya
/// araçlarının güvenli yazma yoluyla uygular.
pub trait GeriYukleyici: Send + Sync {
    /// Şimdiki içeriğin özeti ve bağlantı sayısı; dosya yoksa `None`.
    fn durum(&self, yol: &CalismaYolu) -> io::Result<Option<DosyaDurumu>>;
    /// Şimdiki özeti `beklenen` olan dosyaya `icerik`'i yazar; `None` ise
    /// dosyayı siler. Özet tutmazsa ya da dosyanın başka bir adı (hard link)
    /// varsa `PermissionDenied` döner ve hiçbir şey yazmaz. Eksik dizin
    /// oluşturulmaz.
    fn geri_yaz(&self, yol: &CalismaYolu, icerik: Option<&[u8]>, beklenen: &str) -> io::Result<()>;
    /// Dizin boşsa kaldırır ve `true` döner; boş değilse ya da yoksa `false`.
    fn bos_dizini_kaldir(&self, yol: &CalismaYolu) -> io::Result<bool>;
}

/// Geri alma önizlemesinde gizli dosyanın uyarısı.
pub const GIZLI_GERI_ALMA_UYARISI: &str = "Gizli dosya; içerik gösterilmiyor.";
/// Yazan çağrının sonucu bilinmiyor (Kalfa yazma sırasında kapandı).
pub const BILINMEYEN_YAZMA_UYARISI: &str = "Bu dosyaya yazan çağrının sonucu bilinmiyor.";
/// Önizlemedeki diff'lerin toplam bayt sınırı; aşan öğelerde diff yoktur.
pub const PLAN_DIFF_SINIRI: usize = 1024 * 1024;
/// Kalfa notunda turun ilk mesajından alınan karakter sayısı.
const BASLIK_UZUNLUGU: usize = 60;

/// Bir çağrının geri alma bilgisi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CagriYedegi {
    pub cagri: CagriId,
    /// `AracBitti`'nin durumu; gelmediyse `SonucuBilinmiyor`.
    pub durum: AracDurumu,
    pub yedekler: Vec<DosyaYedegi>,
}

/// Bir turun geri alma bilgisi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurYedekleri {
    pub tur: TurId,
    /// Turun ilk kullanıcı mesajının ilk 60 karakteri.
    pub baslik: String,
    pub cagrilar: Vec<CagriYedegi>,
    /// Turun dosya değiştiren kabuk çağrıları (§5.4); geri alınmaz.
    pub komutlar: Vec<KomutOzeti>,
    /// Depo sınırı yüzünden turun önceki hâlleri silindi (§5.2).
    pub veri_silindi: bool,
}

impl TurYedekleri {
    /// Turdaki bir komut bu dosyayı değiştirdi mi?
    fn komut_degistirdi(&self, yol: &CalismaYolu) -> bool {
        self.komutlar
            .iter()
            .flat_map(|k| &k.degisiklikler)
            .flat_map(|d| &d.dosyalar)
            .any(|d| d.yol == yol.as_str())
    }
}

impl TurYedekleri {
    /// Turun önceki hâllerinin özetleri.
    fn saklanan_ozetler(&self) -> impl Iterator<Item = &str> {
        self.cagrilar
            .iter()
            .flat_map(|c| &c.yedekler)
            .filter_map(|y| match &y.onceki {
                OncekiHal::Saklandi { ozet } => Some(ozet.as_str()),
                OncekiHal::Yoktu | OncekiHal::Buyuk { .. } => None,
            })
    }
}

/// Oturumun turlarının geri alma bilgisi, kayıttaki sırayla. Canlı olarak
/// da kayıttan da (`kayittan`) kurulur.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GeriAlmaDizini {
    turlar: Vec<TurYedekleri>,
}

impl GeriAlmaDizini {
    pub fn tur_basladi(&mut self, tur: TurId, ilk_mesaj: &str) {
        if self.tur(tur).is_none() {
            self.turlar.push(TurYedekleri {
                tur,
                baslik: ilk_mesaj.chars().take(BASLIK_UZUNLUGU).collect(),
                cagrilar: Vec::new(),
                komutlar: Vec::new(),
                veri_silindi: false,
            });
        }
    }

    /// Yazan çağrı başladı; durumu `AracBitti` gelene kadar bilinmiyor.
    pub fn arac_basladi(&mut self, tur: TurId, cagri: CagriId, yedekler: Vec<DosyaYedegi>) {
        if yedekler.is_empty() {
            return;
        }
        self.tur_basladi(tur, "");
        if let Some(t) = self.turlar.iter_mut().find(|t| t.tur == tur) {
            t.cagrilar.push(CagriYedegi {
                cagri,
                durum: AracDurumu::SonucuBilinmiyor,
                yedekler,
            });
        }
    }

    pub fn arac_bitti(&mut self, cagri: CagriId, durum: AracDurumu) {
        let bulunan = self
            .turlar
            .iter_mut()
            .rev()
            .flat_map(|t| t.cagrilar.iter_mut())
            .find(|c| c.cagri == cagri);
        if let Some(c) = bulunan {
            c.durum = durum;
        }
    }

    /// Kabuk çağrısı dosya değiştirdi; değişiklik yoksa kaydedilmez.
    pub fn komut_degisti(&mut self, tur: TurId, komut: KomutOzeti) {
        if komut.degisiklikler.as_ref().is_none_or(|d| d.bos_mu()) {
            return;
        }
        self.tur_basladi(tur, "");
        if let Some(t) = self.turlar.iter_mut().find(|t| t.tur == tur) {
            t.komutlar.push(komut);
        }
    }

    pub fn veri_silindi(&mut self, turlar: &[TurId]) {
        for t in &mut self.turlar {
            if turlar.contains(&t.tur) {
                t.veri_silindi = true;
            }
        }
    }

    pub fn tur(&self, tur: TurId) -> Option<&TurYedekleri> {
        self.turlar.iter().find(|t| t.tur == tur)
    }

    /// Turlar, en eskisi önce.
    pub fn turlar(&self) -> &[TurYedekleri] {
        &self.turlar
    }

    /// Verisi silinmemiş turların kullandığı önceki hâllerin özetleri; depoda
    /// bunların dışındaki dosyalar silinebilir.
    pub fn kullanilan_ozetler(&self) -> HashSet<String> {
        self.turlar
            .iter()
            .filter(|t| !t.veri_silindi)
            .flat_map(TurYedekleri::saklanan_ozetler)
            .map(str::to_owned)
            .collect()
    }

    /// `tur`'un önceki hâllerinden, verisi silinmemiş başka bir turun
    /// kullanmadıkları.
    pub fn yalniz_kullanilan(&self, tur: TurId) -> Vec<String> {
        let kalan: HashSet<String> = self
            .turlar
            .iter()
            .filter(|t| t.tur != tur && !t.veri_silindi)
            .flat_map(TurYedekleri::saklanan_ozetler)
            .map(str::to_owned)
            .collect();
        let tekil: BTreeSet<String> = self
            .tur(tur)
            .into_iter()
            .flat_map(TurYedekleri::saklanan_ozetler)
            .filter(|o| !kalan.contains(*o))
            .map(str::to_owned)
            .collect();
        tekil.into_iter().collect()
    }

    /// Kayıttan kurar (kurtarma, aşama 3 tasarımı §5.7). "Bitti" kaydı
    /// olmayan çağrının durumu `SonucuBilinmiyor` kalır. Turu olmayan
    /// "başladı" kaydı (tur dışında yazılmış) sıfır kimlikli bir tura düşer:
    /// geri alınamaz ama önceki hâlleri depodan silinmez.
    pub fn kayittan(kayitlar: &[Zarf<KayitOgesi>]) -> Self {
        let mut dizin = Self::default();
        let mut durumlar: HashMap<CagriId, AracDurumu> = HashMap::new();
        for z in kayitlar {
            match (&z.yuk, z.tur) {
                (KayitOgesi::Mesaj { mesaj, .. }, Some(tur)) if mesaj.role == Role::User => {
                    dizin.tur_basladi(tur, mesaj.content.as_deref().unwrap_or_default());
                }
                (
                    KayitOgesi::AracBasladi {
                        cagri, yedekler, ..
                    },
                    tur,
                ) => dizin.arac_basladi(
                    tur.unwrap_or(TurId(uuid::Uuid::nil())),
                    *cagri,
                    yedekler.clone(),
                ),
                (KayitOgesi::AracBitti { cagri, durum }, _) => {
                    durumlar.insert(*cagri, *durum);
                    dizin.arac_bitti(*cagri, *durum);
                }
                (
                    KayitOgesi::KabukDegisiklikleri {
                        cagri,
                        komut,
                        degisiklikler,
                    },
                    Some(tur),
                ) => dizin.komut_degisti(
                    tur,
                    KomutOzeti {
                        cagri: *cagri,
                        komut: komut.clone(),
                        durum: durumlar
                            .get(cagri)
                            .copied()
                            .unwrap_or(AracDurumu::SonucuBilinmiyor),
                        degisiklikler: Some(degisiklikler.clone()),
                        ag: None,
                    },
                ),
                (KayitOgesi::GeriAlmaVerisiSilindi { turlar }, _) => dizin.veri_silindi(turlar),
                _ => {}
            }
        }
        dizin
    }
}

/// Bir dosyanın geri almada ne olacağı (aşama 3 tasarımı §5.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum GeriAlmaEylemi {
    /// Önceki içerik (`ozet`) geri yazılır.
    GeriYukle {
        ozet: String,
    },
    /// Dosyayı tur oluşturdu; silinir.
    Sil,
    /// Dosya zaten turdan önceki hâlinde.
    ZatenOnceki,
    /// Dosyaya dokunulmaz (K3).
    Cakisma {
        neden: CakismaNedeni,
    },
    GeriAlinamaz {
        neden: GeriAlinamazNedeni,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum CakismaNedeni {
    /// Dosya sonraki bir turda da değişti; önce o tur geri alınmalı.
    SonrakiTur { tur: TurId },
    /// Bir komut dosyayı değiştirdi (kabuk görüntüsü, 3a Task 6).
    Komut,
    /// Dosya, başarısız bir çağrının yazmak istediği hâlde; kimin yazdığı
    /// bilinmiyor.
    BasarisizCagri,
    /// Dosya Kalfa'dan sonra değişti.
    Elle,
    /// Dosyanın başka bir adı var; yazmak öteki adı da değiştirirdi.
    HardLink,
    /// Dosyanın şimdiki durumu okunamadı.
    Okunamadi { hata: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeriAlinamazNedeni {
    /// Önceki hâl 8 MiB'tan büyüktü.
    Buyuk,
    /// Depo sınırı yüzünden önceki hâl silindi.
    VeriSilindi,
    /// Kayıtsız oturumda önceki hâller saklanmaz.
    Kayitsiz,
    /// Komutların etkileri geri alınmaz (K2).
    Kabuk,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanOgesi {
    pub yol: CalismaYolu,
    pub eylem: GeriAlmaEylemi,
    /// Dosyanın şimdiki özeti; dosya yoksa `None`. Mühre girer.
    pub simdiki: Option<String>,
    pub uyarilar: Vec<String>,
    /// Geri alma yönündeki diff; gizli dosyada ve çakışmada yoktur.
    pub diff: Option<String>,
}

/// Geri alma önizlemesi. Onay, gösterilen plana mühürle bağlanır.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeriAlmaPlani {
    pub ogeler: Vec<PlanOgesi>,
    /// Turun dosya değiştiren komutları; geri alınmaz, yalnızca listelenir
    /// (K2). Mühre girmez.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub komutlar: Vec<KomutOzeti>,
    /// Turun önceki hâlleri depo sınırı yüzünden silindi.
    pub veri_silindi: bool,
}

impl GeriAlmaPlani {
    /// Yazılacak ya da silinecek en az bir dosya var mı?
    pub fn yapilacak_var_mi(&self) -> bool {
        self.ogeler.iter().any(|o| {
            matches!(
                o.eylem,
                GeriAlmaEylemi::GeriYukle { .. } | GeriAlmaEylemi::Sil
            )
        })
    }
}

/// Yürütülen geri almanın dosya başına sonucu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeriAlmaSonucu {
    pub yol: CalismaYolu,
    /// Plandaki eylem.
    pub eylem: GeriAlmaEylemi,
    /// Yürütmede yazılamadıysa nedeni.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hata: Option<String>,
}

/// Bir turdaki bir dosyanın geri alma bilgisi (§5.7'deki `A`, `S`, `F`).
struct DosyaBilgisi {
    /// Turda o dosyaya yazmaya başlamış ilk çağrının önceki hâli.
    a: OncekiHal,
    /// Kalfa'nın yazdığı ya da yazmış olabileceği hâller.
    s: HashSet<String>,
    /// Başarısız çağrıların yazmak istediği hâller.
    f: HashSet<String>,
    bilinmiyor: bool,
    /// `S`'deki yazmaların kayıttaki diff'leri, sırayla.
    diffler: Vec<String>,
}

fn dosya_bilgileri(t: &TurYedekleri) -> BTreeMap<CalismaYolu, DosyaBilgisi> {
    let mut dosyalar: BTreeMap<CalismaYolu, DosyaBilgisi> = BTreeMap::new();
    for c in &t.cagrilar {
        // İptal ve koşul değişikliği yazmadan önce denetlenir; bu çağrılar
        // dosyaya dokunmamıştır.
        if matches!(c.durum, AracDurumu::IptalEdildi | AracDurumu::Reddedildi) {
            continue;
        }
        for y in &c.yedekler {
            let d = dosyalar
                .entry(y.yol.clone())
                .or_insert_with(|| DosyaBilgisi {
                    a: y.onceki.clone(),
                    s: HashSet::new(),
                    f: HashSet::new(),
                    bilinmiyor: false,
                    diffler: Vec::new(),
                });
            let Some(sonraki) = y.sonraki.clone() else {
                continue;
            };
            match c.durum {
                AracDurumu::Basarili | AracDurumu::SonucuBilinmiyor => {
                    d.s.insert(sonraki);
                    d.bilinmiyor |= c.durum == AracDurumu::SonucuBilinmiyor;
                    d.diffler.extend(y.diff.clone());
                }
                AracDurumu::Basarisiz => {
                    d.f.insert(sonraki);
                }
                AracDurumu::IptalEdildi | AracDurumu::Reddedildi => {}
            }
        }
    }
    dosyalar
}

/// `tur`'da ya da sonraki turlarda bir komut bu dosyayı değiştirdi mi?
fn komut_degistirdi(dizin: &GeriAlmaDizini, tur: TurId, yol: &CalismaYolu) -> bool {
    let Some(sira) = dizin.turlar.iter().position(|t| t.tur == tur) else {
        return false;
    };
    dizin.turlar[sira..].iter().any(|t| t.komut_degistirdi(yol))
}

/// `tur`'dan sonraki turlardan bu dosyaya (başarıyla ya da belki) yazan son tur.
fn sonraki_yazan_tur(dizin: &GeriAlmaDizini, tur: TurId, yol: &CalismaYolu) -> Option<TurId> {
    let sira = dizin.turlar.iter().position(|t| t.tur == tur)?;
    dizin.turlar[sira + 1..]
        .iter()
        .rev()
        .find(|t| {
            t.cagrilar.iter().any(|c| {
                matches!(c.durum, AracDurumu::Basarili | AracDurumu::SonucuBilinmiyor)
                    && c.yedekler.iter().any(|y| &y.yol == yol)
            })
        })
        .map(|t| t.tur)
}

/// Turun geri alma planını çıkarır (aşama 3 tasarımı §5.6, §5.7). Tur
/// dizinde yoksa `None`.
pub fn plan_cikar(
    tur: TurId,
    dizin: &GeriAlmaDizini,
    depo: &OncekiHalDeposu,
    yukleyici: &dyn GeriYukleyici,
    siniflandirici: &Siniflandirici,
) -> Option<GeriAlmaPlani> {
    let t = dizin.tur(tur)?;
    // Git korumaları plan başına bir kez çözülür.
    let siniflandirici = siniflandirici.toplu();
    let mut ogeler = Vec::new();
    let mut diff_butcesi = PLAN_DIFF_SINIRI;
    for (yol, d) in dosya_bilgileri(t) {
        let (simdiki, eylem) = match yukleyici.durum(&yol) {
            Err(e) => (
                None,
                GeriAlmaEylemi::Cakisma {
                    neden: CakismaNedeni::Okunamadi {
                        hata: e.to_string(),
                    },
                },
            ),
            Ok(durum) => {
                let simdiki = durum.as_ref().map(|d| d.ozet.clone());
                let eylem = eylem_sec(
                    &d,
                    durum.as_ref(),
                    t,
                    depo,
                    || sonraki_yazan_tur(dizin, tur, &yol),
                    || komut_degistirdi(dizin, tur, &yol),
                );
                (simdiki, eylem)
            }
        };
        let gizli = siniflandirici.gizli_mi(&yol);
        let mut uyarilar = Vec::new();
        match siniflandirici.sinifla(&yol) {
            YolSinifi::Korumali => uyarilar.push(KORUMALI_UYARISI.to_string()),
            YolSinifi::Yapilandirma => uyarilar.push(YAPILANDIRMA_UYARISI.to_string()),
            YolSinifi::Gizli | YolSinifi::Normal => {}
        }
        if gizli {
            uyarilar.push(GIZLI_GERI_ALMA_UYARISI.to_string());
        }
        if d.bilinmiyor {
            uyarilar.push(BILINMEYEN_YAZMA_UYARISI.to_string());
        }
        let yazilacak = matches!(
            eylem,
            GeriAlmaEylemi::GeriYukle { .. } | GeriAlmaEylemi::Sil
        );
        let diff = (yazilacak && !gizli && !d.diffler.is_empty())
            .then(|| {
                d.diffler
                    .iter()
                    .rev()
                    .map(|f| ters_diff(f))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .filter(|f| f.len() <= diff_butcesi);
        if let Some(f) = &diff {
            diff_butcesi -= f.len();
        }
        ogeler.push(PlanOgesi {
            yol,
            eylem,
            simdiki,
            uyarilar,
            diff,
        });
    }
    Some(GeriAlmaPlani {
        ogeler,
        komutlar: t.komutlar.clone(),
        veri_silindi: t.veri_silindi,
    })
}

fn eylem_sec(
    d: &DosyaBilgisi,
    durum: Option<&DosyaDurumu>,
    t: &TurYedekleri,
    depo: &OncekiHalDeposu,
    sonraki_tur: impl FnOnce() -> Option<TurId>,
    komut: impl FnOnce() -> bool,
) -> GeriAlmaEylemi {
    let simdiki = durum.map(|d| d.ozet.as_str());
    let onceki = match &d.a {
        OncekiHal::Yoktu => None,
        OncekiHal::Saklandi { ozet } | OncekiHal::Buyuk { ozet, .. } => Some(ozet.as_str()),
    };
    if simdiki == onceki {
        return GeriAlmaEylemi::ZatenOnceki;
    }
    if durum.is_some_and(|d| d.baglanti_sayisi > 1) {
        return GeriAlmaEylemi::Cakisma {
            neden: CakismaNedeni::HardLink,
        };
    }
    if simdiki.is_some_and(|c| d.s.contains(c)) {
        return match &d.a {
            OncekiHal::Yoktu => GeriAlmaEylemi::Sil,
            OncekiHal::Buyuk { .. } => GeriAlmaEylemi::GeriAlinamaz {
                neden: GeriAlinamazNedeni::Buyuk,
            },
            OncekiHal::Saklandi { ozet } if !t.veri_silindi && depo.var_mi(ozet) => {
                GeriAlmaEylemi::GeriYukle { ozet: ozet.clone() }
            }
            OncekiHal::Saklandi { .. } => GeriAlmaEylemi::GeriAlinamaz {
                neden: GeriAlinamazNedeni::VeriSilindi,
            },
        };
    }
    let neden = if let Some(tur) = sonraki_tur() {
        CakismaNedeni::SonrakiTur { tur }
    } else if komut() {
        CakismaNedeni::Komut
    } else if simdiki.is_some_and(|c| d.f.contains(c)) {
        CakismaNedeni::BasarisizCagri
    } else {
        CakismaNedeni::Elle
    };
    GeriAlmaEylemi::Cakisma { neden }
}

/// Birleşik diff'i ters çevirir: eklenen satırlar silinen, silinenler
/// eklenen olur; başlıklar ve parça aralıkları yer değiştirir.
pub fn ters_diff(diff: &str) -> String {
    let satirlar: Vec<&str> = diff.lines().collect();
    let mut sonuc = Vec::with_capacity(satirlar.len());
    let mut i = 0;
    while i < satirlar.len() {
        let satir = satirlar[i];
        if let (Some(eski), Some(yeni)) = (
            satir.strip_prefix("--- "),
            satirlar.get(i + 1).and_then(|s| s.strip_prefix("+++ ")),
        ) {
            sonuc.push(format!("--- {yeni}"));
            sonuc.push(format!("+++ {eski}"));
            i += 2;
            continue;
        }
        if let Some(parca) = parca_basligi_cevir(satir) {
            sonuc.push(parca);
        } else if let Some(kalan) = satir.strip_prefix('+') {
            sonuc.push(format!("-{kalan}"));
        } else if let Some(kalan) = satir.strip_prefix('-') {
            sonuc.push(format!("+{kalan}"));
        } else {
            sonuc.push(satir.to_string());
        }
        i += 1;
    }
    sonuc.join("\n")
}

/// `@@ -a,b +c,d @@ …` → `@@ -c,d +a,b @@ …`.
fn parca_basligi_cevir(satir: &str) -> Option<String> {
    let govde = satir.strip_prefix("@@ ")?;
    let (araliklar, kuyruk) = govde.split_once(" @@")?;
    let (eski, yeni) = araliklar.split_once(' ')?;
    let eski = eski.strip_prefix('-')?;
    let yeni = yeni.strip_prefix('+')?;
    Some(format!("@@ -{yeni} +{eski} @@{kuyruk}"))
}

fn muhur_alani(h: &mut Sha256, veri: &[u8]) {
    h.update((veri.len() as u64).to_le_bytes());
    h.update(veri);
}

/// Geri almanın onay mührü: tur ve her öğenin yolu, eylemi ve şimdiki
/// özeti. Önizlemeden sonra değişen bir dosya mührü bozar.
pub fn plan_muhru(tur: TurId, plan: &GeriAlmaPlani) -> OnayMuhru {
    let mut h = Sha256::new();
    muhur_alani(&mut h, b"kalfa-geri-alma-v1");
    muhur_alani(&mut h, tur.to_string().as_bytes());
    for o in &plan.ogeler {
        muhur_alani(&mut h, o.yol.as_str().as_bytes());
        muhur_alani(
            &mut h,
            serde_json::to_string(&o.eylem)
                .unwrap_or_default()
                .as_bytes(),
        );
        muhur_alani(&mut h, o.simdiki.as_deref().unwrap_or("-").as_bytes());
    }
    OnayMuhru(hex::encode(h.finalize()))
}

/// Geri almadan sonra modele giden not (aşama 3 tasarımı §5.9).
pub fn kalfa_notu(baslik: &str, sonuclar: &[GeriAlmaSonucu], kabuk_var: bool) -> String {
    let mut donen = Vec::new();
    let mut silinen = Vec::new();
    let mut kalan = Vec::new();
    for s in sonuclar {
        match (&s.eylem, &s.hata) {
            (GeriAlmaEylemi::ZatenOnceki, _) => {}
            (GeriAlmaEylemi::GeriYukle { .. }, None) => donen.push(s.yol.as_str()),
            (GeriAlmaEylemi::Sil, None) => silinen.push(s.yol.as_str()),
            _ => kalan.push(s.yol.as_str()),
        }
    }
    let mut metin =
        format!("[Kalfa] Kullanıcı \"{baslik}\" turunun dosya değişikliklerini geri aldı.");
    if !donen.is_empty() {
        metin.push_str(&format!("\nÖnceki hâline dönen: {}.", donen.join(", ")));
    }
    if !silinen.is_empty() {
        metin.push_str(&format!(
            "\nSilinen (turda oluşturulmuştu): {}.",
            silinen.join(", ")
        ));
    }
    if !kalan.is_empty() {
        metin.push_str(&format!(
            "\nGeri alınmayan (sonradan değişmiş ya da geri alınamaz): {}.",
            kalan.join(", ")
        ));
    }
    if kabuk_var {
        metin.push_str(
            "
Kabukla yapılan değişiklikler geri alınmadı.",
        );
    }
    metin
}

/// Birleşik diff'teki eklenen ve silinen satırlar; başlık satırları
/// (`+++`, `---`) sayılmaz.
pub fn diff_sayilari(diff: &str) -> (u32, u32) {
    let mut eklenen = 0u32;
    let mut silinen = 0u32;
    for satir in diff.lines() {
        if satir.starts_with("+++") || satir.starts_with("---") {
            continue;
        }
        if satir.starts_with('+') {
            eklenen = eklenen.saturating_add(1);
        } else if satir.starts_with('-') {
            silinen = silinen.saturating_add(1);
        }
    }
    (eklenen, silinen)
}

const BOZUK_ILETISI: &str = "önceki hâl bozuk: içerik adındaki özetle uyuşmuyor";

/// Ad tam olarak 64 küçük onaltılık karakter mi? Depodaki bir dosyanın adı
/// yalnızca budur; bir yol parçası olamaz.
pub fn ozet_gecerli_mi(ozet: &str) -> bool {
    ozet.len() == 64 && ozet.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn ozet_denetle(ozet: &str) -> io::Result<()> {
    if ozet_gecerli_mi(ozet) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "geçersiz özet adı",
        ))
    }
}

/// Yazma sırasında yarım kalmış geçici dosya mı (`.<özet>.<uuid>.yeni`)?
fn yarim_mi(ad: &str) -> bool {
    ad.starts_with('.') && ad.ends_with(".yeni")
}

/// Bir oturumun önceki hâllerinin deposu. Dizin veri dizinindedir ve
/// yalıtımın içinde görünmez (aşama 3 tasarımı §5.2).
#[derive(Debug)]
pub struct OncekiHalDeposu {
    dizin: PathBuf,
}

impl OncekiHalDeposu {
    /// Dizini oluşturur ya da açar. Unix'te izinleri `0700`'dır.
    pub fn ac(dizin: &Path) -> io::Result<Self> {
        let mut kurucu = fs::DirBuilder::new();
        kurucu.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            kurucu.mode(0o700);
        }
        kurucu.create(dizin)?;
        let bilgi = fs::symlink_metadata(dizin)?;
        if !bilgi.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "geri alma deposu bir dizin değil",
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(dizin, fs::Permissions::from_mode(0o700))?;
        }
        Ok(Self {
            dizin: dizin.to_path_buf(),
        })
    }

    pub fn dizin(&self) -> &Path {
        &self.dizin
    }

    /// İçeriği özetinin adıyla saklar. Ad geçersizse ya da içerik özetle
    /// uyuşmuyorsa `InvalidInput`. Dosya zaten varsa hiçbir şey yapılmaz.
    /// Yazma önce geçici bir dosyaya yapılır ve kalıcılaştırılır, sonra asıl
    /// adına taşınır: çökme yarım bir önceki hâl bırakmaz.
    pub fn sakla(&self, ozet: &str, icerik: &[u8]) -> io::Result<()> {
        ozet_denetle(ozet)?;
        if ozet_hex(icerik) != ozet {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "içerik verilen özetle uyuşmuyor",
            ));
        }
        let hedef = self.dizin.join(ozet);
        if fs::symlink_metadata(&hedef).is_ok() {
            return Ok(());
        }
        let gecici = self
            .dizin
            .join(format!(".{ozet}.{}.yeni", uuid::Uuid::new_v4().simple()));
        let mut secenek = OpenOptions::new();
        secenek.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            secenek.mode(0o600);
        }
        let mut dosya = secenek.open(&gecici)?;
        let yazildi = dosya.write_all(icerik).and_then(|()| dosya.sync_all());
        drop(dosya);
        if let Err(e) = yazildi.and_then(|()| fs::rename(&gecici, &hedef)) {
            let _ = fs::remove_file(&gecici);
            return Err(e);
        }
        #[cfg(unix)]
        fs::File::open(&self.dizin)?.sync_all()?;
        Ok(())
    }

    /// Saklanan içeriği okur. Okunan içeriğin özeti adla tutmazsa
    /// `InvalidData`: bozuk bir önceki hâl hiçbir zaman geri yazılmaz.
    pub fn oku(&self, ozet: &str) -> io::Result<Vec<u8>> {
        ozet_denetle(ozet)?;
        let veri = fs::read(self.dizin.join(ozet))?;
        if ozet_hex(&veri) != ozet {
            return Err(io::Error::new(io::ErrorKind::InvalidData, BOZUK_ILETISI));
        }
        Ok(veri)
    }

    pub fn var_mi(&self, ozet: &str) -> bool {
        ozet_gecerli_mi(ozet)
            && fs::symlink_metadata(self.dizin.join(ozet)).is_ok_and(|b| b.is_file())
    }

    /// Saklanan içeriklerin toplam boyutu (bayt).
    pub fn boyut(&self) -> io::Result<u64> {
        let mut toplam = 0;
        for girdi in fs::read_dir(&self.dizin)? {
            let girdi = girdi?;
            if girdi.file_name().to_str().is_some_and(ozet_gecerli_mi) {
                toplam += girdi.metadata()?.len();
            }
        }
        Ok(toplam)
    }

    pub fn sil(&self, ozet: &str) -> io::Result<()> {
        ozet_denetle(ozet)?;
        match fs::remove_file(self.dizin.join(ozet)) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    /// Kümede olmayan içerikleri ve yarım kalmış geçici dosyaları siler;
    /// tanınmayan dosyalara dokunmaz. Silinen bayt sayısını döner.
    pub fn kullanilmayanlari_sil(&self, kullanilan: &HashSet<String>) -> io::Result<u64> {
        let mut silinen = 0;
        for girdi in fs::read_dir(&self.dizin)? {
            let girdi = girdi?;
            let Some(ad) = girdi.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let sil = (ozet_gecerli_mi(&ad) && !kullanilan.contains(&ad)) || yarim_mi(&ad);
            if !sil {
                continue;
            }
            let bayt = girdi.metadata()?.len();
            match fs::remove_file(girdi.path()) {
                Ok(()) => silinen += bayt,
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        }
        Ok(silinen)
    }

    /// Deponun bütün içeriğini ve dizinini siler (oturumu silme, §5.10).
    pub fn tumunu_sil(self) -> io::Result<()> {
        match fs::remove_dir_all(&self.dizin) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}
