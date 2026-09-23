# Tasarım kaynakları

## `kalfa-ds/` — Kalfa Design System (anlık kopya)

Kaynak: Claude Design'daki "Kalfa Design System" projesi
(<https://claude.ai/design/p/c5b90314-83d0-4499-b93e-542d2659de8f>).
Kopyalanma tarihi: 2026-09-22. Dosyalar içerik değiştirilmeden alındı.

Bu klasör **referanstır, uygulama kodu değildir.** Bileşenler React (`.jsx`)
ile yazılmış; Kalfa'nın arayüzü Svelte 5 ile yazılacak. 1c'de nasıl
kullanılacağı `docs/GELISTIRME_DURUMU.md`'deki "Verilen kararlar" bölümünde.
Kısaca: tokenlar (`tokens/*.css`) aynen kullanılır, bileşenler `.d.ts` ve
`.prompt.md` dosyalarına bakılarak Svelte'te yeniden yazılır.

Claude Design'daki proje sonradan değişirse bu kopya kendiliğinden
güncellenmez; yeniden çekmek gerekir.

### Kopyada olmayanlar

| Dosya | Neden |
|---|---|
| `_ds_bundle.js` | Bileşenlerin derlenmiş React paketi; kaynaklardan üretilir ve uygulamaya girmeyecek. Bu yüzden `*.card.html` önizlemeleri ve `ui_kits/desktop/index.html` bu kopyada doğrudan açılmaz; görmek için Claude Design'daki projeyi açın. |
| `.thumbnail` | `thumbnail.html`'nin küçük önizleme görseli (webp). |
| `refs/kalfa-ui-*.png` (4 dosya) | Repoda zaten `assets/generated/` altında. |
| `refs/crop-*.png` (4 dosya) | Aynı taslaklardan kesilmiş parçalar. |
| `refs/shot-01.png` … `shot-08.png` | EVREN portalının ekran görüntüleri; üçüncü tarafa ait. Repoya alınmaz (`.gitignore`'daki `Screenshot*.png` kuralıyla aynı gerekçe). |

### Spec ile çelişen içerikler

Tasarım sistemi taslak görsellerden üretildi; spec'i okuyarak değil. Uygularken
şu noktalarda spec geçerlidir:

- **Kaynak açıklaması hatalı.** `readme.md`, `Screenshot 2026-09-21 *.png`
  dosyalarını arayüz taslakları olarak anıyor. Bunlar EVREN portalının ekran
  görüntüleri. Kalfa'nın kendi taslakları `assets/generated/*.png`.
- **Şart ekranı.** Prototipteki `OnboardingScreen`, Kalfa'nın kendi yazdığı
  maddeleri ve bir onay kutusunu gösteriyor. Spec §11.8'e göre şart penceresi
  EVREN'in `/v1/terms/text` metnini özetlemeden, olduğu gibi gösterir. Bu ekran
  yalnızca görsel referanstır.
- **Onay kapsamı.** Onay kartındaki "Bu proje için" seçeneği spec'te yok.
  §7.6'ya göre seçenekler "yalnızca bu çağrı" ve "bu oturumda bu dizin"; 1c'de
  yalnızca ilki var.
- **Dış kaynaklar.** Fontlar Google Fonts'tan, ikonlar unpkg CDN'inden
  yükleniyor. §11.8 ve §11.9 buna izin vermez; uygulamada ikisi de paketlenir.
- **Sonraki aşamalara ait ekranlar.** Geri alma, gönderim günlüğü, eklentiler ve
  sağdaki dosya paneli 1c'nin kapsamında değil.

### Kopyadaki hata ve uygulamadaki değişiklikler

Tokenlar `ui/src/stiller/`'e kopyalanırken (1c-3a) iki yer değişti:

- **`--text-title` çakışması (hata).** `tokens/colors.css` bu adı başlık rengi,
  `tokens/typography.css` başlık yazı boyutu (`17px`) için kullanıyor. Sonra
  yüklenen boyut rengi eziyor; koyu temada başlık rengi çözülemiyor ve rengi
  kendisi vermeyen öğelerde başlık siyah görünüyor. Uygulamadaki kopyada boyut
  `--text-title-size` adını aldı; `ui/src/stiller/tokenlar.test.ts` aynı adın iki
  kez tanımlanmasını yakalar. Claude Design'daki projede de düzeltilmesi gerekir.
- **Font aileleri.** Fontlar `@fontsource-variable` paketlerinden geliyor. Paketler
  aileleri "Source Sans 3 Variable" gibi adlarla tanımladığı için kopyada bu adlar
  önce geliyor.
