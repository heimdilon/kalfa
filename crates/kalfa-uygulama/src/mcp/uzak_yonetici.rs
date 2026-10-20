//! Uzak Streamable HTTP MCP sunucularının bağlantı, kimlik bağlama, katalog ve
//! arayüz durumu yöneticisi (`1.1b` Görev 6).
//!
//! Güvenlik sınırları:
//! - Uzak MCP sunucuları yerel süreç (`surec.rs`) veya AppContainer yalıtımıyla
//!   karıştırılmaz; rozetleri [`UzakMcpRozeti::UzakStreamableHttp`] olarak ayrıdır.
//! - Sunucu ekleme, düzenleme, silme, OAuth tarayıcı açma ve bağlı kimliği
//!   kaldırma işlemleri yerel kullanıcı onayı ([`Iletisim::onayla`]) gerektirir.
//! - Sabit başlık sırları ve OAuth token paketleri yalnızca [`GizliKasa`] içinde
//!   tutulur; DTO'lara, hata metinlerine veya oturum kayıtlarına girmez.
//! - Yazma çağrısında istek gönderildikten sonra bağlantı koparsa otomatik tekrar
//!   yapılmaz; sonuç [`UzakCagriHatasi::SonucuBilinmiyor`] olarak döner ve sunucu
//!   durumunda belirsiz sonuç uyarısı saklanır.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use kalfa_core::arac::{McpAracTanimOzeti, McpBaslatmaSayaci, McpTarifi, async_trait};
use kalfa_core::izin::GizliAyarlari;
use kalfa_mcp::CallToolResult;
use kalfa_mcp::istemci::{McpHatasi, McpIstemcisi};
use kalfa_mcp::maske::{GizliDeger, maskele_metin};
use kalfa_mcp::tanim::{UzakBaglamButcesi, UzakMcpTanim, UzakYetkiTuru};
use kalfa_mcp::uzak::{
    SinirliHttpTasima, SistemDnsCozucu, UzakAracKatalogu, UzakCagriHatasi, UzakDnsCozucu,
    UzakHttpSecenekleri, UzakMcpCagriYurutucusu, UzakSunucuKatalogGirdisi,
};
use kalfa_sandbox::tarama::CancellationToken;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::arayuz::{Acici, Iletisim};
use crate::hata::KomutHatasi;
use crate::mcp::ayarlar::{
    McpAyarlarDeposu, McpAyarlarIslemi, McpDegisikligi, UzakMcpSunucu, UzakMcpSunucuAyrintiDto,
    UzakMcpSunucuOzetiDto, UzakMcpSunucuTaslagi, UzakMcpYetkiAyari, UzakMcpYetkiGirdisi,
    YeniGizliDeger,
};
use crate::mcp::kasa::{GizliKasa, GizliKimlik};
use crate::mcp::uzak_oauth::{
    OAuthBaslatmaSonucuDto, OAuthTamamlamaSonucuDto, UzakOAuthHatasi, UzakOAuthYoneticisi,
};

const UZAK_KAYIT_BASLIGI: &str = "Uzak MCP sunucusunu kaydet";
const UZAK_SILME_BASLIGI: &str = "Uzak MCP sunucusunu sil";
const UZAK_KIMLIK_KALDIR_BASLIGI: &str = "Uzak MCP kimliğini kaldır";

/// Uzak MCP sunucusunun erişim rozeti. Yerel `Yalitimli` (AppContainer/bwrap)
/// veya `TamErisim` rozetlerinden kesin olarak ayrıdır.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UzakMcpRozeti {
    #[default]
    UzakStreamableHttp,
}

/// Uzak MCP sunucusunun bağlantı durumu etiketi.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UzakMcpDurumEtiketi {
    #[default]
    BagliDegil,
    Baglaniyor,
    Bagli,
    KimlikGerekli,
    Hata,
    SonucuBelirsiz,
}

/// Arayüz formundan gelen uzak MCP sunucu taslağı.
///
/// Gizli değer yalnız bu yapıdan [`YeniGizliDeger`]'e geçer; `Debug` ve
/// `Serialize` bilerek uygulanmaz.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UzakMcpSunucuFormu {
    pub ad: String,
    #[serde(default)]
    pub etkin: bool,
    pub uc_url: String,
    #[serde(default)]
    pub yetki: UzakMcpYetkiFormGirdisi,
    pub baslatma_zamani_asimi_sn: u32,
    pub cagri_zamani_asimi_sn: u32,
}

/// Arayüz formundaki uzak MCP yetkilendirme girdisi.
#[derive(Default, Deserialize)]
#[serde(tag = "tur", rename_all = "snake_case", deny_unknown_fields)]
pub enum UzakMcpYetkiFormGirdisi {
    #[default]
    Yetkisiz,
    YeniSabitBaslik {
        baslik_adi: String,
        deger: String,
    },
    KorunanSabitBaslik {
        baslik_adi: String,
        kimlik: GizliKimlik,
    },
    OAuth {
        #[serde(default)]
        istemci_kimligi: Option<String>,
        #[serde(default)]
        kapsamlar: Vec<String>,
        #[serde(default)]
        kaynak_url: Option<String>,
        #[serde(default)]
        korunan_yetki_kimligi: Option<GizliKimlik>,
    },
}

impl TryFrom<UzakMcpSunucuFormu> for UzakMcpSunucuTaslagi {
    type Error = KomutHatasi;

    fn try_from(form: UzakMcpSunucuFormu) -> Result<Self, Self::Error> {
        let yetki = match form.yetki {
            UzakMcpYetkiFormGirdisi::Yetkisiz => UzakMcpYetkiGirdisi::Yetkisiz,
            UzakMcpYetkiFormGirdisi::YeniSabitBaslik { baslik_adi, deger } => {
                let gizli = YeniGizliDeger::yeni(deger)
                    .map_err(|e| KomutHatasi::gecersiz(e.to_string()))?;
                UzakMcpYetkiGirdisi::YeniSabitBaslik {
                    baslik_adi,
                    deger: gizli,
                }
            }
            UzakMcpYetkiFormGirdisi::KorunanSabitBaslik { baslik_adi, kimlik } => {
                UzakMcpYetkiGirdisi::KorunanSabitBaslik { baslik_adi, kimlik }
            }
            UzakMcpYetkiFormGirdisi::OAuth {
                istemci_kimligi,
                kapsamlar,
                kaynak_url,
                korunan_yetki_kimligi,
            } => UzakMcpYetkiGirdisi::OAuth {
                istemci_kimligi,
                kapsamlar,
                kaynak_url,
                korunan_yetki_kimligi,
            },
        };
        Ok(Self {
            ad: form.ad,
            etkin: form.etkin,
            uc_url: form.uc_url,
            yetki,
            baslatma_zamani_asimi_sn: form.baslatma_zamani_asimi_sn,
            cagri_zamani_asimi_sn: form.cagri_zamani_asimi_sn,
        })
    }
}

/// Arayüz listesinde gösterilen uzak MCP sunucu satırı DTO'su.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UzakMcpSunucuSatiriDto {
    pub ozet: UzakMcpSunucuOzetiDto,
    pub rozet: UzakMcpRozeti,
    pub durum: UzakMcpDurumEtiketi,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protokol_surumu: Option<String>,
    pub arac_sayisi: usize,
    pub reddedilen_arac_sayisi: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub son_hata: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub belirsiz_yazma_uyarisi: Option<String>,
}

