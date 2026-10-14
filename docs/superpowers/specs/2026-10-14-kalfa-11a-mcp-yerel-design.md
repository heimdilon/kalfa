# Kalfa 1.1a — Yerel MCP sunucuları: Tasarım

**Tarih:** 2026-10-14.
**Durum:** Tasarım, r5. Kullanıcı ilk sürümü bölüm bölüm onayladı (14 Ekim).
§13'te üç incelemenin yanıtı var:

- Codex'in yerel incelemesi (`dd641a3`, altı bulgu),
- PR #19'daki satır içi Codex yorumları (15 bulgu),
- Codex'in r3 yeniden incelemesi (`5dd79a4`, beş bulgu),
- Codex'in r4 yeniden incelemesi (`e5080c2`, iki bulgu).

Belge yeniden incelemeye hazır. G1 ve uygulama, kullanıcının ayrı onayını
bekliyor.
**Taban:** `main`, `04f16fc` (`v0.1.0` ön sürümü yayımlandı).
**Üst belge:** [Kalfa tasarımı](2026-09-21-kalfa-design.md) §10, §12.2.

## 1. Amaç ve kabul ölçütü

Kullanıcı, bilgisayarındaki MCP sunucularını (stdio) Kalfa'ya ekler. Model
bu sunucuların araçlarını yerleşik araçlar gibi çağırır. Sunucular
varsayılan olarak kabuk aracının yalıtımında çalışır. Kullanıcı sunucu
bazında, açık uyarı ve yerel onayla *Tam sistem erişimi*ne geçebilir.

**Temel ilke:** Araç çağrısını onaylamak süreci onaylamak değildir. Süreç
başladığı andan kapandığı ana kadar, çağrılar arasında da çalışır. Bu
yüzden sürecin başlatılması (§5.1) ile her araç çağrısı (§5.2) ayrı
kapılardır.

