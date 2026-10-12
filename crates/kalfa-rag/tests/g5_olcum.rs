//! G5 aday/dosya, talimat dili, boyut ve parça deneyleri; ürün indeksine yazmaz.
//! Eski indeksin güncel kaynakla hâlâ eşleşen, izinli parçaları sabitlenir.
//! Ürün aramasının artımlı indeksleme/wasm süresi ölçülmez. Canlı çağrılar
//! açık onay ister; kaynak metni/vektörler çıktı kayıtlarına girmez.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use kalfa_core::izin::{Siniflandirici, korumali_listede};
use kalfa_core::yol::CalismaYolu;
use kalfa_evren::{ApiAnahtari, EkUc, EvrenError, EvrenIstemcisi};
use kalfa_plugin::manifest::Manifest;
use kalfa_plugin_sdk::{
    gomme_govdesi, gomme_yanitini_coz, siralama_govdesi, siralama_yanitini_coz,
};
use kalfa_tools::CalismaAlani;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const BOYUT: usize = 1024;
const GOMME: &str = "qwen3-embedding-8b";
const SIRALAMA: &str = "qwen3-reranker-8b";
const HEDEF: &str = "https://evren-llmapi.ssyz.org.tr/v1";
const TALIMAT: &str = "Instruct: Given a code search query, retrieve relevant code or documentation passages\nQuery: ";
const TURKCE_TALIMAT: &str = "Instruct: Bir kod arama sorgusu verildiğinde, ilgili kod veya belge parçalarını bulun\nQuery: ";
const DILLER: [(&str, &str); 2] = [("ingilizce", TALIMAT), ("turkce", TURKCE_TALIMAT)];
const TOKEN_SINIRI: u64 = 5_000_000;
const KIPLER: [(&str, usize, usize); 3] = [
    ("aday50", 50, 50),
    ("aday20", 20, 20),
    ("aday20-dosya2", 20, 2),
];

#[path = "g5_olcum/boyut.rs"]
mod boyut;

#[path = "g5_olcum/parca.rs"]
mod parca;

#[derive(Deserialize)]
struct Meta {
    surum: u32,
    model: String,
    boyut: usize,
    dosyalar: BTreeMap<String, Dosya>,
}

#[derive(Deserialize)]
struct Dosya {
    parcalar: Vec<Kaynak>,
}

#[derive(Clone, Deserialize, Serialize)]
struct Kaynak {
    satir_bas: u32,
    satir_son: u32,
    bas_bayt: usize,
    son_bayt: usize,
    ozet: String,
    dilim: u64,
    sira: usize,
}

struct Parca {
    yol: String,
    sira: usize,
    kaynak: Kaynak,
    metin: String,
    vektor: Vec<f32>,
}

#[derive(Deserialize, Serialize)]
struct Sorgu {
    sorgu: String,
    beklenen: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct Makbuz {
    istek_ozeti: String,
    ms: f64,
    token: u64,
    model: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    vektor: Vec<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    siralama: Vec<(usize, f32)>,
}

fn ozet(bayt: &[u8]) -> String {
    Sha256::digest(bayt)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn deney_yolu(yol: &str) -> bool {
    ["crates/", "ui/", "e2e/", "test-eklentileri/", "docs/"]
        .iter()
        .any(|s| yol.starts_with(s))
        || [
            "README.md",
            "AGENTS.md",
            "Cargo.toml",
            "rust-toolchain.toml",
        ]
        .contains(&yol)
}

fn metni_dogrula<'a>(icerik: &'a str, k: &Kaynak) -> Option<&'a str> {
    let metin = icerik.get(k.bas_bayt..k.son_bayt)?;
    (ozet(metin.as_bytes()) == k.ozet && k.satir_bas > 0 && k.satir_son >= k.satir_bas)
        .then_some(metin)
}

fn vektor_coz(bayt: &[u8], sira: usize) -> Result<Vec<f32>, String> {
    if !bayt.len().is_multiple_of(BOYUT * 4) {
        return Err("vektör dilimi kesilmiş".into());
    }
    let bas = sira.checked_mul(BOYUT * 4).ok_or("vektör konumu taştı")?;
    let son = bas.checked_add(BOYUT * 4).ok_or("vektör konumu taştı")?;
    let satir = bayt.get(bas..son).ok_or("vektör konumu dilim dışında")?;
    let v: Vec<_> = satir
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect();
    let norm = v.iter().map(|x| x * x).sum::<f32>();
    if v.iter().any(|x| !x.is_finite()) || !norm.is_finite() || (norm - 1.0).abs() > 0.01 {
        return Err("indeks vektörü sonlu ve birim uzunlukta değil".into());
    }
    Ok(v)
}

