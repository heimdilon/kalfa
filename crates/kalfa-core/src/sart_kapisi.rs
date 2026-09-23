//! Uygulama genelinde tek şart kapısı (1c tasarımı §7.3, üst belge §11.8).
//!
//! 403 alan oturumlar kapıya kaydolup bekler. Kapı metni her sürüm için bir kez
//! alır, şart penceresine hangi sürümü gösterdiğini saklar ve kullanıcı kabul
//! edince kabul isteğini **bir kez** gönderir. Kabul yalnızca `kabul_et` ile
//! gider; onu da yalnızca şart penceresinin komutu çağırır (1c tasarımı §6).

use std::sync::Arc;

use kalfa_evren::EvrenError;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, watch};

use crate::saglayici::Saglayici;

/// Şart penceresinde gösterilen metin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SartMetni {
    pub surum: u32,
    /// EVREN'in `is_material` bilgisi: değişiklik esaslı mı.
    pub esasli: bool,
    pub metin: String,
}

/// Kapının dışarıdan görünen hâli; uygulama şart penceresini buna göre açar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KapiGorunumu {
    /// Bekleyen bir şart onayı yok.
    Bosta,
    /// Metin gösteriliyor; kullanıcının kararı bekleniyor.
    Gosteriliyor(SartMetni),
    /// Kullanıcı reddetti; uygulama kilitli görünümde.
    Reddedildi,
}

/// Bekleyen oturumların öğrendiği karar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SartKarari {
    Kabul { surum: u32 },
    Ret,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KabulSonucu {
    /// Kabul EVREN'e gönderildi; bekleyen oturumlar uyandı.
    Kabul,
    /// Sürüm gösterilenle aynı değildi ya da bu arada değişti; güncel metin
    /// gösteriliyor.
    YeniMetin(SartMetni),
    /// Eski veya yinelenen tıklamada kapı artık gösterim durumunda değildi.
    Bosta,
    /// Kullanıcı ret verdi; eşzamanlı eski kabul işlemi görünümü açamaz.
    Ret,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IlkDurum {
    KabulEdilmis,
    KabulGerekli(SartMetni),
    Reddedildi,
}

/// 403 yanıtının başladığı andaki kapı nesline göre oturumun yapacağı işlem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GerekliSonucu {
    OnayGerekli { metin: SartMetni, nesil: u64 },
    KabulEdildi { surum: u32 },
    Ret,
}

enum GosterSonucu {
    Gosterildi { nesil: u64 },
    Karar(SartKarari),
}

enum KabulYazSonucu {
    Gosterildi,
    Sonuc(KabulSonucu),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KapiDurumu {
    pub gorunum: KapiGorunumu,
    /// Her kararla bir artar.
    pub nesil: u64,
    pub son_karar: Option<SartKarari>,
}

pub struct SartKapisi {
    saglayici: Arc<dyn Saglayici>,
    /// Metin alma ve kabul gönderme sırayla yapılır; aynı anda iki istek gitmez.
    islem: Mutex<()>,
    durum: watch::Sender<KapiDurumu>,
}

impl SartKapisi {
    pub fn yeni(saglayici: Arc<dyn Saglayici>) -> Self {
        let (durum, _) = watch::channel(KapiDurumu {
            gorunum: KapiGorunumu::Bosta,
            nesil: 0,
            son_karar: None,
        });
        Self {
            saglayici,
            islem: Mutex::new(()),
            durum,
        }
    }

    pub fn durum(&self) -> KapiDurumu {
        self.durum.borrow().clone()
    }

    /// Kapının değişikliklerini izler; uygulama şart penceresini buna göre
    /// açar ya da kapatır.
    pub fn izle(&self) -> watch::Receiver<KapiDurumu> {
        self.durum.subscribe()
    }

    /// İlk açılış: şart durumu sorulur; kabul gerekiyorsa metin gösterime
    /// alınır.
    pub async fn ilk_durum(&self) -> Result<IlkDurum, EvrenError> {
        let baslangic_nesli = self.durum().nesil;
        let durum = self.saglayici.sart_durumu().await?;
        if let Some(karar) = Self::karar_sonrasi(&self.durum(), baslangic_nesli) {
            return Ok(match karar {
                SartKarari::Kabul { .. } => IlkDurum::KabulEdilmis,
                SartKarari::Ret => IlkDurum::Reddedildi,
            });
        }
        if durum.accepted {
            return Ok(IlkDurum::KabulEdilmis);
        }
        match self.gerekli(baslangic_nesli).await? {
            GerekliSonucu::OnayGerekli { metin, .. } => Ok(IlkDurum::KabulGerekli(metin)),
            GerekliSonucu::KabulEdildi { .. } => Ok(IlkDurum::KabulEdilmis),
            GerekliSonucu::Ret => Ok(IlkDurum::Reddedildi),
        }
    }

