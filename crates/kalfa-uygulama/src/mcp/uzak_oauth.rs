//! Uzak MCP OAuth 2.0 kullanıcı bağlama akışı, yerel onay kapısı, işletim sistemi
//! anahtarlığı (`GizliKasa`) entegrasyonu ve yenileme yarışı kilidi (`1.1b` Görev 4).
//!
//! Güvenlik kuralları:
//! - Sistem tarayıcısı (`Acici::ac`) açılmadan önce yerel kullanıcı onay kutusu
//!   (`Iletisim::onayla`) gösterilir; sunucu adı, hedef kaynak (`resource`),
//!   yetki sunucusu (`issuer`) ve istenen kapsamlar açıkça belirtilir.
//! - `access_token`, `refresh_token` ve PKCE `code_verifier` yalnızca arka uçta
//!   ve `GizliKasa` içinde tutulur; model bağlamına, arayüz DTO'larına, JSONL
//!   oturum kaydına veya hata metinlerine asla girmez.
//! - Eşzamanlı 401 / süresi dolmuş token durumlarında sunucu başına tek bir
//!   `tokio::sync::Mutex` yenileme kilidi kullanılır; ikinci çağrı ilk yenilemenin
//!   yazdığı yeni `kimlik_revizyonu` ve tokenı kullanır.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kalfa_mcp::maske::GizliDeger;
use kalfa_mcp::uzak::{
    OAuthDurumBaglami, OAuthGeriCagrimGirdisi, OAuthHatasi, OAuthTokenPaketi,
    OAuthYetkiSunucusuMetadata, SistemDnsCozucu, UzakDnsCozucu, UzakGizliBaslik,
    VARSAYILAN_AKIS_OMRU_SN, geri_cagrim_dogrula, kod_ile_token_al,
    korunmus_kaynak_metadatasini_getir, yenileme_tokeni_ile_token_al,
    yetki_sunucusu_metadatasini_getir, yetkilendirme_urlsi_olustur,
};
use kalfa_sandbox::tarama::CancellationToken;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::arayuz::{Acici, Iletisim};
use crate::mcp::ayarlar::{
    McpAyarlarDeposu, McpAyarlarIslemi, McpDegisikligi, UzakMcpYetkiAyari, YeniGizliDeger,
};
use crate::mcp::kasa::GizliKasa;
use crate::mcp::surec::{SaatKaynagi, SistemSaati};

const OAUTH_ONAY_BASLIGI: &str = "Uzak MCP OAuth hesabını bağla";

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UzakOAuthHatasi {
    #[error("uzak MCP sunucusu bulunamadı: {0}")]
    SunucuBulunamadi(String),
    #[error("uzak MCP sunucusu OAuth yetkilendirmesi kullanmıyor: {0}")]
    OAuthYapilandirilmamis(String),
    #[error("kullanıcı OAuth tarayıcı yetkilendirmesini yerel onay kutusunda reddetti")]
    KullaniciReddetti,
    #[error("tarayıcı açılamadı")]
    TarayiciAcilamadi,
    #[error("bekleyen OAuth yetkilendirme akışı bulunamadı veya iptal edildi")]
    AkisBulunamadiVeyaIptal,
    #[error("kayıtlı OAuth kimliği veya yenileme tokenı bulunmuyor")]
    KimlikBagliDegil,
    #[error("OAuth protokol hatası: {0}")]
    Protokol(#[from] OAuthHatasi),
    #[error("gizli değer kasası veya ayar kaydı hatası: {0}")]
    DepoHatasi(String),
}

/// Arayüze güvenle dönebilen OAuth akış başlatma özeti (sır veya PKCE verifier içermez).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthBaslatmaSonucuDto {
    pub sunucu_adi: String,
    pub state: String,
    pub issuer: String,
    pub resource: String,
    pub istenen_kapsamlar: Vec<String>,
}

/// OAuth geri çağrım (callback) tamamlandığında dönen güvenli özet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthTamamlamaSonucuDto {
    pub sunucu_adi: String,
    pub kimlik_revizyonu: u64,
    pub onaylanan_kapsamlar: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct KasadaSaklananOAuthPaketi {
    access_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    son_kullanma_sn: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    issuer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    token_endpoint: Option<String>,
}

struct BekleyenOAuthAkisi {
    baglam: OAuthDurumBaglami,
    yetki_sunucusu: OAuthYetkiSunucusuMetadata,
    client_id: String,
    test_fiksturu_loopback: bool,
    ozel_ag_izinli: bool,
    zaman_asimi: Duration,
    iptal: CancellationToken,
}

/// Uzak MCP sunucuları için OAuth 2.0 akışlarını, kasa sırlarını ve yenileme
/// kilitlerini yöneten bileşen.
pub struct UzakOAuthYoneticisi {
    mcp_depo: Arc<dyn McpAyarlarDeposu>,
    kasa: Arc<dyn GizliKasa>,
    iletisim: Arc<dyn Iletisim>,
    acici: Arc<dyn Acici>,
    saat: Arc<dyn SaatKaynagi>,
    dns_cozucu: Arc<dyn UzakDnsCozucu>,
    bekleyenler: Mutex<BTreeMap<String, BekleyenOAuthAkisi>>,
    iptal_edilen_stateler: Mutex<std::collections::BTreeSet<String>>,
    yenileme_kilitleri: Mutex<BTreeMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

impl UzakOAuthYoneticisi {
    pub fn yeni(
        mcp_depo: Arc<dyn McpAyarlarDeposu>,
        kasa: Arc<dyn GizliKasa>,
        iletisim: Arc<dyn Iletisim>,
        acici: Arc<dyn Acici>,
    ) -> Arc<Self> {
        Self::yeni_ozel(
            mcp_depo,
            kasa,
            iletisim,
            acici,
            Arc::new(SistemSaati::yeni()),
            Arc::new(SistemDnsCozucu),
        )
    }

    pub fn yeni_ozel(
        mcp_depo: Arc<dyn McpAyarlarDeposu>,
        kasa: Arc<dyn GizliKasa>,
        iletisim: Arc<dyn Iletisim>,
        acici: Arc<dyn Acici>,
        saat: Arc<dyn SaatKaynagi>,
        dns_cozucu: Arc<dyn UzakDnsCozucu>,
    ) -> Arc<Self> {
        Arc::new(Self {
            mcp_depo,
            kasa,
            iletisim,
            acici,
            saat,
            dns_cozucu,
            bekleyenler: Mutex::new(BTreeMap::new()),
            iptal_edilen_stateler: Mutex::new(std::collections::BTreeSet::new()),
            yenileme_kilitleri: Mutex::new(BTreeMap::new()),
        })
    }

