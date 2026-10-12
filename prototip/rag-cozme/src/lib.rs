//! G6 ölçüm konuğu; kaynak metni, dosya, ağ ve depo erişimi yoktur.
pub mod olcum;

use kalfa_plugin_sdk::{AracTanimi, Guest, Hata, Sonuc};

struct Olcum;

impl Guest for Olcum {
    fn araclari_listele() -> Vec<AracTanimi> {
        vec![AracTanimi {
            ad: "cozme".into(),
            aciklama: "G6: aynı vektörlerin çözülme süresini ölçer.".into(),
            parametre_semasi: r#"{"type":"object","properties":{"adet":{"enum":[1,16,256]},"boyut":{"enum":[1024,4096]},"yol":{"enum":["sdk-base64","sdk-float","ham-crate","ham-elle"]}},"required":["adet","boyut","yol"],"additionalProperties":false}"#.into(),
        }]
    }

    fn calistir(ad: String, argumanlar: String) -> Result<Sonuc, Hata> {
        if ad != "cozme" {
            return Err(Hata::Bulunamadi(ad));
        }
        olcum::olc(&argumanlar, "wasm")
            .map(|icerik| Sonuc { icerik })
            .map_err(Hata::Diger)
    }
}

kalfa_plugin_sdk::kalfa_eklentisi!(Olcum);
