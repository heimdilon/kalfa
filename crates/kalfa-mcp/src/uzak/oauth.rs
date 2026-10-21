//! Uzak MCP OAuth 2.0 (PKCE S256, `state`/oturum bağlama, `issuer` ve `resource`
//! doğrulaması, sınırlı metadata/token HTTP istemcisi) protokol yardımcıları (`1.1b` Görev 4).

use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use futures_util::StreamExt;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;

use crate::tanim::uzak_uc_url_dogrula_ve_kanoniklestir;
use crate::tasima::TasimaHatasi;
use crate::uzak::http::{UzakDnsCozucu, dogrulanmis_istemci_ve_url_hazirla};

/// OAuth metadata ve token yanıtları için azami gövde boyutu (64 KiB).
pub const AZAMI_OAUTH_YANIT_BAYTI: usize = 64 * 1024;

/// OAuth yetkilendirme akışının azami geçerlilik süresi (saniye).
pub const VARSAYILAN_AKIS_OMRU_SN: u64 = 300;

/// Uzak MCP OAuth protokol ve doğrulama hataları.
/// Ham token, `code_verifier` veya gizli başlık değeri bu tipte taşınmaz.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OAuthHatasi {
    AgPolitikasiReddi,
    MetadataGecersiz(&'static str),
    YanlisResource,
    YanlisIssuer,
    PkceS256Desteklenmiyor,
    YanlisState,
    YanlisOturum,
    EskiVeyaTekrarlananCallback,
    SuresiDolmusAkis,
    KapsamReddedildi,
    EksikYetkiKodu,
    TokenDegisimiBasarisiz(u16),
    TokenBicimiGecersiz(&'static str),
    BoyutSiniriAsildi,
    IptalEdildi,
    BaglantiHatasi,
}

impl fmt::Display for OAuthHatasi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AgPolitikasiReddi => f.write_str("OAuth uç adresi ağ politikasını ihlal etti"),
            Self::MetadataGecersiz(neden) => write!(f, "OAuth metadata geçersiz: {neden}"),
            Self::YanlisResource => {
                f.write_str("OAuth korunmuş kaynak (resource) hedef MCP ucuyla eşleşmiyor")
            }
            Self::YanlisIssuer => {
                f.write_str("OAuth yetki sunucusu (issuer) beklenen kimlikle eşleşmiyor")
            }
            Self::PkceS256Desteklenmiyor => {
                f.write_str("OAuth yetki sunucusu PKCE S256 yöntemini desteklemiyor")
            }
            Self::YanlisState => f.write_str("OAuth state değeri beklenen akışla eşleşmiyor"),
            Self::YanlisOturum => {
                f.write_str("OAuth geri çağrımı akışı başlatan oturumla eşleşmiyor")
            }
            Self::EskiVeyaTekrarlananCallback => {
                f.write_str("OAuth geri çağrımı zaten kullanılmış veya geçersiz")
            }
            Self::SuresiDolmusAkis => f.write_str("OAuth yetkilendirme akışının süresi doldu"),
            Self::KapsamReddedildi => {
                f.write_str("OAuth yetkilendirmesi veya istenen kapsamlar reddedildi")
            }
            Self::EksikYetkiKodu => f.write_str("OAuth geri çağrımında yetkilendirme kodu yok"),
            Self::TokenDegisimiBasarisiz(kod) => {
                write!(f, "OAuth token değişimi başarısız oldu (HTTP {kod})")
            }
            Self::TokenBicimiGecersiz(neden) => {
                write!(f, "OAuth token yanıtı geçersiz: {neden}")
            }
            Self::BoyutSiniriAsildi => f.write_str("OAuth yanıtı 64 KiB boyut sınırını aştı"),
            Self::IptalEdildi => f.write_str("OAuth yetkilendirme akışı iptal edildi"),
            Self::BaglantiHatasi => f.write_str("OAuth sunucusuyla bağlantı kurulamadı"),
        }
    }
}

impl std::error::Error for OAuthHatasi {}

impl From<TasimaHatasi> for OAuthHatasi {
    fn from(hata: TasimaHatasi) -> Self {
        match hata {
            TasimaHatasi::AgPolitikasiReddi => Self::AgPolitikasiReddi,
            TasimaHatasi::Cerceve => Self::BoyutSiniriAsildi,
            _ => Self::BaglantiHatasi,
        }
    }
}

