# Aşama 4 ön ölçümü: EVREN gömme ve yeniden sıralama, wasmtime

**Tarih:** 2026-10-07
**Yapan:** Claude, kullanıcının izniyle (canlı EVREN istekleri, `wasm32-wasip2`
hedefinin ve wasmtime crate'lerinin indirilmesi).
**Amaç:** [Aşama 4 tasarımını](../specs/2026-10-07-kalfa-4-eklenti-rag-design.md)
varsayımla değil ölçümle yazmak.
**Ortam:** Windows 11, Rust 1.98.1. EVREN tabanı `.env`'deki `EVREN_BASE_URL`.
Şart ucu yalnızca okundu (`GET /v1/terms/status`, `accepted: true`); kabul ucu
çağrılmadı. Linux'ta ölçülmedi; 4a planında ölçülür (tasarım §19, G1).

Betikler ve prototip ürün kodu değildir, depoya girmedi. Ham EVREN yanıtlarından
dördü `crates/kalfa-evren/tests/kayitlar/`'a eklendi (§1.4).

---

## 1. EVREN: `embeddings` ve `rerank`

### 1.1 Biçim

| Uç | Sonuç |
|---|---|
| `POST /v1/embeddings` | OpenAI biçimi: `{model, input: [metin], dimensions?, encoding_format?}` → `data[].embedding`, `usage` (`prompt_tokens`, `total_tokens`, `usage.evren`) |
| `POST /v1/rerank` | Cohere/Jina biçimi (vLLM): `{model, query, documents: [metin], top_n?}` → `results[] {index, relevance_score, document.text}`, `usage` (`usage.evren` dahil) |
| `POST /v1/score` | `404` |

- **Yanıttaki model adı katalogdakinden farklı:** `qwen3-embedding-8b` isteği
  `qwen/qwen3-embedding-8b` döndürüyor. `qwen3-reranker-8b` isteği ise
  `qwen/qwen3-vl-reranker-8b`'ye yönleniyor (hem gövdede hem
  `X-Evren-Routed-Model`'de). Katalogda VL sürümü yok.
- **Boyut:** varsayılan 4096. `dimensions: 1024` kabul ediliyor (Matryoshka);
  yanıt 1024 sayı.
- **`encoding_format: "base64"`** kabul ediliyor. 4096 boyutlu tek vektör JSON'da
  ~88 KB, base64'te ~22 KB.
- **Girdi sınırı:** tek girdi en çok 40 960 token. Aşılınca `400`,
  `code: "bad_request"`, `type: "invalid_request_error"`; ileti token sayısını
  söylüyor. Sınır girdi başına: 2048 girdilik, toplam 58 282 tokenlik istek
  geçti.
- **Başlıklar:** sohbetteki `X-Evren-*` başlıkları ve `X-Request-Id` gömme ve
  yeniden sıralamada da geliyor. Yeni gözlenenler: `X-Ratelimit-Limit-Tokens:
  2500000`, `X-Ratelimit-Remaining-Tokens`, `X-Ratelimit-Reset` (Unix saniyesi).
  `X-Ratelimit-*-Requests` ve `Retry-After` gözlenmedi (429 alınmadı).
- **Kredi:** bütün isteklerde `credits_held_cr: "0.0000"`; bakiye değişmedi
  (beta, katalog fiyatı `0.0 CR`).

### 1.2 Gecikme

| İstek | İlk bayt | Toplam |
|---|---|---|
| Gömme, 1 kısa girdi (art arda 3 kez) | 7,4–8,9 sn | aynı |
| Gömme, 1 kısa girdi (3 paralel istek) | 7,25–7,27 sn (üçü de) | aynı |
| Gömme, 2 kısa girdi (ilk istek) | — | 9,7 sn |
| Gömme, 128 kısa girdi (3 090 token) | — | 14,6 sn |
| Gömme, 1 girdi, 11 993 token | 10,8 sn | 10,8 sn |
| Gömme, 512 girdi, 14 226 token, `dimensions: 1024`, JSON | 11,0 sn | 13,3 sn (11,1 MB) |
| Gömme, 2048 girdi, 58 282 token, `dimensions: 1024`, JSON | 9,7 sn | 26,2 sn (44,5 MB) |
| Yeniden sıralama, 3 kısa belge | — | 0,34 sn |
| Yeniden sıralama, 64 belge, 49 236 token, `top_n: 5` | 2,7 sn | 2,7 sn |

**Sonuç:** gömmede her istekte boyuttan bağımsız ~7–9 sn'lik sabit bir bekleme
var. Paralel istekler sırada beklemiyor: üç paralel istek, tek bir isteğin
süresinde bitti. Büyük toplu istekte süreyi yanıtın boyutu belirliyor (JSON
sayıları); base64 bunu dörtte birine indirir. Yeniden sıralama hızlı.

Bu bir günün ölçümüdür; sabit beklemenin günün saatine ya da yüke göre değişip
değişmediği bilinmiyor (tasarım G4).

### 1.3 Kalite (tek örnek)

64 benzer paragraflı belgede ilgili belge (37. sıra) 0,539 puanla birinci geldi;
ötekiler 0,0145–0,0153. Üç belgelik Türkçe örnekte de sıra doğruydu (0,056'ya
karşı 0,003 ve 0,002). Puanlar mutlak değil, göreli kullanılmalı. Türkçe arama
kalitesi 4c'de bir değerlendirme kümesiyle ölçülür (tasarım G5).

### 1.4 Eklenen kayıtlar

| Dosya | İçerik |
|---|---|
| `embeddings_qwen3_1024.json` | `dimensions: 1024`, tek girdi |
| `embeddings_qwen3_base64.json` | `encoding_format: "base64"`, 4096 boyut |
| `rerank_qwen3.json` | 3 Türkçe belge; `qwen3-vl-reranker-8b`'ye yönlenme |
| `error_embedding_context_length.json` | 40 960 token sınırının `400` yanıtı |

---

## 2. wasmtime ve Component Model

**Sürümler:** wasmtime 49.0.2 (en son kararlı; MSRV 1.96), wasmtime-wasi
49.0.2, wit-bindgen 0.62.0. 50.0.0 henüz rc. Rust 1.98.1 ile sorunsuz.
wasmtime özellikleri: `default-features = false`, `runtime`, `cranelift`,
`component-model`, `async`, `std`. wasmtime-wasi: `default-features = false`,
`p2`.

**Prototip:** üst belge §8.2'deki WIT, bir örnek eklenti ve bir host (scratchpad).
Örnek eklenti `wasm32-wasip2` için `wit-bindgen` ile derlendi; araçları:
sonsuz döngü, bellek şişirme, `std::fs` ile dosya okuma denemesi, host'tan
dosya okuma, saat, rastgele sayı (`HashMap`).

### 2.1 Eklentinin importları

En küçük Rust eklentisinin bileşeni, yalnızca `kalfa:plugin` arayüzlerini değil
şunları da import ediyor:

```
wasi:io/{poll,error,streams}
wasi:clocks/{monotonic-clock,wall-clock}
wasi:cli/{stdout,stderr,stdin,environment,exit,terminal-*}
wasi:filesystem/{types,preopens}
wasi:random/insecure-seed
```

- `wasi:filesystem`, eklenti hiç dosya açmasa da import ediliyor (Rust std'nin
  başlangıçtaki ön açık dizin taraması).
- `wasi:sockets` yalnızca eklenti ağ kullanırsa import ediliyor
  (`std::net::TcpStream::connect` eklenince yedi `wasi:sockets/*` import'u
  geldi).
- **WASI'siz linker'la örnekleme başarısız:** "`wasi:io/poll` … implementation
  was not found".

**Sonuç:** Üst belge §8.3'teki "izin verilmeyen bir import (örneğin WASI dosya
sistemi) taşıyan bileşen bağlanamaz" kuralı olduğu gibi uygulanırsa, Rust std
kullanan hiçbir eklenti yüklenmez. Uygulanabilir kural iki katmanlıdır (tasarım
§7.3):

