# Kalfa 5b — Oturum bütçesi: Uygulama planı

**Durum:** Claude incelemesi ve kullanıcı kararları işlendi (12 Ekim).
Ürün kodunu uygulama yetkisi değildir; uygulama AGENTS'taki planlı devirle başlar.
**Taban:** `3133248`; [tasarım](../specs/2026-10-12-kalfa-5b-oturum-butcesi-design.md).
**Hedef:** Her üretim denemesinin oturum bütçesinden izin alması, kullanımın
istek kimliğiyle sayılması, limit/eksik kullanımda kullanıcı kararının
beklenmesi ve kurtarmanın sayaçları koruması.

## Global Constraints

- Yeni crate/npm paketi, tokenizer, ağ ucu, WIT değişikliği veya `unsafe` yok.
- Üretimde `unwrap`/`expect` yok; Türkçe adlar/metinler, mevcut commit önekleri.
- Kayıt/izin sahibi tek `Oturum` görevidir; yardımcılar JSONL'ye yazmaz.
- İzin başlangıcı ve kullanıcı devam kararı `yaz_kalici` ile yazılmadan
  yeni ağ denemesi başlatılmaz. Kayıt hatası kapıyı kapatır.
- İstek başına kimlik; her 429/bağlam/şart sonrası tekrar yeni denemedir.
- Kullanıcı kararı: kullanım eksikse yeni denemeden önce dur ve sor.
  Eksiklik 0 sayılmaz; bilinmeyeni kabul etmek geçmişi sıfırlamaz.
- Kullanıcı kararı: tasarım §2'deki üretim öncesi ret hataları istek
  sayılır, tokenı bilinen 0'dır ve soru sordurmaz. Liste dışındaki
  hatalar bilinmeyendir; listeyi genişletmeyin.
- Başlamış istekler kayda girer; yeni izin reddi onların sonucunu düşürmez.
- Önizleme reddi, boş denetçi ve yerel gövde/model/izin reddi istek sayılmaz.
  Başlatma izni verilmiş bağlantı hatası/iptal deneme sayısını geri almaz.
- Şart/model kataloğu kontrolleri hariç; şart kabul ucu bu çalışma sırasında
  çağrılmaz. Canlı testler, paket ve sistem kurulumu ayrıca yetki ister.
- Mevcut yalıtım, gizli dosya, denetçi ve eklenti sınırları korunur.
- Bekleyen eklentinin bütçe devamı bütün aracı yeniden çalıştırmaz.
- Varsayılan 200 istek/5 000 000 bilinen token (kullanıcı kararı, 12 Ekim).
  0/sınırsız seçenek eklenmez.
- Şema 1, eski kaydı okuma ve isteğe bağlı alanların serde varsayılanları
  korunur; eski sürümün yeni kayıtları okuyabildiği vaat edilmez.
- Prototip yalnız kabul modelini doğrular; asenkron/IPC entegrasyonunu
  doğrulamış gibi raporlamayın. Her görevde ilgili test/clippy, sonunda
  AGENTS'taki iki platform komutları gerekir. `npx prettier` kullanılmaz.

## Dosya ve sözleşme haritası

