//! İzin motoru (spec §7.2–§7.5). Bütün izin kararları buradan geçer.

use std::path::{Path, PathBuf};

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};

use crate::git_koruma;
use crate::kabuk::{KabukTarifi, YalitimOzeti};
use crate::yol::CalismaYolu;

pub const GIZLI_UYARISI: &str = "Bu dosyanın içeriği EVREN'e gönderilecek.";
pub const KORUMALI_UYARISI: &str = "Bu yol git'in dizininde ya da kanca dizininde. Git buradaki yapılandırmayı ve kancaları kendiliğinden çalıştırır.";
pub const BAGLANTI_UYARISI: &str =
    "Bu dosyanın başka bir yerde de adı var (hard link); yazmak o dosyayı da değiştirir.";
pub const ILK_KABUK_UYARISI: &str = "Bu çalışma alanında ilk kez komut çalıştırılacak. Projenin \
betikleri (build.rs, package.json, Makefile) kod olarak çalışır.";
pub const YALITIMSIZ_UYARISI: &str = "Yalıtımsız: komut senin bütün yetkilerinle çalışacak; ev \
dizinindeki dosyaları okuyabilir ve ağa erişebilir.";
pub const AGLI_KABUK_UYARISI: &str = "Komut ağa erişebilecek. Dosya yalıtımı sürer.";
pub const ALAN_HAZIRLIGI_UYARISI: &str = "Kalfa bu klasörün izinlerine yalıtım için giriş \
ekleyecek. Klasörü ayarlardan kaldırınca geri alınır.";
pub const YAPILANDIRMA_UYARISI: &str = "Bu dosyayı IDE'niz, git ya da paket yöneticiniz \
Kalfa'nın yalıtımı dışında kendiliğinden çalıştırabilir.";

pub const AYNI_ADIM_UYARISI: &str = "Bu dosya aynı adımda bir komutla birlikte yazılıyor; \
komut bu değişikliği çalıştırabilir.";

/// Komutların kod olarak çalıştırdığı derleme ve betik dosyaları (güvenlik
/// analizi Bulgu 3). Dosya adına göre, büyük-küçük harf duyarsız eşleşir.
pub const DERLEME_DOSYALARI: &[&str] = &[
    "build.rs",
    "Cargo.toml",
    "package.json",
    "Makefile",
    "GNUmakefile",
    "CMakeLists.txt",
    "setup.py",
    "pyproject.toml",
    "conftest.py",
    "noxfile.py",
    "tox.ini",
    "justfile",
    "Taskfile.yml",
    "build.gradle",
    "build.gradle.kts",
    "settings.gradle",
    "pom.xml",
];

pub fn derleme_dosyasi_mi(yol: &CalismaYolu) -> bool {
    let ad = yol.as_str().rsplit('/').next().unwrap_or("");
    DERLEME_DOSYALARI.iter().any(|d| d.eq_ignore_ascii_case(ad))
}

/// Kendiliğinden çalışan yapılandırma (güvenlik analizi Bulgu 2): IDE, git,
/// CI ve paket yöneticileri bu dosyaları Kalfa'nın yalıtımı dışında ve çoğu
/// zaman sorulmadan çalıştırır. Her kalıp her derinlikte ve büyük-küçük harf
/// duyarsız eşleşir. Bilinen adlar listesidir: sınıf yalnızca onay ister,
/// hiçbir şeyi serbest bırakmaz.
pub const VARSAYILAN_YAPILANDIRMA_KALIPLARI: &[&str] = &[
    // IDE ve geliştirme ortamı
    ".vscode/**",
    ".idea/**",
    ".devcontainer/**",
    // Araç zinciri
    ".cargo/config",
    ".cargo/config.toml",
    "rust-toolchain",
    "rust-toolchain.toml",
    "mise.toml",
    ".mise.toml",
    // Ortam
    ".envrc",
    // CI
    ".github/workflows/**",
    ".gitlab-ci.yml",
    // Commit kancası yöneticileri
    ".pre-commit-config.yaml",
    ".husky/**",
    "lefthook.yml",
    ".lefthook.yml",
    // Paket yöneticileri
    ".yarnrc",
    ".yarnrc.yml",
    ".pnpmfile.cjs",
];

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
    // Güvenlik analizi Bulgu 4. Kaynak kod adlarını yakalamamak için
    // `secrets.*` gibi geniş kalıplar kullanılmaz.
    "*.env",
    "*.tfstate",
    "*.tfstate.*",
    "*.tfvars",
    ".terraformrc",
    "terraform.rc",
    "secrets.{json,yaml,yml,toml,env,ini}",
    "secret.{json,yaml,yml,toml,env,ini}",
    "service-account*.json",
    "serviceaccount*.json",
    ".pgpass",
    ".htpasswd",
    ".vault-token",
    ".s3cfg",
    ".boto",
    ".docker/config.json",
    ".dockercfg",
    "kubeconfig",
    ".kube/config",
    ".yarnrc.yml",
    "*.jks",
    "*.ovpn",
    "*.ppk",
    "*.keytab",
    "id_ecdsa*",
    "id_dsa*",
];

