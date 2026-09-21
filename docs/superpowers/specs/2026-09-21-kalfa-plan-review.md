# Kalfa uygulama planı — düzeltme ve doğrulama kaydı

Tarih: 2026-09-21 · Tasarım revizyonu: r3.2

Kapsam: [1a EVREN istemcisi](../plans/2026-09-21-kalfa-1a-evren-istemcisi.md),
[1b çekirdek ve dosya araçları](../plans/2026-09-21-kalfa-1b-cekirdek.md),
[tasarım](2026-09-21-kalfa-design.md).

Bu çalışma Claude'un yarım bıraktığı plan düzeltmelerini tamamlar. Depoda
henüz uygulama crate'leri/Cargo.toml yoktur. Aşağıdaki değişiklikler Markdown
içindeki uygulanacak Rust örneklerindedir; çalışan uygulamada doğrulanmış
güvenlik düzeltmesi veya geçmiş test raporu değildir. Önceki değişiklikler
korundu; ekran görüntüsü `tasks.png` değişiklik kapsamına alınmadı.

## İşlenen düzeltmeler

1. **Kayıt hatasında güvenli duruş — 1b Task 6, 8, 9, 12.**
   `KayitHedefi` ile ekleme ve kalıcılaştırma hatası üretilebiliyor. Kayıt
   başarısızlığı kalıcı oturum durumudur; otomatik kayıtsız moda geçiş yoktur.
   Hata iptal belirtecine yayılır. Bir paralel grubun ikinci başlangıç kaydı
   bozulursa ilk kaydı başarılı ama başlamamış araç da çalıştırılmaz. Sonuç
   kaydı bozulduğunda kilitte bekleyen araç da durur. Son `TurBitti` kaydındaki
   hata kullanıcıya `Normal` tamamlanma olarak sunulmaz. Başlamış yan etkiler
   geri alınmış sayılmaz; belirsizlik açıkça belirtilir.

2. **JSONL onarımı — 1b Task 6, 12.**
   Yarım son satır yalnızca okurken atlanmıyor; devam için açarken son geçerli
   bayt sınırına kadar kesiliyor. Geçerli kaydın eksik satır sonu tamamlanıyor,
   onarım kalıcılaştırılıyor. Kurtar → yaz → tekrar aç senaryosu eklendi.
   Ortadaki bozuk kayıt hata olmaya devam ediyor.

3. **İptal — 1a Task 6, 1b Task 8–11.**
   Sağlayıcı görevi alıcının kapanmasını ayrıca bekliyor; sessiz/keep-alive
   akışında yeni bir olay göndermeye ihtiyaç duymuyor. TCP testi istek başlığını
   ve gövdesini bütünüyle tüketiyor; tek `read` çağrısını tam HTTP isteği
   varsaymıyor. Oturum tutamacı iptal belirtecini doğrudan tetikliyor; dolu
   komut kuyruğu veya olayları okumayan UI iptalin tek yolu değil. Yan etkisiz
   hazırlık ve kilit beklemeleri de belirteci dinliyor.

4. **Ortak kilitler — 1b Task 8, 9, 15.**
   `OturumBaglami` çalışma alanının ortak `Arc<KaynakKilitleri>` nesnesini
   alıyor. İki oturumun aynı dosyaya seri yazması için test eklendi; canlı
   uçtan uca testin bağlam kurucusundaki eksik `kilitler` alanı tamamlandı.
   Uygulama katmanı aynı alanın farklı yol yazımlarını tek kimlikte toplamalı.

5. **Yürütme anında izin denetimi — 1b Task 3, 9.**
   Kip değişimi bekleyen onayları geçersiz kılıyor. Kilit alındığında ve
   asenkron hazırlık döndüğünde politika sürümü tekrar kontrol ediliyor.
   Güncel hedef sınıflandırması/izin kararı da eski kararla karşılaştırılıyor;
   yalnızca kip sayacına güvenilmiyor. Hard link sayısı mühre eklendi.
   Hazırlık sırasında kip değişimi testi `Notify` bariyerleriyle kuruldu.

