//! Oturum tablosu (1c tasarımı §5): açık oturumlar, kayıttan açma, köprü,
//! kapatma. Aynı oturum ikinci kez açılmaz; açık olanın geçmişi döner.

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use kalfa_core::aktarim::{KrediTutamaci, Paketleyici};
use kalfa_core::arac::AracKayitDefteri;
use kalfa_core::butce::{ButceDurumu, ButceGorunumu};
use kalfa_core::gecmis::{self, DurumOzeti, GecmisSayfasi};
use kalfa_core::geri_alma::GeriYukleyici;
use kalfa_core::gonderim::{GunlukSayfasi, gunluk_sayfasi, gunluk_uret, oge_metni};
use kalfa_core::izin::{Kip, Siniflandirici};
use kalfa_core::kayit::OturumKaydi;
use kalfa_core::kimlik::{CagriId, IslemId, OturumId, TurId};
use kalfa_core::kurtarma::kurtar;
use kalfa_core::muhur::OnayMuhru;
use kalfa_core::oturum::{self, OlayAlici, OturumAyarlari, OturumBaglami, OturumTutamaci};
use kalfa_core::plan::PlanGuncelleAraci;
use kalfa_core::proje_talimatlari::ProjeTalimatOkuyucu;
use kalfa_core::protokol::{Komut, OnayKarari, OnayOgesi};
use kalfa_core::saglayici::Saglayici;
use kalfa_core::soru::{KullaniciSorusuYaniti, KullaniciyaSorAraci};
use kalfa_evren::tipler::DusunceEforu;
use kalfa_tools::{CalismaAlani as AracAlani, KabukAraci, varsayilan_araclar};
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinHandle;

use crate::arayuz::KanalYazici;
use crate::calisma_alanlari::{AlanGuveni, CalismaAlaniKaydi};
use crate::dosya_gorunumu::DosyaGorunumu;
use crate::hata::KomutHatasi;
use crate::proje_talimatlari::CalismaAlaniTalimatOkuyucu;
use crate::sozlesme::{
    CalismaAlani, ModelSecenegi, OnayGirdisi, OturumAcilisi, OturumSatiri, SaltOkunurNedeni,
    UygulamaDurumu,
};
use crate::{Uygulama, dizin, guvenlik, kopru, modeller};

/// Açılışta ve her eski sayfada yüklenen tur sayısı (1c tasarımı §9.3).
pub const SAYFA_TURU: usize = 20;

/// Kapatılan oturumun görevinin bitmesi için beklenen en uzun süre.
const KAPANIS_SURESI: Duration = Duration::from_secs(5);

/// Varsayılan proje klasörü, ev dizininde (aşama 2 tasarımı §9.8). OneDrive'a
/// yönlendirilen `Belgeler` ya da `Masaüstü`'nün altında değildir.
pub const PROJE_KLASORU: &str = "Kalfa Projeleri";

pub(crate) struct AcikOturum {
    tutamac: OturumTutamaci,
    /// `oturumu_dinle` çağrılana kadar burada durur.
    alici: Option<OlayAlici>,
    kopru: Option<(JoinHandle<()>, KrediTutamaci)>,
    model: String,
    efor: DusunceEforu,
    kip: Kip,
    kip_kilidi: Arc<AsyncMutex<()>>,
    yol: PathBuf,
    pub(crate) calisma_alani: PathBuf,
    pub(crate) calisma_alani_kimligi: String,
    /// Sağ panelin dosya görünümü; kimlik tablosu oturumla silinir.
    pub(crate) gorunum: Arc<DosyaGorunumu>,
}

pub(crate) struct KapananOturum {
    tutamac: OturumTutamaci,
    pub(crate) calisma_alani_kimligi: String,
}

/// Oturumu silmenin yerel onay metni (aşama 3 tasarımı §5.10).
pub const OTURUM_SILME_METNI: &str = "Oturum, kaydı ve geri alma verisi kalıcı olarak silinecek. Bu oturumun turları artık geri alınamaz.";

const ALAN_KALDIRILIYOR: &str =
    "Bu çalışma alanının yalıtım izinleri kaldırılıyor. İşlem bitince yeniden deneyin.";

