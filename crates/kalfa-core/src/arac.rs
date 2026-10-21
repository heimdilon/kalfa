//! Araç sözleşmesi (spec §6, §7). Yerleşik araç ile eklenti aracı arasında
//! izin açısından fark yoktur: ikisi de bu trait'in arkasındadır.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub use async_trait::async_trait;
use kalfa_evren::YanitMeta;
use kalfa_evren::tipler::ToolDef;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::butce::{ButceIslemId, ButceTalepci};
use crate::gonderim::{EklentiIstekOzeti, GunlukToken, IstekSonucu};
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
    /// MCP aracının güvenli kart özeti ve R/D mühür girdileri. Ham komut,
    /// ortam değişkeni, sır ve dosya yolu bu alana konmaz.
    pub mcp: Option<McpAracHazirligi>,
    /// Web getirme aracının (`web_getir`) hedef, yöntem ve yönlendirme tarifi.
    /// Mühre girer: onaydan sonra hedef URL, host, port veya yönlendirme kapsamı
    /// değişirse çağrı çalışmaz.
    pub web: Option<WebTarifi>,
    /// Araç çağrı boyunca kilit tutmaz; dosyaya kendisi dokunurken o yolun
    /// kilidini kısa süre alır (aşama 4 tasarımı §8.3, karar 5). Uzun bir
    /// eklenti çağrısı aynı çalışma alanındaki komutları bekletmez.
    pub kilitsiz: bool,
}

/// Yönlendirme sırasında hangi hedef hostlara geçiş yapılabileceğini belirler.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip", content = "hostlar")]
pub enum YonlendirmeIzni {
    /// Yalnızca ilk istekteki aynı hosta yönlendirmeye izin verilir (varsayılan).
    #[default]
    AyniHost,
    /// İlk hosta ek olarak açıkça onaylanmış hedef host listesine izin verilir.
    IzinliHostlar(Vec<String>),
}

impl YonlendirmeIzni {
    /// Verilen `hedef_host`, `baslangic_host` veya onaylı listede mi?
    pub fn host_izinli_mi(&self, baslangic_host: &str, hedef_host: &str) -> bool {
        let baslangic = baslangic_host.trim().to_ascii_lowercase();
        let hedef = hedef_host.trim().to_ascii_lowercase();
        if baslangic == hedef {
            return true;
        }
        match self {
            Self::AyniHost => false,
            Self::IzinliHostlar(liste) => liste
                .iter()
                .any(|izinli| izinli.trim().eq_ignore_ascii_case(&hedef)),
        }
    }
}

/// Bir `web_getir` çağrısının onay ve mühürlemeye esas tarifi.
/// Ham sorgu parametrelerini veya sırları serileştirmez; yalnız maskelenmiş URL
/// sunumunu ve doğrulanmış hedef bilgilerini taşır.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebTarifi {
    /// Günlük ve onay penceresi için hassas sorgu parametreleri maskelenmiş URL.
    pub maskeli_url: String,
    /// Küçük harfe normalize edilmiş hedef host adı.
    pub host: String,
    /// Etkin hedef port (ör. HTTPS için 443, HTTP için 80).
    pub port: u16,
    /// URL şeması (`"https"` veya `"http"`).
    pub sema: String,
    /// HTTP yöntemi (`"GET"`).
    pub yontem: String,
    /// Yönlendirme kapsamı.
    pub yonlendirme_izni: YonlendirmeIzni,
    /// Bu `(host, port)` çiftine mevcut oturumda ilk kez mi çıkılıyor?
    #[serde(default)]
    pub oturumda_ilk_host: bool,
    /// Ham kanonik URL'nin tek yönlü SHA-256 özeti.
    /// Ham sorgu sırrını taşımadan mührü tam hedefe bağlar; onay DTO'sunda
    /// serileştirilmez.
    #[serde(skip)]
    pub hedef_ozeti: [u8; 32],
}

impl WebTarifi {
    /// Onay kartı / IPC DTO'su için iç mühür özetini sıfırlayarak güvenli kopya üretir.
    pub fn onay_karti_icin(&self) -> Self {
        Self {
            hedef_ozeti: [0; 32],
            ..self.clone()
        }
    }
}

/// Ham kanonik URL için alan sınırları ayrılmış SHA-256 hedef özeti üretir.
pub fn web_hedef_ozeti(kanonik_url: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};

    let mut ozetleyici = Sha256::new();
    let etiket = b"kalfa-web-hedef-url-v1";
    ozetleyici.update((etiket.len() as u64).to_le_bytes());
    ozetleyici.update(etiket);
    ozetleyici.update((kanonik_url.len() as u64).to_le_bytes());
    ozetleyici.update(kanonik_url.as_bytes());
    ozetleyici.finalize().into()
}

/// [`WebTarifi`] için tek yönlü, alan sınırları ayrılmış SHA-256 mühür özeti üretir.
pub fn web_muhur_ozeti(tarif: &WebTarifi) -> [u8; 32] {
    use sha2::{Digest, Sha256};

    fn uzunluklu_yaz(ozetleyici: &mut Sha256, alan: &[u8]) {
        ozetleyici.update((alan.len() as u64).to_le_bytes());
        ozetleyici.update(alan);
    }

    let mut ozetleyici = Sha256::new();
    uzunluklu_yaz(&mut ozetleyici, b"kalfa-web-muhur-v1");
    uzunluklu_yaz(&mut ozetleyici, tarif.yontem.as_bytes());
    uzunluklu_yaz(&mut ozetleyici, tarif.sema.as_bytes());
    uzunluklu_yaz(&mut ozetleyici, tarif.host.as_bytes());
    ozetleyici.update(tarif.port.to_le_bytes());
    uzunluklu_yaz(&mut ozetleyici, tarif.maskeli_url.as_bytes());
    uzunluklu_yaz(&mut ozetleyici, &tarif.hedef_ozeti);
    match &tarif.yonlendirme_izni {
        YonlendirmeIzni::AyniHost => {
            uzunluklu_yaz(&mut ozetleyici, b"ayni-host");
        }
        YonlendirmeIzni::IzinliHostlar(hostlar) => {
            uzunluklu_yaz(&mut ozetleyici, b"izinli-hostlar");
            let mut sirali: Vec<String> = hostlar
                .iter()
                .map(|h| h.trim().to_ascii_lowercase())
                .collect();
            sirali.sort();
            sirali.dedup();
            ozetleyici.update((sirali.len() as u64).to_le_bytes());
            for h in &sirali {
                uzunluklu_yaz(&mut ozetleyici, h.as_bytes());
            }
        }
    }
    ozetleyici.finalize().into()
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
    /// Eklentinin `evren` yeteneği var ve bu çalışma alanında henüz gönderim
    /// izni yok: çağrı her kipte yerel onay ister (aşama 4 tasarımı §8.4).
    #[serde(default)]
    pub ilk_gonderim: bool,
    /// İlk gönderim kartı için: manifest globlarıyla okunabilecek dosya
    /// sayısı ve toplam boyutu. Yalnızca `ilk_gonderim`'de hesaplanır; bilgi
    /// amaçlıdır, mühre girmez.
    #[serde(default)]
    pub dosya_sayisi: u64,
    #[serde(default)]
    pub bayt: u64,
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
    #[error("Araç çağrısı güvenli biçimde tamamlanamadı; çalışma alanı kullanılamıyor.")]
    SonucIsleme,
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

/// Eklentinin bir EVREN isteği için oturumdan istediği izin (aşama 4
/// tasarımı §8.7, §9.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GonderimTalebi {
    pub ozet: EklentiIstekOzeti,
    /// İsteğin gövdesi. Kayda yazılmaz; önizleme açıkken "Metni göster" için
    /// oturum bellekte tutar.
    pub govde: Arc<str>,
}

