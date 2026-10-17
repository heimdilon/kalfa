# MCP 1.1a-2 — Görev 1 ürün doğrulaması

Tarih: 15 Ekim 2026. Ajan: Codex. İş: KALFA-21; PR #20.
Taban: `cf4e5f3`; ürün kodu `4e5464a`, CI/AGENTS `97264ff`; [plan](../plans/2026-10-15-kalfa-11a-2-mcp.md).

## Değişiklik ve kapsam

`kalfa-mcp`, özel `Transport<RoleClient>` üzerinden rmcp 3.5.1 kullanır.
JSON ayrıştırmadan önce 4 MiB; LF ayraçtır, CR bütçeye dahildir. Gelen ve
giden ileti kuyrukları 64 öğedir. Bu, tüm uygulamanın belleğine bir bayt
tavanı iddiası değildir. EOF, bozuk/yarım JSON, taşma ve beklenmeyen görev
bitişi ayrı ölümcül hata kanalından süreç sahibine ulaşır. İlk hata korunur.
Kapatma, okuma/yazma görevlerini durdurup bitişlerini bekler.

İstek kimlikleri sınırlı tutulur; geç yanıtlar atılır. Yalnız aktif
aboneliğin ilk ACK'i geçer; geç/yanlış/tekrarlı ACK reddedilir. Yalnız
ACK almış aboneliğin tek iptali SDK'ye ulaşabilir. Sıradan bildirimler
SDK'de sınırsız görev üretemez. Ping yanıtlanır; roots, sampling ve
elicitation dahil öbür sunucu istekleri -32601 alır. İstemci yetenekleri
boştur. Araç çağrısı MRTR ek girdi/otomatik tekrar akışını çalıştırmaz.

SDK, özel OS iş parçacığında current-thread Tokio runtime'ıyla çalışır.
Runtime'ın oluşturulması, SDK görevleri ve kapanışı `NoSubscriber`
kapsamındadır; uygulamanın global TRACE katmanı bunu açamaz. Ham SDK
future/Peer dışarı verilmez. Özel taşımanın I/O görevleri çağıranın
runtime'ında kalır; Windows dosya borusunun blocking okuması SDK
runtime'ının bırakılmasını bekletmez. SDK cache'i ilk liste öncesi kapalıdır.
Uzak hata gövdesinin Debug/Display'i güvenlidir; ham gövde yalnız ilerideki
denetim için açık yöntemle alınabilir.

Liste en çok 10 sayfa, 128 araç ve 256 KiB ad/açıklama/giriş-çıkış şeması
taşır; hata olunca kısmi liste dönmez. Değişiklikler tek sürüm sayacında
birikir. Yenileme durumu en sık 5 sn, abonelik yoksa/bitmişse 30 sn
yoklaması için hazırdır; gerçek tur/yönetici zamanlaması sonraki görevdir.

Bu crate henüz uygulamanın araç defterine/arayüzüne bağlı değildir.
Başlatma izni, sır kasası, süreç yöneticisi, denetçi, mühür/kilit ve
arayüz kabulü Görev 2–12'de kalır. Test sunucusunun stdin kapanınca çıkması,
gerçek yalıtılmış süreç ağacının durduğu kanıtı olarak kullanılmaz.

## Doğrulama

Windows ve WSL hedefli takım: **37/37** (10 birim, 15 protokol, 12 sınır
testi). Üretim istemcisi yalnız `client` özelliğiyle derlenir; CI ve
AGENTS'a bunun için ayrı `cargo check -p kalfa-mcp --lib --locked` eklendi.
Son kaynakla Windows workspace **1271 geçti / 0 hata / 19 atlanan**;
Ubuntu 24.04 WSL (`kalfatest`) **1224 / 0 / 19**. Hedefli 37 test bu
workspace toplamlarında birer kez bulunur; ayrı hedefli koşu toplamı
ikinci kez eklenmedi. Atlanan testler mevcut canlı testlerdir ve koşmadı.
`cargo clippy --workspace --all-targets --locked -- -D warnings` ve
`cargo fmt --all -- --check` iki platformda temiz. Ağdan indirme yok;
Windows ve WSL mevcut önbellekleri çevrimdışı kullandı.

Arayüz, gerçek yalıtım ve WebDriver kaynakları değişmedi; bu görevde bu
takımlar yeniden koşmadı. Önceki Tauri derleme girdisi `ui/dist` yeniden
kullanıldı; yeni arayüz derlemesi iddia edilmez. Bu doğrulama MCP süreç
yalıtımı, tanım denetimi veya tam ürün kabulünün yerine geçmez.

Kabul senaryoları: iki protokol dönemi ve pozitif TTL'de iki gerçek RPC;
UTF-8 bayt bölünmesi/çok satır; boş/bozuk/yarım JSON; CR ve 4 MiB sınırı;
5 MiB görselin ayrıştırma öncesi reddi; 1.000 liste bildirimi; eşleşmeyen
iptal seli; 64 geçerli ileti ve 65'incide taşma; yanlış/tekrarlı/geç ACK;
boş filtre ve aboneliğin son yanıtı; 11 sayfa/129 araç/256 KiB ve +1;
üç sunucu iptal davranışı; geç yanıt ve sıradayken iptal; gecikmeli çocuk
yazısı; uzak hata yansıması; global TRACE altında sır yansıtma; dolu
yazma kuyruğunda engellenen iptal ve yarım bırakılıp yinelenen kapanış.

