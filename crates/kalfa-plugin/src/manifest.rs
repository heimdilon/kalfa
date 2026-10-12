//! `kalfa.toml` ve ad kuralları (aşama 4 tasarımı §6.4, §6.5).
//!
//! Bilinmeyen alan reddedilir: `kabuk = true` ya da `ag = [...]` yazan bir
//! manifest, yetki aldığını sanan bir eklenti üretmez.

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::Deserialize;

/// Kalfa'nın yüklediği tek sözleşme sürümü.
pub const SOZLESME: &str = "0.1";
pub const AZAMI_GLOB: usize = 64;
pub const AZAMI_DEPO_MB: u32 = 1024;
pub const AZAMI_ACIKLAMA: usize = 200;

const EKLENTI_ALANLARI: &[&str] = &["ad", "surum", "sozlesme", "aciklama"];
const YETENEK_ALANLARI: &[&str] = &["dosya-oku", "evren", "depo-mb"];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManifestHatasi {
    #[error("kalfa.toml okunamadı: {0}")]
    Bicim(String),
    #[error(
        "Bu eklenti Kalfa eklenti sözleşmesinin {0} sürümü için yazılmış. Bu Kalfa {SOZLESME}'i destekliyor. Eklentinin {SOZLESME} için derlenmiş sürümünü kurun ya da Kalfa'yı güncelleyin."
    )]
    Sozlesme(String),
    #[error(
        "kalfa.toml'daki `{0}` alanı Kalfa eklenti sözleşmesi {SOZLESME}'de yok. Eklentiler bu sürümde dosya yazamaz, komut çalıştıramaz ve ağa doğrudan çıkamaz."
    )]
    BilinmeyenAlan(String),
    #[error(
        "Eklenti adı `{0}` geçersiz: küçük harfle başlamalı, 2–24 karakter olmalı, yalnızca küçük harf, rakam ve `-` içermeli; `--` içeremez."
    )]
    Ad(String),
    #[error("Eklenti sürümü `{0}` geçersiz; `1.2.3` biçiminde olmalı.")]
    Surum(String),
    #[error("Eklenti açıklaması en çok {AZAMI_ACIKLAMA} karakter olabilir.")]
    Aciklama,
    #[error("`dosya-oku` en çok {AZAMI_GLOB} kalıp içerebilir.")]
    CokGlob,
    #[error("`dosya-oku` kalıbı `{0}` geçersiz: {1}")]
    Glob(String, String),
    #[error("`evren` yalnızca `embeddings` ve `rerank` içerebilir; `{0}` yok.")]
    EvrenUcu(String),
    #[error("`depo-mb` 0 ile {AZAMI_DEPO_MB} arasında olmalı; {0} verildi.")]
    DepoMb(i64),
}

/// Eklentiye açılabilecek EVREN uçları (aşama 4 tasarımı §2.2, karar 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvrenUcu {
    Embeddings,
    Rerank,
}

impl EvrenUcu {
    pub fn ad(self) -> &'static str {
        match self {
            EvrenUcu::Embeddings => "embeddings",
            EvrenUcu::Rerank => "rerank",
        }
    }
}

#[derive(Deserialize)]
struct Ham {
    eklenti: HamBilgi,
    #[serde(default)]
    yetenekler: HamYetenekler,
}

