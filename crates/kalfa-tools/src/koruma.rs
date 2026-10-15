//! Kabuk aracı ve MCP süreç yöneticisi tarafından paylaşılan çalışma alanı
//! koruma taraması, yalıtım hazırlığı ve yürütme sonrası denetim (1.1a §4.3).
//!
//! Bu modül:
//! - Gizli dosya/dizinleri, her derinlikteki `.git` ve yönlendirilen git
//!   hedeflerini, ayrıca kendiliğinden çalışabilecek yapılandırma yollarını
//!   tek ortak kuralla tarar (`alan_bilgisi`),
//! - Yalnızca `Tarama::koruma_tam == true` olduğunda kanonik `McpKorumaOzeti`
//!   (`K`) üretir; görüntü sınırı (`goruntu.eksik`) koruma tamlığını bozmaz,
//!   okunamayan/ayrıştırılamayan girdi, meta veri/hardlink hatası, iptal veya
//!   30 sn koruma zaman aşımı ise `KorumaSonucu::Dogrulanamadi` döner,
//! - Yürütme öncesi/sonrası `.git` karantinasını ve yeni yapılandırma
//!   uyarılarını (`komut_sonrasi_denetim`) kabuk ve MCP için ortaklaştırır.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use kalfa_core::arac::{
    AracHatasi, CiktiGonderici, DegisiklikTuru, DosyaDegisiklikleri, McpKorumaOzeti,
};
use kalfa_core::git_koruma;
use kalfa_core::izin::Siniflandirici;
use kalfa_core::protokol::CiktiAkisi;
use kalfa_core::yol::CalismaYolu;
use kalfa_sandbox::git_denetimi::{GitDenetimi, yeni_git_denetle};
use kalfa_sandbox::tarama::{self, AdliGirdiler, KorumaTaramaHatasi, TaramaSecenekleri};
use kalfa_sandbox::{AlanBilgisi, Korumali, Yalitici};

use crate::calisma_alani::CalismaAlani;

pub const GIT_DENETLENEMEDI: &str = "Komuttan sonra yeni `.git` denetlenemedi";

fn gecersiz(m: impl Into<String>) -> AracHatasi {
    AracHatasi::Gecersiz(m.into())
}

/// Komut ya da MCP çağrısından sonraki denetimin sonucu.
#[derive(Debug, Clone)]
pub struct SonDenetimSonucu {
    pub uyarilar: Vec<String>,
    pub karantina_basarisiz: bool,
    /// Yürüyüş, meta veri ve yol ayrıştırması hatasız tamamlandı mı?
    pub tarama_tam: bool,
    /// Tarama yapılamadıysa `None`: değişiklikler bilinmiyor.
    pub degisiklikler: Option<DosyaDegisiklikleri>,
    /// Sonraki denetimde temel alınabilecek güncel adli girdiler (karantinaya
    /// alınan `.git` girdileri çıkarılmış hâliyle).
    pub guncel_adli_girdiler: Option<AdliGirdiler>,
}

impl SonDenetimSonucu {
    pub fn denetlenemedi(e: impl std::fmt::Display) -> Self {
        Self {
            uyarilar: vec![format!("{GIT_DENETLENEMEDI}: {e}")],
            karantina_basarisiz: true,
            tarama_tam: false,
            degisiklikler: None,
            guncel_adli_girdiler: None,
        }
    }

    /// MCP zorunlu son denetimi hem taramanın tam olmasını hem de tüm tehlikeli
    /// `.git` karantinalarının başarılı olmasını şart koşar (1.1a §7.3).
    pub fn mcp_icin_basarili_mi(&self) -> bool {
        self.tarama_tam && !self.karantina_basarisiz
    }
}

/// MCP koruma doğrulamasının başarısızlık nedeni (1.1a §4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KorumaDogrulamaHatasi {
    AlanCozulemedi(String),
    TaramaEksik(KorumaTaramaHatasi),
    HazirlikBasarisiz(String),
}

