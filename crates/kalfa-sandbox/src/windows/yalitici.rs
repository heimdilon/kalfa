//! Windows yalıtıcısı: AppContainer, çalışma alanı izinleri ve Job Object
//! (aşama 2 tasarımı §9).

use std::ffi::OsString;
use std::io;
use std::os::windows::fs::MetadataExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use std::sync::{Mutex, RwLock};

use crate::profil::{self, AlanBilgisi, GitKimligi, Korumali, OzelDizinler, alan_kimligi};
use crate::{
    BaslatmaHatasi, CalistirmaIstegi, HazirAlan, KomutBicimi, Surec, Yalitici, YalitimDurumu,
    YalitimYoklukNedeni, program,
};

use super::ace_kaydi::{self, AceKaydi, Girdi};
use super::guvenlik::{
    self, Izin, MASKE_DEGISTIR, MASKE_LISTELE, MASKE_OKU, MASKE_TAM, MIRAS, MIRAS_YOK, SISTEM, Sid,
    YONETICILER,
};
use super::kap::{self, KapIstegi};
use super::komut_satiri;
use super::kurulum::{self, KurulumKaydi};
use super::msvc::MsvcOrtami;
use windows_sys::Win32::Foundation::{HANDLE, WAIT_ABANDONED_0, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{
    CreateMutexW, INFINITE, ReleaseMutex, WaitForSingleObject,
};

const ARKA_UC: &str = "windows";
const TALIMAT: &str = "Windows yalıtımı hazır değil. Ayarlar'da Kalfa yalıtımını kurun; kurulum tamamlanınca yeniden deneyin.";
const PROFIL_ON_EKI: &str = "Kalfa.Alan.";
const DENEME_PROFILI: &str = "Kalfa.Deneme";
const DENEME_PROFILI_KILIDI: &str = r"Local\Kalfa.Deneme";
/// Çalışma alanlarının üst dizinleri alanlar arasında ortaktır; DACL'lerini
/// oku-değiştir-yaz ile güncelleyen adımlar süreçler arasında sıralanır.
const UST_DIZIN_KILIDI: &str = r"Local\Kalfa.UstDizinler";
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;

#[derive(Clone, Default)]
pub struct WindowsAyari {
    /// Kullanıcının ev dizini; araç zincirleri buradan bulunur.
    pub ev: PathBuf,
    /// Çalışma alanına özel dizinlerin kökü (`%LOCALAPPDATA%\Kalfa\yalitim`).
    pub yalitim_dizini: PathBuf,
    /// Kullanıcının ayarlardan eklediği araç zinciri dizinleri.
    pub ek_okunur: Vec<PathBuf>,
    /// Kurulu Kalfa'nın getirdiği Python ve Node dizinleri
    /// (`sistem::kurulu_arac_zinciri`). Program Files'tadırlar ve `ALL
    /// APPLICATION PACKAGES` onları okur; izin verilmez. Kabın `PATH`'inde,
    /// bu sırayla, kullanıcının bilgisayarındaki araçlardan önce gelirler
    /// (yalnızca MSVC'nin `bin`'i onlardan öncedir).
    pub kalfa_araclari: Vec<PathBuf>,
    /// Yalıtım içindeki git yapılandırmasında kullanılacak kimlik.
    pub git: GitKimligi,
    /// Ağ köprüsü ikilisinin yolu (`kalfa-kopru.exe`).
    pub kopru: Option<PathBuf>,
    /// Testlerde sahte çözümleyici kullanmak için.
    pub vekil_cozucu: Option<std::sync::Arc<dyn crate::vekil::Cozucu>>,
    /// Testlerde 127.0.0.1'e erişime izin vermek için.
    pub vekil_yerel_izin: bool,
}

impl std::fmt::Debug for WindowsAyari {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsAyari")
            .field("ev", &self.ev)
            .field("yalitim_dizini", &self.yalitim_dizini)
            .field("ek_okunur", &self.ek_okunur)
            .field("kalfa_araclari", &self.kalfa_araclari)
            .field("git", &self.git)
            .field("kopru", &self.kopru)
            .field("vekil_yerel_izin", &self.vekil_yerel_izin)
            .finish()
    }
}

impl PartialEq for WindowsAyari {
    fn eq(&self, other: &Self) -> bool {
        self.ev == other.ev
            && self.yalitim_dizini == other.yalitim_dizini
            && self.ek_okunur == other.ek_okunur
            && self.kalfa_araclari == other.kalfa_araclari
            && self.git == other.git
            && self.kopru == other.kopru
            && self.vekil_yerel_izin == other.vekil_yerel_izin
    }
}

impl Eq for WindowsAyari {}

pub use crate::vekil::e2e_cozucu_ve_yerel_izin;

impl WindowsAyari {
    /// Var olan araç zinciri dizinleri; Windows'ta izin yeteneğiyle okunur.
    /// cargo'nun `registry` ve `git` önbellekleri de okunur: ağsız komut
    /// geliştiricinin indirdiği crate'lerle derler (2d-3, A15'in gelişmiş
    /// yolu). `.cargo`'nun kökü izin almaz; oradaki `credentials.toml` ve
    /// `config.toml` kapta okunamaz.
    fn okunur(&self) -> Vec<PathBuf> {
        let mut yollar = vec![
            self.ev.join(r".cargo\bin"),
            self.ev.join(r".cargo\registry"),
            self.ev.join(r".cargo\git"),
            self.ev.join(".rustup"),
        ];
        yollar.extend(self.ek_okunur.iter().cloned());
        yollar.retain(|y| y.is_dir());
        yollar.sort();
        yollar.dedup_by(|a, b| ayni_yol(a, b));
        yollar
    }
}

/// Windows arka ucunun algılayıcısı.
#[derive(Debug, Default)]
pub struct WindowsAlgilayici;

impl WindowsAlgilayici {
    /// Kurulum kaydını, sürücü köklerini ve gerçek bir AppContainer
    /// başlatmasını sınar (§9.2).
    pub fn algila(&self, _ayar: &WindowsAyari) -> YalitimDurumu {
        let dizin = match kurulum::kayit_dizini() {
            Ok(d) => d,
            Err(e) => {
                return kurulum_bozuk("kurulum-dizini", &format!("Kurulum dizini okunamadı: {e}"));
            }
        };
        let kayit = match KurulumKaydi::oku(&dizin) {
            Ok(Some(k)) => k,
            Ok(None) => {
                return kurulum_gerekli("kurulum-kaydi", "Kalfa Windows yalıtımı kurulu değil.");
            }
            Err(e) => {
                return kurulum_bozuk("kurulum-kaydi", &format!("Kurulum kaydı okunamadı: {e}"));
            }
        };
        let kokler = guvenlik::sabit_diskler();
        if kokler.is_empty() {
            return kurulum_gerekli("sabit-disk", "ACL destekleyen sabit disk bulunamadı.");
        }
        let yetenek = match guvenlik::arac_zinciri_sidi() {
            Ok(s) => s,
            Err(e) => {
                return kurulum_bozuk(
                    "yetenek-sid",
                    &format!("Araç zinciri yeteneği bulunamadı: {e}"),
                );
            }
        };
        for kok in kokler {
            if !kayit.kok_izinleri.iter().any(|k| ayni_yol(k, &kok)) {
                return kurulum_gerekli(
                    "kurulum-kaydi",
                    &format!("{} sürücüsü kurulum kaydında yok.", kok.display()),
                );
            }
            match guvenlik::kok_izni_var_mi(&kok, &yetenek) {
                Ok(true) => {}
                Ok(false) => {
                    return kurulum_gerekli(
                        "kok-izni",
                        &format!("{} sürücüsünün kök izni eksik.", kok.display()),
                    );
                }
                Err(e) => {
                    return kurulum_bozuk(
                        "kok-izni",
                        &format!("{} denetlenemedi: {e}", kok.display()),
                    );
                }
            }
        }
        // Profil altındaki çalışma alanının yolu çözülürken kap profiller
        // dizinini listeler (A11); izni kurulum verir.
        let profiller = match guvenlik::profiller_dizini() {
            Ok(p) => p,
            Err(e) => {
                return kurulum_bozuk(
                    "profiller-dizini",
                    &format!("Profiller dizini bulunamadı: {e}"),
                );
            }
        };
        match guvenlik::kok_izni_var_mi(&profiller, &yetenek) {
            Ok(true) => {}
            Ok(false) => {
                return kurulum_gerekli(
                    "profiller-dizini",
                    &format!("{} dizininin listeleme izni eksik.", profiller.display()),
                );
            }
            Err(e) => {
                return kurulum_bozuk(
                    "profiller-dizini",
                    &format!("{} denetlenemedi: {e}", profiller.display()),
                );
            }
        }
        match deneme_baslat(&yetenek) {
            Ok(()) => YalitimDurumu::Hazir {
                arka_uc: ARKA_UC.into(),
                surum: "AppContainer".into(),
            },
            Err(e) => kurulum_bozuk(
                "appcontainer-deneme",
                &format!("AppContainer denemesi başarısız: {e}"),
            ),
        }
    }
}

/// AppContainer deneme profili ayrı tutulur; gerçek bir komut ve Job Object
/// ile başlatılması kurulumun kullanılabildiğini doğrular. Ad sabit olduğu
/// için süreçler arası mutex, oluşturma/başlatma/silme adımlarını sıralar.
fn deneme_baslat(yetenek: &Sid) -> io::Result<()> {
    let kilit = AdliKilit::kilitle(DENEME_PROFILI_KILIDI)?;
    let deneme = (|| {
        let profil = guvenlik::profil_olustur(DENEME_PROFILI, "Kalfa Windows yalıtım denemesi")?;
        let cmd = kap::sistem_dizini().join("cmd.exe");
        let komut = komut_satiri::uret(
            &cmd,
            &["/d".into(), "/c".into(), "exit".into(), "0".into()],
            &cmd,
        )?;
        let is = crate::agac::Is::yeni(true)?;
        let dizin = kap::sistem_dizini();
        let ortam = temel_ortam();
        let istek = KapIstegi {
            komut: &komut,
            dizin: &dizin,
            ortam: &ortam,
            paket: &profil,
            yetenekler: std::slice::from_ref(yetenek),
        };
        let kap = kap::baslat(&istek, &is)?;
        let kod = kap::bekle(&kap.surec);
        drop(kap);
        drop(is);
        match kod {
            Some(0) => Ok(()),
            Some(kod) => Err(io::Error::other(format!("cmd.exe {kod} koduyla bitti"))),
            None => Err(io::Error::other("cmd.exe çıkış kodu okunamadı")),
        }
    })();
    // Profil oluşturma ya da başlatma başarısız olsa da önceki yarım denemeden
    // kalmış profili silmeyi dene. Hata algılayıcıya yalıtım hatası olarak döner.
    let temizleme = guvenlik::profil_sil(DENEME_PROFILI);
    let kilit_birakma = kilit.birak();
    match (deneme, temizleme, kilit_birakma) {
        (Ok(()), Ok(()), Ok(())) => Ok(()),
        (deneme, temizleme, kilit_birakma) => {
            let mut hatalar = Vec::new();
            if let Err(hata) = deneme {
                hatalar.push(format!("deneme başarısız: {hata}"));
            }
            if let Err(hata) = temizleme {
                hatalar.push(format!("deneme profili silinemedi: {hata}"));
            }
            if let Err(hata) = kilit_birakma {
                hatalar.push(format!("deneme profili kilidi bırakılamadı: {hata}"));
            }
            Err(io::Error::other(hatalar.join("; ")))
        }
    }
}

/// Süreçler arası adlı mutex: sabit `Kalfa.Deneme` profili üzerindeki
/// denemeleri ve ortak üst dizinlerin DACL güncellemelerini sıralar. Mutex
/// tutamacı bırakıldığında Windows kilidi de bırakılır.
struct AdliKilit {
    tutamac: OwnedHandle,
    kilitli: bool,
}

impl AdliKilit {
    fn kilitle(ad: &str) -> io::Result<Self> {
        let ad = guvenlik::genis(ad);
        // SAFETY: mutex adı NUL ile sonlanır; Windows geçerli, sahip olunan bir tutamaç döndürür.
        let tutamac = unsafe { CreateMutexW(null_mut(), 0, ad.as_ptr()) };
        if tutamac.is_null() {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: `CreateMutexW` yeni ve geçerli tutamaç döndürdü; `OwnedHandle` onu kapatır.
        let tutamac = unsafe { OwnedHandle::from_raw_handle(tutamac as _) };
        // SAFETY: tutamaç bu yapıya aittir; bekleme süresi sonsuzdur.
        let sonuc = unsafe { WaitForSingleObject(tutamac.as_raw_handle() as HANDLE, INFINITE) };
        if sonuc != WAIT_OBJECT_0 && sonuc != WAIT_ABANDONED_0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            tutamac,
            kilitli: true,
        })
    }

    fn birak(mut self) -> io::Result<()> {
        // SAFETY: bu iş parçacığı mutex'i `kilitle` içinde aldı.
        if unsafe { ReleaseMutex(self.tutamac.as_raw_handle() as HANDLE) } == 0 {
            return Err(io::Error::last_os_error());
        }
        self.kilitli = false;
        Ok(())
    }
}

impl Drop for AdliKilit {
    fn drop(&mut self) {
        if self.kilitli {
            // SAFETY: bu iş parçacığı mutex'i `kilitle` içinde aldı; bu yol hata/panik temizliğidir.
            unsafe { ReleaseMutex(self.tutamac.as_raw_handle() as HANDLE) };
            self.kilitli = false;
        }
    }
}

fn temel_ortam() -> Vec<(String, String)> {
    let mut ortam: Vec<(String, String)> = std::env::vars_os()
        .filter_map(|(ad, deger)| {
            let ad = ad.to_string_lossy().into_owned();
            let deger = deger.to_string_lossy().into_owned();
            crate::ortam::izinli_mi(&ad).then_some((ad, deger))
        })
        .collect();
    if let Some(root) = std::env::var_os("SystemRoot") {
        deger_ata(&mut ortam, "SystemRoot", root.to_string_lossy().as_ref());
        deger_ata(&mut ortam, "windir", root.to_string_lossy().as_ref());
    }
    ortam
}

/// `denetim` BT için teknik ayrıntıya girer (§9.8): hangi denetimin düştüğü.
fn kurulum_gerekli(denetim: &str, neden: &str) -> YalitimDurumu {
    YalitimDurumu::Yok {
        neden: YalitimYoklukNedeni::KurulumGerekli,
        talimat: format!("{neden} {TALIMAT}"),
        ayrinti: format!("denetim={denetim}; {neden}"),
    }
}

fn kurulum_bozuk(denetim: &str, neden: &str) -> YalitimDurumu {
    YalitimDurumu::Yok {
        neden: YalitimYoklukNedeni::KurulumBozuk,
        talimat: format!("{neden} {TALIMAT}"),
        ayrinti: format!("denetim={denetim}; {neden}"),
    }
}

