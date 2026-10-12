//! Oturum bütçesi (5b tasarımı §4–§8): istek ve bilinen token sınırlarının
//! saf sayaç geçişleri. Bu modül ağa çıkmaz, kanal ve kayıt yazmaz; oturum
//! başlangıç ve karar kaydını kalıcı yazdıktan sonra bu durumu uygular.

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

/// Yeni oturumun varsayılan istek sınırı (5b tasarımı §2).
pub const VARSAYILAN_ISTEK: u64 = 200;
/// Yeni oturumun varsayılan bilinen token sınırı (5b tasarımı §2).
pub const VARSAYILAN_TOKEN: u64 = 5_000_000;
/// JavaScript'in güvenli tamsayı üst sınırı (2^53 - 1); arayüz girdileri bunu aşamaz.
pub const AZAMI_SINIR: u64 = 9_007_199_254_740_991;

/// İstek ve bilinen token sınırları. 0 sınırsız anlamına gelmez; geçersizdir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButceSinirlari {
    pub istek: u64,
    pub token: u64,
}

impl ButceSinirlari {
    /// İkisi de `1..=AZAMI_SINIR` ise `Some`.
    pub fn yeni(istek: u64, token: u64) -> Option<Self> {
        let sinirlar = Self { istek, token };
        sinirlar.gecerli().then_some(sinirlar)
    }

    pub fn gecerli(&self) -> bool {
        (1..=AZAMI_SINIR).contains(&self.istek) && (1..=AZAMI_SINIR).contains(&self.token)
    }
}

impl Default for ButceSinirlari {
    fn default() -> Self {
        Self {
            istek: VARSAYILAN_ISTEK,
            token: VARSAYILAN_TOKEN,
        }
    }
}

/// Oturum içinde 1'den artan bütçe denemesi kimliği.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ButceIslemId(pub u64);

impl fmt::Display for ButceIslemId {
    fn fmt(&self, bicim: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(bicim, "{}", self.0)
    }
}

/// Yeni izni kapatan neden; öncelik sırası: Bilinmeyen, Token, Istek.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ButceEngeli {
    Bilinmeyen,
    Token,
    Istek,
}

/// Bütçe kuralı ihlalleri ve kayıt tutarsızlıkları.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ButceHatasi {
    #[error(
        "Bütçe sınırı geçersiz: istek ve token sınırları 1 ile 9007199254740991 arasında olmalı ve tüketimin üstünde kalmalıdır."
    )]
    GecersizSinir,
    #[error("Bütçe işlemi beklenen sırada değil: {0}.")]
    SiraDisiKimlik(ButceIslemId),
    #[error("Bilinmeyen bütçe işlemi: {0}.")]
    BilinmeyenIslem(ButceIslemId),
    #[error("Bütçe işlemi zaten sonuçlandı: {0}.")]
    CiftSonuc(ButceIslemId),
    #[error("Aktif bir istek sürüyor; bütçe kararı istek bitmeden verilemez.")]
    SurenIstek,
    #[error("Bütçe kararı eski: bilinmeyen tüketim kararın verildiği andan farklı.")]
    EskiKarar,
    #[error("Bütçe ayarı kayıtta birden fazla var.")]
    CiftAyar,
    #[error("Bütçe kaydı, bütçe ayarından önce geliyor.")]
    AyarsizKayit,
    #[error("Bütçe engeli nedeniyle yeni istek başlatılamaz.")]
    EngelVar(ButceEngeli),
}

/// Arayüze ve kurtarılan görünüme giden özet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButceGorunumu {
    pub sinirlar: ButceSinirlari,
    pub baslatilan: u64,
    pub bilinen_token: u64,
    pub bilinmeyen: u64,
    pub onaylanan_bilinmeyen: u64,
    pub aktif: u64,
    pub engel: Option<ButceEngeli>,
}

