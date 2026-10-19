//! Uygulama ayarları (aşama 3 tasarımı §7.6; 3c Görev 7): veri dizinindeki
//! `ayarlar.json`. Bu görevde yalnızca denetçi kipi vardır; 3e gizli
//! kalıpları ve ağ listesini ekler. Dosya veri dizinindedir, yalıtımın içinde
//! görünmez; depo içindeki hiçbir dosya ayarı değiştiremez.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use kalfa_core::denetci::DenetimKipi;
use kalfa_core::izin::{
    AZAMI_KALIP, GizliAyarlari, KalipHatasi, VARSAYILAN_GIZLI_KALIPLAR, kalip_dogrula,
};
use kalfa_sandbox::vekil::{HostKaynagi, HostKurali, ag_hostu_coz};
use serde::{Deserialize, Deserializer, Serialize};

use crate::hata::KomutHatasi;
use crate::sozlesme::{AgHostu, DenetciDurumu, GizliAyarGorunumu};
use crate::uygulama::Uygulama;

const DOSYA_ADI: &str = "ayarlar.json";

const VARSAYILAN_ETKIN_PENCERE: u64 = 200_000;
const ASGARI_ETKIN_PENCERE: u64 = 32_000;
const AZAMI_ETKIN_PENCERE: u64 = 9_007_199_254_740_991;

const ETKIN_PENCERE_GECERSIZ: &str = "Etkin bağlam penceresi geçersiz. Değer 32000 ile 9007199254740991 arasında bir tam sayı olmalı.";

const DUSURME_ONAYI: &str = "Denetçi kipini düşürmek, dış kaynaklı araç sonuçlarının \
(dosya içeriği, komut çıktısı, arama sonuçları) modele daha az denetimle gitmesi demektir. \
Denetçi izinlerin yerine geçmez, ama içeriğe gömülü talimatları yakalayabilir. Devam etmek \
istiyor musunuz?";

const EK_KALIP_KALDIRMA: &str = "Bu kalıbı kaldırınca ona uyan dosyalar gizli sayılmayabilir: \
onaysız okunup EVREN'e gidebilir ve yalıtımda okunabilir.";

/// Aşama 3 tasarımı §9.2'deki cümle.
const ISTISNA_EKLEME: &str = "Bu kalıba uyan dosyalar gizli sayılmayacak: onaysız okunup \
EVREN'e gidebilir ve yalıtımda okunabilir.";

const OZEL_AG: &str = "Bu host özel ağ adreslerine (yerel ağ, şirket ağı) de bağlanabilecek.";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Dosya {
    #[serde(default)]
    denetci_kipi: DenetimKipi,
    /// Kullanıcının ek gizli kalıpları (3e).
    #[serde(default)]
    ek_gizli_kaliplar: Vec<String>,
    /// Gizli sayılmayanlar (K7).
    #[serde(default)]
    gizli_sayilmayanlar: Vec<String>,
    /// Ağlı komutların kullanıcı listesi (3d, §8.3).
    #[serde(default)]
    ag_izin_listesi: Vec<AgHostu>,
    /// Yeni ve açılan oturumların etkin bağlam penceresi.
    #[serde(
        default = "varsayilan_etkin_pencere",
        deserialize_with = "etkin_pencere_oku"
    )]
    etkin_pencere: u64,
    /// MCP sunucu tanımları ve yetim sır temizlik kuyruğu (1.1a-2 Görev 3).
    #[serde(default)]
    mcp: crate::mcp::ayarlar::McpAyarlar,
    /// MCP ayarları için bellek içi monotonik CAS revizyonu.
    #[serde(skip)]
    mcp_revizyonu: u64,
}

impl Default for Dosya {
    fn default() -> Self {
        Self {
            denetci_kipi: DenetimKipi::default(),
            ek_gizli_kaliplar: Vec::new(),
            gizli_sayilmayanlar: Vec::new(),
            ag_izin_listesi: Vec::new(),
            etkin_pencere: VARSAYILAN_ETKIN_PENCERE,
            mcp: crate::mcp::ayarlar::McpAyarlar::default(),
            mcp_revizyonu: 0,
        }
    }
}

fn varsayilan_etkin_pencere() -> u64 {
    VARSAYILAN_ETKIN_PENCERE
}

fn etkin_pencere_gecerli(deger: u64) -> bool {
    (ASGARI_ETKIN_PENCERE..=AZAMI_ETKIN_PENCERE).contains(&deger)
}