/// Dizindeki dosyaların toplam boyutu; bağlantılar izlenmez. Dizin yoksa 0.
fn dizin_boyutu(dizin: &Path) -> io::Result<u64> {
    let girdiler = match std::fs::read_dir(dizin) {
        Ok(g) => g,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e),
    };
    let mut toplam = 0;
    for girdi in girdiler {
        let girdi = girdi?;
        let tur = girdi.file_type()?;
        if tur.is_dir() {
            toplam += dizin_boyutu(&girdi.path())?;
        } else if tur.is_file() {
            toplam += girdi.metadata()?.len();
        }
    }
    Ok(toplam)
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
            modeller: ic.modeller.clone(),
            varsayilan_model: modeller::varsayilan_model(&ic.modeller),
            varsayilan_efor: modeller::VARSAYILAN_EFOR,
            son_oturum: ic.son_oturum,
            yalitim: self.yalitici.durum(),
            yalitimsiz_izinli: self.yalitimsiz_izinli(),
            platform: std::env::consts::OS.to_string(),
        }
    }

    /// EVREN `/v1/models` uç noktasından sohbet modellerini ve efor düzeylerini
    /// yeniden çeker.
    pub async fn modelleri_yenile(&self) -> Result<Vec<ModelSecenegi>, KomutHatasi> {
        let katalog = self.evren.modeller().await?;
        let liste = modeller::katalogdan_modeller(&katalog);
        self.ic().modeller = liste.clone();
        Ok(liste)
    }

    pub(crate) async fn modelleri_guncelle(&self) {
        if let Ok(katalog) = self.evren.modeller().await {
            self.ic().modeller = modeller::katalogdan_modeller(&katalog);
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
    /// geçerli bir kimlikle arayüze verilir (1c tasarımı §5.1). Seçici ev
    /// dizinindeki "Kalfa Projeleri"nde açılır; klasör yoksa oluşturulur,
    /// oluşturulamazsa başlangıç verilmez (aşama 2 tasarımı §9.8).
    pub async fn calisma_alani_sec(&self) -> Option<CalismaAlani> {
        let projeler = self.ev_dizini.join(PROJE_KLASORU);
        let baslangic = std::fs::create_dir_all(&projeler).ok().map(|()| projeler);
        let yol = self
            .iletisim
            .klasor_sec("Çalışma alanı seçin", baslangic.as_deref())
            .await?;
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
        let (gecerli, efor) = {
            let ic = self.ic();
            (
                modeller::model_gecerli_mi(&ic.modeller, &model),
                modeller::uygun_efor(&ic.modeller, &model, modeller::VARSAYILAN_EFOR),
            )
        };
        if !gecerli {
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
        std::fs::create_dir_all(&self.oturum_dizini)?;
        let oturum = OturumId::yeni();
        let (baglam, gorunum) = self.baglam(oturum, &kok).await?;
        let yol = dizin::kayit_yolu(&self.oturum_dizini, oturum);
        let mut ayarlar = OturumAyarlari::yeni(model.clone(), &kok);
        ayarlar.efor = Some(efor);
        ayarlar.kayit_yolu = Some(yol.clone());
        ayarlar.geri_alma_dizini = Some(dizin::oturum_geri_alma_dizini(
            &self.geri_alma_dizini,
            oturum,
        ));
        ayarlar.sinirlar.etkin_pencere = self.ayarlar.etkin_pencere();
        let butce = ButceDurumu::yeni().gorunum();
        let (tutamac, alici) = oturum::baslat_kimlikle(oturum, ayarlar, baglam)?;
        let acik = AcikOturum {
            tutamac,
            alici: Some(alici),
            kopru: None,
            model: model.clone(),
            efor,
            kip: Kip::Siki,
            kip_kilidi: Arc::new(AsyncMutex::new(())),
            yol,
            calisma_alani: kok.clone(),
            calisma_alani_kimligi: CalismaAlaniKaydi::kimlik(&kok),
            gorunum,
        };
        if let Err(acik) = self.tabloya_ekle(oturum, acik) {
            self.eklenemeyen_oturumu_kapat(oturum, acik).await;
            return Err(KomutHatasi::gecersiz(ALAN_KALDIRILIYOR));
        }
        Ok(OturumAcilisi {
            oturum,
            calisma_alani_kimligi: Some(CalismaAlaniKaydi::kimlik(&kok)),
            calisma_alani: Some(kok.display().to_string()),
            gecmis: GecmisSayfasi {
                ogeler: Vec::new(),
                daha_eski_var: false,
            },
            ozet: DurumOzeti {
                model: Some(model),
                efor: Some(efor),
                kip: Some(Kip::Siki),
                ..DurumOzeti::default()
            },
            salt_okunur: None,
            butce: Some(butce),
        })
    }

    /// Oturumu kayıttan açar (1c tasarımı §5.1). Açıksa açık olanın geçmişini
    /// döner. Çalışma alanı yoksa ya da kayıtta yazmıyorsa oturum çekirdekte
    /// başlatılmaz; yalnızca geçmiş gösterilir.
    pub async fn oturum_ac(&self, oturum: OturumId) -> Result<OturumAcilisi, KomutHatasi> {
        self.kapanmayi_bekle(oturum).await;
        if let Some((yol, kok)) = self.acik_oturum(oturum) {
            return self.acilis(oturum, yol, Some(kok), None, None).await;
        }
        let yol = dizin::kayit_yolu(&self.oturum_dizini, oturum);
        let okunacak = yol.clone();
        let kayitlar = tokio::task::spawn_blocking(move || OturumKaydi::oku(&okunacak))
            .await
            .map_err(ic_hata)?
            .map_err(KomutHatasi::kayit)?;
        let durum = kurtar(&kayitlar).map_err(ic_hata)?;
        let butce = Some(durum.butce);
        if durum.oturum != oturum {
            return Err(KomutHatasi::gecersiz(
                "Kayıt dosyasının adı içindeki oturum kimliğiyle uyuşmuyor.",
            ));
        }
        let Some(kok) = durum.calisma_alani.clone() else {
            let neden = SaltOkunurNedeni::CalismaAlaniKayitsiz;
            return self.acilis(oturum, yol, None, Some(neden), butce).await;
        };
        if !kok.is_dir() {
            let neden = SaltOkunurNedeni::CalismaAlaniYok {
                yol: kok.display().to_string(),
            };
            return self
                .acilis(oturum, yol, Some(kok), Some(neden), butce)
                .await;
        }
        let (baglam, gorunum) = self.baglam(oturum, &kok).await?;
        let mut ayarlar = OturumAyarlari::yeni(durum.model.clone(), &kok);
        let baslangic_efor = {
            let ic = self.ic();
            modeller::uygun_efor(
                &ic.modeller,
                &durum.model,
                durum.efor.unwrap_or(modeller::VARSAYILAN_EFOR),
            )
        };
        ayarlar.efor = Some(baslangic_efor);
        ayarlar.sinirlar.etkin_pencere = self.ayarlar.etkin_pencere();
        ayarlar.geri_alma_dizini = Some(dizin::oturum_geri_alma_dizini(
            &self.geri_alma_dizini,
            oturum,
        ));
        let (tutamac, alici, kurtarilan) = match oturum::devam_et(&yol, ayarlar, baglam) {
            Ok(v) => v,
            Err(e) => {
                // Aynı oturumu eşzamanlı bir çağrı (ör. çift tıklama) açtıysa
                // kayıt kilidi onda; açılmış oturumun geçmişi döner.
                if let Some((yol, kok)) = self.acik_oturum(oturum) {
                    return self.acilis(oturum, yol, Some(kok), None, None).await;
                }
                return Err(e.into());
            }
        };
        let acik = AcikOturum {
            tutamac,
            alici: Some(alici),
            kopru: None,
            model: kurtarilan.model.clone(),
            efor: kurtarilan.efor.unwrap_or(baslangic_efor),
            kip: kurtarilan.kip,
            kip_kilidi: Arc::new(AsyncMutex::new(())),
            yol: yol.clone(),
            calisma_alani: kok.clone(),
            calisma_alani_kimligi: CalismaAlaniKaydi::kimlik(&kok),
            gorunum,
        };
        if let Err(acik) = self.tabloya_ekle(oturum, acik) {
            self.eklenemeyen_oturumu_kapat(oturum, acik).await;
            return Err(KomutHatasi::gecersiz(ALAN_KALDIRILIYOR));
        }
        self.acilis(oturum, yol, Some(kok), None, butce).await
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
        butce: Option<ButceGorunumu>,
    ) -> Result<OturumAcilisi, KomutHatasi> {
        self.isler.oturumu_kurtar(oturum);
        let (ilk_sayfa, mut ozet) = tokio::task::spawn_blocking(move || -> io::Result<_> {
            let kayitlar = OturumKaydi::oku(&yol)?;
            let ogeler = gecmis::gorunum_uret(&kayitlar);
            Ok((
                gecmis::sayfa(&ogeler, None, SAYFA_TURU),
                gecmis::durum_ozeti(&kayitlar),
            ))
        })
        .await
        .map_err(ic_hata)??;
        {
            let mut ic = self.ic();
            ic.son_oturum = Some(oturum);
            if let Some(acik) = ic.oturumlar.get(&oturum) {
                ozet.model = Some(acik.model.clone());
                ozet.efor = Some(acik.efor);
                ozet.kip = Some(acik.kip);
            } else if let Some(model) = &ozet.model {
                let efor = ozet.efor.unwrap_or(modeller::VARSAYILAN_EFOR);
                ozet.efor = Some(modeller::uygun_efor(&ic.modeller, model, efor));
            }
        }
        Ok(OturumAcilisi {
            oturum,
            calisma_alani_kimligi: kok.as_deref().map(CalismaAlaniKaydi::kimlik),
            calisma_alani: kok.map(|k| k.display().to_string()),
            gecmis: ilk_sayfa,
            ozet,
            salt_okunur,
            butce,
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

    /// `once_sira`'dan önceki gönderim günlüğü sayfası (aşama 3b).
    pub async fn gonderim_gunlugu(
        &self,
        oturum: OturumId,
        once_sira: Option<u64>,
    ) -> Result<GunlukSayfasi, KomutHatasi> {
        let yol = dizin::kayit_yolu(&self.oturum_dizini, oturum);
        tokio::task::spawn_blocking(move || -> io::Result<GunlukSayfasi> {
            let kayitlar = OturumKaydi::oku(&yol)?;
            let satirlar = gunluk_uret(&kayitlar);
            Ok(gunluk_sayfasi(&satirlar, once_sira, SAYFA_TURU))
        })
        .await
        .map_err(ic_hata)?
        .map_err(KomutHatasi::kayit)
    }

    /// Kayıttaki bir sıranın tam metnini döndürür (aşama 3b).
    pub async fn gonderim_ogesi(&self, oturum: OturumId, sira: u64) -> Result<String, KomutHatasi> {
        let yol = dizin::kayit_yolu(&self.oturum_dizini, oturum);
        tokio::task::spawn_blocking(move || -> Result<String, KomutHatasi> {
            let kayitlar = OturumKaydi::oku(&yol).map_err(KomutHatasi::kayit)?;
            oge_metni(&kayitlar, sira).ok_or_else(|| KomutHatasi::gecersiz("Öğe bulunamadı."))
        })
        .await
        .map_err(ic_hata)?
    }

    /// Göndermeden önce göster ayarını değiştirir (aşama 3b).
    pub async fn onizleme_ayarla(&self, oturum: OturumId, acik: bool) -> Result<(), KomutHatasi> {
        self.tutamac(oturum)?
            .gonder(Komut::OnizlemeAyarla { acik })
            .await?;
        Ok(())
    }

    /// Oturumdaki bir "bu oturumda izin ver" kısayolunu kaldırır (3e).
    pub async fn kisayolu_kaldir(&self, oturum: OturumId, kimlik: u64) -> Result<(), KomutHatasi> {
        self.tutamac(oturum)?
            .gonder(Komut::KisayoluKaldir { kimlik })
            .await?;
        Ok(())
    }

    /// Göndermeden önce göster kararını iletir (aşama 3b). Eklenti isteğinde
    /// "Bu araç çağrısında hep gönder" `hep_gonderilecekler` ile gelir (aşama 4
    /// tasarımı §9.4).
    pub async fn gonderim_karari(
        &self,
        oturum: OturumId,
        islem: IslemId,
        gonderilmeyecekler: Vec<CagriId>,
        hep_gonderilecekler: Vec<CagriId>,
    ) -> Result<(), KomutHatasi> {
        self.tutamac(oturum)?
            .gonder(Komut::GonderimKarari {
                islem,
                gonderilmeyecekler,
                hep_gonderilecekler,
            })
            .await?;
        Ok(())
    }

    /// Açık oturumun çalışma sınırları.
    pub fn oturum_sinirlari(&self, oturum: OturumId) -> Option<oturum::Sinirlar> {
        self.ic()
            .oturumlar
            .get(&oturum)
            .map(|a| a.tutamac.sinirlar().clone())
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

    pub(crate) fn tutamac(&self, oturum: OturumId) -> Result<OturumTutamaci, KomutHatasi> {
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

    pub async fn oturum_ozetle(
        &self,
        oturum: OturumId,
        talimat: Option<String>,
    ) -> Result<(), KomutHatasi> {
        let tutamac = self.tutamac(oturum)?;
        self.ic().son_oturum = Some(oturum);
        tutamac.gonder(Komut::Ozetle { talimat }).await?;
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
                    guvenlik::yerel_onay_basligi(&yereller),
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
                    kisayol: false,
                },
                None => Komut::AracOnayi {
                    cagri: k.cagri,
                    karar: k.karar,
                    muhur: k.muhur,
                    kisayol: k.kisayol,
                },
            };
            tutamac.gonder(komut).await?;
        }
        Ok(())
    }

    pub async fn kullanici_sorusu_yaniti(
        &self,
        oturum: OturumId,
        tur: TurId,
        soru_kimligi: String,
        yanit: KullaniciSorusuYaniti,
    ) -> Result<(), KomutHatasi> {
        let tutamac = self.tutamac(oturum)?;
        self.ic().son_oturum = Some(oturum);
        tutamac
            .gonder(Komut::KullaniciSorusuYaniti {
                tur,
                soru_kimligi,
                yanit,
            })
            .await?;
        Ok(())
    }

    pub async fn model_degistir(&self, oturum: OturumId, model: String) -> Result<(), KomutHatasi> {
        let (tutamac, yeni_efor) = {
            let mut kilit = self.ic();
            let ic = &mut *kilit;
            if !modeller::model_gecerli_mi(&ic.modeller, &model) {
                return Err(KomutHatasi::gecersiz(format!("Bilinmeyen model: {model}")));
            }
            let acik = ic
                .oturumlar
                .get_mut(&oturum)
                .ok_or_else(KomutHatasi::oturum_yok)?;
            let uygun = modeller::uygun_efor(&ic.modeller, &model, acik.efor);
            let degisti = (uygun != acik.efor).then_some(uygun);
            acik.model = model.clone();
            acik.efor = uygun;
            (acik.tutamac.clone(), degisti)
        };
        tutamac.gonder(Komut::ModelDegistir { model }).await?;
        if let Some(efor) = yeni_efor {
            tutamac.gonder(Komut::EforDegistir { efor }).await?;
        }
        Ok(())
    }

    pub async fn efor_degistir(
        &self,
        oturum: OturumId,
        efor: DusunceEforu,
    ) -> Result<(), KomutHatasi> {
        let tutamac = {
            let mut kilit = self.ic();
            let ic = &mut *kilit;
            let acik = ic
                .oturumlar
                .get_mut(&oturum)
                .ok_or_else(KomutHatasi::oturum_yok)?;
            if modeller::uygun_efor(&ic.modeller, &acik.model, efor) != efor {
                return Err(KomutHatasi::gecersiz(format!(
                    "Seçili model bu efor düzeyini desteklemiyor: {efor:?}"
                )));
            }
            acik.efor = efor;
            acik.tutamac.clone()
        };
        tutamac.gonder(Komut::EforDegistir { efor }).await?;
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
        let (tutamac, eski, alan_kimligi, alan_yolu) = {
            let ic = self.ic();
            let acik = ic
                .oturumlar
                .get(&oturum)
                .ok_or_else(KomutHatasi::oturum_yok)?;
            (
                acik.tutamac.clone(),
                acik.kip,
                acik.calisma_alani_kimligi.clone(),
                acik.calisma_alani.clone(),
            )
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
        self.mcp
            .oturum_kipi_guncelle(&alan_kimligi, Some(&alan_yolu), oturum, Some(kip));
        Ok(kip)
    }

    /// Turun geri alma önizlemesini ister; plan `GeriAlmaPlani` olayıyla gelir
    /// (aşama 3 tasarımı §5.6). Arayüz yol göndermez, tur kimliği gönderir.
    pub async fn geri_alma_onizle(&self, oturum: OturumId, tur: TurId) -> Result<(), KomutHatasi> {
        self.tutamac(oturum)?
            .gonder(Komut::GeriAlmaOnizle { tur })
            .await?;
        Ok(())
    }

    /// Önizlenen planı mührüyle yürütür. Onay webview'da verilir: geri alma
    /// yalnızca Kalfa'nın o turdan önce gördüğü içeriği geri koyar (§5.6).
    pub async fn turu_geri_al(
        &self,
        oturum: OturumId,
        tur: TurId,
        muhur: OnayMuhru,
    ) -> Result<(), KomutHatasi> {
        self.tutamac(oturum)?
            .gonder(Komut::TuruGeriAl { tur, muhur })
            .await?;
        Ok(())
    }

    /// Oturumu, kaydını ve geri alma verisini siler (aşama 3 tasarımı §5.10).
    /// Onay yerel penceredir: webview'a sızmış bir betik geri alma olanağını
    /// tek komutla yok edememeli. Açık oturum önce kapatılır; kayıt başka bir
    /// yerde açıksa silinmez. Vazgeçilirse `false`.
    pub async fn oturumu_sil(&self, oturum: OturumId) -> Result<bool, KomutHatasi> {
        let yol = dizin::kayit_yolu(&self.oturum_dizini, oturum);
        std::fs::metadata(&yol).map_err(KomutHatasi::kayit)?;
        let onay = self
            .iletisim
            .onayla("Oturumu sil", OTURUM_SILME_METNI, "Sil", "Vazgeç")
            .await;
        if !onay {
            return Ok(false);
        }
        if self.acik_oturum(oturum).is_some() {
            self.oturumu_kapat(oturum).await?;
        }
        self.kapanmayi_bekle(oturum).await;
        let depo = dizin::oturum_geri_alma_dizini(&self.geri_alma_dizini, oturum);
        tokio::task::spawn_blocking(move || {
            OturumKaydi::sil(&yol, || match std::fs::remove_dir_all(&depo) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            })
        })
        .await
        .map_err(ic_hata)?
        .map_err(KomutHatasi::kayit)?;
        self.isler.oturum_kaydini_sil(oturum);
        let mut ic = self.ic();
        if ic.son_oturum == Some(oturum) {
            ic.son_oturum = None;
        }
        Ok(true)
    }

    /// Bütün oturumların geri alma verisinin toplam boyutu (bayt); Ayarlar
    /// gösterir.
    pub async fn geri_alma_verisi_boyutu(&self) -> Result<u64, KomutHatasi> {
        let kok = self.geri_alma_dizini.clone();
        tokio::task::spawn_blocking(move || dizin_boyutu(&kok))
            .await
            .map_err(ic_hata)?
            .map_err(KomutHatasi::from)
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

    /// Kapanmakta olan bütün oturumların görevlerini, açık arka plan işlerini ve açık MCP süreçlerini bekler.
    pub async fn kapananlari_bekle(&self) {
        let kapananlar: Vec<(OturumId, OturumTutamaci)> = self
            .ic()
            .kapaniyor
            .iter()
            .map(|(oturum, kapanan)| (*oturum, kapanan.tutamac.clone()))
            .collect();
        self.kapanislari_bekle(kapananlar).await;
        self.isler.tumunu_kapat_ve_bekle().await;
        self.mcp.tumunu_kapat_ve_bekle().await;
    }

    fn kapatmaya_basla(&self, oturum: OturumId) -> Option<OturumTutamaci> {
        let acik = {
            let mut ic = self.ic();
            let acik = ic.oturumlar.remove(&oturum)?;
            ic.kapaniyor.insert(
                oturum,
                KapananOturum {
                    tutamac: acik.tutamac.clone(),
                    calisma_alani_kimligi: acik.calisma_alani_kimligi.clone(),
                },
            );
            acik
        };
        self.mcp.oturum_kipi_guncelle(
            &acik.calisma_alani_kimligi,
            Some(&acik.calisma_alani),
            oturum,
            None,
        );
        acik.tutamac.iptal_et();
        if let Some((gorev, _)) = &acik.kopru {
            // Paketleyici ve içindeki olay alıcısı bırakılır; oturum görevi
            // olay kuyruğunun kapandığını görüp biter.
            gorev.abort();
        }
        Some(acik.tutamac)
    }

    async fn kapanisi_bekle(&self, oturum: OturumId, tutamac: OturumTutamaci) {
        self.isler.oturumu_kapat(oturum).await;
        if tokio::time::timeout(KAPANIS_SURESI, tutamac.kapanmasini_bekle())
            .await
            .is_ok()
        {
            self.ic().kapaniyor.remove(&oturum);
        }
    }

    async fn kapanmayi_bekle(&self, oturum: OturumId) {
        let tutamac = self
            .ic()
            .kapaniyor
            .get(&oturum)
            .map(|kapanan| kapanan.tutamac.clone());
        if let Some(tutamac) = tutamac
            && tokio::time::timeout(KAPANIS_SURESI, tutamac.kapanmasini_bekle())
                .await
                .is_ok()
        {
            self.ic().kapaniyor.remove(&oturum);
        }
    }

    fn tabloya_ekle(&self, oturum: OturumId, acik: AcikOturum) -> Result<(), Box<AcikOturum>> {
        let alan_kimligi = acik.calisma_alani_kimligi.clone();
        let alan_yolu = acik.calisma_alani.clone();
        let kip = acik.kip;
        {
            let mut ic = self.ic();
            if ic
                .yalitim_kaldirma_rezervleri
                .contains(&acik.calisma_alani_kimligi)
            {
                return Err(Box::new(acik));
            }
            ic.son_oturum = Some(oturum);
            ic.oturumlar.insert(oturum, acik);
        }
        self.mcp
            .oturum_kipi_guncelle(&alan_kimligi, Some(&alan_yolu), oturum, Some(kip));
        Ok(())
    }

    async fn eklenemeyen_oturumu_kapat(&self, oturum: OturumId, acik: Box<AcikOturum>) {
        self.isler.oturumu_kapat(oturum).await;
        let tutamac = acik.tutamac.clone();
        tutamac.iptal_et();
        // Olay alıcısını bırakmak, çekirdeğin kapanış belirtecini tamamlar ve
        // kayıt kilidini çözmeden önce görevin bitmesini sağlar.
        drop(acik);
        let _ = tokio::time::timeout(KAPANIS_SURESI, tutamac.kapanmasini_bekle()).await;
    }

    /// Çalışma alanının araçları, eklenti araçları ve ortak kilitleriyle
    /// oturum bağlamı. Eklenti araçları yüklenebilen HER eklenti için eklenir,
    /// etkin ya da açık olmasından bağımsız: kapalı olanlar `sunulur_mu` ile
    /// gizlenir, böylece anahtar açık oturumda canlı çalışır (aşama 4 tasarımı
    /// §13.3). Yüklenemeyen eklenti oturumu düşürmez; yalnızca araçsız kalır.
    /// İkinci değer sağ panelin dosya görünümüdür: araçlarla aynı çalışma
    /// alanı tutamacını kullanır (masaüstü arayüzü tasarımı §6.3).
    pub(crate) async fn baglam(
        &self,
        oturum: OturumId,
        kok: &Path,
    ) -> Result<(OturumBaglami, Arc<DosyaGorunumu>), KomutHatasi> {
        let siniflandirici = Arc::new(
            Siniflandirici::yeni_ayarla(kok, self.ayarlar.gizli_ayarlari()).map_err(ic_hata)?,
        );
        let alan_kimligi = CalismaAlaniKaydi::kimlik(kok);
        let yazma_bildirimi = crate::mcp::yonetici::AlanYazmaBildirimi::yeni(
            self.mcp.clone(),
            alan_kimligi.clone(),
            kok.to_path_buf(),
        );
        let (yazma_basladi, yazma_bitti) = yazma_bildirimi.bildirim_cifti();
        let alan = AracAlani::ac_bildirimli(
            siniflandirici.clone(),
            Some(yazma_basladi),
            Some(yazma_bitti),
        )?;
        let guven = Arc::new(AlanGuveni {
            kayit: self.alanlar.clone(),
            kimlik: alan_kimligi.clone(),
        });
        let kilitler = self.kilitler.al(kok)?;
        let mut araclar = varsayilan_araclar(alan.clone());
        araclar.push(Arc::new(PlanGuncelleAraci::yeni()));
        araclar.push(Arc::new(KullaniciyaSorAraci::yeni()));
        let talimat_okuyucu: Arc<dyn ProjeTalimatOkuyucu> =
            Arc::new(CalismaAlaniTalimatOkuyucu::yeni(alan.clone()));
        let geri_yukleyici: Arc<dyn GeriYukleyici> = alan.clone();
        let gorunum = Arc::new(DosyaGorunumu::yeni(alan.clone()));
        let eklenti_araclari = self
            .eklentiler
            .araclar(&alan, &kilitler, &alan_kimligi)
            .await;
        self.isler.oturumu_etkinlestir(oturum);
        let is_baglantisi = crate::isler::OturumIsBaglantisi::yeni(self.isler.clone(), oturum);
        araclar.push(Arc::new(
            KabukAraci::yeni(
                alan,
                self.yalitici.clone(),
                guven,
                self.yalitim_ayarlari.clone(),
            )
            .kullanici_ag_listesi(self.ayarlar.ag_listesi())
            .arka_plan_baglantisi(is_baglantisi.clone()),
        ));
        araclar.extend(kalfa_tools::kabuk::isler::arka_plan_araclari(
            is_baglantisi,
            self.yalitici.clone(),
            self.yalitim_ayarlari.clone(),
            self.ayarlar.ag_listesi(),
        ));
        // Eklenti araçlarının adı `<önek>__<araç>`'tır, yerleşiklerinkinde `__`
        // yoktur (§6.5); yine de çift ad oturumu düşürmesin.
        let mut adlar: HashSet<String> = araclar.iter().map(|a| a.ad().to_string()).collect();
        araclar.extend(
            eklenti_araclari
                .into_iter()
                .filter(|a| adlar.insert(a.ad().to_string())),
        );
        // Alanın MCP araçları her model turunun başında bu kaynaktan gelir;
        // planlayıcı keşfi ve yayımı aktif turlu oturumun bütçesiyle yürütür.
        let mcp_kaynagi = self.mcp_araclari.al(&alan_kimligi);
        mcp_kaynagi.planlayici_yoksa_ayarla(
            crate::mcp::baglanti::McpAlanBaglantisi::yeni(
                alan_kimligi.clone(),
                kok.to_path_buf(),
                self.mcp.clone(),
                self.mcp_denetleyici.clone(),
                self.ayarlar.clone(),
                self.denetci.clone(),
                self.ayarlar.denetci_kipi_kaynagi(),
                Some(self.mcp_kanali.clone()),
            )
            .planlayici(),
        );
        let araclar =
            Arc::new(AracKayitDefteri::yeni_kaynakli(araclar, mcp_kaynagi).map_err(ic_hata)?);
        Ok((
            OturumBaglami {
                saglayici: self.evren.clone(),
                araclar,
                siniflandirici,
                kilitler,
                sart_kapisi: self.kapi.clone(),
                geri_yukleyici: Some(geri_yukleyici),
                denetci: self.denetci.clone(),
                denetci_kipi: Some(self.ayarlar.denetci_kipi_kaynagi()),
                talimat_okuyucu: Some(talimat_okuyucu),
            },
            gorunum,
        ))
    }

    /// Oturuma ait arka plan işlerinin ve terminallerin özet listesini döner.
    pub fn oturum_isleri(
        &self,
        oturum: OturumId,
    ) -> Result<Vec<crate::isler::IsOzeti>, KomutHatasi> {
        self.isler.oturumu_kurtar(oturum);
        Ok(self.isler.oturum_isleri(oturum))
    }

    /// Oturuma ait bir arka plan işinin çıktısını imleçten itibaren okur.
    pub fn is_oku(
        &self,
        oturum: OturumId,
        is_id: String,
        imlec: usize,
    ) -> Result<crate::isler::IsOkumaSonucu, KomutHatasi> {
        self.isler.oturumu_kurtar(oturum);
        self.isler
            .oku(oturum, &is_id, imlec)
            .map_err(|e| KomutHatasi::gecersiz(e.to_string()))
    }

    /// Oturuma ait bir arka plan işine kullanıcı girdisi ve/veya EOF gönderir.
    pub async fn is_girdi(
        &self,
        oturum: OturumId,
        is_id: String,
        veri: String,
        eof: bool,
    ) -> Result<crate::isler::IsOzeti, KomutHatasi> {
        self.isler
            .girdi(oturum, &is_id, veri.as_bytes(), eof)
            .await
            .map_err(|e| KomutHatasi::gecersiz(e.to_string()))
    }

    /// Oturuma ait bir arka plan işini durdurur ve kapanışını bekler.
    pub async fn is_durdur(
        &self,
        oturum: OturumId,
        is_id: String,
    ) -> Result<crate::isler::IsOzeti, KomutHatasi> {
        self.isler
            .durdur(oturum, &is_id)
            .await
            .map_err(|e| KomutHatasi::gecersiz(e.to_string()))
    }

    /// Oturuma ait bir terminal işinin pencere boyutunu günceller.
    pub fn is_boyutlandir(
        &self,
        oturum: OturumId,
        is_id: String,
        satir: u16,
        sutun: u16,
    ) -> Result<crate::isler::IsOzeti, KomutHatasi> {
        self.isler
            .boyutlandir(
                oturum,
                &is_id,
                kalfa_sandbox::TerminalBoyutu { satir, sutun },
            )
            .map_err(|e| KomutHatasi::gecersiz(e.to_string()))
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
