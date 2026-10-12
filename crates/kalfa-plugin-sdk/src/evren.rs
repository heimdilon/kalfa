//! EVREN yardımcıları (aşama 4 tasarımı §12): eklenti yazarı gövdeyi elle
//! kurmaz, yanıtı elle çözmez. İstek `host::evren_cagir` ile gider; host
//! gövdeyi izin listesiyle doğrular (§9.2).
//!
//! Gövde kuran ve yanıtı çözen fonksiyonlar saf tutulmuştur: `host` çağrısı
//! yalnızca wasm içinde çalışır, bu yüzden testler onları host hedefinde sınar.

use std::fmt::Display;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use crate::{EvrenUcu, Hata, host};

const GOMME: &str = "gömme";
const SIRALAMA: &str = "sıralama";

/// Metinleri gömer: `model`, `dimensions: boyut`, `encoding_format:
/// "base64"`. Her metin için bir vektör, aynı sırayla. Metin yoksa host'a
/// gitmez.
pub fn gomme(model: &str, metinler: &[String], boyut: u32) -> Result<Vec<Vec<f32>>, Hata> {
    if metinler.is_empty() {
        return Ok(vec![]);
    }
    let govde = gomme_govdesi(model, metinler, boyut);
    let yanit = host::evren_cagir(EvrenUcu::Embeddings, &govde)?;
    gomme_yanitini_coz(&yanit, metinler.len(), boyut as usize)
}

/// Belgeleri sorguya göre yeniden sıralar: (belgenin sırası, puan), puana
/// göre azalan, en çok `ilk_n` tane. Belge yoksa host'a gitmez.
pub fn yeniden_sirala(
    model: &str,
    sorgu: &str,
    belgeler: &[String],
    ilk_n: u32,
) -> Result<Vec<(usize, f32)>, Hata> {
    if belgeler.is_empty() {
        return Ok(vec![]);
    }
    let govde = siralama_govdesi(model, sorgu, belgeler, ilk_n);
    let yanit = host::evren_cagir(EvrenUcu::Rerank, &govde)?;
    let mut sonuc = siralama_yanitini_coz(&yanit, belgeler.len())?;
    sonuc.truncate(ilk_n as usize);
    Ok(sonuc)
}

/// `gomme`'nin istek gövdesi: `model`, `input`, `dimensions` ve
/// `encoding_format: "base64"`.
pub fn gomme_govdesi(model: &str, metinler: &[String], boyut: u32) -> String {
    json!({
        "model": model,
        "input": metinler,
        "dimensions": boyut,
        "encoding_format": "base64",
    })
    .to_string()
}

/// `gomme`'nin yanıtını çözer. `data` `index`'e göre sıralanır; her
/// `embedding` base64 metni (küçük endian `f32`) ya da sayı dizisi olabilir.
/// `adet` öğe ve her vektör `boyut` uzunlukta olmalı; indeksler `0..adet`'i
/// tam kaplamalı.
pub fn gomme_yanitini_coz(yanit: &str, adet: usize, boyut: usize) -> Result<Vec<Vec<f32>>, Hata> {
    let kok = yanit_json(yanit, GOMME)?;
    let veri = dizi(&kok, "data", GOMME)?;
    let gelen = veri.len();
    if gelen != adet {
        return Err(bicim(
            GOMME,
            format!("{gelen} öğe var, {adet} bekleniyordu"),
        ));
    }
    let mut ciftler = Vec::with_capacity(adet);
    for oge in veri {
        let sira = indeks(oge)
            .filter(|&s| s < adet)
            .ok_or_else(|| bicim(GOMME, "index yok ya da aralık dışı"))?;
        let vektor =
            vektoru_coz(oge.get("embedding"), boyut).map_err(|neden| bicim(GOMME, neden))?;
        ciftler.push((sira, vektor));
    }
    ciftler.sort_by_key(|(sira, _)| *sira);
    // Sıralıyken her index konumuna eşitse indeksler 0..adet'i tam kaplar.
    if ciftler
        .iter()
        .enumerate()
        .any(|(konum, (sira, _))| konum != *sira)
    {
        return Err(bicim(GOMME, "index'ler tekrarlıyor ya da eksik"));
    }
    Ok(ciftler.into_iter().map(|(_, vektor)| vektor).collect())
}

/// `yeniden_sirala`'nın istek gövdesi: `model`, `query`, `documents`, `top_n`
/// ve `return_documents: false`.
pub fn siralama_govdesi(model: &str, sorgu: &str, belgeler: &[String], ilk_n: u32) -> String {
    json!({
        "model": model,
        "query": sorgu,
        "documents": belgeler,
        "top_n": ilk_n,
        "return_documents": false,
    })
    .to_string()
}

