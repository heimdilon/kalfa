//! `dosya_yaz`: dosya oluşturma ya da tamamen yeniden yazma.

use std::sync::Arc;

use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, Hazirlik, YazmaHedefi};
use kalfa_core::geri_alma::DOSYA_SINIRI;
use kalfa_core::muhur::ozet_hex;
use kalfa_core::protokol::VeriCikisi;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::arguman;
use crate::calisma_alani::CalismaAlani;
use crate::diff::sinirli_diff;

/// Önceki içeriği geri alma için hedefe koyar; `DOSYA_SINIRI`'ni aşan içerik
/// konmaz ve hedef `buyuk` olur (aşama 3 tasarımı §5.3).
pub(crate) fn onceki_icerigi_koy(hedef: &mut YazmaHedefi, onceki: Vec<u8>) {
    hedef.onceki_bayt = onceki.len() as u64;
    if hedef.onceki_bayt > DOSYA_SINIRI {
        hedef.buyuk = true;
    } else {
        hedef.onceki_icerik = Some(onceki.into());
    }
}

pub struct DosyaYaz {
    alan: Arc<CalismaAlani>,
}

impl DosyaYaz {
    pub fn yeni(alan: Arc<CalismaAlani>) -> Self {
        Self { alan }
    }
}

async fn hazirlik_iscisi<T: Send + 'static>(
    is: impl FnOnce(CancellationToken) -> T + Send + 'static,
) -> Result<T, AracHatasi> {
    let iptal = CancellationToken::new();
    // Hazırlık future'ı iptalde bırakılınca özet işçisi de durur.
    let _iptal_koruma = iptal.clone().drop_guard();
    tokio::task::spawn_blocking(move || is(iptal))
        .await
        .map_err(|e| AracHatasi::GirisCikis(e.to_string()))
}

#[async_trait]
impl Arac for DosyaYaz {
    fn ad(&self) -> &str {
        "dosya_yaz"
    }

    fn aciklama(&self) -> &str {
        "Çalışma alanında bir dosyayı oluşturur ya da tamamen yeniden yazar. Var olan bir dosyada küçük bir değişiklik için `dosya_duzenle` kullan."
    }

