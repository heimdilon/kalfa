//! Uygulama katmanı MCP politika özeti (`Π`), koruma doğrulaması (`K`) ve
//! son denetim (`.git` / yapılandırma karantinası) sarmalayıcısı.
//!
//! Tasarım §4.3 ve §5.3 uyarınca:
//! - `Π` (`McpPolitikaOzeti`): yalıtım arka ucu/sürümü, gizli kalıplar, gizli
//!   sayılmayanlar ve okunur dizinlerin canonical SHA-256 özetidir. Daralma
//!   veya genişleme fark etmeksizin her değişiklik `Π`'yi değiştirir.
//! - `K` (`McpKorumaOzeti`): `kalfa_tools::koruma::koruma_dogrula` üzerinden
//!   yalnız `koruma_tam == true` olduğunda üretilir. Görüntü sınırı
//!   (`goruntu.eksik == true`) tek başına `Doğrulanamadı` sayılmaz.
//! - Son denetim (`komut_sonrasi_denetim`): çağrı öncesinde, çağrı sonrasında
//!   ve süreç kapanışında yeni `.git` ve otomatik çalışan yapılandırma
//!   dosyalarını kabuktaki kuralla karantinaya alır.

use std::fmt::Write as _;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use kalfa_core::arac::{McpKorumaOzeti, McpPolitikaOzeti, McpProfilOzeti, McpYalitimProfiliOzeti};
use kalfa_core::izin::{GizliAyarlari, Siniflandirici};
use kalfa_mcp::tarif::{politika_ozeti, profil_ozeti, yalitim_profili_ozeti};
use kalfa_sandbox::tarama::{CancellationToken, TaramaSecenekleri};
use kalfa_sandbox::{AlanBilgisi, CalistirmaKipi, Yalitici, YalitimDurumu};
use kalfa_tools::CalismaAlani;
use kalfa_tools::koruma::{
    AdliGirdiler, KorumaDogrulamaHatasi, KorumaSonucu, SonDenetimSonucu,
    komut_sonrasi_denetim_secenekli, koruma_dogrula,
};

use crate::mcp::surec::SaatKaynagi;

/// 32 baytlık SHA-256 özetini küçük harfli hex dizgesine çevirir.
pub fn ozet_hex(baytlar: &[u8; 32]) -> String {
    let mut cikti = String::with_capacity(64);
    for b in baytlar {
        let _ = write!(&mut cikti, "{b:02x}");
    }
    cikti
}

/// Politika özeti `Π` (§5.3):
/// - yalıtım arka ucu ve sürümü,
/// - ek gizli kalıplar ve gizli sayılmayanlar,
/// - okunur araç zinciri dizinleri.
///
/// Daralma ya da genişleme fark etmeksizin her değişiklik `Π`'yi değiştirir.
pub fn politika_ozeti_hesapla(
    yalitim_durumu: &YalitimDurumu,
    gizli_ayarlari: &GizliAyarlari,
    okunur_dizinler: &[PathBuf],
) -> McpPolitikaOzeti {
    politika_ozeti(yalitim_durumu, gizli_ayarlari, okunur_dizinler)
}

/// Yalıtım profili özeti (`McpYalitimProfiliOzeti`) üretir.
pub fn yalitim_profili_ozeti_hesapla(
    yalitici: &dyn Yalitici,
    kip: CalistirmaKipi,
    alan_bilgisi: &AlanBilgisi,
    ag_ozeti: Option<&str>,
) -> McpYalitimProfiliOzeti {
    yalitim_profili_ozeti(yalitici, kip, alan_bilgisi, ag_ozeti)
}

/// `Π`, `K` ve yalıtım profili özetinden canonical `P` (`McpProfilOzeti`) üretir.
pub fn tam_profil_ozeti(
    politika: McpPolitikaOzeti,
    koruma: McpKorumaOzeti,
    yalitim: McpYalitimProfiliOzeti,
) -> McpProfilOzeti {
    profil_ozeti(politika, koruma, yalitim)
}

/// Koruma doğrulama isteği.
#[derive(Clone)]
pub struct KorumaDenetimIstegi {
    pub kok: PathBuf,
    pub gizli_ayarlari: Arc<GizliAyarlari>,
    pub okunur_dizinler: Vec<PathBuf>,
    pub ag_ozeti: Option<String>,
    pub yalitimli: bool,
    pub alan_hazirla: bool,
    pub onceki_ozet: Option<McpKorumaOzeti>,
    pub iptal: Option<CancellationToken>,
}

/// Son denetim (`.git` ve otomatik yapılandırma karantinası) isteği.
#[derive(Clone)]
pub struct SonDenetimIstegi {
    pub kok: PathBuf,
    pub gizli_ayarlari: Arc<GizliAyarlari>,
    pub onceki: AdliGirdiler,
}

/// MCP yöneticisinin koruma doğrulama ve son denetim arayüzü.
pub trait KorumaDenetleyicisi: Send + Sync {
    fn dogrula<'a>(
        &'a self,
        yalitici: &'a Arc<dyn Yalitici>,
        istek: KorumaDenetimIstegi,
    ) -> Pin<Box<dyn Future<Output = KorumaSonucu> + Send + 'a>>;

    fn son_denetim<'a>(
        &'a self,
        istek: SonDenetimIstegi,
    ) -> Pin<Box<dyn Future<Output = SonDenetimSonucu> + Send + 'a>>;
}

