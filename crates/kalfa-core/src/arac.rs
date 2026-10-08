//! Araç sözleşmesi (spec §6, §7). Yerleşik araç ile eklenti aracı arasında
//! izin açısından fark yoktur: ikisi de bu trait'in arkasındadır.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use kalfa_evren::tipler::ToolDef;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::izin::Kip;
use crate::kabuk::KabukTarifi;
use crate::kimlik::CagriId;
use crate::protokol::{CiktiAkisi, VeriCikisi};
use crate::yol::{CalismaYolu, YolHatasi};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YazmaHedefi {
    pub yol: CalismaYolu,
    /// Dosyanın şu anki içeriğinin SHA-256 özeti; dosya yoksa `None`.
    pub onceki_ozet: Option<String>,
    /// Hard link sayısı; dosya yoksa 1.
    pub baglanti_sayisi: u64,
    /// Dosyanın şu anki içeriği; geri alma için saklanır (aşama 3 tasarımı
    /// §5.3). Dosya yoksa ya da `DOSYA_SINIRI`'ni aşıyorsa `None`. Mühre
    /// girmez: içeriği `onceki_ozet` zaten bağlar.
    pub onceki_icerik: Option<Arc<[u8]>>,
    /// Dosyanın şu anki boyutu (bayt); dosya yoksa 0.
    pub onceki_bayt: u64,
    /// Önceki hâl `DOSYA_SINIRI`'ni aştı: saklanmaz, değişiklik geri alınamaz.
    pub buyuk: bool,
    /// Yazılacak içeriğin SHA-256 özeti. Mühre girmez: içeriği argümanlar
    /// zaten bağlar.
    pub sonraki_ozet: Option<String>,
    /// Yazarken oluşacak üst dizinler, sığdan derine. Geri alma boş kalanları
    /// kaldırır.
    pub olusacak_dizinler: Vec<CalismaYolu>,
}

impl YazmaHedefi {
    /// Geri alma bilgisi taşımayan hedef; yeni alanlar boştur.
    pub fn yeni(yol: CalismaYolu, onceki_ozet: Option<String>, baglanti_sayisi: u64) -> Self {
        Self {
            yol,
            onceki_ozet,
            baglanti_sayisi,
            onceki_icerik: None,
            onceki_bayt: 0,
            buyuk: false,
            sonraki_ozet: None,
            olusacak_dizinler: Vec::new(),
        }
    }

    /// Önceki hâl saklanabilir mi: dosya yoktu ya da içeriği hazırlıkta
    /// verildi. İçeriği vermeyen (örneğin büyük) hedefin değişikliği geri
    /// alınamaz.
    pub fn geri_alinabilir(&self) -> bool {
        !self.buyuk && (self.onceki_ozet.is_none() || self.onceki_icerik.is_some())
    }
}

/// Yürütmeden önce aracın ne yapacağının tarifi: izin kararı, onay kartı ve
/// onay mührü bundan üretilir.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hazirlik {
    pub okunan: Vec<CalismaYolu>,
    /// Okunacak dosyanın bağlantı sayısı; içerik okunmadan alınır ve mühre girer.
    pub okuma_baglanti_sayilari: Vec<(CalismaYolu, u64)>,
    pub yazilan: Vec<YazmaHedefi>,
    /// Hangi yollara dokunacağı önceden bilinemeyen iş (kabuk, aşama 2).
    pub tum_calisma_alani: bool,
    /// Sade Türkçe tek cümle: "`a.txt` dosyasını okumak istiyor."
    pub aciklama: String,
    pub diff: Option<String>,
    /// Diff boyut nedeniyle gösterilmese de görünmez içerik onay kuralı korunur.
    pub gorunmez_icerik: bool,
    /// Kabuk çağrısının tarifi (aşama 2 tasarımı §3.3).
    pub kabuk: Option<KabukTarifi>,
    /// Onay kartında gösterilen veri çıkışı türü (aşama 3b, tasarım §9.3).
    pub veri_cikisi: Option<VeriCikisi>,
    /// Eklenti aracının tarifi (aşama 4 tasarımı §8.1). Mühre girer: onaydan
    /// sonra güncellenen eklentinin çağrısı çalışmaz.
    pub eklenti: Option<EklentiTarifi>,
    /// Araç çağrı boyunca kilit tutmaz; dosyaya kendisi dokunurken o yolun
    /// kilidini kısa süre alır (aşama 4 tasarımı §8.3, karar 5). Uzun bir
    /// eklenti çağrısı aynı çalışma alanındaki komutları bekletmez.
    pub kilitsiz: bool,
}

