# MCP 1.1a-2 Görev 6 — çağrı izni ve kapanış boyunca kilit

Durum: tamamlandı. Dal `codex/mcp-11a2-plan`, PR #20. Kullanıcının son
talimatıyla bu görev kapatılıp devir bırakılır; Görev 7 ürün uygulaması başlamaz.

## Davranış

MCP çağrısı bütün çalışma alanının yazma kilidini alır. Core, kilitli taze
hazırlığın ilk bekleyişinden önce guard'ı yönetici çağrı kaydına devreder.
Çağrı future'ı, yanıt alıcısı veya son oturum bırakıldığında guard çözülmez.
Bağımsız finalizer süreç ağacının kapanışını ve son koruma denetimini bekler.
Kapanış veya tarama başarısızsa kayıt/guard saklanır, alan engelli görünür.

Sıkı/Proje/Serbest/İnceleme ve yalıtımlı/tam erişim kararları gerçek actor
yolunda uygulanır. Tam erişim her çağrıda yerel onay ister. Tek onay
R/D/argümanlara, kısayol T/D/Π'ye bağlıdır; N/K tek başına kısayolu düşürmez.
Eski N'nin temizliği N+1'i durduramaz. Eski kartın temizliği bittikten sonra
kilitsiz yeni onay istenir; yeni çağrı yine kilit altında doğrulanır.
Modele yalnız tek nihai Tool yanıtı verilir.

Actor, sonuç denetimi ve finalizasyonu olay kuyruğunda beklerken bütçe,
iptal ve diğer araç işlerini işler. RPC deadline'ı zorunlu cleanup'ı kesmez.
Hazırlık hatasıyla kalan rezervasyon da doğrulanmış cleanup'tan geçer.

İptal bildirimi için taşıma kuyruğuna ekleme başarı sayılmaz: yazıcı
`write_all` ve `flush` tamamlandığında makbuz verir. İstemci en çok 100 ms
bekleyip tıkalı taşımanın kapanışını zorlar. Bu bekleme ve stdin sonrası
kibar çıkış aynı iki saniyelik deadline'ı paylaşır. `kapat_kadar` kalan
payı büyütemez; süresi tükenmişse ağaç öldürme/doğrulamasına geçer.
Varsayılan `EtkilesimliSurec::kapat()` önceki iki saniyelik yolunu korur.
§4.3'te kabul edilmiş toplam üst sınır genişlemez.

## Karşı örnekler ve bağımsız inceleme

- Kök kilit tablosunu yaşatmayan eski guard davranışıyla yeni oturumun
  kilidi aşması RED; sahipli guard kök Arc'ını yaşatınca GREEN.
- Gerçek actor yolunda MCP izin kararının atlanması RED; sekiz izin
  hücresi, tam erişimde art arda iki yerel onay ve ilk/ağ kartı GREEN.
- Hazırlık hatasının RPC öncesi rezervasyonu bırakması RED; bağımsız
  cleanup ve doğrulama beklemesi GREEN. Worker 12/12.
- RPC deadline'ının cleanup'ı kesmesi, taze onay döngüsü, birleşik ad
  kısayolu ve finalizasyonda bütçe/iptal regresyonları geçti.
- İlk gerçek süreç koşusunda CANCEL gözlenmedi. İstemcinin taşıma
  kapatmasını iptal yazımından önce yapabildiği doğrulandı ve düzeltildi.
- Flush makbuzu bekleyişi tek başına kaldırılınca test assertion ile RED;
  `poll_flush` kapısı açılana kadar gönderim bekleyince GREEN. Yazma
  kuyruğunun tamamen dolu olduğu eski tıkanma testi de geçti.
- Kalan kapanma payı yerine tekrar iki saniye verilince test RED;
  ortak deadline ile GREEN. Son sağlamlık düzeltmesinde test OS öldürme
  süresine 1,5 saniyelik eşik koymaz; EOF sonrası gecikmeli doğal çıkış
  fırsatının süresi tükenmiş payla yeniden açılmadığını ölçer.

