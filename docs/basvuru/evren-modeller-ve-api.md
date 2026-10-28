# EVREN — Modeller ve API başvuru belgesi

**Kaynak:** [EVREN Modeller ve API](https://evren.ssyz.org.tr/llm/models)

**İnceleme tarihi:** 23 Ekim 2026, TSİ

**Hazırlayan:** Codex, Orkun Koray Güngör'ün isteğiyle

Bu belge, resmî sayfanın teknik içeriğini çevrimdışı başvuru için yeniden
düzenler. Sekiz görev sekmesi, dokuz uç kartı, 15 satırlık model tablosu ve
modellerin ayrıntı pencereleri okunmuştur. Tekrarlanan cURL/Python/JavaScript
örnekleri yerine aynı biçimleri gösteren kısa örnekler kullanılmıştır.
Hesap kimliği, bakiye, anahtar ve oturum bilgisi alınmamıştır.

Buradaki ana kaynak **23 Ekim'de görünen belge sözleşmesidir**. Aynı gün
yapılan ayrı dar canlı gözlemler kaynaklarıyla işaretlenmiştir; Kalfa/DSH
destek beyanı değildir. Model erişimi, fiyat ve limit değişikliklerinde
güncel kaynak/katalog tekrar kontrol edilir. Belge hazırlanırken örnekler
çalıştırılmamıştı; sonraki dört model deneyi §6–7'de ayrılır. Açıklar §12'dedir.

## 1. Adres, kimlik ve kullanım şartları

| Konu | Belgelenen değer |
|---|---|
| API kökü | `https://evren-llmapi.ssyz.org.tr` |
| OpenAI biçimindeki istemcinin tabanı | `https://evren-llmapi.ssyz.org.tr/v1` |
| Kimlik başlığı | `Authorization: Bearer <anahtar>` **veya** `X-API-Key: <anahtar>` |
| JSON gövdesi | `Content-Type: application/json` |
| Model kimliği | Kısa `api_name` veya tam depo adı |
| Otomatik yönlendirme | `model: "auto"`; ayrı bir model satırı değildir |

Anahtar oluşturma sayfası `/api-keys`'tir. Kaynağın başlangıç sırası:

1. `GET /v1/terms/status`: `current_version`, `is_material`, `accepted`.
2. `GET /v1/terms/text`: `version`, `doc_path`, `content`.
3. **Kullanıcı kabul etmeye karar verirse** `POST /v1/terms/accept`:
   gövde `{"version": <güncel sayısal sürüm>}`; örnek yanıt alanları
   `accepted_version`, `accepted_at`.
4. Şart kapısı açıldıktan sonra katalog ve üretim çağrıları.

Kabul eksikse `403 terms_not_accepted` döner. Sayfadaki örnek sürüm `1`
sabitlenmez; güncel sürüm durum ucundan gelir. Kalfa ajanları canlı kabul
çağrısı yapmaz; bu adım kullanıcıya aittir.

**Ayrı canlı gözlem, 23 Ekim 2026:** dar deneyin ilk `GET /v1/terms/status`
isteği `401 authentication_error`, `code: "unauthorized"`, “Geçersiz API
anahtarı” döndürdü. Deney burada durdu (1 GET, 0 POST); şart durumu ve
model yanıt biçimleri doğrulanamadı. Bu gözlem yukarıdaki belge
sözleşmesinden ayrıdır. Deney ve fixture kaynağı: DSH–EVREN eklenti deposundaki 23 Ekim canlı deney raporu.

Kullanıcı anahtarı güncelleyip devamı bildirdikten sonra **21:57–21:58 TSİ**
aynı dar kapsam yeni kayıt dizininde geçti: şart `accepted: true`, katalog
16 model, kota uygun; 3 GET + 4 POST yanıtının tamamı 200. Kabul isteği
gönderilmedi. Bu sonuç yeni anahtarın ve o andaki erişimin gözlemidir.

## 2. Uç haritası

Yolların tamamı API köküne göredir. Son sütun, sayfada görülen girdi biçimini
özetler; eksiksiz OpenAPI şeması değildir.

| Yöntem/yol | İş | Girdi veya not |
|---|---|---|
| `POST /v1/chat/completions` | Sohbet, düşünme, araç, görsel, guard | `model`, `messages`; SSE seçilebilir |
| `POST /v1/responses` | Responses biçimi | `model`, `input`; araç tanımı düz yapıdadır |
| `POST /v1/ocr` | OCR | `model` + JSON `image`; model pencerelerinde multipart `file` de var |
| `POST /v1/audio/transcriptions` | ASR | Multipart `file`, `model`, `response_format` |
| `POST /v1/audio/speech` | TTS | JSON `model`, `input`, çıktı/akış seçenekleri |
| `POST /v1/embeddings` | Vektör | `model`, `input` dizisi |
| `POST /v1/rerank` | Yeniden sıralama | `model`, `query`, `documents` dizisi; Cohere/Jina biçimi |
| `POST /v1/systemone` | Tipli karar | `model`, `state`, `questions`; yalnız clef için `images` |
| `GET /v1/models` | Erişilebilir modeller ve fiyat | Hesaba göre katalog |
| `POST /v1/systemone/batch` | Toplu karar | Yalnız laya; `states`, ortak `questions` |
| `GET /v1/quota?model=<api_name>` | Kota ve eşzamanlılık | Model bağlamında güncel kullanım/sınırlar |
| `POST /v1/messages` | Anthropic Messages biçimi | Kaynak akış/araç/görsel ve `count_tokens` uyumundan söz ediyor |

Son üç satır ayrı uç kartı değildir; görev açıklamalarında veya teknik
notlarda belgelenir. Messages için ayrıntılı istek/yanıt örneği gösterilmez.

### Temel sohbet örneği

```sh
curl -s 'https://evren-llmapi.ssyz.org.tr/v1/chat/completions' \
  -H "Authorization: Bearer $EVREN_API_KEY" \
  -H 'Content-Type: application/json' \
  -d '{"model":"glm-5.3","messages":[{"role":"user","content":"Kısa bir selamlama yazın."}],"max_tokens":4096}'
```

Örnek sohbet yanıtında `id`, `object`, `created`, `model`, `choices[]` ve
`usage` vardır. Asistan metni `choices[].message.content`, bitiş nedeni
`finish_reason` içindedir. İstenen kısa ad ile yanıtın yönlendirilmiş tam
model adı aynı olmak zorunda değildir.

## 3. Model tablosu — bu tarihteki görünüm

`B` milyar parametreyi gösterir. Bağlam sütunu sayfanın sunduğu değerdir;
mimarinin olası üst sınırıyla karıştırılmaz. Modalite, her uçta her girdi
biçiminin açık olduğunu tek başına kanıtlamaz.

| Kısa kimlik | Tam depo/yönlendirme adı | Parametre | Sunulan bağlam | Girdi/modalite | Ana iş ve lisans |
|---|---|---|---|---|---|
| `gemma-4-31b` | `google/gemma-4-31b` | 31,3B dense | 256K | metin, görsel | Sohbet/belge; Gemma |
| `qwen3.8-flash-next` | `qwen/qwen3.8-flash-next` | 180B, ~6B aktif | 256K | metin, görsel | Sohbet/ajan; Apache-2.0 |
| `deepseek-v4.1-flash` | `deepseek/deepseek-v4.1-flash` | 763B, ~16B aktif | 1M | metin, görsel | Kod/ajan; MIT |
| `mimo-v2.6-pro` | `xiaomi/mimo-v2.6-pro` | 1.020B, ~42B aktif | 1M | metin, görsel, video, ses | Uzun ajan işleri; MIT |
| `glm-5.3` | `zai/glm-5.3-fp8` | 753B, ~40B aktif | 512K | metin | Muhakeme/kod; Z.ai GLM |
| `qwen3-vl-30b` | `qwen/qwen3-vl-30b-a3b` | 31,1B, ~3B aktif | 256K | metin, görsel | Görsel sohbet; Apache-2.0 |
| `qwen3-embedding-8b` | `qwen/qwen3-embedding-8b` | 7,6B dense | 40K | metin | Vektör; Apache-2.0 |
| `qwen3-reranker-8b` | `qwen/qwen3-vl-reranker-8b` | 8,8B dense | 4K | metin | Sıralama; Apache-2.0 |
| `qwen3-guard-4b` | `qwen/qwen3guard-gen-4b` | 4,4B dense | 32K | metin | İçerik sınıflama; Apache-2.0 |
| `clef` | `Cloudflare/clef` | 27B | 16K / 16.384 | metin, görsel | Karar; Apache-2.0 |
| `laya` | `evren/laya-tr-v5` | 0,32B | varsayılan 1.024, `max_len` ile 8.192 | metin | Karar; Apache-2.0 |
| `deepseek-ocr-2` | `deepseek/deepseek-ocr-2` | 3,4B dense | 8K | metin, görsel | OCR; MIT |
| `dots-ocr` | `rednote/dots.mocr` | 3,0B dense | 128K | metin, görsel | OCR/SVG; MIT |
| `qwen3-asr-1.7b` | `qwen/qwen3-asr-1.7b` | toplam 2,3B | 32K; ses penceresi ~30 sn | tabloda metin; uç girdisi ses | ASR; Apache-2.0 |
| `voxcpm2` | `openbmb/VoxCPM2` | 2B | 4K; metin sınırı 4.096 karakter | metin, ses | TTS; Apache-2.0 |

Ek model notları:

- GLM mimarisi için 1M'den söz edilse de tabloda bu kümede sunulan sınır
  **512K**'dır.
- DeepSeek v4.1 için 1–100 düşünme bütçesi belirtilir; sayfa bu notta ilgili
  parametrenin ayrıntılı tel şemasını göstermez.
- Qwen VL'nin video yeteneği model düzeyinde bulunsa da gateway video
  girdisini bu model için kapalı bildirir. Reranker'ın VL omurgası da
  gateway'de görsel belge kabulü anlamına gelmez.
- Gömme modeli 32–4096 boyut aralığında Matryoshka desteği bildirir.
- Lisanslar kaynak tablonun etiketleridir; model dağıtım koşulları için asıl
  model lisansı ayrıca okunur.

## 4. Düşünme, akış ve araçlar

### Düşünme ve SSE

`max_tokens` düşünme ile nihai çıktının ortak bütçesidir. Kaynak DeepSeek/
Qwen için en az 2048, örneklerde 4096 önerir. Bütçe düşünürken biterse
`finish_reason: "length"` ile eksik veya boş nihai metin görülebilir.

```json
{
  "model": "deepseek-v4.1-flash",
  "messages": [{"role":"user","content":"Çözümün adımlarını açıklayın."}],
  "max_tokens": 4096,
  "stream": true,
  "stream_options": {"include_usage":true}
}
```

Düşünme deltası `delta.reasoning` veya `delta.reasoning_content`, nihai metin
`delta.content` içindedir. SSE'deki yaklaşık 10 saniyelik `: keep-alive`
yorumları JSON olayı olarak ayrıştırılmaz. `thinking: false` sohbet
modellerinde düşünmeyi kapatmak için belgelenir. Kaynak anahtar bazında da
varsayılan düşünme ayarı bulunduğunu, istekte açık parametrenin öncelikli
olduğunu belirtir; bu belge hiçbir hesap ayarını değiştirmez.

Kaynak ayrıca istem içindeki ham `<think>` / `</think>` belirteçlerinin bazı
modellerin düşünme sınırını etkileyebildiğini, literal metin gerekiyorsa
kaçışlı gösterilmesini önerir.

### İki farklı araç şeması

Chat Completions'ta ad/şema `function` altında; Responses'ta doğrudan araç
nesnesindedir. Parametresiz fonksiyonda boş `properties: {}` kabulü belirtilir.

```json
{
  "model":"glm-5.3",
  "messages":[{"role":"user","content":"Örnek kaydı okuyun."}],
  "tools":[{"type":"function","function":{
    "name":"kayit_oku","description":"Örnek kayıt okuma",
    "parameters":{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]}
  }}],
  "tool_choice":"auto"
}
```

Responses karşılığı `input` ve `tools[].name`, `tools[].description`,
`tools[].parameters` kullanır. Kaynağın Codex CLI için işaret ettiği uç
`/v1/responses`'tır. Bu bilgi DSH adaptörünün bu ucu uyguladığı anlamına gelmez.

## 5. Görsel sohbet

User mesajındaki `content` bir parça dizisidir:

```json
{
  "model":"gemma-4-31b",
  "messages":[{"role":"user","content":[
    {"type":"text","text":"Görüntüyü açıklayın."},
    {"type":"image_url","image_url":{"url":"data:image/png;base64,<GÖRSEL_BAYTLARI>"}}
  ]}],
  "max_tokens":1024
}
```

Yer tutucu gerçek base64 değildir. Gateway uzak HTTP görselini indirmez;
belge bu girdiye 400 döndüğünü söyler. Vision sekmesi Gemma, Qwen VL ve
MiMo için en çok **8** görsel belirtir. MiMo ayrıca gömülü `video_url`,
`input_audio` ve `audio_url` girdilerinden söz eder; bunların ayrıntılı
gövde örneği bu sayfada yoktur.

**Kaynak içindeki farklı tarif:** Vision sekmesi 8 üstünü 400 olarak
anlatırken Gemma'nın ayrıntı yardım metni, geçmişteki görsellerin de sayılıp
eskilerinin notla değiştirilebildiğini söyler. Doğrudan API davranışı ile
istemci geçmiş yönetimi bu kayıttan kesin ayrıştırılamaz; yeni uygulama
bunun için fixture/ölçüm istemelidir. DSH eklentisinin yerel sınırı 4'tür.

## 6. Tipli karar — laya ve clef

İki model `/v1/systemone` kullanır; serbest metin üretmez. `state` metin veya
JSON belge olabilir. `questions`, kimlikten soru nesnesine haritadır.
Kaynak üst sınırı **64 soru**dur; DSH aracının yerel sınırı **32**'dir.

**Ayrı canlı gözlem, 23 Ekim 21:58 TSİ:** laya ve clef, kısa sentetik
metindeki üç türü aynı istekte kabul etti. Choice `criteria` haritası,
score sıralı `criteria` ve kesirli sonuç/legend ölçüldü. Laya'nın
`answer_confidence`/`confidence` farkı ve `usage.truncated: false` alanı
görüldü; clef bu iki ek alanı döndürmedi. Laya'nın gerçek yönlendirme
kimliği `convaiinnovations/laya` idi. Bu gözlem uzun metinde kırpılmayı veya
ürünün eski `options` gönderimini doğrulamaz.
Girdi/yanıt kayıtları DSH–EVREN eklenti deposundadır (`test/canli/kayitlar/2026-10-23-2157/`).

**Ürün eşlemesi (23 Ekim sonrası, çevrimdışı):** `evren_karar_ver` choice
sorularını `options` dizisinden açıklamalı `criteria` haritasına çeviriyor;
score sıralı `criteria` dizisini koruyor. Araç şeması seçenek açıklaması
taşımadığı için tel haritasında anahtar kendi adıyla açıklanır. `confidence`
dağılım sivriliği, `answerConfidence` verilen yanıtın olasılığı olarak ayrı
sunulur. Kırpılma `kirpilma` alanında laya `usage.truncated`, clef
`input_tokens == 16384` kaynağına göre raporlanır; alan yokluğu “kırpılmadı”
sayılmaz. Clef için `attachmentIds` ile en çok 4 yetkili görsel,
`images` veri adresleri olarak gönderilir; 4 MiB/görsel ve 8 MiB/toplam
sınırı ağdan önce uygulanır. Laya görsel kabul etmez. Bu görsel eşlemesi
resmî belge sözleşmesine dayanır; **canlı görsel U07 kabulü değildir** ve
yerel görsel küçültme/token tahmini yapılmaz. Bu, yeni bir canlı API
ölçümü değildir.

| Tür | Belgede girdi | Yanıtta görülen alanlar |
|---|---|---|
| `noul` | `type`, `instructions` | `type`, `noul` (0–1) |
| `choice` | `type`, `instructions`, seçenek/açıklama haritası `criteria` | `choice`, `probabilities`, `confidence`; laya'da ayrıca `answer_confidence` |
| `score` | `type`, `instructions`, sıralı metin dizisi `criteria` | `score`, `legend`; laya ayrıntısı olasılık dağılımından da söz eder |

```json
{
  "model":"laya",
  "state":{"body":"Sentetik bir öncelik değerlendirmesi."},
  "questions":{
    "sinif":{"type":"choice","instructions":"Kategori seçin.","criteria":{"a":"Birinci sınıf","b":"İkinci sınıf"}},
    "oncelik":{"type":"score","instructions":"Önceliği değerlendirin.","criteria":["düşük","orta","yüksek"]},
    "uygun":{"type":"noul","instructions":"Kapsamda mı?"}
  }
}
```

`score` sıralı düzeyler arasındaki **beklenen değer**dir; üç düzeyde 1.4
geçerli bir örnektir. `legend` için biçim `{"0":"düşük","1":"orta","2":"yüksek"}`
olur. Yalnız tamsayı veya 0–1 aralığı varsayılmaz. Kaynaktaki örnek yanıt
kısaltılmıştır; `usage` ve `routing` alanları da gösterilir.

### Modele göre önemli farklar

| Konu | laya | clef |
|---|---|---|
| Metin okuma | Varsayılan 1.024; `max_len` ile en çok 8.192 token | 16.384 token |
| Kırpılma işareti | `usage.truncated: true` | `usage.truncated` yok; kaynak `input_tokens == 16384` değerini kırpılma işareti sayıyor |
| Toplu uç | `/v1/systemone/batch`, en çok 64 `states` | Desteklenmiyor; 400 |
| Dil/yönlendirme | Otomatik; İngilizce için `convaiinnovations/laya`, diğerleri Türkçe ince ayar; `routing.model` | `lang` yok sayılır |
| Güven eşiği | `answer_confidence` verilen yanıtın olasılığı; `confidence` dağılım sivriliğidir | `answer_confidence`/`low_confidence` yok; seçilen sınıfın `probabilities` değeri kullanılır |
| Ek seçenek | `min_confidence` ile `low_confidence` işareti | `max_len`, `lang`, `min_confidence` yok sayılır |
| Görsel | Kabul edilmez; 400 | `images` ile kabul edilir |

Laya için `model: "jev-1"` takma adı da belirtilir. DSH aracı açık `laya`
veya `clef` seçimi ister; batch/takma ad seçeneklerini açmış sayılmaz.

### Clef görselleri

`images` içinde PNG/JPEG/WebP veri adresleri; en çok **4**, tek görselde
**4 MiB**, toplam **8 MiB**. Uzak URL ve video kabulü kapalıdır. Görsel
sunucuda küçültülmeden okunur; 32×32 karo yaklaşık bir token olarak sayılır.
Kaynağın örnekleri 1024² için ~1.000, 2048² için ~4.100 token'dır. Metin ve
görseller aynı 16.384 token penceresini paylaşır. Yalnız görseller pencereyi
doldurursa 400; daha büyük toplamda metnin sessiz kırpılma riski belirtilir.
Kaynak uzun kenarın 1024 piksele küçültülmesini önerir; bu bir yerel işlem
önerisidir, bu belgeyle dönüştürücü kurulmamıştır.

**Ürün eşlemesi (çevrimdışı):** `evren_karar_ver` yalnız `clef` için
`attachmentIds` alır; yetkili oturum eklerini 4 MiB/görsel ve 8 MiB/toplam
sınırıyla `images` veri adreslerine çevirir. `laya` + görsel ağdan önce
reddedilir. Araç görseli küçültmez ve token bütçesi hesaplamaz; 16.384
penceresi/ sessiz kırpılma riski açık kalır. Gerçek EVREN'e görselli
`systemone` çağrısı canlı olarak denenmedi; bu satır U07 kabulü değildir.

Kaynağın Türkçe kalite karşılaştırmaları iç ölçüm olarak etiketlidir;
genel doğruluk garantisi değildir. Aynı sayfada clef çoğu uzun/karmaşık
kararda, laya ise bazı Türkçe saldırganlık tespitlerinde öne çıkarılır.

## 7. OCR

**Ayrı canlı gözlem, 23 Ekim 21:58 TSİ:** iki model de aşağıdaki JSON/data-PNG
girdisini kabul etti ve **`{"model": "<model adı>", "text": "<okunan metin>"}`**
JSON gövdesi döndürdü. 640×160 sentetik sayfanın iki satırı okundu; DeepSeek
ilk satır sonunda iki boşluk korudu. Bu alanlar ve boşluklar
gerçek fixture'larda (DSH–EVREN eklenti deposu)
kaydedildi. Gözlem küçük PNG içindir; PDF, multipart ve SVG ölçülmedi.

**Ürün eşlemesi (23 Ekim sonrası, çevrimdışı):** `evren_ocr` aracı yalnız
kanıtlanmış JSON `{model, image: data URL}` isteğini ve `{model, text}`
yanıtını açar. Açık `dots-ocr`/`deepseek-ocr-2` seçimi, en çok 4 sayfa, 2
eşzamanlı istek ve sayfa başına 64 KiB metin sınırı vardır; `ciktiBicimi`
yalnız `metin`dir. Multipart, PDF, yapısal tablo/SVG ve toplu arşiv modu bu
sözleşmeden çıkarılamaz; OCR metni güvenilmeyen düz metindir. Bu, canlı bir
OCR kabulü değildir.

`dots-ocr` ve `deepseek-ocr-2` sohbet yerine `/v1/ocr` kullanır. Sohbet ucuna
gönderimde belgelenen hata `400 endpoint_mismatch`'tır.

Görev sekmesi ve uç kartı JSON gövdesi gösterir:

```json
{"model":"dots-ocr","image":"data:image/png;base64,<GÖRSEL_BAYTLARI>"}
```

İki modelin ayrıntı pencereleri aynı uç için ayrıca multipart örneği gösterir:

```sh
curl -s 'https://evren-llmapi.ssyz.org.tr/v1/ocr' \
  -H "X-API-Key: $EVREN_API_KEY" \
  -F 'model=dots-ocr' -F 'file=@sayfa.png'
```

İki biçim **belgelenmiştir**; JSON/data-PNG yukarıdaki dar deneyde ölçüldü,
multipart ölçülmedi. Dots için tablo/şekil/SVG üretimi, DeepSeek OCR için
toplu arşiv kullanım alanı anlatılır. Biçim seçme alanı, sayfa hataları ve
SVG'nin güvenli gösterimi bu sayfada örneklenmez; küçük metin yanıtı bunların
garantisi değildir. PDF kabulü de bu iki örnekten çıkarılamaz.

## 8. ASR ve TTS

### ASR

`qwen3-asr-1.7b`, multipart `/v1/audio/transcriptions` kullanır.
`response_format` seçenekleri `srt`, `json`, `text`, `verbose_json` olarak
belgelenir. Örnek girdiler WAV'dır; başka ses türlerinin ayrıntılı izin
listesi yoktur. Model açıklamasında ~30 saniyelik ses penceresi ve Türkçe
desteği belirtilir; bu ifade tek başına bir gateway dosya süresi sınırı değildir.

```sh
curl -s 'https://evren-llmapi.ssyz.org.tr/v1/audio/transcriptions' \
  -H "Authorization: Bearer $EVREN_API_KEY" \
  -F 'model=qwen3-asr-1.7b' -F 'file=@ornek.wav' -F 'response_format=srt'
```

### TTS

| Alan/özellik | Belgelenen biçim |
|---|---|
| Yol | `POST /v1/audio/speech` |
| Model | `voxcpm2`; `tts-1` takma adı da kabul edilir |
| Metin | `input`, istek başına en çok 4.096 karakter |
| Dosya biçimleri | `mp3` varsayılan; `wav`, `flac`, `opus`, `pcm` |
| Hız | `speed`: 0,25–4,0 |
| Ses tarifi | `instructions`, İngilizce betimleme |
| Ses adı | Örnekte `voice: "default"`; OpenAI ses adlarının da kabulü belirtilir |
| Gerçek akış | `stream: true`, yalnız `response_format: "pcm"` |
| PCM | Başlıksız, 16 bit, mono, 48 kHz |
| Kapalı özellikler | Ses klonlama ve ses yükleme |

Uzun metnin cümle sınırlarından bölünüp tek dosyada birleştirildiği belirtilir.
Bu, aynı bölümdeki 4.096 karakter üstünü otomatik kabul ettiği anlamına
getirilmez. `with_streaming_response` ile HTTP yanıtını parça parça okumak
ile sunucunun `stream: true` üretim kipini seçmek ayrı işlemlerdir.

```json
{"model":"voxcpm2","input":"Kısa sesli duyuru.","response_format":"wav","voice":"default","speed":1.0}
```

## 9. Gömme, sıralama ve guard

### Gömme ve sıralama

```json
{"model":"qwen3-embedding-8b","input":["Birinci örnek","İkinci örnek"]}
```

Bu gövde `/v1/embeddings` içindir. Boyut ayarı desteği model açıklamasında
32–4096 olarak verilir; uç kartında ek alan örneği yoktur.

```json
{"model":"qwen3-reranker-8b","query":"Sorgu","documents":["A belgesi","B belgesi"]}
```

Bu gövde `/v1/rerank` içindir. Yalnız metin belgeleri; Qwen VL omurgasından
görsel destek çıkarılmaz. Bu sayfada tam gömme/rerank yanıtı örneklenmez.
Kalfa'nın ayrıca kayıtlı canlı fixture'ları bulunması farklı bir kanıttır.

### İçerik guard'ı

`qwen3-guard-4b`, `/v1/chat/completions` ile çağrılır. Yalnız user mesajı
istem denetimi, user + assistant mesajları yanıt denetimi içindir.
`message.content` içinde metinsel `Safety`, `Categories` ve yanıt denetiminde
`Refusal` satırları gösterilir. Safety değerleri `Safe`, `Controversial`,
`Unsafe`; örnek refusal değeri `Yes`'tir. Bu çıktı araç çalıştırma izni değildir.

## 10. Kota, maliyet, hata ve gözlem alanları

Bu bölüm sayfanın **tarihli ilanını** kaydeder. Koda sabit kota veya fiyat
yazmak için kullanılmaz.

- 1 Ocak 2027'ye kadar hesap başına günlük **15.000.000 token** test kotası;
  LLM/OCR/gömme/sıralama/karar ve tüm anahtarlar birlikte sayılır.
- Normal kısa dönem hacmi **500k TPM**; sayfa büyük tekil bağlamların bu
  hacmi aşabilmesine özel tolerans olduğunu belirtir. Bu tolerans sınırsız
  sürekli trafik anlamına gelmez.
- Prefix-cache girdisi günlük token hesabına **%3** ağırlıkla yazılır.
- `GET /v1/quota` kalan günlük kota ve sıfırlanmayı; model sorgusu
  `concurrency_limit`, `concurrent_requests`, gerekirse
  `model_concurrency_limit` alanlarını bildirir.
- Kaynak `x-ratelimit-*`, `x-evren-daily-remaining-tokens` ve
  `X-Evren-Credits-*` başlıklarından söz eder. Tam başlık izin listesi değildir.
- Kaynak 1 Ocak 2027'ye kadar test/entegrasyon kullanımını 0 CR olarak
  ilan eder. Belgeyi okumak canlı çağrı veya hesap işlemi onayı değildir.

| HTTP/kod | Kaynağın açıklaması |
|---|---|
| 403 `terms_not_accepted` | Kullanım şartları kapısı |
| 400 `endpoint_mismatch` | Model yanlış uçtan çağrılmış |
| 429 `concurrency_limit_exceeded` | Hesap genelinde eşzamanlılık dolu |
| 429 `fair_share_limit_exceeded` | Yoğun modelde hesaba düşen pay dolu |
| 429 `rate_limit_exceeded` | Anahtar istek hızı sınırı |
| 429 `token_rate_limit_exceeded` | Kısa token penceresi dolu; `error.evren.resets_at` |
| 429 `daily_token_limit_exceeded` | Hesap günlük kotası dolu |
| 503 `model_capacity_exhausted` | Model kapasitesi geçici olarak dolu |

429 yanıtlarında `Retry-After`, ek ayrıntıda `error.evren` kullanılır.
Resmî SDK'ların kararını esas olarak HTTP durumu belirler; EVREN'e özgü
ek alanların ayrıca okunması gerekir. Akış başladıktan sonraki yeniden
deneme davranışı bu tablodan çıkarılmaz.

Katalog örneğinde `object: "list"`, `data[]` içinde `id`, `task`,
`modalities`, `pricing` vardır. Fiyat örneğinde `prompt_token_price`,
`completion_token_price`, `currency`, `free_until` gösterilir. Bu örnek
hesabın gerçek zamanlı erişim listesinin yerine geçmez.

## 11. İstemci adresleri ve kısa SDK örneği

```python
import os
from openai import OpenAI

istemci = OpenAI(
    base_url="https://evren-llmapi.ssyz.org.tr/v1",
    api_key=os.environ["EVREN_API_KEY"],
)
sonuc = istemci.chat.completions.create(
    model="glm-5.3",
    messages=[{"role":"user","content":"Kısa bir selamlama yazın."}],
)
print(sonuc.choices[0].message.content)
```

Bu kod yalnız başvuru örneğidir; paket kurulmamış ve istek gönderilmemiştir.
Kaynakta Python yanında cURL ve JavaScript karşılıkları bulunur. ASR/TTS
örnekleri SDK'nın `audio.transcriptions` / `audio.speech` yüzeylerini kullanır.

Kaynak, Anthropic istemcilerinde tabanın `/v1` **olmadan** verilmesini ve
model adlarının EVREN kimliği olmasını ister. Claude Code örneğinde
`ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_MODEL`,
`ANTHROPIC_DEFAULT_OPUS_MODEL`, `ANTHROPIC_DEFAULT_SONNET_MODEL`,
`ANTHROPIC_DEFAULT_HAIKU_MODEL`, `CLAUDE_CODE_SUBAGENT_MODEL` ve
`CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` değişkenleri gösterilir.
Bu kayıt güncel Claude/Codex kurulum talimatı veya ayar değişikliği değildir.

## 12. Kanıt boşlukları ve Kalfa için kullanım

| Konu | Bu belgeden çıkarılabilen / açık kalan |
|---|---|
| `score` | Sıralı `criteria`, kesirli beklenen değer ve `legend`, iki modelin 21:58 canlı yanıtıyla ölçüldü; ürün aracı bunu katı yanıt denetimiyle kullanıyor |
| `choice` | `criteria` haritası iki modelde canlı çalıştı; ürün aracı 23 Ekim sonrası `options` girdisini bu haritaya çeviriyor. Seçenek açıklaması araç şemasında yoktur |
| Karar kırpılması | Laya `usage.truncated`, clef `input_tokens == 16384` ile farklı bildiriyor; ürün `kirpilma` alanında ayırıyor, alan yokluğunu “kırpılmadı” saymıyor |
| Görselli karar | Clef `images` resmî belgede 4 görsel/4 MiB/8 MiB olarak tanımlı; 0.2.9 adayı bunu yetkili eklerden çevrimdışı uyguluyor. Canlı görselli `systemone`, görsel küçültme/token bütçesi ve uzun metin kırpılması açık |
| OCR | Küçük JSON/data-PNG girdisine iki modelde `{model,text}` canlı kaydı var; ürün `evren_ocr` yalnız bu düz metin davranışını açıyor. Multipart, PDF, tablo/SVG, büyük girdi ve hata biçimleri açık |
| ASR | Multipart ve dört çıktı biçimi belgeli; tam yanıt, tüm dosya türleri ve kesin süre/bayt sınırları yok |
| TTS | Uç/alanlar/PCM biçimi belgeli; hata ve akış sınırı fixture'ı canlıdan alınmadı |
| Video/ses sohbeti | MiMo için alan adları var; ayrıntılı gövdeler/limitler yok |
| Vision görsel sayımı | Sekme ile Gemma yardım metnindeki geçmiş/düşürme farkı ölçülmeli |
| Model listesi/fiyat | Tarihli görünüm; güncel `GET /v1/models` ve hesap erişimi yerine geçmez |

DSH'nin daha dar 4 görsel, 32 soru, 16 seçenek/düzey, 24.000 karakter ve
64 KiB çıktı sınırları yerel ürün kurallarıdır; gateway'in sınırıymış gibi
sunulmaz. Canlı kabul ve iki platform için DSH–EVREN eklenti deposundaki
durum belgesi ve ilgili bulgu raporu esastır.

### Güncelleme yöntemi

Kaynak değiştiğinde tarih ve etkilenen bölümler birlikte güncellenir.
Yeni istek/yanıt alanı önce bu belgeye kaynağıyla yazılır; ürün desteği
ilgili fixture/olumsuz test ve kabul kanıtı geldikten sonra ilan edilir.
Gerçek örnekler kaydedilecekse anahtar, kimlik başlığı ve hesap bilgisi
ayıklanır; belgede yazan örnek canlı kayıt etiketi almaz.
