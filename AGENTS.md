# AGENTS.md — Kalfa

Bu depoda çalışan kodlama ajanları (Antigravity, Claude Code ve benzerleri) için
yönergeler.

Kalfa, EVREN LLM Gateway için Türkçe, açık kaynak bir ajan harness'ı: Rust
workspace'i, Tauri v2 masaüstü kabuğu ve Svelte 5 arayüzü. Sahibi ve telif
sahibi Orkun Koray Güngör'dür. Lisans Apache-2.0'dır; `kalfa-plugin-sdk` ve WIT
dosyaları Apache-2.0 WITH LLVM-exception'dır.

## Önce okunacaklar

1. [`docs/GELISTIRME_DURUMU.md`](docs/GELISTIRME_DURUMU.md): nerede kalındığı ve
   sıradaki iş. Başka bir yerde çelişen bir bilgi görürseniz bu dosya ile
   `git log` esastır.
2. Tasarım belgeleri (`docs/superpowers/specs/`):
   - Üst belge: `2026-09-21-kalfa-design.md`.
   - 1c'nin tasarımı (Tauri ve Svelte kabuğu): `2026-09-22-kalfa-1c-design.md`.
   - Aşama 2'nin tasarımı (kabuk ve yalıtım):
     `2026-09-24-kalfa-2-kabuk-yalitim-design.md`.
3. Uygulama planları (`docs/superpowers/plans/`). Bir plan üzerinde
   çalışıyorsanız önce onun "Global Constraints" bölümünü okuyun.
4. Ölçüm ve prototip raporları (`docs/superpowers/bulgular/`).
5. Yerel iş takibi kılavuzu (varsa `.yerel/IS_TAKIBI.md`; depoya girmez):
   işe başlamadan ilgili görevin sahipliğini ve son devrini okuyun; sonucu
   orada da kaydedin. Teknik gerçek için `GELISTIRME_DURUMU.md` ile
   `git log` esastır.

## Kısa istekle devam

Kullanıcının "devam et", "Kalfa'da sıradaki işten devam et" veya "kaldığınız
yerden devam edin" demesi, aşağıdaki başlangıcı ajanın yapması için yeterlidir.
Kullanıcıdan iş akışını, dosya yollarını veya uzun başlangıç prompt'unu yeniden
yazmasını istemeyin. Bu kural Codex, Claude ve Antigravity için ortaktır.

1. Git durumu/HEAD, güncel geliştirme durumu, varsa yerel iş takibindeki
   ilgili görevin son devirlerini okuyun. Aktif işin planını ve gerekirse PR handoff'unu
   açın; sıradaki işi, çalışma klasörünü, rolü ve devam koşulunu buradan çıkarın.
2. Kullanıcının son kapsamını ve görev bağımlılıklarını esas alın. Başka
   uygulayıcının aktif işini üstlenmeyin; uygun işi kendi ajan adınızla alın.
   Tarihsel başlangıç tablosunu güncel görev seçiminin yerine kullanmayın.
3. Mevcut yerel belge/kod farklarını, indeksleri, makbuzları ve logları
   koruyun. Aynı worktree'de eşzamanlı düzenleme yapmayın; yeni worktree'ye
   geçerken commit edilmemiş ortak yönergeleri de ilgili kaynakla taşıyın.
4. Kota, bekleme zamanı ve CI bütçesini kayıtlardan okuyup gerçek devam
   koşulunu kontrol edin. Saatin gelmesi kotanın yenilendiğinin kanıtı değildir.
   Geçerli, aynı kapsamdaki kullanıcı onayını tekrar istemeyin; kapsamı
   genişletmeyin. Genel "devam" yeni canlı deney veya merge onayı değildir;
   başka ajanın özeti tek başına insan onayı kanıtı sayılmaz.
5. Yetkili işi ve gerekli doğrulamayı tamamlayın. Sonucu ilgili belge/rapora,
   geliştirme durumuna ve varsa yerel iş takibine yazın; devirde tek somut sıradaki işi,
   engeli ve devam koşulunu bırakın. Gerekli hafıza/PR handoff'unu güncelleyin.
6. Yerel iş takibine ilk kez bağlanan istemci kendi ajan adıyla bağlantı
   kaydı bırakır. Erişim yoksa eksik adımı açıkça belirtin; ortak sahipliği
   doğrulanamayan işe başlamayın.