/// Arayüz ayrıntı ve düzenleme ekranı için uzak MCP sunucu DTO'su.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UzakMcpSunucuAyrintisiDto {
    pub ayrinti: UzakMcpSunucuAyrintiDto,
    pub rozet: UzakMcpRozeti,
    pub durum: UzakMcpDurumEtiketi,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protokol_surumu: Option<String>,
    pub araclar: Vec<String>,
    pub reddedilenler: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub son_hata: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub belirsiz_yazma_uyarisi: Option<String>,
}

#[derive(Clone)]
struct CanliUzakSunucuDurumu {
    durum: UzakMcpDurumEtiketi,
    protokol_surumu: Option<String>,
    araclar: Vec<String>,
    reddedilenler: Vec<usize>,
    son_hata: Option<String>,
    belirsiz_yazma_uyarisi: Option<String>,
    istemci: Option<Arc<McpIstemcisi>>,
    sirlar: Vec<GizliDeger>,
}

impl Default for CanliUzakSunucuDurumu {
    fn default() -> Self {
        Self {
            durum: UzakMcpDurumEtiketi::BagliDegil,
            protokol_surumu: None,
            araclar: Vec::new(),
            reddedilenler: Vec::new(),
            son_hata: None,
            belirsiz_yazma_uyarisi: None,
            istemci: None,
            sirlar: Vec::new(),
        }
    }
}

/// Uzak Streamable HTTP MCP sunucularının yaşam döngüsü, OAuth kimlik bağlaması
/// ve katalog koordinatörü.
pub struct UzakMcpYoneticisi {
    mcp_depo: Arc<dyn McpAyarlarDeposu>,
    kasa: Arc<dyn GizliKasa>,
    iletisim: Arc<dyn Iletisim>,
    oauth: Arc<UzakOAuthYoneticisi>,
    katalog: Arc<UzakAracKatalogu>,
    dns_cozucu: Arc<dyn UzakDnsCozucu>,
    canli: Mutex<BTreeMap<String, CanliUzakSunucuDurumu>>,
    baglanti_sayaci: AtomicU64,
    arac_revizyonu_sayaci: AtomicU64,
    test_loopback_etkin: AtomicBool,
}

impl UzakMcpYoneticisi {
    pub fn yeni(
        mcp_depo: Arc<dyn McpAyarlarDeposu>,
        kasa: Arc<dyn GizliKasa>,
        iletisim: Arc<dyn Iletisim>,
        acici: Arc<dyn Acici>,
    ) -> Arc<Self> {
        let oauth = UzakOAuthYoneticisi::yeni(
            Arc::clone(&mcp_depo),
            Arc::clone(&kasa),
            Arc::clone(&iletisim),
            acici,
        );
        Self::yeni_ozel(
            mcp_depo,
            kasa,
            iletisim,
            oauth,
            Arc::new(UzakAracKatalogu::default()),
            Arc::new(SistemDnsCozucu),
        )
    }

    pub fn yeni_ozel(
        mcp_depo: Arc<dyn McpAyarlarDeposu>,
        kasa: Arc<dyn GizliKasa>,
        iletisim: Arc<dyn Iletisim>,
        oauth: Arc<UzakOAuthYoneticisi>,
        katalog: Arc<UzakAracKatalogu>,
        dns_cozucu: Arc<dyn UzakDnsCozucu>,
    ) -> Arc<Self> {
        Arc::new(Self {
            mcp_depo,
            kasa,
            iletisim,
            oauth,
            katalog,
            dns_cozucu,
            canli: Mutex::new(BTreeMap::new()),
            baglanti_sayaci: AtomicU64::new(0),
            arac_revizyonu_sayaci: AtomicU64::new(0),
            test_loopback_etkin: AtomicBool::new(false),
        })
    }

    pub fn test_loopback_izni_ayarla(&self, etkin: bool) {
        self.test_loopback_etkin.store(etkin, Ordering::SeqCst);
    }

    pub fn test_loopback_etkin_mi(&self) -> bool {
        self.test_loopback_etkin.load(Ordering::SeqCst)
    }

    pub fn katalog(&self) -> &Arc<UzakAracKatalogu> {
        &self.katalog
    }

    pub fn oturum_arac_kaynagi(self: &Arc<Self>) -> Arc<kalfa_mcp::uzak::UzakAracKaynagi> {
        kalfa_mcp::uzak::UzakAracKaynagi::yeni(
            Arc::clone(&self.katalog),
            Arc::clone(self) as Arc<dyn UzakMcpCagriYurutucusu>,
        )
    }

    pub fn oauth(&self) -> &Arc<UzakOAuthYoneticisi> {
        &self.oauth
    }

    fn satir_uret(&self, sunucu: &UzakMcpSunucu) -> UzakMcpSunucuSatiriDto {
        let canli = self.canli.lock().unwrap_or_else(|e| e.into_inner());
        let ozet = sunucu.ozet_dto();
        if let Some(d) = canli.get(&sunucu.ad) {
            UzakMcpSunucuSatiriDto {
                ozet,
                rozet: UzakMcpRozeti::UzakStreamableHttp,
                durum: d.durum,
                protokol_surumu: d.protokol_surumu.clone(),
                arac_sayisi: d.araclar.len(),
                reddedilen_arac_sayisi: d.reddedilenler.len(),
                son_hata: d.son_hata.clone(),
                belirsiz_yazma_uyarisi: d.belirsiz_yazma_uyarisi.clone(),
            }
        } else {
            let varsayilan_durum = match &sunucu.yetki {
                UzakMcpYetkiAyari::OAuth {
                    yetki_kimligi: None,
                    ..
                } => UzakMcpDurumEtiketi::KimlikGerekli,
                _ => UzakMcpDurumEtiketi::BagliDegil,
            };
            UzakMcpSunucuSatiriDto {
                ozet,
                rozet: UzakMcpRozeti::UzakStreamableHttp,
                durum: varsayilan_durum,
                protokol_surumu: None,
                arac_sayisi: 0,
                reddedilen_arac_sayisi: 0,
                son_hata: None,
                belirsiz_yazma_uyarisi: None,
            }
        }
    }

    /// Kayıtlı tüm uzak MCP sunucularının sır içermeyen durum satırlarını döner.
    pub fn durum_listesi(&self) -> Vec<UzakMcpSunucuSatiriDto> {
        let ayarlar = self.mcp_depo.mcp_ayarlarini_oku().0;
        ayarlar
            .uzak_sunucular
            .iter()
            .map(|s| self.satir_uret(s))
            .collect()
    }

