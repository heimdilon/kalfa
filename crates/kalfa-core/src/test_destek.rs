//! Test desteği: sahte sağlayıcı, sahte araç ve oturum sürücüsü.
//! Yalnızca `test-destek` özelliğiyle derlenir; burada `expect`/`panic` serbesttir.

use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures_util::{StreamExt, stream};
use kalfa_evren::karar::{KararIstegi, KararYaniti};
use kalfa_evren::tipler::{
    ChatMessage, ChatRequest, ChatResponse, Choice, FunctionCall, ModelInfo, TermsStatus,
    TermsText, ToolCall, Usage,
};
use kalfa_evren::{AkisOlayi, EkUc, EvrenError, HamYanit, YanitMeta};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use crate::arac::{Arac, AracCiktisi, AracHatasi, AracKayitDefteri, Hazirlik, YazmaHedefi};
use crate::butce::{
    ButceGorunumu, ButceIslemId, ButceMesaji, ButceSinirlari, ButceTalebi, ButceTalepci,
};
use crate::geri_alma::{DosyaDurumu, GeriYukleyici};
use crate::izin::Siniflandirici;
use crate::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
use crate::kayit::{KayitHedefi, OturumKaydi};
use crate::kilit::KaynakKilitleri;
use crate::muhur::ozet_hex;
use crate::oturum::{OlayAlici, OturumAyarlari, OturumBaglami, OturumTutamaci, baslat};
use crate::protokol::{BitisNedeni, ButceKarari, Komut, Olay, Zarf};
use crate::saglayici::{Saglayici, SaglayiciAkisi};
use crate::sart_kapisi::SartKapisi;
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
    /// Yanıt bu kadar bekledikten sonra gelir: tur gerçek bir model gibi
    /// sürer (ör. tura bağlı arka plan işleri tamamlanabilsin).
    Gecikmeli(std::time::Duration, Box<SahteYanit>),
}

/// Başarılı bir yanıtın geçerli kullanımı (5b tasarımı §6): istem 10, tamamlama 5.
fn basarili_kullanim() -> Result<AkisOlayi, EvrenError> {
    Ok(AkisOlayi::Kullanim(Usage {
        prompt_tokens: 10,
        completion_tokens: 5,
        total_tokens: 15,
        completion_tokens_details: None,
        evren: None,
    }))
}

impl SahteYanit {
    pub fn metin(metin: &str) -> Self {
        Self::Akis(vec![
            Ok(AkisOlayi::Metin(metin.into())),
            basarili_kullanim(),
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
            basarili_kullanim(),
            Ok(AkisOlayi::Bitti {
                neden: "tool_calls".into(),
            }),
        ])
    }

    pub fn baglanti_hatasi(mesaj: &str) -> Self {
        Self::Hata(EvrenError::ag_baglanti(mesaj))
    }

    pub fn akis_kopmasi(parcalar: &[&str], hata: EvrenError) -> Self {
        let mut olaylar: Vec<Result<AkisOlayi, EvrenError>> = parcalar
            .iter()
            .map(|p| Ok(AkisOlayi::Metin((*p).to_string())))
            .collect();
        olaylar.push(Err(hata));
        Self::Akis(olaylar)
    }
}

/// Sahte sağlayıcının bir `sohbet()` çağrısına vereceği bloklu yanıt. `Json`,
/// `Metin` ve `Gecikme` geçerli bir kullanım taşır; `KullanimsizMetin` kullanım
/// bildirmez ve 5b bütçesinde bilinmeyen tüketimdir.
#[derive(Debug, Clone)]
pub enum SahteSohbet {
    Json(Value),
    Metin(String),
    KullanimsizMetin(String),
    Hata(EvrenError),
    Gecikme(Duration),
}

/// Başarılı bir `sohbet` yanıtının geçerli kullanımı (5b tasarımı §6): istem 10,
/// tamamlama 5.
fn sohbet_kullanimi() -> Option<Usage> {
    Some(Usage {
        prompt_tokens: 10,
        completion_tokens: 5,
        total_tokens: 15,
        completion_tokens_details: None,
        evren: None,
    })
}

fn sohbet_yaniti(
    sira: usize,
    model: String,
    metin: String,
    usage: Option<Usage>,
    meta: YanitMeta,
) -> (ChatResponse, YanitMeta) {
    let message = ChatMessage::assistant(Some(metin), Vec::new());
    (
        ChatResponse {
            id: format!("sahte-sohbet-{sira}"),
            model,
            choices: vec![Choice {
                index: 0,
                message,
                finish_reason: Some("stop".to_string()),
            }],
            usage,
        },
        meta,
    )
}

struct SahteSart {
    surum: u32,
    metin: String,
    kabul_edilen: Vec<u32>,
}

pub struct SahteSaglayici {
    senaryo: Mutex<VecDeque<SahteYanit>>,
    istekler: Mutex<Vec<ChatRequest>>,
    sohbet_senaryo: Mutex<VecDeque<SahteSohbet>>,
    sohbet_istekleri: Mutex<Vec<ChatRequest>>,
    sart: Mutex<SahteSart>,
    durum_hatasi: Mutex<Option<EvrenError>>,
    ek_istekleri: Mutex<Vec<(EkUc, Value)>>,
    ek_senaryo: Mutex<VecDeque<Result<HamYanit, EvrenError>>>,
    ek_gecikme: Mutex<Option<Duration>>,
    ek_hatalari: Mutex<BTreeMap<usize, EvrenError>>,
    modeller: Mutex<Result<Vec<ModelInfo>, EvrenError>>,
    sohbet_kancasi: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
    karar_senaryo: Mutex<VecDeque<Result<KararYaniti, EvrenError>>>,
    karar_istekleri: Mutex<Vec<KararIstegi>>,
    karar_gecikme: Mutex<Option<Duration>>,
}

