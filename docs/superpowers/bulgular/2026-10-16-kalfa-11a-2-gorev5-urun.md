# MCP 1.1a-2 — Görev 5 ürün doğrulaması

Tarih: 16 Ekim 2026. Ajan: Codex. KALFA-24, PR #20.
Taban: `e75822f`. Görev 5 tamamlandı; ürün ve bu rapor aynı commit içindedir.

## Davranış ve kapsam

Çalışma alanı/sunucu başına tek süreç sahibi N/R'yi sabitler. Süreç
oluşturma, son W/Π/izin/oturum denetimiyle aynı yönetici adımındadır.
Başlatma veya RPC beklenirken kontrol kanalı ve her kuşağın mutlak
denetimi ayrı ilerler. Kapanış ancak süreç ağacı durduğunda ve zorunlu
son denetim tamamlandığında doğrulanmış sayılır.

Mutlak koruma yaşı ve boşta kapanma iki ayrı 10 dakikalık saattir.
Kalfa'nın dosya yazma/düzenleme ve kabuk sonucu, yazma sonrası koruma
taramasını bekler; çağıran gelecek bırakılırsa tarama bağımsız sürer.
W'de kodlu sunucu için yazma kuşağı otomatik yeniden başlatmayı da
yeniden izin istemeye zorlar. Çökme tekrarları 1/4/16 saniyedir.

Bu görev henüz model turuna MCP aracı, çağrı kilit kaydı, sonuç denetimi,
arayüz veya WebDriver kabulü bağlamaz; bunlar Görev 6–12'dedir.

## Devirdeki beş şüphe

| Şüphe | Sonuç ve kanıt |
|---|---|
| `bitisi_bekle` süreç kilidi | Gerçek uçta bekleme iptale dayanıklıdır; izleyici 5 ms'lik bekleme diliminden sonra kilidi bırakır. Uç artık RPC boyunca manager Mutex'i altında değildir. |
| Uç Mutex'i RPC boyunca tutuluyor | Uç `Arc<dyn McpSurecUcu>`; istemci RPC geleceği sahipli komut göndericisini alır. Kilit yalnız geleceğin kurulması sırasında tutulur. Uzun RPC sırasında çıkış/stop testi geçer. |
| t_K 60 sn / 10 dk | Gerçek iki sabit de 600.000 ms. Eski devirdeki 60 sn ifadesi yanlıştı. Sahte saatte bağımsız mutlak/idle testleri ve gerçek zamanlayıcı görevi sınanır. |
| `let _ = profil` | Gereksiz hesap kaldırıldı; P, `McpTarifi::yeni` içinde tek kaynaktan üretilir. |
| Plan kutuları | Görev 3–4 commit ve doğrulama kayıtlarıyla, Görev 5 son kabul sonuçlarıyla güncellenir. |

## Bağımsız incelemede düzeltilen ek kusurlar

- Başlatma rezervasyonunda W ve stop revizyonu yarışları; aynı anda iki
  başlatma; eski kuşağın geciken hatasının yeni kuşağı durdurması.
- El sıkışması geleceği bırakılınca sahipsiz süreç; boş süreç tutamacına
  kapanmış gibi başarı dönülmesi; kapanış beklemesinde kayıp bildirim.
- Elle çağrılması gereken zamanlayıcı ve otomatik yeniden başlatmanın
  yalnız zaman damgası tutması. Her kuşağa gerçek zamanlayıcı eklendi.
- Bırakılan RPC etkinlik sayacının kalması ve tek sunucunun yavaş
  kapanışının başka sunucunun mutlak denetimini bekletmesi.
- Windows yalıtıcısından önce sunucu kökünün normal izinlerle
  oluşturulması; kökü artık yalnız yalıtıcı oluşturur.
- Tanımdaki 30 sn başlatmayı istemcinin 25 sn'de kesmesi. Gerçek
  26 sn `initialize` önce 25,07 sn'de düştü; düzeltmeyle geçti.
- Yazma sonrası taramayı yalnız kuyruğa atıp araç sonucunu döndürme.
  Engellenen taramayla test önce düştü; sonuç artık bitişi bekler.

İncelemeler GPT-6 Luna/max ile yapıldı. Son ek iki düzeltmeyi yazmayan
ajanın salt okunur kontrolünde yeni kesin bulgu kalmadı. Geri alma
sonucunun da aynı beklemeye bağlanması Görev 9 için ayrıca not edildi.

## Test kanıtları