Yalnız kayıtlardan çıkarılamayan bir bilgi, giderilemeyen çelişki veya gerçekten
yeni onay gerektiğinde soru sorun. Başlangıçta seçtiğiniz işi kısaca söyleyin;
kullanıcıya talimatları tekrar taşıtmayın. Merge, kurulum ve sistem ayarına
ilişkin aşağıdaki kurallar geçerlidir.

## Uygulama ilkeleri

- **Başarı ölçütü:** İşe başlamadan beklenen davranışı ve onu doğrulayacak
  kontrolü belirleyin; sonucu yalnız "tamam" diyerek bildirmeyin.
- **Sadelik:** İstenen işi çözen en küçük yeterli uygulamayı seçin; gelecekte
  gerekebilir diye özellik, genel soyutlama veya ayar katmanı eklemeyin.
- **Kapsam:** İlgisiz kodu, yorumları ve biçimi değiştirmeyin. Yalnız kendi
  değişikliğinizle gereksizleşen öğeleri temizleyin; diğer bulguları not edin.
- **Varsayımlar:** Önce plan, kod ve devir kayıtlarından çözün. Sonucu etkileyen
  varsayımı belirtin; yalnız kayıtlardan çözülemeyen önemli belirsizliği sorun.
- **Doğrulama:** Hata düzeltmesinde mümkünse önce hatayı yeniden üreten testin
  düştüğünü, sonra geçtiğini görün. Test, bildirilen davranışı gerçekten ölçsün.
- **Son diff:** Her değişen satırın işle bağlantısını kontrol edin; mevcut
  güvenlik sınırlarını ve gerekli doğrulamayı sadelik adına azaltmayın.

