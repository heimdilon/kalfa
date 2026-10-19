//! Oturumun ağ denemesi ve token defteri. Bu modül ağa çıkmaz, kanal ve kayıt
//! yazmaz; oturum başlangıç ve bitiş kaydını kalıcı yazdıktan sonra bu durumu
//! uygular.

use std::collections::BTreeSet;
use std::fmt;
use std::future::Future;
use std::time::Duration;

use kalfa_evren::tipler::Usage;
use kalfa_evren::{EvrenError, YanitMeta};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

use crate::gonderim::{GunlukToken, IstekSonucu};
use crate::kayit::KayitOgesi;

/// Oturum içinde 1'den artan bütçe denemesi kimliği.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ButceIslemId(pub u64);

impl fmt::Display for ButceIslemId {
    fn fmt(&self, bicim: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(bicim, "{}", self.0)
    }
}

/// Deneme defteri ve kayıt tutarsızlıkları.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ButceHatasi {
    #[error("Bütçe işlemi beklenen sırada değil: {0}.")]
    SiraDisiKimlik(ButceIslemId),
    #[error("Bilinmeyen bütçe işlemi: {0}.")]
    BilinmeyenIslem(ButceIslemId),
    #[error("Bütçe işlemi zaten sonuçlandı: {0}.")]
    CiftSonuc(ButceIslemId),
    #[error("Bütçe işlem kimliği tükendi.")]
    KimlikTukendi,
}

/// Arayüze ve kurtarılan görünüme giden özet.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButceGorunumu {
    pub baslatilan: u64,
    pub bilinen_token: u64,
    pub bilinmeyen: u64,
    pub aktif: u64,
}

/// Bir oturumun deneme ve token defteri. Sayaçların tek sahibi oturumdur.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ButceDurumu {
    baslatilan: u64,
    token: u64,
    bilinmeyen: u64,
    aktif: BTreeSet<ButceIslemId>,
    biten: BTreeSet<ButceIslemId>,
}

impl ButceDurumu {
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Durumu değiştirmeden sıradaki kimliği verir; sayaç taşmasında `None`.
    pub fn siradaki(&self) -> Option<ButceIslemId> {
        self.baslatilan.checked_add(1).map(ButceIslemId)
    }

    /// Kimlik `siradaki()`'nin verdiği olmalıdır.
    pub fn baslat(&mut self, islem: ButceIslemId) -> Result<(), ButceHatasi> {
        let beklenen = self.siradaki().ok_or(ButceHatasi::KimlikTukendi)?;
        if islem != beklenen {
            return Err(ButceHatasi::SiraDisiKimlik(islem));
        }
        self.kimligi_kaydet(islem);
        Ok(())
    }

    /// `kullanim: None` bilinmeyendir; `Some` ise token toplamı doyumlu eklenir.
    pub fn bitir(
        &mut self,
        islem: ButceIslemId,
        kullanim: Option<GunlukToken>,
    ) -> Result<(), ButceHatasi> {
        if self.biten.contains(&islem) {
            return Err(ButceHatasi::CiftSonuc(islem));
        }
        if !self.aktif.remove(&islem) {
            return Err(ButceHatasi::BilinmeyenIslem(islem));
        }
        self.biten.insert(islem);
        match kullanim {
            Some(kullanim) => self.token = self.token.saturating_add(kullanim.toplam()),
            None => self.bilinmeyen = self.bilinmeyen.saturating_add(1),
        }
        Ok(())
    }

    /// Sonuçsuz (aktif) kimlikler, artan sırada.
    pub fn sonuclanmamis(&self) -> Vec<ButceIslemId> {
        self.aktif.iter().copied().collect()
    }

    pub fn gorunum(&self) -> ButceGorunumu {
        ButceGorunumu {
            baslatilan: self.baslatilan,
            bilinen_token: self.token,
            bilinmeyen: self.bilinmeyen,
            aktif: self.aktif.len() as u64,
        }
    }