/// Oturumun gönderim talebine cevabı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GonderimYaniti {
    Gonder,
    /// Kullanıcı önizlemede "Gönderme" dedi. Kaydı oturum yazdı.
    Gonderme,
    /// Oturum başına eklenti sınırı doldu (§9.5). Kaydı oturum yazdı.
    SinirAsildi(String),
    /// Tur iptal edildi ya da oturum kapandı.
    Iptal,
}

/// Aracın oturuma ilettiği istekler. Oturumun araç aşaması bunları araç
/// çalışırken işler.
#[derive(Debug)]
pub enum OrtamIstegi {
    Gonderim {
        cagri: CagriId,
        talep: GonderimTalebi,
        yanit: oneshot::Sender<GonderimYaniti>,
    },
    /// `Gonder` cevabından sonra ağa çıkan isteğin sonucu; oturum kayda
    /// yazar (`KayitOgesi::EklentiIstegi`). Bütçe izni verildiyse `butce_islem`
    /// ve bilinen `kullanim` ile bitirilir (5b tasarımı §6).
    IstekBitti {
        cagri: CagriId,
        ozet: EklentiIstekOzeti,
        meta: YanitMeta,
        sonuc: IstekSonucu,
        butce_islem: Option<ButceIslemId>,
        kullanim: Option<GunlukToken>,
    },
    /// Oturumun çalışma planını günceller (`plan_guncelle`).
    PlanGuncelle {
        cagri: CagriId,
        adimlar: Vec<crate::plan::PlanAdimi>,
        yanit: oneshot::Sender<Result<crate::plan::OturumPlani, String>>,
    },
    /// Kullanıcıya çoktan seçmeli soru sorar ve yanıtını bekler (`kullaniciya_sor`).
    KullaniciSorusu {
        cagri: CagriId,
        baslik: String,
        secenekler: Vec<crate::soru::SoruSecenegi>,
        serbest_yanit_izni: bool,
        yanit: oneshot::Sender<Result<crate::soru::SoruSonucu, String>>,
    },
}

/// Aracın oturumla konuştuğu kanal (aşama 4 tasarımı §8.7): canlı çıktı,
/// EVREN isteğinin izni, bütçe izni ve kaydı. Yerleşik araçlar yalnızca çıktıyı
/// kullanır.
#[derive(Debug, Clone)]
pub struct AracOrtami {
    cikti: CiktiGonderici,
    /// Oturumlu ortamın çağrı kimliği, kanalı ve bütçe talepçisi. Talepçi
    /// zorunludur: ürün oturumunda bütçe sessizce atlanamaz.
    ic: Option<(CagriId, mpsc::Sender<OrtamIstegi>, ButceTalepci)>,
}

impl AracOrtami {
    /// Oturumsuz ortam (testler, oturum dışı çağrı): gönderim talebine
    /// `Gonder` döner, kayıt atılır.
    pub fn bos() -> Self {
        Self {
            cikti: CiktiGonderici::bos(),
            ic: None,
        }
    }

    /// Yalnızca canlı çıktısı olan ortam.
    pub fn ciktiyla(cikti: CiktiGonderici) -> Self {
        Self { cikti, ic: None }
    }

    pub fn yeni(
        cikti: CiktiGonderici,
        cagri: CagriId,
        tx: mpsc::Sender<OrtamIstegi>,
        butce: ButceTalepci,
    ) -> Self {
        Self {
            cikti,
            ic: Some((cagri, tx, butce)),
        }
    }

    pub fn cikti(&self) -> CiktiGonderici {
        self.cikti.clone()
    }

    /// MCP çağrı kaydının model argümanlarından bağımsız kimliği.
    pub fn cagri_kimligi(&self) -> Option<CagriId> {
        self.ic.as_ref().map(|(cagri, _, _)| *cagri)
    }

    /// Talebi oturuma iletir ve cevabı bekler. Oturum kapandıysa `Iptal`.
    pub async fn gonderim_izni(&self, talep: GonderimTalebi) -> GonderimYaniti {
        let Some((cagri, tx, _)) = &self.ic else {
            return GonderimYaniti::Gonder;
        };
        let (yanit, alici) = oneshot::channel();
        let istek = OrtamIstegi::Gonderim {
            cagri: *cagri,
            talep,
            yanit,
        };
        if tx.send(istek).await.is_err() {
            return GonderimYaniti::Iptal;
        }
        alici.await.unwrap_or(GonderimYaniti::Iptal)
    }

    /// Ağdan hemen önce bütçe izni (5b tasarımı §4.1). Oturumsuz ortamda bütçe
    /// yoktur (`Ok(None)`). Oturumlu ortamda izin verilirse `Ok(Some(islem))`;
    /// izin yoksa `Err(())` ve çağıran ağa çıkmaz.
    // Hata nedeni yoktur: çağıran için tek bilgi "ağa çıkılmaz"dır (5b §4.1).
    #[allow(clippy::result_unit_err)]
    pub async fn butce_izni(&self, amac: &str) -> Result<Option<ButceIslemId>, ()> {
        let Some((_, _, butce)) = &self.ic else {
            return Ok(None);
        };
        butce.izin(amac).await.map(Some).ok_or(())
    }

    /// Ağa çıkan isteğin sonucunu ve varsa bütçe bitişini oturuma bildirir.
    pub async fn istek_bitti(
        &self,
        ozet: EklentiIstekOzeti,
        meta: YanitMeta,
        sonuc: IstekSonucu,
        butce_islem: Option<ButceIslemId>,
        kullanim: Option<GunlukToken>,
    ) {
        let Some((cagri, tx, _)) = &self.ic else {
            return;
        };
        let _ = tx
            .send(OrtamIstegi::IstekBitti {
                cagri: *cagri,
                ozet,
                meta,
                sonuc,
                butce_islem,
                kullanim,
            })
            .await;
    }

