# Kalfa

**EVREN LLM Gateway için Türkçe, açık kaynak ajan harness'ı.**

Kalfa, Rust ile yazılmış bir masaüstü ajan uygulamasıdır. EVREN'in model filosunu
kullanarak dosya okur, yazar, arama yapar ve komut çalıştırır — tamamı senin
onayınla. Araçlarını WebAssembly eklentileriyle genişletebilirsin.

> ⚠️ **Durum: tasarım aşaması.** Henüz kod yok. Tasarım belgesi
> [`docs/superpowers/specs/2026-09-21-kalfa-design.md`](docs/superpowers/specs/2026-09-21-kalfa-design.md)
> altındadır.

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
| **İzinler** | Yetenek tabanlı; çalışma alanı sınırı aşılmaz, kalıcı "her şeye izin ver" yoktur |
| **Sağlayıcı** | EVREN LLM Gateway — native tool calling, streaming, `reasoning`, kredi takibi |

## Doğrulanmış API gerçekleri

Tasarım varsayıma değil, canlı API'ye atılmış isteklere dayanıyor. Ham yanıtlar
[`research/evren-probe/`](research/evren-probe/) altında fixture olarak duruyor
ve ayrıştırıcı testleri bunlara karşı koşacak.

## Lisans

MIT ([`LICENSE-MIT`](LICENSE-MIT)) veya Apache-2.0
([`LICENSE-APACHE`](LICENSE-APACHE)) — hangisini istersen.

---

> **Muafiyet:** Kalfa bağımsız bir açık kaynak çalışmasıdır. T.C. Cumhurbaşkanlığı
> Savunma Sanayii Başkanlığı veya EVREN platformu ile resmî bir bağı, onayı ya da
> desteği yoktur. "EVREN uyumlu" ifadesi yalnızca teknik uyumluluğu tanımlar.
