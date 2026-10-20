//! HTTP yanıt gövdesi için sınırlı GZIP / ZLIB / DEFLATE açıcı.
//!
//! Sıkıştırma bombalarına (zip/gzip bomb) karşı açılmış gövde boyutu her
//! bayt ve LZ77 kopyalama adımında [`AZAMI_ACILMIS_GOVDE_BAYT`] (4 MiB) ile
//! sınırlandırılır. Sınır aşıldığı anda bellek tahsisi büyütülmeden işlem
//! durdurulur.

use thiserror::Error;

/// Açılmış (decompressed) HTTP yanıt gövdesi için üst sınır: 4 MiB.
pub const AZAMI_ACILMIS_GOVDE_BAYT: usize = 4 * 1024 * 1024;

/// Desteklenen HTTP `Content-Encoding` biçimleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SikistirmaBicimi {
    /// Sıkıştırma yok (`identity` veya başlık yok).
    Yok,
    /// RFC 1952 GZIP (`gzip` veya `x-gzip`).
    Gzip,
    /// RFC 1950 ZLIB veya ham RFC 1951 DEFLATE (`deflate`).
    Deflate,
}

/// Sıkıştırma çözme hataları.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SikistirmaHatasi {
    #[error("desteklenmeyen içerik sıkıştırma biçimi: {0}")]
    DesteklenmeyenKodlama(String),

    #[error("açılmış yanıt gövdesi sınırı ({sinir} bayt) aşıldı")]
    AcilmisBoyutAsildi { sinir: usize },

    #[error("eksik veya yarıda kesilmiş sıkıştırılmış gövde: {0}")]
    EksikVeri(&'static str),

    #[error("geçersiz sıkıştırılmış veri: {0}")]
    BozukVeri(&'static str),
}

/// `Content-Encoding` başlık değerinden sıkıştırma biçimini çözümler.
pub fn sikistirma_bicimi_bul(baslik: Option<&str>) -> Result<SikistirmaBicimi, SikistirmaHatasi> {
    let Some(ham) = baslik else {
        return Ok(SikistirmaBicimi::Yok);
    };
    let temiz = ham.trim().to_ascii_lowercase();
    match temiz.as_str() {
        "" | "identity" => Ok(SikistirmaBicimi::Yok),
        "gzip" | "x-gzip" => Ok(SikistirmaBicimi::Gzip),
        "deflate" => Ok(SikistirmaBicimi::Deflate),
        diger => Err(SikistirmaHatasi::DesteklenmeyenKodlama(diger.to_string())),
    }
}

/// Akıştan gelen parçaları toplayıp üst sınır dahilinde açan yardımcı.
#[derive(Debug)]
pub struct AkisAcici {
    bicim: SikistirmaBicimi,
    azami_bayt: usize,
    tampon: Vec<u8>,
}

impl AkisAcici {
    /// Verilen sıkıştırma biçimi ve açılmış bayt sınırıyla yeni bir açıcı kurar.
    pub fn yeni(bicim: SikistirmaBicimi, azami_bayt: usize) -> Self {
        Self {
            bicim,
            azami_bayt,
            tampon: Vec::new(),
        }
    }

    /// Ağdan okunan yeni parçayı ekler ve mümkünse erken sınır denetimi yapar.
    pub fn besle(&mut self, parca: &[u8]) -> Result<(), SikistirmaHatasi> {
        if self.tampon.len().saturating_add(parca.len()) > self.azami_bayt {
            return Err(SikistirmaHatasi::AcilmisBoyutAsildi {
                sinir: self.azami_bayt,
            });
        }
        self.tampon.extend_from_slice(parca);

        if self.bicim != SikistirmaBicimi::Yok {
            // Sıkıştırılmış akışta bir sıkıştırma bombası birkaç kilobaytlık
            // ilk parçada bile 4 MiB sınırını aşabilir. Akış sürerken erken
            // sınır kontrolü yapıyoruz; yalnızca sınır aşımını ve kesin
            // biçim bozukluğunu hemen raporluyoruz, eksik veride sonraki
            // parçaları bekliyoruz.
            match ac_sinirli(&self.tampon, self.bicim, self.azami_bayt) {
                Ok(_) | Err(SikistirmaHatasi::EksikVeri(_)) => {}
                Err(hata @ SikistirmaHatasi::AcilmisBoyutAsildi { .. }) => return Err(hata),
                Err(_) => {}
            }
        }
        Ok(())
    }

    /// Akış tamamlandığında tüm gövdeyi doğrular ve açılmış baytları döndürür.
    pub fn bitir(self) -> Result<Vec<u8>, SikistirmaHatasi> {
        match self.bicim {
            SikistirmaBicimi::Yok => Ok(self.tampon),
            SikistirmaBicimi::Gzip | SikistirmaBicimi::Deflate => {
                ac_sinirli(&self.tampon, self.bicim, self.azami_bayt)
            }
        }
    }
}

/// Verilen sıkıştırılmış bayt dilimini `azami_bayt` sınırını aşmadan açar.
pub fn ac_sinirli(
    veri: &[u8],
    bicim: SikistirmaBicimi,
    azami_bayt: usize,
) -> Result<Vec<u8>, SikistirmaHatasi> {
    match bicim {
        SikistirmaBicimi::Yok => {
            if veri.len() > azami_bayt {
                return Err(SikistirmaHatasi::AcilmisBoyutAsildi { sinir: azami_bayt });
            }
            Ok(veri.to_vec())
        }
        SikistirmaBicimi::Gzip => gzip_ac(veri, azami_bayt),
        SikistirmaBicimi::Deflate => deflate_veya_zlib_ac(veri, azami_bayt),
    }
}

fn gzip_ac(veri: &[u8], azami_bayt: usize) -> Result<Vec<u8>, SikistirmaHatasi> {
    if veri.is_empty() {
        return Err(SikistirmaHatasi::EksikVeri("boş gzip gövdesi"));
    }
    let mut imlec = 0usize;
    let mut toplam_cikti = Vec::new();

    while imlec < veri.len() {
        if veri.len() - imlec < 10 {
            return Err(SikistirmaHatasi::EksikVeri("gzip başlığı eksik"));
        }
        if veri[imlec] != 0x1f || veri[imlec + 1] != 0x8b {
            return Err(SikistirmaHatasi::BozukVeri(
                "geçersiz gzip sihirli baytları",
            ));
        }
        if veri[imlec + 2] != 8 {
            return Err(SikistirmaHatasi::BozukVeri(
                "desteklenmeyen gzip sıkıştırma yöntemi",
            ));
        }
        let bayraklar = veri[imlec + 3];
        if (bayraklar & 0xE0) != 0 {
            return Err(SikistirmaHatasi::BozukVeri(
                "ayrılmış gzip bayrak bitleri dolu",
            ));
        }
        imlec += 10;

        // FEXTRA
        if (bayraklar & 0x04) != 0 {
            if veri.len() - imlec < 2 {
                return Err(SikistirmaHatasi::EksikVeri("gzip FEXTRA uzunluğu eksik"));
            }
            let xlen = u16::from_le_bytes([veri[imlec], veri[imlec + 1]]) as usize;
            imlec += 2;
            if veri.len() - imlec < xlen {
                return Err(SikistirmaHatasi::EksikVeri("gzip FEXTRA verisi eksik"));
            }
            imlec += xlen;
        }

        // FNAME
        if (bayraklar & 0x08) != 0 {
            while imlec < veri.len() && veri[imlec] != 0 {
                imlec += 1;
            }
            if imlec >= veri.len() {
                return Err(SikistirmaHatasi::EksikVeri("gzip FNAME sonlandırıcısı yok"));
            }
            imlec += 1;
        }

        // FCOMMENT
        if (bayraklar & 0x10) != 0 {
            while imlec < veri.len() && veri[imlec] != 0 {
                imlec += 1;
            }
            if imlec >= veri.len() {
                return Err(SikistirmaHatasi::EksikVeri(
                    "gzip FCOMMENT sonlandırıcısı yok",
                ));
            }
            imlec += 1;
        }

        // FHCRC
        if (bayraklar & 0x02) != 0 {
            if veri.len() - imlec < 2 {
                return Err(SikistirmaHatasi::EksikVeri("gzip FHCRC eksik"));
            }
            imlec += 2;
        }

        let uye_baslangic = toplam_cikti.len();
        let mut okuyucu = BitOkuyucu::yeni(&veri[imlec..]);
        deflate_akisini_ac(&mut okuyucu, &mut toplam_cikti, azami_bayt)?;
        okuyucu.bayt_sinirina_hizala();
        imlec += okuyucu.okunan_bayt_sayisi();

        if veri.len() - imlec < 8 {
            return Err(SikistirmaHatasi::EksikVeri(
                "gzip son bilgi (CRC32/ISIZE) eksik",
            ));
        }
        let beklenen_crc = u32::from_le_bytes([
            veri[imlec],
            veri[imlec + 1],
            veri[imlec + 2],
            veri[imlec + 3],
        ]);
        let beklenen_boyut = u32::from_le_bytes([
            veri[imlec + 4],
            veri[imlec + 5],
            veri[imlec + 6],
            veri[imlec + 7],
        ]);
        imlec += 8;

        let uye_dilimi = &toplam_cikti[uye_baslangic..];
        let hesaplanan_crc = crc32_hesapla(uye_dilimi);
        if hesaplanan_crc != beklenen_crc {
            return Err(SikistirmaHatasi::BozukVeri(
                "gzip CRC32 doğrulaması başarısız",
            ));
        }
        if (uye_dilimi.len() as u32) != beklenen_boyut {
            return Err(SikistirmaHatasi::BozukVeri(
                "gzip ISIZE doğrulaması başarısız",
            ));
        }
    }

    Ok(toplam_cikti)
}

fn deflate_veya_zlib_ac(veri: &[u8], azami_bayt: usize) -> Result<Vec<u8>, SikistirmaHatasi> {
    if veri.is_empty() {
        return Err(SikistirmaHatasi::EksikVeri("boş deflate gövdesi"));
    }

    // RFC 1950 ZLIB üstbilgisi var mı kontrol et.
    if veri.len() >= 2 {
        let cmf = veri[0];
        let flg = veri[1];
        let cm = cmf & 0x0F;
        let cinfo = cmf >> 4;
        let kontrol = (u16::from(cmf) * 256 + u16::from(flg)).is_multiple_of(31);
        if cm == 8 && cinfo <= 7 && kontrol && (flg & 0x20) == 0 {
            let mut cikti = Vec::new();
            let mut okuyucu = BitOkuyucu::yeni(&veri[2..]);
            if deflate_akisini_ac(&mut okuyucu, &mut cikti, azami_bayt).is_ok() {
                okuyucu.bayt_sinirina_hizala();
                let kalan_baslangic = 2 + okuyucu.okunan_bayt_sayisi();
                if veri.len() < kalan_baslangic + 4 {
                    return Err(SikistirmaHatasi::EksikVeri("zlib Adler32 son eki eksik"));
                }
                let beklenen_adler = u32::from_be_bytes([
                    veri[kalan_baslangic],
                    veri[kalan_baslangic + 1],
                    veri[kalan_baslangic + 2],
                    veri[kalan_baslangic + 3],
                ]);
                if adler32_hesapla(&cikti) != beklenen_adler {
                    return Err(SikistirmaHatasi::BozukVeri(
                        "zlib Adler32 doğrulaması başarısız",
                    ));
                }
                return Ok(cikti);
            }
        }
    }

    // Ham RFC 1951 DEFLATE akışı.
    let mut cikti = Vec::new();
    let mut okuyucu = BitOkuyucu::yeni(veri);
    deflate_akisini_ac(&mut okuyucu, &mut cikti, azami_bayt)?;
    Ok(cikti)
}

struct BitOkuyucu<'a> {
    veri: &'a [u8],
    bayt_indeksi: usize,
    bit_tamponu: u64,
    bit_sayisi: u8,
}

