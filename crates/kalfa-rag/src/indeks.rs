//! İndeksin artımlı güncellenmesi (aşama 4 tasarımı §11.2).

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use kalfa_plugin_sdk::{DosyaBilgisi, Hata};

use crate::meta::{DosyaKaydi, META, Meta, ParcaKaydi, dilim_anahtari, dilim_coz, dilim_kodla};
use crate::ortam::{BOYUT, Ortam};
use crate::parca::{self, Parca};

/// Toplu gömme isteğinin sınırları.
pub const TOPLU_PARCA: usize = 256;
pub const TOPLU_BAYT: usize = 1024 * 1024;
/// Bundan büyük dosyalar indekslenmez.
pub const AZAMI_DOSYA: u64 = 256 * 1024;
/// "Racy clean" penceresi: değişme zamanı son indekslemeye bu kadar yakın
/// dosya, boyutu ve zamanı aynı olsa da yeniden okunur.
pub const RACY_MS: u64 = 2_000;
/// Canlı vektörü bu sayıdan az olan dilim seyrektir; canlı vektörleri sonraki
/// toplu yazmada yeni dilime taşınır (§11.2 sıkıştırma).
const SEYREK_DILIM: usize = TOPLU_PARCA / 2;
const KOTA_ILETISI: &str = "depo kotası doldu; manifestin globlarını daraltmak gerekebilir";

/// Eski dilim konumu → yeni dilim konumu.
type Tasima = BTreeMap<(u64, u32), (u64, u32)>;

/// Güncellemenin sonucu.
#[derive(Debug, Clone, PartialEq)]
pub struct IndeksDurumu {
    /// Diskteki (son) `meta`.
    pub meta: Meta,
    /// Listeden atlananlar çıktıktan sonra indekslenmesi gereken dosya sayısı.
    pub dosya_sayisi: u64,
    /// `meta`'da bulunan (tam indeksli) dosya sayısı.
    pub indeksli: u64,
    /// Bu çağrıda yeniden okunup indekse yazılan dosya sayısı.
    pub guncellenen: u64,
    /// İndeksleme yarıda kaldıysa nedeni (hız sınırı, kota); sonuçta
    /// gösterilir. Tamamsa `None`.
    pub eksik: Option<String>,
}

/// Dosya listesini `meta` ile karşılaştırır, değişenleri yeniden parçalar ve
/// gömer, silinenleri düşürür. Kesintiye dayanıklı yazma sırası §11.2'dedir.
/// İptal edilirse `Err(Hata::IptalEdildi)`; o ana kadar biten toplu istekler
/// depoda kalır.
pub fn guncelle(o: &dyn Ortam) -> Result<IndeksDurumu, Hata> {
    let simdi = o.simdi_ms();
    let (mut disk, bozuk_vardi) = meta_yukle(o)?;
    let mut calisan = disk.clone();
    artiklari_temizle(o, &mut calisan)?;

    let listelenen: Vec<DosyaBilgisi> = o
        .dosya_listele("**/*")?
        .into_iter()
        .filter(|d| !atlanir(d))
        .collect();
    let kume: BTreeSet<&str> = listelenen.iter().map(|d| d.yol.as_str()).collect();
    calisan
        .dosyalar
        .retain(|yol, _| kume.contains(yol.as_str()));
    calisan
        .bekleyen
        .retain(|yol, _| kume.contains(yol.as_str()));

    let mut eksik = None;
    let mut okunamayan = BTreeSet::new();
    let mut kuyruk = degisenleri_oku(o, &calisan, &listelenen, &mut okunamayan, &mut eksik)?;
    for yol in &okunamayan {
        calisan.dosyalar.remove(yol);
        calisan.bekleyen.remove(yol);
    }
    // Eski tamamlanmış sürüme dönmüş dosyanın yarım güncellemesi düşer.
    // Okuma sınırında henüz okunmamış dosyaların ilerlemesi korunur.
    let okunmus: BTreeSet<&str> = kuyruk.iter().map(|d| d.yol.as_str()).collect();
    if eksik.is_none() {
        calisan
            .bekleyen
            .retain(|yol, _| okunmus.contains(yol.as_str()));
    }

    let toplam = kuyruk.len() as u64;
    let mut tamamlanan = 0u64;
    let mut guncellenen = 0u64;
    'toplu: while !kuyruk.is_empty() {
        let konumlar = ozet_konumlari(&calisan);
        let mut grup = Vec::new();
        let mut yeni_metinler: Vec<String> = Vec::new();
        let mut yeni_bayt =
            kalfa_plugin_sdk::gomme_govdesi(crate::ortam::GOMME_MODELI, &[], BOYUT as u32).len();
        while let Some(okunan) = kuyruk.front() {
            let (yeni_say, bayt) = yeni_olcu(okunan, &konumlar);
            if !grup.is_empty()
                && (yeni_metinler.len() + yeni_say > TOPLU_PARCA || yeni_bayt + bayt > TOPLU_BAYT)
            {
                break;
            }
            let Some(mut okunan) = kuyruk.pop_front() else {
                break;
            };
            let mut say = yeni_metinler.len();
            let mut alinacak = 0;
            for h in &okunan.parcalar {
                if !konumlar.contains_key(&h.ozet) {
                    let bayt = metin_bayti(&h.parca.metin);
                    if say == TOPLU_PARCA || yeni_bayt + bayt > TOPLU_BAYT {
                        break;
                    }
                    say += 1;
                    yeni_bayt += bayt;
                }
                alinacak += 1;
            }
            if alinacak == 0 && !okunan.parcalar.is_empty() {
                return Err(Hata::Diger("parça toplu istek sınırına sığmıyor".into()));
            }
            let kalan = okunan.parcalar.split_off(alinacak);
            let tamam = kalan.is_empty();
            let devam = (!tamam).then(|| Okunan {
                yol: okunan.yol.clone(),
                boyut: okunan.boyut,
                degisme: okunan.degisme,
                parcalar: kalan,
                devam: true,
            });
            grup.push(grupla(okunan, tamam, &konumlar, &mut yeni_metinler));
            if let Some(devam) = devam {
                kuyruk.push_front(devam);
                break;
            }
        }
        if grup.is_empty() {
            continue 'toplu;
        }
        let sayi = grup.iter().filter(|g| g.tamam).count() as u64;

        if yeni_metinler.is_empty() {
            // Yalnızca depodaki vektörleri yeniden kullanan dosyalar: yeni
            // vektör yazılmaz; kayıtları bir sonraki meta yazmasında girer.
            let n = calisan.siradaki_dilim;
            for g in grup {
                grubu_yerlestir(&mut calisan, g, n, &Tasima::new())?;
            }
            guncellenen += sayi;
            tamamlanan += sayi;
            continue 'toplu;
        }

        if o.iptal_istendi() {
            return Err(Hata::IptalEdildi);
        }
        o.ilerleme(
            tamamlanan,
            Some(toplam),
            Some(format!("{toplam} dosyanın {tamamlanan} tanesi indekslendi")),
        );
        let vektorler = match o.gomme(&yeni_metinler) {
            Ok(v) => v,
            Err(Hata::SinirAsildi(m)) => {
                eksik = Some(format!("EVREN sınırı: {m}"));
                break 'toplu;
            }
            Err(e) => return Err(e),
        };
        if vektorler.len() != yeni_metinler.len() {
            return Err(Hata::Diger("gömme yanıtı metin sayısıyla uyuşmuyor".into()));
        }
        if !toplu_yaz(
            o,
            &mut calisan,
            &mut disk,
            grup,
            vektorler,
            simdi,
            &mut eksik,
        )? {
            break 'toplu;
        }
        guncellenen += sayi;
        tamamlanan += sayi;
    }

    // Bekleyen değişiklikler (silmeler, yalnızca yeniden kullanılan dosyalar,
    // artık temizliğinden kalan silinecekler) tek meta yazmasıyla kaydedilir.
    if calisan.dosyalar != disk.dosyalar
        || calisan.bekleyen != disk.bekleyen
        || bozuk_vardi
        || !disk.silinecekler.is_empty()
    {
        let mut yeni = calisan.clone();
        yeni.son_indeksleme = simdi;
        yeni.silinecekler = dilim_kumesi(&disk)
            .difference(&dilim_kumesi(&calisan))
            .copied()
            .collect();
        meta_yaz(o, &mut calisan, &mut disk, yeni, &mut eksik)?;
    }

    let indeksli = disk
        .dosyalar
        .keys()
        .filter(|yol| !disk.bekleyen.contains_key(*yol))
        .count() as u64;
    Ok(IndeksDurumu {
        meta: disk,
        dosya_sayisi: listelenen.len() as u64,
        indeksli,
        guncellenen,
        eksik,
    })
}

