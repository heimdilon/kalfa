//! Etkileşimli süreç (1.1a tasarımı §3, §4.1): stdin/stdout borulu,
//! kapatılana kadar yaşayan süreç. MCP sunucuları bununla başlar. Toplam
//! süre ve çıktı sınırı yoktur; süreç sınırları kabuktaki gibidir.

use std::path::Path;
use std::time::Duration;

use crate::profil::{AlanBilgisi, OzelDizinler};
use crate::{BaslatmaHatasi, vekil};

/// `kapat()`'ta stdin kapandıktan sonra kendiliğinden çıkış için tanınan
/// süre (§4.1).
pub const KAPANMA_PAYI: Duration = Duration::from_secs(2);

/// Sunucunun açıkça seçilmiş çalışma kipi (§4.1). Kabuktaki yalıtımsız
/// çalışma bir geri düşme yoludur; burada tam erişim kullanıcının seçimidir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalistirmaKipi {
    Yalitimli,
    TamErisim,
}

#[derive(Debug, Clone)]
pub struct EtkilesimliIstek {
    /// Çıplak program adı ya da mutlak yol.
    pub program: String,
    pub argumanlar: Vec<String>,
    /// Kalıtılan ve yapılandırılan ortam; `sunucu_ortami` özel köke çevirir.
    pub ortam: Vec<(String, String)>,
    pub kip: CalistirmaKipi,
    /// `[a-z0-9-]{1,32}`; özel kökün dizin adıdır.
    pub sunucu: String,
    /// Çalışma alanının bilgisi. Yalıtımlıda bağlamalar ve gizliler
    /// buradan gelir; tam erişimde yalnız `kok` kullanılır.
    pub alan: AlanBilgisi,
    /// Yalnız yalıtımlıda; `None` ağsızdır. Tam erişimde verilirse başlatma
    /// reddedilir: yalıtımsız süreçte ağ kapatılamaz.
    pub ag_listesi: Option<vekil::IzinListesi>,
}

/// Sunucu adı yol bileşeni olarak kullanılır; izin listesiyle sınırlıdır.
pub fn sunucu_adi_dogrula(ad: &str) -> Result<(), BaslatmaHatasi> {
    let gecerli = !ad.is_empty()
        && ad.len() <= 32
        && ad
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if gecerli {
        Ok(())
    } else {
        Err(BaslatmaHatasi::Yalitim(format!(
            "geçersiz sunucu adı: {ad:?}"
        )))
    }
}

/// Sunucunun çalışma alanına özel kökü (§3).
pub fn sunucu_ozel(yalitim_dizini: &Path, alan_kimligi: &str, sunucu: &str) -> OzelDizinler {
    OzelDizinler::yol(&yalitim_dizini.join(alan_kimligi).join("mcp"), sunucu)
}

const AYRILMIS_ONEKLER: &[&str] = &["EVREN_", "KALFA_"];

/// Ad Kalfa'ya ayrılmış mı? Bu adlar hiçbir yoldan sunucuya geçmez (§3).
pub(crate) fn ayrilmis_mi(ad: &str) -> bool {
    let buyuk = ad.to_ascii_uppercase();
    AYRILMIS_ONEKLER.iter().any(|o| buyuk.starts_with(o))
}

/// Sunucunun ortamı (§3): profil ve önbellek değişkenleri özel köke
/// yönelir; `EVREN_` ve `KALFA_` önekli adlar hiçbir koşulda geçmez.
pub fn sunucu_ortami(kaynak: &[(String, String)], ozel: &OzelDizinler) -> Vec<(String, String)> {
    let yol = |p: &Path| p.display().to_string();
    let mut yonlu: Vec<(&str, String)> = vec![
        ("HOME", yol(&ozel.ev)),
        ("USERPROFILE", yol(&ozel.ev)),
        ("APPDATA", yol(&ozel.ev.join("Roaming"))),
        ("LOCALAPPDATA", yol(&ozel.ev)),
        ("TMPDIR", yol(&ozel.gecici)),
        ("TEMP", yol(&ozel.gecici)),
        ("TMP", yol(&ozel.gecici)),
        ("NPM_CONFIG_CACHE", yol(&ozel.npm())),
        ("PIP_CACHE_DIR", yol(&ozel.pip())),
        ("UV_CACHE_DIR", yol(&ozel.onbellek.join("uv"))),
        ("XDG_CACHE_HOME", yol(&ozel.onbellek)),
    ];
    if cfg!(windows) {
        // G1: AppContainer başlatması bu değişkenler olmadan 203 ile düşer.
        let sistem = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let surucu = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        yonlu.push(("SystemRoot", sistem.clone()));
        yonlu.push(("windir", sistem));
        yonlu.push(("SystemDrive", surucu));
    }
    let ayni = |a: &str, b: &str| {
        if cfg!(windows) {
            a.eq_ignore_ascii_case(b)
        } else {
            a == b
        }
    };
    let mut ortam: Vec<(String, String)> = kaynak
        .iter()
        .filter(|(ad, _)| !ayrilmis_mi(ad))
        .filter(|(ad, _)| !yonlu.iter().any(|(y, _)| ayni(y, ad)))
        .cloned()
        .collect();
    ortam.extend(yonlu.into_iter().map(|(a, d)| (a.to_string(), d)));
    ortam
}