impl<'a> BitOkuyucu<'a> {
    fn yeni(veri: &'a [u8]) -> Self {
        Self {
            veri,
            bayt_indeksi: 0,
            bit_tamponu: 0,
            bit_sayisi: 0,
        }
    }

    fn doldur(&mut self, en_az_bit: u8) -> Result<(), SikistirmaHatasi> {
        while self.bit_sayisi < en_az_bit {
            if self.bayt_indeksi >= self.veri.len() {
                return Err(SikistirmaHatasi::EksikVeri(
                    "beklenmeyen sıkıştırılmış akış sonu",
                ));
            }
            let bayt = u64::from(self.veri[self.bayt_indeksi]);
            self.bayt_indeksi += 1;
            self.bit_tamponu |= bayt << self.bit_sayisi;
            self.bit_sayisi += 8;
        }
        Ok(())
    }

    fn bitleri_oku(&mut self, adet: u8) -> Result<u32, SikistirmaHatasi> {
        if adet == 0 {
            return Ok(0);
        }
        self.doldur(adet)?;
        let maske = (1u64 << adet) - 1;
        let deger = (self.bit_tamponu & maske) as u32;
        self.bit_tamponu >>= adet;
        self.bit_sayisi -= adet;
        Ok(deger)
    }

    fn bayt_sinirina_hizala(&mut self) {
        let atilacak = self.bit_sayisi % 8;
        self.bit_tamponu >>= atilacak;
        self.bit_sayisi -= atilacak;
    }

