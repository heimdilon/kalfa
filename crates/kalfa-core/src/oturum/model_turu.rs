//! Model turu: istek, akış tüketimi ve iptal edilebilir bekleme.

use std::future::Future;
use std::time::Duration;

use futures_util::StreamExt;
use kalfa_evren::butce::YanitButcesi;
use kalfa_evren::tipler::{ChatMessage, ChatRequest, ToolCall};
use kalfa_evren::{AkisOlayi, YanitMeta};

use super::{Etki, Oturum};
use crate::gonderim::{GonderilenOge, IstekAsamasi, IstekSonucu};
use crate::kayit::KayitOgesi;
use crate::protokol::{BitisNedeni, Olay};
use crate::saglayici::SaglayiciAkisi;
use crate::sart_kapisi::SartKarari;

pub(super) struct ModelCevabi {
    pub(super) cagrilar: Vec<ToolCall>,
}

impl Oturum {
    fn istek_olustur(&self) -> ChatRequest {
        let mut mesajlar = vec![ChatMessage::system(self.ayarlar.sistem_mesaji.clone())];
        mesajlar.extend(self.mesajlar.iter().cloned());
        let mut istek = ChatRequest::new(self.ayarlar.model.clone(), mesajlar);
        istek.tools = self.baglam.araclar.tanimlar(self.ayarlar.kip);
        istek
    }

    pub(super) async fn model_turu(&mut self) -> Result<ModelCevabi, BitisNedeni> {
        self.baglami_yonet(false).await?;
        let istek = self.istek_olustur();
        let SaglayiciAkisi {
            olaylar: mut akis, ..
        } = self.akis_ac(istek).await?;
        let mut metin = String::new();
        let mut dusunce = String::new();
        let mut cagrilar = Vec::new();
        let mut butce = YanitButcesi::default();

        let sonuc = loop {
            tokio::select! {
                biased;
                // `OturumTutamaci::iptal_et` belirteci doğrudan tetikler; komut kuyruğunu beklemez.
                _ = self.iptal.cancelled() => break Err(BitisNedeni::Iptal),
                komut = self.komutlar.recv() => {
                    let Some(komut) = self.komut_geldi(komut).await else { break Err(BitisNedeni::Iptal) };
                    if self.genel_komut(komut).await == Etki::Iptal {
                        break Err(BitisNedeni::Iptal);
                    }
                }
                olay = akis.next() => {
                    if let Some(Ok(ref o)) = olay && let Err(e) = butce.olay_ekle(o) {
                        self.kaydet(KayitOgesi::IstekKoptu {
                            mesaj: e.to_string(),
                        })
                        .await;
                        self.hata(e.to_string()).await;
                        break Err(BitisNedeni::Hata);
                    }
                    match olay {
                    Some(Ok(AkisOlayi::Metin(s))) => {
                        metin.push_str(&s);
                        self.yay(Olay::TokenParcasi { metin: s }).await;
                    }
                    Some(Ok(AkisOlayi::Dusunce(s))) => {
                        dusunce.push_str(&s);
                        self.yay(Olay::DusunceParcasi { metin: s }).await;
                    }
                    Some(Ok(AkisOlayi::AracCagrilari(v))) => cagrilar.extend(v),
                    Some(Ok(AkisOlayi::Kullanim(u))) => {
                        let (istem, tamamlama, dusunce) =
                            (u.prompt_tokens, u.completion_tokens, u.reasoning_tokens());
                        self.oran = super::baglam::Oran::yeni(istem, self.son_istek_bayti);
                        self.kaydet(KayitOgesi::Kullanim {
                            istem,
                            tamamlama,
                            dusunce,
                        })
                        .await;
                        self.tur_ozeti.kullanim(istem, tamamlama, dusunce);
                        self.yay(Olay::TokenKullanimi {
                            istem,
                            tamamlama,
                            dusunce,
                        })
                        .await;
                    }
                    Some(Ok(AkisOlayi::Bitti { .. })) | None => break Ok(()),
                    Some(Err(e)) => {
                        self.kaydet(KayitOgesi::IstekKoptu {
                            mesaj: e.to_string(),
                        })
                        .await;
                        self.hata(e.to_string()).await;
                        break Err(BitisNedeni::Hata);
                    }
                    }
                },
            }
        };
        // İptalde akış bırakılır; bağlantı kapanır.
        drop(akis);

        match sonuc {
            Ok(()) => {
                let mut mesaj =
                    ChatMessage::assistant((!metin.is_empty()).then_some(metin), cagrilar.clone());
                mesaj.reasoning = (!dusunce.is_empty()).then_some(dusunce);
                self.mesaj_ekle(mesaj, Some("model".to_string())).await;
                Ok(ModelCevabi { cagrilar })
            }
            Err(neden) => {
                // Yarım kalan yanıtın metni bağlamda kalır; araç çağrıları çalıştırılmaz.
                if !metin.is_empty() {
                    self.mesaj_ekle(
                        ChatMessage::assistant(Some(metin), Vec::new()),
                        Some("model".to_string()),
                    )
                    .await;
                }
                Err(neden)
            }
        }
    }

