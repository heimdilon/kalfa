//! Arka plan kabuk işleri ve etkileşimli terminal kontrol araçları (`G2b` / `G2c`).
//!
//! `kabuk` aracının `arka_plan: true` (ve isteğe bağlı `terminal`) kipiyle
//! başlatılan işleri oturum sınırları içinde yönetmek için:
//! - `komut_oku`: imleçli çıktı okuma (`kilitsiz: true`, denetçi ve kaynak etiketine tabi),
//! - `komut_girdi`: ayrı mühürlü (`OnayMuhru`) standart girdi yazma (`kilitsiz: true`),
//! - `komut_durdur`: süreç ağacını durdurma ve son denetimi tamamlama (`kilitsiz: true`),
//! - `komut_boyutlandir`: sözde terminal (`PTY` / `ConPTY`) pencere boyutunu güncelleme.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, Hazirlik};
use kalfa_core::gorunum::gorunmez_var_mi;
use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
use kalfa_core::kilit::KilitSeti;
use kalfa_core::protokol::VeriCikisi;
use kalfa_sandbox::tarama::AdliGirdiler;
use kalfa_sandbox::{CalistirmaIstegi, TerminalBoyutu, Yalitici, YalitimDurumu};
use serde_json::{Value, json};
pub use tokio_util::sync::CancellationToken;

use crate::calisma_alani::CalismaAlani;
use crate::kabuk::{YALITIMSIZ_KAPALI, YalitimsizIzni, kisa, metin};

/// Tek `komut_girdi` çağrısında gönderilebilecek azami bayt (`64 KiB`).
pub const AZAMI_GIRDI_BAYT: usize = 65_536;

/// Arka plan işi başlatılırken `KabukAraci` tarafından gönderilen üst veri.
#[derive(Debug, Clone)]
pub struct ArkaPlanBaslatmaMeta {
    pub gosterim: String,
    pub dizin: String,
    pub ag: bool,
    pub ag_hostlari: Vec<String>,
    pub yalitimli: bool,
    pub terminal: Option<TerminalBoyutu>,
    pub sure: Duration,
}

/// Etkin veya tamamlanmış bir arka plan işinin yetki ve tarif özeti.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArkaPlanIsBilgisi {
    pub is_id: String,
    pub gosterim: String,
    pub dizin: String,
    pub ag: bool,
    pub ag_hostlari: Vec<String>,
    pub yalitimli: bool,
    pub terminal: Option<TerminalBoyutu>,
    pub calisiyor: bool,
}

/// `komut_oku` aracının döndürdüğü okuma sonucu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArkaPlanOkumaSonucu {
    pub is_id: String,
    pub durum_etiketi: String,
    pub cikti: String,
    pub yeni_imlec: usize,
    pub toplam_bayt: u64,
    pub tasti: bool,
    pub uyarilar: Vec<String>,
}

/// `komut_durdur` aracının döndürdüğü kapanış sonucu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArkaPlanDurdurmaSonucu {
    pub is_id: String,
    pub durum_etiketi: String,
    pub cikis_kodu: Option<i32>,
    pub sure_ms: u64,
    pub zaman_asimi: bool,
    pub uyarilar: Vec<String>,
}

/// Uygulama katmanındaki oturum-bağlı iş yöneticisi (`kalfa-uygulama::isler`)
/// ile `kalfa-tools` arasındaki sözleşme.
#[async_trait]
pub trait ArkaPlanIsBaglantisi: Send + Sync {
    async fn is_baslat(
        &self,
        yalitici: &dyn Yalitici,
        istek: &CalistirmaIstegi,
        meta: ArkaPlanBaslatmaMeta,
        kilit: KilitSeti,
        alan: Arc<CalismaAlani>,
        onceki: AdliGirdiler,
    ) -> Result<String, AracHatasi>;

    fn is_bilgisi(&self, is_id: &str) -> Result<ArkaPlanIsBilgisi, AracHatasi>;

    fn is_oku(&self, is_id: &str, imlec: usize) -> Result<ArkaPlanOkumaSonucu, AracHatasi>;

    async fn is_girdi(&self, is_id: &str, veri: &[u8], eof: bool) -> Result<String, AracHatasi>;

    fn is_boyutlandir(&self, is_id: &str, boyut: TerminalBoyutu) -> Result<String, AracHatasi>;

    async fn is_durdur(&self, is_id: &str) -> Result<ArkaPlanDurdurmaSonucu, AracHatasi>;
}

fn gecersiz(m: impl Into<String>) -> AracHatasi {
    AracHatasi::Gecersiz(m.into())
}