    fn tam_bayt_oku(&mut self) -> Result<u8, SikistirmaHatasi> {
        if self.bit_sayisi >= 8 {
            let b = (self.bit_tamponu & 0xFF) as u8;
            self.bit_tamponu >>= 8;
            self.bit_sayisi -= 8;
            return Ok(b);
        }
        if self.bayt_indeksi >= self.veri.len() {
            return Err(SikistirmaHatasi::EksikVeri("beklenmeyen bayt sonu"));
        }
        let b = self.veri[self.bayt_indeksi];
        self.bayt_indeksi += 1;
        Ok(b)
    }

    fn okunan_bayt_sayisi(&self) -> usize {
        let geri_alinacak = usize::from(self.bit_sayisi / 8);
        self.bayt_indeksi.saturating_sub(geri_alinacak)
    }
}

/// Kanonik Huffman ağacı (RFC 1951 §3.2.2).
struct HuffmanAgaci {
    /// Her kod uzunluğu (1..=15) için (ilk kanonik kod, sembol dizisindeki başlangıç indeksi, adet).
    basamaklar: [(u16, u16, u16); 16],
    semboller: Vec<u16>,
    en_uzun: u8,
}

impl HuffmanAgaci {
    fn olustur(uzunluklar: &[u8]) -> Result<Self, SikistirmaHatasi> {
        let mut uzunluk_sayilari = [0u16; 16];
        let mut en_uzun = 0u8;
        for &uz in uzunluklar {
            if uz > 15 {
                return Err(SikistirmaHatasi::BozukVeri(
                    "Huffman kod uzunluğu 15'i aştı",
                ));
            }
            if uz > 0 {
                uzunluk_sayilari[usize::from(uz)] += 1;
                if uz > en_uzun {
                    en_uzun = uz;
                }
            }
        }

        let mut ilk_kodlar = [0u16; 16];
        let mut ilk_indeksler = [0u16; 16];
        let mut kod = 0u32;
        let mut toplam_sembol = 0u16;
        for bit in 1..=15usize {
            kod = (kod + u32::from(uzunluk_sayilari[bit - 1])) << 1;
            if kod + u32::from(uzunluk_sayilari[bit]) > (1u32 << bit) {
                return Err(SikistirmaHatasi::BozukVeri("aşırı dolu Huffman kod ağacı"));
            }
            ilk_kodlar[bit] = kod as u16;
            ilk_indeksler[bit] = toplam_sembol;
            toplam_sembol += uzunluk_sayilari[bit];
        }

        let mut semboller = vec![0u16; usize::from(toplam_sembol)];
        let mut yazma_indeksleri = ilk_indeksler;
        for (sembol, &uz) in uzunluklar.iter().enumerate() {
            if uz > 0 {
                let hedef = usize::from(yazma_indeksleri[usize::from(uz)]);
                semboller[hedef] = sembol as u16;
                yazma_indeksleri[usize::from(uz)] += 1;
            }
        }

        let mut basamaklar = [(0u16, 0u16, 0u16); 16];
        for bit in 1..=15usize {
            basamaklar[bit] = (ilk_kodlar[bit], ilk_indeksler[bit], uzunluk_sayilari[bit]);
        }

        Ok(Self {
            basamaklar,
            semboller,
            en_uzun,
        })
    }