    pub(super) async fn akis_ac(
        &mut self,
        mut istek: ChatRequest,
    ) -> Result<SaglayiciAkisi, BitisNedeni> {
        let mut hiz_denendi = false;
        let mut asim_denendi = false;
        loop {
            let saglayici = self.baglam.saglayici.clone();
            let deneme = istek.clone();
            let kapi = self.baglam.sart_kapisi.clone();
            let onceki = self.gonderilen as u32;
            let onceki_sistem = self.son_sistem_istemi.clone();
            let onceki_araclar = self.son_arac_tanimlari.clone();
            self.son_istek_bayti = self.baglam_bayti();
            let yeni = self.gonderim_oncesi_hazirla().await;
            match self
                .iptal_edilebilir(async move {
                    // Nesli future ilk kez gerçekten çalışmaya başladığında
                    // yakala; önceden snapshot alıp komut beklerken eskimesin.
                    let istek_nesli = kapi.durum().nesil;
                    (istek_nesli, saglayici.akis(deneme).await)
                })
                .await?
            {
                (_, Ok(akis)) => {
                    self.istek_kaydet(
                        "ana",
                        &akis.meta,
                        Some(istek.model.clone()),
                        yeni,
                        onceki,
                        IstekSonucu::Basarili,
                    )
                    .await;
                    self.gonderilen = self.mesajlar.len();
                    self.bekleyen_ozetler.clear();
                    return Ok(akis);
                }
                (istek_nesli, Err(e)) => {
                    let asama = match &e {
                        kalfa_evren::EvrenError::Ag {
                            govde_gitti: false, ..
                        } => IstekAsamasi::Baglanti,
                        _ => IstekAsamasi::Yanit,
                    };
                    self.istek_kaydet_hata(
                        "ana",
                        Some(istek.model.clone()),
                        yeni,
                        onceki,
                        asama,
                        e.to_string(),
                    )
                    .await;
                    if asama != IstekAsamasi::Baglanti {
                        self.gonderilen = self.mesajlar.len();
                        self.bekleyen_ozetler.clear();
                    } else {
                        self.son_sistem_istemi = onceki_sistem;
                        self.son_arac_tanimlari = onceki_araclar;
                    }

                    match e {
                        kalfa_evren::EvrenError::SartKabulGerekli { .. } => {
                            self.sart_akisi(istek_nesli).await?;
                        }
                        kalfa_evren::EvrenError::HizSiniri { tekrar_dene } if !hiz_denendi => {
                            hiz_denendi = true;
                            let sure = tekrar_dene
                                .unwrap_or(Duration::from_secs(5))
                                .min(self.ayarlar.sinirlar.azami_bekleme);
                            self.hata(format!(
                                "EVREN istek sınırına ulaşıldı; {} sn sonra bir kez yeniden denenecek.",
                                sure.as_secs()
                            ))
                            .await;
                            self.iptal_edilebilir(tokio::time::sleep(sure)).await?;
                        }
                        hata @ kalfa_evren::EvrenError::BaglamAsimi { .. } => {
                            // 5a tasarımı §11: bir kez sıkı özetleme, sonra yeniden.
                            if asim_denendi || !self.baglami_yonet(true).await? {
                                self.hata(hata.to_string()).await;
                                return Err(BitisNedeni::Hata);
                            }
                            asim_denendi = true;
                            istek = self.istek_olustur();
                        }
                        _ => {
                            self.hata(e.to_string()).await;
                            return Err(BitisNedeni::Hata);
                        }
                    }
                }
            }
        }
    }

