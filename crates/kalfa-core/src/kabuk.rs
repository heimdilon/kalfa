//! Kabuk çağrısının çekirdekteki tarifi (aşama 2 tasarımı §3.3). İzin kararı,
//! onay kartı ve mühür bundan üretilir; komutu çalıştıran kod
//! `kalfa-sandbox`'tadır.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KabukTarifi {
    /// Onay kartında gösterilen tam komut: program ve argümanları ya da
    /// betiğin bütün metni.
    pub komut: String,
    pub bicim: KabukBicimi,
    /// Çalışma alanına göre çalışma dizini; kök için `.`.
    pub dizin: String,
    /// Model ağ erişimi istedi. Yalıtımsız çalıştırmada ağ kapatılamaz; bu
    /// yalnızca modelin niyetidir.
    pub ag: bool,
    /// Modelin bağlanmak istediği ek hostlar (host:port biçiminde).
    #[serde(default)]
    pub ag_hostlari: Vec<String>,
    pub yalitim: YalitimOzeti,
    /// Bu çalışma alanındaki ilk kabuk çağrısı (aşama 2 tasarımı §4.2).
    /// Tek seferliktir; bkz. [`KabukTarifi::onayla_kapsanan`].
    pub ilk_cagri: bool,
    /// Çalışma alanı yalıtım için hazırlanacak (Windows, aşama 2 tasarımı
    /// §9.4): klasörün izinleri değişir. Onayı her kipte yerel pencerede
    /// sorulur. Tek seferliktir; bkz. [`KabukTarifi::onayla_kapsanan`].
    #[serde(default)]
    pub alan_hazirligi: bool,
}

impl KabukTarifi {
    /// Yürütmeden hemen önce yeniden hazırlanan tarifi onaylanana göre
    /// düzeltir.
    ///
    /// İlk çağrı ve alan hazırlığı tek seferliktir: aynı turda onaylanan
    /// başka bir komut çalışınca tükenir. Onaylanan tarifte olup taze tarifte
    /// olmayan koşul onaylı sayılır; onay bu koşulu içeren daha geniş işlemi
    /// kapsar. Tersi gevşemez: onaylanmayan koşul taze tarifte belirirse
    /// düzeltilmiş tarif onaylanandan farklıdır ve mühür tutmaz.
    pub fn onayla_kapsanan(mut self, onaylanan: &KabukTarifi) -> KabukTarifi {
        self.ilk_cagri |= onaylanan.ilk_cagri;
        self.alan_hazirligi |= onaylanan.alan_hazirligi;
        self
    }

    /// Onaylanan tarif (`self`) ile yürütmeden önceki taze tarif farklıysa
    /// nedenin iletisi.
    pub fn degisiklik_nedeni(&self, taze: &KabukTarifi) -> &'static str {
        if self.yalitim != taze.yalitim {
            "Yalıtım durumu ya da profili onaydan sonra değişti"
        } else if taze.alan_hazirligi && !self.alan_hazirligi {
            "Çalışma alanının yalıtım hazırlığı onaydan sonra geri alındı; hazırlık onaylanmadı"
        } else if taze.ilk_cagri && !self.ilk_cagri {
            "Çalışma alanındaki kabuk onayı onaydan sonra sıfırlandı"
        } else {
            "Komutun koşulları onaydan sonra değişti"
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum KabukBicimi {
    Program,
    Betik { yorumlayici: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum YalitimOzeti {
    /// `profil`: yalıtım profilinin özeti; profil değişirse bekleyen onaylar
    /// geçersizleşir.
    Yalitimli { arka_uc: String, profil: String },
    /// `talimat`: yalıtımın neden olmadığı ve nasıl açılacağı.
    Yalitimsiz { talimat: String },
}

impl YalitimOzeti {
    pub fn yalitimli_mi(&self) -> bool {
        matches!(self, Self::Yalitimli { .. })
    }
}
