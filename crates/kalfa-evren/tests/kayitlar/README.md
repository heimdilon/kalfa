# EVREN kayıtları (aşama 4 ön ölçümü ve 5a)

Bu dizindeki dosyalar EVREN'e atılan gerçek isteklerin **ham yanıtlarıdır**
(2026-10-07). `research/evren-probe/fixtures/` ile aynı türdendir; `research/`
ağacına dokunulmadığı için (AGENTS.md) aşama 4'ün kayıtları burada durur.
Yoklamanın ayrıntısı:
[ön ölçüm §1](../../../../docs/superpowers/bulgular/2026-10-07-kalfa-4-on-olcum.md).

| Dosya | Ne kanıtlıyor |
|---|---|
| `embeddings_qwen3_1024.json` | `qwen3-embedding-8b`, `dimensions: 1024`: OpenAI biçimi, `usage.evren`; yanıttaki model adı `qwen/qwen3-embedding-8b` |
| `embeddings_qwen3_base64.json` | `encoding_format: "base64"`, 4096 boyut |
| `rerank_qwen3.json` | `POST /v1/rerank`, Cohere biçimi; `qwen3-reranker-8b` isteği `qwen/qwen3-vl-reranker-8b`'ye yönleniyor |
| `error_embedding_context_length.json` | Tek girdi 40 960 token sınırını aşınca `400`, `code: "bad_request"` |
| `error_chat_context_length_glm.json` | 12 Ekim 2026: `glm-5.3`'e `max_tokens: 10000000` → `400`, `code: "context_length_exceeded"`, pencere 524 288 ([5a tasarımı §4](../../../../docs/superpowers/specs/2026-10-12-kalfa-5a-baglam-yonetimi-design.md)) |
| `error_chat_context_length_deepseek.json` | Aynı yoklama, `deepseek-v4-flash`, pencere 1 048 576 |
| `error_chat_context_length_qwen.json` | Aynı yoklama, `qwen3.8-flash-next`, pencere 262 144 |
