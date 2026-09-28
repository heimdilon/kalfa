# Kalfa 2d-3 — Kalfa'nın getirdiği araç zinciri: Uygulama Planı

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Teknik olmayan bir kullanıcının kurduğu Kalfa, kapta Python (pip, venv, pytest) ve Node (npm) çalıştırır. Kullanıcının kurduğu araçlar gelişmiş ayar olarak kalır. Yükseltmede kaybolan araç zinciri izinlerini "Onar" geri getirir.

**Architecture:**
- Kurucu derlemesi, Python ve Node'un sabit sürümlerini SHA-256 denetimiyle indirir ve `$INSTDIR\araclar\` altına koyar. Program Files'ı `ALL APPLICATION PACKAGES` zaten okur; izin gerekmez.
- Kurulu Kalfa bu dizinleri bulur. Windows arka ucu onları kabın `PATH`'inin başına koyar.
- Git dağıtılmaz. Kullanıcının kurduğu Git for Windows Program Files'tadır ve kapta zaten okunur.
- Onaylanan tarif ile çalışma anı durumunun karşılaştırması iki arka ucun ortak bir işlevine taşınır ve bir sınır matrisiyle sınanır.
- Sahibi olunmayan araç zinciri dizini (ör. `C:\Program Files\nodejs`) Ayarlar'dan UAC ile açılır. "Onar", `kur`'la birlikte bu dizinleri de yeniden açar.

**Tech Stack:** Rust 1.98.1 (windows-sys), Svelte 5 + TypeScript, Tauri 2 NSIS kurucusu, Node betikleri (`node:test`).

**Spec:** [Aşama 2 tasarımı](../specs/2026-09-24-kalfa-2-kabuk-yalitim-design.md) §7.4, §9.6, §9.8, §10.4, §12.3, §14 (A15, A16), §15; [ayrı kullanıcı karşılaştırması](../bulgular/2026-09-27-kalfa-2d-ayri-kullanici-karsilastirma.md); [2d-2 planı](2026-09-27-kalfa-2d-2-son-kullanici.md) (inceleme ve kabul notları).

**Yürütme (kullanıcı kararı, 2026-09-27):** Kodu Claude PR dalına görev görev yazar ve iki platformda doğrular. Plan hafiftir: görevler, arayüzler, testler ve kabul ölçütleri. Tam kod PR'ın commit'lerindedir.

## Kararlar (2026-09-27)

- **MinGit dağıtılmaz** (kullanıcı kararı). MinGit GPLv2'dir; dağıtılırsa her sürümde kaynağının da yayınlanması gerekir. Kullanıcının kurduğu Git for Windows `C:\Program Files\Git`'tedir. `ALL APPLICATION PACKAGES` orayı okur ve kap Program Files altındaki `PATH` girdilerini zaten korur. Git kurulu değilse model `git` çalıştıramaz; Kalfa'nın kendisi git olmadan çalışır (`git_koruma.rs` ve `GitKimligi::oku` git yoksa boş döner).
- **Python ve Node kurucuya girer.** Lisansları (PSF, MIT) yalnızca lisans metinlerini ister; metinler paketlerin içindedir ve kurulur.
- **İndirme izni** (kullanıcı, 2026-09-27): A16 ölçümü ve kurucu derlemesi için Python, Node ve Git for Windows adaylarının resmi kaynaklarından indirilmesi.
- **Kaldırmada oturum kayıtları silinmez** (Claude kararı; kullanıcı değiştirebilir). Tauri'nin "uygulama verisini sil" seçeneği yalnızca `dev.kalfa.masaustu` dizinlerini (WebView önbelleği) siler. `%APPDATA%\Kalfa` ve `%LOCALAPPDATA%\Kalfa` kalır. Gerekçe: ACE kaydı olmadan hazırlanmış alanların izinleri geri alınamaz. Kurucu makine genelidir, kaldırıcı başka bir hesabın yükseltilmiş belirteciyle çalışabilir ve o hesabın verisine dokunamaz. Kalfa'nın kendi verisi kaldırıcıdan değil Ayarlar'dan temizlenir (hazırlanmış alanlar 2d-2'de geldi). README ve tasarım bunu söyler.
- **Rust ve MSVC otomatik algılaması sürer.** `.cargo\bin`, `.rustup` ve Visual Studio bulunursa bugünkü gibi açılır. "Gelişmiş" olmaları varsayılan kurulumun onları istememesi demektir (§9.8).

## Global Constraints

- **Araçlar:** Rust 1.98.1; `cargo` komutları `--locked` ile. Node `^22.22.2 || >=24.15.0`; kurulum `npm ci`.
- **Bağımlılık:** yeni crate ya da npm paketi eklenmez. İndirilen Python ve Node paketleri bağımlılık değil, kurucu içeriğidir. Sürüm ve SHA-256'ları `crates/kalfa/kurucu/araclar.json`'da sabittir; eşleşmeyen dosya kullanılmaz.
- **Güvenlik sınırları:**
  - Araç onaylanan tarife göre çalışır. Yalıtımlı onaylanan komut yalıtım yoksa, yalıtımsız onaylanan komut yalıtım varsa başlamaz. Yalıtımsız onaylanan komut anahtar kapalıysa başlamaz.
  - Kalfa'nın araç dizinleri yalnızca kurulu sürümde, Program Files'ın altında Kalfa'nın yanında duruyorsa kullanılır (`sistem::kurulu_yardimci` ile aynı kural). Geliştirme derlemesi `target\` altındaki bir dizini araç zinciri saymaz.
  - Yükseltilmiş yardımcıya verilen dizinler yalnızca uygulamanın kayıtlı ayarından gelir, arayüzden gelmez. Yardımcı her dizini `izin-ver`'in bugünkü denetimleriyle yeniden doğrular.
  - `~/.cargo/credentials.toml` ve `~/.cargo/config.toml` kapta hiçbir yolla okunamaz.
  - Test edilmeyen güvenlik sınırı yoktur.
- **`unsafe`:** yalnızca `crates/kalfa-sandbox/src/windows/`'da, her blok `// SAFETY:` açıklamasıyla.
- **Dil:**
  - Kod, test ve açıklamalar Türkçedir. Kullanıcıya görünen metinler `ui/src/dil/tr.ts`'te ya da Rust'ta sabittir ve "siz" diliyle yazılır. Arayüzde "yalıtım" denir.
  - `unwrap()`/`expect()` yalnızca testlerde ve `test_destek.rs`'de kullanılır.