GPT-6 Luna / max salt okunur bağımsız incelemesi actor, wrapper, çağrı
kaydı, Π/D/N yarışı ve kapanış yollarını kapsadı. Ürün bulguları kapandı;
flush testi kapsam notu düzeltildi. Son test sağlamlığı düzeltmesi ayrıca
iki platformda hedefli test/clippy/fmt ile doğrulandı. İncelemeyi yapan
ajan kod yazmadı; test koşularını ana ajan yürüttü.

## İki platform doğrulaması

| Kontrol | Windows | Ubuntu-24.04, kalfatest |
|---|---:|---:|
| Workspace | 1389 geçti, 19 ignore | 1344 geçti, 19 ignore |
| e2e özellikli Rust | 250/250 | 251/251 |
| Gerçek yalıtım | 50/50 | 32/32 |
| Uyumluluk | 24/24 | 16/16 |
| Gerçek stdio kabulü | 4/4 | 4/4 |
| fmt, üretim client-only, workspace/e2e/yalıtım clippy | Temiz | Temiz |

Planın AGENTS komutları çevrimdışı ve `--locked` ile çalıştı; gerçek
yalıtım/uyumluluk testleri tek iş parçacığıyla koştu. Aynı üretim kaynakları
iki platformda sınandı. R2'den sonraki tek Rust farkı kapanma payı testinin
OS süre eşiğini kaldırmasıdır; son kaynakta bu test, ilgili crate clippy'si
ve fmt iki platformda tekrar geçti. Yeni bir ürün kodu değişikliği yoktur.

Gerçek süreç kabulünde iptal+deadline × hemen yanıt/sessiz/yok sayma
altı senaryodur. Gecikmeli torun 5 saniye sonra yazacak şekilde kurulur;
doğrulanan kapanıştan sonra işaret dosyası değişmez. Son tarama bariyeri
ikinci gerçek RPC'yi bekletir. Eski N kartı N+1'e gönderilmez ve N+1
eski cleanup nedeniyle durmaz.

Bu MCP kabulü gerçek platform süreçleri ve SDK kullanır; tanım/sonuç
dönüştürücüsü G7 dikişinde sahtedir, koruma son denetimi kontrollü
bariyerdir. Gerçek G7 kapıları veya tam MCP yalıtım kabulü olduğu iddia
edilmez. Ayrı OS yalıtım takımı yukarıdaki 50/32 testtir. UI/WebDriver
koşulmadı; Görev 11–12 kapsamındadır.

## Kanıtlar ve temizlik

Yerel kayıtlar `.yerel/mcp-tasarim-inceleme/` altındadır:

- `11A2-CODEX-g6-r2-{WINDOWS,WSL}.log`: tam doğrulama.
- `11A2-CODEX-g6-{izin,hazirlik,flush,pay}-{RED,GREEN}.log`:
  davranış karşı örnekleri. İlk iki hata eski uygulamada, son ikisi
  tek düzeltmenin geçici geri alınmasıyla ölçüldü.
- `11A2-CODEX-g6-gercek-RED.log` ve `...gercek-WINDOWS.log`:
  ilk gerçek süreç hatası ve düzeltilmiş kabul.
- `11A2-CODEX-g6-son-saglamlik-{WINDOWS,WSL}.log`: son test düzeltmesi.
- `G6-SON-KAYNAK.json` ve `G6-SON-KAYNAK-DENETIM.log`: son kaynak eşleşmesi.
- `11A2-G6-TEMP-TEMIZLIK.log`: kendi g6-r1/r2 test köklerinin temizliği.

Ortak target, WSL kaynak kopyası ve ürün worktree'si devir için korunur;
önceden kalan 44 dizine dokunulmaz. Görev 5 `db7c391` CI 38869835874
yedi işiyle yeşildir; Görev 6 commit'inin CI'sı PR'da ayrıca izlenir.

Yeni paket/indirme, Cargo.toml/Cargo.lock değişikliği, sistem kurulumu,
yönetici yetkisi, canlı EVREN veya şart kabul çağrısı yoktur. PR merge
edilmez. Sıradaki ürün işi Görev 7'dir; bu devirde başlatılmaz.
