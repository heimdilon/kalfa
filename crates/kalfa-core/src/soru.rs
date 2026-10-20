use std::collections::HashSet;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::arac::{Arac, AracCiktisi, AracHatasi, AracOrtami, Hazirlik};
use crate::kimlik::{CagriId, TurId};

/// Soru başlığında bulunabilecek azami karakter sayısı.
pub const AZAMI_SORU_BASLIGI_KARAKTER: usize = 500;

/// Bir soruda bulunması gereken asgari seçenek sayısı.
pub const ASGARI_SECENEK_SAYISI: usize = 2;

/// Bir soruda bulunabilecek azami seçenek sayısı.
pub const AZAMI_SECENEK_SAYISI: usize = 4;

/// Bir seçenek etiketinde veya açıklamasında bulunabilecek azami karakter sayısı.
pub const AZAMI_SECENEK_KARAKTER: usize = 200;

/// Bir seçenek kimliğinde bulunabilecek azami karakter sayısı.
pub const AZAMI_SECENEK_KIMLIGI_KARAKTER: usize = 64;

/// Serbest kullanıcı yanıtında bulunabilecek azami karakter sayısı.
pub const AZAMI_SERBEST_YANIT_KARAKTER: usize = 4_000;

fn varsayilan_serbest_yanit_izni() -> bool {
    true
}

/// Kullanıcıya sunulan tek bir soru seçeneği.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoruSecenegi {
    #[serde(alias = "id")]
    pub kimlik: String,
    #[serde(alias = "metin", alias = "baslik")]
    pub etiket: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aciklama: Option<String>,
}

/// Oturumda bekleyen veya sorulmuş yapılandırılmış kullanıcı sorusu.
/// Araç onayından tamamen ayrıdır; yanıtlanması izin kipi yükseltmez veya araç onaylamaz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KullaniciSorusu {
    pub soru_kimligi: String,
    pub tur: TurId,
    pub cagri: CagriId,
    pub baslik: String,
    pub secenekler: Vec<SoruSecenegi>,
    #[serde(default = "varsayilan_serbest_yanit_izni")]
    pub serbest_yanit_izni: bool,
}

/// Kullanıcının soruya verdiği yapılandırılmış yanıt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tur")]
pub enum KullaniciSorusuYaniti {
    #[serde(alias = "secenek")]
    Secenek { secenek_kimligi: String },
    #[serde(alias = "serbest_metin", alias = "serbest")]
    SerbestMetin { metin: String },
}

/// Sorunun sonlanma biçimi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "durum")]
pub enum SoruSonucu {
    #[serde(alias = "yanitlandi")]
    Yanitlandi {
        yanit: KullaniciSorusuYaniti,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        secilen_etiket: Option<String>,
    },
    #[serde(alias = "kullanici_mesajiyla_iptal")]
    KullaniciMesajiylaIptal { yeni_mesaj: String },
    #[serde(alias = "iptal_edildi")]
    IptalEdildi,
}

/// Soru ve yanıt doğrulama hataları.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SoruHatasi {
    #[error("soru başlığı boş olamaz")]
    BosBaslik,
    #[error("soru başlığı en çok {sinir} karakter olabilir (gelen: {gercek})")]
    BaslikCokUzun { sinir: usize, gercek: usize },
    #[error("soru {asgari} ile {azami} arasında seçenek içermelidir (gelen: {gercek})")]
    GecersizSecenekSayisi {
        asgari: usize,
        azami: usize,
        gercek: usize,
    },
    #[error("seçenek kimliği boş olamaz")]
    BosSecenekKimligi,
    #[error("seçenek kimliği en çok {sinir} karakter olabilir: '{kimlik}'")]
    UzunSecenekKimligi { kimlik: String, sinir: usize },
    #[error("yinelenen seçenek kimliği: '{kimlik}'")]
    YinelenenSecenekKimligi { kimlik: String },
    #[error("'{kimlik}' seçeneğinin etiketi boş olamaz")]
    BosSecenekEtiketi { kimlik: String },
    #[error("'{kimlik}' seçeneğinin metni en çok {sinir} karakter olabilir (gelen: {gercek})")]
    SecenekCokUzun {
        kimlik: String,
        sinir: usize,
        gercek: usize,
    },
    #[error("geçersiz seçenek kimliği: '{secenek_kimligi}'")]
    GecersizSecenek { secenek_kimligi: String },
    #[error("bu soru serbest metin yanıtına kapalıdır")]
    SerbestYanitKapali,
    #[error("serbest yanıt boş olamaz")]
    BosSerbestYanit,
    #[error("serbest yanıt en çok {sinir} karakter olabilir (gelen: {gercek})")]
    SerbestYanitCokUzun { sinir: usize, gercek: usize },
}

