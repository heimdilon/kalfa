# Kalfa güvenlik analizi: harness sınırları ve prompt injection

**Tarih:** 2026-09-27
**Kapsam:** `cd96c46` (2d-2 birleştikten sonra) üzerindeki üretim kodu:
`kalfa-core`, `kalfa-tools`, `kalfa-sandbox` (Linux ayrıntılı, Windows kısmen),
`kalfa-uygulama`, `kalfa` (Tauri), `kalfa-yalitim` ve `ui/`. Kod değiştirilmedi.
Bulgu 1 bir git deposunda ve geçici bir çekirdek testiyle doğrulandı; test
commit'lenmedi.

## Kısa sonuç

- **Prompt injection'a açık mı?** Evet, her LLM harness'ı gibi. Modele giren
  dosya, arama ve kabuk çıktısı talimat gibi okunabilir. Arada yalnızca sistem
  istemindeki bir cümle ve metin olarak eklenen `[kaynak: …]` etiketi var.
  Güvenlik denetçisi (üst belge §9) henüz yok. Kalfa'nın asıl savunması izin
  katmanı ve yalıtım. Bu katmanlar genel olarak iyi tasarlanmış ve testli.
- **Harness güvenli mi?** Varsayılan Sıkı kipte büyük ölçüde evet. **Proje ve
  Serbest kiplerde doğrulanmış bir açık var:** dosya araçları `.git/commondir`
  yazabiliyor. Bu, onay sorulmadan, yalıtım dışında, kullanıcının bütün
  yetkileriyle kod çalıştırmaya götürüyor (Bulgu 1). Ayrıca tasarımdan gelen
  birkaç açık uç var: onaysız yazılan dosyaları başka araçların kendiliğinden
  çalıştırması, dar gizli dosya listesi, onay metinlerinde görünmez Unicode.

## Güçlü yanlar

İncelemede sağlam bulunan ve korunması gerekenler:

- **Mühürlü onay** (`kalfa-core/src/muhur.rs`, `oturum/arac_asamasi.rs`): Mühür
  şunları kapsar: çağrı kimliği, argümanlar, hedef dosyanın önceki özeti,
  bağlantı sayısı ve kabuk tarifi (komut, dizin, ağ, yalıtım profili). Hazırlık
  kilit altında yeniden yapılır; mühür, karar ve politika sürümü yeniden
  denetlenir. Onaydan sonraki TOCTOU değişiklikleri reddediliyor.
- **Dosya araçları:** `cap-std` ile her bileşen no-follow açılıyor. Sembolik
  bağlantılar reddediliyor. Mutlak yol, `..`, ADS ve aygıt adları metin
  düzeyinde de reddediliyor. Hard link'e yazmak onay istiyor.
- **Pencere sınırı:**
  - Yalıtımsız komut, çalışma alanı hazırlığı, kip yükseltme, yalıtımsız
    anahtarı ve bağlantı açma yalnızca yerel iletişim kutusuyla onaylanıyor.
  - Webview `YerelOnay` gönderemiyor; çekirdek, `AracOnayi` ile gelen yerel
    onaylı çağrıyı reddediyor. Webview ele geçirilse bile bu işler
    onaylanamaz.
- **Arayüz:** Sıkı CSP ve `freezePrototype` var. Markdown AST'den çiziliyor;
  ham HTML metin olarak kalıyor ve görsel yüklenmiyor. Bağlantı yalnızca
  http/https ile ve yerel onayla açılıyor. Pencerelere Tauri'nin çekirdek
  izinleri verilmiyor.
- **Linux yalıtımı:**
  - bwrap boş kökten başlıyor; `--unshare-net`, `--disable-userns` ve
    `--clearenv` ile çalışıyor.
  - seccomp `socket` ailelerini izin listesiyle sınırlıyor. `AF_UNIX` kapalı,
    bu yüzden docker.sock, ssh-agent ve D-Bus'a erişilemiyor.
  - `.git` salt okunur. Gizli dosyaların üstüne `/dev/null` bağlanıyor.
  - Ev, geçici ve önbellek dizinleri çalışma alanına özel.
- **Sırlar:** Ortam değişkenleri izin listesiyle geçiyor; adında `KEY`,
  `TOKEN` gibi parçalar olanlar ayrıca engelleniyor. EVREN anahtarı işletim
  sisteminin kimlik deposunda tutuluyor.
- **Döngü sınırları:** en çok 25 model turu, 50 araç çağrısı, 64 KiB çıktı;
  aynı hata üst üste 3 kez olursa döngü duruyor.

## Bulgular