- **Arayüz:** CSP sıkıdır. Arayüz yol göndermez. Pencerelere Tauri'nin çekirdek izinleri verilmez.
- **Her görevin sonunda:**
  - `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --all -- --check`.
  - Uygulamaya dokunulduysa `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked` ve aynı özellikle clippy.
  - Yalıtıma dokunulduysa Windows'ta `cargo test -p kalfa-sandbox --features yalitim-testleri --test yalitim` ve `cargo test -p kalfa-tools --features yalitim-testleri --test uyumluluk`; WSL'de aynıları.
  - Arayüze dokunulduysa `ui/`'da `npm run check` (0/0), `npm test`, `npm run build`.
  - Kurucu betiklerine dokunulduysa `node --test "crates/kalfa/kurucu/*.test.mjs"` (Node 24 dizin argümanını açmaz).
  - Görev bitmeden önce Windows'ta ve WSL'de (Ubuntu-24.04) yeşil olmalıdır. Windows ve WSL uçtan uca testleri aynı anda koşmaz.
- **Commit:** Türkçe ve önekli (`sandbox:`, `araçlar:`, `uygulama:`, `ui:`, `e2e:`, `docs:`); Claude'un commit'leri `Co-Authored-By` taşır.
- **Dokunulmayanlar:** `design/`, `.env`, `Screenshot*.png`, kökteki `tasks.png`.
- **Plana bağlılık:** Görev 1'in ölçümü bir kabul ölçütünü karşılamazsa sonraki görevlere geçilmez; sonuç kullanıcıya sunulur.

## Dosya Yapısı

```
crates/kalfa/kurucu/araclar.json                  sürüm, adres, SHA-256, iç dizin          yeni
crates/kalfa/kurucu/araclar.mjs                   indirme, özet denetimi, açma             yeni
crates/kalfa/kurucu/araclar.test.mjs              node:test                               yeni
crates/kalfa/kurucu/derle.mjs                     araclar.mjs'i çağırır
crates/kalfa/tauri.nsis.conf.json                 bundle.resources: araclar/
crates/kalfa-sandbox/src/lib.rs                   kip_uyusmasi(); okunur_yonetici_gerekli()
crates/kalfa-sandbox/src/linux/mod.rs             kip_uyusmasi() kullanır
crates/kalfa-sandbox/src/windows/yalitici.rs      kalfa_araclari PATH'in başında; cargo gelişmiş yolu
crates/kalfa-sandbox/src/windows/sistem.rs        kurulu_arac_zinciri()
crates/kalfa-yalitim/src/main.rs                  kur [--izin-ver <dizin>]...
crates/kalfa-uygulama/src/{arayuz,yalitim,yukseltici}.rs   Yukseltici::kur(dizinler), izin_ver
crates/kalfa/src/lib.rs                           kurulu araç zincirini WindowsAyari'ya verir
crates/kalfa-tools/tests/{kabuk,uyumluluk}.rs     sınır matrisi; Windows'ta Kalfa'nın araçları
ui/src/ana/Ayarlar.svelte, ui/src/dil/tr.ts       Windows'ta araç zinciri dizinleri "Gelişmiş"te
docs/superpowers/bulgular/2026-09-27-kalfa-2d-3-a16.md    ölçüm                          yeni
```

---

### Task 1: A16 ölçümü ve paket seçimi

**Files:** `docs/superpowers/bulgular/2026-09-27-kalfa-2d-3-a16.md` (yeni), `crates/kalfa/kurucu/araclar.json` (yeni), ölçüm betikleri scratchpad'de (depoya girmez).