fn girdiler(kok: &Path) -> Result<(Vec<Parca>, Value), String> {
    let depo = kok.join("target/kalfa-rag-degerlendirme");
    let meta_bayt = std::fs::read(depo.join(ozet(b"meta"))).map_err(|_| "mevcut meta okunamadı")?;
    let meta: Meta = serde_json::from_slice(&meta_bayt).map_err(|_| "meta geçersiz")?;
    if meta.surum != 1 || meta.boyut != BOYUT || meta.model != GOMME {
        return Err("indeks sürümü/modeli/boyutu beklenenden farklı".into());
    }
    let sinif = Arc::new(Siniflandirici::yeni(kok, &[]).map_err(|_| "sınıflandırıcı açılamadı")?);
    let alan = CalismaAlani::ac(sinif.clone()).map_err(|_| "çalışma alanı açılamadı")?;
    let manifest = Manifest::coz(include_str!("../kalfa.toml")).map_err(|_| "manifest geçersiz")?;
    let korumali = kalfa_core::git_koruma::korumali_yollar(kok);
    let liste: BTreeSet<_> = alan
        .dosyalar(&tokio_util::sync::CancellationToken::new())
        .map_err(|_| "kaynak listesi okunamadı")?
        .into_iter()
        .map(|y| y.as_str().to_string())
        .collect();
    let mut parcalar = Vec::new();
    let mut dilimler = BTreeMap::new();
    let mut kayitlar = Vec::new();
    let mut atlanan = 0;
    let toplam: usize = meta.dosyalar.values().map(|d| d.parcalar.len()).sum();
    for (yol, d) in &meta.dosyalar {
        let y = CalismaYolu::coz(yol).map_err(|_| "meta yolu geçersiz")?;
        if !deney_yolu(yol)
            || !liste.contains(yol)
            || !manifest.okuyabilir(yol)
            || sinif.gizli_mi(&y)
            || korumali_listede(&y, &korumali)
        {
            atlanan += d.parcalar.len();
            continue;
        }
        let Ok(bayt) = alan.sinirli_oku(&y, &tokio_util::sync::CancellationToken::new()) else {
            atlanan += d.parcalar.len();
            continue;
        };
        let Ok(icerik) = String::from_utf8(bayt) else {
            atlanan += d.parcalar.len();
            continue;
        };
        for (sira, k) in d.parcalar.iter().enumerate() {
            let Some(metin) = metni_dogrula(&icerik, k) else {
                atlanan += 1;
                continue;
            };
            if let std::collections::btree_map::Entry::Vacant(giris) = dilimler.entry(k.dilim) {
                let bayt = std::fs::read(depo.join(ozet(format!("vektor/{}", k.dilim).as_bytes())))
                    .map_err(|_| "indeks dilimi okunamadı")?;
                giris.insert(bayt);
            }
            let vektor = vektor_coz(dilimler.get(&k.dilim).ok_or("dilim yok")?, k.sira)?;
            kayitlar.push(json!({"yol":yol,"sira":sira,"kaynak":k}));
            parcalar.push(Parca {
                yol: yol.clone(),
                sira,
                kaynak: k.clone(),
                metin: metin.into(),
                vektor,
            });
        }
    }
    if parcalar.is_empty() {
        return Err("doğrulanmış parça yok".into());
    }
    let dilim_ozetleri: Vec<_> = dilimler
        .iter()
        .map(|(n, b)| json!({"dilim":n,"ozet":ozet(b)}))
        .collect();
    let girdi_ozeti = ozet(
        serde_json::to_string(&json!({"parcalar":kayitlar,"dilimler":dilim_ozetleri}))
            .map_err(|_| "girdiler kodlanamadı")?
            .as_bytes(),
    );
    let yollar: BTreeSet<_> = parcalar.iter().map(|p| p.yol.as_str()).collect();
    let bilgi = json!({"asama":"girdiler","girdi_ozeti":girdi_ozeti,"meta_ozeti":ozet(&meta_bayt),
        "indeks_dosya":meta.dosyalar.len(),"indeks_parca":toplam,"dosya":yollar.len(),"parca":parcalar.len(),
        "atlanan_parca":atlanan,"metin_bayt":parcalar.iter().map(|p|p.metin.len()).sum::<usize>(),
        "kipler":KIPLER,"token_siniri":TOKEN_SINIRI});
    Ok((parcalar, bilgi))
}

