//! MCP sunucu tanımı ve başlatma öncesi salt yol çözümlemesi.
//!
//! Tanımlar ham komut/ortam değerleri taşıdığı için Debug ve serde uygulamaz.
//! Bu modül dosya içeriği okumaz; içerik özeti yalnız uygulamanın yerel onay
//! kapısından sonra alınır.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io,
    path::{Component, Path, PathBuf},
    time::Duration,
};

use kalfa_sandbox::{CalistirmaKipi, KomutBicimi, OzelDizinler, program};
use same_file::Handle;

pub use crate::ortam::OrtamDegeri;

const VARSAYILAN_BASLATMA_SN: u32 = 30;
const VARSAYILAN_CAGRI_SN: u32 = 120;

/// npx/uvx tarafından çözülen paket kodu T'ye dosya içeriği olarak girmez.
/// Görev 11 bu güvenli, sabit uyarıyı kullanıcıya gösterebilir.
pub const NPX_UVX_SURUM_UYARISI: &str = "npx/uvx ile çözülen paket kodu tanım özetine içerik olarak bağlanmaz; sürümü sabitlenmeyen paketler başlatmalar arasında değişebilir.";

/// Seçilmiş yürütülebilir dosyanın yerel, opak backend kimliği.
/// Arayüz bu kimliği taşır; yolu yalnız backend çözümleyicisi döndürür.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SeciliYurutulebilirKimligi(u128);

impl SeciliYurutulebilirKimligi {
    pub const fn yeni(deger: u128) -> Self {
        Self(deger)
    }
}

/// Yerel dosya seçicisinden gelen kimliği backend'deki mutlak yola çözer.
pub trait SeciliYurutulebilirCozucu {
    fn coz(&self, kimlik: SeciliYurutulebilirKimligi) -> Result<PathBuf, TanimHatasi>;
}

impl<F> SeciliYurutulebilirCozucu for F
where
    F: Fn(SeciliYurutulebilirKimligi) -> Result<PathBuf, TanimHatasi>,
{
    fn coz(&self, kimlik: SeciliYurutulebilirKimligi) -> Result<PathBuf, TanimHatasi> {
        self(kimlik)
    }
}

/// İki tanım biçimi: PATH'te aranan çıplak ad veya yerel seçicinin opak kimliği.
/// Argümanlar her iki biçimde de aynı argv dizisidir; kabuk komutu yoktur.
pub enum Komut {
    ProgramAdi {
        ad: String,
        argumanlar: Vec<String>,
    },
    SeciliYurutulebilir {
        kimlik: SeciliYurutulebilirKimligi,
        argumanlar: Vec<String>,
    },
}

/// Doğrulanmış ağ kuralı. `TamErisim` tanımı ağ listesi taşıyamaz.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AgIzinKurali {
    host: String,
    port: u16,
    ozel_ag: bool,
}

impl AgIzinKurali {
    pub fn yeni(host_ve_port: &str, ozel_ag: bool) -> Result<Self, TanimHatasi> {
        let (host, port) = kalfa_sandbox::vekil::ag_hostu_coz(host_ve_port)
            .map_err(|_| TanimHatasi::GecersizAgKurali)?;
        Ok(Self {
            host,
            port,
            ozel_ag,
        })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub const fn port(&self) -> u16 {
        self.port
    }

    pub const fn ozel_ag(&self) -> bool {
        self.ozel_ag
    }
}

/// W'deki yazılabilir kökler: proje ve kabuğun özel ev/geçici/önbelleği.
/// Her MCP sunucusunun kendi özel kökü W'ye girmez.
pub struct YazilabilirKokler {
    proje: PathBuf,
    kabuk_ozel: OzelDizinler,
}

impl YazilabilirKokler {
    pub fn yeni(proje: PathBuf, kabuk_ozel: OzelDizinler) -> Self {
        Self { proje, kabuk_ozel }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KodGuveni {
    WdeKodlu,
    WDisindaKodlu,
}

/// Yerel onaydan sonra okunması gerekebilecek dosya. Hazırlık aşamasında
/// yalnızca dosya türü ve dosya kimliği alınır; içerik okunmaz.
pub struct DosyaAdayi {
    yol: PathBuf,
    referans_yollar: Vec<PathBuf>,
    kimlik: Handle,
}

impl DosyaAdayi {
    /// Kanonik dosya yolunun kimliğini, içerik okumadan yakalar.
    pub fn yakala(yol: PathBuf) -> Result<Self, TanimHatasi> {
        let referans = if yol.is_absolute() {
            yol
        } else {
            std::env::current_dir()
                .map_err(|_| TanimHatasi::YolCozumlenemedi)?
                .join(yol)
        };
        let kanonik = fs::canonicalize(&referans).map_err(|_| TanimHatasi::YolCozumlenemedi)?;
        Self::yakala_kanonik_ve_referanslar(kanonik, vec![referans])
    }

    fn yakala_kanonik_ve_referanslar(
        kanonik: PathBuf,
        referans_yollar: Vec<PathBuf>,
    ) -> Result<Self, TanimHatasi> {
        if !fs::metadata(&kanonik)
            .map_err(|_| TanimHatasi::YolCozumlenemedi)?
            .is_file()
        {
            return Err(TanimHatasi::YolCozumlenemedi);
        }
        let dosya = File::open(&kanonik).map_err(|_| TanimHatasi::YolCozumlenemedi)?;
        if !dosya
            .metadata()
            .map_err(|_| TanimHatasi::YolCozumlenemedi)?
            .is_file()
        {
            return Err(TanimHatasi::YolCozumlenemedi);
        }
        let kimlik = Handle::from_file(
            dosya
                .try_clone()
                .map_err(|_| TanimHatasi::YolCozumlenemedi)?,
        )
        .map_err(|_| TanimHatasi::YolCozumlenemedi)?;
        if Handle::from_path(&kanonik).map_err(|_| TanimHatasi::CiktiYoluDegisti)? != kimlik {
            return Err(TanimHatasi::CiktiYoluDegisti);
        }
        for referans in &referans_yollar {
            if fs::canonicalize(referans).map_err(|_| TanimHatasi::CiktiYoluDegisti)? != kanonik
                || Handle::from_path(referans).map_err(|_| TanimHatasi::CiktiYoluDegisti)? != kimlik
            {
                return Err(TanimHatasi::CiktiYoluDegisti);
            }
        }
        yol_metni(&kanonik)?;
        Ok(Self {
            yol: kanonik,
            referans_yollar,
            kimlik,
        })
    }

    pub fn yol(&self) -> &Path {
        &self.yol
    }

    /// Açık dosya hâlâ hazırlıkta yakalanan dosya mı ve yol aynı nesneyi mi
    /// gösteriyor? İçerik okumaz.
    pub fn kimlik_uyuyor(&self, dosya: &File) -> io::Result<bool> {
        if !dosya.metadata()?.is_file() {
            return Ok(false);
        }
        let acilan = Handle::from_file(dosya.try_clone()?)?;
        if acilan != self.kimlik {
            return Ok(false);
        }
        if Handle::from_path(&self.yol)? != acilan {
            return Ok(false);
        }
        for referans in &self.referans_yollar {
            if fs::canonicalize(referans)? != self.yol || Handle::from_path(referans)? != acilan {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TanimHatasi {
    GecersizAd,
    GecersizKomut,
    GoreliIkiliYolu,
    YinelenenOrtamAdi,
    GecersizOrtamAdi,
    GecersizOrtamDegeri,
    AyrilmisOrtamAdi,
    SabitOrtamAdi,
    OzelKokOrtamAdi,
    TamErisimAgListesi,
    GecersizAgKurali,
    SifirZamanAsimi,
    KomutCozulemedi,
    YolCozumlenemedi,
    CiktiYoluDegisti,
    DosyaOkunamadi,
    ProfilKuraliEksik,
    GecersizUzakUrl,
    GecersizUzakYetki,
    GecersizBaglamButcesi,
}

impl std::fmt::Display for TanimHatasi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::GecersizAd => "MCP sunucu adı geçersiz.",
            Self::GecersizKomut => "MCP komut biçimi geçersiz.",
            Self::GoreliIkiliYolu => "Seçili yürütülebilir dosya mutlak yol olmalı.",
            Self::YinelenenOrtamAdi => "Ortam değişkeni adı yineleniyor.",
            Self::GecersizOrtamAdi => "Ortam değişkeni adı geçersiz.",
            Self::GecersizOrtamDegeri => "Ortam değişkeni değeri geçersiz.",
            Self::AyrilmisOrtamAdi => "EVREN_ ve KALFA_ ortam adları MCP sunucusuna verilemez.",
            Self::SabitOrtamAdi => "Kalfa'nın sabit ortam değişkeni değiştirilemez.",
            Self::OzelKokOrtamAdi => "Sunucunun özel kök ortam değişkeni değiştirilemez.",
            Self::TamErisimAgListesi => "Tam erişimli sunucu ağ izin listesi taşıyamaz.",
            Self::GecersizAgKurali => "Ağ izin kuralı geçersiz.",
            Self::SifirZamanAsimi => "Zaman aşımı sıfır olamaz.",
            Self::KomutCozulemedi => "Komut sunucu ortamında çözülemedi.",
            Self::YolCozumlenemedi => "Kod güveni için bir yol doğrulanamadı.",
            Self::CiktiYoluDegisti => "Yerel onaydan sonra çözülen yol değişti.",
            Self::DosyaOkunamadi => "Yerel onaydan sonra kod özeti alınamadı.",
            Self::ProfilKuraliEksik => "Sunucunun özel çalışma kökü geçersiz.",
            Self::GecersizUzakUrl => "Uzak MCP sunucu uç URL'si geçersiz veya güvensiz.",
            Self::GecersizUzakYetki => "Uzak MCP yetkilendirme tanımı geçersiz.",
            Self::GecersizBaglamButcesi => "Uzak MCP bağlam bütçesi sınırları geçersiz.",
        })
    }
}

impl std::error::Error for TanimHatasi {}

/// Uzak MCP araç aramasında tek sorguda döndürülebilecek azami aday sayısı.
pub const AZAMI_UZAK_ADAY_ARAC_SINIRI: usize = 8;
/// Oturum başına aynı anda etkin yüklenebilecek azami uzak MCP aracı sayısı.
pub const AZAMI_UZAK_ETKIN_ARAC_SINIRI: usize = 16;
/// Yüklenen uzak MCP araç şemalarının toplam azami bayt bütçesi (64 KiB).
pub const AZAMI_UZAK_SEMA_BAYT_SINIRI: usize = 64 * 1024;

/// Uzak MCP sunucusunun yetkilendirme türü.
///
/// Ham sır veya erişim belirteci bu tipte **asla** tutulmaz; yalnızca opak
/// kasa kimlikleri (`mcp-<uuid>`) ve genel OAuth yapılandırması yer alır.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UzakYetkiTuru {
    Yetkisiz,
    SabitBaslikGizli {
        baslik_adi: String,
        gizli_kimlik: String,
    },
    OAuth {
        istemci_kimligi: Option<String>,
        kapsamlar: Vec<String>,
        kaynak_url: Option<String>,
        yetki_kimligi: Option<String>,
    },
}

/// Uzak MCP sunucusu için bağlam bütçesi sınırları.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UzakBaglamButcesi {
    aday_arac_siniri: usize,
    etkin_arac_siniri: usize,
    sema_bayt_siniri: usize,
}

impl Default for UzakBaglamButcesi {
    fn default() -> Self {
        Self {
            aday_arac_siniri: AZAMI_UZAK_ADAY_ARAC_SINIRI,
            etkin_arac_siniri: AZAMI_UZAK_ETKIN_ARAC_SINIRI,
            sema_bayt_siniri: AZAMI_UZAK_SEMA_BAYT_SINIRI,
        }
    }
}

impl UzakBaglamButcesi {
    pub fn yeni(
        aday_arac_siniri: usize,
        etkin_arac_siniri: usize,
        sema_bayt_siniri: usize,
    ) -> Result<Self, TanimHatasi> {
        if aday_arac_siniri == 0
            || aday_arac_siniri > AZAMI_UZAK_ADAY_ARAC_SINIRI
            || etkin_arac_siniri == 0
            || etkin_arac_siniri > AZAMI_UZAK_ETKIN_ARAC_SINIRI
            || sema_bayt_siniri == 0
            || sema_bayt_siniri > AZAMI_UZAK_SEMA_BAYT_SINIRI
        {
            return Err(TanimHatasi::GecersizBaglamButcesi);
        }
        Ok(Self {
            aday_arac_siniri,
            etkin_arac_siniri,
            sema_bayt_siniri,
        })
    }

