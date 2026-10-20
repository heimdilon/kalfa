# arka plan süreçleri ve terminal planı

**Tarih:** 2026-10-20. **Planlayan/inceleyen:** Codex. **Uygulayıcı:** Antigravity.
**Durum:** Plan taslağı; ürün uygulanmadı, iki platform ön doğrulaması yapılmadı.
**Plan tabanı:** main `4041a39995e3bea3d56515e53ffc8af47f68e1ae`. **Çalışma dalı:** `codex/plan-yetenek-surec-terminal`.
**İlk başlanacak iş:** Önce kilit/süreç sahipliği ve iki platform PTY deney raporunu üretin. Bu rapor yokken kabuğa arka plan bayrağı eklemeyin.
**Koordinasyon:** [iş sırası ve ortak dosyalar](https://github.com/heimdilon/kalfa/blob/codex/plan-yetenek-koordinasyon/docs/superpowers/specs/2026-10-20-kalfa-yetenek-programi.md).

## Global Constraints

- Kullanıcının 20 Ekim talimatı: Codex plan ve PR hazırlar; Antigravity uygular; kullanıcı haber verince Codex inceler. Bu belge ürün kodu veya derlenmiş örnek içermez; ön doğrulama yapılmış gibi sunulamaz.
- İlk teslimat yerel kodlama + web + MCP, Windows ve Linux'tur. EVREN, Türkçe ad/metin, bağlı onay, yalıtım, sır koruması, bütçe ve JSONL kurtarma korunur.
- Başlamadan güncel AGENTS.md, GELISTIRME_DURUMU, Jira sahipliği ve PR başını okuyun. Her hat ayrı worktree'dedir; aynı klasöre iki uygulayıcı girmez.
- İlk görevde planın varsayımlarını ve gerçek SDK/platform arayüzlerini doğrulayın. Yeni paket/indirme/sistem ayarı AGENTS iznine tabidir; planda adı geçmesi kurulum onayı değildir. Yeni unsafe gerekirse dosya listesi ve gerekçe uygulamadan önce tasarımda kesinleşir.
- Hazırlık → izin/onay → taze mühür/kilit → yürütme yolu korunur. Yeni işlem türü mevcut Otomatik kip izin listesine kendiliğinden eklenmez. Yalıtımlı iş sessizce tam erişime düşmez.
- Sonucu bilinmeyen yan etkili iş tekrar yürütülmez. İş başlatıldı kaydı, iş başarıyla bitti kaydı yerine geçmez.
- Üretim kontrolleri AGENTS.md'deki kapsamla iki platformda koşar. CI gerçek yalıtım ve WebDriver yerine geçmez. Canlı EVREN/arama servisi denemesi ve merge mevcut ayrı yetki kurallarına tabidir.
- Her görev: davranışı ölçen düşen test/deney → küçük uygulama → geçen test → ilgili diff incelemesi. Plandan sapma gerekiyorsa gerekçeyi PR'a yazın; güvenlik kuralını sessizce gevşetmeyin.
- Ortak dosyaları değiştirmeden kendi PR'ınıza dar bir sözleşme notu yazın: yeni alan/olay, kaynak sahibi, kayıt/kurtarma ve hata davranışı. Koordinasyon planındaki sıra uygulanır; diğer hattın bekleyen tiplerini uydurmayın.

## Amaç ve en önemli bağımlılık

G2a deney → G2b borulu işler → G2c PTY. Mevcut `EtkilesimliSurec` MCP'ye özeldir; genel terminal gibi sunulamaz. `kabuk` bütün çalışma alanı kilidini çağrı boyunca tutar. Erken araç dönüşü, kilit ve kapanış sahipliğini yeni bir modüle açıkça devretmeden yapılamaz.

## Dosya sahipliği

| Yol | Sorumluluk |
|---|---|
| `prototip/yetenek-surec/` (yeni, ürün workspace'i dışında) | Windows/Linux deney kaynakları ve çalıştırma talimatı |
| `docs/superpowers/bulgular/yetenek-programi/surec-deneyi.md` | Kanıt, seçilen çözüm ve açık engel |
| `crates/kalfa-uygulama/src/isler/` (yeni) | İş kimliği, sahiplik, çıktı deposu ve kapanış |
| `crates/kalfa-tools/src/kabuk/` | Şema ve iş araç sarmalayıcıları |
| `crates/kalfa-sandbox/src/surec.rs`, `etkilesimli.rs`, Linux/Windows modülleri | Başlatma, PTY ve süreç ağacı; mevcut güvenlik sözleşmeleri korunur |
| `ui/src/bilesenler/IsKarti.svelte` (önerilen yeni) | İş durumu/çıktı/girdi |

Core kilit/arac/izin/mühür/olay/kayıt ve IPC tipleri ortak dosyadır. G1 ortak sözleşmesi bütünleştikten sonra ürün entegrasyonu yapılır; deneyler G1'i beklemez. MCP hattı mevcut yerel süreç sözleşmesini bu hattın bekleyen PTY değişikliğine göre yeniden yazmaz.

## İlk yüzey önerisi ve sabit güvenlik koşulları

Mevcut `kabuk` varsayılan ön plan davranışı korunur. Arka plan biçimi iş kimliği döndürür; `komut_oku/komut_girdi/komut_durdur` yalnız ait olduğu oturumun işiyle çalışır. Ham PID, rastgele dosya yolu ve başka oturumun kimliği yetki sayılmaz.

- İş kimliğini uygulama üretir. Süreç, kilit ve kapanış denetiminin sahibi tek iş kaydıdır; araç future'ının bitmesi işi sahipsiz bırakmaz.
- Başlama, çalışma, durdurma, bitiş ve sonucu bilinmiyor durumları ayrıdır. Başlatıldı kaydı bitiş yerine geçmez.
- Yeni stdin her seferinde işlem tarifi/izin/denetim yoluna girer. İlk başlatma izni genel interaktif shell üzerinden sınırsız yeni komut izni değildir.
- Çıktı okuma imleçlidir; ham disk çıktısı denetçi/kaynak etiketlerini atlayan yan yol oluşturmaz.
- Kontrol araçları işin tuttuğu çalışma alanı kilidini bekleyip kilitlenmez. Kapanış başladıktan sonra girdi reddedilir.
- Süreç ağacı gerçekten kapanmadan iş bitmiş veya kaynak serbest sayılmaz. Belirsiz kapanış açık hata/karantina üretir; yeni dosya işini sessizce serbest bırakmaz.
- Uygulama çökmesinde eski PID'ye körlemesine bağlanılmaz/öldürülmez. Kimlik doğrulaması ve işletim sistemi sahipliği kanıtlanır.
- Sayısal iş/çıktı/disk/ömür/girdi sınırları G2a raporunda somut olarak sabitlenir; sınırsız ürün yolu kurulmaz.

## Görev 1 — Mevcut kilidi ve kapanışı yeniden üret

- [ ] Gerçek mevcut kabuğun genel yazma kilidini, kaynak devrini, son taramasını ve yazma kuşağını küçük diyagramla raporlayın.
- [ ] Çalışan uzun iş + bekleyen dosya yazması + durdurma isteği için test kurun. Durdurma genel kilidi bekliyorsa kilitlenmeyi ölçün.
- [ ] Tamam: hangi kaynak kimin yaşamına bağlı olduğu ve bugünkü engel tekrar üretilebilir.

## Görev 2 — İki platform deney kapısı

- [ ] Linux bwrap/seccomp ve Windows AppContainer/Job içinde borulu stdin/çıktı, Ctrl+C/EOF, PTY/ConPTY, torun süreç ve çıktı doluyken kapanışı deneyin.
- [ ] Yeni paket/unsafe gerekiyorsa yalnız deney tasarımında isim/dosya/gerekçe belirtin; kurulum ve yönetici adımları AGENTS kurallarıyla yürür.
- [ ] Aynı alanda sunucu çalışırken düzenleme için seçenekleri güvenlik ve kullanılabilirlik açısından karşılaştırın. Mevcut bütün alan kilidini gizlice kaldırmak seçenek değildir.
- [ ] Önce güvenli seri çözüm kanıtlanabilir; S06 desteklenmiyor kalır. Eşzamanlı yazmanın güvenli sözleşmesi yoksa ürün buna hazır denmez.
- [ ] Tamam: iki platformun log/HEAD'leri, seçilen yaşam/kilit tasarımı, sabit sınırlar ve yeni unsafe dosya listesi rapordadır. Güvence değişikliği gerekiyorsa bu PR'da gerekçeli tasarım incelemesi için durak bırakılır; uygulayıcı kendi başına kural değiştirmez.

## Görev 3 — Borulu iş sahibi

- [ ] Raporun kararı doğrultusunda `isler` modülünü ekleyin; küçük arayüz: başlat/oku/girdi/durdur. İçeride iş kaydı süreç/kilit/çıktı/son taramayı sahiplenir.
- [ ] Yetkisiz/yanlış kimlik, çift durdurma, stdin okumayan çocuk, taşan çıktı, sahip oturumun kapanması, kapanış hatası testleri önce düşsün.
- [ ] Tamam: erken araç dönüşü sonrası süreç sahipsiz değildir; kayıt ve kaynak ömrü doğru.

## Görev 4 — İzin, kayıt ve araçlar

- [ ] Başlatma ve girdi tariflerini ayrı mühürleyin. Kip/alan/sır koruması değişince mevcut onay davranışını koruyun.
- [ ] Otomatik hakem sınıfları ve önizleme açıkken davranış açıkça ele alınır; yeni eylemler kendiliğinden hakeme uygun sayılmaz.
- [ ] İş başlama/bitiş/iptal/sonucu bilinmiyor olayları; eski JSONL okuma uyumu ve crash kurtarma test edilir.
- [ ] Tamam: S01/S02/S04 ve bütün yeni yetki testleri geçer; kontroller kilit bekleyerek kilitlenmez.

## Görev 5 — PTY ve UI

- [ ] Aynı iş arayüzünün arkasına PTY/ConPTY ekleyin; boyut, UTF-8 bölünmesi, ANSI, Ctrl+C/EOF ve bağlantı kapanışı sınansın.
- [ ] Terminal kaçışları ham HTML, panoya yazma, link/komut çalıştırma yetkisi oluşturmaz. Model girdi yolu ile yerel kullanıcı eylemi kaynakları ayrılır.
- [ ] Tamam: S03 ve gerçek yalıtım kabulü iki platformda geçer. S06 ancak ayrı kilit güvencesi kanıtıyla kapanır.

## Görev 6 — Son kabul

- [ ] Gerçek süreç/torun/Job sayacı ve kalan profil/ACE kontrolünü raporlayın; yönetici kurulumunu uygulayıcı otomatik çalıştırmaz.
- [ ] Tamam: AGENTS'teki sandbox/tools gerçek yalıtım + uyumluluk testleri ve UI/e2e kabulü yeşildir; eksik platform ürün tamam sayılmaz.

## Kaynaklar

[Mevcut süreç planı](2026-10-14-kalfa-11a-1-etkilesimli-surec.md), [süreç](../../../crates/kalfa-sandbox/src/surec.rs), [kilit](../../../crates/kalfa-core/src/kilit.rs), [kabuk](../../../crates/kalfa-tools/src/kabuk/mod.rs), [araç yürütme](../../../crates/kalfa-core/src/oturum/arac_asamasi.rs), [ConPTY](https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session).

## Bitirme ve devir

- [ ] Windows ve WSL Ubuntu 24.04 için uygulanan AGENTS kontrollerini komut, HEAD ve sonuçla kaydedin. Koşmayan kontrolü ve nedenini ayrıca yazın.
- [ ] Yeni güvenlik sınırları için olumlu/olumsuz test, iptal ve kurtarma kanıtı ekleyin; gerçek yalıtım kabulü gereken işte yalnız sahtelerle tamam demeyin.
- [ ] Diff yalnız bu hattın kodu, testleri ve belgelerinden oluşsun. Yeni davranış aynı commit'te Rust/TypeScript sözleşmelerine ve Türkçe metinlere işlensin.
- [ ] Kendi plan kutularınızı ve bu hat için ayrı bulgu raporunu güncelleyin. GELISTIRME_DURUMU/README'yi bütünleştirme sırası geldiğinde güncel diff'i okuyarak değiştirin.
- [ ] PR'ı kullanıcı haber verdiğinde Codex'in inceleyeceği şekilde bırakın; otomatik merge veya başka hatta kendiliğinden geçiş yoktur.
- [ ] Devirde baş/HEAD, yapılan görevler, test/loglar, açık kararlar, tek sıradaki adım ve varsa canlı deney bütçesi bulunsun.
