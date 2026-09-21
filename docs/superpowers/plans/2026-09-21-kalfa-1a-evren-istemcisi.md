# Kalfa 1a — EVREN İstemcisi (`kalfa-evren`) Uygulama Planı

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** EVREN LLM Gateway'e konuşan, tek başına yayımlanabilir ve kayıtlı gerçek yanıtlara karşı test edilmiş bir Rust istemcisi (`kalfa-evren`) kurmak.

**Architecture:** Dört katman: (1) bayt akışını SSE olaylarına bölen WHATWG uyumlu çerçeveleyici, (2) EVREN'in OpenAI uyumlu JSON şemasının tipleri — EVREN'e özgü alanlar dahil, (3) akış parçalarını tamamlanmış olaylara çeviren birleştirici, (4) `reqwest` üzerinde HTTP istemcisi (şart uçları, katalog, bloklu ve akışlı sohbet). Hatalar tipli `EvrenError`'a, kredi başlıkları `KrediGozlemi`'ne çevrilir.

**Tech Stack:** Rust 1.98.1 (edition 2024) · tokio · reqwest 0.13 (rustls + ring) · serde / serde_json · rust_decimal · thiserror · wiremock (test)

**Spec:** `docs/superpowers/specs/2026-09-21-kalfa-design.md` — özellikle §3 (doğrulanmış API gerçekleri), §5 (sağlayıcı katmanı), §13.1 (fixture testleri).

**Plan dizisi:** Bu plan birinci aşamanın (§12.3) ilk yarısıdır. İkinci yarısı `2026-09-21-kalfa-1b-cekirdek.md` (`kalfa-core` + `kalfa-tools`) bu crate'in `EvrenIstemcisi`, `AkisOlayi`, `EvrenError`, `YanitMeta`, `KrediGozlemi` ve `tipler` modülünü kullanır.

## Global Constraints

