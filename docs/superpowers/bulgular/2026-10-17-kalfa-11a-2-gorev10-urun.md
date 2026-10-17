# MCP 1.1a-2 Görev 10 — IPC ve yerel onay sınırı

Durum: tamamlandı. Dal `codex/mcp-11a2-plan`, PR #20. Uygulayan: Claude.

## Davranış

**Uçlar (ana pencere, 15 komut).** `mcp_dinle`, `mcp_durumu`,
`mcp_sunucu_ayrintisi`, `mcp_komut_sec`, `mcp_taslak_olustur`,
`mcp_taslak_guncelle`, `mcp_taslak_kaydet`, `mcp_ice_aktar_onizle`,
`mcp_ice_aktar_gizlilik`, `mcp_ice_aktar_kaydet`, `mcp_etkin_ayarla`,
`mcp_sil`, `mcp_yeniden_baslat`, `mcp_baslatma_karti_yanitla`,
`mcp_baslatma_izni_geri_al`. Arka uç `crates/kalfa-uygulama/src/mcp/komutlar.rs`;
Görev 3'ün yerel onaylı `McpKayitDefteri`'si artık `Uygulama`'ya bağlı.

**Yol sınırı.** Arayüz yol göndermez: komut yerel seçiciyle seçilir ve opak
kimlik döner; çalışma alanı oturum (`OturumId`) ya da alan kimliğiyle gelir,
kök yöneticinin alan kaydından çözülür. Kayıt için alan verilen oturumunki,
yoksa son oturumunkidir; açık oturum yoksa kayıt reddedilir.

**Yerel onay.** Kayıt (ekleme, düzenleme, içe aktarma) ve silme yerel
pencereden geçer; ret `false` döner, hiçbir şey yazılmaz. Başlatma kartının
arayüz yanıtı yalnız yalıtımlı kipte yeterlidir (§5.1); tam erişimde yönetici
arayüzün "kabul"üne ek olarak yerel pencere onayı ister. Kart tek
kullanımlıdır; alanı ve kökü kartın kaydından gelir.

**Durum görüntüsü.** `McpDurumuDto`: sunucu özetleri, alan başına durum
(etiket, süreç kuşağı N, hata, izin türü, sunulan araç sayısı) ve bekleyen
başlatma kartları. Ayrıntı: denetlenen araç özetleri, kapatılan araçlar
yalnız sıra numarasıyla, maskeli stderr; `serverInfo`/`instructions`
taşınmaz. Ana pencerenin kanalı (`mcp_dinle`) yalıtım kanalı kalıbındadır:
yeni kanal güncel görüntüyü kilit altında alır, her görüntünün `revizyon`'u
artar ve TS `mcpDinle` son görülenden eski görüntüyü atar. Alan bağlantısı
başlatma kartı ürettiğinde ve yayım değiştiğinde, komutlar her değişiklikten
sonra görüntü yazar.

**Tauri.** Komutlar `komut_adlari.rs`, `komutlar::kur`, `capabilities/ana-pencere.json`
ve TS sözleşmesinde (`ui/src/ipc/{ana,tipler}.ts`) aynı commit'te. Pencereye
Tauri çekirdek izni açılmadı; `tests/izinler.rs` tutarlılığı denetler.

## Ek düzeltme: başlatma tura bağlıydı (Görev 9)

Uygulama düzeyindeki kabul testi kartın kanala hiç düşmediğini gösterdi.
Kaynağın iş sarmalayıcısı (`biased` select) tur iptal edildiyse işi ilk
kez çalıştırmadan bırakıyordu; model hızlı yanıtlayınca başlatma ve kart
hiç gerçekleşmiyordu. Başlatma bütçe harcamaz ve tura bağlı değildir;
artık planlama anında ayrı görevde başlar, iş yalnız sonucunu bekler.
Bütçeli keşif turun iptaline bağlı kalır.

## Kapsam notları

- Svelte bileşenleri (Ayarlar bölümü, form, sohbet başlatma kartı) Görev 11.
- Süreç çökmesi/boşta kapanma gibi yöneticinin kendi geçişleri kanala ayrıca
  yazılmaz; arayüz bölüm açılırken `mcp_durumu` okur, sonraki komut ya da
  tur görüntüyü tazeler.
- Yeni e2e sahtesi eklenmedi; IPC sözleşmesi `mockIPC` ile, uçtan uca kabul
  Görev 12'de.

## Kanıt

- `crates/kalfa-uygulama/tests/mcp_komutlari.rs` (2 test): kayıt ve silme
  yerel ret/onayla; yerel pencere çözülmüş komutu gösterir. Model turu
  başlatma kartını kanala yazar, revizyonlar artar; tam erişimde arayüz
  "kabul"ü yerel ret karşısında izin vermez; kart ikinci kez yanıtlanamaz.
- `ui/src/ipc/ipc.test.ts` (2 test): 14 komutun ad/argüman sözleşmesi;
  kanal eski revizyonu atar (süzgeç `<` yapılınca düştü).
- `crates/kalfa/tests/izinler.rs` 8/8: komut listesi, capability ve işleyici eşit.

Arayüz: `npm run check` 0 hata/0 uyarı, 435/435 test, derleme ve paket
denetimi temiz (worktree'de `node_modules` yok; ana klasörün aynı kilit
dosyasından kurulumu OneDrive dışındaki geçici kopyaya alınarak, indirme
yapılmadan koşuldu; kopya silindi).

Doğrulama (yerel `.yerel/mcp-tasarim-inceleme/11A2-CLAUDE-g10-{WINDOWS,WSL}.log`):

- Windows: workspace **1447 geçti, 19 ignore**; `e2e` özellikli Rust **267/267**;
  fmt, clippy ve `e2e` clippy temiz.
- WSL (`Ubuntu-24.04`, `kalfatest`): workspace **1402 geçti, 19 ignore**;
  `e2e` özellikli Rust **268/268**; fmt, clippy ve `e2e` clippy temiz.
- İlk koşuda iki platformda clippy düştü (`too_many_arguments`; testte
  `redundant_pattern_matching`), Linux'ta testteki sahte yürütülebilirin
  çalıştırma izni eksikti. Düzeltildi; WSL baştan koşuldu. Windows tam test
  koşusu bu davranışsız düzeltmelerden önceydi; sonra Windows fmt, iki
  clippy ve `mcp_komutlari` yeniden geçti.
- Görev 9'a göre workspace ve e2e +2, iki platformda da.

## CI düzeltmesi: test düzeneği süreç ortamını okuyordu

`6acab8f`'nin Windows CI'ında `mcp_komutlari`'nın iki testi "MCP tanımı
doğrulanamadı" ile düştü. CI testleri `kalfatest` kullanıcısıyla, yönetici
oturumunun PATH'iyle koşar. `McpTanim::hazirla` PATH'in her dizinini çözer;
okunamayan dizin (`runneradmin` profili) bilerek ret verir. Test uygulaması
ise MCP yöneticisine gerçek süreç ortamını veriyordu. Ürün davranışı
değişmedi: `test_destek` artık yöneticiye yalnız `PATH=<veri dizini>` verir.
WSL'de `kalfatest` için okunamayan `/root/.cargo/bin` PATH'e eklenince
düzeltmeden önce 0/2, sonra 2/2.
