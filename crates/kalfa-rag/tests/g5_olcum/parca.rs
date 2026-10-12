//! G5 parça penceresi kıyası; tam doğrulanmış aynı dosyaları üç kez parçalar.
//! Ürün parçalama kodu ve mevcut indeks değişmez. Kaynak metni kayda girmez.
use super::*;

#[path = "../../src/parca.rs"]
mod urun;

const BUTCE: u64 = 16_000_000;
const TOPLU: usize = 64;
const BAYT_SINIRI: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
struct Pencere {
    ad: &'static str,
    satir: usize,
    karakter: usize,
    ortusme: usize,
}

const PENCERELER: [Pencere; 3] = [
    Pencere {
        ad: "kucuk",
        satir: 30,
        karakter: 1200,
        ortusme: 5,
    },
    Pencere {
        ad: "mevcut",
        satir: 60,
        karakter: 2400,
        ortusme: 10,
    },
    Pencere {
        ad: "buyuk",
        satir: 120,
        karakter: 4800,
        ortusme: 20,
    },
];

struct DosyaGirdisi {
    yol: String,
    metin: String,
}

struct Kume {
    pencere: Pencere,
    parcalar: Vec<Parca>,
    gruplar: Vec<(usize, usize)>,
    ozet: String,
}

/// Ürünün satır/karakter/örtüşme algoritmasının parametreli test kopyası.
/// Varsayılan kopya her gerçek dosyada ürün çıktısıyla birebir karşılaştırılır.
fn parcala(metin: &str, p: Pencere) -> Result<Vec<urun::Parca>, String> {
    if p.satir == 0 || p.karakter == 0 || p.ortusme >= p.satir {
        return Err("parça penceresi sınırları geçersiz".into());
    }
    let mut satirlar = Vec::new();
    let mut bayt = 0;
    for (i, s) in metin.split_inclusive('\n').enumerate() {
        satirlar.push((
            u32::try_from(i + 1).map_err(|_| "satır sayısı taştı")?,
            bayt,
            bayt + s.len(),
            s.chars().count(),
        ));
        bayt += s.len();
    }
    let mut cikti = Vec::new();
    let mut ekle = |sb, ss, bas, son| -> Result<(), String> {
        let m = metin.get(bas..son).ok_or("parça UTF-8 aralığı geçersiz")?;
        if !m.trim().is_empty() {
            cikti.push(urun::Parca {
                satir_bas: sb,
                satir_son: ss,
                bas_bayt: bas,
                son_bayt: son,
                metin: m.into(),
            });
        }
        Ok(())
    };
    let mut bas = 0;
    while let Some(&(no, sb, ss, n)) = satirlar.get(bas) {
        if n > p.karakter {
            let s = metin.get(sb..ss).ok_or("uzun satır aralığı geçersiz")?;
            let sinirlar: Vec<_> = s
                .char_indices()
                .map(|(i, _)| i)
                .chain(std::iter::once(s.len()))
                .collect();
            for i in (0..n).step_by(p.karakter) {
                ekle(
                    no,
                    no,
                    sb + sinirlar[i],
                    sb + sinirlar[(i + p.karakter).min(n)],
                )?;
            }
            bas += 1;
            continue;
        }
        let mut son = bas;
        let mut karakter = 0;
        while let Some(s) = satirlar.get(son) {
            if s.3 > p.karakter || son - bas == p.satir || karakter + s.3 > p.karakter {
                break;
            }
            karakter += s.3;
            son += 1;
        }
        let sonuncu = satirlar.get(son - 1).ok_or("parça penceresi ilerlemedi")?;
        ekle(no, sonuncu.0, sb, sonuncu.2)?;
        let Some(s) = satirlar.get(son) else {
            break;
        };
        bas = if s.3 > p.karakter {
            son
        } else {
            son.saturating_sub(p.ortusme.min((son - bas) / 2))
                .max(bas + 1)
        };
    }
    Ok(cikti)
}