impl SahteSaglayici {
    pub fn yeni(senaryo: Vec<SahteYanit>) -> Arc<Self> {
        Arc::new(Self {
            senaryo: Mutex::new(senaryo.into()),
            istekler: Mutex::new(Vec::new()),
            sohbet_senaryo: Mutex::new(VecDeque::new()),
            sohbet_istekleri: Mutex::new(Vec::new()),
            sart: Mutex::new(SahteSart {
                surum: 1,
                metin: "# Sahte kullanım şartları".into(),
                kabul_edilen: Vec::new(),
            }),
            durum_hatasi: Mutex::new(None),
            ek_istekleri: Mutex::new(Vec::new()),
            ek_senaryo: Mutex::new(VecDeque::new()),
            ek_gecikme: Mutex::new(None),
            ek_hatalari: Mutex::new(BTreeMap::new()),
            modeller: Mutex::new(Ok(sahte_modeller())),
            sohbet_kancasi: Mutex::new(None),
            karar_senaryo: Mutex::new(VecDeque::new()),
            karar_istekleri: Mutex::new(Vec::new()),
            karar_gecikme: Mutex::new(None),
        })
    }

    /// Her `sohbet()` çağrısı yanıttan önce bunu çalıştırır (ör. turu iptal etmek).
    pub fn sohbet_kancasi(&self, kanca: impl Fn() + Send + Sync + 'static) {
        *self.sohbet_kancasi.lock().unwrap() = Some(Arc::new(kanca));
    }

    /// Eklenti uçlarına gelen istekler, geliş sırasıyla (sahte EVREN'in
    /// aldıkları).
    pub fn ek_istekleri(&self) -> Vec<(EkUc, Value)> {
        self.ek_istekleri.lock().unwrap().clone()
    }

    /// Sıradaki eklenti isteğine bu yanıt verilir; kuyruk boşken yanıt
    /// belirlenimlidir (`sahte_ek_yanit`).
    pub fn ek_yanit_ekle(&self, yanit: Result<HamYanit, EvrenError>) {
        self.ek_senaryo.lock().unwrap().push_back(yanit);
    }

    /// Eklenti isteklerinden `sira`'ncısı (1'den) bu hatayı alır.
    pub fn ek_hata_sirada(&self, sira: usize, hata: EvrenError) {
        self.ek_hatalari.lock().unwrap().insert(sira, hata);
    }

    /// Her eklenti isteği yanıttan önce bu kadar bekler.
    pub fn ek_gecikme(&self, sure: Option<Duration>) {
        *self.ek_gecikme.lock().unwrap() = sure;
    }

    pub fn karar_ekle(&self, yanit: Result<KararYaniti, EvrenError>) {
        self.karar_senaryo.lock().unwrap().push_back(yanit);
    }

    pub fn karar_istekleri(&self) -> Vec<KararIstegi> {
        self.karar_istekleri.lock().unwrap().clone()
    }

    pub fn karar_gecikme(&self, sure: Option<Duration>) {
        *self.karar_gecikme.lock().unwrap() = sure;
    }

    pub fn modeller_ayarla(&self, modeller: Result<Vec<ModelInfo>, EvrenError>) {
        *self.modeller.lock().unwrap() = modeller;
    }

    pub fn istekler(&self) -> Vec<ChatRequest> {
        self.istekler.lock().unwrap().clone()
    }

    /// Senaryonun sonuna yanıt ekler; tur sayısı önceden bilinmeyen
    /// testler her turdan önce kendi yanıtını koyar.
    pub fn senaryoya_ekle(&self, yanit: SahteYanit) {
        self.senaryo.lock().unwrap().push_back(yanit);
    }

    pub fn sohbet_ekle(&self, yanit: SahteSohbet) {
        self.sohbet_senaryo.lock().unwrap().push_back(yanit);
    }

    pub fn sohbet_istekleri(&self) -> Vec<ChatRequest> {
        self.sohbet_istekleri.lock().unwrap().clone()
    }

    pub fn kabul_edilen_surumler(&self) -> Vec<u32> {
        self.sart.lock().unwrap().kabul_edilen.clone()
    }

    pub fn sart_durumu_hatasi(&self, hata: Option<EvrenError>) {
        *self.durum_hatasi.lock().unwrap() = hata;
    }

    pub fn sart_surumunu_degistir(&self, surum: u32) {
        self.sart.lock().unwrap().surum = surum;
    }
}