**Interfaces (Produces):**
- `araclar.json`:
  ```json
  {
    "python": { "surum": "3.14.x", "adres": "https://…", "sha256": "…", "ic_dizin": "tools", "lisans": "LICENSE.txt" },
    "node":   { "surum": "24.x.y", "adres": "https://nodejs.org/dist/v24.x.y/node-v24.x.y-win-x64.zip", "sha256": "…", "ic_dizin": "node-v24.x.y-win-x64", "lisans": "LICENSE" }
  }
  ```
  Değerler ölçümde seçilen paketlerindir. `sha256` yayıncının yayımladığı özetle karşılaştırılarak yazılır (Node için `SHASUMS256.txt`, NuGet için paket imzası ya da python.org'un yayımladığı özet).
- Bulgular belgesi: her araç ve komut için sonuç, hata metni ve çözüm; kabın ortamında gereken ek değişkenler.

**Ölçüm düzeni:** Paket geçici bir dizine açılır ve dizine `ALL APPLICATION PACKAGES` (`S-1-15-2-1`) için okuma ve çalıştırma izni verilir. Bu, Program Files'ın durumudur ve yönetici gerektirmez: dizinin sahibi geliştiricidir. Komutlar ürünün yolundan, `kalfa-tools/tests/uyumluluk.rs` düzeneğiyle (geçici bir test dosyasında) gerçek AppContainer'da çalıştırılır. Paket dizini `WindowsAyari.ek_okunur` ile değil, bu izinle okunur; kurulu sürümün koşulu budur.

- [x] **Step 1: Python adayı.** Python 3.14'ün NuGet paketi (`python`, python.org'un yayımladığı, `tools\` altında tam standart kitaplık). Gömülebilir paket `venv` ve `pip` içermediği için aday değildir. Ölçülenler:
  1. `python -c "import sys, ssl, sqlite3, json; print(sys.prefix)"`: açılışta "Failed to find real location" uyarısı var mı, `sys.prefix` doğru mu (A16).
  2. `python -m ensurepip` derleme makinesinde, kap dışında çalıştırılır; sonra kapta `python -m pip --version`.
  3. Kapta `python -m venv .venv` ve `.venv\Scripts\python -c "print(1)"`.
  4. Kapta ağlı `python -m pip install --user <küçük saf Python paketi>` ve ağsız `pip install` hata iletisi.
  5. Kapta `python -m pytest` (pytest venv'e ağlı kurulur, test ağsız koşar).
  - Uyarı ya da yanlış `sys.prefix` varsa denenecekler sırasıyla: `PYTHONHOME`'u paket dizinine vermek; paket dizinine `python314._pth` koymak. Kabul edilen çözüm ortam değişkeni ya da paket içi dosya olmalıdır; kaba yazma izni gerektirmemelidir.
- [x] **Step 2: Node adayı.** Node 24 LTS'nin en son `win-x64.zip`'i. Ölçülenler: `node -e "console.log(require('fs').realpathSync('.'))"`, `node -e "require('fs').realpathSync.native('.')"` (A16: EPERM beklenir), ağlı `npm ci` ve ağsız `npm test` (`kalfa-tools/tests/ornekler/npm` örneği), `npx --no-install <yerel bin>`.
  - `realpathSync.native` EPERM verirse npm'in ve örnek projenin etkilenip etkilenmediği yazılır. Etkilenmiyorsa bilinen sınırdır (§9.6).
- [x] **Step 3: Git for Windows.** Makinede kurulu `C:\Program Files\Git\cmd\git.exe` ile kapta `git status`, `git diff`, `git log` (worktree dahil). Git'in kapta Program Files'tan bulunduğu (`where git`) doğrulanır. Kurulu değilse Git for Windows'un taşınabilir sürümü aynı izin düzeniyle ölçülür.
- [x] **Step 4: A15'in gelişmiş yolu.** Geliştiricinin `~/.cargo/registry`'si ile bağımlı bir crate'in (`kalfa-tools/tests/ornekler/cargo`) ağsız derlenmesi, iki yolla:
  - **(a)** `CARGO_HOME` = çalışma alanına özel dizin; içindeki `registry` bir dizin bağlantısıdır (junction), geliştiricinin `registry`'sini gösterir. `registry` dizinine yetenek SID'i için okuma verilir (geliştirici sahibidir, yönetici gerekmez). `CARGO_NET_OFFLINE=true`.
  - **(b)** `CARGO_HOME` = geliştiricinin `~/.cargo`'su, salt okunur; `credentials.toml` ve `config.toml` mirası kesilerek gizlenir. `CARGO_NET_OFFLINE=true`.
  - Her birinde: derleme süresi, cargo'nun registry'ye yazmaya çalışıp çalışmadığı, eksik crate'te hata metni. Seçim ölçütü: `credentials.toml` ve `config.toml` hiçbir yolla okunamaz, geliştiricinin dizinine hiçbir şey yazılamaz, derleme 2c'deki 12 sn'den kötü değildir. İkisi de ölçütü karşılamazsa A15 bilinen sınır kalır ve Görev 6 yalnızca hata iletisini yapar.
- [x] **Step 5:** Bulguları yaz, `araclar.json`'u doldur. Kabul ölçütleri: Python'da 1, 3, 4 ve 5 çalışır; Node'da `npm ci` ve `npm test` çalışır; Git'te `status`, `diff` ve `log` çalışır. Karşılanmayan ölçüt varsa dur ve kullanıcıya sun.
- [x] **Step 6:** Commit `docs: 2d-3 A16 ölçümü; araç zinciri paketleri seçildi`.

**Sonuç (2026-09-27):** [Bulgular](../bulgular/2026-09-27-kalfa-2d-3-a16.md). Python 3.14.7 (NuGet, pip içinde) ve Node 24.21.0 seçildi. Node ve Git ek bir şey istemiyor. Python, Kalfa'nın `sitecustomize.py`'siyle (`crates/kalfa/kurucu/python/`) çalışıyor: `0o700` geçici dizin izni ve DOS yol adı. venv'de pip, `cacert.pem` Kalfa'nın gizli kalıbına takıldığı için düştü; kullanıcı kararıyla Görev 2b eklendi. cargo için yol (b) seçildi.

### Task 2: Onaylanan tarif ile çalışma anının sınır matrisi

**Files:** `crates/kalfa-sandbox/src/lib.rs`, `crates/kalfa-sandbox/src/linux/mod.rs`, `crates/kalfa-sandbox/src/windows/yalitici.rs`, `crates/kalfa-tools/tests/kabuk.rs`.

**Interfaces (Produces):**
- `kalfa_sandbox::kip_uyusmasi(yalitimli_onaylandi: bool, yalitim_hazir: bool) -> Result<(), BaslatmaHatasi>`. İki arka ucun `baslat`'ındaki eşleştirme bu işleve taşınır; hata metinleri bugünkülerdir.

- [x] **Step 1: Başarısız testleri yaz**
  - `kip_uyusmasi_dort_hucre` (sandbox, iki platform): `(true, true)` ve `(false, false)` `Ok`; `(true, false)` "artık kullanılamıyor"; `(false, true)` "artık kullanılabiliyor".
  - `sinir_matrisi` (tools, `kabuk.rs`): `KayitliYalitici`'ye çalışma anı durumunu değiştiren bir anahtar eklenir ve `baslat`'ı `kip_uyusmasi`'nı çağırır. Hücreler: onaylanan kip {yalıtımlı, yalıtımsız} × çalışma anında yalıtım {hazır, yok} × anahtar {açık, kapalı} × ağ {açık, kapalı}. Her hücre için hazırlık onay anındaki durumla yapılır, durum `calistir_akisli`'dan önce değiştirilir. Beklenen: süreç yalnızca (yalıtımlı ∧ hazır) ya da (yalıtımsız ∧ yok ∧ anahtar açık) iken başlar; başlayan istekte `ag` onaylanan değerdir; başlamayan hücrede işaret dosyası oluşmaz ve güven kaydedilmez.
- [x] **Step 2:** Başarısız olduklarını gör (işlev yok).
- [x] **Step 3:** Uygula; iki arka uç `kip_uyusmasi`'nı kullanır.
- [x] **Step 4:** Denetimler iki platformda.
- [x] **Step 5:** Commit `sandbox, araçlar: onaylanan kip ile çalışma anı ortak işlevde; sınır matrisi`.

**Not:** Görevi bir alt ajan `2d-3/g2-sinir-matrisi` dalında yazdı (3359545, Linux'ta doğrulandı). Claude inceledi, Windows'ta doğruladı (matris 16 hücre, gerçek yalıtım 29/29) ve bu dala aldı. Çalışma anı anahtarı olarak `KayitliYalitici`'nin mevcut `yok` alanı kullanıldı.

### Task 2b: Bağımlılık dizinleri gizli taramasının dışında

**Karar:** kullanıcı, 2026-09-27 (Görev 1'in bulgusu). Gizli kalıplar (`*.pem`, `credentials*` …) araçların ürettiği bağımlılık dizinlerinde uygulanmaz. Bedeli: kullanıcı bu dizinlere kendisi bir sır koyarsa gizlenmez; tasarım §7.2 bunu söyler.

**Files:** `crates/kalfa-core/src/izin.rs`, `crates/kalfa-core/tests/izin.rs`, `crates/kalfa-tools/tests/uyumluluk.rs`, tasarım §7.2.

**Interfaces (Produces):**
- `kalfa_core::izin::BAGIMLILIK_DIZINLERI`: `node_modules`, `site-packages`, `dist-packages`.
- `kalfa_core::izin::bagimlilik_dizininde_mi(yol: &CalismaYolu) -> bool`: yolun bir üst dizininin adı (büyük-küçük harf duyarsız) bunlardan biri.
- `Siniflandirici::gizli_mi` böyle bir yolu gizli saymaz. Kabuğun taraması ve dosya araçları aynı sınıflandırıcıyı kullanır.
- Tarama bu dizinlere yine iner: gizli bir dosyanın oraya konmuş hard link'i (Linux) gizli kalır.
- İlk taslaktaki `pyvenv.cfg` denetimi alınmadı: taramada dosya başına disk erişimi isterdi; `cacert.pem` zaten `site-packages`'in altında.

- [x] **Step 1: Başarısız testleri yaz**
  - `bagimlilik_dizininde_gizli_sayilmaz` (core `tests/izin.rs`).
  - `bagimlilik_dizinindeki_dosyalar_okunur_gizliler_okunamaz` (tools `uyumluluk.rs`, iki platform, gerçek yalıtım): `.venv\…\site-packages\…\cacert.pem` ve `node_modules\…\credentials.js` okunur; kökteki `.env` ve `anahtar.pem` okunamaz.
  - `bagimlilik_dizinindeki_hard_link_gizli_kalir` (tools `uyumluluk.rs`, Linux): `node_modules/pkg/index.js`, `.env`'nin hard link'i; kapta okunamaz.
- [x] **Step 2:** Başarısız olduklarını gör (Windows'ta gerçek yalıtım testi çekirdek değişikliği olmadan düşüyor).
- [x] **Step 3:** Uygula.
- [x] **Step 4:** Denetimler iki platformda, yalıtım testleri dahil (Windows: uyumluluk 7/7, yalıtım 29/29; WSL: uyumluluk 6/6, yalıtım 17/17).
- [x] **Step 5:** Commit `çekirdek: bağımlılık dizinleri gizli kalıpların dışında`.

### Task 3: Kurucuya Python ve Node

**Files:** `crates/kalfa/kurucu/{araclar.mjs,araclar.test.mjs,derle.mjs,README.md}`, `crates/kalfa/tauri.nsis.conf.json`, `.gitignore` (`/crates/kalfa/araclar/`).

**Interfaces (Produces):**
- `araclar.mjs`:
  - `export function ozetDenetle(dosya: string, beklenen: string): void`: SHA-256 eşleşmezse dosyayı siler ve hata atar.
  - `export async function hazirla(manifest, onbellek: string, hedef: string): Promise<void>`: her araç için önbellekte yoksa indirir, özeti denetler, `tar.exe -xf` (Windows 10+ sistem `tar`'ı, zip açar) ile açar, `ic_dizin`'i `hedef/<ad>`'a taşır, lisans dosyasının varlığını denetler.
- `derle.mjs`: yardımcıyı derledikten sonra `hazirla(araclar.json, target/kurucu-onbellek, crates/kalfa/araclar)`, sonra `crates/kalfa/kurucu/python/sitecustomize.py`'yi `crates/kalfa/araclar/python/Lib/`'e kopyalar. pip paketin içinde gelir; `ensurepip` gerekmez (Görev 1).
- `tauri.nsis.conf.json`: `bundle.resources` `{"araclar/python/": "araclar/python/", "araclar/node/": "araclar/node/"}`. Kurulumda `$INSTDIR\araclar\python\python.exe` ve `$INSTDIR\araclar\node\node.exe` olur.

- [x] **Step 1: Başarısız testleri yaz** (`araclar.test.mjs`, `node:test`, ağsız):
  - `ozet_eslesmezse_dosya_silinir_ve_hata_atilir`.
  - `ozet_eslesirse_dosya_kalir`.
  - `lisans_yoksa_hazirla_basarisiz`: geçici bir zip (`tar -a -cf` ile üretilir), manifestte olmayan bir lisans adı.
  - `onbellekteki_dosya_yeniden_indirilmez`: adres geçersiz, önbellekte doğru özetli dosya var; `hazirla` başarılı.
- [x] **Step 2:** Başarısız olduklarını gör.
- [x] **Step 3:** Uygula; `derle.mjs` kabuksuz kalır (2d-2 incelemesi).
- [x] **Step 4:** `node crates/kalfa/kurucu/derle.mjs` ile kurucuyu derle. `crates/kalfa/araclar/` altında `python\python.exe`, `node\node.exe` ve iki lisans dosyasının olduğunu ve kurucunun boyutunu kaydet. Kurulu yerleşim Görev 7'nin kabulünde denetlenir (kurmak yönetici ister).
- [x] **Step 5:** Commit `uygulama: kurucu Python ve Node'u sabit sürüm ve özetle getirir`.

**Sonuç (2026-09-27):** Kurucu 5,9 MB'tan 42,7 MB'a çıktı. `araclar/` 44 MB Python (pip 26.2.1 içinde) ve 107 MB Node (npm 11.19.0); NSIS betiğinde 3024 Python ve 4870 Node dosyası var. Açılan dizin `hedef/<ad>`'a taşınmıyor, kopyalanıyor: yeni açılmış `.exe`'leri virüs taraması tutarken Windows dizin taşımasını EPERM ile reddetti. Ek test: `gecersiz_ad_ve_ic_dizin_reddedilir` (araç adı, `..`'lı iç dizin, https olmayan adres). Zip isteyen iki test Linux'ta atlanır; kurucu yalnızca Windows'ta derlenir. Doğrulama: Windows 444 test, WSL 402 test; kurucu testleri Windows 5/5, WSL 3/3 (2 atlandı).

### Task 4: Kalfa'nın araçları kabın PATH'inde önce

**Files:** `crates/kalfa-sandbox/src/windows/{sistem,yalitici}.rs`, `crates/kalfa-sandbox/tests/yalitim/windows.rs`, `crates/kalfa/src/lib.rs`, `crates/kalfa-uygulama/src/yalitim.rs`, `crates/kalfa-tools/tests/uyumluluk.rs`.

**Interfaces:**
- Consumes: Görev 3'ün kurulum yerleşimi (Görev 1: ek ortam değişkeni gerekmez), Görev 2b.
- Produces:
  - `windows::sistem::kurulu_arac_zinciri(exe: &Path) -> Vec<PathBuf>`: `exe` Program Files'ın altında Kalfa'nın yanındaysa (`kurulu_yardimci` ile aynı denetim) var olan `araclar\python`, `araclar\python\Scripts`, `araclar\node` dizinleri, bu sırayla; değilse boş.
  - `WindowsAyari.kalfa_araclari: Vec<PathBuf>`. Kabın `PATH`'i: önce `kalfa_araclari` (verilen sırayla), sonra bugünkü süzülmüş ve sıralı liste. Kalfa'nın `node`'u kullanıcının `C:\Program Files\nodejs`'inden önce bulunur.
  - Kapta pip'in kullanıcı kurulumunun `Scripts` dizini (`<özel ev>\Roaming\Python\Python314\Scripts`) `PATH`'e eklenir.
  - Kalfa, `KALFA_ARAC_ZINCIRI` ortam değişkeniyle yalnızca `e2e` özellikli derlemede ve testlerde başka bir dizin verebilir; üretim derlemesi bu değişkeni okumaz.

- [x] **Step 1: Başarısız testleri yaz**
  - `kurulu_arac_zinciri_program_files_disinda_bos` (sandbox, Windows): `target\debug`'daki test ikilisi için boş.
  - `kalfa_araclari_path_in_basinda` (sandbox birim testi, `yalitim_ortami`): `PATH`'te `C:\Program Files\nodejs` varken `kalfa_araclari` ondan önce gelir; sıralama `kalfa_araclari`'nın kendi sırasını bozmaz.
  - `kalfa_python_ve_node_kapta_calisir` (tools `uyumluluk.rs`, Windows, `KALFA_TEST_ARAC_ZINCIRI` verilmişse): değişken, `ALL APPLICATION PACKAGES`'a okuma açılmış ve üst dizini `C:\` olan bir kopyayı gösterir (Görev 1'in düzeni; Görev 3'ün `crates/kalfa/araclar`'ından kopyalanır). Kapta çalışanlar: `python -m venv .venv`, venv'de ağlı `pip install pytest` ve ağsız `python -m pytest`, ağlı `pip install --user`, `os.path.realpath('.')` doğru yol, ağlı `npm ci`, ağsız `npm test`; `where python` ve `where node` Kalfa'nınkini önce gösterir. Değişken yoksa test "araç zinciri verilmedi" diye atlanır; doğrulama tablosu hangi koşuda verildiğini yazar.
  - `git_program_files_ten_calisir` (tools `uyumluluk.rs`, Windows, `C:\Program Files\Git\cmd\git.exe` varsa): mevcut git testinin `ek_okunur`'suz hâli.
- [x] **Step 2:** Başarısız olduklarını gör.
- [x] **Step 3:** Uygula; `kalfa::kur` kurulu araç zincirini `current_exe()`'den bulur ve uygulamanın kuruluş ayarlarına `ev_dizini` ve `yalitim_dizini`'nin yanında `kalfa_araclari` olarak verir; `yalitici_kur` onu `WindowsAyari`'ya geçirir.
- [x] **Step 4:** Denetimler iki platformda; `KALFA_TEST_ARAC_ZINCIRI` ile Windows uyumluluk testleri.
- [x] **Step 5:** Commit `sandbox, uygulama: Kalfa'nın Python ve Node'u kabın PATH'inde önce gelir`.

**Sonuç (2026-09-27):**
- `sistem::arac_zinciri_dizinleri(kok)` kurulu yerleşimi ve test kopyasını aynı kuralla çözer. `kalfa_uygulama::dizin::kalfa_araclari(exe, ortam)` `KALFA_ARAC_ZINCIRI`'yi yalnızca `e2e` derlemesinde kullanır; `kalfa::kur` değişkeni de yalnızca o derlemede okur (test: `kalfa_araclari_kurulum_ya_da_e2e_degiskeninden_gelir`, iki özellikle).
- MSVC ortamı `bin`'ini zaten en başa koyuyor (2d-1: Git'in `link.exe`'si). Kalfa'nın dizinleri ondan sonra, kullanıcının araçlarından önce gelir; birim testi Kalfa'nın Python'undan önceki hiçbir dizinde `python.exe` ya da `node.exe` olmadığını sınar. Testte `C:\Program Files\nodejs` yerine geçici bir "kullanıcı node'u" kullanıldı.
- pip'in kullanıcı `Scripts` dizini, sürümü Kalfa'nın `python3NN.dll`'inden çıkarılarak, dizin henüz yokken de `PATH`'e girer.
- Kabuk aracı mutlak program yolunu kabul etmez; venv'in Python'u göreli yolla çağrılır. Python her açılışta `Failed to find real location` uyarısını basar (G1'in bilinen sınırı, §9.6'ya G7'de).
- Doğrulama: Windows 449 + e2e-özellik 84, yalıtım 29/29, uyumluluk 9/9 (`KALFA_TEST_ARAC_ZINCIRI=C:\kalfa-a16`, `crates/kalfa/araclar`'ın kopyası); WSL 403 + e2e-özellik 84, `kalfatest` ile yalıtım 17/17, uyumluluk 6/6. Makinede yeni `Kalfa.Alan.*` profili kalmadı.

### Task 5: Sahibi olunmayan dizin UAC ile; Onar izinleri yeniden açar

**Files:** `crates/kalfa-yalitim/src/main.rs`, `crates/kalfa-sandbox/src/{lib,windows/yalitici}.rs`, `crates/kalfa-uygulama/src/{arayuz,yalitim,yukseltici}.rs`, `crates/kalfa-uygulama/tests/yalitim.rs`, `crates/kalfa/src/*`, `ui/src/{ana/Ayarlar.svelte,dil/tr.ts,ipc/*}`, `ui/src/ana/ayarlar.test.ts`.

**Interfaces:**
- `kalfa-yalitim kur [--izin-ver <dizin>]...`: önce bugünkü `kur`, sonra her dizin için bugünkü `izin_ver`. Bir dizin reddedilirse kök izinleri kalır, çıkış kodu 3'tür ve reddedilen dizin standart çıktıya yazılır. Argümansız `kur` bugünkü gibidir.
- `Yalitici::okunur_yonetici_gerekli(&self, dizin: &Path) -> bool`, varsayılanı `false`. Windows: dizinde yetenek SID'inin okuma izni yoksa ve geçerli kullanıcı dizinin DACL'ini yazamıyorsa (`WRITE_DAC` ile açılamıyorsa) `true`.
- `Yukseltici`:
  - `async fn kur(&self, izinli: &[PathBuf]) -> Result<Option<u32>, String>`: argümanları `kur --izin-ver …` olarak kurar; her yol tırnaklanır.
  - `async fn izin_ver(&self, dizin: &Path) -> Result<Option<u32>, String>`.
- `Uygulama::yalitimi_onar`: `kur`'a, uygulama ayarındaki (`yalitim.json`) araç zinciri dizinlerinden `okunur_yonetici_gerekli` olanları verir.
- `Uygulama::arac_zinciri_dizini_ekle` (Windows): seçilen dizin `okunur_yonetici_gerekli` ise önce yerel pencere ("Bu dizin bilgisayarın yöneticisine ait. Kalfa'nın komutlarının onu okuyabilmesi için Windows yönetici onayı isteyecek."), sonra UAC ile `izin_ver`. Vazgeçilirse liste değişmez. Kurulu sürüm değilse (Onar'daki gibi) dizin eklenmez ve talimat döner.
- Arayüz: Windows'ta "İzinli araç zinciri dizinleri" kartı "Gelişmiş" bölümüne taşınır; açıklaması "Bilgisayarınızda kurulu araçlar (Python, Node, Rust, Visual Studio). Kalfa kendi Python ve Node'unu getirir; bunlara yalnızca başka bir sürüm gerekirse ihtiyaç vardır." Linux'ta kart yerinde kalır.

- [x] **Step 1: Başarısız testleri yaz**
  - `kur_izin_ver_argumanlari_ayrisir` (kalfa-yalitim birim testi): `["kur"]`, `["kur", "--izin-ver", "C:\\a", "--izin-ver", "C:\\b c"]`; `--izin-ver`'siz fazladan argüman reddedilir.
  - `okunur_yonetici_gerekli_kendi_dizininde_false` (sandbox, Windows): geliştiricinin geçici dizini için `false`; `C:\Windows\System32` için `true`.
  - `onar_uygulama_ayarindaki_yonetici_dizinlerini_kur_a_verir` (uygulama, sahte `Yukseltici` ve sahte yalıtıcı): ayarda iki dizin, biri yönetici gerektiriyor; `kur`'a yalnızca o gider.
  - `yonetici_dizini_eklemek_yerel_onay_ve_uac_ister` ve `vazgecilince_liste_degismez` (uygulama).
  - Vitest: Windows'ta kart "Gelişmiş"in içinde; Linux'ta yerinde.
- [x] **Step 2:** Başarısız olduklarını gör.
- [x] **Step 3:** Uygula.
- [x] **Step 4:** Denetimler iki platformda. Gerçek UAC yolu Görev 7'de elle denenir.
- [x] **Step 5:** Commit `uygulama, ui: yöneticiye ait araç dizini UAC ile açılır; Onar araç izinlerini de geri getirir`.

**Sonuç (2026-09-27):**
- **Sapma (Claude, gerekçeli):** `okunur_yonetici_gerekli` yetenek SID'inin yanında `ALL APPLICATION PACKAGES`'ın (`S-1-15-2-1`) okuma iznini de sayar. Program Files'ın çoğunu (ör. `C:\Program Files\Git`) AAP zaten okur; yalnızca yetenek SID'ine bakılsaydı böyle bir dizin gereksiz UAC isterdi. Güvenlik açısından nötr: AAP'nin okuduğu dizini kap zaten okur. Bu yüzden testin `true` örneği `System32` değil, AAP'nin okumadığı ve yöneticiye ait `System32\Tasks`'tır; `System32` `false`'tur.
- `WRITE_DAC` denetimi yalnızca tutamaç açar (`guvenlik::dacl_yazilabilir_mi`); DACL okunamazsa dizin yönetici ister sayılır, yardımcı onu `izin-ver`'in denetimleriyle yeniden doğrular.
- `sistem::yonetici_olarak_calistir` artık argüman listesi alır ve komut satırını `CommandLineToArgvW` kurallarıyla kendisi yazar (`yardimci_komut_satiri`; test: boşluklu yol, sondaki ters bölü). Çağıran tırnaklamayı unutamaz.
- Onar `kur` 3 koduyla biterse (bir dizin reddedildi) "yalıtım kuruldu, bir dizin açılamadı" der; ek test `onar_dizin_reddedilince_basarisiz_ve_neden_doner`. Kurulu sürüm değilken yönetici dizini eklenmez; ek test `yonetici_dizini_kurulu_surum_degilse_talimat_doner`.
- Arayüz: Windows'ta Gelişmiş kartında "İzinli araç zinciri dizinleri" alt bölümü; Linux'ta kart yerinde (Vitest iki testle). Ayarlar'da kartlar kısa pencerede küçülüp içeriklerini kırpıyordu (`flex-shrink`); önizlemede (`onizleme.html#windows`, yeni) görülüp düzeltildi.
- `Yukseltici::kur(&[])` argümansız `kur`'dur; `kalfa-yalitim`'in `yap`'ı `Hata { kod, ileti }` döner (kısmi kurulum 3).
- Doğrulama: Windows 457 + e2e-özellik 89, yalıtım 29/29, uyumluluk 9/9, arayüz 230 test (check 0/0, build temiz); WSL 408 + 89, `kalfatest` ile yalıtım 17/17, uyumluluk 6/6. Windows uçtan uca 25/25.

### Task 6: A15'in gelişmiş yolu (cargo)

**Files:** `crates/kalfa-sandbox/src/windows/yalitici.rs`, `crates/kalfa-sandbox/src/profil.rs`, `crates/kalfa-tools/tests/uyumluluk.rs`, `crates/kalfa-tools/src/kabuk/mod.rs` (hata iletisi).

**Interfaces:** Görev 1'de seçilen yol (b): ağsız komutta `CARGO_HOME` geliştiricinin `~/.cargo`'sudur ve `CARGO_NET_OFFLINE=true` verilir; `~/.cargo` yetenek SID'ine salt okunur açılır. `credentials.toml`, `credentials`, `config.toml` ve `config` varsa mirası kesilerek gizlenir; bu denetim her yalıtımlı başlatmadan önce yenilenir (sonradan eklenen dosya da gizlenir). `config.toml` okunamazken cargo'nun davranışı testte ölçülür; derleme bozuluyorsa dur ve kullanıcıya sun.
- Yol yalnızca geliştiricinin `~/.cargo\bin`'i okunur dizinlerdeyken (bugünkü otomatik algılama) ve komut ağsızken kullanılır. Ağlı komutta bugünkü özel, boş registry sürer.
- Eksik crate'te cargo'nun hatası modele şu ekle döner: "Bağımlılık bu bilgisayarda indirilmemiş. Yalıtım içinde crates.io'dan paket açılamıyor (Windows sınırı). Geliştiricinin yalıtım dışında `cargo fetch` çalıştırması gerekiyor." Ek yalnızca Windows'ta, yalıtımlı ve çıkış kodu sıfır olmayan bir cargo komutunda, çıktıda `failed to` ve `registry` geçerken eklenir.

- [x] **Step 1: Başarısız testleri yaz**
  - `cargo_gelistirici_registry_ile_agsiz_derler` (tools `uyumluluk.rs`, Windows): bugünkü `cargo_agsiz_onceden_indirilmis_crate_ile_derler_ve_test_eder`'in Windows sürümü, `CARGO_HOME`'u geliştiricininki değil, seçilen yol.
  - `cargo_kimlik_bilgileri_kapta_okunamaz` (sandbox `yalitim/windows.rs`): geçici bir "geliştirici evi"nde `.cargo\credentials.toml` ve `.cargo\config.toml`; kapta hem doğrudan yoldan hem `CARGO_HOME` üzerinden okuma "Erişim engellendi" verir.
  - `cargo_gelistirici_dizinine_yazamaz` (sandbox): kapta `registry` altına dosya yazma reddedilir.
  - `eksik_crate_hatasina_talimat_eklenir` (tools, sahte çıktı).
- [x] **Step 2:** Başarısız olduklarını gör.
- [x] **Step 3:** Uygula.
- [x] **Step 4:** Denetimler iki platformda.
- [x] **Step 5:** Commit `sandbox: Windows'ta ağsız cargo geliştiricinin registry'sini salt okunur kullanır`.

**Sonuç (2026-09-27):**
- **Sapma 1 (Claude, gerekçeli): `.cargo`'nun tamamı değil, yalnızca `registry` ve `git` okunur.** Kök izin almaz; `credentials.toml`, `config.toml` ve oraya sonradan konacak her dosya kapta okunamaz. Kullanıcının dosyalarının mirası kesilmez, her başlatmada yenileme gerekmez (izin listesi). cargo için kökü okumak gerekmedi: gerçek AppContainer'da ağsız `cargo test --offline --locked` geliştiricinin `hex 0.4.3`'üyle geçti. Ortak ACE kaydındaki bugünkü okuma izinleri ve geri alma aynen kullanılır (`WindowsAyari::okunur`).
- `config.toml` varken ölçüldü (geliştiricinin `.cargo`'suna yalnız yorum satırı içeren geçici bir dosya konup silindi): cargo okunamayan dosyayı yok sayıp derliyor. Bedeli: kullanıcının `~/.cargo\config.toml` ayarları kapta uygulanmaz (§9.6'ya G7'de).
- **Sapma 2: ek iletinin koşulu cargo'nun gerçek iletileridir.** Plandaki "`failed to` ve `registry`" ağsız eksik crate'te tutmuyor; cargo 1.98 "no matching package named … (offline)" diyor. Koşul: Windows, yalıtımlı, `cargo` komutu (program adı ya da betikte kelime), çıkış sıfır değil ve çıktıda "no matching package named", "failed to download" ya da (ağlıda A15) "failed to unpack package". Ek `--- Kalfa ---` başlığıyla çıktının sonuna yazılır; ağlı A15 testi eki gerçek yolda da denetler.
- Sınır testi tek testte: `cargo_kimlik_bilgileri_kapta_okunamaz` okumayı (dört dosya, iki yol), `registry`'ye yazamamayı ve ağlı komutta özel registry'yi birlikte sınar. Ağsız derleme testi Linux'a özel değil, iki platformda koşar.
- Doğrulama (hızlı düzen, kullanıcı kararı): Windows yalıtım 30/30, uyumluluk 10/10, sandbox 71, tools 55; WSL sandbox + tools 85, clippy ve fmt temiz. Tam denetim G7'de.

### Task 7: Belgeler ve kabul

**Files:** `docs/GELISTIRME_DURUMU.md`, aşama 2 tasarımı (§7.4 cargo, §9.6 A16 sınırları, §9.8 MinGit kararı ve kaldırmada veri, §10.4, §12.3 Windows, §14 A15/A16 sonuçları, §15), `crates/kalfa/kurucu/README.md`, bu plan.

- [x] **Step 1:** Belgeleri güncelle; doğrulama tablosu (Windows, WSL).
  - **Bulunan açık (Claude, G7'de):** Step 2.4'ün varsayımı tutmuyordu. Yükseltmede yeni kurulumun `kur`'u kök izinlerini geri kurar; yalıtım "Hazır" görünür ve Onar düğmesi çıkmaz, yalnızca araç zinciri dizinlerinin izni gitmiştir. Düzeltme: `arac_zinciri_onarim_gerekli` komutu (kurulu sürümde, kayıtlı ve var olan bir dizin kapta okunamıyorsa `true`); Windows'ta Ayarlar > Gelişmiş bunu söyler ve Onar'ı gösterir. Onar ve bu denetim silinmiş dizini yardımcıya vermez. "Hazır"dayken başarısız Onar'ın iletisi de artık görünür. Testler: `arac_zinciri_onarim_gerekli_yalnizca_okunamayan_kayitli_dizinde`, Vitest "yükseltmeden sonra okunamayan dizin için Gelişmiş'te Onar çıkar".
- [x] **Step 2 (kullanıcı, UAC; yönetici PowerShell, `crates\kalfa\kurucu` içinde):**
  1. Kurun: `.\kabul.ps1 -KurucuYolu ..\..\..\target\release\bundle\nsis\Kalfa_0.1.0_x64-setup.exe -EskiYardimci ..\..\..\target\release\kalfa-yalitim.exe`. Betik geliştirme kurulumunu kaldırır, kurucuyu açar ve eski `izin-ver` dizinlerini (`nodejs`, `Python314`) geri yükler.
  2. Kalfa'yı Başlat menüsünden açın; bir proje klasöründe modelden `where python`, `python --version`, `python -m venv .venv`, `node --version`, `npm --version` ve `git --version` çalıştırmasını isteyin. Onay penceresinde "Yalıtımlı" görünmeli; `where python` önce `C:\Program Files\Kalfa\araclar\python`'ı göstermeli.
  3. UAC ile ekleme: önce `& "$env:ProgramFiles\Kalfa\kalfa-yalitim.exe" izin-geri-al "C:\Program Files\nodejs"`; sonra Ayarlar → Gelişmiş → "Dizin ekle" → `C:\Program Files\nodejs`. Kalfa'nın penceresi ve UAC gelmeli, dizin listeye girmeli.
  4. Yükseltme: kurucuyu yeniden çalıştırın (önce eski sürümü kaldırır). Ayarlar'da yalıtım "Hazır", Gelişmiş'te "Listedeki bir dizin artık … okunamıyor" ve "Onar" görünmeli. Onar → UAC → uyarı kalkmalı.
  5. Windows Ayarlar'dan kaldırın. `C:\Program Files\Kalfa` (araçlar dahil) ve `C:\ProgramData\Kalfa\yalitim\kurulum.json` kalmamalı; gerekirse `.\geri-al.ps1`.
  6. Geliştirme ortamını geri kurun: `..\..\..\target\release\kalfa-yalitim.exe kur`, sonra aynı yardımcıyla `izin-ver "C:\Program Files\nodejs"` ve `izin-ver C:\Python314`.
- [x] **Step 3:** Commit `docs: 2d-3 tamamlandı`.

**Kabul notu (kullanıcı, 2026-09-28; Claude oturum kaydından, ACL'lerden ve kayıtlardan doğruladı):**
1. `kabul.ps1` kurdu; eski `izin-ver` dizinleri geri yüklendi.
2. Kalfa'da model sekiz komutu yalıtımda çalıştırdı: `where.exe python` → `C:\Program Files\Kalfa\araclar\python\python.exe`; `where.exe node` önce Kalfa'nınkini, sonra `C:\Program Files\nodejs`'i gösterdi; Python 3.14.7, `venv` (9 sn, bilinen açılış uyarısıyla), venv'in `sys.prefix`'i, Node 24.21.0, npm 11.19.0, git 2.55.0. **Bulgu:** model sekizini aynı turda isteyince ilki çalıştı, yedisi "Dosya onaydan sonra değişti" ile düştü (ilk hazırlıkla aynı turdaki paralel kabuk çağrıları; 2d-3'ten önceki davranış). Tek tek hepsi geçti. Ayrı işe ayrıldı.
3. `izin-geri-al` sonrası Ayarlar → Gelişmiş → "Dizin ekle": Kalfa'nın penceresi ve UAC geldi; nodejs'e yetenek izni ve `kurulum.json`'a kayıt geri geldi. **Bulgu:** listede yol `\\?\` önekliydi; düzeltildi (Windows'ta öneksiz kayıt, eski kayıt okunurken düzelir).
4. Aynı sürümün kurucusunu yeniden çalıştırmak kaldır-kur yapmadı (izin durdu, uyarı doğru olarak çıkmadı). Yükseltmenin izinlere etkisi `kaldir` + `kur` ile canlandırıldı: nodejs izni ve kaydı gitti, yalıtım "Hazır"; Gelişmiş'te uyarı ve Onar çıktı. Onar (yerel pencere + UAC) nodejs iznini ve kaydını geri getirdi.
5. Windows Ayarlar'dan kaldırma: Program Files'ta Kalfa ve kurtarma kopyası, `kurulum.json`, kaldırma kaydı ve `C:\`, `D:\`, `C:\Users`, nodejs, Python314'teki yetenek izinleri kalmadı; oturum kayıtları durdu. Hazırlanmış çalışma alanının (`Kalfa Projeleri`) profili ve izni tasarım gereği kaldı (§9.8; hesap başına, Ayarlar'dan kaldırılır).
6. Geliştirme ortamı geri kuruldu (kök izinleri, nodejs, Python314).

---

## Self-review

- **§15 (2d-3):**
  - Kurucuya Python ve Node → G3; kabın `PATH`'i → G4.
  - MinGit → dağıtılmaz (Kararlar); Git for Windows → G1 Step 3, G4 testi.
  - Bilgisayardaki araçlar gelişmiş → G5 (arayüz ve UAC). Rust ve MSVC otomatik algılaması sürer (Kararlar).
  - A15'in gelişmiş yolu → G1 Step 4, G6.
  - A16 ölçülür → G1; sınırlar §9.6'ya G7'de.
- **2d-2'den devralınanlar:**
  - Yükseltmede araç zinciri izinleri → G5 (Onar geri getirir), kabulü G7 Step 2.4.
  - Kaldırmada oturum kayıtları → Kararlar; belgelere G7.
  - Sınır matrisi (dış kaynak incelemesi) → G2.
- **Tutarlılık:** `kip_uyusmasi` (G2) iki arka uçta; `kurulu_arac_zinciri` (G4) `kurulu_yardimci`'nin denetimini paylaşır; `Yukseltici::kur(izinli)` (G5) Onar'ın tek çağrısıdır; `okunur_yonetici_gerekli` (G5) hem Onar'da hem "Ekle"de kullanılır.
- **Açık risk:** Görev 1'in ölçütleri karşılanmazsa (örneğin Python'un `venv`'i kapta çalışmazsa) sonraki görevler durur ve kullanıcıya sunulur.