    fn sabit_literal_uzunluk() -> Result<Self, SikistirmaHatasi> {
        let mut uz = vec![0u8; 288];
        for deger in uz.iter_mut().take(144) {
            *deger = 8;
        }
        for deger in uz.iter_mut().take(256).skip(144) {
            *deger = 9;
        }
        for deger in uz.iter_mut().take(280).skip(256) {
            *deger = 7;
        }
        for deger in uz.iter_mut().take(288).skip(280) {
            *deger = 8;
        }
        Self::olustur(&uz)
    }

    fn sabit_mesafe() -> Result<Self, SikistirmaHatasi> {
        let uz = vec![5u8; 32];
        Self::olustur(&uz)
    }

    fn sembol_oku(&self, okuyucu: &mut BitOkuyucu<'_>) -> Result<u16, SikistirmaHatasi> {
        let mut kod = 0u16;
        for bit in 1..=self.en_uzun {
            let b = okuyucu.bitleri_oku(1)? as u16;
            kod = (kod << 1) | b;
            let (ilk_kod, ilk_indeks, adet) = self.basamaklar[usize::from(bit)];
            if adet > 0 && kod >= ilk_kod && kod - ilk_kod < adet {
                let indeks = usize::from(ilk_indeks + (kod - ilk_kod));
                return Ok(self.semboller[indeks]);
            }
        }
        Err(SikistirmaHatasi::BozukVeri("geçersiz Huffman sembolü"))
    }
}