#[async_trait]
impl Saglayici for SahteSaglayici {
    async fn akis(&self, istek: ChatRequest) -> Result<SaglayiciAkisi, EvrenError> {
        let model = istek.model.clone();
        let sira = {
            let mut istekler = self.istekler.lock().unwrap();
            istekler.push(istek);
            istekler.len()
        };
        let mut yanit = self
            .senaryo
            .lock()
            .unwrap()
            .pop_front()
            .expect("sahte senaryoda yanıt kalmadı");
        while let SahteYanit::Gecikmeli(sure, ic) = yanit {
            tokio::time::sleep(sure).await;
            yanit = *ic;
        }
        let meta = YanitMeta {
            istek_id: Some(format!("sahte-{sira}")),
            yonlendirilen_model: Some(model),
            ..Default::default()
        };
        match yanit {
            SahteYanit::Gecikmeli(..) => unreachable!("gecikme yukarıda açıldı"),
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

    async fn sohbet(&self, istek: ChatRequest) -> Result<(ChatResponse, YanitMeta), EvrenError> {
        let model = istek.model.clone();
        let sira = {
            let mut istekler = self.sohbet_istekleri.lock().unwrap();
            istekler.push(istek);
            istekler.len()
        };
        let yanit = self
            .sohbet_senaryo
            .lock()
            .unwrap()
            .pop_front()
            .expect("sahte sohbet senaryosunda yanıt kalmadı");
        let kanca = self.sohbet_kancasi.lock().unwrap().clone();
        if let Some(kanca) = kanca {
            kanca();
        }
        let meta = YanitMeta {
            istek_id: Some(format!("sahte-sohbet-{sira}")),
            yonlendirilen_model: Some(model.clone()),
            ..Default::default()
        };
        match yanit {
            SahteSohbet::Json(deger) => Ok(sohbet_yaniti(
                sira,
                model,
                deger.to_string(),
                sohbet_kullanimi(),
                meta,
            )),
            SahteSohbet::Metin(metin) => {
                Ok(sohbet_yaniti(sira, model, metin, sohbet_kullanimi(), meta))
            }
            SahteSohbet::KullanimsizMetin(metin) => {
                Ok(sohbet_yaniti(sira, model, metin, None, meta))
            }
            SahteSohbet::Hata(hata) => Err(hata),
            SahteSohbet::Gecikme(sure) => {
                tokio::time::sleep(sure).await;
                Ok(sohbet_yaniti(
                    sira,
                    model,
                    String::new(),
                    sohbet_kullanimi(),
                    meta,
                ))
            }
        }
    }

    async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError> {
        if let Some(hata) = self.durum_hatasi.lock().unwrap().clone() {
            return Err(hata);
        }
        let s = self.sart.lock().unwrap();
        Ok(TermsStatus {
            current_version: s.surum,
            is_material: true,
            accepted: s.kabul_edilen.contains(&s.surum),
            doc_path: None,
        })
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

    async fn modeller(&self) -> Result<Vec<ModelInfo>, EvrenError> {
        self.modeller.lock().unwrap().clone()
    }

    async fn ek_uc(&self, uc: EkUc, govde: &Value) -> Result<HamYanit, EvrenError> {
        let sira = {
            let mut istekler = self.ek_istekleri.lock().unwrap();
            istekler.push((uc, govde.clone()));
            istekler.len()
        };
        let gecikme = *self.ek_gecikme.lock().unwrap();
        if let Some(sure) = gecikme {
            tokio::time::sleep(sure).await;
        }
        if let Some(hata) = self.ek_hatalari.lock().unwrap().remove(&sira) {
            return Err(hata);
        }
        if let Some(yanit) = self.ek_senaryo.lock().unwrap().pop_front() {
            return yanit;
        }
        let model = govde["model"].as_str().unwrap_or_default().to_string();
        Ok(HamYanit {
            govde: sahte_ek_yanit(uc, govde).to_string(),
            model: Some(format!("qwen/{model}")),
            meta: YanitMeta {
                istek_id: Some(format!("sahte-ek-{sira}")),
                yonlendirilen_model: Some(format!("qwen/{model}")),
                ..Default::default()
            },
        })
    }

    async fn karar(&self, istek: KararIstegi) -> Result<(KararYaniti, YanitMeta), EvrenError> {
        let model = istek.model.clone();
        let sira = {
            let mut istekler = self.karar_istekleri.lock().unwrap();
            istekler.push(istek);
            istekler.len()
        };
        let gecikme = *self.karar_gecikme.lock().unwrap();
        if let Some(sure) = gecikme {
            tokio::time::sleep(sure).await;
        }
        let yanit = self
            .karar_senaryo
            .lock()
            .unwrap()
            .pop_front()
            .expect("sahte karar senaryosunda yanıt kalmadı")?;
        let meta = YanitMeta {
            istek_id: Some(format!("sahte-karar-{sira}")),
            yonlendirilen_model: Some(model),
            ..Default::default()
        };
        Ok((yanit, meta))
    }
}

/// Sahte EVREN'in model kataloğu: sohbet, gömme ve sıralama modelleri.
pub fn sahte_modeller() -> Vec<ModelInfo> {
    [
        ("glm-5.3", "chat"),
        ("qwen3.8-flash-next", "chat"),
        ("deepseek-v4-flash", "chat"),
        ("auto", "chat"),
        ("qwen3-embedding-8b", "embedding"),
        ("qwen3-reranker-8b", "rerank"),
    ]
    .into_iter()
    .map(|(id, task)| ModelInfo {
        id: id.into(),
        task: task.into(),
        modalities: Vec::new(),
        pricing: None,
        eforlar: Vec::new(),
    })
    .collect()
}

/// Metnin SHA-256'sından türetilen, birim uzunlukta belirlenimli vektör.
/// Aynı metin her zaman aynı vektörü verir.
pub fn sahte_vektor(metin: &str, boyut: usize) -> Vec<f32> {
    let mut v = Vec::with_capacity(boyut);
    let mut blok = 0u32;
    while v.len() < boyut {
        let mut h = Sha256::new();
        h.update(metin.as_bytes());
        h.update(blok.to_le_bytes());
        for b in h.finalize() {
            if v.len() == boyut {
                break;
            }
            v.push((f32::from(b) - 127.5) / 127.5);
        }
        blok += 1;
    }
    let uzunluk = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if uzunluk > 0.0 {
        for x in &mut v {
            *x /= uzunluk;
        }
    }
    v
}

/// Sahte EVREN'in belirlenimli yanıtı. Gömme: her girdiye `sahte_vektor`
/// (`dimensions`, yoksa 1024; `encoding_format: "base64"` ise küçük endian
/// `f32`'lerin base64'ü). Sıralama: puan, sorgunun (en az iki harfli)
/// kelimelerinden belgede geçenlerin oranıdır; eşitlikte sıra korunur.
pub fn sahte_ek_yanit(uc: EkUc, govde: &Value) -> Value {
    let model = govde["model"].as_str().unwrap_or_default();
    match uc {
        EkUc::Gomme => {
            let boyut = govde["dimensions"].as_u64().unwrap_or(1024) as usize;
            let base64 = govde["encoding_format"].as_str() == Some("base64");
            let girdiler: Vec<String> = match &govde["input"] {
                Value::String(s) => vec![s.clone()],
                Value::Array(a) => a
                    .iter()
                    .map(|x| x.as_str().unwrap_or_default().to_string())
                    .collect(),
                _ => Vec::new(),
            };
            let veri: Vec<Value> = girdiler
                .iter()
                .enumerate()
                .map(|(i, metin)| {
                    let v = sahte_vektor(metin, boyut);
                    let gomme = if base64 {
                        let baytlar: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
                        Value::String(base64_kodla(&baytlar))
                    } else {
                        json!(v)
                    };
                    json!({"index": i, "object": "embedding", "embedding": gomme})
                })
                .collect();
            let token: usize = girdiler.iter().map(|m| m.len() / 4 + 1).sum();
            json!({
                "object": "list",
                "model": format!("qwen/{model}"),
                "data": veri,
                "usage": {"prompt_tokens": token, "total_tokens": token},
            })
        }
        EkUc::Siralama => {
            let sorgu = govde["query"].as_str().unwrap_or_default().to_lowercase();
            let kelimeler: Vec<&str> = sorgu
                .split_whitespace()
                .filter(|k| k.chars().count() >= 2)
                .collect();
            let belgeler: Vec<String> = govde["documents"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|x| x.as_str().unwrap_or_default().to_lowercase())
                        .collect()
                })
                .unwrap_or_default();
            let mut puanlar: Vec<(usize, f64)> = belgeler
                .iter()
                .enumerate()
                .map(|(i, b)| {
                    let gecen = kelimeler.iter().filter(|k| b.contains(*k)).count();
                    (i, gecen as f64 / (kelimeler.len() as f64 + 1.0))
                })
                .collect();
            puanlar.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            let ilk_n = govde["top_n"]
                .as_u64()
                .map_or(puanlar.len(), |n| n as usize);
            puanlar.truncate(ilk_n);
            let sonuclar: Vec<Value> = puanlar
                .into_iter()
                .map(|(i, p)| json!({"index": i, "relevance_score": p}))
                .collect();
            // Gerçek sıralama yanıtı da `usage` taşır (5b tasarımı §2): istem tokenı
            // sorgu ve belgelerin kabaca uzunluğudur.
            let token: usize = std::iter::once(sorgu.len())
                .chain(belgeler.iter().map(String::len))
                .map(|uzunluk| uzunluk / 4 + 1)
                .sum();
            json!({
                "id": "sahte-siralama",
                "model": format!("qwen/{model}"),
                "results": sonuclar,
                "usage": {"prompt_tokens": token, "total_tokens": token},
            })
        }
    }
}

fn base64_kodla(veri: &[u8]) -> String {
    const ABECE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut cikti = String::with_capacity(veri.len().div_ceil(3) * 4);
    for parca in veri.chunks(3) {
        let b = [
            parca[0],
            parca.get(1).copied().unwrap_or(0),
            parca.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, kaydir) in [18u32, 12, 6, 0].into_iter().enumerate() {
            if i <= parca.len() {
                cikti.push(ABECE[((n >> kaydir) & 63) as usize] as char);
            } else {
                cikti.push('=');
            }
        }
    }
    cikti
}

/// Sahte yazan aracın hazırlığındaki önceki hâl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SahteOnceki {
    /// Önceki özet `surum-N`; içerik verilmez (1b'den beri).
    Yok,
    /// İçerik `surum-N`, özeti onun SHA-256'sı; sonraki içerik
    /// `sahte-<ad>`.
    Icerikli,
    /// Önceki hâl geri alma sınırını aşar.
    Buyuk,
    /// Dosya yoktu; sonraki içerik `sahte-<ad>`.
    Yoktu,
}