/// `yeniden_sirala`'nın yanıtını çözer. `results[].index` belge sırasıdır:
/// `belge_sayisi`'ndan küçük ve tekrarsız olmalı. Sonuç puana göre azalan
/// döner; eşit puanda belge sırası öne geçer.
pub fn siralama_yanitini_coz(yanit: &str, belge_sayisi: usize) -> Result<Vec<(usize, f32)>, Hata> {
    let kok = yanit_json(yanit, SIRALAMA)?;
    let sonuclar = dizi(&kok, "results", SIRALAMA)?;
    let mut goruldu = vec![false; belge_sayisi];
    let mut sonuc = Vec::with_capacity(sonuclar.len());
    for oge in sonuclar {
        let sira = indeks(oge).ok_or_else(|| bicim(SIRALAMA, "index yok ya da tam sayı değil"))?;
        let yuva = goruldu.get_mut(sira).ok_or_else(|| {
            bicim(
                SIRALAMA,
                format!("index {sira} belge aralığının dışında (belge sayısı: {belge_sayisi})"),
            )
        })?;
        if *yuva {
            return Err(bicim(SIRALAMA, format!("index {sira} tekrarlıyor")));
        }
        *yuva = true;
        let puan = oge
            .get("relevance_score")
            .and_then(Value::as_f64)
            .map(|p| p as f32)
            .filter(|p| p.is_finite())
            .ok_or_else(|| bicim(SIRALAMA, "relevance_score yok ya da sonlu değil"))?;
        sonuc.push((sira, puan));
    }
    sonuc.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    Ok(sonuc)
}

/// Biçim hatası: EVREN yanıtı beklenen şekilde değil.
fn bicim(ne: &str, neden: impl Display) -> Hata {
    Hata::Evren(format!("{ne} yanıtı beklenen biçimde değil: {neden}"))
}

fn yanit_json(yanit: &str, ne: &str) -> Result<Value, Hata> {
    serde_json::from_str(yanit).map_err(|e| bicim(ne, format!("JSON çözülemedi: {e}")))
}

fn dizi<'a>(kok: &'a Value, anahtar: &str, ne: &str) -> Result<&'a [Value], Hata> {
    kok.get(anahtar)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| bicim(ne, format!("{anahtar} dizisi yok")))
}

/// `index` alanı tam sayıysa onu döndürür.
fn indeks(oge: &Value) -> Option<usize> {
    oge.get("index")
        .and_then(Value::as_u64)
        .and_then(|s| usize::try_from(s).ok())
}

/// `embedding` alanını `boyut` uzunlukta bir vektöre çevirir: base64 metni
/// (küçük endian `f32`) ya da sayı dizisi. Sonlu olmayan değer kabul edilmez.
fn vektoru_coz(deger: Option<&Value>, boyut: usize) -> Result<Vec<f32>, String> {
    let vektor: Vec<f32> = match deger {
        Some(Value::String(metin)) => {
            let bayt = STANDARD
                .decode(metin)
                .map_err(|e| format!("base64 çözülemedi: {e}"))?;
            let (dortlu, kalan) = bayt.as_chunks::<4>();
            if !kalan.is_empty() {
                return Err("base64 baytı 4'ün katı değil".into());
            }
            dortlu.iter().map(|b| f32::from_le_bytes(*b)).collect()
        }
        Some(Value::Array(sayilar)) => sayilar
            .iter()
            .map(|s| s.as_f64().map(|x| x as f32))
            .collect::<Option<Vec<f32>>>()
            .ok_or_else(|| "sayı olmayan öğe".to_string())?,
        _ => return Err("embedding alanı yok ya da biçimi tanınmıyor".into()),
    };
    let gelen = vektor.len();
    if gelen != boyut {
        return Err(format!("{gelen} boyutlu vektör, {boyut} bekleniyordu"));
    }
    if !vektor.iter().all(|x| x.is_finite()) {
        return Err("vektörde sonlu olmayan değer var".into());
    }
    Ok(vektor)
}

#[cfg(test)]
mod testler {
    use std::fmt::Debug;

    use super::*;

    /// `(index, vektör)` çiftlerinden base64 biçimli bir gömme yanıtı kurar.
    fn base64_yanit(ciftler: &[(usize, Vec<f32>)]) -> String {
        let veri: Vec<Value> = ciftler
            .iter()
            .map(|(sira, vektor)| {
                let bayt: Vec<u8> = vektor.iter().flat_map(|x| x.to_le_bytes()).collect();
                json!({"index": sira, "object": "embedding", "embedding": STANDARD.encode(bayt)})
            })
            .collect();
        json!({"object": "list", "data": veri}).to_string()
    }

