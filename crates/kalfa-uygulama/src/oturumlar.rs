//! Oturum tablosu (1c tasarımı §5): açık oturumlar, kayıttan açma, köprü,
//! kapatma. Aynı oturum ikinci kez açılmaz; açık olanın geçmişi döner.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use kalfa_core::aktarim::{KrediTutamaci, Paketleyici};
use kalfa_core::arac::AracKayitDefteri;
use kalfa_core::gecmis::{self, DurumOzeti, GecmisSayfasi};
use kalfa_core::izin::{Kip, Siniflandirici};
use kalfa_core::kayit::OturumKaydi;
use kalfa_core::kimlik::OturumId;
use kalfa_core::kurtarma::kurtar;
use kalfa_core::oturum::{self, OlayAlici, OturumAyarlari, OturumBaglami, OturumTutamaci};
use kalfa_core::protokol::{Komut, OnayKarari, OnayOgesi};
use kalfa_tools::{CalismaAlani as AracAlani, KabukAraci, varsayilan_araclar};
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinHandle;

use crate::arayuz::KanalYazici;
use crate::calisma_alanlari::{AlanGuveni, CalismaAlaniKaydi};
use crate::hata::KomutHatasi;
use crate::sozlesme::{
    CalismaAlani, OnayGirdisi, OturumAcilisi, OturumSatiri, SaltOkunurNedeni, UygulamaDurumu,
};
use crate::{Uygulama, dizin, guvenlik, kopru, modeller};

/// Açılışta ve her eski sayfada yüklenen tur sayısı (1c tasarımı §9.3).
pub const SAYFA_TURU: usize = 20;

/// Kapatılan oturumun görevinin bitmesi için beklenen en uzun süre.
const KAPANIS_SURESI: Duration = Duration::from_secs(5);

pub(crate) struct AcikOturum {
    tutamac: OturumTutamaci,
    /// `oturumu_dinle` çağrılana kadar burada durur.
    alici: Option<OlayAlici>,
    kopru: Option<(JoinHandle<()>, KrediTutamaci)>,
    kip: Kip,
    kip_kilidi: Arc<AsyncMutex<()>>,
    yol: PathBuf,
    calisma_alani: PathBuf,
}

fn ic_hata(e: impl std::fmt::Display) -> KomutHatasi {
    KomutHatasi::ic(e.to_string())
}

impl Uygulama {
    pub fn uygulama_durumu(&self) -> UygulamaDurumu {
        let ic = self.ic();
        UygulamaDurumu {
            baglanti: ic.baglanti,
            anahtar_deposu: self.kasa.tur(),
            modeller: modeller::modeller(),
            varsayilan_model: modeller::VARSAYILAN_MODEL.to_string(),
            son_oturum: ic.son_oturum,
            yalitim: self.yalitici.durum(),
            platform: std::env::consts::OS.to_string(),
        }
    }

    pub async fn oturumlari_listele(&self) -> Result<Vec<OturumSatiri>, KomutHatasi> {
        let dizin = self.oturum_dizini.clone();
        let ozetler = tokio::task::spawn_blocking(move || gecmis::oturumlari_listele(&dizin))
            .await
            .map_err(ic_hata)??;
        Ok(ozetler.into_iter().map(OturumSatiri::from).collect())
    }

    /// Yerel klasör seçiciyi açar. Seçilen yol yalnızca bu çalıştırmada
    /// geçerli bir kimlikle arayüze verilir (1c tasarımı §5.1).
    pub async fn calisma_alani_sec(&self) -> Option<CalismaAlani> {
        let yol = self.iletisim.klasor_sec("Çalışma alanı seçin").await?;
        let mut ic = self.ic();
        ic.son_alan += 1;
        let kimlik = ic.son_alan;
        ic.calisma_alanlari.insert(kimlik, yol.clone());
        Some(CalismaAlani {
            kimlik,
            yol: yol.display().to_string(),
        })
    }

