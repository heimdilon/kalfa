# Kalfa geliştirme durumu

Güncelleme: 2026-09-24. Bu dosya uygulamanın devam noktasıdır; tasarım
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
1c uygulama planı parçalara ayrıldı; çekirdek ve arayüz işleri ilgili plan
dosyalarında izleniyor.
Bu bölüm, konuşmada şimdiye kadar verilen kararları kaydeder; böylece başka bir
oturum ya da araç kaldığı yerden devam edebilir.

### 1c-1 — çekirdek eklemeleri (tamamlandı)

Plan: [`2026-09-22-kalfa-1c-1-cekirdek-eklemeleri.md`](superpowers/plans/2026-09-22-kalfa-1c-1-cekirdek-eklemeleri.md).
`kalfa-core`'a olay paketleyici ve kredi sınırı (`aktarim`), terminal
`AktarimDurdu` olayı, tek şart kapısı (`sart_kapisi`), çalışma alanı kilit
yöneticisi, kayıt başına tek yazıcı, kayıtta çalışma alanı / yönlendirilen model
/ token kullanımı, geçmiş görünümü ve oturum listesi (`gecmis`) eklendi.

Doğrulama (2026-09-23):

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `cargo test --workspace --quiet` | 254 geçti, 0 başarısız, 4 yok sayıldı | 253 geçti, 0 başarısız, 4 yok sayıldı |
| Clippy / fmt | Temiz | Temiz |
| `cargo tree` normal bağımlılık izolasyonu | `kalfa-tools` / `tauri` yok | `kalfa-tools` / `tauri` yok |
| `unwrap()` / `expect()` üretim taraması | Eşleşmeler yalnızca `#[cfg(test)]` modüllerinde | Aynı |

`research/` çalışma ağacı temizdi. Task 11 push'undan sonra CodeRabbit yeni
incelemeyi yürütüyordu; önceki inceleme limit nedeniyle satır satır
tamamlanamamıştı.

Linux sütunu PR incelemesinde dolduruldu: Ubuntu 24.04 (WSL2, native ext4),
Rust 1.98.1, `4f26f9d`. Windows'tan tek fark `junction_uzerinden_git_hook_yazilamaz`
testinin `#[cfg(windows)]` olmasıdır.

Uygulama sınırları: eski kayıtta `calisma_alani: None` ise çekirdek kurtarma
API'si yine kullanılabilir; bu tür kayıtları yalnızca geçmiş görünümünde açma
kapısı uygulama/UI katmanında, `oturum_ac` sırasında zorunlu tutulmalı. Kayıt
kilidi yan dosyası (`.kilit`) oturum açıkken silinmemelidir; uygulama kendi
temizliğinde bu dosyayı silmez. Kök kilit yöneticisi pasif yol girdilerini bir
sonraki yeni yol erişiminde toplar; sessiz dönemde tablo geçici olarak en yüksek
eşzamanlı yol sayısına kadar girdi tutabilir. Şart reddi yerel bekleyenleri
iptal eder, ancak başlamış EVREN kabul POST'unu uzaktan geri alamaz; yerel kapı
Ret durumunu korur. Bekleyen çağrı birden fazla karar neslini atlarsa fail-closed
olarak Ret alır; iki hızlı Kabul de atlanırsa aynı korumacı sonuç oluşabilir.
Büyük kayıt listesi 64 KiB bloklarla geriye taranır; tek bir çok uzun JSONL
satırını satır sınırında çözmek daha büyük bir okuma gerektirebilir.

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
   İlk olay 16 ms'lik pencere açar (1c-4'te ölçümle 8 ms'ye indi); pencerede
   biriken olaylar tek pakette
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
| `npm ci` | 208 paket eklendi; 209 paket denetlendi, 0 güvenlik açığı | Başarılı |
| `npm run check` | 0 hata, 0 uyarı | 0 hata, 0 uyarı |
| `npm test` | 10 dosya, 104 test geçti | 10 dosya, 104 test geçti |
| `npm run build` + paket denetimi | Başarılı; `Paket denetimi temiz.` | Başarılı; `Paket denetimi temiz.` |

Linux sütunu PR incelemesinde dolduruldu: Ubuntu 24.04 (WSL2), Node v24.20.0,
`56f5759`.

### 1c-2 — Tauri kabuğu (tamamlandı)

Plan: [`2026-09-23-kalfa-1c-2-tauri-kabugu.md`](superpowers/plans/2026-09-23-kalfa-1c-2-tauri-kabugu.md).
`kalfa-uygulama` (Tauri'siz uygulama mantığı: komut sözleşmesi, şart akışı,
oturum tablosu, olay köprüsü, kimlik deposu) ve `kalfa` (Tauri kabuğu: iki
pencere, 22 komut, pencere izinleri, yerel iletişim kutuları) eklendi. İki
pencerenin ekranları 1c-3b'de.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `cargo test --workspace --all-features --locked` | 301 geçti, 0 başarısız, 4 yok sayıldı | 295 geçti, 0 başarısız, 4 yok sayıldı (`kalfa` hariç) |
| Clippy / fmt | Başarılı | Başarılı (`kalfa` hariç) |
| `cargo tree -p kalfa-uygulama` içinde `tauri`/`wry`/`webkit` | Yok | Yok |
| `unwrap()` / `expect()` taraması | Yalnızca `test_destek.rs` | Aynı |

