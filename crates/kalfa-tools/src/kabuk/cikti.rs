//! Kabuk çıktısının işlenmesi (aşama 2 tasarımı §4.4): modele giden baş ve
//! son, arayüze giden canlı akış.

use std::collections::VecDeque;
use std::fmt::Write;

use kalfa_core::arac::CiktiGonderici;
use kalfa_core::protokol::CiktiAkisi;
use kalfa_sandbox::Akis;
use regex::Regex;

/// Modele giden çıktının baştan ve sondan tutulan bayt sayısı.
pub const BAS_BAYT: usize = 10_000;
pub const SON_BAYT: usize = 20_000;
/// Arayüze bir çağrı için giden canlı çıktının üst sınırı. Terminal görünümü
/// zaten en çok 200 000 karakter tutar (aşama 2 tasarımı §10.1).
pub const CANLI_SINIR: usize = 1024 * 1024;

/// Çıktının ilk `BAS_BAYT`'ını ve son `SON_BAYT`'ını tutar; bellek çıktının
/// boyutundan bağımsızdır.
#[derive(Debug, Default)]
pub struct BasSonTampon {
    bas: Vec<u8>,
    son: VecDeque<u8>,
    toplam: u64,
}

impl BasSonTampon {
    pub fn ekle(&mut self, veri: &[u8]) {
        self.toplam += veri.len() as u64;
        let bas_payi = (BAS_BAYT - self.bas.len()).min(veri.len());
        self.bas.extend_from_slice(&veri[..bas_payi]);
        let kalan = &veri[bas_payi..];
        let kalan = &kalan[kalan.len().saturating_sub(SON_BAYT)..];
        self.son.extend(kalan);
        let fazla = self.son.len().saturating_sub(SON_BAYT);
        self.son.drain(..fazla);
    }

    pub fn toplam(&self) -> u64 {
        self.toplam
    }

    /// Modele gidecek metin: kırpıldıysa baş, kırpılan bayt sayısı ve son.
    pub fn metin(&self) -> String {
        let son: Vec<u8> = self.son.iter().copied().collect();
        let kirpilan = self.toplam - (self.bas.len() + son.len()) as u64;
        if kirpilan == 0 {
            let mut hepsi = self.bas.clone();
            hepsi.extend_from_slice(&son);
            return temizle(&hepsi);
        }
        let bas = bas_sinirinda_kes(&self.bas);
        let son = son_sinirinda_kes(&son);
        let atilan =
            kirpilan + (self.bas.len() - bas.len()) as u64 + (self.son.len() - son.len()) as u64;
        let mut metin = temizle(bas);
        let _ = write!(metin, "\n[... {atilan} bayt kırpıldı ...]\n");
        metin.push_str(&temizle(son));
        metin
    }
}

/// Sonda yarım kalmış çok baytlı karakteri atar.
fn bas_sinirinda_kes(veri: &[u8]) -> &[u8] {
    let gecerli = match std::str::from_utf8(veri) {
        Ok(_) => return veri,
        Err(e) if e.error_len().is_none() => e.valid_up_to(),
        Err(_) => return veri,
    };
    &veri[..gecerli]
}

/// Başta kalan devam baytlarını (yarım karakterin arkası) atar.
fn son_sinirinda_kes(veri: &[u8]) -> &[u8] {
    let atla = veri
        .iter()
        .take(3)
        .take_while(|b| (**b & 0b1100_0000) == 0b1000_0000)
        .count();
    &veri[atla..]
}

/// Desen sabittir ve testler derlendiğini sınar; derlenemezse `None` döner ve
/// kaçışlar atılmaz.
fn ansi_deseni() -> Option<&'static Regex> {
    static DESEN: std::sync::OnceLock<Option<Regex>> = std::sync::OnceLock::new();
    DESEN
        .get_or_init(|| {
            Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b[0-~]").ok()
        })
        .as_ref()
}

/// Modele gidecek metin: geçersiz UTF-8 `U+FFFD` olur, ANSI kaçışları atılır.
/// Satır başı (`\r`) ile üzerine yazılan satırdan (ilerleme çubuğu) yalnızca
/// son hâli kalır.
pub fn temizle(veri: &[u8]) -> String {
    let metin = String::from_utf8_lossy(veri);
    let metin = match ansi_deseni() {
        Some(desen) => desen.replace_all(&metin, ""),
        None => metin,
    };
    let mut cikti = String::with_capacity(metin.len());
    for (i, satir) in metin.split('\n').enumerate() {
        if i > 0 {
            cikti.push('\n');
        }
        let satir = satir.strip_suffix('\r').unwrap_or(satir);
        cikti.push_str(satir.rsplit('\r').next().unwrap_or(""));
    }
    cikti
}

/// Arayüze giden canlı akış. Parçaların ortasında bölünen çok baytlı
/// karakterler bir sonraki parçaya kalır; `sinir` aşılınca bir kez bildirilir
/// ve akış durur.
pub struct CanliAkis {
    gonderici: CiktiGonderici,
    sinir: usize,
    gonderilen: usize,
    bekleyen: [Vec<u8>; 2],
    durdu: bool,
}