/// `{"yol": ...}` alan sahte araç. Okuyan ya da yazan olabilir.
pub struct SahteArac {
    ad: String,
    yazar: bool,
    /// Kabuk gibi davranır: hazırlık yalıtımlı bir kabuk tarifi taşır.
    kabuk: bool,
    yanit: String,
    hata: Option<String>,
    guvenlik_uyarilari: Vec<String>,
    ag_ozeti: Option<crate::tur_ozeti::AgKomutOzeti>,
    /// Kabuk gibi değiştirdiği dosyalar (aşama 3a Task 6): çalışınca bunlar
    /// sonucun `degisiklikler`'ine girer; `dosyalar` verildiyse oraya da
    /// `kabuk-<ad>` yazılır. `None`: değişiklikler bilinmiyor.
    degistirdigi: Option<Vec<String>>,
    gecikme: Duration,
    on_hal: AtomicU64,
    /// Yazan aracın hazırlığında önceki içerik nasıl verilir (aşama 3a).
    onceki: SahteOnceki,
    /// Verilirse yazan araç bu bellekteki dosya sistemine yazar; önceki
    /// içerik de oradan okunur.
    dosyalar: Option<Arc<SahteGeriYukleyici>>,
    cagrilar: Mutex<Vec<Value>>,
    aktif: AtomicU32,
    azami_aktif: AtomicU32,
    sunulur: AtomicBool,
}

