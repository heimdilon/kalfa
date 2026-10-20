//! İzin motoru (spec §7.2–§7.5). Bütün izin kararları buradan geçer.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};

use crate::arac::{McpAgKarti, McpErisim, WebTarifi, YonlendirmeIzni};
use crate::git_koruma;
use crate::kabuk::{KabukTarifi, YalitimOzeti};
use crate::yol::CalismaYolu;

pub const GIZLI_UYARISI: &str = "Bu dosyanın içeriği EVREN'e gönderilecek.";
/// Önceki hâli saklanamayan yazma her kipte onay ister (aşama 3 tasarımı
/// §5.3, K9): onaysız yazmanın güvenlik ağı geri almadır.
pub const GERI_ALINAMAZ_UYARISI: &str = "Bu dosyanın önceki hâli geri alma için saklanamayacak kadar büyük (8 MiB üstü); bu değişiklik geri alınamaz.";
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
pub const MCP_ILK_CAGRI_UYARISI: &str = "Bu MCP sunucusu bu çalışma alanında ilk kez çağrılacak.";
pub const MCP_AG_UYARISI: &str = "Bu MCP sunucusu ağ erişimi kullanabilecek.";
pub const MCP_TAM_ERISIM_UYARISI: &str =
    "MCP sunucusu yalıtım olmadan, hesabınızın yetkileriyle çalışacak.";
pub const WEB_AG_UYARISI: &str = "Bu işlem dış bir web adresine HTTP/HTTPS isteği gönderecek.";
pub const WEB_ILK_HOST_UYARISI: &str = "Bu web hostuna bu oturumda ilk kez erişilecek.";
pub const WEB_YONLENDIRME_UYARISI: &str = "Bu istek farklı bir hosta yönlendirmeye izin veriyor.";
pub const WEB_SIFRESIZ_HTTP_UYARISI: &str =
    "Bağlantı şifresiz HTTP kullanıyor; ağ trafiği üçüncü taraflarca görülebilir.";
pub const YAPILANDIRMA_UYARISI: &str = "Bu dosyayı IDE'niz, git ya da paket yöneticiniz \
Kalfa'nın yalıtımı dışında kendiliğinden çalıştırabilir.";

pub const AYNI_ADIM_UYARISI: &str = "Bu dosya aynı adımda bir komutla birlikte yazılıyor; \
komut bu değişikliği çalıştırabilir.";
pub const MCP_TAZE_ONAY_UYARISI: &str =
    "MCP hazırlığı onaydan sonra değişti; güncel kartı yeniden onaylayın.";

pub const HAKEME_UYGUN_UYARILAR: &[&str] = &[
    ILK_KABUK_UYARISI,
    AGLI_KABUK_UYARISI,
    AYNI_ADIM_UYARISI,
    MCP_ILK_CAGRI_UYARISI,
    MCP_AG_UYARISI,
    MCP_TAZE_ONAY_UYARISI,
];

pub fn hakeme_uygun(uyarilar: &[String], yerel: bool) -> bool {
    !yerel
        && uyarilar
            .iter()
            .all(|u| HAKEME_UYGUN_UYARILAR.contains(&u.as_str()))
}

pub const OTOMATIK_EMIN_DEGIL_UYARISI: &str = "Otomatik kip bu işlemden emin olamadı; karar sizin.";

pub fn otomatik_riskli_uyarisi(risk_aciklamasi: &str) -> String {
    format!("Otomatik kip bu işlemi riskli buldu: {risk_aciklamasi}. Karar sizin.")
}

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
    Otomatik,
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
            if matches!(kip, Kip::Serbest | Kip::Otomatik) && uyarilar.is_empty() {
                Karar::Serbest
            } else {
                Karar::Onayli { uyarilar }
            }
        }
    }
}