/// RFC 7636 PKCE (`S256`) doğrulayıcı ve meydan okuma çifti.
/// `Debug` çıktısı `code_verifier` sırrını göstermez.
#[derive(Clone, PartialEq, Eq)]
pub struct PkceCifti {
    code_verifier: String,
    code_challenge: String,
}

impl fmt::Debug for PkceCifti {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PkceCifti")
            .field("code_verifier", &"<gizli>")
            .field("code_challenge", &self.code_challenge)
            .finish()
    }
}

impl PkceCifti {
    /// 256 bit CSPRNG entropisinden RFC 7636 uyumlu (43 karakter URL-safe Base64)
    /// `code_verifier` ve `S256` `code_challenge` üretir.
    pub fn yeni_rastgele() -> Self {
        let mut ham = [0u8; 32];
        ham[..16].copy_from_slice(Uuid::new_v4().as_bytes());
        ham[16..].copy_from_slice(Uuid::new_v4().as_bytes());
        let code_verifier = URL_SAFE_NO_PAD.encode(ham);
        let ozet = Sha256::digest(code_verifier.as_bytes());
        let code_challenge = URL_SAFE_NO_PAD.encode(ozet);
        Self {
            code_verifier,
            code_challenge,
        }
    }

    /// Verilen doğrulayıcı dizgesini RFC 7636 kurallarına (`43..=128` karakter,
    /// `[A-Za-z0-9-._~]`) göre doğrular ve `S256` özetini hesaplar.
    pub fn dogrulayicidan(code_verifier: impl Into<String>) -> Result<Self, OAuthHatasi> {
        let code_verifier = code_verifier.into();
        if !(43..=128).contains(&code_verifier.len())
            || !code_verifier
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~'))
        {
            return Err(OAuthHatasi::MetadataGecersiz("geçersiz PKCE code_verifier"));
        }
        let ozet = Sha256::digest(code_verifier.as_bytes());
        let code_challenge = URL_SAFE_NO_PAD.encode(ozet);
        Ok(Self {
            code_verifier,
            code_challenge,
        })
    }

    pub fn code_verifier(&self) -> &str {
        &self.code_verifier
    }

    pub fn code_challenge(&self) -> &str {
        &self.code_challenge
    }

    pub const fn code_challenge_method(&self) -> &'static str {
        "S256"
    }
}

/// 256 bit CSPRNG entropisinden tek kullanımlık OAuth `state` değeri üretir.
pub fn rastgele_oauth_state_uret() -> String {
    let mut ham = [0u8; 32];
    ham[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    ham[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    URL_SAFE_NO_PAD.encode(ham)
}

/// RFC 9728 OAuth 2.0 Protected Resource Metadata (`/.well-known/oauth-protected-resource`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthKorunmusKaynakMetadata {
    pub resource: String,
    pub authorization_servers: Vec<String>,
    #[serde(default)]
    pub scopes_supported: Vec<String>,
}

impl OAuthKorunmusKaynakMetadata {
    pub fn dogrula_ve_kanoniklestir(
        ham: &Value,
        beklenen_resource_url: &str,
        test_fiksturu_loopback: bool,
    ) -> Result<Self, OAuthHatasi> {
        let beklenen_kanonik =
            uzak_uc_url_dogrula_ve_kanoniklestir(beklenen_resource_url, test_fiksturu_loopback)
                .map_err(|_| OAuthHatasi::YanlisResource)?;

        let resource_ham = ham
            .get("resource")
            .and_then(Value::as_str)
            .ok_or(OAuthHatasi::MetadataGecersiz("resource alanı eksik"))?;
        let resource_kanonik =
            uzak_uc_url_dogrula_ve_kanoniklestir(resource_ham, test_fiksturu_loopback)
                .map_err(|_| OAuthHatasi::YanlisResource)?;

        if resource_kanonik != beklenen_kanonik {
            return Err(OAuthHatasi::YanlisResource);
        }

        let sunucular_ham = ham
            .get("authorization_servers")
            .and_then(Value::as_array)
            .ok_or(OAuthHatasi::MetadataGecersiz(
                "authorization_servers alanı eksik",
            ))?;
        if sunucular_ham.is_empty() {
            return Err(OAuthHatasi::MetadataGecersiz(
                "authorization_servers boş olamaz",
            ));
        }

        let mut authorization_servers = Vec::with_capacity(sunucular_ham.len());
        for s in sunucular_ham {
            let s_metin = s.as_str().ok_or(OAuthHatasi::MetadataGecersiz(
                "authorization_servers öğesi dizge olmalı",
            ))?;
            let kanonik = uzak_uc_url_dogrula_ve_kanoniklestir(s_metin, test_fiksturu_loopback)
                .map_err(|_| OAuthHatasi::AgPolitikasiReddi)?;
            authorization_servers.push(kanonik);
        }

        let scopes_supported = ham
            .get("scopes_supported")
            .and_then(Value::as_array)
            .map(|dizi| {
                dizi.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            resource: resource_kanonik,
            authorization_servers,
            scopes_supported,
        })
    }
}

/// RFC 8414 OAuth 2.0 Authorization Server Metadata (`/.well-known/oauth-authorization-server`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthYetkiSunucusuMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub code_challenge_methods_supported: Vec<String>,
}