fn deflate_akisini_ac(
    okuyucu: &mut BitOkuyucu<'_>,
    cikti: &mut Vec<u8>,
    azami_bayt: usize,
) -> Result<(), SikistirmaHatasi> {
    loop {
        let son_blok = okuyucu.bitleri_oku(1)? == 1;
        let blok_turu = okuyucu.bitleri_oku(2)?;
        match blok_turu {
            0 => {
                // Sıkıştırılmamış (stored) blok.
                okuyucu.bayt_sinirina_hizala();
                let l0 = okuyucu.tam_bayt_oku()?;
                let l1 = okuyucu.tam_bayt_oku()?;
                let n0 = okuyucu.tam_bayt_oku()?;
                let n1 = okuyucu.tam_bayt_oku()?;
                let uzunluk = u16::from_le_bytes([l0, l1]);
                let ters_uzunluk = u16::from_le_bytes([n0, n1]);
                if uzunluk != !ters_uzunluk {
                    return Err(SikistirmaHatasi::BozukVeri(
                        "stored blok LEN/NLEN uyuşmazlığı",
                    ));
                }
                let eklenecek = usize::from(uzunluk);
                if cikti.len().saturating_add(eklenecek) > azami_bayt {
                    return Err(SikistirmaHatasi::AcilmisBoyutAsildi { sinir: azami_bayt });
                }
                cikti.reserve(eklenecek);
                for _ in 0..eklenecek {
                    cikti.push(okuyucu.tam_bayt_oku()?);
                }
            }
            1 => {
                let lit_agaci = HuffmanAgaci::sabit_literal_uzunluk()?;
                let mes_agaci = HuffmanAgaci::sabit_mesafe()?;
                huffman_blogu_ac(okuyucu, &lit_agaci, &mes_agaci, cikti, azami_bayt)?;
            }
            2 => {
                let (lit_agaci, mes_agaci) = dinamik_agaclari_oku(okuyucu)?;
                huffman_blogu_ac(okuyucu, &lit_agaci, &mes_agaci, cikti, azami_bayt)?;
            }
            _ => {
                return Err(SikistirmaHatasi::BozukVeri(
                    "geçersiz DEFLATE blok türü (3)",
                ));
            }
        }
        if son_blok {
            break;
        }
    }
    Ok(())
}