/// Onay kartında gösterilen ağ satırları (aşama 3 tasarımı §8.4, §8.7).
///
/// Yalıtımsızda "Ağ: sınırsız (yalıtımsız)".
/// Yalıtımlıda varsayılan indirme uçları ve her ek host ayrı bir satırdır;
/// özel ağ işaretli kullanıcı hostunun yanında "(özel ağ)" yazar.
pub fn kart_ag_satirlari(tarif: &KabukTarifi, kullanici_hostlari: &[(&str, bool)]) -> Vec<String> {
    if !tarif.ag {
        return vec!["Ağ: kapalı".to_string()];
    }
    if !tarif.yalitim.yalitimli_mi() {
        return vec!["Ağ: sınırsız (yalıtımsız)".to_string()];
    }

    let mut ek_satirlar = Vec::new();
    let mut gorulen = std::collections::HashSet::new();

    // Önce kullanıcı listesi (özel ağ işaretli olabilir)
    for (host, ozel) in kullanici_hostlari {
        let h = host.strip_suffix(":443").unwrap_or(host);
        if gorulen.insert(h.to_string()) {
            if *ozel {
                ek_satirlar.push(format!("{h} (özel ağ)"));
            } else {
                ek_satirlar.push(h.to_string());
            }
        }
    }

    // Sonra çağrıya özel hostlar (tarif.ag_hostlari)
    for host in &tarif.ag_hostlari {
        let h = host.strip_suffix(":443").unwrap_or(host);
        if gorulen.insert(h.to_string()) {
            ek_satirlar.push(h.to_string());
        }
    }

    if ek_satirlar.is_empty() {
        vec!["Ağ: açık; paket kayıtları (crates.io, npm, PyPI), GitHub indirmeleri".to_string()]
    } else {
        let mut satirlar = Vec::with_capacity(1 + ek_satirlar.len());
        satirlar.push(
            "Ağ: açık; paket kayıtları (crates.io, npm, PyPI), GitHub indirmeleri ve şu adresler:"
                .to_string(),
        );
        satirlar.extend(ek_satirlar);
        satirlar
    }
}

pub fn ag_satirlari(tarif: &KabukTarifi) -> Vec<String> {
    kart_ag_satirlari(tarif, &[])
}