    /// Tek bir uzak MCP sunucusunun sır içermeyen ayrıntısını döner.
    pub fn sunucu_ayrintisi(&self, ad: &str) -> Result<UzakMcpSunucuAyrintisiDto, KomutHatasi> {
        let ayarlar = self.mcp_depo.mcp_ayarlarini_oku().0;
        let sunucu = ayarlar
            .uzak_sunucu(ad)
            .ok_or_else(|| KomutHatasi::gecersiz("Bu adla bir uzak MCP sunucusu yok."))?;
        let satir = self.satir_uret(sunucu);
        let canli = self.canli.lock().unwrap_or_else(|e| e.into_inner());
        let (araclar, reddedilenler) = canli
            .get(ad)
            .map(|d| {
                (
                    d.araclar.clone(),
                    d.reddedilenler
                        .iter()
                        .map(|sira| format!("#{sira}"))
                        .collect(),
                )
            })
            .unwrap_or_default();
        Ok(UzakMcpSunucuAyrintisiDto {
            ayrinti: sunucu.ayrinti_dto(),
            rozet: UzakMcpRozeti::UzakStreamableHttp,
            durum: satir.durum,
            protokol_surumu: satir.protokol_surumu,
            araclar,
            reddedilenler,
            son_hata: satir.son_hata,
            belirsiz_yazma_uyarisi: satir.belirsiz_yazma_uyarisi,
        })
    }

    /// Uzak MCP sunucusunu yerel onaydan sonra kaydeder veya günceller.
    pub async fn sunucu_kaydet(
        &self,
        eski_ad: Option<String>,
        form: UzakMcpSunucuFormu,
        test_fiksturu_loopback: bool,
    ) -> Result<bool, KomutHatasi> {
        let taslak: UzakMcpSunucuTaslagi = form.try_into()?;
        let yetki_etiketi = match &taslak.yetki {
            UzakMcpYetkiGirdisi::Yetkisiz => "Kimliksiz (açık uç)".to_owned(),
            UzakMcpYetkiGirdisi::YeniSabitBaslik { baslik_adi, .. }
            | UzakMcpYetkiGirdisi::KorunanSabitBaslik { baslik_adi, .. } => {
                format!("Sabit gizli başlık ({baslik_adi})")
            }
            UzakMcpYetkiGirdisi::OAuth { kapsamlar, .. } => {
                if kapsamlar.is_empty() {
                    "OAuth 2.0 (PKCE)".to_owned()
                } else {
                    format!("OAuth 2.0 (PKCE, kapsamlar: {})", kapsamlar.join(", "))
                }
            }
        };
        let onay_metni = format!(
            "`{}` uzak MCP sunucusu kaydedilecek.\nUç adres: {}\nYetkilendirme: {}",
            taslak.ad, taslak.uc_url, yetki_etiketi
        );
        if !self
            .iletisim
            .onayla(UZAK_KAYIT_BASLIGI, &onay_metni, "Kaydet", "Vazgeç")
            .await
        {
            return Ok(false);
        }

        let yeni_ad = taslak.ad.clone();
        let mut islem = McpAyarlarIslemi::baslat(&*self.mcp_depo);
        islem.degisiklik_ekle(McpDegisikligi::UzakKaydet {
            eski_ad: eski_ad.clone(),
            taslak,
            test_fiksturu_loopback,
        });
        match islem.kaydet(&*self.mcp_depo, &*self.kasa) {
            Err(e) if !e.kayit_tamamlandi_mi() => return Err(KomutHatasi::gecersiz(e.to_string())),
            _ => {}
        }

        let mut canli = self.canli.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(eski) = &eski_ad {
            canli.remove(eski);
            self.katalog.sunucu_kaldir(eski);
        }
        canli.remove(&yeni_ad);
        self.katalog.sunucu_kaldir(&yeni_ad);
        Ok(true)
    }

    /// Uzak MCP sunucusunu yerel onaydan sonra siler ve kasadaki sırlarını emekliye ayırır.
    pub async fn sunucu_sil(&self, ad: &str) -> Result<bool, KomutHatasi> {
        if self
            .mcp_depo
            .mcp_ayarlarini_oku()
            .0
            .uzak_sunucu(ad)
            .is_none()
        {
            return Err(KomutHatasi::gecersiz("Bu adla bir uzak MCP sunucusu yok."));
        }
        let onay_metni =
            format!("`{ad}` uzak MCP sunucusu ve kayıtlı kimlik bilgileri tamamen silinecek.");
        if !self
            .iletisim
            .onayla(UZAK_SILME_BASLIGI, &onay_metni, "Sil", "Vazgeç")
            .await
        {
            return Ok(false);
        }

        let mut islem = McpAyarlarIslemi::baslat(&*self.mcp_depo);
        islem.degisiklik_ekle(McpDegisikligi::Sil { ad: ad.to_owned() });
        match islem.kaydet(&*self.mcp_depo, &*self.kasa) {
            Err(e) if !e.kayit_tamamlandi_mi() => return Err(KomutHatasi::gecersiz(e.to_string())),
            _ => {}
        }

        self.canli
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(ad);
        self.katalog.sunucu_kaldir(ad);
        Ok(true)
    }

    /// Uzak MCP sunucusunun etkinliğini değiştirir.
    pub fn etkin_ayarla(&self, ad: &str, etkin: bool) -> Result<(), KomutHatasi> {
        if self
            .mcp_depo
            .mcp_ayarlarini_oku()
            .0
            .uzak_sunucu(ad)
            .is_none()
        {
            return Err(KomutHatasi::gecersiz("Bu adla bir uzak MCP sunucusu yok."));
        }
        let mut islem = McpAyarlarIslemi::baslat(&*self.mcp_depo);
        islem.degisiklik_ekle(McpDegisikligi::EtkinlikAyarla {
            ad: ad.to_owned(),
            etkin,
        });
        match islem.kaydet(&*self.mcp_depo, &*self.kasa) {
            Err(e) if !e.kayit_tamamlandi_mi() => return Err(KomutHatasi::gecersiz(e.to_string())),
            _ => {}
        }
        if !etkin {
            self.canli
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(ad);
            self.katalog.sunucu_kaldir(ad);
        }
        Ok(())
    }

    /// Uzak MCP sunucusunun bağlı OAuth kimliğini yerel onaydan sonra kaldırır,
    /// kasadan siler ve `kimlik_revizyonu` sayacını artırarak eski onayları düşürür.
    pub async fn kimlik_kaldir(&self, ad: &str) -> Result<bool, KomutHatasi> {
        if self
            .mcp_depo
            .mcp_ayarlarini_oku()
            .0
            .uzak_sunucu(ad)
            .is_none()
        {
            return Err(KomutHatasi::gecersiz("Bu adla bir uzak MCP sunucusu yok."));
        }
        let onay_metni = format!(
            "`{ad}` uzak MCP sunucusunun bağlı hesap kimliği kaldırılacak ve kasadan silinecek."
        );
        if !self
            .iletisim
            .onayla(
                UZAK_KIMLIK_KALDIR_BASLIGI,
                &onay_metni,
                "Kimliği Kaldır",
                "Vazgeç",
            )
            .await
        {
            return Ok(false);
        }

        self.oauth
            .kimlik_kaldir(ad)
            .map_err(|e| KomutHatasi::gecersiz(e.to_string()))?;

        {
            let mut canli = self.canli.lock().unwrap_or_else(|e| e.into_inner());
            canli.insert(
                ad.to_owned(),
                CanliUzakSunucuDurumu {
                    durum: UzakMcpDurumEtiketi::KimlikGerekli,
                    ..CanliUzakSunucuDurumu::default()
                },
            );
        }
        self.katalog.sunucu_kaldir(ad);
        Ok(true)
    }

