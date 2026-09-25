# Kalfa 2b — Linux Yalıtımı Uygulama Planı

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Linux'ta kabuk komutlarının gerçek yalıtımda çalışması. bubblewrap ile ev dizini, diğer çalışma alanları ve sistemin geri kalanı görünmez; çalışma alanı dışına yazılamaz; gizli dosyalar okunamaz; git kancaları korunur; ağ varsayılan olarak kapalıdır; seccomp tehlikeli sistem çağrılarını engeller. Yalıtım yoksa Kalfa bunu gizlemez: komut yalıtımsız ve yerel onaylı çalışır. Windows'ta yalıtım 2d'dedir.

**Architecture:**
- **`kalfa-sandbox`:**
  - Profil ve tarama (platformdan bağımsız): çalışma alanı bilgisi (`AlanBilgisi`), çalışma alanına özel ev/geçici/önbellek dizinleri ve en küçük araç yapılandırması (`OzelDizinler`), gizli dosya taraması (hard link'ler dahil), profil özeti.
  - Linux arka ucu (`linux/`): `LinuxYalitici` açılışta yalıtımı algılar ve durumu önbelleğe alır. Hazırsa komutu bwrap'ın içinde, `prlimit` ve iki seccomp filtresiyle çalıştırır; değilse yalıtımsız çalıştırır ve durum bunu söyler. İstek onaylandığı kipte çalışır.
- **Çekirdek:** git yerleşimi: korumalı yolların mutlak hâli ve worktree'nin dışarıdaki git dizinleri.
- **Araç:** yalıtım hazırsa `kabuk` her çalıştırmadan hemen önce çalışma alanını tarar ve git yerleşimini çözer; tarife profil özetini koyar (mühre girer).
- **Uygulama:** platformun yalıtıcısını kurar; araç zinciri dizinlerini (`yalitim.json`) tutar; durum değişimini ana pencerenin verdiği kanala yazar; yeniden algılama ve dizin ekleme/kaldırma komutları.
- **Arayüz:** menüde Ayarlar; yalıtım durumu, talimat, "Yeniden dene" ve araç zinciri dizinleri. Durum çubuğundaki gösterge Ayarlar'ı açar.
- **Uçtan uca:** Linux'ta kabuk testleri gerçek yalıtımda koşar; yalıtıma özel bir test eklenir.

**Tech Stack:** Rust 1.98 · bubblewrap 0.9 · seccompiler 0.5 · libc 0.2 · ignore 0.4 · sha2 0.11 · Tauri 2.11 · Svelte 5 · Vitest · WebdriverIO 9.32

**Spec:** `docs/superpowers/specs/2026-09-24-kalfa-2-kabuk-yalitim-design.md` (aşama 2 tasarımı): §3.2, §6.1, §6.3, §7, §8, §10.3–§10.5, §11, §12.2, §12.3, §14. Tasarım belgesinin 2b ayrıntılarıyla güncellenmiş hâli plan commit'indedir.

**Kod hakkında:** Bu plandaki kodun tamamı plan yazılırken ayrı bir kopyada derlendi ve doğrulandı (2026-09-25).

- **Windows 11 (WebView2 153):**
  - `cargo test --workspace --locked`: 360 test geçti.
  - `e2e` özellikli testler (66) geçti; normal ve `e2e` derlemesinde clippy ve fmt temiz.
  - `ui/`'da check (0/0), test (20 dosya, 218 test) ve build temiz.
  - Uçtan uca: iki koşuda 24 test geçti; gecikme p95 (düzenli / yığın / paralel) 24,4–24,6 / 25,6–26,1 / 24,2–24,6 ms.
- **Linux (Ubuntu 24.04, WSL2, bubblewrap 0.9.0, çekirdek 6.18, Xvfb, WebKitGTK 2.52):**
  - `cargo test --workspace --locked`: 369 test geçti. Aradaki fark Linux'a ve Windows'a özel testler.
  - `e2e` özellikli testler (66), clippy (üç derleme: normal, `e2e`, `yalitim-testleri`) ve fmt aynı; `ui/` aynı.
  - Sıradan kullanıcı `kalfatest` ile: `yalitim` testleri (17) ve `uyumluluk` testleri (4) geçti.
  - Uçtan uca: iki koşuda 25 test geçti (birleştirme kapalı, Görev 7); gecikme p95 35,9–37,5 / 33,0–35,5 / 36,8–37,6 ms.
- **Plan yazılırken ölçülenler ve bulunanlar:**
  - **Süreç sayısı sınırı (A1):** sınır bwrap'a dışarıda verilince ad alanının üst sınırı olur ve kullanıcının bütün süreçleri sayılır; komut hiç başlamaz. İçeride (`prlimit`) konunca yalnızca içerideki süreçler sayılıyor.
  - **Cargo registry'si (A2):** salt okunur registry'yle ağsız derleme çalışıyor. Yeni crate salt okunur registry'ye inemediği için ağlı çalıştırmada kullanıcının registry'si bağlanmıyor.
  - **Tarama (A3):** 100 000 dosyada 60–63 ms. `/sys` hiçbir araca gerekmedi (A4).
  - **Rustup:** vekil her çalışmada `~/.rustup/update-hashes`'i istiyor; `~/.rustup`'ın tamamı salt okunur açıldı.
  - **Konuşmanın alta yapışması (1c'den kalan hata):** Linux'ta kabuk testleri yalıtımla koşunca yeni onay kartı görünüm alanının altında kalıyordu. Konuşma, kendi kaydırma olayında, odağın geri dönmesinde ya da bir düğmeye tıklanınca en alttan kopuyordu. Düzeltmeden önce kabuk testlerinin çıktı seli testi altı koşunun üçünde düştü; düzeltmeden sonra on altı koşunun hepsinde geçti.
  - **Linux gecikme ölçümü:** alta yapışma düzelince akan metin görünür oldu ve Xvfb'de p95 56–64 ms'ye çıktı. Karenin çizimi ~5 ms; bekleme GPU'suz Xvfb'de yazılımla (llvmpipe) çalışan GL birleştiricisinde. Kaydırmayı seyreltmek, `will-change` ya da `content-visibility`'yi kaldırmak değiştirmedi. Linux uçtan uca testleri `WEBKIT_DISABLE_COMPOSITING_MODE=1` ile koşar (Görev 7); ürün kodu değişmez.

Kodu olduğu gibi kullan. Bir şey yine de derlenmezse testin sınadığı davranışı değiştirmeden düzelt; davranış değişmesi gerekiyorsa dur ve PR'a yorum yaz.

**Bu planın kapsamı dışında:**
- Windows prototipi (2c) ve arka ucu (2d). Windows'ta yalıtıcı 2a'daki gibi yalıtımsızdır.
- Ayarlar'da hazırlanmış çalışma alanları, önbellek silme, ek ortam değişkenleri ve Windows düğmeleri (2d).
- AppArmor profili ve dağıtım paketleri, Linux dağıtım matrisi (dağıtım aşaması).
- "Bu oturumda izin ver" kısayolu (aşama 3).
- Elle kontroller ve sürekli tümleştirme.

## PR ile yürütme

Kullanıcı kararı (2026-09-23): kodu Codex yazar, Claude inceler.

- **Dal ve PR:** çalışma dalı `2b/yalitim`, PR'ın hedefi `kalfa-asama-1`.
- **Hazır gelenler (plan commit'i):**
  - Plan ve tasarım belgesinin 2b güncellemesi.
  - Kök `Cargo.toml`: `seccompiler = "0.5.0"`.
  - `crates/kalfa-sandbox/Cargo.toml`: `hex`, `ignore`, `sha2`; Linux'ta `seccompiler` ve `tempfile`; `yalitim-testleri` özelliği.
  - `crates/kalfa-tools/Cargo.toml`: `yalitim-testleri` özelliği.
  - `Cargo.lock`.

  Bu dosyaları değiştirme.
- **Ortam:**
  - **Araç sürümleri:** Rust 1.98.1 (`rust-toolchain.toml`); Node `^22.22.2 || >=24.15.0`, npm 10 ya da üstü.
  - **Ağ:** `npm ci` için ağ erişimi gerekir. `seccompiler` kilit dosyasındadır; `cargo` onu bir kez indirebilir. Başka crate eklenmez.
  - **Linux'a özel kod:** `crates/kalfa-sandbox/src/linux/` ve yalıtım testleri `#[cfg(target_os = "linux")]` arkasındadır; Windows'ta derlenmez. Windows'ta çalışıyorsan Görev 2'nin Linux derlemesini ve testlerini koşturamazsın: kodu plandaki gibi yaz, Windows denetimlerini koştur ve bunu PR'a yaz. Claude Linux derlemesini, yalıtım ve uyumluluk testlerini incelemede WSL2'de sıradan bir kullanıcıyla koşturur.
  - **Uçtan uca testler için** (Görev 7): `tauri-driver` PATH'te; Windows'ta WebView2 sürümüyle aynı `msedgedriver` (PATH'te değilse `KALFA_E2E_EDGEDRIVER`); Linux'ta `webkit2gtk-driver`, `xvfb` ve `bubblewrap`. `node` PATH'te olmalıdır. Sürücüleri kendin indirme ya da kurma; ortamında yoksa PR'a yaz.
  - **Biçim:** biçimlendirici (Prettier ve benzeri) projede yok; koşturma. Kod plandaki biçimiyle yazılır. Rust için `cargo fmt` kullanılır.
- **Plandaki değişiklik biçimi:**
  - "(tam içerik)" başlıklı dosya olduğu gibi yazılır: yeni dosya oluşturulur, var olanın içeriği değiştirilir.
  - "şu bloğu … şununla değiştir" çiftlerinde eski blok dosyada tam olarak bir kez geçer. Çiftler dosyadaki sırasıyla verilmiştir; sırayla uygula. Bir dosya birden çok görevde değişebilir; her görevin çiftleri bir önceki görevin sonundaki hâline uygulanır.
- **Sıra:** görevler sırayla. Her görev planda yazan mesajla kendi commit'ini alır; her commit'ten sonra push'la. Geçmiş yeniden yazılmaz, force-push yapılmaz.
- **Soru ve sapma:** plandan sapman gerekirse dur ve PR'a yorum yaz; kendi başına karar verme.
- **İnceleme:** Claude PR'a yorum yazar; Windows'ta ve Linux'ta (WSL2) doğrular, yalıtım testlerini sıradan kullanıcıyla ve uçtan uca testleri iki platformda koşturur; inceleme temizse birleştirir.

## Global Constraints

- **Araçlar:** Rust 1.98.1; `cargo` komutları `--locked` ile. Node `^22.22.2 || >=24.15.0`; kurulum `npm ci`.
- **Bağımlılık:** yeni crate ya da npm paketi eklenmez; `seccompiler` ve diğerleri plan commit'indedir.
- **Güvenlik sınırları (testler sınar):**
  - **bwrap:** yalnızca `/usr/bin/bwrap` ya da `/usr/local/bin/bwrap` kullanılır; `PATH`'ten aranmaz.
  - **Görünürlük:** yalıtımda sistem yolları ve araç zinciri salt okunurdur; ev dizini, diğer çalışma alanları, `/home`, `/root`, `/mnt`, `/srv`, `/opt` ve `/sys` görünmez (kullanıcının eklediği araç zinciri dizini hariç). Çalışma alanı dışına yazılamaz; boş kök salt okunurdur.
  - **Gizli dosyalar:** sınıflandırıcının gizli saydığı dosyalar ve onların hard link'leri okunamaz; gizli dizinler boş ve salt okunur görünür.
  - **Git:** `.git/hooks`, `.git/config` (worktree'de ortak git dizinindekiler) ve `core.hooksPath` hedefi yazılamaz, silinemez, üzerine taşınamaz; eksikse boş olarak oluşturulur.
  - **Ağ ve soketler:** ağsız profilde ağ ad alanı ayrıdır; `AF_INET`, `AF_INET6`, `AF_UNIX`, `AF_PACKET` ve `NETLINK_ROUTE` dışındaki `AF_NETLINK` soketleri iki profilde de engellenir (ağlı profilde `AF_INET`/`AF_INET6` serbest). `socketpair` serbesttir.
  - **seccomp:** `io_uring_*`, `keyctl`, `add_key`, `request_key`, `bpf`, `perf_event_open`, `userfaultfd`, `ptrace` ve ad alanı bayraklı `unshare`/`clone` `EPERM`; `clone3` ve x86_64'te x32 çağrıları `ENOSYS`.
  - **Süreç ağacı:** iptal, zaman aşımı, komutun bitişi ve Kalfa'nın ölümü bütün ağacı sonlandırır; `setsid` ile kaçan torun da ölür. Süreç sayısı sınırı (256) yalıtımın içinde konur.
  - **Kip:** istek onaylandığı kipte çalışır: yalıtımlı onaylanan komut yalıtım yoksa, yalıtımsız onaylanan komut yalıtım varsa başlamaz.
  - **Araç zinciri dizini:** yalnızca yerel klasör seçiciyle eklenir; arayüz yol göndermez. Ev dizinini ya da Kalfa'nın veri dizinini kapsayan, veri dizininin içindeki dizin eklenemez.
  - **Durum iletimi:** yalıtım durumu ana pencereye `yalitimi_dinle` kanalıyla gider; pencerelere Tauri'nin çekirdek izinleri verilmez.
- **`unsafe`:** yalnızca `crates/kalfa-sandbox/src/agac.rs` ve `crates/kalfa-sandbox/src/linux/mod.rs`'de, `SAFETY` açıklamasıyla.
- **CSP (1c-3a):** şablonda sabit `style="…"` yok. `ui/src/csp.test.ts` yeni dosyaları da tarar.
- **Dil:**
  - Kod ve testlerdeki adlar ile açıklamalar Türkçedir.
  - Kullanıcıya görünen metinler `ui/src/dil/tr.ts`'tedir ya da Rust'ta sabittir; plandaki gibi yazılır. Yalıtım talimatları arayüzde görünür ve "siz" diliyle yazılır.
  - Rust'ta `unwrap()` ve `expect()` yalnızca testlerde ve `test_destek.rs`'de kullanılır.
- **Her görevin sonunda:**
  - Rust tarafı: `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings` ve `cargo fmt --all -- --check` geçer.
  - `kalfa-uygulama`'ya ya da `kalfa`'ya dokunan görevlerde ayrıca `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked` ve `cargo clippy -p kalfa -p kalfa-uygulama --features e2e --all-targets --locked -- -D warnings` geçer.
  - `kalfa-sandbox`'a ya da `kalfa-tools`'a dokunan görevlerde ayrıca `cargo clippy -p kalfa-sandbox -p kalfa-tools --features "kalfa-sandbox/yalitim-testleri kalfa-tools/yalitim-testleri" --all-targets --locked -- -D warnings` geçer (Linux'ta anlamlıdır).
  - Arayüze dokunan görevlerde `ui/` içinde `npm run check` (0 hata, 0 uyarı), `npm test` ve `npm run build` (paket denetimi dahil) geçer.
- **Commit:** mesajlar Türkçe (`sandbox: …`, `çekirdek: …`, `araçlar: …`, `uygulama: …`, `ui: …`, `e2e: …`, `docs: …`). Codex'in commit'lerine Claude satırı eklenmez.
- **Dokunulmayanlar:**
  - Plan commit'indeki dosyalar.
  - `design/`, `.env`, `Screenshot*.png`, kök dizindeki `tasks.png`.

## Dosya Yapısı

```
crates/kalfa-sandbox/
  src/profil.rs       AlanBilgisi, Korumali, OzelDizinler, GitKimligi, alan_kimligi, ozet    yeni
  src/tarama.rs       Tarama, tara: gizli dosya taraması, hard link'ler                      yeni
  src/linux/mod.rs    LinuxYalitici, LinuxAyari, Algilayici, BwrapAlgilayici, talimatlar     yeni
  src/linux/bwrap.rs  bwrap seçenekleri (--args biçimi)                                      yeni
  src/linux/seccomp.rs engel ve ENOSYS filtreleri                                            yeni
  src/lib.rs, surec.rs, program.rs                                                           (değişir)
  tests/yalitim.rs    gerçek yalıtım testleri (yalitim-testleri, Linux)                      yeni
  tests/yalitimsiz.rs                                                                        (değişir)
crates/kalfa-core/
  src/git_koruma.rs   GitYerlesimi, KorumaliYol, git_yerlesimi                               (değişir)
  tests/izin.rs                                                                              (değişir)
crates/kalfa-tools/
  src/kabuk/mod.rs    alan bilgisi, profil özeti, yeniden algılama                           (değişir)
  tests/kabuk.rs                                                                             (değişir)
  tests/uyumluluk.rs, tests/ornekler/{rust,node}/  araç zinciri uyumluluğu (Linux)           yeni
crates/kalfa-uygulama/
  src/yalitim.rs      YalitimKaynagi, AracZinciriKaydi, komutlar                             yeni
  src/arayuz.rs, calisma_alanlari.rs, e2e/mod.rs, lib.rs, oturumlar.rs, sozlesme.rs,
  test_destek.rs, uygulama.rs                                                                (değişir)
  tests/yalitim.rs                                                                           yeni
  tests/e2e_parcalari.rs, sart_akisi.rs                                                      (değişir)
crates/kalfa/
  src/iletisim.rs, kanal.rs, komut_adlari.rs, komutlar.rs, lib.rs,
  capabilities/ana-pencere.json                                                              (değişir)
ui/src/
  ana/Ayarlar.svelte, ana/ayarlar.test.ts                                                    yeni
  ana/{Ana,Menu,DurumCubugu}.svelte, ana/anaDurum.svelte.ts, bilesenler/ikonlar.ts,
  dil/tr.ts, ipc/{ana,tipler}.ts, onizleme/senaryo.ts, sahte/arka.ts                         (değişir)
e2e/testler/05-kabuk.test.mjs, e2e/yardimci/kalfa.mjs                                        (değişir)
README.md, docs/GELISTIRME_DURUMU.md                                                         (değişir)
```

---

### Task 1: `kalfa-sandbox`: yalıtım profili ve gizli dosya taraması

**Files:**
- Create: `crates/kalfa-sandbox/src/{profil,tarama}.rs`
- Modify: `crates/kalfa-sandbox/src/{lib,surec,program}.rs`, `crates/kalfa-tools/src/kabuk/mod.rs` (yalnızca `alan: None`)
- Test: `crates/kalfa-sandbox/tests/yalitimsiz.rs` (değişir); birim testleri `profil.rs` ve `tarama.rs`'te

**Interfaces:**
- Consumes: 2a'nın `kalfa-sandbox`'ı.
- Produces (`kalfa_sandbox`):
  - `AlanBilgisi { kok: PathBuf, gizli: Tarama, korumali: Vec<Korumali>, git_dizinleri: Vec<PathBuf> }`, `Korumali { yol: PathBuf, dizin: bool }`.
  - `alan_kimligi(&Path) -> String` (kanonik yolun SHA-256'sından 16 onaltılık karakter; `CalismaAlaniKaydi::kimlik` ile aynı).
  - `OzelDizinler { kok, ev, gecici, onbellek }`: `yol(yalitim_dizini, kimlik)`, `cargo()`, `npm()`, `pip()`, `gitconfig()`, `npmrc()`, `hazirla(&GitKimligi) -> io::Result<()>`.
  - `GitKimligi { ad: Option<String>, eposta: Option<String> }`, `GitKimligi::oku()`.
  - `profil::ozet(arka_uc: &str, okunur: &[PathBuf], ag: bool) -> String`.
  - `tarama::{Tarama { dosyalar, dizinler }, tara(kok: &Path, gizli_mi: &(dyn Fn(&Path, bool) -> bool + Sync)) -> Tarama}`, `Tarama::sayi()`.
  - `CalistirmaIstegi.alan: Option<AlanBilgisi>`; `BaslatmaHatasi::Yalitim(String)`.
  - `Yalitici`'nın varsayılanlı yeni yöntemleri: `profil_ozeti(&self, ag: bool) -> Option<String>`, `yeniden_algila(&self) -> YalitimDurumu`, `okunur_dizinleri_ayarla(&self, Vec<PathBuf>)`.
  - Crate içi: `Surec::komutla(tokio::process::Command)`, `program::calistirilabilir`.

**Davranış notları (testler sınar):**
- **Tarama:** bütün çalışma alanı, `.gitignore` kapsamındakiler dahil; sembolik bağlantılar izlenmez ve listelenmez. Gizli dizinin içi listelenmez (dizin örtülür) ama gezilir: içindeki bir dosyanın dışarıdaki hard link'i de gizlidir. Unix'te bağlantı sayısı birden büyük dosyaların (aygıt, inode) çifti karşılaştırılır. Sonuç sıralıdır.
- **Özel dizinler:** `hazirla` `ev/`, `gecici/`, `onbellek/cargo/registry`, `onbellek/npm`, `onbellek/pip`'i oluşturur; `.gitconfig` yalnızca `[user] name/email`'i tırnaklı ve satır sonları atılmış taşır; `.npmrc` boştur.
- **Profil özeti:** okunur yolların sırası özeti değiştirmez; ağ bayrağı ve yol listesi değiştirir.

- [ ] **Step 1: Başarısız testi yaz**

**`crates/kalfa-sandbox/tests/yalitimsiz.rs`**: şu bloğu

```rust
        ortam: ortam::uret(std::env::vars()),
        ag: false,
    }
}
```

şununla değiştir:

```rust
        ortam: ortam::uret(std::env::vars()),
        ag: false,
        alan: None,
    }
}
```

- [ ] **Step 2: Testin başarısız olduğunu gör**

Run: `cargo test -p kalfa-sandbox --locked`
Expected: derleme hatası (`CalistirmaIstegi`'nde `alan` alanı yok).

- [ ] **Step 3: Profili ve taramayı yaz**

**`crates/kalfa-sandbox/src/profil.rs`** (tam içerik):

```rust
//! Yalıtım profilinin işletim sisteminden bağımsız parçaları (aşama 2
//! tasarımı §7): çalışma alanının yalıtıma giren bilgisi, çalışma alanına özel
//! dizinler ve araç yapılandırması, profil özeti.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::tarama::Tarama;

/// Bir çalıştırmanın çalışma alanı bilgisi. Araç her çalıştırmadan hemen önce
/// üretir; yalıtımsız arka uç kullanmaz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlanBilgisi {
    /// Çalışma alanının kanonik kökü; yalıtımda yazılabilir.
    pub kok: PathBuf,
    /// Okunamayacak dosyalar ve dizinler (§7.2).
    pub gizli: Tarama,
    /// Salt okunur kalacak yollar: `.git/hooks`, `.git/config`,
    /// `core.hooksPath` (§7.2).
    pub korumali: Vec<Korumali>,
    /// Çalışma alanı dışındaki git dizinleri (worktree, submodule);
    /// yazılabilir bağlanır (§7.4).
    pub git_dizinleri: Vec<PathBuf>,
}

/// Salt okunur kalacak bir yol. Yol yoksa yalıtım onu boş olarak oluşturur;
/// yoksa komut, örneğin eksik `.git/hooks` dizinine kanca yazabilirdi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Korumali {
    pub yol: PathBuf,
    pub dizin: bool,
}

/// Çalışma alanının kimliği: kanonik yolunun SHA-256'sından 16 onaltılık
/// karakter (§7.3). Çalışma alanı kaydı da aynı kimliği kullanır.
pub fn alan_kimligi(kok: &Path) -> String {
    let kanonik = std::fs::canonicalize(kok).unwrap_or_else(|_| kok.to_path_buf());
    let ozet = Sha256::digest(kanonik.to_string_lossy().as_bytes());
    hex::encode(ozet)[..16].to_string()
}

/// Çalışma alanına özel, yalıtımda yazılabilir dizinler (§7.3). Ortak
/// yazılabilir durum yoktur: her çalışma alanının kendi ev, geçici ve
/// önbellek dizini vardır.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OzelDizinler {
    pub kok: PathBuf,
    pub ev: PathBuf,
    pub gecici: PathBuf,
    pub onbellek: PathBuf,
}

/// Kullanıcının git kimliği; yalıtımdaki `.gitconfig`'e yalnızca bu iki değer
/// girer (§7.4).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitKimligi {
    pub ad: Option<String>,
    pub eposta: Option<String>,
}

impl GitKimligi {
    /// Kullanıcının genel git yapılandırmasından okur. Depo yapılandırması
    /// yalıtımın içinde zaten okunur.
    pub fn oku() -> Self {
        let deger = |anahtar: &str| {
            let cikti = Command::new("git")
                .args(["config", "--global", "--get", anahtar])
                .output()
                .ok()?;
            let metin = String::from_utf8_lossy(&cikti.stdout).trim().to_string();
            (cikti.status.success() && !metin.is_empty()).then_some(metin)
        };
        Self {
            ad: deger("user.name"),
            eposta: deger("user.email"),
        }
    }

    fn gitconfig(&self) -> String {
        let mut metin = String::from("[user]\n");
        for (anahtar, deger) in [("name", &self.ad), ("email", &self.eposta)] {
            if let Some(d) = deger {
                metin.push_str(&format!("\t{anahtar} = {}\n", git_degeri(d)));
            }
        }
        metin
    }
}

/// Değeri git yapılandırmasında tırnaklı yazar; satır sonları atılır.
fn git_degeri(d: &str) -> String {
    let temiz: String = d
        .chars()
        .filter(|c| *c != '\n' && *c != '\r')
        .flat_map(|c| match c {
            '\\' => vec!['\\', '\\'],
            '"' => vec!['\\', '"'],
            c => vec![c],
        })
        .collect();
    format!("\"{temiz}\"")
}

impl OzelDizinler {
    /// `yalitim_dizini/<kimlik>/` altındaki dizinler; oluşturmaz.
    pub fn yol(yalitim_dizini: &Path, kimlik: &str) -> Self {
        let kok = yalitim_dizini.join(kimlik);
        Self {
            ev: kok.join("ev"),
            gecici: kok.join("gecici"),
            onbellek: kok.join("onbellek"),
            kok,
        }
    }

    pub fn cargo(&self) -> PathBuf {
        self.onbellek.join("cargo")
    }

    pub fn npm(&self) -> PathBuf {
        self.onbellek.join("npm")
    }

    pub fn pip(&self) -> PathBuf {
        self.onbellek.join("pip")
    }

    pub fn gitconfig(&self) -> PathBuf {
        self.ev.join(".gitconfig")
    }

    pub fn npmrc(&self) -> PathBuf {
        self.ev.join(".npmrc")
    }

    /// Dizinleri oluşturur ve en küçük araç yapılandırmasını yazar (§7.4).
    /// Her çalıştırmadan önce çağrılır; kullanıcının git kimliği değiştiyse
    /// yeni hâli yazılır. `.npmrc` boştur: kullanıcınınki token taşıyabilir.
    pub fn hazirla(&self, git: &GitKimligi) -> io::Result<()> {
        for dizin in [
            &self.ev,
            &self.gecici,
            &self.cargo().join("registry"),
            &self.npm(),
            &self.pip(),
        ] {
            std::fs::create_dir_all(dizin)?;
        }
        std::fs::write(self.gitconfig(), git.gitconfig())?;
        std::fs::write(self.npmrc(), "")?;
        Ok(())
    }
}

/// Profil özeti (§7.1): yalıtımın arka ucu, okunur yolların sıralı listesi ve
/// ağ bayrağının SHA-256'sı. Mühre girer: kullanıcı bir dizin eklerse bekleyen
/// onaylar geçersizleşir. Gizli ve korumalı yollar her çalıştırmadan hemen önce
/// tarandığı için özete girmez; onları belirleyen kurallar sabittir.
pub fn ozet(arka_uc: &str, okunur: &[PathBuf], ag: bool) -> String {
    let mut yollar: Vec<String> = okunur.iter().map(|y| y.display().to_string()).collect();
    yollar.sort();
    let mut h = Sha256::new();
    h.update(arka_uc.as_bytes());
    h.update([0]);
    for yol in &yollar {
        h.update(yol.as_bytes());
        h.update([0]);
    }
    h.update(if ag { "ag:acik" } else { "ag:kapali" }.as_bytes());
    hex::encode(h.finalize())[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kimlik_kanonik_yoldan_turer_ve_16_karakterdir() {
        let d = tempfile::tempdir().unwrap();
        let k = alan_kimligi(d.path());
        assert_eq!(k.len(), 16);
        assert!(k.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(k, alan_kimligi(&d.path().join(".")));
    }

    #[test]
    fn ozel_dizinler_hazirlanir_ve_gitconfig_yalnizca_kimligi_tasir() {
        let d = tempfile::tempdir().unwrap();
        let o = OzelDizinler::yol(d.path(), "abc");
        o.hazirla(&GitKimligi {
            ad: Some("Ada \"Kalfa\"".into()),
            eposta: Some("ada@example.com\n[core]".into()),
        })
        .unwrap();
        for dizin in [
            &o.ev,
            &o.gecici,
            &o.cargo().join("registry"),
            &o.npm(),
            &o.pip(),
        ] {
            assert!(dizin.is_dir(), "{} yok", dizin.display());
        }
        let gitconfig = std::fs::read_to_string(o.gitconfig()).unwrap();
        assert_eq!(
            gitconfig,
            "[user]\n\tname = \"Ada \\\"Kalfa\\\"\"\n\temail = \"ada@example.com[core]\"\n"
        );
        assert_eq!(std::fs::read_to_string(o.npmrc()).unwrap(), "");
    }

    #[test]
    fn ozet_sirasiz_listede_ayni_dizin_ve_agda_farkli() {
        let a = [PathBuf::from("/usr"), PathBuf::from("/opt/node")];
        let b = [PathBuf::from("/opt/node"), PathBuf::from("/usr")];
        assert_eq!(ozet("bwrap", &a, false), ozet("bwrap", &b, false));
        assert_ne!(ozet("bwrap", &a, false), ozet("bwrap", &a, true));
        assert_ne!(ozet("bwrap", &a, false), ozet("bwrap", &a[..1], false));
    }
}
```

**`crates/kalfa-sandbox/src/tarama.rs`** (tam içerik):

```rust
//! Gizli dosya taraması (aşama 2 tasarımı §7.2). Her çalıştırmadan hemen önce
//! bütün çalışma alanı taranır; `.gitignore` kapsamındakiler dahil, çünkü
//! `.env` genellikle yok sayılan bir dosyadır. Sembolik bağlantılar izlenmez.
//!
//! Linux'ta bağlantı sayısı birden büyük dosyaların (aygıt, inode) çifti gizli
//! dosyalarınkiyle karşılaştırılır: `.env`'nin hard link'i de gizlidir.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ignore::{WalkBuilder, WalkState};

/// Taramanın sonucu; yollar mutlaktır ve sıralıdır.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tarama {
    /// Okunamayacak dosyalar; gizli bir dizinin içindekiler bu listede yoktur,
    /// dizinin kendisi örtülür.
    pub dosyalar: Vec<PathBuf>,
    /// İçi boş görünecek dizinler.
    pub dizinler: Vec<PathBuf>,
}

impl Tarama {
    pub fn sayi(&self) -> usize {
        self.dosyalar.len() + self.dizinler.len()
    }
}

#[derive(Default)]
struct Toplanan {
    dosyalar: BTreeSet<PathBuf>,
    dizinler: BTreeSet<PathBuf>,
    /// Gizli dosyaların (aygıt, inode) çiftleri; yalnızca bağlantı sayısı
    /// birden büyük olanlar.
    gizli_inode: BTreeSet<(u64, u64)>,
    /// Bağlantı sayısı birden büyük bütün normal dosyalar.
    bagli: Vec<((u64, u64), PathBuf)>,
}

/// Çalışma alanını tarar. `gizli_mi` çalışma alanına göre yolu ve yolun dizin
/// olup olmadığını alır.
pub fn tara(kok: &Path, gizli_mi: &(dyn Fn(&Path, bool) -> bool + Sync)) -> Tarama {
    let toplanan = Mutex::new(Toplanan::default());
    WalkBuilder::new(kok)
        .standard_filters(false)
        .follow_links(false)
        .build_parallel()
        .run(|| {
            let toplanan = &toplanan;
            Box::new(move |giris| {
                let Ok(giris) = giris else {
                    return WalkState::Continue;
                };
                if giris.depth() == 0 {
                    return WalkState::Continue;
                }
                let Some(tur) = giris.file_type() else {
                    return WalkState::Continue;
                };
                let Ok(goreli) = giris.path().strip_prefix(kok) else {
                    return WalkState::Continue;
                };
                // Bağlantı izlenmez; hedefi çalışma alanındaysa kendisi taranır.
                if tur.is_symlink() || !(tur.is_dir() || tur.is_file()) {
                    return WalkState::Continue;
                }
                let gizli = gizli_mi(goreli, tur.is_dir());
                let inode = if tur.is_file() {
                    inode(giris.path())
                } else {
                    None
                };
                let mut t = toplanan.lock().unwrap_or_else(|z| z.into_inner());
                if gizli {
                    if tur.is_dir() {
                        t.dizinler.insert(giris.path().to_path_buf());
                    } else {
                        t.dosyalar.insert(giris.path().to_path_buf());
                    }
                }
                if let Some((cift, bag)) = inode
                    && bag > 1
                {
                    if gizli {
                        t.gizli_inode.insert(cift);
                    }
                    t.bagli.push((cift, giris.path().to_path_buf()));
                }
                // Gizli dizinin içi de gezilir: içindeki bir dosyanın dışarıdaki
                // hard link'i bulunabilsin.
                WalkState::Continue
            })
        });
    let t = toplanan.into_inner().unwrap_or_else(|z| z.into_inner());
    let mut dosyalar = t.dosyalar;
    let gizli_dizin_altinda =
        |yol: &Path| yol.ancestors().skip(1).any(|ata| t.dizinler.contains(ata));
    // Gizli dizindeki dosyaların hard link'leri de gizlidir.
    let mut gizli_inode = t.gizli_inode;
    for (cift, yol) in &t.bagli {
        if gizli_dizin_altinda(yol) {
            gizli_inode.insert(*cift);
        }
    }
    for (cift, yol) in t.bagli {
        if gizli_inode.contains(&cift) {
            dosyalar.insert(yol);
        }
    }
    let dizinler: Vec<PathBuf> = t
        .dizinler
        .iter()
        .filter(|d| !gizli_dizin_altinda(d))
        .cloned()
        .collect();
    Tarama {
        dosyalar: dosyalar
            .into_iter()
            .filter(|d| !gizli_dizin_altinda(d))
            .collect(),
        dizinler,
    }
}

#[cfg(unix)]
fn inode(yol: &Path) -> Option<((u64, u64), u64)> {
    use std::os::unix::fs::MetadataExt;
    let bilgi = std::fs::symlink_metadata(yol).ok()?;
    Some(((bilgi.dev(), bilgi.ino()), bilgi.nlink()))
}

/// Windows'ta gerek yoktur: ACL dosya nesnesindedir, hard link aynı nesneyi
/// paylaşır.
#[cfg(not(unix))]
fn inode(_yol: &Path) -> Option<((u64, u64), u64)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_ya_da_gizli_dizin(yol: &Path, dizin: bool) -> bool {
        let ad = yol.file_name().and_then(|a| a.to_str()).unwrap_or("");
        if dizin {
            ad == "gizli"
        } else {
            ad == ".env" || ad.ends_with(".pem")
        }
    }

    #[test]
    fn gizli_dosya_ve_dizin_bulunur_gizli_dizinin_ici_listelenmez() {
        let d = tempfile::tempdir().unwrap();
        let k = d.path();
        std::fs::create_dir_all(k.join("a/b")).unwrap();
        std::fs::create_dir_all(k.join("gizli/ic")).unwrap();
        std::fs::write(k.join(".env"), "x").unwrap();
        std::fs::write(k.join("a/b/anahtar.pem"), "x").unwrap();
        std::fs::write(k.join("a/b/normal.txt"), "x").unwrap();
        std::fs::write(k.join("gizli/ic/.env"), "x").unwrap();
        // .gitignore kapsamındakiler de taranır.
        std::fs::write(k.join(".gitignore"), ".env\n").unwrap();
        let t = tara(k, &env_ya_da_gizli_dizin);
        assert_eq!(t.dosyalar, vec![k.join(".env"), k.join("a/b/anahtar.pem")]);
        assert_eq!(t.dizinler, vec![k.join("gizli")]);
        assert_eq!(t.sayi(), 3);
    }

    #[cfg(unix)]
    #[test]
    fn hard_link_ve_sembolik_baglanti() {
        let d = tempfile::tempdir().unwrap();
        let k = d.path();
        std::fs::create_dir_all(k.join("gizli")).unwrap();
        std::fs::write(k.join(".env"), "x").unwrap();
        std::fs::hard_link(k.join(".env"), k.join("a.txt")).unwrap();
        std::fs::write(k.join("gizli/sir"), "x").unwrap();
        std::fs::hard_link(k.join("gizli/sir"), k.join("b.txt")).unwrap();
        std::fs::write(k.join("c.txt"), "x").unwrap();
        std::fs::hard_link(k.join("c.txt"), k.join("d.txt")).unwrap();
        // Bağlantı izlenmez ve kendisi listelenmez.
        std::os::unix::fs::symlink(k.join(".env"), k.join("bag.env")).unwrap();
        let t = tara(k, &env_ya_da_gizli_dizin);
        assert_eq!(
            t.dosyalar,
            vec![k.join(".env"), k.join("a.txt"), k.join("b.txt")]
        );
        assert_eq!(t.dizinler, vec![k.join("gizli")]);
    }
}
```

**`crates/kalfa-sandbox/src/lib.rs`**: şu bloğu

```rust
mod agac;
pub mod ortam;
pub mod program;
mod surec;
mod yalitimsiz;
```

şununla değiştir:

```rust
mod agac;
pub mod ortam;
pub mod profil;
pub mod program;
mod surec;
pub mod tarama;
mod yalitimsiz;
```

**`crates/kalfa-sandbox/src/lib.rs`**: şu bloğu

```rust
use serde::Serialize;

pub use surec::{Akis, Bitis, CalismaSonucu, Surec};
pub use yalitimsiz::Yalitimsiz;
```

şununla değiştir:

```rust
use serde::Serialize;

pub use profil::{AlanBilgisi, GitKimligi, Korumali, OzelDizinler, alan_kimligi};
pub use surec::{Akis, Bitis, CalismaSonucu, Surec};
pub use tarama::Tarama;
pub use yalitimsiz::Yalitimsiz;
```

**`crates/kalfa-sandbox/src/lib.rs`**: şu bloğu

```rust
    /// Komut ağa erişmek istiyor. Yalıtımsız arka uç ağı kapatamaz.
    pub ag: bool,
}
```

şununla değiştir:

```rust
    /// Komut ağa erişmek istiyor. Yalıtımsız arka uç ağı kapatamaz.
    pub ag: bool,
    /// Yalıtımlı çalıştırmanın çalışma alanı bilgisi. Verilmişse komut
    /// yalnızca yalıtımda çalışır, verilmemişse yalnızca yalıtımsız: istek
    /// böylece onaylandığı kipte çalışır.
    pub alan: Option<AlanBilgisi>,
}
```

**`crates/kalfa-sandbox/src/lib.rs`**: şu bloğu

```rust
    #[error("Komut başlatılamadı: {0}")]
    GirisCikis(#[from] std::io::Error),
}
```

şununla değiştir:

```rust
    #[error("Komut başlatılamadı: {0}")]
    GirisCikis(#[from] std::io::Error),
    #[error("Yalıtım başlatılamadı: {0}")]
    Yalitim(String),
}
```

**`crates/kalfa-sandbox/src/lib.rs`**: şu bloğu

```rust
pub trait Yalitici: Send + Sync {
    fn durum(&self) -> YalitimDurumu;
    fn baslat(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi>;
}
```

şununla değiştir:

```rust
pub trait Yalitici: Send + Sync {
    fn durum(&self) -> YalitimDurumu;

    /// Yalıtım profilinin özeti (§7.1); mühre girer. Yalıtım yoksa `None`.
    fn profil_ozeti(&self, _ag: bool) -> Option<String> {
        None
    }

    fn baslat(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi>;

    /// Algılamayı yeniden çalıştırır (§8.3): Ayarlar'dan istenince ve
    /// yalıtım başlatılamayınca.
    fn yeniden_algila(&self) -> YalitimDurumu {
        self.durum()
    }

    /// Kullanıcının eklediği araç zinciri dizinleri (§7.1).
    fn okunur_dizinleri_ayarla(&self, _dizinler: Vec<PathBuf>) {}
}
```

**`crates/kalfa-sandbox/src/surec.rs`**: şu bloğu

```rust
            .env_clear()
            .envs(istek.ortam.iter().map(|(a, d)| (a, d)))
            .current_dir(&istek.dizin)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
```

şununla değiştir:

```rust
            .env_clear()
            .envs(istek.ortam.iter().map(|(a, d)| (a, d)))
            .current_dir(&istek.dizin);
        Self::komutla(komut)
    }

    /// Kurulmuş bir komutu (örneğin bwrap) ağacın kökü olarak başlatır:
    /// standart girdi kapalıdır, çıktılar okunur.
    pub(crate) fn komutla(mut komut: Command) -> Result<Self, BaslatmaHatasi> {
        komut
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
```

**`crates/kalfa-sandbox/src/program.rs`**: şu bloğu

```rust
}

fn calistirilabilir(yol: &Path) -> bool {
    let Ok(bilgi) = std::fs::metadata(yol) else {
        return false;
```

şununla değiştir:

```rust
}

pub(crate) fn calistirilabilir(yol: &Path) -> bool {
    let Ok(bilgi) = std::fs::metadata(yol) else {
        return false;
```

Kabuk aracı isteği henüz yalıtımsız kurar:

**`crates/kalfa-tools/src/kabuk/mod.rs`**: şu bloğu

```rust
            ortam: ortam::uret(std::env::vars()),
            ag: istek.ag,
        };
        let surec = self
```

şununla değiştir:

```rust
            ortam: ortam::uret(std::env::vars()),
            ag: istek.ag,
            alan: None,
        };
        let surec = self
```

- [ ] **Step 4: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa-sandbox --locked`
Expected: `profil.rs` ve `tarama.rs`'in birim testleri (Linux'ta hard link testi dahil) ve `tests/yalitimsiz.rs` geçer. Görevin sonundaki genel denetimleri de koştur.

- [ ] **Step 5: Commit**

```bash
git add crates/kalfa-sandbox crates/kalfa-tools/src/kabuk/mod.rs
git commit -m "sandbox: yalıtım profili ve gizli dosya taraması"
```

---

### Task 2: `kalfa-sandbox`: Linux arka ucu (bwrap, seccomp, algılama)

**Files:**
- Create: `crates/kalfa-sandbox/src/linux/{mod,bwrap,seccomp}.rs`, `crates/kalfa-sandbox/tests/yalitim.rs`
- Modify: `crates/kalfa-sandbox/src/lib.rs`

**Interfaces:**
- Consumes: Görev 1.
- Produces (`kalfa_sandbox`):
  - `PlatformAyari { ev: PathBuf, yalitim_dizini: PathBuf, ek_okunur: Vec<PathBuf>, degisince: Box<dyn Fn(&YalitimDurumu) + Send + Sync> }`, `platform_yaliticisi(PlatformAyari) -> Arc<dyn Yalitici>` (Linux'ta `LinuxYalitici`, diğerlerinde `Yalitimsiz`).
  - Linux'ta `linux::{LinuxYalitici, LinuxAyari { ev, yalitim_dizini, ek_okunur, git }, Algilayici, BwrapAlgilayici, bwrap_yolu, ARKA_UC, TALIMAT_BWRAP_YOK, TALIMAT_AD_ALANI}`, `linux::seccomp`.
  - `LinuxYalitici::yeni(LinuxAyari, Box<dyn Algilayici>, Box<dyn Fn(&YalitimDurumu) + Send + Sync>)`.

**Davranış notları (testler sınar):**
- **Algılama:** bwrap sistem yollarında yoksa `BwrapYok`; seccomp filtresi üretilemiyorsa `Desteklenmiyor`; gerçek profille geçici bir çalışma alanında `/usr/bin/true` denenir. Hata metninde "namespace", "uid map", "RTM_NEWADDR" ya da "Operation not permitted" varsa `KullaniciAdAlaniKapali`, yoksa metniyle `Desteklenmiyor`. `yeniden_algila` durum değiştiyse `degisince`'i çağırır.
- **Profil:** okunur yollar `/usr`, `/etc`, `/bin`, `/sbin`, `/lib*` (bağlantıysa `--symlink`), `~/.cargo/bin`, `~/.rustup` ve kullanıcının ekledikleri (yalnızca var olan mutlak yollar). Profil özeti yalıtım hazırken verilir.
- **Ortam:** `HOME` özel `ev/`, `TMPDIR=/tmp`, `CARGO_HOME` özel `onbellek/cargo`, `RUSTUP_HOME` kullanıcının `~/.rustup`'ı, `GIT_CONFIG_GLOBAL`, `NPM_CONFIG_USERCONFIG`, `NPM_CONFIG_CACHE`, `PIP_CACHE_DIR`; `TEMP` ve `TMP` atılır.
- **Registry:** ağsız çalıştırmada ve çalışma alanının kendi registry'si boşken kullanıcının `~/.cargo/registry`'si salt okunur bağlanır; ağlıda bağlanmaz.
- **Program:** yalıtımın `PATH`'inde, dışarıdan ama yalnızca içeride görünen yollarda aranır.
- **Başlatma:** seçenekler ve seccomp programları bellek dosyalarından (`--args`, `--add-seccomp-fd`); komut `/usr/bin/prlimit --nproc=256:256 --` ile başlar (varsa).

- [ ] **Step 1: Yalıtım testlerini yaz**

**`crates/kalfa-sandbox/tests/yalitim.rs`** (tam içerik):

````rust
//! Gerçek yalıtımla testler (aşama 2 tasarımı §12.2, Linux sütunu). Yalıtım
//! yoksa atlanmaz, başarısız olur. Sıradan bir kullanıcıyla koşar:
//!
//! ```text
//! cargo test -p kalfa-sandbox --features yalitim-testleri
//! ```
//!
//! Komutlar `sh` ve `perl` (Debian/Ubuntu'da `perl-base`, her sistemde var)
//! ile yazılır.
#![cfg(all(target_os = "linux", feature = "yalitim-testleri"))]

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use kalfa_sandbox::linux::{self, BwrapAlgilayici, LinuxAyari, LinuxYalitici};
use kalfa_sandbox::{
    Akis, AlanBilgisi, Bitis, CalistirmaIstegi, KomutBicimi, Korumali, Yalitici, YalitimDurumu,
    tarama,
};
use tokio_util::sync::CancellationToken;

const UZUN: Duration = Duration::from_secs(60);

struct Kurgu {
    _gecici: tempfile::TempDir,
    taban: PathBuf,
    /// Sahte ev dizini: araç zinciri ve sırlar.
    ev: PathBuf,
    /// Çalışma alanı.
    kok: PathBuf,
    y: LinuxYalitici,
}

struct Sonuc {
    bitis: Bitis,
    standart: String,
    hata: String,
}

fn yaz(yol: &Path, icerik: &str) {
    if let Some(ust) = yol.parent() {
        std::fs::create_dir_all(ust).unwrap();
    }
    std::fs::write(yol, icerik).unwrap();
}

fn calistirilabilir_yaz(yol: &Path, icerik: &str) {
    use std::os::unix::fs::PermissionsExt;
    yaz(yol, icerik);
    std::fs::set_permissions(yol, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn kurgu() -> Kurgu {
    let gecici = tempfile::tempdir().unwrap();
    let taban = gecici.path().canonicalize().unwrap();
    let ev = taban.join("ev");
    let kok = taban.join("alan");
    std::fs::create_dir_all(&kok).unwrap();
    calistirilabilir_yaz(
        &ev.join(".cargo/bin/arac"),
        "#!/bin/sh\necho arac-calisti\n",
    );
    yaz(&ev.join(".cargo/credentials.toml"), "GIZLI-CARGO\n");
    yaz(&ev.join(".ssh/id_rsa"), "GIZLI-SSH\n");
    let y = LinuxYalitici::yeni(
        LinuxAyari {
            ev: ev.clone(),
            yalitim_dizini: taban.join("veri/yalitim"),
            ek_okunur: Vec::new(),
            git: Default::default(),
        },
        Box::new(BwrapAlgilayici),
        Box::new(|_| {}),
    );
    match y.durum() {
        YalitimDurumu::Hazir { .. } => {}
        d => panic!("yalıtım hazır değil: {d:?}"),
    }
    Kurgu {
        _gecici: gecici,
        taban,
        ev,
        kok,
        y,
    }
}

/// Testlerin gizli kuralı: `.env` ve `*.pem` dosyaları, `sirlar` dizinleri.
fn gizli_mi(yol: &Path, dizin: bool) -> bool {
    let ad = yol.file_name().and_then(|a| a.to_str()).unwrap_or("");
    if dizin {
        ad == "sirlar"
    } else {
        ad == ".env" || ad.ends_with(".pem")
    }
}

/// Git koruma çözücüsünün (kalfa-core) bu testlerdeki karşılığı.
fn korumali(kok: &Path) -> Vec<Korumali> {
    let mut k = Vec::new();
    if kok.join(".git").is_dir() {
        k.push(Korumali {
            yol: kok.join(".git/hooks"),
            dizin: true,
        });
        k.push(Korumali {
            yol: kok.join(".git/config"),
            dizin: false,
        });
    }
    if kok.join(".githooks").is_dir() {
        k.push(Korumali {
            yol: kok.join(".githooks"),
            dizin: true,
        });
    }
    k
}

impl Kurgu {
    fn istek(&self, kok: &Path, komut: KomutBicimi, ag: bool) -> CalistirmaIstegi {
        CalistirmaIstegi {
            komut,
            dizin: kok.to_path_buf(),
            ortam: vec![
                (
                    "PATH".into(),
                    format!("{}:/usr/bin:/bin", self.ev.join(".cargo/bin").display()),
                ),
                ("LANG".into(), "C.UTF-8".into()),
            ],
            ag,
            alan: Some(AlanBilgisi {
                kok: kok.to_path_buf(),
                gizli: tarama::tara(kok, &gizli_mi),
                korumali: korumali(kok),
                git_dizinleri: Vec::new(),
            }),
        }
    }

    async fn calistir_istek(&self, istek: CalistirmaIstegi, iptal: CancellationToken) -> Sonuc {
        let surec = self.y.baslat(&istek).expect("komut başlamalı");
        let mut standart = Vec::new();
        let mut hata = Vec::new();
        let sonuc = surec
            .bekle(UZUN, iptal, |akis, veri| match akis {
                Akis::Standart => standart.extend_from_slice(veri),
                Akis::Hata => hata.extend_from_slice(veri),
            })
            .await;
        Sonuc {
            bitis: sonuc.bitis,
            standart: String::from_utf8_lossy(&standart).into_owned(),
            hata: String::from_utf8_lossy(&hata).into_owned(),
        }
    }

    async fn betik(&self, kok: &Path, metin: &str, ag: bool) -> Sonuc {
        let istek = self.istek(
            kok,
            KomutBicimi::Betik {
                metin: metin.to_string(),
            },
            ag,
        );
        self.calistir_istek(istek, CancellationToken::new()).await
    }
}

/// Satırlardan `ad=değer` biçimindekinin değeri.
fn deger<'a>(metin: &'a str, ad: &str) -> &'a str {
    metin
        .lines()
        .find_map(|s| s.strip_prefix(&format!("{ad}=")))
        .unwrap_or_else(|| panic!("{ad} yok: {metin}"))
}

/// Komut satırında `parca` geçen süreçler (bütün ad alanlarından).
fn surecler(parca: &str) -> Vec<u32> {
    let Ok(dizin) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    dizin
        .filter_map(|g| g.ok())
        .filter_map(|g| g.file_name().to_str()?.parse::<u32>().ok())
        .filter(|pid| {
            std::fs::read(format!("/proc/{pid}/cmdline"))
                .map(|c| {
                    String::from_utf8_lossy(&c)
                        .replace('\0', " ")
                        .contains(parca)
                })
                .unwrap_or(false)
        })
        .collect()
}

fn kalmadi_mi(parca: &str, sure: Duration) -> bool {
    let son = Instant::now() + sure;
    while Instant::now() < son {
        if surecler(parca).is_empty() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

#[tokio::test]
async fn ev_dizinindeki_sirlar_okunamaz_arac_zinciri_calisir() {
    let k = kurgu();
    let ev = k.ev.display();
    let s = k
        .betik(
            &k.kok,
            &format!(
                "arac; cat {ev}/.ssh/id_rsa; cat {ev}/.cargo/credentials.toml; \
                 echo \"cargo=$(ls {ev}/.cargo | tr '\\n' ' ')\"; echo \"ev=$HOME\""
            ),
            false,
        )
        .await;
    assert!(s.standart.contains("arac-calisti"), "{}", s.standart);
    assert!(!s.standart.contains("GIZLI"), "{}", s.standart);
    assert_eq!(deger(&s.standart, "cargo").trim(), "bin");
    assert!(deger(&s.standart, "ev").ends_with("/ev"));
    assert_ne!(deger(&s.standart, "ev"), ev.to_string());
}

#[tokio::test]
async fn secilmeyen_yollar_ve_baska_calisma_alani_gorunmez() {
    let k = kurgu();
    let baska = k.taban.join("baska");
    yaz(&baska.join("dosya"), "BASKA-ICERIK\n");
    let s = k
        .betik(
            &k.kok,
            &format!(
                "for y in /srv /mnt /home /root /opt /media /sys {b}; do \
                 test -e \"$y\" && echo \"gorunur:$y\"; done; cat {b}/dosya; echo son",
                b = baska.display()
            ),
            false,
        )
        .await;
    assert!(s.standart.contains("son"), "{}{}", s.standart, s.hata);
    assert!(!s.standart.contains("gorunur:"), "{}", s.standart);
    assert!(!s.standart.contains("BASKA-ICERIK"));
}

#[tokio::test]
async fn ayni_anda_iki_calisma_alani_birbirini_okuyamaz_ve_yazamaz() {
    let k = kurgu();
    let b = k.taban.join("b");
    yaz(&k.kok.join("dosya"), "A-ICERIK\n");
    yaz(&b.join("dosya"), "B-ICERIK\n");
    let a_betik = format!(
        "cat {b}/dosya; mkdir -p {b} && echo x > {b}/a-yazdi; sleep 1",
        b = b.display()
    );
    let b_betik = format!(
        "cat {a}/dosya; mkdir -p {a} && echo x > {a}/b-yazdi; sleep 1",
        a = k.kok.display()
    );
    let (sa, sb) = tokio::join!(
        k.betik(&k.kok, &a_betik, false),
        k.betik(&b, &b_betik, false)
    );
    assert!(!sa.standart.contains("B-ICERIK"));
    assert!(!sb.standart.contains("A-ICERIK"));
    assert!(!b.join("a-yazdi").exists());
    assert!(!k.kok.join("b-yazdi").exists());
}

#[tokio::test]
async fn calisma_alani_disina_yazilamaz() {
    let k = kurgu();
    let s = k
        .betik(
            &k.kok,
            "echo x > /etc/kalfa-deneme; echo \"etc=$?\"; \
             echo x > /kalfa-deneme; echo \"kok=$?\"; \
             echo x > /tmp/kalfa-gecici; echo \"tmp=$?\"; \
             echo x > icerde.txt; echo \"alan=$?\"",
            false,
        )
        .await;
    assert_ne!(deger(&s.standart, "etc"), "0");
    assert_ne!(deger(&s.standart, "kok"), "0");
    assert_eq!(deger(&s.standart, "tmp"), "0");
    assert_eq!(deger(&s.standart, "alan"), "0");
    assert!(k.kok.join("icerde.txt").exists());
    // Geçici dizin çalışma alanına özeldir; sistemin /tmp'si değildir.
    assert!(!Path::new("/tmp/kalfa-gecici").exists());
}

#[tokio::test]
async fn git_korumali_yollar_yazilamaz_silinemez_uzerine_tasinamaz() {
    let k = kurgu();
    yaz(&k.kok.join(".git/config"), "[core]\n\tbare = false\n");
    yaz(&k.kok.join(".git/hooks/pre-commit.sample"), "#!/bin/sh\n");
    yaz(&k.kok.join(".githooks/.gitkeep"), "");
    let s = k
        .betik(
            &k.kok,
            "echo x > .git/hooks/pre-commit; echo \"kanca=$?\"; \
             rm -f .git/config; echo \"sil=$?\"; \
             echo x > /tmp/c; mv /tmp/c .git/config; echo \"tasi=$?\"; \
             mv .git/hooks .git/h2; echo \"dizin=$?\"; \
             echo x > .githooks/pre-push; echo \"yol=$?\"; \
             echo x > .git/HEAD; echo \"head=$?\"",
            false,
        )
        .await;
    for ad in ["kanca", "sil", "tasi", "dizin", "yol"] {
        assert_ne!(deger(&s.standart, ad), "0", "{ad}: {}", s.standart);
    }
    // Korumalı olmayan git dosyaları yazılabilir.
    assert_eq!(deger(&s.standart, "head"), "0");
    assert_eq!(
        std::fs::read_to_string(k.kok.join(".git/config")).unwrap(),
        "[core]\n\tbare = false\n"
    );
    assert!(!k.kok.join(".git/hooks/pre-commit").exists());
    assert!(!k.kok.join(".githooks/pre-push").exists());
}

#[tokio::test]
async fn eksik_hooks_dizini_olusturulur_ve_yazilamaz() {
    let k = kurgu();
    yaz(&k.kok.join(".git/config"), "[core]\n");
    let s = k
        .betik(
            &k.kok,
            "mkdir -p .git/hooks; echo x > .git/hooks/pre-commit; echo \"kanca=$?\"",
            false,
        )
        .await;
    assert_ne!(deger(&s.standart, "kanca"), "0");
    assert!(k.kok.join(".git/hooks").is_dir());
    assert!(!k.kok.join(".git/hooks/pre-commit").exists());
}

#[tokio::test]
async fn env_ve_hard_linki_ve_gizli_dizin_okunamaz() {
    let k = kurgu();
    yaz(&k.kok.join(".env"), "GIZLI-ENV\n");
    std::fs::hard_link(k.kok.join(".env"), k.kok.join("a.txt")).unwrap();
    yaz(&k.kok.join("sirlar/anahtar"), "GIZLI-DIZIN\n");
    yaz(&k.kok.join("alt/sunucu.pem"), "GIZLI-PEM\n");
    yaz(&k.kok.join("normal.txt"), "ACIK\n");
    let s = k
        .betik(
            &k.kok,
            "cat .env; cat a.txt; cat sirlar/anahtar; cat alt/sunucu.pem; \
             echo \"liste=$(ls sirlar)\"; cat normal.txt; \
             echo x > sirlar/yeni; echo \"yaz=$?\"",
            false,
        )
        .await;
    assert!(!s.standart.contains("GIZLI"), "{}", s.standart);
    assert!(s.standart.contains("ACIK"));
    assert_eq!(deger(&s.standart, "liste"), "");
    assert_ne!(deger(&s.standart, "yaz"), "0");
    // Dosyalar yerinde durur.
    assert_eq!(
        std::fs::read_to_string(k.kok.join(".env")).unwrap(),
        "GIZLI-ENV\n"
    );
}

const SOKETLER: &str = r#"perl -MSocket -e '
for my $a ([PF_INET, "inet"], [PF_INET6, "inet6"], [PF_UNIX, "unix"]) {
  if (socket(my $s, $a->[0], SOCK_STREAM, 0)) { print "$a->[1]=acildi\n" }
  else { print "$a->[1]=$!\n" }
}
if (socketpair(my $x, my $y, AF_UNIX, SOCK_STREAM, PF_UNSPEC)) { print "cift=acildi\n" }
else { print "cift=$!\n" }'"#;

#[tokio::test]
async fn agsiz_profilde_tcp_ve_unix_soketi_acilamaz() {
    let k = kurgu();
    let s = k.betik(&k.kok, SOKETLER, false).await;
    for aile in ["inet", "inet6", "unix"] {
        assert_eq!(
            deger(&s.standart, aile),
            "Operation not permitted",
            "{}{}",
            s.standart,
            s.hata
        );
    }
    assert_eq!(deger(&s.standart, "cift"), "acildi");
}

#[tokio::test]
async fn agli_profilde_tcp_acilir_unix_soketi_acilamaz() {
    let k = kurgu();
    let dinleyici = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = dinleyici.local_addr().unwrap().port();
    let kabul = std::thread::spawn(move || dinleyici.accept().is_ok());
    let s = k
        .betik(
            &k.kok,
            &format!(
                r#"{SOKETLER}; perl -MSocket -e '
socket(my $s, PF_INET, SOCK_STREAM, 0) or die "soket=$!\n";
connect($s, pack_sockaddr_in({port}, inet_aton("127.0.0.1"))) or die "baglanti=$!\n";
print "tcp=baglandi\n"'"#
            ),
            true,
        )
        .await;
    assert_eq!(
        deger(&s.standart, "inet"),
        "acildi",
        "{}{}",
        s.standart,
        s.hata
    );
    assert_eq!(deger(&s.standart, "unix"), "Operation not permitted");
    assert_eq!(deger(&s.standart, "tcp"), "baglandi", "{}", s.hata);
    assert!(kabul.join().unwrap());
}

#[tokio::test]
async fn seccomp_engelleri() {
    let k = kurgu();
    let engelli = [
        ("io_uring_setup", libc::SYS_io_uring_setup, "1, 0"),
        ("keyctl", libc::SYS_keyctl, "0, 0, 0, 0, 0"),
        ("bpf", libc::SYS_bpf, "0, 0, 0"),
        ("ptrace", libc::SYS_ptrace, "0, 0, 0, 0"),
        (
            "perf_event_open",
            libc::SYS_perf_event_open,
            "0, 0, 0, 0, 0",
        ),
        ("userfaultfd", libc::SYS_userfaultfd, "0"),
        (
            "unshare",
            libc::SYS_unshare,
            &*format!("{}", libc::CLONE_NEWUSER),
        ),
    ];
    let mut betik = String::from("perl -e '");
    for (ad, nr, arg) in &engelli {
        betik.push_str(&format!(
            "$r = syscall({nr}, {arg}); print \"{ad}=\" . ($r == -1 ? $! : \"izin\") . \"\\n\"; "
        ));
    }
    betik.push_str(&format!(
        "$r = syscall({}, 0, 0); print \"clone3=\" . ($r == -1 ? $! : \"izin\") . \"\\n\"; ",
        libc::SYS_clone3
    ));
    if cfg!(target_arch = "x86_64") {
        betik.push_str(&format!(
            "$r = syscall({}); print \"x32=\" . ($r == -1 ? $! : \"izin\") . \"\\n\"; ",
            0x4000_0000 | libc::SYS_getpid
        ));
    }
    betik.push('\'');
    let s = k.betik(&k.kok, &betik, false).await;
    for (ad, _, _) in &engelli {
        assert_eq!(
            deger(&s.standart, ad),
            "Operation not permitted",
            "{}{}",
            s.standart,
            s.hata
        );
    }
    assert_eq!(deger(&s.standart, "clone3"), "Function not implemented");
    if cfg!(target_arch = "x86_64") {
        assert_eq!(deger(&s.standart, "x32"), "Function not implemented");
    }
}

#[tokio::test]
async fn iptal_agaci_torun_ve_setsid_kacagi_dahil_durdurur() {
    let k = kurgu();
    let istek = k.istek(
        &k.kok,
        KomutBicimi::Betik {
            metin: "sleep 7771 & setsid sleep 7772 & echo basladi; wait".into(),
        },
        false,
    );
    let iptal = CancellationToken::new();
    let surec = k.y.baslat(&istek).unwrap();
    let tetik = iptal.clone();
    let sonuc = surec
        .bekle(UZUN, iptal, move |_, veri| {
            if String::from_utf8_lossy(veri).contains("basladi") {
                tetik.cancel();
            }
        })
        .await;
    assert_eq!(sonuc.bitis, Bitis::Iptal);
    assert!(kalmadi_mi("sleep 7771", Duration::from_secs(5)));
    assert!(kalmadi_mi("sleep 7772", Duration::from_secs(5)));
}

/// Kalfa gibi davranan yardımcı: ortam değişkeniyle çağrılınca yalıtımda
/// uzun bir komut başlatır, "hazir" yazar ve bekler. Doğrudan koşunca bir şey
/// yapmaz.
#[tokio::test]
async fn yardimci_kalfa_gibi() {
    if std::env::var_os("KALFA_YARDIMCI").is_none() {
        return;
    }
    let k = kurgu();
    let istek = k.istek(
        &k.kok,
        KomutBicimi::Betik {
            metin: "sleep 7781 & echo basladi; wait".into(),
        },
        false,
    );
    let surec = k.y.baslat(&istek).unwrap();
    surec
        .bekle(UZUN, CancellationToken::new(), |_, veri| {
            if String::from_utf8_lossy(veri).contains("basladi") {
                println!("hazir");
            }
        })
        .await;
}

#[test]
fn kalfanin_olumu_agaci_durdurur() {
    let mut yardimci = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "yardimci_kalfa_gibi",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("KALFA_YARDIMCI", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let cikti = BufReader::new(yardimci.stdout.take().unwrap());
    for satir in cikti.lines() {
        if satir.unwrap().contains("hazir") {
            break;
        }
    }
    assert!(!surecler("sleep 7781").is_empty());
    yardimci.kill().unwrap();
    yardimci.wait().unwrap();
    assert!(kalmadi_mi("sleep 7781", Duration::from_secs(5)));
}

#[tokio::test]
async fn surec_sayisi_siniri_yalnizca_icerdekileri_sayar() {
    let k = kurgu();
    // Dışarıda 260 süreç varken içeride 100 süreç açılabilir.
    let mut disari: Vec<_> = (0..260)
        .map(|_| Command::new("sleep").arg("30").spawn().unwrap())
        .collect();
    let s = k
        .betik(
            &k.kok,
            "i=0; while [ $i -lt 100 ]; do sleep 3 & i=$((i+1)); done; echo yuz=tamam; wait",
            false,
        )
        .await;
    for c in &mut disari {
        let _ = c.kill();
        let _ = c.wait();
    }
    assert_eq!(deger(&s.standart, "yuz"), "tamam", "{}", s.hata);
    // İçeride 300 süreç açılamaz.
    let s = k
        .betik(
            &k.kok,
            "i=0; while [ $i -lt 300 ]; do sleep 3 & i=$((i+1)); done; echo uc_yuz=tamam; wait",
            false,
        )
        .await;
    assert!(!s.standart.contains("uc_yuz=tamam"));
    assert!(s.hata.to_lowercase().contains("fork"), "{}", s.hata);
}

#[tokio::test]
async fn sahte_bwrap_secilmez() {
    let k = kurgu();
    let isaret = k.taban.join("sahte-bwrap-calisti");
    calistirilabilir_yaz(
        &k.kok.join("bin/bwrap"),
        &format!("#!/bin/sh\ntouch {}\n", isaret.display()),
    );
    assert!(linux::bwrap_yolu().is_some_and(|y| y.starts_with("/usr")));
    let mut istek = k.istek(
        &k.kok,
        KomutBicimi::Program {
            program: "true".into(),
            argumanlar: Vec::new(),
        },
        false,
    );
    istek.ortam[0].1 = format!("{}:/usr/bin:/bin", k.kok.join("bin").display());
    let s = k.calistir_istek(istek, CancellationToken::new()).await;
    assert_eq!(s.bitis, Bitis::Cikti(Some(0)), "{}", s.hata);
    k.y.yeniden_algila();
    assert!(!isaret.exists());
}

#[tokio::test]
async fn program_yalitimin_icindeki_pathte_aranir() {
    let k = kurgu();
    let istek = k.istek(
        &k.kok,
        KomutBicimi::Program {
            program: "arac".into(),
            argumanlar: Vec::new(),
        },
        false,
    );
    let s = k.calistir_istek(istek, CancellationToken::new()).await;
    assert_eq!(s.standart.trim(), "arac-calisti");
    let istek = k.istek(
        &k.kok,
        KomutBicimi::Program {
            program: "kesinlikle-olmayan".into(),
            argumanlar: Vec::new(),
        },
        false,
    );
    assert!(matches!(
        k.y.baslat(&istek),
        Err(kalfa_sandbox::BaslatmaHatasi::ProgramYok { .. })
    ));
}

#[test]
fn tarama_yuz_bin_dosyada_olculur() {
    let d = tempfile::tempdir().unwrap();
    for i in 0..1000 {
        let dizin = d.path().join(format!("d{i}"));
        std::fs::create_dir(&dizin).unwrap();
        for j in 0..100 {
            std::fs::write(dizin.join(format!("f{j}.txt")), "").unwrap();
        }
    }
    yaz(&d.path().join("d7/.env"), "x");
    let bas = Instant::now();
    let t = tarama::tara(d.path(), &gizli_mi);
    let sure = bas.elapsed();
    println!("tarama: 100 000 dosya, {} ms", sure.as_millis());
    assert_eq!(t.dosyalar, vec![d.path().join("d7/.env")]);
}
````

- [ ] **Step 2: Testlerin başarısız olduğunu gör (Linux)**

Run: `cargo test -p kalfa-sandbox --features yalitim-testleri --locked --no-run`
Expected (Linux): derleme hatası (`kalfa_sandbox::linux` yok). Windows'ta dosya derlenmez.

- [ ] **Step 3: Arka ucu yaz**

**`crates/kalfa-sandbox/src/linux/seccomp.rs`** (tam içerik):

```rust
//! seccomp filtreleri (aşama 2 tasarımı §8.2). bwrap'a iki filtre verilir
//! (`--add-seccomp-fd`); çekirdek her sistem çağrısında ikisini de çalıştırır
//! ve en kısıtlayıcı sonucu uygular.
//!
//! - **Engel filtresi** (`seccompiler`): aşağıdaki çağrılar `EPERM` döner.
//!   Mimari tutmazsa (örneğin x86_64'te 32 bit `int 0x80`) süreç öldürülür.
//! - **`ENOSYS` filtresi** (elle yazılmış): `clone3`'ün bayrakları bir yapının
//!   içindedir ve filtrelenemez; `ENOSYS` alan libc `clone`'a geri düşer, onun
//!   bayrakları engel filtresinde denetlenir. x86_64'te x32 çağrıları
//!   (numarasında `0x40000000` biti olanlar) da `ENOSYS` döner: aynı mimari
//!   değeriyle gelip numara listesini atlatabilirlerdi.

use std::collections::BTreeMap;

use seccompiler::{
    BpfProgram, SeccompAction, SeccompCmpArgLen as Uzunluk, SeccompCmpOp as Islem,
    SeccompCondition, SeccompFilter, SeccompRule, TargetArch, sock_filter,
};

/// Her iki profilde de koşulsuz engellenenler.
const ENGELLI: &[i64] = &[
    libc::SYS_io_uring_setup,
    libc::SYS_io_uring_enter,
    libc::SYS_io_uring_register,
    libc::SYS_keyctl,
    libc::SYS_add_key,
    libc::SYS_request_key,
    libc::SYS_bpf,
    libc::SYS_perf_event_open,
    libc::SYS_userfaultfd,
    libc::SYS_ptrace,
];

/// Ad alanı bayrakları. `CLONE_NEWTIME` `clone`'da çıkış sinyali baytıyla
/// çakıştığı için yalnızca `unshare`'de denetlenir.
const AD_ALANI_BAYRAKLARI: &[u64] = &[
    libc::CLONE_NEWUSER as u64,
    libc::CLONE_NEWNS as u64,
    libc::CLONE_NEWPID as u64,
    libc::CLONE_NEWNET as u64,
    libc::CLONE_NEWIPC as u64,
    libc::CLONE_NEWUTS as u64,
    libc::CLONE_NEWCGROUP as u64,
];
const CLONE_NEWTIME: u64 = 0x80;

#[derive(Debug, thiserror::Error)]
#[error("seccomp filtresi üretilemedi: {0}")]
pub struct FiltreHatasi(String);

fn hata(e: impl std::fmt::Display) -> FiltreHatasi {
    FiltreHatasi(e.to_string())
}

pub(crate) fn mimari() -> Result<TargetArch, FiltreHatasi> {
    TargetArch::try_from(std::env::consts::ARCH).map_err(hata)
}

fn kosul(
    arguman: u8,
    uzunluk: Uzunluk,
    islem: Islem,
    deger: u64,
) -> Result<SeccompCondition, FiltreHatasi> {
    SeccompCondition::new(arguman, uzunluk, islem, deger).map_err(hata)
}

/// Bir bayrak kümesinden biri bile verilmişse eşleşen kurallar.
fn bayrak_kurallari(bayraklar: &[u64]) -> Result<Vec<SeccompRule>, FiltreHatasi> {
    bayraklar
        .iter()
        .map(|b| {
            SeccompRule::new(vec![kosul(0, Uzunluk::Qword, Islem::MaskedEq(*b), *b)?]).map_err(hata)
        })
        .collect()
}

/// `socket` çağrısının engellenen adres aileleri (§8.2). Ağsız profilde
/// `AF_INET` ve `AF_INET6` de engellenir. `AF_NETLINK` yalnızca
/// `NETLINK_ROUTE` (protokol 0) ile açılabilir. `socketpair` serbesttir.
fn soket_kurallari(ag: bool) -> Result<Vec<SeccompRule>, FiltreHatasi> {
    let mut aileler = vec![libc::AF_UNIX, libc::AF_PACKET];
    if !ag {
        aileler.extend([libc::AF_INET, libc::AF_INET6]);
    }
    let mut kurallar: Vec<SeccompRule> = aileler
        .into_iter()
        .map(|aile| {
            SeccompRule::new(vec![kosul(0, Uzunluk::Dword, Islem::Eq, aile as u64)?]).map_err(hata)
        })
        .collect::<Result<_, _>>()?;
    kurallar.push(
        SeccompRule::new(vec![
            kosul(0, Uzunluk::Dword, Islem::Eq, libc::AF_NETLINK as u64)?,
            kosul(2, Uzunluk::Dword, Islem::Ne, libc::NETLINK_ROUTE as u64)?,
        ])
        .map_err(hata)?,
    );
    Ok(kurallar)
}

/// Engel filtresi: eşleşen çağrı `EPERM` döner.
pub(crate) fn engel_filtresi(ag: bool) -> Result<BpfProgram, FiltreHatasi> {
    let mut kurallar: BTreeMap<i64, Vec<SeccompRule>> =
        ENGELLI.iter().map(|c| (*c, Vec::new())).collect();
    let mut unshare = AD_ALANI_BAYRAKLARI.to_vec();
    unshare.push(CLONE_NEWTIME);
    kurallar.insert(libc::SYS_unshare, bayrak_kurallari(&unshare)?);
    kurallar.insert(libc::SYS_clone, bayrak_kurallari(AD_ALANI_BAYRAKLARI)?);
    kurallar.insert(libc::SYS_socket, soket_kurallari(ag)?);
    let filtre = SeccompFilter::new(
        kurallar,
        SeccompAction::Allow,
        SeccompAction::Errno(libc::EPERM as u32),
        mimari()?,
    )
    .map_err(hata)?;
    BpfProgram::try_from(filtre).map_err(hata)
}

/// `BPF_LD | BPF_W | BPF_ABS`.
const BPF_LD_W_ABS: u16 = 0x20;
const BPF_JMP_JEQ_K: u16 = 0x05 | 0x10;
const BPF_JMP_JGE_K: u16 = 0x05 | 0x30;
const BPF_RET_K: u16 = 0x06;
/// `seccomp_data` içinde sistem çağrısı numarası ve mimari.
const NR: u32 = 0;
const ARCH: u32 = 4;
const AUDIT_ARCH_X86_64: u32 = 62 | 0x8000_0000 | 0x4000_0000;
const AUDIT_ARCH_AARCH64: u32 = 183 | 0x8000_0000 | 0x4000_0000;
const X32_BITI: u32 = 0x4000_0000;

fn komut(code: u16, jt: u8, jf: u8, k: u32) -> sock_filter {
    sock_filter { code, jt, jf, k }
}

/// `ENOSYS` filtresi. Mimari tutmazsa izin verir; o durumu engel filtresi
/// süreci öldürerek karşılar.
pub(crate) fn enosys_filtresi() -> Result<BpfProgram, FiltreHatasi> {
    let izin = libc::SECCOMP_RET_ALLOW;
    let enosys = libc::SECCOMP_RET_ERRNO | libc::ENOSYS as u32;
    let clone3 = libc::SYS_clone3 as u32;
    Ok(match mimari()? {
        TargetArch::x86_64 => vec![
            komut(BPF_LD_W_ABS, 0, 0, ARCH),
            komut(BPF_JMP_JEQ_K, 0, 3, AUDIT_ARCH_X86_64),
            komut(BPF_LD_W_ABS, 0, 0, NR),
            komut(BPF_JMP_JEQ_K, 2, 0, clone3),
            komut(BPF_JMP_JGE_K, 1, 0, X32_BITI),
            komut(BPF_RET_K, 0, 0, izin),
            komut(BPF_RET_K, 0, 0, enosys),
        ],
        TargetArch::aarch64 => vec![
            komut(BPF_LD_W_ABS, 0, 0, ARCH),
            komut(BPF_JMP_JEQ_K, 0, 2, AUDIT_ARCH_AARCH64),
            komut(BPF_LD_W_ABS, 0, 0, NR),
            komut(BPF_JMP_JEQ_K, 1, 0, clone3),
            komut(BPF_RET_K, 0, 0, izin),
            komut(BPF_RET_K, 0, 0, enosys),
        ],
        diger => return Err(FiltreHatasi(format!("desteklenmeyen mimari: {diger:?}"))),
    })
}

/// Programı bwrap'ın okuduğu biçime çevirir: `struct sock_filter` dizisi.
pub(crate) fn baytlar(program: &[sock_filter]) -> Vec<u8> {
    program
        .iter()
        .flat_map(|k| {
            let mut b = Vec::with_capacity(8);
            b.extend_from_slice(&k.code.to_ne_bytes());
            b.push(k.jt);
            b.push(k.jf);
            b.extend_from_slice(&k.k.to_ne_bytes());
            b
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iki_profilin_filtreleri_uretilir_ve_farklidir() {
        let agsiz = engel_filtresi(false).unwrap();
        let agli = engel_filtresi(true).unwrap();
        assert!(!agsiz.is_empty());
        assert_ne!(agsiz, agli);
        assert!(agsiz.len() > agli.len());
        let enosys = enosys_filtresi().unwrap();
        assert_eq!(baytlar(&enosys).len(), enosys.len() * 8);
        // Son iki komut: izin ve ENOSYS.
        assert_eq!(enosys[enosys.len() - 2].k, libc::SECCOMP_RET_ALLOW);
        assert_eq!(
            enosys[enosys.len() - 1].k,
            libc::SECCOMP_RET_ERRNO | libc::ENOSYS as u32
        );
    }
}
```

**`crates/kalfa-sandbox/src/linux/bwrap.rs`** (tam içerik):

```rust
//! bwrap seçenekleri (aşama 2 tasarımı §8.1). Seçenekler bwrap'a bir bellek
//! dosyasından (`--args FD`) verilir; yol sayısı komut satırı sınırına takılmaz.
//!
//! Sıra önemlidir: sonra gelen bağlama öncekinin üstüne biner. `/tmp` en başta
//! bağlanır; altındaki yollar (örneğin geçici dizindeki bir çalışma alanı) onun
//! üstüne gelir. Korumalı ve gizli yollar en sonda, çalışma alanının üstüne
//! bağlanır.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::profil::{AlanBilgisi, OzelDizinler};

/// Sistemin salt okunur kökleri: dizin ya da (birleşik `/usr`'da) bağlantı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Sistem {
    Dizin(PathBuf),
    Bag { yol: PathBuf, hedef: PathBuf },
}

impl Sistem {
    pub(crate) fn yol(&self) -> &Path {
        match self {
            Self::Dizin(y) | Self::Bag { yol: y, .. } => y,
        }
    }
}

pub(crate) struct Kurulum<'a> {
    pub sistem: &'a [Sistem],
    /// Araç zinciri ve kullanıcının eklediği dizinler; yalnızca var olanlar.
    pub okunur: &'a [PathBuf],
    /// Kullanıcının `~/.cargo/registry`'si; çalışma alanının cargo
    /// önbelleğine salt okunur bağlanır; ne zaman bağlanacağını
    /// `registry_secimi` belirler (§7.4).
    pub registry: Option<&'a Path>,
    pub alan: &'a AlanBilgisi,
    pub ozel: &'a OzelDizinler,
    pub dizin: &'a Path,
    pub ortam: &'a [(String, String)],
    pub ag: bool,
    /// Ağlı profilde `/etc/resolv.conf`'un gösterdiği dosya; `/etc` dışında
    /// olabilir (`/run/...`, WSL'de `/mnt/wsl/...`).
    pub resolv: Option<&'a Path>,
    pub seccomp: &'a [i32],
}

struct Secenekler(Vec<OsString>);

impl Secenekler {
    fn ekle(&mut self, parcalar: &[&str]) {
        self.0.extend(parcalar.iter().map(OsString::from));
    }

    fn yolla(&mut self, secenek: &str, yollar: &[&Path]) {
        self.0.push(secenek.into());
        self.0
            .extend(yollar.iter().map(|y| y.as_os_str().to_os_string()));
    }

    fn ayni(&mut self, secenek: &str, yol: &Path) {
        self.yolla(secenek, &[yol, yol]);
    }
}

pub(crate) fn secenekler(k: &Kurulum) -> Vec<OsString> {
    let mut s = Secenekler(Vec::new());
    s.ekle(&[
        "--unshare-user",
        "--unshare-pid",
        "--unshare-ipc",
        "--unshare-uts",
        "--unshare-cgroup-try",
    ]);
    if !k.ag {
        s.ekle(&["--unshare-net"]);
    }
    s.ekle(&[
        "--disable-userns",
        "--die-with-parent",
        "--new-session",
        "--clearenv",
        "--tmpfs",
        "/",
    ]);
    for sistem in k.sistem {
        match sistem {
            Sistem::Dizin(yol) => s.ayni("--ro-bind", yol),
            Sistem::Bag { yol, hedef } => s.yolla("--symlink", &[hedef, yol]),
        }
    }
    s.ekle(&["--proc", "/proc", "--dev", "/dev", "--tmpfs", "/run"]);
    if k.ag
        && let Some(resolv) = k.resolv
    {
        s.ayni("--ro-bind", resolv);
    }
    s.yolla("--bind", &[&k.ozel.gecici, Path::new("/tmp")]);
    for yol in k.okunur {
        s.ayni("--ro-bind", yol);
    }
    s.ayni("--bind", &k.alan.kok);
    s.ayni("--bind", &k.ozel.ev);
    s.ayni("--bind", &k.ozel.onbellek);
    if let Some(registry) = k.registry {
        s.yolla("--ro-bind", &[registry, &k.ozel.cargo().join("registry")]);
    }
    for git in &k.alan.git_dizinleri {
        s.ayni("--bind", git);
    }
    for korumali in &k.alan.korumali {
        s.ayni("--ro-bind", &korumali.yol);
    }
    for dosya in &k.alan.gizli.dosyalar {
        s.yolla("--ro-bind", &[Path::new("/dev/null"), dosya]);
    }
    for dizin in &k.alan.gizli.dizinler {
        s.yolla("--tmpfs", &[dizin]);
        s.yolla("--remount-ro", &[dizin]);
    }
    // Boş kök de salt okunur olur: çalışma alanı dışına yazma denemesi
    // sessizce bellekte kalmak yerine hata verir.
    s.ekle(&["--remount-ro", "/"]);
    for (ad, deger) in k.ortam {
        s.ekle(&["--setenv", ad, deger]);
    }
    s.yolla("--chdir", &[k.dizin]);
    for fd in k.seccomp {
        s.ekle(&["--add-seccomp-fd", &fd.to_string()]);
    }
    s.0
}

/// `--args` biçimi: her seçenek NUL ile biter.
pub(crate) fn nul_ile(secenekler: &[OsString]) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    let mut veri = Vec::new();
    for s in secenekler {
        veri.extend_from_slice(s.as_bytes());
        veri.push(0);
    }
    veri
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profil::Korumali;
    use crate::tarama::Tarama;

    fn metin(s: &[OsString]) -> Vec<String> {
        s.iter().map(|p| p.to_string_lossy().into_owned()).collect()
    }

    /// `dizi` içinde `parca`'nın ardışık geçtiği konum.
    fn konum(dizi: &[String], parca: &[&str]) -> usize {
        dizi.windows(parca.len())
            .position(|w| w.iter().zip(parca).all(|(a, b)| a == b))
            .unwrap_or_else(|| panic!("{parca:?} yok: {dizi:?}"))
    }

    fn kurulum_ile<T>(ag: bool, is: impl FnOnce(&Kurulum) -> T) -> T {
        let alan = AlanBilgisi {
            kok: "/tmp/alan".into(),
            gizli: Tarama {
                dosyalar: vec!["/tmp/alan/.env".into()],
                dizinler: vec!["/tmp/alan/gizli".into()],
            },
            korumali: vec![Korumali {
                yol: "/tmp/alan/.git/hooks".into(),
                dizin: true,
            }],
            git_dizinleri: vec!["/depo/.git".into()],
        };
        let ozel = OzelDizinler::yol(Path::new("/veri/yalitim"), "abc");
        let sistem = [
            Sistem::Dizin("/usr".into()),
            Sistem::Bag {
                yol: "/bin".into(),
                hedef: "usr/bin".into(),
            },
        ];
        let okunur = [PathBuf::from("/ev/.cargo/bin")];
        let ortam = [("HOME".to_string(), "/veri/yalitim/abc/ev".to_string())];
        let k = Kurulum {
            sistem: &sistem,
            okunur: &okunur,
            registry: Some(Path::new("/ev/.cargo/registry")),
            alan: &alan,
            ozel: &ozel,
            dizin: Path::new("/tmp/alan/alt"),
            ortam: &ortam,
            ag,
            resolv: Some(Path::new("/run/resolv.conf")),
            seccomp: &[5, 6],
        };
        is(&k)
    }

    #[test]
    fn agsiz_profilde_ag_ayrilir_ve_siralama_dogrudur() {
        let s = kurulum_ile(false, |k| metin(&secenekler(k)));
        assert!(s.contains(&"--unshare-net".to_string()));
        assert!(!s.contains(&"/run/resolv.conf".to_string()));
        for bayrak in [
            "--disable-userns",
            "--die-with-parent",
            "--new-session",
            "--clearenv",
        ] {
            assert!(s.contains(&bayrak.to_string()), "{bayrak} yok");
        }
        let kok = konum(&s, &["--tmpfs", "/"]);
        let usr = konum(&s, &["--ro-bind", "/usr", "/usr"]);
        let bin = konum(&s, &["--symlink", "usr/bin", "/bin"]);
        let tmp = konum(&s, &["--bind", "/veri/yalitim/abc/gecici", "/tmp"]);
        let alan = konum(&s, &["--bind", "/tmp/alan", "/tmp/alan"]);
        let hooks = konum(
            &s,
            &["--ro-bind", "/tmp/alan/.git/hooks", "/tmp/alan/.git/hooks"],
        );
        let env = konum(&s, &["--ro-bind", "/dev/null", "/tmp/alan/.env"]);
        let gizli = konum(
            &s,
            &[
                "--tmpfs",
                "/tmp/alan/gizli",
                "--remount-ro",
                "/tmp/alan/gizli",
            ],
        );
        assert!(kok < usr && usr < bin && bin < tmp && tmp < alan);
        assert!(alan < hooks && alan < env && alan < gizli);
        assert!(gizli < konum(&s, &["--remount-ro", "/"]));
        konum(
            &s,
            &[
                "--ro-bind",
                "/ev/.cargo/registry",
                "/veri/yalitim/abc/onbellek/cargo/registry",
            ],
        );
        konum(&s, &["--bind", "/depo/.git", "/depo/.git"]);
        konum(&s, &["--setenv", "HOME", "/veri/yalitim/abc/ev"]);
        konum(
            &s,
            &[
                "--chdir",
                "/tmp/alan/alt",
                "--add-seccomp-fd",
                "5",
                "--add-seccomp-fd",
                "6",
            ],
        );
    }

    #[test]
    fn agli_profilde_ag_ayrilmaz_resolv_baglanir() {
        let s = kurulum_ile(true, |k| metin(&secenekler(k)));
        assert!(!s.contains(&"--unshare-net".to_string()));
        konum(&s, &["--ro-bind", "/run/resolv.conf", "/run/resolv.conf"]);
    }

    #[test]
    fn args_bicimi_nul_ile_biter() {
        let veri = nul_ile(&["--a".into(), "b c".into()]);
        assert_eq!(veri, b"--a\0b c\0");
    }
}
```

**`crates/kalfa-sandbox/src/linux/mod.rs`** (tam içerik):

```rust
//! Linux arka ucu: `bubblewrap` + `seccomp` (aşama 2 tasarımı §8).
//!
//! `LinuxYalitici` açılışta yalıtımı algılar ve durumu önbelleğe alır. Yalıtım
//! hazırsa komut bwrap'ın içinde çalışır; değilse yalıtımsız çalışır ve durum
//! bunu söyler (kabuk her çağrıda yerel onay ister). İstek hangi kipte
//! onaylandıysa o kipte çalışır: onaydan sonra durum değiştiyse komut hiç
//! başlamaz.

mod bwrap;
pub mod seccomp;

use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Seek, SeekFrom, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::profil::{self, AlanBilgisi, GitKimligi, Korumali, OzelDizinler, alan_kimligi};
use crate::{
    AZAMI_SUREC, BaslatmaHatasi, CalistirmaIstegi, KomutBicimi, Surec, Yalitici, YalitimDurumu,
    YalitimYoklukNedeni, program,
};
use bwrap::{Kurulum, Sistem};

/// bwrap yalnızca sistem yollarından kabul edilir; `PATH`'ten aranmaz (§8.1).
const BWRAP_YOLLARI: &[&str] = &["/usr/bin/bwrap", "/usr/local/bin/bwrap"];
/// Süreç sayısı sınırını yalıtımın içinde koyar (§6.3): sınır bwrap'a
/// dışarıda verilseydi kullanıcının bütün süreçleri sayılırdı.
const PRLIMIT: &str = "/usr/bin/prlimit";
const SISTEM: &[&str] = &[
    "/usr", "/etc", "/bin", "/sbin", "/lib", "/lib32", "/lib64", "/libx32",
];
pub const ARKA_UC: &str = "bwrap";

// Talimatlar arayüzde gösterilir; arayüzün diliyle ("siz") yazılır.
pub const TALIMAT_BWRAP_YOK: &str = "Komutları yalıtmak için `bubblewrap` paketini kurun \
(`sudo apt install bubblewrap` ya da dağıtımınızın paket yöneticisiyle), sonra Ayarlar'da \
\"Yeniden dene\"yi seçin. O zamana kadar her komut yerel pencerede onayınıza sunulur.";
pub const TALIMAT_AD_ALANI: &str = "Sistem, ayrıcalıksız kullanıcı ad alanlarını kısıtlıyor \
(Ubuntu 24.04 ve sonrasında AppArmor). Kalfa'nın paketi bunu kendi AppArmor profiliyle \
çözecek. O zamana kadar bwrap için bir profil ekleyebilirsiniz: /etc/apparmor.d/bwrap \
dosyasına `abi <abi/4.0>, include <tunables/global> profile bwrap /usr/bin/bwrap \
flags=(unconfined) { userns, }` yazın, `sudo systemctl reload apparmor` çalıştırın ve \
Ayarlar'da \"Yeniden dene\"yi seçin. Sistem genelindeki kısıtı kapatmanızı önermiyoruz: \
bütün sistemi etkiler. O zamana kadar her komut yerel pencerede onayınıza sunulur.";

/// Linux arka ucunun ayarları.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LinuxAyari {
    /// Kullanıcının ev dizini; araç zinciri dizinleri buna göre bulunur.
    pub ev: PathBuf,
    /// Çalışma alanına özel dizinlerin kökü: `<veri dizini>/yalitim` (§7.3).
    pub yalitim_dizini: PathBuf,
    /// Kullanıcının Ayarlar'dan eklediği araç zinciri dizinleri (§7.1).
    pub ek_okunur: Vec<PathBuf>,
    pub git: GitKimligi,
}

impl LinuxAyari {
    /// Yalıtıma salt okunur açılan araç zinciri dizinleri: varsayılanlar ve
    /// kullanıcının ekledikleri; yalnızca var olanlar (§7.1). `~/.rustup`'ın
    /// tamamı açılır: rustup vekili her çalışmada `update-hashes` dizinini
    /// ister; dizinde kimlik bilgisi yoktur.
    fn okunur(&self) -> Vec<PathBuf> {
        let varsayilan = [self.ev.join(".cargo/bin"), self.ev.join(".rustup")];
        let mut yollar: Vec<PathBuf> = varsayilan
            .into_iter()
            .chain(self.ek_okunur.iter().cloned())
            .filter(|y| y.is_absolute() && y.exists())
            .collect();
        yollar.dedup();
        yollar
    }

    fn registry(&self) -> Option<PathBuf> {
        Some(self.ev.join(".cargo/registry")).filter(|y| y.is_dir())
    }

    fn rustup(&self) -> Option<PathBuf> {
        Some(self.ev.join(".rustup")).filter(|y| y.is_dir())
    }
}

/// Yalıtımın kullanılabilirliğini bulan birim (§8.3); testte sahtesi kullanılır.
pub trait Algilayici: Send + Sync {
    fn algila(&self, ayar: &LinuxAyari) -> YalitimDurumu;
}

/// Gerçek algılayıcı: bwrap'ı sistem yollarında arar, seccomp filtresini
/// üretir ve kısa bir deneme çalıştırması yapar.
#[derive(Debug, Default)]
pub struct BwrapAlgilayici;

impl Algilayici for BwrapAlgilayici {
    fn algila(&self, ayar: &LinuxAyari) -> YalitimDurumu {
        let Some(bwrap) = bwrap_yolu() else {
            return yok(YalitimYoklukNedeni::BwrapYok, TALIMAT_BWRAP_YOK);
        };
        if let Err(e) = seccomp::engel_filtresi(false).and(seccomp::enosys_filtresi()) {
            return yok(
                YalitimYoklukNedeni::Desteklenmiyor,
                &format!("{e}. Her komut yerel pencerede onayınıza sunulur."),
            );
        }
        match deneme(bwrap, ayar) {
            Ok(()) => YalitimDurumu::Hazir {
                arka_uc: ARKA_UC.into(),
                surum: surum(bwrap),
            },
            Err(hata) => durum_hatadan(&hata),
        }
    }
}

fn yok(neden: YalitimYoklukNedeni, talimat: &str) -> YalitimDurumu {
    YalitimDurumu::Yok {
        neden,
        talimat: talimat.to_string(),
    }
}

/// Deneme çalıştırmasının hatasından durumu çıkarır.
fn durum_hatadan(hata: &str) -> YalitimDurumu {
    let kucuk = hata.to_lowercase();
    let ad_alani = [
        "namespace",
        "uid map",
        "rtm_newaddr",
        "operation not permitted",
    ];
    if ad_alani.iter().any(|p| kucuk.contains(p)) {
        return yok(
            YalitimYoklukNedeni::KullaniciAdAlaniKapali,
            TALIMAT_AD_ALANI,
        );
    }
    yok(
        YalitimYoklukNedeni::Desteklenmiyor,
        &format!(
            "Yalıtım bu sistemde başlatılamadı: {}. Her komut yerel pencerede onayınıza sunulur.",
            hata.trim()
        ),
    )
}

pub fn bwrap_yolu() -> Option<&'static Path> {
    BWRAP_YOLLARI
        .iter()
        .map(Path::new)
        .find(|y| program::calistirilabilir(y))
}

fn surum(bwrap: &Path) -> String {
    std::process::Command::new(bwrap)
        .arg("--version")
        .output()
        .ok()
        .map(|c| String::from_utf8_lossy(&c.stdout).trim().to_string())
        .and_then(|s| s.rsplit(' ').next().map(str::to_string))
        .unwrap_or_default()
}

/// `/usr/bin/true`'yu gerçek profille, geçici bir çalışma alanında çalıştırır.
fn deneme(bwrap: &Path, ayar: &LinuxAyari) -> Result<(), String> {
    let gecici = tempfile::Builder::new()
        .prefix("kalfa-yalitim-denemesi")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let kok = gecici.path().join("alan");
    std::fs::create_dir_all(&kok).map_err(|e| e.to_string())?;
    let ayar = LinuxAyari {
        yalitim_dizini: gecici.path().join("yalitim"),
        ..ayar.clone()
    };
    let istek = CalistirmaIstegi {
        komut: KomutBicimi::Program {
            program: "true".into(),
            argumanlar: Vec::new(),
        },
        dizin: kok.clone(),
        ortam: vec![("PATH".into(), "/usr/bin:/bin".into())],
        ag: false,
        alan: Some(AlanBilgisi {
            kok,
            gizli: Default::default(),
            korumali: Vec::new(),
            git_dizinleri: Vec::new(),
        }),
    };
    let (mut komut, _fdler) = komut_kur(bwrap, &ayar, &istek).map_err(|e| e.to_string())?;
    let cikti = komut.output().map_err(|e| e.to_string())?;
    if cikti.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&cikti.stderr).into_owned())
    }
}

pub struct LinuxYalitici {
    ayar: RwLock<LinuxAyari>,
    algilayici: Box<dyn Algilayici>,
    durum: RwLock<YalitimDurumu>,
    degisince: Box<dyn Fn(&YalitimDurumu) + Send + Sync>,
}

impl LinuxYalitici {
    /// Algılamayı hemen çalıştırır. `degisince`, durum sonradan değişince
    /// (yeniden algılama) çağrılır.
    pub fn yeni(
        ayar: LinuxAyari,
        algilayici: Box<dyn Algilayici>,
        degisince: Box<dyn Fn(&YalitimDurumu) + Send + Sync>,
    ) -> Self {
        let durum = algilayici.algila(&ayar);
        Self {
            ayar: RwLock::new(ayar),
            algilayici,
            durum: RwLock::new(durum),
            degisince,
        }
    }

    fn ayar(&self) -> LinuxAyari {
        self.ayar.read().unwrap_or_else(|z| z.into_inner()).clone()
    }

    fn yalitimli(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi> {
        let bwrap = bwrap_yolu()
            .ok_or_else(|| BaslatmaHatasi::Yalitim("bwrap artık sistem yollarında yok".into()))?;
        let (komut, fdler) = komut_kur(bwrap, &self.ayar(), istek)?;
        let surec = Surec::komutla(komut.into());
        // Bellek dosyaları çocuğa geçti; buradaki kopyalar kapanabilir.
        drop(fdler);
        surec
    }
}

impl Yalitici for LinuxYalitici {
    fn durum(&self) -> YalitimDurumu {
        self.durum.read().unwrap_or_else(|z| z.into_inner()).clone()
    }

    fn profil_ozeti(&self, ag: bool) -> Option<String> {
        if !self.durum().hazir_mi() {
            return None;
        }
        let ayar = self.ayar();
        let mut okunur: Vec<PathBuf> = sistem_dizinleri()
            .iter()
            .map(|s| s.yol().to_path_buf())
            .collect();
        okunur.extend(ayar.okunur());
        Some(profil::ozet(ARKA_UC, &okunur, ag))
    }

    fn baslat(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi> {
        match (&istek.alan, self.durum().hazir_mi()) {
            (Some(_), true) => self.yalitimli(istek),
            (None, false) => Surec::baslat(istek),
            (Some(_), false) => Err(BaslatmaHatasi::Yalitim(
                "komut yalıtımlı çalışmak üzere onaylandı ama yalıtım artık kullanılamıyor".into(),
            )),
            (None, true) => Err(BaslatmaHatasi::Yalitim(
                "komut yalıtımsız çalışmak üzere onaylandı ama yalıtım artık kullanılabiliyor"
                    .into(),
            )),
        }
    }

    fn yeniden_algila(&self) -> YalitimDurumu {
        let yeni = self.algilayici.algila(&self.ayar());
        let degisti = {
            let mut durum = self.durum.write().unwrap_or_else(|z| z.into_inner());
            let degisti = *durum != yeni;
            *durum = yeni.clone();
            degisti
        };
        if degisti {
            (self.degisince)(&yeni);
        }
        yeni
    }

    fn okunur_dizinleri_ayarla(&self, dizinler: Vec<PathBuf>) {
        self.ayar
            .write()
            .unwrap_or_else(|z| z.into_inner())
            .ek_okunur = dizinler;
    }
}

fn sistem_dizinleri() -> Vec<Sistem> {
    SISTEM
        .iter()
        .map(Path::new)
        .filter_map(|yol| {
            let bilgi = std::fs::symlink_metadata(yol).ok()?;
            if bilgi.file_type().is_symlink() {
                let hedef = std::fs::read_link(yol).ok()?;
                Some(Sistem::Bag {
                    yol: yol.to_path_buf(),
                    hedef,
                })
            } else {
                bilgi.is_dir().then(|| Sistem::Dizin(yol.to_path_buf()))
            }
        })
        .collect()
}

/// Kullanıcının cargo registry'si ne zaman bağlanır (§7.4)?
///
/// - Ağsız çalıştırmada, çalışma alanının kendi registry'si boşsa: önceden
///   indirilmiş crate'ler salt okunur kullanılır.
/// - Ağlı çalıştırmada bağlanmaz: cargo yeni crate'i salt okunur registry'ye
///   indiremez. Çalışma alanına özel, başta boş registry'ye indirir.
/// - O registry bir kez dolduysa ağsız çalıştırmalar da onu kullanır.
fn registry_secimi(ag: bool, ozel: &OzelDizinler, ayar: &LinuxAyari) -> Option<PathBuf> {
    let kendi = ozel.cargo().join("registry");
    let dolu = std::fs::read_dir(&kendi).is_ok_and(|mut d| d.next().is_some());
    if ag || dolu { None } else { ayar.registry() }
}

/// Eksik korumalı yolu boş olarak oluşturur: bağlanamayan eksik `.git/hooks`
/// dizinine komut kanca yazabilirdi.
fn korumalilari_hazirla(korumali: &[Korumali]) -> io::Result<()> {
    for k in korumali {
        if k.yol.exists() {
            continue;
        }
        if k.dizin {
            std::fs::create_dir_all(&k.yol)?;
        } else {
            if let Some(ust) = k.yol.parent() {
                std::fs::create_dir_all(ust)?;
            }
            std::fs::File::create(&k.yol)?;
        }
    }
    Ok(())
}

/// Yalıtımdaki ortam: izin listesinden gelenler; ev, geçici dizin ve araç
/// yapılandırması çalışma alanına özel dizinleri gösterir (§6.1, §7.4).
fn yalitim_ortami(
    kaynak: &[(String, String)],
    ozel: &OzelDizinler,
    ayar: &LinuxAyari,
) -> Vec<(String, String)> {
    let yol = |y: &Path| y.display().to_string();
    let mut kalfanin = vec![
        ("HOME", yol(&ozel.ev)),
        ("TMPDIR", "/tmp".to_string()),
        ("CARGO_HOME", yol(&ozel.cargo())),
        ("GIT_CONFIG_GLOBAL", yol(&ozel.gitconfig())),
        ("NPM_CONFIG_USERCONFIG", yol(&ozel.npmrc())),
        ("NPM_CONFIG_CACHE", yol(&ozel.npm())),
        ("PIP_CACHE_DIR", yol(&ozel.pip())),
    ];
    if let Some(rustup) = ayar.rustup() {
        kalfanin.push(("RUSTUP_HOME", yol(&rustup)));
    }
    let mut ortam: Vec<(String, String)> = kaynak
        .iter()
        .filter(|(ad, _)| !kalfanin.iter().any(|(k, _)| k == ad))
        .filter(|(ad, _)| !matches!(ad.as_str(), "TEMP" | "TMP"))
        .cloned()
        .collect();
    ortam.extend(kalfanin.into_iter().map(|(a, d)| (a.to_string(), d)));
    ortam
}

/// Yol yalıtımın içinde görünür mü? Yollar içeride de aynı konumdadır.
fn gorunur_mu(yol: &Path, kokler: &[PathBuf], alan: &AlanBilgisi) -> bool {
    kokler.iter().any(|k| yol.starts_with(k))
        && !alan.gizli.dosyalar.iter().any(|g| g == yol)
        && !alan.gizli.dizinler.iter().any(|g| yol.starts_with(g))
}

/// Programı yalıtımın içindeki `PATH`'te, yalnızca içeride görünen yollarda
/// arar (§4.1). Çalışma alanındaki bir program mutlak yolla gelir.
fn programi_bul(
    program: &str,
    ortam: &[(String, String)],
    kokler: &[PathBuf],
    alan: &AlanBilgisi,
) -> Result<PathBuf, BaslatmaHatasi> {
    let path = crate::ortam::deger(ortam, "PATH").unwrap_or("");
    let gorunen: Vec<PathBuf> = std::env::split_paths(path)
        .filter(|d| d.is_absolute() && gorunur_mu(d, kokler, alan))
        .collect();
    let gorunen_path = std::env::join_paths(&gorunen).unwrap_or_default();
    let yol = program::bul(
        program,
        &[("PATH".into(), gorunen_path.to_string_lossy().into_owned())],
    )
    .map_err(|_| BaslatmaHatasi::ProgramYok {
        program: program.to_string(),
        path: gorunen_path.to_string_lossy().into_owned(),
    })?;
    if gorunur_mu(&yol, kokler, alan) {
        Ok(yol)
    } else {
        Err(BaslatmaHatasi::ProgramYok {
            program: program.to_string(),
            path: gorunen_path.to_string_lossy().into_owned(),
        })
    }
}

/// Kapanınca serbest kalan, adsız bir bellek dosyası: bwrap seçenekleri ve
/// seccomp programları bununla geçer.
fn bellek_dosyasi(veri: &[u8]) -> io::Result<OwnedFd> {
    // SAFETY: `memfd_create` sabit, NUL ile biten bir ad ve bayrak alır;
    // dönen tanımlayıcı başarılıysa yalnızca burada sahiplenilir.
    let fd = unsafe { libc::memfd_create(c"kalfa".as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `fd` az önce açıldı ve başka bir sahibi yok.
    let mut dosya = File::from(unsafe { OwnedFd::from_raw_fd(fd) });
    dosya.write_all(veri)?;
    dosya.seek(SeekFrom::Start(0))?;
    Ok(OwnedFd::from(dosya))
}

/// bwrap komutunu kurar. Dönen tanımlayıcılar komut başlayana kadar açık
/// kalmalıdır.
fn komut_kur(
    bwrap: &Path,
    ayar: &LinuxAyari,
    istek: &CalistirmaIstegi,
) -> Result<(std::process::Command, Vec<OwnedFd>), BaslatmaHatasi> {
    let alan = istek
        .alan
        .as_ref()
        .ok_or_else(|| BaslatmaHatasi::Yalitim("çalışma alanı bilgisi yok".into()))?;
    let ozel = OzelDizinler::yol(&ayar.yalitim_dizini, &alan_kimligi(&alan.kok));
    ozel.hazirla(&ayar.git)?;
    korumalilari_hazirla(&alan.korumali)?;
    let sistem = sistem_dizinleri();
    let okunur = ayar.okunur();
    let registry = registry_secimi(istek.ag, &ozel, ayar);
    let ortam = yalitim_ortami(&istek.ortam, &ozel, ayar);

    let mut kokler: Vec<PathBuf> = sistem.iter().map(|s| s.yol().to_path_buf()).collect();
    kokler.extend(okunur.iter().cloned());
    kokler.push(alan.kok.clone());
    kokler.push(ozel.kok.clone());
    kokler.extend(alan.git_dizinleri.iter().cloned());
    let (program, argumanlar): (PathBuf, Vec<String>) = match &istek.komut {
        KomutBicimi::Program {
            program,
            argumanlar,
        } => (
            programi_bul(program, &ortam, &kokler, alan)?,
            argumanlar.clone(),
        ),
        KomutBicimi::Betik { metin } => ("/bin/sh".into(), vec!["-c".into(), metin.clone()]),
    };

    let hata = |e: seccomp::FiltreHatasi| BaslatmaHatasi::Yalitim(e.to_string());
    let engel = bellek_dosyasi(&seccomp::baytlar(
        &seccomp::engel_filtresi(istek.ag).map_err(hata)?,
    ))?;
    let enosys = bellek_dosyasi(&seccomp::baytlar(
        &seccomp::enosys_filtresi().map_err(hata)?,
    ))?;
    let resolv = std::fs::canonicalize("/etc/resolv.conf").ok();
    let secenekler = bwrap::secenekler(&Kurulum {
        sistem: &sistem,
        okunur: &okunur,
        registry: registry.as_deref(),
        alan,
        ozel: &ozel,
        dizin: &istek.dizin,
        ortam: &ortam,
        ag: istek.ag,
        resolv: resolv.as_deref(),
        seccomp: &[engel.as_raw_fd(), enosys.as_raw_fd()],
    });
    let args = bellek_dosyasi(&bwrap::nul_ile(&secenekler))?;

    let mut komut = std::process::Command::new(bwrap);
    komut
        .arg("--args")
        .arg(args.as_raw_fd().to_string())
        .arg("--");
    if program::calistirilabilir(Path::new(PRLIMIT)) {
        komut
            .arg(PRLIMIT)
            .arg(format!("--nproc={AZAMI_SUREC}:{AZAMI_SUREC}"))
            .arg("--");
    }
    komut
        .arg(&program)
        .args(argumanlar.iter().map(OsString::from))
        .env_clear()
        .current_dir("/");
    let devredilen = [args.as_raw_fd(), engel.as_raw_fd(), enosys.as_raw_fd()];
    // SAFETY: Kapanış `fork` ile `exec` arasında çocukta çalışır ve yalnızca
    // `fcntl`'ı çağırır (async-signal-safe). Tanımlayıcılar komut başlayana
    // kadar açık kalır: sahipleri çağırana döner.
    unsafe {
        komut.pre_exec(move || {
            for fd in devredilen {
                if libc::fcntl(fd, libc::F_SETFD, 0) == -1 {
                    return Err(io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    Ok((komut, vec![args, engel, enosys]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Sahte(RwLock<YalitimDurumu>);

    impl Algilayici for Arc<Sahte> {
        fn algila(&self, _: &LinuxAyari) -> YalitimDurumu {
            self.0.read().unwrap().clone()
        }
    }

    fn hazir() -> YalitimDurumu {
        YalitimDurumu::Hazir {
            arka_uc: ARKA_UC.into(),
            surum: "0.9.0".into(),
        }
    }

    #[test]
    fn algilama_hatalari_nedenlere_esler() {
        for (hata, neden) in [
            (
                "bwrap: setting up uid map: Permission denied",
                YalitimYoklukNedeni::KullaniciAdAlaniKapali,
            ),
            (
                "bwrap: No permissions to create new namespace, likely because the kernel does not allow non-privileged user namespaces.",
                YalitimYoklukNedeni::KullaniciAdAlaniKapali,
            ),
            (
                "bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted",
                YalitimYoklukNedeni::KullaniciAdAlaniKapali,
            ),
            (
                "bwrap: seccomp: Invalid argument",
                YalitimYoklukNedeni::Desteklenmiyor,
            ),
        ] {
            match durum_hatadan(hata) {
                YalitimDurumu::Yok { neden: n, talimat } => {
                    assert_eq!(n, neden, "{hata}");
                    assert!(!talimat.is_empty());
                }
                d => panic!("{hata}: {d:?}"),
            }
        }
    }

    #[test]
    fn durum_degisince_bildirilir_ve_istek_onaylandigi_kipte_calisir() {
        let sahte = Arc::new(Sahte(RwLock::new(yok(
            YalitimYoklukNedeni::BwrapYok,
            TALIMAT_BWRAP_YOK,
        ))));
        let bildirim = Arc::new(AtomicUsize::new(0));
        let b = bildirim.clone();
        let y = LinuxYalitici::yeni(
            LinuxAyari::default(),
            Box::new(sahte.clone()),
            Box::new(move |_| {
                b.fetch_add(1, Ordering::SeqCst);
            }),
        );
        assert!(!y.durum().hazir_mi());
        assert_eq!(y.profil_ozeti(false), None);
        let alanli = CalistirmaIstegi {
            komut: KomutBicimi::Betik {
                metin: "true".into(),
            },
            dizin: std::env::temp_dir(),
            ortam: Vec::new(),
            ag: false,
            alan: Some(AlanBilgisi {
                kok: std::env::temp_dir(),
                gizli: Default::default(),
                korumali: Vec::new(),
                git_dizinleri: Vec::new(),
            }),
        };
        // Yalıtımlı onaylanan komut, yalıtım yokken başlamaz.
        assert!(matches!(y.baslat(&alanli), Err(BaslatmaHatasi::Yalitim(_))));

        // Aynı durum yeniden algılanınca bildirim gitmez.
        y.yeniden_algila();
        assert_eq!(bildirim.load(Ordering::SeqCst), 0);
        *sahte.0.write().unwrap() = hazir();
        assert_eq!(y.yeniden_algila(), hazir());
        assert_eq!(bildirim.load(Ordering::SeqCst), 1);
        assert!(y.profil_ozeti(false).is_some());
        assert_ne!(y.profil_ozeti(false), y.profil_ozeti(true));

        // Yalıtımsız onaylanan komut, yalıtım gelince başlamaz.
        let alansiz = CalistirmaIstegi {
            alan: None,
            ..alanli
        };
        assert!(matches!(
            y.baslat(&alansiz),
            Err(BaslatmaHatasi::Yalitim(_))
        ));
    }

    #[test]
    fn eklenen_dizin_profil_ozetini_degistirir() {
        let ev = tempfile::tempdir().unwrap();
        let ek = tempfile::tempdir().unwrap();
        let y = LinuxYalitici::yeni(
            LinuxAyari {
                ev: ev.path().into(),
                ..Default::default()
            },
            Box::new(Arc::new(Sahte(RwLock::new(hazir())))),
            Box::new(|_| {}),
        );
        let once = y.profil_ozeti(false);
        y.okunur_dizinleri_ayarla(vec![ek.path().into()]);
        assert_ne!(y.profil_ozeti(false), once);
        // Olmayan dizin özete girmez.
        y.okunur_dizinleri_ayarla(vec![ek.path().join("yok")]);
        assert_eq!(y.profil_ozeti(false), once);
    }

    #[test]
    fn ortamda_ev_ve_arac_yapilandirmasi_ozel_dizinleri_gosterir() {
        let ev = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(ev.path().join(".rustup")).unwrap();
        let ayar = LinuxAyari {
            ev: ev.path().into(),
            ..Default::default()
        };
        let ozel = OzelDizinler::yol(Path::new("/veri/yalitim"), "abc");
        let ortam = yalitim_ortami(
            &[
                ("PATH".into(), "/usr/bin".into()),
                ("HOME".into(), "/home/ada".into()),
                ("CARGO_HOME".into(), "/home/ada/.cargo".into()),
                ("TMP".into(), "/home/ada/tmp".into()),
            ],
            &ozel,
            &ayar,
        );
        let deger = |ad: &str| crate::ortam::deger(&ortam, ad).map(str::to_string);
        assert_eq!(deger("PATH").as_deref(), Some("/usr/bin"));
        assert_eq!(deger("HOME").as_deref(), Some("/veri/yalitim/abc/ev"));
        assert_eq!(deger("TMPDIR").as_deref(), Some("/tmp"));
        assert_eq!(deger("TMP"), None);
        assert_eq!(
            deger("CARGO_HOME").as_deref(),
            Some("/veri/yalitim/abc/onbellek/cargo")
        );
        assert_eq!(
            deger("RUSTUP_HOME"),
            Some(ev.path().join(".rustup").display().to_string())
        );
        assert_eq!(ortam.iter().filter(|(a, _)| a == "HOME").count(), 1);
    }

    #[test]
    fn registry_yalnizca_agsiz_ve_kendi_registry_bosken_baglanir() {
        let ev = tempfile::tempdir().unwrap();
        let veri = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(ev.path().join(".cargo/registry/cache")).unwrap();
        let ayar = LinuxAyari {
            ev: ev.path().into(),
            ..Default::default()
        };
        let ozel = OzelDizinler::yol(veri.path(), "abc");
        ozel.hazirla(&GitKimligi::default()).unwrap();
        let kullanicinin = Some(ev.path().join(".cargo/registry"));
        assert_eq!(registry_secimi(false, &ozel, &ayar), kullanicinin);
        assert_eq!(registry_secimi(true, &ozel, &ayar), None);
        // Ağlı çalıştırma çalışma alanının registry'sini doldurdu.
        std::fs::create_dir_all(ozel.cargo().join("registry/index")).unwrap();
        assert_eq!(registry_secimi(false, &ozel, &ayar), None);
    }

    #[test]
    fn program_yalnizca_gorunen_yollarda_aranir() {
        let d = tempfile::tempdir().unwrap();
        let gorunen = d.path().join("gorunen");
        let gizli = d.path().join("gizli");
        for dizin in [&gorunen, &gizli] {
            std::fs::create_dir_all(dizin).unwrap();
            let yol = dizin.join("arac");
            std::fs::write(&yol, "#!/bin/sh\n").unwrap();
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&yol, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let alan = AlanBilgisi {
            kok: d.path().join("alan"),
            gizli: Default::default(),
            korumali: Vec::new(),
            git_dizinleri: Vec::new(),
        };
        let path = std::env::join_paths([&gizli, &gorunen]).unwrap();
        let ortam = vec![("PATH".to_string(), path.to_string_lossy().into_owned())];
        let bulunan = programi_bul("arac", &ortam, std::slice::from_ref(&gorunen), &alan).unwrap();
        assert_eq!(bulunan, gorunen.join("arac"));
        match programi_bul("arac", &ortam, &[], &alan) {
            Err(BaslatmaHatasi::ProgramYok { path, .. }) => assert_eq!(path, ""),
            s => panic!("{s:?}"),
        }
    }
}
```

**`crates/kalfa-sandbox/src/lib.rs`**: şu bloğu

```rust
//! `kalfa-core`'a bağlı değildir.

mod agac;
pub mod ortam;
pub mod profil;
pub mod program;
```

şununla değiştir:

```rust
//! `kalfa-core`'a bağlı değildir.

mod agac;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod ortam;
pub mod profil;
pub mod program;
```

**`crates/kalfa-sandbox/src/lib.rs`**: şu bloğu

```rust
mod yalitimsiz;

use std::path::PathBuf;

use serde::Serialize;
```

şununla değiştir:

```rust
mod yalitimsiz;

use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
```

**`crates/kalfa-sandbox/src/lib.rs`**: şu bloğu

```rust
    fn okunur_dizinleri_ayarla(&self, _dizinler: Vec<PathBuf>) {}
}
```

şununla değiştir:

```rust
    fn okunur_dizinleri_ayarla(&self, _dizinler: Vec<PathBuf>) {}
}

/// Platformun yalıtıcısının ayarları.
pub struct PlatformAyari {
    /// Kullanıcının ev dizini; araç zinciri dizinleri buna göre bulunur.
    pub ev: PathBuf,
    /// Çalışma alanına özel dizinlerin kökü: `<veri dizini>/yalitim`.
    pub yalitim_dizini: PathBuf,
    /// Kullanıcının eklediği araç zinciri dizinleri.
    pub ek_okunur: Vec<PathBuf>,
    /// Durum sonradan değişince çağrılır.
    pub degisince: Box<dyn Fn(&YalitimDurumu) + Send + Sync>,
}

/// Platformun yalıtıcısı. Linux'ta bwrap; bwrap kullanılamıyorsa komutlar
/// yalıtımsız çalışır ve durum bunu söyler. Diğer platformlarda şimdilik
/// yalıtımsız (aşama 2d).
pub fn platform_yaliticisi(ayar: PlatformAyari) -> Arc<dyn Yalitici> {
    #[cfg(target_os = "linux")]
    {
        Arc::new(linux::LinuxYalitici::yeni(
            linux::LinuxAyari {
                ev: ayar.ev,
                yalitim_dizini: ayar.yalitim_dizini,
                ek_okunur: ayar.ek_okunur,
                git: GitKimligi::oku(),
            },
            Box::new(linux::BwrapAlgilayici),
            ayar.degisince,
        ))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = ayar;
        Arc::new(Yalitimsiz)
    }
}
```

- [ ] **Step 4: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa-sandbox --locked`
Expected: Windows'ta Görev 1'deki testler geçer. Linux'ta ayrıca `linux::` birim testleri (seccomp, bwrap seçenekleri, algılama hataları, durum ve kip, ortam, registry, program arama) geçer.

Linux'ta, root olmayan bir kullanıcıyla ve `bubblewrap` kuruluyken:

Run: `cargo test -p kalfa-sandbox --features yalitim-testleri --locked --test yalitim`
Expected: 17 test geçer. Windows'tan koşturamazsan PR'a yaz; Claude incelemede koşturur.

- [ ] **Step 5: Commit**

```bash
git add crates/kalfa-sandbox
git commit -m "sandbox: Linux arka ucu: bwrap, seccomp ve algılama"
```

---

### Task 3: Çekirdek: yalıtımın git yerleşimi

**Files:**
- Modify: `crates/kalfa-core/src/git_koruma.rs`
- Test: `crates/kalfa-core/tests/izin.rs` (değişir)

**Interfaces:**
- Consumes: `git_koruma::korumali_yollar` (değişmez; dosya araçları kullanır).
- Produces: `git_koruma::{GitYerlesimi { korumali: Vec<KorumaliYol>, dis_dizinler: Vec<PathBuf> }, KorumaliYol { yol, dizin }, git_yerlesimi(&Path) -> GitYerlesimi}`.

**Davranış notları (testler sınar):**
- Yollar kanonik ve gerçek yazımıyladır (küçük harfe çevrilmez).
- Kancalar ve `config` ortak git dizinindedir: worktree'de `gitdir`'deki `commondir`'in gösterdiği yer. Worktree'nin `config.worktree`'si varsa o da korunur.
- Çalışma alanı dışındaki git dizinlerinden yalnızca dıştaki listelenir (worktree'nin `gitdir`'i ortak dizinin altındadır).
- `core.hooksPath` hedefi çalışma alanında ya da dışarıdaki git dizininde ise korunur; `~` ile başlayan değer yok sayılır. Depo değilse yerleşim boştur.

- [ ] **Step 1: Başarısız testleri yaz**

**`crates/kalfa-core/tests/izin.rs`**: şu bloğu

```rust
    assert_eq!(s.sinifla(&y(".HUSKY/pre-push")), YolSinifi::Korumali);
    assert_eq!(s.sinifla(&y("src/husky.rs")), YolSinifi::Normal);
}
```

şununla değiştir:

```rust
    assert_eq!(s.sinifla(&y(".HUSKY/pre-push")), YolSinifi::Korumali);
    assert_eq!(s.sinifla(&y("src/husky.rs")), YolSinifi::Normal);

    // Yalıtımın yerleşiminde de mutlak ve gerçek yazımıyla bulunur.
    let kok = d.path().canonicalize().unwrap();
    let yerlesim = git_koruma::git_yerlesimi(d.path());
    assert!(yerlesim.korumali.contains(&git_koruma::KorumaliYol {
        yol: kok.join(".husky"),
        dizin: true,
    }));
}

#[test]
fn git_yerlesimi_depoda_kanca_ve_yapilandirmayi_korur() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join(".git/hooks")).unwrap();
    std::fs::write(d.path().join(".git/config"), "").unwrap();
    let kok = d.path().canonicalize().unwrap();
    let yerlesim = git_koruma::git_yerlesimi(d.path());
    assert_eq!(
        yerlesim.korumali,
        vec![
            git_koruma::KorumaliYol {
                yol: kok.join(".git/hooks"),
                dizin: true,
            },
            git_koruma::KorumaliYol {
                yol: kok.join(".git/config"),
                dizin: false,
            },
        ]
    );
    assert!(yerlesim.dis_dizinler.is_empty());

    // Depo olmayan dizinde yerleşim boştur.
    let bos = tempfile::tempdir().unwrap();
    assert_eq!(
        git_koruma::git_yerlesimi(bos.path()),
        git_koruma::GitYerlesimi::default()
    );
}

#[test]
fn git_yerlesimi_worktreede_ortak_git_dizinini_bulur() {
    let d = tempfile::tempdir().unwrap();
    let ana = d.path().join("ana/.git");
    std::fs::create_dir_all(ana.join("hooks")).unwrap();
    std::fs::create_dir_all(ana.join("worktrees/dal")).unwrap();
    std::fs::write(ana.join("config"), "").unwrap();
    std::fs::write(ana.join("worktrees/dal/commondir"), "../..\n").unwrap();
    std::fs::write(ana.join("worktrees/dal/config.worktree"), "").unwrap();
    let alan = d.path().join("dal");
    std::fs::create_dir_all(&alan).unwrap();
    std::fs::write(alan.join(".git"), "gitdir: ../ana/.git/worktrees/dal\n").unwrap();

    let ana = ana.canonicalize().unwrap();
    let yerlesim = git_koruma::git_yerlesimi(&alan);
    assert_eq!(yerlesim.dis_dizinler, vec![ana.clone()]);
    let yollar: Vec<_> = yerlesim.korumali.iter().map(|k| k.yol.clone()).collect();
    assert_eq!(
        yollar,
        vec![
            ana.join("hooks"),
            ana.join("config"),
            ana.join("worktrees/dal/config.worktree"),
        ]
    );
}
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa-core --test izin --locked`
Expected: derleme hatası (`git_koruma::git_yerlesimi` yok).

- [ ] **Step 3: Yerleşimi yaz**

**`crates/kalfa-core/src/git_koruma.rs`**: şu bloğu

```rust
//! Git'in kendiliğinden kod çalıştırdığı korumalı yollar (spec §7.3).

use std::path::Path;
use std::process::Command;

use crate::yol::CalismaYolu;

/// Çalışma alanındaki korumalı yollar; her öğe küçük harfli, göreli yoldur.
```

şununla değiştir:

```rust
//! Git'in kendiliğinden kod çalıştırdığı korumalı yollar (spec §7.3).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::yol::CalismaYolu;

/// Yalıtımın git yerleşimi (aşama 2 tasarımı §7.2, §7.4): korumalı yolların
/// mutlak ve gerçek yazımlı hâli, çalışma alanı dışındaki git dizinleri.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitYerlesimi {
    /// Yalıtımda salt okunur kalacak yollar.
    pub korumali: Vec<KorumaliYol>,
    /// Çalışma alanı dışındaki git dizinleri (worktree, submodule). Yalıtımda
    /// yazılabilir bağlanır: yazılan yer çalışma alanının kendi deposudur.
    pub dis_dizinler: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KorumaliYol {
    pub yol: PathBuf,
    pub dizin: bool,
}

/// Çalışma alanının git yerleşimi. Git deposu değilse boştur.
///
/// - Kancalar ve yapılandırma ortak git dizinindedir (worktree'de
///   `commondir`'in gösterdiği yer); worktree'nin kendi `config.worktree`'si
///   varsa o da korunur.
/// - `core.hooksPath` hedefi yalıtımda görünen bir yerdeyse korunur.
pub fn git_yerlesimi(kok: &Path) -> GitYerlesimi {
    let Ok(kok) = kok.canonicalize() else {
        return GitYerlesimi::default();
    };
    let mut y = GitYerlesimi::default();
    if let Some((gitdir, ortak)) = git_dizinleri(&kok) {
        y.korumali.push(KorumaliYol {
            yol: ortak.join("hooks"),
            dizin: true,
        });
        y.korumali.push(KorumaliYol {
            yol: ortak.join("config"),
            dizin: false,
        });
        let calisma_agaci = gitdir.join("config.worktree");
        if gitdir != ortak && calisma_agaci.is_file() {
            y.korumali.push(KorumaliYol {
                yol: calisma_agaci,
                dizin: false,
            });
        }
        let disarida: Vec<PathBuf> = [ortak, gitdir]
            .into_iter()
            .filter(|d| !d.starts_with(&kok))
            .collect();
        // İç içe olanlardan dıştaki yeter (worktree'nin gitdir'i ortak dizinin
        // altındadır).
        for d in &disarida {
            let kapsanir = disarida.iter().any(|o| o != d && d.starts_with(o));
            if !kapsanir && !y.dis_dizinler.contains(d) {
                y.dis_dizinler.push(d.clone());
            }
        }
    }
    if let Some(hooks) = hooks_path_mutlak(&kok) {
        let gorunur =
            hooks.starts_with(&kok) || y.dis_dizinler.iter().any(|d| hooks.starts_with(d));
        if gorunur && !y.korumali.iter().any(|k| k.yol == hooks) {
            y.korumali.push(KorumaliYol {
                yol: hooks,
                dizin: true,
            });
        }
    }
    y
}

/// Kanonik (gitdir, ortak git dizini).
fn git_dizinleri(kok: &Path) -> Option<(PathBuf, PathBuf)> {
    let nokta_git = kok.join(".git");
    let gitdir = if nokta_git.is_dir() {
        nokta_git
    } else {
        let icerik = std::fs::read_to_string(&nokta_git).ok()?;
        let hedef = icerik
            .lines()
            .find_map(|s| s.trim().strip_prefix("gitdir:"))?
            .trim()
            .to_string();
        kok.join(hedef).canonicalize().ok()?
    };
    let ortak = std::fs::read_to_string(gitdir.join("commondir"))
        .ok()
        .and_then(|m| gitdir.join(m.trim()).canonicalize().ok())
        .unwrap_or_else(|| gitdir.clone());
    Some((gitdir, ortak))
}

/// `core.hooksPath`'in mutlak hâli; göreli değer çalışma ağacının köküne
/// göredir. `~` ile başlayan değer ev dizinindedir ve yalıtımda görünmez.
fn hooks_path_mutlak(kok: &Path) -> Option<PathBuf> {
    let deger = hooks_path_degeri(kok)?;
    if deger.starts_with('~') {
        return None;
    }
    let yol = kok.join(&deger);
    Some(yol.canonicalize().unwrap_or(yol))
}

/// Çalışma alanındaki korumalı yollar; her öğe küçük harfli, göreli yoldur.
```

**`crates/kalfa-core/src/git_koruma.rs`**: şu bloğu

```rust
}

/// `core.hooksPath`'in etkin değeri (depo + kullanıcı yapılandırması).
/// `git config` yapılandırmayı yalnızca okur; kanca ya da fsmonitor çalıştırmaz.
fn hooks_path(kok: &Path) -> Option<String> {
    let cikti = Command::new("git")
        .arg("-C")
```

şununla değiştir:

```rust
}

/// `core.hooksPath`'in çalışma alanındaki göreli yolu.
fn hooks_path(kok: &Path) -> Option<String> {
    let deger = hooks_path_degeri(kok)?;
    if deger.starts_with('~') {
        return None;
    }
    goreli_yap(kok, Path::new(&deger))
}

/// `core.hooksPath`'in etkin değeri (depo + kullanıcı yapılandırması).
/// `git config` yapılandırmayı yalnızca okur; kanca ya da fsmonitor çalıştırmaz.
fn hooks_path_degeri(kok: &Path) -> Option<String> {
    let cikti = Command::new("git")
        .arg("-C")
```

**`crates/kalfa-core/src/git_koruma.rs`**: şu bloğu

```rust
    }
    let deger = String::from_utf8_lossy(&cikti.stdout).trim().to_string();
    if deger.is_empty() || deger.starts_with('~') {
        return None;
    }
    goreli_yap(kok, Path::new(&deger))
}
```

şununla değiştir:

```rust
    }
    let deger = String::from_utf8_lossy(&cikti.stdout).trim().to_string();
    (!deger.is_empty()).then_some(deger)
}
```

- [ ] **Step 4: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa-core --test izin --locked`
Expected: bütün testler geçer (yeni: depo, worktree ve `core.hooksPath` yerleşimi). Görevin sonundaki genel denetimleri de koştur.

- [ ] **Step 5: Commit**

```bash
git add crates/kalfa-core
git commit -m "çekirdek: yalıtımın git yerleşimi"
```

---

### Task 4: `kalfa-tools`: kabuk yalıtımda

**Files:**
- Modify: `crates/kalfa-tools/src/kabuk/mod.rs`
- Test: `crates/kalfa-tools/tests/kabuk.rs` (değişir); `crates/kalfa-tools/tests/uyumluluk.rs` ve `crates/kalfa-tools/tests/ornekler/{rust,node}/` (yeni)

**Interfaces:**
- Consumes: Görev 1–3 (`AlanBilgisi`, `tarama::tara`, `git_koruma::git_yerlesimi`, `Yalitici::profil_ozeti`/`yeniden_algila`).
- Produces: davranış; genel arayüz değişmez.

**Davranış notları (testler sınar):**
- **Tarif:** yalıtım hazırsa `YalitimOzeti::Yalitimli { arka_uc, profil }`'deki `profil` yalıtıcının profil özetidir (ağ bayrağıyla); yoksa sürüm.
- **Çalıştırma:** yalıtım hazırsa araç, çalıştırmadan hemen önce `spawn_blocking`'de çalışma alanını tarar (gizli dosya sınıflandırıcısıyla) ve git yerleşimini çözer; `CalistirmaIstegi.alan`'ı doldurur. Çalışma dizini ve çalışma alanındaki program kanonik kökten kurulur.
- **Yeniden algılama:** başlatma `BaslatmaHatasi::Yalitim` ile başarısız olursa ya da yalıtımlı komut çıkış kodu 1 ile bitip çıktısı `bwrap: ` ile başlıyorsa `yeniden_algila` çağrılır.
- **Uyumluluk (Linux, `yalitim-testleri`):** git worktree'de commit, `cargo test --offline` (önceden indirilmiş `hex` ile), `npm ci` (ağlı) ve `npm test`, `python3 -c`, kabuk aracının bütün yolundan geçerek çalışır. Örnek projeler derlemeye gömülür (`include_str!`).

- [ ] **Step 1: Başarısız testleri yaz**

**`crates/kalfa-tools/tests/kabuk.rs`**: şu bloğu

```rust
use kalfa_core::izin::Siniflandirici;
use kalfa_core::kabuk::{KabukBicimi, YalitimOzeti};
use kalfa_core::kimlik::CagriId;
use kalfa_core::protokol::CiktiAkisi;
use kalfa_sandbox::Yalitimsiz;
use kalfa_tools::{CalismaAlani, KabukAraci, KabukGuveni};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;
```

şununla değiştir:

```rust
use kalfa_core::izin::Siniflandirici;
use kalfa_core::kabuk::{KabukBicimi, YalitimOzeti};
use kalfa_core::kimlik::CagriId;
use kalfa_core::protokol::CiktiAkisi;
use kalfa_sandbox::{BaslatmaHatasi, CalistirmaIstegi, Surec, Yalitici, YalitimDurumu, Yalitimsiz};
use kalfa_tools::{CalismaAlani, KabukAraci, KabukGuveni};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;
```

**`crates/kalfa-tools/tests/kabuk.rs`**: şu bloğu

```rust
        "başlamayan komut güven kaydetmez"
    );
}
```

şununla değiştir:

```rust
        "başlamayan komut güven kaydetmez"
    );
}

/// Yalıtım hazırmış gibi davranan yalıtıcı: isteği kaydeder, komutu
/// yalıtımsız çalıştırır. `basarisiz` ise yalıtım başlatılamaz.
#[derive(Default)]
struct KayitliYalitici {
    istekler: std::sync::Mutex<Vec<CalistirmaIstegi>>,
    algilama: AtomicUsize,
    basarisiz: AtomicBool,
}

impl Yalitici for KayitliYalitici {
    fn durum(&self) -> YalitimDurumu {
        YalitimDurumu::Hazir {
            arka_uc: "bwrap".into(),
            surum: "0.9.0".into(),
        }
    }

    fn profil_ozeti(&self, ag: bool) -> Option<String> {
        Some(format!("ozet-{ag}"))
    }

    fn baslat(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi> {
        self.istekler.lock().unwrap().push(istek.clone());
        if self.basarisiz.load(Ordering::SeqCst) {
            return Err(BaslatmaHatasi::Yalitim("deneme".into()));
        }
        Yalitimsiz.baslat(&CalistirmaIstegi {
            dizin: std::env::temp_dir(),
            alan: None,
            ..istek.clone()
        })
    }

    fn yeniden_algila(&self) -> YalitimDurumu {
        self.algilama.fetch_add(1, Ordering::SeqCst);
        self.durum()
    }
}

fn kur_yalitimli() -> (TempDir, KabukAraci, Arc<KayitliYalitici>) {
    let d = TempDir::new().unwrap();
    let s = Arc::new(Siniflandirici::yeni(d.path(), &[]).unwrap());
    let alan = CalismaAlani::ac(s).unwrap();
    let yalitici = Arc::new(KayitliYalitici::default());
    let arac = KabukAraci::yeni(alan, yalitici.clone(), Arc::new(SahteGuven::default()));
    (d, arac, yalitici)
}

#[tokio::test]
async fn yalitim_hazirsa_tarif_profil_ozetini_tasir_ve_alan_bilgisi_verilir() {
    let (d, arac, yalitici) = kur_yalitimli();
    std::fs::create_dir_all(d.path().join(".git/hooks")).unwrap();
    std::fs::write(d.path().join(".git/config"), "").unwrap();
    std::fs::create_dir_all(d.path().join("alt")).unwrap();
    std::fs::write(d.path().join(".env"), "x").unwrap();
    std::fs::write(d.path().join("alt/sunucu.pem"), "x").unwrap();

    let t = arac
        .hazirla(&json!({"betik": "echo a"}))
        .await
        .unwrap()
        .kabuk
        .unwrap();
    assert_eq!(
        t.yalitim,
        YalitimOzeti::Yalitimli {
            arka_uc: "bwrap".into(),
            profil: "ozet-false".into(),
        }
    );
    let t = arac
        .hazirla(&json!({"betik": "echo a", "ag": true}))
        .await
        .unwrap()
        .kabuk
        .unwrap();
    assert!(matches!(t.yalitim, YalitimOzeti::Yalitimli { profil, .. } if profil == "ozet-true"));

    let yazdir = if cfg!(windows) {
        "Write-Output tamam"
    } else {
        "echo tamam"
    };
    let (sonuc, _) = calistir(&arac, &json!({"betik": yazdir, "dizin": "alt"})).await;
    assert!(sonuc.unwrap().contains("tamam"));
    let istek = yalitici.istekler.lock().unwrap().pop().unwrap();
    let kok = d.path().canonicalize().unwrap();
    let alan = istek.alan.expect("yalıtımlı istek alan bilgisi taşır");
    assert_eq!(alan.kok, kok);
    assert_eq!(istek.dizin, kok.join("alt"));
    assert_eq!(
        alan.gizli.dosyalar,
        vec![kok.join(".env"), kok.join("alt").join("sunucu.pem")]
    );
    let korumali: Vec<_> = alan.korumali.iter().map(|k| k.yol.clone()).collect();
    assert_eq!(
        korumali,
        vec![
            kok.join(".git").join("hooks"),
            kok.join(".git").join("config")
        ]
    );
    assert!(alan.git_dizinleri.is_empty());
}

#[tokio::test]
async fn yalitim_baslatilamazsa_algilama_yeniden_calisir() {
    let (_d, arac, yalitici) = kur_yalitimli();
    yalitici.basarisiz.store(true, Ordering::SeqCst);
    let (sonuc, _) = calistir(&arac, &json!({"betik": "echo a"})).await;
    match sonuc {
        Err(AracHatasi::Gecersiz(m)) => assert!(m.contains("Yalıtım başlatılamadı"), "{m}"),
        diger => panic!("{diger:?}"),
    }
    assert_eq!(yalitici.algilama.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn bwrap_kurulum_hatasi_algilamayi_yeniden_calistirir() {
    let (_d, arac, yalitici) = kur_yalitimli();
    let (sonuc, _) = calistir(
        &arac,
        &betik(
            "echo 'bwrap: Can not mount tmpfs' >&2; exit 1",
            "[Console]::Error.WriteLine('bwrap: Can not mount tmpfs'); exit 1",
        ),
    )
    .await;
    assert!(matches!(sonuc, Err(AracHatasi::Basarisiz(_))), "{sonuc:?}");
    assert_eq!(yalitici.algilama.load(Ordering::SeqCst), 1);
    // Çıkış kodu 1 ama bwrap öneki yoksa algılama çalışmaz.
    let _ = calistir(
        &arac,
        &betik(
            "echo hata >&2; exit 1",
            "[Console]::Error.WriteLine('hata'); exit 1",
        ),
    )
    .await;
    assert_eq!(yalitici.algilama.load(Ordering::SeqCst), 1);
}
```

**`crates/kalfa-tools/tests/uyumluluk.rs`** (tam içerik):

````rust
//! Araç zinciri uyumluluğu (aşama 2 tasarımı §12.3): `git`, `cargo`, `npm` ve
//! `python` yalıtımın içinde, kabuk aracının bütün yolundan (gizli dosya
//! taraması, git yerleşimi, bwrap) geçerek çalışır. Kullanıcının gerçek ev
//! dizinindeki araç zincirini kullanır; yalıtım yoksa atlanmaz, başarısız olur:
//!
//! ```text
//! cargo test -p kalfa-tools --features yalitim-testleri --test uyumluluk
//! ```
#![cfg(all(target_os = "linux", feature = "yalitim-testleri"))]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use kalfa_core::arac::{Arac, AracHatasi, CiktiGonderici};
use kalfa_core::izin::Siniflandirici;
use kalfa_sandbox::linux::{BwrapAlgilayici, LinuxAyari, LinuxYalitici};
use kalfa_sandbox::{Yalitici, YalitimDurumu};
use kalfa_tools::{CalismaAlani, KabukAraci, KabukGuveni};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct Guven(AtomicBool);

impl KabukGuveni for Guven {
    fn guvenildi_mi(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
    fn guvenildi(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// Testlerin `node`'u sistem dizinlerinde değilse (örneğin /opt/node) yalıtıma
/// açılır; kullanıcı bunu Ayarlar'dan yapar.
fn node_koku() -> Option<PathBuf> {
    let cikti = Command::new("sh")
        .args(["-c", "command -v node"])
        .output()
        .ok()?;
    let node = PathBuf::from(String::from_utf8_lossy(&cikti.stdout).trim())
        .canonicalize()
        .ok()?;
    let kok = node.parent()?.parent()?.to_path_buf();
    (!kok.starts_with("/usr")).then_some(kok)
}

struct Kurgu {
    _veri: TempDir,
    arac: KabukAraci,
}

fn kurgu(kok: &Path) -> Kurgu {
    let veri = TempDir::new().unwrap();
    let yalitici = LinuxYalitici::yeni(
        LinuxAyari {
            ev: std::env::var_os("HOME").map(PathBuf::from).expect("HOME"),
            yalitim_dizini: veri.path().join("yalitim"),
            ek_okunur: node_koku().into_iter().collect(),
            git: Default::default(),
        },
        Box::new(BwrapAlgilayici),
        Box::new(|_| {}),
    );
    match yalitici.durum() {
        YalitimDurumu::Hazir { .. } => {}
        d => panic!("yalıtım hazır değil: {d:?}"),
    }
    let s = Arc::new(Siniflandirici::yeni(kok, &[]).unwrap());
    let alan = CalismaAlani::ac(s).unwrap();
    Kurgu {
        _veri: veri,
        arac: KabukAraci::yeni(alan, Arc::new(yalitici), Arc::new(Guven::default())),
    }
}

impl Kurgu {
    /// Komutu çalıştırır; başarılıysa çıktıyı, değilse hatayı panikle verir.
    async fn calistir(&self, a: Value) -> String {
        let h = self.arac.hazirla(&a).await.expect("hazırlık");
        match self
            .arac
            .calistir_akisli(&a, &h, CancellationToken::new(), CiktiGonderici::bos())
            .await
        {
            Ok(c) => c.icerik,
            Err(AracHatasi::Basarisiz(c)) => panic!("{a}\n{}", c.icerik),
            Err(e) => panic!("{a}\n{e:?}"),
        }
    }
}

fn betik(metin: &str) -> Value {
    json!({ "betik": metin, "zaman_asimi_sn": 600 })
}

fn git_host(dizin: &Path, args: &[&str]) {
    let durum = Command::new("git")
        .arg("-C")
        .arg(dizin)
        .args([
            "-c",
            "user.name=Kalfa",
            "-c",
            "user.email=kalfa@example.com",
        ])
        .args(args)
        .status()
        .unwrap();
    assert!(durum.success(), "git {args:?}");
}

/// Örnek projeler derlemeye gömülür: testi koşan kullanıcı kaynak ağacını
/// okuyamayabilir.
const RUST_ORNEGI: &[(&str, &str)] = &[
    ("Cargo.toml", include_str!("ornekler/rust/Cargo.toml")),
    ("Cargo.lock", include_str!("ornekler/rust/Cargo.lock")),
    ("src/lib.rs", include_str!("ornekler/rust/src/lib.rs")),
];
const NODE_ORNEGI: &[(&str, &str)] = &[
    ("package.json", include_str!("ornekler/node/package.json")),
    (
        "package-lock.json",
        include_str!("ornekler/node/package-lock.json"),
    ),
];

fn ornek_yaz(dosyalar: &[(&str, &str)], hedef: &Path) {
    for (ad, icerik) in dosyalar {
        let yol = hedef.join(ad);
        std::fs::create_dir_all(yol.parent().unwrap()).unwrap();
        std::fs::write(yol, icerik).unwrap();
    }
}

#[tokio::test]
async fn git_worktreede_durum_fark_ekleme_ve_commit() {
    let d = TempDir::new().unwrap();
    let ana = d.path().join("ana");
    std::fs::create_dir_all(&ana).unwrap();
    git_host(&ana, &["init", "-q", "-b", "ana"]);
    std::fs::write(ana.join("benioku.md"), "ilk\n").unwrap();
    git_host(&ana, &["add", "."]);
    git_host(&ana, &["commit", "-q", "-m", "ilk"]);
    git_host(&ana, &["worktree", "add", "-q", "-b", "dal", "../dal"]);
    let dal = d.path().join("dal");

    let k = kurgu(&dal);
    let cikti = k
        .calistir(betik(
            "echo ikinci >> benioku.md && git status --short && git diff --stat && \
             git add benioku.md && \
             git -c user.name=Kalfa -c user.email=kalfa@example.com commit -q -m yalitimdan && \
             git log --oneline -1",
        ))
        .await;
    assert!(cikti.contains("benioku.md"), "{cikti}");
    assert!(cikti.contains("yalitimdan"), "{cikti}");
    // Commit ana deponun nesnelerine yazıldı.
    let log = Command::new("git")
        .args(["-C"])
        .arg(&ana)
        .args(["log", "--oneline", "dal", "-1"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&log.stdout).contains("yalitimdan"));
}

#[tokio::test]
async fn cargo_agsiz_onceden_indirilmis_crate_ile_derler_ve_test_eder() {
    let d = TempDir::new().unwrap();
    ornek_yaz(RUST_ORNEGI, d.path());
    let k = kurgu(d.path());
    let cikti = k
        .calistir(json!({
            "program": "cargo",
            "argumanlar": ["test", "--offline", "--locked"],
            "zaman_asimi_sn": 600
        }))
        .await;
    assert!(cikti.contains("test result: ok. 1 passed"), "{cikti}");
}

#[tokio::test]
async fn npm_ci_agli_ve_npm_test_agsiz_calisir() {
    let d = TempDir::new().unwrap();
    ornek_yaz(NODE_ORNEGI, d.path());
    let k = kurgu(d.path());
    let ci = k
        .calistir(json!({
            "program": "npm",
            "argumanlar": ["ci", "--no-audit", "--no-fund"],
            "ag": true,
            "zaman_asimi_sn": 600
        }))
        .await;
    assert!(ci.contains("Çıkış kodu: 0"), "{ci}");
    let test = k
        .calistir(json!({ "program": "npm", "argumanlar": ["test"] }))
        .await;
    assert!(test.contains("npm-test-ok"), "{test}");
}

#[tokio::test]
async fn python_calisir() {
    let d = TempDir::new().unwrap();
    let k = kurgu(d.path());
    let cikti = k
        .calistir(json!({
            "program": "python3",
            "argumanlar": ["-c", "import json, sqlite3, ssl; print('py-ok')"]
        }))
        .await;
    assert!(cikti.contains("py-ok"), "{cikti}");
}
````

**`crates/kalfa-tools/tests/ornekler/rust/Cargo.toml`** (tam içerik):

```toml
# Uyumluluk testinin örnek crate'i (aşama 2 tasarımı §12.3): yalıtımda,
# ağsız, önceden indirilmiş bir bağımlılıkla derlenir. Çalışma alanının
# parçası değildir.
[package]
name = "kalfa-ornek"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
hex = "0.4.3"

[workspace]
```

**`crates/kalfa-tools/tests/ornekler/rust/Cargo.lock`** (tam içerik):

```toml
# This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 4

[[package]]
name = "hex"
version = "0.4.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7f24254aa9a54b5c858eaee2f5bccdb46aaf0e486a595ed5fd8f86ba55232a70"

[[package]]
name = "kalfa-ornek"
version = "0.1.0"
dependencies = [
 "hex",
]
```

**`crates/kalfa-tools/tests/ornekler/rust/src/lib.rs`** (tam içerik):

```rust
//! Uyumluluk testinin örnek crate'i.

pub fn onaltilik(veri: &[u8]) -> String {
    hex::encode(veri)
}

#[cfg(test)]
mod tests {
    #[test]
    fn onaltilik_yazar() {
        assert_eq!(super::onaltilik(b"kalfa"), "6b616c6661");
    }
}
```

**`crates/kalfa-tools/tests/ornekler/node/package.json`** (tam içerik):

```json
{
  "name": "kalfa-ornek",
  "version": "1.0.0",
  "private": true,
  "scripts": {
    "test": "node -e \"console.log('npm-test-ok')\""
  }
}
```

**`crates/kalfa-tools/tests/ornekler/node/package-lock.json`** (tam içerik):

```json
{
  "name": "kalfa-ornek",
  "version": "1.0.0",
  "lockfileVersion": 3,
  "requires": true,
  "packages": {
    "": {
      "name": "kalfa-ornek",
      "version": "1.0.0"
    }
  }
}
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa-tools --test kabuk --locked`
Expected: yeni testler başarısız (tarif profil özeti yerine sürümü taşıyor, istek alan bilgisi taşımıyor, algılama yeniden çalışmıyor).

- [ ] **Step 3: Aracı yaz**

**`crates/kalfa-tools/src/kabuk/mod.rs`**: şu bloğu

```rust
pub mod cikti;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
```

şununla değiştir:

```rust
pub mod cikti;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
```

**`crates/kalfa-tools/src/kabuk/mod.rs`**: şu bloğu

```rust
use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, CiktiGonderici, Hazirlik};
use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
use kalfa_core::yol::CalismaYolu;
use kalfa_sandbox::{
    Bitis, CalistirmaIstegi, KomutBicimi, Yalitici, YalitimDurumu, ortam, program,
};
use serde_json::{Value, json};
```

şununla değiştir:

```rust
use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, CiktiGonderici, Hazirlik};
use kalfa_core::git_koruma;
use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
use kalfa_core::yol::CalismaYolu;
use kalfa_sandbox::{
    AlanBilgisi, BaslatmaHatasi, Bitis, CalistirmaIstegi, KomutBicimi, Korumali, Yalitici,
    YalitimDurumu, ortam, program, tarama,
};
use serde_json::{Value, json};
```

**`crates/kalfa-tools/src/kabuk/mod.rs`**: şu bloğu

```rust
}

fn yalitim_ozeti(durum: YalitimDurumu) -> YalitimOzeti {
    match durum {
        YalitimDurumu::Hazir { arka_uc, surum } => YalitimOzeti::Yalitimli {
            arka_uc,
            profil: surum,
        },
        YalitimDurumu::Yok { talimat, .. } => YalitimOzeti::Yalitimsiz { talimat },
    }
}
```

şununla değiştir:

```rust
}

impl KabukAraci {
    /// Tarifteki yalıtım: arka uç ve profil özeti (§7.1). Özet mühre girer;
    /// kullanıcı bir dizin eklerse bekleyen onay geçersizleşir.
    fn yalitim_ozeti(&self, ag: bool) -> YalitimOzeti {
        match self.yalitici.durum() {
            YalitimDurumu::Hazir { arka_uc, surum } => YalitimOzeti::Yalitimli {
                arka_uc,
                profil: self.yalitici.profil_ozeti(ag).unwrap_or(surum),
            },
            YalitimDurumu::Yok { talimat, .. } => YalitimOzeti::Yalitimsiz { talimat },
        }
    }

    /// Yalıtımın çalışma alanı bilgisi; her çalıştırmadan hemen önce üretilir
    /// (§7.2): gizli dosya taraması ve git yerleşimi.
    async fn alan_bilgisi(&self) -> Result<AlanBilgisi, AracHatasi> {
        let alan = self.alan.clone();
        tokio::task::spawn_blocking(move || {
            let kok = alan
                .kok()
                .canonicalize()
                .map_err(|e| gecersiz(format!("Çalışma alanının yolu çözülemedi: {e}")))?;
            let gizli = tarama::tara(&kok, &|yol, _dizin| {
                CalismaYolu::coz(&yol.to_string_lossy())
                    .map(|y| alan.siniflandirici().gizli_mi(&y))
                    .unwrap_or(false)
            });
            let git = git_koruma::git_yerlesimi(&kok);
            Ok(AlanBilgisi {
                kok,
                gizli,
                korumali: git
                    .korumali
                    .into_iter()
                    .map(|k| Korumali {
                        yol: k.yol,
                        dizin: k.dizin,
                    })
                    .collect(),
                git_dizinleri: git.dis_dizinler,
            })
        })
        .await
        .map_err(|e| gecersiz(format!("Gizli dosya taraması yarıda kaldı: {e}")))?
    }
}
```

**`crates/kalfa-tools/src/kabuk/mod.rs`**: şu bloğu

```rust
                    .map_or_else(|| ".".to_string(), |d| d.as_str().to_string()),
                ag: istek.ag,
                yalitim: yalitim_ozeti(self.yalitici.durum()),
                ilk_cagri: !self.guven.guvenildi_mi(),
            }),
```

şununla değiştir:

```rust
                    .map_or_else(|| ".".to_string(), |d| d.as_str().to_string()),
                ag: istek.ag,
                yalitim: self.yalitim_ozeti(istek.ag),
                ilk_cagri: !self.guven.guvenildi_mi(),
            }),
```

**`crates/kalfa-tools/src/kabuk/mod.rs`**: şu bloğu

```rust
        }
        let istek = self.coz(a)?;
        let calistirma = CalistirmaIstegi {
            komut: istek.komut,
            dizin: self.calisma_dizini(&istek.dizin),
            ortam: ortam::uret(std::env::vars()),
            ag: istek.ag,
            alan: None,
        };
        let surec = self
            .yalitici
            .baslat(&calistirma)
            .map_err(|e| gecersiz(e.to_string()))?;
        // Komut başladı; çalışma alanındaki ilk onay kaydedilir.
        self.guven.guvenildi();
```

şununla değiştir:

```rust
        }
        let istek = self.coz(a)?;
        // Yalıtım hazırsa komut yalnızca yalıtımda çalışır; yalıtıcı, isteğin
        // onaylandığı kiple şimdiki durumu karşılaştırır.
        let alan = if self.yalitici.durum().hazir_mi() {
            Some(self.alan_bilgisi().await?)
        } else {
            None
        };
        let dizin = match (&alan, &istek.dizin) {
            (Some(b), Some(d)) => b.kok.join(d.std_yolu()),
            (Some(b), None) => b.kok.clone(),
            (None, _) => self.calisma_dizini(&istek.dizin),
        };
        let yalitimli = alan.is_some();
        // Yalıtımda çalışma alanı kanonik yolunda bağlanır; çalışma alanındaki
        // program da o yoldan çağrılır.
        let mut komut = istek.komut;
        if let (Some(b), KomutBicimi::Program { program, .. }) = (&alan, &mut komut)
            && let Ok(goreli) = Path::new(program.as_str()).strip_prefix(self.alan.kok())
        {
            *program = b.kok.join(goreli).display().to_string();
        }
        let calistirma = CalistirmaIstegi {
            komut,
            dizin,
            ortam: ortam::uret(std::env::vars()),
            ag: istek.ag,
            alan,
        };
        let surec = self.yalitici.baslat(&calistirma).map_err(|e| {
            if matches!(e, BaslatmaHatasi::Yalitim(_)) {
                self.yalitici.yeniden_algila();
            }
            gecersiz(e.to_string())
        })?;
        // Komut başladı; çalışma alanındaki ilk onay kaydedilir.
        self.guven.guvenildi();
```

**`crates/kalfa-tools/src/kabuk/mod.rs`**: şu bloğu

```rust
        };
        let sure = format!("{:.1}", sonuc.sure.as_secs_f64()).replace('.', ",");
        let govde = if tampon.toplam() == 0 {
            "--- çıktı yok ---".to_string()
        } else {
            format!("--- çıktı ---\n{}", tampon.metin())
        };
        let cikti = AracCiktisi {
```

şununla değiştir:

```rust
        };
        let sure = format!("{:.1}", sonuc.sure.as_secs_f64()).replace('.', ",");
        let metin = tampon.metin();
        // bwrap kurulum hatasını "bwrap: " önekiyle yazar ve 1 ile çıkar
        // (§11): yalıtım bozulmuş olabilir, algılama yeniden çalışır.
        if yalitimli && sonuc.bitis == Bitis::Cikti(Some(1)) && metin.starts_with("bwrap: ") {
            self.yalitici.yeniden_algila();
        }
        let govde = if tampon.toplam() == 0 {
            "--- çıktı yok ---".to_string()
        } else {
            format!("--- çıktı ---\n{metin}")
        };
        let cikti = AracCiktisi {
```

- [ ] **Step 4: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa-tools --test kabuk --locked`
Expected: 12 test geçer. Görevin sonundaki genel denetimleri de koştur.

Linux'ta, root olmayan bir kullanıcıyla:

Run: `cargo test -p kalfa-tools --features yalitim-testleri --locked --test uyumluluk`
Expected: 4 test geçer. Windows'tan koşturamazsan PR'a yaz; Claude incelemede koşturur.

- [ ] **Step 5: Commit**

```bash
git add crates/kalfa-tools
git commit -m "araçlar: kabuk yalıtımda: tarama, git yerleşimi ve uyumluluk testleri"
```

---

### Task 5: Uygulama: platform yalıtıcısı, araç zinciri dizinleri, yalıtım kanalı

**Files:**
- Create: `crates/kalfa-uygulama/src/yalitim.rs`, `crates/kalfa-uygulama/tests/yalitim.rs`
- Modify: `crates/kalfa-uygulama/src/{arayuz,calisma_alanlari,lib,oturumlar,sozlesme,test_destek,uygulama}.rs`, `crates/kalfa-uygulama/src/e2e/mod.rs`, `crates/kalfa/src/{iletisim,kanal,komut_adlari,komutlar,lib}.rs`, `crates/kalfa/capabilities/ana-pencere.json`
- Test: `crates/kalfa-uygulama/tests/{e2e_parcalari,sart_akisi}.rs` (değişir)

**Interfaces:**
- Consumes: Görev 2 (`platform_yaliticisi`, `PlatformAyari`), Görev 1 (`alan_kimligi`).
- Produces:
  - `yalitim::{YalitimKaynagi::{Platform, Kurucu(Box<dyn FnOnce(Degisince) -> Arc<dyn Yalitici> + Send>)}, Degisince, AracZinciriKaydi}`.
  - `Bilesenler.ev_dizini: PathBuf`, `Bilesenler.yalitim: YalitimKaynagi`.
  - `arayuz::YalitimYazici` (`gonder(&YalitimDurumu) -> Result<(), String>`), `arayuz::YalitimDurumu` (yeniden dışa açılır); `Iletisim::klasor_sec(&self, baslik: &str)`.
  - `Uygulama::{yalitimi_dinle(Box<dyn YalitimYazici>), yalitimi_algila() -> Result<YalitimDurumu, KomutHatasi>, arac_zinciri_dizinleri() -> Vec<String>, arac_zinciri_dizini_ekle() -> Result<Vec<String>, KomutHatasi>, arac_zinciri_dizini_kaldir(&str) -> Result<Vec<String>, KomutHatasi>}`.
  - `UygulamaDurumu.platform: String` (`std::env::consts::OS`).
  - `test_destek::{test_uygulamasi_yalitimla, SahteYalitimYazici}`.
  - Komutlar (ana pencere): `yalitimi_dinle(kanal)`, `yalitimi_algila`, `arac_zinciri_dizinleri`, `arac_zinciri_dizini_ekle`, `arac_zinciri_dizini_kaldir(yol)`.

**Davranış notları (testler sınar):**
- **Kanal:** `yalitimi_dinle` şimdiki durumu hemen yazar; sonraki değişimler aynı kanala gider; yeni kanal eskisinin yerini alır. Değişmeyen durum yazılmaz.
- **Araç zinciri dizinleri:** veri dizinindeki `yalitim.json`'da; ekleme yerel klasör seçiciyle ("Yalıtıma açılacak araç zinciri dizinini seçin"). Vazgeçilirse liste değişmez; aynı dizin iki kez eklenmez; kayıt kalıcıdır; ekleme ve çıkarma yalıtıcıya bildirilir. Ev dizinini ya da veri dizinini kapsayan, veri dizininin içindeki dizin `GecersizGirdi` ile reddedilir.
- **Çalışma alanı kimliği:** `CalismaAlaniKaydi::kimlik` `kalfa_sandbox::alan_kimligi`'ni kullanır (değer değişmez).
- **Uçtan uca derleme** de platformun yalıtıcısını kullanır; Linux'ta komutlar gerçek yalıtımda çalışır.

- [ ] **Step 1: Başarısız testleri yaz**

**`crates/kalfa-uygulama/tests/yalitim.rs`** (tam içerik):

```rust
//! Yalıtımın uygulama tarafı (aşama 2 tasarımı §8.3, §10.3–§10.5): durumun
//! ana pencereye yayılması, araç zinciri dizinleri.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use kalfa_core::test_destek::SahteSaglayici;
use kalfa_sandbox::{
    BaslatmaHatasi, CalistirmaIstegi, Surec, Yalitici, YalitimDurumu, YalitimYoklukNedeni,
    Yalitimsiz,
};
use kalfa_uygulama::hata::HataKodu;
use kalfa_uygulama::test_destek::{SahteYalitimYazici, TestUygulamasi, test_uygulamasi_yalitimla};
use kalfa_uygulama::yalitim::{Degisince, YalitimKaynagi};

/// Durumu testin verdiği sıradaki değere geçen yalıtıcı.
struct DegisenYalitici {
    durum: Mutex<YalitimDurumu>,
    sonraki: Mutex<Option<YalitimDurumu>>,
    degisince: Mutex<Option<Degisince>>,
    dizinler: Mutex<Vec<PathBuf>>,
}

impl Yalitici for DegisenYalitici {
    fn durum(&self) -> YalitimDurumu {
        self.durum.lock().unwrap().clone()
    }

    fn baslat(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi> {
        Yalitimsiz.baslat(istek)
    }

    fn yeniden_algila(&self) -> YalitimDurumu {
        if let Some(yeni) = self.sonraki.lock().unwrap().take() {
            let degisti = *self.durum.lock().unwrap() != yeni;
            *self.durum.lock().unwrap() = yeni.clone();
            if degisti && let Some(f) = self.degisince.lock().unwrap().as_ref() {
                f(&yeni);
            }
        }
        self.durum()
    }

    fn okunur_dizinleri_ayarla(&self, dizinler: Vec<PathBuf>) {
        *self.dizinler.lock().unwrap() = dizinler;
    }
}

fn yok() -> YalitimDurumu {
    YalitimDurumu::Yok {
        neden: YalitimYoklukNedeni::BwrapYok,
        talimat: "bubblewrap kur".into(),
    }
}

fn hazir() -> YalitimDurumu {
    YalitimDurumu::Hazir {
        arka_uc: "bwrap".into(),
        surum: "0.9.0".into(),
    }
}

fn kur(dizin: &Path) -> (TestUygulamasi, Arc<DegisenYalitici>) {
    let sahte = Arc::new(DegisenYalitici {
        durum: Mutex::new(yok()),
        sonraki: Mutex::new(None),
        degisince: Mutex::new(None),
        dizinler: Mutex::new(Vec::new()),
    });
    let kurulan = sahte.clone();
    let t = test_uygulamasi_yalitimla(
        dizin,
        SahteSaglayici::yeni(Vec::new()),
        Some("evren_llm_a"),
        YalitimKaynagi::Kurucu(Box::new(move |degisince| {
            *kurulan.degisince.lock().unwrap() = Some(degisince);
            kurulan as Arc<dyn Yalitici>
        })),
    );
    (t, sahte)
}

#[tokio::test]
async fn yalitim_durumu_degisince_ana_pencerenin_kanalina_yazilir() {
    let d = tempfile::tempdir().unwrap();
    let (t, sahte) = kur(d.path());
    let durum = t.uygulama.uygulama_durumu();
    assert_eq!(durum.yalitim, yok());
    assert_eq!(durum.platform, std::env::consts::OS);

    // Kanal verilince şimdiki durum hemen yazılır.
    let kanal = SahteYalitimYazici::default();
    t.uygulama.yalitimi_dinle(Box::new(kanal.clone()));
    assert_eq!(kanal.yazilanlar(), vec![yok()]);

    // Değişmeyen durum yazılmaz.
    assert_eq!(t.uygulama.yalitimi_algila().await.unwrap(), yok());
    assert_eq!(kanal.yazilanlar().len(), 1);

    *sahte.sonraki.lock().unwrap() = Some(hazir());
    assert_eq!(t.uygulama.yalitimi_algila().await.unwrap(), hazir());
    assert_eq!(kanal.yazilanlar(), vec![yok(), hazir()]);
    assert_eq!(t.uygulama.uygulama_durumu().yalitim, hazir());

    // Yeniden yüklenen pencerenin kanalı eskisinin yerini alır.
    let yeni = SahteYalitimYazici::default();
    t.uygulama.yalitimi_dinle(Box::new(yeni.clone()));
    *sahte.sonraki.lock().unwrap() = Some(yok());
    t.uygulama.yalitimi_algila().await.unwrap();
    assert_eq!(yeni.yazilanlar(), vec![hazir(), yok()]);
    assert_eq!(kanal.yazilanlar().len(), 2);
}

#[tokio::test]
async fn arac_zinciri_dizini_klasor_seciciyle_eklenir_kalir_ve_kaldirilir() {
    let d = tempfile::tempdir().unwrap();
    let veri = d.path().join("veri");
    let arac = d.path().join("araclar/node");
    std::fs::create_dir_all(&arac).unwrap();
    let arac = arac.canonicalize().unwrap();
    let (t, sahte) = kur(&veri);
    assert!(t.uygulama.arac_zinciri_dizinleri().is_empty());

    // Vazgeçilirse liste değişmez.
    assert!(
        t.uygulama
            .arac_zinciri_dizini_ekle()
            .await
            .unwrap()
            .is_empty()
    );

    t.iletisim.klasor_ver(&arac);
    let beklenen = vec![arac.display().to_string()];
    assert_eq!(
        t.uygulama.arac_zinciri_dizini_ekle().await.unwrap(),
        beklenen
    );
    // Aynı dizin iki kez eklenmez.
    assert_eq!(
        t.uygulama.arac_zinciri_dizini_ekle().await.unwrap(),
        beklenen
    );
    assert_eq!(*sahte.dizinler.lock().unwrap(), vec![arac.clone()]);

    // Kayıt kalıcıdır: aynı veri diziniyle açılan uygulama okur.
    drop(t);
    let (t, _) = kur(&veri);
    assert_eq!(t.uygulama.arac_zinciri_dizinleri(), beklenen);

    let kalan = t
        .uygulama
        .arac_zinciri_dizini_kaldir(&arac.display().to_string())
        .unwrap();
    assert!(kalan.is_empty());
    assert!(
        t.uygulama
            .arac_zinciri_dizini_kaldir("/olmayan")
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn ev_ya_da_veri_dizinini_kapsayan_dizin_eklenemez() {
    let d = tempfile::tempdir().unwrap();
    // Ev dizini `<veri>/ev`dir (test_destek); veri dizini `d/veri`.
    let veri = d.path().join("veri");
    std::fs::create_dir_all(veri.join("ev")).unwrap();
    std::fs::create_dir_all(veri.join("yalitim/abc")).unwrap();
    let (t, sahte) = kur(&veri);
    for secilen in [
        d.path().to_path_buf(),
        veri.clone(),
        veri.join("ev"),
        veri.join("yalitim/abc"),
    ] {
        t.iletisim.klasor_ver(&secilen);
        let hata = t.uygulama.arac_zinciri_dizini_ekle().await.unwrap_err();
        assert_eq!(hata.kod, HataKodu::GecersizGirdi, "{}", secilen.display());
    }
    assert!(t.uygulama.arac_zinciri_dizinleri().is_empty());
    assert!(sahte.dizinler.lock().unwrap().is_empty());
}
```

**`crates/kalfa-uygulama/tests/e2e_parcalari.rs`**: şu bloğu

```rust
use kalfa_evren::tipler::{ChatMessage, ChatRequest, FunctionCall, ToolCall};
use kalfa_evren::{AkisOlayi, EvrenError};
use kalfa_uygulama::anahtar::AnahtarKasasi;
use kalfa_uygulama::arayuz::Iletisim;
```

şununla değiştir:

```rust
use kalfa_evren::tipler::{ChatMessage, ChatRequest, FunctionCall, ToolCall};
use kalfa_evren::{AkisOlayi, EvrenError};
use kalfa_sandbox::Yalitimsiz;
use kalfa_uygulama::anahtar::AnahtarKasasi;
use kalfa_uygulama::arayuz::Iletisim;
```

**`crates/kalfa-uygulama/tests/e2e_parcalari.rs`**: şu bloğu

```rust
use kalfa_uygulama::evren::SaglayiciUretici;
use kalfa_uygulama::test_destek::{SahteAcici, SahteKanal, SahtePencereler};
use kalfa_uygulama::{Bilesenler, Uygulama};
use serde_json::json;
```

şununla değiştir:

```rust
use kalfa_uygulama::evren::SaglayiciUretici;
use kalfa_uygulama::test_destek::{SahteAcici, SahteKanal, SahtePencereler};
use kalfa_uygulama::yalitim::YalitimKaynagi;
use kalfa_uygulama::{Bilesenler, Uygulama};
use serde_json::json;
```

**`crates/kalfa-uygulama/tests/e2e_parcalari.rs`**: şu bloğu

```rust
    let i = SahteIletisim::yeni(&a);
    assert_eq!(
        i.klasor_sec().await.as_deref(),
        Some(std::path::Path::new("/tmp/alan"))
    );
    assert!(i.onayla("b", "m", "Evet", "Hayır").await);
    let hayir = SahteIletisim::yeni(&E2eAyarlari::default());
    assert_eq!(hayir.klasor_sec().await, None);
    assert!(!hayir.onayla("b", "m", "Evet", "Hayır").await);
}
```

şununla değiştir:

```rust
    let i = SahteIletisim::yeni(&a);
    assert_eq!(
        i.klasor_sec("x").await.as_deref(),
        Some(std::path::Path::new("/tmp/alan"))
    );
    assert!(i.onayla("b", "m", "Evet", "Hayır").await);
    let hayir = SahteIletisim::yeni(&E2eAyarlari::default());
    assert_eq!(hayir.klasor_sec("x").await, None);
    assert!(!hayir.onayla("b", "m", "Evet", "Hayır").await);
}
```

**`crates/kalfa-uygulama/tests/e2e_parcalari.rs`**: şu bloğu

```rust
        pencereler: Arc::new(SahtePencereler::default()),
        aktarim: AktarimAyarlari::default(),
    });
    u.baslat().await;
```

şununla değiştir:

```rust
        pencereler: Arc::new(SahtePencereler::default()),
        aktarim: AktarimAyarlari::default(),
        ev_dizini: std::env::temp_dir(),
        yalitim: YalitimKaynagi::Kurucu(Box::new(|_| Arc::new(Yalitimsiz))),
    });
    u.baslat().await;
```

**`crates/kalfa-uygulama/tests/sart_akisi.rs`**: şu bloğu

```rust
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::SahteSaglayici;
use kalfa_uygulama::anahtar::{AnahtarDeposu, AnahtarKasasi};
use kalfa_uygulama::sozlesme::{
```

şununla değiştir:

```rust
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::SahteSaglayici;
use kalfa_sandbox::Yalitimsiz;
use kalfa_uygulama::anahtar::{AnahtarDeposu, AnahtarKasasi};
use kalfa_uygulama::sozlesme::{
```

**`crates/kalfa-uygulama/tests/sart_akisi.rs`**: şu bloğu

```rust
    TestUygulamasi, YazilamayanDepo, test_uygulamasi,
};
use kalfa_uygulama::{Bilesenler, Uygulama};
```

şununla değiştir:

```rust
    TestUygulamasi, YazilamayanDepo, test_uygulamasi,
};
use kalfa_uygulama::yalitim::YalitimKaynagi;
use kalfa_uygulama::{Bilesenler, Uygulama};
```

**`crates/kalfa-uygulama/tests/sart_akisi.rs`**: şu bloğu

```rust
        pencereler: Arc::new(SahtePencereler::default()),
        aktarim: Default::default(),
    });
    uygulama.baslat().await;
```

şununla değiştir:

```rust
        pencereler: Arc::new(SahtePencereler::default()),
        aktarim: Default::default(),
        ev_dizini: std::env::temp_dir(),
        yalitim: YalitimKaynagi::Kurucu(Box::new(|_| Arc::new(Yalitimsiz))),
    });
    uygulama.baslat().await;
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa-uygulama --locked`
Expected: derleme hatası (`kalfa_uygulama::yalitim` yok).

- [ ] **Step 3: Uygulama tarafını yaz**

**`crates/kalfa-uygulama/src/yalitim.rs`** (tam içerik):

```rust
//! Yalıtımın uygulama tarafı (aşama 2 tasarımı §7.1, §8.3, §10.3–§10.5):
//! yalıtıcının kurulması, durum değişiminin ana pencereye iletilmesi ve
//! kullanıcının eklediği araç zinciri dizinleri.
//!
//! Durum, ana pencerenin `yalitimi_dinle` ile verdiği kanala yazılır
//! (`oturumu_dinle` gibi); pencereye Tauri'nin olay izinleri açılmaz.
//!
//! Dizinler uygulamanın veri dizinindeki `yalitim.json`'dadır; yalıtımın içinde
//! görünmez. Depo içindeki hiçbir dosya profili genişletemez.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use kalfa_sandbox::{PlatformAyari, Yalitici, YalitimDurumu};
use serde::{Deserialize, Serialize};

