# kalfa-plugin-sdk

Kalfa eklentilerini Rust'ta yazmak için SDK. Sözleşme: [`wit/kalfa.wit`](wit/kalfa.wit)
(`kalfa:plugin@0.1.0`). Tasarım: aşama 4 tasarımı §6 ve §12.

Lisans: `Apache-2.0 WITH LLVM-exception`. Eklentiniz kendi lisansını taşıyabilir.

## En küçük eklenti

`Cargo.toml`:

```toml
[package]
name = "ornek"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
kalfa-plugin-sdk = { git = "https://github.com/heimdilon/kalfa" }
```

`src/lib.rs`:

```rust
use kalfa_plugin_sdk::{AracTanimi, Guest, Hata, Sonuc, host, serde_json::Value};

struct Ornek;

impl Guest for Ornek {
    fn araclari_listele() -> Vec<AracTanimi> {
        vec![AracTanimi {
            ad: "satir_say".into(),
            aciklama: "Bir dosyanın satırlarını sayar.".into(),
            parametre_semasi: r#"{"type":"object","properties":{"yol":{"type":"string"}},"required":["yol"]}"#.into(),
        }]
    }

    fn calistir(_ad: String, argumanlar: String) -> Result<Sonuc, Hata> {
        let a: Value = kalfa_plugin_sdk::argumanlar(&argumanlar)?;
        let metin = host::dosya_oku(a["yol"].as_str().unwrap_or_default())?;
        Ok(Sonuc {
            icerik: format!("{} satır", metin.lines().count()),
        })
    }
}

kalfa_plugin_sdk::kalfa_eklentisi!(Ornek);
```

`kalfa.toml`:

```toml
[eklenti]
ad = "ornek"
surum = "0.1.0"
sozlesme = "0.1"
aciklama = "Satır sayar"

[yetenekler]
dosya-oku = ["**/*.rs"]
```

Derleme:

```sh
cargo build --target wasm32-wasip2 --release
```

Eklenti dizini `kalfa.toml` ve tek bir `.wasm` içerir. Eklentinin dosya
sistemine, ağa ya da ortam değişkenlerine doğrudan erişimi yoktur; dosyaları
yalnızca `host::dosya_oku` ve `host::dosya_listele` ile, manifestteki
globlar içinde okuyabilir. Gizli dosyalar eklentiye hiç açılmaz.

## EVREN ve depo yardımcıları

`gomme`, `yeniden_sirala` ve `depo` host çağrısı yaptığı için yalnızca eklenti
içinde (wasm) çalışır. İlgili yetenek manifestte olmalıdır, yoksa host
`izin-yok` döner:

- `gomme`: `evren = ["embeddings"]`
- `yeniden_sirala`: `evren = ["rerank"]`
- `depo`: `depo-mb`

```rust
use kalfa_plugin_sdk::{depo, gomme, yeniden_sirala};

// Her metin için bir vektör, aynı sırayla.
let vektorler = gomme("qwen3-embedding-8b", &metinler, 1024)?;

// (belgenin sırası, puan); puana göre azalan, en çok 5 tane.
let sirali = yeniden_sirala("qwen3-reranker-8b", "sorgu", &belgeler, 5)?;

// Değer JSON olarak saklanır; anahtar yoksa `None` döner.
depo::yaz("son-sorgu", &"merhaba")?;
let eski: Option<String> = depo::oku("son-sorgu")?;
```

İstek gövdesini ve yanıtı elle kurmak isterseniz `gomme_govdesi`,
`gomme_yanitini_coz`, `siralama_govdesi` ve `siralama_yanitini_coz` de dışa
verilir. Bunlar host çağrısı yapmaz, bu yüzden `cargo test` ile host hedefinde
sınanabilir.