Başarı (her madde §11'de bir testle karşılanır):

- Depodaki test sunucusu Linux'ta (bwrap) ve Windows'ta (AppContainer)
  yalıtımlı başlar, araçları listelenir ve çağrılır. Çalışma alanı dışını
  okuyamaz, izin listesi dışına ağla çıkamaz.
- Başlatma izni verilmeden ya da yalnız İnceleme kipinde oturum varken
  sunucu süreci hiç oluşmaz (§5.1).
- Koruma kümesi değişince (§4.3) sonraki çağrı yeni korumalarla başlatılmış
  süreçte çalışır. Etkin çağrısı olan süreç dahil hiçbir süreç, 10 dk'dan
  eski bir koruma denetimiyle çalışmayı sürdürmez.
- Sunucu kodu modelin yazabildiği bir kökteyse, o köke yazılmasından
  sonraki ilk başlatma yeniden izin ister (§5.1.1). Kökler yalnız proje
  dizini değildir; kabuğun özel ev, geçici ve önbellek dizinleri de
  dahildir.
- Koruma taraması doğrulanamazsa (hata, eksik yürüyüş, süre aşımı) t_K
  yenilenmez ve süreç durdurulur (§4.3).
- Ham stdio sınırları (§4.2) ayrıştırmadan önce uygulanır. Sınırı aşan
  sunucu kontrollü kapanır, uygulama yanıt vermeye devam eder.
- İptal ya da zaman aşımında, sunucu ne yanıt verirse versin süreç ağacı
  durdurulur. Ağaç durup son denetim bitmeden çalışma alanı kilidi
  bırakılmaz (§7.3).
- Onay ve kısayollar §5.3'teki tarife bağlıdır. Tarif değişince eski onay
  yeni tarifle çalışmaz.
- Araç tanımları denetçiden geçer. Katı kipte şüpheli ya da denetlenemeyen
  tanım modele gitmez (§6.2).
- İçinde bilinen bir sır geçen araç tanımı; denetçiye, önbelleğe,
  arayüze, JSONL kayda ve modele hiç ulaşmaz (§6.2).
- MCP kendisi AppContainer profili ya da `S-1-15-2-` ACE'si bırakmaz.
  Çalışma alanı hazırlığı mevcut kurala tabidir (§5.6).
- E2e: Ayarlar'dan test sunucusu eklenir, başlatma izni verilir, sohbette
  çağrılır ve onaylanır, sonuç görünür.

## 2. Kararlar ve kapsam

**Kullanıcı kararları (14 Ekim):**

- MCP desteği hem yerel hem uzak sunucuları kapsar. İş iki aşamaya
  bölünür: **1.1a** yerel stdio (bu belge), **1.1b** uzak Streamable HTTP
  ve OAuth (ayrı tasarım).
- Yerel sunucular **varsayılan yalıtımlı** çalışır; sunucu bazında *Tam
  sistem erişimi* seçilebilir. Gerekçe: Claude Code ve Codex yerel
  sunucuları yalıtımsız çalıştırıp onayla denetliyor. Kalfa'nın iki
  platformda kabuk yalıtımı olduğu için daha güvenli bir varsayılan mümkün.
- Yapılandırma **Ayarlar ekranı + JSON içe aktarma** ile yapılır. Çalışma
  alanından (`.mcp.json` gibi) sunucu gelmez.
- Araç çağrısının onayı **kabuk gibi** işler (§5.2).
- Yaklaşım **A**: `kalfa-sandbox`'a etkileşimli süreç eklenir. Yalıtımlı
  ve tam erişimli sunucular aynı arayüzden başlar.

**İnceleme sonrası değişen karar (§13, bulgu 4):** İlk sürümde "iptalde
süreç öldürülmez" yazıyordu. r4'te iptal ve zaman aşımı, sunucunun
yanıtından bağımsız olarak süreç ağacını **her durumda** durdurur; 2 sn
yalnız kapanma payıdır (§7.3).

- **Gerekçe:** MCP iptali işbirliğine dayanır. Sunucunun yanıtı yalnız
  RPC verisidir, durdurmanın kanıtı olamaz. Aksi hâlde "iptal edildi"
  görünen bir işlem dosya yazmayı sürdürebilir.
- **Bedel:** Her iptalde sunucunun bellekteki durumu kaybolur.

Bu değişiklik kullanıcının onayına sunulur.

**Kapsam içinde:** stdio taşıması; MCP `tools` (`tools/list` sayfalamasıyla,
`tools/call`, `notifications/tools/list_changed`, `notifications/cancelled`).

**Kapsam dışında:** uzak HTTP ve OAuth (1.1b); `resources` ve `prompts`;
sunucunun istediği `sampling`, `elicitation` ve `roots`; çalışma alanındaki
yapılandırma dosyaları; görsel ve ses içeriğin modele gönderilmesi.

**Bütçe:** MCP RPC'leri EVREN'e istek göndermez. Ancak sonuçların (§6.1)
ve araç tanımlarının (§6.2) denetimi EVREN'e gider ve oturum bütçesinden
(5b) "denetçi" amacıyla harcar. Sonuçlar, modele giden içerik olarak bağlam
yönetimine (5a) girer.

## 3. Bileşenler

| Birim | Görevi | Bağımlılığı |
|---|---|---|
| `kalfa-sandbox` etkileşimli süreç (§4) | stdin/stdout borulu, süre sınırsız süreç: Linux'ta bwrap ve seccomp, Windows'ta AppContainer ve Job, yalıtımsız arka uç | Mevcut profiller, vekil ve köprü |
| `kalfa-mcp` (yeni crate) | Sunucu tanımı, sınırlı çerçeveleyici (§4.2), `rmcp` istemcisi, el sıkışma, araç listesi, çağrı, iptal, yeniden başlatma. Her MCP aracını `Arac` trait'ine sarar | `rmcp` 3.5.1 (`client`), `kalfa-core`, `kalfa-sandbox` |
| `kalfa-uygulama` MCP yöneticisi | Tanımları saklar; gizli değerleri kimlik deposuna yazar; başlatma izinlerini (§5.1) tutar; süreçleri çalışma alanı bazında yönetir; oturumlara her turda araç anlık görüntüsü verir (§7.2) | `kalfa-mcp`, `ayarlar.rs`, `anahtar.rs` |
| Arayüz (§9) | Ayarlar'da "MCP sunucuları", başlatma izni kartı, rozetler, onay ve araç kartı | Mevcut Ayarlar ve kart bileşenleri |

Ajan döngüsü değişmez: MCP araçları oturumun araç listesine `Arac` olarak
girer (üst belge §10).

**Araç adı:** `mcp__<sunucu>__<araç>`. Sunucu adı `[a-z0-9-]{1,32}`
olmalıdır. Bir araç şu durumlarda kapatılır (§6.2): adında `[A-Za-z0-9_-]`
dışında karakter varsa ya da toplam ad 64 karakteri aşıyorsa.

**Sunucu tanımı** uygulama verisindeki ayarlar dosyasında durur:

```text
ad, etkin, komut, argümanlar, ortam (ad → düz değer | gizli kimliği),
erişim (Yalitimli | TamErisim), ağ izin listesi (yalnız Yalitimli),
başlatma zaman aşımı (vars. 30 sn), çağrı zaman aşımı (vars. 120 sn)
```

- **Komut:** İki biçimden biri olabilir.
  - Çıplak bir program adı (`npx`, `uvx`, `node` gibi). Ad, arka uçta
    sunucunun ortamındaki `PATH`'te aranır.
  - Yerel dosya seçiciyle seçilmiş bir yürütülebilir dosya. Bu durumda yolu
    arka uç tutar; arayüz yalnız bir kimlik görür (§9).
- **Sunucunun özel kökü:** Her (sunucu, çalışma alanı) çiftinin kendi
  `ev`, `gecici` ve `onbellek` dizinleri vardır. Bunlar sunucunun **özel
  köküdür** (§5.1.1, §5.6).
  - Yalıtımlıda kök `<yalıtım dizini>/<alan>/mcp/<sunucu>/` altındadır.
  - Tam erişimde kök uygulama verisinde, aynı adla durur.
  - Sunucunun ortamındaki `HOME`, `TMPDIR`/`TEMP`, `NPM_CONFIG_CACHE`,
    `PIP_CACHE_DIR`, `UV_CACHE_DIR` ve `XDG_CACHE_HOME` bu köke yönlenir.
    Böylece `npx` ya da `uvx`'in indirdiği paket kodu bu kökte durur.
  - Kabuğun özel ev, geçici ve önbellek dizinleri sunucuya bağlanmaz.
    Sunucunun özel kökü de kabuğa ve öbür sunuculara bağlanmaz.
- **Çalışma dizini:** Sunucunun özel kökündeki `ev` dizinidir. Böylece
  `npx` ya da `uvx`, çalışma alanındaki `node_modules` ya da `.venv`
  içinden kendiliğinden kod yüklemez. Çalışma alanının yolu sunucuya
  yalnız açıkça verilir: argümanda ya da ortam değerinde `${CALISMA_ALANI}`
  yer tutucusuyla. Kalfa yer tutucuyu başlatma anında çözer.
- **Ortam:** §5.5'teki birleştirme kuralı geçerlidir.

## 4. Süreç ve taşıma

### 4.1 Etkileşimli süreç (`kalfa-sandbox`)

Mevcut `Yalitici::baslat` stdin'i kapalı (`Stdio::null`) başlatır ve
çıktıyı toplayıp süreç bitince verir (`surec.rs`). MCP için ayrı bir giriş
eklenir:

- `baslat_etkilesimli(istek) -> EtkilesimliSurec` stdin için `AsyncWrite`,
  stdout için `AsyncRead`, stderr için ayrı bir okuyucu ve `kapat()` verir.
- Toplam süre ve toplam çıktı sınırı yoktur. Kabuğun süreç sınırları
  (Windows'ta Job Object'in etkin süreç sınırı, Linux'ta `prlimit`) aynen
  geçerlidir.
- `kapat()` önce stdin'i kapatır, 2 sn bekler, sonra süreç ağacını öldürür.
  Windows'ta `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, Linux'ta bwrap
  `--die-with-parent` ve mevcut `agac` kapanışı kullanılır. `kapat()`
  döndüğünde ağaçta yaşayan süreç kalmaz.
- **Çalışma kipi açıkça seçilir.** Mevcut `kip_uyusmasi(false, true)`
  yalıtım hazırken yalıtımsız başlatmayı reddeder
  (`kalfa-sandbox/src/lib.rs`), çünkü kabukta yalıtımsız çalışma yalnız
  bir geri düşme yoludur. MCP'de *Tam sistem erişimi* ise kullanıcının
  açık seçimidir. Bu yüzden istek seçilen kipi taşır
  (`CalistirmaKipi::{Yalitimli, TamErisim}`):

  | Seçilen | Yalıtım hazır | Yalıtım yok |
  |---|---|---|
  | Yalıtımlı | Yalıtımlı başlar | Başlamaz |
  | Tam erişim | Yalıtımsız arka uçla başlar | Yalıtımsız arka uçla başlar |

  Yalıtımlı tarif hiçbir zaman yalıtımsız arka uca düşmez. Tam erişim de
  hiçbir zaman sessizce yalıtıma geçmez. Seçilen kip mühürdedir (§5.3).
- **Ağ (yalnız yalıtımlıda):** Ağ listesi doluysa mevcut vekil ve köprü
  yolu kullanılır; boşsa ağ yoktur. Yalıtımsız arka uç ağı kapatamaz
  (`CalistirmaIstegi::ag` belgesi). Bu yüzden tam erişimli sunucuda ağ
  listesi yoktur: arayüz onu gizler, ağı "sınırsız" diye gösterir ve
  liste tanım özetine girmez.

**Ön doğrulama G1 (plan 1.1a-1'den önce):** Küçük bir prototiple
(`prototip/` altında, 2c gibi) Windows'ta şunlar ölçülür:

1. AppContainer içinde `CreateProcessW` ile başlatılan sürecin stdin ve
   stdout borularının kalıtımı.
2. Sürecin uzun süre yaşaması ve Job Object ile kapanışı.
3. **Sunucuya özel yetenek SID'i (§5.6):** Aynı paket SID'iyle çalışan,
   yetenek SID'i olmayan bir süreç (kabuk), yalnız o yetenek SID'ine izin
   verilmiş bir dizine yazamaz. Aynı dizine o yetenek SID'iyle başlatılan
   süreç ise yazabilir.

Engel çıkarsa bu belge güncellenir.

### 4.2 Ham stdio sınırları

`rmcp`'nin `AsyncRwTransport`'u gelen satırı `read_until` ile sınırsız
bir `Vec`'e okur. Kodlayıcısının varsayılan sınırı `usize::MAX`'tır
([kaynak](https://docs.rs/rmcp/latest/src/rmcp/transport/async_rw.rs.html)).
Bu yüzden Kalfa stdout'u `rmcp`'ye doğrudan vermez. Arada kendi **sınırlı
çerçeveleyicisi** durur; `rmcp`'ye ancak sınırdan geçmiş tam satırlar
ulaşır. Çerçeveleyicinin `rmcp`'ye hangi arayüzle bağlanacağı (`Transport`
trait'i ya da `Sink` ve `Stream` sarmalayıcısı) plan 1.1a-2'nin ilk
adımında derlenerek doğrulanır.

| Sınır | Değer | Aşılınca |
|---|---:|---|
| Tek JSON-RPC çerçevesi (satır sonuna kadar, ayrıştırmadan önce) | 4 MiB | Sunucu kapatılır (§4.1); *Hata*: "çerçeve sınırı" |
| İşlenmeyi bekleyen gelen ileti kuyruğu | 64 ileti | Sunucu kapatılır; *Hata*: "ileti seli" |
| stderr satırı | 4 KiB (maskeleme kesmeden önce, §5.5) | Satır kesilir |
| stderr halkası | Son 50 satır (≤ 200 KiB) | En eski satır düşer |
| `tools/list` sayfası | En çok 10 sayfa | *Hata*: "araç listesi çok uzun" |
| Araç sayısı | Sunucu başına 128 | *Hata* (kısmi liste kullanılmaz) |
| Toplam tanım baytı (ad, açıklama, `inputSchema` ve `outputSchema` JSON'u) | Sunucu başına 256 KiB | *Hata* |
| Açıklama | 1 KiB | Kesilir (§6.2) |

- **Bildirimler:**
  - Liste değişikliği haberi gelirse (§7.1'deki sürüme göre) tek bir
    "yenileme bekliyor" bayrağı kalkar. Yenileme en sık 5 sn'de bir
    yapılır.
  - Öbür bildirimler (`logging`, `progress` gibi) okunup atılır.
- **Sunucudan gelen istekler:** `ping` yanıtlanır. Öbürleri (`sampling`,
  `elicitation`, `roots/list`) "method not found" hatası alır.
- **Sonuç:** Çerçeve sınırı sonucu da sınırlar. Ardından mevcut araç
  çıktısı kısaltması gelir.

### 4.3 Koruma kümesi ve uzun yaşayan süreç

Kabuk her çalıştırmadan önce gizli ve korumalı yolları yeniden tarar
(`kalfa-tools/src/kabuk/mod.rs`, `alan_bilgisi`). Linux bu yolları bwrap
başlatılırken bağlamalarla örter (`linux/bwrap.rs`). Uzun yaşayan bir
süreçte bu bağlamalar başlatma anında donar.

**Koruma özeti K**, `alan_bilgisi`'nin ürettiği şu kümelerin özetidir:

- gizli dosyalar ve gizli dizinler,
- korumalı yollar,
- `.git` dizinleri,
- yalıtıcının okunur dizinleri,
- ağ listesi.

Her süreç kendi K'sıyla başlar ve onu saklar.

- **Her çağrıdan önce** `alan_bilgisi` yeniden üretilir. Windows'ta ayrıca
  `alani_hazirla` çalışır; bu, kabuktaki gibi yeni gizli ve korumalı
  yolların ACL'ini keser. Güncel K, çalışan sürecin K'sından farklıysa
  süreç kapatılır (§4.1) ve güncel korumalarla yeniden başlatılır. Çağrı
  yeni süreçte yapılır. Onay bu denetimden sonra alınır; denetim onaydan
  sonra tekrarlanır ve K o arada değiştiyse yeniden onay istenir (§7.3).
- **Kalfa'nın kendi değişikliklerinden sonra:** Çalışma alanında bir yazma
  aracı ya da kabuk komutu bitince aynı karşılaştırma hemen yapılır.
  Değişen K'lı süreç kapatılır. Yeniden başlatma bir sonraki çağrıda olur.
- **Ayar değişince:** Gizli kalıplar, gizli sayılmayanlar, okunur dizinler
  ya da ağ listesi değişirse (`ayarlar.rs` `degistir`) o anda bütün çalışan
  yalıtımlı sunucular kapatılır ve bekleyen çağrıları hata alır. Bu bir
  politika (Π) değişikliği olduğu için başlatma izinleri ve kısayollar da
  düşer (§5.3).
- **Mutlak denetim yaşı:** Her süreç, K'sının son doğrulandığı anı (t_K)
  saklar. Bir zamanlayıcı, t_K'dan en geç 10 dk sonra koruma denetimi
  yapar. Zamanlayıcı sürecin durumuna bakmaz: keşif sürerken, onay
  beklenirken ve bir çağrı çalışırken de denetler. Bu denetim kilit almaz,
  yalnız tarar. Denetimin üç sonucu vardır (`KorumaSonucu`):
  - **Aynı:** Tarama doğrulandı ve K aynı. Yalnız bu durumda t_K yenilenir.
  - **Değişti:** Tarama doğrulandı ve K farklı. Süreç ağacı hemen
    durdurulur (§4.1). Etkin çağrı "korumalar değişti, sunucu durduruldu"
    hatasıyla biter. Ardından son denetim çalışır ve kilit bırakılır
    (§7.3).
  - **Doğrulanamadı:** t_K **yenilenmez**, süreç "Değişti" ile aynı yoldan
    durdurulur. Etkin çağrı "korumalar doğrulanamadı" hatasıyla biter.
    Başarılı bir tarama gelene kadar sunucu yeniden başlatılmaz. Ayarlar'da
    hata ve nedeni (yol ya da süre) görünür.
  - Çağrı zaman aşımı ne kadar uzun tutulursa tutulsun bu sınır uzamaz.
- **Doğrulanamadı** sayılan durumlar:
  - Tarayıcı ya da `alan_bilgisi` bir hata döndürür.
  - Windows'ta `alani_hazirla` başarısız olur.
  - Koruma yürüyüşü bir girdiyi okuyamaz. Mevcut `tara_ayrintili`
    okunamayan girdiyi atlayıp devam eder (`tarama.rs`); MCP için bu
    durumda ayrı bir **`koruma_tam = false`** işareti konur.
  - Tarama **30 sn** koruma tarama süresini aşar ve iptal edilir.

  `goruntu.eksik` bu ölçüt değildir. O alan, koruma taraması sürerken
  yalnız değişiklik görüntüsünün girdi ya da süre sınırı dolduğunda da
  `true` olabilir (`GORUNTU_AZAMI_GIRDI`, `GORUNTU_AZAMI_SURE`).
  Görüntü sınırının dolması tek başına "Doğrulanamadı" sayılmaz.
- **Aynı kural her koruma denetiminde geçerlidir:** çağrı öncesi, onaydan
  sonraki ikinci karşılaştırma, çağrı sonrası, Kalfa yazması sonrası ve
  başlatma. Doğrulanamayan denetimden sonra çağrı yapılmaz, süreç
  başlatılmaz.
- **Kullanılabilirlik bedeli:** Kalfa'nın sürekli okuyamadığı bir dizin
  (örneğin başka kullanıcıya ait bir dizin), o çalışma alanında yalıtımlı
  MCP'yi kullanılamaz kılar. Ayarlar'daki hata o yolu gösterir.
- **Boşta kapanma:** Son çağrısından bu yana 10 dk çağrı almayan süreç
  kapatılır (§4.1). Sonraki çağrıda izin geçerliyse onaysız yeniden başlar.
  W'de kodlu sunucuda ise yazma koşulu da aranır (§5.1.1).
- **`.git` ve yapılandırma denetimi:** Kabuğun `komut_sonrasi_denetim`
  karşılığı şu anlarda çalışır: her çağrıdan sonra (iptal ve zaman aşımı
  dahil), her çağrıdan önce (son denetimden beri oluşanlar için) ve süreç
  kapanırken. Bulunan yeni `.git` ve yapılandırma dosyaları kabuktaki
  kuralla karantinaya alınır.

**Kalan risk (kullanıcı kararına sunulur):** Kalfa dışındaki bir program
(örneğin kullanıcının editörü) çalışma alanına gizli kalıba uyan bir dosya
yazabilir. O zaman çalışan yalıtımlı sunucu bu dosyayı bir süre okuyabilir.
Risk yalnız yalıtımlı sunucular için geçerlidir; tam erişimli sunucu zaten
her dosyaya erişebilir.

- **Süre (üst sınır):** Dosyanın yazıldığı andan sürecin durmasına kadar en
  çok şu kadar geçer: **10 dk + en çok 30 sn koruma taraması + 2 sn
  kapanma payı + ağacın öldürülmesi**. Tarama 30 sn'de bitmezse
  "Doğrulanamadı" sayılır ve süreç yine durdurulur; süre bu yüzden
  uzamaz.
  - 10 dk, mutlak denetim yaşıdır. Çalışan çağrı, keşif ve onay bekleme bu
    süreyi uzatmaz.
  - Çoğu durumda süre daha kısadır. Kalfa'nın bir sonraki yazması, sonraki
    çağrı ya da boşta kapanma daha önce denetler.
  - Kabukta aynı pencere, komutun çalıştığı süredir (aşama 2 tasarımı
    §7.2).
- **Veri çıkış yolları:** Bu sürede okunan içerik iki yoldan dışarı
  çıkabilir:
  1. **Doğrudan ağ:** Ağ listesi boşsa bu yol yoktur. Doluysa sunucu yalnız
     izinli hostlara çıkabilir.
  2. **Stdout üzerinden EVREN'e:** Sunucu içeriği bir araç sonucuna
     (`tools/call`) ya da bir araç tanımına (açıklama, şema) koyabilir.
     Kalfa bunları denetçiye ve modele, yani EVREN'e gönderir:
     `denetci.rs` denetlenen metni sağlayıcıya yollar, `model_turu.rs`
     tanımları isteğe koyar. Denetçi metni yalnız sınıflandırır; gizli
     dosya içeriğini tanıyıp yerelde engelleyen bir kapı değildir. Bu yol,
     ağ listesi boş sunucuda da açıktır.
- **Geri alınamazlık:** Süreci sonradan durdurmak, okunmuş ya da EVREN'e
  gönderilmiş veriyi geri almaz.
- **Platform farkı:** Windows'ta ACL kesildikten sonra dosyayı yeniden
  açma girişimleri reddedilir. Linux'ta ise bağlama, süreç yeniden
  başlayana kadar dosyayı görünür bırakır.
- **Neden dosya izleyicisi yok:** Dosya izleyicisi atomik bir sınır
  kurmadığı için eklenmez. Daha kısa bir üst sınır gerekiyorsa denetim yaşı
  düşürülür. Bunun bedeli, büyük depolarda daha sık tarama yapılmasıdır.

## 5. Güvenlik

### 5.1 Başlatma izni

Sürecin başlaması, kullanıcının yazdığı kodu çalışma alanında çalıştırmak
demektir. `initialize` ve `tools/list` sırasında da yazma ve ağ erişimi
olabilir. Bu yüzden süreç ancak şu koşullar birlikte sağlanınca oluşur:

1. Sunucu etkin olmalı.
2. O çalışma alanında **İnceleme dışı kipte en az bir oturum** açık olmalı.
   Yalnız İnceleme oturumları varsa süreç başlatılmaz; çalışıyorsa kapatılır.
3. **Başlatma izni** bulunmalı. İzin şunlardan biridir:
   - *Yalıtımlı:* (tanım özeti T, politika özeti Π, çalışma alanı) için
     verilmiş kalıcı izin. Ayarlar'dan geri alınabilir; T ya da Π
     değişince düşer.
   - *Tam erişimli:* T için verilen **yerel pencere** onayı. Uygulama
     açık kaldıkça geçerlidir; Kalfa kapatılıp açılınca yeniden sorulur.
4. Sunucu W'de kodluysa (modelin yazabildiği bir kökte) §5.1.1'in koşulu da sağlanmalı.

İzin yoksa oturumda, model dışında bir **başlatma kartı** görünür:
"`<sunucu>` bu çalışma alanında başlatılsın mı?". Karttaki bilgiler: erişim
türü, komut, ağ listesi ve "başladığı andan kapanana kadar, araç
çağrılmasa da çalışır" uyarısı. Model bu kartı açamaz, yanıtlayamaz.
İzin verilene kadar sunucunun araçları modele sunulmaz.

Çökme sonrası yeniden başlatma, boşta kapanmadan dönüş ve koruma
yenilemesi (§4.3) mevcut izinle onaysız yapılır. İki istisna vardır, bu
durumlarda yeniden izin istenir:

- Π değişmişse,
- §5.1.1'deki yazma koşulu artık sağlanmıyorsa.

#### 5.1.1 Sunucu kodu ve başlatma güveni

T, komutun çözülmüş yürütülebilir dosyasının özetini içerir. Ama `node`,
`npx`, `uvx` ya da `python` gibi komutlarda bu dosya sunucunun kendisi
değildir; onu yükleyen yorumlayıcı ya da başlatıcıdır. `node ./sunucu.js`
tanımında `sunucu.js` değişse bile T aynı kalır. **Yalnız yorumlayıcının
özeti, çalışan kodu sabitlemez.**

**Modelin yazabildiği kökler (W).** "Proje içi / dışı" ayrımı yetmez.
Yalıtımlı kabuk proje kökünün yanında şunlara da yazar:

- Linux'ta kabuğun özel ev ve önbellek dizinleri ile `/tmp`'ye bağlanan
  geçici dizin (`linux/bwrap.rs`). `HOME` ve `NPM_CONFIG_CACHE` de bunlara
  yönelir (`linux/mod.rs`).
- Windows'ta paket SID'ine miraslı değiştirme izni verilmiş özel ev,
  geçici ve önbellek dizinleri (`windows/yalitici.rs`).

Bir çalışma alanının **W** kümesi, modelin ya da öbür araçların yerel
onaysız yazabildiği bütün köklerdir:

1. proje kökü,
2. kabuğun o alana özel `ev`, `gecici` ve `onbellek` dizinleri,
3. aynı alandaki **öbür** MCP sunucularının yazabildiği her yer. Bu yerler
   1. ve 2. maddedeki köklerdir; öbür sunucuların kendi özel kökleri
   onlara aittir.

W dışında kalan ve modelin yerel onaysız yazamadığı yerler şunlardır:

- sunucunun kendi özel kökü (§3, §5.6),
- kullanıcının sistem dizinleri.

Yalıtımsız kabuk ve tam erişimli araçlar her çağrıda yerel onay ister.

**Yüklenen kodun yeri.** Sunucunun yüklediği kod üç yerden gelir:

- **Sunucunun özel kökü:** `npx` ve `uvx` paketleri, `HOME`/önbellek
  yönlendirmesiyle (§3) burada açılır. Kabuk ve öbür sunucular bu köke
  yazamaz: Linux'ta bağlanmaz, Windows'ta yalnız sunucunun yetenek SID'ine
  izin verilir (§5.6). Sunucunun kendi köküne kendisinin yazması, sunucunun
  kendi davranışıdır (aşağıdaki kalan sınır).
- **Açıkça verilen yollar:** Komut, argümanlar ya da ortam değerleri.
  Kalfa bunları kayıtta ve her başlatmada çözer, W'nin içinde mi diye
  bakar.
- **`PATH` araması:** Çıplak program adı `PATH`'te çözülür. Çözülen dosya
  W'nin içindeyse "açık yol" gibi sayılır. `PATH`'in W'deki dizinleri bu
  denetime girer.

Bu üç yere göre sunucu iki türden biridir:

- **W'de kodlu sunucu:** Komutun çözüldüğü dosya, bir argüman ya da bir
  ortam değeri W'nin içini gösteriyorsa sunucu bu türdendir. Buna
  `${CALISMA_ALANI}` ve kabuğun özel dizinleri de dahildir.
- **W dışında kodlu sunucu:** T şunların içerik özetini kapsar: komut
  dosyası, dosyaya çözülen her argüman ve dosyaya çözülen her ortam yolu.
  Paket kodu sunucunun özel kökündedir ve model oraya yazamaz. Otomatik
  yeniden başlatma serbesttir.

**W'de kodlu sunucuda yazma kuşağı.** Her çalışma alanının bir **yazma
kuşağı** sayacı vardır. Sayacı şunların her biri artırır:

- Kalfa'nın W'ye yazan her aracı: dosya araçları ve her kabuk komutu.
  Kabuk özel dizinlere de yazabildiği için her komut sayılır.
- O alandaki her MCP çağrısı. Sunucu çağrı sırasında proje köküne
  yazmış olabilir.

  Başlatma izni, son onaylı başlatmanın kuşağını saklar. Kuşak arttıysa
  sonraki **her** başlatma yeniden izin ister: çökme, boşta kapanma ve
  koruma yenilemesi sonrası başlatmalar da dahil. Yalıtımlı sunucuda izin
  başlatma kartıyla, tam erişimde yerel pencereyle sorulur.
- **Kalan sınır:**
  - Sunucu programı çalışırken çalışma alanındaki dosyaları kendi
    kararıyla kod olarak yüklerse bu sunucunun kendi davranışıdır; Kalfa
    bunu sabitleyemez.
  - `npx -y paket` gibi uzak paketlerde paket sürümü T'de yer almaz.
    Arayüz sürüm sabitlemeyi önerir (`paket@1.2.3`). Sabitlenmemiş paket
    güncellenince yeni kod aynı izinle çalışabilir.
  - Yalıtımlı sunucu kendi özel kökündeki paket kodunu kendisi
    değiştirebilir. Bu, model için ayrı bir yol açmaz: Model sunucuya
    ancak sunucunun araçları üzerinden ulaşır. Bir sunucunun kendi
    önbelleğine keyfî yazan bir aracı varsa bu, sunucunun kendi
    davranışıdır.

### 5.2 Çağrı onayı

`kalfa-core::izin`'e `mcp_karari(kip, erisim, ilk_cagri, agli)` eklenir:

| Kip | Yalıtımlı sunucu | Tam erişimli sunucu |
|---|---|---|
| İnceleme | Araçlar modele sunulmaz | Araçlar modele sunulmaz |
| Sıkı, Proje | `Onayli` | `YerelOnayli`, her çağrıda |
| Serbest | `Serbest`; sunucunun bu çalışma alanındaki ilk çağrısıysa ya da ağ listesi doluysa `Onayli` | `YerelOnayli`, her çağrıda (ağ sınırsız, §4.1) |

- "Bu oturumda izin ver" kısayolu yalnız yalıtımlı sunucularda, araç
  bazında çalışır. Tam erişimli sunucuda kısayol yoktur; yalıtımsız kabuk
  kuralının aynısı.
- Modelin sunucu ekleyen, değiştiren ya da başlatan bir aracı yoktur.
- Bir sunucuyu *Tam sistem erişimi*ne geçirmek yerel pencerede onaylanır.

### 5.3 Tarif ve mühür

Üç özet tanımlanır. Gizli değerler özetlere yalnız SHA-256 olarak girer;
değerin kendisi hiçbir özete, kayda ya da loga yazılmaz.

- **Tanım özeti T** = ad, komut (çözülmüş mutlak yol ve dosyanın
  SHA-256'sı), argümanlar, **bütün** ortam (birleştirilmiş son ortamın
  ad-değer çiftleri; gizli değerler özetleriyle), çalışma kipi, ağ listesi
  (yalnız yalıtımlıda), çalışma alanı kimliği.
- **Politika özeti Π** = yalıtım profilinin içerikten bağımsız kısmıdır.
  Kullanıcı ayarı değişmedikçe Π de değişmez. Şunları içerir:
  - arka uç ve sürüm,
  - gizli kalıplar ve gizli sayılmayanlar,
  - okunur dizinler.
- **Çalışma tarifi R** = T, yalıtım profilinin özeti P ve **kuşak N**.
  P, `profil_ozeti`'dir; Π'yi ve içerik kümesi K'yı içerir. N, her süreç
  başlatmada artan bir sayaçtır. R süreç başlarken sabitlenir ve süreçte
  saklanır.
- **Araç tanım özeti D** = aracın modele giden adı, açıklaması (kesilmiş
  hâliyle), giriş şeması ve varsa çıkış şeması (`outputSchema`).

Bu özetler şöyle bağlanır:

- **Çağrı onayının mührü** R'ye (T, P ve N), D'ye ve argümanlara bağlanır.
  Çağrı yalnız R'si mühürdekiyle aynı olan **çalışan sürece** ve D'si
  aynı kalan araca gönderilir. Karşılaştırma ayar dosyasıyla değil, çalışan
  sürecin sakladığı R ile yapılır. Şu durumların her biri T'yi, P'yi, N'yi
  ya da D'yi değiştirir ve bekleyen onayı geçersiz kılar: düzenleme,
  kapatma, yeniden başlatma, koruma yenilemesi, aynı adlı aracın tanım
  değişikliği.
- **Kısayol** T, D ve Π'ye bağlanır; biri değişince düşer.
  - Π'deki her değişiklik kısayolu düşürür, daralma da genişleme de. Böylece
    okunur dizin eklemek ya da bir gizli kalıbı kaldırmak gibi
    genişlemeler eski kısayolla otomatik onaylanmaz.
  - Yalnız N ya da yalnız K (içerik) değişirse kısayol korunur. İçerik
    değişimi aynı politikanın sonucudur ve yeni süreç güncel korumalarla
    başlar.
- **Başlatma izni** T, Π ve çalışma alanına bağlanır. W'de kodlu
  sunucuda ayrıca yazma kuşağına bağlanır (§5.1.1).

### 5.4 Çalışma alanı kilidi

Her MCP çağrısı, kabuk gibi bütün çalışma alanının yazma kilidini alır
(`tum_calisma_alani`). Kilit çalışma alanı başına paylaşılır
(`kalfa-core::kilit`), bu yüzden aynı çalışma alanındaki MCP çağrıları
öbür araçlarla ve birbirleriyle çakışmaz. Kilit, çağrı bitene ya da süreç
ağacı durdurulana kadar (§7.3) tutulur.

### 5.5 Ortam ve sırlar

**Ortamın birleştirilmesi.** `ortam::uret` yalnız kalıtılan ortam içindir.
İzin listesi dışındaki adları ve `KEY`, `TOKEN`, `SECRET`, `PASSWORD`,
`CREDENTIAL`, `AUTH` içeren adları düşürür; sabit değerleri de en sona
yazar. Kullanıcının bir sunucu için açıkça verdiği değişkenler ayrı bir
adımla eklenir:

1. Kalıtılan kısım `ortam::uret(sistem ortamı)` ile üretilir.
2. Yapılandırılan değişkenler eklenir. Kayıt sırasında şu kurallar
   doğrulanır:
   - Ad `[A-Za-z_][A-Za-z0-9_]{0,63}` olmalı.
   - Aynı ad iki kez yazılamaz (Windows'ta büyük-küçük harf duyarsız).
   - Ayrılmış adlar reddedilir: `EVREN_API_KEY`, `EVREN_BASE_URL`, `KALFA_`
     önekli her ad ve `ortam::SABIT`'teki adlar.
   - Kalıtılan izinli bir değişkeni ezmek serbesttir (`NODE_OPTIONS`
     gibi). Ezilen değer T'ye girer.
   - `TOKEN` ya da `KEY` içeren adlar burada serbesttir; kullanıcı onları
     bilerek veriyordur. EVREN anahtarı ve Kalfa'nın iç değişkenleri ise
     ayrılmış adlar kuralı yüzünden hiçbir yoldan geçmez.

**Gizli değerler kasası.** `anahtar.rs`'teki `SistemDeposu` tek bir yuvaya
(`Kalfa`/`evren-api-anahtari`) bağlıdır. MCP için ayrı, anahtarlı bir kasa
eklenir:

- `GizliKasa::{oku, yaz, sil}(kimlik)`. Kimlik opak ve kalıcıdır
  (`mcp-<uuid v4>`; `uuid` zaten workspace bağımlılığı). Girişin adı
  kimlik deposunda `Kalfa`/`mcp-…` olur.
- Değişkeni ya da sunucuyu yeniden adlandırmak kimliği değiştirmez.
  Değişkeni ya da sunucuyu silmek girişi siler.
- Kimlik deposu kullanılamıyorsa gizli değerli sunucu başlamaz ve *Hata*
  olur: "kimlik deposu kullanılamıyor". Değer başka bir yere (dosyaya)
  düşülmez.
- Gizli değer en az 8 karakter olmalıdır; kayıt sırasında doğrulanır. Bu,
  aşağıdaki maskelemenin her sırrı kapsamasını sağlar.

**Sızmayı önleme:**

- Gizli değerler ayarlar dosyasına, JSONL kayda, loglara ve gönderim
  günlüğüne yazılmaz.
- JSON içe aktarmada bütün `env` değerleri varsayılan olarak gizli sayılır.
  Kullanıcı istediğini düz değere çevirebilir.
- **Maskeleme:** Sunucunun gizli değerleri; stderr'de, hata metinlerinde ve
  araç sonucunda, alt dize olarak geçtikleri her yerde `[gizli]` ile
  değiştirilir.
  - Değiştirme, veri bir tampona, durum nesnesine, arayüz olayına ya da
    modele girmeden önce yapılır.
  - stderr satırı kesilmeden önce maskelenir. Satır en çok 4 KiB + en
    uzun sır uzunluğu kadar tutulur; böylece kesme yeri bir sırrı ikiye
    bölemez.
- Sunucunun tanımı ya da gizli değerleri değişince stderr halkası
  temizlenir.
- **Araç tanımları da** sır denetiminden geçer (§6.2, 0. adım). Sunucu bir
  sırrı tanımda geri gönderirse araç kapatılır ve ham metin hiçbir yere
  yazılmaz.
- Sunucunun kendini tanıttığı metinler (`serverInfo`'daki ad ve sürüm,
  `instructions`) modele gönderilmez. Arayüzde maskelenmiş olarak
  gösterilir.

### 5.6 Windows çalışma alanı hazırlığı

- AppContainer, çalışma alanının hazırlanmasını ister (aşama 2 §9.4).
  Alan hazır değilse yalıtımlı sunucu başlamaz. Hazırlık mevcut akıştaki
  gibi yerel pencerede onaylanır.
- MCP süreçleri kabukla aynı çalışma alanı profilini ve paket SID'ini
  kullanır. MCP hazırlığı ya da ACE'leri hiçbir zaman kaldırmaz.
- **Sunucunun özel kökü** paket SID'ine açılmaz. Çünkü paket SID'ini
  kabuk ve öbür sunucular da taşır.
  - Her (sunucu, çalışma alanı) için bir **yetenek SID'i** türetilir.
    Özel kökte miraslı değiştirme izni yalnız bu SID'e verilir.
  - Sunucu süreci bu yetenekle başlatılır; kabuk ve öbür sunucular bu
    yeteneği taşımaz.
  - Bu ACE'ler de kayda girer. Sunucu ya da çalışma alanı kaldırılınca
    silinir.
  - Ölçümü G1'dedir (§4.1).
  - Linux'ta aynı ayrım bağlamayla kurulur: özel kök yalnız o sunucunun
    bwrap'ına bağlanır.
- Mevcut "hazırlanmış alanı kaldır" işlemi, o çalışma alanında çalışan MCP
  süreçlerini önce kapatır; kabuk ve öbür sunucular bundan etkilenmez.

## 6. Dış kaynaklı içerik

### 6.1 Sonuçlar

- MCP sonucu her kipte güvenlik denetçisinden (üst belge §9) ve mevcut
  araç çıktısı kısaltmasından geçer.
- Sonuç gönderim günlüğünde ve önizlemede görünür. Katı denetçi kipinde
  şüpheli sonuç tutulur.

**İçerik dönüşümü:**

| Gelen | Modele giden |
|---|---|
| `text` | Olduğu gibi |
| `content` dolu | Yalnız `content` kullanılır |
| `content` boş, `structuredContent` var | JSON metne çevrilir |
| `image`, `audio` | "[görsel içerik atlandı]" ya da "[ses içeriği atlandı]" |
| `resource_link` | Yalnız URI |
| Gömülü `resource`, metin | Metin, "kaynak: `<uri>`" başlığıyla |
| Gömülü `resource`, ikili | "[ikili kaynak atlandı: `<uri>`]" |

**Çıkış şeması:**

MCP belirtimi, istemcinin yapılandırılmış sonucu aracın `outputSchema`'sına
göre doğrulamasını ister.

- `outputSchema` keşifte saklanır ve derlenir. Derlenemeyen şemalı araç
  kapatılır (§6.2).
- Şemalı bir aracın `isError` olmayan sonucu iki durumda dış kaynaklı bir
  araç hatası (`Basarisiz`) olur: `structuredContent` şemaya uymuyorsa ya
  da hiç yoksa. Hata metni "yapılandırılmış sonuç şemaya uymuyor" olur ve
  denetlenir.
- Modele giden metin yukarıdaki tabloya göre yine `content`'ten üretilir.

**Hatalar:**

- `isError: true` ve sunucunun JSON-RPC hata iletisi dış kaynaklıdır.
  `AracHatasi::Basarisiz(AracCiktisi)` yolundan gider, böylece denetlenir.
- Kalfa'nın ürettiği hatalar (zaman aşımı, çerçeve sınırı, mühür) genel
  hata yolunu kullanır. Güvenilmeyen metin genel hata yoluna taşınmaz.

### 6.2 Araç tanımları

`tools/list` ile gelen ad, açıklama ve giriş şeması modele gider. Bu, tool
poisoning saldırılarının yüzeyidir. Bugün `AracKayitDefteri::tanimlar`
tanımları denetimsiz verir (`kalfa-core/src/arac.rs`). Tanımlar ayrıca
JSONL kayda yazılır (`model_turu.rs`, `KayitOgesi::AracTanimlari`). MCP
araçları için şu kapılar, bu sırayla uygulanır:

0. **Sır denetimi (her şeyden önce):**
   - Sunucunun gizli değerlerinden biri aracın tanımında herhangi bir
     yerde alt dize olarak geçiyorsa araç **kapatılır**. Tanımın her yeri
     şunlardır: ad, açıklama, `inputSchema` ile `outputSchema`'nın bütün
     metin değerleri ve özellik adları.
   - Maskeleme yapılmaz, araç kapatılır. Çünkü yapısal alanları (`enum`,
     `default`, özellik adları) değiştirmek protokolü bozar.
   - Kapatılan aracın ham tanımı şunların hiçbirine gitmez: denetçi, D
     hesabı, önbellek, arayüz, JSONL kayıt, model. Ayarlar'da adı bile
     görünmez; yalnız "araç #n: tanımında gizli değer var" yazar.

1. **Biçim:**
   - Açıklama 1 KiB'de kesilir.
   - Şu durumlarda araç kapatılır: görünmez ya da yön değiştiren Unicode
     içeriyorsa (onay ekranındaki mevcut denetim), adı geçersizse ya da
     şeması JSON Schema olarak çözülemiyorsa.
2. **Denetçi:**
   - Denetim metni araç adından, açıklamadan ve `inputSchema` içindeki
     bütün metin değerlerinden (her derinlikte) oluşur. Metin değerleri
     şunlardır: `description`, `title`, `enum`, `const`, `default`,
     `examples`, `pattern` ve özellik adları.
   - Metin mevcut sınıflandırıcıya araç başına gönderilir.
   - Denetçi kipine göre sonuç:

| Denetçi kipi | Şüpheli tanım | Denetlenemeyen tanım (hata, bütçe, zaman aşımı) |
|---|---|---|
| Kapalı | Denetim yok; sunulur | Sunulur |
| Uyar | Sunulur; Ayarlar'da ve sohbette uyarı | Sunulur; uyarı |
| Katı | **Sunulmaz** | **Sunulmaz** |

3. **Yeniden denetim ve önbellek:**
   - `list_changed` sonrasında yalnız D'si değişen araçlar yeniden
     denetlenir.
   - Sonuç, D ve denetçi modeli anahtarıyla uygulama verisinde
     önbelleklenir. Önbellek isabeti maliyetsizdir.
   - Denetim isteği, aracı ilk kez sunacak oturumun bütçesinden "denetçi"
     amacıyla harcar. İstek o oturumun gönderim günlüğünde görünür.
4. **Görünürlük:** Denetlenen tanım metni ve sonucu Ayarlar'daki sunucu
   ayrıntısında görünür. Kapatılan ya da sunulmayan araçlar nedeniyle
   listelenir ve modele gitmez.

## 7. Yaşam döngüsü

### 7.1 Başlatma ve kapatma

- **Çalışma alanına bağlılık:** Yalıtım çalışma alanına özeldir; bu yüzden
  her (sunucu, çalışma alanı) çifti için bir süreç açılır. Tam erişimli
  sunucular da aynı kurala uyar. İki türde de çalışma dizini, §3'teki
  sunucuya özel dizindir.
- **Başlatma:** §5.1'in koşulları sağlanınca süreç başlar. İstemci
  yetenekleri **boş** ilan edilir (`roots`, `sampling`, `elicitation` yok).
- **Protokol dönemleri:** MCP 2026-07-28, `initialize` el sıkışmasını
  kaldırıp yerine `server/discover`'ı koydu. Liste değişikliklerini de
  `subscriptions/listen` akışıyla iletiyor. Kalfa iki dönemi de destekler:
  - `rmcp` 3.5.1'in `ClientLifecycleMode::Auto` kipi kullanılır. Bu kip
    önce `server/discover` dener; eş eski dönemdeyse ya da 10 sn içinde
    yanıt vermezse `initialize` + `notifications/initialized` yoluna düşer.
    Bu 10 sn, başlatma zaman aşımının içindedir.
  - **Liste değişikliği pazarlıktaki sürüme göre izlenir.** Eski dönemde
    `notifications/tools/list_changed` dinlenir. 2026 döneminde
    `subscriptions/listen` kullanılır. `rmcp` bu aboneliği stdio'da
    istemci olarak sunmuyorsa yedek yol devreye girer: liste her tur
    başında, en sık 30 sn'de bir `tools/list` ile yeniden okunur ve D'ler
    karşılaştırılır. Hangi yolun mümkün olduğu plan 1.1a-2'nin ilk
    adımında derlenerek doğrulanır.
  - Ardından `tools/list` (sayfalamayla) gelir. Desteklenmeyen sürüm
    *Hata* olur.
  - Başlatma zaman aşımı sunucu tanımındadır.
  - Başlamayan sunucu oturumu bekletmez: araçları o tur gelmez, Ayarlar'da
    hata görünür, sohbette tek satırlık bir not çıkar.
- **Kapatma (§4.1 `kapat()`) şu durumlarda olur:**
  - çalışma alanında İnceleme dışı oturum kalmayınca,
  - 10 dk boşta kalınca (§4.3),
  - koruma ya da ayar değişince (§4.3),
  - sunucu düzenlenince ya da kapatılınca,
  - uygulama çıkarken.

### 7.2 Araç listesi ve oturumlar

Bugün her oturum, bağlamı kurulurken tek seferlik bir `AracKayitDefteri`
alır (`kalfa-uygulama/src/oturumlar.rs`). Defterin araç listesi ve
derlenmiş şemaları sabittir (`kalfa-core/src/arac.rs`). Bu yapıyla,
oturum açıldıktan sonra keşfini bitiren ya da listesini değiştiren bir
sunucunun araçları o oturuma hiç giremez.

Çözüm, **yenilenebilir bir kaynak ve tur başında atomik bir anlık
görüntüdür:**

- Uygulama yöneticisi her çalışma alanı için paylaşılan bir
  `McpAracKaynagi` tutar. Kaynak sürümlüdür; her keşif ya da liste
  değişikliği, denetimden geçmiş (§6.2) yeni bir sürüm yayımlar.
- Oturumun defteri iki parçadan oluşur: sabit araçlar (bugünkü gibi) ve
  bu kaynak. Her model turunun başında kaynaktan **tek bir anlık görüntü**
  alınır. Görüntü her araç için tanımı, derlenmiş argüman doğrulayıcısını
  ve çağrı sarmalayıcısını birlikte taşır; üçü aynı sürüme ve aynı D'ye
  aittir.
- O turun `tanimlar`, `dogrula` ve `bul` çağrıları yalnız bu görüntüyü
  kullanır. Tur ortasında modelin gördüğü tanım değişmez.
- Liste değişikliği, araç ekleme ve çıkarma dahil, bir sonraki turda
  geçerli olur. Tur ortasında D'si değişen aracın çağrısı mühür
  denetiminde (§5.3) reddedilir.

### 7.3 Çağrı, iptal ve zaman aşımı

Çağrı adımları:

1. Model aracı çağırır.
2. Argümanlar şemaya göre doğrulanır.
3. Koruma denetimi yapılır; gerekirse süreç yeniden başlatılır (§4.3).
   Böylece onay kartı güncel R'yi gösterir.
4. §5.2'ye göre onay alınır.
5. Çalışma alanı kilidi alınır (§5.4).
6. K yeniden karşılaştırılır ve mühür denetlenir (§5.3). Onaydan sonra K
   değiştiyse süreç yeniden başlatılır ve çağrı 4. adıma, yeni onaya döner.
7. `tools/call` gönderilir.
8. Sonuç §6.1'e göre dönüştürülür, kısaltılır ve denetçiden geçer.
9. `komut_sonrasi_denetim` karşılığı çalışır (§4.3).
10. Kilit bırakılır.

**İptal ya da zaman aşımı:**

1. Sunucuya `notifications/cancelled` gönderilir. Bu yalnız kibar bir
   uyarıdır; sunucunun doğru kapanmasına fırsat verir.
2. Süreç ağacı **her durumda** `kapat()` ile durdurulur: stdin kapatılır,
   2 sn kapanma payı tanınır, sonra ağaç öldürülür (§4.1).
   - Sunucunun bu arada gönderdiği sonuç ya da iptal hatası durdurmanın
     kanıtı sayılmaz. Bunlar sıradan RPC verisidir; sunucu yanıt verip arka
     plandaki bir görevi ya da çocuk süreci çalıştırmayı sürdürebilir.
   - Eski MCP döneminde iptal alan sunucunun yanıt göndermemesi önerilir.
     Sessiz kalan uyumlu bir sunucu da aynı yoldan durdurulur. Durdurma
     yanıta değil, `kapat()`'ın ağacın bittiğini doğrulamasına dayanır.
3. O kuşağın bütün bekleyen çağrıları "sunucu durduruldu" hatasıyla biter.
   Sunucunun gönderdiği geç sonuç atılır.
4. Ardından `komut_sonrasi_denetim` karşılığı çalışır (§4.3).
5. Kilit **ancak bundan sonra** bırakılır.

Böylece "iptal edildi" ya da "zaman aşımı" denildiğinde yan etki sürmez.
Bu, kabuktaki garantinin aynısıdır. Kilit (§5.4) aynı anda tek çağrıya
izin verdiği için başka bir oturumun aynı süreçte bekleyen bir çağrısı
olamaz. Durdurulan süreç, sonraki çağrıda izinle onaysız yeniden başlar
(§5.1.1'in yazma koşulu hariç). Bu yeniden başlatma çökme sayacına
sayılmaz. Bedeli, sunucunun bellekteki durumunun kaybolmasıdır.

### 7.4 Çökme

- Süreç kendiliğinden ölürse sunucu *Durdu* olur ve bekleyen çağrılar hata
  döner.
- Sonraki çağrıda en çok 3 kez, artan aralıklarla (1, 4, 16 sn) yeniden
  başlatılır. Sonra *Hata*da kalır; kullanıcı Ayarlar'dan yeniden
  başlatabilir.

### 7.5 Kayıt ve kurtarma

Çağrılar ve sonuçlar, diğer araçlar gibi JSONL kayda girer. Kurtarmada
yarım kalmış MCP çağrısı yeniden çalıştırılmaz; yarım kaldı diye
işaretlenir.

## 8. Ortak çalışma alanı yaşamı

Bir çalışma alanında kabuk komutları ve birden çok MCP sunucusu aynı anda
yaşayabilir:

- Bir sunucunun kapanması öbürlerini ve kabuğu etkilemez.
- Ortak izinler (Windows hazırlığı) erken kaldırılmaz (§5.6).
- Koruma ya da ayar değişikliği (§4.3) yalnız MCP süreçlerini yeniden
  başlatır. Kabuk zaten her çalıştırmada yeniden tarar.

## 9. Arayüz

Ayarlar'da **MCP sunucuları** bölümü bulunur. Bu bölüm WASM eklentilerinden
ayrıdır (üst belge §10.1).

- **Liste:**
  - ad,
  - rozet (*Yalıtılmış süreç* ya da *Tam sistem erişimi*),
  - durum (Çalışıyor, Başlıyor, Durdu, Boşta kapandı, Hata),
  - araç sayısı,
  - aç/kapat,
  - yeniden başlat,
  - çalışma alanı başına başlatma izinleri ve geri alma.
- **Rozet açıklaması:** Rozet, erişimin yalnız çağrı sırasında değil,
  başladığı andan kapanana kadar sürdüğünü söyler.
  - Yalıtımlı: "çalışma alanına yazabilir, izinli ağa çıkabilir".
  - Tam erişim: "hesabınızın erişebildiği her şeye erişir".
- **Ekle/düzenle formu:** ad, komut, argümanlar, ortam değişkenleri (gizli
  işaretiyle), ağ izin listesi (yalnız yalıtımlıda), zaman aşımları,
  erişim. *Tam sistem erişimi* seçilince formda kalıcı bir uyarı görünür.
- **Yollar arayüzden gelmez** (AGENTS: "Arayüz arka uca dosya yolu
  göndermez").
  - Komut ya çıplak bir program adıdır ya da "Gözat" ile yerel dosya
    seçicisinden seçilir. Seçilen yolu arka uç tutar, arayüze bir kimlik
    döner.
  - Argümanlar programa verilen metinlerdir; Kalfa onları yol olarak
    açmaz.
  - Ele geçirilmiş bir arayüzün sessizce sunucu tanımlamasını önlemek için
    **her kayıt** (ekleme, düzenleme, içe aktarma) yerel pencerede
    onaylanır. Pencere, arka ucun çözdüğü tam komut satırını, çalışma
    kipini ve ağ listesini gösterir. Tam erişim uyarısı da bu
    penceredadır.
- **JSON içe aktarma:**
  - Metin arayüzden arka uca gider; ayrıştırmayı arka uç yapar.
  - Mutlak `command` yolları arka uçta kimliğe çevrilir ve yerel onay
    penceresinde açıkça gösterilir.
  - Sunucular yalıtımlı ve kapalı olarak eklenir.
  - `url` içerenler "1.1b'de" notuyla atlanır.
- **Ayrıntı:**
  - araçlar, denetlenen tanım metinleri ve denetim sonuçları,
  - kapatılan ya da sunulmayan araçların nedeni,
  - son hata,
  - stderr'in son 50 satırı (maskeli, yalnız yerelde).
- **Sohbet:**
  - Başlatma kartı (§5.1).
  - MCP araç kartı mevcut kart biçimini kullanır; başlıkta sunucu adı ve
    rozet görünür.
  - Onay kartındaki "neden izin gerekiyor" satırı erişim türünü söyler.
- Metinler `ui/src/dil/tr.ts`'te, "siz" diliyle yazılır.

## 10. Hatalar

| Durum | Davranış |
|---|---|
| Komut bulunamadı | *Hata*: "`<komut>` bulunamadı; PATH: …" |
| El sıkışma zaman aşımı | *Hata*; stderr özeti (maskeli) |
| Protokol sürümü uyuşmuyor | *Hata*; sunucunun bildirdiği sürüm |
| Çerçeve sınırı, ileti seli, araç listesi sınırı | Sunucu kapatılır; *Hata* (§4.2) |
| Çağrı zaman aşımı | §7.3 durdurma; modele "araç <n> sn'de yanıt vermedi; sunucu durduruldu" |
| Sunucu çöktü | Bekleyen çağrılar hata alır; §7.4 |
| Onaydan sonra tarif ya da tanım değişti | Çağrı yapılmaz; mühür hatası |
| Yalıtım yok, sunucu yalıtımlı | Başlamaz; Ayarlar'da yalıtım durumu |
| Başlatma izni yok | Araçlar sunulmaz; başlatma kartı (§5.1) |

## 11. Test

Her test EVREN'siz ve dış hizmetsiz koşar. Tek istisna, en sondaki ayrı
onaylı canlı denemedir. Test sunucusu depoda, `rmcp` `server`
özelliğiyle (yalnız dev bağımlılığı) yazılır. Senaryoları komut satırı
argümanıyla seçilir.

**Süreç ve başlatma**
- Başlangıçta işaret dosyası yazan ve izinli bir yerel test ucuna bağlanan
  sunucu kullanılır. Yalnız İnceleme oturumu varken, başlatma izni
  reddedildiğinde ve izin hiç verilmediğinde işaret dosyası oluşmamalı ve
  bağlantı gelmemeli (§5.1).
- Tam erişim izni Kalfa yeniden açılınca yeniden sorulmalı.

**Koruma** (§4.3; iki platform, yalıtım testleri)
- Şu değişikliklerden sonra sonraki çağrı eski erişimle çalışmamalı:
  - ilk çağrıdan sonra `.env` oluşturmak,
  - normal bir dosyayı yeni bir gizli kalıpla korumak,
  - okunur dizin iznini kaldırmak,
  - ağ listesini daraltmak.
- Çağrılar arasında yeni `.git` ve yapılandırma dosyası oluşturan
  sunucunun dosyaları karantinaya alınmalı.
- Boşta kapanma çalışmalı.
- **Doğrulanamayan tarama:** Periyodik denetimde sırayla tarayıcı hatası,
  okunamayan bir dizin (`koruma_tam = false`) ve 30 sn'yi aşan bir tarama
  (sahte saat) üretilir. K'nın kısmi özeti eski K ile eşleşecek biçimde
  kurulur.
  - t_K yenilenmemeli.
  - Süreç, etkin çağrı ya da keşif sürerken de tanımlı süre içinde
    durmalı.
  - Başarılı tarama gelene kadar yeniden başlatma olmamalı.
- **Görüntü sınırı ayrı:** Yalnız değişiklik görüntüsünün girdi ya da
  süre sınırı dolar (`goruntu.eksik = true`), koruma yürüyüşü tamdır. Bu
  durum "Doğrulanamadı" sayılmamalı.
- **Mutlak denetim yaşı** (sahte saatle, kısaltılmış yaş): Uzun ve etkin
  bir çağrı sürerken dışarıdan bir gizli dosya oluşturulur.
  - Süreç, denetim yaşı dolduktan sonra en geç bir tarama ve kapanma
    payı içinde durmalı. Etkin çağrı "korumalar değişti" hatasıyla bitmeli.
  - Keşif ve onay bekleme sırasında da aynı sınama yapılır.
- **Stdout üzerinden çıkış** (yerel sahte sağlayıcı, EVREN yok): Ağ listesi
  boş bir sunucu, sınama penceresinde okuduğu gizli dosyanın içeriğini önce
  bir araç sonucuna, sonra ayrı olarak bir araç açıklamasına koyar.
  - Bu test kalan riskin belgedeki tarifini doğrular: içerik sahte
    sağlayıcıya ulaşır.
  - Ulaşma, mutlak denetim yaşı dolduktan sonra yeni bir süreçle tekrar
    edilememeli.

**Sınırlar** (§4.2)
- Satır sonusuz 4 MiB + 1 bayt.
- 5 MiB'lık image bloğu.
- Tek dev stderr satırı.
- 11 sayfalık ve 129 araçlı liste.
- Bildirim seli.

Uygulama yanıt vermeye devam etmeli; sunucu *Hata* olmalı ve süreç
ağacının temizlendiği gözlenmeli.

**İptal** (§7.3) — iptal ve zaman aşımı her senaryoda ayrı ayrı denenir.
- (a) İptal hatasını **hemen** gönderip gecikmeli yazmaya devam eden bir
  çocuk süreç başlatan sunucu: Durdurma tamamlandıktan sonra işaret
  dosyası oluşmamalı.
- (b) İptali uygulayıp **hiç yanıt göndermeyen** uyumlu sunucu: Bu sunucu
  da aynı yoldan durdurulmalı.
- (c) İptali yok sayıp gecikmeli yazan sunucu: Yan etki olmamalı.
- Her senaryoda kilit, ağaç durup son denetim bitmeden bırakılmamalı.
  Bekleyen ikinci yazma aracı kilidi ancak bundan sonra alabilmeli.
- İkinci oturumun sonraki çağrısı yeni süreçte çalışmalı.

**Mühür** (§5.3)
- Onay beklerken şunlardan her biri tek tek yapılır:
  - düz bir ortam değerini değiştirmek,
  - gizli bir ortam değerini değiştirmek,
  - sunucuyu kapatmak,
  - sunucuyu yeniden başlatmak,
  - aynı adlı aracın açıklamasını değiştirmek,
  - aynı adlı aracın şemasını değiştirmek.
- Eski onay hiçbir durumda çalışmamalı. Kısayol T ya da D değişince
  düşmeli.
- **Kısayol ve Π:** T ve D aynı kalırken şunlar ayrı ayrı denenir:
  - (a) yeni bir okunur dizin eklemek,
  - (b) bir gizli kalıbı kaldırmak,
  - (c) yalnız kuşak N'yi değiştirmek (yeniden başlatma),
  - (d) yalnız içeriği değiştirmek (yeni `.env`).

  (a) ve (b)'de eski kısayol yeni erişimi otomatik onaylamamalı ve
  başlatma izni yeniden sorulmalı. (c) ve (d)'de kısayol korunmalı.

**Kod kökleri** (§5.1.1, §5.6; iki platformda gerçek erişim sınırıyla)
- **Karşı örnek:** İzin verilmiş yalıtımlı bir sunucu `npx -y paket@1.2.3`
  ile başlar. Sonra yalıtımlı kabukla şunlar denenir:
  - Sunucunun özel kökündeki açılmış paket betiğini değiştirmek. Yazma
    reddedilmeli.
  - Kabuğun kendi npm önbelleğine aynı paketin değiştirilmiş bir kopyasını
    koymak. Sunucu bu önbelleği görmemeli.

  Sürüm, komut, ortam, açıklama ve şema aynı kalır. Otomatik yeniden
  başlatma değiştirilmiş kodu çalıştırmamalı.
- Ortamında `NPM_CONFIG_CACHE` kabuğun önbelleğini gösteren bir sunucu
  "W'de kodlu" sayılmalı. Kabuk komutundan sonraki başlatma yeniden izin
  istemeli.
- `PATH`'te W'deki bir dizinden çözülen komut "W'de kodlu" sayılmalı.
- Bir sunucu, öbür sunucunun özel köküne yazamamalı.

**Sunucu kodu** (§5.1.1)
- Tam erişimde `node ${CALISMA_ALANI}/sunucu.js` tanımlanır. İlk başlatma
  izninden sonra yalnız betik, normal dosya aracıyla değiştirilir. Araç
  açıklaması ve şeması aynı kalır.
  - Otomatik yeniden başlatma (çökme, boşta kapanma, koruma yenilemesi)
    yeni başlangıç kodunu eski izinle çalıştırmamalı; yerel pencere
    açılmalı.
  - Aynı sınama yalıtımlıda başlatma kartıyla yapılır.
- Çalışma alanı dışındaki bir betiğin içeriği değişince T değişmeli.
- Göreli bir argüman çalışma alanında değil, sunucunun çalışma dizininde
  çözülmeli.
- Çalışma alanında `node_modules/.bin/<ad>` bulunan bir depoda `npx <ad>`
  çalışma alanındaki dosyayı çalıştırmamalı.

**Tanım denetimi** (§6.2)
- ASCII enjeksiyon önce üst açıklamaya, sonra ayrı olarak
  `inputSchema.properties.*.description`'a yerleştirilir.
- Katı kipte model isteğinin `tools` alanında araç olmamalı.
- `list_changed` sonrasında aynı sınama tekrarlanır.
- Önbellek isabetinde denetçi isteği gitmemeli.
- **Tanımda sır:** Bir test sırrını sırayla şu yerlerde geri gönderen
  sunucu kullanılır: üst açıklama, iç içe açıklama, `default`, özellik
  adı.
  - Her durumda araç kapatılmalı.
  - Ham değer şunların hiçbirinde bulunmamalı: sahte denetçi girdisi,
    sahte model girdisi, önbellek, arayüz olayları, JSONL.
  - Bu test, stderr ve sonuç maskelemesinden ayrıdır.

**Protokol**
- `notifications/initialized` gönderilmeli.
- İstemci yetenekleri boş ilan edilmeli.
- Sayfalama çalışmalı.
- `ping` yanıtlanmalı.
- `sampling` isteği "method not found" almalı.

**İçerik ve sırlar** (§5.5, §6.1)
- Gömülü metin ve ikili `resource` §6.1'deki gibi dönüşmeli.
- `content` ile `structuredContent` birlikte gelirse yalnız `content`
  kullanılmalı.
- `isError` ve JSON-RPC hata metni denetçi yolundan gitmeli.
- stderr'deki ve hatadaki gizli değer `[gizli]` olmalı.

**Windows** (§5.6)
- Sunucu kapanınca kabuğun izinleri ve öbür sunucular çalışmayı
  sürdürmeli.
- MCP kendisi profil ya da ACE bırakmamalı.

**Çalışma kipi ve ağ** (§4.1)
- İki çalışma kipinin iki yalıtım durumuyla dört birleşiminin hepsi denenir.
- Yalıtımlı tarif yalıtım yokken başlamamalı.
- Tam erişim, yalıtım hazırken yalıtımsız başlamalı.
- Tam erişimli tanım ağ listesi taşımamalı.

**Ortam ve kasa** (§5.5)
- `GITHUB_TOKEN` ve `API_KEY` sunucuya ulaşmalı.
- `EVREN_API_KEY`, `KALFA_*` ve `SABIT` adları reddedilmeli.
- Yinelenen ad reddedilmeli.
- `NODE_OPTIONS` ezilince T değişmeli.
- İki sunucu ve birden çok sır karışmamalı.
- Yeniden adlandırma kimliği korumalı.
- Silme girişi silmeli.
- Kimlik deposu yoksa sunucu başlamamalı.
- 8 karakterden kısa sır reddedilmeli.
- Sır, tam ve alt dize olarak, stderr'de, hatada, sonuçta ve kesme sınırında
  maskelenmeli.
- Tanım değişince stderr halkası temizlenmeli.

**Protokol dönemleri** (§7.1)
- Yalnız eski dönemi bilen bir test sunucusu çalışmalı.
- Yalnız 2026 dönemini bilen bir test sunucusu çalışmalı.
- Her ikisinde liste değişikliği sonraki turda görünmeli.

**Araç kaynağı** (§7.2)
- Oturum açıldıktan sonra keşfini bitiren sunucunun araçları sonraki turda
  görünmeli.
- Açık oturumda araç eklenmeli ve çıkarılmalı.
- Tur ortasında tanım, doğrulayıcı ve sarmalayıcı aynı sürümden gelmeli.

**Çıkış şeması** (§6.1)
- Şemaya uymayan `structuredContent` denetlenen araç hatası olmalı.
- Eksik `structuredContent` denetlenen araç hatası olmalı.
- Derlenemeyen `outputSchema`'lı araç kapatılmalı.

**Arayüz sınırı** (§9)
- Arayüz komutları yol taşımamalı.
- Kayıt, içe aktarma ve tam erişim yerel onay penceresi olmadan
  kaydedilmemeli.

**`kalfa-core`**
- `mcp_karari` tablosunun her hücresi.

**Arayüz** (Vitest)
- Form, içe aktarma, rozetler, tam erişim uyarısı, başlatma kartı.

**E2e**
- Test sunucusu eklenir, başlatma izni verilir, araç çağrılır, onaylanır,
  sonuç görünür.
- Tam erişim uyarısı ve yerel onay.

**Canlı** (`#[ignore]`, ayrı onay)
- EVREN modeli test sunucusunun aracını seçip çağırır.

## 12. Teslim

1. **G1:** Windows'ta borulu AppContainer prototipi (§4.1).
2. **Plan 1.1a-1:** `kalfa-sandbox` etkileşimli süreç (iki platform).
3. **Plan 1.1a-2:**
   - sınırlı çerçeveleyici ve `rmcp` bağlantısı (§4.2), protokol dönemleri
     (§7.1),
   - `kalfa-mcp`, `mcp_karari`, başlatma izni, tarif ve mühür,
   - ortam birleştirme ve `GizliKasa` (§5.5),
   - tanım denetimi ve çıkış şeması (§6),
   - `McpAracKaynagi` (§7.2), uygulama yöneticisi,
   - arayüz ve yerel onaylı kayıt (§9), e2e.

**Yeni bağımlılıklar:** `rmcp` 3.5.1 (`client`; dev'de `server`).
`rust-version` 1.88, depo araç zinciri 1.98.1'in altındadır.
`transport-async-rw` kullanılmaz (§4.2).

## 13. İncelemelerin yanıtı

### 13.1 Codex'in yerel incelemesi (`INCELEME-dd641a3.md`)

| # | Bulgu | Karar | Çözüm |
|---|---|---|---|
| 1 | Süreç, ilk çağrı onayından önce çalışıyor | Katılıyorum | §5.1: başlatma izni ve İnceleme kuralı; §9 rozet açıklaması |
| 2 | Uzun yaşayan süreçte korumalar eski kalıyor | Katılıyorum; kod doğrulandı (`kabuk/mod.rs` `alan_bilgisi`, `bwrap.rs` gizli bağlamaları, `ayarlar.rs` `degistir`) | §4.3: K, çağrı öncesi ve sonrası denetim, ayar değişiminde kapatma, boşta kapanma; kalan risk açıkça yazıldı |
| 3 | Ham stdio belleği sınırsız | Katılıyorum; `rmcp` kaynağı doğrulandı (`read_until`, `max_length: usize::MAX`) | §4.2: Kalfa'nın sınırlı çerçeveleyicisi ve sınır tablosu |
| 4 | İptal yan etkiyi durdurmuyor | Katılıyorum | §7.3: 2 sn sonra ağacı durdurma, kilidi durdurmadan sonra bırakma; §2'de karar değişikliği |
| 5 | Mühür bütün tanımı ve çalışan örneği bağlamıyor | Katılıyorum; tek alt maddede kısmen ayrılıyorum | §5.3: T, R (P, N dahil), D; çalışan örneğin R'siyle karşılaştırma |
| 6 | Araç tanımları denetlenmiyor | Katılıyorum; kod doğrulandı (`arac.rs` `tanimlar`, `denetci.rs` yalnız sonuç) | §6.2: denetçi, kip tablosu, önbellek, bütçe; §2 bütçe netleştirildi |

**Kısmen ayrıldığım nokta (bulgu 5):** Codex, yeniden başlatmanın araç
kısayollarını da düşürmesini öneriyor. Bekleyen onaylar için katılıyorum:
onlar N'ye bağlıdır ve her yeniden başlatmada düşer. Kısayollar ise T ve
D'ye bağlı kalır. Gerekçeler:

1. Kabukta kısayol süreç örneğine bağlı değildir. Her komut yeni bir
   süreçtir ve kısayol tarifle eşleşir (`arac_asamasi.rs`, `kisayol_bul`,
   `teklif.kapsar`).
2. Koruma değişikliği P'yi değiştirir ama kısayolun onayladığını
   genişletmez: yeni süreç daha güçlü korumayla başlar.
3. Aynı tarifle çökme sonrası yeniden başlatmada kısayolu düşürmek güvenlik
   kazandırmaz, kullanıcıyı yeniden sorgular. T'deki ya da D'deki her
   değişiklik kısayolu yine düşürür.

**Plan notları (Codex'in ek maddeleri):**

| Madde | Yanıtı |
|---|---|
| Liste aktarımı | §7.2 |
| Sayfalama, `initialized`, sürümler, yetenekler | §7.1, §11 |
| `isError` ve hata yolu | §6.1 |
| `resource`, `structuredContent`, maskeleme | §6.1, §5.5 |
| Windows ortak yaşamı | §5.6, §8 |

Ek olarak, ilk sürümün §1'indeki "geride ACE kalmaz" ölçütü, kalıcı
çalışma alanı hazırlığıyla çelişiyordu. Ölçüt "MCP kendisi bırakmaz"
olarak düzeltildi.

### 13.2 PR #19'daki satır içi Codex yorumları (`dd641a3`)

Yorumlar ilk sürüme yazıldı. Altısı r2'de zaten çözülmüştü; geri kalanı
r3'te koda karşı doğrulanıp çözüldü.

| Yorum | Konu | Karar | Çözüm |
|---|---|---|---|
| Stdout ve stderr sınırı | Ham çerçeve ve stderr | Katılıyorum (r2) | §4.2 |
| Tam erişimde ağ süzgeci | Yalıtımsız arka uç ağı kapatamaz (`lib.rs`, `CalistirmaIstegi::ag`) | Katılıyorum | §4.1: tam erişimde ağ listesi yok, ağ "sınırsız" gösterilir |
| Mührün tam tanımı | Düz ortam değerleri | Katılıyorum (r2) | §5.3: T bütün ortamı, komut dosyasının özetini ve kuşağı kapsar |
| Tam tanımın denetimi ve sınırı | İç içe şema metinleri | Katılıyorum (r2) | §4.2 toplam bayt, §6.2 bütün metin değerleri |
| Başlatmadan önce kapı | Süreç başlatma | Katılıyorum (r2) | §5.1 |
| İki protokol dönemi | 2026-07-28'de `server/discover` ve `subscriptions/listen` | Katılıyorum; `rmcp` 3.5.1 `ClientLifecycleMode::Auto` doğrulandı | §7.1 |
| Yenilenebilir araç defteri | `AracKayitDefteri` oturumda sabit (`oturumlar.rs`, `arac.rs`) | Katılıyorum | §7.2: `McpAracKaynagi` ve tur başında anlık görüntü |
| `outputSchema` doğrulaması | MCP belirtimi | Katılıyorum | §6.1 |
| Gömülü kaynak blokları | `EmbeddedResource` | Katılıyorum (r2) | §6.1 |
| stderr'de sır | Maskeleme sırası | Katılıyorum | §5.5: tampondan önce maskeleme, kesme sınırı, en kısa sır 8 karakter, halkanın temizlenmesi |
| Yolların webview'den gelmemesi | AGENTS kuralı | Katılıyorum | §3, §9: çıplak ad ya da yerel seçici; her kayıt yerel onaylı |
| Paylaşılan profil korunsun | Windows hazırlığı | Katılıyorum (r2) | §1, §5.6 |
| Açık seçilen tam erişim | `kip_uyusmasi(false, true)` reddi | Katılıyorum | §4.1: `CalistirmaKipi` tablosu |
| Ortamın güvenli birleşimi | `ortam::uret` `TOKEN`/`KEY` adlarını düşürür | Katılıyorum | §5.5: iki adımlı birleştirme ve ayrılmış adlar |
| Anahtarlı kimlik kasası | `SistemDeposu` tek yuvalı (`anahtar.rs`) | Katılıyorum | §5.5: `GizliKasa` |

### 13.3 Codex'in r3 yeniden incelemesi (`INCELEME-5dd79a4.md`)

| # | Bulgu | Karar | Çözüm |
|---|---|---|---|
| 1 | İptal yanıtı, ağacın durduğunu kanıtlamaz | Katılıyorum. Sessiz iptalin MCP'de önerildiği de doğru. | §7.3: ağaç her durumda `kapat()` ile durdurulur; 2 sn yalnız kapanma payıdır; kilit durdurma ve son denetimden sonra bırakılır. Testler (a), (b), (c). |
| 2 | "En çok 10 dk" ve "ağsız sunucu dışarı gönderemez" desteksiz | Katılıyorum. `denetci.rs` metni sağlayıcıya gönderir; `model_turu.rs` tanımları isteğe koyar. | §4.3: durumdan bağımsız mutlak denetim yaşı; üst sınır "10 dk + tarama + 2 sn + öldürme"; stdout üzerinden EVREN'e çıkış yolu riske yazıldı. Testler: sahte saat ve sahte sağlayıcı. |
| 3 | Başlatma güveni, yorumlayıcının arkasındaki kodu kapsamıyor | Katılıyorum | §3 ve §5.1.1: çalışma dizini çalışma alanı dışında; çalışma alanı kodlu sunucuda yazma kuşağı geçmeden otomatik yeniden başlatma yapılmaz; dışarıdaki dosya yolları T'de. Kalan sınır (sunucunun kendi yüklemesi, sabitlenmemiş paket) yazıldı. |
| 4 | Sır maskelemesi araç tanımlarını kapsamıyor | Katılıyorum. `model_turu.rs` tanımları JSONL'ye yazar. | §6.2'nin 0. adımı: sır geçen araç her şeyden önce kapatılır; ham tanım hiçbir yere gitmez. §5.5'e `serverInfo` ve `instructions` eklendi. |
| 5 | P değişimi her zaman daraltmaz | Katılıyorum | §5.3: Π (politika) tanımlandı; kısayol ve başlatma izni Π'ye bağlı. Π'deki her değişiklik ikisini de düşürür. Yalnız N ya da yalnız içerik değişirse korunur. |

### 13.4 Codex'in r4 yeniden incelemesi (`INCELEME-e5080c2.md`)

| # | Bulgu | Karar | Çözüm |
|---|---|---|---|
| 1 | Proje dışındaki özel ev ve önbelleğe de model yazabilir | Katılıyorum. Kod doğrulandı: `linux/bwrap.rs` `--bind` ev/önbellek/gecici, `linux/mod.rs` `HOME` ve `NPM_CONFIG_CACHE`, `windows/yalitici.rs` miraslı `MASKE_DEGISTIR`. | §5.1.1: modelin yazabildiği bütün kökleri kapsayan W; W'de kodlu sunucuda yazma kuşağı (her kabuk komutu sayılır). §3: sunucuya özel kök (`npx`/`uvx` önbelleği orada). §5.6: Windows'ta sunucuya özel yetenek SID'i, Linux'ta yalnız o sunucuya bağlama. G1'e 3. ölçüm eklendi. §11'de karşı örnek testleri. |
| 2 | Başarısız ya da eksik tarama korumayı doğrulamış sayılmamalı | Katılıyorum. Kod doğrulandı: `tara_ayrintili` okunamayan girdiyi atlar; `goruntu.eksik` görüntü sınırını da işaretler. | §4.3: `KorumaSonucu::{Aynı, Değişti, Doğrulanamadı}`; `koruma_tam` işareti görüntü sınırından ayrı; 30 sn tarama süresi; Doğrulanamadı'da t_K yenilenmez, süreç durur, yeniden başlatma yapılmaz. Üst sınır "10 dk + en çok 30 sn tarama + 2 sn + öldürme". §11'de karşı örnek testleri. |
