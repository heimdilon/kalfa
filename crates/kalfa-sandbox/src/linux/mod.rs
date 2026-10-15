//! Linux arka ucu: `bubblewrap` + `seccomp` (aşama 2 tasarımı §8).
//!
//! `LinuxYalitici` açılışta yalıtımı algılar ve durumu önbelleğe alır. Yalıtım
//! hazırsa komut bwrap'ın içinde çalışır; değilse yalıtımsız çalışır ve durum
//! bunu söyler (kabuk her çağrıda yerel onay ister). İstek hangi kipte
//! onaylandıysa o kipte çalışır: onaydan sonra durum değiştiyse komut hiç
//! başlamaz.

mod bwrap;
pub mod seccomp;

use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Seek, SeekFrom, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::profil::{self, AlanBilgisi, GitKimligi, Korumali, OzelDizinler, alan_kimligi};
use crate::{
    AZAMI_SUREC, BaslatmaHatasi, CalistirmaIstegi, KomutBicimi, Surec, Yalitici, YalitimDurumu,
    YalitimYoklukNedeni, program,
};
use bwrap::{Kurulum, Sistem};

/// bwrap yalnızca sistem yollarından kabul edilir; `PATH`'ten aranmaz (§8.1).
/// Önce .deb paketinin kendi kopyası: AppArmor profili yalnız ona ad alanı
/// izni verir (Linux paket tasarımı).
const BWRAP_YOLLARI: &[&str] = &[
    "/usr/lib/kalfa/bwrap",
    "/usr/bin/bwrap",
    "/usr/local/bin/bwrap",
];
/// Süreç sayısı sınırını yalıtımın içinde koyar (§6.3): sınır bwrap'a
/// dışarıda verilseydi kullanıcının bütün süreçleri sayılırdı.
const PRLIMIT: &str = "/usr/bin/prlimit";
const SISTEM: &[&str] = &[
    "/usr", "/etc", "/bin", "/sbin", "/lib", "/lib32", "/lib64", "/libx32",
];
pub const ARKA_UC: &str = "bwrap";

// Talimatlar arayüzde gösterilir; arayüzün diliyle ("siz") yazılır.
pub const TALIMAT_BWRAP_YOK: &str = "Komutları yalıtmak için `bubblewrap` paketini kurun \
(`sudo apt install bubblewrap` ya da dağıtımınızın paket yöneticisiyle), sonra Ayarlar'da \
\"Yeniden dene\"yi seçin. O zamana kadar Kalfa komut çalıştırmaz.";
pub const TALIMAT_AD_ALANI: &str = "Sistem, ayrıcalıksız kullanıcı ad alanlarını kısıtlıyor \
(Ubuntu 24.04 ve sonrasında AppArmor). Kalfa'nın .deb paketi bunu yalnız kendi bwrap \
kopyası için kurduğu bir AppArmor profiliyle çözer. Kalfa'yı .deb paketiyle kurduysanız \
profili `sudo dpkg-reconfigure kalfa` ile yeniden kurun; başka yolla kurduysanız .deb \
paketini kullanın. Sonra Ayarlar'da \"Yeniden dene\"yi seçin. Sistem genelindeki kısıtı \
kapatmanızı ya da /usr/bin/bwrap'a sınırsız bir profil eklemenizi önermiyoruz: bütün \
sistemi etkiler. O zamana kadar Kalfa komut çalıştırmaz.";

/// BT için gereksinim paragrafı (aşama 2 tasarımı §9.8).
pub const GEREKSINIM: &str = "Kalfa, yapay zekânın önerdiği komutları kullanıcının kendi \
hesabıyla, bubblewrap (bwrap) ve seccomp ile yalıtır: ayrıcalıksız kullanıcı ad alanlarında, \
çalışma alanı dışı salt okunur ya da görünmez, ağ varsayılan olarak kapalı. Gerekenler: \
/usr/lib/kalfa/bwrap (.deb paketinin kopyası), /usr/bin/bwrap ya da /usr/local/bin/bwrap \
(bubblewrap paketi) ve ayrıcalıksız kullanıcı ad alanları. Ubuntu 24.04 ve sonrasında \
AppArmor bunları kısıtlar (kernel.apparmor_restrict_unprivileged_userns); .deb paketi \
yalnız kendi kopyası için, başlattığı süreçleri yeteneksiz bırakan bir AppArmor profili \
(kalfa-bwrap) kurar. Kalfa açılışta kısa bir bwrap denemesiyle yalıtımı doğrular.";

/// Dağıtımın adı ve çekirdek sürümü (§9.8).
pub fn sistem_surumu() -> String {
    let dagitim = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|m| {
            m.lines()
                .find_map(|s| s.strip_prefix("PRETTY_NAME="))
                .map(|d| d.trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "Linux".into());
    let cekirdek = std::fs::read_to_string("/proc/sys/kernel/osrelease").unwrap_or_default();
    format!("{dagitim} (çekirdek {})", cekirdek.trim())
}

/// Linux arka ucunun ayarları.
#[derive(Clone, Default)]
pub struct LinuxAyari {
    /// Kullanıcının ev dizini; araç zinciri dizinleri buna göre bulunur.
    pub ev: PathBuf,
    /// Çalışma alanına özel dizinlerin kökü: `<veri dizini>/yalitim` (§7.3).
    pub yalitim_dizini: PathBuf,
    /// Kullanıcının Ayarlar'dan eklediği araç zinciri dizinleri (§7.1).
    pub ek_okunur: Vec<PathBuf>,
    /// Ağ köprüsü ikilisinin yolu (`kalfa-kopru`).
    pub kopru: Option<PathBuf>,
    pub git: GitKimligi,
    pub vekil_cozucu: Option<std::sync::Arc<dyn crate::vekil::Cozucu>>,
    pub vekil_yerel_izin: bool,
}

impl std::fmt::Debug for LinuxAyari {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinuxAyari")
            .field("ev", &self.ev)
            .field("yalitim_dizini", &self.yalitim_dizini)
            .field("ek_okunur", &self.ek_okunur)
            .field("kopru", &self.kopru)
            .field("git", &self.git)
            .field("vekil_yerel_izin", &self.vekil_yerel_izin)
            .finish()
    }
}

