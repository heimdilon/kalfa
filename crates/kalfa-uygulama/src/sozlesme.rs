//! Komut sözleşmesinin uygulamaya ait tipleri (1c tasarımı §6.1). JSON
//! biçimleri arayüzdeki `ui/src/ipc/tipler.ts` ile aynıdır; biri değişirse
//! öbürü de aynı commit'te değişir.

use kalfa_core::butce::ButceGorunumu;
use kalfa_core::gecmis::{DurumOzeti, GecmisSayfasi, OturumDurumu, OturumOzeti};
pub use kalfa_core::gonderim::{GunlukSatiri, GunlukSayfasi, GunlukToken};
use kalfa_core::kimlik::{CagriId, OturumId};
use kalfa_core::muhur::OnayMuhru;
pub use kalfa_core::plan::{OturumPlani, PlanAdimDurumu, PlanAdimi};
pub use kalfa_core::proje_talimatlari::{
    ProjeTalimatAnlikGoruntusu, ProjeTalimatDegisimi, ProjeTalimatKapsami, ProjeTalimatKaynagi,
    ProjeTalimatUyariTuru, ProjeTalimatUyarisi,
};
use kalfa_core::protokol::OnayKarari;
use kalfa_core::sart_kapisi::SartMetni;
pub use kalfa_core::soru::{KullaniciSorusu, KullaniciSorusuYaniti, SoruSecenegi, SoruSonucu};
pub use kalfa_evren::tipler::DusunceEforu;
use kalfa_sandbox::YalitimDurumu;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BaglantiDurumu {
    Var,
    /// Açılışta EVREN'e ulaşılamadı; ana pencere şerit gösterir (1c tasarımı §7.1).
    Yok,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DepoTuru {
    /// İşletim sisteminin kimlik deposu.
    Kalici,
    /// Depo yok; anahtar Kalfa açık kaldıkça bellekte (1c tasarımı §7.2).
    Bellekte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ModelProfili {
    Hizli,
    Dengeli,
    Derin,
    Otomatik,
}

/// "Onar"ın sonucu (aşama 2 tasarımı §9.8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum OnarimSonucu {
    /// Kurulum yinelendi; yeni durum ana pencerenin kanalına da yazıldı.
    Tamam { yalitim: YalitimDurumu },
    /// Yerel pencerede ya da UAC'de vazgeçildi.
    Vazgecildi,
    /// Kurulum başarısız oldu; mesaj teknik ayrıntıdır.
    Basarisiz { mesaj: String },
    /// Kurulu sürüm değil (geliştirme derlemesi ya da Linux): Onar yok.
    KuruluDegil { talimat: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModelSecenegi {
    pub kimlik: String,
    pub profil: ModelProfili,
    pub eforlar: Vec<DusunceEforu>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UygulamaDurumu {
    pub baglanti: BaglantiDurumu,
    pub anahtar_deposu: DepoTuru,
    pub modeller: Vec<ModelSecenegi>,
    pub varsayilan_model: String,
    pub varsayilan_efor: DusunceEforu,
    /// Bu çalıştırmada en son açılan ya da başlatılan oturum; yeniden
    /// yüklenen ana pencere onu kayıttan açar (1c tasarımı §5.6).
    pub son_oturum: Option<OturumId>,
    /// Kabuk komutlarının yalıtımı (aşama 2 tasarımı §10.3).
    pub yalitim: YalitimDurumu,
    /// Yalıtım yokken komutlar yalıtımsız çalışabilir mi (§9.8); Ayarlar'daki
    /// anahtar, varsayılan kapalı.
    pub yalitimsiz_izinli: bool,
    /// İşletim sistemi (`linux`, `windows`, `macos`): Ayarlar'daki yalıtım
    /// bölümü araç zinciri dizinlerini yalnızca Linux'ta gösterir.
    pub platform: String,
}

/// Oturum listesinin bir satırı (1c tasarımı §5.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OturumSatiri {
    /// Kayıt okunamadıysa `None`; o satır açılamaz.
    pub oturum: Option<OturumId>,
    /// Kayıt dosyasının adı.
    pub dosya: String,
    pub baslik: Option<String>,
    pub calisma_alani: Option<String>,
    pub son_degisiklik_ms: u64,
    pub durum: OturumDurumu,
}

impl From<OturumOzeti> for OturumSatiri {
    fn from(o: OturumOzeti) -> Self {
        Self {
            oturum: o.oturum,
            dosya: o
                .yol
                .file_name()
                .map(|ad| ad.to_string_lossy().into_owned())
                .unwrap_or_default(),
            baslik: o.baslik,
            calisma_alani: o.calisma_alani.map(|yol| yol.display().to_string()),
            son_degisiklik_ms: o.son_degisiklik_ms,
            durum: o.durum,
        }
    }
}

/// Yerel klasör seçiciden gelen çalışma alanı. Arayüz yol göndermez; bu
/// kimlikle başvurur. Kimlik yalnızca bu çalıştırmada geçerlidir (1c tasarımı
/// §5.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CalismaAlani {
    pub kimlik: u64,
    pub yol: String,
}

/// Yalıtım için hazırlanmış çalışma alanının Ayarlar'da gösterilen satırı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct YalitimCalismaAlani {
    pub kimlik: String,
    pub yol: String,
}

/// Kayıttan açılan oturumun neden yalnızca okunabildiği (1c tasarımı §10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum SaltOkunurNedeni {
    CalismaAlaniYok {
        yol: String,
    },
    /// Kayıt çalışma alanını yazmıyor (1c'den önceki kayıt).
    CalismaAlaniKayitsiz,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OturumAcilisi {
    pub oturum: OturumId,
    pub calisma_alani: Option<String>,
    /// MCP başlatma kartı bu kimlikle oturuma bağlanır (1.1a tasarımı §5.1).
    pub calisma_alani_kimligi: Option<String>,
    pub gecmis: GecmisSayfasi,
    pub ozet: DurumOzeti,
    pub salt_okunur: Option<SaltOkunurNedeni>,
    /// Oturumun istek ve token sayaçları.
    pub butce: Option<ButceGorunumu>,
}

/// Onay sayfasındaki tek bir çağrının kararı.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OnayGirdisi {
    pub cagri: CagriId,
    pub muhur: OnayMuhru,
    pub karar: OnayKarari,
    /// Kartın önerdiği kısayolu kabul et (aşama 3 tasarımı §9.1).
    #[serde(default)]
    pub kisayol: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AnahtarNedeni {
    /// Kimlik deposunda anahtar yok (ilk açılış).
    Yok,
    /// Kayıtlı anahtar artık geçerli değil (401).
    KayitliGecersiz,
    /// Kullanıcı "Anahtarı değiştir"i seçti.
    Degistir,
}

/// Şart penceresinin göstereceği görünüm (1c tasarımı §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum SartPenceresiDurumu {
    AnahtarGerekli {
        neden: AnahtarNedeni,
    },
    SartGerekli {
        metin: SartMetni,
    },
    /// Kullanıcı şartları reddetti (1c tasarımı §7.4).
    Kilitli,
    /// İş bitti; pencere kapanır.
    Tamam,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum AnahtarSonucu {
    /// 401: anahtar saklanmadı.
    Gecersiz,
    /// EVREN'e ulaşılamadı: anahtar saklanmadı.
    Ulasilamadi,
    Kaydedildi {
        depo: DepoTuru,
        sonraki: SartPenceresiDurumu,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum KabulYaniti {
    Kabul,
    /// Sürüm gösterilenle aynı değildi ya da bu arada değişti.
    YeniMetin {
        metin: SartMetni,
    },
    /// Aynı anda "Kabul etmiyorum" da verildi; ret geçerli.
    Kilitli,
}

/// Kullanıcının ağ izin listesindeki bir host (aşama 3 tasarımı §8.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgHostu {
    pub host: String,
    pub port: u16,
    /// Özel ağ adreslerine (yerel ağ, şirket ağı) de bağlanabilir.
    #[serde(default)]
    pub ozel_ag: bool,
}

/// Ayarlar > "Gizli dosyalar" (3e Görev 4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GizliAyarGorunumu {
    /// Kapatılamayan varsayılan kalıplar; salt okunur.
    pub varsayilanlar: Vec<String>,
    pub ek_kaliplar: Vec<String>,
    pub gizli_sayilmayanlar: Vec<String>,
    /// `ayarlar.json` okunamadıysa nedeni.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ayar_hatasi: Option<String>,
}

/// Denetçi kipi ve ayar dosyasının durumu (3c Görev 7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenetciDurumu {
    pub kip: kalfa_core::denetci::DenetimKipi,
    /// `ayarlar.json` okunamadıysa nedeni; kip o zaman kapalıdır.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ayar_hatasi: Option<String>,
}

