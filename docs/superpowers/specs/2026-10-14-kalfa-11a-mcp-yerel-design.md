# Kalfa 1.1a — Yerel MCP sunucuları: Tasarım

**Tarih:** 2026-10-14.
**Durum:** Tasarım; kullanıcı bölüm bölüm onayladı (14 Ekim, bu çalışmanın
sohbeti). Uygulama planları yazılmadı.
**Taban:** `main`, `04f16fc` (`v0.1.0` ön sürümü yayımlandı).
**Üst belge:** [Kalfa tasarımı](2026-09-21-kalfa-design.md) §10, §12.2.

## 1. Amaç ve kabul ölçütü

Kullanıcı bilgisayarındaki MCP sunucularını (stdio) Kalfa'ya ekler; model
sunucuların araçlarını yerleşik araçlar gibi çağırır. Sunucular varsayılan
olarak kabuk aracının yalıtımında çalışır; kullanıcı sunucu bazında, açık
uyarı ve yerel onayla *Tam sistem erişimi*ne geçebilir.

Başarı:

- Depodaki test sunucusu Linux'ta (bwrap) ve Windows'ta (AppContainer)
  yalıtımlı başlar, araçları listelenir, çağrılır; çalışma alanı dışını
  okuyamaz, izin listesi dışına ağla çıkamaz.
- Onay kararı §5.1'deki tablonun her hücresinde testle doğrulanır; onaydan
  sonra değişen sunucu tanımıyla çağrı yapılmaz.
- MCP çıktısı ve araç tanımları güvenlik denetçisinden ve kısaltmadan geçer.
- Uygulama kapanınca ya da çalışma alanının son oturumu kapanınca süreç
  ağacı ölür; Windows'ta geride Kalfa AppContainer profili ya da `S-1-15-2-`
  ACE'si kalmaz.
- E2e: Ayarlar'dan test sunucusu eklenir, sohbette çağrılır, onaylanır,
  sonuç görünür.

## 2. Kararlar ve kapsam

**Kullanıcı kararları (14 Ekim):**

- MCP desteği hem yerel hem uzak sunucuları kapsayacak; iş iki aşamaya
  bölünür: **1.1a** yerel stdio (bu belge), **1.1b** uzak Streamable HTTP
  ve OAuth (ayrı tasarım).
- Yerel sunucular **varsayılan yalıtımlı**; sunucu bazında *Tam sistem
  erişimi* seçilebilir. Gerekçe: Claude Code ve Codex yerel sunucuları
  yalıtımsız çalıştırıp onayla denetliyor; Kalfa'nın iki platformda kabuk
  yalıtımı olduğu için daha güvenli varsayılan mümkün.
- Yapılandırma **Ayarlar ekranı + JSON içe aktarma**; çalışma alanından
  (`.mcp.json` gibi) sunucu gelmez.
- Araç çağrısı onayı **kabuk gibi** (§5.1).
- Yaklaşım **A**: `kalfa-sandbox`'a etkileşimli süreç eklenir; yalıtımlı ve
  tam erişimli sunucular aynı arayüzden başlar.

**Kapsam içinde:** stdio taşıması; MCP `tools` (`tools/list`, `tools/call`,
`notifications/tools/list_changed`, `notifications/cancelled`).

**Kapsam dışında:** uzak HTTP ve OAuth (1.1b); `resources`, `prompts`;
sunucunun istediği `sampling` ve `elicitation`; çalışma alanındaki
yapılandırma dosyaları; görsel/ses içeriğin modele gönderilmesi.

MCP çağrıları EVREN'e istek göndermez; oturum bütçesini (5b) tüketmez.
Sonuçları modele giden içerik olarak bağlam yönetimine (5a) girer.

## 3. Bileşenler

