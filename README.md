# Kalfa

**EVREN LLM Gateway için Türkçe, açık kaynak ajan harness'ı.**

Kalfa, Rust ile yazılmış bir masaüstü ajan uygulamasıdır. EVREN'in model filosunu
kullanarak dosya okur, yazar, arama yapar ve komut çalıştırır — tamamı senin
onayınla. Araçlarını WebAssembly eklentileriyle genişletebilirsin.

> **Durum: masaüstü uygulaması Windows ve Linux'ta çalışıyor; kabuk komutları
> Linux'ta yalıtımda çalışıyor.** Rust workspace'inde EVREN istemcisi
> (`kalfa-evren`), arayüzden bağımsız oturum çekirdeği (`kalfa-core`), dosya
> araçları ve kabuk aracı (`kalfa-tools`), komut çalıştırma, süreç sınırları ve
> Linux yalıtımı (`kalfa-sandbox`) ve masaüstü uygulaması (`kalfa-uygulama`,
> Tauri kabuğu `kalfa`) bulunuyor; Svelte arayüzü `ui/`'dadır. Linux'ta
> komutlar bubblewrap ve seccomp ile yalıtılır. Windows'ta yalıtım henüz yok:
> komutlar yalıtımsız çalışır ve her biri yerel pencerede onaylanır. Windows
> yalıtımı ve eklentiler sonraki aşamalarda.
> [Geliştirme durumu ve devam noktası](docs/GELISTIRME_DURUMU.md).

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

Linux'ta `kalfa` crate'i WebKitGTK geliştirme paketlerini ister
(`libwebkit2gtk-4.1-dev`).

Linux'ta kabuk komutlarını yalıtmak için `bubblewrap` gerekir (`sudo apt install
bubblewrap`). Yoksa Kalfa bunu durum çubuğunda ve Ayarlar'da söyler; komutlar
yalıtımsız çalışır ve her biri yerel pencerede onaylanır. Gerçek yalıtım
testleri root olmayan bir kullanıcıyla koşar:

```sh
cargo test -p kalfa-sandbox --features yalitim-testleri --test yalitim
cargo test -p kalfa-tools --features yalitim-testleri --test uyumluluk
```

Arayüz (`ui/`, Node `^22.22.2 || >=24.15.0`):

```sh
cd ui && npm ci && npm run check && npm test && npm run build
```

Uçtan uca testler gerçek uygulamayı sahte EVREN'li bir test derlemesiyle
`tauri-driver` üzerinden sürer:

```sh
cargo test -p kalfa-uygulama -p kalfa --features e2e
cd e2e && npm ci && npm test        # Linux'ta: xvfb-run -a npm test
```

- `tauri-driver` gerekir: `cargo install tauri-driver --locked`.
- **Windows:** WebView2 sürümüyle aynı `msedgedriver` gerekir. PATH'te değilse
  yolu `KALFA_E2E_EDGEDRIVER` ile verin.
- **Linux:** `webkit2gtk-driver`, `xvfb` ve `bubblewrap` paketleri gerekir.

`npm test` önce arayüzü test kancasıyla (`ui/dist-e2e`), sonra uygulamayı
`target/e2e/` altına derler. Ölçümler `e2e/sonuclar/`'a yazılır.

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
