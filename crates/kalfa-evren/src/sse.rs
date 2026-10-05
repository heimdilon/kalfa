//! SSE çerçeveleme — WHATWG "event stream" ayrıştırma kuralları.
//!
//! Bayt akışını olaylara böler. UTF-8 çözümü satır ayrıldıktan **sonra**
//! yapılır: `\r` ve `\n` baytları çok baytlı bir UTF-8 dizisinin içinde yer
//! alamaz, bu yüzden bir karakter ağ parçası sınırında bölünse de bozulmaz
//! (spec §5.1).
//!
//! "Dispatch the event" adımında yalnızca veri ve olay-türü arabellekleri
//! sıfırlanır; son-olay-kimliği arabelleği bağlantı boyunca kalıcıdır ve
//! yalnızca sonraki bir `id:` alanıyla değiştirilir (spec §9.2.6). Burada
//! bu, `kimlik: Option<String>` alanının dispatch'ler arasında hiç
//! sıfırlanmamasıyla temsil edilir: `None` "henüz hiç `id:` alanı
//! görülmedi", `Some("")` ise "`id:` alanı boş değerle görüldü" anlamına
//! gelir.

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
    #[error("SSE parçasının olay çıktısı {azami} baytlık sınırı aştı")]
    CiktiCokBuyuk { azami: usize },
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
    /// Son dispatch'ten (ya da başlangıçtan) beri en az bir alan satırı
    /// görüldü mü. `kimlik` dispatch'ler arasında kalıcı olduğundan
    /// `bitir()` içinde "tamamlanmamış olay var mı" sorusunu `kimlik` yerine
    /// bu bayrak cevaplar.
    alan_var: bool,
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
            alan_var: false,
        }
    }

    /// Bir ağ parçasını işler; tamamlanan olayları döndürür.
    pub fn besle(&mut self, parca: &[u8]) -> Result<Vec<SseOlayi>, SseHatasi> {
        let mut olaylar = Vec::new();
        let mut cikti_bayt = 0usize;
        let azami = crate::butce::YANIT_BAYT_SINIRI.max(
            self.azami
                .saturating_mul(3)
                .saturating_add(std::mem::size_of::<SseOlayi>()),
        );
        for olay in self.olaylar(parca) {
            let olay = olay?;
            for adet in [
                std::mem::size_of::<SseOlayi>(),
                olay.veri.len(),
                olay.olay.as_ref().map_or(0, String::len),
                olay.kimlik.as_ref().map_or(0, String::len),
            ] {
                cikti_bayt = cikti_bayt
                    .checked_add(adet)
                    .filter(|n| *n <= azami)
                    .ok_or(SseHatasi::CiktiCokBuyuk { azami })?;
            }
            olaylar.push(olay);
        }
        Ok(olaylar)
    }

    /// Olayları bir Vec içinde biriktirmeden, tamamlandıkça verir.
    pub fn olaylar<'a>(
        &'a mut self,
        parca: &'a [u8],
    ) -> impl Iterator<Item = Result<SseOlayi, SseHatasi>> + 'a {
        self.olay_iter(parca, true)
    }

    /// Sohbet istemcisi SSE kimliğini kullanmaz; büyük kimliği her yanıtta
    /// kopyalamaz. Ayrıştırıcının kalıcı kimlik durumu yine korunur.
    pub(crate) fn veri_olaylari<'a>(
        &'a mut self,
        parca: &'a [u8],
    ) -> impl Iterator<Item = Result<SseOlayi, SseHatasi>> + 'a {
        self.olay_iter(parca, false)
    }

    fn olay_iter<'a>(
        &'a mut self,
        parca: &'a [u8],
        kimligi_ekle: bool,
    ) -> impl Iterator<Item = Result<SseOlayi, SseHatasi>> + 'a {
        let mut durdu = false;
        parca.iter().filter_map(move |&bayt| {
            if durdu {
                return None;
            }
            match self.bayt_isle(bayt, kimligi_ekle) {
                Ok(Some(olay)) => Some(Ok(olay)),
                Ok(None) => None,
                Err(e) => {
                    durdu = true;
                    Some(Err(e))
                }
            }
        })
    }

    fn bayt_isle(&mut self, bayt: u8, kimligi_ekle: bool) -> Result<Option<SseOlayi>, SseHatasi> {
        if self.cr_sonrasi {
            self.cr_sonrasi = false;
            if bayt == b'\n' {
                return Ok(None);
            }
        }
        match bayt {
            b'\r' => {
                self.cr_sonrasi = true;
                self.satiri_isle(kimligi_ekle)
            }
            b'\n' => self.satiri_isle(kimligi_ekle),
            _ => {
                self.satir.push(bayt);
                if self.satir.len() > self.azami {
                    return Err(SseHatasi::OlayCokBuyuk { azami: self.azami });
                }
                Ok(None)
            }
        }
    }

    /// Akış bitti. Tamamlanmamış olay WHATWG kuralı gereği atılır; dönüş
    /// değeri böyle bir artık kalıp kalmadığını söyler.
    pub fn bitir(self) -> bool {
        !self.satir.is_empty() || self.veri_var || self.olay.is_some() || self.alan_var
    }

    fn satiri_isle(&mut self, kimligi_ekle: bool) -> Result<Option<SseOlayi>, SseHatasi> {
        let ham = std::mem::take(&mut self.satir);
        let mut satir = String::from_utf8_lossy(&ham).into_owned();
        if self.ilk_satir {
            self.ilk_satir = false;
            if let Some(bomsuz) = satir.strip_prefix('\u{FEFF}') {
                satir = bomsuz.to_string();
            }
        }

        if satir.is_empty() {
            let olay = if self.veri_var {
                let mut veri = std::mem::take(&mut self.veri);
                if veri.ends_with('\n') {
                    veri.pop();
                }
                // `kimlik` burada bilerek `clone`'lanır, `take()` edilmez:
                // son-olay-kimliği arabelleği dispatch sonrasında
                // sıfırlanmaz, sonraki olaylara da taşınır.
                Some(SseOlayi {
                    olay: self.olay.take(),
                    veri,
                    kimlik: if kimligi_ekle {
                        self.kimlik.clone()
                    } else {
                        None
                    },
                })
            } else {
                self.olay = None;
                None
            };
            self.veri_var = false;
            self.alan_var = false;
            return Ok(olay);
        }

        if satir.starts_with(':') {
            return Ok(None);
        }

        let (alan, deger) = match satir.find(':') {
            Some(i) => {
                let deger = &satir[i + 1..];
                (&satir[..i], deger.strip_prefix(' ').unwrap_or(deger))
            }
            None => (satir.as_str(), ""),
        };
        self.alan_var = true;
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
        Ok(None)
    }
}