    /// Oturumun çalışma planını günceller ve doğrulanan planı döner.
    pub async fn plan_guncelle(
        &self,
        adimlar: Vec<crate::plan::PlanAdimi>,
        iptal: CancellationToken,
    ) -> Result<crate::plan::OturumPlani, AracHatasi> {
        let Some((cagri, tx, _)) = &self.ic else {
            return Err(AracHatasi::Gecersiz(
                "plan_guncelle yalnız oturum ortamında çalıştırılabilir.".to_string(),
            ));
        };
        let (yanit, alici) = oneshot::channel();
        tx.send(OrtamIstegi::PlanGuncelle {
            cagri: *cagri,
            adimlar,
            yanit,
        })
        .await
        .map_err(|_| AracHatasi::Iptal)?;
        tokio::select! {
            _ = iptal.cancelled() => Err(AracHatasi::Iptal),
            gelen = alici => {
                gelen
                    .map_err(|_| AracHatasi::Iptal)?
                    .map_err(AracHatasi::Gecersiz)
            }
        }
    }

    /// Kullanıcıya soru sorar ve yanıt gelene (ya da iptal/yönlendirme olana) kadar bekler.
    pub async fn kullaniciya_sor(
        &self,
        baslik: String,
        secenekler: Vec<crate::soru::SoruSecenegi>,
        serbest_yanit_izni: bool,
        iptal: CancellationToken,
    ) -> Result<crate::soru::SoruSonucu, AracHatasi> {
        let Some((cagri, tx, _)) = &self.ic else {
            return Err(AracHatasi::Gecersiz(
                "kullaniciya_sor yalnız oturum ortamında çalıştırılabilir.".to_string(),
            ));
        };
        let (yanit, alici) = oneshot::channel();
        tx.send(OrtamIstegi::KullaniciSorusu {
            cagri: *cagri,
            baslik,
            secenekler,
            serbest_yanit_izni,
            yanit,
        })
        .await
        .map_err(|_| AracHatasi::Iptal)?;
        tokio::select! {
            _ = iptal.cancelled() => Err(AracHatasi::Iptal),
            gelen = alici => {
                gelen
                    .map_err(|_| AracHatasi::Iptal)?
                    .map_err(AracHatasi::Gecersiz)
            }
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

    /// Oturumla konuşan araçlar (eklenti) bunu uygular: EVREN isteğinin izni
    /// ve kaydı `AracOrtami`'yle gider (aşama 4 tasarımı §8.7). Oturum
    /// araçları bununla çalıştırır; varsayılanı `calistir_akisli`'dir.
    async fn calistir_ortamli(
        &self,
        argumanlar: &Value,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
        ortam: AracOrtami,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.calistir_akisli(argumanlar, hazirlik, iptal, ortam.cikti())
            .await
    }

    /// Kilidi çağrı boyunca varsayılan olarak bu future tutar. MCP sarmalayıcısı
    /// aynı sahipli kilidi ilk RPC await'inden önce yönetici çağrı kaydına
    /// devretmek için bu metodu override eder; core MCP türlerini tanımaz.
    async fn calistir_kilitli(
        &self,
        argumanlar: &Value,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
        ortam: AracOrtami,
        kilit: Option<crate::kilit::KilitSeti>,
    ) -> Result<AracCiktisi, AracHatasi> {
        let _kilit = kilit;
        self.calistir_ortamli(argumanlar, hazirlik, iptal, ortam)
            .await
    }

    /// MCP, ilk kilitli hazırlık await'inden önce guard'ı yöneticiye taşır.
    /// Öteki araçlar aynı guard'ı normal yürütme future'ına bırakır.
    fn kilidi_devral(
        &self,
        _cagri: CagriId,
        _hazirlik: &Hazirlik,
        _iptal: CancellationToken,
        kilit: Option<crate::kilit::KilitSeti>,
    ) -> Result<Option<crate::kilit::KilitSeti>, AracHatasi> {
        Ok(kilit)
    }

    /// Core, araç sonucunu denetleyip kaydettikten sonra çağrıyı kapatır.
    /// MCP yöneticisi burada çalışma alanının son koruma denetimini yapar ve
    /// yalnız doğrulanmış kapanışta çağrı kaydını/kilidini çözer. Hata
    /// yöneticideki kaydı ve kilidi korumalıdır.
    async fn sonuc_isleme_bitti(&self, _cagri: CagriId) -> Result<(), AracHatasi> {
        Ok(())
    }

    /// Sonuç denetimi tamamlanmadan çağrı future'ı bırakılırsa yönetici
    /// bağımsız cleanup başlatır. Cleanup başarılı olmadan kaydı/kilidi çözmez.
    fn sonuc_isleme_iptal(&self, _cagri: CagriId) {}

    /// Onay eskidiği için araç yeniden hazırlanacaksa cleanup'ın bitmesini
    /// bekler. Yönetici bu metodu cleanup ve çağrı kilidi bırakma tamamlanınca
    /// dönecek şekilde uygular. Varsayılan yalnızca iptal kancasını çağırıp
    /// başarısız döner; cleanup tamamlandığı kanıtlanmadan taze onaya geçilmez.
    async fn sonuc_isleme_iptal_ve_bekle(&self, cagri: CagriId) -> Result<(), AracHatasi> {
        self.sonuc_isleme_iptal(cagri);
        Err(AracHatasi::SonucIsleme)
    }

    /// Günlük (JSONL), geçmiş görünümü, bağlam kaydı ve şema doğrulama hataları
    /// için ham JSON argüman metnindeki hassas alanları (ör. `web_getir` URL
    /// sorgu parametreleri) maskeler. İç hazırlık, mühür ve yürütme ham
    /// argümanları kullanmaya devam eder.
    fn kayit_argumanlarini_maskele(&self, argumanlar: &str) -> String {
        argumanlar.to_string()
    }
}

/// Core'un sonuç işlemesi bitene kadar yönetici çağrı kaydını açık tutar.
/// Düşürülürse yöneticiye eşzamanlı iptal/cleanup bildirimi gider.
pub struct AracTamamlama {
    arac: Arc<dyn Arac>,
    cagri: CagriId,
    tamamlandi: bool,
}

impl AracTamamlama {
    pub fn yeni(arac: Arc<dyn Arac>, cagri: CagriId) -> Self {
        Self {
            arac,
            cagri,
            tamamlandi: false,
        }
    }

    /// Başarıda yönetici kaydı kapatılır. Hata ayrıntısı atılır; core yalnız
    /// güvenli ve sabit bir hata üretir. Bekleyen future bırakılırsa `Drop`
    /// iptal yolunu çağırır; tamamlanma hatasında yönetici kaydı korunur.
    pub async fn tamamla(mut self) -> Result<(), AracHatasi> {
        let sonuc = self.arac.sonuc_isleme_bitti(self.cagri).await;
        self.tamamlandi = true;
        sonuc.map_err(|_| AracHatasi::SonucIsleme)
    }

    /// Bu çağrıyı sonuç olarak kapatmadan cleanup yapar ve bittiğini bekler.
    /// Future bırakılırsa ikinci bir cleanup kancası çağırmamak için sahiplik
    /// önce tüketilir; yönetici kendi cleanup görevini sürdürür.
    pub async fn iptal_ve_bekle(mut self) -> Result<(), AracHatasi> {
        self.tamamlandi = true;
        self.arac
            .sonuc_isleme_iptal_ve_bekle(self.cagri)
            .await
            .map_err(|_| AracHatasi::SonucIsleme)
    }
}

impl std::fmt::Debug for AracTamamlama {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AracTamamlama")
            .field("cagri", &self.cagri)
            .field("tamamlandi", &self.tamamlandi)
            .finish()
    }
}

impl Drop for AracTamamlama {
    fn drop(&mut self) {
        if !self.tamamlandi {
            self.arac.sonuc_isleme_iptal(self.cagri);
        }
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
    semalar: HashMap<String, Arc<jsonschema::Validator>>,
    kaynak: Option<Arc<dyn AracKaynagi>>,
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
            semalar.insert(arac.ad().to_string(), Arc::new(dogrulayici));
        }
        Ok(Self {
            araclar,
            semalar,
            kaynak: None,
        })
    }

