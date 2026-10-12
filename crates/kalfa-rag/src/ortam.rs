//! Eklentinin dış dünyası. Eklentide host çağrılarıyla (`crate::HostOrtami`),
//! birim testlerinde bellekte (`crate::test_ortami::SahteOrtam`) karşılanır.
//! İndeks ve arama yalnızca bu trait'i görür; böylece host hedefinde, wasm
//! derlemeden sınanır (aşama 4 tasarımı §16.4).

use kalfa_plugin_sdk::{DosyaBilgisi, Hata};

/// Gömme modeli ve boyutu (aşama 4 tasarımı §11.2).
pub const GOMME_MODELI: &str = "qwen3-embedding-8b";
pub const BOYUT: usize = 1024;
/// Yeniden sıralama modeli (§11.3).
pub const SIRALAMA_MODELI: &str = "qwen3-reranker-8b";

pub trait Ortam {
    /// `desen` ∩ manifest globları; gizli dosyalar yok; yola göre sıralı.
    fn dosya_listele(&self, desen: &str) -> Result<Vec<DosyaBilgisi>, Hata>;
    /// Dosyanın UTF-8 içeriği.
    fn dosya_oku(&self, yol: &str) -> Result<String, Hata>;
    /// Metinleri `GOMME_MODELI` ile `BOYUT` boyutta gömer. Her metin için
    /// birim uzunlukta bir vektör, aynı sırayla. Tek istek; toplu isteğin
    /// sınırlarını (§11.2) çağıran gözetir.
    fn gomme(&self, metinler: &[String]) -> Result<Vec<Vec<f32>>, Hata>;
    /// Belgeleri `SIRALAMA_MODELI` ile sorguya göre sıralar: (belgenin
    /// sırası, puan), puana göre azalan, en çok `ilk_n` tane.
    fn sirala(
        &self,
        sorgu: &str,
        belgeler: &[String],
        ilk_n: u32,
    ) -> Result<Vec<(usize, f32)>, Hata>;
    fn depo_oku(&self, anahtar: &str) -> Result<Option<Vec<u8>>, Hata>;
    /// Atomik: ya eski değer kalır ya yenisi yazılır.
    fn depo_yaz(&self, anahtar: &str, deger: &[u8]) -> Result<(), Hata>;
    /// Olmayan anahtarı silmek hata değildir.
    fn depo_sil(&self, anahtar: &str) -> Result<(), Hata>;
    fn ilerleme(&self, tamamlanan: u64, toplam: Option<u64>, mesaj: Option<String>);
    fn iptal_istendi(&self) -> bool;
    /// Unix epoch'tan beri milisaniye.
    fn simdi_ms(&self) -> u64;
}
