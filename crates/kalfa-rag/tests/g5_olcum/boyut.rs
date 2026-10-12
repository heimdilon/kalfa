//! 1024/4096 kontrollü kıyası. Eski 1024 vektörün yönü, yeni 4096'nın
//! ilk 1024 bileşeniyle her parçada doğrulanmadan kalite ölçülmez.
use super::*;

const BUYUK_BOYUT: usize = 4096;
const TOPLU: usize = 64;
const BAYT_SINIRI: usize = 1024 * 1024;
const BUTCE: u64 = 8_000_000;
const ON_EK_ESIGI: f64 = 0.9999;
const BOYUTLAR: [usize; 2] = [BOYUT, BUYUK_BOYUT];

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
enum Tur {
    Gomme { boyut: usize, adet: usize },
    Siralama { adet: usize },
}

#[derive(Deserialize, Serialize)]
struct BoyutMakbuzu {
    istek_ozeti: String,
    tur: Tur,
    ms: f64,
    token: u64,
    model: Option<String>,
    yanit_bayt: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    vektorler: Vec<Vec<f32>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    siralama: Vec<(usize, f32)>,
}

fn norm(v: &[f32], boyut: usize) -> Result<f64, String> {
    if v.len() != boyut || v.iter().any(|x| !x.is_finite()) {
        return Err("boyut vektörü eksik veya sonlu değil".into());
    }
    let n = v.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>().sqrt();
    if !n.is_finite() || n <= 0.0 {
        return Err("boyut vektörü sıfır veya taşmış".into());
    }
    Ok(n)
}

fn birim(v: &[f32], boyut: usize) -> Result<Vec<f32>, String> {
    let n = norm(v, boyut)?;
    Ok(v.iter().map(|x| (f64::from(*x) / n) as f32).collect())
}

fn on_ek_kosinusu(kucuk: &[f32], buyuk: &[f32]) -> Result<f64, String> {
    norm(buyuk, BUYUK_BOYUT)?;
    let on_ek = &buyuk[..BOYUT];
    let n = norm(kucuk, BOYUT)? * norm(on_ek, BOYUT)?;
    let c = kucuk
        .iter()
        .zip(on_ek)
        .map(|(a, b)| f64::from(*a) * f64::from(*b))
        .sum::<f64>()
        / n;
    if c < ON_EK_ESIGI {
        return Err(format!(
            "1024/4096 ön ek kosinüsü {c:.8}; boyut kıyası durdu"
        ));
    }
    Ok(c.min(1.0))
}

fn boyutlu_sira(
    parcalar: &[Parca],
    vektorler: &[Vec<f32>],
    q: &[f32],
    boyut: usize,
) -> Result<Vec<usize>, String> {
    let q = birim(q, boyut)?;
    if vektorler.len() != parcalar.len() || vektorler.iter().any(|v| v.len() != boyut) {
        return Err("parça ve boyut vektörlerinin sayısı farklı".into());
    }
    let mut puanlar: Vec<_> = vektorler
        .iter()
        .enumerate()
        .map(|(i, v)| (i, q.iter().zip(v).map(|(x, y)| x * y).sum::<f32>()))
        .collect();
    if puanlar.iter().any(|(_, p)| !p.is_finite()) {
        return Err("boyut sıralama puanı sonlu değil".into());
    }
    puanlar.sort_by(|a, b| {
        b.1.total_cmp(&a.1)
            .then_with(|| parcalar[a.0].yol.cmp(&parcalar[b.0].yol))
            .then(parcalar[a.0].sira.cmp(&parcalar[b.0].sira))
    });
    Ok(puanlar.into_iter().map(|(i, _)| i).collect())
}

fn toplular(parcalar: &[Parca]) -> Result<Vec<(usize, usize)>, String> {
    let mut gruplar = Vec::new();
    let mut bas = 0;
    while bas < parcalar.len() {
        let mut son = (bas + TOPLU).min(parcalar.len());
        loop {
            let metinler: Vec<_> = parcalar[bas..son].iter().map(|p| p.metin.clone()).collect();
            if gomme_govdesi(GOMME, &metinler, BUYUK_BOYUT as u32).len() <= BAYT_SINIRI {
                break;
            }
            son -= 1;
            if son == bas {
                return Err("tek boyut girdisi 1 MiB gövde sınırını aşıyor".into());
            }
        }
        gruplar.push((bas, son));
        bas = son;
    }
    Ok(gruplar)
}