| # | Önem | Doğrulama | Özet | Durum (2026-09-28) |
|---|---|---|---|---|
| 1 | **Yüksek** | Doğrulandı | Dosya araçları `.git/commondir` ve `.git/modules/*` yazabiliyor; git'in kod çalıştırma koruması aşılıyor | Giderildi: `1e22c40` (dosya araçları), `cf7c5ee` (yalıtım, yeni `.git` karantinası) |
| 2 | Yüksek (tasarım) | Doğrulandı (sınıflama) | Onaysız yazılan dosyaları IDE, git ve CI yalıtım dışında çalıştırıyor | Giderildi: `551b44d` (yapılandırma sınıfı) |
| 3 | Orta | Kod okuması | Kabuk onayı komutu gösteriyor, komutun çalıştıracağı onaysız değişmiş kodu göstermiyor | Giderildi: `37b0c77` |
| 4 | Orta | Doğrulandı (sınıflama) | Gizli dosya listesi dar: yaygın sır dosyaları onaysız okunup EVREN'e gidiyor, yalıtımda da açık | Giderildi: `551b44d`; kullanıcının ek kalıpları için ayar arayüzü açık iş |
| 5 | Orta | Kod okuması | Prompt injection savunması yalnızca istem düzeyinde; kaynak etiketi taklit edilebilir | Azaltıldı: `3dc26f3` (rastgele sınırlı sarmal); denetçi açık iş |
| 6 | Orta | Doğrulanmadı (Windows) | Yönetici yardımcısı `ProgramData\Kalfa` altında reparse noktasını izleyerek DACL ve dosya yazabilir | Giderildi: `6b4fe4f`; yönetici olarak kurulum kabulü kullanıcıda |
| 7 | Düşük–orta | Kod okuması | Onay kartında ve yerel kutuda görünmez ve yön değiştiren Unicode etkisiz hâle getirilmiyor | Giderildi: `8b58f20` |
| 8 | Düşük | Tasarım | Ağlı komutun çıkışı sınırsız; bir onay bütün çalışma alanını dışarı taşıyabilir | Kapsam dışı: aşama 3'te çıkış vekili tasarımı |

### 1. `.git/commondir` ile yalıtım dışında kod çalıştırma (Yüksek, doğrulandı)

**Sorun:**
- `git_koruma::korumali_yollar` (`kalfa-core/src/git_koruma.rs:176`) dosya
  araçları için yalnızca `.git/hooks`, `.git/config` ve `core.hooksPath`'i
  korumalı sayıyor.
- `.git`'in geri kalanı Normal sınıftadır. Proje ve Serbest kipte buralara
  **onaysız**, Sıkı kipte **uyarısız** yazılabiliyor.
- Kabuk tarafında `.git`'in tamamı salt okunur (aşama 2 tasarımı §7.2). Dosya
  araçları ise bu korumayı taşımıyor.

**Saldırı:** Okunan bir dosyadaki talimat modeli şu yazmalara yönlendirir:

1. `zz/config` dosyasına `[core] fsmonitor = "<komut>"` yazar.
2. `zz/objects/.keep` ve `zz/refs/.keep` dosyalarını oluşturur.
3. `.git/commondir` dosyasına `../zz` yazar.

Git, `commondir`'deki yapılandırmayı ve kancaları kullanır. Kullanıcı ya da
IDE'nin git eklentisi (VS Code kendiliğinden çalıştırır) `git status`
çalıştırdığında komut, kullanıcının bütün yetkileriyle ve yalıtım dışında
çalışır.

**Kanıt:**
- Git 2.43'te yalnızca metin dosyası yazılarak `git status` sırasında komut
  çalıştı.
- Geçici çekirdek testi `Kip::Proje` için şu kararları verdi:

  ```
  .git/commondir         sinif=Normal   proje_yazma=ONAYSIZ
  .git/modules/x/config  sinif=Normal   proje_yazma=ONAYSIZ
  .git/config            sinif=Korumali proje_yazma=onayli
  ```

**Öneri:**
- Dosya araçları için `.git` dizininin (ya da `.git` dosyasının) tamamı
  korumalı sayılsın. Buna çözülen gerçek git dizini ile ortak dizin de girer.
  Bu, kabuk profilindeki kuralla aynı olur.
- Sınır, her dosya aracı için bir testle gelsin: `commondir`,
  `modules/*/config`, `modules/*/hooks`, `worktrees/*`.

### 2. Onaysız yazılan dosyaları yalıtım dışındaki araçlar çalıştırıyor (Yüksek, tasarımdan)

**Sorun:** Proje ve Serbest kipte çalışma alanına onaysız yazılabiliyor.
Aşağıdaki dosyaları kullanıcının araçları, yalıtım dışında ve çoğu zaman
kendiliğinden çalıştırır:

