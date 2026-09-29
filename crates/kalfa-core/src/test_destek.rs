//! Test desteği: sahte sağlayıcı, sahte araç ve oturum sürücüsü.
//! Yalnızca `test-destek` özelliğiyle derlenir; burada `expect`/`panic` serbesttir.

use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures_util::{StreamExt, stream};
use kalfa_evren::tipler::{ChatRequest, FunctionCall, TermsStatus, TermsText, ToolCall};
use kalfa_evren::{AkisOlayi, EvrenError, YanitMeta};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::arac::{Arac, AracCiktisi, AracHatasi, AracKayitDefteri, Hazirlik, YazmaHedefi};
use crate::geri_alma::{DosyaDurumu, GeriYukleyici};
use crate::izin::Siniflandirici;
use crate::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
use crate::kayit::{KayitHedefi, OturumKaydi};
use crate::kilit::KaynakKilitleri;
use crate::muhur::ozet_hex;
use crate::oturum::{OlayAlici, OturumAyarlari, OturumBaglami, OturumTutamaci, baslat};
use crate::protokol::{BitisNedeni, Komut, Olay, Zarf};
use crate::saglayici::{Saglayici, SaglayiciAkisi};
use crate::sart_kapisi::SartKapisi;
use crate::yol::CalismaYolu;

static MODEL_CAGRI_SAYACI: AtomicU64 = AtomicU64::new(0);

/// Sahte sağlayıcının bir `akis()` çağrısına vereceği yanıt.
pub enum SahteYanit {
    /// Olayları sırayla verir, sonra biter.
    Akis(Vec<Result<AkisOlayi, EvrenError>>),
    /// Olayları verir, sonra hiç bitmez (iptal testleri için).
    Askida(Vec<AkisOlayi>),
    /// `akis()` doğrudan hata döner (HTTP hatası gibi).
    Hata(EvrenError),
}

impl SahteYanit {
    pub fn metin(metin: &str) -> Self {
        Self::Akis(vec![
            Ok(AkisOlayi::Metin(metin.into())),
            Ok(AkisOlayi::Bitti {
                neden: "stop".into(),
            }),
        ])
    }

    pub fn arac(ad: &str, argumanlar: Value) -> Self {
        Self::araclar(vec![(ad, argumanlar)])
    }

    pub fn araclar(cagrilar: Vec<(&str, Value)>) -> Self {
        let cagrilar = cagrilar
            .into_iter()
            .map(|(ad, argumanlar)| ToolCall {
                id: format!(
                    "model-cagri-{}",
                    MODEL_CAGRI_SAYACI.fetch_add(1, Ordering::SeqCst)
                ),
                kind: "function".into(),
                function: FunctionCall {
                    name: ad.into(),
                    arguments: argumanlar.to_string(),
                },
            })
            .collect();
        Self::Akis(vec![
            Ok(AkisOlayi::AracCagrilari(cagrilar)),
            Ok(AkisOlayi::Bitti {
                neden: "tool_calls".into(),
            }),
        ])
    }
}

struct SahteSart {
    surum: u32,
    metin: String,
    kabul_edilen: Vec<u32>,
}

pub struct SahteSaglayici {
    senaryo: Mutex<VecDeque<SahteYanit>>,
    istekler: Mutex<Vec<ChatRequest>>,
    sart: Mutex<SahteSart>,
    durum_hatasi: Mutex<Option<EvrenError>>,
}

impl SahteSaglayici {
    pub fn yeni(senaryo: Vec<SahteYanit>) -> Arc<Self> {
        Arc::new(Self {
            senaryo: Mutex::new(senaryo.into()),
            istekler: Mutex::new(Vec::new()),
            sart: Mutex::new(SahteSart {
                surum: 1,
                metin: "# Sahte kullanım şartları".into(),
                kabul_edilen: Vec::new(),
            }),
            durum_hatasi: Mutex::new(None),
        })
    }