| Dosya | Değişiklik |
|---|---|
| `kalfa-core/src/butce.rs` (yeni) | Saf sayaç, limit, görünüm ve oturuma bağlı bütçe kanalı |
| `kalfa-core/src/lib.rs` | Bütçe tiplerinin dışa açılması |
| `kalfa-core/src/oturum/mod.rs` | Durum/kanal, kalıcı izin ve karar, bekleme döngülerinin ortak işlemesi |
| `kalfa-core/src/oturum/model_turu.rs` | Her ana deneme ve akışın tek sonlandırması |
| `kalfa-core/src/denetci.rs` | Her iç tekrar için kanal izni/sonucu; trait'e oturum bağlamı |
| `kalfa-core/src/oturum/baglam.rs` | Her özet denemesi; kapalı kapıda yeni başlatmama, başlamışları kaydetme |
| `kalfa-core/src/arac.rs`, `oturum/arac_asamasi.rs` | Eklenti izin kimliği/kullanımı ve bütçe kanalını işleme |
| `kalfa-core/src/kayit.rs`, `kurtarma.rs`, `gonderim.rs`, `tur_ozeti.rs` | Kimlikli kayıt, eski kayıt devri, tek muhasebe ve günlük eşlemesi |
| `kalfa-core/src/protokol.rs` | Bütçe görünümü, karar olayı/komutu |
| `kalfa-evren/src/tipler.rs`, `sse.rs`, `istemci.rs` | Kullanım alanlarının varlığı ve geçerli toplam bilgisini koruma; gerçek dosya ayrıştırma noktalarını doğrulayın |
| `kalfa-plugin/src/host.rs` | Ham kullanım okuma, aynı izin future'ı ve kimlikli bitiş |
| `kalfa-uygulama/src/ayarlar.rs`, `oturumlar.rs`, `kopru.rs`, `sozlesme.rs` | Varsayılanlar, yeni/kurtarılan görünüm ve karar yönlendirme |
| `kalfa/src/komutlar.rs`, ana pencere yetki dosyaları | Mevcut ana pencere kapsamındaki IPC yolları |
| `ui/src/ipc/ana.ts`, `tipler.ts`, `durum/oturum.ts`, `ana/anaDurum.svelte.ts` | Görünüm/karar/ayar türleri; mevcut gerçek dosya adlarını koruyun |
| `ui/src/ana/ButceKarti.svelte` (yeni), `Ayarlar.svelte`, `dil/tr.ts` | Sayaç, karar kartı ve yeni oturum varsayılanları |
| Çekirdek/uygulama/eklenti/e2e test destekleri | Geçerli sahte kullanım, bilinmeyen kullanım için açık fixture |

Bu tabloda crate yolları `crates/` altında kısaltılmıştır. Bir dosya adı
tabanla uyuşmuyorsa önce gerçek konumu belirleyin; ilgisiz yeniden adlandırma
yapmayın. Yeni kanalın istek modeli tasarım §4–7'de sabittir.

## Görev 1 — Saf bütçe durumu ve kalıcı kayıtlar

1. Önce `oturum_butcesi` birim testlerini yazın: sıfır limit, sınırdaki son
   izin, paralel izinler, tokenın sonradan aşması, eksik kullanım, stale
   karar, çift sonuç, taşma. Prototipteki gerçek kanal yarışını taşıyın.
2. `ButceSinirlari`, `ButceDurumu`, `ButceGorunumu` ve `ButceEngeli`
   tiplerini ekleyin. Sınırsız kip/ayar veya atomik sayaç kopyaları eklemeyin.
3. Tasarım §6 kayıtlarını ekleyin. Başlangıç/sonuç `IslemId` ile bağlanır;
   `Istek`/`EklentiIstegi`'nde `butce_islem: Option<IslemId>` eski kayıtta
   `None` olur. Başlatma/karar kalıcı yazılır, sonra durum uygulanır.
4. Sahte `KayitHedefi` ile hem yazma hem kalıcılaştırma hatası üretin;
   sağlayıcının sayaçlanan ağ çağrısı **0** kalmalı. Bu güvenlik testini
   sadece "hata döndü" olarak yazmayın.
5. Çift/bilinmeyen bitiş kimliğini canlıda hata olarak kapatın; kurtarmada
   tipli hata döndürün. Yeni kayıtların serde dönüşlerini sınayın.

Doğrulama: `cargo test -p kalfa-core --locked`, ilgili clippy/fmt.

## Görev 2 — Kullanım bilgisini kaybetmeden normalize etme

1. Gömme/sıralama ham fixture'larını salt okunur kullanın. Bilinen fixture
   toplamını hesaplayın; vektör veya ham yanıtları yeni belgede çoğaltmayın.
