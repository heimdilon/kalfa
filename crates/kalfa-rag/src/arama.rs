//! Arama ve sonuç biçimi (aşama 4 tasarımı §11.3, §11.4).

use std::collections::{BTreeMap, BTreeSet};

use kalfa_plugin_sdk::Hata;
use serde::Deserialize;

use crate::indeks::{self, IndeksDurumu};
use crate::ortam::{BOYUT, Ortam};
use crate::parca;

/// Sorgu gömülürken başına eklenen talimat (Qwen3-Embedding'in önerdiği
/// biçim). Belgelere talimat eklenmez.
pub const TALIMAT: &str = "Instruct: Given a code search query, retrieve relevant code or documentation passages\nQuery: ";
/// Kosinüs benzerliğiyle seçilip yeniden sıralamaya giden parça sayısı.
pub const ADAY: usize = 50;
/// Sonucun bayt sınırı (§7.4: `sonuc` 64 KiB).
pub const SONUC_SINIRI: usize = 60 * 1024;
/// En çok 20 sonuç varken sona yazılan "gösterilmedi" notu için ayrılan yer.
const KAPANIS_REZERVI: usize = "\n\n(20 sonuç sınır yüzünden gösterilmedi)".len();

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Argumanlar {
    pub sorgu: String,
    #[serde(default = "varsayilan_sonuc_sayisi")]
    pub sonuc_sayisi: u32,
}

fn varsayilan_sonuc_sayisi() -> u32 {
    8
}

/// Dosyadan okunmuş ve özeti doğrulanmış aday parça.
struct Aday {
    yol: String,
    satir_bas: u32,
    satir_son: u32,
    metin: String,
    puan: f32,
}

/// İndeksi günceller, arar ve modele gidecek metni üretir.
pub fn ara(o: &dyn Ortam, a: &Argumanlar) -> Result<String, Hata> {
    // Boş sorgu, pahalı indeksleme başlamadan reddedilir.
    let sorgu = temiz_sorgu(&a.sorgu)?;
    // Sorgu indekslemeden önce gömülür: indeksleme hız sınırına çarpsa da
    // eldeki vektörlerle arama yapılabilir. Sorgu kalıcı olarak saklanmaz.
    let q = sorguyu_gom(o, sorgu)?;
    let durum = indeks::guncelle(o)?;
    let vektorler = indeks::vektorleri_yukle(o, &durum.meta)?;
    vektorle_ara(o, a, &durum, vektorler, q)
}

/// `ara`'nın indeksten sonraki yarısı: en yakın parçaları seçer, metinlerini
/// dosyadan okur, yeniden sıralar ve sonucu biçimler.
pub fn sonuclari_bul(
    o: &dyn Ortam,
    a: &Argumanlar,
    durum: &IndeksDurumu,
    vektorler: Vec<(String, usize, Vec<f32>)>,
) -> Result<String, Hata> {
    let sorgu = temiz_sorgu(&a.sorgu)?;
    if vektorler.is_empty() {
        return Ok(ikinci_satir(
            format!("\"{sorgu}\" için sonuç yok: indekste dosya yok."),
            eksiklik_satiri(durum),
        ));
    }
    let q = sorguyu_gom(o, sorgu)?;
    vektorle_ara(o, a, durum, vektorler, q)
}

fn sorguyu_gom(o: &dyn Ortam, sorgu: &str) -> Result<Vec<f32>, Hata> {
    let mut q = o
        .gomme(&[format!("{TALIMAT}{sorgu}")])?
        .into_iter()
        .next()
        .ok_or_else(|| Hata::Diger("sorgu gömülemedi".into()))?;
    if q.len() != BOYUT {
        return Err(Hata::Diger(format!(
            "sorgu vektörü {} boyutlu; {BOYUT} bekleniyordu",
            q.len()
        )));
    }
    // Belgeler birim uzunlukta saklanır; yalnızca sorgu normalize edilir.
    // Sıfır vektörde bölme yapılmaz: bütün puanlar sıfır olur.
    let uzunluk = q.iter().map(|x| x * x).sum::<f32>().sqrt();
    let uzunluk = if uzunluk > 0.0 { uzunluk } else { 1.0 };
    q.iter_mut().for_each(|x| *x /= uzunluk);
    Ok(q)
}