    /// Sabit araçlara ek olarak her model turunun başında `kaynak`'ın hazır
    /// yayımını da görüntüye alan defter (MCP; Görev 8).
    pub fn yeni_kaynakli(
        araclar: Vec<Arc<dyn Arac>>,
        kaynak: Arc<dyn AracKaynagi>,
    ) -> Result<Self, KayitHatasi> {
        Ok(Self {
            kaynak: Some(kaynak),
            ..Self::yeni(araclar)?
        })
    }

    pub fn kaynak(&self) -> Option<&Arc<dyn AracKaynagi>> {
        self.kaynak.as_ref()
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
        match semaya_gore_dogrula(ad, self.semalar.get(ad).map(Arc::as_ref), argumanlar) {
            Ok(deger) => Ok(deger),
            Err(hata) => {
                if let Some(arac) = self.bul(ad) {
                    let maskeli = arac.kayit_argumanlarini_maskele(argumanlar);
                    if maskeli != argumanlar {
                        return Err(semaya_gore_dogrula(
                            ad,
                            self.semalar.get(ad).map(Arc::as_ref),
                            &maskeli,
                        )
                        .err()
                        .unwrap_or(hata));
                    }
                }
                Err(hata)
            }
        }
    }

    /// Bir model turunun tek araç görüntüsü. Kaynak yalnız bir kez ve
    /// beklemeden okunur; hazır olmayan keşif turu bekletmez. İnceleme
    /// kipinde kaynak araçları görüntüye hiç girmez.
    pub fn anlik_goruntu(&self, kip: Kip) -> AracAnlikGoruntusu {
        let mut tanimlar = self.tanimlar(kip);
        let mut araclar: HashMap<String, (Arc<dyn Arac>, Arc<jsonschema::Validator>)> = self
            .araclar
            .iter()
            .filter_map(|a| {
                let sema = self.semalar.get(a.ad())?.clone();
                Some((a.ad().to_string(), (a.clone(), sema)))
            })
            .collect();
        if let Some(kaynak) = self.kaynak.as_ref().filter(|_| kip != Kip::Inceleme) {
            for girdi in kaynak.hazir_araclar().iter() {
                // Sabit araç ya da önceki kaynak girdisi aynı adı tutuyorsa
                // ikincisi görüntüye girmez.
                if araclar.contains_key(girdi.ad()) {
                    continue;
                }
                if girdi.arac.sunulur_mu() {
                    tanimlar.push(girdi.tanim.clone());
                }
                araclar.insert(
                    girdi.ad().to_string(),
                    (girdi.arac.clone(), girdi.sema.clone()),
                );
            }
        }
        AracAnlikGoruntusu { tanimlar, araclar }
    }
}

