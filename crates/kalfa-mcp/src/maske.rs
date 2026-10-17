//! MCP gizli değer doğrulaması, ham bayt maskelemesi ve sınırlı `stderr` halkası.
//!
//! Bu modül Görev 3 kapsamında üç garantiyi birlikte uygular:
//! - [`GizliDeger`]: en az 8 UTF-8 karakter (`chars().count() >= 8`), NUL baytı
//!   içeremez, `Debug` çıktısı sabit `GizliDeger([gizli])` biçimindedir ve
//!   `Serialize` özellikle uygulanmaz.
//! - [`maskele_bayt`] / [`maskele_metin`] / [`maskeli_utf8_kes`]: en uzundan en
//!   kısaya deterministik sıra ve soldan sağa örtüşmeyen tarama ile tam ve alt
//!   dize eşleşmelerini `[gizli]` ile değiştirir; kesme her zaman maskelemeden
//!   sonra ve UTF-8 karakter sınırında yapılır.
//! - [`StderrHalkasi`]: akan `stderr` baytlarını satır başına en çok
//!   `4 KiB + en_uzun_sir_bayt` penceresinde tutar; dev satırların taşan
//!   kuyruğunu sınırlı bloklarla tüketirken sınırda parçalanan ya da taşan
//!   bölgedeki sırları yakalar; son 50 satırı ve toplam 200 KiB üst sınırını
//!   korur; tanım veya sır kümesi değiştiğinde [`StderrHalkasi::temizle`] ile
//!   sıfırlanır.

use std::collections::VecDeque;
use std::fmt;

/// Maskelenen gizli değerlerin yerine yazılan sabit etiket.
pub const GIZLI_ETIKETI: &str = "[gizli]";

/// Maskeleme sonrasında kesilen satır veya metinlerin sonuna eklenen sabit etiket.
pub const KESILDI_ETIKETI: &str = "[kesildi]";

/// Ayarlar veya tanım güncellemelerinde silinen ortam değişkenleri için sabit yer tutucu.
pub const ENV_SILINDI_ETIKETI: &str = "[env silindi]";

/// Tek bir `stderr` satırı için maskeleme sonrası azami bayt sınırı (4 KiB).
pub const SATIR_SINIRI_BAYT: usize = 4 * 1024;

/// `stderr` halkasında tutulan azami satır sayısı.
pub const SON_SATIR_ADEDI: usize = 50;

/// `stderr` halkasında tutulan satırların toplam azami bayt sınırı (200 KiB).
pub const STDERR_TOPLAM_SINIR_BAYT: usize = SATIR_SINIRI_BAYT * SON_SATIR_ADEDI;

/// Gizli bir değerin taşıması gereken en az UTF-8 karakter sayısı.
pub const EN_AZ_GIZLI_KARAKTER: usize = 8;

const GIZLI_BAYT: &[u8] = GIZLI_ETIKETI.as_bytes();
const KESILDI_BAYT: &[u8] = KESILDI_ETIKETI.as_bytes();
const SENTETIK_ETIKETLER: [&[u8]; 2] = [GIZLI_BAYT, KESILDI_BAYT];
const RK_TABAN_1: u64 = 257;
const RK_MOD_1: u64 = 1_000_000_007;
const RK_TABAN_2: u64 = 263;
const RK_MOD_2: u64 = 1_000_000_009;

/// Gizli değer doğrulama hataları.
///
/// Hata değerleri hiçbir zaman aday gizli metni taşımaz; böylece `Debug` veya
/// `Display` üzerinden sır sızıntısı oluşmaz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaskeHatasi {
    /// Gizli değer 8 UTF-8 karakterden kısa (`chars().count() < 8`).
    CokKisa {
        karakter_sayisi: usize,
        en_az: usize,
    },
    /// Gizli değer NUL (`\0`) baytı içeriyor.
    NulBaytiVar,
}

impl fmt::Display for MaskeHatasi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CokKisa {
                karakter_sayisi,
                en_az,
            } => write!(
                f,
                "gizli değer en az {en_az} karakter olmalıdır (verilen: {karakter_sayisi})"
            ),
            Self::NulBaytiVar => f.write_str("gizli değer NUL baytı içeremez"),
        }
    }
}

impl std::error::Error for MaskeHatasi {}

/// Doğrulanmış gizli değer.
///
/// `Serialize` özellikle uygulanmaz; `Debug` çıktısı ham değeri asla göstermez.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct GizliDeger {
    deger: Box<str>,
    ozet_1: u64,
    ozet_2: u64,
    taban_kuvveti_1: u64,
    taban_kuvveti_2: u64,
}

impl GizliDeger {
    /// Yeni bir gizli değer doğrular ve oluşturur.
    pub fn yeni(deger: impl Into<String>) -> Result<Self, MaskeHatasi> {
        let deger = deger.into();
        if deger.as_bytes().contains(&0) {
            return Err(MaskeHatasi::NulBaytiVar);
        }
        let karakter_sayisi = deger.chars().count();
        if karakter_sayisi < EN_AZ_GIZLI_KARAKTER {
            return Err(MaskeHatasi::CokKisa {
                karakter_sayisi,
                en_az: EN_AZ_GIZLI_KARAKTER,
            });
        }
        let baytlar = deger.as_bytes();
        let (ozet_1, ozet_2) = yuvarlanan_ozetler(baytlar);
        let (taban_kuvveti_1, taban_kuvveti_2) = taban_kuvvetleri(baytlar.len());
        Ok(Self {
            deger: deger.into_boxed_str(),
            ozet_1,
            ozet_2,
            taban_kuvveti_1,
            taban_kuvveti_2,
        })
    }