fn yalitim_ozeti_hesapla(
    yalitici: &dyn Yalitici,
    kullanici_ag: &std::sync::RwLock<Vec<kalfa_sandbox::vekil::HostKurali>>,
    ag: bool,
    ag_hostlari: &[String],
) -> YalitimOzeti {
    let liste_ozeti = ag.then(|| {
        let cagri: Vec<(String, u16)> = ag_hostlari
            .iter()
            .filter_map(|s| kalfa_sandbox::vekil::ag_hostu_coz(s).ok())
            .collect();
        let kullanici = kullanici_ag
            .read()
            .unwrap_or_else(|z| z.into_inner())
            .clone();
        kalfa_sandbox::vekil::IzinListesi::yeni(&kullanici, &cagri).ozet()
    });
    match yalitici.durum() {
        YalitimDurumu::Hazir { arka_uc, surum } => YalitimOzeti::Yalitimli {
            arka_uc,
            profil: yalitici
                .profil_ozeti(ag, liste_ozeti.as_deref())
                .unwrap_or(surum),
        },
        YalitimDurumu::Yok { talimat, .. } => YalitimOzeti::Yalitimsiz { talimat },
    }
}

/// Oturum için arka plan iş kontrol araçlarını (`komut_oku`, `komut_girdi`,
/// `komut_durdur`, `komut_boyutlandir`) üretir.
pub fn arka_plan_araclari(
    baglanti: Arc<dyn ArkaPlanIsBaglantisi>,
    yalitici: Arc<dyn Yalitici>,
    izin: Arc<dyn YalitimsizIzni>,
    kullanici_ag: Arc<std::sync::RwLock<Vec<kalfa_sandbox::vekil::HostKurali>>>,
) -> Vec<Arc<dyn Arac>> {
    vec![
        Arc::new(KomutOkuAraci::yeni(baglanti.clone())) as Arc<dyn Arac>,
        Arc::new(KomutGirdiAraci::yeni(
            baglanti.clone(),
            yalitici,
            izin,
            kullanici_ag,
        )) as Arc<dyn Arac>,
        Arc::new(KomutDurdurAraci::yeni(baglanti.clone())) as Arc<dyn Arac>,
        Arc::new(KomutBoyutlandirAraci::yeni(baglanti)) as Arc<dyn Arac>,
    ]
}

/// `komut_oku`: arka plan işinin çıktısını imleçten itibaren okur.
pub struct KomutOkuAraci {
    baglanti: Arc<dyn ArkaPlanIsBaglantisi>,
}

impl KomutOkuAraci {
    pub fn yeni(baglanti: Arc<dyn ArkaPlanIsBaglantisi>) -> Self {
        Self { baglanti }
    }
}

#[async_trait]
impl Arac for KomutOkuAraci {
    fn ad(&self) -> &str {
        "komut_oku"
    }

    fn aciklama(&self) -> &str {
        "Bu oturumda başlatılmış bir arka plan işinin (`is_id`) yeni çıktısını verilen \
         bayt imlecinden (`imlec`) itibaren okur ve güncel durumunu döner."
    }

    fn parametre_semasi(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "is_id": {"type": "string", "description": "Uygulamanın verdiği iş kimliği (is-...)"},
                "imlec": {"type": "integer", "minimum": 0, "default": 0, "description": "Okunacak başlangıç bayt imleci"}
            },
            "required": ["is_id"],
            "additionalProperties": false
        })
    }

    fn yazar_mi(&self) -> bool {
        false
    }

    async fn hazirla(&self, a: &Value) -> Result<Hazirlik, AracHatasi> {
        let is_id = metin(a, "is_id").ok_or_else(|| gecersiz("`is_id` zorunludur."))?;
        let _bilgi = self.baglanti.is_bilgisi(is_id)?;
        let _imlec = match a.get("imlec") {
            None => 0usize,
            Some(v) => v
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(|| gecersiz("`imlec` negatif olmayan bir tam sayı olmalı."))?,
        };
        Ok(Hazirlik {
            aciklama: format!("`{is_id}` işinin çıktısını okumak istiyor."),
            veri_cikisi: Some(VeriCikisi::KomutCiktisi),
            kilitsiz: true,
            ..Default::default()
        })
    }

    async fn calistir(
        &self,
        a: &Value,
        _h: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        if iptal.is_cancelled() {
            return Err(AracHatasi::Iptal);
        }
        let is_id = metin(a, "is_id").ok_or_else(|| gecersiz("`is_id` zorunludur."))?;
        let imlec = a
            .get("imlec")
            .and_then(Value::as_u64)
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(0);
        let sonuc = self.baglanti.is_oku(is_id, imlec)?;
        let tasti_notu = if sonuc.tasti {
            "\n[not] Çıktı tamponu 1 MiB sınırına ulaştı; sonraki baytlar atıldı."
        } else {
            ""
        };
        let govde = if sonuc.cikti.is_empty() {
            "--- yeni çıktı yok ---".to_string()
        } else {
            format!("--- çıktı ---\n{}", sonuc.cikti)
        };
        Ok(AracCiktisi {
            icerik: format!(
                "İş: {}\nDurum: {}\nYeni imleç: {}\nToplam bayt: {}{}\n{}",
                sonuc.is_id,
                sonuc.durum_etiketi,
                sonuc.yeni_imlec,
                sonuc.toplam_bayt,
                tasti_notu,
                govde
            ),
            kaynak: format!("kabuk:is:{}", sonuc.is_id),
            guvenlik_uyarilari: sonuc.uyarilar,
            kalfa_uretimi: false,
            ..Default::default()
        })
    }
}

