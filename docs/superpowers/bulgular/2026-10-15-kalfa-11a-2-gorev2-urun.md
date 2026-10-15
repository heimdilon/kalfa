# MCP 1.1a-2 — Görev 2 ürün doğrulaması

Tarih: 15 Ekim 2026. Ajan: Codex. PR #20; taban `1ede4c3`.
Ürün commit'i: `c4f12ab`. [Plan](../plans/2026-10-15-kalfa-11a-2-mcp.md).

## Davranış ve kapsam

Sunucu tanımı yalnız PATH'te çözülen çıplak program adı veya arka ucun
seçici kimliğini kabul eder; genel kabuk metni komut biçimi değildir.
Ad `[a-z0-9-]{1,32}`, ortam adı ASCII/en çok64, NUL ve sıfır timeout,
Windows harf duyarsız çakışması, ayrılmış önekler/sabitler ve tam erişim
ağ listesi doğrulanır. Sistem izin listesi, açık değerler ve sunucunun
zorlanan özel ortamı bu sırayla birleşir. Açık `GITHUB_TOKEN` korunur;
HOME/cache/temp kullanıcının değeriyle ezilemez. Göreli PATH ve argüman
sunucunun özel evine göre çözülür; `${CALISMA_ALANI}` arka uçta açılır.

W, proje ve kabuğun özel ev/geçici/önbellek kökleridir. Sunucunun kendi
özel kökü bu kümede değildir. Çözülmüş yürütülebilir, PATH dizinleri ve
açık argüman/ortam yolları kanonik hedefle denetlenir. Windows junction
ve `..`, işletim sisteminin tam yol anlamını kullanır; Unix symlink
semantiğiyle karıştırılmaz. Dosya kimliği ve bütün özgün referans yolları
onay sonrasında yeniden doğrulanır; aynı yolun başka dosyaya/bağlantıya
çevrilmesi eski özetle kabul edilmez.

`NODE_OPTIONS` yükleyici hedefleri açık ve mevcut normal dosya olmalıdır.
Uzantı tahmini, dizinin package-main/index çözümü, çıplak modül adı ve
belirsiz tırnaklı metin güvenli hatadır. `NODE_PATH`, `PYTHONPATH`,
`RUBYLIB`, `PERL5LIB` ve `CLASSPATH` ayrı arama girdilerine bölünür;
her girdinin W konumu denetlenir, boş/eksik girdi reddedilir. Normal
`C.UTF-8`, sürüm veya PATHEXT metnindeki nokta tek başına yol sayılmaz;
mevcut çıplak dosya yine içerik özetine girer. Kalfa'nın zorladığı özel
mutlak dizinler oluşmamış olabilir; yalnız backend'in aynen ürettiği bu
değer istisnadır ve W denetimi devam eder.

Bu bir genel yorumlayıcı/modül çözümleyicisi değildir. Öbür çalışma
zamanlarının metin içindeki yükleyici seçenekleri yorumlanmaz; bütün
geçişli bağımlılıkların kodunun sabitlendiği iddia edilmez. Sunucunun kendi
kararıyla kod yüklemesi ve npx/uvx sürüm çözümlemesi tasarımın mevcut
sınırlarıdır. npx/uvx için sabitlenmeyen paket kodu uyarısı korunur.
Açık yol gibi görünen fakat bulunmayan bir ortam değeri (opak bir sırın
içindeki ayraçlar dahil) güvenli ret alabilir; sessizce güvenilir sayılmaz.

Dosya içeriği yalnız native kayıt onayından sonra, blocking işte sabit
32KiB bloklarla SHA-256'ya girer. Native ret ve onay bekleyişinin iptali
okuma işini hiç başlatmaz. Ham içerik UI/model dönüş değeri değildir.
Onaydan sonra başlamış blocking okuma, çağıran future bırakılırsa
tamamlanabilir; sonucu kaydetme veya süreç başlatma yetkisi vermez.

Core'da T, Π, K, yalıtım profili, P, N, R, D ve yazma kuşağı ayrı tiplerdir.
P ve R, bileşenlerinden içeride hesaplanır; tutarsız R verilmez. N,
checked artan süreç sayacıdır. T son ortamı, gizli/düz değer özetlerini,
sıralı argv'yi, çözülmüş ikili/dosya içeriklerini, kip/ağ, timeout, özel
çalışma dizini ve alan kimliğini bağlar. D adı/açıklamayı ve iki şemayı
canonical JSON ile bağlar. Eksik dosya özetinden kısmi T üretilmez.

Tam K üretimi Görev4/5'tedir. `Hazirlik.mcp`, süreç sahibi, başlatma izni,
ayar/kasa, bütün çıkışların maskelemesi ve arayüz bağlantısı bu görevin
tamamlandığı iddiasına dahil değildir. Özellikle native önizlemeye kopyalanmış
sırların engellenmesi/maskelemesi Görev3'te bağlanmadan bu Rust girişi IPC
olarak açılmaz. Core, mcp'ye bağımlı olmadı; mcp core/sandbox'ı kullanır.

