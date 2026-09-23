# Kalfa geliştirme durumu

Güncelleme: 2026-09-22. Bu dosya uygulamanın devam noktasıdır; tasarım
belgelerindeki tarihsel inceleme kayıtlarının yerine geçmez.

## Mevcut uygulama

- **1a — EVREN istemcisi:** `kalfa-evren`; istek/yanıt tipleri, SSE ve araç
  çağrısı akışı, tipli hatalar, şart uçları, kredi gözlemi ve fixture testleri.
- **1b — çekirdek:** `kalfa-core`; kimlikler ve yol doğrulama, dört izin kipi,
  şema doğrulama, mühürlü onay, çalışma alanı ortak kilitleri, JSONL kayıt ve
  kurtarma, sağlayıcı soyutlaması ve UI'sız oturum döngüsü. Araç aşaması
  (izin, mühürlü onay, kilitli paralel yürütme), iptal (akış/onay/hazırlık/
  yürütme sırasında), tur ve tekrar sınırları, hız sınırında tek yeniden
  deneme, şart akışı ve kayıttan devam tamamlandı.
- **1b — dosya araçları:** `kalfa-tools`; `dosya_oku`, `dosya_listele`,
  `icerik_ara`, `dosya_yaz`, `dosya_duzenle`. Çalışma alanına `cap-std` dizin
  tutamacıyla erişilir; üst dizin ve dosya bileşenlerinde sembolik
  bağlantılar (iç ya da dış, ayrıca Windows junction) izlenmez; yazma/
  düzenleme onaydan sonra içerik ya da hard link sayısı değişirse çalışmaz.
- **Uçtan uca:** gerçek dosya araçlarıyla, arayüz olmadan, okuma → onay →
  düzenleme → model yanıtı → kayıttan kurtarma akışı test edilir; aynı akış
  gerçek EVREN'e karşı `#[ignore]` testte de var (spec §4.2).

1b uygulama planındaki (`docs/superpowers/plans/2026-09-21-kalfa-1b-cekirdek.md`)
15 görevin tümü ayrı commit'ler halinde repoya işlendi (`git log --oneline` ile
görülebilir). Plandaki kutular tarihsel uygulama adımlarını tarif eder; bu
geçişte her göreve karşılık gelen kodun ve testlerin plan niyetini karşıladığı
yeniden doğrulandı (bkz. Doğrulama).

## 1b doğrulama kaydı

Windows üzerinde Rust 1.98.1 ile (bu geçişte yeniden koşturuldu):

| Kontrol | Sonuç |
| --- | --- |
| `cargo test --workspace --quiet` | 173 geçti, 0 başarısız, 4 canlı test atlandı |
| `kalfa-core` | 95 test geçti |
| `kalfa-evren` | 51 test geçti, 3 canlı test atlandı |
| `kalfa-tools` | 27 test geçti, 1 canlı test atlandı |
| `cargo clippy --workspace --all-targets -- -D warnings` | Temiz |
| `cargo fmt --all -- --check` | Temiz |
| `git diff --check` | Temiz (yalnızca LF/CRLF uyarıları) |
| `cargo tree -p kalfa-core -e normal \| grep -E "kalfa-tools\|tauri"` | Boş — çekirdek `kalfa-tools`'a ya da bir UI kütüphanesine bağlı değil |
| `git status research/` | Temiz — fixture'lar değişmedi |
| `git grep` ile kütüphane kodunda `unwrap()`/`expect()` taraması (`test_destek.rs` hariç, `--untracked` dahil) | Sonuç yok |

Windows symlink, bağlantılı üst dizin, hard link ve junction testleri
atlanmadan geçti (`baglantili_ust_dizinden_git_hook_yazilamaz`,
`junction_uzerinden_git_hook_yazilamaz`, `hard_link_onaydan_sonra_eklenirse_*`,
`baglanti_takma_adlari_serbest_kipte_bile_yetki_atlatamaz` dahil).

**Linux doğrulaması yapıldı.** Ubuntu 24.04.5 LTS (WSL2, `Ubuntu-24.04`
dağıtımı, native ext4 — Docker Desktop'ın iç VM'i değil), Rust 1.98.1
(`rustc 1.98.1 (48a229cea 2026-09-01)`, workspace'in `rust-toolchain.toml`
pinini karşılıyor), repo `/root/kalfa`'ya `git clone` ile kopyalanarak:

| Kontrol (Linux) | Sonuç |
| --- | --- |
| `cargo test --workspace` | 172 geçti, 0 başarısız, 4 canlı test atlandı |
| `kalfa-tools` | 26 geçti, 1 canlı test atlandı |

Windows'a göre tek fark: toplam 173 yerine 172 — `junction_uzerinden_git_hook_yazilamaz`
`#[cfg(windows)]` ile işaretli olduğu için Linux derlemesine hiç girmiyor
(beklenen davranış, eksik/başarısız değil). Diğer bütün testler, canlı
EVREN testleri hariç, atlanmadan koştu ve geçti. Unix'e özgü bağlantı
senaryoları ada göre doğrulandı (`cargo test -p kalfa-tools`, `--quiet`
olmadan, tek tek test adlarıyla):

- `sembolik_baglanti_ile_disari_cikilamaz` — dış symlink ile çalışma
  alanından çıkılamıyor
- `ic_baglanti_gizli_dosyayi_okutamaz_ve_aramaya_sokamaz` — iç symlink
  gizli dosyayı okutmuyor/aratmıyor
- `okuma_hazirligindan_sonra_baglanti_konursa_acilmaz`,
  `yazma_hazirligindan_sonra_konulan_baglanti_izlenmez` — onay/hazırlıktan
  sonra konan bağlantı izlenmiyor
- `baglantili_ust_dizinden_git_hook_yazilamaz` — bağlantılı üst dizin
  üzerinden `.git/hooks`'a yazılamıyor
- `hard_link_onaydan_sonra_eklenirse_dosya_yazilmaz`,
  `hard_link_onaydan_sonra_eklenirse_duzenleme_yapilmaz` — onaydan sonra
  eklenen hard link, yazma/düzenlemeyi `OnayEskidi` ile durduruyor
- `baglanti_takma_adlari_serbest_kipte_bile_yetki_atlatamaz` — uçtan uca,
  Serbest kipte bile bağlantı takma adlarıyla izin atlatılamıyor
- `disari_cikma_ve_gizli_dosya_uctan_uca` — dışarı çıkan yol ve gizli
  dosya reddi uçtan uca

Sonuç: plan sonu kontrol listesindeki "Linux ve Windows'ta bağlantı
testleri çalışır" maddesi artık her iki platformda da karşılanıyor.

Canlı EVREN testleri (`#[ignore]`) bu geçişte de çalıştırılmadı — depo
politikası gereği ajan bu testleri hiç koşturmaz (`.env` var olsa da).
Kullanıcı isterse: `set -a && . ./.env && set +a && cargo test --workspace -- --ignored`.

## Devam eden aşama — 1c (Tauri + Svelte kabuğu)

1c'nin tasarım belgesi yazıldı:
[`2026-09-22-kalfa-1c-design.md`](superpowers/specs/2026-09-22-kalfa-1c-design.md).
Uygulama planı **henüz yazılmadı.**
Bu bölüm, konuşmada şimdiye kadar verilen kararları kaydeder; böylece başka bir
oturum ya da araç kaldığı yerden devam edebilir.

### İlk altyapı adımı: olay kuyruğu

`kalfa-core` olay kanalı artık olay sayısı ve serileştirilmiş JSON baytları
üzerinden sınırlanıyor. `OturumAyarlari::olay_butcesi` varsayılanı 16.384 olay
ve 8 MiB; bunun bir olaylık ve 512 baytlık bölümü terminal aktarım hatasına
ayrılmıştır. Kuyruğa kabul edilen olaylar sırayla korunur. Yeni olay bütçeye
sığmazsa oturum iptal edilir ve kapanır; tüketici `transport_hatasi()` ile
nedeni görebilir. Bu durumda tüm olayların teslim edildiği veya `Tamamlandi`
olayının geleceği varsayılamaz; tüketici aktarım hatasını ayrıca ele almalıdır.

`OlayAlici` artık `UnboundedReceiver` takma adı değil, `recv()` / `try_recv()`
sunabilen bir sarmalayıcıdır. Alıcının kapanması süren turu iptal eder.
Bu sınır yalnızca olay kuyruğunun JSON yüküne aittir; bütün oturumun RAM
kullanımını veya gelecekteki webview kuyruğunu sınırladığı iddia edilmez.

