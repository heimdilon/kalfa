//! Bağlam penceresi, token/bayt oranı ve oturum özetleme (`baglami_ozetle`).

use std::sync::Arc;
use std::time::Duration;

use kalfa_evren::EvrenError;
use kalfa_evren::tipler::{ChatMessage, ChatRequest, ChatResponse};

use super::Oturum;
use super::compact;
use crate::arac::AracAnlikGoruntusu;
use crate::butce::{ButceDenemesi, ButceTalepci, hata_kullanimi, sureli, yanit_kullanimi};
use crate::gonderim::{IstekAsamasi, IstekSonucu};
use crate::kayit::KayitOgesi;
use crate::protokol::{BitisNedeni, Olay, OzetTetigi};
use crate::saglayici::Saglayici;

/// 12 Ekim 2026'da ölçülen bağlam pencereleri (§4). Tabloda olmayan model,
/// `qwen3.8-flash-next` ve `auto` dahil, en küçüğünü alır.
pub fn pencere(model: &str) -> u64 {
    match model {
        "glm-5.3" => 524_288,
        "deepseek-v4-flash" => 1_048_576,
        _ => 262_144,
    }
}

/// Son ana isteğin token/bayt oranı (§5). Kullanım görülmeden 1/3 sayılır.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Oran {
    token: u64,
    bayt: u64,
}

impl Oran {
    pub fn yeni(token: u64, bayt: u64) -> Self {
        Self { token, bayt }
    }

    /// `bayt` büyüklüğündeki bağlamın tahmini istem tokenı.
    pub fn tahmin(self, bayt: u64) -> u64 {
        if self.token == 0 || self.bayt == 0 {
            return bayt / 3;
        }
        u64::try_from(u128::from(bayt) * u128::from(self.token) / u128::from(self.bayt))
            .unwrap_or(u64::MAX)
    }
}

/// Mesajın bağlamdaki bayt ölçüsü: içerik, düşünce ve araç çağrıları
/// (gönderim günlüğüyle aynı).
pub fn mesaj_bayti(m: &ChatMessage) -> u64 {
    let mut bayt = m.content.as_deref().map_or(0, str::len) as u64;
    bayt += m.reasoning.as_deref().map_or(0, str::len) as u64;
    for tc in m.tool_calls.iter().flatten() {
        bayt += (tc.function.name.len() + tc.function.arguments.len()) as u64;
    }
    bayt
}

/// Bir özetleme çağrısının mantıksal sonucu. `IzinYok`: deneme defteri izin
/// vermedi; ağa çıkılmadı.
enum OzetSonucu {
    Basarili(ChatResponse),
    Hata(EvrenError),
    IzinYok,
}

/// Özetleme isteği; hız sınırında bir kez yeniden dener, 120 sn'de zaman
/// aşımına uğrar. Her ağ denemesi kendi izni ve tek bitişini alır; dönen
/// denemeler kayda yazılır.
async fn ozetle(
    saglayici: Arc<dyn Saglayici>,
    istek: ChatRequest,
    butce: ButceTalepci,
) -> (OzetSonucu, Vec<ButceDenemesi>) {
    let mut kalan = compact::OZET_ZAMAN_ASIMI;
    let mut denemeler = Vec::new();
    let sonuc = match dene(
        &saglayici,
        istek.clone(),
        &butce,
        &mut kalan,
        &mut denemeler,
    )
    .await
    {
        None => return (OzetSonucu::IzinYok, denemeler),
        Some(Err(EvrenError::HizSiniri { tekrar_dene })) => {
            let bekle = tekrar_dene
                .unwrap_or(Duration::from_millis(500))
                .min(Duration::from_secs(5));
            if sureli(&mut kalan, tokio::time::sleep(bekle))
                .await
                .is_none()
            {
                Err(zaman_asimi_hatasi())
            } else {
                match dene(&saglayici, istek, &butce, &mut kalan, &mut denemeler).await {
                    None => return (OzetSonucu::IzinYok, denemeler),
                    Some(sonuc) => sonuc,
                }
            }
        }
        Some(sonuc) => sonuc,
    };
    let sonuc = match sonuc {
        Ok(yanit) => OzetSonucu::Basarili(yanit),
        Err(hata) => OzetSonucu::Hata(hata),
    };
    (sonuc, denemeler)
}

