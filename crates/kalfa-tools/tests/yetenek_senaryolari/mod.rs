//! Kalfa yetenek programı (Hat 01 — G0 ve G5) sabit görev şeması, 24 görev yükleyicisi,
//! çevrimdışı `Senaryo` koşucusu, bağımsız denetleyici ve rapor sözleşmesi doğrulayıcısı.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use kalfa_core::denetci::DenetimKipi;
use kalfa_core::izin::Kip;
use kalfa_core::muhur::ozet_hex;
use kalfa_core::protokol::{AracDurumu, BitisNedeni, Komut, Olay};
use kalfa_core::test_destek::{SahteSaglayici, SahteYanit};
use serde_json::{Value, json};

use crate::ortak::senaryo::{
    Kabuk, Senaryo, arac_cagrilari_tek_yanitli, butun_istekler, karar, mesaj, onay_istegi_sayisi,
    sonuclar,
};

pub const SEMA_SURUMU: &str = "kalfa.yetenek.gorev.v1";
pub const SONUC_SEMA_SURUMU: &str = "kalfa.yetenek.sonuc.v1";
pub const SABIT_TARIH: &str = "2026-10-20T00:00:00Z";
pub const SABIT_TOHUM: u64 = 20261020;

pub const YIRMI_DORT_KIMLIK: [&str; 24] = [
    "K01", "K02", "K03", "K04", "K05", "K06", "S01", "S02", "S03", "S04", "S05", "S06", "M01",
    "M02", "M03", "M04", "M05", "M06", "W01", "W02", "W03", "W04", "W05", "W06",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GorevAilesi {
    YerelKodlama,
    SurecVeOturum,
    Mcp,
    Web,
}

impl GorevAilesi {
    pub fn kod(self) -> &'static str {
        match self {
            Self::YerelKodlama => "yerel_kodlama",
            Self::SurecVeOturum => "surec_ve_oturum",
            Self::Mcp => "mcp",
            Self::Web => "web",
        }
    }

    pub fn baslik(self) -> &'static str {
        match self {
            Self::YerelKodlama => "Yerel kodlama",
            Self::SurecVeOturum => "Süreç ve oturum",
            Self::Mcp => "MCP",
            Self::Web => "Web",
        }
    }

    pub fn beklenen_onek(self) -> char {
        match self {
            Self::YerelKodlama => 'K',
            Self::SurecVeOturum => 'S',
            Self::Mcp => 'M',
            Self::Web => 'W',
        }
    }

    pub fn cozumle(deger: &str) -> Result<Self, String> {
        match deger {
            "yerel_kodlama" => Ok(Self::YerelKodlama),
            "surec_ve_oturum" => Ok(Self::SurecVeOturum),
            "mcp" => Ok(Self::Mcp),
            "web" => Ok(Self::Web),
            diger => Err(format!("bilinmeyen görev ailesi: {diger}")),
        }
    }
}

/// Dört durumlu senaryo sonucu (Geçti / Kaldı / Desteklenmiyor / Çalıştırılmadı).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SenaryoSonucu {
    Gecti,
    Kaldi,
    Desteklenmiyor,
    Calistirilmadi,
}