/// `komut_girdi`: çalışan arka plan işinin standart girdisine ayrı mühürlü onay
/// ile veri yazar.
pub struct KomutGirdiAraci {
    baglanti: Arc<dyn ArkaPlanIsBaglantisi>,
    yalitici: Arc<dyn Yalitici>,
    izin: Arc<dyn YalitimsizIzni>,
    kullanici_ag: Arc<std::sync::RwLock<Vec<kalfa_sandbox::vekil::HostKurali>>>,
}

impl KomutGirdiAraci {
    pub fn yeni(
        baglanti: Arc<dyn ArkaPlanIsBaglantisi>,
        yalitici: Arc<dyn Yalitici>,
        izin: Arc<dyn YalitimsizIzni>,
        kullanici_ag: Arc<std::sync::RwLock<Vec<kalfa_sandbox::vekil::HostKurali>>>,
    ) -> Self {
        Self {
            baglanti,
            yalitici,
            izin,
            kullanici_ag,
        }
    }

    fn coz<'a>(&self, a: &'a Value) -> Result<(&'a str, &'a str, bool), AracHatasi> {
        let is_id = metin(a, "is_id").ok_or_else(|| gecersiz("`is_id` zorunludur."))?;
        let girdi = match a.get("girdi") {
            None => "",
            Some(v) => v
                .as_str()
                .ok_or_else(|| gecersiz("`girdi` metin olmalı."))?,
        };
        let eof = a.get("eof").and_then(Value::as_bool).unwrap_or(false);
        if girdi.is_empty() && !eof {
            return Err(gecersiz(
                "`girdi` metni ya da `eof: true` değerinden en az biri verilmeli.",
            ));
        }
        if girdi.len() > AZAMI_GIRDI_BAYT {
            return Err(gecersiz(format!(
                "`girdi` en çok {AZAMI_GIRDI_BAYT} bayt olabilir."
            )));
        }
        Ok((is_id, girdi, eof))
    }
}

fn girdi_gosterimi(is_id: &str, girdi: &str, eof: bool) -> String {
    match (girdi.is_empty(), eof) {
        (true, true) => format!("[girdi -> {is_id}] <EOF>"),
        (false, true) => format!("[girdi -> {is_id} + EOF] {girdi}"),
        (false, false) => format!("[girdi -> {is_id}] {girdi}"),
        (true, false) => format!("[girdi -> {is_id}]"),
    }
}

#[async_trait]
impl Arac for KomutGirdiAraci {
    fn ad(&self) -> &str {
        "komut_girdi"
    }

    fn aciklama(&self) -> &str {
        "Çalışmakta olan bir arka plan işinin (`is_id`) standart girdisine metin gönderir \
         veya `eof: true` ile standart girdiyi kapatır. Her girdi çağrısı ayrı izin ve \
         mühür denetimine tabidir."
    }

