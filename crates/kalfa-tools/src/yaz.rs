//! `dosya_yaz`: dosya oluşturma ya da tamamen yeniden yazma.

use std::sync::Arc;

use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, Hazirlik, YazmaHedefi};
use kalfa_core::muhur::ozet_hex;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::arguman;
use crate::calisma_alani::CalismaAlani;
use crate::diff::birlesik_diff;

pub struct DosyaYaz {
    alan: Arc<CalismaAlani>,
}

impl DosyaYaz {
    pub fn yeni(alan: Arc<CalismaAlani>) -> Self {
        Self { alan }
    }
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
        let onceki = self.alan.oku_varsa(&yol)?;
        let baglanti_sayisi = self.alan.baglanti_sayisi(&yol)?;
        let eski = onceki
            .as_deref()
            .map(String::from_utf8_lossy)
            .unwrap_or_default();
        let satir = icerik.lines().count();
        let aciklama = if onceki.is_some() {
            format!("`{yol}` dosyasını yeniden yazmak istiyor ({satir} satır).")
        } else {
            format!("`{yol}` dosyasını oluşturmak istiyor ({satir} satır).")
        };
        Ok(Hazirlik {
            aciklama,
            diff: Some(birlesik_diff(yol.as_str(), &eski, icerik)),
            yazilan: vec![YazmaHedefi {
                yol,
                onceki_ozet: onceki.as_deref().map(ozet_hex),
                baglanti_sayisi,
            }],
            ..Default::default()
        })
    }

    async fn calistir(
        &self,
        a: &Value,
        h: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        if iptal.is_cancelled() {
            return Err(AracHatasi::Iptal);
        }
        let yol = arguman::yol(a, "yol")?;
        let icerik = arguman::metin(a, "icerik")?;
        let hedef = h
            .yazilan
            .first()
            .ok_or_else(|| AracHatasi::Gecersiz("Yazma hazırlığında hedef yok.".into()))?;
        if hedef.yol != yol
            || self.alan.icerik_ozeti(&yol)?.as_deref() != hedef.onceki_ozet.as_deref()
            || self.alan.baglanti_sayisi(&yol)? != hedef.baglanti_sayisi
        {
            return Err(AracHatasi::OnayEskidi);
        }
        self.alan.yaz(&yol, icerik.as_bytes())?;
        Ok(AracCiktisi {
            icerik: format!("`{yol}` yazıldı ({} bayt).", icerik.len()),
            kaynak: format!("dosya:{yol}"),
            guvenlik_uyarilari: Vec::new(),
        })
    }
}