fn vektorle_ara(
    o: &dyn Ortam,
    a: &Argumanlar,
    durum: &IndeksDurumu,
    vektorler: Vec<(String, usize, Vec<f32>)>,
    q: Vec<f32>,
) -> Result<String, Hata> {
    let sorgu = temiz_sorgu(&a.sorgu)?;
    let n = a.sonuc_sayisi.clamp(1, 20);
    if vektorler.is_empty() {
        return Ok(ikinci_satir(
            format!("\"{sorgu}\" için sonuç yok: indekste dosya yok."),
            eksiklik_satiri(durum),
        ));
    }

    // ponytail: tam sıralama; ~45 bin parçada milisaniyeler sürer. Büyürse
    // select_nth_unstable_by ile yalnızca ilk ADAY seçilir.
    let mut puanli: Vec<_> = vektorler
        .iter()
        .filter_map(|(yol, sira, v)| {
            let kayit = durum.meta.dosyalar.get(yol)?.parcalar.get(*sira)?;
            let puan: f32 = q.iter().zip(v).map(|(x, y)| x * y).sum();
            Some((puan, yol.as_str(), *sira, kayit))
        })
        .collect();
    // Eşit puanda yol, sonra dosyadaki sıra belirler.
    puanli.sort_by(|x, y| {
        y.0.total_cmp(&x.0)
            .then_with(|| x.1.cmp(y.1))
            .then(x.2.cmp(&y.2))
    });
    // İlk ADAY parça adaydır. Sonuçlar dosyaya göre tekilleştiği için bu
    // parçalar `n`'den az dosyadan geliyorsa, sonraki dosyaların en iyi
    // parçası da eklenir (en çok `n - 1` belge). Böylece tek dosya adayları
    // doldurduğunda da liste öteki dosyalarla dolabilir.
    let mut dosyalar: BTreeSet<&str> = puanli.iter().take(ADAY).map(|p| p.1).collect();
    let ek: Vec<_> = puanli
        .iter()
        .skip(ADAY)
        .filter(|p| dosyalar.len() < n as usize && dosyalar.insert(p.1))
        .copied()
        .collect();
    puanli.truncate(ADAY);
    puanli.extend(ek);

    // Her dosya bir kez okunur. Okunamayan, arada değişmiş ya da bayt aralığı
    // bozuk parça atlanır.
    let mut icerikler: BTreeMap<&str, Option<String>> = BTreeMap::new();
    let mut adaylar = Vec::new();
    for &(puan, yol, _, kayit) in &puanli {
        let Some(icerik) = icerikler
            .entry(yol)
            .or_insert_with(|| o.dosya_oku(yol).ok())
        else {
            continue;
        };
        let (Ok(bas), Ok(son)) = (
            usize::try_from(kayit.bas_bayt),
            usize::try_from(kayit.son_bayt),
        ) else {
            continue;
        };
        let Some(metin) = icerik.get(bas..son) else {
            continue;
        };
        if parca::ozet(metin) != kayit.ozet {
            continue;
        }
        adaylar.push(Aday {
            yol: yol.to_string(),
            satir_bas: kayit.satir_bas,
            satir_son: kayit.satir_son,
            metin: metin.to_string(),
            puan,
        });
    }

    // Bütün adayların sırası istenir: aynı dosyanın parçaları atlanınca liste
    // sonraki dosyalarla dolar. `top_n` maliyeti değiştirmez; token sorgu ve
    // belgelerden sayılır.
    let mut arama_notu = None;
    let siralama = if adaylar.is_empty() {
        Vec::new()
    } else {
        let metinler: Vec<String> = adaylar.iter().map(|a| a.metin.clone()).collect();
        match o.sirala(sorgu, &metinler, adaylar.len() as u32) {
            Ok(siralama) => siralama,
            Err(Hata::SinirAsildi(_)) => {
                arama_notu = Some("Yeniden sıralama EVREN sınırına takıldı; sonuçlar kosinüs benzerliğiyle sıralandı.".into());
                adaylar
                    .iter()
                    .enumerate()
                    .map(|(i, a)| (i, a.puan))
                    .collect()
            }
            Err(e) => return Err(e),
        }
    };
    // Her dosya en iyi sıradaki parçasıyla bir kez gelir: aynı dosyanın
    // parçaları ilk sonuçları doldurup başka dosyaları aşağı itmez (G5: ilk
    // beşte 26/32 yerine 28/32). Dosyanın geri kalanı `dosya_oku` ile okunur.
    let mut gorulen = BTreeSet::new();
    let sonuclar: Vec<(&Aday, f32)> = siralama
        .iter()
        .filter_map(|&(i, puan)| adaylar.get(i).map(|aday| (aday, puan)))
        .filter(|(aday, _)| gorulen.insert(aday.yol.as_str()))
        .take(n as usize)
        .collect();

    if sonuclar.is_empty() {
        return Ok(ikinci_satir(
            format!(
                "\"{sorgu}\" için sonuç bulunamadı (indeks: {} dosya; bu aramada güncellenen: {}).",
                durum.indeksli, durum.guncellenen
            ),
            eksiklik_satiri(durum),
        ));
    }
    Ok(bicimle(sorgu, durum, &sonuclar, arama_notu))
}