impl PartialEq for LinuxAyari {
    fn eq(&self, other: &Self) -> bool {
        self.ev == other.ev
            && self.yalitim_dizini == other.yalitim_dizini
            && self.ek_okunur == other.ek_okunur
            && self.kopru == other.kopru
            && self.git == other.git
            && self.vekil_yerel_izin == other.vekil_yerel_izin
    }
}

impl Eq for LinuxAyari {}

impl LinuxAyari {
    /// Yalıtıma salt okunur açılan araç zinciri dizinleri: varsayılanlar ve
    /// kullanıcının ekledikleri; yalnızca var olanlar (§7.1). `~/.rustup`'ın
    /// tamamı açılır: rustup vekili her çalışmada `update-hashes` dizinini
    /// ister; dizinde kimlik bilgisi yoktur.
    fn okunur(&self) -> Vec<PathBuf> {
        let varsayilan = [self.ev.join(".cargo/bin"), self.ev.join(".rustup")];
        let mut yollar: Vec<PathBuf> = varsayilan
            .into_iter()
            .chain(self.ek_okunur.iter().cloned())
            .filter(|y| y.is_absolute() && y.exists())
            .collect();
        yollar.dedup();
        yollar
    }

    fn registry(&self) -> Option<PathBuf> {
        Some(self.ev.join(".cargo/registry")).filter(|y| y.is_dir())
    }

    fn rustup(&self) -> Option<PathBuf> {
        Some(self.ev.join(".rustup")).filter(|y| y.is_dir())
    }
}

/// Yalıtımın kullanılabilirliğini bulan birim (§8.3); testte sahtesi kullanılır.
pub trait Algilayici: Send + Sync {
    fn algila(&self, ayar: &LinuxAyari) -> YalitimDurumu;
}

/// Gerçek algılayıcı: bwrap'ı sistem yollarında arar, seccomp filtresini
/// üretir ve kısa bir deneme çalıştırması yapar.
#[derive(Debug, Default)]
pub struct BwrapAlgilayici;

impl Algilayici for BwrapAlgilayici {
    fn algila(&self, ayar: &LinuxAyari) -> YalitimDurumu {
        let Some(bwrap) = bwrap_yolu() else {
            return yok(
                YalitimYoklukNedeni::BwrapYok,
                TALIMAT_BWRAP_YOK,
                &format!("denetim=bwrap-yolu; aranan={}", BWRAP_YOLLARI.join(", ")),
            );
        };
        if let Err(e) = seccomp::engel_filtresi(false).and(seccomp::enosys_filtresi()) {
            return yok(
                YalitimYoklukNedeni::Desteklenmiyor,
                &format!("{e}. Kalfa komut çalıştırmaz."),
                &format!("denetim=seccomp; {e}"),
            );
        }
        match deneme(bwrap, ayar) {
            Ok(()) => YalitimDurumu::Hazir {
                arka_uc: ARKA_UC.into(),
                surum: surum(bwrap),
            },
            Err(hata) => durum_hatadan(&hata),
        }
    }
}

fn yok(neden: YalitimYoklukNedeni, talimat: &str, ayrinti: &str) -> YalitimDurumu {
    YalitimDurumu::Yok {
        neden,
        talimat: talimat.to_string(),
        ayrinti: ayrinti.to_string(),
    }
}

/// Deneme çalıştırmasının hatasından durumu çıkarır.
fn durum_hatadan(hata: &str) -> YalitimDurumu {
    let kucuk = hata.to_lowercase();
    let ad_alani = [
        "namespace",
        "uid map",
        "rtm_newaddr",
        "operation not permitted",
    ];
    if ad_alani.iter().any(|p| kucuk.contains(p)) {
        return yok(
            YalitimYoklukNedeni::KullaniciAdAlaniKapali,
            TALIMAT_AD_ALANI,
            &format!("denetim=bwrap-deneme; {}", hata.trim()),
        );
    }
    yok(
        YalitimYoklukNedeni::Desteklenmiyor,
        &format!(
            "Yalıtım bu sistemde başlatılamadı: {}. Kalfa komut çalıştırmaz.",
            hata.trim()
        ),
        &format!("denetim=bwrap-deneme; {}", hata.trim()),
    )
}

pub fn bwrap_yolu() -> Option<&'static Path> {
    BWRAP_YOLLARI
        .iter()
        .map(Path::new)
        .find(|y| program::calistirilabilir(y))
}

fn surum(bwrap: &Path) -> String {
    std::process::Command::new(bwrap)
        .arg("--version")
        .output()
        .ok()
        .map(|c| String::from_utf8_lossy(&c.stdout).trim().to_string())
        .and_then(|s| s.rsplit(' ').next().map(str::to_string))
        .unwrap_or_default()
}