    fn parametre_semasi(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "yol": {"type": "string", "description": "Çalışma alanına göre göreli yol"},
                "icerik": {"type": "string", "description": "Dosyanın yeni içeriğinin tamamı"}
            },
            "required": ["yol", "icerik"],
            "additionalProperties": false
        })
    }

    fn yazar_mi(&self) -> bool {
        true
    }

    async fn hazirla(&self, a: &Value) -> Result<Hazirlik, AracHatasi> {
        let yol = arguman::yol(a, "yol")?;
        let icerik = arguman::metin(a, "icerik")?;
        let alan = self.alan.clone();
        let okuma_yolu = yol.clone();
        let onceki =
            hazirlik_iscisi(move |iptal| alan.onceki_dosya(&okuma_yolu, true, Some(&iptal)))
                .await??;
        let baglanti_sayisi = onceki.as_ref().map_or(1, |d| d.baglanti_sayisi);
        let gorunmez_icerik = kalfa_core::gorunum::gorunmez_var_mi(icerik)
            || onceki.as_ref().is_some_and(|d| d.gorunmez_icerik);
        let satir = icerik.lines().count();
        let mut aciklama = if onceki.is_some() {
            format!("`{yol}` dosyasını yeniden yazmak istiyor ({satir} satır).")
        } else {
            format!("`{yol}` dosyasını oluşturmak istiyor ({satir} satır).")
        };
        let olusacak_dizinler = if onceki.is_none() {
            self.alan.olusacak_dizinler(&yol)?
        } else {
            Vec::new()
        };
        let diff = if onceki.as_ref().is_some_and(|d| d.icerik.is_none()) {
            None
        } else {
            let eski = onceki
                .as_ref()
                .and_then(|d| d.icerik.as_deref())
                .map(String::from_utf8_lossy)
                .unwrap_or_default();
            sinirli_diff(yol.as_str(), &eski, icerik)
        };
        if diff.is_none() {
            aciklama.push_str(" Diff, boyut sınırı nedeniyle gösterilemiyor.");
        }
        let mut hedef = YazmaHedefi {
            olusacak_dizinler,
            sonraki_ozet: Some(ozet_hex(icerik.as_bytes())),
            ..YazmaHedefi::yeni(
                yol,
                onceki.as_ref().map(|d| d.ozet.clone()),
                baglanti_sayisi,
            )
        };
        if let Some(onceki) = onceki {
            hedef.onceki_bayt = onceki.bayt;
            hedef.buyuk = onceki.bayt > DOSYA_SINIRI;
            hedef.onceki_icerik = onceki.icerik.map(Into::into);
        }
        Ok(Hazirlik {
            aciklama,
            diff,
            gorunmez_icerik,
            yazilan: vec![hedef],
            veri_cikisi: Some(VeriCikisi::YalnizcaYazildi),
            ..Default::default()
        })
    }

    async fn calistir(
        &self,
        a: &Value,
        h: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        let iptal = iptal.child_token();
        let _iptal_koruma = iptal.clone().drop_guard();
        if iptal.is_cancelled() {
            return Err(AracHatasi::Iptal);
        }
        let yol = arguman::yol(a, "yol")?;
        let icerik = arguman::metin(a, "icerik")?;
        let hedef = h
            .yazilan
            .first()
            .ok_or_else(|| AracHatasi::Gecersiz("Yazma hazırlığında hedef yok.".into()))?;
        let alan = self.alan.clone();
        let yazma_yolu = yol.clone();
        let veri = icerik.to_string();
        let hedef = hedef.clone();
        let sonuc = tokio::task::spawn_blocking(move || {
            let onceki = alan
                .onceki_dosya(&yazma_yolu, false, Some(&iptal))
                .map_err(|e| {
                    if iptal.is_cancelled() {
                        AracHatasi::Iptal
                    } else {
                        e.into()
                    }
                })?;
            if hedef.yol != yazma_yolu
                || onceki.as_ref().map(|d| d.ozet.as_str()) != hedef.onceki_ozet.as_deref()
                || onceki.as_ref().map_or(1, |d| d.baglanti_sayisi) != hedef.baglanti_sayisi
            {
                return Err(AracHatasi::OnayEskidi);
            }
            if iptal.is_cancelled() {
                return Err(AracHatasi::Iptal);
            }
            alan.yaz(&yazma_yolu, veri.as_bytes())
                .map_err(AracHatasi::from)
        })
        .await
        .map_err(|e| AracHatasi::GirisCikis(e.to_string()))?;
        self.alan.yazma_bitisini_bekle().await?;
        sonuc?;
        Ok(AracCiktisi {
            icerik: format!("`{yol}` yazıldı ({} bayt).", icerik.len()),
            kaynak: format!("dosya:{yol}"),
            guvenlik_uyarilari: Vec::new(),
            degisiklikler: None,
            kalfa_uretimi: true,
            ag: None,
        })
    }
}

#[cfg(test)]
mod iptal_testleri {
    use super::*;

    #[tokio::test]
    async fn birakilan_hazirligin_iscisi_iptal_edilir() {
        let (basladi_tx, basladi_rx) = tokio::sync::oneshot::channel();
        let (durdu_tx, durdu_rx) = tokio::sync::oneshot::channel();
        let gorev = tokio::spawn(hazirlik_iscisi(move |iptal| {
            basladi_tx.send(()).unwrap();
            tokio::runtime::Handle::current().block_on(iptal.cancelled());
            durdu_tx.send(()).unwrap();
        }));
        basladi_rx.await.unwrap();
        gorev.abort();
        assert!(gorev.await.unwrap_err().is_cancelled());
        tokio::time::timeout(std::time::Duration::from_secs(1), durdu_rx)
            .await
            .unwrap()
            .unwrap();
    }
}