/// Paket yöneticilerinin ürettiği bağımlılık dizinleri (2d-3, kullanıcı
/// kararı). İçlerindeki `.pem`, `credentials*` gibi dosyalar kitaplık
/// dosyasıdır (pip'in `cacert.pem`'i, npm paketlerinin `credentials.js`'i);
/// gizlenirlerse araçlar bozulur. Bedeli: kullanıcı bu dizinlere kendisi bir
/// sır koyarsa gizlenmez.
pub const BAGIMLILIK_DIZINLERI: &[&str] = &["node_modules", "site-packages", "dist-packages"];

/// Yolun bir üst dizini bağımlılık dizini mi (büyük-küçük harf duyarsız).
pub fn bagimlilik_dizininde_mi(yol: &CalismaYolu) -> bool {
    let mut parcalar: Vec<&str> = yol.as_str().split('/').collect();
    parcalar.pop();
    parcalar.iter().any(|parca| {
        BAGIMLILIK_DIZINLERI
            .iter()
            .any(|ad| parca.eq_ignore_ascii_case(ad))
    })
}

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
    /// Kendiliğinden çalışan yapılandırma (Bulgu 2).
    Yapilandirma,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Karar {
    Serbest,
    Onayli {
        uyarilar: Vec<String>,
    },
    /// Onay yalnızca yerel iletişim kutusundan verilebilir (aşama 2 tasarımı
    /// §5.2): yalıtımsız kabuk çağrısı.
    YerelOnayli {
        uyarilar: Vec<String>,
    },
    Yasak {
        neden: String,
    },
}

fn uyari_birlestir(mut a: Vec<String>, b: Vec<String>) -> Vec<String> {
    for u in b {
        if !a.contains(&u) {
            a.push(u);
        }
    }
    a
}

impl Karar {
    /// Birden çok hedefin kararını birleştirir: Yasak > Yerel onaylı > Onaylı
    /// > Serbest. Uyarılar birleşir.
    pub fn birlestir(self, diger: Karar) -> Karar {
        match (self, diger) {
            (Karar::Yasak { neden }, _) | (_, Karar::Yasak { neden }) => Karar::Yasak { neden },
            (Karar::YerelOnayli { uyarilar: a }, Karar::YerelOnayli { uyarilar: b })
            | (Karar::YerelOnayli { uyarilar: a }, Karar::Onayli { uyarilar: b })
            | (Karar::Onayli { uyarilar: a }, Karar::YerelOnayli { uyarilar: b }) => {
                Karar::YerelOnayli {
                    uyarilar: uyari_birlestir(a, b),
                }
            }
            (Karar::YerelOnayli { uyarilar }, Karar::Serbest)
            | (Karar::Serbest, Karar::YerelOnayli { uyarilar }) => Karar::YerelOnayli { uyarilar },
            (Karar::Onayli { uyarilar: a }, Karar::Onayli { uyarilar: b }) => Karar::Onayli {
                uyarilar: uyari_birlestir(a, b),
            },
            (Karar::Onayli { uyarilar }, Karar::Serbest)
            | (Karar::Serbest, Karar::Onayli { uyarilar }) => Karar::Onayli { uyarilar },
            (Karar::Serbest, Karar::Serbest) => Karar::Serbest,
        }
    }
}

/// Kabuk çağrısının kararı (aşama 2 tasarımı §5.1).
///
/// - Yalıtımsız çağrı her kipte yalnızca yerel pencerede onaylanır.
/// - Yalıtımlı çağrı Serbest kipte, ağsızsa ve çalışma alanındaki ilk çağrı
///   değilse onay istemez; diğer kiplerde onay ister.
/// - İnceleme kipinde kabuk yoktur; araç modele zaten sunulmaz.
pub fn kabuk_karari(kip: Kip, tarif: &KabukTarifi) -> Karar {
    if kip == Kip::Inceleme {
        return Karar::Yasak {
            neden: "İnceleme kipinde komut çalıştırılamaz.".to_string(),
        };
    }
    let mut uyarilar = Vec::new();
    if tarif.ilk_cagri {
        uyarilar.push(ILK_KABUK_UYARISI.to_string());
    }
    match &tarif.yalitim {
        YalitimOzeti::Yalitimsiz { talimat } => {
            uyarilar.push(YALITIMSIZ_UYARISI.to_string());
            uyarilar.push(talimat.clone());
            Karar::YerelOnayli { uyarilar }
        }
        YalitimOzeti::Yalitimli { .. } => {
            if tarif.ag {
                uyarilar.push(AGLI_KABUK_UYARISI.to_string());
            }
            // Hazırlık çalışma alanının izinlerini değiştirir (aşama 2
            // tasarımı §9.4): her kipte yerel pencerede sorulur.
            if tarif.alan_hazirligi {
                uyarilar.push(ALAN_HAZIRLIGI_UYARISI.to_string());
                return Karar::YerelOnayli { uyarilar };
            }
            if kip == Kip::Serbest && uyarilar.is_empty() {
                Karar::Serbest
            } else {
                Karar::Onayli { uyarilar }
            }
        }
    }
}