## Karşı örnekler ve bağımsız inceleme

- Native ret denetimi geçici kaldırılınca gerçek okuma sayacı1 oldu
  (beklenen0); açık ortamı ekleme kaldırılınca GITHUB_TOKEN kayboldu;
  dosya içerik özeti kaldırılınca aynı boyutlu değişiklik T'yi değiştirmedi;
  N yok sayılınca R değişmedi. Dört test düştü; kaynaklar geri yüklendi.
- Windows `${CALISMA_ALANI}/sunucu.js` verbatim kökle yanlış çözülüyordu.
  Gerçek test düştü; yalnız aynı hedefe çözüldüğü doğrulanan normal kökle geçti.
- Bağımsız GPT-6 Luna/max incelemesi, `--require=./hook` için hook.js'nin
  özet dışında kalmasını ve NODE_PATH'in ikinci W girdisinin kaçmasını
  buldu. İki gerçek regresyon testi önce düştü; dar ret/arama ayrıştırmasıyla geçti.
  Kurulu Node24.20.0'da uzantısız require'ın hook.js'yi yüklediği ayrıca ölçüldü.
- Yeni Windows junction/.. testi, bileşenleri Unix sırasıyla çözmenin
  farklı hedef verdiğini gösterdi. OS'nin tam yol çözümüyle geçti.
  Junction/symlink hedef değişimi dosya kimliği testleri iki platformdadır.
- Son bağımsız inceleme R2 kopyasının17/17 dosyasını SHA-256 ile
  doğruladı; önceki iki bulgu kapalı, başka kesin bulgu yok. İnceleyici
  testleri ayrıca koşturmadı.

## Son doğrulama

| Kontrol | Windows | Ubuntu24.04 WSL (`kalfatest`) |
|---|---:|---:|
| MCP hedefli takım | 61/61 | 62/62 |
| Workspace | 1300 geçti / 0 hata / 19 atlanan | 1254 / 0 / 19 |
| `e2e` özellikli Rust | 195/195 | 195/195 |
| `cargo check -p kalfa-mcp --lib --bins --locked` | Geçti | Geçti |
| Workspace ve e2e clippy (`-D warnings`), fmt | Temiz | Temiz |

Atlanan19 test mevcut canlı/dokümantasyon testleridir; canlı EVREN koşmadı.
Hedefli MCP sayıları workspace toplamına ikinci kez eklenmez. Client-only
kontrol artık `--lib --bins` içerir: test sunucusundaki `enable_tools`
builder'ı yalnız server özelliğinde bulunuyordu. Üretim özellikleriyle
ikili derlemesinin gerçek hatası, ortak capability alanları kullanılarak
düzeltildi; AGENTS ve CI aynı yeni komutu uygular.

Yeni paket veya indirme yok; yalnız mevcut workspace bağımlılıkları
yeniden kullanıldı. Windows/WSL önbellekleri çevrimdışıydı. Arayüz, gerçek
OS yalıtım ve WebDriver bu görevde yeniden koşmadı. Önceki `ui/dist`
Tauri girdisi yeniden kullanıldı; yeni arayüz derlemesi iddia edilmez.

Yerel kanıtlar `.yerel/mcp-tasarim-inceleme/11A2-G2-*`:
`ONAY-KIRMIZI`, `ORTAM-KIRMIZI`, `ICERIK-KIRMIZI`, `KUSAK-KIRMIZI`,
`NODE-OPTIONS-KIRMIZI`, `NODE-PATH-KIRMIZI`, `R2-WINDOWS-HEDEFLI`
(junction karşı örneği), `R2-WINDOWS-SON`, `R2-WSL-SON`.
İlk `WINDOWS-SON`/`WSL-SON` logları ara sürümlerdir ve nihai R2 kanıtı
yerine geçmez. R2 sabit kaynak makbuzu `mcp-11a2-gorev2-inceleme-r2/SHA256.json`:
`34169df83c0e464c36fc02b3b0517494676f507b01a9af137fba825028d8af75`.

İki Windows test TEMP kökü (`g2-son`, `g2-r2-son`) standart kullanıcıyla
silindi, yoklukları doğrulandı. Mevcut bir sandbox birim testi sahte
S-1-15-2-98765 izniyle boş `paylasilan` dizini bırakıyordu; yalnız bu işe
ait yaprağın DACL mirası açılıp temizlik tamamlandı. Yönetici veya sistem
kök izni kullanılmadı. Eski44 dizin ve ortak derleme önbellekleri korunur.
WSL kaynak kopyası aynı 1.1a-2'nin sonraki görevinde yeniden kullanılacak.

Görev2 kabulü tamam; sıradaki ürün işi Görev3 — ayar/kasa/native kayıt ve
maskeleme. PR20 taslaktır; MCP1.1a-2'nin bütünü ve Görev3–12 açıktır.
Merge veya yeni canlı deney yapılmadı.
