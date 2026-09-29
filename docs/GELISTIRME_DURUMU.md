# Kalfa geliştirme durumu

Güncelleme: 2026-09-28. Bu dosya uygulamanın devam noktasıdır; tasarım
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
- **1c — masaüstü uygulaması:** `kalfa-uygulama`, Tauri kabuğu `kalfa`,
  Svelte arayüzü `ui/`; ayrıntısı aşağıda.
- **2a — kabuk aracı:** `kabuk` (`kalfa-tools`) ve komut çalıştırma, süreç
  sınırları (`kalfa-sandbox`). Ayrıntısı aşağıda.
- **2b — Linux yalıtımı:** kabuk komutları Linux'ta bubblewrap ve seccomp ile
  yalıtımda çalışır (`kalfa-sandbox`); Ayarlar'da yalıtım durumu ve araç
  zinciri dizinleri. Ayrıntısı aşağıda.
- **2c — Windows prototipi:** ürün kodu yok; ölçüm aracı `prototip/windows-yalitim/`.
  Kısıtlı belirteç yolu Dal B'de kaldı; AppContainer ek ölçümüyle Dal A'ya
  ulaşılıyor. Ayrıntısı aşağıda.
- **2d-1 — Windows arka ucu:** `kalfa-sandbox`'ta AppContainer yalıtımı,
  yönetici yardımcısı `kalfa-yalitim.exe`, çalışma alanı hazırlığı ve ACE
  kaydı. Kurulumu yapılmış makinede uygulama kabuk komutlarını yalıtımda
  çalıştırır; kurulumu uygulamadan başlatmak ve Ayarlar'ın Windows bölümü
  2d-2'dedir. Ayrıntısı aşağıda.
- **2d-2 — son kullanıcı akışı (tamamlandı):** yalıtım yokken kapalı kabuk,
  sade ileti ve BT raporu, yalıtımsız anahtarı, "Onar", Kalfa Projeleri
  klasörü, Ayarlar'da hazırlanmış alanlar, Windows kabuk e2e'si gerçek
  yalıtımda ve makine geneli NSIS kurucusu. Kurucu kurulu makinede UAC ile
  kabul edildi. Ayrıntısı aşağıda.
- **2d-3 — Kalfa'nın getirdiği araç zinciri (tamamlandı; kurucu UAC ile
  kabul edildi):** kurucu Python ve Node'u sabit sürüm ve SHA-256'yla getirir,
  kabın `PATH`'inde önce gelirler; yöneticiye ait araç dizini UAC ile açılır,
  Onar yükseltmede kaybolan izinleri geri getirir; Windows'ta ağsız cargo
  geliştiricinin registry'siyle derler; sınır matrisi. Ayrıntısı aşağıda.