1. Yüklemeden önce import listesi bir izin listesine karşı denetlenir
   (`Component::component_type().imports()`). `wasi:sockets`, `wasi:http` ve
   listede olmayan her şey reddedilir.
2. Dosya sistemi ön açık dizin olmadan bağlanır. Dizin tutamacı olmadığı için
   hiçbir dosya açılamaz.

### 2.2 Davranış

| Deneme | Sonuç |
|---|---|
| `std::fs::read_to_string("C:\\Windows\\win.ini")`, ön açık dizin yok | `No such file or directory (os error 44)` |
| `std::fs::read_to_string("/etc/passwd")`, ön açık dizin yok | aynı |
| `TcpStream::connect("1.1.1.1:443")`, tam WASI p2, varsayılan `WasiCtx` | `Permission denied`; import denetimi bu eklentiyi zaten yüklemez |
| Sonsuz döngü, epoch tiki 10 ms, sınır 50 tik | 514 ms'de tuzak (`wasm trap: interrupt`) |
| 1 MiB'lık parçalarla bellek ayırma, `StoreLimits` 64 MiB, `trap_on_grow_failure` | 10 ms'de tuzak ("forcing trap when growing memory to 67174400 bytes") |
| Tuzaktan sonra yeni `Store` ve aynı `InstancePre` ile çağrı | Çalışıyor |
| Host'a çağrı (`dosya-oku`), `gunluk` | Çalışıyor; async host fonksiyonu (`imports: { default: async }`) |