/// Tur ve araç çağrısı kimliğinden kararlı soru kimliği üretir.
pub fn soru_kimligi_uret(tur: TurId, cagri: CagriId) -> String {
    format!("soru-{tur}-{cagri}")
}

/// Soru girdisini sınır ve benzersizlik kurallarına göre doğrular.
pub fn soru_girdisini_dogrula(
    tur: TurId,
    cagri: CagriId,
    baslik: &str,
    ham_secenekler: Vec<SoruSecenegi>,
    serbest_yanit_izni: bool,
) -> Result<KullaniciSorusu, SoruHatasi> {
    let temiz_baslik = baslik.trim().to_string();
    if temiz_baslik.is_empty() {
        return Err(SoruHatasi::BosBaslik);
    }
    let baslik_uzunluk = temiz_baslik.chars().count();
    if baslik_uzunluk > AZAMI_SORU_BASLIGI_KARAKTER {
        return Err(SoruHatasi::BaslikCokUzun {
            sinir: AZAMI_SORU_BASLIGI_KARAKTER,
            gercek: baslik_uzunluk,
        });
    }

    if !(ASGARI_SECENEK_SAYISI..=AZAMI_SECENEK_SAYISI).contains(&ham_secenekler.len()) {
        return Err(SoruHatasi::GecersizSecenekSayisi {
            asgari: ASGARI_SECENEK_SAYISI,
            azami: AZAMI_SECENEK_SAYISI,
            gercek: ham_secenekler.len(),
        });
    }

    let mut gorulen = HashSet::with_capacity(ham_secenekler.len());
    let mut secenekler = Vec::with_capacity(ham_secenekler.len());

    for secenek in ham_secenekler {
        let kimlik = secenek.kimlik.trim().to_string();
        if kimlik.is_empty() {
            return Err(SoruHatasi::BosSecenekKimligi);
        }
        if kimlik.chars().count() > AZAMI_SECENEK_KIMLIGI_KARAKTER {
            return Err(SoruHatasi::UzunSecenekKimligi {
                kimlik,
                sinir: AZAMI_SECENEK_KIMLIGI_KARAKTER,
            });
        }
        if !gorulen.insert(kimlik.clone()) {
            return Err(SoruHatasi::YinelenenSecenekKimligi { kimlik });
        }

        let etiket = secenek.etiket.trim().to_string();
        if etiket.is_empty() {
            return Err(SoruHatasi::BosSecenekEtiketi { kimlik });
        }
        let etiket_uzunluk = etiket.chars().count();
        if etiket_uzunluk > AZAMI_SECENEK_KARAKTER {
            return Err(SoruHatasi::SecenekCokUzun {
                kimlik,
                sinir: AZAMI_SECENEK_KARAKTER,
                gercek: etiket_uzunluk,
            });
        }

        let aciklama = match secenek.aciklama {
            Some(ham) => {
                let temiz = ham.trim().to_string();
                if temiz.is_empty() {
                    None
                } else {
                    let uz = temiz.chars().count();
                    if uz > AZAMI_SECENEK_KARAKTER {
                        return Err(SoruHatasi::SecenekCokUzun {
                            kimlik,
                            sinir: AZAMI_SECENEK_KARAKTER,
                            gercek: uz,
                        });
                    }
                    Some(temiz)
                }
            }
            None => None,
        };

        secenekler.push(SoruSecenegi {
            kimlik,
            etiket,
            aciklama,
        });
    }

    Ok(KullaniciSorusu {
        soru_kimligi: soru_kimligi_uret(tur, cagri),
        tur,
        cagri,
        baslik: temiz_baslik,
        secenekler,
        serbest_yanit_izni,
    })
}

