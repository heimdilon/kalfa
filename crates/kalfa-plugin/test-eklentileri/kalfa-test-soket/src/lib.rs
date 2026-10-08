//! `wasi:sockets` import eden test eklentisi (aşama 4 tasarımı §16.1).
//! Yükleme anındaki import denetimi onu reddetmeli; denetim atlansa bile
//! ağsız `WasiCtx` bağlantıya izin vermemeli.

use kalfa_plugin_sdk::serde_json::Value;
use kalfa_plugin_sdk::{AracTanimi, Guest, Hata, Sonuc};

struct Soket;

impl Guest for Soket {
    fn araclari_listele() -> Vec<AracTanimi> {
        vec![AracTanimi {
            ad: "baglan".into(),
            aciklama: "Verilen adrese TCP ile bağlanmayı dener.".into(),
            parametre_semasi: r#"{"type":"object"}"#.into(),
        }]
    }

    fn calistir(_ad: String, argumanlar: String) -> Result<Sonuc, Hata> {
        let a: Value = kalfa_plugin_sdk::argumanlar(&argumanlar)?;
        let adres = a["adres"].as_str().unwrap_or("127.0.0.1:9");
        let icerik = match std::net::TcpStream::connect(adres) {
            Ok(_) => "BAĞLANDI".to_string(),
            Err(e) => format!("bağlanamadı: {e}"),
        };
        Ok(Sonuc { icerik })
    }
}

kalfa_plugin_sdk::kalfa_eklentisi!(Soket);