### 2.3 Süreler (sürüm derlemesi)

| Ölçüm | Değer |
|---|---|
| Bileşeni derleme (`Component::new`, 94 KB, cranelift) | 121 ms (hata ayıklama derlemesinde 1,6 sn) |
| Derlenmiş bileşeni serileştirme / açma | 0,4 ms (368 KB) / 0,3 ms |
| `InstancePre`'den örnekleme | 0,15 ms |
| Basit çağrı | 0,09 ms |
| Örnek eklentinin derlenmesi (`wasm32-wasip2`, `opt-level = "s"`, LTO) | 24 sn, 94 KB; soket kullanan sürüm 151 KB |
| Host'un temiz derlenmesi, hata ayıklama (wasmtime + tokio) | 1 dk 25 sn |
| Host'un temiz derlenmesi, sürüm | 3 dk 26 sn |
| Host ikilisi (sürüm) | 16,4 MB |

**Sonuç:** örnekleme o kadar ucuz ki her araç çağrısı yeni bir `Store` ve
örnekle yapılabilir. Çağrılar arasında eklenti belleğinde durum kalmaz; tuzak
sonrası "zehirlenmiş örnek" sorunu doğmaz. Bileşen uygulama açıkken bir kez
derlenir. Disk önbelleği (`Component::deserialize`) `unsafe` gerektirir ve
kazancı ~120 ms'dir, gerekmez.

### 2.4 API notları (wasmtime 49)

- `Config::async_support` artık etkisiz ve kullanımdan kalkmış; `async`
  özelliği açıksa async çağrılar çalışıyor.
- `bindgen!`: `imports: { default: async }`, `exports: { default: async }`.
  Bağlama `Eklenti::add_to_linker::<T, HasSelf<T>>(&mut linker, |t| t)`.
- Hata tipi `wasmtime::Error`'dır; `anyhow` değil.
- `wasmtime_wasi::p2::add_to_linker_async` bütün WASI p2'yi (soketler dahil)
  bağlar. Yalnızca bazı arayüzleri bağlamak iç API'lere dayanır; import
  denetimi bu yüzden yükleme anında yapılır (§2.1).