    /// Kayıt öğelerini sırayla oynatır. Bütçe satırı hiç yoksa eski kayıt
    /// devri bellekte uygulanır (kayda yazılmaz).
    pub fn kayittan<'a>(
        ogeler: impl IntoIterator<Item = &'a KayitOgesi> + Clone,
    ) -> Result<Self, ButceHatasi> {
        let mut durum = Self::yeni();
        let butce_satiri_var = ogeler.clone().into_iter().any(butce_kaydi_mi);
        if !butce_satiri_var {
            durum.devral(eski_kayit_devri(ogeler));
            return Ok(durum);
        }
        for oge in ogeler {
            if butce_kaydi_mi(oge) {
                durum.oynat(oge)?;
            }
        }
        Ok(durum)
    }

    fn devral(&mut self, devir: ButceDevri) {
        self.baslatilan = self.baslatilan.saturating_add(devir.istek);
        self.token = self.token.saturating_add(devir.token);
        if devir.belirsiz {
            self.bilinmeyen = self.bilinmeyen.saturating_add(1);
        }
    }

    fn kimligi_kaydet(&mut self, islem: ButceIslemId) {
        self.baslatilan = islem.0;
        self.aktif.insert(islem);
    }

    fn oynat(&mut self, oge: &KayitOgesi) -> Result<(), ButceHatasi> {
        match oge {
            // Eski kayıttaki başlangıç ayarı: yalnızca devir varsa sayılır.
            KayitOgesi::ButceAyarlandi { devir, .. } => {
                if let Some(devir) = devir {
                    self.devral(*devir);
                }
                Ok(())
            }
            KayitOgesi::ButceIstegiBasladi { islem, .. } => {
                if self.siradaki() != Some(*islem) {
                    return Err(ButceHatasi::SiraDisiKimlik(*islem));
                }
                self.kimligi_kaydet(*islem);
                Ok(())
            }
            KayitOgesi::ButceIstegiBitti { islem, kullanim } => self.bitir(*islem, *kullanim),
            // Eski devam kararları sınır değiştirmek içindi; defteri etkilemez.
            KayitOgesi::ButceDevamKarari { .. } => Ok(()),
            _ => Ok(()),
        }
    }
}

/// Bütçe kaydı olan öğeleri (5b tasarımı §6) diğer kayıtlardan ayırır.
fn butce_kaydi_mi(oge: &KayitOgesi) -> bool {
    matches!(
        oge,
        KayitOgesi::ButceAyarlandi { .. }
            | KayitOgesi::ButceIstegiBasladi { .. }
            | KayitOgesi::ButceIstegiBitti { .. }
            | KayitOgesi::ButceDevamKarari { .. }
    )
}

/// 5b öncesi kayıttan bilinen en az tüketim (5b tasarımı §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButceDevri {
    pub istek: u64,
    pub token: u64,
    pub belirsiz: bool,
}

/// 5b öncesi kayıttaki `Istek` ve gönderilmiş `EklentiIstegi` satırlarından
/// bilinen en az deneme ve token sayısını çıkarır. Her deneme bir istektir;
/// token, her `Istek`'i izleyen son `Kullanim` satırlarının toplamıdır (akış
/// birikimli kullanımı birden çok kez yazabilir; eski gönderim günlüğü de
/// sonuncuyu alır). Eski kayıtta tekrar ve eklenti tokenı tam belirlenemediği
/// için bir istek varsa belirsizdir.
pub fn eski_kayit_devri<'a>(ogeler: impl IntoIterator<Item = &'a KayitOgesi>) -> ButceDevri {
    let mut istek: u64 = 0;
    let mut token: u64 = 0;
    let mut son_kullanim: u64 = 0;
    for oge in ogeler {
        match oge {
            KayitOgesi::Istek { .. } => {
                istek = istek.saturating_add(1);
                token = token.saturating_add(std::mem::take(&mut son_kullanim));
            }
            KayitOgesi::EklentiIstegi { sonuc, .. }
                if !matches!(sonuc, IstekSonucu::Gonderilmedi { .. }) =>
            {
                istek = istek.saturating_add(1)
            }
            KayitOgesi::Kullanim {
                istem, tamamlama, ..
            } => son_kullanim = istem.saturating_add(*tamamlama),
            _ => {}
        }
    }
    let token = token.saturating_add(son_kullanim);
    if istek == 0 {
        return ButceDevri {
            istek: 0,
            token: 0,
            belirsiz: false,
        };
    }
    ButceDevri {
        istek,
        token,
        belirsiz: true,
    }
}