/// Seçilen kip ile yalıtımın durumu (§4.1). Yalıtımlı tarif hiçbir zaman
/// yalıtımsız arka uca düşmez; tam erişim her durumda yalıtımsız başlar.
pub fn mcp_kip_uyusmasi(kip: CalistirmaKipi, yalitim_hazir: bool) -> Result<(), BaslatmaHatasi> {
    match (kip, yalitim_hazir) {
        (CalistirmaKipi::Yalitimli, false) => Err(BaslatmaHatasi::Yalitim(
            "sunucu yalıtımlı çalışmak üzere onaylandı ama yalıtım kullanılamıyor".into(),
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sunucu_adi_yalniz_izin_listesindeki_karakterler() {
        for gecerli in ["a", "github", "fs-2", &"a".repeat(32)] {
            assert!(sunucu_adi_dogrula(gecerli).is_ok(), "{gecerli}");
        }
        for gecersiz in ["", "../x", "a/b", "a\\b", "A", "ğ", " a", &"a".repeat(33)] {
            assert!(sunucu_adi_dogrula(gecersiz).is_err(), "{gecersiz:?}");
        }
    }

    #[test]
    fn ozel_kok_alanin_mcp_dizininde() {
        let o = sunucu_ozel(Path::new("/v/yalitim"), "abcd", "github");
        assert_eq!(o.kok, Path::new("/v/yalitim/abcd/mcp/github"));
        assert_eq!(o.ev, o.kok.join("ev"));
        assert_eq!(o.gecici, o.kok.join("gecici"));
        assert_eq!(o.onbellek, o.kok.join("onbellek"));
    }

    #[test]
    fn ortam_ozel_koke_yonelir_ayrilmis_adlar_gecmez() {
        let o = sunucu_ozel(Path::new("/v"), "a", "s");
        let kaynak = vec![
            ("HOME".to_string(), "/home/kullanici".to_string()),
            ("PATH".to_string(), "/usr/bin".to_string()),
            ("EVREN_API_KEY".to_string(), "gizli".to_string()),
            ("evren_base_url".to_string(), "x".to_string()),
            ("KALFA_E2E".to_string(), "1".to_string()),
            ("GITHUB_TOKEN".to_string(), "ghp".to_string()),
        ];
        let ortam = sunucu_ortami(&kaynak, &o);
        let deger = |ad: &str| ortam.iter().find(|(a, _)| a == ad).map(|(_, d)| d.as_str());
        assert_eq!(deger("HOME"), Some(o.ev.display().to_string().as_str()));
        assert_eq!(
            deger("TMPDIR"),
            Some(o.gecici.display().to_string().as_str())
        );
        assert_eq!(
            deger("NPM_CONFIG_CACHE"),
            Some(o.npm().display().to_string().as_str())
        );
        assert_eq!(
            deger("PIP_CACHE_DIR"),
            Some(o.pip().display().to_string().as_str())
        );
        assert_eq!(
            deger("UV_CACHE_DIR"),
            Some(o.onbellek.join("uv").display().to_string().as_str())
        );
        assert_eq!(
            deger("XDG_CACHE_HOME"),
            Some(o.onbellek.display().to_string().as_str())
        );
        assert_eq!(deger("PATH"), Some("/usr/bin"));
        assert_eq!(deger("GITHUB_TOKEN"), Some("ghp"));
        assert!(ortam.iter().all(|(a, _)| !ayrilmis_mi(a)));
        assert_eq!(ortam.iter().filter(|(a, _)| a == "HOME").count(), 1);
    }

    #[test]
    fn kip_uyusmasi_dort_hucre() {
        assert!(mcp_kip_uyusmasi(CalistirmaKipi::Yalitimli, true).is_ok());
        assert!(mcp_kip_uyusmasi(CalistirmaKipi::TamErisim, true).is_ok());
        assert!(mcp_kip_uyusmasi(CalistirmaKipi::TamErisim, false).is_ok());
        assert!(matches!(
            mcp_kip_uyusmasi(CalistirmaKipi::Yalitimli, false),
            Err(BaslatmaHatasi::Yalitim(_))
        ));
    }
}