fn tam_eslesir(metin: &str, eski: &[Kaynak]) -> bool {
    let yeni = urun::parcala(metin);
    yeni.len() == eski.len()
        && yeni.iter().zip(eski).all(|(p, k)| {
            p.bas_bayt == k.bas_bayt
                && p.son_bayt == k.son_bayt
                && p.satir_bas == k.satir_bas
                && p.satir_son == k.satir_son
                && ozet(p.metin.as_bytes()) == k.ozet
        })
}

fn dosyalar(kok: &Path) -> Result<(Vec<DosyaGirdisi>, Value), String> {
    let bayt = std::fs::read(
        kok.join("target/kalfa-rag-degerlendirme")
            .join(ozet(b"meta")),
    )
    .map_err(|_| "mevcut meta okunamadı")?;
    let meta: Meta = serde_json::from_slice(&bayt).map_err(|_| "meta geçersiz")?;
    if meta.surum != 1 || meta.boyut != BOYUT || meta.model != GOMME {
        return Err("indeks sürümü/modeli/boyutu farklı".into());
    }
    let sinif = Arc::new(Siniflandirici::yeni(kok, &[]).map_err(|_| "sınıflandırıcı açılamadı")?);
    let alan = CalismaAlani::ac(sinif.clone()).map_err(|_| "çalışma alanı açılamadı")?;
    let manifest =
        Manifest::coz(include_str!("../../kalfa.toml")).map_err(|_| "manifest geçersiz")?;
    let korumali = kalfa_core::git_koruma::korumali_yollar(kok);
    let liste: BTreeSet<_> = alan
        .dosyalar(&tokio_util::sync::CancellationToken::new())
        .map_err(|_| "kaynak listesi okunamadı")?
        .into_iter()
        .map(|y| y.as_str().to_string())
        .collect();
    let mut dosyalar = Vec::new();
    let mut kayit = Vec::new();
    for (yol, d) in &meta.dosyalar {
        let y = CalismaYolu::coz(yol).map_err(|_| "meta yolu geçersiz")?;
        if !deney_yolu(yol)
            || !liste.contains(yol)
            || !manifest.okuyabilir(yol)
            || sinif.gizli_mi(&y)
            || korumali_listede(&y, &korumali)
        {
            continue;
        }
        let Ok(b) = alan.sinirli_oku(&y, &tokio_util::sync::CancellationToken::new()) else {
            continue;
        };
        let Ok(metin) = String::from_utf8(b) else {
            continue;
        };
        // Tek tek sağlam parçalar yeterli değildir: yeni/eksik/değişmiş tüm
        // dosyalar elenir, üç pencerenin kaynak kapsaması birebir aynıdır.
        if !tam_eslesir(&metin, &d.parcalar) || d.parcalar.is_empty() {
            continue;
        }
        if parcala(&metin, PENCERELER[1])? != urun::parcala(&metin) {
            return Err("mevcut pencere ürün algoritmasından farklı".into());
        }
        kayit.push(json!({"yol":yol,"bayt":metin.len(),"ozet":ozet(metin.as_bytes())}));
        dosyalar.push(DosyaGirdisi {
            yol: yol.clone(),
            metin,
        });
    }
    if dosyalar.is_empty() {
        return Err("tam doğrulanmış dosya yok".into());
    }
    let bilgi = json!({"asama":"girdiler","meta_ozeti":ozet(&bayt),"indeks_dosya":meta.dosyalar.len(),
        "dosya":dosyalar.len(),"atlanan_dosya":meta.dosyalar.len()-dosyalar.len(),
        "kaynak_bayt":dosyalar.iter().map(|d|d.metin.len()).sum::<usize>(),
        "dosyalar":kayit,"girdi_ozeti":ozet(&serde_json::to_vec(&kayit).map_err(|_|"dosya kayıtları kodlanamadı")?)});
    Ok((dosyalar, bilgi))
}

