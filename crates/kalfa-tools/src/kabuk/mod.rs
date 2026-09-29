//! `kabuk`: çalışma alanında komut çalıştırma (aşama 2 tasarımı §4).
//!
//! Aracın iki biçimi vardır: `{program, argumanlar}` kabuk yorumlayıcısı
//! olmadan çalışır; `{betik}` platformun kabuğunda. Komutu `kalfa-sandbox`'ın
//! yalıtıcısı başlatır. İzin kararı, onay kartı ve mühür hazırlıktaki
//! `KabukTarifi`'nden üretilir.

pub mod cikti;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use kalfa_core::arac::{
    Arac, AracCiktisi, AracHatasi, CiktiGonderici, DegisiklikTuru, DosyaDegisiklikleri, Hazirlik,
};
use kalfa_core::git_koruma;
use kalfa_core::izin::Siniflandirici;
use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
use kalfa_core::protokol::CiktiAkisi;
use kalfa_core::yol::CalismaYolu;
use kalfa_sandbox::git_denetimi::{GitDenetimi, yeni_git_denetle};
use kalfa_sandbox::tarama::AdliGirdiler;
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
    ///
    /// Her derinlikteki `.git` ve onların çalışma alanındaki `gitdir` ile
    /// `commondir` hedefleri de salt okunurdur (güvenlik analizi Bulgu 1).
    /// Gizli dizinlerin altındakiler listelenmez: onlara verilen açık okuma
    /// izni (Windows) gizli dizinin içini gösterirdi; zaten görünmezler.
    async fn alan_bilgisi(&self) -> Result<AlanBilgisi, AracHatasi> {
        let alan = self.alan.clone();
        tokio::task::spawn_blocking(move || {
            let kok = alan
                .kok()
                .canonicalize()
                .map_err(|e| gecersiz(format!("Çalışma alanının yolu çözülemedi: {e}")))?;
            let s = alan.siniflandirici();
            let gizli = tarama::tara_ayrintili(
                &kok,
                &|yol, _dizin| {
                    CalismaYolu::coz(&yol.to_string_lossy())
                        .map(|y| s.gizli_mi(&y))
                        .unwrap_or(false)
                },
                &git_adi_mi,
                &|yol, dizin| yapilandirma_mi(s, yol, dizin),
            );
            let git = git_koruma::git_yerlesimi(&kok);
            let mut korumali: Vec<Korumali> = git
                .korumali
                .into_iter()
                .map(|k| Korumali {
                    yol: k.yol,
                    dizin: k.dizin,
                })
                .collect();
            // Bağlantı olan `.git` izlenmez ve bağlanmaz (bilinen sınır).
            for g in gizli.git.iter().filter(|g| !g.baglanti) {
                korumali.push(Korumali {
                    yol: g.yol.clone(),
                    dizin: g.dizin,
                });
                for yol in git_koruma::yonlendirilen_dizinler(&g.yol, g.dizin, &kok) {
                    korumali.push(Korumali { yol, dizin: true });
                }
            }
            // Kendiliğinden çalışan yapılandırma da salt okunurdur (Bulgu 2).
            for y in &gizli.yapilandirma {
                korumali.push(Korumali {
                    yol: y.yol.clone(),
                    dizin: y.dizin,
                });
            }
            korumali.retain(|k| {
                !gizli.dizinler.iter().any(|d| k.yol.starts_with(d))
                    && !gizli.dosyalar.contains(&k.yol)
            });
            korumali.sort_by(|a, b| a.yol.cmp(&b.yol));
            korumali.dedup_by(|a, b| a.yol == b.yol);
            Ok(AlanBilgisi {
                kok,
                gizli,
                korumali,
                git_dizinleri: git.dis_dizinler,
            })
        })
        .await
        .map_err(|e| gecersiz(format!("Gizli dosya taraması yarıda kaldı: {e}")))?
    }
}

impl KabukAraci {
    /// Çalışma alanındaki `.git` girdileri ve yapılandırma yolları;
    /// yalıtımsız komuttan önce.
    async fn adli_girdiler(&self) -> Result<AdliGirdiler, AracHatasi> {
        let alan = self.alan.clone();
        tokio::task::spawn_blocking(move || {
            let kok = alan
                .kok()
                .canonicalize()
                .map_err(|e| gecersiz(format!("Çalışma alanının yolu çözülemedi: {e}")))?;
            let s = alan.siniflandirici();
            Ok(tarama::adli_girdiler(&kok, &git_adi_mi, &|yol, dizin| {
                yapilandirma_mi(s, yol, dizin)
            }))
        })
        .await
        .map_err(|e| gecersiz(format!("Git taraması yarıda kaldı: {e}")))?
    }