6. **Bağlantı takma adları — 1b Task 13–15, tasarım §7.3.**
   v1'de symlink/junction izlemek reddediliyor; hedefin çalışma alanının içinde
   olması istisna değil. Bu bilinçli bir işlev kısıtıdır: gerçek dosya yolu
   kullanılmalıdır. Üst dizinler tek bileşenlik no-follow açılışlarla yürünüyor,
   son dosya da no-follow açılıyor. Okuma, önceki içerik özeti, hard link sayısı
   ve yazma aynı yardımcıları kullanıyor. `.env` okuma takma adı, `.git/hooks`
   yazma takma adı, hazırlıktan sonra bağlantı konması, Windows junction ve
   Serbest kipte gerçek araçlarla uçtan uca senaryolar eklendi.

## Statik doğrulama ve uygulama beklentileri

Planlardaki test tanımları sayıldı; değişen `Expected` satırları güncellendi.
Bunlar **çalıştırılan test sonuçları değil**, uygulama sırasında beklenen
sonuçlardır:

| Test dosyası | Linux | Windows |
| --- | ---: | ---: |
| 1a `istemci` (Task 5 + 6) | 11 | 11 |
| 1b `kayit_kurtarma` | 6 | 6 |
| 1b `oturum_metin` | 8 | 8 |
| 1b `oturum_arac` | 17 | 17 |
| 1b `oturum_iptal_sinir` | 13 | 13 |
| 1b `oturum_sart` | 4 | 4 |
| 1b `oturum_kurtarma` | 3 | 3 |
| `okuma_araclari` | 10 | 10 |
| `yazma_araclari` | 11 | 12 |
| `uctan_uca` | 3 + 1 canlı/ignored | 3 + 1 canlı/ignored |

Windows symlink testleri yetki/Developer Mode gerektirir. Yetki yoksa test
başarılı sayılmamalı veya sessizce atlanmamalıdır. Junction testi Windows'a
özeldir. Uygulamada her iki platformda `cargo test`, `cargo clippy` ve
`cargo fmt --check` çalıştırılmalıdır.

Bu ortamda `cargo`, `rustc`, `rustfmt` bulunamadığı ve uygulama kaynakları
henüz olmadığı için derleme, lint, biçim ve çalışma zamanı testleri
**çalıştırılmadı**. Canlı EVREN isteği yapılmadı; `.env` okunmadı. Markdown kod
çitleri, doküman bağlantıları, test sayımları ve `git diff --check` statik
kontrol kapsamındadır.

## Açık sınırlar / sonraki aşamanın kabul kapıları

- **Olay kuyruğu:** Claude'un eklediği sınırsız çekirdek → uygulama kanalı
  iptal kilitlenmesini giderir ama bellek tüketimini sınırlamaz. 1c'de bayt ve
  olay bütçesi, birleştirme, yavaş/kopmuş UI davranışı ve bütçe aşımında
  kullanıcıya bildirilen duruş zorunludur. Yalnızca 10.000 olayla iptal testi
  üretim için sabit bellek garantisi değildir.
- **Sahiplik:** ortak kilit nesnesi aynı süreçteki oturumları kapsar. Uygulama
  aynı çalışma alanı için tek yönetici, aynı kayıt için tek yazıcı sağlamalı;
  birden çok uygulama süreci desteklenecekse ayrıca süreçler arası sahiplik
  tasarlanmalıdır.
- **Dosya sistemi yarışları:** no-follow, bağlantı üzerinden yönlendirme
  sorununu hedefler. İçerik hash kontrolü + sonraki yazma atomik bir
  compare-and-swap değildir; dış süreçlerin eşzamanlı normal dosya değişimleri
  ve hard link nesne kimliği ayrı sınırlardır. Bu revizyon bütün yerel yarışları
  veya her Windows reparse türünü çözdüğünü iddia etmez.

## API dayanakları

No-follow davranışı son yol bileşenine uygulanır; bu yüzden üst zincirin
bileşen bileşen açılması gerekir. Kaynaklar:
[DirExt::open_dir_nofollow](https://docs.rs/cap-fs-ext/latest/cap_fs_ext/trait.DirExt.html#tymethod.open_dir_nofollow),
[OpenOptionsFollowExt kaynak sözleşmesi](https://github.com/bytecodealliance/cap-std/blob/main/cap-fs-ext/src/open_options_follow_ext.rs).

Akış görevinin alıcı bırakılınca uyanması için kullanılan API:
[Tokio Sender::closed](https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.Sender.html#method.closed).
