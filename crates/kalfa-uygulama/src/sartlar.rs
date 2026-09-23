//! Açılış ve şart penceresi akışı (1c tasarımı §7). Kalfa şartları kullanıcı
//! adına asla kabul etmez: kabul isteği yalnızca `sartlari_kabul_et`'ten,
//! yani şart penceresindeki "Kabul ediyorum" düğmesinden gider (üst belge
//! §11.8).

use std::sync::Arc;

use kalfa_core::sart_kapisi::{
    GerekliSonucu, IlkDurum, KabulSonucu, KapiGorunumu, SartKarari, SartMetni,
};
use kalfa_evren::EvrenError;

use crate::Uygulama;
use crate::hata::KomutHatasi;
use crate::sozlesme::{
    AnahtarNedeni, AnahtarSonucu, BaglantiDurumu, KabulYaniti, SartPenceresiDurumu,
};

/// Şart penceresi komutu sürerken kapı izleyicisi pencereyi yeniden yüklemez;
/// komut yeni durumu kendisi döner.
struct PencereKomutu<'a>(&'a Uygulama);

impl<'a> PencereKomutu<'a> {
    fn yeni(uygulama: &'a Uygulama) -> Self {
        uygulama.ic().pencere_komutu += 1;
        Self(uygulama)
    }
}

impl Drop for PencereKomutu<'_> {
    fn drop(&mut self) {
        let mut ic = self.0.ic();
        ic.pencere_komutu = ic.pencere_komutu.saturating_sub(1);
    }
}

impl Uygulama {
    /// Açılış (1c tasarımı §7.1): kayıtlı anahtar yoksa anahtar formu; varsa
    /// şart durumu sorulur. Kabul gerekiyorsa kapı metni göstermeye geçer ve
    /// kapı izleyicisi şart penceresini açar.
    pub async fn baslat(self: &Arc<Self>) {
        self.kapi_izleyicisini_baslat();
        let u = self.clone();
        let anahtar = tokio::task::spawn_blocking(move || u.kasa.oku())
            .await
            .ok()
            .flatten();
        let Some(anahtar) = anahtar else {
            return self.anahtar_iste(AnahtarNedeni::Yok);
        };
        let Ok(saglayici) = self.uretici.uret(&anahtar) else {
            return self.anahtar_iste(AnahtarNedeni::KayitliGecersiz);
        };
        self.evren.ayarla(saglayici);
        match self.kapi.ilk_durum().await {
            Ok(IlkDurum::KabulEdilmis) => {
                self.ic().sart_penceresi = SartPenceresiDurumu::Tamam;
                self.pencereler.ana_ac();
            }
            Ok(IlkDurum::KabulGerekli(_)) => {}
            Ok(IlkDurum::Reddedildi) => {
                self.ic().sart_penceresi = SartPenceresiDurumu::Kilitli;
                self.pencereler.sartlar_goster();
            }
            Err(EvrenError::AnahtarGecersiz) => self.anahtar_iste(AnahtarNedeni::KayitliGecersiz),
            Err(_) => {
                // EVREN'e ulaşılamıyor: geçmiş okunabilsin diye ana pencere açılır.
                self.ic().baglanti = BaglantiDurumu::Yok;
                self.pencereler.ana_ac();
            }
        }
    }

    fn anahtar_iste(&self, neden: AnahtarNedeni) {
        self.ic().sart_penceresi = SartPenceresiDurumu::AnahtarGerekli { neden };
        self.pencereler.sartlar_goster();
    }

    /// Kapı metin göstermeye geçtiğinde şart penceresini açar. Pencere
    /// açıksa yeniden yüklenir; ikinci pencere açılmaz (1c tasarımı §7.3).
    fn kapi_izleyicisini_baslat(self: &Arc<Self>) {
        let mut izleyici = self.kapi.izle();
        let zayif = Arc::downgrade(self);
        tokio::spawn(async move {
            loop {
                let gorunum = izleyici.borrow_and_update().gorunum.clone();
                let Some(uygulama) = zayif.upgrade() else {
                    return;
                };
                if let KapiGorunumu::Gosteriliyor(metin) = gorunum {
                    uygulama.sart_metni_geldi(metin);
                }
                drop(uygulama);
                if izleyici.changed().await.is_err() {
                    return;
                }
            }
        });
    }

    fn sart_metni_geldi(&self, metin: SartMetni) {
        let yeni = SartPenceresiDurumu::SartGerekli { metin };
        let goster = {
            let mut ic = self.ic();
            if ic.sart_penceresi == yeni {
                false
            } else {
                ic.sart_penceresi = yeni;
                ic.pencere_komutu == 0
            }
        };
        if goster {
            self.pencereler.sartlar_goster();
        }
    }

    pub fn sart_penceresi_durumu(&self) -> SartPenceresiDurumu {
        self.ic().sart_penceresi.clone()
    }