/// `meta`'nın gösterdiği bütün vektörler: (yol, dosyadaki parça sırası,
/// vektör). Dilimler birer kez okunur.
pub fn vektorleri_yukle(
    o: &dyn Ortam,
    meta: &Meta,
) -> Result<Vec<(String, usize, Vec<f32>)>, Hata> {
    let mut dilimler: BTreeMap<u64, Vec<Vec<f32>>> = BTreeMap::new();
    let mut sonuc = Vec::new();
    for (yol, kayit) in &meta.dosyalar {
        for (sira, p) in kayit.parcalar.iter().enumerate() {
            let dilim = match dilimler.entry(p.dilim) {
                Entry::Occupied(e) => e.into_mut(),
                Entry::Vacant(e) => {
                    let veri = o
                        .depo_oku(&dilim_anahtari(p.dilim))?
                        .ok_or_else(|| bozuk("vektör dilimi eksik"))?;
                    let vektorler =
                        dilim_coz(&veri, BOYUT).map_err(|_| bozuk("vektör dilimi bozuk"))?;
                    e.insert(vektorler)
                }
            };
            let vektor = dilim
                .get(p.sira as usize)
                .ok_or_else(|| bozuk("vektör yeri yok"))?;
            sonuc.push((yol.clone(), sira, vektor.clone()));
        }
    }
    Ok(sonuc)
}

fn bozuk(ayrinti: &str) -> Hata {
    Hata::Diger(format!("indeks bozuk: {ayrinti}"))
}

/// Dosya indekslenmeyecekler arasında mı (§11.2: büyük dosya, kilit dosyaları,
/// küçültülmüş betikler).
fn atlanir(d: &DosyaBilgisi) -> bool {
    let ad = d.yol.rsplit('/').next().unwrap_or(d.yol.as_str());
    d.boyut > AZAMI_DOSYA
        || ad.ends_with(".lock")
        || ad.ends_with(".min.js")
        || ad == "package-lock.json"
        || ad == "pnpm-lock.yaml"
}

/// Dosya meta'dan farklıysa ya da "racy clean" penceresindeyse yeniden
/// okunmalı. Meta'da olmayan dosya her zaman okunur.
fn degismis_mi(meta: &Meta, d: &DosyaBilgisi) -> bool {
    if meta.bekleyen.contains_key(&d.yol) {
        return true;
    }
    match meta.dosyalar.get(&d.yol) {
        None => true,
        Some(kayit) => {
            kayit.boyut != d.boyut
                || kayit.degisme != d.degisme
                || d.degisme.abs_diff(meta.son_indeksleme) <= RACY_MS
        }
    }
}