/// Bozuk ya da geçersiz etkin pencere (tür hatası, alt sınır altı, sınır üstü)
/// yalnızca kendisini düşürür: varsayılan kullanılır, dosyanın diğer ayarları
/// okunmaya devam eder.
fn etkin_pencere_oku<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    let ham = serde_json::Value::deserialize(d)?;
    Ok(ham
        .as_u64()
        .filter(|&v| etkin_pencere_gecerli(v))
        .unwrap_or(VARSAYILAN_ETKIN_PENCERE))
}

pub struct Ayarlar {
    yol: PathBuf,
    dosya: Mutex<Dosya>,
    /// Dosya okunamadıysa nedeni; varsayılanlar kullanılır.
    hata: Mutex<Option<String>>,
    /// Açık oturumlar her turun başında buradan okur (Görev 7).
    denetci_kipi: Arc<RwLock<DenetimKipi>>,
    /// Sınıflandırıcıların paylaştığı gizli ayarları (3e); değişiklik açık
    /// oturumlarda ve yalıtımda hemen geçerli.
    gizli: Arc<GizliAyarlari>,
    /// Kabuk aracının okuduğu kullanıcı ağ listesi (3e).
    ag_listesi: Arc<RwLock<Vec<HostKurali>>>,
}

fn kurallar(liste: &[AgHostu]) -> Vec<HostKurali> {
    liste
        .iter()
        .map(|h| HostKurali {
            host: h.host.clone(),
            port: h.port,
            ozel_ag: h.ozel_ag,
            kaynak: HostKaynagi::Kullanici,
        })
        .collect()
}