#[derive(Deserialize)]
struct HamBilgi {
    ad: String,
    surum: String,
    aciklama: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct HamYetenekler {
    #[serde(default)]
    dosya_oku: Vec<String>,
    #[serde(default)]
    evren: Vec<String>,
    #[serde(default)]
    depo_mb: i64,
}

#[derive(Debug, Clone)]
pub struct Manifest {
    pub ad: String,
    pub surum: semver::Version,
    pub aciklama: String,
    /// `dosya-oku` kalıpları, yazıldığı gibi.
    pub dosya_oku: Vec<String>,
    pub evren: Vec<EvrenUcu>,
    pub depo_mb: u32,
    globlar: GlobSet,
}

impl Manifest {
    pub fn coz(metin: &str) -> Result<Self, ManifestHatasi> {
        let tablo: toml::Table =
            toml::from_str(metin).map_err(|e| ManifestHatasi::Bicim(e.message().to_string()))?;
        // Sözleşme önce: 0.2 için yazılmış bir manifestin yeni alanları
        // "bilinmeyen alan" değil sürüm uyumsuzluğu iletisi almalı.
        let sozlesme = tablo
            .get("eklenti")
            .and_then(|e| e.get("sozlesme"))
            .and_then(|s| s.as_str())
            .ok_or_else(|| ManifestHatasi::Bicim("`[eklenti]` içinde `sozlesme` yok".into()))?;
        if sozlesme != SOZLESME {
            return Err(ManifestHatasi::Sozlesme(sozlesme.to_string()));
        }
        bilinmeyen_alan(&tablo)?;
        let ham: Ham = tablo
            .try_into()
            .map_err(|e: toml::de::Error| ManifestHatasi::Bicim(e.message().to_string()))?;

        let b = ham.eklenti;
        if !eklenti_adi_gecerli(&b.ad) {
            return Err(ManifestHatasi::Ad(b.ad));
        }
        let surum =
            semver::Version::parse(&b.surum).map_err(|_| ManifestHatasi::Surum(b.surum.clone()))?;
        if b.aciklama.chars().count() > AZAMI_ACIKLAMA {
            return Err(ManifestHatasi::Aciklama);
        }

        let y = ham.yetenekler;
        if y.dosya_oku.len() > AZAMI_GLOB {
            return Err(ManifestHatasi::CokGlob);
        }
        let mut kurucu = GlobSetBuilder::new();
        for kalip in &y.dosya_oku {
            kurucu.add(glob_kur(kalip)?);
        }
        let globlar = kurucu
            .build()
            .map_err(|e| ManifestHatasi::Glob(String::new(), e.to_string()))?;
        let mut evren = Vec::new();
        for uc in &y.evren {
            let uc = match uc.as_str() {
                "embeddings" => EvrenUcu::Embeddings,
                "rerank" => EvrenUcu::Rerank,
                _ => return Err(ManifestHatasi::EvrenUcu(uc.clone())),
            };
            if !evren.contains(&uc) {
                evren.push(uc);
            }
        }
        let depo_mb = u32::try_from(y.depo_mb)
            .ok()
            .filter(|mb| *mb <= AZAMI_DEPO_MB)
            .ok_or(ManifestHatasi::DepoMb(y.depo_mb))?;

        Ok(Self {
            ad: b.ad,
            surum,
            aciklama: b.aciklama,
            dosya_oku: y.dosya_oku,
            evren,
            depo_mb,
            globlar,
        })
    }

    /// Çalışma alanına göre yol manifestin `dosya-oku` globlarından birine
    /// uyuyor mu?
    pub fn okuyabilir(&self, yol: &str) -> bool {
        self.globlar.is_match(yol)
    }

    /// Modelin gördüğü araç adı (§6.5): `kalfa-rag` + `anlamsal_ara` →
    /// `kalfa_rag__anlamsal_ara`.
    pub fn model_adi(&self, arac: &str) -> String {
        format!("{}__{arac}", self.ad.replace('-', "_"))
    }