    /// Komuttan sonra yeni `.git` girdilerini denetler ve tehlikelileri
    /// karantinaya alır (güvenlik analizi Bulgu 1); yeni yapılandırma
    /// dosyalarını bildirir (Bulgu 2). Aynı taramanın görüntüsü komutun
    /// değiştirdiği dosyaları verir (aşama 3 tasarımı §5.4). Uyarıları, bir
    /// karantinanın başarısız olup olmadığını ve değişiklikleri döner.
    async fn komut_sonrasi_denetim(&self, onceki: AdliGirdiler) -> KomutSonrasi {
        let alan = self.alan.clone();
        tokio::task::spawn_blocking(move || {
            let kok = match alan.kok().canonicalize() {
                Ok(k) => k,
                Err(e) => return KomutSonrasi::denetlenemedi(e),
            };
            let damga = utc_damgasi(std::time::SystemTime::now());
            let mut uyarilar = Vec::new();
            let mut basarisiz = false;
            let s = alan.siniflandirici();
            let simdi = tarama::adli_girdiler(&kok, &git_adi_mi, &|yol, dizin| {
                yapilandirma_mi(s, yol, dizin)
            });
            let degisiklikler = DosyaDegisiklikleri::olustur(
                tarama::karsilastir(&onceki.goruntu, &simdi.goruntu)
                    .into_iter()
                    .map(|d| (d.yol, degisiklik_turu(d.tur))),
                onceki.goruntu.eksik || simdi.goruntu.eksik,
            );
            for y in &simdi.yapilandirma {
                let yeni = !onceki
                    .yapilandirma
                    .iter()
                    .any(|o| y.yol == o.yol || (o.dizin && y.yol.starts_with(&o.yol)));
                if yeni {
                    let goreli = y.yol.strip_prefix(&kok).unwrap_or(&y.yol);
                    uyarilar.push(format!(
                        "Komut, kendiliğinden çalışabilecek bir yapılandırma dosyası \
                         oluşturdu: `{}`. İnceleyin.",
                        goreli.to_string_lossy().replace('\\', "/")
                    ));
                }
            }
            for g in simdi.git {
                if onceki.git.iter().any(|o| o.yol == g.yol) {
                    continue;
                }
                let GitDenetimi::Tehlikeli { nedenler } =
                    yeni_git_denetle(&g.yol, g.dizin && !g.baglanti)
                else {
                    continue;
                };
                let nedenler = nedenler.join("; ");
                let goreli = g
                    .yol
                    .strip_prefix(&kok)
                    .ok()
                    .and_then(|y| CalismaYolu::coz(&y.to_string_lossy()).ok());
                let Some(yol) = goreli else {
                    basarisiz = true;
                    uyarilar.push(format!("{GIT_DENETLENEMEDI}: {}", g.yol.display()));
                    continue;
                };
                match alan.karantinaya_al(&yol, &damga) {
                    Ok(yeni) => uyarilar.push(format!(
                        "Komut, git'in kendiliğinden kod çalıştırabileceği bir `.git` \
                         oluşturdu: `{yol}` ({nedenler}). Güvenlik için `{yeni}` olarak \
                         kenara alındı."
                    )),
                    Err(e) => {
                        basarisiz = true;
                        uyarilar.push(format!(
                            "Komut, git'in kendiliğinden kod çalıştırabileceği bir `.git` \
                             oluşturdu: `{yol}` ({nedenler}); karantinaya alınamadı ({e}). \
                             Bu klasörde git çalıştırmadan önce `.git`'i inceleyip silin."
                        ));
                    }
                }
            }
            KomutSonrasi {
                uyarilar,
                karantina_basarisiz: basarisiz,
                degisiklikler: Some(degisiklikler),
            }
        })
        .await
        .unwrap_or_else(KomutSonrasi::denetlenemedi)
    }

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
        let mut alan = if yalitimli {
            let bilgi = self.alan_bilgisi().await?;
            let onaylandi = h.kabuk.as_ref().is_some_and(|t| t.alan_hazirligi);
            self.alani_hazirla(&bilgi, onaylandi, &gonderici).await?;
            Some(bilgi)
        } else {
            None
        };
        let onceki = match &mut alan {
            Some(b) => AdliGirdiler {
                git: b.gizli.git.clone(),
                yapilandirma: b.gizli.yapilandirma.clone(),
                // Yalıtıcı görüntüyü kullanmaz.
                goruntu: std::mem::take(&mut b.gizli.goruntu),
            },
            None => self.adli_girdiler().await?,
        };
        let dizin = match (&alan, &istek.dizin) {
            (Some(b), Some(d)) => b.kok.join(d.std_yolu()),
            (Some(b), None) => b.kok.clone(),
            (None, _) => self.calisma_dizini(&istek.dizin),
        };
        // Yalıtımda çalışma alanı kanonik yolunda bağlanır; çalışma alanındaki
        // program da o yoldan çağrılır.
        let mut komut = istek.komut;
        let cargo = cargo_komutu_mu(&komut);
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
        // Komut nasıl biterse bitsin yeni `.git` ve yapılandırma denetlenir.
        // İptalde uyarılar modele gidemez; karantina yine yapılır.
        let KomutSonrasi {
            uyarilar: guvenlik_uyarilari,
            karantina_basarisiz,
            degisiklikler,
        } = self.komut_sonrasi_denetim(onceki).await;
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
        let mut govde = if tampon.toplam() == 0 {
            "--- çıktı yok ---".to_string()
        } else {
            format!("--- çıktı ---\n{metin}")
        };
        let basarili = sonuc.bitis == Bitis::Cikti(Some(0));
        if let Some(talimat) = cargo_talimati(cfg!(windows), yalitimli, cargo, basarili, &metin) {
            govde.push_str(&format!("\n--- Kalfa ---\n{talimat}"));
        }
        let cikti = AracCiktisi {
            icerik: format!("{durum}\nSüre: {sure} sn\n{govde}"),
            kaynak: istek.kaynak,
            guvenlik_uyarilari,
            degisiklikler: degisiklikler.map(Box::new),
        };
        if sonuc.bitis == Bitis::Cikti(Some(0)) && !karantina_basarisiz {
            Ok(cikti)
        } else {
            Err(AracHatasi::Basarisiz(cikti))
        }
    }
}

