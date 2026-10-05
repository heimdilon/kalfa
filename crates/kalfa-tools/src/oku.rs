//! `dosya_oku`: satır numaralı metin okuma.

use std::fmt::Write;
use std::sync::Arc;

use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, Hazirlik};
use kalfa_core::protokol::VeriCikisi;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::arguman;
use crate::calisma_alani::{CalismaAlani, ikili_mi};

const AZAMI_SATIR: u64 = 2000;

pub struct DosyaOku {
    alan: Arc<CalismaAlani>,
}

impl DosyaOku {
    pub fn yeni(alan: Arc<CalismaAlani>) -> Self {
        Self { alan }
    }
}

#[async_trait]
impl Arac for DosyaOku {
    fn ad(&self) -> &str {
        "dosya_oku"
    }

    fn aciklama(&self) -> &str {
        "Çalışma alanındaki bir metin dosyasını satır numaralarıyla okur. Uzun dosyalarda `baslangic_satiri` ve `satir_sayisi` ile bir bölüm seç (en çok 2000 satır)."
    }

    fn parametre_semasi(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "yol": {"type": "string", "description": "Çalışma alanına göre göreli yol"},
                "baslangic_satiri": {"type": "integer", "minimum": 1},
                "satir_sayisi": {"type": "integer", "minimum": 1, "maximum": AZAMI_SATIR}
            },
            "required": ["yol"],
            "additionalProperties": false
        })
    }

    fn yazar_mi(&self) -> bool {
        false
    }

    async fn hazirla(&self, a: &Value) -> Result<Hazirlik, AracHatasi> {
        let yol = arguman::yol(a, "yol")?;
        let alan = self.alan.clone();
        let okuma_yolu = yol.clone();
        let sayi = tokio::task::spawn_blocking(move || alan.baglanti_sayisi(&okuma_yolu))
            .await
            .map_err(|e| AracHatasi::GirisCikis(e.to_string()))??;
        Ok(Hazirlik {
            aciklama: format!("`{yol}` dosyasını okumak istiyor."),
            okuma_baglanti_sayilari: vec![(yol.clone(), sayi)],
            okunan: vec![yol],
            veri_cikisi: Some(VeriCikisi::DosyaIcerigi),
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
        let sayi = h
            .okuma_baglanti_sayilari
            .iter()
            .find(|(hedef, _)| hedef == &yol)
            .map(|(_, sayi)| *sayi)
            .ok_or(AracHatasi::OnayEskidi)?;
        let alan = self.alan.clone();
        let okuma_yolu = yol.clone();
        let okuma_iptali = iptal.clone();
        let veri = tokio::task::spawn_blocking(move || {
            alan.sinirli_onayli_oku(&okuma_yolu, &okuma_iptali, sayi)
        })
        .await
        .map_err(|e| AracHatasi::GirisCikis(e.to_string()))??;
        if iptal.is_cancelled() {
            return Err(AracHatasi::Iptal);
        }
        if ikili_mi(&veri) {
            return Err(AracHatasi::Gecersiz(format!(
                "`{yol}` ikili (binary) bir dosya; okunmadı."
            )));
        }

        let metin = String::from_utf8_lossy(&veri);
        let toplam = metin.lines().count();
        if toplam == 0 {
            return Ok(AracCiktisi {
                icerik: "[dosya boş]".into(),
                kaynak: format!("dosya:{yol}"),
                guvenlik_uyarilari: Vec::new(),
                degisiklikler: None,
            });
        }

        let bas = usize::try_from(arguman::sayi(a, "baslangic_satiri").unwrap_or(1).max(1))
            .unwrap_or(usize::MAX);
        let adet = usize::try_from(
            arguman::sayi(a, "satir_sayisi")
                .unwrap_or(AZAMI_SATIR)
                .min(AZAMI_SATIR),
        )
        .unwrap_or(usize::MAX);
        if bas > toplam {
            return Err(AracHatasi::Gecersiz(format!(
                "`{yol}` {toplam} satır; {bas}. satır yok."
            )));
        }

        let son = (bas - 1).saturating_add(adet).min(toplam);
        let mut cikti = String::new();
        for (i, satir) in metin.lines().enumerate().take(son).skip(bas - 1) {
            let _ = writeln!(cikti, "{:>6}\t{satir}", i + 1);
        }
        if son < toplam {
            let _ = writeln!(
                cikti,
                "[devamı var: {toplam} satırın {bas}-{son} arasındaki gösterildi]"
            );
        }
        Ok(AracCiktisi {
            icerik: cikti,
            kaynak: format!("dosya:{yol}:{bas}-{son}"),
            guvenlik_uyarilari: Vec::new(),
            degisiklikler: None,
        })
    }
}
