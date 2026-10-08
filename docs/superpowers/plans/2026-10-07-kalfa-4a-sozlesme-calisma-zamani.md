# Kalfa 4a — Sözleşme ve çalışma zamanı: Uygulama Planı

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Kalfa'nın eklenti sözleşmesi ve çalışma zamanı. Bu planın sonunda:
- **Sözleşme:** WIT 0.1 (`kalfa:plugin@0.1.0`) ve eklenti yazarının SDK'sı
  (`kalfa-plugin-sdk`). `wasm32-wasip2` için yazılan bir eklenti yüklenir,
  araçları `Arac` olarak sunulur ve çağrılır.
- **Yükleme:** manifest (`kalfa.toml`) doğrulanır. Bileşen derlenir.
  İzinsiz import (ağ) ve sözleşmeye uymayan dünya reddedilir.
- **Sınırlar:** her çağrıda yeni bir örnek oluşur. Sınırlar şunlardır:
  - Bellek 512 MiB.
  - CPU bütçesi 60 sn. Eklenti her tikte tokio'ya yer verir.
  - Toplam süre 30 dk.
  - İptalden sonra eklentiye çıkması için 2 sn verilir.
  - Çıktı ve günlük de sınırlıdır.
- **Yetki:**
  - Eklenti dosyaları yalnızca host fonksiyonlarıyla, manifestin globları
    içinde ve yerleşik okumanın yolundan okur.
  - Gizli dosya, `.git`, korumalı yol, sembolik bağlantı ve hard link'li
    dosya eklentiye hiç açılmaz.
  - `std::fs` hiçbir dosyayı açamaz.
- **Depo:** eklentiye ve çalışma alanına özel anahtar-değer deposu; kota,
  atomik yazma.
- **Çekirdek:**
  - `Hazirlik.eklenti`: tarif mühre girer.
  - `Hazirlik.kilitsiz`: eklenti çağrı boyunca kilit tutmaz.

**Architecture:**
- **`crates/kalfa-plugin-sdk`:**
  - `wit/kalfa.wit` ve `wit_bindgen::generate!`.
  - `kalfa_eklentisi!` makrosu ve `argumanlar` yardımcısı.
  - Lisansı `Apache-2.0 WITH LLVM-exception`.
- **`crates/kalfa-plugin`:**
  - `manifest`: `kalfa.toml`, ad kuralları.
  - `Motor`: tek `wasmtime::Engine` ve epoch iş parçacığı. `yukle` ve
    eşzamanlılık burada.
  - `host`: bir çağrının `Store` durumu ve host fonksiyonları.
  - `depo`: anahtar-değer deposu.
  - `EklentiAraci`: `kalfa_core::arac::Arac`.
- **Test eklentileri** (`crates/kalfa-plugin/test-eklentileri/`, workspace
  üyesi): `kalfa-test-eklentisi`, `kalfa-test-soket`,
  `kalfa-test-yanlis-dunya`. Testler onları `wasm32-wasip2` için kendileri
  derler (G7).
- **Çekirdek:**
  - `Hazirlik`'e `eklenti: Option<EklentiTarifi>` ve `kilitsiz: bool`.
  - `muhur_hesapla` ikisini de bağlar.
  - `tek_calistir` kilitsiz hazırlıkta kilit almaz.
  - `VeriCikisi::EklentiCiktisi`.
  - `izin::korumali_listede` dışa açılır.
- **`kalfa-tools`:** `CalismaAlani::dosya_bilgisi` (boyut ve değişme zamanı,
  okuma yolundan).

**Tech Stack:** Rust 1.98.1, wasmtime 49.0.2 (component model, async),
wasmtime-wasi 49.0.2 (p2), wit-bindgen 0.62, `wasm32-wasip2`.

**Kaynaklar:**
- [Aşama 4 tasarımı](../specs/2026-10-07-kalfa-4-eklenti-rag-design.md):
  §5, §6, §7, §8.1–§8.3, §10, §12, §16.1, §17.1, §19 (G1, G3, G7), §20.
- [Ön ölçüm](../bulgular/2026-10-07-kalfa-4-on-olcum.md) §2.
- [Üst belge](../specs/2026-09-21-kalfa-design.md) §6.1, §7, §8 (r3.10).

**Doğrulama durumu:**
- **Kod önceden yazıldı ve iki platformda doğrulandı** (tasarım karar 12).
  Planın her dosyası doğrulanmış çalışma ağacından üretildi (`deneme/4a`).
- Doğrulama sayıları "Ölçümler" bölümündedir.

**Uygulayan:** kullanıcı seçer.
- S15'te prompt injection örneği yok; Antigravity'ye verilebilir.
- wasmtime API'si yeni olduğu için kod hazırdır. Uygulayıcının işi
  dosyaları yazmak, testleri koşturmak ve iki platformda yeşil göstermektir.

**Sıra:** aşama 4'ün ilk planıdır (tasarım §20). 4b (EVREN yolu) ve 4c
(`kalfa-rag`) bunun sözleşmesine ve host'una dayanır.

**Kapsam dışı (sonraki planlar):**
- `evren-cagir`'ın gerçek yolu, ilk gönderim onayı, gönderim günlüğü,
  önizleme, oturum başına sınır: **4b**. Bu planda `evren-cagir`, yetenek
  varsa da `izin-yok("… aşama 4b")` döndürür.
- SDK'nın `gomme` ve `yeniden_sirala` yardımcıları (§12): onları kullanan
  ilk eklentiyle, **4c**.
- `eklentiler.json`, kurulum, Eklentiler ekranı, çalışma alanında aç/kapat
  (`sunulur_mu`), depo dizininin yeri ve onun uyumluluk testi (depo
  yalıtımdan görünmez), kurucu: **4d**.
- `VeriCikisi::EklentiCiktisi`'nin arayüz metni: **4b**. 4a'da eklenti aracı
  her kipte onaysızdır; onay kartına çıkmaz.

## Kararlar

Tasarımın kararları (§2.2) olduğu gibi uygulanır. Plan yazılırken verilen
kararlar şunlar; tasarım belgesinden farklı olanlar "Tasarımdan farklar"da.

1. **Test eklentilerini testler derler (G7).** `tests/ortak/mod.rs` ilk
   kullanımda `cargo build --target wasm32-wasip2 --locked -p …` çalıştırır
   ve `.wasm`'ın yerini cargo'nun JSON çıktısından alır.
   - Derlenmişse cargo hiçbir şey yapmaz (~0,3 sn). Test eski bir `.wasm`'la
     koşmaz.
   - `cargo test --workspace` tek başına yeterlidir. Doğrulama komutları ve
     CI değişmez; CI'ya yalnızca eklentilerin `wasm32-wasip2` clippy'si
     eklenir.
   - Hedef `rust-toolchain.toml`'dan kendiliğinden kurulur.
   - Eklenmesi bir kez gerekir. WSL'de `rustup target add wasm32-wasip2
     --toolchain 1.98.1`.
   - Hedef yoksa test, ne yapılacağını söyleyerek düşer.
   - Build betiği yolu denenmedi. İç içe cargo, build betiğinin içinde aynı
     `target` kilidini bekler; test anında kilit serbesttir.
2. **`stderr` yalnızca son 4 KiB'ı tutar.** wasmtime'ın `MemoryOutputPipe`'ı
   dolunca eklentiyi tuzağa düşürüyor. Kalfa'nın `SonBaytlar`'ı eskileri
   atar.
3. **Dünyaya uymayan bileşen örneklemede düşer.** wasmtime 49'da
   `InstancePre` export tiplerini denetlemiyor; bağların `instantiate`'i
   denetliyor. Yükleme `araclari-listele`'den önce örneklediği için sonuç
   aynıdır: hata `YuklemeHatasi::Dunya`.
4. **Süreler testte kısaltılabilir.** `Motor::test_sureleri` (yalnızca
   `test-destek`) CPU bütçesini ve toplam süreyi alır; ürün yolu
   `Motor::yeni`'dir ve sabitleri kullanır (karar 10).
5. **Dosya erişim hataları:** yok → `bulunamadi`. Bağlantı, izin ve başka
   G/Ç hatası → `izin-yok("… okunamadı: …")`. Hard link →
   `izin-yok("… hard link …")`. Yerleşik okumanın hata türü bunları ayırmıyor.
6. **İlerleme olayları birleştirilmez, seyreltilir:** 100 ms içinde gelen
   ikinci olay atılır; sonraki olay güncel durumu taşır.
7. **`dosya-listele` `.gitignore`'a uyar** (yerleşik `dosya_listele`'nin
   `CalismaAlani::dosyalar`'ı). Boyut ve değişme zamanı, okuma yolundan
   açılan tutamaçtan alınır.

### Tasarımdan farklar

| Tasarım | Plan | Neden |
|---|---|---|
| Host'un gerçek işini bir `EklentiOrtami` trait'i yapar; `kalfa-uygulama` uygular, testler sahtesini (§5.1) | Trait yok. Dosya çağrıları `kalfa-tools::CalismaAlani`'yla, depo `kalfa-plugin::depo`'yla, ikisi de `kalfa-plugin`'de | Güvenlik sınırı gerçek okuma yoluyla sınanmalı. Sahte, sınadığı şeyi atlardı. Testler geçici dizinlerle Tauri'siz ve ağsız koşar. 4b, `CagriOrtami`'na EVREN köprüsünü ekler. |
| Depo `kalfa-uygulama/src/eklentiler/`'da (§5.1) | Depo `kalfa-plugin/src/depo.rs`'te; dizini çağıran verir (`EklentiAraci::hepsi(…, depo_dizini)`) | Depo host'un parçası. Yeri (`<yerel veri>/eklenti-verisi/…`) ve uyumluluk testi 4d'de. |
| `kalfa-plugin`, `kalfa-core`'a bağlı (§5.1) | `kalfa-core` ve `kalfa-tools`'a bağlı | Tek okuma yolu (§8.3): `CalismaAlani`. |
| `.wasm`'lar ayrı komutla derlenir, testler yoksa düşer (§17.1) | Testler derler (karar 1) | G7'nin tercih ettiği sonuç: `cargo test` tek başına yeterli. |
| `stderr` 64 KiB belleğe yazılır, son 4 KiB gösterilir (§7.3) | Yalnızca son 4 KiB tutulur (karar 2) | Gösterilmeyen 60 KiB'ı tutmanın gereği yok. |
| Bağımlılık tablosunda `semver` yok (§5.2) | `semver` (Cargo.lock'ta 1.0.28 var) | Manifest sürümünün doğrulanması; 4d'de sürüm karşılaştırması. |

Tasarım belgesi bu planın PR'ında bu farklara göre güncellendi.

## Global Constraints

- **Araçlar:** Rust 1.98.1; `cargo` komutları `--locked` ile.
  `rust-toolchain.toml`'a `targets = ["wasm32-wasip2"]` eklenir.
- **Bağımlılıklar** (tasarım §5.2; başka paket girmez):

  | Bağımlılık | Nerede | Not |
  |---|---|---|
  | `wasmtime = "=49.0.2"` | `kalfa-plugin` | `default-features = false`; `runtime`, `cranelift`, `component-model`, `async`, `std` |
  | `wasmtime-wasi = "=49.0.2"` | `kalfa-plugin` | `default-features = false`; `p2` |
  | `wit-bindgen = "0.62"` | `kalfa-plugin-sdk`, `kalfa-test-yanlis-dunya` | |
  | `toml = "1.1"` | `kalfa-plugin` | Cargo.lock'ta vardı |
  | `semver = "1.0.28"` | `kalfa-plugin` | Cargo.lock'ta vardı |
  | `globset`, `jsonschema`, `sha2`, `hex`, `serde`, `serde_json`, `thiserror`, `tokio`, `tokio-util`, `async-trait` | `kalfa-plugin` | workspace'te vardı |

  wasmtime tam sürümle sabittir; yükseltme ayrı bir iştir.
- **`unsafe`:** yoktur.
- **`unwrap`/`expect`:** üretim kodunda yoktur. `kalfa-plugin`'in üretim
  kodunda `Mutex` zehirlenmesi `unwrap_or_else(|z| z.into_inner())` ile
  işlenir.
- **Güvenlik sınırları** (her biri testlidir):
  - **İmport izin listesi:** yalnızca `kalfa:plugin/{tipler,host}@0.1.0` ve
    WASI 0.2'nin `io`, `clocks`, `random`, `cli`, `filesystem` paketleri.
    Başka her import yüklemeyi reddeder (`izinli_import`).
  - **İkinci katman:** `WasiCtx`'te ön açık dizin yoktur; TCP, UDP ve ad
    çözme kapalıdır. Import denetimi atlansa bile soket bağlanamaz.
  - **Dosya:** manifest globu, gizli değil, korumalı değil (`.git` dahil).
    Yerleşik okuma yolundan (no-follow, gerçek ad, hard link reddi). Kilit:
    okuma süresince o yolun okuma kilidi ve genel okuma kilidi.
  - **Yetenek:** manifestte olmayan yetenek `izin-yok` alır.
  - **Manifest:** bilinmeyen alan reddedilir; `sozlesme` yalnızca `"0.1"`.
  - **Mühür:** eklentinin özeti, sürümü, globları, uçları ve `kilitsiz`.
- **Dil:** adlar, açıklamalar, testler ve ileti metinleri Türkçe. Eklentiye
  dönen hata iletileri ve modelin gördüğü iletiler `kalfa-plugin`'de
  sabittir.
- **Biçim:** `cargo fmt`. `npx prettier` kullanılmaz.

## Ölçümler

Doğrulanmış çalışma ağacında (Windows 11, WSL2 Ubuntu 24.04):

| | Windows | WSL (Ubuntu 24.04) |
|---|---|---|
| `cargo test --workspace` | 860 geçti, 10 atlandı (canlı) | 802 geçti, 10 atlandı |
| `cargo test -p kalfa-uygulama -p kalfa --features e2e` | 114 | 113 |
| `kalfa-plugin` | 41 (9 birim + 32 bütünleşme) | 41 |
| Üç clippy ve `fmt` | temiz | temiz |

- **G1 (Linux derlemesi):** `kalfa-plugin`'in temiz sürüm derlemesi
  (wasmtime dahil) 2 dk 7 sn, 12 çekirdek. Ön ölçümde Windows'ta sürüm 3 dk
  26 sn, hata ayıklama 1 dk 25 sn idi. İkili boyutu `kalfa`'ya girdiği 4d'de
  ölçülür. CI süresine etkisi bu planın PR'ında görünür.
- **G3 (CPU bütçesi):**
  - `sonsuz_dongu_cpu_butcesinde_durur_ve_yer_verir`: 2 sn'lik bütçe 1,8–10
    sn içinde tuzak kuruyor. Tek iş parçacıklı çalışma zamanında öteki görev
    sürerken ilerliyor (>50 tik).
  - `host_cagrisinda_beklenen_sure_cpu_butcesine_sayilmaz`: kilit bekleyen
    3 sn'lik okuma, 2 sn'lik bütçeye takılmıyor.
- **G7 (test eklentilerinin derlenmesi):** testin içinden `cargo build`
  çalışıyor. İlk derleme ~10 sn (hata ayıklama), sonra ~0,3 sn. Windows'ta
  ve WSL'de `cargo test --workspace` tek başına yeşil.
- **İptal:** iptale uyan eklenti 1,5 sn içinde çıkıyor. Uymayan eklenti
  2,1–2,9 sn'de bırakılıyor; sonraki çağrı yeni örnekle çalışıyor.

## Görevler

Her görev bir commit'tir (önek parantez içinde). Testler aynı görevde yazılır
ve görev sonunda koşturulur. Kod blokları doğrulanmış çalışma ağacından
alınmıştır; dosyalar olduğu gibi yazılır, farklar uygulanır.

### Görev 1: Workspace ve araç zinciri (`derleme:`)

- [ ] `Cargo.toml`'a üyeler ve bağımlılıklar, `rust-toolchain.toml`'a hedef:

`Cargo.toml`:

```diff
diff --git a/Cargo.toml b/Cargo.toml
index 7b918a2..bc139d6 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -8,6 +8,11 @@ members = [
     "crates/kalfa-uygulama",
     "crates/kalfa",
     "crates/kalfa-yalitim",
+    "crates/kalfa-plugin-sdk",
+    "crates/kalfa-plugin",
+    "crates/kalfa-plugin/test-eklentileri/kalfa-test-eklentisi",
+    "crates/kalfa-plugin/test-eklentileri/kalfa-test-soket",
+    "crates/kalfa-plugin/test-eklentileri/kalfa-test-yanlis-dunya",
 ]
 
 [workspace.package]
@@ -40,6 +45,7 @@ rust_decimal = { version = "1.43", features = ["serde"] }
 rustls = { version = "0.23.45", default-features = false, features = ["ring", "std", "tls12", "logging"] }
 regex = "1.13"
 same-file = "1.0.6"
+semver = "1.0.28"
 seccompiler = "0.5.0"
 serde = { version = "1.0.229", features = ["derive"] }
 serde_json = "1.0.151"
@@ -50,7 +56,13 @@ thiserror = "2.0.20"
 tokio = { version = "1.53", features = ["rt-multi-thread", "macros", "sync", "time", "net"] }
 tokio-stream = "0.1.19"
 tokio-util = "0.7.19"
+toml = "1.1"
 url = "2.5"
