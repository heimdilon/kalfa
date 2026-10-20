//! `kabuk`: çalışma alanında komut çalıştırma (aşama 2 tasarımı §4).
//!
//! Aracın iki biçimi vardır: `{program, argumanlar}` kabuk yorumlayıcısı
//! olmadan çalışır; `{betik}` platformun kabuğunda. Komutu `kalfa-sandbox`'ın
//! yalıtıcısı başlatır. İzin kararı, onay kartı ve mühür hazırlıktaki
//! `KabukTarifi`'nden üretilir.

pub mod cikti;
pub mod isler;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, AracOrtami, CiktiGonderici, Hazirlik};
use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
use kalfa_core::kilit::KilitSeti;
use kalfa_core::protokol::VeriCikisi;
use kalfa_core::yol::CalismaYolu;
use kalfa_sandbox::tarama::AdliGirdiler;
use kalfa_sandbox::{
    AlanBilgisi, BaslatmaHatasi, Bitis, CalistirmaIstegi, KomutBicimi, TerminalBoyutu, Yalitici,
    YalitimDurumu, ortam, program,
};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::calisma_alani::CalismaAlani;
use crate::koruma::{self, SonDenetimSonucu};
use cikti::{BasSonTampon, CANLI_SINIR, CanliAkis};
use isler::{ArkaPlanBaslatmaMeta, ArkaPlanIsBaglantisi};

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
    kullanici_ag: Arc<std::sync::RwLock<Vec<kalfa_sandbox::vekil::HostKurali>>>,
    arka_plan: Option<Arc<dyn ArkaPlanIsBaglantisi>>,
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
            kullanici_ag: Arc::default(),
            arka_plan: None,
        }
    }

    /// Kullanıcının ağ izin listesi (aşama 3 tasarımı §8.3; 3e Görev 4).
    /// Her hazırlıkta ve çalıştırmada okunur: liste değişince profil özeti
    /// ve mühür değişir, bekleyen onay geçersizleşir.
    pub fn kullanici_ag_listesi(
        mut self,
        liste: Arc<std::sync::RwLock<Vec<kalfa_sandbox::vekil::HostKurali>>>,
    ) -> Self {
        self.kullanici_ag = liste;
        self
    }

    /// Arka plan işleri ve etkileşimli terminal (`G2b` / `G2c`) için oturum-bağlı
    /// iş yöneticisi bağlantısı.
    pub fn arka_plan_baglantisi(mut self, baglanti: Arc<dyn ArkaPlanIsBaglantisi>) -> Self {
        self.arka_plan = Some(baglanti);
        self
    }

    fn ag_izin_listesi(&self, ag_hostlari: &[String]) -> kalfa_sandbox::vekil::IzinListesi {
        let cagri: Vec<(String, u16)> = ag_hostlari
            .iter()
            .filter_map(|s| kalfa_sandbox::vekil::ag_hostu_coz(s).ok())
            .collect();
        let kullanici = self
            .kullanici_ag
            .read()
            .unwrap_or_else(|z| z.into_inner())
            .clone();
        kalfa_sandbox::vekil::IzinListesi::yeni(&kullanici, &cagri)
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
    ag_hostlari: Vec<String>,
    sure: Duration,
    /// Sonucun kaynak etiketi: `kabuk:<program>`.
    kaynak: String,
    arka_plan: bool,
    terminal: Option<TerminalBoyutu>,
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
        let ag_hostlari = match a.get("ag_hostlari") {
            None => Vec::new(),
            Some(Value::Array(l)) => {
                if !ag {
                    return Err(gecersiz(
                        "`ag_hostlari` yalnızca `ag: true` ile verilebilir.",
                    ));
                }
                if l.len() > 16 {
                    return Err(gecersiz("`ag_hostlari` en çok 16 öğe içerebilir."));
                }
                let mut cozulmus = Vec::new();
                let mut gorulen = std::collections::HashSet::new();
                for v in l {
                    let s = v
                        .as_str()
                        .ok_or_else(|| gecersiz("`ag_hostlari` yalnızca metin içerebilir."))?;
                    let (host, port) = kalfa_sandbox::vekil::ag_hostu_coz(s)
                        .map_err(|e| gecersiz(format!("Geçersiz ağ hostu `{s}`: {e}")))?;
                    let normal = format!("{host}:{port}");
                    if gorulen.insert(normal.clone()) {
                        cozulmus.push(normal);
                    }
                }
                cozulmus
            }
            Some(_) => return Err(gecersiz("`ag_hostlari` bir metin listesi olmalı.")),
        };
        let sure_sn = a
            .get("zaman_asimi_sn")
            .and_then(Value::as_u64)
            .unwrap_or(VARSAYILAN_SURE_SN)
            .clamp(1, AZAMI_SURE_SN);
        let arka_plan = a.get("arka_plan").and_then(Value::as_bool).unwrap_or(false);
        if arka_plan && self.arka_plan.is_none() {
            return Err(gecersiz("Bu oturumda arka plan kabuk işleri etkin değil."));
        }
        let terminal = match a.get("terminal") {
            None | Some(Value::Bool(false)) => None,
            Some(Value::Bool(true)) => {
                if !arka_plan {
                    return Err(gecersiz(
                        "`terminal` yalnızca `arka_plan: true` ile kullanılabilir.",
                    ));
                }
                Some(TerminalBoyutu::default())
            }
            Some(Value::Object(obj)) => {
                if !arka_plan {
                    return Err(gecersiz(
                        "`terminal` yalnızca `arka_plan: true` ile kullanılabilir.",
                    ));
                }
                let sutun = obj
                    .get("sutun")
                    .and_then(Value::as_u64)
                    .and_then(|n| u16::try_from(n).ok())
                    .unwrap_or(TerminalBoyutu::VARSAYILAN_SUTUN);
                let satir = obj
                    .get("satir")
                    .and_then(Value::as_u64)
                    .and_then(|n| u16::try_from(n).ok())
                    .unwrap_or(TerminalBoyutu::VARSAYILAN_SATIR);
                Some(TerminalBoyutu::yeni(sutun, satir).map_err(|e| gecersiz(e.to_string()))?)
            }
            Some(_) => {
                return Err(gecersiz(
                    "`terminal` mantıksal değer veya `{sutun, satir}` nesnesi olmalı.",
                ));
            }
        };
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
            ag_hostlari,
            sure: Duration::from_secs(sure_sn),
            kaynak,
            arka_plan,
            terminal,
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
    fn yalitim_ozeti(&self, ag: bool, liste_ozeti: Option<&str>) -> YalitimOzeti {
        match self.yalitici.durum() {
            YalitimDurumu::Hazir { arka_uc, surum } => YalitimOzeti::Yalitimli {
                arka_uc,
                profil: self.yalitici.profil_ozeti(ag, liste_ozeti).unwrap_or(surum),
            },
            YalitimDurumu::Yok { talimat, .. } => YalitimOzeti::Yalitimsiz { talimat },
        }
    }

    /// Yalıtımın çalışma alanı bilgisi; her çalıştırmadan hemen önce üretilir
    /// (§7.2): gizli dosya taraması ve git yerleşimi.
    async fn alan_bilgisi(&self) -> Result<AlanBilgisi, AracHatasi> {
        koruma::alan_bilgisi(self.alan.clone()).await
    }

    /// Çalışma alanındaki `.git` girdileri ve yapılandırma yolları;
    /// yalıtımsız komuttan önce.
    async fn adli_girdiler(&self) -> Result<AdliGirdiler, AracHatasi> {
        koruma::adli_girdiler(self.alan.clone()).await
    }

    /// Komuttan sonra yeni `.git` girdilerini denetler ve tehlikelileri
    /// karantinaya alır (güvenlik analizi Bulgu 1); yeni yapılandırma
    /// dosyalarını bildirir (Bulgu 2). Aynı taramanın görüntüsü komutun
    /// değiştirdiği dosyaları verir (aşama 3 tasarımı §5.4). Uyarıları, bir
    /// karantinanın başarısız olup olmadığını ve değişiklikleri döner.
    async fn komut_sonrasi_denetim(&self, onceki: AdliGirdiler) -> SonDenetimSonucu {
        koruma::komut_sonrasi_denetim(self.alan.clone(), onceki).await
    }

    /// Çalışma alanını yalıtım için hazırlar (§9.4); ilerleme araç kartında
    /// görünür, modele gitmez.
    async fn alani_hazirla(
        &self,
        alan: &AlanBilgisi,
        onaylandi: bool,
        gonderici: &CiktiGonderici,
    ) -> Result<(), AracHatasi> {
        koruma::alani_hazirla(self.yalitici.clone(), alan, onaylandi, gonderici).await
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
         kipte onay gerektirir. Ağ açıkken yalnızca izin listesindeki paket kayıtlarına ve \
         GitHub indirme uçlarına erişilebilir; `github.com` varsayılan listede yoktur, Git \
         ve GitHub indirmeleri için `ag_hostlari` ile istenmelidir. `ag_hostlari` en çok 16 \
         öğe alabilir. Standart girdi kapalıdır; etkileşim bekleyen komut çalıştırma. \
         Çıktı uzunsa baştan ve sondan kırpılır."
    }

    fn parametre_semasi(&self) -> Value {
        let mut semasi = json!({
            "type": "object",
            "properties": {
                "program": {"type": "string", "description": "Çıplak program adı (PATH'te aranır) ya da çalışma alanına göre yol"},
                "argumanlar": {"type": "array", "items": {"type": "string"}},
                "betik": {"type": "string", "description": "Kabukta çalışacak metin"},
                "dizin": {"type": "string", "description": "Çalışma alanına göre çalışma dizini; varsayılan kök"},
                "ag": {"type": "boolean", "default": false},
                "ag_hostlari": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Ağ açıkken bağlanılacak ek hostlar (host veya host:port; en çok 16 adet; github.com varsayılan listede yoktur)"
                },
                "zaman_asimi_sn": {"type": "integer", "minimum": 1, "maximum": AZAMI_SURE_SN, "default": VARSAYILAN_SURE_SN}
            },
            "oneOf": [{"required": ["program"]}, {"required": ["betik"]}],
            "additionalProperties": false
        });
        if self.arka_plan.is_some()
            && let Some(props) = semasi.get_mut("properties").and_then(Value::as_object_mut)
        {
            props.insert(
                "arka_plan".to_string(),
                json!({
                    "type": "boolean",
                    "default": false,
                    "description": "Komutu oturuma bağlı arka plan işi olarak başlatır ve `is_id` döner"
                }),
            );
            props.insert(
                "terminal".to_string(),
                json!({
                    "description": "Yalnızca `arka_plan: true` ile sözde terminal (PTY/ConPTY) açar; mantıksal değer veya `{sutun, satir}`",
                    "oneOf": [
                        {"type": "boolean"},
                        {
                            "type": "object",
                            "properties": {
                                "sutun": {"type": "integer", "minimum": TerminalBoyutu::EN_AZ_SUTUN, "maximum": TerminalBoyutu::EN_COK_SUTUN, "default": TerminalBoyutu::VARSAYILAN_SUTUN},
                                "satir": {"type": "integer", "minimum": TerminalBoyutu::EN_AZ_SATIR, "maximum": TerminalBoyutu::EN_COK_SATIR, "default": TerminalBoyutu::VARSAYILAN_SATIR}
                            },
                            "additionalProperties": false
                        }
                    ]
                }),
            );
        }
        semasi
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
        let (bicim, program_adi) = match &istek.komut {
            // Arka plan işlerinde `program: None` bırakılır ki tek seferlik program kısayolu
            // (`KisayolTeklifi::Kabuk`) uzun süreli veya etkileşimli işlere devredilmesin.
            KomutBicimi::Program { program, .. } => (
                KabukBicimi::Program,
                (!istek.arka_plan).then(|| program.clone()),
            ),
            KomutBicimi::Betik { .. } => (
                KabukBicimi::Betik {
                    yorumlayici: program::yorumlayici_adi().to_string(),
                },
                None,
            ),
        };
        let liste_ozeti = istek
            .ag
            .then(|| self.ag_izin_listesi(&istek.ag_hostlari).ozet());
        let yalitim = self.yalitim_ozeti(istek.ag, liste_ozeti.as_deref());
        if !yalitim.yalitimli_mi() && !self.izin.izinli_mi() {
            return Err(gecersiz(YALITIMSIZ_KAPALI));
        }
        // Windows'ta çalışma alanı ilk yalıtımlı çağrıda hazırlanır (§9.4).
        let alan_hazirligi =
            yalitim.yalitimli_mi() && self.yalitici.hazirlik_gerekli(self.alan.kok());
        let gosterim = match (istek.arka_plan, istek.terminal) {
            (false, _) => istek.gosterim.clone(),
            (true, None) => format!("[arka_plan] {}", istek.gosterim),
            (true, Some(boyut)) => format!(
                "[arka_plan:terminal:{}x{}] {}",
                boyut.sutun, boyut.satir, istek.gosterim
            ),
        };
        Ok(Hazirlik {
            aciklama: if istek.arka_plan {
                format!(
                    "`{}` komutunu arka plan işi olarak başlatmak istiyor.",
                    kisa(&istek.gosterim)
                )
            } else {
                format!("`{}` komutunu çalıştırmak istiyor.", kisa(&istek.gosterim))
            },
            tum_calisma_alani: true,
            veri_cikisi: Some(VeriCikisi::KomutCiktisi),
            kabuk: Some(KabukTarifi {
                komut: gosterim,
                bicim,
                program: program_adi,
                dizin: istek
                    .dizin
                    .as_ref()
                    .map_or_else(|| ".".to_string(), |d| d.as_str().to_string()),
                ag: istek.ag,
                ag_hostlari: istek.ag_hostlari,
                yalitim,
                // Arka plan işleri her zaman açık onay gerektirir.
                ilk_cagri: istek.arka_plan || !self.guven.guvenildi_mi(),
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
        self.calistir_ic(a, h, iptal, gonderici, None).await
    }

    async fn calistir_kilitli(
        &self,
        a: &Value,
        h: &Hazirlik,
        iptal: CancellationToken,
        ortam: AracOrtami,
        kilit: Option<KilitSeti>,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.calistir_ic(a, h, iptal, ortam.cikti(), kilit).await
    }
}

impl KabukAraci {
    async fn calistir_ic(
        &self,
        a: &Value,
        h: &Hazirlik,
        iptal: CancellationToken,
        gonderici: CiktiGonderici,
        kilit: Option<KilitSeti>,
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
            Some(b) => koruma::alan_bilgisinden_adli_girdiler(b),
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
        let ag_listesi = istek.ag.then(|| self.ag_izin_listesi(&istek.ag_hostlari));
        let calistirma = CalistirmaIstegi {
            komut,
            dizin,
            ortam: ortam::uret(std::env::vars()),
            ag: istek.ag,
            ag_listesi,
            alan,
        };
        if istek.arka_plan {
            let Some(baglanti) = &self.arka_plan else {
                return Err(gecersiz("Bu oturumda arka plan kabuk işleri etkin değil."));
            };
            let Some(kilit) = kilit else {
                return Err(gecersiz(
                    "Arka plan kabuk işi çalışma alanı yazma kilidi olmadan başlatılamaz.",
                ));
            };
            let meta = ArkaPlanBaslatmaMeta {
                gosterim: istek.gosterim,
                dizin: istek
                    .dizin
                    .as_ref()
                    .map_or_else(|| ".".to_string(), |d| d.as_str().to_string()),
                ag: istek.ag,
                ag_hostlari: istek.ag_hostlari,
                yalitimli,
                terminal: istek.terminal,
                sure: istek.sure,
            };
            let is_id = baglanti
                .is_baslat(
                    self.yalitici.as_ref(),
                    &calistirma,
                    meta,
                    kilit,
                    self.alan.clone(),
                    onceki,
                )
                .await?;
            self.guven.guvenildi();
            let kip_etiketi = match istek.terminal {
                Some(b) => format!("terminal ({}x{})", b.sutun, b.satir),
                None => "borulu".to_string(),
            };
            return Ok(AracCiktisi {
                icerik: format!(
                    "Arka plan işi başlatıldı.\nİş: {is_id}\nKip: {kip_etiketi}\nDurum: çalışıyor"
                ),
                kaynak: istek.kaynak,
                kalfa_uretimi: true,
                ..Default::default()
            });
        }
        struct KabukYazmaBitisKoruma<'a>(&'a CalismaAlani);
        impl Drop for KabukYazmaBitisKoruma<'_> {
            fn drop(&mut self) {
                self.0.yazma_bitti_bildir();
            }
        }
        self.alan.yazma_basladi_bildir();
        let _yazma_bitis = KabukYazmaBitisKoruma(&self.alan);
        let surec = match self.yalitici.baslat(&calistirma) {
            Ok(surec) => surec,
            Err(e) => {
                if matches!(e, BaslatmaHatasi::Yalitim(_)) {
                    self.yalitici.yeniden_algila();
                }
                drop(_yazma_bitis);
                self.alan.yazma_bitisini_bekle().await?;
                return Err(gecersiz(e.to_string()));
            }
        };
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
        let SonDenetimSonucu {
            uyarilar: guvenlik_uyarilari,
            karantina_basarisiz,
            degisiklikler,
            ..
        } = self.komut_sonrasi_denetim(onceki).await;
        drop(_yazma_bitis);
        self.alan.yazma_bitisini_bekle().await?;
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
        let ag_satirlari = ag_satirlari_uret(sonuc.ag.as_ref());
        let ag_metni = if ag_satirlari.is_empty() {
            String::new()
        } else {
            format!("\n{}", ag_satirlari.join("\n"))
        };
        let ag_ozeti = ag_komut_ozeti(sonuc.ag.as_ref());
        let cikti = AracCiktisi {
            icerik: format!("{durum}\nSüre: {sure} sn\n{govde}{ag_metni}"),
            kaynak: istek.kaynak,
            guvenlik_uyarilari,
            degisiklikler: degisiklikler.map(Box::new),
            kalfa_uretimi: false,
            ag: ag_ozeti.map(Box::new),
        };
        if sonuc.bitis == Bitis::Cikti(Some(0)) && !karantina_basarisiz {
            Ok(cikti)
        } else {
            Err(AracHatasi::Basarisiz(cikti))
        }
    }
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

/// Ağ özetinden reddedilen bağlantılar için modelin göreceği `[ağ]` satırlarını üretir.
fn ag_satirlari_uret(ag: Option<&kalfa_sandbox::vekil::AgOzeti>) -> Vec<String> {
    use kalfa_sandbox::vekil::AgKarari;
    let Some(ag) = ag else {
        return Vec::new();
    };
    let kurumsal =
        std::env::var_os("HTTPS_PROXY").is_some() || std::env::var_os("https_proxy").is_some();
    let mut sirali: Vec<(String, u32)> = Vec::new();
    for b in &ag.baglantilar {
        let satir = match &b.karar {
            AgKarari::Izinli => continue,
            AgKarari::ListedeYok | AgKarari::IpIleIstek => {
                format!(
                    "[ağ] `{}:{}` engellendi (izin listesinde yok)",
                    b.host, b.port
                )
            }
            AgKarari::YasakAdres(_) => {
                format!(
                    "[ağ] `{}:{}` engellendi: özel ya da yerel bir adrese çözüldü",
                    b.host, b.port
                )
            }
            AgKarari::SniUyusmuyor { .. } => {
                format!("[ağ] `{}:{}` engellendi: SNI farklı", b.host, b.port)
            }
            AgKarari::Cozulemedi(_) => {
                if kurumsal {
                    format!(
                        "[ağ] `{}:{}` çözülemedi (kurumsal vekil bu sürümde desteklenmiyor)",
                        b.host, b.port
                    )
                } else {
                    format!("[ağ] `{}:{}` çözülemedi", b.host, b.port)
                }
            }
            AgKarari::Baglanamadi(_) => {
                if kurumsal {
                    format!(
                        "[ağ] `{}:{}` bağlanılamadı (kurumsal vekil bu sürümde desteklenmiyor)",
                        b.host, b.port
                    )
                } else {
                    format!("[ağ] `{}:{}` bağlanılamadı", b.host, b.port)
                }
            }
            AgKarari::Gecersiz(_) => {
                format!("[ağ] `{}:{}` engellendi (geçersiz istek)", b.host, b.port)
            }
        };
        if let Some((_, sayi)) = sirali.iter_mut().find(|(s, _)| s == &satir) {
            *sayi += 1;
        } else {
            sirali.push((satir, 1));
        }
    }
    sirali
        .into_iter()
        .map(|(satir, sayi)| {
            if sayi > 1 {
                format!("{satir} ({sayi})")
            } else {
                satir
            }
        })
        .collect()
}

/// Ağ özetinden tur özetine gidecek `AgKomutOzeti`'ni üretir.
fn ag_komut_ozeti(
    ag: Option<&kalfa_sandbox::vekil::AgOzeti>,
) -> Option<kalfa_core::tur_ozeti::AgKomutOzeti> {
    let ag = ag?;
    if ag.baglantilar.is_empty() {
        return None;
    }
    use kalfa_core::tur_ozeti::{AgHostOzeti, AgKomutOzeti};
    let mut hostlar: Vec<AgHostOzeti> = Vec::new();
    let mut toplam_giden = 0u64;
    let mut toplam_gelen = 0u64;
    let mut reddedilen_sayisi = 0u32;

    for b in &ag.baglantilar {
        toplam_giden = toplam_giden.saturating_add(b.giden);
        toplam_gelen = toplam_gelen.saturating_add(b.gelen);
        let ret = b.karar != kalfa_sandbox::vekil::AgKarari::Izinli;
        if ret {
            reddedilen_sayisi = reddedilen_sayisi.saturating_add(1);
        }
        if let Some(h) = hostlar
            .iter_mut()
            .find(|h| h.host == b.host && h.port == b.port)
        {
            h.baglanti_sayisi = h.baglanti_sayisi.saturating_add(1);
            h.giden_bayt = h.giden_bayt.saturating_add(b.giden);
            h.gelen_bayt = h.gelen_bayt.saturating_add(b.gelen);
            if ret {
                h.reddedilenler = h.reddedilenler.saturating_add(1);
            }
        } else {
            hostlar.push(AgHostOzeti {
                host: b.host.clone(),
                port: b.port,
                baglanti_sayisi: 1,
                giden_bayt: b.giden,
                gelen_bayt: b.gelen,
                reddedilenler: if ret { 1 } else { 0 },
            });
        }
    }

    Some(AgKomutOzeti {
        hostlar,
        toplam_giden,
        toplam_gelen,
        reddedilen_sayisi,
    })
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
        assert_eq!(koruma::utc_damgasi(UNIX_EPOCH), "19700101T000000");
        // 2026-09-28 13:45:07 UTC
        let an = UNIX_EPOCH + Duration::from_secs(1_790_603_107);
        assert_eq!(koruma::utc_damgasi(an), "20260928T134507");
        // Artık gün: 2024-02-29 23:59:59 UTC
        let an = UNIX_EPOCH + Duration::from_secs(1_709_251_199);
        assert_eq!(koruma::utc_damgasi(an), "20240229T235959");
    }

    #[test]
    fn aciklama_ilk_satiri_kisaltir() {
        assert_eq!(kisa("cargo test"), "cargo test");
        assert_eq!(kisa("echo a\necho b"), "echo a…");
        assert_eq!(kisa(&"x".repeat(100)), format!("{}…", "x".repeat(80)));
    }
}