impl SahteArac {
    fn yeni(ad: &str, yazar: bool) -> Self {
        Self {
            ad: ad.into(),
            yazar,
            kabuk: false,
            yanit: format!("{ad} tamam"),
            hata: None,
            guvenlik_uyarilari: Vec::new(),
            ag_ozeti: None,
            degistirdigi: None,
            gecikme: Duration::ZERO,
            on_hal: AtomicU64::new(0),
            onceki: SahteOnceki::Yok,
            dosyalar: None,
            cagrilar: Mutex::new(Vec::new()),
            aktif: AtomicU32::new(0),
            azami_aktif: AtomicU32::new(0),
            sunulur: AtomicBool::new(true),
        }
    }

    pub fn okuyan(ad: &str) -> Self {
        Self::yeni(ad, false)
    }

    pub fn yazan(ad: &str) -> Self {
        Self::yeni(ad, true)
    }

    /// Yalıtımlı kabuk gibi hazırlanan sahte araç: `{"yol": ".", "ag": bool}`;
    /// komut `test`, dizin `yol`'dur.
    pub fn kabuk(ad: &str) -> Self {
        Self {
            kabuk: true,
            ..Self::yeni(ad, true)
        }
    }

    pub fn yanit(mut self, yanit: String) -> Self {
        self.yanit = yanit;
        self
    }

    pub fn hata(mut self, hata: String) -> Self {
        self.hata = Some(hata);
        self
    }

    pub fn guvenlik_uyarisi(mut self, mesaj: &str) -> Self {
        self.guvenlik_uyarilari.push(mesaj.into());
        self
    }

    pub fn ag_ozeti(mut self, ag: crate::tur_ozeti::AgKomutOzeti) -> Self {
        self.ag_ozeti = Some(ag);
        self
    }

    /// Kabuğun sonucundaki değişiklikler; kabuk olmayan araçta ve
    /// `degistirdigi` verilmediyse yok.
    fn kabuk_degisiklikleri(&self) -> Option<Box<crate::arac::DosyaDegisiklikleri>> {
        let yollar = self.degistirdigi.as_ref().filter(|_| self.kabuk)?;
        Some(Box::new(crate::arac::DosyaDegisiklikleri::olustur(
            yollar
                .iter()
                .map(|y| (y.clone(), crate::arac::DegisiklikTuru::Degisti)),
            false,
        )))
    }

    /// Çalışınca bu dosyaları değiştirmiş gibi davranır (kabuk görüntüsü).
    pub fn degistirdigi(mut self, yollar: &[&str]) -> Self {
        self.degistirdigi = Some(yollar.iter().map(|y| (*y).to_string()).collect());
        self
    }

    pub fn gecikme(mut self, gecikme: Duration) -> Self {
        self.gecikme = gecikme;
        self
    }

    /// Yazan aracın hazırlığı önceki içeriği (`surum-N`) ve sonraki özeti
    /// (`sahte-<ad>`) taşır; geri alma bunları saklar.
    pub fn onceki_icerikli(mut self) -> Self {
        self.onceki = SahteOnceki::Icerikli;
        self
    }

    /// Yazan aracın "dosyası" yoktu: önceki özet yok, sonraki özet var.
    pub fn onceki_yok(mut self) -> Self {
        self.onceki = SahteOnceki::Yoktu;
        self
    }

    /// Yazan aracın önceki hâli geri alma sınırını aşar (`buyuk`).
    pub fn onceki_buyuk(mut self) -> Self {
        self.onceki = SahteOnceki::Buyuk;
        self
    }

    /// Yazan araç bellekteki dosya sistemine yazar: hazırlıkta önceki içerik
    /// oradan okunur (yoksa "yoktu"), çalışınca argümandaki `icerik` (yoksa
    /// `sahte-<ad>`) yazılır.
    pub fn dosyalar(mut self, dosyalar: Arc<SahteGeriYukleyici>) -> Self {
        self.dosyalar = Some(dosyalar);
        self
    }