| Kontrol | Windows | Ubuntu-24.04, kalfatest |
|---|---:|---:|
| Workspace | 1354 geçti / 0 hata / 19 ignore | 1309 geçti / 0 hata / 19 ignore |
| e2e özellikli Rust | 234 / 234 | 235 / 235 |
| Gerçek yalıtım | 50 / 50 | 32 / 32 |
| Uyumluluk | 24 / 24 | 16 / 16 |
| Client-only lib/bins, workspace/e2e/yalıtım clippy, fmt | Geçti | Geçti |

Son kaynakta genel ve uyumluluk kontrolleri
`11A2-CODEX-g5-son-{WINDOWS,WSL}.log`; değişmeyen kernelin gerçek yalıtımı
R3 loglarındadır. Yönetici hedefli 23/23 (`11A2-G5-HEDEFLI-R2.log`).
25 Rust dosyası Windows/WSL'de birebir aynı; son manifest SHA-256:
`4102a5a181b22eef34ece021ab8b1e2860a1de3178040c3f7105c2d8b9dc16ed`.

Başlatma 26 saniye testi ve yazma sonrası tarama önce başarısız oldu,
düzeltmeyle geçti. Sayaç, zamanlayıcı, stop revizyonu, tek sahip, eski kuşak
ve otomatik yeniden başlatma düzeltmeleri tek tek geri alınınca ilgili
davranış testi düştü (`11A2-G5-*-KIRMIZI.log`); geçici varyantlar geri alındı.
Ham günlükler ve kaynak manifestleri yerel `.yerel/mcp-tasarim-inceleme/`
altındadır; açık depoya özel ortam/günlük taşınmaz.

### Eski uyumluluk beklentileri

İlk R1 Windows uyumluluk koşusu 22/24 oldu. İki A15 testi Cargo'nun
kanonikleştirme hatası vermesini beklerken fetch başarılı oldu ve itoa/hex
indirdi. Bu beklenmeyen indirme kullanıcıya/Jira/PR'a açıklandı; deney
aynı biçimde tekrarlanmadı. Önceki Antigravity Windows workspace koşusunda
bu feature'lı takım 0 testti.

Eski A15 beklentileri, önceden önbelleğe alınmış memchr arşivi ve indeksini
özel CARGO_HOME'a kopyalayan testle değiştirildi. Kaynak ağacı kopyalanmaz:
`cargo fetch --offline` onu gerçekten oluşturur ve CONNECT üretmez.
Windows Cargo CONNECT yolu ayrıca boş sahte DNS ile ölçülür; dış ağa
çözümleme veya paket indirmesi yoktur. Python CONNECT probu pip download
yerine urllib ile bellekte yanıt okur. Npm örneğinin bağımlılığı yoktur.

Sistem pip testindeki genel ERROR, A16 kanıtı değildi. Yeni test
`PIP_CONFIG_FILE=nul`, `--isolated`, `--no-index` ile config/find-links ve
ortam seçeneklerini kapatır; AC/Temp temizleme uyarısını ve gerçekten
kalan pip dizinini birlikte ölçer. Bu test paket kurmanın başarı veya
başarısızlığını ölçmez. Son düzeltmeler bağımsız salt okunur incelemeden
geçti; §7.4/§9.6/§14 eski A15 iddiası ölçülen kapsamla güncellendi.

R2'nin genel workspace kısmı da yeşildi; e2e bölümü sohbet kesintisinde
yarım kaldı ve başarı sayılmadı. R3 ve son koşular bu eksiği kapatır.
e2e özellikli Rust kontrolleri WebDriver kabulü değildir. Mevcut Windows
paketli Python/Node testi KALFA_TEST_ARAC_ZINCIRI verilmediğinde erken
döner; bu koşu paketli araç zinciri kabulü olarak sunulmaz.

## Temizlik

Bu işin bitmiş Windows TEMP kökleri yalnız mutlak yol, bağlantı ve sahip
denetimiyle temizlenir. Testin tek sahte SID'li paylasilan yaprağı eski
güvenli temizleme kuralıyla, yönetici olmadan geri alınır; DACL yedeği
yerel kayıttadır. Eski 44 dizine dokunulmadı. Ortak target'lar, ürün
worktree'si ve WSL kaynak kopyası Görev 6–12 devamı için korunur.

## Sınırlar ve sonraki iş

Görev 6: çağrıya ait sahipli çalışma alanı kilidi, deadline/iptal/drop
sonrası bağımsız temizlik ve core tamamlanma bağlantısı. Gerçek MCP
yalıtım kabulü ile UI/WebDriver henüz yapılmadı. Canlı EVREN, şart
kabulü, Cargo dosyası/bağımlılık değişikliği veya merge yok.