Kapsam ve kabul ölçütleri:
[`2026-09-22-kalfa-1c-olay-aktarimi.md`](superpowers/plans/2026-09-22-kalfa-1c-olay-aktarimi.md).
Metin birleştirme, Tauri köprüsü ve gerçek arayüzde p95 gecikme ölçümü henüz
yapılmadı; §11.1 kabul kapısı bütünüyle tamamlanmış değildir.

Bu değişiklik için Windows üzerinde yeniden doğrulama (2026-09-22):

| Kontrol | Sonuç |
| --- | --- |
| `cargo test --workspace --quiet` | 182 geçti, 0 başarısız, 4 canlı test atlandı |
| Yeni olay aktarımı testleri | 2 birim + 7 entegrasyon testi geçti |
| `cargo clippy --workspace --all-targets -- -D warnings` | Temiz |
| `cargo fmt --all -- --check` | Temiz |
| `git diff --check` | Temiz; yalnızca LF/CRLF uyarıları |

Yeni testler olay ve bayt bütçelerini ayrı ayrı, UTF-8/JSON kaçışlarını,
terminal hata rezervini, gösterilemeyen onayda yazmanın engellenmesini,
sessiz akışta alıcı kapanışını, kritik olay sırasını ve geçersiz kapasiteyi
sınar. Bu yeni değişiklik Linux'ta yeniden çalıştırılmadı; yukarıdaki Linux
sonuçları tamamlanmış 1b sürümüne aittir. Canlı EVREN çağrısı yapılmadı.

### Kapsam (2026-09-22'de kesinleşti)

Tasarım belgesinin §11 ve §12.3 bölümlerinden:

1. Anahtar girişi ve kimlik deposu, şart metni/onayı, çalışma alanı seçimi.
2. `Komut`/`Zarf<Olay>` köprüsü, akış görünümü, iptal, araç onay kartları ve
   kayıttan oturum açma.
3. Dosya sistemi kimliği üzerinden çalışma alanı başına ortak kilit yöneticisi;
   oturum kaydı başına tek yazıcı sahipliği.
4. Olay birleştirme ve sert bayt/olay bütçeli IPC. Çekirdek kuyruğundaki sınırın
   Tauri/webview boyunca korunması ve gerçek tüketiciyle ölçülmesi gerekir.
5. Pencereye göre komut yetkileri, sıkı CSP ve güvenli Markdown gösterimi.

### Verilen kararlar (2026-09-22)

1. **Görsel temel:** Claude Design'daki "Kalfa Design System" projesi. Repodaki
   anlık kopyası `design/kalfa-ds/`. Kaynağı, kopyada eksik olanlar ve spec'le
   çelişen içerikler (şart ekranı, onay kapsamı vb.) `design/README.md`'de.
2. **Tokenlar aynen kullanılır:** `design/kalfa-ds/tokens/*.css` Svelte
   uygulamasına olduğu gibi girer. Tek istisna `fonts.css` (madde 4).
3. **Bileşenler Svelte 5'te yeniden yazılır.** Tasarım sistemindeki bileşenler
   React (`.jsx`). Her bileşenin `.d.ts` dosyası arayüz tanımı, `.prompt.md`
   dosyası kullanım notu olarak kullanılır. `_ds_bundle.js` ve
   `_adherence.oxlintrc.json` React'e özgüdür, uygulamaya girmez.
4. **Dış kaynak yok (§11.8, §11.9):** Tasarım sistemi fontları Google Fonts'tan,
   ikonları unpkg CDN'inden yüklüyor. Uygulamada Source Sans 3, Source Serif 4
   ve JetBrains Mono (hepsi OFL lisanslı) uygulamayla birlikte paketlenir;
   Lucide ikonları npm paketinden gelir.
5. **Üç sütunlu yerleşim:** menü (213 px), oturum listesi (295 px), konuşma.
   Tasarım sistemindeki sağ inspector sütunu (proje dosya ağacı ve dosya
   önizlemesi) 1c'ye girmez. Bu sütun, arayüzün çalışma alanını ajan döngüsü
   dışında okumasını ve gizli dosya kuralını (§7.4) arayüz tarafında da
   uygulamayı gerektirir; sonraki bir aşamada ele alınır.
6. **1c yalnızca kendi payını uygular.** Tasarım sisteminde olup sonraki
   aşamalara kalanlar:
   - `TurnSummary`'deki "Geri al" ve gönderim günlüğü ekranı: aşama 3.
   - Onay kartındaki "bu oturumda" kapsam seçimi: aşama 3. 1c'de kapsam her
     zaman "yalnızca bu çağrı".
   - Menüdeki "Eklentiler": aşama 4.
   - "Paylaş" düğmesi: v1 dışında (§12.2).