impl KorumaDogrulamaHatasi {
    /// Ayarlar'da ve hata durumunda gösterilecek kısa Türkçe açıklama.
    pub fn kullanici_mesaji(&self) -> String {
        match self {
            Self::AlanCozulemedi(m) => format!("çalışma alanı çözülemedi: {m}"),
            Self::TaramaEksik(KorumaTaramaHatasi::Okunamadi {
                yol: Some(yol),
                mesaj,
            }) => format!(
                "koruma taraması `{}` yolunu okuyamadı: {mesaj}",
                yol.display()
            ),
            Self::TaramaEksik(KorumaTaramaHatasi::Okunamadi { yol: None, mesaj }) => {
                format!("koruma taraması tamamlanamadı: {mesaj}")
            }
            Self::TaramaEksik(KorumaTaramaHatasi::ZamanAsimi) => {
                "koruma taraması 30 sn sınırını aştı".into()
            }
            Self::TaramaEksik(KorumaTaramaHatasi::Iptal) => "koruma taraması iptal edildi".into(),
            Self::HazirlikBasarisiz(m) => format!("çalışma alanı yalıtım hazırlığı başarısız: {m}"),
        }
    }
}

/// Doğrulanmış çalışma alanı koruması ve kanonik `K` özeti.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DogrulanmisKoruma {
    pub alan_bilgisi: AlanBilgisi,
    pub adli_girdiler: AdliGirdiler,
    pub ozet: McpKorumaOzeti,
}

/// Koruma karşılaştırmasının üç durumlu sonucu (1.1a §4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KorumaSonucu {
    Ayni(DogrulanmisKoruma),
    Degisti(DogrulanmisKoruma),
    Dogrulanamadi(KorumaDogrulamaHatasi),
}

/// Çalışma alanının yalıtım bilgisini varsayılan seçeneklerle senkron üretir.
pub fn alan_bilgisi_senkron(alan: &CalismaAlani) -> Result<AlanBilgisi, AracHatasi> {
    alan_bilgisi_secenekli_senkron(alan, &TaramaSecenekleri::default())
}

/// Çalışma alanının yalıtım bilgisini verilen seçeneklerle senkron üretir.
pub fn alan_bilgisi_secenekli_senkron(
    alan: &CalismaAlani,
    secenekler: &TaramaSecenekleri,
) -> Result<AlanBilgisi, AracHatasi> {
    let kok = alan
        .kok()
        .canonicalize()
        .map_err(|e| gecersiz(format!("Çalışma alanının yolu çözülemedi: {e}")))?;
    let s = alan.siniflandirici();
    let ayristirma_hatasi: Mutex<Option<PathBuf>> = Mutex::new(None);
    let kayit_et = |yol: &Path| {
        let mut kilit = ayristirma_hatasi.lock().unwrap_or_else(|z| z.into_inner());
        if kilit.is_none() {
            *kilit = Some(yol.to_path_buf());
        }
    };
    let mut gizli = tarama::tara_ayrintili_secenekli(
        &kok,
        &|yol, _dizin| match goreli_calisma_yolu(yol) {
            Some(y) => s.gizli_mi(&y),
            None => {
                kayit_et(yol);
                false
            }
        },
        &git_adi_mi,
        &|yol, dizin| match goreli_calisma_yolu(yol) {
            Some(y) => yapilandirma_yolu_mu(s, &y, dizin),
            None => {
                kayit_et(yol);
                false
            }
        },
        secenekler,
    );
    if let Some(hatali) = ayristirma_hatasi
        .into_inner()
        .unwrap_or_else(|z| z.into_inner())
    {
        gizli.koruma_hata_kaydet(KorumaTaramaHatasi::Okunamadi {
            yol: Some(kok.join(hatali)),
            mesaj: "Yürüyüş girdisi çalışma alanı yolu olarak ayrıştırılamadı.".into(),
        });
    }
    let git = git_koruma::git_yerlesimi(&kok);
    let mut korumali: Vec<Korumali> = git
        .korumali
        .into_iter()
        .map(|k| Korumali {
            yol: k.yol,
            dizin: k.dizin,
        })
        .collect();
    // Bağlantı olan `.git` izlenmez ve bağlanmaz (bilinen sınır).
    for g in gizli.git.iter().filter(|g| !g.baglanti) {
        korumali.push(Korumali {
            yol: g.yol.clone(),
            dizin: g.dizin,
        });
        for yol in git_koruma::yonlendirilen_dizinler(&g.yol, g.dizin, &kok) {
            korumali.push(Korumali { yol, dizin: true });
        }
    }
    // Kendiliğinden çalışan yapılandırma da salt okunurdur (Bulgu 2).
    for y in &gizli.yapilandirma {
        korumali.push(Korumali {
            yol: y.yol.clone(),
            dizin: y.dizin,
        });
    }
    korumali.retain(|k| {
        !gizli.dizinler.iter().any(|d| k.yol.starts_with(d)) && !gizli.dosyalar.contains(&k.yol)
    });
    korumali.sort_by(|a, b| a.yol.cmp(&b.yol));
    korumali.dedup_by(|a, b| a.yol == b.yol);
    Ok(AlanBilgisi {
        kok,
        gizli,
        korumali,
        git_dizinleri: git.dis_dizinler,
    })
}