2. Sohbet kullanımında mevcut `Usage` sayısal alanlarını koruyun; alan
   varlığını custom `Deserialize` ile özel ham tipe (`Option<u64>`)
   taşıyıp bütçe için `Option<u64>` geçerli toplam üretin. Serileştirilen
   EVREN JSON'una yeni metadata alanı eklenmez. Rust test kurucuları yeni
   alan-varlığı bilgisini açıkça kurar. Boş kullanım 0'a dönüştürülemez.
3. Gömme/sıralama için aynı doğrulama kurallarıyla küçük kullanım çözücü
   yazın: istem + varsa tamamlama; yalnız toplam; boş/negatif/taşmış/
   çelişkili alanlar. Yeni bağımlılık yoktur. Hatalı yanıtın mevcut hata
   yolunu koruyun; bütçe tüketimi bilinmeyen olur.
4. `EvrenError`'dan bütçe kullanımına tek saf işlev yazın: tasarım §2'deki
   üretim öncesi ret sınıfı `Some(0, 0)`, diğerleri `None`. Her varyant
   için tablo testi; `Sunucu` için 4xx/5xx sınırı ve `Ag`'ın iki
   `govde_gitti` değeri ayrı sınanır.
5. `reasoning_tokens` ikinci kez sayılmamalı. Akışta yinelenen kullanım
   tek toplam olarak sonlandırılmalı; kullanım olmadan bitiş/iptal test
   edilmelidir. Kredi bilgisi kullanımın yerine geçmez.

Dosyalar: `kalfa-evren` tipler/ayrıştırma noktaları ve testleri; `kalfa-plugin`
host kullanım yardımcısı. Doğrulama: iki crate'in test/clippy'si.

## Görev 3 — Oturum kanalı ve ana akış

1. Bütçe gönderici/alıcı çiftini her `Oturum` için ayrı oluşturun. Başlatma
   kuyruğu sınırlı olsun. Başlangıç kaydı sonrası sahipli, kopyalanamayan
   `ButceIzni` verin; tek `oneshot` bitiş alıcısı çekirdekte izlenir.
   İzin alıcısı/future kapanması sonuç tüketimini kaybettirmesin.
   `Drop` içinde async görev veya kaybolabilen `try_send` kullanmayın.
2. Her `akis_ac` iterasyonunda, sağlayıcı future'ı poll edilmeden önce
   izin alın. 429/bağlam/şart tekrarlarının her biri yeni kimlik alır.
3. Açılan akışın kimliğini `model_turu` boyunca tutun. Başarılı bitiş,
   hata, yarım akış, kullanıcı iptali ve kayıt hatasında tek sonlandırma
   yapın. Kullanım yoksa belirsizlik kaydedilir.
4. Bütçe beklerken komut/iptal ve bütçe sonuç kanalı dinlenir; çekirdek
   kendi isteğinin cevabını işlerken kilitlenmez. Yeni kullanıcı mesajı
   ikinci tur başlatamaz. Bütçe kartının güncel kimliği kontrol edilir.
5. İzin sonucu yazımı/send yarışı ve çökme aralıklarını sınayın. Başlatma
   kaydı var, ağın başladığı bilinmiyorsa istek konservatif olarak sayılır.

Kabul: sahte sağlayıcıda 1 istek limitinde ikinci `akis` çağrısı yok;
ikinci tur ve kayıttan açma sıfırlamaz; eksik kullanımda kullanıcı kararından
önce yeni `akis`/özet isteği yok; iptal beklemeyi sonlandırır. 429 ve
`BaglamAsimi` sonrası tek tekrar soru sormadan sürer, her ikisi de istek
sayılır; 5a'nın sıkı özetleme regresyonları değişmeden geçer.

## Görev 4 — Denetçi ve özetleyicinin her denemesi

1. `GuvenlikDenetcisi::denetle` oturuma bağlı bütçe bağlamını alır.
   Yapılandırılmış denetçi gerçek `sohbet` denemelerinden önce izin ister;
   `Yok` bunu yapmaz. Mevcut sahte trait uygulamalarını açıkça uyarlayın.
