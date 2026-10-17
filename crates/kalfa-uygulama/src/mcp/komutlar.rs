//! MCP'nin arayüz uçları ve yerel onay sınırı (1.1a-2 Görev 10, tasarım §9).
//!
//! - Arayüz yol göndermez: komut yerel seçiciyle seçilir (opak kimlik),
//!   çalışma alanı oturum ya da alan kimliğiyle gelir; kök arka uçta çözülür.
//! - Her kayıt (ekleme, düzenleme, içe aktarma) ve silme yerel pencerede
//!   onaylanır; tam erişimli başlatma da yöneticide yerel onay ister. Arayüzün
//!   "onaylandı" yanıtı yalnız yalıtımlı başlatma kartında yeterlidir (§5.1).
//! - Durum görüntüsü sır taşımaz; reddedilen araç yalnız sıra numarasıyla,
//!   stderr maskeli gelir. `serverInfo`/`instructions` hiç taşınmaz.
//! - Ana pencerenin kanalına yazılan her görüntünün `revizyon`'u artar; yeni
//!   kanal güncel görüntüyü kilit altında alır, eski bir yazım onu ezmez.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use kalfa_core::kimlik::OturumId;
use kalfa_mcp::tanim::YazilabilirKokler;
use kalfa_sandbox::{CalistirmaKipi, OzelDizinler};
use serde::{Deserialize, Serialize};

use crate::arayuz::McpYazici;
use crate::hata::KomutHatasi;
use crate::mcp::ayarlar::{
    McpAgKurali, McpAyarlarDeposu, McpAyarlarIslemi, McpDegisikligi, McpKipi, McpKomut,
    McpOrtamGirdisi, McpSunucuAyrintiDto, McpSunucuOzetiDto, McpSunucuTaslagi, YeniGizliDeger,
};
use crate::mcp::denetim::{DenetlenmisMcpAracOzeti, McpTanimDenetleyicisi};
use crate::mcp::kasa::GizliKimlik;
use crate::mcp::surec::McpSunucuDurumu;
use crate::mcp::tanimlar::{IceAktarmaOnizlemeDto, KayitHatasi, SeciliKomutBilgisi, TaslakKimligi};
use crate::mcp::yonetici::{McpBaslatmaKarti, McpYoneticisi};
use crate::uygulama::Uygulama;

const SILME_BASLIGI: &str = "MCP sunucusunu sil";

/// Arayüz formundan gelen sunucu taslağı. Gizli değer yalnız bu yapıdan
/// `YeniGizliDeger`'e geçer; `Debug`/`Serialize` bilerek yoktur.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpSunucuFormu {
    pub ad: String,
    #[serde(default)]
    pub etkin: bool,
    pub komut: McpKomut,
    #[serde(default)]
    pub ortam: Vec<McpOrtamFormGirdisi>,
    #[serde(default)]
    pub kip: McpKipi,
    #[serde(default)]
    pub ag_izinleri: Vec<McpAgKurali>,
    pub baslatma_zamani_asimi_sn: u32,
    pub cagri_zamani_asimi_sn: u32,
}

#[derive(Deserialize)]
#[serde(tag = "tur", rename_all = "snake_case", deny_unknown_fields)]
pub enum McpOrtamFormGirdisi {
    Duz { ad: String, deger: String },
    YeniGizli { ad: String, deger: String },
    KorunanGizli { ad: String, kimlik: GizliKimlik },
}

impl TryFrom<McpSunucuFormu> for McpSunucuTaslagi {
    type Error = KomutHatasi;