fn kosinus_sirasi(parcalar: &[Parca], q: &[f32]) -> Result<Vec<usize>, String> {
    if q.len() != BOYUT || q.iter().any(|x| !x.is_finite()) {
        return Err("sorgu vektörü geçersiz".into());
    }
    let norm = q.iter().map(|x| x * x).sum::<f32>().sqrt();
    if !norm.is_finite() || norm <= 0.0 {
        return Err("sorgu vektörü sıfır ya da taşmış".into());
    }
    let q: Vec<_> = q.iter().map(|x| x / norm).collect();
    let mut puanlar: Vec<_> = parcalar
        .iter()
        .enumerate()
        .map(|(i, p)| (i, q.iter().zip(&p.vektor).map(|(x, y)| x * y).sum::<f32>()))
        .collect();
    puanlar.sort_by(|a, b| {
        b.1.total_cmp(&a.1)
            .then_with(|| parcalar[a.0].yol.cmp(&parcalar[b.0].yol))
            .then(parcalar[a.0].sira.cmp(&parcalar[b.0].sira))
    });
    Ok(puanlar.into_iter().map(|(i, _)| i).collect())
}

fn adaylari_sec(
    parcalar: &[Parca],
    sira: &[usize],
    adet: usize,
    dosya_siniri: usize,
) -> Vec<usize> {
    let mut sayac = BTreeMap::new();
    sira.iter()
        .copied()
        .filter(|i| {
            let n = sayac.entry(parcalar[*i].yol.as_str()).or_insert(0);
            if *n >= dosya_siniri {
                false
            } else {
                *n += 1;
                true
            }
        })
        .take(adet)
        .collect()
}

fn isabet(yol: &str, beklenen: &[String]) -> bool {
    beklenen
        .iter()
        .any(|b| yol == b || yol.strip_prefix(b).is_some_and(|s| s.starts_with('/')))
}