    pub fn istekler(&self) -> Vec<ChatRequest> {
        self.istekler.lock().unwrap().clone()
    }

    pub fn kabul_edilen_surumler(&self) -> Vec<u32> {
        self.sart.lock().unwrap().kabul_edilen.clone()
    }

    pub fn sart_durumu_hatasi(&self, hata: Option<EvrenError>) {
        *self.durum_hatasi.lock().unwrap() = hata;
    }

    pub fn sart_surumunu_degistir(&self, surum: u32) {
        self.sart.lock().unwrap().surum = surum;
    }
}

#[async_trait]
impl Saglayici for SahteSaglayici {
    async fn akis(&self, istek: ChatRequest) -> Result<SaglayiciAkisi, EvrenError> {
        let model = istek.model.clone();
        let sira = {
            let mut istekler = self.istekler.lock().unwrap();
            istekler.push(istek);
            istekler.len()
        };
        let yanit = self
            .senaryo
            .lock()
            .unwrap()
            .pop_front()
            .expect("sahte senaryoda yanıt kalmadı");
        let meta = YanitMeta {
            istek_id: Some(format!("sahte-{sira}")),
            yonlendirilen_model: Some(model),
            ..Default::default()
        };
        match yanit {
            SahteYanit::Akis(olaylar) => Ok(SaglayiciAkisi {
                meta,
                olaylar: Box::pin(stream::iter(olaylar)),
            }),
            SahteYanit::Askida(olaylar) => Ok(SaglayiciAkisi {
                meta,
                olaylar: Box::pin(
                    stream::iter(olaylar.into_iter().map(Ok)).chain(stream::pending()),
                ),
            }),
            SahteYanit::Hata(hata) => Err(hata),
        }
    }

    async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError> {
        if let Some(hata) = self.durum_hatasi.lock().unwrap().clone() {
            return Err(hata);
        }
        let s = self.sart.lock().unwrap();
        Ok(TermsStatus {
            current_version: s.surum,
            is_material: true,
            accepted: s.kabul_edilen.contains(&s.surum),
            doc_path: None,
        })
    }

    async fn sart_metni(&self) -> Result<TermsText, EvrenError> {
        let s = self.sart.lock().unwrap();
        Ok(TermsText {
            version: s.surum,
            content: s.metin.clone(),
            doc_path: None,
        })
    }

    async fn sart_kabul(&self, surum: u32) -> Result<(), EvrenError> {
        let mut s = self.sart.lock().unwrap();
        if surum != s.surum {
            return Err(EvrenError::SartKabulGerekli {
                surum: Some(s.surum),
            });
        }
        s.kabul_edilen.push(surum);
        Ok(())
    }
}

/// Sahte yazan aracın hazırlığındaki önceki hâl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SahteOnceki {
    /// Önceki özet `surum-N`; içerik verilmez (1b'den beri).
    Yok,
    /// İçerik `surum-N`, özeti onun SHA-256'sı; sonraki içerik
    /// `sahte-<ad>`.
    Icerikli,
    /// Önceki hâl geri alma sınırını aşar.
    Buyuk,
    /// Dosya yoktu; sonraki içerik `sahte-<ad>`.
    Yoktu,
}

/// `{"yol": ...}` alan sahte araç. Okuyan ya da yazan olabilir.
pub struct SahteArac {
    ad: String,
    yazar: bool,
    /// Kabuk gibi davranır: hazırlık yalıtımlı bir kabuk tarifi taşır.
    kabuk: bool,
    yanit: String,
    hata: Option<String>,
    guvenlik_uyarilari: Vec<String>,
    /// Kabuk gibi değiştirdiği dosyalar (aşama 3a Task 6): çalışınca bunlar
    /// sonucun `degisiklikler`'ine girer; `dosyalar` verildiyse oraya da
    /// `kabuk-<ad>` yazılır. `None`: değişiklikler bilinmiyor.
    degistirdigi: Option<Vec<String>>,
    gecikme: Duration,
    on_hal: AtomicU64,
    /// Yazan aracın hazırlığında önceki içerik nasıl verilir (aşama 3a).
    onceki: SahteOnceki,
    /// Verilirse yazan araç bu bellekteki dosya sistemine yazar; önceki
    /// içerik de oradan okunur.
    dosyalar: Option<Arc<SahteGeriYukleyici>>,
    cagrilar: Mutex<Vec<Value>>,
    aktif: AtomicU32,
    azami_aktif: AtomicU32,
    sunulur: AtomicBool,
}