- **Araç zinciri:** Rust `1.98.1`, `rust-toolchain.toml` ile sabit; edition `2024`; workspace resolver `3`; `rust-version = "1.98"`.
- **Lisans:** bu plandaki her crate `license = "Apache-2.0"` (spec §8.7; LLVM istisnası yalnızca `kalfa-plugin-sdk` içindir, bu planda yok).
- **TLS:** `reqwest` `default-features = false` ve `rustls-no-provider`; kripto sağlayıcı `rustls` `ring` özelliği. `aws-lc-rs` kullanılmaz (Windows'ta CMake/NASM ister).
- **Fixture kuralı:** `research/evren-probe/fixtures/` altındaki dosyalar gerçek EVREN yanıtlarıdır; **değiştirilmez**. Gözlenmemiş durumlar için sentetik veri yalnızca `crates/kalfa-evren/tests/sentetik/` altına konur.
- **Anahtar gizliliği:** API anahtarı hiçbir `Debug`, `Display`, hata metni veya günlük çıktısında görünmez.
- **Hata metinleri:** kullanıcıya dönük metinler Türkçedir ve ne olduğunu, mümkünse ne yapılacağını söyler (spec §11.6).
- **Adlandırma:** spec'teki alan kavramları Türkçe ASCII adlarla (`EvrenError::SartKabulGerekli`, `KrediGozlemi`, `AkisOlayi`); EVREN/OpenAI JSON şemasını yansıtan tipler İngilizce ve JSON alan adlarıyla birebir (`ChatRequest`, `ToolCall`).
- **Kod kuralı:** kütüphane kodunda `unwrap()`/`expect()` yok; testlerde serbest.
- **Her görevin sonunda:** `cargo test -p kalfa-evren` yeşil, `cargo clippy -p kalfa-evren --all-targets -- -D warnings` temiz, `cargo fmt --all -- --check` temiz.
- **Commit mesajları** Türkçe, tek satır özet + gerekirse gövde.

## Dosya Yapısı

```
Cargo.toml                                 workspace kökü (yeni)
rust-toolchain.toml                        araç zinciri sabitleme (yeni)
crates/kalfa-evren/
  Cargo.toml
  src/lib.rs                               modül bildirimleri ve yeniden dışa aktarımlar
  src/sse.rs                               SSE çerçeveleyici (bayt → SseOlayi)
  src/tipler.rs                            JSON tipleri (istek, yanıt, parça, katalog, şart, hata gövdesi)
  src/hata.rs                              EvrenError + HTTP durum/gövde → hata eşlemesi
  src/kredi.rs                             yanıt başlıkları → YanitMeta / KrediGozlemi
  src/akis.rs                              AkisBirlestirici (ChatChunk → AkisOlayi)
  src/istemci.rs                           EvrenIstemcisi (reqwest)
  tests/ortak/mod.rs                       fixture okuma yardımcıları
  tests/sse.rs                             çerçeveleyici testleri (sentetik + fixture)
  tests/tipler.rs                          fixture'ları tiplere çözme testleri
  tests/hata_kredi.rs                      hata eşleme ve başlık testleri
  tests/akis.rs                            birleştirici testleri (3 akış fixture'ı)
  tests/istemci.rs                         wiremock ile HTTP testleri
  tests/canli.rs                           #[ignore] canlı EVREN testleri
  tests/sentetik/README.md                 sentetik verinin neden var olduğu
  tests/sentetik/terms_not_accepted.json   403 şart kapısı gövdesi (spec §3.1'de belgeli)
```

---

### Task 1: Ortam, workspace ve SSE çerçeveleyici

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`
- Create: `crates/kalfa-evren/Cargo.toml`, `crates/kalfa-evren/src/lib.rs`, `crates/kalfa-evren/src/sse.rs`
- Test: `crates/kalfa-evren/tests/ortak/mod.rs`, `crates/kalfa-evren/tests/sse.rs`

**Interfaces:**
- Consumes: —
- Produces:
  - `kalfa_evren::sse::SseOlayi { olay: Option<String>, veri: String, kimlik: Option<String> }`
  - `kalfa_evren::sse::SseHatasi::OlayCokBuyuk { azami: usize }`
  - `kalfa_evren::sse::VARSAYILAN_AZAMI_OLAY: usize` (4 MiB)
  - `kalfa_evren::sse::SseCozucu::yeni(azami_olay: usize) -> SseCozucu`
  - `SseCozucu::besle(&mut self, parca: &[u8]) -> Result<Vec<SseOlayi>, SseHatasi>`
  - `SseCozucu::bitir(self) -> bool` (atılan tamamlanmamış olay kaldıysa `true`)
  - `tests/ortak`: `fixture_bayt(ad: &str) -> Vec<u8>`, `fixture_metin(ad: &str) -> String`

- [ ] **Step 1: Rust'ı kur ve doğrula**

Bu makinede Rust kurulu değil; Visual Studio Build Tools'un C++ bileşeni kurulu (MSVC bağlayıcısı için gerekli). PowerShell'de:

```powershell
winget install --id Rustlang.Rustup -e --accept-source-agreements --accept-package-agreements
```

Yeni bir kabuk aç (ya da Git Bash'te `export PATH="$HOME/.cargo/bin:$PATH"`), sonra:

```bash
rustup --version
```

Expected: `rustup 1.x.y ...` satırı. (Araç zincirinin kendisi Step 2'deki `rust-toolchain.toml` ile ilk `cargo` çağrısında `1.98.1` olarak indirilir.)

> Bağlayıcı sorunu çıkarsa: `PATH`'te Git'in `usr/bin/link.exe`'si MSVC'nin `link.exe`'sinin önünde olabilir. `rustc` MSVC'yi normalde `vswhere` ile bulur; "link: extra operand" türü bir hata görürsen komutu "Developer PowerShell for VS" içinden çalıştır.

- [ ] **Step 2: Workspace dosyalarını oluştur**

`Cargo.toml`:

```toml
[workspace]
resolver = "3"
members = ["crates/kalfa-evren"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.98"
license = "Apache-2.0"
authors = ["Orkun Koray Güngör"]

[workspace.dependencies]
bytes = "1.12"
futures-util = "0.3.34"
reqwest = { version = "0.13.5", default-features = false, features = ["rustls-no-provider", "json", "stream", "http2", "charset", "system-proxy"] }
rust_decimal = { version = "1.43", features = ["serde"] }
rustls = { version = "0.23.45", default-features = false, features = ["ring", "std", "tls12", "logging"] }
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0.151"
thiserror = "2.0.20"
tokio = { version = "1.53", features = ["rt-multi-thread", "macros", "sync", "time"] }
tokio-stream = "0.1.19"
wiremock = "0.6.5"
```

`rust-toolchain.toml`:

```toml
[toolchain]
channel = "1.98.1"
components = ["rustfmt", "clippy"]
```

`crates/kalfa-evren/Cargo.toml`:

```toml
[package]
name = "kalfa-evren"
description = "EVREN LLM Gateway için Rust istemcisi"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true

[dependencies]
bytes.workspace = true
futures-util.workspace = true
reqwest.workspace = true
rust_decimal.workspace = true
rustls.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tokio.workspace = true
tokio-stream.workspace = true

[dev-dependencies]
wiremock.workspace = true
```

`crates/kalfa-evren/src/lib.rs`:

```rust
//! EVREN LLM Gateway için Rust istemcisi.
//!
//! Tasarım: `docs/superpowers/specs/2026-09-21-kalfa-design.md` §3 ve §5.

pub mod sse;
```

`crates/kalfa-evren/tests/ortak/mod.rs`:

```rust
#![allow(dead_code)]

use std::path::PathBuf;

/// `research/evren-probe/fixtures/` altındaki gerçek EVREN yanıtının yolu.
pub fn fixture_yolu(ad: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/evren-probe/fixtures")
        .join(ad)
}

pub fn fixture_bayt(ad: &str) -> Vec<u8> {
    std::fs::read(fixture_yolu(ad)).unwrap_or_else(|e| panic!("fixture {ad} okunamadı: {e}"))
}

pub fn fixture_metin(ad: &str) -> String {
    String::from_utf8(fixture_bayt(ad)).expect("fixture UTF-8 olmalı")
}

pub fn sentetik_bayt(ad: &str) -> Vec<u8> {
    let yol = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/sentetik").join(ad);
    std::fs::read(&yol).unwrap_or_else(|e| panic!("sentetik veri {ad} okunamadı: {e}"))
}
```

- [ ] **Step 3: Başarısız testleri yaz**

`crates/kalfa-evren/tests/sse.rs`:

```rust
mod ortak;

use kalfa_evren::sse::{SseCozucu, SseHatasi, SseOlayi, VARSAYILAN_AZAMI_OLAY};

fn tek_seferde(veri: &[u8]) -> Vec<SseOlayi> {
    let mut c = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    c.besle(veri).expect("sınır aşılmamalı")
}

fn parcali(veri: &[u8], boyut: usize) -> Vec<SseOlayi> {
    let mut c = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    let mut hepsi = Vec::new();
    for parca in veri.chunks(boyut) {
        hepsi.extend(c.besle(parca).expect("sınır aşılmamalı"));
    }
    hepsi
}

fn veriler(olaylar: &[SseOlayi]) -> Vec<&str> {
    olaylar.iter().map(|o| o.veri.as_str()).collect()
}

#[test]
fn basit_olay() {
    let o = tek_seferde(b"data: merhaba\n\n");
    assert_eq!(o, vec![SseOlayi { olay: None, veri: "merhaba".into(), kimlik: None }]);
}

#[test]
fn yorum_satirlari_atlanir() {
    let o = tek_seferde(b": keep-alive\n\n: keep-alive\n\ndata: x\n\n");
    assert_eq!(veriler(&o), vec!["x"]);
}

#[test]
fn crlf_cr_ve_lf_satir_sonlari() {
    let o = tek_seferde(b"data: a\r\n\r\ndata: b\r\rdata: c\n\n");
    assert_eq!(veriler(&o), vec!["a", "b", "c"]);
}

#[test]
fn cok_satirli_data_birlesir() {
    let o = tek_seferde(b"data: bir\ndata: iki\n\n");
    assert_eq!(veriler(&o), vec!["bir\niki"]);
}

#[test]
fn degerdeki_bastaki_tek_bosluk_atilir() {
    let o = tek_seferde(b"data:  iki bosluk\n\ndata:bosluksuz\n\n");
    assert_eq!(veriler(&o), vec![" iki bosluk", "bosluksuz"]);
}

#[test]
fn event_ve_id_alanlari() {
    let o = tek_seferde(b"event: ping\nid: 7\ndata: {}\n\n");
    assert_eq!(o, vec![SseOlayi { olay: Some("ping".into()), veri: "{}".into(), kimlik: Some("7".into()) }]);
}

#[test]
fn bos_data_olay_uretir_ama_hic_data_yoksa_olay_yok() {
    let o = tek_seferde(b"data:\n\nevent: yalniz\n\n");
    assert_eq!(veriler(&o), vec![""]);
}

#[test]
fn eksik_son_olay_atilir_ve_bitir_bildirir() {
    let mut c = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    assert!(c.besle(b"data: tamam\n\ndata: yarim").unwrap().len() == 1);
    assert!(c.bitir(), "tamamlanmamış olay kaldığı bildirilmeli");

    let mut c = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    c.besle(b"data: tamam\n\n").unwrap();
    assert!(!c.bitir());
}

#[test]
fn bom_atilir() {
    let o = tek_seferde("\u{FEFF}data: x\n\n".as_bytes());
    assert_eq!(veriler(&o), vec!["x"]);
}

#[test]
fn boyut_siniri_asilinca_hata() {
    let mut c = SseCozucu::yeni(16);
    let hata = c.besle(&[b'a'; 32]).unwrap_err();
    assert_eq!(hata, SseHatasi::OlayCokBuyuk { azami: 16 });

    let mut c = SseCozucu::yeni(16);
    let hata = c.besle(b"data: 0123456789\ndata: 0123456789\n").unwrap_err();
    assert_eq!(hata, SseHatasi::OlayCokBuyuk { azami: 16 });
}

#[test]
fn turkce_cok_bayti_karakter_her_bayt_sinirinda_bozulmaz() {
    let akis = "data: {\"metin\":\"İğüşçö ÇĞİÖŞÜ ı\"}\r\n\r\n: yorum\r\ndata: ikinci\r\n\r\n".as_bytes();
    let beklenen = tek_seferde(akis);
    assert_eq!(veriler(&beklenen), vec!["{\"metin\":\"İğüşçö ÇĞİÖŞÜ ı\"}", "ikinci"]);
    for boyut in 1..=akis.len() {
        assert_eq!(parcali(akis, boyut), beklenen, "parça boyutu {boyut}");
    }
    // CRLF'in CR ve LF'si ayrı beslemelerde gelse de tek satır sonu sayılır.
    for kesim in 0..=akis.len() {
        let mut c = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
        let mut o = c.besle(&akis[..kesim]).unwrap();
        o.extend(c.besle(&akis[kesim..]).unwrap());
        assert_eq!(o, beklenen, "kesim noktası {kesim}");
    }
}

fn fixture_dogrula(ad: &str) {
    let ham = ortak::fixture_bayt(ad);
    let metin = ortak::fixture_metin(ad);
    let beklenen_sayi = metin.lines().filter(|s| s.starts_with("data:")).count();

    let tum = tek_seferde(&ham);
    assert_eq!(tum.len(), beklenen_sayi, "{ad}: her data satırı bir olay olmalı");
    assert_eq!(tum.last().map(|o| o.veri.as_str()), Some("[DONE]"), "{ad}: son olay [DONE]");
    for boyut in [1, 2, 3, 7, 64, 1024] {
        assert_eq!(parcali(&ham, boyut), tum, "{ad}: parça boyutu {boyut}");
    }
}

#[test]
fn fixture_glm_akisi() {
    fixture_dogrula("chat_tool_calls_stream.sse");
}

#[test]
fn fixture_qwen_akisi() {
    fixture_dogrula("chat_tool_calls_stream_qwen_flash.sse");
}

#[test]
fn fixture_deepseek_akisi() {
    fixture_dogrula("chat_tool_calls_stream_deepseek_flash.sse");
}
```

- [ ] **Step 4: Testin başarısız olduğunu gör**

Run: `cargo test -p kalfa-evren --test sse`
Expected: derleme hatası — `could not find `SseCozucu` in `sse`` (modül boş).

- [ ] **Step 5: Çerçeveleyiciyi yaz**

`crates/kalfa-evren/src/sse.rs`:

```rust
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
                olaylar.push(SseOlayi { olay: self.olay.take(), veri, kimlik: self.kimlik.take() });
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
```

- [ ] **Step 6: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa-evren --test sse`
Expected: `test result: ok. 14 passed`

- [ ] **Step 7: Lint ve biçim**

Run: `cargo clippy -p kalfa-evren --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: uyarısız çıkış. `fmt` fark bildirirse `cargo fmt --all` çalıştır ve tekrar denetle.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml rust-toolchain.toml crates/kalfa-evren
git commit -m "kalfa-evren: workspace ve WHATWG uyumlu SSE çerçeveleyici"
```

---

### Task 2: EVREN JSON tipleri

**Files:**
- Create: `crates/kalfa-evren/src/tipler.rs`
- Modify: `crates/kalfa-evren/src/lib.rs`
- Test: `crates/kalfa-evren/tests/tipler.rs`

**Interfaces:**
- Consumes: —
- Produces (hepsi `kalfa_evren::tipler`):
  - `enum Role { System, User, Assistant, Tool }` (JSON: küçük harf)
  - `struct ChatMessage { role: Role, content: Option<String>, tool_calls: Option<Vec<ToolCall>>, tool_call_id: Option<String>, reasoning: Option<String> }` — `reasoning` yalnızca okunur, gönderilmez; kurucular `ChatMessage::system(s)`, `::user(s)`, `::assistant(content: Option<String>, tool_calls: Vec<ToolCall>)`, `::tool(tool_call_id, content)`
  - `struct ToolCall { id: String, kind: String, function: FunctionCall }`, `struct FunctionCall { name: String, arguments: String }`
  - `struct ToolDef { kind: String, function: FunctionDef }`, `ToolDef::function(name, description, parameters: serde_json::Value)`
  - `struct ChatRequest { model, messages, tools: Vec<ToolDef>, stream: bool, stream_options: Option<StreamOptions>, response_format: Option<Value> }`, `ChatRequest::new(model, messages)`; `struct StreamOptions { include_usage: bool }`
  - `struct ChatResponse { id, model, choices: Vec<Choice>, usage: Option<Usage> }`, `struct Choice { index: u32, message: ChatMessage, finish_reason: Option<String> }`
  - `struct Usage { prompt_tokens: u64, completion_tokens: u64, total_tokens: u64, completion_tokens_details: Option<CompletionTokensDetails>, evren: Option<UsageEvren> }`, `Usage::reasoning_tokens(&self) -> u64`
  - `struct UsageEvren { credits_held_cr: Decimal, credits_remaining_cr: Decimal, routed_model: Option<String> }`
  - `struct ChatChunk { id, model, choices: Vec<ChunkChoice>, usage: Option<Usage> }`, `struct ChunkChoice { index: u32, delta: Delta, finish_reason: Option<String> }`, `struct Delta { content, reasoning, tool_calls: Option<Vec<ToolCallDelta>> }`, `struct ToolCallDelta { index: u32, id: Option<String>, function: Option<FunctionDelta> }`, `struct FunctionDelta { name: Option<String>, arguments: Option<String> }`
  - `struct ModelList { data: Vec<ModelInfo> }`, `struct ModelInfo { id, task, modalities: Vec<String>, pricing: Option<Pricing> }`, `struct Pricing { prompt_token_price: Decimal, completion_token_price: Decimal, currency: String }`
  - `struct TermsStatus { current_version: u32, is_material: bool, accepted: bool, doc_path: Option<String> }`, `struct TermsText { version: u32, content: String, doc_path: Option<String> }`
  - `struct ApiErrorBody { error: ApiError }`, `struct ApiError { message: String, kind: Option<String>, code: Option<String>, evren: Option<ErrorEvren> }`, `struct ErrorEvren { required_terms_version: Option<u32>, suggested_models: Vec<String> }`

- [ ] **Step 1: Başarısız testleri yaz**

`crates/kalfa-evren/tests/tipler.rs`:

```rust
mod ortak;

use std::str::FromStr;

use kalfa_evren::tipler::*;
use rust_decimal::Decimal;
use serde_json::json;

fn yanit(ad: &str) -> ChatResponse {
    serde_json::from_slice(&ortak::fixture_bayt(ad)).unwrap_or_else(|e| panic!("{ad}: {e}"))
}

#[test]
fn glm_bloklu_arac_cagrisi_ve_evren_alanlari() {
    let y = yanit("chat_tool_calls_blocking.json");
    let m = &y.choices[0].message;
    assert_eq!(m.role, Role::Assistant);
    let cagrilar = m.tool_calls.as_ref().expect("araç çağrısı olmalı");
    assert_eq!(cagrilar.len(), 2, "paralel iki çağrı");
    assert_eq!(cagrilar[0].kind, "function");
    assert_eq!(cagrilar[0].function.name, "hava_durumu");
    let arg: serde_json::Value = serde_json::from_str(&cagrilar[0].function.arguments).unwrap();
    assert_eq!(arg["sehir"], "Ankara");
    assert!(m.reasoning.as_deref().is_some_and(|r| !r.is_empty()), "reasoning alanı");

    let u = y.usage.expect("usage");
    assert_eq!(u.reasoning_tokens(), 32);
    assert!(u.reasoning_tokens() <= u.completion_tokens, "reasoning completion'ın içinde");
    let e = u.evren.expect("usage.evren");
    assert_eq!(e.credits_remaining_cr, Decimal::from_str("1000").unwrap());
    assert_eq!(e.credits_held_cr, Decimal::ZERO);
    assert_eq!(e.routed_model.as_deref(), Some("zai/glm-5.3-fp8"));
}

#[test]
fn qwen_ve_deepseek_bloklu_paralel_arac_cagrisi() {
    for ad in ["chat_tool_calls_qwen_flash.json", "chat_tool_calls_deepseek_flash.json"] {
        let y = yanit(ad);
        let cagrilar = y.choices[0].message.tool_calls.clone().expect(ad);
        let sehirler: Vec<String> = cagrilar
            .iter()
            .map(|c| {
                let v: serde_json::Value = serde_json::from_str(&c.function.arguments).unwrap();
                v["sehir"].as_str().unwrap().to_string()
            })
            .collect();
        assert_eq!(sehirler, vec!["İstanbul", "Ankara"], "{ad}");
    }
}

#[test]
fn json_schema_yanitlari() {
    let y = yanit("chat_json_schema_qwen_flash.json");
    let icerik: serde_json::Value =
        serde_json::from_str(y.choices[0].message.content.as_deref().unwrap()).unwrap();
    assert_eq!(icerik["guvenli"], false);
    assert_eq!(icerik["kategori"], "talimat_enjeksiyonu");

    let y = yanit("chat_json_schema.json");
    let icerik: serde_json::Value =
        serde_json::from_str(y.choices[0].message.content.as_deref().unwrap()).unwrap();
    assert!(icerik.is_object());
}

#[test]
fn model_katalogu() {
    let k: ModelList = serde_json::from_slice(&ortak::fixture_bayt("models.json")).unwrap();
    assert_eq!(k.data.len(), 11);
    let glm = k.data.iter().find(|m| m.id == "glm-5.3").expect("glm-5.3");
    assert_eq!(glm.task, "chat");
    let p = glm.pricing.as_ref().expect("pricing");
    assert_eq!(p.currency, "CR");
    assert_eq!(p.prompt_token_price, Decimal::ZERO);
    let gomme = k.data.iter().find(|m| m.id == "qwen3-embedding-8b").unwrap();
    assert_eq!(gomme.task, "embedding");
}

#[test]
fn sart_durumu_ve_metni() {
    let d: TermsStatus = serde_json::from_slice(&ortak::fixture_bayt("terms_status.json")).unwrap();
    assert_eq!(d, TermsStatus { current_version: 1, is_material: true, accepted: true, doc_path: Some("docs/legal/LLM_TERMS_OF_SERVICE_v1.md".into()) });
    let m: TermsText = serde_json::from_slice(&ortak::fixture_bayt("terms_text.json")).unwrap();
    assert_eq!(m.version, 1);
    assert!(m.content.starts_with("# EVREN LLM Gateway"));
}

#[test]
fn hata_govdeleri() {
    let g: ApiErrorBody = serde_json::from_slice(&ortak::fixture_bayt("error_model_not_found.json")).unwrap();
    assert_eq!(g.error.code.as_deref(), Some("model_not_found"));
    assert_eq!(g.error.evren.unwrap().suggested_models, vec!["qwen3-guard-4b"]);

    let g: ApiErrorBody = serde_json::from_slice(&ortak::fixture_bayt("error_task_mismatch.json")).unwrap();
    assert_eq!(g.error.code.as_deref(), Some("bad_request"));
    assert!(g.error.evren.is_none(), "error.evren koşullu: burada yok");

    let g: ApiErrorBody = serde_json::from_slice(&ortak::fixture_bayt("error_unauthorized.json")).unwrap();
    assert_eq!(g.error.code.as_deref(), Some("unauthorized"));
    assert_eq!(g.error.kind.as_deref(), Some("authentication_error"));
}

#[test]
fn istek_serilestirme() {
    let mut r = ChatRequest::new("glm-5.3", vec![ChatMessage::system("s"), ChatMessage::user("u")]);
    let j = serde_json::to_value(&r).unwrap();
    assert_eq!(j["model"], "glm-5.3");
    assert_eq!(j["stream"], false);
    assert!(j.get("tools").is_none(), "boş araç listesi gönderilmez");
    assert!(j.get("stream_options").is_none());
    assert_eq!(j["messages"][1], json!({"role": "user", "content": "u"}));

    r.tools.push(ToolDef::function("hava_durumu", "Hava durumunu getirir", json!({"type": "object"})));
    r.stream = true;
    r.stream_options = Some(StreamOptions { include_usage: true });
    let j = serde_json::to_value(&r).unwrap();
    assert_eq!(j["tools"][0]["type"], "function");
    assert_eq!(j["tools"][0]["function"]["name"], "hava_durumu");
    assert_eq!(j["stream_options"]["include_usage"], true);
}

#[test]
fn mesaj_serilestirme_reasoning_gondermez() {
    let mut a = ChatMessage::assistant(
        Some("bakıyorum".into()),
        vec![ToolCall {
            id: "c1".into(),
            kind: "function".into(),
            function: FunctionCall { name: "dosya_oku".into(), arguments: "{\"yol\":\"a\"}".into() },
        }],
    );
    a.reasoning = Some("gizli düşünce".into());
    let j = serde_json::to_value(&a).unwrap();
    assert!(j.get("reasoning").is_none());
    assert_eq!(j["tool_calls"][0]["type"], "function");

    let t = serde_json::to_value(ChatMessage::tool("c1", "içerik")).unwrap();
    assert_eq!(t, json!({"role": "tool", "content": "içerik", "tool_call_id": "c1"}));

    let bos = serde_json::to_value(ChatMessage::assistant(Some("x".into()), vec![])).unwrap();
    assert!(bos.get("tool_calls").is_none());
}
```

- [ ] **Step 2: Testin başarısız olduğunu gör**

Run: `cargo test -p kalfa-evren --test tipler`
Expected: derleme hatası — `could not find `tipler` in `kalfa_evren``.

- [ ] **Step 3: Tipleri yaz**

`crates/kalfa-evren/src/lib.rs` içine `pub mod sse;` satırının altına ekle:

```rust
pub mod tipler;
```

`crates/kalfa-evren/src/tipler.rs`:

```rust
//! EVREN'in OpenAI uyumlu JSON şemasının Rust karşılıkları.
//!
//! Bilinmeyen alanlar yok sayılır. EVREN'e özgü alanlar — `reasoning`,
//! `usage.evren`, `error.evren`, katalog uzantıları — açıkça modellenir
//! (spec §3.3).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Modelin düşünce metni. Yalnızca yanıtta gelir; isteğe geri gönderilmez.
    #[serde(default, skip_serializing)]
    pub reasoning: Option<String>,
}

impl ChatMessage {
    fn bos(role: Role) -> Self {
        Self { role, content: None, tool_calls: None, tool_call_id: None, reasoning: None }
    }

    pub fn system(metin: impl Into<String>) -> Self {
        Self { content: Some(metin.into()), ..Self::bos(Role::System) }
    }

    pub fn user(metin: impl Into<String>) -> Self {
        Self { content: Some(metin.into()), ..Self::bos(Role::User) }
    }

    pub fn assistant(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            content,
            tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
            ..Self::bos(Role::Assistant)
        }
    }

    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            tool_call_id: Some(tool_call_id.into()),
            ..Self::bos(Role::Tool)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type", default = "function_turu")]
    pub kind: String,
    pub function: FunctionCall,
}

fn function_turu() -> String {
    "function".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDef {
    #[serde(rename = "type")]
    pub kind: String,
    pub function: FunctionDef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionDef {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

impl ToolDef {
    pub fn function(name: impl Into<String>, description: impl Into<String>, parameters: Value) -> Self {
        Self {
            kind: function_turu(),
            function: FunctionDef { name: name.into(), description: description.into(), parameters },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDef>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<Value>,
}

impl ChatRequest {
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            tools: Vec::new(),
            stream: false,
            stream_options: None,
            response_format: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StreamOptions {
    pub include_usage: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Choice {
    #[serde(default)]
    pub index: u32,
    pub message: ChatMessage,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub completion_tokens_details: Option<CompletionTokensDetails>,
    #[serde(default)]
    pub evren: Option<UsageEvren>,
}

impl Usage {
    /// `reasoning_tokens`, `completion_tokens`'ın dökümüdür; ona eklenmez (§5.2).
    pub fn reasoning_tokens(&self) -> u64 {
        self.completion_tokens_details.as_ref().map_or(0, |d| d.reasoning_tokens)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionTokensDetails {
    #[serde(default)]
    pub reasoning_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageEvren {
    pub credits_held_cr: Decimal,
    pub credits_remaining_cr: Decimal,
    #[serde(default)]
    pub routed_model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ChatChunk {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub choices: Vec<ChunkChoice>,
    #[serde(default)]
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ChunkChoice {
    #[serde(default)]
    pub index: u32,
    #[serde(default)]
    pub delta: Delta,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Delta {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ToolCallDelta {
    pub index: u32,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub function: Option<FunctionDelta>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FunctionDelta {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ModelList {
    pub data: Vec<ModelInfo>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub task: String,
    #[serde(default)]
    pub modalities: Vec<String>,
    #[serde(default)]
    pub pricing: Option<Pricing>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Pricing {
    pub prompt_token_price: Decimal,
    pub completion_token_price: Decimal,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TermsStatus {
    pub current_version: u32,
    pub is_material: bool,
    pub accepted: bool,
    #[serde(default)]
    pub doc_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TermsText {
    pub version: u32,
    pub content: String,
    #[serde(default)]
    pub doc_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ApiErrorBody {
    pub error: ApiError,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ApiError {
    #[serde(default)]
    pub message: String,
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub evren: Option<ErrorEvren>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ErrorEvren {
    #[serde(default)]
    pub required_terms_version: Option<u32>,
    #[serde(default)]
    pub suggested_models: Vec<String>,
}
```

- [ ] **Step 4: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa-evren --test tipler`
Expected: `test result: ok. 8 passed`

> `model_katalogu` testi `pricing` alanında (JSON sayısı `0.0`) başarısız olursa: `rust_decimal`'in sayıdan çözmesi kapalıdır. `Pricing`'in iki alanını `#[serde(with = "rust_decimal::serde::float")]` ile işaretle ve workspace'te `rust_decimal`'e `serde-with-float` özelliğini ekle.

- [ ] **Step 5: Lint, biçim, commit**

Run: `cargo clippy -p kalfa-evren --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: temiz.

```bash
git add crates/kalfa-evren
git commit -m "kalfa-evren: EVREN JSON tipleri, fixture'lara karşı test edildi"
```

---

### Task 3: Hata eşleme ve yanıt başlıkları

**Files:**
- Create: `crates/kalfa-evren/src/hata.rs`, `crates/kalfa-evren/src/kredi.rs`
- Create: `crates/kalfa-evren/tests/sentetik/README.md`, `crates/kalfa-evren/tests/sentetik/terms_not_accepted.json`
- Modify: `crates/kalfa-evren/src/lib.rs`
- Test: `crates/kalfa-evren/tests/hata_kredi.rs`

**Interfaces:**
- Consumes: `tipler::ApiErrorBody` (Task 2)
- Produces:
  - `kalfa_evren::EvrenError` (varyantlar: `AnahtarGecersiz`, `SartKabulGerekli { surum: Option<u32> }`, `ModelBulunamadi { mesaj: String, oneriler: Vec<String> }`, `HizSiniri { tekrar_dene: Option<Duration> }`, `AkisYarimKaldi`, `GecersizYanit(String)`, `Ag(String)`, `Sunucu { durum: u16, kod: Option<String>, mesaj: String }`) — `Clone + PartialEq + Eq`
  - `kalfa_evren::hata::hata_esle(durum: u16, retry_after: Option<&str>, govde: &[u8]) -> EvrenError`
  - `kalfa_evren::KrediGozlemi { kalan: Option<Decimal>, tutulan: Option<Decimal>, istek_id: Option<String>, zaman_ms: u64 }` — `Serialize + Deserialize + Clone + Eq`
  - `kalfa_evren::YanitMeta { istek_id: Option<String>, yonlendirilen_model: Option<String>, kredi: Option<KrediGozlemi> }` — `Default + Clone + Eq`
  - `kalfa_evren::kredi::basliklardan(basliklar: &reqwest::header::HeaderMap, zaman_ms: u64) -> YanitMeta`

> **Spec sapması (bilinçli):** spec §5.4'teki `GorevUyusmazligi` ve `KotaYetersiz` bu planda ayrı varyant değildir. Görev uyuşmazlığı EVREN'den `code: "bad_request"` genel koduyla geliyor (`error_task_mismatch.json`), kota yetersizliği için gözlenmiş bir yanıt yok. İkisi de `Sunucu { durum, kod, mesaj }` altında taşınır; görev uyuşmazlığı ayrıca kaynağında önlenir: arayüzün model seçicisi yalnızca `task: chat` modellerini sunar (plan 1c). Spec §5.4 buna göre güncellenmiştir.

- [ ] **Step 1: Sentetik veriyi ekle**

`crates/kalfa-evren/tests/sentetik/README.md`:

```markdown
# Sentetik test verisi

Bu dizindeki dosyalar **gerçek EVREN yanıtı değildir.** Davranışı tasarım
belgesinde (spec §3.1) belgelenen ama ham gövdesi kaydedilmemiş yanıtların
belgelenen şemaya göre yazılmış karşılıklarıdır. Gerçek yanıt yakalandığında
`research/evren-probe/fixtures/` altına taşınır ve buradaki kopya silinir.

| Dosya | Kaynak |
|---|---|
| `terms_not_accepted.json` | Spec §3.1: `403`, `code: "terms_not_accepted"`, `error.evren.required_terms_version` |
```

`crates/kalfa-evren/tests/sentetik/terms_not_accepted.json`:

```json
{"error":{"message":"EVREN kullanım şartları kabul edilmedi","type":"permission_error","param":null,"code":"terms_not_accepted","evren":{"required_terms_version":1}}}
```

- [ ] **Step 2: Başarısız testleri yaz**

`crates/kalfa-evren/tests/hata_kredi.rs`:

```rust
mod ortak;

use std::str::FromStr;
use std::time::Duration;

use kalfa_evren::hata::hata_esle;
use kalfa_evren::kredi::basliklardan;
use kalfa_evren::{EvrenError, KrediGozlemi, YanitMeta};
use reqwest::header::{HeaderMap, HeaderValue};
use rust_decimal::Decimal;

#[test]
fn gecersiz_anahtar() {
    let h = hata_esle(401, None, &ortak::fixture_bayt("error_unauthorized.json"));
    assert_eq!(h, EvrenError::AnahtarGecersiz);
    assert!(h.to_string().starts_with("Anahtar geçersiz ya da eksik."), "sunucu mesajı değil kendi metnimiz (§5.4)");
}

#[test]
fn sart_kabul_gerekli_sentetik() {
    let h = hata_esle(403, None, &ortak::sentetik_bayt("terms_not_accepted.json"));
    assert_eq!(h, EvrenError::SartKabulGerekli { surum: Some(1) });
}

#[test]
fn model_bulunamadi_onerilerle() {
    let h = hata_esle(404, None, &ortak::fixture_bayt("error_model_not_found.json"));
    assert_eq!(
        h,
        EvrenError::ModelBulunamadi { mesaj: "model bulunamadı: guard".into(), oneriler: vec!["qwen3-guard-4b".into()] }
    );
}

#[test]
fn gorev_uyusmazligi_genel_sunucu_hatasi_olarak_gelir() {
    match hata_esle(400, None, &ortak::fixture_bayt("error_task_mismatch.json")) {
        EvrenError::Sunucu { durum, kod, mesaj } => {
            assert_eq!(durum, 400);
            assert_eq!(kod.as_deref(), Some("bad_request"));
            assert!(mesaj.contains("görev: guard"));
        }
        diger => panic!("beklenmeyen: {diger:?}"),
    }
}

#[test]
fn hiz_siniri_retry_after_ile_ve_onsuz() {
    assert_eq!(hata_esle(429, Some("12"), b"{}"), EvrenError::HizSiniri { tekrar_dene: Some(Duration::from_secs(12)) });
    assert_eq!(hata_esle(429, None, b""), EvrenError::HizSiniri { tekrar_dene: None });
    assert_eq!(
        hata_esle(429, Some("Wed, 21 Oct 2026 07:28:00 GMT"), b""),
        EvrenError::HizSiniri { tekrar_dene: None },
        "tarih biçimli Retry-After süre olarak yorumlanmaz"
    );
}

#[test]
fn json_olmayan_govde_kirpilarak_tasinir() {
    assert_eq!(
        hata_esle(502, None, b"  Bad Gateway  "),
        EvrenError::Sunucu { durum: 502, kod: None, mesaj: "Bad Gateway".into() }
    );
    let uzun = vec![b'x'; 2000];
    match hata_esle(500, None, &uzun) {
        EvrenError::Sunucu { mesaj, .. } => assert_eq!(mesaj.chars().count(), 501, "500 karakter + üç nokta"),
        diger => panic!("{diger:?}"),
    }
}

#[test]
fn basliklardan_kredi_gozlemi() {
    let mut h = HeaderMap::new();
    h.insert("x-evren-credits-remaining", HeaderValue::from_static("999.5000"));
    h.insert("x-evren-credits-held", HeaderValue::from_static("0.5000"));
    h.insert("x-request-id", HeaderValue::from_static("f28c7cc1"));
    h.insert("x-evren-routed-model", HeaderValue::from_static("qwen/qwen3.8-flash-next"));
    let m = basliklardan(&h, 1_700_000_000_000);
    assert_eq!(
        m,
        YanitMeta {
            istek_id: Some("f28c7cc1".into()),
            yonlendirilen_model: Some("qwen/qwen3.8-flash-next".into()),
            kredi: Some(KrediGozlemi {
                kalan: Some(Decimal::from_str("999.5").unwrap()),
                tutulan: Some(Decimal::from_str("0.5").unwrap()),
                istek_id: Some("f28c7cc1".into()),
                zaman_ms: 1_700_000_000_000,
            }),
        }
    );
}

#[test]
fn kredi_basligi_yoksa_gozlem_yok_bozuksa_alan_bos() {
    let mut h = HeaderMap::new();
    h.insert("x-request-id", HeaderValue::from_static("r1"));
    let m = basliklardan(&h, 0);
    assert_eq!(m.kredi, None);
    assert_eq!(m.istek_id.as_deref(), Some("r1"));

    h.insert("x-evren-credits-remaining", HeaderValue::from_static("bozuk"));
    h.insert("x-evren-credits-held", HeaderValue::from_static("1.0000"));
    let k = basliklardan(&h, 0).kredi.expect("tutulan var");
    assert_eq!(k.kalan, None);
    assert_eq!(k.tutulan, Some(Decimal::ONE));
}

#[test]
fn kredi_gozlemi_serilestirilebilir() {
    let k = KrediGozlemi { kalan: Some(Decimal::from_str("1000.0000").unwrap()), tutulan: None, istek_id: None, zaman_ms: 5 };
    let geri: KrediGozlemi = serde_json::from_str(&serde_json::to_string(&k).unwrap()).unwrap();
    assert_eq!(geri, k);
}
```

- [ ] **Step 3: Testin başarısız olduğunu gör**

Run: `cargo test -p kalfa-evren --test hata_kredi`
Expected: derleme hatası — `unresolved import `kalfa_evren::hata``.

- [ ] **Step 4: Hata ve kredi modüllerini yaz**

`crates/kalfa-evren/src/lib.rs` son hâli:

```rust
//! EVREN LLM Gateway için Rust istemcisi.
//!
//! Tasarım: `docs/superpowers/specs/2026-09-21-kalfa-design.md` §3 ve §5.

pub mod hata;
pub mod kredi;
pub mod sse;
pub mod tipler;

pub use hata::EvrenError;
pub use kredi::{KrediGozlemi, YanitMeta};
```

`crates/kalfa-evren/src/hata.rs`:

```rust
//! EVREN hata yanıtlarının tipli karşılığı (spec §5.4).

use std::time::Duration;

use crate::tipler::ApiErrorBody;

const AZAMI_MESAJ: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EvrenError {
    #[error("Anahtar geçersiz ya da eksik. EVREN portalından anahtarını kontrol edip yeniden gir.")]
    AnahtarGecersiz,
    #[error("EVREN kullanım şartlarının kabul edilmesi gerekiyor.")]
    SartKabulGerekli { surum: Option<u32> },
    #[error("Model bulunamadı: {mesaj}")]
    ModelBulunamadi { mesaj: String, oneriler: Vec<String> },
    #[error("EVREN istek sınırına ulaşıldı; biraz sonra yeniden denenecek.")]
    HizSiniri { tekrar_dene: Option<Duration> },
    #[error("Yanıt akışı yarıda kaldı; model yanıtı tamamlanmadı.")]
    AkisYarimKaldi,
    #[error("EVREN'den beklenmeyen yanıt: {0}")]
    GecersizYanit(String),
    #[error("EVREN'e ulaşılamadı: {0}")]
    Ag(String),
    #[error("EVREN hatası ({durum}): {mesaj}")]
    Sunucu { durum: u16, kod: Option<String>, mesaj: String },
}

/// HTTP durum kodu, `Retry-After` başlığı ve gövdeden tipli hata üretir.
pub fn hata_esle(durum: u16, retry_after: Option<&str>, govde: &[u8]) -> EvrenError {
    let (kod, mesaj, evren) = match serde_json::from_slice::<ApiErrorBody>(govde) {
        Ok(g) => (g.error.code, g.error.message, g.error.evren),
        Err(_) => (None, kirp(&String::from_utf8_lossy(govde)), None),
    };

    if durum == 401 || kod.as_deref() == Some("unauthorized") {
        return EvrenError::AnahtarGecersiz;
    }
    if kod.as_deref() == Some("terms_not_accepted") {
        return EvrenError::SartKabulGerekli { surum: evren.and_then(|e| e.required_terms_version) };
    }
    if kod.as_deref() == Some("model_not_found") {
        return EvrenError::ModelBulunamadi {
            mesaj,
            oneriler: evren.map(|e| e.suggested_models).unwrap_or_default(),
        };
    }
    if durum == 429 {
        let tekrar_dene = retry_after.and_then(|v| v.trim().parse::<u64>().ok()).map(Duration::from_secs);
        return EvrenError::HizSiniri { tekrar_dene };
    }
    EvrenError::Sunucu { durum, kod, mesaj }
}

fn kirp(metin: &str) -> String {
    let t = metin.trim();
    if t.chars().count() <= AZAMI_MESAJ {
        t.to_string()
    } else {
        let mut k: String = t.chars().take(AZAMI_MESAJ).collect();
        k.push('…');
        k
    }
}
```

`crates/kalfa-evren/src/kredi.rs`:

```rust
//! Yanıt başlıklarından kredi gözlemi (spec §3.4, §5.2).
//!
//! Kalfa kredi hakkında yalnızca **gözlem** bildirir: kesinleşmiş harcama
//! API'den gözlenemiyor.

use std::str::FromStr;

use reqwest::header::HeaderMap;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub const KALAN: &str = "x-evren-credits-remaining";
pub const TUTULAN: &str = "x-evren-credits-held";
pub const YONLENDIRILEN_MODEL: &str = "x-evren-routed-model";
pub const ISTEK_KIMLIGI: &str = "x-request-id";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KrediGozlemi {
    pub kalan: Option<Decimal>,
    pub tutulan: Option<Decimal>,
    pub istek_id: Option<String>,
    /// Gözlem zamanı, Unix epoch milisaniyesi.
    pub zaman_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct YanitMeta {
    pub istek_id: Option<String>,
    pub yonlendirilen_model: Option<String>,
    /// Kredi başlıklarından hiçbiri yoksa `None`.
    pub kredi: Option<KrediGozlemi>,
}

pub fn basliklardan(basliklar: &HeaderMap, zaman_ms: u64) -> YanitMeta {
    let metin = |ad: &str| {
        basliklar
            .get(ad)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };
    let ondalik = |ad: &str| metin(ad).and_then(|s| Decimal::from_str(&s).ok());

    let istek_id = metin(ISTEK_KIMLIGI);
    let kalan = ondalik(KALAN);
    let tutulan = ondalik(TUTULAN);
    let kredi = (kalan.is_some() || tutulan.is_some()).then(|| KrediGozlemi {
        kalan,
        tutulan,
        istek_id: istek_id.clone(),
        zaman_ms,
    });
    YanitMeta { istek_id, yonlendirilen_model: metin(YONLENDIRILEN_MODEL), kredi }
}
```

- [ ] **Step 5: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa-evren --test hata_kredi`
Expected: `test result: ok. 9 passed`

- [ ] **Step 6: Lint, biçim, commit**

Run: `cargo clippy -p kalfa-evren --all-targets -- -D warnings && cargo fmt --all -- --check`

```bash
git add crates/kalfa-evren
git commit -m "kalfa-evren: tipli hata eşlemesi ve kredi başlıklarından gözlem"
```

---

### Task 4: Akış birleştirici

**Files:**
- Create: `crates/kalfa-evren/src/akis.rs`
- Modify: `crates/kalfa-evren/src/lib.rs`
- Test: `crates/kalfa-evren/tests/akis.rs`

**Interfaces:**
- Consumes: `tipler::{ChatChunk, ToolCall, FunctionCall, Usage}` (Task 2), `EvrenError` (Task 3), `sse::SseCozucu` (Task 1, testlerde)
- Produces:
  - `kalfa_evren::AkisOlayi { Metin(String), Dusunce(String), AracCagrilari(Vec<ToolCall>), Kullanim(Usage), Bitti { neden: String } }` — `Clone + PartialEq`
  - `kalfa_evren::akis::VeriSonucu { Olaylar(Vec<AkisOlayi>), Bitti }`
  - `kalfa_evren::AkisBirlestirici::yeni() -> Self`
  - `AkisBirlestirici::veri_isle(&mut self, veri: &str) -> Result<VeriSonucu, EvrenError>` (`[DONE]` → `VeriSonucu::Bitti`)
  - `AkisBirlestirici::ekle(&mut self, parca: ChatChunk) -> Result<Vec<AkisOlayi>, EvrenError>`
  - `AkisBirlestirici::bitir(&mut self) -> Result<Vec<AkisOlayi>, EvrenError>` (`finish_reason` görülmediyse `Err(AkisYarimKaldi)`, görüldüyse `[Bitti]`; ikinci çağrı boş döner)
- Davranış sözleşmesi: araç çağrıları `index` bazında biriktirilir ve **yalnızca** `finish_reason` gelince tek bir `AracCagrilari` olayıyla yayılır; boş `content`/`reasoning` parçaları olay üretmez; yalnızca `index == 0` seçimi işlenir.

- [ ] **Step 1: Başarısız testleri yaz**

`crates/kalfa-evren/tests/akis.rs`:

```rust
mod ortak;

use kalfa_evren::akis::VeriSonucu;
use kalfa_evren::sse::{SseCozucu, VARSAYILAN_AZAMI_OLAY};
use kalfa_evren::tipler::ChatChunk;
use kalfa_evren::{AkisBirlestirici, AkisOlayi, EvrenError};

/// Fixture'ı SSE → birleştirici hattından geçirir; `[DONE]` gelince bitirir.
fn hattan_gecir(ham: &[u8]) -> Result<Vec<AkisOlayi>, EvrenError> {
    let mut sse = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    let mut b = AkisBirlestirici::yeni();
    let mut olaylar = Vec::new();
    for o in sse.besle(ham).unwrap() {
        match b.veri_isle(&o.veri)? {
            VeriSonucu::Olaylar(v) => olaylar.extend(v),
            VeriSonucu::Bitti => {
                olaylar.extend(b.bitir()?);
                return Ok(olaylar);
            }
        }
    }
    olaylar.extend(b.bitir()?);
    Ok(olaylar)
}

fn arac_cagrilari(olaylar: &[AkisOlayi]) -> Vec<(String, String)> {
    olaylar
        .iter()
        .filter_map(|o| match o {
            AkisOlayi::AracCagrilari(v) => Some(v.clone()),
            _ => None,
        })
        .flatten()
        .map(|c| (c.function.name, c.function.arguments))
        .collect()
}

fn dusunce(olaylar: &[AkisOlayi]) -> String {
    olaylar.iter().filter_map(|o| if let AkisOlayi::Dusunce(s) = o { Some(s.as_str()) } else { None }).collect()
}

#[test]
fn glm_akisi_istanbul_ascii() {
    let o = hattan_gecir(&ortak::fixture_bayt("chat_tool_calls_stream.sse")).unwrap();
    assert_eq!(arac_cagrilari(&o), vec![("hava_durumu".to_string(), "{\"sehir\": \"Istanbul\"}".to_string())]);
    assert!(!dusunce(&o).is_empty(), "delta.reasoning ayrı akış olarak gelmeli");
    let kullanim = o.iter().find_map(|x| if let AkisOlayi::Kullanim(u) = x { Some(u.clone()) } else { None }).unwrap();
    assert_eq!(kullanim.reasoning_tokens(), 51);
    assert!(kullanim.evren.is_none(), "usage.evren akışta gelmez (§3.3)");
    assert_eq!(o.last(), Some(&AkisOlayi::Bitti { neden: "tool_calls".into() }));
    let arac_sirasi = o.iter().position(|x| matches!(x, AkisOlayi::AracCagrilari(_))).unwrap();
    let kullanim_sirasi = o.iter().position(|x| matches!(x, AkisOlayi::Kullanim(_))).unwrap();
    assert!(arac_sirasi < kullanim_sirasi, "araç çağrıları finish_reason ile, usage'dan önce");
}

#[test]
fn qwen_akisi_turkce_karakter_token_sinirinda() {
    let o = hattan_gecir(&ortak::fixture_bayt("chat_tool_calls_stream_qwen_flash.sse")).unwrap();
    assert_eq!(arac_cagrilari(&o), vec![("hava_durumu".to_string(), "{\"sehir\": \"İzmir\"}".to_string())]);
    assert_eq!(o.last(), Some(&AkisOlayi::Bitti { neden: "tool_calls".into() }));
}

#[test]
fn deepseek_akisi() {
    let o = hattan_gecir(&ortak::fixture_bayt("chat_tool_calls_stream_deepseek_flash.sse")).unwrap();
    assert_eq!(arac_cagrilari(&o), vec![("hava_durumu".to_string(), "{\"sehir\": \"İzmir\"}".to_string())]);
}

#[test]
fn yarim_kalan_akis_hata_verir_ve_arac_cagrisi_yaymaz() {
    let metin = ortak::fixture_metin("chat_tool_calls_stream.sse");
    let bitis = metin.find("\"finish_reason\": \"tool_calls\"").expect("fixture'da finish satırı");
    let kes = metin[..bitis].rfind("data:").unwrap();
    let mut sse = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    let mut b = AkisBirlestirici::yeni();
    let mut olaylar = Vec::new();
    for o in sse.besle(&metin.as_bytes()[..kes]).unwrap() {
        if let VeriSonucu::Olaylar(v) = b.veri_isle(&o.veri).unwrap() {
            olaylar.extend(v);
        }
    }
    assert!(!olaylar.iter().any(|o| matches!(o, AkisOlayi::AracCagrilari(_))), "tamamlanmamış çağrı yayılmaz");
    assert_eq!(b.bitir(), Err(EvrenError::AkisYarimKaldi));
}

fn parca(json: &str) -> ChatChunk {
    serde_json::from_str(json).unwrap()
}

#[test]
fn yalnizca_metin_ve_bos_parcalar() {
    let mut b = AkisBirlestirici::yeni();
    let mut o = Vec::new();
    o.extend(b.ekle(parca(r#"{"choices":[{"index":0,"delta":{"role":"assistant","content":""}}]}"#)).unwrap());
    o.extend(b.ekle(parca(r#"{"choices":[{"index":0,"delta":{"content":"Mer"}}]}"#)).unwrap());
    o.extend(b.ekle(parca(r#"{"choices":[{"index":0,"delta":{"content":"haba"},"finish_reason":"stop"}]}"#)).unwrap());
    o.extend(b.bitir().unwrap());
    assert_eq!(
        o,
        vec![AkisOlayi::Metin("Mer".into()), AkisOlayi::Metin("haba".into()), AkisOlayi::Bitti { neden: "stop".into() }]
    );
    assert_eq!(b.bitir().unwrap(), vec![], "ikinci bitir boş döner");
}

#[test]
fn adi_hic_gelmeyen_arac_cagrisi_gecersiz_yanittir() {
    let mut b = AkisBirlestirici::yeni();
    b.ekle(parca(r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"c1","function":{"arguments":"{}"}}]}}]}"#))
        .unwrap();
    let hata = b.ekle(parca(r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#)).unwrap_err();
    assert!(matches!(hata, EvrenError::GecersizYanit(_)));
}

#[test]
fn paralel_iki_cagri_index_sirasiyla() {
    let mut b = AkisBirlestirici::yeni();
    b.ekle(parca(r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":1,"id":"b","function":{"name":"y","arguments":"{\"a\":"}}]}}]}"#)).unwrap();
    b.ekle(parca(r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"a","function":{"name":"x","arguments":"{}"}}]}}]}"#)).unwrap();
    b.ekle(parca(r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":1,"function":{"arguments":"1}"}}]}}]}"#)).unwrap();
    let o = b.ekle(parca(r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#)).unwrap();
    assert_eq!(arac_cagrilari(&o), vec![("x".into(), "{}".into()), ("y".into(), "{\"a\":1}".into())]);
}

#[test]
fn bozuk_parca_gecersiz_yanit() {
    let mut b = AkisBirlestirici::yeni();
    assert!(matches!(b.veri_isle("{bozuk"), Err(EvrenError::GecersizYanit(_))));
    assert_eq!(b.veri_isle(" [DONE] ").unwrap(), VeriSonucu::Bitti);
}
```

- [ ] **Step 2: Testin başarısız olduğunu gör**

Run: `cargo test -p kalfa-evren --test akis`
Expected: derleme hatası — `unresolved import `kalfa_evren::akis``.

- [ ] **Step 3: Birleştiriciyi yaz**

`crates/kalfa-evren/src/lib.rs` içine modül ve dışa aktarım ekle (son hâli):

```rust
//! EVREN LLM Gateway için Rust istemcisi.
//!
//! Tasarım: `docs/superpowers/specs/2026-09-21-kalfa-design.md` §3 ve §5.

pub mod akis;
pub mod hata;
pub mod kredi;
pub mod sse;
pub mod tipler;

pub use akis::{AkisBirlestirici, AkisOlayi};
pub use hata::EvrenError;
pub use kredi::{KrediGozlemi, YanitMeta};
```

`crates/kalfa-evren/src/akis.rs`:

```rust
//! Akış parçalarını (`ChatChunk`) Kalfa'nın kullandığı olaylara çevirir.
//!
//! Araç çağrıları `index` bazında biriktirilir ve yalnızca `finish_reason`
//! geldiğinde, tamamlanmış olarak yayılır: tamamlanmamış bir çağrı asla
//! çalıştırılmaz (spec §5.1).

use std::collections::BTreeMap;

use crate::hata::EvrenError;
use crate::tipler::{ChatChunk, FunctionCall, ToolCall, Usage};

#[derive(Debug, Clone, PartialEq)]
pub enum AkisOlayi {
    Metin(String),
    Dusunce(String),
    AracCagrilari(Vec<ToolCall>),
    Kullanim(Usage),
    Bitti { neden: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum VeriSonucu {
    Olaylar(Vec<AkisOlayi>),
    Bitti,
}

#[derive(Debug, Default)]
struct KismiCagri {
    id: Option<String>,
    ad: Option<String>,
    argumanlar: String,
}

#[derive(Debug, Default)]
pub struct AkisBirlestirici {
    cagrilar: BTreeMap<u32, KismiCagri>,
    neden: Option<String>,
    bitirildi: bool,
}

impl AkisBirlestirici {
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir SSE olayının `data` alanını işler.
    pub fn veri_isle(&mut self, veri: &str) -> Result<VeriSonucu, EvrenError> {
        if veri.trim() == "[DONE]" {
            return Ok(VeriSonucu::Bitti);
        }
        let parca: ChatChunk = serde_json::from_str(veri)
            .map_err(|e| EvrenError::GecersizYanit(format!("akış parçası çözülemedi: {e}")))?;
        Ok(VeriSonucu::Olaylar(self.ekle(parca)?))
    }

    pub fn ekle(&mut self, parca: ChatChunk) -> Result<Vec<AkisOlayi>, EvrenError> {
        let mut olaylar = Vec::new();
        for secim in parca.choices.into_iter().filter(|s| s.index == 0) {
            let delta = secim.delta;
            if let Some(r) = delta.reasoning.filter(|s| !s.is_empty()) {
                olaylar.push(AkisOlayi::Dusunce(r));
            }
            if let Some(m) = delta.content.filter(|s| !s.is_empty()) {
                olaylar.push(AkisOlayi::Metin(m));
            }
            for tc in delta.tool_calls.unwrap_or_default() {
                let k = self.cagrilar.entry(tc.index).or_default();
                if let Some(id) = tc.id.filter(|s| !s.is_empty()) {
                    k.id = Some(id);
                }
                if let Some(f) = tc.function {
                    if let Some(ad) = f.name.filter(|s| !s.is_empty()) {
                        k.ad = Some(ad);
                    }
                    if let Some(a) = f.arguments {
                        k.argumanlar.push_str(&a);
                    }
                }
            }
            if let Some(neden) = secim.finish_reason
                && self.neden.is_none()
            {
                let cagrilar = self.tamamla()?;
                if !cagrilar.is_empty() {
                    olaylar.push(AkisOlayi::AracCagrilari(cagrilar));
                }
                self.neden = Some(neden);
            }
        }
        if let Some(u) = parca.usage {
            olaylar.push(AkisOlayi::Kullanim(u));
        }
        Ok(olaylar)
    }

    /// Akış bitti (`[DONE]` ya da bağlantı kapandı).
    pub fn bitir(&mut self) -> Result<Vec<AkisOlayi>, EvrenError> {
        if self.bitirildi {
            return Ok(Vec::new());
        }
        self.bitirildi = true;
        match self.neden.clone() {
            Some(neden) => Ok(vec![AkisOlayi::Bitti { neden }]),
            None => Err(EvrenError::AkisYarimKaldi),
        }
    }

    fn tamamla(&mut self) -> Result<Vec<ToolCall>, EvrenError> {
        std::mem::take(&mut self.cagrilar)
            .into_iter()
            .map(|(index, k)| {
                let id = k
                    .id
                    .ok_or_else(|| EvrenError::GecersizYanit(format!("{index}. araç çağrısının kimliği yok")))?;
                let name = k
                    .ad
                    .ok_or_else(|| EvrenError::GecersizYanit(format!("{index}. araç çağrısının adı yok")))?;
                Ok(ToolCall {
                    id,
                    kind: "function".to_string(),
                    function: FunctionCall { name, arguments: k.argumanlar },
                })
            })
            .collect()
    }
}
```

(`if let ... && ...` zincirleri edition 2024'te kararlıdır.)

- [ ] **Step 4: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa-evren --test akis`
Expected: `test result: ok. 8 passed`

- [ ] **Step 5: Lint, biçim, commit**

Run: `cargo clippy -p kalfa-evren --all-targets -- -D warnings && cargo fmt --all -- --check`

```bash
git add crates/kalfa-evren
git commit -m "kalfa-evren: akış birleştirici; araç çağrısı yalnızca tamamlanınca yayılır"
```

---

### Task 5: HTTP istemcisi — şart, katalog, bloklu sohbet

**Files:**
- Create: `crates/kalfa-evren/src/istemci.rs`
- Modify: `crates/kalfa-evren/src/lib.rs`
- Test: `crates/kalfa-evren/tests/istemci.rs`

**Interfaces:**
- Consumes: `tipler::*` (Task 2), `hata::hata_esle`, `EvrenError` (Task 3), `kredi::basliklardan`, `YanitMeta` (Task 3)
- Produces:
  - `kalfa_evren::VARSAYILAN_TABAN: &str` = `"https://evren-llmapi.ssyz.org.tr/v1"`
  - `kalfa_evren::ApiAnahtari::yeni(deger: impl Into<String>) -> ApiAnahtari` — `Debug` çıktısı `ApiAnahtari(***)`
  - `kalfa_evren::EvrenIstemcisi` (`Clone`, `Debug` anahtarsız):
    - `EvrenIstemcisi::yeni(taban: impl Into<String>, anahtar: ApiAnahtari) -> Result<Self, EvrenError>`
    - `async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError>` — `GET {taban}/terms/status`
    - `async fn sart_metni(&self) -> Result<TermsText, EvrenError>` — `GET {taban}/terms/text`
    - `async fn sart_kabul(&self, surum: u32) -> Result<(), EvrenError>` — `POST {taban}/terms/accept` gövde `{"version": surum}`
    - `async fn modeller(&self) -> Result<Vec<ModelInfo>, EvrenError>` — `GET {taban}/models`
    - `async fn sohbet(&self, istek: &ChatRequest) -> Result<(ChatResponse, YanitMeta), EvrenError>` — `POST {taban}/chat/completions`, `stream: false` zorlanır
  - Kimlik doğrulama `Authorization: Bearer <anahtar>` ile yapılır (probe'larda doğrulanan biçim).

- [ ] **Step 1: Başarısız testleri yaz**

`crates/kalfa-evren/tests/istemci.rs`:

```rust
mod ortak;

use std::str::FromStr;

use kalfa_evren::tipler::{ChatMessage, ChatRequest};
use kalfa_evren::{ApiAnahtari, EvrenError, EvrenIstemcisi};
use rust_decimal::Decimal;
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ANAHTAR: &str = "evren_llm_test_anahtari";

async fn sunucu_ve_istemci() -> (MockServer, EvrenIstemcisi) {
    let sunucu = MockServer::start().await;
    let istemci = EvrenIstemcisi::yeni(format!("{}/v1", sunucu.uri()), ApiAnahtari::yeni(ANAHTAR)).unwrap();
    (sunucu, istemci)
}

fn json_yanit(durum: u16, govde: Vec<u8>) -> ResponseTemplate {
    ResponseTemplate::new(durum).set_body_raw(govde, "application/json")
}

#[tokio::test]
async fn sart_durumu_bearer_ile_ister() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("GET"))
        .and(path("/v1/terms/status"))
        .and(header("authorization", format!("Bearer {ANAHTAR}").as_str()))
        .respond_with(json_yanit(200, ortak::fixture_bayt("terms_status.json")))
        .expect(1)
        .mount(&s)
        .await;
    let d = i.sart_durumu().await.unwrap();
    assert!(d.accepted);
    assert_eq!(d.current_version, 1);
}

#[tokio::test]
async fn sart_metni_ve_kabul() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("GET"))
        .and(path("/v1/terms/text"))
        .respond_with(json_yanit(200, ortak::fixture_bayt("terms_text.json")))
        .mount(&s)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/terms/accept"))
        .and(body_json(json!({"version": 1})))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{\"accepted\":true}", "application/json"))
        .expect(1)
        .mount(&s)
        .await;
    assert_eq!(i.sart_metni().await.unwrap().version, 1);
    i.sart_kabul(1).await.unwrap();
}