    pub async fn oturum_baslat(
        &self,
        calisma_alani: u64,
        model: String,
    ) -> Result<OturumAcilisi, KomutHatasi> {
        if !modeller::bilinen_model_mi(&model) {
            return Err(KomutHatasi::gecersiz(format!("Bilinmeyen model: {model}")));
        }
        let kok = self
            .ic()
            .calisma_alanlari
            .get(&calisma_alani)
            .cloned()
            .ok_or_else(|| {
                KomutHatasi::gecersiz("Çalışma alanı seçilmemiş. Klasörü yeniden seçin.")
            })?;
        let baglam = self.baglam(&kok)?;
        std::fs::create_dir_all(&self.oturum_dizini)?;
        let oturum = OturumId::yeni();
        let yol = dizin::kayit_yolu(&self.oturum_dizini, oturum);
        let mut ayarlar = OturumAyarlari::yeni(model.clone(), &kok);
        ayarlar.kayit_yolu = Some(yol.clone());
        let (tutamac, alici) = oturum::baslat_kimlikle(oturum, ayarlar, baglam)?;
        self.tabloya_ekle(
            oturum,
            AcikOturum {
                tutamac,
                alici: Some(alici),
                kopru: None,
                kip: Kip::Siki,
                kip_kilidi: Arc::new(AsyncMutex::new(())),
                yol,
                calisma_alani: kok.clone(),
            },
        );
        Ok(OturumAcilisi {
            oturum,
            calisma_alani: Some(kok.display().to_string()),
            gecmis: GecmisSayfasi {
                ogeler: Vec::new(),
                daha_eski_var: false,
            },
            ozet: DurumOzeti {
                model: Some(model),
                kip: Some(Kip::Siki),
                ..DurumOzeti::default()
            },
            salt_okunur: None,
        })
    }

    /// Oturumu kayıttan açar (1c tasarımı §5.1). Açıksa açık olanın geçmişini
    /// döner. Çalışma alanı yoksa ya da kayıtta yazmıyorsa oturum çekirdekte
    /// başlatılmaz; yalnızca geçmiş gösterilir.
    pub async fn oturum_ac(&self, oturum: OturumId) -> Result<OturumAcilisi, KomutHatasi> {
        self.kapanmayi_bekle(oturum).await;
        if let Some((yol, kok)) = self.acik_oturum(oturum) {
            return self.acilis(oturum, yol, Some(kok), None).await;
        }
        let yol = dizin::kayit_yolu(&self.oturum_dizini, oturum);
        let okunacak = yol.clone();
        let kayitlar = tokio::task::spawn_blocking(move || OturumKaydi::oku(&okunacak))
            .await
            .map_err(ic_hata)?
            .map_err(KomutHatasi::kayit)?;
        let durum = kurtar(&kayitlar).map_err(ic_hata)?;
        if durum.oturum != oturum {
            return Err(KomutHatasi::gecersiz(
                "Kayıt dosyasının adı içindeki oturum kimliğiyle uyuşmuyor.",
            ));
        }
        let Some(kok) = durum.calisma_alani.clone() else {
            let neden = SaltOkunurNedeni::CalismaAlaniKayitsiz;
            return self.acilis(oturum, yol, None, Some(neden)).await;
        };
        if !kok.is_dir() {
            let neden = SaltOkunurNedeni::CalismaAlaniYok {
                yol: kok.display().to_string(),
            };
            return self.acilis(oturum, yol, Some(kok), Some(neden)).await;
        }
        let baglam = self.baglam(&kok)?;
        let ayarlar = OturumAyarlari::yeni(durum.model.clone(), &kok);
        let (tutamac, alici, kurtarilan) = match oturum::devam_et(&yol, ayarlar, baglam) {
            Ok(v) => v,
            Err(e) => {
                // Aynı oturumu eşzamanlı bir çağrı (ör. çift tıklama) açtıysa
                // kayıt kilidi onda; açılmış oturumun geçmişi döner.
                if let Some((yol, kok)) = self.acik_oturum(oturum) {
                    return self.acilis(oturum, yol, Some(kok), None).await;
                }
                return Err(e.into());
            }
        };
        self.tabloya_ekle(
            oturum,
            AcikOturum {
                tutamac,
                alici: Some(alici),
                kopru: None,
                kip: kurtarilan.kip,
                kip_kilidi: Arc::new(AsyncMutex::new(())),
                yol: yol.clone(),
                calisma_alani: kok.clone(),
            },
        );
        self.acilis(oturum, yol, Some(kok), None).await
    }

    fn acik_oturum(&self, oturum: OturumId) -> Option<(PathBuf, PathBuf)> {
        self.ic()
            .oturumlar
            .get(&oturum)
            .map(|a| (a.yol.clone(), a.calisma_alani.clone()))
    }