/// `/usr/bin/true`'yu gerçek profille, geçici bir çalışma alanında çalıştırır.
fn deneme(bwrap: &Path, ayar: &LinuxAyari) -> Result<(), String> {
    let gecici = tempfile::Builder::new()
        .prefix("kalfa-yalitim-denemesi")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let kok = gecici.path().join("alan");
    std::fs::create_dir_all(&kok).map_err(|e| e.to_string())?;
    let ayar = LinuxAyari {
        yalitim_dizini: gecici.path().join("yalitim"),
        ..ayar.clone()
    };
    let alan = AlanBilgisi {
        kok: kok.clone(),
        gizli: Default::default(),
        korumali: Vec::new(),
        git_dizinleri: Vec::new(),
    };
    let ozel = OzelDizinler::yol(&ayar.yalitim_dizini, &alan_kimligi(&kok));
    ozel.hazirla(&ayar.git).map_err(|e| e.to_string())?;
    let komut = KomutBicimi::Program {
        program: "true".into(),
        argumanlar: Vec::new(),
    };
    let (mut komut, _fdler, _vekil_kanali) = komut_kur(
        bwrap,
        &ayar,
        &KurIstegi {
            komut: &komut,
            dizin: &kok,
            ortam: &[("PATH".into(), "/usr/bin:/bin".into())],
            ag: false,
            alan: &alan,
            ozel: &ozel,
        },
    )
    .map_err(|e| e.to_string())?;
    let cikti = komut.output().map_err(|e| e.to_string())?;
    if cikti.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&cikti.stderr).into_owned())
    }
}

pub struct LinuxYalitici {
    ayar: RwLock<LinuxAyari>,
    algilayici: Box<dyn Algilayici>,
    durum: RwLock<YalitimDurumu>,
    degisince: Box<dyn Fn(&YalitimDurumu) + Send + Sync>,
}

impl LinuxYalitici {
    /// Algılamayı hemen çalıştırır. `degisince`, durum sonradan değişince
    /// (yeniden algılama) çağrılır.
    pub fn yeni(
        ayar: LinuxAyari,
        algilayici: Box<dyn Algilayici>,
        degisince: Box<dyn Fn(&YalitimDurumu) + Send + Sync>,
    ) -> Self {
        let durum = algilayici.algila(&ayar);
        Self {
            ayar: RwLock::new(ayar),
            algilayici,
            durum: RwLock::new(durum),
            degisince,
        }
    }

    fn ayar(&self) -> LinuxAyari {
        self.ayar.read().unwrap_or_else(|z| z.into_inner()).clone()
    }