#[tokio::test]
async fn modeller() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(json_yanit(200, ortak::fixture_bayt("models.json")))
        .mount(&s)
        .await;
    let m = i.modeller().await.unwrap();
    assert_eq!(m.len(), 11);
    assert!(m.iter().any(|x| x.id == "qwen3.8-flash-next" && x.task == "chat"));
}

#[tokio::test]
async fn bloklu_sohbet_yanit_ve_meta() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            json_yanit(200, ortak::fixture_bayt("chat_tool_calls_blocking.json"))
                .insert_header("x-evren-credits-remaining", "1000.0000")
                .insert_header("x-evren-credits-held", "0.0000")
                .insert_header("x-request-id", "istek-1")
                .insert_header("x-evren-routed-model", "zai/glm-5.3-fp8"),
        )
        .mount(&s)
        .await;
    let mut istek = ChatRequest::new("glm-5.3", vec![ChatMessage::user("hava?")]);
    istek.stream = true; // sohbet() bunu false'a çekmeli
    let (y, meta) = i.sohbet(&istek).await.unwrap();
    assert_eq!(y.choices[0].message.tool_calls.as_ref().unwrap().len(), 2);
    assert_eq!(meta.istek_id.as_deref(), Some("istek-1"));
    assert_eq!(meta.yonlendirilen_model.as_deref(), Some("zai/glm-5.3-fp8"));
    assert_eq!(meta.kredi.unwrap().kalan, Some(Decimal::from_str("1000").unwrap()));

    let gonderilen: serde_json::Value =
        serde_json::from_slice(&s.received_requests().await.unwrap()[0].body).unwrap();
    assert_eq!(gonderilen["stream"], false);
    assert!(gonderilen.get("stream_options").is_none());
}