/// Tek özetleme denemesi: izin, kalan süreyle sınırlı çağrı, denemenin tek
/// bitişi ve kaydı. İzin yoksa `None`; ağa çıkılmaz.
async fn dene(
    saglayici: &Arc<dyn Saglayici>,
    istek: ChatRequest,
    butce: &ButceTalepci,
    kalan: &mut Duration,
    denemeler: &mut Vec<ButceDenemesi>,
) -> Option<Result<ChatResponse, EvrenError>> {
    let islem = butce.izin("özetleme").await?;
    let cagri = sureli(kalan, saglayici.sohbet(istek))
        .await
        .unwrap_or_else(|| Err(zaman_asimi_hatasi()));
    let (kullanim, meta, kayit) = match &cagri {
        Ok((yanit, meta)) => (
            yanit_kullanimi(yanit.usage.as_ref()),
            Some(meta.clone()),
            IstekSonucu::Basarili,
        ),
        Err(e) => (
            hata_kullanimi(e),
            None,
            IstekSonucu::Hata {
                asama: match e {
                    EvrenError::Ag {
                        govde_gitti: false, ..
                    } => IstekAsamasi::Baglanti,
                    _ => IstekAsamasi::Yanit,
                },
                mesaj: e.to_string(),
            },
        ),
    };
    butce.bitir(islem, kullanim).await;
    denemeler.push(ButceDenemesi {
        islem,
        kullanim,
        meta,
        sonuc: kayit,
    });
    Some(cagri.map(|(yanit, _)| yanit))
}

fn zaman_asimi_hatasi() -> EvrenError {
    EvrenError::ag_yanit("zaman aşımı (120 sn)")
}

impl Oturum {
    /// Bağlamın bayt ölçüsü: sistem mesajı, araç tanımları ve mesajlar (§5).
    pub(super) fn baglam_bayti(&self, goruntu: &AracAnlikGoruntusu) -> u64 {
        let tanimlar = serde_json::to_vec(goruntu.tanimlar()).map_or(0, |v| v.len() as u64);
        self.ayarlar.sistem_mesaji.len() as u64
            + tanimlar
            + self.mesajlar.iter().map(mesaj_bayti).sum::<u64>()
    }

    pub(super) async fn baglam_durumu_yay_anlik(&mut self) {
        let goruntu = self.baglam.araclar.anlik_goruntu(self.ayarlar.kip);
        self.baglam_durumunu_yay(&goruntu).await;
    }

    pub(super) async fn baglam_durumunu_yay(&mut self, goruntu: &AracAnlikGoruntusu) {
        let tahmin = self.oran.tahmin(self.baglam_bayti(goruntu));
        let esik = compact::esik(&self.ayarlar.model, self.ayarlar.sinirlar.etkin_pencere);
        self.yay(Olay::BaglamDurumu { tahmin, esik }).await;
    }