    fn yenileme_kilidi(&self, sunucu_adi: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut kilitler = self
            .yenileme_kilitleri
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        kilitler
            .entry(sunucu_adi.to_owned())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    fn simdi_sn(&self) -> u64 {
        self.saat.simdi_ms() / 1000
    }

    /// Kullanıcının başlattığı OAuth yetkilendirme akışını hazırlar, metadata
    /// uçlarını doğrular, yerel onay alır ve sistem tarayıcısını açar.
    #[allow(clippy::too_many_arguments)]
    pub async fn oauth_akisi_baslat(
        &self,
        sunucu_adi: &str,
        oturum_kimligi: Option<String>,
        korunmus_kaynak_metadata_url: &str,
        yetki_sunucusu_metadata_url: &str,
        redirect_uri: &str,
        test_fiksturu_loopback: bool,
        ozel_ag_izinli: bool,
    ) -> Result<OAuthBaslatmaSonucuDto, UzakOAuthHatasi> {
        let (beklenen_kaynak_url, client_id, kapsamlar, zaman_asimi) = {
            let ayarlar = self.mcp_depo.mcp_ayarlarini_oku().0;
            let sunucu = ayarlar
                .uzak_sunucu(sunucu_adi)
                .ok_or_else(|| UzakOAuthHatasi::SunucuBulunamadi(sunucu_adi.to_owned()))?;
            let UzakMcpYetkiAyari::OAuth {
                istemci_kimligi,
                kapsamlar,
                kaynak_url,
                ..
            } = &sunucu.yetki
            else {
                return Err(UzakOAuthHatasi::OAuthYapilandirilmamis(
                    sunucu_adi.to_owned(),
                ));
            };
            let istemci = istemci_kimligi
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("kalfa")
                .to_owned();
            let hedef_kaynak = kaynak_url
                .as_deref()
                .unwrap_or(sunucu.uc_url.as_str())
                .to_owned();
            (
                hedef_kaynak,
                istemci,
                kapsamlar.clone(),
                Duration::from_secs(u64::from(sunucu.baslatma_zamani_asimi_sn.max(5))),
            )
        };

        let kaynak_meta = korunmus_kaynak_metadatasini_getir(
            korunmus_kaynak_metadata_url,
            &beklenen_kaynak_url,
            test_fiksturu_loopback,
            ozel_ag_izinli,
            zaman_asimi,
            &self.dns_cozucu,
        )
        .await?;

        let beklenen_issuer =
            kaynak_meta
                .authorization_servers
                .first()
                .cloned()
                .ok_or(UzakOAuthHatasi::Protokol(OAuthHatasi::MetadataGecersiz(
                    "authorization_servers boş",
                )))?;

        let yetki_meta = yetki_sunucusu_metadatasini_getir(
            yetki_sunucusu_metadata_url,
            &beklenen_issuer,
            test_fiksturu_loopback,
            ozel_ag_izinli,
            zaman_asimi,
            &self.dns_cozucu,
        )
        .await?;

        // Korunmuş kaynak metadatasındaki yetki sunucuları arasında beklenen issuer bulunmalıdır.
        if !kaynak_meta
            .authorization_servers
            .iter()
            .any(|s| s == &yetki_meta.issuer)
        {
            return Err(UzakOAuthHatasi::Protokol(OAuthHatasi::YanlisIssuer));
        }

        let kapsam_ozeti = if kapsamlar.is_empty() {
            "(varsayılan kapsam)".to_owned()
        } else {
            kapsamlar.join(", ")
        };
        let onay_mesaji = format!(
            "`{sunucu_adi}` uzak MCP sunucusu için tarayıcıda OAuth yetkilendirmesi açılacak.\n\
             Hedef kaynak: {}\n\
             Yetki sunucusu: {}\n\
             İstenen kapsamlar: {kapsam_ozeti}",
            kaynak_meta.resource, yetki_meta.issuer
        );
        if !self
            .iletisim
            .onayla(OAUTH_ONAY_BASLIGI, &onay_mesaji, "Tarayıcıda Aç", "Vazgeç")
            .await
        {
            return Err(UzakOAuthHatasi::KullaniciReddetti);
        }

        let baglam = OAuthDurumBaglami::yeni(
            sunucu_adi.to_owned(),
            oturum_kimligi,
            yetki_meta.issuer.clone(),
            kaynak_meta.resource.clone(),
            redirect_uri.to_owned(),
            kapsamlar.clone(),
            self.simdi_sn(),
        );
        let yetki_url = yetkilendirme_urlsi_olustur(&yetki_meta, &client_id, &baglam)?;
        let state = baglam.state.clone();
        let issuer = yetki_meta.issuer.clone();
        let resource = kaynak_meta.resource.clone();

        let iptal = CancellationToken::new();
        {
            let mut bekleyenler = self.bekleyenler.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(eski) = bekleyenler.remove(sunucu_adi) {
                eski.iptal.cancel();
                self.iptal_edilen_stateler
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(eski.baglam.state);
            }
            bekleyenler.insert(
                sunucu_adi.to_owned(),
                BekleyenOAuthAkisi {
                    baglam,
                    yetki_sunucusu: yetki_meta,
                    client_id,
                    test_fiksturu_loopback,
                    ozel_ag_izinli,
                    zaman_asimi,
                    iptal,
                },
            );
        }

        if self.acici.ac(&yetki_url).is_err() {
            self.oauth_akisi_iptal_et(sunucu_adi);
            return Err(UzakOAuthHatasi::TarayiciAcilamadi);
        }

        Ok(OAuthBaslatmaSonucuDto {
            sunucu_adi: sunucu_adi.to_owned(),
            state,
            issuer,
            resource,
            istenen_kapsamlar: kapsamlar,
        })
    }

    /// Bekleyen OAuth akışını iptal eder; bu akışa ait sonradan gelecek geri çağrımlar reddedilir.
    pub fn oauth_akisi_iptal_et(&self, sunucu_adi: &str) -> bool {
        let mut bekleyenler = self.bekleyenler.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(akis) = bekleyenler.remove(sunucu_adi) {
            akis.iptal.cancel();
            self.iptal_edilen_stateler
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(akis.baglam.state);
            true
        } else {
            false
        }
    }

    /// OAuth geri çağrımını (callback) doğrular, yetkilendirme kodunu token ile
    /// takas eder ve sırları yalnızca `GizliKasa` içine yazar.
    pub async fn oauth_geri_cagrim_tamamla(
        &self,
        sunucu_adi: &str,
        gelen_state: &str,
        gelen_oturum: Option<&str>,
        gelen_issuer: Option<&str>,
        gelen_kod: Option<&str>,
        gelen_hata: Option<&str>,
    ) -> Result<OAuthTamamlamaSonucuDto, UzakOAuthHatasi> {
        if self
            .iptal_edilen_stateler
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(gelen_state)
        {
            return Err(UzakOAuthHatasi::Protokol(
                OAuthHatasi::EskiVeyaTekrarlananCallback,
            ));
        }

        let (
            yetki_kodu,
            baglam_kopyasi,
            yetki_sunucusu,
            client_id,
            loopback,
            ozel_ag,
            zaman_asimi,
            iptal,
        ) = {
            let mut bekleyenler = self.bekleyenler.lock().unwrap_or_else(|e| e.into_inner());
            let Some(akis) = bekleyenler.get_mut(sunucu_adi) else {
                return Err(UzakOAuthHatasi::Protokol(
                    OAuthHatasi::EskiVeyaTekrarlananCallback,
                ));
            };
            if akis.iptal.is_cancelled() {
                return Err(UzakOAuthHatasi::Protokol(OAuthHatasi::IptalEdildi));
            }
            let kod = geri_cagrim_dogrula(
                &mut akis.baglam,
                &OAuthGeriCagrimGirdisi {
                    gelen_state,
                    gelen_oturum,
                    gelen_issuer,
                    gelen_kod,
                    gelen_hata,
                    simdi_sn: self.simdi_sn(),
                    azami_yas_sn: VARSAYILAN_AKIS_OMRU_SN,
                    test_fiksturu_loopback: akis.test_fiksturu_loopback,
                },
            )?;
            (
                kod,
                akis.baglam.clone(),
                akis.yetki_sunucusu.clone(),
                akis.client_id.clone(),
                akis.test_fiksturu_loopback,
                akis.ozel_ag_izinli,
                akis.zaman_asimi,
                akis.iptal.clone(),
            )
        };

        let token_paketi = tokio::select! {
            biased;
            _ = iptal.cancelled() => {
                return Err(UzakOAuthHatasi::Protokol(OAuthHatasi::IptalEdildi));
            }
            sonuc = kod_ile_token_al(
                &yetki_sunucusu.token_endpoint,
                &client_id,
                &yetki_kodu,
                &baglam_kopyasi,
                loopback,
                ozel_ag,
                zaman_asimi,
                &self.dns_cozucu,
            ) => sonuc?,
        };

        if iptal.is_cancelled() {
            return Err(UzakOAuthHatasi::Protokol(OAuthHatasi::IptalEdildi));
        }

        let son_kullanma_sn = token_paketi
            .expires_in
            .map(|sure| self.simdi_sn().saturating_add(sure));
        let revizyon = self.token_paketini_kasaya_ve_ayarlara_yaz(
            sunucu_adi,
            &token_paketi,
            son_kullanma_sn,
            Some(baglam_kopyasi.beklenen_issuer),
            Some(yetki_sunucusu.token_endpoint),
        )?;

        Ok(OAuthTamamlamaSonucuDto {
            sunucu_adi: sunucu_adi.to_owned(),
            kimlik_revizyonu: revizyon,
            onaylanan_kapsamlar: token_paketi.onaylanan_kapsamlar,
        })
    }

    fn token_paketini_kasaya_ve_ayarlara_yaz(
        &self,
        sunucu_adi: &str,
        paket: &OAuthTokenPaketi,
        son_kullanma_sn: Option<u64>,
        issuer: Option<String>,
        token_endpoint: Option<String>,
    ) -> Result<u64, UzakOAuthHatasi> {
        let saklanan = KasadaSaklananOAuthPaketi {
            access_token: paket.access_token().to_owned(),
            refresh_token: paket.refresh_token().map(str::to_owned),
            son_kullanma_sn,
            issuer,
            token_endpoint,
        };
        let paket_json = serde_json::to_string(&saklanan)
            .map_err(|e| UzakOAuthHatasi::DepoHatasi(e.to_string()))?;
        let token_paketi_json = YeniGizliDeger::yeni(paket_json)
            .map_err(|e| UzakOAuthHatasi::DepoHatasi(e.to_string()))?;

        let mut islem = McpAyarlarIslemi::baslat(&*self.mcp_depo);
        islem.degisiklik_ekle(McpDegisikligi::UzakOAuthKimlikBagla {
            ad: sunucu_adi.to_owned(),
            token_paketi_json,
        });
        if let Err(hata) = islem.kaydet(&*self.mcp_depo, &*self.kasa)
            && !hata.kayit_tamamlandi_mi()
        {
            return Err(UzakOAuthHatasi::DepoHatasi(hata.to_string()));
        }

        let ayarlar = self.mcp_depo.mcp_ayarlarini_oku().0;
        let rev = ayarlar
            .uzak_sunucu(sunucu_adi)
            .map(|s| s.kimlik_revizyonu)
            .ok_or_else(|| UzakOAuthHatasi::SunucuBulunamadi(sunucu_adi.to_owned()))?;
        Ok(rev)
    }

    /// Uzak sunucunun bağlı kimliğini kasadan ve ayarlardan kaldırır;
    /// `kimlik_revizyonu` artarak eski çalışma tariflerini geçersiz kılar.
    pub fn kimlik_kaldir(&self, sunucu_adi: &str) -> Result<u64, UzakOAuthHatasi> {
        self.oauth_akisi_iptal_et(sunucu_adi);
        let mut islem = McpAyarlarIslemi::baslat(&*self.mcp_depo);
        islem.degisiklik_ekle(McpDegisikligi::UzakKimlikKaldir {
            ad: sunucu_adi.to_owned(),
        });
        if let Err(hata) = islem.kaydet(&*self.mcp_depo, &*self.kasa)
            && !hata.kayit_tamamlandi_mi()
        {
            return Err(UzakOAuthHatasi::DepoHatasi(hata.to_string()));
        }
        let ayarlar = self.mcp_depo.mcp_ayarlarini_oku().0;
        let rev = ayarlar
            .uzak_sunucu(sunucu_adi)
            .map(|s| s.kimlik_revizyonu)
            .ok_or_else(|| UzakOAuthHatasi::SunucuBulunamadi(sunucu_adi.to_owned()))?;
        Ok(rev)
    }

    /// Sunucunun güncel yetki başlığını (`Authorization: Bearer ...` veya sabit gizli başlık)
    /// kasadan çözer. OAuth erişim tokenının süresi dolmuşsa veya `zorla_yenile`
    /// verilmişse sunucu başına tekil yenileme kilidi altında token yeniler.
    ///
    /// Eşzamanlı iki çağrı aynı `beklenen_kimlik_revizyonu` ile yenilemeye girerse
    /// yalnız ilki HTTP yenileme isteği yapar; ikincisi kilit açıldığında yeni
    /// revizyonun tokenını kasadan okur.
    #[allow(clippy::too_many_arguments)]
    pub async fn gecerli_gizli_baslik_coz_veya_yenile(
        &self,
        sunucu_adi: &str,
        beklenen_kimlik_revizyonu: u64,
        zorla_yenile: bool,
        token_endpoint_override: Option<&str>,
        test_fiksturu_loopback: bool,
        ozel_ag_izinli: bool,
    ) -> Result<(Option<UzakGizliBaslik>, u64), UzakOAuthHatasi> {
        let anlik = {
            let ayarlar = self.mcp_depo.mcp_ayarlarini_oku().0;
            ayarlar
                .uzak_sunucu(sunucu_adi)
                .cloned()
                .ok_or_else(|| UzakOAuthHatasi::SunucuBulunamadi(sunucu_adi.to_owned()))?
        };

        match &anlik.yetki {
            UzakMcpYetkiAyari::Yetkisiz => Ok((None, anlik.kimlik_revizyonu)),
            UzakMcpYetkiAyari::SabitBaslik {
                baslik_adi,
                gizli_kimlik,
            } => {
                let deger = self
                    .kasa
                    .oku(gizli_kimlik)
                    .map_err(|e| UzakOAuthHatasi::DepoHatasi(e.to_string()))?
                    .ok_or(UzakOAuthHatasi::KimlikBagliDegil)?;
                let baslik = if baslik_adi.eq_ignore_ascii_case("authorization")
                    && let Some(token) = deger
                        .strip_prefix("Bearer ")
                        .or_else(|| deger.strip_prefix("bearer "))
                {
                    UzakGizliBaslik::bearer(token).map_err(|_| {
                        UzakOAuthHatasi::DepoHatasi("geçersiz Bearer başlığı".into())
                    })?
                } else {
                    UzakGizliBaslik::sabit_baslik(baslik_adi, &deger)
                        .map_err(|_| UzakOAuthHatasi::DepoHatasi("geçersiz sabit başlık".into()))?
                };
                Ok((Some(baslik), anlik.kimlik_revizyonu))
            }
            UzakMcpYetkiAyari::OAuth { yetki_kimligi, .. } => {
                if !zorla_yenile
                    && let Some(kimlik) = yetki_kimligi
                    && let Some(ham_json) = self
                        .kasa
                        .oku(kimlik)
                        .map_err(|e| UzakOAuthHatasi::DepoHatasi(e.to_string()))?
                    && let Ok(paket) = serde_json::from_str::<KasadaSaklananOAuthPaketi>(&ham_json)
                {
                    let suresi_doldu = paket
                        .son_kullanma_sn
                        .is_some_and(|bitis| self.simdi_sn() >= bitis);
                    if !suresi_doldu {
                        let baslik =
                            UzakGizliBaslik::bearer(&paket.access_token).map_err(|_| {
                                UzakOAuthHatasi::DepoHatasi("geçersiz OAuth Bearer başlığı".into())
                            })?;
                        return Ok((Some(baslik), anlik.kimlik_revizyonu));
                    }
                }

                // Yenileme kilidini al: eşzamanlı 401/süre dolumu yarışında tek yenileme yapılır.
                let kilit = self.yenileme_kilidi(sunucu_adi);
                let _koruma = kilit.lock().await;

                let guncel = {
                    let ayarlar = self.mcp_depo.mcp_ayarlarini_oku().0;
                    ayarlar
                        .uzak_sunucu(sunucu_adi)
                        .cloned()
                        .ok_or_else(|| UzakOAuthHatasi::SunucuBulunamadi(sunucu_adi.to_owned()))?
                };

                let UzakMcpYetkiAyari::OAuth {
                    istemci_kimligi,
                    kapsamlar,
                    kaynak_url,
                    yetki_kimligi,
                } = &guncel.yetki
                else {
                    return Err(UzakOAuthHatasi::OAuthYapilandirilmamis(
                        sunucu_adi.to_owned(),
                    ));
                };

                let kimlik = yetki_kimligi
                    .as_ref()
                    .ok_or(UzakOAuthHatasi::KimlikBagliDegil)?;
                let ham_json = self
                    .kasa
                    .oku(kimlik)
                    .map_err(|e| UzakOAuthHatasi::DepoHatasi(e.to_string()))?
                    .ok_or(UzakOAuthHatasi::KimlikBagliDegil)?;
                let mevcut_paket: KasadaSaklananOAuthPaketi = serde_json::from_str(&ham_json)
                    .map_err(|e| UzakOAuthHatasi::DepoHatasi(e.to_string()))?;

                // Biz kilidi beklerken başka bir görev tokenı zaten yenilediyse
                // (`kimlik_revizyonu` ilerlediyse ve yeni token geçerliyse) tekrar yenileme yapma.
                let guncel_suresi_doldu = mevcut_paket
                    .son_kullanma_sn
                    .is_some_and(|bitis| self.simdi_sn() >= bitis);
                if guncel.kimlik_revizyonu > beklenen_kimlik_revizyonu && !guncel_suresi_doldu {
                    let baslik =
                        UzakGizliBaslik::bearer(&mevcut_paket.access_token).map_err(|_| {
                            UzakOAuthHatasi::DepoHatasi("geçersiz OAuth Bearer başlığı".into())
                        })?;
                    return Ok((Some(baslik), guncel.kimlik_revizyonu));
                }

                let mevcut_refresh = mevcut_paket
                    .refresh_token
                    .as_deref()
                    .ok_or(UzakOAuthHatasi::KimlikBagliDegil)?;
                let client_id = istemci_kimligi
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .unwrap_or("kalfa");
                let hedef_kaynak = kaynak_url.as_deref().unwrap_or(&guncel.uc_url);

                let zaman_asimi =
                    Duration::from_secs(u64::from(guncel.baslatma_zamani_asimi_sn.max(5)));
                let token_endpoint = match token_endpoint_override {
                    Some(ep) => ep.to_owned(),
                    None => match mevcut_paket.token_endpoint.as_deref() {
                        Some(ep) => ep.to_owned(),
                        None => {
                            let issuer = mevcut_paket
                                .issuer
                                .as_deref()
                                .ok_or(UzakOAuthHatasi::KimlikBagliDegil)?;
                            let meta_url = format!(
                                "{}/.well-known/oauth-authorization-server",
                                issuer.trim_end_matches('/')
                            );
                            let meta = yetki_sunucusu_metadatasini_getir(
                                &meta_url,
                                issuer,
                                test_fiksturu_loopback,
                                ozel_ag_izinli,
                                zaman_asimi,
                                &self.dns_cozucu,
                            )
                            .await?;
                            meta.token_endpoint
                        }
                    },
                };

                let yeni_paket = yenileme_tokeni_ile_token_al(
                    &token_endpoint,
                    client_id,
                    mevcut_refresh,
                    hedef_kaynak,
                    kapsamlar,
                    test_fiksturu_loopback,
                    ozel_ag_izinli,
                    zaman_asimi,
                    &self.dns_cozucu,
                )
                .await?;

                let yeni_bitis = yeni_paket
                    .expires_in
                    .map(|sure| self.simdi_sn().saturating_add(sure));
                let guncellenmis_saklanan = KasadaSaklananOAuthPaketi {
                    access_token: yeni_paket.access_token().to_owned(),
                    refresh_token: Some(
                        yeni_paket
                            .refresh_token()
                            .unwrap_or(mevcut_refresh)
                            .to_owned(),
                    ),
                    son_kullanma_sn: yeni_bitis,
                    issuer: mevcut_paket.issuer.clone(),
                    token_endpoint: Some(token_endpoint),
                };
                let guncel_json = serde_json::to_string(&guncellenmis_saklanan)
                    .map_err(|e| UzakOAuthHatasi::DepoHatasi(e.to_string()))?;
                let token_paketi_json = YeniGizliDeger::yeni(guncel_json)
                    .map_err(|e| UzakOAuthHatasi::DepoHatasi(e.to_string()))?;

                let mut islem = McpAyarlarIslemi::baslat(&*self.mcp_depo);
                islem.degisiklik_ekle(McpDegisikligi::UzakOAuthKimlikBagla {
                    ad: sunucu_adi.to_owned(),
                    token_paketi_json,
                });
                if let Err(hata) = islem.kaydet(&*self.mcp_depo, &*self.kasa)
                    && !hata.kayit_tamamlandi_mi()
                {
                    return Err(UzakOAuthHatasi::DepoHatasi(hata.to_string()));
                }

                let son_ayarlar = self.mcp_depo.mcp_ayarlarini_oku().0;
                let yeni_rev = son_ayarlar
                    .uzak_sunucu(sunucu_adi)
                    .map(|s| s.kimlik_revizyonu)
                    .ok_or_else(|| UzakOAuthHatasi::SunucuBulunamadi(sunucu_adi.to_owned()))?;
                let baslik = UzakGizliBaslik::bearer(yeni_paket.access_token()).map_err(|_| {
                    UzakOAuthHatasi::DepoHatasi("geçersiz OAuth Bearer başlığı".into())
                })?;
                Ok((Some(baslik), yeni_rev))
            }
        }
    }

    /// Uzak sunucunun kasada kayıtlı tüm sırlarını (`access_token`, `refresh_token`
    /// veya gizli başlık değeri) maskeleme katmanı için `GizliDeger` listesi olarak döner.
    pub fn sunucu_sirlarini_oku(&self, sunucu_adi: &str) -> Vec<GizliDeger> {
        let ayarlar = self.mcp_depo.mcp_ayarlarini_oku().0;
        let Some(sunucu) = ayarlar.uzak_sunucu(sunucu_adi) else {
            return Vec::new();
        };
        let mut sirlar = Vec::new();
        for kimlik in sunucu.gizli_kimlikler() {
            if let Ok(Some(deger)) = self.kasa.oku(&kimlik) {
                if let Ok(paket) = serde_json::from_str::<KasadaSaklananOAuthPaketi>(&deger) {
                    if let Ok(g) = GizliDeger::yeni(paket.access_token) {
                        sirlar.push(g);
                    }
                    if let Some(rt) = paket.refresh_token
                        && let Ok(g) = GizliDeger::yeni(rt)
                    {
                        sirlar.push(g);
                    }
                }
                if let Ok(gizli) = GizliDeger::yeni(deger) {
                    sirlar.push(gizli);
                }
            }
        }
        sirlar
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use kalfa_mcp::uzak::SabitDnsCozucu;
    use serde_json::{Value, json};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    use crate::ayarlar::Ayarlar;
    use crate::mcp::ayarlar::{UzakMcpSunucuTaslagi, UzakMcpYetkiGirdisi};
    use crate::mcp::kasa::test_destek::SahteGizliKasa;
    use crate::mcp::surec::SahteSaat;
    use crate::test_destek::{SahteAcici, SahteIletisim};

    const SENTETIK_ERISIM_CANARY: &str = "CANARY-OAUTH-ACCESS-TOKEN-99887766";
    const SENTETIK_YENILEME_CANARY: &str = "CANARY-OAUTH-REFRESH-TOKEN-55443322";
    const SENTETIK_YENI_ERISIM_CANARY: &str = "CANARY-OAUTH-REFRESHED-ACCESS-11223344";

    struct SahteOAuthSunucusu {
        adres: SocketAddr,
        token_istek_sayaci: Arc<AtomicUsize>,
        yenileme_istek_sayaci: Arc<AtomicUsize>,
        kapat_tx: Option<tokio::sync::watch::Sender<bool>>,
        gorev: tokio::task::JoinHandle<()>,
    }

    impl SahteOAuthSunucusu {
        async fn baslat(
            resource_url: String,
            issuer_override: Option<String>,
            gecikmeli_yenileme_ms: u64,
        ) -> Self {
            let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let adres = dinleyici.local_addr().unwrap();
            let token_istek_sayaci = Arc::new(AtomicUsize::new(0));
            let yenileme_istek_sayaci = Arc::new(AtomicUsize::new(0));
            let (kapat_tx, mut kapat_rx) = tokio::sync::watch::channel(false);

            let t_sayac = token_istek_sayaci.clone();
            let y_sayac = yenileme_istek_sayaci.clone();
            let varsayilan_issuer = format!("http://{adres}/issuer");
            let efektif_issuer = issuer_override.unwrap_or(varsayilan_issuer);

            let gorev = tokio::spawn(async move {
                loop {
                    let kabul = tokio::select! {
                        biased;
                        _ = kapat_rx.changed() => break,
                        sonuc = dinleyici.accept() => sonuc,
                    };
                    let Ok((soket, _)) = kabul else { break };
                    let resource_url = resource_url.clone();
                    let efektif_issuer = efektif_issuer.clone();
                    let t_sayac = t_sayac.clone();
                    let y_sayac = y_sayac.clone();
                    tokio::spawn(async move {
                        let _ = oauth_baglantiyi_isle(
                            soket,
                            adres,
                            &resource_url,
                            &efektif_issuer,
                            t_sayac,
                            y_sayac,
                            gecikmeli_yenileme_ms,
                        )
                        .await;
                    });
                }
            });

            Self {
                adres,
                token_istek_sayaci,
                yenileme_istek_sayaci,
                kapat_tx: Some(kapat_tx),
                gorev,
            }
        }

        fn korunmus_kaynak_meta_url(&self) -> String {
            format!("http://{}/.well-known/oauth-protected-resource", self.adres)
        }

        fn yetki_sunucusu_meta_url(&self) -> String {
            format!(
                "http://{}/.well-known/oauth-authorization-server",
                self.adres
            )
        }

        fn issuer_url(&self) -> String {
            format!("http://{}/issuer", self.adres)
        }

        fn token_url(&self) -> String {
            format!("http://{}/token", self.adres)
        }

        async fn kapat(mut self) {
            if let Some(tx) = self.kapat_tx.take() {
                let _ = tx.send(true);
            }
            let _ = tokio::time::timeout(Duration::from_secs(1), self.gorev).await;
        }
    }

    async fn http_oku(soket: &mut TcpStream) -> Option<(String, String)> {
        let mut tampon = Vec::new();
        let mut gecici = [0u8; 4096];
        let baslik_sonu = loop {
            let n = soket.read(&mut gecici).await.ok()?;
            if n == 0 {
                return None;
            }
            tampon.extend_from_slice(&gecici[..n]);
            if let Some(konum) = tampon.windows(4).position(|w| w == b"\r\n\r\n") {
                break konum + 4;
            }
        };
        let baslik_metni = String::from_utf8_lossy(&tampon[..baslik_sonu]);
        let ilk_satir = baslik_metni.lines().next().unwrap_or_default().to_owned();
        let mut icerik_uzunlugu = 0usize;
        for satir in baslik_metni.lines().skip(1) {
            if let Some((ad, deger)) = satir.split_once(':')
                && ad.trim().eq_ignore_ascii_case("content-length")
            {
                icerik_uzunlugu = deger.trim().parse().unwrap_or(0);
            }
        }
        let mut govde = tampon[baslik_sonu..].to_vec();
        while govde.len() < icerik_uzunlugu {
            let n = soket.read(&mut gecici).await.ok()?;
            if n == 0 {
                break;
            }
            govde.extend_from_slice(&gecici[..n]);
        }
        govde.truncate(icerik_uzunlugu);
        Some((ilk_satir, String::from_utf8_lossy(&govde).into_owned()))
    }

    async fn json_yaz(soket: &mut TcpStream, durum: &str, govde: &Value) {
        let baytlar = serde_json::to_vec(govde).unwrap();
        let baslik = format!(
            "HTTP/1.1 {durum}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            baytlar.len()
        );
        let _ = soket.write_all(baslik.as_bytes()).await;
        let _ = soket.write_all(&baytlar).await;
        let _ = soket.flush().await;
    }

    async fn oauth_baglantiyi_isle(
        mut soket: TcpStream,
        adres: SocketAddr,
        resource_url: &str,
        issuer_url: &str,
        token_istek_sayaci: Arc<AtomicUsize>,
        yenileme_istek_sayaci: Arc<AtomicUsize>,
        gecikmeli_yenileme_ms: u64,
    ) -> Option<()> {
        let (ilk_satir, govde) = http_oku(&mut soket).await?;
        if ilk_satir.contains("/.well-known/oauth-protected-resource") {
            json_yaz(
                &mut soket,
                "200 OK",
                &json!({
                    "resource": resource_url,
                    "authorization_servers": [format!("http://{adres}/issuer")],
                    "scopes_supported": ["tools:read", "tools:call"]
                }),
            )
            .await;
            return Some(());
        }

        if ilk_satir.contains("/.well-known/oauth-authorization-server") {
            json_yaz(
                &mut soket,
                "200 OK",
                &json!({
                    "issuer": issuer_url,
                    "authorization_endpoint": format!("http://{adres}/authorize"),
                    "token_endpoint": format!("http://{adres}/token"),
                    "code_challenge_methods_supported": ["S256"]
                }),
            )
            .await;
            return Some(());
        }

        if ilk_satir.starts_with("POST /token") {
            token_istek_sayaci.fetch_add(1, Ordering::SeqCst);
            let form: HashMap<String, String> = url::form_urlencoded::parse(govde.as_bytes())
                .into_owned()
                .collect();
            let grant_type = form
                .get("grant_type")
                .map(String::as_str)
                .unwrap_or_default();
            if grant_type == "authorization_code" {
                if form.get("code").map(String::as_str) == Some("dar-kapsam-kodu") {
                    json_yaz(
                        &mut soket,
                        "200 OK",
                        &json!({
                            "access_token": SENTETIK_ERISIM_CANARY,
                            "token_type": "Bearer",
                            "expires_in": 60,
                            "scope": "tools:read"
                        }),
                    )
                    .await;
                    return Some(());
                }
                json_yaz(
                    &mut soket,
                    "200 OK",
                    &json!({
                        "access_token": SENTETIK_ERISIM_CANARY,
                        "refresh_token": SENTETIK_YENILEME_CANARY,
                        "token_type": "Bearer",
                        "expires_in": 60,
                        "scope": "tools:read tools:call",
                        "resource": resource_url
                    }),
                )
                .await;
                return Some(());
            } else if grant_type == "refresh_token" {
                yenileme_istek_sayaci.fetch_add(1, Ordering::SeqCst);
                if gecikmeli_yenileme_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(gecikmeli_yenileme_ms)).await;
                }
                json_yaz(
                    &mut soket,
                    "200 OK",
                    &json!({
                        "access_token": SENTETIK_YENI_ERISIM_CANARY,
                        "refresh_token": SENTETIK_YENILEME_CANARY,
                        "token_type": "Bearer",
                        "expires_in": 3600,
                        "scope": "tools:read tools:call",
                        "resource": resource_url
                    }),
                )
                .await;
                return Some(());
            }
        }

        None
    }