impl OAuthYetkiSunucusuMetadata {
    pub fn dogrula_ve_kanoniklestir(
        ham: &Value,
        beklenen_issuer: &str,
        test_fiksturu_loopback: bool,
    ) -> Result<Self, OAuthHatasi> {
        let beklenen_issuer_kanonik =
            uzak_uc_url_dogrula_ve_kanoniklestir(beklenen_issuer, test_fiksturu_loopback)
                .map_err(|_| OAuthHatasi::YanlisIssuer)?;

        let issuer_ham = ham
            .get("issuer")
            .and_then(Value::as_str)
            .ok_or(OAuthHatasi::MetadataGecersiz("issuer alanı eksik"))?;
        let issuer_kanonik =
            uzak_uc_url_dogrula_ve_kanoniklestir(issuer_ham, test_fiksturu_loopback)
                .map_err(|_| OAuthHatasi::YanlisIssuer)?;

        if issuer_kanonik != beklenen_issuer_kanonik {
            return Err(OAuthHatasi::YanlisIssuer);
        }

        let auth_ham = ham
            .get("authorization_endpoint")
            .and_then(Value::as_str)
            .ok_or(OAuthHatasi::MetadataGecersiz(
                "authorization_endpoint alanı eksik",
            ))?;
        let authorization_endpoint =
            uzak_uc_url_dogrula_ve_kanoniklestir(auth_ham, test_fiksturu_loopback)
                .map_err(|_| OAuthHatasi::AgPolitikasiReddi)?;

        let token_ham = ham
            .get("token_endpoint")
            .and_then(Value::as_str)
            .ok_or(OAuthHatasi::MetadataGecersiz("token_endpoint alanı eksik"))?;
        let token_endpoint =
            uzak_uc_url_dogrula_ve_kanoniklestir(token_ham, test_fiksturu_loopback)
                .map_err(|_| OAuthHatasi::AgPolitikasiReddi)?;

        let yontemler: Vec<String> = ham
            .get("code_challenge_methods_supported")
            .and_then(Value::as_array)
            .map(|dizi| {
                dizi.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();

        if !yontemler.iter().any(|y| y == "S256") {
            return Err(OAuthHatasi::PkceS256Desteklenmiyor);
        }

        Ok(Self {
            issuer: issuer_kanonik,
            authorization_endpoint,
            token_endpoint,
            code_challenge_methods_supported: yontemler,
        })
    }
}

/// Başlatılmış bir OAuth yetkilendirme akışının oturum/sunucu/hedef bağlamı.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OAuthDurumBaglami {
    pub state: String,
    pub pkce: PkceCifti,
    pub sunucu_adi: String,
    pub oturum_kimligi: Option<String>,
    pub beklenen_issuer: String,
    pub beklenen_resource: String,
    pub redirect_uri: String,
    pub istenen_kapsamlar: Vec<String>,
    pub olusturulma_ani_sn: u64,
    pub kullanildi: bool,
}

impl OAuthDurumBaglami {
    #[allow(clippy::too_many_arguments)]
    pub fn yeni(
        sunucu_adi: impl Into<String>,
        oturum_kimligi: Option<String>,
        beklenen_issuer: impl Into<String>,
        beklenen_resource: impl Into<String>,
        redirect_uri: impl Into<String>,
        istenen_kapsamlar: Vec<String>,
        olusturulma_ani_sn: u64,
    ) -> Self {
        Self {
            state: rastgele_oauth_state_uret(),
            pkce: PkceCifti::yeni_rastgele(),
            sunucu_adi: sunucu_adi.into(),
            oturum_kimligi,
            beklenen_issuer: beklenen_issuer.into(),
            beklenen_resource: beklenen_resource.into(),
            redirect_uri: redirect_uri.into(),
            istenen_kapsamlar,
            olusturulma_ani_sn,
            kullanildi: false,
        }
    }
}

/// OAuth geri çağrım (callback) parametreleri.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OAuthGeriCagrimGirdisi<'a> {
    pub gelen_state: &'a str,
    pub gelen_oturum: Option<&'a str>,
    pub gelen_issuer: Option<&'a str>,
    pub gelen_kod: Option<&'a str>,
    pub gelen_hata: Option<&'a str>,
    pub simdi_sn: u64,
    pub azami_yas_sn: u64,
    pub test_fiksturu_loopback: bool,
}

