//! Kabuk komutlarının çalıştırılması (aşama 2 tasarımı §3, §6).
//!
//! `Yalitici` bir komutu başlatır; `Surec` çıktısını okur, süre ve iptali
//! uygular, bitince bütün süreç ağacını sonlandırır. Süreç sınırları
//! yalıtımdan bağımsızdır: yalıtımsız arka uç da onları uygular. Crate
//! `kalfa-core`'a bağlı değildir.

mod agac;
pub mod etkilesimli;
pub mod git_denetimi;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod ortam;
pub mod profil;
pub mod program;
mod surec;
pub mod tarama;
pub mod vekil;
#[cfg(windows)]
pub mod windows;
mod yalitimsiz;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;

pub use etkilesimli::{
    CalistirmaKipi, EtkilesimliIstek, KAPANMA_PAYI, TerminalBoyutu, mcp_kip_uyusmasi,
    sunucu_ortami, sunucu_ozel,
};
pub use profil::{AlanBilgisi, GitKimligi, Korumali, OzelDizinler, alan_kimligi};
pub use surec::{
    Akis, Bitis, CalismaSonucu, EtkilesimliSurec, KapanisSonucu, Surec, TerminalDenetimi,
};
pub use tarama::Tarama;
pub use yalitimsiz::Yalitimsiz;

/// Süreç ağacının en çok süreç sayısı (aşama 2 tasarımı §6.3).
pub const AZAMI_SUREC: u32 = 256;

/// Yalıtımın kullanılabilirliği (aşama 2 tasarımı §3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum YalitimDurumu {
    Hazir {
        arka_uc: String,
        surum: String,
    },
    Yok {
        neden: YalitimYoklukNedeni,
        /// Geliştiriciye ya da BT'ye yönelik talimat.
        talimat: String,
        /// BT için teknik ayrıntı (§9.8): düşen denetim ve işletim sisteminin
        /// hatası, örneğin `denetim=kok-izni; C:\ sürücüsünün kök izni eksik.`
        ayrinti: String,
    },
}

/// İşletim sisteminin sürümü (§9.8); BT için teknik raporda yazılır.
pub fn sistem_surumu() -> String {
    #[cfg(windows)]
    {
        windows::sistem::surum()
    }
    #[cfg(target_os = "linux")]
    {
        linux::sistem_surumu()
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        std::env::consts::OS.to_string()
    }
}

/// Yalıtımın bu platformda neye ihtiyaç duyduğu (§9.8); BT için teknik
/// raporda yazılır.
pub fn gereksinim() -> String {
    #[cfg(windows)]
    {
        windows::sistem::gereksinim()
    }
    #[cfg(target_os = "linux")]
    {
        linux::GEREKSINIM.to_string()
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        "Bu platformda Kalfa'nın yalıtım arka ucu yok; komutlar yalnızca Ayarlar'da \
         yalıtımsız çalıştırma açıkken, her biri ayrıca onaylanarak çalışır."
            .to_string()
    }
}

