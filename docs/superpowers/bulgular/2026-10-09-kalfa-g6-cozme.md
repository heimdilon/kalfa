# Kalfa G6 — Base64 ve sayı dizisi çözme maliyeti

**Tarih:** 2026-10-09.
**SDK kodu:** `2153f5299a4f3aefddc0ec2e57d30da5801f24b7`, PR #52.
**Durum:** G6 bu SDK ve ölçülen geçerli yanıtlar için kapandı. Ürünün
`encoding_format: "base64"` ve `base64` crate'i seçimi korunur.

## Sonuç ve karar

- Gerçek wasm konuğunda, tam SDK çözme yolu base64 ile sayı dizisine göre
  **8,6–10,7 kat hızlı**. RAG'ın 256 × 1024 toplu yanıtı Windows'ta
  **1,97 ms / 18,35 ms**, WSL'de **1,99 ms / 17,95 ms** (base64 / float).
- Aynı `f32` bitlerini taşıyan base64 JSON yanıtı **%74,6–75,1 küçük**.
  256 × 1024 yanıt 1 411 500 / 5 570 476 bayt; 256 × 4096 yanıt
  5 605 804 / 22 533 036 bayt.
- JSON çözmeyi dışarıda bırakan ham base64 + `f32` dönüşümünde crate,
  basit skaler elle çözücüden wasm'da **2,2–5,9 kat hızlı**. Elle çözücüye
  geçmek için kanıt yok; mevcut crate korunur. Bu sonuç her olası özel
  çözücünün daha yavaş olacağını söylemez.
- Dört çıktı dosyasındaki 96 ölçüm satırının tamamında çözülen vektörlerin
  boyutu, adedi ve bütün `f32` bitleri beklenenle eşleşti. Ağ/EVREN
  kullanılmadı, anahtar okunmadı ve kredi harcanmadı.

## Düzenek

- Ryzen 5 5600, 6 çekirdek / 12 mantıksal işlemci; aynı makinede Windows 11
  Home 10.0.26200 ve WSL2 Ubuntu 24.04, `kalfatest`. WSL çekirdeği
  `6.18.33.2-microsoft-standard-WSL2`.
- Rust 1.98.1 (`48a229cea`); SDK'nın `base64` 0.22.1, `serde_json` 1.0.151
  ve wit-bindgen 0.62.0 sürümleri. Prototipin bütün bağımlılık/sürüm çiftleri
  ürünün Cargo.lock'unda bulunuyor. Yeni üçüncü taraf bağımlılık yok.
- Konuk ve yerel ölçüm `release`, host örneği `dev` profilinde. Wasm yolu
  gerçek `Motor::yeni` (wasmtime 49.0.2), `EklentiAraci` ve WIT 0.1 üzerinden
  çalışır; eklentiye dosya, EVREN ya da depo yeteneği verilmez.
- 1024 boyut için `embeddings_qwen3_1024.json`, 4096 için
  `embeddings_qwen3_base64.json` kullanılır. Topludaki her vektör aynı gerçek
  değerlerin farklı döndürmesini içerir. Sayı dizisi ve base64 aynı `f32`
  bitlerinden üretilir; sayıların JSON gösterimi `serde_json`'undur.
- Her boyut için 1, 16 ve 256 vektör; her durum için tam SDK base64/float
  ve ham crate/elle olmak üzere dört yol. Üç ısınma, en az 10 ms için
  uyarlanan tekrar sayısı, ardından 21 örnek. Medyan ve en yakın sıra
  yöntemiyle p95 kaydedilir; p95, sıralı 21 örneğin 20.'sidir.
- Yanıt üretimi, doğruluk karşılaştırması, wasm yükleme ve örnekleme
  zamanlamaya girmez. Tam SDK süresi JSON çözme, ayırma, vektör üretme,
  sıralama ve doğrulamayı; ham süre yalnızca base64 + küçük endian `f32`
  dönüşümünü içerir. Vektörlerin tahsisi ve bırakılması ölçümün içindedir.
- Süre `Instant` ile ölçülen geçen süredir; süreç CPU sayacı değildir.
  Wasm'ın epoch/yer verme davranışı dahildir. Windows ve WSL ölçümleri
  sırayla, derleme ve testler ölçüm aralığının dışında çalıştırıldı.

## Tam SDK çözme süresi

Her değer **medyan ms / yanıt**, küçük değer iyidir. Yerel satırlar SDK'nın
işletim sistemi hedefinde; wasm satırları eklenti içindeki aynı fonksiyondur.

| Boyut × adet | Windows yerel base64 | Windows yerel float | WSL yerel base64 | WSL yerel float |
|---|---:|---:|---:|---:|
| 1024 × 1 | 0,0041 | 0,0356 | 0,0038 | 0,0356 |
| 1024 × 16 | 0,0618 | 0,7114 | 0,0599 | 0,5855 |
| 1024 × 256 | 1,2463 | 12,4187 | 1,4116 | 9,7938 |
| 4096 × 1 | 0,0141 | 0,1422 | 0,0132 | 0,1352 |
| 4096 × 16 | 0,2270 | 2,3287 | 0,2134 | 2,2064 |
| 4096 × 256 | 6,8010 | 47,2745 | 3,6407 | 38,2175 |