    /// OAuth 2.0 kullanıcı hesap bağlama akışını başlatır.
    #[allow(clippy::too_many_arguments)]
    pub async fn oauth_bagla_baslat(
        &self,
        ad: &str,
        oturum_kimligi: Option<String>,
        korunmus_kaynak_metadata_url: &str,
        yetki_sunucusu_metadata_url: &str,
        redirect_uri: &str,
        test_fiksturu_loopback: bool,
        ozel_ag_izinli: bool,
    ) -> Result<OAuthBaslatmaSonucuDto, KomutHatasi> {
        self.oauth
            .oauth_akisi_baslat(
                ad,
                oturum_kimligi,
                korunmus_kaynak_metadata_url,
                yetki_sunucusu_metadata_url,
                redirect_uri,
                test_fiksturu_loopback,
                ozel_ag_izinli,
            )
            .await
            .map_err(|e| KomutHatasi::gecersiz(e.to_string()))
    }

    /// Arayüzden veya IPC üzerinden başlatılan OAuth 2.0 kullanıcı bağlama akışı.
    ///
    /// Metadata uçları verilmemişse RFC 9728 / RFC 8414 `.well-known` adreslerinden
    /// türetilir; `redirect_uri` verilmemişse yerel `127.0.0.1:0` loopback geri çağrım
    /// dinleyicisi açılır ve tarayıcı yönlendirmesi geldiğinde token takası tamamlanır.
    #[allow(clippy::too_many_arguments)]
    pub async fn kullanici_oauth_bagla(
        self: &Arc<Self>,
        ad: &str,
        oturum_kimligi: Option<String>,
        korunmus_kaynak_metadata_url: Option<&str>,
        yetki_sunucusu_metadata_url: Option<&str>,
        redirect_uri: Option<&str>,
        test_fiksturu_loopback: bool,
        ozel_ag_izinli: bool,
    ) -> Result<OAuthBaslatmaSonucuDto, KomutHatasi> {
        let kaynak_meta = korunmus_kaynak_metadata_url.unwrap_or("");
        let yetki_meta = yetki_sunucusu_metadata_url.unwrap_or("");
        if let Some(acik_redirect) = redirect_uri.map(str::trim).filter(|s| !s.is_empty()) {
            return self
                .oauth_bagla_baslat(
                    ad,
                    oturum_kimligi,
                    kaynak_meta,
                    yetki_meta,
                    acik_redirect,
                    test_fiksturu_loopback,
                    ozel_ag_izinli,
                )
                .await;
        }

        let dinleyici = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|_| {
                KomutHatasi::gecersiz("Yerel OAuth geri çağrım dinleyicisi başlatılamadı.")
            })?;
        let yerel_adres = dinleyici
            .local_addr()
            .map_err(|_| KomutHatasi::gecersiz("Yerel OAuth geri çağrım adresi okunamadı."))?;
        let otomatik_redirect = format!("http://{yerel_adres}/oauth/callback");

        let baslatma = self
            .oauth_bagla_baslat(
                ad,
                oturum_kimligi.clone(),
                kaynak_meta,
                yetki_meta,
                &otomatik_redirect,
                test_fiksturu_loopback,
                ozel_ag_izinli,
            )
            .await?;

        let yonetici = Arc::clone(self);
        let sunucu_adi = ad.to_owned();
        tokio::spawn(async move {
            let _ = loopback_oauth_geri_cagrimini_bekle(
                dinleyici,
                yonetici,
                sunucu_adi,
                oturum_kimligi,
            )
            .await;
        });