/// Çalışma alanının yalıtım bilgisini bloklayan iş parçacığında üretir.
pub async fn alan_bilgisi(alan: Arc<CalismaAlani>) -> Result<AlanBilgisi, AracHatasi> {
    alan_bilgisi_secenekli(alan, TaramaSecenekleri::default()).await
}

/// Çalışma alanının yalıtım bilgisini verilen seçeneklerle bloklayan iş
/// parçacığında üretir.
pub async fn alan_bilgisi_secenekli(
    alan: Arc<CalismaAlani>,
    secenekler: TaramaSecenekleri,
) -> Result<AlanBilgisi, AracHatasi> {
    tokio::task::spawn_blocking(move || alan_bilgisi_secenekli_senkron(&alan, &secenekler))
        .await
        .map_err(|e| gecersiz(format!("Gizli dosya taraması yarıda kaldı: {e}")))?
}

/// Çalışma alanını yalıtım için hazırlar (§9.4); ilerleme araç kartında
/// görünür, modele gitmez.
pub async fn alani_hazirla(
    yalitici: Arc<dyn Yalitici>,
    alan: &AlanBilgisi,
    onaylandi: bool,
    gonderici: &CiktiGonderici,
) -> Result<(), AracHatasi> {
    let alan = alan.clone();
    let gonderici = gonderici.clone();
    tokio::task::spawn_blocking(move || {
        yalitici.alani_hazirla(&alan, onaylandi, &mut |satir| {
            gonderici.gonder(CiktiAkisi::Standart, format!("{satir}\n"));
        })
    })
    .await
    .map_err(|e| gecersiz(format!("Çalışma alanının hazırlığı yarıda kaldı: {e}")))?
    .map_err(|e| gecersiz(e.to_string()))
}

/// Çalışma alanındaki `.git` girdileri ve yapılandırma yollarını senkron
/// toplar.
pub fn adli_girdiler_senkron(
    alan: &CalismaAlani,
    secenekler: &TaramaSecenekleri,
) -> Result<AdliGirdiler, AracHatasi> {
    let kok = alan
        .kok()
        .canonicalize()
        .map_err(|e| gecersiz(format!("Çalışma alanının yolu çözülemedi: {e}")))?;
    Ok(adli_girdiler_kok_ile(
        &kok,
        alan.siniflandirici(),
        secenekler,
    ))
}

fn adli_girdiler_kok_ile(
    kok: &Path,
    s: &Siniflandirici,
    secenekler: &TaramaSecenekleri,
) -> AdliGirdiler {
    let ayristirma_hatasi: Mutex<Option<PathBuf>> = Mutex::new(None);
    let mut sonuc = tarama::adli_girdiler_secenekli(
        kok,
        &git_adi_mi,
        &|yol, dizin| match goreli_calisma_yolu(yol) {
            Some(y) => yapilandirma_yolu_mu(s, &y, dizin),
            None => {
                let mut kilit = ayristirma_hatasi.lock().unwrap_or_else(|z| z.into_inner());
                if kilit.is_none() {
                    *kilit = Some(yol.to_path_buf());
                }
                false
            }
        },
        secenekler,
    );
    if let Some(hatali) = ayristirma_hatasi
        .into_inner()
        .unwrap_or_else(|z| z.into_inner())
    {
        sonuc.koruma_hata_kaydet(KorumaTaramaHatasi::Okunamadi {
            yol: Some(kok.join(hatali)),
            mesaj: "Yürüyüş girdisi çalışma alanı yolu olarak ayrıştırılamadı.".into(),
        });
    }
    sonuc
}