impl SahteArac {
    fn yeni(ad: &str, yazar: bool) -> Self {
        Self {
            ad: ad.into(),
            yazar,
            kabuk: false,
            yanit: format!("{ad} tamam"),
            hata: None,
            guvenlik_uyarilari: Vec::new(),
            degistirdigi: None,
            gecikme: Duration::ZERO,
            on_hal: AtomicU64::new(0),
            onceki: SahteOnceki::Yok,
            dosyalar: None,
            cagrilar: Mutex::new(Vec::new()),
            aktif: AtomicU32::new(0),
            azami_aktif: AtomicU32::new(0),
            sunulur: AtomicBool::new(true),
        }
    }

    pub fn okuyan(ad: &str) -> Self {
        Self::yeni(ad, false)
    }

    pub fn yazan(ad: &str) -> Self {
        Self::yeni(ad, true)
    }

    /// Yalıtımlı kabuk gibi hazırlanan sahte araç: `{"yol": ".", "ag": bool}`;
    /// komut `test`, dizin `yol`'dur.
    pub fn kabuk(ad: &str) -> Self {
        Self {
            kabuk: true,
            ..Self::yeni(ad, true)
        }
    }

    pub fn yanit(mut self, yanit: String) -> Self {
        self.yanit = yanit;
        self
    }

    pub fn hata(mut self, hata: String) -> Self {
        self.hata = Some(hata);
        self
    }

    pub fn guvenlik_uyarisi(mut self, mesaj: &str) -> Self {
        self.guvenlik_uyarilari.push(mesaj.into());
        self
    }

    /// Kabuğun sonucundaki değişiklikler; kabuk olmayan araçta ve
    /// `degistirdigi` verilmediyse yok.
    fn kabuk_degisiklikleri(&self) -> Option<Box<crate::arac::DosyaDegisiklikleri>> {
        let yollar = self.degistirdigi.as_ref().filter(|_| self.kabuk)?;
        Some(Box::new(crate::arac::DosyaDegisiklikleri::olustur(
            yollar
                .iter()
                .map(|y| (y.clone(), crate::arac::DegisiklikTuru::Degisti)),
            false,
        )))
    }

    /// Çalışınca bu dosyaları değiştirmiş gibi davranır (kabuk görüntüsü).
    pub fn degistirdigi(mut self, yollar: &[&str]) -> Self {
        self.degistirdigi = Some(yollar.iter().map(|y| (*y).to_string()).collect());
        self
    }

    pub fn gecikme(mut self, gecikme: Duration) -> Self {
        self.gecikme = gecikme;
        self
    }

    /// Yazan aracın hazırlığı önceki içeriği (`surum-N`) ve sonraki özeti
    /// (`sahte-<ad>`) taşır; geri alma bunları saklar.
    pub fn onceki_icerikli(mut self) -> Self {
        self.onceki = SahteOnceki::Icerikli;
        self
    }

    /// Yazan aracın "dosyası" yoktu: önceki özet yok, sonraki özet var.
    pub fn onceki_yok(mut self) -> Self {
        self.onceki = SahteOnceki::Yoktu;
        self
    }

    /// Yazan aracın önceki hâli geri alma sınırını aşar (`buyuk`).
    pub fn onceki_buyuk(mut self) -> Self {
        self.onceki = SahteOnceki::Buyuk;
        self
    }