    fn parametre_semasi(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "is_id": {"type": "string", "description": "Uygulamanın verdiği iş kimliği (is-...)"},
                "girdi": {"type": "string", "description": "Standart girdiye yazılacak metin (en çok 64 KiB)"},
                "eof": {"type": "boolean", "default": false, "description": "Yazmadan sonra standart girdiyi kapat"}
            },
            "required": ["is_id"],
            "additionalProperties": false
        })
    }

    fn yazar_mi(&self) -> bool {
        true
    }

    fn sunulur_mu(&self) -> bool {
        self.yalitici.durum().hazir_mi() || self.izin.izinli_mi()
    }

    async fn hazirla(&self, a: &Value) -> Result<Hazirlik, AracHatasi> {
        let (is_id, girdi, eof) = self.coz(a)?;
        let bilgi = self.baglanti.is_bilgisi(is_id)?;
        if !bilgi.calisiyor {
            return Err(gecersiz(
                "İş artık çalışmıyor veya durduruluyor; girdi gönderilemez.",
            ));
        }
        let yalitim = yalitim_ozeti_hesapla(
            self.yalitici.as_ref(),
            &self.kullanici_ag,
            bilgi.ag,
            &bilgi.ag_hostlari,
        );
        if bilgi.yalitimli != yalitim.yalitimli_mi() {
            return Err(gecersiz(
                "Yalıtım durumu iş başlatıldıktan sonra değişti; girdi gönderilemez.",
            ));
        }
        if !yalitim.yalitimli_mi() && !self.izin.izinli_mi() {
            return Err(gecersiz(YALITIMSIZ_KAPALI));
        }
        let gosterim = girdi_gosterimi(&bilgi.is_id, girdi, eof);
        Ok(Hazirlik {
            aciklama: format!(
                "`{}` işine standart girdi göndermek istiyor: `{}`.",
                bilgi.is_id,
                kisa(if girdi.is_empty() { "<EOF>" } else { girdi })
            ),
            gorunmez_icerik: gorunmez_var_mi(girdi),
            veri_cikisi: Some(VeriCikisi::YalnizcaYazildi),
            // İşin kendisi çalışma alanı yazma kilidini tutar; kontrol aracı
            // aynı kilidi bekleyerek kilitlenmez.
            kilitsiz: true,
            kabuk: Some(KabukTarifi {
                komut: gosterim,
                // Betik biçimi ve `program: None`, program kısayollarının (`KisayolTeklifi::Kabuk`)
                // yeni stdin komutlarını otomatik onaylamasını engeller.
                bicim: KabukBicimi::Betik {
                    yorumlayici: "stdin".to_string(),
                },
                program: None,
                dizin: bilgi.dizin,
                ag: bilgi.ag,
                ag_hostlari: bilgi.ag_hostlari,
                yalitim,
                // Her yeni stdin eylemi kendi onayını ister; ilk başlatma izni
                // sınırsız etkileşimli komut izni sayılmaz.
                ilk_cagri: true,
                alan_hazirligi: false,
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
        if iptal.is_cancelled() {
            return Err(AracHatasi::Iptal);
        }
        let (is_id, girdi, eof) = self.coz(a)?;
        let yalitimli = match h.kabuk.as_ref().map(|t| &t.yalitim) {
            Some(YalitimOzeti::Yalitimli { .. }) => true,
            Some(YalitimOzeti::Yalitimsiz { .. }) => false,
            None => return Err(gecersiz("Hazırlıkta kabuk girdi tarifi yok.")),
        };
        if !yalitimli && !self.izin.izinli_mi() {
            return Err(gecersiz(YALITIMSIZ_KAPALI));
        }
        let bilgi = self.baglanti.is_bilgisi(is_id)?;
        if bilgi.yalitimli != yalitimli {
            return Err(gecersiz(
                "Yalıtım durumu onaydan sonra değişti; girdi gönderilemez.",
            ));
        }
        let durum_etiketi = self.baglanti.is_girdi(is_id, girdi.as_bytes(), eof).await?;
        Ok(AracCiktisi {
            icerik: format!(
                "İş: {}\nGönderilen bayt: {}\nEOF: {}\nDurum: {}",
                bilgi.is_id,
                girdi.len(),
                if eof { "evet" } else { "hayır" },
                durum_etiketi
            ),
            kaynak: format!("kabuk:girdi:{}", bilgi.is_id),
            kalfa_uretimi: true,
            ..Default::default()
        })
    }
}

/// `komut_durdur`: çalışan arka plan işini ve süreç ağacını durdurur.
pub struct KomutDurdurAraci {
    baglanti: Arc<dyn ArkaPlanIsBaglantisi>,
}

impl KomutDurdurAraci {
    pub fn yeni(baglanti: Arc<dyn ArkaPlanIsBaglantisi>) -> Self {
        Self { baglanti }
    }
}

#[async_trait]
impl Arac for KomutDurdurAraci {
    fn ad(&self) -> &str {
        "komut_durdur"
    }

    fn aciklama(&self) -> &str {
        "Bu oturumda başlatılmış bir arka plan işini (`is_id`) ve bütün alt süreçlerini \
         durdurur, son dosya denetimini çalıştırır ve çalışma alanı kilidini serbest bırakır."
    }

    fn parametre_semasi(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "is_id": {"type": "string", "description": "Uygulamanın verdiği iş kimliği (is-...)"}
            },
            "required": ["is_id"],
            "additionalProperties": false
        })
    }

    fn yazar_mi(&self) -> bool {
        true
    }

    async fn hazirla(&self, a: &Value) -> Result<Hazirlik, AracHatasi> {
        let is_id = metin(a, "is_id").ok_or_else(|| gecersiz("`is_id` zorunludur."))?;
        let bilgi = self.baglanti.is_bilgisi(is_id)?;
        Ok(Hazirlik {
            aciklama: format!("`{}` arka plan işini durdurmak istiyor.", bilgi.is_id),
            veri_cikisi: Some(VeriCikisi::YalnizcaYazildi),
            kilitsiz: true,
            ..Default::default()
        })
    }

    async fn calistir(
        &self,
        a: &Value,
        _h: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        if iptal.is_cancelled() {
            return Err(AracHatasi::Iptal);
        }
        let is_id = metin(a, "is_id").ok_or_else(|| gecersiz("`is_id` zorunludur."))?;
        let sonuc = self.baglanti.is_durdur(is_id).await?;
        let cikis = match sonuc.cikis_kodu {
            Some(kod) => kod.to_string(),
            None => "sinyal/zorunlu".to_string(),
        };
        Ok(AracCiktisi {
            icerik: format!(
                "İş: {}\nDurum: {}\nÇıkış kodu: {}\nSüre: {} ms",
                sonuc.is_id, sonuc.durum_etiketi, cikis, sonuc.sure_ms
            ),
            kaynak: format!("kabuk:durdur:{}", sonuc.is_id),
            guvenlik_uyarilari: sonuc.uyarilar,
            kalfa_uretimi: true,
            ..Default::default()
        })
    }
}