/// Windows arka ucunun yaşayan örneği.
pub struct WindowsYalitici {
    ayar: RwLock<WindowsAyari>,
    durum: RwLock<YalitimDurumu>,
    degisince: Box<dyn Fn(&YalitimDurumu) + Send + Sync>,
    ace_yolu: PathBuf,
    ace: Mutex<AceKaydi>,
    /// Eski araç zinciri izinleri güvenle geri alınamadığında yalıtımlı
    /// başlatmaları kapalı tutar; yeniden algılama bu hatayı temizlemez.
    okunur_izin_hatasi: RwLock<Option<String>>,
}

impl WindowsYalitici {
    /// Kurulum algılamasını hemen yapar.
    pub fn yeni(ayar: WindowsAyari, degisince: Box<dyn Fn(&YalitimDurumu) + Send + Sync>) -> Self {
        kap::hata_kutularini_kapat();
        let mut durum = WindowsAlgilayici.algila(&ayar);
        let ace_yolu = ayar.yalitim_dizini.join("ace-kaydi.jsonl");
        let mut ace = AceKaydi::oku(&ace_yolu);
        // Bozuk kayıt, alani_hazirla/alani_kaldir içindeki kanıta dayalı
        // kurtarmaya bırakılır. Bu durumda ortak ACL'lere dokunma; yalıtımlı
        // başlatma kaydın bozukluğunu ayrıca denetleyip fail closed kalır.
        let okunur_izin_hatasi = if ace.bozuk_mu() {
            None
        } else {
            arac_yollarini_esitle(&mut ace, &ayar, None)
                .err()
                .map(|hata| format!("İzinli araç zinciri dizinleri güvenle hazırlanamadı: {hata}"))
        };
        if let Some(neden) = &okunur_izin_hatasi {
            durum = kurulum_bozuk("arac-zinciri-izni", neden);
        }
        Self {
            ayar: RwLock::new(ayar),
            durum: RwLock::new(durum),
            degisince,
            ace_yolu,
            ace: Mutex::new(ace),
            okunur_izin_hatasi: RwLock::new(okunur_izin_hatasi),
        }
    }

    fn ayar(&self) -> WindowsAyari {
        self.ayar.read().unwrap_or_else(|z| z.into_inner()).clone()
    }

    fn ace(&self) -> std::sync::MutexGuard<'_, AceKaydi> {
        self.ace.lock().unwrap_or_else(|z| z.into_inner())
    }

    /// Kaldırılan çalışma alanının izinlerini geri alır, sonra profilini siler
    /// (§9.2, §9.4). Uygulama bu metodu çalışma alanı kaldırılırken çağırır.
    pub fn alani_kaldir(&self, alan: &AlanBilgisi) -> io::Result<()> {
        let kimlik = alan_kimligi(&alan.kok);
        let mut ace = self.ace();
        self.alani_kaldir_kilitli(&mut ace, &kimlik, &alan.kok)
    }

    /// ACE kilidi alınmışken geri alır. Kimlikten kaldırma aynı kilit altında
    /// kayıttaki kökü çözüp bu işleve geçer; araya yeni hazırlık giremez.
    fn alani_kaldir_kilitli(&self, ace: &mut AceKaydi, kimlik: &str, kok: &Path) -> io::Result<()> {
        let ad = profil_adi(kimlik);
        let paket = guvenlik::profil_sidi(&ad)?;
        let hatali = ace.bozuk_mu();
        if hatali {
            taramali_geri_al(ace, kok, kimlik, &paket)?;
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ACE kaydı bozuk; kanıtlanan izinler geri alındı ancak çalışma alanı kaldırılamadı",
            ));
        } else {
            // Başka bir iç alanın önceki DACL'i bu paketin miras kalan ACE'ini
            // taşıyabilir. Alanı geri almadan önce anlık görüntüleri temizle;
            // böylece iç alan daha sonra kaldırılırsa silinmiş SID dirilmez.
            ace.dacl_oncelerini_guncelle(|onceki| {
                guvenlik::dacl_sddl_sidini_kaldir(onceki, &paket)
            })?;
            // Sunucuların yetenek SID'leri (1.1a §5.6) alanla birlikte gider.
            let onek = format!("{kimlik}/mcp/");
            let mut sunucular: Vec<String> = ace
                .girdiler()
                .iter()
                .map(|g| g.alan().to_string())
                .filter(|a| a.starts_with(&onek))
                .collect();
            sunucular.sort();
            sunucular.dedup();
            for kayit in sunucular {
                kayda_gore_geri_al(ace, &kayit)?;
                ace.alani_cikar(&kayit, &[])?;
            }
            kayda_gore_geri_al(ace, kimlik)?;
        }
        guvenlik::profil_sil(&ad)?;
        ace.alani_cikar(kimlik, &[])?;
        *ace = AceKaydi::oku(&self.ace_yolu);
        Ok(())
    }
}

impl Yalitici for WindowsYalitici {
    fn durum(&self) -> YalitimDurumu {
        if let Some(neden) = self
            .okunur_izin_hatasi
            .read()
            .unwrap_or_else(|z| z.into_inner())
            .as_ref()
        {
            return kurulum_bozuk("arac-zinciri-izni", neden);
        }
        self.durum.read().unwrap_or_else(|z| z.into_inner()).clone()
    }

    fn hazirlanmis_alanlar(&self) -> Vec<HazirAlan> {
        self.ace()
            .hazir_alanlar()
            .into_iter()
            .filter(|(kimlik, kok)| kayitli_kok_kimligi(kok).eq(kimlik))
            .map(|(kimlik, kok)| HazirAlan { kimlik, kok })
            .collect()
    }

    fn alani_kaldir_kimlikle(&self, kimlik: &str) -> Result<(), String> {
        let mut ace = self.ace();
        let kok = ace
            .hazir_alanlar()
            .into_iter()
            .find_map(|(alan, kok)| (alan == kimlik).then_some(kok))
            .ok_or_else(|| "Hazırlanmış çalışma alanı bulunamadı".to_string())?;
        if kayitli_kok_kimligi(&kok) != kimlik {
            return Err("Çalışma alanının kayıtlı kimliği artık köküyle eşleşmiyor".into());
        }
        self.alani_kaldir_kilitli(&mut ace, kimlik, &kok)
            .map_err(|hata| hata.to_string())
    }

    fn profil_ozeti(&self, ag: bool, liste_ozeti: Option<&str>) -> Option<String> {
        if !self.durum().hazir_mi() {
            return None;
        }
        let ayar = self.ayar();
        let ace = self.ace();
        let (araclar, uygun) = arac_yollarini_siniflandir(&ace, &ayar, None).ok()?;
        if !uygun {
            return None;
        }
        let mut okunur = sistem_yollari();
        okunur.extend(araclar);
        Some(profil::ozet(ARKA_UC, &okunur, ag, liste_ozeti))
    }

