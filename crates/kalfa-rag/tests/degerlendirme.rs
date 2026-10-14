//! Canlı değerlendirme (aşama 4 tasarımı §11.5, başarı ölçütü §3.5): Kalfa
//! deposunda Türkçe sorgular; beklenen dosya ilk 5 sonuçta olmalı, sorguların
//! en az %80'inde. Kredi harcar; varsayılan olarak atlanır.
//!
//! Çalıştırma (depo kökünde, `.env` içinde EVREN_API_KEY ve EVREN_BASE_URL):
//!   set -a && . ./.env && set +a && cargo test -p kalfa-rag --test degerlendirme -- --ignored --nocapture
//!
//! İndeks `target/kalfa-rag-degerlendirme`'de kalır; sonraki koşu yalnızca
//! değişen dosyaları gömer. `KALFA_RAG_DEPO` (depo köküne göre) başka bir
//! indeks verir; boş bir dizinle tam ilk indekslemenin maliyeti ölçülür.
//! `KALFA_TOKEN_BUTCE` verilirse bu koşuda sayılan `usage.prompt_tokens`
//! bu sayıya ulaşınca yeni istek gönderilmez (EVREN'in günlük token sınırına
//! karşı). Kalan kredi (`x-evren-credits-remaining`) ayrıca raporlanır.
//! `KALFA_YALNIZ_INDEKS` verilirse indeksleme bitince sorgulara geçilmez.

mod ortak;

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_trait::async_trait;
use kalfa_core::saglayici::{EvrenSaglayici, Saglayici, SaglayiciAkisi};
use kalfa_evren::tipler::{ChatRequest, ChatResponse, ModelInfo, TermsStatus, TermsText};
use kalfa_evren::{
    ApiAnahtari, EkUc, EvrenError, EvrenIstemcisi, HamYanit, VARSAYILAN_TABAN, YanitMeta,
};
use ortak::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct Sorgu {
    sorgu: String,
    beklenen: Vec<String>,
}

/// EVREN sağlayıcısını sarar; eklenti isteklerinin sayısını ve yanıttaki
/// `usage.prompt_tokens`'ı uca göre toplar (ilk indekslemenin gerçek maliyeti).
/// Şart kabulünü hiçbir koşulda iletmez (üst belge §11.8).
struct Sayan {
    ic: EvrenSaglayici,
    gomme_istek: AtomicU64,
    gomme_token: AtomicU64,
    siralama_istek: AtomicU64,
    siralama_token: AtomicU64,
    /// İndeksleme döngüsünde yeniden sıralama gönderilmez (sonuç kullanılmaz,
    /// her biri ~39 000 token); eklenti kosinüs sonucuna düşer.
    siralama_kapali: AtomicBool,
    /// İlk ve son yanıttaki kalan kredi.
    kredi: Mutex<(Option<f64>, Option<f64>)>,
    butce: Option<u64>,
}

impl Sayan {
    fn harcanan_kredi(&self) -> Option<f64> {
        match *self.kredi.lock().unwrap() {
            (Some(ilk), Some(son)) => Some(ilk - son),
            _ => None,
        }
    }

    fn ozet(&self) -> String {
        format!(
            "gömme: {} istek, {} token; yeniden sıralama: {} istek, {} token; kalan kredi ilk/son yanıtta: {:?}",
            self.gomme_istek.load(Ordering::Relaxed),
            self.gomme_token.load(Ordering::Relaxed),
            self.siralama_istek.load(Ordering::Relaxed),
            self.siralama_token.load(Ordering::Relaxed),
            *self.kredi.lock().unwrap(),
        )
    }
}

