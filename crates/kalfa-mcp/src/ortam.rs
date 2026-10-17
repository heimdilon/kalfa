//! MCP ortamı: sistem izin listesi, açıkça tanımlanan değerler, en sonda
//! sunucunun özel HOME/cache/temp kökleri.
//!
//! Açık kullanıcı değerleri sistem allowlist'inden ayrı doğrulanır. Bu yüzden
//! `GITHUB_TOKEN` ve `API_KEY` açıkça verilebilir; sistemden kendiliğinden geçmez.
//! Ham değer tipleri Debug/serde taşımaz.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use kalfa_sandbox::{OzelDizinler, ortam as sandbox_ortam, sunucu_ortami};

use crate::tanim::TanimHatasi;

const WINDOWS_KOK_ADLARI: &[&str] = &["SYSTEMROOT", "WINDIR", "SYSTEMDRIVE"];

/// Raw ortam değeri; bilerek Debug, Serialize ve Deserialize uygulamaz.
pub struct OrtamDegeri {
    ad: String,
    deger: String,
    gizli: bool,
}

impl OrtamDegeri {
    pub fn duz(ad: impl Into<String>, deger: impl Into<String>) -> Self {
        Self {
            ad: ad.into(),
            deger: deger.into(),
            gizli: false,
        }
    }

    pub fn gizli(ad: impl Into<String>, deger: impl Into<String>) -> Self {
        Self {
            ad: ad.into(),
            deger: deger.into(),
            gizli: true,
        }
    }

    pub fn ad(&self) -> &str {
        &self.ad
    }

    pub const fn gizli_mi(&self) -> bool {
        self.gizli
    }
}

/// Başlatılacak süreçte kullanılacak son ortam; ham değerler dışarı serileştirilmez.
pub struct SonOrtam {
    degerler: Vec<OrtamDegeri>,
}

impl SonOrtam {
    pub(crate) fn ham_ciftler(&self) -> Vec<(String, String)> {
        self.degerler
            .iter()
            .map(|d| (d.ad.clone(), d.deger.clone()))
            .collect()
    }

    pub(crate) fn ham_degerler(&self) -> impl Iterator<Item = (&str, &str, bool)> {
        self.degerler
            .iter()
            .map(|d| (d.ad.as_str(), d.deger.as_str(), d.gizli))
    }

    pub(crate) fn deger(&self, ad: &str) -> Option<&str> {
        self.degerler
            .iter()
            .find(|d| ayni_ad(&d.ad, ad))
            .map(|d| d.deger.as_str())
    }
}

pub(crate) fn girdileri_dogrula(girdiler: &[OrtamDegeri]) -> Result<(), TanimHatasi> {
    let mut adlar = BTreeSet::new();
    for girdi in girdiler {
        if !ortam_adi_gecerli(&girdi.ad) {
            return Err(TanimHatasi::GecersizOrtamAdi);
        }
        if girdi.deger.contains('\0') {
            return Err(TanimHatasi::GecersizOrtamDegeri);
        }
        let buyuk = girdi.ad.to_ascii_uppercase();
        if buyuk.starts_with("EVREN_") || buyuk.starts_with("KALFA_") || buyuk == "CALISMA_ALANI" {
            return Err(TanimHatasi::AyrilmisOrtamAdi);
        }
        if sandbox_ortam::SABIT
            .iter()
            .any(|(ad, _)| ayni_ad(ad, &girdi.ad))
        {
            return Err(TanimHatasi::SabitOrtamAdi);
        }
        if cfg!(windows) && WINDOWS_KOK_ADLARI.iter().any(|ad| ayni_ad(ad, &girdi.ad)) {
            return Err(TanimHatasi::OzelKokOrtamAdi);
        }
        if !adlar.insert(anahtar(&girdi.ad)) {
            return Err(TanimHatasi::YinelenenOrtamAdi);
        }
    }
    Ok(())
}