const GIT_DENETLENEMEDI: &str = "Komuttan sonra yeni `.git` denetlenemedi";

/// Komuttan sonraki denetimin sonucu.
struct KomutSonrasi {
    uyarilar: Vec<String>,
    karantina_basarisiz: bool,
    /// Tarama yapılamadıysa `None`: değişiklikler bilinmiyor.
    degisiklikler: Option<DosyaDegisiklikleri>,
}

impl KomutSonrasi {
    fn denetlenemedi(e: impl std::fmt::Display) -> Self {
        Self {
            uyarilar: vec![format!("{GIT_DENETLENEMEDI}: {e}")],
            karantina_basarisiz: true,
            degisiklikler: None,
        }
    }
}

fn degisiklik_turu(tur: tarama::DegisiklikTuru) -> DegisiklikTuru {
    match tur {
        tarama::DegisiklikTuru::Eklendi => DegisiklikTuru::Eklendi,
        tarama::DegisiklikTuru::Degisti => DegisiklikTuru::Degisti,
        tarama::DegisiklikTuru::Silindi => DegisiklikTuru::Silindi,
    }
}

/// Tarama için git'in dizin adı kuralı (çekirdekteki `git_bileseni_mi`).
fn git_adi_mi(ad: &OsStr) -> bool {
    git_koruma::git_bileseni_mi(&ad.to_string_lossy())
}

/// Tarama için yapılandırma kuralı (çekirdekteki sınıflandırıcı): dizin
/// bütünüyle yapılandırma sayılıyorsa (`.vscode`) dizin, değilse dosya.
fn yapilandirma_mi(s: &Siniflandirici, yol: &Path, dizin: bool) -> bool {
    let Ok(y) = CalismaYolu::coz(&yol.to_string_lossy()) else {
        return false;
    };
    if dizin {
        s.yapilandirma_dizini_mi(&y)
    } else {
        s.yapilandirma_mi(&y)
    }
}

/// Karantina adındaki UTC zaman damgası: `yyyymmddThhmmss`.
fn utc_damgasi(an: std::time::SystemTime) -> String {
    let sn = an
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |s| s.as_secs());
    let (gun, gun_ici) = (sn / 86_400, sn % 86_400);
    // Howard Hinnant'ın `civil_from_days` algoritması.
    let z = gun as i64 + 719_468;
    let cag = z.div_euclid(146_097);
    let gun_cag = z.rem_euclid(146_097);
    let yil_cag = (gun_cag - gun_cag / 1460 + gun_cag / 36_524 - gun_cag / 146_096) / 365;
    let yil_gunu = gun_cag - (365 * yil_cag + yil_cag / 4 - yil_cag / 100);
    let ay_p = (5 * yil_gunu + 2) / 153;
    let gun_ay = yil_gunu - (153 * ay_p + 2) / 5 + 1;
    let ay = if ay_p < 10 { ay_p + 3 } else { ay_p - 9 };
    let yil = yil_cag + cag * 400 + i64::from(ay <= 2);
    format!(
        "{yil:04}{ay:02}{gun_ay:02}T{:02}{:02}{:02}",
        gun_ici / 3600,
        gun_ici % 3600 / 60,
        gun_ici % 60
    )
}