/// Kayıt, yeniden okunan dosyayla bütünüyle aynıysa (boyut, zaman ve her
/// parçanın satırı, bayt aralığı, özeti) dosya değişmemiştir.
fn ayni_mi(kayit: &DosyaKaydi, d: &DosyaBilgisi, parcalar: &[Hazir]) -> bool {
    kayit.boyut == d.boyut
        && kayit.degisme == d.degisme
        && kayit.parcalar.len() == parcalar.len()
        && kayit.parcalar.iter().zip(parcalar).all(|(k, h)| {
            k.satir_bas == h.parca.satir_bas
                && k.satir_son == h.parca.satir_son
                && k.bas_bayt == h.parca.bas_bayt as u64
                && k.son_bayt == h.parca.son_bayt as u64
                && k.ozet == h.ozet
        })
}

/// `meta`'yı depodan okur. Yoksa boş indeks döner (`false`). Uymuyorsa ya da
/// bozuksa eski dilimleri temizler, boş indeks döner ve `true` verir: bu
/// durumda `meta` yeniden yazılmalıdır.
fn meta_yukle(o: &dyn Ortam) -> Result<(Meta, bool), Hata> {
    let Some(ham) = o.depo_oku(META)? else {
        return Ok((Meta::yeni(), false));
    };
    if let Some(m) = Meta::coz(&ham) {
        return Ok((m, false));
    }
    match serde_json::from_slice::<Meta>(&ham) {
        Ok(eski) => {
            for n in 0..=eski.siradaki_dilim {
                o.depo_sil(&dilim_anahtari(n))?;
            }
        }
        Err(_) => {
            let mut n = 0;
            while o.depo_oku(&dilim_anahtari(n))?.is_some() {
                o.depo_sil(&dilim_anahtari(n))?;
                n += 1;
            }
        }
    }
    Ok((Meta::yeni(), true))
}

/// Kesilmiş yazmaların artıkları: `siradaki_dilim`'den başlayıp var olan
/// dilimler (sahipsiz) ve `silinecekler` silinir.
fn artiklari_temizle(o: &dyn Ortam, meta: &mut Meta) -> Result<(), Hata> {
    let mut n = meta.siradaki_dilim;
    while o.depo_oku(&dilim_anahtari(n))?.is_some() {
        o.depo_sil(&dilim_anahtari(n))?;
        n += 1;
    }
    for &d in &meta.silinecekler {
        o.depo_sil(&dilim_anahtari(d))?;
    }
    meta.silinecekler.clear();
    Ok(())
}

/// Okunmuş bir dosyanın parçası ve özeti.
struct Hazir {
    parca: Parca,
    ozet: String,
}

/// Okunmuş, yerleştirilmeyi bekleyen değişen dosya.
struct Okunan {
    yol: String,
    boyut: u64,
    degisme: u64,
    parcalar: Vec<Hazir>,
    devam: bool,
}

/// Değişen dosyaları okur ve parçalarının özetini çıkarır. Okunamayanlar
/// `okunamayan`a girer (kaydı düşer). Okuma sınıra çarparsa `eksik` dolar ve
/// kalan dosyalar okunmaz.
fn degisenleri_oku(
    o: &dyn Ortam,
    calisan: &Meta,
    dosyalar: &[DosyaBilgisi],
    okunamayan: &mut BTreeSet<String>,
    eksik: &mut Option<String>,
) -> Result<VecDeque<Okunan>, Hata> {
    let mut kuyruk = VecDeque::new();
    for d in dosyalar {
        if !degismis_mi(calisan, d) {
            continue;
        }
        let icerik = match o.dosya_oku(&d.yol) {
            Ok(icerik) => icerik,
            Err(Hata::Bulunamadi(_) | Hata::IzinYok(_) | Hata::Diger(_)) => {
                okunamayan.insert(d.yol.clone());
                continue;
            }
            Err(Hata::SinirAsildi(m)) => {
                *eksik = Some(format!("okuma sınırı: {m}"));
                break;
            }
            Err(e) => return Err(e),
        };
        let parcalar: Vec<Hazir> = parca::parcala(&icerik)
            .into_iter()
            .map(|p| {
                let ozet = parca::ozet(&p.metin);
                Hazir { parca: p, ozet }
            })
            .collect();
        if calisan
            .dosyalar
            .get(&d.yol)
            .is_some_and(|k| ayni_mi(k, d, &parcalar))
        {
            continue;
        }
        kuyruk.push_back(Okunan {
            yol: d.yol.clone(),
            boyut: d.boyut,
            degisme: d.degisme,
            parcalar,
            devam: false,
        });
    }
    Ok(kuyruk)
}

/// Toplu isteğin bir parçasının vektörü: yeni gömülecek (sırası) ya da
/// depoda zaten olan (dilim, sıra).
#[derive(Clone, Copy)]
enum Kaynak {
    Yeni(usize),
    Eski(u64, u32),
}

/// Toplu isteğe giren bir dosya; parçaları henüz yerleşmemiş.
struct GrupDosya {
    yol: String,
    boyut: u64,
    degisme: u64,
    parcalar: Vec<(ParcaKaydi, Kaynak)>,
    devam: bool,
    tamam: bool,
}