/// Bir oturumun bütçe durumu. Sayaçların tek sahibi oturumdur; bu tip yalnız
/// geçişleri ve kabul kurallarını tutar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButceDurumu {
    sinirlar: ButceSinirlari,
    baslatilan: u64,
    token: u64,
    bilinmeyen: u64,
    onaylanan_bilinmeyen: u64,
    aktif: BTreeSet<ButceIslemId>,
    biten: BTreeSet<ButceIslemId>,
}

impl ButceDurumu {
    pub fn yeni(sinirlar: ButceSinirlari) -> Result<Self, ButceHatasi> {
        if !sinirlar.gecerli() {
            return Err(ButceHatasi::GecersizSinir);
        }
        Ok(Self {
            sinirlar,
            baslatilan: 0,
            token: 0,
            bilinmeyen: 0,
            onaylanan_bilinmeyen: 0,
            aktif: BTreeSet::new(),
            biten: BTreeSet::new(),
        })
    }

    pub fn engel(&self) -> Option<ButceEngeli> {
        if self.bilinmeyen > self.onaylanan_bilinmeyen {
            Some(ButceEngeli::Bilinmeyen)
        } else if self.token >= self.sinirlar.token {
            Some(ButceEngeli::Token)
        } else if self.baslatilan >= self.sinirlar.istek {
            Some(ButceEngeli::Istek)
        } else {
            None
        }
    }

    /// Durumu değiştirmeden sıradaki kimliği verir ya da engeli döner.
    pub fn siradaki(&self) -> Result<ButceIslemId, ButceEngeli> {
        if let Some(engel) = self.engel() {
            return Err(engel);
        }
        self.siradaki_kimlik().ok_or(ButceEngeli::Istek)
    }

    /// Kimlik `siradaki()`'nin verdiği olmalıdır; engel varken hata döner.
    pub fn baslat(&mut self, islem: ButceIslemId) -> Result<(), ButceHatasi> {
        let beklenen = self.siradaki().map_err(ButceHatasi::EngelVar)?;
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

    /// Kullanıcı kararını, kalıcı yazmadan önce doğrular.
    pub fn devam_dogrula(
        &self,
        gozlenen_baslatilan: u64,
        gozlenen_bilinmeyen: u64,
        sinirlar: ButceSinirlari,
    ) -> Result<(), ButceHatasi> {
        if !self.aktif.is_empty() {
            return Err(ButceHatasi::SurenIstek);
        }
        if gozlenen_baslatilan != self.baslatilan || gozlenen_bilinmeyen != self.bilinmeyen {
            return Err(ButceHatasi::EskiKarar);
        }
        if !sinirlar.gecerli() || sinirlar.istek <= self.baslatilan || sinirlar.token <= self.token
        {
            return Err(ButceHatasi::GecersizSinir);
        }
        Ok(())
    }

    /// Doğrulanmış ve kalıcı yazılmış kararı uygular. Sayaçlar sıfırlanmaz.
    pub fn devam_uygula(&mut self, sinirlar: ButceSinirlari) {
        self.sinirlar = sinirlar;
        self.onaylanan_bilinmeyen = self.bilinmeyen;
    }

    /// Sonuçsuz (aktif) kimlikler, artan sırada.
    pub fn sonuclanmamis(&self) -> Vec<ButceIslemId> {
        self.aktif.iter().copied().collect()
    }

    pub fn gorunum(&self) -> ButceGorunumu {
        ButceGorunumu {
            sinirlar: self.sinirlar,
            baslatilan: self.baslatilan,
            bilinen_token: self.token,
            bilinmeyen: self.bilinmeyen,
            onaylanan_bilinmeyen: self.onaylanan_bilinmeyen,
            aktif: self.aktif.len() as u64,
            engel: self.engel(),
        }
    }

    /// Kayıt öğelerini sırayla oynatır. Bütçe ayarı yoksa `Ok(None)` (eski kayıt).
    pub fn kayittan<'a>(
        ogeler: impl IntoIterator<Item = &'a KayitOgesi>,
    ) -> Result<Option<Self>, ButceHatasi> {
        let mut durum: Option<Self> = None;
        for oge in ogeler {
            if !butce_kaydi_mi(oge) {
                continue;
            }
            if let Some(d) = durum.as_mut() {
                d.oynat(oge)?;
            } else if let KayitOgesi::ButceAyarlandi { sinirlar, devir } = oge {
                let mut yeni = Self::yeni(*sinirlar)?;
                if let Some(devir) = devir {
                    yeni.devral(*devir);
                }
                durum = Some(yeni);
            } else {
                return Err(ButceHatasi::AyarsizKayit);
            }
        }
        Ok(durum)
    }