    /// Sahte yazan aracın "yazdığı" içerik.
    pub fn sonraki_icerik(&self) -> String {
        format!("sahte-{}", self.ad)
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

    /// Aracın modele sunulup sunulmadığını değiştirir (`Arac::sunulur_mu`).
    pub fn sunulur_ayarla(&self, sunulur: bool) {
        self.sunulur.store(sunulur, Ordering::SeqCst);
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
            "properties": {
                "yol": {"type": "string"},
                "ag": {"type": "boolean"},
                "icerik": {"type": "string"}
            },
            "required": ["yol"],
            "additionalProperties": false
        })
    }

    fn yazar_mi(&self) -> bool {
        self.yazar
    }

    fn sunulur_mu(&self) -> bool {
        self.sunulur.load(Ordering::SeqCst)
    }

    async fn hazirla(&self, argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
        if self.kabuk {
            return Ok(Hazirlik {
                aciklama: "`test` komutunu çalıştırmak istiyor.".into(),
                tum_calisma_alani: true,
                kabuk: Some(KabukTarifi {
                    komut: "test".into(),
                    bicim: KabukBicimi::Program,
                    program: Some("test".into()),
                    dizin: argumanlar["yol"].as_str().unwrap_or(".").to_string(),
                    ag: argumanlar["ag"].as_bool().unwrap_or(false),
                    ag_hostlari: Vec::new(),
                    yalitim: YalitimOzeti::Yalitimli {
                        arka_uc: "sahte".into(),
                        profil: "p".into(),
                    },
                    ilk_cagri: false,
                    alan_hazirligi: false,
                }),
                ..Default::default()
            });
        }
        let yol = CalismaYolu::coz(argumanlar["yol"].as_str().unwrap_or_default())?;
        let mut h = Hazirlik {
            aciklama: format!("sahte {} `{yol}`", self.ad),
            ..Default::default()
        };
        if self.yazar {
            let surum = self.on_hal.load(Ordering::SeqCst);
            let hedef = match (&self.dosyalar, self.onceki) {
                (Some(dosyalar), _) => {
                    let sonraki = argumanlar["icerik"]
                        .as_str()
                        .map_or_else(|| self.sonraki_icerik(), str::to_owned);
                    let mut hedef = YazmaHedefi {
                        sonraki_ozet: Some(ozet_hex(sonraki.as_bytes())),
                        ..YazmaHedefi::yeni(yol.clone(), None, 1)
                    };
                    if let Some((icerik, baglanti)) = dosyalar.ham(yol.as_str()) {
                        hedef.onceki_ozet = Some(ozet_hex(&icerik));
                        hedef.onceki_bayt = icerik.len() as u64;
                        hedef.onceki_icerik = Some(icerik.into());
                        hedef.baglanti_sayisi = baglanti;
                    }
                    hedef
                }
                (None, onceki) => match onceki {
                    SahteOnceki::Yok => YazmaHedefi::yeni(yol, Some(format!("surum-{surum}")), 1),
                    SahteOnceki::Icerikli => {
                        let onceki = format!("surum-{surum}").into_bytes();
                        YazmaHedefi {
                            onceki_ozet: Some(ozet_hex(&onceki)),
                            onceki_bayt: onceki.len() as u64,
                            onceki_icerik: Some(onceki.into()),
                            sonraki_ozet: Some(ozet_hex(self.sonraki_icerik().as_bytes())),
                            ..YazmaHedefi::yeni(yol, None, 1)
                        }
                    }
                    SahteOnceki::Yoktu => YazmaHedefi {
                        sonraki_ozet: Some(ozet_hex(self.sonraki_icerik().as_bytes())),
                        ..YazmaHedefi::yeni(yol, None, 1)
                    },
                    SahteOnceki::Buyuk => YazmaHedefi {
                        onceki_ozet: Some(ozet_hex(format!("buyuk-{surum}").as_bytes())),
                        onceki_bayt: crate::geri_alma::DOSYA_SINIRI + 1,
                        buyuk: true,
                        sonraki_ozet: Some(ozet_hex(self.sonraki_icerik().as_bytes())),
                        ..YazmaHedefi::yeni(yol, None, 1)
                    },
                },
            };
            h.yazilan.push(hedef);
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
        if let (Some(dosyalar), true) = (&self.dosyalar, self.kabuk) {
            for yol in self.degistirdigi.iter().flatten() {
                dosyalar.yaz(yol, format!("kabuk-{}", self.ad).as_bytes());
            }
        } else if let (Some(dosyalar), true, None) = (&self.dosyalar, self.yazar, &self.hata) {
            let yol = argumanlar["yol"].as_str().unwrap_or_default();
            let icerik = argumanlar["icerik"]
                .as_str()
                .map_or_else(|| self.sonraki_icerik(), str::to_owned);
            dosyalar.yaz(yol, icerik.as_bytes());
        }
        let simdi = self.aktif.fetch_add(1, Ordering::SeqCst) + 1;
        self.azami_aktif.fetch_max(simdi, Ordering::SeqCst);
        let sonuc = tokio::select! {
            _ = tokio::time::sleep(self.gecikme) => match &self.hata {
                Some(h) => Err(AracHatasi::Gecersiz(h.clone())),
                None => Ok(AracCiktisi {
                    icerik: self.yanit.clone(),
                    kaynak: format!("sahte:{}", self.ad),
                    guvenlik_uyarilari: self.guvenlik_uyarilari.clone(),
                    degisiklikler: self.kabuk_degisiklikleri(),
                    kalfa_uretimi: self.yazar,
                    ag: self.ag_ozeti.clone().map(Box::new),
                }),
            },
            _ = iptal.cancelled() => Err(AracHatasi::Iptal),
        };
        self.aktif.fetch_sub(1, Ordering::SeqCst);
        sonuc
    }
}

/// Bellekteki dosya sistemi: yol → (içerik, bağlantı sayısı). Geri alma
/// testlerinde çalışma alanının yerini tutar; sahte yazan araç da buraya
/// yazabilir (`SahteArac::dosyalar`).
#[derive(Default)]
pub struct SahteGeriYukleyici {
    dosyalar: Mutex<BTreeMap<String, (Vec<u8>, u64)>>,
    kaldirilan_dizinler: Mutex<Vec<String>>,
}

impl SahteGeriYukleyici {
    pub fn yeni() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn yaz(&self, yol: &str, icerik: &[u8]) {
        let mut d = self.dosyalar.lock().unwrap();
        let baglanti = d.get(yol).map_or(1, |(_, b)| *b);
        d.insert(yol.to_string(), (icerik.to_vec(), baglanti));
    }

    pub fn oku(&self, yol: &str) -> Option<Vec<u8>> {
        self.ham(yol).map(|(icerik, _)| icerik)
    }

