//! SSE çerçeveleme — WHATWG "event stream" ayrıştırma kuralları.
//!
//! Bayt akışını olaylara böler. UTF-8 çözümü satır ayrıldıktan **sonra**
//! yapılır: `\r` ve `\n` baytları çok baytlı bir UTF-8 dizisinin içinde yer
//! alamaz, bu yüzden bir karakter ağ parçası sınırında bölünse de bozulmaz
//! (spec §5.1).

/// 4 MiB: tek bir olayın ya da tamamlanmamış satırın üst sınırı.
pub const VARSAYILAN_AZAMI_OLAY: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseOlayi {
    pub olay: Option<String>,
    pub veri: String,
    pub kimlik: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SseHatasi {
    #[error("SSE olayı {azami} baytlık sınırı aştı")]
    OlayCokBuyuk { azami: usize },
}

#[derive(Debug)]
pub struct SseCozucu {
    azami: usize,
    satir: Vec<u8>,
    cr_sonrasi: bool,
    ilk_satir: bool,
    veri: String,
    veri_var: bool,
    olay: Option<String>,
    kimlik: Option<String>,
}

impl SseCozucu {
    pub fn yeni(azami_olay: usize) -> Self {
        Self {
            azami: azami_olay,
            satir: Vec::new(),
            cr_sonrasi: false,
            ilk_satir: true,
            veri: String::new(),
            veri_var: false,
            olay: None,
            kimlik: None,
        }
    }

    /// Bir ağ parçasını işler; tamamlanan olayları döndürür.
    pub fn besle(&mut self, parca: &[u8]) -> Result<Vec<SseOlayi>, SseHatasi> {
        let mut olaylar = Vec::new();
        for &bayt in parca {
            if self.cr_sonrasi {
                self.cr_sonrasi = false;
                if bayt == b'\n' {
                    continue;
                }
            }
            match bayt {
                b'\r' => {
                    self.cr_sonrasi = true;
                    self.satiri_isle(&mut olaylar)?;
                }
                b'\n' => self.satiri_isle(&mut olaylar)?,
                _ => {
                    self.satir.push(bayt);
                    if self.satir.len() > self.azami {
                        return Err(SseHatasi::OlayCokBuyuk { azami: self.azami });
                    }
                }
            }
        }
        Ok(olaylar)
    }

    /// Akış bitti. Tamamlanmamış olay WHATWG kuralı gereği atılır; dönüş
    /// değeri böyle bir artık kalıp kalmadığını söyler.
    pub fn bitir(self) -> bool {
        !self.satir.is_empty() || self.veri_var || self.olay.is_some() || self.kimlik.is_some()
    }

    fn satiri_isle(&mut self, olaylar: &mut Vec<SseOlayi>) -> Result<(), SseHatasi> {
        let ham = std::mem::take(&mut self.satir);
        let mut satir = String::from_utf8_lossy(&ham).into_owned();
        if self.ilk_satir {
            self.ilk_satir = false;
            if let Some(bomsuz) = satir.strip_prefix('\u{FEFF}') {
                satir = bomsuz.to_string();
            }
        }

        if satir.is_empty() {
            if self.veri_var {
                let mut veri = std::mem::take(&mut self.veri);
                if veri.ends_with('\n') {
                    veri.pop();
                }
                olaylar.push(SseOlayi {
                    olay: self.olay.take(),
                    veri,
                    kimlik: self.kimlik.take(),
                });
            } else {
                self.olay = None;
                self.kimlik = None;
            }
            self.veri_var = false;
            return Ok(());
        }

        if satir.starts_with(':') {
            return Ok(());
        }

        let (alan, deger) = match satir.find(':') {
            Some(i) => {
                let deger = &satir[i + 1..];
                (&satir[..i], deger.strip_prefix(' ').unwrap_or(deger))
            }
            None => (satir.as_str(), ""),
        };
        match alan {
            "data" => {
                self.veri.push_str(deger);
                self.veri.push('\n');
                self.veri_var = true;
                if self.veri.len() > self.azami {
                    return Err(SseHatasi::OlayCokBuyuk { azami: self.azami });
                }
            }
            "event" => self.olay = Some(deger.to_string()),
            "id" if !deger.contains('\0') => self.kimlik = Some(deger.to_string()),
            _ => {} // "retry" ve bilinmeyen alanlar yok sayılır
        }
        Ok(())
    }
}
