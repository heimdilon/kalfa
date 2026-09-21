# Kalfa tasarımı — değerlendirme ve öneriler

Tarih: 2026-09-21  
İncelenen belge: [Kalfa tasarım spec’i](2026-09-21-kalfa-design.md)  
Kapsam: Tasarım incelemesi, ürün fikirleri ve güvenlik önerileri. Bu belge bir uygulama planı veya doğrulanmış kod açığı raporu değildir. Mevcut spec ve proje dosyaları değiştirilmemiştir.

## 1. Genel görüşüm

Kalfa’nın yönünü doğru buluyorum. Türkçe arayüz, EVREN’in şart ve kredi davranışlarını bilen sağlayıcı katmanı, kullanıcıya anlaşılır izinler ve genişletilebilir araç sistemi birlikte anlamlı bir ürün oluşturuyor. Özellikle canlı API gözlemlerinin fixture olarak saklanması, çekirdeğin UI olmadan test edilebilmesi ve MCP ile WASM arasındaki güven farkının açıkça yazılması güçlü kararlar.

Bence spec, uygulama planına geçmek için iyi bir temel; ancak güvenlik ve hata durumları konusunda birkaç iddiası henüz tanımladığı mekanizmaların önünde. En büyük eksik yeni bir özellik değil: **hangi işlemin, hangi yetkiyle, hangi veriyi kullanarak, hangi sınırlar içinde gerçekleşeceğinin uçtan uca sözleşmesi.**

İlk sürümde önceliğim şu olurdu: küçük ama güvenilir bir kod ajanı; anlaşılır onay ekranı; sağlam iptal ve kurtarma; görünür veri çıkışı; sınırları test edilmiş araç yürütme. Plugin ekosistemini bu temelin üzerinde büyütürdüm.

Bu değerlendirme spec’in tamamına, kök README’ye, probe README’sine ve seçilmiş model/sohbet/akış/hata fixture’larına dayanıyor. Yeni EVREN çağrısı yapılmadı; ücretli işlem veya şart kabulü gerçekleştirilmedi. Sağlayıcının güncel hizmet davranışı ve hukuki uygunluğu bu incelemede yeniden doğrulanmış sayılmamalı.

## 2. Korunmasını önerdiğim kararlar

- **UI’dan bağımsız çekirdek (§4):** Aynı döngüyü sahte sağlayıcılarla çalıştırabilmek test edilebilirlik açısından doğru temel.
- **EVREN adaptörü (§5):** Kredi başlıkları, şart kapısı ve hata alanları ürünün gerçek farklılaşma noktaları.
- **Geçici Serbest kip (§7):** Kalıcı sınırsız izin bulunmaması iyi bir varsayılan. Oturum yeniden açıldığında da bu izinlerin geri gelmemesi açıkça tanımlanmalı.
- **Host üzerinden WASM erişimi (§8):** Doğrudan dosya ve soket erişimini kapatmak, uygulanabilir bir yetenek modeline kapı açıyor.
- **Gruplanmış diff onayı (§11):** Kullanıcının karar verebildiği bir ekran, araç adlarını tek tek onaylamaktan daha kullanışlı.
- **Şart kabulünü kullanıcı eylemine bağlamak ve anahtarı OS kasasında saklamak (§11.8):** Korunmalı; aşağıdaki IPC ve alt süreç sınırlarıyla tamamlanmalı.
- **MCP, router ve alt ajanları sonraya bırakmak (§12):** v1 odağını koruyor. Ek fikirlerin tamamını ilk sürüme taşımamak gerekir.

## 3. Uygulama öncesinde netleştireceğim noktalar

### 3.1 Kabuk erişimi ile çalışma alanı garantisi arasında boşluk var — P0

§7.2 her kipte çalışma alanı dışına çıkan yolun onay gerektirdiğini söylüyor. Fakat §7.1’deki kabuk komut allowlist’i, çalıştırılan programın sonradan hangi dosyaları açacağını veya nereye bağlanacağını denetlemiyor. Proje içindeki bir test betiği bile dışarıdaki dosyayı okuyabilir; `cwd` belirlemek bunu engellemez. Benzer şekilde git, proje yapılandırmaları ve hook’lar üzerinden ek süreçler çalıştırabilir.