    /// Yazan araç bellekteki dosya sistemine yazar: hazırlıkta önceki içerik
    /// oradan okunur (yoksa "yoktu"), çalışınca argümandaki `icerik` (yoksa
    /// `sahte-<ad>`) yazılır.
    pub fn dosyalar(mut self, dosyalar: Arc<SahteGeriYukleyici>) -> Self {
        self.dosyalar = Some(dosyalar);
        self
    }

    /// Sahte yazan aracın "yazdığı" içerik.
    pub fn sonraki_icerik(&self) -> String {
        format!("sahte-{}", self.ad)
    }

    /// Yazma hedefinin "dosyasını" dışarıdan değiştirir: önceki özet değişir.
    pub fn dosyayi_degistir(&self) {
        self.on_hal.fetch_add(1, Ordering::SeqCst);
    }

    pub fn cagri_sayisi(&self) -> usize {
        self.cagrilar.lock().unwrap().len()
    }

    pub fn azami_eszamanli(&self) -> u32 {
        self.azami_aktif.load(Ordering::SeqCst)
    }

    /// Aracın modele sunulup sunulmadığını değiştirir (`Arac::sunulur_mu`).
    pub fn sunulur_ayarla(&self, sunulur: bool) {
        self.sunulur.store(sunulur, Ordering::SeqCst);
    }
}

#[async_trait]
impl Arac for SahteArac {
    fn ad(&self) -> &str {
        &self.ad
    }

    fn aciklama(&self) -> &str {
        "sahte araç"
    }

