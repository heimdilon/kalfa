//! İzin motoru (spec §7.2–§7.5). Bütün izin kararları buradan geçer.

use std::path::{Path, PathBuf};

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};

use crate::git_koruma;
use crate::yol::CalismaYolu;

pub const GIZLI_UYARISI: &str = "Bu dosyanın içeriği EVREN'e gönderilecek.";
pub const KORUMALI_UYARISI: &str = "Bu dosyayı git kendiliğinden çalıştırır.";
pub const BAGLANTI_UYARISI: &str =
    "Bu dosyanın başka bir yerde de adı var (hard link); yazmak o dosyayı da değiştirir.";

/// Spec §7.4'teki varsayılan gizli dosya kalıpları. Her kalıp her derinlikte
/// ve büyük-küçük harf duyarsız eşleşir.
pub const VARSAYILAN_GIZLI_KALIPLAR: &[&str] = &[
    ".env",
    ".env.*",
    "*.pem",
    "*.key",
    "*.p12",
    "*.pfx",
    "*.keystore",
    "*.kdbx",
    "id_rsa*",
    "id_ed25519*",
    ".netrc",
    ".npmrc",
    ".pypirc",
    ".git-credentials",
    "credentials*",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kip {
    Inceleme,
    Siki,
    Proje,
    Serbest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Islem {
    Okuma,
    Yazma,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YolSinifi {
    Normal,
    Gizli,
    Korumali,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Karar {
    Serbest,
    Onayli { uyarilar: Vec<String> },
    Yasak { neden: String },
}

impl Karar {
    /// Birden çok hedefin kararını birleştirir: Yasak > Onaylı > Serbest.
    pub fn birlestir(self, diger: Karar) -> Karar {
        match (self, diger) {
            (Karar::Yasak { neden }, _) | (_, Karar::Yasak { neden }) => Karar::Yasak { neden },
            (Karar::Onayli { uyarilar: mut a }, Karar::Onayli { uyarilar: b }) => {
                for u in b {
                    if !a.contains(&u) {
                        a.push(u);
                    }
                }
                Karar::Onayli { uyarilar: a }
            }
            (Karar::Onayli { uyarilar }, Karar::Serbest)
            | (Karar::Serbest, Karar::Onayli { uyarilar }) => Karar::Onayli { uyarilar },
            (Karar::Serbest, Karar::Serbest) => Karar::Serbest,
        }
    }
}

pub fn karar_ver(kip: Kip, islem: Islem, sinif: YolSinifi, coklu_baglanti: bool) -> Karar {
    match islem {
        Islem::Okuma => match sinif {
            YolSinifi::Gizli => Karar::Onayli {
                uyarilar: vec![GIZLI_UYARISI.to_string()],
            },
            YolSinifi::Normal | YolSinifi::Korumali => Karar::Serbest,
        },
        Islem::Yazma => {
            if kip == Kip::Inceleme {
                return Karar::Yasak {
                    neden: "İnceleme kipinde dosya yazılamaz.".to_string(),
                };
            }
            let mut uyarilar = Vec::new();
            if sinif == YolSinifi::Korumali {
                uyarilar.push(KORUMALI_UYARISI.to_string());
            }
            if coklu_baglanti {
                uyarilar.push(BAGLANTI_UYARISI.to_string());
            }
            if kip == Kip::Siki || !uyarilar.is_empty() {
                Karar::Onayli { uyarilar }
            } else {
                Karar::Serbest
            }
        }
    }
}

/// Bir yolun sınıfını (gizli / korumalı) belirler.
#[derive(Debug)]
pub struct Siniflandirici {
    kok: PathBuf,
    gizli: GlobSet,
}

impl Siniflandirici {
    pub fn yeni(kok: impl Into<PathBuf>, ek_gizli: &[String]) -> Result<Self, globset::Error> {
        let mut kurucu = GlobSetBuilder::new();
        let kaliplar = VARSAYILAN_GIZLI_KALIPLAR
            .iter()
            .map(|k| k.to_string())
            .chain(ek_gizli.iter().cloned());
        for kalip in kaliplar {
            kurucu.add(
                GlobBuilder::new(&format!("**/{kalip}"))
                    .case_insensitive(true)
                    .literal_separator(true)
                    .build()?,
            );
        }
        Ok(Self {
            kok: kok.into(),
            gizli: kurucu.build()?,
        })
    }

    pub fn kok(&self) -> &Path {
        &self.kok
    }

    pub fn gizli_mi(&self, yol: &CalismaYolu) -> bool {
        self.gizli.is_match(yol.as_str())
    }

    /// Git korumaları her çağrıda yeniden çözülür (spec §7.3): `core.hooksPath`
    /// oturum sırasında değişebilir.
    pub fn korumali_mi(&self, yol: &CalismaYolu) -> bool {
        let anahtar = yol.anahtar();
        git_koruma::korumali_yollar(&self.kok).iter().any(|k| {
            let k = k.trim_end_matches('/');
            anahtar == k || anahtar.starts_with(&format!("{k}/"))
        })
    }

    pub fn sinifla(&self, yol: &CalismaYolu) -> YolSinifi {
        if self.korumali_mi(yol) {
            YolSinifi::Korumali
        } else if self.gizli_mi(yol) {
            YolSinifi::Gizli
        } else {
            YolSinifi::Normal
        }
    }
}