/// Yardımcının bir ağ denemesi (5b tasarımı §6): denetçi ya da özetleyici her
/// `sohbet` çağrısı için bir kayıt. Gönderim günlüğü her denemeyi kendi `Istek`
/// satırı olarak yazar. Bütçe bitişi bu kayıttan bağımsız, denemenin hemen
/// ardından `ButceTalepci::bitir` ile oturuma gider.
#[derive(Debug, Clone, PartialEq)]
pub struct ButceDenemesi {
    pub islem: ButceIslemId,
    pub kullanim: Option<GunlukToken>,
    pub meta: Option<YanitMeta>,
    pub sonuc: IstekSonucu,
}

/// Bütçe talep kanalının kapasitesi (5b tasarımı §4.1).
const TALEP_KAPASITESI: usize = 16;

/// Denetçi, özetleyici ya da eklentinin oturumdan istediği tek izin. `yanit`
/// `None` ile dönerse izin yoktur ve çağıran ağa çıkmaz.
pub struct ButceTalebi {
    pub amac: String,
    pub yanit: oneshot::Sender<Option<ButceIslemId>>,
}

/// Bütçe kanalının mesajı. Aynı göndericinin mesajları gönderildiği sırayla
/// işlenir: bir denemenin bitişi, aynı çağıranın sonraki izin talebinden önce gelir.
pub enum ButceMesaji {
    Talep(ButceTalebi),
    Bitis {
        islem: ButceIslemId,
        kullanim: Option<GunlukToken>,
    },
    /// Oturum bütçesiyle çalışan harici denetçi çağrısının (ör. MCP araç tanımı
    /// denetimi) gönderim günlüğü (`Istek`) ve `Kullanim` satırlarını oturum
    /// kaydına yazar. Oturum iptal edilmiş, kapanmış veya kayıt hatasına
    /// düşmüşse `false` döner.
    DenetimGunlugu {
        girdi: crate::denetci::DenetimGirdisi,
        model: String,
        yanit: Box<crate::denetci::DenetimYaniti>,
        tamam: oneshot::Sender<bool>,
    },
}

/// Oturuma bağlı izin istemcisi (5b tasarımı §4.1). Her oturumun kendi kanalı
/// vardır; iki oturum birbirinin sayacını tüketmez.
#[derive(Debug, Clone)]
pub struct ButceTalepci(mpsc::Sender<ButceMesaji>);

impl ButceTalepci {
    /// Oturumun talep kanalı: gönderici istemcide, alıcı oturum görevindedir.
    pub fn kanal() -> (Self, mpsc::Receiver<ButceMesaji>) {
        let (gonderici, alici) = mpsc::channel(TALEP_KAPASITESI);
        (Self(gonderici), alici)
    }

    /// İzin ister ve bekler. `None`: izin yok (tur durduruldu ya da oturum
    /// kapandı); çağıran ağa çıkmamalıdır. Bekleyiş bu future düşerek bırakılırsa
    /// oturum izni teslim edemez ya da kapatır; ağa çıkılmaz.
    pub async fn izin(&self, amac: &str) -> Option<ButceIslemId> {
        let (yanit, alici) = oneshot::channel();
        self.0
            .send(ButceMesaji::Talep(ButceTalebi {
                amac: amac.to_string(),
                yanit,
            }))
            .await
            .ok()?;
        alici.await.ok().flatten()
    }

