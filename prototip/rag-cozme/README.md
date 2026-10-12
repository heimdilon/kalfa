# G6 — Gömme yanıtını çözme ölçümü

Ürün workspace'ine girmeyen ölçüm konuğu. Mevcut `kalfa-plugin-sdk`,
`serde_json` ve `base64` kullanılır; ağ/dosya/depo yeteneği yoktur. EVREN
çağrısı yapılmaz, anahtar okunmaz ve kredi harcanmaz.

1024/4096 boyut × 1/16/256 vektör × dört çözme yolu ölçülür:

- `sdk-base64` ve `sdk-float`: gerçek `gomme_yanitini_coz`; JSON çözme,
  ayırma, vektör üretme ve SDK doğrulaması dahil.
- `ham-crate` ve `ham-elle`: hazırlanmış metinden yalnızca base64 çözme ve
  küçük endian `f32` dönüşümü. JSON ve SDK doğrulaması bu iki yolda yoktur;
  tam SDK yollarıyla doğrudan aynı aşama gibi karşılaştırılmaz.

Vektörler deponun iki gerçek EVREN kaydından gelir. Her toplu vektör aynı
değerlerin farklı döndürmesini kullanır. İki JSON biçimi aynı `f32` bitlerini
taşır; çözülmüş bütün bitler zamanlama başlamadan eşleştirilir. Yanıt üretimi,
bu doğrulama ve wasm yükleme/örnekleme süresi ölçüm aralığının dışındadır.

Her yol üç kez ısınır; tekrar sayısı en az 10 ms'lik bir pencereye kadar
ikiye katlanır (en çok 16 384). Ardından 21 örneğin medyanı, 20. sıralı
örneği (en yakın sıra yöntemiyle p95) ve en küçüğü JSONL'ye yazılır. Süre
`Instant` ile ölçülen geçen süredir. Wasm'da gerçek motorun epoch/yer verme
davranışı dahildir; işletim sisteminin süreç CPU sayacı değildir.

## Çalıştırma

Rust 1.98.1 ve deponun önceden kurulmuş bağımlılıkları gerekir. Önce depo
köküne geçin. Tek bir platformdaki ölçüm sırasında başka derleme/test
başlatmayın. Konuk ve yerel ölçüm `release`, host örneği `dev` profilindedir;
`Motor::yeni` üretimdeki wasmtime 49.0.2 ve sınırları kullanır.

Windows PowerShell:

```powershell
$env:CARGO_TARGET_DIR = Join-Path (Get-Location) 'target'
$env:CARGO_NET_OFFLINE = 'true'
cargo rustc --manifest-path prototip/rag-cozme/Cargo.toml --lib --crate-type cdylib --target wasm32-wasip2 --release --locked --offline
cargo build --manifest-path prototip/rag-cozme/Cargo.toml --release --bin g6-yerel --locked --offline
cargo build -p kalfa-plugin --example g6_cozme --locked --offline
& "$env:CARGO_TARGET_DIR/release/g6-yerel.exe" > g6-yerel.jsonl
& "$env:CARGO_TARGET_DIR/debug/examples/g6_cozme.exe" "$env:CARGO_TARGET_DIR/wasm32-wasip2/release/kalfa_g6_olcum.wasm" > g6-wasm.jsonl
```

WSL/Linux (root olmayan kullanıcı):

```sh
export CARGO_TARGET_DIR="$PWD/target"
export CARGO_NET_OFFLINE=true
cargo rustc --manifest-path prototip/rag-cozme/Cargo.toml --lib --crate-type cdylib --target wasm32-wasip2 --release --locked --offline
cargo build --manifest-path prototip/rag-cozme/Cargo.toml --release --bin g6-yerel --locked --offline
cargo build -p kalfa-plugin --example g6_cozme --locked --offline
"$CARGO_TARGET_DIR/release/g6-yerel" > g6-yerel.jsonl
"$CARGO_TARGET_DIR/debug/examples/g6_cozme" "$CARGO_TARGET_DIR/wasm32-wasip2/release/kalfa_g6_olcum.wasm" > g6-wasm.jsonl
```

## Düzeneğin doğrulaması

```sh
cargo test --manifest-path prototip/rag-cozme/Cargo.toml --locked --offline
cargo clippy --manifest-path prototip/rag-cozme/Cargo.toml --all-targets --locked --offline -- -D warnings
cargo clippy --manifest-path prototip/rag-cozme/Cargo.toml --lib --target wasm32-wasip2 --locked --offline -- -D warnings
cargo fmt --manifest-path prototip/rag-cozme/Cargo.toml -- --check
```

Host örneği workspace'in normal `--all-targets` clippy ve biçim denetimine
girer. Ölçümün örnek sayısı ya da vektörleri değiştirilirse eski JSONL ile
tek ölçümmüş gibi birleştirmeyin. Elle çözücü yalnızca kıyas prototipidir;
ürünün SDK çözücüsünü değiştirmez.