use crate::arayuz::YalitimYazici;
use crate::hata::KomutHatasi;
use crate::uygulama::Uygulama;

const DOSYA_ADI: &str = "yalitim.json";

/// Durum değişince çağrılan işlev.
pub type Degisince = Box<dyn Fn(&YalitimDurumu) + Send + Sync>;

/// Ana pencerenin kanalı; tek ana pencere vardır, yeni kanal eskisinin yerini
/// alır.
pub(crate) type Dinleyici = Arc<Mutex<Option<Box<dyn YalitimYazici>>>>;

/// Yalıtıcının nereden geldiği.
pub enum YalitimKaynagi {
    /// Platformun yalıtıcısı: Linux'ta bwrap, diğerlerinde şimdilik yalıtımsız.
    Platform,
    /// Testler: yalıtıcıyı durum değişince çağrılacak işlevle kurar.
    Kurucu(Box<dyn FnOnce(Degisince) -> Arc<dyn Yalitici> + Send>),
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Dosya {
    #[serde(default)]
    arac_zinciri_dizinleri: Vec<PathBuf>,
}

/// Kullanıcının Ayarlar'dan eklediği araç zinciri dizinleri.
pub struct AracZinciriKaydi {
    yol: PathBuf,
    dizinler: Mutex<Vec<PathBuf>>,
}

impl AracZinciriKaydi {
    /// Kaydı okur; okunamayan ya da bozuk kayıt boş sayılır.
    pub fn oku(veri_dizini: &Path) -> Self {
        let yol = veri_dizini.join(DOSYA_ADI);
        let dosya: Dosya = std::fs::read(&yol)
            .ok()
            .and_then(|v| serde_json::from_slice(&v).ok())
            .unwrap_or_default();
        Self {
            yol,
            dizinler: Mutex::new(dosya.arac_zinciri_dizinleri),
        }
    }