fn sonuclar(
    parcalar: &[Parca],
    adaylar: &[usize],
    siralama: &[(usize, f32)],
    beklenen: &[String],
    tekil: bool,
) -> Value {
    let mut gorulen = BTreeSet::new();
    let ilk: Vec<_> = siralama.iter().filter_map(|(i,puan)| {
        let p = &parcalar[*adaylar.get(*i)?];
        if tekil && !gorulen.insert(p.yol.as_str()) { return None; }
        Some(json!({"yol":p.yol,"satir_bas":p.kaynak.satir_bas,"satir_son":p.kaynak.satir_son,"ozet":p.kaynak.ozet,"puan":puan}))
    }).take(5).collect();
    let yollar: BTreeSet<_> = ilk.iter().filter_map(|v| v["yol"].as_str()).collect();
    json!({"isabet":yollar.iter().any(|y|isabet(y,beklenen)),"tekil_dosya":yollar.len(),"ilk5":ilk})
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

fn makbuz_dogrula(m: &Makbuz, uc: EkUc, adet: usize) -> Result<(), String> {
    if !m.ms.is_finite() || m.ms < 0.0 || m.token == 0 {
        return Err("makbuz süre/token alanı geçersiz".into());
    }
    match uc {
        EkUc::Gomme => {
            kosinus_sirasi(&[], &m.vektor)?;
            if !m.siralama.is_empty() {
                return Err("gömme makbuzunda sıralama var".into());
            }
        }
        EkUc::Siralama => {
            let mut gorulen = BTreeSet::new();
            if !m.vektor.is_empty()
                || m.siralama.len() != adet
                || m.siralama
                    .iter()
                    .any(|(i, p)| *i >= adet || !p.is_finite() || !gorulen.insert(*i))
            {
                return Err("sıralama makbuzunda eksik/tekrar/geçersiz sonuç".into());
            }
        }
    }
    Ok(())
}

fn onceki_token(depo: &Path) -> Result<u64, String> {
    let mut toplam = 0u64;
    for g in std::fs::read_dir(depo).map_err(|_| "makbuz dizini okunamadı")? {
        let yol = g.map_err(|_| "makbuz yolu okunamadı")?.path();
        if yol.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let m: Makbuz =
            serde_json::from_slice(&std::fs::read(&yol).map_err(|_| "makbuz okunamadı")?)
                .map_err(|_| "makbuz bozuk")?;
        if yol.file_stem().and_then(|s| s.to_str()) != Some(m.istek_ozeti.as_str()) || m.token == 0
        {
            return Err("önceki makbuz bütçesi doğrulanamadı".into());
        }
        toplam = toplam.checked_add(m.token).ok_or("token bütçesi taştı")?;
    }
    Ok(toplam)
}

async fn istek(
    istemci: &EvrenIstemcisi,
    depo: &Path,
    uc: EkUc,
    govde: String,
    adet: usize,
    harcanan: &mut u64,
) -> Result<(Makbuz, bool), String> {
    let istek_ozeti = ozet(govde.as_bytes());
    let yol = depo.join(format!("{istek_ozeti}.json"));
    if yol.exists() {
        let m: Makbuz =
            serde_json::from_slice(&std::fs::read(&yol).map_err(|_| "makbuz okunamadı")?)
                .map_err(|_| "makbuz bozuk")?;
        if m.istek_ozeti != istek_ozeti {
            return Err("makbuz isteği farklı".into());
        }
        makbuz_dogrula(&m, uc, adet)?;
        return Ok((m, true));
    }
    // İstek baytının iki katı + pay, bir sonraki çağrı için ihtiyatlı tavan.
    // Kullanım alanı eksikse tahminle devam edilmez. Başarısız isteklerin
    // sağlayıcıdaki tüketimi bilinmez; sınır/hata ilk çağrıda durdurur.
    if !butceye_sigar(*harcanan, govde.len()) {
        return Err("5 milyon token bütçesi; sonraki çağrı başlatılmadı".into());
    }
    let json: Value = serde_json::from_str(&govde).map_err(|_| "istek JSON değil")?;
    tokio::time::sleep(Duration::from_secs(2)).await;
    let bas = Instant::now();
    let yanit = istemci.ek_uc(uc, &json).await.map_err(guvenli_hata)?;
    let ms = bas.elapsed().as_secs_f64() * 1000.0;
    let kok: Value = serde_json::from_str(&yanit.govde).map_err(|_| "yanıt JSON değil")?;
    let token = kok["usage"]["prompt_tokens"]
        .as_u64()
        .ok_or("token kullanımı eksik; ölçüm durdu")?;
    *harcanan += token;
    let (vektor, siralama) = match uc {
        EkUc::Gomme => (
            gomme_yanitini_coz(&yanit.govde, 1, BOYUT)
                .map_err(|_| "gömme yanıtı geçersiz")?
                .remove(0),
            Vec::new(),
        ),
        EkUc::Siralama => {
            let s = siralama_yanitini_coz(&yanit.govde, adet)
                .map_err(|_| "sıralama yanıtı geçersiz")?;
            if s.len() != adet {
                return Err("tam sıralama dönmedi; ölçüm durdu".into());
            }
            (Vec::new(), s)
        }
    };
    let m = Makbuz {
        istek_ozeti,
        ms,
        token,
        model: yanit.meta.yonlendirilen_model.or(yanit.model),
        vektor,
        siralama,
    };
    makbuz_dogrula(&m, uc, adet)?;
    let mut gecici =
        tempfile::NamedTempFile::new_in(depo).map_err(|_| "makbuz dosyası açılamadı")?;
    serde_json::to_writer(gecici.as_file_mut(), &m).map_err(|_| "makbuz yazılamadı")?;
    gecici
        .as_file()
        .sync_all()
        .map_err(|_| "makbuz diske yazılamadı")?;
    gecici
        .persist_noclobber(yol)
        .map_err(|_| "makbuz kaydedilemedi")?;
    Ok((m, false))
}

fn butceye_sigar(harcanan: u64, istek_bayt: usize) -> bool {
    u64::try_from(istek_bayt)
        .ok()
        .and_then(|n| n.checked_mul(2))
        .and_then(|n| n.checked_add(4096))
        .and_then(|n| n.checked_add(harcanan))
        .is_some_and(|n| n <= TOKEN_SINIRI)
}

fn dil_deneyi_tarifi(girdi_ozeti: &str, sorgular: &[Sorgu]) -> Result<Value, String> {
    Ok(json!({"deney":"talimat-dili","girdi_ozeti":girdi_ozeti,
        "sorgu_ozeti":ozet(&serde_json::to_vec(sorgular).map_err(|_| "sorgular kodlanamadı")?),
        "talimatlar":DILLER,"gomme_modeli":GOMME,"siralama_modeli":SIRALAMA,
        "boyut":BOYUT,"aday":50,"top_n":50,"token_siniri":TOKEN_SINIRI}))
}

#[tokio::test]
#[ignore = "G5 talimat dili; açık onay, EVREN_API_KEY ve mevcut indeks gerekir"]
async fn canli_talimat_dili_karsilastirmasi() -> Result<(), String> {
    let kok = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|_| "depo kökü açılamadı")?;
    let (parcalar, mut bilgi) = girdiler(&kok)?;
    let sorgular: Vec<Sorgu> = serde_json::from_str(include_str!("degerlendirme.json"))
        .map_err(|_| "sorgular geçersiz")?;
    if sorgular.len() != 32 {
        return Err("32 sorgu bekleniyordu".into());
    }
    let tarif = dil_deneyi_tarifi(
        bilgi["girdi_ozeti"].as_str().ok_or("girdi özeti yok")?,
        &sorgular,
    )?;
    let deney_ozeti = ozet(
        serde_json::to_string(&tarif)
            .map_err(|_| "deney tarifi kodlanamadı")?
            .as_bytes(),
    );
    bilgi["kipler"] = json!(["ingilizce", "turkce"]);
    bilgi["tarif"] = tarif;
    bilgi["deney_ozeti"] = json!(deney_ozeti);
    println!("{bilgi}");
    let eksik: Vec<_> = sorgular
        .iter()
        .enumerate()
        .filter(|(_, s)| !parcalar.iter().any(|p| isabet(&p.yol, &s.beklenen)))
        .map(|(i, _)| i)
        .collect();
    println!(
        "{}",
        json!({"asama":"kapsam","beklenen_dosyasi_olmayan_sorgular":eksik,
        "sorgu":32,"gomme_istek":64,"siralama_istek":64,"aday":50,"boyut":BOYUT})
    );
    if std::env::var("KALFA_G5_KURU").as_deref() == Ok("1") {
        return Ok(());
    }
    let onay =
        std::env::var("KALFA_G5_DIL_DENEY_OZETI").map_err(|_| "onaylı dil deneyi özeti gerekli")?;
    if onay != deney_ozeti {
        return Err("dil deneyi onaylanandan farklı".into());
    }
    let anahtar = std::env::var("EVREN_API_KEY").map_err(|_| "EVREN_API_KEY tanımlı değil")?;
    let taban = std::env::var("EVREN_BASE_URL").unwrap_or_else(|_| HEDEF.into());
    if taban.trim_end_matches('/') != HEDEF {
        return Err("EVREN hedefi onaylanan adres değil".into());
    }
    let istemci = EvrenIstemcisi::yeni(taban, ApiAnahtari::yeni(anahtar)).map_err(guvenli_hata)?;
    // Önceki aday deneyiyle aynı gövdeler de yeni deneyde tekrar ölçülür;
    // yalnız bu dil deneyinde tamamlanan çağrılar kesintiden sonra kullanılır.
    let depo = kok.join("target/kalfa-g5-dil").join(&onay);
    std::fs::create_dir_all(&depo).map_err(|_| "dil ölçüm dizini açılamadı")?;
    let onceki = onceki_token(&depo)?;
    let mut harcanan = onceki;
    for (i, s) in sorgular.iter().enumerate() {
        // Dil sırası sorgu başına döner. Yalnız task açıklaması çevrilir;
        // Instruct/Query işaretleri ve gerçek sorgu iki dilde aynıdır.
        for j in 0..DILLER.len() {
            let (dil, talimat) = DILLER[(i + j) % DILLER.len()];
            let (q, onceki) = istek(
                &istemci,
                &depo,
                EkUc::Gomme,
                gomme_govdesi(GOMME, &[format!("{talimat}{}", s.sorgu)], BOYUT as u32),
                1,
                &mut harcanan,
            )
            .await?;
            println!(
                "{}",
                json!({"asama":"sorgu","dil":dil,"sorgu_no":i,"token":q.token,
                "ms":q.ms,"model":q.model,"onceki":onceki,"istek_ozeti":q.istek_ozeti})
            );
            let sira = kosinus_sirasi(&parcalar, &q.vektor)?;
            let adaylar = adaylari_sec(&parcalar, &sira, 50, 50);
            let belgeler: Vec<_> = adaylar.iter().map(|n| parcalar[*n].metin.clone()).collect();
            let (m, onceki) = istek(
                &istemci,
                &depo,
                EkUc::Siralama,
                siralama_govdesi(SIRALAMA, &s.sorgu, &belgeler, adaylar.len() as u32),
                adaylar.len(),
                &mut harcanan,
            )
            .await?;
            let kayitlar: Vec<_> = adaylar
                .iter()
                .map(|n| {
                    let p = &parcalar[*n];
                    json!({"yol":p.yol,"sira":p.sira,"ozet":p.kaynak.ozet})
                })
                .collect();
            println!(
                "{}",
                json!({"asama":"siralama","dil":dil,"sorgu_no":i,"sorgu":s.sorgu,"beklenen":s.beklenen,
                "aday":adaylar.len(),"aday_tekil_dosya":adaylar.iter().map(|n|parcalar[*n].yol.as_str()).collect::<BTreeSet<_>>().len(),
                "aday_isabet":adaylar.iter().any(|n|isabet(&parcalar[*n].yol,&s.beklenen)),"adaylar":kayitlar,
                "token":m.token,"ms":m.ms,"model":m.model,"onceki":onceki,"istek_ozeti":m.istek_ozeti,
                "parca_ilk5":sonuclar(&parcalar,&adaylar,&m.siralama,&s.beklenen,false),
                "dosya_ilk5":sonuclar(&parcalar,&adaylar,&m.siralama,&s.beklenen,true)})
            );
        }
    }
    println!(
        "{}",
        json!({"asama":"bitti","bu_kosu_token":harcanan-onceki,"tum_kosular_token":harcanan,
        "deney_ozeti":onay,"urun_indeksine_yazildi":false})
    );
    Ok(())
}