fn gruplar(parcalar: &[Parca]) -> Result<Vec<(usize, usize)>, String> {
    let mut sonuc = Vec::new();
    let mut bas = 0;
    while bas < parcalar.len() {
        let mut son = (bas + TOPLU).min(parcalar.len());
        while gomme_govdesi(
            GOMME,
            &parcalar[bas..son]
                .iter()
                .map(|p| p.metin.clone())
                .collect::<Vec<_>>(),
            BOYUT as u32,
        )
        .len()
            > BAYT_SINIRI
        {
            son -= 1;
            if son == bas {
                return Err("tek parça 1 MiB isteği aşıyor".into());
            }
        }
        sonuc.push((bas, son));
        bas = son;
    }
    Ok(sonuc)
}

fn kumeler(dosyalar: &[DosyaGirdisi]) -> Result<Vec<Kume>, String> {
    PENCERELER
        .iter()
        .map(|&pencere| {
            let mut parcalar = Vec::new();
            let mut kayit = Vec::new();
            for d in dosyalar {
                for (sira, p) in parcala(&d.metin, pencere)?.into_iter().enumerate() {
                    let kaynak = Kaynak {
                        satir_bas: p.satir_bas,
                        satir_son: p.satir_son,
                        bas_bayt: p.bas_bayt,
                        son_bayt: p.son_bayt,
                        ozet: ozet(p.metin.as_bytes()),
                        dilim: 0,
                        sira,
                    };
                    kayit.push(json!({"yol":d.yol,"kaynak":kaynak}));
                    parcalar.push(Parca {
                        yol: d.yol.clone(),
                        sira,
                        kaynak,
                        metin: p.metin,
                        vektor: Vec::new(),
                    });
                }
            }
            let gruplar = gruplar(&parcalar)?;
            Ok(Kume {
                pencere,
                parcalar,
                gruplar,
                ozet: ozet(&serde_json::to_vec(&kayit).map_err(|_| "parça kayıtları kodlanamadı")?),
            })
        })
        .collect()
}

fn tarif(bilgi: &Value, kumeler: &[Kume], sorgular: &[Sorgu]) -> Result<Value, String> {
    Ok(
        json!({"deney":"parca","girdi_ozeti":bilgi["girdi_ozeti"],"meta_ozeti":bilgi["meta_ozeti"],
        "sorgu_ozeti":ozet(&serde_json::to_vec(sorgular).map_err(|_| "sorgular kodlanamadı")?),
        "kumeler":kumeler.iter().map(|k|json!({"pencere":k.pencere,"parca":k.parcalar.len(),"ozet":k.ozet,"gruplar":k.gruplar})).collect::<Vec<_>>(),
        "gomme_modeli":GOMME,"siralama_modeli":SIRALAMA,"gomme_yonlendirmesi":"qwen3-embedding-8b",
        "siralama_yonlendirmesi":"qwen3-vl-reranker-8b","talimat":TALIMAT,"boyut":BOYUT,
        "aday":50,"top_n":50,"toplu":TOPLU,"istek_bayt_siniri":BAYT_SINIRI,"token_siniri":BUTCE,
        "kaynak":"tam-dogrulanmis-dosyalar","gomme":"uc-kip-yeni","sorgu":"32-yeni-ortak"}),
    )
}

#[test]
fn pencere_varsayilani_urunle_utf8_uzun_satir_ve_ortusmede_ayni() {
    let metinler = [
        String::new(),
        " \r\n\t".into(),
        "Türkçe 🦀\r\nson".into(),
        "ö".repeat(10001),
        "orta bir satır\n".repeat(250),
        "ö".repeat(1400) + "\n" + &"uzun\n".repeat(70),
    ];
    for m in metinler {
        assert_eq!(parcala(&m, PENCERELER[1]).unwrap(), urun::parcala(&m));
    }
}