/// Geri çağrımı state, oturum, süre, tek-kullanım ve issuer bağlamlarına göre doğrular.
pub fn geri_cagrim_dogrula(
    akis: &mut OAuthDurumBaglami,
    girdi: &OAuthGeriCagrimGirdisi<'_>,
) -> Result<String, OAuthHatasi> {
    if akis.kullanildi {
        return Err(OAuthHatasi::EskiVeyaTekrarlananCallback);
    }
    if girdi.simdi_sn.saturating_sub(akis.olusturulma_ani_sn) > girdi.azami_yas_sn {
        akis.kullanildi = true;
        return Err(OAuthHatasi::SuresiDolmusAkis);
    }
    if akis.oturum_kimligi.as_deref() != girdi.gelen_oturum {
        return Err(OAuthHatasi::YanlisOturum);
    }
    if akis.state != girdi.gelen_state {
        return Err(OAuthHatasi::YanlisState);
    }
    if let Some(gelen_iss) = girdi.gelen_issuer {
        let kanonik_iss =
            uzak_uc_url_dogrula_ve_kanoniklestir(gelen_iss, girdi.test_fiksturu_loopback)
                .map_err(|_| OAuthHatasi::YanlisIssuer)?;
        if kanonik_iss != akis.beklenen_issuer {
            akis.kullanildi = true;
            return Err(OAuthHatasi::YanlisIssuer);
        }
    }
    // Tekrar oynatmayı önlemek için state eşleştikten sonra akış kullanıldı işaretlenir.
    akis.kullanildi = true;

    if let Some(hata) = girdi.gelen_hata
        && !hata.trim().is_empty()
    {
        return Err(OAuthHatasi::KapsamReddedildi);
    }

    let kod = girdi
        .gelen_kod
        .map(str::trim)
        .filter(|k| !k.is_empty() && !k.contains('\0'))
        .ok_or(OAuthHatasi::EksikYetkiKodu)?;

    Ok(kod.to_owned())
}

/// Tarayıcıda açılacak OAuth yetkilendirme adresini oluşturur.
pub fn yetkilendirme_urlsi_olustur(
    yetki_sunucusu: &OAuthYetkiSunucusuMetadata,
    client_id: &str,
    akis: &OAuthDurumBaglami,
) -> Result<Url, OAuthHatasi> {
    let mut url = Url::parse(&yetki_sunucusu.authorization_endpoint)
        .map_err(|_| OAuthHatasi::MetadataGecersiz("geçersiz authorization_endpoint"))?;
    {
        let mut sorgu = url.query_pairs_mut();
        sorgu.append_pair("response_type", "code");
        sorgu.append_pair("client_id", client_id);
        sorgu.append_pair("redirect_uri", &akis.redirect_uri);
        sorgu.append_pair("state", &akis.state);
        sorgu.append_pair("code_challenge", akis.pkce.code_challenge());
        sorgu.append_pair("code_challenge_method", akis.pkce.code_challenge_method());
        sorgu.append_pair("resource", &akis.beklenen_resource);
        if !akis.istenen_kapsamlar.is_empty() {
            sorgu.append_pair("scope", &akis.istenen_kapsamlar.join(" "));
        }
    }
    Ok(url)
}