- `build.rs` ve proc-macro'lar: rust-analyzer proje açılınca derler.
- `.cargo/config.toml`: `rustc-wrapper`, `runner`, `[alias]`.
- `.vscode/tasks.json`, `.vscode/settings.json`, `.devcontainer/`.
- `.envrc`, `package.json` betikleri, `Makefile`.
- `.github/workflows/`: sırlarla CI'da çalışır.

Kalfa'nın yalıtımı yalnızca Kalfa'nın kendi komutlarını sarıyor.
`ILK_KABUK_UYARISI` bu riski Kalfa'nın ilk komutu için anlatıyor. Kullanıcının
kendi araçları için hiçbir uyarı yok. Serbest kipte yalıtımlı ve ağsız komutlar
da bu dosyaları onaysız değiştirebiliyor.

**Öneri:**
- "Kendiliğinden çalışan yapılandırma" adıyla ikinci bir korumalı sınıf
  eklensin. Önce en az şu yollar girsin: `.cargo/config*`, `.vscode/`,
  `.idea/`, `.devcontainer/`, `.envrc`, `.github/workflows/`,
  `.gitlab-ci.yml`.
- Bu sınıfa yazmak her kipte onay istesin. Aynı yollar kabuk profilinde salt
  okunur olsun.
- `build.rs` ve `package.json` meşru ve sık düzenlendiği için onları korumak
  pratik değil. Onlar Bulgu 3'teki öneriyle ele alınsın.

### 3. Kabuk onayı, çalıştırılacak kodu göstermiyor (Orta)

**Sorun:** Mühür komut metnini kapsıyor, komutun çalıştıracağı dosyaları
kapsamıyor. Proje kipinde aynı turda şu olabilir:

1. `build.rs` onaysız yazılır.
2. Masum görünen `cargo test` onaya gelir.

Kullanıcı `cargo test`'i onaylar ama aslında modelin yazdığı kodu onaylamış
olur. Komut `ag: true` ile gelirse bu kod ağa da çıkar ve çalışma alanını
dışarı taşıyabilir. "Tümünü onayla" düğmesi bu yorgunluğu artırıyor.

**Öneri:**
- Kabuk onay kartında "bu oturumda onaysız değişen dosyalar" listelensin.
- Bu listede derleme ya da betik dosyası (`build.rs`, `package.json`,
  `Makefile` vb.) varsa ayrı bir uyarı gösterilsin.
- Ağlı komutlar "Tümünü onayla"nın dışında tutulsun.

### 4. Gizli dosya listesi dar (Orta)

**Sorun:** `VARSAYILAN_GIZLI_KALIPLAR` (`kalfa-core/src/izin.rs:26`) listesi
kısa. Şunlar Normal sınıfta kalıyor:

- `prod.env` (`.env.*` yalnızca `.env.x`'i yakalar)
- `*.tfstate`, `*.tfvars`
- `secrets.*`, `service-account*.json`
- `.pgpass`, `.htpasswd`, `.vault-token`
- `.docker/config.json`, `kubeconfig`, `.kube/config`
- `.yarnrc.yml` (`npmAuthToken`), `*.jks`, `*.ovpn`

Bu dosyalar onaysız okunup EVREN'e gidiyor. Yalıtımlı komut da bunları
okuyabiliyor.

Not: Gizli dosya tespiti doğası gereği bir engel listesi. AGENTS.md'deki
"izin listesi" ilkesi burada uygulanamaz.

**Öneri:**
- Liste genişletilsin.
- Kullanıcıya ait ek kalıplar ayarlardan verilebilsin; `Siniflandirici::yeni`
  zaten `ek_gizli` alıyor, ama arayüzde bu ayar yok.
- İçerikte anahtar biçimi arayan hafif bir tarama düşünülebilir: `-----BEGIN`,
  `AKIA…`, `ghp_…`, `evren_llm_…`.

### 5. Prompt injection savunması istem düzeyinde (Orta)

**Sorun:**
- Araç çıktısı modele `[kaynak: …]\n<içerik>` biçiminde gidiyor
  (`oturum/arac_asamasi.rs:661`). Bu etiket düz metin. İçerik kendi
  `[kaynak: kullanıcı]` satırını, sahte bir araç sonucunu ya da "kullanıcı
  onayladı" metnini taklit edebilir.
- Sistem istemindeki uyarı (`oturum/mod.rs:89`) faydalı ama tek başına
  güvenilir değil.
- Denetçi henüz yok (üst belge §9).

Etki, Bulgu 1–4'teki izin açıklarıyla sınırlı. İzinler doğruysa injection en
kötü durumda yanlış ama onaylı işler yaptırır.

**Öneri:**
- Araç çıktısı çağrı başına rastgele bir sınırla sarılsın, örneğin
  `<<<veri-3f9a…>>> … <<<son-3f9a…>>>`. İçerikteki sınır dizisi kaçışlansın.
- Sistem istemi bu sınırı anlatsın.
- Denetçi takvimde kalsın. Senaryo testleri (bulgular/awesome-harness
  incelemesi) injection içeren fikstürlerle genişletilsin.

### 6. Yönetici yardımcısında reparse noktası (Orta, Windows'ta doğrulanmalı)

**Sorun:**
- `kalfa-yalitim kur`, kurucu tarafından yönetici olarak çalıştırılıyor.
  `KurulumKaydi::yaz` (`kalfa-sandbox/src/windows/kurulum.rs:54`) önce
  `%ProgramData%\Kalfa\yalitim`'i `create_dir_all` ile açıyor. Sonra
  `mirasi_kes` ile DACL yazıyor ve oraya dosya yazıp yeniden adlandırıyor.
- `dacl_yaz`, `SetNamedSecurityInfoW` kullanıyor ve reparse noktasını izliyor.
  `guvenlik.rs`'de reparse denetimi yok.
- `%ProgramData%` altında standart kullanıcı klasör oluşturabilir ve
  oluşturduğunun sahibi olur. Başka bir yerel kullanıcı ya da zararlı yazılım
  `Kalfa` klasörünü önceden açıp `yalitim`'i bir junction'la başka bir dizine
  yönlendirebilir.
- Bu durumda yönetici süreci hedef dizinin DACL'ini değiştirir ve oraya dosya
  yazar. Bu, bilinen bir yerel yetki yükseltme örüntüsü.

**Öneri:**
- Kurucu `Kalfa` klasörünü sahibi Administrators olacak biçimde oluştursun.
  Klasör zaten varsa sahibini ve DACL'ini denetlesin; sahibi yönetici değilse
  dursun.
- Yazmadan önce her bileşen `FILE_FLAG_OPEN_REPARSE_POINT` ile açılsın;
  reparse noktası varsa reddedilsin.
- Windows'ta bir test eklensin: önceden oluşturulmuş junction ile `kur`
  başarısız olmalı.

### 7. Onay metinlerinde görünmez Unicode (Düşük–orta)

**Sorun:** Komut, dizin, yol ve diff hem webview'da
(`OnaySayfasi.svelte:161`, `<pre>`) hem yerel kutuda
(`kalfa-uygulama/src/guvenlik.rs:93`) olduğu gibi basılıyor. Şu karakterler
etkisiz hâle getirilmiyor:

- Yön değiştirenler: U+202A–202E, U+2066–2069.
- Sıfır genişlikli karakterler.
- CR ve diğer denetim karakterleri.

"Trojan Source" türü bir komut ya da diff, kullanıcıya gördüğünden farklı bir
şeyi onaylatabilir. Yerel kutuda çok uzun tek satırlık komutlar da kırpılabilir.

**Öneri:**
- Bu karakterler onay yüzeylerinde görünür kaçışla gösterilsin (`⟨U+202E⟩`)
  ve karta uyarı eklensin.
- Yerel kutuda komut belli bir uzunlukta kesilsin ve "tamamı onay sayfasında"
  notu eklensin.

### 8. Ağlı komutun çıkışı sınırsız (Düşük, tasarım)

**Sorun:** `ag: true` onaylanınca komut her hosta ulaşabiliyor. Onay kartı
bunu söylüyor, bu yüzden sorun tasarım kararı düzeyinde.

**Öneri (ileride):** İzin listeli bir çıkış vekili kullanılsın: crates.io,
npm, PyPI ve GitHub varsayılan; diğer hostlar ayrı onayla.

## İncelenmeyenler

- Windows AppContainer yalıtıcısı (`windows/yalitici.rs`, ~2000 satır)
  ayrıntılı okunmadı ve burada koşturulamaz. Bulgu 6 dışında Windows
  hakkında yargı yok.
- JSONL kurtarma sırasında yan etkinin yinelenip yinelenmediği, EVREN
  istemcisinin TLS ayarları ve e2e özelliğinin sürüm derlemesine
  sızmaması ayrıca doğrulanmadı.
- Canlı EVREN ile gerçek bir modele injection denenmedi.

## Önerilen sıra

1. **Bulgu 1:** Tek ve küçük bir değişiklik; `.git`'in tamamını korumalı
   saymak ve testleri. Proje kipi kullanılmadan önce yapılmalı.
2. **Bulgu 6:** Kurucu ve yardımcı. 2d-2 sürüm öncesi, Windows'ta.
3. **Bulgu 2 ve 3:** Korumalı yapılandırma sınıfı ve onay kartında onaysız
   değişiklik listesi. 2d-3 sınır matrisine eklenebilir.
4. **Bulgu 4 ve 7.**
5. **Bulgu 5 ve 8:** Aşama 3'ün denetçi ve senaryo testi işleriyle birlikte.
