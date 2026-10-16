//! Core araç sözleşmesini yerel MCP çağrı yöneticisine bağlayan ince sarmalayıcı.
//!
//! App katmanı `McpAracYurutucusu` uygulamasını sağlar. O uygulama R/D'yi RPC
//! öncesinde yeniden doğrular, kilidi bağımsız çağrı kaydına alır ve yalnız
//! maskelenmiş `AracCiktisi` döndürür.

use std::path::PathBuf;
use std::sync::Arc;

use crate::CallToolResult;
pub use kalfa_core::arac::async_trait;
use kalfa_core::arac::{
    Arac, AracCiktisi, AracHatasi, AracOrtami, Hazirlik, McpAracHazirligi, McpKartEtiketi,
};
use kalfa_core::kilit::KilitSeti;
use kalfa_core::kimlik::CagriId;
use kalfa_sandbox::tarama::CancellationToken;
use serde_json::{Map, Value};
use tokio::sync::oneshot;

const OTURUM_GEREKLI: &str = "MCP aracı yalnız oturumun çağrı yöneticisiyle çalıştırılabilir.";
const KILIT_GEREKLI: &str = "MCP çağrısı için çalışma alanı kilidi alınamadı.";
const GUVENLI_CAGRI_HATASI: &str = "MCP çağrısı güvenli biçimde tamamlanamadı.";

/// Uzak araçtan G7 tanım denetimi geçmiş, model ve kullanıcı yüzeyinde
/// gösterilmesine izin verilen alanlar. Ham RMCP `Tool` nesnesi burada tutulmaz.
pub struct McpAracSunumu {
    pub ad: String,
    pub aciklama: String,
    pub parametre_semasi: Value,
}

/// App yöneticisinin çağrı için ihtiyaç duyduğu, UI'ya verilmeyen kimlikler.
/// Sunucu ve uzak araç adları, tanım denetiminde onaylanmış kart etiketleridir.
pub struct McpAracKimligi {
    alan_kimligi: String,
    kok: PathBuf,
    sunucu: McpKartEtiketi,
    arac: McpKartEtiketi,
}

impl McpAracKimligi {
    pub fn yeni(
        alan_kimligi: String,
        kok: PathBuf,
        sunucu: McpKartEtiketi,
        arac: McpKartEtiketi,
    ) -> Self {
        Self {
            alan_kimligi,
            kok,
            sunucu,
            arac,
        }
    }

    pub fn alan_kimligi(&self) -> &str {
        &self.alan_kimligi
    }

    pub fn kok(&self) -> &std::path::Path {
        &self.kok
    }

    pub fn sunucu(&self) -> &McpKartEtiketi {
        &self.sunucu
    }

    pub fn arac(&self) -> &McpKartEtiketi {
        &self.arac
    }
}

/// Kilit ve model argümanları bu sahipli istekle çağrı kaydına devredilir.
/// App uygulaması bu değeri almadan RPC başlatmamalıdır.
pub struct McpAracCagriIstegi {
    pub cagri: CagriId,
    pub kimlik: Arc<McpAracKimligi>,
    pub hazirlik: McpAracHazirligi,
    pub argumanlar: Map<String, Value>,
    pub iptal: CancellationToken,
    pub kilit: Option<KilitSeti>,
}

/// G7'nin her çağrı için bağladığı zorunlu sır maskeli yanıt dönüşümü.
/// RMCP sonucu yalnız çağrı worker'ı içindeki bu closure'a verilir.
pub type McpAracSonucDonusturucu =
    Arc<dyn Fn(CallToolResult, bool) -> Result<AracCiktisi, AracHatasi> + Send + Sync>;

/// Uygulama yöneticisiyle tek dikiş. Her metot zorunludur; varsayılan izin,
/// güncellik veya temizlik davranışı yoktur.
#[async_trait]
pub trait McpAracYurutucusu: Send + Sync {
    /// Taze hazırlık await'inden önce çağrının guard'ını kaydeder. Burada RPC yoktur.
    fn kilidi_devral(&self, istek: McpAracCagriIstegi) -> Result<(), AracHatasi>;

    /// Güncel T/D/Π/K/P/N'den kartın R ve D'sini üretir. Core aynı metodu
    /// onaydan sonra, çalışma alanı kilidi altındayken tekrar çağırır.
    async fn guncel_hazirlik(
        &self,
        kimlik: &McpAracKimligi,
    ) -> Result<McpAracHazirligi, AracHatasi>;

    /// Bağımsız worker'ın RPC'den hemen önce yaptığı salt doğrulama. App
    /// uygulaması aynı T/D/Π/K/P/N kaynağını kullanıp onaylı kartla eşitliği
    /// zorunlu tutar; bu dikişte varsayılan allow yolu bulunmaz.
    async fn guncelligi_dogrula(
        &self,
        kimlik: &McpAracKimligi,
        onayli: &McpAracHazirligi,
    ) -> Result<(), AracHatasi>;

    /// Çağrı kaydını ve guard'ı eşzamanlı kurup bağımsız RPC görevini başlatır.
    /// Görev RPC'den hemen önce R/D'yi tekrar doğrular. Alıcı bırakılırsa,
    /// iptal/deadline gelirse ya da süreç durursa cleanup başlatır.
    fn cagriyi_baslat(
        self: Arc<Self>,
        istek: McpAracCagriIstegi,
    ) -> Result<oneshot::Receiver<Result<AracCiktisi, AracHatasi>>, AracHatasi>;

    /// Core sonuç denetimi ve kaydı bitirdikten sonra son koruma denetimini
    /// yapar. Yalnız tam başarılı denetim çağrı kaydını ve kilidi çözer.
    async fn sonuc_isleme_bitti(&self, cagri: CagriId) -> Result<(), AracHatasi>;