/// Okunmuş dosyayı toplu isteğe yerleştirir: özeti bilinen parça depodaki
/// vektörünü kullanır, bilinmeyen metni `yeni_metinler`e eklenir.
fn grupla(
    okunan: Okunan,
    tamam: bool,
    konumlar: &BTreeMap<String, (u64, u32)>,
    yeni_metinler: &mut Vec<String>,
) -> GrupDosya {
    let parcalar = okunan
        .parcalar
        .into_iter()
        .map(|mut h| {
            let kaynak = match konumlar.get(&h.ozet) {
                Some(&(d, s)) => Kaynak::Eski(d, s),
                None => {
                    yeni_metinler.push(std::mem::take(&mut h.parca.metin));
                    Kaynak::Yeni(yeni_metinler.len() - 1)
                }
            };
            let kayit = ParcaKaydi {
                satir_bas: h.parca.satir_bas,
                satir_son: h.parca.satir_son,
                bas_bayt: h.parca.bas_bayt as u64,
                son_bayt: h.parca.son_bayt as u64,
                ozet: h.ozet,
                dilim: 0,
                sira: 0,
            };
            (kayit, kaynak)
        })
        .collect();
    GrupDosya {
        yol: okunan.yol,
        boyut: okunan.boyut,
        degisme: okunan.degisme,
        parcalar,
        devam: okunan.devam,
        tamam,
    }
}

/// Dosyanın, depoda vektörü olmayan (gömülecek) parçalarının sayısı ve
/// bayt toplamı.
fn yeni_olcu(okunan: &Okunan, konumlar: &BTreeMap<String, (u64, u32)>) -> (usize, usize) {
    okunan
        .parcalar
        .iter()
        .filter(|h| !konumlar.contains_key(&h.ozet))
        .fold((0, 0), |(say, bayt), h| {
            (say + 1, bayt + metin_bayti(&h.parca.metin))
        })
}

/// JSON kaçışları, tırnaklar ve ayırıcı dahil istek gövdesindeki üst sınır.
fn metin_bayti(metin: &str) -> usize {
    serde_json::Value::String(metin.into()).to_string().len() + 1
}

/// Yarım dosya ayrı tutulur; son parça yazılınca tam dosya tablosuna geçer.
fn grubu_yerlestir(meta: &mut Meta, g: GrupDosya, n: u64, tasima: &Tasima) -> Result<(), Hata> {
    let devam = g.devam;
    let tamam = g.tamam;
    let (yol, mut kayit) = kayit_olustur(g, n, tasima);
    if devam {
        let mut onceki = meta
            .bekleyen
            .remove(&yol)
            .ok_or_else(|| bozuk("dosyanın önceki toplu parçaları eksik"))?;
        onceki.parcalar.append(&mut kayit.parcalar);
        kayit.parcalar = onceki.parcalar;
    }
    if tamam {
        meta.bekleyen.remove(&yol);
        meta.dosyalar.insert(yol, kayit);
    } else {
        meta.bekleyen.insert(yol, kayit);
    }
    Ok(())
}

/// Grup dosyasının kaydını kurar. Yeni parçalar `n` dilimindeki sıralarını
/// alır; taşınan eski konumlar `tasima`'ya göre yenisine çevrilir.
fn kayit_olustur(g: GrupDosya, n: u64, tasima: &Tasima) -> (String, DosyaKaydi) {
    let parcalar = g
        .parcalar
        .into_iter()
        .map(|(mut k, kaynak)| {
            let (dilim, sira) = match kaynak {
                Kaynak::Yeni(i) => (n, i as u32),
                Kaynak::Eski(d, s) => tasima.get(&(d, s)).copied().unwrap_or((d, s)),
            };
            k.dilim = dilim;
            k.sira = sira;
            k
        })
        .collect();
    (
        g.yol,
        DosyaKaydi {
            boyut: g.boyut,
            degisme: g.degisme,
            parcalar,
        },
    )
}

/// Bir toplu isteğin vektörlerini §11.2 sırasıyla yazar: önce `vektor/<n>`,
/// sonra `meta`, sonra ölü dilimler. Dönüş: `true` yazıldı; `false` depo
/// kotası doldu ve indeksleme durmalı.
fn toplu_yaz(
    o: &dyn Ortam,
    calisan: &mut Meta,
    disk: &mut Meta,
    grup: Vec<GrupDosya>,
    mut vektorler: Vec<Vec<f32>>,
    simdi: u64,
    eksik: &mut Option<String>,
) -> Result<bool, Hata> {
    let n = calisan.siradaki_dilim;
    let mut tasima = Tasima::new();
    for (dilim, siralar) in tasinacak_dilimler(calisan, TOPLU_PARCA.saturating_sub(vektorler.len()))
    {
        let veri = o
            .depo_oku(&dilim_anahtari(dilim))?
            .ok_or_else(|| bozuk("vektör dilimi eksik"))?;
        let dilim_vektorleri = dilim_coz(&veri, BOYUT).map_err(|_| bozuk("vektör dilimi bozuk"))?;
        for s in siralar {
            let vektor = dilim_vektorleri
                .get(s as usize)
                .ok_or_else(|| bozuk("vektör yeri yok"))?;
            tasima.insert((dilim, s), (n, vektorler.len() as u32));
            vektorler.push(vektor.clone());
        }
    }

    let mut yeni = calisan.clone();
    for kayit in yeni.dosyalar.values_mut().chain(yeni.bekleyen.values_mut()) {
        for p in &mut kayit.parcalar {
            if let Some(&(d, s)) = tasima.get(&(p.dilim, p.sira)) {
                p.dilim = d;
                p.sira = s;
            }
        }
    }
    for g in grup {
        grubu_yerlestir(&mut yeni, g, n, &tasima)?;
    }

    match o.depo_yaz(&dilim_anahtari(n), &dilim_kodla(&vektorler)) {
        Ok(()) => {}
        Err(Hata::SinirAsildi(_)) => {
            *eksik = Some(KOTA_ILETISI.into());
            return Ok(false);
        }
        Err(e) => return Err(e),
    }

    yeni.siradaki_dilim = n + 1;
    yeni.son_indeksleme = simdi;
    yeni.silinecekler = dilim_kumesi(disk)
        .difference(&dilim_kumesi(&yeni))
        .copied()
        .collect();
    meta_yaz(o, calisan, disk, yeni, eksik)
}

