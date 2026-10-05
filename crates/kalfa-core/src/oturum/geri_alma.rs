//! Turu geri alma: önizleme, mühürlü yürütme ve depo sınırı (aşama 3
//! tasarımı §5.2, §5.6, §5.7, §5.9).

use kalfa_evren::tipler::ChatMessage;

use super::Oturum;
use crate::geri_alma::{
    GeriAlmaEylemi, GeriAlmaPlani, GeriAlmaSonucu, kalfa_notu, plan_cikar, plan_muhru,
};
use crate::kayit::KayitOgesi;
use crate::kimlik::TurId;
use crate::muhur::OnayMuhru;
use crate::protokol::Olay;
use crate::yol::CalismaYolu;

pub(super) const TUR_SURUYOR: &str = "Önceki tur sürüyor; bitmesini bekleyin ya da iptal edin.";
const GERI_ALMA_YOK: &str = "Bu oturumda geri alma yok: kayıtsız oturumda önceki hâller saklanmaz.";
const TUR_YOK: &str = "Bu tur bulunamadı.";
const YAPILACAK_YOK: &str = "Geri alınacak bir değişiklik yok.";
const PLAN_ESKIDI: &str =
    "Dosyalar önizlemeden sonra değişti; hiçbir şey geri alınmadı. Yeni önizlemeye bakın.";