fn semaya_gore_dogrula(
    ad: &str,
    dogrulayici: Option<&jsonschema::Validator>,
    argumanlar: &str,
) -> Result<Value, String> {
    let dogrulayici = dogrulayici.ok_or_else(|| format!("`{ad}` adlı bir araç yok."))?;
    let metin = if argumanlar.trim().is_empty() {
        "{}"
    } else {
        argumanlar
    };
    let deger: Value =
        serde_json::from_str(metin).map_err(|e| format!("Argümanlar geçerli JSON değil: {e}"))?;
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

/// Bir model turunun araç görüntüsü (Görev 8). Sağlayıcı isteği, gönderim
/// günlüğü, ad çözümlemesi ve argüman doğrulaması aynı görüntüyü kullanır;
/// tur sürerken kaynakta yayımlanan yeni sürüm bir sonraki model turunda görünür.
pub struct AracAnlikGoruntusu {
    tanimlar: Vec<ToolDef>,
    araclar: HashMap<String, (Arc<dyn Arac>, Arc<jsonschema::Validator>)>,
}

impl AracAnlikGoruntusu {
    pub fn tanimlar(&self) -> &[ToolDef] {
        &self.tanimlar
    }

    pub fn bul(&self, ad: &str) -> Option<Arc<dyn Arac>> {
        self.araclar.get(ad).map(|(arac, _)| arac.clone())
    }

    pub fn dogrula(&self, ad: &str, argumanlar: &str) -> Result<Value, String> {
        let sema = self.araclar.get(ad).map(|(_, sema)| sema.as_ref());
        match semaya_gore_dogrula(ad, sema, argumanlar) {
            Ok(deger) => Ok(deger),
            Err(hata) => {
                if let Some(arac) = self.bul(ad) {
                    let maskeli = arac.kayit_argumanlarini_maskele(argumanlar);
                    if maskeli != argumanlar {
                        return Err(semaya_gore_dogrula(ad, sema, &maskeli)
                            .err()
                            .unwrap_or(hata));
                    }
                }
                Err(hata)
            }
        }
    }

    /// Modelin ürettiği araç çağrılarının kayıt/geçmiş kopyasında hassas
    /// argümanları maskeler; yürütmeye giden ham `ToolCall` listesini değiştirmez.
    pub fn kayit_cagrilarini_maskele(
        &self,
        cagrilar: &[kalfa_evren::tipler::ToolCall],
    ) -> Vec<kalfa_evren::tipler::ToolCall> {
        cagrilar
            .iter()
            .map(|tc| {
                let mut kopya = tc.clone();
                if let Some(arac) = self.bul(&tc.function.name) {
                    kopya.function.arguments =
                        arac.kayit_argumanlarini_maskele(&tc.function.arguments);
                }
                kopya
            })
            .collect()
    }
}

/// Kaynağın yayımladığı değişmez araç girdisi. Tanım ve şema yapılırken bir
/// kez okunur; görüntü sarmalayıcıyı yeniden sorgulamaz. Böylece modelin
/// gördüğü tanım, doğrulanan şema ve çalışan sarmalayıcı aynı sürümdendir.
#[derive(Clone)]
pub struct KaynakAraci {
    tanim: ToolDef,
    sema: Arc<jsonschema::Validator>,
    arac: Arc<dyn Arac>,
}

impl KaynakAraci {
    /// Harici başvurulu şema derleyiciye ulaşmadan reddedilir.
    pub fn yeni(arac: Arc<dyn Arac>) -> Result<Self, KayitHatasi> {
        let sema = arac.parametre_semasi();
        let gecersiz = |hata: String| KayitHatasi::GecersizSema {
            arac: arac.ad().to_string(),
            hata,
        };
        if harici_ref_var_mi(&sema) {
            return Err(gecersiz("harici başvuru".into()));
        }
        let dogrulayici = jsonschema::validator_for(&sema).map_err(|e| gecersiz(e.to_string()))?;
        Ok(Self {
            tanim: ToolDef::function(arac.ad(), arac.aciklama(), sema),
            sema: Arc::new(dogrulayici),
            arac,
        })
    }

    pub fn ad(&self) -> &str {
        &self.tanim.function.name
    }
}

/// Bağımsız görev olarak başlatılan tanım denetimi.
pub type AracDenetimIsi = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;

/// Tanım denetiminin bütçe ve günlük sahibi: aktif turu olan oturum. `iptal`
/// turun belirtecidir; tur bitince ya da oturum kapanınca tetiklenir.
#[derive(Clone)]
pub struct AracDenetimSahibi {
    pub oturum: crate::kimlik::OturumId,
    pub butce: ButceTalepci,
    pub iptal: CancellationToken,
}

/// Oturum dışında yayımlanan araçların kaynağı (MCP). Core `kalfa-mcp`'yi
/// tanımaz; uygulama alan başına denetlenmiş ve sürümlü kümeyi buradan verir.
pub trait AracKaynagi: Send + Sync {
    /// Son hazır yayım. Senkron ve kısadır; keşfi ya da denetimi beklemez.
    fn hazir_araclar(&self) -> Arc<[KaynakAraci]>;

    /// Bekleyen tanım denetimlerini bu sahiple sahiplenir. Oturum dönen
    /// işleri bağımsız görev olarak başlatır, beklemez; böylece işlerin
    /// bütçe talepleri oturumun olay döngüsünde işlenir.
    fn denetim_islerini_al(&self, sahip: AracDenetimSahibi) -> Vec<AracDenetimIsi>;
}

// MCP çalışma tarifinin ayrı özet ve sayaç tipleri.
// Bunlar yalnız SHA-256 özetleri ve sayaç taşır; ham komut/ortam verisi yoktur.

macro_rules! mcp_ozet_tipi {
    ($ad:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $ad([u8; 32]);

        impl $ad {
            pub const fn sha256(self) -> [u8; 32] {
                self.0
            }

            pub const fn ozetten(ozet: [u8; 32]) -> Self {
                Self(ozet)
            }
        }
    };
}

mcp_ozet_tipi!(McpTanimOzeti); // T
mcp_ozet_tipi!(McpPolitikaOzeti); // Π
mcp_ozet_tipi!(McpKorumaOzeti); // K
mcp_ozet_tipi!(McpYalitimProfiliOzeti); // P üretimine giren ham profil özeti
mcp_ozet_tipi!(McpProfilOzeti); // P = Π + K + yalıtım profili
mcp_ozet_tipi!(McpCalismaTarifiOzeti); // R = T + P + N
mcp_ozet_tipi!(McpAracTanimOzeti); // D

/// Her süreç başlangıcında artan sayaçtır; rastgele nonce veya yazma kuşağı değildir.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct McpBaslatmaSayaci(u64); // N

impl McpBaslatmaSayaci {
    pub const fn yeni(deger: u64) -> Self {
        Self(deger)
    }

    pub const fn deger(self) -> u64 {
        self.0
    }

    pub const fn ileri(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    pub const fn sonraki(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(deger) => Some(Self(deger)),
            None => None,
        }
    }
}

/// W'de yazma kaynaklı izin yenilemesi için ayrı sayaç; R'deki N ile karıştırılmaz.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct McpYazmaKusagi(u64);

impl McpYazmaKusagi {
    pub const fn yeni(deger: u64) -> Self {
        Self(deger)
    }

    pub const fn deger(self) -> u64 {
        self.0
    }

    pub const fn ileri(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Mühür için güvenli, tipleri ayrılmış çalışma tarifi. D her MCP aracı için
/// ayrıdır; çağrı mührü daha sonra R, D ve argümanları birlikte bağlar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpTarifi {
    tanim: McpTanimOzeti,
    politika: McpPolitikaOzeti,
    koruma: McpKorumaOzeti,
    profil: McpProfilOzeti,
    baslatma: McpBaslatmaSayaci,
    calisma: McpCalismaTarifiOzeti,
}

impl McpTarifi {
    pub fn yeni(
        tanim: McpTanimOzeti,
        politika: McpPolitikaOzeti,
        koruma: McpKorumaOzeti,
        yalitim: McpYalitimProfiliOzeti,
        baslatma: McpBaslatmaSayaci,
    ) -> Self {
        let profil = McpProfilOzeti::birlestir(politika, koruma, yalitim);
        let calisma = McpCalismaTarifiOzeti::ozetten(mcp_birlesik_ozet(
            b"calisma-tarifi-v1",
            &[
                &tanim.sha256(),
                &profil.sha256(),
                &baslatma.deger().to_le_bytes(),
            ],
        ));
        Self {
            tanim,
            politika,
            koruma,
            profil,
            baslatma,
            calisma,
        }
    }

    pub const fn tanim(&self) -> McpTanimOzeti {
        self.tanim
    }

    pub const fn politika(&self) -> McpPolitikaOzeti {
        self.politika
    }

    pub const fn koruma(&self) -> McpKorumaOzeti {
        self.koruma
    }

    pub const fn profil(&self) -> McpProfilOzeti {
        self.profil
    }

    pub const fn baslatma(&self) -> McpBaslatmaSayaci {
        self.baslatma
    }

    pub const fn calisma(&self) -> McpCalismaTarifiOzeti {
        self.calisma
    }
}

impl McpProfilOzeti {
    pub fn birlestir(
        politika: McpPolitikaOzeti,
        koruma: McpKorumaOzeti,
        yalitim: McpYalitimProfiliOzeti,
    ) -> Self {
        Self::ozetten(mcp_birlesik_ozet(
            b"mcp-profil-v1",
            &[&politika.sha256(), &koruma.sha256(), &yalitim.sha256()],
        ))
    }
}

/// Yalıtım kipinin MCP sunucusu için özeti. Tam erişim her çağrıda yerel
/// onay gerektirir; uzak Streamable HTTP ise yerel süreç yalıtımı (AppContainer/bwrap)
/// olmadığını ve çağrının uzak sunucuya ağ üzerinden çıktığını açıkça belirtir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum McpErisim {
    Yalitimli,
    TamErisim,
    UzakStreamableHttp,
}

/// Kartta gösterilecek sunucu veya araç etiketi. Yalnız görünür ad alır;
/// yol/komut biçimi ve kontrol karakterleri reddedilir. Sır taraması çağrı
/// sonucunu işlerken ayrıca yapılır. Hata girdiyi geri yansıtmaz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpKartEtiketi(String);

impl McpKartEtiketi {
    pub fn yeni(ad: &str) -> Result<Self, &'static str> {
        let temiz = ad.trim();
        let guvenli_karakterler = temiz
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.'));
        if temiz.is_empty() || temiz != ad || temiz.len() > 128 || !guvenli_karakterler {
            return Err("MCP kart etiketi güvenli bir görünen ad değil.");
        }
        Ok(Self(temiz.to_string()))
    }

    pub fn gorunen(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for McpKartEtiketi {
    type Error = &'static str;

    fn try_from(deger: String) -> Result<Self, Self::Error> {
        Self::yeni(&deger)
    }
}

impl<'de> Deserialize<'de> for McpKartEtiketi {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let ad = String::deserialize(deserializer)?;
        Self::yeni(&ad).map_err(serde::de::Error::custom)
    }
}

impl Serialize for McpKartEtiketi {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

/// Ağ kartında yalnızca host ve isteğe bağlı port saklanabilir; URL, kullanıcı
/// bilgisi, yol, sorgu veya sır içeremez.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpAgHostu(String);

impl McpAgHostu {
    pub fn yeni(host: &str) -> Result<Self, &'static str> {
        if host.trim() != host
            || host.is_empty()
            || host.len() > 260
            || host.chars().any(char::is_control)
            || host
                .chars()
                .any(|c| matches!(c, '/' | '\\' | '@' | '?' | '#' | '%'))
            || host.contains("://")
        {
            return Err("MCP ağ kartı yalnızca host ve isteğe bağlı port içerebilir.");
        }
        let (ad, port) = if let Some(kalan) = host.strip_prefix('[') {
            let Some((ipv6, son)) = kalan.split_once(']') else {
                return Err("MCP ağ hostu geçersiz.");
            };
            if ipv6.parse::<std::net::Ipv6Addr>().is_err() {
                return Err("MCP ağ hostu geçersiz.");
            }
            let port = if son.is_empty() {
                None
            } else if let Some(port) = son.strip_prefix(':') {
                Some(port)
            } else {
                return Err("MCP ağ hostu geçersiz.");
            };
            (ipv6, port)
        } else if host.parse::<IpAddr>().is_ok() {
            (host, None)
        } else if let Some((ad, port)) = host.rsplit_once(':') {
            if ad.contains(':') {
                return Err("IPv6 MCP ağ hostu köşeli ayraçla yazılmalıdır.");
            }
            (ad, Some(port))
        } else {
            (host, None)
        };
        if let Some(port) = port
            && !port.parse::<u16>().is_ok_and(|deger| deger != 0)
        {
            return Err("MCP ağ hostunun portu geçersiz.");
        }
        let dns = ad.len() <= 253
            && !ad.is_empty()
            && ad.split('.').all(|etiket| {
                !etiket.is_empty()
                    && etiket.len() <= 63
                    && !etiket.starts_with('-')
                    && !etiket.ends_with('-')
                    && etiket
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            });
        if ad.parse::<IpAddr>().is_err() && !dns {
            return Err("MCP ağ hostu geçersiz.");
        }
        Ok(Self(host.to_ascii_lowercase()))
    }

    pub fn gorunen(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for McpAgHostu {
    type Error = &'static str;

    fn try_from(deger: String) -> Result<Self, Self::Error> {
        Self::yeni(&deger)
    }
}

impl Serialize for McpAgHostu {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

/// MCP onay kartına gidecek ağ özeti. Hostlar doğrulanmış ayrı tiptir;
/// `Sinirsiz` sınırsız erişimi, `Kapali` ağ erişimi olmadığını belirtir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kip")]
pub enum McpAgKarti {
    Kapali,
    Sinirli {
        hostlar: Vec<McpAgHostu>,
    },
    Sinirsiz,
    /// Uzak Streamable HTTP MCP hedefi: kanonik origin ve sır içermeyen kimlik kapsamı.
    UzakHedef {
        host: McpAgHostu,
        origin: String,
        kimlik_kapsami: String,
    },
}

impl McpAgKarti {
    pub fn ag_kullaniliyor(&self) -> bool {
        match self {
            Self::Kapali => false,
            Self::Sinirli { hostlar } => !hostlar.is_empty(),
            Self::Sinirsiz | Self::UzakHedef { .. } => true,
        }
    }

    pub fn uzak_origin(&self) -> Option<&str> {
        match self {
            Self::UzakHedef { origin, .. } => Some(origin.as_str()),
            _ => None,
        }
    }

    pub fn kimlik_kapsami(&self) -> Option<&str> {
        match self {
            Self::UzakHedef { kimlik_kapsami, .. } => Some(kimlik_kapsami.as_str()),
            _ => None,
        }
    }
}

/// MCP çağrısı için UI'ya güvenle gösterilebilen hazırlık. R, D, adlar,
/// oturum kipi, erişim kipi, ağ özeti ve ilk çağrı bilgisi taşır; ham süreç
/// yapılandırması veya gizli değer taşıyan alan içermez.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpAracHazirligi {
    pub sunucu: McpKartEtiketi,
    pub arac: McpKartEtiketi,
    pub kip: Kip,
    pub erisim: McpErisim,
    pub ag: McpAgKarti,
    pub ilk_cagri: bool,
    pub tarif: McpTarifi,
    pub arac_tanimi: McpAracTanimOzeti,
}

impl Serialize for McpAracHazirligi {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct GuvenliKart<'a> {
            sunucu: &'a McpKartEtiketi,
            arac: &'a McpKartEtiketi,
            kip: Kip,
            erisim: McpErisim,
            ag: &'a McpAgKarti,
            ilk_cagri: bool,
            /// R ve D; T/Π/K/P/N bileşenleri veya ham süreç girdileri değil.
            calisma_tarifi_ozeti: String,
            arac_tanimi_ozeti: String,
        }

        GuvenliKart {
            sunucu: &self.sunucu,
            arac: &self.arac,
            kip: self.kip,
            erisim: self.erisim,
            ag: &self.ag,
            ilk_cagri: self.ilk_cagri,
            calisma_tarifi_ozeti: hex::encode(self.tarif.calisma().sha256()),
            arac_tanimi_ozeti: hex::encode(self.arac_tanimi.sha256()),
        }
        .serialize(serializer)
    }
}

fn mcp_birlesik_ozet(alan_adi: &[u8], parcalar: &[&[u8]]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut ozet = Sha256::new();
    for parca in [b"kalfa-mcp-canonical-v1".as_slice(), alan_adi]
        .into_iter()
        .chain(parcalar.iter().copied())
    {
        ozet.update((parca.len() as u64).to_le_bytes());
        ozet.update(parca);
    }
    ozet.finalize().into()
}

/// Ham şema ve jsonschema hata ayrıntısını dışarı vermeyen derleme hatası.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpJsonSemasiHatasi;

impl std::fmt::Display for McpJsonSemasiHatasi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MCP JSON Schema güvenli biçimde derlenemedi.")
    }
}