/// OAuth token uç noktasından dönen doğrulanmış token paketi.
/// `Debug` çıktısı ham `access_token` ve `refresh_token` değerlerini gizler.
#[derive(Clone, PartialEq, Eq)]
pub struct OAuthTokenPaketi {
    access_token: String,
    refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub onaylanan_kapsamlar: Vec<String>,
}

impl fmt::Debug for OAuthTokenPaketi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OAuthTokenPaketi")
            .field("access_token", &"<gizli>")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "<gizli>"),
            )
            .field("expires_in", &self.expires_in)
            .field("onaylanan_kapsamlar", &self.onaylanan_kapsamlar)
            .finish()
    }
}

impl OAuthTokenPaketi {
    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    pub fn refresh_token(&self) -> Option<&str> {
        self.refresh_token.as_deref()
    }

    pub fn json_dogrula(
        ham: &Value,
        istenen_kapsamlar: &[String],
        beklenen_resource: &str,
        test_fiksturu_loopback: bool,
    ) -> Result<Self, OAuthHatasi> {
        if let Some(hata) = ham.get("error").and_then(Value::as_str) {
            if hata == "invalid_scope" || hata == "access_denied" {
                return Err(OAuthHatasi::KapsamReddedildi);
            }
            return Err(OAuthHatasi::TokenBicimiGecersiz(
                "sunucu OAuth hatası döndü",
            ));
        }

        let token_type = ham
            .get("token_type")
            .and_then(Value::as_str)
            .ok_or(OAuthHatasi::TokenBicimiGecersiz("token_type eksik"))?;
        if !token_type.eq_ignore_ascii_case("bearer") {
            return Err(OAuthHatasi::TokenBicimiGecersiz(
                "yalnız Bearer token desteklenir",
            ));
        }

        if let Some(res) = ham.get("resource").and_then(Value::as_str) {
            let kanonik = uzak_uc_url_dogrula_ve_kanoniklestir(res, test_fiksturu_loopback)
                .map_err(|_| OAuthHatasi::YanlisResource)?;
            if kanonik != beklenen_resource {
                return Err(OAuthHatasi::YanlisResource);
            }
        }

        let access_token = ham
            .get("access_token")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| s.chars().count() >= 8 && !s.contains('\0'))
            .ok_or(OAuthHatasi::TokenBicimiGecersiz("geçersiz access_token"))?
            .to_owned();

        let refresh_token = match ham.get("refresh_token").and_then(Value::as_str) {
            Some(rt) => {
                let temiz = rt.trim();
                if temiz.chars().count() < 8 || temiz.contains('\0') {
                    return Err(OAuthHatasi::TokenBicimiGecersiz("geçersiz refresh_token"));
                }
                Some(temiz.to_owned())
            }
            None => None,
        };

        let expires_in = ham.get("expires_in").and_then(Value::as_u64);

        let onaylanan_kapsamlar: Vec<String> = match ham.get("scope").and_then(Value::as_str) {
            Some(kapsam_metni) => {
                let kume: BTreeSet<String> = kapsam_metni
                    .split_whitespace()
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect();
                for istenen in istenen_kapsamlar {
                    if !kume.contains(istenen) {
                        return Err(OAuthHatasi::KapsamReddedildi);
                    }
                }
                kume.into_iter().collect()
            }
            None => istenen_kapsamlar.to_vec(),
        };

        Ok(Self {
            access_token,
            refresh_token,
            expires_in,
            onaylanan_kapsamlar,
        })
    }
}