    /// Hatanın EVREN iletisini döndürür; başarı ya da başka bir hata testi bozar.
    fn evren_iletisi<T: Debug>(sonuc: Result<T, Hata>) -> String {
        match sonuc {
            Err(Hata::Evren(ileti)) => ileti,
            diger => panic!("Evren hatası bekleniyordu: {diger:?}"),
        }
    }

    /// Kayıt dosyasındaki `data` öğe sayısı.
    fn veri_adedi(yanit: &str) -> usize {
        let kok: Value = serde_json::from_str(yanit).expect("kayıt JSON");
        kok["data"].as_array().map_or(0, Vec::len)
    }

    #[test]
    fn gomme_govdesi_beklenen_alanlari_tasir() {
        let metinler = ["birinci".to_string(), "ikinci".to_string()];
        let govde: Value =
            serde_json::from_str(&gomme_govdesi("qwen3-embedding-8b", &metinler, 1024))
                .expect("geçerli JSON");
        assert_eq!(govde["model"], "qwen3-embedding-8b");
        assert_eq!(govde["input"], json!(["birinci", "ikinci"]));
        assert_eq!(govde["dimensions"].as_u64(), Some(1024));
        assert_eq!(govde["encoding_format"], "base64");
        assert_eq!(govde.as_object().map(|o| o.len()), Some(4));
    }

    #[test]
    fn siralama_govdesi_beklenen_alanlari_tasir() {
        let belgeler = ["a".to_string(), "b".to_string()];
        let govde: Value = serde_json::from_str(&siralama_govdesi(
            "qwen3-reranker-8b",
            "sorgu",
            &belgeler,
            2,
        ))
        .expect("geçerli JSON");
        assert_eq!(govde["model"], "qwen3-reranker-8b");
        assert_eq!(govde["query"], "sorgu");
        assert_eq!(govde["documents"], json!(["a", "b"]));
        assert_eq!(govde["top_n"].as_u64(), Some(2));
        assert_eq!(govde["return_documents"], false);
        assert_eq!(govde.as_object().map(|o| o.len()), Some(5));
    }

    #[test]
    fn bos_girdide_host_cagrilmadan_bos_doner() {
        let vektorler = gomme("qwen3-embedding-8b", &[], 1024).expect("boş girdi hata değil");
        assert!(vektorler.is_empty());
        let sirali =
            yeniden_sirala("qwen3-reranker-8b", "sorgu", &[], 3).expect("boş girdi hata değil");
        assert!(sirali.is_empty());
    }

    #[test]
    fn gomme_kaydi_1024_boyutlu_cozulur() {
        let yanit = include_str!("../../kalfa-evren/tests/kayitlar/embeddings_qwen3_1024.json");
        let adet = veri_adedi(yanit);
        let vektorler = gomme_yanitini_coz(yanit, adet, 1024).expect("kayıt çözülür");
        assert_eq!(vektorler.len(), adet);
        assert!(vektorler.iter().all(|v| v.len() == 1024));
    }

    #[test]
    fn base64_gomme_kaydi_4096_boyutlu_cozulur() {
        let yanit = include_str!("../../kalfa-evren/tests/kayitlar/embeddings_qwen3_base64.json");
        let adet = veri_adedi(yanit);
        let vektorler = gomme_yanitini_coz(yanit, adet, 4096).expect("kayıt çözülür");
        assert_eq!(vektorler.len(), adet);
        assert_eq!(vektorler[0].len(), 4096);
        // İlk sayı, base64 metninin ilk 4 baytının küçük endian f32'sine eşit.
        let kok: Value = serde_json::from_str(yanit).expect("kayıt JSON");
        let metin = kok["data"][0]["embedding"].as_str().expect("base64 metni");
        let bayt = STANDARD.decode(metin).expect("base64 çözülür");
        let ilk: [u8; 4] = bayt[..4].try_into().expect("4 bayt");
        assert_eq!(vektorler[0][0], f32::from_le_bytes(ilk));
    }

    #[test]
    fn base64_gidis_donus_index_sirasina_gore_cozulur() {
        // Yanıtta index sırası karışık; sonuç index'e göre dizilir.
        let yanit = base64_yanit(&[(1, vec![3.0, 4.0, 5.0]), (0, vec![1.0, -2.5, 0.25])]);
        let vektorler = gomme_yanitini_coz(&yanit, 2, 3).expect("çözülür");
        assert_eq!(vektorler, [[1.0, -2.5, 0.25], [3.0, 4.0, 5.0]]);
    }