7. **Proje yapısı:** Tauri uygulaması `crates/kalfa/` altında (spec §4'teki
   `kalfa` crate'i); `tauri.conf.json` ve pencere yetkileri (`capabilities/`)
   orada. Arayüz `ui/` altında: Vite + Svelte 5 + TypeScript, SvelteKit yok.
   Paket yöneticisi npm; `package-lock.json` commit'lenir, kurulum `npm ci`
   ile. Ana pencere (`index.html`) ve şart penceresi (`sartlar.html`) ayrı
   giriş noktalarıdır; şart penceresinin paketinde sohbet ekranının kodu yer
   almaz. Tokenlar `ui/src/stiller/`'e kopyalanır (`fonts.css` hariç), fontlar
   `ui/src/fontlar/`'da paketlenir. Bileşen adları tasarım sistemindekilerle
   aynıdır. `kalfa` ince tutulur (pencere kurulumu, Tauri komutları, kanal
   bağdaştırıcısı, kimlik deposu); test gerektiren mantık Tauri'ye bağlı
   olmayan crate'lerde kalır. Sonuçları: Linux'ta `cargo test --workspace`
   WebKitGTK geliştirme paketlerini ister; `cargo test`'in önce
   `npm run build` gerektirip gerektirmediği plan aşamasında doğrulanacak.
8. **Olay köprüsü (§11.1): Tauri `Channel` + kredi.** Paketleyici ve kredi
   hesabı `kalfa-core`'da, oturum döngüsünden ayrı bir `aktarim` modülünde.
   İlk olay 16 ms'lik pencere açar; pencerede biriken olaylar tek pakette
   gider; art arda gelen aynı türden metin parçaları birleştirilir. Onay,
   hata, tamamlanma, araç ve şart olayları birleştirilmez, düşürülmez ve
   pencereyi beklemeden hemen gönderilir. Arayüzün "işledim" demediği veri en
   çok 4 paket ya da 256 KiB (başlangıç değerleri, ölçümle ayarlanır). Kredi
   bitince paketleyici çekirdek kuyruğundan okumaz; uzun süre okumayan
   arayüzde çekirdek bütçesi dolar ve iptal yolu çalışır. Krediden büyük tek
   olay, yolda paket yokken tek başına gider. Arayüz "işledim"i paketi
   durumuna uyguladığında gönderir, çizimi beklemez. `kalfa` yalnızca
   Channel'a yazar ve "işledim" komutunu karşılar. Kopukluğu paket numarası
   yakalar: `Zarf.sira` kayıt satırlarıyla ortak sayaçtan arttığı için olay
   akışında boşluk olağandır; `sira` yalnızca sıralama içindir (spec §4.1'deki
   boşluk kuralı buna göre güncellenecek).
9. **Oturum yaşam döngüsü, kilitler, tek yazıcı.** Açık oturum tablosu
   `kalfa`'da (her oturumun çekirdek tutamacı ve aktarım görevi); aynı oturum
   ikinci kez açılmaz, açık olana geçilir.
   - Çalışma alanı yalnızca yerel klasör seçiciden gelir; arayüz yol
     gönderemez, seçilmiş alanlara kimlikle başvurur.
   - Tek yazıcı: kaydın yanındaki kilit dosyasında işletim sistemi kilidi
     (`File::try_lock`), kayıt açık kaldıkça tutulur; kayıttan açarken önce
     kilit alınır, sonra okunur. `kalfa-core`'daki `OturumKaydi`'de. Kaydın
     kendisi kilitlenmez: Windows'ta dosya kilidi aynı süreçteki okumaları da
     engeller.
   - Tek Kalfa süreci: ikinci açılış mevcut pencereyi öne getirir (Tauri
     single-instance eklentisi); kilitler süreç içidir.
   - Kilit yöneticisi çalışma alanı kökünü dosya sistemi kimliğiyle tanır
     (Windows: birim seri numarası + dosya dizini; Linux: aygıt + inode);
     `kalfa-core`'da; son oturum kapanınca silinir. Bilinen sınır: iç içe
     çalışma alanları ayrı yönetici alır.
   - Arayüz kopması (sayfa yeniden yüklenmesi, pencere kapanması): o pencereye
     bağlı oturumlar kapanır, süren tur iptal edilir, kayıt kilidi bırakılır;
     arayüz son açık oturumu kayıttan açar. Yakalanamayan donmada kredi ve
     çekirdek bütçesi devreye girer.
   - Oturum listesi kayıt dizininden çıkarılır, dizin dosyası tutulmaz:
     kaydın başından çalışma alanı ve başlık, sonundan son durum okunur.
     Kayıttan açılan oturumun geçmişi kayıt öğelerinden üretilir; canlı
     olaylar `sira` ile devam eder.
   - `OturumAcildi` kaydına çalışma alanı yolu eklenecek (şu an yalnızca model
     ve kip var).
10. **Pencereler ve komut yetkileri (§11.9).** İki pencere; ikisini de yalnızca
    Rust açar.
    - Ana pencere: oturumlar, konuşma, onay kartları. Komutları: mesaj gönder,
      iptal, araç onayı, model ve kip değiştir, oturum aç/kapat, çalışma alanı
      seç, "işledim".
    - Şart penceresi: anahtar girişi ve EVREN şart metni. Komutları: anahtarı
      kaydet, şart metnini al, kabul et, reddet.
    - Genel bir "komut gönder" yok; her işlemin kendi Tauri komutu var.
      `SartlariKabulEt` yalnızca şart penceresindeki "Kabul ediyorum"
      düğmesinden doğar. Komutlar uygulama manifest'inde tanımlanır, yetkiler
      pencere etiketine göre dağıtılır.
    - Anahtar ana pencereye hiç girmez: şart penceresinde yazılır, bir kez
      Rust'a gider, arayüze geri dönmez.
    - Ana pencere yeni pencere açamaz (açabilseydi `sartlar` etiketli sahte
      pencereyle şart yetkilerini alabilirdi). Tauri'nin kendi izinlerinden
      yalnızca gerekenler verilir.
    - İki pencerede de uygulama sayfaları dışına gezinme kapalı.
    - Bağlantılar: tıklanınca Rust adresin tamamını yerel iletişim kutusunda
      gösterir, onayla sistem tarayıcısında açar; yalnızca `http`/`https`.
    - İzin kipini yükseltmek yerel iletişim kutusunda onaylanır; düşürmek onay
      istemez. Yalıtımsız kabuk onayı kabukla birlikte sonraki aşamada.
    - CSP: yalnızca uygulamanın kendi betik, stil, font ve görselleri; satır
      içi betik ve uzak kaynak yok.
11. **İlk açılış, şart akışı ve kimlik deposu (§11.8).**
    - Açılış: anahtar yoksa yalnızca şart penceresi açılır ve anahtar ister.
      Anahtar varsa `GET /v1/terms/status`: kabul edilmiş ve güncelse ana
      pencere; kabul gerekiyorsa metin şart penceresinde, ana pencere kabulden
      sonra; 401'de yeni anahtar istenir; EVREN'e ulaşılamıyorsa ana pencere
      bağlantı yok uyarısıyla açılır, şart gerekiyorsa ilk istekte 403 ile
      yakalanır.
    - Anahtar önce şart durumu sorgusuyla denenir; 401 dönerse ya da EVREN'e
      ulaşılamazsa saklanmaz. Geçerliyse işletim sistemi kimlik deposuna
      yazılır (`keyring`: Credential Manager / Secret Service). Secret Service
      yoksa pencere bunu söyler, anahtar yalnızca Kalfa açıkken bellekte
      tutulur. Anahtar değişirse açık oturumlar da yenisini kullanır.
    - Şart penceresi: başlıkta sürüm ve değişikliğin esaslı olup olmadığı;
      metin özetlenmeden güvenli Markdown ile; uzak görseller (EVREN
      metnindeki shields.io rozetleri) yüklenmez, alt metinleri görünür. Onay
      kutusu yok; yalnızca "Kabul ediyorum" ve "Kabul etmiyorum". Ret: anahtar
      saklı kalır, pencere nedenini söyleyen kilitli görünüme geçer ve metni
      yeniden gösterme düğmesi sunar; bekleyen turlar iptal edilir.
    - **Tek şart kapısı** (`kalfa-core`, Tauri'siz): oturumlar 403 alınca
      kapıya kaydolup bekler; kapı metni her sürüm için bir kez alır, gösterilen
      sürümü doğrular, kabul isteğini bir kez gönderir ve bekleyen oturumların
      hepsini uyandırır; ilk açılış da aynı kapıyı kullanır. Gerekçe: bugün her
      oturum kabulü kendisi gönderiyor; iki oturum beklerken tek tık iki
      `POST /v1/terms/accept` olur ve EVREN'in aynı sürümün tekrar kabulüne
      yanıtı bilinmiyor (kullanıcı adına kabul yasak olduğundan canlı
      denenmez).
12. **Ekranlar.**
    - 1c'de yazılacak bileşenler (tasarım sistemindeki adlarla): Badge, Button,
      Card, Icon, IconButton, IconTile; Checkbox, Input, Select, Textarea;
      Composer, Message, ThoughtPanel, ToolCallCard; CodeBlock, DiffView,
      MetaList, TokenMeter; NavItem, NavRail, PanelHeader, SessionRow,
      StatusBar. Sonraki aşamalara kalanlar: TurnSummary, FileTree, LogTable,
      Pagination, SourceTag. Menüde 1c'de yalnızca Oturumlar ve anahtar
      değiştirme var.
    - Onay isteği konuşmada küçük bir kart olarak görünür; "İncele" tasarım
      sistemindeki onay sayfasını açar (dosya listesi, dosya başına diff, "ne"
      ve "nerede", uyarılar, ret gerekçesi, üç karar düğmesi). Sayfa karar
      verilmeden kapatılabilir, istek kartta bekler; onay bekleyen oturum
      varken diğer oturumlara geçilebilir. Veri çıkışı, yalıtım,
      geri alınabilirlik ve kapsam alanları 3. aşamadaki tam kartla gelir.
      "Bu değişiklikler geri alınabilir" cümlesi geri alma gelene kadar
      yazılmaz.
    - Durum çubuğu: son gözlenen bakiye ve zamanı, oturumun token kullanımı
      (düşünce dökümüyle), yönlendirilen model, izin kipi. Bağlam doluluğu
      yok (EVREN kataloğu bağlam penceresini vermiyor). Tahmini maliyet
      yalnızca katalog fiyatı sıfırdan büyükse görünür (§11.5); şu an bütün
      fiyatlar sıfır.
    - Arayüz durumu saf bir TypeScript işleviyle güncellenir (paketi oturumun
      durumuna uygular, DOM'a dokunmaz, Vitest ile test edilir); Svelte
      yalnızca bu durumu çizer.

### 1c-3a — arayüz temeli (tamamlandı)

Plan: [`2026-09-23-kalfa-1c-3a-arayuz-temeli.md`](superpowers/plans/2026-09-23-kalfa-1c-3a-arayuz-temeli.md).
`ui/` altında Vite + Svelte 5 + TypeScript iskeleti; tasarım tokenları ve
paketlenmiş fontlar; Türkçe katalog; komut sözleşmesinin tipleri ve
çağrıları; oturum durumu işlevi; güvenli Markdown ve Shiki; tasarım
sisteminin 1c bileşenleri ve geliştirmede açılan bileşen vitrini. İki
pencerenin ekranları 1c-3b'de.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `npm ci` | 208 paket eklendi; 209 paket denetlendi, 0 güvenlik açığı | (inceleme) |
| `npm run check` | 0 hata, 0 uyarı | (inceleme) |
| `npm test` | 10 dosya, 104 test geçti | (inceleme) |
| `npm run build` + paket denetimi | Başarılı; `Paket denetimi temiz.` | (inceleme) |

### Tasarım belgesi

Bütün kararlar, yukarıdakilere ek olarak çizim ve dil, hata durumları, test ve
kabul ölçümü dahil,
[`2026-09-22-kalfa-1c-design.md`](superpowers/specs/2026-09-22-kalfa-1c-design.md)
belgesindedir. Yukarıdaki liste onun özetidir; ayrılırlarsa belge geçerlidir.
Sıradaki adım uygulama planıdır; belge §14 dört parçaya bölünmesini öneriyor
(1c-1 çekirdek eklemeleri, 1c-2 Tauri kabuğu, 1c-3 arayüz, 1c-4 kabul).

Kabuk yalıtımı, bağlam özetleme, geri alma, gönderim günlüğü, oturum bütçesi,
güvenlik denetçisi ve WASM eklentileri sonraki aşamalardadır. Henüz çalışır
bir masaüstü uygulaması veya tamamlanmış v1 sürümü yoktur.