    fn try_from(form: McpSunucuFormu) -> Result<Self, Self::Error> {
        let ortam = form
            .ortam
            .into_iter()
            .map(|girdi| match girdi {
                McpOrtamFormGirdisi::Duz { ad, deger } => Ok(McpOrtamGirdisi::Duz { ad, deger }),
                McpOrtamFormGirdisi::YeniGizli { ad, deger } => YeniGizliDeger::yeni(deger)
                    .map(|deger| McpOrtamGirdisi::YeniGizli { ad, deger })
                    .map_err(|e| KomutHatasi::gecersiz(e.to_string())),
                McpOrtamFormGirdisi::KorunanGizli { ad, kimlik } => {
                    Ok(McpOrtamGirdisi::KorunanGizli { ad, kimlik })
                }
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            ad: form.ad,
            etkin: form.etkin,
            komut: form.komut,
            ortam,
            kip: form.kip,
            ag_izinleri: form.ag_izinleri,
            baslatma_zamani_asimi_sn: form.baslatma_zamani_asimi_sn,
            cagri_zamani_asimi_sn: form.cagri_zamani_asimi_sn,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpTaslakDto {
    pub taslak_kimligi: TaslakKimligi,
    pub kusak: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum McpDurumEtiketi {
    Durdu,
    Basliyor,
    Calisiyor,
    BostaKapandi,
    Hata,
}

/// Bir sunucunun bir çalışma alanındaki durumu. `kusak` süreç kuşağı N'dir;
/// arayüz eski kuşağın olayını yenisinin üstüne yazmaz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpAlanDurumuDto {
    pub alan_kimligi: String,
    pub durum: McpDurumEtiketi,
    pub kusak: Option<u64>,
    pub hata: Option<String>,
    pub izin: Option<McpKipi>,
    pub arac_sayisi: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpSunucuSatiri {
    pub ozet: McpSunucuOzetiDto,
    pub alanlar: Vec<McpAlanDurumuDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpDurumuDto {
    pub revizyon: u64,
    pub sunucular: Vec<McpSunucuSatiri>,
    pub baslatma_kartlari: Vec<McpBaslatmaKarti>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpAlanAyrintisiDto {
    pub alan_kimligi: String,
    pub araclar: Vec<DenetlenmisMcpAracOzeti>,
    /// Kapatılan araçlar yalnız sıra numarasıyla (ham ad yok).
    pub reddedilenler: Vec<String>,
    /// Maskeli son stderr satırları (yalnız yerelde).
    pub stderr: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpSunucuAyrintisiDto {
    pub ayrinti: McpSunucuAyrintiDto,
    pub alanlar: Vec<McpAlanAyrintisiDto>,
}

/// Ana pencerenin MCP kanalı ve görüntü üreticisi.
pub struct McpArayuzKanali {
    yonetici: Arc<McpYoneticisi>,
    denetleyici: Arc<McpTanimDenetleyicisi>,
    depo: Arc<dyn McpAyarlarDeposu>,
    yazici: Mutex<Option<Box<dyn McpYazici>>>,
    revizyon: AtomicU64,
}

impl McpArayuzKanali {
    pub fn yeni(
        yonetici: Arc<McpYoneticisi>,
        denetleyici: Arc<McpTanimDenetleyicisi>,
        depo: Arc<dyn McpAyarlarDeposu>,
    ) -> Arc<Self> {
        Arc::new(Self {
            yonetici,
            denetleyici,
            depo,
            yazici: Mutex::default(),
            revizyon: AtomicU64::new(0),
        })
    }

    pub fn durum(&self) -> McpDurumuDto {
        let revizyon = self.revizyon.fetch_add(1, Ordering::SeqCst) + 1;
        let izinler = self.yonetici.kayitli_baslatma_izinleri();
        let sunucular = self
            .depo
            .mcp_ayarlarini_oku()
            .0
            .sunucular
            .iter()
            .map(|sunucu| {
                let alanlar = self
                    .yonetici
                    .sunucu_alanlari(&sunucu.ad)
                    .into_iter()
                    .map(|(alan, durum)| {
                        let izin = izinler
                            .iter()
                            .find(|(a, s, _)| *a == alan && *s == sunucu.ad)
                            .map(|(_, _, kip)| match kip {
                                CalistirmaKipi::Yalitimli => McpKipi::Yalitimli,
                                CalistirmaKipi::TamErisim => McpKipi::TamErisim,
                            });
                        let arac_sayisi = self
                            .denetleyici
                            .sunucu_durumu_ozeti(&alan, &sunucu.ad)
                            .map_or(0, |(_, ozetler, _)| {
                                ozetler.iter().filter(|o| o.sunulur).count()
                            });
                        let (etiket, kusak, hata) = durum_etiketi(&durum);
                        McpAlanDurumuDto {
                            alan_kimligi: alan,
                            durum: etiket,
                            kusak,
                            hata,
                            izin,
                            arac_sayisi,
                        }
                    })
                    .collect();
                McpSunucuSatiri {
                    ozet: sunucu.ozet_dto(),
                    alanlar,
                }
            })
            .collect();
        McpDurumuDto {
            revizyon,
            sunucular,
            baslatma_kartlari: self.yonetici.bekleyen_baslatma_kartlari(),
        }
    }

    /// Yeni kanal eskisinin yerini alır; güncel görüntü kilit altında yazılır.
    pub fn dinle(&self, yazici: Box<dyn McpYazici>) {
        let mut kanal = self.yazici.lock().unwrap_or_else(|z| z.into_inner());
        let _ = yazici.gonder(&self.durum());
        *kanal = Some(yazici);
    }

    /// Güncel görüntüyü kanala yazar. Kilit altında üretildiği için kanala
    /// giden revizyonlar sıralıdır.
    pub fn yay(&self) {
        let kanal = self.yazici.lock().unwrap_or_else(|z| z.into_inner());
        if let Some(yazici) = kanal.as_ref() {
            let _ = yazici.gonder(&self.durum());
        }
    }
}

fn durum_etiketi(durum: &McpSunucuDurumu) -> (McpDurumEtiketi, Option<u64>, Option<String>) {
    match durum {
        McpSunucuDurumu::Durdu => (McpDurumEtiketi::Durdu, None, None),
        McpSunucuDurumu::Basliyor { kusak, .. } => {
            (McpDurumEtiketi::Basliyor, Some(kusak.deger()), None)
        }
        McpSunucuDurumu::Calisiyor { kusak, .. } => {
            (McpDurumEtiketi::Calisiyor, Some(kusak.deger()), None)
        }
        McpSunucuDurumu::BostaKapandi => (McpDurumEtiketi::BostaKapandi, None, None),
        McpSunucuDurumu::Hata { mesaj, .. } => (McpDurumEtiketi::Hata, None, Some(mesaj.clone())),
    }
}

fn kayit_hatasi(hata: KayitHatasi) -> KomutHatasi {
    KomutHatasi::gecersiz(hata.to_string())
}

fn yonetici_hatasi(hata: impl std::fmt::Display) -> KomutHatasi {
    KomutHatasi::gecersiz(hata.to_string())
}

impl Uygulama {
    pub fn mcp_dinle(&self, yazici: Box<dyn McpYazici>) {
        self.mcp_kanali.dinle(yazici);
    }

    pub fn mcp_durumu(&self) -> McpDurumuDto {
        self.mcp_kanali.durum()
    }

    pub fn mcp_sunucu_ayrintisi(&self, ad: &str) -> Result<McpSunucuAyrintisiDto, KomutHatasi> {
        let ayarlar = self.ayarlar.mcp_ayarlarini_oku().0;
        let sunucu = ayarlar
            .sunucu(ad)
            .ok_or_else(|| KomutHatasi::gecersiz("Bu adla bir MCP sunucusu yok."))?;
        let alanlar = self
            .mcp
            .sunucu_alanlari(ad)
            .into_iter()
            .map(|(alan, _)| {
                let (araclar, reddedilenler) = self
                    .mcp_denetleyici
                    .sunucu_durumu_ozeti(&alan, ad)
                    .map(|(_, ozetler, reddedilenler)| {
                        (
                            ozetler,
                            reddedilenler.iter().map(|r| r.kullanici_mesaji()).collect(),
                        )
                    })
                    .unwrap_or_default();
                McpAlanAyrintisiDto {
                    stderr: self.mcp.stderr_satirlari(&alan, ad),
                    alan_kimligi: alan,
                    araclar,
                    reddedilenler,
                }
            })
            .collect();
        Ok(McpSunucuAyrintisiDto {
            ayrinti: sunucu.ayrinti_dto(),
            alanlar,
        })
    }

    pub async fn mcp_komut_sec(&self) -> Result<Option<SeciliKomutBilgisi>, KomutHatasi> {
        self.mcp_kayit.komut_sec(None).await.map_err(kayit_hatasi)
    }

    pub fn mcp_taslak_olustur(
        &self,
        eski_ad: Option<String>,
        form: McpSunucuFormu,
    ) -> Result<McpTaslakDto, KomutHatasi> {
        let (taslak_kimligi, kusak) = self.mcp_kayit.taslak_olustur(eski_ad, form.try_into()?);
        Ok(McpTaslakDto {
            taslak_kimligi,
            kusak,
        })
    }

    pub fn mcp_taslak_guncelle(
        &self,
        taslak_kimligi: TaslakKimligi,
        eski_ad: Option<String>,
        form: McpSunucuFormu,
    ) -> Result<u64, KomutHatasi> {
        self.mcp_kayit
            .taslak_guncelle(&taslak_kimligi, eski_ad, form.try_into()?)
            .map_err(kayit_hatasi)
    }

    /// Kaydın kod güveni hesabı için çalışma alanı: verilen oturumunki, yoksa
    /// son oturumunki. Arayüz yol göndermez.
    fn mcp_kayit_alani(&self, oturum: Option<OturumId>) -> Result<(String, PathBuf), KomutHatasi> {
        let ic = self.ic();
        let oturum = oturum.or(ic.son_oturum).ok_or_else(|| {
            KomutHatasi::gecersiz("MCP sunucusunu kaydetmek için bir çalışma alanında oturum açın.")
        })?;
        let acik = ic
            .oturumlar
            .get(&oturum)
            .ok_or_else(KomutHatasi::oturum_yok)?;
        Ok((
            acik.calisma_alani_kimligi.clone(),
            acik.calisma_alani.clone(),
        ))
    }

    fn mcp_kokler(&self, alan: &str, kok: PathBuf) -> YazilabilirKokler {
        YazilabilirKokler::yeni(kok, OzelDizinler::yol(self.mcp.yalitim_dizini(), alan))
    }

    /// Yerel onaylı kayıt. `false`: kullanıcı yerel pencerede reddetti.
    pub async fn mcp_taslak_kaydet(
        &self,
        taslak_kimligi: TaslakKimligi,
        kusak: u64,
        oturum: Option<OturumId>,
    ) -> Result<bool, KomutHatasi> {
        let (alan, kok) = self.mcp_kayit_alani(oturum)?;
        let kokler = self.mcp_kokler(&alan, kok);
        let sonuc = self
            .mcp_kayit
            .taslak_kaydet(
                &taslak_kimligi,
                kusak,
                &self.mcp.sistem_ortami(),
                self.mcp.yalitim_dizini(),
                &kokler,
                &alan,
            )
            .await;
        self.mcp_kayit_sonucu(sonuc)
    }

    pub fn mcp_ice_aktar_onizle(&self, json: &str) -> Result<IceAktarmaOnizlemeDto, KomutHatasi> {
        self.mcp_kayit.ice_aktar_onizle(json).map_err(kayit_hatasi)
    }

    pub fn mcp_ice_aktar_gizlilik(
        &self,
        taslak_kimligi: TaslakKimligi,
        sunucu: &str,
        ortam: &str,
        gizli: bool,
    ) -> Result<u64, KomutHatasi> {
        self.mcp_kayit
            .ice_aktar_ortam_gizliligi_degistir(&taslak_kimligi, sunucu, ortam, gizli)
            .map_err(kayit_hatasi)
    }

    pub async fn mcp_ice_aktar_kaydet(
        &self,
        taslak_kimligi: TaslakKimligi,
        kusak: u64,
        oturum: Option<OturumId>,
    ) -> Result<bool, KomutHatasi> {
        let (alan, kok) = self.mcp_kayit_alani(oturum)?;
        let kokler = self.mcp_kokler(&alan, kok);
        let sonuc = self
            .mcp_kayit
            .ice_aktar_kaydet(
                &taslak_kimligi,
                kusak,
                &self.mcp.sistem_ortami(),
                self.mcp.yalitim_dizini(),
                &kokler,
                &alan,
            )
            .await;
        self.mcp_kayit_sonucu(sonuc)
    }

    fn mcp_kayit_sonucu(
        &self,
        sonuc: Result<crate::mcp::tanimlar::KayitSonucu, KayitHatasi>,
    ) -> Result<bool, KomutHatasi> {
        match sonuc {
            Ok(kayit) => {
                // Değişen tanımların süreçleri durur; sonraki tur yeniden keşfeder.
                for ad in kayit.tanim_ozetleri.keys() {
                    self.mcp.sunucu_tanimi_degisti(ad);
                }
                self.mcp_kanali.yay();
                Ok(true)
            }
            Err(KayitHatasi::KullaniciReddetti) => Ok(false),
            Err(hata) => Err(kayit_hatasi(hata)),
        }
    }

    fn mcp_ayar_degistir(&self, degisiklik: McpDegisikligi) -> Result<(), KomutHatasi> {
        let mut islem = McpAyarlarIslemi::baslat(&*self.ayarlar);
        islem.degisiklik_ekle(degisiklik);
        islem
            .kaydet(&*self.ayarlar, &*self.mcp_kasa)
            .map_err(|e| KomutHatasi::gecersiz(e.to_string()))?;
        Ok(())
    }

    pub fn mcp_etkin_ayarla(&self, ad: String, etkin: bool) -> Result<(), KomutHatasi> {
        self.mcp_ayar_degistir(McpDegisikligi::EtkinlikAyarla {
            ad: ad.clone(),
            etkin,
        })?;
        self.mcp.sunucu_tanimi_degisti(&ad);
        self.mcp_kanali.yay();
        Ok(())
    }

    /// Yerel onaylı silme. `false`: kullanıcı reddetti.
    pub async fn mcp_sil(&self, ad: String) -> Result<bool, KomutHatasi> {
        if self.ayarlar.mcp_ayarlarini_oku().0.sunucu(&ad).is_none() {
            return Err(KomutHatasi::gecersiz("Bu adla bir MCP sunucusu yok."));
        }
        let metin = format!(
            "`{ad}` MCP sunucusu, kayıtlı gizli değerleri ve başlatma izinleriyle silinecek."
        );
        if !self
            .iletisim
            .onayla(SILME_BASLIGI, &metin, "Sil", "Vazgeç")
            .await
        {
            return Ok(false);
        }
        self.mcp_ayar_degistir(McpDegisikligi::Sil { ad: ad.clone() })?;
        self.mcp.sunucu_tanimi_degisti(&ad);
        for (alan, sunucu, _) in self.mcp.kayitli_baslatma_izinleri() {
            if sunucu == ad {
                self.mcp.baslatma_izni_geri_al(&alan, &ad);
            }
        }
        self.mcp_kanali.yay();
        Ok(true)
    }

    pub async fn mcp_yeniden_baslat(
        &self,
        oturum: OturumId,
        ad: String,
    ) -> Result<(), KomutHatasi> {
        let (alan, kok) = {
            let ic = self.ic();
            let acik = ic
                .oturumlar
                .get(&oturum)
                .ok_or_else(KomutHatasi::oturum_yok)?;
            (
                acik.calisma_alani_kimligi.clone(),
                acik.calisma_alani.clone(),
            )
        };
        let sonuc = self.mcp.kullanici_yeniden_baslat(&alan, &kok, &ad).await;
        self.mcp_kanali.yay();
        sonuc.map(|_| ()).map_err(yonetici_hatasi)
    }

    /// Model dışı başlatma kartının yanıtı. Kart tek kullanımlıdır; alanı ve
    /// kökü kartın kaydından gelir. Tam erişimde yönetici yerel onay ister.
    pub async fn mcp_baslatma_karti_yanitla(
        &self,
        istek_kimligi: String,
        kabul: bool,
    ) -> Result<bool, KomutHatasi> {
        let kart = self
            .mcp
            .bekleyen_baslatma_kartlari()
            .into_iter()
            .find(|k| k.istek_kimligi == istek_kimligi)
            .ok_or_else(|| KomutHatasi::gecersiz("Bu başlatma kartı artık geçerli değil."))?;
        let kok = self
            .mcp
            .alan_koku(&kart.alan_kimligi)
            .ok_or_else(|| KomutHatasi::gecersiz("Çalışma alanı açık değil."))?;
        let sonuc = self
            .mcp
            .baslatma_izni_yanitla(&istek_kimligi, &kart.alan_kimligi, &kok, kabul)
            .await;
        self.mcp_kanali.yay();
        sonuc.map_err(yonetici_hatasi)
    }

    pub fn mcp_baslatma_izni_geri_al(&self, alan_kimligi: String, ad: String) {
        self.mcp.baslatma_izni_geri_al(&alan_kimligi, &ad);
        self.mcp_kanali.yay();
    }
}