    /// Bir ağ denemesinin tek bitişini oturuma bildirir (5b tasarımı §4.1). Denetçi
    /// ve özetleyici her denemeyi, bir sonraki izin talebinden önce bitirir; aksi
    /// hâlde yeniden deneme, başlamış ama bitmemiş bir denemenin arkasında kart
    /// bekler ve tur kilitlenir. Oturum kapanmışsa bitiş yazılamaz; tur sonu
    /// süpürmesi o denemeyi zaten bilinmeyen olarak kapatmıştır.
    pub async fn bitir(&self, islem: ButceIslemId, kullanim: Option<GunlukToken>) {
        let _ = self.0.send(ButceMesaji::Bitis { islem, kullanim }).await;
    }

    /// Bu oturumun bütçesiyle yapılan harici denetçi çağrısının gönderim günlüğü
    /// (`Istek`) ve `Kullanim` satırlarını oturum kaydına yazar. Oturum/tur
    /// aktif ve sağlıklıysa `true`, iptal edilmiş veya kapanmışsa `false` döner.
    pub async fn denetim_gunlugu_kaydet(
        &self,
        girdi: crate::denetci::DenetimGirdisi,
        model: String,
        yanit: crate::denetci::DenetimYaniti,
    ) -> bool {
        let (tamam, alici) = oneshot::channel();
        if self
            .0
            .send(ButceMesaji::DenetimGunlugu {
                girdi,
                model,
                yanit: Box::new(yanit),
                tamam,
            })
            .await
            .is_err()
        {
            return false;
        }
        alici.await.unwrap_or(false)
    }
}

/// EVREN hatasının bütçe kullanımı (5b tasarımı §2). Üretimden önce reddedilen
/// deneme bilinen 0 tüketimdir; diğer hatalar bilinmeyendir.
pub fn hata_kullanimi(hata: &EvrenError) -> Option<GunlukToken> {
    match hata {
        EvrenError::AnahtarGecersiz
        | EvrenError::SartKabulGerekli { .. }
        | EvrenError::ModelBulunamadi { .. }
        | EvrenError::HizSiniri { .. }
        | EvrenError::GunlukSinir { .. }
        | EvrenError::BaglamAsimi { .. } => Some(GunlukToken::default()),
        EvrenError::Sunucu { durum, .. } if (400..=499).contains(durum) => {
            Some(GunlukToken::default())
        }
        EvrenError::Ag {
            govde_gitti: false, ..
        } => Some(GunlukToken::default()),
        // Üretim sonrası ya da belirsiz: AkisYarimKaldi, YanitSiniri, GecersizYanit,
        // 5xx Sunucu, Ag { govde_gitti: true } ve gelecekte eklenecek varyantlar.
        _ => None,
    }
}

/// Başarılı bir yanıtın bütçe kullanımı (5b tasarımı §2, §6). Kullanım yoksa ya
/// da tutarsızsa bilinmeyendir: `None` sıfır sayılmaz.
pub fn yanit_kullanimi(usage: Option<&Usage>) -> Option<GunlukToken> {
    let usage = usage?;
    let (istem, tamamlama) = usage.butce_kullanimi()?;
    Some(GunlukToken {
        istem,
        tamamlama,
        dusunce: usage.reasoning_tokens(),
    })
}

/// Ağ süresinden (`kalan`) harcayarak bekler: geçen süre düşülür, süre dolarsa
/// `None`. İzin beklemesi bu süreye girmez; çağıran izni bu çağrıdan önce alır
/// (5b tasarımı §7).
pub(crate) async fn sureli<F: Future>(kalan: &mut Duration, gelecek: F) -> Option<F::Output> {
    let basla = tokio::time::Instant::now();
    let sonuc = tokio::time::timeout(*kalan, gelecek).await.ok();
    *kalan = kalan.saturating_sub(basla.elapsed());
    sonuc
}

#[cfg(test)]
mod testler {
    use kalfa_evren::EkUc;

    use super::*;
    use crate::gonderim::EklentiIstekOzeti;
    use crate::kayit::EskiButceSinirlari;
    use crate::kimlik::CagriId;