fn tarif(
    girdi_ozeti: &str,
    sorgular: &[Sorgu],
    gruplar: &[(usize, usize)],
) -> Result<Value, String> {
    Ok(json!({"deney":"boyut","girdi_ozeti":girdi_ozeti,
        "sorgu_ozeti":ozet(&serde_json::to_vec(sorgular).map_err(|_| "sorgular kodlanamadı")?),
        "boyutlar":BOYUTLAR,"gomme_modeli":GOMME,"siralama_modeli":SIRALAMA,
        "talimat":TALIMAT,"aday":50,"top_n":50,"toplu":TOPLU,"gruplar":gruplar,
        "istek_bayt_siniri":BAYT_SINIRI,"token_siniri":BUTCE,
        "on_ek_kosinus_esigi":ON_EK_ESIGI,"kaynak1024":"mevcut-dogrulanmis-indeks",
        "gomme_yonlendirmesi":"qwen3-embedding-8b","siralama_yonlendirmesi":"qwen3-vl-reranker-8b"}))
}

fn makbuzu_dogrula(m: &BoyutMakbuzu) -> Result<(), String> {
    if m.token == 0 || !m.ms.is_finite() || m.ms < 0.0 || m.yanit_bayt == 0 {
        return Err("boyut makbuzu süre/token/yanıt alanı geçersiz".into());
    }
    match m.tur {
        Tur::Gomme { boyut, adet } => {
            if !BOYUTLAR.contains(&boyut)
                || adet == 0
                || adet > TOPLU
                || m.vektorler.len() != adet
                || !m.siralama.is_empty()
            {
                return Err("boyut makbuzunda gömme sayısı/boyutu geçersiz".into());
            }
            for v in &m.vektorler {
                norm(v, boyut)?;
            }
            if !matches!(
                m.model.as_deref(),
                Some("qwen/qwen3-embedding-8b" | "qwen3-embedding-8b")
            ) {
                return Err("boyut deneyinde gömme yönlendirmesi değişti veya eksik".into());
            }
        }
        Tur::Siralama { adet } => {
            let mut gorulen = BTreeSet::new();
            if adet != 50
                || !m.vektorler.is_empty()
                || m.siralama.len() != adet
                || m.siralama
                    .iter()
                    .any(|(i, p)| *i >= adet || !p.is_finite() || !gorulen.insert(*i))
            {
                return Err("boyut makbuzunda sıralama eksik/tekrarlı/geçersiz".into());
            }
            if !matches!(
                m.model.as_deref(),
                Some("qwen/qwen3-vl-reranker-8b" | "qwen3-vl-reranker-8b")
            ) {
                return Err("boyut deneyinde sıralama yönlendirmesi değişti veya eksik".into());
            }
        }
    }
    Ok(())
}

fn eski_token(depo: &Path) -> Result<u64, String> {
    let mut toplam = 0u64;
    for g in std::fs::read_dir(depo).map_err(|_| "boyut makbuz dizini okunamadı")? {
        let yol = g.map_err(|_| "boyut makbuz yolu okunamadı")?.path();
        if yol.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let m: BoyutMakbuzu =
            serde_json::from_slice(&std::fs::read(&yol).map_err(|_| "boyut makbuzu okunamadı")?)
                .map_err(|_| "boyut makbuzu bozuk")?;
        if yol.file_stem().and_then(|s| s.to_str()) != Some(m.istek_ozeti.as_str()) {
            return Err("boyut makbuzu gövde özeti farklı".into());
        }
        makbuzu_dogrula(&m)?;
        toplam = toplam
            .checked_add(m.token)
            .ok_or("boyut token bütçesi taştı")?;
    }
    if toplam > BUTCE {
        return Err("önceki boyut koşuları 8 milyon token sınırını aştı".into());
    }
    Ok(toplam)
}