        Ok(baslatma)
    }

    /// Arayüzden "Hesabı bağla (OAuth)" düğmesiyle başlatılan ve loopback geri çağrımında
    /// token takası tamamlanana kadar bekleyip güncel sunucu satırını döndüren akış.
    pub async fn kullanici_oauth_bagla_ve_tamamla(
        self: &Arc<Self>,
        ad: &str,
        oturum_kimligi: Option<String>,
        test_fiksturu_loopback: bool,
        ozel_ag_izinli: bool,
    ) -> Result<UzakMcpSunucuSatiriDto, KomutHatasi> {
        let dinleyici = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|_| {
                KomutHatasi::gecersiz("Yerel OAuth geri çağrım dinleyicisi başlatılamadı.")
            })?;
        let yerel_adres = dinleyici
            .local_addr()
            .map_err(|_| KomutHatasi::gecersiz("Yerel OAuth geri çağrım adresi okunamadı."))?;
        let otomatik_redirect = format!("http://{yerel_adres}/oauth/callback");

        let _baslatma = self
            .oauth_bagla_baslat(
                ad,
                oturum_kimligi.clone(),
                "",
                "",
                &otomatik_redirect,
                test_fiksturu_loopback,
                ozel_ag_izinli,
            )
            .await?;

        loopback_oauth_geri_cagrimini_bekle(
            dinleyici,
            Arc::clone(self),
            ad.to_owned(),
            oturum_kimligi,
        )
        .await?;

        let guncel = self
            .mcp_depo
            .mcp_ayarlarini_oku()
            .0
            .uzak_sunucu(ad)
            .cloned()
            .ok_or_else(|| KomutHatasi::gecersiz("Bu adla bir uzak MCP sunucusu yok."))?;
        Ok(self.satir_uret(&guncel))
    }

    /// OAuth 2.0 geri çağrımını (callback) tamamlar ve token paketini kasaya bağlar.
    pub async fn oauth_bagla_tamamla(
        &self,
        ad: &str,
        gelen_state: &str,
        gelen_oturum: Option<&str>,
        gelen_issuer: Option<&str>,
        gelen_kod: Option<&str>,
        gelen_hata: Option<&str>,
    ) -> Result<OAuthTamamlamaSonucuDto, KomutHatasi> {
        let sonuc = self
            .oauth
            .oauth_geri_cagrim_tamamla(
                ad,
                gelen_state,
                gelen_oturum,
                gelen_issuer,
                gelen_kod,
                gelen_hata,
            )
            .await
            .map_err(|e| KomutHatasi::gecersiz(e.to_string()))?;
        self.canli.lock().unwrap_or_else(|e| e.into_inner()).insert(
            ad.to_owned(),
            CanliUzakSunucuDurumu {
                durum: UzakMcpDurumEtiketi::BagliDegil,
                ..CanliUzakSunucuDurumu::default()
            },
        );
        self.katalog.sunucu_kaldir(ad);
        Ok(sonuc)
    }

    /// Uzak MCP sunucusuna bağlanır, `tools/list` keşfini ve G7 denetimini yapar
    /// ve denetlenmiş araçları [`UzakAracKatalogu`] içine yazar.
    pub async fn baglan(
        &self,
        ad: &str,
        alan_kimligi: &str,
        gizli_ayarlari: GizliAyarlari,
        test_fiksturu_loopback: bool,
        ozel_ag_izinli: bool,
    ) -> Result<UzakMcpSunucuSatiriDto, KomutHatasi> {
        let sunucu = self
            .mcp_depo
            .mcp_ayarlarini_oku()
            .0
            .uzak_sunucu(ad)
            .cloned()
            .ok_or_else(|| KomutHatasi::gecersiz("Bu adla bir uzak MCP sunucusu yok."))?;

        {
            let mut canli = self.canli.lock().unwrap_or_else(|e| e.into_inner());
            let girdi = canli.entry(ad.to_owned()).or_default();
            girdi.durum = UzakMcpDurumEtiketi::Baglaniyor;
            girdi.son_hata = None;
        }

        let (yetki_turu, gizli_baslik, kimlik_revizyonu) = match &sunucu.yetki {
            UzakMcpYetkiAyari::Yetkisiz => (UzakYetkiTuru::Yetkisiz, None, sunucu.kimlik_revizyonu),
            UzakMcpYetkiAyari::SabitBaslik {
                baslik_adi,
                gizli_kimlik,
            } => {
                let (baslik, rev) = self
                    .oauth
                    .gecerli_gizli_baslik_coz_veya_yenile(
                        ad,
                        sunucu.kimlik_revizyonu,
                        false,
                        None,
                        test_fiksturu_loopback,
                        ozel_ag_izinli,
                    )
                    .await
                    .map_err(|e| {
                        self.hata_kaydet(ad, UzakMcpDurumEtiketi::Hata, e.to_string(), &[]);
                        KomutHatasi::gecersiz(e.to_string())
                    })?;
                (
                    UzakYetkiTuru::SabitBaslikGizli {
                        baslik_adi: baslik_adi.clone(),
                        gizli_kimlik: gizli_kimlik.as_str().to_owned(),
                    },
                    baslik,
                    rev,
                )
            }
            UzakMcpYetkiAyari::OAuth {
                istemci_kimligi,
                kapsamlar,
                kaynak_url,
                yetki_kimligi,
            } => {
                let Some(kimlik) = yetki_kimligi else {
                    let mesaj =
                        "OAuth hesabı henüz bağlanmadı; önce hesabınızı bağlayın.".to_owned();
                    self.hata_kaydet(ad, UzakMcpDurumEtiketi::KimlikGerekli, mesaj.clone(), &[]);
                    return Err(KomutHatasi::gecersiz(mesaj));
                };
                let (baslik, rev) = self
                    .oauth
                    .gecerli_gizli_baslik_coz_veya_yenile(
                        ad,
                        sunucu.kimlik_revizyonu,
                        false,
                        None,
                        test_fiksturu_loopback,
                        ozel_ag_izinli,
                    )
                    .await
                    .map_err(|e| {
                        let etiket = match e {
                            UzakOAuthHatasi::KimlikBagliDegil => UzakMcpDurumEtiketi::KimlikGerekli,
                            _ => UzakMcpDurumEtiketi::Hata,
                        };
                        self.hata_kaydet(ad, etiket, e.to_string(), &[]);
                        KomutHatasi::gecersiz(e.to_string())
                    })?;
                (
                    UzakYetkiTuru::OAuth {
                        istemci_kimligi: istemci_kimligi.clone(),
                        kapsamlar: kapsamlar.clone(),
                        kaynak_url: kaynak_url.clone(),
                        yetki_kimligi: Some(kimlik.as_str().to_owned()),
                    },
                    baslik,
                    rev,
                )
            }
        };

        let sirlar: Vec<GizliDeger> = gizli_baslik
            .as_ref()
            .map(|b| b.maskeler().to_vec())
            .unwrap_or_default();

        let tanim = UzakMcpTanim::yeni(
            sunucu.ad.clone(),
            sunucu.uc_url.clone(),
            yetki_turu,
            sunucu.baslatma_zamani_asimi_sn,
            sunucu.cagri_zamani_asimi_sn,
            test_fiksturu_loopback,
            UzakBaglamButcesi::default(),
        )
        .map_err(|e| {
            self.hata_kaydet(ad, UzakMcpDurumEtiketi::Hata, e.to_string(), &sirlar);
            KomutHatasi::gecersiz(e.to_string())
        })?;

        let mut secenekler = UzakHttpSecenekleri::tanimdan(&tanim)
            .gizli_baslik_ile(gizli_baslik)
            .dns_cozucu_ile(Arc::clone(&self.dns_cozucu));
        secenekler.ozel_ag_izinli = ozel_ag_izinli;

        let (tasima, hata_alici) = SinirliHttpTasima::yeni(secenekler);
        let istemci = Arc::new(
            McpIstemcisi::yeni_uzak(tasima, hata_alici)
                .await
                .map_err(|e| {
                    let maskeli = maskele_metin(&e.to_string(), &sirlar);
                    self.hata_kaydet(ad, UzakMcpDurumEtiketi::Hata, maskeli.clone(), &sirlar);
                    KomutHatasi::gecersiz(maskeli)
                })?,
        );

        let ham_araclar = istemci.araclari_listele().await.map_err(|e| {
            let maskeli = maskele_metin(&e.to_string(), &sirlar);
            self.hata_kaydet(ad, UzakMcpDurumEtiketi::Hata, maskeli.clone(), &sirlar);
            KomutHatasi::gecersiz(maskeli)
        })?;

        let protokol_surumu = istemci
            .uzak_oturum_durumu()
            .and_then(|o| o.anlasilan_protokol())
            .unwrap_or_else(|| "2025-06-18".to_owned());

        let baglanti_no = self.baglanti_sayaci.fetch_add(1, Ordering::SeqCst) + 1;
        let arac_revizyonu = self.arac_revizyonu_sayaci.fetch_add(1, Ordering::SeqCst) + 1;

        let reddedilenler = self
            .katalog
            .sunucu_katalogunu_guncelle(UzakSunucuKatalogGirdisi {
                tanim,
                alan_kimligi: alan_kimligi.to_owned(),
                gizli_ayarlari,
                tanim_revizyonu: sunucu.tanim_revizyonu,
                kimlik_revizyonu,
                arac_revizyonu,
                protokol_surumu: protokol_surumu.clone(),
                baglanti_sayaci: McpBaslatmaSayaci::yeni(baglanti_no),
                ham_araclar,
                sirlar: sirlar.clone(),
            });

        let sunucu_adaylari = self.katalog.ara("", Some(ad));
        let arac_adlari: Vec<String> = sunucu_adaylari.into_iter().map(|a| a.birlesik_ad).collect();

        {
            let mut canli = self.canli.lock().unwrap_or_else(|e| e.into_inner());
            canli.insert(
                ad.to_owned(),
                CanliUzakSunucuDurumu {
                    durum: UzakMcpDurumEtiketi::Bagli,
                    protokol_surumu: Some(protokol_surumu),
                    araclar: arac_adlari,
                    reddedilenler,
                    son_hata: None,
                    belirsiz_yazma_uyarisi: None,
                    istemci: Some(istemci),
                    sirlar,
                },
            );
        }

        let guncel = self
            .mcp_depo
            .mcp_ayarlarini_oku()
            .0
            .uzak_sunucu(ad)
            .cloned()
            .ok_or_else(|| KomutHatasi::gecersiz("Bu adla bir uzak MCP sunucusu yok."))?;
        Ok(self.satir_uret(&guncel))
    }

    fn hata_kaydet(
        &self,
        ad: &str,
        etiket: UzakMcpDurumEtiketi,
        mesaj: String,
        sirlar: &[GizliDeger],
    ) {
        let maskeli = maskele_metin(&mesaj, sirlar);
        let mut canli = self.canli.lock().unwrap_or_else(|e| e.into_inner());
        let girdi = canli.entry(ad.to_owned()).or_default();
        girdi.durum = etiket;
        girdi.son_hata = Some(maskeli);
        girdi.istemci = None;
    }
}

