# G5 v2 — gerçek araç ve masaüstü kabulü

## Global Constraints

- Kullanıcının 21 Ekim “sana güveniyorum sen yaparsın hepsini” talimatı, kalan çevrimdışı uygulama ve kabulü Codex'e verir. Canlı hesap/anahtar/bütçe, yeni indirme/kurulum, Windows yönetici/root ACL adımları ve merge mevcut ayrı yetki kurallarına tabidir.
- Çalışma dalı codex/g5-butunlesik-kabul; son #41 d33f239 ve #37 1d88a1a yerel bütünleşik tabanı5a42f4e. Diğer uygulayıcıların worktree'leri ve ana checkout farkları korunur.
- G0'ın24 JSON fixture'ı ve SHA-256 özetleri sabit kalır. G5v2, güncel ürünün gerçek çağrı/onay sözleşmesine ayrı sürümlü adaptördür. M02'nin eski onaysız hedefi ürün politikasını değiştirmez; G5'te onaylı gerçek çağrı ölçülür. Eski G0 ile birebir aynı deney gibi karşılaştırma yapılmaz.
- Dış model yanıtı senaryolu olabilir; uygulama, araç, kilit, izin/mühür, JSONL ve yerel HTTP/TLS/stdio/süreç yürütmesi gerçektir. Kaynak dosyası okumak veya başarı metni görmek uzaktaki çağrı yerine geçmez.
- Kabul sınırları mevcut planda onaylıdır: araçların hazırlık/yürütmesi, Uygulama oturum/olay API'si ve Tauri WebDriver. Mevcut test yardımcıları kullanılabilir; güvenlik sınırları için özel sahte ürün implementasyonu eklenmez. Yalnız dış servis/diyalog/model sabitlenir.
- Her dikey dilim önce gerçek kusur/eksik adaptörle kırmızı, sonra gerçek yürütmeyle yeşil kanıt verir. Ürün kusuru çıkarsa dar düzeltme ve regresyon eklenir. Yeni bağımlılık/unsafe öngörülmez.
- Her iki platform ve iki tekrarda sabit G5v2 tanımları kullanılır. Atlanan/sıfır test, eksik yardımcı, zaman aşımı veya belirsiz çıktı Geçti sayılamaz. En az20/24 ve bütün zorunlu güvenlik kontrolleri gerekir. Sonuç harness davranış kabulüdür; canlı LLM/rakip kalite iddiası değildir.

## Uygulama ve sahiplik

1. kalfa-uygulama/tests/g5_uygulama.rs ve test yardımcıları: M02 gerçek HTTP+onay, K06 talimat kapsamı, süreç/oturum S01–S06, uygulama MCP akışları. Bilinen ortak yardımcılar mümkün olduğunda yeniden kullanılır.
2. kalfa-tools/tests/g5_kodlama.rs ve g5_web.rs: K01–K05 gerçek bağımsız denetleyiciler; M01 gerçek yalıtımlı stdio; W01–W06 gerçek TLS/HTTP, araç ve kaynak/sır denetimi. G0 davranışı değiştirilmez.
3. e2e/yetenekler/: sürümlü manifest, sonuç/kanıt doğrulayan iki-tekrar koşucu ve gerçek Tauri süreç/MCP/web kabulü. Başarı yalnız test/araç kanıtından üretilir; puan sabitlenmez.
4. Windows M01 geliştirici makinesinde çalıştırılmaz: otomatik onay denetimi, testin AppContainer/ACL hazırlığını AGENTS sınırı nedeniyle reddetti. Kullanıcının açık geçici-CI izniyle Sürüm paketleri workflow'una yalnız workflow_dispatch için g5_m01 seçeneği eklenir. Önceden derlenmiş gerçek M01 test ikilisi, kurucu kurulduktan sonra mevcut CI kullanıcı betiğinin sabit M01 kipinde yönetici olmayan kalfatest ile iki kez çalışır; test ve kurucu izinleri geri alınır. Yeni paket/bağımlılık eklenmez. Linux doğrulaması normal kalfatest kullanıcısıyla devam eder.
5. Paket kabulü mevcut sürüm workflow'u ve kurucu testleri üzerinden; gerçek kurulumu geliştirici makinesinde yönetici olarak otomatik yapma. CI'daki geçici makine kabulü veya kurulum gerektirmeyen inceleme kullanılabilir.
6. AGENTS doğrulaması, iki eksenli bağımsız kod incelemesi, kaynak/baş/log makbuzu, PR ve ortak hafıza güncellemesi. Merge yok. Geçici worktree/target temizlenir.

## Kapanış

Her24görevin gerçek giriş ve bağımsız kabulü, iki tekrar sonucu, platform önkoşulları ve zorunlu güvenlik kontrolleri tek raporda görünür. Kullanıcı 21 Ekim yanıtıyla ücretli canlı model/dış arama denemelerini bu teslimattan çıkardı; yalnız çevrimdışı ve geçici CI kabulü yürütülür. M02/M05/M06/W görevlerinin G0 içindeki eski onaysız adımları güncel açık onay sözleşmesine uyarlanır; G0 dosya özetleri korunur.

## Gerçek masaüstü kabulü — 22 Ekim

Terminalde UI girdisi ve doğal çıkış0; mevcut gerçek stdio MCP form/başlatma/onay/sonuç; üretim web aracının özel IP reddi, sıfır TCP bağlantısı ve JSONL; plan/soru akışı çalıştırılır. Web başarı/kaynak denetimi gerçek uygulama/HTTP/TLS G5 görevlerinde kalır; masaüstü web testi güvenlik reddini ölçer. Windows süreç/MCP kabulü yalnız kullanıcının onayladığı geçici kurucu CI makinesinde, kurulu ürünün gerçek yalıtım altyapısı ve standart kullanıcıyla koşar. g5_masaustu varsayılan kapalıdır. E2e derlemesi üretim NSIS paketinden sonra ayrı kimlikle üretilir ve kurucuya girmez. Mevcut test aracı tauri-driver2.0.6 ve WebView2 ile aynı, Microsoft imzalı EdgeDriver yalnız geçici CI test klasörüne alınır; geliştirici bilgisayarına kurulum/sistem değişikliği yapılmaz. Sıfır/atlanan test kabul edilmez.