    fn parametre_semasi(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "yol": {"type": "string"},
                "ag": {"type": "boolean"},
                "icerik": {"type": "string"}
            },
            "required": ["yol"],
            "additionalProperties": false
        })
    }

    fn yazar_mi(&self) -> bool {
        self.yazar
    }

    fn sunulur_mu(&self) -> bool {
        self.sunulur.load(Ordering::SeqCst)
    }

    async fn hazirla(&self, argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
        if self.kabuk {
            return Ok(Hazirlik {
                aciklama: "`test` komutunu çalıştırmak istiyor.".into(),
                tum_calisma_alani: true,
                kabuk: Some(KabukTarifi {
                    komut: "test".into(),
                    bicim: KabukBicimi::Program,
                    dizin: argumanlar["yol"].as_str().unwrap_or(".").to_string(),
                    ag: argumanlar["ag"].as_bool().unwrap_or(false),
                    yalitim: YalitimOzeti::Yalitimli {
                        arka_uc: "sahte".into(),
                        profil: "p".into(),
                    },
                    ilk_cagri: false,
                    alan_hazirligi: false,
                }),
                ..Default::default()
            });
        }
        let yol = CalismaYolu::coz(argumanlar["yol"].as_str().unwrap_or_default())?;
        let mut h = Hazirlik {
            aciklama: format!("sahte {} `{yol}`", self.ad),
            ..Default::default()
        };
        if self.yazar {
            let surum = self.on_hal.load(Ordering::SeqCst);
            let hedef = match (&self.dosyalar, self.onceki) {
                (Some(dosyalar), _) => {
                    let sonraki = argumanlar["icerik"]
                        .as_str()
                        .map_or_else(|| self.sonraki_icerik(), str::to_owned);
                    let mut hedef = YazmaHedefi {
                        sonraki_ozet: Some(ozet_hex(sonraki.as_bytes())),
                        ..YazmaHedefi::yeni(yol.clone(), None, 1)
                    };
                    if let Some((icerik, baglanti)) = dosyalar.ham(yol.as_str()) {
                        hedef.onceki_ozet = Some(ozet_hex(&icerik));
                        hedef.onceki_bayt = icerik.len() as u64;
                        hedef.onceki_icerik = Some(icerik.into());
                        hedef.baglanti_sayisi = baglanti;
                    }
                    hedef
                }
                (None, onceki) => match onceki {
                    SahteOnceki::Yok => YazmaHedefi::yeni(yol, Some(format!("surum-{surum}")), 1),
                    SahteOnceki::Icerikli => {
                        let onceki = format!("surum-{surum}").into_bytes();
                        YazmaHedefi {
                            onceki_ozet: Some(ozet_hex(&onceki)),
                            onceki_bayt: onceki.len() as u64,
                            onceki_icerik: Some(onceki.into()),
                            sonraki_ozet: Some(ozet_hex(self.sonraki_icerik().as_bytes())),
                            ..YazmaHedefi::yeni(yol, None, 1)
                        }
                    }
                    SahteOnceki::Yoktu => YazmaHedefi {
                        sonraki_ozet: Some(ozet_hex(self.sonraki_icerik().as_bytes())),
                        ..YazmaHedefi::yeni(yol, None, 1)
                    },
                    SahteOnceki::Buyuk => YazmaHedefi {
                        onceki_ozet: Some(ozet_hex(format!("buyuk-{surum}").as_bytes())),
                        onceki_bayt: crate::geri_alma::DOSYA_SINIRI + 1,
                        buyuk: true,
                        sonraki_ozet: Some(ozet_hex(self.sonraki_icerik().as_bytes())),
                        ..YazmaHedefi::yeni(yol, None, 1)
                    },
                },
            };
            h.yazilan.push(hedef);
            h.diff = Some(format!("+sahte {}", self.ad));
        } else {
            h.okunan.push(yol);
        }
        Ok(h)
    }

    async fn calistir(
        &self,
        argumanlar: &Value,
        _hazirlik: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.cagrilar.lock().unwrap().push(argumanlar.clone());
        if let (Some(dosyalar), true) = (&self.dosyalar, self.kabuk) {
            for yol in self.degistirdigi.iter().flatten() {
                dosyalar.yaz(yol, format!("kabuk-{}", self.ad).as_bytes());
            }
        } else if let (Some(dosyalar), true, None) = (&self.dosyalar, self.yazar, &self.hata) {
            let yol = argumanlar["yol"].as_str().unwrap_or_default();
            let icerik = argumanlar["icerik"]
                .as_str()
                .map_or_else(|| self.sonraki_icerik(), str::to_owned);
            dosyalar.yaz(yol, icerik.as_bytes());
        }
        let simdi = self.aktif.fetch_add(1, Ordering::SeqCst) + 1;
        self.azami_aktif.fetch_max(simdi, Ordering::SeqCst);
        let sonuc = tokio::select! {
            _ = tokio::time::sleep(self.gecikme) => match &self.hata {
                Some(h) => Err(AracHatasi::Gecersiz(h.clone())),
                None => Ok(AracCiktisi {
                    icerik: self.yanit.clone(),
                    kaynak: format!("sahte:{}", self.ad),
                    guvenlik_uyarilari: self.guvenlik_uyarilari.clone(),
                    degisiklikler: self.kabuk_degisiklikleri(),
                }),
            },
            _ = iptal.cancelled() => Err(AracHatasi::Iptal),
        };
        self.aktif.fetch_sub(1, Ordering::SeqCst);
        sonuc
    }
}

/// Bellekteki dosya sistemi: yol → (içerik, bağlantı sayısı). Geri alma
/// testlerinde çalışma alanının yerini tutar; sahte yazan araç da buraya
/// yazabilir (`SahteArac::dosyalar`).
#[derive(Default)]
pub struct SahteGeriYukleyici {
    dosyalar: Mutex<BTreeMap<String, (Vec<u8>, u64)>>,
    kaldirilan_dizinler: Mutex<Vec<String>>,
}

impl SahteGeriYukleyici {
    pub fn yeni() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn yaz(&self, yol: &str, icerik: &[u8]) {
        let mut d = self.dosyalar.lock().unwrap();
        let baglanti = d.get(yol).map_or(1, |(_, b)| *b);
        d.insert(yol.to_string(), (icerik.to_vec(), baglanti));
    }

    pub fn oku(&self, yol: &str) -> Option<Vec<u8>> {
        self.ham(yol).map(|(icerik, _)| icerik)
    }