    pub const fn aday_arac_siniri(&self) -> usize {
        self.aday_arac_siniri
    }

    pub const fn etkin_arac_siniri(&self) -> usize {
        self.etkin_arac_siniri
    }

    pub const fn sema_bayt_siniri(&self) -> usize {
        self.sema_bayt_siniri
    }

    pub const fn varsayilan() -> Self {
        Self {
            aday_arac_siniri: AZAMI_UZAK_ADAY_ARAC_SINIRI,
            etkin_arac_siniri: AZAMI_UZAK_ETKIN_ARAC_SINIRI,
            sema_bayt_siniri: AZAMI_UZAK_SEMA_BAYT_SINIRI,
        }
    }
}

/// Uzak Streamable HTTP MCP sunucu tanımı (`1.1b`).
///
/// Yerel süreç (`stdio`), sahte PID, kod kökü (`W`) veya yerel AppContainer/bwrap
/// yalıtımı alanı içermez.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UzakMcpTanim {
    ad: String,
    kanonik_uc_url: String,
    test_fiksturu_loopback: bool,
    yetki: UzakYetkiTuru,
    izinli_baslik_kimlikleri: Vec<String>,
    baslatma_zamani_asimi: u32,
    cagri_zamani_asimi: u32,
    baglam_butcesi: UzakBaglamButcesi,
}

impl UzakMcpTanim {
    pub fn yeni(
        ad: String,
        uc_url: String,
        yetki: UzakYetkiTuru,
        baslatma_zamani_asimi: u32,
        cagri_zamani_asimi: u32,
        test_fiksturu_loopback: bool,
        baglam_butcesi: UzakBaglamButcesi,
    ) -> Result<Self, TanimHatasi> {
        if !sunucu_adi_gecerli(&ad) {
            return Err(TanimHatasi::GecersizAd);
        }
        if baslatma_zamani_asimi == 0 || cagri_zamani_asimi == 0 {
            return Err(TanimHatasi::SifirZamanAsimi);
        }
        let _ = UzakBaglamButcesi::yeni(
            baglam_butcesi.aday_arac_siniri,
            baglam_butcesi.etkin_arac_siniri,
            baglam_butcesi.sema_bayt_siniri,
        )?;
        let kanonik_uc_url = uzak_uc_url_dogrula_ve_kanoniklestir(&uc_url, test_fiksturu_loopback)?;
        let (yetki, izinli_baslik_kimlikleri) = uzak_yetki_dogrula(yetki, test_fiksturu_loopback)?;

        Ok(Self {
            ad,
            kanonik_uc_url,
            test_fiksturu_loopback,
            yetki,
            izinli_baslik_kimlikleri,
            baslatma_zamani_asimi,
            cagri_zamani_asimi,
            baglam_butcesi,
        })
    }

    pub fn varsayilan_sinirlarla(
        ad: String,
        uc_url: &str,
        test_fiksturu_loopback: bool,
        yetki: UzakYetkiTuru,
    ) -> Result<Self, TanimHatasi> {
        Self::yeni(
            ad,
            uc_url.to_owned(),
            yetki,
            VARSAYILAN_BASLATMA_SN,
            VARSAYILAN_CAGRI_SN,
            test_fiksturu_loopback,
            UzakBaglamButcesi::default(),
        )
    }

    pub fn ad(&self) -> &str {
        &self.ad
    }

    pub fn kanonik_uc_url(&self) -> &str {
        &self.kanonik_uc_url
    }

    pub fn uc_url(&self) -> &str {
        &self.kanonik_uc_url
    }

    pub fn origin(&self) -> String {
        url::Url::parse(&self.kanonik_uc_url)
            .map(|u| u.origin().ascii_serialization())
            .unwrap_or_default()
    }

    pub fn yetki_turu(&self) -> &UzakYetkiTuru {
        &self.yetki
    }

    pub fn varsayilan_zaman_asimlari(
        ad: String,
        uc_url: String,
        yetki: UzakYetkiTuru,
    ) -> Result<Self, TanimHatasi> {
        Self::varsayilan_sinirlarla(ad, &uc_url, false, yetki)
    }

    pub fn uzak_onay_metni(&self) -> String {
        self.maskeli_uzak_onay_metni(&[])
    }

    /// Uzak MCP sunucusu için yerel onay penceresi metnini üretir; yerel yalıtım
    /// veya tam sistem erişimi rozeti kullanmaz ve tüm sırları maskeler.
    pub fn maskeli_uzak_onay_metni(&self, ek_sirlar: &[crate::maske::GizliDeger]) -> String {
        let yetki_ozeti = match &self.yetki {
            UzakYetkiTuru::Yetkisiz => "Yetkisiz".to_owned(),
            UzakYetkiTuru::SabitBaslikGizli { baslik_adi, .. } => {
                format!("Sabit başlık ({baslik_adi}: [gizli])")
            }
            UzakYetkiTuru::OAuth { kapsamlar, .. } => {
                if kapsamlar.is_empty() {
                    "OAuth 2.0 (varsayılan kapsam)".to_owned()
                } else {
                    format!("OAuth 2.0 (kapsamlar: {})", kapsamlar.join(", "))
                }
            }
        };
        let metin = format!(
            "Sunucu: {}\nTür: Uzak MCP (Streamable HTTP)\nUç: {}\nOrigin: {}\nYetkilendirme: {yetki_ozeti}\nBaşlatma zaman aşımı: {} sn\nÇağrı zaman aşımı: {} sn",
            self.ad,
            self.kanonik_uc_url,
            self.origin(),
            self.baslatma_zamani_asimi,
            self.cagri_zamani_asimi,
        );
        crate::maske::maskele_metin(&metin, ek_sirlar)
    }

    pub const fn test_fiksturu_loopback(&self) -> bool {
        self.test_fiksturu_loopback
    }

    pub fn yetki(&self) -> &UzakYetkiTuru {
        &self.yetki
    }

    pub fn izinli_baslik_kimlikleri(&self) -> &[String] {
        &self.izinli_baslik_kimlikleri
    }

    pub const fn baslatma_zamani_asimi(&self) -> Duration {
        Duration::from_secs(self.baslatma_zamani_asimi as u64)
    }

    pub const fn cagri_zamani_asimi(&self) -> Duration {
        Duration::from_secs(self.cagri_zamani_asimi as u64)
    }

    pub const fn timeoutlar(&self) -> (u32, u32) {
        (self.baslatma_zamani_asimi, self.cagri_zamani_asimi)
    }

    pub const fn baglam_butcesi(&self) -> UzakBaglamButcesi {
        self.baglam_butcesi
    }
}

/// Yerel (`1.1a`) ve uzak (`1.1b`) MCP sunucu tanımlarını ayıran üst varyant.
pub enum McpSunucuTanimi {
    Yerel(McpTanim),
    Uzak(UzakMcpTanim),
}

impl McpSunucuTanimi {
    pub fn ad(&self) -> &str {
        match self {
            Self::Yerel(y) => y.ad(),
            Self::Uzak(u) => u.ad(),
        }
    }