2. `sohbet_ve_yeniden_dene` ve `ozetle` içindeki ilk/tekrar denemesi ayrı
   kimlikli sonuç verir. Final sınıflandırma/özet kaydı bu denemeleri
   yeniden saymaz. Gönderim günlüğü iki denemeyi iki satır gösterir.
3. Bütçe beklemesi 30 sn ağ bütçesini tüketmesin; ağ ve retry beklemesinin
   mevcut etkin toplam zaman sınırı artmasın. İptal her bekleme noktasında
   çalışır. Bütçe nedeniyle bekleme sunucu hatası/boş özet sayılmaz.
4. Dört özet başladıktan sonra biri kullanım vermeden biterse beşinci
   gönderilmez; diğer üç istek kayda girer. Kullanıcı devam ederse kalan
   adaylar bir kez sürer. `f9ee299` regresyonu korunur.
5. Denetçi yanıtının anlamı bütçe devamıyla değişmez; katı kipte
   denetlenmemiş çıktıyı güvenli sayan kısayol eklenmez.

Testler: `denetci_butcesi`, `baglam_butcesi`, mevcut bağlam/denetçi regresyonları.

## Görev 5 — Eklenti izin/kullanım aktarımı

1. `AracOrtami`, o oturumun bütçe göndericisini taşır. Oturumsuz test ortamı
   ayrı, açık bir kullanım olmalıdır; ürün oturumunda bütçe bağlamı
   eksikse sınır sessizce kaldırılmaz.
2. Mevcut gövde/manifest/şart/katalog/ilk gönderim/önizleme denetiminden
   sonra, `ek_istegi`'nden önce bütçe izni alın. Kimlik bitişe taşınır.
3. Ham yanıtın kullanımını §Görev 2 ile çözün. Sahipli izin bitişinde
   `butce_islem`, `token`, meta ve sonuç birlikte taşınır; eski ortam
   bitişi aynı isteği ayrıca göndermez. Eklenti günlük
   satırı ve tur özeti bu kullanımı bir kez gösterir.
4. Araç aşamasının bütün bekleme/drain yolları bütçe taleplerini işler.
   Bekleyen bütçe isteği eklentiye `SinirAsildi`/429 dönmez; aynı future
   devam eder. RAG toplu daraltma davranışı değişmez.
5. Araç yakıt/süre/iptal sınırı aşılırsa bekleyen izin temizlenir. Geç
   karar provider'ı açamaz. Bir eklenti önce dosya yazmışsa bütçe devamı
   bu yazmayı tekrar etmez: testte yazma/araç yürütme sayacı **1** olmalı.

Kabul: iki paralel eklenti + denetçi/özet aynı sayacı paylaşır; iki ayrı
oturum paylaşmaz; önizlemede ret tüketmez; gömme + sıralama toplamı doğru;
IPTAL/ZAMAN AŞIMI/SINIR yolları kayıt ve izin future'ı bırakmaz. WIT aynı.

## Görev 6 — Kurtarma ve günlük

1. Yeni kayıtları saf bütçe reducer'ıyla oynatın. Sonuçsuz başlangıç
   bilinmeyen sayılır; kullanıcı kararı/yeni izin öncesinde bu kimliklerin
   `ButceIstegiBitti { kullanim: None }` kayıtlarını kalıcı tamamlayın.
   Tekrar kurtarmada bu bitişler çift sayılmaz; karar oynatılırken aktif
   bırakılmış eski kimlik yoktur. İki kurtarma yeni tüketim oluşturmaz.
2. Eski `Istek`/`EklentiIstegi`/`Kullanim` fixture'larından bilinen alt
   toplamı çıkarın. Gönderilmeyen satırları saymayın. İç tekrar/eksik
   token belirsizliğini ilk yeni istekte sorun; geçişi bir kez kaydedin.