    fn eski_sinir(istek: u64, token: u64) -> EskiButceSinirlari {
        EskiButceSinirlari { istek, token }
    }

    fn tok(istem: u64, tamamlama: u64) -> Option<GunlukToken> {
        Some(GunlukToken {
            istem,
            tamamlama,
            dusunce: 0,
        })
    }

    fn ayar(istek: u64, token: u64) -> KayitOgesi {
        KayitOgesi::ButceAyarlandi {
            sinirlar: eski_sinir(istek, token),
            devir: None,
        }
    }

    fn basla(id: u64) -> KayitOgesi {
        KayitOgesi::ButceIstegiBasladi {
            islem: ButceIslemId(id),
            amac: "ana".into(),
        }
    }

    fn bitis(id: u64, kullanim: Option<GunlukToken>) -> KayitOgesi {
        KayitOgesi::ButceIstegiBitti {
            islem: ButceIslemId(id),
            kullanim,
        }
    }

    fn istek_satiri(butce_islem: Option<ButceIslemId>) -> KayitOgesi {
        KayitOgesi::Istek {
            amac: "ana".into(),
            istek_id: None,
            kredi: None,
            yonlendirilen_model: None,
            model: None,
            yeni: Vec::new(),
            onceki: 0,
            sonuc: IstekSonucu::Basarili,
            butce_islem,
        }
    }

    fn ek_istek() -> EklentiIstekOzeti {
        EklentiIstekOzeti {
            eklenti: "rag".into(),
            uc: EkUc::Gomme,
            model: "model".into(),
            metin_sayisi: 1,
            bayt: 10,
            ozet: "00".into(),
            dosyalar: Vec::new(),
            dosya_fazlasi: 0,
        }
    }

    fn eklenti_satiri(sonuc: IstekSonucu) -> KayitOgesi {
        KayitOgesi::EklentiIstegi {
            cagri: CagriId::yeni(),
            istek: ek_istek(),
            istek_id: None,
            kredi: None,
            yonlendirilen_model: None,
            sonuc,
            butce_islem: None,
        }
    }

    #[test]
    fn sinirsiz_defter_binlerce_istekte_engel_vermez() {
        let mut d = ButceDurumu::yeni();
        for _ in 0..10_000 {
            let islem = d.siradaki().expect("kimlik gelmeli");
            d.baslat(islem).unwrap();
            d.bitir(islem, tok(600, 400)).unwrap();
        }
        assert!(d.siradaki().is_some());
        assert_eq!(
            d.gorunum(),
            ButceGorunumu {
                baslatilan: 10_000,
                bilinen_token: 10_000_000,
                bilinmeyen: 0,
                aktif: 0,
            }
        );
    }

    #[test]
    fn bilinmeyen_kullanim_izni_kapatmaz() {
        let mut d = ButceDurumu::yeni();
        let islem = d.siradaki().unwrap();
        d.baslat(islem).unwrap();
        d.bitir(islem, None).unwrap();
        assert_eq!(d.siradaki(), Some(ButceIslemId(2)));
        assert_eq!(d.gorunum().bilinmeyen, 1);
    }

    #[test]
    fn eski_5b_kaydı_oynatılır() {
        let kayit = vec![
            KayitOgesi::ButceAyarlandi {
                sinirlar: EskiButceSinirlari {
                    istek: 200,
                    token: 5_000_000,
                },
                devir: None,
            },
            basla(1),
            bitis(1, tok(10, 5)),
            KayitOgesi::ButceDevamKarari {
                sinirlar: EskiButceSinirlari {
                    istek: 400,
                    token: 10_000_000,
                },
                onaylanan_bilinmeyen: 0,
            },
            basla(2),
            bitis(2, tok(20, 10)),
        ];
        let d = ButceDurumu::kayittan(&kayit).unwrap();
        assert_eq!(d.gorunum().baslatilan, 2);
        assert_eq!(d.gorunum().bilinen_token, 45);
        assert_eq!(d.siradaki(), Some(ButceIslemId(3)));
    }

