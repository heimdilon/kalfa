# Kalfa 1c — Masaüstü kabuğu (Tauri + Svelte): Tasarım

**Tarih:** 2026-09-22
**Durum:** §1–§9 kullanıcıyla bölüm bölüm konuşulup onaylandı. §10–§14,
kullanıcının "sormadan yaz" isteğiyle ayrıca onaya sunulmadan yazıldı. Sıradaki
adım uygulama planı.
**Üst belge:** [Kalfa tasarım belgesi](2026-09-21-kalfa-design.md), özellikle
§4, §6.1, §6.6, §11 ve §12.3. Bu belgede `§` işaretleri bu belgenin
bölümlerini gösterir; üst belgeye "üst belge §x" diye başvurulur.
**Temel aldığı iş:** çekirdek olay kuyruğunun olay sayısı ve bayt bütçesiyle
sınırlanması ([plan](../plans/2026-09-22-kalfa-1c-olay-aktarimi.md)). Bu belge
yazılırken o değişiklik çalışma ağacındaydı, commit'lenmemişti.
**Görsel kaynak:** `design/kalfa-ds/` (Claude Design'daki "Kalfa Design
System"in anlık kopyası). Spec'le çelişen içerikleri `design/README.md` listeler.

---

## 1. Amaç

1c, üst belgedeki teslim sırasının (üst belge §12.3) birinci aşamasını kapatır.
1a ve 1b'nin arayüzsüz çekirdeği, Windows ve Linux'ta çalışan bir masaüstü
uygulamasına dönüşür. 1c bittiğinde kullanıcı:

- ilk açılışta API anahtarını girer, EVREN şartlarını okuyup kabul eder;
- bir çalışma alanı seçip oturum açar, mesaj gönderir, yanıtı akarken izler;
- dosya yazma ve düzenleme isteklerini diff'iyle inceleyip onaylar ya da reddeder;
- süren turu iptal eder;
- oturumu kapatıp daha sonra kayıttan yeniden açar.

### 1.1 Kapsam

1. Anahtar girişi ve kimlik deposu, şart metni ve onayı, çalışma alanı seçimi.
2. `Komut`/`Zarf<Olay>` köprüsü, akış görünümü, iptal, araç onay kartları ve
   kayıttan oturum açma.
3. Dosya sistemi kimliği üzerinden çalışma alanı başına ortak kilit yöneticisi;
   oturum kaydı başına tek yazıcı.
4. Olay birleştirme ve sert bayt/olay bütçeli IPC. Çekirdek kuyruğundaki sınır
   Tauri/webview boyunca korunur ve gerçek arayüzle ölçülür.
5. Pencereye göre komut yetkileri, sıkı CSP ve güvenli Markdown gösterimi.

### 1.2 Kapsam dışı

| Konu | Ne zaman |
|---|---|
| Sağdaki dosya paneli (inspector, `FileTree`) | Sonraki bir aşama. Arayüzün çalışma alanını ajan döngüsü dışında okumasını ve gizli dosya kuralını (üst belge §7.4) arayüzde de uygulamasını gerektirir |
| Tur özeti ve geri alma (`TurnSummary`) | Aşama 3 |
| Gönderim günlüğü ve önizleme (`LogTable`) | Aşama 3 |
| Onay kartının tam hâli (veri çıkışı, yalıtım, geri alınabilirlik, kapsam) ve "bu oturumda" kapsamı | Aşama 3. 1c'de kapsam her zaman "yalnızca bu çağrı" |
| Kabuk aracı, yalıtım ve yalıtımsız kabuk onayı | Aşama 2 |
| Eklentiler | Aşama 4 |
| "Paylaş" düğmesi | v1 dışında (üst belge §12.2) |
| İngilizce arayüz | v1'den önce; 1c yalnızca Türkçe |
| Kayıtsız oturum seçeneği ve oturum silme | 1c'den sonra. Çekirdek kayıtsız oturumu zaten destekliyor |
| Mesaj listesinin tam sanallaştırması | Uzun oturum ölçümü gerektirirse (§9.3) |

---

## 2. Başarı ölçütleri

- §1'deki akış Windows 11 ve Ubuntu 24.04'te gerçek uygulamada baştan sona
  çalışır (§11.4).
- Üst belge §11.1'deki kabul kapısı geçer: token'ın ekrana düşme gecikmesi
  p95 < 50 ms; duraklayan arayüzde taşıma belleği sınırlı kalır, bütçe aşılınca
  tur iptal edilir ve kullanıcıya bildirilir; arayüz koptuğunda süren tur iptal
  edilir (§11.4).
- Güvenlik sınırları testle kanıtlanır: ana pencere şart kabul edemez ve pencere
  açamaz; anahtar ana pencereye hiç girmez; CSP satır içi betiği engeller;
  Markdown'daki betik, uzak görsel ve tehlikeli bağlantı çalışmaz (üst belge
  §13.3).
- `cargo test --workspace`, Clippy (`-D warnings`) ve `cargo fmt --check`;
  `ui/` için `npm ci`, `npm run check`, `npm test` ve `npm run build` temiz geçer.
- `kalfa-core` Tauri'ye ve `kalfa-tools`'a bağımlı olmaz (`cargo tree` denetimi
  sürer).

---

## 3. Mimari

### 3.1 Repo yerleşimi

```
crates/
  kalfa-evren/        (var) EVREN istemcisi
  kalfa-core/         (var) çekirdek; 1c'de yeni modüller: aktarim, sart_kapisi, gecmis
  kalfa-tools/        (var) dosya araçları
  kalfa/              (yeni) Tauri uygulaması ve binary (üst belge §4)
    tauri.conf.json
    capabilities/     ana-pencere.json, sart-penceresi.json
    src/
ui/                   (yeni) Svelte 5 arayüzü
  package.json, package-lock.json, vite.config.ts, tsconfig.json
  index.html          ana pencere
  sartlar.html        şart penceresi
  src/
    stiller/          tokenlar (design/kalfa-ds/tokens'tan kopya, fonts.css hariç)
    fontlar/          Source Sans 3, Source Serif 4, JetBrains Mono (woff2) ve OFL lisans metinleri
    bilesenler/       tasarım sistemindeki adlarla: Button.svelte, ToolCallCard.svelte …
    durum/            kanal mesajı → oturum durumu işlevi
    markdown/         güvenli Markdown çizicisi
    dil/              Türkçe metin kataloğu
    ana/, sartlar/    iki pencerenin ekranları
```