    fn yalitimli(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi> {
        let bwrap = bwrap_yolu()
            .ok_or_else(|| BaslatmaHatasi::Yalitim("bwrap artık sistem yollarında yok".into()))?;
        let ayar = self.ayar();
        let alan = istek
            .alan
            .as_ref()
            .ok_or_else(|| BaslatmaHatasi::Yalitim("çalışma alanı bilgisi yok".into()))?;
        let ozel = OzelDizinler::yol(&ayar.yalitim_dizini, &alan_kimligi(&alan.kok));
        ozel.hazirla(&ayar.git)?;
        korumalilari_hazirla(&alan.korumali)?;
        let (komut, fdler, vekil_kanali) = komut_kur(
            bwrap,
            &ayar,
            &KurIstegi {
                komut: &istek.komut,
                dizin: &istek.dizin,
                ortam: &istek.ortam,
                ag: istek.ag,
                alan,
                ozel: &ozel,
            },
        )?;
        let mut surec = Surec::komutla(komut.into())?;
        // Bellek dosyaları ve çocuğun kanalı çocuğa geçti; buradaki kopyalar kapanabilir.
        drop(fdler);
        if let Some(kapat) = vekili_bagla(&ayar, vekil_kanali, istek.ag_listesi.clone()) {
            surec.ag_kapat_ekle(kapat);
        }
        Ok(surec)
    }

    /// Sunucuyu yalıtımda başlatır (1.1a §3, §4.1): kabuğun bwrap kurulumu,
    /// ama özel kök, çalışma dizini (`ev`) ve ortam sunucunundur. Kabuğun
    /// özel dizinleri sunucuya bağlanmaz.
    fn sunucuyu_baslat(
        &self,
        istek: &crate::EtkilesimliIstek,
    ) -> Result<crate::EtkilesimliSurec, BaslatmaHatasi> {
        let bwrap = bwrap_yolu()
            .ok_or_else(|| BaslatmaHatasi::Yalitim("bwrap artık sistem yollarında yok".into()))?;
        let ayar = self.ayar();
        let ozel = crate::sunucu_ozel(
            &ayar.yalitim_dizini,
            &alan_kimligi(&istek.alan.kok),
            &istek.sunucu,
        );
        ozel.hazirla(&ayar.git)?;
        korumalilari_hazirla(&istek.alan.korumali)?;
        let ortam = crate::sunucu_ortami(&istek.ortam, &ozel);
        let komut = KomutBicimi::Program {
            program: istek.program.clone(),
            argumanlar: istek.argumanlar.clone(),
        };
        let (komut, fdler, vekil_kanali) = komut_kur(
            bwrap,
            &ayar,
            &KurIstegi {
                komut: &komut,
                dizin: &ozel.ev,
                ortam: &ortam,
                ag: istek.ag_listesi.is_some(),
                alan: &istek.alan,
                ozel: &ozel,
            },
        )?;
        let mut surec = Surec::komutla_etkilesimli(komut.into())?;
        drop(fdler);
        if let Some(kapat) = vekili_bagla(&ayar, vekil_kanali, istek.ag_listesi.clone()) {
            surec.ag_kapat_ekle(kapat);
        }
        Ok(surec)
    }
}

/// Ağlı çalıştırmada vekili başlatır ve köprünün kanalına bağlar (§8.5).
/// Kanal yoksa (ağsız) bir şey yapmaz. Dönen kapatıcı kanalı ve vekili
/// kapatıp ağ özetini verir.
fn vekili_bagla(
    ayar: &LinuxAyari,
    vekil_kanali: Option<OwnedFd>,
    liste: Option<crate::vekil::IzinListesi>,
) -> Option<crate::surec::AgKapatici> {
    let vekil_kanali = vekil_kanali?;
    let ayar_vekil = crate::vekil::VekilAyari {
        liste: liste.unwrap_or_else(|| crate::vekil::IzinListesi::yeni(&[], &[])),
        cozucu: ayar
            .vekil_cozucu
            .clone()
            .unwrap_or_else(|| std::sync::Arc::new(crate::vekil::SistemCozucu)),
        yerel_izin: ayar.vekil_yerel_izin,
        ..Default::default()
    };
    let vekil = std::sync::Arc::new(crate::vekil::Vekil::baslat(ayar_vekil));
    let vekil_icin = vekil.clone();
    // Okuyucu ve kapatıcı aynı tanımlayıcıyı paylaşır; son sahibi
    // bırakınca kapanır. Ham numarayla paylaşılsaydı kapatıcının
    // kapattığı numara yeniden kullanılıp okuyucuya başka bir dosya
    // olarak görünebilirdi.
    let vekil_kanali = std::sync::Arc::new(vekil_kanali);
    let okuyucu_kanali = vekil_kanali.clone();

    tokio::task::spawn_blocking(move || {
        loop {
            let fd = match crate::vekil::kanal::al(okuyucu_kanali.as_raw_fd()) {
                Ok(Some(fd)) => fd,
                Ok(None) => break,
                // Tek tanımlayıcılı olmayan ya da kesik ileti reddedilir;
                // kanal açık kaldıkça sonraki iletiler okunur (§8.5).
                Err(e) if e.kind() == io::ErrorKind::InvalidData => continue,
                Err(_) => break,
            };
            // Bağlı bir TCP soketi olmayan tanımlayıcı burada kapanır.
            if crate::vekil::kanal::dogrula(&fd).is_err() {
                continue;
            }
            let std_stream = std::net::TcpStream::from(fd);
            if std_stream.set_nonblocking(true).is_ok()
                && let Ok(tokio_stream) = tokio::net::TcpStream::from_std(std_stream)
            {
                vekil_icin.baglanti_ver(tokio_stream);
            }
        }
    });

    let ag_kapatici: crate::surec::AgKapatici = Box::new(move || {
        Box::pin(async move {
            // SAFETY: tanımlayıcı `vekil_kanali` yaşadıkça açıktır;
            // `shutdown` okuyucunun `recvmsg`'ini 0 ile döndürür.
            unsafe {
                libc::shutdown(vekil_kanali.as_raw_fd(), libc::SHUT_RDWR);
            }
            drop(vekil_kanali);
            vekil.bitir().await
        })
    });
    Some(ag_kapatici)
}

impl Yalitici for LinuxYalitici {
    fn durum(&self) -> YalitimDurumu {
        self.durum.read().unwrap_or_else(|z| z.into_inner()).clone()
    }

    fn profil_ozeti(&self, ag: bool, liste_ozeti: Option<&str>) -> Option<String> {
        if !self.durum().hazir_mi() {
            return None;
        }
        let ayar = self.ayar();
        let mut okunur: Vec<PathBuf> = sistem_dizinleri()
            .iter()
            .map(|s| s.yol().to_path_buf())
            .collect();
        okunur.extend(ayar.okunur());
        Some(profil::ozet(ARKA_UC, &okunur, ag, liste_ozeti))
    }