async fn sinirli_json_oku(yanit: reqwest::Response) -> Result<Value, OAuthHatasi> {
    let mut akis = yanit.bytes_stream();
    let mut tampon = Vec::new();
    while let Some(parca) = akis.next().await {
        let baytlar = parca.map_err(|_| OAuthHatasi::BaglantiHatasi)?;
        if tampon
            .len()
            .checked_add(baytlar.len())
            .is_none_or(|toplam| toplam > AZAMI_OAUTH_YANIT_BAYTI)
        {
            return Err(OAuthHatasi::BoyutSiniriAsildi);
        }
        tampon.extend_from_slice(&baytlar);
    }
    serde_json::from_slice(&tampon).map_err(|_| OAuthHatasi::MetadataGecersiz("bozuk JSON yanıtı"))
}

/// Korunmuş kaynak metadatasını ağ politikası ve boyut sınırlarıyla indirip doğrular.
pub async fn korunmus_kaynak_metadatasini_getir(
    metadata_url: &str,
    beklenen_resource_url: &str,
    test_fiksturu_loopback: bool,
    ozel_ag_izinli: bool,
    zaman_asimi: Duration,
    dns_cozucu: &Arc<dyn UzakDnsCozucu>,
) -> Result<OAuthKorunmusKaynakMetadata, OAuthHatasi> {
    let (istemci, url, _) = dogrulanmis_istemci_ve_url_hazirla(
        metadata_url,
        test_fiksturu_loopback,
        ozel_ag_izinli,
        zaman_asimi,
        dns_cozucu.as_ref(),
    )
    .await?;

    let yanit = istemci
        .get(url)
        .header(ACCEPT, HeaderValue::from_static("application/json"))
        .send()
        .await
        .map_err(|_| OAuthHatasi::BaglantiHatasi)?;

    if !yanit.status().is_success() {
        return Err(OAuthHatasi::MetadataGecersiz(
            "korunmuş kaynak metadata HTTP hatası",
        ));
    }

    let json = sinirli_json_oku(yanit).await?;
    OAuthKorunmusKaynakMetadata::dogrula_ve_kanoniklestir(
        &json,
        beklenen_resource_url,
        test_fiksturu_loopback,
    )
}

/// Yetki sunucusu metadatasını ağ politikası ve boyut sınırlarıyla indirip doğrular.
pub async fn yetki_sunucusu_metadatasini_getir(
    metadata_url: &str,
    beklenen_issuer: &str,
    test_fiksturu_loopback: bool,
    ozel_ag_izinli: bool,
    zaman_asimi: Duration,
    dns_cozucu: &Arc<dyn UzakDnsCozucu>,
) -> Result<OAuthYetkiSunucusuMetadata, OAuthHatasi> {
    let (istemci, url, _) = dogrulanmis_istemci_ve_url_hazirla(
        metadata_url,
        test_fiksturu_loopback,
        ozel_ag_izinli,
        zaman_asimi,
        dns_cozucu.as_ref(),
    )
    .await?;

    let yanit = istemci
        .get(url)
        .header(ACCEPT, HeaderValue::from_static("application/json"))
        .send()
        .await
        .map_err(|_| OAuthHatasi::BaglantiHatasi)?;

    if !yanit.status().is_success() {
        return Err(OAuthHatasi::MetadataGecersiz(
            "yetki sunucusu metadata HTTP hatası",
        ));
    }

    let json = sinirli_json_oku(yanit).await?;
    OAuthYetkiSunucusuMetadata::dogrula_ve_kanoniklestir(
        &json,
        beklenen_issuer,
        test_fiksturu_loopback,
    )
}

fn form_url_kodla(ciftler: &[(&str, &str)]) -> String {
    let mut s = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in ciftler {
        s.append_pair(k, v);
    }
    s.finish()
}