    #[tokio::test]
    async fn m03_oauth_tam_akis_yerel_onay_state_oturum_issuer_yenileme_yarisi_ve_canary_denetimi()
    {
        let gecici = tempfile::tempdir().unwrap();
        let ayarlar = Arc::new(Ayarlar::oku(gecici.path()));
        let kasa = Arc::new(SahteGizliKasa::yeni());
        let iletisim = Arc::new(SahteIletisim::default());
        let acici = Arc::new(SahteAcici::default());
        let saat = SahteSaat::yeni();
        saat.ayarla(Duration::from_secs(1_000));

        let resource_url = "http://127.0.0.1:19443/mcp".to_owned();
        let sahte_oauth = SahteOAuthSunucusu::baslat(resource_url.clone(), None, 80).await;

        // Uzak sunucuyu kaydet.
        let mut islem = McpAyarlarIslemi::baslat(&*ayarlar);
        islem.degisiklik_ekle(McpDegisikligi::UzakKaydet {
            eski_ad: None,
            taslak: UzakMcpSunucuTaslagi {
                ad: "uzak-oauth".into(),
                etkin: true,
                uc_url: resource_url.clone(),
                yetki: UzakMcpYetkiGirdisi::OAuth {
                    istemci_kimligi: Some("kalfa-istemci".into()),
                    kapsamlar: vec!["tools:read".into(), "tools:call".into()],
                    kaynak_url: Some(resource_url.clone()),
                    korunan_yetki_kimligi: None,
                },
                baslatma_zamani_asimi_sn: 10,
                cagri_zamani_asimi_sn: 30,
            },
            test_fiksturu_loopback: true,
        });
        islem.kaydet(&*ayarlar, &*kasa).unwrap();

        let yonetici = UzakOAuthYoneticisi::yeni_ozel(
            ayarlar.clone(),
            kasa.clone(),
            iletisim.clone(),
            acici.clone(),
            saat.clone(),
            Arc::new(SabitDnsCozucu::yeni()),
        );

        // 1. Kullanıcı yerel onay kutusunda reddederse tarayıcı açılmaz.
        iletisim.onay_ver(false);
        let red_sonucu = yonetici
            .oauth_akisi_baslat(
                "uzak-oauth",
                Some("oturum-1".into()),
                &sahte_oauth.korunmus_kaynak_meta_url(),
                &sahte_oauth.yetki_sunucusu_meta_url(),
                "http://127.0.0.1:49152/callback",
                true,
                false,
            )
            .await;
        assert_eq!(red_sonucu, Err(UzakOAuthHatasi::KullaniciReddetti));
        assert!(acici.acilanlar().is_empty());

        // 2. Kullanıcı yerel onayı kabul ederse tarayıcı PKCE S256 ve state ile açılır.
        iletisim.onay_ver(true);
        let baslatma = yonetici
            .oauth_akisi_baslat(
                "uzak-oauth",
                Some("oturum-1".into()),
                &sahte_oauth.korunmus_kaynak_meta_url(),
                &sahte_oauth.yetki_sunucusu_meta_url(),
                "http://127.0.0.1:49152/callback",
                true,
                false,
            )
            .await
            .unwrap();
        assert_eq!(acici.acilanlar().len(), 1);
        assert!(acici.acilanlar()[0].contains("code_challenge_method=S256"));
        assert!(acici.acilanlar()[0].contains(&format!("state={}", baslatma.state)));

        // 3. Yanlış oturum veya yanlış state geri çağrımı reddedilir.
        assert_eq!(
            yonetici
                .oauth_geri_cagrim_tamamla(
                    "uzak-oauth",
                    &baslatma.state,
                    Some("baska-oturum"),
                    Some(&sahte_oauth.issuer_url()),
                    Some("yetki-kodu-1"),
                    None,
                )
                .await,
            Err(UzakOAuthHatasi::Protokol(OAuthHatasi::YanlisOturum))
        );
        assert_eq!(
            yonetici
                .oauth_geri_cagrim_tamamla(
                    "uzak-oauth",
                    "yanlis-state-degeri",
                    Some("oturum-1"),
                    Some(&sahte_oauth.issuer_url()),
                    Some("yetki-kodu-1"),
                    None,
                )
                .await,
            Err(UzakOAuthHatasi::Protokol(OAuthHatasi::YanlisState))
        );

        // 4. Geçerli geri çağrım tokenları kasaya yazar ve kimlik_revizyonu'nu artırır.
        let tamamlama = yonetici
            .oauth_geri_cagrim_tamamla(
                "uzak-oauth",
                &baslatma.state,
                Some("oturum-1"),
                Some(&sahte_oauth.issuer_url()),
                Some("yetki-kodu-1"),
                None,
            )
            .await
            .unwrap();
        assert_eq!(tamamlama.kimlik_revizyonu, 2);
        assert_eq!(kasa.kayit_sayisi(), 1);
        assert_eq!(sahte_oauth.token_istek_sayaci.load(Ordering::SeqCst), 1);

        // Aynı callback tekrar kullanılırsa EskiVeyaTekrarlananCallback döner.
        assert_eq!(
            yonetici
                .oauth_geri_cagrim_tamamla(
                    "uzak-oauth",
                    &baslatma.state,
                    Some("oturum-1"),
                    Some(&sahte_oauth.issuer_url()),
                    Some("yetki-kodu-1"),
                    None,
                )
                .await,
            Err(UzakOAuthHatasi::Protokol(
                OAuthHatasi::EskiVeyaTekrarlananCallback
            ))
        );

        // 5. Sentetik canary denetimi: ayarlar.json, DTO'lar ve onay mesajları ham token taşımaz!
        let ayarlar_json = std::fs::read_to_string(gecici.path().join("ayarlar.json")).unwrap();
        let sunucu_kaydi = ayarlar
            .mcp_ayarlarini_oku()
            .0
            .uzak_sunucu("uzak-oauth")
            .unwrap()
            .clone();
        let ozet_json = serde_json::to_string(&sunucu_kaydi.ozet_dto()).unwrap();
        let ayrinti_json = serde_json::to_string(&sunucu_kaydi.ayrinti_dto()).unwrap();
        let tamamlama_json = serde_json::to_string(&tamamlama).unwrap();
        for yuzey in [&ayarlar_json, &ozet_json, &ayrinti_json, &tamamlama_json] {
            assert!(!yuzey.contains(SENTETIK_ERISIM_CANARY));
            assert!(!yuzey.contains(SENTETIK_YENILEME_CANARY));
        }

        // 6. Eşzamanlı yenileme yarışı: iki görev aynı anda süresi dolmuş token için
        // yenilemeye girdiğinde sunucuya yalnız TEK refresh_token isteği gider!
        saat.ilerlet(Duration::from_secs(120)); // 60 sn'lik expires_in doldu
        let y1 = yonetici.clone();
        let y2 = yonetici.clone();
        let token_ep = sahte_oauth.token_url();
        let token_ep_2 = token_ep.clone();
        let (sonuc_1, sonuc_2) = tokio::join!(
            y1.gecerli_gizli_baslik_coz_veya_yenile(
                "uzak-oauth",
                2,
                false,
                Some(&token_ep),
                true,
                false,
            ),
            y2.gecerli_gizli_baslik_coz_veya_yenile(
                "uzak-oauth",
                2,
                false,
                Some(&token_ep_2),
                true,
                false,
            ),
        );
        let (_, rev_1) = sonuc_1.unwrap();
        let (_, rev_2) = sonuc_2.unwrap();
        assert_eq!(rev_1, 3);
        assert_eq!(rev_2, 3);
        assert_eq!(
            sahte_oauth.yenileme_istek_sayaci.load(Ordering::SeqCst),
            1,
            "eşzamanlı yenileme yarışında yalnız 1 refresh HTTP isteği yapılmalıdır"
        );

        // 7. İptal edilen akışın sonradan gelen callback'i reddedilir.
        let baslatma_2 = yonetici
            .oauth_akisi_baslat(
                "uzak-oauth",
                Some("oturum-2".into()),
                &sahte_oauth.korunmus_kaynak_meta_url(),
                &sahte_oauth.yetki_sunucusu_meta_url(),
                "http://127.0.0.1:49152/callback",
                true,
                false,
            )
            .await
            .unwrap();
        assert!(yonetici.oauth_akisi_iptal_et("uzak-oauth"));
        assert_eq!(
            yonetici
                .oauth_geri_cagrim_tamamla(
                    "uzak-oauth",
                    &baslatma_2.state,
                    Some("oturum-2"),
                    Some(&sahte_oauth.issuer_url()),
                    Some("yetki-kodu-2"),
                    None,
                )
                .await,
            Err(UzakOAuthHatasi::Protokol(
                OAuthHatasi::EskiVeyaTekrarlananCallback
            ))
        );

        // 8. Kimlik kaldırma kasadaki tüm OAuth tokenlarını siler ve revizyonu artırır.
        let son_rev = yonetici.kimlik_kaldir("uzak-oauth").unwrap();
        assert_eq!(son_rev, 4);
        assert_eq!(kasa.kayit_sayisi(), 0);

        sahte_oauth.kapat().await;
    }