Esin kaynağı: [Karpathy gözlemlerinden türetilen ajan yönergeleri](https://github.com/multica-ai/andrej-karpathy-skills/blob/main/CLAUDE.md).

## Depo yerleşimi

| Yol | İçerik |
|---|---|
| `crates/kalfa-evren` | EVREN istemcisi: istek ve yanıt tipleri, SSE, şart uçları, kredi |
| `crates/kalfa-core` | Arayüzden bağımsız oturum çekirdeği: izin kipleri, mühürlü onay, JSONL kayıt ve kurtarma, git koruması |
| `crates/kalfa-tools` | Dosya araçları ve kabuk aracı |
| `crates/kalfa-sandbox` | Komut çalıştırma, süreç sınırları, Linux yalıtımı (bwrap ve seccomp) |
| `crates/kalfa-plugin` | Eklenti çalışma zamanı: manifest, wasmtime host'u, sınırlar, depo, eklenti aracı; test eklentileri `test-eklentileri/`'nde |
| `crates/kalfa-plugin-sdk` | Eklenti yazarının SDK'sı ve WIT sözleşmesi (`wit/kalfa.wit`); lisansı Apache-2.0 WITH LLVM-exception |
| `crates/kalfa-uygulama` | Uygulamanın Tauri'siz mantığı: oturumlar, köprü, yalıtım durumu, e2e sahteleri |
| `crates/kalfa` | Tauri kabuğu: komutlar, pencereler, yetkiler |
| `ui/` | Svelte 5 + TypeScript arayüzü (Vite, Vitest) |
| `e2e/` | `tauri-driver` + WebdriverIO uçtan uca testleri |
| `prototip/windows-yalitim/` | 2c Windows yalıtım prototipi; workspace'e girmez, ürün kodu değildir |
| `design/`, `assets/`, `research/` | Tasarım girdileri ve araştırma; dokunmayın |

## Kurulum ve doğrulama

Rust sürümü `rust-toolchain.toml` ile sabittir (1.98.1); eklentiler için
`wasm32-wasip2` hedefi de oradadır. Rustup hedefi kendiliğinden kurmazsa:
`rustup target add wasm32-wasip2 --toolchain 1.98.1`. Node sürümü
`^22.22.2 || >=24.15.0`'tır. Bağımlılıklar `--locked` ve `npm ci` ile kurulur.

`kalfa-plugin`'in testleri test eklentilerini `wasm32-wasip2` için kendileri
derler; ayrı bir komut gerekmez.

Her değişiklikten sonra:

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

Değişikliğin dokunduğu yere göre ayrıca:

- **`kalfa-uygulama` ya da `kalfa`:**

  ```sh
  cargo test -p kalfa-uygulama -p kalfa --features e2e --locked
  cargo clippy -p kalfa -p kalfa-uygulama --features e2e --all-targets --locked -- -D warnings
  ```

- **`kalfa-plugin-sdk`, `kalfa-rag` ya da test eklentileri:**

  ```sh
  cargo clippy --target wasm32-wasip2 --locked -p kalfa-plugin-sdk -p kalfa-rag -p kalfa-test-eklentisi -p kalfa-test-soket -p kalfa-test-yanlis-dunya -- -D warnings
  ```

- **`kalfa-sandbox` ya da `kalfa-tools`:** Aşağıdaki komutlar Linux'ta anlamlıdır
  ve root olmayan bir kullanıcıyla koşar. WSL'de bu kullanıcı `kalfatest`'tir.

  ```sh
  cargo clippy -p kalfa-sandbox -p kalfa-tools --features "kalfa-sandbox/yalitim-testleri kalfa-tools/yalitim-testleri" --all-targets --locked -- -D warnings
  cargo test -p kalfa-sandbox --features yalitim-testleri --test yalitim
  cargo build -p kalfa-sandbox --bin kalfa-kopru --locked
  cargo test -p kalfa-tools --features yalitim-testleri --test uyumluluk
  ```

  Ağlı testler kabın içindeki köprüyü `target/debug/kalfa-kopru`'dan alır.
  Köprü derlenmemişse ya da eskiyse testler `köprü bulunamadı` ile ya da eski
  ortamla koşar. Windows'ta uyumluluk testleri de koşturulur.

- **Arayüz** (`ui/` içinde):

  ```sh
  npm ci && npm run check && npm test && npm run build
  ```

  `npm run check` 0 hata ve 0 uyarı vermelidir. `npm run build` paket
  denetimini de içerir.

- **Uçtan uca** (`e2e/` içinde):

  ```sh
  npm ci && npm test   # Linux'ta: xvfb-run -a npm test
  ```

  - `tauri-driver` gerekir.
  - Windows'ta WebView2 sürümüyle aynı `msedgedriver` gerekir. PATH'te
    değilse yolunu `KALFA_E2E_EDGEDRIVER` ile verin.

Varsayılan testler EVREN anahtarı gerektirmez. Canlı API testleri `#[ignore]`
ile ayrıdır. Bir işin doğrulaması gerektiriyorsa ajan onları koşturabilir:

- Anahtar depo kökündeki `.env`'den (`EVREN_API_KEY`, `EVREN_BASE_URL`) okunur:
  `set -a && . ./.env && set +a && cargo test -p <crate> --test <test> <ad> -- --ignored`.
- Yalnızca işle ilgili testler koşturulur, bütün `--ignored` takımı değil;
  kredi harcarlar.
- Anahtar ekrana, günlüğe ya da commit'e yazılmaz.
- Şart kabul ucu hiçbir koşulda çağrılmaz (aşağıda "Yapılmayacaklar").

Bağlantı testleri Windows'ta sembolik bağlantı oluşturma izni ister.

### Platform notları

- **Linux:** Doğrulama WSL2'de, Ubuntu 24.04'te yapılır; dağıtımı
  `wsl -d Ubuntu-24.04` ile seçin, çünkü varsayılan dağıtım docker-desktop
  olabilir.
  - Git Bash'ten çağırırken `MSYS_NO_PATHCONV=1` verin.
  - `kalfa` crate'i `libwebkit2gtk-4.1-dev` ister. Yalıtım `bubblewrap` ister.
  - e2e yardımcısı Linux'ta `WEBKIT_DISABLE_COMPOSITING_MODE=1` verir: Xvfb'de
    yazılım GL birleştiricisi gecikme ölçümünü bozuyor.
- **Windows:** Bir işi bitmiş saymak için iki platformda da yeşil olmalıdır.

### Sürekli tümleştirme (CI)

`.github/workflows/ci.yml`, `main`'e her push'ta ve her PR'da koşar.

- **Kapsam:** Yukarıdaki varsayılan komutlar: `cargo fmt`, arayüz
  (`check`, `test`, `build`), üç clippy (düz, `e2e` özellikli ve eklentilerin
  `wasm32-wasip2`'si) ve iki `cargo test` (düz ve `e2e` özellikli). Rust işi
  Linux'ta ve Windows'ta koşar.
- **Kapsam dışı:** Yalıtım testleri (bwrap, seccomp, AppContainer) ve uçtan
  uca testler. Bunlar yukarıdaki gibi yerelde koşturulur; yeşil CI onların
  yerini tutmaz.
- **Linux paketleri:** `.github/workflows/linux-paket.yml`, paket dosyalarına
  dokunan PR'larda, `v*` etiketinde ve elle koşar: Debian 12 kabında `.deb`
  ve `.rpm` derler (glibc 2.36 tabanı), `SHA256SUMS` üretir, paketleri
  Debian 12, Ubuntu 24.04 ve Fedora kaplarında kurup kaldırır. Etikette
  taslak sürüm oluşturur; yayımlamayı kullanıcı yapar. AppArmor kabulü
  kapta yapılamaz (`kurucu/linux/kabul.sh`, VM).
- **Windows testleri** `.github/scripts/windows-testleri.ps1` ile, yönetici
  olmayan geçici bir `kalfatest` kullanıcısıyla, tek iş parçacığıyla
  (`--test-threads=1`) ve geçici dizin `C:\kalfatest-tmp` iken koşar. Testler
  yönetici hakkı, paralel koşu ya da profildeki `%TEMP%` varsaymamalıdır. Bu
  betik yalnızca CI makinesi içindir; kullanıcı açar ve sistem ayarı
  değiştirir, geliştirici makinesinde çalıştırmayın.
- **Değişiklik:** Buradaki doğrulama komutları değişirse `ci.yml` de aynı
  PR'da güncellenir. CI değişikliklerinin öneki `derleme:`'dir.
- **PR:** Kırmızı CI'lı PR birleştirmeye hazır sayılmaz. Hata sizin
  değişikliğinizden değilse (taban dalda da kırmızıysa) bunu PR'a yazın; testi
  atlamayın ya da devre dışı bırakmayın.

## Kod kuralları

- **Dil:**
  - Koddaki adlar, açıklamalar, testler ve commit mesajları Türkçedir.
  - Kullanıcıya görünen metinler `ui/src/dil/tr.ts`'te ya da Rust'ta sabittir
    ve "siz" diliyle yazılır.
  - Arayüzde "sandbox" değil "yalıtım" denir.
- **Hata işleme:** Rust'ta `unwrap()` ve `expect()` yalnızca testlerde ve
  `test_destek.rs`'de kullanılır. Üretim kodunda `Result` ve `Option`
  açıkça işlenir.
- **`unsafe`:** Yalnızca planın izin verdiği dosyalarda, her blok bir
  `// SAFETY:` açıklamasıyla yazılır.
- **Bağımlılıklar:** Planda yazmıyorsa yeni crate ya da npm paketi eklenmez.
- **Güvenlik sınırları:**
  - Filtreler engel listesiyle değil izin listesiyle yazılır. Örnekler seccomp
    soket kuralları ve ortam değişkenleridir.
  - Araç, çalışma anındaki duruma göre değil onaylanan tarife göre çalışır.
    Yalıtımlı onaylanan komut yalıtım yoksa başlamaz; tersi de geçerlidir.
  - Test edilmeyen güvenlik sınırı yoktur. Yeni bir sınır testiyle gelir.
- **Arayüz ve Tauri:**
  - CSP sıkıdır: şablonlarda sabit `style="…"` kullanılmaz ve ham HTML
    basılmaz. Markdown, AST üzerinden güvenli biçimde işlenir.
  - Pencerelere Tauri'nin çekirdek izinleri verilmez; durum kanallarla iletilir.
  - Arayüz arka uca dosya yolu göndermez. Yollar yerel klasör seçiciyle ya da
    kimlikle gelir.
- **Biçim:** Rust'ta `cargo fmt`. `npx prettier` kullanmayın: projede yok,
  çalıştırılınca paketi indirir ve stili bozar.

## Yapılmayacaklar

- EVREN şartlarını kullanıcı adına asla kabul etmeyin (üst belge §11.8).
  Canlı `POST /v1/terms/accept` çağrısı yapmayın.
- `.env` dosyalarını, anahtarları ve belirteçleri commit'lemeyin.
- Şunlara dokunmayın ve commit'lemeyin: `Screenshot*.png` (EVREN portalının
  ekran görüntüleri; bize ait değil), `refs/shot-*.png`, kökteki `tasks.png` ve
  `design/`.
- Paket kurma, dosya indirme ve sistem ayarı değiştirmeden önce kullanıcıdan
  izin alın.
- Windows'ta yönetici gerektiren adımları (UAC, ACL, kök izni, kullanıcı ya da
  güvenlik duvarı değişikliği) siz çalıştırmayın. Onları, geri alma betiğiyle
  birlikte kullanıcının çalıştıracağı bir PowerShell betiğine yazın.