#[tokio::test]
async fn hata_durumlari_tipli_doner() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("GET"))
        .and(path("/v1/terms/status"))
        .respond_with(json_yanit(401, ortak::fixture_bayt("error_unauthorized.json")))
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(json_yanit(403, ortak::sentetik_bayt("terms_not_accepted.json")))
        .mount(&s)
        .await;
    assert_eq!(i.sart_durumu().await.unwrap_err(), EvrenError::AnahtarGecersiz);
    assert_eq!(i.modeller().await.unwrap_err(), EvrenError::SartKabulGerekli { surum: Some(1) });
}

#[tokio::test]
async fn ulasilamayan_sunucu_ag_hatasi() {
    let i = EvrenIstemcisi::yeni("http://127.0.0.1:1/v1", ApiAnahtari::yeni(ANAHTAR)).unwrap();
    assert!(matches!(i.sart_durumu().await, Err(EvrenError::Ag(_))));
}

#[test]
fn anahtar_hicbir_ciktida_gorunmez() {
    let a = ApiAnahtari::yeni(ANAHTAR);
    assert_eq!(format!("{a:?}"), "ApiAnahtari(***)");
    let i = EvrenIstemcisi::yeni("https://ornek/v1/", a).unwrap();
    assert!(!format!("{i:?}").contains(ANAHTAR));
}
```

- [ ] **Step 2: Testin başarısız olduğunu gör**

Run: `cargo test -p kalfa-evren --test istemci`
Expected: derleme hatası — `unresolved imports `kalfa_evren::ApiAnahtari`, `kalfa_evren::EvrenIstemcisi``.

- [ ] **Step 3: İstemciyi yaz**

`crates/kalfa-evren/src/lib.rs` içine `pub mod istemci;` ekle (modüller alfabetik: `akis, hata, istemci, kredi, sse, tipler`) ve dışa aktarımlara şunu ekle:

```rust
pub use istemci::{ApiAnahtari, EvrenIstemcisi, VARSAYILAN_TABAN};
```

`crates/kalfa-evren/src/istemci.rs`:

```rust
//! EVREN LLM Gateway HTTP istemcisi (spec §3, §5).

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::RETRY_AFTER;
use serde::de::DeserializeOwned;