    #[test]
    fn sayi_dizisi_gomme_yaniti_cozulur() {
        let yanit = json!({"data": [
            {"index": 1, "embedding": [3.0, 4.0, 5.0]},
            {"index": 0, "embedding": [1.0, -2.5, 0.25]},
        ]})
        .to_string();
        let vektorler = gomme_yanitini_coz(&yanit, 2, 3).expect("çözülür");
        assert_eq!(vektorler, [[1.0, -2.5, 0.25], [3.0, 4.0, 5.0]]);
    }

    #[test]
    fn bozuk_gomme_yanitlari_hata_verir() {
        let durumlar = [
            (r#"{"object":"list"}"#.to_string(), 1, 3, "data dizisi yok"),
            (
                base64_yanit(&[(0, vec![1.0, 2.0, 3.0])]),
                2,
                3,
                "1 öğe var, 2 bekleniyordu",
            ),
            (
                base64_yanit(&[(0, vec![1.0, 2.0])]),
                1,
                3,
                "2 boyutlu vektör, 3 bekleniyordu",
            ),
            (
                r#"{"data":[{"index":0,"embedding":"!!!"}]}"#.to_string(),
                1,
                3,
                "base64 çözülemedi",
            ),
            (
                r#"{"data":[{"embedding":[1.0,2.0,3.0]}]}"#.to_string(),
                1,
                3,
                "index yok",
            ),
            (
                base64_yanit(&[(0, vec![1.0, 2.0, 3.0]), (0, vec![1.0, 2.0, 3.0])]),
                2,
                3,
                "index'ler tekrarlıyor",
            ),
            (
                base64_yanit(&[(5, vec![1.0, 2.0, 3.0])]),
                1,
                3,
                "index yok ya da aralık dışı",
            ),
            (
                base64_yanit(&[(0, vec![f32::NAN, 2.0, 3.0])]),
                1,
                3,
                "sonlu olmayan",
            ),
        ];
        for (yanit, adet, boyut, parca) in durumlar {
            let ileti = evren_iletisi(gomme_yanitini_coz(&yanit, adet, boyut));
            assert!(
                ileti.starts_with("gömme yanıtı beklenen biçimde değil"),
                "{ileti}"
            );
            assert!(ileti.contains(parca), "{ileti}");
        }
    }

    #[test]
    fn rerank_kaydi_puana_gore_azalan_cozulur() {
        let yanit = include_str!("../../kalfa-evren/tests/kayitlar/rerank_qwen3.json");
        let sonuc = siralama_yanitini_coz(yanit, 3).expect("kayıt çözülür");
        let siralar: Vec<usize> = sonuc.iter().map(|(sira, _)| *sira).collect();
        assert_eq!(siralar, [0, 1, 2]);
        assert!(sonuc.windows(2).all(|p| p[0].1 >= p[1].1));
        assert!((sonuc[0].1 - 0.0555).abs() < 1e-3);
    }

    #[test]
    fn siralama_puana_gore_azalan_siralar() {
        // Puan sırası, belge sırasından farklı: 1 (0,9), 2 (0,5), 0 (0,1).
        let yanit = r#"{"results":[{"index":0,"relevance_score":0.1},{"index":1,"relevance_score":0.9},{"index":2,"relevance_score":0.5}]}"#;
        let sonuc = siralama_yanitini_coz(yanit, 3).expect("çözülür");
        let siralar: Vec<usize> = sonuc.iter().map(|(sira, _)| *sira).collect();
        assert_eq!(siralar, [1, 2, 0]);
    }

    #[test]
    fn bozuk_siralama_yanitlari_hata_verir() {
        let durumlar = [
            (r#"{"x":1}"#, 3, "results dizisi yok"),
            (
                r#"{"results":[{"index":3,"relevance_score":0.5}]}"#,
                3,
                "index 3 belge aralığının dışında",
            ),
            (
                r#"{"results":[{"index":1,"relevance_score":0.5},{"index":1,"relevance_score":0.4}]}"#,
                3,
                "index 1 tekrarlıyor",
            ),
            (r#"{"results":[{"index":1}]}"#, 3, "relevance_score yok"),
            (
                r#"{"results":[{"index":0,"relevance_score":1e300}]}"#,
                3,
                "sonlu değil",
            ),
            (r#"{"results":[{"relevance_score":0.5}]}"#, 3, "index yok"),
        ];
        for (yanit, belge_sayisi, parca) in durumlar {
            let ileti = evren_iletisi(siralama_yanitini_coz(yanit, belge_sayisi));
            assert!(
                ileti.starts_with("sıralama yanıtı beklenen biçimde değil"),
                "{ileti}"
            );
            assert!(ileti.contains(parca), "{ileti}");
        }
    }
}
