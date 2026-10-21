//! Çalışma alanı sınırlı proje talimatı (`AGENTS.md`) okuyucusu (G1a, K06).
//!
//! Çekirdek (`kalfa-core`) doğrudan dosya sistemi açmaz; bu adaptör
//! `kalfa_tools::CalismaAlani` (`cap-std`, `FollowSymlinks::No`) üzerinden
//! yalnız çalışma alanı içindeki normal, bağlantısız, gizli/korumalı olmayan
//! ve 16 KiB sınırını aşmayan UTF-8 `AGENTS.md` dosyalarını okur.

use std::io;
use std::path::Path;
use std::sync::Arc;

use kalfa_core::proje_talimatlari::{
    DOSYA_BOYUT_SINIRI, ProjeTalimatOkuyucu, ProjeTalimatUyariTuru, TalimatOkumaSonucu,
    icerik_ozeti_hesapla,
};
use kalfa_core::yol::CalismaYolu;
use kalfa_tools::CalismaAlani;

pub struct CalismaAlaniTalimatOkuyucu {
    alan: Arc<CalismaAlani>,
}

impl CalismaAlaniTalimatOkuyucu {
    pub fn yeni(alan: Arc<CalismaAlani>) -> Self {
        Self { alan }
    }
}

fn yol_baglanti_iceriyor_mu(kok: &Path, yol: &CalismaYolu) -> bool {
    let mut simdiki = kok.to_path_buf();
    for parca in yol.as_str().split('/') {
        simdiki.push(parca);
        if let Ok(meta) = std::fs::symlink_metadata(&simdiki) {
            if meta.file_type().is_symlink() {
                return true;
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
                if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                    return true;
                }
            }
        } else {
            return false;
        }
    }
    false
}

impl ProjeTalimatOkuyucu for CalismaAlaniTalimatOkuyucu {
    fn dizin_talimatini_oku(&self, goreli_dizin: &str) -> TalimatOkumaSonucu {
        let goreli_yol = if goreli_dizin.is_empty() {
            "AGENTS.md".to_string()
        } else {
            format!("{goreli_dizin}/AGENTS.md")
        };
        let Ok(yol) = CalismaYolu::coz(&goreli_yol) else {
            return TalimatOkumaSonucu::Uyari(ProjeTalimatUyariTuru::Okunamadi {
                neden: "geçersiz göreli yol".into(),
            });
        };

        let siniflandirici = self.alan.siniflandirici();
        if siniflandirici.gizli_mi(&yol) || siniflandirici.korumali_mi(&yol) {
            return TalimatOkumaSonucu::Uyari(ProjeTalimatUyariTuru::GizliVeyaKorumaliReddedildi);
        }

        let (boyut, _) = match self.alan.dosya_bilgisi(&yol) {
            Ok(bilgi) => bilgi,
            Err(e) => {
                if yol_baglanti_iceriyor_mu(self.alan.kok(), &yol) {
                    return TalimatOkumaSonucu::Uyari(
                        ProjeTalimatUyariTuru::SembolikBaglantiReddedildi,
                    );
                }
                if matches!(
                    e.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) {
                    return TalimatOkumaSonucu::Yok;
                }
                return TalimatOkumaSonucu::Uyari(ProjeTalimatUyariTuru::Okunamadi {
                    neden: e.to_string(),
                });
            }
        };

        match self.alan.baglanti_sayisi(&yol) {
            Ok(sayi) if sayi > 1 => {
                return TalimatOkumaSonucu::Uyari(
                    ProjeTalimatUyariTuru::SembolikBaglantiReddedildi,
                );
            }
            Ok(_) => {}
            Err(e) => {
                return TalimatOkumaSonucu::Uyari(ProjeTalimatUyariTuru::Okunamadi {
                    neden: e.to_string(),
                });
            }
        }

        if boyut > DOSYA_BOYUT_SINIRI as u64 {
            return TalimatOkumaSonucu::Uyari(ProjeTalimatUyariTuru::DosyaBoyutuAsildi {
                sinir: DOSYA_BOYUT_SINIRI,
                gercek: usize::try_from(boyut).unwrap_or(usize::MAX),
            });
        }

        let baytlar = match self.alan.oku(&yol) {
            Ok(b) => b,
            Err(e) => {
                return TalimatOkumaSonucu::Uyari(ProjeTalimatUyariTuru::Okunamadi {
                    neden: e.to_string(),
                });
            }
        };

        if baytlar.len() > DOSYA_BOYUT_SINIRI {
            return TalimatOkumaSonucu::Uyari(ProjeTalimatUyariTuru::DosyaBoyutuAsildi {
                sinir: DOSYA_BOYUT_SINIRI,
                gercek: baytlar.len(),
            });
        }

        if baytlar.contains(&0) {
            return TalimatOkumaSonucu::Uyari(ProjeTalimatUyariTuru::GecersizUtf8);
        }

        let Ok(icerik) = String::from_utf8(baytlar) else {
            return TalimatOkumaSonucu::Uyari(ProjeTalimatUyariTuru::GecersizUtf8);
        };
        let bayt = icerik.len();
        let ozet = icerik_ozeti_hesapla(icerik.as_bytes());
        TalimatOkumaSonucu::Yuklendi { icerik, bayt, ozet }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kalfa_core::izin::Siniflandirici;

    fn test_okuyucu(kok: &Path, ek_gizli: &[String]) -> CalismaAlaniTalimatOkuyucu {
        let siniflandirici = Arc::new(Siniflandirici::yeni(kok, ek_gizli).unwrap());
        let alan = CalismaAlani::ac(siniflandirici).unwrap();
        CalismaAlaniTalimatOkuyucu::yeni(alan)
    }

    #[test]
    fn kok_ve_alt_dizin_agents_okunur_olmayan_yok_doner() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("AGENTS.md"), "Kök kuralı: ğüşıöç").unwrap();
        std::fs::create_dir_all(d.path().join("crates/kalfa-core")).unwrap();
        std::fs::write(
            d.path().join("crates/kalfa-core/AGENTS.md"),
            "Alt dizin kuralı",
        )
        .unwrap();