/// Eklenti aracının kimliği ve yetenekleri (aşama 4 tasarımı §8.1, §8.3).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EklentiTarifi {
    pub ad: String,
    pub surum: String,
    /// `.wasm`'ın SHA-256 özeti.
    pub ozet: String,
    /// Manifestin `dosya-oku` globları.
    pub globlar: Vec<String>,
    /// Manifestin `evren` uçları.
    pub evren_uclari: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AracCiktisi {
    pub icerik: String,
    /// Modele giden içeriğin kaynak etiketi (spec §9), örn. `dosya:src/a.rs:1-40`.
    pub kaynak: String,
    /// Kullanıcıya ayrıca gösterilen güvenlik uyarıları (güvenlik analizi
    /// Bulgu 1): örneğin komutun oluşturduğu tehlikeli `.git`'in karantinası.
    /// Modele de `[güvenlik] …` satırıyla gider.
    pub guvenlik_uyarilari: Vec<String>,
    /// Kabuğun değiştirdiği dosyalar (aşama 3 tasarımı §5.4); yalnızca kabuk
    /// doldurur. Kayda ve tur özetine girer, modele gitmez. Kutulu:
    /// `AracHatasi::Basarisiz` bu tipi taşır.
    pub degisiklikler: Option<Box<DosyaDegisiklikleri>>,
    /// Kalfa'nın kendisi tarafından üretilen sonuç (aşama 3c, tasarım §7.3);
    /// örn. `dosya_yaz`'ın "yazıldı" iletisi. Güvenlik denetçisine gönderilmez.
    pub kalfa_uretimi: bool,
    /// Komutun vekilden geçen ağ özeti (aşama 3d, tasarım §8.2); yalnızca kabuk
    /// doldurur. Tur özetine girer.
    pub ag: Option<Box<crate::tur_ozeti::AgKomutOzeti>>,
}

/// Bir araç çıktısının denetim durumunu belirtir (tasarım §7.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DenetimGereksinimi {
    /// Dış kaynaklı içerik; verilen girdiyle denetçiye gönderilmeli.
    Denetlenecek(crate::denetci::DenetimGirdisi),
    /// Kalfa'nın kendisinin ürettiği çıktı (dosya yazma/düzenleme başarısı, hata, izin ret vb.); denetlenmez.
    KalfaUretimi,
    /// Kullanıcı onayıyla okunan gizli dosya; sırrı dışarı çıkarmamak için denetlenmez.
    GizliDosya,
}

/// Metni UTF-8 sınırlarına dikkat ederek azami bayt sınırına kırpar.
pub fn kirp(metin: &str, azami: usize) -> String {
    if metin.len() <= azami {
        return metin.to_string();
    }
    let mut kes = azami;
    while !metin.is_char_boundary(kes) {
        kes -= 1;
    }
    format!(
        "{}\n[çıktı {} bayttan {} bayta kırpıldı]",
        &metin[..kes],
        metin.len(),
        kes
    )
}

/// Araç çıktısının güvenlik denetimi gereksinimini ve denetlenecek kırpılmış metnini belirler.
pub fn denetim_durumu(
    c: &AracCiktisi,
    uyarilar: &[String],
    azami_bayt: usize,
) -> DenetimGereksinimi {
    if c.kalfa_uretimi {
        return DenetimGereksinimi::KalfaUretimi;
    }
    if uyarilar.iter().any(|u| u == crate::izin::GIZLI_UYARISI) {
        return DenetimGereksinimi::GizliDosya;
    }
    let kirpilmis = kirp(&c.icerik, azami_bayt);
    DenetimGereksinimi::Denetlenecek(crate::denetci::DenetimGirdisi {
        kaynak: c.kaynak.clone(),
        icerik: kirpilmis,
    })
}