Karşı örnekler:

- G2 tabanıyla temiz EOF ve beklenmeyen ACK testleri önce düştü.
- Okuyucu görevi panic edince sahibine hata ulaşmıyordu; yeni test önce
  düştü, görev korumasıyla geçti.
- `NoSubscriber` kapsamı geçici kaldırılınca ham yük marker'ı global
  yakalayıcıya ulaştı ve ilgili test düştü.
- SDK cache'i geçici açılınca ikinci liste RPC'si çıkmadı, test düştü.
- Engellenmiş yazıcı ve dolu giden kuyrukta iptal bildirimi kapanışı
  kilitliyordu. Yeni test eski kodda düştü; kapanışın hem sahip hem taşıma
  kanalını doğrudan sinyallemesiyle geçti.
- Yarım bırakılan `kapat()` çağrısı join handle'ını kaybediyordu. Sonraki
  kapanışın gerçek iş parçacığı bitmeden dönmesini yakalayan test önce
  düştü. Join görevi `await` öncesinde istemcide saklanarak düzeltildi.
- Her geçici değişiklik `finally` ile geri alındı. Kapanış düzeltmelerinin
  ardından asıl kaynakta 37 test iki platformda yeniden geçti.

GPT-6 Luna / max bağımsız incelemesinin iki kapanış bulgusu kabul edilip
düzeltildi; son yeniden incelemede açık somut bulgu kalmadı. Bu inceleme
kaynak ve kırmızı/yeşil loglara dayanır; incelemeci testleri ayrıca
koşturmadı. Tanım boyutunda `title` gibi ek alanları sayma önerisi,
tasarım §4.2'nin açıkça ad/açıklama/giriş-çıkış şemalarını sınırladığı
gösterilince geri çekildi; sözleşme değiştirilmedi.

Yerel loglar `.yerel/mcp-tasarim-inceleme/11A2-G1-*` altındadır.
`kirmizi.log`, `PANIK-KIRMIZI.log`, `LOG-KARSITORNEK.log`,
`CACHE-KARSITORNEK.log`, `KAPATMA-KIRMIZI.log`, `KUYRUK-KIRMIZI.log`,
`KAPATMA-YESIL.log`, `WINDOWS-SON.log`, `WSL-SON.log`. Son eki `SON`
olmayan geniş doğrulama logları, son iki kapanış düzeltmesi öncesindeki
35 testli kaynağa aittir; nihai kanıt yerine kullanılmaz.

Yeni bağımlılık yalnız sabit rmcp ve üç zorunlu geçişli paketidir;
mevcut tracing doğrudan kullanıldı. Çalışma çevrimdışı önbelleklerden
yürütüldü. Canlı EVREN, sistem kurulumu/ayar değişikliği ve PR merge'i yok.

## Sonuç ve devam

Görev 1'in plan kapsamındaki kabulü tamamlandı. Sonraki ürün işi Görev 2 —
tanım, kod kökleri ve tarif. PR #20 taslak ve 1.1a-2'nin bütünü açık kalır.

## Doğrulanan kaynak

İstemci SHA-256:
`6e3fe26126c1410eec98b6a1edfccb64a45ab97d745fc82fa31c3830ec92ef7a`.
Taşıma:
`9d6e6862a54d76d08e082da09787d237fae35b3c615a731ebbc8b7add985c5a5`.
Ürün Cargo.lock:
`f98e26af2e26d6b5a662a37def2968a115bd2c9f22263fb0c9cbcbd036c391bb`.
Test sunucusu:
`e01a1f40dfadf822d38a546b6acbb9a2b290564b827b4a83d64f568dd20383d2`.
İki platformun kaynak özetleri aynıdır; yerel tam liste
`11A2-G1-KAYNAK-SON.json` ve WSL son logundadır.
## Temizlik

Kapanış karşı örneğinin ayrı derleme dizini kaldırıldı. Tam Windows
koşularının bu işe özel iki TEMP kökü de silindi. Mevcut
`kayitsiz_ayni_sid_ace_hazirlama_ve_geri_alma_sirasinda_korunur` testi,
`paylasilan` adlı boş dizini sahte `S-1-15-2-98765` izniyle korumalı
bırakıyordu. Yalnız bu iki koşunun kendi dizinlerinde sahiplik ve sahte
SID doğrulandı; yönetici kullanılmadan DACL mirası geri açılıp dizinler
silindi. İlk genel `Set-Acl` denemesi gereksiz güvenlik ayrıcalığı
istedi ve başarısız oldu; yalnız DACL işlemi yapan `icacls` başarılıydı.
Bu mevcut testin temizleme kusuru ürün taşıması değişikliğine katılmadı.
Eski 44 dizine, başka ajanın kopyalarına ve ortak target/cache'e dokunulmadı.

WSL kaynak kopyası aynı ürün çalışmasının sonraki görevlerinin doğrulaması
için korunuyor; iş sonunda yalnız bu kopya kaldırılacak. Etkin ürün
worktree'si de PR #20 uygulaması devam ettiği için korunuyor.