    pub fn dizinler(&self) -> Vec<PathBuf> {
        self.dizinler
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .clone()
    }

    /// Listeyi değiştirir ve yazar; yeni listeyi döner.
    fn degistir(&self, is: impl FnOnce(&mut Vec<PathBuf>)) -> Result<Vec<PathBuf>, KomutHatasi> {
        let mut dizinler = self.dizinler.lock().unwrap_or_else(|z| z.into_inner());
        let mut yeni = dizinler.clone();
        is(&mut yeni);
        let metin = serde_json::to_vec_pretty(&Dosya {
            arac_zinciri_dizinleri: yeni.clone(),
        })
        .map_err(|e| KomutHatasi::ic(e.to_string()))?;
        if let Some(ust) = self.yol.parent() {
            std::fs::create_dir_all(ust)?;
        }
        let gecici = self.yol.with_extension("json.yeni");
        std::fs::write(&gecici, metin)?;
        std::fs::rename(&gecici, &self.yol)?;
        *dizinler = yeni.clone();
        Ok(yeni)
    }
}

/// Yalıtıcıyı kurar; durum değişimi ana pencerenin kanalına yazılır.
pub(crate) fn yalitici_kur(
    kaynak: YalitimKaynagi,
    veri_dizini: &Path,
    ev: &Path,
    kayit: &AracZinciriKaydi,
    dinleyici: Dinleyici,
) -> Arc<dyn Yalitici> {
    let degisince: Degisince = Box::new(move |d| {
        if let Some(yazici) = dinleyici.lock().unwrap_or_else(|z| z.into_inner()).as_ref() {
            // Kapanmış pencerenin kanalı yazılamaz; yeni pencere yeniden verir.
            let _ = yazici.gonder(d);
        }
    });
    match kaynak {
        YalitimKaynagi::Platform => kalfa_sandbox::platform_yaliticisi(PlatformAyari {
            ev: ev.to_path_buf(),
            yalitim_dizini: veri_dizini.join("yalitim"),
            ek_okunur: kayit.dizinler(),
            degisince,
        }),
        YalitimKaynagi::Kurucu(kur) => kur(degisince),
    }
}

fn metinler(dizinler: &[PathBuf]) -> Vec<String> {
    dizinler.iter().map(|d| d.display().to_string()).collect()
}

impl Uygulama {
    /// Ana pencerenin yalıtım kanalı. Şimdiki durum hemen yazılır; sonraki
    /// değişimler de bu kanala gider.
    pub fn yalitimi_dinle(&self, yazici: Box<dyn YalitimYazici>) {
        let mut dinleyici = self
            .yalitim_dinleyici
            .lock()
            .unwrap_or_else(|z| z.into_inner());
        // Kilit tutulurken yazılır: araya giren bir değişim eski durumu ezmez.
        let _ = yazici.gonder(&self.yalitici.durum());
        *dinleyici = Some(yazici);
    }

