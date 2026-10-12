//! EVREN yolu (aşama 4 tasarımı §8.4, §9.1, §9.2, §9.5): eklentinin
//! `evren-cagir` gövdesinin izin listesi, ilk gönderim izni ve model kataloğu.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use kalfa_core::gonderim::{IstekAsamasi, IstekSonucu};
use kalfa_core::saglayici::Saglayici;
use kalfa_core::sart_kapisi::{KapiGorunumu, SartKapisi};
use kalfa_evren::tipler::ModelInfo;
use kalfa_evren::{EkUc, EvrenError, HamYanit};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use crate::bag::kalfa::plugin::tipler::Hata;
use crate::manifest::EvrenUcu;
use crate::sinir;

/// Bir eklentinin bir çalışma alanındaki durumu: EVREN'e ilk kez veri gönderip
/// gönderemeyeceği (§8.4) ve araçlarının modele sunulup sunulmadığı (§13.3).
/// Uygulamanın kalıcı uygulaması `eklentiler.json`'dadır.
pub trait GonderimIzni: Send + Sync {
    fn var_mi(&self) -> bool;
    /// İzni kaydeder; kaydedilemezse nedeni döner.
    fn ver(&self) -> Result<(), String>;
    /// Eklenti bu çalışma alanında açık mı? Her turda sorulur: kapalıyken
    /// araçları modele sunulmaz ve çağrılırsa çalışmaz.
    fn acik_mi(&self) -> bool {
        true
    }
}

/// Süreç ömrü kadar geçerli izin; testler ve oturumsuz kullanım için.
pub struct BellekIzni(AtomicBool);

impl BellekIzni {
    pub fn yeni(var: bool) -> Arc<Self> {
        Arc::new(Self(AtomicBool::new(var)))
    }
}

impl GonderimIzni for BellekIzni {
    fn var_mi(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    fn ver(&self) -> Result<(), String> {
        self.0.store(true, Ordering::SeqCst);
        Ok(())
    }
}

/// Uygulamanın EVREN bağlantısı: sağlayıcı, şart kapısı ve model kataloğu.
/// Katalog ilk kullanımda alınır; yalnızca alımı başarılıysa önbelleğe girer.
pub struct EvrenBaglami {
    saglayici: Arc<dyn Saglayici>,
    kapi: Arc<SartKapisi>,
    katalog: Mutex<Option<Vec<ModelInfo>>>,
}

impl EvrenBaglami {
    pub fn yeni(saglayici: Arc<dyn Saglayici>, kapi: Arc<SartKapisi>) -> Arc<Self> {
        Arc::new(Self {
            saglayici,
            kapi,
            katalog: Mutex::new(None),
        })
    }

    /// Modelin katalogda bu uca uyduğunu denetler (§9.2).
    pub(crate) async fn model_denetle(&self, model: &str, uc: EkUc) -> Result<(), Hata> {
        let mut katalog = self.katalog.lock().await;
        if katalog.is_none() {
            let liste = self
                .saglayici
                .modeller()
                .await
                .map_err(|_| Hata::Evren("model kataloğu alınamadı".into()))?;
            *katalog = Some(liste);
        }
        let uygun = katalog
            .as_ref()
            .is_some_and(|liste| liste.iter().any(|m| m.id == model && m.task == uc.gorev()));
        if uygun {
            Ok(())
        } else {
            Err(Hata::IzinYok("model bu uç için değil".into()))
        }
    }

    /// Şart kapısı açık değilse istek ağa çıkmaz (§9.5).
    pub(crate) fn sart_denetle(&self) -> Result<(), Hata> {
        if matches!(self.kapi.durum().gorunum, KapiGorunumu::Bosta) {
            Ok(())
        } else {
            Err(Hata::Evren("EVREN şartları yeniden onay bekliyor".into()))
        }
    }