/// `yeni` meta'yı yazar; kabul edilince bellekteki ve diskteki kopyayı
/// eşitler, ardından `silinecekler`i siler. Kota dolduysa `Ok(false)` döner
/// ve hiçbir dilim silinmez.
fn meta_yaz(
    o: &dyn Ortam,
    calisan: &mut Meta,
    disk: &mut Meta,
    yeni: Meta,
    eksik: &mut Option<String>,
) -> Result<bool, Hata> {
    match o.depo_yaz(META, &yeni.kodla()) {
        Ok(()) => {}
        Err(Hata::SinirAsildi(_)) => {
            *eksik = Some(KOTA_ILETISI.into());
            return Ok(false);
        }
        Err(e) => return Err(e),
    }
    let olu = yeni.silinecekler.clone();
    *calisan = yeni.clone();
    *disk = yeni;
    for d in olu {
        o.depo_sil(&dilim_anahtari(d))?;
    }
    Ok(true)
}

/// Canlı vektörü `SEYREK_DILIM`'den az olan dilimleri, yer kaldıkça, bütün
/// canlı vektörleriyle birlikte seçer. Dilim numarasına göre dolaşır.
fn tasinacak_dilimler(meta: &Meta, mut yer: usize) -> Vec<(u64, Vec<u32>)> {
    let mut canli: BTreeMap<u64, BTreeSet<u32>> = BTreeMap::new();
    for kayit in meta.dosyalar.values().chain(meta.bekleyen.values()) {
        for p in &kayit.parcalar {
            canli.entry(p.dilim).or_default().insert(p.sira);
        }
    }
    let mut secilen = Vec::new();
    for (dilim, siralar) in canli {
        if siralar.len() >= SEYREK_DILIM || siralar.len() > yer {
            continue;
        }
        yer -= siralar.len();
        secilen.push((dilim, siralar.into_iter().collect()));
    }
    secilen
}

/// Özeti depoda zaten bulunan parçaların (dilim, sıra) konumu.
fn ozet_konumlari(meta: &Meta) -> BTreeMap<String, (u64, u32)> {
    let mut konumlar = BTreeMap::new();
    for kayit in meta.dosyalar.values().chain(meta.bekleyen.values()) {
        for p in &kayit.parcalar {
            konumlar.entry(p.ozet.clone()).or_insert((p.dilim, p.sira));
        }
    }
    konumlar
}