#[tokio::test]
#[ignore = "G5 canlı EVREN ölçümü; açık onay, EVREN_API_KEY ve mevcut indeks gerekir"]
async fn canli_aday_sayisi_ve_dosya_cesitliligi() -> Result<(), String> {
    let kok = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|_| "depo kökü açılamadı")?;
    let (parcalar, mut bilgi) = girdiler(&kok)?;
    let sorgular: Vec<Sorgu> = serde_json::from_str(include_str!("degerlendirme.json"))
        .map_err(|_| "sorgular geçersiz")?;
    if sorgular.len() != 32 {
        return Err("32 sorgu bekleniyordu".into());
    }
    bilgi["sorgu_ozeti"] = json!(ozet(
        &serde_json::to_vec(&sorgular).map_err(|_| "sorgular kodlanamadı")?
    ));
    println!("{bilgi}");
    let eksik: Vec<_> = sorgular
        .iter()
        .enumerate()
        .filter(|(_, s)| !parcalar.iter().any(|p| isabet(&p.yol, &s.beklenen)))
        .map(|(i, _)| i)
        .collect();
    println!(
        "{}",
        json!({"asama":"kapsam","beklenen_dosyasi_olmayan_sorgular":eksik,"sorgu":sorgular.len(),"gomme_istek":32,"siralama_istek":96})
    );
    if std::env::var("KALFA_G5_KURU").as_deref() == Ok("1") {
        return Ok(());
    }
    let onay = std::env::var("KALFA_G5_GIRDI_OZETI").map_err(|_| "onaylı girdi özeti gerekli")?;
    if bilgi["girdi_ozeti"].as_str() != Some(onay.as_str()) {
        return Err("girdi kümesi onaylanandan farklı".into());
    }
    let anahtar = std::env::var("EVREN_API_KEY").map_err(|_| "EVREN_API_KEY tanımlı değil")?;
    let taban = std::env::var("EVREN_BASE_URL").unwrap_or_else(|_| HEDEF.into());
    if taban.trim_end_matches('/') != HEDEF {
        return Err("EVREN hedefi onaylanan adres değil".into());
    }
    let istemci = EvrenIstemcisi::yeni(taban, ApiAnahtari::yeni(anahtar)).map_err(guvenli_hata)?;
    // Yalnız ölçüm makbuzları ve deney sorgu vektörleri; kaynak metni yok.
    // Ürün indeksi/sorgu önbelleği değildir. Hash farklıysa yeni dizin açılır.
    let depo = kok.join("target/kalfa-g5-olcum").join(&onay);
    std::fs::create_dir_all(&depo).map_err(|_| "ölçüm dizini açılamadı")?;
    let onceki_toplam = onceki_token(&depo)?;
    let mut harcanan = onceki_toplam;
    for (i, s) in sorgular.iter().enumerate() {
        let (q, onceki) = istek(
            &istemci,
            &depo,
            EkUc::Gomme,
            gomme_govdesi(GOMME, &[format!("{TALIMAT}{}", s.sorgu)], BOYUT as u32),
            1,
            &mut harcanan,
        )
        .await?;
        println!(
            "{}",
            json!({"asama":"sorgu","sorgu_no":i,"token":q.token,"ms":q.ms,"model":q.model,"onceki":onceki,"istek_ozeti":q.istek_ozeti})
        );
        let sira = kosinus_sirasi(&parcalar, &q.vektor)?;
        for j in 0..KIPLER.len() {
            let (kip, adet, dosya_siniri) = KIPLER[(i + j) % KIPLER.len()];
            let adaylar = adaylari_sec(&parcalar, &sira, adet, dosya_siniri);
            let belgeler: Vec<_> = adaylar.iter().map(|n| parcalar[*n].metin.clone()).collect();
            let govde = siralama_govdesi(SIRALAMA, &s.sorgu, &belgeler, adaylar.len() as u32);
            let (m, onceki) = istek(
                &istemci,
                &depo,
                EkUc::Siralama,
                govde,
                adaylar.len(),
                &mut harcanan,
            )
            .await?;
            let kayitlar: Vec<_> = adaylar
                .iter()
                .map(|n| {
                    let p = &parcalar[*n];
                    json!({"yol":p.yol,"sira":p.sira,"ozet":p.kaynak.ozet})
                })
                .collect();
            println!(
                "{}",
                json!({"asama":"siralama","sorgu_no":i,"kip":kip,"sorgu":s.sorgu,"beklenen":s.beklenen,
                "aday":adaylar.len(),"aday_tekil_dosya":adaylar.iter().map(|n|parcalar[*n].yol.as_str()).collect::<BTreeSet<_>>().len(),
                "aday_isabet":adaylar.iter().any(|n|isabet(&parcalar[*n].yol,&s.beklenen)),"adaylar":kayitlar,
                "token":m.token,"ms":m.ms,"model":m.model,"onceki":onceki,"istek_ozeti":m.istek_ozeti,
                "parca_ilk5":sonuclar(&parcalar,&adaylar,&m.siralama,&s.beklenen,false),
                "dosya_ilk5":sonuclar(&parcalar,&adaylar,&m.siralama,&s.beklenen,true)})
            );
        }
    }
    println!(
        "{}",
        json!({"asama":"bitti","bu_kosu_token":harcanan-onceki_toplam,"tum_kosular_token":harcanan,"girdi_ozeti":onay,"urun_indeksine_yazildi":false})
    );
    Ok(())
}

