//! `kabuk`: çalışma alanında komut çalıştırma (aşama 2 tasarımı §4).
//!
//! Aracın iki biçimi vardır: `{program, argumanlar}` kabuk yorumlayıcısı
//! olmadan çalışır; `{betik}` platformun kabuğunda. Komutu `kalfa-sandbox`'ın
//! yalıtıcısı başlatır. İzin kararı, onay kartı ve mühür hazırlıktaki
//! `KabukTarifi`'nden üretilir.

pub mod cikti;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, CiktiGonderici, Hazirlik};
use kalfa_core::git_koruma;
use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
use kalfa_core::protokol::CiktiAkisi;
use kalfa_core::yol::CalismaYolu;
use kalfa_sandbox::{
    AlanBilgisi, BaslatmaHatasi, Bitis, CalistirmaIstegi, KomutBicimi, Korumali, Yalitici,
    YalitimDurumu, ortam, program, tarama,
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

/// Yalıtım yokken komutların yalıtımsız çalışmasına izin var mı (aşama 2
/// tasarımı §9.8)? Ayarlardaki anahtardır; varsayılanı kapalıdır. Her
/// hazırlıkta ve her çalıştırmadan hemen önce sorulur.
pub trait YalitimsizIzni: Send + Sync {
    fn izinli_mi(&self) -> bool;
}

impl YalitimsizIzni for AtomicBool {
    fn izinli_mi(&self) -> bool {
        self.load(Ordering::SeqCst)
    }
}

const YALITIMSIZ_KAPALI: &str =
    "Yalıtım kullanılamıyor ve yalıtımsız çalıştırma kapalı; komut çalıştırılamaz.";

pub struct KabukAraci {
    alan: Arc<CalismaAlani>,
    yalitici: Arc<dyn Yalitici>,
    guven: Arc<dyn KabukGuveni>,
    izin: Arc<dyn YalitimsizIzni>,
}

impl KabukAraci {
    pub fn yeni(
        alan: Arc<CalismaAlani>,
        yalitici: Arc<dyn Yalitici>,
        guven: Arc<dyn KabukGuveni>,
        izin: Arc<dyn YalitimsizIzni>,
    ) -> Self {
        Self {
            alan,
            yalitici,
            guven,
            izin,
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

impl KabukAraci {
    /// Tarifteki yalıtım: arka uç ve profil özeti (§7.1). Özet mühre girer;
    /// kullanıcı bir dizin eklerse bekleyen onay geçersizleşir.
    fn yalitim_ozeti(&self, ag: bool) -> YalitimOzeti {
        match self.yalitici.durum() {
            YalitimDurumu::Hazir { arka_uc, surum } => YalitimOzeti::Yalitimli {
                arka_uc,
                profil: self.yalitici.profil_ozeti(ag).unwrap_or(surum),
            },
            YalitimDurumu::Yok { talimat, .. } => YalitimOzeti::Yalitimsiz { talimat },
        }
    }

    /// Yalıtımın çalışma alanı bilgisi; her çalıştırmadan hemen önce üretilir
    /// (§7.2): gizli dosya taraması ve git yerleşimi.
    async fn alan_bilgisi(&self) -> Result<AlanBilgisi, AracHatasi> {
        let alan = self.alan.clone();
        tokio::task::spawn_blocking(move || {
            let kok = alan
                .kok()
                .canonicalize()
                .map_err(|e| gecersiz(format!("Çalışma alanının yolu çözülemedi: {e}")))?;
            let gizli = tarama::tara(&kok, &|yol, _dizin| {
                CalismaYolu::coz(&yol.to_string_lossy())
                    .map(|y| alan.siniflandirici().gizli_mi(&y))
                    .unwrap_or(false)
            });
            let git = git_koruma::git_yerlesimi(&kok);
            Ok(AlanBilgisi {
                kok,
                gizli,
                korumali: git
                    .korumali
                    .into_iter()
                    .map(|k| Korumali {
                        yol: k.yol,
                        dizin: k.dizin,
                    })
                    .collect(),
                git_dizinleri: git.dis_dizinler,
            })
        })
        .await
        .map_err(|e| gecersiz(format!("Gizli dosya taraması yarıda kaldı: {e}")))?
    }
}

impl KabukAraci {
    /// Çalışma alanını yalıtım için hazırlar (§9.4); ilerleme araç kartında
    /// görünür, modele gitmez.
    async fn alani_hazirla(
        &self,
        alan: &AlanBilgisi,
        onaylandi: bool,
        gonderici: &CiktiGonderici,
    ) -> Result<(), AracHatasi> {
        let yalitici = self.yalitici.clone();
        let alan = alan.clone();
        let gonderici = gonderici.clone();
        tokio::task::spawn_blocking(move || {
            yalitici.alani_hazirla(&alan, onaylandi, &mut |satir| {
                gonderici.gonder(CiktiAkisi::Standart, format!("{satir}\n"));
            })
        })
        .await
        .map_err(|e| gecersiz(format!("Çalışma alanının hazırlığı yarıda kaldı: {e}")))?
        .map_err(|e| gecersiz(e.to_string()))
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

    /// Yalıtım hazırsa ya da yalıtımsız çalıştırma açıksa (§9.8).
    fn sunulur_mu(&self) -> bool {
        self.yalitici.durum().hazir_mi() || self.izin.izinli_mi()
    }

    async fn hazirla(&self, a: &Value) -> Result<Hazirlik, AracHatasi> {
        let istek = self.coz(a)?;
        let bicim = match istek.komut {
            KomutBicimi::Program { .. } => KabukBicimi::Program,
            KomutBicimi::Betik { .. } => KabukBicimi::Betik {
                yorumlayici: program::yorumlayici_adi().to_string(),
            },
        };
        let yalitim = self.yalitim_ozeti(istek.ag);
        if !yalitim.yalitimli_mi() && !self.izin.izinli_mi() {
            return Err(gecersiz(YALITIMSIZ_KAPALI));
        }
        // Windows'ta çalışma alanı ilk yalıtımlı çağrıda hazırlanır (§9.4).
        let alan_hazirligi =
            yalitim.yalitimli_mi() && self.yalitici.hazirlik_gerekli(self.alan.kok());
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
                yalitim,
                ilk_cagri: !self.guven.guvenildi_mi(),
                alan_hazirligi,
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
        h: &Hazirlik,
        iptal: CancellationToken,
        gonderici: CiktiGonderici,
    ) -> Result<AracCiktisi, AracHatasi> {
        if iptal.is_cancelled() {
            return Err(AracHatasi::Iptal);
        }
        let istek = self.coz(a)?;
        // Kip onaylanan hazırlıktan alınır, şimdiki durumdan değil: mühür
        // denetiminden sonra yeniden algılama durumu değiştirebilir. Yalıtıcı
        // onaylanan kiple şimdiki durumu karşılaştırır; tutmazsa başlatmaz.
        let yalitimli = match h.kabuk.as_ref().map(|t| &t.yalitim) {
            Some(YalitimOzeti::Yalitimli { .. }) => true,
            Some(YalitimOzeti::Yalitimsiz { .. }) => false,
            None => return Err(gecersiz("Hazırlıkta kabuk tarifi yok.".to_string())),
        };
        // Onaydan sonra anahtar kapatıldıysa yalıtımsız komut başlamaz.
        if !yalitimli && !self.izin.izinli_mi() {
            return Err(gecersiz(YALITIMSIZ_KAPALI));
        }
        let alan = if yalitimli {
            let bilgi = self.alan_bilgisi().await?;
            let onaylandi = h.kabuk.as_ref().is_some_and(|t| t.alan_hazirligi);
            self.alani_hazirla(&bilgi, onaylandi, &gonderici).await?;
            Some(bilgi)
        } else {
            None
        };
        let dizin = match (&alan, &istek.dizin) {
            (Some(b), Some(d)) => b.kok.join(d.std_yolu()),
            (Some(b), None) => b.kok.clone(),
            (None, _) => self.calisma_dizini(&istek.dizin),
        };
        // Yalıtımda çalışma alanı kanonik yolunda bağlanır; çalışma alanındaki
        // program da o yoldan çağrılır.
        let mut komut = istek.komut;
        if let (Some(b), KomutBicimi::Program { program, .. }) = (&alan, &mut komut)
            && let Ok(goreli) = Path::new(program.as_str()).strip_prefix(self.alan.kok())
        {
            *program = b.kok.join(goreli).display().to_string();
        }
        let calistirma = CalistirmaIstegi {
            komut,
            dizin,
            ortam: ortam::uret(std::env::vars()),
            ag: istek.ag,
            alan,
        };
        let surec = self.yalitici.baslat(&calistirma).map_err(|e| {
            if matches!(e, BaslatmaHatasi::Yalitim(_)) {
                self.yalitici.yeniden_algila();
            }
            gecersiz(e.to_string())
        })?;
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
        let metin = tampon.metin();
        // bwrap kurulum hatasını "bwrap: " önekiyle yazar ve 1 ile çıkar
        // (§11): yalıtım bozulmuş olabilir, algılama yeniden çalışır.
        if yalitimli && sonuc.bitis == Bitis::Cikti(Some(1)) && metin.starts_with("bwrap: ") {
            self.yalitici.yeniden_algila();
        }
        let govde = if tampon.toplam() == 0 {
            "--- çıktı yok ---".to_string()
        } else {
            format!("--- çıktı ---\n{metin}")
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
