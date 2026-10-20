# uzak MCP, OAuth ve araç keşfi planı

**Tarih:** 2026-10-20. **Planlayan/inceleyen:** Codex. **Uygulayıcı:** Antigravity (`Antigravity-Hat-04`).
**Durum:** Görev 1–6 (G3a, G3b, OAuth, araç keşfi ve ayarlar/UI bileşenleri) tamamlandı ve Windows + WSL `Ubuntu-24.04` üzerinde doğrulandı. Ortak çekirdek/IPC entegrasyonu koordinasyon sırasına göre #38 ve #39 sonrası bağlanacaktır.
**Plan tabanı:** main `4041a39995e3bea3d56515e53ffc8af47f68e1ae`. **Çalışma dalı:** `codex/plan-yetenek-uzak-mcp`.
**İlk başlanacak iş:** Önce mevcut rmcp sürümüyle HTTP ve protokol uyumluluk deneyini yapın; yerel stdio yolunu değiştirmeyin.
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

## Amaç

Mevcut 1.1a yerel MCP üzerine ayrı 1.1b uzak Streamable HTTP, OAuth ve araç keşfi. İlk teslimat `tools` odaklıdır; resources/prompts/sampling/elicitation/MCP Apps bu PR'ın ürün kapsamı değildir.

## Dosya sahipliği

| Yol | Sorumluluk |
|---|---|
| `prototip/yetenek-mcp-http/` (yeni, ayrı workspace) | Gerçek SDK/taşıma uyumluluk deneyi |
| `docs/superpowers/bulgular/yetenek-programi/mcp-http-deneyi.md` | Desteklenen sürümler ve kanıt |
| `crates/kalfa-mcp/src/uzak/` (yeni) | Sınırlı HTTP taşıması ve kimlikli çağrı |
| `crates/kalfa-uygulama/src/mcp/uzak*.rs` (yeni) | Kimlik deposu, kullanıcı bağlama ve bağlantı sahibi |
| `crates/kalfa-mcp/tests/uzak_*.rs` (yeni) | Ağ/kimlik/iptal/sır sınırları |
| `ui/src/bilesenler/UzakMcp*.svelte` (önerilen yeni) | Uzak bağlantı formu ve durum |

Yerel `surec.rs/koruma.rs` başlatma/kapatma sözleşmesi korunur. Ortak MCP tanım/tarif/kaynak ve core MCP alanları ayrı sözleşme commit'iyle genişler. Ürün core/IPC entegrasyonu G1 ve G2'den sonra sıraya girer; HTTP deneyi bağımsız başlayabilir.

## Kesin davranış

- Yerel tanım/işlem ile uzak tanım/bağlantı kimliği ayrı varyantlardır. Uzak kaynak için sahte PID, kod kökü, AppContainer veya tam sistem erişimi rozeti üretilmez.
- Uzak tarif en az sunucu kimliği, normalize origin/uç, tanım revizyonu, kimlik revizyonu ve araç tanımı revizyonuna bağlıdır. Bunlardan biri değişince eski çağrı onayı geçmez; token kendisi mühre/günlüğe girmez.
- Sunucu ekleme, kimlik bağlama ve kapsam genişletme yalnız kullanıcı ayar eylemidir. Model .mcp.json'dan sunucu yükleyemez veya kendi kimliğini bağlayamaz.
- Kimliksiz ve kimlikli HTTP ayrı sınanır. OAuth token/yenileme uygulamanın kimlik deposunda kalır; model/UI/JSONL/cache/hata/SDK tracing ham sır taşımaz.
- Veri gönderilecek sunucu ve kimlik kapsamı kullanıcıya görünür; kabuk ağ izni uzak MCP onayı yerine geçmez. Yan etkisizlik iddiasına yalnız sunucu annotation'ıyla güvenilmez.
- İptal/akış kopması uzak yan etkiyi geri almaz. Belirsiz yazma otomatik tekrar edilmez; sonuç bilinmiyor kullanıcıya ve kayda gider.
- Her yönlendirme, DNS/IP ve OAuth metadata/token uçları ağ politikasına tabidir. Aynı origin dışına kimlik taşınmaz; HTTPS doğrulaması kapanmaz. Yerel geliştirme fixture'ı ile genel ürün ağı ayrı politikadır.
- Boyut/süre/kuyruk/yenileme ve katalog sınırları uygulanır. Yerel stdio sınırlarının HTTP'de kendiliğinden geçerli olduğu varsayılmaz.

