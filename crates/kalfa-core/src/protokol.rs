//! `Komut` / `Olay` sınırı (spec §4.1). Tauri IPC'ye, WebSocket'e, stdio'ya ve
//! birim testine eşit uzaklıktadır.

use kalfa_evren::KrediGozlemi;
use kalfa_evren::tipler::DusunceEforu;
use serde::{Deserialize, Serialize};

use crate::arac::{McpAgKarti, McpAracHazirligi, McpErisim};
use crate::butce::ButceGorunumu;
use crate::denetci::DenetimSonucu;
use crate::geri_alma::{GeriAlmaPlani, GeriAlmaSonucu};
use crate::izin::Kip;
use crate::kabuk::KabukTarifi;
use crate::kimlik::{CagriId, IslemId, OturumId, TurId};
use crate::muhur::OnayMuhru;
use crate::tur_ozeti::{DosyaDegisikligi, GeriAlinabilirlik, TurOzeti};

pub const SEMA_SURUMU: u32 = 1;

/// Her komut ve olay bu zarfla taşınır. `sira` oturum başına tekdüze artar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Zarf<T> {
    pub sema: u32,
    pub oturum: OturumId,
    pub tur: Option<TurId>,
    pub sira: u64,
    pub yuk: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum Komut {
    KullaniciMesaji {
        metin: String,
    },
    Iptal {
        tur: TurId,
    },
    AracOnayi {
        cagri: CagriId,
        karar: OnayKarari,
        muhur: OnayMuhru,
        /// Kartın önerdiği kısayolu kabul et (aşama 3 tasarımı §9.1).
        /// Webview kapsamı gönderemez; çekirdek önermediği kısayolu kabul etmez.
        #[serde(default)]
        kisayol: bool,
    },
    /// Oturumdaki bir kısayolu kaldırır (§9.1).
    KisayoluKaldir {
        kimlik: u64,
    },
    /// Yerel iletişim kutusunda onaylanan işlem (aşama 2 tasarımı §5.2).
    /// Yalnızca `kalfa-uygulama` gönderir; webview gönderemez. Yerel onay
    /// isteyen bir öğe `AracOnayi` ile onaylanamaz, yalnızca reddedilebilir.
    YerelOnay {
        cagri: CagriId,
        muhur: OnayMuhru,
    },
    ModelDegistir {
        model: String,
    },
    EforDegistir {
        efor: DusunceEforu,
    },
    IzinKipiDegistir {
        kip: Kip,
    },
    /// Turun geri alma planını ister (aşama 3 tasarımı §5.6). Tur sürerken
    /// reddedilir.
    GeriAlmaOnizle {
        tur: TurId,
    },
    /// Önizlenen planı mührüyle yürütür.
    TuruGeriAl {
        tur: TurId,
        muhur: OnayMuhru,
    },
    /// Göndermeden önce gösterme ayarı (aşama 3b).
    OnizlemeAyarla {
        acik: bool,
    },
    /// Gönderim önizlemesi kararı (aşama 3b).
    GonderimKarari {
        islem: IslemId,
        #[serde(default)]
        gonderilmeyecekler: Vec<CagriId>,
        /// Bu araç çağrılarının sonraki eklenti istekleri sorulmadan gider:
        /// "Bu araç çağrısında hep gönder" (aşama 4 tasarımı §9.4).
        #[serde(default)]
        hep_gonderilecekler: Vec<CagriId>,
    },
    /// Oturumu elle özetler (`/compact`). Tur sürerken reddedilir.
    Ozetle {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        talimat: Option<String>,
    },
    /// Bekleyen yapılandırılmış kullanıcı sorusuna verilen yanıt.
    /// Araç onayından tamamen ayrıdır; yalnız eşleşen `(tur, soru_kimligi)` için geçerlidir.
    KullaniciSorusuYaniti {
        tur: TurId,
        soru_kimligi: String,
        yanit: crate::soru::KullaniciSorusuYaniti,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum OnayKarari {
    Onayla,
    Reddet { gerekce: Option<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AracDurumu {
    Basarili,
    Basarisiz,
    Reddedildi,
    IptalEdildi,
    SonucuBilinmiyor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnayOgesi {
    pub cagri: CagriId,
    pub arac: String,
    pub aciklama: String,
    pub hedefler: Vec<String>,
    pub diff: Option<String>,
    pub uyarilar: Vec<String>,
    pub muhur: OnayMuhru,
    /// Kabuk çağrısında diff yerine gösterilenler (aşama 2 tasarımı §10.2).
    #[serde(default)]
    pub kabuk: Option<KabukTarifi>,
    /// Onay yalnızca yerel iletişim kutusundan verilebilir (§5.2).
    #[serde(default)]
    pub yerel_onay: bool,
    /// Kabuk öğesinde: bu oturumda onaysız değişen dosyalar (güvenlik analizi
    /// Bulgu 3). Mühre girmez: işlemi değil bağlamı anlatır.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub onaysiz_degisiklikler: Option<OnaysizDegisiklikler>,
    /// "Geri alınabilir mi" (aşama 3 tasarımı §5.11): yazan dosya öğesinde
    /// `Evet` ya da nedeniyle `Hayir`, kabukta `Hayir(Kabuk)`, yalnızca okuyan
    /// öğede yok. Mühre girmez.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geri_alinabilir: Option<GeriAlinabilirlik>,
    /// Onay kartında "Veri çıkışı" satırı (aşama 3b, tasarım §9.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub veri_cikisi: Option<VeriCikisi>,
    /// "Bu oturumda izin ver" teklifi (aşama 3 tasarımı §9.1); yalnızca
    /// uygun öğede. Mühre girmez.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kisayol: Option<crate::kisayol::KisayolTeklifi>,
    /// Eklenti aracında eklentinin tarifi; arayüz ilk gönderim kartını buna
    /// göre gösterir (aşama 4 tasarımı §8.4, §14). Mühre hazırlıktan girer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eklenti: Option<crate::arac::EklentiTarifi>,
    /// MCP aracında kartın sunucu, araç, erişim ve ağ özeti (1.1a tasarımı
    /// §9). Mühre girmez; mühür hazırlıktan gelir.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp: Option<McpOnayKarti>,
}

/// MCP onay kartının özeti. Yalnız doğrulanmış görünen adlar ve hostlar;
/// komut, ortam ve yol taşımaz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpOnayKarti {
    pub sunucu: String,
    pub arac: String,
    pub erisim: McpErisim,
    /// İzinli hostlar; ağ kapalıysa ve tam erişimde boş.
    pub ag_hostlari: Vec<String>,
    /// Tam erişimde ağ sınırsızdır.
    pub ag_sinirsiz: bool,
    /// Uzak Streamable HTTP sunucusunda hedef origin (`https://...`); yerel sunucuda `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uzak_origin: Option<String>,
    /// Uzak Streamable HTTP sunucusunda sır içermeyen kimlik/yetki kapsamı özeti; yerel sunucuda `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kimlik_kapsami: Option<String>,
}

impl McpOnayKarti {
    pub fn yeni(h: &McpAracHazirligi) -> Self {
        let (ag_hostlari, ag_sinirsiz, uzak_origin, kimlik_kapsami) = match &h.ag {
            McpAgKarti::Kapali => (Vec::new(), false, None, None),
            McpAgKarti::Sinirli { hostlar } => (
                hostlar.iter().map(|x| x.gorunen().to_owned()).collect(),
                false,
                None,
                None,
            ),
            McpAgKarti::Sinirsiz => (Vec::new(), true, None, None),
            McpAgKarti::UzakHedef {
                host,
                origin,
                kimlik_kapsami,
            } => (
                vec![host.gorunen().to_owned()],
                false,
                Some(origin.clone()),
                Some(kimlik_kapsami.clone()),
            ),
        };
        Self {
            sunucu: h.sunucu.gorunen().to_owned(),
            arac: h.arac.gorunen().to_owned(),
            erisim: h.erisim,
            ag_hostlari,
            ag_sinirsiz,
            uzak_origin,
            kimlik_kapsami,
        }
    }
}

/// Onay kartında "Veri çıkışı" satırı (aşama 3b, tasarım §9.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VeriCikisi {
    DosyaIcerigi,
    KomutCiktisi,
    AramaSonuclari,
    DosyaAdlari,
    YalnizcaYazildi,
    /// Eklentinin ürettiği metin (aşama 4 tasarımı §8.1).
    EklentiCiktisi,
    /// Uzak MCP sunucusuna gönderilen argümanlar ve dönen yanıt (1.1b).
    UzakMcpCagrisi,
}

/// Kabuk onayında gösterilen bağlam: komut, modelin onaysız yazdığı kodu
/// çalıştırabilir (güvenlik analizi Bulgu 3).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnaysizDegisiklikler {
    /// Onay sorulmadan yazılan dosyalar.
    pub yollar: Vec<String>,
    /// Onaysız bir komut da çalıştı; değiştirdiği dosyalar bilinmiyor.
    pub komut_calisti: bool,
    /// `yollar` içindeki derleme ve betik dosyaları (`build.rs`,
    /// `package.json` …).
    pub derleme_dosyalari: Vec<String>,
}

/// Araç çıktısının hangi akıştan geldiği.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CiktiAkisi {
    Standart,
    Hata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum BitisNedeni {
    Normal,
    Iptal,
    Sinir { aciklama: String },
    Hata,
}

/// Olay aktarımının neden durduğu (1c tasarımı §4.4). Arayüz gösterilecek
/// metni buna göre seçer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AktarimDurmaNedeni {
    /// Tüketici kuyruğu boşaltamadı: olay ya da bayt bütçesi doldu.
    TuketiciYetismedi,
    /// Tek bir olay bütçeden büyüktü.
    OlayCokBuyuk,
    /// Olay serileştirilemedi ya da alıcı kapandı.
    Diger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OzetTetigi {
    Otomatik,
    Zorunlu,
    Elle,
}

/// Otomatik kip hakeminin çağrı başına kararı (tasarım §5, §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OtomatikSonuc {
    Izin,
    Ret,
    Kullaniciya,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum Olay {
    TurBasladi,
    TokenParcasi {
        metin: String,
    },
    DusunceParcasi {
        metin: String,
    },
    AracCagrisiBasladi {
        cagri: CagriId,
        arac: String,
        aciklama: String,
    },
    /// Yazan çağrının dosya değişiklikleri (masaüstü arayüzü tasarımı §4);
    /// `AracCagrisiBasladi`'nın hemen ardından, sonuçtan önce gelir.
    DosyaDegisiklikleri {
        cagri: CagriId,
        dosyalar: Vec<DosyaDegisikligi>,
    },
    AracSonucu {
        cagri: CagriId,
        arac: String,
        durum: AracDurumu,
        icerik: String,
    },
    /// Aracın bildirdiği güvenlik uyarısı; `AracSonucu`'ndan sonra gelir.
    GuvenlikUyarisi {
        cagri: CagriId,
        mesaj: String,
    },
    /// Süren bir aracın ara çıktısı (aşama 2 tasarımı §4.4). Kayda yazılmaz.
    /// Ekran yetişemezse parçalar atılır; `atlanan_bayt` bu parçadan önce
    /// atılanların toplamıdır.
    AracCiktisiParcasi {
        cagri: CagriId,
        akis: CiktiAkisi,
        metin: String,
        atlanan_bayt: u64,
    },
    OnayGerekiyor {
        islem: IslemId,
        ogeler: Vec<OnayOgesi>,
    },
    /// Bir EVREN isteğinin kimliği ve yönlendirilen model (üst belge §6.4).
    IstekBilgisi {
        istek_id: Option<String>,
        yonlendirilen_model: Option<String>,
        #[serde(default)]
        amac: String,
        #[serde(default)]
        basarili: bool,
    },
    KrediGozlemi {
        gozlem: KrediGozlemi,
    },
    TokenKullanimi {
        istem: u64,
        tamamlama: u64,
        dusunce: u64,
    },
    SartOnayiGerekiyor {
        surum: u32,
        metin: String,
    },
    Hata {
        mesaj: String,
    },
    Tamamlandi {
        neden: BitisNedeni,
    },
    /// Olay kuyruğu durdu ve oturum kapanıyor; kuyruğun son olayıdır.
    AktarimDurdu {
        neden: AktarimDurmaNedeni,
    },
    /// Geri alma önizlemesi (aşama 3 tasarımı §5.6).
    GeriAlmaPlani {
        geri_alinan_tur: TurId,
        plan: GeriAlmaPlani,
        muhur: OnayMuhru,
    },
    TurGeriAlindi {
        geri_alinan_tur: TurId,
        sonuc: Vec<GeriAlmaSonucu>,
    },
    /// Depo sınırı yüzünden bu turların önceki hâlleri silindi; özetlerindeki
    /// "Turu geri al" kalkar (§5.2).
    GeriAlmaVerisiSilindi {
        turlar: Vec<TurId>,
    },
    /// Turun özeti (aşama 3 tasarımı §5.5); `Tamamlandi`'dan sonra, turun
    /// zarfıyla gelir.
    TurOzeti {
        ozet: TurOzeti,
    },
    /// Gönderim öncesi önizleme (aşama 3b).
    GonderimOnizlemesi {
        islem: IslemId,
        ogeler: Vec<OnizlemeOgesi>,
        neden: OnizlemeNedeni,
    },
    /// Güvenlik denetçisi çağrıyı incelemeye başladı (aşama 3c).
    DenetimBasladi {
        cagri: CagriId,
    },
    /// Dış kaynaklı sonuç onayla okunan gizli (ya da çok bağlantılı) bir
    /// dosyadan geldiği için denetçiye gönderilmedi (aşama 3c, Görev 4 ve 8).
    DenetimAtlandi {
        cagri: CagriId,
    },
    /// Oturumdaki kısayolların güncel listesi (aşama 3 tasarımı §9.1).
    /// Kayda yazılmaz.
    Kisayollar {
        liste: Vec<crate::kisayol::Kisayol>,
    },
    /// Bütçe sayaçlarının güncel görünümü; her başlatma ve bitiş sonrası
    /// (5b tasarımı §9).
    ButceDurumu {
        durum: ButceGorunumu,
    },
    /// Güvenlik denetçisinin sonucu (aşama 3c).
    DenetimSonucu {
        cagri: CagriId,
        sonuc: DenetimSonucu,
    },
    /// Oturumun önceki kısmı özetlendi. `once`/`sonra` tahmini istem tokenıdır.
    OturumOzetlendi {
        tetik: OzetTetigi,
        once: u64,
        sonra: u64,
        talimat: Option<String>,
        ozet: String,
    },
    /// Bağlamın tahmini tokenı ve özetleme eşiği. Kayda yazılmaz.
    BaglamDurumu {
        tahmin: u64,
        esik: u64,
    },
    /// Otomatik kip hakeminin kararı (tasarım §6).
    OtomatikKarar {
        cagri: CagriId,
        arac: String,
        aciklama: String,
        sonuc: OtomatikSonuc,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        risk: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        olasilik: Option<f64>,
    },
    /// Bu turda etkin olan proje talimatı (`AGENTS.md`) kaynakları ve uyarıları.
    ProjeTalimatlari {
        goruntu: crate::proje_talimatlari::ProjeTalimatAnlikGoruntusu,
    },
    /// Oturum çalışma planı güncellendi (`plan_guncelle`).
    PlanGuncellendi {
        plan: crate::plan::OturumPlani,
    },
    /// Kullanıcıya yapılandırılmış soru soruldu (`kullaniciya_sor`).
    KullaniciSorusuGerekiyor {
        soru: crate::soru::KullaniciSorusu,
    },
    /// Bekleyen kullanıcı sorusu yanıtlandı, yönlendirildi veya iptal edildi.
    KullaniciSorusuCozuldu {
        soru_kimligi: String,
        cagri: CagriId,
        sonuc: crate::soru::SoruSonucu,
    },
}

/// Gönderim önizlemesinin gösterilme nedeni.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnizlemeNedeni {
    Onizleme,
    Denetim,
    /// Eklentinin EVREN isteği (aşama 4 tasarımı §9.4).
    Eklenti,
}

/// Gönderim önizleme kartındaki bir öğe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnizlemeOgesi {
    pub cagri: CagriId,
    pub kaynak: String,
    pub bayt: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denetim: Option<String>,
    /// Eklenti isteğinin özeti; yalnızca `OnizlemeNedeni::Eklenti`'de.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eklenti: Option<crate::gonderim::EklentiIstekOzeti>,
}