fn dinamik_agaclari_oku(
    okuyucu: &mut BitOkuyucu<'_>,
) -> Result<(HuffmanAgaci, HuffmanAgaci), SikistirmaHatasi> {
    let hlit = usize::from(okuyucu.bitleri_oku(5)? as u16 + 257);
    let hdist = usize::from(okuyucu.bitleri_oku(5)? as u16 + 1);
    let hclen = usize::from(okuyucu.bitleri_oku(4)? as u16 + 4);

    const SIRA: [usize; 19] = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut kod_uzunluklari = [0u8; 19];
    for &indeks in SIRA.iter().take(hclen) {
        kod_uzunluklari[indeks] = okuyucu.bitleri_oku(3)? as u8;
    }
    let kod_agaci = HuffmanAgaci::olustur(&kod_uzunluklari)?;

    let toplam = hlit + hdist;
    let mut uzunluklar = Vec::with_capacity(toplam);
    while uzunluklar.len() < toplam {
        let sembol = kod_agaci.sembol_oku(okuyucu)?;
        match sembol {
            0..=15 => uzunluklar.push(sembol as u8),
            16 => {
                let Some(&onceki) = uzunluklar.last() else {
                    return Err(SikistirmaHatasi::BozukVeri(
                        "dinamik Huffman RLE 16 önceki sembol olmadan geldi",
                    ));
                };
                let tekrar = usize::from(okuyucu.bitleri_oku(2)? as u8 + 3);
                if uzunluklar.len() + tekrar > toplam {
                    return Err(SikistirmaHatasi::BozukVeri(
                        "dinamik Huffman RLE uzunluğu tabloyu aştı",
                    ));
                }
                uzunluklar.resize(uzunluklar.len() + tekrar, onceki);
            }
            17 => {
                let tekrar = usize::from(okuyucu.bitleri_oku(3)? as u8 + 3);
                if uzunluklar.len() + tekrar > toplam {
                    return Err(SikistirmaHatasi::BozukVeri(
                        "dinamik Huffman RLE 17 tabloyu aştı",
                    ));
                }
                uzunluklar.resize(uzunluklar.len() + tekrar, 0);
            }
            18 => {
                let tekrar = usize::from(okuyucu.bitleri_oku(7)? as u8 + 11);
                if uzunluklar.len() + tekrar > toplam {
                    return Err(SikistirmaHatasi::BozukVeri(
                        "dinamik Huffman RLE 18 tabloyu aştı",
                    ));
                }
                uzunluklar.resize(uzunluklar.len() + tekrar, 0);
            }
            _ => {
                return Err(SikistirmaHatasi::BozukVeri(
                    "geçersiz dinamik Huffman kod uzunluğu sembolü",
                ));
            }
        }
    }

    let lit_agaci = HuffmanAgaci::olustur(&uzunluklar[..hlit])?;
    let mes_agaci = HuffmanAgaci::olustur(&uzunluklar[hlit..])?;
    Ok((lit_agaci, mes_agaci))
}

fn huffman_blogu_ac(
    okuyucu: &mut BitOkuyucu<'_>,
    lit_agaci: &HuffmanAgaci,
    mes_agaci: &HuffmanAgaci,
    cikti: &mut Vec<u8>,
    azami_bayt: usize,
) -> Result<(), SikistirmaHatasi> {
    const UZUNLUK_TABAN: [u16; 29] = [
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
        131, 163, 195, 227, 258,
    ];
    const UZUNLUK_EK_BIT: [u8; 29] = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];
    const MESAFE_TABAN: [u16; 30] = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];
    const MESAFE_EK_BIT: [u8; 30] = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];

    loop {
        let sembol = lit_agaci.sembol_oku(okuyucu)?;
        match sembol {
            0..=255 => {
                if cikti.len() >= azami_bayt {
                    return Err(SikistirmaHatasi::AcilmisBoyutAsildi { sinir: azami_bayt });
                }
                cikti.push(sembol as u8);
            }
            256 => break,
            257..=285 => {
                let uz_indeks = usize::from(sembol - 257);
                let ek_uz = okuyucu.bitleri_oku(UZUNLUK_EK_BIT[uz_indeks])? as usize;
                let uzunluk = usize::from(UZUNLUK_TABAN[uz_indeks]) + ek_uz;

                let mes_sembol = mes_agaci.sembol_oku(okuyucu)?;
                if mes_sembol >= 30 {
                    return Err(SikistirmaHatasi::BozukVeri("geçersiz DEFLATE mesafe kodu"));
                }
                let mes_indeks = usize::from(mes_sembol);
                let ek_mes = okuyucu.bitleri_oku(MESAFE_EK_BIT[mes_indeks])? as usize;
                let mesafe = usize::from(MESAFE_TABAN[mes_indeks]) + ek_mes;

                if mesafe > cikti.len() {
                    return Err(SikistirmaHatasi::BozukVeri(
                        "DEFLATE geri referans mesafesi çıktı tamponundan büyük",
                    ));
                }
                if cikti.len().saturating_add(uzunluk) > azami_bayt {
                    return Err(SikistirmaHatasi::AcilmisBoyutAsildi { sinir: azami_bayt });
                }
                cikti.reserve(uzunluk);
                for _ in 0..uzunluk {
                    let kaynak_bayt = cikti[cikti.len() - mesafe];
                    cikti.push(kaynak_bayt);
                }
            }
            _ => {
                return Err(SikistirmaHatasi::BozukVeri(
                    "geçersiz DEFLATE uzunluk sembolü",
                ));
            }
        }
    }
    Ok(())
}