fn butce_uygun(harcanan: u64, bayt: usize) -> bool {
    u64::try_from(bayt)
        .ok()
        .and_then(|n| n.checked_mul(2))
        .and_then(|n| n.checked_add(4096))
        .and_then(|n| n.checked_add(harcanan))
        .is_some_and(|n| n <= BUTCE)
}

async fn boyut_istegi(
    istemci: &EvrenIstemcisi,
    depo: &Path,
    tur: Tur,
    govde: String,
    harcanan: &mut u64,
) -> Result<(BoyutMakbuzu, bool), String> {
    if govde.len() > BAYT_SINIRI {
        return Err("boyut isteği 1 MiB sınırını aşıyor".into());
    }
    let istek_ozeti = ozet(govde.as_bytes());
    let yol = depo.join(format!("{istek_ozeti}.json"));
    if yol.exists() {
        let m: BoyutMakbuzu =
            serde_json::from_slice(&std::fs::read(&yol).map_err(|_| "boyut makbuzu okunamadı")?)
                .map_err(|_| "boyut makbuzu bozuk")?;
        if m.istek_ozeti != istek_ozeti || m.tur != tur {
            return Err("boyut makbuzu istekten farklı".into());
        }
        makbuzu_dogrula(&m)?;
        return Ok((m, true));
    }
    if !butce_uygun(*harcanan, govde.len()) {
        return Err("8 milyon boyut token bütçesi; yeni çağrı başlatılmadı".into());
    }
    let govde: Value = serde_json::from_str(&govde).map_err(|_| "boyut isteği JSON değil")?;
    let uc = match tur {
        Tur::Gomme { .. } => EkUc::Gomme,
        Tur::Siralama { .. } => EkUc::Siralama,
    };
    tokio::time::sleep(Duration::from_secs(2)).await;
    let bas = Instant::now();
    let yanit = istemci.ek_uc(uc, &govde).await.map_err(guvenli_hata)?;
    let ms = bas.elapsed().as_secs_f64() * 1000.0;
    let kok: Value = serde_json::from_str(&yanit.govde).map_err(|_| "boyut yanıtı JSON değil")?;
    let token = kok["usage"]["prompt_tokens"]
        .as_u64()
        .ok_or("boyut kullanımı eksik; ölçüm durdu")?;
    *harcanan = harcanan
        .checked_add(token)
        .ok_or("boyut token toplamı taştı")?;
    let (vektorler, siralama) = match tur {
        Tur::Gomme { boyut, adet } => (
            gomme_yanitini_coz(&yanit.govde, adet, boyut)
                .map_err(|_| "boyut gömme yanıtı geçersiz")?,
            Vec::new(),
        ),
        Tur::Siralama { adet } => (
            Vec::new(),
            siralama_yanitini_coz(&yanit.govde, adet)
                .map_err(|_| "boyut sıralama yanıtı geçersiz")?,
        ),
    };
    let m = BoyutMakbuzu {
        istek_ozeti,
        tur,
        ms,
        token,
        model: yanit.meta.yonlendirilen_model.or(yanit.model),
        yanit_bayt: yanit.govde.len(),
        vektorler,
        siralama,
    };
    makbuzu_dogrula(&m)?;
    let mut gecici =
        tempfile::NamedTempFile::new_in(depo).map_err(|_| "boyut makbuz dosyası açılamadı")?;
    serde_json::to_writer(gecici.as_file_mut(), &m).map_err(|_| "boyut makbuzu yazılamadı")?;
    gecici
        .as_file()
        .sync_all()
        .map_err(|_| "boyut makbuzu diske yazılamadı")?;
    gecici
        .persist_noclobber(yol)
        .map_err(|_| "boyut makbuzu kaydedilemedi")?;
    if *harcanan > BUTCE {
        return Err("boyut yanıtının raporlanan kullanımı bütçeyi aştı; ölçüm durdu".into());
    }
    Ok((m, false))
}