    fn baslat(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi> {
        crate::kip_uyusmasi(istek.alan.is_some(), self.durum().hazir_mi())?;
        match &istek.alan {
            Some(alan) => {
                // ACE kilidi ACL güncellemelerini ve yeni süreç başlatmayı
                // sıraya sokar. Böylece kaldırılmakta olan izinle yarışan bir
                // süreç sınır değişikliğinden sonra başlatılamaz.
                let ace = self.ace();
                match self.yalitilmis_baslat(istek, alan, &ace) {
                    Ok(s) => Ok(s),
                    Err(e) => {
                        drop(ace);
                        if !matches!(&e, BaslatmaHatasi::ProgramYok { .. }) {
                            let _ = self.yeniden_algila();
                        }
                        Err(e)
                    }
                }
            }
            None => Surec::baslat(istek),
        }
    }

    fn baslat_kabuk_etkilesimli(
        &self,
        istek: &CalistirmaIstegi,
        terminal: Option<crate::TerminalBoyutu>,
    ) -> Result<crate::EtkilesimliSurec, BaslatmaHatasi> {
        crate::kip_uyusmasi(istek.alan.is_some(), self.durum().hazir_mi())?;
        if let Some(boyut) = terminal {
            boyut.dogrula()?;
        }
        match &istek.alan {
            Some(alan) => {
                let ace = self.ace();
                match self.yalitilmis_etkilesimli_baslat(istek, alan, &ace, terminal) {
                    Ok(s) => Ok(s),
                    Err(e) => {
                        drop(ace);
                        if !matches!(&e, BaslatmaHatasi::ProgramYok { .. }) {
                            let _ = self.yeniden_algila();
                        }
                        Err(e)
                    }
                }
            }
            None => Surec::baslat_kabuk_etkilesimli(istek, terminal),
        }
    }

    fn baslat_etkilesimli(
        &self,
        istek: &crate::EtkilesimliIstek,
    ) -> Result<crate::EtkilesimliSurec, BaslatmaHatasi> {
        crate::etkilesimli::sunucu_adi_dogrula(&istek.sunucu)?;
        crate::mcp_kip_uyusmasi(istek.kip, self.durum().hazir_mi())?;
        if istek.kip == crate::CalistirmaKipi::TamErisim {
            let ayar = self.ayar();
            // Kök iki kipte ortaktır; yalıtımlı başlatma onu korumalı bekler.
            let kimlik = alan_kimligi(&istek.alan.kok);
            let yetenek = guvenlik::yetenek_sidi(&sunucu_yetenek_adi(&kimlik, &istek.sunucu))
                .map_err(yalitim_hatasi)?;
            sunucu_kokunu_kur(
                &self.ace(),
                &ayar.yalitim_dizini,
                &kimlik,
                &istek.sunucu,
                &yetenek,
            )
            .map_err(yalitim_hatasi)?;
            return crate::surec::tam_erisimle(istek, &ayar.yalitim_dizini, &ayar.git);
        }
        // Kabuktaki gibi: ACE kilidi izin değişikliklerini ve başlatmayı
        // sıraya sokar.
        let mut ace = self.ace();
        self.sunucuyu_baslat(istek, &mut ace)
    }

    fn sunucu_kokunu_kaldir(&self, alan_koku: &Path, sunucu: &str) -> Result<(), String> {
        crate::etkilesimli::sunucu_adi_dogrula(sunucu).map_err(|e| e.to_string())?;
        let kimlik = alan_kimligi(alan_koku);
        let kayit = sunucu_kaydi(&kimlik, sunucu);
        let mut ace = self.ace();
        if ace.bozuk_mu() {
            return Err("ACE kaydı bozuk; sunucu kökü kaldırılmadı".into());
        }
        kayda_gore_geri_al(&mut ace, &kayit).map_err(|e| e.to_string())?;
        ace.alani_cikar(&kayit, &[]).map_err(|e| e.to_string())?;
        let kok = crate::sunucu_ozel(&self.ayar().yalitim_dizini, &kimlik, sunucu).kok;
        match std::fs::remove_dir_all(&kok) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn yeniden_algila(&self) -> YalitimDurumu {
        let algilanan = WindowsAlgilayici.algila(&self.ayar());
        let yeni = self
            .okunur_izin_hatasi
            .read()
            .unwrap_or_else(|z| z.into_inner())
            .as_ref()
            .map_or(algilanan, |neden| kurulum_bozuk("arac-zinciri-izni", neden));
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
        let (sonuc, hata_nedeni) = {
            let mut ace = self.ace();
            let mut yeni_ayar = self.ayar();
            yeni_ayar.ek_okunur = dizinler.clone();
            let sonuc = arac_yollarini_esitle(&mut ace, &yeni_ayar, None).map(|_| ());
            let hata_nedeni = sonuc
                .as_ref()
                .err()
                .map(|hata| format!("İzinli araç zinciri dizinleri güvenle hazırlanamadı: {hata}"));
            if let Some(neden) = &hata_nedeni {
                *self
                    .okunur_izin_hatasi
                    .write()
                    .unwrap_or_else(|z| z.into_inner()) = Some(neden.clone());
            }
            self.ayar
                .write()
                .unwrap_or_else(|z| z.into_inner())
                .ek_okunur = dizinler;
            (sonuc, hata_nedeni)
        };
        match sonuc {
            Ok(()) => {
                let onceki_hata = self
                    .okunur_izin_hatasi
                    .write()
                    .unwrap_or_else(|z| z.into_inner())
                    .take();
                if onceki_hata.is_some() {
                    let _ = self.yeniden_algila();
                }
            }
            Err(_) => {
                let Some(neden) = hata_nedeni else {
                    return;
                };
                let yeni = kurulum_bozuk("arac-zinciri-izni", &neden);
                let degisti = {
                    let mut durum = self.durum.write().unwrap_or_else(|z| z.into_inner());
                    let degisti = *durum != yeni;
                    *durum = yeni.clone();
                    degisti
                };
                if degisti {
                    (self.degisince)(&yeni);
                }
            }
        }
    }

    fn okunur_yonetici_gerekli(&self, dizin: &Path) -> bool {
        okunur_yonetici_gerekli(dizin)
    }

    fn hazirlik_gerekli(&self, kok: &Path) -> bool {
        let ace = self.ace();
        self.okunur_izin_hatasi
            .read()
            .unwrap_or_else(|z| z.into_inner())
            .is_some()
            || ace.bozuk_mu()
            || !ace.hazir_mi(&alan_kimligi(kok))
    }

    fn alani_hazirla(
        &self,
        alan: &AlanBilgisi,
        hazirlik_onaylandi: bool,
        ilerleme: &mut dyn FnMut(&str),
    ) -> Result<(), BaslatmaHatasi> {
        if !self.durum().hazir_mi() {
            return Err(BaslatmaHatasi::Yalitim(
                "Windows yalıtımı hazır değil; çalışma alanı hazırlanmadı".into(),
            ));
        }
        let kimlik = alan_kimligi(&alan.kok);
        let ad = profil_adi(&kimlik);
        let mut ace = self.ace();
        if let Some(neden) = self
            .okunur_izin_hatasi
            .read()
            .unwrap_or_else(|z| z.into_inner())
            .as_ref()
        {
            return Err(BaslatmaHatasi::Yalitim(neden.clone()));
        }
        if !hazirlik_onaylandi && !ace.hazir_mi(&kimlik) {
            return Err(BaslatmaHatasi::Yalitim(
                "çalışma alanı yalıtım için hazırlanmamış ve hazırlık onaylanmamış".into(),
            ));
        }
        if ace.bozuk_mu() {
            ilerleme("Bozuk izin kaydı taranıyor ve geri alınıyor…");
            let paket = guvenlik::profil_sidi(&ad).map_err(yalitim_hatasi)?;
            taramali_geri_al(&mut ace, &alan.kok, &kimlik, &paket).map_err(yalitim_hatasi)?;
            return Err(yalitim_hatasi(io::Error::new(
                io::ErrorKind::InvalidData,
                "ACE kaydı bozuk; bilinen izinler geri alındı ancak kayıtta olmayan miras değişiklikleri bilinmiyor. Kayıt ve profil korundu; çalışma alanı hazırlanmadı",
            )));
        }
        let ayar = self.ayar();
        hazirla(&mut ace, &ayar, alan, &kimlik, &ad, ilerleme).map_err(yalitim_hatasi)
    }
}

impl WindowsYalitici {
    fn yalitilmis_baslat(
        &self,
        istek: &CalistirmaIstegi,
        alan: &AlanBilgisi,
        ace: &AceKaydi,
    ) -> Result<Surec, BaslatmaHatasi> {
        if let Some(neden) = self
            .okunur_izin_hatasi
            .read()
            .unwrap_or_else(|z| z.into_inner())
            .as_ref()
        {
            return Err(BaslatmaHatasi::Yalitim(neden.clone()));
        }
        if ace.bozuk_mu() {
            return Err(BaslatmaHatasi::Yalitim(
                "ACE kaydı bozuk; yalıtımlı komut başlatılmadı".into(),
            ));
        }
        if !ace.hazir_mi(&alan_kimligi(&alan.kok)) {
            return Err(BaslatmaHatasi::Yalitim(
                "çalışma alanı yalıtım için hazırlanmadı".into(),
            ));
        }
        let ayar = self.ayar();
        let kimlik = alan_kimligi(&alan.kok);
        let ad = profil_adi(&kimlik);
        let paket = guvenlik::profil_olustur(&ad, "Kalfa çalışma alanı yalıtımı")
            .map_err(yalitim_hatasi)?;
        let yetenek = guvenlik::arac_zinciri_sidi().map_err(yalitim_hatasi)?;
        let yetenekler = vec![yetenek];
        let ozel = OzelDizinler::yol(&ayar.yalitim_dizini, &kimlik);
        std::fs::create_dir_all(&ayar.yalitim_dizini).map_err(yalitim_hatasi)?;
        profil::guvenli_dizin(&ayar.yalitim_dizini, Path::new(&kimlik)).map_err(yalitim_hatasi)?;
        ozel.hazirla(&ayar.git).map_err(yalitim_hatasi)?;
        // Windows, kabın `TEMP` ve `TMP`'sini verilen değerden bağımsız olarak
        // `%LOCALAPPDATA%\Packages\<paket>\AC\Temp`'e çevirir ama dizini
        // kurmaz; `LOCALAPPDATA` özel evdir.
        let _ = profil::guvenli_dizin(
            &ozel.kok,
            Path::new("ev/Packages")
                .join(&ad)
                .join("AC")
                .join("Temp")
                .as_path(),
        )
        .map_err(yalitim_hatasi)?;
        let _ =
            profil::guvenli_dizin(&ozel.kok, Path::new("ev/Roaming")).map_err(yalitim_hatasi)?;
        let alan_kok =
            guvenlik::sade(&std::fs::canonicalize(&alan.kok).map_err(BaslatmaHatasi::GirisCikis)?);
        let (araclar, uygun) =
            arac_yollarini_siniflandir(ace, &ayar, Some(&alan_kok)).map_err(yalitim_hatasi)?;
        if !uygun {
            return Err(BaslatmaHatasi::Yalitim(
                "araç zinciri dizini çalışma alanı ya da Kalfa'nın özel verisiyle çakışıyor".into(),
            ));
        }
        let ortam = yalitim_ortami(&istek.ortam, &ozel, &ayar, &araclar, istek.ag);
        let kokler = izinli_kokler(&ayar, alan, &ozel, &araclar);
        let (program, argumanlar) = program::coz(&istek.komut, &ortam)?;
        let program = gorunen_program(&program, &kokler, alan, &ortam)?;
        let cmd = kap::sistem_dizini().join("cmd.exe");
        let komut = komut_satiri::uret(&program, &argumanlar, &cmd).map_err(yalitim_hatasi)?;
        let calisma_dizini = guvenlik::sade(
            &std::fs::canonicalize(&istek.dizin).map_err(BaslatmaHatasi::GirisCikis)?,
        );
        let alan_koku =
            guvenlik::sade(&std::fs::canonicalize(&alan.kok).map_err(BaslatmaHatasi::GirisCikis)?);
        if !altinda(&calisma_dizini, &alan_koku) {
            return Err(BaslatmaHatasi::Yalitim(
                "çalışma dizini çalışma alanının dışında".into(),
            ));
        }

        let is = crate::agac::Is::yeni(true).map_err(yalitim_hatasi)?;
        if istek.ag {
            let (kopru_komut, kap_ortami, ag_kapatici) = ag_hazirla(
                &ayar,
                &ozel,
                &paket,
                &is,
                istek.ag_listesi.clone(),
                &komut,
                ortam,
            )?;
            let kap_istegi = KapIstegi {
                komut: &kopru_komut,
                dizin: &calisma_dizini,
                ortam: &kap_ortami,
                paket: &paket,
                yetenekler: &yetenekler,
            };
            let kap = kap::baslat(&kap_istegi, &is).map_err(yalitim_hatasi)?;
            let mut surec = Surec::kaptan(kap, is);
            surec.ag_kapat_ekle(ag_kapatici);
            Ok(surec)
        } else {
            let kap_istegi = KapIstegi {
                komut: &komut,
                dizin: &calisma_dizini,
                ortam: &ortam,
                paket: &paket,
                yetenekler: &yetenekler,
            };
            let kap = kap::baslat(&kap_istegi, &is).map_err(yalitim_hatasi)?;
            Ok(Surec::kaptan(kap, is))
        }
    }

    fn yalitilmis_etkilesimli_baslat(
        &self,
        istek: &CalistirmaIstegi,
        alan: &AlanBilgisi,
        ace: &AceKaydi,
        terminal: Option<crate::TerminalBoyutu>,
    ) -> Result<crate::EtkilesimliSurec, BaslatmaHatasi> {
        if let Some(neden) = self
            .okunur_izin_hatasi
            .read()
            .unwrap_or_else(|z| z.into_inner())
            .as_ref()
        {
            return Err(BaslatmaHatasi::Yalitim(neden.clone()));
        }
        if ace.bozuk_mu() {
            return Err(BaslatmaHatasi::Yalitim(
                "ACE kaydı bozuk; yalıtımlı komut başlatılmadı".into(),
            ));
        }
        if !ace.hazir_mi(&alan_kimligi(&alan.kok)) {
            return Err(BaslatmaHatasi::Yalitim(
                "çalışma alanı yalıtım için hazırlanmadı".into(),
            ));
        }
        let ayar = self.ayar();
        let kimlik = alan_kimligi(&alan.kok);
        let ad = profil_adi(&kimlik);
        let paket = guvenlik::profil_olustur(&ad, "Kalfa çalışma alanı yalıtımı")
            .map_err(yalitim_hatasi)?;
        let yetenek = guvenlik::arac_zinciri_sidi().map_err(yalitim_hatasi)?;
        let yetenekler = vec![yetenek];
        let ozel = OzelDizinler::yol(&ayar.yalitim_dizini, &kimlik);
        std::fs::create_dir_all(&ayar.yalitim_dizini).map_err(yalitim_hatasi)?;
        profil::guvenli_dizin(&ayar.yalitim_dizini, Path::new(&kimlik)).map_err(yalitim_hatasi)?;
        ozel.hazirla(&ayar.git).map_err(yalitim_hatasi)?;
        let _ = profil::guvenli_dizin(
            &ozel.kok,
            Path::new("ev/Packages")
                .join(&ad)
                .join("AC")
                .join("Temp")
                .as_path(),
        )
        .map_err(yalitim_hatasi)?;
        let _ =
            profil::guvenli_dizin(&ozel.kok, Path::new("ev/Roaming")).map_err(yalitim_hatasi)?;
        let alan_kok =
            guvenlik::sade(&std::fs::canonicalize(&alan.kok).map_err(BaslatmaHatasi::GirisCikis)?);
        let (araclar, uygun) =
            arac_yollarini_siniflandir(ace, &ayar, Some(&alan_kok)).map_err(yalitim_hatasi)?;
        if !uygun {
            return Err(BaslatmaHatasi::Yalitim(
                "araç zinciri dizini çalışma alanı ya da Kalfa'nın özel verisiyle çakışıyor".into(),
            ));
        }
        let mut ortam = yalitim_ortami(&istek.ortam, &ozel, &ayar, &araclar, istek.ag);
        if terminal.is_some() {
            crate::surec::terminal_ortami_ekle(&mut ortam);
        }
        let kokler = izinli_kokler(&ayar, alan, &ozel, &araclar);
        let (program, argumanlar) = program::coz(&istek.komut, &ortam)?;
        let program = gorunen_program(&program, &kokler, alan, &ortam)?;
        let cmd = kap::sistem_dizini().join("cmd.exe");
        let komut = komut_satiri::uret(&program, &argumanlar, &cmd).map_err(yalitim_hatasi)?;
        let calisma_dizini = guvenlik::sade(
            &std::fs::canonicalize(&istek.dizin).map_err(BaslatmaHatasi::GirisCikis)?,
        );
        let alan_koku =
            guvenlik::sade(&std::fs::canonicalize(&alan.kok).map_err(BaslatmaHatasi::GirisCikis)?);
        if !altinda(&calisma_dizini, &alan_koku) {
            return Err(BaslatmaHatasi::Yalitim(
                "çalışma dizini çalışma alanının dışında".into(),
            ));
        }

        let is = crate::agac::Is::yeni(true).map_err(yalitim_hatasi)?;
        let (hedef_komut, hedef_ortam, ag_kapatici) = if istek.ag {
            let (k, o, a) = ag_hazirla(
                &ayar,
                &ozel,
                &paket,
                &is,
                istek.ag_listesi.clone(),
                &komut,
                ortam,
            )?;
            (k, o, Some(a))
        } else {
            (komut, ortam, None)
        };
        let kap_istegi = KapIstegi {
            komut: &hedef_komut,
            dizin: &calisma_dizini,
            ortam: &hedef_ortam,
            paket: &paket,
            yetenekler: &yetenekler,
        };
        let mut surec = match terminal {
            Some(boyut) => {
                let (kap, denetim) =
                    kap::baslat_terminal(&kap_istegi, &is, boyut).map_err(yalitim_hatasi)?;
                crate::EtkilesimliSurec::kaptan_terminalli(kap, is, Some(denetim))?
            }
            None => {
                let kap = kap::baslat_etkilesimli(&kap_istegi, &is).map_err(yalitim_hatasi)?;
                crate::EtkilesimliSurec::kaptan(kap, is)?
            }
        };
        if let Some(k) = ag_kapatici {
            surec.ag_kapat_ekle(k);
        }
        Ok(surec)
    }

    /// Sunucuyu yalıtımda başlatır (1.1a §4.1, §5.6). ACE kilidi tutulurken
    /// çağrılır.
    fn sunucuyu_baslat(
        &self,
        istek: &crate::EtkilesimliIstek,
        ace: &mut AceKaydi,
    ) -> Result<crate::EtkilesimliSurec, BaslatmaHatasi> {
        if let Some(neden) = self
            .okunur_izin_hatasi
            .read()
            .unwrap_or_else(|z| z.into_inner())
            .as_ref()
        {
            return Err(BaslatmaHatasi::Yalitim(neden.clone()));
        }
        if ace.bozuk_mu() {
            return Err(BaslatmaHatasi::Yalitim(
                "ACE kaydı bozuk; sunucu başlatılmadı".into(),
            ));
        }
        let kimlik = alan_kimligi(&istek.alan.kok);
        if !ace.hazir_mi(&kimlik) {
            return Err(BaslatmaHatasi::Yalitim(
                "çalışma alanı yalıtım için hazırlanmadı".into(),
            ));
        }
        let ayar = self.ayar();
        let ad = profil_adi(&kimlik);
        let paket = guvenlik::profil_olustur(&ad, "Kalfa çalışma alanı yalıtımı")
            .map_err(yalitim_hatasi)?;
        let arac = guvenlik::arac_zinciri_sidi().map_err(yalitim_hatasi)?;
        let sunucu_yetenegi = guvenlik::yetenek_sidi(&sunucu_yetenek_adi(&kimlik, &istek.sunucu))
            .map_err(yalitim_hatasi)?;
        let ozel = sunucu_kokunu_hazirla(
            ace,
            &ayar.yalitim_dizini,
            &kimlik,
            &istek.sunucu,
            &ad,
            &sunucu_yetenegi,
            &ayar.git,
        )
        .map_err(yalitim_hatasi)?;
        let alan_kok = guvenlik::sade(
            &std::fs::canonicalize(&istek.alan.kok).map_err(BaslatmaHatasi::GirisCikis)?,
        );
        let (araclar, uygun) =
            arac_yollarini_siniflandir(ace, &ayar, Some(&alan_kok)).map_err(yalitim_hatasi)?;
        if !uygun {
            return Err(BaslatmaHatasi::Yalitim(
                "araç zinciri dizini çalışma alanı ya da Kalfa'nın özel verisiyle çakışıyor".into(),
            ));
        }
        let ag = istek.ag_listesi.is_some();
        // `yalitim_ortami` kalıtılan ortamı izin listesinden geçirir; adında
        // `TOKEN`/`KEY` geçenleri de düşürür. Sunucunun ortamı yapılandırılmış
        // değerleri taşır (1.1a §5.5): yalıtımın koyduğu değerler kalır,
        // yalıtımın düşürdükleri PATH dışında geri eklenir.
        let sunucu_ortami = crate::sunucu_ortami(&istek.ortam, &ozel);
        let mut ortam = yalitim_ortami(&sunucu_ortami, &ozel, &ayar, &araclar, ag);
        for (ad, deger) in &sunucu_ortami {
            if !ad.eq_ignore_ascii_case("PATH") && deger_al(&ortam, ad).is_none() {
                ortam.push((ad.clone(), deger.clone()));
            }
        }
        let kokler = izinli_kokler(&ayar, &istek.alan, &ozel, &araclar);
        let (program, argumanlar) = program::coz(
            &KomutBicimi::Program {
                program: istek.program.clone(),
                argumanlar: istek.argumanlar.clone(),
            },
            &ortam,
        )?;
        let program = gorunen_program(&program, &kokler, &istek.alan, &ortam)?;
        let komut =
            komut_satiri::uret(&program, &argumanlar, &kap::sistem_dizini().join("cmd.exe"))
                .map_err(yalitim_hatasi)?;
        let yetenekler = vec![arac, sunucu_yetenegi];
        let is = crate::agac::Is::yeni(true).map_err(yalitim_hatasi)?;
        let (komut, ortam, ag_kapat) = if ag {
            let kabuk_ozel = OzelDizinler::yol(&ayar.yalitim_dizini, &kimlik);
            let (k, o, a) = ag_hazirla(
                &ayar,
                &kabuk_ozel,
                &paket,
                &is,
                istek.ag_listesi.clone(),
                &komut,
                ortam,
            )?;
            (k, o, Some(a))
        } else {
            (komut, ortam, None)
        };
        let kap = kap::baslat_etkilesimli(
            &KapIstegi {
                komut: &komut,
                dizin: &ozel.ev,
                ortam: &ortam,
                paket: &paket,
                yetenekler: &yetenekler,
            },
            &is,
        )
        .map_err(yalitim_hatasi)?;
        let mut surec = crate::EtkilesimliSurec::kaptan(kap, is)?;
        if let Some(k) = ag_kapat {
            surec.ag_kapat_ekle(k);
        }
        Ok(surec)
    }
}

/// Köprünün komutu, köprünün ortamı ve ağın kapatıcısı.
type AgHazirligi = (
    komut_satiri::KomutSatiri,
    Vec<(String, String)>,
    crate::surec::AgKapatici,
);

/// Ağlı başlatma (§9.3, 1.1a §4.1): köprü kopyası, vekil ve `is`'e bağlı
/// boru sunucusu. Boru yalnız bu işteki istemciyi kabul eder; köprü de asıl
/// komut da aynı işte başlamalıdır. Asıl komut ve boru adı köprünün ortamına
/// `KALFA_KOPRU_*` olarak girer; köprü onları çocuğa geçirmez. Dönen komut
/// köprüdür. Köprünün kopyası kabuğun özel kökündedir (`ozel`).
fn ag_hazirla(
    ayar: &WindowsAyari,
    ozel: &OzelDizinler,
    paket: &Sid,
    is: &crate::agac::Is,
    liste: Option<crate::vekil::IzinListesi>,
    komut: &komut_satiri::KomutSatiri,
    ortam: Vec<(String, String)>,
) -> Result<AgHazirligi, BaslatmaHatasi> {
    let kaynak_kopru = match ayar.kopru.as_ref() {
        Some(k) if k.is_file() => k,
        _ => {
            return Err(BaslatmaHatasi::Yalitim(
                "Ağlı yalıtımlı komut başlatılamadı: köprü bulunamadı".into(),
            ));
        }
    };
    let kopru_dizini = ozel.kok.join("kopru");
    std::fs::create_dir_all(&kopru_dizini).map_err(|e| {
        BaslatmaHatasi::Yalitim(format!(
            "Ağlı yalıtımlı komut başlatılamadı: köprü kopyalanamadı ({e})"
        ))
    })?;
    let kopru_izni_var = guvenlik::aceler_oku(&kopru_dizini)
        .map(|aceler| {
            aceler.iter().any(|a| {
                a.tur == 0 && a.maske & MASKE_OKU == MASKE_OKU && a.sid.as_ref() == Some(paket)
            })
        })
        .unwrap_or(false);
    if !kopru_izni_var {
        let izin = Izin {
            sid: paket.clone(),
            maske: MASKE_OKU,
            miras: MIRAS,
        };
        guvenlik::izin_ekle(&kopru_dizini, &izin).map_err(|e| {
            BaslatmaHatasi::Yalitim(format!(
                "Ağlı yalıtımlı komut başlatılamadı: köprü dizini izni verilemedi ({e})"
            ))
        })?;
    }
    let hedef_kopru = kopru_dizini.join("kalfa-kopru.exe");
    let ayni_mi = if hedef_kopru.is_file() {
        match (dosya_sha256(kaynak_kopru), dosya_sha256(&hedef_kopru)) {
            (Ok(h1), Ok(h2)) => h1 == h2,
            _ => false,
        }
    } else {
        false
    };
    if !ayni_mi {
        std::fs::copy(kaynak_kopru, &hedef_kopru).map_err(|e| {
            BaslatmaHatasi::Yalitim(format!(
                "Ağlı yalıtımlı komut başlatılamadı: köprü kopyalanamadı ({e})"
            ))
        })?;
    }

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
    let gelistirici = guvenlik::kullanici_sidi().map_err(yalitim_hatasi)?;
    let boru = match super::kopru::BoruSunucusu::ac(paket, &gelistirici, is, vekil.clone(), 64) {
        Ok(b) => b,
        Err(e) => {
            return Err(BaslatmaHatasi::Yalitim(format!(
                "Ağlı yalıtımlı komut başlatılamadı: boru açılamadı ({e})"
            )));
        }
    };

    let mut kap_ortami = ortam;
    kap_ortami.push((
        "KALFA_KOPRU_UYGULAMA".into(),
        komut.uygulama.display().to_string(),
    ));
    kap_ortami.push((
        "KALFA_KOPRU_SATIR".into(),
        String::from_utf16_lossy(&komut.satir),
    ));
    kap_ortami.push(("KALFA_KOPRU_BORU".into(), boru.ad().to_string()));
    let cmd = kap::sistem_dizini().join("cmd.exe");
    let kopru_komut = komut_satiri::uret(&hedef_kopru, &[], &cmd).map_err(yalitim_hatasi)?;
    let ag_kapatici: crate::surec::AgKapatici = Box::new(move || {
        Box::pin(async move {
            boru.kapat();
            vekil.bitir().await
        })
    });
    Ok((kopru_komut, kap_ortami, ag_kapatici))
}

fn sunucu_kaydi(alan: &str, sunucu: &str) -> String {
    format!("{alan}/mcp/{sunucu}")
}

fn sunucu_yetenek_adi(alan: &str, sunucu: &str) -> String {
    format!("kalfa-mcp-{alan}-{sunucu}")
}

/// Sunucunun özel kökü (1.1a §3, §5.6; G1.3). `mcp` ve sunucu kökü mirassız
/// (korumalı) DACL'le kurulur: yalnız geliştirici, SYSTEM ve Administrators.
/// Üst dizinlerden gelen paket ya da ortak yetenek izni sınırı aşamaz.
/// Kökte yalnız listeleme, `ev`/`gecici`/`onbellek`'te miraslı değiştirme
/// yalnız sunucunun yetenek SID'ine verilir; ACE'ler sunucu kaydına yazılır.
/// Var olan kök ilk değişiklikten önce denetlenir: mirası açıksa ya da başka
/// bir kimliğe izin veriyorsa başlatma reddedilir ve hiçbir şey değişmez.
fn sunucu_kokunu_hazirla(
    ace: &mut AceKaydi,
    yalitim_dizini: &Path,
    alan: &str,
    sunucu: &str,
    profil_adi: &str,
    yetenek: &Sid,
    git: &GitKimligi,
) -> io::Result<OzelDizinler> {
    let kayit = sunucu_kaydi(alan, sunucu);
    let (mcp, ozel) = sunucu_kokunu_kur(ace, yalitim_dizini, alan, sunucu, yetenek)?;
    ozel.hazirla(git)?;
    let listele = Izin {
        sid: yetenek.clone(),
        maske: MASKE_LISTELE,
        miras: MIRAS_YOK,
    };
    for yol in [&mcp, &ozel.kok] {
        izin_kaydet_ve_ver(ace, &kayit, yol, &listele)?;
    }
    let degistir = Izin {
        sid: yetenek.clone(),
        maske: MASKE_DEGISTIR,
        miras: MIRAS,
    };
    for yol in [&ozel.ev, &ozel.gecici, &ozel.onbellek] {
        izin_kaydet_ve_ver(ace, &kayit, yol, &degistir)?;
    }
    // Windows kabın TEMP'ini `LOCALAPPDATA\Packages\<paket>\AC\Temp`'e çevirir.
    profil::guvenli_dizin(
        &ozel.kok,
        Path::new("ev/Packages")
            .join(profil_adi)
            .join("AC")
            .join("Temp")
            .as_path(),
    )?;
    profil::guvenli_dizin(&ozel.kok, Path::new("ev/Roaming"))?;
    // Son denetim, araya giren bir değişikliği yakalar.
    sunucu_kokunu_denetle(ace, yalitim_dizini, alan, sunucu, yetenek)?;
    Ok(ozel)
}

/// `mcp` ve sunucunun kökü. Önce var olanlar denetlenir; sonra eksikler
/// mirassız ve yalnız geliştirici, SYSTEM ve Administrators'la oluşturulur.
fn sunucu_kokunu_kur(
    ace: &AceKaydi,
    yalitim_dizini: &Path,
    alan: &str,
    sunucu: &str,
    yetenek: &Sid,
) -> io::Result<(PathBuf, OzelDizinler)> {
    let (mcp, ozel) = sunucu_kokunu_denetle(ace, yalitim_dizini, alan, sunucu, yetenek)?;
    std::fs::create_dir_all(yalitim_dizini)?;
    profil::guvenli_dizin(yalitim_dizini, Path::new(alan))?;
    let temel = temel_izinler(true)?;
    for yol in [&mcp, &ozel.kok] {
        match guvenlik::korumali_dizin_olustur(yol, &temel) {
            // Var olan dizin yukarıda denetlendi.
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Ok((mcp, ozel))
}

/// Var olan `mcp`, sunucu kökü ve özel alt dizinleri değiştirmeden denetler.
/// Olmayan dizin geçer. Bağlantı, mirası açık kök ya da izinli kimlikler
/// dışında bir kimlik hatadır. `mcp`'de aynı alanın öbür sunucularının
/// kayıtlı listeleme izinleri de vardır.
fn sunucu_kokunu_denetle(
    ace: &AceKaydi,
    yalitim_dizini: &Path,
    alan: &str,
    sunucu: &str,
    yetenek: &Sid,
) -> io::Result<(PathBuf, OzelDizinler)> {
    let mcp = OzelDizinler::yol(yalitim_dizini, alan).kok.join("mcp");
    let ozel = crate::sunucu_ozel(yalitim_dizini, alan, sunucu);
    let mut izinli: Vec<Sid> = temel_izinler(true)?.into_iter().map(|i| i.sid).collect();
    izinli.push(yetenek.clone());
    mevcut_dizini_denetle(&ozel.kok, true, &izinli)?;
    for yol in [&ozel.ev, &ozel.gecici, &ozel.onbellek] {
        mevcut_dizini_denetle(yol, false, &izinli)?;
    }
    let onek = format!("{alan}/mcp/");
    for girdi in ace.girdiler() {
        if let Girdi::Izin { alan, yol, sid, .. } = girdi
            && alan.starts_with(&onek)
            && yol == &mcp
        {
            izinli.push(Sid::metinden(sid)?);
        }
    }
    mevcut_dizini_denetle(&mcp, true, &izinli)?;
    Ok((mcp, ozel))
}

fn mevcut_dizini_denetle(yol: &Path, korumali: bool, izinli: &[Sid]) -> io::Result<()> {
    match std::fs::symlink_metadata(yol) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
        Ok(_) => {}
    }
    guvenlik::yeniden_ayristirma_izlemeden_ac(yol, true)?;
    if korumali && !guvenlik::mirasi_kesik_mi(yol)? {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{}: sunucu kökünün mirası açık", yol.display()),
        ));
    }
    yalniz_izinli_kimlikler(yol, izinli)
}

/// Dizinin DACL'inde `izinli` dışında kimlik yoksa `Ok`.
fn yalniz_izinli_kimlikler(yol: &Path, izinli: &[Sid]) -> io::Result<()> {
    let fazla: Vec<String> = guvenlik::aceler_oku(yol)?
        .iter()
        .filter_map(|a| a.sid.as_ref())
        .filter(|sid| !izinli.contains(sid))
        .map(Sid::metin)
        .collect();
    if fazla.is_empty() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "{}: sunucu kökünde beklenmeyen izin: {}",
                yol.display(),
                fazla.join(", ")
            ),
        ))
    }
}