pub fn karar_ver(kip: Kip, islem: Islem, sinif: YolSinifi, coklu_baglanti: bool) -> Karar {
    match islem {
        Islem::Okuma => {
            if sinif == YolSinifi::Gizli || coklu_baglanti {
                Karar::Onayli {
                    uyarilar: vec![GIZLI_UYARISI.to_string()],
                }
            } else {
                Karar::Serbest
            }
        }
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

/// MCP çağrısının izin kararı (11a-2 tasarımı §5.2). Tam erişim her çağrıda
/// yerel onay ister; Serbest kipte yalıtımlı çağrı yalnız ilk kullanım veya
/// ağ erişimi varsa oturum onayına yükselir.
pub fn mcp_karari(kip: Kip, erisim: McpErisim, ilk_cagri: bool, ag: &McpAgKarti) -> Karar {
    match (kip, erisim) {
        (Kip::Inceleme, _) => Karar::Yasak {
            neden: "İnceleme kipinde MCP araçları sunulmaz.".to_string(),
        },
        (_, McpErisim::TamErisim) => Karar::YerelOnayli {
            uyarilar: vec![MCP_TAM_ERISIM_UYARISI.to_string()],
        },
        (Kip::Siki | Kip::Proje, McpErisim::Yalitimli) => Karar::Onayli {
            uyarilar: Vec::new(),
        },
        (Kip::Serbest | Kip::Otomatik, McpErisim::Yalitimli) => {
            let mut uyarilar = Vec::new();
            if ilk_cagri {
                uyarilar.push(MCP_ILK_CAGRI_UYARISI.to_string());
            }
            if ag.ag_kullaniliyor() {
                uyarilar.push(MCP_AG_UYARISI.to_string());
            }
            if uyarilar.is_empty() {
                Karar::Serbest
            } else {
                Karar::Onayli { uyarilar }
            }
        }
    }
}

/// `web_getir` çağrısının izin kararı (Yetenek-05 planı G4a ve SP-05).
///
/// - `Kip::Inceleme`: her zaman `Karar::Yasak`.
/// - `Kip::Siki`, `Kip::Proje`, `Kip::Serbest`, `Kip::Otomatik`: `web_getir`
///   hiçbir kipte otomatik (`Karar::Serbest`) çalışmaz; ilk host erişimi,
///   şifresiz HTTP, farklı hosta yönlendirme izni ya da `Serbest`/`Otomatik`
///   kipinde `Karar::YerelOnayli` ister.
pub fn web_karari(kip: Kip, tarif: &WebTarifi) -> Karar {
    if kip == Kip::Inceleme {
        return Karar::Yasak {
            neden: "İnceleme kipinde dış ağ isteği (`web_getir`) çalıştırılamaz.".into(),
        };
    }

    if tarif.sema != "https" && tarif.sema != "http" {
        return Karar::Yasak {
            neden: format!("Desteklenmeyen web şeması: {}", tarif.sema),
        };
    }

    if tarif.yontem != "GET" {
        return Karar::Yasak {
            neden: format!("Yalnızca GET yöntemine izin verilir: {}", tarif.yontem),
        };
    }

    let mut uyarilar = vec![WEB_AG_UYARISI.to_string()];
    let mut yerel_onay_gerekli = false;

    if tarif.oturumda_ilk_host {
        uyarilar.push(WEB_ILK_HOST_UYARISI.to_string());
        yerel_onay_gerekli = true;
    }
    if tarif.sema == "http" {
        uyarilar.push(WEB_SIFRESIZ_HTTP_UYARISI.to_string());
        yerel_onay_gerekli = true;
    }
    if let YonlendirmeIzni::IzinliHostlar(ek_hostlar) = &tarif.yonlendirme_izni
        && !ek_hostlar.is_empty()
    {
        uyarilar.push(WEB_YONLENDIRME_UYARISI.to_string());
        yerel_onay_gerekli = true;
    }

    if yerel_onay_gerekli || matches!(kip, Kip::Serbest | Kip::Otomatik) {
        Karar::YerelOnayli { uyarilar }
    } else {
        Karar::Onayli { uyarilar }
    }
}

#[cfg(test)]
mod mcp_izin_testleri {
    use super::*;

    fn yerel(uyari: &str) -> Karar {
        Karar::YerelOnayli {
            uyarilar: vec![uyari.to_string()],
        }
    }

    #[test]
    fn mcp_izin_matrisi_dort_kip_iki_erisim() {
        let ag = McpAgKarti::Kapali;
        let onay = Karar::Onayli {
            uyarilar: Vec::new(),
        };
        for (kip, yalitimli, tam_erisim) in [
            (
                Kip::Inceleme,
                Karar::Yasak {
                    neden: "İnceleme kipinde MCP araçları sunulmaz.".into(),
                },
                Karar::Yasak {
                    neden: "İnceleme kipinde MCP araçları sunulmaz.".into(),
                },
            ),
            (Kip::Siki, onay.clone(), yerel(MCP_TAM_ERISIM_UYARISI)),
            (Kip::Proje, onay.clone(), yerel(MCP_TAM_ERISIM_UYARISI)),
            (Kip::Serbest, Karar::Serbest, yerel(MCP_TAM_ERISIM_UYARISI)),
        ] {
            assert_eq!(
                mcp_karari(kip, McpErisim::Yalitimli, false, &ag),
                yalitimli,
                "yalıtımlı: {kip:?}"
            );
            assert_eq!(
                mcp_karari(kip, McpErisim::TamErisim, false, &ag),
                tam_erisim,
                "tam erişim: {kip:?}"
            );
        }
    }

    #[test]
    fn serbest_mcp_ilk_cagri_ve_ag_icin_onay_ister() {
        assert_eq!(
            mcp_karari(
                Kip::Serbest,
                McpErisim::Yalitimli,
                true,
                &McpAgKarti::Kapali
            ),
            Karar::Onayli {
                uyarilar: vec![MCP_ILK_CAGRI_UYARISI.to_string()]
            }
        );
        assert_eq!(
            mcp_karari(
                Kip::Serbest,
                McpErisim::Yalitimli,
                false,
                &McpAgKarti::Sinirsiz
            ),
            Karar::Onayli {
                uyarilar: vec![MCP_AG_UYARISI.to_string()]
            }
        );
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

/// Bir kalıp listesinde en çok bu kadar kalıp olur (3e).
pub const AZAMI_KALIP: usize = 200;
/// Bir kalıbın en uzun hâli (karakter).
pub const AZAMI_KALIP_UZUNLUGU: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KalipHatasi {
    #[error("Kalıp boş olamaz.")]
    Bos,
    #[error("`{0}`: kalıp çalışma alanına göre yazılır; mutlak yol olamaz.")]
    Mutlak(String),
    #[error("`{0}`: kalıp `..` içeremez.")]
    UstDizin(String),
    #[error("`{0}`: kalıp en çok {AZAMI_KALIP_UZUNLUGU} karakter olabilir.")]
    Uzun(String),
    #[error("`{0}`: yalnızca jokerden oluşan bir istisna bütün dosyaları açardı.")]
    YalnizJoker(String),
    #[error("Bir listede en çok {AZAMI_KALIP} kalıp olabilir.")]
    CokKalip,
    #[error("`{0}`: geçersiz kalıp ({1}).")]
    Gecersiz(String, String),
}

/// Kullanıcının bir kalıbını doğrular (3e planı, Global Constraints).
/// `istisna`: gizli sayılmayanlar listesi için; yalnızca jokerden oluşan
/// istisna reddedilir.
pub fn kalip_dogrula(k: &str, istisna: bool) -> Result<(), KalipHatasi> {
    let k = k.trim();
    if k.is_empty() {
        return Err(KalipHatasi::Bos);
    }
    if k.chars().count() > AZAMI_KALIP_UZUNLUGU {
        return Err(KalipHatasi::Uzun(k.to_string()));
    }
    let duz = k.replace('\\', "/");
    let bayt = duz.as_bytes();
    if duz.starts_with('/') || (bayt.len() >= 2 && bayt[1] == b':') {
        return Err(KalipHatasi::Mutlak(k.to_string()));
    }
    if duz.split('/').any(|p| p == "..") {
        return Err(KalipHatasi::UstDizin(k.to_string()));
    }
    if istisna && duz.chars().all(|c| matches!(c, '*' | '?' | '.' | '/')) {
        return Err(KalipHatasi::YalnizJoker(k.to_string()));
    }
    kalip_kumesi(std::iter::once(duz))
        .map(|_| ())
        .map_err(|e| KalipHatasi::Gecersiz(k.to_string(), e.kind().to_string()))
}

#[derive(Debug, Default)]
struct GizliKumeleri {
    ek_kaliplar: Vec<String>,
    istisnalar: Vec<String>,
    ek: GlobSet,
    istisna: GlobSet,
}

/// Kullanıcının ek gizli kalıpları ve gizli sayılmayanları (aşama 3
/// tasarımı §9.2, K7). Uygulama genelinde paylaşılır; değişiklik açık
/// oturumlarda ve yalıtımda hemen geçerlidir. Varsayılan kalıplar burada
/// değildir ve kapatılamaz.
#[derive(Debug, Default)]
pub struct GizliAyarlari {
    ic: RwLock<GizliKumeleri>,
}

impl GizliAyarlari {
    pub fn yeni(ek: &[String], istisna: &[String]) -> Result<Self, KalipHatasi> {
        Ok(Self {
            ic: RwLock::new(kumeler(ek, istisna)?),
        })
    }

    /// İki listeyi birden değiştirir. Doğrulama geçmezse eski hâl kalır.
    pub fn degistir(&self, ek: &[String], istisna: &[String]) -> Result<(), KalipHatasi> {
        let yeni = kumeler(ek, istisna)?;
        *self.ic.write().unwrap_or_else(|z| z.into_inner()) = yeni;
        Ok(())
    }

    pub fn ek_kaliplar(&self) -> Vec<String> {
        self.ic
            .read()
            .unwrap_or_else(|z| z.into_inner())
            .ek_kaliplar
            .clone()
    }

    pub fn istisnalar(&self) -> Vec<String> {
        self.ic
            .read()
            .unwrap_or_else(|z| z.into_inner())
            .istisnalar
            .clone()
    }
}

fn kumeler(ek: &[String], istisna: &[String]) -> Result<GizliKumeleri, KalipHatasi> {
    if ek.len() > AZAMI_KALIP || istisna.len() > AZAMI_KALIP {
        return Err(KalipHatasi::CokKalip);
    }
    let duzelt = |l: &[String], istisna: bool| -> Result<Vec<String>, KalipHatasi> {
        l.iter()
            .map(|k| {
                kalip_dogrula(k, istisna)?;
                Ok(k.trim().replace('\\', "/"))
            })
            .collect()
    };
    let ek_kaliplar = duzelt(ek, false)?;
    let istisnalar = duzelt(istisna, true)?;
    let hata = |e: globset::Error| KalipHatasi::Gecersiz(String::new(), e.kind().to_string());
    Ok(GizliKumeleri {
        ek: kalip_kumesi(ek_kaliplar.iter().cloned()).map_err(hata)?,
        istisna: kalip_kumesi(istisnalar.iter().cloned()).map_err(hata)?,
        ek_kaliplar,
        istisnalar,
    })
}

/// Bir yolun sınıfını (gizli / korumalı / yapılandırma) belirler.
#[derive(Debug)]
pub struct Siniflandirici {
    kok: PathBuf,
    gizli: GlobSet,
    yapilandirma: GlobSet,
    /// Kullanıcının ek kalıpları ve istisnaları (3e); her sınıflandırmada
    /// okunur.
    ayarlar: Arc<GizliAyarlari>,
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
            ayarlar: Arc::default(),
        })
    }

    /// Kullanıcının paylaşılan gizli ayarlarıyla (3e). Ayarlar değişince
    /// sınıflandırıcı yeniden kurulmaz.
    pub fn yeni_ayarla(
        kok: impl Into<PathBuf>,
        ayarlar: Arc<GizliAyarlari>,
    ) -> Result<Self, globset::Error> {
        Ok(Self {
            ayarlar,
            ..Self::yeni(kok, &[])?
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
    /// Yolun kendisi ya da bir üst dizini gizli mi? Her aday ayrı
    /// değerlendirilir: bir gizli kalıba (varsayılan ya da ek) uyup hiçbir
    /// istisnaya uymuyorsa yol gizlidir. Dosyaya verilen istisna gizli üst
    /// dizini açmaz (3e planı, güvenlik serisinden sonra).
    pub fn gizli_mi(&self, yol: &CalismaYolu) -> bool {
        if bagimlilik_dizininde_mi(yol) {
            return false;
        }
        let ayar = self.ayarlar.ic.read().unwrap_or_else(|z| z.into_inner());
        // Gizli bir dizinin adı, altındaki normal dosya adıyla kaybolmaz.
        let mut aday = yol.as_str();
        loop {
            if (self.gizli.is_match(aday) || ayar.ek.is_match(aday)) && !ayar.istisna.is_match(aday)
            {
                return true;
            }
            match aday.rsplit_once('/') {
                Some((ust, _)) => aday = ust,
                None => return false,
            }
        }
    }

    /// Git korumaları her çağrıda yeniden çözülür (spec §7.3): `core.hooksPath`
    /// oturum sırasında değişebilir. Her derinlikteki `.git` her zaman
    /// korumalıdır.
    pub fn korumali_mi(&self, yol: &CalismaYolu) -> bool {
        korumali_listede(yol, &git_koruma::korumali_yollar(&self.kok))
    }

    /// Öncelik: Korumalı > Yapılandırma > Gizli > Normal. Okuma kararı
    /// `gizli_mi`'yi doğrudan kullanır; bu öncelik gizli dosyanın okuma
    /// onayını etkilemez.
    pub fn sinifla(&self, yol: &CalismaYolu) -> YolSinifi {
        self.sinifla_ile(yol, self.korumali_mi(yol))
    }

    fn sinifla_ile(&self, yol: &CalismaYolu, korumali: bool) -> YolSinifi {
        if korumali {
            YolSinifi::Korumali
        } else if self.yapilandirma_mi(yol) {
            YolSinifi::Yapilandirma
        } else if self.gizli_mi(yol) {
            YolSinifi::Gizli
        } else {
            YolSinifi::Normal
        }
    }

    /// Birçok yolu aynı anda sınıflandırmak için: git korumaları bir kez
    /// çözülür. Korumaların çözülmesi `git config` çalıştırır; yol başına
    /// çözmek büyük bir geri alma planında saniyeler sürerdi.
    pub fn toplu(&self) -> TopluSiniflandirici<'_> {
        TopluSiniflandirici {
            s: self,
            korumali: git_koruma::korumali_yollar(&self.kok),
        }
    }

    /// `toplu`'nun async bağlamdaki hâli: `git config` tokio'nun engelleyen
    /// iş parçacıklarında çalışır. Oturum görevi ve aynı adımda paralel
    /// yürüyen araçlar onu beklerken durmaz.
    pub async fn toplu_engellemeden(&self) -> TopluSiniflandirici<'_> {
        let kok = self.kok.clone();
        let korumali =
            match tokio::task::spawn_blocking(move || git_koruma::korumali_yollar(&kok)).await {
                Ok(k) => k,
                // Görev çalışamadıysa (çalışma zamanı kapanıyor) korumalar
                // burada çözülür: karar korumasız verilmez.
                Err(_) => git_koruma::korumali_yollar(&self.kok),
            };
        TopluSiniflandirici { s: self, korumali }
    }
}

/// Git korumaları bir anda çözülmüş sınıflandırıcı (`Siniflandirici::toplu`).
pub struct TopluSiniflandirici<'a> {
    s: &'a Siniflandirici,
    korumali: Vec<String>,
}

impl TopluSiniflandirici<'_> {
    pub fn sinifla(&self, yol: &CalismaYolu) -> YolSinifi {
        self.s
            .sinifla_ile(yol, korumali_listede(yol, &self.korumali))
    }

    pub fn gizli_mi(&self, yol: &CalismaYolu) -> bool {
        self.s.gizli_mi(yol)
    }
}

/// Her derinlikteki `.git` ya da listedeki bir korumalı yolun altı.
/// `korumali`: [`git_koruma::korumali_yollar`]'ın sonucu.
pub fn korumali_listede(yol: &CalismaYolu, korumali: &[String]) -> bool {
    if git_koruma::git_ic_yolu_mu(yol) {
        return true;
    }
    let anahtar = yol.anahtar();
    korumali.iter().any(|k| {
        let k = k.trim_end_matches('/');
        k == "." || anahtar == k || anahtar.starts_with(&format!("{k}/"))
    })
}