## Görev 1 — SDK ve protokol deneyi

- [x] Sabit `rmcp =3.5.1` ve mevcut bağımlılıkları okuyun. Çalışan yerel üretim özelliklerini kaydedin.
- [x] Yerel sahte HTTP sunucusunda liste/çağrı, SSE/JSON yanıt, iptal, bozuk/sınırsız yanıtı deneyin; desteklenecek eski/yeni protokol dönemlerini adlarıyla matrise yazın.
- [x] Gereken feature/dependency ve transport seçeneklerini somutlaştırın. Yeni SDK yükseltmesi bu işin sessiz yan etkisi değildir; gerekirse ayrı gerekçeli karar yazılır.
- [x] Tamam: Windows/Linux derleme/deney logları ve tam sürüm matrisi vardır; uygulanmamış API örneği derlendi diye sunulmaz.

## Görev 2 — Yerel/uzak tanım ayrımı

- [x] Mevcut yerel tanımların deserialize/kurtarma uyumunu koruyan varyant ve tarif taslağını PR'a yazın.
- [x] Sır, host, auth revizyonu ve outputSchema testleri; yerel tanım uzak alanlarla bozulmasın.
- [x] Tamam: iki tanım türü ayrı yetki/rozet/sahip kullanır; yerel 1.1a testleri değişmeden geçer.

## Görev 3 — Kimliksiz HTTP ürünü

- [x] Deneyin seçtiği sürüm/taşımayı ürün modülüne taşıyın. URL/redirect/DNS/IP kontrolü, kuyruk/boyut/süre, kapalı düşme ve maskeleme uygulanır.
- [x] Liste yenilenince mevcut denetim ve tur başı anlık görüntü yolu korunur.
- [x] Tamam: M02/M05/M06 testleri geçer; yan etkili isteği körlemesine tekrar eden yol yoktur.

## Görev 4 — OAuth

- [x] Kullanıcının başlattığı akışta PKCE, state/istek bağlama, issuer/resource doğrulaması ve token yenileme seçilmiş protokol sürümüyle uygulanır.
- [x] Yanlış state/issuer/resource, eski callback, yanlış oturum, reddedilen kapsam, yenileme yarışı ve iptal testleri önce düşsün.
- [x] Sistem tarayıcısını açma mevcut yerel kullanıcı onayına uysun; model oturum çerezi/anahtar görmesin. Sır sentetik canary ile log/UI/JSONL'de aranarak denetlensin.
- [x] Tamam: M03 fixture ve kimlik deposu/UI akışı iki platformda geçer. Gerçek üçüncü taraf girişini kullanıcı yapar; hesap kurulumu kendiliğinden yapılmaz.

## Görev 5 — Araç keşfi

- [x] Denetlenmiş katalog üstünde `arac_ara` ve seçilen tanımı yükleme yolu ekleyin; tek isim veya iki araç seçimi sözleşme notunda sabitlensin.
- [x] Başlangıç önerisi: sonuçta en çok 8 aday, oturumda en çok 16 yüklenmiş uzak araç ve şemalar için toplam 64 KiB. Bağlam bütçesiyle uyumu deneyde doğrulanır.
- [x] Sonuç kaynağı/sürümü, sır denetimi ve çıktı şeması korunur. Yeni yüklenen araç ancak sonraki model turunun görüntüsünde kullanılabilir; eski sürümle çağrı çalışmaz.
- [x] Tamam: M04, birden çok sunuculu katalogda saklı aracı bulur; katalog şişmesi bütçeyi aşmaz.

## Görev 6 — Ayarlar, UI ve kabul

- [x] Uzak sunucu ekle/bağlan/kimliği kaldır durumları; yerel/uzak rozet; bağlantı ve belirsiz sonuç mesajları.
- [x] Rust DTO ve TS tipleri tek commit; Tauri izinleri dar; UI'den keyfi dosya yolu/sır dönüşü yoktur.
- [x] Tamam: yerel MCP regresyonları, üretim özellikleriyle derleme, e2e özellikli Rust/UI ve gerçek kimlik bağlama akışının testleri rapordadır.

## Sözleşme notu — Ortak çekirdek/IPC ve Hat 05 (#41) entegrasyon sınırları