fn dosya_sha256(yol: &Path) -> io::Result<[u8; 32]> {
    use sha2::{Digest, Sha256};
    let mut dosya = std::fs::File::open(yol)?;
    let mut hasher = Sha256::new();
    let mut tampon = [0u8; 8192];
    loop {
        let n = std::io::Read::read(&mut dosya, &mut tampon)?;
        if n == 0 {
            break;
        }
        hasher.update(&tampon[..n]);
    }
    Ok(hasher.finalize().into())
}

fn hazirla(
    ace: &mut AceKaydi,
    ayar: &WindowsAyari,
    alan: &AlanBilgisi,
    kimlik: &str,
    ad: &str,
    ilerleme: &mut dyn FnMut(&str),
) -> io::Result<()> {
    let kok = guvenlik::sade(&std::fs::canonicalize(&alan.kok)?);
    let surucu = guvenlik::surucu_koku(&kok).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "ağ yolu çalışma alanı desteklenmiyor",
        )
    })?;
    if !guvenlik::sabit_diskler()
        .iter()
        .any(|s| ayni_yol(s, &surucu))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "çalışma alanı ACL destekleyen yerel sabit diskte değil",
        ));
    }
    let araclar = arac_yollarini_esitle(ace, ayar, Some(&kok))?;
    let paket = guvenlik::profil_sidi(ad)?;
    kaydet(
        ace,
        Girdi::Profil {
            alan: kimlik.into(),
            ad: ad.into(),
        },
    )?;
    let _ = guvenlik::profil_olustur(ad, "Kalfa çalışma alanı yalıtımı")?;

    let kok_izin = Izin {
        sid: paket.clone(),
        maske: MASKE_DEGISTIR,
        miras: MIRAS,
    };
    let ic_alan = ic_alan_koku_mu(ace, &kok, kimlik);
    // Önce kayıtlı en derin alt alanları ayır. Dış köke verilecek miraslı
    // ACE'ler, bu sınırlar korunmadan alt alanlara geçmemelidir.
    let mut alt_alanlar: Vec<(String, PathBuf)> = kayitli_alan_kokleri(ace)
        .into_iter()
        .filter(|(alan, alt_kok)| {
            alan != kimlik && altinda(alt_kok, &kok) && !ayni_yol(alt_kok, &kok)
        })
        .collect();
    alt_alanlar.sort_by_key(|(_, alt_kok)| std::cmp::Reverse(alt_kok.components().count()));
    for (alt_alan, alt_kok) in alt_alanlar {
        if alan_kimligi(&alt_kok) != alt_alan {
            continue;
        }
        match kanonik_varsa_baglantisiz(&alt_kok)? {
            Some(gercek) if ayni_yol(&gercek, &alt_kok) => {}
            Some(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "kayıtlı alt çalışma alanının yolu değişmiş",
                ));
            }
            None => continue,
        }
        let alt_paket = guvenlik::profil_sidi(&profil_adi(&alt_alan))?;
        let mut alt_izinler = temel_izinler(true)?;
        alt_izinler.push(Izin {
            sid: alt_paket.clone(),
            maske: MASKE_DEGISTIR,
            miras: MIRAS,
        });
        let onceki = match ace.ilk_dacl(&alt_kok) {
            Some(onceki) => onceki.to_string(),
            None => guvenlik::dacl_sddl(&alt_kok)?,
        };
        // Alt alan daha önce kendi kök ACE'sini verdiyse, bu ACE'yi geri
        // yüklenecek ilk DACL anlık görüntüsüne taşımayız. Geri alma önce
        // kayıtlı ACE'yi kaldırır, sonra gerçek önceki DACL'i geri yazar.
        let onceki = if ace.ilk_dacl(&alt_kok).is_some() {
            onceki
        } else {
            guvenlik::dacl_sddl_sidini_kaldir(&onceki, &alt_paket)?
        };
        // Bu sınır alt alanın yaşam döngüsüne aittir: dış alan kalkınca da
        // alt alan kendi özel DACL'iyle çalışmaya devam eder.
        kes_ve_kaydet_onceki(ace, &alt_alan, &alt_kok, &alt_izinler, &[], onceki)?;
    }
    if ic_alan {
        // Dış alanın OI|CI ACE'i bu köke kadar ulaşmıştır. Önce mevcut
        // DACL'i kaydet, sonra mirası kesip yalnız geliştirici, SYSTEM,
        // Administrators ve bu alanın paket SID'ine izin ver.
        let mut izinler = temel_izinler(true)?;
        izinler.push(kok_izin);
        kes_ve_kaydet(ace, kimlik, &kok, &izinler, &[])?;
    } else {
        izin_kaydet_ve_ver(ace, kimlik, &kok, &kok_izin)?;
    }

    let yetenek = guvenlik::arac_zinciri_sidi()?;
    ust_dizinlere_listeleme(ace, kimlik, &kok, &paket, &yetenek)?;

    let ozel = OzelDizinler::yol(&ayar.yalitim_dizini, kimlik);
    std::fs::create_dir_all(&ayar.yalitim_dizini)?;
    profil::guvenli_dizin(&ayar.yalitim_dizini, Path::new(kimlik))?;
    ozel.hazirla(&ayar.git)?;
    // Özel dizinlerdeki yollar da çözülür (cargo registry'yi kanonikleştirir).
    let ozel_kanonik = guvenlik::sade(&std::fs::canonicalize(&ozel.kok)?);
    ust_dizinlere_listeleme(ace, kimlik, &ozel_kanonik, &paket, &yetenek)?;
    // Özel kökte Kalfa'nın verdiği izin yalnızca kök adlarını listeletir;
    // kayıtsız aynı-SID ACE varsa hazırlık onu değiştirmeden başarısız olur.
    let ozel_kok_izin = Izin {
        sid: paket.clone(),
        maske: MASKE_LISTELE,
        miras: MIRAS_YOK,
    };
    izin_kaydet_ve_ver(ace, kimlik, &ozel.kok, &ozel_kok_izin)?;
    for yol in [&ozel.ev, &ozel.gecici, &ozel.onbellek] {
        let izin = Izin {
            sid: paket.clone(),
            maske: MASKE_DEGISTIR,
            miras: MIRAS,
        };
        izin_kaydet_ve_ver(ace, kimlik, yol, &izin)?;
    }
    let kopru_dizini = ozel.kok.join("kopru");
    std::fs::create_dir_all(&kopru_dizini)?;
    let kopru_izin = Izin {
        sid: paket.clone(),
        maske: MASKE_OKU,
        miras: MIRAS,
    };
    izin_kaydet_ve_ver(ace, kimlik, &kopru_dizini, &kopru_izin)?;
    profil::guvenli_dizin(
        &ozel.kok,
        Path::new("ev/Packages").join(ad).join("AC").as_path(),
    )?;
    profil::guvenli_dizin(&ozel.kok, Path::new("ev/Roaming"))?;

    for yol in araclar {
        match kanonik_varsa_baglantisiz(&yol)? {
            Some(gercek) if ayni_yol(&gercek, &yol) => {}
            Some(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "araç zinciri dizini hazırlanırken değişti",
                ));
            }
            None => continue,
        }
        let zaten_var = guvenlik::aceler_oku(&yol)?.iter().any(|a| {
            a.tur == 0 && a.maske & MASKE_OKU == MASKE_OKU && a.sid.as_ref() == Some(&yetenek)
        });
        if zaten_var {
            continue;
        }
        let izin = Izin {
            sid: yetenek.clone(),
            maske: MASKE_OKU,
            miras: MIRAS,
        };
        izin_kaydet_ve_ver(ace, ace_kaydi::ORTAK, &yol, &izin)?;
    }

    let mut korunan = alan.korumali.clone();
    korunan.extend(alan.git_dizinleri.iter().map(|y| Korumali {
        yol: y.clone(),
        dizin: y.is_dir(),
    }));
    korunan.sort_by(|a, b| a.yol.cmp(&b.yol));
    korunan.dedup_by(|a, b| ayni_yol(&a.yol, &b.yol));
    for k in &korunan {
        if !k.yol.exists() {
            continue;
        }
        ilerleme(&format!("İzinler hazırlanıyor: {}", k.yol.display()));
        let mut izinler = temel_izinler(k.dizin)?;
        izinler.push(Izin {
            sid: paket.clone(),
            maske: MASKE_OKU,
            miras: if k.dizin { MIRAS } else { MIRAS_YOK },
        });
        let kaydedilecek = izinler.clone();
        diger_paketleri_ekle(ace, kimlik, &k.yol, k.dizin, &mut izinler)?;
        kes_ve_kaydet(ace, kimlik, &k.yol, &izinler, &kaydedilecek)?;
    }

    for yol in alan.gizli.dizinler.iter().chain(&alan.gizli.dosyalar) {
        if korunan.iter().any(|k| altinda(yol, &k.yol)) {
            continue;
        }
        let dizin = std::fs::metadata(yol).map(|m| m.is_dir()).unwrap_or(false);
        ilerleme(&format!("Gizli yol korunuyor: {}", yol.display()));
        let izinler = temel_izinler(dizin)?;
        kes_ve_kaydet(ace, kimlik, yol, &izinler, &izinler)?;
    }

    kaydet(
        ace,
        Girdi::Hazir {
            alan: kimlik.into(),
            kok,
        },
    )?;
    Ok(())
}