    fn ham(&self, yol: &str) -> Option<(Vec<u8>, u64)> {
        self.dosyalar.lock().unwrap().get(yol).cloned()
    }

    pub fn sil(&self, yol: &str) {
        self.dosyalar.lock().unwrap().remove(yol);
    }

    /// Dosyaya ikinci bir ad (hard link) verilmiş gibi yapar.
    pub fn baglanti_ekle(&self, yol: &str) {
        if let Some((_, b)) = self.dosyalar.lock().unwrap().get_mut(yol) {
            *b += 1;
        }
    }

    /// `bos_dizini_kaldir`'ın çağrıldığı dizinler, sırayla.
    pub fn kaldirilan_dizinler(&self) -> Vec<String> {
        self.kaldirilan_dizinler.lock().unwrap().clone()
    }
}

impl GeriYukleyici for SahteGeriYukleyici {
    fn durum(&self, yol: &CalismaYolu) -> io::Result<Option<DosyaDurumu>> {
        Ok(self
            .ham(yol.as_str())
            .map(|(icerik, baglanti_sayisi)| DosyaDurumu {
                ozet: ozet_hex(&icerik),
                baglanti_sayisi,
            }))
    }

    fn geri_yaz(&self, yol: &CalismaYolu, icerik: Option<&[u8]>, beklenen: &str) -> io::Result<()> {
        let mut d = self.dosyalar.lock().unwrap();
        let Some((simdiki, baglanti)) = d.get(yol.as_str()) else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "dosya yok"));
        };
        if ozet_hex(simdiki) != beklenen {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Dosya önizlemeden sonra değişti; geri yazılmadı.",
            ));
        }
        if *baglanti > 1 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Dosyanın başka bir adı var (hard link).",
            ));
        }
        match icerik {
            Some(icerik) => {
                d.insert(yol.as_str().to_string(), (icerik.to_vec(), 1));
            }
            None => {
                d.remove(yol.as_str());
            }
        }
        Ok(())
    }

    fn bos_dizini_kaldir(&self, yol: &CalismaYolu) -> io::Result<bool> {
        self.kaldirilan_dizinler
            .lock()
            .unwrap()
            .push(yol.as_str().to_string());
        Ok(false)
    }
}

/// Oturumu bir test içinden süren yardımcı: komut gönderir, olayları zaman
/// aşımıyla bekler ve gelen her olayı `gecmis`'e yazar.
pub struct Surucu {
    pub tutamac: OturumTutamaci,
    olaylar: OlayAlici,
    pub gecmis: Vec<Zarf<Olay>>,
    zaman_asimi: Duration,
}

impl Surucu {
    pub fn yeni(tutamac: OturumTutamaci, olaylar: OlayAlici) -> Self {
        Self {
            tutamac,
            olaylar,
            gecmis: Vec::new(),
            zaman_asimi: Duration::from_secs(5),
        }
    }

    pub fn zaman_asimi(mut self, sure: Duration) -> Self {
        self.zaman_asimi = sure;
        self
    }

    pub async fn gonder(&self, komut: Komut) {
        self.tutamac
            .gonder(komut)
            .await
            .expect("oturum açık olmalı");
    }

    /// Olay alıcısını bırakır ve oturum görevi bitene kadar bekler; ardından
    /// kayıt yeniden açılabilir.
    pub async fn kapat(self) {
        let Self {
            tutamac, olaylar, ..
        } = self;
        drop(olaylar);
        tokio::time::timeout(Duration::from_secs(5), tutamac.kapanmasini_bekle())
            .await
            .expect("oturum kapanmalı");
    }

    pub async fn sonraki(&mut self) -> Zarf<Olay> {
        let zarf = tokio::time::timeout(self.zaman_asimi, self.olaylar.recv())
            .await
            .expect("olay beklerken zaman aşımı")
            .expect("olay kanalı kapandı");
        self.gecmis.push(zarf.clone());
        zarf
    }