    /// 5b öncesi kaydın bilinen tabanı; belirsizse ilk yeni istekten önce
    /// sorulsun diye bir bilinmeyen sayılır.
    fn devral(&mut self, devir: ButceDevri) {
        self.baslatilan = self.baslatilan.saturating_add(devir.istek);
        self.token = self.token.saturating_add(devir.token);
        if devir.belirsiz {
            self.bilinmeyen = self.bilinmeyen.saturating_add(1);
        }
    }

    /// Kimlik üretimi tükenmişse (sayaç taşması) `None`.
    fn siradaki_kimlik(&self) -> Option<ButceIslemId> {
        self.baslatilan.checked_add(1).map(ButceIslemId)
    }

    fn kimligi_kaydet(&mut self, islem: ButceIslemId) {
        self.baslatilan = islem.0;
        self.aktif.insert(islem);
    }

    /// Kalıcı kayıttan bir bütçe öğesini uygular. Engel denetlenmez: kayıt
    /// yazılmıştır; yalnız kimlik sırası, çiftlik ve karar tutarlılığı bakılır.
    fn oynat(&mut self, oge: &KayitOgesi) -> Result<(), ButceHatasi> {
        match oge {
            KayitOgesi::ButceAyarlandi { .. } => Err(ButceHatasi::CiftAyar),
            KayitOgesi::ButceIstegiBasladi { islem, .. } => {
                if self.siradaki_kimlik() != Some(*islem) {
                    return Err(ButceHatasi::SiraDisiKimlik(*islem));
                }
                self.kimligi_kaydet(*islem);
                Ok(())
            }
            KayitOgesi::ButceIstegiBitti { islem, kullanim } => self.bitir(*islem, *kullanim),
            KayitOgesi::ButceDevamKarari {
                sinirlar,
                onaylanan_bilinmeyen,
            } => {
                if !self.aktif.is_empty() {
                    return Err(ButceHatasi::SurenIstek);
                }
                if *onaylanan_bilinmeyen != self.bilinmeyen {
                    return Err(ButceHatasi::EskiKarar);
                }
                if !sinirlar.gecerli() {
                    return Err(ButceHatasi::GecersizSinir);
                }
                self.sinirlar = *sinirlar;
                self.onaylanan_bilinmeyen = *onaylanan_bilinmeyen;
                Ok(())
            }
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
/// token yalnız `Kullanim` satırlarının toplamıdır. Eski kayıtta tekrar ve
/// eklenti tokenı tam belirlenemediği için bir istek varsa belirsizdir.
pub fn eski_kayit_devri<'a>(ogeler: impl IntoIterator<Item = &'a KayitOgesi>) -> ButceDevri {
    let mut istek: u64 = 0;
    let mut token: u64 = 0;
    for oge in ogeler {
        match oge {
            KayitOgesi::Istek { .. } => istek = istek.saturating_add(1),
            KayitOgesi::EklentiIstegi { sonuc, .. }
                if !matches!(sonuc, IstekSonucu::Gonderilmedi { .. }) =>
            {
                istek = istek.saturating_add(1)
            }
            KayitOgesi::Kullanim {
                istem, tamamlama, ..
            } => token = token.saturating_add(istem.saturating_add(*tamamlama)),
            _ => {}
        }
    }
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
}

/// Oturuma bağlı izin istemcisi (5b tasarımı §4.1). Her oturumun kendi kanalı
/// vardır; iki oturum birbirinin sayacını tüketmez.
#[derive(Clone)]
pub struct ButceTalepci(mpsc::Sender<ButceMesaji>);

impl ButceTalepci {
    /// Oturumun talep kanalı: gönderici istemcide, alıcı oturum görevindedir.
    pub(crate) fn kanal() -> (Self, mpsc::Receiver<ButceMesaji>) {
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
    use std::sync::mpsc;
    use std::thread;

    use kalfa_evren::EkUc;

    use super::*;
    use crate::gonderim::EklentiIstekOzeti;
    use crate::kimlik::CagriId;

    fn sinir(istek: u64, token: u64) -> ButceSinirlari {
        ButceSinirlari { istek, token }
    }

    fn yeni_durum(istek: u64, token: u64) -> ButceDurumu {
        ButceDurumu::yeni(sinir(istek, token)).unwrap()
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
            sinirlar: sinir(istek, token),
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
    fn sinir_dogrulama_sifir_sinirsiz_degil() {
        assert_eq!(ButceSinirlari::yeni(0, 1), None);
        assert_eq!(ButceSinirlari::yeni(1, 0), None);
        assert_eq!(ButceSinirlari::yeni(AZAMI_SINIR + 1, 1), None);
        assert_eq!(ButceSinirlari::yeni(1, AZAMI_SINIR + 1), None);
        assert_eq!(
            ButceSinirlari::yeni(1, AZAMI_SINIR),
            Some(sinir(1, AZAMI_SINIR))
        );
        assert_eq!(
            ButceDurumu::yeni(sinir(0, 1)),
            Err(ButceHatasi::GecersizSinir)
        );
        assert_eq!(ButceSinirlari::default(), sinir(200, 5_000_000));
    }

    #[test]
    fn sinirdaki_istek_kabul_sonraki_red() {
        let mut d = yeni_durum(1, 100);
        let id = d.siradaki().unwrap();
        d.baslat(id).unwrap();
        d.bitir(id, tok(2, 1)).unwrap();
        assert_eq!(d.siradaki(), Err(ButceEngeli::Istek));
        assert_eq!(
            d.baslat(ButceIslemId(2)),
            Err(ButceHatasi::EngelVar(ButceEngeli::Istek))
        );
        assert_eq!((d.gorunum().baslatilan, d.gorunum().bilinen_token), (1, 3));
    }

    #[test]
    fn sekiz_paralel_talep_dort_izni_asamaz() {
        let (tx, rx) = mpsc::channel::<mpsc::Sender<Result<ButceIslemId, ButceHatasi>>>();
        let isler: Vec<_> = (0..8)
            .map(|_| {
                let tx = tx.clone();
                thread::spawn(move || {
                    let (yanit_tx, yanit_rx) = mpsc::channel();
                    tx.send(yanit_tx).unwrap();
                    yanit_rx.recv().unwrap()
                })
            })
            .collect();
        drop(tx);
        // Tek sahip: talepler sırayla karşılanır; her karar güncel sayaçla verilir.
        let mut d = yeni_durum(4, 100);
        for yanit in rx {
            let sonuc = d
                .siradaki()
                .map_err(ButceHatasi::EngelVar)
                .and_then(|id| d.baslat(id).map(|()| id));
            yanit.send(sonuc).unwrap();
        }
        let sonuclar: Vec<_> = isler.into_iter().map(|is| is.join().unwrap()).collect();
        assert_eq!(sonuclar.iter().filter(|s| s.is_ok()).count(), 4);
        assert_eq!(
            sonuclar
                .iter()
                .filter(|s| **s == Err(ButceHatasi::EngelVar(ButceEngeli::Istek)))
                .count(),
            4
        );
        assert_eq!(d.gorunum().baslatilan, 4);
        assert_eq!(d.sonuclanmamis().len(), 4);
    }

    #[test]
    fn token_sonradan_bilinir_asimdan_sonra_yeni_istek_yok() {
        let mut d = yeni_durum(10, 100);
        let a = d.siradaki().unwrap();
        d.baslat(a).unwrap();
        let b = d.siradaki().unwrap();
        d.baslat(b).unwrap();
        d.bitir(a, tok(50, 30)).unwrap();
        d.bitir(b, tok(40, 20)).unwrap();
        assert_eq!(d.gorunum().bilinen_token, 140);
        assert_eq!(d.siradaki(), Err(ButceEngeli::Token));
    }

    #[test]
    fn bilinmeyen_kullanim_yeni_istegi_keser_baslamis_olan_biter() {
        let mut d = yeni_durum(10, 100);
        let a = d.siradaki().unwrap();
        d.baslat(a).unwrap();
        let b = d.siradaki().unwrap();
        d.baslat(b).unwrap();
        d.bitir(a, None).unwrap();
        assert_eq!(d.siradaki(), Err(ButceEngeli::Bilinmeyen));
        let sinirlar = d.gorunum().sinirlar;
        assert_eq!(
            d.devam_dogrula(2, 1, sinirlar),
            Err(ButceHatasi::SurenIstek)
        );
        d.bitir(b, tok(15, 5)).unwrap();
        d.devam_dogrula(2, 1, sinirlar).unwrap();
        d.devam_uygula(sinirlar);
        let g = d.gorunum();
        assert_eq!((g.bilinen_token, g.bilinmeyen), (20, 1));
        assert!(d.siradaki().is_ok());
    }

    #[test]
    fn onay_gecmisi_sifirlamaz_yeni_bilinmeyen_yeniden_sorar() {
        let mut d = yeni_durum(10, 100);
        let a = d.siradaki().unwrap();
        d.baslat(a).unwrap();
        d.bitir(a, None).unwrap();
        let sinirlar = d.gorunum().sinirlar;
        d.devam_dogrula(1, 1, sinirlar).unwrap();
        d.devam_uygula(sinirlar);
        let b = d.siradaki().unwrap();
        d.baslat(b).unwrap();
        d.bitir(b, None).unwrap();
        assert_eq!(d.siradaki(), Err(ButceEngeli::Bilinmeyen));
        let g = d.gorunum();
        assert_eq!((g.bilinmeyen, g.onaylanan_bilinmeyen), (2, 1));
        // Eski gözlem (tek bilinmeyen) artık geçerli değil.
        assert_eq!(d.devam_dogrula(2, 1, sinirlar), Err(ButceHatasi::EskiKarar));
        d.devam_dogrula(2, 2, sinirlar).unwrap();
    }

    #[test]
    fn kurtarmada_sonuclanmamis_istek_bir_kez_bitirilir() {
        let kayit = vec![ayar(10, 100), basla(1)];
        let d = ButceDurumu::kayittan(&kayit).unwrap().unwrap();
        assert_eq!(d.sonuclanmamis(), vec![ButceIslemId(1)]);
        // Kurtarma sonuçsuz başlangıcı bir kez kalıcı kapatır.
        let mut kapali = kayit.clone();
        kapali.push(bitis(1, None));
        let d = ButceDurumu::kayittan(&kapali).unwrap().unwrap();
        assert!(d.sonuclanmamis().is_empty());
        let g = d.gorunum();
        assert_eq!((g.baslatilan, g.bilinmeyen), (1, 1));
        assert_eq!(d.siradaki(), Err(ButceEngeli::Bilinmeyen));
        // Aynı kayıt yeniden oynatılınca sayaç değişmez.
        let yeniden = ButceDurumu::kayittan(&kapali).unwrap().unwrap();
        assert_eq!(yeniden, d);
    }

    #[test]
    fn ayni_sonuc_iki_kez_token_eklemez() {
        let mut d = yeni_durum(10, 100);
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
    fn devam_sayaclari_sifirlamaz_yeni_mutlak_sinir_ister() {
        let mut d = yeni_durum(1, 100);
        let a = d.siradaki().unwrap();
        d.baslat(a).unwrap();
        d.bitir(a, tok(10, 10)).unwrap();
        assert_eq!(
            d.devam_dogrula(1, 0, sinir(1, 100)),
            Err(ButceHatasi::GecersizSinir)
        );
        let yeni = sinir(2, 100);
        d.devam_dogrula(1, 0, yeni).unwrap();
        d.devam_uygula(yeni);
        let g = d.gorunum();
        assert_eq!((g.baslatilan, g.bilinen_token), (1, 20));
        assert!(d.siradaki().is_ok());
    }

    #[test]
    fn token_tasmasi_kapiyi_acamaz() {
        let mut d = yeni_durum(10, AZAMI_SINIR);
        let a = d.siradaki().unwrap();
        d.baslat(a).unwrap();
        let b = d.siradaki().unwrap();
        d.baslat(b).unwrap();
        d.bitir(
            a,
            Some(GunlukToken {
                istem: u64::MAX - 1,
                tamamlama: 0,
                dusunce: 0,
            }),
        )
        .unwrap();
        d.bitir(b, tok(100, 0)).unwrap();
        assert_eq!(d.gorunum().bilinen_token, u64::MAX);
        assert_eq!(d.siradaki(), Err(ButceEngeli::Token));
    }

    #[test]
    fn izin_alicisi_kapanirsa_sonuc_kanali_bilinmeyeni_haber_verir() {
        let mut d = yeni_durum(10, 100);
        let id = d.siradaki().unwrap();
        d.baslat(id).unwrap();
        let (sonuc_tx, sonuc_rx) = mpsc::channel::<Option<GunlukToken>>();
        let (izin_tx, izin_rx) = mpsc::channel::<mpsc::Sender<Option<GunlukToken>>>();
        drop(izin_rx);
        // İzin teslim edilemezse taşınan sonuç göndericisi de bırakılır.
        drop(izin_tx.send(sonuc_tx).unwrap_err());
        // Kanal kapandığında kullanım bilinmez.
        let kullanim = sonuc_rx.recv().ok().flatten();
        d.bitir(id, kullanim).unwrap();
        let g = d.gorunum();
        assert_eq!((g.baslatilan, g.bilinmeyen), (1, 1));
        assert_eq!(d.siradaki(), Err(ButceEngeli::Bilinmeyen));
    }

    #[test]
    fn siradaki_durum_degistirmez_ve_yanlis_kimlik_reddedilir() {
        let mut d = yeni_durum(10, 100);
        let once = d.clone();
        assert_eq!(d.siradaki(), Ok(ButceIslemId(1)));
        assert_eq!(d.siradaki(), Ok(ButceIslemId(1)));
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
    fn ayarsiz_eski_kayit_butce_yok_sayilir() {
        let kayit = vec![istek_satiri(None), eklenti_satiri(IstekSonucu::Basarili)];
        assert_eq!(ButceDurumu::kayittan(&kayit), Ok(None));
    }

    #[test]
    fn oynatma_ayar_baslangic_ve_bitisi_gorunumu_kurar() {
        let kayit = vec![
            ayar(10, 100),
            basla(1),
            basla(2),
            bitis(1, tok(10, 5)),
            bitis(2, None),
        ];
        let d = ButceDurumu::kayittan(&kayit).unwrap().unwrap();
        assert_eq!(
            d.gorunum(),
            ButceGorunumu {
                sinirlar: sinir(10, 100),
                baslatilan: 2,
                bilinen_token: 15,
                bilinmeyen: 1,
                onaylanan_bilinmeyen: 0,
                aktif: 0,
                engel: Some(ButceEngeli::Bilinmeyen),
            }
        );
    }

    #[test]
    fn sonuclanmamis_baslangic_gorunur() {
        let kayit = vec![ayar(10, 100), basla(1), basla(2), bitis(2, tok(1, 1))];
        let d = ButceDurumu::kayittan(&kayit).unwrap().unwrap();
        assert_eq!(d.sonuclanmamis(), vec![ButceIslemId(1)]);
        assert_eq!(d.gorunum().aktif, 1);
    }

    #[test]
    fn kayit_hatalari_tipli_doner() {
        let karar = KayitOgesi::ButceDevamKarari {
            sinirlar: sinir(20, 100),
            onaylanan_bilinmeyen: 0,
        };
        assert_eq!(
            ButceDurumu::kayittan(&[basla(1)]),
            Err(ButceHatasi::AyarsizKayit)
        );
        assert_eq!(
            ButceDurumu::kayittan(&[ayar(10, 100), ayar(10, 100)]),
            Err(ButceHatasi::CiftAyar)
        );
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
        assert_eq!(
            ButceDurumu::kayittan(&[ayar(10, 100), basla(1), karar]),
            Err(ButceHatasi::SurenIstek)
        );
    }

    #[test]
    fn oynatilan_karar_gozlenen_bilinmeyeni_tutmazsa_eski_sayilir() {
        let eski_karar = KayitOgesi::ButceDevamKarari {
            sinirlar: sinir(20, 100),
            onaylanan_bilinmeyen: 0,
        };
        let kayit = vec![ayar(10, 100), basla(1), bitis(1, None), eski_karar];
        assert_eq!(ButceDurumu::kayittan(&kayit), Err(ButceHatasi::EskiKarar));
        let gecersiz_karar = KayitOgesi::ButceDevamKarari {
            sinirlar: sinir(0, 100),
            onaylanan_bilinmeyen: 1,
        };
        let kayit = vec![ayar(10, 100), basla(1), bitis(1, None), gecersiz_karar];
        assert_eq!(
            ButceDurumu::kayittan(&kayit),
            Err(ButceHatasi::GecersizSinir)
        );
        let dogru_karar = KayitOgesi::ButceDevamKarari {
            sinirlar: sinir(20, 100),
            onaylanan_bilinmeyen: 1,
        };
        let kayit = vec![ayar(10, 100), basla(1), bitis(1, None), dogru_karar];
        let d = ButceDurumu::kayittan(&kayit).unwrap().unwrap();
        assert_eq!(d.engel(), None);
    }

    #[test]
    fn belirsiz_devir_bilinmeyeni_sorar_ve_siradaki_kimligi_surdurur() {
        let kayit = vec![KayitOgesi::ButceAyarlandi {
            sinirlar: sinir(10, 2000),
            devir: Some(ButceDevri {
                istek: 3,
                token: 900,
                belirsiz: true,
            }),
        }];
        let mut d = ButceDurumu::kayittan(&kayit).unwrap().unwrap();
        assert_eq!(d.engel(), Some(ButceEngeli::Bilinmeyen));
        assert_eq!(d.siradaki(), Err(ButceEngeli::Bilinmeyen));
        let g = d.gorunum();
        assert_eq!((g.baslatilan, g.bilinen_token, g.bilinmeyen), (3, 900, 1));
        d.devam_dogrula(3, 1, g.sinirlar).unwrap();
        d.devam_uygula(g.sinirlar);
        assert_eq!(d.siradaki(), Ok(ButceIslemId(4)));
    }

    #[test]
    fn eski_kayit_devri_gonderilen_denemeleri_ve_tokeni_toplar() {
        let kayit = vec![
            istek_satiri(None),
            istek_satiri(None),
            eklenti_satiri(IstekSonucu::Gonderilmedi {
                neden: "önizleme".into(),
            }),
            eklenti_satiri(IstekSonucu::Basarili),
            KayitOgesi::Kullanim {
                istem: 10,
                tamamlama: 5,
                dusunce: 2,
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
                sinirlar: sinir(20, 100),
                onaylanan_bilinmeyen: 1,
            },
            KayitOgesi::ButceAyarlandi {
                sinirlar: sinir(10, 100),
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
