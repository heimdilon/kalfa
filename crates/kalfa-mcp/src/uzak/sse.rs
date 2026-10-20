//! Uzak MCP Streamable HTTP yanıtları için sınırlı (`4 MiB`) SSE ayrıştırıcısı.
//!
//! Tamponlama sınırı her satır ve her olay (`data:`) birikimi için ayrı ayrı
//! zorlanır; yarım kalan satır veya boş satırla (`\n\n`) kapanmadan kesilen
//! olay [`TasimaHatasi::YarimSatir`] olarak reddedilir.

use crate::tasima::{CERCEVE, TasimaHatasi};

/// Tek bir tamamlanmış SSE olayı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseOlayi {
    pub olay_turu: String,
    pub veri: String,
    pub son_olay_kimligi: Option<String>,
}

/// Akıştan gelen bayt parçalarını (`chunk`) sınırlı tamponla SSE olaylarına ayırır.
pub struct SseAkisAyristiricisi {
    cerceve_siniri: usize,
    satir_tamponu: Vec<u8>,
    mevcut_olay_turu: Option<String>,
    mevcut_veri: Vec<u8>,
    mevcut_kimlik: Option<String>,
    veri_satiri_goruldu: bool,
}

impl Default for SseAkisAyristiricisi {
    fn default() -> Self {
        Self::yeni(CERCEVE)
    }
}

impl SseAkisAyristiricisi {
    pub fn yeni(cerceve_siniri: usize) -> Self {
        Self {
            cerceve_siniri,
            satir_tamponu: Vec::new(),
            mevcut_olay_turu: None,
            mevcut_veri: Vec::new(),
            mevcut_kimlik: None,
            veri_satiri_goruldu: false,
        }
    }

    /// Gelen bayt parçasını işler ve tamamlanan SSE olaylarını sırayla döner.
    pub fn parca_besle(&mut self, parca: &[u8]) -> Result<Vec<SseOlayi>, TasimaHatasi> {
        let mut olaylar = Vec::new();
        for &bayt in parca {
            if bayt == b'\n' {
                if self.satir_tamponu.last() == Some(&b'\r') {
                    self.satir_tamponu.pop();
                }
                let ham_satir = std::mem::take(&mut self.satir_tamponu);
                if let Some(olay) = self.satir_isle(&ham_satir)? {
                    olaylar.push(olay);
                }
            } else {
                if self.satir_tamponu.len() >= self.cerceve_siniri {
                    return Err(TasimaHatasi::Cerceve);
                }
                self.satir_tamponu.push(bayt);
            }
        }
        Ok(olaylar)
    }

    /// HTTP gövdesi sona erdiğinde çağrılır; tamamlanmamış satır veya boş satırla
    /// kapanmamış olay varsa [`TasimaHatasi::YarimSatir`] döner.
    pub fn bitir(&mut self) -> Result<(), TasimaHatasi> {
        if !self.satir_tamponu.is_empty() || self.veri_satiri_goruldu {
            return Err(TasimaHatasi::YarimSatir);
        }
        Ok(())
    }

    fn satir_isle(&mut self, ham_satir: &[u8]) -> Result<Option<SseOlayi>, TasimaHatasi> {
        if ham_satir.is_empty() {
            if !self.veri_satiri_goruldu {
                self.mevcut_olay_turu = None;
                return Ok(None);
            }
            let veri_bayt = std::mem::take(&mut self.mevcut_veri);
            let veri = String::from_utf8(veri_bayt).map_err(|_| TasimaHatasi::BozukJson)?;
            let olay_turu = self
                .mevcut_olay_turu
                .take()
                .unwrap_or_else(|| "message".to_owned());
            self.veri_satiri_goruldu = false;
            return Ok(Some(SseOlayi {
                olay_turu,
                veri,
                son_olay_kimligi: self.mevcut_kimlik.clone(),
            }));
        }

        // Yorum satırları (`:` ile başlayanlar) yoksayılır.
        if ham_satir.starts_with(b":") {
            return Ok(None);
        }

        let satir = std::str::from_utf8(ham_satir).map_err(|_| TasimaHatasi::BozukJson)?;
        let (alan, ham_deger) = match satir.split_once(':') {
            Some((a, d)) => (a, d.strip_prefix(' ').unwrap_or(d)),
            None => (satir, ""),
        };

        match alan {
            "event" => {
                self.mevcut_olay_turu = Some(ham_deger.to_owned());
            }
            "data" => {
                let eklenecek_uzunluk = ham_deger.len() + usize::from(self.veri_satiri_goruldu);
                if self
                    .mevcut_veri
                    .len()
                    .checked_add(eklenecek_uzunluk)
                    .is_none_or(|toplam| toplam > self.cerceve_siniri)
                {
                    return Err(TasimaHatasi::Cerceve);
                }
                if self.veri_satiri_goruldu {
                    self.mevcut_veri.push(b'\n');
                }
                self.mevcut_veri.extend_from_slice(ham_deger.as_bytes());
                self.veri_satiri_goruldu = true;
            }
            "id" if !ham_deger.contains('\0') && ham_deger.len() <= 256 => {
                self.mevcut_kimlik = Some(ham_deger.to_owned());
            }
            _ => {}
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_cok_satirli_olayi_birlestirir_ve_cerceve_ile_yarim_akisi_reddeder() {
        let mut sse = SseAkisAyristiricisi::yeni(64);
        let olaylar = sse
            .parca_besle(b": yorum\nevent: message\nid: 7\ndata: {\"a\":\ndata: 1}\n\n")
            .unwrap();
        assert_eq!(olaylar.len(), 1);
        assert_eq!(olaylar[0].olay_turu, "message");
        assert_eq!(olaylar[0].veri, "{\"a\":\n1}");
        assert_eq!(olaylar[0].son_olay_kimligi.as_deref(), Some("7"));
        sse.bitir().unwrap();

        // Boş satır olmadan kapanan olay YarimSatir üretmeli.
        let mut yarim = SseAkisAyristiricisi::yeni(64);
        assert!(
            yarim
                .parca_besle(b"data: {\"yarim\":true}\n")
                .unwrap()
                .is_empty()
        );
        assert_eq!(yarim.bitir(), Err(TasimaHatasi::YarimSatir));

        // 64 bayt sınırını aşan veri Cerceve üretmeli.
        let mut tasan = SseAkisAyristiricisi::yeni(16);
        assert_eq!(
            tasan.parca_besle(b"data: 0123456789abcdef0\n\n"),
            Err(TasimaHatasi::Cerceve)
        );
    }
}