    #[test]
    fn ayarsız_5b_öncesi_kayıt_devralınır() {
        let kayit = vec![
            istek_satiri(None),
            KayitOgesi::Kullanim {
                istem: 10,
                tamamlama: 5,
                dusunce: 0,
            },
            istek_satiri(None),
        ];
        let d = ButceDurumu::kayittan(&kayit).unwrap();
        assert_eq!(
            d.gorunum(),
            ButceGorunumu {
                baslatilan: 2,
                bilinen_token: 15,
                bilinmeyen: 1,
                aktif: 0,
            }
        );
        assert_eq!(d.siradaki(), Some(ButceIslemId(3)));
    }

    #[test]
    fn kurtarmada_sonuclanmamis_istek_bir_kez_bitirilir() {
        let kayit = vec![ayar(10, 100), basla(1)];
        let d = ButceDurumu::kayittan(&kayit).unwrap();
        assert_eq!(d.sonuclanmamis(), vec![ButceIslemId(1)]);
        // Kurtarma sonuçsuz başlangıcı bir kez kalıcı kapatır.
        let mut kapali = kayit.clone();
        kapali.push(bitis(1, None));
        let d = ButceDurumu::kayittan(&kapali).unwrap();
        assert!(d.sonuclanmamis().is_empty());
        let g = d.gorunum();
        assert_eq!((g.baslatilan, g.bilinmeyen), (1, 1));
        assert_eq!(d.siradaki(), Some(ButceIslemId(2)));
        // Aynı kayıt yeniden oynatılınca sayaç değişmez.
        let yeniden = ButceDurumu::kayittan(&kapali).unwrap();
        assert_eq!(yeniden, d);
    }

    #[test]
    fn ayni_sonuc_iki_kez_token_eklemez() {
        let mut d = ButceDurumu::yeni();
        let a = d.siradaki().unwrap();
        d.baslat(a).unwrap();
        d.bitir(a, tok(4, 3)).unwrap();
        assert_eq!(d.bitir(a, tok(4, 3)), Err(ButceHatasi::CiftSonuc(a)));
        assert_eq!(
            d.bitir(ButceIslemId(999), tok(9, 0)),
            Err(ButceHatasi::BilinmeyenIslem(ButceIslemId(999)))
        );
        assert_eq!(d.gorunum().bilinen_token, 7);
    }

    #[test]
    fn siradaki_durum_degistirmez_ve_yanlis_kimlik_reddedilir() {
        let mut d = ButceDurumu::yeni();
        let once = d.clone();
        assert_eq!(d.siradaki(), Some(ButceIslemId(1)));
        assert_eq!(d.siradaki(), Some(ButceIslemId(1)));
        assert_eq!(d, once);
        assert_eq!(
            d.baslat(ButceIslemId(2)),
            Err(ButceHatasi::SiraDisiKimlik(ButceIslemId(2)))
        );
        assert_eq!(d, once);
        d.baslat(ButceIslemId(1)).unwrap();
        assert_eq!(d.sonuclanmamis(), vec![ButceIslemId(1)]);
    }

    #[test]
    fn hata_kullanimi_uretimden_once_ret_ve_belirsiz_ayrimi() {
        let bilinen_sifir = Some(GunlukToken::default());
        let sunucu = |durum: u16| EvrenError::Sunucu {
            durum,
            kod: None,
            mesaj: String::new(),
        };
        // Üretimden önce reddedilen denemeler: istek sayılır, token bilinen 0.
        for hata in [
            EvrenError::AnahtarGecersiz,
            EvrenError::SartKabulGerekli { surum: None },
            EvrenError::ModelBulunamadi {
                mesaj: String::new(),
                oneriler: Vec::new(),
            },
            EvrenError::HizSiniri { tekrar_dene: None },
            EvrenError::GunlukSinir { sifirlanma: None },
            EvrenError::BaglamAsimi {
                mesaj: String::new(),
            },
            sunucu(400),
            sunucu(429),
            sunucu(499),
            EvrenError::ag_baglanti("bağlantı yok"),
        ] {
            assert_eq!(hata_kullanimi(&hata), bilinen_sifir, "{hata:?}");
        }
        // Belirsiz: üretim sonrası ya da sunucu tarafı; token bilinmez.
        for hata in [
            EvrenError::AkisYarimKaldi,
            EvrenError::YanitSiniri,
            EvrenError::GecersizYanit(String::new()),
            sunucu(500),
            sunucu(503),
            EvrenError::ag_yanit("gövde gitti"),
        ] {
            assert_eq!(hata_kullanimi(&hata), None, "{hata:?}");
        }
    }