    /// Kaydın son `SAYFA_TURU` turu ve durum çubuğunun başlangıç değerleri.
    async fn acilis(
        &self,
        oturum: OturumId,
        yol: PathBuf,
        kok: Option<PathBuf>,
        salt_okunur: Option<SaltOkunurNedeni>,
    ) -> Result<OturumAcilisi, KomutHatasi> {
        let (ilk_sayfa, ozet) = tokio::task::spawn_blocking(move || -> io::Result<_> {
            let kayitlar = OturumKaydi::oku(&yol)?;
            let ogeler = gecmis::gorunum_uret(&kayitlar);
            Ok((
                gecmis::sayfa(&ogeler, None, SAYFA_TURU),
                gecmis::durum_ozeti(&kayitlar),
            ))
        })
        .await
        .map_err(ic_hata)??;
        self.ic().son_oturum = Some(oturum);
        Ok(OturumAcilisi {
            oturum,
            calisma_alani: kok.map(|k| k.display().to_string()),
            gecmis: ilk_sayfa,
            ozet,
            salt_okunur,
        })
    }

    /// `once_sira`'dan önceki `SAYFA_TURU` tur. Kayıt açıkken de okunabilir:
    /// kilit kaydın yanındaki dosyadadır (1c tasarımı §5.2).
    pub async fn gecmis_sayfasi(
        &self,
        oturum: OturumId,
        once_sira: u64,
    ) -> Result<GecmisSayfasi, KomutHatasi> {
        let yol = dizin::kayit_yolu(&self.oturum_dizini, oturum);
        tokio::task::spawn_blocking(move || -> io::Result<GecmisSayfasi> {
            let kayitlar = OturumKaydi::oku(&yol)?;
            Ok(gecmis::sayfa(
                &gecmis::gorunum_uret(&kayitlar),
                Some(once_sira),
                SAYFA_TURU,
            ))
        })
        .await
        .map_err(ic_hata)?
        .map_err(KomutHatasi::kayit)
    }

    /// Oturumun olaylarını paketleyip `yazici`ya yazan köprüyü başlatır.
    /// Açılan oturum başına bir kez çağrılır.
    pub fn oturumu_dinle(
        &self,
        oturum: OturumId,
        yazici: Box<dyn KanalYazici>,
    ) -> Result<(), KomutHatasi> {
        let mut ic = self.ic();
        let acik = ic
            .oturumlar
            .get_mut(&oturum)
            .ok_or_else(KomutHatasi::oturum_yok)?;
        let alici = acik
            .alici
            .take()
            .ok_or_else(|| KomutHatasi::gecersiz("Bu oturum zaten dinleniyor."))?;
        // Uçtan uca test derlemesi kuyruğu ve yoldaki veriyi ölçer (1c
        // tasarımı §11.4).
        #[cfg(feature = "e2e")]
        let alici = crate::e2e::OlcumluKaynak(alici);
        let (paketleyici, kredi) = Paketleyici::yeni(alici, self.aktarim);
        #[cfg(feature = "e2e")]
        let yazici: Box<dyn KanalYazici> =
            Box::new(crate::e2e::OlcumluYazici::yeni(yazici, kredi.clone()));
        let gorev = tokio::spawn(kopru::calistir(paketleyici, yazici));
        acik.kopru = Some((gorev, kredi));
        Ok(())
    }

    pub fn paketi_isledim(&self, oturum: OturumId, no: u64) -> Result<(), KomutHatasi> {
        let kredi = self
            .ic()
            .oturumlar
            .get(&oturum)
            .and_then(|a| a.kopru.as_ref().map(|(_, kredi)| kredi.clone()))
            .ok_or_else(KomutHatasi::oturum_yok)?;
        kredi
            .isledim(no)
            .map_err(|e| KomutHatasi::gecersiz(e.to_string()))
    }

    fn tutamac(&self, oturum: OturumId) -> Result<OturumTutamaci, KomutHatasi> {
        self.ic()
            .oturumlar
            .get(&oturum)
            .map(|a| a.tutamac.clone())
            .ok_or_else(KomutHatasi::oturum_yok)
    }

    pub async fn mesaj_gonder(&self, oturum: OturumId, metin: String) -> Result<(), KomutHatasi> {
        let tutamac = self.tutamac(oturum)?;
        self.ic().son_oturum = Some(oturum);
        tutamac.gonder(Komut::KullaniciMesaji { metin }).await?;
        Ok(())
    }

    pub fn turu_iptal_et(&self, oturum: OturumId) -> Result<(), KomutHatasi> {
        self.tutamac(oturum)?.iptal_et();
        Ok(())
    }