use crate::hata::{EvrenError, hata_esle};
use crate::kredi::{YanitMeta, basliklardan};
use crate::tipler::{ChatRequest, ChatResponse, ModelInfo, ModelList, TermsStatus, TermsText};

pub const VARSAYILAN_TABAN: &str = "https://evren-llmapi.ssyz.org.tr/v1";

/// API anahtarı. Hiçbir `Debug`/`Display` çıktısında görünmez.
pub struct ApiAnahtari(String);

impl ApiAnahtari {
    pub fn yeni(deger: impl Into<String>) -> Self {
        Self(deger.into())
    }

    fn deger(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiAnahtari {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiAnahtari(***)")
    }
}

#[derive(Clone)]
pub struct EvrenIstemcisi {
    pub(crate) taban: String,
    pub(crate) anahtar: Arc<ApiAnahtari>,
    pub(crate) http: reqwest::Client,
}

impl fmt::Debug for EvrenIstemcisi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EvrenIstemcisi").field("taban", &self.taban).finish_non_exhaustive()
    }
}

impl EvrenIstemcisi {
    pub fn yeni(taban: impl Into<String>, anahtar: ApiAnahtari) -> Result<Self, EvrenError> {
        // reqwest `rustls-no-provider` ile derlendi; süreç genelinde ring'i kur.
        // Zaten kuruluysa hata döner; yok sayılır.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(180))
            .user_agent(concat!("kalfa-evren/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| EvrenError::Ag(e.to_string()))?;
        Ok(Self {
            taban: taban.into().trim_end_matches('/').to_string(),
            anahtar: Arc::new(anahtar),
            http,
        })
    }

    pub(crate) fn url(&self, yol: &str) -> String {
        format!("{}/{}", self.taban, yol)
    }

    /// İsteği kimlikle gönderir; başarısız durum kodunu tipli hataya çevirir.
    pub(crate) async fn gonder(
        &self,
        istek: reqwest::RequestBuilder,
    ) -> Result<(reqwest::Response, YanitMeta), EvrenError> {
        let yanit = istek.bearer_auth(self.anahtar.deger()).send().await.map_err(ag_hatasi)?;
        let meta = basliklardan(yanit.headers(), simdi_ms());
        if yanit.status().is_success() {
            return Ok((yanit, meta));
        }
        let durum = yanit.status().as_u16();
        let retry_after = yanit.headers().get(RETRY_AFTER).and_then(|v| v.to_str().ok()).map(str::to_owned);
        let govde = yanit.bytes().await.map_err(ag_hatasi)?;
        Err(hata_esle(durum, retry_after.as_deref(), &govde))
    }

    async fn json_al<T: DeserializeOwned>(&self, yol: &str) -> Result<T, EvrenError> {
        let (yanit, _) = self.gonder(self.http.get(self.url(yol))).await?;
        json_coz(yanit).await
    }

    pub async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError> {
        self.json_al("terms/status").await
    }

    pub async fn sart_metni(&self) -> Result<TermsText, EvrenError> {
        self.json_al("terms/text").await
    }

    pub async fn sart_kabul(&self, surum: u32) -> Result<(), EvrenError> {
        let istek = self.http.post(self.url("terms/accept")).json(&serde_json::json!({ "version": surum }));
        self.gonder(istek).await.map(|_| ())
    }

    pub async fn modeller(&self) -> Result<Vec<ModelInfo>, EvrenError> {
        self.json_al::<ModelList>("models").await.map(|l| l.data)
    }

    pub async fn sohbet(&self, istek: &ChatRequest) -> Result<(ChatResponse, YanitMeta), EvrenError> {
        let mut govde = istek.clone();
        govde.stream = false;
        govde.stream_options = None;
        let (yanit, meta) = self.gonder(self.http.post(self.url("chat/completions")).json(&govde)).await?;
        Ok((json_coz(yanit).await?, meta))
    }
}

async fn json_coz<T: DeserializeOwned>(yanit: reqwest::Response) -> Result<T, EvrenError> {
    let govde = yanit.bytes().await.map_err(ag_hatasi)?;
    serde_json::from_slice(&govde).map_err(|e| EvrenError::GecersizYanit(format!("JSON çözülemedi: {e}")))
}

pub(crate) fn ag_hatasi(e: reqwest::Error) -> EvrenError {
    EvrenError::Ag(e.to_string())
}

pub(crate) fn simdi_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}
```

- [ ] **Step 4: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa-evren --test istemci`
Expected: `test result: ok. 7 passed`

- [ ] **Step 5: Tüm crate testleri, lint, biçim, commit**

Run: `cargo test -p kalfa-evren && cargo clippy -p kalfa-evren --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: bütün test dosyaları yeşil, lint temiz.

```bash
git add crates/kalfa-evren
git commit -m "kalfa-evren: HTTP istemcisi — şart uçları, katalog ve bloklu sohbet"
```

---

### Task 6: Akışlı sohbet ve canlı testler

**Files:**
- Modify: `crates/kalfa-evren/src/istemci.rs`, `crates/kalfa-evren/src/lib.rs`
- Modify: `crates/kalfa-evren/tests/istemci.rs`
- Create: `crates/kalfa-evren/tests/canli.rs`

**Interfaces:**
- Consumes: `EvrenIstemcisi::gonder/url` (Task 5), `SseCozucu` (Task 1), `AkisBirlestirici`, `VeriSonucu` (Task 4)
- Produces:
  - `kalfa_evren::AkisKanali` = `Pin<Box<dyn Stream<Item = Result<AkisOlayi, EvrenError>> + Send>>`
  - `EvrenIstemcisi::sohbet_akisi(&self, istek: &ChatRequest) -> Result<(YanitMeta, AkisKanali), EvrenError>` — `stream: true` ve `stream_options.include_usage: true` zorlanır; HTTP hatası akış başlamadan `Err` olarak döner; akış sırasındaki hatalar kanalda `Err` öğesi olarak gelir ve kanal orada biter; kanal bırakılırsa (drop) arka plan görevi durur ve bağlantı kapanır.

- [ ] **Step 1: Başarısız testleri ekle**

`crates/kalfa-evren/tests/istemci.rs` sonuna ekle:

```rust
use futures_util::StreamExt;
use kalfa_evren::AkisOlayi;

fn akis_yaniti(govde: Vec<u8>) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_raw(govde, "text/event-stream")
        .insert_header("x-request-id", "akis-1")
        .insert_header("x-evren-credits-remaining", "1000.0000")
}

