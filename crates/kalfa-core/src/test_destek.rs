//! Test desteği: sahte sağlayıcı ve sahte araç (oturum sürücüsü Task 8'de).
//! Yalnızca `test-destek` özelliğiyle derlenir; burada `expect`/`panic` serbesttir.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures_util::{StreamExt, stream};
use kalfa_evren::tipler::{ChatRequest, FunctionCall, TermsText, ToolCall};
use kalfa_evren::{AkisOlayi, EvrenError, YanitMeta};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::arac::{Arac, AracCiktisi, AracHatasi, Hazirlik, YazmaHedefi};
use crate::saglayici::{Saglayici, SaglayiciAkisi};
use crate::yol::CalismaYolu;

static MODEL_CAGRI_SAYACI: AtomicU64 = AtomicU64::new(0);

/// Sahte sağlayıcının bir `akis()` çağrısına vereceği yanıt.
pub enum SahteYanit {
    /// Olayları sırayla verir, sonra biter.
    Akis(Vec<Result<AkisOlayi, EvrenError>>),
    /// Olayları verir, sonra hiç bitmez (iptal testleri için).
    Askida(Vec<AkisOlayi>),
    /// `akis()` doğrudan hata döner (HTTP hatası gibi).
    Hata(EvrenError),
}

impl SahteYanit {
    pub fn metin(metin: &str) -> Self {
        Self::Akis(vec![
            Ok(AkisOlayi::Metin(metin.into())),
            Ok(AkisOlayi::Bitti {
                neden: "stop".into(),
            }),
        ])
    }

    pub fn arac(ad: &str, argumanlar: Value) -> Self {
        Self::araclar(vec![(ad, argumanlar)])
    }

    pub fn araclar(cagrilar: Vec<(&str, Value)>) -> Self {
        let cagrilar = cagrilar
            .into_iter()
            .map(|(ad, argumanlar)| ToolCall {
                id: format!(
                    "model-cagri-{}",
                    MODEL_CAGRI_SAYACI.fetch_add(1, Ordering::SeqCst)
                ),
                kind: "function".into(),
                function: FunctionCall {
                    name: ad.into(),
                    arguments: argumanlar.to_string(),
                },
            })
            .collect();
        Self::Akis(vec![
            Ok(AkisOlayi::AracCagrilari(cagrilar)),
            Ok(AkisOlayi::Bitti {
                neden: "tool_calls".into(),
            }),
        ])
    }
}

struct SahteSart {
    surum: u32,
    metin: String,
    kabul_edilen: Vec<u32>,
}

pub struct SahteSaglayici {
    senaryo: Mutex<VecDeque<SahteYanit>>,
    istekler: Mutex<Vec<ChatRequest>>,
    sart: Mutex<SahteSart>,
}

impl SahteSaglayici {
    pub fn yeni(senaryo: Vec<SahteYanit>) -> Arc<Self> {
        Arc::new(Self {
            senaryo: Mutex::new(senaryo.into()),
            istekler: Mutex::new(Vec::new()),
            sart: Mutex::new(SahteSart {
                surum: 1,
                metin: "# Sahte kullanım şartları".into(),
                kabul_edilen: Vec::new(),
            }),
        })
    }

    pub fn istekler(&self) -> Vec<ChatRequest> {
        self.istekler.lock().unwrap().clone()
    }

    pub fn kabul_edilen_surumler(&self) -> Vec<u32> {
        self.sart.lock().unwrap().kabul_edilen.clone()
    }

    pub fn sart_surumunu_degistir(&self, surum: u32) {
        self.sart.lock().unwrap().surum = surum;
    }
}

#[async_trait]
impl Saglayici for SahteSaglayici {
    async fn akis(&self, istek: ChatRequest) -> Result<SaglayiciAkisi, EvrenError> {
        let sira = {
            let mut istekler = self.istekler.lock().unwrap();
            istekler.push(istek);
            istekler.len()
        };
        let yanit = self
            .senaryo
            .lock()
            .unwrap()
            .pop_front()
            .expect("sahte senaryoda yanıt kalmadı");
        let meta = YanitMeta {
            istek_id: Some(format!("sahte-{sira}")),
            ..Default::default()
        };
        match yanit {
            SahteYanit::Akis(olaylar) => Ok(SaglayiciAkisi {
                meta,
                olaylar: Box::pin(stream::iter(olaylar)),
            }),
            SahteYanit::Askida(olaylar) => Ok(SaglayiciAkisi {
                meta,
                olaylar: Box::pin(
                    stream::iter(olaylar.into_iter().map(Ok)).chain(stream::pending()),
                ),
            }),
            SahteYanit::Hata(hata) => Err(hata),
        }
    }