#[tokio::test]
#[ignore = "G5 boyut kıyası; ayrıca açık onay, EVREN_API_KEY ve mevcut indeks gerekir"]
async fn canli_boyut_karsilastirmasi() -> Result<(), String> {
    let kok = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|_| "depo kökü açılamadı")?;
    let (parcalar, mut bilgi) = girdiler(&kok)?;
    let sorgular: Vec<Sorgu> = serde_json::from_str(include_str!("../degerlendirme.json"))
        .map_err(|_| "sorgular geçersiz")?;
    if sorgular.len() != 32 || parcalar.len() < 50 {
        return Err("32 sorgu ve en az 50 parça gerekli".into());
    }
    let gruplar = toplular(&parcalar)?;
    let tarif = tarif(
        bilgi["girdi_ozeti"].as_str().ok_or("girdi özeti yok")?,
        &sorgular,
        &gruplar,
    )?;
    let deney_ozeti = ozet(
        serde_json::to_string(&tarif)
            .map_err(|_| "boyut tarifi kodlanamadı")?
            .as_bytes(),
    );
    bilgi["kipler"] = json!(BOYUTLAR);
    bilgi["token_siniri"] = json!(BUTCE);
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
        json!({"asama":"kapsam","sorgu":32,"boyutlar":BOYUTLAR,"aday":50,
        "indeks_gomme_istek":gruplar.len(),"sorgu_gomme_istek":64,"siralama_istek":64,
        "beklenen_dosyasi_olmayan_sorgular":eksik,
        "vektor_bayt1024":parcalar.len()*BOYUT*4,"vektor_bayt4096":parcalar.len()*BUYUK_BOYUT*4})
    );
    if !eksik.is_empty() {
        return Err("beklenen dosyası bulunmayan boyut sorgusu var".into());
    }
    if std::env::var("KALFA_G5_KURU").as_deref() == Ok("1") {
        return Ok(());
    }
    let onay = std::env::var("KALFA_G5_BOYUT_DENEY_OZETI")
        .map_err(|_| "onaylı boyut deneyi özeti gerekli")?;
    if onay != deney_ozeti {
        return Err("boyut deneyi onaylanandan farklı".into());
    }
    let anahtar = std::env::var("EVREN_API_KEY").map_err(|_| "EVREN_API_KEY tanımlı değil")?;
    let taban = std::env::var("EVREN_BASE_URL").unwrap_or_else(|_| HEDEF.into());
    if taban.trim_end_matches('/') != HEDEF {
        return Err("EVREN hedefi onaylanan adres değil".into());
    }
    let istemci = EvrenIstemcisi::yeni(taban, ApiAnahtari::yeni(anahtar)).map_err(guvenli_hata)?;
    let depo = kok.join("target/kalfa-g5-boyut").join(&onay);
    std::fs::create_dir_all(&depo).map_err(|_| "boyut dizini açılamadı")?;
    let onceki = eski_token(&depo)?;
    let mut harcanan = onceki;
    let mut buyukler = Vec::with_capacity(parcalar.len());
    let kucukler: Vec<_> = parcalar
        .iter()
        .map(|p| birim(&p.vektor, BOYUT))
        .collect::<Result<_, _>>()?;
    let mut enaz = 1.0f64;
    for (bas, son) in gruplar {
        let metinler: Vec<_> = parcalar[bas..son].iter().map(|p| p.metin.clone()).collect();
        let (m, eski) = boyut_istegi(
            &istemci,
            &depo,
            Tur::Gomme {
                boyut: BUYUK_BOYUT,
                adet: metinler.len(),
            },
            gomme_govdesi(GOMME, &metinler, BUYUK_BOYUT as u32),
            &mut harcanan,
        )
        .await?;
        let mut grup_enaz = 1.0f64;
        for (i, v) in m.vektorler.iter().enumerate() {
            let c = on_ek_kosinusu(&kucukler[bas + i], v)
                .map_err(|e| format!("parça {}: {e}", bas + i))?;
            grup_enaz = grup_enaz.min(c);
            buyukler.push(birim(v, BUYUK_BOYUT)?);
        }
        enaz = enaz.min(grup_enaz);
        println!(
            "{}",
            json!({"asama":"indeks4096","bas":bas,"adet":son-bas,"token":m.token,"ms":m.ms,
            "model":m.model,"yanit_bayt":m.yanit_bayt,"onceki":eski,"istek_ozeti":m.istek_ozeti,
            "on_ek_kosinus_enaz":grup_enaz})
        );
    }
    println!(
        "{}",
        json!({"asama":"on_ek_dogrulama","parca":buyukler.len(),"kosinus_enaz":enaz,"esik":ON_EK_ESIGI})
    );
    for (i, s) in sorgular.iter().enumerate() {
        let mut qlar = Vec::new();
        for j in 0..2 {
            let boyut = BOYUTLAR[(i + j) % 2];
            let (m, eski) = boyut_istegi(
                &istemci,
                &depo,
                Tur::Gomme { boyut, adet: 1 },
                gomme_govdesi(GOMME, &[format!("{TALIMAT}{}", s.sorgu)], boyut as u32),
                &mut harcanan,
            )
            .await?;
            println!(
                "{}",
                json!({"asama":"sorgu","sorgu_no":i,"boyut":boyut,"token":m.token,"ms":m.ms,
                "model":m.model,"yanit_bayt":m.yanit_bayt,"onceki":eski,"istek_ozeti":m.istek_ozeti})
            );
            qlar.push((boyut, m));
        }
        let q1024 = qlar
            .iter()
            .find(|(b, _)| *b == BOYUT)
            .ok_or("1024 sorgusu yok")?;
        let q4096 = qlar
            .iter()
            .find(|(b, _)| *b == BUYUK_BOYUT)
            .ok_or("4096 sorgusu yok")?;
        let qcos = on_ek_kosinusu(&q1024.1.vektorler[0], &q4096.1.vektorler[0])?;
        for j in 0..2 {
            let boyut = BOYUTLAR[(i + j) % 2];
            let v = if boyut == BOYUT { &kucukler } else { &buyukler };
            let q = if boyut == BOYUT {
                &q1024.1.vektorler[0]
            } else {
                &q4096.1.vektorler[0]
            };
            let bas = Instant::now();
            let sira = boyutlu_sira(&parcalar, v, q, boyut)?;
            let kosinus_ms = bas.elapsed().as_secs_f64() * 1000.0;
            let adaylar = adaylari_sec(&parcalar, &sira, 50, 50);
            let belgeler: Vec<_> = adaylar.iter().map(|n| parcalar[*n].metin.clone()).collect();
            let (m, eski) = boyut_istegi(
                &istemci,
                &depo,
                Tur::Siralama { adet: 50 },
                siralama_govdesi(SIRALAMA, &s.sorgu, &belgeler, 50),
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
                json!({"asama":"siralama","sorgu_no":i,"sorgu":s.sorgu,"beklenen":s.beklenen,
                "boyut":boyut,"aday":adaylar.len(),"adaylar":kayitlar,
                "aday_tekil_dosya":adaylar.iter().map(|n|parcalar[*n].yol.as_str()).collect::<BTreeSet<_>>().len(),
                "aday_isabet":adaylar.iter().any(|n|isabet(&parcalar[*n].yol,&s.beklenen)),
                "token":m.token,"ms":m.ms,"model":m.model,"yanit_bayt":m.yanit_bayt,"onceki":eski,"istek_ozeti":m.istek_ozeti,
                "kosinus_ms":kosinus_ms,"sorgu_on_ek_kosinus":qcos,
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

#[test]
fn on_ek_yonu_buyuk_boyutu_sifiri_ve_sonlu_olmayan_degeri_dogrular() {
    let mut k = vec![0.0; BOYUT];
    k[0] = 3.0;
    let mut b = vec![0.0; BUYUK_BOYUT];
    b[0] = 6.0;
    b[BUYUK_BOYUT - 1] = 100.0;
    assert!((on_ek_kosinusu(&k, &b).unwrap() - 1.0).abs() < 1e-12);
    b[1] = 6.0;
    assert!(on_ek_kosinusu(&k, &b).is_err());
    assert!(on_ek_kosinusu(&k, &b[..BOYUT]).is_err());
    b[0] = f32::NAN;
    assert!(on_ek_kosinusu(&k, &b).is_err());
    assert!(on_ek_kosinusu(&vec![0.0; BOYUT], &vec![0.0; BUYUK_BOYUT]).is_err());
}

#[test]
fn boyutlu_kosinus_tam_boyutu_ve_esit_puanda_yol_sirasini_kullanir() {
    let p = [sahte("b", 0), sahte("a", 0)];
    let v = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
    assert_eq!(boyutlu_sira(&p, &v, &[1.0, 1.0], 2).unwrap(), [1, 0]);
    assert_eq!(boyutlu_sira(&p, &v, &[2.0, 1.0], 2).unwrap(), [0, 1]);
    assert!(boyutlu_sira(&p, &v, &[1.0], 2).is_err());
    assert!(boyutlu_sira(&p, &v[..1], &[1.0, 1.0], 2).is_err());
    assert!(boyutlu_sira(&p, &[vec![f32::NAN, 0.0], vec![1.0, 0.0]], &[1.0, 1.0], 2).is_err());
}

#[test]
fn toplu_sinir_adet_ve_json_kacislarini_dahil_eder() {
    let p: Vec<_> = (0..130).map(|i| sahte("a", i)).collect();
    assert_eq!(toplular(&p).unwrap(), [(0, 64), (64, 128), (128, 130)]);
    let mut p = vec![sahte("a", 0), sahte("a", 1)];
    for a in &mut p {
        a.metin = "\"".repeat(300_000);
    }
    assert_eq!(toplular(&p).unwrap(), [(0, 1), (1, 2)]);
    p[0].metin = "\"".repeat(BAYT_SINIRI);
    assert!(toplular(&p).is_err());
}

#[test]
fn boyut_makbuzu_yonlendirme_sayisi_sira_ve_devam_butcesini_dogrular() {
    let mut v = vec![0.0; BUYUK_BOYUT];
    v[0] = 1.0;
    let mut m = BoyutMakbuzu {
        istek_ozeti: "a".into(),
        tur: Tur::Gomme {
            boyut: BUYUK_BOYUT,
            adet: 1,
        },
        ms: 1.0,
        token: 10,
        model: Some("qwen/qwen3-embedding-8b".into()),
        yanit_bayt: 12,
        vektorler: vec![v],
        siralama: vec![],
    };
    assert!(makbuzu_dogrula(&m).is_ok());
    m.model = None;
    assert!(makbuzu_dogrula(&m).is_err());
    m.model = Some("qwen/qwen3-embedding-8b".into());
    m.tur = Tur::Gomme {
        boyut: BOYUT,
        adet: 1,
    };
    assert!(makbuzu_dogrula(&m).is_err());
    m.tur = Tur::Siralama { adet: 50 };
    m.vektorler.clear();
    m.siralama = (0..50).map(|i| (i, 1.0)).collect();
    m.model = Some("qwen/qwen3-vl-reranker-8b".into());
    assert!(makbuzu_dogrula(&m).is_ok());
    m.siralama[49].0 = 0;
    assert!(makbuzu_dogrula(&m).is_err());
    m.siralama[49].0 = 49;
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("a.json"), serde_json::to_vec(&m).unwrap()).unwrap();
    assert_eq!(eski_token(d.path()).unwrap(), 10);
    m.token = BUTCE + 1;
    std::fs::write(d.path().join("a.json"), serde_json::to_vec(&m).unwrap()).unwrap();
    assert!(eski_token(d.path()).is_err());
    assert!(!butce_uygun(u64::MAX, 100));
    assert!(!butce_uygun(0, usize::MAX));
    assert!(butce_uygun(BUTCE - 4296, 100));
    assert!(!butce_uygun(BUTCE - 4295, 100));
}

#[test]
fn boyut_onayi_kaynak_sorgu_grup_ve_esigi_baglar() {
    let mut s = vec![Sorgu {
        sorgu: "sorgu".into(),
        beklenen: vec!["a".into()],
    }];
    let t = tarif("kaynak-a", &s, &[(0, 64)]).unwrap();
    assert_ne!(t, tarif("kaynak-b", &s, &[(0, 64)]).unwrap());
    assert_ne!(t, tarif("kaynak-a", &s, &[(0, 32), (32, 64)]).unwrap());
    s[0].beklenen[0] = "b".into();
    assert_ne!(t, tarif("kaynak-a", &s, &[(0, 64)]).unwrap());
    assert_eq!(t["boyutlar"], json!([1024, 4096]));
    assert_eq!(t["on_ek_kosinus_esigi"], ON_EK_ESIGI);
    assert_eq!(t["token_siniri"], BUTCE);
}