/// Tek tek listelenen değişikliklerin sınırı; ötekiler dizin sayılarında.
pub const DEGISIKLIK_LISTE_SINIRI: usize = 200;
/// Listede sona konan üst düzey dizinler: bir derleme bunların altında
/// binlerce dosya değiştirir.
pub const BAGIMLILIK_DIZINLERI: &[&str] = &[
    "target",
    "node_modules",
    ".venv",
    "venv",
    "__pycache__",
    "dist",
    "build",
    ".next",
    ".cache",
    ".gradle",
    "vendor",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DegisiklikTuru {
    Eklendi,
    Degisti,
    Silindi,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DegisenDosya {
    /// Çalışma alanına göre yol, `/` ayraçlı.
    pub yol: String,
    pub tur: DegisiklikTuru,
}

/// Bir üst düzey dizinin altındaki değişikliklerin sayıları.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DizinSayisi {
    pub dizin: String,
    pub eklenen: u32,
    pub degisen: u32,
    pub silinen: u32,
}

/// Bir komutun çalışırken değişen dosyaları. İçerik kopyalanmaz; komuttan
/// önceki ve sonraki meta veri görüntülerinin farkıdır. Aynı anda Kalfa
/// dışında yapılan değişiklikler de burada görünür.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DosyaDegisiklikleri {
    /// En çok `DEGISIKLIK_LISTE_SINIRI` dosya; bağımlılık dizinlerinin
    /// dışındakiler önce.
    pub dosyalar: Vec<DegisenDosya>,
    /// Üst düzey dizinlerin sayıları.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dizinler: Vec<DizinSayisi>,
    /// Bütün değişikliklerin sayısı; `dosyalar`'dan büyükse liste kesildi.
    pub toplam: u32,
    /// Tarama sınırı aşıldı; liste eksik olabilir.
    #[serde(default)]
    pub eksik: bool,
}

impl DosyaDegisiklikleri {
    pub fn olustur(
        degisiklikler: impl IntoIterator<Item = (String, DegisiklikTuru)>,
        eksik: bool,
    ) -> Self {
        let mut hepsi: Vec<(String, DegisiklikTuru)> = degisiklikler.into_iter().collect();
        let ust = |yol: &str| yol.split_once('/').map(|(d, _)| d.to_string());
        let bagimlilik =
            |yol: &str| ust(yol).is_some_and(|d| BAGIMLILIK_DIZINLERI.contains(&d.as_str()));
        hepsi.sort_by(|a, b| (bagimlilik(&a.0), &a.0).cmp(&(bagimlilik(&b.0), &b.0)));
        let mut dizinler: Vec<DizinSayisi> = Vec::new();
        for (yol, tur) in &hepsi {
            let Some(dizin) = ust(yol) else {
                continue;
            };
            let sayi = match dizinler.iter_mut().find(|d| d.dizin == dizin) {
                Some(s) => s,
                None => {
                    dizinler.push(DizinSayisi {
                        dizin,
                        eklenen: 0,
                        degisen: 0,
                        silinen: 0,
                    });
                    let son = dizinler.len() - 1;
                    &mut dizinler[son]
                }
            };
            let alan = match tur {
                DegisiklikTuru::Eklendi => &mut sayi.eklenen,
                DegisiklikTuru::Degisti => &mut sayi.degisen,
                DegisiklikTuru::Silindi => &mut sayi.silinen,
            };
            *alan = alan.saturating_add(1);
        }
        dizinler.sort_by(|a, b| a.dizin.cmp(&b.dizin));
        let toplam = u32::try_from(hepsi.len()).unwrap_or(u32::MAX);
        hepsi.truncate(DEGISIKLIK_LISTE_SINIRI);
        Self {
            dosyalar: hepsi
                .into_iter()
                .map(|(yol, tur)| DegisenDosya { yol, tur })
                .collect(),
            dizinler,
            toplam,
            eksik,
        }
    }

