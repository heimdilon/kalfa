//! `kabuk`: çalışma alanında komut çalıştırma (aşama 2 tasarımı §4).
//!
//! Aracın iki biçimi vardır: `{program, argumanlar}` kabuk yorumlayıcısı
//! olmadan çalışır; `{betik}` platformun kabuğunda. Komutu `kalfa-sandbox`'ın
//! yalıtıcısı başlatır. İzin kararı, onay kartı ve mühür hazırlıktaki
//! `KabukTarifi`'nden üretilir.

pub mod cikti;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, CiktiGonderici, Hazirlik};
use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
use kalfa_core::yol::CalismaYolu;
use kalfa_sandbox::{
    Bitis, CalistirmaIstegi, KomutBicimi, Yalitici, YalitimDurumu, ortam, program,
};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::calisma_alani::CalismaAlani;
use cikti::{BasSonTampon, CANLI_SINIR, CanliAkis};

pub const VARSAYILAN_SURE_SN: u64 = 120;
pub const AZAMI_SURE_SN: u64 = 1800;

/// Çalışma alanında kabuğun daha önce onaylanıp onaylanmadığı (aşama 2
/// tasarımı §4.2). Kayıt uygulamadadır ve yalıtımın içinde görünmez.
pub trait KabukGuveni: Send + Sync {
    fn guvenildi_mi(&self) -> bool;
    /// İlk çağrı onaylandı ve komut başlıyor.
    fn guvenildi(&self);
}

pub struct KabukAraci {
    alan: Arc<CalismaAlani>,
    yalitici: Arc<dyn Yalitici>,
    guven: Arc<dyn KabukGuveni>,
}

impl KabukAraci {
    pub fn yeni(
        alan: Arc<CalismaAlani>,
        yalitici: Arc<dyn Yalitici>,
        guven: Arc<dyn KabukGuveni>,
    ) -> Self {
        Self {
            alan,
            yalitici,
            guven,
        }
    }
}

/// Doğrulanmış argümanlar.
struct Istek {
    komut: KomutBicimi,
    /// Gösterim: onay kartı ve açıklama.
    gosterim: String,
    /// Çalışma alanına göre dizin; kök için `None`.
    dizin: Option<CalismaYolu>,
    ag: bool,
    sure: Duration,
    /// Sonucun kaynak etiketi: `kabuk:<program>`.
    kaynak: String,
}

fn metin<'a>(a: &'a Value, ad: &str) -> Option<&'a str> {
    a.get(ad).and_then(Value::as_str)
}

fn gecersiz(m: impl Into<String>) -> AracHatasi {
    AracHatasi::Gecersiz(m.into())
}

impl KabukAraci {
    fn coz(&self, a: &Value) -> Result<Istek, AracHatasi> {
        let dizin = match metin(a, "dizin") {
            None | Some("" | ".") => None,
            Some(d) => {
                let yol = CalismaYolu::coz(d)?;
                self.alan.dizin_dogrula(&yol).map_err(|e| {
                    gecersiz(format!(
                        "`{d}` çalışma alanında bir dizin değil ya da bağlantı içeriyor: {e}"
                    ))
                })?;
                Some(yol)
            }
        };
        let ag = a.get("ag").and_then(Value::as_bool).unwrap_or(false);
        let sure_sn = a
            .get("zaman_asimi_sn")
            .and_then(Value::as_u64)
            .unwrap_or(VARSAYILAN_SURE_SN)
            .clamp(1, AZAMI_SURE_SN);
        let (komut, gosterim, kaynak) = match (metin(a, "program"), metin(a, "betik")) {
            (Some(p), None) => {
                let argumanlar: Vec<String> = match a.get("argumanlar") {
                    None => Vec::new(),
                    Some(Value::Array(l)) => l
                        .iter()
                        .map(|v| {
                            v.as_str()
                                .map(str::to_string)
                                .ok_or_else(|| gecersiz("`argumanlar` yalnızca metin içerebilir."))
                        })
                        .collect::<Result<_, _>>()?,
                    Some(_) => return Err(gecersiz("`argumanlar` bir metin listesi olmalı.")),
                };
                let program = self.program_yolu(p)?;
                let gosterim = std::iter::once(p.to_string())
                    .chain(argumanlar.iter().map(|a| alintila(a)))
                    .collect::<Vec<_>>()
                    .join(" ");
                let ad = p.rsplit(['/', '\\']).next().unwrap_or(p).to_string();
                (
                    KomutBicimi::Program {
                        program,
                        argumanlar,
                    },
                    gosterim,
                    format!("kabuk:{ad}"),
                )
            }
            (None, Some(b)) => {
                if a.get("argumanlar").is_some() {
                    return Err(gecersiz(
                        "`argumanlar` yalnızca `program` ile kullanılır; betikte argümanları metne yaz.",
                    ));
                }
                if b.trim().is_empty() {
                    return Err(gecersiz("Betik boş."));
                }
                (
                    KomutBicimi::Betik {
                        metin: b.to_string(),
                    },
                    b.to_string(),
                    format!("kabuk:{}", program::yorumlayici_adi()),
                )
            }
            _ => {
                return Err(gecersiz(
                    "`program` ya da `betik`ten tam olarak biri verilmeli.",
                ));
            }
        };
        Ok(Istek {
            komut,
            gosterim,
            dizin,
            ag,
            sure: Duration::from_secs(sure_sn),
            kaynak,
        })
    }

