//! G4/G8 canlı ölçümü. Aynı 256 parçayı toplu/sıralı/paralel gönderir.
//! Metin, vektör, anahtar ve kredi bakiyesi günlüğe yazılmaz. İndeks okunmaz
//! ya da değiştirilmez; sabit kayıttaki bayt aralıkları kaynakta doğrulanır.
//! Şart kabulünü çağıran bir yol yoktur. Hız/günlük sınırda yeniden denenmez.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use kalfa_evren::{ApiAnahtari, EkUc, EvrenError, EvrenIstemcisi, VARSAYILAN_TABAN};
use kalfa_plugin_sdk::{gomme_govdesi, gomme_yanitini_coz};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::task::JoinSet;

const MODEL: &str = "qwen3-embedding-8b";
const BOYUT: usize = 1024;
const SORGU: &str = "Instruct: Given a code search query, retrieve relevant code or documentation passages\nQuery: yarım kalmış araç çağrısı oturum yeniden açılınca nasıl işaretleniyor";
const YOLLAR: &[&str] = &[
    "crates/kalfa-core/src/kurtarma.rs",
    "crates/kalfa-core/src/kilit.rs",
    "crates/kalfa-core/src/geri_alma.rs",
    "crates/kalfa-sandbox/src/linux/seccomp.rs",
    "crates/kalfa-sandbox/src/tarama.rs",
    "crates/kalfa-rag/src/arama.rs",
    "crates/kalfa-rag/src/indeks.rs",
    "docs/superpowers/specs/2026-09-21-kalfa-design.md",
    "docs/superpowers/specs/2026-09-24-kalfa-2-kabuk-yalitim-design.md",
    "docs/superpowers/specs/2026-09-22-kalfa-1c-design.md",
    "docs/superpowers/plans/2026-09-24-kalfa-2a-kabuk.md",
];

#[derive(Deserialize)]
struct Girdi {
    yol: String,
    bas_bayt: usize,
    son_bayt: usize,
    ozet: String,
}

fn ozet(bayt: &[u8]) -> String {
    Sha256::digest(bayt)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn girdiler() -> Result<Vec<String>, String> {
    let kok = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let kayitlar: Vec<Girdi> = serde_json::from_str(include_str!("gomme-girdileri.json"))
        .map_err(|_| "girdi kaydı JSON değil")?;
    if kayitlar.len() != 256 {
        return Err("256 girdi bekleniyordu".into());
    }
    let mut icerikler = BTreeMap::new();
    let mut gorulen = BTreeSet::new();
    let mut metinler = Vec::new();
    for k in kayitlar {
        if !YOLLAR.contains(&k.yol.as_str()) {
            return Err("girdi yolu izin listesinde değil".into());
        }
        if !icerikler.contains_key(&k.yol) {
            let icerik = std::fs::read_to_string(kok.join(&k.yol))
                .map_err(|_| format!("kaynak okunamadı: {}", k.yol))?
                .replace("\r\n", "\n");
            icerikler.insert(k.yol.clone(), icerik);
        }
        let metin = icerikler
            .get(&k.yol)
            .and_then(|s| s.get(k.bas_bayt..k.son_bayt))
            .ok_or("girdi aralığı geçersiz")?;
        if ozet(metin.as_bytes()) != k.ozet || !gorulen.insert(k.ozet) {
            return Err("girdi özeti değişmiş ya da yineleniyor".into());
        }
        metinler.push(metin.to_string());
    }
    Ok(metinler)
}

fn guvenli_hata(hata: EvrenError) -> String {
    match hata {
        EvrenError::HizSiniri { tekrar_dene } => format!("hız sınırı; bekleme: {tekrar_dene:?}"),
        EvrenError::GunlukSinir { sifirlanma } => {
            format!("günlük sınır; sıfırlanma: {sifirlanma:?}")
        }
        EvrenError::AnahtarGecersiz => "anahtar geçersiz".into(),
        EvrenError::SartKabulGerekli { .. } => "şart kabulü gerekiyor; ölçüm durdu".into(),
        EvrenError::Sunucu { durum, .. } => format!("EVREN durum kodu: {durum}"),
        EvrenError::Ag { .. } => "ağ hatası".into(),
        _ => "EVREN yanıtı geçersiz ya da sınırı aştı".into(),
    }
}

async fn gonder(
    istemci: Arc<EvrenIstemcisi>,
    bas: usize,
    metinler: Vec<String>,
) -> Result<(usize, Vec<Vec<f32>>, Value), String> {
    let govde: Value = serde_json::from_str(&gomme_govdesi(MODEL, &metinler, BOYUT as u32))
        .map_err(|_| "gömme gövdesi çözülemedi")?;
    let istek_bayt = govde.to_string().len();
    if istek_bayt > 1024 * 1024 {
        return Err("istek 1 MiB sınırını aşıyor".into());
    }
    let zaman_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "saat Unix başlangıcından önce")?
        .as_millis();
    let baslangic = Instant::now();
    let yanit = tokio::time::timeout(Duration::from_secs(240), istemci.ek_uc(EkUc::Gomme, &govde))
        .await
        .map_err(|_| "240 sn ölçüm zaman aşımı")?
        .map_err(guvenli_hata)?;
    let istek_ms = baslangic.elapsed().as_secs_f64() * 1000.0;
    let token = serde_json::from_str::<Value>(&yanit.govde)
        .ok()
        .and_then(|v| v["usage"]["prompt_tokens"].as_u64())
        .ok_or("usage.prompt_tokens yok; maliyet ölçülemedi")?;
    let coz_bas = Instant::now();
    let vektorler = gomme_yanitini_coz(&yanit.govde, metinler.len(), BOYUT)
        .map_err(|_| "SDK yanıt doğrulaması düşüyor")?;
    let coz_ms = coz_bas.elapsed().as_secs_f64() * 1000.0;
    let kayit = json!({"bas":bas,"adet":metinler.len(),"zaman_ms":zaman_ms,
        "istek_ms":istek_ms,"coz_ms":coz_ms,"token":token,
        "istek_bayt":istek_bayt,"yanit_bayt":yanit.govde.len(),
        "model":yanit.meta.yonlendirilen_model.or(yanit.model)});
    Ok((bas, vektorler, kayit))
}