/// Eksik crate'te modele giden ek (A15, 2d-3).
const CARGO_EKSIK_CRATE: &str = "Bağımlılık bu bilgisayarda indirilmemiş. Yalıtım içinde \
crates.io'dan paket açılamıyor (Windows sınırı). Geliştiricinin yalıtım dışında \
`cargo fetch` çalıştırması gerekiyor.";

/// Komut `cargo` mu: program adı `cargo` ya da betikte `cargo` geçiyor.
fn cargo_komutu_mu(komut: &KomutBicimi) -> bool {
    match komut {
        KomutBicimi::Program { program, .. } => Path::new(program)
            .file_stem()
            .is_some_and(|ad| ad.eq_ignore_ascii_case("cargo")),
        KomutBicimi::Betik { metin } => metin
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .any(|kelime| kelime.eq_ignore_ascii_case("cargo")),
    }
}

/// Windows'ta yalıtımlı ve başarısız bir cargo komutu crate bulamadıysa
/// (ağsız: geliştiricinin registry'sinde yok; ağlı: kapta açılamadı) modele
/// ne yapılacağı söylenir. İletiler cargo 1.98'inkilerdir.
fn cargo_talimati(
    windows: bool,
    yalitimli: bool,
    cargo: bool,
    basarili: bool,
    metin: &str,
) -> Option<&'static str> {
    let eksik = [
        "no matching package named",
        "failed to download",
        "failed to unpack package",
    ]
    .iter()
    .any(|iz| metin.contains(iz));
    (windows && yalitimli && cargo && !basarili && eksik).then_some(CARGO_EKSIK_CRATE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eksik_crate_hatasina_talimat_eklenir() {
        let ag_siz = "error: no matching package named `hex` found\n\
                      location searched: crates.io index\n\
                      note: offline mode (via `--offline`) can sometimes cause surprising resolution failures";
        let agli = "error: failed to unpack package `hex v0.4.3`\n  Erişim engellendi. (os error 5) while canonicalizing C:\\x";
        for metin in [ag_siz, agli] {
            assert_eq!(
                cargo_talimati(true, true, true, false, metin),
                Some(CARGO_EKSIK_CRATE)
            );
        }
        // Linux, yalıtımsız, cargo olmayan, başarılı ya da başka bir hata: ek yok.
        assert_eq!(cargo_talimati(false, true, true, false, ag_siz), None);
        assert_eq!(cargo_talimati(true, false, true, false, ag_siz), None);
        assert_eq!(cargo_talimati(true, true, false, false, ag_siz), None);
        assert_eq!(cargo_talimati(true, true, true, true, ag_siz), None);
        assert_eq!(
            cargo_talimati(true, true, true, false, "error[E0308]: mismatched types"),
            None
        );

        let program = |p: &str| KomutBicimi::Program {
            program: p.into(),
            argumanlar: Vec::new(),
        };
        assert!(cargo_komutu_mu(&program("cargo")));
        let tam_yol = if cfg!(windows) {
            r"C:\Users\x\.cargo\bin\CARGO.exe"
        } else {
            "/home/x/.cargo/bin/cargo"
        };
        assert!(cargo_komutu_mu(&program(tam_yol)));
        assert!(!cargo_komutu_mu(&program("cargo-foo")));
        assert!(!cargo_komutu_mu(&program("npm")));
    }

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
    fn utc_damgasi_bicimi() {
        use std::time::{Duration, UNIX_EPOCH};
        assert_eq!(utc_damgasi(UNIX_EPOCH), "19700101T000000");
        // 2026-09-28 13:45:07 UTC
        let an = UNIX_EPOCH + Duration::from_secs(1_790_603_107);
        assert_eq!(utc_damgasi(an), "20260928T134507");
        // Artık gün: 2024-02-29 23:59:59 UTC
        let an = UNIX_EPOCH + Duration::from_secs(1_709_251_199);
        assert_eq!(utc_damgasi(an), "20240229T235959");
    }

    #[test]
    fn aciklama_ilk_satiri_kisaltir() {
        assert_eq!(kisa("cargo test"), "cargo test");
        assert_eq!(kisa("echo a\necho b"), "echo a…");
        assert_eq!(kisa(&"x".repeat(100)), format!("{}…", "x".repeat(80)));
    }
}