    /// Algılamayı yeniden çalıştırır (§8.3); durum değiştiyse kanala da yazılır.
    pub async fn yalitimi_algila(&self) -> Result<YalitimDurumu, KomutHatasi> {
        let yalitici = self.yalitici.clone();
        tokio::task::spawn_blocking(move || yalitici.yeniden_algila())
            .await
            .map_err(|e| KomutHatasi::ic(e.to_string()))
    }

    pub fn arac_zinciri_dizinleri(&self) -> Vec<String> {
        metinler(&self.arac_zinciri.dizinler())
    }

    /// Yerel klasör seçiciyle bir dizin ekler; arayüz yol gönderemez (§10.5).
    /// Vazgeçilirse liste değişmez.
    pub async fn arac_zinciri_dizini_ekle(&self) -> Result<Vec<String>, KomutHatasi> {
        let Some(secilen) = self
            .iletisim
            .klasor_sec("Yalıtıma açılacak araç zinciri dizinini seçin")
            .await
        else {
            return Ok(self.arac_zinciri_dizinleri());
        };
        let dizin = self.eklenebilir_mi(&secilen)?;
        let yeni = self.arac_zinciri.degistir(|l| {
            if !l.contains(&dizin) {
                l.push(dizin);
            }
        })?;
        self.yalitici.okunur_dizinleri_ayarla(yeni.clone());
        Ok(metinler(&yeni))
    }