/// Kullanıcı yanıtını etkin sorunun seçenek ve serbest metin sınırlarına göre doğrular.
pub fn yaniti_dogrula(
    soru: &KullaniciSorusu,
    yanit: KullaniciSorusuYaniti,
) -> Result<(KullaniciSorusuYaniti, Option<String>), SoruHatasi> {
    match yanit {
        KullaniciSorusuYaniti::Secenek { secenek_kimligi } => {
            let aranan = secenek_kimligi.trim();
            let Some(bulunan) = soru.secenekler.iter().find(|s| s.kimlik == aranan) else {
                return Err(SoruHatasi::GecersizSecenek {
                    secenek_kimligi: aranan.to_string(),
                });
            };
            Ok((
                KullaniciSorusuYaniti::Secenek {
                    secenek_kimligi: bulunan.kimlik.clone(),
                },
                Some(bulunan.etiket.clone()),
            ))
        }
        KullaniciSorusuYaniti::SerbestMetin { metin } => {
            if !soru.serbest_yanit_izni {
                return Err(SoruHatasi::SerbestYanitKapali);
            }
            let temiz = metin.trim().to_string();
            if temiz.is_empty() {
                return Err(SoruHatasi::BosSerbestYanit);
            }
            let uzunluk = temiz.chars().count();
            if uzunluk > AZAMI_SERBEST_YANIT_KARAKTER {
                return Err(SoruHatasi::SerbestYanitCokUzun {
                    sinir: AZAMI_SERBEST_YANIT_KARAKTER,
                    gercek: uzunluk,
                });
            }
            Ok((KullaniciSorusuYaniti::SerbestMetin { metin: temiz }, None))
        }
    }
}

#[derive(Debug, Deserialize)]
struct KullaniciyaSorGirdisi {
    baslik: String,
    secenekler: Vec<SoruSecenegi>,
    #[serde(default = "varsayilan_serbest_yanit_izni")]
    serbest_yanit_izni: bool,
}

/// Belirsiz gereksinimlerde kullanıcıya çoktan seçmeli soru soran çekirdek aracı (`kullaniciya_sor`).
/// Soru beklerken çalışma alanı veya dosya kilidi tutmaz; süre dolması yanıt sayılmaz.
#[derive(Debug, Default)]
pub struct KullaniciyaSorAraci;

impl KullaniciyaSorAraci {
    pub fn yeni() -> Self {
        Self
    }
}

#[async_trait]
impl Arac for KullaniciyaSorAraci {
    fn ad(&self) -> &str {
        "kullaniciya_sor"
    }

    fn aciklama(&self) -> &str {
        "Gereksinim belirsizliğinde veya tasarım tercihinde kullanıcıya 2-4 seçenekli tek bir \
         soru sorar ve yanıtı bekler. Araç onayı yerine kullanılmaz."
    }