impl std::error::Error for McpJsonSemasiHatasi {}

/// Derlenmiş MCP şeması. Validator alanı kasıtlı olarak özel kalır.
pub struct McpJsonSemasi {
    validator: jsonschema::Validator,
}

impl std::fmt::Debug for McpJsonSemasi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpJsonSemasi").finish_non_exhaustive()
    }
}

impl McpJsonSemasi {
    /// Harici başvuru derleyiciye ulaşmadan reddedilir.
    pub fn derle(sema: &Value) -> Result<Self, McpJsonSemasiHatasi> {
        if !sema.is_object() || harici_ref_var_mi(sema) {
            return Err(McpJsonSemasiHatasi);
        }

        let validator = jsonschema::validator_for(sema).map_err(|_| McpJsonSemasiHatasi)?;
        Ok(Self { validator })
    }

    /// Değerin derlenmiş şemaya uyup uymadığını hata metni üretmeden döndürür.
    pub fn uygun_mu(&self, deger: &Value) -> bool {
        self.validator.is_valid(deger)
    }
}

fn harici_ref_var_mi(sema: &Value) -> bool {
    let Some(nesne) = sema.as_object() else {
        return false;
    };

    // `$id` (Draft 4: `id`) başvuru tabanını değiştirerek `#fragment` ref'ini
    // bile dış kaynağa yönlendirebilir. Bu dar API'de taban tanımları kapalıdır.
    for anahtar in ["$id", "id"] {
        if matches!(nesne.get(anahtar), Some(Value::String(deger)) if !deger.is_empty()) {
            return true;
        }
    }

    for anahtar in ["$ref", "$dynamicRef", "$recursiveRef"] {
        if let Some(Value::String(ref_degeri)) = nesne.get(anahtar)
            && !ref_degeri.starts_with('#')
        {
            return true;
        }
    }

    for (anahtar, alt) in nesne {
        match anahtar.as_str() {
            "const" | "default" | "enum" | "examples" => {}
            "$defs" | "definitions" | "properties" | "patternProperties" | "dependentSchemas"
            | "dependencies" => {
                if let Some(alt_nesne) = alt.as_object()
                    && alt_nesne.values().any(harici_ref_var_mi_deger)
                {
                    return true;
                }
            }
            _ if harici_ref_var_mi_deger(alt) => {
                return true;
            }
            _ => {}
        }
    }

    false
}