#[test]
fn pencereler_ayni_kaynagi_sinirli_ve_tutarli_boler() {
    let m = "ş 🦀 küçük satır\r\n".repeat(250) + &"ü".repeat(10001);
    for p in PENCERELER {
        let parcalar = parcala(&m, p).unwrap();
        let mut kapsama = vec![false; m.len()];
        for k in parcalar {
            assert_eq!(k.metin, &m[k.bas_bayt..k.son_bayt]);
            assert!(k.metin.chars().count() <= p.karakter);
            assert!(k.satir_son - k.satir_bas < p.satir as u32);
            kapsama[k.bas_bayt..k.son_bayt].fill(true);
        }
        assert!(kapsama.into_iter().all(|b| b));
    }
    assert!(
        parcala(
            "x",
            Pencere {
                satir: 0,
                ..PENCERELER[0]
            }
        )
        .is_err()
    );
}

#[test]
fn tam_kaynak_yeni_son_ek_degisiklik_ve_eksik_parcayi_reddeder() {
    let m = "bir satır\n".repeat(150);
    let k = kumeler(&[DosyaGirdisi {
        yol: "crates/a.rs".into(),
        metin: m.clone(),
    }])
    .unwrap();
    let kaynak: Vec<_> = k[1].parcalar.iter().map(|p| p.kaynak.clone()).collect();
    assert!(tam_eslesir(&m, &kaynak));
    assert!(!tam_eslesir(&(m.clone() + "yeni satır"), &kaynak));
    assert!(!tam_eslesir(&m.replace("bir", "iki"), &kaynak));
    assert!(!tam_eslesir(&m, &kaynak[..kaynak.len() - 1]));
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
enum Tur {
    Gomme { adet: usize },
    Siralama,
}

#[derive(Deserialize, Serialize)]
struct ParcaMakbuzu {
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

fn birim(v: &[f32]) -> Result<Vec<f32>, String> {
    if v.len() != BOYUT || v.iter().any(|x| !x.is_finite()) {
        return Err("parça vektörü boyutu/sonluluğu geçersiz".into());
    }
    let n = v.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>().sqrt();
    if !n.is_finite() || n <= 0.0 {
        return Err("parça vektörü sıfır veya taşmış".into());
    }
    Ok(v.iter().map(|x| (f64::from(*x) / n) as f32).collect())
}

fn makbuzu_dogrula(m: &ParcaMakbuzu) -> Result<(), String> {
    if m.token == 0 || !m.ms.is_finite() || m.ms < 0.0 || m.yanit_bayt == 0 {
        return Err("parça makbuzu süre/token/yanıt geçersiz".into());
    }
    match m.tur {
        Tur::Gomme { adet } => {
            if adet == 0 || adet > TOPLU || m.vektorler.len() != adet || !m.siralama.is_empty() {
                return Err("parça makbuzu gömme sayısı farklı".into());
            }
            for v in &m.vektorler {
                birim(v)?;
            }
            if !matches!(
                m.model.as_deref(),
                Some("qwen/qwen3-embedding-8b" | "qwen3-embedding-8b")
            ) {
                return Err("parça gömme yönlendirmesi değişti veya eksik".into());
            }
        }
        Tur::Siralama => {
            let mut gorulen = BTreeSet::new();
            if !m.vektorler.is_empty()
                || m.siralama.len() != 50
                || m.siralama
                    .iter()
                    .any(|(i, p)| *i >= 50 || !p.is_finite() || !gorulen.insert(*i))
            {
                return Err("parça sıralama makbuzu eksik/tekrarlı/geçersiz".into());
            }
            if !matches!(
                m.model.as_deref(),
                Some("qwen/qwen3-vl-reranker-8b" | "qwen3-vl-reranker-8b")
            ) {
                return Err("parça sıralama yönlendirmesi değişti veya eksik".into());
            }
        }
    }
    Ok(())
}

fn eski_token(depo: &Path) -> Result<u64, String> {
    let mut toplam = 0u64;
    for g in std::fs::read_dir(depo).map_err(|_| "parça makbuz dizini okunamadı")? {
        let yol = g.map_err(|_| "parça makbuz yolu okunamadı")?.path();
        if yol.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let m: ParcaMakbuzu =
            serde_json::from_slice(&std::fs::read(&yol).map_err(|_| "parça makbuzu okunamadı")?)
                .map_err(|_| "parça makbuzu bozuk")?;
        if yol.file_stem().and_then(|s| s.to_str()) != Some(m.istek_ozeti.as_str()) {
            return Err("parça makbuzu özeti farklı".into());
        }
        makbuzu_dogrula(&m)?;
        toplam = toplam.checked_add(m.token).ok_or("parça bütçesi taştı")?;
    }
    if toplam > BUTCE {
        return Err("önceki parça koşuları bütçeyi aştı".into());
    }
    Ok(toplam)
}

fn butce_uygun(harcanan: u64, bayt: usize) -> bool {
    u64::try_from(bayt)
        .ok()
        .and_then(|b| b.checked_mul(2))
        .and_then(|b| b.checked_add(4096))
        .and_then(|b| b.checked_add(harcanan))
        .is_some_and(|b| b <= BUTCE)
}

async fn parca_istegi(
    istemci: &EvrenIstemcisi,
    depo: &Path,
    tur: Tur,
    govde: String,
    harcanan: &mut u64,
) -> Result<(ParcaMakbuzu, bool), String> {
    if govde.len() > BAYT_SINIRI {
        return Err("parça isteği 1 MiB sınırını aşıyor".into());
    }
    let istek_ozeti = ozet(govde.as_bytes());
    let yol = depo.join(format!("{istek_ozeti}.json"));
    if yol.exists() {
        let m: ParcaMakbuzu =
            serde_json::from_slice(&std::fs::read(&yol).map_err(|_| "parça makbuzu okunamadı")?)
                .map_err(|_| "parça makbuzu bozuk")?;
        if m.istek_ozeti != istek_ozeti || m.tur != tur {
            return Err("parça makbuzu istekten farklı".into());
        }
        makbuzu_dogrula(&m)?;
        return Ok((m, true));
    }
    if !butce_uygun(*harcanan, govde.len()) {
        return Err("16 milyon parça token bütçesi; yeni çağrı başlamadı".into());
    }
    let govde: Value = serde_json::from_str(&govde).map_err(|_| "parça isteği JSON değil")?;
    let uc = match tur {
        Tur::Gomme { .. } => EkUc::Gomme,
        Tur::Siralama => EkUc::Siralama,
    };
    tokio::time::sleep(Duration::from_secs(2)).await;
    let bas = Instant::now();
    let yanit = istemci.ek_uc(uc, &govde).await.map_err(guvenli_hata)?;
    let ms = bas.elapsed().as_secs_f64() * 1000.0;
    let kok: Value = serde_json::from_str(&yanit.govde).map_err(|_| "parça yanıtı JSON değil")?;
    let token = kok["usage"]["prompt_tokens"]
        .as_u64()
        .ok_or("parça kullanımı eksik; ölçüm durdu")?;
    *harcanan = harcanan
        .checked_add(token)
        .ok_or("parça token toplamı taştı")?;
    let (vektorler, siralama) = match tur {
        Tur::Gomme { adet } => (
            gomme_yanitini_coz(&yanit.govde, adet, BOYUT)
                .map_err(|_| "parça gömme yanıtı geçersiz")?,
            Vec::new(),
        ),
        Tur::Siralama => (
            Vec::new(),
            siralama_yanitini_coz(&yanit.govde, 50)
                .map_err(|_| "parça sıralama yanıtı geçersiz")?,
        ),
    };
    let m = ParcaMakbuzu {
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
        tempfile::NamedTempFile::new_in(depo).map_err(|_| "parça makbuz dosyası açılamadı")?;
    serde_json::to_writer(gecici.as_file_mut(), &m).map_err(|_| "parça makbuzu yazılamadı")?;
    gecici
        .as_file()
        .sync_all()
        .map_err(|_| "parça makbuzu diske yazılamadı")?;
    gecici
        .persist_noclobber(yol)
        .map_err(|_| "parça makbuzu kaydedilemedi")?;
    if *harcanan > BUTCE {
        return Err("parça yanıtının kullanımı bütçeyi aştı; ölçüm durdu".into());
    }
    Ok((m, false))
}

#[tokio::test]
#[ignore = "G5 parça penceresi kıyası; ayrıca açık onay, EVREN_API_KEY ve mevcut indeks gerekir"]
async fn canli_parca_karsilastirmasi() -> Result<(), String> {
    if (
        PENCERELER[1].satir,
        PENCERELER[1].karakter,
        PENCERELER[1].ortusme,
    ) != (urun::PENCERE_SATIR, urun::PENCERE_KARAKTER, urun::ORTUSME)
    {
        return Err("ürün varsayılan penceresi değişti".into());
    }
    let kok = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|_| "depo kökü açılamadı")?;
    let (dosyalar, mut bilgi) = dosyalar(&kok)?;
    let mut kumeler = kumeler(&dosyalar)?;
    let sorgular: Vec<Sorgu> = serde_json::from_str(include_str!("../degerlendirme.json"))
        .map_err(|_| "sorgular geçersiz")?;
    if sorgular.len() != 32 || kumeler.iter().any(|k| k.parcalar.len() < 50) {
        return Err("32 sorgu ve her kipte 50 parça gerekli".into());
    }
    let tarif = tarif(&bilgi, &kumeler, &sorgular)?;
    let deney_ozeti = ozet(&serde_json::to_vec(&tarif).map_err(|_| "parça tarifi kodlanamadı")?);
    bilgi["tarif"] = tarif;
    bilgi["deney_ozeti"] = json!(deney_ozeti);
    println!("{bilgi}");
    let eksik: Vec<_> = sorgular
        .iter()
        .enumerate()
        .filter(|(_, s)| !dosyalar.iter().any(|d| isabet(&d.yol, &s.beklenen)))
        .map(|(i, _)| i)
        .collect();
    let kaynak_istek: usize = kumeler.iter().map(|k| k.gruplar.len()).sum();
    let kipler:Vec<_>=kumeler.iter().map(|k|json!({"pencere":k.pencere,"parca":k.parcalar.len(),
        "gomme_istek":k.gruplar.len(),"metin_bayt":k.parcalar.iter().map(|p|p.metin.len()).sum::<usize>(),
        "ham_vektor_bayt":k.parcalar.len()*BOYUT*4})).collect();
    println!(
        "{}",
        json!({"asama":"kapsam","kipler":kipler,"sorgu":32,"aday":50,
        "kaynak_gomme_istek":kaynak_istek,"sorgu_gomme_istek":32,"siralama_istek":96,"http_enfazla":kaynak_istek+128,
        "beklenen_dosyasi_olmayan_sorgular":eksik,"token_siniri":BUTCE})
    );
    if !eksik.is_empty() {
        return Err("beklenen dosyası bulunmayan parça sorgusu var".into());
    }
    if std::env::var("KALFA_G5_KURU").as_deref() == Ok("1") {
        return Ok(());
    }
    let onay = std::env::var("KALFA_G5_PARCA_DENEY_OZETI")
        .map_err(|_| "onaylı parça deneyi özeti gerekli")?;
    if onay != deney_ozeti {
        return Err("parça deneyi onaylanandan farklı".into());
    }
    let anahtar = std::env::var("EVREN_API_KEY").map_err(|_| "EVREN_API_KEY tanımlı değil")?;
    let taban = std::env::var("EVREN_BASE_URL").unwrap_or_else(|_| HEDEF.into());
    if taban.trim_end_matches('/') != HEDEF {
        return Err("EVREN hedefi onaylanandan farklı".into());
    }
    let istemci = EvrenIstemcisi::yeni(taban, ApiAnahtari::yeni(anahtar)).map_err(guvenli_hata)?;
    let depo = kok.join("target/kalfa-g5-parca").join(&onay);
    std::fs::create_dir_all(&depo).map_err(|_| "parça dizini açılamadı")?;
    let onceki = eski_token(&depo)?;
    let mut harcanan = onceki;
    // Tüm kaynak vektörleri aynı koşuda yeni alınır; eski kıyas makbuzları kullanılmaz.
    for k in &mut kumeler {
        for &(bas, son) in &k.gruplar {
            let metinler: Vec<_> = k.parcalar[bas..son]
                .iter()
                .map(|p| p.metin.clone())
                .collect();
            let (m, eski) = parca_istegi(
                &istemci,
                &depo,
                Tur::Gomme { adet: son - bas },
                gomme_govdesi(GOMME, &metinler, BOYUT as u32),
                &mut harcanan,
            )
            .await?;
            for (p, v) in k.parcalar[bas..son].iter_mut().zip(&m.vektorler) {
                p.vektor = birim(v)?;
            }
            println!(
                "{}",
                json!({"asama":"indeks","kip":k.pencere.ad,"bas":bas,"adet":son-bas,
                "token":m.token,"ms":m.ms,"model":m.model,"yanit_bayt":m.yanit_bayt,"onceki":eski,"istek_ozeti":m.istek_ozeti})
            );
        }
    }
    for (i, s) in sorgular.iter().enumerate() {
        let (q, eski) = parca_istegi(
            &istemci,
            &depo,
            Tur::Gomme { adet: 1 },
            gomme_govdesi(GOMME, &[format!("{TALIMAT}{}", s.sorgu)], BOYUT as u32),
            &mut harcanan,
        )
        .await?;
        println!(
            "{}",
            json!({"asama":"sorgu","sorgu_no":i,"token":q.token,"ms":q.ms,"model":q.model,"yanit_bayt":q.yanit_bayt,"onceki":eski,"istek_ozeti":q.istek_ozeti})
        );
        for j in 0..kumeler.len() {
            // Sağlayıcı zamanının etkisini azaltmak için sorgu başına kip sırası döner.
            let k = &kumeler[(i + j) % kumeler.len()];
            let bas = Instant::now();
            let sira = kosinus_sirasi(&k.parcalar, &q.vektorler[0])?;
            let kosinus_ms = bas.elapsed().as_secs_f64() * 1000.0;
            let adaylar = adaylari_sec(&k.parcalar, &sira, 50, 50);
            let belgeler: Vec<_> = adaylar
                .iter()
                .map(|n| k.parcalar[*n].metin.clone())
                .collect();
            let (m, eski) = parca_istegi(
                &istemci,
                &depo,
                Tur::Siralama,
                siralama_govdesi(SIRALAMA, &s.sorgu, &belgeler, 50),
                &mut harcanan,
            )
            .await?;
            let kayit: Vec<_> = adaylar
                .iter()
                .map(|n| {
                    let p = &k.parcalar[*n];
                    json!({"yol":p.yol,"sira":p.sira,"ozet":p.kaynak.ozet})
                })
                .collect();
            println!(
                "{}",
                json!({"asama":"siralama","sorgu_no":i,"sorgu":s.sorgu,"beklenen":s.beklenen,"kip":k.pencere.ad,
                "adaylar":kayit,"aday":adaylar.len(),"aday_tekil_dosya":adaylar.iter().map(|n|k.parcalar[*n].yol.as_str()).collect::<BTreeSet<_>>().len(),
                "aday_isabet":adaylar.iter().any(|n|isabet(&k.parcalar[*n].yol,&s.beklenen)),
                "token":m.token,"ms":m.ms,"model":m.model,"yanit_bayt":m.yanit_bayt,"onceki":eski,"istek_ozeti":m.istek_ozeti,"kosinus_ms":kosinus_ms,
                "parca_ilk5":sonuclar(&k.parcalar,&adaylar,&m.siralama,&s.beklenen,false),
                "dosya_ilk5":sonuclar(&k.parcalar,&adaylar,&m.siralama,&s.beklenen,true)})
            );
        }
    }
    println!(
        "{}",
        json!({"asama":"bitti","bu_kosu_token":harcanan-onceki,"tum_kosular_token":harcanan,"deney_ozeti":onay,"urun_indeksine_yazildi":false})
    );
    Ok(())
}

#[test]
fn parca_onayi_pencere_kaynak_sorgu_grup_ve_butceyi_baglar() {
    let dosyalar = [DosyaGirdisi {
        yol: "crates/a.rs".into(),
        metin: "bir satır\n".repeat(150),
    }];
    let mut k = kumeler(&dosyalar).unwrap();
    let bilgi = json!({"girdi_ozeti":"a","meta_ozeti":"b"});
    let s = [Sorgu {
        sorgu: "örnek".into(),
        beklenen: vec!["crates/a.rs".into()],
    }];
    let t = tarif(&bilgi, &k, &s).unwrap();
    k[0].pencere.ortusme += 1;
    assert_ne!(t, tarif(&bilgi, &k, &s).unwrap());
    for alan in [
        "girdi_ozeti",
        "meta_ozeti",
        "sorgu_ozeti",
        "kumeler",
        "boyut",
        "talimat",
        "aday",
        "top_n",
        "token_siniri",
    ] {
        assert!(t.get(alan).is_some());
    }
}

#[test]
fn parca_makbuzu_yonlendirme_boyut_sira_ve_butceyi_dogrular() {
    let mut m = ParcaMakbuzu {
        istek_ozeti: ozet(b"ornek"),
        tur: Tur::Gomme { adet: 1 },
        ms: 1.0,
        token: 1,
        model: Some("qwen/qwen3-embedding-8b".into()),
        yanit_bayt: 1,
        vektorler: vec![vec![1.0; BOYUT]],
        siralama: Vec::new(),
    };
    assert!(makbuzu_dogrula(&m).is_ok());
    m.model = None;
    assert!(makbuzu_dogrula(&m).is_err());
    m.model = Some("baska".into());
    assert!(makbuzu_dogrula(&m).is_err());
    m.model = Some("qwen3-embedding-8b".into());
    m.vektorler[0].pop();
    assert!(makbuzu_dogrula(&m).is_err());
    m.vektorler = vec![vec![0.0; BOYUT]];
    assert!(makbuzu_dogrula(&m).is_err());
    m.vektorler[0][0] = f32::NAN;
    assert!(makbuzu_dogrula(&m).is_err());
    m.tur = Tur::Siralama;
    m.vektorler.clear();
    m.model = Some("qwen3-vl-reranker-8b".into());
    m.siralama = (0..50).map(|i| (i, 0.5)).collect();
    assert!(makbuzu_dogrula(&m).is_ok());
    m.siralama[1].0 = 0;
    assert!(makbuzu_dogrula(&m).is_err());
    assert!(!butce_uygun(BUTCE, 1));
    assert!(!butce_uygun(0, usize::MAX));
    let d = tempfile::tempdir().unwrap();
    m.siralama[1].0 = 1;
    m.token = BUTCE + 1;
    std::fs::write(
        d.path().join(format!("{}.json", m.istek_ozeti)),
        serde_json::to_vec(&m).unwrap(),
    )
    .unwrap();
    assert!(eski_token(d.path()).is_err());
}

#[test]
fn parca_grubu_adet_ve_kacisli_json_butcesini_sinirlar() {
    let d = [DosyaGirdisi {
        yol: "crates/a.rs".into(),
        metin: "\"\\\tö\n".repeat(1000),
    }];
    for k in kumeler(&d).unwrap() {
        let mut son = 0;
        for (b, s) in k.gruplar {
            assert_eq!(b, son);
            assert!(s > b && s - b <= TOPLU);
            assert!(
                gomme_govdesi(
                    GOMME,
                    &k.parcalar[b..s]
                        .iter()
                        .map(|p| p.metin.clone())
                        .collect::<Vec<_>>(),
                    BOYUT as u32
                )
                .len()
                    <= BAYT_SINIRI
            );
            son = s;
        }
        assert_eq!(son, k.parcalar.len());
    }
}
