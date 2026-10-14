# Kalfa 1.1a — Yerel MCP sunucuları: Tasarım

**Tarih:** 2026-10-14.
**Durum:** Tasarım, r3. Kullanıcı ilk sürümü bölüm bölüm onayladı (14 Ekim).
§13'te iki incelemenin yanıtı var: Codex'in yerel incelemesi (`dd641a3`,
altı bulgu) ve PR #19'daki satır içi Codex yorumları (15 bulgu). Yeniden
incelemeye hazır. G1 ve uygulama, kullanıcının ayrı onayını bekliyor.
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
  süreçte çalışır.
- Ham stdio sınırları (§4.2) ayrıştırmadan önce uygulanır. Sınırı aşan
  sunucu kontrollü kapanır, uygulama yanıt vermeye devam eder.
- İptal ya da zaman aşımı sonrasında yan etki sürmez. Süreç ağacı
  durdurulmadan çalışma alanı kilidi bırakılmaz (§7.3).
- Onay ve kısayollar §5.3'teki tarife bağlıdır. Tarif değişince eski onay
  yeni tarifle çalışmaz.
- Araç tanımları denetçiden geçer. Katı kipte şüpheli ya da denetlenemeyen
  tanım modele gitmez (§6.2).
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
süreç öldürülmez" yazıyordu. r2'de iptal ve zaman aşımı, sınırlı
beklemeden sonra süreç ağacını durdurur. Gerekçe: MCP iptali işbirliğine
dayanır; aksi hâlde "iptal edildi" görünen işlem dosya yazmayı sürdürebilir.
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

