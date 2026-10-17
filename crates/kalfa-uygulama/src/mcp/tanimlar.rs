//! MCP tanım hazırlığı, yerel onaylı kayıt defteri, dosya seçici kimliği ve
//! toplu JSON içe aktarma.
//!
//! Bu modül Görev 2'nin onay sonrası dosya özeti (`dosyayi_ozetle`,
//! `tanim_ozeti_hesapla`) kapılarını korur ve Görev 3'ün yerel kayıt sınırlarını
//! (`McpKayitDefteri`) ekler:
//! - Yürütülebilir dosya seçici arka uçta opak kimlik üretir; arayüz yalnız
//!   dosya adını (`gorunen_ad`) ve kimliği görür, ham yol gönderilmez.
//! - Her kayıt (ekleme, düzenleme, toplu içe aktarma) tam çözülmüş komut
//!   satırını, çalışma kipini ve ağ listesini gösteren yerel onay kutusundan
//!   (`Iletisim::onayla`) geçer; gizli değerler (hem yeni hem korunan sırlar)
//!   onay metnine girmeden önce maskelenir.
//! - Onay kutusundan dönüldüğünde, aday dosyalar açılıp özetlenmeden (`spawn_blocking`)
//!   **önce** ve kasa/ayar kaydından **önce** taslak kuşağı (`kusak`) ile ayar
//!   revizyonu yeniden doğrulanır; onay beklerken değiştirilen taslak eski
//!   onayla özetlenmez ve kaydedilmez.
//! - JSON içe aktarma arka uçta ayrıştırılır: `url` içeren girişler `"1.1b'de"`
//!   notuyla atlanır, mutlak `command` yolları arka uç kimliğine çevrilir,
//!   ortam değişkenleri varsayılan olarak gizli (`>= 8` karakter) kabul edilir
//!   ve sunucular kapalı (`etkin: false`) + yalıtımlı (`Yalitimli`) eklenir.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use kalfa_core::arac::McpTanimOzeti;
use kalfa_mcp::maske::{GizliDeger, maskele_metin};
use kalfa_mcp::ortam::OrtamDegeri;
use kalfa_mcp::tanim::{
    AgIzinKurali, CozulmusTanim, DosyaAdayi, Komut, McpTanim, SeciliYurutulebilirKimligi,
    TanimHatasi, YazilabilirKokler,
};
use kalfa_mcp::tarif::{DosyaIcerikOzeti, dosya_akisi_ozeti, tanim_ozeti};
use kalfa_sandbox::CalistirmaKipi;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use crate::arayuz::Iletisim;
use crate::mcp::ayarlar::{
    McpAyarlar, McpAyarlarDeposu, McpAyarlarHatasi, McpAyarlarIslemi, McpDegisikligi, McpKipi,
    McpKomut, McpOrtamDegiskeniDto, McpOrtamGirdisi, McpSunucuTaslagi,
    VARSAYILAN_BASLATMA_ZAMAN_ASIMI_SN, VARSAYILAN_CAGRI_ZAMAN_ASIMI_SN, YeniGizliDeger,
    sunucu_adi_gecerli,
};
use crate::mcp::kasa::{GizliDegerHatasi, GizliKasa, GizliKimlik, KasaHatasi, gizli_deger_dogrula};

/// `url` içeren MCP sunucu girişlerinin 1.1a'da atlanma nedeni.
pub const URL_ATLAMA_NEDENI: &str = "1.1b'de";

const KAYIT_ONAY_BASLIGI: &str = "MCP sunucusunu kaydet";
const ICE_AKTAR_ONAY_BASLIGI: &str = "MCP sunucularını içe aktar";
const DOSYA_SEC_BASLIGI: &str = "MCP sunucu yürütülebilirini seç";

/// Yalnız yerel kayıt onayından sonra veya onaylı tanımın başlatma/yeniden
/// başlatma hazırlığında çağrılır. İlk yerel onaydan önce argüman dosyaları
/// bu fonksiyonla açılmaz.
pub fn dosyayi_ozetle(aday: &DosyaAdayi) -> Result<DosyaIcerikOzeti, TanimHatasi> {
    let dosya = File::open(aday.yol()).map_err(|_| TanimHatasi::DosyaOkunamadi)?;
    if !aday
        .kimlik_uyuyor(&dosya)
        .map_err(|_| TanimHatasi::CiktiYoluDegisti)?
    {
        return Err(TanimHatasi::CiktiYoluDegisti);
    }
    let sha256 = dosya_akisi_ozeti(dosya).map_err(|_| TanimHatasi::DosyaOkunamadi)?;
    let tekrar = File::open(aday.yol()).map_err(|_| TanimHatasi::CiktiYoluDegisti)?;
    if !aday
        .kimlik_uyuyor(&tekrar)
        .map_err(|_| TanimHatasi::CiktiYoluDegisti)?
    {
        return Err(TanimHatasi::CiktiYoluDegisti);
    }
    Ok(DosyaIcerikOzeti::yeni(aday.yol().to_owned(), sha256))
}

/// Dosya özetlerini bloklayıcı iş parçacığında üretip `T` özetini hesaplar.
pub async fn tanim_ozeti_hesapla(
    tanim: McpTanim,
    cozulmus: CozulmusTanim,
    alan_kimligi: String,
) -> Result<McpTanimOzeti, TanimHatasi> {
    tokio::task::spawn_blocking(move || {
        let mut dosyalar = Vec::with_capacity(cozulmus.dosya_adaylari().len());
        for aday in cozulmus.dosya_adaylari() {
            dosyalar.push(dosyayi_ozetle(aday)?);
        }
        tanim_ozeti(&tanim, &cozulmus, &dosyalar, &alan_kimligi)
    })
    .await
    .map_err(|_| TanimHatasi::DosyaOkunamadi)?
}

/// Yerel dosya seçiciyle seçilen yürütülebilir dosyanın arayüze dönen güvenli
/// gösterimi. Tam dosya yolu hiçbir zaman bu yapıda yer almaz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeciliKomutBilgisi {
    pub kimlik: String,
    pub gorunen_ad: String,
}

/// Arka uçta tutulan kayıt veya içe aktarma taslağının opak kimliği.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TaslakKimligi(String);

impl TaslakKimligi {
    pub fn yeni() -> Self {
        Self(format!("mcp-taslak-{}", Uuid::new_v4()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TaslakKimligi {
    fn default() -> Self {
        Self::yeni()
    }
}

/// Toplu JSON içe aktarmada `url` alanı nedeniyle atlanan sunucu satırı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AtlananUrlSunucusuDto {
    pub ad: String,
    pub neden: String,
}

/// Toplu JSON içe aktarma önizlemesindeki tek bir sunucu DTO'su.
///
/// Sırlar ve mutlak dosya yolları bu yapıda ham olarak yer almaz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IceAktarmaSunucuOnizlemeDto {
    pub ad: String,
    pub etkin: bool,
    pub kip: McpKipi,
    pub komut_gorunumu: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secili_komut_kimligi: Option<String>,
    pub argumanlar: Vec<String>,
    pub ortam: Vec<McpOrtamDegiskeniDto>,
}

/// Toplu JSON içe aktarma önizleme sonucu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IceAktarmaOnizlemeDto {
    pub taslak_kimligi: TaslakKimligi,
    pub kusak: u64,
    pub sunucular: Vec<IceAktarmaSunucuOnizlemeDto>,
    pub atlanan_url_sunuculari: Vec<AtlananUrlSunucusuDto>,
}

/// Onaylı kayıt veya içe aktarma işleminin sonucu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KayitSonucu {
    pub ayarlar: McpAyarlar,
    pub tanim_ozetleri: BTreeMap<String, McpTanimOzeti>,
    pub temizlik_bekleyenler: Vec<GizliKimlik>,
    /// Yeniden adlandırmada artık bulunmayan eski ad.
    pub eski_adlar: Vec<String>,
}

