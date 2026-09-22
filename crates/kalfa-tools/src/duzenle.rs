//! `dosya_duzenle`: bir metni birebir bulup değiştirme.

use std::sync::Arc;

use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, Hazirlik, YazmaHedefi};
use kalfa_core::muhur::ozet_hex;
use kalfa_core::yol::CalismaYolu;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::arguman;
use crate::calisma_alani::{CalismaAlani, ikili_mi};
use crate::diff::birlesik_diff;

pub struct DosyaDuzenle {
    alan: Arc<CalismaAlani>,
}

impl DosyaDuzenle {
    pub fn yeni(alan: Arc<CalismaAlani>) -> Self {
        Self { alan }
    }

    /// Dosyayı ham bayt ve UTF-8 metin olarak okur.
    fn metin_oku(&self, yol: &CalismaYolu) -> Result<(Vec<u8>, String), AracHatasi> {
        let veri = self.alan.oku_varsa(yol)?.ok_or_else(|| {
            AracHatasi::Gecersiz(format!("`{yol}` yok; yeni dosya için `dosya_yaz` kullan."))
        })?;
        if ikili_mi(&veri) {
            return Err(AracHatasi::Gecersiz(format!(
                "`{yol}` ikili (binary) bir dosya; düzenlenmez."
            )));
        }
        let metin = String::from_utf8(veri.clone()).map_err(|_| {
            AracHatasi::Gecersiz(format!(
                "`{yol}` UTF-8 değil; `dosya_duzenle` yalnızca metin dosyalarında çalışır."
            ))
        })?;
        Ok((veri, metin))
    }
}

struct Duzenleme {
    yeni: String,
    adet: usize,
}

fn uygula(icerik: &str, a: &Value) -> Result<Duzenleme, AracHatasi> {
    let eski = arguman::metin(a, "eski_metin")?;
    let yeni = arguman::metin(a, "yeni_metin")?;
    let tumu = arguman::mantiksal(a, "tumu");
    if eski.is_empty() {
        return Err(AracHatasi::Gecersiz("`eski_metin` boş olamaz.".into()));
    }
    match icerik.matches(eski).count() {
        0 => Err(AracHatasi::Gecersiz(
            "`eski_metin` dosyada bulunamadı; dosyayı yeniden okuyup metni birebir ver.".into(),
        )),
        n if n > 1 && !tumu => Err(AracHatasi::Gecersiz(format!(
            "`eski_metin` dosyada {n} kez geçiyor; daha fazla bağlam ekle ya da `tumu: true` kullan."
        ))),
        n if tumu => Ok(Duzenleme {
            yeni: icerik.replace(eski, yeni),
            adet: n,
        }),
        _ => Ok(Duzenleme {
            yeni: icerik.replacen(eski, yeni, 1),
            adet: 1,
        }),
    }
}

#[async_trait]
impl Arac for DosyaDuzenle {
    fn ad(&self) -> &str {
        "dosya_duzenle"
    }

    fn aciklama(&self) -> &str {
        "Bir metin dosyasında `eski_metin`i `yeni_metin` ile değiştirir. `eski_metin` dosyada tek bir yerde geçmeli; birden çok yerde geçiyorsa daha fazla bağlam ekle ya da `tumu` kullan."
    }

    fn parametre_semasi(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "yol": {"type": "string", "description": "Çalışma alanına göre göreli yol"},
                "eski_metin": {"type": "string", "description": "Değiştirilecek metin; dosyada birebir geçmeli"},
                "yeni_metin": {"type": "string"},
                "tumu": {"type": "boolean", "description": "true ise bütün geçişler değişir"}
            },
            "required": ["yol", "eski_metin", "yeni_metin"],
            "additionalProperties": false
        })
    }

    fn yazar_mi(&self) -> bool {
        true
    }

    async fn hazirla(&self, a: &Value) -> Result<Hazirlik, AracHatasi> {
        let yol = arguman::yol(a, "yol")?;
        let (veri, metin) = self.metin_oku(&yol)?;
        let d = uygula(&metin, a)?;
        Ok(Hazirlik {
            aciklama: format!("`{yol}` dosyasında {} değişiklik yapmak istiyor.", d.adet),
            diff: Some(birlesik_diff(yol.as_str(), &metin, &d.yeni)),
            yazilan: vec![YazmaHedefi {
                onceki_ozet: Some(ozet_hex(&veri)),
                baglanti_sayisi: self.alan.baglanti_sayisi(&yol)?,
                yol,
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
        let (veri, metin) = self.metin_oku(&yol)?;
        let hedef = h
            .yazilan
            .first()
            .ok_or_else(|| AracHatasi::Gecersiz("Düzenleme hazırlığında hedef yok.".into()))?;
        if hedef.yol != yol
            || Some(ozet_hex(&veri)).as_deref() != hedef.onceki_ozet.as_deref()
            || self.alan.baglanti_sayisi(&yol)? != hedef.baglanti_sayisi
        {
            return Err(AracHatasi::OnayEskidi);
        }
        let d = uygula(&metin, a)?;
        self.alan.yaz(&yol, d.yeni.as_bytes())?;
        Ok(AracCiktisi {
            icerik: format!("`{yol}` dosyasında {} değişiklik yapıldı.", d.adet),
            kaynak: format!("dosya:{yol}"),
        })
    }
}