    async fn sart_metni(&self) -> Result<TermsText, EvrenError> {
        let s = self.sart.lock().unwrap();
        Ok(TermsText {
            version: s.surum,
            content: s.metin.clone(),
            doc_path: None,
        })
    }

    async fn sart_kabul(&self, surum: u32) -> Result<(), EvrenError> {
        let mut s = self.sart.lock().unwrap();
        if surum != s.surum {
            return Err(EvrenError::SartKabulGerekli {
                surum: Some(s.surum),
            });
        }
        s.kabul_edilen.push(surum);
        Ok(())
    }
}

/// `{"yol": ...}` alan sahte araç. Okuyan ya da yazan olabilir.
pub struct SahteArac {
    ad: String,
    yazar: bool,
    yanit: String,
    hata: Option<String>,
    gecikme: Duration,
    on_hal: AtomicU64,
    cagrilar: Mutex<Vec<Value>>,
    aktif: AtomicU32,
    azami_aktif: AtomicU32,
}

impl SahteArac {
    fn yeni(ad: &str, yazar: bool) -> Self {
        Self {
            ad: ad.into(),
            yazar,
            yanit: format!("{ad} tamam"),
            hata: None,
            gecikme: Duration::ZERO,
            on_hal: AtomicU64::new(0),
            cagrilar: Mutex::new(Vec::new()),
            aktif: AtomicU32::new(0),
            azami_aktif: AtomicU32::new(0),
        }
    }

    pub fn okuyan(ad: &str) -> Self {
        Self::yeni(ad, false)
    }

    pub fn yazan(ad: &str) -> Self {
        Self::yeni(ad, true)
    }

    pub fn yanit(mut self, yanit: String) -> Self {
        self.yanit = yanit;
        self
    }

    pub fn hata(mut self, hata: String) -> Self {
        self.hata = Some(hata);
        self
    }

    pub fn gecikme(mut self, gecikme: Duration) -> Self {
        self.gecikme = gecikme;
        self
    }

    /// Yazma hedefinin "dosyasını" dışarıdan değiştirir: önceki özet değişir.
    pub fn dosyayi_degistir(&self) {
        self.on_hal.fetch_add(1, Ordering::SeqCst);
    }

    pub fn cagri_sayisi(&self) -> usize {
        self.cagrilar.lock().unwrap().len()
    }

    pub fn azami_eszamanli(&self) -> u32 {
        self.azami_aktif.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Arac for SahteArac {
    fn ad(&self) -> &str {
        &self.ad
    }

    fn aciklama(&self) -> &str {
        "sahte araç"
    }

    fn parametre_semasi(&self) -> Value {
        json!({
            "type": "object",
            "properties": {"yol": {"type": "string"}},
            "required": ["yol"],
            "additionalProperties": false
        })
    }

    fn yazar_mi(&self) -> bool {
        self.yazar
    }

    async fn hazirla(&self, argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
        let yol = CalismaYolu::coz(argumanlar["yol"].as_str().unwrap_or_default())?;
        let mut h = Hazirlik {
            aciklama: format!("sahte {} `{yol}`", self.ad),
            ..Default::default()
        };
        if self.yazar {
            h.yazilan.push(YazmaHedefi {
                yol,
                onceki_ozet: Some(format!("surum-{}", self.on_hal.load(Ordering::SeqCst))),
                baglanti_sayisi: 1,
            });
            h.diff = Some(format!("+sahte {}", self.ad));
        } else {
            h.okunan.push(yol);
        }
        Ok(h)
    }

    async fn calistir(
        &self,
        argumanlar: &Value,
        _hazirlik: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.cagrilar.lock().unwrap().push(argumanlar.clone());
        let simdi = self.aktif.fetch_add(1, Ordering::SeqCst) + 1;
        self.azami_aktif.fetch_max(simdi, Ordering::SeqCst);
        let sonuc = tokio::select! {
            _ = tokio::time::sleep(self.gecikme) => match &self.hata {
                Some(h) => Err(AracHatasi::Gecersiz(h.clone())),
                None => Ok(AracCiktisi { icerik: self.yanit.clone(), kaynak: format!("sahte:{}", self.ad) }),
            },
            _ = iptal.cancelled() => Err(AracHatasi::Iptal),
        };
        self.aktif.fetch_sub(1, Ordering::SeqCst);
        sonuc
    }
}
