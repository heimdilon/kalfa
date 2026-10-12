//! G6 ölçüm konuğunu ürünün gerçek wasm/host yolunda çalıştırır.
use std::sync::Arc;

use kalfa_core::arac::AracOrtami;
use kalfa_core::izin::Siniflandirici;
use kalfa_core::kilit::KaynakKilitleri;
use kalfa_plugin::{BellekIzni, EklentiAraci, Motor};
use kalfa_tools::CalismaAlani;
use serde_json::json;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let yol = std::env::args_os()
        .nth(1)
        .ok_or("ölçüm .wasm yolunu verin")?;
    let motor = Motor::yeni()?;
    let manifest = "[eklenti]\nad = \"kalfa-g6\"\nsurum = \"0.1.0\"\nsozlesme = \"0.1\"\naciklama = \"G6 çözme ölçümü\"\n[yetenekler]\ndosya-oku = []\nevren = []\ndepo-mb = 0\n";
    let eklenti = Arc::new(motor.yukle(manifest, std::fs::read(yol)?).await?);
    let alan = tempfile::tempdir()?;
    let depo = tempfile::tempdir()?;
    let siniflandirici = Arc::new(Siniflandirici::yeni(alan.path(), &[])?);
    let arac = EklentiAraci::hepsi(
        &motor,
        &eklenti,
        &CalismaAlani::ac(siniflandirici)?,
        &Arc::new(KaynakKilitleri::yeni()),
        depo.path(),
        None,
        BellekIzni::yeni(false),
    )
    .into_iter()
    .next()
    .ok_or("ölçüm aracı yok")?;
    for boyut in [1024, 4096] {
        for adet in [1, 16, 256] {
            for yol in ["sdk-base64", "sdk-float", "ham-crate", "ham-elle"] {
                let a = json!({"adet":adet,"boyut":boyut,"yol":yol});
                let hazirlik = arac.hazirla(&a).await?;
                let sonuc = arac
                    .calistir_ortamli(&a, &hazirlik, CancellationToken::new(), AracOrtami::bos())
                    .await?;
                println!("{}", sonuc.icerik);
            }
        }
    }
    Ok(())
}