fn sahte(yol: &str, sira: usize) -> Parca {
    Parca {
        yol: yol.into(),
        sira,
        kaynak: Kaynak {
            satir_bas: 1,
            satir_son: 1,
            bas_bayt: 0,
            son_bayt: 1,
            ozet: ozet(b"x"),
            dilim: 0,
            sira,
        },
        metin: "x".into(),
        vektor: vec![0.0; BOYUT],
    }
}

#[test]
fn dosya_siniri_kosinusu_korur_ve_uzaktaki_dosyayla_doldurur() {
    let p = [
        sahte("a", 0),
        sahte("a", 1),
        sahte("a", 2),
        sahte("b", 0),
        sahte("c", 0),
    ];
    assert_eq!(adaylari_sec(&p, &[0, 1, 2, 3, 4], 4, 2), [0, 1, 3, 4]);
    assert_eq!(adaylari_sec(&p, &[0, 1, 2, 3, 4], 2, 2), [0, 1]);
    assert_eq!(adaylari_sec(&p, &[0, 1, 2, 3, 4], 20, 1), [0, 3, 4]);
}

#[test]
fn tekil_sonuc_en_yuksek_puanli_parcayi_tutar_ve_dosya_isabetini_degistirir() {
    let p = [
        sahte("a", 0),
        sahte("a", 1),
        sahte("a", 2),
        sahte("a", 3),
        sahte("a", 4),
        sahte("b", 0),
    ];
    let s = [(0, 0.9), (1, 0.8), (2, 0.7), (3, 0.6), (4, 0.5), (5, 0.4)];
    let beklenen = vec!["b".into()];
    assert_eq!(
        sonuclar(&p, &[0, 1, 2, 3, 4, 5], &s, &beklenen, false)["isabet"],
        false
    );
    let sonuc = sonuclar(&p, &[0, 1, 2, 3, 4, 5], &s, &beklenen, true);
    assert_eq!(sonuc["isabet"], true);
    assert_eq!(sonuc["tekil_dosya"], 2);
    assert_eq!(sonuc["ilk5"][0]["puan"], json!(0.9f32));
    assert!(!isabet("baska", &beklenen));
    assert!(isabet("b/dosya.rs", &beklenen));
}