    /// 403 yanıtını başlatan istekten önce yakalanan nesli verir. O istekten
    /// sonra kabul olduysa oturum onayı tekrar istemeden sürer; kabulden sonra
    /// başlatılan yeni bir 403 ise yeni şart metni ister.
    pub async fn gerekli(&self, istek_nesli: u64) -> Result<GerekliSonucu, EvrenError> {
        let ilk_durum = self.durum();
        let istek_nesli = istek_nesli.min(ilk_durum.nesil);
        if let Some(karar) = Self::karar_sonrasi(&ilk_durum, istek_nesli) {
            return Ok(Self::gerekli_karar(karar));
        }
        if let KapiGorunumu::Gosteriliyor(metin) = ilk_durum.gorunum {
            return Ok(GerekliSonucu::OnayGerekli {
                metin,
                nesil: ilk_durum.nesil,
            });
        }

        let _sira = self.islem.lock().await;
        let mevcut = self.durum();
        if let Some(karar) = Self::karar_sonrasi(&mevcut, istek_nesli) {
            return Ok(Self::gerekli_karar(karar));
        }
        if let KapiGorunumu::Gosteriliyor(metin) = mevcut.gorunum {
            return Ok(GerekliSonucu::OnayGerekli {
                metin,
                nesil: mevcut.nesil,
            });
        }
        let metin = self.metni_al().await?;
        let guncel = self.durum();
        if let Some(karar) = Self::karar_sonrasi(&guncel, istek_nesli) {
            return Ok(Self::gerekli_karar(karar));
        }
        match self.goster_if_nesil(metin.clone(), guncel.nesil) {
            GosterSonucu::Gosterildi { nesil } => Ok(GerekliSonucu::OnayGerekli { metin, nesil }),
            GosterSonucu::Karar(karar) => Ok(Self::gerekli_karar(karar)),
        }
    }

    /// `nesil`den sonraki ilk kararı bekler. Future bırakılırsa kapı
    /// etkilenmez; oturumun iptali böyle çalışır. Birden fazla nesil atlandıysa
    /// aradaki kararlardan biri kaçmış olabilir; bu durumda `Ret` döner.
    pub async fn karar_bekle(&self, nesil: u64) -> SartKarari {
        let mut izleyici = self.durum.subscribe();
        loop {
            let karar = {
                let d = izleyici.borrow_and_update();
                if d.nesil > nesil {
                    if d.nesil - nesil == 1 {
                        d.son_karar
                    } else {
                        // Birden fazla karar atlandıysa, kapı en az birini
                        // bekleyene ulaştıramadı. Eski tur fail-closed iptal olur.
                        Some(SartKarari::Ret)
                    }
                } else {
                    None
                }
            };
            if let Some(karar) = karar {
                return karar;
            }
            if izleyici.changed().await.is_err() {
                return SartKarari::Ret;
            }
        }
    }

    /// Şart penceresinin "Kabul ediyorum" düğmesi. Sürüm gösterilenle aynı
    /// değilse kabul gönderilmez; kabul EVREN'e bir kez gider. Çağrı eski veya
    /// kapanmış bir pencereden geldiyse yeni metin çekip kapıyı kendiliğinden
    /// açmaz.
    pub async fn kabul_et(&self, surum: u32) -> Result<KabulSonucu, EvrenError> {
        let ilk_durum = self.durum();
        match ilk_durum.gorunum {
            KapiGorunumu::Bosta => return Ok(KabulSonucu::Bosta),
            KapiGorunumu::Reddedildi => return Ok(KabulSonucu::Ret),
            KapiGorunumu::Gosteriliyor(_) => {}
        }

        let _sira = self.islem.lock().await;
        let mevcut = self.durum();
        if let Some(karar) = Self::karar_sonrasi(&mevcut, ilk_durum.nesil) {
            return Ok(Self::kabul_karar(karar));
        }
        let gosterilen = match &mevcut.gorunum {
            KapiGorunumu::Bosta => return Ok(KabulSonucu::Bosta),
            KapiGorunumu::Reddedildi => return Ok(KabulSonucu::Ret),
            KapiGorunumu::Gosteriliyor(metin) => metin.clone(),
        };
        if gosterilen.surum != surum {
            let metin = self.metni_al().await?;
            let guncel = self.durum();
            if let Some(karar) = Self::karar_sonrasi(&guncel, mevcut.nesil) {
                return Ok(Self::kabul_karar(karar));
            }
            return match self.goster_if_nesil(metin.clone(), guncel.nesil) {
                GosterSonucu::Gosterildi { .. } => Ok(KabulSonucu::YeniMetin(metin)),
                GosterSonucu::Karar(karar) => Ok(Self::kabul_karar(karar)),
            };
        }

        match self.saglayici.sart_kabul(surum).await {
            Ok(()) => match self.kabul_yaz_if_nesil(mevcut.nesil, surum) {
                Err(karar) => Ok(Self::kabul_karar(karar)),
                Ok(KabulYazSonucu::Gosterildi) => Ok(KabulSonucu::Kabul),
                Ok(KabulYazSonucu::Sonuc(sonuc)) => Ok(sonuc),
            },
            Err(EvrenError::SartKabulGerekli { .. }) => {
                let metin = self.metni_al().await?;
                let guncel = self.durum();
                if let Some(karar) = Self::karar_sonrasi(&guncel, mevcut.nesil) {
                    return Ok(Self::kabul_karar(karar));
                }
                match self.goster_if_nesil(metin.clone(), guncel.nesil) {
                    GosterSonucu::Gosterildi { .. } => Ok(KabulSonucu::YeniMetin(metin)),
                    GosterSonucu::Karar(karar) => Ok(Self::kabul_karar(karar)),
                }
            }
            Err(e) => {
                if let Some(karar) = Self::karar_sonrasi(&self.durum(), mevcut.nesil) {
                    Ok(Self::kabul_karar(karar))
                } else {
                    Err(e)
                }
            }
        }
    }

