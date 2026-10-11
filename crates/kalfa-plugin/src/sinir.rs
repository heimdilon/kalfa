//! Kaynak sınırları (aşama 4 tasarımı §7.4). v1'de uygulama sabitleridir
//! (karar 10); manifestle değişen tek sınır depo kotasıdır.

use std::time::Duration;

/// Doğrusal bellek, örnek başına.
pub const BELLEK: usize = 512 * 1024 * 1024;
pub const TABLO_OGESI: usize = 100_000;
pub const ORNEK: usize = 16;
pub const BELLEK_SAYISI: usize = 4;
pub const TABLO: usize = 64;
/// Epoch tikinin aralığı.
pub const TIK: Duration = Duration::from_millis(10);
/// WebAssembly yürütme süresi, çağrı başına (60 sn = 6 000 tik).
pub const CPU: Duration = Duration::from_secs(60);
/// Çağrının toplam süresi, host çağrıları dahil.
pub const TOPLAM: Duration = Duration::from_secs(30 * 60);
/// `araclari-listele`.
pub const LISTE_SURESI: Duration = Duration::from_secs(5);
/// İptalden sonra eklentiye çıkması için verilen süre (§7.5).
pub const IPTAL_SURESI: Duration = Duration::from_secs(2);
/// `dosya-listele` sonucu.
pub const LISTE: usize = 100_000;
/// `dosya-oku`: tek dosya.
pub const DOSYA: u64 = 4 * 1024 * 1024;
/// `dosya-oku`: çağrı başına toplam.
pub const CAGRI_OKUMASI: u64 = 1024 * 1024 * 1024;
pub const GUNLUK_SATIRI: u32 = 200;
pub const GUNLUK_BAYTI: usize = 4 * 1024;
/// `sonuc.icerik`.
pub const SONUC: usize = 64 * 1024;
pub const HATA_ILETISI: usize = 4 * 1024;
/// `stderr`'in tutulan sonu.
pub const STDERR: usize = 4 * 1024;
/// `evren-cagir`: gövde (§7.4).
pub const EVREN_GOVDESI: usize = 4 * 1024 * 1024;
/// `evren-cagir`: yanıt.
pub const EVREN_YANITI: usize = 32 * 1024 * 1024;
/// `evren-cagir`: çağrı başına istek.
pub const CAGRI_EVREN_ISTEGI: u32 = 1_000;
/// `evren-cagir`: istek süresi.
pub const EVREN_SURESI: Duration = Duration::from_secs(120);
/// Uygulama genelinde aynı anda çalışan eklenti çağrısı.
pub const ESZAMANLI: usize = 4;
pub const ARAC_ACIKLAMASI: usize = 1_000;
pub const SEMA: usize = 16 * 1024;

/// Çağrının süre sınırları. Ürün yolu hep [`Sureler::default`]'u kullanır;
/// testler kısaltabilir (`test-destek`).
#[derive(Debug, Clone, Copy)]
pub struct Sureler {
    pub cpu: Duration,
    pub toplam: Duration,
}

impl Default for Sureler {
    fn default() -> Self {
        Self {
            cpu: CPU,
            toplam: TOPLAM,
        }
    }
}

impl Sureler {
    pub(crate) fn cpu_tik(&self) -> u64 {
        u64::try_from(self.cpu.as_millis() / TIK.as_millis()).unwrap_or(u64::MAX)
    }
}