async fn loopback_oauth_geri_cagrimini_bekle(
    dinleyici: tokio::net::TcpListener,
    yonetici: Arc<UzakMcpYoneticisi>,
    sunucu_adi: String,
    oturum_kimligi: Option<String>,
) -> Result<(), KomutHatasi> {
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let Ok(Ok((mut soket, _))) =
        tokio::time::timeout(Duration::from_secs(300), dinleyici.accept()).await
    else {
        return Err(KomutHatasi::gecersiz(
            "OAuth geri çağrımı zaman aşımına uğradı.",
        ));
    };

    let mut tampon = [0u8; 4096];
    let Ok(Ok(n)) = tokio::time::timeout(Duration::from_secs(5), soket.read(&mut tampon)).await
    else {
        return Err(KomutHatasi::gecersiz("OAuth geri çağrım isteği okunamadı."));
    };
    if n == 0 {
        return Err(KomutHatasi::gecersiz("OAuth geri çağrım isteği boş geldi."));
    }

    let istek_metni = String::from_utf8_lossy(&tampon[..n]);
    let ilk_satir = istek_metni.lines().next().unwrap_or_default();
    let yol_ve_sorgu = ilk_satir.split_whitespace().nth(1).unwrap_or("/");
    let tam_url = format!("http://127.0.0.1{yol_ve_sorgu}");
    let Ok(parsed) = url::Url::parse(&tam_url) else {
        return Err(KomutHatasi::gecersiz(
            "OAuth geri çağrım adresi ayrıştırılamadı.",
        ));
    };

    let mut state: Option<String> = None;
    let mut code: Option<String> = None;
    let mut iss: Option<String> = None;
    let mut error: Option<String> = None;
    for (anahtar, deger) in parsed.query_pairs() {
        match anahtar.as_ref() {
            "state" => state = Some(deger.into_owned()),
            "code" => code = Some(deger.into_owned()),
            "iss" => iss = Some(deger.into_owned()),
            "error" => error = Some(deger.into_owned()),
            _ => {}
        }
    }

    let sonuc = yonetici
        .oauth_bagla_tamamla(
            &sunucu_adi,
            state.as_deref().unwrap_or(""),
            oturum_kimligi.as_deref(),
            iss.as_deref(),
            code.as_deref(),
            error.as_deref(),
        )
        .await;

    let (durum_satiri, govde) = match &sonuc {
        Ok(_) => (
            "200 OK",
            "Kalfa OAuth yetkilendirmesi tamamlandı; bu sekmeyi kapatabilirsiniz.",
        ),
        Err(hata) => {
            yonetici.hata_kaydet(
                &sunucu_adi,
                UzakMcpDurumEtiketi::KimlikGerekli,
                hata.mesaj.clone(),
                &[],
            );
            (
                "400 Bad Request",
                "Kalfa OAuth yetkilendirmesi tamamlanamadı; uygulama ekranını kontrol edin.",
            )
        }
    };
    let yanit = format!(
        "HTTP/1.1 {durum_satiri}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{govde}",
        govde.len()
    );
    let _ = soket.write_all(yanit.as_bytes()).await;
    let _ = soket.flush().await;
    sonuc.map(|_| ())
}