#[async_trait]
impl Saglayici for Sayan {
    async fn akis(&self, istek: ChatRequest) -> Result<SaglayiciAkisi, EvrenError> {
        self.ic.akis(istek).await
    }
    async fn sohbet(&self, istek: ChatRequest) -> Result<(ChatResponse, YanitMeta), EvrenError> {
        self.ic.sohbet(istek).await
    }
    async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError> {
        self.ic.sart_durumu().await
    }
    async fn sart_metni(&self) -> Result<TermsText, EvrenError> {
        self.ic.sart_metni().await
    }
    async fn sart_kabul(&self, _surum: u32) -> Result<(), EvrenError> {
        Err(EvrenError::GecersizYanit(
            "değerlendirmede şartlar kabul edilmez".into(),
        ))
    }
    async fn modeller(&self) -> Result<Vec<ModelInfo>, EvrenError> {
        self.ic.modeller().await
    }
    async fn ek_uc(&self, uc: EkUc, govde: &serde_json::Value) -> Result<HamYanit, EvrenError> {
        if uc == EkUc::Siralama && self.siralama_kapali.load(Ordering::Relaxed) {
            return Err(EvrenError::HizSiniri { tekrar_dene: None });
        }
        let harcanan =
            self.gomme_token.load(Ordering::Relaxed) + self.siralama_token.load(Ordering::Relaxed);
        if self.butce.is_some_and(|b| harcanan >= b) {
            return Err(EvrenError::GecersizYanit(format!(
                "token bütçesi doldu: {harcanan} / {:?}",
                self.butce
            )));
        }
        let yanit = self.ic.ek_uc(uc, govde).await?;
        if let Some(kalan) = yanit
            .meta
            .kredi
            .as_ref()
            .and_then(|k| k.kalan)
            .and_then(|k| k.to_string().parse::<f64>().ok())
        {
            let mut k = self.kredi.lock().unwrap();
            if k.0.is_none() {
                println!("ilk yanıtta kalan kredi: {kalan}");
            }
            k.0.get_or_insert(kalan);
            k.1 = Some(kalan);
        }
        let token = serde_json::from_str::<serde_json::Value>(&yanit.govde)
            .ok()
            .and_then(|d| d["usage"]["prompt_tokens"].as_u64())
            .unwrap_or(0);
        let (istek, toplam) = match uc {
            EkUc::Gomme => (&self.gomme_istek, &self.gomme_token),
            EkUc::Siralama => (&self.siralama_istek, &self.siralama_token),
        };
        if istek.fetch_add(1, Ordering::Relaxed) % 5 == 4 {
            println!("ara durum: {}", self.ozet());
        }
        toplam.fetch_add(token, Ordering::Relaxed);
        Ok(yanit)
    }
}