#[test]
fn kosinus_esitliginde_yol_ve_parca_sirasi_belirleyicidir() {
    let mut p = [sahte("b", 0), sahte("a", 1), sahte("a", 0)];
    for x in &mut p {
        x.vektor[0] = 1.0;
    }
    let mut q = vec![0.0; BOYUT];
    q[0] = 2.0;
    assert_eq!(kosinus_sirasi(&p, &q).unwrap(), [2, 1, 0]);
    q[0] = 0.0;
    assert!(kosinus_sirasi(&p, &q).is_err());
}

#[test]
fn kesilmis_vektor_ve_degismis_veya_utf8i_bolen_aralik_reddedilir() {
    let mut k = sahte("a", 0).kaynak;
    assert_eq!(metni_dogrula("x", &k), Some("x"));
    assert_eq!(metni_dogrula("y", &k), None);
    assert_eq!(metni_dogrula("ş", &k), None);
    k.bas_bayt = 2;
    assert_eq!(metni_dogrula("x", &k), None);
    assert!(vektor_coz(&[0; 3], 0).is_err());
    assert!(vektor_coz(&vec![0; BOYUT * 4], 0).is_err());
    let mut bayt = vec![0; BOYUT * 4];
    bayt[..4].copy_from_slice(&1.0f32.to_le_bytes());
    assert!(vektor_coz(&bayt, 0).is_ok());
    assert!(vektor_coz(&bayt, 1).is_err());
    bayt[..4].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(vektor_coz(&bayt, 0).is_err());
}