    /// Onay kararlarını çekirdeğe iletir. Yerel onay isteyen bir öğeyi
    /// arayüz onayladıysa Rust komutu yerel iletişim kutusunda yeniden sorar
    /// (aşama 2 tasarımı §5.2). Kutuda gösterilen komut arayüzden değil
    /// çekirdekten okunur; webview'a sızmış bir betik başka bir komutu
    /// onaylatamaz.
    pub async fn arac_onayi(
        &self,
        oturum: OturumId,
        kararlar: Vec<OnayGirdisi>,
    ) -> Result<(), KomutHatasi> {
        let tutamac = self.tutamac(oturum)?;
        let yereller: Vec<OnayOgesi> = kararlar
            .iter()
            .filter(|k| k.karar == OnayKarari::Onayla)
            .filter_map(|k| tutamac.bekleyen_onay(k.cagri))
            .filter(|o| o.yerel_onay)
            .collect();
        let yerel_evet = !yereller.is_empty()
            && self
                .iletisim
                .onayla(
                    "Komutu yalıtımsız çalıştır",
                    &guvenlik::yerel_onay_metni(&yereller),
                    "Çalıştır",
                    "Vazgeç",
                )
                .await;
        for k in kararlar {
            let komut = match yereller.iter().find(|o| o.cagri == k.cagri) {
                Some(o) if yerel_evet => Komut::YerelOnay {
                    cagri: o.cagri,
                    muhur: o.muhur.clone(),
                },
                Some(o) => Komut::AracOnayi {
                    cagri: o.cagri,
                    karar: OnayKarari::Reddet {
                        gerekce: Some("Kullanıcı yerel pencerede vazgeçti.".into()),
                    },
                    muhur: o.muhur.clone(),
                },
                None => Komut::AracOnayi {
                    cagri: k.cagri,
                    karar: k.karar,
                    muhur: k.muhur,
                },
            };
            tutamac.gonder(komut).await?;
        }
        Ok(())
    }

    pub async fn model_degistir(&self, oturum: OturumId, model: String) -> Result<(), KomutHatasi> {
        if !modeller::bilinen_model_mi(&model) {
            return Err(KomutHatasi::gecersiz(format!("Bilinmeyen model: {model}")));
        }
        self.tutamac(oturum)?
            .gonder(Komut::ModelDegistir { model })
            .await?;
        Ok(())
    }

    /// İzin kipini değiştirir. Yükseltme yerel iletişim kutusunda onaylanır;
    /// reddedilirse kip değişmez ve eski kip döner (1c tasarımı §6).
    pub async fn izin_kipi_degistir(&self, oturum: OturumId, kip: Kip) -> Result<Kip, KomutHatasi> {
        let kip_kilidi = self
            .ic()
            .oturumlar
            .get(&oturum)
            .map(|acik| acik.kip_kilidi.clone())
            .ok_or_else(KomutHatasi::oturum_yok)?;
        let _kip_nobeti = kip_kilidi.lock().await;
        let (tutamac, eski) = {
            let ic = self.ic();
            let acik = ic
                .oturumlar
                .get(&oturum)
                .ok_or_else(KomutHatasi::oturum_yok)?;
            (acik.tutamac.clone(), acik.kip)
        };
        if kip == eski {
            return Ok(eski);
        }
        if guvenlik::yukseltme_mi(eski, kip) {
            let mesaj = format!(
                "İzin kipi {} yerine {} olacak.\n\n{}",
                guvenlik::kip_adi(eski),
                guvenlik::kip_adi(kip),
                guvenlik::yukseltme_aciklamasi(kip)
            );
            let onay = self
                .iletisim
                .onayla("İzin kipini yükselt", &mesaj, "Yükselt", "Vazgeç")
                .await;
            if !onay {
                return Ok(eski);
            }
        }
        tutamac.gonder(Komut::IzinKipiDegistir { kip }).await?;
        if let Some(acik) = self.ic().oturumlar.get_mut(&oturum) {
            acik.kip = kip;
        }
        Ok(kip)
    }

    /// Oturumu kapatır: süren tur iptal edilir, olay alıcısı bırakılır, kayıt
    /// kilidi çözülür (1c tasarımı §5.1).
    pub async fn oturumu_kapat(&self, oturum: OturumId) -> Result<(), KomutHatasi> {
        let tutamac = self
            .kapatmaya_basla(oturum)
            .ok_or_else(KomutHatasi::oturum_yok)?;
        {
            let mut ic = self.ic();
            if ic.son_oturum == Some(oturum) {
                ic.son_oturum = None;
            }
        }
        self.kapanisi_bekle(oturum, tutamac).await;
        Ok(())
    }

    /// Ana pencere yeniden yüklendi ya da kapandı: ona bağlı bütün oturumlar
    /// kapanır (1c tasarımı §5.6). Tablo hemen boşalır; yeni sayfa aynı oturumu
    /// açarsa önce kapanış beklenir. Görevlerin bitmesini `kapanislari_bekle`
    /// bekler.
    pub fn ana_pencere_gitti(&self) -> Vec<(OturumId, OturumTutamaci)> {
        let kimlikler: Vec<OturumId> = self.ic().oturumlar.keys().copied().collect();
        kimlikler
            .into_iter()
            .filter_map(|oturum| self.kapatmaya_basla(oturum).map(|t| (oturum, t)))
            .collect()
    }