    /// Çıplak ad olduğu gibi kalır (yalıtımın `PATH`'inde aranır); yol
    /// çalışma alanına göre çözülür. Mutlak yol reddedilir (aşama 2
    /// tasarımı §4.1).
    fn program_yolu(&self, p: &str) -> Result<String, AracHatasi> {
        if p.trim().is_empty() {
            return Err(gecersiz("Program adı boş."));
        }
        if !p.contains(['/', '\\']) {
            return Ok(p.to_string());
        }
        let yol = CalismaYolu::coz(p)?;
        Ok(self.alan.kok().join(yol.std_yolu()).display().to_string())
    }

    fn calisma_dizini(&self, dizin: &Option<CalismaYolu>) -> PathBuf {
        match dizin {
            None => self.alan.kok().to_path_buf(),
            Some(d) => self.alan.kok().join(d.std_yolu()),
        }
    }
}

/// Gösterim için argümanı tırnaklar. Yalnızca okunuş içindir; komut
/// argüman dizisi olarak çalışır.
fn alintila(a: &str) -> String {
    let guvenli = !a.is_empty()
        && a.chars()
            .all(|c| c.is_alphanumeric() || "-_./:=@%+,".contains(c));
    if guvenli {
        a.to_string()
    } else {
        format!("'{}'", a.replace('\'', r"'\''"))
    }
}

fn yalitim_ozeti(durum: YalitimDurumu) -> YalitimOzeti {
    match durum {
        YalitimDurumu::Hazir { arka_uc, surum } => YalitimOzeti::Yalitimli {
            arka_uc,
            profil: surum,
        },
        YalitimDurumu::Yok { talimat, .. } => YalitimOzeti::Yalitimsiz { talimat },
    }
}

/// Açıklamadaki kısa gösterim: ilk satır, en çok 80 karakter.
fn kisa(gosterim: &str) -> String {
    let ilk = gosterim.lines().next().unwrap_or("");
    let mut kisa: String = ilk.chars().take(80).collect();
    if kisa.len() < ilk.len() || gosterim.lines().nth(1).is_some() {
        kisa.push('…');
    }
    kisa
}

#[async_trait]
impl Arac for KabukAraci {
    fn ad(&self) -> &str {
        "kabuk"
    }

    fn aciklama(&self) -> &str {
        "Çalışma alanında bir komut çalıştırır. Mümkünse `program` ve `argumanlar` ver; boru, \
         yönlendirme ya da birden çok komut gerekiyorsa `betik` ver (Linux'ta sh, Windows'ta \
         PowerShell). Ağ varsayılan olarak kapalıdır; ağ gerekiyorsa `ag: true` iste, bu her \
         kipte onay gerektirir. Standart girdi kapalıdır; etkileşim bekleyen komut çalıştırma. \
         Çıktı uzunsa baştan ve sondan kırpılır."
    }