    pub(crate) async fn ek_istegi(&self, uc: EkUc, govde: &Value) -> Result<HamYanit, EvrenError> {
        self.saglayici.ek_uc(uc, govde).await
    }
}

/// Gövdenin izin listesiyle doğrulanmış hâli (§9.2).
#[derive(Debug, Clone, PartialEq)]
pub struct DogrulanmisGovde {
    /// Ayrıştırılmış gövde; izin verilmeyen alan içermez.
    pub deger: Value,
    pub model: String,
    /// `input` ya da `documents`'in metin sayısı; sıralamada sorgu sayılmaz.
    pub metin_sayisi: u32,
    /// Metinlerin SHA-256 özeti, onaltılık.
    pub ozet: String,
}

/// Eklentinin `evren-cagir` gövdesini doğrular. İzin listesinde olmayan alan,
/// eksik ya da aralık dışı değer reddedilir (§9.2).
pub fn govde_dogrula(uc: EkUc, govde: &str) -> Result<DogrulanmisGovde, Hata> {
    if govde.len() > sinir::EVREN_GOVDESI {
        return Err(Hata::SinirAsildi(format!(
            "EVREN gövdesi en çok {} MiB olabilir",
            sinir::EVREN_GOVDESI / (1024 * 1024)
        )));
    }
    let deger: Value = serde_json::from_str(govde)
        .map_err(|e| Hata::IzinYok(format!("gövde geçerli bir JSON değil: {e}")))?;
    let Some(nesne) = deger.as_object() else {
        return Err(Hata::IzinYok("gövde bir JSON nesnesi olmalı".into()));
    };
    let izinli: &[&str] = match uc {
        EkUc::Gomme => &["model", "input", "dimensions", "encoding_format"],
        EkUc::Siralama => &["model", "query", "documents", "top_n", "return_documents"],
    };
    if let Some(alan) = nesne.keys().find(|k| !izinli.contains(&k.as_str())) {
        return Err(Hata::IzinYok(format!("`{alan}` alanına izin yok")));
    }
    let model = nesne
        .get("model")
        .and_then(Value::as_str)
        .filter(|m| !m.trim().is_empty())
        .ok_or_else(|| Hata::IzinYok("`model` zorunlu ve boş olmayan bir metin olmalı".into()))?
        .to_string();
    let (metinler, metin_sayisi) = match uc {
        EkUc::Gomme => {
            let girdi = nesne
                .get("input")
                .ok_or_else(|| Hata::IzinYok("`input` zorunlu".into()))?;
            let metinler = match girdi {
                Value::String(metin) => vec![metin.as_str()],
                Value::Array(_) => metin_dizisi(girdi, "input", 512)?,
                _ => {
                    return Err(Hata::IzinYok(
                        "`input` tek metin ya da 1–512 metinlik dizi olmalı".into(),
                    ));
                }
            };
            aralik_denetle(nesne, "dimensions", 32, 4096)?;
            match nesne.get("encoding_format") {
                None => {}
                Some(bicim) if matches!(bicim.as_str(), Some("float" | "base64")) => {}
                Some(_) => {
                    return Err(Hata::IzinYok(
                        "`encoding_format` `float` ya da `base64` olmalı".into(),
                    ));
                }
            }
            let sayi = metinler.len();
            (metinler, sayi)
        }
        EkUc::Siralama => {
            let sorgu = nesne
                .get("query")
                .and_then(Value::as_str)
                .ok_or_else(|| Hata::IzinYok("`query` zorunlu bir metin olmalı".into()))?;
            let belge_degeri = nesne
                .get("documents")
                .ok_or_else(|| Hata::IzinYok("`documents` zorunlu".into()))?;
            let belgeler = metin_dizisi(belge_degeri, "documents", 256)?;
            aralik_denetle(nesne, "top_n", 1, 256)?;
            if nesne
                .get("return_documents")
                .is_some_and(|b| !b.is_boolean())
            {
                return Err(Hata::IzinYok(
                    "`return_documents` true ya da false olmalı".into(),
                ));
            }
            let sayi = belgeler.len();
            let mut metinler = Vec::with_capacity(sayi + 1);
            metinler.push(sorgu);
            metinler.extend(belgeler);
            (metinler, sayi)
        }
    };
    let ozet = metin_ozeti(&metinler);
    Ok(DogrulanmisGovde {
        deger,
        model,
        metin_sayisi: u32::try_from(metin_sayisi).unwrap_or(u32::MAX),
        ozet,
    })
}

/// 1–`azami` öğeli bir dizi; her öğe metin olmalı.
fn metin_dizisi<'a>(deger: &'a Value, alan: &str, azami: usize) -> Result<Vec<&'a str>, Hata> {
    let dizi = deger
        .as_array()
        .filter(|d| (1..=azami).contains(&d.len()))
        .ok_or_else(|| Hata::IzinYok(format!("`{alan}` 1–{azami} metin olmalı")))?;
    dizi.iter()
        .map(|oge| {
            oge.as_str()
                .ok_or_else(|| Hata::IzinYok(format!("`{alan}` yalnızca metinlerden oluşmalı")))
        })
        .collect()
}

/// `alan` varsa `alt`–`ust` arasında bir tamsayı olmalı.
fn aralik_denetle(nesne: &Map<String, Value>, alan: &str, alt: u64, ust: u64) -> Result<(), Hata> {
    match nesne.get(alan) {
        None => Ok(()),
        Some(deger) if deger.as_u64().is_some_and(|n| (alt..=ust).contains(&n)) => Ok(()),
        Some(_) => Err(Hata::IzinYok(format!(
            "`{alan}` {alt}–{ust} arasında bir tamsayı olmalı"
        ))),
    }
}