- Arayüz Vite + Svelte 5 (runes) + TypeScript ile yazılır. SvelteKit yok:
  masaüstü uygulamasında sunucu tarafına ve sayfa yönlendirmesine gerek yok;
  bağımlılık ve CSP ile çatışabilecek parça azalır.
- Paket yöneticisi npm. `package-lock.json` commit'lenir, kurulum `npm ci` ile
  yapılır.
- İki pencere ayrı HTML giriş noktalarından yüklenir. Şart penceresinin
  paketinde yalnızca anahtar formu, şart metni ve güvenli Markdown çizicisi
  bulunur; sohbet ekranının kodu yoktur. Bu, üst belge §11.9'daki pencere
  ayrımına derleme düzeyinde ikinci bir katman ekler.
- Tokenlar `ui/src/stiller/`'e kopyalanır. `design/kalfa-ds/` referanstır,
  derlemeye girmez. Lucide ikonları npm paketinden gelir. Dış kaynaktan font,
  ikon ya da stil yüklenmez.
- `kalfa` crate'i ince tutulur: pencere kurulumu, Tauri komutları, kanal
  bağdaştırıcısı, kimlik deposu ve yerel iletişim kutuları. Test gerektiren
  mantık Tauri'ye bağlı olmayan crate'lerde durur.
- Sonuçları: `kalfa` workspace'e girince Linux'ta `cargo test --workspace`
  WebKitGTK geliştirme paketlerini ister. `cargo test`'in önce `npm run build`
  gerektirip gerektirmediği plan aşamasında doğrulanır (§13).

### 3.2 Birimler

| Birim | Yer | Sorumluluk |
|---|---|---|
| Paketleyici | `kalfa-core::aktarim` | Olay kuyruğundan okur; pencere, birleştirme, kritik olayların hemen gönderilmesi, kredi, paket numarası (§4) |
| Şart kapısı | `kalfa-core::sart_kapisi` | Uygulama genelinde tek; şart durumu, metni ve kabulü; gösterilen sürümün denetimi; bekleyen oturumları uyandırma (§7.3) |
| Kilit yöneticisi | `kalfa-core::kilit` | Çalışma alanı kökünün dosya sistemi kimliği → paylaşılan `KaynakKilitleri` (§5.3) |
| Kayıt sahipliği | `kalfa-core::kayit` | Kaydın yanındaki kilit dosyasında işletim sistemi kilidi (§5.2) |
| Geçmiş ve oturum listesi | `kalfa-core::gecmis` | Kayıt öğelerinden ekran öğeleri, tur bazlı sayfalama, kayıt dizininden oturum listesi (§5.4, §5.5) |
| Oturum tablosu | `kalfa` | Açık oturumlar; her birinin çekirdek tutamacı, aktarım görevi ve bağlı olduğu pencere (§5.1) |
| Köprü | `kalfa` | Paketleri `Channel`'a yazar; "işledim" bildirimini paketleyiciye iletir |
| Komutlar ve yetkiler | `kalfa` | Pencere başına Tauri komutları, uygulama manifest'i, capability dosyaları (§6) |
| Kimlik deposu | `kalfa` | `keyring`; depo yoksa bellekte tutma (§7.2) |
| Yerel iletişim kutuları | `kalfa` | Klasör seçici, izin kipi yükseltme onayı, bağlantı açma onayı |
| Arayüz durumu | `ui/src/durum` | Kanal mesajını oturum durumuna uygulayan saf işlev (§8.4) |
| Markdown çizicisi | `ui/src/markdown` | Sözdizimi ağacından Svelte bileşenleri; `{@html}` yok (§9.1) |

### 3.3 Çekirdekteki değişiklikler

1c'nin çekirdekte yaptığı, arayüzden bağımsız değişiklikler:

- **`aktarim` modülü** (yeni): paketleyici ve kredi (§4).
- **Şart kapısı** (yeni): oturumun şart akışı kapıya devredilir; oturum kabul
  isteğini artık kendisi göndermez. `SartlariKabulEt` oturum komutu olmaktan
  çıkar, kapının girdisi olur; kabul edilen sürüm her bekleyen oturumun kaydına
  yazılır. `Saglayici` trait'ine şart durumu sorgusu eklenir (§7.3).
- **Kilit yöneticisi** (yeni): kök dizinin dosya sistemi kimliğine göre paylaşım.
  `KaynakKilitleri`'nin yol tablosu bugün hiç küçülmüyor; kullanılmayan girdiler
  temizlenir (§5.3).
- **Kayıt kilidi:** `OturumKaydi` oluşturulurken ve devam için açılırken kilit
  alınır; kayıttan devamda kilit okumadan önce alınır (§5.2).
- **`OturumAcildi` kaydı** çalışma alanı yolunu da taşır. Bu alanı olmayan
  kayıtlar açılabilir ama yalnızca geçmişleri gösterilir (§10).
- **İstek bilgisi:** `x-evren-routed-model` başlığı bugün ayrıştırılıyor ama ne
  olay olarak yayılıyor ne kayda yazılıyor. Durum çubuğu (§8.3) için
  yönlendirilen model olay olarak yayılır; yönlendirilen model ve token
  kullanımı kayda da yazılır. Böylece kayıttan açılan oturumun durum çubuğu
  doğru başlar.
- **Geçmiş görünümü ve oturum listesi** (yeni; §5.4, §5.5).
- **Metin tonu:** kullanıcıya giden hata metinleri "siz" diline çevrilir (§9.4).

### 3.4 Veri akışı

```
Ana pencere ──mesaj_gonder──► kalfa: oturum tablosu ──Komut──► kalfa-core: oturum
                                                                   │ Zarf<Olay>
                                                                   ▼
                                                  olay kuyruğu (sayı + bayt bütçesi)
                                                                   │
                                                                   ▼
Ana pencere ◄──Channel<KanalMesaji>── kalfa: köprü ◄── kalfa-core: paketleyici
     │                                                             ▲
     └────────────────────── paketi_isledim(no) ───────────────────┘
```

---

## 4. Olay köprüsü (üst belge §11.1)

**Karar:** Tauri `Channel` ile itme ve kredi.

Değerlendirilen alternatif, arayüzün her paketi invoke ile kendisinin
istemesiydi. Kredi hesabı gerektirmezdi ve yolda en fazla bir paket olurdu.
Ama üst belgedeki `Channel` kararını değiştirir ve oturum boştayken uzun süre
açık bekleyen istekler bırakırdı.