    fn ham(&self, yol: &str) -> Option<(Vec<u8>, u64)> {
        self.dosyalar.lock().unwrap().get(yol).cloned()
    }

    pub fn sil(&self, yol: &str) {
        self.dosyalar.lock().unwrap().remove(yol);
    }

    /// Dosyaya ikinci bir ad (hard link) verilmiş gibi yapar.
    pub fn baglanti_ekle(&self, yol: &str) {
        if let Some((_, b)) = self.dosyalar.lock().unwrap().get_mut(yol) {
            *b += 1;
        }
    }

    /// `bos_dizini_kaldir`'ın çağrıldığı dizinler, sırayla.
    pub fn kaldirilan_dizinler(&self) -> Vec<String> {
        self.kaldirilan_dizinler.lock().unwrap().clone()
    }
}

impl GeriYukleyici for SahteGeriYukleyici {
    fn durum(&self, yol: &CalismaYolu) -> io::Result<Option<DosyaDurumu>> {
        Ok(self
            .ham(yol.as_str())
            .map(|(icerik, baglanti_sayisi)| DosyaDurumu {
                ozet: ozet_hex(&icerik),
                baglanti_sayisi,
            }))
    }

    fn geri_yaz(&self, yol: &CalismaYolu, icerik: Option<&[u8]>, beklenen: &str) -> io::Result<()> {
        let mut d = self.dosyalar.lock().unwrap();
        let Some((simdiki, baglanti)) = d.get(yol.as_str()) else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "dosya yok"));
        };
        if ozet_hex(simdiki) != beklenen {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Dosya önizlemeden sonra değişti; geri yazılmadı.",
            ));
        }
        if *baglanti > 1 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Dosyanın başka bir adı var (hard link).",
            ));
        }
        match icerik {
            Some(icerik) => {
                d.insert(yol.as_str().to_string(), (icerik.to_vec(), 1));
            }
            None => {
                d.remove(yol.as_str());
            }
        }
        Ok(())
    }

    fn bos_dizini_kaldir(&self, yol: &CalismaYolu) -> io::Result<bool> {
        self.kaldirilan_dizinler
            .lock()
            .unwrap()
            .push(yol.as_str().to_string());
        Ok(false)
    }
}

/// Oturumu bir test içinden süren yardımcı: komut gönderir, olayları zaman
/// aşımıyla bekler ve gelen her olayı `gecmis`'e yazar.
pub struct Surucu {
    pub tutamac: OturumTutamaci,
    olaylar: OlayAlici,
    pub gecmis: Vec<Zarf<Olay>>,
    zaman_asimi: Duration,
}

impl Surucu {
    pub fn yeni(tutamac: OturumTutamaci, olaylar: OlayAlici) -> Self {
        Self {
            tutamac,
            olaylar,
            gecmis: Vec::new(),
            zaman_asimi: Duration::from_secs(5),
        }
    }

    pub fn zaman_asimi(mut self, sure: Duration) -> Self {
        self.zaman_asimi = sure;
        self
    }

    pub async fn gonder(&self, komut: Komut) {
        self.tutamac
            .gonder(komut)
            .await
            .expect("oturum açık olmalı");
    }

    /// Olay alıcısını bırakır ve oturum görevi bitene kadar bekler; ardından
    /// kayıt yeniden açılabilir.
    pub async fn kapat(self) {
        let Self {
            tutamac, olaylar, ..
        } = self;
        drop(olaylar);
        tokio::time::timeout(Duration::from_secs(5), tutamac.kapanmasini_bekle())
            .await
            .expect("oturum kapanmalı");
    }

    /// Bütçe kartını bekler ve kartın sayaçlarını kabul eder (5b tasarımı §7).
    /// `sinirlar` verilmezse kartın sınırları korunur; verilirse onlar kullanılır.
    pub async fn butce_devam_et(&mut self, sinirlar: Option<ButceSinirlari>) -> ButceGorunumu {
        let durum = match self
            .bekle(|o| matches!(o, Olay::ButceKarariGerekli { .. }))
            .await
            .yuk
        {
            Olay::ButceKarariGerekli { durum } => durum,
            diger => unreachable!("{diger:?}"),
        };
        self.gonder(Komut::ButceKarari {
            karar: ButceKarari::DevamEt {
                sinirlar: sinirlar.unwrap_or(durum.sinirlar),
                gozlenen_baslatilan: durum.baslatilan,
                gozlenen_bilinmeyen: durum.bilinmeyen,
            },
        })
        .await;
        durum
    }

    pub async fn sonraki(&mut self) -> Zarf<Olay> {
        let zarf = tokio::time::timeout(self.zaman_asimi, self.olaylar.recv())
            .await
            .expect("olay beklerken zaman aşımı")
            .expect("olay kanalı kapandı");
        self.gecmis.push(zarf.clone());
        zarf
    }

    pub async fn bekle(&mut self, kosul: impl Fn(&Olay) -> bool) -> Zarf<Olay> {
        loop {
            let zarf = self.sonraki().await;
            if kosul(&zarf.yuk) {
                return zarf;
            }
        }
    }

    pub async fn tur_sonu(&mut self) -> BitisNedeni {
        match self
            .bekle(|o| matches!(o, Olay::Tamamlandi { .. }))
            .await
            .yuk
        {
            Olay::Tamamlandi { neden } => neden,
            diger => unreachable!("{diger:?}"),
        }
    }