Linux sütunu PR incelemesinde dolduruldu: Ubuntu 24.04 (WSL2), Rust 1.98.1,
`767e8f5`. `kalfa` crate'i WebKitGTK geliştirme paketleri istediği için Linux'ta
derlenmedi; fark onun 5 izin testi ve Windows'a özel junction testidir.

Gerçek pencere denemesi (Windows, WebView2, sahte EVREN sunucusu; planın
İnceleme bölümü): şart penceresinde anahtar kaydı ve tek `POST` ile kabul, ana
pencerenin açılması ve şart penceresinin kapanması, pencereler arası ve
Tauri/eklenti komutlarının reddi, `window.open` ve dış adrese gezinme engeli,
kaydı olmayan oturumda `OturumYok` beklenen gibi.

Plandan sapma (Codex, PR yorumu): aynı oturumdaki izin kipi değişiklikleri
async kilitle sıraya alındı. Plandaki kodda eşzamanlı iki düşürme (ör. Serbest'ten
Proje ve İnceleme) ters sırayla çekirdeğe ulaşırsa kip onaysız yükselebiliyordu;
`eszamanli_kip_degisimleri_ayni_oturumda_sirayla_islenir` testi sınar.

### 1c-3b — ekranlar (tamamlandı)

Plan: [`2026-09-23-kalfa-1c-3b-ekranlar.md`](superpowers/plans/2026-09-23-kalfa-1c-3b-ekranlar.md).
İki pencerenin ekranları yazılıp komutlara bağlandı: ana pencerede menü,
oturum listesi, konuşma, onay kartı ve onay sayfası, durum çubuğu; şart
penceresinde anahtar formu, EVREN şart metni ve kilitli görünüm. Testler ve
geliştirmedeki ekran önizlemesi (`ui/onizleme.html`) sahte bir arka uçla
(`ui/src/sahte/arka.ts`) çalışır; uygulama paketine girmez.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `npm ci` | 208 paket eklendi; 209 paket denetlendi, 0 güvenlik açığı | Başarılı |
| `npm run check` | 0 hata, 0 uyarı | 0 hata, 0 uyarı |
| `npm test` | 14 dosya, 167 test geçti | 14 dosya, 167 test geçti |
| `npm run build` + paket denetimi | Başarılı; `Paket denetimi temiz.` | Başarılı; `Paket denetimi temiz.` |

Linux sütunu PR incelemesinde, 1c-2 ile birleşmiş ağaçta dolduruldu: Ubuntu
24.04 (WSL2), Node v24.20.0.

Birleşik uygulama denemesi (Windows, WebView2, sahte EVREN sunucusu): 1c-2'nin
kabuğu ile bu ekranlar ilk kez birlikte çalıştırıldı. Anahtar formdan girildi,
alan boşaldı, şart metni geldi. "Kabul ediyorum" tek `POST` gönderdi; ana pencere
boş liste ve karşılama ekranıyla açıldı; "Anahtarı değiştir" şart penceresini
anahtar formuyla açtı. Denemede görülen bir hata incelemede düzeltildi: açık
temadaki şart penceresinde not ve uyarı zemini koyu çıkıyordu, çünkü tasarım
sistemindeki `--amber-tint` ve `--blue-tint` yalnızca koyu temada tanımlı. Zemin
artık durum renginden `color-mix` ile türetilir.

### 1c-4 — kabul (tamamlandı)

Plan: [`2026-09-24-kalfa-1c-4-kabul.md`](superpowers/plans/2026-09-24-kalfa-1c-4-kabul.md).

Neler eklendi:
- Uygulama gerçek pencerede, sahte EVREN'li bir test derlemesiyle ve
  `tauri-driver` üzerinden sınanıyor. Kapsam: ilk açılış, konuşma, onay, iptal,
  kayıttan açma ve güvenlik sınırları.
- Üst belge §11.1'in kabul kapısı iki platformda geçti.
- Test derlemesi `e2e` özelliğidir ve sürüm derlemesine girmez. Özellik şunları
  getirir:
  - Sahte sağlayıcı, sahte iletişim kutuları ve aktarım sayaçları.
  - Test komutları ve arayüzün test kancası.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `cargo test --workspace --locked` | 303 geçti, 0 başarısız, 4 yok sayıldı | 302 geçti, 0 başarısız, 4 yok sayıldı |