        let okuyucu = test_okuyucu(d.path(), &[]);
        assert!(matches!(
            okuyucu.dizin_talimatini_oku(""),
            TalimatOkumaSonucu::Yuklendi { ref icerik, .. } if icerik == "Kök kuralı: ğüşıöç"
        ));
        assert_eq!(
            okuyucu.dizin_talimatini_oku("crates"),
            TalimatOkumaSonucu::Yok
        );
        assert!(matches!(
            okuyucu.dizin_talimatini_oku("crates/kalfa-core"),
            TalimatOkumaSonucu::Yuklendi { ref icerik, .. } if icerik == "Alt dizin kuralı"
        ));
    }

    #[test]
    fn asiri_buyuk_gecersiz_utf8_gizli_ve_hardlink_reddedilir() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("buyuk")).unwrap();
        std::fs::write(
            d.path().join("buyuk/AGENTS.md"),
            vec![b'a'; DOSYA_BOYUT_SINIRI + 1],
        )
        .unwrap();

        std::fs::create_dir_all(d.path().join("ikili")).unwrap();
        std::fs::write(d.path().join("ikili/AGENTS.md"), [0xff, 0xfe, 0x00]).unwrap();

        std::fs::create_dir_all(d.path().join("gizli-dizin")).unwrap();
        std::fs::write(d.path().join("gizli-dizin/AGENTS.md"), "gizli").unwrap();

        std::fs::create_dir_all(d.path().join("hardlink")).unwrap();
        let kaynak = d.path().join("kaynak.txt");
        std::fs::write(&kaynak, "bağlantılı").unwrap();
        std::fs::hard_link(&kaynak, d.path().join("hardlink/AGENTS.md")).unwrap();

        let okuyucu = test_okuyucu(d.path(), &["gizli-dizin/**".to_string()]);

        assert_eq!(
            okuyucu.dizin_talimatini_oku("buyuk"),
            TalimatOkumaSonucu::Uyari(ProjeTalimatUyariTuru::DosyaBoyutuAsildi {
                sinir: DOSYA_BOYUT_SINIRI,
                gercek: DOSYA_BOYUT_SINIRI + 1,
            })
        );
        assert_eq!(
            okuyucu.dizin_talimatini_oku("ikili"),
            TalimatOkumaSonucu::Uyari(ProjeTalimatUyariTuru::GecersizUtf8)
        );
        assert_eq!(
            okuyucu.dizin_talimatini_oku("gizli-dizin"),
            TalimatOkumaSonucu::Uyari(ProjeTalimatUyariTuru::GizliVeyaKorumaliReddedildi)
        );
        assert_eq!(
            okuyucu.dizin_talimatini_oku("hardlink"),
            TalimatOkumaSonucu::Uyari(ProjeTalimatUyariTuru::SembolikBaglantiReddedildi)
        );
    }
}
