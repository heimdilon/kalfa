//! Eklenti aracı (aşama 4 tasarımı §8.1, §8.4): yerleşik araçlarla aynı izin
//! motorundan geçer. Bir çalışma alanında EVREN'e ilk gönderimden önce onay ister.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use kalfa_core::arac::{
    Arac, AracCiktisi, AracHatasi, AracOrtami, CiktiGonderici, EklentiTarifi, Hazirlik, kirp,
};
use kalfa_core::kilit::KaynakKilitleri;
use kalfa_core::protokol::{CiktiAkisi, VeriCikisi};
use kalfa_tools::CalismaAlani;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::bag::kalfa::plugin::tipler::Hata;
use crate::calistirici::{CagriHatasi, Eklenti, Motor};
use crate::depo::Depo;
use crate::evren::{EvrenBaglami, GonderimIzni};
use crate::host::{CagriOrtami, acik_dosyalar};
use crate::manifest::{EvrenUcu, Manifest};
use crate::sinir;

pub struct EklentiAraci {
    motor: Arc<Motor>,
    eklenti: Arc<Eklenti>,
    /// `eklenti.araclar` içindeki sırası.
    sira: usize,
    model_adi: String,
    alan: Arc<CalismaAlani>,
    kilitler: Arc<KaynakKilitleri>,
    /// Bu eklentinin bu çalışma alanındaki deposu (§10).
    depo_dizini: PathBuf,
    /// EVREN bağlantısı; yoksa `evren-cagir` "bağlantı yok" döner.
    evren: Option<Arc<EvrenBaglami>>,
    /// Bu eklentinin bu çalışma alanında EVREN'e gönderim izni (§8.4).
    izin: Arc<dyn GonderimIzni>,
}

impl EklentiAraci {
    /// Eklentinin bütün araçları; bir çalışma alanının oturumu için.
    /// `depo_dizini` eklentiye ve çalışma alanına özeldir.
    pub fn hepsi(
        motor: &Arc<Motor>,
        eklenti: &Arc<Eklenti>,
        alan: &Arc<CalismaAlani>,
        kilitler: &Arc<KaynakKilitleri>,
        depo_dizini: &Path,
        evren: Option<Arc<EvrenBaglami>>,
        izin: Arc<dyn GonderimIzni>,
    ) -> Vec<Arc<dyn Arac>> {
        (0..eklenti.araclar.len())
            .map(|sira| {
                Arc::new(Self {
                    motor: motor.clone(),
                    eklenti: eklenti.clone(),
                    sira,
                    model_adi: eklenti.manifest.model_adi(&eklenti.araclar[sira].ad),
                    alan: alan.clone(),
                    kilitler: kilitler.clone(),
                    depo_dizini: depo_dizini.to_path_buf(),
                    evren: evren.clone(),
                    izin: izin.clone(),
                }) as Arc<dyn Arac>
            })
            .collect()
    }

    fn arac_adi(&self) -> &str {
        &self.eklenti.araclar[self.sira].ad
    }

    fn tarif(&self) -> EklentiTarifi {
        let m = &self.eklenti.manifest;
        EklentiTarifi {
            ad: m.ad.clone(),
            surum: m.surum.to_string(),
            ozet: self.eklenti.ozet.clone(),
            globlar: m.dosya_oku.clone(),
            evren_uclari: m.evren.iter().map(|u| u.ad().to_string()).collect(),
            ..Default::default()
        }
    }