/// A11: Git for Windows (`getcwd`), PowerShell (`Set-Location`) ve yol
/// kanonikleştiren araçlar (cargo) yolu çözerken üst dizinleri listeler.
/// Sürücü kökünün ve profiller dizininin (`C:\Users`) izni kurulumda
/// yeteneğe verilir; `yol`'un öteki her üst dizinine yalnız bu alanın
/// paketine, mirassız listeleme verilir. Kap bu dizinlerdeki adları görür,
/// içeriklerini göremez. `yol` kanoniktir.
fn ust_dizinlere_listeleme(
    ace: &mut AceKaydi,
    kimlik: &str,
    yol: &Path,
    paket: &Sid,
    yetenek: &Sid,
) -> io::Result<()> {
    let izin = Izin {
        sid: paket.clone(),
        maske: MASKE_LISTELE,
        miras: MIRAS_YOK,
    };
    let kilit = AdliKilit::kilitle(UST_DIZIN_KILIDI)?;
    // Sürücü kökü (üstü olmayan) atlanır; onun izni kurulumdadır.
    for ust in yol.ancestors().skip(1).filter(|u| u.parent().is_some()) {
        if guvenlik::kok_izni_var_mi(ust, yetenek)? {
            continue;
        }
        izin_kaydet_ve_ver(ace, kimlik, ust, &izin).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!(
                    "{}: üst dizine listeleme izni verilemedi ({e}). Çalışma alanını size ait bir dizine taşıyın.",
                    ust.display()
                ),
            )
        })?;
    }
    kilit.birak()
}

/// Kayıttaki tamamlanmış ve yarım kalmış çalışma alanlarının kökleri.
/// Hazırlıkta her alanın kök girdisi ilk yazılan yol girdisidir; böylece
/// büyük bir ağaçtaki her ACE yolunu tekrar kanonikleştirmek gerekmez.
fn kayitli_alan_kokleri(ace: &AceKaydi) -> Vec<(String, PathBuf)> {
    let mut kokler = ace.hazir_alanlar();
    for girdi in ace.girdiler() {
        let (alan, yol) = match girdi {
            Girdi::Izin { alan, yol, .. } | Girdi::MirasKesildi { alan, yol, .. }
                if alan != ace_kaydi::ORTAK =>
            {
                (alan, yol)
            }
            _ => continue,
        };
        if kokler.iter().any(|(bilinen, _)| bilinen == alan) || alan_kimligi(yol) != *alan {
            continue;
        }
        kokler.push((alan.clone(), guvenlik::sade(yol)));
    }
    kokler.sort_by(|(a, _), (b, _)| a.cmp(b));
    kokler.dedup_by(|(a_alan, a_yol), (b_alan, b_yol)| a_alan == b_alan && ayni_yol(a_yol, b_yol));
    kokler
}

/// Kök, başka bir hazırlanmış ya da yarım hazırlanmış alanın altındaysa
/// `true` döner. ACE kaydı hazırlık ortasında kesilse bile kök ACE'si bunu
/// tanımaya yeter.
fn ic_alan_koku_mu(ace: &AceKaydi, kok: &Path, kimlik: &str) -> bool {
    kayitli_alan_kokleri(ace)
        .into_iter()
        .any(|(alan, diger_kok)| {
            alan != kimlik && altinda(kok, &diger_kok) && !ayni_yol(kok, &diger_kok)
        })
}

/// İstenen araç yollarını fiziksel ve güvenli yollar olarak çözer, çakışan
/// dizinleri ortak capability listesinden çıkarır ve artık kullanılmayan
/// ACE'leri kayda göre geri alır.
fn arac_yollarini_esitle(
    ace: &mut AceKaydi,
    ayar: &WindowsAyari,
    ek_alan_koku: Option<&Path>,
) -> io::Result<Vec<PathBuf>> {
    match arac_yollarini_siniflandir(ace, ayar, ek_alan_koku) {
        Ok((araclar, uygun)) => {
            ortak_arac_izinlerini_esitle(ace, &araclar)?;
            if !uygun {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "araç zinciri dizini çalışma alanı ya da Kalfa'nın özel verisiyle çakışıyor veya bağlantı içeriyor",
                ));
            }
            Ok(araclar)
        }
        Err(hata) => {
            // Sınıflandırma başarısızsa eski ortak ACE'leri tutma. Kaldırma
            // başarısız olursa kayıt korunur ve çağıran yalıtımı kapatır.
            ortak_arac_izinlerini_esitle(ace, &[])?;
            Err(hata)
        }
    }
}

fn arac_yollarini_siniflandir(
    ace: &AceKaydi,
    ayar: &WindowsAyari,
    ek_alan_koku: Option<&Path>,
) -> io::Result<(Vec<PathBuf>, bool)> {
    if ace.bozuk_mu() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "ACE kaydı bozuk; ortak araç zinciri izinleri denetlenemedi",
        ));
    }

    let ozel_kok = kanonik_yeni_baglantisiz(&ayar.yalitim_dizini)?;
    let mut alan_kokleri: Vec<PathBuf> = kayitli_alan_kokleri(ace)
        .into_iter()
        .map(|(_, kok)| guvenlik::sade(&kok))
        .collect();
    if let Some(kok) = ek_alan_koku {
        alan_kokleri.push(guvenlik::sade(kok));
    }
    let sistem = sistem_yollari();
    let mut araclar: Vec<PathBuf> = Vec::new();
    let mut uygun = true;
    for yol in ayar.okunur() {
        let kanonik = match kanonik_varsa_baglantisiz(&yol) {
            Ok(Some(kanonik)) => kanonik,
            Ok(None) => continue,
            Err(hata) if hata.kind() == io::ErrorKind::InvalidInput => {
                uygun = false;
                continue;
            }
            Err(hata) => return Err(hata),
        };
        if sistem.iter().any(|kok| altinda(&kanonik, kok)) {
            continue;
        }
        let cakisma = sistem.iter().any(|kok| altinda(kok, &kanonik))
            || alan_kokleri
                .iter()
                .any(|kok| altinda(&kanonik, kok) || altinda(kok, &kanonik))
            || altinda(&kanonik, &ozel_kok)
            || altinda(&ozel_kok, &kanonik);
        if cakisma {
            uygun = false;
            continue;
        }
        if !araclar.iter().any(|onceki| ayni_yol(onceki, &kanonik)) {
            araclar.push(kanonik);
        }
    }
    Ok((araclar, uygun))
}

/// Kullanıcı izin listesinden çıkarılan araç zinciri yollarındaki ortak
/// capability ACE'lerini geri alır. Başarısız geri alma ya da kayıt yazımı
/// çağıranı kapalı duruma geçirir; eski kayıtlar hatadan sonra korunur.
fn ortak_arac_izinlerini_esitle(ace: &mut AceKaydi, okunur: &[PathBuf]) -> io::Result<()> {
    if ace.bozuk_mu() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "ACE kaydı bozuk; ortak araç zinciri izinleri denetlenemedi",
        ));
    }
    let ortak: Vec<Girdi> = ace
        .girdiler()
        .iter()
        .filter(|girdi| girdi.alan() == ace_kaydi::ORTAK)
        .cloned()
        .collect();
    if ortak.is_empty() {
        return Ok(());
    }
    let yetenek = guvenlik::arac_zinciri_sidi()?;
    let yetenek_metin = yetenek.metin();
    let hedefler = okunur;

    let mut kalan = Vec::new();
    let mut kaldirilacak = Vec::new();
    for girdi in ortak {
        let Girdi::Izin {
            sid,
            yol,
            maske,
            miras,
            ..
        } = &girdi
        else {
            kalan.push(girdi);
            continue;
        };
        if sid != &yetenek_metin || *maske != MASKE_OKU || *miras != MIRAS {
            kalan.push(girdi);
            continue;
        }
        let gercek_yol = kanonik_varsa_baglantisiz(yol)?;
        if gercek_yol
            .as_ref()
            .is_some_and(|gercek| !ayni_yol(gercek, yol))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ACE kaydındaki araç yolu kanonik değil ya da sonradan bağlantıya çevrilmiş",
            ));
        }
        let listede = gercek_yol
            .as_ref()
            .is_some_and(|gercek| hedefler.iter().any(|hedef| ayni_yol(gercek, hedef)));
        if listede {
            kalan.push(girdi);
        } else {
            kaldirilacak.push((yol.clone(), girdi));
        }
    }

    let mut ilk_hata = None;
    for (yol, _) in &kaldirilacak {
        match kanonik_varsa_baglantisiz(yol) {
            Ok(Some(gercek)) if ayni_yol(&gercek, yol) => {
                if let Err(hata) = guvenlik::kayitli_izni_kaldir(yol, &yetenek, MASKE_OKU, MIRAS) {
                    ilk_hata.get_or_insert(hata);
                }
            }
            Ok(None) => {}
            Ok(Some(_)) => {
                ilk_hata.get_or_insert_with(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "ACE kaydındaki araç yolu sonradan değişmiş",
                    )
                });
            }
            Err(hata) => {
                ilk_hata.get_or_insert(hata);
            }
        }
    }
    if let Some(hata) = ilk_hata {
        return Err(hata);
    }
    if !kaldirilacak.is_empty() {
        ace.alani_cikar(ace_kaydi::ORTAK, &kalan)?;
    }
    Ok(())
}