impl YalitimDurumu {
    pub fn hazir_mi(&self) -> bool {
        matches!(self, Self::Hazir { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum YalitimYoklukNedeni {
    BwrapYok,
    KullaniciAdAlaniKapali,
    KurulumGerekli,
    KurulumBozuk,
    Desteklenmiyor,
}

/// Çalıştırılacak komutun biçimi (aşama 2 tasarımı §4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KomutBicimi {
    /// Kabuk yorumlayıcısı olmadan: `program` çıplak bir ad ya da mutlak bir
    /// yoldur. Çıplak ad komutun ortamındaki `PATH`'te aranır.
    Program {
        program: String,
        argumanlar: Vec<String>,
    },
    /// Platformun kabuğunda: Linux'ta `/bin/sh -c`, Windows'ta PowerShell.
    Betik { metin: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalistirmaIstegi {
    pub komut: KomutBicimi,
    /// Mutlak çalışma dizini.
    pub dizin: PathBuf,
    /// Komutun ortamı; `ortam::uret` ile üretilir.
    pub ortam: Vec<(String, String)>,
    /// Komut ağa erişmek istiyor. Yalıtımsız arka uç ağı kapatamaz.
    pub ag: bool,
    /// Ağ çıkışında izin verilen hostların listesi (§8.3).
    pub ag_listesi: Option<crate::vekil::IzinListesi>,
    /// Yalıtımlı çalıştırmanın çalışma alanı bilgisi. Verilmişse komut
    /// yalnızca yalıtımda çalışır, verilmemişse yalnızca yalıtımsız: istek
    /// böylece onaylandığı kipte çalışır.
    pub alan: Option<AlanBilgisi>,
}

#[derive(Debug, thiserror::Error)]
pub enum BaslatmaHatasi {
    #[error("`{program}` bulunamadı. Komutun PATH'i: {path}")]
    ProgramYok { program: String, path: String },
    #[error("Komut başlatılamadı: {0}")]
    GirisCikis(#[from] std::io::Error),
    #[error("Yalıtım başlatılamadı: {0}")]
    Yalitim(String),
}

/// Onaylanan kip ile çalışma anındaki yalıtım durumunu karşılaştırır (aşama 2
/// tasarımı §3.2). Araç onaylanan tarife göre çalışır: yalıtımlı onaylanan
/// komut yalıtım yoksa, yalıtımsız onaylanan komut yalıtım varsa başlamaz.
/// İki arka ucun `baslat`'ı süreci başlatmadan önce bunu çağırır.
pub fn kip_uyusmasi(yalitimli_onaylandi: bool, yalitim_hazir: bool) -> Result<(), BaslatmaHatasi> {
    match (yalitimli_onaylandi, yalitim_hazir) {
        (true, true) | (false, false) => Ok(()),
        (true, false) => Err(BaslatmaHatasi::Yalitim(
            "komut yalıtımlı çalışmak üzere onaylandı ama yalıtım artık kullanılamıyor".into(),
        )),
        (false, true) => Err(BaslatmaHatasi::Yalitim(
            "komut yalıtımsız çalışmak üzere onaylandı ama yalıtım artık kullanılabiliyor".into(),
        )),
    }
}

/// Yalıtım için hazırlanmış çalışma alanı. Kimlik, kullanıcı arayüzünden yol
/// kabul etmeden alanı kaldırmak için kullanılır.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HazirAlan {
    pub kimlik: String,
    pub kok: PathBuf,
}

/// Bir komutu başlatan arka uç (aşama 2 tasarımı §3.2).
pub trait Yalitici: Send + Sync {
    fn durum(&self) -> YalitimDurumu;

    /// Yalıtım profilinin özeti (§7.1); mühre girer. Yalıtım yoksa `None`.
    fn profil_ozeti(&self, _ag: bool, _liste_ozeti: Option<&str>) -> Option<String> {
        None
    }

    fn baslat(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi>;

    /// MCP sunucusu gibi uzun yaşayan, borulu süreç (1.1a §4.1). Tokio
    /// çalışma zamanının içinden çağrılır.
    fn baslat_etkilesimli(
        &self,
        _istek: &EtkilesimliIstek,
    ) -> Result<EtkilesimliSurec, BaslatmaHatasi> {
        Err(BaslatmaHatasi::Yalitim(
            "bu yalıtıcı etkileşimli süreç başlatamaz".into(),
        ))
    }

    /// Kabuk komutunu arka plan işi ya da etkileşimli terminal (PTY/ConPTY)
    /// olarak başlatır. `terminal` `None` ise borulu stdin/stdout/stderr ile,
    /// `Some(boyut)` ise sözde terminalle başlar. Tokio çalışma zamanının
    /// içinden çağrılır.
    fn baslat_kabuk_etkilesimli(
        &self,
        _istek: &CalistirmaIstegi,
        _terminal: Option<TerminalBoyutu>,
    ) -> Result<EtkilesimliSurec, BaslatmaHatasi> {
        Err(BaslatmaHatasi::Yalitim(
            "bu yalıtıcı etkileşimli kabuk süreci başlatamaz".into(),
        ))
    }

    /// Sunucunun özel kökünü ve Windows'ta ona verilen izinleri kaldırır
    /// (§3, §5.6).
    fn sunucu_kokunu_kaldir(&self, _alan_koku: &Path, _sunucu: &str) -> Result<(), String> {
        Err("bu yalıtıcı sunucu kökü kaldıramaz".into())
    }

    /// Algılamayı yeniden çalıştırır (§8.3): Ayarlar'dan istenince ve
    /// yalıtım başlatılamayınca.
    fn yeniden_algila(&self) -> YalitimDurumu {
        self.durum()
    }

    /// ACE kaydındaki tamamlanmış hazırlıklar. Linux'ta hazırlık gerekmez.
    fn hazirlanmis_alanlar(&self) -> Vec<HazirAlan> {
        Vec::new()
    }

    /// Yalnızca ACE kaydında bulunan kimliği kaldırır. Desteklenmeyen arka
    /// uçlar başarı bildirmez.
    fn alani_kaldir_kimlikle(&self, _kimlik: &str) -> Result<(), String> {
        Err("Bu platformda hazırlanmış çalışma alanı kaldırılamıyor".into())
    }

    /// Kullanıcının eklediği araç zinciri dizinleri (§7.1).
    fn okunur_dizinleri_ayarla(&self, _dizinler: Vec<PathBuf>) {}

    /// Çalışma alanı yalıtım için hazırlanmalı mı (§9.4)? Hazırlık çalışma
    /// alanının izinlerini değiştirir; onayı her kipte yerel pencerede
    /// sorulur. Linux'ta hazırlık yoktur.
    fn hazirlik_gerekli(&self, _kok: &Path) -> bool {
        false
    }

    /// Araç zinciri dizininin kapta okunabilmesi yönetici onayı ister mi
    /// (2d-3)? Windows'ta: dizini ne araç zinciri yeteneği ne de bütün
    /// uygulama paketleri okuyabiliyorsa ve geçerli kullanıcı dizinin
    /// DACL'ini yazamıyorsa. Linux'ta böyle bir izin yoktur.
    fn okunur_yonetici_gerekli(&self, _dizin: &Path) -> bool {
        false
    }

    /// Yalıtımlı her çalıştırmadan hemen önce çağrılır (§9.4).
    ///
    /// - Hazırlanmamış çalışma alanı yalnızca `hazirlik_onaylandi` ise
    ///   hazırlanır: onaylanan tarif hazırlığı içeriyordu. Değilse hata döner;
    ///   onaydan sonra hazırlık geri alındıysa komut başlamaz.
    /// - Hazırlanmış çalışma alanında yeni gizli dosyalar ve korumalı yollar
    ///   işlenir.
    /// - `ilerleme` araç kartına giden satırlardır.
    fn alani_hazirla(
        &self,
        _alan: &AlanBilgisi,
        _hazirlik_onaylandi: bool,
        _ilerleme: &mut dyn FnMut(&str),
    ) -> Result<(), BaslatmaHatasi> {
        Ok(())
    }
}

/// Platformun yalıtıcısının ayarları.
pub struct PlatformAyari {
    /// Kullanıcının ev dizini; araç zinciri dizinleri buna göre bulunur.
    pub ev: PathBuf,
    /// Çalışma alanına özel dizinlerin kökü: `<veri dizini>/yalitim`.
    pub yalitim_dizini: PathBuf,
    /// Kullanıcının eklediği araç zinciri dizinleri.
    pub ek_okunur: Vec<PathBuf>,
    /// Kurulu Kalfa'nın getirdiği araç zinciri; yalnızca Windows'ta vardır
    /// (`windows::sistem::kurulu_arac_zinciri`), Linux'ta kullanılmaz.
    pub kalfa_araclari: Vec<PathBuf>,
    /// Ağ köprüsü ikilisinin yolu (`kalfa-kopru.exe`).
    pub kopru: Option<PathBuf>,
    /// Durum sonradan değişince çağrılır.
    pub degisince: Box<dyn Fn(&YalitimDurumu) + Send + Sync>,
}

/// Platformun yalıtıcısı. Linux'ta bwrap, Windows'ta AppContainer kullanılır;
/// desteklenmiyorsa durum bunu söyler.
pub fn platform_yaliticisi(ayar: PlatformAyari) -> Arc<dyn Yalitici> {
    #[cfg(target_os = "linux")]
    {
        let (vekil_cozucu, vekil_yerel_izin) = crate::vekil::e2e_cozucu_ve_yerel_izin();
        Arc::new(linux::LinuxYalitici::yeni(
            linux::LinuxAyari {
                ev: ayar.ev,
                yalitim_dizini: ayar.yalitim_dizini,
                ek_okunur: ayar.ek_okunur,
                kopru: ayar.kopru,
                git: GitKimligi::oku(),
                vekil_cozucu,
                vekil_yerel_izin,
            },
            Box::new(linux::BwrapAlgilayici),
            ayar.degisince,
        ))
    }
    #[cfg(windows)]
    {
        let (vekil_cozucu, vekil_yerel_izin) = windows::yalitici::e2e_cozucu_ve_yerel_izin();
        Arc::new(windows::yalitici::WindowsYalitici::yeni(
            windows::yalitici::WindowsAyari {
                ev: ayar.ev,
                yalitim_dizini: ayar.yalitim_dizini,
                ek_okunur: ayar.ek_okunur,
                kalfa_araclari: ayar.kalfa_araclari,
                kopru: ayar.kopru,
                git: GitKimligi::oku(),
                vekil_cozucu,
                vekil_yerel_izin,
            },
            ayar.degisince,
        ))
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = ayar;
        Arc::new(Yalitimsiz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kip_uyusmasi_dort_hucre() {
        assert!(kip_uyusmasi(true, true).is_ok());
        assert!(kip_uyusmasi(false, false).is_ok());
        match kip_uyusmasi(true, false) {
            Err(BaslatmaHatasi::Yalitim(m)) => assert_eq!(
                m,
                "komut yalıtımlı çalışmak üzere onaylandı ama yalıtım artık kullanılamıyor"
            ),
            diger => panic!("{diger:?}"),
        }
        match kip_uyusmasi(false, true) {
            Err(BaslatmaHatasi::Yalitim(m)) => assert_eq!(
                m,
                "komut yalıtımsız çalışmak üzere onaylandı ama yalıtım artık kullanılabiliyor"
            ),
            diger => panic!("{diger:?}"),
        }
    }
}