/// Çalışma alanındaki `.git` girdileri ve yapılandırma yolları; yalıtımsız
/// komuttan önce veya MCP başlangıç anlık görüntüsü için.
pub async fn adli_girdiler(alan: Arc<CalismaAlani>) -> Result<AdliGirdiler, AracHatasi> {
    adli_girdiler_secenekli(alan, TaramaSecenekleri::default()).await
}

pub async fn adli_girdiler_secenekli(
    alan: Arc<CalismaAlani>,
    secenekler: TaramaSecenekleri,
) -> Result<AdliGirdiler, AracHatasi> {
    tokio::task::spawn_blocking(move || adli_girdiler_senkron(&alan, &secenekler))
        .await
        .map_err(|e| gecersiz(format!("Git taraması yarıda kaldı: {e}")))?
}

/// `AlanBilgisi` içinden yürütme öncesi `AdliGirdiler` anlık görüntüsünü
/// çıkarır; yalıtıcı meta veri görüntüsünü kullanmadığı için kopyalamadan taşır.
pub fn alan_bilgisinden_adli_girdiler(alan: &mut AlanBilgisi) -> AdliGirdiler {
    AdliGirdiler {
        git: alan.gizli.git.clone(),
        yapilandirma: alan.gizli.yapilandirma.clone(),
        goruntu: std::mem::take(&mut alan.gizli.goruntu),
        koruma_tam: alan.gizli.koruma_tam,
        koruma_hatasi: alan.gizli.koruma_hatasi.clone(),
    }
}

/// Komut ya da MCP çağrısından sonra yeni `.git` girdilerini denetler ve
/// tehlikelileri karantinaya alır (Bulgu 1); yeni yapılandırma dosyalarını
/// bildirir (Bulgu 2). Aynı taramanın görüntüsü değişen dosyaları verir.
pub fn komut_sonrasi_denetim_senkron(
    alan: &CalismaAlani,
    onceki: &AdliGirdiler,
    secenekler: &TaramaSecenekleri,
) -> SonDenetimSonucu {
    let kok = match alan.kok().canonicalize() {
        Ok(k) => k,
        Err(e) => return SonDenetimSonucu::denetlenemedi(e),
    };
    let damga = utc_damgasi(SystemTime::now());
    let mut uyarilar = Vec::new();
    let mut basarisiz = false;
    let simdi = adli_girdiler_kok_ile(&kok, alan.siniflandirici(), secenekler);
    let tarama_tam = onceki.koruma_tam && simdi.koruma_tam;
    let degisiklikler = DosyaDegisiklikleri::olustur(
        tarama::karsilastir(&onceki.goruntu, &simdi.goruntu)
            .into_iter()
            .map(|d| (d.yol, degisiklik_turu(d.tur))),
        onceki.goruntu.eksik || simdi.goruntu.eksik,
    );
    for y in &simdi.yapilandirma {
        let yeni = !onceki
            .yapilandirma
            .iter()
            .any(|o| y.yol == o.yol || (o.dizin && y.yol.starts_with(&o.yol)));
        if yeni {
            let goreli = y.yol.strip_prefix(&kok).unwrap_or(&y.yol);
            uyarilar.push(format!(
                "Komut, kendiliğinden çalışabilecek bir yapılandırma dosyası \
                 oluşturdu: `{}`. İnceleyin.",
                goreli.to_string_lossy().replace('\\', "/")
            ));
        }
    }
    let mut kalan_git = Vec::with_capacity(simdi.git.len());
    for g in &simdi.git {
        if onceki.git.iter().any(|o| o.yol == g.yol) {
            kalan_git.push(g.clone());
            continue;
        }
        let GitDenetimi::Tehlikeli { nedenler } = yeni_git_denetle(&g.yol, g.dizin && !g.baglanti)
        else {
            kalan_git.push(g.clone());
            continue;
        };
        let nedenler = nedenler.join("; ");
        let goreli = g.yol.strip_prefix(&kok).ok().and_then(goreli_calisma_yolu);
        let Some(yol) = goreli else {
            basarisiz = true;
            kalan_git.push(g.clone());
            uyarilar.push(format!("{GIT_DENETLENEMEDI}: {}", g.yol.display()));
            continue;
        };
        match alan.karantinaya_al(&yol, &damga) {
            Ok(yeni) => uyarilar.push(format!(
                "Komut, git'in kendiliğinden kod çalıştırabileceği bir `.git` \
                 oluşturdu: `{yol}` ({nedenler}). Güvenlik için `{yeni}` olarak \
                 kenara alındı."
            )),
            Err(e) => {
                basarisiz = true;
                kalan_git.push(g.clone());
                uyarilar.push(format!(
                    "Komut, git'in kendiliğinden kod çalıştırabileceği bir `.git` \
                     oluşturdu: `{yol}` ({nedenler}); karantinaya alınamadı ({e}). \
                     Bu klasörde git çalıştırmadan önce `.git`'i inceleyip silin."
                ));
            }
        }
    }
    let mut guncel = simdi;
    guncel.git = kalan_git;
    SonDenetimSonucu {
        uyarilar,
        karantina_basarisiz: basarisiz,
        tarama_tam,
        degisiklikler: Some(degisiklikler),
        guncel_adli_girdiler: Some(guncel),
    }
}