/// Gerçek dosya sistemi ve `kalfa_tools::koruma` kullanan varsayılan denetleyici.
#[derive(Default)]
pub struct GercekKorumaDenetleyicisi {
    tarama_saati: Option<Arc<dyn SaatKaynagi>>,
}

impl GercekKorumaDenetleyicisi {
    pub fn yeni() -> Self {
        Self { tarama_saati: None }
    }

    pub fn saatli(saat: Arc<dyn SaatKaynagi>) -> Self {
        Self {
            tarama_saati: Some(saat),
        }
    }

    fn secenekler(&self, iptal: Option<CancellationToken>) -> TaramaSecenekleri {
        let mut secenekler = TaramaSecenekleri::default();
        if let Some(iptal) = iptal {
            secenekler = secenekler.iptal_ile(iptal);
        }
        if let Some(saat) = self.tarama_saati.clone() {
            let baslangic = saat.simdi();
            secenekler =
                secenekler.sahte_gecen_sure_ile(move || saat.simdi().saturating_sub(baslangic));
        }
        secenekler
    }
}

impl KorumaDenetleyicisi for GercekKorumaDenetleyicisi {
    fn dogrula<'a>(
        &'a self,
        yalitici: &'a Arc<dyn Yalitici>,
        istek: KorumaDenetimIstegi,
    ) -> Pin<Box<dyn Future<Output = KorumaSonucu> + Send + 'a>> {
        let secenekler = self.secenekler(istek.iptal.clone());
        Box::pin(async move {
            let siniflandirici =
                match Siniflandirici::yeni_ayarla(&istek.kok, istek.gizli_ayarlari.clone()) {
                    Ok(s) => Arc::new(s),
                    Err(hata) => {
                        return KorumaSonucu::Dogrulanamadi(KorumaDogrulamaHatasi::AlanCozulemedi(
                            hata.to_string(),
                        ));
                    }
                };
            let alan = match CalismaAlani::ac(siniflandirici) {
                Ok(a) => a,
                Err(hata) => {
                    return KorumaSonucu::Dogrulanamadi(KorumaDogrulamaHatasi::AlanCozulemedi(
                        hata.to_string(),
                    ));
                }
            };
            let yalitici_arg = (istek.alan_hazirla && istek.yalitimli).then(|| yalitici.clone());
            koruma_dogrula(
                alan,
                yalitici_arg,
                true,
                istek.okunur_dizinler,
                istek.ag_ozeti,
                istek.onceki_ozet,
                secenekler,
            )
            .await
        })
    }

    fn son_denetim<'a>(
        &'a self,
        istek: SonDenetimIstegi,
    ) -> Pin<Box<dyn Future<Output = SonDenetimSonucu> + Send + 'a>> {
        let secenekler = self.secenekler(None);
        Box::pin(async move {
            let siniflandirici = match Siniflandirici::yeni_ayarla(&istek.kok, istek.gizli_ayarlari)
            {
                Ok(s) => Arc::new(s),
                Err(hata) => return SonDenetimSonucu::denetlenemedi(hata),
            };
            let alan = match CalismaAlani::ac(siniflandirici) {
                Ok(a) => a,
                Err(hata) => return SonDenetimSonucu::denetlenemedi(hata),
            };
            komut_sonrasi_denetim_secenekli(alan, istek.onceki, secenekler).await
        })
    }
}

pub use kalfa_tools::koruma::{
    AdliGirdiler as McpAdliGirdiler, DogrulanmisKoruma as McpDogrulanmisKoruma,
    KorumaDogrulamaHatasi as McpKorumaHatasi, KorumaSonucu as McpKorumaSonucu,
    SonDenetimSonucu as McpSonDenetimSonucu,
};

#[cfg(test)]
mod tests {
    use super::*;
    use kalfa_sandbox::YalitimYoklukNedeni;

    #[test]
    fn politika_ozeti_daralma_ve_genislemede_degisir() {
        let hazir = YalitimDurumu::Hazir {
            arka_uc: "bwrap".into(),
            surum: "0.9.0".into(),
        };
        let yok = YalitimDurumu::Yok {
            neden: YalitimYoklukNedeni::BwrapYok,
            talimat: "kur".into(),
            ayrinti: "yok".into(),
        };
        let gizli_1 = GizliAyarlari::default();
        let gizli_2 = GizliAyarlari::yeni(&["*.secret".into()], &[]).unwrap();
        let gizli_3 = GizliAyarlari::yeni(&[], &[".env.example".into()]).unwrap();

        let taban = politika_ozeti_hesapla(&hazir, &gizli_1, &[]);
        let arka_uc_farkli = politika_ozeti_hesapla(&yok, &gizli_1, &[]);
        let ek_kalip = politika_ozeti_hesapla(&hazir, &gizli_2, &[]);
        let istisna = politika_ozeti_hesapla(&hazir, &gizli_3, &[]);
        let okunur_ekli =
            politika_ozeti_hesapla(&hazir, &gizli_1, &[PathBuf::from("/opt/toolchain")]);

        assert_ne!(taban, arka_uc_farkli);
        assert_ne!(taban, ek_kalip);
        assert_ne!(taban, istisna);
        assert_ne!(taban, okunur_ekli);
    }
}