    pub fn metin(&self) -> String {
        self.gecmis
            .iter()
            .filter_map(|z| match &z.yuk {
                Olay::TokenParcasi { metin } => Some(metin.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn hatalar(&self) -> Vec<String> {
        self.gecmis
            .iter()
            .filter_map(|z| match &z.yuk {
                Olay::Hata { mesaj } => Some(mesaj.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn olaylar(&self) -> Vec<Olay> {
        self.gecmis.iter().map(|z| z.yuk.clone()).collect()
    }
}

/// Bir ortama verilecek bütçe talepçisi ve alıcısı. Alıcıyı çağıran tutar ve
/// izni o verir (ya da vermez): fake oturumun yerini tutar.
pub fn butce_kanali() -> (ButceTalepci, tokio::sync::mpsc::Receiver<ButceTalebi>) {
    // Yalnız izin talepleri iletilir; eklenti bitişi araç ortamından geçer,
    // bu kanaldaki bitiş mesajları burada yok sayılır.
    let (talepci, mut alici) = ButceTalepci::kanal();
    let (tx, rx) = tokio::sync::mpsc::channel(16);
    tokio::spawn(async move {
        while let Some(mesaj) = alici.recv().await {
            if let ButceMesaji::Talep(talep) = mesaj
                && tx.send(talep).await.is_err()
            {
                break;
            }
        }
    });
    (talepci, rx)
}

/// Var olmayan bir kök: git koruması yoktur, yalnızca gizli kalıplar çalışır.
pub fn sanal_siniflandirici() -> Arc<Siniflandirici> {
    Arc::new(
        Siniflandirici::yeni(std::env::temp_dir().join("kalfa-sanal-calisma-alani"), &[])
            .expect("sınıflandırıcı"),
    )
}

/// Her izin talebini sırayla kabul eden sahte bütçe kanalı: bütçe kapısı hiç
/// kapanmaz. Bitişler yok sayılır. Bütçe davranışını değil, yardımcının kendi
/// mantığını (ağ çağrısı sayısı, süre, sonuç) sınayan testler için.
pub fn sinirsiz_butce_talepci() -> ButceTalepci {
    let (talepci, mut alici) = ButceTalepci::kanal();
    tokio::spawn(async move {
        let mut sira = 0;
        while let Some(mesaj) = alici.recv().await {
            if let ButceMesaji::Talep(talep) = mesaj {
                sira += 1;
                let _ = talep.yanit.send(Some(ButceIslemId(sira)));
            }
        }
    });
    talepci
}

/// Her izin talebini reddeden sahte bütçe kanalı: izin yok, çağıran ağa çıkmaz.
pub fn reddeden_butce_talepci() -> ButceTalepci {
    let (talepci, mut alici) = ButceTalepci::kanal();
    tokio::spawn(async move {
        while let Some(mesaj) = alici.recv().await {
            if let ButceMesaji::Talep(talep) = mesaj {
                let _ = talep.yanit.send(None);
            }
        }
    });
    talepci
}

/// Yeni bir kilit yöneticisiyle bağlam. Aynı çalışma alanını paylaşan iki
/// oturum için bağlamı `clone()`'la: kilitler ortak kalır.
pub fn test_baglami(
    saglayici: Arc<SahteSaglayici>,
    araclar: Vec<Arc<dyn Arac>>,
    siniflandirici: Arc<Siniflandirici>,
) -> OturumBaglami {
    OturumBaglami {
        sart_kapisi: Arc::new(SartKapisi::yeni(saglayici.clone())),
        denetci: Arc::new(crate::denetci::YapilandirilmisSiniflandirici::yeni(
            saglayici.clone(),
        )),
        denetci_kipi: None,
        saglayici,
        araclar: Arc::new(AracKayitDefteri::yeni(araclar).expect("araç kaydı")),
        siniflandirici,
        kilitler: Arc::new(KaynakKilitleri::yeni()),
        geri_yukleyici: None,
    }
}

/// Yazma hatası benzeten kayıt hedefi (spec §6.6 testleri).
pub struct BozulanHedef {
    ekleme_bozuk: bool,
    kalan_kalici: usize,
}

impl BozulanHedef {
    /// Her yazım başarısız olur.
    pub fn hep_bozuk() -> OturumKaydi {
        OturumKaydi::hedefle(Box::new(Self {
            ekleme_bozuk: true,
            kalan_kalici: 0,
        }))
    }

    /// Satır eklemek başarılı, kalıcılaştırmak (fsync) başarısız: yalnızca yan
    /// etkili araçtan önceki "başladı" kaydı düşer.
    pub fn kalicida_bozuk() -> OturumKaydi {
        Self::kalicidan_sonra_bozuk(0)
    }

    /// İlk n kalıcılaştırma başarılı, sonraki başarısız.
    pub fn kalicidan_sonra_bozuk(n: usize) -> OturumKaydi {
        OturumKaydi::hedefle(Box::new(Self {
            ekleme_bozuk: false,
            kalan_kalici: n,
        }))
    }
}

impl KayitHedefi for BozulanHedef {
    fn ekle(&mut self, _satir: &[u8]) -> io::Result<()> {
        if self.ekleme_bozuk {
            Err(io::Error::other("disk dolu"))
        } else {
            Ok(())
        }
    }

    fn kalici_yap(&mut self) -> io::Result<()> {
        if self.kalan_kalici == 0 {
            Err(io::Error::other("disk dolu"))
        } else {
            self.kalan_kalici -= 1;
            Ok(())
        }
    }
}

pub fn test_oturumu(baglam: OturumBaglami, ayarlar: OturumAyarlari) -> Surucu {
    let (tutamac, olaylar) = baslat(ayarlar, baglam).expect("oturum başlatılamadı");
    Surucu::yeni(tutamac, olaylar)
}