impl Ayarlar {
    /// Dosya yoksa varsayılanlar sessizce kullanılır. Bozuksa da varsayılanlar
    /// kullanılır, ama neden Ayarlar'da görünür.
    pub fn oku(veri_dizini: &Path) -> Self {
        let yol = veri_dizini.join(DOSYA_ADI);
        let (dosya, hata) = match std::fs::read(&yol) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Dosya::default(), None),
            Err(e) => (
                Dosya::default(),
                Some(format!(
                    "ayarlar.json okunamadı ({e}); varsayılanlar kullanılıyor."
                )),
            ),
            Ok(veri) => match serde_json::from_slice(&veri) {
                Ok(d) => (d, None),
                Err(e) => (
                    Dosya::default(),
                    Some(format!(
                        "ayarlar.json bozuk ({e}); varsayılanlar kullanılıyor."
                    )),
                ),
            },
        };
        let (mut dosya, mut hata) = (dosya, hata);
        // Elle bozulmuş kalıp bütün dosyayı düşürmez; yalnızca geçersiz
        // kalıplar ayıklanır ve neden Ayarlar'da görünür. Bütün listeyi
        // atmak kullanıcının geçerli ek kalıplarını da sessizce kapatırdı.
        let mut gecersiz = Vec::new();
        for (liste, istisna) in [
            (&mut dosya.ek_gizli_kaliplar, false),
            (&mut dosya.gizli_sayilmayanlar, true),
        ] {
            liste.retain(|k| match kalip_dogrula(k, istisna) {
                Ok(()) => true,
                Err(e) => {
                    gecersiz.push(e.to_string());
                    false
                }
            });
            if liste.len() > AZAMI_KALIP {
                gecersiz.push(KalipHatasi::CokKalip.to_string());
                liste.truncate(AZAMI_KALIP);
            }
        }
        if !gecersiz.is_empty() {
            hata = Some(format!(
                "ayarlar.json'daki bazı gizli kalıplar geçersiz; bunlar kullanılmıyor: {}",
                gecersiz.join(" ")
            ));
        }
        let gizli = match GizliAyarlari::yeni(&dosya.ek_gizli_kaliplar, &dosya.gizli_sayilmayanlar)
        {
            Ok(g) => g,
            Err(e) => {
                hata = Some(format!(
                    "ayarlar.json'daki gizli kalıplar geçersiz ({e}); bu kalıplar kullanılmıyor."
                ));
                dosya.ek_gizli_kaliplar.clear();
                dosya.gizli_sayilmayanlar.clear();
                GizliAyarlari::default()
            }
        };
        let gecerli = dosya
            .ag_izin_listesi
            .iter()
            .all(|h| ag_hostu_coz(&h.host).is_ok());
        if !gecerli {
            hata = Some(
                "ayarlar.json'daki ağ izin listesi geçersiz; liste kullanılmıyor.".to_string(),
            );
            dosya.ag_izin_listesi.clear();
        }
        Self {
            yol,
            denetci_kipi: Arc::new(RwLock::new(dosya.denetci_kipi)),
            gizli: Arc::new(gizli),
            ag_listesi: Arc::new(RwLock::new(kurallar(&dosya.ag_izin_listesi))),
            dosya: Mutex::new(dosya),
            hata: Mutex::new(hata),
        }
    }

    /// Sınıflandırıcıların paylaştığı gizli ayarları.
    pub fn gizli_ayarlari(&self) -> Arc<GizliAyarlari> {
        self.gizli.clone()
    }

    /// Kabuk aracının okuduğu kullanıcı ağ listesi.
    pub fn ag_listesi(&self) -> Arc<RwLock<Vec<HostKurali>>> {
        self.ag_listesi.clone()
    }

    fn hata_metni(&self) -> Option<String> {
        self.hata.lock().unwrap_or_else(|z| z.into_inner()).clone()
    }

    pub fn gizli_gorunumu(&self) -> GizliAyarGorunumu {
        let d = self.kilit();
        GizliAyarGorunumu {
            varsayilanlar: VARSAYILAN_GIZLI_KALIPLAR
                .iter()
                .map(|k| k.to_string())
                .collect(),
            ek_kaliplar: d.ek_gizli_kaliplar.clone(),
            gizli_sayilmayanlar: d.gizli_sayilmayanlar.clone(),
            ayar_hatasi: self.hata_metni(),
        }
    }

    pub fn ag_izin_listesi(&self) -> Vec<AgHostu> {
        self.kilit().ag_izin_listesi.clone()
    }

    pub fn etkin_pencere(&self) -> u64 {
        self.kilit().etkin_pencere
    }

    fn kilit(&self) -> std::sync::MutexGuard<'_, Dosya> {
        self.dosya.lock().unwrap_or_else(|z| z.into_inner())
    }

    /// Oturumların paylaştığı kip kaynağı.
    pub fn denetci_kipi_kaynagi(&self) -> Arc<RwLock<DenetimKipi>> {
        self.denetci_kipi.clone()
    }

    pub fn denetci_durumu(&self) -> DenetciDurumu {
        DenetciDurumu {
            kip: self.kilit().denetci_kipi,
            ayar_hatasi: self.hata_metni(),
        }
    }

    /// Dosyayı atomik yazar (`yalitim.json` gibi); yazılamazsa eski hâl kalır.
    fn degistir(&self, is: impl FnOnce(&mut Dosya)) -> Result<(), KomutHatasi> {
        let mut dosya = self.kilit();
        let mut yeni = dosya.clone();
        is(&mut yeni);
        // Gizli listeler yazılmadan önce doğrulanır: geçersizse dosya da
        // bellek de değişmez.
        GizliAyarlari::yeni(&yeni.ek_gizli_kaliplar, &yeni.gizli_sayilmayanlar)
            .map_err(|e| KomutHatasi::gecersiz(e.to_string()))?;
        let metin = serde_json::to_vec_pretty(&yeni).map_err(|e| KomutHatasi::ic(e.to_string()))?;
        if let Some(ust) = self.yol.parent() {
            std::fs::create_dir_all(ust)?;
        }
        let gecici = self.yol.with_extension("json.yeni");
        std::fs::write(&gecici, metin)?;
        std::fs::rename(&gecici, &self.yol)?;
        *self.denetci_kipi.write().unwrap_or_else(|z| z.into_inner()) = yeni.denetci_kipi;
        self.gizli
            .degistir(&yeni.ek_gizli_kaliplar, &yeni.gizli_sayilmayanlar)
            .map_err(|e| KomutHatasi::gecersiz(e.to_string()))?;
        *self.ag_listesi.write().unwrap_or_else(|z| z.into_inner()) =
            kurallar(&yeni.ag_izin_listesi);
        *dosya = yeni;
        *self.hata.lock().unwrap_or_else(|z| z.into_inner()) = None;
        Ok(())
    }
}

impl crate::mcp::ayarlar::McpAyarlarDeposu for Ayarlar {
    fn mcp_ayarlarini_oku(&self) -> (crate::mcp::ayarlar::McpAyarlar, u64) {
        let dosya = self.kilit();
        (dosya.mcp.clone(), dosya.mcp_revizyonu)
    }

