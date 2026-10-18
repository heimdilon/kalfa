//! Çekirdekten arayüze olay aktarımı: paketleme ve kredi (1c tasarımı §4).
//!
//! Paketleyici olay kuyruğunu okur ve olayları kısa bir pencerede paketler.
//! Arayüzün "işledim" demediği veri krediyi doldurunca kuyruktan okumayı
//! bırakır: arayüz takılırsa birikim webview'da değil, sınırı belli olan
//! çekirdek kuyruğunda olur ve kuyruk dolunca oturumun iptal yolu çalışır.

use std::collections::VecDeque;
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::Notify;
use tokio::time::Instant;

use crate::oturum::OlayAlici;
use crate::protokol::{Olay, Zarf};

/// Paketleyici ayarları. Varsayılanlar gerçek arayüzle yapılan gecikme
/// ölçümüyle seçildi (1c tasarımı §11.4): 16 ms'lik pencerede token'ın ekrana
/// düşmesinin p95'i Windows'ta 41, Linux'ta 47 ms'ydi; 8 ms'de 26 ve 30–42 ms.
/// Paket sınırı ve kredi başlangıç değerinde kaldı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AktarimAyarlari {
    /// İlk olaydan sonra aynı pakete olay toplanan süre (varsayılan 8 ms).
    pub pencere: Duration,
    /// Paketin hedef bayt üst sınırı; sınırdan büyük tek olay yine gider.
    pub azami_paket_bayt: usize,
    /// Arayüzün "işledim" demediği en fazla paket sayısı (en az 1 sayılır).
    pub kredi_paket: usize,
    /// Arayüzün "işledim" demediği en fazla bayt.
    pub kredi_bayt: usize,
}

impl Default for AktarimAyarlari {
    fn default() -> Self {
        Self {
            pencere: Duration::from_millis(8),
            azami_paket_bayt: 64 * 1024,
            kredi_paket: 4,
            kredi_bayt: 256 * 1024,
        }
    }
}

/// Arayüze giden kanal mesajı (1c tasarımı §4.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum KanalMesaji {
    /// Bir paket olay. `no` her abonelikte 1'den başlar ve boşluksuz artar.
    Olaylar { no: u64, zarflar: Vec<Zarf<Olay>> },
    /// Olay kuyruğu kapandı; bu abonelikten başka mesaj gelmez.
    Kapandi { no: u64 },
}

impl KanalMesaji {
    pub fn no(&self) -> u64 {
        match self {
            Self::Olaylar { no, .. } | Self::Kapandi { no } => *no,
        }
    }
}

/// Paketleyicinin okuduğu olay kaynağı. `recv` iptale dayanıklı olmalıdır:
/// dönen future bırakılırsa olay kaybolmamalıdır.
pub trait OlayKaynagi: Send {
    fn recv(&mut self) -> impl Future<Output = Option<Zarf<Olay>>> + Send;
}

impl OlayKaynagi for OlayAlici {
    fn recv(&mut self) -> impl Future<Output = Option<Zarf<Olay>>> + Send {
        OlayAlici::recv(self)
    }
}