    /// Değişen her dosya listede ve tarama tamam.
    pub fn tam_mi(&self) -> bool {
        !self.eksik && self.dosyalar.len() as u64 == u64::from(self.toplam)
    }

    pub fn bos_mu(&self) -> bool {
        self.toplam == 0 && !self.eksik
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AracHatasi {
    #[error("{0}")]
    Gecersiz(String),
    #[error(transparent)]
    Yol(#[from] YolHatasi),
    #[error("Dosya onaydan sonra değişti; işlem yapılmadı.")]
    OnayEskidi,
    /// Kabuk tarifi onaydan sonra değişti; ileti nedeni söyler.
    #[error("{0}; komut çalıştırılmadı. Gerekirse yeniden dene.")]
    KomutKosuluDegisti(String),
    #[error(
        "İzin kipi veya hedefin izin koşulları değişti; işlem yapılmadı. Gerekirse yeniden dene."
    )]
    PolitikaDegisti,
    #[error("İptal edildi.")]
    Iptal,
    #[error("{0}")]
    GirisCikis(String),
    /// Araç çalıştı ama başarısız oldu; çıktı yine de modele gider. Kabukta
    /// sıfırdan farklı çıkış kodu ve zaman aşımı böyledir. Aynı çağrının
    /// üst üste başarısızlık sınırına sayılmaz (aşama 2 tasarımı §4.5):
    /// düzeltmeden sonra aynı testi yeniden koşmak olağandır.
    #[error("{}", .0.icerik)]
    Basarisiz(AracCiktisi),
}

/// Süren bir aracın ara çıktı parçası.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiktiParcasi {
    pub cagri: CagriId,
    pub akis: CiktiAkisi,
    pub metin: String,
    /// Bu parçadan önce atılan baytların toplamı.
    pub atlanan_bayt: u64,
}

/// Aracın ara çıktısını oturuma taşır (aşama 2 tasarımı §4.4). Göndermek
/// hiç beklemez: kanal doluysa parça atılır ve atılan bayt sayısı sonraki
/// parçaya yazılır. Aracın modele verdiği sonuç bundan etkilenmez.
#[derive(Debug, Clone)]
pub struct CiktiGonderici {
    ic: Option<(CagriId, mpsc::Sender<CiktiParcasi>)>,
    atlanan: Arc<AtomicU64>,
}

impl CiktiGonderici {
    /// Hiçbir yere göndermeyen gönderici.
    pub fn bos() -> Self {
        Self {
            ic: None,
            atlanan: Arc::default(),
        }
    }

    /// `cagri`'nın parçalarını `tx`'e gönderen gönderici.
    pub fn yeni(cagri: CagriId, tx: mpsc::Sender<CiktiParcasi>) -> Self {
        Self {
            ic: Some((cagri, tx)),
            atlanan: Arc::default(),
        }
    }

    /// Oturumun, araç bittikten sonra atılan baytları okuması için.
    pub(crate) fn atlanan_sayaci(&self) -> Arc<AtomicU64> {
        self.atlanan.clone()
    }

    pub fn gonder(&self, akis: CiktiAkisi, metin: String) {
        let Some((cagri, tx)) = &self.ic else {
            return;
        };
        let atlanan = self.atlanan.swap(0, Ordering::SeqCst);
        let bayt = metin.len() as u64;
        let parca = CiktiParcasi {
            cagri: *cagri,
            akis,
            metin,
            atlanan_bayt: atlanan,
        };
        if tx.try_send(parca).is_err() {
            self.atlanan.fetch_add(atlanan + bayt, Ordering::SeqCst);
        }
    }
}

impl From<std::io::Error> for AracHatasi {
    fn from(hata: std::io::Error) -> Self {
        AracHatasi::GirisCikis(hata.to_string())
    }
}

