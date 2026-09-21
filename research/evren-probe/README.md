# EVREN LLM API — Probe Kayıtları (2026-09-21)

Bu klasördeki dosyalar, Kalfa'nın tasarımı sırasında EVREN LLM Gateway'e
(`https://evren-llmapi.ssyz.org.tr/v1`) atılan gerçek isteklerin **ham yanıtlarıdır**.
Uydurma örnek değildirler.

`kalfa-evren` crate'inin ayrıştırıcı testleri bu dosyalara karşı koşar. Böylece
"bizim varsaydığımız şema" ile "gateway'in gerçekte döndürdüğü şema" arasında
sessiz bir ayrışma oluşamaz.

| Dosya | Ne kanıtlıyor |
|---|---|
| `models.json` | Katalog şeması: OpenAI'da olmayan `task`, `modalities`, `pricing` alanları |
| `chat_tool_calls_blocking.json` | `glm-5.3`: native tool calling + **paralel çağrı** + `message.reasoning` + `usage.evren` |
| `chat_tool_calls_stream.sse` | `glm-5.3`: `: keep-alive` yorum satırları, parçalı argüman akışı (`I` + `stanbul` — düz ASCII, token sınırı), `delta.reasoning`, son parçada `usage.evren`'in **olmayışı** |
| `chat_json_schema.json` | `glm-5.3`: `response_format: json_schema` desteği (vLLM arkaplanı) |
| `chat_tool_calls_qwen_flash.json` | `qwen3.8-flash-next`: native tool calling + paralel çağrı + `reasoning` |
| `chat_tool_calls_stream_qwen_flash.sse` | `qwen3.8-flash-next`: akışta araç çağrısı; argüman parçaları `İ` → `z` → `mir` (token sınırı Türkçe karaktere denk geliyor) |
| `chat_json_schema_qwen_flash.json` | `qwen3.8-flash-next`: `json_schema` ile güvenlik denetçisi şeması; örnek enjeksiyonu `talimat_enjeksiyonu` olarak işaretledi |
| `chat_tool_calls_deepseek_flash.json` | `deepseek-v4-flash`: native tool calling + paralel çağrı + `reasoning` |
| `chat_tool_calls_stream_deepseek_flash.sse` | `deepseek-v4-flash`: akışta araç çağrısı |
| `error_model_not_found.json` | `error.evren.suggested_models` dizisi |
| `error_task_mismatch.json` | Görev uyuşmazlığı hatası (`task: guard` modelini sohbet ucuna çağırma) |
| `error_unauthorized.json` | Geçersiz anahtar → `401`, `code: "unauthorized"`; mesaj geçersiz anahtar için de "API anahtarı eksik" diyor |
| `terms_status.json` | Şart kapısı durum şeması |
| `terms_text.json` | Şart metni yanıt şeması |

## Notlar

- **`terms_status.json` kabul sonrası çekildi** (`accepted: true`). Kabul öncesi
  durumda `accepted: false` döner ve her uç — `/v1/models` dahil —
  `403 terms_not_accepted` verir. Şema aynıdır.
- Probe sırasında tüm modellerin `pricing` değeri `0.0 CR` idi (beta dönemi).
  Maliyet göstergesi bu tabloya değil, `X-Evren-Credits-*` yanıt başlıklarına
  dayanmalıdır.
- Yanıt başlıkları dosyalara kaydedilmedi; gözlemlenen başlıklar:
  `X-Evren-Credits-Held`, `X-Evren-Credits-Remaining`, `X-Evren-Routed-Model`,
  `X-Request-Id`, `X-Evren-Quota-Weight`.
- Hiçbir dosyada API anahtarı yoktur.