    /// Listeden çıkarır. Yalnızca çıkarma yapar; listede olmayan yol yok
    /// sayılır.
    pub fn arac_zinciri_dizini_kaldir(&self, yol: &str) -> Result<Vec<String>, KomutHatasi> {
        let yeni = self
            .arac_zinciri
            .degistir(|l| l.retain(|d| d.as_os_str() != yol))?;
        self.yalitici.okunur_dizinleri_ayarla(yeni.clone());
        Ok(metinler(&yeni))
    }

    /// Ev dizinini ya da Kalfa'nın veri dizinini kapsayan, onların içinde
    /// kalan dizin eklenemez: sırlar ve oturum kayıtları yalıtıma açılırdı.
    fn eklenebilir_mi(&self, secilen: &Path) -> Result<PathBuf, KomutHatasi> {
        let dizin = secilen
            .canonicalize()
            .ok()
            .filter(|d| d.is_dir())
            .ok_or_else(|| KomutHatasi::gecersiz("Seçilen dizin bulunamadı."))?;
        let kanonik = |y: &Path| y.canonicalize().unwrap_or_else(|_| y.to_path_buf());
        let ev = kanonik(&self.ev_dizini);
        let veri = kanonik(&self.veri_dizini);
        if ev.starts_with(&dizin) || veri.starts_with(&dizin) {
            return Err(KomutHatasi::gecersiz(
                "Bu dizin ev dizinini ya da Kalfa'nın veri dizinini kapsıyor; yalıtıma açılamaz.",
            ));
        }
        if dizin.starts_with(&veri) {
            return Err(KomutHatasi::gecersiz(
                "Kalfa'nın veri dizinindeki bir dizin yalıtıma açılamaz.",
            ));
        }
        Ok(dizin)
    }
}
```

**`crates/kalfa-uygulama/src/lib.rs`**: şu bloğu

```rust
pub mod modeller;
pub mod sozlesme;

mod kopru;
```

şununla değiştir:

```rust
pub mod modeller;
pub mod sozlesme;
pub mod yalitim;

mod kopru;
```

**`crates/kalfa-uygulama/src/arayuz.rs`**: şu bloğu

```rust
use async_trait::async_trait;
use kalfa_core::aktarim::KanalMesaji;
use url::Url;
```

şununla değiştir:

```rust
use async_trait::async_trait;
use kalfa_core::aktarim::KanalMesaji;
pub use kalfa_sandbox::YalitimDurumu;
use url::Url;
```

**`crates/kalfa-uygulama/src/arayuz.rs`**: şu bloğu

```rust
pub trait Iletisim: Send + Sync {
    /// Yerel klasör seçici; vazgeçilirse `None`.
    async fn klasor_sec(&self) -> Option<PathBuf>;
    /// Yerel onay kutusu; `tamam` düğmesi seçilirse `true`.
    async fn onayla(&self, baslik: &str, mesaj: &str, tamam: &str, vazgec: &str) -> bool;
```

şununla değiştir:

```rust
pub trait Iletisim: Send + Sync {
    /// Yerel klasör seçici; vazgeçilirse `None`.
    async fn klasor_sec(&self, baslik: &str) -> Option<PathBuf>;
    /// Yerel onay kutusu; `tamam` düğmesi seçilirse `true`.
    async fn onayla(&self, baslik: &str, mesaj: &str, tamam: &str, vazgec: &str) -> bool;
```

**`crates/kalfa-uygulama/src/arayuz.rs`**: şu bloğu

```rust
}

/// Bir oturumun kanal mesajlarının yazıldığı yer (Tauri'de `Channel`).
pub trait KanalYazici: Send + Sync + 'static {
```

şununla değiştir:

```rust
}

/// Yalıtım durumunun ana pencereye yazıldığı yer (Tauri'de `Channel`;
/// aşama 2 tasarımı §10.5).
pub trait YalitimYazici: Send + Sync + 'static {
    fn gonder(&self, durum: &YalitimDurumu) -> Result<(), String>;
}