| Birim | Görevi | Bağımlılığı |
|---|---|---|
| `kalfa-sandbox` etkileşimli süreç (§4) | stdin/stdout borulu, süre sınırsız süreç; Linux bwrap+seccomp, Windows AppContainer+Job, yalıtımsız arka uç | Mevcut profiller, vekil ve köprü |
| `kalfa-mcp` (yeni crate) | Sunucu tanımı, `rmcp` istemcisi, el sıkışma, araç listesi, çağrı, zaman aşımı, yeniden başlatma; her MCP aracını `Arac` trait'ine sarar | `rmcp` 3.5.1 (`client`, `transport-async-rw`), `kalfa-core`, `kalfa-sandbox` |
| `kalfa-uygulama` MCP yöneticisi | Sunucu tanımlarını saklar, gizli ortam değerlerini kimlik deposuna yazar; sunucuları çalışma alanı bazında başlatır, oturumlara araçlarını verir | `kalfa-mcp`, `ayarlar.rs`, `anahtar.rs` |
| Arayüz (§8) | Ayarlar'da "MCP sunucuları"; rozetler; onay ve araç kartı | Mevcut Ayarlar ve kart bileşenleri |

Ajan döngüsü değişmez: MCP araçları oturumun araç listesine `Arac` olarak
girer (üst belge §10).

**Araç adı:** `mcp__<sunucu>__<araç>`. Sunucu adı Ayarlar'da
`[a-z0-9-]{1,32}` ile sınırlıdır; araç adı `[A-Za-z0-9_-]` dışına çıkarsa ya
da toplam ad 64 karakteri aşarsa araç kapatılır (§6.2).

**Sunucu tanımı** (uygulama verisindeki ayarlar dosyasında):

```text
ad, etkin, komut, argümanlar, ortam (ad → düz değer | kimlik deposu
anahtarı), erişim (Yalitimli | TamErisim), ağ izin listesi,
başlatma zaman aşımı (vars. 30 sn), çağrı zaman aşımı (vars. 120 sn)
```

**Ortam:** Kabuktaki izin listesi ilkesi geçerlidir (`ortam::uret`).
Sunucu yalnız temel izinli değişkenleri ve kendi tanımındakileri görür.
EVREN anahtarı ve Kalfa'nın iç değişkenleri asla geçmez.

## 4. Etkileşimli süreç (`kalfa-sandbox`)

Mevcut `Yalitici::baslat` stdin'i kapalı (`Stdio::null`) ve çıktıyı
toplanacak biçimde başlatır (`surec.rs`). MCP için ayrı bir giriş eklenir:

- `baslat_etkilesimli(istek) -> EtkilesimliSurec`; `EtkilesimliSurec`
  `AsyncWrite` (stdin), `AsyncRead` (stdout), satır bazlı stderr okuyucu ve
  `kapat()` verir.
