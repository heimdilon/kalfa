# kaynaklı web getirme ve MCP arama planı

**Tarih:** 2026-10-20. **Planlayan/inceleyen:** Codex. **Uygulayıcı:** Antigravity (`Antigravity-Hat-05`).
**Durum:** G4a (`web_getir`, açık `WebTarifi`/`VeriCikisi::WebSayfasi` onay/mühür/kayıt sözleşmesi [SP-05], kayıpsız UTF-8 bayt ofsetli devam imleci [SP-07], HTML/metin çıkarımı, sızıntı önleme, UI kaynak kartı ve `varsayilan_araclar`/oturum/IPC entegrasyonu [SP-08]) ile G4b yerel/Uzak MCP (`Hat04` `arac_ara` + `UzakAracKaynagi`) arama-kaynak-yanıt sözleşmesi (`W01`–`W06`) Windows + WSL Ubuntu 24.04 üzerinde tamamlandı; canlı dış arama sağlayıcısı denemesi kullanıcı sağlayıcı/hesap/anahtar kararını bekliyor.
**Plan tabanı:** main `4041a39995e3bea3d56515e53ffc8af47f68e1ae`. **Çalışma dalı:** `codex/plan-yetenek-web`.
**İlk başlanacak iş:** Önce ağ/izin sözleşmesini ve yerel HTTP fixture'larını hazırlayın. Arama sağlayıcısı kararı sayfa getirme çalışmalarını bekletmez.
**Koordinasyon:** [iş sırası ve ortak dosyalar](https://github.com/heimdilon/kalfa/blob/codex/plan-yetenek-koordinasyon/docs/superpowers/specs/2026-10-20-kalfa-yetenek-programi.md).

## Global Constraints

- **21 Ekim rol devri:** Kullanıcının “sen halletsene” talimatıyla kalan inceleme düzeltmelerini ve çevrimdışı kabulü Codex üstlenir. Önceki Antigravity uygulama rolü bu dar kapsamda devredildi; kurulum, canlı deney ve merge sınırları değişmedi.

- Kullanıcının 20 Ekim talimatı: Codex plan ve PR hazırlar; Antigravity uygular; kullanıcı haber verince Codex inceler. Bu belge ürün kodu veya derlenmiş örnek içermez; ön doğrulama yapılmış gibi sunulamaz.
- İlk teslimat yerel kodlama + web + MCP, Windows ve Linux'tur. EVREN, Türkçe ad/metin, bağlı onay, yalıtım, sır koruması, bütçe ve JSONL kurtarma korunur.
- Başlamadan güncel AGENTS.md, GELISTIRME_DURUMU, Jira sahipliği ve PR başını okuyun. Her hat ayrı worktree'dedir; aynı klasöre iki uygulayıcı girmez.
- İlk görevde planın varsayımlarını ve gerçek SDK/platform arayüzlerini doğrulayın. Yeni paket/indirme/sistem ayarı AGENTS iznine tabidir; planda adı geçmesi kurulum onayı değildir. Yeni unsafe gerekirse dosya listesi ve gerekçe uygulamadan önce tasarımda kesinleşir.
- Hazırlık → izin/onay → taze mühür/kilit → yürütme yolu korunur. Yeni işlem türü mevcut Otomatik kip izin listesine kendiliğinden eklenmez. Yalıtımlı iş sessizce tam erişime düşmez.
- Sonucu bilinmeyen yan etkili iş tekrar yürütülmez. İş başlatıldı kaydı, iş başarıyla bitti kaydı yerine geçmez.
- Üretim kontrolleri AGENTS.md'deki kapsamla iki platformda koşar. CI gerçek yalıtım ve WebDriver yerine geçmez. Canlı EVREN/arama servisi denemesi ve merge mevcut ayrı yetki kurallarına tabidir.
- Her görev: davranışı ölçen düşen test/deney → küçük uygulama → geçen test → ilgili diff incelemesi. Plandan sapma gerekiyorsa gerekçeyi PR'a yazın; güvenlik kuralını sessizce gevşetmeyin.
- Ortak dosyaları değiştirmeden kendi PR'ınıza dar bir sözleşme notu yazın: yeni alan/olay, kaynak sahibi, kayıt/kurtarma ve hata davranışı. Koordinasyon planındaki sıra uygulanır; diğer hattın bekleyen tiplerini uydurmayın.

## Amaç ve sınır

G4a kaynaklı metin/HTML getirme → G4b seçilmiş tek arama MCP'siyle arama ve kaynağı açma. JS tarayıcısı, giriş çerezi, genel scraping/crawl, otomatik içerik indirme ve özel ağ gezintisi yoktur. Web araması gerçek sağlayıcıyla kanıtlanmadan tamam sayılmaz.

## Dosya sahipliği

| Yol | Sorumluluk |
|---|---|
| `crates/kalfa-tools/src/web/` (yeni) | Getirme, sınırlar, metin ve kaynak dönüşümü |
| `crates/kalfa-tools/tests/web_getir.rs` (yeni) | Gerçek bağlantı ve içerik sınırları |
| `crates/kalfa-tools/tests/web_fixture/` (yeni) | Sentetik sayfa/redirect/DNS fixture'ları |
| `docs/superpowers/bulgular/yetenek-programi/web-kararlari.md` | İzin politikası, parser/sağlayıcı seçimi ve kanıt |
| `ui/src/bilesenler/WebKaynak*.svelte` (gerekirse yeni) | Kaynak/eksik içerik gösterimi |

`kalfa-tools/Cargo.toml/lib.rs`, core Hazirlik/izin/mühür/çıktı ve uygulama araç kaydı ortak dosyadır. Ürün kaydı/IPC entegrasyonu G1 → G2 → G3 → Web sırasındadır. Yerel fixture ve özel web modülü bağımsız hazırlanabilir. Sandbox vekili süreç hattınındır; adres yardımcıları değiştirilmeden önce o hattın sözleşmesi kontrol edilir.

## Davranış sözleşmesi

- `web_getir` girdisi URL ve sınırlı bölüm seçimi; çıktı asıl/son URL, başlık, kaynak etiketi, getirilen metin ve açık kırpılma/devam bilgisidir.
- Genel ürün yolu yalnız HTTPS; HTTP yalnız yerel test fixture'ında açık ayrı ayarla çalışır. Kullanıcı bilgili URL ve HTTP'ye yönlendirme reddedilir. Kullanıcının özel ağını web aracıyla tarama bu sürümde yoktur.
- Scheme/host/port/çözülen IP ve gerçekten bağlanılan IP kontrol edilir; her redirect aynı denetimden geçer. DNS rebinding, IPv4 eşlenmiş IPv6, loopback/link-local/metadata/özel ağ ve proxy üzerinden atlama test edilir.
- Önerilen üst sınırlar: 5 redirect, 30 sn toplam ağ süresi, açılmış gövdede 4 MiB, model yanıtında 64 KiB. Ham/sıkıştırılmış gövde ve başlıklar ayrıca sınırlanır; sayılar ilk deneyde CPU/bellekle doğrulanır.
- TLS doğrulaması kapanmaz. Host dışına Authorization/cookie taşınmaz; makinenin oturum çerezleri alınmaz. Sistem proxy'si izin denetimini atlatamaz.
- Ana süreçten çıkan reqwest isteği kabuk yalıtımıyla korunuyor varsayılmaz. HTTP bağlantı yolunun bütün parçaları uygulama politikasından geçer.
- İlk çağrının host/veri çıkışı tarifi mevcut onay yoluna bağlanır. Web işlemleri açık yeni tarif olmadan kabuk/MCP uyarısı gibi hakeme uygun sayılmaz. Kayıtta URL ve kapsam değişimi eski onayı geçersiz kılar.
- HTML script/ham HTML olarak çalıştırılmaz. Metin mevcut dış kaynak denetimi ve gönderim önizlemesinden geçer; kaynak sayfadaki talimat yetki vermez.
- Sayfalama varsa sonuç belirli içerik anlık görüntüsüne bağlı imleç taşır; URL kimlik/anahtarları günlükte maskelenir. Gövde sınırı aşılmış yanıt başarılı tam sayfa sayılmaz.

## Görev 1 — Ağ ve izin deneyi

- [x] Mevcut `vekil/adres.rs`, DNS/bağlantı ve sistem proxy davranışını inceleyin. Adres yardımcılarını çağırmakla soket hedefinin doğrulanmasının aynı olmadığını testle gösterin.
- [x] Ürün izin tarifi, host/redirect kapsamı ve özel ağ reddini PR'a somut olarak yazın. Test fixture özel izni ürün moduna taşınmasın.
- [x] TLS/redirect/IP değişimi ve açılmış gövde sınırı testleri eksik davranışta düşsün.
- [x] Tamam: hedef IP/host politikasını atlayan bağlantı yolu kalmaz; yeni onay sınıfı ve mühür girdileri belgelenir.

## Görev 2 — Sınırlı HTTP ve metin

- [x] Mevcut workspace reqwest/url'yi kullanın. HTML için mevcut bağımlılık yeterli değilse en fazla bir küçük parser adayı, sürüm/lisans/bağımlılık/audit bulgusu ve alternatifini karar raporuna yazın; kurulumu ayrıca yetkilendirilir.
- [x] HTML'yi regex'le genel ayrıştırıcı gibi işlemeyin; parser kanıtı yoksa düz metin sınırını açık gösterin ve HTML'yi tamamlandı saymayın.
- [x] Charset/UTF-8, sıkıştırma bombası, eksik cevap, sonsuz akış ve yanlış content-type testleri.
- [x] Tamam: W02/W04/W05, gerçek yerel HTTP bağlantı testlerinde geçer; mock DNS testi tek başına SSRF kanıtı değildir.

## Görev 3 — Araç, kaynak ve görünürlük

- [x] Hazırlık/izin/mühür ve araç kaydına bağlayın; yeni araç mevcut Review/Otomatik kip kuralını aşmasın.
- [x] Çıktıda URL/kaynak/eksik içerik tek anlam taşır. UI ham HTML basmaz; link açma mevcut yerel onaydan geçer.
- [x] Dış sayfa sahte sistem/kullanıcı onayı içerdiğinde meşru görev devam eder; sır istenirse engellenir.
- [x] Tamam: W06 ve gönderim önizlemesi/denetçi/bütçe testi geçer.

## Görev 4 — Arama sağlayıcısı kararı

- [x] Kullanıcının mevcut uygun arama MCP'si varsa önce onu değerlendirin. Yoksa en çok iki adayın güncel resmî belgesini, hesap/ücret/veri gönderimi ve kurulum ihtiyacını raporlayın.
- [x] Tek sağlayıcı seçimi kullanıcıya açık olsun; plan görevi servis satın alma/kayıt onayı değildir. Arama için model sağlayıcısı değiştirilmez.
- [x] Uzak arama G3b'ye bağlıdır. Aynı uzak aracı ikinci bir yerleşik isimle sunup izin/bütçe yolu çoğaltmayın.
- [x] Tamam: somut sağlayıcı/tanım, erişim koşulu ve fixture sonuç formatı sabittir; karar veya erişim eksikse G4b açık bekler.

## Görev 5 — Arama → kaynak → yanıt

- [x] Başlık/URL/kısa arama sonucu → asıl sayfayı getir → iddiayı o sayfaya bağla akışını fixture'larla kurun.
- [x] İptal, hız sınırı, yetersiz bakiye/kimlik, boş sonuç, hatalı URL ve kaynağın değişmesi durumları açık olsun.
- [x] W01/W03'ün fixture başarısı gerçek sağlayıcı denemesi diye sunulmaz. Gerçek bağlantı yetkisi ve varsa deney bütçesi kullanıcıdan gelir.
- [ ] Tamam: gerçek sağlayıcıyla dar yetkili kabul raporu vardır; yoksa arama aşaması tamamlanmadı görünür.

## Kaynaklar ve doğrulama

[Adres kuralları](../../../crates/kalfa-sandbox/src/vekil/adres.rs), [vekil bağlantısı](../../../crates/kalfa-sandbox/src/vekil/mod.rs), [araç kaydı](../../../crates/kalfa-tools/src/lib.rs), [izin motoru](../../../crates/kalfa-core/src/izin.rs), [gönderim](../../../crates/kalfa-core/src/gonderim.rs).

`cargo test -p kalfa-tools --test web_getir --locked` ve tools/sandbox/UI/uygulama için ilgili AGENTS kontrolleri. Yeni parser varsa audit ve iki platform paket derlemesi kayda girer.

## Bitirme ve devir

- [x] Windows ve WSL Ubuntu 24.04 için uygulanan AGENTS kontrollerini komut, HEAD ve sonuçla kaydedin. Koşmayan kontrolü ve nedenini ayrıca yazın.
- [x] Yeni güvenlik sınırları için olumlu/olumsuz test, iptal ve kurtarma kanıtı ekleyin; gerçek yalıtım kabulü gereken işte yalnız sahtelerle tamam demeyin.
- [x] Diff yalnız bu hattın kodu, testleri ve belgelerinden oluşsun. Yeni davranış aynı commit'te Rust/TypeScript sözleşmelerine ve Türkçe metinlere işlensin.
- [x] Kendi plan kutularınızı ve bu hat için ayrı bulgu raporunu güncelleyin. GELISTIRME_DURUMU/README'yi bütünleştirme sırası geldiğinde güncel diff'i okuyarak değiştirin.
- [x] PR'ı kullanıcı haber verdiğinde Codex'in inceleyeceği şekilde bırakın; otomatik merge veya başka hatta kendiliğinden geçiş yoktur.
- [x] Devirde baş/HEAD, yapılan görevler, test/loglar, açık kararlar, tek sıradaki adım ve varsa canlı deney bütçesi bulunsun.