// --- Eklentiler (aşama 4 tasarımı §13, §14) ---

/// Eklentiler ekranının bir satırı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EklentiSatiri {
    pub ad: String,
    /// Kayıttaki ya da manifestteki sürüm.
    pub surum: String,
    pub aciklama: String,
    /// Kalfa'yla gelen eklenti (`kalfa-rag`): kaldırılamaz, güncellenemez.
    pub yerlesik: bool,
    pub durum: EklentiDurumu,
    /// `.wasm`'ın SHA-256'sı (onaltılık).
    pub sha256: String,
    /// Klasörden kurulanın kaynak klasörü; yerleşikte `None`.
    pub kaynak: Option<String>,
    /// Kurulum zamanı (Unix ms); yerleşikte `None`.
    pub kurulum_ms: Option<u64>,
    /// Manifestten üretilen yetenek cümlesi (§6.4); manifest okunamadıysa
    /// `None`.
    pub yetenek: Option<String>,
    pub araclar: Vec<EklentiAraci>,
    /// Eklentinin kayıtta adı geçen çalışma alanları.
    pub alanlar: Vec<EklentiAlani>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum EklentiDurumu {
    /// Etkin ve yüklü.
    Yuklendi,
    /// Kullanıcı devre dışı bıraktı; hiçbir çalışma alanında sunulmaz.
    DevreDisi,
    /// Manifest ya da bileşen yüklenemedi; `neden` kullanıcıya gösterilir.
    Yuklenemedi { neden: String },
}

