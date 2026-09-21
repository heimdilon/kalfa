//! Model turu: istek, akış tüketimi ve iptal edilebilir bekleme.

use std::future::Future;

use futures_util::StreamExt;
use kalfa_evren::tipler::{ChatMessage, ChatRequest, ToolCall};
use kalfa_evren::{AkisOlayi, YanitMeta};

use super::{Etki, Oturum};
use crate::kayit::KayitOgesi;
use crate::protokol::{BitisNedeni, Olay};
use crate::saglayici::SaglayiciAkisi;

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
                olay = akis.next() => match olay {
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
                        let olay = Olay::TokenKullanimi {
                            istem: u.prompt_tokens,
                            tamamlama: u.completion_tokens,
                            dusunce: u.reasoning_tokens(),
                        };
                        self.yay(olay).await;
                    }
                    Some(Ok(AkisOlayi::Bitti { .. })) | None => break Ok(()),
                    Some(Err(e)) => {
                        self.hata(e.to_string()).await;
                        break Err(BitisNedeni::Hata);
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
        let saglayici = self.baglam.saglayici.clone();
        match self
            .iptal_edilebilir(async move { saglayici.akis(istek).await })
            .await?
        {
            Ok(akis) => {
                self.istek_kaydet("ana", &akis.meta).await;
                Ok(akis)
            }
            Err(e) => {
                self.hata(e.to_string()).await;
                Err(BitisNedeni::Hata)
            }
        }
    }

    pub(super) async fn istek_kaydet(&mut self, amac: &str, meta: &YanitMeta) {
        let oge = KayitOgesi::Istek {
            amac: amac.to_string(),
            istek_id: meta.istek_id.clone(),
            kredi: meta.kredi.clone(),
        };
        self.kaydet(oge).await;
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
}