    fn parametre_semasi(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "baslik": {
                    "type": "string",
                    "description": "Kullanıcıya sorulacak net soru metni (en çok 500 karakter)."
                },
                "secenekler": {
                    "type": "array",
                    "description": "Kullanıcının seçebileceği 2 ile 4 arası seçenek.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "kimlik": {
                                "type": "string",
                                "description": "Benzersiz seçenek kimliği (ör. 'secenek-a')."
                            },
                            "etiket": {
                                "type": "string",
                                "description": "Kısa seçenek metni (en çok 200 karakter)."
                            },
                            "aciklama": {
                                "type": "string",
                                "description": "İsteğe bağlı ek açıklama (en çok 200 karakter)."
                            }
                        },
                        "required": ["kimlik", "etiket"]
                    }
                },
                "serbest_yanit_izni": {
                    "type": "boolean",
                    "description": "Kullanıcının seçenekler dışında serbest metin (<= 4000 karakter) yazabilmesi."
                }
            },
            "required": ["baslik", "secenekler"]
        })
    }

    fn yazar_mi(&self) -> bool {
        false
    }

    async fn hazirla(&self, argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
        let ayristirilan: KullaniciyaSorGirdisi = serde_json::from_value(argumanlar.clone())
            .map_err(|e| AracHatasi::Gecersiz(format!("kullaniciya_sor girdisi geçersiz: {e}")))?;
        let dogrulanan = soru_girdisini_dogrula(
            TurId::yeni(),
            CagriId::yeni(),
            &ayristirilan.baslik,
            ayristirilan.secenekler,
            ayristirilan.serbest_yanit_izni,
        )
        .map_err(|e| AracHatasi::Gecersiz(e.to_string()))?;

        Ok(Hazirlik {
            aciklama: format!("Kullanıcıya soru sorulacak: {}", dogrulanan.baslik),
            kilitsiz: true,
            ..Hazirlik::default()
        })
    }

    async fn calistir(
        &self,
        _argumanlar: &Value,
        _hazirlik: &Hazirlik,
        _iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        Err(AracHatasi::Gecersiz(
            "kullaniciya_sor yalnız oturum ortamında çalıştırılabilir.".to_string(),
        ))
    }

    async fn calistir_ortamli(
        &self,
        argumanlar: &Value,
        _hazirlik: &Hazirlik,
        iptal: CancellationToken,
        ortam: AracOrtami,
    ) -> Result<AracCiktisi, AracHatasi> {
        if iptal.is_cancelled() {
            return Err(AracHatasi::Iptal);
        }
        let ayristirilan: KullaniciyaSorGirdisi = serde_json::from_value(argumanlar.clone())
            .map_err(|e| AracHatasi::Gecersiz(format!("kullaniciya_sor girdisi geçersiz: {e}")))?;
        let dogrulanan = soru_girdisini_dogrula(
            TurId::yeni(),
            CagriId::yeni(),
            &ayristirilan.baslik,
            ayristirilan.secenekler,
            ayristirilan.serbest_yanit_izni,
        )
        .map_err(|e| AracHatasi::Gecersiz(e.to_string()))?;

        let sonuc = ortam
            .kullaniciya_sor(
                dogrulanan.baslik,
                dogrulanan.secenekler,
                dogrulanan.serbest_yanit_izni,
                iptal,
            )
            .await?;

        match sonuc {
            SoruSonucu::Yanitlandi {
                yanit,
                secilen_etiket,
            } => {
                let icerik = match yanit {
                    KullaniciSorusuYaniti::Secenek { secenek_kimligi } => {
                        let etiket_eki = secilen_etiket
                            .map(|e| format!(" ({e})"))
                            .unwrap_or_default();
                        format!("Kullanıcı yanıtı (seçenek): {secenek_kimligi}{etiket_eki}")
                    }
                    KullaniciSorusuYaniti::SerbestMetin { metin } => {
                        format!("Kullanıcı yanıtı (serbest metin): {metin}")
                    }
                };
                Ok(AracCiktisi {
                    icerik,
                    kaynak: "kalfa:soru".to_string(),
                    kalfa_uretimi: true,
                    ..AracCiktisi::default()
                })
            }
            SoruSonucu::KullaniciMesajiylaIptal { .. } => Ok(AracCiktisi {
                icerik: "Kullanıcı soruyu yanıtlamak yerine yeni bir mesaj göndererek yönlendirmeyi değiştirdi."
                    .to_string(),
                kaynak: "kalfa:soru".to_string(),
                kalfa_uretimi: true,
                ..AracCiktisi::default()
            }),
            SoruSonucu::IptalEdildi => Err(AracHatasi::Iptal),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ornek_secenekler() -> Vec<SoruSecenegi> {
        vec![
            SoruSecenegi {
                kimlik: "evet".to_string(),
                etiket: "Evet, uygula".to_string(),
                aciklama: None,
            },
            SoruSecenegi {
                kimlik: "hayir".to_string(),
                etiket: "Hayır, vazgeç".to_string(),
                aciklama: None,
            },
        ]
    }

    #[test]
    fn soru_sinirlari_dogrulanir() {
        let tek_secenek = soru_girdisini_dogrula(
            TurId::yeni(),
            CagriId::yeni(),
            "Devam edelim mi?",
            vec![SoruSecenegi {
                kimlik: "tek".to_string(),
                etiket: "Tek".to_string(),
                aciklama: None,
            }],
            true,
        );
        assert!(matches!(
            tek_secenek,
            Err(SoruHatasi::GecersizSecenekSayisi { gercek: 1, .. })
        ));

        let uzun_baslik = "x".repeat(AZAMI_SORU_BASLIGI_KARAKTER + 1);
        assert!(matches!(
            soru_girdisini_dogrula(
                TurId::yeni(),
                CagriId::yeni(),
                &uzun_baslik,
                ornek_secenekler(),
                true
            ),
            Err(SoruHatasi::BaslikCokUzun { .. })
        ));
    }

    #[test]
    fn yanit_secenek_ve_serbest_metin_sinirlarini_dogrular() {
        let soru = soru_girdisini_dogrula(
            TurId::yeni(),
            CagriId::yeni(),
            "Hangi yaklaşım?",
            ornek_secenekler(),
            true,
        )
        .unwrap();

        assert!(matches!(
            yaniti_dogrula(
                &soru,
                KullaniciSorusuYaniti::Secenek {
                    secenek_kimligi: "olmayan".to_string()
                }
            ),
            Err(SoruHatasi::GecersizSecenek { .. })
        ));

        let cok_uzun = "a".repeat(AZAMI_SERBEST_YANIT_KARAKTER + 1);
        assert!(matches!(
            yaniti_dogrula(
                &soru,
                KullaniciSorusuYaniti::SerbestMetin { metin: cok_uzun }
            ),
            Err(SoruHatasi::SerbestYanitCokUzun { .. })
        ));

        let gecerli = yaniti_dogrula(
            &soru,
            KullaniciSorusuYaniti::Secenek {
                secenek_kimligi: "evet".to_string(),
            },
        )
        .unwrap();
        assert_eq!(gecerli.1.as_deref(), Some("Evet, uygula"));
    }
}
