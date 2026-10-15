//! Sır taşımayan canonical MCP özetleri.
//!
//! T ve R burada üretilir. Π/K/P üreticileri ile süreç başına artan N sayacı
//! Görev 4/5 yöneticisine aittir; eksik görüntüden K üretilemez.

use std::{
    io::{self, Read},
    path::{Path, PathBuf},
};

use kalfa_core::arac::{
    McpAracTanimOzeti, McpBaslatmaSayaci, McpKorumaOzeti, McpPolitikaOzeti, McpProfilOzeti,
    McpTanimOzeti, McpTarifi, McpYalitimProfiliOzeti,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::tanim::{CozulmusTanim, McpTanim, TanimHatasi};

/// Onaydan sonra okunan dosyanın yalnız yolu ve SHA-256'sı tutulur.
/// Ham dosya içeriği bu tipe alınmaz; Debug/serde uygulanmaz.
pub struct DosyaIcerikOzeti {
    yol: PathBuf,
    sha256: [u8; 32],
}

impl DosyaIcerikOzeti {
    pub fn yeni(yol: PathBuf, sha256: [u8; 32]) -> Self {
        Self { yol, sha256 }
    }

    pub fn yol(&self) -> &std::path::Path {
        &self.yol
    }
}

fn alan(h: &mut Sha256, veri: &[u8]) {
    h.update((veri.len() as u64).to_le_bytes());
    h.update(veri);
}

/// Ham değer yalnızca SHA-256 akışına girer; canonical tarifte içerik yerine özeti taşınır.
pub fn dosya_icerik_ozeti(icerik: &[u8]) -> [u8; 32] {
    Sha256::digest(icerik).into()
}

fn gizli_deger_ozeti(deger: &str) -> [u8; 32] {
    Sha256::digest(deger.as_bytes()).into()
}

/// İçeriği tek Vec'e almadan, sabit 32 KiB bloklarla SHA-256 üretir.
pub fn dosya_akisi_ozeti(mut okuyucu: impl Read) -> io::Result<[u8; 32]> {
    let mut ozet = Sha256::new();
    let mut tampon = [0_u8; 32 * 1024];
    loop {
        let adet = match okuyucu.read(&mut tampon) {
            Ok(0) => break,
            Ok(adet) => adet,
            Err(hata) if hata.kind() == io::ErrorKind::Interrupted => continue,
            Err(hata) => return Err(hata),
        };
        ozet.update(&tampon[..adet]);
    }
    Ok(ozet.finalize().into())
}

fn yol_metni(yol: &Path) -> Result<&str, TanimHatasi> {
    yol.to_str().ok_or(TanimHatasi::YolCozumlenemedi)
}

/// T = ad, çözülmüş ikili ve içeriği, sıraya duyarlı argümanlar, son ortam,
/// kip/ağ, özel çalışma dizini, alan kimliği ve onay sonrası dosya özetleri.
pub fn tanim_ozeti(
    tanim: &McpTanim,
    cozulmus: &CozulmusTanim,
    dosyalar: &[DosyaIcerikOzeti],
    alan_kimligi: &str,
) -> Result<McpTanimOzeti, TanimHatasi> {
    let mut beklenen: Vec<_> = cozulmus.dosya_adaylari().iter().map(|d| d.yol()).collect();
    let mut gelen: Vec<_> = dosyalar.iter().map(|d| d.yol.as_path()).collect();
    beklenen.sort();
    gelen.sort();
    if beklenen != gelen {
        return Err(TanimHatasi::DosyaOkunamadi);
    }

    let ikili = dosyalar
        .iter()
        .find(|f| f.yol == cozulmus.program())
        .ok_or(TanimHatasi::DosyaOkunamadi)?;
    let mut h = Sha256::new();
    alan(&mut h, b"kalfa-mcp-tanim-v1");
    alan(&mut h, tanim.ad().as_bytes());
    alan(&mut h, &[cozulmus.komut_tipi()]);
    alan(&mut h, yol_metni(cozulmus.program())?.as_bytes());
    alan(&mut h, &ikili.sha256);
    alan(&mut h, yol_metni(cozulmus.calisma_dizini())?.as_bytes());
    alan(&mut h, alan_kimligi.as_bytes());
    alan(&mut h, b"argumanlar");
    alan(&mut h, &(cozulmus.argumanlar().len() as u64).to_le_bytes());
    for arg in cozulmus.argumanlar() {
        alan(&mut h, arg.as_bytes());
    }
    let mut ortam: Vec<_> = cozulmus.ortam().ham_degerler().collect();
    ortam.sort_by(|a, b| {
        let a = if cfg!(windows) {
            a.0.to_ascii_lowercase()
        } else {
            a.0.to_string()
        };
        let b = if cfg!(windows) {
            b.0.to_ascii_lowercase()
        } else {
            b.0.to_string()
        };
        a.cmp(&b)
    });
    alan(&mut h, b"ortam");
    alan(&mut h, &(ortam.len() as u64).to_le_bytes());
    for (ad, deger, gizli) in ortam {
        let ad = if cfg!(windows) {
            ad.to_ascii_lowercase()
        } else {
            ad.to_owned()
        };
        alan(&mut h, ad.as_bytes());
        alan(&mut h, &[u8::from(gizli)]);
        alan(&mut h, &gizli_deger_ozeti(deger));
    }
    alan(
        &mut h,
        &[match cozulmus.kip() {
            kalfa_sandbox::CalistirmaKipi::Yalitimli => 1,
            kalfa_sandbox::CalistirmaKipi::TamErisim => 2,
        }],
    );
    let mut ag = cozulmus.ag().to_vec();
    ag.sort();
    alan(&mut h, b"ag");
    alan(&mut h, &(ag.len() as u64).to_le_bytes());
    for kural in ag {
        alan(&mut h, kural.host().as_bytes());
        alan(&mut h, &kural.port().to_le_bytes());
        alan(&mut h, &[u8::from(kural.ozel_ag())]);
    }
    alan(&mut h, &cozulmus.timeoutlar().0.to_le_bytes());
    alan(&mut h, &cozulmus.timeoutlar().1.to_le_bytes());
    let mut dosyalar: Vec<_> = dosyalar.iter().collect();
    dosyalar.sort_by(|a, b| a.yol.cmp(&b.yol));
    alan(&mut h, b"dosyalar");
    alan(&mut h, &(dosyalar.len() as u64).to_le_bytes());
    for dosya in dosyalar {
        alan(&mut h, yol_metni(&dosya.yol)?.as_bytes());
        alan(&mut h, &dosya.sha256);
    }
    Ok(McpTanimOzeti::ozetten(h.finalize().into()))
}

/// P = Π + K + yalıtım profili. Görev 4/5 doğrulanmış girdileri sağlayacaktır;
/// bu fonksiyon mevcut özetleri tek canonical profile bağlar.
pub fn profil_ozeti(
    politika: McpPolitikaOzeti,
    koruma: McpKorumaOzeti,
    yalitim_profili: McpYalitimProfiliOzeti,
) -> McpProfilOzeti {
    McpProfilOzeti::birlestir(politika, koruma, yalitim_profili)
}

/// R = T + P + N. Π/K girdilerinin üretim doğrulaması Görev 4/5'tedir.
pub fn calisma_tarifi(
    tanim: McpTanimOzeti,
    politika: McpPolitikaOzeti,
    koruma: McpKorumaOzeti,
    yalitim_profili: McpYalitimProfiliOzeti,
    baslatma: McpBaslatmaSayaci,
) -> McpTarifi {
    McpTarifi::yeni(tanim, politika, koruma, yalitim_profili, baslatma)
}

/// D, araca modele sunulan tam ad/açıklama/girdi/çıkış şeması içindir.
/// Açıklamanın kırpılmış değeri çağıran tarafından verilmelidir.
pub fn arac_tanim_ozeti(
    ad: &str,
    aciklama: &str,
    girdi_semasi: &Value,
    cikti_semasi: Option<&Value>,
) -> McpAracTanimOzeti {
    let mut h = Sha256::new();
    alan(&mut h, b"kalfa-mcp-arac-tanimi-v1");
    alan(&mut h, ad.as_bytes());
    alan(&mut h, aciklama.as_bytes());
    kanonik_json(&mut h, girdi_semasi);
    if let Some(sema) = cikti_semasi {
        alan(&mut h, b"cikti-var");
        kanonik_json(&mut h, sema);
    } else {
        alan(&mut h, b"cikti-yok");
    }
    McpAracTanimOzeti::ozetten(h.finalize().into())
}

fn kanonik_json(h: &mut Sha256, deger: &Value) {
    match deger {
        Value::Null => alan(h, b"null"),
        Value::Bool(v) => alan(h, if *v { b"true" } else { b"false" }),
        Value::Number(v) => {
            alan(h, b"number");
            alan(h, v.to_string().as_bytes());
        }
        Value::String(v) => {
            alan(h, b"string");
            alan(h, v.as_bytes());
        }
        Value::Array(degerler) => {
            alan(h, b"array");
            alan(h, &(degerler.len() as u64).to_le_bytes());
            for oge in degerler {
                kanonik_json(h, oge);
            }
        }
        Value::Object(degerler) => {
            alan(h, b"object");
            alan(h, &(degerler.len() as u64).to_le_bytes());
            let mut anahtarlar: Vec<_> = degerler.keys().collect();
            anahtarlar.sort();
            for anahtar in anahtarlar {
                alan(h, anahtar.as_bytes());
                if let Some(oge) = degerler.get(anahtar) {
                    kanonik_json(h, oge);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ortam::OrtamDegeri,
        tanim::{AgIzinKurali, KodGuveni, Komut, SeciliYurutulebilirKimligi, YazilabilirKokler},
    };
    use kalfa_sandbox::{CalistirmaKipi, OzelDizinler};

    #[derive(Clone, Copy)]
    enum KodYeri {
        Sunucu,
        BoslukluSunucu,
        Proje,
        Kabuk,
    }
    #[derive(Clone, Copy)]
    enum Degisim {
        Yok,
        Ad,
        DuzOrtam,
        GizliOrtam,
        NodeOptions,
        Path,
        Kip,
        Ag,
        Arguman,
        BaslatmaZamani,
        CagriZamani,
    }

    fn hazir_tanim(
        kok: &Path,
        yer: KodYeri,
        degisim: Degisim,
    ) -> (McpTanim, CozulmusTanim, PathBuf) {
        let proje = kok.join("proje");
        let yalitim = kok.join("yalitim");
        let ozel = kalfa_sandbox::sunucu_ozel(&yalitim, "alan", "ornek");
        let kabuk = OzelDizinler::yol(&yalitim, "alan");
        let bin = kok.join("bin");
        let bin2 = kok.join("bin2");
        for dizin in [
            &proje,
            &ozel.ev,
            &ozel.gecici,
            &ozel.onbellek,
            &kabuk.ev,
            &kabuk.gecici,
            &kabuk.onbellek,
            &bin,
            &bin2,
        ] {
            std::fs::create_dir_all(dizin).unwrap();
        }
        let ikili = bin.join(if cfg!(windows) {
            "deneme.exe"
        } else {
            "deneme"
        });
        std::fs::write(&ikili, b"ikili").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&ikili, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let (betik, arguman) = match yer {
            KodYeri::Sunucu => (ozel.ev.join("sunucu.js"), "sunucu.js".to_string()),
            KodYeri::BoslukluSunucu => {
                (ozel.ev.join("sunucu kodu.js"), "sunucu kodu.js".to_string())
            }
            KodYeri::Proje => (
                proje.join("sunucu.js"),
                "${CALISMA_ALANI}/sunucu.js".to_string(),
            ),
            KodYeri::Kabuk => {
                let yol = kabuk.ev.join("sunucu.js");
                let arguman = yol.to_str().unwrap().to_owned();
                (yol, arguman)
            }
        };
        std::fs::write(&betik, b"aaaa").unwrap();
        let mut argumanlar = vec![arguman];
        if matches!(degisim, Degisim::Arguman) {
            argumanlar.push("--renk=evet".into());
        }
        let tanim = McpTanim::yeni(
            if matches!(degisim, Degisim::Ad) {
                "ornek-iki"
            } else {
                "ornek"
            }
            .into(),
            Komut::SeciliYurutulebilir {
                kimlik: SeciliYurutulebilirKimligi::yeni(7),
                argumanlar,
            },
            vec![
                OrtamDegeri::gizli(
                    "GITHUB_TOKEN",
                    if matches!(degisim, Degisim::GizliOrtam) {
                        "sir-degisti"
                    } else {
                        "sir-ozgun"
                    },
                ),
                OrtamDegeri::duz(
                    "DUZ",
                    if matches!(degisim, Degisim::DuzOrtam) {
                        "iki"
                    } else {
                        "bir"
                    },
                ),
                OrtamDegeri::duz(
                    "NODE_OPTIONS",
                    if matches!(degisim, Degisim::NodeOptions) {
                        "--trace-warnings"
                    } else {
                        "--no-warnings"
                    },
                ),
            ],
            if matches!(degisim, Degisim::Kip) {
                CalistirmaKipi::TamErisim
            } else {
                CalistirmaKipi::Yalitimli
            },
            if matches!(degisim, Degisim::Ag) {
                Some(vec![AgIzinKurali::yeni("example.com:443", false).unwrap()])
            } else {
                None
            },
            if matches!(degisim, Degisim::BaslatmaZamani) {
                31
            } else {
                30
            },
            if matches!(degisim, Degisim::CagriZamani) {
                121
            } else {
                120
            },
        )
        .unwrap();
        let path = std::env::join_paths(if matches!(degisim, Degisim::Path) {
            vec![bin, bin2]
        } else {
            vec![bin]
        })
        .unwrap();
        let sistem = vec![("PATH".into(), path.to_str().unwrap().to_owned())];
        let kokler = YazilabilirKokler::yeni(proje, kabuk);
        let cozucu = move |_: SeciliYurutulebilirKimligi| Ok(ikili.clone());
        let cozulmus = tanim.hazirla(&sistem, &ozel, &kokler, &cozucu).unwrap();
        (tanim, cozulmus, betik)
    }

    fn icerikleri_ozetle(cozulmus: &CozulmusTanim) -> Vec<DosyaIcerikOzeti> {
        cozulmus
            .dosya_adaylari()
            .iter()
            .map(|aday| {
                DosyaIcerikOzeti::yeni(
                    aday.yol().to_owned(),
                    dosya_akisi_ozeti(std::fs::File::open(aday.yol()).unwrap()).unwrap(),
                )
            })
            .collect()
    }

    #[test]
    fn ayni_boyutlu_ve_bosluklu_kod_dosyasi_degisiminde_t_degisir() {
        for yer in [KodYeri::Sunucu, KodYeri::BoslukluSunucu] {
            let kok = tempfile::tempdir().unwrap();
            let (tanim, cozulmus, betik) = hazir_tanim(kok.path(), yer, Degisim::Yok);
            let ilk =
                tanim_ozeti(&tanim, &cozulmus, &icerikleri_ozetle(&cozulmus), "alan").unwrap();
            std::fs::write(&betik, b"bbbb").unwrap();
            let son =
                tanim_ozeti(&tanim, &cozulmus, &icerikleri_ozetle(&cozulmus), "alan").unwrap();
            assert_ne!(ilk, son);
        }
    }

    #[test]
    fn tanim_ozeti_ortam_kip_ag_path_alan_ve_komut_ayrintilarina_baglidir() {
        let kok = tempfile::tempdir().unwrap();
        let (tanim, cozulmus, _) = hazir_tanim(kok.path(), KodYeri::Sunucu, Degisim::Yok);
        let ilk = tanim_ozeti(&tanim, &cozulmus, &icerikleri_ozetle(&cozulmus), "alan").unwrap();
        for degisim in [
            Degisim::Ad,
            Degisim::DuzOrtam,
            Degisim::GizliOrtam,
            Degisim::NodeOptions,
            Degisim::Path,
            Degisim::Kip,
            Degisim::Ag,
            Degisim::Arguman,
            Degisim::BaslatmaZamani,
            Degisim::CagriZamani,
        ] {
            let (tanim, cozulmus, _) = hazir_tanim(kok.path(), KodYeri::Sunucu, degisim);
            let son =
                tanim_ozeti(&tanim, &cozulmus, &icerikleri_ozetle(&cozulmus), "alan").unwrap();
            assert_ne!(ilk, son);
        }
        let farkli_alan = tanim_ozeti(
            &tanim,
            &cozulmus,
            &icerikleri_ozetle(&cozulmus),
            "baska-alan",
        )
        .unwrap();
        assert_ne!(ilk, farkli_alan);
    }

    #[test]
    fn w_proje_ve_kabuk_koklerini_kapsar_sunucunun_ozel_koku_disarida_kalir() {
        for (yer, beklenen) in [
            (KodYeri::Proje, KodGuveni::WdeKodlu),
            (KodYeri::Kabuk, KodGuveni::WdeKodlu),
            (KodYeri::Sunucu, KodGuveni::WDisindaKodlu),
        ] {
            let kok = tempfile::tempdir().unwrap();
            let (_, cozulmus, betik) = hazir_tanim(kok.path(), yer, Degisim::Yok);
            assert_eq!(cozulmus.kod_guveni(), beklenen);
            if matches!(yer, KodYeri::Sunucu) {
                assert_eq!(
                    cozulmus.calisma_dizini(),
                    betik.parent().unwrap().canonicalize().unwrap()
                );
            }
        }
    }

    #[test]
    fn eksik_dosya_ozeti_kismi_tanim_uretmez() {
        let kok = tempfile::tempdir().unwrap();
        let (tanim, cozulmus, _) = hazir_tanim(kok.path(), KodYeri::Sunucu, Degisim::Yok);
        assert!(matches!(
            tanim_ozeti(&tanim, &cozulmus, &[], "alan"),
            Err(TanimHatasi::DosyaOkunamadi)
        ));
    }

    #[test]
    fn d_nesne_sirasindan_etkilenmez_cikti_semasi_ve_aciklama_degisiminden_etkilenir() {
        let a: Value = serde_json::from_str(
            r#"{"type":"object","properties":{"x":{"type":"string"},"y":{"type":"integer"}}}"#,
        )
        .unwrap();
        let b: Value = serde_json::from_str(
            r#"{"properties":{"y":{"type":"integer"},"x":{"type":"string"}},"type":"object"}"#,
        )
        .unwrap();
        let ilk = arac_tanim_ozeti("arac", "aciklama", &a, None);
        assert_eq!(ilk, arac_tanim_ozeti("arac", "aciklama", &b, None));
        assert_ne!(ilk, arac_tanim_ozeti("arac", "farkli", &a, None));
        assert_ne!(
            ilk,
            arac_tanim_ozeti("arac", "aciklama", &a, Some(&Value::Null))
        );
        assert_ne!(ilk, arac_tanim_ozeti("baska", "aciklama", &a, None));
    }

    #[test]
    fn akis_ozeti_sha256_ile_ayni_olur_okuma_hatasi_yutulmaz() {
        let veri = vec![b'x'; 100_001];
        assert_eq!(
            dosya_akisi_ozeti(std::io::Cursor::new(&veri)).unwrap(),
            dosya_icerik_ozeti(&veri)
        );
        struct Hata;
        impl Read for Hata {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("sahte okuma hatası"))
            }
        }
        assert!(dosya_akisi_ozeti(Hata).is_err());
    }
}