fn ortam_adi_gecerli(ad: &str) -> bool {
    if ad.len() > 64 {
        return false;
    }
    let mut baytlar = ad.bytes();
    let Some(ilk) = baytlar.next() else {
        return false;
    };
    (ilk.is_ascii_alphabetic() || ilk == b'_')
        && baytlar.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn ayni_ad(a: &str, b: &str) -> bool {
    if cfg!(windows) {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

fn anahtar(ad: &str) -> String {
    if cfg!(windows) {
        ad.to_ascii_lowercase()
    } else {
        ad.to_string()
    }
}

/// İzinli sistem ortamını alır, açık değerleri üzerine yazar, sunucu köklerini
/// en son zorlar ve sonuçta adları kararlı sıraya koyar.
pub(crate) fn uret(
    sistem: &[(String, String)],
    acik: &[OrtamDegeri],
    ozel: &OzelDizinler,
    proje: &std::path::Path,
    cwd: &std::path::Path,
) -> Result<SonOrtam, TanimHatasi> {
    girdileri_dogrula(acik)?;
    proje.to_str().ok_or(TanimHatasi::YolCozumlenemedi)?;
    for yol in [&ozel.ev, &ozel.gecici, &ozel.onbellek] {
        if yol.to_str().is_none() {
            return Err(TanimHatasi::YolCozumlenemedi);
        }
    }
    let mut gecici = BTreeMap::<String, OrtamDegeri>::new();
    for (ad, deger) in sandbox_ortam::uret(sistem.iter().cloned()) {
        gecici.insert(anahtar(&ad), OrtamDegeri::duz(ad, deger));
    }
    for girdi in acik {
        let deger = crate::tanim::calisma_alani_yerine_koy(&girdi.deger, proje)?;
        gecici.insert(
            anahtar(&girdi.ad),
            OrtamDegeri {
                ad: girdi.ad.clone(),
                deger,
                gizli: girdi.gizli,
            },
        );
    }
    let kaynak = gecici
        .values()
        .map(|d| (d.ad.clone(), d.deger.clone()))
        .collect::<Vec<_>>();
    let zorlanan_ozel = sunucu_ortami(&[], ozel);
    let zorlanmis = sunucu_ortami(&kaynak, ozel);
    let gizlilik: BTreeMap<String, bool> =
        gecici.iter().map(|(k, v)| (k.clone(), v.gizli)).collect();
    let mut son = BTreeMap::<String, OrtamDegeri>::new();
    for (ad, ham) in zorlanmis {
        let ozel_kok_zorlamasi = zorlanan_ozel
            .iter()
            .any(|(z_ad, z_deger)| ayni_ad(z_ad, &ad) && z_deger == &ham);
        let deger = if ad.eq_ignore_ascii_case("PATH") {
            pathi_ozel_calisma_dizinine_gore_mutlak_yap(&ham, cwd)?
        } else {
            ham
        };
        let anahtar = anahtar(&ad);
        let gizli = !ozel_kok_zorlamasi && gizlilik.get(&anahtar).copied().unwrap_or(false);
        son.insert(anahtar, OrtamDegeri { ad, deger, gizli });
    }
    Ok(SonOrtam {
        degerler: son.into_values().collect(),
    })
}

fn pathi_ozel_calisma_dizinine_gore_mutlak_yap(
    path: &str,
    cwd: &Path,
) -> Result<String, TanimHatasi> {
    let mut dizinler = Vec::new();
    for dizin in std::env::split_paths(path) {
        if dizin.as_os_str().is_empty() {
            continue;
        }
        let mutlak = if dizin.is_absolute() {
            dizin
        } else {
            cwd.join(dizin)
        };
        let mutlak = std::fs::canonicalize(&mutlak)
            .or_else(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(mutlak.clone())
                } else {
                    Err(e)
                }
            })
            .map_err(|_| TanimHatasi::YolCozumlenemedi)?;
        if mutlak.to_str().is_none() {
            return Err(TanimHatasi::YolCozumlenemedi);
        }
        dizinler.push(mutlak);
    }
    let birlesik = std::env::join_paths(dizinler).map_err(|_| TanimHatasi::YolCozumlenemedi)?;
    birlesik
        .into_string()
        .map_err(|_| TanimHatasi::YolCozumlenemedi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acik_gizli_anahtar_allowlistten_gecmese_de_ozel_kok_ustun_kalir() {
        let kok = tempfile::tempdir().unwrap();
        let ozel = OzelDizinler::yol(kok.path(), "mcp");
        let proje = kok.path().join("proje");
        let cwd = ozel.ev.clone();
        let path = std::env::join_paths([kok.path().join("tools")]).unwrap();
        let acik = vec![
            OrtamDegeri::gizli("GITHUB_TOKEN", "ghp-private"),
            OrtamDegeri::gizli("API_KEY", "api-private"),
            OrtamDegeri::duz("NODE_OPTIONS", "--require=./hook.js"),
            OrtamDegeri::duz("HOME", "user-home"),
            OrtamDegeri::duz("NPM_CONFIG_CACHE", "user-cache"),
        ];
        let son = uret(
            &[
                ("PATH".into(), path.to_string_lossy().into_owned()),
                ("HOME".into(), "user-home".into()),
            ],
            &acik,
            &ozel,
            &proje,
            &cwd,
        )
        .unwrap();
        assert_eq!(son.deger("GITHUB_TOKEN"), Some("ghp-private"));
        assert_eq!(son.deger("API_KEY"), Some("api-private"));
        assert_eq!(son.deger("HOME"), Some(ozel.ev.to_string_lossy().as_ref()));
        assert_eq!(
            son.deger("NPM_CONFIG_CACHE"),
            Some(ozel.onbellek.join("npm").to_string_lossy().as_ref())
        );
    }

    #[test]
    fn ayrilmis_sabit_ve_ozel_kok_adlari_acikca_reddedilir() {
        for girdi in [
            OrtamDegeri::duz("EVREN_API_KEY", "x"),
            OrtamDegeri::duz("kalfa_secret", "x"),
            OrtamDegeri::duz("TERM", "x"),
        ] {
            assert!(girdileri_dogrula(&[girdi]).is_err());
        }
        assert!(girdileri_dogrula(&[OrtamDegeri::duz("A".repeat(65), "x")]).is_err());
        assert!(matches!(
            girdileri_dogrula(&[OrtamDegeri::duz("API_KEY", "x\0y")]),
            Err(TanimHatasi::GecersizOrtamDegeri)
        ));
        assert!(girdileri_dogrula(&[OrtamDegeri::duz("HOME", "kullanici-degeri")]).is_ok());
        // Açıkça verilen GITHUB_TOKEN özel değer olarak geçer; sistemden otomatik geçmez.
        let sistem = sandbox_ortam::uret([("GITHUB_TOKEN".into(), "system-secret".into())]);
        assert!(!sistem.iter().any(|(ad, _)| ad == "GITHUB_TOKEN"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_ad_cakismalari_harf_duyarsizdir() {
        assert!(matches!(
            girdileri_dogrula(&[OrtamDegeri::duz("Path", "a"), OrtamDegeri::duz("PATH", "b")]),
            Err(TanimHatasi::YinelenenOrtamAdi)
        ));
    }
}