    #[tokio::test]
    async fn m03_yanlis_issuer_ve_reddedilen_kapsam_reddedilir() {
        let gecici = tempfile::tempdir().unwrap();
        let ayarlar = Arc::new(Ayarlar::oku(gecici.path()));
        let kasa = Arc::new(SahteGizliKasa::yeni());
        let iletisim = Arc::new(SahteIletisim::default());
        iletisim.onay_ver(true);
        let acici = Arc::new(SahteAcici::default());
        let saat = SahteSaat::yeni();
        saat.ayarla(Duration::from_secs(1_000));

        let resource_url = "http://127.0.0.1:19444/mcp".to_owned();
        let sahte_oauth = SahteOAuthSunucusu::baslat(resource_url.clone(), None, 0).await;

        let mut islem = McpAyarlarIslemi::baslat(&*ayarlar);
        islem.degisiklik_ekle(McpDegisikligi::UzakKaydet {
            eski_ad: None,
            taslak: UzakMcpSunucuTaslagi {
                ad: "uzak-kapsam".into(),
                etkin: true,
                uc_url: resource_url.clone(),
                yetki: UzakMcpYetkiGirdisi::OAuth {
                    istemci_kimligi: Some("kalfa-istemci".into()),
                    kapsamlar: vec!["tools:read".into(), "tools:call".into()],
                    kaynak_url: Some(resource_url.clone()),
                    korunan_yetki_kimligi: None,
                },
                baslatma_zamani_asimi_sn: 10,
                cagri_zamani_asimi_sn: 30,
            },
            test_fiksturu_loopback: true,
        });
        islem.kaydet(&*ayarlar, &*kasa).unwrap();

        let yonetici = UzakOAuthYoneticisi::yeni_ozel(
            ayarlar.clone(),
            kasa.clone(),
            iletisim.clone(),
            acici.clone(),
            saat.clone(),
            Arc::new(SabitDnsCozucu::yeni()),
        );

        // Yanlış resource URL ile metadata doğrulaması YanlisResource döner.
        let yanlis_res_oauth =
            SahteOAuthSunucusu::baslat("http://127.0.0.1:19999/baska".into(), None, 0).await;
        assert_eq!(
            yonetici
                .oauth_akisi_baslat(
                    "uzak-kapsam",
                    Some("oturum-1".into()),
                    &yanlis_res_oauth.korunmus_kaynak_meta_url(),
                    &sahte_oauth.yetki_sunucusu_meta_url(),
                    "http://127.0.0.1:49152/callback",
                    true,
                    false,
                )
                .await,
            Err(UzakOAuthHatasi::Protokol(OAuthHatasi::YanlisResource))
        );
        yanlis_res_oauth.kapat().await;

        // Dar kapsam dönen token yanıtı KapsamReddedildi ile reddedilir ve kasaya yazılmaz.
        let baslatma = yonetici
            .oauth_akisi_baslat(
                "uzak-kapsam",
                Some("oturum-1".into()),
                &sahte_oauth.korunmus_kaynak_meta_url(),
                &sahte_oauth.yetki_sunucusu_meta_url(),
                "http://127.0.0.1:49152/callback",
                true,
                false,
            )
            .await
            .unwrap();
        assert_eq!(
            yonetici
                .oauth_geri_cagrim_tamamla(
                    "uzak-kapsam",
                    &baslatma.state,
                    Some("oturum-1"),
                    Some(&sahte_oauth.issuer_url()),
                    Some("dar-kapsam-kodu"),
                    None,
                )
                .await,
            Err(UzakOAuthHatasi::Protokol(OAuthHatasi::KapsamReddedildi))
        );
        assert_eq!(kasa.kayit_sayisi(), 0);

        sahte_oauth.kapat().await;
    }
}