/// Onay, hata, tamamlanma, araç, şart ve aktarım olayları birleştirilmez ve
/// pencereyi beklemeden gönderilir (1c tasarımı §4.2).
pub fn kritik_mi(olay: &Olay) -> bool {
    matches!(
        olay,
        Olay::OnayGerekiyor { .. }
            | Olay::Hata { .. }
            | Olay::Tamamlandi { .. }
            | Olay::AracCagrisiBasladi { .. }
            | Olay::DosyaDegisiklikleri { .. }
            | Olay::AracSonucu { .. }
            | Olay::SartOnayiGerekiyor { .. }
            | Olay::ButceKarariGerekli { .. }
            | Olay::AktarimDurdu { .. }
            | Olay::GeriAlmaPlani { .. }
            | Olay::TurGeriAlindi { .. }
            | Olay::GeriAlmaVerisiSilindi { .. }
            | Olay::TurOzeti { .. }
            | Olay::GonderimOnizlemesi { .. }
            | Olay::DenetimBasladi { .. }
            | Olay::DenetimSonucu { .. }
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum IsledimHatasi {
    #[error("{no} numaralı paket henüz gönderilmedi (son gönderilen: {son})")]
    GonderilmemisPaket { no: u64, son: u64 },
}

#[derive(Debug, Default)]
struct KrediDurumu {
    /// "İşledim" beklenen paketler: (numara, bayt).
    yolda: VecDeque<(u64, usize)>,
    yolda_bayt: usize,
    son_gonderilen: u64,
}

#[derive(Debug, Default)]
struct KrediOrtak {
    durum: Mutex<KrediDurumu>,
    uyandir: Notify,
}

impl KrediOrtak {
    fn durum(&self) -> MutexGuard<'_, KrediDurumu> {
        self.durum.lock().unwrap_or_else(|z| z.into_inner())
    }
}

/// Arayüzün "işledim" bildirimini paketleyiciye iletir. Paketleyici başka bir
/// görevde çalışırken de kullanılabilir.
#[derive(Debug, Clone)]
pub struct KrediTutamaci {
    ortak: Arc<KrediOrtak>,
}

impl KrediTutamaci {
    /// Arayüz `no` numaralı pakete kadar (o dahil) bütün paketleri işledi.
    /// Aynı ya da daha eski numarayı yeniden bildirmek zararsızdır.
    pub fn isledim(&self, no: u64) -> Result<(), IsledimHatasi> {
        let mut d = self.ortak.durum();
        if no > d.son_gonderilen {
            return Err(IsledimHatasi::GonderilmemisPaket {
                no,
                son: d.son_gonderilen,
            });
        }
        while d.yolda.front().is_some_and(|&(ilk, _)| ilk <= no) {
            if let Some((_, bayt)) = d.yolda.pop_front() {
                d.yolda_bayt = d.yolda_bayt.saturating_sub(bayt);
            }
        }
        drop(d);
        self.ortak.uyandir.notify_one();
        Ok(())
    }

    /// "İşledim" beklenen paket sayısı ve bayt toplamı.
    pub fn yolda(&self) -> (usize, usize) {
        let d = self.ortak.durum();
        (d.yolda.len(), d.yolda_bayt)
    }
}

/// Serileştirilmiş boyutu bilinen olay.
struct OlculuOlay {
    zarf: Zarf<Olay>,
    bayt: usize,
}

impl OlculuOlay {
    fn yeni(zarf: Zarf<Olay>) -> Self {
        let bayt = olc(&zarf);
        Self { zarf, bayt }
    }
}

fn olc<T: Serialize>(deger: &T) -> usize {
    serde_json::to_vec(deger).map_or(usize::MAX, |v| v.len())
}

fn olaylar_on_eki(no: u64) -> usize {
    olc(&KanalMesaji::Olaylar {
        no,
        zarflar: Vec::new(),
    })
}

/// Aynı turdaki bitişik iki metin parçasını birleştirir; birleşen parça son
/// parçanın sırasını taşır. Araç çıktısı yalnızca aynı çağrının aynı
/// akışındaki parçayla birleşir (aşama 2 tasarımı §3.3).
fn birlestir(onceki: &Zarf<Olay>, yeni: &Zarf<Olay>) -> Option<Zarf<Olay>> {
    if onceki.oturum != yeni.oturum || onceki.tur != yeni.tur {
        return None;
    }
    let yuk = match (&onceki.yuk, &yeni.yuk) {
        (Olay::TokenParcasi { metin: a }, Olay::TokenParcasi { metin: b }) => Olay::TokenParcasi {
            metin: format!("{a}{b}"),
        },
        (Olay::DusunceParcasi { metin: a }, Olay::DusunceParcasi { metin: b }) => {
            Olay::DusunceParcasi {
                metin: format!("{a}{b}"),
            }
        }
        (
            Olay::AracCiktisiParcasi {
                cagri: c1,
                akis: a1,
                metin: m1,
                atlanan_bayt: t1,
            },
            Olay::AracCiktisiParcasi {
                cagri: c2,
                akis: a2,
                metin: m2,
                atlanan_bayt: t2,
            },
        ) if c1 == c2 && a1 == a2 && *t2 == 0 => Olay::AracCiktisiParcasi {
            cagri: *c1,
            akis: *a1,
            metin: format!("{m1}{m2}"),
            atlanan_bayt: *t1,
        },
        _ => return None,
    };
    Some(Zarf {
        sema: yeni.sema,
        oturum: yeni.oturum,
        tur: yeni.tur,
        sira: yeni.sira,
        yuk,
    })
}

/// Doldurulmakta olan paket.
struct Paket {
    zarflar: Vec<Zarf<Olay>>,
    /// Kanal mesajının serileştirilmiş boyutu.
    bayt: usize,
    /// Son zarfın boyutu; birleştirmede güncellenir.
    son_bayt: usize,
}

impl Paket {
    fn yeni(ilk: OlculuOlay, on_ek_bayt: usize) -> Self {
        Self {
            bayt: on_ek_bayt.saturating_add(ilk.bayt),
            son_bayt: ilk.bayt,
            zarflar: vec![ilk.zarf],
        }
    }

    /// Olayı ekler; son parçayla birleşebiliyorsa birleştirir. Sınırı
    /// aşacaksa eklemez ve olayı geri verir.
    fn ekle(&mut self, olay: OlculuOlay, sinir: usize) -> Result<(), Box<OlculuOlay>> {
        let birlesik = self
            .zarflar
            .last()
            .and_then(|son| birlestir(son, &olay.zarf));
        if let Some(birlesik) = birlesik {
            let yeni_bayt = olc(&birlesik);
            let toplam = self
                .bayt
                .saturating_sub(self.son_bayt)
                .saturating_add(yeni_bayt);
            if toplam > sinir {
                return Err(Box::new(olay));
            }
            if let Some(son) = self.zarflar.last_mut() {
                *son = birlesik;
            }
            self.bayt = toplam;
            self.son_bayt = yeni_bayt;
            return Ok(());
        }
        let toplam = self.bayt.saturating_add(olay.bayt).saturating_add(1);
        if toplam > sinir {
            return Err(Box::new(olay));
        }
        self.bayt = toplam;
        self.son_bayt = olay.bayt;
        self.zarflar.push(olay.zarf);
        Ok(())
    }
}

/// Olay kuyruğunu paketleyip kanala hazırlayan birim.
pub struct Paketleyici<K: OlayKaynagi> {
    kaynak: K,
    ayarlar: AktarimAyarlari,
    ortak: Arc<KrediOrtak>,
    sonraki_no: u64,
    /// Kaynaktan alınmış ama henüz bir pakete girmemiş olay.
    bekleyen: Option<OlculuOlay>,
    kaynak_kapandi: bool,
    bitti: bool,
}

impl<K: OlayKaynagi> Paketleyici<K> {
    pub fn yeni(kaynak: K, ayarlar: AktarimAyarlari) -> (Self, KrediTutamaci) {
        let ortak = Arc::new(KrediOrtak::default());
        let paketleyici = Self {
            kaynak,
            ayarlar,
            ortak: ortak.clone(),
            sonraki_no: 1,
            bekleyen: None,
            kaynak_kapandi: false,
            bitti: false,
        };
        (paketleyici, KrediTutamaci { ortak })
    }

    /// Sonraki kanal mesajı; `Kapandi`dan sonra `None`.
    ///
    /// Kredi yoksa "işledim" gelene kadar bekler ve bu sırada kaynaktan okumaz.
    /// Kredi beklenirken future bırakılırsa olay kaybolmaz; paket doldurulurken
    /// bırakılırsa o paketin olayları kaybolur. Köprü bu yüzden `sonraki`yi
    /// ayrı bir görevde, `select!` dışında çağırır.
    pub async fn sonraki(&mut self) -> Option<KanalMesaji> {
        if self.bitti {
            return None;
        }
        self.kredi_bekle(1).await;
        if self.bekleyen.is_none() {
            let zarf = if self.kaynak_kapandi {
                None
            } else {
                self.kaynak.recv().await
            };
            match zarf {
                Some(zarf) => self.bekleyen = Some(OlculuOlay::yeni(zarf)),
                None => return Some(self.kapanis()),
            }
        }
        let gereken = self.bekleyen.as_ref().map_or(0, |o| {
            olc(&KanalMesaji::Olaylar {
                no: self.sonraki_no,
                zarflar: vec![o.zarf.clone()],
            })
        });
        self.kredi_bekle(gereken).await;
        let ilk = self.bekleyen.take()?;
        let sinir = self.paket_siniri();
        let zarflar = self.paketle(ilk, sinir).await;
        Some(self.gonder(zarflar))
    }

    /// İlk olayla başlayan paketi pencere dolana, bayt sınırına ulaşılana ya
    /// da kritik bir olay gelene kadar doldurur.
    async fn paketle(&mut self, ilk: OlculuOlay, sinir: usize) -> Vec<Zarf<Olay>> {
        let ilk_kritik = kritik_mi(&ilk.zarf.yuk);
        let on_ek_bayt = olaylar_on_eki(self.sonraki_no);
        let mut paket = Paket::yeni(ilk, on_ek_bayt);
        if ilk_kritik {
            return paket.zarflar;
        }
        let son = Instant::now() + self.ayarlar.pencere;
        while paket.bayt < sinir {
            let olay = match tokio::time::timeout_at(son, self.kaynak.recv()).await {
                Ok(Some(zarf)) => OlculuOlay::yeni(zarf),
                Ok(None) => {
                    self.kaynak_kapandi = true;
                    break;
                }
                Err(_) => break,
            };
            let kritik = kritik_mi(&olay.zarf.yuk);
            if let Err(sigmayan) = paket.ekle(olay, sinir) {
                self.bekleyen = Some(*sigmayan);
                break;
            }
            if kritik {
                break;
            }
        }
        paket.zarflar
    }

    fn paket_siniri(&self) -> usize {
        let yolda_bayt = self.ortak.durum().yolda_bayt;
        let kalan = self.ayarlar.kredi_bayt.saturating_sub(yolda_bayt);
        kalan.min(self.ayarlar.azami_paket_bayt)
    }

    /// `bayt` büyüklüğünde bir olay krediye sığana ya da yol boşalana kadar
    /// bekler.
    async fn kredi_bekle(&self, bayt: usize) {
        loop {
            let bildirim = self.ortak.uyandir.notified();
            if self.kredi_var(bayt) {
                return;
            }
            bildirim.await;
        }
    }

    fn kredi_var(&self, bayt: usize) -> bool {
        let d = self.ortak.durum();
        d.yolda.len() < self.ayarlar.kredi_paket.max(1)
            && (d.yolda.is_empty() || d.yolda_bayt.saturating_add(bayt) <= self.ayarlar.kredi_bayt)
    }

    fn gonder(&mut self, zarflar: Vec<Zarf<Olay>>) -> KanalMesaji {
        let no = self.numara_al();
        let mesaj = KanalMesaji::Olaylar { no, zarflar };
        let bayt = olc(&mesaj);
        let mut d = self.ortak.durum();
        d.yolda.push_back((no, bayt));
        d.yolda_bayt = d.yolda_bayt.saturating_add(bayt);
        d.son_gonderilen = no;
        drop(d);
        mesaj
    }

    fn kapanis(&mut self) -> KanalMesaji {
        self.bitti = true;
        let no = self.numara_al();
        self.ortak.durum().son_gonderilen = no;
        KanalMesaji::Kapandi { no }
    }

    fn numara_al(&mut self) -> u64 {
        let no = self.sonraki_no;
        self.sonraki_no += 1;
        no
    }
}