/// Yolu mutlaklaştırır ve mevcut bileşenleri bağlantı/junction olmadıkları
/// sürece kabul eder. Yol `canonicalize` edilmeden önce kaynak bileşenleri
/// denetlenir; böylece bir junction'ın hedefindeki dizine ACE verilmez.
fn mutlak_baglantisiz_yol(yol: &Path) -> io::Result<PathBuf> {
    use std::path::Component;

    let yol = if yol.is_absolute() {
        yol.to_path_buf()
    } else {
        std::env::current_dir()?.join(yol)
    };
    let mut mutlak = PathBuf::new();
    for bilesen in yol.components() {
        match bilesen {
            Component::Prefix(_) | Component::RootDir => mutlak.push(bilesen.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                mutlak.pop();
            }
            Component::Normal(_) => {
                mutlak.push(bilesen.as_os_str());
                match std::fs::symlink_metadata(&mutlak) {
                    Ok(metadata)
                        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 =>
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("bağlantı içeren yol kabul edilmedi: {}", mutlak.display()),
                        ));
                    }
                    Ok(_) => {}
                    Err(hata) if hata.kind() == io::ErrorKind::NotFound => {}
                    Err(hata) => return Err(hata),
                }
            }
        }
    }
    Ok(mutlak)
}

/// Varsa, yolun kendisini ve bütün üst bileşenlerini kanonik ve bağlantısız
/// olarak döndürür. Olmayan son bileşen `None` olur; bozuk junction reddedilir.
fn kanonik_varsa_baglantisiz(yol: &Path) -> io::Result<Option<PathBuf>> {
    let mutlak = mutlak_baglantisiz_yol(yol)?;
    match std::fs::canonicalize(&mutlak) {
        Ok(kanonik) => {
            let kanonik = guvenlik::sade(&kanonik);
            let kanonik = mutlak_baglantisiz_yol(&kanonik)?;
            Ok(Some(kanonik))
        }
        Err(hata) if hata.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(hata) => Err(hata),
    }
}

/// Var olan ya da henüz oluşturulmamış dizin yolunu çözer. Son bileşenler
/// yoksa en yakın var olan üst dizin kanonikleştirilip sona eklenir.
fn kanonik_yeni_baglantisiz(yol: &Path) -> io::Result<PathBuf> {
    let mut var_olan = mutlak_baglantisiz_yol(yol)?;
    let mut eksik = Vec::new();
    loop {
        match std::fs::canonicalize(&var_olan) {
            Ok(kanonik) => {
                if !std::fs::metadata(&var_olan)?.is_dir() && !eksik.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "özel yalıtım yolu bir dizin olmayan bileşenden geçiyor",
                    ));
                }
                let mut sonuc = guvenlik::sade(&kanonik);
                for bilesen in eksik.iter().rev() {
                    sonuc.push(bilesen);
                }
                return Ok(sonuc);
            }
            Err(hata) if hata.kind() == io::ErrorKind::NotFound => {
                let Some(bilesen) = var_olan.file_name() else {
                    return Err(hata);
                };
                eksik.push(bilesen.to_os_string());
                if !var_olan.pop() {
                    return Err(hata);
                }
            }
            Err(hata) => return Err(hata),
        }
    }
}

fn temel_izinler(dizin: bool) -> io::Result<Vec<Izin>> {
    let miras = if dizin { MIRAS } else { MIRAS_YOK };
    Ok([
        guvenlik::kullanici_sidi()?,
        Sid::metinden(SISTEM)?,
        Sid::metinden(YONETICILER)?,
    ]
    .into_iter()
    .map(|sid| Izin {
        sid,
        maske: MASKE_TAM,
        miras,
    })
    .collect())
}

fn kes_ve_kaydet(
    ace: &mut AceKaydi,
    kimlik: &str,
    yol: &Path,
    izinler: &[Izin],
    kaydedilecek: &[Izin],
) -> io::Result<()> {
    let onceki = ace
        .ilk_dacl(yol)
        .map(str::to_string)
        .map(Ok)
        .unwrap_or_else(|| guvenlik::dacl_sddl(yol))?;
    kes_ve_kaydet_onceki(ace, kimlik, yol, izinler, kaydedilecek, onceki)
}

fn kes_ve_kaydet_onceki(
    ace: &mut AceKaydi,
    kimlik: &str,
    yol: &Path,
    izinler: &[Izin],
    kaydedilecek: &[Izin],
    onceki: String,
) -> io::Result<()> {
    kaydet(
        ace,
        Girdi::MirasKesildi {
            alan: kimlik.into(),
            yol: yol.to_path_buf(),
            onceki,
        },
    )?;
    for izin in kaydedilecek {
        izin_kaydet(ace, kimlik, yol, &izin.sid, izin.maske, izin.miras)?;
    }
    guvenlik::mirasi_kes(yol, izinler)
}

fn izin_kaydet_ve_ver(ace: &mut AceKaydi, kimlik: &str, yol: &Path, izin: &Izin) -> io::Result<()> {
    let sid = izin.sid.metin();
    let kayitli = ace.girdiler().iter().any(|girdi| {
        matches!(girdi, Girdi::Izin { alan, yol: kayit_yolu, sid: kayit_sid, maske, miras }
            if alan == kimlik
                && kayit_yolu == yol
                && kayit_sid == &sid
                && *maske == izin.maske
                && *miras == izin.miras)
    });
    let mevcut: Vec<guvenlik::Ace> = guvenlik::aceler_oku(yol)?
        .into_iter()
        .filter(|ace| ace.sid.as_ref() == Some(&izin.sid))
        .collect();
    let ayni_izin_sayisi = mevcut
        .iter()
        .filter(|ace| {
            ace.tur == 0
                && !ace.miras_mi()
                && ace.maske == izin.maske
                && u32::from(ace.bayraklar) & 0x0F == izin.miras
        })
        .count();
    if kayitli && ayni_izin_sayisi == 1 {
        return Ok(());
    }
    if ayni_izin_sayisi > 1 || !mevcut.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "aynı SID için kayıtsız ya da belirsiz ACL girdisi var; izin birleştirilmedi",
        ));
    }

    izin_kaydet(ace, kimlik, yol, &izin.sid, izin.maske, izin.miras)?;
    guvenlik::izin_ekle(yol, izin)
}

fn izin_kaydet(
    ace: &mut AceKaydi,
    kimlik: &str,
    yol: &Path,
    sid: &Sid,
    maske: u32,
    miras: u32,
) -> io::Result<()> {
    kaydet(
        ace,
        Girdi::Izin {
            alan: kimlik.into(),
            yol: yol.to_path_buf(),
            sid: sid.metin(),
            maske,
            miras,
        },
    )
}

fn kaydet(ace: &mut AceKaydi, girdi: Girdi) -> io::Result<()> {
    ace.ekle(girdi)
}

fn diger_paketleri_ekle(
    ace: &AceKaydi,
    kimlik: &str,
    yol: &Path,
    dizin: bool,
    izinler: &mut Vec<Izin>,
) -> io::Result<()> {
    for metin in ace.baska_alanlarin_sidleri(kimlik, yol) {
        let sid = Sid::metinden(&metin)?;
        if izinler.iter().any(|i| i.sid == sid) {
            continue;
        }
        izinler.push(Izin {
            sid,
            maske: MASKE_OKU,
            miras: if dizin { MIRAS } else { MIRAS_YOK },
        });
    }
    Ok(())
}

fn kayda_gore_geri_al(ace: &mut AceKaydi, kimlik: &str) -> io::Result<()> {
    let _kilit = AdliKilit::kilitle(UST_DIZIN_KILIDI)?;
    let girdiler = ace.girdiler().to_vec();
    let alan_girdileri: Vec<Girdi> = girdiler
        .iter()
        .filter(|g| g.alan() == kimlik)
        .cloned()
        .collect();
    for girdi in &alan_girdileri {
        if let Girdi::Izin {
            alan,
            yol,
            sid,
            maske,
            miras,
        } = girdi
        {
            if diger_izin_var(&girdiler, alan, yol, sid) || !yol.exists() {
                continue;
            }
            let sid = Sid::metinden(sid)?;
            guvenlik::kayitli_izni_kaldir(yol, &sid, *maske, *miras)?;
        }
    }
    for girdi in &alan_girdileri {
        if let Girdi::MirasKesildi { yol, .. } = girdi {
            let ic_alan_siniri = kaldirilan_ic_alan_siniri(&girdiler, kimlik, yol);
            if (diger_miras_var(&girdiler, kimlik, yol) && !ic_alan_siniri) || !yol.exists() {
                continue;
            }
            if let Some(onceki) = ace.ilk_dacl(yol) {
                guvenlik::dacl_geri_yaz(yol, onceki)?;
            }
        }
    }
    Ok(())
}

fn diger_izin_var(girdiler: &[Girdi], kimlik: &str, yol: &Path, sid: &str) -> bool {
    girdiler.iter().any(|g| {
        matches!(g, Girdi::Izin { alan, yol: y, sid: s, .. }
            if alan != kimlik && y == yol && s == sid)
    })
}

fn diger_miras_var(girdiler: &[Girdi], kimlik: &str, yol: &Path) -> bool {
    girdiler.iter().any(|g| {
        matches!(g, Girdi::MirasKesildi { alan, yol: y, .. }
            if alan != kimlik && y == yol)
    })
}

/// İç alan kaldırılırken etkin dış alanın aynı kökteki sınır kaydı kalır.
/// İç alanın önceki DACL'ini şimdiden yüklemek, kökün dış alanın devralınan
/// iznine dönmesini sağlar; dış alanın kaydı sonradan aynı başlangıç DACL'ini
/// yeniden yükleyebilir. Ortak git yolları alan kökü olmadığından bu dala girmez.
fn kaldirilan_ic_alan_siniri(girdiler: &[Girdi], kimlik: &str, yol: &Path) -> bool {
    let Some(kok) = girdiler.iter().find_map(|girdi| match girdi {
        Girdi::Hazir { alan, kok } if alan == kimlik => Some(kok.as_path()),
        _ => None,
    }) else {
        return false;
    };
    ayni_yol(kok, yol)
        && girdiler.iter().any(|girdi| match girdi {
            Girdi::Hazir {
                alan: dis_alan,
                kok: dis_kok,
            } => dis_alan != kimlik && altinda(kok, dis_kok) && !ayni_yol(kok, dis_kok),
            _ => false,
        })
}