impl CanliAkis {
    pub fn yeni(gonderici: CiktiGonderici, sinir: usize) -> Self {
        Self {
            gonderici,
            sinir,
            gonderilen: 0,
            bekleyen: [Vec::new(), Vec::new()],
            durdu: false,
        }
    }

    pub fn ekle(&mut self, akis: Akis, veri: &[u8]) {
        if self.durdu {
            return;
        }
        if self.gonderilen >= self.sinir {
            self.durdu = true;
            self.gonderici.gonder(
                CiktiAkisi::Standart,
                format!("\n[Çıktının kalanı gösterilmiyor; model son {SON_BAYT} baytı alacak.]\n"),
            );
            return;
        }
        let (sira, cikti_akisi) = match akis {
            Akis::Standart => (0, CiktiAkisi::Standart),
            Akis::Hata => (1, CiktiAkisi::Hata),
        };
        let bekleyen = &mut self.bekleyen[sira];
        bekleyen.extend_from_slice(veri);
        let tam = bas_sinirinda_kes(bekleyen).len();
        let kalan = bekleyen.split_off(tam);
        let metin = String::from_utf8_lossy(bekleyen).into_owned();
        *bekleyen = kalan;
        if !metin.is_empty() {
            self.gonderilen += metin.len();
            self.gonderici.gonder(cikti_akisi, metin);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kucuk_cikti_kirpilmaz() {
        let mut t = BasSonTampon::default();
        t.ekle(b"merhaba\n");
        t.ekle(b"dunya");
        assert_eq!(t.metin(), "merhaba\ndunya");
    }

    #[test]
    fn buyuk_cikti_bastan_ve_sondan_kirpilir() {
        let mut t = BasSonTampon::default();
        let parca = vec![b'a'; 7_000];
        for _ in 0..10 {
            t.ekle(&parca);
        }
        t.ekle(b"SON");
        let m = t.metin();
        assert!(m.starts_with(&"a".repeat(BAS_BAYT)));
        assert!(m.ends_with("SON"));
        assert!(
            m.contains("[... 40003 bayt kırpıldı ...]"),
            "{}",
            &m[BAS_BAYT..BAS_BAYT + 60]
        );
        assert_eq!(t.toplam(), 70_003);
    }

    #[test]
    fn kirpma_cok_bayti_karakteri_bolmez() {
        let mut t = BasSonTampon::default();
        // 9_999 bayt + "ş" (2 bayt): baş sınırı karakterin ortasına düşer.
        t.ekle(&vec![b'a'; BAS_BAYT - 1]);
        t.ekle("ş".repeat(20_000).as_bytes());
        let m = t.metin();
        assert!(!m.contains('\u{FFFD}'), "yarım karakter kalmamalı");
        assert!(m.ends_with('ş'));
    }

    #[test]
    fn ansi_kacislari_ve_ilerleme_satirlari_temizlenir() {
        assert_eq!(temizle(b"\x1b[1;31mhata\x1b[0m\n"), "hata\n");
        assert_eq!(temizle(b"%10\r%50\r%100\nbitti\r\n"), "%100\nbitti\n");
        assert_eq!(temizle(b"\x1b]0;baslik\x07metin"), "metin");
        assert_eq!(temizle(b"a\x1b7b\x1bMc"), "abc");
        assert_eq!(temizle(&[0x66, 0xff, 0x67]), "f\u{FFFD}g");
    }

    #[tokio::test]
    async fn canli_akis_karakteri_bolmez_ve_sinirda_durur() {
        let (gonderici, mut rx) = kanal();
        let mut canli = CanliAkis::yeni(gonderici, 8);
        let s = "ş".as_bytes();
        canli.ekle(Akis::Standart, &[b'a', s[0]]);
        canli.ekle(Akis::Standart, &[s[1], b'b']);
        canli.ekle(Akis::Hata, b"12345678");
        canli.ekle(Akis::Standart, b"gitmez");
        canli.ekle(Akis::Standart, b"gitmez");
        let mut gelen = Vec::new();
        while let Ok(p) = rx.try_recv() {
            gelen.push((p.akis, p.metin));
        }
        assert_eq!(gelen[0], (CiktiAkisi::Standart, "a".into()));
        assert_eq!(gelen[1], (CiktiAkisi::Standart, "şb".into()));
        assert_eq!(gelen[2], (CiktiAkisi::Hata, "12345678".into()));
        assert!(gelen[3].1.contains("gösterilmiyor"));
        assert_eq!(gelen.len(), 4, "sınırdan sonra akış durur");
    }

    fn kanal() -> (
        CiktiGonderici,
        tokio::sync::mpsc::Receiver<kalfa_core::arac::CiktiParcasi>,
    ) {
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        (
            CiktiGonderici::yeni(kalfa_core::kimlik::CagriId::yeni(), tx),
            rx,
        )
    }
}