    /// Oturumun gönderilmiş geçmişini özetler (§5). Bir şey özetlendiyse `true`.
    pub(super) async fn baglami_ozetle(
        &mut self,
        tetik: OzetTetigi,
        talimat: Option<String>,
        goruntu: &AracAnlikGoruntusu,
    ) -> Result<bool, BitisNedeni> {
        let kapsanan = compact::kapsam(
            &self.mesajlar,
            &self.mesaj_bilgileri,
            self.gonderilen,
            tetik,
        );
        if kapsanan.is_empty() {
            return Ok(false);
        }

        let bayt = self.baglam_bayti(goruntu);
        let once = self.oran.tahmin(bayt);
        let secilen: Vec<&ChatMessage> = kapsanan.iter().map(|&i| &self.mesajlar[i]).collect();
        let parcalar = compact::duz_metin(&secilen);
        let azami_token = pencere(&self.ayarlar.model) / 10 * 9;
        let (tutulan, atilan) = compact::girdiyi_sigdir(parcalar, self.oran, azami_token);
        let istek = compact::ozet_istegi(&self.ayarlar.model, &tutulan, atilan, talimat.as_deref());
        let onceki = u32::try_from(kapsanan.len()).unwrap_or(u32::MAX);

        let ozet = loop {
            let saglayici = self.baglam.saglayici.clone();
            let butce = self.butce_talepci();
            let kapi = self.baglam.sart_kapisi.clone();
            let deneme = istek.clone();
            let (istek_nesli, (sonuc, denemeler)) = self
                .iptal_edilebilir(async move {
                    let istek_nesli = kapi.durum().nesil;
                    (istek_nesli, ozetle(saglayici, deneme, butce).await)
                })
                .await?;
            self.ozet_istegini_kaydet(&sonuc, denemeler, onceki).await;
            match sonuc {
                OzetSonucu::Basarili(yanit) => {
                    let metin = yanit
                        .choices
                        .into_iter()
                        .next()
                        .and_then(|c| c.message.content)
                        .map(|m| m.trim().to_string())
                        .filter(|m| !m.is_empty());
                    match metin {
                        Some(m) => break m,
                        None => {
                            if tetik != OzetTetigi::Zorunlu {
                                self.hata("Bağlam özetlenemedi: boş yanıt.").await;
                            }
                            return Ok(false);
                        }
                    }
                }
                OzetSonucu::IzinYok => {
                    return Err(BitisNedeni::Hata);
                }
                OzetSonucu::Hata(EvrenError::SartKabulGerekli { .. }) => {
                    self.sart_akisi(istek_nesli).await?;
                }
                OzetSonucu::Hata(e @ EvrenError::GunlukSinir { .. }) => {
                    self.hata(e.to_string()).await;
                    return Err(BitisNedeni::Hata);
                }
                OzetSonucu::Hata(e) => {
                    if tetik != OzetTetigi::Zorunlu {
                        self.hata(format!("Bağlam özetlenemedi: {e}")).await;
                    }
                    return Ok(false);
                }
            }
        };

        let icerik = compact::ozet_mesaji(&ozet, &uuid::Uuid::new_v4().simple().to_string());
        let kapsanan_bayt: u64 = kapsanan
            .iter()
            .map(|&i| mesaj_bayti(&self.mesajlar[i]))
            .sum();
        let yeni_bayt = bayt
            .saturating_sub(kapsanan_bayt)
            .saturating_add(icerik.len() as u64);
        let sonra = self.oran.tahmin(yeni_bayt);
        let kapsanan_u32: Vec<u32> = kapsanan
            .iter()
            .map(|&i| u32::try_from(i).unwrap_or(u32::MAX))
            .collect();

        if !self
            .kaydet_kalici(KayitOgesi::OturumOzetlendi {
                kapsanan: kapsanan_u32,
                icerik: icerik.clone(),
                tetik,
                talimat: talimat.clone(),
                once,
                sonra,
            })
            .await
        {
            return Err(BitisNedeni::Hata);
        }
        let ozet_sira = self.kayit.is_some().then_some(self.sira);
        let mut dizi = compact::BaglamDizisi {
            mesajlar: &mut self.mesajlar,
            bilgiler: &mut self.mesaj_bilgileri,
            gonderilen: &mut self.gonderilen,
            bekleyen: &mut self.bekleyen_ozetler,
        };
        compact::uygula(&mut dizi, &kapsanan, icerik, ozet_sira);
        self.yay(Olay::OturumOzetlendi {
            tetik,
            once,
            sonra,
            talimat,
            ozet,
        })
        .await;
        self.baglam_durumunu_yay(goruntu).await;
        Ok(true)
    }

    /// Özetleme denemelerini günlüğe yazar: her ağ denemesi kendi `Istek`
    /// satırıdır ve bütçe işlemine bağlanır. Başarılı yanıtın kullanımı ayrıca
    /// kaydedilir.
    async fn ozet_istegini_kaydet(
        &mut self,
        sonuc: &OzetSonucu,
        denemeler: Vec<ButceDenemesi>,
        onceki: u32,
    ) {
        let model = Some(self.ayarlar.model.clone());
        for deneme in &denemeler {
            match (&deneme.meta, &deneme.sonuc) {
                (Some(meta), _) => {
                    self.istek_kaydet(
                        "özetleme",
                        meta,
                        model.clone(),
                        Vec::new(),
                        onceki,
                        deneme.sonuc.clone(),
                        Some(deneme.islem),
                    )
                    .await;
                }
                (None, IstekSonucu::Hata { asama, mesaj }) => {
                    self.istek_kaydet_hata(
                        "özetleme",
                        model.clone(),
                        Vec::new(),
                        onceki,
                        *asama,
                        mesaj.clone(),
                        Some(deneme.islem),
                    )
                    .await;
                }
                (None, _) => {}
            }
        }
        if let OzetSonucu::Basarili(yanit) = sonuc
            && let Some(u) = &yanit.usage
        {
            let (istem, tamamlama, dusunce) =
                (u.prompt_tokens, u.completion_tokens, u.reasoning_tokens());
            self.kaydet(KayitOgesi::Kullanim {
                istem,
                tamamlama,
                dusunce,
            })
            .await;
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn pencere_olculen_degerler() {
        assert_eq!(pencere("glm-5.3"), 524_288);
        assert_eq!(pencere("deepseek-v4-flash"), 1_048_576);
        assert_eq!(pencere("qwen3.8-flash-next"), 262_144);
        assert_eq!(pencere("auto"), 262_144);
        assert_eq!(pencere("bilinmeyen"), 262_144);
    }

    #[test]
    fn oran_kullanimsiz_ucte_bir_kullanimla_olcer() {
        assert_eq!(Oran::default().tahmin(3000), 1000);
        assert_eq!(Oran::yeni(500, 2000).tahmin(8000), 2000);
        assert_eq!(Oran::yeni(u64::MAX, 1).tahmin(u64::MAX), u64::MAX);
    }
}