    /// Core stale onay yüzünden yeni kart istemeden önce bağımsız cleanup'ın
    /// ve guard çözümünün tamamlandığını bekler.
    async fn sonuc_isleme_iptal_ve_bekle(&self, cagri: CagriId) -> Result<(), AracHatasi>;

    /// Core'un sonuç işlemesi düşerse bağımsız cleanup başlatır. Bu çağrı
    /// bloklamaz ve çağrı kaydını/kilidini kendisi bırakmaz.
    fn sonuc_isleme_iptal(&self, cagri: CagriId);
}

/// MCP sunucusunun keşfedilmiş ve denetlenmiş tek araç tanımı.
pub struct McpArac {
    kimlik: Arc<McpAracKimligi>,
    sunum: McpAracSunumu,
    yurutucu: Arc<dyn McpAracYurutucusu>,
}

impl McpArac {
    pub fn yeni(
        kimlik: McpAracKimligi,
        sunum: McpAracSunumu,
        yurutucu: Arc<dyn McpAracYurutucusu>,
    ) -> Self {
        Self {
            kimlik: Arc::new(kimlik),
            sunum,
            yurutucu,
        }
    }

    fn guvenli_hata() -> AracHatasi {
        AracHatasi::Gecersiz(GUVENLI_CAGRI_HATASI.to_owned())
    }
}

#[async_trait]
impl Arac for McpArac {
    fn ad(&self) -> &str {
        &self.sunum.ad
    }

    fn aciklama(&self) -> &str {
        &self.sunum.aciklama
    }

    fn parametre_semasi(&self) -> Value {
        self.sunum.parametre_semasi.clone()
    }

    /// MCP araçlarının yan etkileri önceden yol listesine indirgenemez.
    fn yazar_mi(&self) -> bool {
        true
    }

    async fn hazirla(&self, _argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
        let mcp = self.yurutucu.guncel_hazirlik(&self.kimlik).await?;
        if &mcp.sunucu != self.kimlik.sunucu() || &mcp.arac != self.kimlik.arac() {
            return Err(AracHatasi::OnayEskidi);
        }
        Ok(Hazirlik {
            aciklama: "Yerel MCP aracını çağırmak istiyor.".to_owned(),
            tum_calisma_alani: true,
            kilitsiz: false,
            mcp: Some(mcp),
            ..Hazirlik::default()
        })
    }

    /// Kilitsiz/oturumsuz çalıştırma yolunu kapatır. Normal akışta core,
    /// `calistir_kilitli` metodunu çağırır ve güvenilir CagriId'yi ortamdan alır.
    async fn calistir(
        &self,
        _argumanlar: &Value,
        _hazirlik: &Hazirlik,
        _iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        Err(AracHatasi::Gecersiz(OTURUM_GEREKLI.to_owned()))
    }

    async fn calistir_kilitli(
        &self,
        argumanlar: &Value,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
        ortam: AracOrtami,
        kilit: Option<KilitSeti>,
    ) -> Result<AracCiktisi, AracHatasi> {
        let cagri = ortam
            .cagri_kimligi()
            .ok_or_else(|| AracHatasi::Gecersiz(OTURUM_GEREKLI.to_owned()))?;
        let mcp = hazirlik.mcp.clone().ok_or_else(Self::guvenli_hata)?;
        if &mcp.sunucu != self.kimlik.sunucu() || &mcp.arac != self.kimlik.arac() {
            return Err(AracHatasi::OnayEskidi);
        }
        let argumanlar = argumanlar
            .as_object()
            .cloned()
            .ok_or_else(|| AracHatasi::Gecersiz("MCP araç girdisi nesne olmalıdır.".to_owned()))?;

        // Bu senkron çağrı kilidi yönetici kaydına taşır ve bağımsız görevi
        // başlatır. İlk await ancak sahiplik devrinden sonra gerçekleşir.
        let yanit = Arc::clone(&self.yurutucu).cagriyi_baslat(McpAracCagriIstegi {
            cagri,
            kimlik: Arc::clone(&self.kimlik),
            hazirlik: mcp,
            argumanlar,
            iptal,
            kilit,
        })?;
        yanit.await.unwrap_or_else(|_| Err(Self::guvenli_hata()))
    }

    fn kilidi_devral(
        &self,
        cagri: CagriId,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
        kilit: Option<KilitSeti>,
    ) -> Result<Option<KilitSeti>, AracHatasi> {
        let kilit = kilit.ok_or_else(|| AracHatasi::Gecersiz(KILIT_GEREKLI.to_owned()))?;
        let mcp = hazirlik.mcp.clone().ok_or_else(Self::guvenli_hata)?;
        self.yurutucu.kilidi_devral(McpAracCagriIstegi {
            cagri,
            kimlik: self.kimlik.clone(),
            hazirlik: mcp,
            argumanlar: Map::new(),
            iptal,
            kilit: Some(kilit),
        })?;
        Ok(None)
    }

    async fn sonuc_isleme_bitti(&self, cagri: CagriId) -> Result<(), AracHatasi> {
        self.yurutucu.sonuc_isleme_bitti(cagri).await
    }

    async fn sonuc_isleme_iptal_ve_bekle(&self, cagri: CagriId) -> Result<(), AracHatasi> {
        self.yurutucu.sonuc_isleme_iptal_ve_bekle(cagri).await
    }

    fn sonuc_isleme_iptal(&self, cagri: CagriId) {
        self.yurutucu.sonuc_isleme_iptal(cagri);
    }
}