Önerim: yerleşik dosya araçlarının sınırı ile alt süreçlerin sınırını ayrı tanımlamak. Güçlü çalışma alanı garantisi için Windows ve Linux’ta dosya/ağ erişimini gerçekten sınırlandıran bir süreç izolasyonu gerekir. Bu v1’e yetişmezse, yalıtımsız kabuk ayrı ve açık bir yetki yükseltme olarak gösterilmeli; kullanıcıya tüm araçlar için geçerli bir çalışma alanı izolasyonu vaat edilmemeli. Alt süreçleri sonlandırabilmek veya kaynaklarını sınırlamak, tek başına dosya/ağ izolasyonu sağlamaz.

### 3.2 Dosyayı okumak ile sağlayıcıya göndermek aynı izin olmamalı — P0

§7 çalışma alanı içindeki okumaları büyük ölçüde serbest bırakıyor. Oysa proje içindeki `.env`, özel anahtar, müşteri verisi veya test dump’ı da hassas olabilir. Bu verinin yerel araç tarafından okunması, chat’e, özetleyiciye, embedding’e veya güvenlik sınıflandırıcısına gönderilmesine otomatik izin vermemeli.

Önerim: veri çıkışı için ortak bir kapı oluşturmak. Bu kapı tüm EVREN çağrılarını, plugin host çağrılarını ve ileride diğer ağ araçlarını kapsamalı. Denetim, uzaktaki güvenlik sınıflandırıcısına gönderimden **önce** uygulanmalı; aksi hâlde “güvenli mi?” sorusunu sormak için veri zaten dışarı gönderilmiş olur.

§8.4’teki “Ağ erişimi istemiyor” ifadesi teknik olarak doğrudan soket erişimini anlatıyor, ancak kullanıcı bunu “veri bilgisayardan çıkmıyor” diye anlayabilir. Daha açık bir metin öneririm: “Doğrudan internet erişimi yok; seçilen dosya parçalarını embedding ve rerank için EVREN’e gönderir.”

### 3.3 Onay, gösterilen işlemin kendisine bağlanmalı — P0

`AraçOnayı(id, karar)` (§4.1) tek başına, onaylanan diff’in gerçekten uygulanacağını tarif etmiyor. Kullanıcı diff’i incelerken dosya değişebilir; araç argümanları veya etkin plugin sürümü farklılaşabilir.

Önerim: onay kaydı oturum, tur, çağrı kimliği, araç/plugin sürümü, kanonik hedef, argüman özeti/hash’i, dosyanın beklenen eski içerik hash’i ve politika sürümüne bağlansın. Bunlardan biri değişirse onay geçersiz olsun. Onay tek kullanımlık olmalı; iptal, süre dolumu veya oturum değişikliği onu düşürmeli.

“Bu araca hep izin ver” yerine, kullanıcıya anlaşılır dar kapsamlar sunulabilir: “Bu oturumda yalnızca `src/` altındaki metin dosyalarını düzenleyebilir.” Araç adına verilen genel izin, aracın sonraki tüm argümanlarına izin anlamına gelmemeli.

### 3.4 Güvenlik denetçisi ek savunma olmalı; izinlerin yerine geçmemeli — P0