**Ön doğrulama G1 (plan 1.1a-1'den önce):** Windows'ta AppContainer içinde
`CreateProcessW` ile başlatılan sürecin stdin ve stdout borularının
kalıtımı, uzun yaşaması ve Job Object ile kapanışı küçük bir prototiple
ölçülür (`prototip/` altında, 2c gibi). Engel çıkarsa bu belge güncellenir.

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
  yalıtımlı sunucular kapatılır. Bekleyen çağrıları hata alır.
- **`.git` ve yapılandırma denetimi:** Kabuğun `komut_sonrasi_denetim`
  karşılığı şu anlarda çalışır: her çağrıdan sonra (iptal ve zaman aşımı
  dahil), her çağrıdan önce (son denetimden beri oluşanlar için) ve süreç
  kapanırken. Bulunan yeni `.git` ve yapılandırma dosyaları kabuktaki
  kuralla karantinaya alınır.

**Kalan risk (kullanıcı kararına sunulur):** Kalfa dışındaki bir program
(örneğin kullanıcının editörü) çalışma alanına gizli kalıba uyan bir dosya
yazabilir. O zaman çalışan yalıtımlı sunucu dosyayı bir sonraki
denetime kadar okuyabilir. Sonraki denetim şunlardan hangisi önce gelirse
odur: bir sonraki çağrı, Kalfa'nın bir sonraki yazması ya da boşta kapanma.

- **Pencere:** Kabukta bu pencere komutun süresidir (aşama 2 tasarımı
  §7.2). MCP'de pencereyi en çok **10 dk boşta kapanma** sınırlar: çağrı
  almayan yalıtımlı süreç kapatılır, sonraki çağrıda izin geçerliyse
  onaysız yeniden başlar.
- **Ağ ve okunabilirlik:** Ağ listesi boş sunucu okuduğunu dışarı
  gönderemez. Dolu listede ise yalnız izinli hostlara çıkabilir. Windows'ta
  ACL kesildiği andan sonra yeni açılışlar reddedilir; Linux'ta ise
  bağlama, süreç yeniden başlayana kadar dosyayı görünür bırakır.
- **Neden izleyici yok:** Bir dosya izleyicisi atomik bir sınır kurmadığı
  için eklenmez.

## 5. Güvenlik

### 5.1 Başlatma izni

Sürecin başlaması, kullanıcının yazdığı kodu çalışma alanında çalıştırmak
demektir. `initialize` ve `tools/list` sırasında da yazma ve ağ erişimi
olabilir. Bu yüzden süreç ancak şu koşullar birlikte sağlanınca oluşur:

1. Sunucu etkin olmalı.
2. O çalışma alanında **İnceleme dışı kipte en az bir oturum** açık olmalı.
   Yalnız İnceleme oturumları varsa süreç başlatılmaz; çalışıyorsa kapatılır.
3. **Başlatma izni** bulunmalı. İzin şunlardan biridir:
   - *Yalıtımlı:* (tanım özeti T, çalışma alanı) için verilmiş kalıcı izin.
     Ayarlar'dan geri alınabilir; T değişince düşer.
   - *Tam erişimli:* T için verilen **yerel pencere** onayı. Uygulama
     açık kaldıkça geçerlidir; Kalfa kapatılıp açılınca yeniden sorulur.

İzin yoksa oturumda, model dışında bir **başlatma kartı** görünür:
"`<sunucu>` bu çalışma alanında başlatılsın mı?". Karttaki bilgiler: erişim
türü, komut, ağ listesi ve "başladığı andan kapanana kadar, araç
çağrılmasa da çalışır" uyarısı. Model bu kartı açamaz, yanıtlayamaz.
İzin verilene kadar sunucunun araçları modele sunulmaz.

Çökme sonrası yeniden başlatma, boşta kapanmadan dönüş ve koruma
yenilemesi (§4.3) mevcut izinle onaysız yapılır.

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
- **Çalışma tarifi R** = T, yalıtım profilinin özeti P (`profil_ozeti`;
  K'yı içerir) ve **kuşak N**. N her süreç başlatmada artan sayaçtır. R
  süreç başlarken sabitlenir ve süreçte saklanır.
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
- **Kısayol** T ve D'ye bağlanır. T ya da D değişince düşer.
- **Başlatma izni** T ve çalışma alanına bağlanır.

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

### 5.6 Windows çalışma alanı hazırlığı

- AppContainer, çalışma alanının hazırlanmasını ister (aşama 2 §9.4).
  Alan hazır değilse yalıtımlı sunucu başlamaz. Hazırlık mevcut akıştaki
  gibi yerel pencerede onaylanır.
- MCP süreçleri kabukla aynı çalışma alanı profilini ve paket SID'ini
  kullanır. MCP hazırlığı ya da ACE'leri hiçbir zaman kaldırmaz.
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
tanımları denetimsiz verir (`kalfa-core/src/arac.rs`). MCP araçları için
şu kapılar uygulanır:

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
  sunucular da aynı kuralla, çalışma alanında çalışır.
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

1. Sunucuya `notifications/cancelled` gönderilir.
2. En çok **2 sn** yanıt beklenir. Sunucu bu sürede o isteğe yanıt verirse
   (sonuç ya da iptal hatası) süreç yaşar ve sonuç atılır.
3. Yanıt yoksa süreç ağacı `kapat()` ile durdurulur. O kuşağın bütün
   bekleyen çağrıları "sunucu durduruldu" hatasıyla biter.
4. Ardından `komut_sonrasi_denetim` karşılığı çalışır.
5. Kilit **ancak bundan sonra** bırakılır.

Böylece "iptal edildi" ya da "zaman aşımı" denildiğinde yan etki
sürmüyordur; bu, kabuktaki garantiyle aynıdır. Kilit (§5.4) tek çağrıya
izin verdiği için başka bir oturumun aynı süreçte bekleyen çağrısı olmaz.
Durdurulan süreç sonraki çağrıda izinle onaysız yeniden başlar. Bu
yeniden başlatma, çökme sayacına sayılmaz.

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

**Sınırlar** (§4.2)
- Satır sonusuz 4 MiB + 1 bayt.
- 5 MiB'lık image bloğu.
- Tek dev stderr satırı.
- 11 sayfalık ve 129 araçlı liste.
- Bildirim seli.

Uygulama yanıt vermeye devam etmeli; sunucu *Hata* olmalı ve süreç
ağacının temizlendiği gözlenmeli.

**İptal** (§7.3)
- İptali yok sayıp gecikmeli dosya yazan sunucu; iptal ve zaman aşımı
  ayrı ayrı denenir.
- Durdurma tamamlandıktan sonra yeni yan etki olmamalı.
- Kilit durdurmadan önce bırakılmamalı.
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

**Tanım denetimi** (§6.2)
- ASCII enjeksiyon önce üst açıklamaya, sonra ayrı olarak
  `inputSchema.properties.*.description`'a yerleştirilir.
- Katı kipte model isteğinin `tools` alanında araç olmamalı.
- `list_changed` sonrasında aynı sınama tekrarlanır.
- Önbellek isabetinde denetçi isteği gitmemeli.

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