    pub(super) async fn istek_kaydet(
        &mut self,
        amac: &str,
        meta: &YanitMeta,
        model: Option<String>,
        yeni: Vec<GonderilenOge>,
        onceki: u32,
        sonuc: IstekSonucu,
    ) {
        let oge = KayitOgesi::Istek {
            amac: amac.to_string(),
            istek_id: meta.istek_id.clone(),
            kredi: meta.kredi.clone(),
            yonlendirilen_model: meta.yonlendirilen_model.clone(),
            model,
            yeni,
            onceki,
            sonuc,
            butce_islem: None,
        };
        self.kaydet(oge).await;
        self.tur_ozeti.istek();
        self.yay(Olay::IstekBilgisi {
            istek_id: meta.istek_id.clone(),
            yonlendirilen_model: meta.yonlendirilen_model.clone(),
            amac: amac.to_string(),
            basarili: true,
        })
        .await;
        if let Some(gozlem) = meta.kredi.clone() {
            self.yay(Olay::KrediGozlemi { gozlem }).await;
        }
    }

    pub(super) async fn istek_kaydet_hata(
        &mut self,
        amac: &str,
        model: Option<String>,
        yeni: Vec<GonderilenOge>,
        onceki: u32,
        asama: IstekAsamasi,
        mesaj: String,
    ) {
        let oge = KayitOgesi::Istek {
            amac: amac.to_string(),
            istek_id: None,
            kredi: None,
            yonlendirilen_model: None,
            model,
            yeni,
            onceki,
            sonuc: IstekSonucu::Hata { asama, mesaj },
            butce_islem: None,
        };
        self.kaydet(oge).await;
        self.tur_ozeti.istek();
        self.yay(Olay::IstekBilgisi {
            istek_id: None,
            yonlendirilen_model: None,
            amac: amac.to_string(),
            basarili: false,
        })
        .await;
    }

    pub(super) async fn gonderim_oncesi_hazirla(&mut self) -> Vec<GonderilenOge> {
        let mut ogeler = Vec::new();

        // 1. Sistem istemi
        if self.son_sistem_istemi.as_ref().map(|(_, m)| m.as_str())
            != Some(&self.ayarlar.sistem_mesaji)
        {
            let metin = self.ayarlar.sistem_mesaji.clone();
            let bayt = metin.len() as u64;
            self.kaydet(KayitOgesi::SistemIstemi {
                metin: metin.clone(),
            })
            .await;
            let sira = if self.kayit.is_some() {
                Some(self.sira)
            } else {
                None
            };
            self.son_sistem_istemi = Some((sira, metin));
            ogeler.push(GonderilenOge {
                kaynak: "kalfa:sistem".to_string(),
                sira,
                bayt,
            });
        }

        // 2. Araç tanımları
        let mevcut_tanimlar = self.baglam.araclar.tanimlar(self.ayarlar.kip);
        if self.son_arac_tanimlari.as_ref().map(|(_, t)| t) != Some(&mevcut_tanimlar) {
            let bayt = serde_json::to_vec(&mevcut_tanimlar).map_or(0, |v| v.len() as u64);
            self.kaydet(KayitOgesi::AracTanimlari {
                tanimlar: mevcut_tanimlar.clone(),
            })
            .await;
            let sira = if self.kayit.is_some() {
                Some(self.sira)
            } else {
                None
            };
            self.son_arac_tanimlari = Some((sira, mevcut_tanimlar));
            ogeler.push(GonderilenOge {
                kaynak: "kalfa:araçlar".to_string(),
                sira,
                bayt,
            });
        }

        // Gönderilmemiş özetler (5a tasarımı §9): daha önce gönderilmiş
        // mesajların yerine geçen içerik.
        for &i in &self.bekleyen_ozetler {
            if let (Some(m), Some((sira, kaynak))) =
                (self.mesajlar.get(i), self.mesaj_bilgileri.get(i))
            {
                ogeler.push(GonderilenOge {
                    kaynak: kaynak.clone().unwrap_or_default(),
                    sira: *sira,
                    bayt: super::baglam::mesaj_bayti(m),
                });
            }
        }

        // 3. Mesajlar
        for i in self.gonderilen..self.mesajlar.len() {
            let mesaj = &self.mesajlar[i];
            let info = self.mesaj_bilgileri.get(i);
            let sira = info.and_then(|(s, _)| *s);
            let kaynak = info
                .and_then(|(_, k)| k.clone())
                .unwrap_or_else(|| match mesaj.role {
                    kalfa_evren::tipler::Role::User => "mesaj:kullanıcı".to_string(),
                    kalfa_evren::tipler::Role::Assistant => "model".to_string(),
                    kalfa_evren::tipler::Role::Tool => "kalfa:sonuç".to_string(),
                    kalfa_evren::tipler::Role::System => "kalfa:sistem".to_string(),
                });
            let bayt = super::baglam::mesaj_bayti(mesaj);
            ogeler.push(GonderilenOge { kaynak, sira, bayt });
        }
        ogeler
    }