pub fn karar_ver(kip: Kip, islem: Islem, sinif: YolSinifi, coklu_baglanti: bool) -> Karar {
    match islem {
        Islem::Okuma => match sinif {
            YolSinifi::Gizli => Karar::Onayli {
                uyarilar: vec![GIZLI_UYARISI.to_string()],
            },
            YolSinifi::Normal | YolSinifi::Korumali | YolSinifi::Yapilandirma => Karar::Serbest,
        },
        Islem::Yazma => {
            if kip == Kip::Inceleme {
                return Karar::Yasak {
                    neden: "İnceleme kipinde dosya yazılamaz.".to_string(),
                };
            }
            let mut uyarilar = Vec::new();
            match sinif {
                YolSinifi::Korumali => uyarilar.push(KORUMALI_UYARISI.to_string()),
                YolSinifi::Yapilandirma => uyarilar.push(YAPILANDIRMA_UYARISI.to_string()),
                YolSinifi::Normal | YolSinifi::Gizli => {}
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

/// Kalıplar her derinlikte ve büyük-küçük harf duyarsız eşleşir.
fn kalip_kumesi(kaliplar: impl Iterator<Item = String>) -> Result<GlobSet, globset::Error> {
    let mut kurucu = GlobSetBuilder::new();
    for kalip in kaliplar {
        kurucu.add(
            GlobBuilder::new(&format!("**/{kalip}"))
                .case_insensitive(true)
                .literal_separator(true)
                .build()?,
        );
    }
    kurucu.build()
}

/// Bir yolun sınıfını (gizli / korumalı / yapılandırma) belirler.
#[derive(Debug)]
pub struct Siniflandirici {
    kok: PathBuf,
    gizli: GlobSet,
    yapilandirma: GlobSet,
}

impl Siniflandirici {
    pub fn yeni(kok: impl Into<PathBuf>, ek_gizli: &[String]) -> Result<Self, globset::Error> {
        let gizli = kalip_kumesi(
            VARSAYILAN_GIZLI_KALIPLAR
                .iter()
                .map(|k| k.to_string())
                .chain(ek_gizli.iter().cloned()),
        )?;
        let yapilandirma = kalip_kumesi(
            VARSAYILAN_YAPILANDIRMA_KALIPLARI
                .iter()
                .map(|k| k.to_string()),
        )?;
        Ok(Self {
            kok: kok.into(),
            gizli,
            yapilandirma,
        })
    }

    /// Kendiliğinden çalışan yapılandırma mı (Bulgu 2)? Bağımlılık dizinleri
    /// muaf değildir: oraya yazılan bir `.vscode` de çalışabilir.
    pub fn yapilandirma_mi(&self, yol: &CalismaYolu) -> bool {
        self.yapilandirma.is_match(yol.as_str())
    }

    /// İçindeki her şey yapılandırma sayılan dizin mi (`.vscode`,
    /// `.github/workflows`)? Yalıtımda bütünüyle salt okunur olur.
    pub fn yapilandirma_dizini_mi(&self, yol: &CalismaYolu) -> bool {
        self.yapilandirma
            .is_match(format!("{}/kalfa-ornek", yol.as_str()))
    }

    pub fn kok(&self) -> &Path {
        &self.kok
    }

    /// Bağımlılık dizinlerinin altındaki yollar gizli sayılmaz; tarama yine de
    /// oralara iner ve gizli dosyaların hard link'lerini bulur.
    pub fn gizli_mi(&self, yol: &CalismaYolu) -> bool {
        !bagimlilik_dizininde_mi(yol) && self.gizli.is_match(yol.as_str())
    }

    /// Git korumaları her çağrıda yeniden çözülür (spec §7.3): `core.hooksPath`
    /// oturum sırasında değişebilir. Her derinlikteki `.git` her zaman
    /// korumalıdır.
    pub fn korumali_mi(&self, yol: &CalismaYolu) -> bool {
        if git_koruma::git_ic_yolu_mu(yol) {
            return true;
        }
        let anahtar = yol.anahtar();
        git_koruma::korumali_yollar(&self.kok).iter().any(|k| {
            let k = k.trim_end_matches('/');
            anahtar == k || anahtar.starts_with(&format!("{k}/"))
        })
    }

    /// Öncelik: Korumalı > Yapılandırma > Gizli > Normal. Okuma kararı
    /// `gizli_mi`'yi doğrudan kullanır; bu öncelik gizli dosyanın okuma
    /// onayını etkilemez.
    pub fn sinifla(&self, yol: &CalismaYolu) -> YolSinifi {
        if self.korumali_mi(yol) {
            YolSinifi::Korumali
        } else if self.yapilandirma_mi(yol) {
            YolSinifi::Yapilandirma
        } else if self.gizli_mi(yol) {
            YolSinifi::Gizli
        } else {
            YolSinifi::Normal
        }
    }
}