### 4.1 Mevcut durum ve kapatılacak boşluk

Çekirdek kuyruğu olay sayısı ve serileştirilmiş JSON baytıyla sınırlıdır
(varsayılan 16.384 olay ve 8 MiB). Kuyruk dolarsa tur iptal edilir, oturum
kapanır ve tüketiciye ayrılmış kapasiteden son bir hata olayı gider.

Tauri'nin `Channel`'ı ise karşı tarafın okuyup okumadığına bakmadan gönderir.
Arayüz takılırsa mesajlar webview'da sınırsız birikir. Köprünün işi bu boşluğu
kapatmaktır: Rust kuyruğunu sınırlamak webview kuyruğunu kendiliğinden
sınırlamaz.

### 4.2 Paketleme

- İlk olay 16 ms'lik bir pencere açar. Pencerede biriken olaylar tek pakette
  gider.
- Art arda gelen aynı türden metin parçaları (`TokenParcasi`, `DusunceParcasi`)
  tek parçada birleştirilir. Birleştirme yalnızca bitişik ve aynı turdaki
  parçalar arasında yapılır; araya giren başka türden bir olay birleştirmeyi
  keser.
- Onay, hata, tamamlanma, araç ve şart olayları (`OnayGerekiyor`, `Hata`,
  `Tamamlandi`, `AracCagrisiBasladi`, `AracSonucu`, `SartOnayiGerekiyor`)
  birleştirilmez ve düşürülmez. Böyle bir olay gelince pencere beklenmez, paket
  hemen gönderilir.
- Paketin bir bayt üst sınırı vardır; sınıra ulaşan paket pencereyi beklemeden
  gider. Sınırdan büyük tek bir olay kendi paketinde, bölünmeden gider.
- Başlangıç değerleri 16 ms ve 64 KiB'tır; §11.4'teki ölçümle ayarlanır.

### 4.3 Kredi

- Arayüzün "işledim" demediği veri en çok 4 paket ya da 256 KiB olabilir
  (başlangıç değerleri; ölçümle ayarlanır).
- Kredi bitince paketleyici çekirdek kuyruğundan okumayı bırakır. Arayüz uzun
  süre okumazsa çekirdek kuyruğu dolar ve mevcut iptal yolu çalışır.
- Krediden büyük tek bir olay (örneğin büyük bir diff), yolda başka paket
  yokken tek başına gönderilir.
- Böylece bir oturumun taşıma belleği çekirdek bütçesi, kredi ve hazırlanmakta
  olan tek paketle sınırlıdır.
- Arayüz "işledim" bildirimini paketi kendi durumuna uyguladığı anda gönderir,
  ekrana çizilmesini beklemez. Çizimi bekleseydi küçültülmüş bir pencere okumayı
  bırakmış sayılır ve turu iptal ettirirdi.

### 4.4 Kanal mesajları ve numaralandırma

```
KanalMesaji    ::= Olaylar  { no: u64, zarflar: [Zarf<Olay>] }
                 | Bildirim { no: u64, bildirim: KopruBildirimi }
KopruBildirimi ::= AktarimDurdu { neden } | OturumKapandi { neden }
```

- `no` her abonelikte 1'den başlar ve boşluksuz artar. Arayüz boşluk görürse
  aboneliği bırakır ve oturumu kayıttan yeniden açar. Çekirdek kuyruğu olay
  düşürmediği ve `Channel` sırayı koruduğu için boşluk ancak bir hatanın
  belirtisidir.
- `Zarf.sira` kayıt satırlarıyla ortak sayaçtan artar. Bu yüzden olay akışında
  boşluk olağandır ve `sira` yalnızca sıralama için kullanılır. Ortak sayacın
  faydası, kayıttan okunan geçmişle canlı olayların tek sırada birleşmesidir
  (§5.5). Üst belge §4.1'deki "boşluk görürse durumu yeniden ister" kuralı bu
  biçimde güncellendi.
- Aktarım durduğunda (çekirdek bütçesi aşıldı ya da alıcı kapandı) köprü
  `AktarimDurdu` bildirimini gönderir. Arayüz çekirdeğin teknik metnini değil,
  ne olduğunu ve ne yapılacağını gösterir (§10).

---

## 5. Oturum yaşam döngüsü, kilitler ve tek yazıcı

### 5.1 Oturum tablosu

`kalfa`'daki oturum tablosu açık oturumları tutar: her oturumun çekirdek
tutamacı, aktarım görevi ve bağlı olduğu pencere. Aynı oturum ikinci kez
açılmaz; açık olana geçilir.

- **Yeni oturum:** çalışma alanı yalnızca yerel klasör seçiciden gelir. Arayüz
  yol gönderemez; uygulamanın verdiği kimliklerle başvurur. Kimlikler klasör
  seçiciden ya da kayıtlardaki çalışma alanlarından üretilir ve yalnızca o
  çalıştırma boyunca geçerlidir. Böylece webview'a sızmış bir betik bir oturumu,
  örneğin ev dizininde, açtıramaz.
- **Kayıttan açma:** önce kayıt kilidi alınır, sonra kayıt okunur ve çekirdeğin
  `devam_et` işlevi çağrılır. `oturum_ac` son 20 turu döndürür (§9.3).
- **Kapatma:** kullanıcı oturumu kapatınca süren tur iptal edilir, olay alıcısı
  bırakılır, kayıt kilidi çözülür.
- **Kayıtların yeri** üst belge §6.6'daki dizindir: Windows'ta
  `%APPDATA%\Kalfa\oturumlar`, Linux'ta `$XDG_DATA_HOME/kalfa/oturumlar`. Dosya
  adı oturum kimliğidir.

### 5.2 Tek yazıcı

- Her kaydın yanında bir kilit dosyası (`<oturum>.jsonl.kilit`) bulunur. Kayıt
  açık kaldığı sürece bu dosyada işletim sistemi kilidi tutulur
  (`std::fs::File::try_lock`).
- Kilit kaydın kendisine konmaz: Windows'ta dosya kilidi aynı süreçteki başka
  tutamaçlardan yapılan okumaları da engeller. Oturum listesi ve geçmiş sayfaları
  kayıt açıkken de okunabilmelidir.
- Kilit `kalfa-core`'daki `OturumKaydi`'de alınır. Böylece testler dahil, kaydı
  açan her kod bu kurala uyar.