    #[test]
    fn sonuclanmamis_baslangic_gorunur() {
        let kayit = vec![ayar(10, 100), basla(1), basla(2), bitis(2, tok(1, 1))];
        let d = ButceDurumu::kayittan(&kayit).unwrap();
        assert_eq!(d.sonuclanmamis(), vec![ButceIslemId(1)]);
        assert_eq!(d.gorunum().aktif, 1);
    }

    #[test]
    fn kayit_hatalari_tipli_doner() {
        assert_eq!(
            ButceDurumu::kayittan(&[ayar(10, 100), bitis(1, None)]),
            Err(ButceHatasi::BilinmeyenIslem(ButceIslemId(1)))
        );
        assert_eq!(
            ButceDurumu::kayittan(&[ayar(10, 100), basla(1), bitis(1, None), bitis(1, None)]),
            Err(ButceHatasi::CiftSonuc(ButceIslemId(1)))
        );
        assert_eq!(
            ButceDurumu::kayittan(&[ayar(10, 100), basla(2)]),
            Err(ButceHatasi::SiraDisiKimlik(ButceIslemId(2)))
        );
    }

    #[test]
    fn eski_kayit_devri_gonderilen_denemeleri_ve_tokeni_toplar() {
        // Bir isteğin birden çok (birikimli) `Kullanim`'ından yalnız sonuncusu
        // sayılır; eski gönderim günlüğü de sonuncuyu iliştirirdi.
        let kayit = vec![
            istek_satiri(None),
            KayitOgesi::Kullanim {
                istem: 10,
                tamamlama: 5,
                dusunce: 2,
            },
            istek_satiri(None),
            eklenti_satiri(IstekSonucu::Gonderilmedi {
                neden: "önizleme".into(),
            }),
            eklenti_satiri(IstekSonucu::Basarili),
            KayitOgesi::Kullanim {
                istem: 3,
                tamamlama: 0,
                dusunce: 0,
            },
            KayitOgesi::Kullanim {
                istem: 4,
                tamamlama: 1,
                dusunce: 0,
            },
        ];
        assert_eq!(
            eski_kayit_devri(&kayit),
            ButceDevri {
                istek: 3,
                token: 20,
                belirsiz: true,
            }
        );
        // İstek yoksa token da bilinen sayılmaz.
        let yalniz_kullanim = vec![KayitOgesi::Kullanim {
            istem: 1,
            tamamlama: 1,
            dusunce: 0,
        }];
        assert_eq!(
            eski_kayit_devri(&yalniz_kullanim),
            ButceDevri {
                istek: 0,
                token: 0,
                belirsiz: false,
            }
        );
    }