    /// `dosya-listele`'nin manifest globlarıyla vereceği dosyaların sayısı ve
    /// toplam boyutu (§8.1). Yalnızca ilk gönderimde hesaplanır.
    // ponytail: izin bekleyen her hazırlıkta ağaç taranır; büyük çalışma
    // alanında önbellek gerekirse dosya değişimine göre tutulabilir.
    async fn acik_dosya_ozeti(&self) -> Result<(u64, u64), AracHatasi> {
        let manifest = self.eklenti.manifest.clone();
        let alan = self.alan.clone();
        tokio::task::spawn_blocking(move || -> Result<(u64, u64), AracHatasi> {
            let korumali = kalfa_core::git_koruma::korumali_yollar(alan.kok());
            let liste =
                acik_dosyalar(&manifest, &alan, &korumali, None, &CancellationToken::new())?;
            let bayt: u64 = liste.iter().map(|(_, boyut, _)| boyut).sum();
            Ok((u64::try_from(liste.len()).unwrap_or(u64::MAX), bayt))
        })
        .await
        .map_err(|e| AracHatasi::GirisCikis(e.to_string()))?
    }

    fn hata_iletisi(&self, h: &CagriHatasi) -> String {
        let ad = &self.eklenti.manifest.ad;
        let ileti = match h {
            CagriHatasi::Eklenti(h) => {
                let (tur, ileti) = match h {
                    Hata::IzinYok(m) => ("izin-yok", m.as_str()),
                    Hata::Bulunamadi(m) => ("bulunamadi", m.as_str()),
                    Hata::IptalEdildi => ("iptal-edildi", ""),
                    Hata::SinirAsildi(m) => ("sinir-asildi", m.as_str()),
                    Hata::Evren(m) => ("evren", m.as_str()),
                    Hata::Diger(m) => ("diger", m.as_str()),
                };
                format!("Eklenti hatası ({ad}): {tur}: {ileti}")
            }
            CagriHatasi::Bellek => format!(
                "Eklenti durduruldu ({ad}): bellek sınırı ({} MiB) aşıldı.",
                sinir::BELLEK / (1024 * 1024)
            ),
            CagriHatasi::Cpu => format!("Eklenti durduruldu ({ad}): işlem süresi sınırı aşıldı."),
            CagriHatasi::Sure => format!("Eklenti durduruldu ({ad}): süre sınırı aşıldı."),
            CagriHatasi::Cokme => format!("Eklenti çalışırken çöktü ({ad})."),
            CagriHatasi::Iptal => "İptal edildi.".into(),
        };
        kirp(&ileti, sinir::HATA_ILETISI)
    }
}