#[tokio::test]
async fn akisli_sohbet_fixture_uzerinden() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(akis_yaniti(ortak::fixture_bayt("chat_tool_calls_stream.sse")))
        .mount(&s)
        .await;
    let istek = ChatRequest::new("glm-5.3", vec![ChatMessage::user("İstanbul'da hava?")]);
    let (meta, akis) = i.sohbet_akisi(&istek).await.unwrap();
    assert_eq!(meta.istek_id.as_deref(), Some("akis-1"));

    let olaylar: Vec<_> = akis.collect().await;
    assert!(olaylar.iter().all(|o| o.is_ok()), "{olaylar:?}");
    let olaylar: Vec<AkisOlayi> = olaylar.into_iter().map(Result::unwrap).collect();
    assert!(olaylar.iter().any(|o| matches!(o, AkisOlayi::AracCagrilari(v) if v[0].function.arguments == "{\"sehir\": \"Istanbul\"}")));
    assert_eq!(olaylar.last(), Some(&AkisOlayi::Bitti { neden: "tool_calls".into() }));

    let gonderilen: serde_json::Value =
        serde_json::from_slice(&s.received_requests().await.unwrap()[0].body).unwrap();
    assert_eq!(gonderilen["stream"], true);
    assert_eq!(gonderilen["stream_options"]["include_usage"], true);
}