/// Yetkilendirme kodunu (`authorization_code`) PKCE `code_verifier` ve `resource`
/// bağlamıyla token uç noktasında takas eder.
#[allow(clippy::too_many_arguments)]
pub async fn kod_ile_token_al(
    token_endpoint: &str,
    client_id: &str,
    yetki_kodu: &str,
    akis: &OAuthDurumBaglami,
    test_fiksturu_loopback: bool,
    ozel_ag_izinli: bool,
    zaman_asimi: Duration,
    dns_cozucu: &Arc<dyn UzakDnsCozucu>,
) -> Result<OAuthTokenPaketi, OAuthHatasi> {
    let (istemci, url, _) = dogrulanmis_istemci_ve_url_hazirla(
        token_endpoint,
        test_fiksturu_loopback,
        ozel_ag_izinli,
        zaman_asimi,
        dns_cozucu.as_ref(),
    )
    .await?;

    let govde = form_url_kodla(&[
        ("grant_type", "authorization_code"),
        ("client_id", client_id),
        ("code", yetki_kodu),
        ("redirect_uri", &akis.redirect_uri),
        ("code_verifier", akis.pkce.code_verifier()),
        ("resource", &akis.beklenen_resource),
    ]);

    let mut basliklar = HeaderMap::new();
    basliklar.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    basliklar.insert(ACCEPT, HeaderValue::from_static("application/json"));

    let yanit = istemci
        .post(url)
        .headers(basliklar)
        .body(govde)
        .send()
        .await
        .map_err(|_| OAuthHatasi::BaglantiHatasi)?;

    let durum = yanit.status();
    if !durum.is_success() {
        if let Ok(json) = sinirli_json_oku(yanit).await
            && let Some(err) = json.get("error").and_then(Value::as_str)
            && (err == "invalid_scope" || err == "access_denied")
        {
            return Err(OAuthHatasi::KapsamReddedildi);
        }
        return Err(OAuthHatasi::TokenDegisimiBasarisiz(durum.as_u16()));
    }

    let json = sinirli_json_oku(yanit).await?;
    OAuthTokenPaketi::json_dogrula(
        &json,
        &akis.istenen_kapsamlar,
        &akis.beklenen_resource,
        test_fiksturu_loopback,
    )
}