    /// Girilen anahtar önce şart durumu sorgusuyla denenir; 401 dönerse ya da
    /// EVREN'e ulaşılamazsa saklanmaz (1c tasarımı §7.2).
    pub async fn anahtari_kaydet(
        self: &Arc<Self>,
        anahtar: String,
    ) -> Result<AnahtarSonucu, KomutHatasi> {
        let _komut = PencereKomutu::yeni(self);
        let anahtar = anahtar.trim().to_string();
        if anahtar.is_empty() {
            return Ok(AnahtarSonucu::Gecersiz);
        }
        let Ok(saglayici) = self.uretici.uret(&anahtar) else {
            return Ok(AnahtarSonucu::Gecersiz);
        };
        match saglayici.sart_durumu().await {
            Ok(_) => {}
            Err(EvrenError::AnahtarGecersiz) => return Ok(AnahtarSonucu::Gecersiz),
            Err(EvrenError::Ag(_)) => return Ok(AnahtarSonucu::Ulasilamadi),
            Err(e) => return Err(e.into()),
        }
        let u = self.clone();
        let depo = tokio::task::spawn_blocking(move || u.kasa.yaz(&anahtar))
            .await
            .map_err(|e| KomutHatasi::ic(e.to_string()))?;
        self.evren.ayarla(saglayici);
        self.ic().baglanti = BaglantiDurumu::Var;
        let sonraki = match self.kapi.ilk_durum().await? {
            IlkDurum::KabulEdilmis => {
                self.sart_tamam();
                SartPenceresiDurumu::Tamam
            }
            IlkDurum::KabulGerekli(metin) => {
                let durum = SartPenceresiDurumu::SartGerekli { metin };
                self.ic().sart_penceresi = durum.clone();
                durum
            }
            IlkDurum::Reddedildi => {
                self.ic().sart_penceresi = SartPenceresiDurumu::Kilitli;
                SartPenceresiDurumu::Kilitli
            }
        };
        Ok(AnahtarSonucu::Kaydedildi { depo, sonraki })
    }

    /// "Kabul ediyorum". Sürüm gösterilenle aynı değilse kabul gönderilmez,
    /// güncel metin döner (1c tasarımı §7.3). Aynı anda verilen ret kabulü
    /// geçersiz kılar; pencere kilitli görünüme geçer.
    pub async fn sartlari_kabul_et(&self, surum: u32) -> Result<KabulYaniti, KomutHatasi> {
        let _komut = PencereKomutu::yeni(self);
        match self.kapi.kabul_et(surum).await? {
            KabulSonucu::Kabul => {
                self.sart_tamam();
                Ok(KabulYaniti::Kabul)
            }
            KabulSonucu::YeniMetin(metin) => {
                self.ic().sart_penceresi = SartPenceresiDurumu::SartGerekli {
                    metin: metin.clone(),
                };
                Ok(KabulYaniti::YeniMetin { metin })
            }
            KabulSonucu::Ret => {
                self.ic().sart_penceresi = SartPenceresiDurumu::Kilitli;
                Ok(KabulYaniti::Kilitli)
            }
            // Kapı artık metin göstermiyor: çift tıklamada ilk tıklama kabulü
            // çoktan göndermiştir. Son karar kabul değilse kabul edilmiş gibi
            // yapılmaz.
            KabulSonucu::Bosta => match self.kapi.durum().son_karar {
                Some(SartKarari::Kabul { .. }) => {
                    self.sart_tamam();
                    Ok(KabulYaniti::Kabul)
                }
                _ => Err(KomutHatasi::gecersiz("Onay bekleyen şart metni yok.")),
            },
        }
    }

    /// "Kabul etmiyorum": bekleyen turlar iptal edilir, pencere kilitli
    /// görünüme geçer (1c tasarımı §7.4).
    pub fn sartlari_reddet(&self) {
        let kilitli =
            self.kapi.reddet() || matches!(self.kapi.durum().gorunum, KapiGorunumu::Reddedildi);
        // Kapı bu arada kabulle kapandıysa ret bir şey değiştirmez.
        if kilitli {
            self.ic().sart_penceresi = SartPenceresiDurumu::Kilitli;
        }
    }

    /// Kilitli görünümdeki "Metni yeniden göster"; pencerenin yeni durumunu
    /// döner.
    pub async fn sart_metnini_goster(&self) -> Result<SartPenceresiDurumu, KomutHatasi> {
        let _komut = PencereKomutu::yeni(self);
        let nesil = self.kapi.durum().nesil;
        let durum = match self.kapi.gerekli(nesil).await? {
            GerekliSonucu::OnayGerekli { metin, .. } => SartPenceresiDurumu::SartGerekli { metin },
            GerekliSonucu::KabulEdildi { .. } => {
                self.sart_tamam();
                SartPenceresiDurumu::Tamam
            }
            GerekliSonucu::Ret => SartPenceresiDurumu::Kilitli,
        };
        self.ic().sart_penceresi = durum.clone();
        Ok(durum)
    }

    fn sart_tamam(&self) {
        self.ic().sart_penceresi = SartPenceresiDurumu::Tamam;
        self.pencereler.ana_ac();
        self.pencereler.sartlar_kapat();
    }

    /// Ana penceredeki "Anahtarı değiştir". Şart onayı bekleniyorsa önce şart
    /// metni gösterilir.
    pub fn anahtar_penceresini_ac(&self) {
        {
            let mut ic = self.ic();
            if !matches!(ic.sart_penceresi, SartPenceresiDurumu::SartGerekli { .. }) {
                ic.sart_penceresi = SartPenceresiDurumu::AnahtarGerekli {
                    neden: AnahtarNedeni::Degistir,
                };
            }
        }
        self.pencereler.sartlar_goster();
    }

    /// Kapatılmış şart penceresini yeniden açar. Şart penceresini kapatmak
    /// ret sayılmaz; bekleyen turlar beklemeyi sürdürür.
    pub fn sart_penceresini_ac(&self) {
        let ac = matches!(
            self.ic().sart_penceresi,
            SartPenceresiDurumu::SartGerekli { .. } | SartPenceresiDurumu::Kilitli
        );
        if ac {
            self.pencereler.sartlar_goster();
        }
    }
}