    /// Ham bayt dilimini döner (yalnızca iç eşleştirme ve alt süreç ortam hazırlığı için).
    pub fn as_bytes(&self) -> &[u8] {
        self.deger.as_bytes()
    }

    /// Gizli değerin bayt uzunluğunu döner.
    pub fn bayt_uzunlugu(&self) -> usize {
        self.deger.len()
    }

    /// Gizli değerin UTF-8 karakter sayısını döner.
    pub fn karakter_sayisi(&self) -> usize {
        self.deger.chars().count()
    }

    /// Alt süreç ortamına veya kasaya yazmak için ham metin dilimini döner.
    pub fn expose_secret(&self) -> &str {
        &self.deger
    }
}

impl fmt::Debug for GizliDeger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GizliDeger([gizli])")
    }
}

fn yuvarlanan_ozetler(baytlar: &[u8]) -> (u64, u64) {
    let mut o1 = 0_u64;
    let mut o2 = 0_u64;
    for &b in baytlar {
        let deger = u64::from(b) + 1;
        o1 = (o1 * RK_TABAN_1 + deger) % RK_MOD_1;
        o2 = (o2 * RK_TABAN_2 + deger) % RK_MOD_2;
    }
    (o1, o2)
}

fn taban_kuvvetleri(uzunluk: usize) -> (u64, u64) {
    let mut k1 = 1_u64;
    let mut k2 = 1_u64;
    for _ in 0..uzunluk.saturating_sub(1) {
        k1 = (k1 * RK_TABAN_1) % RK_MOD_1;
        k2 = (k2 * RK_TABAN_2) % RK_MOD_2;
    }
    (k1, k2)
}

#[derive(Clone, Copy)]
struct SirliDesen<'a> {
    baytlar: &'a [u8],
    ozet_1: u64,
    ozet_2: u64,
    taban_kuvveti_1: u64,
    taban_kuvveti_2: u64,
}

impl<'a> From<&'a GizliDeger> for SirliDesen<'a> {
    fn from(deger: &'a GizliDeger) -> Self {
        Self {
            baytlar: deger.as_bytes(),
            ozet_1: deger.ozet_1,
            ozet_2: deger.ozet_2,
            taban_kuvveti_1: deger.taban_kuvveti_1,
            taban_kuvveti_2: deger.taban_kuvveti_2,
        }
    }
}

/// Sıralanmış ve tekilleştirilmiş sır dilimleri (en uzundan en kısaya).
fn sirali_sirlar(sirlar: &[GizliDeger]) -> Vec<SirliDesen<'_>> {
    let mut liste: Vec<SirliDesen<'_>> = sirlar
        .iter()
        .filter(|s| !s.as_bytes().is_empty())
        .map(SirliDesen::from)
        .collect();
    liste.sort_by(|a, b| {
        b.baytlar
            .len()
            .cmp(&a.baytlar.len())
            .then_with(|| a.baytlar.cmp(b.baytlar))
    });
    liste.dedup_by(|a, b| a.baytlar == b.baytlar);
    liste
}