    #[test]
    fn yeni_kayit_ogeleri_serde_gidis_donus_yapar() {
        let ogeler = vec![
            ayar(10, 100),
            basla(1),
            bitis(1, None),
            KayitOgesi::ButceDevamKarari {
                sinirlar: eski_sinir(20, 100),
                onaylanan_bilinmeyen: 1,
            },
            KayitOgesi::ButceAyarlandi {
                sinirlar: eski_sinir(10, 100),
                devir: Some(ButceDevri {
                    istek: 1,
                    token: 2,
                    belirsiz: true,
                }),
            },
        ];
        for oge in ogeler {
            let json = serde_json::to_string(&oge).unwrap();
            let geri: KayitOgesi = serde_json::from_str(&json).unwrap();
            assert_eq!(geri, oge, "{json}");
        }
        // Sonuçsuz kullanım alanı yazılır; eksik alanla karışmaz.
        let json = serde_json::to_string(&bitis(1, None)).unwrap();
        assert!(json.contains(r#""kullanim":null"#), "{json}");
    }

    #[test]
    fn butce_islemi_olmayan_eski_istek_okunur() {
        let eski = r#"{"tip":"Istek","amac":"ana","istek_id":null,"kredi":null,"onceki":0,"sonuc":{"tip":"Basarili"}}"#;
        let oge: KayitOgesi = serde_json::from_str(eski).unwrap();
        assert!(matches!(
            oge,
            KayitOgesi::Istek {
                butce_islem: None,
                ..
            }
        ));
        let yeni = istek_satiri(Some(ButceIslemId(7)));
        let json = serde_json::to_string(&yeni).unwrap();
        assert!(json.contains(r#""butce_islem":7"#), "{json}");
        assert_eq!(serde_json::from_str::<KayitOgesi>(&json).unwrap(), yeni);
    }

    #[tokio::test]
    async fn izin_oturum_kapaninca_none_doner() {
        let (talepci, alici) = ButceTalepci::kanal();
        drop(alici);
        assert_eq!(talepci.izin("ana").await, None);
    }

    #[tokio::test]
    async fn izin_verilen_kimligi_dondurur() {
        let (talepci, mut alici) = ButceTalepci::kanal();
        let izin = tokio::spawn(async move { talepci.izin("denetci").await });
        let Some(ButceMesaji::Talep(talep)) = alici.recv().await else {
            panic!("talep gelmeli");
        };
        assert_eq!(talep.amac, "denetci");
        talep
            .yanit
            .send(Some(ButceIslemId(4)))
            .expect("çağıran bekliyor");
        assert_eq!(izin.await.expect("görev bitmeli"), Some(ButceIslemId(4)));
    }

    #[tokio::test]
    async fn izin_cevap_gelmeden_dusurulurse_none_doner() {
        let (talepci, mut alici) = ButceTalepci::kanal();
        let izin = tokio::spawn(async move { talepci.izin("ozet").await });
        let Some(ButceMesaji::Talep(talep)) = alici.recv().await else {
            panic!("talep gelmeli");
        };
        drop(talep.yanit);
        assert_eq!(izin.await.expect("görev bitmeli"), None);
    }

    #[test]
    fn yanit_kullanimi_eksik_ya_da_tutarsizi_bilinmeyen_sayar() {
        let kullanim = |istem, tamamlama, toplam| Usage {
            prompt_tokens: istem,
            completion_tokens: tamamlama,
            total_tokens: toplam,
            completion_tokens_details: None,
            evren: None,
        };
        assert_eq!(yanit_kullanimi(None), None);
        assert_eq!(
            yanit_kullanimi(Some(&kullanim(10, 5, 15))),
            Some(GunlukToken {
                istem: 10,
                tamamlama: 5,
                dusunce: 0,
            })
        );
        // Boş ya da toplamı tutmayan kullanım bilinmeyendir; sıfır değildir.
        assert_eq!(yanit_kullanimi(Some(&kullanim(0, 0, 0))), None);
        assert_eq!(yanit_kullanimi(Some(&kullanim(10, 5, 16))), None);
    }

    #[tokio::test(start_paused = true)]
    async fn sureli_yalniz_gecen_sureyi_harcar() {
        let mut kalan = Duration::from_secs(30);
        assert_eq!(
            sureli(&mut kalan, tokio::time::sleep(Duration::from_secs(10))).await,
            Some(())
        );
        assert_eq!(kalan, Duration::from_secs(20));
        assert_eq!(
            sureli(&mut kalan, tokio::time::sleep(Duration::from_secs(60))).await,
            None
        );
        assert_eq!(kalan, Duration::ZERO);
    }
}