/// Metinlerin SHA-256 özeti. Her metnin uzunluğu önüne yazılır; böylece
/// `["ab", "c"]` ile `["a", "bc"]` aynı özeti vermez.
fn metin_ozeti(metinler: &[&str]) -> String {
    let mut hash = Sha256::new();
    for metin in metinler {
        hash.update((metin.len() as u64).to_be_bytes());
        hash.update(metin.as_bytes());
    }
    hex::encode(hash.finalize())
}

/// Manifestin ucunu (`evren = [...]`) eklentinin çağırdığı uca eşler.
pub(crate) fn ek_uc(uc: EvrenUcu) -> EkUc {
    match uc {
        EvrenUcu::Embeddings => EkUc::Gomme,
        EvrenUcu::Rerank => EkUc::Siralama,
    }
}

/// EVREN hatasının eklentiye giden hâli (§9.2, §9.5).
pub(crate) fn eklenti_hatasi(hata: EvrenError) -> Hata {
    match hata {
        EvrenError::SartKabulGerekli { .. } => {
            Hata::Evren("EVREN şartları yeniden onay bekliyor".into())
        }
        EvrenError::HizSiniri { tekrar_dene } => Hata::SinirAsildi(match tekrar_dene {
            Some(sure) => format!(
                "EVREN hız sınırı; {} sn sonra yeniden deneyin",
                sure.as_secs()
            ),
            None => "EVREN hız sınırı; biraz sonra yeniden deneyin".into(),
        }),
        gunluk @ EvrenError::GunlukSinir { .. } => Hata::SinirAsildi(gunluk.to_string()),
        diger => Hata::Evren(diger.to_string()),
    }
}