fn en_uzun_sir_bayt(sirlar: &[SirliDesen<'_>]) -> usize {
    sirlar.first().map_or(0, |s| s.baytlar.len())
}

fn konumda_eslesiyor_mu(girdi: &[u8], desenler: &[SirliDesen<'_>], konum: usize) -> Option<usize> {
    let kalan = &girdi[konum..];
    for desen in desenler {
        if kalan.len() >= desen.baytlar.len() && kalan.starts_with(desen.baytlar) {
            return Some(desen.baytlar.len());
        }
    }
    None
}

fn sentetik_etiket_sir_parcasi_mi(etiket: &[u8], desenler: &[SirliDesen<'_>]) -> bool {
    desenler.iter().any(|d| {
        d.baytlar.len() >= etiket.len()
            && d.baytlar
                .windows(etiket.len())
                .any(|pencere| pencere == etiket)
    })
}

/// Maskelemenin ürettiği sentetik `[gizli]` veya `[kesildi]` etiketlerinin komşu
/// baytlarla birleşerek yeni bir sır oluşturmasını engeller.
///
/// Eğer bir sır `[gizli]` veya `[kesildi]` dizisini içeriyorsa ve çıktıdaki bir
/// sır tam olarak sentetik bir etiketin üzerinden geçiyorsa, o tam eşleşme ve
/// içindeki sentetik etiket tek bir `[gizli]` etiketine indirgenir. Girdide zaten
/// var olan düz `[gizli]` / `[kesildi]` metinleri ise sentetik olmadıkları için
/// yeni bir sır birleşmesi üretmez.
fn sentetik_etiket_cakisimini_temizle(
    girdi: Vec<u8>,
    sentetik_araliklar: &[(usize, usize)],
    desenler: &[SirliDesen<'_>],
) -> Vec<u8> {
    if desenler.is_empty()
        || sentetik_araliklar.is_empty()
        || !SENTETIK_ETIKETLER
            .iter()
            .any(|etiket| sentetik_etiket_sir_parcasi_mi(etiket, desenler))
    {
        return girdi;
    }

    let mut mevcut = girdi;
    let mut araliklar = sentetik_araliklar.to_vec();

    loop {
        let mut degisti = false;
        let mut yeni = Vec::with_capacity(mevcut.len());
        let mut yeni_araliklar = Vec::with_capacity(araliklar.len());
        let mut i = 0;
        let mut aralik_indisi = 0;

        while i < mevcut.len() {
            while aralik_indisi < araliklar.len() && araliklar[aralik_indisi].1 <= i {
                aralik_indisi += 1;
            }

            if let Some(uzunluk) = konumda_eslesiyor_mu(&mevcut, desenler, i) {
                let bitis = i + uzunluk;
                let mut sentetik_kesisiyor = false;
                let mut k = aralik_indisi;
                while k < araliklar.len() && araliklar[k].0 < bitis {
                    let (bas, son) = araliklar[k];
                    if bas >= i && son <= bitis && !(bas == i && son == bitis) {
                        sentetik_kesisiyor = true;
                        break;
                    }
                    k += 1;
                }
                if sentetik_kesisiyor {
                    let yeni_bas = yeni.len();
                    yeni.extend_from_slice(GIZLI_BAYT);
                    yeni_araliklar.push((yeni_bas, yeni.len()));
                    i = bitis;
                    degisti = true;
                    continue;
                }
            }

            if aralik_indisi < araliklar.len() && araliklar[aralik_indisi].0 == i {
                let (bas, son) = araliklar[aralik_indisi];
                let yeni_bas = yeni.len();
                yeni.extend_from_slice(&mevcut[bas..son]);
                yeni_araliklar.push((yeni_bas, yeni.len()));
                i = son;
                aralik_indisi += 1;
            } else {
                yeni.push(mevcut[i]);
                i += 1;
            }
        }

        mevcut = yeni;
        araliklar = yeni_araliklar;
        if !degisti {
            break;
        }
    }

    mevcut
}

fn govdeyi_maskele_ve_sentetik_temizle(
    girdi: &[u8],
    desenler: &[SirliDesen<'_>],
    taranacak_son: usize,
) -> (Vec<u8>, Vec<(usize, usize)>, usize) {
    let sinir = taranacak_son.min(girdi.len());
    if desenler.is_empty() || sinir == 0 {
        return (girdi[..sinir].to_vec(), Vec::new(), sinir);
    }

    let mut cikti = Vec::with_capacity(sinir);
    let mut sentetik = Vec::new();
    let mut i = 0;
    while i < sinir {
        if let Some(eslesen_uzunluk) = konumda_eslesiyor_mu(girdi, desenler, i) {
            let bas = cikti.len();
            cikti.extend_from_slice(GIZLI_BAYT);
            sentetik.push((bas, cikti.len()));
            i += eslesen_uzunluk;
        } else {
            cikti.push(girdi[i]);
            i += 1;
        }
    }

    let temiz = sentetik_etiket_cakisimini_temizle(cikti, &sentetik, desenler);
    (temiz, sentetik, i)
}

/// Ham bayt dilimindeki tüm gizli değerleri `[gizli]` ile değiştirir.
///
/// Eşleştirme en uzun sırdan en kısaya doğru, soldan sağa örtüşmeyen biçimde
/// yapılır. Böylece daha uzun bir sırrın parçası olan kısa sır, uzun sırrın
/// kuyruğunu açıkta bırakamaz.
pub fn maskele_bayt(girdi: &[u8], sirlar: &[GizliDeger]) -> Vec<u8> {
    let desenler = sirali_sirlar(sirlar);
    govdeyi_maskele_ve_sentetik_temizle(girdi, &desenler, girdi.len()).0
}

/// Metin içindeki tüm gizli değerleri `[gizli]` ile değiştirir.
pub fn maskele_metin(girdi: &str, sirlar: &[GizliDeger]) -> String {
    let maskeli = maskele_bayt(girdi.as_bytes(), sirlar);
    String::from_utf8_lossy(&maskeli).into_owned()
}

/// Metni önce maskeler, ardından `azami_bayt` sınırını aşıyorsa geçerli bir
/// UTF-8 karakter sınırından kesip sonuna `[kesildi]` ekler.
///
/// Kesme işlemi maskelemeden sonra yapıldığı için kesme sınırına denk gelen bir
/// gizli değer önce `[gizli]` etiketine dönüştürülür; kısmi sır öneki dışarı
/// sızmaz. Kesme noktası `[gizli]` etiketinin ortasına denk gelirse yarım etiket
/// bırakılmaz, etiket başlangıcından kesilir.
pub fn maskeli_utf8_kes(girdi: &str, sirlar: &[GizliDeger], azami_bayt: usize) -> String {
    let desenler = sirali_sirlar(sirlar);
    let (maskeli, sentetik, _) =
        govdeyi_maskele_ve_sentetik_temizle(girdi.as_bytes(), &desenler, girdi.len());
    let (kesilmis, kesildi_eklendi) =
        maskeli_baytlari_utf8_kes_sentetik(&maskeli, &sentetik, azami_bayt, false);
    if kesildi_eklendi {
        let mut araliklar = sentetik;
        araliklar.retain(|(_, son)| *son <= kesilmis.len().saturating_sub(KESILDI_BAYT.len()));
        let son_bas = kesilmis.len() - KESILDI_BAYT.len();
        araliklar.push((son_bas, kesilmis.len()));
        let temiz = sentetik_etiket_cakisimini_temizle(kesilmis, &araliklar, &desenler);
        String::from_utf8_lossy(&temiz).into_owned()
    } else {
        String::from_utf8_lossy(&kesilmis).into_owned()
    }
}

/// Maskelenmiş baytları UTF-8 karakter sınırında en çok `azami_bayt` olacak
/// biçimde keser; kesildiyse veya `zorla_kesildi_etiketi` doğruysa sonuna
/// `[kesildi]` ekler.
fn maskeli_baytlari_utf8_kes_sentetik(
    maskeli: &[u8],
    sentetik_araliklar: &[(usize, usize)],
    azami_bayt: usize,
    zorla_kesildi_etiketi: bool,
) -> (Vec<u8>, bool) {
    let mut govde = utf8_kayipsiz_baytlar(maskeli);
    let mut kesildi = zorla_kesildi_etiketi;

    if govde.len() > azami_bayt {
        kesildi = true;
        let mut kesme_noktasi = utf8_sinirini_bul(&govde, azami_bayt);
        kesme_noktasi =
            sentetik_etiket_ortasini_geri_cek(kesme_noktasi, sentetik_araliklar, maskeli);
        govde.truncate(kesme_noktasi);
    }

    if kesildi {
        govde.extend_from_slice(KESILDI_BAYT);
    }
    (govde, kesildi)
}

fn utf8_kayipsiz_baytlar(baytlar: &[u8]) -> Vec<u8> {
    match std::str::from_utf8(baytlar) {
        Ok(_) => baytlar.to_vec(),
        Err(_) => String::from_utf8_lossy(baytlar).into_owned().into_bytes(),
    }
}

fn utf8_sinirini_bul(utf8_baytlar: &[u8], azami_bayt: usize) -> usize {
    let mut sinir = azami_bayt.min(utf8_baytlar.len());
    while sinir > 0 && (utf8_baytlar[sinir] & 0b1100_0000) == 0b1000_0000 {
        sinir -= 1;
    }
    sinir
}

/// Kesme noktası yalnızca maskelemenin ürettiği sentetik bir `[gizli]` etiketinin
/// ortasına denk geliyorsa, yarım `[giz` bırakmamak için kesme noktasını etiketin
/// başına çeker.
fn sentetik_etiket_ortasini_geri_cek(
    kesme_noktasi: usize,
    sentetik_araliklar: &[(usize, usize)],
    orijinal_maskeli: &[u8],
) -> usize {
    if std::str::from_utf8(orijinal_maskeli).is_ok() {
        for &(bas, son) in sentetik_araliklar {
            if bas < kesme_noktasi && kesme_noktasi < son {
                return bas;
            }
        }
        return kesme_noktasi;
    }
    let baslangic = kesme_noktasi.saturating_sub(GIZLI_BAYT.len() - 1);
    for aday in baslangic..kesme_noktasi {
        let parca = &orijinal_maskeli[aday..kesme_noktasi.min(orijinal_maskeli.len())];
        if GIZLI_BAYT.starts_with(parca) && sentetik_araliklar.iter().any(|&(b, _)| b == aday) {
            return aday;
        }
    }
    kesme_noktasi
}

/// Akan bir satır taşma kipine geçtiğinde, taşan kuyruktaki sırları sınırlı
/// bellekle yakalamak için tutulan durum.
#[derive(Default)]
struct TasmaDurumu {
    /// Sınırda bölünen bir sırrı yakalamak için tutulan en çok `en_uzun_sir - 1`
    /// baytlık kuyruk.
    kuyruk: Vec<u8>,
    /// Taşan kısım içinde en az bir gizli değer yakalandı mı?
    sir_yakalandi: bool,
}

/// Akan `stderr` baytları için maskeli ve sınırlı satır halkası.
///
/// - Her satırda maskelemeden önce en çok `4 KiB + en_uzun_sir_bayt` tutulur.
/// - Satır bu sınırı aşarsa taşma kipine geçilir: ilk `4 KiB`'lık güvenli gövde
///   maskelenir, taşan kuyruk ise sınırlı bloklarla tüketilir. Sınırda bölünen
///   bir sır varsa gövdeye `[gizli]` olarak yansır; taşan bölgede başka bir sır
///   görülürse satır sonuna `[gizli][kesildi]` eklenir.
/// - En çok son 50 satır (`SON_SATIR_ADEDI`) ve toplam 200 KiB
///   (`STDERR_TOPLAM_SINIR_BAYT`) tutulur.
pub struct StderrHalkasi {
    sirlar: Vec<GizliDeger>,
    satirlar: VecDeque<String>,
    toplam_bayt: usize,
    aktif_tampon: Vec<u8>,
    aktif_maskeli_govde: Vec<u8>,
    aktif_sentetik_araliklar: Vec<(usize, usize)>,
    tasma: Option<TasmaDurumu>,
}

impl fmt::Debug for StderrHalkasi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StderrHalkasi")
            .field("sir_sayisi", &self.sirlar.len())
            .field("satir_sayisi", &self.satirlar.len())
            .field("toplam_bayt", &self.toplam_bayt)
            .finish()
    }
}

impl StderrHalkasi {
    /// Verilen gizli değer kümesiyle yeni bir `stderr` halkası oluşturur.
    pub fn yeni(sirlar: Vec<GizliDeger>) -> Self {
        Self {
            sirlar,
            satirlar: VecDeque::with_capacity(SON_SATIR_ADEDI),
            toplam_bayt: 0,
            aktif_tampon: Vec::new(),
            aktif_maskeli_govde: Vec::new(),
            aktif_sentetik_araliklar: Vec::new(),
            tasma: None,
        }
    }

    /// Halkadaki tüm tamamlanmış satırları ve aktif satır durumunu temizler.
    pub fn temizle(&mut self) {
        self.satirlar.clear();
        self.toplam_bayt = 0;
        self.aktif_tampon.clear();
        self.aktif_maskeli_govde.clear();
        self.aktif_sentetik_araliklar.clear();
        self.tasma = None;
    }

    /// Gizli değer kümesini günceller ve eski tanımdan kalan tüm satırları
    /// temizler.
    pub fn sirlari_guncelle(&mut self, yeni_sirlar: Vec<GizliDeger>) {
        self.sirlar = yeni_sirlar;
        self.temizle();
    }

    /// Tamamlanmış satırları eskiden yeniye doğru döner.
    pub fn satirlar(&self) -> impl Iterator<Item = &str> {
        self.satirlar.iter().map(String::as_str)
    }

    /// Tamamlanmış satır sayısını döner.
    pub fn satir_sayisi(&self) -> usize {
        self.satirlar.len()
    }

    /// Halkada tutulan tamamlanmış satırların toplam bayt uzunluğunu döner.
    pub fn toplam_bayt(&self) -> usize {
        self.toplam_bayt
    }

    /// Akan `stderr` baytlarını halkaya besler.
    pub fn besle(&mut self, girdi: &[u8]) {
        let mut kalan = girdi;
        while !kalan.is_empty() {
            if let Some(nl_konumu) = kalan.iter().position(|&b| b == b'\n') {
                self.parca_isle(&kalan[..nl_konumu]);
                self.satiri_bitir();
                kalan = &kalan[nl_konumu + 1..];
            } else {
                self.parca_isle(kalan);
                break;
            }
        }
    }

    /// Akış kapandığında (`EOF`) satır sonu karakteri bekleyen kısmi bir satır
    /// varsa onu da maskeleyip halkaya ekler.
    pub fn akisi_bitir(&mut self) {
        if !self.aktif_tampon.is_empty()
            || !self.aktif_maskeli_govde.is_empty()
            || self.tasma.is_some()
        {
            self.satiri_bitir();
        }
    }

    fn parca_isle(&mut self, mut parca: &[u8]) {
        if parca.is_empty() {
            return;
        }

        let desenler = sirali_sirlar(&self.sirlar);
        let en_uzun = en_uzun_sir_bayt(&desenler);
        let pencere_siniri = SATIR_SINIRI_BAYT.saturating_add(en_uzun);

        if self.tasma.is_none() {
            let bos_yer = pencere_siniri.saturating_sub(self.aktif_tampon.len());
            let alinacak = parca.len().min(bos_yer);
            self.aktif_tampon.extend_from_slice(&parca[..alinacak]);
            parca = &parca[alinacak..];

            if parca.is_empty() {
                return;
            }

            // Pencere doldu ve aynı satırda hâlâ bayt var: ilk 4 KiB içinde
            // başlayan tüm sırları (sınırda uzananlar dahil) maskele ve taşma
            // kipine geç.
            let (maskeli_govde, sentetik, tuketilen) = govdeyi_maskele_ve_sentetik_temizle(
                &self.aktif_tampon,
                &desenler,
                SATIR_SINIRI_BAYT,
            );
            self.aktif_maskeli_govde = maskeli_govde;
            self.aktif_sentetik_araliklar = sentetik;

            let kalan_kuyruk = self.aktif_tampon[tuketilen..].to_vec();
            self.aktif_tampon.clear();

            let mut tasma = TasmaDurumu {
                kuyruk: Vec::new(),
                sir_yakalandi: false,
            };
            Self::tasma_kuyruguna_besle(&mut tasma, &kalan_kuyruk, &desenler, en_uzun);
            self.tasma = Some(tasma);
        }

        if let Some(tasma) = self.tasma.as_mut() {
            Self::tasma_kuyruguna_besle(tasma, parca, &desenler, en_uzun);
        }
    }

    fn tasma_kuyruguna_besle(
        tasma: &mut TasmaDurumu,
        parca: &[u8],
        desenler: &[SirliDesen<'_>],
        en_uzun: usize,
    ) {
        if parca.is_empty() {
            return;
        }
        if tasma.sir_yakalandi || desenler.is_empty() || en_uzun == 0 {
            tasma.kuyruk.clear();
            return;
        }

        let tutulan_kuyruk = en_uzun.saturating_sub(1);
        let adim_boyutu = SATIR_SINIRI_BAYT.max(en_uzun);
        let mut ofset = 0;

        while ofset < parca.len() {
            let bitis = (ofset + adim_boyutu).min(parca.len());
            tasma.kuyruk.extend_from_slice(&parca[ofset..bitis]);

            if Self::dilimde_sir_var_mi(&tasma.kuyruk, desenler) {
                tasma.sir_yakalandi = true;
                tasma.kuyruk.clear();
                return;
            }

            if tasma.kuyruk.len() > tutulan_kuyruk {
                let atilacak = tasma.kuyruk.len() - tutulan_kuyruk;
                tasma.kuyruk.drain(..atilacak);
            }
            ofset = bitis;
        }
    }

    fn dilimde_sir_var_mi(dilim: &[u8], desenler: &[SirliDesen<'_>]) -> bool {
        for desen in desenler {
            if yuvarlanan_eslesme_var_mi(dilim, desen) {
                return true;
            }
        }
        false
    }

    fn satiri_bitir(&mut self) {
        let desenler = sirali_sirlar(&self.sirlar);
        let satir = if let Some(tasma) = self.tasma.take() {
            let (mut govde, _) = maskeli_baytlari_utf8_kes_sentetik(
                &self.aktif_maskeli_govde,
                &self.aktif_sentetik_araliklar,
                SATIR_SINIRI_BAYT,
                false,
            );
            let mut araliklar = self.aktif_sentetik_araliklar.clone();
            araliklar.retain(|(_, son)| *son <= govde.len());
            self.aktif_maskeli_govde.clear();
            self.aktif_sentetik_araliklar.clear();
            if tasma.sir_yakalandi {
                let bas = govde.len();
                govde.extend_from_slice(GIZLI_BAYT);
                araliklar.push((bas, govde.len()));
            }
            let bas = govde.len();
            govde.extend_from_slice(KESILDI_BAYT);
            araliklar.push((bas, govde.len()));
            let temiz = sentetik_etiket_cakisimini_temizle(govde, &araliklar, &desenler);
            String::from_utf8_lossy(&temiz).into_owned()
        } else {
            let mut ham = std::mem::take(&mut self.aktif_tampon);
            if ham.last() == Some(&b'\r') {
                ham.pop();
            }
            let (maskeli, sentetik, _) =
                govdeyi_maskele_ve_sentetik_temizle(&ham, &desenler, ham.len());
            let (kesilmis, kesildi_eklendi) =
                maskeli_baytlari_utf8_kes_sentetik(&maskeli, &sentetik, SATIR_SINIRI_BAYT, false);
            if kesildi_eklendi {
                let mut araliklar = sentetik;
                araliklar
                    .retain(|(_, son)| *son <= kesilmis.len().saturating_sub(KESILDI_BAYT.len()));
                let son_bas = kesilmis.len() - KESILDI_BAYT.len();
                araliklar.push((son_bas, kesilmis.len()));
                let temiz = sentetik_etiket_cakisimini_temizle(kesilmis, &araliklar, &desenler);
                String::from_utf8_lossy(&temiz).into_owned()
            } else {
                String::from_utf8_lossy(&kesilmis).into_owned()
            }
        };

        self.satir_ekle(satir);
    }

    fn satir_ekle(&mut self, satir: String) {
        let satir_bayt = satir.len();
        self.satirlar.push_back(satir);
        self.toplam_bayt += satir_bayt;

        while self.satirlar.len() > SON_SATIR_ADEDI
            || (self.toplam_bayt > STDERR_TOPLAM_SINIR_BAYT && self.satirlar.len() > 1)
        {
            if let Some(cikan) = self.satirlar.pop_front() {
                self.toplam_bayt = self.toplam_bayt.saturating_sub(cikan.len());
            }
        }
    }
}

fn yuvarlanan_eslesme_var_mi(dilim: &[u8], desen: &SirliDesen<'_>) -> bool {
    let m = desen.baytlar.len();
    if m == 0 || dilim.len() < m {
        return false;
    }

    let mut h1 = 0_u64;
    let mut h2 = 0_u64;
    for &b in &dilim[..m] {
        let deger = u64::from(b) + 1;
        h1 = (h1 * RK_TABAN_1 + deger) % RK_MOD_1;
        h2 = (h2 * RK_TABAN_2 + deger) % RK_MOD_2;
    }
    if h1 == desen.ozet_1 && h2 == desen.ozet_2 && &dilim[..m] == desen.baytlar {
        return true;
    }

    for i in m..dilim.len() {
        let cikan = u64::from(dilim[i - m]) + 1;
        let giren = u64::from(dilim[i]) + 1;

        let cikan_1 = (cikan * desen.taban_kuvveti_1) % RK_MOD_1;
        h1 = (h1 + RK_MOD_1 - cikan_1) % RK_MOD_1;
        h1 = (h1 * RK_TABAN_1 + giren) % RK_MOD_1;

        let cikan_2 = (cikan * desen.taban_kuvveti_2) % RK_MOD_2;
        h2 = (h2 + RK_MOD_2 - cikan_2) % RK_MOD_2;
        h2 = (h2 * RK_TABAN_2 + giren) % RK_MOD_2;

        let bas = i + 1 - m;
        if h1 == desen.ozet_1 && h2 == desen.ozet_2 && &dilim[bas..=i] == desen.baytlar {
            return true;
        }
    }
    false
}

/// Girdinin sonunda herhangi bir gizli değerin gerçek öneki (`1..sir.len()`)
/// varsa, en uzun önek uzunluğunu döner.
///
/// Bu yardımcı, akan metin pencerelerinde sınırda bölünmüş olabilecek bir sırrı
/// kesmeden önce bekletmek için kullanılır. Çift tabanlı yuvarlanan özet
/// sayesinde uzun sırlar için kuadratik bayt karşılaştırması yapmaz.
pub fn son_kismi_sir_oneki_uzunlugu(girdi: &[u8], sirlar: &[GizliDeger]) -> usize {
    if girdi.is_empty() || sirlar.is_empty() {
        return 0;
    }
    let desenler = sirali_sirlar(sirlar);
    let en_uzun = en_uzun_sir_bayt(&desenler);
    if en_uzun <= 1 {
        return 0;
    }

    let azami_onek = (en_uzun - 1).min(girdi.len());
    let baslangic_alt_siniri = girdi.len() - azami_onek;

    for baslangic in baslangic_alt_siniri..girdi.len() {
        let sonek = &girdi[baslangic..];
        if desenler
            .iter()
            .any(|d| d.baytlar.len() > sonek.len() && d.baytlar.starts_with(sonek))
        {
            return sonek.len();
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sir(deger: &str) -> GizliDeger {
        GizliDeger::yeni(deger).expect("geçerli test sırrı")
    }

    #[test]
    fn gizli_deger_uzunluk_nul_ve_debug_kurallarini_zorlar() {
        for kisa in ["", "q", "1234567", "ğüşiöç7"] {
            let hata = GizliDeger::yeni(kisa).expect_err("8 karakterden kısa sır reddedilmeli");
            assert!(matches!(
                hata,
                MaskeHatasi::CokKisa {
                    en_az: EN_AZ_GIZLI_KARAKTER,
                    ..
                }
            ));
            let hata_metni = format!("{hata} {hata:?}");
            if !kisa.is_empty() {
                assert!(
                    !hata_metni.contains(kisa),
                    "hata metni aday sırrı sızdırmamalı: {hata_metni}"
                );
            }
        }

        // 8 UTF-8 karakter (çok baytlı Türkçe karakterlerle) kabul edilmeli.
        let turkce = GizliDeger::yeni("ğüşiöçab").expect("8 UTF-8 karakter kabul edilmeli");
        assert_eq!(turkce.karakter_sayisi(), 8);
        assert!(turkce.bayt_uzunlugu() > 8);

        // NUL baytı reddedilmeli.
        let nul_hatasi =
            GizliDeger::yeni("gizli\0deger123").expect_err("NUL baytlı sır reddedilmeli");
        assert_eq!(nul_hatasi, MaskeHatasi::NulBaytiVar);

        // Debug çıktısı ham sırrı asla göstermemeli.
        let g = sir("cok-gizli-anahtar-99");
        let debug_metni = format!("{g:?}");
        assert_eq!(debug_metni, "GizliDeger([gizli])");
        assert!(!debug_metni.contains("cok-gizli-anahtar-99"));
    }

    #[test]
    fn tam_alt_dize_ve_ortusen_sirlari_en_uzundan_baslayarak_maskeler() {
        let kisa = sir("gizli-1234");
        let uzun = sir("gizli-1234-uzun-5678");
        let ic_ice = sir("1234-uzun-5678-ek");

        // Kısa sır önce verilse bile uzun sır bütün olarak maskelenmeli,
        // "-uzun-5678" kuyruğu açıkta kalmamalı.
        let girdi = "bas=gizli-1234-uzun-5678; orta=gizli-1234; son=gizli-1234-uzun-5678-ek!";
        let maskeli = maskele_metin(girdi, &[kisa.clone(), uzun.clone(), ic_ice.clone()]);
        assert_eq!(maskeli, "bas=[gizli]; orta=[gizli]; son=[gizli]-ek!");
        assert!(!maskeli.contains("gizli-1234"));
        assert!(!maskeli.contains("uzun-5678"));

        // Soldan sağa örtüşen iki farklı sır: "abcdefghij" ve "efghijklmn".
        let sol = sir("abcdefghij");
        let sag = sir("efghijklmn");
        let birlesik = "xxabcdefghijklmnyy";
        let sonuc = maskele_metin(birlesik, &[sag, sol]);
        assert_eq!(sonuc, "xx[gizli]klmnyy");
        assert!(!sonuc.contains("abcdefghij"));
        assert!(!sonuc.contains("efghijklmn"));
    }

    #[test]
    fn maskeleme_kesmeden_once_calisir_ve_utf8_sinirini_korur() {
        let g = sir("SIR_TOKEN_123456");
        // Sır tam kesme sınırının (20 bayt) üzerinden geçiyor: 14 bayt önek + 16 bayt sır.
        let girdi = format!("0123456789abcd{}", g.expose_secret());
        let kesilmis = maskeli_utf8_kes(&girdi, std::slice::from_ref(&g), 20);

        // Sır önce "[gizli]" (7 bayt) olur; toplam "0123456789abcd[gizli]" = 21 bayt > 20.
        // Kesme noktası "[gizli]" etiketinin ortasına düşeceği için etiket başından
        // kesilir ve "[kesildi]" eklenir; kısmi "SIR_TO" veya yarım "[giz" sızmaz.
        assert_eq!(kesilmis, "0123456789abcd[kesildi]");
        assert!(!kesilmis.contains("SIR"));
        assert!(!kesilmis.contains("[giz"));

        // Çok baytlı UTF-8 karakter sınırında panik yapmadan kesmeli.
        let utf8_girdi = "ğğğğğğğğğğ"; // her 'ğ' 2 bayt, toplam 20 bayt
        let utf8_kesilmis = maskeli_utf8_kes(utf8_girdi, &[], 5);
        assert_eq!(utf8_kesilmis, "ğğ[kesildi]");
    }

    #[test]
    fn stderr_halkasi_parcalanan_sirlari_ve_4kib_kesmesini_dogru_uygular() {
        let g = sir("PARCALI_SIR_987654321");
        let mut halka = StderrHalkasi::yeni(vec![g.clone()]);

        // Sırrı üç ayrı parça halinde besle.
        halka.besle(b"hata: yetki=");
        halka.besle(b"PARCALI_");
        halka.besle(b"SIR_9876");
        halka.besle(b"54321 ve devam\n");

        let satirlar: Vec<&str> = halka.satirlar().collect();
        assert_eq!(satirlar, vec!["hata: yetki=[gizli] ve devam"]);

        // Tam 4 KiB sınırında (4090..4111) başlayan ve 4 KiB'ı aşan sır:
        // "4 KiB + en_uzun_sir" penceresi sayesinde önce maskelenmeli, sonra kesilmeli.
        halka.temizle();
        let mut uzun_satir = vec![b'a'; SATIR_SINIRI_BAYT - 5];
        uzun_satir.extend_from_slice(g.as_bytes());
        uzun_satir.extend_from_slice(b"-kuyruk\n");
        halka.besle(&uzun_satir);

        let satir = halka.satirlar().next().expect("satır üretilmeli");
        assert!(!satir.contains("PARCALI"));
        assert!(!satir.contains("987654321"));
        assert!(satir.ends_with(KESILDI_ETIKETI));
    }

    #[test]
    fn stderr_halkasi_dev_satiri_sinirli_tuketir_ve_tasan_sirri_gizler() {
        let g = sir("DEV_SATIR_SIRRI_2026");
        let mut halka = StderrHalkasi::yeni(vec![g.clone()]);

        // 64 KiB gürültü + parçalı sır + 32 KiB gürültü + '\n'
        let gurultu = vec![b'x'; 8 * 1024];
        for _ in 0..8 {
            halka.besle(&gurultu);
        }
        // Sır taşan bölgede iki parçaya bölünerek geliyor.
        halka.besle(b"DEV_SATIR_");
        halka.besle(b"SIRRI_2026");
        for _ in 0..4 {
            halka.besle(&gurultu);
        }
        halka.besle(b"\nsonraki temiz satir\n");

        let satirlar: Vec<&str> = halka.satirlar().collect();
        assert_eq!(satirlar.len(), 2);
        assert!(
            !satirlar[0].contains("DEV_SATIR"),
            "dev satırda sır sızmamalı"
        );
        assert!(
            satirlar[0].ends_with("[gizli][kesildi]"),
            "taşan bölgede yakalanan sır [gizli][kesildi] ile işaretlenmeli"
        );
        assert!(
            satirlar[0].len() <= SATIR_SINIRI_BAYT + GIZLI_ETIKETI.len() + KESILDI_ETIKETI.len()
        );
        assert_eq!(satirlar[1], "sonraki temiz satir");
    }

    #[test]
    fn stderr_halkasi_son_50_satiri_ve_temizleme_kuralini_uygular() {
        let g1 = sir("ILK_GIZLI_DEGER_11");
        let g2 = sir("IKINCI_GIZLI_DEGER_22");
        let mut halka = StderrHalkasi::yeni(vec![g1]);

        for i in 0..65 {
            let satir = format!("satir-{i}: ILK_GIZLI_DEGER_11\n");
            halka.besle(satir.as_bytes());
        }

        assert_eq!(halka.satir_sayisi(), SON_SATIR_ADEDI);
        assert!(halka.toplam_bayt() <= STDERR_TOPLAM_SINIR_BAYT);
        let ilk_kalan = halka.satirlar().next().expect("ilk satır");
        assert_eq!(ilk_kalan, "satir-15: [gizli]");
        let son_kalan = halka.satirlar().last().expect("son satır");
        assert_eq!(son_kalan, "satir-64: [gizli]");

        // Tanım/sır değiştiğinde halka temizlenmeli.
        halka.sirlari_guncelle(vec![g2]);
        assert_eq!(halka.satir_sayisi(), 0);
        assert_eq!(halka.toplam_bayt(), 0);

        halka.besle(b"yeni: IKINCI_GIZLI_DEGER_22");
        halka.akisi_bitir();
        let satirlar: Vec<&str> = halka.satirlar().collect();
        assert_eq!(satirlar, vec!["yeni: [gizli]"]);
    }

    #[test]
    fn sentetik_gizli_ve_kesildi_etiketleri_yeni_sir_olusturamaz() {
        let ic_sir = sir("IC_SIR_123456");
        let dis_sir = sir("aa[gizli]bb");

        // İç sır maskelendiğinde oluşan sentetik "[gizli]" komşu "aa" ve "bb" ile
        // birleşerek `dis_sir` ("aa[gizli]bb") dizisini oluşturmamalı.
        let girdi = format!("xxaa{}bbyy", ic_sir.expose_secret());
        let maskeli = maskele_metin(&girdi, &[ic_sir.clone(), dis_sir.clone()]);
        assert!(
            !maskeli.contains(dis_sir.expose_secret()),
            "sentetik [gizli] etiketi dış sırrı oluşturmamalı: {maskeli}"
        );
        assert_eq!(maskeli, "xx[gizli]yy");

        // Kesme sonucunda eklenen sentetik "[kesildi]" etiketi de sonek ile
        // birleşerek `[kesildi]` içeren bir sırrı oluşturmamalı.
        let kesildi_sirri = sir("ab[kesildi]");
        let kesilen = maskeli_utf8_kes("ab1234567890", std::slice::from_ref(&kesildi_sirri), 2);
        assert!(
            !kesilen.contains(kesildi_sirri.expose_secret()),
            "sentetik [kesildi] etiketi sır oluşturmamalı: {kesilen}"
        );
        assert_eq!(kesilen, "[gizli]");
    }

    #[test]
    fn uzun_sir_ile_dev_satir_taramasi_dogrusal_zamanda_calisir() {
        // 8 KiB uzunluğunda bir sır (son baytı farklı) ve neredeyse tamamen
        // önek eşleşmesi üreten 256 KiB'lık taşan satır: yuvarlanan özet
        // sayesinde kuadratik davranış göstermeden tamamlanmalı.
        let mut uzun_sir_metni = "a".repeat(8191);
        uzun_sir_metni.push('Z');
        let uzun_sir = sir(&uzun_sir_metni);

        let mut halka = StderrHalkasi::yeni(vec![uzun_sir.clone()]);
        let parca = vec![b'a'; 32 * 1024];
        for _ in 0..8 {
            halka.besle(&parca);
        }
        halka.besle(uzun_sir.as_bytes());
        halka.besle(b"\n");

        let satir = halka.satirlar().next().expect("satır üretilmeli");
        assert!(satir.ends_with("[gizli][kesildi]"));
    }
}