/// Kayıtların gösterdiği bütün dilimler.
fn dilim_kumesi(meta: &Meta) -> BTreeSet<u64> {
    meta.dosyalar
        .values()
        .chain(meta.bekleyen.values())
        .flat_map(|k| k.parcalar.iter().map(|p| p.dilim))
        .collect()
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::test_ortami::{SahteOrtam, sahte_vektor};

    fn meta_oku(o: &SahteOrtam) -> Meta {
        let ham = o.depo.borrow().get(META).cloned().expect("meta yok");
        Meta::coz(&ham).expect("meta geçerli olmalı")
    }

    /// Meta'daki her parçanın özeti dosya metnine uyar, vektörü metnin sahte
    /// vektörüne eşittir ve gösterilen her dilim depodadır.
    fn tutarli(o: &SahteOrtam) {
        let meta = meta_oku(o);
        let vektorler = vektorleri_yukle(o, &meta).expect("vektörler okunmalı");
        let mut beklenen = Vec::new();
        for (yol, kayit) in &meta.dosyalar {
            let icerik = o
                .dosyalar
                .borrow()
                .get(yol)
                .map(|(c, _)| c.clone())
                .expect("dosya yok");
            for (sira, p) in kayit.parcalar.iter().enumerate() {
                let metin = icerik
                    .get(p.bas_bayt as usize..p.son_bayt as usize)
                    .expect("bayt aralığı");
                assert_eq!(parca::ozet(metin), p.ozet, "{yol}#{sira} özeti");
                beklenen.push((yol.clone(), sira, sahte_vektor(metin)));
            }
        }
        assert_eq!(vektorler, beklenen);
        for (yol, kayit) in &meta.bekleyen {
            let icerik = o.dosyalar.borrow()[yol].0.clone();
            for p in &kayit.parcalar {
                let metin = &icerik[p.bas_bayt as usize..p.son_bayt as usize];
                assert_eq!(parca::ozet(metin), p.ozet);
                let ham = o.depo.borrow()[&dilim_anahtari(p.dilim)].clone();
                assert_eq!(
                    dilim_coz(&ham, BOYUT).unwrap()[p.sira as usize],
                    sahte_vektor(metin)
                );
            }
        }
        for p in meta
            .dosyalar
            .values()
            .chain(meta.bekleyen.values())
            .flat_map(|k| &k.parcalar)
        {
            assert!(
                o.depo.borrow().contains_key(&dilim_anahtari(p.dilim)),
                "gösterilen dilim depoda yok"
            );
        }
    }

    /// Meta'nın göstermediği vektör dilimi kalmamış olmalı.
    fn yetim_yok(o: &SahteOrtam) {
        let gosterilen = dilim_kumesi(&meta_oku(o));
        let anahtarlar: Vec<String> = o
            .depo
            .borrow()
            .keys()
            .filter(|a| a.starts_with("vektor/"))
            .cloned()
            .collect();
        for a in anahtarlar {
            let n: u64 = a["vektor/".len()..].parse().expect("dilim numarası");
            assert!(gosterilen.contains(&n), "yetim dilim: {a}");
        }
    }

    fn vektor_sayisi(o: &SahteOrtam) -> usize {
        o.depo
            .borrow()
            .keys()
            .filter(|a| a.starts_with("vektor/"))
            .count()
    }

    fn calistir(o: &SahteOrtam) -> IndeksDurumu {
        guncelle(o).expect("indeksleme başarılı olmalı")
    }

    fn parcalar_toplam(o: &SahteOrtam) -> usize {
        o.dosyalar
            .borrow()
            .values()
            .map(|(c, _)| parca::parcala(c).len())
            .sum()
    }

    /// `on` önekli `aralik` satırları; her satır özgün olsun diye numaralı.
    fn satirlar(on: &str, aralik: std::ops::Range<usize>) -> String {
        aralik.map(|i| format!("{on} {i}\n")).collect()
    }

    /// Yaklaşık 200 parçalık üç dosya: her biri ayrı bir toplu istek.
    fn uc_uzun_dosya() -> SahteOrtam {
        let o = SahteOrtam::yeni();
        o.dosya("a.txt", &satirlar("a", 0..10_000));
        o.dosya("b.txt", &satirlar("b", 0..10_000));
        o.dosya("c.txt", &satirlar("c", 0..10_000));
        o
    }

    /// Dosya sınırının altında, fakat toplu parça sınırının üstünde.
    fn cok_parcali_dosya() -> SahteOrtam {
        let o = SahteOrtam::yeni();
        let metin: String = (0..300)
            .map(|i| format!("{i:04}{}\n", "a".repeat(844)))
            .collect();
        assert!(metin.len() as u64 <= AZAMI_DOSYA);
        assert_eq!(parca::parcala(&metin).len(), 299);
        o.dosya("uzun.txt", &metin);
        o
    }

    #[test]
    fn tek_dosya_birden_cok_toplu_istekle_tamamlanir() {
        let o = cok_parcali_dosya();
        let durum = calistir(&o);
        assert_eq!(durum.indeksli, 1);
        assert_eq!(durum.guncellenen, 1);
        assert!(durum.eksik.is_none());
        assert!(durum.meta.bekleyen.is_empty());
        assert_eq!(
            o.gommeler.borrow().iter().map(Vec::len).collect::<Vec<_>>(),
            [256, 43]
        );
        tutarli(&o);
        yetim_yok(&o);
    }

    #[test]
    fn yarim_dosya_iptal_sonrasi_yalniz_kalan_parcalari_gomer() {
        let o = cok_parcali_dosya();
        o.iptal_sonrasi.set(Some(1));
        assert!(matches!(guncelle(&o), Err(Hata::IptalEdildi)));
        let meta = meta_oku(&o);
        assert!(meta.dosyalar.is_empty());
        assert_eq!(meta.bekleyen["uzun.txt"].parcalar.len(), 256);
        tutarli(&o);
        o.iptal_sonrasi.set(None);
        let durum = calistir(&o);
        assert_eq!(durum.indeksli, 1);
        assert_eq!(o.gomulenler().len(), 299);
        assert_eq!(o.gommeler.borrow()[1].len(), 43);
        tutarli(&o);
        yetim_yok(&o);
    }

    #[test]
    fn yarim_dosya_hiz_sinirinda_tam_sayilmaz_ve_silinirse_temizlenir() {
        let o = cok_parcali_dosya();
        *o.gomme_hatasi.borrow_mut() = Some((2, Hata::SinirAsildi("429".into())));
        let durum = calistir(&o);
        assert_eq!(durum.indeksli, 0);
        assert_eq!(durum.guncellenen, 0);
        assert!(durum.eksik.is_some());
        assert_eq!(durum.meta.bekleyen["uzun.txt"].parcalar.len(), 256);
        tutarli(&o);
        yetim_yok(&o);
        o.sil("uzun.txt");
        let durum = calistir(&o);
        assert!(durum.meta.bekleyen.is_empty());
        assert_eq!(vektor_sayisi(&o), 0);
    }

    #[test]
    fn yarim_dosyanin_her_yazma_adiminda_kesilme_ve_kota() {
        for n in 1..=4 {
            for hata in [
                Hata::Diger("çökme".into()),
                Hata::SinirAsildi("kota".into()),
            ] {
                let o = cok_parcali_dosya();
                *o.yazma_hatasi.borrow_mut() = Some((n, hata.clone()));
                let sonuc = guncelle(&o);
                match hata {
                    Hata::SinirAsildi(_) => assert!(sonuc.unwrap().eksik.is_some()),
                    _ => assert!(sonuc.is_err()),
                }
                if n > 2 {
                    assert_eq!(meta_oku(&o).bekleyen["uzun.txt"].parcalar.len(), 256);
                    tutarli(&o);
                }
                *o.yazma_hatasi.borrow_mut() = None;
                assert_eq!(calistir(&o).indeksli, 1, "yazma {n}");
                tutarli(&o);
                yetim_yok(&o);
            }
        }
    }

    #[test]
    fn yarim_guncelleme_eski_surumune_donerse_artiklari_silinir() {
        let o = cok_parcali_dosya();
        let uzun = o.dosyalar.borrow()["uzun.txt"].0.clone();
        let eski = "eski sürüm\n";
        o.dosya("uzun.txt", eski);
        calistir(&o);
        let eski_zaman = o.simdi.get();
        o.ilerlet(10_000);
        o.dosya("uzun.txt", &uzun);
        *o.gomme_hatasi.borrow_mut() = Some((3, Hata::SinirAsildi("429".into())));
        let durum = calistir(&o);
        assert_eq!(durum.indeksli, 0);
        assert!(durum.meta.dosyalar.contains_key("uzun.txt"));
        assert_eq!(durum.meta.bekleyen["uzun.txt"].parcalar.len(), 256);
        o.dosya_zamanla("uzun.txt", eski, eski_zaman);
        assert_eq!(calistir(&o).indeksli, 1);
        assert!(meta_oku(&o).bekleyen.is_empty());
        tutarli(&o);
        yetim_yok(&o);
    }

    #[test]
    fn yarim_dosya_degismisse_yeni_ozetlerle_tamamlanir() {
        let o = cok_parcali_dosya();
        o.iptal_sonrasi.set(Some(1));
        assert!(guncelle(&o).is_err());
        let mut yeni = o.dosyalar.borrow()["uzun.txt"].0.clone();
        yeni.replace_range(..4, "yeni");
        o.dosya("uzun.txt", &yeni);
        o.iptal_sonrasi.set(None);
        assert_eq!(calistir(&o).indeksli, 1);
        assert_eq!(o.gommeler.borrow()[1].len(), 44);
        tutarli(&o);
        yetim_yok(&o);
    }

    #[test]
    fn toplu_bayt_siniri_json_kacislarini_da_sayar() {
        let o = SahteOrtam::yeni();
        let metin: String = (0..900)
            .map(|i| format!("{i:04}{}\n", "\u{1}".repeat(255)))
            .collect();
        assert!(metin.len() as u64 <= AZAMI_DOSYA);
        o.dosya("kacis.txt", &metin);
        assert_eq!(calistir(&o).indeksli, 1);
        assert!(o.gommeler.borrow().len() > 1);
        for metinler in o.gommeler.borrow().iter() {
            assert!(metinler.len() <= TOPLU_PARCA);
            assert!(
                kalfa_plugin_sdk::gomme_govdesi(crate::ortam::GOMME_MODELI, metinler, BOYUT as u32)
                    .len()
                    <= TOPLU_BAYT
            );
        }
        tutarli(&o);
        yetim_yok(&o);
    }

    #[test]
    fn onceki_meta_bekleyen_alani_olmadan_okunur() {
        let ham = Meta::yeni().kodla();
        assert!(!String::from_utf8_lossy(&ham).contains("bekleyen"));
        assert_eq!(Meta::coz(&ham), Some(Meta::yeni()));
    }

    #[test]
    fn ilk_indeksleme_her_parcayi_bir_kez_gomer() {
        let o = SahteOrtam::yeni();
        o.dosya("a.rs", &satirlar("a", 0..130));
        o.dosya("b.md", "# Başlık\nmetin\n");
        o.dosya("c.txt", "");
        let mut beklenen: Vec<String> = o
            .dosyalar
            .borrow()
            .values()
            .flat_map(|(c, _)| parca::parcala(c))
            .map(|p| p.metin)
            .collect();
        let durum = calistir(&o);
        assert_eq!(durum.indeksli, 3);
        assert_eq!(durum.dosya_sayisi, 3);
        assert!(durum.eksik.is_none());
        let mut gomulen = o.gomulenler();
        gomulen.sort();
        beklenen.sort();
        assert_eq!(gomulen, beklenen);
        tutarli(&o);
        yetim_yok(&o);
    }

    #[test]
    fn degisiklik_yokken_ikinci_cagri_yazmaz() {
        let o = uc_uzun_dosya();
        calistir(&o);
        let yazma = o.yazma_denemesi.get();
        let gomme = o.gommeler.borrow().len();
        let durum = calistir(&o);
        assert_eq!(o.yazma_denemesi.get(), yazma);
        assert_eq!(o.gommeler.borrow().len(), gomme);
        assert_eq!(durum.guncellenen, 0);
    }

    #[test]
    fn degisen_dosyada_yalniz_degisen_parca_gomulur() {
        let o = SahteOrtam::yeni();
        o.dosya("a.rs", &satirlar("a", 0..130));
        calistir(&o);
        o.ilerlet(10_000);
        // Yalnızca ilk satır değişti: 1. pencere değişir, 2. ve 3. aynı kalır.
        let yeni = format!("x 0\n{}", satirlar("a", 1..130));
        o.dosya("a.rs", &yeni);
        let once = o.gommeler.borrow().len();
        calistir(&o);
        let gommeler = o.gommeler.borrow();
        assert_eq!(gommeler.len(), once + 1);
        let son = gommeler.last().expect("gömme yapılmalı");
        assert_eq!(son.len(), 1);
        assert_eq!(son[0], parca::parcala(&yeni)[0].metin);
        drop(gommeler);
        tutarli(&o);
        yetim_yok(&o);
    }

    #[test]
    fn ayni_boyutlu_racy_yeniden_yazim_yakalanir() {
        let o = SahteOrtam::yeni();
        o.dosya("a.txt", "aaaa\n");
        calistir(&o);
        // Aynı boyut ve aynı zaman damgası: son indekslemeyle aynı ms.
        o.dosya("a.txt", "bbbb\n");
        let durum = calistir(&o);
        assert_eq!(durum.guncellenen, 1);
        assert_eq!(
            o.gommeler.borrow().last().cloned(),
            Some(vec!["bbbb\n".to_string()])
        );
        tutarli(&o);
    }

    #[test]
    fn racy_penceresi_disindaki_ayni_boyutlu_yazim_atlanir() {
        // Bilinen sınır (§11.2): zaman damgası eski değere geri alınan dosya
        // yakalanmaz. Bu yüzden burada tutarlılık denetimi yapılmaz.
        let o = SahteOrtam::yeni();
        o.dosya("a.txt", "aaaa\n");
        calistir(&o);
        o.ilerlet(10_000);
        o.dosya("b.txt", "bu yeni dosya\n");
        calistir(&o);
        o.dosya_zamanla("a.txt", "bbbb\n", 1_700_000_000_000);
        let once = o.gommeler.borrow().len();
        calistir(&o);
        assert_eq!(o.gommeler.borrow().len(), once);
    }

    #[test]
    fn silinen_dosyalar_duser_olu_dilim_silinir() {
        let o = SahteOrtam::yeni();
        o.dosya("a.rs", "fn a() {}\n");
        o.dosya("b.rs", "fn b() {}\n");
        calistir(&o);
        o.ilerlet(5_000);
        o.sil("a.rs");
        let durum = calistir(&o);
        assert_eq!(durum.indeksli, 1);
        tutarli(&o);
        yetim_yok(&o);
        o.ilerlet(5_000);
        o.sil("b.rs");
        let durum = calistir(&o);
        assert_eq!(durum.indeksli, 0);
        yetim_yok(&o);
        assert_eq!(vektor_sayisi(&o), 0);
    }

    #[test]
    fn buyuk_ve_kilit_dosyalari_atlanir() {
        let o = SahteOrtam::yeni();
        o.dosya("buyuk.txt", &"a".repeat(300 * 1024));
        o.dosya("Cargo.lock", "x\n");
        o.dosya("package-lock.json", "{}\n");
        o.dosya("pnpm-lock.yaml", "a: 1\n");
        o.dosya("uygulama.min.js", "var a=1;\n");
        o.dosya("ok.rs", "fn main() {}\n");
        let durum = calistir(&o);
        assert_eq!(durum.dosya_sayisi, 1);
        assert_eq!(durum.indeksli, 1);
        let beklenen: Vec<String> = parca::parcala("fn main() {}\n")
            .into_iter()
            .map(|p| p.metin)
            .collect();
        assert_eq!(o.gomulenler(), beklenen);
        tutarli(&o);
    }

    #[test]
    fn iptal_sonrasi_yalniz_kalanlar_gomulur() {
        let o = uc_uzun_dosya();
        let toplam = parcalar_toplam(&o);
        o.iptal_sonrasi.set(Some(1));
        let sonuc = guncelle(&o);
        assert!(matches!(sonuc, Err(Hata::IptalEdildi)));
        assert_eq!(o.gommeler.borrow().len(), 1);
        o.iptal_sonrasi.set(None);
        let durum = calistir(&o);
        assert_eq!(durum.indeksli, 3);
        assert!(durum.eksik.is_none());
        assert_eq!(o.gommeler.borrow().len(), 3);
        assert_eq!(o.gomulenler().len(), toplam);
        tutarli(&o);
        yetim_yok(&o);
    }

    #[test]
    fn hiz_siniri_kismi_sonuc_sonra_tamamlanir() {
        let o = uc_uzun_dosya();
        *o.gomme_hatasi.borrow_mut() = Some((2, Hata::SinirAsildi("hız sınırı".into())));
        let ilk = calistir(&o);
        assert!(
            ilk.eksik
                .as_deref()
                .is_some_and(|m| m.contains("EVREN sınırı"))
        );
        assert_eq!(ilk.indeksli, 1);
        tutarli(&o);
        *o.gomme_hatasi.borrow_mut() = None;
        let son = calistir(&o);
        assert_eq!(son.indeksli, 3);
        assert!(son.eksik.is_none());
        // İlk çağrının 2. isteği (başarısız) + ikinci çağrının 2 isteği.
        assert_eq!(o.gommeler.borrow().len(), 4);
        tutarli(&o);
        yetim_yok(&o);
    }

    #[test]
    fn depo_kotasinda_durur_tutarli_kalir() {
        let o = uc_uzun_dosya();
        // Yazmalar: a vektör, a meta, b vektör, b meta, c vektör (5.).
        *o.yazma_hatasi.borrow_mut() = Some((5, Hata::SinirAsildi("kota".into())));
        let ilk = calistir(&o);
        assert_eq!(ilk.eksik.as_deref(), Some(KOTA_ILETISI));
        assert_eq!(ilk.indeksli, 2);
        tutarli(&o);
        *o.yazma_hatasi.borrow_mut() = None;
        let son = calistir(&o);
        assert_eq!(son.indeksli, 3);
        assert!(son.eksik.is_none());
        tutarli(&o);
        yetim_yok(&o);
    }

    #[test]
    fn her_yazma_adiminda_kesilme_sonra_tutarlilik() {
        let referans = uc_uzun_dosya();
        calistir(&referans);
        let k = referans.yazma_denemesi.get();
        assert!(k >= 4, "yazma sayısı {k}");
        for n in 1..=k {
            let o = uc_uzun_dosya();
            *o.yazma_hatasi.borrow_mut() = Some((n, Hata::Diger("çökme".into())));
            assert!(guncelle(&o).is_err(), "yazma {n} kesilmeliydi");
            *o.yazma_hatasi.borrow_mut() = None;
            let durum = calistir(&o);
            assert_eq!(durum.indeksli, 3, "yazma {n}");
            assert!(durum.eksik.is_none(), "yazma {n}");
            tutarli(&o);
            yetim_yok(&o);
        }
    }

    #[test]
    fn sikistirma_dilim_sayisini_sinirli_tutar() {
        let o = SahteOrtam::yeni();
        for i in 0..20 {
            o.dosya(&format!("f{i}.txt"), &format!("dosya {i}\n"));
        }
        calistir(&o);
        for tur in 0..20 {
            o.ilerlet(10_000);
            o.dosya(&format!("f{}.txt", tur % 20), &format!("değişen {tur}\n"));
            let durum = calistir(&o);
            assert!(durum.eksik.is_none());
        }
        let dilimler = vektor_sayisi(&o);
        assert!(dilimler <= 2, "dilim sayısı {dilimler}");
        tutarli(&o);
        yetim_yok(&o);
    }

    #[test]
    fn bozuk_meta_indeksi_bastan_kurar() {
        let o = SahteOrtam::yeni();
        o.dosya("a.rs", &satirlar("a", 0..130));
        calistir(&o);
        o.depo.borrow_mut().insert(META.into(), vec![0, 159, 1, 2]);
        let durum = calistir(&o);
        assert_eq!(durum.indeksli, 1);
        assert!(durum.eksik.is_none());
        tutarli(&o);
        yetim_yok(&o);
    }
}