#[tokio::test]
async fn yarim_kalan_akis_kanalda_hata_olarak_biter() {
    let (s, i) = sunucu_ve_istemci().await;
    let metin = ortak::fixture_metin("chat_tool_calls_stream.sse");
    let bitis = metin.find("\"finish_reason\": \"tool_calls\"").unwrap();
    let kes = metin[..bitis].rfind("data:").unwrap();
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(akis_yaniti(metin.as_bytes()[..kes].to_vec()))
        .mount(&s)
        .await;
    let (_, akis) = i.sohbet_akisi(&ChatRequest::new("glm-5.3", vec![])).await.unwrap();
    let olaylar: Vec<_> = akis.collect().await;
    assert_eq!(olaylar.last(), Some(&Err(EvrenError::AkisYarimKaldi)));
    assert!(!olaylar.iter().any(|o| matches!(o, Ok(AkisOlayi::AracCagrilari(_)))));
}

#[tokio::test]
async fn akis_oncesi_http_hatasi_err_doner() {
    let (s, i) = sunucu_ve_istemci().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(json_yanit(401, ortak::fixture_bayt("error_unauthorized.json")))
        .mount(&s)
        .await;
    let sonuc = i.sohbet_akisi(&ChatRequest::new("glm-5.3", vec![])).await;
    assert_eq!(sonuc.err(), Some(EvrenError::AnahtarGecersiz));
}
```

`crates/kalfa-evren/Cargo.toml`'daki `[dev-dependencies]` bölümüne `futures-util`'i de ekle:

```toml
[dev-dependencies]
futures-util.workspace = true
wiremock.workspace = true
```

- [ ] **Step 2: Testin başarısız olduğunu gör**

Run: `cargo test -p kalfa-evren --test istemci`
Expected: derleme hatası — `no method named `sohbet_akisi` found for struct `EvrenIstemcisi``.

- [ ] **Step 3: Akışlı sohbeti yaz**

`crates/kalfa-evren/src/istemci.rs` başındaki `use` bloğunu şununla değiştir:

```rust
use std::fmt;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{Stream, StreamExt};
use reqwest::header::RETRY_AFTER;
use serde::de::DeserializeOwned;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::akis::{AkisBirlestirici, AkisOlayi, VeriSonucu};
use crate::hata::{EvrenError, hata_esle};
use crate::kredi::{YanitMeta, basliklardan};
use crate::sse::{SseCozucu, VARSAYILAN_AZAMI_OLAY};
use crate::tipler::{ChatRequest, ChatResponse, ModelInfo, ModelList, StreamOptions, TermsStatus, TermsText};