- **Güvenlik sertleştirmesi (PR #16):** [güvenlik analizinin](superpowers/bulgular/2026-09-27-guvenlik-analizi.md)
  1–7. bulguları: `.git`'in tamamı korumalı ve yeni `.git` karantinası,
  kendiliğinden çalışan yapılandırma sınıfı, kabuk onayında onaysız
  değişiklikler, genişleyen gizli kalıplar, araç çıktısının rastgele sınırlı
  sarmalı, yönetici yardımcısının kayıt dizini denetimi ve görünmez
  karakterler. Ayrıntısı aşağıda.
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
kabuk aracı ve yalıtım. Tasarımı
[`2026-09-24-kalfa-2-kabuk-yalitim-design.md`](superpowers/specs/2026-09-24-kalfa-2-kabuk-yalitim-design.md)
belgesindedir. Aşama dört plana bölünür: 2a kabuk aracı ve süreç sınırları, 2b
Linux arka ucu, 2c Windows prototipi, 2d Windows arka ucu.

## Devam eden aşama — 2 (kabuk aracı ve yalıtım)

Tasarım:
[`2026-09-24-kalfa-2-kabuk-yalitim-design.md`](superpowers/specs/2026-09-24-kalfa-2-kabuk-yalitim-design.md).
Dört plana bölünür: 2a kabuk aracı ve süreç sınırları, 2b Linux arka ucu, 2c
Windows prototipi, 2d Windows arka ucu.

### 2a — kabuk aracı ve süreç sınırları (tamamlandı)

Plan: [`2026-09-24-kalfa-2a-kabuk.md`](superpowers/plans/2026-09-24-kalfa-2a-kabuk.md).

Neler eklendi:
- **`kabuk` aracı.** İki biçimi var: `{program, argumanlar}` ve `{betik}`.
  Betik Linux'ta sh, Windows'ta PowerShell ile çalışır. Seçenekler: çalışma
  dizini, ağ isteği, zaman aşımı.
- **`kalfa-sandbox`.** Komutu başlatır ve süreç sınırlarını uygular:
  - Ortam değişkenleri izin listesiyle geçer.
  - Varsayılan süre 120 sn, en çok 1800 sn.
  - En çok 256 süreç çalışabilir (Windows'ta).
  - İptal, zaman aşımı ya da komutun bitişi bütün süreç ağacını durdurur:
    Linux'ta süreç grubu, Windows'ta Job Object.
- **İzin kararı.** Yalıtım olmadığı için her komut, her kipte yerel pencerede
  onaylanır. Webview'daki "Onayla" komutu çalıştırmaz; Rust yerel pencereyi
  açar ve gösterdiği komutu çekirdekten okur.
- **İlk çağrı uyarısı.** Bir çalışma alanındaki ilk komut, projenin betiklerinin
  kod olarak çalışacağını söyler. Bu onay uygulamanın veri dizinine kaydedilir.
- **Canlı çıktı.** Komut çalışırken çıktı araç kartındaki terminal görünümünde
  akar; ANSI renkleri çizilir. Model, komut bitince kırpılmış çıktıyı alır:
  baştan 10 000, sondan 20 000 bayt ve çıkış kodu.
- **Durum çubuğu.** Yalıtım göstergesi ("Yalıtımsız") eklendi.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `cargo test --workspace --locked` | 348 geçti, 0 başarısız | 346 geçti, 0 başarısız |
| `cargo test -p kalfa-uygulama -p kalfa --features e2e` | 63 geçti | 63 geçti |
| Clippy (normal ve `e2e` derlemesi) / fmt | Başarılı | Başarılı |
| `npm run check` / `npm test` / `npm run build` | 0 hata, 0 uyarı; 18 dosya, 205 test; paket denetimi temiz | Aynı |
| Uçtan uca (`cd e2e && npm test`) | 24 test geçti (iki koşuda) | 24 test geçti (Xvfb) |
| Gecikme p95 (düzenli / yığın / paralel) | 24,2–24,4 / 24,8–26,1 / 24,5–24,6 ms | 37,8 / 31,5 / 37,9 ms |

Plan yazılırken bulunanlar:
- **Çıktı seli Linux'ta arayüzü boğuyordu.** Terminal görünümü megabaytlarca
  metin tutarken WebKitGTK (Xvfb, yazılım çizimi) saniyede 50–60 KB
  işleyebildi; tur 60 saniyede bitmedi. Görünüm artık en çok 200 000 karakter
  tutuyor, tek satır 4 000 karakterde kesiliyor ve araçtan arayüze en çok
  1 MiB gidiyor. Aynı test 3,6 saniyede bitiyor.
- **Canlı çıktının kayıplı olması iki yerde gerekli.** Araçtan oturuma giden
  kanal doluyken parça atılır. Oturum da çekirdeğin olay kuyruğu yarıdan fazla
  doluyken parçayı iletmez. Yoksa yavaş bir ekran kuyruğu doldurup turu iptal
  ettirirdi. Atılan her bayt sayılır ve ekranda "… N bayt gösterilmedi …" diye
  görünür.
- **Aynı çağrının üç kez başarısız olma sınırı kabuğu durdururdu.** Model testi
  düzeltip aynı komutu yeniden koştukça sınır dolardı. Sıfırdan farklı çıkış
  kodu ve zaman aşımı bu sınıra sayılmıyor.
- **Yerel onay penceresinin gösterdiği komut webview'dan gelmemeli.** Gelseydi
  sızmış bir betik zararsız bir metinle zararlı komutu onaylatabilirdi. Komut
  çekirdeğin bekleyen onayından okunuyor. Çekirdek de webview'ın gönderebildiği
  onay komutuyla yerel onaylı öğeyi onaylamıyor.

### 2b — Linux yalıtımı (tamamlandı)

Plan: [`2026-09-25-kalfa-2b-yalitim.md`](superpowers/plans/2026-09-25-kalfa-2b-yalitim.md).

Neler eklendi:
- **Linux arka ucu.** Komut bubblewrap'ın içinde çalışır:
  - Sistem yolları ve araç zinciri (`~/.cargo/bin`, `~/.rustup`, Ayarlar'dan
    eklenenler) salt okunurdur. Ev dizini, diğer çalışma alanları ve sistemin
    geri kalanı görünmez.
  - Çalışma alanı yazılabilir, dışına yazılamaz. Her çalışma alanının kendi
    ev, geçici ve önbellek dizini vardır (veri dizinindeki `yalitim/`).
  - Gizli dosyalar ve hard link'leri okunamaz. `.git/hooks`, `.git/config` ve
    `core.hooksPath` hedefi salt okunurdur; worktree'nin dışarıdaki git dizini
    yazılabilir bağlanır.
  - Ağ varsayılan olarak kapalıdır. seccomp `io_uring`, `keyctl`, `bpf` ve
    `ptrace` gibi çağrıları engeller; soket aileleri izin listesiyle
    denetlenir (Unix ve vsock soketleri açılamaz).
  - Süreç sayısı sınırı (256) yalıtımın içinde konur. İptal ve Kalfa'nın ölümü
    bütün ağacı durdurur.
- **Algılama.** bwrap yoksa ya da kullanıcı ad alanları kısıtlıysa Kalfa bunu
  durum çubuğunda ve Ayarlar'da talimatıyla söyler; komutlar yalıtımsız ve
  yerel onaylı çalışır.
- **Ayarlar.** Menüye Ayarlar eklendi: yalıtım durumu, talimat, "Yeniden dene"
  ve araç zinciri dizinleri (Linux; yerel klasör seçiciyle eklenir).
- **İzin kararı.** Yalıtımlı komut Sıkı ve Proje kipinde olağan onayla, Serbest
  kipte ağsızsa onaysız çalışır. Çalışma alanındaki ilk komut ve ağlı komut her
  kipte onay ister.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `cargo test --workspace --locked` | 360 geçti, 0 başarısız | 369 geçti, 0 başarısız |
| `cargo test -p kalfa-uygulama -p kalfa --features e2e` | 66 geçti | 66 geçti |
| Clippy (normal, `e2e`; Linux'ta ayrıca `yalitim-testleri`) / fmt | Başarılı | Başarılı |
| Yalıtım testleri (root olmayan kullanıcıyla) | — | `yalitim` 17, `uyumluluk` 4 geçti |
| `npm run check` / `npm test` / `npm run build` | 0 hata, 0 uyarı; 20 dosya, 218 test; paket denetimi temiz | Aynı |
| Uçtan uca (`cd e2e && npm test`) | 24 test geçti (iki koşuda) | 25 test geçti (iki koşuda; Xvfb, birleştirme kapalı) |
| Gecikme p95 (düzenli / yığın / paralel) | 24,4–24,6 / 25,6–26,1 / 24,2–24,6 ms | 35,9–37,5 / 33,0–35,5 / 36,8–37,6 ms |

Plan yazılırken bulunanlar:
- **Süreç sayısı sınırı yalıtımın içinde konmalı.** Sınır bwrap'a dışarıda
  verilince ad alanının üst sınırı oluyor ve kullanıcının bütün süreçleri
  sayılıyor; komut hiç başlamıyor. İçeride (`prlimit`) konunca yalnızca
  yalıtımdaki süreçler sayılıyor.
- **Salt okunur cargo registry'si ağsız derlemeye yetiyor.** Yeni crate ise
  salt okunur registry'ye inemez; ağlı çalıştırmada kullanıcının registry'si
  bağlanmaz, cargo çalışma alanına özel registry'ye indirir.
- **Rustup vekili her çalışmada `~/.rustup/update-hashes`'i istiyor.**
  `~/.rustup`'ın tamamı salt okunur açıldı; dizinde kimlik bilgisi yok.
- **x32 çağrıları seccomp listesini atlatabilirdi.** `seccompiler` bunları
  kapsamıyor; elle yazılmış ikinci bir filtre `ENOSYS` döndürüyor.
- **Konuşma en alttan kopuyordu (1c'den kalan hata).** Kendi kaydırma olayı,
  onay sayfası kapanınca odağın geri dönmesi ya da konuşmadaki bir düğmeye
  tıklamak alta yapışmayı bırakıyordu; yeni onay kartı görünüm alanının altında
  kalıyordu. Artık yalnızca kullanıcının kaydırması bırakıyor. Linux'ta kabuk
  testleri yalıtımla koşunca görünür oldu.
- **Linux gecikme ölçümü Xvfb'de birleştiriciyi ölçüyordu.** Alta yapışma
  düzelince akan metin görünür oldu ve p95 56–64 ms'ye çıktı. Karenin çizimi
  ~5 ms sürüyor; asıl bekleme, GPU'suz Xvfb'de yazılımla (llvmpipe) çalışan GL
  birleştiricisinde: görünür her değişiklikte bir sonraki kare ~25 ms gecikiyor.
  Linux uçtan uca testleri `WEBKIT_DISABLE_COMPOSITING_MODE=1` ile koşar; ürün
  kodu değişmez. 1c'deki Linux ölçümü, akan metin görünüm alanının dışındayken
  alınmıştı.

PR incelemesinde düzeltilenler (Codex'in inceleme botu buldu):
- **Onaylanan kip çalıştırmada korunmuyordu.** Araç yalıtımı o anki durumdan
  çıkarıyordu. Mühür denetiminden sonra yeniden algılama durumu "yok"a
  çevirirse yalıtımlı onaylanan komut yalıtımsız başlayabilirdi. Kip artık
  onaylanan tariften alınıyor; yalıtımsız arka uç da yalıtımlı onaylanan
  isteği reddediyor.
- **`AF_VSOCK` engellenmiyordu.** seccomp soket kuralı engel listesiydi ve ağ ad
  alanı vsock'u kesmez; WSL2'de bu, Windows'taki servislere giden yoldur. Soket
  aileleri artık izin listesiyle denetleniyor.

### 2c — Windows prototipi (tamamlandı)

Bulgular: [`2026-09-25-kalfa-2c-windows-prototip.md`](superpowers/bulgular/2026-09-25-kalfa-2c-windows-prototip.md).
Prototip: [`prototip/windows-yalitim/`](../prototip/windows-yalitim/README.md). Ürün
kodu değildir; Kalfa'nın workspace'ine girmez.

Tasarımdaki P1–P10 geliştiricinin makinesinde (Windows 11 Home) ölçüldü:
- Yönetici adımlarını kullanıcı çalıştırdı: `kur.ps1`, `guvenlik-duvari.ps1`, `geri-al.ps1`.
- Deneme ağacını da kullanıcı kurdu: `agac.ps1`.
- Ölçümleri Claude koştu: `kalfa-prototip olc`.

- **Sonuç Dal B.**
  - Windows, S-1-15 SID'lerini (ALL APPLICATION PACKAGES dahil) kısıtlayıcı SID olarak kabul
    etmiyor.
  - Sistem dizinlerini okutan tek kullanılabilir SID Users. Bu yüzden Users'ın okuyabildiği
    klasörler de okunuyor.
  - Çalışma alanları arası yazma ayrımı, profil koruması, gizli dosya ve korumalı yollar tutuyor.
  - Araç zincirlerinin tamamı çalışıyor: cmd, PowerShell, git, node, npm, python, cargo, rustc.
- **Tasarıma işlenen düzeltmeler:**
  - Kısıtlayıcı küme: çalışma alanı + ortak okuma + Everyone + Users + RESTRICTED.
  - Varsayılan DACL: kullanıcı, SYSTEM ve çalışma alanı SID'i.
  - `SetErrorMode`: yükleyici hata kutuları UI kısıtlarına rağmen masaüstünde açılıyordu.
  - Güvenlik duvarı kuralının kullanıcı koşuluna ortak okuma SID'i eklendi. Yoksa kural kısıtlı
    belirteçlere uygulanmıyordu.
  - Users grubundan çıkarma adımı kalktı: etkisiz.
  - İzinler API'yle veriliyor: `icacls` hesaba karşılık gelmeyen SID'leri reddediyor.
- **Kısıtlı belirteç yolunda açık kalanlar:**
  - A8: Schannel kısıtlı belirteçte çalışmıyor; ağlı cargo crates.io'ya ulaşamıyor.
  - A9: Geri döngü süzülmüyor.
  - A10: Varsayılan izinli borular salt okunur açılıyor.

**AppContainer ek ölçümü:**

- Bulgular: [`2026-09-25-kalfa-2c-appcontainer.md`](superpowers/bulgular/2026-09-25-kalfa-2c-appcontainer.md).
- Yönetici adımlarını (`kap-kur.ps1`) kullanıcı çalıştırdı; ölçümleri
  (`kalfa-prototip kap olc`) Claude koştu.

- **Sonuç Dal A.**
  - Okunamayanlar: kullanıcı klasörleri, `C:\Users` ve profil.
  - Araç zincirlerinin tamamı çalışıyor.
  - Schannel çalışıyor, geri döngü kapalı, varsayılan izinli borular
    açılamıyor (A8–A10).
  - Ayrı kullanıcı ve güvenlik duvarı kuralı gerekmiyor.
  - Çağrı başına 22–29 ms.
- **Bedeli:**
  - `C:\` köküne kurulumda yayılmayan bir listeleme izni.
  - Korumalı yollar yalnızca izinle; ret izinleri AppContainer'da işlemiyor.
  - MSVC ortamını Kalfa verir.
  - Yönetici sahipli araç zinciri dizinlerine kurulumda izin.
- **Açık:** A11 (profil altındaki çalışma alanının üst dizinleri), A12 (C
  derleyen `build.rs`), A13 (LAN, ssh-agent ve docker boruları).

- **Tasarım:** §9 AppContainer'a göre yeniden yazıldı; ayrı kullanıcı yolu
  §9.7'de yedek. Üst belge r3.5.
- **Güvenlik:** Kabın içinden başka bir paket SID'iyle süreç başlatılamıyor
  (A14).
- **Makine:** Prototipin kurulumu geri alındı.

### Güvenlik düzeltmesi: yalıtımda `.git` salt okunur (2026-09-26)

2d-1 planlanırken 2b'nin Linux yalıtımında üç açık bulundu. Üçü de
Windows arka ucunu da ilgilendirdiği için 2d'den önce ayrı bir PR'la kapatıldı
(aşama 2 tasarımı §7.2–§7.4, üst belge r3.6):

- **`.git` yalnızca kısmen korunuyordu.** Kancalar ve yapılandırma salt
  okunurdu, ama yalıtımdaki komut `.git`'in kendisini yeniden adlandırıp yerine
  yenisini kurabiliyordu (WSL'de doğrulandı). Artık `.git`'in tamamı (dizin ya
  da worktree'nin `.git` dosyası) salt okunur bağlanır. Kullanıcı kararıyla
  yalıtımda `git status`, `git diff` ve `git log` çalışır; `git add` ve
  `git commit` çalışmaz.
- **Dış git dizini doğrulanmıyordu.** `.git` dosyasının `gitdir:`'i ve
  `commondir` çalışma alanındadır; gösterdikleri dizin doğrulanmadan
  yazılabilir bağlanıyordu. Artık geri başvurusu tutmayan dizin açılmaz,
  tutan da salt okunur açılır.
- **Kalfa'nın özel dizinlere yazması bağlantı izliyordu.** `ev/.gitconfig` ve
  `ev/.npmrc` her çalıştırmadan önce yazılır; bu dizinlere yalıtımdaki komut da
  yazabilir. Artık bağlantılar (Windows'ta junction'lar da) izlenmez.
  Windows'ta junction'lı test eski kodda düştü.

Bilinen sınır: `.git` sembolik bağlantıysa, çalışma alanı git deposu değilse ya
da alt dizinlerde, yalıtımdaki komut yeni bir `.git` oluşturabilir.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `cargo test --workspace --locked` | 365 geçti, 0 başarısız | 375 geçti, 0 başarısız |
| `cargo test -p kalfa-uygulama -p kalfa --features e2e` | 66 geçti | 66 geçti |
| Clippy / fmt | Başarılı | Başarılı (üç derleme) |
| Yalıtım testleri (`kalfatest`) | — | `yalitim` 17, `uyumluluk` 4 geçti |
| Uçtan uca | 24 test geçti (iki koşuda); p95 24–27 ms | 25 test geçti (iki koşuda); p95 35–38 ms |

### Hata düzeltmesi: aynı turdaki kabuk çağrıları ve alan hazırlığı (2026-09-28)

Windows'ta hazırlanmamış çalışma alanında model aynı turda birden çok `kabuk`
çağrısı yapınca yalnızca ilki çalışıyor, ötekiler "Dosya onaydan sonra
değişti" ile düşüyordu. Sebep şuydu: ilk çağrı alanı hazırlayınca ötekilerin
yürütmeden önceki taze tarifinde alan hazırlığı ve ilk çağrı artık yoktu, bu
yüzden mühür tutmuyordu. Tükenen bu tek seferlik koşullar artık onayla
kapsanır ve araç onaylanan tarifle çalışır. Onaylanmayan hazırlık sonradan
belirirse komut yine başlamaz; ileti nedeni söyler (aşama 2 tasarımı §3.3).

| Kontrol | Windows | WSL Ubuntu-24.04 |
| --- | --- | --- |
| `cargo test --workspace --locked` ve `e2e` özellikli testler | Geçti | Geçti |
| Biçim ve Clippy (tam, `e2e`, yalıtım özellikli) | Geçti | Geçti |
| Gerçek yalıtım testleri | `yalitim` 29/29, `uyumluluk` 7/7 (AppContainer) | `yalitim` 17/17, `uyumluluk` 4/4 (bwrap) |
| 2d-3 birleştikten sonra (ana dal PR dalına birleştirildi) | Çalışma alanı 464, `e2e` özellikli 91, `yalitim` 30/30, `uyumluluk` 11/11 | Çalışma alanı 414, `e2e` özellikli 90, `yalitim` 17/17, `uyumluluk` 6/6 |

### 2d-1 — Windows arka ucu (PR #12)

Plan: [`2026-09-26-kalfa-2d-1-windows.md`](superpowers/plans/2026-09-26-kalfa-2d-1-windows.md).
2d iki plana bölündü; 2d-2 uygulama tarafı ve Ayarlar'dır. Görev 1–4'ü
Codex yazdı. Görev 5'i Codex'in taslağı üzerine, Görev 6'yı ve inceleme
düzeltmelerini Claude yazdı (Codex'in kotası bitti).

Neler eklendi:
- **AppContainer yalıtımı** (aşama 2 tasarımı §9):
  - Komut, geliştiricinin hesabıyla, çalışma alanına özel bir AppContainer'da
    (`Kalfa.Alan.<kimlik>`) ve bir Job Object'te çalışır.
  - Araç zinciri dizinlerine ortak bir yetenek SID'i okuma izni taşır. Ağlı
    çalıştırmada `internetClient` verilir.
  - Kabın ortamı çalışma alanına özel ev, geçici ve önbellek dizinlerini
    gösterir; MSVC ortamını Kalfa verir.
- **`kalfa-yalitim.exe`** (yönetici): `kur` sürücü köklerine ve `C:\Users`'a
  listeleme izni verir; `kaldir` geri alır; `izin-ver` / `izin-geri-al`
  geliştiricinin sahibi olmadığı araç zinciri dizinlerini açar ya da kapatır
  (örneğin `C:\Program Files\nodejs`, `C:\Python314`).
- **Çalışma alanı hazırlığı:**
  - Paket SID'ine çalışma alanında değiştirme izni verilir.
  - Gizli dosyaların mirası kesilir; `.git` ve kancalar salt okunurdur.
  - Üst dizinlere mirassız listeleme verilir (A11).
  - Her değişiklik ACE kaydına yazılır, kaldırmada kayda göre geri alınır.
- **Hazırlığın onayı:** hazırlanmamış çalışma alanındaki ilk yalıtımlı komutun
  onayı her kipte yerel penceredir.
- **Yalıtım dizini:** Windows'ta `%LOCALAPPDATA%\Kalfa\yalitim`.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `cargo test --workspace --locked` | 414 geçti, 0 başarısız | 380 geçti, 0 başarısız (4 atlandı) |
| `cargo test -p kalfa-uygulama -p kalfa --features e2e` | 68 geçti | 68 geçti |
| Clippy (normal, `e2e`, `yalitim-testleri`) / fmt | Başarılı | Başarılı |
| Yalıtım testleri | `yalitim` 27, `uyumluluk` 6 geçti (kurulumdan sonra; `yalitim` üç koşuda) | `yalitim` 17, `uyumluluk` 4 geçti (`kalfatest`) |
| `npm run check` / `npm test` / `npm run build` | 0 hata, 0 uyarı; 20 dosya, 218 test; paket denetimi temiz | Aynı |
| Uçtan uca (`cd e2e && npm test`) | 24 test geçti (iki koşuda); p95 24–25 ms | 25 test geçti (iki koşuda); p95 33–37 ms |

Linux uçtan uca testleri Windows'takilerle aynı anda koşunca gecikme testleri
yük altında düştü (p95 63–108 ms); tek başına koşuda geçti.

Kurulumu yapılmış makinede Windows uçtan uca testlerinin kabuk senaryoları
gerçek yalıtımla koşar ve iki koşuda iki AppContainer profili bıraktı
(çalışma alanları kaldırılmıyor). 2d-2'nin Windows uçtan uca işinde
kapatılır.

A11–A15 (aşama 2 tasarımı §14):
- **A11:** üst dizinlerin listelenmesi gerekiyor. Git for Windows (`getcwd`)
  ve PowerShell (`Set-Location`) istiyor. PSDrive yalnızca PowerShell'i
  kurtarıyor. Hazırlık üst dizinlere paket SID'ine mirassız listeleme verir;
  `C:\Users` yöneticiye ait olduğundan izni kurulum verir. Kap bu dizinlerdeki
  adları görür, içeriklerini göremez.
- **A12:** C derleyen `build.rs` çalışıyor. MSVC dizini `PATH`'in başında
  olmalı ve kabın `TEMP`'i kurulmalı (aşağıda).
- **A13:** ölçülmedi; makinede LAN hedefi, ssh-agent ve Docker yok.
- **A15 (bilinen sınır):** çalışma alanına özel cargo registry'si kapta
  doldurulamıyor. cargo paketi açarken hedefi kanonikleştiriyor; DOS yol adı
  Mount Manager'a erişim istiyor, AppContainer'da "Erişim engellendi". Windows
  yalıtımında crates.io bağımlılığı indirilemez; bağımlılıksız derleme
  çalışır. Yol 2d-2'de seçilir; en olası aday geliştiricinin registry'sinin
  salt okunur verilmesi.

Uygulanırken ve incelemede bulunanlar:
- **Gerçek yalıtım testleri Codex'in ortamında koşamıyordu.** Codex'in kendi
  yalıtımında `CreateAppContainerProfile` açılamıyor. Görev 4'ün testleri ilk
  kez kurulu makinede koştu: 24 testten 4'ü test hatasıyla düştü (betikler
  kendi alanına yazıyordu, `mklink` yanlış kaçışla çağrılıyordu, kabın
  yazamadığı yere işaret konuyordu).
- **Kabın `TEMP`'i yoktu.** Windows, AppContainer süreçlerinde `TEMP`'i
  `%LOCALAPPDATA%\Packages\<paket>\AC\Temp`'e çeviriyor ama dizini kurmuyor.
  PowerShell kendisi kurduğu için görünmüyordu; `link.exe` LNK1104 veriyordu.
- **rustc Git'in `link.exe`'sini çalıştırıyordu.** Kapta Visual Studio'yu COM
  ile bulamayınca `PATH`'teki ilk `link.exe`'ye düşüyor. MSVC dizini sondaydı;
  Git'in MSYS `link.exe`'si kapta 0xC0000142 ile başlayamıyor.
- **`SetNamedSecurityInfoW` üst dizinde dakikalar sürüyordu.** Her yazımda
  mirası bütün alt ağaca yeniden yayıyor. Mirassız izinler artık kök
  iznindeki gibi `SetFileSecurityW` ile yalnızca nesneye yazılıyor.
- **Node ve Python dizinleri uygulama paketlerine kapalı.** Kurucuları DACL'i
  korumalı yazıyor; `izin-ver` gerekiyor.
- **Bir test her koşuda makinede AppContainer profili bırakıyordu** (bozuk ACE
  kaydı testi, profili bilerek korunan durum). Test artık profilini siliyor.
- **`kaldir` bazı hatalarda başarı bildirip kaydı silebiliyordu** (bozuk
  kayıt, okunamayan kök ACL'i, "bulunamadı" dışındaki erişim hatası). Codex
  düzeltti; PR'da onaylanan plandan sapma.
- **Bozuk ACE kaydında eksiksiz geri alma güvenli değil.** Kaydı kaybolmuş bir
  yolun kesik mirası Kalfa'nın mı geliştiricinin mi, ayırt edilemiyor.
  Yalnızca kanıtlanan değişiklikler geri alınıyor; kayıt ve profil korunuyor
  (kullanıcı kararı).

Makine: `kalfa-yalitim kur` kurulu (C:\, D:\ ve C:\Users); `izin-ver` ile
`C:\Program Files\nodejs` ve `C:\Python314` açık. Geri alma `kaldir` ve
`izin-geri-al`'dır.

### Ayrı kullanıcı karşılaştırması ve son kullanıcı kararları (2026-09-27)

Bulgular: [`2026-09-27-kalfa-2d-ayri-kullanici-karsilastirma.md`](superpowers/bulgular/2026-09-27-kalfa-2d-ayri-kullanici-karsilastirma.md).

- **Ölçüm:** yönetici gerekmedi.
  - Kısıtlı belirteçte DOS yol adı çözülüyor ve cargo paketi açıyor (A15
    yok). Ama Schannel `SEC_E_NO_CREDENTIALS` veriyor, cargo indiremiyor (A8).
  - AppContainer'da A15 bir sınıf. Python `realpath` sessizce yanlış yol
    döndürüyor; Node `realpathSync.native` EPERM veriyor (A16).
- **Kullanıcı kararları.** Hedef, teknik olmayan birinin kolay kurulumu:
  - AppContainer kesinleşti.
  - Araç zincirini (Python, Node, Git) Kalfa'nın kurucusu getirir.
  - Rust ve MSVC gelişmiş özelliktir.
  - Yalıtım yoksa kabuk kapalıdır. Kullanıcıya sade ileti, BT'ye teknik
    ayrıntı verilir. Yalıtımsız çalıştırma yalnızca Ayarlar'dan açılır.
- **Tasarım:** aşama 2 tasarımı §9.8 ve §15; üst belge r3.7.

### Dış kaynak incelemesi: awesome-harness-engineering (2026-09-27)

Bulgular: [`2026-09-27-awesome-harness-engineering-inceleme.md`](superpowers/bulgular/2026-09-27-awesome-harness-engineering-inceleme.md).
Liste bir okuma dizini; alınacak kod ya da bağımlılık yok. 2d-2'nin kapsamı
değişmez. Aşağıdakiler 2d-3 ve aşama 3 planlarına girecek:

- **Sınır matrisi (2d-3):** onaylanan tarife × çalışma anı durumu (yalıtım
  var/yok, izin kipi, ağ, yalıtımsız anahtarı) için tablo biçiminde test.
  Uyuşmazlık olan her hücrede süreç başlamaz. 2d-2 incelemesinde kontrol
  listesi olarak da kullanılır.
- **A15/A16 için önce okuma (2d-3):** OpenAI'nin Codex Windows yalıtımı yazısı
  AppContainer'ı kendi iş yükü için uygun bulmuyor. Tasarım kararı değişmez;
  plan yazılmadan önce hangi araçlarda benzer sorun çıkacağını görmek için
  okunur.
- **Senaryo testleri (aşama 3):** sahte EVREN ile çok turlu, betikli model
  yanıtları. Her senaryo iki şeyi ayrı sınar: son durum (çalışma alanı, JSONL)
  ve iz (onay, tarife, yalıtım). İlk adaylar: sunulmayan kabuğun çağrılması,
  onaydan sonra durum değişmesi, onay beklerken kapanma, kurtarmada yan etkinin
  yinelenmemesi. Komut dizisi değil güvenlik değişmezleri sınanır. Bağlam
  özetlemenin onay kapsamını genişletmediği de bu altyapıyla sınanır.
- **Araç sonuçları (aşama 3):** modele dönen hata ve kırpma sonuçları "ne oldu,
  şimdi ne yapabilirsiniz" biçiminde yazılır; kullanıcıya sade ileti, BT'ye
  teknik rapor ayrımı korunur.
- **Uymayanlar:** Claude API'nin sunucu tarafı özetlemesi (EVREN üzerinden
  gidiyoruz; özetleme bizde), yeni ajan çatıları, vektör veritabanı, onayı
  sınıflayıcıyla gevşetmek.

### 2d-2 devam noktası (2026-09-27)

Plan: [son kullanıcı akışı](superpowers/plans/2026-09-27-kalfa-2d-2-son-kullanici.md).
Görev 1–8'in kodu PR dalında. Görev 6 hazırlanmış alanları, Görev 7 profil
sayımı ve alan temizliğini, Görev 8 NSIS kancalarını ve kullanıcı kabul/geri
alma betiklerini ekledi. Görev 9 bu doğrulama kaydını güncelledi.

Claude'un incelemesi ve iki platformdaki doğrulaması (kurulu Windows makinesi):

| Doğrulama | Windows | WSL Ubuntu-24.04 |
|---|---|---|
| Biçim ve Clippy (tam, `e2e`, yalıtım özellikli) | Geçti | Geçti |
| `cargo test --workspace --locked`, `e2e` özellikli testler | Geçti | Geçti |
| Gerçek yalıtım testleri | 29/29 (AppContainer) | 17/17 (bwrap); uyumluluk 4/4 |
| Arayüz denetim, test ve üretim derlemesi | 0 hata/uyarı; 229/229 test; paket denetimi temiz | — |
| Uçtan uca | 25/25; "kabuk yalıtımı" gerçek AppContainer'da koştu; `Kalfa.Alan.*` profil sayısı koşudan önce ve sonra aynı | 25/25 (Codex) |

İncelemede düzeltilenler plandaki inceleme notunda. Sürüm yükseltmede araç
zinciri dizinlerinin izinlerinin gitmesi kullanıcı kararıyla bilinen sınırdır
(2d-3'te yeniden ele alınır). §9.8'deki araç zinciri kurucuda kararı plan
gereği 2d-3'tedir.

Kullanıcının açık izniyle `@tauri-apps/cli@2.11.4` ve Tauri'nin NSIS araçları
indirildi. Windows x64 kurucusu
`target/release/bundle/nsis/Kalfa_0.1.0_x64-setup.exe` üretildi.

**Kurucu kabulü (kullanıcı, UAC):** kurulum, kurulu sürümde `Hazir` durumu,
bozulan kurulumu "Onar"la düzeltme ve kaldırma denendi. Kaldırmadan sonra
kök ve araç zinciri izinleri, kurulum kaydı ve Program Files dizini kalmadı.
Ayrıntı plandaki kabul notunda. 2d-2 tamamlandı.

### 2d-3 devam noktası (2026-09-27)

Plan: [araç zinciri](superpowers/plans/2026-09-27-kalfa-2d-3-arac-zinciri.md),
PR #15 (`2d-3/arac-zinciri`). Kodu Claude yazdı; G1–G6 commit'lendi, her
görevin sonucu ve sapmaları plandaki "Sonuç" notlarında.

- **G1:** A16 ölçümü ([bulgular](superpowers/bulgular/2026-09-27-kalfa-2d-3-a16.md));
  Python 3.14.7 (NuGet) ve Node 24.21.0 seçildi; Python Kalfa'nın
  `sitecustomize.py`'siyle çalışıyor.
- **G2 / G2b:** onaylanan kip ile çalışma anı `kip_uyusmasi`'nda, 16 hücreli
  sınır matrisi; bağımlılık dizinleri gizli kalıpların dışında.
- **G3:** kurucu araçları indirir, özeti denetler, açar (5,9 → 42,7 MB).
- **G4:** kurulu Kalfa'nın araçları kabın `PATH`'inde kullanıcınınkilerden önce.
- **G5:** `kalfa-yalitim kur --izin-ver`, `okunur_yonetici_gerekli`, Onar ve
  "Ekle"de UAC; Windows'ta araç zinciri dizinleri Ayarlar > Gelişmiş'te.
- **G6:** ağsız cargo geliştiricinin `.cargo`'suyla; kapa yalnızca `registry`
  ve `git` açılır, kimlik dosyaları okunamaz.
- **G7:** belgeler; yükseltmeden sonra yalıtım "Hazır" görünürken kaybolan
  dizin izinleri için Ayarlar > Gelişmiş'te Onar (kabul adımını yazarken
  bulundu).
- Git dağıtılmaz; kaldırmada oturum kayıtları silinmez (tasarım §9.8).

Son doğrulama (tam, PR dalının son hâli):

| Doğrulama | Windows | WSL Ubuntu-24.04 |
|---|---|---|
| Biçim ve Clippy (tam, `e2e`, yalıtım özellikli) | Geçti | Geçti |
| `cargo test --workspace --locked` | 459 | 409 (Onar düzeltmesinden sonra uygulama crate'leri yeniden: 81/81) |
| `e2e` özellikli testler | 90 | 89 |
| Gerçek yalıtım testleri | 30/30 (AppContainer) | 17/17 (bwrap, `kalfatest`) |
| Uyumluluk | 10/10 (`KALFA_TEST_ARAC_ZINCIRI` ile) | 6/6 |
| Kurucu betik testleri | 5/5 | 3/3 (2 zip testi atlandı) |
| Arayüz denetim, test ve derleme | 0/0; 231 test; paket denetimi temiz | — |
| Uçtan uca | 25/25; `Kalfa.Alan.*` profili kalmadı | — (arayüzün Linux yolu değişmedi) |

**Kurucu kabulü (kullanıcı, UAC, 2026-09-28):** kurulum, Kalfa'nın Python,
Node ve Git'iyle yalıtımda komutlar, Gelişmiş'ten UAC ile dizin ekleme,
yükseltmenin izinlere etkisi ve Onar'la geri gelmesi, temiz kaldırma ve
geliştirme ortamının geri kurulması denendi. Ayrıntı plandaki kabul notunda.
Kabulde bulunan `\\?\` önekli yol görüntüsü düzeltildi. Aynı turdaki paralel
kabuk çağrılarının ilk hazırlıkta düşmesi 2d-3'ten önceki bir hatadır; ayrı
işe ayrıldı. 2d-3 tamamlandı; sıradaki PR #15'in birleştirilmesi.

### Güvenlik sertleştirmesi (PR #16, 2026-09-28)

[Güvenlik analizi](superpowers/bulgular/2026-09-27-guvenlik-analizi.md) ve
[plan](superpowers/plans/2026-09-27-kalfa-guvenlik-sertlestirme.md); kararlar
ve plandan sapmalar planın "Kararlar" ve "Uygulama sonucu" bölümlerinde.

- **Bulgu 1 (yüksek):** dosya araçları `.git/commondir` yazabiliyordu; git bununla
  yalıtım dışında kod çalıştırıyordu. Artık her derinlikteki `.git`'in tamamı
  korumalı (onay ister), 8.3 kısa adla da yazılamaz. Yalıtımda bütün `.git`
  girdileri salt okunur; komutun oluşturduğu tehlikeli yeni `.git` izin
  listesiyle bulunur, `.git.kalfa-karantina-<zaman>` olarak kenara alınır ve
  "Güvenlik uyarısı" olarak gösterilir.
- **Bulgu 2:** `.vscode/`, `.cargo/config*`, `.envrc`, CI dosyaları gibi
  kendiliğinden çalışan yapılandırmaya yazmak her kipte onay ister; yalıtımda
  salt okunur, yenisi bildirilir.
- **Bulgu 3:** kabuk onayı oturumdaki onaysız değişiklikleri ve derleme
  dosyalarını gösterir; aynı adımdaki yazmalar da onaya çıkar; riskli öğeler
  "Tümünü onayla" ile geçmez.
- **Bulgu 4:** gizli dosya kalıpları genişledi.
- **Bulgu 5:** araç çıktısı çağrı başına rastgele sınırlı sarmalla gelir.
- **Bulgu 6:** yönetici yardımcısı kayıt dizininin sahibini ve reparse
  noktasını denetler, DACL'i tutamaçla yazar.
- **Bulgu 7:** görünmez ve yön değiştiren karakterler onay yüzeylerinde
  `⟨U+XXXX⟩` olarak görünür ve onay ister.

Son doğrulama (tam, PR dalının son hâli):

| Doğrulama | Windows | WSL Ubuntu-24.04 |
|---|---|---|
| Biçim ve Clippy (tam, `e2e`, yalıtım özellikli) | Geçti | Geçti |
| `cargo test --workspace --locked` | 513 | 458 |
| `e2e` özellikli testler | 92 | 91 |
| Gerçek yalıtım testleri | 30/30 (AppContainer) | 17/17 (bwrap, `kalfatest`) |
| Uyumluluk | 14/14 | 9/9 |
| Arayüz denetim, test ve derleme | 0/0; 242 test; paket denetimi temiz | — |
| Uçtan uca | 25/25 | 25/25 (`xvfb-run`) |

Testler makinede fazladan Kalfa AppContainer profili bırakmadı. Uçtan uca
`05-kabuk`'taki etiket beklentisi yeni sarmala taşındı; ilk koşuda yalnız o
test eski beklentiyle düştü, düzeltmeden sonra iki platformda geçti.

**Açık işler:**
- Görev 3'ün kurulu makinede yönetici olarak kabulü (kullanıcı; UAC): kurulum,
  "Onar" ve kaldırma kayıt dizininin yeni denetimiyle.
- Bulgu 8, ek gizli kalıplar ve güvenlik denetçisi aşama 3'e taşındı
  (aşağıda).

Bağlam özetleme, oturum bütçesi ve WASM eklentileri sonraki aşamalardadır.
Masaüstü uygulaması Windows ve Linux'ta çalışıyor; tamamlanmış bir v1 sürümü
yoktur.

## Devam eden aşama — 3 (görünürlük)

Tasarım:
[`2026-09-28-kalfa-3-gorunurluk-design.md`](superpowers/specs/2026-09-28-kalfa-3-gorunurluk-design.md);
üst belge r3.9. Aşama 3 beş plana bölünür:

- **3a:** tur özeti ve geri alma.
- **3b:** gönderim günlüğü ve önizleme.
- **3c:** prompt injection senaryo testleri ve güvenlik denetçisi.
- **3d:** ağ çıkışının kısıtlanması (Bulgu 8).
- **3e:** "bu oturumda izin ver" kısayolları, ek gizli kalıplar ve onay
  kartının kalan alanları.

Bağlam özetleme ve oturum bütçesi aşama 3'ün dışındadır.

### Kullanıcı kararları (2026-09-28)

1–4 tasarım yazılmadan önce, 5–10 PR'da verildi (tasarım §2.1, K1–K10).

1. **Önceki içerik oturumla birlikte saklanır.**
   - Veri dizininde, kaydın dışındaki bir depoda durur. Dosyaların adı
     içeriklerinin özetidir; kayda yalnızca özet girer.
   - Oturum silinene kadar kalır; 3a'ya "Oturumu sil" eklenir.
   - Sınırlar: dosya başına 8 MiB, oturum başına 512 MiB. Aşılınca en eski
     turların verisi silinir.
2. **Kabuğun değiştirdiği dosyalar geri alınamaz, listelenir.** Komuttan önce
   ve sonra meta veri karşılaştırılır.
3. **Sonradan değişmiş dosyaya geri almada dokunulmaz;** dosya çakışma olarak
   listelenir. Önizleme hangi dosyanın döneceğini önceden gösterir.
4. **Varsayılan listede olmayan host çağrıda istenir** (`ag_hostlari`); onay
   kartında görünür ve mühre girer. Varsayılan liste crates.io, npm, PyPI ve
   GitHub'ın indirme uçlarıdır.
5. **`github.com` varsayılan listede değildir.** git'in `push` ucudur; git
   işlemleri onu çağrıda ister.
6. **Windows'ta vekilin ölçümü olumsuz çıkarsa** ağlı komut bugünkü gibi
   `internetClient` ile çalışır; kart "Ağ: sınırsız (Windows'ta vekil yok)"
   der.
7. **Varsayılan gizli kalıplar kapatılamaz.** Yanlış pozitif (örneğin
   `src/credentials.rs`) Ayarlar'daki "gizli sayılmayanlar" listesiyle
   çözülür; eklemek yerel pencerede onaylanır.
8. **Gizli dosyanın önceki hâli de saklanır;** oturum silinince o da silinir.
9. **Geri alınamayacak kadar büyük dosyaya yazmak her kipte onay ister;**
   kayıtsız oturum hariç.
10. **3a'yı Claude yerel makinede uygular.**

Tasarımın kendi verdiği kararlar ve gerekçeleri tasarım belgesinin
§2.2'sindedir. Windows'a ve Linux'a özgü davranışlar (AppContainer'da geri
döngü, bwrap'ta köprü, Windows'ta tarama maliyeti) bu belge yazılırken
ölçülmedi; açık noktalar G1–G17'dir.

### 3a — tur özeti ve geri alma (uygulandı; PR incelemede)

Plan: [`2026-09-28-kalfa-3a-geri-alma.md`](superpowers/plans/2026-09-28-kalfa-3a-geri-alma.md).
Dal `3a/geri-alma`, PR #19 (tabanı `3/tasarim`, PR #18). Hafif bir plandı;
platformdan bağımsız görevleri bir bulut oturumu Linux'ta, kalanları ve iki
platformdaki doğrulamayı Claude yerel makinede yaptı.

- **Tur özeti:** her tur tasarım sistemindeki `TurnSummary` ile biter:
  dosyalar, komutların değiştirdiği dosyalar, EVREN istekleri, güvenlik
  uyarıları.
- **Geri alma:**
  - Dosya araçları yazmadan önce önceki hâli oturumun deposuna koyar
    (Windows'ta `%LOCALAPPDATA%\Kalfa\geri-alma`) ve "başladı" kaydına yazar.
  - "Turu geri al…" önce planı gösterir, sonra mühürle yürütür. Sonradan
    değişen, bir komutun değiştirdiği ya da hard link'li dosyaya dokunmaz;
    nedenini söyler.
  - Kayıttan açılan oturumda da çalışır. Model bir Kalfa notuyla bilgilenir.
- **Kabuk:** komuttan önce ve sonra meta veri görüntüsü alınır. Değişen
  dosyalar tur özetinde ve sonraki kabuk onayında listelenir, geri alınmaz.
- **Onay kartı:** her yazan öğe için "Geri alınabilir mi". 8 MiB'tan büyük
  dosyaya yazmak kayıtlı oturumda her kipte onay ister.
- **Oturumu silme:** yerel pencerede onaylanır; kayıt, kilit dosyası ve geri
  alma verisi birlikte silinir.
- **Ölçümler:**
  - G1: 100 000 dosyada görüntünün eki Windows'ta 17 ms, WSL'de 45 ms.
  - G10 kapandı: depo yalıtımın içinden görünmüyor.
  - G2 kapandı: üç model art arda iki kullanıcı mesajını kabul ediyor
    (canlı test).

Ayrıntılar, plandan sapmalar ve doğrulama tablosu planın "Uygulama sonucu"
bölümünde.

**Açık işler:**
- Araç planlaması her yazılan hedef için `git config` çalıştırıyor (Windows'ta
  ~250 ms). Ayrı bir iş olarak işaretlendi.

**Sıradaki iş:**
- PR #18 (tasarım) ve PR #19'un (3a) incelenmesi ve birleştirilmesi.
- Ardından 3b (gönderim günlüğü). Windows'ta G6'nın ölçümü (3d) ayrıca
  bekliyor.
- 3c'nin senaryo testleri (PR #21) ve 3d'nin prototipi (PR #20) açık.