- Makinedeki başka araçların yalıtım altyapısına dokunmayın. Örneğin Codex'in
  kendi Windows sandbox kullanıcıları ve grupları.

## İş akışı

- **Dallar:** Varsayılan dal `main`'dir. Her iş ayrı bir dalda ve PR
  ile yürür (`heimdilon/kalfa`, açık depo). PR #1–#53 ve eski CI kayıtları
  özel arşivdedir (`heimdilon/kalfa-arsiv`).
- **Planlı işler:**
  1. Claude planı yazar ve PR'ı açar. Planın kodu önceden iki platformda
     doğrulanmıştır.
  2. Antigravity PR'daki planı uygular. Kullanıcı Antigravity'de bu klasörde
     bir proje açar; Claude ona başlangıç prompt'unu verir.
  3. Claude inceler ve Windows ile WSL'de doğrular.
  4. Kullanıcının onayıyla PR squash olarak birleştirilir.
- **Plana bağlılık:** Plandan sapmanız gerekirse durun, gerekçeyi PR'a yazın.
  Plandaki bir kural kodla çelişiyorsa planı değil çelişkiyi bildirin.
- **Commit mesajları:** Türkçe ve önekli yazılır: `sandbox:`, `çekirdek:`,
  `araçlar:`, `uygulama:`, `ui:`, `e2e:`, `docs:`, `prototip:`, `derleme:`
  (workspace ve derleme ayarları).
  - Antigravity'nin commit'lerine Claude satırı eklenmez.
  - Claude'un commit'leri `Co-Authored-By` satırı taşır.