- Süre sınırı ve çıktı sınırı yoktur; süreç kapatılana ya da ölene kadar
  yaşar. Kabuğun süreç sınırları (Windows Job Object'in etkin süreç sınırı,
  Linux'ta `prlimit`) aynen geçerlidir.
- `kapat()`: stdin kapanır, 2 sn beklenir, süreç ağacı öldürülür (Windows
  `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, Linux bwrap `--die-with-parent` ve
  mevcut `agac` kapanışı).
- Kip uyuşması (`kip_uyusmasi`) aynen uygulanır: yalıtımlı onaylanan
  sunucu yalıtım yoksa başlamaz, tersi de.
- Ağ: `ag_listesi` doluysa mevcut vekil/köprü yolu; boşsa ağ yok.

**Ön doğrulama G1 (plan 1.1a-1'den önce):** Windows'ta AppContainer içinde
`CreateProcessW` ile başlatılan sürecin stdin/stdout borularının kalıtımı,
uzun yaşaması ve Job Object kapanışı küçük bir prototiple ölçülür
(`prototip/` altında, 2c gibi). Engel çıkarsa bu belge güncellenir.

## 5. Güvenlik

### 5.1 Onay kararı

`kalfa-core::izin`'e `mcp_karari(kip, erisim, ilk_cagri, agli)` eklenir:

| Kip | Yalıtımlı sunucu | Tam erişimli sunucu |
|---|---|---|
| İnceleme | Araçlar modele sunulmaz | Araçlar modele sunulmaz |
| Sıkı, Proje | `Onayli` | `YerelOnayli`, her çağrı |
| Serbest | `Serbest`; sunucunun bu çalışma alanındaki ilk çağrısı ya da ağ listesi doluysa `Onayli` | `YerelOnayli`, her çağrı |

- "Bu oturumda izin ver" kısayolu yalnız yalıtımlı sunucularda, araç
  bazında. Tam erişimlide kısayol yok (yalıtımsız kabuk kuralı).
- Modelin sunucu ekleyen ya da değiştiren bir aracı yoktur.
- Bir sunucuyu *Tam sistem erişimi*ne geçirmek yerel pencerede onaylanır.

### 5.2 Mühür

Onay mühre şunları bağlar: sunucu adı, komut ve argümanlar, erişim kipi,
ağ listesinin özeti, gizli ortam değerlerinin SHA-256 özeti (değer değil),
araç adı ve argümanlar. Çağrı anındaki tanım mühürle uyuşmazsa çağrı
yapılmaz.

### 5.3 Windows hazırlığı

AppContainer çalışma alanının hazırlanmasını ister (aşama 2 §9.4). Alan
hazır değilse yalıtımlı sunucu başlamaz; hazırlık mevcut akıştaki gibi
yerel pencerede onaylanır.

### 5.4 Sırlar

Gizli ortam değerleri yalnız işletim sisteminin kimlik deposundadır;
ayarlar dosyasına, JSONL kayda, loglara ve gönderim günlüğüne yazılmaz.
JSON içe aktarmada bütün `env` değerleri varsayılan olarak gizli sayılır;
kullanıcı tek tek düz değere çevirebilir.

## 6. Dış kaynaklı içerik

### 6.1 Sonuçlar

MCP sonucu her kipte güvenlik denetçisinden (üst belge §9) ve mevcut araç
çıktısı kısaltmasından geçer; gönderim günlüğünde ve önizlemede görünür;
katı denetçi kipinde şüpheli sonuç tutulur.

İçerik dönüşümü: `text` olduğu gibi; `structuredContent` JSON metne;
`image`/`audio` yerine "[görsel içerik atlandı]" / "[ses içeriği atlandı]";
`resource_link` yalnız URI'si; `isError: true` araç hatası.

### 6.2 Araç tanımları

`tools/list` ile gelen ad, açıklama ve giriş şeması modele gider (tool
poisoning yüzeyi):

- Açıklama 1 KiB'de kesilir.
- Görünmez ya da yön değiştiren Unicode içeren araç kapatılır (mevcut
  onay ekranı denetimi).
- Geçersiz adlı ya da şeması JSON Schema olarak çözülemeyen araç kapatılır.
- Kapatılan araçlar Ayarlar'da nedeniyle görünür; modele gitmez.

## 7. Yaşam döngüsü

**Çalışma alanına bağlılık:** Yalıtım çalışma alanına özeldir; her
(sunucu, çalışma alanı) çifti için bir süreç. Tam erişimli sunucular da
çalışma alanında, aynı kuralla çalışır.

**Başlatma:** Çalışma alanında ilk oturum açılınca o alanın etkin
sunucuları paralel başlar: `initialize`, sonra `tools/list`. Başlatma zaman
aşımı sunucu tanımındadır. Başlamayan sunucu oturumu bekletmez: araçları o
tur gelmez, Ayarlar'da hata görünür, sohbette tek satırlık not çıkar.

**Çağrı:** model çağrısı → argüman şema doğrulaması → §5.1 kararı ve
onay → mühür denetimi → `tools/call` (çağrı zaman aşımı) → §6.1 dönüşüm →
kısaltma ve denetçi → model.

**İptal:** Tur iptalinde sunucuya `notifications/cancelled` gider, yerel
bekleme hemen bırakılır; geç gelen sonuç atılır. Süreç öldürülmez.

**Liste değişimi:** `notifications/tools/list_changed` gelince liste
yeniden okunur; değişiklik sonraki turda geçerlidir.

**Çökme:** Süreç ölürse sunucu *Durdu* olur, bekleyen çağrılar hata döner.
Sonraki çağrıda en çok 3 kez artan aralıkla (1, 4, 16 sn) yeniden
başlatılır; sonra *Hata*da kalır, Ayarlar'dan yeniden başlatılır.

**Kapatma:** Çalışma alanının son oturumu kapanınca ya da uygulama çıkarken
§4'teki `kapat()`.

**Kayıt ve kurtarma:** Çağrılar ve sonuçlar JSONL kayda diğer araçlar gibi
girer. Kurtarmada yarım MCP çağrısı yeniden çalıştırılmaz, yarım kaldı
olarak işaretlenir.

## 8. Arayüz

Ayarlar'da **MCP sunucuları** bölümü (WASM eklentilerinden ayrı; üst belge
§10.1):

- **Liste:** ad, rozet (*Yalıtılmış süreç* / *Tam sistem erişimi*), durum
  (Çalışıyor, Başlıyor, Durdu, Hata), araç sayısı, aç/kapat, yeniden başlat.
- **Ekle/düzenle formu:** ad, komut, argümanlar, ortam değişkenleri (gizli
  işaretiyle), ağ izin listesi, zaman aşımları, erişim. *Tam sistem
  erişimi* seçilince formda kalıcı uyarı; kaydederken yerel pencere.
- **JSON içe aktarma:** `{"mcpServers": {...}}` yapıştırılır, önizlenir;
  sunucular yalıtımlı ve kapalı eklenir. `url` içerenler "1.1b'de" notuyla
  atlanır.
- **Ayrıntı:** araçlar ve açıklamaları, kapatılan araçların nedeni, son
  hata, stderr'in son 50 satırı (yalnız yerel).

Sohbette MCP araç kartı mevcut kart biçimini kullanır; başlıkta sunucu adı
ve rozet görünür. Onay kartının "neden izin gerekiyor" satırı erişim
türünü söyler. Metinler `ui/src/dil/tr.ts`'te, "siz" diliyle.

## 9. Hatalar

| Durum | Davranış |
|---|---|
| Komut bulunamadı | *Hata*; "`<komut>` bulunamadı; PATH: …" |
| El sıkışma zaman aşımı | *Hata*; stderr özeti |
| Protokol sürümü uyuşmuyor | *Hata*; sunucunun bildirdiği sürüm |
| Çağrı zaman aşımı | Modele "araç <n> sn'de yanıt vermedi" |
| Sunucu çöktü | Bekleyen çağrılar hata; §7 yeniden başlatma |
| Onaydan sonra tanım değişti | Çağrı yapılmaz; mühür hatası |
| Yalıtım yok, sunucu yalıtımlı | Başlamaz; Ayarlar'da yalıtım durumu |

## 10. Test

- **`kalfa-sandbox`** (yalıtım testleri; Linux `kalfatest`, Windows gerçek
  AppContainer): stdin/stdout yankısı, ağ izin listesi, çalışma alanı dışı
  okuma reddi, kapanışta ağacın ölmesi, geride profil/ACE kalmaması.
- **`kalfa-mcp`:** depoda `rmcp` `server` özelliğiyle (yalnız dev
  bağımlılığı) yazılmış test sunucusu; senaryolar: araçlar, yavaş araç,
  çöken araç, liste değişimi, zehirli açıklama, büyük çıktı, `isError`.
- **`kalfa-core`:** `mcp_karari` tablosunun her hücresi; mühür değişimi.
- **Arayüz:** Vitest; form, içe aktarma, rozetler, uyarı.
- **E2e:** test sunucusunu ekle, çağır, onayla, sonucu gör; tam erişim
  uyarısı ve yerel onay.
- **Canlı** (`#[ignore]`, ayrı onay): EVREN modeli test sunucusunun aracını
  seçip çağırır.

Yeni güvenlik sınırları (§5, §6.2) testleriyle gelir.

## 11. Teslim

1. **G1:** Windows borulu AppContainer prototipi (§4).
2. **Plan 1.1a-1:** `kalfa-sandbox` etkileşimli süreç (iki platform).
3. **Plan 1.1a-2:** `kalfa-mcp`, `mcp_karari`, uygulama yöneticisi,
   arayüz, e2e.

**Yeni bağımlılıklar:** `rmcp` 3.5.1 (`client`, `transport-async-rw`;
dev'de `server`). `rust-version` 1.88 ≤ depo araç zinciri 1.98.1.