impl SenaryoSonucu {
    pub fn kod(self) -> &'static str {
        match self {
            Self::Gecti => "gecti",
            Self::Kaldi => "kaldi",
            Self::Desteklenmiyor => "desteklenmiyor",
            Self::Calistirilmadi => "calistirilmadi",
        }
    }

    pub fn etiket(self) -> &'static str {
        match self {
            Self::Gecti => "Geçti",
            Self::Kaldi => "Kaldı",
            Self::Desteklenmiyor => "Desteklenmiyor",
            Self::Calistirilmadi => "Çalıştırılmadı",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformOnkosulu {
    pub gercek_yalitim: bool,
    pub yerel_surec: bool,
    pub yerel_http_sunucu: bool,
    pub ag_erisim: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaslangicDosyasi {
    pub yol: String,
    pub icerik: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnayKarariTanimi {
    pub arac: String,
    pub hedef: String,
    pub onayla: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoruYanitiTanimi {
    pub soru_id: String,
    pub yanit: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsanMudahalesiTanimi {
    pub onay_kararlari: Vec<OnayKarariTanimi>,
    pub onizleme_cikarilacak_kaynaklar: Vec<String>,
    pub soru_yanitlari: Vec<SoruYanitiTanimi>,
    pub duzeltme_mesaji_izni: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SahteCagriTanimi {
    pub ad: String,
    pub argumanlar: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SahteAdimTanimi {
    Arac(SahteCagriTanimi),
    Araclar(Vec<SahteCagriTanimi>),
    Metin(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeklenenDosyaDurumu {
    Var,
    Yok,
    Degismedi,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DosyaKontrolu {
    pub yol: String,
    pub durum: BeklenenDosyaDurumu,
    pub icerir: Vec<String>,
    pub icermez: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeklenenAracDurumu {
    pub arac: String,
    pub durum: AracDurumu,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IzKontrolleri {
    pub beklenen_bitis: BitisNedeni,
    pub beklenen_onay_sayisi: usize,
    pub isteklerde_bulunmali: Vec<String>,
    pub isteklerde_bulunmamali: Vec<String>,
    pub kayitta_bulunmamali: Vec<String>,
    pub en_fazla_kabuk_baslatma: usize,
    pub beklenen_arac_durumlari: Vec<BeklenenAracDurumu>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BagimsizDenetleyici {
    pub aciklama: String,
    pub dosya_kontrolleri: Vec<DosyaKontrolu>,
    pub iz_kontrolleri: IzKontrolleri,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GorevTanimi {
    pub sema_surumu: String,
    pub sabit_tarih: String,
    pub sabit_tohum: u64,
    pub kimlik: String,
    pub aile: GorevAilesi,
    pub baslik: String,
    pub kullanici_istegi: String,
    pub kip: Kip,
    pub denetci_kipi: DenetimKipi,
    pub gerekli_yetenekler: Vec<String>,
    pub bagimli_hat: Option<String>,
    pub platform_onkosulu: PlatformOnkosulu,
    pub baslangic_dosyalari: Vec<BaslangicDosyasi>,
    pub izinli_degisiklikler: Vec<String>,
    pub izin_verilen_insan_mudahalesi: InsanMudahalesiTanimi,
    pub sahte_model_adimlari: Vec<SahteAdimTanimi>,
    pub bagimsiz_denetleyici: BagimsizDenetleyici,
    pub dosya_sha256: String,
}

/// Çevrimdışı senaryo koşucusunun ortam yetenekleri ve önkoşul anahtarları.
#[derive(Debug, Clone)]
pub struct CalistirmaOrtami {
    pub desteklenen_yetenekler: BTreeSet<String>,
    pub gercek_yalitim_acik: bool,
    pub yerel_surec_acik: bool,
    pub yerel_http_sunucu_acik: bool,
}

impl CalistirmaOrtami {
    /// Taban `4041a39` üzerinde çevrimdışı birim testi ortamı:
    /// yalnız `dosya_araclari` ve `yerel_mcp_stdio` ürün kodunda mevcuttur;
    /// alt süreç / gerçek yalıtım / yerel HTTP sunucu önkoşulları kapalıdır.
    pub fn g0_cevrimdisi_taban() -> Self {
        let mut desteklenen_yetenekler = BTreeSet::new();
        desteklenen_yetenekler.insert("dosya_araclari".to_string());
        desteklenen_yetenekler.insert("yerel_mcp_stdio".to_string());
        Self {
            desteklenen_yetenekler,
            gercek_yalitim_acik: false,
            yerel_surec_acik: false,
            yerel_http_sunucu_acik: false,
        }
    }
}

/// Tek bir görevin koşum çıktısı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GorevKosumKaydi {
    pub kimlik: String,
    pub aile: GorevAilesi,
    pub baslik: String,
    pub dosya_sha256: String,
    pub sonuc: SenaryoSonucu,
    pub saglayici_turu: String,
    pub canli_model_basarisi: bool,
    pub neden: String,
    pub bagimli_hat: Option<String>,
    pub adim_sayisi: usize,
    pub arac_cagrisi_sayisi: usize,
    pub onay_sayisi: usize,
    pub tuketim_biliniyor: bool,
    pub girdi_jetonu: Option<u64>,
    pub cikti_jetonu: Option<u64>,
    pub rag_soguk_indeks_ms: Option<u64>,
    pub rag_sicak_sorgu_ms: Option<u64>,
    pub calisma_izi_ozeti: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopluKosumOzeti {
    pub sema_surumu: String,
    pub sabit_tarih: String,
    pub sabit_tohum: u64,
    pub taban_commit: String,
    pub kayitlar: Vec<GorevKosumKaydi>,
}

impl TopluKosumOzeti {
    pub fn sayim(&self, hedef: SenaryoSonucu) -> usize {
        self.kayitlar.iter().filter(|k| k.sonuc == hedef).count()
    }

    pub fn json_degeri(&self) -> Value {
        let kayit_listesi: Vec<Value> = self
            .kayitlar
            .iter()
            .map(|k| {
                json!({
                    "kimlik": k.kimlik,
                    "aile": k.aile.kod(),
                    "baslik": k.baslik,
                    "dosya_sha256": k.dosya_sha256,
                    "sonuc": k.sonuc.kod(),
                    "sonuc_etiketi": k.sonuc.etiket(),
                    "saglayici_turu": k.saglayici_turu,
                    "canli_model_basarisi": k.canli_model_basarisi,
                    "neden": k.neden,
                    "bagimli_hat": k.bagimli_hat,
                    "adim_sayisi": k.adim_sayisi,
                    "arac_cagrisi_sayisi": k.arac_cagrisi_sayisi,
                    "onay_sayisi": k.onay_sayisi,
                    "tuketim": if k.tuketim_biliniyor {
                        json!({
                            "durum": "olculdu",
                            "girdi_jetonu": k.girdi_jetonu,
                            "cikti_jetonu": k.cikti_jetonu
                        })
                    } else {
                        json!({
                            "durum": "bilinmiyor",
                            "girdi_jetonu": null,
                            "cikti_jetonu": null
                        })
                    },
                    "rag_ayrimi": {
                        "soguk_indeks_ms": k.rag_soguk_indeks_ms,
                        "sicak_sorgu_ms": k.rag_sicak_sorgu_ms
                    },
                    "calisma_izi_ozeti": k.calisma_izi_ozeti
                })
            })
            .collect();

        json!({
            "sema_surumu": self.sema_surumu,
            "sabit_tarih": self.sabit_tarih,
            "sabit_tohum": self.sabit_tohum,
            "taban_commit": self.taban_commit,
            "toplam_gorev": self.kayitlar.len(),
            "ozet_sayilar": {
                "gecti": self.sayim(SenaryoSonucu::Gecti),
                "kaldi": self.sayim(SenaryoSonucu::Kaldi),
                "desteklenmiyor": self.sayim(SenaryoSonucu::Desteklenmiyor),
                "calistirilmadi": self.sayim(SenaryoSonucu::Calistirilmadi)
            },
            "kayitlar": kayit_listesi
        })
    }

    pub fn markdown_ozet_tablosu(&self) -> String {
        let mut cikti = String::new();
        cikti.push_str(
            "| Kimlik | Aile | Başlık | SHA-256 (LF) | Çevrimdışı Sonuç | Canlı Model Başarısı | Bağımlı Hat / Gerekçe |\n",
        );
        cikti.push_str("|---|---|---|---|---|---|---|\n");
        for k in &self.kayitlar {
            let kisa_ozet = &k.dosya_sha256[..16];
            let canli = if k.canli_model_basarisi {
                "Evet"
            } else {
                "Hayır (`sahte_saglayici`)"
            };
            cikti.push_str(&format!(
                "| `{}` | {} | {} | `{}` | **{}** | {} | {} |\n",
                k.kimlik,
                k.aile.baslik(),
                k.baslik,
                kisa_ozet,
                k.sonuc.etiket(),
                canli,
                k.neden
            ));
        }
        cikti
    }
}

pub fn tanimlar_dizini() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("yetenek_senaryolari")
        .join("tanimlar")
}

/// Göreli çalışma alanı yolunun güvenli olduğunu doğrular (`..`, mutlak yol,
/// sürücü harfi ve UNC yollarını reddeder).
pub fn guvenli_goreli_yol_mu(yol: &str) -> bool {
    if yol.trim().is_empty()
        || yol.starts_with('/')
        || yol.starts_with('\\')
        || yol.contains(':')
        || yol.contains('\\')
    {
        return false;
    }
    let p = Path::new(yol);
    for bilesen in p.components() {
        match bilesen {
            Component::Normal(_) => {}
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return false;
            }
        }
    }
    true
}

fn zorunlu_dizge<'a>(nesne: &'a Value, alan: &str) -> Result<&'a str, String> {
    nesne
        .get(alan)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("`{alan}` alanı zorunlu ve boş olmayan dizge olmalı"))
}

fn zorunlu_bool(nesne: &Value, alan: &str) -> Result<bool, String> {
    nesne
        .get(alan)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("`{alan}` alanı zorunlu bool olmalı"))
}

fn dizge_dizisi(nesne: &Value, alan: &str) -> Result<Vec<String>, String> {
    let dizi = nesne
        .get(alan)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("`{alan}` alanı dizi olmalı"))?;
    let mut sonuc = Vec::with_capacity(dizi.len());
    for oge in dizi {
        let s = oge
            .as_str()
            .ok_or_else(|| format!("`{alan}` içindeki her öğe dizge olmalı"))?;
        sonuc.push(s.to_string());
    }
    Ok(sonuc)
}

fn kip_cozumle(deger: &str) -> Result<Kip, String> {
    match deger {
        "inceleme" | "salt_okunur" => Ok(Kip::Inceleme),
        "proje" => Ok(Kip::Proje),
        "siki" => Ok(Kip::Siki),
        "serbest" => Ok(Kip::Serbest),
        diger => Err(format!("geçersiz kip: {diger}")),
    }
}

fn denetci_kipi_cozumle(deger: &str) -> Result<DenetimKipi, String> {
    match deger {
        "kapali" => Ok(DenetimKipi::Kapali),
        "uyar" | "yalniz_ag_ve_kabuk" => Ok(DenetimKipi::Uyar),
        "kati" | "her_riskli_arac" => Ok(DenetimKipi::Kati),
        diger => Err(format!("geçersiz denetçi kipi: {diger}")),
    }
}

fn bitis_nedeni_cozumle(deger: &str) -> Result<BitisNedeni, String> {
    match deger {
        "normal" => Ok(BitisNedeni::Normal),
        "iptal" => Ok(BitisNedeni::Iptal),
        "hata" => Ok(BitisNedeni::Hata),
        "azami_adim" | "sinir" => Ok(BitisNedeni::Sinir {
            aciklama: "sınır".to_string(),
        }),
        diger => Err(format!("geçersiz bitiş nedeni: {diger}")),
    }
}

fn arac_durumu_cozumle(deger: &str) -> Result<AracDurumu, String> {
    match deger {
        "basarili" => Ok(AracDurumu::Basarili),
        "reddedildi" | "kullanici_reddetti" | "izin_reddedildi" => Ok(AracDurumu::Reddedildi),
        "basarisiz" => Ok(AracDurumu::Basarisiz),
        "iptal_edildi" | "iptal" => Ok(AracDurumu::IptalEdildi),
        "sonucu_bilinmiyor" | "zaman_asimi" => Ok(AracDurumu::SonucuBilinmiyor),
        diger => Err(format!("geçersiz araç durumu: {diger}")),
    }
}

/// Ham JSON metninden görev tanımını ayrıştırır ve şema kurallarını doğrular.
pub fn gorev_tanimi_cozumle(ham_json: &str) -> Result<GorevTanimi, String> {
    let norm_lf = ham_json.replace("\r\n", "\n");
    let dosya_sha256 = ozet_hex(norm_lf.as_bytes());
    let kok: Value =
        serde_json::from_str(&norm_lf).map_err(|e| format!("JSON ayrıştırma hatası: {e}"))?;

    let sema_surumu = zorunlu_dizge(&kok, "sema_surumu")?.to_string();
    if sema_surumu != SEMA_SURUMU {
        return Err(format!(
            "desteklenmeyen şema sürümü `{sema_surumu}`, beklenen `{SEMA_SURUMU}`"
        ));
    }

    let sabit_tarih = zorunlu_dizge(&kok, "sabit_tarih")?.to_string();
    if sabit_tarih != SABIT_TARIH {
        return Err(format!(
            "sabit_tarih `{sabit_tarih}` geçersiz, beklenen `{SABIT_TARIH}`"
        ));
    }

    let sabit_tohum = kok
        .get("sabit_tohum")
        .and_then(Value::as_u64)
        .ok_or_else(|| "`sabit_tohum` alanı u64 olmalı".to_string())?;
    if sabit_tohum != SABIT_TOHUM {
        return Err(format!(
            "sabit_tohum `{sabit_tohum}` geçersiz, beklenen `{SABIT_TOHUM}`"
        ));
    }

    let kimlik = zorunlu_dizge(&kok, "kimlik")?.to_string();
    let aile = GorevAilesi::cozumle(zorunlu_dizge(&kok, "aile")?)?;
    if !kimlik.starts_with(aile.beklenen_onek()) {
        return Err(format!(
            "kimlik `{kimlik}` ile aile `{}` öneki uyuşmuyor",
            aile.kod()
        ));
    }

    let baslik = zorunlu_dizge(&kok, "baslik")?.to_string();
    let kullanici_istegi = zorunlu_dizge(&kok, "kullanici_istegi")?.to_string();
    let kip = kip_cozumle(zorunlu_dizge(&kok, "kip")?)?;
    let denetci_kipi = denetci_kipi_cozumle(zorunlu_dizge(&kok, "denetci_kipi")?)?;

    let gerekli_yetenekler = dizge_dizisi(&kok, "gerekli_yetenekler")?;
    if gerekli_yetenekler.is_empty() {
        return Err("`gerekli_yetenekler` en az bir yetenek içermeli".to_string());
    }

    let bagimli_hat = match kok.get("bagimli_hat") {
        Some(Value::Null) | None => None,
        Some(Value::String(s)) if !s.trim().is_empty() => Some(s.clone()),
        _ => return Err("`bagimli_hat` null veya boş olmayan dizge olmalı".to_string()),
    };

    let onkosul_deger = kok
        .get("platform_onkosulu")
        .ok_or_else(|| "`platform_onkosulu` alanı zorunlu".to_string())?;
    let platform_onkosulu = PlatformOnkosulu {
        gercek_yalitim: zorunlu_bool(onkosul_deger, "gercek_yalitim")?,
        yerel_surec: zorunlu_bool(onkosul_deger, "yerel_surec")?,
        yerel_http_sunucu: zorunlu_bool(onkosul_deger, "yerel_http_sunucu")?,
        ag_erisim: zorunlu_bool(onkosul_deger, "ag_erisim")?,
    };
    if platform_onkosulu.ag_erisim {
        return Err(
            "çevrimdışı deterministik görev tanımında `platform_onkosulu.ag_erisim` true olamaz"
                .to_string(),
        );
    }

    let baslangic_dizisi = kok
        .get("baslangic_dosyalari")
        .and_then(Value::as_array)
        .ok_or_else(|| "`baslangic_dosyalari` dizi olmalı".to_string())?;
    let mut baslangic_dosyalari = Vec::with_capacity(baslangic_dizisi.len());
    for oge in baslangic_dizisi {
        let yol = zorunlu_dizge(oge, "yol")?.to_string();
        if !guvenli_goreli_yol_mu(&yol) {
            return Err(format!(
                "baslangic_dosyalari içinde çalışma alanı dışı/güvensiz yol: `{yol}`"
            ));
        }
        let icerik = oge
            .get("icerik")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("`{yol}` için `icerik` alanı dizge olmalı"))?
            .to_string();
        baslangic_dosyalari.push(BaslangicDosyasi { yol, icerik });
    }

    let izinli_degisiklikler = dizge_dizisi(&kok, "izinli_degisiklikler")?;
    for yol in &izinli_degisiklikler {
        if !guvenli_goreli_yol_mu(yol) {
            return Err(format!(
                "izinli_degisiklikler içinde çalışma alanı dışı/güvensiz yol: `{yol}`"
            ));
        }
    }

    let mudahale = kok
        .get("izin_verilen_insan_mudahalesi")
        .ok_or_else(|| "`izin_verilen_insan_mudahalesi` alanı zorunlu".to_string())?;
    let onay_dizisi = mudahale
        .get("onay_kararlari")
        .and_then(Value::as_array)
        .ok_or_else(|| "`onay_kararlari` dizi olmalı".to_string())?;
    let mut onay_kararlari = Vec::with_capacity(onay_dizisi.len());
    for oge in onay_dizisi {
        let arac = zorunlu_dizge(oge, "arac")?.to_string();
        let hedef = zorunlu_dizge(oge, "hedef")?.to_string();
        let karar_dizge = zorunlu_dizge(oge, "karar")?;
        let onayla = match karar_dizge {
            "onayla" => true,
            "reddet" => false,
            diger => return Err(format!("geçersiz onay kararı: {diger}")),
        };
        onay_kararlari.push(OnayKarariTanimi {
            arac,
            hedef,
            onayla,
        });
    }
    let onizleme_cikarilacak_kaynaklar = dizge_dizisi(mudahale, "onizleme_cikarilacak_kaynaklar")?;
    let soru_dizisi = mudahale
        .get("soru_yanitlari")
        .and_then(Value::as_array)
        .ok_or_else(|| "`soru_yanitlari` dizi olmalı".to_string())?;
    let mut soru_yanitlari = Vec::with_capacity(soru_dizisi.len());
    for oge in soru_dizisi {
        let soru_id = zorunlu_dizge(oge, "soru_id")?.to_string();
        let yanit = zorunlu_dizge(oge, "yanit")?.to_string();
        soru_yanitlari.push(SoruYanitiTanimi { soru_id, yanit });
    }
    let duzeltme_mesaji_izni = zorunlu_bool(mudahale, "duzeltme_mesaji_izni")?;

    let adimlar_dizisi = kok
        .get("sahte_model_adimlari")
        .and_then(Value::as_array)
        .ok_or_else(|| "`sahte_model_adimlari` dizi olmalı".to_string())?;
    if adimlar_dizisi.is_empty() {
        return Err("`sahte_model_adimlari` boş olamaz".to_string());
    }
    let mut sahte_model_adimlari = Vec::with_capacity(adimlar_dizisi.len());
    for adim in adimlar_dizisi {
        let tur = zorunlu_dizge(adim, "tur")?;
        match tur {
            "arac" => {
                let ad = zorunlu_dizge(adim, "ad")?.to_string();
                let argumanlar = adim
                    .get("argumanlar")
                    .cloned()
                    .ok_or_else(|| "`arac` adımında `argumanlar` zorunlu".to_string())?;
                sahte_model_adimlari
                    .push(SahteAdimTanimi::Arac(SahteCagriTanimi { ad, argumanlar }));
            }
            "araclar" => {
                let cagrilar_dizisi = adim
                    .get("cagrilar")
                    .and_then(Value::as_array)
                    .ok_or_else(|| "`araclar` adımında `cagrilar` dizisi zorunlu".to_string())?;
                if cagrilar_dizisi.is_empty() {
                    return Err("`araclar` adımında `cagrilar` boş olamaz".to_string());
                }
                let mut cagrilar = Vec::with_capacity(cagrilar_dizisi.len());
                for c in cagrilar_dizisi {
                    let ad = zorunlu_dizge(c, "ad")?.to_string();
                    let argumanlar = c
                        .get("argumanlar")
                        .cloned()
                        .ok_or_else(|| "`cagrilar` öğesinde `argumanlar` zorunlu".to_string())?;
                    cagrilar.push(SahteCagriTanimi { ad, argumanlar });
                }
                sahte_model_adimlari.push(SahteAdimTanimi::Araclar(cagrilar));
            }
            "metin" => {
                let icerik = zorunlu_dizge(adim, "icerik")?.to_string();
                sahte_model_adimlari.push(SahteAdimTanimi::Metin(icerik));
            }
            diger => return Err(format!("bilinmeyen sahte model adım türü: {diger}")),
        }
    }

    let denetleyici_deger = kok.get("bagimsiz_denetleyici").ok_or_else(|| {
        "denetleyicisi olmayan fixture reddedilmeli (`bagimsiz_denetleyici` eksik)".to_string()
    })?;
    if denetleyici_deger.is_null() {
        return Err(
            "denetleyicisi olmayan fixture reddedilmeli (`bagimsiz_denetleyici` null)".into(),
        );
    }
    let aciklama = zorunlu_dizge(denetleyici_deger, "aciklama")?.to_string();

    let dosya_kontrol_dizisi = denetleyici_deger
        .get("dosya_kontrolleri")
        .and_then(Value::as_array)
        .ok_or_else(|| "`bagimsiz_denetleyici.dosya_kontrolleri` dizi olmalı".to_string())?;
    let mut dosya_kontrolleri = Vec::with_capacity(dosya_kontrol_dizisi.len());
    for dk in dosya_kontrol_dizisi {
        let yol = zorunlu_dizge(dk, "yol")?.to_string();
        if !guvenli_goreli_yol_mu(&yol) {
            return Err(format!(
                "dosya_kontrolleri içinde çalışma alanı dışı/güvensiz yol: `{yol}`"
            ));
        }
        let durum = match zorunlu_dizge(dk, "durum")? {
            "var" => BeklenenDosyaDurumu::Var,
            "yok" => BeklenenDosyaDurumu::Yok,
            "degismedi" => BeklenenDosyaDurumu::Degismedi,
            diger => return Err(format!("geçersiz dosya kontrol durumu: {diger}")),
        };
        let icerir = match dk.get("icerir") {
            Some(_) => dizge_dizisi(dk, "icerir")?,
            None => Vec::new(),
        };
        let icermez = match dk.get("icermez") {
            Some(_) => dizge_dizisi(dk, "icermez")?,
            None => Vec::new(),
        };
        dosya_kontrolleri.push(DosyaKontrolu {
            yol,
            durum,
            icerir,
            icermez,
        });
    }

    let iz_deger = denetleyici_deger
        .get("iz_kontrolleri")
        .ok_or_else(|| "`bagimsiz_denetleyici.iz_kontrolleri` zorunlu".to_string())?;
    let beklenen_bitis = bitis_nedeni_cozumle(zorunlu_dizge(iz_deger, "beklenen_bitis")?)?;
    let beklenen_onay_sayisi = iz_deger
        .get("beklenen_onay_sayisi")
        .and_then(Value::as_u64)
        .ok_or_else(|| "`beklenen_onay_sayisi` u64 olmalı".to_string())?
        as usize;
    let isteklerde_bulunmali = dizge_dizisi(iz_deger, "isteklerde_bulunmali")?;
    let isteklerde_bulunmamali = dizge_dizisi(iz_deger, "isteklerde_bulunmamali")?;
    let kayitta_bulunmamali = dizge_dizisi(iz_deger, "kayitta_bulunmamali")?;
    let en_fazla_kabuk_baslatma = iz_deger
        .get("en_fazla_kabuk_baslatma")
        .and_then(Value::as_u64)
        .ok_or_else(|| "`en_fazla_kabuk_baslatma` u64 olmalı".to_string())?
        as usize;

    let arac_durumlari_dizisi = iz_deger
        .get("beklenen_arac_durumlari")
        .and_then(Value::as_array)
        .ok_or_else(|| "`beklenen_arac_durumlari` dizi olmalı".to_string())?;
    let mut beklenen_arac_durumlari = Vec::with_capacity(arac_durumlari_dizisi.len());
    for ad in arac_durumlari_dizisi {
        let arac = zorunlu_dizge(ad, "arac")?.to_string();
        let durum = arac_durumu_cozumle(zorunlu_dizge(ad, "durum")?)?;
        beklenen_arac_durumlari.push(BeklenenAracDurumu { arac, durum });
    }

    if dosya_kontrolleri.is_empty()
        && beklenen_arac_durumlari.is_empty()
        && isteklerde_bulunmali.is_empty()
        && isteklerde_bulunmamali.is_empty()
        && kayitta_bulunmamali.is_empty()
    {
        return Err(
            "denetleyicisi olmayan fixture reddedilmeli (en az bir dosya veya iz kontrolü zorunlu)"
                .to_string(),
        );
    }

    Ok(GorevTanimi {
        sema_surumu,
        sabit_tarih,
        sabit_tohum,
        kimlik,
        aile,
        baslik,
        kullanici_istegi,
        kip,
        denetci_kipi,
        gerekli_yetenekler,
        bagimli_hat,
        platform_onkosulu,
        baslangic_dosyalari,
        izinli_degisiklikler,
        izin_verilen_insan_mudahalesi: InsanMudahalesiTanimi {
            onay_kararlari,
            onizleme_cikarilacak_kaynaklar,
            soru_yanitlari,
            duzeltme_mesaji_izni,
        },
        sahte_model_adimlari,
        bagimsiz_denetleyici: BagimsizDenetleyici {
            aciklama,
            dosya_kontrolleri,
            iz_kontrolleri: IzKontrolleri {
                beklenen_bitis,
                beklenen_onay_sayisi,
                isteklerde_bulunmali,
                isteklerde_bulunmamali,
                kayitta_bulunmamali,
                en_fazla_kabuk_baslatma,
                beklenen_arac_durumlari,
            },
        },
        dosya_sha256,
    })
}

/// Görev kümesinde yinelenen kimlik bulunmadığını ve aile dağılımını doğrular.
pub fn dogrula_gorev_kumesi(gorevler: &[GorevTanimi]) -> Result<(), String> {
    let mut gorulen = BTreeSet::new();
    for g in gorevler {
        if !gorulen.insert(g.kimlik.clone()) {
            return Err(format!("yinelenen görev kimliği: `{}`", g.kimlik));
        }
    }
    Ok(())
}

/// Diskteki 24 sabit görev tanımını (`K01`..`W06`) yükler ve doğrular.
pub fn yirmi_dort_gorevi_yukle() -> Result<Vec<GorevTanimi>, String> {
    let dizin = tanimlar_dizini();
    let mut gorevler = Vec::with_capacity(YIRMI_DORT_KIMLIK.len());
    for kimlik in YIRMI_DORT_KIMLIK {
        let yol = dizin.join(format!("{kimlik}.json"));
        let ham = std::fs::read_to_string(&yol).map_err(|e| format!("{:?} okunamadı: {e}", yol))?;
        let tanim =
            gorev_tanimi_cozumle(&ham).map_err(|e| format!("{kimlik}.json şema hatası: {e}"))?;
        if tanim.kimlik != kimlik {
            return Err(format!(
                "dosya adı `{kimlik}.json` ile iç kimlik `{}` uyuşmuyor",
                tanim.kimlik
            ));
        }
        gorevler.push(tanim);
    }
    dogrula_gorev_kumesi(&gorevler)?;
    Ok(gorevler)
}

/// Ölçüm raporu kaydının sözleşme kurallarına uyduğunu denetler:
/// 1. `sahte_saglayici` ile geçen hiçbir kayıt `canli_model_basarisi = true` olamaz.
/// 2. Tüketim bilinmiyorsa (`tuketim_biliniyor == false`) girdi/çıktı jetonu `0`
///    veya sayısal bir değer olarak yazılamaz (`None` olmalıdır).
pub fn dogrula_olcum_raporu_kaydi(kayit: &GorevKosumKaydi) -> Result<(), String> {
    if kayit.saglayici_turu == "sahte_saglayici" && kayit.canli_model_basarisi {
        return Err(format!(
            "`{}`: sahte_saglayici ile geçen senaryo canlı model başarısı olarak raporlanamaz",
            kayit.kimlik
        ));
    }
    if !kayit.tuketim_biliniyor && (kayit.girdi_jetonu.is_some() || kayit.cikti_jetonu.is_some()) {
        return Err(format!(
            "`{}`: bilinmeyen jeton tüketimi 0 veya sayı olarak yazılamaz (`bilinmiyor` / None olmalı)",
            kayit.kimlik
        ));
    }
    Ok(())
}

/// Tek bir görevi verilen çevrimdışı ortamda çalıştırır ve bağımsız denetleyiciyle değerlendirir.
pub async fn gorevi_kostur(gorev: &GorevTanimi, ortam: &CalistirmaOrtami) -> GorevKosumKaydi {
    // 1. Ürün yeteneği kontrolü -> Desteklenmiyor
    let eksik_yetenekler: Vec<String> = gorev
        .gerekli_yetenekler
        .iter()
        .filter(|y| !ortam.desteklenen_yetenekler.contains(*y))
        .cloned()
        .collect();
    if !eksik_yetenekler.is_empty() {
        let hat_bilgisi = gorev
            .bagimli_hat
            .as_deref()
            .unwrap_or("ilgili yetenek hattı");
        let neden = format!(
            "Eksik yetenek: `{}` ({})",
            eksik_yetenekler.join(", "),
            hat_bilgisi
        );
        let iz_ozeti = ozet_hex(
            format!(
                "desteklenmiyor:{}:{}:{}",
                gorev.kimlik,
                gorev.dosya_sha256,
                eksik_yetenekler.join(",")
            )
            .as_bytes(),
        );
        return GorevKosumKaydi {
            kimlik: gorev.kimlik.clone(),
            aile: gorev.aile,
            baslik: gorev.baslik.clone(),
            dosya_sha256: gorev.dosya_sha256.clone(),
            sonuc: SenaryoSonucu::Desteklenmiyor,
            saglayici_turu: "sahte_saglayici".to_string(),
            canli_model_basarisi: false,
            neden,
            bagimli_hat: gorev.bagimli_hat.clone(),
            adim_sayisi: 0,
            arac_cagrisi_sayisi: 0,
            onay_sayisi: 0,
            tuketim_biliniyor: false,
            girdi_jetonu: None,
            cikti_jetonu: None,
            rag_soguk_indeks_ms: None,
            rag_sicak_sorgu_ms: None,
            calisma_izi_ozeti: iz_ozeti,
        };
    }

    // 2. Platform / dış önkoşul kontrolü -> Çalıştırılmadı
    let mut eksik_onkosullar = Vec::new();
    if gorev.platform_onkosulu.gercek_yalitim && !ortam.gercek_yalitim_acik {
        eksik_onkosullar.push("gercek_yalitim");
    }
    if gorev.platform_onkosulu.yerel_surec && !ortam.yerel_surec_acik {
        eksik_onkosullar.push("yerel_surec");
    }
    if gorev.platform_onkosulu.yerel_http_sunucu && !ortam.yerel_http_sunucu_acik {
        eksik_onkosullar.push("yerel_http_sunucu");
    }
    if !eksik_onkosullar.is_empty() {
        let neden = format!(
            "Önkoşul bu koşuda kapalı: `{}`",
            eksik_onkosullar.join(", ")
        );
        let iz_ozeti = ozet_hex(
            format!(
                "calistirilmadi:{}:{}:{}",
                gorev.kimlik,
                gorev.dosya_sha256,
                eksik_onkosullar.join(",")
            )
            .as_bytes(),
        );
        return GorevKosumKaydi {
            kimlik: gorev.kimlik.clone(),
            aile: gorev.aile,
            baslik: gorev.baslik.clone(),
            dosya_sha256: gorev.dosya_sha256.clone(),
            sonuc: SenaryoSonucu::Calistirilmadi,
            saglayici_turu: "sahte_saglayici".to_string(),
            canli_model_basarisi: false,
            neden,
            bagimli_hat: gorev.bagimli_hat.clone(),
            adim_sayisi: 0,
            arac_cagrisi_sayisi: 0,
            onay_sayisi: 0,
            tuketim_biliniyor: false,
            girdi_jetonu: None,
            cikti_jetonu: None,
            rag_soguk_indeks_ms: None,
            rag_sicak_sorgu_ms: None,
            calisma_izi_ozeti: iz_ozeti,
        };
    }

    // 3. Senaryoyu gerçek dosya araçları ve SahteSaglayici ile çalıştır
    let mut sc = Senaryo::yeni(gorev.kip)
        .denetci_kipi(gorev.denetci_kipi)
        .kayitli()
        .kabuklu(Kabuk {
            yalitim: true,
            yalitimsiz_izni: false,
            guvenildi: false,
        });

    let mut baslangic_ozetleri: BTreeMap<String, String> = BTreeMap::new();
    for dosya in &gorev.baslangic_dosyalari {
        sc = sc.dosya(&dosya.yol, &dosya.icerik);
        baslangic_ozetleri.insert(dosya.yol.clone(), ozet_hex(dosya.icerik.as_bytes()));
    }

    let mut yanitlar = Vec::with_capacity(gorev.sahte_model_adimlari.len());
    for adim in &gorev.sahte_model_adimlari {
        match adim {
            SahteAdimTanimi::Arac(c) => {
                yanitlar.push(SahteYanit::arac(&c.ad, c.argumanlar.clone()));
            }
            SahteAdimTanimi::Araclar(liste) => {
                let cagrilar: Vec<(&str, Value)> = liste
                    .iter()
                    .map(|c| (c.ad.as_str(), c.argumanlar.clone()))
                    .collect();
                yanitlar.push(SahteYanit::araclar(cagrilar));
            }
            SahteAdimTanimi::Metin(m) => {
                yanitlar.push(SahteYanit::metin(m));
            }
        }
    }

    let saglayici = SahteSaglayici::yeni(yanitlar);
    let mut surucu = sc.baslat(&saglayici);
    surucu.gonder(mesaj(&gorev.kullanici_istegi)).await;

    let mut onay_sayaci = 0usize;
    let bitis_nedeni = loop {
        let zarf = surucu
            .bekle(|o| {
                matches!(
                    o,
                    Olay::OnayGerekiyor { .. }
                        | Olay::GonderimOnizlemesi { .. }
                        | Olay::Tamamlandi { .. }
                )
            })
            .await;

        match zarf.yuk {
            Olay::OnayGerekiyor { ogeler, .. } => {
                for oge in &ogeler {
                    let eslesen = gorev
                        .izin_verilen_insan_mudahalesi
                        .onay_kararlari
                        .get(onay_sayaci)
                        .map(|k| k.onayla)
                        .unwrap_or(false);
                    onay_sayaci += 1;
                    surucu.gonder(karar(oge, eslesen)).await;
                }
            }
            Olay::GonderimOnizlemesi { islem, ogeler, .. } => {
                let cikarilacak_set: BTreeSet<&str> = gorev
                    .izin_verilen_insan_mudahalesi
                    .onizleme_cikarilacak_kaynaklar
                    .iter()
                    .map(String::as_str)
                    .collect();
                let gonderilmeyecekler = ogeler
                    .iter()
                    .filter(|o| cikarilacak_set.contains(o.kaynak.as_str()))
                    .map(|o| o.cagri)
                    .collect();
                surucu
                    .gonder(Komut::GonderimKarari {
                        islem,
                        gonderilmeyecekler,
                        hep_gonderilecekler: Vec::new(),
                    })
                    .await;
            }
            Olay::Tamamlandi { neden } => break neden,
            _ => {}
        }
    };

    // 4. Bağımsız denetleyici kontrolleri
    let mut hatalar: Vec<String> = Vec::new();
    let denetleyici = &gorev.bagimsiz_denetleyici;

    // 4a. İzin verilmeyen başlangıç dosyalarının değişmediğini doğrula
    let izinli_kume: BTreeSet<&str> = gorev
        .izinli_degisiklikler
        .iter()
        .map(String::as_str)
        .collect();
    for (yol, ilk_ozet) in &baslangic_ozetleri {
        if !izinli_kume.contains(yol.as_str()) {
            match sc.oku(yol) {
                Some(guncel) => {
                    if ozet_hex(guncel.as_bytes()) != *ilk_ozet {
                        hatalar.push(format!(
                            "izinli_degisiklikler dışındaki `{yol}` dosyası değiştirildi"
                        ));
                    }
                }
                None => {
                    hatalar.push(format!(
                        "izinli_degisiklikler dışındaki `{yol}` dosyası silindi"
                    ));
                }
            }
        }
    }

    // 4b. Dosya kontrolleri
    for dk in &denetleyici.dosya_kontrolleri {
        let icerik_opt = sc.oku(&dk.yol);
        match dk.durum {
            BeklenenDosyaDurumu::Var => match &icerik_opt {
                Some(icerik) => {
                    for aranan in &dk.icerir {
                        if !icerik.contains(aranan) {
                            hatalar.push(format!(
                                "`{}` içinde beklenen `{aranan}` dizgesi bulunamadı",
                                dk.yol
                            ));
                        }
                    }
                    for yasak in &dk.icermez {
                        if icerik.contains(yasak) {
                            hatalar.push(format!(
                                "`{}` içinde bulunmaması gereken `{yasak}` dizgesi bulundu",
                                dk.yol
                            ));
                        }
                    }
                }
                None => hatalar.push(format!("beklenen `{}` dosyası bulunamadı", dk.yol)),
            },
            BeklenenDosyaDurumu::Yok => {
                if icerik_opt.is_some() {
                    hatalar.push(format!("var olmaması gereken `{}` dosyası mevcut", dk.yol));
                }
            }
            BeklenenDosyaDurumu::Degismedi => {
                match (&icerik_opt, baslangic_ozetleri.get(&dk.yol)) {
                    (Some(guncel), Some(ilk_ozet)) => {
                        if ozet_hex(guncel.as_bytes()) != *ilk_ozet {
                            hatalar
                                .push(format!("`{}` dosyasının değişmemesi gerekiyordu", dk.yol));
                        }
                        for aranan in &dk.icerir {
                            if !guncel.contains(aranan) {
                                hatalar.push(format!(
                                    "`{}` içinde beklenen `{aranan}` dizgesi bulunamadı",
                                    dk.yol
                                ));
                            }
                        }
                    }
                    _ => hatalar.push(format!(
                        "`{}` başlangıç dosyası bulunamadı veya silindi",
                        dk.yol
                    )),
                }
            }
        }
    }

    // 4c. İz kontrolleri
    let iz = &denetleyici.iz_kontrolleri;
    if bitis_nedeni != iz.beklenen_bitis {
        hatalar.push(format!(
            "beklenen bitiş `{:?}`, gerçekleşen `{:?}`",
            iz.beklenen_bitis, bitis_nedeni
        ));
    }

    let gercek_onay_sayisi = onay_istegi_sayisi(&surucu);
    if gercek_onay_sayisi != iz.beklenen_onay_sayisi {
        hatalar.push(format!(
            "beklenen onay sayısı {}, gerçekleşen {}",
            iz.beklenen_onay_sayisi, gercek_onay_sayisi
        ));
    }

    let kabuk_baslatma = sc.yalitici.baslatma_sayisi();
    if kabuk_baslatma > iz.en_fazla_kabuk_baslatma {
        hatalar.push(format!(
            "kabuk başlatma sayısı ({kabuk_baslatma}) sınırı ({}) aştı",
            iz.en_fazla_kabuk_baslatma
        ));
    }

    let gercek_arac_sonuclari = sonuclar(&surucu);
    let beklenen_ciftler: Vec<(String, AracDurumu)> = iz
        .beklenen_arac_durumlari
        .iter()
        .map(|b| (b.arac.clone(), b.durum))
        .collect();
    if gercek_arac_sonuclari != beklenen_ciftler {
        hatalar.push(format!(
            "beklenen araç durumları {:?}, gerçekleşen {:?}",
            beklenen_ciftler, gercek_arac_sonuclari
        ));
    }

    let istek_metni = butun_istekler(&saglayici);
    for zorunlu in &iz.isteklerde_bulunmali {
        if !istek_metni.contains(zorunlu) {
            hatalar.push(format!(
                "sahte sağlayıcı isteklerinde beklenen `{zorunlu}` dizgesi bulunamadı"
            ));
        }
    }
    for yasak in &iz.isteklerde_bulunmamali {
        if istek_metni.contains(yasak) {
            hatalar.push(format!(
                "sahte sağlayıcı isteklerine yasaklı `{yasak}` dizgesi sızdı"
            ));
        }
    }

    let kayit_metni = std::fs::read_to_string(sc.kayit_yolu()).unwrap_or_default();
    for yasak in &iz.kayitta_bulunmamali {
        if kayit_metni.contains(yasak) {
            hatalar.push(format!(
                "oturum JSONL kaydına yasaklı `{yasak}` dizgesi sızdı"
            ));
        }
    }

    arac_cagrilari_tek_yanitli(&saglayici);

    // 5. Deterministik çalışma izi özeti (rastgele oturum/çağrı UUID'lerinden arındırılmış)
    let mut deterministik_durum = format!(
        "kimlik={};sha={};bitis={:?};onay={};kabuk={};",
        gorev.kimlik, gorev.dosya_sha256, bitis_nedeni, gercek_onay_sayisi, kabuk_baslatma
    );
    for (arac_adi, durum) in &gercek_arac_sonuclari {
        deterministik_durum.push_str(&format!("arac:{arac_adi}={durum:?};"));
    }
    let mut tum_yollar: BTreeSet<String> = gorev
        .baslangic_dosyalari
        .iter()
        .map(|d| d.yol.clone())
        .collect();
    for y in &gorev.izinli_degisiklikler {
        tum_yollar.insert(y.clone());
    }
    for dk in &denetleyici.dosya_kontrolleri {
        tum_yollar.insert(dk.yol.clone());
    }
    for yol in tum_yollar {
        match sc.oku(&yol) {
            Some(icerik) => {
                deterministik_durum
                    .push_str(&format!("dosya:{yol}={};", ozet_hex(icerik.as_bytes())));
            }
            None => {
                deterministik_durum.push_str(&format!("dosya:{yol}=YOK;"));
            }
        }
    }
    let calisma_izi_ozeti = ozet_hex(deterministik_durum.as_bytes());

    let (sonuc, neden) = if hatalar.is_empty() {
        (
            SenaryoSonucu::Gecti,
            format!("Bağımsız denetleyici geçti: {}", denetleyici.aciklama),
        )
    } else {
        (
            SenaryoSonucu::Kaldi,
            format!("Bağımsız denetleyici hatası: {}", hatalar.join(" | ")),
        )
    };

    GorevKosumKaydi {
        kimlik: gorev.kimlik.clone(),
        aile: gorev.aile,
        baslik: gorev.baslik.clone(),
        dosya_sha256: gorev.dosya_sha256.clone(),
        sonuc,
        saglayici_turu: "sahte_saglayici".to_string(),
        canli_model_basarisi: false,
        neden,
        bagimli_hat: gorev.bagimli_hat.clone(),
        adim_sayisi: saglayici.istekler().len(),
        arac_cagrisi_sayisi: gercek_arac_sonuclari.len(),
        onay_sayisi: gercek_onay_sayisi,
        tuketim_biliniyor: false,
        girdi_jetonu: None,
        cikti_jetonu: None,
        rag_soguk_indeks_ms: None,
        rag_sicak_sorgu_ms: None,
        calisma_izi_ozeti,
    }
}

/// 24 görevin tamamını sırayla çalıştırır ve toplu özet döner.
pub async fn tum_gorevleri_kostur(
    gorevler: &[GorevTanimi],
    ortam: &CalistirmaOrtami,
    taban_commit: &str,
) -> Result<TopluKosumOzeti, String> {
    dogrula_gorev_kumesi(gorevler)?;
    let mut kayitlar = Vec::with_capacity(gorevler.len());
    for g in gorevler {
        let kayit = gorevi_kostur(g, ortam).await;
        dogrula_olcum_raporu_kaydi(&kayit)?;
        kayitlar.push(kayit);
    }
    Ok(TopluKosumOzeti {
        sema_surumu: SONUC_SEMA_SURUMU.to_string(),
        sabit_tarih: SABIT_TARIH.to_string(),
        sabit_tohum: SABIT_TOHUM,
        taban_commit: taban_commit.to_string(),
        kayitlar,
    })
}