#[async_trait]
impl UzakMcpCagriYurutucusu for UzakMcpYoneticisi {
    async fn uzak_arac_cagir(
        &self,
        sunucu_adi: &str,
        uzak_arac_adi: &str,
        argumanlar: Map<String, Value>,
        onayli_tarif: McpTarifi,
        _onayli_d: McpAracTanimOzeti,
        iptal: CancellationToken,
    ) -> Result<(CallToolResult, bool), UzakCagriHatasi> {
        let Some(guncel_tarif) = self.katalog.sunucu_tarifi(sunucu_adi) else {
            return Err(UzakCagriHatasi::OnayEskidi);
        };
        if guncel_tarif != onayli_tarif {
            return Err(UzakCagriHatasi::OnayEskidi);
        }

        let (istemci, sirlar) = {
            let canli = self.canli.lock().unwrap_or_else(|e| e.into_inner());
            let Some(durum) = canli.get(sunucu_adi) else {
                return Err(UzakCagriHatasi::OnayEskidi);
            };
            let Some(istemci) = durum.istemci.clone() else {
                return Err(UzakCagriHatasi::OnayEskidi);
            };
            (istemci, durum.sirlar.clone())
        };

        let cagri_fut = istemci.arac_cagir_cevabi(uzak_arac_adi.to_owned(), argumanlar);
        let cagri_sonucu = tokio::select! {
            biased;
            _ = iptal.cancelled() => {
                let uyari = format!(
                    "İptal gönderildi; `{sunucu_adi}` uzak sunucusundaki `{uzak_arac_adi}` işleminin durup durmadığı bilinmiyor. Otomatik tekrar yapılmadı."
                );
                let maskeli = maskele_metin(&uyari, &sirlar);
                let mut canli = self.canli.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(girdi) = canli.get_mut(sunucu_adi) {
                    girdi.durum = UzakMcpDurumEtiketi::SonucuBelirsiz;
                    girdi.belirsiz_yazma_uyarisi = Some(maskeli.clone());
                }
                return Err(UzakCagriHatasi::SonucBelirsiz(maskeli));
            }
            sonuc = cagri_fut => sonuc,
        };

        match cagri_sonucu {
            Ok(cevap) => Ok((cevap.sonuc, cevap.content_alani_var)),
            Err(ref hata) if hata.belirsiz_cagri_sonucu_mu() => {
                let uyari = format!(
                    "Bağlantı kesildi; `{sunucu_adi}` uzak sunucusundaki `{uzak_arac_adi}` işleminin tamamlanıp tamamlanmadığı bilinmiyor. Otomatik tekrar yapılmadı."
                );
                let maskeli = maskele_metin(&uyari, &sirlar);
                {
                    let mut canli = self.canli.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(girdi) = canli.get_mut(sunucu_adi) {
                        girdi.durum = UzakMcpDurumEtiketi::SonucuBelirsiz;
                        girdi.belirsiz_yazma_uyarisi = Some(maskeli.clone());
                    }
                }
                Err(UzakCagriHatasi::SonucBelirsiz(maskeli))
            }
            Err(McpHatasi::UzakRpc(rpc)) => Err(UzakCagriHatasi::Rpc(rpc)),
            Err(diger) => {
                let maskeli = maskele_metin(&diger.to_string(), &sirlar);
                Err(UzakCagriHatasi::Iletisim(maskeli))
            }
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicUsize;

    use kalfa_core::arac::{AracHatasi, AracKayitDefteri};
    use kalfa_core::izin::Kip;
    use kalfa_mcp::uzak::{ARAC_ARA_ADI, AracAraAraci, UzakAracKaynagi};
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use crate::mcp::ayarlar::{McpAyarlar, McpDepoHatasi};
    use crate::mcp::kasa::test_destek::SahteGizliKasa;

    const CANARY_TOKEN: &str = "CANARY-GOREV6-SECRET-99887766";

    #[derive(Default)]
    struct SahteMcpDepo {
        ic: Mutex<(McpAyarlar, u64)>,
    }

    impl McpAyarlarDeposu for SahteMcpDepo {
        fn mcp_ayarlarini_oku(&self) -> (McpAyarlar, u64) {
            self.ic.lock().unwrap().clone()
        }

        fn mcp_ayarlarini_kaydet(
            &self,
            yeni: &McpAyarlar,
            beklenen_revizyon: u64,
        ) -> Result<u64, McpDepoHatasi> {
            let mut g = self.ic.lock().unwrap();
            if g.1 != beklenen_revizyon {
                return Err(McpDepoHatasi::EsZamanliDegisiklik);
            }
            g.0 = yeni.clone();
            g.1 += 1;
            Ok(g.1)
        }
    }

    struct SahteIletisim {
        onay_yaniti: Mutex<bool>,
        basliklar: Mutex<Vec<String>>,
    }

    impl SahteIletisim {
        fn yeni(onay: bool) -> Arc<Self> {
            Arc::new(Self {
                onay_yaniti: Mutex::new(onay),
                basliklar: Mutex::new(Vec::new()),
            })
        }
    }

    #[async_trait]
    impl Iletisim for SahteIletisim {
        async fn klasor_sec(
            &self,
            _baslik: &str,
            _baslangic: Option<&std::path::Path>,
        ) -> Option<PathBuf> {
            None
        }

        async fn dosya_sec(
            &self,
            _baslik: &str,
            _baslangic: Option<&std::path::Path>,
        ) -> Option<PathBuf> {
            None
        }

        async fn onayla(&self, baslik: &str, _metin: &str, _evet: &str, _hayir: &str) -> bool {
            self.basliklar.lock().unwrap().push(baslik.to_owned());
            *self.onay_yaniti.lock().unwrap()
        }
    }

    #[derive(Default)]
    struct SahteAcici {
        acilan: Mutex<Vec<String>>,
    }

    impl Acici for SahteAcici {
        fn ac(&self, adres: &url::Url) -> Result<(), String> {
            self.acilan.lock().unwrap().push(adres.to_string());
            Ok(())
        }
    }

    async fn sahte_mcp_http_sunucusu_baslat(
        kop_cagri_sonrasi: bool,
    ) -> (u16, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
        let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = dinleyici.local_addr().unwrap().port();
        let cagri_sayisi = Arc::new(AtomicUsize::new(0));
        let sayac_kopya = Arc::clone(&cagri_sayisi);

        let gorev = tokio::spawn(async move {
            loop {
                let Ok((mut soket, _)) = dinleyici.accept().await else {
                    break;
                };
                let sayac = Arc::clone(&sayac_kopya);
                tokio::spawn(async move {
                    let mut tampon = Vec::new();
                    let mut gecici = [0u8; 4096];
                    let baslik_sonu = loop {
                        let Ok(n) = soket.read(&mut gecici).await else {
                            return;
                        };
                        if n == 0 {
                            return;
                        }
                        tampon.extend_from_slice(&gecici[..n]);
                        if let Some(konum) = tampon.windows(4).position(|w| w == b"\r\n\r\n") {
                            break konum + 4;
                        }
                    };
                    let baslik_metni = String::from_utf8_lossy(&tampon[..baslik_sonu]);
                    let icerik_uzunlugu: usize = baslik_metni
                        .lines()
                        .skip(1)
                        .find_map(|satir| {
                            let (ad, deger) = satir.split_once(':')?;
                            if ad.trim().eq_ignore_ascii_case("content-length") {
                                deger.trim().parse().ok()
                            } else {
                                None
                            }
                        })
                        .unwrap_or(0);
                    let mut govde = tampon[baslik_sonu..].to_vec();
                    while govde.len() < icerik_uzunlugu {
                        let Ok(n) = soket.read(&mut gecici).await else {
                            break;
                        };
                        if n == 0 {
                            break;
                        }
                        govde.extend_from_slice(&gecici[..n]);
                    }
                    govde.truncate(icerik_uzunlugu);
                    let Ok(json_istek) = serde_json::from_slice::<Value>(&govde) else {
                        return;
                    };
                    let metod = json_istek
                        .get("method")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let id = json_istek.get("id").cloned().unwrap_or(json!(1));

                    let yanit_json = match metod {
                        "initialize" => json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "protocolVersion": "2025-06-18",
                                "capabilities": { "tools": {} },
                                "serverInfo": { "name": "sahte-uzak", "version": "1.0.0" }
                            }
                        }),
                        "notifications/initialized" => {
                            let http = "HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                            let _ = soket.write_all(http.as_bytes()).await;
                            return;
                        }
                        "tools/list" => json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "tools": [
                                    {
                                        "name": "kayit_olustur",
                                        "description": "Uzak sistemde yeni kayıt oluşturur",
                                        "inputSchema": {
                                            "type": "object",
                                            "properties": { "baslik": { "type": "string" } },
                                            "required": ["baslik"]
                                        }
                                    },
                                    {
                                        "name": "sizinti_araci",
                                        "description": format!("Sır sızdıran araç: {CANARY_TOKEN}"),
                                        "inputSchema": { "type": "object" }
                                    }
                                ]
                            }
                        }),
                        "tools/call" => {
                            sayac.fetch_add(1, Ordering::SeqCst);
                            if kop_cagri_sonrasi {
                                // İstek alındıktan sonra yanıt dönmeden bağlantıyı kapat -> belirsiz yazma!
                                drop(soket);
                                return;
                            }
                            json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "content": [{
                                        "type": "text",
                                        "text": format!("Kayıt oluşturuldu (token={CANARY_TOKEN})")
                                    }]
                                }
                            })
                        }
                        _ => json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {
                                "code": -32601,
                                "message": "Method not found"
                            }
                        }),
                    };

                    let govde_str = serde_json::to_string(&yanit_json).unwrap();
                    let http = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nMcp-Session-Id: oturum-g6\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        govde_str.len(),
                        govde_str
                    );
                    let _ = soket.write_all(http.as_bytes()).await;
                });
            }
        });

        (port, cagri_sayisi, gorev)
    }

    #[tokio::test]
    async fn uzak_sunucu_ekle_baglan_kesfet_cagir_ve_kimlik_kaldir_akisi() {
        let (port, cagri_sayisi, _gorev) = sahte_mcp_http_sunucusu_baslat(false).await;
        let depo = Arc::new(SahteMcpDepo::default());
        let kasa = Arc::new(SahteGizliKasa::default());
        let iletisim = SahteIletisim::yeni(true);
        let acici = Arc::new(SahteAcici::default());

        let yonetici = UzakMcpYoneticisi::yeni(
            Arc::clone(&depo) as Arc<dyn McpAyarlarDeposu>,
            Arc::clone(&kasa) as Arc<dyn GizliKasa>,
            Arc::clone(&iletisim) as Arc<dyn Iletisim>,
            acici,
        );

        // 1. Uzak sunucuyu sabit gizli başlıkla kaydet; rozet UzakStreamableHttp olmalı, sır DTO'da görünmemeli.
        let kaydedildi = yonetici
            .sunucu_kaydet(
                None,
                UzakMcpSunucuFormu {
                    ad: "uzak-servis".to_owned(),
                    etkin: true,
                    uc_url: format!("http://127.0.0.1:{port}/mcp"),
                    yetki: UzakMcpYetkiFormGirdisi::YeniSabitBaslik {
                        baslik_adi: "Authorization".to_owned(),
                        deger: format!("Bearer {CANARY_TOKEN}"),
                    },
                    baslatma_zamani_asimi_sn: 10,
                    cagri_zamani_asimi_sn: 10,
                },
                true,
            )
            .await
            .unwrap();
        assert!(kaydedildi);

        let durumlar = yonetici.durum_listesi();
        assert_eq!(durumlar.len(), 1);
        assert_eq!(durumlar[0].rozet, UzakMcpRozeti::UzakStreamableHttp);
        assert_eq!(durumlar[0].durum, UzakMcpDurumEtiketi::BagliDegil);
        let dto_json = serde_json::to_string(&durumlar).unwrap();
        assert!(!dto_json.contains(CANARY_TOKEN));

        // 2. Sunucuya bağlan; sır sızdıran 2. araç G7'de reddedilir, 1. araç kataloğa girer.
        let bagli_satir = yonetici
            .baglan(
                "uzak-servis",
                "alan-g6",
                GizliAyarlari::default(),
                true,
                false,
            )
            .await
            .unwrap();
        assert_eq!(bagli_satir.durum, UzakMcpDurumEtiketi::Bagli);
        assert_eq!(bagli_satir.arac_sayisi, 1);
        assert_eq!(bagli_satir.reddedilen_arac_sayisi, 1);

        let ayrinti = yonetici.sunucu_ayrintisi("uzak-servis").unwrap();
        assert_eq!(
            ayrinti.araclar,
            vec!["mcp__uzak-servis__kayit_olustur".to_owned()]
        );
        assert_eq!(ayrinti.reddedilenler, vec!["#2".to_owned()]);
        assert!(
            !serde_json::to_string(&ayrinti)
                .unwrap()
                .contains(CANARY_TOKEN)
        );

        // 3. `arac_ara` ile aracı oturuma yükle ve sonraki model turunda çalıştır.
        let kaynak = UzakAracKaynagi::yeni(
            Arc::clone(yonetici.katalog()),
            Arc::clone(&yonetici) as Arc<dyn UzakMcpCagriYurutucusu>,
        );
        let arac_ara = AracAraAraci::yeni(Arc::clone(yonetici.katalog()), Arc::clone(&kaynak));
        let defter = AracKayitDefteri::yeni_kaynakli(vec![arac_ara], kaynak.clone()).unwrap();

        kaynak
            .araclari_yukle(&["mcp__uzak-servis__kayit_olustur".to_owned()])
            .unwrap();
        let goruntu = defter.anlik_goruntu(Kip::Proje);
        let arac = goruntu.bul("mcp__uzak-servis__kayit_olustur").unwrap();
        let arg = json!({"baslik": "Deneme"});
        let hazirlik = arac.hazirla(&arg).await.unwrap();
        let cikti = arac
            .calistir(&arg, &hazirlik, CancellationToken::new())
            .await
            .unwrap();
        assert!(cikti.icerik.contains("Kayıt oluşturuldu"));
        assert!(!cikti.icerik.contains(CANARY_TOKEN));
        assert!(cikti.icerik.contains("[gizli]"));
        assert_eq!(cagri_sayisi.load(Ordering::SeqCst), 1);

        // 4. Sunucu silindiğinde eski görüntüdeki onaylı hazırlık OnayEskidi ile reddedilir.
        assert!(yonetici.sunucu_sil("uzak-servis").await.unwrap());
        assert_eq!(
            arac.calistir(&arg, &hazirlik, CancellationToken::new())
                .await,
            Err(AracHatasi::OnayEskidi)
        );
        assert_eq!(defter.anlik_goruntu(Kip::Proje).tanimlar().len(), 1);
        assert_eq!(
            defter.anlik_goruntu(Kip::Proje).tanimlar()[0].function.name,
            ARAC_ARA_ADI
        );
    }

    #[tokio::test]
    async fn uzak_yazma_cagrisinda_baglanti_koparsa_sonuc_belirsiz_doner_ve_tekrar_edilmez() {
        let (port, cagri_sayisi, _gorev) = sahte_mcp_http_sunucusu_baslat(true).await;
        let depo = Arc::new(SahteMcpDepo::default());
        let kasa = Arc::new(SahteGizliKasa::default());
        let iletisim = SahteIletisim::yeni(true);
        let acici = Arc::new(SahteAcici::default());

        let yonetici = UzakMcpYoneticisi::yeni(
            Arc::clone(&depo) as Arc<dyn McpAyarlarDeposu>,
            Arc::clone(&kasa) as Arc<dyn GizliKasa>,
            Arc::clone(&iletisim) as Arc<dyn Iletisim>,
            acici,
        );

        yonetici
            .sunucu_kaydet(
                None,
                UzakMcpSunucuFormu {
                    ad: "kopan-servis".to_owned(),
                    etkin: true,
                    uc_url: format!("http://127.0.0.1:{port}/mcp"),
                    yetki: UzakMcpYetkiFormGirdisi::Yetkisiz,
                    baslatma_zamani_asimi_sn: 10,
                    cagri_zamani_asimi_sn: 10,
                },
                true,
            )
            .await
            .unwrap();

        yonetici
            .baglan(
                "kopan-servis",
                "alan-kopan",
                GizliAyarlari::default(),
                true,
                false,
            )
            .await
            .unwrap();

        let kaynak = UzakAracKaynagi::yeni(
            Arc::clone(yonetici.katalog()),
            Arc::clone(&yonetici) as Arc<dyn UzakMcpCagriYurutucusu>,
        );
        kaynak
            .araclari_yukle(&["mcp__kopan-servis__kayit_olustur".to_owned()])
            .unwrap();
        let defter = AracKayitDefteri::yeni_kaynakli(vec![], kaynak).unwrap();
        let goruntu = defter.anlik_goruntu(Kip::Proje);
        let arac = goruntu.bul("mcp__kopan-servis__kayit_olustur").unwrap();
        let arg = json!({"baslik": "Kritik Yazma"});
        let hazirlik = arac.hazirla(&arg).await.unwrap();

        let sonuc = arac
            .calistir(&arg, &hazirlik, CancellationToken::new())
            .await;
        assert!(sonuc.is_err());

        // Sunucuya yalnızca 1 kez çağrı gitmiş olmalı (kör otomatik tekrar yok!).
        assert_eq!(cagri_sayisi.load(Ordering::SeqCst), 1);

        // Arayüz durumunda SonucuBelirsiz etiketi ve belirsiz yazma uyarısı bulunmalı.
        let durumlar = yonetici.durum_listesi();
        assert_eq!(durumlar[0].durum, UzakMcpDurumEtiketi::SonucuBelirsiz);
        assert!(
            durumlar[0]
                .belirsiz_yazma_uyarisi
                .as_deref()
                .unwrap_or("")
                .contains("Otomatik tekrar yapılmadı")
        );
    }
}