fn taramali_geri_al(ace: &mut AceKaydi, kok: &Path, kimlik: &str, paket: &Sid) -> io::Result<()> {
    let _kilit = AdliKilit::kilitle(UST_DIZIN_KILIDI)?;
    let girdiler = ace.girdiler().to_vec();
    let mut ilk_hata = None;
    let paket_metin = paket.metin();
    for girdi in &girdiler {
        let Girdi::Izin {
            alan: kayit_alani,
            yol,
            sid,
            maske,
            miras,
        } = girdi
        else {
            continue;
        };
        if kayit_alani != kimlik
            || sid != &paket_metin
            || diger_izin_var(&girdiler, kimlik, yol, sid)
            || !yol.exists()
        {
            continue;
        }
        match kanonik_varsa_baglantisiz(yol) {
            Ok(Some(gercek)) if ayni_yol(&gercek, yol) => {}
            Ok(None) => continue,
            Ok(Some(_)) => {
                ilk_hata.get_or_insert_with(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "ACE kaydındaki yol artık aynı nesneye işaret etmiyor",
                    )
                });
                continue;
            }
            Err(e) => {
                ilk_hata.get_or_insert(e);
                continue;
            }
        }
        if let Err(e) = guvenlik::kayitli_izni_kaldir(yol, paket, *maske, *miras) {
            ilk_hata.get_or_insert(e);
        }
    }

    // Bozuk kayıtta yalnızca DACL'i kayda geçmiş ve çalışma alanının içinde
    // kalan nesnelerin önceki hâli kanıtlanabilir. Kayıt dışı yolların ve
    // başka alanlarla paylaşılabilen dış git dizinlerinin mirası tahmin edilmez.
    let mut geri_yazilan: Vec<PathBuf> = Vec::new();
    for girdi in &girdiler {
        let Girdi::MirasKesildi {
            alan: kayit_alani,
            yol,
            onceki,
        } = girdi
        else {
            continue;
        };
        if kayit_alani != kimlik
            || !altinda(yol, kok)
            || diger_miras_var(&girdiler, kimlik, yol)
            || geri_yazilan.iter().any(|y| ayni_yol(y, yol))
        {
            continue;
        }
        if !yol.exists() {
            continue;
        }
        if let Err(e) = guvenlik::dacl_geri_yaz(yol, onceki) {
            ilk_hata.get_or_insert(e);
        } else {
            geri_yazilan.push(yol.clone());
        }
    }

    match ilk_hata {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

fn yalitim_ortami(
    kaynak: &[(String, String)],
    ozel: &OzelDizinler,
    ayar: &WindowsAyari,
    araclar: &[PathBuf],
    ag: bool,
) -> Vec<(String, String)> {
    // PowerShell'in modül önbelleği kapta çalışma alanına özel
    // `LOCALAPPDATA`'da kalır; geliştiricinin paylaşılan önbellek dosyası
    // kaba gösterilmez.
    let mut ortam: Vec<(String, String)> = kaynak
        .iter()
        .filter(|(ad, _)| {
            crate::ortam::izinli_mi(ad) && !ad.eq_ignore_ascii_case("PSModuleAnalysisCachePath")
        })
        .cloned()
        .collect();
    let system_root = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let windows = system_root.display().to_string();
    deger_ata(&mut ortam, "SystemRoot", &windows);
    deger_ata(&mut ortam, "windir", &windows);
    deger_ata(
        &mut ortam,
        "COMSPEC",
        &kap::sistem_dizini().join("cmd.exe").display().to_string(),
    );

    let mut kokler = sistem_yollari();
    kokler.extend(araclar.iter().cloned());
    kokler.extend(ayar.kalfa_araclari.iter().cloned());
    kokler.extend([ozel.kok.clone(), ozel.ev.clone()]);
    let mevcut_yol = deger_al(&ortam, "PATH").unwrap_or_default();
    let mut yollar: Vec<PathBuf> = std::env::split_paths(&OsString::from(mevcut_yol))
        .filter(|y| y.is_dir() && kokler.iter().any(|k| altinda(y, k)))
        .collect();
    yollar.extend([kap::sistem_dizini(), system_root.clone()]);
    if let Ok(Some(cargo_bin)) = kanonik_varsa_baglantisiz(&ayar.ev.join(r".cargo\bin"))
        && araclar.iter().any(|y| ayni_yol(y, &cargo_bin))
    {
        yollar.push(cargo_bin);
    }
    yollar.retain(|y| y.is_dir() && kokler.iter().any(|k| altinda(y, k)));
    yollar.sort();
    yollar.dedup_by(|a, b| ayni_yol(a, b));
    // Kalfa'nın araçları, kullanıcının bilgisayarındakilerden (ör.
    // `C:\Program Files\nodejs`) önce ve verilen sırayla gelir. pip'in
    // kullanıcı kurulumunun betikleri henüz yoksa da yerini alır.
    let mut once: Vec<PathBuf> = ayar
        .kalfa_araclari
        .iter()
        .filter(|y| y.is_dir())
        .cloned()
        .collect();
    if let Some(surum) = python_surumu(&once) {
        once.push(
            ozel.ev
                .join("Roaming")
                .join("Python")
                .join(format!("Python{surum}"))
                .join("Scripts"),
        );
    }
    yollar.retain(|y| !once.iter().any(|o| ayni_yol(o, y)));
    once.extend(yollar);
    if let Ok(path) = std::env::join_paths(once) {
        deger_ata(&mut ortam, "PATH", &path.to_string_lossy());
    }

    deger_ata(&mut ortam, "HOME", &ozel.ev.display().to_string());
    deger_ata(&mut ortam, "USERPROFILE", &ozel.ev.display().to_string());
    deger_ata(
        &mut ortam,
        "APPDATA",
        &ozel.ev.join("Roaming").display().to_string(),
    );
    deger_ata(&mut ortam, "LOCALAPPDATA", &ozel.ev.display().to_string());
    deger_ata(&mut ortam, "TEMP", &ozel.gecici.display().to_string());
    deger_ata(&mut ortam, "TMP", &ozel.gecici.display().to_string());
    // A15'in gelişmiş yolu (2d-3): kapta crates.io paketi açılamaz. Ağsız
    // komut, geliştiricinin `cargo` ve `registry`'si okunur dizinlerdeyse
    // onun `.cargo`'suyla ve çevrimdışı derler; kap oraya yazamaz, kökteki
    // kimlik dosyalarını okuyamaz. Ağlı komutta özel, boş registry kalır.
    let okunur_mu = |alt: &str| {
        kanonik_varsa_baglantisiz(&ayar.ev.join(alt))
            .ok()
            .flatten()
            .is_some_and(|y| araclar.iter().any(|a| ayni_yol(a, &y)))
    };
    let gelistirici_cargo = (!ag && okunur_mu(r".cargo\bin") && okunur_mu(r".cargo\registry"))
        .then(|| {
            kanonik_varsa_baglantisiz(&ayar.ev.join(".cargo"))
                .ok()
                .flatten()
        })
        .flatten();
    match gelistirici_cargo {
        Some(cargo) => {
            deger_ata(&mut ortam, "CARGO_HOME", &cargo.display().to_string());
            deger_ata(&mut ortam, "CARGO_NET_OFFLINE", "true");
        }
        None => deger_ata(
            &mut ortam,
            "CARGO_HOME",
            &ozel.cargo().display().to_string(),
        ),
    }
    deger_ata(
        &mut ortam,
        "GIT_CONFIG_GLOBAL",
        &ozel.gitconfig().display().to_string(),
    );
    deger_ata(
        &mut ortam,
        "NPM_CONFIG_USERCONFIG",
        &ozel.npmrc().display().to_string(),
    );
    deger_ata(
        &mut ortam,
        "NPM_CONFIG_CACHE",
        &ozel.npm().display().to_string(),
    );
    deger_ata(
        &mut ortam,
        "PIP_CACHE_DIR",
        &ozel.pip().display().to_string(),
    );
    if let Ok(Some(rustup)) = kanonik_varsa_baglantisiz(&ayar.ev.join(".rustup"))
        && araclar.iter().any(|y| ayni_yol(y, &rustup))
    {
        deger_ata(&mut ortam, "RUSTUP_HOME", &rustup.display().to_string());
    }
    for (ad, deger) in crate::ortam::SABIT {
        deger_ata(&mut ortam, ad, deger);
    }
    MsvcOrtami::bul().uygula(&mut ortam);
    ortam
}

/// `Yalitici::okunur_yonetici_gerekli`: dizini ne araç zinciri yeteneği ne de
/// bütün uygulama paketleri okuyabiliyor ve geçerli kullanıcı DACL'ini
/// yazamıyor. Okunamayan DACL de yönetici ister; yükseltilmiş yardımcı dizini
/// `izin-ver`'in denetimleriyle yeniden doğrular.
fn okunur_yonetici_gerekli(dizin: &Path) -> bool {
    let sidler = match (
        guvenlik::arac_zinciri_sidi(),
        Sid::metinden(guvenlik::TUM_UYGULAMA_PAKETLERI),
    ) {
        (Ok(yetenek), Ok(paketler)) => vec![yetenek, paketler],
        _ => return true,
    };
    if guvenlik::okuma_izni_var_mi(dizin, &sidler).unwrap_or(false) {
        return false;
    }
    !guvenlik::dacl_yazilabilir_mi(dizin).unwrap_or(false)
}

/// Kalfa'nın Python'unun kısa sürümü (`python314.dll` → `314`): pip'in
/// kullanıcı kurulumu `%APPDATA%\Python\Python314\Scripts`'e iner.
fn python_surumu(dizinler: &[PathBuf]) -> Option<String> {
    dizinler.iter().find_map(|dizin| {
        std::fs::read_dir(dizin).ok()?.flatten().find_map(|girdi| {
            let ad = girdi.file_name().to_string_lossy().to_ascii_lowercase();
            let surum = ad.strip_prefix("python")?.strip_suffix(".dll")?;
            (surum.len() >= 3 && surum.bytes().all(|b| b.is_ascii_digit()))
                .then(|| surum.to_string())
        })
    })
}

fn deger_ata(ortam: &mut Vec<(String, String)>, ad: &str, deger: &str) {
    ortam.retain(|(a, _)| !a.eq_ignore_ascii_case(ad));
    ortam.push((ad.to_string(), deger.to_string()));
}

fn deger_al(ortam: &[(String, String)], ad: &str) -> Option<String> {
    ortam
        .iter()
        .find(|(a, _)| a.eq_ignore_ascii_case(ad))
        .map(|(_, d)| d.clone())
}

fn gorunen_program(
    program: &Path,
    kokler: &[PathBuf],
    alan: &AlanBilgisi,
    ortam: &[(String, String)],
) -> Result<PathBuf, BaslatmaHatasi> {
    let gorunen = program
        .canonicalize()
        .map(|p| guvenlik::sade(&p))
        .map_err(|e| BaslatmaHatasi::Yalitim(format!("programın gerçek yolu çözülemedi: {e}")))?;
    let gizli = alan.gizli.dosyalar.iter().any(|g| ayni_yol(g, &gorunen))
        || alan.gizli.dizinler.iter().any(|g| altinda(&gorunen, g));
    if !gizli && kokler.iter().any(|k| altinda(&gorunen, k)) {
        return Ok(gorunen);
    }
    Err(BaslatmaHatasi::ProgramYok {
        program: program.display().to_string(),
        path: deger_al(ortam, "PATH").unwrap_or_default(),
    })
}

fn izinli_kokler(
    ayar: &WindowsAyari,
    alan: &AlanBilgisi,
    ozel: &OzelDizinler,
    araclar: &[PathBuf],
) -> Vec<PathBuf> {
    let mut kokler = sistem_yollari();
    kokler.extend(araclar.iter().cloned());
    kokler.extend(ayar.kalfa_araclari.iter().cloned());
    kokler.push(alan.kok.clone());
    kokler.push(ozel.kok.clone());
    kokler.extend(alan.git_dizinleri.iter().cloned());
    kokler
}

fn sistem_yollari() -> Vec<PathBuf> {
    let mut y = Vec::new();
    for ad in [
        "SystemRoot",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramW6432",
        "CommonProgramFiles",
        "CommonProgramFiles(x86)",
        "CommonProgramW6432",
    ] {
        if let Some(p) = std::env::var_os(ad)
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
        {
            y.push(p);
        }
    }
    y.push(kap::sistem_dizini());
    y.sort();
    y.dedup_by(|a, b| ayni_yol(a, b));
    y
}

fn profil_adi(kimlik: &str) -> String {
    format!("{PROFIL_ON_EKI}{kimlik}")
}

fn yalitim_hatasi(hata: io::Error) -> BaslatmaHatasi {
    BaslatmaHatasi::Yalitim(hata.to_string())
}

fn ayni_yol(a: &Path, b: &Path) -> bool {
    normal_yol(a).eq_ignore_ascii_case(&normal_yol(b))
}

fn altinda(yol: &Path, kok: &Path) -> bool {
    let yol = normal_yol(yol);
    let kok = normal_yol(kok);
    yol.eq_ignore_ascii_case(&kok)
        || yol
            .strip_prefix(&kok)
            .is_some_and(|kalan| kalan.starts_with('\\') || kalan.starts_with('/'))
}

fn normal_yol(yol: &Path) -> String {
    let yol = guvenlik::sade(yol);
    yol.to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_string()
}

/// ACE kaydındaki kökün kimliği. Kimlik hazırlıkta kanonik (`\\?\…`) yoldan
/// hesaplanır, kayıt ise kökü `sade` biçimde saklar. Kök silinmiş ya da
/// taşınmışsa kanonikleştirme başarısız olur; kimlik o zaman `sade`'nin
/// kaldırdığı önek geri eklenerek bulunur. Yoksa silinen çalışma alanının
/// izinleri Ayarlar'dan hiç kaldırılamazdı.
fn kayitli_kok_kimligi(kok: &Path) -> String {
    if std::fs::canonicalize(kok).is_ok() {
        return alan_kimligi(kok);
    }
    let metin = kok.to_string_lossy();
    let uzun = if metin.starts_with(r"\\?\") {
        kok.to_path_buf()
    } else if let Some(kalan) = metin.strip_prefix(r"\\") {
        PathBuf::from(format!(r"\\?\UNC\{kalan}"))
    } else {
        PathBuf::from(format!(r"\\?\{metin}"))
    };
    alan_kimligi(&uzun)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    /// BT için ayrıntı düşen denetimi taşır; kullanıcıya giden talimattan
    /// ayrıdır (§9.8).
    #[test]
    fn yalitim_yokken_ayrinti_denetimi_ve_nedeni_tasir() {
        for (durum, beklenen) in [
            (
                kurulum_gerekli("kok-izni", r"C:\ sürücüsünün kök izni eksik."),
                YalitimYoklukNedeni::KurulumGerekli,
            ),
            (
                kurulum_bozuk("appcontainer-deneme", "AppContainer denemesi başarısız: x"),
                YalitimYoklukNedeni::KurulumBozuk,
            ),
        ] {
            let YalitimDurumu::Yok {
                neden,
                talimat,
                ayrinti,
            } = durum
            else {
                panic!("{durum:?}");
            };
            assert_eq!(neden, beklenen);
            assert!(ayrinti.starts_with("denetim="), "{ayrinti}");
            assert!(!talimat.contains("denetim="), "{talimat}");
        }
        let YalitimDurumu::Yok { ayrinti, .. } =
            kurulum_gerekli("kok-izni", r"C:\ sürücüsünün kök izni eksik.")
        else {
            unreachable!()
        };
        assert_eq!(
            ayrinti,
            r"denetim=kok-izni; C:\ sürücüsünün kök izni eksik."
        );
    }

    /// Kalfa'nın getirdiği araçlar kabın `PATH`'inde, kullanıcının
    /// bilgisayarındaki araçlardan (ör. `C:\Program Files\nodejs`) önce ve
    /// verilen sırada gelir; pip'in kullanıcı kurulumu da `PATH`'tedir.
    #[test]
    fn kalfa_araclari_path_in_basinda() {
        let d = tempfile::tempdir().unwrap();
        let kok = guvenlik::sade(&d.path().canonicalize().unwrap());
        let kalfa_python = kok.join(r"z\python");
        let kalfa_node = kok.join(r"b\node");
        let kullanici_node = kok.join(r"a\nodejs");
        for dizin in [&kalfa_python, &kalfa_node, &kullanici_node] {
            std::fs::create_dir_all(dizin).unwrap();
        }
        std::fs::write(kalfa_python.join("python3.dll"), b"").unwrap();
        std::fs::write(kalfa_python.join("python314.dll"), b"").unwrap();
        let ayar = WindowsAyari {
            ev: kok.join("ev"),
            yalitim_dizini: kok.join("yalitim"),
            ek_okunur: vec![kullanici_node.clone()],
            kalfa_araclari: vec![kalfa_python.clone(), kalfa_node.clone()],
            git: GitKimligi::default(),
            ..WindowsAyari::default()
        };
        let ozel = OzelDizinler::yol(&ayar.yalitim_dizini, "deneme");
        let kaynak = vec![(
            "PATH".to_string(),
            std::env::join_paths([&kullanici_node, &kalfa_node])
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        )];

        let ortam = yalitim_ortami(
            &kaynak,
            &ozel,
            &ayar,
            std::slice::from_ref(&kullanici_node),
            false,
        );
        let path = deger_al(&ortam, "PATH").unwrap();
        let yollar: Vec<PathBuf> = std::env::split_paths(&path).collect();
        // MSVC ortamı `bin`'ini en başa koyar (Git'in `link.exe`'sinden
        // önce); orada `python` ya da `node` yoktur.
        let p = yollar
            .iter()
            .position(|y| y == &kalfa_python)
            .unwrap_or_else(|| panic!("{path}"));
        assert!(
            yollar[..p]
                .iter()
                .all(|y| !y.join("python.exe").exists() && !y.join("node.exe").exists()),
            "{path}"
        );
        assert_eq!(yollar[p + 1], kalfa_node, "{path}");
        assert_eq!(
            yollar[p + 2],
            ozel.ev.join(r"Roaming\Python\Python314\Scripts"),
            "{path}"
        );
        let kullanici = yollar.iter().position(|y| y == &kullanici_node);
        assert!(kullanici.is_some_and(|i| i > p + 2), "{path}");
        assert_eq!(
            yollar.iter().filter(|y| *y == &kalfa_node).count(),
            1,
            "Kalfa'nın dizini yinelenmemeli: {path}"
        );
    }

    /// Geliştiricinin kendi dizini ve bütün uygulama paketlerinin okuduğu
    /// `System32` yönetici istemez; ikisini de karşılamayan, yöneticiye ait
    /// `System32\Tasks` ister (2d-3, Görev 5).
    #[test]
    fn okunur_yonetici_gerekli_kendi_dizininde_false() {
        let d = tempfile::tempdir().unwrap();
        assert!(!okunur_yonetici_gerekli(d.path()));
        let windows = std::env::var_os("SystemRoot")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
        assert!(!okunur_yonetici_gerekli(&windows.join("System32")));
        assert!(okunur_yonetici_gerekli(&windows.join(r"System32\Tasks")));
    }

    /// Kalfa'nın araçları yoksa `PATH` bugünkü gibidir; pip'in kullanıcı
    /// dizini de eklenmez.
    #[test]
    fn kalfa_araclari_yoksa_pip_kullanici_dizini_eklenmez() {
        let d = tempfile::tempdir().unwrap();
        let kok = guvenlik::sade(&d.path().canonicalize().unwrap());
        let ayar = WindowsAyari {
            ev: kok.join("ev"),
            yalitim_dizini: kok.join("yalitim"),
            ek_okunur: Vec::new(),
            kalfa_araclari: Vec::new(),
            git: GitKimligi::default(),
            ..WindowsAyari::default()
        };
        let ozel = OzelDizinler::yol(&ayar.yalitim_dizini, "deneme");
        let ortam = yalitim_ortami(&[], &ozel, &ayar, &[], false);
        let path = deger_al(&ortam, "PATH").unwrap();
        assert!(!path.contains("Python"), "{path}");
    }

    /// Yalıtımsız komuta geçen PowerShell modül önbelleği yolu kaba geçmez;
    /// kapta önbellek çalışma alanına özel `LOCALAPPDATA`'dadır.
    #[test]
    fn powershell_modul_onbellegi_kaba_gecmez() {
        let d = tempfile::tempdir().unwrap();
        let kok = guvenlik::sade(&d.path().canonicalize().unwrap());
        let ayar = WindowsAyari {
            ev: kok.join("ev"),
            yalitim_dizini: kok.join("yalitim"),
            ek_okunur: Vec::new(),
            kalfa_araclari: Vec::new(),
            git: GitKimligi::default(),
            ..WindowsAyari::default()
        };
        let ozel = OzelDizinler::yol(&ayar.yalitim_dizini, "deneme");
        let kaynak = vec![(
            "PSModuleAnalysisCachePath".to_string(),
            r"C:\PSModuleAnalysisCachePath\ModuleAnalysisCache".to_string(),
        )];
        let ortam = yalitim_ortami(&kaynak, &ozel, &ayar, &[], false);
        assert_eq!(deger_al(&ortam, "PSModuleAnalysisCachePath"), None);
        assert_eq!(
            deger_al(&ortam, "LOCALAPPDATA"),
            Some(ozel.ev.display().to_string())
        );
    }

    #[test]
    fn deneme_profili_mutexu_eszamanli_tutulmasini_engeller() {
        let ilk = AdliKilit::kilitle(DENEME_PROFILI_KILIDI).unwrap();
        let (basladi_gonder, basladi_al) = mpsc::channel();
        let (kilitlendi_gonder, kilitlendi_al) = mpsc::channel();
        let is = thread::spawn(move || {
            basladi_gonder.send(()).unwrap();
            let ikinci = AdliKilit::kilitle(DENEME_PROFILI_KILIDI).unwrap();
            kilitlendi_gonder.send(()).unwrap();
            ikinci.birak().unwrap();
        });
        basladi_al.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(
            kilitlendi_al
                .recv_timeout(Duration::from_millis(50))
                .is_err()
        );

        ilk.birak().unwrap();
        kilitlendi_al.recv_timeout(Duration::from_secs(2)).unwrap();
        is.join().unwrap();
    }

    #[test]
    fn kayitsiz_ayni_sid_ace_hazirlama_ve_geri_alma_sirasinda_korunur() {
        let gecici = tempfile::tempdir().unwrap();
        let yol = gecici.path().join("paylasilan");
        std::fs::create_dir(&yol).unwrap();
        let sid = Sid::metinden("S-1-15-2-98765").unwrap();
        let kullanici_izi = Izin {
            sid: sid.clone(),
            maske: MASKE_OKU,
            miras: MIRAS_YOK,
        };
        let kalfa_izi = Izin {
            sid: sid.clone(),
            maske: MASKE_LISTELE,
            miras: MIRAS_YOK,
        };
        guvenlik::izin_ekle(&yol, &kullanici_izi).unwrap();
        let onceki = guvenlik::aceler_oku(&yol).unwrap();
        let mut ace = AceKaydi::oku(&gecici.path().join("ace-kaydi.jsonl"));

        let hata = izin_kaydet_ve_ver(&mut ace, "deneme", &yol, &kalfa_izi)
            .expect_err("kayıtsız aynı-SID ACE üzerine izin verilmemeli");
        assert_eq!(hata.kind(), io::ErrorKind::InvalidData);
        assert!(ace.girdiler().is_empty());
        assert_eq!(guvenlik::aceler_oku(&yol).unwrap(), onceki);

        guvenlik::dacl_geri_yaz(
            &yol,
            &format!(
                "D:P(A;;0x{:08X};;;{})(A;;0x{:08X};;;{})",
                kullanici_izi.maske,
                sid.metin(),
                kalfa_izi.maske,
                sid.metin()
            ),
        )
        .unwrap();
        assert_eq!(guvenlik::aceler_oku(&yol).unwrap().len(), 2);
        ace.ekle(Girdi::Izin {
            alan: "deneme".into(),
            yol: yol.clone(),
            sid: sid.metin(),
            maske: kalfa_izi.maske,
            miras: kalfa_izi.miras,
        })
        .unwrap();
        izin_kaydet_ve_ver(&mut ace, "deneme", &yol, &kalfa_izi)
            .expect("kayıtlı aynı izin yinelenmemeli ama başarılı olmalı");

        kayda_gore_geri_al(&mut ace, "deneme").unwrap();
        let kalan = guvenlik::aceler_oku(&yol).unwrap();
        assert!(kalan.iter().any(|ace| {
            ace.tur == 0
                && ace.sid.as_ref() == Some(&sid)
                && ace.maske == kullanici_izi.maske
                && u32::from(ace.bayraklar) & 0x0F == kullanici_izi.miras
        }));
        assert!(!kalan.iter().any(|ace| {
            ace.tur == 0
                && ace.sid.as_ref() == Some(&sid)
                && ace.maske == kalfa_izi.maske
                && u32::from(ace.bayraklar) & 0x0F == kalfa_izi.miras
        }));
    }

    #[test]
    fn alan_kaldirma_belirsiz_ace_varken_kaydi_korur() {
        let gecici = tempfile::tempdir().unwrap();
        let kok = gecici.path().join("calisma");
        let paylasilan = kok.join("paylasilan");
        std::fs::create_dir_all(&paylasilan).unwrap();
        let kimlik = alan_kimligi(&kok);
        let ad = profil_adi(&kimlik);
        let sid = guvenlik::profil_sidi(&ad).unwrap();
        guvenlik::izin_ekle(
            &paylasilan,
            &Izin {
                sid: sid.clone(),
                maske: MASKE_OKU,
                miras: MIRAS_YOK,
            },
        )
        .unwrap();
        let dacl_onceki = guvenlik::dacl_sddl(&paylasilan).unwrap();
        let ace_yolu = gecici.path().join("ace-kaydi.jsonl");
        let mut ace = AceKaydi::oku(&ace_yolu);
        ace.ekle(Girdi::Profil {
            alan: kimlik.clone(),
            ad: ad.clone(),
        })
        .unwrap();
        ace.ekle(Girdi::Izin {
            alan: kimlik.clone(),
            yol: paylasilan.clone(),
            sid: sid.metin(),
            maske: MASKE_DEGISTIR,
            miras: MIRAS,
        })
        .unwrap();
        let kayit_onceki = std::fs::read(&ace_yolu).unwrap();
        let y = WindowsYalitici {
            ayar: RwLock::new(WindowsAyari::default()),
            durum: RwLock::new(YalitimDurumu::Hazir {
                arka_uc: ARKA_UC.into(),
                surum: "AppContainer".into(),
            }),
            degisince: Box::new(|_| {}),
            ace_yolu: ace_yolu.clone(),
            ace: Mutex::new(ace),
            okunur_izin_hatasi: RwLock::new(None),
        };
        let alan = AlanBilgisi {
            kok,
            gizli: Default::default(),
            korumali: Vec::new(),
            git_dizinleri: Vec::new(),
        };

        assert!(y.alani_kaldir(&alan).is_err());
        assert_eq!(std::fs::read(&ace_yolu).unwrap(), kayit_onceki);
        assert_eq!(guvenlik::dacl_sddl(&paylasilan).unwrap(), dacl_onceki);
        let kalan = AceKaydi::oku(&ace_yolu);
        assert!(kalan.girdiler().iter().any(|girdi| matches!(girdi,
            Girdi::Profil { alan, ad: kayitli_ad } if alan == &kimlik && kayitli_ad == &ad)));
        assert!(kalan.girdiler().iter().any(|girdi| matches!(girdi,
            Girdi::Izin { alan, yol, .. } if alan == &kimlik && yol == &paylasilan)));
    }

    #[test]
    fn kimlikle_kaldirma_kayitsiz_ve_uyusmayan_kimligi_reddeder() {
        let gecici = tempfile::tempdir().unwrap();
        let kok = gecici.path().join("calisma");
        std::fs::create_dir(&kok).unwrap();
        let ace_yolu = gecici.path().join("ace-kaydi.jsonl");
        let mut ace = AceKaydi::oku(&ace_yolu);
        ace.ekle(Girdi::Hazir {
            alan: "yanlis-kimlik".into(),
            kok: kok.clone(),
        })
        .unwrap();
        let kayit_onceki = std::fs::read(&ace_yolu).unwrap();
        let y = WindowsYalitici {
            ayar: RwLock::new(WindowsAyari::default()),
            durum: RwLock::new(YalitimDurumu::Hazir {
                arka_uc: ARKA_UC.into(),
                surum: "AppContainer".into(),
            }),
            degisince: Box::new(|_| {}),
            ace_yolu: ace_yolu.clone(),
            ace: Mutex::new(ace),
            okunur_izin_hatasi: RwLock::new(None),
        };

        assert!(y.alani_kaldir_kimlikle("kayitsiz").is_err());
        assert!(y.alani_kaldir_kimlikle("yanlis-kimlik").is_err());
        assert_eq!(std::fs::read(&ace_yolu).unwrap(), kayit_onceki);
        assert!(y.hazirlanmis_alanlar().is_empty());
    }

    #[test]
    fn silinen_kok_kayittaki_kimligini_korur_ve_listede_kalir() {
        let gecici = tempfile::tempdir().unwrap();
        let kok = gecici.path().join("calisma");
        std::fs::create_dir(&kok).unwrap();
        let kimlik = alan_kimligi(&kok);
        let kayitli_kok = guvenlik::sade(&std::fs::canonicalize(&kok).unwrap());
        assert_eq!(kayitli_kok_kimligi(&kayitli_kok), kimlik);
        std::fs::remove_dir(&kok).unwrap();
        assert_eq!(kayitli_kok_kimligi(&kayitli_kok), kimlik);

        let ace_yolu = gecici.path().join("ace-kaydi.jsonl");
        let mut ace = AceKaydi::oku(&ace_yolu);
        ace.ekle(Girdi::Hazir {
            alan: kimlik.clone(),
            kok: kayitli_kok.clone(),
        })
        .unwrap();
        let y = WindowsYalitici {
            ayar: RwLock::new(WindowsAyari::default()),
            durum: RwLock::new(YalitimDurumu::Hazir {
                arka_uc: ARKA_UC.into(),
                surum: "AppContainer".into(),
            }),
            degisince: Box::new(|_| {}),
            ace_yolu,
            ace: Mutex::new(ace),
            okunur_izin_hatasi: RwLock::new(None),
        };
        assert_eq!(
            y.hazirlanmis_alanlar(),
            vec![HazirAlan {
                kimlik,
                kok: kayitli_kok,
            }]
        );
    }

    #[test]
    fn olmayan_uzun_bicimli_kokun_kimligi_degismez() {
        assert_eq!(
            kayitli_kok_kimligi(Path::new(r"\\?\C:\kalfa-yok\calisma")),
            alan_kimligi(Path::new(r"\\?\C:\kalfa-yok\calisma"))
        );
    }

    #[test]
    fn kimlikle_kaldirma_bozuk_ace_kaydini_koruyup_hata_verir() {
        let gecici = tempfile::tempdir().unwrap();
        let kok = gecici.path().join("calisma");
        std::fs::create_dir(&kok).unwrap();
        let kimlik = alan_kimligi(&kok);
        let ace_yolu = gecici.path().join("ace-kaydi.jsonl");
        let mut ace = AceKaydi::oku(&ace_yolu);
        ace.ekle(Girdi::Hazir {
            alan: kimlik.clone(),
            kok: kok.clone(),
        })
        .unwrap();
        let mut kayit_onceki = std::fs::read(&ace_yolu).unwrap();
        kayit_onceki.extend_from_slice(b"gecersiz-kayit\n");
        std::fs::write(&ace_yolu, &kayit_onceki).unwrap();
        let ace = AceKaydi::oku(&ace_yolu);
        assert!(ace.bozuk_mu());
        let y = WindowsYalitici {
            ayar: RwLock::new(WindowsAyari::default()),
            durum: RwLock::new(YalitimDurumu::Hazir {
                arka_uc: ARKA_UC.into(),
                surum: "AppContainer".into(),
            }),
            degisince: Box::new(|_| {}),
            ace_yolu: ace_yolu.clone(),
            ace: Mutex::new(ace),
            okunur_izin_hatasi: RwLock::new(None),
        };

        assert_eq!(
            y.hazirlanmis_alanlar(),
            vec![HazirAlan {
                kimlik: kimlik.clone(),
                kok,
            }]
        );
        let hata = y.alani_kaldir_kimlikle(&kimlik).unwrap_err();
        assert!(hata.contains("ACE kaydı bozuk"), "{hata}");
        assert_eq!(std::fs::read(&ace_yolu).unwrap(), kayit_onceki);
        assert_eq!(y.hazirlanmis_alanlar().len(), 1);
    }

    #[test]
    fn ortak_izin_esitleme_belirsiz_ace_varken_kaydi_korur() {
        let gecici = tempfile::tempdir().unwrap();
        let yol = gecici.path().join("araclar");
        std::fs::create_dir(&yol).unwrap();
        let yol = guvenlik::sade(&std::fs::canonicalize(&yol).unwrap());
        let sid = guvenlik::arac_zinciri_sidi().unwrap();
        guvenlik::izin_ekle(
            &yol,
            &Izin {
                sid: sid.clone(),
                maske: MASKE_TAM,
                miras: MIRAS,
            },
        )
        .unwrap();
        let dacl_onceki = guvenlik::dacl_sddl(&yol).unwrap();
        let ace_yolu = gecici.path().join("ace-kaydi.jsonl");
        let mut ace = AceKaydi::oku(&ace_yolu);
        ace.ekle(Girdi::Izin {
            alan: ace_kaydi::ORTAK.into(),
            yol: yol.clone(),
            sid: sid.metin(),
            maske: MASKE_OKU,
            miras: MIRAS,
        })
        .unwrap();
        let girdiler_onceki = ace.girdiler().to_vec();
        let kayit_onceki = std::fs::read(&ace_yolu).unwrap();

        assert!(ortak_arac_izinlerini_esitle(&mut ace, &[]).is_err());
        assert_eq!(ace.girdiler(), girdiler_onceki);
        assert_eq!(std::fs::read(&ace_yolu).unwrap(), kayit_onceki);
        assert_eq!(guvenlik::dacl_sddl(&yol).unwrap(), dacl_onceki);
    }
}