    pub async fn kapanislari_bekle(&self, kapananlar: Vec<(OturumId, OturumTutamaci)>) {
        for (oturum, tutamac) in kapananlar {
            self.kapanisi_bekle(oturum, tutamac).await;
        }
    }

    /// Kapanmakta olan oturum var mı. Son pencere kapanınca süreç bunların
    /// kaydı tamamlanmadan çıkmaz (1c tasarımı §5.6).
    pub fn kapanan_oturum_var(&self) -> bool {
        !self.ic().kapaniyor.is_empty()
    }

    /// Kapanmakta olan bütün oturumların görevlerini bekler.
    pub async fn kapananlari_bekle(&self) {
        let kapananlar: Vec<(OturumId, OturumTutamaci)> = self
            .ic()
            .kapaniyor
            .iter()
            .map(|(oturum, tutamac)| (*oturum, tutamac.clone()))
            .collect();
        self.kapanislari_bekle(kapananlar).await;
    }

    fn kapatmaya_basla(&self, oturum: OturumId) -> Option<OturumTutamaci> {
        let acik = {
            let mut ic = self.ic();
            let acik = ic.oturumlar.remove(&oturum)?;
            ic.kapaniyor.insert(oturum, acik.tutamac.clone());
            acik
        };
        acik.tutamac.iptal_et();
        if let Some((gorev, _)) = &acik.kopru {
            // Paketleyici ve içindeki olay alıcısı bırakılır; oturum görevi
            // olay kuyruğunun kapandığını görüp biter.
            gorev.abort();
        }
        Some(acik.tutamac)
    }

    async fn kapanisi_bekle(&self, oturum: OturumId, tutamac: OturumTutamaci) {
        let _ = tokio::time::timeout(KAPANIS_SURESI, tutamac.kapanmasini_bekle()).await;
        self.ic().kapaniyor.remove(&oturum);
    }

    async fn kapanmayi_bekle(&self, oturum: OturumId) {
        let tutamac = self.ic().kapaniyor.get(&oturum).cloned();
        if let Some(tutamac) = tutamac {
            let _ = tokio::time::timeout(KAPANIS_SURESI, tutamac.kapanmasini_bekle()).await;
        }
    }

    fn tabloya_ekle(&self, oturum: OturumId, acik: AcikOturum) {
        let mut ic = self.ic();
        ic.son_oturum = Some(oturum);
        ic.oturumlar.insert(oturum, acik);
    }

    /// Çalışma alanının araçları ve ortak kilitleriyle oturum bağlamı.
    pub(crate) fn baglam(&self, kok: &Path) -> Result<OturumBaglami, KomutHatasi> {
        let siniflandirici = Arc::new(Siniflandirici::yeni(kok, &[]).map_err(ic_hata)?);
        let alan = AracAlani::ac(siniflandirici.clone())?;
        let guven = Arc::new(AlanGuveni {
            kayit: self.alanlar.clone(),
            kimlik: CalismaAlaniKaydi::kimlik(kok),
        });
        let mut araclar = varsayilan_araclar(alan.clone());
        araclar.push(Arc::new(KabukAraci::yeni(
            alan,
            self.yalitici.clone(),
            guven,
        )));
        let araclar = Arc::new(AracKayitDefteri::yeni(araclar).map_err(ic_hata)?);
        Ok(OturumBaglami {
            saglayici: self.evren.clone(),
            araclar,
            siniflandirici,
            kilitler: self.kilitler.al(kok)?,
            sart_kapisi: self.kapi.clone(),
        })
    }

    /// Bağlantıyı sistem tarayıcısında açar: yalnızca http/https ve yalnızca
    /// kullanıcı yerel iletişim kutusunda onaylarsa (1c tasarımı §6).
    pub async fn baglanti_ac(&self, adres: String) -> Result<(), KomutHatasi> {
        let url = guvenlik::acilabilir_adres(&adres)
            .ok_or_else(|| KomutHatasi::gecersiz("Yalnızca http ve https adresleri açılabilir."))?;
        let mesaj = format!("Bu adres sistem tarayıcısında açılacak:\n\n{url}");
        if self
            .iletisim
            .onayla("Bağlantıyı aç", &mesaj, "Aç", "Vazgeç")
            .await
        {
            self.acici.ac(&url).map_err(KomutHatasi::ic)?;
        }
        Ok(())
    }
}