- **Tek Kalfa süreci:** uygulama ikinci kez başlatılırsa mevcut pencere öne
  gelir (Tauri'nin single-instance eklentisi). Kilit yöneticisi süreç içidir;
  iki Kalfa süreci aynı çalışma alanında birbirinin kilitlerini görmezdi. Kayıt
  kilidi ise süreçler arasında da çalışır.

### 5.3 Kilit yöneticisi

- Çalışma alanı kökünü dosya sistemi kimliğiyle tanır: Windows'ta birim seri
  numarası ve dosya dizini, Linux'ta aygıt ve inode. Bunun için `same-file`
  crate'i kullanılır (workspace'te zaten dolaylı bağımlılık).
- Aynı kökü açan oturumlar, kök farklı yazılmış olsa da (büyük/küçük harf,
  junction, sembolik bağlantı) aynı `KaynakKilitleri`'ni paylaşır.
- Yönetici kökleri zayıf referansla tutar; son oturum kapanınca o kökün kaydı
  silinir. Yol kilidi tablosundaki kullanılmayan girdiler de temizlenir.
- **Bilinen sınır:** iç içe iki çalışma alanı (`proje` ve `proje/alt`) ayrı
  yönetici alır. Bu durumda bile `kalfa-tools`'un "onaydan sonra değişen dosyaya
  yazılmaz" kuralı, başka oturumun değiştirdiği dosyanın üzerine sessizce
  yazılmasını büyük ölçüde önler.

### 5.4 Oturum listesi

- Liste kayıt dizininden çıkarılır; ayrı bir dizin dosyası tutulmaz. Kaynak her
  zaman kayıttır.
- Her kaydın başından (ilk 16 KiB) çalışma alanı ve başlık okunur. Başlık ilk
  kullanıcı mesajının ilk satırıdır, en çok 80 karakter.
- Sonundan (son 64 KiB) son durum okunur: tamamlandı, iptal edildi, hata, yarım
  kaldı (son turun `TurBitti` kaydı yok) ya da okunamadı.
- Liste son değişiklik zamanına göre sıralanır.

### 5.5 Geçmiş ve canlı olaylar

- Kayıttan açılan oturumun ekrandaki geçmişi kayıt öğelerinden üretilir:
  kullanıcı mesajı, yanıt metni, düşünce, araç çağrısı ve sonucu (durumuyla),
  tur sonu nedeni. "Başladı" kaydı olup "bitti" kaydı olmayan çağrı "sonucu
  bilinmiyor" olarak görünür ve yeniden çalıştırılmaz.
- Her öğe kayıttaki `sira`'yı taşır. Canlı olaylar aynı sayaçtan devam ettiği
  için arayüz ikisini tek sırada birleştirir.

### 5.6 Arayüz kopması

- Sayfa yeniden yüklenirse ya da pencere kapanırsa, o pencereye bağlı oturumlar
  kapanır: süren tur iptal edilir, kayıt kilidi bırakılır (üst belge §11.1).
- Yeniden yüklenen arayüz son açık oturumu kayıttan açar.
- Bu yolla yakalanamayan bir donmada kredi ve çekirdek bütçesi devreye girer
  (§4.3).

---

## 6. Pencereler ve komut yetkileri (üst belge §11.9)

İki pencere vardır; ikisini de yalnızca Rust açar. Komut adları öneridir; plan
aşamasında kesinleşir.

| Pencere | Gösterdiği | Komutları |
|---|---|---|
| `ana` | Oturum listesi, konuşma, onay kartları, durum çubuğu | `uygulama_durumu`, `oturumlari_listele`, `calisma_alani_sec`, `oturum_baslat`, `oturum_ac`, `oturumu_kapat`, `oturumu_dinle`, `paketi_isledim`, `gecmis_sayfasi`, `mesaj_gonder`, `turu_iptal_et`, `arac_onayi`, `model_degistir`, `izin_kipi_degistir`, `baglanti_ac`, `anahtar_penceresini_ac` |
| `sartlar` | Anahtar formu, EVREN şart metni, kilitli görünüm | `anahtari_kaydet`, `sart_metnini_al`, `sartlari_kabul_et`, `sartlari_reddet`, `baglanti_ac` |

- **Genel bir "komut gönder" yoktur.** Her işlemin kendi Tauri komutu vardır.
  Şart kabulüne giden tek yol şart penceresindeki "Kabul ediyorum" düğmesidir.
- Komutlar uygulama manifest'inde tanımlanır (`tauri-build`'in `AppManifest`'i).
  Yetkiler capability dosyalarıyla pencere etiketine göre dağıtılır; bir
  pencereye verilmemiş komut reddedilir.
- **Anahtar ana pencereye hiç girmez.** Şart penceresinde yazılır, bir kez
  Rust'a gider; hiçbir komut onu geri döndürmez.
- **Ana pencere yeni pencere açamaz.** Açabilseydi `sartlar` etiketli sahte bir
  pencereyle şart yetkilerini alabilirdi. Tauri'nin kendi izinlerinden yalnızca
  gerekenler verilir; `core:default` verilmez.
- **Gezinme kapalıdır.** İki pencere de uygulamanın kendi sayfaları dışına
  gidemez; gezinme işleyicisi her başka adresi reddeder.
- **Bağlantılar:** tıklanınca Rust adresin tamamını yerel bir iletişim kutusunda
  gösterir; onay gelirse sistem tarayıcısında açar. Yalnızca `http` ve `https`
  açılır. Webview'a sızmış bir betik de veriyi bir adrese gömüp kullanıcı
  görmeden dışarı gönderemez.
- **İzin kipini yükseltmek** yerel iletişim kutusunda onaylanır. Sıra İnceleme
  < Sıkı < Proje < Serbest'tir; sağa her geçiş yükseltmedir. Kipi düşürmek onay
  istemez. Yalıtımsız kabuk onayı kabukla birlikte aşama 2'de gelir.
- **CSP** (`tauri.conf.json`): `default-src 'none'; script-src 'self';
  style-src 'self'; font-src 'self'; img-src 'self'; connect-src ipc:
  http://ipc.localhost; base-uri 'none'; form-action 'none'`. Geliştirme
  sunucusu için ayrı bir `devCsp` yalnızca geliştirmede geçerlidir. `asset:`
  protokolü kapalıdır; arayüz dosya sistemine doğrudan erişmez.