/// Yenileme tokenını (`refresh_token`) `resource` bağlamıyla token uç noktasında yeniler.
#[allow(clippy::too_many_arguments)]
pub async fn yenileme_tokeni_ile_token_al(
    token_endpoint: &str,
    client_id: &str,
    refresh_token: &str,
    beklenen_resource: &str,
    istenen_kapsamlar: &[String],
    test_fiksturu_loopback: bool,
    ozel_ag_izinli: bool,
    zaman_asimi: Duration,
    dns_cozucu: &Arc<dyn UzakDnsCozucu>,
) -> Result<OAuthTokenPaketi, OAuthHatasi> {
    let (istemci, url, _) = dogrulanmis_istemci_ve_url_hazirla(
        token_endpoint,
        test_fiksturu_loopback,
        ozel_ag_izinli,
        zaman_asimi,
        dns_cozucu.as_ref(),
    )
    .await?;

    let kapsam_metni = istenen_kapsamlar.join(" ");
    let mut ciftler = vec![
        ("grant_type", "refresh_token"),
        ("client_id", client_id),
        ("refresh_token", refresh_token),
        ("resource", beklenen_resource),
    ];
    if !kapsam_metni.is_empty() {
        ciftler.push(("scope", &kapsam_metni));
    }
    let govde = form_url_kodla(&ciftler);

    let mut basliklar = HeaderMap::new();
    basliklar.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    basliklar.insert(ACCEPT, HeaderValue::from_static("application/json"));

    let yanit = istemci
        .post(url)
        .headers(basliklar)
        .body(govde)
        .send()
        .await
        .map_err(|_| OAuthHatasi::BaglantiHatasi)?;

    let durum = yanit.status();
    if !durum.is_success() {
        if let Ok(json) = sinirli_json_oku(yanit).await
            && let Some(err) = json.get("error").and_then(Value::as_str)
            && (err == "invalid_scope" || err == "access_denied")
        {
            return Err(OAuthHatasi::KapsamReddedildi);
        }
        return Err(OAuthHatasi::TokenDegisimiBasarisiz(durum.as_u16()));
    }

    let json = sinirli_json_oku(yanit).await?;
    OAuthTokenPaketi::json_dogrula(
        &json,
        istenen_kapsamlar,
        beklenen_resource,
        test_fiksturu_loopback,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pkce_s256_ve_geri_cagrim_state_oturum_issuer_ve_tekrar_sinirlarini_dogrular() {
        let pkce = PkceCifti::yeni_rastgele();
        assert_eq!(pkce.code_verifier().len(), 43);
        assert_eq!(pkce.code_challenge().len(), 43);
        assert!(!format!("{pkce:?}").contains(pkce.code_verifier()));

        let mut akis = OAuthDurumBaglami::yeni(
            "uzak-1",
            Some("oturum-a".into()),
            "https://auth.example.com/",
            "https://mcp.example.com/v1",
            "http://127.0.0.1:49152/callback",
            vec!["tools:read".into()],
            1000,
        );
        let dogru_state = akis.state.clone();

        // Yanlış oturum reddedilir ve akışı tüketmez.
        assert_eq!(
            geri_cagrim_dogrula(
                &mut akis,
                &OAuthGeriCagrimGirdisi {
                    gelen_state: &dogru_state,
                    gelen_oturum: Some("oturum-b"),
                    gelen_issuer: Some("https://auth.example.com"),
                    gelen_kod: Some("kod-123"),
                    gelen_hata: None,
                    simdi_sn: 1010,
                    azami_yas_sn: 300,
                    test_fiksturu_loopback: false,
                }
            ),
            Err(OAuthHatasi::YanlisOturum)
        );

        // Yanlış state reddedilir.
        assert_eq!(
            geri_cagrim_dogrula(
                &mut akis,
                &OAuthGeriCagrimGirdisi {
                    gelen_state: "yanlis-state",
                    gelen_oturum: Some("oturum-a"),
                    gelen_issuer: Some("https://auth.example.com"),
                    gelen_kod: Some("kod-123"),
                    gelen_hata: None,
                    simdi_sn: 1010,
                    azami_yas_sn: 300,
                    test_fiksturu_loopback: false,
                }
            ),
            Err(OAuthHatasi::YanlisState)
        );

        // Doğru geri çağrım kodu döner.
        let kod = geri_cagrim_dogrula(
            &mut akis,
            &OAuthGeriCagrimGirdisi {
                gelen_state: &dogru_state,
                gelen_oturum: Some("oturum-a"),
                gelen_issuer: Some("https://auth.example.com"),
                gelen_kod: Some("kod-123"),
                gelen_hata: None,
                simdi_sn: 1010,
                azami_yas_sn: 300,
                test_fiksturu_loopback: false,
            },
        )
        .unwrap();
        assert_eq!(kod, "kod-123");

        // Aynı callback tekrar gönderilirse EskiVeyaTekrarlananCallback döner.
        assert_eq!(
            geri_cagrim_dogrula(
                &mut akis,
                &OAuthGeriCagrimGirdisi {
                    gelen_state: &dogru_state,
                    gelen_oturum: Some("oturum-a"),
                    gelen_issuer: Some("https://auth.example.com"),
                    gelen_kod: Some("kod-123"),
                    gelen_hata: None,
                    simdi_sn: 1011,
                    azami_yas_sn: 300,
                    test_fiksturu_loopback: false,
                }
            ),
            Err(OAuthHatasi::EskiVeyaTekrarlananCallback)
        );
    }

    #[test]
    fn token_paketi_kapsam_ve_resource_uyumsuzlugunu_reddeder_ve_debugda_maskeler() {
        let gecerli = json!({
            "access_token": "erisim-gizli-token-123456",
            "refresh_token": "yenileme-gizli-token-654321",
            "token_type": "Bearer",
            "expires_in": 3600,
            "scope": "tools:read tools:call",
            "resource": "https://mcp.example.com/v1"
        });
        let paket = OAuthTokenPaketi::json_dogrula(
            &gecerli,
            &["tools:read".into()],
            "https://mcp.example.com/v1",
            false,
        )
        .unwrap();
        let debug_metni = format!("{paket:?}");
        assert!(!debug_metni.contains("erisim-gizli-token-123456"));
        assert!(!debug_metni.contains("yenileme-gizli-token-654321"));

        // İstenen kapsam verilmemişse KapsamReddedildi.
        let eksik_kapsam = OAuthTokenPaketi::json_dogrula(
            &gecerli,
            &["tools:read".into(), "tools:admin".into()],
            "https://mcp.example.com/v1",
            false,
        );
        assert_eq!(eksik_kapsam, Err(OAuthHatasi::KapsamReddedildi));
    }
}