    pub const fn uzak_mi(&self) -> bool {
        matches!(self, Self::Uzak(_))
    }

    pub const fn yerel(&self) -> Option<&McpTanim> {
        match self {
            Self::Yerel(y) => Some(y),
            Self::Uzak(_) => None,
        }
    }

    pub const fn uzak(&self) -> Option<&UzakMcpTanim> {
        match self {
            Self::Yerel(_) => None,
            Self::Uzak(u) => Some(u),
        }
    }
}

/// Uzak uç URL'sini doğrular ve kanonik metin biçimine dönüştürür.
pub fn uzak_uc_url_dogrula_ve_kanoniklestir(
    ham_url: &str,
    test_fiksturu_loopback: bool,
) -> Result<String, TanimHatasi> {
    let temiz = ham_url.trim();
    if temiz != ham_url || temiz.is_empty() || temiz.chars().any(char::is_control) {
        return Err(TanimHatasi::GecersizUzakUrl);
    }
    let url = url::Url::parse(temiz).map_err(|_| TanimHatasi::GecersizUzakUrl)?;
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return Err(TanimHatasi::GecersizUzakUrl);
    }
    let Some(host) = url.host_str() else {
        return Err(TanimHatasi::GecersizUzakUrl);
    };
    let loopback_mi =
        matches!(host, "127.0.0.1" | "::1" | "[::1]") || host.eq_ignore_ascii_case("localhost");
    match url.scheme() {
        "https" => {}
        "http" if test_fiksturu_loopback && loopback_mi => {}
        _ => return Err(TanimHatasi::GecersizUzakUrl),
    }
    Ok(url.into())
}

fn http_baslik_adi_gecerli(ad: &str) -> bool {
    !ad.is_empty()
        && ad.len() <= 64
        && ad.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(
                    c,
                    '!' | '#'
                        | '$'
                        | '%'
                        | '&'
                        | '\''
                        | '*'
                        | '+'
                        | '-'
                        | '.'
                        | '^'
                        | '_'
                        | '`'
                        | '|'
                        | '~'
                )
        })
}

fn opak_kimlik_gecerli(kimlik: &str) -> bool {
    !kimlik.is_empty()
        && kimlik.len() <= 128
        && kimlik.trim() == kimlik
        && !kimlik.chars().any(char::is_control)
}

fn uzak_yetki_dogrula(
    yetki: UzakYetkiTuru,
    test_fiksturu_loopback: bool,
) -> Result<(UzakYetkiTuru, Vec<String>), TanimHatasi> {
    match yetki {
        UzakYetkiTuru::Yetkisiz => Ok((UzakYetkiTuru::Yetkisiz, Vec::new())),
        UzakYetkiTuru::SabitBaslikGizli {
            baslik_adi,
            gizli_kimlik,
        } => {
            if !http_baslik_adi_gecerli(&baslik_adi) || !opak_kimlik_gecerli(&gizli_kimlik) {
                return Err(TanimHatasi::GecersizUzakYetki);
            }
            let norm_baslik = baslik_adi.to_ascii_lowercase();
            if matches!(
                norm_baslik.as_str(),
                "host" | "content-length" | "transfer-encoding" | "connection" | "cookie"
            ) {
                return Err(TanimHatasi::GecersizUzakYetki);
            }
            Ok((
                UzakYetkiTuru::SabitBaslikGizli {
                    baslik_adi: norm_baslik.clone(),
                    gizli_kimlik: gizli_kimlik.clone(),
                },
                vec![format!("{norm_baslik}:{gizli_kimlik}")],
            ))
        }
        UzakYetkiTuru::OAuth {
            istemci_kimligi,
            mut kapsamlar,
            kaynak_url,
            yetki_kimligi,
        } => {
            if let Some(ref istemci) = istemci_kimligi
                && (istemci.trim().is_empty()
                    || istemci.trim() != istemci
                    || istemci.chars().any(char::is_control))
            {
                return Err(TanimHatasi::GecersizUzakYetki);
            }
            for kapsam in &kapsamlar {
                if kapsam.trim().is_empty()
                    || kapsam.trim() != kapsam
                    || kapsam.chars().any(|c| c.is_control() || c.is_whitespace())
                {
                    return Err(TanimHatasi::GecersizUzakYetki);
                }
            }
            kapsamlar.sort();
            kapsamlar.dedup();
            let norm_kaynak = match kaynak_url {
                Some(ref u) => Some(
                    uzak_uc_url_dogrula_ve_kanoniklestir(u, test_fiksturu_loopback)
                        .map_err(|_| TanimHatasi::GecersizUzakYetki)?,
                ),
                None => None,
            };
            if let Some(ref yk) = yetki_kimligi
                && !opak_kimlik_gecerli(yk)
            {
                return Err(TanimHatasi::GecersizUzakYetki);
            }
            let mut kimlikler = Vec::new();
            if let Some(ref yk) = yetki_kimligi {
                kimlikler.push(format!("oauth:{yk}"));
            }
            Ok((
                UzakYetkiTuru::OAuth {
                    istemci_kimligi,
                    kapsamlar,
                    kaynak_url: norm_kaynak,
                    yetki_kimligi,
                },
                kimlikler,
            ))
        }
    }
}

/// Ham tanım; özellikle Debug/Serialize/Deserialize uygulanmaz.
pub struct McpTanim {
    ad: String,
    komut: Komut,
    ortam: Vec<OrtamDegeri>,
    kip: CalistirmaKipi,
    ag: Option<Vec<AgIzinKurali>>,
    baslatma_zamani_asimi: u32,
    cagri_zamani_asimi: u32,
}

impl McpTanim {
    pub fn yeni(
        ad: String,
        komut: Komut,
        ortam: Vec<OrtamDegeri>,
        kip: CalistirmaKipi,
        ag: Option<Vec<AgIzinKurali>>,
        baslatma_zamani_asimi: u32,
        cagri_zamani_asimi: u32,
    ) -> Result<Self, TanimHatasi> {
        if !sunucu_adi_gecerli(&ad) {
            return Err(TanimHatasi::GecersizAd);
        }
        komut_dogrula(&komut)?;
        crate::ortam::girdileri_dogrula(&ortam)?;
        if baslatma_zamani_asimi == 0 || cagri_zamani_asimi == 0 {
            return Err(TanimHatasi::SifirZamanAsimi);
        }
        if kip == CalistirmaKipi::TamErisim && ag.is_some() {
            return Err(TanimHatasi::TamErisimAgListesi);
        }
        let ag = match ag {
            Some(mut kurallar) => {
                kurallar.sort();
                if kurallar.windows(2).any(|ikili| ikili[0] == ikili[1]) {
                    return Err(TanimHatasi::GecersizAgKurali);
                }
                (!kurallar.is_empty()).then_some(kurallar)
            }
            None => None,
        };
        Ok(Self {
            ad,
            komut,
            ortam,
            kip,
            ag,
            baslatma_zamani_asimi,
            cagri_zamani_asimi,
        })
    }

    pub fn varsayilan_zaman_asimlari(
        ad: String,
        komut: Komut,
        ortam: Vec<OrtamDegeri>,
        kip: CalistirmaKipi,
        ag: Option<Vec<AgIzinKurali>>,
    ) -> Result<Self, TanimHatasi> {
        Self::yeni(
            ad,
            komut,
            ortam,
            kip,
            ag,
            VARSAYILAN_BASLATMA_SN,
            VARSAYILAN_CAGRI_SN,
        )
    }

    pub const fn baslatma_zamani_asimi(&self) -> Duration {
        Duration::from_secs(self.baslatma_zamani_asimi as u64)
    }

    pub const fn cagri_zamani_asimi(&self) -> Duration {
        Duration::from_secs(self.cagri_zamani_asimi as u64)
    }

    pub const fn kip(&self) -> CalistirmaKipi {
        self.kip
    }

    pub fn ad(&self) -> &str {
        &self.ad
    }

