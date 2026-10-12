//! Parçalama (aşama 4 tasarımı §11.2): satır pencereleri. Pencere en çok
//! `PENCERE_SATIR` satır ve `PENCERE_KARAKTER` karakterdir; ardışık pencereler
//! `ORTUSME` satır örtüşür. Tek satır `PENCERE_KARAKTER`'i aşarsa karakter
//! sınırından bölünür.

use sha2::{Digest, Sha256};

pub const PENCERE_SATIR: usize = 60;
pub const PENCERE_KARAKTER: usize = 2_400;
pub const ORTUSME: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parca {
    /// 1'den başlayan, iki uç dahil satır aralığı.
    pub satir_bas: u32,
    pub satir_son: u32,
    /// İçerikte bayt aralığı `[bas_bayt, son_bayt)`; `metin` tam olarak
    /// `&icerik[bas_bayt..son_bayt]`'tir.
    pub bas_bayt: usize,
    pub son_bayt: usize,
    pub metin: String,
}

/// Metnin SHA-256'sı, onaltılık.
pub fn ozet(metin: &str) -> String {
    Sha256::digest(metin.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Bir satır: 1'den başlayan numarası, içerikteki bayt aralığı ve karakter
/// sayısı (satır sonu dahil).
struct Satir {
    no: u32,
    bas: usize,
    son: usize,
    karakter: usize,
}

impl Satir {
    /// Tek başına bir pencereye sığmaz; karakter sınırından bölünür.
    fn uzun(&self) -> bool {
        self.karakter > PENCERE_KARAKTER
    }
}

/// İçeriği parçalara böler. Boş ya da yalnızca boşluk olan içerik parça
/// vermez.
pub fn parcala(icerik: &str) -> Vec<Parca> {
    let satirlar = satirlari_ayir(icerik);
    let mut parcalar = Vec::new();
    let mut bas = 0;
    while let Some(satir) = satirlar.get(bas) {
        if satir.uzun() {
            uzun_satiri_bol(icerik, satir, &mut parcalar);
            bas += 1;
            continue;
        }
        let son = pencere_sonu(&satirlar, bas);
        pencereyi_ekle(icerik, &satirlar[bas..son], &mut parcalar);
        // Son satıra ulaşan pencereden sonra durulur.
        let Some(sonraki) = satirlar.get(son) else {
            break;
        };
        bas = if sonraki.uzun() {
            // Uzun satır kendi parçalarıyla başlar; komşu pencerelere girmez.
            son
        } else {
            // ORTUSME satır örtüşür, ama örtüşme pencerenin yarısını geçmez:
            // karakter sınırına takılan kısa pencerelerde (uzun satırlar)
            // pencere bir satır ilerleyip aynı metni defalarca üretmesin.
            son.saturating_sub(ORTUSME.min((son - bas) / 2))
                .max(bas + 1)
        };
    }
    parcalar
}

/// Satırları içerikteki sırasıyla ayırır; her satır kendi sonunu (`\n`, varsa
/// önceki `\r` ile birlikte) taşır.
fn satirlari_ayir(icerik: &str) -> Vec<Satir> {
    let mut satirlar = Vec::new();
    let mut bas = 0;
    for (i, metin) in icerik.split_inclusive('\n').enumerate() {
        let son = bas + metin.len();
        satirlar.push(Satir {
            no: u32::try_from(i + 1).unwrap_or(u32::MAX),
            bas,
            son,
            karakter: metin.chars().count(),
        });
        bas = son;
    }
    satirlar
}

/// `bas`'tan başlayan pencerenin (dışlayıcı) bitişi. Pencere, `bas` dizinindeki
/// normal satırla başlar; uzun satır pencereye girmez.
fn pencere_sonu(satirlar: &[Satir], bas: usize) -> usize {
    let mut son = bas;
    let mut karakter = 0;
    while let Some(satir) = satirlar.get(son) {
        if satir.uzun()
            || son - bas == PENCERE_SATIR
            || karakter + satir.karakter > PENCERE_KARAKTER
        {
            break;
        }
        karakter += satir.karakter;
        son += 1;
    }
    son
}

/// Uzun satırı `PENCERE_KARAKTER` karakterlik parçalara böler. Parçalar
/// karakter sınırından kesilir ve hepsi satırın tek numarasını taşır.
fn uzun_satiri_bol(icerik: &str, satir: &Satir, parcalar: &mut Vec<Parca>) {
    let Some(metin) = icerik.get(satir.bas..satir.son) else {
        return;
    };
    // Her karakterin bayt konumu; sonuncusu metnin bayt uzunluğudur.
    let sinirlar: Vec<usize> = metin
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(metin.len()))
        .collect();
    let karakter_sayisi = sinirlar.len().saturating_sub(1);
    for bas_karakter in (0..karakter_sayisi).step_by(PENCERE_KARAKTER) {
        let son_karakter = bas_karakter
            .saturating_add(PENCERE_KARAKTER)
            .min(karakter_sayisi);
        let (Some(&cb), Some(&ce)) = (sinirlar.get(bas_karakter), sinirlar.get(son_karakter))
        else {
            break;
        };
        parca_ekle(
            icerik,
            parcalar,
            satir.no,
            satir.no,
            satir.bas + cb,
            satir.bas + ce,
        );
    }
}

/// Pencerenin ilk ve son satırından parçayı ekler.
fn pencereyi_ekle(icerik: &str, pencere: &[Satir], parcalar: &mut Vec<Parca>) {
    if let (Some(ilk), Some(sonuncu)) = (pencere.first(), pencere.last()) {
        parca_ekle(icerik, parcalar, ilk.no, sonuncu.no, ilk.bas, sonuncu.son);
    }
}

/// `[bas_bayt, son_bayt)` aralığını parça olarak ekler; yalnızca boşluktan
/// oluşuyorsa atlar.
fn parca_ekle(
    icerik: &str,
    parcalar: &mut Vec<Parca>,
    satir_bas: u32,
    satir_son: u32,
    bas_bayt: usize,
    son_bayt: usize,
) {
    let Some(metin) = icerik.get(bas_bayt..son_bayt) else {
        return;
    };
    if metin.trim().is_empty() {
        return;
    }
    parcalar.push(Parca {
        satir_bas,
        satir_son,
        bas_bayt,
        son_bayt,
        metin: metin.to_owned(),
    });
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Her parçanın metni, bayt aralığının içeriğiyle birebir aynı olmalı.
    fn tutarli(icerik: &str, parcalar: &[Parca]) {
        for p in parcalar {
            assert_eq!(p.metin, &icerik[p.bas_bayt..p.son_bayt]);
        }
    }

    fn aralıklar(parcalar: &[Parca]) -> Vec<(u32, u32)> {
        parcalar
            .iter()
            .map(|p| (p.satir_bas, p.satir_son))
            .collect()
    }

    fn satir_metni(n: usize) -> String {
        (1..=n).map(|i| format!("satır {i}\n")).collect()
    }

    #[test]
    fn bos_ve_bosluk_icerik_parca_vermez() {
        assert!(parcala("").is_empty());
        assert!(parcala("  \n\t\n   ").is_empty());
    }

    #[test]
    fn uc_satir_tek_pencere() {
        let icerik = "bir\niki\nüç\n";
        let p = parcala(icerik);
        assert_eq!(aralıklar(&p), [(1, 3)]);
        assert_eq!((p[0].bas_bayt, p[0].son_bayt), (0, icerik.len()));
        tutarli(icerik, &p);
    }

    #[test]
    fn tam_altmis_satir_tek_pencere() {
        let icerik = satir_metni(60);
        let p = parcala(&icerik);
        assert_eq!(aralıklar(&p), [(1, 60)]);
        tutarli(&icerik, &p);
    }

    #[test]
    fn yuz_otuz_satir_on_satir_ortusur() {
        let icerik = satir_metni(130);
        let p = parcala(&icerik);
        assert_eq!(aralıklar(&p), [(1, 60), (51, 110), (101, 130)]);
        // Her satır en az bir pencerede: kapsama eksiksiz.
        for satir in 1..=130 {
            assert!(
                p.iter()
                    .any(|x| x.satir_bas <= satir && satir <= x.satir_son)
            );
        }
        tutarli(&icerik, &p);
    }

    #[test]
    fn karakter_siniri_pencereyi_keser() {
        // Her satır 500 karakter: dört satır 2 000, beş satır 2 500 eder.
        let icerik = format!("{}\n", "a".repeat(499)).repeat(5);
        let p = parcala(&icerik);
        // Dört satırlık pencerede örtüşme iki satır.
        assert_eq!(aralıklar(&p), [(1, 4), (3, 5)]);
        assert!(
            p.iter()
                .all(|x| x.metin.chars().count() <= PENCERE_KARAKTER)
        );
        tutarli(&icerik, &p);
    }

    #[test]
    fn kisa_pencerede_ortusme_yariyi_gecmez() {
        // 290 karakterlik satırlar: pencere sekiz satır; her pencere dört
        // satır ilerler, 700 satır ~175 parça eder (her biri bir satır
        // ilerleseydi ~700 olurdu).
        let icerik = format!(
            "{}
",
            "k".repeat(289)
        )
        .repeat(700);
        let p = parcala(&icerik);
        assert!(p.len() < 200, "{}", p.len());
        assert!(p.windows(2).all(|w| w[1].satir_bas >= w[0].satir_bas + 4));
        tutarli(&icerik, &p);
    }

    #[test]
    fn uzun_satir_karakter_sinirindan_bolunur() {
        // 6 000 karakter, her biri 2 bayt: 2 400 + 2 400 + 1 200.
        let icerik = "çğışöü".repeat(1000);
        let p = parcala(&icerik);
        let uzunluklar: Vec<usize> = p.iter().map(|x| x.metin.chars().count()).collect();
        assert_eq!(uzunluklar, [2400, 2400, 1200]);
        assert_eq!(aralıklar(&p), [(1, 1), (1, 1), (1, 1)]);
        tutarli(&icerik, &p);
    }

    #[test]
    fn uzun_satirin_komsulari_ayri_pencerede() {
        let icerik = format!("önce\n{}\nsonra\n", "x".repeat(3000));
        let p = parcala(&icerik);
        assert_eq!(aralıklar(&p), [(1, 1), (2, 2), (2, 2), (3, 3)]);
        tutarli(&icerik, &p);
    }

    #[test]
    fn crlf_satir_sonu_metinde_kalir() {
        let icerik = "a\r\nb\r\nc";
        let p = parcala(icerik);
        assert_eq!(aralıklar(&p), [(1, 3)]);
        assert_eq!(p[0].metin, icerik);
        tutarli(icerik, &p);
    }

    #[test]
    fn son_satirda_yeni_satir_yok() {
        let icerik = "bir\nikinci";
        let p = parcala(icerik);
        assert_eq!(aralıklar(&p), [(1, 2)]);
        assert_eq!(p[0].metin, icerik);
        tutarli(icerik, &p);
    }

    #[test]
    fn her_pencere_ilerler_ve_durur() {
        // Her satır 1 200 karakter: pencere iki satır, örtüşme bir satır.
        let icerik = format!("{}\n", "ş".repeat(1199)).repeat(1000);
        let p = parcala(&icerik);
        assert!(p.windows(2).all(|w| w[0].satir_bas < w[1].satir_bas));
        assert_eq!(p.last().map(|x| x.satir_son), Some(1000));
        tutarli(&icerik, &p);
    }

    #[test]
    fn ozet_sha256_onaltilik() {
        assert_eq!(
            ozet("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