    fn baslat(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi> {
        crate::kip_uyusmasi(istek.alan.is_some(), self.durum().hazir_mi())?;
        match &istek.alan {
            Some(_) => self.yalitimli(istek),
            None => Surec::baslat(istek),
        }
    }

    fn baslat_etkilesimli(
        &self,
        istek: &crate::EtkilesimliIstek,
    ) -> Result<crate::EtkilesimliSurec, BaslatmaHatasi> {
        crate::etkilesimli::sunucu_adi_dogrula(&istek.sunucu)?;
        crate::mcp_kip_uyusmasi(istek.kip, self.durum().hazir_mi())?;
        match istek.kip {
            crate::CalistirmaKipi::TamErisim => {
                let ayar = self.ayar();
                crate::surec::tam_erisimle(istek, &ayar.yalitim_dizini, &ayar.git)
            }
            crate::CalistirmaKipi::Yalitimli => self.sunucuyu_baslat(istek),
        }
    }

    fn sunucu_kokunu_kaldir(&self, alan_koku: &Path, sunucu: &str) -> Result<(), String> {
        crate::etkilesimli::sunucu_adi_dogrula(sunucu).map_err(|e| e.to_string())?;
        let kok = crate::sunucu_ozel(
            &self.ayar().yalitim_dizini,
            &alan_kimligi(alan_koku),
            sunucu,
        )
        .kok;
        match std::fs::remove_dir_all(&kok) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn yeniden_algila(&self) -> YalitimDurumu {
        let yeni = self.algilayici.algila(&self.ayar());
        let degisti = {
            let mut durum = self.durum.write().unwrap_or_else(|z| z.into_inner());
            let degisti = *durum != yeni;
            *durum = yeni.clone();
            degisti
        };
        if degisti {
            (self.degisince)(&yeni);
        }
        yeni
    }

    fn okunur_dizinleri_ayarla(&self, dizinler: Vec<PathBuf>) {
        self.ayar
            .write()
            .unwrap_or_else(|z| z.into_inner())
            .ek_okunur = dizinler;
    }
}

fn sistem_dizinleri() -> Vec<Sistem> {
    SISTEM
        .iter()
        .map(Path::new)
        .filter_map(|yol| {
            let bilgi = std::fs::symlink_metadata(yol).ok()?;
            if bilgi.file_type().is_symlink() {
                let hedef = std::fs::read_link(yol).ok()?;
                Some(Sistem::Bag {
                    yol: yol.to_path_buf(),
                    hedef,
                })
            } else {
                bilgi.is_dir().then(|| Sistem::Dizin(yol.to_path_buf()))
            }
        })
        .collect()
}

/// Kullanıcının cargo registry'si ne zaman bağlanır (§7.4)?
///
/// - Ağsız çalıştırmada, çalışma alanının kendi registry'si boşsa: önceden
///   indirilmiş crate'ler salt okunur kullanılır.
/// - Ağlı çalıştırmada bağlanmaz: cargo yeni crate'i salt okunur registry'ye
///   indiremez. Çalışma alanına özel, başta boş registry'ye indirir.
/// - O registry bir kez dolduysa ağsız çalıştırmalar da onu kullanır.
fn registry_secimi(ag: bool, ozel: &OzelDizinler, ayar: &LinuxAyari) -> Option<PathBuf> {
    let kendi = ozel.cargo().join("registry");
    let dolu = std::fs::read_dir(&kendi).is_ok_and(|mut d| d.next().is_some());
    if ag || dolu { None } else { ayar.registry() }
}

/// Eksik korumalı yolu boş olarak oluşturur: bağlanamayan eksik `.git/hooks`
/// dizinine komut kanca yazabilirdi.
fn korumalilari_hazirla(korumali: &[Korumali]) -> io::Result<()> {
    for k in korumali {
        if k.yol.exists() {
            continue;
        }
        if k.dizin {
            std::fs::create_dir_all(&k.yol)?;
        } else {
            if let Some(ust) = k.yol.parent() {
                std::fs::create_dir_all(ust)?;
            }
            std::fs::File::create(&k.yol)?;
        }
    }
    Ok(())
}

/// Yalıtımdaki ortam: izin listesinden gelenler; ev, geçici dizin ve araç
/// yapılandırması çalışma alanına özel dizinleri gösterir (§6.1, §7.4).
fn yalitim_ortami(
    kaynak: &[(String, String)],
    ozel: &OzelDizinler,
    ayar: &LinuxAyari,
) -> Vec<(String, String)> {
    let yol = |y: &Path| y.display().to_string();
    let mut kalfanin = vec![
        ("HOME", yol(&ozel.ev)),
        ("TMPDIR", "/tmp".to_string()),
        ("CARGO_HOME", yol(&ozel.cargo())),
        ("GIT_CONFIG_GLOBAL", yol(&ozel.gitconfig())),
        ("NPM_CONFIG_USERCONFIG", yol(&ozel.npmrc())),
        ("NPM_CONFIG_CACHE", yol(&ozel.npm())),
        ("PIP_CACHE_DIR", yol(&ozel.pip())),
    ];
    if let Some(rustup) = ayar.rustup() {
        kalfanin.push(("RUSTUP_HOME", yol(&rustup)));
    }
    let mut ortam: Vec<(String, String)> = kaynak
        .iter()
        .filter(|(ad, _)| !kalfanin.iter().any(|(k, _)| k == ad))
        .filter(|(ad, _)| !matches!(ad.as_str(), "TEMP" | "TMP"))
        .cloned()
        .collect();
    ortam.extend(kalfanin.into_iter().map(|(a, d)| (a.to_string(), d)));
    ortam
}

/// Yol yalıtımın içinde görünür mü? Yollar içeride de aynı konumdadır.
fn gorunur_mu(yol: &Path, kokler: &[PathBuf], alan: &AlanBilgisi) -> bool {
    kokler.iter().any(|k| yol.starts_with(k))
        && !alan.gizli.dosyalar.iter().any(|g| g == yol)
        && !alan.gizli.dizinler.iter().any(|g| yol.starts_with(g))
}

/// Programı yalıtımın içindeki `PATH`'te, yalnızca içeride görünen yollarda
/// arar (§4.1). Çalışma alanındaki bir program mutlak yolla gelir.
fn programi_bul(
    program: &str,
    ortam: &[(String, String)],
    kokler: &[PathBuf],
    alan: &AlanBilgisi,
) -> Result<PathBuf, BaslatmaHatasi> {
    let path = crate::ortam::deger(ortam, "PATH").unwrap_or("");
    let gorunen: Vec<PathBuf> = std::env::split_paths(path)
        .filter(|d| d.is_absolute() && gorunur_mu(d, kokler, alan))
        .collect();
    let gorunen_path = std::env::join_paths(&gorunen).unwrap_or_default();
    let yol = program::bul(
        program,
        &[("PATH".into(), gorunen_path.to_string_lossy().into_owned())],
    )
    .map_err(|_| BaslatmaHatasi::ProgramYok {
        program: program.to_string(),
        path: gorunen_path.to_string_lossy().into_owned(),
    })?;
    if gorunur_mu(&yol, kokler, alan) {
        Ok(yol)
    } else {
        Err(BaslatmaHatasi::ProgramYok {
            program: program.to_string(),
            path: gorunen_path.to_string_lossy().into_owned(),
        })
    }
}

/// Kapanınca serbest kalan, adsız bir bellek dosyası: bwrap seçenekleri ve
/// seccomp programları bununla geçer.
fn bellek_dosyasi(veri: &[u8]) -> io::Result<OwnedFd> {
    // SAFETY: `memfd_create` sabit, NUL ile biten bir ad ve bayrak alır;
    // dönen tanımlayıcı başarılıysa yalnızca burada sahiplenilir.
    let fd = unsafe { libc::memfd_create(c"kalfa".as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `fd` az önce açıldı ve başka bir sahibi yok.
    let mut dosya = File::from(unsafe { OwnedFd::from_raw_fd(fd) });
    dosya.write_all(veri)?;
    dosya.seek(SeekFrom::Start(0))?;
    Ok(OwnedFd::from(dosya))
}

/// `komut_kur`'un girdisi. `ozel`, kabukta çalışma alanının, etkileşimli
/// süreçte sunucunun özel köküdür (1.1a §3); hazırlanmış olmalıdır.
struct KurIstegi<'a> {
    komut: &'a KomutBicimi,
    dizin: &'a Path,
    ortam: &'a [(String, String)],
    ag: bool,
    alan: &'a AlanBilgisi,
    ozel: &'a OzelDizinler,
}

/// bwrap komutunu kurar. Dönen tanımlayıcılar komut başlayana kadar açık
/// kalmalıdır.
fn komut_kur(
    bwrap: &Path,
    ayar: &LinuxAyari,
    istek: &KurIstegi,
) -> Result<(std::process::Command, Vec<OwnedFd>, Option<OwnedFd>), BaslatmaHatasi> {
    let alan = istek.alan;
    let ozel = istek.ozel;
    let sistem = sistem_dizinleri();
    let okunur = ayar.okunur();
    let registry = registry_secimi(istek.ag, ozel, ayar);
    let ortam = yalitim_ortami(istek.ortam, ozel, ayar);

    let (vekil_kanali, cocuk_kanali) = if istek.ag {
        let (v, c) = crate::vekil::kanal::kanal_cifti()
            .map_err(|e| BaslatmaHatasi::Yalitim(format!("Vekil kanalı açılamadı: {e}")))?;
        (Some(v), Some(c))
    } else {
        (None, None)
    };

    let kopru = if istek.ag {
        let k = ayar.kopru.as_deref().ok_or_else(|| {
            BaslatmaHatasi::Yalitim(
                "Ağ köprüsü ikilisi (kalfa-kopru) ayarlanmamış; ağlı yalıtımlı komut başlatılamaz"
                    .into(),
            )
        })?;
        if !program::calistirilabilir(k) {
            return Err(BaslatmaHatasi::Yalitim(format!(
                "Ağ köprüsü ikilisi bulunamadı ya da çalıştırılabilir değil: {}",
                k.display()
            )));
        }
        Some(k)
    } else {
        None
    };

    let mut kokler: Vec<PathBuf> = sistem.iter().map(|s| s.yol().to_path_buf()).collect();
    kokler.extend(okunur.iter().cloned());
    kokler.push(alan.kok.clone());
    kokler.push(ozel.kok.clone());
    kokler.extend(alan.git_dizinleri.iter().cloned());
    let (program, argumanlar): (PathBuf, Vec<String>) = match istek.komut {
        KomutBicimi::Program {
            program,
            argumanlar,
        } => (
            programi_bul(program, &ortam, &kokler, alan)?,
            argumanlar.clone(),
        ),
        KomutBicimi::Betik { metin } => ("/bin/sh".into(), vec!["-c".into(), metin.clone()]),
    };

    let hata = |e: seccomp::FiltreHatasi| BaslatmaHatasi::Yalitim(e.to_string());
    let engel = bellek_dosyasi(&seccomp::baytlar(
        &seccomp::engel_filtresi(istek.ag).map_err(hata)?,
    ))?;
    let enosys = bellek_dosyasi(&seccomp::baytlar(
        &seccomp::enosys_filtresi().map_err(hata)?,
    ))?;
    let secenekler = bwrap::secenekler(&Kurulum {
        sistem: &sistem,
        okunur: &okunur,
        registry: registry.as_deref(),
        alan,
        ozel,
        dizin: istek.dizin,
        ortam: &ortam,
        ag: istek.ag,
        kopru,
        seccomp: &[engel.as_raw_fd(), enosys.as_raw_fd()],
    });
    let args = bellek_dosyasi(&bwrap::nul_ile(&secenekler))?;

    let mut komut = std::process::Command::new(bwrap);
    komut
        .arg("--args")
        .arg(args.as_raw_fd().to_string())
        .arg("--");
    if let Some(cocuk) = &cocuk_kanali {
        komut
            .arg("/run/kalfa-kopru")
            .arg("--kanal")
            .arg(cocuk.as_raw_fd().to_string())
            .arg("--");
    }
    if program::calistirilabilir(Path::new(PRLIMIT)) {
        komut
            .arg(PRLIMIT)
            .arg(format!("--nproc={AZAMI_SUREC}:{AZAMI_SUREC}"))
            .arg("--");
    }
    komut
        .arg(&program)
        .args(argumanlar.iter().map(OsString::from))
        .env_clear()
        .current_dir("/");

    let mut fdler = vec![args, engel, enosys];
    let mut devredilen = vec![
        fdler[0].as_raw_fd(),
        fdler[1].as_raw_fd(),
        fdler[2].as_raw_fd(),
    ];
    if let Some(ck) = cocuk_kanali {
        devredilen.push(ck.as_raw_fd());
        fdler.push(ck);
    }

    // SAFETY: Kapanış `fork` ile `exec` arasında çocukta çalışır ve yalnızca
    // async-signal-safe sistem çağrılarını kullanır (`close_range`, `getrlimit`, `fcntl`).
    unsafe {
        komut.pre_exec(move || {
            // Önce 3'ten yukarıdaki tüm tanımlayıcılara CLOEXEC bayrağı koy.
            let ret = libc::syscall(
                libc::SYS_close_range,
                3u32,
                !0u32,
                libc::CLOSE_RANGE_CLOEXEC,
            );
            if ret != 0 {
                let err = io::Error::last_os_error();
                if err.raw_os_error() == Some(libc::ENOSYS) {
                    let mut rlim: libc::rlimit = std::mem::zeroed();
                    let azami = if libc::getrlimit(libc::RLIMIT_NOFILE, &mut rlim) == 0 {
                        rlim.rlim_cur as i32
                    } else {
                        1024
                    };
                    for fd in 3..azami {
                        let bayrak = libc::fcntl(fd, libc::F_GETFD);
                        if bayrak >= 0 {
                            let _ = libc::fcntl(fd, libc::F_SETFD, bayrak | libc::FD_CLOEXEC);
                        }
                    }
                } else {
                    return Err(err);
                }
            }

            // Yalnızca devredilen tanımlayıcıların CLOEXEC'ini kaldır.
            for fd in &devredilen {
                if libc::fcntl(*fd, libc::F_SETFD, 0) == -1 {
                    return Err(io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    Ok((komut, fdler, vekil_kanali))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Sahte(RwLock<YalitimDurumu>);

    impl Algilayici for Arc<Sahte> {
        fn algila(&self, _: &LinuxAyari) -> YalitimDurumu {
            self.0.read().unwrap().clone()
        }
    }

    fn hazir() -> YalitimDurumu {
        YalitimDurumu::Hazir {
            arka_uc: ARKA_UC.into(),
            surum: "0.9.0".into(),
        }
    }

    /// Paketin kopyası önce denenir; talimat sistem bwrap'ına sınırsız profil
    /// önermez (Linux paket tasarımı §5).
    #[test]
    fn paket_bwrap_kopyasi_once_ve_talimat_sinirsiz_profil_onermez() {
        assert_eq!(BWRAP_YOLLARI.first(), Some(&"/usr/lib/kalfa/bwrap"));
        assert!(TALIMAT_AD_ALANI.contains("dpkg-reconfigure kalfa"));
        assert!(!TALIMAT_AD_ALANI.contains("unconfined"));
    }

    #[test]
    fn algilama_hatalari_nedenlere_esler() {
        for (hata, neden) in [
            (
                "bwrap: setting up uid map: Permission denied",
                YalitimYoklukNedeni::KullaniciAdAlaniKapali,
            ),
            (
                "bwrap: No permissions to create new namespace, likely because the kernel does not allow non-privileged user namespaces.",
                YalitimYoklukNedeni::KullaniciAdAlaniKapali,
            ),
            (
                "bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted",
                YalitimYoklukNedeni::KullaniciAdAlaniKapali,
            ),
            (
                "bwrap: seccomp: Invalid argument",
                YalitimYoklukNedeni::Desteklenmiyor,
            ),
        ] {
            match durum_hatadan(hata) {
                YalitimDurumu::Yok {
                    neden: n,
                    talimat,
                    ayrinti,
                } => {
                    assert_eq!(n, neden, "{hata}");
                    assert!(!talimat.is_empty());
                    // BT için ayrıntı düşen denetimi ve hatanın kendisini taşır (§9.8).
                    assert_eq!(ayrinti, format!("denetim=bwrap-deneme; {hata}"));
                }
                d => panic!("{hata}: {d:?}"),
            }
        }
    }

    #[test]
    fn durum_degisince_bildirilir_ve_istek_onaylandigi_kipte_calisir() {
        let sahte = Arc::new(Sahte(RwLock::new(yok(
            YalitimYoklukNedeni::BwrapYok,
            TALIMAT_BWRAP_YOK,
            "denetim=bwrap-yolu",
        ))));
        let bildirim = Arc::new(AtomicUsize::new(0));
        let b = bildirim.clone();
        let y = LinuxYalitici::yeni(
            LinuxAyari::default(),
            Box::new(sahte.clone()),
            Box::new(move |_| {
                b.fetch_add(1, Ordering::SeqCst);
            }),
        );
        assert!(!y.durum().hazir_mi());
        assert_eq!(y.profil_ozeti(false, None), None);
        let alanli = CalistirmaIstegi {
            komut: KomutBicimi::Betik {
                metin: "true".into(),
            },
            dizin: std::env::temp_dir(),
            ortam: Vec::new(),
            ag: false,
            ag_listesi: None,
            alan: Some(AlanBilgisi {
                kok: std::env::temp_dir(),
                gizli: Default::default(),
                korumali: Vec::new(),
                git_dizinleri: Vec::new(),
            }),
        };
        // Yalıtımlı onaylanan komut, yalıtım yokken başlamaz.
        assert!(matches!(y.baslat(&alanli), Err(BaslatmaHatasi::Yalitim(_))));

        // Aynı durum yeniden algılanınca bildirim gitmez.
        y.yeniden_algila();
        assert_eq!(bildirim.load(Ordering::SeqCst), 0);
        *sahte.0.write().unwrap() = hazir();
        assert_eq!(y.yeniden_algila(), hazir());
        assert_eq!(bildirim.load(Ordering::SeqCst), 1);
        assert!(y.profil_ozeti(false, None).is_some());
        assert_ne!(y.profil_ozeti(false, None), y.profil_ozeti(true, None));

        // Yalıtımsız onaylanan komut, yalıtım gelince başlamaz.
        let alansiz = CalistirmaIstegi {
            alan: None,
            ..alanli
        };
        assert!(matches!(
            y.baslat(&alansiz),
            Err(BaslatmaHatasi::Yalitim(_))
        ));
    }

    #[test]
    fn eklenen_dizin_profil_ozetini_degistirir() {
        let ev = tempfile::tempdir().unwrap();
        let ek = tempfile::tempdir().unwrap();
        let y = LinuxYalitici::yeni(
            LinuxAyari {
                ev: ev.path().into(),
                ..Default::default()
            },
            Box::new(Arc::new(Sahte(RwLock::new(hazir())))),
            Box::new(|_| {}),
        );
        let once = y.profil_ozeti(false, None);
        y.okunur_dizinleri_ayarla(vec![ek.path().into()]);
        assert_ne!(y.profil_ozeti(false, None), once);
        // Olmayan dizin özete girmez.
        y.okunur_dizinleri_ayarla(vec![ek.path().join("yok")]);
        assert_eq!(y.profil_ozeti(false, None), once);
    }

    #[test]
    fn ortamda_ev_ve_arac_yapilandirmasi_ozel_dizinleri_gosterir() {
        let ev = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(ev.path().join(".rustup")).unwrap();
        let ayar = LinuxAyari {
            ev: ev.path().into(),
            ..Default::default()
        };
        let ozel = OzelDizinler::yol(Path::new("/veri/yalitim"), "abc");
        let ortam = yalitim_ortami(
            &[
                ("PATH".into(), "/usr/bin".into()),
                ("HOME".into(), "/home/ada".into()),
                ("CARGO_HOME".into(), "/home/ada/.cargo".into()),
                ("TMP".into(), "/home/ada/tmp".into()),
            ],
            &ozel,
            &ayar,
        );
        let deger = |ad: &str| crate::ortam::deger(&ortam, ad).map(str::to_string);
        assert_eq!(deger("PATH").as_deref(), Some("/usr/bin"));
        assert_eq!(deger("HOME").as_deref(), Some("/veri/yalitim/abc/ev"));
        assert_eq!(deger("TMPDIR").as_deref(), Some("/tmp"));
        assert_eq!(deger("TMP"), None);
        assert_eq!(
            deger("CARGO_HOME").as_deref(),
            Some("/veri/yalitim/abc/onbellek/cargo")
        );
        assert_eq!(
            deger("RUSTUP_HOME"),
            Some(ev.path().join(".rustup").display().to_string())
        );
        assert_eq!(ortam.iter().filter(|(a, _)| a == "HOME").count(), 1);
    }

    #[test]
    fn registry_yalnizca_agsiz_ve_kendi_registry_bosken_baglanir() {
        let ev = tempfile::tempdir().unwrap();
        let veri = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(ev.path().join(".cargo/registry/cache")).unwrap();
        let ayar = LinuxAyari {
            ev: ev.path().into(),
            ..Default::default()
        };
        let ozel = OzelDizinler::yol(veri.path(), "abc");
        ozel.hazirla(&GitKimligi::default()).unwrap();
        let kullanicinin = Some(ev.path().join(".cargo/registry"));
        assert_eq!(registry_secimi(false, &ozel, &ayar), kullanicinin);
        assert_eq!(registry_secimi(true, &ozel, &ayar), None);
        // Ağlı çalıştırma çalışma alanının registry'sini doldurdu.
        std::fs::create_dir_all(ozel.cargo().join("registry/index")).unwrap();
        assert_eq!(registry_secimi(false, &ozel, &ayar), None);
    }

    #[test]
    fn program_yalnizca_gorunen_yollarda_aranir() {
        let d = tempfile::tempdir().unwrap();
        let gorunen = d.path().join("gorunen");
        let gizli = d.path().join("gizli");
        for dizin in [&gorunen, &gizli] {
            std::fs::create_dir_all(dizin).unwrap();
            let yol = dizin.join("arac");
            std::fs::write(&yol, "#!/bin/sh\n").unwrap();
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&yol, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let alan = AlanBilgisi {
            kok: d.path().join("alan"),
            gizli: Default::default(),
            korumali: Vec::new(),
            git_dizinleri: Vec::new(),
        };
        let path = std::env::join_paths([&gizli, &gorunen]).unwrap();
        let ortam = vec![("PATH".to_string(), path.to_string_lossy().into_owned())];
        let bulunan = programi_bul("arac", &ortam, std::slice::from_ref(&gorunen), &alan).unwrap();
        assert_eq!(bulunan, gorunen.join("arac"));
        match programi_bul("arac", &ortam, &[], &alan) {
            Err(BaslatmaHatasi::ProgramYok { path, .. }) => assert_eq!(path, ""),
            s => panic!("{s:?}"),
        }
    }
}
