# Kalfa

**EVREN LLM Gateway için Türkçe, açık kaynak ajan harness'ı.**

Kalfa, Rust ile yazılmış bir masaüstü ajan uygulamasıdır. EVREN'in model filosunu
kullanarak dosya okur, yazar, arama yapar ve komut çalıştırır — tamamı senin
onayınla. Araçlarını WebAssembly eklentileriyle genişletebilirsin.

> **Durum: çekirdek ve dosya araçları geliştiriliyor.** Rust workspace içinde
> EVREN istemcisi (`kalfa-evren`), arayüzden bağımsız oturum çekirdeği
> (`kalfa-core`) ve beş yerleşik dosya aracı (`kalfa-tools`) bulunuyor.
> Masaüstü arayüzü henüz uygulanmadı. [Geliştirme durumu ve devam noktası](docs/GELISTIRME_DURUMU.md).

## Geliştirme

Rust araç zinciri `rust-toolchain.toml` ile sabitlenmiştir. Depo kökünde:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Varsayılan testler sahte sağlayıcılar ve yerel HTTP sunucuları kullanır;
EVREN anahtarı gerektirmez. Canlı API testleri `#[ignore]` ile ayrıdır.
Bağlantı testleri Windows'ta sembolik bağlantı oluşturma izni gerektirir.

---

## Neden?

EVREN'in model filosu Türkçe için özel olarak güçlü — `qwen3-embedding-8b`
TR-TEB'de birinci, `qwen3-asr-1.7b` Türkçe teyitli — ve verinin yurt dışına
çıkmadığı bir altyapıda çalışıyor. Ama bu filoyu bir ajan harness'ı içinde
kullanmanın yerli bir yolu yok.

Mevcut ajan araçları ya başka sağlayıcılara bağlı, ya İngilizce birincil, ya da
EVREN'in kendine özgü sözleşmesini bilmiyor: şart kapısı, kredi başlıkları,
`error.evren` alt-nesnesi, `reasoning` alanı.

Kalfa bu boşluğu doldurmak için var.

## Adı nereden geliyor?

Ahi lonca geleneğinden: **çırak → kalfa → usta**. Kalfa, ustanın yönlendirmesiyle
iş yapan yetkin zanaatkârdır. Ürünün izin modeli de tam olarak budur — yetenekli,
ama son söz senin.

## Tasarımın ana hatları

| | |
|---|---|
| **Çekirdek** | Rust; UI'dan bağımsız, serileştirilebilir `Command`/`Event` sınırı |
| **Arayüz** | Tauri v2 + Svelte 5 — Windows ve Linux |
| **Eklentiler** | WebAssembly Component Model (WIT), yalıtılmış çalışır |
| **İzinler** | Yetenek tabanlı; dört kip; onay gösterilen işleme bağlı; kalıcı "her şeye izin ver" yoktur |
| **Kabuk** | İşletim sistemi düzeyinde yalıtım — Linux'ta bubblewrap, Windows'ta ayrı kullanıcı; varsayılan olarak ağsız |
| **Şeffaflık** | EVREN'e giden her içerik gönderim günlüğünde; turlar geri alınabilir |
| **Sağlayıcı** | EVREN LLM Gateway — native tool calling, streaming, `reasoning`, kredi gözlemi |

## Doğrulanmış API gerçekleri

Tasarım varsayıma değil, canlı API'ye atılmış isteklere dayanıyor. Ham yanıtlar
[`research/evren-probe/`](research/evren-probe/) altında fixture olarak duruyor
ve ayrıştırıcı testleri bunlara karşı koşacak.

## Lisans

Apache-2.0 — [`LICENSE`](LICENSE). Telif: Orkun Koray Güngör ([`NOTICE`](NOTICE)).

Eklenti geliştirmek için kullanılan `kalfa-plugin-sdk` ve eklenti sözleşmesinin
WIT dosyaları buna ek olarak [LLVM istisnası](LICENSE-LLVM-exception) taşır
(`Apache-2.0 WITH LLVM-exception`). Eklentini derleyip dağıtırken lisans ya da
NOTICE dosyası eklemek zorunda kalmazsın; eklentinin lisansını sen seçersin.

Katkılar, katkı yapılan kısmın lisansıyla kabul edilir (Apache-2.0 §5).

---

> **Muafiyet:** Kalfa bağımsız bir açık kaynak çalışmasıdır. T.C. Cumhurbaşkanlığı
> Savunma Sanayii Başkanlığı veya EVREN platformu ile resmî bir bağı, onayı ya da
> desteği yoktur. "EVREN uyumlu" ifadesi yalnızca teknik uyumluluğu tanımlar.