§9’daki sınıflandırıcı yararlı olabilir; fakat yanlış negatif, yanlış pozitif, zaman aşımı ve bozuk JSON durumları var. Ayrıca dış kaynak yalnızca dosya veya kabuk çıktısı değildir: plugin araç açıklaması, arama sonucu, RAG parçası ve önceki içeriğin özeti de talimat taşıyabilir. Katmanlı savunma ve araçlara en az yetki verme yaklaşımı, OWASP’ın da önerdiği temel çerçevedir. [OWASP: Prompt Injection Prevention](https://cheatsheetseries.owasp.org/cheatsheets/LLM_Prompt_Injection_Prevention_Cheat_Sheet.html)

Kalfa için önerim: LLM denetçisi kapalıyken de dosya sınırı, veri çıkışı politikası, sır koruması, argüman doğrulama ve yürütme onayları çalışsın. Dış içerik kaynak ve güven etiketiyle taşınsın; özetlenince güvenilir sistem talimatına dönüşmesin.

“Araç çıktısıyla eşzamanlı” ile “modele geri beslenmeden önce” ifadeleri de kesinleştirilmeli. Katı kipte inceleme sürerken içerik karantinada tutulmalı; onaylanmamış parçalar ana modele akmamalı. Denetçi erişilemezse durum `bilinmiyor` olmalı ve katı kip güvenli durmalı; sessizce kapalı kipe geçmemeli. Uyar kipindeki başarısız denetim de görünür olmalı.

### 3.5 Kredi modelinde kesinleşme iddiası kanıtı aşıyor — P1

§5.2 bloklu yanıtın `usage.evren` alanını `Settled` sayıyor. İncelenen [bloklu fixture](../../../research/evren-probe/fixtures/chat_tool_calls_blocking.json) ise `credits_held_cr`, `credits_remaining_cr` ve `routed_model` içeriyor; ayrı bir kesinleşmiş tahsilat alanı içermiyor. [Probe README](../../../research/evren-probe/README.md) başlıkların kaydedilmediğini de belirtiyor.

Dolayısıyla bu kayıt, tek başına kesinleşme anını veya bir sonraki isteğin bakiyesinin önceki tüm işlemleri kapattığını kanıtlamıyor. Paralel istekler, embedding çağrıları, başka istemciler ve `billing_pending` varken bakiye farkından oturum maliyeti çıkarmak özellikle riskli.

Önerim: sağlayıcı sözleşmesi doğrulanana kadar `son gözlenen bakiye`, `tutulan kredi`, `doğrulanmış harcama` ve `mutabakat bekliyor` durumlarını ayırmak. CR değerlerini sabit hassasiyetli ondalık türle tutmak; guard, özetleme ve RAG çağrılarını da istek kimliğiyle izlemek. `reasoning_tokens` toplam completion sayısının içinde olabilir; maliyete ikinci kez eklenmemeli.

### 3.6 SSE uyumsuzluğu gerekçesi fazla geniş — P1

§3.3’teki `:` yorum satırları SSE standardının parçasıdır. Parçalı tool argümanları da OpenAI’nin streaming function calling akışında bulunur. Bunlar tek başına hazır istemci veya SSE ayrıştırıcısı kullanmayı imkânsız kılmaz. EVREN’e özel adaptör yine anlamlıdır; düşük seviyeli framing’i elle yazma kararı ayrıca gerekçelendirilmelidir. [WHATWG SSE](https://html.spec.whatwg.org/multipage/server-sent-events.html#parsing-an-event-stream), [OpenAI function calling](https://developers.openai.com/api/docs/guides/function-calling)

Önerim: SSE çerçeveleme, JSON olay ayrıştırma ve EVREN alan eşlemesini ayrı katmanlar yapmak. Testler TCP parçalarını rastgele bayt sınırlarından bölmeli; çok satırlı `data:`, CRLF, eksik son olay, aşırı büyük olay, iç içe araç indeksleri ve bağlantı kopması senaryolarını içermeli. Araç argümanı tamamlanmadan ve şema doğrulanmadan araç çalışmamalı.

[Mevcut stream fixture’ında](../../../research/evren-probe/fixtures/chat_tool_calls_stream.sse) `I` ve `stanbul` ayrı argüman parçaları; bu, çok baytlı Türkçe karakterin TCP bayt sınırında bölündüğünü kanıtlamıyor. Ayrıca ilk tool parçasında `id` ve `name` var, argüman bir sonraki parçada başlıyor. Test beklentisi fixture’ın gerçekten gösterdiği davranışa dayanmalı.

### 3.7 Model yetenekleri model bazında doğrulanmalı — P1

[JSON şema fixture’ı](../../../research/evren-probe/fixtures/chat_json_schema.json) `zai/glm-5.3-fp8` modelinden geliyor. §9’da denetçi için seçilen `qwen3.8-flash-next` üzerinde aynı şema davranışı bu fixture ile kanıtlanmış değil. Tek bir modelde başarılı tool çağrısı da tüm katalog için garanti vermez.

Önerim: model başına tools, streaming tools, structured output, reasoning ve bağlam sınırı tablosu tutmak. Bilgiye gözlem tarihi ve kanıt eklemek; bilinmeyen yetenekleri destekleniyor saymamak. Sağlayıcı model kaldırdığında profil anlaşılır hata vermeli; özellikle denetçi için sessiz model değişimi yapılmamalı. Yeni canlı doğrulamalar ayrı, açıkça başlatılan bir test akışı olmalı.

### 3.8 WIT sözleşmesi RAG hedefini henüz tam taşımıyor — P1

§8.6 ilerleme bildiriminden söz ediyor, ancak §8.2’de tipli ilerleme, iptal veya iş durumu arayüzü yok. Dosya okuma var fakat dosya keşfi/listeleme ve kalıcı indeks saklama modeli belirtilmemiş. `gunluk` çağrısı tek başına ilerleme sözleşmesi yerine geçmez.

Önerim: import/export’ları bağlayan bir `world` tanımı, çağrı kimliğiyle ilerleme, iptal semantiği ve yapılandırılmış hata türleri eklemek. RAG için v1’de host’un izinli dosya listesini sağlaması, indeksi ise workspace/plugin bazında ayrılmış host deposunda tutması düşünülebilir. Böylece genel dosya yazma yetkisi açmadan kullanım senaryosu taşınabilir. WIT dünyaları import ve export sözleşmesini bu amaçla tanımlar. [Component Model: Worlds](https://component-model.bytecodealliance.org/design/worlds.html)

WIT fonksiyon sınırını tiplendirir; `argumanlar: string` içindeki JSON’u veya `parametre-semasi: string` içeriğini derleme zamanında doğrulamaz. Host bunları çalıştırma öncesinde doğrulamalı. Ayrı üretilmiş plugin’in host’a uyumu da yükleme/bağlama aşamasında kontrol edilmeli. “Bütün tip uyuşmazlıkları derleme zamanında yakalanır” iddiası daraltılmalı.

### 3.9 Paralellikte okuma-yazma ilişkisi eksik — P1

§6.1 yalnızca yazma araçlarını aynı alanda seri yapıyor. Bir dosyayı okurken başka araç onu değiştirirse, model karışık bir görünüm alabilir. Tek `option<string>` çakışma alanı, birden çok dosya değiştiren araç ve yan etkileri bilinmeyen kabuk için de yetersiz kalabilir.

Önerim: çağrı bazında okuma/yazma kaynak kümeleri veya sürümlenmiş snapshot kullanmak. Bilinmeyen yan etkili kabuk çağrıları başlangıçta workspace genelinde özel kilit alabilir. Plugin’in beyan ettiği alan, host dosya izinlerinin veya güvenlik denetiminin yerine geçmemeli. Kısmi başarısızlık ve seçili onayda her çağrının sonucu ayrı dönmeli; reddedilen çağrı başarılı gibi kaydedilmemeli.

### 3.10 Akış ve kalıcılık hedefleri ölçülebilir olmalı — P1

§11.1’de 16 ms pencere ile “50 olay/sn → 3 olay/sn” sonucu matematiksel olarak çıkmıyor. 16 ms, yaklaşık 62,5 gönderim fırsatı/sn demektir; düzenli 50 parça/sn akışında çoğu pencere tek parça taşıyabilir. Önerim: gecikme ve kuyruk boyutu hedeflerini yazıp ölçmek; toplama penceresini ölçüme göre seçmek.

Sanallaştırma da (§11.2) toplam belleği otomatik sabitlemez; mesaj kayıtları, render önbelleği ve ajan bağlamı büyüyebilir. Sınırlı kuyruk, diskten sayfalama ve arka basınç politikası gerekir. Metin deltaları birleştirilebilir; onay, hata ve tamamlanma olayları düşürülmemeli.

`Command`/`Event` sınırına `session_id`, `turn_id`, `call_id`, olay sıra numarası ve şema sürümü eklenmesini öneririm. Yeniden açılışta durum yeniden kurulabilmeli; yarıda kalmış yan etkili araç otomatik tekrar çalıştırılmamalı. UI’ın çekirdeği bloklamaması, onay bekleyen işlemin beklemeden devam etmesi anlamına gelmemeli: tur güvenli duruma geçerken diğer oturumlar ve iptal komutu yanıt verebilmeli.

## 4. Güvenlik için ek öneriler

### 4.1 Tehdit kapsamını bir sayfada tanımlamak

Korunan varlıklar: API anahtarı, kaynak kodu, proje dışı dosyalar, özel veriler, kullanıcı onayları, EVREN kredisi ve oturum kayıtları.

Güvenilmeyen girdiler: klonlanan depo, dosya/terminal içeriği, plugin paketi ve metadata’sı, model yanıtı, uzak servis çıktısı. Kötü niyetli repo ve plugin tasarımın kapsamına alınmalı. İşletim sisteminin veya yönetici hesabının tamamen ele geçirilmesine karşı hangi garantilerin verilmediği de belirtilmeli.

“Yurt içinde işleniyor” ifadesinden otomatik olarak “bu projedeki her veriyi göndermek uygundur” sonucu çıkarılmamalı. Kurumsal kullanım için veri sınıflandırması, saklama ve paylaşım kuralları ayrıca belirlenmeli.

### 4.2 İzin politikası tek yerde uygulanmalı

Etkin yetkiyi şu kesişim olarak düşünürdüm: uygulamanın sunduğu yetenekler ∩ plugin beyanı ∩ kullanıcının verdiği izin ∩ workspace/kurum politikası. Bunların üstünde açık yasaklar bulunmalı. Manifest yetki talebidir; tek başına yetki vermez.

Her host çağrısı bu kapıdan geçmeli. Model çıktısı ve plugin, izin kipini veya kurum politikasını değiştirememeli. Repo içindeki yapılandırma dosyaları da güvenlik politikasını gevşetememeli. Serbest kip, kullanıcı tarafından konulan açık yasakları kaldırmamalı.

### 4.3 Yol güvenliğini kanonikleştirmenin ötesine taşımak

§7.2 iyi bir başlangıç. Ek olarak kontrol ile kullanım arasında symlink/junction değiştirilmesi, Windows reparse point’leri, UNC/device yolları, sürücü göreli yollar, alternatif veri akışları, harf duyarlılığı farkları ve henüz mevcut olmayan dosyalar ele alınmalı.

Önerim: yalnızca metinsel yol öneki karşılaştırmamak; mümkün olan platformlarda doğrulanmış dizin tutamacına göre erişim yapmak ve yarış koşullarına dayanıklı açma/yazma yöntemleri kullanmak. Yeni dosyada üst dizin zinciri de doğrulanmalı. Hard link senaryoları ayrıca düşünülmeli; aynı dosya nesnesine workspace dışında da ad verilebilir. Belirsiz durumda işlem reddedilmeli veya açık kapsamlı onay istenmeli.

### 4.4 Kabuk için ayrı yürütme politikası

Basit araçlar mümkünse shell metni yerine program + argüman dizisi ile çağrılsın. Genel shell gerekiyorsa tam komut, çalışma dizini ve izolasyon durumu onayda görülsün. Komut öneki allowlist’i, script’in davranışını garanti ediyor gibi sunulmasın.

Alt sürece yalnızca gerekli ortam değişkenleri aktarılsın; EVREN anahtarı, token’lar ve gereksiz ajan soketleri miras bırakılmasın. Süre, çıktı boyutu ve alt süreç sayısı sınırı olsun. İptal tüm süreç ağacını kapsasın; iptal sonrası yapılmış dosya/ağ etkilerinin otomatik geri alınamadığı açık olsun. Proje betiği çalıştırmak, proje içeriğini kod olarak güvenmek anlamına geldiğinden ilk çalıştırmada görünür karar olmalı.

### 4.5 Anahtar ve ağ sınırları

Anahtar sağlayıcı katmanında tutulmalı; modele, plugin’e, frontend durumuna, loglara veya hata gövdelerine taşınmamalı. OS kasasında saklama zaten doğru karar; alt süreçler ve tanılama çıktıları da aynı korumaya dahil edilmeli.

`evren-cagir(uc, govde)` serbest URL veya HTTP vekili olmamalı. `uc`, izinli işlem enum’una çözümlenmeli; host sabit hedefi ve yöntemi üretmeli. Şart kabulü, hesap yönetimi veya key işlemleri plugin’e açılmamalı. TLS doğrulaması kapatılamamalı; yönlendirme başka hedefe anahtar taşımamalı.

Genel ağ aracı eklenirse HTTPS/hedef/port kısıtları, yönlendirmede tekrar denetim ve loopback/özel ağ erişimi için ayrı yetki düşünülmeli. Yalnızca host adına bakmak DNS değişimleri ve yönlendirmeler karşısında yeterli politika değildir.

### 4.6 Tauri ve Markdown yüzeyi

§11.8’de şart metni için uzak görsel yasağı var; bu yaklaşım sohbet, reasoning, araç çıktısı ve plugin açıklamalarına da uygulanmalı. Ham HTML temizlenmeli veya kapatılmalı; `javascript:`, tehlikeli özel protokoller ve otomatik uzak görsel yükleme engellenmeli. Bağlantı açma kullanıcı eylemi olmalı; yerel asset erişimi de dar kapsamlı tutulmalı.

Tauri CSP’si açıkça yapılandırılmalı; uzaktaki script/font gibi kaynaklara ihtiyaç en aza indirilmeli. Tauri belgeleri CSP korumasının yapılandırıldığında etkinleştiğini belirtiyor. [Tauri CSP](https://tauri.app/security/csp/)

Özel uygulama komutlarının varsayılan erişimine güvenilmemeli. Tauri belgelerine göre `invoke_handler` ile kayıtlı uygulama komutları, ayrıca sınırlandırılmadığında tüm uygulama pencereleri/webview’ları tarafından kullanılabilir. Pencere bazlı izinler ve Rust tarafında çağrı doğrulaması gerekir. [Tauri capabilities](https://tauri.app/security/capabilities/)

Bu nedenle “ajan Command üretmez” (§11.8), şart kabulünü tek başına güvenceye almaz: güvenilmeyen içerik aynı yetkili webview’da script çalıştırabilirse IPC üzerinden komut isteyebilir. Şart kabulü ve izin artırımı güvenilir UI akışına bağlanmalı; beklenen durum, hesap ve gösterilen şart sürümü backend’de doğrulanmalı.

### 4.7 WASM için kaynak ve paket sınırları

Dosya ve soketleri kapatmak CPU, bellek veya kredi tüketimini sınırlamaz. Wasmtime fuel/epoch kesmesi ve bellek/tablo/instance limitleri değerlendirilmeli. Wasmtime’ın `ResourceLimiter` dokümanı CPU için ayrıca fuel veya epoch gerektiğini belirtiyor. [Wasmtime interruption](https://docs.wasmtime.dev/examples-interrupting-wasm.html), [ResourceLimiter](https://docs.wasmtime.dev/api/wasmtime/trait.ResourceLimiter.html)

Kalfa’ya özel önerim: host HTTP/dosya çağrılarına ayrı timeout, yanıt ve log boyutu sınırı, eşzamanlılık sınırı ve EVREN çağrı bütçesi koymak. WASM yakıtı, host fonksiyonunun kendi maliyetini otomatik sınırlamaz. Plugin çökmesi veya limit aşımı tüm uygulamayı sonlandırmamalı. İzin verilmeyen import taşıyan paket yüklenmemeli.

Plugin kurulumu sırasında paket hash’i/sürümü, kaynak ve yetenekler gösterilmeli; güncellemede yetki artışı yeniden onaylanmalı. Arşivden kurulum varsa yol kaçışı ve açılmış boyut sınırı denetlenmeli. İmza, yayıncı kimliği ve paket bütünlüğüne yardımcı olur; plugin davranışının güvenli olduğunu kanıtlamaz.

### 4.8 Oturum, günlük ve RAG verisini korumak

Kalıcı veriyi mümkünse repo dışında, kullanıcıya özel uygulama veri dizininde tutmak iyi olur. Dosya izinleri, saklama süresi, dışa aktarım ve silme davranışı tanımlanmalı. Hassas kullanım için kayıtsız oturum seçeneği düşünülebilir; yerelde kayıt tutmamak sağlayıcının kayıt politikasını değiştirmez.

Loglar varsayılan olarak olay kimliği, araç adı, karar, hata türü ve istek kimliğini içersin; ham prompt, sır ve tam dosya içeriği otomatik kaydedilmesin. Reasoning paneli de hassas metin içerebilir. Hata raporu paylaşılmadan önce kullanıcıya temizlenmiş önizleme sunulsun.

RAG indeksleri workspace ve plugin’e göre ayrı tutulmalı; kaynak hash’i ve chunk kökeni korunmalı. Dosya silinince veya erişim izni kalkınca indeks/kopyalar da geçersizleştirilmeli. `.gitignore` yardımcı filtre olabilir ama güvenlik politikası yerine geçmez. Embedding ve indeksler de türetilmiş hassas veri olarak ele alınmalı.

### 4.9 Kaynak, maliyet ve tekrar döngülerini sınırlamak

Tur başına azami araç çağrısı, toplam süre, çıktı baytı, model turu ve eşzamanlılık sınırı tanımlanmalı. Aynı hatalı çağrının tekrarı döngüyü durdurmalı. Guard, özetleme ve embedding işlemleri de bütçeye dahil edilmeli.

`Retry-After` üst sınırı, iptal edilebilir bekleme ve sınırlı yeniden deneme tanımlanmalı. Zaman aşımı, yan etkili bir aracın hiç çalışmadığı anlamına gelmez; sonucu bilinmeyen yazma/komut otomatik tekrarlanmamalı. Kesin fiyat veya tahsilat doğrulanamıyorsa kullanıcıya kesin CR harcama garantisi yerine denetlenebilir çağrı/token/süre sınırı sunulmalı.

### 4.10 Dağıtım güvenliği

İmzasız v1 geliştirme denemeleri için kabul edilebilir bir kapsam tercihi; kurumsal dağıtım hedefiyle arasındaki fark görünür olmalı. Yayınlanan dosyalar için bağımsız doğrulanabilir bütünlük bilgisi, bağımlılık envanteri, sürüm sabitleme, açık takibi ve güvenlik bildirim kanalı öneririm. Kod imzalama ve güvenli güncelleme sonraki dağıtım kapısına alınabilir. Aynı ele geçirilmiş indirme kanalındaki hash tek başına yayıncı doğrulaması sağlamaz.

## 5. Ürüne değer katacak ek fikirler

| Fikir | Kullanıcıya faydası | Önerdiğim zaman |
|---|---|---|
| Salt okunur inceleme kipi | Depoyu tanıtır, plan ve öneri üretir; yazma veya kabuk çalıştırma yetkisi vermez | v1 |
| Gönderilecek bağlam önizlemesi | Hangi dosya parçalarının hangi işlem/model için gönderildiğini açıklar; hariç tutmaya izin verir | v1 |
| Değişiklik özeti ve güvenli geri alma | Turun dosya etkilerini gösterir; yalnızca Kalfa’nın değişikliklerini geri almayı dener, kullanıcı değişmişse çakışmayı bildirir | v1 temel düzey |
| Bütçe ve durdurma paneli | Uzayan döngüyü, paralel işleri ve belirsiz maliyeti görünür kılar | v1 |
| Oturum kurtarma | Çöküşten sonra konuşmayı ve tamamlanan araçları geri getirir; belirsiz yan etkileri yeniden çalıştırmadan gösterir | v1 |
| Kaynaklı RAG yanıtı | Dosya/satır/chunk ve içerik sürümüyle cevabın dayanağını açar | RAG ile birlikte |
| “Neden izin gerekiyor?” açıklaması | Araç adı yerine hedef, etki, veri çıkışı ve kapsamı anlatır | v1 |
| Güvenli destek paketi | İstek kimliklerini ve sürüm bilgisini sırları temizleyerek dışa aktarır | v1 sonrası |
| Yerel proje profilleri | Dil, test komutu ve tercihleri hatırlar; repo profili kullanıcı güvenlik politikasını gevşetemez | v1 sonrası |
| Kurumsal politika profili | İzinli modelleri, plugin’leri, ağ hedeflerini ve saklama davranışını merkezi sınırlar | Kurumsal sürüm |
| Türkçe görev değerlendirme seti | Kod değişikliği başarısını, RAG doğruluğunu, gereksiz onayı, gecikme ve maliyeti ölçer | Erken başlanmalı |

Türkçe deneyimde yalnızca çeviriyle yetinmezdim: anlaşılır hata dili, klavye kullanımı, ekran okuyucu desteği, diff okunabilirliği ve izin gerekçeleri ürün kalitesini belirler. `reasoning` paneli modelin sunduğu metin olarak sunulmalı; doğrulanmış açıklama veya işlemlerin gerçek denetim kaydı gibi kullanılmamalı.

## 6. Eklenmesini önerdiğim kabul testleri

Bu liste §13’ün yerine geçmez; eksik kalan davranışları somutlaştırır. Tablo testlerin geçtiğini değil, geçmesi gereken koşulları ifade eder.

| Senaryo | Beklenen sonuç |
|---|---|
| Proje dosyası başka bir dosyadaki sırrı göndermeyi telkin ediyor | Dosya/çıkış politikası engeller; LLM denetçisi kapalıyken de sınır korunur |
| Hassas içerik denetçiye gönderilmek üzere seçiliyor | Yerel veri çıkışı kontrolü uzak denetçi çağrısından önce çalışır |
| Kullanıcı diff’i onayladıktan sonra dosya değişiyor | Eski onayla yazılmaz; çakışma ve yeni diff gösterilir |
| Yol kontrolünden sonra symlink/junction hedefi değişiyor | Onay dışı dosyaya erişilemez |
| Shell alt süreç doğuruyor ve tur iptal ediliyor | Süreç ağacı durur; kalan/gerçekleşmiş etkiler raporlanır |
| Plugin sonsuz döngüye giriyor veya çıktı şişiriyor | Limit devreye girer; UI ve diğer oturumlar yanıt vermeye devam eder |
| Plugin izinli EVREN yetkisiyle şart kabul ucunu istiyor | Host reddeder; key veya genel HTTP erişimi verilmez |
| Katı denetçi timeout veya bozuk JSON üretiyor | İncelenmemiş içerik ana modele gitmez; durum görünür kalır |
| Markdown script, uzak görsel veya tehlikeli URL içeriyor | Script/otomatik ağ isteği/ayrıcalıklı IPC çağrısı oluşmaz |
| UTF-8/SSE akışı her olası bayt sınırında bölünüyor | Aynı içerik elde edilir veya tipli eksik-akış hatası çıkar; yarım araç çalışmaz |
| Aynı kaynağa okuma ve yazma aynı anda geliyor | Tanımlı snapshot/kilit davranışı korunur |
| Yazma tamamlandıktan sonra kayıt öncesi süreç çöküyor | Yeniden açılışta işlem körlemesine tekrarlanmaz; belirsizlik gösterilir |
| Şart sürümü kullanıcı okurken değişiyor | Eski sürüm kabul edilmiş gibi devam edilmez; güncel metin gösterilir |
| Oturum Serbest kipte kapatılıp yeniden açılıyor | Geçici izinler geri yüklenmez; bekleyen onaylar yeniden kullanılamaz |
| Paralel model/RAG çağrıları farklı bakiye gözlemleri dönüyor | İstekler ilişkilendirilir; toplam maliyet kesinmiş gibi türetilmez |
| Qwen denetçisi beklenen şemayı üretmiyor | Başarılı sınıflandırma sayılmaz; açık hata/güvenli duruş oluşur |

Gerçek fixture’ları korurken sentetik hata ve saldırı örnekleri de eklerdim. Gerçek kayıt mutlu yolu kanıtlar; henüz gözlemlenmemiş sınır koşulları için kontrollü test verisi gerekir. Parser fuzzing’i ve yol/onay özellik testleri burada anlamlı olur.

## 7. Ben olsam hangi sırayla ilerlerdim?

**Önce tasarımı netleştirme:** Kabuk izolasyonu iddiası, veri çıkışı kapısı, onay bağlama, IPC yetkileri ve denetçi başarısızlığı semantiği. Bunlar P0; sonradan eklenmeleri mimariyi değiştirebilir.

**Ardından küçük uçtan uca sürüm:** Şart akışı, EVREN adaptörü, tek ajan döngüsü, dosya okuma/düzenleme, diff onayı, iptal ve kalıcı oturum. Model yetenekleri, kredi belirsizliği ve kaynak limitleri bu aşamada görünür olmalı.

**Sonra plugin ve RAG:** Tamamlanmış WIT sözleşmesi, kaynak limitleri, host veri çıkışı denetimi ve kaynaklı cevaplar üzerinden ilk plugin. Mevcut v1 kapsamı korunacaksa bile bu sırada teslim etmek entegrasyon riskini azaltır.

**Daha sonra MCP ve kurumsal dağıtım:** Süreç izolasyonu, paket güveni, politika dağıtımı ve denetim ihtiyaçları ölçülerek genişletilmeli. MCP’nin bir `Tool` adaptörüne sarılabilmesi, süreç yaşam döngüsü ve güvenlik işinin de mekanik olduğu anlamına gelmez.

Benim ürün açısından ana önerim: Kalfa’nın ayırt edici vaadini “Türkçe çalışan ajan”ın ötesine, **“hangi veriyi kullandığını, neyi değiştirdiğini ve hangi yetkiyle çalıştığını anlayabildiğim ajan”** noktasına taşımak. Spec’in güçlü tarafları bu vaadi destekliyor; yukarıdaki sınırlar tamamlandığında daha güvenilir bir temele oturur.