#[tokio::test]
#[ignore = "canlı EVREN API; EVREN_API_KEY gerekir, kredi harcar"]
async fn turkce_sorgularin_en_az_yuzde_sekseninde_beklenen_dosya_ilk_beste() {
    let anahtar = std::env::var("EVREN_API_KEY").expect("EVREN_API_KEY tanımlı olmalı");
    let taban = std::env::var("EVREN_BASE_URL").unwrap_or_else(|_| VARSAYILAN_TABAN.into());
    let sayan = Arc::new(Sayan {
        ic: EvrenSaglayici::yeni(EvrenIstemcisi::yeni(taban, ApiAnahtari::yeni(anahtar)).unwrap()),
        gomme_istek: AtomicU64::new(0),
        gomme_token: AtomicU64::new(0),
        siralama_istek: AtomicU64::new(0),
        siralama_token: AtomicU64::new(0),
        siralama_kapali: AtomicBool::new(true),
        kredi: Mutex::new((None, None)),
        butce: std::env::var("KALFA_TOKEN_BUTCE")
            .ok()
            .map(|b| b.parse().expect("KALFA_TOKEN_BUTCE sayı olmalı")),
    });
    let saglayici: Arc<dyn Saglayici> = sayan.clone();
    let kok = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let kok = kok.canonicalize().unwrap();
    let depo = kok.join(
        std::env::var("KALFA_RAG_DEPO").unwrap_or_else(|_| "target/kalfa-rag-degerlendirme".into()),
    );
    std::fs::create_dir_all(&depo).unwrap();
    let arac = arac(&kok, &depo, saglayici).await;

    // İlk indeksleme dakikalık hız sınırına takılabilir (R19): söylenen süre
    // kadar beklenir ve yeniden çağrılır; indeks kaldığı yerden sürer. Günlük
    // token sınırında (bütün anahtarların toplamı, ~15 M) beklemek boşunadır;
    // test sıfırlanma anını söyleyip durur.
    let bas = Instant::now();
    let mut beklemeler = 0;
    loop {
        match ara(&arac, "indeks", 1).await {
            Ok(c) if !baslik(&c).contains("İndeks eksik") => break,
            Ok(c) if baslik(&c).contains("günlük token sınırı") => gunluk_sinir(&c, &sayan),
            Ok(c) => println!("{}", c.lines().take(2).collect::<Vec<_>>().join(" / ")),
            Err(e) if e.contains("günlük token sınırı") => gunluk_sinir(&e, &sayan),
            Err(e) if e.contains("hız sınırı") && beklemeler < 30 => {
                beklemeler += 1;
                let sn = bekleme_suresi(&e);
                println!("hız sınırı ({beklemeler}): {sn} sn bekleniyor");
                tokio::time::sleep(std::time::Duration::from_secs(sn + 1)).await;
            }
            Err(e) => panic!("{e}"),
        }
    }
    println!(
        "indeksleme: {:.0} sn, {beklemeler} hız sınırı beklemesi; {}",
        bas.elapsed().as_secs_f64(),
        sayan.ozet()
    );
    if std::env::var_os("KALFA_YALNIZ_INDEKS").is_some() {
        return;
    }
    sayan.siralama_kapali.store(false, Ordering::Relaxed);

    // 20 sonuç istenir: yeniden sıralama yine 50 adayı puanlar (aynı token).
    // Sonuçlar dosyaya göre tekildir; ilk 5 sonuç ilk 5 farklı dosyadır.
    let sorgular: Vec<Sorgu> = serde_json::from_str(include_str!("degerlendirme.json")).unwrap();
    let mut isabet = 0;
    for (i, s) in sorgular.iter().enumerate() {
        let bas = Instant::now();
        let cikti = loop {
            match ara(&arac, &s.sorgu, 20).await {
                Err(e) if e.contains("günlük token sınırı") => gunluk_sinir(&e, &sayan),
                Err(e) if e.contains("hız sınırı") => {
                    let sn = bekleme_suresi(&e);
                    println!("hız sınırı: {sn} sn bekleniyor");
                    tokio::time::sleep(std::time::Duration::from_secs(sn + 1)).await;
                }
                r => break r.unwrap(),
            }
        };
        let sure = bas.elapsed();
        if i == 0 {
            println!("{}", cikti.lines().next().unwrap_or_default());
        }
        let yollar = sonuc_yollari(&cikti);
        let beklenen = |y: &String| s.beklenen.iter().any(|b| y.starts_with(b.as_str()));
        let bulundu = yollar.iter().take(5).any(beklenen);
        isabet += u32::from(bulundu);
        println!(
            "{} {:>5.1} sn  {}\n    {:?}",
            if bulundu { "✓" } else { "✗" },
            sure.as_secs_f64(),
            s.sorgu,
            yollar
        );
    }
    let oran = f64::from(isabet) / sorgular.len() as f64;
    println!("isabet: {isabet}/{} ({:.0}%)", sorgular.len(), oran * 100.0);
    println!("toplam maliyet: {}", sayan.ozet());
    println!(
        "harcanan kredi (ilk yanıttan sona): {:?}",
        sayan.harcanan_kredi()
    );
    assert!(oran >= 0.8, "isabet oranı %80'in altında: {oran:.2}");
}

/// Sonuçlardan önceki başlık satırları. Sonuç parçaları da "İndeks eksik"
/// gibi metinler içerebilir (örneğin `arama.rs`'in testleri); durum yalnız
/// başlıktan okunur.
fn baslik(cikti: &str) -> &str {
    cikti.split("\n\n").next().unwrap_or_default()
}

#[test]
fn baslik_sonuc_metnini_saymaz() {
    let tam = "\"indeks\" için 1 sonuç (indeks: 2 dosya; bu aramada güncellenen: 0).\n\n\
               [1] dosya:a.rs:1-2 (puan 0,64)\nassert!(!tam.contains(\"İndeks eksik\"));";
    assert!(!baslik(tam).contains("İndeks eksik"));
    let yarim = "\"indeks\" için 1 sonuç (indeks: 1 dosya; bu aramada güncellenen: 1).\n\
                 İndeks eksik: 2 dosyanın 1 tanesi indekslendi.\n\n[1] dosya:a.rs:1-2";
    assert!(baslik(yarim).contains("İndeks eksik"));
}

/// "EVREN hız sınırı; <n> sn sonra yeniden deneyin" iletisinden `n`; yoksa 60.
fn bekleme_suresi(ileti: &str) -> u64 {
    ileti
        .split_whitespace()
        .find_map(|k| k.parse().ok())
        .unwrap_or(60)
}

fn gunluk_sinir(ileti: &str, sayan: &Sayan) -> ! {
    panic!(
        "EVREN'in günlük token sınırı doldu; değerlendirme sınır sıfırlanınca yeniden \
         koşturulmalı. İndeks kaldığı yerden sürer.\nBu koşuda: {}\n{ileti}",
        sayan.ozet()
    )
}