/// Bir oturumun kanal mesajlarının yazıldığı yer (Tauri'de `Channel`).
pub trait KanalYazici: Send + Sync + 'static {
```

**`crates/kalfa-uygulama/src/uygulama.rs`**: şu bloğu

```rust
use kalfa_core::oturum::OturumTutamaci;
use kalfa_core::sart_kapisi::SartKapisi;
use kalfa_sandbox::{Yalitici, Yalitimsiz};

use crate::anahtar::AnahtarKasasi;
```

şununla değiştir:

```rust
use kalfa_core::oturum::OturumTutamaci;
use kalfa_core::sart_kapisi::SartKapisi;
use kalfa_sandbox::Yalitici;

use crate::anahtar::AnahtarKasasi;
```

**`crates/kalfa-uygulama/src/uygulama.rs`**: şu bloğu

```rust
use crate::oturumlar::AcikOturum;
use crate::sozlesme::{AnahtarNedeni, BaglantiDurumu, SartPenceresiDurumu};

/// Uygulamayı kurmak için gerekenler; Tauri katmanı ya da test verir.
```

şununla değiştir:

```rust
use crate::oturumlar::AcikOturum;
use crate::sozlesme::{AnahtarNedeni, BaglantiDurumu, SartPenceresiDurumu};
use crate::yalitim::{AracZinciriKaydi, Dinleyici, YalitimKaynagi, yalitici_kur};

/// Uygulamayı kurmak için gerekenler; Tauri katmanı ya da test verir.
```

**`crates/kalfa-uygulama/src/uygulama.rs`**: şu bloğu

```rust
    pub pencereler: Arc<dyn Pencereler>,
    pub aktarim: AktarimAyarlari,
}
```

şununla değiştir:

```rust
    pub pencereler: Arc<dyn Pencereler>,
    pub aktarim: AktarimAyarlari,
    /// Kullanıcının ev dizini: araç zinciri dizinleri buna göre bulunur.
    pub ev_dizini: PathBuf,
    pub yalitim: YalitimKaynagi,
}
```

**`crates/kalfa-uygulama/src/uygulama.rs`**: şu bloğu

```rust
    pub(crate) pencereler: Arc<dyn Pencereler>,
    pub(crate) aktarim: AktarimAyarlari,
    /// Kabuk komutlarını başlatan arka uç. Aşama 2a'da yalıtım yoktur.
    pub(crate) yalitici: Arc<dyn Yalitici>,
    pub(crate) alanlar: Arc<CalismaAlaniKaydi>,
    ic: Mutex<IcDurum>,
```

şununla değiştir:

```rust
    pub(crate) pencereler: Arc<dyn Pencereler>,
    pub(crate) aktarim: AktarimAyarlari,
    pub(crate) veri_dizini: PathBuf,
    pub(crate) ev_dizini: PathBuf,
    /// Kabuk komutlarını başlatan arka uç (aşama 2 tasarımı §3.2).
    pub(crate) yalitici: Arc<dyn Yalitici>,
    pub(crate) arac_zinciri: AracZinciriKaydi,
    pub(crate) yalitim_dinleyici: Dinleyici,
    pub(crate) alanlar: Arc<CalismaAlaniKaydi>,
    ic: Mutex<IcDurum>,
