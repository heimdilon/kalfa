# Kalfa yetenek programı — Antigravity paralel çalışma planı

**Tarih:** 2026-10-20. **Durum:** Plan PR'ları; ürün kodu uygulanmadı.
**Taban:** `main`, `4041a39995e3bea3d56515e53ffc8af47f68e1ae`.
**Kullanıcı kararı:** İlk teslimat yerel kodlama + web + MCP. Codex yalnız planları ve PR'ları hazırlar; Antigravity ayrı çalışma klasörlerinde uygular. Kullanıcı sonuçları bildirdiğinde Codex inceler. Bu kayıt Antigravity'ye otomatik mesaj göndermez veya uygulama başlatmaz.

## Hedef ve kapsam

Kullanıcının hata/özellik isteğinden proje keşfi, güncel belge, değişiklik, test ve kanıtlı sonuç akışına ulaşmak. Windows ve Linux, EVREN, Türkçe arayüz, bağlı onay, yalıtım ve kayıt/kurtarma korunur. Modelin kodlama yeteneği ile araç altyapısı ayrı ölçülür.

İlk teslimatta alt ajanlar, worktree orkestrasyonu, tarayıcı/masaüstü kontrolü, zamanlanmış işler, macOS, eklenti mağazası ve otomatik model router yoktur. LSP, görsel/PDF/notebook, toplu yama ve tam beceri sistemi sonuçlara göre sonraya sıralanır.

## Plan paketleri

Her satır ayrı plan dalı/PR'ıdır; bağımsız doküman yolları kullanır. Ürün uygulaması ilgili plan dalında veya ondan türetilmiş, aynı PR'a bağlı dalda yapılır. Başka hattın planı ve devir dosyası düzenlenmez.

