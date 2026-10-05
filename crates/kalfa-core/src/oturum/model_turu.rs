//! Model turu: istek, akış tüketimi ve iptal edilebilir bekleme.

use std::future::Future;
use std::time::Duration;

use futures_util::StreamExt;
use kalfa_evren::butce::YanitButcesi;
use kalfa_evren::tipler::{ChatMessage, ChatRequest, ToolCall};
use kalfa_evren::{AkisOlayi, YanitMeta};

use super::{Etki, Oturum};
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
                self.mesaj_ekle(mesaj).await;
                Ok(ModelCevabi { cagrilar })
            }
            Err(neden) => {
                // Yarım kalan yanıtın metni bağlamda kalır; araç çağrıları çalıştırılmaz.
                if !metin.is_empty() {
                    self.mesaj_ekle(ChatMessage::assistant(Some(metin), Vec::new()))
                        .await;
                }
                Err(neden)
            }
        }
    }

    pub(super) async fn akis_ac(
        &mut self,
        istek: ChatRequest,
    ) -> Result<SaglayiciAkisi, BitisNedeni> {
        let mut hiz_denendi = false;
        loop {
            let saglayici = self.baglam.saglayici.clone();
            let deneme = istek.clone();
            let kapi = self.baglam.sart_kapisi.clone();
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
                    self.istek_kaydet("ana", &akis.meta).await;
                    return Ok(akis);
                }
                (istek_nesli, Err(kalfa_evren::EvrenError::SartKabulGerekli { .. })) => {
                    self.sart_akisi(istek_nesli).await?;
                }
                (_, Err(kalfa_evren::EvrenError::HizSiniri { tekrar_dene })) if !hiz_denendi => {
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
                (_, Err(e)) => {
                    self.hata(e.to_string()).await;
                    return Err(BitisNedeni::Hata);
                }
            }
        }
    }

    pub(super) async fn istek_kaydet(&mut self, amac: &str, meta: &YanitMeta) {
        let oge = KayitOgesi::Istek {
            amac: amac.to_string(),
            istek_id: meta.istek_id.clone(),
            kredi: meta.kredi.clone(),
            yonlendirilen_model: meta.yonlendirilen_model.clone(),
        };
        self.kaydet(oge).await;
        self.tur_ozeti.istek();
        self.yay(Olay::IstekBilgisi {
            istek_id: meta.istek_id.clone(),
            yonlendirilen_model: meta.yonlendirilen_model.clone(),
        })
        .await;
        if let Some(gozlem) = meta.kredi.clone() {
            self.yay(Olay::KrediGozlemi { gozlem }).await;
        }
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