pub async fn komut_sonrasi_denetim(
    alan: Arc<CalismaAlani>,
    onceki: AdliGirdiler,
) -> SonDenetimSonucu {
    komut_sonrasi_denetim_secenekli(alan, onceki, TaramaSecenekleri::default()).await
}

pub async fn komut_sonrasi_denetim_secenekli(
    alan: Arc<CalismaAlani>,
    onceki: AdliGirdiler,
    secenekler: TaramaSecenekleri,
) -> SonDenetimSonucu {
    tokio::task::spawn_blocking(move || komut_sonrasi_denetim_senkron(&alan, &onceki, &secenekler))
        .await
        .unwrap_or_else(SonDenetimSonucu::denetlenemedi)
}

/// Doğrulanmış `AlanBilgisi`, okunur dizinler ve ağ özeti üzerinden kanonik
/// `McpKorumaOzeti` (`K`) hesaplar.
///
/// `alan.gizli.koruma_tam == false` ise özet üretilmez ve `TaramaEksik` döner.
/// Yalnız `alan.gizli.goruntu.eksik == true` olması (500.000 girdi / 5 sn
/// değişiklik görüntüsü sınırı) hata sayılmaz.
pub fn mcp_koruma_ozeti(
    alan: &AlanBilgisi,
    okunur_dizinler: &[PathBuf],
    ag_ozeti: Option<&str>,
) -> Result<McpKorumaOzeti, KorumaDogrulamaHatasi> {
    let Some(baytlar) = alan.koruma_ozeti_baytlari(okunur_dizinler, ag_ozeti) else {
        let hata =
            alan.gizli
                .koruma_hatasi
                .clone()
                .unwrap_or_else(|| KorumaTaramaHatasi::Okunamadi {
                    yol: Some(alan.kok.clone()),
                    mesaj: "Koruma taraması eksik.".into(),
                });
        return Err(KorumaDogrulamaHatasi::TaramaEksik(hata));
    };
    Ok(McpKorumaOzeti::ozetten(baytlar))
}

/// Çalışma alanını tarar, tamlığını doğrular, gerekiyorsa yalıtıcı hazırlığını
/// (`alani_hazirla`) çalıştırır ve beklenen `K` özetiyle karşılaştırır.
pub fn koruma_dogrula_senkron(
    alan: &CalismaAlani,
    yalitici: Option<&dyn Yalitici>,
    hazirlik_onaylandi: bool,
    okunur_dizinler: &[PathBuf],
    ag_ozeti: Option<&str>,
    beklenen: Option<McpKorumaOzeti>,
    secenekler: &TaramaSecenekleri,
) -> KorumaSonucu {
    let mut alan_bilgisi = match alan_bilgisi_secenekli_senkron(alan, secenekler) {
        Ok(b) => b,
        Err(e) => {
            return KorumaSonucu::Dogrulanamadi(KorumaDogrulamaHatasi::AlanCozulemedi(
                e.to_string(),
            ));
        }
    };
    let ozet = match mcp_koruma_ozeti(&alan_bilgisi, okunur_dizinler, ag_ozeti) {
        Ok(o) => o,
        Err(hata) => return KorumaSonucu::Dogrulanamadi(hata),
    };
    if let Some(y) = yalitici
        && let Err(e) = y.alani_hazirla(&alan_bilgisi, hazirlik_onaylandi, &mut |_| {})
    {
        return KorumaSonucu::Dogrulanamadi(KorumaDogrulamaHatasi::HazirlikBasarisiz(
            e.to_string(),
        ));
    }
    let adli_girdiler = alan_bilgisinden_adli_girdiler(&mut alan_bilgisi);
    let dogrulanmis = DogrulanmisKoruma {
        alan_bilgisi,
        adli_girdiler,
        ozet,
    };
    if beklenen == Some(ozet) {
        KorumaSonucu::Ayni(dogrulanmis)
    } else {
        KorumaSonucu::Degisti(dogrulanmis)
    }
}