/// RFC 1952 CRC-32 hesaplaması.
pub fn crc32_hesapla(veri: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &bayt in veri {
        crc ^= u32::from(bayt);
        for _ in 0..8 {
            let maske = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & maske);
        }
    }
    !crc
}

/// RFC 1950 Adler-32 hesaplaması.
pub fn adler32_hesapla(veri: &[u8]) -> u32 {
    const MOD_ADLER: u32 = 65_521;
    let mut a = 1u32;
    let mut b = 0u32;
    for &bayt in veri {
        a = (a + u32::from(bayt)) % MOD_ADLER;
        b = (b + a) % MOD_ADLER;
    }
    (b << 16) | a
}

/// Verilen baytları RFC 1952 GZIP biçiminde (stored DEFLATE bloklarıyla) paketler.
pub fn gzip_paketle(veri: &[u8]) -> Vec<u8> {
    let mut cikti = Vec::with_capacity(veri.len() + 32);
    // GZIP başlığı: ID1=0x1f, ID2=0x8b, CM=8, FLG=0, MTIME=0, XFL=0, OS=255
    cikti.extend_from_slice(&[0x1f, 0x8b, 0x08, 0x00, 0, 0, 0, 0, 0x00, 0xff]);

    if veri.is_empty() {
        cikti.push(0x01); // BFINAL=1, BTYPE=00
        cikti.extend_from_slice(&0u16.to_le_bytes());
        cikti.extend_from_slice(&(!0u16).to_le_bytes());
    } else {
        let parcalar: Vec<&[u8]> = veri.chunks(65_535).collect();
        for (indeks, parca) in parcalar.iter().enumerate() {
            let son = indeks + 1 == parcalar.len();
            cikti.push(u8::from(son));
            let uz = parca.len() as u16;
            cikti.extend_from_slice(&uz.to_le_bytes());
            cikti.extend_from_slice(&(!uz).to_le_bytes());
            cikti.extend_from_slice(parca);
        }
    }

    let crc = crc32_hesapla(veri);
    cikti.extend_from_slice(&crc.to_le_bytes());
    cikti.extend_from_slice(&(veri.len() as u32).to_le_bytes());
    cikti
}