    pub async fn bekle(&mut self, kosul: impl Fn(&Olay) -> bool) -> Zarf<Olay> {
        loop {
            let zarf = self.sonraki().await;
            if kosul(&zarf.yuk) {
                return zarf;
            }
        }
    }

    pub async fn tur_sonu(&mut self) -> BitisNedeni {
        match self
            .bekle(|o| matches!(o, Olay::Tamamlandi { .. }))
            .await
            .yuk
        {
            Olay::Tamamlandi { neden } => neden,
            diger => unreachable!("{diger:?}"),
        }
    }

    pub fn metin(&self) -> String {
        self.gecmis
            .iter()
            .filter_map(|z| match &z.yuk {
                Olay::TokenParcasi { metin } => Some(metin.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn hatalar(&self) -> Vec<String> {
        self.gecmis
            .iter()
            .filter_map(|z| match &z.yuk {
                Olay::Hata { mesaj } => Some(mesaj.clone()),
                _ => None,
            })
            .collect()
    }
}

/// Var olmayan bir kök: git koruması yoktur, yalnızca gizli kalıplar çalışır.
pub fn sanal_siniflandirici() -> Arc<Siniflandirici> {
    Arc::new(
        Siniflandirici::yeni(std::env::temp_dir().join("kalfa-sanal-calisma-alani"), &[])
            .expect("sınıflandırıcı"),
    )
}

/// Yeni bir kilit yöneticisiyle bağlam. Aynı çalışma alanını paylaşan iki
/// oturum için bağlamı `clone()`'la: kilitler ortak kalır.
pub fn test_baglami(
    saglayici: Arc<SahteSaglayici>,
    araclar: Vec<Arc<dyn Arac>>,
    siniflandirici: Arc<Siniflandirici>,
) -> OturumBaglami {
    OturumBaglami {
        sart_kapisi: Arc::new(SartKapisi::yeni(saglayici.clone())),
        saglayici,
        araclar: Arc::new(AracKayitDefteri::yeni(araclar).expect("araç kaydı")),
        siniflandirici,
        kilitler: Arc::new(KaynakKilitleri::yeni()),
        geri_yukleyici: None,
    }
}

/// Yazma hatası benzeten kayıt hedefi (spec §6.6 testleri).
pub struct BozulanHedef {
    ekleme_bozuk: bool,
    kalan_kalici: usize,
}

impl BozulanHedef {
    /// Her yazım başarısız olur.
    pub fn hep_bozuk() -> OturumKaydi {
        OturumKaydi::hedefle(Box::new(Self {
            ekleme_bozuk: true,
            kalan_kalici: 0,
        }))
    }

    /// Satır eklemek başarılı, kalıcılaştırmak (fsync) başarısız: yalnızca yan
    /// etkili araçtan önceki "başladı" kaydı düşer.
    pub fn kalicida_bozuk() -> OturumKaydi {
        Self::kalicidan_sonra_bozuk(0)
    }

    /// İlk n kalıcılaştırma başarılı, sonraki başarısız.
    pub fn kalicidan_sonra_bozuk(n: usize) -> OturumKaydi {
        OturumKaydi::hedefle(Box::new(Self {
            ekleme_bozuk: false,
            kalan_kalici: n,
        }))
    }
}

impl KayitHedefi for BozulanHedef {
    fn ekle(&mut self, _satir: &[u8]) -> io::Result<()> {
        if self.ekleme_bozuk {
            Err(io::Error::other("disk dolu"))
        } else {
            Ok(())
        }
    }

    fn kalici_yap(&mut self) -> io::Result<()> {
        if self.kalan_kalici == 0 {
            Err(io::Error::other("disk dolu"))
        } else {
            self.kalan_kalici -= 1;
            Ok(())
        }
    }
}

pub fn test_oturumu(baglam: OturumBaglami, ayarlar: OturumAyarlari) -> Surucu {
    let (tutamac, olaylar) = baslat(ayarlar, baglam).expect("oturum başlatılamadı");
    Surucu::yeni(tutamac, olaylar)
}
