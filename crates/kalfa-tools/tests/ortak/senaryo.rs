//! Senaryo testlerinin düzeneği (aşama 3 tasarımı §7.1).
//!
//! - Sahte EVREN ile çok turlu, betikli model yanıtları.
//! - Gerçek dosya araçları. İstenirse kabuk aracı, komut başlatmayan bir
//!   yalıtıcıyla: senaryolarda hiçbir süreç başlamaz.
//! - Senaryo iki şeyi ayrı sınar: son durum (çalışma alanı ve kayıt) ve iz
//!   (onay istekleri, uyarılar, sahte sağlayıcının aldığı istekler).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use kalfa_core::arac::Arac;
use kalfa_core::izin::{Kip, Siniflandirici};
use kalfa_core::oturum::{OturumAyarlari, OturumBaglami, devam_et};
use kalfa_core::protokol::{AracDurumu, Komut, Olay, OnayKarari, OnayOgesi};
use kalfa_core::test_destek::*;
use kalfa_evren::tipler::Role;
use kalfa_sandbox::{
    BaslatmaHatasi, CalistirmaIstegi, Surec, Yalitici, YalitimDurumu, YalitimYoklukNedeni,
};
use kalfa_tools::{CalismaAlani, KabukAraci, KabukGuveni, varsayilan_araclar};
use tempfile::TempDir;

/// Komut başlatmayan yalıtıcı. Durumu senaryo belirler; her başlatma isteği
/// sayılır ve reddedilir.
pub struct SenaryoYaliticisi {
    hazir: bool,
    baslatma: AtomicUsize,
}

impl SenaryoYaliticisi {
    pub fn baslatma_sayisi(&self) -> usize {
        self.baslatma.load(Ordering::SeqCst)
    }
}

impl Yalitici for SenaryoYaliticisi {
    fn durum(&self) -> YalitimDurumu {
        if self.hazir {
            YalitimDurumu::Hazir {
                arka_uc: "senaryo".into(),
                surum: "1".into(),
            }
        } else {
            YalitimDurumu::Yok {
                neden: YalitimYoklukNedeni::Desteklenmiyor,
                talimat: "Senaryoda yalıtım yok.".into(),
                ayrinti: "senaryo".into(),
            }
        }
    }

    fn profil_ozeti(&self, ag: bool) -> Option<String> {
        self.hazir.then(|| format!("senaryo-{ag}"))
    }