    /// Kurulumda kullanıcının gördüğü yetenek cümlesi (§6.4, üst belge §8.4).
    pub fn yetenek_cumlesi(&self) -> String {
        let okuma = if self.dosya_oku.is_empty() {
            "Bu eklenti çalışma alanındaki dosyaları okumayacak".to_string()
        } else {
            let globlar = self
                .dosya_oku
                .iter()
                .map(|g| format!("`{g}`"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("Bu eklenti {globlar} ile eşleşen dosyaları okuyacak")
        };
        let evren = match self.evren.as_slice() {
            [] => " ve EVREN'e veri göndermeyecek.".to_string(),
            uclar => {
                let amac = match uclar {
                    [EvrenUcu::Embeddings] => "gömme",
                    [EvrenUcu::Rerank] => "yeniden sıralama",
                    _ => "gömme ve yeniden sıralama",
                };
                let kaynak = if self.dosya_oku.is_empty() {
                    "ürettiği metinleri"
                } else {
                    "bu dosyalardan seçtiği parçaları"
                };
                format!(" ve {kaynak} {amac} için EVREN'e gönderecek.")
            }
        };
        let depo = if self.depo_mb == 0 {
            " Kendi verisini saklamayacak.".to_string()
        } else {
            format!(
                " Kendi verisi için çalışma alanı başına en fazla {} MB yer kullanacak.",
                self.depo_mb
            )
        };
        format!(
            "{okuma}{evren} Gizli dosyaları okuyamaz. Dosya yazamaz, komut çalıştıramaz, internete doğrudan erişemez.{depo}"
        )
    }

    /// Güncellemede `eski`'de olmayıp bunda olan yetenekler (§13.2): yeni
    /// `dosya-oku` kalıpları, yeni EVREN uçları ve büyüyen depo kotası. Boşsa
    /// güncelleme erişimi genişletmez.
    pub fn yeni_yetenekler(&self, eski: &Manifest) -> Vec<String> {
        let mut yeni: Vec<String> = self
            .dosya_oku
            .iter()
            .filter(|g| !eski.dosya_oku.contains(g))
            .map(|g| format!("`{g}` ile eşleşen dosyaları okuma"))
            .collect();
        yeni.extend(
            self.evren
                .iter()
                .filter(|u| !eski.evren.contains(u))
                .map(|u| format!("EVREN `{}` ucuna gönderim", u.ad())),
        );
        if self.depo_mb > eski.depo_mb {
            yeni.push(format!(
                "depo kotası {} MB yerine {} MB",
                eski.depo_mb, self.depo_mb
            ));
        }
        yeni
    }

    /// Güncelleme gönderim izinlerini düşürür mü (§13.2): `evren` uçları ya
    /// da `dosya-oku` kalıpları değiştiyse.
    pub fn gonderim_kapsami_degisti(&self, eski: &Manifest) -> bool {
        let mut a = self.dosya_oku.clone();
        let mut b = eski.dosya_oku.clone();
        a.sort();
        b.sort();
        a != b
            || self.evren.iter().any(|u| !eski.evren.contains(u))
            || eski.evren.iter().any(|u| !self.evren.contains(u))
    }
}

/// Sözleşme 0.1'de olmayan her alan reddedilir.
fn bilinmeyen_alan(tablo: &toml::Table) -> Result<(), ManifestHatasi> {
    for (anahtar, deger) in tablo {
        let izinli = match anahtar.as_str() {
            "eklenti" => EKLENTI_ALANLARI,
            "yetenekler" => YETENEK_ALANLARI,
            _ => return Err(ManifestHatasi::BilinmeyenAlan(anahtar.clone())),
        };
        let Some(alt) = deger.as_table() else {
            return Err(ManifestHatasi::Bicim(format!(
                "`{anahtar}` bir tablo olmalı"
            )));
        };
        if let Some(alan) = alt.keys().find(|k| !izinli.contains(&k.as_str())) {
            return Err(ManifestHatasi::BilinmeyenAlan(format!("{anahtar}.{alan}")));
        }
    }
    Ok(())
}

/// Kalıp çalışma alanının köküne göredir: mutlak yol, `..`, sürücü harfi ve
/// ters bölü reddedilir. `*` yol ayracını geçmez; her derinlik için `**/`.
fn glob_kur(kalip: &str) -> Result<globset::Glob, ManifestHatasi> {
    let hata = |neden: &str| ManifestHatasi::Glob(kalip.to_string(), neden.to_string());
    if kalip.contains('\\') {
        return Err(hata("yol ayracı olarak `/` kullanın"));
    }
    kalfa_core::izin::kalip_dogrula(kalip, false).map_err(|e| hata(&e.to_string()))?;
    GlobBuilder::new(kalip.trim())
        .literal_separator(true)
        .build()
        .map_err(|e| hata(&e.kind().to_string()))
}

/// `^[a-z][a-z0-9-]{1,23}$`, `--` içermez.
pub fn eklenti_adi_gecerli(ad: &str) -> bool {
    let mut k = ad.chars();
    (2..=24).contains(&ad.len())
        && k.next().is_some_and(|c| c.is_ascii_lowercase())
        && k.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !ad.contains("--")
}

/// `^[a-z][a-z0-9_]{0,31}$`, `__` içermez. Yerleşik araçların adında `__`
/// olmadığı için modelin gördüğü ad onlarla çakışamaz.
pub fn arac_adi_gecerli(ad: &str) -> bool {
    let mut k = ad.chars();
    (1..=32).contains(&ad.len())
        && k.next().is_some_and(|c| c.is_ascii_lowercase())
        && k.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !ad.contains("__")
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAM: &str = r#"
[eklenti]
ad = "kalfa-rag"
surum = "0.1.0"
sozlesme = "0.1"
aciklama = "Çalışma alanında Türkçe anlamsal arama"

[yetenekler]
dosya-oku = ["**/*.{md,rs,ts}", "docs/**"]
evren = ["embeddings", "rerank"]
depo-mb = 200
"#;

    fn hata(metin: &str) -> ManifestHatasi {
        Manifest::coz(metin).unwrap_err()
    }

    #[test]
    fn tam_manifest_cozulur() {
        let m = Manifest::coz(TAM).unwrap();
        assert_eq!(m.ad, "kalfa-rag");
        assert_eq!(m.surum, semver::Version::new(0, 1, 0));
        assert_eq!(m.evren, vec![EvrenUcu::Embeddings, EvrenUcu::Rerank]);
        assert_eq!(m.depo_mb, 200);
        assert_eq!(m.model_adi("anlamsal_ara"), "kalfa_rag__anlamsal_ara");
    }

    #[test]
    fn yeteneksiz_manifest_yalnizca_hesap_yapar() {
        let m = Manifest::coz(
            "[eklenti]\nad = \"hesap\"\nsurum = \"1.0.0\"\nsozlesme = \"0.1\"\naciklama = \"x\"\n",
        )
        .unwrap();
        assert!(m.dosya_oku.is_empty() && m.evren.is_empty() && m.depo_mb == 0);
        assert!(!m.okuyabilir("a.rs"));
    }

    #[test]
    fn yetenek_cumlesi_manifestten_uretilir() {
        let m = Manifest::coz(TAM).unwrap();
        assert_eq!(
            m.yetenek_cumlesi(),
            "Bu eklenti `**/*.{md,rs,ts}`, `docs/**` ile eşleşen dosyaları okuyacak ve bu \
             dosyalardan seçtiği parçaları gömme ve yeniden sıralama için EVREN'e gönderecek. \
             Gizli dosyaları okuyamaz. Dosya yazamaz, komut çalıştıramaz, internete doğrudan \
             erişemez. Kendi verisi için çalışma alanı başına en fazla 200 MB yer kullanacak."
        );
        let hesap = Manifest::coz(
            "[eklenti]\nad = \"hesap\"\nsurum = \"1.0.0\"\nsozlesme = \"0.1\"\naciklama = \"x\"\n",
        )
        .unwrap();
        let cumle = hesap.yetenek_cumlesi();
        assert!(cumle.starts_with(
            "Bu eklenti çalışma alanındaki dosyaları okumayacak ve EVREN'e veri göndermeyecek."
        ));
        assert!(cumle.ends_with("Kendi verisini saklamayacak."));
    }

    #[test]
    fn guncellemenin_yeni_yetenekleri_ve_gonderim_kapsami() {
        let eski = Manifest::coz(TAM).unwrap();
        assert!(eski.yeni_yetenekler(&eski).is_empty());
        assert!(!eski.gonderim_kapsami_degisti(&eski));

        let sirasi_degisik = Manifest::coz(&TAM.replace(
            r#"["**/*.{md,rs,ts}", "docs/**"]"#,
            r#"["docs/**", "**/*.{md,rs,ts}"]"#,
        ))
        .unwrap();
        assert!(!sirasi_degisik.gonderim_kapsami_degisti(&eski));

        let genis = Manifest::coz(
            &TAM.replace(r#""docs/**"]"#, r#""docs/**", "**/*.py"]"#)
                .replace("depo-mb = 200", "depo-mb = 300"),
        )
        .unwrap();
        assert_eq!(
            genis.yeni_yetenekler(&eski),
            vec![
                "`**/*.py` ile eşleşen dosyaları okuma".to_string(),
                "depo kotası 200 MB yerine 300 MB".to_string(),
            ]
        );
        assert!(genis.gonderim_kapsami_degisti(&eski));

        let dar = Manifest::coz(&TAM.replace(
            r#"evren = ["embeddings", "rerank"]"#,
            r#"evren = ["embeddings"]"#,
        ))
        .unwrap();
        assert!(
            dar.yeni_yetenekler(&eski).is_empty(),
            "daralma yeni yetenek değil"
        );
        assert!(dar.gonderim_kapsami_degisti(&eski), "uç kümesi değişti");
        assert_eq!(
            eski.yeni_yetenekler(&dar),
            vec!["EVREN `rerank` ucuna gönderim".to_string()]
        );
    }

    #[test]
    fn globlar_kokten_ve_ayraci_gecmeden_eslesir() {
        let m = Manifest::coz(TAM).unwrap();
        assert!(m.okuyabilir("a.rs"));
        assert!(m.okuyabilir("src/derin/b.md"));
        assert!(m.okuyabilir("docs/x/y.txt"));
        assert!(!m.okuyabilir("a.py"));
        assert!(!m.okuyabilir("alt/docs/y.txt"));
    }

    #[test]
    fn baska_sozlesme_surumu_ileti_alir() {
        let h = hata(&TAM.replace("sozlesme = \"0.1\"", "sozlesme = \"0.2\"\nkabuk = true"));
        assert_eq!(h, ManifestHatasi::Sozlesme("0.2".into()));
        assert!(h.to_string().contains("0.2 sürümü için yazılmış"));
    }

    #[test]
    fn bilinmeyen_alan_reddedilir() {
        let h = hata(&TAM.replace("depo-mb = 200", "depo-mb = 200\nkabuk = true"));
        assert_eq!(h, ManifestHatasi::BilinmeyenAlan("yetenekler.kabuk".into()));
        assert!(h.to_string().contains("komut çalıştıramaz"));
        assert_eq!(
            hata(&format!("{TAM}\n[ag]\nhost = \"x\"\n")),
            ManifestHatasi::BilinmeyenAlan("ag".into())
        );
        assert!(matches!(
            hata(&TAM.replace("aciklama =", "yazar = \"x\"\naciklama =")),
            ManifestHatasi::BilinmeyenAlan(a) if a == "eklenti.yazar"
        ));
    }

    #[test]
    fn ad_kurallari() {
        for ad in [
            "a",
            "Kalfa",
            "1abc",
            "kalfa--rag",
            "kalfa_rag",
            &"a".repeat(25),
        ] {
            assert!(!eklenti_adi_gecerli(ad), "{ad}");
        }
        for ad in ["ab", "kalfa-rag", "x9", &"a".repeat(24)] {
            assert!(eklenti_adi_gecerli(ad), "{ad}");
        }
        for ad in ["", "Ara", "1a", "a__b", "a-b", &"a".repeat(33)] {
            assert!(!arac_adi_gecerli(ad), "{ad}");
        }
        for ad in ["a", "anlamsal_ara", "a1", &"a".repeat(32)] {
            assert!(arac_adi_gecerli(ad), "{ad}");
        }
        assert!(matches!(
            hata(&TAM.replace("kalfa-rag", "Kalfa")),
            ManifestHatasi::Ad(_)
        ));
    }

    #[test]
    fn surum_aciklama_ve_depo_denetlenir() {
        assert!(matches!(
            hata(&TAM.replace("\"0.1.0\"", "\"1.0\"")),
            ManifestHatasi::Surum(_)
        ));
        assert_eq!(
            hata(&TAM.replace("Çalışma alanında", &"x".repeat(201))),
            ManifestHatasi::Aciklama
        );
        assert_eq!(
            hata(&TAM.replace("200", "2000")),
            ManifestHatasi::DepoMb(2000)
        );
        assert_eq!(hata(&TAM.replace("200", "-1")), ManifestHatasi::DepoMb(-1));
        assert_eq!(
            hata(&TAM.replace("\"rerank\"", "\"chat\"")),
            ManifestHatasi::EvrenUcu("chat".into())
        );
    }

    #[test]
    fn zararli_globlar_reddedilir() {
        for kalip in ["/etc/*", "../x/**", "C:/x/**", "src\\\\*.rs", ""] {
            let m = TAM.replace("\"docs/**\"", &format!("\"{kalip}\""));
            assert!(
                matches!(hata(&m), ManifestHatasi::Glob(..)),
                "{kalip}: {:?}",
                Manifest::coz(&m).map(|_| ())
            );
        }
        let cok: Vec<String> = (0..65).map(|i| format!("\"d{i}/**\"")).collect();
        let m = TAM.replace(
            "[\"**/*.{md,rs,ts}\", \"docs/**\"]",
            &format!("[{}]", cok.join(",")),
        );
        assert_eq!(hata(&m), ManifestHatasi::CokGlob);
    }
}