    /// PATH ve yollar çözülür, fakat hiçbir dosya içeriği okunmaz.
    pub fn hazirla(
        &self,
        sistem_ortami: &[(String, String)],
        ozel: &OzelDizinler,
        yazilabilir_kokler: &YazilabilirKokler,
        cozucu: &dyn SeciliYurutulebilirCozucu,
    ) -> Result<CozulmusTanim, TanimHatasi> {
        let proje = mutlak_kok(&yazilabilir_kokler.proje, Path::new("."))?;
        let cwd = mutlak_kok(&ozel.ev, &proje)?;
        for yol in [&ozel.ev, &ozel.gecici, &ozel.onbellek] {
            yol_metni(yol)?;
        }
        let ortam = crate::ortam::uret(sistem_ortami, &self.ortam, ozel, &proje, &cwd)?;
        let (native_komut, komut_tipi) = komut_ortamini_coz(&self.komut, &proje, cozucu)?;
        let pairs = ortam.ham_ciftler();
        let (program_yolu, argumanlar) =
            program::coz(&native_komut, &pairs).map_err(|_| TanimHatasi::KomutCozulemedi)?;
        let program_referansi = if program_yolu.is_absolute() {
            program_yolu.clone()
        } else {
            cwd.join(&program_yolu)
        };
        let program_yolu = mutlak_kok(&program_yolu, &cwd)?;
        let metadata = fs::metadata(&program_yolu).map_err(|_| TanimHatasi::KomutCozulemedi)?;
        if !metadata.is_file() {
            return Err(TanimHatasi::KomutCozulemedi);
        }

        let kokler = kanonik_w_kokleri(yazilabilir_kokler, &cwd)?;
        let mut aday_yollar = BTreeMap::<PathBuf, BTreeSet<PathBuf>>::new();
        aday_yollar
            .entry(program_yolu.clone())
            .or_default()
            .insert(program_referansi);
        let mut kod_wde = yol_wde_mi(&program_yolu, &kokler)?;

        for path in std::env::split_paths(ortam.deger("PATH").unwrap_or_default()) {
            if path.as_os_str().is_empty() {
                continue;
            }
            let yol = mutlak_kok(&path, &cwd)?;
            kod_wde |= yol_wde_mi(&yol, &kokler)?;
        }

        let arguman_yollari: Vec<String> = match &self.komut {
            Komut::ProgramAdi { argumanlar, .. }
            | Komut::SeciliYurutulebilir { argumanlar, .. } => argumanlar
                .iter()
                .map(|a| calisma_alani_yerine_koy(a, &proje))
                .collect::<Result<_, _>>()?,
        };
        for deger in &arguman_yollari {
            for (referans_yolu, yol, dosya_mi) in yol_degerlerini_coz(deger, &cwd, &proje)? {
                kod_wde |= yol_wde_mi(&yol, &kokler)?;
                if dosya_mi {
                    aday_yollar.entry(yol).or_default().insert(referans_yolu);
                }
            }
        }
        let zorlanan_ortam = kalfa_sandbox::sunucu_ortami(&[], ozel);
        for (ad, deger, _) in ortam
            .ham_degerler()
            .filter(|(ad, _, _)| !ad.eq_ignore_ascii_case("PATH"))
        {
            // Kalfa'nın ürettiği özel HOME/cache/temp dizinleri ilk kayıtta
            // oluşmamış olabilir. Yalnız backend'in aynen zorladığı mutlak
            // dizin buna istisnadır; açık argüman/ortam yolu olamaz.
            let bilinen_dizin = Path::new(deger).is_absolute()
                && zorlanan_ortam.iter().any(|(sabit_ad, sabit_deger)| {
                    let ayni_ad = if cfg!(windows) {
                        ad.eq_ignore_ascii_case(sabit_ad)
                    } else {
                        ad == sabit_ad
                    };
                    ayni_ad && deger == sabit_deger
                });
            if bilinen_dizin {
                let yol = mutlak_kok(Path::new(deger), &cwd)?;
                match fs::metadata(&yol) {
                    Ok(m) if m.is_dir() => {}
                    Ok(_) => return Err(TanimHatasi::YolCozumlenemedi),
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                    Err(_) => return Err(TanimHatasi::YolCozumlenemedi),
                }
                kod_wde |= yol_wde_mi(&yol, &kokler)?;
                continue;
            }
            let ortam_adi = if cfg!(windows) {
                ad.to_ascii_uppercase()
            } else {
                ad.to_owned()
            };
            let yollar = match ortam_adi.as_str() {
                "NODE_OPTIONS" => node_ortam_yollarini_coz(deger, &cwd, &proje)?,
                "NODE_PATH" | "PYTHONPATH" | "RUBYLIB" | "PERL5LIB" | "CLASSPATH" => {
                    ortam_arama_yollarini_coz(deger, &cwd)?
                }
                _ => yol_degerlerini_coz(deger, &cwd, &proje)?,
            };
            for (referans_yolu, yol, dosya_mi) in yollar {
                kod_wde |= yol_wde_mi(&yol, &kokler)?;
                if dosya_mi {
                    aday_yollar.entry(yol).or_default().insert(referans_yolu);
                }
            }
        }
        let dosya_adaylari = aday_yollar
            .into_iter()
            .map(|(kanonik, referanslar)| {
                DosyaAdayi::yakala_kanonik_ve_referanslar(
                    kanonik,
                    referanslar.into_iter().collect(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CozulmusTanim {
            ad: self.ad.clone(),
            program: program_yolu,
            argumanlar,
            ortam,
            cwd,
            kip: self.kip,
            ag: self.ag.clone().unwrap_or_default(),
            kod_guveni: if kod_wde {
                KodGuveni::WdeKodlu
            } else {
                KodGuveni::WDisindaKodlu
            },
            dosya_adaylari,
            baslatma_zamani_asimi: self.baslatma_zamani_asimi,
            cagri_zamani_asimi: self.cagri_zamani_asimi,
            komut_tipi,
        })
    }
}

/// Çözülmüş değerler ham sır taşıyabilir; yine Debug/serde uygulanmaz.
pub struct CozulmusTanim {
    ad: String,
    program: PathBuf,
    argumanlar: Vec<String>,
    ortam: crate::ortam::SonOrtam,
    cwd: PathBuf,
    kip: CalistirmaKipi,
    ag: Vec<AgIzinKurali>,
    kod_guveni: KodGuveni,
    dosya_adaylari: Vec<DosyaAdayi>,
    baslatma_zamani_asimi: u32,
    cagri_zamani_asimi: u32,
    komut_tipi: u8,
}

impl CozulmusTanim {
    pub fn dosya_adaylari(&self) -> &[DosyaAdayi] {
        &self.dosya_adaylari
    }

    pub const fn kod_guveni(&self) -> KodGuveni {
        self.kod_guveni
    }

    pub fn calisma_dizini(&self) -> &Path {
        &self.cwd
    }

    pub(crate) fn program(&self) -> &Path {
        &self.program
    }

    pub(crate) fn argumanlar(&self) -> &[String] {
        &self.argumanlar
    }

    pub(crate) fn ortam(&self) -> &crate::ortam::SonOrtam {
        &self.ortam
    }

    pub(crate) fn kip(&self) -> CalistirmaKipi {
        self.kip
    }

    pub(crate) fn ag(&self) -> &[AgIzinKurali] {
        &self.ag
    }

    pub(crate) const fn timeoutlar(&self) -> (u32, u32) {
        (self.baslatma_zamani_asimi, self.cagri_zamani_asimi)
    }

    pub(crate) const fn komut_tipi(&self) -> u8 {
        self.komut_tipi
    }

    pub const fn calistirma_kipi(&self) -> CalistirmaKipi {
        self.kip
    }

    pub fn ag_kurallari(&self) -> &[AgIzinKurali] {
        &self.ag
    }

    pub const fn baslatma_zamani_asimi(&self) -> Duration {
        Duration::from_secs(self.baslatma_zamani_asimi as u64)
    }

    pub const fn cagri_zamani_asimi(&self) -> Duration {
        Duration::from_secs(self.cagri_zamani_asimi as u64)
    }

    /// Çözülmüş tanımdan etkileşimli süreç başlatma isteği üretir.
    pub fn etkilesimli_istek(
        &self,
        alan: kalfa_sandbox::AlanBilgisi,
    ) -> Result<kalfa_sandbox::EtkilesimliIstek, TanimHatasi> {
        let program = yol_metni(&self.program)?.to_owned();
        let ag_listesi = if self.kip == CalistirmaKipi::TamErisim || self.ag.is_empty() {
            None
        } else {
            let kurallar: Vec<kalfa_sandbox::vekil::HostKurali> = self
                .ag
                .iter()
                .map(|k| kalfa_sandbox::vekil::HostKurali {
                    host: k.host().to_owned(),
                    port: k.port(),
                    ozel_ag: k.ozel_ag(),
                    kaynak: kalfa_sandbox::vekil::HostKaynagi::Kullanici,
                })
                .collect();
            Some(kalfa_sandbox::vekil::IzinListesi::dogrudan(&kurallar))
        };
        Ok(kalfa_sandbox::EtkilesimliIstek {
            program,
            argumanlar: self.argumanlar.clone(),
            ortam: self.ortam.ham_ciftler(),
            kip: self.kip,
            sunucu: self.ad.clone(),
            alan,
            ag_listesi,
        })
    }

    /// npx/uvx ile çözülen paket kodu tanım özetine içerik olarak girmez.
    pub fn paket_surumu_uyarisi(&self) -> Option<&'static str> {
        paket_surumu_uyarisi(&self.program)
    }

    /// Yerel onay penceresinde gösterilir; sır ortam değerleri hiçbir zaman eklenmez.
    pub fn yerel_onay_metni(&self) -> String {
        self.maskeli_yerel_onay_metni(&[])
    }

    /// Yerel onay metnini hem tanımdaki gizli ortam değerleriyle hem de verilen ek
    /// sırlarla (örneğin kasadan okunan korunmuş sırlarla) maskeleyerek üretir.
    /// Argümanlar `Debug` kaçışından önce ayrı ayrı maskelenir; birleştirilen komut
    /// ve nihai metin de ayrıca maskelenir.
    pub fn maskeli_yerel_onay_metni(&self, ek_sirlar: &[crate::maske::GizliDeger]) -> String {
        let mut sirlar: Vec<crate::maske::GizliDeger> = ek_sirlar.to_vec();
        for (_, deger, gizli) in self.ortam.ham_degerler() {
            if gizli && let Ok(sir) = crate::maske::GizliDeger::yeni(deger.to_owned()) {
                sirlar.push(sir);
            }
        }
        let kip = match self.kip {
            CalistirmaKipi::Yalitimli => "Yalıtımlı",
            CalistirmaKipi::TamErisim => "Tam erişim",
        };
        let program_metni =
            crate::maske::maskele_metin(&self.program.display().to_string(), &sirlar);
        let birlesik_komut = std::iter::once(program_metni)
            .chain(self.argumanlar.iter().map(|a| {
                let maskeli = crate::maske::maskele_metin(a, &sirlar);
                format!("{maskeli:?}")
            }))
            .collect::<Vec<_>>()
            .join(" ");
        let komut = crate::maske::maskele_metin(&birlesik_komut, &sirlar);
        let cwd_metni = crate::maske::maskele_metin(&self.cwd.display().to_string(), &sirlar);
        let mut metin = format!(
            "Sunucu: {}\nKip: {kip}\nKomut: {komut}\nÇalışma dizini: {cwd_metni}\nOrtam adları:",
            self.ad,
        );
        for (ad, _, gizli) in self.ortam.ham_degerler() {
            metin.push_str("\n- ");
            metin.push_str(ad);
            if gizli {
                metin.push_str(" = [gizli]");
            }
        }
        if self.kip == CalistirmaKipi::Yalitimli {
            metin.push_str("\nAğ izinleri:");
            for kural in &self.ag {
                metin.push_str(&format!(
                    "\n- {}:{}{}",
                    kural.host,
                    kural.port,
                    if kural.ozel_ag { " (özel ağ)" } else { "" }
                ));
            }
        } else {
            metin.push_str("\nAğ izinleri: sınırsız");
            metin.push_str(
                "\nUyarı: Hesabınızın erişebildiği her şeye erişir. Sunucu kendiliğinden kapansa da geride çalışan süreçler kalabilir.",
            );
        }
        metin.push_str(&format!(
            "\nBaşlatma zaman aşımı: {} sn\nÇağrı zaman aşımı: {} sn",
            self.baslatma_zamani_asimi, self.cagri_zamani_asimi
        ));
        if let Some(uyari) = self.paket_surumu_uyarisi() {
            metin.push('\n');
            metin.push_str(uyari);
        }
        crate::maske::maskele_metin(&metin, &sirlar)
    }
}

fn sunucu_adi_gecerli(ad: &str) -> bool {
    !ad.is_empty()
        && ad.len() <= 32
        && ad
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn komut_dogrula(komut: &Komut) -> Result<(), TanimHatasi> {
    let argumanlar = match komut {
        Komut::ProgramAdi { ad, argumanlar } => {
            if !program_adi_gecerli(ad) {
                return Err(TanimHatasi::GecersizKomut);
            }
            argumanlar
        }
        Komut::SeciliYurutulebilir { argumanlar, .. } => argumanlar,
    };
    if argumanlar.iter().any(|a| a.contains('\0')) {
        return Err(TanimHatasi::GecersizKomut);
    }
    Ok(())
}

fn program_adi_gecerli(ad: &str) -> bool {
    !ad.is_empty()
        && ad != "."
        && ad != ".."
        && ad
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

fn komut_ortamini_coz(
    komut: &Komut,
    proje: &Path,
    cozucu: &dyn SeciliYurutulebilirCozucu,
) -> Result<(KomutBicimi, u8), TanimHatasi> {
    let (program, argumanlar, tip) = match komut {
        Komut::ProgramAdi { ad, argumanlar } => (ad.clone(), argumanlar, 1),
        Komut::SeciliYurutulebilir { kimlik, argumanlar } => {
            let yol = cozucu.coz(*kimlik)?;
            if !yol.is_absolute() {
                return Err(TanimHatasi::GoreliIkiliYolu);
            }
            (yol_metni(&yol)?.to_string(), argumanlar, 2)
        }
    };
    Ok((
        KomutBicimi::Program {
            program,
            argumanlar: argumanlar
                .iter()
                .map(|a| calisma_alani_yerine_koy(a, proje))
                .collect::<Result<_, _>>()?,
        },
        tip,
    ))
}

pub(crate) fn calisma_alani_yerine_koy(deger: &str, proje: &Path) -> Result<String, TanimHatasi> {
    if !deger.contains("${CALISMA_ALANI}") {
        return Ok(deger.to_owned());
    }
    let metin = yol_metni(proje)?;
    #[cfg(windows)]
    let metin = {
        // Windows verbatim yolda '/' ayıracını normalleştirmez. Yer tutucunun
        // '/dosya.js' ekiyle de çalışması için yalnız aynı hedefe çözüldüğü
        // doğrulanan normal yolu kullanırız; özel adlı yollar sessizce değişmez.
        let normal = if let Some(unc) = metin.strip_prefix(r"\\?\UNC\") {
            format!(r"\\{unc}")
        } else if let Some(disk) = metin.strip_prefix(r"\\?\") {
            disk.to_owned()
        } else {
            metin.to_owned()
        };
        if fs::canonicalize(&normal).map_err(|_| TanimHatasi::YolCozumlenemedi)? != proje {
            return Err(TanimHatasi::YolCozumlenemedi);
        }
        normal
    };
    #[cfg(windows)]
    let metin = metin.as_str();
    Ok(deger.replace("${CALISMA_ALANI}", metin))
}

fn yol_metni(yol: &Path) -> Result<&str, TanimHatasi> {
    yol.to_str().ok_or(TanimHatasi::YolCozumlenemedi)
}

fn paket_surumu_uyarisi(program: &Path) -> Option<&'static str> {
    let ad = program.file_stem()?.to_str()?.to_ascii_lowercase();
    matches!(ad.as_str(), "npx" | "uvx").then_some(NPX_UVX_SURUM_UYARISI)
}

fn kanonik_w_kokleri(kokler: &YazilabilirKokler, cwd: &Path) -> Result<Vec<PathBuf>, TanimHatasi> {
    let ham = vec![
        kokler.proje.clone(),
        kokler.kabuk_ozel.ev.clone(),
        kokler.kabuk_ozel.gecici.clone(),
        kokler.kabuk_ozel.onbellek.clone(),
    ];
    ham.into_iter().map(|p| mutlak_kok(&p, cwd)).collect()
}

fn yol_wde_mi(yol: &Path, kokler: &[PathBuf]) -> Result<bool, TanimHatasi> {
    let kanonik = mutlak_kok(yol, Path::new("."))?;
    Ok(kokler.iter().any(|kok| altinda(&kanonik, kok)))
}

fn altinda(yol: &Path, kok: &Path) -> bool {
    let yol: Vec<_> = yol.components().collect();
    let kok: Vec<_> = kok.components().collect();
    kok.len() <= yol.len()
        && yol.iter().zip(kok.iter()).all(|(a, b)| {
            if cfg!(windows) {
                a.as_os_str()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(&b.as_os_str().to_string_lossy())
            } else {
                a.as_os_str() == b.as_os_str()
            }
        })
}

/// Önce işletim sisteminin tam yol anlamını kullanır; Windows junction/..
/// ile Unix symlink/.. farklıdır. Eksik son dizinlerde mevcut bileşenleri
/// sırayla çözer. Dangling symlink ve erişim hatası güvenli rettir.
fn mutlak_kok(yol: &Path, cwd: &Path) -> Result<PathBuf, TanimHatasi> {
    let aday = if yol.is_absolute() {
        yol.to_path_buf()
    } else {
        match fs::canonicalize(cwd) {
            Ok(baslangic) => baslangic.join(yol),
            Err(e) if e.kind() == io::ErrorKind::NotFound && cwd.is_absolute() => cwd.join(yol),
            Err(_) => return Err(TanimHatasi::YolCozumlenemedi),
        }
    };
    match fs::canonicalize(&aday) {
        Ok(kanonik) => {
            yol_metni(&kanonik)?;
            return Ok(kanonik);
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(_) => return Err(TanimHatasi::YolCozumlenemedi),
    }
    let mut sonuc = PathBuf::new();
    let mut eksik = false;
    for parca in aday.components() {
        match parca {
            Component::Prefix(_) | Component::RootDir => sonuc.push(parca.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if eksik {
                    return Err(TanimHatasi::YolCozumlenemedi);
                }
                let _ = sonuc.pop();
            }
            Component::Normal(ad) => {
                sonuc.push(ad);
                if !eksik {
                    match fs::canonicalize(&sonuc) {
                        Ok(kanonik) => sonuc = kanonik,
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            match fs::symlink_metadata(&sonuc) {
                                Ok(m) if m.file_type().is_symlink() => {
                                    return Err(TanimHatasi::YolCozumlenemedi);
                                }
                                Ok(_) => eksik = true,
                                Err(e) if e.kind() == std::io::ErrorKind::NotFound => eksik = true,
                                Err(_) => return Err(TanimHatasi::YolCozumlenemedi),
                            }
                        }
                        Err(_) => return Err(TanimHatasi::YolCozumlenemedi),
                    }
                }
            }
        }
    }
    yol_metni(&sonuc)?;
    Ok(sonuc)
}

fn yol_degerlerini_coz(
    deger: &str,
    cwd: &Path,
    proje: &Path,
) -> Result<Vec<(PathBuf, PathBuf, bool)>, TanimHatasi> {
    let deger = calisma_alani_yerine_koy(deger, proje)?;
    if deger.contains(['\'', '"']) {
        return if deger.contains('/') || deger.contains('\\') {
            Err(TanimHatasi::YolCozumlenemedi)
        } else {
            Ok(Vec::new())
        };
    }
    let kucuk = deger.to_ascii_lowercase();
    if kucuk.contains("file://") {
        return Err(TanimHatasi::YolCozumlenemedi);
    }
    if deger.contains("://") {
        return if deger.split_ascii_whitespace().count() > 1 {
            Err(TanimHatasi::YolCozumlenemedi)
        } else {
            Ok(Vec::new())
        };
    }
    let olasi = match deger.split_once('=') {
        Some((sol, sag)) if sol.starts_with('-') && !sag.is_empty() => vec![sag],
        _ => vec![deger.as_str()],
    };
    let belirsiz_bilesik_deger =
        deger.split_ascii_whitespace().count() > 1 && (deger.contains('/') || deger.contains('\\'));
    if belirsiz_bilesik_deger
        && deger
            .split_ascii_whitespace()
            .skip(1)
            .any(|parca| parca.starts_with('-'))
    {
        return Err(TanimHatasi::YolCozumlenemedi);
    }
    let mut cozulmus = Vec::new();
    for parca in olasi {
        if parca.is_empty() || !yol_adayi_mi(parca, cwd)? {
            continue;
        }
        let yol = PathBuf::from(parca);
        let referans = if yol.is_absolute() {
            yol
        } else {
            cwd.join(yol)
        };
        let kanonik = mutlak_kok(&referans, cwd)?;
        let metadata = match fs::metadata(&kanonik) {
            Ok(m) => Some(m),
            // Bir yorumlayıcı uzantısız adı .js/.json gibi başka bir dosyaya
            // çözebilir. Bulunmayan açık yol, W dışında sabit kod kanıtı değildir.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(TanimHatasi::YolCozumlenemedi);
            }
            Err(_) => return Err(TanimHatasi::YolCozumlenemedi),
        };
        let dosya = metadata.as_ref().is_some_and(|m| m.is_file());
        let mevcut = metadata.is_some();
        cozulmus.push((referans, kanonik, dosya, mevcut));
    }
    if belirsiz_bilesik_deger {
        // Argüman tek argv öğesidir; ortam değeri tek metindir. Boşluklu
        // mevcut bir yol veya `--seçenek=yol` bütünüyle eşleşirse koru.
        // Birden çok token/yol ayrıştırıcısı eklemeden belirsiz olanı reddet.
        let mevcut: Vec<_> = cozulmus.iter().filter(|(_, _, _, var)| *var).collect();
        if mevcut.len() != 1 {
            return Err(TanimHatasi::YolCozumlenemedi);
        }
        return Ok(vec![(
            mevcut[0].0.clone(),
            mevcut[0].1.clone(),
            mevcut[0].2,
        )]);
    }
    if cozulmus.is_empty() && (deger.contains('/') || deger.contains('\\')) {
        return Err(TanimHatasi::YolCozumlenemedi);
    }
    let yollar = cozulmus
        .into_iter()
        .map(|(referans, kanonik, dosya, _)| (referans, kanonik, dosya))
        .collect::<BTreeSet<_>>();
    Ok(yollar.into_iter().collect())
}

/// Bilinen çalışma zamanlarının kod arama dizileri tek yol sayılmaz.
fn ortam_arama_yollarini_coz(
    deger: &str,
    cwd: &Path,
) -> Result<Vec<(PathBuf, PathBuf, bool)>, TanimHatasi> {
    let mut sonuc = Vec::new();
    for yol in std::env::split_paths(deger) {
        if yol.as_os_str().is_empty() {
            return Err(TanimHatasi::YolCozumlenemedi);
        }
        let referans = if yol.is_absolute() {
            yol
        } else {
            cwd.join(yol)
        };
        let kanonik = fs::canonicalize(&referans).map_err(|_| TanimHatasi::YolCozumlenemedi)?;
        yol_metni(&kanonik)?;
        let metadata = fs::metadata(&kanonik).map_err(|_| TanimHatasi::YolCozumlenemedi)?;
        if !metadata.is_file() && !metadata.is_dir() {
            return Err(TanimHatasi::YolCozumlenemedi);
        }
        sonuc.push((referans, kanonik, metadata.is_file()));
    }
    if sonuc.is_empty() {
        return Err(TanimHatasi::YolCozumlenemedi);
    }
    Ok(sonuc)
}

/// Node loader hedefinde uzantı/package-main/module-name tahmini yapılmaz.
/// Desteklenen hedef açık, mevcut normal dosyadır; öbür biçimler güvenli rettir.
fn node_ortam_yollarini_coz(
    deger: &str,
    cwd: &Path,
    proje: &Path,
) -> Result<Vec<(PathBuf, PathBuf, bool)>, TanimHatasi> {
    const YUKLEYICILER: &[&str] = &["--require", "--import", "--loader", "--experimental-loader"];
    if deger.contains(['\'', '"']) {
        return Err(TanimHatasi::YolCozumlenemedi);
    }
    let mut parcalar = deger.split_ascii_whitespace();
    let mut sonuc = Vec::new();
    while let Some(parca) = parcalar.next() {
        let hedef = if parca == "-r" || YUKLEYICILER.contains(&parca) {
            Some(parcalar.next().ok_or(TanimHatasi::YolCozumlenemedi)?)
        } else if let Some((anahtar, hedef)) = parca.split_once('=') {
            YUKLEYICILER.contains(&anahtar).then_some(hedef)
        } else {
            parca.strip_prefix("-r").filter(|hedef| !hedef.is_empty())
        };
        if let Some(hedef) = hedef {
            let hedef = calisma_alani_yerine_koy(hedef, proje)?;
            if !(Path::new(&hedef).is_absolute()
                || hedef.starts_with("./")
                || hedef.starts_with("../")
                || (cfg!(windows) && (hedef.starts_with(".\\") || hedef.starts_with("..\\"))))
            {
                return Err(TanimHatasi::YolCozumlenemedi);
            }
            let yollar = yol_degerlerini_coz(&hedef, cwd, proje)?;
            if yollar.len() != 1 || !yollar[0].2 {
                return Err(TanimHatasi::YolCozumlenemedi);
            }
            sonuc.extend(yollar);
        } else {
            if !parca.starts_with('-') {
                return Err(TanimHatasi::YolCozumlenemedi);
            }
            sonuc.extend(yol_degerlerini_coz(parca, cwd, proje)?);
        }
    }
    Ok(sonuc)
}

fn yol_adayi_mi(deger: &str, cwd: &Path) -> Result<bool, TanimHatasi> {
    if deger.contains("://") {
        return Ok(false);
    }
    let yol = Path::new(deger);
    if yol.is_absolute() || deger.contains('/') || deger.contains('\\') {
        return Ok(true);
    }
    match fs::metadata(cwd.join(yol)) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(TanimHatasi::YolCozumlenemedi),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program() -> Komut {
        Komut::ProgramAdi {
            ad: "node".into(),
            argumanlar: vec!["./sunucu.js".into()],
        }
    }

    #[test]
    fn uzantisiz_modul_yolu_ozetsiz_dis_kod_guveni_uretmez() {
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
            fs::create_dir_all(dizin).unwrap();
        }
        fs::write(ozel.ev.join("hook.js"), b"console.log('hook')").unwrap();
        let ikili = std::env::current_exe().unwrap();
        let tanim = McpTanim::varsayilan_zaman_asimlari(
            "ornek".into(),
            Komut::SeciliYurutulebilir {
                kimlik: SeciliYurutulebilirKimligi::yeni(9),
                argumanlar: vec![],
            },
            vec![OrtamDegeri::duz("NODE_OPTIONS", "--require=./hook")],
            CalistirmaKipi::Yalitimli,
            None,
        )
        .unwrap();
        assert!(matches!(
            tanim.hazirla(&[], &ozel, &YazilabilirKokler::yeni(proje, kabuk), &|_| Ok(
                ikili.clone()
            )),
            Err(TanimHatasi::YolCozumlenemedi)
        ));
    }

    #[test]
    fn node_path_icindeki_ikinci_w_koku_guven_denetimine_girer() {
        let kok = tempfile::tempdir().unwrap();
        let proje = kok.path().join("proje");
        let ozel = kalfa_sandbox::sunucu_ozel(kok.path(), "alan", "ornek");
        let kabuk = OzelDizinler::yol(kok.path(), "kabuk");
        let dis_moduller = kok.path().join("dis-moduller");
        let proje_modulleri = proje.join("node_modules");
        for dizin in [
            &proje,
            &ozel.ev,
            &ozel.gecici,
            &ozel.onbellek,
            &kabuk.ev,
            &kabuk.gecici,
            &kabuk.onbellek,
            &dis_moduller,
            &proje_modulleri,
        ] {
            fs::create_dir_all(dizin).unwrap();
        }
        fs::create_dir_all(proje_modulleri.join("pkg")).unwrap();
        fs::write(proje_modulleri.join("pkg/index.js"), b"module.exports=1").unwrap();
        let ikili = std::env::current_exe().unwrap();
        let node_path = std::env::join_paths([&dis_moduller, &proje_modulleri]).unwrap();
        let tanim = McpTanim::varsayilan_zaman_asimlari(
            "ornek".into(),
            Komut::SeciliYurutulebilir {
                kimlik: SeciliYurutulebilirKimligi::yeni(9),
                argumanlar: vec![],
            },
            vec![OrtamDegeri::duz("NODE_PATH", node_path.to_str().unwrap())],
            CalistirmaKipi::Yalitimli,
            None,
        )
        .unwrap();
        let cozulmus = tanim
            .hazirla(&[], &ozel, &YazilabilirKokler::yeni(proje, kabuk), &|_| {
                Ok(ikili.clone())
            })
            .unwrap();
        assert_eq!(cozulmus.kod_guveni(), KodGuveni::WdeKodlu);
    }

    #[test]
    fn node_yukleyici_belirsiz_modulu_ve_dizini_reddeder_tam_dosyayi_ozetler() {
        let kok = tempfile::tempdir().unwrap();
        fs::write(kok.path().join("hook.js"), b"module.exports=1").unwrap();
        fs::create_dir(kok.path().join("paket")).unwrap();
        fs::write(kok.path().join("paket/index.js"), b"module.exports=2").unwrap();
        for deger in [
            "--require=./hook",
            "--require=./paket",
            "--require=pkg",
            "-r pkg",
            "--import=data:text/javascript,0",
        ] {
            assert!(matches!(
                node_ortam_yollarini_coz(deger, kok.path(), kok.path()),
                Err(TanimHatasi::YolCozumlenemedi)
            ));
        }
        for deger in [
            "--require=./hook.js",
            "--require ./hook.js",
            "-r./hook.js",
            "--import=./hook.js",
        ] {
            let yollar = node_ortam_yollarini_coz(deger, kok.path(), kok.path()).unwrap();
            assert_eq!(yollar.len(), 1);
            assert!(yollar[0].2);
            assert_eq!(
                yollar[0].1,
                fs::canonicalize(kok.path().join("hook.js")).unwrap()
            );
        }
    }

    #[test]
    fn dosya_uzantisi_tek_basina_yol_kaniti_sayilmaz() {
        let kok = tempfile::tempdir().unwrap();
        for deger in ["C.UTF-8", ".COM;.EXE;.BAT;.CMD", "49.0.3"] {
            assert!(
                yol_degerlerini_coz(deger, kok.path(), kok.path())
                    .unwrap()
                    .is_empty()
            );
        }
        fs::write(kok.path().join("hook.js"), b"module.exports=1").unwrap();
        let yollar = yol_degerlerini_coz("hook.js", kok.path(), kok.path()).unwrap();
        assert!(yollar[0].2);
        assert_eq!(
            yollar[0].1,
            fs::canonicalize(kok.path().join("hook.js")).unwrap()
        );
    }

    #[test]
    fn goreli_path_programi_sunucunun_ozel_evinde_bulur() {
        let kok = tempfile::tempdir().unwrap();
        let proje = kok.path().join("proje");
        let ozel = kalfa_sandbox::sunucu_ozel(kok.path(), "alan", "ornek");
        let kabuk = OzelDizinler::yol(kok.path(), "kabuk");
        let bin = ozel.ev.join("bin");
        for dizin in [
            &proje,
            &ozel.ev,
            &ozel.gecici,
            &ozel.onbellek,
            &kabuk.ev,
            &kabuk.gecici,
            &kabuk.onbellek,
            &bin,
        ] {
            fs::create_dir_all(dizin).unwrap();
        }
        let ikili = bin.join(if cfg!(windows) { "npx.exe" } else { "npx" });
        fs::write(&ikili, b"yalniz yol cozumu").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&ikili, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let tanim = McpTanim::varsayilan_zaman_asimlari(
            "ornek".into(),
            Komut::ProgramAdi {
                ad: "npx".into(),
                argumanlar: vec![],
            },
            vec![],
            CalistirmaKipi::Yalitimli,
            None,
        )
        .unwrap();
        let cozulmus = tanim
            .hazirla(
                &[("PATH".into(), "bin".into())],
                &ozel,
                &YazilabilirKokler::yeni(proje, kabuk),
                &|_| Err(TanimHatasi::KomutCozulemedi),
            )
            .unwrap();
        assert_eq!(cozulmus.program(), fs::canonicalize(ikili).unwrap());
        assert_eq!(
            cozulmus.calisma_dizini(),
            fs::canonicalize(ozel.ev).unwrap()
        );
        assert_eq!(cozulmus.kod_guveni(), KodGuveni::WDisindaKodlu);
        assert_eq!(cozulmus.paket_surumu_uyarisi(), Some(NPX_UVX_SURUM_UYARISI));
    }

    #[test]
    fn kod_arama_listesinde_bos_ve_eksik_girdi_guvenilir_sayilmaz() {
        let kok = tempfile::tempdir().unwrap();
        let boslu = std::env::join_paths([kok.path(), Path::new("")]).unwrap();
        let eksik = std::env::join_paths([kok.path(), &kok.path().join("yok")]).unwrap();
        for deger in [boslu, eksik] {
            assert!(matches!(
                ortam_arama_yollarini_coz(deger.to_str().unwrap(), kok.path()),
                Err(TanimHatasi::YolCozumlenemedi)
            ));
        }
    }

    #[test]
    fn ad_komut_timeout_ve_tam_erisim_ag_kurali_dogrulanir() {
        assert!(
            McpTanim::varsayilan_zaman_asimlari(
                "gecerli-2".into(),
                program(),
                vec![],
                CalistirmaKipi::TamErisim,
                None
            )
            .is_ok()
        );
        assert!(matches!(
            McpTanim::yeni(
                "../disari".into(),
                program(),
                vec![],
                CalistirmaKipi::Yalitimli,
                None,
                1,
                1
            ),
            Err(TanimHatasi::GecersizAd)
        ));
        assert!(matches!(
            McpTanim::yeni(
                "gecerli".into(),
                program(),
                vec![],
                CalistirmaKipi::TamErisim,
                Some(vec![AgIzinKurali::yeni("example.com", false).unwrap()]),
                1,
                1
            ),
            Err(TanimHatasi::TamErisimAgListesi)
        ));
        assert!(matches!(
            McpTanim::yeni(
                "gecerli".into(),
                program(),
                vec![],
                CalistirmaKipi::Yalitimli,
                None,
                0,
                1
            ),
            Err(TanimHatasi::SifirZamanAsimi)
        ));
    }

    #[test]
    fn program_adi_yol_ve_shell_metni_olamaz() {
        let e = McpTanim::yeni(
            "gecerli".into(),
            Komut::ProgramAdi {
                ad: "./node".into(),
                argumanlar: vec![],
            },
            vec![],
            CalistirmaKipi::Yalitimli,
            None,
            1,
            1,
        );
        assert!(matches!(e, Err(TanimHatasi::GecersizKomut)));
        for ad in ["node -e", "node;whoami", "node|sh", "C:\\tools\\node.exe"] {
            assert!(
                McpTanim::yeni(
                    "gecerli".into(),
                    Komut::ProgramAdi {
                        ad: ad.into(),
                        argumanlar: vec![]
                    },
                    vec![],
                    CalistirmaKipi::Yalitimli,
                    None,
                    1,
                    1
                )
                .is_err()
            );
        }
    }

    #[test]
    fn secili_ikili_yolu_yalniz_arka_uc_kimlik_cozucusunden_gelir() {
        let kimlik = SeciliYurutulebilirKimligi::yeni(42);
        let secili = Komut::SeciliYurutulebilir {
            kimlik,
            argumanlar: vec![],
        };
        let yol = std::env::current_exe().unwrap();
        let cozulmus = komut_ortamini_coz(&secili, Path::new("."), &|alinan| {
            assert!(alinan == kimlik);
            Ok(yol.clone())
        })
        .unwrap();
        assert_eq!(cozulmus.1, 2);
        assert!(matches!(cozulmus.0, KomutBicimi::Program { .. }));
        assert!(matches!(
            komut_ortamini_coz(&secili, Path::new("."), &|_| Ok(PathBuf::from(
                "goreli.exe"
            ))),
            Err(TanimHatasi::GoreliIkiliYolu)
        ));
    }

    #[test]
    fn npx_ve_uvx_uyarisi_guvenli_sabit_metindir() {
        assert_eq!(
            paket_surumu_uyarisi(Path::new("/usr/bin/npx")),
            Some(NPX_UVX_SURUM_UYARISI)
        );
        assert_eq!(
            paket_surumu_uyarisi(Path::new("/usr/bin/uvx.exe")),
            Some(NPX_UVX_SURUM_UYARISI)
        );
        assert_eq!(paket_surumu_uyarisi(Path::new("/usr/bin/node")), None);
    }

    #[test]
    fn tek_arguman_ve_esittir_sonrasi_bosluklu_dosya_yolu_butun_korunur() {
        let kok = tempfile::tempdir().unwrap();
        let dizin = kok.path().join("my plugins");
        fs::create_dir_all(&dizin).unwrap();
        let dosya = dizin.join("hook.js");
        fs::write(&dosya, b"hook").unwrap();
        let dosya_metni = dosya.to_str().unwrap();
        let beklenen = fs::canonicalize(&dosya).unwrap();

        assert_eq!(
            yol_degerlerini_coz(dosya_metni, kok.path(), kok.path()).unwrap(),
            vec![(dosya.clone(), beklenen.clone(), true)]
        );
        assert_eq!(
            yol_degerlerini_coz(&format!("--require={dosya_metni}"), kok.path(), kok.path())
                .unwrap(),
            vec![(dosya.clone(), beklenen, true)]
        );
    }

    #[test]
    fn ortamda_belirsiz_birden_cok_yol_guvenilir_dis_kod_sayilmaz() {
        let kok = tempfile::tempdir().unwrap();
        let bir = kok.path().join("bir.js");
        let iki = kok.path().join("iki.js");
        fs::write(&bir, b"bir").unwrap();
        fs::write(&iki, b"iki").unwrap();
        let deger = format!("--require {} --import {}", bir.display(), iki.display());
        assert!(matches!(
            yol_degerlerini_coz(&deger, kok.path(), kok.path()),
            Err(TanimHatasi::YolCozumlenemedi)
        ));
    }

    #[cfg(windows)]
    fn junction_kur(yol: &Path, hedef: &Path) {
        let cikti = std::process::Command::new("cmd.exe")
            .args(["/D", "/C", "mklink", "/J"])
            .arg(yol)
            .arg(hedef)
            .output()
            .unwrap();
        assert!(cikti.status.success(), "junction kurulamadı: {cikti:?}");
    }

    #[cfg(windows)]
    #[test]
    fn junction_ve_ebeveyn_yolu_windows_ile_ayni_hedefe_cozulur() {
        let kok = tempfile::tempdir().unwrap();
        let temel = kok.path().join("temel");
        let hedef = kok.path().join("hedef").join("alt");
        for dizin in [
            &temel,
            &hedef,
            &temel.join("dosyalar"),
            &kok.path().join("hedef").join("dosyalar"),
        ] {
            fs::create_dir_all(dizin).unwrap();
        }
        let bag = temel.join("bag");
        junction_kur(&bag, &hedef);
        let referans = bag.join("..").join("dosyalar");
        assert_eq!(
            mutlak_kok(&referans, kok.path()).unwrap(),
            fs::canonicalize(&referans).unwrap()
        );
    }

    #[cfg(windows)]
    #[test]
    fn dosya_adayi_junction_hedefi_degisimini_yakalar() {
        let kok = tempfile::tempdir().unwrap();
        let bir = kok.path().join("bir");
        let iki = kok.path().join("iki");
        fs::create_dir_all(&bir).unwrap();
        fs::create_dir_all(&iki).unwrap();
        fs::write(bir.join("hook.js"), b"bir").unwrap();
        fs::write(iki.join("hook.js"), b"iki").unwrap();
        let bag = kok.path().join("bag");
        junction_kur(&bag, &bir);
        let aday = DosyaAdayi::yakala(bag.join("hook.js")).unwrap();
        let acilan = File::open(aday.yol()).unwrap();
        assert!(aday.kimlik_uyuyor(&acilan).unwrap());
        fs::remove_dir(&bag).unwrap();
        junction_kur(&bag, &iki);
        assert!(!aday.kimlik_uyuyor(&acilan).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_ardindan_ebeveyn_bileseni_gercek_kokten_cozulur() {
        use std::os::unix::fs::symlink;

        let kok = tempfile::tempdir().unwrap();
        let temel = kok.path().join("temel");
        let hedef_dizin = kok.path().join("hedef").join("alt");
        fs::create_dir_all(&temel).unwrap();
        fs::create_dir_all(&hedef_dizin).unwrap();
        fs::create_dir_all(kok.path().join("hedef").join("dosyalar")).unwrap();
        fs::create_dir_all(temel.join("dosyalar")).unwrap();
        symlink(&hedef_dizin, temel.join("bag")).unwrap();

        let cozulmus =
            mutlak_kok(&temel.join("bag").join("..").join("dosyalar"), kok.path()).unwrap();
        let beklenen = fs::canonicalize(kok.path().join("hedef").join("dosyalar")).unwrap();
        assert_eq!(cozulmus, beklenen);
    }

    #[cfg(unix)]
    #[test]
    fn dangling_symlink_eksik_dizin_gibi_kabul_edilmez() {
        use std::os::unix::fs::symlink;

        let kok = tempfile::tempdir().unwrap();
        symlink(kok.path().join("yok"), kok.path().join("dangling")).unwrap();
        assert!(matches!(
            mutlak_kok(&kok.path().join("dangling"), kok.path()),
            Err(TanimHatasi::YolCozumlenemedi)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn secili_yurutulebilirde_utf8siz_yol_guvenli_hatayla_reddedilir() {
        use std::os::unix::ffi::OsStringExt;

        let kimlik = SeciliYurutulebilirKimligi::yeni(43);
        let secili = Komut::SeciliYurutulebilir {
            kimlik,
            argumanlar: vec![],
        };
        let yol = PathBuf::from(std::ffi::OsString::from_vec(vec![
            b'/', b't', b'm', b'p', b'/', 0x80,
        ]));
        assert!(matches!(
            komut_ortamini_coz(&secili, Path::new("."), &|_| Ok(yol.clone())),
            Err(TanimHatasi::YolCozumlenemedi)
        ));
    }

    #[test]
    fn dosya_adayi_yalniz_yakalanan_dosya_kimligini_kabul_eder() {
        let kok = tempfile::tempdir().unwrap();
        let bir = kok.path().join("bir.js");
        let iki = kok.path().join("iki.js");
        fs::write(&bir, b"bir").unwrap();
        fs::write(&iki, b"iki").unwrap();

        let aday = DosyaAdayi::yakala(bir).unwrap();
        let acilan_bir = File::open(aday.yol()).unwrap();
        let acilan_iki = File::open(iki).unwrap();
        assert!(aday.kimlik_uyuyor(&acilan_bir).unwrap());
        assert!(!aday.kimlik_uyuyor(&acilan_iki).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn dosya_adayi_symlink_hedefi_degisimini_yakalar() {
        use std::os::unix::fs::symlink;

        let kok = tempfile::tempdir().unwrap();
        let bir = kok.path().join("bir.js");
        let iki = kok.path().join("iki.js");
        let bag = kok.path().join("hook.js");
        fs::write(&bir, b"bir").unwrap();
        fs::write(&iki, b"iki").unwrap();
        symlink(&bir, &bag).unwrap();

        let aday = DosyaAdayi::yakala(bag.clone()).unwrap();
        let acilan = File::open(aday.yol()).unwrap();
        assert!(aday.kimlik_uyuyor(&acilan).unwrap());

        fs::remove_file(&bag).unwrap();
        symlink(&iki, &bag).unwrap();
        assert!(!aday.kimlik_uyuyor(&acilan).unwrap());
    }

    #[test]
    fn yerel_onay_metni_arguman_ve_kacisli_sirlari_maskeler() {
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
            fs::create_dir_all(dizin).unwrap();
        }
        let ikili = std::env::current_exe().unwrap();
        let kacisli_sir = "sir\"kacis\"deger99";
        let tanim = McpTanim::varsayilan_zaman_asimlari(
            "ornek".into(),
            Komut::SeciliYurutulebilir {
                kimlik: SeciliYurutulebilirKimligi::yeni(1),
                argumanlar: vec![format!("--token={kacisli_sir}")],
            },
            vec![OrtamDegeri::gizli("API_KEY", kacisli_sir)],
            CalistirmaKipi::TamErisim,
            None,
        )
        .unwrap();
        let cozulmus = tanim
            .hazirla(&[], &ozel, &YazilabilirKokler::yeni(proje, kabuk), &|_| {
                Ok(ikili.clone())
            })
            .unwrap();
        let onay_metni = cozulmus.yerel_onay_metni();
        assert!(
            !onay_metni.contains("kacis"),
            "yerel onay metni argümandaki kaçışlı sırrı maskelemeli: {onay_metni}"
        );
        assert!(onay_metni.contains("sınırsız"));
        assert!(onay_metni.contains("geride çalışan süreçler kalabilir"));
    }

    #[test]
    fn uzak_mcp_tanim_https_kanoniklestirir_ve_yerel_tanimdan_ayrilir() {
        let uzak = UzakMcpTanim::yeni(
            "uzak-ornek".into(),
            "https://MCP.Example.com:443/v1/mcp?kip=test".into(),
            UzakYetkiTuru::OAuth {
                istemci_kimligi: Some("kalfa-istemci".into()),
                kapsamlar: vec![
                    "tools:call".into(),
                    "tools:read".into(),
                    "tools:call".into(),
                ],
                kaynak_url: Some("https://MCP.Example.com:443/v1/mcp".into()),
                yetki_kimligi: Some("mcp-oauth-1".into()),
            },
            30,
            120,
            false,
            UzakBaglamButcesi::varsayilan(),
        )
        .unwrap();

        assert_eq!(uzak.ad(), "uzak-ornek");
        assert_eq!(uzak.uc_url(), "https://mcp.example.com/v1/mcp?kip=test");
        assert_eq!(uzak.origin(), "https://mcp.example.com");
        assert_eq!(uzak.izinli_baslik_kimlikleri(), &["oauth:mcp-oauth-1"]);
        assert_eq!(
            uzak.yetki_turu(),
            &UzakYetkiTuru::OAuth {
                istemci_kimligi: Some("kalfa-istemci".into()),
                kapsamlar: vec!["tools:call".into(), "tools:read".into()],
                kaynak_url: Some("https://mcp.example.com/v1/mcp".into()),
                yetki_kimligi: Some("mcp-oauth-1".into()),
            }
        );

        let kapsayici = McpSunucuTanimi::Uzak(uzak.clone());
        assert!(kapsayici.uzak_mi());
        assert_eq!(kapsayici.ad(), "uzak-ornek");

        let sir = crate::maske::GizliDeger::yeni("cok-gizli-oauth-token-123").unwrap();
        let onay = uzak.maskeli_uzak_onay_metni(&[sir]);
        assert!(onay.contains("Tür: Uzak MCP (Streamable HTTP)"));
        assert!(onay.contains("Uç: https://mcp.example.com/v1/mcp?kip=test"));
        assert!(onay.contains("OAuth 2.0 (kapsamlar: tools:call, tools:read)"));
        assert!(!onay.contains("cok-gizli-oauth-token-123"));
        assert!(!onay.contains("Yalıtımlı"));
        assert!(!onay.contains("Tam erişim"));

        // Loopback ve yasaklı başlık sınırları da aynı test modülünde doğrulanır.
    }

    #[test]
    fn uzak_mcp_tanim_http_loopback_ve_yasakli_baslik_sinirlarini_zorlar() {
        assert_eq!(
            UzakMcpTanim::varsayilan_zaman_asimlari(
                "yerel-http".into(),
                "http://127.0.0.1:8080/mcp".into(),
                UzakYetkiTuru::Yetkisiz,
            )
            .unwrap_err(),
            TanimHatasi::GecersizUzakUrl
        );

        let fikstur = UzakMcpTanim::yeni(
            "yerel-http".into(),
            "http://127.0.0.1:8080/mcp".into(),
            UzakYetkiTuru::Yetkisiz,
            10,
            30,
            true,
            UzakBaglamButcesi::varsayilan(),
        )
        .unwrap();
        assert!(fikstur.test_fiksturu_loopback());
        assert_eq!(
            UzakMcpTanim::yeni(
                "dis-http".into(),
                "http://example.com/mcp".into(),
                UzakYetkiTuru::Yetkisiz,
                10,
                30,
                true,
                UzakBaglamButcesi::varsayilan(),
            )
            .unwrap_err(),
            TanimHatasi::GecersizUzakUrl
        );

        for gecersiz_url in [
            "https://kullanici:parola@example.com/mcp",
            "https://example.com/mcp#frag",
        ] {
            assert_eq!(
                UzakMcpTanim::varsayilan_zaman_asimlari(
                    "ornek".into(),
                    gecersiz_url.into(),
                    UzakYetkiTuru::Yetkisiz,
                )
                .unwrap_err(),
                TanimHatasi::GecersizUzakUrl
            );
        }

        for yasakli_baslik in ["Host", "Cookie", "Content-Length", " custom "] {
            assert_eq!(
                UzakMcpTanim::varsayilan_zaman_asimlari(
                    "ornek".into(),
                    "https://example.com/mcp".into(),
                    UzakYetkiTuru::SabitBaslikGizli {
                        baslik_adi: yasakli_baslik.into(),
                        gizli_kimlik: "mcp-123".into(),
                    },
                )
                .unwrap_err(),
                TanimHatasi::GecersizUzakYetki
            );
        }
    }
}