#[tokio::test]
#[ignore = "G4/G8 canlı EVREN ölçümü; kredi harcar, EVREN_API_KEY gerekir"]
async fn canli_gomme_toplu_ve_paralel_olcumu() -> Result<(), String> {
    let metinler = girdiler()?;
    let girdi_ozeti = ozet(
        serde_json::to_string(&metinler)
            .map_err(|_| "girdi kodlanamadı")?
            .as_bytes(),
    );
    let ortam = std::env::var("KALFA_OLCUM_ORTAMI").map_err(|_| "ölçüm ortamını belirtin")?;
    println!(
        "{}",
        json!({"asama":"girdiler","ortam":ortam,"adet":metinler.len(),
        "girdi_ozeti":girdi_ozeti,"karakter":metinler.iter().map(|m| m.chars().count()).sum::<usize>(),
        "bayt":metinler.iter().map(String::len).sum::<usize>()})
    );
    if std::env::var("KALFA_OLCUM_KURU").as_deref() == Ok("1") {
        return Ok(());
    }
    let baslangic_kipi = match std::env::var("KALFA_OLCUM_BASLANGIC") {
        Ok(s) => s
            .parse::<usize>()
            .map_err(|_| "başlangıç kipi sayı değil")?,
        Err(_) => 0,
    };
    if baslangic_kipi >= 15 {
        return Err("başlangıç kipi 0..15 aralığında olmalı".into());
    }
    let anahtar = std::env::var("EVREN_API_KEY").map_err(|_| "EVREN_API_KEY tanımlı değil")?;
    let taban = std::env::var("EVREN_BASE_URL").unwrap_or_else(|_| VARSAYILAN_TABAN.into());
    let istemci =
        Arc::new(EvrenIstemcisi::yeni(taban, ApiAnahtari::yeni(anahtar)).map_err(guvenli_hata)?);
    let kipler = [(256, 1), (64, 1), (16, 1), (128, 2), (64, 4)];
    let mut beklenen: Option<Vec<Vec<f32>>> = None;
    // Her turda kip sırası döner; tek toplu istek her zaman ilk/son olmaz.
    for tur in 0..3 {
        for i in 0..kipler.len() {
            if tur * kipler.len() + i < baslangic_kipi {
                continue;
            }
            let (toplu, paralel) = kipler[(tur + i) % kipler.len()];
            // Kipler arasında kısa ara; süre karşılaştırmasına katılmaz.
            // Başka kullanım varsa yine 429 gelebilir; otomatik tekrar yoktur.
            tokio::time::sleep(Duration::from_secs(2)).await;
            let (_, _, kayit) = gonder(istemci.clone(), 0, vec![SORGU.into()]).await?;
            println!(
                "{}",
                json!({"asama":"sorgu","ortam":ortam,"tur":tur,
                "toplu":toplu,"paralel":paralel,"istek":kayit})
            );

            let isler: Vec<_> = metinler
                .chunks(toplu)
                .enumerate()
                .map(|(j, m)| (j * toplu, m.to_vec()))
                .collect();
            let baslangic = Instant::now();
            let mut sonuclar: Vec<Option<Vec<f32>>> = vec![None; metinler.len()];
            let mut kayitlar = Vec::new();
            let mut hata = None;
            for grup in isler.chunks(paralel) {
                let mut gorevler = JoinSet::new();
                for (bas, metinler) in grup {
                    gorevler.spawn(gonder(istemci.clone(), *bas, metinler.clone()));
                }
                while let Some(sonuc) = gorevler.join_next().await {
                    match sonuc
                        .map_err(|_| "ölçüm görevi çöktü".to_string())
                        .and_then(|s| s)
                    {
                        Ok((bas, vektorler, kayit)) => {
                            for (j, v) in vektorler.into_iter().enumerate() {
                                let yuva =
                                    sonuclar.get_mut(bas + j).ok_or("girdi aralığı aşıldı")?;
                                if yuva.is_some() {
                                    return Err("girdi iki kez döndü".into());
                                }
                                *yuva = Some(v);
                            }
                            kayitlar.push(kayit);
                        }
                        Err(e) => {
                            hata = Some(e);
                        }
                    }
                }
                if hata.is_some() {
                    break;
                }
            }
            let tam_ms = baslangic.elapsed().as_secs_f64() * 1000.0;
            kayitlar.sort_by_key(|v| v["bas"].as_u64());
            let token: u64 = kayitlar.iter().filter_map(|v| v["token"].as_u64()).sum();
            if let Some(hata) = hata {
                println!(
                    "{}",
                    json!({"asama":"yarim","ortam":ortam,"tur":tur,
                    "toplu":toplu,"paralel":paralel,"token":token,"hata":hata,"istekler":kayitlar})
                );
                return Err(hata);
            }
            let vektorler: Vec<_> = sonuclar
                .into_iter()
                .collect::<Option<Vec<_>>>()
                .ok_or("girdi eksik döndü")?;
            if vektorler
                .iter()
                .any(|v| v.iter().map(|x| x * x).sum::<f32>() <= 0.0)
            {
                return Err("sıfır vektör karşılaştırılamaz".into());
            }
            let mut en_az_kosinus = 1.0f32;
            if let Some(beklenen) = &beklenen {
                for (a, b) in vektorler.iter().zip(beklenen) {
                    let ab: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
                    let aa: f32 = a.iter().map(|x| x * x).sum();
                    let bb: f32 = b.iter().map(|x| x * x).sum();
                    let kosinus = ab / (aa * bb).sqrt();
                    if !kosinus.is_finite() {
                        return Err("kosinüs sonlu değil".into());
                    }
                    en_az_kosinus = en_az_kosinus.min(kosinus);
                }
                if !en_az_kosinus.is_finite() || en_az_kosinus < 0.9999 {
                    return Err("toplu/paralel yanıtlar aynı girdiye eşdeğer vektör vermedi".into());
                }
            } else {
                beklenen = Some(vektorler);
            }
            println!(
                "{}",
                json!({"asama":"toplu","ortam":ortam,"tur":tur,
                "toplu":toplu,"paralel":paralel,"tam_ms":tam_ms,"token":token,
                "girdi_ozeti":girdi_ozeti,"en_az_kosinus":en_az_kosinus,"istekler":kayitlar})
            );
        }
    }
    Ok(())
}