/// Modelin gördüğü araç (§6.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EklentiAraci {
    /// Modelin gördüğü ad: `kalfa_rag__anlamsal_ara`.
    pub ad: String,
    pub aciklama: String,
}

/// Bir eklentinin bir çalışma alanındaki durumu (§13.3, §14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EklentiAlani {
    /// Çalışma alanı kimliği (`CalismaAlaniKaydi::kimlik`).
    pub kimlik: String,
    /// Gösterim için; arayüz geri göndermez.
    pub yol: String,
    pub acik: bool,
    /// EVREN'e gönderim izninin verildiği zaman (Unix ms).
    pub gonderim_izni_ms: Option<u64>,
    /// Eklentinin bu çalışma alanındaki deposunun boyutu (bayt).
    pub depo_bayt: u64,
}

/// "Klasörden kur"un sonucu (§13.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum KurulumSonucu {
    Kuruldu {
        ad: String,
        surum: String,
        /// Aynı adlı eklentinin yerine geçti.
        guncelleme: bool,
        /// Güncelleme gönderim kapsamını değiştirdi; izinler silindi.
        izinler_silindi: bool,
    },
    /// Klasör seçilmedi ya da yerel pencerede vazgeçildi.
    Vazgecildi,
    /// Klasör eklenti değil ya da yüklenemedi; ileti kullanıcıya gösterilir.
    Basarisiz { mesaj: String },
}

/// Eklentinin açılıp kapatılacağı çalışma alanı. Arayüz yol göndermez:
/// ya açık bir oturumun alanı ya da kayıttaki kimlik.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "tip")]
pub enum AlanHedefi {
    Oturum { oturum: OturumId },
    Kimlik { kimlik: String },
}

/// Oturum görünümündeki "Bu oturumdaki izinler" panelinin eklenti satırı
/// (§14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OturumEklentisi {
    pub ad: String,
    pub yerlesik: bool,
    /// Eklenti bu çalışma alanında açık.
    pub acik: bool,
    /// Eklenti devre dışı ya da yüklenemedi: anahtar değiştirilemez.
    pub kullanilamaz: bool,
    /// EVREN'e gönderim izninin verildiği zaman (Unix ms).
    pub gonderim_izni_ms: Option<u64>,
}

/// EVREN'in günlük token sınırı doldu (aşama 4c, §9.5). Sınır hesap
/// geneldir; bir istek başarılı olunca kalkar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GunlukSinirDurumu {
    /// Sınırın sıfırlanacağı an (RFC 3339), EVREN söylediyse.
    pub sifirlanma: Option<String>,
}

// --- MCP sunucu yapılandırma ve kayıt sözleşmesi (1.1a-2 Görev 3) ---
pub use crate::mcp::ayarlar::{
    McpAgKurali, McpKipi, McpKomut, McpOrtamDegiskeniDto, McpSunucuAyrintiDto, McpSunucuOzetiDto,
};
pub use crate::mcp::kasa::GizliKimlik;
pub use crate::mcp::tanimlar::{
    AtlananUrlSunucusuDto, IceAktarmaOnizlemeDto, IceAktarmaSunucuOnizlemeDto, SeciliKomutBilgisi,
    TaslakKimligi,
};