/// Yüksek sıkıştırma oranlı geçerli bir RFC 1952 GZIP bombası üretir.
///
/// Sabit Huffman (BTYPE=01) RLE geri referansları (`uzunluk=258, mesafe=1`)
/// kullanarak birkaç on kilobaytlık GZIP çıktısının `acilmis_hedef_bayt`
/// boyutuna (ör. 10 MiB) açılmasını sağlar.
pub fn gzip_bomba_uret(acilmis_hedef_bayt: usize) -> Vec<u8> {
    struct BitYazici {
        baytlar: Vec<u8>,
        tampon: u64,
        bit_sayisi: u8,
    }

    impl BitYazici {
        fn yeni() -> Self {
            Self {
                baytlar: vec![0x1f, 0x8b, 0x08, 0x00, 0, 0, 0, 0, 0x00, 0xff],
                tampon: 0,
                bit_sayisi: 0,
            }
        }

        fn lsb_bit_yaz(&mut self, deger: u32, adet: u8) {
            self.tampon |= u64::from(deger) << self.bit_sayisi;
            self.bit_sayisi += adet;
            while self.bit_sayisi >= 8 {
                self.baytlar.push((self.tampon & 0xFF) as u8);
                self.tampon >>= 8;
                self.bit_sayisi -= 8;
            }
        }

        fn msb_kod_yaz(&mut self, kod: u16, adet: u8) {
            for kaydir in (0..adet).rev() {
                let bit = u32::from((kod >> kaydir) & 1);
                self.lsb_bit_yaz(bit, 1);
            }
        }

        fn bitir(mut self) -> Vec<u8> {
            if self.bit_sayisi > 0 {
                self.baytlar.push((self.tampon & 0xFF) as u8);
            }
            self.baytlar
        }
    }

    let mut yazici = BitYazici::yeni();
    // BFINAL=1 (1 bit), BTYPE=01 (2 bit, sabit Huffman).
    yazici.lsb_bit_yaz(1, 1);
    yazici.lsb_bit_yaz(1, 2);

    if acilmis_hedef_bayt > 0 {
        // İlk bayt: 'A' (65). Sabit Huffman kodu: 65 + 0x30 = 0x71 (8 bit).
        yazici.msb_kod_yaz(u16::from(b'A') + 0x30, 8);
        let mut kalan = acilmis_hedef_bayt - 1;

        while kalan >= 258 {
            // Sembol 285 (uzunluk 258): (285 - 280) + 0xC0 = 0xC5 (8 bit).
            yazici.msb_kod_yaz(0xC5, 8);
            // Mesafe 1 (sembol 0): 5 sıfır biti.
            yazici.msb_kod_yaz(0, 5);
            kalan -= 258;
        }

        while kalan >= 3 {
            // Sembol 257 (uzunluk 3): 257 - 256 = 0x01 (7 bit).
            yazici.msb_kod_yaz(0x01, 7);
            // Mesafe 1 (sembol 0): 5 sıfır biti.
            yazici.msb_kod_yaz(0, 5);
            kalan -= 3;
        }

        while kalan > 0 {
            yazici.msb_kod_yaz(u16::from(b'A') + 0x30, 8);
            kalan -= 1;
        }
    }

    // Blok sonu sembolü 256: 7 sıfır biti.
    yazici.msb_kod_yaz(0, 7);

    let mut cikti = yazici.bitir();
    // CRC32 ve ISIZE (doğrulama için, bomba 4 MiB sınırında daha önce kesilir).
    let ornek = vec![b'A'; acilmis_hedef_bayt];
    let crc = crc32_hesapla(&ornek);
    cikti.extend_from_slice(&crc.to_le_bytes());
    cikti.extend_from_slice(&(acilmis_hedef_bayt as u32).to_le_bytes());
    cikti
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn gzip_paketle_ve_ac_yuvarlak_tur_calisir() {
        let orijinal =
            b"Merhaba Kalfa web araci! UTF-8: \xc3\xbc\xc5\x9f\xc4\x9f\xc4\xb1\xc3\xb6\xc3\xa7";
        let sikistirilmis = gzip_paketle(orijinal);
        let acilmis = ac_sinirli(
            &sikistirilmis,
            SikistirmaBicimi::Gzip,
            AZAMI_ACILMIS_GOVDE_BAYT,
        )
        .expect("gzip açılmalı");
        assert_eq!(acilmis, orijinal);
    }

    #[test]
    fn gzip_bomba_sinir_icinde_acilir_sinir_asiminda_reddedilir() {
        let kucuk = gzip_bomba_uret(2048);
        assert!(kucuk.len() < 160);
        let acilmis = ac_sinirli(&kucuk, SikistirmaBicimi::Gzip, 4096).expect("sınır içi açılmalı");
        assert_eq!(acilmis.len(), 2048);
        assert!(acilmis.iter().all(|&b| b == b'A'));

        let bomba = gzip_bomba_uret(AZAMI_ACILMIS_GOVDE_BAYT + 1024);
        assert!(bomba.len() < 64 * 1024);
        let sonuc = ac_sinirli(&bomba, SikistirmaBicimi::Gzip, AZAMI_ACILMIS_GOVDE_BAYT);
        assert_eq!(
            sonuc,
            Err(SikistirmaHatasi::AcilmisBoyutAsildi {
                sinir: AZAMI_ACILMIS_GOVDE_BAYT
            })
        );
    }
}