pub async fn koruma_dogrula(
    alan: Arc<CalismaAlani>,
    yalitici: Option<Arc<dyn Yalitici>>,
    hazirlik_onaylandi: bool,
    okunur_dizinler: Vec<PathBuf>,
    ag_ozeti: Option<String>,
    beklenen: Option<McpKorumaOzeti>,
    secenekler: TaramaSecenekleri,
) -> KorumaSonucu {
    tokio::task::spawn_blocking(move || {
        koruma_dogrula_senkron(
            &alan,
            yalitici.as_deref(),
            hazirlik_onaylandi,
            &okunur_dizinler,
            ag_ozeti.as_deref(),
            beklenen,
            &secenekler,
        )
    })
    .await
    .unwrap_or_else(|e| {
        KorumaSonucu::Dogrulanamadi(KorumaDogrulamaHatasi::AlanCozulemedi(format!(
            "Koruma taraması yarıda kaldı: {e}"
        )))
    })
}

fn degisiklik_turu(tur: tarama::DegisiklikTuru) -> DegisiklikTuru {
    match tur {
        tarama::DegisiklikTuru::Eklendi => DegisiklikTuru::Eklendi,
        tarama::DegisiklikTuru::Degisti => DegisiklikTuru::Degisti,
        tarama::DegisiklikTuru::Silindi => DegisiklikTuru::Silindi,
    }
}

/// Tarama için git'in dizin adı kuralı (çekirdekteki `git_bileseni_mi`).
pub(crate) fn git_adi_mi(ad: &OsStr) -> bool {
    git_koruma::git_bileseni_mi(&ad.to_string_lossy())
}

/// Göreli işletim sistemi yolunu yalnızca geçerli UTF-8 ve geçerli
/// `CalismaYolu` ise çözer; kayıplı dönüşümle sessizce atlamaz.
fn goreli_calisma_yolu(yol: &Path) -> Option<CalismaYolu> {
    let metin = yol.to_str()?;
    CalismaYolu::coz(metin).ok()
}

fn yapilandirma_yolu_mu(s: &Siniflandirici, y: &CalismaYolu, dizin: bool) -> bool {
    if dizin {
        s.yapilandirma_dizini_mi(y)
    } else {
        s.yapilandirma_mi(y)
    }
}