| Boyut × adet | Windows wasm base64 | Windows wasm float | WSL wasm base64 | WSL wasm float |
|---|---:|---:|---:|---:|
| 1024 × 1 | 0,0079 | 0,0701 | 0,0078 | 0,0671 |
| 1024 × 16 | 0,1181 | 1,1059 | 0,1200 | 1,1213 |
| 1024 × 256 | 1,9739 | 18,3530 | 1,9850 | 17,9527 |
| 4096 × 1 | 0,0271 | 0,2742 | 0,0269 | 0,2709 |
| 4096 × 16 | 0,4292 | 4,5089 | 0,4147 | 4,3084 |
| 4096 × 256 | 7,0021 | 74,8599 | 6,9412 | 70,4751 |

## Crate ve elle çözücü

JSON dışındaki base64 + `f32` dönüşümü, **medyan ms / yanıt**:

| Boyut × adet | Windows wasm crate | Windows wasm elle | WSL wasm crate | WSL wasm elle |
|---|---:|---:|---:|---:|
| 1024 × 1 | 0,0050 | 0,0110 | 0,0050 | 0,0114 |
| 1024 × 16 | 0,0804 | 0,2357 | 0,0801 | 0,2243 |
| 1024 × 256 | 1,2763 | 3,6330 | 1,2543 | 4,2101 |
| 4096 × 1 | 0,0193 | 0,0928 | 0,0194 | 0,0936 |
| 4096 × 16 | 0,3236 | 1,7079 | 0,3161 | 1,8504 |
| 4096 × 256 | 5,2417 | 28,4741 | 4,7919 | 26,6912 |

Ham ölçüm tam SDK maliyeti değildir; farklı aşamaları aynı işmiş gibi
karşılaştırmak yanlış olur. Elle çözücü ürün koduna alınmadı.

## Tekrar üretim ve ham sonuçlar

[Düzeneğin README'si](../../../prototip/rag-cozme/README.md), kaynakları ve
kilidi depodadır. Host örneği `crates/kalfa-plugin/examples/g6_cozme.rs`.
Her satırda boyut/adet/yol, yanıt boyutları, tekrar ve örnek sayısı, medyan,
p95, en küçük süre ve bit eşleşmesi bulunur:

- [Windows yerel](2026-10-09-kalfa-g6-windows-yerel.jsonl)
- [Windows wasm](2026-10-09-kalfa-g6-windows-wasm.jsonl)
- [WSL yerel](2026-10-09-kalfa-g6-wsl-yerel.jsonl)
- [WSL wasm](2026-10-09-kalfa-g6-wsl-wasm.jsonl)

**Düzeneğin doğrulaması:** Üç prototip testi, yerel ve wasm clippy ve biçim
denetimi iki platformda geçti. Aynı dört Rust ölçüm dosyasının SHA-256
değerleri iki platformda eşleşti. Ölçüm kodu eklendikten sonraki depo
kontrolleri:

| Kontrol | Windows | WSL2 Ubuntu 24.04, `kalfatest` |
|---|---|---|
| Tam workspace testleri | 994 geçti, 0 düştü, 13 atlandı | 936 geçti, 0 düştü, 13 atlandı |
| E2e özellikli testler | 114 geçti | 113 geçti |
| Düz, e2e ve wasm clippy; fmt | Temiz | Temiz |

Prototip workspace dışında kalır; ürün SDK'sı ve kök Cargo.lock değişmedi.
WSL'deki geçici kaynak kopyası kaldırıldı; mevcut paylaşılan derleme önbelleği
ve canlı indeks korundu.

## Sınırlar ve kalan işler

Bu aynı makinede, geçerli kayıtlardan türetilmiş yanıtlarla yapılan bir
yerel ölçümdür. Canlı ağ, JSON'un host tarafında işlenmesi/kopyalanması,
uçtan uca arama, farklı işlemci ve kötü yanıt maliyeti ölçülmedi. Sayı
dizisindeki ondalıkların uzunluğu yanıt büyüklüğünü etkiler; burada aynı
`f32` değerlerin `serde_json` gösterimi kullanıldı. Belleğin tepe değeri
ayrıca ölçülmedi. Başka makinelere aynı milisaniyeler vaat edilmez.

G6'nın mevcut uygulama kararı için gereken kıyas yapıldı. G4 gecikme,
G5 kalite/talimat/boyut/parça/aday sayısı, G8 paralel gömme ve tam ilk
indeksleme token maliyeti bu ölçümle kapanmaz. Mevcut canlı indeks korundu.