/// EVREN hatasının kayda giren hâli (§9.3).
pub(crate) fn kayit_hatasi(hata: &EvrenError) -> IstekSonucu {
    let asama = match hata {
        EvrenError::Ag {
            govde_gitti: false, ..
        } => IstekAsamasi::Baglanti,
        _ => IstekAsamasi::Yanit,
    };
    IstekSonucu::Hata {
        asama,
        mesaj: hata.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn kabul(uc: EkUc, govde: &str) -> DogrulanmisGovde {
        match govde_dogrula(uc, govde) {
            Ok(d) => d,
            Err(_) => panic!("reddedildi: {govde}"),
        }
    }

    /// Gövdenin reddedilme iletisi; kabul edilirse panik.
    fn red(uc: EkUc, govde: &str) -> String {
        match govde_dogrula(uc, govde) {
            Ok(_) => panic!("kabul edildi: {govde}"),
            Err(Hata::IzinYok(ileti) | Hata::SinirAsildi(ileti)) => ileti,
            Err(_) => panic!("beklenmeyen hata türü: {govde}"),
        }
    }

    fn gomme(girdi: Value) -> String {
        json!({"model": "qwen3-embedding-8b", "input": girdi}).to_string()
    }

    #[test]
    fn gecerli_gomme_govdesi_kabul_edilir() {
        let d = kabul(
            EkUc::Gomme,
            r#"{"model":"qwen3-embedding-8b","input":["a","b"],"dimensions":1024,"encoding_format":"base64"}"#,
        );
        assert_eq!(d.model, "qwen3-embedding-8b");
        assert_eq!(d.metin_sayisi, 2);
        assert_eq!(d.deger["dimensions"], 1024);
    }

    #[test]
    fn gecerli_siralama_govdesi_kabul_edilir_sorgu_sayilmaz() {
        let d = kabul(
            EkUc::Siralama,
            r#"{"model":"qwen3-reranker-8b","query":"sorgu","documents":["a","b","c"],"top_n":2,"return_documents":true}"#,
        );
        assert_eq!(d.metin_sayisi, 3);
    }

    #[test]
    fn bilinmeyen_alan_reddedilir() {
        let ileti = red(EkUc::Gomme, r#"{"model":"m","input":"a","foo":1}"#);
        assert_eq!(ileti, "`foo` alanına izin yok");
        assert!(
            red(
                EkUc::Siralama,
                r#"{"model":"m","query":"q","documents":["a"],"input":"a"}"#
            )
            .contains("`input` alanına izin yok")
        );
    }

    #[test]
    fn model_zorunlu_ve_metin() {
        assert!(red(EkUc::Gomme, r#"{"input":"a"}"#).contains("`model`"));
        assert!(red(EkUc::Gomme, r#"{"model":"  ","input":"a"}"#).contains("`model`"));
        assert!(red(EkUc::Gomme, r#"{"model":5,"input":"a"}"#).contains("`model`"));
    }

    #[test]
    fn gomme_input_siniri() {
        assert_eq!(
            red(EkUc::Gomme, &gomme(json!([]))),
            "`input` 1–512 metin olmalı"
        );
        assert_eq!(
            red(EkUc::Gomme, &gomme(json!(vec!["a"; 513]))),
            "`input` 1–512 metin olmalı"
        );
        assert_eq!(
            kabul(EkUc::Gomme, &gomme(json!(vec!["a"; 512]))).metin_sayisi,
            512
        );
    }

    #[test]
    fn gomme_input_metin_olmayan_oge_ve_eksik() {
        assert!(red(EkUc::Gomme, &gomme(json!(["a", 1]))).contains("yalnızca metinlerden"));
        assert!(red(EkUc::Gomme, r#"{"model":"m"}"#).contains("`input` zorunlu"));
        assert!(red(EkUc::Gomme, &gomme(json!(5))).contains("tek metin ya da 1–512"));
    }

    #[test]
    fn dimensions_siniri() {
        for n in [31, 4097] {
            assert!(
                red(
                    EkUc::Gomme,
                    &json!({"model": "m", "input": "a", "dimensions": n}).to_string()
                )
                .contains("`dimensions` 32–4096")
            );
        }
        for n in [32, 4096] {
            kabul(
                EkUc::Gomme,
                &json!({"model": "m", "input": "a", "dimensions": n}).to_string(),
            );
        }
    }

    #[test]
    fn encoding_format_yalnizca_float_ya_da_base64() {
        assert!(
            red(
                EkUc::Gomme,
                r#"{"model":"m","input":"a","encoding_format":"hex"}"#
            )
            .contains("`encoding_format`")
        );
    }

    #[test]
    fn documents_top_n_ve_return_documents_siniri() {
        let belgeler =
            |n: usize| json!({"model": "m", "query": "q", "documents": vec!["a"; n]}).to_string();
        assert!(red(EkUc::Siralama, &belgeler(0)).contains("`documents` 1–256 metin olmalı"));
        assert!(red(EkUc::Siralama, &belgeler(257)).contains("`documents` 1–256 metin olmalı"));
        kabul(EkUc::Siralama, &belgeler(256));
        for n in [0, 257] {
            let govde =
                json!({"model": "m", "query": "q", "documents": ["a"], "top_n": n}).to_string();
            assert!(red(EkUc::Siralama, &govde).contains("`top_n` 1–256"));
        }
        assert!(
            red(
                EkUc::Siralama,
                r#"{"model":"m","query":"q","documents":["a"],"return_documents":"evet"}"#
            )
            .contains("`return_documents`")
        );
    }

    #[test]
    fn govde_nesne_ve_boyut_siniri() {
        assert!(red(EkUc::Gomme, "[1,2]").contains("bir JSON nesnesi"));
        assert!(red(EkUc::Gomme, "{").contains("geçerli bir JSON değil"));
        let buyuk = format!("\"{}\"", "x".repeat(sinir::EVREN_GOVDESI));
        match govde_dogrula(EkUc::Gomme, &buyuk) {
            Err(Hata::SinirAsildi(ileti)) => assert!(ileti.contains("4 MiB")),
            _ => panic!("4 MiB üstü gövde kabul edildi"),
        }
    }

    #[test]
    fn ozet_belirlenimli_ve_birlesmeye_karsi_korumali() {
        let a = kabul(EkUc::Gomme, &gomme(json!(["ab", "c"])));
        assert_eq!(a.ozet, kabul(EkUc::Gomme, &gomme(json!(["ab", "c"]))).ozet);
        assert_ne!(a.ozet, kabul(EkUc::Gomme, &gomme(json!(["a", "bc"]))).ozet);
        assert_eq!(a.ozet.len(), 64);
    }

    #[test]
    fn siralama_ozeti_once_sorgu_sonra_belgeler() {
        let govde = r#"{"model":"m","query":"q","documents":["d"]}"#;
        let mut beklenen = Sha256::new();
        for metin in ["q", "d"] {
            beklenen.update((metin.len() as u64).to_be_bytes());
            beklenen.update(metin.as_bytes());
        }
        assert_eq!(
            kabul(EkUc::Siralama, govde).ozet,
            hex::encode(beklenen.finalize())
        );
        let ters = r#"{"model":"m","query":"d","documents":["q"]}"#;
        assert_ne!(
            kabul(EkUc::Siralama, govde).ozet,
            kabul(EkUc::Siralama, ters).ozet
        );
    }
}