/// Yerel kayıt, dosya seçimi ve JSON içe aktarma hataları.
///
/// Hiçbir varyant ham JSON metnini, aday sır değerini veya sistem anahtarlığı
/// iç hata dizgesini taşımaz.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KayitHatasi {
    #[error("kullanıcı yerel onay penceresinde işlemi reddetti")]
    KullaniciReddetti,
    #[error("onay beklenirken taslak veya ayarlar değişti; işlem iptal edildi")]
    TaslakDegisti,
    #[error("geçersiz veya süresi dolmuş taslak kimliği")]
    GecersizTaslak,
    #[error("seçilen yürütülebilir dosya geçersiz veya bulunamadı")]
    GecersizSeciliDosya,
    #[error("geçersiz MCP JSON yapılandırması")]
    GecersizJson,
    #[error("MCP tanımı doğrulanamadı")]
    Tanim(TanimHatasi),
    #[error(transparent)]
    Ayarlar(#[from] McpAyarlarHatasi),
    #[error(transparent)]
    Kasa(#[from] KasaHatasi),
}

impl From<TanimHatasi> for KayitHatasi {
    fn from(hata: TanimHatasi) -> Self {
        Self::Tanim(hata)
    }
}

impl From<GizliDegerHatasi> for KayitHatasi {
    fn from(hata: GizliDegerHatasi) -> Self {
        Self::Ayarlar(McpAyarlarHatasi::GizliDegerHatasi(hata))
    }
}

#[derive(Clone)]
struct IceAktarmaOrtamGirdisi {
    ad: String,
    ham_deger: String,
    gizli: bool,
}

#[derive(Clone)]
struct IceAktarmaSunucuTaslagi {
    ad: String,
    komut: McpKomut,
    komut_gorunumu: String,
    secili_komut_kimligi: Option<String>,
    ham_argumanlar: Vec<String>,
    ortam: Vec<IceAktarmaOrtamGirdisi>,
}

#[derive(Clone)]
enum TaslakTuru {
    TekSunucu {
        eski_ad: Option<String>,
        taslak: McpSunucuTaslagi,
    },
    IceAktarma {
        sunucular: Vec<IceAktarmaSunucuTaslagi>,
    },
}

#[derive(Clone)]
struct TaslakKaydi {
    kusak: u64,
    form_kusagi: u64,
    tur: TaslakTuru,
}

#[derive(Default)]
struct KayitDurumu {
    son_komut_no: u128,
    yurutulebilirler: BTreeMap<String, (SeciliYurutulebilirKimligi, PathBuf)>,
    aktif_form_kusagi: u64,
    taslaklar: BTreeMap<TaslakKimligi, TaslakKaydi>,
}

/// MCP sunucu kayıtlarını, seçili yürütülebilir kimliklerini, taslak kuşaklarını
/// ve yerel onay kapısını yöneten arka uç servisi.
pub struct McpKayitDefteri {
    depo: Arc<dyn McpAyarlarDeposu>,
    kasa: Arc<dyn GizliKasa>,
    iletisim: Arc<dyn Iletisim>,
    durum: Mutex<KayitDurumu>,
    ozetleme_sayaci: AtomicUsize,
}

fn yeni_komut_kimligi_uret(durum: &mut KayitDurumu, yol: PathBuf) -> String {
    for (mevcut_kimlik, (_, mevcut_yol)) in &durum.yurutulebilirler {
        if *mevcut_yol == yol {
            return mevcut_kimlik.clone();
        }
    }
    loop {
        durum.son_komut_no = durum.son_komut_no.saturating_add(1);
        let no = durum.son_komut_no;
        let kimlik = format!("mcp-komut-{no}");
        if !durum.yurutulebilirler.contains_key(&kimlik) {
            durum
                .yurutulebilirler
                .insert(kimlik.clone(), (SeciliYurutulebilirKimligi::yeni(no), yol));
            return kimlik;
        }
    }
}

impl McpKayitDefteri {
    pub fn yeni(
        depo: Arc<dyn McpAyarlarDeposu>,
        kasa: Arc<dyn GizliKasa>,
        iletisim: Arc<dyn Iletisim>,
    ) -> Self {
        let (mevcut_ayarlar, _) = depo.mcp_ayarlarini_oku();
        let mut durum = KayitDurumu::default();
        for kimlik in mevcut_ayarlar.secili_yurutulebilirler.keys() {
            if let Some(sonek) = kimlik.strip_prefix("mcp-komut-")
                && let Ok(no) = sonek.parse::<u128>()
            {
                durum.son_komut_no = durum.son_komut_no.max(no);
            }
        }
        for (kimlik, yol) in &mevcut_ayarlar.secili_yurutulebilirler {
            let no = kimlik
                .strip_prefix("mcp-komut-")
                .and_then(|s| s.parse::<u128>().ok())
                .filter(|&n| n > 0)
                .unwrap_or_else(|| {
                    durum.son_komut_no = durum.son_komut_no.saturating_add(1);
                    durum.son_komut_no
                });
            let ic_kimlik = SeciliYurutulebilirKimligi::yeni(no);
            durum
                .yurutulebilirler
                .insert(kimlik.clone(), (ic_kimlik, yol.clone()));
        }
        Self {
            depo,
            kasa,
            iletisim,
            durum: Mutex::new(durum),
            ozetleme_sayaci: AtomicUsize::new(0),
        }
    }

    /// Testlerde onay sonrası dosya özetleme adımının kaç kez çalıştığını döner.
    pub fn ozetleme_sayisi(&self) -> usize {
        self.ozetleme_sayaci.load(Ordering::SeqCst)
    }

    /// Yerel dosya seçiciyi açar, seçilen mutlak yürütülebilir dosyayı arka uç
    /// kimliğine bağlar ve arayüze yalnız dosya adını + opak kimliği döner.
    pub async fn komut_sec(
        &self,
        baslangic: Option<&Path>,
    ) -> Result<Option<SeciliKomutBilgisi>, KayitHatasi> {
        let Some(secilen) = self.iletisim.dosya_sec(DOSYA_SEC_BASLIGI, baslangic).await else {
            return Ok(None);
        };
        let bilgi = self.mutlak_yurutulebilir_kaydet(&secilen)?;
        Ok(Some(bilgi))
    }

    /// Verilen mutlak yürütülebilir yolunu doğrulayıp arka uç tablosuna kaydeder.
    pub fn mutlak_yurutulebilir_kaydet(
        &self,
        yol: &Path,
    ) -> Result<SeciliKomutBilgisi, KayitHatasi> {
        if !yol.is_absolute() || yol.to_str().is_none() {
            return Err(KayitHatasi::GecersizSeciliDosya);
        }
        let meta = std::fs::metadata(yol).map_err(|_| KayitHatasi::GecersizSeciliDosya)?;
        if !meta.is_file() {
            return Err(KayitHatasi::GecersizSeciliDosya);
        }
        let Some(dosya_adi) = yol.file_name().and_then(|ad| ad.to_str()) else {
            return Err(KayitHatasi::GecersizSeciliDosya);
        };

        let mut durum = self.durum.lock().unwrap_or_else(|z| z.into_inner());
        let kimlik = yeni_komut_kimligi_uret(&mut durum, yol.to_path_buf());
        Ok(SeciliKomutBilgisi {
            kimlik,
            gorunen_ad: dosya_adi.to_owned(),
        })
    }

    /// Arka uç kimliğine karşılık gelen mutlak yürütülebilir yolunu döner.
    pub fn yurutulebilir_yolu(&self, kimlik: &str) -> Option<PathBuf> {
        if let Some((_, yol)) = self
            .durum
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .yurutulebilirler
            .get(kimlik)
            .cloned()
        {
            return Some(yol);
        }
        let (ayarlar, _) = self.depo.mcp_ayarlarini_oku();
        ayarlar.secili_yurutulebilirler.get(kimlik).cloned()
    }

    /// Yeni bir sunucu ekleme/düzenleme taslağı oluşturur ve `(TaslakKimligi, kusak)` döner.
    pub fn taslak_olustur(
        &self,
        eski_ad: Option<String>,
        taslak: McpSunucuTaslagi,
    ) -> (TaslakKimligi, u64) {
        let mut durum = self.durum.lock().unwrap_or_else(|z| z.into_inner());
        durum.aktif_form_kusagi = durum.aktif_form_kusagi.saturating_add(1);
        let form_kusagi = durum.aktif_form_kusagi;
        let kimlik = TaslakKimligi::yeni();
        let kusak = 1;
        durum.taslaklar.insert(
            kimlik.clone(),
            TaslakKaydi {
                kusak,
                form_kusagi,
                tur: TaslakTuru::TekSunucu { eski_ad, taslak },
            },
        );
        (kimlik, kusak)
    }

    /// Mevcut bir taslağı günceller ve yeni kuşağı (`kusak`) döner.
    ///
    /// Onay penceresi açıkken bu metot çağrılırsa eski kuşakla bekleyen onay
    /// dönüşte `KayitHatasi::TaslakDegisti` ile reddedilir.
    pub fn taslak_guncelle(
        &self,
        kimlik: &TaslakKimligi,
        eski_ad: Option<String>,
        taslak: McpSunucuTaslagi,
    ) -> Result<u64, KayitHatasi> {
        let mut durum = self.durum.lock().unwrap_or_else(|z| z.into_inner());
        durum.aktif_form_kusagi = durum.aktif_form_kusagi.saturating_add(1);
        let form_kusagi = durum.aktif_form_kusagi;
        let kayit = durum
            .taslaklar
            .get_mut(kimlik)
            .ok_or(KayitHatasi::GecersizTaslak)?;
        kayit.kusak = kayit.kusak.saturating_add(1);
        kayit.form_kusagi = form_kusagi;
        kayit.tur = TaslakTuru::TekSunucu { eski_ad, taslak };
        Ok(kayit.kusak)
    }

    /// Tek sunucu taslağını yerel onaydan geçirip doğrular, onay sonrasında
    /// aday dosyaları özetler ve kasaya/ayarlara atomik olarak kaydeder.
    pub async fn taslak_kaydet(
        &self,
        kimlik: &TaslakKimligi,
        beklenen_kusak: u64,
        sistem_ortami: &[(String, String)],
        ozel_kok_tabani: &Path,
        yazilabilir_kokler: &YazilabilirKokler,
        alan_kimligi: &str,
    ) -> Result<KayitSonucu, KayitHatasi> {
        let (eski_ad, taslak, beklenen_form_kusagi) = {
            let durum = self.durum.lock().unwrap_or_else(|z| z.into_inner());
            let kayit = durum
                .taslaklar
                .get(kimlik)
                .ok_or(KayitHatasi::GecersizTaslak)?;
            if kayit.kusak != beklenen_kusak || kayit.form_kusagi != durum.aktif_form_kusagi {
                return Err(KayitHatasi::TaslakDegisti);
            }
            let TaslakTuru::TekSunucu { eski_ad, taslak } = &kayit.tur else {
                return Err(KayitHatasi::GecersizTaslak);
            };
            (eski_ad.clone(), taslak.clone(), kayit.form_kusagi)
        };

        let (baz_ayarlar, baz_revizyon) = self.depo.mcp_ayarlarini_oku();
        let hazir = self.tek_sunucu_hazirla(
            eski_ad.as_deref(),
            &taslak,
            &baz_ayarlar,
            sistem_ortami,
            ozel_kok_tabani,
            yazilabilir_kokler,
            alan_kimligi,
        )?;

        let onay_metni = hazir.cozulmus.maskeli_yerel_onay_metni(&hazir.sirlar);
        if !self
            .iletisim
            .onayla(KAYIT_ONAY_BASLIGI, &onay_metni, "Kaydet", "Vazgeç")
            .await
        {
            return Err(KayitHatasi::KullaniciReddetti);
        }

        // KAPI 1: Onaydan dönüldüğünde, aday dosyaları açıp özetlemeden ÖNCE
        // taslak kuşağını ve ayar revizyonunu yeniden doğrula!
        if !self.taslak_hala_gecerli_mi(kimlik, beklenen_kusak, beklenen_form_kusagi, baz_revizyon)
        {
            return Err(KayitHatasi::TaslakDegisti);
        }

        self.ozetleme_sayaci.fetch_add(1, Ordering::SeqCst);
        let ozet =
            tanim_ozeti_hesapla(hazir.tanim, hazir.cozulmus, alan_kimligi.to_owned()).await?;

        // KAPI 2: Dosya özeti tamamlandıktan sonra, kasaya veya ayarlara
        // yazmadan ÖNCE taslağı atomik olarak doğrula ve tüket!
        if !self.taslagi_tuket(kimlik, beklenen_kusak, beklenen_form_kusagi, baz_revizyon) {
            return Err(KayitHatasi::TaslakDegisti);
        }

        let mut islem = McpAyarlarIslemi::baslat(&*self.depo);
        if islem.baz_revizyon() != baz_revizyon {
            return Err(KayitHatasi::TaslakDegisti);
        }
        if let Some((secili_kimlik, secili_yol)) = hazir.secili_yurutulebilir {
            islem.degisiklik_ekle(McpDegisikligi::SeciliYurutulebilirKaydet {
                kimlik: secili_kimlik,
                yol: secili_yol,
            });
        }
        let eski_adlar = eski_ad
            .iter()
            .filter(|a| **a != taslak.ad)
            .cloned()
            .collect();
        islem.degisiklik_ekle(McpDegisikligi::Kaydet {
            eski_ad,
            taslak: taslak.clone(),
        });

        let mut tanim_ozetleri = BTreeMap::new();
        tanim_ozetleri.insert(taslak.ad.clone(), ozet);
        let mut sonuc = self.islemi_tamamla(islem, tanim_ozetleri)?;
        sonuc.eski_adlar = eski_adlar;
        Ok(sonuc)
    }

    /// Ham `mcpServers` JSON metnini arka uçta ayrıştırır, doğrular ve sır/yol
    /// sızdırmayan önizleme DTO'su ile taslak kimliğini döner.
    pub fn ice_aktar_onizle(&self, json_metni: &str) -> Result<IceAktarmaOnizlemeDto, KayitHatasi> {
        let kok: Value = serde_json::from_str(json_metni).map_err(|_| KayitHatasi::GecersizJson)?;
        let Some(kok_nesne) = kok.as_object() else {
            return Err(KayitHatasi::GecersizJson);
        };
        if kok_nesne.keys().any(|k| k != "mcpServers") {
            return Err(KayitHatasi::GecersizJson);
        }
        let Some(sunucular_nesnesi) = kok_nesne.get("mcpServers").and_then(Value::as_object) else {
            return Err(KayitHatasi::GecersizJson);
        };

        let (mevcut_ayarlar, _) = self.depo.mcp_ayarlarini_oku();
        let mut gorulen_adlar = BTreeSet::new();
        let mut taslak_sunucular = Vec::new();
        let mut atlanan_url_sunuculari = Vec::new();
        let mut yeni_secili_komutlar: Vec<(String, SeciliYurutulebilirKimligi, PathBuf)> =
            Vec::new();

        for (ad, deger) in sunucular_nesnesi {
            if !sunucu_adi_gecerli(ad) {
                return Err(KayitHatasi::Ayarlar(McpAyarlarHatasi::GecersizSunucuAdi));
            }
            if !gorulen_adlar.insert(ad.clone()) {
                return Err(KayitHatasi::Ayarlar(McpAyarlarHatasi::YinelenenSunucuAdi));
            }
            let Some(alanlar) = deger.as_object() else {
                return Err(KayitHatasi::GecersizJson);
            };

            if let Some(url_degeri) = alanlar.get("url") {
                if !url_degeri.is_string() {
                    return Err(KayitHatasi::GecersizJson);
                }
                atlanan_url_sunuculari.push(AtlananUrlSunucusuDto {
                    ad: ad.clone(),
                    neden: URL_ATLAMA_NEDENI.to_owned(),
                });
                continue;
            }

            if mevcut_ayarlar.sunucu(ad).is_some() {
                return Err(KayitHatasi::Ayarlar(McpAyarlarHatasi::YinelenenSunucuAdi));
            }

            for anahtar in alanlar.keys() {
                if !matches!(anahtar.as_str(), "command" | "args" | "env") {
                    return Err(KayitHatasi::GecersizJson);
                }
            }

            let Some(komut_dizgesi) = alanlar.get("command").and_then(Value::as_str) else {
                return Err(KayitHatasi::GecersizJson);
            };
            if komut_dizgesi.trim().is_empty() || komut_dizgesi.contains('\0') {
                return Err(KayitHatasi::Ayarlar(McpAyarlarHatasi::GecersizKomut));
            }

            let ham_argumanlar = match alanlar.get("args") {
                None => Vec::new(),
                Some(Value::Array(dizi)) => {
                    let mut cikti = Vec::with_capacity(dizi.len());
                    for oge in dizi {
                        let Some(metin) = oge.as_str() else {
                            return Err(KayitHatasi::GecersizJson);
                        };
                        if metin.contains('\0') {
                            return Err(KayitHatasi::Ayarlar(McpAyarlarHatasi::GecersizKomut));
                        }
                        cikti.push(metin.to_owned());
                    }
                    cikti
                }
                Some(_) => return Err(KayitHatasi::GecersizJson),
            };

            let mut ortam = Vec::new();
            if let Some(env_degeri) = alanlar.get("env") {
                let Some(env_nesnesi) = env_degeri.as_object() else {
                    return Err(KayitHatasi::GecersizJson);
                };
                for (env_ad, env_val) in env_nesnesi {
                    let Some(env_metin) = env_val.as_str() else {
                        return Err(KayitHatasi::GecersizJson);
                    };
                    if env_metin.contains('\0') {
                        return Err(KayitHatasi::Ayarlar(McpAyarlarHatasi::GizliDegerHatasi(
                            GizliDegerHatasi::NulKarakteri,
                        )));
                    }
                    // İçe aktarılan ortam değerleri önizlemede varsayılan olarak gizlidir;
                    // kullanıcı kaydetmeden önce kısa olanları düz metne çevirebilir.
                    ortam.push(IceAktarmaOrtamGirdisi {
                        ad: env_ad.clone(),
                        ham_deger: env_metin.to_owned(),
                        gizli: true,
                    });
                }
            }

            let komut_yolu = Path::new(komut_dizgesi);
            let (komut, komut_gorunumu, secili_komut_kimligi) = if komut_yolu.is_absolute() {
                let meta = std::fs::metadata(komut_yolu)
                    .map_err(|_| KayitHatasi::Ayarlar(McpAyarlarHatasi::GecersizKomut))?;
                if !meta.is_file() {
                    return Err(KayitHatasi::Ayarlar(McpAyarlarHatasi::GecersizKomut));
                }
                let Some(dosya_adi) = komut_yolu.file_name().and_then(|n| n.to_str()) else {
                    return Err(KayitHatasi::Ayarlar(McpAyarlarHatasi::GecersizKomut));
                };
                let gecici_no = (yeni_secili_komutlar.len() as u128) + 1;
                let gecici_kimlik = format!("ice-aktar-gecici-{gecici_no}");
                yeni_secili_komutlar.push((
                    gecici_kimlik.clone(),
                    SeciliYurutulebilirKimligi::yeni(gecici_no),
                    komut_yolu.to_path_buf(),
                ));
                (
                    McpKomut::SeciliYurutulebilir {
                        kimlik: gecici_kimlik.clone(),
                        argumanlar: ham_argumanlar.clone(),
                    },
                    dosya_adi.to_owned(),
                    Some(gecici_kimlik),
                )
            } else {
                (
                    McpKomut::ProgramAdi {
                        ad: komut_dizgesi.to_owned(),
                        argumanlar: ham_argumanlar.clone(),
                    },
                    komut_dizgesi.to_owned(),
                    None,
                )
            };

            let aday_taslak = McpSunucuTaslagi {
                ad: ad.clone(),
                etkin: false,
                komut: komut.clone(),
                ortam: ortam
                    .iter()
                    .map(|g| McpOrtamGirdisi::Duz {
                        ad: g.ad.clone(),
                        deger: g.ham_deger.clone(),
                    })
                    .collect(),
                kip: McpKipi::Yalitimli,
                ag_izinleri: Vec::new(),
                baslatma_zamani_asimi_sn: VARSAYILAN_BASLATMA_ZAMAN_ASIMI_SN,
                cagri_zamani_asimi_sn: VARSAYILAN_CAGRI_ZAMAN_ASIMI_SN,
            };
            // Ortam ve komut kurallarını kasa/ayarlar işlemine girmeden doğrula:
            sunucu_taslagini_tanim_icin_kontrol_et(&aday_taslak)?;

            taslak_sunucular.push(IceAktarmaSunucuTaslagi {
                ad: ad.clone(),
                komut,
                komut_gorunumu,
                secili_komut_kimligi,
                ham_argumanlar,
                ortam,
            });
        }

        // Tüm girdi geçerliyse seçili yürütülebilir kimliklerini ve taslağı kaydet.
        let mut durum = self.durum.lock().unwrap_or_else(|z| z.into_inner());
        let mut kimlik_eslemesi = BTreeMap::new();
        for (gecici_kimlik, _, yol) in yeni_secili_komutlar {
            let gercek_kimlik = yeni_komut_kimligi_uret(&mut durum, yol);
            kimlik_eslemesi.insert(gecici_kimlik, gercek_kimlik);
        }

        for sunucu in &mut taslak_sunucular {
            if let McpKomut::SeciliYurutulebilir { kimlik, .. } = &mut sunucu.komut
                && let Some(gercek) = kimlik_eslemesi.get(kimlik)
            {
                *kimlik = gercek.clone();
                sunucu.secili_komut_kimligi = Some(gercek.clone());
            }
        }

        durum.aktif_form_kusagi = durum.aktif_form_kusagi.saturating_add(1);
        let form_kusagi = durum.aktif_form_kusagi;
        let taslak_kimligi = TaslakKimligi::yeni();
        let kusak = 1;
        let onizleme_sunucular = taslak_sunucular
            .iter()
            .map(ice_aktarma_onizleme_satiri)
            .collect();

        durum.taslaklar.insert(
            taslak_kimligi.clone(),
            TaslakKaydi {
                kusak,
                form_kusagi,
                tur: TaslakTuru::IceAktarma {
                    sunucular: taslak_sunucular,
                },
            },
        );

        Ok(IceAktarmaOnizlemeDto {
            taslak_kimligi,
            kusak,
            sunucular: onizleme_sunucular,
            atlanan_url_sunuculari,
        })
    }

    /// İçe aktarma taslağındaki bir ortam değişkeninin gizli/düz durumunu
    /// günceller ve yeni taslak kuşağını (`kusak`) döner.
    pub fn ice_aktar_ortam_gizliligi_degistir(
        &self,
        kimlik: &TaslakKimligi,
        sunucu_adi: &str,
        ortam_adi: &str,
        gizli: bool,
    ) -> Result<u64, KayitHatasi> {
        let mut durum = self.durum.lock().unwrap_or_else(|z| z.into_inner());
        durum.aktif_form_kusagi = durum.aktif_form_kusagi.saturating_add(1);
        let form_kusagi = durum.aktif_form_kusagi;
        let kayit = durum
            .taslaklar
            .get_mut(kimlik)
            .ok_or(KayitHatasi::GecersizTaslak)?;
        let TaslakTuru::IceAktarma { sunucular, .. } = &mut kayit.tur else {
            return Err(KayitHatasi::GecersizTaslak);
        };
        let sunucu = sunucular
            .iter_mut()
            .find(|s| s.ad == sunucu_adi)
            .ok_or(KayitHatasi::Ayarlar(McpAyarlarHatasi::SunucuBulunamadi))?;
        let girdi = sunucu
            .ortam
            .iter_mut()
            .find(|o| o.ad == ortam_adi)
            .ok_or(KayitHatasi::Ayarlar(McpAyarlarHatasi::GecersizOrtamAdi))?;
        if gizli {
            gizli_deger_dogrula(&girdi.ham_deger)?;
        }
        girdi.gizli = gizli;
        kayit.kusak = kayit.kusak.saturating_add(1);
        kayit.form_kusagi = form_kusagi;
        Ok(kayit.kusak)
    }

    /// Toplu JSON içe aktarma taslağını yerel onaydan geçirip doğrular, onay
    /// sonrasında aday dosyaları özetler ve tüm sunucuları tek atomik işlemde
    /// kaydeder.
    pub async fn ice_aktar_kaydet(
        &self,
        kimlik: &TaslakKimligi,
        beklenen_kusak: u64,
        sistem_ortami: &[(String, String)],
        ozel_kok_tabani: &Path,
        yazilabilir_kokler: &YazilabilirKokler,
        alan_kimligi: &str,
    ) -> Result<KayitSonucu, KayitHatasi> {
        let (sunucular, beklenen_form_kusagi) = {
            let durum = self.durum.lock().unwrap_or_else(|z| z.into_inner());
            let kayit = durum
                .taslaklar
                .get(kimlik)
                .ok_or(KayitHatasi::GecersizTaslak)?;
            if kayit.kusak != beklenen_kusak || kayit.form_kusagi != durum.aktif_form_kusagi {
                return Err(KayitHatasi::TaslakDegisti);
            }
            let TaslakTuru::IceAktarma { sunucular, .. } = &kayit.tur else {
                return Err(KayitHatasi::GecersizTaslak);
            };
            (sunucular.clone(), kayit.form_kusagi)
        };

        let (baz_ayarlar, baz_revizyon) = self.depo.mcp_ayarlarini_oku();
        if sunucular.is_empty() {
            if !self.taslagi_tuket(kimlik, beklenen_kusak, beklenen_form_kusagi, baz_revizyon) {
                return Err(KayitHatasi::TaslakDegisti);
            }
            return Ok(KayitSonucu {
                ayarlar: baz_ayarlar,
                tanim_ozetleri: BTreeMap::new(),
                temizlik_bekleyenler: Vec::new(),
                eski_adlar: Vec::new(),
            });
        }

        let mut hazirlar = Vec::with_capacity(sunucular.len());
        let mut tum_sirlar = Vec::new();
        for sunucu in &sunucular {
            let taslak = ice_aktarma_sunucusundan_taslak(sunucu)?;
            let hazir = self.tek_sunucu_hazirla(
                None,
                &taslak,
                &baz_ayarlar,
                sistem_ortami,
                ozel_kok_tabani,
                yazilabilir_kokler,
                alan_kimligi,
            )?;
            tum_sirlar.extend(hazir.sirlar.iter().cloned());
            hazirlar.push((taslak, hazir));
        }

        let birlesik_onay = hazirlar
            .iter()
            .map(|(_, h)| h.cozulmus.maskeli_yerel_onay_metni(&tum_sirlar))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");
        let onay_metni = maskele_metin(&birlesik_onay, &tum_sirlar);

        if !self
            .iletisim
            .onayla(ICE_AKTAR_ONAY_BASLIGI, &onay_metni, "İçe aktar", "Vazgeç")
            .await
        {
            return Err(KayitHatasi::KullaniciReddetti);
        }

        // KAPI 1: Onaydan dönüldüğünde dosya özetlemeden ÖNCE kuşak kontrolü!
        if !self.taslak_hala_gecerli_mi(kimlik, beklenen_kusak, beklenen_form_kusagi, baz_revizyon)
        {
            return Err(KayitHatasi::TaslakDegisti);
        }

        self.ozetleme_sayaci.fetch_add(1, Ordering::SeqCst);
        let mut tanim_ozetleri = BTreeMap::new();
        let mut secili_kayitlar = Vec::new();
        for (taslak, hazir) in hazirlar {
            let ozet =
                tanim_ozeti_hesapla(hazir.tanim, hazir.cozulmus, alan_kimligi.to_owned()).await?;
            tanim_ozetleri.insert(taslak.ad.clone(), ozet);
            secili_kayitlar.push((taslak, hazir.secili_yurutulebilir));
        }

        // KAPI 2: Dosya özetleri bittikten sonra kasa/ayar kaydından ÖNCE kuşak kontrolü!
        if !self.taslagi_tuket(kimlik, beklenen_kusak, beklenen_form_kusagi, baz_revizyon) {
            return Err(KayitHatasi::TaslakDegisti);
        }

        let mut islem = McpAyarlarIslemi::baslat(&*self.depo);
        if islem.baz_revizyon() != baz_revizyon {
            return Err(KayitHatasi::TaslakDegisti);
        }
        for (taslak, secili_yurutulebilir) in secili_kayitlar {
            if let Some((secili_kimlik, secili_yol)) = secili_yurutulebilir {
                islem.degisiklik_ekle(McpDegisikligi::SeciliYurutulebilirKaydet {
                    kimlik: secili_kimlik,
                    yol: secili_yol,
                });
            }
            islem.degisiklik_ekle(McpDegisikligi::Kaydet {
                eski_ad: None,
                taslak,
            });
        }

        self.islemi_tamamla(islem, tanim_ozetleri)
    }

    fn taslak_hala_gecerli_mi(
        &self,
        kimlik: &TaslakKimligi,
        beklenen_kusak: u64,
        beklenen_form_kusagi: u64,
        beklenen_revizyon: u64,
    ) -> bool {
        let (_, guncel_revizyon) = self.depo.mcp_ayarlarini_oku();
        if guncel_revizyon != beklenen_revizyon {
            return false;
        }
        let durum = self.durum.lock().unwrap_or_else(|z| z.into_inner());
        let Some(kayit) = durum.taslaklar.get(kimlik) else {
            return false;
        };
        kayit.kusak == beklenen_kusak
            && kayit.form_kusagi == beklenen_form_kusagi
            && durum.aktif_form_kusagi == beklenen_form_kusagi
    }

    fn taslagi_tuket(
        &self,
        kimlik: &TaslakKimligi,
        beklenen_kusak: u64,
        beklenen_form_kusagi: u64,
        beklenen_revizyon: u64,
    ) -> bool {
        let (_, guncel_revizyon) = self.depo.mcp_ayarlarini_oku();
        if guncel_revizyon != beklenen_revizyon {
            return false;
        }
        let mut durum = self.durum.lock().unwrap_or_else(|z| z.into_inner());
        let Some(kayit) = durum.taslaklar.get(kimlik) else {
            return false;
        };
        if kayit.kusak != beklenen_kusak
            || kayit.form_kusagi != beklenen_form_kusagi
            || durum.aktif_form_kusagi != beklenen_form_kusagi
        {
            return false;
        }
        durum.taslaklar.remove(kimlik);
        true
    }

    fn islemi_tamamla(
        &self,
        islem: McpAyarlarIslemi,
        tanim_ozetleri: BTreeMap<String, McpTanimOzeti>,
    ) -> Result<KayitSonucu, KayitHatasi> {
        // Ayarlar yazıldıysa kasa temizliği sonraki kayda kalır; kayıt
        // tamamlanmış sayılır ki süreçler yeni tanıma göre dursun.
        let (ayarlar, temizlik_bekleyenler) = match islem.kaydet(&*self.depo, &*self.kasa) {
            Ok(ayarlar) => (ayarlar, Vec::new()),
            Err(McpAyarlarHatasi::TemizlikGerekiyor { kalanlar }) => {
                (self.depo.mcp_ayarlarini_oku().0, kalanlar)
            }
            Err(hata) => return Err(hata.into()),
        };
        Ok(KayitSonucu {
            ayarlar,
            tanim_ozetleri,
            temizlik_bekleyenler,
            eski_adlar: Vec::new(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn tek_sunucu_hazirla(
        &self,
        eski_ad: Option<&str>,
        taslak: &McpSunucuTaslagi,
        baz_ayarlar: &McpAyarlar,
        sistem_ortami: &[(String, String)],
        ozel_kok_tabani: &Path,
        yazilabilir_kokler: &YazilabilirKokler,
        alan_kimligi: &str,
    ) -> Result<HazirSunucu, KayitHatasi> {
        sunucu_taslagini_tanim_icin_kontrol_et(taslak)?;

        let mevcut_indis = match eski_ad {
            Some(ad) => Some(
                baz_ayarlar
                    .sunucular
                    .iter()
                    .position(|s| s.ad == ad)
                    .ok_or(KayitHatasi::Ayarlar(McpAyarlarHatasi::SunucuBulunamadi))?,
            ),
            None => None,
        };
        if baz_ayarlar
            .sunucular
            .iter()
            .enumerate()
            .any(|(i, s)| Some(i) != mevcut_indis && s.ad == taslak.ad)
        {
            return Err(KayitHatasi::Ayarlar(McpAyarlarHatasi::YinelenenSunucuAdi));
        }

        let mevcut_kimlikler: BTreeSet<GizliKimlik> = mevcut_indis
            .map(|i| baz_ayarlar.sunucular[i].gizli_kimlikler())
            .unwrap_or_default();

        let mut korunan_gorulen = BTreeSet::new();
        let mut ortam_degerleri = Vec::with_capacity(taslak.ortam.len());
        let mut sirlar = Vec::new();

        for girdi in &taslak.ortam {
            match girdi {
                McpOrtamGirdisi::Duz { ad, deger } => {
                    ortam_degerleri.push(OrtamDegeri::duz(ad.clone(), deger.clone()));
                }
                McpOrtamGirdisi::YeniGizli { ad, deger } => {
                    let sir = GizliDeger::yeni(deger.as_str()).map_err(maske_hatasini_cevir)?;
                    sirlar.push(sir);
                    ortam_degerleri.push(OrtamDegeri::gizli(ad.clone(), deger.as_str()));
                }
                McpOrtamGirdisi::KorunanGizli { ad, kimlik } => {
                    let _ = GizliKimlik::yeni_dogrulanmis(kimlik.as_str())?;
                    if !mevcut_kimlikler.contains(kimlik) {
                        return Err(KayitHatasi::Ayarlar(
                            McpAyarlarHatasi::BilinmeyenKorunanGizliKimlik,
                        ));
                    }
                    if !korunan_gorulen.insert(kimlik.clone()) {
                        return Err(KayitHatasi::Ayarlar(
                            McpAyarlarHatasi::PaylasilanGizliKimlik,
                        ));
                    }
                    let Some(mevcut_deger) = self.kasa.oku(kimlik)? else {
                        return Err(KayitHatasi::Kasa(KasaHatasi::ErisimHatasi));
                    };
                    let sir = GizliDeger::yeni(&mevcut_deger).map_err(maske_hatasini_cevir)?;
                    sirlar.push(sir);
                    ortam_degerleri.push(OrtamDegeri::gizli(ad.clone(), mevcut_deger));
                }
            }
        }

        let (komut, secili_yurutulebilir, ic_esleme) = match &taslak.komut {
            McpKomut::ProgramAdi { ad, argumanlar } => (
                Komut::ProgramAdi {
                    ad: ad.clone(),
                    argumanlar: argumanlar.clone(),
                },
                None,
                None,
            ),
            McpKomut::SeciliYurutulebilir { kimlik, argumanlar } => {
                let Some(yol) = self.yurutulebilir_yolu(kimlik) else {
                    return Err(KayitHatasi::GecersizSeciliDosya);
                };
                let ic_kimlik = SeciliYurutulebilirKimligi::yeni(1);
                (
                    Komut::SeciliYurutulebilir {
                        kimlik: ic_kimlik,
                        argumanlar: argumanlar.clone(),
                    },
                    Some((kimlik.clone(), yol.clone())),
                    Some((ic_kimlik, yol)),
                )
            }
        };

        let kip = match taslak.kip {
            McpKipi::Yalitimli => CalistirmaKipi::Yalitimli,
            McpKipi::TamErisim => CalistirmaKipi::TamErisim,
        };
        let ag = if taslak.ag_izinleri.is_empty() {
            None
        } else {
            let mut kurallar = Vec::with_capacity(taslak.ag_izinleri.len());
            for kural in &taslak.ag_izinleri {
                let giris = format!("{}:{}", kural.host, kural.port);
                kurallar.push(AgIzinKurali::yeni(&giris, kural.ozel_ag)?);
            }
            Some(kurallar)
        };

        let tanim = McpTanim::yeni(
            taslak.ad.clone(),
            komut,
            ortam_degerleri,
            kip,
            ag,
            taslak.baslatma_zamani_asimi_sn,
            taslak.cagri_zamani_asimi_sn,
        )?;
        let ozel = kalfa_sandbox::sunucu_ozel(ozel_kok_tabani, alan_kimligi, &taslak.ad);

        let cozulmus =
            tanim.hazirla(sistem_ortami, &ozel, yazilabilir_kokler, &|aranan_kimlik| {
                match &ic_esleme {
                    Some((beklenen, yol)) if *beklenen == aranan_kimlik => Ok(yol.clone()),
                    _ => Err(TanimHatasi::KomutCozulemedi),
                }
            })?;

        Ok(HazirSunucu {
            tanim,
            cozulmus,
            sirlar,
            secili_yurutulebilir,
        })
    }
}

struct HazirSunucu {
    tanim: McpTanim,
    cozulmus: CozulmusTanim,
    sirlar: Vec<GizliDeger>,
    secili_yurutulebilir: Option<(String, PathBuf)>,
}

fn maske_hatasini_cevir(hata: kalfa_mcp::maske::MaskeHatasi) -> KayitHatasi {
    let gizli_hata = match hata {
        kalfa_mcp::maske::MaskeHatasi::CokKisa { .. } => GizliDegerHatasi::CokKisa,
        kalfa_mcp::maske::MaskeHatasi::NulBaytiVar => GizliDegerHatasi::NulKarakteri,
    };
    KayitHatasi::Ayarlar(McpAyarlarHatasi::GizliDegerHatasi(gizli_hata))
}

fn sunucu_taslagini_tanim_icin_kontrol_et(taslak: &McpSunucuTaslagi) -> Result<(), KayitHatasi> {
    crate::mcp::ayarlar::sunucu_taslagini_dogrula(taslak).map_err(KayitHatasi::Ayarlar)
}

fn ice_aktarma_onizleme_satiri(sunucu: &IceAktarmaSunucuTaslagi) -> IceAktarmaSunucuOnizlemeDto {
    let sirlar: Vec<GizliDeger> = sunucu
        .ortam
        .iter()
        .filter(|g| g.gizli)
        .filter_map(|g| GizliDeger::yeni(g.ham_deger.clone()).ok())
        .collect();
    let argumanlar = sunucu
        .ham_argumanlar
        .iter()
        .map(|a| maskele_metin(a, &sirlar))
        .collect();
    let ortam = sunucu
        .ortam
        .iter()
        .map(|g| McpOrtamDegiskeniDto {
            ad: g.ad.clone(),
            gizli: g.gizli,
            deger_acik: if g.gizli {
                None
            } else {
                Some(maskele_metin(&g.ham_deger, &sirlar))
            },
            gizli_var: g.gizli,
            gizli_kimlik: None,
        })
        .collect();
    IceAktarmaSunucuOnizlemeDto {
        ad: sunucu.ad.clone(),
        etkin: false,
        kip: McpKipi::Yalitimli,
        komut_gorunumu: maskele_metin(&sunucu.komut_gorunumu, &sirlar),
        secili_komut_kimligi: sunucu.secili_komut_kimligi.clone(),
        argumanlar,
        ortam,
    }
}

fn ice_aktarma_sunucusundan_taslak(
    sunucu: &IceAktarmaSunucuTaslagi,
) -> Result<McpSunucuTaslagi, KayitHatasi> {
    let mut ortam = Vec::with_capacity(sunucu.ortam.len());
    for girdi in &sunucu.ortam {
        if girdi.gizli {
            ortam.push(McpOrtamGirdisi::YeniGizli {
                ad: girdi.ad.clone(),
                deger: YeniGizliDeger::yeni(girdi.ham_deger.clone())?,
            });
        } else {
            ortam.push(McpOrtamGirdisi::Duz {
                ad: girdi.ad.clone(),
                deger: girdi.ham_deger.clone(),
            });
        }
    }
    Ok(McpSunucuTaslagi {
        ad: sunucu.ad.clone(),
        etkin: false,
        komut: sunucu.komut.clone(),
        ortam,
        kip: McpKipi::Yalitimli,
        ag_izinleri: Vec::new(),
        baslatma_zamani_asimi_sn: VARSAYILAN_BASLATMA_ZAMAN_ASIMI_SN,
        cagri_zamani_asimi_sn: VARSAYILAN_CAGRI_ZAMAN_ASIMI_SN,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ayarlar::Ayarlar;
    use crate::mcp::ayarlar::McpAgKurali;
    use crate::mcp::kasa::test_destek::SahteGizliKasa;
    use crate::test_destek::SahteIletisim;
    use kalfa_sandbox::OzelDizinler;

    fn test_ortami(kok: &Path) -> (PathBuf, PathBuf, YazilabilirKokler) {
        let veri = kok.join("veri");
        let proje = kok.join("proje");
        let yalitim = kok.join("yalitim");
        let kabuk = OzelDizinler::yol(&yalitim, "alan");
        for d in [&veri, &proje, &kabuk.ev, &kabuk.gecici, &kabuk.onbellek] {
            std::fs::create_dir_all(d).unwrap();
        }
        (veri, yalitim, YazilabilirKokler::yeni(proje, kabuk))
    }

    #[tokio::test]
    async fn onay_sonrasi_dosya_ozeti_degisen_kodu_yakalar() {
        let kok = tempfile::tempdir().unwrap();
        let proje = kok.path().join("proje");
        let ozel = kalfa_sandbox::sunucu_ozel(kok.path(), "alan", "ornek");
        let kabuk = OzelDizinler::yol(kok.path(), "kabuk");
        for dizin in [
            &proje,
            &ozel.ev,
            &ozel.gecici,
            &ozel.onbellek,
            &kabuk.ev,
            &kabuk.gecici,
            &kabuk.onbellek,
        ] {
            std::fs::create_dir_all(dizin).unwrap();
        }
        let ikili = std::env::current_exe().unwrap();
        let betik = proje.join("sunucu.js");
        std::fs::write(&betik, b"birinci").unwrap();

        let yeni_tanim = || {
            McpTanim::varsayilan_zaman_asimlari(
                "ornek".into(),
                Komut::SeciliYurutulebilir {
                    kimlik: SeciliYurutulebilirKimligi::yeni(1),
                    argumanlar: vec!["${CALISMA_ALANI}/sunucu.js".into()],
                },
                vec![],
                CalistirmaKipi::Yalitimli,
                None,
            )
            .unwrap()
        };
        let yazilabilir = YazilabilirKokler::yeni(proje, kabuk);
        let tanim_ilk = yeni_tanim();
        let cozulmus = tanim_ilk
            .hazirla(&[], &ozel, &yazilabilir, &|_| Ok(ikili.clone()))
            .unwrap();

        let ilk = tanim_ozeti_hesapla(tanim_ilk, cozulmus, "alan".into())
            .await
            .unwrap();
        std::fs::write(&betik, b"ikinci-").unwrap();
        let tanim_ikinci = yeni_tanim();
        let cozulmus_ikinci = tanim_ikinci
            .hazirla(&[], &ozel, &yazilabilir, &|_| Ok(ikili.clone()))
            .unwrap();
        let ikinci = tanim_ozeti_hesapla(tanim_ikinci, cozulmus_ikinci, "alan".into())
            .await
            .unwrap();
        assert_ne!(ilk, ikinci);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn onay_sonrasi_dosya_ozeti_degisen_symlink_hedefini_reddeder() {
        use std::os::unix::fs::symlink;

        let kok = tempfile::tempdir().unwrap();
        let proje = kok.path().join("proje");
        let ozel = kalfa_sandbox::sunucu_ozel(kok.path(), "alan", "ornek");
        let kabuk = OzelDizinler::yol(kok.path(), "kabuk");
        for dizin in [
            &proje,
            &ozel.ev,
            &ozel.gecici,
            &ozel.onbellek,
            &kabuk.ev,
            &kabuk.gecici,
            &kabuk.onbellek,
        ] {
            std::fs::create_dir_all(dizin).unwrap();
        }
        let ikili = std::env::current_exe().unwrap();
        let bir = ozel.ev.join("bir.js");
        let iki = ozel.ev.join("iki.js");
        let bag = ozel.ev.join("sunucu.js");
        std::fs::write(&bir, b"bir").unwrap();
        std::fs::write(&iki, b"iki").unwrap();
        symlink(&bir, &bag).unwrap();

        let tanim = McpTanim::varsayilan_zaman_asimlari(
            "ornek".into(),
            Komut::SeciliYurutulebilir {
                kimlik: SeciliYurutulebilirKimligi::yeni(1),
                argumanlar: vec!["sunucu.js".into()],
            },
            vec![],
            CalistirmaKipi::Yalitimli,
            None,
        )
        .unwrap();
        let cozulmus = tanim
            .hazirla(&[], &ozel, &YazilabilirKokler::yeni(proje, kabuk), &|_| {
                Ok(ikili.clone())
            })
            .unwrap();

        std::fs::remove_file(&bag).unwrap();
        symlink(&iki, &bag).unwrap();
        assert!(matches!(
            tanim_ozeti_hesapla(tanim, cozulmus, "alan".into()).await,
            Err(TanimHatasi::CiktiYoluDegisti)
        ));
    }

    #[test]
    fn onay_metni_gizli_ortam_degerini_gostermez() {
        let kok = tempfile::tempdir().unwrap();
        let proje = kok.path().join("proje");
        let ozel = kalfa_sandbox::sunucu_ozel(kok.path(), "alan", "ornek");
        let kabuk = OzelDizinler::yol(kok.path(), "kabuk");
        for dizin in [
            &proje,
            &ozel.ev,
            &ozel.gecici,
            &ozel.onbellek,
            &kabuk.ev,
            &kabuk.gecici,
            &kabuk.onbellek,
        ] {
            std::fs::create_dir_all(dizin).unwrap();
        }
        let ikili = std::env::current_exe().unwrap();
        let tanim = McpTanim::varsayilan_zaman_asimlari(
            "ornek".into(),
            Komut::SeciliYurutulebilir {
                kimlik: SeciliYurutulebilirKimligi::yeni(1),
                argumanlar: vec![],
            },
            vec![OrtamDegeri::gizli("GITHUB_TOKEN", "cok-gizli-deger")],
            CalistirmaKipi::Yalitimli,
            None,
        )
        .unwrap();
        let cozulmus = tanim
            .hazirla(&[], &ozel, &YazilabilirKokler::yeni(proje, kabuk), &|_| {
                Ok(ikili.clone())
            })
            .unwrap();
        assert!(!cozulmus.yerel_onay_metni().contains("cok-gizli-deger"));
        assert!(
            cozulmus
                .yerel_onay_metni()
                .contains("GITHUB_TOKEN = [gizli]")
        );
    }

    #[tokio::test]
    async fn dosya_secici_dtoya_yalniz_kimlik_ve_dosya_adi_doner_yerel_onay_tam_yolu_gosterir() {
        let kok = tempfile::tempdir().unwrap();
        let (veri, yalitim, kokler) = test_ortami(kok.path());
        let ayarlar = Arc::new(Ayarlar::oku(&veri));
        let kasa = Arc::new(SahteGizliKasa::yeni());
        let iletisim = Arc::new(SahteIletisim::default());

        let ikili = std::env::current_exe().unwrap();
        iletisim.dosya_ver(&ikili);
        iletisim.onay_ver(true);

        let kayit = McpKayitDefteri::yeni(ayarlar.clone(), kasa.clone(), iletisim.clone());
        let secili = kayit
            .komut_sec(None)
            .await
            .unwrap()
            .expect("dosya seçilmeli");

        // DTO yalnız dosya adını taşımalı; üst dizin yolu DTO'da bulunmamalı.
        let ust_dizin = ikili.parent().unwrap().to_str().unwrap();
        let dto_json = serde_json::to_string(&secili).unwrap();
        assert!(!dto_json.contains(ust_dizin));
        assert_eq!(
            secili.gorunen_ad,
            ikili.file_name().unwrap().to_str().unwrap()
        );

        let gizli_deger = "sir\"kacis\"deger999";
        let taslak = McpSunucuTaslagi {
            ad: "ornek-sunucu".into(),
            etkin: true,
            komut: McpKomut::SeciliYurutulebilir {
                kimlik: secili.kimlik.clone(),
                argumanlar: vec![format!("--auth={gizli_deger}")],
            },
            ortam: vec![McpOrtamGirdisi::YeniGizli {
                ad: "API_KEY".into(),
                deger: YeniGizliDeger::yeni(gizli_deger).unwrap(),
            }],
            kip: McpKipi::Yalitimli,
            ag_izinleri: vec![McpAgKurali {
                host: "api.example.com".into(),
                port: 443,
                ozel_ag: false,
            }],
            baslatma_zamani_asimi_sn: 30,
            cagri_zamani_asimi_sn: 120,
        };

        let (taslak_id, kusak) = kayit.taslak_olustur(None, taslak);
        let sonuc = kayit
            .taslak_kaydet(&taslak_id, kusak, &[], &yalitim, &kokler, "alan")
            .await
            .unwrap();
        assert_eq!(sonuc.ayarlar.sunucular.len(), 1);
        assert_eq!(kayit.ozetleme_sayisi(), 1);

        // Yerel onay kutusu çözülmüş tam yolu ve ağ listesini göstermeli, ama
        // sırrı (argümandaki kaçışlı hali dahil) göstermemeli.
        let sorulanlar = iletisim.sorulanlar();
        assert_eq!(sorulanlar.len(), 1);
        assert!(sorulanlar[0].contains("api.example.com:443"));
        assert!(!sorulanlar[0].contains("kacis"));
        assert!(!sorulanlar[0].contains(gizli_deger));
    }

    #[tokio::test]
    async fn yerel_onay_reddedildiginde_dosya_ozetlenmez_ve_kasa_ayarlar_degismez() {
        let kok = tempfile::tempdir().unwrap();
        let (veri, yalitim, kokler) = test_ortami(kok.path());
        let ayarlar = Arc::new(Ayarlar::oku(&veri));
        let kasa = Arc::new(SahteGizliKasa::yeni());
        let iletisim = Arc::new(SahteIletisim::default());

        let ikili = std::env::current_exe().unwrap();
        iletisim.dosya_ver(&ikili);
        iletisim.onay_ver(false);

        let kayit = McpKayitDefteri::yeni(ayarlar.clone(), kasa.clone(), iletisim.clone());
        let secili = kayit.komut_sec(None).await.unwrap().unwrap();
        let (taslak_id, kusak) = kayit.taslak_olustur(
            None,
            McpSunucuTaslagi {
                ad: "reddedilen".into(),
                etkin: true,
                komut: McpKomut::SeciliYurutulebilir {
                    kimlik: secili.kimlik,
                    argumanlar: vec![],
                },
                ortam: vec![McpOrtamGirdisi::YeniGizli {
                    ad: "GITHUB_TOKEN".into(),
                    deger: YeniGizliDeger::yeni("reddedilen-sir-12345").unwrap(),
                }],
                kip: McpKipi::Yalitimli,
                ag_izinleri: Vec::new(),
                baslatma_zamani_asimi_sn: 30,
                cagri_zamani_asimi_sn: 120,
            },
        );

        let hata = kayit
            .taslak_kaydet(&taslak_id, kusak, &[], &yalitim, &kokler, "alan")
            .await
            .unwrap_err();
        assert_eq!(hata, KayitHatasi::KullaniciReddetti);
        assert_eq!(kayit.ozetleme_sayisi(), 0);
        assert_eq!(kasa.kayit_sayisi(), 0);
        assert!(ayarlar.mcp_ayarlarini_oku().0.sunucular.is_empty());
    }

    #[tokio::test]
    async fn onay_beklerken_degistirilen_taslak_eski_onayla_ozetlenmez_ve_yazilmaz() {
        let kok = tempfile::tempdir().unwrap();
        let (veri, yalitim, kokler) = test_ortami(kok.path());
        let ayarlar = Arc::new(Ayarlar::oku(&veri));
        let kasa = Arc::new(SahteGizliKasa::yeni());
        let iletisim = Arc::new(SahteIletisim::default());

        let ikili = std::env::current_exe().unwrap();
        iletisim.dosya_ver(&ikili);
        iletisim.onay_ver(true);
        let bekleme = iletisim.onayi_beklet();

        let kayit = Arc::new(McpKayitDefteri::yeni(
            ayarlar.clone(),
            kasa.clone(),
            iletisim.clone(),
        ));
        let secili = kayit.komut_sec(None).await.unwrap().unwrap();

        let ilk_taslak = McpSunucuTaslagi {
            ad: "yaris-sunucusu".into(),
            etkin: true,
            komut: McpKomut::SeciliYurutulebilir {
                kimlik: secili.kimlik.clone(),
                argumanlar: vec!["--ilk".into()],
            },
            ortam: vec![McpOrtamGirdisi::YeniGizli {
                ad: "API_KEY".into(),
                deger: YeniGizliDeger::yeni("ilk-sir-degeri-12345").unwrap(),
            }],
            kip: McpKipi::Yalitimli,
            ag_izinleri: Vec::new(),
            baslatma_zamani_asimi_sn: 30,
            cagri_zamani_asimi_sn: 120,
        };

        let (taslak_id, ilk_kusak) = kayit.taslak_olustur(None, ilk_taslak.clone());

        let kayit_gorevi = {
            let kayit = kayit.clone();
            let taslak_id = taslak_id.clone();
            let yalitim = yalitim.clone();
            tokio::spawn(async move {
                kayit
                    .taslak_kaydet(&taslak_id, ilk_kusak, &[], &yalitim, &kokler, "alan")
                    .await
            })
        };

        // Onay penceresine girilene kadar bekle.
        while iletisim.sorulanlar().is_empty() {
            tokio::task::yield_now().await;
        }

        // Onay penceresi açıkken taslak değiştiriliyor (örneğin TamErisim'e yükseltiliyor).
        let mut degisen_taslak = ilk_taslak;
        degisen_taslak.kip = McpKipi::TamErisim;
        let yeni_kusak = kayit
            .taslak_guncelle(&taslak_id, None, degisen_taslak)
            .unwrap();
        assert_eq!(yeni_kusak, 2);

        // Şimdi bekleyen yerel onay "Evet" ile dönüyor.
        bekleme.add_permits(1);

        let sonuc = kayit_gorevi.await.unwrap();
        assert_eq!(sonuc, Err(KayitHatasi::TaslakDegisti));
        // Eski onayla dosya özetleme (`spawn_blocking`) dahi çalışmamalı!
        assert_eq!(kayit.ozetleme_sayisi(), 0);
        assert_eq!(kasa.kayit_sayisi(), 0);
        assert!(ayarlar.mcp_ayarlarini_oku().0.sunucular.is_empty());
    }

    #[tokio::test]
    async fn json_ice_aktarma_url_atlar_mutlak_komutu_kimlige_cevirir_ve_sir_sizdirmaz() {
        let kok = tempfile::tempdir().unwrap();
        let (veri, yalitim, kokler) = test_ortami(kok.path());
        let ayarlar = Arc::new(Ayarlar::oku(&veri));
        let kasa = Arc::new(SahteGizliKasa::yeni());
        let iletisim = Arc::new(SahteIletisim::default());
        iletisim.onay_ver(true);

        let kayit = McpKayitDefteri::yeni(ayarlar.clone(), kasa.clone(), iletisim.clone());
        let ikili = std::env::current_exe().unwrap();
        let ikili_metin = ikili.to_str().unwrap();
        let gizli_token = "ghp_ice_aktarilan_gizli_999";

        let json_girdi = serde_json::json!({
            "mcpServers": {
                "uzak-sunucu": {
                    "url": "https://mcp.example.com/sse"
                },
                "yerel-sunucu": {
                    "command": ikili_metin,
                    "args": [format!("--token={gizli_token}")],
                    "env": {
                        "GITHUB_TOKEN": gizli_token,
                        "PLAIN_OPT": "varsayilan-gizli-ama-duze-cevrilecek"
                    }
                }
            }
        })
        .to_string();

        let onizleme = kayit.ice_aktar_onizle(&json_girdi).unwrap();
        assert_eq!(
            onizleme.atlanan_url_sunuculari,
            vec![AtlananUrlSunucusuDto {
                ad: "uzak-sunucu".into(),
                neden: "1.1b'de".into(),
            }]
        );
        assert_eq!(onizleme.sunucular.len(), 1);
        assert!(!onizleme.sunucular[0].etkin);
        assert_eq!(onizleme.sunucular[0].kip, McpKipi::Yalitimli);
        assert!(onizleme.sunucular[0].secili_komut_kimligi.is_some());

        // Önizleme DTO'su ne sırrı ne de mutlak üst dizin yolunu içermeli!
        let onizleme_json = serde_json::to_string(&onizleme).unwrap();
        assert!(!onizleme_json.contains(gizli_token));
        assert!(!onizleme_json.contains("varsayilan-gizli-ama-duze-cevrilecek"));
        assert!(!onizleme_json.contains(ikili.parent().unwrap().to_str().unwrap()));

        // Kullanıcı taslakta PLAIN_OPT değişkenini düz metne çevirebilir (kuşak artar).
        let yeni_kusak = kayit
            .ice_aktar_ortam_gizliligi_degistir(
                &onizleme.taslak_kimligi,
                "yerel-sunucu",
                "PLAIN_OPT",
                false,
            )
            .unwrap();
        assert_eq!(yeni_kusak, 2);

        let sonuc = kayit
            .ice_aktar_kaydet(
                &onizleme.taslak_kimligi,
                yeni_kusak,
                &[],
                &yalitim,
                &kokler,
                "alan",
            )
            .await
            .unwrap();

        assert_eq!(sonuc.ayarlar.sunucular.len(), 1);
        let kaydedilen = sonuc.ayarlar.sunucu("yerel-sunucu").unwrap();
        assert!(!kaydedilen.etkin);
        assert_eq!(kaydedilen.kip, McpKipi::Yalitimli);
        assert_eq!(kasa.kayit_sayisi(), 1);

        // Yerel onay kutusunda mutlak komut yolu açıkça gösterilmeli ama gizli
        // değer maskelenmiş olmalı.
        let onay_metni = &iletisim.sorulanlar()[0];
        assert!(onay_metni.contains(ikili.file_name().unwrap().to_str().unwrap()));
        assert!(!onay_metni.contains(gizli_token));

        // Geçersiz JSON veya kısa sır hata mesajında ham girdiyi sızdırmamalı
        // ve mevcut ayarları kısmen değiştirmemeli.
        let bozuk_json = r#"{"mcpServers": {"kisa": {"command": "npx", "env": {"API_KEY": "kisa7"}}}} gizli-ham-parca"#;
        let hata = kayit.ice_aktar_onizle(bozuk_json).unwrap_err();
        let hata_metni = format!("{hata} {hata:?}");
        assert!(!hata_metni.contains("gizli-ham-parca"));
        assert!(!hata_metni.contains("kisa7"));
        assert_eq!(ayarlar.mcp_ayarlarini_oku().0.sunucular.len(), 1);

        // 8 karakterden kısa `env` değeri önizlemede kabul edilmeli; gizli
        // bırakılırsa kayıtta CokKisa dönmeli, düz metne çevrilirse kaydedilebilmeli.
        let kisa_env_json = serde_json::json!({
            "mcpServers": {
                "kisa-env-sunucu": {
                    "command": ikili_metin,
                    "env": {
                        "LOG_LEVEL": "info"
                    }
                }
            }
        })
        .to_string();
        let kisa_onizleme = kayit.ice_aktar_onizle(&kisa_env_json).unwrap();
        let kisa_hata = kayit
            .ice_aktar_kaydet(
                &kisa_onizleme.taslak_kimligi,
                kisa_onizleme.kusak,
                &[],
                &yalitim,
                &kokler,
                "alan",
            )
            .await
            .unwrap_err();
        assert_eq!(
            kisa_hata,
            KayitHatasi::Ayarlar(McpAyarlarHatasi::GizliDegerHatasi(
                GizliDegerHatasi::CokKisa
            ))
        );
        assert!(!format!("{kisa_hata} {kisa_hata:?}").contains("info"));

        let kisa_kusak_2 = kayit
            .ice_aktar_ortam_gizliligi_degistir(
                &kisa_onizleme.taslak_kimligi,
                "kisa-env-sunucu",
                "LOG_LEVEL",
                false,
            )
            .unwrap();
        let kisa_sonuc = kayit
            .ice_aktar_kaydet(
                &kisa_onizleme.taslak_kimligi,
                kisa_kusak_2,
                &[],
                &yalitim,
                &kokler,
                "alan",
            )
            .await
            .unwrap();
        assert_eq!(kisa_sonuc.ayarlar.sunucular.len(), 2);
        // Kayıt ve önizleme adımları diskte `<yalitim>/alan/mcp` dizini oluşturmamalı (§5.6).
        assert!(!yalitim.join("alan").join("mcp").exists());
    }

    #[tokio::test]
    async fn secili_yurutulebilir_kimligi_silme_ve_yeniden_yukleme_sonrasi_cakismaz() {
        let kok = tempfile::tempdir().unwrap();
        let (veri, yalitim, kokler) = test_ortami(kok.path());
        let ayarlar = Arc::new(Ayarlar::oku(&veri));
        let kasa = Arc::new(SahteGizliKasa::yeni());
        let iletisim = Arc::new(SahteIletisim::default());
        iletisim.onay_ver(true);

        let bin1 = kok.path().join("bin1.exe");
        let bin2 = kok.path().join("bin2.exe");
        let bin3 = kok.path().join("bin3.exe");
        for bin in [&bin1, &bin2, &bin3] {
            std::fs::write(bin, b"#!/bin/sh\n").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(bin, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
        }

        let kayit = McpKayitDefteri::yeni(ayarlar.clone(), kasa.clone(), iletisim.clone());
        let k1 = kayit.mutlak_yurutulebilir_kaydet(&bin1).unwrap();
        let k2 = kayit.mutlak_yurutulebilir_kaydet(&bin2).unwrap();
        assert_eq!(k1.kimlik, "mcp-komut-1");
        assert_eq!(k2.kimlik, "mcp-komut-2");

        for (ad, kimlik) in [("s1", k1.kimlik.clone()), ("s2", k2.kimlik.clone())] {
            let (id, kusak) = kayit.taslak_olustur(
                None,
                McpSunucuTaslagi {
                    ad: ad.into(),
                    etkin: true,
                    komut: McpKomut::SeciliYurutulebilir {
                        kimlik,
                        argumanlar: vec![],
                    },
                    ortam: vec![],
                    kip: McpKipi::Yalitimli,
                    ag_izinleri: vec![],
                    baslatma_zamani_asimi_sn: 30,
                    cagri_zamani_asimi_sn: 120,
                },
            );
            kayit
                .taslak_kaydet(&id, kusak, &[], &yalitim, &kokler, "alan")
                .await
                .unwrap();
        }

        // s1 silinince `mcp-komut-1` referanssız kalıp temizlenir; yalnız `mcp-komut-2` kalır.
        let mut sil = McpAyarlarIslemi::baslat(&*ayarlar);
        sil.degisiklik_ekle(McpDegisikligi::Sil { ad: "s1".into() });
        sil.kaydet(&*ayarlar, &*kasa).unwrap();

        // Yeniden başlatılan kayıt defteri `mcp-komut-2`'yi ezmeyip `mcp-komut-3` üretmeli.
        let yeniden_kayit = McpKayitDefteri::yeni(ayarlar.clone(), kasa.clone(), iletisim.clone());
        let k3 = yeniden_kayit.mutlak_yurutulebilir_kaydet(&bin3).unwrap();
        assert_eq!(k3.kimlik, "mcp-komut-3");
        assert_eq!(
            yeniden_kayit.yurutulebilir_yolu("mcp-komut-2").as_deref(),
            Some(bin2.as_path())
        );
    }

    #[tokio::test]
    async fn korunan_gizli_yerel_onayda_maskelenir_ve_baska_sunucu_kimligi_onaydan_once_reddedilir()
    {
        let kok = tempfile::tempdir().unwrap();
        let (veri, yalitim, kokler) = test_ortami(kok.path());
        let ayarlar = Arc::new(Ayarlar::oku(&veri));
        let kasa = Arc::new(SahteGizliKasa::yeni());
        let iletisim = Arc::new(SahteIletisim::default());
        let ikili = std::env::current_exe().unwrap();
        iletisim.dosya_ver(&ikili);
        iletisim.onay_ver(true);

        let kayit = McpKayitDefteri::yeni(ayarlar.clone(), kasa.clone(), iletisim.clone());
        let secili = kayit.komut_sec(None).await.unwrap().unwrap();
        let korunan_sir = "korunan-ozel-sir-987654";

        let (ilk_id, ilk_kusak) = kayit.taslak_olustur(
            None,
            McpSunucuTaslagi {
                ad: "birinci".into(),
                etkin: true,
                komut: McpKomut::SeciliYurutulebilir {
                    kimlik: secili.kimlik.clone(),
                    argumanlar: vec![],
                },
                ortam: vec![McpOrtamGirdisi::YeniGizli {
                    ad: "SECRET_A".into(),
                    deger: YeniGizliDeger::yeni(korunan_sir).unwrap(),
                }],
                kip: McpKipi::Yalitimli,
                ag_izinleri: Vec::new(),
                baslatma_zamani_asimi_sn: 30,
                cagri_zamani_asimi_sn: 120,
            },
        );
        let ilk_sonuc = kayit
            .taslak_kaydet(&ilk_id, ilk_kusak, &[], &yalitim, &kokler, "alan")
            .await
            .unwrap();
        let gizli_kimlik = ilk_sonuc
            .ayarlar
            .sunucu("birinci")
            .unwrap()
            .gizli_kimlikler()
            .into_iter()
            .next()
            .unwrap();

        // Düzenleme sırasında korunan sır argümana yazılsa bile yerel onayda maskelenmeli.
        let (duzenle_id, duzenle_kusak) = kayit.taslak_olustur(
            Some("birinci".into()),
            McpSunucuTaslagi {
                ad: "birinci-yeni".into(),
                etkin: true,
                komut: McpKomut::SeciliYurutulebilir {
                    kimlik: secili.kimlik.clone(),
                    argumanlar: vec![format!("--secret={korunan_sir}")],
                },
                ortam: vec![McpOrtamGirdisi::KorunanGizli {
                    ad: "SECRET_A_YENI".into(),
                    kimlik: gizli_kimlik.clone(),
                }],
                kip: McpKipi::Yalitimli,
                ag_izinleri: Vec::new(),
                baslatma_zamani_asimi_sn: 30,
                cagri_zamani_asimi_sn: 120,
            },
        );
        let duzenle_sonuc = kayit
            .taslak_kaydet(&duzenle_id, duzenle_kusak, &[], &yalitim, &kokler, "alan")
            .await
            .unwrap();
        assert_eq!(
            duzenle_sonuc
                .ayarlar
                .sunucu("birinci-yeni")
                .unwrap()
                .gizli_kimlikler(),
            BTreeSet::from([gizli_kimlik.clone()])
        );
        let son_onay = iletisim.sorulanlar().pop().unwrap();
        assert!(!son_onay.contains(korunan_sir));

        // Başka bir sunucu bu `gizli_kimlik` değerini kullanmaya çalışırsa yerel onay
        // penceresi dahi açılmadan reddedilmeli.
        let onceki_sorulan_sayisi = iletisim.sorulanlar().len();
        let (saldiri_id, saldiri_kusak) = kayit.taslak_olustur(
            None,
            McpSunucuTaslagi {
                ad: "ikinci".into(),
                etkin: true,
                komut: McpKomut::SeciliYurutulebilir {
                    kimlik: secili.kimlik,
                    argumanlar: vec![],
                },
                ortam: vec![McpOrtamGirdisi::KorunanGizli {
                    ad: "CALINAN".into(),
                    kimlik: gizli_kimlik,
                }],
                kip: McpKipi::Yalitimli,
                ag_izinleri: Vec::new(),
                baslatma_zamani_asimi_sn: 30,
                cagri_zamani_asimi_sn: 120,
            },
        );
        let hata = kayit
            .taslak_kaydet(&saldiri_id, saldiri_kusak, &[], &yalitim, &kokler, "alan")
            .await
            .unwrap_err();
        assert_eq!(
            hata,
            KayitHatasi::Ayarlar(McpAyarlarHatasi::BilinmeyenKorunanGizliKimlik)
        );
        assert_eq!(iletisim.sorulanlar().len(), onceki_sorulan_sayisi);
    }

    /// Ayarlar yazılıp yalnız eski sırrın kasadan silinmesi düşerse kayıt
    /// tamamlanmış sayılır: çağıran süreçleri yeni tanıma göre durdurabilsin.
    /// Yeniden adlandırmada eski ad da sonuçla döner.
    #[cfg(unix)]
    #[tokio::test]
    async fn kasa_temizligi_kalan_duzenleme_tamamlanmis_sayilir_ve_eski_adi_doner() {
        let kok = tempfile::tempdir().unwrap();
        let (veri, yalitim, kokler) = test_ortami(kok.path());
        let ayarlar = Arc::new(Ayarlar::oku(&veri));
        let kasa = Arc::new(SahteGizliKasa::yeni());
        let iletisim = Arc::new(SahteIletisim::default());
        iletisim.dosya_ver(&std::env::current_exe().unwrap());
        iletisim.onay_ver(true);
        let kayit = McpKayitDefteri::yeni(ayarlar.clone(), kasa.clone(), iletisim.clone());
        let secili = kayit.komut_sec(None).await.unwrap().unwrap();
        let taslak = |ad: &str, deger: &str| McpSunucuTaslagi {
            ad: ad.into(),
            etkin: true,
            komut: McpKomut::SeciliYurutulebilir {
                kimlik: secili.kimlik.clone(),
                argumanlar: vec![],
            },
            ortam: vec![McpOrtamGirdisi::YeniGizli {
                ad: "ANAHTAR".into(),
                deger: YeniGizliDeger::yeni(deger).unwrap(),
            }],
            kip: McpKipi::Yalitimli,
            ag_izinleri: Vec::new(),
            baslatma_zamani_asimi_sn: 30,
            cagri_zamani_asimi_sn: 120,
        };

        let (id, kusak) = kayit.taslak_olustur(None, taslak("birinci", "eski-gizli-deger"));
        let ilk = kayit
            .taslak_kaydet(&id, kusak, &[], &yalitim, &kokler, "alan")
            .await
            .unwrap();
        assert!(ilk.eski_adlar.is_empty());
        let eski = ilk.ayarlar.sunucu("birinci").unwrap().gizli_kimlikler();
        for kimlik in &eski {
            kasa.sil_hatasi_ekle(kimlik.clone());
        }

        let (id, kusak) =
            kayit.taslak_olustur(Some("birinci".into()), taslak("ikinci", "yeni-gizli-deger"));
        let sonuc = kayit
            .taslak_kaydet(&id, kusak, &[], &yalitim, &kokler, "alan")
            .await
            .expect("ayarlar yazıldı; kayıt tamamlanmış sayılır");
        assert_eq!(sonuc.eski_adlar, vec!["birinci".to_owned()]);
        assert_eq!(
            sonuc
                .temizlik_bekleyenler
                .into_iter()
                .collect::<BTreeSet<_>>(),
            eski
        );
        assert!(sonuc.ayarlar.sunucu("birinci").is_none());
        assert!(sonuc.ayarlar.sunucu("ikinci").is_some());
    }
}