3. Yeni satır kullanımını `butce_islem` ile eşleyin; iç içe eklenti/denetçi
   bitişi komşu isteğin tokenını değiştirmesin. Eski günlük görünümü açılır.
4. Başlangıç görünümü (`KurtarilmisDurum` → uygulama sözleşmesi → UI)
   sayaç ve son limitleri taşır. Son kullanıcı kararının onaylanan
   bilinmeyen sayısı geri gelir; yeni eksiklik yeniden sorulur.
5. Kimliksiz/çift/çelişkili yeni kayıtları reddedin. Sonrası kesilmiş JSONL
   ve yazma/kalıcılaştırma hatalarını mevcut kurtarma testlerine katın.

## Görev 7 — Ayarlar, karar ve arayüz

1. `ayarlar.json`'da yeni oturum varsayılanları; eski/bozuk/sıfır değer
   testleri. Açık/kurtarılan oturumun limiti genel ayarla değişmesin.
2. `ButceDurumu`/`ButceKarariGerekli` olayları, `ButceKarari` komutu ve
   uygulama/Tauri ana pencere yollarını ekleyin. Yanlış oturum/eski işlem
   kimliği ve etkin istek sırasında karar reddedilir.
3. Kartta bilinen tüketim, eksiklik ve iki sonlu mutlak limit gösterin.
   Devam kararını kalıcı yazmadan kuyruktaki izni açmayın. Devam sayaçları
   sıfırlamasın; durdurma bütün mevcut iptal yollarına bağlansın.
4. Türkçe metinler `tr.ts`; ham HTML/sabit inline style yok. Klavye,
   ekran okuyucu adı, odak, hata metni ve bekleyen kararın yeniden
   bağlanınca görünmesi sınanır. Yeni tasarım paketi eklenmez.
5. Eski test sahteleri başarılı cevaplarda açık, geçerli kullanım üretir;
   kullanım eksikliği testleri ayrı `None` fixture kullanır. Testleri
   geçsin diye ürün kapısını kapatan bir global bypass eklenmez.

Vitest: istek/token/eksik kullanım kartı, hata girdisi, yanlış/stale karar,
iptal ve yeniden açma. Uygulama testi: aynı oturum, kayıtlı limitler ve
ana pencere yetki listesi. e2e: 1 istek → soru → mutlak limit yükselt →
aynı iş; eksik kullanım → durdur; eski kayıt → soru. Canlı API gerekmez.

## Görev 8 — Son inceleme, iki platform ve devir

- Her kabul maddesini ağ/araç sayaçları ve kayıt fixture'ıyla karşılaştırın.
  Mutlu yol testi güvenlik sınırlarının yerine geçmez.
- Windows ve WSL `kalfatest`: AGENTS'taki workspace test, düz/e2e/wasm
  clippy, fmt, e2e özellikli testler; UI check 0 hata/uyarı, test/build.
- Eklenti host/araç akışı değiştiği için mevcut eklenti/sınır testleri
  koşar; değişen yeni kullanıcı akışı iki platformda gerçek e2e ile
  doğrulanır. Mevcut paketler yoksa kurulum onayı olmadan indirmeyin.
- Canlı EVREN gerektirmez. Şart kabul çağrısı, API anahtarı çıktısı ve
  kurulum/sistem değişikliği yoktur.
- Geliştirme durumu, bu tasarımın sonuç bölümü, PR açıklaması ve yerel
  devir güncellenir. Yeşil CI ve ayrıca açık kullanıcı merge onayı gerekir.

## Ön doğrulama kanıtı

12 Ekim: `prototip/oturum-butcesi/cekirdek.rs`, Rust 1.98.1 ile Windows ve
WSL'de 11 test geçti; [rapor](../bulgular/2026-10-12-kalfa-5b-butce-on-dogrulama.md).
Bu planın async/serde/IPC değişiklikleri henüz uygulanmadı ve derlenmedi.
Uygulayıcı yukarıdaki regresyonları önce düşürüp sonra geçirmelidir.