/// Karantina adındaki UTC zaman damgası: `yyyymmddThhmmss`.
pub fn utc_damgasi(an: SystemTime) -> String {
    let sn = an
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |s| s.as_secs());
    let (gun, gun_ici) = (sn / 86_400, sn % 86_400);
    // Howard Hinnant'ın `civil_from_days` algoritması.
    let z = gun as i64 + 719_468;
    let cag = z.div_euclid(146_097);
    let gun_cag = z.rem_euclid(146_097);
    let yil_cag = (gun_cag - gun_cag / 1460 + gun_cag / 36_524 - gun_cag / 146_096) / 365;
    let yil_gunu = gun_cag - (365 * yil_cag + yil_cag / 4 - yil_cag / 100);
    let ay_p = (5 * yil_gunu + 2) / 153;
    let gun_ay = yil_gunu - (153 * ay_p + 2) / 5 + 1;
    let ay = if ay_p < 10 { ay_p + 3 } else { ay_p - 9 };
    let yil = yil_cag + cag * 400 + i64::from(ay <= 2);
    format!(
        "{yil:04}{ay:02}{gun_ay:02}T{:02}{:02}{:02}",
        gun_ici / 3600,
        gun_ici % 3600 / 60,
        gun_ici % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use kalfa_sandbox::tarama::{GORUNTU_AZAMI_SURE, GoruntuSiniri, KORUMA_AZAMI_SURE};
    use kalfa_sandbox::{BaslatmaHatasi, CalistirmaIstegi, Surec, YalitimDurumu};
    use std::time::{Duration, UNIX_EPOCH};

    struct SahteHazirlikYaliticisi {
        hata: Option<String>,
    }

    impl Yalitici for SahteHazirlikYaliticisi {
        fn durum(&self) -> YalitimDurumu {
            YalitimDurumu::Hazir {
                arka_uc: "sahte".into(),
                surum: "1".into(),
            }
        }

        fn baslat(&self, _istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi> {
            Err(BaslatmaHatasi::Yalitim("kullanilmaz".into()))
        }

        fn alani_hazirla(
            &self,
            _alan: &AlanBilgisi,
            _hazirlik_onaylandi: bool,
            _ilerleme: &mut dyn FnMut(&str),
        ) -> Result<(), BaslatmaHatasi> {
            if let Some(m) = &self.hata {
                Err(BaslatmaHatasi::Yalitim(m.clone()))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn ayni_k_gibi_gorunen_kismi_tarama_reddedilir_goruntu_eksik_koruma_tam_kabul_edilir() {
        let d = tempfile::tempdir().unwrap();
        let k = d.path();
        std::fs::create_dir_all(k.join("src")).unwrap();
        for i in 0..5 {
            std::fs::write(k.join(format!("src/f{i}.rs")), "fn main() {}").unwrap();
        }
        std::fs::write(k.join(".env"), "SIR=1").unwrap();

        let alan = CalismaAlani::ac(Arc::new(Siniflandirici::yeni(k, &[]).unwrap())).unwrap();

        // 1. Tam tarama K üretir.
        let ilk = match koruma_dogrula_senkron(
            &alan,
            None,
            false,
            &[],
            Some("ag-1"),
            None,
            &TaramaSecenekleri::yeni(),
        ) {
            KorumaSonucu::Degisti(d) => d,
            diger => panic!("beklenen Degisti, gelen: {diger:?}"),
        };

        // 2. Değişiklik görüntüsü sınırı (goruntu.eksik == true) dolsa bile koruma tamdır ve Ayni döner.
        let dar_goruntu = TaramaSecenekleri::yeni().goruntu_siniri_ile(GoruntuSiniri {
            girdi: 1,
            sure: GORUNTU_AZAMI_SURE,
        });
        let ayni = koruma_dogrula_senkron(
            &alan,
            None,
            false,
            &[],
            Some("ag-1"),
            Some(ilk.ozet),
            &dar_goruntu,
        );
        match ayni {
            KorumaSonucu::Ayni(d) => {
                assert!(d.adli_girdiler.goruntu.eksik);
                assert!(d.alan_bilgisi.gizli.koruma_tam);
                assert_eq!(d.ozet, ilk.ozet);
            }
            diger => panic!("beklenen Ayni, gelen: {diger:?}"),
        }

        // 3. Gizli olmayan bir dosyada yürüyüş hatası oluştuğunda gizli dosya kümesi aynı kalsa bile
        //    kısmi tarama K üretmez ve Dogrulanamadi döner.
        let kismi = TaramaSecenekleri::yeni().yuruyus_hata_enjeksiyonu_ile(|yol| {
            yol.file_name().and_then(|a| a.to_str()) == Some("f0.rs")
        });
        assert!(matches!(
            koruma_dogrula_senkron(
                &alan,
                None,
                false,
                &[],
                Some("ag-1"),
                Some(ilk.ozet),
                &kismi,
            ),
            KorumaSonucu::Dogrulanamadi(KorumaDogrulamaHatasi::TaramaEksik(
                KorumaTaramaHatasi::Okunamadi { .. }
            ))
        ));

        // 4. 30 sn koruma zaman aşımı Dogrulanamadi döner.
        let zaman_asimi = TaramaSecenekleri::yeni()
            .sahte_gecen_sure_ile(|| KORUMA_AZAMI_SURE + Duration::from_secs(1));
        assert!(matches!(
            koruma_dogrula_senkron(
                &alan,
                None,
                false,
                &[],
                Some("ag-1"),
                Some(ilk.ozet),
                &zaman_asimi,
            ),
            KorumaSonucu::Dogrulanamadi(KorumaDogrulamaHatasi::TaramaEksik(
                KorumaTaramaHatasi::ZamanAsimi
            ))
        ));

        // 5. Windows/yalıtıcı alan hazırlığı hatası Dogrulanamadi döner.
        let hatali_yalitici = SahteHazirlikYaliticisi {
            hata: Some("ACL yazilamadi".into()),
        };
        assert!(matches!(
            koruma_dogrula_senkron(
                &alan,
                Some(&hatali_yalitici),
                true,
                &[],
                Some("ag-1"),
                Some(ilk.ozet),
                &TaramaSecenekleri::yeni(),
            ),
            KorumaSonucu::Dogrulanamadi(KorumaDogrulamaHatasi::HazirlikBasarisiz(_))
        ));

        // 6. Yeni gizli dosya, yeni .git veya yeni yapılandırma eklendiğinde K değişir (Degisti).
        std::fs::write(k.join(".env.local"), "SIR=2").unwrap();
        let degisti_gizli = match koruma_dogrula_senkron(
            &alan,
            None,
            false,
            &[],
            Some("ag-1"),
            Some(ilk.ozet),
            &TaramaSecenekleri::yeni(),
        ) {
            KorumaSonucu::Degisti(d) => d,
            diger => panic!("beklenen Degisti, gelen: {diger:?}"),
        };
        assert_ne!(degisti_gizli.ozet, ilk.ozet);

        std::fs::create_dir_all(k.join(".vscode")).unwrap();
        std::fs::write(k.join(".vscode/tasks.json"), "{}").unwrap();
        let degisti_yapilandirma = match koruma_dogrula_senkron(
            &alan,
            None,
            false,
            &[],
            Some("ag-1"),
            Some(degisti_gizli.ozet),
            &TaramaSecenekleri::yeni(),
        ) {
            KorumaSonucu::Degisti(d) => d,
            diger => panic!("beklenen Degisti, gelen: {diger:?}"),
        };
        assert_ne!(degisti_yapilandirma.ozet, degisti_gizli.ozet);
    }

    #[test]
    fn son_denetim_tehlikeli_git_ve_yeni_yapilandirmayi_kabuk_gibi_raporlar() {
        let d = tempfile::tempdir().unwrap();
        let k = d.path();
        std::fs::write(k.join("mevcut.txt"), "1").unwrap();
        let alan = CalismaAlani::ac(Arc::new(Siniflandirici::yeni(k, &[]).unwrap())).unwrap();

        let onceki = adli_girdiler_senkron(&alan, &TaramaSecenekleri::yeni()).unwrap();
        assert!(onceki.koruma_tam);

        // Tehlikeli `.git/config` (fsmonitor) ve yeni `.vscode/tasks.json` oluştur.
        std::fs::create_dir_all(k.join("alt/.git")).unwrap();
        std::fs::write(k.join("alt/.git/config"), "[core]\n\tfsmonitor = evil\n").unwrap();
        std::fs::create_dir_all(k.join(".vscode")).unwrap();
        std::fs::write(k.join(".vscode/tasks.json"), "{}").unwrap();

        let sonuc = komut_sonrasi_denetim_senkron(&alan, &onceki, &TaramaSecenekleri::yeni());
        assert!(sonuc.mcp_icin_basarili_mi());
        assert!(!k.join("alt/.git").exists());
        assert!(
            sonuc
                .uyarilar
                .iter()
                .any(|u| u.contains("yapılandırma dosyası oluşturdu: `.vscode`"))
        );
        assert!(
            sonuc
                .uyarilar
                .iter()
                .any(|u| u.contains("alt/.git") && u.contains("kenara alındı"))
        );

        // Son denetim sırasında tarama eksik kalırsa MCP için başarılı sayılmaz.
        let eksik_secenek = TaramaSecenekleri::yeni().yuruyus_hata_enjeksiyonu_ile(|yol| {
            yol.file_name().and_then(|a| a.to_str()) == Some("mevcut.txt")
        });
        let eksik_sonuc = komut_sonrasi_denetim_senkron(&alan, &onceki, &eksik_secenek);
        assert!(!eksik_sonuc.tarama_tam);
        assert!(!eksik_sonuc.mcp_icin_basarili_mi());
    }

    #[test]
    fn utc_damgasi_bicimi() {
        assert_eq!(utc_damgasi(UNIX_EPOCH), "19700101T000000");
        let an = UNIX_EPOCH + Duration::from_secs(1_790_603_107);
        assert_eq!(utc_damgasi(an), "20260928T134507");
        let an = UNIX_EPOCH + Duration::from_secs(1_709_251_199);
        assert_eq!(utc_damgasi(an), "20240229T235959");
    }
}