#[test]
fn olcumun_kaynak_kapsami_aciktir() {
    for y in [
        "crates/kalfa-core/src/kurtarma.rs",
        "docs/superpowers/specs/a.md",
        "README.md",
    ] {
        assert!(deney_yolu(y));
    }
    for y in [
        ".env",
        "design/a.md",
        "research/a.txt",
        "target/a.rs",
        "refs/shot-a.png",
    ] {
        assert!(!deney_yolu(y));
    }
}

#[test]
fn devam_makbuzu_tekrar_ve_eksik_siralama_kabul_etmez_butceyi_korur() {
    let depo = tempfile::tempdir().unwrap();
    let mut m = Makbuz {
        istek_ozeti: "a".into(),
        ms: 1.0,
        token: 20,
        model: None,
        vektor: Vec::new(),
        siralama: vec![(0, 0.5), (1, 0.4)],
    };
    assert!(makbuz_dogrula(&m, EkUc::Siralama, 2).is_ok());
    m.siralama[1].0 = 0;
    assert!(makbuz_dogrula(&m, EkUc::Siralama, 2).is_err());
    m.siralama.pop();
    assert!(makbuz_dogrula(&m, EkUc::Siralama, 2).is_err());
    std::fs::write(depo.path().join("a.json"), serde_json::to_vec(&m).unwrap()).unwrap();
    assert_eq!(onceki_token(depo.path()).unwrap(), 20);
    std::fs::write(depo.path().join("b.json"), serde_json::to_vec(&m).unwrap()).unwrap();
    assert!(onceki_token(depo.path()).is_err());
}

#[test]
fn butce_tasmasi_ve_eski_makbuzla_dolan_sinir_cagri_baslatmaz() {
    assert!(butceye_sigar(0, 1000));
    assert!(butceye_sigar(TOKEN_SINIRI - 6096, 1000));
    assert!(!butceye_sigar(TOKEN_SINIRI - 6095, 1000));
    assert!(!butceye_sigar(TOKEN_SINIRI, 0));
    assert!(!butceye_sigar(u64::MAX, 1000));
    assert!(!butceye_sigar(0, usize::MAX));
}

#[test]
fn dil_onay_tarifi_kaynak_kumesini_ve_beklenen_dosyayi_baglar() {
    let mut sorgular = vec![Sorgu {
        sorgu: "sorgu".into(),
        beklenen: vec!["a.rs".into()],
    }];
    let ilk = dil_deneyi_tarifi("kaynak-a", &sorgular).unwrap();
    assert_ne!(ilk, dil_deneyi_tarifi("kaynak-b", &sorgular).unwrap());
    sorgular[0].beklenen[0] = "b.rs".into();
    assert_ne!(ilk, dil_deneyi_tarifi("kaynak-a", &sorgular).unwrap());
    assert_eq!(ilk["talimatlar"][0][1], TALIMAT);
    assert_eq!(ilk["talimatlar"][1][1], TURKCE_TALIMAT);
    assert_eq!(ilk["aday"], 50);
    assert_eq!(ilk["boyut"], BOYUT);
}
