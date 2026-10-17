# MCP 1.1a-2 Görev 7 — araç tanımları ve sonuç denetimi

Durum: tamamlandı. Dal `codex/mcp-11a2-plan`, PR #20.

## Davranış

Keşiften gelen ham `tools/list` yanıtı geçici taşıma verisi olarak işlenir.
Önce her aracın ham adı, açıklaması, `inputSchema` ve `outputSchema` içindeki
tüm metinler ve nesne özellik adları bilinen sırlar için taranır. Sır bulunan
araç denetçiye, modele, önbelleğe, arayüze veya JSONL günlüğüne gönderilmeden
yalnız 1 tabanlı sıra numarasıyla (`N. araç tanımında olası gizli bilgi...`)
reddedilir; ham araç adı bile uyarıda yer almaz.

Sayfa sayısı (`10`), sunucu başına araç sayısı (`64`) ve ham tanım baytı
(`256 KiB`) sınırlanır; 1 KiB'yi aşan açıklamalar UTF-8 karakter sınırında
kesilir. Boş ad, `mcp__<sunucu>__<araç>` birleşik adının 64 karakteri aşması
veya `[a-zA-Z0-9_-]` dışına çıkması, ad/açıklama/şemalarda görünmez Unicode
karakteri bulunması ve giriş/çıkış JSON Schema derleme hataları aracı kapatır.
`McpJsonSemasi::derle`, her derinlikteki harici `$ref`, `$dynamicRef`,
`$recursiveRef`, `$id` ve `id` değerlerini şema derleyicisine ulaşmadan
reddeder; ağ veya dosya çözümleyicisi açılmaz. Kanonik tanım özeti `D`,
sunucu kimliği, araç adı, kesilmiş açıklama, giriş şeması ve varsa çıkış
şemasından hesaplanır.

Denetçi aday metni aracın adını, kesilmiş açıklamasını ve `inputSchema`'daki
tüm metin ile özellik adlarını içerir (`outputSchema` modele gönderilmediği
için denetçi metnine girmez, ancak sır/görünmez karakter taramasından ve `D`
özetinden geçer). `(D, denetçi_modeli)` önbelleği yalnız sır taramasından
sonra okunur ve atomik olarak `mcp-tanim-onbellek.json` dosyasına yazılır.
`Katı` kipinde `Şüpheli` veya `Bilinmiyor` araçlar sunulmaz; `Uyar` kipinde
görünür uyarıyla sunulur; `Kapalı` kipinde sınıflandırıcı çağrılmaz. Geçici
bütçe veya ağ hatasından kaynaklanan `Bilinmiyor` sonucu önbelleğe yazılmaz.

Keşif sonrası denetçi çağrıları, sunucuyu ilk sunan oturumun mevcut
`ButceTalepci`'sini (`ButceAmaci::Denetci`) ve aktif tur iptal belirtecini
kullanır; tüm denetçi denemeleri ve gönderim günlükleri
(`ButceTalepci::denetim_gunlugu_kaydet`) o oturumun JSONL kaydına işlenir.
Bütçe sahibi oturum kapanır veya turu iptal edilirse sonuç yeni tanım
kuşağına yayımlanmaz.

`tools/call` sonucunda önce tüm metin, kaynak, `structuredContent`, JSON-RPC
hatası ve `stderr` alanları bilinen sırlar için maskelenir; ardından içerik
dönüşümü (`content` alanı varsa yalnız o, yoksa `structuredContent` JSON)
ve mevcut araç çıktısı kısaltması uygulanır. Görsel ve ses içerikleri atlanır;
`resource_link` URI olarak, metin `resource` başlıklı metin olarak, ikili
`resource` yer tutucu olarak eklenir. `outputSchema` tanımlıysa ve `isError`
değilse `structuredContent` zorunludur ve derlenmiş şemaya göre doğrulanır;
eksik veya uyumsuz yanıt ham değeri sızdırmadan sabit hata mesajına çevrilir.
`isError: true`, şema uyumsuzluğu ve JSON-RPC hata yanıtları
`AracHatasi::Basarisiz(AracCiktisi)` olarak döner ve core'un mevcut araç
sonucu güvenlik denetçisinden geçer.

