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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IlkDurum {
    KabulEdilmis,
    KabulGerekli(SartMetni),
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
        if self.saglayici.sart_durumu().await?.accepted {
            return Ok(IlkDurum::KabulEdilmis);
        }
        let (metin, _) = self.gerekli().await?;
        Ok(IlkDurum::KabulGerekli(metin))
    }

    /// 403 alan oturum çağırır. Gösterilen metin varsa ona katılır, yoksa
    /// EVREN'den alır. Dönen nesil `karar_bekle`'ye verilir.
    pub async fn gerekli(&self) -> Result<(SartMetni, u64), EvrenError> {
        let _sira = self.islem.lock().await;
        let mevcut = self.durum();
        if let KapiGorunumu::Gosteriliyor(metin) = mevcut.gorunum {
            return Ok((metin, mevcut.nesil));
        }
        let metin = self.metni_al().await?;
        self.goster(metin.clone());
        Ok((metin, mevcut.nesil))
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
    /// değilse kabul gönderilmez; kabul EVREN'e bir kez gider.
    pub async fn kabul_et(&self, surum: u32) -> Result<KabulSonucu, EvrenError> {
        let _sira = self.islem.lock().await;
        let gosterilen = match self.durum().gorunum {
            KapiGorunumu::Gosteriliyor(metin) => Some(metin.surum),
            _ => None,
        };
        if gosterilen != Some(surum) {
            let metin = self.metni_al().await?;
            self.goster(metin.clone());
            return Ok(KabulSonucu::YeniMetin(metin));
        }
        match self.saglayici.sart_kabul(surum).await {
            Ok(()) => {
                self.karar(KapiGorunumu::Bosta, SartKarari::Kabul { surum });
                Ok(KabulSonucu::Kabul)
            }
            Err(EvrenError::SartKabulGerekli { .. }) => {
                let metin = self.metni_al().await?;
                self.goster(metin.clone());
                Ok(KabulSonucu::YeniMetin(metin))
            }
            Err(e) => Err(e),
        }
    }

    /// "Kabul etmiyorum": bekleyen oturumların turları iptal edilir.
    pub fn reddet(&self) {
        self.karar(KapiGorunumu::Reddedildi, SartKarari::Ret);
    }

    fn goster(&self, metin: SartMetni) {
        self.durum
            .send_modify(|d| d.gorunum = KapiGorunumu::Gosteriliyor(metin));
    }

    fn karar(&self, gorunum: KapiGorunumu, karar: SartKarari) {
        self.durum.send_modify(|d| {
            d.gorunum = gorunum;
            d.nesil += 1;
            d.son_karar = Some(karar);
        });
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
