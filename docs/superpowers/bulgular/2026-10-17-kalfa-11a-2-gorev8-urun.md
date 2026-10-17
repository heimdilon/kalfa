# MCP 1.1a-2 Görev 8 — tek tur görüntüsü ve bütçe olay akışı

Durum: tamamlandı. Dal `codex/mcp-11a2-plan`, PR #20. Uygulayan: Claude.

## Davranış

**Görüntü.** `AracKayitDefteri::anlik_goruntu(kip)` her model turunun başında
bir kez alınır. Sabit araçlar bugünkü `AracKayitDefteri::yeni` ile aynen
çalışır; `yeni_kaynakli` ayrıca bir `AracKaynagi` bağlar. Kaynağın hazır
yayımı (`hazir_araclar`) senkron okunur; bitmemiş keşif ya da denetim turu
bekletmez. Kaynak girdisi `KaynakAraci` yapılırken ad, açıklama ve giriş
şeması bir kez okunur ve derlenir (harici başvurulu şema derleyiciye
ulaşmaz); görüntü sarmalayıcıyı yeniden sorgulamaz. Sağlayıcı isteği,
`AracTanimlari` gönderim günlüğü, bağlam bayt ölçüsü, 429 tekrarı, 413
sonrası yeniden kurulan istek, `planla` içindeki ad çözümlemesi ve argüman
doğrulaması aynı görüntüyü kullanır; `ModelCevabi` görüntüyü araç aşamasına
taşır. Tur ortasında yayımlanan yeni sürüm bir sonraki model turunda görünür.
İnceleme kipinde kaynak hiç okunmaz; kaynak araçları ne sunulur ne çözülür.
Aynı adı tutan sabit araç ya da önceki girdi varsa sonraki kaynak girdisi
görüntüye girmez.

**Denetim ve bütçe.** Oturum her model turunun başında, yalnız aktif tur
varken ve İnceleme kipinde değilken `denetim_islerini_al` ile bekleyen tanım
denetimlerini sahiplenir ve dönen işleri `tokio::spawn` ile başlatır; hiçbirini
beklemez. Sahip (`AracDenetimSahibi`): oturum kimliği, oturumun mevcut
`ButceTalepci`'si ve turun iptal belirteci; yeni bütçe/EVREN sayacı
yaratılmaz. İşlerin izin talepleri mevcut 5b döngülerinde (model akışı,
iptal edilebilir bekleme, araç yürütme, boşta komut bekleme) işlenir; kart,
5b kuralıyla süren deneme bitince açılır.

**Uygulama kaynağı.** `mcp::kaynak::McpAlanAracKaynagi` alan başına tek
kaynaktır (`McpAracKaynaklari::al`); `oturumlar.rs` her oturum bağlamını
`yeni_kaynakli` ile kurar. `denetim_bekliyor(anahtar, iş)` keşfi biten sunucuyu
bekletir; aynı anahtar için aynı anda tek sahip olur ve sahiplenen oturumlar
`denemeler`'de tutulur. İş sonuç vermezse (bütçe reddi, iptal), sahibin turu
biterse ya da iş bırakılırsa kayıt yayımlanmadan bekleyene döner; sıradaki
uygun aktif oturum sahiplenir. Yeni `denetim_bekliyor` aynı anahtarın eski
işinin sonucunu geçersiz kılar; `kaldir` yayımı ve bekleyeni siler. Yayım
değişmezdir ve sürümü artar.

## Kapsam notları

- Keşfin `denetim_bekliyor`'a ve yayımlanan `McpArac` sarmalayıcılarına
  bağlanması yaşam döngüsü işidir (Görev 9); bu görev kaynağı ve core
  dikişini kurar.
- Görüntü `D`'yi ayrıca taşımaz: `D` sarmalayıcının hazırlığındadır ve G6'nın
  canlı R/D denetimi tur ortasında değişen tanımın RPC'sini reddeder
  (`cagri.rs` kabul testleri).
- 5b kuralı gereği süren bir deneme varken bütçe kartı açılmaz; denetimin
  talebi o deneme bitince karta düşer. Durdur kararından sonra kart açık
  kalır ve sonraki turda aynı kartın kararıyla sürer.

## Ek düzeltme: görünmez Unicode kapısı (G7)

Push sonrası otomatik güvenlik incelemesi, G7 raporunun aksine görünmez
Unicode denetiminin `tanimi_denetle` içinde yalnız (zaten ASCII'yle sınırlı)
birleşik ada uygulandığını buldu; açıklama ve şema metinleri denetlenmiyordu.
Artık açıklamada ya da giriş/çıkış şemasının herhangi bir yazısında veya
anahtarında görünmez/yön değiştiren karakter varsa araç sıra numarasıyla
kapanır (`\n`, `\t`, `\r\n` serbest). Test önce düştü, sonra geçti. Windows
`kalfa-mcp` 99/0 ve `kalfa-uygulama` 242/0; WSL ikisi birlikte 343/0;
fmt/clippy temiz.

## Kanıt

Yeni core entegrasyon testleri (`crates/kalfa-core/tests/arac_goruntusu.rs`,
6 test), her okumada sıradaki sürümü veren bir test kaynağıyla:

- Tur ortasında değişen kaynak: kaynak model turu başına bir kez okunur;
  çağrı v1 şemasıyla doğrulanır ve v1 sarmalayıcısı çalışır (v2 hiç
  çalışmaz); sonraki tur v2'yi görür; JSONL `AracTanimlari` kayıtları iki
  isteğin tanımlarıyla birebir aynı.
- 429 tekrarı aynı tanımları gönderir; kaynak bir kez okunur.
- İnceleme kipinde kaynak okunmaz, araç sunulmaz/çalışmaz, denetim sahiplenilmez.
- Bitmeyen denetim turu bekletmez; boştaki oturum sahiplenmez; tur bitince
  işin belirteci iptal olur.
- Bütçe sınırı 1 iken denetimin talebi oturumu kilitlemeden karta düşer;
  devam kararından sonra araç sonraki model turunun görüntüsünde görünür ve
  çalışır.
- Durdur kararında denetim yayımlamaz; sonraki turda yeniden sahiplenilir.

Uygulama birim testleri (`mcp::kaynak`, 5 test): tek sahip ve yayım, kapanan
sahibin sonucunun yayımlanmaması ve sıradaki sahip, sonuçsuz ya da bırakılan
işin kaydı geri vermesi, yeni nesil/kaldırma, alan başına tek kaynak.

Mutasyon denetimi: `planla` görüntü yerine kaynağı yeniden okuyunca ilk test,
429 dalı isteği yeniden kurunca 429 testi düştü; ikisi de geri alındı.

Doğrulama (yerel `.yerel/mcp-tasarim-inceleme/11A2-CLAUDE-g8-{WINDOWS,WSL}.log`):

- Windows: workspace **1438 geçti, 19 ignore**; `e2e` özellikli Rust **260/260**;
  fmt, clippy ve `e2e` clippy temiz.
- WSL (`Ubuntu-24.04`, `kalfatest`): workspace **1393 geçti, 19 ignore**;
  `e2e` özellikli Rust **261/261**; fmt, clippy ve `e2e` clippy temiz.
- Artış iki platformda da G7 tabanına göre +11 (6 core + 5 uygulama).
  `kalfa-sandbox`/`kalfa-tools` ve eklenti/WASM koduna dokunulmadığı için
  yalıtım, uyumluluk ve `wasm32-wasip2` clippy koşulmadı.