/// `komut_boyutlandir`: sözde terminal (`PTY` / `ConPTY`) kipindeki işin boyutunu değiştirir.
pub struct KomutBoyutlandirAraci {
    baglanti: Arc<dyn ArkaPlanIsBaglantisi>,
}

impl KomutBoyutlandirAraci {
    pub fn yeni(baglanti: Arc<dyn ArkaPlanIsBaglantisi>) -> Self {
        Self { baglanti }
    }

    fn coz<'a>(&self, a: &'a Value) -> Result<(&'a str, TerminalBoyutu), AracHatasi> {
        let is_id = metin(a, "is_id").ok_or_else(|| gecersiz("`is_id` zorunludur."))?;
        let sutun = a
            .get("sutun")
            .and_then(Value::as_u64)
            .and_then(|n| u16::try_from(n).ok())
            .ok_or_else(|| gecersiz("`sutun` 1..=500 aralığında bir tam sayı olmalı."))?;
        let satir = a
            .get("satir")
            .and_then(Value::as_u64)
            .and_then(|n| u16::try_from(n).ok())
            .ok_or_else(|| gecersiz("`satir` 1..=200 aralığında bir tam sayı olmalı."))?;
        let boyut = TerminalBoyutu::yeni(sutun, satir).map_err(|e| gecersiz(e.to_string()))?;
        Ok((is_id, boyut))
    }
}

#[async_trait]
impl Arac for KomutBoyutlandirAraci {
    fn ad(&self) -> &str {
        "komut_boyutlandir"
    }

    fn aciklama(&self) -> &str {
        "Sözde terminal (PTY/ConPTY) kipinde başlatılmış bir arka plan işinin (`is_id`) \
         sütun ve satır boyutunu günceller."
    }

