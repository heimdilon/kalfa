//! Sahte sağlayıcının senaryoları: bir model isteğine verilecek yanıtın
//! adımları ve bunlardan üretilen zamanlı akış.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::stream;
use kalfa_evren::tipler::{FunctionCall, ToolCall, Usage};
use kalfa_evren::{AkisKanali, AkisOlayi};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::time::Instant;

/// Gecikme ölçümündeki damganın başı. Damga `[[e2e:<no>:<ms>]]` biçimindedir;
/// `<ms>` parçanın üretildiği anın Unix zamanıdır (milisaniye).
pub const DAMGA_ONEKI: &str = "[[e2e:";

static CAGRI_SAYACI: AtomicU64 = AtomicU64::new(1);

/// Bir kullanıcı mesajına verilecek yanıtlar. Turun `i`. model isteği
/// `yanitlar[i]`'yi alır: araç sonuçlarından sonra gelen istek bir sonrakini.
/// Yanıtlar bitmişse tur "Tamamlandı." ile biter.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Senaryo {
    pub yanitlar: Vec<Vec<Adim>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "tur", rename_all = "snake_case", deny_unknown_fields)]
pub enum Adim {
    /// Metin parçası.
    Metin { metin: String },
    /// Düşünce parçası.
    Dusunce { metin: String },
    /// Araç çağrısı. Yanıtın bütün çağrıları sonda tek olayda verilir ve
    /// yanıt `tool_calls` nedeniyle biter.
    Arac {
        ad: String,
        argumanlar: serde_json::Value,
    },
    /// Sonraki adımdan önce bekler.
    Bekle { ms: u64 },
    /// Turun son araç sonucunu "Araç sonucu: …" diye yazar. Ret gerekçesinin
    /// modele gittiği böyle görülür.
    AracSonucu,
    /// Gecikme ölçümü için zaman damgalı parçalar (1c tasarımı §11.4).
    Damgali { desen: Desen, sayi: u32 },
    /// Tüketildiği kadar hızlı, bitmeyen düşünce akışı (kabul kapısı 2). Tur
    /// iptal edilince durur. Düşünce paneli kapalıyken çizilmez: arayüz
    /// yeniden işlemeye başlayınca biriken megabaytları çizmekle uğraşmaz.
    Sel { parca_bayt: usize },
}

/// Damgalı parçaların zamanlaması.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "ad", rename_all = "snake_case", deny_unknown_fields)]
pub enum Desen {
    /// `aralik_ms`'de bir metin parçası.
    Duzenli { aralik_ms: u64 },
    /// `en_az`–`en_cok` parçalık yığınlar; yığınların arası `aralik_ms`.
    Yigin {
        en_az: u32,
        en_cok: u32,
        aralik_ms: u64,
    },
    /// Her `aralik_ms`'de bir düşünce ve bir metin parçası.
    Paralel { aralik_ms: u64 },
}

/// Şimdiki Unix zamanı (milisaniye).
pub fn simdi_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64() * 1000.0)
}

fn damga(no: u32) -> String {
    format!("{DAMGA_ONEKI}{no}:{:.3}]]", simdi_ms())
}

/// Adımları akışa çevirir. Üretici ayrı bir görevde çalışır ve akış
/// okundukça ilerler; akış bırakılınca (tur iptal) durur.
pub fn akis(adimlar: Vec<Adim>, son_arac_sonucu: Option<String>) -> AkisKanali {
    let (tx, rx) = mpsc::channel::<AkisOlayi>(1);
    tokio::spawn(async move {
        let _ = uret(adimlar, son_arac_sonucu, &tx).await;
    });
    Box::pin(stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|olay| (Ok(olay), rx))
    }))
}

/// Alıcı gitmişse (akış bırakıldı) `Err` döner.
struct Gonderici<'a>(&'a mpsc::Sender<AkisOlayi>);

impl Gonderici<'_> {
    async fn gonder(&self, olay: AkisOlayi) -> Result<(), ()> {
        self.0.send(olay).await.map_err(|_| ())
    }
}

async fn uret(
    adimlar: Vec<Adim>,
    son_arac_sonucu: Option<String>,
    tx: &mpsc::Sender<AkisOlayi>,
) -> Result<(), ()> {
    let g = Gonderici(tx);
    let mut cagrilar = Vec::new();
    for adim in adimlar {
        match adim {
            Adim::Metin { metin } => g.gonder(AkisOlayi::Metin(metin)).await?,
            Adim::Dusunce { metin } => g.gonder(AkisOlayi::Dusunce(metin)).await?,
            Adim::Arac { ad, argumanlar } => cagrilar.push(ToolCall {
                id: format!("e2e-cagri-{}", CAGRI_SAYACI.fetch_add(1, Ordering::Relaxed)),
                kind: "function".into(),
                function: FunctionCall {
                    name: ad,
                    arguments: argumanlar.to_string(),
                },
            }),
            Adim::Bekle { ms } => tokio::time::sleep(Duration::from_millis(ms)).await,
            Adim::AracSonucu => {
                let sonuc = son_arac_sonucu.clone().unwrap_or_else(|| "yok".into());
                g.gonder(AkisOlayi::Metin(format!("Araç sonucu: {sonuc}")))
                    .await?;
            }
            Adim::Damgali { desen, sayi } => damgali(&g, desen, sayi).await?,
            Adim::Sel { parca_bayt } => {
                let parca = "x".repeat(parca_bayt.max(1));
                loop {
                    g.gonder(AkisOlayi::Dusunce(parca.clone())).await?;
                    tokio::task::yield_now().await;
                }
            }
        }
    }
    let neden = if cagrilar.is_empty() {
        "stop"
    } else {
        g.gonder(AkisOlayi::AracCagrilari(cagrilar)).await?;
        "tool_calls"
    };
    // Tamamlanan yanıt geçerli kullanım bildirir; bütçe bilinmeyen tüketimi
    // sormaz (5b tasarımı §6). Kesilen akışlar bu olayı üretmez.
    g.gonder(AkisOlayi::Kullanim(Usage {
        prompt_tokens: 10,
        completion_tokens: 5,
        total_tokens: 15,
        completion_tokens_details: None,
        evren: None,
    }))
    .await?;
    g.gonder(AkisOlayi::Bitti {
        neden: neden.into(),
    })
    .await
}

/// Parçalar bir saate göre zamanlanır: gecikmeler birikmez.
async fn damgali(g: &Gonderici<'_>, desen: Desen, sayi: u32) -> Result<(), ()> {
    let baslangic = Instant::now();
    let mut no = 0;
    let mut adim = 0u64;
    while no < sayi {
        let (aralik, yigin, paralel) = match desen {
            Desen::Duzenli { aralik_ms } => (aralik_ms, 1, false),
            Desen::Yigin {
                en_az,
                en_cok,
                aralik_ms,
            } => {
                let en_az = en_az.max(1);
                let genislik = en_cok.saturating_sub(en_az) + 1;
                (aralik_ms, en_az + (adim as u32 % genislik), false)
            }
            Desen::Paralel { aralik_ms } => (aralik_ms, 1, true),
        };
        tokio::time::sleep_until(baslangic + Duration::from_millis(aralik * adim)).await;
        for _ in 0..yigin {
            if no >= sayi {
                break;
            }
            no += 1;
            if paralel {
                g.gonder(AkisOlayi::Dusunce(damga(no))).await?;
            }
            g.gonder(AkisOlayi::Metin(damga(no))).await?;
        }
        adim += 1;
    }
    Ok(())
}