/// Sonuç metnini biçimler: başlık, varsa eksiklik satırı ve kaynaklı
/// sonuçlar. Toplam `SONUC_SINIRI`'nı aşmaz; sığmayan parça kısaltılır.
fn bicimle(
    sorgu: &str,
    durum: &IndeksDurumu,
    sonuclar: &[(&Aday, f32)],
    arama_notu: Option<String>,
) -> String {
    let mut cikti = ikinci_satir(
        format!(
            "\"{sorgu}\" için {} sonuç (indeks: {} dosya; bu aramada güncellenen: {}).",
            sonuclar.len(),
            durum.indeksli,
            durum.guncellenen
        ),
        eksiklik_satiri(durum),
    );
    cikti = ikinci_satir(cikti, arama_notu);
    // Kapanış notunun yeri baştan ayrılır: çıktı hiçbir durumda sınırı aşmaz.
    let butce = SONUC_SINIRI - KAPANIS_REZERVI;
    let mut gosterilen = 0;
    for (sira, (aday, puan)) in sonuclar.iter().enumerate() {
        // Her sonuç, öncesindeki başlıktan ya da sonuçtan tek boş satırla ayrılır.
        let baslik = format!(
            "\n\n[{}] dosya:{}:{}-{} (puan {})",
            sira + 1,
            aday.yol,
            aday.satir_bas,
            aday.satir_son,
            puan_bicimi(*puan)
        );
        let govde = aday.metin.trim_end_matches(['\r', '\n']);
        let tam = format!("{baslik}\n{govde}");
        if cikti.len() + tam.len() <= butce {
            cikti.push_str(&tam);
            gosterilen += 1;
            continue;
        }
        // Sığmayan parça UTF-8 sınırında kısaltılır. Başlığa bile yer yoksa bu
        // ve sonraki sonuçlar atlanır.
        let on = format!("{baslik}\n");
        let not = format!(
            "\n… (kısaltıldı; tamamı için dosya_oku ile {} satır {}-{})",
            aday.yol, aday.satir_bas, aday.satir_son
        );
        if cikti.len() + on.len() + not.len() <= butce {
            let yer = butce - cikti.len() - on.len() - not.len();
            cikti.push_str(&on);
            cikti.push_str(&govde[..karakter_siniri(govde, yer)]);
            cikti.push_str(&not);
            gosterilen += 1;
        }
        break;
    }
    let gizli = sonuclar.len() - gosterilen;
    if gizli > 0 {
        cikti.push_str(&format!("\n\n({gizli} sonuç sınır yüzünden gösterilmedi)"));
    }
    cikti
}

/// Sorguyu kırpar; boşluktan ibaretse hata döner.
fn temiz_sorgu(sorgu: &str) -> Result<&str, Hata> {
    let temiz = sorgu.trim();
    if temiz.is_empty() {
        return Err(Hata::Diger("sorgu boş".into()));
    }
    Ok(temiz)
}

/// İndeks eksikse ya da dosyaların hepsi indekslenmemişse sonuca konan satır.
fn eksiklik_satiri(durum: &IndeksDurumu) -> Option<String> {
    if durum.eksik.is_none() && durum.indeksli >= durum.dosya_sayisi {
        return None;
    }
    let mut satir = format!(
        "İndeks eksik: {} dosyanın {} tanesi indekslendi.",
        durum.dosya_sayisi, durum.indeksli
    );
    if let Some(neden) = &durum.eksik {
        satir.push(' ');
        satir.push_str(neden);
    }
    Some(satir)
}

/// Ana satırın altına, varsa, ikinci bir satır ekler.
fn ikinci_satir(mut ana: String, ek: Option<String>) -> String {
    if let Some(satir) = ek {
        ana.push('\n');
        ana.push_str(&satir);
    }
    ana
}

/// Puanı iki ondalıkla ve ondalık virgülle yazar: `0,54`.
fn puan_bicimi(puan: f32) -> String {
    format!("{puan:.2}").replace('.', ",")
}