| `cargo test -p kalfa-uygulama -p kalfa --features e2e` | Başarılı | Başarılı |
| Clippy (normal ve `e2e` derlemesi) / fmt | Başarılı | Başarılı |
| `npm run check` / `npm test` / `npm run build` | 0 hata, 0 uyarı; 16 dosya, 184 test; paket denetimi temiz | Aynı |
| Uçtan uca (`cd e2e && npm test`) | 21 test geçti | 21 test geçti (Xvfb) |

Kabul ölçümü (plan doğrulaması, 2026-09-24):
- **Windows:** Windows 11, WebView2 153.
- **Linux:** Ubuntu 24.04, WSL2, Xvfb (yazılım çizimi), WebKitGTK 2.52.
- **Gecikme:** parçanın üretilmesinden ekrana çizilmesine kadar geçen süre.

| Ölçüm | Windows | Linux |
| --- | --- | --- |
| Gecikme p50 / p95 / p99, 20 ms'de bir parça | 21,9 / 25,2 / 25,9 ms | 26,8 / 40,1 / 44,3 ms |
| Gecikme, 5–10 parçalık yığınlar | 21,9 / 26,0 / 31,8 ms | 21,2 / 31,3 / 32,9 ms |
| Gecikme, düşünce ve metin paralel | 21,9 / 26,0 / 27,2 ms | 20,9 / 29,2 / 31,9 ms |
| Duraklayan arayüz: yoldaki en çok veri (kredi 4 paket, 256 KiB) | 4 paket, 246 889 bayt | Aynı |
| Duraklayan arayüz: kuyruğun en yükseği (bütçe 16 384 olay, 8 MiB) | 1 973 olay, 8 387 088 bayt | Aynı |
| Duraklarken Kalfa sürecinin belleği | 29 → 38 MB | 205 → 232 MB |
| Uzun oturum açılışı, 20 / 500 tur | 36,6 / 66,3 ms | 54 / 111 ms |
| Ekrandaki DOM düğümü, 20 / 500 tur | 635 / 639 | 635 / 639 |

- **Paketleme penceresi 16 ms'den 8 ms'ye indi.** 16 ms'de p95 Windows'ta 41,
  Linux'ta 47 ms'ydi; hedef 50 ms'ye çok yakındı.
- **Kredi ve paket sınırı değişmedi.**
- **Duraklayan arayüzde tur iptal edildi, sınırlar aşılmadı.** Arayüz yeniden
  işlemeye başlayınca `AktarimDurdu` uyarısı geldi. Kuyruk bütçeye kadar doldu,
  ama ne o ne de yoldaki veri sınırı aştı.
- **Linux'ta bellek daha yüksek görünüyor:** WebKitGTK'nin arayüz süreci
  Kalfa'nın kendi sürecidir. Windows'ta WebView2 ayrı süreçlerdedir.
- **Uzun oturumda tam sanallaştırma gerekmedi (§9.3).** Açılış 20 turluk
  kayıttakiyle aynı mertebede kaldı.

Plan yazılırken bulunanlar:
- **Pencere kapanınca oturum kaydı süreç çıkışıyla yarışıyordu.** Süren tur
  iptal edilip kayda yazılırken süreç çıkabiliyordu. Artık süreç kapanan
  oturumları bekliyor (`kapananlari_bekle`).
- **WebDriver'ın pencere kapatması uygulamanın kapanış yolunu sınamıyor.** Son
  pencerede süreci doğrudan sonlandırıyor. Test bu yüzden pencereyi test
  komutuyla kapatıyor.
- **Chromium, WebDriver'ın betik bağlamında `eval`'i engellemiyor.** Yalnızca
  ihlali bildiriyor; dizeden kod bu yüzden dize zamanlayıcısıyla sınanıyor.
- **WebKit'te pencereyi kapatan tıklama "no such window" dönebiliyor.** Test
  yardımcısı bunu beklenen sonuç sayıyor.

Açık madde: `same-file`'ın ReFS'teki (Dev Drive) kimliği doğrulanamadı;
makinede ReFS birimi yok.

### Tasarım belgesi

Bütün kararlar, yukarıdakilere ek olarak çizim ve dil, hata durumları, test ve
kabul ölçümü dahil,
[`2026-09-22-kalfa-1c-design.md`](superpowers/specs/2026-09-22-kalfa-1c-design.md)
belgesindedir. Yukarıdaki liste onun özetidir; ayrılırlarsa belge geçerlidir.
1c'nin dört parçası (1c-1 çekirdek eklemeleri, 1c-2 Tauri kabuğu, 1c-3 arayüz,
1c-4 kabul) tamamlandı. Sıradaki aşama üst belge §12.3'e göre ikincisidir:
kabuk aracı ve yalıtım.

Kabuk yalıtımı, bağlam özetleme, geri alma, gönderim günlüğü, oturum bütçesi,
güvenlik denetçisi ve WASM eklentileri sonraki aşamalardadır. Masaüstü
uygulaması Windows ve Linux'ta çalışıyor; tamamlanmış bir v1 sürümü yoktur.