Ayrıca `crates/kalfa-uygulama/src/mcp/gercek_surec_testleri.rs` içinde
yönetici olmayan Windows CI kullanıcısında (`38920973582`) görülen süreçler
arası derleme kilidi, hedef dizin yazma izni yedeklemesi ve DACL okuma
bayrağı (`FILE_READ_ATTRIBUTES`) düzeltildi.

## Karşı örnekler ve bağımsız inceleme

- Tanımın 5 konumunda (araç adı, açıklama, `inputSchema` özellik adı/metni,
  `outputSchema` özellik adı/metni) bilinen sır bulunduğunda denetçiye,
  modele, önbelleğe, UI'ya ve JSONL'ye sızmama; yalnız sıra numarasıyla ret.
- İç içe `inputSchema` özelliğinde enjeksiyon: `Katı` kipinde reddedilir,
  `Uyar` kipinde uyarıyla kabul edilir, `Kapalı` kipinde denetçi çağrılmaz.
- `(D, denetçi_modeli)` önbellek isabetinde denetçinin yeniden çağrılmaması;
  bütçe reddinde (`Bilinmiyor`) önbelleğin kirlenmemesi ve `Katı` kipinde
  aracın sunulmaması.
- Harici `http://`, `https://`, `file://` ve göreli `$ref`/`$id` şemalarının
  her derinlikte reddedilmesi; yerel `#/$defs/...` başvurularının kabulü.
- `outputSchema` varken `structuredContent` eksik/uyumsuz olduğunda ham
  değer sızdırmadan `AracHatasi::Basarisiz` dönmesi; `isError: true`,
  JSON-RPC hatası ve `stderr` içindeki sırların maskelenmesi.
- Bağımsız salt okunur inceleme (`46d15f95-88a3-4401-b4a5-96b3937263da`):
  alt şemalardaki `$id`/`id` kontrolü, `_meta: null` durumunda `content`
  varlık işaretinin korunması, `denetim.rs` içinde iptal belirtecinin
  `tokio::select!` ile denetçi çağrısını kesmesi ve önbellek yazımında kilit
  tutulmaması bulguları kapatıldı.

## İki platform doğrulaması

| Kontrol | Windows | Ubuntu-24.04, kalfatest |
|---|---:|---:|
| Workspace | 1427 geçti, 19 ignore | 1382 geçti, 19 ignore |
| e2e özellikli Rust | 255/255 | 256/256 |
| Gerçek yalıtım | — (non-interactive oturum; G6'da 50/50) | 32/32 |
| Uyumluluk | — (G6'da 24/24) | 16/16 |
| Gerçek stdio kabulü | 4/4 | 4/4 |
| fmt, workspace/e2e/yalıtım clippy | Temiz | Temiz |

Görev 7'de `kalfa-sandbox` ve `kalfa-tools` üretim koduna dokunulmadı;
WSL `Ubuntu-24.04` (`kalfatest`) üzerinde gerçek yalıtım (32/32) ve
uyumluluk (16/16) tam koştu. Windows'ta gerçek stdio süreç kabulü
(`gercek_surec_testleri`, 4/4) AppContainer içinde geçti.

## Kanıtlar

Yerel kayıtlar `.yerel/mcp-tasarim-inceleme/` altındadır:

- `11A2-AGY-g7-WINDOWS.log`: Windows fmt, check, clippy (workspace + e2e) ve test (workspace + e2e) çıktıları.
- `11A2-AGY-g7-WSL.log`: WSL `Ubuntu-24.04` (`kalfatest`) fmt, check, clippy (workspace + e2e + yalıtım), test (workspace + e2e), gerçek yalıtım ve uyumluluk çıktıları.