fn harici_ref_var_mi_deger(deger: &Value) -> bool {
    match deger {
        Value::Object(_) => harici_ref_var_mi(deger),
        Value::Array(diziler) => diziler.iter().any(harici_ref_var_mi_deger),
        _ => false,
    }
}

#[cfg(test)]
mod mcp_tarif_testleri {
    use super::*;

    #[test]
    fn mcp_tarifi_bilesenleri_degisince_profil_ve_calisma_ozeti_degistirilir() {
        let t = McpTanimOzeti::ozetten([1; 32]);
        let pi = McpPolitikaOzeti::ozetten([2; 32]);
        let k = McpKorumaOzeti::ozetten([3; 32]);
        let yalitim = McpYalitimProfiliOzeti::ozetten([4; 32]);
        let n = McpBaslatmaSayaci::yeni(8);
        let ilk = McpTarifi::yeni(t, pi, k, yalitim, n);
        assert_eq!(ilk.profil(), McpProfilOzeti::birlestir(pi, k, yalitim));
        for farkli in [
            McpTarifi::yeni(McpTanimOzeti::ozetten([9; 32]), pi, k, yalitim, n),
            McpTarifi::yeni(t, McpPolitikaOzeti::ozetten([9; 32]), k, yalitim, n),
            McpTarifi::yeni(t, pi, McpKorumaOzeti::ozetten([9; 32]), yalitim, n),
            McpTarifi::yeni(t, pi, k, McpYalitimProfiliOzeti::ozetten([9; 32]), n),
            McpTarifi::yeni(t, pi, k, yalitim, n.sonraki().unwrap()),
        ] {
            assert_ne!(ilk.calisma(), farkli.calisma());
        }
        assert!(McpBaslatmaSayaci::yeni(u64::MAX).sonraki().is_none());
    }