    fn parametre_semasi(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "program": {"type": "string", "description": "Çıplak program adı (PATH'te aranır) ya da çalışma alanına göre yol"},
                "argumanlar": {"type": "array", "items": {"type": "string"}},
                "betik": {"type": "string", "description": "Kabukta çalışacak metin"},
                "dizin": {"type": "string", "description": "Çalışma alanına göre çalışma dizini; varsayılan kök"},
                "ag": {"type": "boolean", "default": false},
                "zaman_asimi_sn": {"type": "integer", "minimum": 1, "maximum": AZAMI_SURE_SN, "default": VARSAYILAN_SURE_SN}
            },
            "oneOf": [{"required": ["program"]}, {"required": ["betik"]}],
            "additionalProperties": false
        })
    }

    fn yazar_mi(&self) -> bool {
        true
    }

    async fn hazirla(&self, a: &Value) -> Result<Hazirlik, AracHatasi> {
        let istek = self.coz(a)?;
        let bicim = match istek.komut {
            KomutBicimi::Program { .. } => KabukBicimi::Program,
            KomutBicimi::Betik { .. } => KabukBicimi::Betik {
                yorumlayici: program::yorumlayici_adi().to_string(),
            },
        };
        Ok(Hazirlik {
            aciklama: format!("`{}` komutunu çalıştırmak istiyor.", kisa(&istek.gosterim)),
            tum_calisma_alani: true,
            kabuk: Some(KabukTarifi {
                komut: istek.gosterim,
                bicim,
                dizin: istek
                    .dizin
                    .as_ref()
                    .map_or_else(|| ".".to_string(), |d| d.as_str().to_string()),
                ag: istek.ag,
                yalitim: yalitim_ozeti(self.yalitici.durum()),
                ilk_cagri: !self.guven.guvenildi_mi(),
            }),
            ..Default::default()
        })
    }

    async fn calistir(
        &self,
        a: &Value,
        h: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.calistir_akisli(a, h, iptal, CiktiGonderici::bos())
            .await
    }

    async fn calistir_akisli(
        &self,
        a: &Value,
        _h: &Hazirlik,
        iptal: CancellationToken,
        gonderici: CiktiGonderici,
    ) -> Result<AracCiktisi, AracHatasi> {
        if iptal.is_cancelled() {
            return Err(AracHatasi::Iptal);
        }
        let istek = self.coz(a)?;
        let calistirma = CalistirmaIstegi {
            komut: istek.komut,
            dizin: self.calisma_dizini(&istek.dizin),
            ortam: ortam::uret(std::env::vars()),
            ag: istek.ag,
        };
        let surec = self
            .yalitici
            .baslat(&calistirma)
            .map_err(|e| gecersiz(e.to_string()))?;
        // Komut başladı; çalışma alanındaki ilk onay kaydedilir.
        self.guven.guvenildi();
        let mut tampon = BasSonTampon::default();
        let mut canli = CanliAkis::yeni(gonderici, CANLI_SINIR);
        let sonuc = surec
            .bekle(istek.sure, iptal, |akis, veri| {
                tampon.ekle(veri);
                canli.ekle(akis, veri);
            })
            .await;
        let durum = match sonuc.bitis {
            Bitis::Iptal => return Err(AracHatasi::Iptal),
            Bitis::Cikti(Some(kod)) => format!("Çıkış kodu: {kod}"),
            Bitis::Cikti(None) => "Komut bir sinyalle sonlandı.".to_string(),
            Bitis::ZamanAsimi => format!(
                "Zaman aşımı ({} sn); süreç ağacı durduruldu.",
                istek.sure.as_secs()
            ),
        };
        let sure = format!("{:.1}", sonuc.sure.as_secs_f64()).replace('.', ",");
        let govde = if tampon.toplam() == 0 {
            "--- çıktı yok ---".to_string()
        } else {
            format!("--- çıktı ---\n{}", tampon.metin())
        };
        let cikti = AracCiktisi {
            icerik: format!("{durum}\nSüre: {sure} sn\n{govde}"),
            kaynak: istek.kaynak,
        };
        if sonuc.bitis == Bitis::Cikti(Some(0)) {
            Ok(cikti)
        } else {
            Err(AracHatasi::Basarisiz(cikti))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gosterimde_ozel_karakterli_arguman_tirnaklanir() {
        assert_eq!(alintila("test"), "test");
        assert_eq!(alintila("--features=e2e"), "--features=e2e");
        assert_eq!(alintila("iki kelime"), "'iki kelime'");
        assert_eq!(alintila(""), "''");
        assert_eq!(alintila("a'b"), r"'a'\''b'");
        assert_eq!(alintila("$(rm)"), "'$(rm)'");
    }

    #[test]
    fn aciklama_ilk_satiri_kisaltir() {
        assert_eq!(kisa("cargo test"), "cargo test");
        assert_eq!(kisa("echo a\necho b"), "echo a…");
        assert_eq!(kisa(&"x".repeat(100)), format!("{}…", "x".repeat(80)));
    }
}