    fn parametre_semasi(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "is_id": {"type": "string", "description": "Uygulamanın verdiği iş kimliği (is-...)"},
                "sutun": {"type": "integer", "minimum": TerminalBoyutu::EN_AZ_SUTUN, "maximum": TerminalBoyutu::EN_COK_SUTUN},
                "satir": {"type": "integer", "minimum": TerminalBoyutu::EN_AZ_SATIR, "maximum": TerminalBoyutu::EN_COK_SATIR}
            },
            "required": ["is_id", "sutun", "satir"],
            "additionalProperties": false
        })
    }

    fn yazar_mi(&self) -> bool {
        true
    }

    async fn hazirla(&self, a: &Value) -> Result<Hazirlik, AracHatasi> {
        let (is_id, boyut) = self.coz(a)?;
        let bilgi = self.baglanti.is_bilgisi(is_id)?;
        if bilgi.terminal.is_none() {
            return Err(gecersiz(
                "Bu iş sözde terminal (PTY/ConPTY) kipinde başlatılmadı.",
            ));
        }
        Ok(Hazirlik {
            aciklama: format!(
                "`{}` terminalinin boyutunu {}x{} yapmak istiyor.",
                bilgi.is_id, boyut.sutun, boyut.satir
            ),
            veri_cikisi: Some(VeriCikisi::YalnizcaYazildi),
            kilitsiz: true,
            ..Default::default()
        })
    }

    async fn calistir(
        &self,
        a: &Value,
        _h: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        if iptal.is_cancelled() {
            return Err(AracHatasi::Iptal);
        }
        let (is_id, boyut) = self.coz(a)?;
        let durum = self.baglanti.is_boyutlandir(is_id, boyut)?;
        Ok(AracCiktisi {
            icerik: format!(
                "İş: {is_id}\nBoyut: {}x{}\nDurum: {durum}",
                boyut.sutun, boyut.satir
            ),
            kaynak: format!("kabuk:boyut:{is_id}"),
            kalfa_uretimi: true,
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kabuk::{KabukAraci, KabukGuveni};
    use kalfa_core::izin::{Karar, Kip, Siniflandirici, kabuk_karari};
    use kalfa_core::kimlik::CagriId;
    use kalfa_core::kisayol::{KisayolTeklifi, teklif};
    use kalfa_core::muhur::muhur_hesapla;
    use kalfa_sandbox::vekil::{HostKaynagi, HostKurali};
    use kalfa_sandbox::{BaslatmaHatasi, EtkilesimliSurec, Surec, YalitimYoklukNedeni};
    use std::path::Path;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct SahteGuven(AtomicBool);
    impl KabukGuveni for SahteGuven {
        fn guvenildi_mi(&self) -> bool {
            self.0.load(Ordering::SeqCst)
        }
        fn guvenildi(&self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    struct DegisebilirYalitici {
        hazir: AtomicBool,
        profil: Mutex<String>,
    }

    impl Yalitici for DegisebilirYalitici {
        fn durum(&self) -> YalitimDurumu {
            if self.hazir.load(Ordering::SeqCst) {
                YalitimDurumu::Hazir {
                    arka_uc: "bwrap".to_string(),
                    surum: "bwrap-test".to_string(),
                }
            } else {
                YalitimDurumu::Yok {
                    neden: YalitimYoklukNedeni::BwrapYok,
                    talimat: "kur".to_string(),
                    ayrinti: "yok".to_string(),
                }
            }
        }

        fn profil_ozeti(&self, ag: bool, ag_listesi_ozeti: Option<&str>) -> Option<String> {
            self.hazir.load(Ordering::SeqCst).then(|| {
                format!(
                    "{}:ag={}:liste={}",
                    self.profil.lock().unwrap_or_else(|z| z.into_inner()),
                    ag,
                    ag_listesi_ozeti.unwrap_or("-")
                )
            })
        }

        fn baslat(&self, _istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi> {
            Err(BaslatmaHatasi::Yalitim("sahte".into()))
        }

        fn baslat_kabuk_etkilesimli(
            &self,
            _istek: &CalistirmaIstegi,
            _terminal: Option<TerminalBoyutu>,
        ) -> Result<EtkilesimliSurec, BaslatmaHatasi> {
            Err(BaslatmaHatasi::Yalitim("sahte".into()))
        }
    }

    struct SahteBaglanti {
        bilgi: Mutex<ArkaPlanIsBilgisi>,
    }

    #[async_trait]
    impl ArkaPlanIsBaglantisi for SahteBaglanti {
        async fn is_baslat(
            &self,
            _yalitici: &dyn Yalitici,
            _istek: &CalistirmaIstegi,
            _meta: ArkaPlanBaslatmaMeta,
            _kilit: KilitSeti,
            _alan: Arc<CalismaAlani>,
            _onceki: AdliGirdiler,
        ) -> Result<String, AracHatasi> {
            Ok("is-11111111-2222-3333-4444-555555555555".to_string())
        }

        fn is_bilgisi(&self, is_id: &str) -> Result<ArkaPlanIsBilgisi, AracHatasi> {
            let b = self.bilgi.lock().unwrap_or_else(|z| z.into_inner()).clone();
            if b.is_id != is_id {
                return Err(AracHatasi::Gecersiz("İş bulunamadı veya yetkisiz.".into()));
            }
            Ok(b)
        }

        fn is_oku(&self, is_id: &str, imlec: usize) -> Result<ArkaPlanOkumaSonucu, AracHatasi> {
            let b = self.is_bilgisi(is_id)?;
            Ok(ArkaPlanOkumaSonucu {
                is_id: b.is_id,
                durum_etiketi: "çalışıyor".into(),
                cikti: "merhaba\n".into(),
                yeni_imlec: imlec + 8,
                toplam_bayt: (imlec + 8) as u64,
                tasti: false,
                uyarilar: Vec::new(),
            })
        }

        async fn is_girdi(
            &self,
            is_id: &str,
            _veri: &[u8],
            _eof: bool,
        ) -> Result<String, AracHatasi> {
            let _ = self.is_bilgisi(is_id)?;
            Ok("çalışıyor".into())
        }

        fn is_boyutlandir(
            &self,
            is_id: &str,
            _boyut: TerminalBoyutu,
        ) -> Result<String, AracHatasi> {
            let _ = self.is_bilgisi(is_id)?;
            Ok("çalışıyor".into())
        }

        async fn is_durdur(&self, is_id: &str) -> Result<ArkaPlanDurdurmaSonucu, AracHatasi> {
            let b = self.is_bilgisi(is_id)?;
            Ok(ArkaPlanDurdurmaSonucu {
                is_id: b.is_id,
                durum_etiketi: "durduruldu".into(),
                cikis_kodu: None,
                sure_ms: 12,
                zaman_asimi: false,
                uyarilar: Vec::new(),
            })
        }
    }

    struct OrnekOrtam {
        alan: Arc<CalismaAlani>,
        yalitici: Arc<DegisebilirYalitici>,
        izin: Arc<AtomicBool>,
        kullanici_ag: Arc<std::sync::RwLock<Vec<HostKurali>>>,
        baglanti: Arc<SahteBaglanti>,
    }

    fn ornek_ortam(gecici: &Path) -> OrnekOrtam {
        let siniflandirici =
            Arc::new(Siniflandirici::yeni(gecici.to_path_buf(), &[]).expect("sınıflandırıcı"));
        let alan = CalismaAlani::ac(siniflandirici).expect("alan");
        let yalitici = Arc::new(DegisebilirYalitici {
            hazir: AtomicBool::new(true),
            profil: Mutex::new("p1".to_string()),
        });
        let izin = Arc::new(AtomicBool::new(false));
        let kullanici_ag = Arc::default();
        let baglanti = Arc::new(SahteBaglanti {
            bilgi: Mutex::new(ArkaPlanIsBilgisi {
                is_id: "is-11111111-2222-3333-4444-555555555555".to_string(),
                gosterim: "cargo test".to_string(),
                dizin: ".".to_string(),
                ag: false,
                ag_hostlari: Vec::new(),
                yalitimli: true,
                terminal: Some(TerminalBoyutu::default()),
                calisiyor: true,
            }),
        });
        OrnekOrtam {
            alan,
            yalitici,
            izin,
            kullanici_ag,
            baglanti,
        }
    }

    #[tokio::test]
    async fn baslatma_ve_komut_girdi_ayri_muhur_uretir_ve_kisayol_devralmaz() {
        let gecici = tempfile::tempdir().expect("tempdir");
        let OrnekOrtam {
            alan,
            yalitici,
            izin,
            kullanici_ag,
            baglanti,
        } = ornek_ortam(gecici.path());
        let guven = Arc::new(SahteGuven(AtomicBool::new(true)));

        let kabuk = KabukAraci::yeni(alan, yalitici.clone(), guven, izin.clone())
            .kullanici_ag_listesi(kullanici_ag.clone())
            .arka_plan_baglantisi(baglanti.clone());
        let girdi_araci =
            KomutGirdiAraci::yeni(baglanti.clone(), yalitici.clone(), izin, kullanici_ag);

        let baslat_arg = json!({
            "program": "cargo",
            "argumanlar": ["test"],
            "arka_plan": true,
            "terminal": {"sutun": 80, "satir": 24}
        });
        let baslat_hazirlik = kabuk.hazirla(&baslat_arg).await.expect("başlat hazırlık");
        let baslat_muhru =
            muhur_hesapla(CagriId::yeni(), kabuk.ad(), &baslat_arg, &baslat_hazirlik);

        let girdi_arg_1 = json!({
            "is_id": "is-11111111-2222-3333-4444-555555555555",
            "girdi": "evet\n"
        });
        let girdi_hazirlik_1 = girdi_araci
            .hazirla(&girdi_arg_1)
            .await
            .expect("girdi hazırlık 1");
        let cagri_id = CagriId::yeni();
        let girdi_muhru_1 =
            muhur_hesapla(cagri_id, girdi_araci.ad(), &girdi_arg_1, &girdi_hazirlik_1);

        let girdi_arg_2 = json!({
            "is_id": "is-11111111-2222-3333-4444-555555555555",
            "girdi": "hayir\n"
        });
        let girdi_hazirlik_2 = girdi_araci
            .hazirla(&girdi_arg_2)
            .await
            .expect("girdi hazırlık 2");
        let girdi_muhru_2 =
            muhur_hesapla(cagri_id, girdi_araci.ad(), &girdi_arg_2, &girdi_hazirlik_2);

        assert_ne!(baslat_muhru, girdi_muhru_1);
        assert_ne!(girdi_muhru_1, girdi_muhru_2);

        // Tek seferlik program kısayolu ne arka plan başlatmayı ne de `komut_girdi`'yi kapsar.
        let mevcut_kisayol = KisayolTeklifi::Kabuk {
            program: "cargo".to_string(),
            dizin: ".".to_string(),
        };
        assert!(!mevcut_kisayol.kapsar(kabuk.ad(), &baslat_hazirlik));
        assert!(!mevcut_kisayol.kapsar(girdi_araci.ad(), &girdi_hazirlik_1));

        // Her kipte (`Siki`, `Proje`, `Serbest`) arka plan başlatma ve `komut_girdi` onay ister
        // ve kısayol teklif etmez.
        for kip in [Kip::Siki, Kip::Proje, Kip::Serbest] {
            let karar_baslat =
                kabuk_karari(kip, baslat_hazirlik.kabuk.as_ref().expect("kabuk tarifi"));
            assert!(matches!(karar_baslat, Karar::Onayli { .. }));
            assert!(teklif(kabuk.ad(), &baslat_hazirlik, &karar_baslat).is_none());

            let karar_girdi =
                kabuk_karari(kip, girdi_hazirlik_1.kabuk.as_ref().expect("girdi tarifi"));
            assert!(matches!(karar_girdi, Karar::Onayli { .. }));
            assert!(teklif(girdi_araci.ad(), &girdi_hazirlik_1, &karar_girdi).is_none());
        }
    }

    #[tokio::test]
    async fn profil_ve_ag_degisimi_muhru_gecersizlestirir_yalitim_degisimi_reddeder() {
        let gecici = tempfile::tempdir().expect("tempdir");
        let OrnekOrtam {
            alan: _alan,
            yalitici,
            izin,
            kullanici_ag,
            baglanti,
        } = ornek_ortam(gecici.path());
        baglanti.bilgi.lock().unwrap_or_else(|z| z.into_inner()).ag = true;

        let girdi_araci = KomutGirdiAraci::yeni(
            baglanti.clone(),
            yalitici.clone(),
            izin.clone(),
            kullanici_ag.clone(),
        );
        let arg = json!({
            "is_id": "is-11111111-2222-3333-4444-555555555555",
            "girdi": " devam\n"
        });
        let cagri_id = CagriId::yeni();
        let h1 = girdi_araci.hazirla(&arg).await.expect("h1");
        let m1 = muhur_hesapla(cagri_id, girdi_araci.ad(), &arg, &h1);

        // Kullanıcı ağ izin listesi değiştiğinde mühür değişir.
        kullanici_ag
            .write()
            .unwrap_or_else(|z| z.into_inner())
            .push(HostKurali {
                host: "ornek.com".to_string(),
                port: 443,
                ozel_ag: false,
                kaynak: HostKaynagi::Kullanici,
            });
        let h2 = girdi_araci.hazirla(&arg).await.expect("h2");
        let m2 = muhur_hesapla(cagri_id, girdi_araci.ad(), &arg, &h2);
        assert_ne!(m1, m2);

        // Yalıtım profil özeti değiştiğinde mühür değişir.
        *yalitici.profil.lock().unwrap_or_else(|z| z.into_inner()) = "p2".to_string();
        let h3 = girdi_araci.hazirla(&arg).await.expect("h3");
        let m3 = muhur_hesapla(cagri_id, girdi_araci.ad(), &arg, &h3);
        assert_ne!(m2, m3);

        // İş yalıtımlı başlatılmışken yalıtım düşerse `komut_girdi` reddeder.
        yalitici.hazir.store(false, Ordering::SeqCst);
        izin.store(true, Ordering::SeqCst);
        let hata = girdi_araci
            .hazirla(&arg)
            .await
            .expect_err("yalıtım düşüşü reddedilmeli");
        assert!(hata.to_string().contains("Yalıtım durumu"));
    }

    #[tokio::test]
    async fn kontrol_araclari_kilitsizdir_ve_komut_oku_guvenilmez_cikti_isaretler() {
        let gecici = tempfile::tempdir().expect("tempdir");
        let OrnekOrtam {
            alan: _alan,
            yalitici,
            izin,
            kullanici_ag,
            baglanti,
        } = ornek_ortam(gecici.path());
        let araclar = arka_plan_araclari(baglanti, yalitici, izin, kullanici_ag);
        assert_eq!(araclar.len(), 4);

        let oku = &araclar[0];
        let h_oku = oku
            .hazirla(&json!({"is_id": "is-11111111-2222-3333-4444-555555555555", "imlec": 0}))
            .await
            .expect("oku hazırlık");
        assert!(h_oku.kilitsiz);
        assert_eq!(h_oku.veri_cikisi, Some(VeriCikisi::KomutCiktisi));
        let c_oku = oku
            .calistir(
                &json!({"is_id": "is-11111111-2222-3333-4444-555555555555", "imlec": 0}),
                &h_oku,
                CancellationToken::new(),
            )
            .await
            .expect("oku çalıştır");
        assert!(!c_oku.kalfa_uretimi);
        assert!(c_oku.icerik.contains("merhaba"));

        let durdur = &araclar[2];
        let h_durdur = durdur
            .hazirla(&json!({"is_id": "is-11111111-2222-3333-4444-555555555555"}))
            .await
            .expect("durdur hazırlık");
        assert!(h_durdur.kilitsiz);

        let boyut = &araclar[3];
        let h_boyut = boyut
            .hazirla(&json!({
                "is_id": "is-11111111-2222-3333-4444-555555555555",
                "sutun": 120,
                "satir": 40
            }))
            .await
            .expect("boyut hazırlık");
        assert!(h_boyut.kilitsiz);
    }
}