    #[test]
    fn mcp_kart_etiketi_ve_ag_hostu_yol_url_ve_kontrol_karakterini_almaz() {
        assert!(McpKartEtiketi::yeni("sunucu 1").is_ok());
        assert!(McpKartEtiketi::yeni(r"C:\Users\koray\ayar.json").is_err());
        assert!(McpKartEtiketi::yeni("https://example.test/token").is_err());
        let uzun_gecerli_ad = "a".repeat(48);
        assert_eq!(
            McpKartEtiketi::yeni(&uzun_gecerli_ad).unwrap().gorunen(),
            uzun_gecerli_ad.as_str()
        );
        assert!(McpKartEtiketi::yeni("sunucu\nadi").is_err());
        assert!(serde_json::from_str::<McpKartEtiketi>(r#""C:/secret/token""#).is_err());

        assert_eq!(
            McpAgHostu::yeni("API.Example.test:443").unwrap().gorunen(),
            "api.example.test:443"
        );
        assert!(McpAgHostu::yeni("https://user:pass@example.test/secret").is_err());
        assert!(McpAgHostu::yeni("example.test/path").is_err());

        let kart = McpAracHazirligi {
            sunucu: McpKartEtiketi::yeni("sunucu 1").unwrap(),
            arac: McpKartEtiketi::yeni("dosya-oku").unwrap(),
            kip: Kip::Proje,
            erisim: McpErisim::Yalitimli,
            ag: McpAgKarti::Sinirli {
                hostlar: vec![McpAgHostu::yeni("api.example.test:443").unwrap()],
            },
            ilk_cagri: true,
            tarif: McpTarifi::yeni(
                McpTanimOzeti::ozetten([1; 32]),
                McpPolitikaOzeti::ozetten([2; 32]),
                McpKorumaOzeti::ozetten([3; 32]),
                McpYalitimProfiliOzeti::ozetten([4; 32]),
                McpBaslatmaSayaci::yeni(5),
            ),
            arac_tanimi: McpAracTanimOzeti::ozetten([6; 32]),
        };
        let json = serde_json::to_string(&kart).unwrap();
        assert!(json.contains("calisma_tarifi_ozeti"));
        assert!(json.contains("arac_tanimi_ozeti"));
        assert!(json.contains("api.example.test:443"));
        assert!(!json.contains("baslatma"), "N kartta ayrı alan olarak yok");
        assert!(!json.contains("koruma"), "K kartta ayrı alan olarak yok");
    }

    #[test]
    fn yerel_fragment_basvurusu_derlenir_ve_calistirilir() {
        let sema = serde_json::json!({
            "$defs": { "sayi": { "type": "integer", "minimum": 1 } },
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "adet": { "$ref": "#/$defs/sayi" }
            },
            "required": ["adet"]
        });
        let derlenmis = McpJsonSemasi::derle(&sema).unwrap();

        assert!(derlenmis.uygun_mu(&serde_json::json!({"adet": 2})));
        assert!(!derlenmis.uygun_mu(&serde_json::json!({"adet": 0})));
    }

    #[test]
    fn ag_ve_dosya_refleri_hata_ayrintisi_tasimadan_reddedilir() {
        for ref_degeri in [
            "https://ornek.invalid/secret.json",
            "file:///C:/private/schema.json",
            "../schema.json",
        ] {
            let sema = serde_json::json!({ "$ref": ref_degeri });
            let hata = McpJsonSemasi::derle(&sema).unwrap_err();

            assert_eq!(hata, McpJsonSemasiHatasi);
            assert!(!format!("{hata:?}").contains(ref_degeri));
        }
    }

    #[test]
    fn ic_ice_sema_dis_refleri_de_cozucuye_ulasmaz() {
        for sema in [
            serde_json::json!({
                "type": "object",
                "properties": {
                    "alan": { "$ref": "../private/schema.json" }
                }
            }),
            serde_json::json!({
                "type": "object",
                "$defs": {
                    "grup": {
                        "alt": { "$ref": "https://ornek.invalid/sema.json" }
                    }
                },
                "$ref": "#/$defs/grup/alt"
            }),
            serde_json::json!({
                "type": "object",
                "ozel": { "$id": "https://ornek.invalid/taban/", "type": "string" },
                "$ref": "#/ozel"
            }),
        ] {
            assert!(McpJsonSemasi::derle(&sema).is_err());
        }
    }

    #[test]
    fn dis_id_local_fragment_basvurusunu_disari_yonlendiremez() {
        let sema = serde_json::json!({
            "$id": "https://ornek.invalid/uzak.json",
            "$defs": {"sayi": {"type": "integer"}},
            "$ref": "#/$defs/sayi"
        });

        assert!(McpJsonSemasi::derle(&sema).is_err());
    }

    #[test]
    fn sema_olmayan_ornek_verideki_ref_metni_yanlis_eslesmez() {
        let sema = serde_json::json!({
            "type": "object",
            "examples": [{ "$ref": "https://ornek.invalid/yalnizca-ornek" }]
        });

        assert!(McpJsonSemasi::derle(&sema).is_ok());
    }

    #[test]
    fn derleme_hatasi_ham_sema_metnini_tasima_yolu_acmaz() {
        let sema = serde_json::json!({
            "type": "gecersiz-tur-adini-ham-hata-ile-gosterme"
        });
        let hata = McpJsonSemasi::derle(&sema).unwrap_err();

        assert_eq!(hata, McpJsonSemasiHatasi);
        assert!(!format!("{hata:?}").contains("gecersiz-tur"));
    }
}

#[cfg(test)]
mod kilitli_arac_testleri {
    use super::*;
    use crate::kilit::KilitYoneticisi;
    use crate::yol::CalismaYolu;
    use std::time::Duration;
    use tokio::sync::Notify;

    struct BekleyenArac {
        basladi: Arc<Notify>,
        devam: Arc<Notify>,
    }

    #[async_trait]
    impl Arac for BekleyenArac {
        fn ad(&self) -> &str {
            "kilit_bekleyen"
        }
        fn aciklama(&self) -> &str {
            "test aracı"
        }
        fn parametre_semasi(&self) -> Value {
            serde_json::json!({"type": "object"})
        }
        fn yazar_mi(&self) -> bool {
            false
        }
        async fn hazirla(&self, _argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
            Ok(Hazirlik::default())
        }
        async fn calistir(
            &self,
            _argumanlar: &Value,
            _hazirlik: &Hazirlik,
            _iptal: CancellationToken,
        ) -> Result<AracCiktisi, AracHatasi> {
            self.basladi.notify_one();
            self.devam.notified().await;
            Ok(AracCiktisi::default())
        }
    }

    #[tokio::test]
    async fn varsayilan_kilitli_arac_await_sirasinda_yazma_kilidini_tutar() {
        let kok = tempfile::tempdir().unwrap();
        let yonetici = KilitYoneticisi::yeni();
        let kilitler = yonetici.al(kok.path()).unwrap();
        let hedef = CalismaYolu::coz("rapor.txt").unwrap();
        let kilit = kilitler.al(&[], std::slice::from_ref(&hedef), false).await;
        let basladi = Arc::new(Notify::new());
        let devam = Arc::new(Notify::new());
        let arac = Arc::new(BekleyenArac {
            basladi: basladi.clone(),
            devam: devam.clone(),
        });
        let is = tokio::spawn(async move {
            arac.calistir_kilitli(
                &serde_json::json!({}),
                &Hazirlik::default(),
                CancellationToken::new(),
                AracOrtami::bos(),
                Some(kilit),
            )
            .await
        });

        basladi.notified().await;
        assert!(
            tokio::time::timeout(
                Duration::from_millis(25),
                kilitler.al(&[], std::slice::from_ref(&hedef), false)
            )
            .await
            .is_err(),
            "araç await ederken aynı yolun yazma kilidi tutulmalı"
        );
        devam.notify_one();
        assert!(is.await.unwrap().is_ok());
        assert!(
            tokio::time::timeout(
                Duration::from_secs(1),
                kilitler.al(&[], std::slice::from_ref(&hedef), false)
            )
            .await
            .is_ok(),
            "varsayılan çağrı bitince kilit bırakılmalı"
        );
    }
}