    /// Bir geleceği beklerken komutları ve iptal belirtecini de dinler; iptal
    /// gelirse `Err(Iptal)`. Gelecek `self`'i ödünç almamalıdır.
    pub(super) async fn iptal_edilebilir<F: Future>(
        &mut self,
        gelecek: F,
    ) -> Result<F::Output, BitisNedeni> {
        tokio::pin!(gelecek);
        loop {
            tokio::select! {
                biased;
                _ = self.iptal.cancelled() => return Err(BitisNedeni::Iptal),
                komut = self.komutlar.recv() => {
                    let Some(komut) = self.komut_geldi(komut).await else { return Err(BitisNedeni::Iptal) };
                    if self.genel_komut(komut).await == Etki::Iptal {
                        return Err(BitisNedeni::Iptal);
                    }
                }
                cikti = &mut gelecek => return Ok(cikti),
            }
        }
    }

    /// Şart kapısı (1c tasarımı §7.3, üst belge §11.8). Oturum kabul isteğini
    /// kendisi göndermez: uygulama genelindeki kapıya kaydolur, metni yayar ve
    /// kullanıcının şart penceresindeki kararını bekler.
    async fn sart_akisi(&mut self, istek_nesli: u64) -> Result<(), BitisNedeni> {
        let kapi = self.baglam.sart_kapisi.clone();
        let gerekli = match self
            .iptal_edilebilir(async move { kapi.gerekli(istek_nesli).await })
            .await?
        {
            Ok(v) => v,
            Err(e) => {
                self.hata(e.to_string()).await;
                return Err(BitisNedeni::Hata);
            }
        };
        let (metin, nesil) = match gerekli {
            crate::sart_kapisi::GerekliSonucu::OnayGerekli { metin, nesil } => (metin, nesil),
            crate::sart_kapisi::GerekliSonucu::KabulEdildi { surum } => {
                self.kaydet(KayitOgesi::SartKabulEdildi { surum }).await;
                return Ok(());
            }
            crate::sart_kapisi::GerekliSonucu::Ret => return Err(BitisNedeni::Iptal),
        };
        self.yay(Olay::SartOnayiGerekiyor {
            surum: metin.surum,
            metin: metin.metin,
        })
        .await;
        let kapi = self.baglam.sart_kapisi.clone();
        match self
            .iptal_edilebilir(async move { kapi.karar_bekle(nesil).await })
            .await?
        {
            SartKarari::Kabul { surum } => {
                self.kaydet(KayitOgesi::SartKabulEdildi { surum }).await;
                Ok(())
            }
            SartKarari::Ret => Err(BitisNedeni::Iptal),
        }
    }
}