#[async_trait]
pub trait Arac: Send + Sync {
    fn ad(&self) -> &str;
    fn aciklama(&self) -> &str;
    fn parametre_semasi(&self) -> Value;
    /// Dosya yazar ya da yan etkisi olan bir iş yapar mı? İnceleme kipinde
    /// yazan araçlar modele hiç sunulmaz (spec §7.5).
    fn yazar_mi(&self) -> bool;
    /// Araç şu an modele sunulur mu? Her turda sorulur: sunulmayan araç
    /// tanımlanmaz ve çağrılırsa çalışmaz. Örneğin yalıtım yokken ve
    /// yalıtımsız çalıştırma kapalıyken kabuk (aşama 2 tasarımı §9.8).
    fn sunulur_mu(&self) -> bool {
        true
    }
    async fn hazirla(&self, argumanlar: &Value) -> Result<Hazirlik, AracHatasi>;
    async fn calistir(
        &self,
        argumanlar: &Value,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi>;

    /// Ara çıktı üreten araçlar bunu uygular; oturum araçları bununla
    /// çalıştırır. Varsayılanı `calistir`'dır ve ara çıktı üretmez.
    async fn calistir_akisli(
        &self,
        argumanlar: &Value,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
        _cikti: CiktiGonderici,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.calistir(argumanlar, hazirlik, iptal).await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KayitHatasi {
    #[error("`{0}` adlı araç iki kez kaydedildi.")]
    Yinelenen(String),
    #[error("`{arac}` aracının parametre şeması geçersiz: {hata}")]
    GecersizSema { arac: String, hata: String },
}

pub struct AracKayitDefteri {
    araclar: Vec<Arc<dyn Arac>>,
    semalar: HashMap<String, jsonschema::Validator>,
}

impl AracKayitDefteri {
    pub fn yeni(araclar: Vec<Arc<dyn Arac>>) -> Result<Self, KayitHatasi> {
        let mut semalar = HashMap::new();
        for arac in &araclar {
            if semalar.contains_key(arac.ad()) {
                return Err(KayitHatasi::Yinelenen(arac.ad().to_string()));
            }
            let dogrulayici = jsonschema::validator_for(&arac.parametre_semasi()).map_err(|e| {
                KayitHatasi::GecersizSema {
                    arac: arac.ad().to_string(),
                    hata: e.to_string(),
                }
            })?;
            semalar.insert(arac.ad().to_string(), dogrulayici);
        }
        Ok(Self { araclar, semalar })
    }

    pub fn bul(&self, ad: &str) -> Option<Arc<dyn Arac>> {
        self.araclar.iter().find(|a| a.ad() == ad).cloned()
    }

    pub fn tanimlar(&self, kip: Kip) -> Vec<ToolDef> {
        self.araclar
            .iter()
            .filter(|a| a.sunulur_mu() && (kip != Kip::Inceleme || !a.yazar_mi()))
            .map(|a| ToolDef::function(a.ad(), a.aciklama(), a.parametre_semasi()))
            .collect()
    }

    /// Argüman metnini JSON olarak çözer ve aracın şemasına karşı doğrular.
    /// Hata metni modele gider; model argümanı düzeltip yeniden deneyebilir.
    pub fn dogrula(&self, ad: &str, argumanlar: &str) -> Result<Value, String> {
        let dogrulayici = self
            .semalar
            .get(ad)
            .ok_or_else(|| format!("`{ad}` adlı bir araç yok."))?;
        let metin = if argumanlar.trim().is_empty() {
            "{}"
        } else {
            argumanlar
        };
        let deger: Value = serde_json::from_str(metin)
            .map_err(|e| format!("Argümanlar geçerli JSON değil: {e}"))?;
        let hatalar: Vec<String> = dogrulayici
            .iter_errors(&deger)
            .map(|e| format!("{}: {}", e.instance_path(), e))
            .collect();
        if hatalar.is_empty() {
            Ok(deger)
        } else {
            Err(format!("Argümanlar şemaya uymuyor: {}", hatalar.join("; ")))
        }
    }
}