    /// "Kabul etmiyorum": bekleyen oturumların turları iptal edilir.
    pub fn reddet(&self) -> bool {
        let mut reddedildi = false;
        self.durum.send_if_modified(|d| {
            if !matches!(d.gorunum, KapiGorunumu::Gosteriliyor(_)) {
                return false;
            }
            d.gorunum = KapiGorunumu::Reddedildi;
            d.nesil += 1;
            d.son_karar = Some(SartKarari::Ret);
            reddedildi = true;
            true
        });
        reddedildi
    }

    /// Nesil kontrolü ve metni gösterme aynı watch kilidi altında yapılır;
    /// araya giren bir ret böylece eski bir fetch ile ezilemez.
    fn goster_if_nesil(&self, metin: SartMetni, beklenen_nesil: u64) -> GosterSonucu {
        let mut sonuc = GosterSonucu::Karar(SartKarari::Ret);
        self.durum.send_if_modified(|d| {
            if d.nesil != beklenen_nesil {
                let karar = Self::karar_sonrasi(d, beklenen_nesil).unwrap_or(SartKarari::Ret);
                sonuc = GosterSonucu::Karar(karar);
                return false;
            }
            d.gorunum = KapiGorunumu::Gosteriliyor(metin);
            sonuc = GosterSonucu::Gosterildi { nesil: d.nesil };
            true
        });
        sonuc
    }

    /// Kabul yanıtından sonra Ret araya girdiyse Kabul yazımını yapmaz.
    fn kabul_yaz_if_nesil(
        &self,
        beklenen_nesil: u64,
        surum: u32,
    ) -> Result<KabulYazSonucu, SartKarari> {
        let mut sonuc = Err(SartKarari::Ret);
        self.durum.send_if_modified(|d| {
            if d.nesil != beklenen_nesil {
                let karar = Self::karar_sonrasi(d, beklenen_nesil).unwrap_or(SartKarari::Ret);
                sonuc = Err(karar);
                return false;
            }
            match &d.gorunum {
                KapiGorunumu::Bosta => {
                    sonuc = Ok(KabulYazSonucu::Sonuc(KabulSonucu::Bosta));
                    false
                }
                KapiGorunumu::Reddedildi => {
                    sonuc = Ok(KabulYazSonucu::Sonuc(KabulSonucu::Ret));
                    false
                }
                KapiGorunumu::Gosteriliyor(metin) if metin.surum != surum => {
                    sonuc = Ok(KabulYazSonucu::Sonuc(KabulSonucu::YeniMetin(metin.clone())));
                    false
                }
                KapiGorunumu::Gosteriliyor(_) => {
                    d.gorunum = KapiGorunumu::Bosta;
                    d.nesil += 1;
                    d.son_karar = Some(SartKarari::Kabul { surum });
                    sonuc = Ok(KabulYazSonucu::Gosterildi);
                    true
                }
            }
        });
        sonuc
    }

    fn karar_sonrasi(durum: &KapiDurumu, nesil: u64) -> Option<SartKarari> {
        if durum.nesil <= nesil {
            return None;
        }
        if durum.nesil - nesil != 1 {
            return Some(SartKarari::Ret);
        }
        Some(durum.son_karar.unwrap_or(SartKarari::Ret))
    }

    fn gerekli_karar(karar: SartKarari) -> GerekliSonucu {
        match karar {
            SartKarari::Kabul { surum } => GerekliSonucu::KabulEdildi { surum },
            SartKarari::Ret => GerekliSonucu::Ret,
        }
    }

    fn kabul_karar(karar: SartKarari) -> KabulSonucu {
        match karar {
            SartKarari::Kabul { .. } => KabulSonucu::Kabul,
            SartKarari::Ret => KabulSonucu::Ret,
        }
    }

    async fn metni_al(&self) -> Result<SartMetni, EvrenError> {
        let durum = self.saglayici.sart_durumu().await?;
        let metin = self.saglayici.sart_metni().await?;
        Ok(SartMetni {
            surum: metin.version,
            esasli: durum.is_material,
            metin: metin.content,
        })
    }
}