impl Oturum {
    /// Turun planını şimdiki dosya durumlarından çıkarır.
    fn geri_alma_plani(&self, tur: TurId) -> Result<GeriAlmaPlani, &'static str> {
        let (Some(depo), Some(yukleyici)) = (&self.depo, &self.baglam.geri_yukleyici) else {
            return Err(GERI_ALMA_YOK);
        };
        plan_cikar(
            tur,
            &self.geri_alma,
            depo,
            yukleyici.as_ref(),
            &self.baglam.siniflandirici,
        )
        .ok_or(TUR_YOK)
    }

    pub(super) async fn geri_alma_onizle(&mut self, tur: TurId) {
        match self.geri_alma_plani(tur) {
            Ok(plan) => {
                let muhur = plan_muhru(tur, &plan);
                self.yay(Olay::GeriAlmaPlani {
                    geri_alinan_tur: tur,
                    plan,
                    muhur,
                })
                .await;
            }
            Err(mesaj) => self.hata(mesaj).await,
        }
    }

    /// Önizlenen planı yürütür. Plan kilit altında yeniden çıkarılır; mühür
    /// tutmazsa hiçbir şey yazılmaz ve yeni plan gösterilir.
    pub(super) async fn turu_geri_al(&mut self, tur: TurId, muhur: OnayMuhru) {
        let ilk = match self.geri_alma_plani(tur) {
            Ok(plan) => plan,
            Err(mesaj) => return self.hata(mesaj).await,
        };
        let yollar: Vec<CalismaYolu> = ilk.ogeler.iter().map(|o| o.yol.clone()).collect();
        let kilitler = self.baglam.kilitler.clone();
        let _kilit = kilitler.al(&[], &yollar, false).await;
        let plan = match self.geri_alma_plani(tur) {
            Ok(plan) => plan,
            Err(mesaj) => return self.hata(mesaj).await,
        };
        let taze = plan_muhru(tur, &plan);
        if taze != muhur {
            self.yay(Olay::GeriAlmaPlani {
                geri_alinan_tur: tur,
                plan,
                muhur: taze,
            })
            .await;
            return self.hata(PLAN_ESKIDI).await;
        }
        if !plan.yapilacak_var_mi() {
            return self.hata(YAPILACAK_YOK).await;
        }
        // Yazmadan önce kalıcı: yarım kalan geri alma sonraki açılışta görünür.
        let basladi = KayitOgesi::GeriAlmaBasladi {
            geri_alinan_tur: tur,
            muhur,
        };
        if !self.kaydet_kalici(basladi).await {
            return;
        }
        let sonuc = self.plani_yurut(tur, &plan);
        let (baslik, kabuk_var) = self
            .geri_alma
            .tur(tur)
            .map(|t| (t.baslik.clone(), !t.komutlar.is_empty()))
            .unwrap_or_default();
        let metin = kalfa_notu(&baslik, &sonuc, kabuk_var);
        self.kaydet(KayitOgesi::GeriAlindi {
            geri_alinan_tur: tur,
            sonuc: sonuc.clone(),
        })
        .await;
        let sira = if self.kayit.is_some() {
            Some(self.sira + 1)
        } else {
            None
        };
        self.kaydet(KayitOgesi::KalfaNotu {
            metin: metin.clone(),
        })
        .await;
        self.mesajlar.push(ChatMessage::user(metin));
        self.mesaj_bilgileri
            .push((sira, Some("kalfa:not".to_string())));
        self.yay(Olay::TurGeriAlindi {
            geri_alinan_tur: tur,
            sonuc,
        })
        .await;
    }

    /// Plandaki yazmaları yapar; çakışan ve geri alınamayan öğelere dokunmaz.
    /// Her yazmadan hemen önce dosyanın şimdiki özeti yine denetlenir.
    fn plani_yurut(&self, tur: TurId, plan: &GeriAlmaPlani) -> Vec<GeriAlmaSonucu> {
        let (Some(depo), Some(yukleyici)) = (&self.depo, &self.baglam.geri_yukleyici) else {
            return Vec::new();
        };
        let mut sonuc = Vec::with_capacity(plan.ogeler.len());
        for o in &plan.ogeler {
            let beklenen = o.simdiki.as_deref().unwrap_or_default();
            let hata = match &o.eylem {
                GeriAlmaEylemi::GeriYukle { ozet } => depo
                    .oku(ozet)
                    .and_then(|icerik| yukleyici.geri_yaz(&o.yol, Some(&icerik), beklenen))
                    .err(),
                GeriAlmaEylemi::Sil => yukleyici.geri_yaz(&o.yol, None, beklenen).err(),
                GeriAlmaEylemi::ZatenOnceki
                | GeriAlmaEylemi::Cakisma { .. }
                | GeriAlmaEylemi::GeriAlinamaz { .. } => None,
            };
            sonuc.push(GeriAlmaSonucu {
                yol: o.yol.clone(),
                eylem: o.eylem.clone(),
                hata: hata.map(|e| e.to_string()),
            });
        }
        // Turun oluşturduğu dizinler boşsa kaldırılır, doluysa kalır; derinden
        // sığa.
        let mut dizinler: Vec<CalismaYolu> = self
            .geri_alma
            .tur(tur)
            .into_iter()
            .flat_map(|t| &t.cagrilar)
            .flat_map(|c| &c.yedekler)
            .flat_map(|y| y.olusan_dizinler.iter().cloned())
            .collect();
        dizinler.sort_by(|a, b| {
            let derinlik = |d: &CalismaYolu| d.as_str().matches('/').count();
            derinlik(b).cmp(&derinlik(a)).then_with(|| a.cmp(b))
        });
        dizinler.dedup();
        for dizin in dizinler {
            // Kaldırılamayan dizin geri almayı bozmaz.
            let _ = yukleyici.bos_dizini_kaldir(&dizin);
        }
        sonuc
    }

    /// Tur sonunda depo `geri_alma_oturum`'u aşmışsa en eski turların verisi
    /// `geri_alma_oturum_alt`'a inene kadar silinir. Başka bir turun da
    /// kullandığı içerik kalır (§5.2).
    pub(super) async fn geri_alma_sinirini_uygula(&mut self) {
        let Some(depo) = &self.depo else {
            return;
        };
        let sinir = self.ayarlar.sinirlar.geri_alma_oturum;
        let alt = self.ayarlar.sinirlar.geri_alma_oturum_alt;
        let Ok(mut boyut) = depo.boyut() else {
            return;
        };
        if boyut <= sinir {
            return;
        }
        let adaylar: Vec<TurId> = self
            .geri_alma
            .turlar()
            .iter()
            .filter(|t| !t.veri_silindi)
            .map(|t| t.tur)
            .collect();
        let mut silinen = Vec::new();
        for tur in adaylar {
            if boyut <= alt {
                break;
            }
            let ozetler = self.geri_alma.yalniz_kullanilan(tur);
            self.geri_alma.veri_silindi(&[tur]);
            silinen.push(tur);
            for ozet in ozetler {
                // Silinemeyen dosya bir sonraki açılışın temizliğine kalır.
                let _ = depo.sil(&ozet);
            }
            match depo.boyut() {
                Ok(b) => boyut = b,
                Err(_) => break,
            }
        }
        if silinen.is_empty() {
            return;
        }
        self.kaydet(KayitOgesi::GeriAlmaVerisiSilindi {
            turlar: silinen.clone(),
        })
        .await;
        self.yay(Olay::GeriAlmaVerisiSilindi { turlar: silinen })
            .await;
    }
}
