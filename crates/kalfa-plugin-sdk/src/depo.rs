//! Depo için tipli okuma ve yazma (aşama 4 tasarımı §12): değer JSON olarak
//! saklanır.

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{Hata, host};

/// Anahtarın değeri; yoksa `None`.
pub fn oku<T: DeserializeOwned>(anahtar: &str) -> Result<Option<T>, Hata> {
    match host::depo_oku(anahtar)? {
        Some(bayt) => degeri_coz(&bayt).map(Some),
        None => Ok(None),
    }
}

/// Değeri JSON olarak yazar; anahtarda eski değer varsa yerini alır.
pub fn yaz<T: Serialize>(anahtar: &str, deger: &T) -> Result<(), Hata> {
    host::depo_yaz(anahtar, &degeri_kodla(deger)?)
}

fn degeri_coz<T: DeserializeOwned>(bayt: &[u8]) -> Result<T, Hata> {
    serde_json::from_slice(bayt).map_err(|e| Hata::Diger(format!("depo değeri çözülemedi: {e}")))
}

fn degeri_kodla<T: Serialize>(deger: &T) -> Result<Vec<u8>, Hata> {
    serde_json::to_vec(deger).map_err(|e| Hata::Diger(format!("depo değeri kodlanamadı: {e}")))
}

#[cfg(test)]
mod testler {
    use serde::{Deserialize, Serialize};

    use super::{degeri_coz, degeri_kodla};
    use crate::Hata;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Kayit {
        ad: String,
        sayi: u32,
    }

    #[test]
    fn deger_kodlanip_cozulur() {
        let kayit = Kayit {
            ad: "kalfa".into(),
            sayi: 7,
        };
        let bayt = degeri_kodla(&kayit).expect("kodlanır");
        let geri: Kayit = degeri_coz(&bayt).expect("çözülür");
        assert_eq!(geri, kayit);
    }

    #[test]
    fn bozuk_deger_diger_hatasi_verir() {
        match degeri_coz::<Kayit>(b"{bozuk") {
            Err(Hata::Diger(ileti)) => assert!(ileti.starts_with("depo değeri çözülemedi: ")),
            diger => panic!("Diger hatası bekleniyordu: {diger:?}"),
        }
    }
}