/// `en_fazla` bayta sığan en uzun önekin sonu; UTF-8 karakter sınırında.
fn karakter_siniri(metin: &str, en_fazla: usize) -> usize {
    let mut i = en_fazla.min(metin.len());
    while !metin.is_char_boundary(i) {
        i -= 1;
    }
    i
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::meta::{DosyaKaydi, Meta, ParcaKaydi};
    use crate::test_ortami::{SahteOrtam, sahte_vektor};

    fn arg(sorgu: &str, sonuc_sayisi: u32) -> Argumanlar {
        Argumanlar {
            sorgu: sorgu.into(),
            sonuc_sayisi,
        }
    }

    fn basarili(s: Result<String, Hata>) -> String {
        s.unwrap_or_else(|_| panic!("beklenmeyen hata"))
    }

    /// Sorgunun gömme vektörü: `SahteOrtam` talimatlı sorguyu böyle gömer.
    fn sorgu_vektoru(sorgu: &str) -> Vec<f32> {
        sahte_vektor(&format!("{TALIMAT}{sorgu}"))
    }

    /// Dosyayı üç satırlık, örtüşmesiz pencerelere böler: her pencerenin kaydını
    /// (vektör yeri hariç) ve metnini verir.
    fn parcala_elle(icerik: &str) -> Vec<(ParcaKaydi, String)> {
        let mut satirlar = Vec::new();
        let mut konum = 0;
        for satir in icerik.split_inclusive('\n') {
            satirlar.push((konum, konum + satir.len()));
            konum += satir.len();
        }
        (0..satirlar.len())
            .step_by(3)
            .map(|bas| {
                let son = (bas + 3).min(satirlar.len()) - 1;
                let (bas_bayt, son_bayt) = (satirlar[bas].0, satirlar[son].1);
                let metin = &icerik[bas_bayt..son_bayt];
                let kayit = ParcaKaydi {
                    satir_bas: bas as u32 + 1,
                    satir_son: son as u32 + 1,
                    bas_bayt: bas_bayt as u64,
                    son_bayt: son_bayt as u64,
                    ozet: parca::ozet(metin),
                    ..Default::default()
                };
                (kayit, metin.to_string())
            })
            .collect()
    }

    /// Dosyaları `SahteOrtam`'a yazar; `meta`'yı ve vektörleri elle kurar.
    fn indeks_kur<Y: AsRef<str>, I: AsRef<str>>(
        o: &SahteOrtam,
        dosyalar: &[(Y, I)],
    ) -> (IndeksDurumu, Vec<(String, usize, Vec<f32>)>) {
        let mut meta = Meta::yeni();
        let mut vektorler = Vec::new();
        for (yol, icerik) in dosyalar {
            let (yol, icerik): (&str, &str) = (yol.as_ref(), icerik.as_ref());
            o.dosya(yol, icerik);
            let mut parcalar = Vec::new();
            for (idx, (mut kayit, metin)) in parcala_elle(icerik).into_iter().enumerate() {
                kayit.sira = vektorler.len() as u32;
                vektorler.push((yol.to_string(), idx, sahte_vektor(&metin)));
                parcalar.push(kayit);
            }
            meta.dosyalar.insert(
                yol.to_string(),
                DosyaKaydi {
                    boyut: icerik.len() as u64,
                    degisme: o.simdi.get(),
                    parcalar,
                },
            );
        }
        let sayi = dosyalar.len() as u64;
        let durum = IndeksDurumu {
            meta,
            dosya_sayisi: sayi,
            indeksli: sayi,
            guncellenen: 0,
            eksik: None,
        };
        (durum, vektorler)
    }

    #[test]
    fn argumanlar_varsayilan_sonuc_sayisi() {
        let a: Argumanlar = serde_json::from_str(r#"{"sorgu": "alfa"}"#).expect("geçerli JSON");
        assert_eq!(a.sorgu, "alfa");
        assert_eq!(a.sonuc_sayisi, 8);
    }

    #[test]
    fn bos_sorgu_hata_verir() {
        let o = SahteOrtam::yeni();
        let (durum, vektorler) = indeks_kur(&o, &[("a.rs", "alfa\n")]);
        let s = sonuclari_bul(&o, &arg("   ", 8), &durum, vektorler);
        assert!(matches!(s, Err(Hata::Diger(m)) if m == "sorgu boş"));
        // İndeksleme ve gömme başlamadan reddedilir.
        let s = ara(&o, &arg("", 8));
        assert!(matches!(s, Err(Hata::Diger(m)) if m == "sorgu boş"));
        assert!(o.gommeler.borrow().is_empty());
    }

    #[test]
    fn bos_indeks_gomme_yapmaz() {
        let o = SahteOrtam::yeni();
        let durum = IndeksDurumu {
            meta: Meta::yeni(),
            dosya_sayisi: 3,
            indeksli: 0,
            guncellenen: 0,
            eksik: Some("hız sınırı aşıldı".into()),
        };
        let s = basarili(sonuclari_bul(&o, &arg("alfa", 8), &durum, Vec::new()));
        assert_eq!(
            s,
            "\"alfa\" için sonuç yok: indekste dosya yok.\n\
             İndeks eksik: 3 dosyanın 0 tanesi indekslendi. hız sınırı aşıldı"
        );
        assert!(o.gommeler.borrow().is_empty());
    }

    #[test]
    fn eksik_indeks_satiri_gerektiginde_gelir() {
        let o = SahteOrtam::yeni();
        let (mut durum, vektorler) = indeks_kur(&o, &[("a.rs", "alfa beta\n"), ("b.rs", "alfa\n")]);
        let tam = basarili(sonuclari_bul(
            &o,
            &arg("alfa", 8),
            &durum,
            vektorler.clone(),
        ));
        assert!(!tam.contains("İndeks eksik"));

        durum.dosya_sayisi = 3;
        durum.indeksli = 2;
        let yarim = basarili(sonuclari_bul(
            &o,
            &arg("alfa", 8),
            &durum,
            vektorler.clone(),
        ));
        assert!(yarim.contains("\nİndeks eksik: 3 dosyanın 2 tanesi indekslendi.\n\n[1] "));

        durum.dosya_sayisi = 2;
        durum.eksik = Some("kota doldu".into());
        let kota = basarili(sonuclari_bul(&o, &arg("alfa", 8), &durum, vektorler));
        assert!(
            kota.contains("\nİndeks eksik: 2 dosyanın 2 tanesi indekslendi. kota doldu\n\n[1] ")
        );
    }

    #[test]
    fn sonuc_bicimi_ve_yeniden_siralama() {
        let o = SahteOrtam::yeni();
        let (durum, mut vektorler) = indeks_kur(&o, &[("a.rs", "alfa beta\n"), ("b.rs", "alfa\n")]);
        // Kosinüste önce b.rs (vektörü sorguyla aynı yönde), sonra a.rs.
        vektorler[0].2 = sahte_vektor("baska");
        vektorler[1].2 = sorgu_vektoru("alfa beta");
        let s = basarili(sonuclari_bul(&o, &arg("alfa beta", 8), &durum, vektorler));
        // Yeniden sıralama kosinüsü geçersiz kılar: iki sözcüğü de içeren a.rs önde.
        assert_eq!(
            s,
            "\"alfa beta\" için 2 sonuç (indeks: 2 dosya; bu aramada güncellenen: 0).\n\n\
             [1] dosya:a.rs:1-1 (puan 0,67)\nalfa beta\n\n\
             [2] dosya:b.rs:1-1 (puan 0,33)\nalfa"
        );
        // Gömme talimatlı gider; sıralamaya talimatsız sorgu ve kosinüs sırası gider.
        assert_eq!(
            *o.gommeler.borrow(),
            vec![vec![format!("{TALIMAT}alfa beta")]]
        );
        assert_eq!(o.siralamalar.borrow()[0].0, "alfa beta");
        assert_eq!(
            o.siralamalar.borrow()[0].1,
            vec!["alfa\n".to_string(), "alfa beta\n".to_string()]
        );
    }

    /// a.rs'in iki parçası da sorguyu tam içerir; b.rs yalnızca yarısını.
    fn iki_parcali_dosya(o: &SahteOrtam) -> (IndeksDurumu, Vec<(String, usize, Vec<f32>)>) {
        indeks_kur(
            o,
            &[
                ("a.rs", "alfa beta\nx\ny\nalfa beta\nx\ny\n"),
                ("b.rs", "alfa\n"),
            ],
        )
    }

    #[test]
    fn her_dosya_en_iyi_parcasiyla_bir_kez_gelir() {
        let o = SahteOrtam::yeni();
        let (durum, vektorler) = iki_parcali_dosya(&o);
        let s = basarili(sonuclari_bul(&o, &arg("alfa beta", 2), &durum, vektorler));
        // a.rs'in ikinci parçası b.rs'den yüksek puanlıdır ama aynı dosyadır.
        assert!(s.starts_with("\"alfa beta\" için 2 sonuç"), "{s}");
        assert!(s.contains("[1] dosya:a.rs:1-3"), "{s}");
        assert!(s.contains("[2] dosya:b.rs:1-1"), "{s}");
        assert!(!s.contains("a.rs:4-6"), "{s}");
        // Tekilleştirmenin doldurabilmesi için bütün adayların sırası istenir.
        assert_eq!(o.siralamalar.borrow()[0].1.len(), 3);
    }

    #[test]
    fn adaylari_tek_dosya_doldurunca_sonraki_dosyalar_eklenir() {
        let o = SahteOrtam::yeni();
        // a.rs'in 50 parçasının hepsi sorguyla aynı yönde: ilk ADAY adayı
        // doldurur. b.rs ve c.rs daha uzaktır.
        let a: String = (0..150).map(|i| format!("alfa {i}\n")).collect();
        let (durum, mut vektorler) = indeks_kur(
            &o,
            &[
                ("a.rs", a.as_str()),
                ("b.rs", "alfa b\n"),
                ("c.rs", "alfa c\n"),
            ],
        );
        for v in &mut vektorler {
            v.2 = if v.0 == "a.rs" {
                sorgu_vektoru("alfa")
            } else {
                sahte_vektor("baska")
            };
        }
        let s = basarili(sonuclari_bul(&o, &arg("alfa", 2), &durum, vektorler));
        assert!(s.starts_with("\"alfa\" için 2 sonuç"), "{s}");
        assert!(s.contains("dosya:a.rs:"), "{s}");
        assert!(s.contains("dosya:b.rs:1-1"), "{s}");
        // Yalnızca eksik dosya sayısı kadar aday eklenir.
        assert_eq!(o.siralamalar.borrow()[0].1.len(), ADAY + 1);
    }

    #[test]
    fn kosinus_sirasi_da_dosyaya_gore_tekillesir() {
        let o = SahteOrtam::yeni();
        let (durum, mut vektorler) = iki_parcali_dosya(&o);
        vektorler[0].2 = sorgu_vektoru("alfa beta");
        vektorler[1].2 = sorgu_vektoru("alfa beta");
        vektorler[2].2 = sahte_vektor("baska");
        *o.siralama_hatasi.borrow_mut() = Some(Hata::SinirAsildi("429".into()));
        let s = basarili(sonuclari_bul(&o, &arg("alfa beta", 2), &durum, vektorler));
        assert!(s.contains("kosinüs benzerliğiyle sıralandı"));
        assert!(s.contains("[1] dosya:a.rs:1-3 (puan 1,00)"), "{s}");
        assert!(s.contains("[2] dosya:b.rs:1-1"), "{s}");
        assert!(!s.contains("a.rs:4-6"), "{s}");
    }

    #[test]
    fn sonuc_sayisi_bir_ile_yirmi_arasina_kirpilir() {
        let o = SahteOrtam::yeni();
        let dosyalar: Vec<(String, String)> = (0..25)
            .map(|i| (format!("d{i:02}.rs"), format!("satir {i}\n")))
            .collect();
        let (durum, vektorler) = indeks_kur(&o, &dosyalar);
        let az = basarili(sonuclari_bul(
            &o,
            &arg("satir", 0),
            &durum,
            vektorler.clone(),
        ));
        assert!(az.starts_with("\"satir\" için 1 sonuç"));
        let cok = basarili(sonuclari_bul(&o, &arg("satir", 99), &durum, vektorler));
        assert!(cok.starts_with("\"satir\" için 20 sonuç"));
        assert_eq!(cok.matches("] dosya:").count(), 20);
    }

    #[test]
    fn aday_sayisi_elliyle_sinirli() {
        let o = SahteOrtam::yeni();
        let icerik: String = (0..240).map(|i| format!("satir {i}\n")).collect();
        let (durum, vektorler) = indeks_kur(&o, &[("uzun.rs", icerik.as_str())]);
        basarili(sonuclari_bul(&o, &arg("satir", 8), &durum, vektorler));
        // 240 satır = 80 parça; yeniden sıralamaya en çok ADAY gider.
        assert_eq!(o.siralamalar.borrow()[0].1.len(), ADAY);
    }

    #[test]
    fn kosinus_en_yakin_parcayi_adaya_alir() {
        let o = SahteOrtam::yeni();
        let icerik: String = (0..240).map(|i| format!("satir {i}\n")).collect();
        let (durum, mut vektorler) = indeks_kur(&o, &[("uzun.rs", icerik.as_str())]);
        // 55. parça (sıra 54) sorguyla aynı yönde; öteki parçalar değil.
        vektorler[54].2 = sorgu_vektoru("alfa");
        basarili(sonuclari_bul(&o, &arg("alfa", 8), &durum, vektorler));
        let beklenen = parcala_elle(&icerik)[54].1.clone();
        assert_eq!(o.siralamalar.borrow()[0].1[0], beklenen);
    }

    #[test]
    fn esit_puanda_yol_ve_sira_belirler() {
        let o = SahteOrtam::yeni();
        let a: String = (0..90).map(|i| format!("a {i}\n")).collect();
        let b: String = (0..90).map(|i| format!("b {i}\n")).collect();
        let (durum, mut vektorler) = indeks_kur(&o, &[("b.rs", b.as_str()), ("a.rs", a.as_str())]);
        for v in &mut vektorler {
            v.2 = sahte_vektor("ayni");
        }
        basarili(sonuclari_bul(&o, &arg("alfa", 8), &durum, vektorler));
        // Eşit puanda önce yol: a.rs'in 30 parçası, sonra b.rs'in ilk 20'si.
        let mut beklenen: Vec<String> = parcala_elle(&a).into_iter().map(|(_, m)| m).collect();
        beklenen.extend(parcala_elle(&b).into_iter().take(20).map(|(_, m)| m));
        assert_eq!(o.siralamalar.borrow()[0].1, beklenen);
    }

    #[test]
    fn degisen_ve_silinen_parcalar_atlanir() {
        let o = SahteOrtam::yeni();
        let (durum, vektorler) = indeks_kur(
            &o,
            &[
                ("a.rs", "alfa beta\n"),
                ("b.rs", "alfa beta\n"),
                ("c.rs", "alfa beta\n"),
            ],
        );
        o.dosya("b.rs", "alfa değişti\n");
        o.sil("c.rs");
        let s = basarili(sonuclari_bul(&o, &arg("alfa beta", 8), &durum, vektorler));
        assert_eq!(o.siralamalar.borrow()[0].1, vec!["alfa beta\n".to_string()]);
        assert!(s.starts_with("\"alfa beta\" için 1 sonuç"));
        assert!(s.contains("[1] dosya:a.rs:1-1"));
    }

    #[test]
    fn bozuk_bayt_araligi_atlanir() {
        let o = SahteOrtam::yeni();
        let (mut durum, vektorler) = indeks_kur(&o, &[("a.rs", "alfa\n"), ("x.rs", "ğ alfa\n")]);
        // "ğ" iki bayttır: 1. bayt bir karakterin ortasına düşer.
        if let Some(kayit) = durum.meta.dosyalar.get_mut("x.rs") {
            kayit.parcalar[0].bas_bayt = 1;
        }
        basarili(sonuclari_bul(&o, &arg("alfa", 8), &durum, vektorler));
        assert_eq!(o.siralamalar.borrow()[0].1, vec!["alfa\n".to_string()]);
    }

    #[test]
    fn aday_kalmazsa_bulunamadi_yazar() {
        let o = SahteOrtam::yeni();
        let (mut durum, vektorler) = indeks_kur(&o, &[("x.rs", "ğ alfa\n")]);
        if let Some(kayit) = durum.meta.dosyalar.get_mut("x.rs") {
            kayit.parcalar[0].bas_bayt = 1;
        }
        let s = basarili(sonuclari_bul(&o, &arg("alfa", 8), &durum, vektorler));
        assert_eq!(
            s,
            "\"alfa\" için sonuç bulunamadı (indeks: 1 dosya; bu aramada güncellenen: 0)."
        );
        assert!(o.siralamalar.borrow().is_empty());
    }

    #[test]
    fn gomme_hatasi_yukari_doner() {
        let o = SahteOrtam::yeni();
        *o.gomme_hatasi.borrow_mut() = Some((1, Hata::Diger("gömme patladı".into())));
        let (durum, vektorler) = indeks_kur(&o, &[("a.rs", "alfa\n")]);
        let s = sonuclari_bul(&o, &arg("alfa", 8), &durum, vektorler);
        assert!(matches!(s, Err(Hata::Diger(m)) if m == "gömme patladı"));
    }

    #[test]
    fn siralama_sinirinda_kosinus_sirasi_puani_ve_notu_kullanilir() {
        let o = SahteOrtam::yeni();
        let (durum, mut vektorler) = indeks_kur(&o, &[("a.rs", "alfa beta\n"), ("b.rs", "alfa\n")]);
        vektorler[0].2 = sorgu_vektoru("alfa beta").iter().map(|x| -x).collect();
        vektorler[1].2 = sorgu_vektoru("alfa beta");
        *o.siralama_hatasi.borrow_mut() = Some(Hata::SinirAsildi("429".into()));
        let s = basarili(sonuclari_bul(&o, &arg("alfa beta", 1), &durum, vektorler));
        assert!(s.contains("sonuçlar kosinüs benzerliğiyle sıralandı"));
        assert!(s.contains("[1] dosya:b.rs:1-1 (puan 1,00)"), "{s}");
        assert_eq!(s.matches("] dosya:").count(), 1);
        assert!(!s.contains("dosya:a.rs"));
    }

    #[test]
    fn siralamadaki_diger_hata_izin_ve_iptal_yukari_doner() {
        for hata in [
            Hata::Diger("bozuk yanıt".into()),
            Hata::IzinYok("ret".into()),
            Hata::IptalEdildi,
        ] {
            let o = SahteOrtam::yeni();
            let (durum, vektorler) = indeks_kur(&o, &[("a.rs", "alfa\n")]);
            *o.siralama_hatasi.borrow_mut() = Some(hata.clone());
            let sonuc = sonuclari_bul(&o, &arg("alfa", 8), &durum, vektorler);
            match hata {
                Hata::Diger(m) => assert!(matches!(sonuc, Err(Hata::Diger(h)) if h == m)),
                Hata::IzinYok(m) => assert!(matches!(sonuc, Err(Hata::IzinYok(h)) if h == m)),
                Hata::IptalEdildi => assert!(matches!(sonuc, Err(Hata::IptalEdildi))),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn sorgu_sinirinda_indeksleme_baslamaz() {
        let o = SahteOrtam::yeni();
        o.dosya("a.rs", "alfa\n");
        *o.gomme_hatasi.borrow_mut() = Some((1, Hata::SinirAsildi("günlük sınır".into())));
        assert!(matches!(
            ara(&o, &arg("alfa", 8)),
            Err(Hata::SinirAsildi(_))
        ));
        assert!(o.gommeler.borrow()[0][0].starts_with(TALIMAT));
        assert_eq!(o.gommeler.borrow().len(), 1);
        assert_eq!(o.yazma_denemesi.get(), 0);
        assert!(o.siralamalar.borrow().is_empty());
    }

    #[test]
    fn indeksleme_ve_siralama_sinirinda_kaynakli_kismi_sonuc_doner() {
        let o = SahteOrtam::yeni();
        o.dosya("eski.rs", "alfa eski belge\n");
        basarili(ara(&o, &arg("alfa", 8)));
        o.dosya("yeni.rs", "alfa yeni belge\n");
        // İlk arama: sorgu, indeks; ikinci arama: sorgu, indeks (4.).
        *o.gomme_hatasi.borrow_mut() = Some((4, Hata::SinirAsildi("günlük sınır".into())));
        *o.siralama_hatasi.borrow_mut() = Some(Hata::SinirAsildi("günlük sınır".into()));
        let s = basarili(ara(&o, &arg("alfa", 8)));
        assert!(
            s.contains("İndeks eksik: 2 dosyanın 1 tanesi indekslendi."),
            "{s}"
        );
        assert!(s.contains("kosinüs benzerliğiyle sıralandı"));
        assert!(s.contains("dosya:eski.rs:1-1"));
        assert!(!s.contains("dosya:yeni.rs"));
        assert_eq!(o.gommeler.borrow().len(), 4);
    }

    /// Dört dosya, her biri üç satırlık tek parça. Her satır yaklaşık 7 KB
    /// (Türkçe harfli): parça ~21 KB, dört parça sınırı aşar.
    fn uzun_dosyalar() -> Vec<(String, String)> {
        (0..4)
            .map(|d| {
                let icerik: String = (0..3)
                    .map(|i| format!("{:05} {}\n", d * 3 + i, "ğ".repeat(3500)))
                    .collect();
                (format!("uzun{d}.rs"), icerik)
            })
            .collect()
    }

    #[test]
    fn kosinus_sonuclarinin_notu_da_bayt_sinirina_dahildir() {
        let o = SahteOrtam::yeni();
        let (durum, vektorler) = indeks_kur(&o, &uzun_dosyalar());
        *o.siralama_hatasi.borrow_mut() = Some(Hata::SinirAsildi("429".into()));
        let s = basarili(sonuclari_bul(&o, &arg("uzun metin", 8), &durum, vektorler));
        assert!(s.len() <= SONUC_SINIRI);
        assert!(s.contains("kosinüs benzerliğiyle sıralandı"));
        assert!(s.contains("kısaltıldı"));
    }

    #[test]
    fn uzun_sonuc_sinira_kirpilir() {
        let o = SahteOrtam::yeni();
        let (durum, vektorler) = indeks_kur(&o, &uzun_dosyalar());
        let s = basarili(sonuclari_bul(&o, &arg("uzun metin", 8), &durum, vektorler));
        assert!(s.len() <= SONUC_SINIRI);
        assert!(s.contains("\n… (kısaltıldı; tamamı için dosya_oku ile uzun"));
        assert!(s.ends_with("(1 sonuç sınır yüzünden gösterilmedi)"));
        assert_eq!(s.matches("] dosya:").count(), 3);
    }
}