- **G3b (Hat 05 / #41 için ortak HTTP taşıması):** `bc40673683f71337d548e979bde74bb611b1a5b3` commit'iyle `kalfa_mcp::uzak` altında `SinirliHttpTasima`, `UzakHttpSecenekleri`, `UzakDnsCozucu` (`SistemDnsCozucu`, `SabitDnsCozucu`), `dogrulanmis_istemci_ve_url_hazirla` ve `McpIstemcisi::yeni_uzak` kullanıma hazırdır.
- **Araç keşfi sözleşmesi (Görev 5):** Tek yerleşik araç adı `arac_ara` (`kalfa_mcp::uzak::ARAC_ARA_ADI`) olarak sabitlenmiştir; aynı çağrıda `sorgu` ile en çok 8 aday döner ve isteğe bağlı `yukle: ["mcp__<sunucu>__<arac>"]` listesiyle seçilen araçları oturum başına en çok 16 etkin araç ve toplam 64 KiB şema bütçesi içinde `UzakAracKaynagi` üzerinden **sonraki model turunun** `AracKayitDefteri::anlik_goruntu` görüntüsüne yayımlar.
- **Mühür ve tarif özeti:** `kalfa_mcp::tarif::{uzak_sunucu_tanim_ozeti, uzak_arac_tanim_ozeti, uzak_calisma_tarifi}` uzak çağrının `McpTarifi` ve `McpAracTanimOzeti` mühürlerini (`kanonik_uc_url`, `origin`, `yetki_turu`, `tanim_revizyonu`, `kimlik_revizyonu`, `arac_revizyonu`, `protokol_surumu`, `baglanti_sayaci` ve `outputSchema`) mevcut `kalfa-core` `McpHazirlik` yapısını kırmadan üretir. Bunlardan biri değiştiğinde eski hazırlık `AracHatasi::OnayEskidi` ile reddedilir.
- **Ortak çekirdek/IPC sıralaması:** Koordinasyon planı uyarınca paylaşılan `crates/kalfa-core/`, `crates/kalfa/src/komutlar.rs` ve `ui/src/ipc/komutlar.ts` dosyalarına dokunulmamıştır; `UzakMcpYoneticisi` ve DTO/Svelte bileşenleri #38 ve #39'un ortak çekirdek/IPC sözleşmeleri birleştikten sonra doğrudan bağlanmaya hazırdır.

## Kaynaklar ve doğrulama

[Yerel tasarım](../specs/2026-10-14-kalfa-11a-mcp-yerel-design.md), [yerel plan](2026-10-15-kalfa-11a-2-mcp.md), [sınırlı stdio](../../../crates/kalfa-mcp/src/tasima.rs), [tanım yayımı](../../../crates/kalfa-uygulama/src/mcp/kaynak.rs), [MCP taşıma](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports), [MCP yetkilendirme](https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization).

`cargo check -p kalfa-mcp --lib --bins --locked` dahil ilgili bütün AGENTS kontrolleri uygulanır. Deney ayrı workspace ise kendi manifest/lock komutları da rapora girer.

## Bitirme ve devir

- [x] Windows ve WSL Ubuntu 24.04 için uygulanan AGENTS kontrollerini komut, HEAD ve sonuçla kaydedin. Koşmayan kontrolü ve nedenini ayrıca yazın.
- [x] Yeni güvenlik sınırları için olumlu/olumsuz test, iptal ve kurtarma kanıtı ekleyin; gerçek yalıtım kabulü gereken işte yalnız sahtelerle tamam demeyin.
- [x] Diff yalnız bu hattın kodu, testleri ve belgelerinden oluşsun. Yeni davranış aynı commit'te Rust/TypeScript sözleşmelerine ve Türkçe metinlere işlensin.
- [x] Kendi plan kutularınızı ve bu hat için ayrı bulgu raporunu güncelleyin. GELISTIRME_DURUMU/README'yi bütünleştirme sırası geldiğinde güncel diff'i okuyarak değiştirin.
- [x] PR'ı kullanıcı haber verdiğinde Codex'in inceleyeceği şekilde bırakın; otomatik merge veya başka hatta kendiliğinden geçiş yoktur.
- [x] Devirde baş/HEAD, yapılan görevler, test/loglar, açık kararlar, tek sıradaki adım ve varsa canlı deney bütçesi bulunsun.