- **Worktree'ler:** Her worktree'nin `target`'ı 12–25 GB tutar. PR birleşince
  ya da deneme bitince worktree'yi aynı oturumda kaldırın:
  `git worktree remove <yol>`, ardından `git worktree prune`.
  - Commit'lenmemiş değişiklik varsa önce `git diff HEAD --binary` ile yama
    olarak saklayın.
  - Uzun yollar yüzünden kaldırma yarıda kalabilir. Klasör kaldıysa
    `git worktree list` ile kaydın gittiğini doğrulayıp klasörü Git Bash'te
    `rm -rf` ile silin.
  - WSL'deki doğrulama kopyaları ve ayrı `CARGO_TARGET_DIR`'lar da iş bitince
    silinir.
- **Paylaşılan dosyalar:** `docs/GELISTIRME_DURUMU.md` ve README'yi
  commit'lemeden önce `git diff` ile başkasının düzenlemesi var mı bakın.
- **Belgeler:** Bir aşama bitince `docs/GELISTIRME_DURUMU.md` ve ilgili tasarım
  belgesi güncellenir. Ölçüm sonuçları `docs/superpowers/bulgular/`'a yazılır.

## Windows yalıtımı (aşama 2d)

Tasarım AppContainer'a göredir (aşama 2 tasarımı §9; 2c'de ölçüldü).

- **Çalıştırma:** Komut, geliştiricinin hesabıyla, çalışma alanına özel bir
  AppContainer'da ve bir Job Object içinde çalışır.
- **Kimlikler:** Çalışma alanını paket SID'i ayırır. Araç zinciri dizinleri
  bir yetenek SID'ine okuma izni verir.
- **Korumalı yollar:** AppContainer, paket SID'ine verilen ret (deny) izinlerini
  uygulamaz. Korumalı yollar ve gizli dosyalar, mirası kesip izin vermeyerek
  korunur.
- **Kök izni:** `C:\` köküne verilen listeleme izni yalnızca kökü değiştiren
  `SetFileSecurityW` ile yazılır, `SE_DACL_AUTO_INHERIT_REQ` bayrağıyla.
  `SetNamedSecurityInfo` mirası bütün sürücüye yayar. Mirassız her izin
  (örneğin üst dizinlere listeleme) aynı yolla yazılır ve silinir.
- **Gerçek yalıtım testleri** başka bir ajanın kendi yalıtımı içinde koşamaz
  (Codex'in yalıtımında `CreateAppContainerProfile` açılamıyordu); kurulu
  makinede Claude koşturur.
  Testler makinede Kalfa AppContainer profili ya da `S-1-15-2-` ACE'si
  bırakmamalıdır.
- **Prototip:** Ölçüm aracı ve betikler `prototip/windows-yalitim/`'dedir
  (README'ye bakın).