+# Eklenti çalışma zamanı (aşama 4 tasarımı §5.2). Tam sürümle sabit: bileşen
+# API'si sürümler arasında değişiyor; yükseltme ayrı bir iştir.
+wasmtime = { version = "=49.0.2", default-features = false, features = ["runtime", "cranelift", "component-model", "async", "std"] }
+wasmtime-wasi = { version = "=49.0.2", default-features = false, features = ["p2"] }
+wit-bindgen = "0.62"
 uuid = { version = "1.26", features = ["v4", "serde"] }
 wiremock = "0.6.5"
 windows-sys = { version = "0.61", features = [
```


`rust-toolchain.toml`:

```diff
diff --git a/rust-toolchain.toml b/rust-toolchain.toml
index 68c0ea2..53d631b 100644
--- a/rust-toolchain.toml
+++ b/rust-toolchain.toml
@@ -1,3 +1,5 @@
 [toolchain]
 channel = "1.98.1"
 components = ["rustfmt", "clippy"]
+# Eklentilerin hedefi (aşama 4 tasarımı §5.2).
+targets = ["wasm32-wasip2"]
```


- [ ] `Cargo.lock`'u güncellemek için Görev 2–6'nın crate'leri yazıldıktan
  sonra bir kez `cargo build` (kilitsiz) çalıştırın; sonraki bütün komutlar
  `--locked`'dır. Beklenen yeni paketler: `wasmtime*` 49.0.2,
  `cranelift-*`, `wasmtime-wasi*`, `wit-bindgen*` 0.62, `kalfa-plugin*`,
  `kalfa-test-*`.
- [ ] Hedef yoksa: `rustup target add wasm32-wasip2 --toolchain 1.98.1`.

### Görev 2: Sözleşme ve SDK (`eklenti:`)

- [ ] WIT (tasarım §6.1):

`crates/kalfa-plugin-sdk/wit/kalfa.wit` (yeni):

```wit
// Kalfa eklenti sözleşmesi 0.1 (aşama 4 tasarımı §6). Sözleşme 0.x boyunca
// kırılabilir (üst belge §8.5).
package kalfa:plugin@0.1.0;

interface tipler {
  enum evren-ucu { embeddings, rerank }
  enum seviye { hata, uyari, bilgi, ayrinti }

  variant hata {
    // Manifestte olmayan yetenek, glob dışı yol, gizli dosya, onaysız gönderim.
    izin-yok(string),
    bulunamadi(string),
    iptal-edildi,
    // Süre, boyut, kota, istek sayısı, hız sınırı.
    sinir-asildi(string),
    // Sağlayıcı hatası.
    evren(string),
    diger(string),
  }

  record ilerleme { tamamlanan: u64, toplam: option<u64>, mesaj: option<string> }

  record dosya-bilgisi {
    // Çalışma alanına göre, `/` ayraçlı.
    yol: string,
    // Bayt.
    boyut: u64,
    // Son değişme zamanı, Unix epoch'tan beri milisaniye.
    degisme: u64,
  }
}

// Eklenti → host.
interface host {
  use tipler.{evren-ucu, seviye, hata, ilerleme, dosya-bilgisi};

  dosya-listele:   func(desen: string) -> result<list<dosya-bilgisi>, hata>;
  dosya-oku:       func(yol: string) -> result<string, hata>;
  evren-cagir:     func(uc: evren-ucu, govde: string) -> result<string, hata>;
  depo-oku:        func(anahtar: string) -> result<option<list<u8>>, hata>;
  depo-yaz:        func(anahtar: string, deger: list<u8>) -> result<_, hata>;
  depo-sil:        func(anahtar: string) -> result<_, hata>;
  ilerleme-bildir: func(durum: ilerleme);
  iptal-istendi:   func() -> bool;
  gunluk:          func(duzey: seviye, mesaj: string);
}

// Host → eklenti.
interface arac {
  use tipler.{hata};

  record arac-tanimi { ad: string, aciklama: string, parametre-semasi: string }
  record sonuc { icerik: string }

  araclari-listele: func() -> list<arac-tanimi>;
  calistir:         func(ad: string, argumanlar: string) -> result<sonuc, hata>;
}

world eklenti {
  import host;
  export arac;
}
```


- [ ] Crate:

`crates/kalfa-plugin-sdk/Cargo.toml` (yeni):

```toml
[package]
name = "kalfa-plugin-sdk"
description = "Kalfa eklentilerini yazmak için SDK: WIT 0.1 sözleşmesi ve yardımcılar"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
# Eklenti yazarları SDK'yı kendi lisanslarıyla kullanabilsin (üst belge §8.7).
license = "Apache-2.0 WITH LLVM-exception"
authors.workspace = true
include = ["src/**", "wit/**", "Cargo.toml", "README.md", "LICENSE", "LICENSE-LLVM-exception", "NOTICE"]

[dependencies]
serde.workspace = true
serde_json.workspace = true
wit-bindgen.workspace = true
```


`crates/kalfa-plugin-sdk/src/lib.rs` (yeni):

````rust
//! Kalfa eklenti SDK'sı (aşama 4 tasarımı §12).
//!
//! Eklenti yazarı wit-bindgen ile uğraşmaz: araçlarını [`Guest`] ile yazar ve
//! [`kalfa_eklentisi!`] ile dışa verir. Eklenti `wasm32-wasip2` için derlenir.
//!
//! ```ignore
//! use kalfa_plugin_sdk::{AracTanimi, Guest, Hata, Sonuc, host};
//!
//! struct Ornek;
//!
//! impl Guest for Ornek {
//!     fn araclari_listele() -> Vec<AracTanimi> { /* … */ }
//!     fn calistir(ad: String, argumanlar: String) -> Result<Sonuc, Hata> { /* … */ }
//! }
//!
//! kalfa_plugin_sdk::kalfa_eklentisi!(Ornek);
//! ```

// Üretilen kod ve dışa verme makrosu SDK'ya `kalfa_plugin_sdk` adıyla başvurur;
// SDK'nın kendi içinde de bu ad çözülsün.
extern crate self as kalfa_plugin_sdk;

wit_bindgen::generate!({
    world: "eklenti",
    path: "wit",
    pub_export_macro: true,
    export_macro_name: "kalfa_eklentisi",
    default_bindings_module: "kalfa_plugin_sdk",
    runtime_path: "kalfa_plugin_sdk::wit_bindgen::rt",
});

pub use exports::kalfa::plugin::arac::{AracTanimi, Guest, Sonuc};
pub use kalfa::plugin::host;
pub use kalfa::plugin::tipler::{DosyaBilgisi, EvrenUcu, Hata, Ilerleme, Seviye};
pub use {serde_json, wit_bindgen};

/// Aracın JSON argümanlarını çözer. Host argümanları aracın şemasına karşı
/// zaten doğrulamıştır; bu yine de bozuk girdiyi hataya çevirir.
pub fn argumanlar<T: serde::de::DeserializeOwned>(metin: &str) -> Result<T, Hata> {
    serde_json::from_str(metin).map_err(|e| Hata::Diger(format!("argümanlar çözülemedi: {e}")))
}
````


`crates/kalfa-plugin-sdk/README.md` (yeni):

````markdown
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
````


- [ ] Lisans dosyaları: depo kökündeki `LICENSE`, `LICENSE-LLVM-exception` ve
  `NOTICE` `crates/kalfa-plugin-sdk/`'ye kopyalanır (`cargo package`'a
  girsinler; üst belge §8.7).
- [ ] `cargo build -p kalfa-plugin-sdk --target wasm32-wasip2 --locked` ve
  host hedefinde `cargo build -p kalfa-plugin-sdk --locked` geçer.

### Görev 3: Test eklentileri (`eklenti:`)

Her araç bir davranışı sınar (tasarım §16.1). Testler aynı `.wasm`'ı farklı
manifestlerle yükler.

`crates/kalfa-plugin/test-eklentileri/kalfa-test-eklentisi/Cargo.toml` (yeni):

```toml
[package]
name = "kalfa-test-eklentisi"
description = "Kalfa eklenti çalışma zamanının test eklentisi: temel yol, sınırlar, dosya ve depo"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
kalfa-plugin-sdk = { path = "../../../kalfa-plugin-sdk" }
```


`crates/kalfa-plugin/test-eklentileri/kalfa-test-eklentisi/src/lib.rs` (yeni):

```rust
//! `kalfa-plugin`'in testlerinin eklentisi (aşama 4 tasarımı §16.1). Her araç
//! bir davranışı sınar; testler aynı `.wasm`'ı farklı manifestlerle yükler.

use kalfa_plugin_sdk::serde_json::Value;
use kalfa_plugin_sdk::{AracTanimi, EvrenUcu, Guest, Hata, Ilerleme, Seviye, Sonuc, host};

struct TestEklentisi;

const ARACLAR: &[&str] = &[
    "yanki", "dongu", "bellek", "sisirme", "dosya", "yetkisiz", "depo", "iptal", "panik",
];

fn metin(a: &Value, alan: &str) -> String {
    a[alan].as_str().unwrap_or_default().to_string()
}

fn hata_metni(h: &Hata) -> String {
    match h {
        Hata::IzinYok(m) => format!("izin-yok: {m}"),
        Hata::Bulunamadi(m) => format!("bulunamadi: {m}"),
        Hata::IptalEdildi => "iptal-edildi".into(),
        Hata::SinirAsildi(m) => format!("sinir-asildi: {m}"),
        Hata::Evren(m) => format!("evren: {m}"),
        Hata::Diger(m) => format!("diger: {m}"),
    }
}

fn sonuc<T>(r: Result<T, Hata>, f: impl FnOnce(T) -> String) -> String {
    match r {
        Ok(d) => f(d),
        Err(h) => hata_metni(&h),
    }
}

impl Guest for TestEklentisi {
    fn araclari_listele() -> Vec<AracTanimi> {
        ARACLAR
            .iter()
            .map(|ad| AracTanimi {
                ad: ad.to_string(),
                aciklama: format!("Test aracı: {ad}"),
                parametre_semasi: r#"{"type":"object"}"#.into(),
            })
            .collect()
    }

    fn calistir(ad: String, argumanlar: String) -> Result<Sonuc, Hata> {
        let a: Value = kalfa_plugin_sdk::argumanlar(&argumanlar)?;
        let icerik = match ad.as_str() {
            "yanki" => {
                host::gunluk(Seviye::Bilgi, "yanki çağrıldı");
                host::ilerleme_bildir(&Ilerleme {
                    tamamlanan: 1,
                    toplam: Some(1),
                    mesaj: Some("bitti".into()),
                });
                metin(&a, "metin")
            }
            // CPU bütçesi: epoch kesmesi bunu durdurmalı.
            "dongu" => {
                let mut x: u64 = 1;
                loop {
                    x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
                    if x == 0 {
                        break format!("{x}");
                    }
                }
            }
            // Bellek sınırı: 1 GiB istenir, sınır 512 MiB.
            "bellek" => {
                let mut parcalar: Vec<Vec<u8>> = Vec::new();
                for _ in 0..1024 {
                    parcalar.push(vec![1u8; 1 << 20]);
                }
                format!("{} MiB ayrıldı", parcalar.len())
            }
            // Sonuç 64 KiB'ta kırpılmalı; günlük 200 satırda kesilmeli.
            "sisirme" => {
                for i in 0..300 {
                    host::gunluk(Seviye::Ayrinti, &format!("satır {i}"));
                }
                "x".repeat(200 * 1024)
            }
            "dosya" => match metin(&a, "islem").as_str() {
                "oku" => sonuc(host::dosya_oku(&metin(&a, "yol")), |m| m),
                "listele" => sonuc(host::dosya_listele(&metin(&a, "desen")), |l| {
                    l.iter()
                        .map(|d| format!("{}\t{}\t{}", d.yol, d.boyut, d.degisme))
                        .collect::<Vec<_>>()
                        .join("\n")
                }),
                // WASI dosya sistemi: ön açık dizin yokken her yol açılamamalı.
                "std" => match std::fs::read_to_string(metin(&a, "yol")) {
                    Ok(m) => format!("OKUNDU: {m}"),
                    Err(e) => format!("okunamadı: {e}"),
                },
                diger => return Err(Hata::Diger(format!("bilinmeyen işlem: {diger}"))),
            },
            "yetkisiz" => {
                let evren = sonuc(host::evren_cagir(EvrenUcu::Embeddings, "{}"), |y| y);
                let depo = sonuc(host::depo_yaz("a", &[1]), |()| "yazıldı".into());
                format!("evren: {evren}\ndepo: {depo}")
            }
            "depo" => {
                let anahtar = metin(&a, "anahtar");
                match metin(&a, "islem").as_str() {
                    "oku" => sonuc(host::depo_oku(&anahtar), |d| match d {
                        Some(b) => format!("değer: {}", String::from_utf8_lossy(&b)),
                        None => "yok".into(),
                    }),
                    "yaz" => {
                        let deger = match a["boyut"].as_u64() {
                            Some(n) => vec![b'd'; n as usize],
                            None => metin(&a, "deger").into_bytes(),
                        };
                        sonuc(host::depo_yaz(&anahtar, &deger), |()| "yazıldı".into())
                    }
                    "sil" => sonuc(host::depo_sil(&anahtar), |()| "silindi".into()),
                    diger => return Err(Hata::Diger(format!("bilinmeyen işlem: {diger}"))),
                }
            }
            // `inatci`: iptali yok sayar; host 2 sn sonra bırakmalı.
            "iptal" => {
                let inatci = a["inatci"].as_bool().unwrap_or(false);
                let mut x: u64 = 1;
                loop {
                    for _ in 0..100_000 {
                        x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
                    }
                    if !inatci && host::iptal_istendi() {
                        return Err(Hata::IptalEdildi);
                    }
                    if x == 0 {
                        break format!("{x}");
                    }
                }
            }
            "panik" => {
                eprintln!("stderr satırı");
                panic!("bilerek panik");
            }
            _ => return Err(Hata::Bulunamadi(ad)),
        };
        Ok(Sonuc { icerik })
    }
}

kalfa_plugin_sdk::kalfa_eklentisi!(TestEklentisi);
```


`crates/kalfa-plugin/test-eklentileri/kalfa-test-soket/Cargo.toml` (yeni):

```toml
[package]
name = "kalfa-test-soket"
description = "Test eklentisi: wasi:sockets import eder; yüklenmemeli"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
kalfa-plugin-sdk = { path = "../../../kalfa-plugin-sdk" }
```


`crates/kalfa-plugin/test-eklentileri/kalfa-test-soket/src/lib.rs` (yeni):

```rust
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
```


`crates/kalfa-plugin/test-eklentileri/kalfa-test-yanlis-dunya/Cargo.toml` (yeni):

```toml
[package]
name = "kalfa-test-yanlis-dunya"
description = "Test eklentisi: sözleşmeye uymayan dünya; yüklenmemeli"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
wit-bindgen.workspace = true
```


`crates/kalfa-plugin/test-eklentileri/kalfa-test-yanlis-dunya/src/lib.rs` (yeni):

```rust
//! Sözleşmeye uymayan dünya (aşama 4 tasarımı §16.1): `calistir` farklı
//! imzalı. Yükleme `InstancePre` adımında düşmeli.

wit_bindgen::generate!({
    inline: r#"
        package kalfa:plugin@0.1.0;

        interface arac {
          record arac-tanimi { ad: string, aciklama: string, parametre-semasi: string }
          araclari-listele: func() -> list<arac-tanimi>;
          calistir: func(ad: string) -> string;
        }

        world eklenti {
          export arac;
        }
    "#,
});

use exports::kalfa::plugin::arac::{AracTanimi, Guest};

struct YanlisDunya;

impl Guest for YanlisDunya {
    fn araclari_listele() -> Vec<AracTanimi> {
        vec![AracTanimi {
            ad: "yanlis".into(),
            aciklama: "İmzası sözleşmeye uymaz.".into(),
            parametre_semasi: r#"{"type":"object"}"#.into(),
        }]
    }

    fn calistir(ad: String) -> String {
        ad
    }
}

export!(YanlisDunya);
```


- [ ] `cargo build --target wasm32-wasip2 --locked -p kalfa-test-eklentisi
  -p kalfa-test-soket -p kalfa-test-yanlis-dunya` geçer. Host hedefinde de
  derlenirler (`cargo build --workspace`); wit-bindgen'in import'ları orada
  çağrılınca `unreachable` olur ve çağrılmazlar.

### Görev 4: Çekirdek — eklenti tarifi, kilitsiz araç (`çekirdek:`)

- [ ] Önce testler. Mühür eklentinin tarifine ve `kilitsiz`'e bağlı:

`crates/kalfa-core/tests/arac_muhur.rs`:

```diff
diff --git a/crates/kalfa-core/tests/arac_muhur.rs b/crates/kalfa-core/tests/arac_muhur.rs
index 708b2fe..1a3bc56 100644
--- a/crates/kalfa-core/tests/arac_muhur.rs
+++ b/crates/kalfa-core/tests/arac_muhur.rs
@@ -521,3 +521,68 @@ fn ag_hostlari_muhre_girer() {
         "tek host eklemek mührü değiştirir"
     );
 }
+
+/// Eklenti aracının onayı eklentinin özetine, globlarına ve uçlarına
+/// bağlıdır (aşama 4 tasarımı §8.3): onaydan sonra güncellenen eklentinin
+/// çağrısı çalışmaz.
+#[test]
+fn eklenti_tarifi_muhre_baglanir() {
+    let cagri = CagriId::yeni();
+    let tarif = EklentiTarifi {
+        ad: "kalfa-rag".into(),
+        surum: "0.1.0".into(),
+        ozet: "a".repeat(64),
+        globlar: vec!["**/*.rs".into()],
+        evren_uclari: vec!["embeddings".into()],
+    };
+    let h = Hazirlik {
+        eklenti: Some(tarif.clone()),
+        kilitsiz: true,
+        ..Default::default()
+    };
+    let m = muhur_hesapla(cagri, "kalfa_rag__ara", &json!({}), &h);
+    let degisik = |t: EklentiTarifi| {
+        let h = Hazirlik {
+            eklenti: Some(t),
+            ..h.clone()
+        };
+        muhur_hesapla(cagri, "kalfa_rag__ara", &json!({}), &h)
+    };
+    assert_ne!(
+        degisik(EklentiTarifi {
+            ozet: "b".repeat(64),
+            ..tarif.clone()
+        }),
+        m
+    );
+    assert_ne!(
+        degisik(EklentiTarifi {
+            surum: "0.2.0".into(),
+            ..tarif.clone()
+        }),
+        m
+    );
+    assert_ne!(
+        degisik(EklentiTarifi {
+            globlar: vec!["**/*".into()],
+            ..tarif.clone()
+        }),
+        m
+    );
+    assert_ne!(
+        degisik(EklentiTarifi {
+            evren_uclari: vec![],
+            ..tarif.clone()
+        }),
+        m
+    );
+    assert_eq!(degisik(tarif.clone()), m);
+    let kilitli = Hazirlik {
+        kilitsiz: false,
+        ..h.clone()
+    };
+    assert_ne!(
+        muhur_hesapla(cagri, "kalfa_rag__ara", &json!({}), &kilitli),
+        m
+    );
+}
```


- [ ] Kilitsiz araç, bütün çalışma alanını kilitleyen işi bekletmez;
  karşılaştırma için kilitli olanı bekletir:

`crates/kalfa-core/tests/oturum_kilitsiz.rs` (yeni):

```rust
//! Kilitsiz araç (aşama 4 tasarımı §8.3, karar 5): eklenti aracı çağrı
//! boyunca kilit tutmaz; aynı adımdaki, bütün çalışma alanını kilitleyen bir
//! iş (kabuk) onu beklemeden çalışır.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, Hazirlik};
use kalfa_core::izin::Kip;
use kalfa_core::oturum::OturumAyarlari;
use kalfa_core::protokol::{BitisNedeni, Komut};
use kalfa_core::test_destek::{
    SahteSaglayici, SahteYanit, sanal_siniflandirici, test_baglami, test_oturumu,
};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

/// Öteki aracın çalışmasını 3 sn'ye kadar bekler.
struct Bekleyen {
    kilitsiz: bool,
    isaret: Arc<AtomicBool>,
}

/// Bütün çalışma alanını kilitler (kabuk gibi) ve işareti koyar.
struct Genel {
    isaret: Arc<AtomicBool>,
}

#[async_trait]
impl Arac for Bekleyen {
    fn ad(&self) -> &str {
        "bekleyen"
    }
    fn aciklama(&self) -> &str {
        "bekler"
    }
    fn parametre_semasi(&self) -> Value {
        json!({"type": "object"})
    }
    fn yazar_mi(&self) -> bool {
        false
    }
    async fn hazirla(&self, _: &Value) -> Result<Hazirlik, AracHatasi> {
        Ok(Hazirlik {
            kilitsiz: self.kilitsiz,
            ..Default::default()
        })
    }
    async fn calistir(
        &self,
        _: &Value,
        _: &Hazirlik,
        _: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        for _ in 0..300 {
            if self.isaret.load(Ordering::SeqCst) {
                return Ok(AracCiktisi {
                    icerik: "öteki araç çalıştı".into(),
                    ..Default::default()
                });
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Err(AracHatasi::Gecersiz("öteki araç beklendi".into()))
    }
}

#[async_trait]
impl Arac for Genel {
    fn ad(&self) -> &str {
        "genel"
    }
    fn aciklama(&self) -> &str {
        "bütün alanı kilitler"
    }
    fn parametre_semasi(&self) -> Value {
        json!({"type": "object"})
    }
    fn yazar_mi(&self) -> bool {
        false
    }
    async fn hazirla(&self, _: &Value) -> Result<Hazirlik, AracHatasi> {
        Ok(Hazirlik {
            tum_calisma_alani: true,
            ..Default::default()
        })
    }
    async fn calistir(
        &self,
        _: &Value,
        _: &Hazirlik,
        _: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.isaret.store(true, Ordering::SeqCst);
        Ok(AracCiktisi::default())
    }
}

async fn calistir(kilitsiz: bool) -> String {
    let isaret = Arc::new(AtomicBool::new(false));
    let araclar: Vec<Arc<dyn Arac>> = vec![
        Arc::new(Bekleyen {
            kilitsiz,
            isaret: isaret.clone(),
        }),
        Arc::new(Genel { isaret }),
    ];
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![("bekleyen", json!({})), ("genel", json!({}))]),
        SahteYanit::metin("tamam"),
    ]);
    let mut ayarlar = OturumAyarlari::yeni("glm-5.3", &std::env::temp_dir());
    ayarlar.kip = Kip::Serbest;
    let mut d = test_oturumu(
        test_baglami(s.clone(), araclar, sanal_siniflandirici()),
        ayarlar,
    )
    .zaman_asimi(Duration::from_secs(10));
    d.gonder(Komut::KullaniciMesaji { metin: "x".into() }).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    format!("{:?}", s.istekler())
}

#[tokio::test]
async fn kilitsiz_arac_tum_alani_kilitleyeni_bekletmez() {
    let istekler = calistir(true).await;
    assert!(istekler.contains("öteki araç çalıştı"), "{istekler}");
}

/// Karşılaştırma: kilit tutan araç genel okuma kilidini çağrı boyunca
/// tutar; bütün alanı kilitleyen iş onun bitmesini bekler.
#[tokio::test]
async fn kilitli_arac_tum_alani_kilitleyeni_bekletir() {
    let istekler = calistir(false).await;
    assert!(istekler.contains("öteki araç beklendi"), "{istekler}");
}
```


- [ ] Koşturun, düşsün: `cargo test -p kalfa-core --test arac_muhur --test
  oturum_kilitsiz` (derlenmez: `EklentiTarifi` ve `kilitsiz` yok).
- [ ] Uygulama:

`crates/kalfa-core/src/arac.rs`:

```diff
diff --git a/crates/kalfa-core/src/arac.rs b/crates/kalfa-core/src/arac.rs
index f4a97cc..85f8471 100644
--- a/crates/kalfa-core/src/arac.rs
+++ b/crates/kalfa-core/src/arac.rs
@@ -83,6 +83,26 @@ pub struct Hazirlik {
     pub kabuk: Option<KabukTarifi>,
     /// Onay kartında gösterilen veri çıkışı türü (aşama 3b, tasarım §9.3).
     pub veri_cikisi: Option<VeriCikisi>,
+    /// Eklenti aracının tarifi (aşama 4 tasarımı §8.1). Mühre girer: onaydan
+    /// sonra güncellenen eklentinin çağrısı çalışmaz.
+    pub eklenti: Option<EklentiTarifi>,
+    /// Araç çağrı boyunca kilit tutmaz; dosyaya kendisi dokunurken o yolun
+    /// kilidini kısa süre alır (aşama 4 tasarımı §8.3, karar 5). Uzun bir
+    /// eklenti çağrısı aynı çalışma alanındaki komutları bekletmez.
+    pub kilitsiz: bool,
+}
+
+/// Eklenti aracının kimliği ve yetenekleri (aşama 4 tasarımı §8.1, §8.3).
+#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
+pub struct EklentiTarifi {
+    pub ad: String,
+    pub surum: String,
+    /// `.wasm`'ın SHA-256 özeti.
+    pub ozet: String,
+    /// Manifestin `dosya-oku` globları.
+    pub globlar: Vec<String>,
+    /// Manifestin `evren` uçları.
+    pub evren_uclari: Vec<String>,
 }
 
 #[derive(Debug, Clone, Default, PartialEq, Eq)]
```


`crates/kalfa-core/src/muhur.rs`:

```diff
diff --git a/crates/kalfa-core/src/muhur.rs b/crates/kalfa-core/src/muhur.rs
index cc6a32a..8f4289a 100644
--- a/crates/kalfa-core/src/muhur.rs
+++ b/crates/kalfa-core/src/muhur.rs
@@ -145,5 +145,18 @@ pub fn muhur_hesapla(
             serde_json::to_string(kabuk).unwrap_or_default().as_bytes(),
         );
     }
+    // Eklenti: sürüm, özet, globlar ve uçlar (aşama 4 tasarımı §8.3).
+    if let Some(eklenti) = &hazirlik.eklenti {
+        alan(&mut h, b"e");
+        alan(
+            &mut h,
+            serde_json::to_string(eklenti)
+                .unwrap_or_default()
+                .as_bytes(),
+        );
+    }
+    if hazirlik.kilitsiz {
+        alan(&mut h, b"kilitsiz");
+    }
     OnayMuhru(hex::encode(h.finalize()))
 }
```


`crates/kalfa-core/src/protokol.rs`:

```diff
diff --git a/crates/kalfa-core/src/protokol.rs b/crates/kalfa-core/src/protokol.rs
index 6067779..e32ff0b 100644
--- a/crates/kalfa-core/src/protokol.rs
+++ b/crates/kalfa-core/src/protokol.rs
@@ -138,6 +138,8 @@ pub enum VeriCikisi {
     AramaSonuclari,
     DosyaAdlari,
     YalnizcaYazildi,
+    /// Eklentinin ürettiği metin (aşama 4 tasarımı §8.1).
+    EklentiCiktisi,
 }
 
 /// Kabuk onayında gösterilen bağlam: komut, modelin onaysız yazdığı kodu
```


`crates/kalfa-core/src/oturum/arac_asamasi.rs`:

```diff
diff --git a/crates/kalfa-core/src/oturum/arac_asamasi.rs b/crates/kalfa-core/src/oturum/arac_asamasi.rs
index 6795617..2b89c6e 100644
--- a/crates/kalfa-core/src/oturum/arac_asamasi.rs
+++ b/crates/kalfa-core/src/oturum/arac_asamasi.rs
@@ -1389,9 +1389,14 @@ async fn tek_calistir(
     cikti: CiktiGonderici,
 ) -> Result<AracCiktisi, AracHatasi> {
     let yazilan: Vec<CalismaYolu> = p.hazirlik.yazilan.iter().map(|h| h.yol.clone()).collect();
-    let _kilit = tokio::select! {
-        k = kilitler.al(&p.hazirlik.okunan, &yazilan, p.hazirlik.tum_calisma_alani) => k,
-        _ = iptal.cancelled() => return Err(AracHatasi::Iptal),
+    // Kilitsiz araç (eklenti) kilidini dokunduğu her yol için kendisi alır.
+    let _kilit = if p.hazirlik.kilitsiz {
+        None
+    } else {
+        Some(tokio::select! {
+            k = kilitler.al(&p.hazirlik.okunan, &yazilan, p.hazirlik.tum_calisma_alani) => k,
+            _ = iptal.cancelled() => return Err(AracHatasi::Iptal),
+        })
     };
     // İzin kipi planlamadan sonra değişti ise yeni kipte yeniden
     // yetkilendirilmeden işlem yapılmaz.
```


`crates/kalfa-core/src/izin.rs`:

```diff
diff --git a/crates/kalfa-core/src/izin.rs b/crates/kalfa-core/src/izin.rs
index bac1f52..0f45a37 100644
--- a/crates/kalfa-core/src/izin.rs
+++ b/crates/kalfa-core/src/izin.rs
@@ -649,7 +649,8 @@ impl TopluSiniflandirici<'_> {
 }
 
 /// Her derinlikteki `.git` ya da listedeki bir korumalı yolun altı.
-fn korumali_listede(yol: &CalismaYolu, korumali: &[String]) -> bool {
+/// `korumali`: [`git_koruma::korumali_yollar`]'ın sonucu.
+pub fn korumali_listede(yol: &CalismaYolu, korumali: &[String]) -> bool {
     if git_koruma::git_ic_yolu_mu(yol) {
         return true;
     }
```


- [ ] `cargo test -p kalfa-core --locked` geçer. `Hazirlik`'i alan alan
  kuran başka yer yoktur (hepsi `..Default::default()`); yeni alanlar
  varsayılanla dolar.

### Görev 5: `kalfa-tools` — dosya bilgisi (`araçlar:`)

`crates/kalfa-tools/src/calisma_alani.rs`:

```diff
diff --git a/crates/kalfa-tools/src/calisma_alani.rs b/crates/kalfa-tools/src/calisma_alani.rs
index 5ac2d04..135977c 100644
--- a/crates/kalfa-tools/src/calisma_alani.rs
+++ b/crates/kalfa-tools/src/calisma_alani.rs
@@ -317,6 +317,14 @@ impl CalismaAlani {
         })
     }
 
+    /// Dosyanın boyutu ve son değişme zamanı; dosya okumanın yolundan açılır
+    /// (bağlantı izlenmez, yalnızca normal dosya). Eklentinin dosya listesi
+    /// için (aşama 4 tasarımı §6.3).
+    pub fn dosya_bilgisi(&self, yol: &CalismaYolu) -> io::Result<(u64, std::time::SystemTime)> {
+        let bilgi = self.dosya_ac(yol)?.metadata()?;
+        Ok((bilgi.len(), bilgi.modified()?.into_std()))
+    }
+
     /// Hard link sayısı; dosya yoksa 1.
     pub fn baglanti_sayisi(&self, yol: &CalismaYolu) -> io::Result<u64> {
         match self.dosya_ac(yol) {
```


- [ ] `cargo test -p kalfa-tools --locked` geçer. Yöntemin testi Görev 8'deki
  dosya listesi testidir: boyut ve milisaniye zamanı.

### Görev 6: `kalfa-plugin` — manifest (`eklenti:`)

`crates/kalfa-plugin/Cargo.toml` (yeni):

```toml
[package]
name = "kalfa-plugin"
description = "Kalfa eklenti çalışma zamanı: manifest, yükleme, wasmtime host'u, sınırlar, eklenti aracı"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true

[features]
# Testlerin import denetimini atlayan yükleme yolu (aşama 4 tasarımı §7.3,
# ikinci katman). Sürüm derlemesine girmez.
test-destek = []

[dependencies]
async-trait.workspace = true
globset.workspace = true
hex.workspace = true
jsonschema.workspace = true
kalfa-core = { path = "../kalfa-core" }
kalfa-tools = { path = "../kalfa-tools" }
semver.workspace = true
serde.workspace = true
serde_json.workspace = true
sha2.workspace = true
thiserror.workspace = true
tokio.workspace = true
tokio-util.workspace = true
toml.workspace = true
wasmtime.workspace = true
wasmtime-wasi.workspace = true

[dev-dependencies]
kalfa-core = { path = "../kalfa-core", features = ["test-destek"] }
kalfa-plugin = { path = ".", features = ["test-destek"] }
tempfile.workspace = true
tokio = { workspace = true, features = ["test-util"] }
```


- [ ] `src/lib.rs` önce yalnızca `pub mod manifest;` ile; Görev 7'de son
  hâlini alır.

`crates/kalfa-plugin/src/manifest.rs` (yeni):

```rust
//! `kalfa.toml` ve ad kuralları (aşama 4 tasarımı §6.4, §6.5).
//!
//! Bilinmeyen alan reddedilir: `kabuk = true` ya da `ag = [...]` yazan bir
//! manifest, yetki aldığını sanan bir eklenti üretmez.

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::Deserialize;

/// Kalfa'nın yüklediği tek sözleşme sürümü.
pub const SOZLESME: &str = "0.1";
pub const AZAMI_GLOB: usize = 64;
pub const AZAMI_DEPO_MB: u32 = 1024;
pub const AZAMI_ACIKLAMA: usize = 200;

const EKLENTI_ALANLARI: &[&str] = &["ad", "surum", "sozlesme", "aciklama"];
const YETENEK_ALANLARI: &[&str] = &["dosya-oku", "evren", "depo-mb"];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManifestHatasi {
    #[error("kalfa.toml okunamadı: {0}")]
    Bicim(String),
    #[error(
        "Bu eklenti Kalfa eklenti sözleşmesinin {0} sürümü için yazılmış. Bu Kalfa {SOZLESME}'i destekliyor. Eklentinin {SOZLESME} için derlenmiş sürümünü kurun ya da Kalfa'yı güncelleyin."
    )]
    Sozlesme(String),
    #[error(
        "kalfa.toml'daki `{0}` alanı Kalfa eklenti sözleşmesi {SOZLESME}'de yok. Eklentiler bu sürümde dosya yazamaz, komut çalıştıramaz ve ağa doğrudan çıkamaz."
    )]
    BilinmeyenAlan(String),
    #[error(
        "Eklenti adı `{0}` geçersiz: küçük harfle başlamalı, 2–24 karakter olmalı, yalnızca küçük harf, rakam ve `-` içermeli; `--` içeremez."
    )]
    Ad(String),
    #[error("Eklenti sürümü `{0}` geçersiz; `1.2.3` biçiminde olmalı.")]
    Surum(String),
    #[error("Eklenti açıklaması en çok {AZAMI_ACIKLAMA} karakter olabilir.")]
    Aciklama,
    #[error("`dosya-oku` en çok {AZAMI_GLOB} kalıp içerebilir.")]
    CokGlob,
    #[error("`dosya-oku` kalıbı `{0}` geçersiz: {1}")]
    Glob(String, String),
    #[error("`evren` yalnızca `embeddings` ve `rerank` içerebilir; `{0}` yok.")]
    EvrenUcu(String),
    #[error("`depo-mb` 0 ile {AZAMI_DEPO_MB} arasında olmalı; {0} verildi.")]
    DepoMb(i64),
}

/// Eklentiye açılabilecek EVREN uçları (aşama 4 tasarımı §2.2, karar 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvrenUcu {
    Embeddings,
    Rerank,
}

impl EvrenUcu {
    pub fn ad(self) -> &'static str {
        match self {
            EvrenUcu::Embeddings => "embeddings",
            EvrenUcu::Rerank => "rerank",
        }
    }
}

#[derive(Deserialize)]
struct Ham {
    eklenti: HamBilgi,
    #[serde(default)]
    yetenekler: HamYetenekler,
}

#[derive(Deserialize)]
struct HamBilgi {
    ad: String,
    surum: String,
    aciklama: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct HamYetenekler {
    #[serde(default)]
    dosya_oku: Vec<String>,
    #[serde(default)]
    evren: Vec<String>,
    #[serde(default)]
    depo_mb: i64,
}

#[derive(Debug, Clone)]
pub struct Manifest {
    pub ad: String,
    pub surum: semver::Version,
    pub aciklama: String,
    /// `dosya-oku` kalıpları, yazıldığı gibi.
    pub dosya_oku: Vec<String>,
    pub evren: Vec<EvrenUcu>,
    pub depo_mb: u32,
    globlar: GlobSet,
}

impl Manifest {
    pub fn coz(metin: &str) -> Result<Self, ManifestHatasi> {
        let tablo: toml::Table =
            toml::from_str(metin).map_err(|e| ManifestHatasi::Bicim(e.message().to_string()))?;
        // Sözleşme önce: 0.2 için yazılmış bir manifestin yeni alanları
        // "bilinmeyen alan" değil sürüm uyumsuzluğu iletisi almalı.
        let sozlesme = tablo
            .get("eklenti")
            .and_then(|e| e.get("sozlesme"))
            .and_then(|s| s.as_str())
            .ok_or_else(|| ManifestHatasi::Bicim("`[eklenti]` içinde `sozlesme` yok".into()))?;
        if sozlesme != SOZLESME {
            return Err(ManifestHatasi::Sozlesme(sozlesme.to_string()));
        }
        bilinmeyen_alan(&tablo)?;
        let ham: Ham = tablo
            .try_into()
            .map_err(|e: toml::de::Error| ManifestHatasi::Bicim(e.message().to_string()))?;

        let b = ham.eklenti;
        if !eklenti_adi_gecerli(&b.ad) {
            return Err(ManifestHatasi::Ad(b.ad));
        }
        let surum =
            semver::Version::parse(&b.surum).map_err(|_| ManifestHatasi::Surum(b.surum.clone()))?;
        if b.aciklama.chars().count() > AZAMI_ACIKLAMA {
            return Err(ManifestHatasi::Aciklama);
        }

        let y = ham.yetenekler;
        if y.dosya_oku.len() > AZAMI_GLOB {
            return Err(ManifestHatasi::CokGlob);
        }
        let mut kurucu = GlobSetBuilder::new();
        for kalip in &y.dosya_oku {
            kurucu.add(glob_kur(kalip)?);
        }
        let globlar = kurucu
            .build()
            .map_err(|e| ManifestHatasi::Glob(String::new(), e.to_string()))?;
        let mut evren = Vec::new();
        for uc in &y.evren {
            let uc = match uc.as_str() {
                "embeddings" => EvrenUcu::Embeddings,
                "rerank" => EvrenUcu::Rerank,
                _ => return Err(ManifestHatasi::EvrenUcu(uc.clone())),
            };
            if !evren.contains(&uc) {
                evren.push(uc);
            }
        }
        let depo_mb = u32::try_from(y.depo_mb)
            .ok()
            .filter(|mb| *mb <= AZAMI_DEPO_MB)
            .ok_or(ManifestHatasi::DepoMb(y.depo_mb))?;

        Ok(Self {
            ad: b.ad,
            surum,
            aciklama: b.aciklama,
            dosya_oku: y.dosya_oku,
            evren,
            depo_mb,
            globlar,
        })
    }

    /// Çalışma alanına göre yol manifestin `dosya-oku` globlarından birine
    /// uyuyor mu?
    pub fn okuyabilir(&self, yol: &str) -> bool {
        self.globlar.is_match(yol)
    }

    /// Modelin gördüğü araç adı (§6.5): `kalfa-rag` + `anlamsal_ara` →
    /// `kalfa_rag__anlamsal_ara`.
    pub fn model_adi(&self, arac: &str) -> String {
        format!("{}__{arac}", self.ad.replace('-', "_"))
    }
}

/// Sözleşme 0.1'de olmayan her alan reddedilir.
fn bilinmeyen_alan(tablo: &toml::Table) -> Result<(), ManifestHatasi> {
    for (anahtar, deger) in tablo {
        let izinli = match anahtar.as_str() {
            "eklenti" => EKLENTI_ALANLARI,
            "yetenekler" => YETENEK_ALANLARI,
            _ => return Err(ManifestHatasi::BilinmeyenAlan(anahtar.clone())),
        };
        let Some(alt) = deger.as_table() else {
            return Err(ManifestHatasi::Bicim(format!(
                "`{anahtar}` bir tablo olmalı"
            )));
        };
        if let Some(alan) = alt.keys().find(|k| !izinli.contains(&k.as_str())) {
            return Err(ManifestHatasi::BilinmeyenAlan(format!("{anahtar}.{alan}")));
        }
    }
    Ok(())
}

/// Kalıp çalışma alanının köküne göredir: mutlak yol, `..`, sürücü harfi ve
/// ters bölü reddedilir. `*` yol ayracını geçmez; her derinlik için `**/`.
fn glob_kur(kalip: &str) -> Result<globset::Glob, ManifestHatasi> {
    let hata = |neden: &str| ManifestHatasi::Glob(kalip.to_string(), neden.to_string());
    if kalip.contains('\\') {
        return Err(hata("yol ayracı olarak `/` kullanın"));
    }
    kalfa_core::izin::kalip_dogrula(kalip, false).map_err(|e| hata(&e.to_string()))?;
    GlobBuilder::new(kalip.trim())
        .literal_separator(true)
        .build()
        .map_err(|e| hata(&e.kind().to_string()))
}

/// `^[a-z][a-z0-9-]{1,23}$`, `--` içermez.
pub fn eklenti_adi_gecerli(ad: &str) -> bool {
    let mut k = ad.chars();
    (2..=24).contains(&ad.len())
        && k.next().is_some_and(|c| c.is_ascii_lowercase())
        && k.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !ad.contains("--")
}

/// `^[a-z][a-z0-9_]{0,31}$`, `__` içermez. Yerleşik araçların adında `__`
/// olmadığı için modelin gördüğü ad onlarla çakışamaz.
pub fn arac_adi_gecerli(ad: &str) -> bool {
    let mut k = ad.chars();
    (1..=32).contains(&ad.len())
        && k.next().is_some_and(|c| c.is_ascii_lowercase())
        && k.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !ad.contains("__")
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAM: &str = r#"
[eklenti]
ad = "kalfa-rag"
surum = "0.1.0"
sozlesme = "0.1"
aciklama = "Çalışma alanında Türkçe anlamsal arama"

[yetenekler]
dosya-oku = ["**/*.{md,rs,ts}", "docs/**"]
evren = ["embeddings", "rerank"]
depo-mb = 200
"#;

    fn hata(metin: &str) -> ManifestHatasi {
        Manifest::coz(metin).unwrap_err()
    }

    #[test]
    fn tam_manifest_cozulur() {
        let m = Manifest::coz(TAM).unwrap();
        assert_eq!(m.ad, "kalfa-rag");
        assert_eq!(m.surum, semver::Version::new(0, 1, 0));
        assert_eq!(m.evren, vec![EvrenUcu::Embeddings, EvrenUcu::Rerank]);
        assert_eq!(m.depo_mb, 200);
        assert_eq!(m.model_adi("anlamsal_ara"), "kalfa_rag__anlamsal_ara");
    }

    #[test]
    fn yeteneksiz_manifest_yalnizca_hesap_yapar() {
        let m = Manifest::coz(
            "[eklenti]\nad = \"hesap\"\nsurum = \"1.0.0\"\nsozlesme = \"0.1\"\naciklama = \"x\"\n",
        )
        .unwrap();
        assert!(m.dosya_oku.is_empty() && m.evren.is_empty() && m.depo_mb == 0);
        assert!(!m.okuyabilir("a.rs"));
    }

    #[test]
    fn globlar_kokten_ve_ayraci_gecmeden_eslesir() {
        let m = Manifest::coz(TAM).unwrap();
        assert!(m.okuyabilir("a.rs"));
        assert!(m.okuyabilir("src/derin/b.md"));
        assert!(m.okuyabilir("docs/x/y.txt"));
        assert!(!m.okuyabilir("a.py"));
        assert!(!m.okuyabilir("alt/docs/y.txt"));
    }

    #[test]
    fn baska_sozlesme_surumu_ileti_alir() {
        let h = hata(&TAM.replace("sozlesme = \"0.1\"", "sozlesme = \"0.2\"\nkabuk = true"));
        assert_eq!(h, ManifestHatasi::Sozlesme("0.2".into()));
        assert!(h.to_string().contains("0.2 sürümü için yazılmış"));
    }

    #[test]
    fn bilinmeyen_alan_reddedilir() {
        let h = hata(&TAM.replace("depo-mb = 200", "depo-mb = 200\nkabuk = true"));
        assert_eq!(h, ManifestHatasi::BilinmeyenAlan("yetenekler.kabuk".into()));
        assert!(h.to_string().contains("komut çalıştıramaz"));
        assert_eq!(
            hata(&format!("{TAM}\n[ag]\nhost = \"x\"\n")),
            ManifestHatasi::BilinmeyenAlan("ag".into())
        );
        assert!(matches!(
            hata(&TAM.replace("aciklama =", "yazar = \"x\"\naciklama =")),
            ManifestHatasi::BilinmeyenAlan(a) if a == "eklenti.yazar"
        ));
    }

    #[test]
    fn ad_kurallari() {
        for ad in [
            "a",
            "Kalfa",
            "1abc",
            "kalfa--rag",
            "kalfa_rag",
            &"a".repeat(25),
        ] {
            assert!(!eklenti_adi_gecerli(ad), "{ad}");
        }
        for ad in ["ab", "kalfa-rag", "x9", &"a".repeat(24)] {
            assert!(eklenti_adi_gecerli(ad), "{ad}");
        }
        for ad in ["", "Ara", "1a", "a__b", "a-b", &"a".repeat(33)] {
            assert!(!arac_adi_gecerli(ad), "{ad}");
        }
        for ad in ["a", "anlamsal_ara", "a1", &"a".repeat(32)] {
            assert!(arac_adi_gecerli(ad), "{ad}");
        }
        assert!(matches!(
            hata(&TAM.replace("kalfa-rag", "Kalfa")),
            ManifestHatasi::Ad(_)
        ));
    }

    #[test]
    fn surum_aciklama_ve_depo_denetlenir() {
        assert!(matches!(
            hata(&TAM.replace("\"0.1.0\"", "\"1.0\"")),
            ManifestHatasi::Surum(_)
        ));
        assert_eq!(
            hata(&TAM.replace("Çalışma alanında", &"x".repeat(201))),
            ManifestHatasi::Aciklama
        );
        assert_eq!(
            hata(&TAM.replace("200", "2000")),
            ManifestHatasi::DepoMb(2000)
        );
        assert_eq!(hata(&TAM.replace("200", "-1")), ManifestHatasi::DepoMb(-1));
        assert_eq!(
            hata(&TAM.replace("\"rerank\"", "\"chat\"")),
            ManifestHatasi::EvrenUcu("chat".into())
        );
    }

    #[test]
    fn zararli_globlar_reddedilir() {
        for kalip in ["/etc/*", "../x/**", "C:/x/**", "src\\\\*.rs", ""] {
            let m = TAM.replace("\"docs/**\"", &format!("\"{kalip}\""));
            assert!(
                matches!(hata(&m), ManifestHatasi::Glob(..)),
                "{kalip}: {:?}",
                Manifest::coz(&m).map(|_| ())
            );
        }
        let cok: Vec<String> = (0..65).map(|i| format!("\"d{i}/**\"")).collect();
        let m = TAM.replace(
            "[\"**/*.{md,rs,ts}\", \"docs/**\"]",
            &format!("[{}]", cok.join(",")),
        );
        assert_eq!(hata(&m), ManifestHatasi::CokGlob);
    }
}
```


- [ ] `cargo test -p kalfa-plugin --lib --locked`: manifest testleri geçer
  (8 test; import izin listesinin birim testi Görev 7'de gelir).

### Görev 7: `kalfa-plugin` — motor, host, depo, eklenti aracı (`eklenti:`)

- [ ] Önce testlerin düzeneği ve yükleme ile çağrı testleri:

`crates/kalfa-plugin/tests/ortak/mod.rs` (yeni):

```rust
//! `kalfa-plugin` testlerinin düzeneği: test eklentilerinin derlenmesi, bir
//! çalışma alanı ve depo, aracın çağrılması.
#![allow(dead_code)]

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, CiktiGonderici, CiktiParcasi};
use kalfa_core::izin::Siniflandirici;
use kalfa_core::kilit::KaynakKilitleri;
use kalfa_core::kimlik::CagriId;
use kalfa_core::protokol::CiktiAkisi;
use kalfa_plugin::sinir::Sureler;
use kalfa_plugin::{Eklenti, EklentiAraci, Motor};
use kalfa_tools::CalismaAlani;
use serde_json::Value;
use tempfile::TempDir;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Test eklentilerini `wasm32-wasip2` için derler (aşama 4 tasarımı §17.1,
/// G7). Test ikilisi başına bir kez; derlenmişse cargo hiçbir şey yapmaz.
/// Testler `.wasm`'ları kendileri derlediği için `cargo test` tek başına
/// yeterlidir ve eski bir `.wasm`'la koşmaz.
fn derle() -> Result<PathBuf, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let cikti = Command::new(cargo)
        .args([
            "build",
            "--target",
            "wasm32-wasip2",
            "--locked",
            "--message-format=json-render-diagnostics",
            "-p",
            "kalfa-test-eklentisi",
            "-p",
            "kalfa-test-soket",
            "-p",
            "kalfa-test-yanlis-dunya",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("cargo çalıştırılamadı: {e}"))?;
    if !cikti.status.success() {
        return Err(format!(
            "Test eklentileri derlenemedi. `rustup target add wasm32-wasip2` gerekebilir. Durum: {}",
            cikti.status
        ));
    }
    String::from_utf8_lossy(&cikti.stdout)
        .lines()
        .filter_map(|s| serde_json::from_str::<Value>(s).ok())
        .filter(|m| m["reason"] == "compiler-artifact")
        .flat_map(|m| {
            m["filenames"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
        })
        .filter_map(|f| f.as_str().map(PathBuf::from))
        .find(|f| f.extension().is_some_and(|e| e == "wasm"))
        .and_then(|f| f.parent().map(PathBuf::from))
        .ok_or_else(|| "cargo çıktısında .wasm yok".into())
}

/// `crate_adi` test eklentisinin `.wasm`'ı.
pub fn wasm(crate_adi: &str) -> Vec<u8> {
    static DIZIN: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    let dizin = match DIZIN.get_or_init(derle) {
        Ok(d) => d,
        Err(e) => panic!("{e}"),
    };
    std::fs::read(dizin.join(format!("{}.wasm", crate_adi.replace('-', "_"))))
        .unwrap_or_else(|e| panic!("{crate_adi}.wasm okunamadı: {e}"))
}

/// `kalfa-test` adlı eklentinin manifesti.
pub fn manifest(dosya_oku: &[&str], evren: &[&str], depo_mb: u32) -> String {
    let liste = |l: &[&str]| {
        l.iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "[eklenti]\nad = \"kalfa-test\"\nsurum = \"0.1.0\"\nsozlesme = \"0.1\"\naciklama = \"Test eklentisi\"\n\n[yetenekler]\ndosya-oku = [{}]\nevren = [{}]\ndepo-mb = {depo_mb}\n",
        liste(dosya_oku),
        liste(evren)
    )
}

/// Testler için kısa süreler: CPU bütçesi 2 sn, toplam 20 sn.
pub fn motor() -> Arc<Motor> {
    motor_cpu(Duration::from_secs(2))
}

pub fn motor_cpu(cpu: Duration) -> Arc<Motor> {
    Motor::test_sureleri(Sureler {
        cpu,
        toplam: Duration::from_secs(20),
    })
    .unwrap()
}

/// Bir çağrının sonucu ve canlı çıktısı.
pub struct Cagri {
    pub sonuc: Result<AracCiktisi, AracHatasi>,
    pub cikti: Vec<(CiktiAkisi, String)>,
}

impl Cagri {
    pub fn icerik(&self) -> &str {
        match &self.sonuc {
            Ok(c) => &c.icerik,
            Err(e) => panic!("araç başarısız: {e}"),
        }
    }

    pub fn hata(&self) -> String {
        match &self.sonuc {
            Ok(c) => panic!("araç başarılı: {}", c.icerik),
            Err(e) => e.to_string(),
        }
    }

    pub fn cikti_metni(&self) -> String {
        self.cikti.iter().map(|(_, m)| m.as_str()).collect()
    }
}

pub struct Duzenek {
    pub alan: TempDir,
    pub depo: TempDir,
    pub motor: Arc<Motor>,
    pub eklenti: Arc<Eklenti>,
    pub calisma_alani: Arc<CalismaAlani>,
    pub kilitler: Arc<KaynakKilitleri>,
}

impl Duzenek {
    pub async fn yeni(manifest: &str) -> Self {
        Self::alanla(manifest, tempfile::tempdir().unwrap()).await
    }

    pub async fn alanla(manifest: &str, alan: TempDir) -> Self {
        Self::kur(manifest, alan, motor()).await
    }

    /// CPU bütçesi verilen motorla.
    pub async fn cpu(manifest: &str, cpu: Duration) -> Self {
        Self::kur(manifest, tempfile::tempdir().unwrap(), motor_cpu(cpu)).await
    }

    async fn kur(manifest: &str, alan: TempDir, motor: Arc<Motor>) -> Self {
        let eklenti = Arc::new(
            motor
                .yukle(manifest, wasm("kalfa-test-eklentisi"))
                .await
                .unwrap(),
        );
        let siniflandirici = Arc::new(Siniflandirici::yeni(alan.path(), &[]).unwrap());
        Self {
            calisma_alani: CalismaAlani::ac(siniflandirici).unwrap(),
            alan,
            depo: tempfile::tempdir().unwrap(),
            motor,
            eklenti,
            kilitler: Arc::new(KaynakKilitleri::yeni()),
        }
    }

    /// Çalışma alanına bir dosya koyar; üst dizinler oluşturulur.
    pub fn dosya(&self, yol: &str, icerik: &[u8]) -> &Self {
        let tam = self.alan.path().join(yol);
        std::fs::create_dir_all(tam.parent().unwrap()).unwrap();
        std::fs::write(tam, icerik).unwrap();
        self
    }

    /// Bu çalışma alanının, `depo_dizini`'ni kullanan araçları.
    pub fn araclar_depoyla(&self, depo_dizini: &std::path::Path) -> Vec<Arc<dyn Arac>> {
        EklentiAraci::hepsi(
            &self.motor,
            &self.eklenti,
            &self.calisma_alani,
            &self.kilitler,
            depo_dizini,
        )
    }

    pub fn arac(&self, ad: &str) -> Arc<dyn Arac> {
        self.arac_depoyla(ad, self.depo.path())
    }

    pub fn arac_depoyla(&self, ad: &str, depo: &std::path::Path) -> Arc<dyn Arac> {
        let model_adi = format!("kalfa_test__{ad}");
        self.araclar_depoyla(depo)
            .into_iter()
            .find(|a| a.ad() == model_adi)
            .unwrap_or_else(|| panic!("{model_adi} yok"))
    }

    pub async fn cagir(&self, ad: &str, argumanlar: Value) -> Cagri {
        self.cagir_iptalle(ad, argumanlar, CancellationToken::new())
            .await
    }

    pub async fn cagir_iptalle(
        &self,
        ad: &str,
        argumanlar: Value,
        iptal: CancellationToken,
    ) -> Cagri {
        cagir(&self.arac(ad), argumanlar, iptal).await
    }
}

pub async fn cagir(arac: &Arc<dyn Arac>, argumanlar: Value, iptal: CancellationToken) -> Cagri {
    let (tx, mut rx) = mpsc::channel::<CiktiParcasi>(1024);
    let hazirlik = arac.hazirla(&argumanlar).await.unwrap();
    let sonuc = arac
        .calistir_akisli(
            &argumanlar,
            &hazirlik,
            iptal,
            CiktiGonderici::yeni(CagriId::yeni(), tx),
        )
        .await;
    let mut cikti = Vec::new();
    while let Ok(p) = rx.try_recv() {
        cikti.push((p.akis, p.metin));
    }
    Cagri { sonuc, cikti }
}
```


`crates/kalfa-plugin/tests/yukleme.rs` (yeni):

```rust
//! Yükleme (aşama 4 tasarımı §7.2, §7.3): sözleşmeye uyan bileşen yüklenir,
//! uymayan ve izinsiz import eden yüklenmez.

mod ortak;

use std::io::Read;
use std::time::Duration;

use kalfa_plugin::YuklemeHatasi;
use kalfa_plugin::manifest::ManifestHatasi;
use ortak::*;
use serde_json::json;

#[tokio::test]
async fn sozlesmeye_uyan_eklenti_yuklenir_ve_araclari_sunulur() {
    let d = Duzenek::yeni(&manifest(&[], &[], 0)).await;
    let adlar: Vec<&str> = d.eklenti.araclar.iter().map(|a| a.ad.as_str()).collect();
    assert!(
        adlar.contains(&"yanki") && adlar.contains(&"dongu"),
        "{adlar:?}"
    );
    assert_eq!(d.arac("yanki").ad(), "kalfa_test__yanki");
    assert_eq!(d.eklenti.ozet.len(), 64);
}

#[tokio::test]
async fn soket_import_eden_bilesen_yuklenmez() {
    let h = motor()
        .yukle(&manifest(&[], &[], 0), wasm("kalfa-test-soket"))
        .await
        .unwrap_err();
    assert!(
        matches!(&h, YuklemeHatasi::Import(ad) if ad.starts_with("wasi:sockets/")),
        "{h:?}"
    );
    let ileti = h.to_string();
    assert!(
        ileti.contains("ağ erişimi") && ileti.contains("evren-cagir"),
        "{ileti}"
    );
}

/// İkinci katman (§7.3): import denetimi atlansa bile `WasiCtx` ağsızdır;
/// eklenti bu makinede dinleyen bir sokete bağlanamaz.
#[tokio::test]
async fn import_denetimi_atlansa_da_soket_baglanamaz() {
    let dinleyici = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    dinleyici.set_nonblocking(true).unwrap();
    let adres = dinleyici.local_addr().unwrap().to_string();
    let motor = motor();
    let eklenti = std::sync::Arc::new(
        motor
            .yukle_import_denetimsiz(&manifest(&[], &[], 0), wasm("kalfa-test-soket"))
            .await
            .unwrap(),
    );
    let alan = tempfile::tempdir().unwrap();
    let siniflandirici =
        std::sync::Arc::new(kalfa_core::izin::Siniflandirici::yeni(alan.path(), &[]).unwrap());
    let araclar = kalfa_plugin::EklentiAraci::hepsi(
        &motor,
        &eklenti,
        &kalfa_tools::CalismaAlani::ac(siniflandirici).unwrap(),
        &std::sync::Arc::new(kalfa_core::kilit::KaynakKilitleri::yeni()),
        alan.path(),
    );
    let c = cagir(
        &araclar[0],
        json!({"adres": adres}),
        tokio_util::sync::CancellationToken::new(),
    )
    .await;
    assert!(c.icerik().starts_with("bağlanamadı"), "{}", c.icerik());
    tokio::time::sleep(Duration::from_millis(100)).await;
    match dinleyici.accept() {
        Ok((mut s, _)) => {
            let mut b = Vec::new();
            let _ = s.read_to_end(&mut b);
            panic!("eklenti bağlandı");
        }
        Err(e) => assert_eq!(e.kind(), std::io::ErrorKind::WouldBlock),
    }
}

#[tokio::test]
async fn sozlesmeye_uymayan_dunya_yuklenmez() {
    let h = motor()
        .yukle(&manifest(&[], &[], 0), wasm("kalfa-test-yanlis-dunya"))
        .await
        .unwrap_err();
    assert!(matches!(h, YuklemeHatasi::Dunya(_)), "{h:?}");
    assert!(h.to_string().contains("kalfa:plugin@0.1.0"));
}

#[tokio::test]
async fn cekirdek_modul_ve_bozuk_dosya_yuklenmez() {
    let m = manifest(&[], &[], 0);
    let h = motor()
        .yukle(&m, b"\0asm\x01\0\0\0".to_vec())
        .await
        .unwrap_err();
    assert!(h.to_string().contains("çekirdek modül"), "{h}");
    let h = motor().yukle(&m, b"bozuk".to_vec()).await.unwrap_err();
    assert!(matches!(h, YuklemeHatasi::BilesenDegil(_)), "{h:?}");
}

#[tokio::test]
async fn baska_sozlesme_surumunun_manifesti_yuklenmez() {
    let m = manifest(&[], &[], 0).replace("sozlesme = \"0.1\"", "sozlesme = \"0.2\"");
    let h = motor()
        .yukle(&m, wasm("kalfa-test-eklentisi"))
        .await
        .unwrap_err();
    assert!(
        matches!(&h, YuklemeHatasi::Manifest(ManifestHatasi::Sozlesme(s)) if s == "0.2"),
        "{h:?}"
    );
}
```


`crates/kalfa-plugin/tests/calisma.rs` (yeni):

```rust
//! Çağrı ve kaynak sınırları (aşama 4 tasarımı §7.1, §7.4–§7.6).

mod ortak;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use kalfa_core::arac::AracHatasi;
use kalfa_core::protokol::CiktiAkisi;
use ortak::*;
use serde_json::json;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn temel_yol_sonuc_gunluk_ve_ilerleme() {
    let d = Duzenek::yeni(&manifest(&[], &[], 0)).await;
    let c = d.cagir("yanki", json!({"metin": "merhaba"})).await;
    assert_eq!(c.icerik(), "merhaba");
    let Ok(cikti) = &c.sonuc else { unreachable!() };
    assert_eq!(cikti.kaynak, "eklenti:kalfa-test:yanki");
    assert!(!cikti.kalfa_uretimi);
    let canli = c.cikti_metni();
    assert!(canli.contains("[bilgi] yanki çağrıldı"), "{canli}");
    assert!(canli.contains("[ilerleme] 1/1 · bitti"), "{canli}");
}

#[tokio::test]
async fn hazirlik_eklentiyi_tarif_eder_ve_kilitsizdir() {
    let d = Duzenek::yeni(&manifest(&["**/*.rs"], &["embeddings"], 0)).await;
    let h = d.arac("yanki").hazirla(&json!({})).await.unwrap();
    assert!(h.kilitsiz);
    assert!(h.okunan.is_empty() && h.yazilan.is_empty() && !h.tum_calisma_alani);
    let t = h.eklenti.unwrap();
    assert_eq!(t.ad, "kalfa-test");
    assert_eq!(t.ozet, d.eklenti.ozet);
    assert_eq!(t.globlar, vec!["**/*.rs"]);
    assert_eq!(t.evren_uclari, vec!["embeddings"]);
    assert!(
        h.aciklama
            .contains("kalfa-test eklentisinin `yanki` aracını")
    );
}

/// CPU bütçesinde durur (§7.4, G3). Tek iş parçacıklı çalışma zamanında
/// başka bir görev ilerlemeye devam eder: eklenti her tikte yer verir.
#[tokio::test(flavor = "current_thread")]
async fn sonsuz_dongu_cpu_butcesinde_durur_ve_yer_verir() {
    let d = Duzenek::yeni(&manifest(&[], &[], 0)).await;
    let sayac = Arc::new(AtomicU64::new(0));
    let s = sayac.clone();
    let gorev = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(10)).await;
            s.fetch_add(1, Ordering::SeqCst);
        }
    });
    let bas = Instant::now();
    let c = d.cagir("dongu", json!({})).await;
    let sure = bas.elapsed();
    gorev.abort();
    assert!(c.hata().contains("işlem süresi sınırı"), "{}", c.hata());
    // Bütçe 2 sn (test süreleri).
    assert!(
        sure >= Duration::from_millis(1800) && sure < Duration::from_secs(10),
        "{sure:?}"
    );
    let ilerleme = sayac.load(Ordering::SeqCst);
    assert!(ilerleme > 50, "öteki görev ilerlemedi: {ilerleme}");
    // Sonraki çağrı temiz bir örnekle çalışır.
    assert_eq!(d.cagir("yanki", json!({"metin": "x"})).await.icerik(), "x");
}

#[tokio::test]
async fn bellek_siniri_cagriyi_durdurur_sonraki_calisir() {
    let d = Duzenek::yeni(&manifest(&[], &[], 0)).await;
    let c = d.cagir("bellek", json!({})).await;
    assert!(c.hata().contains("bellek sınırı (512 MiB)"), "{}", c.hata());
    assert_eq!(d.cagir("yanki", json!({"metin": "x"})).await.icerik(), "x");
}

#[tokio::test]
async fn sisen_sonuc_kirpilir_gunluk_sinirlanir() {
    let d = Duzenek::yeni(&manifest(&[], &[], 0)).await;
    let c = d.cagir("sisirme", json!({})).await;
    let icerik = c.icerik();
    assert!(icerik.len() < 64 * 1024 + 200, "{}", icerik.len());
    assert!(icerik.contains("bayta kırpıldı"));
    let canli = c.cikti_metni();
    assert_eq!(canli.matches("[ayrıntı]").count(), 200);
    assert!(canli.contains("[günlük] 100 satır"), "{canli}");
}

#[tokio::test]
async fn panik_tuzak_olur_stderr_yalnizca_canli_ciktida() {
    let d = Duzenek::yeni(&manifest(&[], &[], 0)).await;
    let c = d.cagir("panik", json!({})).await;
    let hata = c.hata();
    assert!(hata.contains("çalışırken çöktü"), "{hata}");
    assert!(!hata.contains("bilerek panik"), "{hata}");
    let stderr: String = c
        .cikti
        .iter()
        .filter(|(a, _)| *a == CiktiAkisi::Hata)
        .map(|(_, m)| m.as_str())
        .collect();
    assert!(stderr.contains("bilerek panik"), "{stderr}");
    assert!(stderr.contains("stderr satırı"), "{stderr}");
}

#[tokio::test]
async fn eklenti_hatasi_tur_ve_iletiyle_doner() {
    let d = Duzenek::yeni(&manifest(&[], &[], 0)).await;
    let c = d.cagir("dosya", json!({"islem": "yok"})).await;
    assert_eq!(
        c.hata(),
        "Eklenti hatası (kalfa-test): diger: bilinmeyen işlem: yok"
    );
}

#[tokio::test]
async fn iptale_uyan_eklenti_hemen_cikar() {
    let d = Duzenek::yeni(&manifest(&[], &[], 0)).await;
    let iptal = CancellationToken::new();
    let i = iptal.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        i.cancel();
    });
    let bas = Instant::now();
    let c = d.cagir_iptalle("iptal", json!({}), iptal).await;
    assert!(matches!(c.sonuc, Err(AracHatasi::Iptal)), "{:?}", c.sonuc);
    assert!(
        bas.elapsed() < Duration::from_millis(1500),
        "{:?}",
        bas.elapsed()
    );
}

/// İptali yok sayan eklenti 2 sn sonra bırakılır (§7.5).
#[tokio::test]
async fn iptali_yok_sayan_eklenti_iki_saniyede_birakilir() {
    let d = Duzenek::cpu(&manifest(&[], &[], 0), Duration::from_secs(20)).await;
    let iptal = CancellationToken::new();
    let i = iptal.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        i.cancel();
    });
    let bas = Instant::now();
    let c = d
        .cagir_iptalle("iptal", json!({"inatci": true}), iptal)
        .await;
    let sure = bas.elapsed();
    assert!(matches!(c.sonuc, Err(AracHatasi::Iptal)), "{:?}", c.sonuc);
    assert!(
        sure >= Duration::from_millis(2100) && sure < Duration::from_millis(2900),
        "{sure:?}"
    );
}

/// G3: host çağrısında beklenen süre CPU bütçesine sayılmaz. Dosyanın yazma
/// kilidi bütçeden (2 sn) uzun tutulur; eklentinin okuması kilidi bekler ve
/// yine de başarılı olur. Kilit bekleyen okuma kabukla eklentinin aynı
/// dosyada birbirinin ortasına girmediğini de gösterir (§8.3).
#[tokio::test]
async fn host_cagrisinda_beklenen_sure_cpu_butcesine_sayilmaz() {
    let d = Duzenek::yeni(&manifest(&["**/*"], &[], 0)).await;
    d.dosya("a.rs", b"fn a() {}\n");
    let yol = kalfa_core::yol::CalismaYolu::coz("a.rs").unwrap();
    let kilit = d.kilitler.al(&[], &[yol], false).await;
    let birak = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(3)).await;
        drop(kilit);
    });
    let bas = Instant::now();
    let c = d
        .cagir("dosya", json!({"islem": "oku", "yol": "a.rs"}))
        .await;
    assert_eq!(c.icerik(), "fn a() {}\n");
    assert!(
        bas.elapsed() >= Duration::from_millis(2900),
        "{:?}",
        bas.elapsed()
    );
    birak.await.unwrap();
}
```


- [ ] Uygulama:

`crates/kalfa-plugin/src/lib.rs` (yeni):

```rust
//! Kalfa eklenti çalışma zamanı (aşama 4 tasarımı §5–§8).
//!
//! - [`manifest`]: `kalfa.toml` ve ad kuralları.
//! - [`Motor`]: wasmtime motoru; eklentiyi yükler, her çağrıya yeni örnek.
//! - [`EklentiAraci`]: eklentinin aracı, `kalfa-core`'un `Arac` trait'iyle.
//!
//! Eklenti Kalfa'nın sürecinde çalışır. Sınır, WebAssembly'nin bellek
//! yalıtımı ve yeteneksiz WASI'dir: ağ yok, ön açık dizin yok. Dosyalara
//! yalnızca host fonksiyonlarıyla, yerleşik okumanın yolundan erişir.

mod arac;
mod bag;
mod calistirici;
pub mod depo;
mod host;
pub mod manifest;
pub mod sinir;

pub use arac::EklentiAraci;
pub use calistirici::{AracBilgisi, Eklenti, Motor, YuklemeHatasi};
```


`crates/kalfa-plugin/src/sinir.rs` (yeni):

```rust
//! Kaynak sınırları (aşama 4 tasarımı §7.4). v1'de uygulama sabitleridir
//! (karar 10); manifestle değişen tek sınır depo kotasıdır.

use std::time::Duration;

/// Doğrusal bellek, örnek başına.
pub const BELLEK: usize = 512 * 1024 * 1024;
pub const TABLO_OGESI: usize = 100_000;
pub const ORNEK: usize = 16;
pub const BELLEK_SAYISI: usize = 4;
pub const TABLO: usize = 64;
/// Epoch tikinin aralığı.
pub const TIK: Duration = Duration::from_millis(10);
/// WebAssembly yürütme süresi, çağrı başına (60 sn = 6 000 tik).
pub const CPU: Duration = Duration::from_secs(60);
/// Çağrının toplam süresi, host çağrıları dahil.
pub const TOPLAM: Duration = Duration::from_secs(30 * 60);
/// `araclari-listele`.
pub const LISTE_SURESI: Duration = Duration::from_secs(5);
/// İptalden sonra eklentiye çıkması için verilen süre (§7.5).
pub const IPTAL_SURESI: Duration = Duration::from_secs(2);
/// `dosya-listele` sonucu.
pub const LISTE: usize = 100_000;
/// `dosya-oku`: tek dosya.
pub const DOSYA: u64 = 4 * 1024 * 1024;
/// `dosya-oku`: çağrı başına toplam.
pub const CAGRI_OKUMASI: u64 = 1024 * 1024 * 1024;
pub const GUNLUK_SATIRI: u32 = 200;
pub const GUNLUK_BAYTI: usize = 4 * 1024;
/// `sonuc.icerik`.
pub const SONUC: usize = 64 * 1024;
pub const HATA_ILETISI: usize = 4 * 1024;
/// `stderr`'in tutulan sonu.
pub const STDERR: usize = 4 * 1024;
/// Uygulama genelinde aynı anda çalışan eklenti çağrısı.
pub const ESZAMANLI: usize = 4;
pub const ARAC_ACIKLAMASI: usize = 1_000;
pub const SEMA: usize = 16 * 1024;

/// Çağrının süre sınırları. Ürün yolu hep [`Sureler::default`]'u kullanır;
/// testler kısaltabilir (`test-destek`).
#[derive(Debug, Clone, Copy)]
pub struct Sureler {
    pub cpu: Duration,
    pub toplam: Duration,
}

impl Default for Sureler {
    fn default() -> Self {
        Self {
            cpu: CPU,
            toplam: TOPLAM,
        }
    }
}

impl Sureler {
    pub(crate) fn cpu_tik(&self) -> u64 {
        u64::try_from(self.cpu.as_millis() / TIK.as_millis()).unwrap_or(u64::MAX)
    }
}
```


`crates/kalfa-plugin/src/bag.rs` (yeni):

```rust
//! WIT 0.1'in host tarafı bağları (`kalfa-plugin-sdk/wit/kalfa.wit`).

wasmtime::component::bindgen!({
    world: "eklenti",
    path: "../kalfa-plugin-sdk/wit",
    imports: { default: async },
    exports: { default: async },
});
```


`crates/kalfa-plugin/src/calistirici.rs` (yeni):

```rust
//! wasmtime motoru, eklentinin yüklenmesi ve bir aracın çağrılması (aşama 4
//! tasarımı §7).
//!
//! Uygulamada tek bir motor vardır; epoch'u bir iş parçacığı 10 ms'de bir
//! ilerletir. Her eklenti bir kez derlenir; her çağrı yeni bir `Store` ve
//! örnekle yapılır.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};

use serde_json::Value;
use tokio::sync::Semaphore;
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Config, Engine, Store, UpdateDeadline};

use crate::bag::kalfa::plugin::tipler::Hata;
use crate::bag::{Eklenti as Dunya, EklentiPre};
use crate::host::{CagriOrtami, Durum};
use crate::manifest::{Manifest, ManifestHatasi, arac_adi_gecerli};
use crate::sinir::{self, Sureler};

#[derive(Debug, thiserror::Error)]
pub enum YuklemeHatasi {
    #[error(transparent)]
    Manifest(#[from] ManifestHatasi),
    #[error("Eklenti çalışma zamanı başlatılamadı: {0}")]
    Motor(String),
    #[error(
        "`.wasm` bir WebAssembly bileşeni değil{0}. Eklentiyi `wasm32-wasip2` hedefi için derleyin."
    )]
    BilesenDegil(&'static str),
    #[error("Bileşen derlenemedi: {0}")]
    Derleme(String),
    #[error("{}", import_iletisi(.0))]
    Import(String),
    #[error("Bileşen Kalfa eklenti sözleşmesine (kalfa:plugin@0.1.0) uymuyor: {0}")]
    Dunya(String),
    #[error("Eklentinin araç listesi alınamadı: {0}")]
    AracListesi(String),
    #[error("Eklentinin `{arac}` aracı geçersiz: {neden}")]
    Arac { arac: String, neden: String },
}

fn import_iletisi(ad: &str) -> String {
    if ad.starts_with("wasi:sockets/") || ad.starts_with("wasi:http/") {
        format!(
            "Bu eklenti ağ erişimi (`{ad}`) istiyor. Kalfa eklentileri ağa doğrudan çıkamaz; EVREN'e yalnızca `evren-cagir` ile ulaşabilir."
        )
    } else {
        format!("Bu eklenti Kalfa'nın sunmadığı bir arayüzü (`{ad}`) istiyor; yüklenmedi.")
    }
}

/// İzinli importlar (§7.3): Kalfa'nın host'u ve WASI 0.2'nin ağsız kısmı.
/// Dosya sistemi bağlanır ama ön açık dizin yoktur.
fn izinli_import(ad: &str) -> bool {
    let (arayuz, surum) = ad.split_once('@').unwrap_or((ad, ""));
    let paket = arayuz.split('/').next().unwrap_or_default();
    match paket {
        "kalfa:plugin" => {
            surum == "0.1.0" && matches!(arayuz, "kalfa:plugin/tipler" | "kalfa:plugin/host")
        }
        "wasi:io" | "wasi:clocks" | "wasi:random" | "wasi:cli" | "wasi:filesystem" => {
            surum.starts_with("0.2.")
        }
        _ => false,
    }
}

/// Modele sunulan bir aracın tanımı.
#[derive(Debug, Clone)]
pub struct AracBilgisi {
    pub ad: String,
    pub aciklama: String,
    pub sema: Value,
}

/// Yüklenmiş, derlenmiş eklenti.
pub struct Eklenti {
    pub manifest: Arc<Manifest>,
    /// `.wasm`'ın SHA-256 özeti.
    pub ozet: String,
    pub araclar: Vec<AracBilgisi>,
    pre: EklentiPre<Durum>,
}

impl std::fmt::Debug for Eklenti {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Eklenti")
            .field("ad", &self.manifest.ad)
            .field("ozet", &self.ozet)
            .field("araclar", &self.araclar)
            .finish()
    }
}

/// Bir çağrının başarısızlığı (§7.6).
#[derive(Debug)]
pub(crate) enum CagriHatasi {
    /// Eklenti `Err(hata)` döndürdü.
    Eklenti(Hata),
    Bellek,
    Cpu,
    Sure,
    Iptal,
    /// Panik, `unreachable` ya da başka bir tuzak.
    Cokme,
}

pub struct Motor {
    engine: Engine,
    sureler: Sureler,
    eszamanli: Arc<Semaphore>,
    /// Eklenti × depo dizini başına sıra: aynı deponun iki yazarı olmaz.
    siralar: Mutex<HashMap<String, Weak<tokio::sync::Mutex<()>>>>,
    dur: Arc<AtomicBool>,
}

impl Drop for Motor {
    fn drop(&mut self) {
        self.dur.store(true, Ordering::SeqCst);
    }
}

impl Motor {
    pub fn yeni() -> Result<Arc<Self>, YuklemeHatasi> {
        Self::sureli(Sureler::default())
    }

    /// Kısaltılmış sürelerle motor; yalnızca testler için.
    #[cfg(feature = "test-destek")]
    pub fn test_sureleri(sureler: Sureler) -> Result<Arc<Self>, YuklemeHatasi> {
        Self::sureli(sureler)
    }

    fn sureli(sureler: Sureler) -> Result<Arc<Self>, YuklemeHatasi> {
        let mut config = Config::new();
        config.epoch_interruption(true);
        let engine = Engine::new(&config).map_err(|e| YuklemeHatasi::Motor(e.to_string()))?;
        let dur = Arc::new(AtomicBool::new(false));
        let (tikleyen, durdur) = (engine.clone(), dur.clone());
        std::thread::Builder::new()
            .name("kalfa-eklenti-epoch".into())
            .spawn(move || {
                while !durdur.load(Ordering::SeqCst) {
                    std::thread::sleep(sinir::TIK);
                    tikleyen.increment_epoch();
                }
            })
            .map_err(|e| YuklemeHatasi::Motor(e.to_string()))?;
        Ok(Arc::new(Self {
            engine,
            sureler,
            eszamanli: Arc::new(Semaphore::new(sinir::ESZAMANLI)),
            siralar: Mutex::default(),
            dur,
        }))
    }

    /// Bellek sınırı, epoch'la CPU bütçesi ve her tikte tokio'ya yer verme
    /// (§7.4). Bütçe yalnızca WebAssembly çalışırken tükenir; bir host
    /// çağrısında geçen süre en çok bir tik sayılır.
    fn store(&self, durum: Durum, cpu_tik: u64) -> Store<Durum> {
        let mut store = Store::new(&self.engine, durum);
        store.limiter(|d| &mut d.sinirlayici);
        store.set_epoch_deadline(1);
        store.epoch_deadline_callback(move |c| {
            let iz = &c.data().iz;
            if iz.cpu_tik.fetch_add(1, Ordering::SeqCst) + 1 > cpu_tik {
                iz.cpu.store(true, Ordering::SeqCst);
                return Err(wasmtime::format_err!("işlem süresi sınırı aşıldı"));
            }
            Ok(UpdateDeadline::YieldCustom(
                1,
                Box::pin(tokio::task::yield_now()),
            ))
        });
        store
    }

    fn sira(&self, anahtar: String) -> Arc<tokio::sync::Mutex<()>> {
        let mut siralar = self.siralar.lock().unwrap_or_else(|z| z.into_inner());
        siralar.retain(|_, s| s.strong_count() > 0);
        if let Some(s) = siralar.get(&anahtar).and_then(Weak::upgrade) {
            return s;
        }
        let s = Arc::new(tokio::sync::Mutex::new(()));
        siralar.insert(anahtar, Arc::downgrade(&s));
        s
    }

    /// Eklentiyi yükler (§7.2): manifest, bileşen, import denetimi, dünya,
    /// araç listesi.
    pub async fn yukle(&self, manifest: &str, wasm: Vec<u8>) -> Result<Eklenti, YuklemeHatasi> {
        self.yukle_ic(manifest, wasm, true).await
    }

    /// Import denetimini atlayan yükleme: ikinci katmanın (ağsız, ön açık
    /// dizinsiz WASI) testi için.
    #[cfg(feature = "test-destek")]
    pub async fn yukle_import_denetimsiz(
        &self,
        manifest: &str,
        wasm: Vec<u8>,
    ) -> Result<Eklenti, YuklemeHatasi> {
        self.yukle_ic(manifest, wasm, false).await
    }

    async fn yukle_ic(
        &self,
        manifest: &str,
        wasm: Vec<u8>,
        import_denetimi: bool,
    ) -> Result<Eklenti, YuklemeHatasi> {
        let manifest = Arc::new(Manifest::coz(manifest)?);
        let ozet = kalfa_core::muhur::ozet_hex(&wasm);
        // Bileşen sürüm 0x0d, katman 1; çekirdek modül sürüm 1, katman 0.
        if !wasm.starts_with(b"\0asm") {
            return Err(YuklemeHatasi::BilesenDegil(""));
        }
        if wasm.get(4..8) == Some(&[1, 0, 0, 0]) {
            return Err(YuklemeHatasi::BilesenDegil(" (çekirdek modül)"));
        }
        let engine = self.engine.clone();
        let component = tokio::task::spawn_blocking(move || Component::new(&engine, &wasm))
            .await
            .map_err(|e| YuklemeHatasi::Derleme(e.to_string()))?
            .map_err(|e| YuklemeHatasi::Derleme(format!("{e:#}")))?;
        if import_denetimi
            && let Some((ad, _)) = component
                .component_type()
                .imports(&self.engine)
                .find(|(ad, _)| !izinli_import(ad))
        {
            return Err(YuklemeHatasi::Import(ad.to_string()));
        }

        // İkinci katman: bütün WASI p2 bağlanır, ama `WasiCtx` ağsız ve ön
        // açık dizinsizdir (`Durum::yeni`).
        let mut linker: Linker<Durum> = Linker::new(&self.engine);
        wasmtime_wasi::p2::add_to_linker_async(&mut linker)
            .map_err(|e| YuklemeHatasi::Motor(format!("{e:#}")))?;
        Dunya::add_to_linker::<Durum, HasSelf<Durum>>(&mut linker, |d| d)
            .map_err(|e| YuklemeHatasi::Motor(format!("{e:#}")))?;
        let pre = linker
            .instantiate_pre(&component)
            .and_then(EklentiPre::new)
            .map_err(|e| YuklemeHatasi::Dunya(format!("{e:#}")))?;

        let cpu_tik = Sureler {
            cpu: sinir::LISTE_SURESI,
            ..self.sureler
        }
        .cpu_tik();
        let mut store = self.store(Durum::yeni(None), cpu_tik);
        let liste = tokio::time::timeout(sinir::LISTE_SURESI, async {
            // Export'ların tipi örneklemede denetlenir: dünyaya uymayan
            // bileşen burada düşer.
            let ornek = pre
                .instantiate_async(&mut store)
                .await
                .map_err(|e| YuklemeHatasi::Dunya(format!("{e:#}")))?;
            ornek
                .kalfa_plugin_arac()
                .call_araclari_listele(&mut store)
                .await
                .map_err(|e| YuklemeHatasi::AracListesi(format!("{e:#}")))
        })
        .await
        .map_err(|_| YuklemeHatasi::AracListesi("5 sn içinde yanıt vermedi".into()))??;
        let araclar = araclari_dogrula(liste)?;
        Ok(Eklenti {
            manifest,
            ozet,
            araclar,
            pre,
        })
    }
}

fn araclari_dogrula(
    liste: Vec<crate::bag::exports::kalfa::plugin::arac::AracTanimi>,
) -> Result<Vec<AracBilgisi>, YuklemeHatasi> {
    if liste.is_empty() {
        return Err(YuklemeHatasi::AracListesi(
            "eklenti hiç araç sunmuyor".into(),
        ));
    }
    let mut araclar: Vec<AracBilgisi> = Vec::with_capacity(liste.len());
    for t in liste {
        let hata = |neden: &str| YuklemeHatasi::Arac {
            arac: kalfa_core::arac::kirp(&t.ad, 64),
            neden: neden.to_string(),
        };
        if !arac_adi_gecerli(&t.ad) {
            return Err(hata(
                "ad küçük harfle başlamalı, en çok 32 karakter olmalı, yalnızca küçük harf, rakam ve `_` içermeli; `__` içeremez",
            ));
        }
        if araclar.iter().any(|a| a.ad == t.ad) {
            return Err(hata("aynı adla iki araç var"));
        }
        // Açıklama ve şema güvenilmeyen içeriktir (§6.5).
        if t.aciklama.chars().count() > sinir::ARAC_ACIKLAMASI {
            return Err(hata("açıklama en çok 1 000 karakter olabilir"));
        }
        if t.parametre_semasi.len() > sinir::SEMA {
            return Err(hata("parametre şeması en çok 16 KiB olabilir"));
        }
        let sema: Value = serde_json::from_str(&t.parametre_semasi)
            .map_err(|e| hata(&format!("parametre şeması JSON değil: {e}")))?;
        if !sema.is_object() {
            return Err(hata("parametre şeması bir JSON nesnesi olmalı"));
        }
        jsonschema::validator_for(&sema)
            .map_err(|e| hata(&format!("parametre şeması geçersiz: {e}")))?;
        araclar.push(AracBilgisi {
            ad: t.ad,
            aciklama: t.aciklama,
            sema,
        });
    }
    Ok(araclar)
}

impl Eklenti {
    /// Bir aracı yeni bir örnekte çağırır. İkinci değer `stderr`'in sonudur.
    pub(crate) async fn calistir(
        &self,
        motor: &Motor,
        arac: &str,
        argumanlar: &str,
        ortam: CagriOrtami,
    ) -> (Result<String, CagriHatasi>, String) {
        let iptal = ortam.iptal.clone();
        let sira = motor.sira(format!(
            "{}\u{0}{}",
            self.manifest.ad,
            ortam.depo.dizin().display()
        ));
        let _sira = tokio::select! {
            s = sira.lock_owned() => s,
            _ = iptal.cancelled() => return (Err(CagriHatasi::Iptal), String::new()),
        };
        let _izin = tokio::select! {
            i = motor.eszamanli.clone().acquire_owned() => i,
            _ = iptal.cancelled() => return (Err(CagriHatasi::Iptal), String::new()),
        };
        let durum = Durum::yeni(Some(ortam));
        let iz = durum.iz.clone();
        let mut store = motor.store(durum, motor.sureler.cpu_tik());
        let pre = self.pre.clone();
        // Store çağrıyla birlikte taşınır; future bırakılınca örnek de atılır.
        let cagri = async move {
            let ornek = pre.instantiate_async(&mut store).await?;
            let sonuc = ornek
                .kalfa_plugin_arac()
                .call_calistir(&mut store, arac, argumanlar)
                .await;
            if let Some(c) = &store.data().cagri {
                c.bitir();
            }
            sonuc
        };
        // İptalden sonra eklentiye çıkması için süre verilir (§7.5).
        let iptal_bekle = async {
            iptal.cancelled().await;
            tokio::time::sleep(sinir::IPTAL_SURESI).await;
        };
        let sonuc = tokio::select! {
            r = tokio::time::timeout(motor.sureler.toplam, cagri) => r,
            () = iptal_bekle => return (Err(CagriHatasi::Iptal), iz.stderr.metin()),
        };
        let stderr = iz.stderr.metin();
        let sonuc = match sonuc {
            Err(_) => Err(CagriHatasi::Sure),
            Ok(Ok(Ok(s))) => Ok(s.icerik),
            Ok(Ok(Err(Hata::IptalEdildi))) if iptal.is_cancelled() => Err(CagriHatasi::Iptal),
            Ok(Ok(Err(h))) => Err(CagriHatasi::Eklenti(h)),
            Ok(Err(_)) if iz.bellek.load(Ordering::SeqCst) => Err(CagriHatasi::Bellek),
            Ok(Err(_)) if iz.cpu.load(Ordering::SeqCst) => Err(CagriHatasi::Cpu),
            Ok(Err(_)) if iptal.is_cancelled() => Err(CagriHatasi::Iptal),
            Ok(Err(_)) => Err(CagriHatasi::Cokme),
        };
        (sonuc, stderr)
    }
}

#[cfg(test)]
mod tests {
    use super::izinli_import;

    #[test]
    fn import_izin_listesi() {
        for ad in [
            "kalfa:plugin/host@0.1.0",
            "kalfa:plugin/tipler@0.1.0",
            "wasi:io/streams@0.2.6",
            "wasi:cli/stderr@0.2.0",
            "wasi:filesystem/preopens@0.2.3",
            "wasi:clocks/wall-clock@0.2.6",
            "wasi:random/random@0.2.6",
        ] {
            assert!(izinli_import(ad), "{ad}");
        }
        for ad in [
            "wasi:sockets/tcp@0.2.6",
            "wasi:http/outgoing-handler@0.2.6",
            "wasi:io/streams@0.3.0",
            "wasi:io/streams",
            "kalfa:plugin/host@0.2.0",
            "kalfa:plugin/arac@0.1.0",
            "baska:paket/x@1.0.0",
            "wasi:iox/streams@0.2.0",
        ] {
            assert!(!izinli_import(ad), "{ad}");
        }
    }
}
```


`crates/kalfa-plugin/src/host.rs` (yeni):

```rust
//! Eklentinin çağırdığı host fonksiyonları ve bir çağrının `Store` durumu
//! (aşama 4 tasarımı §6.3, §7.3, §7.4, §8.3).

use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant, UNIX_EPOCH};

use globset::GlobBuilder;
use kalfa_core::arac::{AracHatasi, CiktiGonderici};
use kalfa_core::kilit::KaynakKilitleri;
use kalfa_core::protokol::CiktiAkisi;
use kalfa_core::yol::CalismaYolu;
use kalfa_tools::CalismaAlani;
use tokio_util::sync::CancellationToken;
use wasmtime::component::ResourceTable;
use wasmtime_wasi::cli::{IsTerminal, StdoutStream};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use crate::bag::kalfa::plugin::tipler::{DosyaBilgisi, EvrenUcu, Hata, Ilerleme, Seviye};
use crate::depo::Depo;
use crate::manifest::Manifest;
use crate::sinir;

/// Bir çağrının izi: tuzağın nedeni ve `stderr`'in sonu. `Store` atıldıktan
/// sonra da okunur.
#[derive(Default)]
pub(crate) struct Iz {
    pub bellek: AtomicBool,
    pub cpu: AtomicBool,
    pub cpu_tik: AtomicU64,
    pub stderr: SonBaytlar,
}

/// Bellek, tablo ve örnek sınırları (§7.4). Sınırı aşan büyüme tuzak kurar.
pub(crate) struct Sinirlayici {
    iz: Arc<Iz>,
}

impl wasmtime::ResourceLimiter for Sinirlayici {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        if desired > sinir::BELLEK {
            self.iz.bellek.store(true, Ordering::SeqCst);
            return Err(wasmtime::format_err!("bellek sınırı aşıldı"));
        }
        Ok(true)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        if desired > sinir::TABLO_OGESI {
            return Err(wasmtime::format_err!("tablo sınırı aşıldı"));
        }
        Ok(true)
    }

    fn instances(&self) -> usize {
        sinir::ORNEK
    }

    fn tables(&self) -> usize {
        sinir::TABLO
    }

    fn memories(&self) -> usize {
        sinir::BELLEK_SAYISI
    }
}

/// `stderr`'in son `STDERR` baytı (§7.3). Dolunca eskiler atılır; eklenti
/// yazarken tuzağa düşmez.
#[derive(Clone, Default)]
pub(crate) struct SonBaytlar(Arc<Mutex<VecDeque<u8>>>);

impl SonBaytlar {
    fn ekle(&self, bayt: &[u8]) {
        let mut ic = self.0.lock().unwrap_or_else(|z| z.into_inner());
        ic.extend(bayt);
        let fazla = ic.len().saturating_sub(sinir::STDERR);
        ic.drain(..fazla);
    }

    pub fn metin(&self) -> String {
        let ic = self.0.lock().unwrap_or_else(|z| z.into_inner());
        String::from_utf8_lossy(&ic.iter().copied().collect::<Vec<_>>()).into_owned()
    }
}

impl IsTerminal for SonBaytlar {
    fn is_terminal(&self) -> bool {
        false
    }
}

impl StdoutStream for SonBaytlar {
    fn async_stream(&self) -> Box<dyn tokio::io::AsyncWrite + Send + Sync> {
        Box::new(self.clone())
    }
}

impl tokio::io::AsyncWrite for SonBaytlar {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.ekle(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// Bir araç çağrısının host tarafı: çalışma alanı, depo, iptal ve canlı çıktı.
pub(crate) struct CagriOrtami {
    pub manifest: Arc<Manifest>,
    pub alan: Arc<CalismaAlani>,
    pub kilitler: Arc<KaynakKilitleri>,
    pub depo: Depo,
    pub iptal: CancellationToken,
    pub cikti: CiktiGonderici,
    /// Git'in korumalı yolları; ilk dosya çağrısında bir kez çözülür.
    korumali: Option<Vec<String>>,
    okunan_bayt: u64,
    gunluk_satiri: u32,
    gunluk_atilan: u32,
    son_ilerleme: Option<Instant>,
}

impl CagriOrtami {
    pub fn yeni(
        manifest: Arc<Manifest>,
        alan: Arc<CalismaAlani>,
        kilitler: Arc<KaynakKilitleri>,
        depo: Depo,
        iptal: CancellationToken,
        cikti: CiktiGonderici,
    ) -> Self {
        Self {
            manifest,
            alan,
            kilitler,
            depo,
            iptal,
            cikti,
            korumali: None,
            okunan_bayt: 0,
            gunluk_satiri: 0,
            gunluk_atilan: 0,
            son_ilerleme: None,
        }
    }

    /// Çağrı bitti: atılan günlük satırları bildirilir.
    pub fn bitir(&self) {
        if self.gunluk_atilan > 0 {
            self.cikti.gonder(
                CiktiAkisi::Standart,
                format!(
                    "[günlük] {} satır sınırı aştığı için atıldı\n",
                    self.gunluk_atilan
                ),
            );
        }
    }

    async fn korumali(&mut self) -> Result<&[String], Hata> {
        if self.korumali.is_none() {
            let kok = self.alan.kok().to_path_buf();
            let k =
                tokio::task::spawn_blocking(move || kalfa_core::git_koruma::korumali_yollar(&kok))
                    .await
                    .map_err(|e| Hata::Diger(format!("git korumaları çözülemedi: {e}")))?;
            self.korumali = Some(k);
        }
        Ok(self.korumali.as_deref().unwrap_or_default())
    }

    fn dosya_yetenegi(&self) -> Result<(), Hata> {
        if self.manifest.dosya_oku.is_empty() {
            return Err(Hata::IzinYok("dosya-oku yeteneği manifestte yok".into()));
        }
        if self.iptal.is_cancelled() {
            return Err(Hata::IptalEdildi);
        }
        Ok(())
    }

    /// Eklentiye açık mı: manifest globu, gizli değil, `.git` ya da korumalı
    /// değil.
    async fn acik_mi(&mut self, yol: &CalismaYolu) -> Result<Result<(), Hata>, Hata> {
        if !self.manifest.okuyabilir(yol.as_str()) {
            return Ok(Err(Hata::IzinYok(format!(
                "`{yol}` manifestteki globlara uymuyor"
            ))));
        }
        if self.alan.siniflandirici().gizli_mi(yol) {
            return Ok(Err(Hata::IzinYok("gizli dosya".into())));
        }
        if kalfa_core::izin::korumali_listede(yol, self.korumali().await?) {
            return Ok(Err(Hata::IzinYok("korumalı yol".into())));
        }
        Ok(Ok(()))
    }

    async fn dosya_listele(&mut self, desen: String) -> Result<Vec<DosyaBilgisi>, Hata> {
        self.dosya_yetenegi()?;
        let eslestirici = GlobBuilder::new(&desen)
            .literal_separator(true)
            .build()
            .map_err(|e| Hata::Diger(format!("desen geçersiz: {e}")))?
            .compile_matcher();
        let alan = self.alan.clone();
        let iptal = self.iptal.clone();
        let dosyalar = tokio::task::spawn_blocking(move || alan.dosyalar(&iptal))
            .await
            .map_err(|e| Hata::Diger(e.to_string()))?
            .map_err(|e| match e {
                AracHatasi::Iptal => Hata::IptalEdildi,
                e => Hata::Diger(e.to_string()),
            })?;
        let mut acik = Vec::new();
        for yol in dosyalar {
            if eslestirici.is_match(yol.as_str()) && self.acik_mi(&yol).await?.is_ok() {
                if acik.len() == sinir::LISTE {
                    return Err(Hata::SinirAsildi(format!(
                        "dosya listesi {} kaydı aşıyor; deseni daraltın",
                        sinir::LISTE
                    )));
                }
                acik.push(yol);
            }
        }
        let alan = self.alan.clone();
        tokio::task::spawn_blocking(move || {
            acik.into_iter()
                // Tarama ile bilgi arasında silinen ya da değişen dosya atlanır.
                .filter_map(|yol| {
                    let (boyut, zaman) = alan.dosya_bilgisi(&yol).ok()?;
                    let degisme = zaman
                        .duration_since(UNIX_EPOCH)
                        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
                    Some(DosyaBilgisi {
                        yol: yol.as_str().to_string(),
                        boyut,
                        degisme,
                    })
                })
                .collect()
        })
        .await
        .map_err(|e| Hata::Diger(e.to_string()))
    }

    async fn dosya_oku(&mut self, yol: String) -> Result<String, Hata> {
        self.dosya_yetenegi()?;
        let yol = CalismaYolu::coz(&yol)
            .map_err(|e| Hata::IzinYok(format!("`{yol}` geçersiz yol: {e}")))?;
        self.acik_mi(&yol).await??;
        // Okuma süresince o yolun okuma kilidi ve genel okuma kilidi (§8.3).
        let _kilit = tokio::select! {
            k = self.kilitler.al(std::slice::from_ref(&yol), &[], false) => k,
            _ = self.iptal.cancelled() => return Err(Hata::IptalEdildi),
        };
        let alan = self.alan.clone();
        let iptal = self.iptal.clone();
        let okuma_yolu = yol.clone();
        let veri = tokio::task::spawn_blocking(move || {
            let (boyut, _) = alan.dosya_bilgisi(&okuma_yolu).map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Hata::Bulunamadi(okuma_yolu.to_string())
                } else {
                    Hata::IzinYok(format!("`{okuma_yolu}` okunamadı: {e}"))
                }
            })?;
            if boyut > sinir::DOSYA {
                return Err(Hata::SinirAsildi(format!(
                    "`{okuma_yolu}` 4 MiB okuma sınırını aşıyor"
                )));
            }
            alan.sinirli_oku(&okuma_yolu, &iptal).map_err(|e| match e {
                AracHatasi::Iptal => Hata::IptalEdildi,
                // Hard link'li dosya: içeriği başka bir yolla da paylaşılıyor.
                AracHatasi::OnayEskidi => Hata::IzinYok(format!(
                    "`{okuma_yolu}` başka bir yerde de adı olan (hard link) bir dosya"
                )),
                e => Hata::IzinYok(format!("`{okuma_yolu}` okunamadı: {e}")),
            })
        })
        .await
        .map_err(|e| Hata::Diger(e.to_string()))??;
        if veri.len() as u64 > sinir::DOSYA {
            return Err(Hata::SinirAsildi(format!(
                "`{yol}` 4 MiB okuma sınırını aşıyor"
            )));
        }
        self.okunan_bayt += veri.len() as u64;
        if self.okunan_bayt > sinir::CAGRI_OKUMASI {
            return Err(Hata::SinirAsildi(
                "bu çağrıda okunan dosyalar 1 GiB sınırını aştı".into(),
            ));
        }
        String::from_utf8(veri).map_err(|_| Hata::Diger(format!("`{yol}` metin değil")))
    }

    fn depo_yetenegi(&self) -> Result<(), Hata> {
        if self.manifest.depo_mb == 0 {
            return Err(Hata::IzinYok("depo yeteneği manifestte yok".into()));
        }
        if self.iptal.is_cancelled() {
            return Err(Hata::IptalEdildi);
        }
        Ok(())
    }

    fn evren_cagir(&mut self, uc: EvrenUcu) -> Result<String, Hata> {
        let uc = match uc {
            EvrenUcu::Embeddings => crate::manifest::EvrenUcu::Embeddings,
            EvrenUcu::Rerank => crate::manifest::EvrenUcu::Rerank,
        };
        if !self.manifest.evren.contains(&uc) {
            return Err(Hata::IzinYok(format!(
                "evren yeteneği manifestte yok: `{}`",
                uc.ad()
            )));
        }
        if self.iptal.is_cancelled() {
            return Err(Hata::IptalEdildi);
        }
        // ponytail: EVREN yolu aşama 4b'de bağlanır (tasarım §20).
        Err(Hata::IzinYok(
            "EVREN çağrısı bu Kalfa sürümünde henüz yok (aşama 4b)".into(),
        ))
    }

    fn gunluk(&mut self, duzey: Seviye, mesaj: String) {
        if self.gunluk_satiri >= sinir::GUNLUK_SATIRI {
            self.gunluk_atilan += 1;
            return;
        }
        self.gunluk_satiri += 1;
        let duzey = match duzey {
            Seviye::Hata => "hata",
            Seviye::Uyari => "uyarı",
            Seviye::Bilgi => "bilgi",
            Seviye::Ayrinti => "ayrıntı",
        };
        let mesaj = kalfa_core::arac::kirp(&mesaj, sinir::GUNLUK_BAYTI);
        self.cikti
            .gonder(CiktiAkisi::Standart, format!("[{duzey}] {mesaj}\n"));
    }

    /// Saniyede en çok 10 olay; arada gelenler atılır, sonraki olay güncel
    /// durumu taşır.
    fn ilerleme(&mut self, d: Ilerleme) {
        let simdi = Instant::now();
        if self
            .son_ilerleme
            .is_some_and(|s| simdi.duration_since(s) < Duration::from_millis(100))
        {
            return;
        }
        self.son_ilerleme = Some(simdi);
        let sayi = match d.toplam {
            Some(t) => format!("{}/{t}", d.tamamlanan),
            None => d.tamamlanan.to_string(),
        };
        let metin = match d.mesaj {
            Some(m) => format!("[ilerleme] {sayi} · {}", kalfa_core::arac::kirp(&m, 512)),
            None => format!("[ilerleme] {sayi}"),
        };
        self.cikti
            .gonder(CiktiAkisi::Standart, format!("{metin}\n"));
    }
}

/// Bir `Store`'un durumu. `cagri` yükleme sırasında (`araclari-listele`)
/// yoktur; o sırada host çağrıları reddedilir.
pub(crate) struct Durum {
    wasi: WasiCtx,
    tablo: ResourceTable,
    pub sinirlayici: Sinirlayici,
    pub iz: Arc<Iz>,
    pub cagri: Option<CagriOrtami>,
}

impl Durum {
    pub fn yeni(cagri: Option<CagriOrtami>) -> Self {
        let iz = Arc::new(Iz::default());
        // Ortam değişkeni, argüman ve ön açık dizin yok; ağ kapalı. `stdout`
        // atılır, `stderr`'in sonu tutulur (§7.3).
        let wasi = WasiCtx::builder()
            .stderr(iz.stderr.clone())
            .allow_tcp(false)
            .allow_udp(false)
            .allow_ip_name_lookup(false)
            .build();
        Self {
            wasi,
            tablo: ResourceTable::new(),
            sinirlayici: Sinirlayici { iz: iz.clone() },
            iz,
            cagri,
        }
    }

    fn cagri(&mut self) -> Result<&mut CagriOrtami, Hata> {
        self.cagri
            .as_mut()
            .ok_or_else(|| Hata::IzinYok("araç listesi alınırken host çağrısı yapılamaz".into()))
    }
}

impl WasiView for Durum {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.tablo,
        }
    }
}

impl crate::bag::kalfa::plugin::tipler::Host for Durum {}

impl crate::bag::kalfa::plugin::host::Host for Durum {
    async fn dosya_listele(&mut self, desen: String) -> Result<Vec<DosyaBilgisi>, Hata> {
        self.cagri()?.dosya_listele(desen).await
    }

    async fn dosya_oku(&mut self, yol: String) -> Result<String, Hata> {
        self.cagri()?.dosya_oku(yol).await
    }

    async fn evren_cagir(&mut self, uc: EvrenUcu, _govde: String) -> Result<String, Hata> {
        self.cagri()?.evren_cagir(uc)
    }

    async fn depo_oku(&mut self, anahtar: String) -> Result<Option<Vec<u8>>, Hata> {
        let c = self.cagri()?;
        c.depo_yetenegi()?;
        c.depo.oku(&anahtar).await
    }

    async fn depo_yaz(&mut self, anahtar: String, deger: Vec<u8>) -> Result<(), Hata> {
        let c = self.cagri()?;
        c.depo_yetenegi()?;
        c.depo.yaz(&anahtar, deger).await
    }

    async fn depo_sil(&mut self, anahtar: String) -> Result<(), Hata> {
        let c = self.cagri()?;
        c.depo_yetenegi()?;
        c.depo.sil(&anahtar).await
    }

    async fn ilerleme_bildir(&mut self, durum: Ilerleme) {
        if let Some(c) = self.cagri.as_mut() {
            c.ilerleme(durum);
        }
    }

    async fn iptal_istendi(&mut self) -> bool {
        self.cagri.as_ref().is_some_and(|c| c.iptal.is_cancelled())
    }

    async fn gunluk(&mut self, duzey: Seviye, mesaj: String) {
        if let Some(c) = self.cagri.as_mut() {
            c.gunluk(duzey, mesaj);
        }
    }
}
```


`crates/kalfa-plugin/src/depo.rs` (yeni):

```rust
//! Eklentiye ve çalışma alanına özel anahtar-değer deposu (aşama 4 tasarımı
//! §10).
//!
//! Her anahtar bir dosyadır; adı anahtarın SHA-256'sıdır. Yazma atomiktir:
//! aynı dizinde geçici dosya, sonra yeniden adlandırma. Kota, dizindeki
//! dosyaların toplam boyutudur; çağrının başında bir kez sayılır ve yazdıkça
//! güncellenir. Bir dizinin aynı anda tek yazarı vardır (§7.1).

use std::io;
use std::path::{Path, PathBuf};

use crate::bag::kalfa::plugin::tipler::Hata;

pub const AZAMI_ANAHTAR: usize = 200;
pub const AZAMI_DEGER: usize = 64 * 1024 * 1024;

pub(crate) struct Depo {
    dizin: PathBuf,
    /// Bayt.
    kota: u64,
    kota_mb: u32,
    /// Dizindeki dosyaların toplam boyutu; ilk kullanımda sayılır.
    kullanilan: Option<u64>,
}

fn dosya_adi(anahtar: &str) -> Result<String, Hata> {
    if anahtar.is_empty() || anahtar.len() > AZAMI_ANAHTAR {
        return Err(Hata::Diger(format!(
            "depo anahtarı 1–{AZAMI_ANAHTAR} bayt olmalı"
        )));
    }
    Ok(kalfa_core::muhur::ozet_hex(anahtar.as_bytes()))
}

fn yazilamadi(e: io::Error) -> Hata {
    Hata::Diger(format!("depo yazılamadı: {e}"))
}

/// Görev çalışamadıysa (çalışma zamanı kapanıyor) hata.
fn gorev_hatasi(e: tokio::task::JoinError) -> Hata {
    Hata::Diger(format!("depo işlemi tamamlanamadı: {e}"))
}

impl Depo {
    pub(crate) fn yeni(dizin: PathBuf, kota_mb: u32) -> Self {
        Self {
            dizin,
            kota: u64::from(kota_mb) * 1024 * 1024,
            kota_mb,
            kullanilan: None,
        }
    }

    pub(crate) fn dizin(&self) -> &Path {
        &self.dizin
    }

    async fn kullanilan(&mut self) -> Result<u64, Hata> {
        if let Some(k) = self.kullanilan {
            return Ok(k);
        }
        let dizin = self.dizin.clone();
        let k = tokio::task::spawn_blocking(move || dizin_boyutu(&dizin))
            .await
            .map_err(gorev_hatasi)?
            .map_err(|e| Hata::Diger(format!("depo okunamadı: {e}")))?;
        self.kullanilan = Some(k);
        Ok(k)
    }

    pub(crate) async fn oku(&mut self, anahtar: &str) -> Result<Option<Vec<u8>>, Hata> {
        let yol = self.dizin.join(dosya_adi(anahtar)?);
        tokio::task::spawn_blocking(move || match std::fs::read(&yol) {
            Ok(v) => Ok(Some(v)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Hata::Diger(format!("depo okunamadı: {e}"))),
        })
        .await
        .map_err(gorev_hatasi)?
    }

    pub(crate) async fn yaz(&mut self, anahtar: &str, deger: Vec<u8>) -> Result<(), Hata> {
        let ad = dosya_adi(anahtar)?;
        if deger.len() > AZAMI_DEGER {
            return Err(Hata::SinirAsildi(
                "depo değeri en çok 64 MiB olabilir".into(),
            ));
        }
        let kullanilan = self.kullanilan().await?;
        let yol = self.dizin.join(&ad);
        let eski = boyut(&yol).await?;
        let yeni = kullanilan - eski.min(kullanilan) + deger.len() as u64;
        if yeni > self.kota {
            return Err(Hata::SinirAsildi(format!(
                "depo kotası ({} MB) doldu",
                self.kota_mb
            )));
        }
        let dizin = self.dizin.clone();
        tokio::task::spawn_blocking(move || atomik_yaz(&dizin, &ad, &deger))
            .await
            .map_err(gorev_hatasi)?
            .map_err(yazilamadi)?;
        self.kullanilan = Some(yeni);
        Ok(())
    }

    pub(crate) async fn sil(&mut self, anahtar: &str) -> Result<(), Hata> {
        let kullanilan = self.kullanilan().await?;
        let yol = self.dizin.join(dosya_adi(anahtar)?);
        let eski = boyut(&yol).await?;
        tokio::task::spawn_blocking(move || match std::fs::remove_file(&yol) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(yazilamadi(e)),
            _ => Ok(()),
        })
        .await
        .map_err(gorev_hatasi)??;
        self.kullanilan = Some(kullanilan - eski.min(kullanilan));
        Ok(())
    }
}

async fn boyut(yol: &Path) -> Result<u64, Hata> {
    let yol = yol.to_path_buf();
    tokio::task::spawn_blocking(move || match std::fs::metadata(&yol) {
        Ok(m) => Ok(m.len()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(Hata::Diger(format!("depo okunamadı: {e}"))),
    })
    .await
    .map_err(gorev_hatasi)?
}

/// Dizindeki dosyaların toplam boyutu; dizin yoksa 0.
pub fn dizin_boyutu(dizin: &Path) -> io::Result<u64> {
    let girdiler = match std::fs::read_dir(dizin) {
        Ok(g) => g,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e),
    };
    let mut toplam = 0;
    for girdi in girdiler {
        let girdi = girdi?;
        if girdi.file_type()?.is_file() {
            toplam += girdi.metadata()?.len();
        }
    }
    Ok(toplam)
}

fn atomik_yaz(dizin: &Path, ad: &str, deger: &[u8]) -> io::Result<()> {
    dizin_olustur(dizin)?;
    let gecici = dizin.join(format!("{ad}.yeni"));
    std::fs::write(&gecici, deger)?;
    std::fs::rename(&gecici, dizin.join(ad))
}

/// Dizin yalnızca kullanıcıya açıktır (Linux'ta 0700). Windows'ta yerel veri
/// dizininin izinlerini miras alır.
fn dizin_olustur(dizin: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dizin)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(dizin)
    }
}
```


`crates/kalfa-plugin/src/arac.rs` (yeni):

```rust
//! Eklenti aracı (aşama 4 tasarımı §8.1): yerleşik araçlarla aynı izin
//! motorundan geçer.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use kalfa_core::arac::{
    Arac, AracCiktisi, AracHatasi, CiktiGonderici, EklentiTarifi, Hazirlik, kirp,
};
use kalfa_core::kilit::KaynakKilitleri;
use kalfa_core::protokol::{CiktiAkisi, VeriCikisi};
use kalfa_tools::CalismaAlani;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::bag::kalfa::plugin::tipler::Hata;
use crate::calistirici::{CagriHatasi, Eklenti, Motor};
use crate::depo::Depo;
use crate::host::CagriOrtami;
use crate::sinir;

pub struct EklentiAraci {
    motor: Arc<Motor>,
    eklenti: Arc<Eklenti>,
    /// `eklenti.araclar` içindeki sırası.
    sira: usize,
    model_adi: String,
    alan: Arc<CalismaAlani>,
    kilitler: Arc<KaynakKilitleri>,
    /// Bu eklentinin bu çalışma alanındaki deposu (§10).
    depo_dizini: PathBuf,
}

impl EklentiAraci {
    /// Eklentinin bütün araçları; bir çalışma alanının oturumu için.
    /// `depo_dizini` eklentiye ve çalışma alanına özeldir.
    pub fn hepsi(
        motor: &Arc<Motor>,
        eklenti: &Arc<Eklenti>,
        alan: &Arc<CalismaAlani>,
        kilitler: &Arc<KaynakKilitleri>,
        depo_dizini: &Path,
    ) -> Vec<Arc<dyn Arac>> {
        (0..eklenti.araclar.len())
            .map(|sira| {
                Arc::new(Self {
                    motor: motor.clone(),
                    eklenti: eklenti.clone(),
                    sira,
                    model_adi: eklenti.manifest.model_adi(&eklenti.araclar[sira].ad),
                    alan: alan.clone(),
                    kilitler: kilitler.clone(),
                    depo_dizini: depo_dizini.to_path_buf(),
                }) as Arc<dyn Arac>
            })
            .collect()
    }

    fn arac_adi(&self) -> &str {
        &self.eklenti.araclar[self.sira].ad
    }

    fn tarif(&self) -> EklentiTarifi {
        let m = &self.eklenti.manifest;
        EklentiTarifi {
            ad: m.ad.clone(),
            surum: m.surum.to_string(),
            ozet: self.eklenti.ozet.clone(),
            globlar: m.dosya_oku.clone(),
            evren_uclari: m.evren.iter().map(|u| u.ad().to_string()).collect(),
        }
    }

    fn hata_iletisi(&self, h: &CagriHatasi) -> String {
        let ad = &self.eklenti.manifest.ad;
        let ileti = match h {
            CagriHatasi::Eklenti(h) => {
                let (tur, ileti) = match h {
                    Hata::IzinYok(m) => ("izin-yok", m.as_str()),
                    Hata::Bulunamadi(m) => ("bulunamadi", m.as_str()),
                    Hata::IptalEdildi => ("iptal-edildi", ""),
                    Hata::SinirAsildi(m) => ("sinir-asildi", m.as_str()),
                    Hata::Evren(m) => ("evren", m.as_str()),
                    Hata::Diger(m) => ("diger", m.as_str()),
                };
                format!("Eklenti hatası ({ad}): {tur}: {ileti}")
            }
            CagriHatasi::Bellek => format!(
                "Eklenti durduruldu ({ad}): bellek sınırı ({} MiB) aşıldı.",
                sinir::BELLEK / (1024 * 1024)
            ),
            CagriHatasi::Cpu => format!("Eklenti durduruldu ({ad}): işlem süresi sınırı aşıldı."),
            CagriHatasi::Sure => format!("Eklenti durduruldu ({ad}): süre sınırı aşıldı."),
            CagriHatasi::Cokme => format!("Eklenti çalışırken çöktü ({ad})."),
            CagriHatasi::Iptal => "İptal edildi.".into(),
        };
        kirp(&ileti, sinir::HATA_ILETISI)
    }
}

#[async_trait]
impl Arac for EklentiAraci {
    fn ad(&self) -> &str {
        &self.model_adi
    }

    fn aciklama(&self) -> &str {
        &self.eklenti.araclar[self.sira].aciklama
    }

    fn parametre_semasi(&self) -> Value {
        self.eklenti.araclar[self.sira].sema.clone()
    }

    /// v1 eklentileri yazamaz; İnceleme kipinde de sunulur.
    fn yazar_mi(&self) -> bool {
        false
    }

    async fn hazirla(&self, _argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
        Ok(Hazirlik {
            aciklama: format!(
                "{} eklentisinin `{}` aracını çalıştırmak istiyor.",
                self.eklenti.manifest.ad,
                self.arac_adi()
            ),
            veri_cikisi: Some(VeriCikisi::EklentiCiktisi),
            eklenti: Some(self.tarif()),
            kilitsiz: true,
            ..Default::default()
        })
    }

    async fn calistir(
        &self,
        argumanlar: &Value,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.calistir_akisli(argumanlar, hazirlik, iptal, CiktiGonderici::bos())
            .await
    }

    async fn calistir_akisli(
        &self,
        argumanlar: &Value,
        _hazirlik: &Hazirlik,
        iptal: CancellationToken,
        cikti: CiktiGonderici,
    ) -> Result<AracCiktisi, AracHatasi> {
        let ortam = CagriOrtami::yeni(
            self.eklenti.manifest.clone(),
            self.alan.clone(),
            self.kilitler.clone(),
            Depo::yeni(self.depo_dizini.clone(), self.eklenti.manifest.depo_mb),
            iptal,
            cikti.clone(),
        );
        let (sonuc, stderr) = self
            .eklenti
            .calistir(&self.motor, self.arac_adi(), &argumanlar.to_string(), ortam)
            .await;
        match sonuc {
            Ok(icerik) => Ok(AracCiktisi {
                icerik: kirp(&icerik, sinir::SONUC),
                kaynak: format!("eklenti:{}:{}", self.eklenti.manifest.ad, self.arac_adi()),
                ..Default::default()
            }),
            Err(CagriHatasi::Iptal) => Err(AracHatasi::Iptal),
            Err(h) => {
                // Tuzakta `stderr`'in sonu (panik iletisi) yalnızca canlı
                // çıktıya gider; modele gitmez (§7.3).
                if !matches!(h, CagriHatasi::Eklenti(_)) && !stderr.is_empty() {
                    cikti.gonder(CiktiAkisi::Hata, stderr);
                }
                Err(AracHatasi::Gecersiz(self.hata_iletisi(&h)))
            }
        }
    }
}
```


- [ ] `cargo test -p kalfa-plugin --locked --test yukleme --test calisma`
  geçer. İlk koşu test eklentilerini derler.
- [ ] Süre testleri yük altında: `iptali_yok_sayan_eklenti_iki_saniyede_birakilir`
  2,1–2,9 sn bekler. Paralel derleme sürerken düşerse tek başına yeniden
  koşturun; düzeltmek için eşiği gevşetmeyin, bildirin.

### Görev 8: Dosya, depo ve S15 testleri (`eklenti:`)

Bu görevin kodu yoktur; Görev 7'nin sınırlarını sınar. Biri düşerse sınır
eksik demektir: testi değil kodu düzeltin.

`crates/kalfa-plugin/tests/dosya.rs` (yeni):

```rust
//! Dosya çağrıları ve yetki (aşama 4 tasarımı §6.3, §7.3, §8.2, §8.3).
//! Eklenti dosyalara yalnızca host fonksiyonlarıyla, manifestin globları
//! içinde ve yerleşik okumanın yolundan erişir. Gizli dosya, `.git`,
//! bağlantı ve çalışma alanı dışı yol açılmaz.

mod ortak;

use ortak::*;
use serde_json::{Value, json};

const GIZLI: &str = "GIZLI_DEGER=dosya-testi-41c7";

async fn duzenek(globlar: &[&str]) -> Duzenek {
    let d = Duzenek::yeni(&manifest(globlar, &[], 0)).await;
    d.dosya("a.rs", b"fn a() {}\n")
        .dosya("docs/b.md", b"# B\n")
        .dosya(".env", GIZLI.as_bytes())
        .dosya("ayar/credentials.json", GIZLI.as_bytes())
        .dosya(".git/config", b"[core]\n")
        .dosya("ikili.rs", &[0xff, 0xfe, 0x00, 0x01]);
    d
}

fn oku(yol: &str) -> Value {
    json!({"islem": "oku", "yol": yol})
}

#[tokio::test]
async fn std_fs_on_acik_dizin_olmadigi_icin_hicbir_dosyayi_acamaz() {
    let d = duzenek(&["**/*"]).await;
    let mutlak = d.alan.path().join("a.rs").display().to_string();
    for yol in ["a.rs", "/a.rs", mutlak.as_str(), ".env"] {
        let c = d.cagir("dosya", json!({"islem": "std", "yol": yol})).await;
        assert!(c.icerik().starts_with("okunamadı"), "{yol}: {}", c.icerik());
    }
}

#[tokio::test]
async fn glob_icindeki_dosya_okunur() {
    let d = duzenek(&["**/*"]).await;
    assert_eq!(d.cagir("dosya", oku("a.rs")).await.icerik(), "fn a() {}\n");
    assert_eq!(d.cagir("dosya", oku("docs/b.md")).await.icerik(), "# B\n");
}

#[tokio::test]
async fn gizli_korumali_ve_disaridaki_yollar_okunamaz() {
    let d = duzenek(&["**/*"]).await;
    let mutlak = d.alan.path().join("a.rs").display().to_string();
    for (yol, beklenen) in [
        (".env", "izin-yok: gizli dosya"),
        ("ayar/credentials.json", "izin-yok: gizli dosya"),
        (".git/config", "izin-yok: korumalı yol"),
        ("../disari.rs", "izin-yok: `../disari.rs` geçersiz yol"),
        (mutlak.as_str(), "izin-yok"),
        ("yok.rs", "bulunamadi: yok.rs"),
        ("ikili.rs", "diger: `ikili.rs` metin değil"),
    ] {
        let c = d.cagir("dosya", oku(yol)).await;
        assert!(c.icerik().starts_with(beklenen), "{yol}: {}", c.icerik());
        assert!(!c.icerik().contains("dosya-testi-41c7"));
    }
}

#[tokio::test]
async fn glob_disindaki_dosya_okunamaz_ve_listelenmez() {
    let d = duzenek(&["**/*.rs"]).await;
    let c = d.cagir("dosya", oku("docs/b.md")).await;
    assert_eq!(
        c.icerik(),
        "izin-yok: `docs/b.md` manifestteki globlara uymuyor"
    );
    let l = d
        .cagir("dosya", json!({"islem": "listele", "desen": "**/*"}))
        .await;
    let yollar: Vec<&str> = l
        .icerik()
        .lines()
        .map(|s| s.split('\t').next().unwrap())
        .collect();
    assert_eq!(yollar, vec!["a.rs", "ikili.rs"]);
}

#[tokio::test]
async fn liste_gizli_ve_git_dosyalarini_gostermez() {
    let d = duzenek(&["**/*"]).await;
    let l = d
        .cagir("dosya", json!({"islem": "listele", "desen": "**/*"}))
        .await;
    let satirlar: Vec<Vec<&str>> = l
        .icerik()
        .lines()
        .map(|s| s.split('\t').collect())
        .collect();
    let yollar: Vec<&str> = satirlar.iter().map(|s| s[0]).collect();
    assert_eq!(yollar, vec!["a.rs", "docs/b.md", "ikili.rs"]);
    // Boyut ve değişme zamanı (ms).
    assert_eq!(satirlar[0][1], "10");
    let zaman: u64 = satirlar[0][2].parse().unwrap();
    assert!(zaman > 1_700_000_000_000, "{zaman}");
    // Desen manifestle kesişir.
    let l = d
        .cagir("dosya", json!({"islem": "listele", "desen": "docs/**"}))
        .await;
    assert_eq!(l.icerik().split('\t').next(), Some("docs/b.md"));
}

#[tokio::test]
async fn buyuk_dosya_sinirda_durur() {
    let d = duzenek(&["**/*"]).await;
    d.dosya("buyuk.rs", &vec![b'a'; 4 * 1024 * 1024 + 1]);
    let c = d.cagir("dosya", oku("buyuk.rs")).await;
    assert!(c.icerik().starts_with("sinir-asildi"), "{}", c.icerik());
}

#[tokio::test]
async fn sembolik_baglanti_izlenmez() {
    let d = duzenek(&["**/*"]).await;
    let hedef = d.alan.path().join(".env");
    let baglanti = d.alan.path().join("bag.rs");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&hedef, &baglanti).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&hedef, &baglanti).unwrap();
    let c = d.cagir("dosya", oku("bag.rs")).await;
    assert!(c.icerik().starts_with("izin-yok"), "{}", c.icerik());
    assert!(!c.icerik().contains("dosya-testi-41c7"));
    let l = d
        .cagir("dosya", json!({"islem": "listele", "desen": "**/*"}))
        .await;
    assert!(!l.icerik().contains("bag.rs"), "{}", l.icerik());
}

#[tokio::test]
async fn hard_linkli_dosya_okunmaz() {
    let d = duzenek(&["**/*"]).await;
    d.dosya("c.rs", GIZLI.as_bytes());
    std::fs::hard_link(d.alan.path().join("c.rs"), d.alan.path().join("c2.rs")).unwrap();
    let c = d.cagir("dosya", oku("c2.rs")).await;
    assert!(c.icerik().starts_with("izin-yok"), "{}", c.icerik());
    assert!(c.icerik().contains("hard link"), "{}", c.icerik());
}

#[tokio::test]
async fn manifestte_olmayan_yetenek_izin_yok_alir() {
    let d = duzenek(&[]).await;
    assert_eq!(
        d.cagir("dosya", oku("a.rs")).await.icerik(),
        "izin-yok: dosya-oku yeteneği manifestte yok"
    );
    let c = d.cagir("yetkisiz", json!({})).await;
    assert_eq!(
        c.icerik(),
        "evren: izin-yok: evren yeteneği manifestte yok: `embeddings`\ndepo: izin-yok: depo yeteneği manifestte yok"
    );
}

#[tokio::test]
async fn evren_yetenegi_olsa_da_4a_da_cagri_yok() {
    let d = Duzenek::yeni(&manifest(&[], &["embeddings"], 0)).await;
    let c = d.cagir("yetkisiz", json!({})).await;
    assert!(c.icerik().contains("aşama 4b"), "{}", c.icerik());
}
```


`crates/kalfa-plugin/tests/depo.rs` (yeni):

```rust
//! Eklenti deposu (aşama 4 tasarımı §10): kota, atomik yazma, eklenti ×
//! çalışma alanı ayrımı.

mod ortak;

use ortak::*;
use serde_json::json;

async fn yaz(d: &Duzenek, anahtar: &str, deger: &str) -> String {
    d.cagir(
        "depo",
        json!({"islem": "yaz", "anahtar": anahtar, "deger": deger}),
    )
    .await
    .icerik()
    .to_string()
}

async fn oku(d: &Duzenek, anahtar: &str) -> String {
    d.cagir("depo", json!({"islem": "oku", "anahtar": anahtar}))
        .await
        .icerik()
        .to_string()
}

#[tokio::test]
async fn yazilan_okunur_silinen_gider() {
    let d = Duzenek::yeni(&manifest(&[], &[], 1)).await;
    assert_eq!(oku(&d, "meta").await, "yok");
    assert_eq!(yaz(&d, "meta", "bir").await, "yazıldı");
    assert_eq!(yaz(&d, "meta", "iki").await, "yazıldı");
    assert_eq!(oku(&d, "meta").await, "değer: iki");
    let c = d
        .cagir("depo", json!({"islem": "sil", "anahtar": "meta"}))
        .await;
    assert_eq!(c.icerik(), "silindi");
    assert_eq!(oku(&d, "meta").await, "yok");
}

#[tokio::test]
async fn dosya_adi_anahtarin_ozeti_gecici_dosya_kalmaz() {
    let d = Duzenek::yeni(&manifest(&[], &[], 1)).await;
    yaz(&d, "vektor/1", "x").await;
    let adlar: Vec<String> = std::fs::read_dir(d.depo.path())
        .unwrap()
        .map(|g| g.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        adlar,
        vec![kalfa_core::muhur::ozet_hex(b"vektor/1")],
        "geçici dosya kalmamalı"
    );
}

#[tokio::test]
async fn kota_asilinca_yazma_reddedilir_eski_deger_kalir() {
    let d = Duzenek::yeni(&manifest(&[], &[], 1)).await;
    let yaz_boyut = |anahtar: &'static str, n: u64| {
        d.cagir(
            "depo",
            json!({"islem": "yaz", "anahtar": anahtar, "boyut": n}),
        )
    };
    assert_eq!(yaz_boyut("a", 600_000).await.icerik(), "yazıldı");
    assert_eq!(
        yaz_boyut("b", 600_000).await.icerik(),
        "sinir-asildi: depo kotası (1 MB) doldu"
    );
    // Üzerine yazmada eski değerin yeri sayılmaz.
    assert_eq!(yaz_boyut("a", 900_000).await.icerik(), "yazıldı");
    assert_eq!(oku(&d, "b").await, "yok");
    // Kota çağrılar arasında diskteki dosyalardan yeniden sayılır.
    assert!(
        yaz_boyut("c", 200_000)
            .await
            .icerik()
            .starts_with("sinir-asildi")
    );
}

#[tokio::test]
async fn gecersiz_anahtar_reddedilir() {
    let d = Duzenek::yeni(&manifest(&[], &[], 1)).await;
    assert!(yaz(&d, "", "x").await.starts_with("diger: depo anahtarı"));
    assert!(
        yaz(&d, &"a".repeat(201), "x")
            .await
            .starts_with("diger: depo anahtarı")
    );
}

/// Depo dizini eklenti × çalışma alanı başınadır; çağıran onu ayrı verir.
#[tokio::test]
async fn ayri_depo_dizinleri_birbirini_gormez() {
    let d = Duzenek::yeni(&manifest(&[], &[], 1)).await;
    let obur = tempfile::tempdir().unwrap();
    yaz(&d, "meta", "birinci alan").await;
    let c = cagir(
        &d.arac_depoyla("depo", obur.path()),
        json!({"islem": "oku", "anahtar": "meta"}),
        tokio_util::sync::CancellationToken::new(),
    )
    .await;
    assert_eq!(c.icerik(), "yok");
}
```


`crates/kalfa-plugin/tests/senaryo.rs` (yeni):

```rust
//! S15 (aşama 4 tasarımı §16.3): kötü niyetli eklenti gizli bir dosyayı
//! okumaya çalışıyor. Oturumun içinden, yerleşik araçlarla birlikte.
//! Gizli içerik hiçbir model isteğine girmez.

mod ortak;

use kalfa_core::izin::Kip;
use kalfa_core::oturum::OturumAyarlari;
use kalfa_core::protokol::{BitisNedeni, Komut, Olay};
use kalfa_core::test_destek::{SahteSaglayici, SahteYanit, test_baglami, test_oturumu};
use ortak::*;
use serde_json::json;

const GIZLI: &str = "GIZLI_DEGER=senaryo-s15-9d2e";

#[tokio::test]
async fn s15_eklenti_gizli_dosyayi_okuyamaz_icerik_modele_gitmez() {
    let d = Duzenek::yeni(&manifest(&["**/*"], &[], 0)).await;
    d.dosya(".env", GIZLI.as_bytes())
        .dosya("BENIOKU.md", b"# Proje\n");
    let mutlak = d.alan.path().join(".env").display().to_string();
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![
            ("kalfa_test__dosya", json!({"islem": "oku", "yol": ".env"})),
            ("kalfa_test__dosya", json!({"islem": "std", "yol": mutlak})),
            (
                "kalfa_test__dosya",
                json!({"islem": "listele", "desen": "**/*"}),
            ),
        ]),
        SahteYanit::metin("Bitti."),
    ]);
    let mut araclar = kalfa_tools::varsayilan_araclar(d.calisma_alani.clone());
    araclar.extend(d.araclar_depoyla(d.depo.path()));
    let baglam = test_baglami(
        s.clone(),
        araclar,
        std::sync::Arc::new(kalfa_core::izin::Siniflandirici::yeni(d.alan.path(), &[]).unwrap()),
    );
    // Serbest kipte de eklenti onaysız çalışır; korumayı izin kartı değil
    // host'un dosya kuralı sağlar.
    let mut ayarlar = OturumAyarlari::yeni("glm-5.3", d.alan.path());
    ayarlar.kip = Kip::Serbest;
    let mut surucu = test_oturumu(baglam, ayarlar);
    surucu
        .gonder(Komut::KullaniciMesaji {
            metin: "Projeyi incele.".into(),
        })
        .await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert!(
        !surucu
            .olaylar()
            .iter()
            .any(|o| matches!(o, Olay::OnayGerekiyor { .. }))
    );

    let istekler = format!("{:?}", s.istekler());
    assert!(
        !istekler.contains("senaryo-s15-9d2e"),
        "gizli içerik modele gitti"
    );
    assert!(istekler.contains("izin-yok: gizli dosya"), "{istekler}");
    assert!(istekler.contains("okunamadı"), "{istekler}");
    assert!(istekler.contains("BENIOKU.md"), "{istekler}");
    assert!(!istekler.contains(".env\\t"), "{istekler}");
}
```


- [ ] `cargo test -p kalfa-plugin --locked` geçer.
- [ ] Windows'ta sembolik bağlantı testi bağlantı oluşturma izni ister
  (AGENTS.md: Geliştirici Modu).

### Görev 9: CI ve belgeler (`derleme:`, `docs:`)

- [ ] CI'ya eklentilerin `wasm32-wasip2` clippy'si (Rust işinde, öteki
  clippy'lerden sonra):

`.github/workflows/ci.yml`:

```diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 7a3098d..3aa01a4 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -90,6 +90,8 @@ jobs:
         run: cargo clippy --workspace --all-targets --locked -- -D warnings
       - name: cargo clippy (e2e özelliği)
         run: cargo clippy -p kalfa -p kalfa-uygulama --features e2e --all-targets --locked -- -D warnings
+      - name: cargo clippy (eklentiler, wasm32-wasip2)
+        run: cargo clippy --target wasm32-wasip2 --locked -p kalfa-plugin-sdk -p kalfa-test-eklentisi -p kalfa-test-soket -p kalfa-test-yanlis-dunya -- -D warnings
       - name: cargo test
         if: runner.os == 'Linux'
         run: cargo test --workspace --locked
@@ -104,6 +106,7 @@ jobs:
         run: |
           cargo test --workspace --locked --no-run
           cargo test -p kalfa-uygulama -p kalfa --features e2e --locked --no-run
+          cargo build --target wasm32-wasip2 --locked -p kalfa-test-eklentisi -p kalfa-test-soket -p kalfa-test-yanlis-dunya
       - name: cargo test, yönetici olmayan kullanıcıyla (Windows)
         if: runner.os == 'Windows'
         shell: pwsh
```


- [ ] AGENTS.md:

`AGENTS.md`:

````diff
diff --git a/AGENTS.md b/AGENTS.md
index 6f80807..295a6ac 100644
--- a/AGENTS.md
+++ b/AGENTS.md
@@ -30,6 +30,8 @@ dosyaları Apache-2.0 WITH LLVM-exception'dır.
 | `crates/kalfa-core` | Arayüzden bağımsız oturum çekirdeği: izin kipleri, mühürlü onay, JSONL kayıt ve kurtarma, git koruması |
 | `crates/kalfa-tools` | Dosya araçları ve kabuk aracı |
 | `crates/kalfa-sandbox` | Komut çalıştırma, süreç sınırları, Linux yalıtımı (bwrap ve seccomp) |
+| `crates/kalfa-plugin` | Eklenti çalışma zamanı: manifest, wasmtime host'u, sınırlar, depo, eklenti aracı; test eklentileri `test-eklentileri/`'nde |
+| `crates/kalfa-plugin-sdk` | Eklenti yazarının SDK'sı ve WIT sözleşmesi (`wit/kalfa.wit`); lisansı Apache-2.0 WITH LLVM-exception |
 | `crates/kalfa-uygulama` | Uygulamanın Tauri'siz mantığı: oturumlar, köprü, yalıtım durumu, e2e sahteleri |
 | `crates/kalfa` | Tauri kabuğu: komutlar, pencereler, yetkiler |
 | `ui/` | Svelte 5 + TypeScript arayüzü (Vite, Vitest) |
@@ -39,9 +41,14 @@ dosyaları Apache-2.0 WITH LLVM-exception'dır.
 
 ## Kurulum ve doğrulama
 
-Rust sürümü `rust-toolchain.toml` ile sabittir (1.98.1). Node sürümü
+Rust sürümü `rust-toolchain.toml` ile sabittir (1.98.1); eklentiler için
+`wasm32-wasip2` hedefi de oradadır. Rustup hedefi kendiliğinden kurmazsa:
+`rustup target add wasm32-wasip2 --toolchain 1.98.1`. Node sürümü
 `^22.22.2 || >=24.15.0`'tır. Bağımlılıklar `--locked` ve `npm ci` ile kurulur.
 
+`kalfa-plugin`'in testleri test eklentilerini `wasm32-wasip2` için kendileri
+derler; ayrı bir komut gerekmez.
+
 Her değişiklikten sonra:
 
 ```sh
@@ -59,6 +66,12 @@ Değişikliğin dokunduğu yere göre ayrıca:
   cargo clippy -p kalfa -p kalfa-uygulama --features e2e --all-targets --locked -- -D warnings
   ```
 
+- **`kalfa-plugin-sdk` ya da test eklentileri:**
+
+  ```sh
+  cargo clippy --target wasm32-wasip2 --locked -p kalfa-plugin-sdk -p kalfa-test-eklentisi -p kalfa-test-soket -p kalfa-test-yanlis-dunya -- -D warnings
+  ```
+
 - **`kalfa-sandbox` ya da `kalfa-tools`:** Aşağıdaki komutlar Linux'ta anlamlıdır
   ve root olmayan bir kullanıcıyla koşar. WSL'de bu kullanıcı `kalfatest`'tir.
 
@@ -120,8 +133,9 @@ Bağlantı testleri Windows'ta sembolik bağlantı oluşturma izni ister.
 `.github/workflows/ci.yml`, `kalfa-asama-1`'e her push'ta ve her PR'da koşar.
 
 - **Kapsam:** Yukarıdaki varsayılan komutlar: `cargo fmt`, arayüz
-  (`check`, `test`, `build`), iki clippy ve iki `cargo test` (düz ve `e2e`
-  özellikli). Rust işi Linux'ta ve Windows'ta koşar.
+  (`check`, `test`, `build`), üç clippy (düz, `e2e` özellikli ve eklentilerin
+  `wasm32-wasip2`'si) ve iki `cargo test` (düz ve `e2e` özellikli). Rust işi
+  Linux'ta ve Windows'ta koşar.
 - **Kapsam dışı:** Yalıtım testleri (bwrap, seccomp, AppContainer) ve uçtan
   uca testler. Bunlar yukarıdaki gibi yerelde koşturulur; yeşil CI onların
   yerini tutmaz.
````


- [ ] Tasarım belgesi bu planın PR'ında güncellendi ("Tasarımdan farklar",
  G1, G3, G7); uygulayıcı ona dokunmaz.
- [ ] `docs/GELISTIRME_DURUMU.md`: 4a uygulandı, sırada 4b ve 4c.
  Commit'lemeden önce `git diff` ile başkasının düzenlemesi var mı bakın.

## Son kontrol listesi

İki platformda (Windows ve WSL Ubuntu 24.04):

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings`
- [ ] `cargo clippy --target wasm32-wasip2 --locked -p kalfa-plugin-sdk -p kalfa-test-eklentisi -p kalfa-test-soket -p kalfa-test-yanlis-dunya -- -D warnings`
- [ ] `cargo test --workspace --locked`
- [ ] `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked` ve
  aynı özellikle clippy
- [ ] `kalfa-plugin`: 41 test (9 birim + 32 bütünleşme), hepsi geçer;
  `kalfa-core`'da 3 yeni test (`arac_muhur` 1, `oturum_kilitsiz` 2).
- [ ] CI yeşil. Arayüz ve uçtan uca testler bu planda değişmez.