/// Akış olayları kanalı. Bırakılırsa (drop) bağlantı kapanır.
pub type AkisKanali = Pin<Box<dyn Stream<Item = Result<AkisOlayi, EvrenError>> + Send>>;
```

`impl EvrenIstemcisi` bloğunun içine, `sohbet` yönteminin altına ekle:

```rust
    /// Akışlı sohbet. HTTP hatası akış başlamadan `Err` döner; akış sırasındaki
    /// hata kanalda son öğe olarak gelir.
    pub async fn sohbet_akisi(&self, istek: &ChatRequest) -> Result<(YanitMeta, AkisKanali), EvrenError> {
        let mut govde = istek.clone();
        govde.stream = true;
        govde.stream_options = Some(StreamOptions { include_usage: true });
        let (yanit, meta) = self.gonder(self.http.post(self.url("chat/completions")).json(&govde)).await?;
        let (tx, rx) = mpsc::channel(64);
        tokio::spawn(akisi_isle(yanit, tx));
        Ok((meta, Box::pin(ReceiverStream::new(rx))))
    }
```

Dosyanın sonuna ekle:

```rust
type Gonderici = mpsc::Sender<Result<AkisOlayi, EvrenError>>;

/// Yanıt gövdesini okur, SSE → birleştirici hattından geçirip kanala yazar.
/// Alıcı bırakılırsa gönderim başarısız olur, görev biter ve yanıt (bağlantı)
/// bırakılır.
async fn akisi_isle(yanit: reqwest::Response, tx: Gonderici) {
    let mut baytlar = yanit.bytes_stream();
    let mut cozucu = SseCozucu::yeni(VARSAYILAN_AZAMI_OLAY);
    let mut birlestirici = AkisBirlestirici::yeni();

    while let Some(parca) = baytlar.next().await {
        let parca = match parca {
            Ok(p) => p,
            Err(e) => {
                let _ = tx.send(Err(ag_hatasi(e))).await;
                return;
            }
        };
        let sse_olaylari = match cozucu.besle(&parca) {
            Ok(o) => o,
            Err(e) => {
                let _ = tx.send(Err(EvrenError::GecersizYanit(e.to_string()))).await;
                return;
            }
        };
        for sse in sse_olaylari {
            match birlestirici.veri_isle(&sse.veri) {
                Ok(VeriSonucu::Olaylar(olaylar)) => {
                    for o in olaylar {
                        if tx.send(Ok(o)).await.is_err() {
                            return;
                        }
                    }
                }
                Ok(VeriSonucu::Bitti) => {
                    son_olaylari_gonder(&mut birlestirici, &tx).await;
                    return;
                }
                Err(e) => {
                    let _ = tx.send(Err(e)).await;
                    return;
                }
            }
        }
    }
    // Bağlantı `[DONE]` gelmeden kapandı: finish_reason görüldüyse akış tamamdır,
    // görülmediyse yarıda kalmıştır.
    son_olaylari_gonder(&mut birlestirici, &tx).await;
}

async fn son_olaylari_gonder(birlestirici: &mut AkisBirlestirici, tx: &Gonderici) {
    match birlestirici.bitir() {
        Ok(son) => {
            for o in son {
                let _ = tx.send(Ok(o)).await;
            }
        }
        Err(e) => {
            let _ = tx.send(Err(e)).await;
        }
    }
}
```

`crates/kalfa-evren/src/lib.rs` dışa aktarımını güncelle:

```rust
pub use istemci::{AkisKanali, ApiAnahtari, EvrenIstemcisi, VARSAYILAN_TABAN};
```

- [ ] **Step 4: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa-evren --test istemci`
Expected: `test result: ok. 10 passed`

- [ ] **Step 5: Canlı testleri yaz**

`crates/kalfa-evren/tests/canli.rs`:

```rust
//! Canlı EVREN testleri. Varsayılan olarak atlanır.
//!
//! Çalıştırma (depo kökünde, `.env` içinde EVREN_API_KEY ve EVREN_BASE_URL):
//!   set -a && . ./.env && set +a && cargo test -p kalfa-evren --test canli -- --ignored
//!
//! Probe sırasında bütün modellerin fiyatı 0.0 CR idi (spec §3.5).

use futures_util::StreamExt;
use kalfa_evren::tipler::{ChatMessage, ChatRequest, ToolDef};
use kalfa_evren::{AkisOlayi, ApiAnahtari, EvrenIstemcisi, VARSAYILAN_TABAN};
use serde_json::json;

fn canli_istemci() -> EvrenIstemcisi {
    let anahtar = std::env::var("EVREN_API_KEY").expect("EVREN_API_KEY tanımlı olmalı");
    let taban = std::env::var("EVREN_BASE_URL").unwrap_or_else(|_| VARSAYILAN_TABAN.to_string());
    EvrenIstemcisi::yeni(taban, ApiAnahtari::yeni(anahtar)).unwrap()
}

#[tokio::test]
#[ignore = "canlı EVREN API; EVREN_API_KEY gerekir"]
async fn canli_sart_durumu_kabul_edilmis() {
    let d = canli_istemci().sart_durumu().await.unwrap();
    assert!(d.accepted, "hesapta şartlar kabul edilmiş olmalı (şart kabulü testle yapılmaz)");
}

#[tokio::test]
#[ignore = "canlı EVREN API; EVREN_API_KEY gerekir"]
async fn canli_katalog_profil_modellerini_icerir() {
    let m = canli_istemci().modeller().await.unwrap();
    for id in ["glm-5.3", "deepseek-v4-flash", "qwen3.8-flash-next"] {
        assert!(m.iter().any(|x| x.id == id && x.task == "chat"), "{id} katalogda chat olarak olmalı");
    }
}

#[tokio::test]
#[ignore = "canlı EVREN API; EVREN_API_KEY gerekir"]
async fn canli_akisli_arac_cagrisi() {
    let mut istek = ChatRequest::new("qwen3.8-flash-next", vec![ChatMessage::user("İzmir'de hava nasıl?")]);
    istek.tools.push(ToolDef::function(
        "hava_durumu",
        "Bir şehrin güncel hava durumunu getirir",
        json!({"type": "object", "properties": {"sehir": {"type": "string"}}, "required": ["sehir"]}),
    ));
    let (meta, akis) = canli_istemci().sohbet_akisi(&istek).await.unwrap();
    assert!(meta.istek_id.is_some(), "X-Request-Id gelmeli");
    let olaylar: Vec<_> = akis.collect().await;
    let cagri = olaylar
        .iter()
        .find_map(|o| match o {
            Ok(AkisOlayi::AracCagrilari(v)) => Some(v[0].clone()),
            _ => None,
        })
        .expect("araç çağrısı gelmeli");
    assert_eq!(cagri.function.name, "hava_durumu");
    let arg: serde_json::Value = serde_json::from_str(&cagri.function.arguments).unwrap();
    assert!(arg["sehir"].as_str().unwrap().contains("zmir"));
}
```

- [ ] **Step 6: Canlı testlerin atlandığını, sonra çalıştığını gör**

Run: `cargo test -p kalfa-evren --test canli`
Expected: `test result: ok. 0 passed; 0 failed; 3 ignored`

Run (depo kökünde, `.env` dosyası varken): `set -a && . ./.env && set +a && cargo test -p kalfa-evren --test canli -- --ignored`
Expected: `test result: ok. 3 passed`. Anahtar yoksa ya da ağ yoksa bu adımı atla ve raporda "canlı testler koşturulmadı" diye belirt; Step 7'ye engel değildir.

- [ ] **Step 7: Tüm crate, lint, biçim, commit**

Run: `cargo test -p kalfa-evren && cargo clippy -p kalfa-evren --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: yeşil ve temiz.

```bash
git add crates/kalfa-evren
git commit -m "kalfa-evren: akışlı sohbet ve #[ignore] canlı testler"
```

---

## Plan sonu kontrol listesi

- [ ] `cargo test -p kalfa-evren` — 6 test dosyası yeşil (`sse`, `tipler`, `hata_kredi`, `akis`, `istemci`; `canli` atlanır)
- [ ] `research/evren-probe/fixtures/` altında değişiklik yok: `git status research/` temiz
- [ ] `git grep -n "evren_llm_" -- crates/` yalnızca testteki sahte anahtarı bulur