```

**`crates/kalfa-uygulama/src/uygulama.rs`**: şu bloğu

```rust
        let evren = Arc::new(DegisirSaglayici::default());
        let kapi = Arc::new(SartKapisi::yeni(evren.clone()));
        Arc::new(Self {
            oturum_dizini: dizin::oturum_dizini(&b.veri_dizini),
```

şununla değiştir:

```rust
        let evren = Arc::new(DegisirSaglayici::default());
        let kapi = Arc::new(SartKapisi::yeni(evren.clone()));
        let arac_zinciri = AracZinciriKaydi::oku(&b.veri_dizini);
        let yalitim_dinleyici = Dinleyici::default();
        let yalitici = yalitici_kur(
            b.yalitim,
            &b.veri_dizini,
            &b.ev_dizini,
            &arac_zinciri,
            yalitim_dinleyici.clone(),
        );
        Arc::new(Self {
            oturum_dizini: dizin::oturum_dizini(&b.veri_dizini),
```

**`crates/kalfa-uygulama/src/uygulama.rs`**: şu bloğu

```rust
            pencereler: b.pencereler,
            aktarim: b.aktarim,
            yalitici: Arc::new(Yalitimsiz),
            alanlar: Arc::new(CalismaAlaniKaydi::yeni(&b.veri_dizini)),
            ic: Mutex::new(IcDurum {
                baglanti: BaglantiDurumu::Var,
```

şununla değiştir:

```rust
            pencereler: b.pencereler,
            aktarim: b.aktarim,
            yalitici,
            arac_zinciri,
            yalitim_dinleyici,
            alanlar: Arc::new(CalismaAlaniKaydi::yeni(&b.veri_dizini)),
            veri_dizini: b.veri_dizini,
            ev_dizini: b.ev_dizini,
            ic: Mutex::new(IcDurum {
                baglanti: BaglantiDurumu::Var,
```

**`crates/kalfa-uygulama/src/calisma_alanlari.rs`**: şu bloğu

```rust
use std::sync::{Arc, Mutex};

use kalfa_core::muhur::ozet_hex;
use kalfa_tools::KabukGuveni;
use serde::{Deserialize, Serialize};
```

şununla değiştir:

```rust
use std::sync::{Arc, Mutex};

use kalfa_tools::KabukGuveni;
use serde::{Deserialize, Serialize};
```

**`crates/kalfa-uygulama/src/calisma_alanlari.rs`**: şu bloğu

```rust
    /// Çalışma alanının kimliği: kanonik yolunun SHA-256'sından 16 onaltılık
    /// karakter.
    pub fn kimlik(kok: &Path) -> String {
        let kanonik = std::fs::canonicalize(kok).unwrap_or_else(|_| kok.to_path_buf());
        ozet_hex(kanonik.to_string_lossy().as_bytes())[..16].to_string()
    }
```

şununla değiştir:

```rust
    /// Çalışma alanının kimliği: kanonik yolunun SHA-256'sından 16 onaltılık
    /// karakter.
    /// Yalıtımın çalışma alanına özel dizinleri de bu kimliği kullanır.
    pub fn kimlik(kok: &Path) -> String {
        kalfa_sandbox::alan_kimligi(kok)
    }
```

**`crates/kalfa-uygulama/src/oturumlar.rs`**: şu bloğu

```rust
            son_oturum: ic.son_oturum,
            yalitim: self.yalitici.durum(),
        }
    }
```

şununla değiştir:

```rust
            son_oturum: ic.son_oturum,
            yalitim: self.yalitici.durum(),
            platform: std::env::consts::OS.to_string(),
        }
    }
```

**`crates/kalfa-uygulama/src/oturumlar.rs`**: şu bloğu

```rust
    /// geçerli bir kimlikle arayüze verilir (1c tasarımı §5.1).
    pub async fn calisma_alani_sec(&self) -> Option<CalismaAlani> {
        let yol = self.iletisim.klasor_sec().await?;
        let mut ic = self.ic();
        ic.son_alan += 1;
```

şununla değiştir:

```rust
    /// geçerli bir kimlikle arayüze verilir (1c tasarımı §5.1).
    pub async fn calisma_alani_sec(&self) -> Option<CalismaAlani> {
        let yol = self.iletisim.klasor_sec("Çalışma alanı seçin").await?;
        let mut ic = self.ic();
        ic.son_alan += 1;
```

**`crates/kalfa-uygulama/src/sozlesme.rs`**: şu bloğu

```rust
    /// Kabuk komutlarının yalıtımı (aşama 2 tasarımı §10.3).
    pub yalitim: YalitimDurumu,
}
```

şununla değiştir:

```rust
    /// Kabuk komutlarının yalıtımı (aşama 2 tasarımı §10.3).
    pub yalitim: YalitimDurumu,
    /// İşletim sistemi (`linux`, `windows`, `macos`): Ayarlar'daki yalıtım
    /// bölümü araç zinciri dizinlerini yalnızca Linux'ta gösterir.
    pub platform: String,
}
```

**`crates/kalfa-uygulama/src/test_destek.rs`**: şu bloğu

```rust
use kalfa_core::test_destek::SahteSaglayici;
use kalfa_evren::EvrenError;
use tokio::sync::{Semaphore, mpsc};
use url::Url;

use crate::anahtar::{AnahtarDeposu, AnahtarKasasi, BellekDeposu};
use crate::arayuz::{Acici, Iletisim, KanalYazici, Pencereler};
use crate::evren::SaglayiciUretici;
use crate::{Bilesenler, Uygulama};
```

şununla değiştir:

```rust
use kalfa_core::test_destek::SahteSaglayici;
use kalfa_evren::EvrenError;
use kalfa_sandbox::{YalitimDurumu, Yalitimsiz};
use tokio::sync::{Semaphore, mpsc};
use url::Url;

use crate::anahtar::{AnahtarDeposu, AnahtarKasasi, BellekDeposu};
use crate::arayuz::{Acici, Iletisim, KanalYazici, Pencereler, YalitimYazici};
use crate::evren::SaglayiciUretici;
use crate::yalitim::YalitimKaynagi;
use crate::{Bilesenler, Uygulama};
```

**`crates/kalfa-uygulama/src/test_destek.rs`**: şu bloğu

```rust
#[async_trait]
impl Iletisim for SahteIletisim {
    async fn klasor_sec(&self) -> Option<PathBuf> {
        self.klasor.lock().unwrap().clone()
    }
```

şununla değiştir:

```rust
#[async_trait]
impl Iletisim for SahteIletisim {
    async fn klasor_sec(&self, _baslik: &str) -> Option<PathBuf> {
        self.klasor.lock().unwrap().clone()
    }
```

**`crates/kalfa-uygulama/src/test_destek.rs`**: şu bloğu

```rust
}

/// Kanal mesajlarını bir `mpsc` kanalına aktarır.
pub struct SahteKanal {
```

şununla değiştir:

```rust
}

/// Ana pencerenin yalıtım kanalının yerine geçer; yazılan durumları kaydeder.
#[derive(Clone, Default)]
pub struct SahteYalitimYazici(Arc<Mutex<Vec<YalitimDurumu>>>);

impl SahteYalitimYazici {
    pub fn yazilanlar(&self) -> Vec<YalitimDurumu> {
        self.0.lock().unwrap().clone()
    }
}

impl YalitimYazici for SahteYalitimYazici {
    fn gonder(&self, durum: &YalitimDurumu) -> Result<(), String> {
        self.0.lock().unwrap().push(durum.clone());
        Ok(())
    }
}

/// Kanal mesajlarını bir `mpsc` kanalına aktarır.
pub struct SahteKanal {
```

**`crates/kalfa-uygulama/src/test_destek.rs`**: şu bloğu

```rust

/// `kayitli_anahtar` verilirse kalıcı depoda o anahtar kayıtlıymış gibi
/// başlar. Veri dizini `dizin`dir.
pub fn test_uygulamasi(
    dizin: &Path,
    saglayici: Arc<SahteSaglayici>,
    kayitli_anahtar: Option<&str>,
) -> TestUygulamasi {
    let depo = Arc::new(BellekDeposu::default());
```

şununla değiştir:

```rust

/// `kayitli_anahtar` verilirse kalıcı depoda o anahtar kayıtlıymış gibi
/// başlar. Veri dizini `dizin`dir. Kabuk yalıtımsızdır.
pub fn test_uygulamasi(
    dizin: &Path,
    saglayici: Arc<SahteSaglayici>,
    kayitli_anahtar: Option<&str>,
) -> TestUygulamasi {
    test_uygulamasi_yalitimla(
        dizin,
        saglayici,
        kayitli_anahtar,
        YalitimKaynagi::Kurucu(Box::new(|_| Arc::new(Yalitimsiz))),
    )
}

/// `test_uygulamasi`, verilen yalıtım kaynağıyla. Ev dizini `dizin/ev`dir.
pub fn test_uygulamasi_yalitimla(
    dizin: &Path,
    saglayici: Arc<SahteSaglayici>,
    kayitli_anahtar: Option<&str>,
    yalitim: YalitimKaynagi,
) -> TestUygulamasi {
    let depo = Arc::new(BellekDeposu::default());
```

**`crates/kalfa-uygulama/src/test_destek.rs`**: şu bloğu

```rust
        pencereler: pencereler.clone(),
        aktarim: AktarimAyarlari::default(),
    });
    TestUygulamasi {
```

şununla değiştir:

```rust
        pencereler: pencereler.clone(),
        aktarim: AktarimAyarlari::default(),
        ev_dizini: dizin.join("ev"),
        yalitim,
    });
    TestUygulamasi {
```

**`crates/kalfa-uygulama/src/e2e/mod.rs`**: şu bloğu

```rust
#[async_trait]
impl Iletisim for SahteIletisim {
    async fn klasor_sec(&self) -> Option<PathBuf> {
        self.calisma_alani.clone()
    }
```

şununla değiştir:

```rust
#[async_trait]
impl Iletisim for SahteIletisim {
    async fn klasor_sec(&self, _baslik: &str) -> Option<PathBuf> {
        self.calisma_alani.clone()
    }
```

- [ ] **Step 4: Tauri tarafını yaz**

**`crates/kalfa/src/komut_adlari.rs`**: şu bloğu

```rust

/// Ana pencerenin komutları.
pub const ANA_PENCERE_KOMUTLARI: [&str; 17] = [
    "uygulama_durumu",
    "oturumlari_listele",
```

şununla değiştir:

```rust

/// Ana pencerenin komutları.
pub const ANA_PENCERE_KOMUTLARI: [&str; 22] = [
    "uygulama_durumu",
    "oturumlari_listele",
```

**`crates/kalfa/src/komut_adlari.rs`**: şu bloğu

```rust
    "anahtar_penceresini_ac",
    "sart_penceresini_ac",
];
```

şununla değiştir:

```rust
    "anahtar_penceresini_ac",
    "sart_penceresini_ac",
    "yalitimi_dinle",
    "yalitimi_algila",
    "arac_zinciri_dizinleri",
    "arac_zinciri_dizini_ekle",
    "arac_zinciri_dizini_kaldir",
];
```

**`crates/kalfa/capabilities/ana-pencere.json`**: şu bloğu

```json
    "allow-baglanti-ac",
    "allow-anahtar-penceresini-ac",
    "allow-sart-penceresini-ac"
  ]
}
```

şununla değiştir:

```json
    "allow-baglanti-ac",
    "allow-anahtar-penceresini-ac",
    "allow-sart-penceresini-ac",
    "allow-yalitimi-dinle",
    "allow-yalitimi-algila",
    "allow-arac-zinciri-dizinleri",
    "allow-arac-zinciri-dizini-ekle",
    "allow-arac-zinciri-dizini-kaldir"
  ]
}
```

**`crates/kalfa/src/komutlar.rs`**: şu bloğu

```rust
use kalfa_core::kimlik::OturumId;
use kalfa_uygulama::Uygulama;
use kalfa_uygulama::hata::KomutHatasi;
use kalfa_uygulama::sozlesme::{
```

şununla değiştir:

```rust
use kalfa_core::kimlik::OturumId;
use kalfa_uygulama::Uygulama;
use kalfa_uygulama::arayuz::YalitimDurumu;
use kalfa_uygulama::hata::KomutHatasi;
use kalfa_uygulama::sozlesme::{
```

**`crates/kalfa/src/komutlar.rs`**: şu bloğu

```rust
use tauri::{Builder, Runtime, State};

use crate::kanal::TauriKanal;

type Sonuc<T> = Result<T, KomutHatasi>;
```

şununla değiştir:

```rust
use tauri::{Builder, Runtime, State};

use crate::kanal::{TauriKanal, TauriYalitimKanali};

type Sonuc<T> = Result<T, KomutHatasi>;
```

**`crates/kalfa/src/komutlar.rs`**: şu bloğu

```rust
        anahtar_penceresini_ac,
        sart_penceresini_ac,
        sart_penceresi_durumu,
        anahtari_kaydet,
```

şununla değiştir:

```rust
        anahtar_penceresini_ac,
        sart_penceresini_ac,
        yalitimi_dinle,
        yalitimi_algila,
        arac_zinciri_dizinleri,
        arac_zinciri_dizini_ekle,
        arac_zinciri_dizini_kaldir,
        sart_penceresi_durumu,
        anahtari_kaydet,
```

**`crates/kalfa/src/komutlar.rs`**: şu bloğu

```rust
}

// --- Şart penceresi ---
```

şununla değiştir:

```rust
}

/// Ana pencere açılınca bir kez çağırır; durum değişimi bu kanala yazılır.
#[tauri::command(rename_all = "snake_case")]
pub async fn yalitimi_dinle(
    u: State<'_, Arc<Uygulama>>,
    kanal: Channel<YalitimDurumu>,
) -> Sonuc<()> {
    u.yalitimi_dinle(Box::new(TauriYalitimKanali(kanal)));
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn yalitimi_algila(u: State<'_, Arc<Uygulama>>) -> Sonuc<YalitimDurumu> {
    u.yalitimi_algila().await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn arac_zinciri_dizinleri(u: State<'_, Arc<Uygulama>>) -> Sonuc<Vec<String>> {
    Ok(u.arac_zinciri_dizinleri())
}

/// Dizin yerel klasör seçiciyle seçilir; arayüz yol gönderemez.
#[tauri::command(rename_all = "snake_case")]
pub async fn arac_zinciri_dizini_ekle(u: State<'_, Arc<Uygulama>>) -> Sonuc<Vec<String>> {
    u.arac_zinciri_dizini_ekle().await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn arac_zinciri_dizini_kaldir(
    u: State<'_, Arc<Uygulama>>,
    yol: String,
) -> Sonuc<Vec<String>> {
    u.arac_zinciri_dizini_kaldir(&yol)
}

// --- Şart penceresi ---
```

**`crates/kalfa/src/kanal.rs`**: şu bloğu

```rust
//! Oturumun kanal mesajlarını webview'a taşıyan Tauri kanalı (1c tasarımı §4).

use kalfa_core::aktarim::KanalMesaji;
use kalfa_uygulama::arayuz::KanalYazici;
use tauri::ipc::Channel;

pub struct TauriKanal(pub Channel<KanalMesaji>);
```

şununla değiştir:

```rust
//! Oturumun kanal mesajlarını webview'a taşıyan Tauri kanalı (1c tasarımı §4)
//! ve ana pencerenin yalıtım kanalı (aşama 2 tasarımı §10.5).

use kalfa_core::aktarim::KanalMesaji;
use kalfa_uygulama::arayuz::{KanalYazici, YalitimDurumu, YalitimYazici};
use tauri::ipc::Channel;

pub struct TauriKanal(pub Channel<KanalMesaji>);
```

**`crates/kalfa/src/kanal.rs`**: şu bloğu

```rust
    }
}
```

şununla değiştir:

```rust
    }
}

pub struct TauriYalitimKanali(pub Channel<YalitimDurumu>);

impl YalitimYazici for TauriYalitimKanali {
    fn gonder(&self, durum: &YalitimDurumu) -> Result<(), String> {
        self.0.send(durum.clone()).map_err(|e| e.to_string())
    }
}
```

**`crates/kalfa/src/iletisim.rs`**: şu bloğu

```rust
#[async_trait]
impl<R: Runtime> Iletisim for TauriIletisim<R> {
    async fn klasor_sec(&self) -> Option<PathBuf> {
        let (gonder, al) = oneshot::channel();
        let mut kutu = self.0.dialog().file().set_title("Çalışma alanı seçin");
        if let Some(pencere) = odakli_pencere(&self.0) {
            kutu = kutu.set_parent(&pencere);
```

şununla değiştir:

```rust
#[async_trait]
impl<R: Runtime> Iletisim for TauriIletisim<R> {
    async fn klasor_sec(&self, baslik: &str) -> Option<PathBuf> {
        let (gonder, al) = oneshot::channel();
        let mut kutu = self.0.dialog().file().set_title(baslik);
        if let Some(pencere) = odakli_pencere(&self.0) {
            kutu = kutu.set_parent(&pencere);
```

**`crates/kalfa/src/lib.rs`**: şu bloğu

```rust
use kalfa_uygulama::anahtar::{AnahtarDeposu, AnahtarKasasi, SistemDeposu};
use kalfa_uygulama::dizin::{VERI_DIZINI_DEGISKENI, veri_dizini};
use kalfa_uygulama::{Bilesenler, Uygulama};
use tauri::webview::PageLoadEvent;
```

şununla değiştir:

```rust
use kalfa_uygulama::anahtar::{AnahtarDeposu, AnahtarKasasi, SistemDeposu};
use kalfa_uygulama::dizin::{VERI_DIZINI_DEGISKENI, veri_dizini};
use kalfa_uygulama::yalitim::YalitimKaynagi;
use kalfa_uygulama::{Bilesenler, Uygulama};
use tauri::webview::PageLoadEvent;
```

**`crates/kalfa/src/lib.rs`**: şu bloğu

```rust
        pencereler: Arc::new(TauriPencereler(app.clone())),
        aktarim: AktarimAyarlari::default(),
    }))
}
```

şununla değiştir:

```rust
        pencereler: Arc::new(TauriPencereler(app.clone())),
        aktarim: AktarimAyarlari::default(),
        ev_dizini: app.path().home_dir()?,
        yalitim: YalitimKaynagi::Platform,
    }))
}
```

- [ ] **Step 5: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa-uygulama -p kalfa --locked`
Expected: `tests/yalitim.rs`'in 3 testi ve `crates/kalfa/tests/izinler.rs` (yeni komutlar yalnızca ana pencereye açık) dahil bütün testler geçer. Görevin sonundaki genel denetimleri (`e2e` özellikli testler dahil) de koştur.

- [ ] **Step 6: Commit**

```bash
git add crates/kalfa-uygulama crates/kalfa
git commit -m "uygulama: platform yalıtıcısı, araç zinciri dizinleri ve yalıtım kanalı"
```

---

### Task 6: Arayüz: Ayarlar ekranı, yalıtım göstergesi, konuşmanın alta yapışması

**Files:**
- Create: `ui/src/ana/Ayarlar.svelte`, `ui/src/ana/ayarlar.test.ts`, `ui/src/ana/altaYapisma.ts`, `ui/src/ana/altaYapisma.test.ts`
- Modify: `ui/src/ana/{Ana,Menu,DurumCubugu,Konusma}.svelte`, `ui/src/ana/anaDurum.svelte.ts`, `ui/src/bilesenler/ikonlar.ts`, `ui/src/dil/tr.ts`, `ui/src/ipc/{ana,tipler}.ts`, `ui/src/onizleme/senaryo.ts`, `ui/src/sahte/arka.ts`

**Interfaces:**
- Consumes: Görev 5'in komutları ve `UygulamaDurumu.platform`.
- Produces:
  - `ipc/ana.ts`: `yalitimiDinle(dinleyici)`, `yalitimiAlgila()`, `aracZinciriDizinleri()`, `aracZinciriDiziniEkle()`, `aracZinciriDiziniKaldir(yol)`.
  - `AnaDurum`: `gorunum: 'oturumlar' | 'ayarlar'`, `aracZinciri: string[]`, `algilaniyor`, `ayarlariAc()`, `oturumlaraDon()`, `yalitimiAlgila()`, `aracZinciriEkle()`, `aracZinciriKaldir(yol)`.
  - `dil/tr.ts`: `YALITIM_NEDENI`; ikon `settings`.
  - `SahteArka`: `uygulama.platform`, `aracZinciri`, `aracKlasoru`, `sonrakiYalitim`, `yalitimDurumu(yeni)`.
  - `ana/altaYapisma.ts`: `AltaYapisma` (`altta`, `niyet()`, `kaydir(kutu)`, `olay(kutu)`, `sifirla()`), `kaydirmaNiyetiMi(olay, kutu)`.

**Davranış notları (testler sınar):**
- Menüde "Ayarlar"; seçilince oturum listesi ve konuşmanın yerine Ayarlar gelir. Oturum seçimi ve yeni oturum konuşmaya döner.
- Yalıtım kartı: hazırsa "Yalıtımlı: bwrap 0.9.0" ve açıklama; değilse "Yalıtımsız: <neden>" ve talimat. "Yeniden dene" yalnızca Linux'ta.
- Araç zinciri kartı yalnızca Linux'ta; "Dizin ekle" komutu argümansız çağırır (yolu Rust seçtirir); her satırda "<yol> dizinini kaldır". Hata Ayarlar'da gösterilir.
- Durum çubuğundaki yalıtım göstergesi bir düğmedir ve Ayarlar'ı açar; kanaldan gelen durum göstergeyi günceller.
- **Konuşmanın alta yapışması (1c'den kalan hata):** konuşma, kullanıcı yukarı kaydırmadıkça en altta kalır. Yapışma yalnızca kullanıcının kaydırma niyetinden sonraki 1 sn içinde gelen, en alttan 48 pikselden uzak bir `scroll` olayıyla bırakılır. Niyet: tekerlek, dokunarak sürükleme, kutunun kendisine (kaydırma çubuğuna) basmak, kaydırma tuşları. Konuşmadaki bir düğmeye tıklamak, odağın geri dönmesi ya da kendi kaydırmamızın olayı bırakmaz; en alta hangi yoldan inilirse inilsin yeniden yapışılır. Linux'ta kabuk testleri yalıtımla koşarken yeni onay kartı görünüm alanının altında kalıyordu.

- [ ] **Step 1: Başarısız testleri yaz**

**`ui/src/ana/ayarlar.test.ts`** (tam içerik):

```ts
import { clearMocks } from '@tauri-apps/api/mocks';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import type { YalitimDurumu } from '../ipc/tipler';
import { SahteArka } from '../sahte/arka';
import Ana from './Ana.svelte';
import { AnaDurum } from './anaDurum.svelte';

afterEach(() => clearMocks());

const HAZIR: YalitimDurumu = { tip: 'Hazir', arka_uc: 'bwrap', surum: '0.9.0' };
const BWRAP_YOK: YalitimDurumu = {
  tip: 'Yok',
  neden: 'BwrapYok',
  talimat: 'Komutları yalıtmak için bubblewrap paketini kurun.',
};

async function ciz(ayar: (a: SahteArka) => void = () => {}) {
  const arka = new SahteArka().kur();
  arka.uygulama.yalitim = BWRAP_YOK;
  ayar(arka);
  const durum = new AnaDurum();
  render(Ana, { props: { durum } });
  await screen.findByRole('region', { name: 'Konuşma' });
  return { arka, durum };
}

function menu() {
  return screen.getByRole('navigation', { name: 'Menü' });
}

async function ayarlariAc() {
  await fireEvent.click(within(menu()).getByRole('button', { name: 'Ayarlar' }));
  return screen.findByRole('region', { name: 'Ayarlar' });
}

function durumCubugu() {
  return screen.getByRole('contentinfo', { name: 'Durum çubuğu' });
}

describe('Ayarlar: yalıtım (aşama 2 tasarımı §10.3, §10.4)', () => {
  it('menüden açılır; durum, neden ve talimat görünür; Oturumlar\'a dönülür', async () => {
    await ciz();
    const ayarlar = await ayarlariAc();
    expect(within(menu()).getByRole('button', { name: 'Ayarlar' }).getAttribute('aria-current')).toBe(
      'page',
    );
    expect(within(ayarlar).getByText('Yalıtımsız: bubblewrap bulunamadı')).toBeTruthy();
    expect(within(ayarlar).getByText(BWRAP_YOK.talimat)).toBeTruthy();
    expect(screen.queryByRole('region', { name: 'Konuşma' })).toBeNull();

    await fireEvent.click(within(menu()).getByRole('button', { name: 'Oturumlar' }));
    expect(await screen.findByRole('region', { name: 'Konuşma' })).toBeTruthy();
    expect(screen.queryByRole('region', { name: 'Ayarlar' })).toBeNull();
  });

  it('durum çubuğundaki gösterge Ayarlar\'ı açar', async () => {
    await ciz();
    await fireEvent.click(within(durumCubugu()).getByRole('button', { name: 'Yalıtımsız' }));
    expect(await screen.findByRole('region', { name: 'Ayarlar' })).toBeTruthy();
  });

  it('"Yeniden dene" algılamayı çalıştırır; ekran ve durum çubuğu güncellenir', async () => {
    const { arka } = await ciz((a) => {
      a.sonrakiYalitim = HAZIR;
    });
    const ayarlar = await ayarlariAc();
    await fireEvent.click(within(ayarlar).getByRole('button', { name: 'Yeniden dene' }));
    expect(await within(ayarlar).findByText('Yalıtımlı: bwrap 0.9.0')).toBeTruthy();
    expect(arka.komutlar('yalitimi_algila')).toHaveLength(1);
    const gosterge = within(durumCubugu()).getByText('Yalıtımlı', { selector: '.durum-ogesi' });
    expect(gosterge.getAttribute('title')).toBe('Komutlar yalıtımda çalışıyor (bwrap)');
  });

  it('kanaldan gelen durum değişimi durum çubuğunu günceller', async () => {
    const { arka } = await ciz();
    await waitFor(() => expect(arka.komutlar('yalitimi_dinle')).toHaveLength(1));
    arka.yalitimDurumu(HAZIR);
    expect(
      await within(durumCubugu()).findByText('Yalıtımlı', { selector: '.durum-ogesi' }),
    ).toBeTruthy();
  });

  it('araç zinciri dizini klasör seçiciyle eklenir ve kaldırılır', async () => {
    const { arka } = await ciz((a) => {
      a.aracZinciri = ['/opt/araclar'];
    });
    const ayarlar = await ayarlariAc();
    const liste = await within(ayarlar).findByRole('list', { name: 'Araç zinciri dizinleri' });
    expect(within(liste).getByText('/opt/araclar')).toBeTruthy();

    await fireEvent.click(within(ayarlar).getByRole('button', { name: 'Dizin ekle' }));
    expect(await within(liste).findByText('/opt/node')).toBeTruthy();
    // Arayüz yol göndermez; klasörü Rust seçtirir.
    expect(arka.komutlar('arac_zinciri_dizini_ekle')).toEqual([{}]);

    await fireEvent.click(
      within(liste).getByRole('button', { name: '/opt/araclar dizinini kaldır' }),
    );
    await waitFor(() => expect(within(liste).queryByText('/opt/araclar')).toBeNull());
    expect(arka.komutlar('arac_zinciri_dizini_kaldir')).toEqual([{ yol: '/opt/araclar' }]);
  });

  it('eklenemeyen dizinin hatası Ayarlar\'da gösterilir', async () => {
    await ciz((a) => {
      a.hatalar.set('arac_zinciri_dizini_ekle', {
        kod: 'GecersizGirdi',
        mesaj: 'Bu dizin ev dizinini ya da Kalfa\'nın veri dizinini kapsıyor; yalıtıma açılamaz.',
      });
    });
    const ayarlar = await ayarlariAc();
    expect(within(ayarlar).getByText('Eklenmiş dizin yok.')).toBeTruthy();
    await fireEvent.click(within(ayarlar).getByRole('button', { name: 'Dizin ekle' }));
    expect((await within(ayarlar).findByRole('alert')).textContent).toContain(
      'yalıtıma açılamaz',
    );
  });

  it('Windows\'ta araç zinciri bölümü ve "Yeniden dene" yok', async () => {
    const { arka } = await ciz((a) => {
      a.uygulama.platform = 'windows';
      a.uygulama.yalitim = {
        tip: 'Yok',
        neden: 'Desteklenmiyor',
        talimat: 'Bu sürümde komutlar yalıtımsız çalışıyor.',
      };
    });
    const ayarlar = await ayarlariAc();
    expect(within(ayarlar).getByText('Yalıtımsız: bu sistemde kullanılamıyor')).toBeTruthy();
    expect(within(ayarlar).queryByText('Araç zinciri dizinleri')).toBeNull();
    expect(within(ayarlar).queryByRole('button', { name: 'Yeniden dene' })).toBeNull();
    expect(arka.komutlar('arac_zinciri_dizinleri')).toHaveLength(0);
  });
});
```

**`ui/src/ana/altaYapisma.test.ts`** (tam içerik):

```ts
import { describe, expect, it } from 'vitest';
import { AltaYapisma, type Kutu, kaydirmaNiyetiMi } from './altaYapisma';

/** Tarayıcı gibi `scrollTop`'u içerik sınırına kıstıran kutu. */
function kutu(scrollHeight: number, clientHeight = 500): Kutu {
  let top = 0;
  return {
    get scrollTop() {
      return top;
    },
    set scrollTop(deger: number) {
      top = Math.max(0, Math.min(deger, this.scrollHeight - this.clientHeight));
    },
    scrollHeight,
    clientHeight,
  };
}

function kur() {
  let simdi = 10_000;
  const y = new AltaYapisma(48, () => simdi);
  return { y, ilerle: (ms: number) => (simdi += ms) };
}

describe('alta yapışma', () => {
  it('kullanıcıdan gelmeyen kaydırma yapışmayı bırakmaz', () => {
    const { y } = kur();
    const k = kutu(1000);
    y.kaydir(k);
    expect(k.scrollTop).toBe(500);
    // Kendi kaydırmamızın olayı gelmeden içerik büyüdü (yeni onay kartı).
    k.scrollHeight = 1200;
    y.olay(k);
    expect(y.altta).toBe(true);
    // Onay sayfası kapanınca odak yukarıdaki düğmeye döndü.
    k.scrollTop = 300;
    y.olay(k);
    expect(y.altta).toBe(true);
    y.kaydir(k);
    expect(k.scrollTop).toBe(700);
  });

  it('kullanıcı yukarı kaydırınca bırakır; en alta inince yeniden yapışır', () => {
    const { y, ilerle } = kur();
    const k = kutu(1000);
    y.kaydir(k);
    y.niyet();
    ilerle(50);
    k.scrollTop = 200;
    y.olay(k);
    expect(y.altta).toBe(false);
    k.scrollHeight = 1300;
    y.kaydir(k);
    expect(k.scrollTop).toBe(200);
    // Niyetin süresi geçse de en alta inmek yeniden yapıştırır.
    ilerle(5000);
    k.scrollTop = 790;
    y.olay(k);
    expect(y.altta).toBe(true);
  });

  it('kaydırma niyeti: tekerlek, sürükleme, kaydırma çubuğu ve kaydırma tuşları', () => {
    const kutu = {};
    const dugme = {};
    expect(kaydirmaNiyetiMi({ type: 'wheel', target: dugme }, kutu)).toBe(true);
    expect(kaydirmaNiyetiMi({ type: 'touchmove', target: dugme }, kutu)).toBe(true);
    expect(kaydirmaNiyetiMi({ type: 'pointerdown', target: kutu }, kutu)).toBe(true);
    // Kutudaki bir düğmeye tıklamak niyet değildir.
    expect(kaydirmaNiyetiMi({ type: 'pointerdown', target: dugme }, kutu)).toBe(false);
    expect(kaydirmaNiyetiMi({ type: 'keydown', target: kutu, key: 'PageUp' }, kutu)).toBe(true);
    expect(kaydirmaNiyetiMi({ type: 'keydown', target: kutu, key: 'Enter' }, kutu)).toBe(false);
    expect(kaydirmaNiyetiMi({ type: 'touchstart', target: kutu }, kutu)).toBe(false);
  });

  it('niyetin süresi geçince gelen kaydırma bırakmaz; sıfırlama en alttan başlatır', () => {
    const { y, ilerle } = kur();
    const k = kutu(1000);
    y.niyet();
    ilerle(1500);
    k.scrollTop = 100;
    y.olay(k);
    expect(y.altta).toBe(true);
    y.niyet();
    y.olay(k);
    expect(y.altta).toBe(false);
    y.sifirla();
    expect(y.altta).toBe(true);
    y.olay(k);
    expect(y.altta).toBe(true);
  });
});
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cd ui && npm test`
Expected: `ayarlar.test.ts` başarısız (menüde "Ayarlar" yok); `altaYapisma.test.ts` derlenmez (`./altaYapisma` yok).

- [ ] **Step 3: Tipleri, IPC'yi, durumu ve metinleri yaz**

**`ui/src/ipc/tipler.ts`**: şu bloğu

```ts
  son_oturum: OturumId | null;
  yalitim: YalitimDurumu;
}
```

şununla değiştir:

```ts
  son_oturum: OturumId | null;
  yalitim: YalitimDurumu;
  /** `linux`, `windows`, `macos`: araç zinciri dizinleri yalnızca Linux'ta. */
  platform: string;
}
```

**`ui/src/ipc/ana.ts`**: şu bloğu

```ts
  OturumSatiri,
  UygulamaDurumu,
} from './tipler';
```

şununla değiştir:

```ts
  OturumSatiri,
  UygulamaDurumu,
  YalitimDurumu,
} from './tipler';
```

**`ui/src/ipc/ana.ts`**: şu bloğu

```ts
export const sartPenceresiniAc = () => invoke<void>('sart_penceresini_ac');
```

şununla değiştir:

```ts
export const sartPenceresiniAc = () => invoke<void>('sart_penceresini_ac');

/**
 * Yalıtım durumunu dinler (aşama 2 tasarımı §10.5): şimdiki durum hemen,
 * sonraki değişimler geldikçe yazılır. Pencere açılınca bir kez çağrılır.
 */
export function yalitimiDinle(dinleyici: (durum: YalitimDurumu) => void) {
  const kanal = new Channel<YalitimDurumu>(dinleyici);
  return invoke<void>('yalitimi_dinle', { kanal });
}

export const yalitimiAlgila = () => invoke<YalitimDurumu>('yalitimi_algila');

export const aracZinciriDizinleri = () => invoke<string[]>('arac_zinciri_dizinleri');

/** Dizini Rust yerel klasör seçiciyle sorar; vazgeçilirse liste değişmez. */
export const aracZinciriDiziniEkle = () => invoke<string[]>('arac_zinciri_dizini_ekle');

export const aracZinciriDiziniKaldir = (yol: string) =>
  invoke<string[]>('arac_zinciri_dizini_kaldir', { yol });
```

**`ui/src/dil/tr.ts`**: şu bloğu

```ts
// Arayüzün Türkçe metin kataloğu (1c tasarımı §9.4). Kurallar: "siz" dili;
// ünlem ve emoji yok; biten iş "Tamamlandı" ile özetlenir; hata metni ne
// olduğunu ve ne yapılacağını söyler. Değişkenler `{ad}` biçimindedir.
import type { AktarimDurmaNedeni, AracDurumu, Kip, ModelProfili } from '../ipc/tipler';

export const tr = {
  'uygulama.adi': 'Kalfa',
```

şununla değiştir:

```ts
// Arayüzün Türkçe metin kataloğu (1c tasarımı §9.4). Kurallar: "siz" dili;
// ünlem ve emoji yok; biten iş "Tamamlandı" ile özetlenir; hata metni ne
// olduğunu ve ne yapılacağını söyler. Değişkenler `{ad}` biçimindedir.
import type {
  AktarimDurmaNedeni,
  AracDurumu,
  Kip,
  ModelProfili,
  YalitimYoklukNedeni,
} from '../ipc/tipler';

export const tr = {
  'uygulama.adi': 'Kalfa',
```

**`ui/src/dil/tr.ts`**: şu bloğu

```ts

  'menu.ad': 'Menü',
  'menu.oturumlar': 'Oturumlar',
  'menu.anahtar': 'Anahtarı değiştir',

  'liste.baslik': 'Oturumlar',
```

şununla değiştir:

```ts

  'menu.ad': 'Menü',
  'menu.oturumlar': 'Oturumlar',
  'menu.ayarlar': 'Ayarlar',
  'menu.anahtar': 'Anahtarı değiştir',

  'liste.baslik': 'Oturumlar',
```

**`ui/src/dil/tr.ts`**: şu bloğu

```ts
  'durum.model_ipucu': 'Yönlendirilen model',
  'durum.yalitimli': 'Yalıtımlı',
  'durum.yalitimsiz': 'Yalıtımsız',

  'duyuru.onay': 'Onay isteği: {sayi} değişiklik kararınızı bekliyor.',
  'duyuru.sart': 'EVREN şartlarının onayı bekleniyor.',
```

şununla değiştir:

```ts
  'durum.model_ipucu': 'Yönlendirilen model',
  'durum.yalitimli': 'Yalıtımlı',
  'durum.yalitimsiz': 'Yalıtımsız',
  'durum.yalitimli_ipucu': 'Komutlar yalıtımda çalışıyor ({arka_uc})',

  'ayarlar.baslik': 'Ayarlar',
  'ayarlar.yalitim': 'Yalıtım',
  'ayarlar.yalitim.hazir': 'Yalıtımlı: {arka_uc} {surum}',
  'ayarlar.yalitim.hazir_aciklama':
    'Komutlar yalıtımda çalışır. Ev dizininiz ve diğer çalışma alanları görünmez, çalışma alanı dışına yazılamaz, ağ kapalıdır; ağ isteyen komut ayrıca onayınıza sunulur.',
  'ayarlar.yalitim.yok': 'Yalıtımsız: {neden}',
  'ayarlar.yalitim.neden.BwrapYok': 'bubblewrap bulunamadı',
  'ayarlar.yalitim.neden.KullaniciAdAlaniKapali': 'sistem kullanıcı ad alanlarını kısıtlıyor',
  'ayarlar.yalitim.neden.KurulumGerekli': 'yalıtımın kurulması gerekiyor',
  'ayarlar.yalitim.neden.KurulumBozuk': 'yalıtımın kurulumu bozuk',
  'ayarlar.yalitim.neden.Desteklenmiyor': 'bu sistemde kullanılamıyor',
  'ayarlar.yalitim.yeniden_dene': 'Yeniden dene',
  'ayarlar.arac_zinciri': 'Araç zinciri dizinleri',
  'ayarlar.arac_zinciri.aciklama':
    "Yalıtımda salt okunur görünen dizinler. Cargo ve rustup araç zincirleri kendiliğinden görünür; başka bir yere kurulan araçlar (örneğin /opt/node) için dizini buradan ekleyin. Ev dizininiz ve Kalfa'nın veri dizini eklenemez.",
  'ayarlar.arac_zinciri.ekle': 'Dizin ekle',
  'ayarlar.arac_zinciri.kaldir': '{yol} dizinini kaldır',
  'ayarlar.arac_zinciri.bos': 'Eklenmiş dizin yok.',

  'duyuru.onay': 'Onay isteği: {sayi} değişiklik kararınızı bekliyor.',
  'duyuru.sart': 'EVREN şartlarının onayı bekleniyor.',
```

**`ui/src/dil/tr.ts`**: şu bloğu

```ts
  Diger: 'aktarim.Diger',
};
```

şununla değiştir:

```ts
  Diger: 'aktarim.Diger',
};

export const YALITIM_NEDENI: Record<YalitimYoklukNedeni, MetinAnahtari> = {
  BwrapYok: 'ayarlar.yalitim.neden.BwrapYok',
  KullaniciAdAlaniKapali: 'ayarlar.yalitim.neden.KullaniciAdAlaniKapali',
  KurulumGerekli: 'ayarlar.yalitim.neden.KurulumGerekli',
  KurulumBozuk: 'ayarlar.yalitim.neden.KurulumBozuk',
  Desteklenmiyor: 'ayarlar.yalitim.neden.Desteklenmiyor',
};
```

**`ui/src/bilesenler/ikonlar.ts`**: şu bloğu

```ts
  Search,
  Send,
  Shield,
  ShieldAlert,
```

şununla değiştir:

```ts
  Search,
  Send,
  Settings,
  Shield,
  ShieldAlert,
```

**`ui/src/bilesenler/ikonlar.ts`**: şu bloğu

```ts
  search: Search,
  send: Send,
  shield: Shield,
  'shield-alert': ShieldAlert,
```

şununla değiştir:

```ts
  search: Search,
  send: Send,
  settings: Settings,
  shield: Shield,
  'shield-alert': ShieldAlert,
```

**`ui/src/sahte/arka.ts`**: şu bloğu

```ts
  TurId,
  UygulamaDurumu,
} from '../ipc/tipler';
```

şununla değiştir:

```ts
  TurId,
  UygulamaDurumu,
  YalitimDurumu,
} from '../ipc/tipler';
```

**`ui/src/sahte/arka.ts`**: şu bloğu

```ts
      talimat: 'Bu sürümde komutlar yalıtımsız çalışıyor.',
    },
  };
  /** `calisma_alani_sec`'in döneceği klasör; `null` vazgeçmektir. */
  klasor: CalismaAlani | null = { kimlik: 1, yol: 'C:\\projeler\\kalfa' };
```

şununla değiştir:

```ts
      talimat: 'Bu sürümde komutlar yalıtımsız çalışıyor.',
    },
    platform: 'linux',
  };
  /** Ayarlar'daki araç zinciri dizinleri. */
  aracZinciri: string[] = [];
  /** `arac_zinciri_dizini_ekle`'nin klasör seçicisinin döneceği dizin; `null` vazgeçmektir. */
  aracKlasoru: string | null = '/opt/node';
  /** `yalitimi_algila` bu durumu bulur; `null` ise durum değişmez. */
  sonrakiYalitim: YalitimDurumu | null = null;
  #yalitimKanali: Channel<YalitimDurumu> | null = null;
  /** `calisma_alani_sec`'in döneceği klasör; `null` vazgeçmektir. */
  klasor: CalismaAlani | null = { kimlik: 1, yol: 'C:\\projeler\\kalfa' };
```

**`ui/src/sahte/arka.ts`**: şu bloğu

```ts
  }

  komutlar(ad: string): Argumanlar[] {
    return this.cagrilar.filter((c) => c.komut === ad).map((c) => c.argumanlar);
```

şununla değiştir:

```ts
  }

  /** Yalıtım durumu değişir ve dinleyen pencereye yazılır. */
  yalitimDurumu(yeni: YalitimDurumu): void {
    this.uygulama.yalitim = yeni;
    this.#yalitimKanali?.onmessage(yeni);
  }

  komutlar(ad: string): Argumanlar[] {
    return this.cagrilar.filter((c) => c.komut === ad).map((c) => c.argumanlar);
```

**`ui/src/sahte/arka.ts`**: şu bloğu

```ts
        this.sart = { tip: 'SartGerekli', metin: this.sartMetni };
        return this.sart;
      default:
        // mesaj_gonder, turu_iptal_et, arac_onayi, model_degistir, paketi_isledim,
```

şununla değiştir:

```ts
        this.sart = { tip: 'SartGerekli', metin: this.sartMetni };
        return this.sart;
      case 'yalitimi_dinle':
        this.#yalitimKanali = a.kanal as Channel<YalitimDurumu>;
        this.#yalitimKanali.onmessage(this.uygulama.yalitim);
        return null;
      case 'yalitimi_algila':
        if (this.sonrakiYalitim) this.yalitimDurumu(this.sonrakiYalitim);
        this.sonrakiYalitim = null;
        return this.uygulama.yalitim;
      case 'arac_zinciri_dizinleri':
        return [...this.aracZinciri];
      case 'arac_zinciri_dizini_ekle':
        if (this.aracKlasoru && !this.aracZinciri.includes(this.aracKlasoru)) {
          this.aracZinciri.push(this.aracKlasoru);
        }
        return [...this.aracZinciri];
      case 'arac_zinciri_dizini_kaldir':
        this.aracZinciri = this.aracZinciri.filter((d) => d !== a.yol);
        return [...this.aracZinciri];
      default:
        // mesaj_gonder, turu_iptal_et, arac_onayi, model_degistir, paketi_isledim,
```

**`ui/src/ana/anaDurum.svelte.ts`**: şu bloğu

```ts
  OturumSatiri,
  UygulamaDurumu,
} from '../ipc/tipler';
```

şununla değiştir:

```ts
  OturumSatiri,
  UygulamaDurumu,
  YalitimDurumu,
} from '../ipc/tipler';
```

**`ui/src/ana/anaDurum.svelte.ts`**: şu bloğu

```ts
  /** Ekran okuyucuya okunan son duyuru (`aria-live`, 1c tasarımı §9.5). */
  duyuru = $state('');
  /** Eski sayfası yüklenen oturumlar; aynı sayfa iki kez istenmez. */
  #eskiYukleniyor = new Set<OturumId>();
```

şununla değiştir:

```ts
  /** Ekran okuyucuya okunan son duyuru (`aria-live`, 1c tasarımı §9.5). */
  duyuru = $state('');
  /** Menünün seçtiği görünüm: oturumlar ya da Ayarlar (aşama 2 tasarımı §10.4). */
  gorunum = $state<'oturumlar' | 'ayarlar'>('oturumlar');
  /** Yalıtıma açılan araç zinciri dizinleri (Linux). */
  aracZinciri = $state<string[]>([]);
  algilaniyor = $state(false);
  /** Eski sayfası yüklenen oturumlar; aynı sayfa iki kez istenmez. */
  #eskiYukleniyor = new Set<OturumId>();
```

**`ui/src/ana/anaDurum.svelte.ts`**: şu bloğu

```ts
      this.uygulama = await ipc.uygulamaDurumu();
    });
    await this.listeyiYenile();
    const son = this.uygulama?.son_oturum;
```

şununla değiştir:

```ts
      this.uygulama = await ipc.uygulamaDurumu();
    });
    await this.#dene(() => ipc.yalitimiDinle((d) => this.#yalitimYaz(d)));
    await this.listeyiYenile();
    const son = this.uygulama?.son_oturum;
```

**`ui/src/ana/anaDurum.svelte.ts`**: şu bloğu

```ts
    }
    this.secili = oturum;
    this.hata = null;
  }
```

şununla değiştir:

```ts
    }
    this.secili = oturum;
    this.gorunum = 'oturumlar';
    this.hata = null;
  }
```

**`ui/src/ana/anaDurum.svelte.ts`**: şu bloğu

```ts
      await this.#kur(acilis);
      this.secili = acilis.oturum;
      this.hata = null;
      await this.listeyiYenile();
```

şununla değiştir:

```ts
      await this.#kur(acilis);
      this.secili = acilis.oturum;
      this.gorunum = 'oturumlar';
      this.hata = null;
      await this.listeyiYenile();
```

**`ui/src/ana/anaDurum.svelte.ts`**: şu bloğu

```ts
  }

  async #ac(oturum: OturumId): Promise<void> {
    await this.#kur(await ipc.oturumAc(oturum));
```

şununla değiştir:

```ts
  }

  /** Ayarlar'ı açar; Linux'ta araç zinciri dizinlerini okur. */
  async ayarlariAc(): Promise<void> {
    this.gorunum = 'ayarlar';
    this.hata = null;
    if (this.uygulama?.platform !== 'linux') return;
    const dizinler = await this.#dene(() => ipc.aracZinciriDizinleri());
    if (dizinler) this.aracZinciri = dizinler;
  }

  oturumlaraDon(): void {
    this.gorunum = 'oturumlar';
    this.hata = null;
  }

  /** Algılamayı yeniden çalıştırır (aşama 2 tasarımı §8.3). */
  async yalitimiAlgila(): Promise<void> {
    this.algilaniyor = true;
    try {
      const yeni = await this.#dene(() => ipc.yalitimiAlgila());
      if (yeni) this.#yalitimYaz(yeni);
    } finally {
      this.algilaniyor = false;
    }
  }

  async aracZinciriEkle(): Promise<void> {
    const dizinler = await this.#dene(() => ipc.aracZinciriDiziniEkle());
    if (dizinler) this.aracZinciri = dizinler;
  }

  async aracZinciriKaldir(yol: string): Promise<void> {
    const dizinler = await this.#dene(() => ipc.aracZinciriDiziniKaldir(yol));
    if (dizinler) this.aracZinciri = dizinler;
  }

  #yalitimYaz(yeni: YalitimDurumu): void {
    if (this.uygulama) this.uygulama.yalitim = yeni;
  }

  async #ac(oturum: OturumId): Promise<void> {
    await this.#kur(await ipc.oturumAc(oturum));
```

- [ ] **Step 4: Ekranları yaz**

**`ui/src/ana/altaYapisma.ts`** (tam içerik):

```ts
// Kaydırılan bir kutunun en alta yapışması: kullanıcı en alttayken içerik
// büyüdükçe kutu da aşağı kayar; kullanıcı yukarı kaydırınca bırakılır.
//
// Kutunun konumunu kullanıcıdan başkası da değiştirir: kendi kaydırmamızın
// geç gelen olayı (o arada içerik yeniden büyümüş olabilir), onay sayfası
// kapanınca odağın geri döndüğü düğme, tarayıcının kaydırma çapası. Bunlara
// bakıp yapışmayı bırakmak yeni içeriği görünmez kılardı. Bu yüzden yapışma
// yalnızca kullanıcının yakın zamandaki kaydırma niyetinden
// (`kaydirmaNiyetiMi`) sonra bırakılır; en alta hangi yoldan inilirse inilsin
// yeniden yapışılır.

export interface Kutu {
  scrollTop: number;
  scrollHeight: number;
  clientHeight: number;
}

/** Kullanıcının niyetinden sonra kaydırma olaylarının ona sayıldığı süre. */
const NIYET_SURESI_MS = 1000;

const KAYDIRMA_TUSLARI = new Set(['PageUp', 'PageDown', 'ArrowUp', 'ArrowDown', 'Home', 'End', ' ']);

/**
 * Olay kullanıcının kutuyu kaydırma niyeti mi? Tekerlek ve dokunarak
 * sürükleme evet; işaretçi yalnızca kutunun kendisine (kaydırma çubuğuna)
 * basınca; tuşlardan yalnızca kaydırma tuşları. Kutunun içindeki bir düğmeye
 * tıklamak niyet değildir: tıklamanın ardından içerik değişip kutu kayabilir.
 */
export function kaydirmaNiyetiMi(
  olay: { type: string; target: unknown; key?: string },
  kutu: unknown,
): boolean {
  switch (olay.type) {
    case 'wheel':
    case 'touchmove':
      return true;
    case 'pointerdown':
      return olay.target === kutu;
    case 'keydown':
      return KAYDIRMA_TUSLARI.has(olay.key ?? '');
    default:
      return false;
  }
}

export class AltaYapisma {
  altta = true;
  #niyet = -Infinity;

  /** `esik`: en alta bu kadar pikselden yakınsa en altta sayılır. */
  constructor(
    private readonly esik: number,
    private readonly saat: () => number = () => performance.now(),
  ) {}

  /** Kullanıcı kaydırmaya niyetlendi (`kaydirmaNiyetiMi`). */
  niyet(): void {
    this.#niyet = this.saat();
  }

  /** İçerik büyüdü: en alttaysa kutuyu en alta kaydırır. */
  kaydir(kutu: Kutu): void {
    if (this.altta) kutu.scrollTop = kutu.scrollHeight;
  }

  /** `scroll` olayı. */
  olay(kutu: Kutu): void {
    if (kutu.scrollHeight - kutu.scrollTop - kutu.clientHeight < this.esik) {
      this.altta = true;
    } else if (this.saat() - this.#niyet < NIYET_SURESI_MS) {
      this.altta = false;
    }
  }

  /** Oturum değişince: en alttan başlanır. */
  sifirla(): void {
    this.altta = true;
    this.#niyet = -Infinity;
  }
}
```

**`ui/src/ana/Konusma.svelte`**: şu bloğu

```svelte
  import { KIP_ADI, MODEL_PROFILI_ADI, m } from '../dil/tr';
  import { KIPLER, type Kip } from '../ipc/tipler';
  import type { AnaDurum } from './anaDurum.svelte';
  import { satirBasligi } from './bicim';
```

şununla değiştir:

```svelte
  import { KIP_ADI, MODEL_PROFILI_ADI, m } from '../dil/tr';
  import { KIPLER, type Kip } from '../ipc/tipler';
  import { AltaYapisma, kaydirmaNiyetiMi } from './altaYapisma';
  import type { AnaDurum } from './anaDurum.svelte';
  import { satirBasligi } from './bicim';
```

**`ui/src/ana/Konusma.svelte`**: şu bloğu

```svelte
  let akis = $state<HTMLDivElement>();
  let icerik = $state<HTMLDivElement>();
  let altta = true;

  // Oturum değişince en alttan başlanır.
  $effect(() => {
    void durum.secili;
    altta = true;
  });
```

şununla değiştir:

```svelte
  let akis = $state<HTMLDivElement>();
  let icerik = $state<HTMLDivElement>();
  const yapisma = new AltaYapisma(48);

  // Oturum değişince en alttan başlanır.
  $effect(() => {
    void durum.secili;
    yapisma.sifirla();
  });
```

**`ui/src/ana/Konusma.svelte`**: şu bloğu

```svelte
    void durum.secili;
    if (!akis) return;
    akis.scrollTop = akis.scrollHeight;
    if (!icerik || typeof ResizeObserver === 'undefined') return;
    const kutu = akis;
    const gozcu = new ResizeObserver(() => {
      if (altta) kutu.scrollTop = kutu.scrollHeight;
    });
    gozcu.observe(icerik);
    return () => gozcu.disconnect();
  });

  function kaydirildi() {
    if (!akis) return;
    altta = akis.scrollHeight - akis.scrollTop - akis.clientHeight < 48;
    if (akis.scrollTop < 40) void eskileriYukle();
  }
```

şununla değiştir:

```svelte
    void durum.secili;
    if (!akis) return;
    yapisma.kaydir(akis);
    if (!icerik || typeof ResizeObserver === 'undefined') return;
    const kutu = akis;
    const gozcu = new ResizeObserver(() => yapisma.kaydir(kutu));
    gozcu.observe(icerik);
    return () => gozcu.disconnect();
  });

  // Kullanıcının kaydırma niyeti (`kaydirmaNiyetiMi`). Yalnızca gözlenir; bu
  // yüzden şablonda değil, pasif dinleyicilerle bağlanır.
  $effect(() => {
    if (!akis) return;
    const kutu = akis;
    const niyet = (e: Event) => {
      if (kaydirmaNiyetiMi(e as Event & { key?: string }, kutu)) yapisma.niyet();
    };
    const olaylar = ['wheel', 'touchmove', 'pointerdown', 'keydown'] as const;
    for (const o of olaylar) kutu.addEventListener(o, niyet, { passive: true });
    return () => {
      for (const o of olaylar) kutu.removeEventListener(o, niyet);
    };
  });

  function kaydirildi() {
    if (!akis) return;
    yapisma.olay(akis);
    if (akis.scrollTop < 40) void eskileriYukle();
  }
```

**`ui/src/ana/Ayarlar.svelte`** (tam içerik):

```svelte
<!--
  Ayarlar (aşama 2 tasarımı §10.4): yalıtımın durumu ve talimatı, algılamayı
  yeniden çalıştırma; Linux'ta yalıtıma açılan araç zinciri dizinleri. Dizin
  yerel klasör seçiciyle eklenir; arayüz yol göndermez. Windows'un düğmeleri
  aşama 2d'de gelir.
-->
<script lang="ts">
  import Button from '../bilesenler/Button.svelte';
  import Card from '../bilesenler/Card.svelte';
  import Icon from '../bilesenler/Icon.svelte';
  import IconButton from '../bilesenler/IconButton.svelte';
  import { YALITIM_NEDENI, m } from '../dil/tr';
  import type { AnaDurum } from './anaDurum.svelte';

  let { durum }: { durum: AnaDurum } = $props();

  const yalitim = $derived(durum.uygulama?.yalitim ?? null);
  const linux = $derived(durum.uygulama?.platform === 'linux');
</script>

<section class="ayarlar" aria-label={m('ayarlar.baslik')}>
  {#if durum.hata}
    <div class="serit" role="alert">
      <Icon name="triangle-alert" size={15} />
      <span>{durum.hata}</span>
      <IconButton icon="x" size="sm" label={m('genel.kapat')} onclick={() => (durum.hata = null)} />
    </div>
  {/if}
  <header class="ust">
    <h2>{m('ayarlar.baslik')}</h2>
  </header>
  <div class="govde">
    <Card title={m('ayarlar.yalitim')} icon="shield">
      {#snippet actions()}
        {#if linux}
          <Button
            size="sm"
            iconLeft="rotate-ccw"
            loading={durum.algilaniyor}
            onclick={() => durum.yalitimiAlgila()}
          >
            {m('ayarlar.yalitim.yeniden_dene')}
          </Button>
        {/if}
      {/snippet}
      {#if yalitim?.tip === 'Hazir'}
        <p class="durum ok">
          <Icon name="shield-check" size={16} />
          {m('ayarlar.yalitim.hazir', { arka_uc: yalitim.arka_uc, surum: yalitim.surum })}
        </p>
        <p class="aciklama">{m('ayarlar.yalitim.hazir_aciklama')}</p>
      {:else if yalitim}
        <p class="durum uyari">
          <Icon name="shield-alert" size={16} />
          {m('ayarlar.yalitim.yok', { neden: m(YALITIM_NEDENI[yalitim.neden]) })}
        </p>
        <p class="aciklama">{yalitim.talimat}</p>
      {/if}
    </Card>

    {#if linux}
      <Card title={m('ayarlar.arac_zinciri')} icon="folder">
        {#snippet actions()}
          <Button size="sm" iconLeft="folder-plus" onclick={() => durum.aracZinciriEkle()}>
            {m('ayarlar.arac_zinciri.ekle')}
          </Button>
        {/snippet}
        <p class="aciklama">{m('ayarlar.arac_zinciri.aciklama')}</p>
        {#if durum.aracZinciri.length > 0}
          <ul class="dizinler" aria-label={m('ayarlar.arac_zinciri')}>
            {#each durum.aracZinciri as yol (yol)}
              <li>
                <span class="yol">{yol}</span>
                <IconButton
                  icon="x"
                  size="sm"
                  label={m('ayarlar.arac_zinciri.kaldir', { yol })}
                  onclick={() => durum.aracZinciriKaldir(yol)}
                />
              </li>
            {/each}
          </ul>
        {:else}
          <p class="bos">{m('ayarlar.arac_zinciri.bos')}</p>
        {/if}
      </Card>
    {/if}
  </div>
</section>

<style>
  .ayarlar {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-app);
  }
  .serit {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 22px;
    font: var(--type-small);
    color: var(--status-danger);
    background: color-mix(in srgb, var(--status-danger) 14%, transparent);
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .serit span {
    flex: 1;
  }
  .ust {
    padding: 14px 22px;
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  h2 {
    margin: 0;
    font: var(--fw-semibold) 20px / 1.2 var(--font-sans);
    letter-spacing: var(--tracking-tight);
    color: var(--text-title);
  }
  .govde {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 20px 22px;
    max-width: 760px;
  }
  .durum {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 8px;
    font: var(--type-body);
    font-weight: var(--fw-semibold);
  }
  .ok {
    color: var(--status-ok);
  }
  .uyari {
    color: var(--status-warn);
  }
  .aciklama,
  .bos {
    margin: 0;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .bos {
    margin-top: 10px;
    color: var(--text-muted);
  }
  .dizinler {
    list-style: none;
    margin: 12px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 4px 4px 10px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
  }
  .yol {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: var(--type-meta);
    color: var(--text-body);
  }
</style>
```

**`ui/src/ana/Ana.svelte`**: şu bloğu

```svelte
<!--
  Ana pencere (1c tasarımı §8.1): menü, oturum listesi, konuşma ve en altta
  durum çubuğu. Onay sayfası konuşmanın üstünde açılır.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { AnaDurum } from './anaDurum.svelte';
  import DurumCubugu from './DurumCubugu.svelte';
  import Konusma from './Konusma.svelte';
```

şununla değiştir:

```svelte
<!--
  Ana pencere (1c tasarımı §8.1): menü, oturum listesi, konuşma ve en altta
  durum çubuğu. Onay sayfası konuşmanın üstünde açılır. Menüden Ayarlar
  seçilince liste ve konuşmanın yerine Ayarlar gelir.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { AnaDurum } from './anaDurum.svelte';
  import Ayarlar from './Ayarlar.svelte';
  import DurumCubugu from './DurumCubugu.svelte';
  import Konusma from './Konusma.svelte';
```

**`ui/src/ana/Ana.svelte`**: şu bloğu

```svelte
  <div class="govde">
    <Menu {durum} />
    <OturumListesi {durum} />
    <Konusma {durum} />
  </div>
  <DurumCubugu {durum} />
```

şununla değiştir:

```svelte
  <div class="govde">
    <Menu {durum} />
    {#if durum.gorunum === 'ayarlar'}
      <Ayarlar {durum} />
    {:else}
      <OturumListesi {durum} />
      <Konusma {durum} />
    {/if}
  </div>
  <DurumCubugu {durum} />
```

**`ui/src/ana/Menu.svelte`**: şu bloğu

```svelte
<!-- Menü (1c tasarımı §8.1): 1c'de yalnızca Oturumlar ve "Anahtarı değiştir". -->
<script lang="ts">
  import NavItem from '../bilesenler/NavItem.svelte';
```

şununla değiştir:

```svelte
<!-- Menü (1c tasarımı §8.1): Oturumlar, Ayarlar (aşama 2 tasarımı §10.4) ve "Anahtarı değiştir". -->
<script lang="ts">
  import NavItem from '../bilesenler/NavItem.svelte';
```

**`ui/src/ana/Menu.svelte`**: şu bloğu

```svelte
    <div class="slogan">{m('uygulama.slogan')}</div>
  {/snippet}
  <NavItem icon="message-square" active>{m('menu.oturumlar')}</NavItem>
  <NavItem icon="key-round" onclick={() => durum.anahtariDegistir()}>
    {m('menu.anahtar')}
```

şununla değiştir:

```svelte
    <div class="slogan">{m('uygulama.slogan')}</div>
  {/snippet}
  <NavItem
    icon="message-square"
    active={durum.gorunum === 'oturumlar'}
    onclick={() => durum.oturumlaraDon()}
  >
    {m('menu.oturumlar')}
  </NavItem>
  <NavItem
    icon="settings"
    active={durum.gorunum === 'ayarlar'}
    onclick={() => durum.ayarlariAc()}
  >
    {m('menu.ayarlar')}
  </NavItem>
  <NavItem icon="key-round" onclick={() => durum.anahtariDegistir()}>
    {m('menu.anahtar')}
```

**`ui/src/ana/DurumCubugu.svelte`**: şu bloğu

```svelte
  Durum çubuğu (1c tasarımı §8.3): oturumun hâli, bağlantı ve kimlik deposu
  uyarıları; son gözlenen bakiye ve zamanı, token kullanımı (düşünce dökümüyle),
  yönlendirilen model, izin kipi. Bağlam doluluğu gösterilmez.
-->
<script lang="ts">
```

şununla değiştir:

```svelte
  Durum çubuğu (1c tasarımı §8.3): oturumun hâli, bağlantı ve kimlik deposu
  uyarıları; son gözlenen bakiye ve zamanı, token kullanımı (düşünce dökümüyle),
  yönlendirilen model, izin kipi. Bağlam doluluğu gösterilmez. Yalıtım
  göstergesine tıklanınca Ayarlar açılır (aşama 2 tasarımı §10.3).
-->
<script lang="ts">
```

**`ui/src/ana/DurumCubugu.svelte`**: şu bloğu

```svelte
      <StatusItem icon="wifi-off" tone="danger">{m('durum.baglanti_yok')}</StatusItem>
    {/if}
    {#if durum.uygulama?.yalitim.tip === 'Hazir'}
      <StatusItem icon="shield-check" tone="ok">{m('durum.yalitimli')}</StatusItem>
    {:else if durum.uygulama?.yalitim.tip === 'Yok'}
      <StatusItem icon="shield-alert" tone="warn" title={durum.uygulama.yalitim.talimat}>
        {m('durum.yalitimsiz')}
      </StatusItem>
    {/if}
    {#if durum.uygulama?.anahtar_deposu === 'Bellekte'}
```

şununla değiştir:

```svelte
      <StatusItem icon="wifi-off" tone="danger">{m('durum.baglanti_yok')}</StatusItem>
    {/if}
    {#if durum.uygulama}
      {@const yalitim = durum.uygulama.yalitim}
      <button type="button" class="yalitim" onclick={() => durum.ayarlariAc()}>
        {#if yalitim.tip === 'Hazir'}
          <StatusItem
            icon="shield-check"
            tone="ok"
            title={m('durum.yalitimli_ipucu', { arka_uc: yalitim.arka_uc })}
          >
            {m('durum.yalitimli')}
          </StatusItem>
        {:else}
          <StatusItem icon="shield-alert" tone="warn" title={yalitim.talimat}>
            {m('durum.yalitimsiz')}
          </StatusItem>
        {/if}
      </button>
    {/if}
    {#if durum.uygulama?.anahtar_deposu === 'Bellekte'}
```

**`ui/src/ana/DurumCubugu.svelte`**: şu bloğu

```svelte
</StatusBar>
```

şununla değiştir:

```svelte
</StatusBar>

<style>
  .yalitim {
    display: inline-flex;
    padding: 0;
    border: 0;
    background: none;
    font: inherit;
    cursor: pointer;
  }
  .yalitim:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
    border-radius: var(--radius-sm);
  }
</style>
```

**`ui/src/onizleme/senaryo.ts`**: şu bloğu

```ts
  const simdi = Date.now();
  arka.uygulama.son_oturum = 'o1';
  arka.oturumEkle(
    {
```

şununla değiştir:

```ts
  const simdi = Date.now();
  arka.uygulama.son_oturum = 'o1';
  // Ayarlar (aşama 2 tasarımı §10.4): "Yeniden dene" yalıtımı bulur.
  arka.aracZinciri = ['/opt/node'];
  arka.sonrakiYalitim = { tip: 'Hazir', arka_uc: 'bwrap', surum: '0.9.0' };
  arka.oturumEkle(
    {
```

- [ ] **Step 5: Testlerin geçtiğini gör**

Run: `cd ui && npm run check && npm test && npm run build`
Expected: check 0 hata 0 uyarı; 20 dosya, 218 test; paket denetimi temiz.

- [ ] **Step 6: Commit**

```bash
git add ui/src
git commit -m "ui: Ayarlar ekranı, yalıtım göstergesi ve konuşmanın alta yapışması"
```

---

### Task 7: Uçtan uca: kabuk Linux'ta gerçek yalıtımda

**Files:**
- Modify: `e2e/yardimci/kalfa.mjs`, `e2e/testler/05-kabuk.test.mjs`

**Interfaces:**
- Consumes: Görev 1–6.
- Produces: `Kalfa.ac` Linux'ta `node`'un kurulu olduğu dizini (sistem dizinlerinde değilse) veri dizinindeki `yalitim.json`'a yazar; `Kalfa.onaySayfasiniAc()`.

**Davranış notları:**
- Linux'ta mevcut üç kabuk testi gerçek yalıtımda koşar (onay kartında karar yerel onay değil, olağan onaydır; akış aynıdır).
- Yeni test (yalnızca Linux): durum çubuğu "Yalıtımlı" gösterir; onay sayfası "Yalıtımlı (bwrap)" der; komut `.env`'i okuyamaz ve ağa bağlanamaz.
- `Kalfa.onaySayfasiniAc()`: "İncele"ye basar ve onay sayfası açılana kadar yeniden basar. Araç kartı adım adım çizilirken içerik kayar; WebDriver'ın tıklaması düğmenin eski konumuna düşebiliyordu. Kabuk testleri bununla onaylar.
- Linux'ta uygulama `WEBKIT_DISABLE_COMPOSITING_MODE=1` ile açılır. Görev 6'da konuşma en altta kalınca akan metin görünür oldu; Xvfb'de GPU olmadığı için WebKitGTK'nın GL birleştiricisi yazılımla çalışıyor ve görünür her karede ~25 ms bekletiyor: gecikme testleri (p95 < 50 ms) 56–64 ms'ye çıktı. Birleştirme kapalıyken p95 33–38 ms. Ürün kodu değişmez; eşik değişmez.

- [ ] **Step 1: Testleri yaz**

**`e2e/yardimci/kalfa.mjs`**: şu bloğu

```js
import { connect } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { remote } from 'webdriverio';
```

şununla değiştir:

```js
import { connect } from 'node:net';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { remote } from 'webdriverio';
```

**`e2e/yardimci/kalfa.mjs`**: şu bloğu

```js
    mkdirSync(veri);
    mkdirSync(alan);
    const ayarDosyasi = join(kok, 'e2e.json');
    writeFileSync(ayarDosyasi, JSON.stringify({ calisma_alani: alan, ...ayarlar }));
```

şununla değiştir:

```js
    mkdirSync(veri);
    mkdirSync(alan);
    // Linux'ta kabuk yalıtımda çalışır (aşama 2 tasarımı §12.5). Testlerin
    // `node`'u sistem dizinlerinde değilse (örneğin /opt/node) yalıtıma açılır;
    // kullanıcı bunu Ayarlar'dan yapar.
    const nodeKoku = dirname(dirname(process.execPath));
    if (process.platform === 'linux' && !nodeKoku.startsWith('/usr')) {
      writeFileSync(
        join(veri, 'yalitim.json'),
        JSON.stringify({ arac_zinciri_dizinleri: [nodeKoku] }),
      );
    }
    const ayarDosyasi = join(kok, 'e2e.json');
    writeFileSync(ayarDosyasi, JSON.stringify({ calisma_alani: alan, ...ayarlar }));
```

**`e2e/yardimci/kalfa.mjs`**: şu bloğu

```js
        KALFA_ANAHTAR_DEPOSU: 'bellek',
        KALFA_E2E: ayarDosyasi,
      },
      stdio: ['ignore', gunluk, gunluk],
```

şununla değiştir:

```js
        KALFA_ANAHTAR_DEPOSU: 'bellek',
        KALFA_E2E: ayarDosyasi,
        // Xvfb'de GPU yok: WebKitGTK'nın GL birleştiricisi yazılımla
        // (llvmpipe) çalışır ve görünür her değişiklikte bir sonraki kare
        // ~25 ms gecikir. Gecikme ölçümü birleştiricinin değil arayüzün
        // gecikmesini sormalı; yazılım çiziminde birleştirme kapatılır.
        ...(process.platform === 'linux' ? { WEBKIT_DISABLE_COMPOSITING_MODE: '1' } : {}),
      },
      stdio: ['ignore', gunluk, gunluk],
```

**`e2e/yardimci/kalfa.mjs`**: şu bloğu

```js

  /**
   * Pencereyi kapatan düğmeye basar ("Kabul ediyorum"). Pencere tıklamanın
   * yanıtından önce kapanabilir; sürücünün "no such window" hatası o zaman
```

şununla değiştir:

```js

  /**
   * Onay kartındaki "İncele" ile onay sayfasını açar. İçerik o sırada büyüyüp
   * kayıyorsa (araç kartı adım adım çizilirken) tıklama düğmenin eski konumuna
   * düşebilir; sayfa açılana kadar yeniden basılır.
   */
  async onaySayfasiniAc() {
    await sureyleBekle(
      async () => {
        await this.dugme('İncele');
        return this.b.execute(() => document.querySelector('[role="dialog"]') !== null);
      },
      20000,
      'onay sayfası açılmadı',
    );
  }

  /**
   * Pencereyi kapatan düğmeye basar ("Kabul ediyorum"). Pencere tıklamanın
   * yanıtından önce kapanabilir; sürücünün "no such window" hatası o zaman
```

**`e2e/testler/05-kabuk.test.mjs`**: şu bloğu

```js
// Kabuk (aşama 2 tasarımı §12.5): onay sayfası → yerel onay → canlı çıktı →
// sonuç; süren komutun iptali süreç ağacını durdurur; çıktı seli arayüzü
// kilitlemez ve turu iptal ettirmez. 2a'da yalıtım yoktur: her komut yerel
// pencerede onaylanır (sahte iletişim kutusu `onay: true` ile onaylar).
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync } from 'node:fs';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa, sureyleBekle } from '../yardimci/kalfa.mjs';
```

şununla değiştir:

```js
// Kabuk (aşama 2 tasarımı §12.5): onay sayfası → onay → canlı çıktı → sonuç;
// süren komutun iptali süreç ağacını durdurur; çıktı seli arayüzü kilitlemez ve
// turu iptal ettirmez. Linux'ta komutlar gerçek yalıtımda (bwrap) çalışır;
// Windows'ta yalıtım aşama 2d'ye kadar yoktur ve her komut yerel pencerede
// onaylanır (sahte iletişim kutusu `onay: true` ile onaylar).
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa, sureyleBekle } from '../yardimci/kalfa.mjs';
```

**`e2e/testler/05-kabuk.test.mjs`**: şu bloğu

```js
        },
      ],
      [{ tur: 'metin', metin: 'olmamalı' }],
    ],
  },
  'kabuk-sel': {
```

şununla değiştir:

```js
        },
      ],
      [{ tur: 'metin', metin: 'olmamalı' }],
    ],
  },
  'kabuk-gizli': {
    yanitlar: [
      [
        {
          tur: 'arac',
          ad: 'kabuk',
          argumanlar: node(
            "const fs = require('fs'); " +
              "try { console.log('env=' + fs.readFileSync('.env', 'utf8')); } " +
              "catch (e) { console.log('env-okunamadi'); } " +
              "require('net').connect(80, '1.1.1.1')" +
              ".on('error', () => console.log('ag-kapali'))" +
              ".on('connect', () => { console.log('ag-acik'); process.exit(0); });",
          ),
        },
      ],
      [{ tur: 'metin', metin: 'Yalıtım denendi.' }],
    ],
  },
  'kabuk-sel': {
```

**`e2e/testler/05-kabuk.test.mjs`**: şu bloğu

```js
  );
}

async function onayla(k) {
  await k.dugme('İncele');
  await k.dugme('Tümünü onayla');
}
```

şununla değiştir:

```js
  );
}

/** Seçiciye uyan ilk öğenin metni. */
async function sayfaMetni(k, secici) {
  return k.b.execute((s) => document.querySelector(s)?.textContent ?? '', secici);
}

async function onayla(k) {
  await k.onaySayfasiniAc();
  await k.dugme('Tümünü onayla');
}
```

**`e2e/testler/05-kabuk.test.mjs`**: şu bloğu

```js
  });
});
```

şununla değiştir:

```js
  });
});

describe('kabuk yalıtımı (Linux)', { skip: process.platform !== 'linux' }, () => {
  let k;
  before(async () => {
    k = await Kalfa.ac({
      sart: 'kabul',
      anahtar: GECERLI_ANAHTAR,
      onay: true,
      senaryolar: SENARYOLAR,
    });
    writeFileSync(join(k.alan, '.env'), 'GIZLI_DEGER=e2e-sir\n');
    await k.pencere('ana');
    await k.yeniOturum();
  });
  after(async () => {
    await k?.kapat();
  });

  it('komut yalıtımda çalışır: gizli dosya okunamaz, ağ kapalıdır', async () => {
    await sureyleBekle(
      async () => (await sayfaMetni(k, '.durum-cubugu')).includes('Yalıtımlı'),
      10000,
      'durum çubuğu yalıtımı göstermedi',
    );
    await k.gonder('kabuk-gizli');
    await k.onaySayfasiniAc();
    assert.match(await sayfaMetni(k, '[role="dialog"]'), /Yalıtımlı \(bwrap\)/);
    await k.dugme('Tümünü onayla');
    assert.equal(await k.turSonuBekle(1), 'Normal');
    await k.metniBekle('Yalıtım denendi.');
    const terminal = await terminalMetni(k);
    assert.match(terminal, /env-okunamadi/);
    assert.match(terminal, /ag-kapali/);
    assert.doesNotMatch(terminal, /e2e-sir/);
  });
});
```

- [ ] **Step 2: Uçtan uca testleri koştur**

Run: `cd e2e && npm ci && npm test` (Linux'ta `xvfb-run -a npm test`)
Expected: Windows'ta 24 test geçer, Linux'a özel yalıtım testi atlanır; Linux'ta 25 test geçer. Sürücüler yoksa PR'a yaz; Claude iki platformda koşturur.

- [ ] **Step 3: Commit**

```bash
git add e2e
git commit -m "e2e: kabuk Linux'ta gerçek yalıtımda"
```

---

### Task 8: Belgeler ve son doğrulama

**Files:**
- Modify: `README.md`, `docs/GELISTIRME_DURUMU.md`

- [ ] **Step 1: Son doğrulama**

Global Constraints'teki bütün denetimleri koştur; uçtan uca testleri koşturabildiysen onları da.

- [ ] **Step 2: Belgeleri güncelle**

**`README.md`**: şu bloğu

```markdown
onayınla. Araçlarını WebAssembly eklentileriyle genişletebilirsin.

> **Durum: masaüstü uygulaması Windows ve Linux'ta çalışıyor; kabuk aracı
> var, yalıtımı henüz yok.** Rust workspace'inde EVREN istemcisi
> (`kalfa-evren`), arayüzden bağımsız oturum çekirdeği (`kalfa-core`), dosya
> araçları ve kabuk aracı (`kalfa-tools`), komut çalıştırma ve süreç sınırları
> (`kalfa-sandbox`) ve masaüstü uygulaması (`kalfa-uygulama`, Tauri kabuğu
> `kalfa`) bulunuyor; Svelte arayüzü `ui/`'dadır. Komutlar şimdilik yalıtımsız
> çalışır ve her biri yerel pencerede onaylanır; yalıtım ve eklentiler sonraki
> aşamalarda. [Geliştirme durumu ve devam noktası](docs/GELISTIRME_DURUMU.md).

## Geliştirme
```

şununla değiştir:

```markdown
onayınla. Araçlarını WebAssembly eklentileriyle genişletebilirsin.

> **Durum: masaüstü uygulaması Windows ve Linux'ta çalışıyor; kabuk komutları
> Linux'ta yalıtımda çalışıyor.** Rust workspace'inde EVREN istemcisi
> (`kalfa-evren`), arayüzden bağımsız oturum çekirdeği (`kalfa-core`), dosya
> araçları ve kabuk aracı (`kalfa-tools`), komut çalıştırma, süreç sınırları ve
> Linux yalıtımı (`kalfa-sandbox`) ve masaüstü uygulaması (`kalfa-uygulama`,
> Tauri kabuğu `kalfa`) bulunuyor; Svelte arayüzü `ui/`'dadır. Linux'ta
> komutlar bubblewrap ve seccomp ile yalıtılır. Windows'ta yalıtım henüz yok:
> komutlar yalıtımsız çalışır ve her biri yerel pencerede onaylanır. Windows
> yalıtımı ve eklentiler sonraki aşamalarda.
> [Geliştirme durumu ve devam noktası](docs/GELISTIRME_DURUMU.md).

## Geliştirme
```

**`README.md`**: şu bloğu

```markdown
(`libwebkit2gtk-4.1-dev`).

Arayüz (`ui/`, Node `^22.22.2 || >=24.15.0`):
```

şununla değiştir:

````markdown
(`libwebkit2gtk-4.1-dev`).

Linux'ta kabuk komutlarını yalıtmak için `bubblewrap` gerekir (`sudo apt install
bubblewrap`). Yoksa Kalfa bunu durum çubuğunda ve Ayarlar'da söyler; komutlar
yalıtımsız çalışır ve her biri yerel pencerede onaylanır. Gerçek yalıtım
testleri root olmayan bir kullanıcıyla koşar:

```sh
cargo test -p kalfa-sandbox --features yalitim-testleri --test yalitim
cargo test -p kalfa-tools --features yalitim-testleri --test uyumluluk
```

Arayüz (`ui/`, Node `^22.22.2 || >=24.15.0`):
````

**`README.md`**: şu bloğu

```markdown
- **Windows:** WebView2 sürümüyle aynı `msedgedriver` gerekir. PATH'te değilse
  yolu `KALFA_E2E_EDGEDRIVER` ile verin.
- **Linux:** `webkit2gtk-driver` ve `xvfb` paketleri gerekir.

`npm test` önce arayüzü test kancasıyla (`ui/dist-e2e`), sonra uygulamayı
```

şununla değiştir:

```markdown
- **Windows:** WebView2 sürümüyle aynı `msedgedriver` gerekir. PATH'te değilse
  yolu `KALFA_E2E_EDGEDRIVER` ile verin.
- **Linux:** `webkit2gtk-driver`, `xvfb` ve `bubblewrap` paketleri gerekir.

`npm test` önce arayüzü test kancasıyla (`ui/dist-e2e`), sonra uygulamayı
```

**`docs/GELISTIRME_DURUMU.md`**: şu bloğu

```markdown
# Kalfa geliştirme durumu

Güncelleme: 2026-09-24. Bu dosya uygulamanın devam noktasıdır; tasarım
belgelerindeki tarihsel inceleme kayıtlarının yerine geçmez.
```

şununla değiştir:

```markdown
# Kalfa geliştirme durumu

Güncelleme: 2026-09-25. Bu dosya uygulamanın devam noktasıdır; tasarım
belgelerindeki tarihsel inceleme kayıtlarının yerine geçmez.
```

**`docs/GELISTIRME_DURUMU.md`**: şu bloğu

```markdown
  Svelte arayüzü `ui/`; ayrıntısı aşağıda.
- **2a — kabuk aracı:** `kabuk` (`kalfa-tools`) ve komut çalıştırma, süreç
  sınırları (`kalfa-sandbox`). Yalıtım henüz yok; her komut yerel pencerede
  onaylanır. Ayrıntısı aşağıda.
- **Uçtan uca:** gerçek dosya araçlarıyla, arayüz olmadan, okuma → onay →
  düzenleme → model yanıtı → kayıttan kurtarma akışı test edilir; aynı akış
```

şununla değiştir:

```markdown
  Svelte arayüzü `ui/`; ayrıntısı aşağıda.
- **2a — kabuk aracı:** `kabuk` (`kalfa-tools`) ve komut çalıştırma, süreç
  sınırları (`kalfa-sandbox`). Ayrıntısı aşağıda.
- **2b — Linux yalıtımı:** kabuk komutları Linux'ta bubblewrap ve seccomp ile
  yalıtımda çalışır (`kalfa-sandbox`); Ayarlar'da yalıtım durumu ve araç
  zinciri dizinleri. Windows'ta yalıtım henüz yok; komutlar yalıtımsız çalışır
  ve her biri yerel pencerede onaylanır. Ayrıntısı aşağıda.
- **Uçtan uca:** gerçek dosya araçlarıyla, arayüz olmadan, okuma → onay →
  düzenleme → model yanıtı → kayıttan kurtarma akışı test edilir; aynı akış
```

**`docs/GELISTIRME_DURUMU.md`**: şu bloğu

```markdown
  onay komutuyla yerel onaylı öğeyi onaylamıyor.

Sıradaki plan 2b: Linux arka ucu (bubblewrap + seccomp).

Kabuk yalıtımı, bağlam özetleme, geri alma, gönderim günlüğü, oturum bütçesi,
```

şununla değiştir:

```markdown
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
    `ptrace` gibi çağrıları ve Unix soketlerini engeller.
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

Sıradaki plan 2c: Windows prototipi. Ürün koduna dokunmaz; yönetici adımlarını
kullanıcı kendi makinesinde çalıştırır.

Kabuk yalıtımı, bağlam özetleme, geri alma, gönderim günlüğü, oturum bütçesi,
```

Tablodaki sayılar plan yazılırken ölçülendir; senin sonuçların farklıysa PR yorumunda yaz, belgeyi değiştirme.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/GELISTIRME_DURUMU.md
git commit -m "docs: 2b Linux yalıtımı"
```

---

## İnceleme (Claude)

- Plan commit'inden sonraki her commit'in plandaki görevle eşleşmesi; kodun plandakiyle aynı olması.
- Windows'ta: Global Constraints'teki bütün denetimler ve uçtan uca testler (24).
- Linux'ta (WSL2): aynı denetimler; `yalitim` (17) ve `uyumluluk` (4) testleri sıradan kullanıcıyla; uçtan uca testler (25).
- Güvenlik sınırlarının testleri: `tests/yalitim.rs`'in her satırı, isteğin onaylandığı kipte çalışması, araç zinciri dizininin reddedilen yolları, yeni komutların yalnızca ana pencereye açık olması.
- Belgelerdeki sayıların inceleme sonuçlarıyla aynı olması.