    fn baslat(&self, _istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi> {
        self.baslatma.fetch_add(1, Ordering::SeqCst);
        Err(BaslatmaHatasi::Yalitim(
            "senaryo yalıtıcısı komut başlatmaz".into(),
        ))
    }
}

struct Guven(AtomicBool);

impl KabukGuveni for Guven {
    fn guvenildi_mi(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    fn guvenildi(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// Kabuk aracının kurulumu.
#[derive(Debug, Clone, Copy)]
pub struct Kabuk {
    /// Yalıtım hazır.
    pub yalitim: bool,
    /// Yalıtım yokken yalıtımsız çalıştırma anahtarı.
    pub yalitimsiz_izni: bool,
    /// Çalışma alanındaki ilk kabuk onayı daha önce verildi.
    pub guvenildi: bool,
}

pub struct Senaryo {
    pub alan: TempDir,
    kayitlar: TempDir,
    kip: Kip,
    kayitli: bool,
    kabuk: Option<Kabuk>,
    pub yalitici: Arc<SenaryoYaliticisi>,
}

impl Senaryo {
    pub fn yeni(kip: Kip) -> Self {
        Self {
            alan: tempfile::tempdir().unwrap(),
            kayitlar: tempfile::tempdir().unwrap(),
            kip,
            kayitli: false,
            kabuk: None,
            yalitici: Arc::new(SenaryoYaliticisi {
                hazir: false,
                baslatma: AtomicUsize::new(0),
            }),
        }
    }

    /// Çalışma alanına bir dosya koyar; üst dizinler oluşturulur.
    pub fn dosya(self, yol: &str, icerik: &str) -> Self {
        let tam = self.alan.path().join(yol);
        std::fs::create_dir_all(tam.parent().unwrap()).unwrap();
        std::fs::write(tam, icerik).unwrap();
        self
    }

    /// Oturum kaydı çalışma alanının dışında tutulur.
    pub fn kayitli(mut self) -> Self {
        self.kayitli = true;
        self
    }

    pub fn kabuklu(mut self, k: Kabuk) -> Self {
        self.kabuk = Some(k);
        self.yalitici = Arc::new(SenaryoYaliticisi {
            hazir: k.yalitim,
            baslatma: AtomicUsize::new(0),
        });
        self
    }

    pub fn kayit_yolu(&self) -> PathBuf {
        self.kayitlar.path().join("oturum.jsonl")
    }

    /// Çalışma alanındaki dosyanın içeriği; yoksa `None`.
    pub fn oku(&self, yol: &str) -> Option<String> {
        std::fs::read_to_string(self.alan.path().join(yol)).ok()
    }

    fn ayarlar(&self) -> OturumAyarlari {
        let mut ayar = OturumAyarlari::yeni("glm-5.3", self.alan.path());
        ayar.kip = self.kip;
        if self.kayitli {
            ayar.kayit_yolu = Some(self.kayit_yolu());
        }
        ayar
    }

    fn baglam(&self, s: &Arc<SahteSaglayici>) -> OturumBaglami {
        let siniflandirici = Arc::new(Siniflandirici::yeni(self.alan.path(), &[]).unwrap());
        let calisma_alani = CalismaAlani::ac(siniflandirici.clone()).unwrap();
        let mut araclar: Vec<Arc<dyn Arac>> = varsayilan_araclar(calisma_alani.clone());
        if let Some(k) = self.kabuk {
            araclar.push(Arc::new(KabukAraci::yeni(
                calisma_alani,
                self.yalitici.clone(),
                Arc::new(Guven(AtomicBool::new(k.guvenildi))),
                Arc::new(AtomicBool::new(k.yalitimsiz_izni)),
            )));
        }
        test_baglami(s.clone(), araclar, siniflandirici)
    }

    pub fn baslat(&self, s: &Arc<SahteSaglayici>) -> Surucu {
        test_oturumu(self.baglam(s), self.ayarlar())
    }

    /// Kayıttan yeni bir sağlayıcıyla devam eder (Kalfa yeniden açılmış gibi).
    pub fn devam_et(&self, s: &Arc<SahteSaglayici>) -> Surucu {
        let (t, o, _) = devam_et(&self.kayit_yolu(), self.ayarlar(), self.baglam(s)).unwrap();
        Surucu::yeni(t, o)
    }
}

pub fn mesaj(m: &str) -> Komut {
    Komut::KullaniciMesaji { metin: m.into() }
}

pub async fn onay_bekle(d: &mut Surucu) -> Vec<OnayOgesi> {
    match d
        .bekle(|o| matches!(o, Olay::OnayGerekiyor { .. }))
        .await
        .yuk
    {
        Olay::OnayGerekiyor { ogeler, .. } => ogeler,
        _ => unreachable!(),
    }
}

pub fn karar(o: &OnayOgesi, onay: bool) -> Komut {
    Komut::AracOnayi {
        cagri: o.cagri,
        karar: if onay {
            OnayKarari::Onayla
        } else {
            OnayKarari::Reddet { gerekce: None }
        },
        muhur: o.muhur.clone(),
    }
}

/// Oturumun bildirdiği onay isteklerinin sayısı.
pub fn onay_istegi_sayisi(d: &Surucu) -> usize {
    d.gecmis
        .iter()
        .filter(|z| matches!(z.yuk, Olay::OnayGerekiyor { .. }))
        .count()
}

/// Araç sonuçları: araç, durum.
pub fn sonuclar(d: &Surucu) -> Vec<(String, AracDurumu)> {
    d.gecmis
        .iter()
        .filter_map(|z| match &z.yuk {
            Olay::AracSonucu { arac, durum, .. } => Some((arac.clone(), *durum)),
            _ => None,
        })
        .collect()
}

/// `istek`'inci isteğin araç iletileri.
pub fn arac_yanitlari(s: &SahteSaglayici, istek: usize) -> Vec<String> {
    s.istekler()[istek]
        .messages
        .iter()
        .filter(|m| m.role == Role::Tool)
        .map(|m| m.content.clone().unwrap_or_default())
        .collect()
}

/// Sahte sağlayıcının aldığı bütün istekler, EVREN'e gidecekleri biçimde.
/// Sızıntı denetimi bunun üzerinde yapılır: ileti, araç argümanı ve tanım
/// dahil hiçbir yerde geçmemeli.
pub fn butun_istekler(s: &SahteSaglayici) -> String {
    serde_json::to_string(&s.istekler()).unwrap()
}

/// Her istekte her araç çağrısının tam olarak bir yanıtı vardır.
pub fn arac_cagrilari_tek_yanitli(s: &SahteSaglayici) {
    for (istek_no, istek) in s.istekler().iter().enumerate() {
        let cagrilar: Vec<&str> = istek
            .messages
            .iter()
            .flat_map(|m| m.tool_calls.iter().flatten())
            .map(|tc| tc.id.as_str())
            .collect();
        let yanitlar: Vec<&str> = istek
            .messages
            .iter()
            .filter(|m| m.role == Role::Tool)
            .filter_map(|m| m.tool_call_id.as_deref())
            .collect();
        assert_eq!(
            cagrilar.len(),
            yanitlar.len(),
            "istek {istek_no}: her araç çağrısı için bir araç yanıtı olmalı"
        );
        for cagri in cagrilar {
            assert_eq!(
                yanitlar.iter().filter(|y| **y == cagri).count(),
                1,
                "istek {istek_no}: {cagri} tam olarak bir yanıta bağlanmalı"
            );
        }
    }
}