    fn mcp_ayarlarini_kaydet(
        &self,
        yeni_mcp: &crate::mcp::ayarlar::McpAyarlar,
        beklenen_revizyon: u64,
    ) -> Result<u64, crate::mcp::ayarlar::McpDepoHatasi> {
        let mut dosya = self.kilit();
        if dosya.mcp_revizyonu != beklenen_revizyon {
            return Err(crate::mcp::ayarlar::McpDepoHatasi::EsZamanliDegisiklik);
        }
        let mut yeni = dosya.clone();
        yeni.mcp = yeni_mcp.clone();
        yeni.mcp_revizyonu = dosya.mcp_revizyonu.saturating_add(1);
        let metin = serde_json::to_vec_pretty(&yeni)
            .map_err(|e| crate::mcp::ayarlar::McpDepoHatasi::Io(e.to_string()))?;
        if let Some(ust) = self.yol.parent() {
            std::fs::create_dir_all(ust)
                .map_err(|e| crate::mcp::ayarlar::McpDepoHatasi::Io(e.to_string()))?;
        }
        let gecici = self.yol.with_extension("json.yeni");
        std::fs::write(&gecici, metin)
            .map_err(|e| crate::mcp::ayarlar::McpDepoHatasi::Io(e.to_string()))?;
        std::fs::rename(&gecici, &self.yol)
            .map_err(|e| crate::mcp::ayarlar::McpDepoHatasi::Io(e.to_string()))?;
        let yeni_revizyon = yeni.mcp_revizyonu;
        *dosya = yeni;
        *self.hata.lock().unwrap_or_else(|z| z.into_inner()) = None;
        Ok(yeni_revizyon)
    }
}

/// Kipler sıkılık sırasıyla: kapalı < uyar < katı.
fn sikilik(kip: DenetimKipi) -> u8 {
    match kip {
        DenetimKipi::Kapali => 0,
        DenetimKipi::Uyar => 1,
        DenetimKipi::Kati => 2,
    }
}

impl Uygulama {
    pub fn denetci_kipi(&self) -> DenetciDurumu {
        self.ayarlar.denetci_durumu()
    }

    /// Kipi yükseltmek onaysızdır. Düşürmek yerel pencerede onaylanır;
    /// vazgeçilirse kip değişmez (kural `yalitimsizi_ayarla`'dakiyle aynı).
    pub async fn denetci_kipini_ayarla(
        &self,
        kip: DenetimKipi,
    ) -> Result<DenetciDurumu, KomutHatasi> {
        let simdiki = self.ayarlar.denetci_durumu().kip;
        if sikilik(kip) < sikilik(simdiki)
            && !self
                .iletisim
                .onayla("Denetçi kipini düşür", DUSURME_ONAYI, "Düşür", "Vazgeç")
                .await
        {
            return Ok(self.ayarlar.denetci_durumu());
        }
        self.ayarlar.degistir(|d| d.denetci_kipi = kip)?;
        Ok(self.ayarlar.denetci_durumu())
    }

    /// Yeni ve açılan oturumların etkin bağlam penceresi.
    pub fn etkin_pencere(&self) -> u64 {
        self.ayarlar.etkin_pencere()
    }

    /// Etkin bağlam penceresini değiştirir; sınırlar burada doğrulanır ve
    /// dosyaya yazılır. Geçersiz değerde dosya ve bellek değişmez.
    pub fn etkin_pencereyi_ayarla(&self, deger: u64) -> Result<u64, KomutHatasi> {
        if !etkin_pencere_gecerli(deger) {
            return Err(KomutHatasi::gecersiz(ETKIN_PENCERE_GECERSIZ));
        }
        self.ayarlar.degistir(|d| d.etkin_pencere = deger)?;
        Ok(deger)
    }

    pub fn gizli_ayarlari(&self) -> GizliAyarGorunumu {
        self.ayarlar.gizli_gorunumu()
    }

    /// Ek gizli kalıp eklemek onaysızdır: yalnızca sıkılaştırır (§9.2).
    pub async fn ek_gizli_kalip_ekle(
        &self,
        kalip: String,
    ) -> Result<GizliAyarGorunumu, KomutHatasi> {
        let k = kalip_duzelt(&kalip, false)?;
        self.ayarlar.degistir(|d| {
            if !d.ek_gizli_kaliplar.contains(&k) {
                d.ek_gizli_kaliplar.push(k);
            }
        })?;
        self.mcp.politika_degisti();
        Ok(self.ayarlar.gizli_gorunumu())
    }

    /// Kaldırmak yerel pencerede onaylanır; vazgeçilirse değişmez.
    pub async fn ek_gizli_kalip_kaldir(
        &self,
        kalip: String,
    ) -> Result<GizliAyarGorunumu, KomutHatasi> {
        let k = kalip.trim().replace('\\', "/");
        if !self.ayarlar.kilit().ek_gizli_kaliplar.contains(&k) {
            return Ok(self.ayarlar.gizli_gorunumu());
        }
        let metin = format!("`{k}`\n\n{EK_KALIP_KALDIRMA}");
        if !self
            .iletisim
            .onayla("Ek gizli kalıbı kaldır", &metin, "Kaldır", "Vazgeç")
            .await
        {
            return Ok(self.ayarlar.gizli_gorunumu());
        }
        self.ayarlar
            .degistir(|d| d.ek_gizli_kaliplar.retain(|x| x != &k))?;
        self.mcp.politika_degisti();
        Ok(self.ayarlar.gizli_gorunumu())
    }