/// İlk gönderim kartının metni (§8.4). Kartın başındaki cümle eklentinin
/// aracını adlandırır; asıl onay metni ondan sonra gelir.
fn ilk_gonderim_metni(m: &Manifest, sayi: u64, bayt: u64) -> String {
    let amac = match m.evren.as_slice() {
        [EvrenUcu::Embeddings] => "gömme için",
        [EvrenUcu::Rerank] => "yeniden sıralama için",
        _ => "gömme ve yeniden sıralama için",
    };
    let kapsam = if m.dosya_oku.is_empty() {
        "Okuyabileceği dosya yok.".to_string()
    } else {
        let globlar = m
            .dosya_oku
            .iter()
            .map(|g| format!("`{g}`"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "Okuyabileceği dosyalar: {globlar} ile eşleşen **{sayi} dosya ({})**.",
            boyut_metni(bayt)
        )
    };
    format!(
        "**{}** bu çalışma alanında ilk kez EVREN'e veri gönderecek.\n\n{kapsam} Bu dosyalardan seçtiği parçalar {amac} EVREN'e gidecek. Gizli dosyalar gönderilmez.\n\nİzin verirseniz bu çalışma alanında bir daha sorulmaz. İzni Eklentiler ekranından geri alabilirsiniz.",
        m.ad
    )
}

/// Bayt sayısı Türkçe ondalıkla: `6,0 MB`.
fn boyut_metni(bayt: u64) -> String {
    format!("{:.1} MB", bayt as f64 / 1_000_000.0).replace('.', ",")
}

#[async_trait]
impl Arac for EklentiAraci {
    fn ad(&self) -> &str {
        &self.model_adi
    }

    fn aciklama(&self) -> &str {
        &self.eklenti.araclar[self.sira].aciklama
    }

    fn parametre_semasi(&self) -> Value {
        self.eklenti.araclar[self.sira].sema.clone()
    }

    /// v1 eklentileri yazamaz; İnceleme kipinde de sunulur.
    fn yazar_mi(&self) -> bool {
        false
    }

    /// Eklenti bu çalışma alanında kapatıldıysa, devre dışıysa ya da
    /// kaldırıldıysa araç sunulmaz (§13.3).
    fn sunulur_mu(&self) -> bool {
        self.izin.acik_mi()
    }

    async fn hazirla(&self, _argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
        let m = &self.eklenti.manifest;
        let mut tarif = self.tarif();
        let mut aciklama = format!(
            "{} eklentisinin `{}` aracını çalıştırmak istiyor.",
            m.ad,
            self.arac_adi()
        );
        if !m.evren.is_empty() && !self.izin.var_mi() {
            let (sayi, bayt) = self.acik_dosya_ozeti().await?;
            tarif.ilk_gonderim = true;
            tarif.dosya_sayisi = sayi;
            tarif.bayt = bayt;
            aciklama = format!("{aciklama}\n\n{}", ilk_gonderim_metni(m, sayi, bayt));
        }
        Ok(Hazirlik {
            aciklama,
            veri_cikisi: Some(VeriCikisi::EklentiCiktisi),
            eklenti: Some(tarif),
            kilitsiz: true,
            ..Default::default()
        })
    }

    async fn calistir(
        &self,
        argumanlar: &Value,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.calistir_ortamli(argumanlar, hazirlik, iptal, AracOrtami::bos())
            .await
    }

    async fn calistir_akisli(
        &self,
        argumanlar: &Value,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
        cikti: CiktiGonderici,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.calistir_ortamli(argumanlar, hazirlik, iptal, AracOrtami::ciktiyla(cikti))
            .await
    }

    /// Oturum yalnızca onaylanmış çağrıyı çalıştırır (§8.4): hazırlıkta ilk
    /// gönderim varsa onay bu noktada kalıcı olur. Kayıt başarısızsa araç
    /// çalışmaz.
    async fn calistir_ortamli(
        &self,
        argumanlar: &Value,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
        ortam: AracOrtami,
    ) -> Result<AracCiktisi, AracHatasi> {
        let ilk_onayli = hazirlik.eklenti.as_ref().is_some_and(|t| t.ilk_gonderim);
        if ilk_onayli {
            self.izin
                .ver()
                .map_err(|e| AracHatasi::GirisCikis(format!("gönderim izni kaydedilemedi: {e}")))?;
        }
        let cikti = ortam.cikti();
        let cagri = CagriOrtami::yeni(
            self.eklenti.manifest.clone(),
            self.alan.clone(),
            self.kilitler.clone(),
            Depo::yeni(self.depo_dizini.clone(), self.eklenti.manifest.depo_mb),
            iptal,
            ortam,
        )
        .evrenli(self.evren.clone(), self.izin.clone(), ilk_onayli);
        let (sonuc, stderr) = self
            .eklenti
            .calistir(&self.motor, self.arac_adi(), &argumanlar.to_string(), cagri)
            .await;
        match sonuc {
            Ok(icerik) => Ok(AracCiktisi {
                icerik: kirp(&icerik, sinir::SONUC),
                kaynak: format!("eklenti:{}:{}", self.eklenti.manifest.ad, self.arac_adi()),
                ..Default::default()
            }),
            Err(CagriHatasi::Iptal) => Err(AracHatasi::Iptal),
            Err(h) => {
                // Tuzakta `stderr`'in sonu (panik iletisi) yalnızca canlı
                // çıktıya gider; modele gitmez (§7.3).
                if !matches!(h, CagriHatasi::Eklenti(_)) && !stderr.is_empty() {
                    cikti.gonder(CiktiAkisi::Hata, stderr);
                }
                Err(AracHatasi::Gecersiz(self.hata_iletisi(&h)))
            }
        }
    }
}