| Hat | Dal | Plan | İlk paralel iş |
|---|---|---|---|
| 01 | `codex/plan-yetenek-olcum` | [plan](https://github.com/heimdilon/kalfa/blob/codex/plan-yetenek-olcum/docs/superpowers/plans/2026-10-20-kalfa-yetenek-01-olcum.md) | 24 fixture ve puanlama |
| 02 | `codex/plan-yetenek-proje-akisi` | [plan](https://github.com/heimdilon/kalfa/blob/codex/plan-yetenek-proje-akisi/docs/superpowers/plans/2026-10-20-kalfa-yetenek-02-proje-akisi.md) | AGENTS kapsamı/durum sözleşmesi |
| 03 | `codex/plan-yetenek-surec-terminal` | [plan](https://github.com/heimdilon/kalfa/blob/codex/plan-yetenek-surec-terminal/docs/superpowers/plans/2026-10-20-kalfa-yetenek-03-surec-terminal.md) | kilit/süreç ve PTY deneyi |
| 04 | `codex/plan-yetenek-uzak-mcp` | [plan](https://github.com/heimdilon/kalfa/blob/codex/plan-yetenek-uzak-mcp/docs/superpowers/plans/2026-10-20-kalfa-yetenek-04-uzak-mcp.md) | SDK/HTTP protokol deneyi |
| 05 | `codex/plan-yetenek-web` | [plan](https://github.com/heimdilon/kalfa/blob/codex/plan-yetenek-web/docs/superpowers/plans/2026-10-20-kalfa-yetenek-05-web.md) | ağ/izin ve HTTP fixture'ları |

## Paralel başlama ve bütünleştirme

İlk dalgada beş hattın yukarıdaki işleri aynı anda, ayrı worktree'lerde yürüyebilir. Başka işin sonuç beklemesi genel hazırlığı durdurmaz. Sınırlı deneyin yeşil olması ürünün tamamlandığı anlamına gelmez.

Ortak dosyalar: core `arac/izin/muhur/protokol/kayit/kurtarma/oturum`, uygulama `oturumlar/sozlesme`, Tauri komutları, UI `ipc/tipler`, ana konuşma/ayar bileşenleri, workspace manifest/lock.

1. Her hat özel modül/fixture/deneyi kendi dalında hazırlar; ortak sözleşme ihtiyacını PR'a alan/olay/hata/kurtarma biçimiyle yazar.
2. Ortak core/IPC ürün eklemeleri **02 Proje → 03 Süreç → 04 MCP → 05 Web** sırasıyla bütünleşir. 01 Ölçüm bunlardan bağımsızdır; en son ortak kabulü yapar.
3. Sıradaki hat önce tamamlanan ortak sözleşme commit'ini kendi dalına alır ve diff/testini tekrar doğrular. Bu sıra merge onayı veya başkasının dalına yazma yetkisi değildir.
4. Ortak manifest/lock değişikliğini aynı anda iki hat hazırlamaz: ilk gerekçeli dar ekleme sabitlenir; sonraki hat o commit'i alır. Kilit dosyası elle çatışma işaretleri temizlenerek üretilmez.
5. Bağımlı alt görev bekliyorsa yalnız bağımsız görevler sürer. Örneğin web sayfa getirme devam edebilir; uzak arama gerçek G3b bağlantısı gelmeden tamamlanamaz.
6. Yalnız doküman PR'larının birleşmesi ürün uygulama doğrulaması değildir. Kullanıcı haber verdiğinde Codex her uygulama diff'ini ve ortak son HEAD'i inceler.

## Teknik geçiş kapıları

- Süreç: mevcut kabuk bütün alan kilidini çağrı boyunca tutar. Erken dönüşte süreç/kilit/çıktı ve doğrulanan kapanış sahibi netleşmelidir. Kilidi bırakıp çalışmayı sürdürme güvenlik modeli değişikliğidir.
- PTY: Linux ve Windows'ta mevcut yalıtımla gerçek deney gerekir. Mevcut MCP borulu süreç desteği PTY kanıtı değildir.
- MCP: sabit SDK ile desteklenen protokol sürümleri ve HTTP/OAuth davranışı ölçülür. Güncel spec ile eski el sıkışma aynı varsayılmaz.
- Web: ana süreçten HTTP, kabuk yalıtımıyla kendiliğinden korunmaz; bağlanılan IP ve her yönlendirme doğrulanır. Tek arama sağlayıcısının hesap/erişim kararı ayrıca somutlaştırılır.

Bu kapılar için planlarda üretim Rust kodu verilmez ve ön derleme iddiası yoktur. AGENTS'in plan ön doğrulama gereği, uygulayıcının ilk deney görevlerine açık kabul koşulu olarak konmuştur. Kapı geçmeden ilgili ürün adımı hazır sayılmaz; geçilemeyen koşul PR'a kanıtıyla yazılır.

## Değişmez koşullar

Her planda Global Constraints ve bitirme listesi vardır. Güvenlik sınırları, izin listeleri, sır maskeleme, bütçe, gönderim önizlemesi ve sonucu bilinmeyen işin tekrarlanmaması korunur. Yeni işlem otomatik hakem iznine kendiliğinden girmez. Yeni paket/indirme/sistem ayarı ve merge için AGENTS kuralları geçerlidir.

Mevcut PR #35 kullanıcı/Claude akışında kalır; bu program onu devralmaz veya birleştirmez. Yeni ürün entegrasyonu öncesi #35'in güncel durumu ve ortak dosya farkları yeniden okunur; bekleyen uygulama yanlışlıkla kaybedilmez.

## Ölçüm ve tamamlanma

24 sabit görev; yerel kodlama, süreç/oturum, MCP ve web aileleri. Her görevin bağımsız denetleyicisi vardır. Fixture araç sözleşmesini, canlı model koşusu görev başarısını ölçer; birbirlerinin yerine geçmezler.

Başlangıç ürün hedefi iki tekrarda da en az 20/24 ve bütün zorunlu güvenlik testlerinin geçmesidir. Rakip ürünlerle eşitlik gerçek karşılaştırma raporundan değerlendirilir; farklı modellerin etkisi saklanmaz. Canlı küçük tur 36, tam tur 144 koşu önerisidir; mevcut onay olmadan başlatılmaz ve yapılmış ölçüm diye yazılmaz.

Bütünleştirme sonunda Windows/WSL, ilgili UI/e2e ve gerçek yalıtım kabulü gerekir. CI bunların yerini tutmaz. Başarı, eksik görevler, insan müdahalesi, süre ve bilinen/bilinmeyen tüketim raporlanır.

## PR ve CI kullanımı

Bu altı başlangıç PR'ı yalnız Markdown plan/devir içerir. Kod/prototip/test uygulanmış veya iki platform yeşil gösterilmez. Güncel Actions bütçesi bu oturumda okunamadığı için belge commit'lerinde standart `[skip ci]` işareti kullanılır; workflow/test kodu değişmez. Uygulama commit'leri bu işareti taşımaz; zorunlu CI ve yerel kontroller çalışır. Taslak plan PR'ları bu nedenle ürün olarak merge-ready sayılmaz.

## Başlangıç ve devir

Kullanıcı her Antigravity oturumuna ilgili PR'ı ve o PR'daki `devirler/2026-10-20-yetenek-NN.md` başlangıç prompt'unu verir. Bir uygulayıcı bir işin sahibidir. Kullanıcı haber vermeden Codex arka planda izleme, uygulama veya otomatik birleştirme yapmaz.

İş sonunda Antigravity kendi plan kutuları, bulgu raporu ve PR'a HEAD/komut/sonuç, iki platform kanıtı, açık karar ve tek sonraki işi yazar. Codex plan uyumu, davranış, regresyon ve güvenlik sözleşmelerini bu diff/HEAD üzerinde inceleyecektir.