---

## 7. İlk açılış, şart akışı ve kimlik deposu (üst belge §11.8)

### 7.1 Açılış sırası

```
Kalfa açılır ─► kimlik deposunda anahtar var mı?
  ├─ yok ─► şart penceresi: anahtar formu
  └─ var ─► GET /v1/terms/status
              ├─ kabul edilmiş, sürüm güncel ─► ana pencere
              ├─ kabul gerekli ─► şart penceresi: metin ─┬─ Kabul ediyorum ─► ana pencere
              │                                          └─ Kabul etmiyorum ─► kilitli görünüm
              ├─ 401 ─► şart penceresi: anahtar formu ("kayıtlı anahtar artık geçerli değil")
              └─ EVREN'e ulaşılamıyor ─► ana pencere, "bağlantı yok" uyarısıyla
```

Bağlantı yokken açılan ana pencerede geçmiş oturumlar okunabilir. Şart onayı
gerekiyorsa ilk istekte 403 olarak yakalanır ve §7.3'teki yol çalışır.

### 7.2 Anahtar

- Girilen anahtar önce şart durumu sorgusuyla denenir. 401 dönerse ya da
  EVREN'e ulaşılamazsa saklanmaz; form nedenini söyler.
- Geçerliyse işletim sisteminin kimlik deposuna yazılır: Windows'ta Credential
  Manager, Linux'ta Secret Service (`keyring` crate'i). Secret Service yoksa
  form bunu söyler ve anahtar yalnızca Kalfa açık kaldıkça bellekte tutulur.
- Anahtar yalnızca sağlayıcı katmanında durur; arayüz durumuna, günlüklere ve
  kayda girmez.
- Anahtar değişirse açık oturumlar da yenisini kullanır: sağlayıcı, anahtarı
  değiştirilebilen paylaşılan bir istemci üzerinden kullanır.
- Ana penceredeki "Anahtarı değiştir" şart penceresini anahtar formuyla açar.

### 7.3 Şart kapısı

Bugün her oturum 403 aldığında şart metnini de kabul isteğini de kendisi
gönderiyor. İki oturum aynı anda beklerken tek tık iki `POST /v1/terms/accept`
isteğine dönüşür. EVREN'in aynı sürümün ikinci kez kabulüne ne yanıt verdiği
bilinmiyor; kullanıcı adına kabul yasak olduğu için canlı denenmez. Bu yüzden:

- Uygulama genelinde tek bir şart kapısı vardır (`kalfa-core`, Tauri'siz). İlk
  açılış da oturumlar da aynı kapıyı kullanır.
- 403 alan oturum kapıya kaydolur, `SartOnayiGerekiyor` olayını yayar ve iptal
  edilebilir biçimde bekler. Oturum kabul isteğini kendisi göndermez.
- Kapı metni her sürüm için bir kez alır ve şart penceresine hangi sürümü
  gösterdiğini saklar.
- "Kabul ediyorum", `sartlari_kabul_et(surum)` komutunu çağırır. Sürüm
  gösterilenle aynı değilse kabul gönderilmez, güncel metin gösterilir. Aynıysa
  kabul isteği bir kez gönderilir; başarılı olursa bekleyen bütün oturumlar
  uyanır ve reddedilen isteklerini yeniden gönderir. Kabul edilen sürüm her
  bekleyen oturumun kaydına yazılır.
- Kabul isteği, sürüm bu arada değiştiği için reddedilirse kapı yeni metni alır.
  Pencere yeni metni "şart metni bu sırada güncellendi" notuyla gösterir.
- "Kabul etmiyorum" seçilirse bekleyen oturumların turları iptal edilir;
  oturumlar kaybolmaz.
- Şart penceresi açıkken başka bir oturum da 403 alırsa aynı pencereye bağlanır;
  ikinci pencere açılmaz.

### 7.4 Şart penceresi

- Başlıkta sürüm numarası ve değişikliğin esaslı olup olmadığı (`is_material`)
  yazar.
- Metin özetlenmeden, güvenli Markdown ile gösterilir (§9.1). EVREN metnindeki
  shields.io rozetleri yüklenmez, yerlerinde alt metinleri görünür. İlk ekran
  üçüncü taraf bir sunucuya istek atmaz.
- Onay kutusu yoktur; yalnızca "Kabul ediyorum" ve "Kabul etmiyorum" düğmeleri
  vardır. Tasarım sistemindeki `OnboardingScreen` Kalfa'nın kendi maddelerini ve
  bir onay kutusunu gösteriyor; yalnızca görsel referanstır (`design/README.md`).
- **Kilitli görünüm:** anahtar saklı kalır. Pencere, EVREN'in şartlar kabul
  edilmeden hiçbir ucu açmadığını söyler ve metni yeniden gösteren bir düğme
  sunar. Sonraki açılışta şart penceresi yeniden gelir.

---

## 8. Ekranlar

### 8.1 Yerleşim ve bileşenler

Yerleşim tasarım sistemindeki üç sütundur: menü (213 px), oturum listesi
(295 px), konuşma. En altta durum çubuğu bulunur.

1c'de yazılacak bileşenler, tasarım sistemindeki adlarıyla. Her birinin `.d.ts`
dosyası arayüz tanımı, `.prompt.md` dosyası kullanım notu olarak kullanılır:

| Grup | Bileşenler |
|---|---|
| Temel | Badge, Button, Card, Icon, IconButton, IconTile |
| Form | Checkbox, Input, Select, Textarea |
| Sohbet | Composer, Message, ThoughtPanel, ToolCallCard |
| Veri | CodeBlock, DiffView, MetaList, TokenMeter |
| Gezinme | NavItem, NavRail, PanelHeader, SessionRow, StatusBar |

Sonraki aşamalara kalanlar: TurnSummary, FileTree, LogTable, Pagination,
SourceTag.

- **Menü** 1c'de yalnızca iki öğe içerir: Oturumlar ve "Anahtarı değiştir".
- **Composer:** metin alanı, gönder düğmesi, süren turda iptal düğmesi, model
  seçici (Hızlı, Dengeli, Derin ve `auto`; üst belge §6.4) ve izin kipi seçici.
- **Düşünce paneli** varsayılan olarak kapalıdır (üst belge §6.2).
- **Oturum satırı** durumu gösterir: çalışıyor, onay bekliyor, şart bekliyor,
  yarım kaldı, hata.

### 8.2 Onay isteği

- Konuşmanın içinde küçük bir kart olarak görünür. Karttaki "İncele" düğmesi
  tasarım sistemindeki onay sayfasını (`ApprovalSheet`) açar.
- Sayfada dosya listesi, dosya başına diff, "ne" (`aciklama`) ve "nerede"
  (`hedefler`), uyarılar, ret gerekçesi alanı ve üç karar düğmesi bulunur:
  Tümünü onayla, Seçilenleri onayla, Reddet. Seçilmeyen çağrılar gerekçeyle
  reddedilir.
- Sayfa karar verilmeden kapatılabilir; istek kartta beklemeye devam eder. Bir
  oturum onay beklerken diğer oturumlara geçilebilir.
- Veri çıkışı, yalıtım, geri alınabilirlik ve kapsam alanları aşama 3'teki tam
  kartla gelir. Tasarımdaki "Bu değişiklikler geri alınabilir" cümlesi geri alma
  gelene kadar yazılmaz; yazılsaydı yanlış olurdu.

### 8.3 Durum çubuğu

- Gösterilenler: son gözlenen bakiye ve gözlem zamanı, oturumun token kullanımı
  (düşünce dökümüyle), yönlendirilen model, izin kipi.
- Bağlam doluluğu gösterilmez: EVREN'in model kataloğu bağlam penceresini
  vermiyor.
- Tahmini maliyet yalnızca katalog fiyatı sıfırdan büyükse görünür (üst belge
  §11.5). Şu an bütün fiyatlar sıfır.
- Kabuk yalıtım durumu kabukla birlikte aşama 2'de gelir.

### 8.4 Arayüz durumu

- Her oturumun durumu saf bir TypeScript işleviyle güncellenir:
  `uygula(durum, girdi) → durum`. Girdi bir kanal mesajı ya da bir geçmiş
  sayfasıdır. İşlev DOM'a dokunmaz ve Vitest ile test edilir.
- Svelte 5 runes yalnızca bu durumu çizer.
- İşlev paket numarasında boşluk görürse bunu hata olarak bildirir; ekran
  oturumu kayıttan yeniden açar (§4.4).

---

## 9. Çizim ve dil

### 9.1 Güvenli Markdown

- Metin önce sözdizimi ağacına çevrilir (micromark ve GFM eklentisiyle,
  `mdast`). Ağaç Svelte bileşenleriyle çizilir; `{@html}` hiç kullanılmaz. Ham
  HTML yapı gereği çalışamaz, düz metin olarak görünür.
- Bağlantılar düğme olarak çizilir ve `baglanti_ac` yolundan geçer (§6).
  `javascript:`, `file:` ve diğer protokoller bağlantıya dönüşmez, düz metin
  kalır.
- Görsellerin yerine alt metinleri gösterilir; hiçbir görsel yüklenmez.
- Aynı çizici sohbette, düşünce panelinde, araç çıktısında ve şart metninde
  kullanılır.

### 9.2 Akış görünümü (üst belge §11.3)

- Akan mesaj ekran karesi başına en çok bir kez yeniden çözülür.
- CommonMark'ta kapanmamış bir kod bloğu metnin sonuna kadar sürer. Yarım
  bloklara tolerans böylece kendiliğinden sağlanır; yarım liste ve tablolar da
  metin geldikçe çizilir.
- Tamamlanan mesaj bir kez çizilir ve dondurulur.
- Kod blokları akış sırasında renklendirilmez; bu, üst belgedeki "hafif"
  renklendirmenin en yalın hâlidir. Mesaj tamamlanınca Shiki ile renklendirilir.
  Shiki yerel olarak paketlenir, sınırlı bir dil kümesiyle ve WASM yerine
  JavaScript düzenli ifade motoruyla çalışır. WASM motoru CSP'de
  `wasm-unsafe-eval` izni isterdi.

### 9.3 Uzun oturumlar (üst belge §11.2)

- Oturum açılınca kayıttan son 20 tur yüklenir. Yukarı kaydırdıkça eskileri
  sayfa sayfa eklenir (`gecmis_sayfasi`).
- Ekran dışındaki mesajlar `content-visibility: auto` ile çizilmez.
- Tam sanallaştırma, yani ekrandan uzaktaki turların DOM'dan çıkarılması,
  §11.4'teki uzun oturum ölçümü gerektirirse eklenir. Üst belgenin istediği
  sabit maliyetin 1c'deki karşılığı budur.

### 9.4 Dil ve yazım (üst belge §11.6)

- Arayüz metinleri tek bir Türkçe katalogda (`ui/src/dil/`) toplanır. İngilizce
  v1'den önce eklenecek; 1c'de yok.
- Tasarım sisteminin yazım kuralları geçerlidir: arayüzde "siz"; ünlem ve emoji
  yok; biten iş "Tamamlandı" ile özetlenir.
- Çekirdekte kullanıcıya giden birkaç metin "sen" diliyle yazılmış:
  "bitmesini bekle ya da iptal et", "yeni bir oturum aç" ve `kalfa-evren`'de
  "anahtarını kontrol edip yeniden gir". Bunlar "siz"e çevrilir. Modele giden
  araç hataları "sen" kalır; sistem mesajı modele "sen" diye hitap ediyor.
- Hata metinleri ne olduğunu ve ne yapılacağını söyler (§10).

### 9.5 Klavye ve ekran okuyucu

- Enter mesajı gönderir, Shift+Enter yeni satır açar.
- Onay sayfası dahil her işlem klavyeyle yapılabilir. Onay sayfası açıkken odak
  sayfanın içinde kalır.
- Onay istekleri ve tur sonları ekran okuyucuya duyurulur (`aria-live`); akan
  token'lar duyurulmaz.

---

## 10. Hata durumları

| Durum | Kullanıcının gördüğü | Kullanıcının yapabileceği |
|---|---|---|
| Girilen anahtar geçersiz (401) | "Anahtar geçersiz ya da eksik. EVREN portalından anahtarınızı kontrol edip yeniden girin." Anahtar saklanmaz | Yeni anahtar girmek |
| Kayıtlı anahtar oturum ortasında geçersizleşti | Tur hatayla biter; kartta aynı açıklama ve "Anahtarı değiştir" düğmesi | Anahtarı değiştirip mesajı yeniden göndermek |
| EVREN'e ulaşılamıyor | Ana pencerede "bağlantı yok" şeridi; gönderilen mesaj hatayla biter | Bağlantı gelince yeniden göndermek |
| Kimlik deposu yok (Linux) | "Anahtar bu oturum boyunca bellekte tutulacak. Kalfa kapanınca yeniden girmeniz gerekecek." | Secret Service kurmak |
| Aktarım durdu | "Arayüz uzun süre yanıt vermediği için oturum durduruldu. Oturumu yeniden açabilirsiniz." | Oturumu kayıttan açmak |
| Kayıt yazılamadı | Çekirdeğin mevcut davranışı: oturum durur. "Oturum kaydı yazılamadı. Yeni bir oturum açın." | Yeni oturum açmak |
| Kayıt başka bir süreçte açık | "Bu oturum başka bir Kalfa sürecinde açık." | O süreci kapatmak |
| Kayıt bozuk | Listede "okunamadı"; ayrıntıda bozuk satırın numarası | — |
| Çalışma alanı bulunamadı ya da kayıtta yolu yok | Geçmiş görünür, mesaj alanı kapalıdır: "Çalışma alanı bulunamadı: `<yol>`" | Klasörü yerine koymak |
| Şartlar reddedildi | Kilitli görünüm (§7.4) | Metni yeniden görüp karar vermek |
| Şart sürümü okurken değişti | Yeni metin ve "şart metni bu sırada güncellendi" notu | Yeni metni okuyup karar vermek |
| İzin kipi yükseltmesi iletişim kutusunda reddedildi | Kip değişmez | — |
| Sonucu bilinmeyen çağrı (kurtarmada) | Araç kartında "sonucu bilinmiyor"; çağrı yeniden çalıştırılmaz | Modelden yeniden yapmasını istemek |
| Paket numarasında boşluk | Oturum kayıttan yeniden açılır | — |

---

## 11. Test stratejisi ve kabul ölçümü

Uygulama TDD ile yürür (üst belge §13). Test edilmeyen güvenlik sınırı yoktur:
pencere yetkileri, CSP, Markdown ve anahtarın ana pencereye girmemesi bu yüzden
pazarlık konusu değildir.

### 11.1 Çekirdek (Tauri'siz, `cargo test`)

- **Paketleyici** (`tokio::time::pause` ile): 16 ms pencere; bitişik metin
  parçalarının birleşmesi, başka türden olayın birleştirmeyi kesmesi; kritik
  olayların hemen gitmesi, hiç birleştirilmemesi ve düşürülmemesi; paket bayt
  sınırı; kredi (paket ve bayt) dolunca çekirdek kuyruğundan okumanın durması;
  krediden büyük tek olay; paket numarasının boşluksuz artması; duraklayan
  tüketicide çekirdek bütçesinin dolması, turun iptali ve `AktarimDurdu`.
- **Şart kapısı:** iki bekleyen oturum → tek kabul isteği, ikisi de uyanır;
  gösterilenden farklı sürümle kabul reddi; okurken sürümün değişmesi; ret →
  bekleyen turlar iptal; ilk açılış dalları (kabul edilmiş, gerekli, 401,
  ulaşılamıyor); kabul edilen sürümün her oturumun kaydına yazılması.
- **Kilit yöneticisi:** aynı kök farklı yazılışla, junction ile (Windows) ve
  sembolik bağlantıyla aynı yöneticiyi verir; farklı kökler farklı yönetici
  alır; son oturumdan sonra kök kaydı silinir; yol tablosu temizlenir.
- **Kayıt kilidi:** aynı süreçte ikinci yazıcı reddedilir; başka süreçte ikinci
  yazıcı reddedilir (test ikilisi alt süreç olarak başlatılır); kilit
  tutulurken kayıt okunabilir; kayıttan devamda kilit okumadan önce alınır.
- **Geçmiş ve liste:** kayıttan görünüm öğeleri; sonucu bilinmeyen çağrı;
  `sira` ile canlı olaylarla birleşme; tur bazlı sayfalama; başlık ve son durum
  çıkarımı; çalışma alanı yolu olmayan kayıt.
- **İstek bilgisi:** yönlendirilen modelin olay olarak yayılması; token
  kullanımıyla birlikte kayda yazılması.

### 11.2 Uygulama (`cargo test -p kalfa`)

- Tauri'nin test çalışma zamanıyla (`tauri::test`) komut yetkileri: ana
  pencereden `sartlari_kabul_et` ve pencere açma reddedilir; şart penceresinden
  oturum komutları reddedilir.
- Test çalışma zamanı uygulama komutlarının yetkilerini uygulamıyorsa
  capability dosyaları statik olarak doğrulanır; gerçek pencere testi (§11.4)
  bu boşluğu kapatır.
- `baglanti_ac` yalnızca `http` ve `https` açar; iletişim kutusu reddedilince
  hiçbir şey açılmaz (sahte iletişim kutusuyla).
- `izin_kipi_degistir` yükseltmede iletişim kutusu sorar, düşürmede sormaz.
- Kimlik deposu: sahte depoyla kaydetme ve okuma; depo yokken bellekte tutma.

### 11.3 Arayüz (`ui/`, Vitest ve `svelte-check`)

- Durum işlevi: kanal mesajı → durum; kritik olaylar; onay isteği; tur sonu;
  paket numarası boşluğu; geçmiş sayfasının canlı olaylarla birleşmesi.
- Markdown: ham HTML metin olarak görünür; `javascript:` ve `file:` bağlantıya
  dönüşmez; uzak görsel alt metne döner; kapanmamış kod bloğu; GFM tablosu.
- "İşledim" bildirimi çizimi beklemez.
- Kodda kullanılan her metin anahtarı dil kataloğunda vardır.

### 11.4 Uçtan uca testler ve üst belge §11.1 kabul ölçümü

Gerçek uygulama, sahte sağlayıcılı bir test derlemesiyle ve `tauri-driver`
(WebDriver) üzerinden Windows 11 ve Ubuntu 24.04'te koşar. Sahte sağlayıcı
yalnızca test özelliği açıkken derlenir; sürüm derlemesine girmez.

**Akış testleri**

- İlk açılış: anahtar → şart → kabul → ana pencere; ret → kilitli görünüm;
  401 → form hatası.
- Mesaj → akış → onay sayfası → yazma; ret gerekçesinin modele gitmesi; iptal.
- Oturumu kapatıp kayıttan açma; geçmişin doğru sırayla gelmesi.
- Ana pencerenin JavaScript'inden `sartlari_kabul_et` çağrısı gerçek webview'da
  reddedilir.
- Anahtar girildikten sonra ana pencerenin DOM'unda ve durumunda anahtar
  bulunmaz.
- Satır içi betik ve uzak görsel içeren model çıktısı: betik çalışmaz, ağ
  isteği ve IPC çağrısı oluşmaz.

**Kabul kapısı**

1. **Gecikme:** sahte sağlayıcı üç desende parça üretir: 20 ms'de bir düzenli;
   5–10 parçalık yığınlar; düşünce ve metin paralel. Her parça üretildiği anın
   duvar saati zamanını taşır; arayüz parçanın çizildiği anı
   (`requestAnimationFrame` sonrası) kaydeder. İki saat aynı makinededir. Hedef:
   p95 < 50 ms. Pencere, paket sınırı ve kredi değerleri bu ölçümle ayarlanır;
   sonuçlar `docs/GELISTIRME_DURUMU.md`'ye yazılır.
2. **Duraklayan arayüz:** bir test kancası arayüzün paket işlemesini durdurur,
   sahte sağlayıcı yüksek hızda akar. Beklenen: yoldaki veri krediyi, çekirdek
   kuyruğu bütçeyi hiç aşmaz (test derlemesindeki sayaçlarla ölçülür); bütçe
   dolunca tur iptal edilir; arayüz yeniden işlemeye başlayınca `AktarimDurdu`
   bildirimi görünür. Sürecin belleği (RSS) ek bir akıl sağlığı denetimi olarak
   izlenir.
3. **Arayüz kopması:** tur sürerken sayfa yeniden yüklenir. Tur iptal edilir,
   kayıtta `TurBitti { Iptal }` bulunur, kayıt kilidi bırakılır ve oturum
   kayıttan açılabilir. Pencere kapatmada da aynısı olur.
4. **Uzun oturum:** 500 turluk bir kayıt açılır; açılış süresi ve bellek,
   20 turluk bir kayıtla aynı mertebede kalır. Kalmıyorsa tam sanallaştırma
   eklenir (§9.3).

### 11.5 Elle kontrol

- NVDA (Windows) ve Orca (Linux) ile onay isteği ve tur sonu duyurusu.
- Yalnızca klavyeyle baştan sona akış.
- Tasarım sistemiyle görsel karşılaştırma: tokenlar, üç sütun, onay sayfası,
  şart penceresi.
- Linux'ta Secret Service olmadan açılış.

### 11.6 Doğrulama komutları

```
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo tree -p kalfa-core -e normal | grep -E "kalfa-tools|tauri"   # çıktı boş olmalı
cd ui && npm ci && npm run check && npm test && npm run build
```

Uçtan uca testlerin komutu plan aşamasında belirlenir. Canlı EVREN testleri
(`#[ignore]`) ajan tarafından koşturulmaz.

---

## 12. Üst belgede değişenler

Bu tasarımla birlikte üst belgede güncellenen yerler (r3.3):

- **§4.1:** `sira` kayıt öğeleriyle ortak sayaçtan artar; olay akışındaki boşluk
  olağandır. Kopukluğu aktarım katmanının paket numarası yakalar (§4.4).
- **§6.6:** tek yazıcı, kaydın yanındaki kilit dosyasında tutulan işletim
  sistemi kilidiyle sağlanır (§5.2).
- **§11.1:** köprünün tasarımı ve kabul ölçümü bu belgenin §4 ve §11.4
  bölümlerindedir.
- **§11.8:** şart kabulü uygulama genelindeki tek şart kapısından bir kez
  gönderilir; oturumlar kabul isteğini kendileri göndermez (§7.3).

---

## 13. Plan aşamasında doğrulanacaklar

- Tauri'nin `generate_context!` makrosu arayüz dosyalarını derleme sırasında
  okuyabiliyor. `cargo test --workspace` öncesinde `npm run build` gerekip
  gerekmediği doğrulanır.
- WSL'deki Ubuntu 24.04'e WebKitGTK geliştirme paketlerinin kurulumu; uçtan uca
  testler için görüntü sunucusu (WSLg ya da Xvfb).
- `keyring` 3'te platform depoları özellik bayraklarıyla açılır. Bayraklar
  açılmazsa crate sessizce kalıcı olmayan bir sahte depo kullanır. Windows ve
  Secret Service bayrakları açıkça verilir ve testle doğrulanır.
- Tauri'nin test çalışma zamanının uygulama komutlarının yetkilerini uygulayıp
  uygulamadığı (§11.2).
- Küçültülmüş pencerede WebView2 ve WebKitGTK'nin betik yürütmesi. Gerekirse
  pencere yapılandırmasında arka plan kısıtlaması kapatılır (§4.3).
- `tauri-driver` kurulumu: Windows'ta WebView2 sürümüyle uyumlu
  `msedgedriver`, Linux'ta `WebKitWebDriver`.
- `same-file`'ın Windows'ta dizinler için verdiği kimlik; ReFS'deki 128 bitlik
  dosya kimlikleri.
- Tauri'nin veri dizini API'siyle üst belge §6.6'daki yolların
  (`%APPDATA%\Kalfa`, `$XDG_DATA_HOME/kalfa`) eşleştirilmesi.
- Shiki'nin JavaScript motorunun seçilen dillerdeki doğruluğu.

---

## 14. Uygulama planına geçiş

1c tek bir plan için büyük. 1a ve 1b'deki gibi bölünmesi öneriliyor; her parça
kendi başına test edilebilir:

1. **1c-1 Çekirdek eklemeleri** (Tauri'siz): çalışma ağacındaki olay kuyruğu
   değişikliğinin gözden geçirilip kullanıcı onayıyla commit'lenmesi,
   paketleyici, şart kapısı, kilit yöneticisi, kayıt kilidi, `OturumAcildi` ve
   istek bilgisi, geçmiş ve oturum listesi, metin tonu.
2. **1c-2 Tauri kabuğu:** `kalfa` crate'i, pencereler, komutlar ve yetkiler,
   köprü, kimlik deposu, tek süreç, yerel iletişim kutuları.
3. **1c-3 Arayüz:** `ui/` iskeleti, tokenlar ve fontlar, bileşenler, durum
   işlevi, Markdown çizicisi, iki pencerenin ekranları.
4. **1c-4 Kabul:** uçtan uca testler ve üst belge §11.1 ölçümü.