    /// İstisna eklemek yerel pencerede onaylanır (K7).
    pub async fn gizli_sayilmayan_ekle(
        &self,
        kalip: String,
    ) -> Result<GizliAyarGorunumu, KomutHatasi> {
        let k = kalip_duzelt(&kalip, true)?;
        if self.ayarlar.kilit().gizli_sayilmayanlar.contains(&k) {
            return Ok(self.ayarlar.gizli_gorunumu());
        }
        let metin = format!("`{k}`\n\n{ISTISNA_EKLEME}");
        if !self
            .iletisim
            .onayla("Gizli sayılmayanlara ekle", &metin, "Ekle", "Vazgeç")
            .await
        {
            return Ok(self.ayarlar.gizli_gorunumu());
        }
        self.ayarlar.degistir(|d| {
            if !d.gizli_sayilmayanlar.contains(&k) {
                d.gizli_sayilmayanlar.push(k);
            }
        })?;
        self.mcp.politika_degisti();
        Ok(self.ayarlar.gizli_gorunumu())
    }

    /// İstisnayı kaldırmak onaysızdır: yalnızca sıkılaştırır.
    pub async fn gizli_sayilmayan_kaldir(
        &self,
        kalip: String,
    ) -> Result<GizliAyarGorunumu, KomutHatasi> {
        let k = kalip.trim().replace('\\', "/");
        self.ayarlar
            .degistir(|d| d.gizli_sayilmayanlar.retain(|x| x != &k))?;
        self.mcp.politika_degisti();
        Ok(self.ayarlar.gizli_gorunumu())
    }

    pub fn ag_izin_listesi(&self) -> Vec<AgHostu> {
        self.ayarlar.ag_izin_listesi()
    }

    /// Host eklemek yerel pencerede onaylanır (§8.3). Liste değişince profil
    /// özeti değişir; bekleyen onaylar geçersizleşir.
    pub async fn ag_hostu_ekle(
        &self,
        host: String,
        ozel_ag: bool,
    ) -> Result<Vec<AgHostu>, KomutHatasi> {
        let (h, port) = ag_hostu_coz(&host).map_err(|e| KomutHatasi::gecersiz(e.to_string()))?;
        let yeni = AgHostu {
            host: h.clone(),
            port,
            ozel_ag,
        };
        if self.ayarlar.kilit().ag_izin_listesi.contains(&yeni) {
            return Ok(self.ayarlar.ag_izin_listesi());
        }
        let mut metin = format!(
            "`{h}:{port}` ağlı komutların izin listesine eklenecek: yalıtımlı komutlar bu \
             adrese vekil üzerinden bağlanabilecek."
        );
        if ozel_ag {
            metin.push_str("\n\n");
            metin.push_str(OZEL_AG);
        }
        if !self
            .iletisim
            .onayla("Ağ izin listesine ekle", &metin, "Ekle", "Vazgeç")
            .await
        {
            return Ok(self.ayarlar.ag_izin_listesi());
        }
        self.ayarlar.degistir(|d| {
            d.ag_izin_listesi
                .retain(|x| !(x.host == yeni.host && x.port == yeni.port));
            d.ag_izin_listesi.push(yeni);
        })?;
        Ok(self.ayarlar.ag_izin_listesi())
    }

    /// Host kaldırmak onaysızdır: yalnızca sıkılaştırır.
    pub async fn ag_hostu_kaldir(&self, host: String) -> Result<Vec<AgHostu>, KomutHatasi> {
        let (h, port) = ag_hostu_coz(&host).map_err(|e| KomutHatasi::gecersiz(e.to_string()))?;
        self.ayarlar.degistir(|d| {
            d.ag_izin_listesi
                .retain(|x| !(x.host == h && x.port == port));
        })?;
        Ok(self.ayarlar.ag_izin_listesi())
    }
}

/// Kalıbı doğrular ve biçimini düzeltir (ters bölü, boşluklar).
fn kalip_duzelt(kalip: &str, istisna: bool) -> Result<String, KomutHatasi> {
    kalip_dogrula(kalip, istisna).map_err(|e| KomutHatasi::gecersiz(e.to_string()))?;
    Ok(kalip.trim().replace('\\', "/"))
}
