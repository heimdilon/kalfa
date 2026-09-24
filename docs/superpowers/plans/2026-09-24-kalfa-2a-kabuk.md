# Kalfa 2a — Kabuk Aracı ve Süreç Sınırları Uygulama Planı

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Modelin çalışma alanında komut çalıştırabilmesi. `kabuk` aracı komutu yalıtımsız ama süreç sınırlarıyla çalıştırır; her komut, her kipte yerel pencerede onaylanır; çıktı çalışırken arayüzde akar. Yalıtım (bwrap, Windows) 2b–2d'dedir.

**Architecture:**
- **`kalfa-sandbox` (yeni crate):** komutu başlatır, çıktıyı okur, süreyi ve iptali uygular, bitince süreç ağacını sonlandırır (Linux'ta süreç grubu, Windows'ta Job Object). Ortam değişkenlerini izin listesiyle süzer. `Yalitici` trait'inin 2a'daki tek uygulaması `Yalitimsiz`'dır. `kalfa-core`'a bağlı değildir.
- **Çekirdek:**
  - Kabuk tarifi (`KabukTarifi`) ve kabuk izin kararı.
  - Yalnızca yerel pencereden verilebilen onay: `Karar::YerelOnayli`, `Komut::YerelOnay`, `OturumTutamaci::bekleyen_onay`.
  - Ara çıktı olayı (`AracCiktisiParcasi`) ve kayıplı iletimi.
- **Araç:** `kalfa-tools`'taki `kabuk` aracı argümanları doğrular, tarifi üretir, komutu çalıştırır; modele baştan ve sondan kırpılmış çıktıyı, arayüze canlı akışı verir.
- **Uygulama:** kabuk aracını oturuma bağlar. Yerel onay penceresini açar; pencerede gösterilen komutu çekirdekten okur. Çalışma alanındaki ilk onayı kaydeder. Yalıtım durumunu `uygulama_durumu`'na ekler.
- **Arayüz:** araç kartında terminal görünümü (ANSI renkleri, sınırlı metin), onay sayfasında kabuk ayrıntısı, durum çubuğunda yalıtım göstergesi.

**Tech Stack:** Rust 1.98 · tokio 1.53 (`process`, `io-util`) · libc 0.2 (Linux) · windows-sys 0.61 (Job Object, Toolhelp) · Tauri 2.11 · Svelte 5 · Vitest · WebdriverIO 9.32 (uçtan uca)

**Spec:** `docs/superpowers/specs/2026-09-24-kalfa-2-kabuk-yalitim-design.md` (aşama 2 tasarımı): §3, §4, §5, §6, §10, §11, §12.1, §12.4, §12.5. Tasarım belgesinin 2a ayrıntılarıyla güncellenmiş hâli plan commit'indedir.

**Kod hakkında:** Bu plandaki kodun tamamı plan yazılırken ayrı bir kopyada derlendi ve doğrulandı (2026-09-24).

- **Windows 11 (WebView2 153):**
  - `cargo test --workspace --locked`: 348 test geçti.
  - `e2e` özellikli testler (63) geçti; normal ve `e2e` derlemesinde clippy ve fmt temiz.
  - `ui/`'da check (0/0), test (18 dosya, 205 test) ve build temiz.
  - Uçtan uca: 24 test (1c'nin 21'i ve 3 kabuk testi) iki tam koşuda ikisinde de geçti; gecikme p95'i 24–26 ms.
- **Linux (Ubuntu 24.04, WSL2, Xvfb, WebKitGTK 2.52):**
  - `cargo test --workspace --locked`: 346 test geçti. Aradaki fark Windows'a özel testler.
  - `e2e` özellikli testler (63), clippy ve fmt aynı; `ui/` aynı.
  - Uçtan uca: aynı 24 test geçti; kabuk testi ayrıca üç kez art arda koşturuldu, hepsi geçti.

Kodu olduğu gibi kullan. Bir şey yine de derlenmezse testin sınadığı davranışı değiştirmeden düzelt; davranış değişmesi gerekiyorsa dur ve PR'a yorum yaz.

**Bu planın kapsamı dışında:**
- Yalıtım: Linux arka ucu (2b), Windows prototipi (2c) ve arka ucu (2d). 2a'da `YalitimDurumu` hep "yok"tur.
- Ayarlardaki yalıtım bölümü, yalıtım durumunun değişim olayı (2b/2d).
- "Bu oturumda izin ver" kısayolu (aşama 3; kuralı tasarım §5.3'te).
- Yalıtımsız Linux'ta Kalfa çökerse komutun süreç grubunun sonlandırılması (tasarım §6.4; 2b'de bwrap kapatır).
- Elle kontroller ve sürekli tümleştirme.

## PR ile yürütme

Kullanıcı kararı (2026-09-23): kodu Codex yazar, Claude inceler.

- **Dal ve PR:** çalışma dalı `2a/kabuk`, PR'ın hedefi `kalfa-asama-1`. Dal, aşama 2 tasarımının commit'iyle başlar.
- **Hazır gelenler (plan commit'i):**
  - Plan ve tasarım belgesinin 2a güncellemesi.
  - Kök `Cargo.toml`: `kalfa-sandbox` üyesi, `libc` ve `windows-sys`'in yeni özellikleri.
  - `crates/kalfa-sandbox/Cargo.toml` ve yer tutan `crates/kalfa-sandbox/src/lib.rs` (Görev 1 doldurur).
  - `crates/kalfa-tools/Cargo.toml` ve `crates/kalfa-uygulama/Cargo.toml`: `kalfa-sandbox` bağımlılığı; `kalfa-uygulama`'da `serde_json` artık her zaman gelir.
  - `Cargo.lock`.

  `lib.rs` dışında bu dosyaları değiştirme.
- **Ortam:**
  - **Araç sürümleri:** Rust 1.98.1 (`rust-toolchain.toml`); Node `^22.22.2 || >=24.15.0`, npm 10 ya da üstü.
  - **Ağ:** `npm ci` için ağ erişimi gerekir. Yeni Rust crate'i indirilmez; hepsi kilit dosyasında.
  - **Uçtan uca testler için** (Görev 6): `tauri-driver` PATH'te; Windows'ta WebView2 sürümüyle aynı `msedgedriver` (PATH'te değilse `KALFA_E2E_EDGEDRIVER`); Linux'ta `webkit2gtk-driver` ve `xvfb`. `node` PATH'te olmalıdır: kabuk testleri komut olarak `node` çalıştırır. Sürücüleri kendin indirme ya da kurma; ortamında yoksa PR'a yaz. Claude uçtan uca testleri incelemede iki platformda koşturur.
  - **Linux'ta `kalfa` crate'i:** `libwebkit2gtk-4.1-dev` ister.
  - **Biçim:** biçimlendirici (Prettier ve benzeri) projede yok; koşturma. Kod plandaki biçimiyle yazılır. Rust için `cargo fmt` kullanılır.
- **Plandaki değişiklik biçimi:**
  - "(tam içerik)" başlıklı dosya olduğu gibi yazılır: yeni dosya oluşturulur, var olanın içeriği değiştirilir.
  - "şu bloğu … şununla değiştir" çiftlerinde eski blok dosyada tam olarak bir kez geçer. Çiftler dosyadaki sırasıyla verilmiştir; sırayla uygula.
- **Sıra:** görevler sırayla. Her görev planda yazan mesajla kendi commit'ini alır; her commit'ten sonra push'la. Geçmiş yeniden yazılmaz, force-push yapılmaz.
- **Soru ve sapma:** plandan sapman gerekirse dur ve PR'a yorum yaz; kendi başına karar verme.
- **İnceleme:** Claude PR'a yorum yazar, Windows'ta ve Linux'ta doğrular, uçtan uca testleri iki platformda koşturur; inceleme temizse birleştirir.

## Global Constraints

- **Araçlar:** Rust 1.98.1; `cargo` komutları `--locked` ile. Node `^22.22.2 || >=24.15.0`; kurulum `npm ci`.
- **Bağımlılık:** yeni crate ya da npm paketi eklenmez. `kalfa-sandbox`'ın bağımlılıkları workspace'tendir ve plan commit'indedir.
- **Güvenlik sınırları (testler sınar):**
  - **Yerel onay:** yalıtımsız komut yalnızca yerel pencerede onaylanır. Pencerede gösterilen komut `OturumTutamaci::bekleyen_onay`'dan, yani çekirdekten okunur; webview'dan gelen hiçbir metin gösterilmez. Çekirdek, yerel onaylı öğeyi `Komut::AracOnayi` ile onaylamaz.
  - **Ortam:** komutun ortamı izin listesiyle kurulur. Adında `KEY`, `TOKEN`, `SECRET`, `PASSWORD`, `PASSWD`, `CREDENTIAL` ya da `AUTH` geçen değişken hiçbir koşulda geçmez.
  - **Program ve dizin:** mutlak program yolu reddedilir; çalışma dizini çalışma alanının içinde, bağlantısız bir dizin olmalıdır. Standart girdi kapalıdır.
  - **Süreç ağacı:** iptal, zaman aşımı ve komutun bitişi bütün ağacı sonlandırır.
  - **Terminal görünümü:** çıktı metin düğümü olarak eklenir; `{@html}`, `innerHTML` ve `eval` yok. ANSI'den yalnızca renk ve kalınlık çizilir.
- **Kayıplı canlı çıktı:** ara çıktı turu hiçbir zaman iptal ettirmez. Araç tarafında kanal, oturum tarafında olay kuyruğunun yarısı dolunca parça atılır ve atılan bayt bildirilir. Modelin aldığı çıktı eksiksizdir (kırpma dışında).
- **`unsafe`:** yalnızca `crates/kalfa-sandbox/src/agac.rs`'de, `SAFETY` açıklamasıyla.
- **CSP (1c-3a):** şablonda sabit `style="…"` yok. `ui/src/csp.test.ts` yeni dosyaları da tarar.
- **Dil:**
  - Kod ve testlerdeki adlar ile açıklamalar Türkçedir.
  - Kullanıcıya görünen metinler `ui/src/dil/tr.ts`'tedir ya da Rust'ta sabittir; plandaki gibi yazılır.
  - Rust'ta `unwrap()` ve `expect()` yalnızca testlerde ve `test_destek.rs`'de kullanılır.
- **Platform farkı:** betik Linux'ta `/bin/sh -c`, Windows'ta PowerShell (`-EncodedCommand`) ile çalışır. Testlerde platforma göre betik seçilir (`sh_ya_da_ps`, `betik`).
- **Her görevin sonunda:**
  - Rust tarafı: `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings` ve `cargo fmt --all -- --check` geçer.
  - `kalfa-uygulama`'ya dokunan görevlerde ayrıca `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked` ve `cargo clippy -p kalfa -p kalfa-uygulama --features e2e --all-targets --locked -- -D warnings` geçer.
  - Arayüze dokunan görevlerde `ui/` içinde `npm run check` (0 hata, 0 uyarı), `npm test` ve `npm run build` (paket denetimi dahil) geçer.
- **Commit:** mesajlar Türkçe (`sandbox: …`, `çekirdek: …`, `araçlar: …`, `uygulama: …`, `ui: …`, `e2e: …`, `docs: …`). Codex'in commit'lerine Claude satırı eklenmez.
- **Dokunulmayanlar:**
  - Plan commit'indeki dosyalar (`crates/kalfa-sandbox/src/lib.rs` dışında).
  - `crates/kalfa/` (2a'da değişmez), `design/`.
  - `.env`, `Screenshot*.png`, kök dizindeki `tasks.png`.

## Dosya Yapısı

```
crates/kalfa-sandbox/               yeni crate
  src/lib.rs          YalitimDurumu, KomutBicimi, CalistirmaIstegi, Yalitici, BaslatmaHatasi
  src/surec.rs        Surec: çıktı okuma, süre, iptal, ağacın sonlandırılması
  src/agac.rs         süreç ağacı: Linux süreç grubu, Windows Job Object
  src/ortam.rs        ortam değişkeni izin listesi
  src/program.rs      programın PATH/PATHEXT'te bulunması, betiğin yorumlayıcısı
  src/yalitimsiz.rs   Yalitimsiz arka ucu
  tests/yalitimsiz.rs gerçek süreçlerle testler
crates/kalfa-core/
  src/kabuk.rs        KabukTarifi, KabukBicimi, YalitimOzeti
  src/arac.rs, izin.rs, muhur.rs, protokol.rs, aktarim.rs, lib.rs, oturum/mod.rs,
  oturum/arac_asamasi.rs                                     (değişir)
  tests/oturum_kabuk.rs                                      yeni
  tests/protokol.rs, izin.rs, arac_muhur.rs, aktarim.rs      (değişir)
crates/kalfa-tools/
  src/kabuk/mod.rs    KabukAraci, KabukGuveni
  src/kabuk/cikti.rs  BasSonTampon, temizle, CanliAkis
  src/lib.rs, src/calisma_alani.rs                           (değişir)
  tests/kabuk.rs                                             yeni
crates/kalfa-uygulama/
  src/calisma_alanlari.rs  CalismaAlaniKaydi, AlanGuveni
  src/lib.rs, uygulama.rs, oturumlar.rs, sozlesme.rs, guvenlik.rs, e2e/mod.rs (değişir)
  tests/kabuk.rs                                             yeni
ui/src/
  durum/terminal.ts, ana/ansi.ts, ana/TerminalGorunumu.svelte   yeni (+ testleri)
  ipc/tipler.ts, sahte/arka.ts, bilesenler/ikonlar.ts, dil/tr.ts, durum/oturum.ts,
  ana/{bicim.ts, onay.ts, KonusmaOgesi.svelte, OnaySayfasi.svelte, DurumCubugu.svelte},
  onizleme/senaryo.ts                                        (değişir)
e2e/testler/05-kabuk.test.mjs                                yeni
README.md, docs/GELISTIRME_DURUMU.md                         (değişir)
```

---

### Task 1: `kalfa-sandbox`: yalıtımsız komut çalıştırma ve süreç sınırları

**Files:**
- Create: `crates/kalfa-sandbox/src/{agac,surec,ortam,program,yalitimsiz}.rs`, `crates/kalfa-sandbox/tests/yalitimsiz.rs`
- Modify: `crates/kalfa-sandbox/src/lib.rs` (plan commit'indeki yer tutucu)

**Interfaces:**
- Consumes: yok (crate `kalfa-core`'a bağlı değildir).
- Produces (`kalfa_sandbox`):
  - `YalitimDurumu::{Hazir { arka_uc, surum }, Yok { neden: YalitimYoklukNedeni, talimat }}` (serde `tip` etiketli), `hazir_mi()`; `YalitimYoklukNedeni::{BwrapYok, KullaniciAdAlaniKapali, KurulumGerekli, KurulumBozuk, Desteklenmiyor}`.
  - `KomutBicimi::{Program { program, argumanlar }, Betik { metin }}`, `CalistirmaIstegi { komut, dizin: PathBuf, ortam: Vec<(String, String)>, ag }`.
  - `trait Yalitici: Send + Sync { fn durum(&self) -> YalitimDurumu; fn baslat(&self, &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi>; }`, `BaslatmaHatasi::{ProgramYok { program, path }, GirisCikis(io::Error)}`.
  - `Surec::bekle(self, sure: Duration, iptal: CancellationToken, cikti: impl FnMut(Akis, &[u8])) -> CalismaSonucu`; `Akis::{Standart, Hata}`, `Bitis::{Cikti(Option<i32>), ZamanAsimi, Iptal}`, `CalismaSonucu { bitis, sure }`.
  - `Yalitimsiz` (`Yalitici`), `yalitimsiz::TALIMAT`, `AZAMI_SUREC = 256`.
  - `ortam::{uret, izinli_mi, deger, SABIT}`, `program::{coz, bul, yorumlayici_adi}`.

**Davranış notları (testler sınar):**
- **Ortam:** boş başlar; izin listesindekiler ve `SABIT` değerleri (`TERM=dumb`, `GIT_TERMINAL_PROMPT=0`, `GIT_PAGER=cat`, `PAGER=cat`, `PYTHONIOENCODING=utf-8`, `PYTHONUTF8=1`) eklenir. Windows'ta adlar büyük/küçük harf ayırmaz.
- **Program:** çıplak ad komutun ortamındaki `PATH`'te aranır; Windows'ta `PATHEXT` uzantıları denenir (`npm` → `npm.cmd`). Linux'ta çalıştırılamayan dosya atlanır.
- **Betik:** Linux'ta `/bin/sh -c`; Windows'ta `powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -EncodedCommand <UTF-16LE base64>`. Betiğin önüne UTF-8 çıktı ve sessiz ilerleme satırı eklenir.
- **Süreç ağacı:**
  - Linux'ta komut kendi süreç grubunda başlar; durdurma gruba `SIGTERM`, 2 sn sonra `SIGKILL` gönderir.
  - Windows'ta komut askıda başlar, Job Object'e (`KILL_ON_JOB_CLOSE`, en çok 256 süreç) atanır, sonra sürdürülür; durdurma işi sonlandırır.
  - Komut kendisi bitince de ağaç durdurulur: arka planda kalan torunlar yaşamaz.
- **Bekleme:** çıktı borularının kapanması komut bittikten sonra en çok 2 sn beklenir. Dönüşte ağaç sonlandırılmıştır.

- [ ] **Step 1: Başarısız testleri yaz**

**`crates/kalfa-sandbox/tests/yalitimsiz.rs`** (tam içerik):

```rust
//! Yalıtımsız arka uç gerçek süreçlerle (aşama 2 tasarımı §6, §12.1).

use std::time::Duration;

use kalfa_sandbox::{
    Akis, BaslatmaHatasi, Bitis, CalismaSonucu, CalistirmaIstegi, KomutBicimi, Yalitici,
    Yalitimsiz, ortam,
};
use tokio_util::sync::CancellationToken;

fn betik(metin: &str) -> KomutBicimi {
    KomutBicimi::Betik {
        metin: metin.to_string(),
    }
}

/// Platforma göre betik: Linux'ta sh, Windows'ta PowerShell.
fn sh_ya_da_ps(sh: &str, ps: &str) -> KomutBicimi {
    betik(if cfg!(windows) { ps } else { sh })
}

fn istek(komut: KomutBicimi) -> CalistirmaIstegi {
    CalistirmaIstegi {
        komut,
        dizin: std::env::temp_dir(),
        ortam: ortam::uret(std::env::vars()),
        ag: false,
    }
}

struct Toplanan {
    sonuc: CalismaSonucu,
    standart: String,
    hata: String,
}

async fn calistir(komut: KomutBicimi, sure: Duration, iptal: CancellationToken) -> Toplanan {
    let surec = Yalitimsiz.baslat(&istek(komut)).expect("komut başlamalı");
    let mut standart = Vec::new();
    let mut hata = Vec::new();
    let sonuc = surec
        .bekle(sure, iptal, |akis, veri| match akis {
            Akis::Standart => standart.extend_from_slice(veri),
            Akis::Hata => hata.extend_from_slice(veri),
        })
        .await;
    Toplanan {
        sonuc,
        standart: String::from_utf8_lossy(&standart).into_owned(),
        hata: String::from_utf8_lossy(&hata).into_owned(),
    }
}

const UZUN: Duration = Duration::from_secs(60);

#[tokio::test]
async fn cikis_kodu_ve_iki_akis_ayri_gelir() {
    let t = calistir(
        sh_ya_da_ps(
            "echo merhaba; echo sorun 1>&2; exit 3",
            "Write-Output merhaba; [Console]::Error.WriteLine('sorun'); exit 3",
        ),
        UZUN,
        CancellationToken::new(),
    )
    .await;
    assert_eq!(t.sonuc.bitis, Bitis::Cikti(Some(3)));
    assert_eq!(t.standart.trim(), "merhaba");
    assert_eq!(t.hata.trim(), "sorun");
}

#[tokio::test]
async fn program_bicimi_kabuksuz_calisir() {
    let komut = if cfg!(windows) {
        KomutBicimi::Program {
            program: "cmd".into(),
            argumanlar: vec!["/c".into(), "echo tamam".into()],
        }
    } else {
        KomutBicimi::Program {
            program: "sh".into(),
            argumanlar: vec!["-c".into(), "echo tamam".into()],
        }
    };
    let t = calistir(komut, UZUN, CancellationToken::new()).await;
    assert_eq!(t.sonuc.bitis, Bitis::Cikti(Some(0)));
    assert_eq!(t.standart.trim(), "tamam");
}

#[tokio::test]
async fn olmayan_program_baslatilmaz() {
    let hata = Yalitimsiz
        .baslat(&istek(KomutBicimi::Program {
            program: "kalfa-kesinlikle-olmayan-program".into(),
            argumanlar: vec![],
        }))
        .err()
        .expect("başlamamalı");
    assert!(matches!(hata, BaslatmaHatasi::ProgramYok { .. }), "{hata}");
}

#[tokio::test]
async fn izin_listesi_disindaki_ortam_gecmez() {
    // `cargo test` bu değişkeni test sürecine verir; izin listesinde yok.
    assert!(std::env::var("CARGO_MANIFEST_DIR").is_ok());
    let t = calistir(
        sh_ya_da_ps(
            "echo \"${CARGO_MANIFEST_DIR:-yok}|$TERM\"",
            "if ($env:CARGO_MANIFEST_DIR) { 'var|' + $env:TERM } else { 'yok|' + $env:TERM }",
        ),
        UZUN,
        CancellationToken::new(),
    )
    .await;
    assert_eq!(t.standart.trim(), "yok|dumb");
}

#[tokio::test]
async fn zaman_asimi_agaci_durdurur() {
    let t = calistir(
        sh_ya_da_ps("sleep 30", "Start-Sleep -Seconds 30"),
        Duration::from_millis(1500),
        CancellationToken::new(),
    )
    .await;
    assert_eq!(t.sonuc.bitis, Bitis::ZamanAsimi);
    assert!(t.sonuc.sure < Duration::from_secs(10), "{:?}", t.sonuc.sure);
}

#[tokio::test]
async fn iptal_torun_sureci_de_durdurur() {
    let iptal = CancellationToken::new();
    let tetik = iptal.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(2500)).await;
        tetik.cancel();
    });
    // Komut arka planda bir torun başlatır, onun kimliğini yazar ve bekler.
    let t = calistir(
        sh_ya_da_ps(
            "sleep 60 & echo $!; wait",
            "$p = Start-Process -PassThru -WindowStyle Hidden ping -ArgumentList '-n','60','127.0.0.1'; \
             Write-Output $p.Id; Start-Sleep -Seconds 60",
        ),
        UZUN,
        iptal,
    )
    .await;
    assert_eq!(t.sonuc.bitis, Bitis::Iptal);
    let torun: u32 = t
        .standart
        .trim()
        .parse()
        .expect("torunun kimliği yazılmalı");
    for _ in 0..30 {
        if !yasiyor_mu(torun) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("torun süreç {torun} hâlâ yaşıyor");
}

#[tokio::test]
async fn buyuk_cikti_eksiksiz_okunur() {
    let t = calistir(
        sh_ya_da_ps(
            "i=0; while [ $i -lt 20000 ]; do echo 0123456789012345678901234567890123456789012345678; i=$((i+1)); done",
            "$s = '0123456789012345678901234567890123456789012345678'; \
             [Console]::Out.Write((($s + \"`n\") * 20000))",
        ),
        UZUN,
        CancellationToken::new(),
    )
    .await;
    assert_eq!(t.sonuc.bitis, Bitis::Cikti(Some(0)));
    assert_eq!(t.standart.lines().count(), 20000);
}

#[cfg(unix)]
fn yasiyor_mu(pid: u32) -> bool {
    // Ölmüş ama biçilmemiş (zombi) süreç de ölü sayılır.
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .map(|stat| !stat.split(") ").nth(1).is_some_and(|s| s.starts_with('Z')))
        .unwrap_or(false)
}

#[cfg(windows)]
fn yasiyor_mu(pid: u32) -> bool {
    let cikti = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
        .output()
        .expect("tasklist çalışmalı");
    String::from_utf8_lossy(&cikti.stdout).contains(&format!("\"{pid}\""))
}
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa-sandbox --locked`
Expected: derleme hatası (`kalfa_sandbox::Yalitimsiz` ve diğerleri yok).

- [ ] **Step 3: Crate'i yaz**

**`crates/kalfa-sandbox/src/lib.rs`** (tam içerik):

```rust
//! Kabuk komutlarının çalıştırılması (aşama 2 tasarımı §3, §6).
//!
//! `Yalitici` bir komutu başlatır; `Surec` çıktısını okur, süre ve iptali
//! uygular, bitince bütün süreç ağacını sonlandırır. Süreç sınırları
//! yalıtımdan bağımsızdır: yalıtımsız arka uç da onları uygular. Crate
//! `kalfa-core`'a bağlı değildir.

mod agac;
pub mod ortam;
pub mod program;
mod surec;
mod yalitimsiz;

use std::path::PathBuf;

use serde::Serialize;

pub use surec::{Akis, Bitis, CalismaSonucu, Surec};
pub use yalitimsiz::Yalitimsiz;

/// Süreç ağacının en çok süreç sayısı (aşama 2 tasarımı §6.3).
pub const AZAMI_SUREC: u32 = 256;

/// Yalıtımın kullanılabilirliği (aşama 2 tasarımı §3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum YalitimDurumu {
    Hazir {
        arka_uc: String,
        surum: String,
    },
    Yok {
        neden: YalitimYoklukNedeni,
        talimat: String,
    },
}

impl YalitimDurumu {
    pub fn hazir_mi(&self) -> bool {
        matches!(self, Self::Hazir { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum YalitimYoklukNedeni {
    BwrapYok,
    KullaniciAdAlaniKapali,
    KurulumGerekli,
    KurulumBozuk,
    Desteklenmiyor,
}

/// Çalıştırılacak komutun biçimi (aşama 2 tasarımı §4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KomutBicimi {
    /// Kabuk yorumlayıcısı olmadan: `program` çıplak bir ad ya da mutlak bir
    /// yoldur. Çıplak ad komutun ortamındaki `PATH`'te aranır.
    Program {
        program: String,
        argumanlar: Vec<String>,
    },
    /// Platformun kabuğunda: Linux'ta `/bin/sh -c`, Windows'ta PowerShell.
    Betik { metin: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalistirmaIstegi {
    pub komut: KomutBicimi,
    /// Mutlak çalışma dizini.
    pub dizin: PathBuf,
    /// Komutun ortamı; `ortam::uret` ile üretilir.
    pub ortam: Vec<(String, String)>,
    /// Komut ağa erişmek istiyor. Yalıtımsız arka uç ağı kapatamaz.
    pub ag: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum BaslatmaHatasi {
    #[error("`{program}` bulunamadı. Komutun PATH'i: {path}")]
    ProgramYok { program: String, path: String },
    #[error("Komut başlatılamadı: {0}")]
    GirisCikis(#[from] std::io::Error),
}

/// Bir komutu başlatan arka uç (aşama 2 tasarımı §3.2).
pub trait Yalitici: Send + Sync {
    fn durum(&self) -> YalitimDurumu;
    fn baslat(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi>;
}
```

**`crates/kalfa-sandbox/src/agac.rs`** (tam içerik):

```rust
//! Süreç ağacı (aşama 2 tasarımı §6.3, §6.4). Linux'ta komut kendi süreç
//! grubunda başlar, sonlandırma gruba gider. Windows'ta komut askıda başlar,
//! bir Job Object'e atanır ve sonra sürdürülür; iş kapanınca ağacın tamamı
//! ölür.

use std::io;

use tokio::process::{Child, Command};

pub(crate) struct Agac {
    #[cfg(unix)]
    grup: i32,
    #[cfg(windows)]
    is: windows::Is,
}

/// Komutu ağacın kökü olarak başlatır.
pub(crate) fn baslat(komut: &mut Command) -> io::Result<(Child, Agac)> {
    #[cfg(unix)]
    {
        komut.process_group(0);
        let cocuk = komut.spawn()?;
        let grup = cocuk
            .id()
            .and_then(|p| i32::try_from(p).ok())
            .ok_or_else(|| io::Error::other("sürecin kimliği okunamadı"))?;
        Ok((cocuk, Agac { grup }))
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, CREATE_SUSPENDED};
        let is = windows::Is::yeni()?;
        komut.creation_flags(CREATE_SUSPENDED | CREATE_NO_WINDOW);
        let cocuk = komut.spawn()?;
        let hazir = (|| {
            let tutamac = cocuk
                .raw_handle()
                .ok_or_else(|| io::Error::other("sürecin tutamacı yok"))?;
            is.ata(tutamac)?;
            let kimlik = cocuk
                .id()
                .ok_or_else(|| io::Error::other("sürecin kimliği okunamadı"))?;
            windows::surdur(kimlik)
        })();
        if let Err(e) = hazir {
            // İşe atanamayan süreç askıda kalmasın.
            is.sonlandir();
            return Err(e);
        }
        Ok((cocuk, Agac { is }))
    }
}

impl Agac {
    /// Ağaca kibarca durmasını söyler (Linux'ta `SIGTERM`). Windows'ta
    /// kibar sonlandırma yoktur; ağaç hemen ölür.
    pub(crate) fn durdur(&self) {
        // SAFETY: `killpg` yalnızca bir sayı ve sinyal alır; grup yoksa ESRCH
        // döner, bellek güvenliğini etkilemez.
        #[cfg(unix)]
        unsafe {
            libc::killpg(self.grup, libc::SIGTERM);
        }
        #[cfg(windows)]
        self.is.sonlandir();
    }

    /// Ağacı öldürür.
    pub(crate) fn oldur(&self) {
        // SAFETY: `durdur`'daki gibi.
        #[cfg(unix)]
        unsafe {
            libc::killpg(self.grup, libc::SIGKILL);
        }
        #[cfg(windows)]
        self.is.sonlandir();
    }
}

/// Ağaç bırakılınca (örneğin görev iptal edilince) içindeki süreçler de
/// ölür. Windows'ta bunu işin son tutamacının kapanması yapar.
#[cfg(unix)]
impl Drop for Agac {
    fn drop(&mut self) {
        self.oldur();
    }
}

#[cfg(windows)]
mod windows {
    use std::io;
    use std::os::windows::io::RawHandle;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
    };
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JobObjectExtendedLimitInformation, SetInformationJobObject, TerminateJobObject,
    };
    use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

    use crate::AZAMI_SUREC;

    /// Kapanınca içindeki bütün süreçleri öldüren Job Object.
    pub(super) struct Is(HANDLE);

    // Tutamaç yalnızca çekirdek nesnesine bir başvurudur; iş parçacıkları
    // arasında taşınabilir.
    unsafe impl Send for Is {}
    unsafe impl Sync for Is {}

    // SAFETY (bu `impl` ve `surdur` için): Windows API'lerine yalnızca açık
    // tutamaçlar ve sıfırla başlatılıp boyutu doğru verilmiş yapılar geçer.
    // `Is`'in tutamacı değer yaşadıkça açıktır; `Drop`'ta bir kez kapanır.
    impl Is {
        pub(super) fn yeni() -> io::Result<Self> {
            let is = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if is.is_null() {
                return Err(io::Error::last_os_error());
            }
            let is = Self(is);
            let mut bilgi: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
            bilgi.BasicLimitInformation.LimitFlags =
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
            bilgi.BasicLimitInformation.ActiveProcessLimit = AZAMI_SUREC;
            let tamam = unsafe {
                SetInformationJobObject(
                    is.0,
                    JobObjectExtendedLimitInformation,
                    (&raw const bilgi).cast(),
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            };
            if tamam == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(is)
        }

        pub(super) fn ata(&self, surec: RawHandle) -> io::Result<()> {
            if unsafe { AssignProcessToJobObject(self.0, surec as HANDLE) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }

        pub(super) fn sonlandir(&self) {
            unsafe {
                TerminateJobObject(self.0, 1);
            }
        }
    }

    impl Drop for Is {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    /// Askıda başlatılan sürecin iş parçacıklarını sürdürür.
    pub(super) fn surdur(surec: u32) -> io::Result<()> {
        let anlik = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
        if anlik == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let mut giris: THREADENTRY32 = unsafe { std::mem::zeroed() };
        giris.dwSize = size_of::<THREADENTRY32>() as u32;
        let mut surduruldu = false;
        let mut var = unsafe { Thread32First(anlik, &mut giris) } != 0;
        while var {
            if giris.th32OwnerProcessID == surec {
                let is_parcacigi =
                    unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, giris.th32ThreadID) };
                if !is_parcacigi.is_null() {
                    unsafe {
                        ResumeThread(is_parcacigi);
                        CloseHandle(is_parcacigi);
                    }
                    surduruldu = true;
                }
            }
            var = unsafe { Thread32Next(anlik, &mut giris) } != 0;
        }
        unsafe {
            CloseHandle(anlik);
        }
        if surduruldu {
            Ok(())
        } else {
            Err(io::Error::other("askıdaki sürecin iş parçacığı bulunamadı"))
        }
    }
}
```

**`crates/kalfa-sandbox/src/surec.rs`** (tam içerik):

```rust
//! Başlamış bir komut: çıktının okunması, süre, iptal ve süreç ağacının
//! sonlandırılması (aşama 2 tasarımı §6.2, §6.4).

use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::time::{Instant, sleep_until};
use tokio_util::sync::CancellationToken;

use crate::agac::{self, Agac};
use crate::{BaslatmaHatasi, CalistirmaIstegi, program};

/// `SIGTERM`'den sonra `SIGKILL`'e kadar beklenen süre.
const KIBAR_SURE: Duration = Duration::from_secs(2);
/// Komut bittikten sonra çıktı borularının kapanması için beklenen en uzun
/// süre. Boruyu tutan torun süreçler ağaçla birlikte sonlandırılır.
const BOSALTMA_SURESI: Duration = Duration::from_secs(2);
const OKUMA_PARCASI: usize = 8 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Akis {
    Standart,
    Hata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bitis {
    /// Komut kendisi çıktı. Sinyalle ölen süreçte kod yoktur.
    Cikti(Option<i32>),
    ZamanAsimi,
    Iptal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalismaSonucu {
    pub bitis: Bitis,
    pub sure: Duration,
}

pub struct Surec {
    cocuk: Child,
    agac: Agac,
}

impl Surec {
    /// İsteği yalıtımsız başlatır: yalnızca süreç sınırları uygulanır.
    pub(crate) fn baslat(istek: &CalistirmaIstegi) -> Result<Self, BaslatmaHatasi> {
        let (yol, argumanlar) = program::coz(&istek.komut, &istek.ortam)?;
        let mut komut = Command::new(yol);
        komut
            .args(argumanlar)
            .env_clear()
            .envs(istek.ortam.iter().map(|(a, d)| (a, d)))
            .current_dir(&istek.dizin)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let (cocuk, agac) = agac::baslat(&mut komut)?;
        Ok(Self { cocuk, agac })
    }

    /// Komut bitene, süre dolana ya da iptal gelene kadar bekler; çıktı
    /// parçalarını geldikçe `cikti`'ya verir. Döndüğünde süreç ağacı
    /// sonlandırılmıştır.
    pub async fn bekle(
        mut self,
        sure: Duration,
        iptal: CancellationToken,
        mut cikti: impl FnMut(Akis, &[u8]),
    ) -> CalismaSonucu {
        let baslangic = Instant::now();
        let (tx, mut rx) = mpsc::channel::<(Akis, Vec<u8>)>(16);
        let mut okuyucular = Vec::new();
        if let Some(o) = self.cocuk.stdout.take() {
            okuyucular.push(tokio::spawn(oku(o, Akis::Standart, tx.clone())));
        }
        if let Some(o) = self.cocuk.stderr.take() {
            okuyucular.push(tokio::spawn(oku(o, Akis::Hata, tx.clone())));
        }
        drop(tx);

        let son = baslangic + sure;
        let mut bitis: Option<Bitis> = None;
        let mut cikis_alindi = false;
        let mut boru_kapali = false;
        let mut oldurme: Option<Instant> = None;
        let mut bosaltma: Option<Instant> = None;
        let uzak = baslangic + Duration::from_secs(365 * 24 * 3600);

        loop {
            if cikis_alindi && boru_kapali {
                break;
            }
            tokio::select! {
                biased;
                parca = rx.recv(), if !boru_kapali => match parca {
                    Some((akis, veri)) => cikti(akis, &veri),
                    None => boru_kapali = true,
                },
                durum = self.cocuk.wait(), if !cikis_alindi => {
                    cikis_alindi = true;
                    if bitis.is_none() {
                        bitis = Some(Bitis::Cikti(durum.ok().and_then(|d| d.code())));
                    }
                    // Komut bitti; geride kalan torunlar da durur.
                    self.agac.durdur();
                    oldurme.get_or_insert(Instant::now() + KIBAR_SURE);
                    bosaltma = Some(Instant::now() + BOSALTMA_SURESI);
                }
                _ = sleep_until(son), if bitis.is_none() => {
                    bitis = Some(Bitis::ZamanAsimi);
                    self.agac.durdur();
                    oldurme = Some(Instant::now() + KIBAR_SURE);
                }
                _ = iptal.cancelled(), if bitis.is_none() => {
                    bitis = Some(Bitis::Iptal);
                    self.agac.durdur();
                    oldurme = Some(Instant::now() + KIBAR_SURE);
                }
                _ = sleep_until(oldurme.unwrap_or(uzak)), if oldurme.is_some() => {
                    self.agac.oldur();
                    oldurme = None;
                }
                _ = sleep_until(bosaltma.unwrap_or(uzak)), if bosaltma.is_some() => break,
            }
        }
        self.agac.oldur();
        for okuyucu in okuyucular {
            okuyucu.abort();
        }
        CalismaSonucu {
            bitis: bitis.unwrap_or(Bitis::Cikti(None)),
            sure: baslangic.elapsed(),
        }
    }
}

async fn oku(mut kaynak: impl AsyncRead + Unpin, akis: Akis, tx: mpsc::Sender<(Akis, Vec<u8>)>) {
    let mut tampon = vec![0u8; OKUMA_PARCASI];
    loop {
        match kaynak.read(&mut tampon).await {
            Ok(0) | Err(_) => return,
            Ok(n) => {
                if tx.send((akis, tampon[..n].to_vec())).await.is_err() {
                    return;
                }
            }
        }
    }
}
```

**`crates/kalfa-sandbox/src/ortam.rs`** (tam içerik):

```rust
//! Komutun ortam değişkenleri (aşama 2 tasarımı §6.1): ortam boş başlar,
//! yalnızca izin listesindekiler geçer. EVREN anahtarı, token'lar ve ajan
//! soketleri hiçbir koşulda aktarılmaz.

/// Her platformda geçen değişkenler.
const ORTAK: &[&str] = &[
    "PATH",
    "HOME",
    "USER",
    "LOGNAME",
    "LANG",
    "LANGUAGE",
    "TZ",
    "TMPDIR",
    "TEMP",
    "TMP",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
    "GOPATH",
    "GOROOT",
    "JAVA_HOME",
];

/// Windows'ta sistem araçlarının çalışması için gerekenler.
const WINDOWS: &[&str] = &[
    "SystemRoot",
    "windir",
    "ComSpec",
    "PATHEXT",
    "SystemDrive",
    "ProgramFiles",
    "ProgramFiles(x86)",
    "ProgramW6432",
    "ProgramData",
    "CommonProgramFiles",
    "CommonProgramFiles(x86)",
    "CommonProgramW6432",
    "NUMBER_OF_PROCESSORS",
    "PROCESSOR_ARCHITECTURE",
    "PROCESSOR_IDENTIFIER",
    "PROCESSOR_LEVEL",
    "PROCESSOR_REVISION",
    "OS",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "HOMEDRIVE",
    "HOMEPATH",
    "USERNAME",
];

/// Bu önekle başlayanlar geçer.
const ONEKLER: &[&str] = &["LC_"];

/// Adında bunlardan biri geçen değişken, izin listesinde olsa da geçmez.
const YASAK_PARCALAR: &[&str] = &[
    "KEY",
    "TOKEN",
    "SECRET",
    "PASSWORD",
    "PASSWD",
    "CREDENTIAL",
    "AUTH",
];

/// Etkileşim bekleyen araçlar için sabit değerler (aşama 2 tasarımı §4.3).
pub const SABIT: &[(&str, &str)] = &[
    ("TERM", "dumb"),
    ("GIT_TERMINAL_PROMPT", "0"),
    ("GIT_PAGER", "cat"),
    ("PAGER", "cat"),
    ("PYTHONIOENCODING", "utf-8"),
    ("PYTHONUTF8", "1"),
];

fn ayni_ad(a: &str, b: &str) -> bool {
    // Windows'ta ortam değişkeni adları büyük/küçük harf ayırmaz.
    if cfg!(windows) {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

/// Değişken komuta geçebilir mi?
pub fn izinli_mi(ad: &str) -> bool {
    let buyuk = ad.to_ascii_uppercase();
    if YASAK_PARCALAR.iter().any(|p| buyuk.contains(p)) {
        return false;
    }
    let listede = ORTAK
        .iter()
        .chain(if cfg!(windows) { WINDOWS } else { &[] })
        .any(|izinli| ayni_ad(izinli, ad));
    listede || ONEKLER.iter().any(|o| ad.starts_with(o))
}

/// Kaynak ortamdan komutun ortamını üretir: izin listesindekiler ve sabit
/// değerler. Sabit değerler kaynaktakileri ezer.
pub fn uret(kaynak: impl IntoIterator<Item = (String, String)>) -> Vec<(String, String)> {
    let mut ortam: Vec<(String, String)> = kaynak
        .into_iter()
        .filter(|(ad, _)| izinli_mi(ad) && !SABIT.iter().any(|(s, _)| ayni_ad(s, ad)))
        .collect();
    ortam.extend(
        SABIT
            .iter()
            .map(|(ad, deger)| (ad.to_string(), deger.to_string())),
    );
    ortam
}

/// Ortamdaki bir değişkenin değeri.
pub fn deger<'a>(ortam: &'a [(String, String)], ad: &str) -> Option<&'a str> {
    ortam
        .iter()
        .find(|(a, _)| ayni_ad(a, ad))
        .map(|(_, d)| d.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cift(ad: &str, deger: &str) -> (String, String) {
        (ad.to_string(), deger.to_string())
    }

    #[test]
    fn izin_listesi_disindakiler_ve_sirlar_gecmez() {
        let ortam = uret([
            cift("PATH", "/usr/bin"),
            cift("LC_ALL", "tr_TR.UTF-8"),
            cift("EVREN_API_KEY", "evren_llm_x"),
            cift("GITHUB_TOKEN", "ghp_x"),
            cift("SSH_AUTH_SOCK", "/tmp/ssh"),
            cift("AWS_SECRET_ACCESS_KEY", "x"),
            cift("DISPLAY", ":0"),
            cift("RASTGELE", "1"),
        ]);
        let adlar: Vec<&str> = ortam.iter().map(|(a, _)| a.as_str()).collect();
        assert!(adlar.contains(&"PATH"));
        assert!(adlar.contains(&"LC_ALL"));
        for yasak in [
            "EVREN_API_KEY",
            "GITHUB_TOKEN",
            "SSH_AUTH_SOCK",
            "AWS_SECRET_ACCESS_KEY",
            "DISPLAY",
            "RASTGELE",
        ] {
            assert!(!adlar.contains(&yasak), "{yasak} geçmemeliydi");
        }
    }

    #[test]
    fn sabit_degerler_kaynaktakini_ezer() {
        let ortam = uret([cift("TERM", "xterm-256color")]);
        assert_eq!(deger(&ortam, "TERM"), Some("dumb"));
        assert_eq!(ortam.iter().filter(|(a, _)| a == "TERM").count(), 1);
        assert_eq!(deger(&ortam, "GIT_TERMINAL_PROMPT"), Some("0"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_adlari_buyuk_kucuk_harf_ayirmaz() {
        let ortam = uret([
            cift("Path", "C:\\Windows"),
            cift("SYSTEMROOT", "C:\\Windows"),
        ]);
        assert_eq!(deger(&ortam, "PATH"), Some("C:\\Windows"));
        assert_eq!(deger(&ortam, "SystemRoot"), Some("C:\\Windows"));
    }
}
```

**`crates/kalfa-sandbox/src/program.rs`** (tam içerik):

```rust
//! Programın bulunması ve betiğin yorumlayıcısı (aşama 2 tasarımı §4.1).
//!
//! Çıplak ad komutun kendi ortamındaki `PATH`'te aranır, Kalfa'nınkinde değil.
//! Windows'ta `PATHEXT` uzantıları da denenir: `npm` gibi araçlar `npm.cmd`'dir.

use std::path::{Path, PathBuf};

use crate::ortam;
use crate::{BaslatmaHatasi, KomutBicimi};

/// Betiğin çalıştırılacağı yorumlayıcının gösterim adı.
pub fn yorumlayici_adi() -> &'static str {
    if cfg!(windows) { "PowerShell" } else { "sh" }
}

/// Windows'ta PowerShell'in çıktıyı UTF-8 yazması ve ilerleme kaydı basmaması
/// için betiğin önüne eklenen satır.
const POWERSHELL_ONEKI: &str = "$ProgressPreference='SilentlyContinue';\
[Console]::OutputEncoding=[System.Text.Encoding]::UTF8;\
$OutputEncoding=[System.Text.Encoding]::UTF8;\n";

/// Komutu çalıştırılacak programa ve argümanlarına çevirir.
pub fn coz(
    komut: &KomutBicimi,
    ortam: &[(String, String)],
) -> Result<(PathBuf, Vec<String>), BaslatmaHatasi> {
    match komut {
        KomutBicimi::Program {
            program,
            argumanlar,
        } => Ok((bul(program, ortam)?, argumanlar.clone())),
        KomutBicimi::Betik { metin } => {
            if cfg!(windows) {
                let kodlu = base64(&utf16le(&format!("{POWERSHELL_ONEKI}{metin}")));
                let args = [
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-EncodedCommand",
                    &kodlu,
                ];
                Ok((
                    bul("powershell", ortam)?,
                    args.iter().map(|a| a.to_string()).collect(),
                ))
            } else {
                Ok((PathBuf::from("/bin/sh"), vec!["-c".into(), metin.clone()]))
            }
        }
    }
}

/// Programı bulur. Yol ayırıcısı içeren ad olduğu gibi kullanılır.
pub fn bul(program: &str, ortam: &[(String, String)]) -> Result<PathBuf, BaslatmaHatasi> {
    let path = ortam::deger(ortam, "PATH").unwrap_or("");
    let yok = || BaslatmaHatasi::ProgramYok {
        program: program.to_string(),
        path: path.to_string(),
    };
    if program.contains('/') || program.contains('\\') {
        let yol = PathBuf::from(program);
        return if calistirilabilir(&yol) {
            Ok(yol)
        } else {
            Err(yok())
        };
    }
    for dizin in std::env::split_paths(path) {
        if dizin.as_os_str().is_empty() {
            continue;
        }
        for aday in adaylar(&dizin, program, ortam) {
            if calistirilabilir(&aday) {
                return Ok(aday);
            }
        }
    }
    Err(yok())
}

fn adaylar(dizin: &Path, program: &str, ortam: &[(String, String)]) -> Vec<PathBuf> {
    let mut adaylar = vec![dizin.join(program)];
    if cfg!(windows) {
        let pathext = ortam::deger(ortam, "PATHEXT").unwrap_or(".COM;.EXE;.BAT;.CMD");
        let uzantili = Path::new(program).extension().is_some();
        let ekler = pathext
            .split(';')
            .filter(|u| !u.is_empty())
            .map(|u| dizin.join(format!("{program}{}", u.to_ascii_lowercase())));
        if uzantili {
            adaylar.extend(ekler);
        } else {
            // Uzantısız dosya Windows'ta çalıştırılamaz.
            adaylar = ekler.collect();
        }
    }
    adaylar
}

fn calistirilabilir(yol: &Path) -> bool {
    let Ok(bilgi) = std::fs::metadata(yol) else {
        return false;
    };
    if !bilgi.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        bilgi.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn utf16le(metin: &str) -> Vec<u8> {
    metin.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

fn base64(veri: &[u8]) -> String {
    const ABECE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut cikti = String::with_capacity(veri.len().div_ceil(3) * 4);
    for parca in veri.chunks(3) {
        let b = [
            parca[0],
            parca.get(1).copied().unwrap_or(0),
            parca.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= parca.len() {
                cikti.push(char::from(ABECE[(n >> (18 - 6 * i)) as usize & 63]));
            } else {
                cikti.push('=');
            }
        }
    }
    cikti
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_rfc4648_orneklerini_uretir() {
        for (girdi, beklenen) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(girdi.as_bytes()), beklenen);
        }
    }

    #[test]
    fn path_disindaki_program_bulunamaz() {
        let bos = tempfile::tempdir().unwrap();
        let ortam = vec![("PATH".to_string(), bos.path().display().to_string())];
        let hata = bul("kesinlikle-olmayan-program", &ortam).unwrap_err();
        assert!(matches!(hata, BaslatmaHatasi::ProgramYok { .. }));
    }

    #[cfg(windows)]
    #[test]
    fn windows_pathext_ile_cmd_bulunur() {
        let dizin = tempfile::tempdir().unwrap();
        std::fs::write(dizin.path().join("arac.cmd"), "@echo off\r\n").unwrap();
        let ortam = vec![
            ("PATH".to_string(), dizin.path().display().to_string()),
            ("PATHEXT".to_string(), ".EXE;.CMD".to_string()),
        ];
        assert_eq!(bul("arac", &ortam).unwrap(), dizin.path().join("arac.cmd"));
    }

    #[cfg(unix)]
    #[test]
    fn unix_calistirilamayan_dosya_atlanir() {
        use std::os::unix::fs::PermissionsExt;
        let dizin = tempfile::tempdir().unwrap();
        let yol = dizin.path().join("arac");
        std::fs::write(&yol, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&yol, std::fs::Permissions::from_mode(0o644)).unwrap();
        let ortam = vec![("PATH".to_string(), dizin.path().display().to_string())];
        assert!(bul("arac", &ortam).is_err());
        std::fs::set_permissions(&yol, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(bul("arac", &ortam).unwrap(), yol);
    }
}
```

**`crates/kalfa-sandbox/src/yalitimsiz.rs`** (tam içerik):

```rust
//! Yalıtımsız arka uç (aşama 2 tasarımı §6.5): süreç sınırlarını uygular,
//! başka kısıt koymaz. Aşama 2a'da iki platformda da tek arka uç budur.

use crate::{
    BaslatmaHatasi, CalistirmaIstegi, Surec, Yalitici, YalitimDurumu, YalitimYoklukNedeni,
};

/// 2a'da yalıtım yoktur; durum bunu ve kullanıcının ne beklemesi gerektiğini
/// söyler.
pub const TALIMAT: &str = "Bu sürümde komutlar yalıtımsız çalışıyor; yalıtım sonraki sürümde \
gelecek. Her komut, çalışmadan önce yerel pencerede onayınıza sunulur.";

#[derive(Debug, Default)]
pub struct Yalitimsiz;

impl Yalitici for Yalitimsiz {
    fn durum(&self) -> YalitimDurumu {
        YalitimDurumu::Yok {
            neden: YalitimYoklukNedeni::Desteklenmiyor,
            talimat: TALIMAT.to_string(),
        }
    }

    fn baslat(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi> {
        Surec::baslat(istek)
    }
}
```

- [ ] **Step 4: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa-sandbox --locked`
Expected: birim testleri ve `tests/yalitimsiz.rs`'in 7 testi geçer (Windows'ta PowerShell betikleri birkaç saniye sürer). Görevin sonundaki genel denetimleri de koştur.

- [ ] **Step 5: Commit**

```bash
git add crates/kalfa-sandbox
git commit -m "sandbox: yalıtımsız komut çalıştırma ve süreç sınırları"
```

---

### Task 2: Çekirdek: kabuk tarifi, yerel onay ve ara çıktı

**Files:**
- Create: `crates/kalfa-core/src/kabuk.rs`, `crates/kalfa-core/tests/oturum_kabuk.rs`
- Modify: `crates/kalfa-core/src/{lib,protokol,arac,izin,muhur,aktarim}.rs`, `crates/kalfa-core/src/oturum/{mod,arac_asamasi}.rs`
- Test: `crates/kalfa-core/tests/{protokol,izin,arac_muhur,aktarim}.rs` (değişir)

**Interfaces:**
- Consumes: 1b–1c çekirdeği (`Arac`, `Hazirlik`, `Karar`, `muhur_hesapla`, `Oturum`, `OlayKanali`, `Paketleyici`).
- Produces:
  - `kabuk::{KabukTarifi { komut, bicim, dizin, ag, yalitim, ilk_cagri }, KabukBicimi::{Program, Betik { yorumlayici }}, YalitimOzeti::{Yalitimli { arka_uc, profil }, Yalitimsiz { talimat }}}`, `YalitimOzeti::yalitimli_mi()`.
  - `Hazirlik.kabuk: Option<KabukTarifi>`.
  - `arac::{CiktiParcasi, CiktiGonderici}`: `CiktiGonderici::bos()`, `CiktiGonderici::yeni(CagriId, mpsc::Sender<CiktiParcasi>)`, `gonder(CiktiAkisi, String)`.
  - `Arac::calistir_akisli(&self, &Value, &Hazirlik, CancellationToken, CiktiGonderici)` (varsayılanı `calistir`).
  - `AracHatasi::Basarisiz(AracCiktisi)`.
  - `izin::{kabuk_karari, ILK_KABUK_UYARISI, YALITIMSIZ_UYARISI, AGLI_KABUK_UYARISI}`, `Karar::YerelOnayli { uyarilar }`.
  - `protokol::{CiktiAkisi::{Standart, Hata}, Olay::AracCiktisiParcasi { cagri, akis, metin, atlanan_bayt }, Komut::YerelOnay { cagri, muhur }}`; `OnayOgesi.kabuk: Option<KabukTarifi>`, `OnayOgesi.yerel_onay: bool` (ikisi de `#[serde(default)]`).
  - `OturumTutamaci::bekleyen_onay(CagriId) -> Option<OnayOgesi>`.

**Davranış notları (testler sınar):**
- **Kabuk kararı (tasarım §5.1):** yalıtımsız çağrı her kipte `YerelOnayli`; yalıtımlı ağsız çağrı Serbest'te serbest, diğer kiplerde onaylı; ağlı çağrı ve çalışma alanındaki ilk çağrı her kipte onaylı; İnceleme'de yasak. Birleştirme: Yasak > YerelOnayli > Onayli > Serbest.
- **Mühür:** kabuk tarifinin bütün alanlarını (komut, dizin, ağ, yalıtım, profil, ilk çağrı) içerir.
- **Yerel onay:** `AracOnayi { Onayla }` yerel öğeyi onaylamaz, "yalnızca yerel pencerede onaylanabilir" hatası verir ve öğe beklemeye devam eder; ret serbesttir. `YerelOnay` yalnızca yerel öğeyi onaylar. Bekleyen öğeler `OnayGerekiyor`'dan önce tutamaca konur, onay aşaması bitince temizlenir.
- **Ara çıktı:** araç parçaları sonucu döndürmeden önce gönderir; oturum onları sonuçtan önce iletir. Kuyruk olay ya da bayt bütçesinin yarısından fazla doluysa parça atılır; araç ve oturum tarafında atılanların toplamı çağrı bitince boş bir parçayla bildirilir. Paketleyici aynı çağrının aynı akışındaki bitişik parçaları birleştirir; atlanan bildiren parça birleşmeyi keser.
- **Başarısızlık sayacı:** `AracHatasi::Basarisiz` sonucu `Basarisiz` olarak ve `[kaynak: …]` etiketiyle modele gider; aynı çağrının üst üste başarısızlık sınırına sayılmaz.

- [ ] **Step 1: Başarısız testleri yaz**

**`crates/kalfa-core/tests/oturum_kabuk.rs`** (tam içerik):

```rust
//! Kabuk çağrısı oturumda (aşama 2 tasarımı §4.4, §4.5, §5): yerel onay,
//! ara çıktı, çıktı seli ve başarısızlık sayacı. Kabuk burada sahtedir;
//! gerçek komutlar `kalfa-tools` ve `kalfa-sandbox` testlerindedir.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, CiktiGonderici, Hazirlik};
use kalfa_core::izin::Kip;
use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
use kalfa_core::oturum::{OlayButcesi, OturumAyarlari};
use kalfa_core::protokol::{
    AracDurumu, BitisNedeni, CiktiAkisi, Komut, Olay, OnayKarari, OnayOgesi,
};
use kalfa_core::test_destek::*;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

struct SahteKabuk {
    yalitimli: bool,
    parca_sayisi: usize,
    parca_bayt: usize,
    basarisiz: bool,
    calisma: AtomicUsize,
    bitti: AtomicBool,
}

impl SahteKabuk {
    fn yeni(yalitimli: bool) -> Self {
        Self {
            yalitimli,
            parca_sayisi: 3,
            parca_bayt: 4,
            basarisiz: false,
            calisma: AtomicUsize::new(0),
            bitti: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl Arac for SahteKabuk {
    fn ad(&self) -> &str {
        "kabuk"
    }
    fn aciklama(&self) -> &str {
        "sahte kabuk"
    }
    fn parametre_semasi(&self) -> Value {
        json!({"type": "object"})
    }
    fn yazar_mi(&self) -> bool {
        true
    }
    async fn hazirla(&self, _a: &Value) -> Result<Hazirlik, AracHatasi> {
        let yalitim = if self.yalitimli {
            YalitimOzeti::Yalitimli {
                arka_uc: "sahte".into(),
                profil: "p".into(),
            }
        } else {
            YalitimOzeti::Yalitimsiz {
                talimat: "yalıtım yok".into(),
            }
        };
        Ok(Hazirlik {
            tum_calisma_alani: true,
            aciklama: "`test` komutunu çalıştırmak istiyor.".into(),
            kabuk: Some(KabukTarifi {
                komut: "test".into(),
                bicim: KabukBicimi::Program,
                dizin: ".".into(),
                ag: false,
                yalitim,
                ilk_cagri: false,
            }),
            ..Default::default()
        })
    }
    async fn calistir(
        &self,
        a: &Value,
        h: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.calistir_akisli(a, h, iptal, CiktiGonderici::bos())
            .await
    }
    async fn calistir_akisli(
        &self,
        _a: &Value,
        _h: &Hazirlik,
        _iptal: CancellationToken,
        cikti: CiktiGonderici,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.calisma.fetch_add(1, Ordering::SeqCst);
        for i in 0..self.parca_sayisi {
            let akis = if i % 2 == 0 {
                CiktiAkisi::Standart
            } else {
                CiktiAkisi::Hata
            };
            cikti.gonder(akis, "x".repeat(self.parca_bayt));
            tokio::task::yield_now().await;
        }
        self.bitti.store(true, Ordering::SeqCst);
        let c = AracCiktisi {
            icerik: "Çıkış kodu: 1".into(),
            kaynak: "kabuk:test".into(),
        };
        if self.basarisiz {
            Err(AracHatasi::Basarisiz(c))
        } else {
            Ok(c)
        }
    }
}

fn surucu(kabuk: &Arc<SahteKabuk>, yanitlar: Vec<SahteYanit>, ayar: OturumAyarlari) -> Surucu {
    let s = SahteSaglayici::yeni(yanitlar);
    test_oturumu(
        test_baglami(
            s,
            vec![kabuk.clone() as Arc<dyn Arac>],
            sanal_siniflandirici(),
        ),
        ayar,
    )
}

fn ayarlar(kip: Kip) -> OturumAyarlari {
    let mut a = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    a.kip = kip;
    a
}

fn mesaj() -> Komut {
    Komut::KullaniciMesaji {
        metin: "çalıştır".into(),
    }
}

async fn onay_bekle(d: &mut Surucu) -> Vec<OnayOgesi> {
    match d
        .bekle(|o| matches!(o, Olay::OnayGerekiyor { .. }))
        .await
        .yuk
    {
        Olay::OnayGerekiyor { ogeler, .. } => ogeler,
        _ => unreachable!(),
    }
}

fn parcalar(d: &Surucu) -> Vec<(CiktiAkisi, String, u64)> {
    d.gecmis
        .iter()
        .filter_map(|z| match &z.yuk {
            Olay::AracCiktisiParcasi {
                akis,
                metin,
                atlanan_bayt,
                ..
            } => Some((*akis, metin.clone(), *atlanan_bayt)),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn yalitimsiz_kabuk_yalnizca_yerel_onayla_calisir() {
    let kabuk = Arc::new(SahteKabuk::yeni(false));
    let mut d = surucu(
        &kabuk,
        vec![
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::metin("bitti"),
        ],
        ayarlar(Kip::Serbest),
    );
    d.gonder(mesaj()).await;
    let ogeler = onay_bekle(&mut d).await;
    let oge = &ogeler[0];
    assert!(oge.yerel_onay, "Serbest kipte de yerel onay ister");
    assert_eq!(oge.kabuk.as_ref().map(|k| k.komut.as_str()), Some("test"));
    assert_eq!(
        d.tutamac.bekleyen_onay(oge.cagri).as_ref(),
        Some(oge),
        "yerel iletişim kutusu komutu çekirdekten okur"
    );

    // Webview'ın gönderebildiği onay yerel öğeyi onaylamaz.
    d.gonder(Komut::AracOnayi {
        cagri: oge.cagri,
        karar: OnayKarari::Onayla,
        muhur: oge.muhur.clone(),
    })
    .await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("yerel pencerede")))
        .await;
    assert_eq!(kabuk.calisma.load(Ordering::SeqCst), 0);

    d.gonder(Komut::YerelOnay {
        cagri: oge.cagri,
        muhur: oge.muhur.clone(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(kabuk.calisma.load(Ordering::SeqCst), 1);
    assert_eq!(
        d.tutamac.bekleyen_onay(oge.cagri),
        None,
        "onay bitince temizlenir"
    );
}

#[tokio::test]
async fn yerel_oge_reddedilebilir_ve_yerel_olmayan_yerel_onayla_onaylanmaz() {
    let kabuk = Arc::new(SahteKabuk::yeni(false));
    let mut d = surucu(
        &kabuk,
        vec![
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::metin("peki"),
        ],
        ayarlar(Kip::Siki),
    );
    d.gonder(mesaj()).await;
    let oge = onay_bekle(&mut d).await.remove(0);
    d.gonder(Komut::AracOnayi {
        cagri: oge.cagri,
        karar: OnayKarari::Reddet { gerekce: None },
        muhur: oge.muhur.clone(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(kabuk.calisma.load(Ordering::SeqCst), 0);

    let yalitimli = Arc::new(SahteKabuk::yeni(true));
    let mut d = surucu(
        &yalitimli,
        vec![
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::metin("peki"),
        ],
        ayarlar(Kip::Siki),
    );
    d.gonder(mesaj()).await;
    let oge = onay_bekle(&mut d).await.remove(0);
    assert!(!oge.yerel_onay);
    d.gonder(Komut::YerelOnay {
        cagri: oge.cagri,
        muhur: oge.muhur.clone(),
    })
    .await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("yerel onay gerektirmiyor")))
        .await;
    d.gonder(Komut::AracOnayi {
        cagri: oge.cagri,
        karar: OnayKarari::Onayla,
        muhur: oge.muhur.clone(),
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(yalitimli.calisma.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn yalitimli_agsiz_kabuk_serbest_kipte_onaysiz_calisir() {
    let kabuk = Arc::new(SahteKabuk::yeni(true));
    let mut d = surucu(
        &kabuk,
        vec![
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::metin("bitti"),
        ],
        ayarlar(Kip::Serbest),
    );
    d.gonder(mesaj()).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(kabuk.calisma.load(Ordering::SeqCst), 1);
    assert!(
        !d.gecmis
            .iter()
            .any(|z| matches!(z.yuk, Olay::OnayGerekiyor { .. }))
    );
}

#[tokio::test]
async fn ara_cikti_sonuctan_once_ve_akisiyla_gelir() {
    let kabuk = Arc::new(SahteKabuk::yeni(true));
    let mut d = surucu(
        &kabuk,
        vec![
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::metin("bitti"),
        ],
        ayarlar(Kip::Serbest),
    );
    d.gonder(mesaj()).await;
    d.bekle(|o| matches!(o, Olay::AracSonucu { .. })).await;
    assert_eq!(
        parcalar(&d),
        vec![
            (CiktiAkisi::Standart, "xxxx".into(), 0),
            (CiktiAkisi::Hata, "xxxx".into(), 0),
            (CiktiAkisi::Standart, "xxxx".into(), 0),
        ]
    );
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
}

/// Arayüz okumazken bile çıktı seli olay kuyruğunu doldurmaz ve turu iptal
/// ettirmez; atılan her bayt sayılır.
#[tokio::test]
async fn cikti_seli_turu_iptal_ettirmez_atilan_bayt_sayilir() {
    let kabuk = Arc::new(SahteKabuk {
        parca_sayisi: 2000,
        parca_bayt: 1024,
        ..SahteKabuk::yeni(true)
    });
    let mut ayar = ayarlar(Kip::Serbest);
    ayar.olay_butcesi = OlayButcesi::yeni(64, 64 * 1024).unwrap();
    let mut d = surucu(
        &kabuk,
        vec![
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::metin("bitti"),
        ],
        ayar,
    );
    d.gonder(mesaj()).await;
    // Arayüz takılmış gibi: araç bitene kadar olay okunmaz.
    tokio::time::timeout(Duration::from_secs(10), async {
        while !kabuk.bitti.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("araç bitmeli");
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert!(
        !d.gecmis
            .iter()
            .any(|z| matches!(z.yuk, Olay::AktarimDurdu { .. })),
        "kuyruk dolmamalı"
    );
    let p = parcalar(&d);
    let gelen: u64 = p.iter().map(|(_, m, _)| m.len() as u64).sum();
    let atlanan: u64 = p.iter().map(|(_, _, a)| a).sum();
    assert!(atlanan > 0, "ekran yetişemediği için parça atılmalı");
    assert_eq!(gelen + atlanan, 2000 * 1024, "atılan her bayt bildirilir");
}

/// Kabukta sıfırdan farklı çıkış kodu aynı çağrının başarısızlık sınırına
/// sayılmaz (aşama 2 tasarımı §4.5).
#[tokio::test]
async fn basarisiz_cikti_ayni_hata_sinirina_sayilmaz() {
    let kabuk = Arc::new(SahteKabuk {
        basarisiz: true,
        ..SahteKabuk::yeni(true)
    });
    let mut d = surucu(
        &kabuk,
        vec![
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::arac("kabuk", json!({})),
            SahteYanit::metin("düzeldi"),
        ],
        ayarlar(Kip::Serbest),
    );
    d.gonder(mesaj()).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    let sonuclar: Vec<(AracDurumu, String)> = d
        .gecmis
        .iter()
        .filter_map(|z| match &z.yuk {
            Olay::AracSonucu { durum, icerik, .. } => Some((*durum, icerik.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(sonuclar.len(), 4);
    for (durum, icerik) in sonuclar {
        assert_eq!(durum, AracDurumu::Basarisiz);
        assert_eq!(icerik, "[kaynak: kabuk:test]\nÇıkış kodu: 1");
    }
}
```

**`crates/kalfa-core/tests/protokol.rs`**: şu bloğu

```rust
use kalfa_core::izin::Kip;
use kalfa_core::kimlik::{CagriId, IslemId, OturumId, TurId};
use kalfa_core::muhur::OnayMuhru;
```

şununla değiştir:

```rust
use kalfa_core::izin::Kip;
use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
use kalfa_core::kimlik::{CagriId, IslemId, OturumId, TurId};
use kalfa_core::muhur::OnayMuhru;
```

**`crates/kalfa-core/tests/protokol.rs`**: şu bloğu

```rust
        },
        Komut::IzinKipiDegistir { kip: Kip::Proje },
    ] {
        gidip_gelir(&k);
```

şununla değiştir:

```rust
        },
        Komut::IzinKipiDegistir { kip: Kip::Proje },
        Komut::YerelOnay {
            cagri: CagriId::yeni(),
            muhur: OnayMuhru("m".into()),
        },
    ] {
        gidip_gelir(&k);
```

**`crates/kalfa-core/tests/protokol.rs`**: şu bloğu

```rust
                uyarilar: vec!["dikkat".into()],
                muhur: OnayMuhru("m".into()),
            }],
        },
        Olay::IstekBilgisi {
```

şununla değiştir:

```rust
                uyarilar: vec!["dikkat".into()],
                muhur: OnayMuhru("m".into()),
                kabuk: None,
                yerel_onay: false,
            }],
        },
        Olay::OnayGerekiyor {
            islem: IslemId::yeni(),
            ogeler: vec![kabuk_ogesi(c)],
        },
        Olay::AracCiktisiParcasi {
            cagri: c,
            akis: CiktiAkisi::Hata,
            metin: "uyarı: ş\n".into(),
            atlanan_bayt: 12,
        },
        Olay::IstekBilgisi {
```

**`crates/kalfa-core/tests/protokol.rs`**: şu bloğu

```rust
}

#[test]
fn zarf_bicimi() {
```

şununla değiştir:

```rust
}

fn kabuk_ogesi(cagri: CagriId) -> OnayOgesi {
    OnayOgesi {
        cagri,
        arac: "kabuk".into(),
        aciklama: "`cargo test` komutunu çalıştırmak istiyor.".into(),
        hedefler: vec![],
        diff: None,
        uyarilar: vec!["Yalıtımsız".into()],
        muhur: OnayMuhru("m".into()),
        kabuk: Some(KabukTarifi {
            komut: "cargo test".into(),
            bicim: KabukBicimi::Program,
            dizin: ".".into(),
            ag: false,
            yalitim: YalitimOzeti::Yalitimsiz {
                talimat: "yok".into(),
            },
            ilk_cagri: true,
        }),
        yerel_onay: true,
    }
}

/// Arayüzün `tipler.ts`'teki tanımları bu biçime dayanır.
#[test]
fn kabuk_json_bicimi() {
    let c = CagriId::yeni();
    let j = serde_json::to_value(kabuk_ogesi(c)).unwrap();
    assert_eq!(j["yerel_onay"], json!(true));
    assert_eq!(
        j["kabuk"],
        json!({
            "komut": "cargo test",
            "bicim": {"tip": "Program"},
            "dizin": ".",
            "ag": false,
            "yalitim": {"tip": "Yalitimsiz", "talimat": "yok"},
            "ilk_cagri": true
        })
    );
    let parca = serde_json::to_value(Olay::AracCiktisiParcasi {
        cagri: c,
        akis: CiktiAkisi::Standart,
        metin: "a".into(),
        atlanan_bayt: 0,
    })
    .unwrap();
    assert_eq!(
        parca,
        json!({"tip": "AracCiktisiParcasi", "cagri": c.to_string(), "akis": "Standart", "metin": "a", "atlanan_bayt": 0})
    );
    // Kabuk alanları olmayan eski biçim de okunur.
    let mut eski = serde_json::to_value(kabuk_ogesi(c)).unwrap();
    eski.as_object_mut().unwrap().remove("kabuk");
    eski.as_object_mut().unwrap().remove("yerel_onay");
    let oge: OnayOgesi = serde_json::from_value(eski).unwrap();
    assert_eq!(oge.kabuk, None);
    assert!(!oge.yerel_onay);
}

#[test]
fn zarf_bicimi() {
```

**`crates/kalfa-core/tests/izin.rs`**: şu bloğu

```rust
    assert_eq!(s.sinifla(&y("src/husky.rs")), YolSinifi::Normal);
}
```

şununla değiştir:

```rust
    assert_eq!(s.sinifla(&y("src/husky.rs")), YolSinifi::Normal);
}

fn kabuk(yalitimli: bool, ag: bool, ilk_cagri: bool) -> kalfa_core::kabuk::KabukTarifi {
    use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
    KabukTarifi {
        komut: "cargo test".into(),
        bicim: KabukBicimi::Program,
        dizin: ".".into(),
        ag,
        yalitim: if yalitimli {
            YalitimOzeti::Yalitimli {
                arka_uc: "bwrap".into(),
                profil: "p".into(),
            }
        } else {
            YalitimOzeti::Yalitimsiz {
                talimat: "yalıtım yok".into(),
            }
        },
        ilk_cagri,
    }
}

/// Aşama 2 tasarımı §5.1'in tablosu.
#[test]
fn kabuk_kararlari() {
    for kip in [Kip::Siki, Kip::Proje, Kip::Serbest] {
        for ag in [false, true] {
            assert_eq!(
                kabuk_karari(kip, &kabuk(false, ag, false)),
                Karar::YerelOnayli {
                    uyarilar: vec![YALITIMSIZ_UYARISI.into(), "yalıtım yok".into()]
                },
                "yalıtımsız çağrı her kipte yerel onay ister ({kip:?}, ağ: {ag})"
            );
        }
        let beklenen = if kip == Kip::Serbest {
            Karar::Serbest
        } else {
            onayli(&[])
        };
        assert_eq!(
            kabuk_karari(kip, &kabuk(true, false, false)),
            beklenen,
            "{kip:?}"
        );
        assert_eq!(
            kabuk_karari(kip, &kabuk(true, true, false)),
            onayli(&[AGLI_KABUK_UYARISI]),
            "ağlı çağrı her kipte onay ister ({kip:?})"
        );
        assert_eq!(
            kabuk_karari(kip, &kabuk(true, false, true)),
            onayli(&[ILK_KABUK_UYARISI]),
            "çalışma alanındaki ilk çağrı Serbest kipte de onay ister ({kip:?})"
        );
    }
    assert!(matches!(
        kabuk_karari(Kip::Inceleme, &kabuk(true, false, false)),
        Karar::Yasak { .. }
    ));
    assert_eq!(
        kabuk_karari(Kip::Serbest, &kabuk(false, false, true)),
        Karar::YerelOnayli {
            uyarilar: vec![
                ILK_KABUK_UYARISI.into(),
                YALITIMSIZ_UYARISI.into(),
                "yalıtım yok".into()
            ]
        }
    );
}

#[test]
fn yerel_onay_birlesmede_onayliyi_ezer() {
    let yerel = |u: &[&str]| Karar::YerelOnayli {
        uyarilar: u.iter().map(|s| s.to_string()).collect(),
    };
    assert_eq!(onayli(&["a"]).birlestir(yerel(&["b"])), yerel(&["a", "b"]));
    assert_eq!(yerel(&["b"]).birlestir(onayli(&["a"])), yerel(&["b", "a"]));
    assert_eq!(yerel(&["a"]).birlestir(Karar::Serbest), yerel(&["a"]));
    assert_eq!(Karar::Serbest.birlestir(yerel(&["a"])), yerel(&["a"]));
    let yasak = Karar::Yasak { neden: "x".into() };
    assert_eq!(yerel(&["a"]).birlestir(yasak.clone()), yasak);
}
```

**`crates/kalfa-core/tests/arac_muhur.rs`**: şu bloğu

```rust
}

#[test]
fn ozet_hex_sha256() {
```

şununla değiştir:

```rust
}

/// Kabukta mühür komuta, dizine, ağa, yalıtım durumuna ve profil özetine
/// bağlıdır (aşama 2 tasarımı §3.3).
#[test]
fn kabuk_muhru_tarifin_her_alanina_baglidir() {
    use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
    let c = CagriId::yeni();
    let a = json!({"program": "cargo", "argumanlar": ["test"]});
    let tarif = KabukTarifi {
        komut: "cargo test".into(),
        bicim: KabukBicimi::Program,
        dizin: ".".into(),
        ag: false,
        yalitim: YalitimOzeti::Yalitimli {
            arka_uc: "bwrap".into(),
            profil: "p1".into(),
        },
        ilk_cagri: false,
    };
    let h = |t: KabukTarifi| Hazirlik {
        tum_calisma_alani: true,
        kabuk: Some(t),
        ..Default::default()
    };
    let temel = muhur_hesapla(c, "kabuk", &a, &h(tarif.clone()));
    assert_eq!(temel, muhur_hesapla(c, "kabuk", &a, &h(tarif.clone())));
    let degisikler = [
        KabukTarifi {
            dizin: "src".into(),
            ..tarif.clone()
        },
        KabukTarifi {
            ag: true,
            ..tarif.clone()
        },
        KabukTarifi {
            yalitim: YalitimOzeti::Yalitimli {
                arka_uc: "bwrap".into(),
                profil: "p2".into(),
            },
            ..tarif.clone()
        },
        KabukTarifi {
            yalitim: YalitimOzeti::Yalitimsiz {
                talimat: "yok".into(),
            },
            ..tarif.clone()
        },
        KabukTarifi {
            ilk_cagri: true,
            ..tarif.clone()
        },
    ];
    for d in degisikler {
        assert_ne!(temel, muhur_hesapla(c, "kabuk", &a, &h(d.clone())), "{d:?}");
    }
}

#[test]
fn ozet_hex_sha256() {
```

**`crates/kalfa-core/tests/aktarim.rs`**: şu bloğu

```rust
    AktarimAyarlari, IsledimHatasi, KanalMesaji, KrediTutamaci, OlayKaynagi, Paketleyici,
};
use kalfa_core::kimlik::{OturumId, TurId};
use kalfa_core::protokol::{BitisNedeni, Olay, SEMA_SURUMU, Zarf};
use tokio::sync::mpsc;
use tokio::time::Instant;
```

şununla değiştir:

```rust
    AktarimAyarlari, IsledimHatasi, KanalMesaji, KrediTutamaci, OlayKaynagi, Paketleyici,
};
use kalfa_core::kimlik::{CagriId, OturumId, TurId};
use kalfa_core::protokol::{BitisNedeni, CiktiAkisi, Olay, SEMA_SURUMU, Zarf};
use tokio::sync::mpsc;
use tokio::time::Instant;
```

**`crates/kalfa-core/tests/aktarim.rs`**: şu bloğu

```rust
        zarflar[0].sira, 3,
        "birleşen parça son parçanın sırasını taşır"
    );
}
```

şununla değiştir:

```rust
        zarflar[0].sira, 3,
        "birleşen parça son parçanın sırasını taşır"
    );
}

fn cikti(cagri: CagriId, akis: CiktiAkisi, metin: &str, atlanan_bayt: u64) -> Olay {
    Olay::AracCiktisiParcasi {
        cagri,
        akis,
        metin: metin.into(),
        atlanan_bayt,
    }
}

/// Araç çıktısı yalnızca aynı çağrının aynı akışındaki bitişik parçayla
/// birleşir; atlanan bayt bildiren parça birleşmeyi keser (aşama 2 tasarımı
/// §3.3).
#[tokio::test(start_paused = true)]
async fn arac_ciktisi_ayni_cagri_ve_akista_birlesir() {
    let (mut u, mut p, _kredi, _) = kur(AktarimAyarlari::default());
    let a = CagriId::yeni();
    let b = CagriId::yeni();
    u.gonder(cikti(a, CiktiAkisi::Standart, "bir ", 5));
    u.gonder(cikti(a, CiktiAkisi::Standart, "iki", 0));
    u.gonder(cikti(a, CiktiAkisi::Hata, "uyarı", 0));
    u.gonder(cikti(b, CiktiAkisi::Hata, "başka", 0));
    u.gonder(cikti(b, CiktiAkisi::Hata, "sonra", 7));
    let m = p.sonraki().await.unwrap();
    assert_eq!(
        olaylar(&m),
        vec![
            cikti(a, CiktiAkisi::Standart, "bir iki", 5),
            cikti(a, CiktiAkisi::Hata, "uyarı", 0),
            cikti(b, CiktiAkisi::Hata, "başka", 0),
            cikti(b, CiktiAkisi::Hata, "sonra", 7),
        ]
    );
}
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa-core --locked`
Expected: derleme hatası (`kalfa_core::kabuk`, `CiktiGonderici`, `Komut::YerelOnay` yok).

- [ ] **Step 3: Tipleri, kararı ve mührü yaz**

**`crates/kalfa-core/src/kabuk.rs`** (tam içerik):

```rust
//! Kabuk çağrısının çekirdekteki tarifi (aşama 2 tasarımı §3.3). İzin kararı,
//! onay kartı ve mühür bundan üretilir; komutu çalıştıran kod
//! `kalfa-sandbox`'tadır.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KabukTarifi {
    /// Onay kartında gösterilen tam komut: program ve argümanları ya da
    /// betiğin bütün metni.
    pub komut: String,
    pub bicim: KabukBicimi,
    /// Çalışma alanına göre çalışma dizini; kök için `.`.
    pub dizin: String,
    /// Model ağ erişimi istedi. Yalıtımsız çalıştırmada ağ kapatılamaz; bu
    /// yalnızca modelin niyetidir.
    pub ag: bool,
    pub yalitim: YalitimOzeti,
    /// Bu çalışma alanındaki ilk kabuk çağrısı (aşama 2 tasarımı §4.2).
    pub ilk_cagri: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum KabukBicimi {
    Program,
    Betik { yorumlayici: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum YalitimOzeti {
    /// `profil`: yalıtım profilinin özeti; profil değişirse bekleyen onaylar
    /// geçersizleşir.
    Yalitimli { arka_uc: String, profil: String },
    /// `talimat`: yalıtımın neden olmadığı ve nasıl açılacağı.
    Yalitimsiz { talimat: String },
}

impl YalitimOzeti {
    pub fn yalitimli_mi(&self) -> bool {
        matches!(self, Self::Yalitimli { .. })
    }
}
```

**`crates/kalfa-core/src/lib.rs`**: şu bloğu

```rust
pub mod git_koruma;
pub mod izin;
pub mod kayit;
pub mod kilit;
```

şununla değiştir:

```rust
pub mod git_koruma;
pub mod izin;
pub mod kabuk;
pub mod kayit;
pub mod kilit;
```

**`crates/kalfa-core/src/protokol.rs`**: şu bloğu

```rust

use crate::izin::Kip;
use crate::kimlik::{CagriId, IslemId, OturumId, TurId};
use crate::muhur::OnayMuhru;
```

şununla değiştir:

```rust

use crate::izin::Kip;
use crate::kabuk::KabukTarifi;
use crate::kimlik::{CagriId, IslemId, OturumId, TurId};
use crate::muhur::OnayMuhru;
```

**`crates/kalfa-core/src/protokol.rs`**: şu bloğu

```rust
        cagri: CagriId,
        karar: OnayKarari,
        muhur: OnayMuhru,
    },
```

şununla değiştir:

```rust
        cagri: CagriId,
        karar: OnayKarari,
        muhur: OnayMuhru,
    },
    /// Yerel iletişim kutusunda onaylanan işlem (aşama 2 tasarımı §5.2).
    /// Yalnızca `kalfa-uygulama` gönderir; webview gönderemez. Yerel onay
    /// isteyen bir öğe `AracOnayi` ile onaylanamaz, yalnızca reddedilebilir.
    YerelOnay {
        cagri: CagriId,
        muhur: OnayMuhru,
    },
```

**`crates/kalfa-core/src/protokol.rs`**: şu bloğu

```rust
    pub uyarilar: Vec<String>,
    pub muhur: OnayMuhru,
}
```

şununla değiştir:

```rust
    pub uyarilar: Vec<String>,
    pub muhur: OnayMuhru,
    /// Kabuk çağrısında diff yerine gösterilenler (aşama 2 tasarımı §10.2).
    #[serde(default)]
    pub kabuk: Option<KabukTarifi>,
    /// Onay yalnızca yerel iletişim kutusundan verilebilir (§5.2).
    #[serde(default)]
    pub yerel_onay: bool,
}

/// Araç çıktısının hangi akıştan geldiği.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CiktiAkisi {
    Standart,
    Hata,
}
```

**`crates/kalfa-core/src/protokol.rs`**: şu bloğu

```rust
        icerik: String,
    },
    OnayGerekiyor {
        islem: IslemId,
```

şununla değiştir:

```rust
        icerik: String,
    },
    /// Süren bir aracın ara çıktısı (aşama 2 tasarımı §4.4). Kayda yazılmaz.
    /// Ekran yetişemezse parçalar atılır; `atlanan_bayt` bu parçadan önce
    /// atılanların toplamıdır.
    AracCiktisiParcasi {
        cagri: CagriId,
        akis: CiktiAkisi,
        metin: String,
        atlanan_bayt: u64,
    },
    OnayGerekiyor {
        islem: IslemId,
```

**`crates/kalfa-core/src/arac.rs`**: şu bloğu

```rust
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use kalfa_evren::tipler::ToolDef;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::izin::Kip;
use crate::yol::{CalismaYolu, YolHatasi};
```

şununla değiştir:

```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use kalfa_evren::tipler::ToolDef;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::izin::Kip;
use crate::kabuk::KabukTarifi;
use crate::kimlik::CagriId;
use crate::protokol::CiktiAkisi;
use crate::yol::{CalismaYolu, YolHatasi};
```

**`crates/kalfa-core/src/arac.rs`**: şu bloğu

```rust
    pub aciklama: String,
    pub diff: Option<String>,
}
```

şununla değiştir:

```rust
    pub aciklama: String,
    pub diff: Option<String>,
    /// Kabuk çağrısının tarifi (aşama 2 tasarımı §3.3).
    pub kabuk: Option<KabukTarifi>,
}
```

**`crates/kalfa-core/src/arac.rs`**: şu bloğu

```rust
    #[error("{0}")]
    GirisCikis(String),
}
```

şununla değiştir:

```rust
    #[error("{0}")]
    GirisCikis(String),
    /// Araç çalıştı ama başarısız oldu; çıktı yine de modele gider. Kabukta
    /// sıfırdan farklı çıkış kodu ve zaman aşımı böyledir. Aynı çağrının
    /// üst üste başarısızlık sınırına sayılmaz (aşama 2 tasarımı §4.5):
    /// düzeltmeden sonra aynı testi yeniden koşmak olağandır.
    #[error("{}", .0.icerik)]
    Basarisiz(AracCiktisi),
}

/// Süren bir aracın ara çıktı parçası.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiktiParcasi {
    pub cagri: CagriId,
    pub akis: CiktiAkisi,
    pub metin: String,
    /// Bu parçadan önce atılan baytların toplamı.
    pub atlanan_bayt: u64,
}

/// Aracın ara çıktısını oturuma taşır (aşama 2 tasarımı §4.4). Göndermek
/// hiç beklemez: kanal doluysa parça atılır ve atılan bayt sayısı sonraki
/// parçaya yazılır. Aracın modele verdiği sonuç bundan etkilenmez.
#[derive(Debug, Clone)]
pub struct CiktiGonderici {
    ic: Option<(CagriId, mpsc::Sender<CiktiParcasi>)>,
    atlanan: Arc<AtomicU64>,
}

impl CiktiGonderici {
    /// Hiçbir yere göndermeyen gönderici.
    pub fn bos() -> Self {
        Self {
            ic: None,
            atlanan: Arc::default(),
        }
    }

    /// `cagri`'nın parçalarını `tx`'e gönderen gönderici.
    pub fn yeni(cagri: CagriId, tx: mpsc::Sender<CiktiParcasi>) -> Self {
        Self {
            ic: Some((cagri, tx)),
            atlanan: Arc::default(),
        }
    }

    /// Oturumun, araç bittikten sonra atılan baytları okuması için.
    pub(crate) fn atlanan_sayaci(&self) -> Arc<AtomicU64> {
        self.atlanan.clone()
    }

    pub fn gonder(&self, akis: CiktiAkisi, metin: String) {
        let Some((cagri, tx)) = &self.ic else {
            return;
        };
        let atlanan = self.atlanan.swap(0, Ordering::SeqCst);
        let bayt = metin.len() as u64;
        let parca = CiktiParcasi {
            cagri: *cagri,
            akis,
            metin,
            atlanan_bayt: atlanan,
        };
        if tx.try_send(parca).is_err() {
            self.atlanan.fetch_add(atlanan + bayt, Ordering::SeqCst);
        }
    }
}
```

**`crates/kalfa-core/src/arac.rs`**: şu bloğu

```rust
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi>;
}
```

şununla değiştir:

```rust
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi>;

    /// Ara çıktı üreten araçlar bunu uygular; oturum araçları bununla
    /// çalıştırır. Varsayılanı `calistir`'dır ve ara çıktı üretmez.
    async fn calistir_akisli(
        &self,
        argumanlar: &Value,
        hazirlik: &Hazirlik,
        iptal: CancellationToken,
        _cikti: CiktiGonderici,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.calistir(argumanlar, hazirlik, iptal).await
    }
}
```

**`crates/kalfa-core/src/izin.rs`**: şu bloğu

```rust

use crate::git_koruma;
use crate::yol::CalismaYolu;
```

şununla değiştir:

```rust

use crate::git_koruma;
use crate::kabuk::{KabukTarifi, YalitimOzeti};
use crate::yol::CalismaYolu;
```

**`crates/kalfa-core/src/izin.rs`**: şu bloğu

```rust
pub const BAGLANTI_UYARISI: &str =
    "Bu dosyanın başka bir yerde de adı var (hard link); yazmak o dosyayı da değiştirir.";

/// Spec §7.4'teki varsayılan gizli dosya kalıpları. Her kalıp her derinlikte
```

şununla değiştir:

```rust
pub const BAGLANTI_UYARISI: &str =
    "Bu dosyanın başka bir yerde de adı var (hard link); yazmak o dosyayı da değiştirir.";
pub const ILK_KABUK_UYARISI: &str = "Bu çalışma alanında ilk kez komut çalıştırılacak. Projenin \
betikleri (build.rs, package.json, Makefile) kod olarak çalışır.";
pub const YALITIMSIZ_UYARISI: &str = "Yalıtımsız: komut senin bütün yetkilerinle çalışacak; ev \
dizinindeki dosyaları okuyabilir ve ağa erişebilir.";
pub const AGLI_KABUK_UYARISI: &str = "Komut ağa erişebilecek. Dosya yalıtımı sürer.";

/// Spec §7.4'teki varsayılan gizli dosya kalıpları. Her kalıp her derinlikte
```

**`crates/kalfa-core/src/izin.rs`**: şu bloğu

```rust
pub enum Karar {
    Serbest,
    Onayli { uyarilar: Vec<String> },
    Yasak { neden: String },
}

impl Karar {
    /// Birden çok hedefin kararını birleştirir: Yasak > Onaylı > Serbest.
    pub fn birlestir(self, diger: Karar) -> Karar {
        match (self, diger) {
            (Karar::Yasak { neden }, _) | (_, Karar::Yasak { neden }) => Karar::Yasak { neden },
            (Karar::Onayli { uyarilar: mut a }, Karar::Onayli { uyarilar: b }) => {
                for u in b {
                    if !a.contains(&u) {
                        a.push(u);
                    }
                }
                Karar::Onayli { uyarilar: a }
            }
            (Karar::Onayli { uyarilar }, Karar::Serbest)
            | (Karar::Serbest, Karar::Onayli { uyarilar }) => Karar::Onayli { uyarilar },
            (Karar::Serbest, Karar::Serbest) => Karar::Serbest,
        }
    }
```

şununla değiştir:

```rust
pub enum Karar {
    Serbest,
    Onayli {
        uyarilar: Vec<String>,
    },
    /// Onay yalnızca yerel iletişim kutusundan verilebilir (aşama 2 tasarımı
    /// §5.2): yalıtımsız kabuk çağrısı.
    YerelOnayli {
        uyarilar: Vec<String>,
    },
    Yasak {
        neden: String,
    },
}

fn uyari_birlestir(mut a: Vec<String>, b: Vec<String>) -> Vec<String> {
    for u in b {
        if !a.contains(&u) {
            a.push(u);
        }
    }
    a
}

impl Karar {
    /// Birden çok hedefin kararını birleştirir: Yasak > Yerel onaylı > Onaylı
    /// > Serbest. Uyarılar birleşir.
    pub fn birlestir(self, diger: Karar) -> Karar {
        match (self, diger) {
            (Karar::Yasak { neden }, _) | (_, Karar::Yasak { neden }) => Karar::Yasak { neden },
            (Karar::YerelOnayli { uyarilar: a }, Karar::YerelOnayli { uyarilar: b })
            | (Karar::YerelOnayli { uyarilar: a }, Karar::Onayli { uyarilar: b })
            | (Karar::Onayli { uyarilar: a }, Karar::YerelOnayli { uyarilar: b }) => {
                Karar::YerelOnayli {
                    uyarilar: uyari_birlestir(a, b),
                }
            }
            (Karar::YerelOnayli { uyarilar }, Karar::Serbest)
            | (Karar::Serbest, Karar::YerelOnayli { uyarilar }) => Karar::YerelOnayli { uyarilar },
            (Karar::Onayli { uyarilar: a }, Karar::Onayli { uyarilar: b }) => Karar::Onayli {
                uyarilar: uyari_birlestir(a, b),
            },
            (Karar::Onayli { uyarilar }, Karar::Serbest)
            | (Karar::Serbest, Karar::Onayli { uyarilar }) => Karar::Onayli { uyarilar },
            (Karar::Serbest, Karar::Serbest) => Karar::Serbest,
        }
    }
}

/// Kabuk çağrısının kararı (aşama 2 tasarımı §5.1).
///
/// - Yalıtımsız çağrı her kipte yalnızca yerel pencerede onaylanır.
/// - Yalıtımlı çağrı Serbest kipte, ağsızsa ve çalışma alanındaki ilk çağrı
///   değilse onay istemez; diğer kiplerde onay ister.
/// - İnceleme kipinde kabuk yoktur; araç modele zaten sunulmaz.
pub fn kabuk_karari(kip: Kip, tarif: &KabukTarifi) -> Karar {
    if kip == Kip::Inceleme {
        return Karar::Yasak {
            neden: "İnceleme kipinde komut çalıştırılamaz.".to_string(),
        };
    }
    let mut uyarilar = Vec::new();
    if tarif.ilk_cagri {
        uyarilar.push(ILK_KABUK_UYARISI.to_string());
    }
    match &tarif.yalitim {
        YalitimOzeti::Yalitimsiz { talimat } => {
            uyarilar.push(YALITIMSIZ_UYARISI.to_string());
            uyarilar.push(talimat.clone());
            Karar::YerelOnayli { uyarilar }
        }
        YalitimOzeti::Yalitimli { .. } => {
            if tarif.ag {
                uyarilar.push(AGLI_KABUK_UYARISI.to_string());
            }
            if kip == Kip::Serbest && uyarilar.is_empty() {
                Karar::Serbest
            } else {
                Karar::Onayli { uyarilar }
            }
        }
    }
```

**`crates/kalfa-core/src/muhur.rs`**: şu bloğu

```rust
        },
    );
    OnayMuhru(hex::encode(h.finalize()))
}
```

şununla değiştir:

```rust
        },
    );
    // Kabuk: komut, dizin, ağ, yalıtım durumu ve profil özeti, ilk çağrı
    // (aşama 2 tasarımı §3.3). Biri değişirse onay geçersizdir.
    if let Some(kabuk) = &hazirlik.kabuk {
        alan(&mut h, b"k");
        alan(
            &mut h,
            serde_json::to_string(kabuk).unwrap_or_default().as_bytes(),
        );
    }
    OnayMuhru(hex::encode(h.finalize()))
}
```

- [ ] **Step 4: Oturumu ve paketleyiciyi yaz**

**`crates/kalfa-core/src/aktarim.rs`**: şu bloğu

```rust

/// Aynı turdaki bitişik iki metin parçasını birleştirir; birleşen parça son
/// parçanın sırasını taşır.
fn birlestir(onceki: &Zarf<Olay>, yeni: &Zarf<Olay>) -> Option<Zarf<Olay>> {
    if onceki.oturum != yeni.oturum || onceki.tur != yeni.tur {
```

şununla değiştir:

```rust

/// Aynı turdaki bitişik iki metin parçasını birleştirir; birleşen parça son
/// parçanın sırasını taşır. Araç çıktısı yalnızca aynı çağrının aynı
/// akışındaki parçayla birleşir (aşama 2 tasarımı §3.3).
fn birlestir(onceki: &Zarf<Olay>, yeni: &Zarf<Olay>) -> Option<Zarf<Olay>> {
    if onceki.oturum != yeni.oturum || onceki.tur != yeni.tur {
```

**`crates/kalfa-core/src/aktarim.rs`**: şu bloğu

```rust
            }
        }
        _ => return None,
    };
```

şununla değiştir:

```rust
            }
        }
        (
            Olay::AracCiktisiParcasi {
                cagri: c1,
                akis: a1,
                metin: m1,
                atlanan_bayt: t1,
            },
            Olay::AracCiktisiParcasi {
                cagri: c2,
                akis: a2,
                metin: m2,
                atlanan_bayt: t2,
            },
        ) if c1 == c2 && a1 == a2 && *t2 == 0 => Olay::AracCiktisiParcasi {
            cagri: *c1,
            akis: *a1,
            metin: format!("{m1}{m2}"),
            atlanan_bayt: *t1,
        },
        _ => return None,
    };
```

**`crates/kalfa-core/src/oturum/mod.rs`**: şu bloğu

```rust
use crate::kayit::{KayitOgesi, OturumKaydi};
use crate::kilit::KaynakKilitleri;
use crate::kimlik::{OturumId, TurId};
use crate::kurtarma::{BILINMEYEN_SONUC, KurtarilmisDurum, KurtarmaHatasi, kurtar};
use crate::protokol::{
    AktarimDurmaNedeni, AracDurumu, BitisNedeni, Komut, Olay, SEMA_SURUMU, Zarf,
};
use crate::saglayici::Saglayici;
```

şununla değiştir:

```rust
use crate::kayit::{KayitOgesi, OturumKaydi};
use crate::kilit::KaynakKilitleri;
use crate::kimlik::CagriId;
use crate::kimlik::{OturumId, TurId};
use crate::kurtarma::{BILINMEYEN_SONUC, KurtarilmisDurum, KurtarmaHatasi, kurtar};
use crate::protokol::{
    AktarimDurmaNedeni, AracDurumu, BitisNedeni, Komut, Olay, OnayOgesi, SEMA_SURUMU, Zarf,
};
use crate::saglayici::Saglayici;
```

**`crates/kalfa-core/src/oturum/mod.rs`**: şu bloğu

```rust

type AktifTur = Arc<Mutex<Option<(TurId, CancellationToken)>>>;

/// Taşıma kuyruğunda terminal hata için ayrılan kapasite. Bu kapasite toplam
```

şununla değiştir:

```rust

type AktifTur = Arc<Mutex<Option<(TurId, CancellationToken)>>>;

/// Onay bekleyen öğeler; yerel iletişim kutusu komutu buradan okur.
type BekleyenOnaylar = Arc<Mutex<HashMap<CagriId, OnayOgesi>>>;

/// Taşıma kuyruğunda terminal hata için ayrılan kapasite. Bu kapasite toplam
```

**`crates/kalfa-core/src/oturum/mod.rs`**: şu bloğu

```rust
    fn kapanma(&self) -> CancellationToken {
        self.durum.kapanma.clone()
    }
```

şununla değiştir:

```rust
    fn kapanma(&self) -> CancellationToken {
        self.durum.kapanma.clone()
    }

    /// Kuyrukta `bayt` büyüklüğünde bir olay için rahat yer var mı: olay ve
    /// bayt bütçesinin yarısı. Atılabilen ara çıktı bununla sınırlanır;
    /// kuyruğun geri kalanı düşürülemeyen olaylara kalır.
    fn yer_var(&self, bayt: usize) -> bool {
        let (olay, bekleyen_bayt) = self.durum.bekleyen();
        // Zarfın ve JSON kaçışlarının payı.
        let tahmin = bayt.saturating_mul(2).saturating_add(256);
        olay < self.durum.butce.azami_olay / 2
            && bekleyen_bayt.saturating_add(tahmin) < self.durum.butce.azami_bayt / 2
    }
```

**`crates/kalfa-core/src/oturum/mod.rs`**: şu bloğu

```rust
    aktif_tur: AktifTur,
    kapanis: watch::Receiver<bool>,
}
```

şununla değiştir:

```rust
    aktif_tur: AktifTur,
    kapanis: watch::Receiver<bool>,
    bekleyen_onaylar: BekleyenOnaylar,
}
```

**`crates/kalfa-core/src/oturum/mod.rs`**: şu bloğu

```rust
    pub fn oturum_id(&self) -> OturumId {
        self.oturum
    }
```

şununla değiştir:

```rust
    pub fn oturum_id(&self) -> OturumId {
        self.oturum
    }

    /// Onay bekleyen bir öğe. Yerel iletişim kutusu gösterdiği komutu
    /// buradan alır (aşama 2 tasarımı §5.2).
    pub fn bekleyen_onay(&self, cagri: CagriId) -> Option<OnayOgesi> {
        self.bekleyen_onaylar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .get(&cagri)
            .cloned()
    }
```

**`crates/kalfa-core/src/oturum/mod.rs`**: şu bloğu

```rust
    /// yürütülür; sürüm değiştiyse yürütülmez (spec §7.6, Task 9).
    politika: Arc<AtomicU64>,
    kapandi: bool,
}
```

şununla değiştir:

```rust
    /// yürütülür; sürüm değiştiyse yürütülmez (spec §7.6, Task 9).
    politika: Arc<AtomicU64>,
    bekleyen_onaylar: BekleyenOnaylar,
    kapandi: bool,
}
```

**`crates/kalfa-core/src/oturum/mod.rs`**: şu bloğu

```rust
        let (olay_tx, olay_rx) = OlayKanali::yeni(ayarlar.olay_butcesi, aktif_tur.clone());
        let olay_kapanma = olay_tx.kapanma();
        let oturum = Self {
            kimlik,
```

şununla değiştir:

```rust
        let (olay_tx, olay_rx) = OlayKanali::yeni(ayarlar.olay_butcesi, aktif_tur.clone());
        let olay_kapanma = olay_tx.kapanma();
        let bekleyen_onaylar = BekleyenOnaylar::default();
        let oturum = Self {
            kimlik,
```

**`crates/kalfa-core/src/oturum/mod.rs`**: şu bloğu

```rust
            aktif_tur: aktif_tur.clone(),
            politika: Arc::new(AtomicU64::new(0)),
            kapandi: false,
        };
```

şununla değiştir:

```rust
            aktif_tur: aktif_tur.clone(),
            politika: Arc::new(AtomicU64::new(0)),
            bekleyen_onaylar: bekleyen_onaylar.clone(),
            kapandi: false,
        };
```

**`crates/kalfa-core/src/oturum/mod.rs`**: şu bloğu

```rust
                aktif_tur,
                kapanis: kapanis_rx,
            },
            olay_rx,
```

şununla değiştir:

```rust
                aktif_tur,
                kapanis: kapanis_rx,
                bekleyen_onaylar,
            },
            olay_rx,
```

**`crates/kalfa-core/src/oturum/mod.rs`**: şu bloğu

```rust
                    .await;
            }
            Komut::AracOnayi { .. } => {
                self.hata("Bu onay artık geçerli değil; bekleyen bir işlem yok.")
                    .await;
```

şununla değiştir:

```rust
                    .await;
            }
            Komut::AracOnayi { .. } | Komut::YerelOnay { .. } => {
                self.hata("Bu onay artık geçerli değil; bekleyen bir işlem yok.")
                    .await;
```

**`crates/kalfa-core/src/oturum/arac_asamasi.rs`** (tam içerik):

```rust
//! Araç aşaması: planlama (doğrulama + izin) → onay → kilitli paralel
//! yürütme (spec §6, §6.1, §7.4–§7.6).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;
use kalfa_evren::tipler::{ChatMessage, ToolCall};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{Etki, Oturum};
use crate::arac::{Arac, AracCiktisi, AracHatasi, CiktiGonderici, CiktiParcasi, Hazirlik};
use crate::izin::{Islem, Karar, Kip, Siniflandirici, YolSinifi, kabuk_karari, karar_ver};
use crate::kayit::KayitOgesi;
use crate::kimlik::{CagriId, IslemId};
use crate::muhur::{OnayMuhru, muhur_hesapla};
use crate::protokol::{AracDurumu, BitisNedeni, Komut, Olay, OnayKarari, OnayOgesi};
use crate::yol::CalismaYolu;

/// Araçlardan oturuma gelen ara çıktı kanalının kapasitesi (parça).
const CIKTI_KANALI: usize = 64;

struct Planlanan {
    cagri: CagriId,
    model_id: String,
    anahtar: String,
    arac: Arc<dyn Arac>,
    argumanlar: Value,
    hazirlik: Hazirlik,
    muhur: OnayMuhru,
    /// Planlandığı andaki politika sürümü (spec §7.6).
    politika: u64,
    kip: Kip,
    karar: Karar,
}

struct CagriSonucu {
    cagri: CagriId,
    model_id: String,
    arac: String,
    /// Tekrar koruması anahtarı: araç adı + ham argüman.
    anahtar: String,
    durum: AracDurumu,
    icerik: String,
    /// Başarısızlık, aynı çağrının üst üste başarısızlık sınırına sayılmaz
    /// (kabukta sıfırdan farklı çıkış kodu; aşama 2 tasarımı §4.5).
    hata_sayilmaz: bool,
}

impl CagriSonucu {
    fn plandan(p: &Planlanan, durum: AracDurumu, icerik: String) -> Self {
        Self {
            cagri: p.cagri,
            model_id: p.model_id.clone(),
            arac: p.arac.ad().to_string(),
            anahtar: p.anahtar.clone(),
            durum,
            icerik,
            hata_sayilmaz: false,
        }
    }
}

struct OnayToplama {
    onaylanan: Vec<Planlanan>,
    reddedilen: Vec<CagriSonucu>,
    yanitsiz: Vec<Planlanan>,
    iptal: Option<BitisNedeni>,
    /// Onay beklenirken izin kipi değişti: bu aşamanın hiçbir çağrısı yürütülmez.
    politika_degisti: bool,
}

pub(super) enum AsamaSonucu {
    Devam,
    Bitir(BitisNedeni),
}

fn tekrar_anahtari(tc: &ToolCall) -> String {
    format!("{}\u{0}{}", tc.function.name, tc.function.arguments)
}

impl Oturum {
    pub(super) async fn arac_asamasi(
        &mut self,
        cagrilar: Vec<ToolCall>,
        hata_sayaci: &mut HashMap<String, u32>,
    ) -> AsamaSonucu {
        let mut serbest = Vec::new();
        let mut onayli = Vec::new();
        for tc in cagrilar {
            let cagri = CagriId::yeni();
            let plan = if self.iptal.is_cancelled() {
                Err("tur iptal edildi".into())
            } else {
                self.planla(cagri, &tc).await
            };
            match plan {
                Ok((plan, Karar::Serbest)) => serbest.push(plan),
                Ok((plan, Karar::Onayli { uyarilar })) => onayli.push((plan, uyarilar, false)),
                Ok((plan, Karar::YerelOnayli { uyarilar })) => onayli.push((plan, uyarilar, true)),
                Ok((plan, Karar::Yasak { neden })) => {
                    let s = CagriSonucu::plandan(
                        &plan,
                        AracDurumu::Reddedildi,
                        format!("[izin verilmedi] {neden}"),
                    );
                    self.sonuc_isle(s, hata_sayaci).await;
                }
                Err(mesaj) => {
                    let durum = if self.iptal.is_cancelled() {
                        AracDurumu::IptalEdildi
                    } else {
                        AracDurumu::Basarisiz
                    };
                    let icerik = if durum == AracDurumu::IptalEdildi {
                        "[çalıştırılmadı: tur iptal edildi]".to_string()
                    } else {
                        format!("[hata] {mesaj}")
                    };
                    let s = CagriSonucu {
                        cagri,
                        model_id: tc.id.clone(),
                        arac: tc.function.name.clone(),
                        anahtar: tekrar_anahtari(&tc),
                        durum,
                        icerik,
                        hata_sayilmaz: false,
                    };
                    self.sonuc_isle(s, hata_sayaci).await;
                }
            }
        }

        // Hazırlıkta iptal, önceden planlanan ve henüz ele alınmayan
        // çağrıları da kapatır; sonraki model isteğinde eksik tool yanıtı kalmaz.
        if self.iptal.is_cancelled() {
            for p in serbest.iter().chain(onayli.iter().map(|(p, _, _)| p)) {
                self.sonuc_isle(
                    CagriSonucu::plandan(
                        p,
                        AracDurumu::IptalEdildi,
                        "[çalıştırılmadı: tur iptal edildi]".into(),
                    ),
                    hata_sayaci,
                )
                .await;
            }
            return AsamaSonucu::Bitir(BitisNedeni::Iptal);
        }

        // Daha sonraki bir hazırlık beklenirken önceki planların kipi de
        // değişmiş olabilir. Geçersiz planlar için onay kartı gösterme.
        let politika = self.politika.load(Ordering::SeqCst);
        let (mut serbest, mut eski): (Vec<_>, Vec<_>) =
            serbest.into_iter().partition(|p| p.politika == politika);
        let (onayli, eski_onayli): (Vec<_>, Vec<_>) = onayli
            .into_iter()
            .partition(|(p, _, _)| p.politika == politika);
        eski.extend(eski_onayli.into_iter().map(|(p, _, _)| p));
        for p in eski {
            self.sonuc_isle(
                CagriSonucu::plandan(
                    &p,
                    AracDurumu::IptalEdildi,
                    format!("[çalıştırılmadı] {}", AracHatasi::PolitikaDegisti),
                ),
                hata_sayaci,
            )
            .await;
        }

        if !onayli.is_empty() {
            let toplama = self.onay_topla(onayli).await;
            for s in toplama.reddedilen {
                self.sonuc_isle(s, hata_sayaci).await;
            }
            if let Some(neden) = toplama.iptal {
                for p in serbest
                    .iter()
                    .chain(&toplama.onaylanan)
                    .chain(&toplama.yanitsiz)
                {
                    let s = CagriSonucu::plandan(
                        p,
                        AracDurumu::IptalEdildi,
                        "[çalıştırılmadı: tur iptal edildi]".into(),
                    );
                    self.sonuc_isle(s, hata_sayaci).await;
                }
                return AsamaSonucu::Bitir(neden);
            }
            if toplama.politika_degisti {
                self.hata(
                    "İzin kipi değişti; bekleyen onaylar geçersiz sayıldı ve bu adımdaki işlemler yapılmadı.",
                )
                .await;
                for p in serbest
                    .iter()
                    .chain(&toplama.onaylanan)
                    .chain(&toplama.yanitsiz)
                {
                    let icerik =
                        "[çalıştırılmadı: izin kipi değişti; gerekirse yeniden dene]".to_string();
                    let s = CagriSonucu::plandan(p, AracDurumu::IptalEdildi, icerik);
                    self.sonuc_isle(s, hata_sayaci).await;
                }
                return AsamaSonucu::Devam;
            }
            serbest.extend(toplama.onaylanan);
        }

        if let Some(neden) = self.yurut(serbest, hata_sayaci).await {
            return AsamaSonucu::Bitir(neden);
        }
        let esik = self.ayarlar.sinirlar.ayni_hata_esigi;
        if hata_sayaci.values().any(|n| *n >= esik) {
            return AsamaSonucu::Bitir(BitisNedeni::Sinir {
                aciklama: format!(
                    "Aynı araç çağrısı üst üste {esik} kez başarısız oldu; döngü durduruldu."
                ),
            });
        }
        AsamaSonucu::Devam
    }

    async fn planla(
        &mut self,
        cagri: CagriId,
        tc: &ToolCall,
    ) -> Result<(Planlanan, Karar), String> {
        let ad = tc.function.name.as_str();
        let arac = self
            .baglam
            .araclar
            .bul(ad)
            .ok_or_else(|| format!("`{ad}` adlı bir araç yok."))?;
        let kip = self.ayarlar.kip;
        if kip == Kip::Inceleme && arac.yazar_mi() {
            return Err("İnceleme kipinde bu araç kullanılamaz; bu kip yalnızca okur.".into());
        }
        let argumanlar = self.baglam.araclar.dogrula(ad, &tc.function.arguments)?;
        let siniflandirici = self.baglam.siniflandirici.clone();
        // Hazırlık beklerken komut kanalını da işle. Politika sürümünü
        // beklemeden önce al: hazırlık sırasında değişen kip eski işlemi yetkilendirmez.
        let politika = self.politika.load(Ordering::SeqCst);
        let hazirlik = self
            .iptal_edilebilir(arac.hazirla(&argumanlar))
            .await
            .map_err(|_| "tur iptal edildi".to_string())?
            .map_err(|e| e.to_string())?;
        let karar = Self::karar(kip, &siniflandirici, &hazirlik);
        let muhur = muhur_hesapla(cagri, ad, &argumanlar, &hazirlik);
        let plan = Planlanan {
            cagri,
            model_id: tc.id.clone(),
            anahtar: tekrar_anahtari(tc),
            arac,
            argumanlar,
            hazirlik,
            muhur,
            politika,
            kip,
            karar: karar.clone(),
        };
        Ok((plan, karar))
    }

    fn karar(kip: Kip, s: &Siniflandirici, hazirlik: &Hazirlik) -> Karar {
        let mut karar = Karar::Serbest;
        for yol in &hazirlik.okunan {
            let sinif = if s.gizli_mi(yol) {
                YolSinifi::Gizli
            } else {
                YolSinifi::Normal
            };
            karar = karar.birlestir(karar_ver(kip, Islem::Okuma, sinif, false));
        }
        for hedef in &hazirlik.yazilan {
            let yeni = karar_ver(
                kip,
                Islem::Yazma,
                s.sinifla(&hedef.yol),
                hedef.baglanti_sayisi > 1,
            );
            karar = karar.birlestir(yeni);
        }
        if let Some(kabuk) = &hazirlik.kabuk {
            karar = karar.birlestir(kabuk_karari(kip, kabuk));
        }
        karar
    }

    async fn onay_topla(&mut self, bekleyen: Vec<(Planlanan, Vec<String>, bool)>) -> OnayToplama {
        let ogeler: Vec<OnayOgesi> = bekleyen
            .iter()
            .map(|(p, uyarilar, yerel)| OnayOgesi {
                cagri: p.cagri,
                arac: p.arac.ad().to_string(),
                aciklama: p.hazirlik.aciklama.clone(),
                hedefler: hedefler(&p.hazirlik),
                diff: p.hazirlik.diff.clone(),
                uyarilar: uyarilar.clone(),
                muhur: p.muhur.clone(),
                kabuk: p.hazirlik.kabuk.clone(),
                yerel_onay: *yerel,
            })
            .collect();
        // Yerel iletişim kutusu komutu çekirdekten okur, webview'dan değil
        // (aşama 2 tasarımı §5.2).
        *self
            .bekleyen_onaylar
            .lock()
            .unwrap_or_else(|z| z.into_inner()) =
            ogeler.iter().map(|o| (o.cagri, o.clone())).collect();
        let yerel: HashSet<CagriId> = bekleyen
            .iter()
            .filter(|(_, _, yerel)| *yerel)
            .map(|(p, _, _)| p.cagri)
            .collect();
        let mut kalan: HashMap<CagriId, Planlanan> =
            bekleyen.into_iter().map(|(p, _, _)| (p.cagri, p)).collect();
        let mut t = OnayToplama {
            onaylanan: Vec::new(),
            reddedilen: Vec::new(),
            yanitsiz: Vec::new(),
            iptal: None,
            politika_degisti: false,
        };
        let baslangic_politikasi = self.politika.load(Ordering::SeqCst);
        self.yay(Olay::OnayGerekiyor {
            islem: IslemId::yeni(),
            ogeler,
        })
        .await;

        while !kalan.is_empty() {
            let Some(komut) = self.tur_komutu_al().await else {
                t.iptal = Some(BitisNedeni::Iptal);
                break;
            };
            match komut {
                Komut::AracOnayi {
                    cagri,
                    karar,
                    muhur,
                } => match kalan.get(&cagri).map(|p| p.muhur == muhur) {
                    None => {
                        self.hata("Bu onay artık geçerli değil; bekleyen bir işlem yok.")
                            .await
                    }
                    Some(false) => {
                        self.hata("Onay mührü eşleşmedi; onay kartı güncel değil.")
                            .await
                    }
                    Some(true) if karar == OnayKarari::Onayla && yerel.contains(&cagri) => {
                        self.hata("Bu komut yalnızca yerel pencerede onaylanabilir.")
                            .await
                    }
                    Some(true) => {
                        if let Some(p) = kalan.remove(&cagri) {
                            match karar {
                                OnayKarari::Onayla => t.onaylanan.push(p),
                                OnayKarari::Reddet { gerekce } => {
                                    let g = gerekce
                                        .filter(|g| !g.trim().is_empty())
                                        .unwrap_or_else(|| "gerekçe belirtilmedi".into());
                                    let icerik = format!("[kullanıcı reddetti] {g}");
                                    t.reddedilen.push(CagriSonucu::plandan(
                                        &p,
                                        AracDurumu::Reddedildi,
                                        icerik,
                                    ));
                                }
                            }
                        }
                    }
                },
                Komut::YerelOnay { cagri, muhur } => {
                    match kalan.get(&cagri).map(|p| p.muhur == muhur) {
                        None => {
                            self.hata("Bu onay artık geçerli değil; bekleyen bir işlem yok.")
                                .await
                        }
                        Some(false) => {
                            self.hata("Onay mührü eşleşmedi; onay kartı güncel değil.")
                                .await
                        }
                        Some(true) if !yerel.contains(&cagri) => {
                            self.hata("Bu işlem yerel onay gerektirmiyor.").await
                        }
                        Some(true) => {
                            if let Some(p) = kalan.remove(&cagri) {
                                t.onaylanan.push(p);
                            }
                        }
                    }
                }
                diger => {
                    if self.genel_komut(diger).await == Etki::Iptal {
                        t.iptal = Some(BitisNedeni::Iptal);
                        break;
                    }
                    if self.politika.load(Ordering::SeqCst) != baslangic_politikasi {
                        t.politika_degisti = true;
                        break;
                    }
                }
            }
        }
        t.yanitsiz = kalan.into_values().collect();
        self.bekleyen_onaylar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .clear();
        t
    }

    /// Planları paralel yürütür; iptal geldiyse ya da kayıt yazılamadıysa
    /// nedeni döner. Sonuçlar, araçların iptal belirtecini görmesi için
    /// yine toplanır.
    async fn yurut(
        &mut self,
        planlar: Vec<Planlanan>,
        hata_sayaci: &mut HashMap<String, u32>,
    ) -> Option<BitisNedeni> {
        let mut calisacak = Vec::with_capacity(planlar.len());
        let mut kaydi_yazilamayan = Vec::new();
        for p in planlar {
            let oge = KayitOgesi::AracBasladi {
                cagri: p.cagri,
                model_cagri_id: p.model_id.clone(),
                arac: p.arac.ad().to_string(),
            };
            // Yan etkili aracın "başladı" kaydı diske ulaşmadan araç çalışmaz.
            let yazildi = if p.arac.yazar_mi() {
                self.kaydet_kalici(oge).await
            } else {
                self.kaydet(oge).await
            };
            if !yazildi {
                kaydi_yazilamayan.push(p);
                continue;
            }
            let olay = Olay::AracCagrisiBasladi {
                cagri: p.cagri,
                arac: p.arac.ad().to_string(),
                aciklama: p.hazirlik.aciklama.clone(),
            };
            self.yay(olay).await;
            calisacak.push(p);
        }
        // Henüz hiçbir future poll edilmedi. Grubun sonraki kaydı bozulduysa
        // önceki başarılı başlangıç kayıtları da yürütülmez.
        if self.kayit_hatasi.is_some() {
            kaydi_yazilamayan.append(&mut calisacak);
        }
        for p in &kaydi_yazilamayan {
            let icerik = "[çalıştırılmadı: oturum kaydı yazılamadı]".to_string();
            self.sonuc_isle(
                CagriSonucu::plandan(p, AracDurumu::IptalEdildi, icerik),
                hata_sayaci,
            )
            .await;
        }

        let azami = self.ayarlar.sinirlar.azami_arac_ciktisi;
        let (cikti_tx, mut cikti_rx) = mpsc::channel::<CiktiParcasi>(CIKTI_KANALI);
        let mut isler = FuturesUnordered::new();
        // Araç tarafında atılan (oturuma hiç ulaşmayan) baytların sayaçları.
        let mut arac_sayaclari = HashMap::new();
        for p in calisacak {
            // Kilitler çalışma alanı düzeyinde ortaktır.
            let kilitler = self.baglam.kilitler.clone();
            let politika = self.politika.clone();
            let siniflandirici = self.baglam.siniflandirici.clone();
            let iptal = self.iptal.child_token();
            let cikti = CiktiGonderici::yeni(p.cagri, cikti_tx.clone());
            arac_sayaclari.insert(p.cagri, cikti.atlanan_sayaci());
            isler.push(async move {
                let sonuc =
                    tek_calistir(&p, &kilitler, &politika, &siniflandirici, iptal, cikti).await;
                (p, sonuc)
            });
        }
        drop(cikti_tx);

        let mut iptal_nedeni = None;
        let mut cikti_kapali = false;
        // Arayüze iletilemeyen ara çıktının çağrı başına bayt sayısı.
        let mut atlanan: HashMap<CagriId, u64> = HashMap::new();
        loop {
            tokio::select! {
                biased;
                _ = self.iptal.cancelled(), if iptal_nedeni.is_none() => iptal_nedeni = Some(BitisNedeni::Iptal),
                // Ara çıktı sonuçtan önce işlenir: araç parçayı sonucu
                // döndürmeden önce gönderir.
                parca = cikti_rx.recv(), if !cikti_kapali => match parca {
                    Some(parca) => self.cikti_ilet(parca, &mut atlanan).await,
                    None => cikti_kapali = true,
                },
                komut = self.komutlar.recv(), if iptal_nedeni.is_none() => {
                    match self.komut_geldi(komut).await {
                        Some(k) => {
                            if self.genel_komut(k).await == Etki::Iptal {
                                iptal_nedeni = Some(BitisNedeni::Iptal);
                            }
                        }
                        None => iptal_nedeni = Some(BitisNedeni::Iptal),
                    }
                }
                sonraki = isler.next() => match sonraki {
                    Some((p, sonuc)) => {
                        if let Some(sayac) = arac_sayaclari.remove(&p.cagri) {
                            let bayt = sayac.swap(0, Ordering::SeqCst);
                            *atlanan.entry(p.cagri).or_default() += bayt;
                        }
                        self.atlanani_bildir(p.cagri, &mut atlanan).await;
                        let s = sonuc_olustur(&p, sonuc, azami);
                        self.sonuc_isle(s, hata_sayaci).await;
                    }
                    None => break,
                },
            }
        }
        if self.kayit_hatasi.is_some() {
            Some(BitisNedeni::Hata)
        } else {
            iptal_nedeni
        }
    }

    /// Ara çıktı parçasını arayüze iletir. Olay kuyruğu yarıdan fazla doluysa
    /// parça atılır ve sayılır: çıktı seli kuyruğu doldurup turu iptal
    /// ettirmemeli (aşama 2 tasarımı §4.4).
    async fn cikti_ilet(&mut self, parca: CiktiParcasi, atlanan: &mut HashMap<CagriId, u64>) {
        let onceki = atlanan.remove(&parca.cagri).unwrap_or(0) + parca.atlanan_bayt;
        if self.olaylar.yer_var(parca.metin.len()) {
            self.yay(Olay::AracCiktisiParcasi {
                cagri: parca.cagri,
                akis: parca.akis,
                metin: parca.metin,
                atlanan_bayt: onceki,
            })
            .await;
        } else {
            atlanan.insert(parca.cagri, onceki + parca.metin.len() as u64);
        }
    }

    /// Çağrı bitti: atılmış ama bildirilmemiş çıktı varsa boş bir parçayla
    /// bildirilir. Bu parça atılmaz; çağrı başına en çok bir tane olur ve
    /// kuyruğun yarısı düşürülemeyen olaylara ayrılmıştır.
    async fn atlanani_bildir(&mut self, cagri: CagriId, atlanan: &mut HashMap<CagriId, u64>) {
        let Some(bayt) = atlanan.remove(&cagri) else {
            return;
        };
        if bayt > 0 {
            self.yay(Olay::AracCiktisiParcasi {
                cagri,
                akis: crate::protokol::CiktiAkisi::Standart,
                metin: String::new(),
                atlanan_bayt: bayt,
            })
            .await;
        }
    }

    /// Sonucu bağlama, kayda ve arayüze işler. Mesaj `AracBitti`'den önce
    /// yazılır: çökme arada olursa kurtarma sonucu bilir.
    async fn sonuc_isle(&mut self, s: CagriSonucu, hata_sayaci: &mut HashMap<String, u32>) {
        if s.hata_sayilmaz {
            // Sayaç ne artar ne sıfırlanır.
        } else if s.durum == AracDurumu::Basarisiz {
            *hata_sayaci.entry(s.anahtar.clone()).or_default() += 1;
        } else {
            hata_sayaci.remove(&s.anahtar);
        }
        self.mesaj_ekle(ChatMessage::tool(s.model_id.clone(), s.icerik.clone()))
            .await;
        self.kaydet(KayitOgesi::AracBitti {
            cagri: s.cagri,
            durum: s.durum,
        })
        .await;
        self.yay(Olay::AracSonucu {
            cagri: s.cagri,
            arac: s.arac,
            durum: s.durum,
            icerik: s.icerik,
        })
        .await;
    }
}

fn hedefler(h: &Hazirlik) -> Vec<String> {
    h.okunan
        .iter()
        .map(|y| y.to_string())
        .chain(h.yazilan.iter().map(|y| y.yol.to_string()))
        .collect()
}

async fn tek_calistir(
    p: &Planlanan,
    kilitler: &crate::kilit::KaynakKilitleri,
    politika: &AtomicU64,
    siniflandirici: &Siniflandirici,
    iptal: CancellationToken,
    cikti: CiktiGonderici,
) -> Result<AracCiktisi, AracHatasi> {
    let yazilan: Vec<CalismaYolu> = p.hazirlik.yazilan.iter().map(|h| h.yol.clone()).collect();
    let _kilit = tokio::select! {
        k = kilitler.al(&p.hazirlik.okunan, &yazilan, p.hazirlik.tum_calisma_alani) => k,
        _ = iptal.cancelled() => return Err(AracHatasi::Iptal),
    };
    // İzin kipi planlamadan sonra değişti ise yeni kipte yeniden
    // yetkilendirilmeden işlem yapılmaz.
    if politika.load(Ordering::SeqCst) != p.politika {
        return Err(AracHatasi::PolitikaDegisti);
    }
    // Kilit altında hedef yeniden hazırlanır: onaydan sonra değiştiyse
    // mührü tutmaz ve işlem yapılmaz.
    let taze = tokio::select! {
        biased;
        _ = iptal.cancelled() => return Err(AracHatasi::Iptal),
        h = p.arac.hazirla(&p.argumanlar) => h?,
    };
    if muhur_hesapla(p.cagri, p.arac.ad(), &p.argumanlar, &taze) != p.muhur {
        return Err(AracHatasi::OnayEskidi);
    }
    // Etkin git koruması değişmiş olabilir; yeni karar eski onayla geçmez.
    if Oturum::karar(p.kip, siniflandirici, &taze) != p.karar {
        return Err(AracHatasi::PolitikaDegisti);
    }
    if iptal.is_cancelled() {
        return Err(AracHatasi::Iptal);
    }
    // `hazirla` await içerir; bu sırada kip değişmiş olabilir.
    if politika.load(Ordering::SeqCst) != p.politika {
        return Err(AracHatasi::PolitikaDegisti);
    }
    p.arac
        .calistir_akisli(&p.argumanlar, &taze, iptal, cikti)
        .await
}

fn sonuc_olustur(
    p: &Planlanan,
    sonuc: Result<AracCiktisi, AracHatasi>,
    azami: usize,
) -> CagriSonucu {
    match sonuc {
        Ok(c) => {
            let icerik = kirp(format!("[kaynak: {}]\n{}", c.kaynak, c.icerik), azami);
            CagriSonucu::plandan(p, AracDurumu::Basarili, icerik)
        }
        Err(AracHatasi::Basarisiz(c)) => {
            let icerik = kirp(format!("[kaynak: {}]\n{}", c.kaynak, c.icerik), azami);
            CagriSonucu {
                hata_sayilmaz: true,
                ..CagriSonucu::plandan(p, AracDurumu::Basarisiz, icerik)
            }
        }
        Err(AracHatasi::Iptal) => {
            CagriSonucu::plandan(p, AracDurumu::IptalEdildi, "[iptal edildi]".into())
        }
        Err(e @ AracHatasi::PolitikaDegisti) => {
            CagriSonucu::plandan(p, AracDurumu::IptalEdildi, format!("[çalıştırılmadı] {e}"))
        }
        Err(e) => CagriSonucu::plandan(p, AracDurumu::Basarisiz, format!("[hata] {e}")),
    }
}

fn kirp(metin: String, azami: usize) -> String {
    if metin.len() <= azami {
        return metin;
    }
    let mut kes = azami;
    while !metin.is_char_boundary(kes) {
        kes -= 1;
    }
    format!(
        "{}\n[çıktı {} bayttan {} bayta kırpıldı]",
        &metin[..kes],
        metin.len(),
        kes
    )
}
```

- [ ] **Step 5: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa-core --locked`
Expected: bütün testler geçer; yeni olanlar `oturum_kabuk.rs`'in 6 testi, `izin.rs`'te `kabuk_kararlari` ve `yerel_onay_birlesmede_onayliyi_ezer`, `arac_muhur.rs`'te `kabuk_muhru_tarifin_her_alanina_baglidir`, `aktarim.rs`'te `arac_ciktisi_ayni_cagri_ve_akista_birlesir`, `protokol.rs`'te `kabuk_json_bicimi`. Görevin sonundaki genel denetimleri de koştur; dosya araçları ve uygulama değişmeden derlenir.

- [ ] **Step 6: Commit**

```bash
git add crates/kalfa-core
git commit -m "çekirdek: kabuk tarifi, yerel onay ve ara çıktı"
```

---

### Task 3: `kalfa-tools`: `kabuk` aracı

**Files:**
- Create: `crates/kalfa-tools/src/kabuk/{mod,cikti}.rs`, `crates/kalfa-tools/tests/kabuk.rs`
- Modify: `crates/kalfa-tools/src/lib.rs`, `crates/kalfa-tools/src/calisma_alani.rs`

**Interfaces:**
- Consumes: Görev 1 (`Yalitici`, `Yalitimsiz`, `KomutBicimi`, `CalistirmaIstegi`, `Bitis`, `ortam::uret`, `program::yorumlayici_adi`), Görev 2 (`KabukTarifi`, `CiktiGonderici`, `AracHatasi::Basarisiz`, `calistir_akisli`).
- Produces (`kalfa_tools`):
  - `KabukAraci::yeni(Arc<CalismaAlani>, Arc<dyn Yalitici>, Arc<dyn KabukGuveni>)` (`Arac` uygular, adı `kabuk`, `yazar_mi() = true`).
  - `trait KabukGuveni: Send + Sync { fn guvenildi_mi(&self) -> bool; fn guvenildi(&self); }`.
  - `CalismaAlani::dizin_dogrula(&CalismaYolu) -> io::Result<()>`.
  - `kabuk::cikti::{BasSonTampon, temizle, CanliAkis, BAS_BAYT = 10_000, SON_BAYT = 20_000, CANLI_SINIR = 1 MiB}`.

**Davranış notları (testler sınar):**
- **Argümanlar:** şema `oneOf [program, betik]`, `additionalProperties: false`, `zaman_asimi_sn` 1–1800 (varsayılan 120). Ayrıca reddedilenler: mutlak ya da `..` içeren program yolu, boş program, boş betik, betikle birlikte `argumanlar`, çalışma alanında olmayan ya da bağlantı içeren dizin.
- **Hazırlık:** `tum_calisma_alani = true`. Tarifin gösterimi program ve argümanlardır; özel karakterli argüman tek tırnakla gösterilir (`'iki kelime'`). Açıklama: "`<ilk satır, en çok 80 karakter>` komutunu çalıştırmak istiyor." `ilk_cagri` güven kaydından gelir.
- **Çalıştırma:** güven, komut başladıktan sonra kaydedilir; başlamayan komut kaydetmez. Başlatılamayan komut `AracHatasi::Gecersiz` döner.
- **Sonuç metni:** `Çıkış kodu: <n>` (ya da zaman aşımı, sinyal), `Süre: <s,s> sn`, `--- çıktı ---` ve çıktı (ya da `--- çıktı yok ---`). Çıkış kodu 0 değilse ve zaman aşımında `AracHatasi::Basarisiz`. Kaynak etiketi `kabuk:<program adı>`, betikte `kabuk:sh` ya da `kabuk:PowerShell`.
- **Çıktı:** model baştan 10 000, sondan 20 000 bayt alır; kırpma UTF-8 karakter sınırında yapılır, arada kırpılan bayt sayısı yazılır. ANSI kaçışları atılır; satır başıyla (`\r`) üzerine yazılan satırın yalnızca son hâli kalır. Canlı akış, bölünen çok baytlı karakteri sonraki parçaya bırakır ve 1 MiB'ta bir kez bildirip durur.

- [ ] **Step 1: Başarısız testleri yaz**

**`crates/kalfa-tools/tests/kabuk.rs`** (tam içerik):

```rust
//! `kabuk` aracı gerçek komutlarla, yalıtımsız arka uçta (aşama 2 tasarımı
//! §4, §12.1).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use kalfa_core::arac::{Arac, AracHatasi, AracKayitDefteri, CiktiGonderici, Hazirlik};
use kalfa_core::izin::Siniflandirici;
use kalfa_core::kabuk::{KabukBicimi, YalitimOzeti};
use kalfa_core::kimlik::CagriId;
use kalfa_core::protokol::CiktiAkisi;
use kalfa_sandbox::Yalitimsiz;
use kalfa_tools::{CalismaAlani, KabukAraci, KabukGuveni};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct SahteGuven {
    guvenildi: AtomicBool,
    kayit: AtomicUsize,
}

impl KabukGuveni for SahteGuven {
    fn guvenildi_mi(&self) -> bool {
        self.guvenildi.load(Ordering::SeqCst)
    }
    fn guvenildi(&self) {
        self.guvenildi.store(true, Ordering::SeqCst);
        self.kayit.fetch_add(1, Ordering::SeqCst);
    }
}

fn kur() -> (TempDir, KabukAraci, Arc<SahteGuven>) {
    let d = TempDir::new().unwrap();
    let s = Arc::new(Siniflandirici::yeni(d.path(), &[]).unwrap());
    let alan = CalismaAlani::ac(s).unwrap();
    let guven = Arc::new(SahteGuven::default());
    let arac = KabukAraci::yeni(alan, Arc::new(Yalitimsiz), guven.clone());
    (d, arac, guven)
}

/// Platforma göre betik: Linux'ta sh, Windows'ta PowerShell.
fn betik(sh: &str, ps: &str) -> Value {
    json!({ "betik": if cfg!(windows) { ps } else { sh } })
}

async fn calistir(
    arac: &KabukAraci,
    a: &Value,
) -> (Result<String, AracHatasi>, Vec<(CiktiAkisi, String)>) {
    let h = arac.hazirla(a).await.expect("hazırlık");
    let (tx, mut rx) = tokio::sync::mpsc::channel(1024);
    let sonuc = arac
        .calistir_akisli(
            a,
            &h,
            CancellationToken::new(),
            CiktiGonderici::yeni(CagriId::yeni(), tx),
        )
        .await
        .map(|c| format!("[{}] {}", c.kaynak, c.icerik));
    let mut parcalar = Vec::new();
    while let Ok(p) = rx.try_recv() {
        parcalar.push((p.akis, p.metin));
    }
    (sonuc, parcalar)
}

#[tokio::test]
async fn hazirlik_tarifi_uretir_ve_ilk_cagriyi_bilir() {
    let (d, arac, guven) = kur();
    std::fs::create_dir(d.path().join("alt")).unwrap();
    let a = json!({
        "program": "cargo",
        "argumanlar": ["test", "--features=e2e", "iki kelime"],
        "dizin": "alt",
        "ag": true
    });
    let h: Hazirlik = arac.hazirla(&a).await.unwrap();
    assert!(
        h.tum_calisma_alani,
        "kabuk çalışma alanının özel kilidini alır"
    );
    assert_eq!(
        h.aciklama,
        "`cargo test --features=e2e 'iki kelime'` komutunu çalıştırmak istiyor."
    );
    let t = h.kabuk.expect("kabuk tarifi");
    assert_eq!(t.komut, "cargo test --features=e2e 'iki kelime'");
    assert_eq!(t.bicim, KabukBicimi::Program);
    assert_eq!(t.dizin, "alt");
    assert!(t.ag);
    assert!(matches!(t.yalitim, YalitimOzeti::Yalitimsiz { .. }));
    assert!(t.ilk_cagri);

    guven.guvenildi.store(true, Ordering::SeqCst);
    let t = arac
        .hazirla(&json!({"betik": "echo a"}))
        .await
        .unwrap()
        .kabuk
        .unwrap();
    assert!(!t.ilk_cagri);
    assert!(matches!(t.bicim, KabukBicimi::Betik { .. }));
    assert_eq!(t.dizin, ".");
}

#[tokio::test]
async fn gecersiz_argumanlar_reddedilir() {
    let (d, arac, _) = kur();
    std::fs::write(d.path().join("dosya.txt"), "x").unwrap();
    let defter = AracKayitDefteri::yeni(vec![Arc::new(kur().1) as Arc<dyn Arac>]).unwrap();
    for metin in [
        r#"{"program": "a", "betik": "b"}"#,
        r#"{"dizin": "."}"#,
        r#"{"program": "a", "fazla": 1}"#,
        r#"{"program": "a", "zaman_asimi_sn": 5000}"#,
    ] {
        assert!(defter.dogrula("kabuk", metin).is_err(), "{metin}");
    }
    for a in [
        json!({"program": "/usr/bin/env"}),
        json!({"program": "C:\\Windows\\System32\\cmd.exe"}),
        json!({"program": "../disarida"}),
        json!({"program": "a", "dizin": ".."}),
        json!({"program": "a", "dizin": "yok"}),
        json!({"program": "a", "dizin": "dosya.txt"}),
        json!({"betik": "echo", "argumanlar": ["a"]}),
        json!({"betik": "   "}),
        json!({"program": ""}),
    ] {
        assert!(arac.hazirla(&a).await.is_err(), "{a}");
    }
}

#[tokio::test]
async fn calisma_dizini_baglanti_olamaz() {
    let (d, arac, _) = kur();
    let dis = TempDir::new().unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(dis.path(), d.path().join("baglanti")).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(dis.path(), d.path().join("baglanti"))
        .expect("Windows test ajanında symlink yetkisi gerekir");
    assert!(
        arac.hazirla(&json!({"program": "a", "dizin": "baglanti"}))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn basarili_komut_ciktisi_modele_ve_canli_akisa_gider() {
    let (d, arac, guven) = kur();
    std::fs::create_dir(d.path().join("alt")).unwrap();
    let mut a = betik(
        "echo merhaba; pwd; echo uyari 1>&2",
        "Write-Output merhaba; (Get-Location).Path; [Console]::Error.WriteLine('uyari')",
    );
    a["dizin"] = json!("alt");
    let (sonuc, parcalar) = calistir(&arac, &a).await;
    let metin = sonuc.expect("çıkış kodu 0");
    let kaynak = if cfg!(windows) {
        "kabuk:PowerShell"
    } else {
        "kabuk:sh"
    };
    assert!(
        metin.starts_with(&format!("[{kaynak}] Çıkış kodu: 0\nSüre: ")),
        "{metin}"
    );
    assert!(metin.contains("--- çıktı ---\nmerhaba"), "{metin}");
    assert!(
        metin.contains("alt"),
        "komut `alt` dizininde çalışmalı: {metin}"
    );
    assert!(
        parcalar
            .iter()
            .any(|(a, m)| *a == CiktiAkisi::Standart && m.contains("merhaba"))
    );
    assert!(
        parcalar
            .iter()
            .any(|(a, m)| *a == CiktiAkisi::Hata && m.contains("uyari"))
    );
    assert_eq!(guven.kayit.load(Ordering::SeqCst), 1, "ilk onay kaydedilir");
}

#[tokio::test]
async fn program_bicimi_calisir() {
    let (_d, arac, _) = kur();
    let a = if cfg!(windows) {
        json!({"program": "cmd", "argumanlar": ["/c", "echo tamam"]})
    } else {
        json!({"program": "sh", "argumanlar": ["-c", "echo tamam"]})
    };
    let (sonuc, _) = calistir(&arac, &a).await;
    let metin = sonuc.unwrap();
    assert!(metin.contains("tamam"), "{metin}");
    let ad = if cfg!(windows) {
        "kabuk:cmd"
    } else {
        "kabuk:sh"
    };
    assert!(metin.starts_with(&format!("[{ad}]")), "{metin}");
}

#[tokio::test]
async fn sifirdan_farkli_cikis_basarisiz_ama_ciktisiyla_doner() {
    let (_d, arac, _) = kur();
    let (sonuc, _) = calistir(
        &arac,
        &betik("echo bozuk; exit 3", "Write-Output bozuk; exit 3"),
    )
    .await;
    match sonuc {
        Err(AracHatasi::Basarisiz(c)) => {
            assert!(c.icerik.starts_with("Çıkış kodu: 3\n"), "{}", c.icerik);
            assert!(c.icerik.contains("bozuk"));
        }
        diger => panic!("{diger:?}"),
    }
}

#[tokio::test]
async fn zaman_asimi_basarisiz_doner() {
    let (_d, arac, _) = kur();
    let mut a = betik("sleep 30", "Start-Sleep -Seconds 30");
    a["zaman_asimi_sn"] = json!(1);
    let (sonuc, _) = calistir(&arac, &a).await;
    match sonuc {
        Err(AracHatasi::Basarisiz(c)) => {
            assert!(c.icerik.starts_with("Zaman aşımı (1 sn)"), "{}", c.icerik)
        }
        diger => panic!("{diger:?}"),
    }
}

#[tokio::test]
async fn iptal_komutu_durdurur() {
    let (_d, arac, _) = kur();
    let a = betik("sleep 30", "Start-Sleep -Seconds 30");
    let h = arac.hazirla(&a).await.unwrap();
    let iptal = CancellationToken::new();
    let tetik = iptal.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(1500)).await;
        tetik.cancel();
    });
    let baslangic = std::time::Instant::now();
    let sonuc = arac
        .calistir_akisli(&a, &h, iptal, CiktiGonderici::bos())
        .await;
    assert!(matches!(sonuc, Err(AracHatasi::Iptal)), "{sonuc:?}");
    assert!(baslangic.elapsed() < Duration::from_secs(10));
}

#[tokio::test]
async fn olmayan_program_hata_verir() {
    let (_d, arac, guven) = kur();
    let (sonuc, _) = calistir(&arac, &json!({"program": "kalfa-kesinlikle-olmayan"})).await;
    match sonuc {
        Err(AracHatasi::Gecersiz(m)) => assert!(m.contains("bulunamadı"), "{m}"),
        diger => panic!("{diger:?}"),
    }
    assert_eq!(
        guven.kayit.load(Ordering::SeqCst),
        0,
        "başlamayan komut güven kaydetmez"
    );
}
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa-tools --locked --test kabuk`
Expected: derleme hatası (`KabukAraci`, `KabukGuveni` yok).

- [ ] **Step 3: Aracı yaz**

**`crates/kalfa-tools/src/calisma_alani.rs`**: şu bloğu

```rust
            "Dosya yolu boş",
        ))
    }
```

şununla değiştir:

```rust
            "Dosya yolu boş",
        ))
    }

    /// Yolun çalışma alanının içinde bir dizin olduğunu doğrular; yolun hiçbir
    /// bileşeni bağlantı olamaz. Kabuğun çalışma dizini için (aşama 2
    /// tasarımı §4.1).
    pub fn dizin_dogrula(&self, yol: &CalismaYolu) -> io::Result<()> {
        let (ust, ad) = self.ust_dizin(yol, false)?;
        ust.open_dir_nofollow(ad)?;
        Ok(())
    }
```

**`crates/kalfa-tools/src/kabuk/cikti.rs`** (tam içerik):

```rust
//! Kabuk çıktısının işlenmesi (aşama 2 tasarımı §4.4): modele giden baş ve
//! son, arayüze giden canlı akış.

use std::collections::VecDeque;
use std::fmt::Write;

use kalfa_core::arac::CiktiGonderici;
use kalfa_core::protokol::CiktiAkisi;
use kalfa_sandbox::Akis;
use regex::Regex;

/// Modele giden çıktının baştan ve sondan tutulan bayt sayısı.
pub const BAS_BAYT: usize = 10_000;
pub const SON_BAYT: usize = 20_000;
/// Arayüze bir çağrı için giden canlı çıktının üst sınırı. Terminal görünümü
/// zaten en çok 200 000 karakter tutar (aşama 2 tasarımı §10.1).
pub const CANLI_SINIR: usize = 1024 * 1024;

/// Çıktının ilk `BAS_BAYT`'ını ve son `SON_BAYT`'ını tutar; bellek çıktının
/// boyutundan bağımsızdır.
#[derive(Debug, Default)]
pub struct BasSonTampon {
    bas: Vec<u8>,
    son: VecDeque<u8>,
    toplam: u64,
}

impl BasSonTampon {
    pub fn ekle(&mut self, veri: &[u8]) {
        self.toplam += veri.len() as u64;
        let bas_payi = (BAS_BAYT - self.bas.len()).min(veri.len());
        self.bas.extend_from_slice(&veri[..bas_payi]);
        let kalan = &veri[bas_payi..];
        let kalan = &kalan[kalan.len().saturating_sub(SON_BAYT)..];
        self.son.extend(kalan);
        let fazla = self.son.len().saturating_sub(SON_BAYT);
        self.son.drain(..fazla);
    }

    pub fn toplam(&self) -> u64 {
        self.toplam
    }

    /// Modele gidecek metin: kırpıldıysa baş, kırpılan bayt sayısı ve son.
    pub fn metin(&self) -> String {
        let son: Vec<u8> = self.son.iter().copied().collect();
        let kirpilan = self.toplam - (self.bas.len() + son.len()) as u64;
        if kirpilan == 0 {
            let mut hepsi = self.bas.clone();
            hepsi.extend_from_slice(&son);
            return temizle(&hepsi);
        }
        let bas = bas_sinirinda_kes(&self.bas);
        let son = son_sinirinda_kes(&son);
        let atilan =
            kirpilan + (self.bas.len() - bas.len()) as u64 + (self.son.len() - son.len()) as u64;
        let mut metin = temizle(bas);
        let _ = write!(metin, "\n[... {atilan} bayt kırpıldı ...]\n");
        metin.push_str(&temizle(son));
        metin
    }
}

/// Sonda yarım kalmış çok baytlı karakteri atar.
fn bas_sinirinda_kes(veri: &[u8]) -> &[u8] {
    let gecerli = match std::str::from_utf8(veri) {
        Ok(_) => return veri,
        Err(e) if e.error_len().is_none() => e.valid_up_to(),
        Err(_) => return veri,
    };
    &veri[..gecerli]
}

/// Başta kalan devam baytlarını (yarım karakterin arkası) atar.
fn son_sinirinda_kes(veri: &[u8]) -> &[u8] {
    let atla = veri
        .iter()
        .take(3)
        .take_while(|b| (**b & 0b1100_0000) == 0b1000_0000)
        .count();
    &veri[atla..]
}

/// Desen sabittir ve testler derlendiğini sınar; derlenemezse `None` döner ve
/// kaçışlar atılmaz.
fn ansi_deseni() -> Option<&'static Regex> {
    static DESEN: std::sync::OnceLock<Option<Regex>> = std::sync::OnceLock::new();
    DESEN
        .get_or_init(|| {
            Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b[0-~]").ok()
        })
        .as_ref()
}

/// Modele gidecek metin: geçersiz UTF-8 `U+FFFD` olur, ANSI kaçışları atılır.
/// Satır başı (`\r`) ile üzerine yazılan satırdan (ilerleme çubuğu) yalnızca
/// son hâli kalır.
pub fn temizle(veri: &[u8]) -> String {
    let metin = String::from_utf8_lossy(veri);
    let metin = match ansi_deseni() {
        Some(desen) => desen.replace_all(&metin, ""),
        None => metin,
    };
    let mut cikti = String::with_capacity(metin.len());
    for (i, satir) in metin.split('\n').enumerate() {
        if i > 0 {
            cikti.push('\n');
        }
        let satir = satir.strip_suffix('\r').unwrap_or(satir);
        cikti.push_str(satir.rsplit('\r').next().unwrap_or(""));
    }
    cikti
}

/// Arayüze giden canlı akış. Parçaların ortasında bölünen çok baytlı
/// karakterler bir sonraki parçaya kalır; `sinir` aşılınca bir kez bildirilir
/// ve akış durur.
pub struct CanliAkis {
    gonderici: CiktiGonderici,
    sinir: usize,
    gonderilen: usize,
    bekleyen: [Vec<u8>; 2],
    durdu: bool,
}

impl CanliAkis {
    pub fn yeni(gonderici: CiktiGonderici, sinir: usize) -> Self {
        Self {
            gonderici,
            sinir,
            gonderilen: 0,
            bekleyen: [Vec::new(), Vec::new()],
            durdu: false,
        }
    }

    pub fn ekle(&mut self, akis: Akis, veri: &[u8]) {
        if self.durdu {
            return;
        }
        if self.gonderilen >= self.sinir {
            self.durdu = true;
            self.gonderici.gonder(
                CiktiAkisi::Standart,
                format!("\n[Çıktının kalanı gösterilmiyor; model son {SON_BAYT} baytı alacak.]\n"),
            );
            return;
        }
        let (sira, cikti_akisi) = match akis {
            Akis::Standart => (0, CiktiAkisi::Standart),
            Akis::Hata => (1, CiktiAkisi::Hata),
        };
        let bekleyen = &mut self.bekleyen[sira];
        bekleyen.extend_from_slice(veri);
        let tam = bas_sinirinda_kes(bekleyen).len();
        let kalan = bekleyen.split_off(tam);
        let metin = String::from_utf8_lossy(bekleyen).into_owned();
        *bekleyen = kalan;
        if !metin.is_empty() {
            self.gonderilen += metin.len();
            self.gonderici.gonder(cikti_akisi, metin);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kucuk_cikti_kirpilmaz() {
        let mut t = BasSonTampon::default();
        t.ekle(b"merhaba\n");
        t.ekle(b"dunya");
        assert_eq!(t.metin(), "merhaba\ndunya");
    }

    #[test]
    fn buyuk_cikti_bastan_ve_sondan_kirpilir() {
        let mut t = BasSonTampon::default();
        let parca = vec![b'a'; 7_000];
        for _ in 0..10 {
            t.ekle(&parca);
        }
        t.ekle(b"SON");
        let m = t.metin();
        assert!(m.starts_with(&"a".repeat(BAS_BAYT)));
        assert!(m.ends_with("SON"));
        assert!(
            m.contains("[... 40003 bayt kırpıldı ...]"),
            "{}",
            &m[BAS_BAYT..BAS_BAYT + 60]
        );
        assert_eq!(t.toplam(), 70_003);
    }

    #[test]
    fn kirpma_cok_bayti_karakteri_bolmez() {
        let mut t = BasSonTampon::default();
        // 9_999 bayt + "ş" (2 bayt): baş sınırı karakterin ortasına düşer.
        t.ekle(&vec![b'a'; BAS_BAYT - 1]);
        t.ekle("ş".repeat(20_000).as_bytes());
        let m = t.metin();
        assert!(!m.contains('\u{FFFD}'), "yarım karakter kalmamalı");
        assert!(m.ends_with('ş'));
    }

    #[test]
    fn ansi_kacislari_ve_ilerleme_satirlari_temizlenir() {
        assert_eq!(temizle(b"\x1b[1;31mhata\x1b[0m\n"), "hata\n");
        assert_eq!(temizle(b"%10\r%50\r%100\nbitti\r\n"), "%100\nbitti\n");
        assert_eq!(temizle(b"\x1b]0;baslik\x07metin"), "metin");
        assert_eq!(temizle(b"a\x1b7b\x1bMc"), "abc");
        assert_eq!(temizle(&[0x66, 0xff, 0x67]), "f\u{FFFD}g");
    }

    #[tokio::test]
    async fn canli_akis_karakteri_bolmez_ve_sinirda_durur() {
        let (gonderici, mut rx) = kanal();
        let mut canli = CanliAkis::yeni(gonderici, 8);
        let s = "ş".as_bytes();
        canli.ekle(Akis::Standart, &[b'a', s[0]]);
        canli.ekle(Akis::Standart, &[s[1], b'b']);
        canli.ekle(Akis::Hata, b"12345678");
        canli.ekle(Akis::Standart, b"gitmez");
        canli.ekle(Akis::Standart, b"gitmez");
        let mut gelen = Vec::new();
        while let Ok(p) = rx.try_recv() {
            gelen.push((p.akis, p.metin));
        }
        assert_eq!(gelen[0], (CiktiAkisi::Standart, "a".into()));
        assert_eq!(gelen[1], (CiktiAkisi::Standart, "şb".into()));
        assert_eq!(gelen[2], (CiktiAkisi::Hata, "12345678".into()));
        assert!(gelen[3].1.contains("gösterilmiyor"));
        assert_eq!(gelen.len(), 4, "sınırdan sonra akış durur");
    }

    fn kanal() -> (
        CiktiGonderici,
        tokio::sync::mpsc::Receiver<kalfa_core::arac::CiktiParcasi>,
    ) {
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        (
            CiktiGonderici::yeni(kalfa_core::kimlik::CagriId::yeni(), tx),
            rx,
        )
    }
}
```

**`crates/kalfa-tools/src/kabuk/mod.rs`** (tam içerik):

```rust
//! `kabuk`: çalışma alanında komut çalıştırma (aşama 2 tasarımı §4).
//!
//! Aracın iki biçimi vardır: `{program, argumanlar}` kabuk yorumlayıcısı
//! olmadan çalışır; `{betik}` platformun kabuğunda. Komutu `kalfa-sandbox`'ın
//! yalıtıcısı başlatır. İzin kararı, onay kartı ve mühür hazırlıktaki
//! `KabukTarifi`'nden üretilir.

pub mod cikti;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, CiktiGonderici, Hazirlik};
use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
use kalfa_core::yol::CalismaYolu;
use kalfa_sandbox::{
    Bitis, CalistirmaIstegi, KomutBicimi, Yalitici, YalitimDurumu, ortam, program,
};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::calisma_alani::CalismaAlani;
use cikti::{BasSonTampon, CANLI_SINIR, CanliAkis};

pub const VARSAYILAN_SURE_SN: u64 = 120;
pub const AZAMI_SURE_SN: u64 = 1800;

/// Çalışma alanında kabuğun daha önce onaylanıp onaylanmadığı (aşama 2
/// tasarımı §4.2). Kayıt uygulamadadır ve yalıtımın içinde görünmez.
pub trait KabukGuveni: Send + Sync {
    fn guvenildi_mi(&self) -> bool;
    /// İlk çağrı onaylandı ve komut başlıyor.
    fn guvenildi(&self);
}

pub struct KabukAraci {
    alan: Arc<CalismaAlani>,
    yalitici: Arc<dyn Yalitici>,
    guven: Arc<dyn KabukGuveni>,
}

impl KabukAraci {
    pub fn yeni(
        alan: Arc<CalismaAlani>,
        yalitici: Arc<dyn Yalitici>,
        guven: Arc<dyn KabukGuveni>,
    ) -> Self {
        Self {
            alan,
            yalitici,
            guven,
        }
    }
}

/// Doğrulanmış argümanlar.
struct Istek {
    komut: KomutBicimi,
    /// Gösterim: onay kartı ve açıklama.
    gosterim: String,
    /// Çalışma alanına göre dizin; kök için `None`.
    dizin: Option<CalismaYolu>,
    ag: bool,
    sure: Duration,
    /// Sonucun kaynak etiketi: `kabuk:<program>`.
    kaynak: String,
}

fn metin<'a>(a: &'a Value, ad: &str) -> Option<&'a str> {
    a.get(ad).and_then(Value::as_str)
}

fn gecersiz(m: impl Into<String>) -> AracHatasi {
    AracHatasi::Gecersiz(m.into())
}

impl KabukAraci {
    fn coz(&self, a: &Value) -> Result<Istek, AracHatasi> {
        let dizin = match metin(a, "dizin") {
            None | Some("" | ".") => None,
            Some(d) => {
                let yol = CalismaYolu::coz(d)?;
                self.alan.dizin_dogrula(&yol).map_err(|e| {
                    gecersiz(format!(
                        "`{d}` çalışma alanında bir dizin değil ya da bağlantı içeriyor: {e}"
                    ))
                })?;
                Some(yol)
            }
        };
        let ag = a.get("ag").and_then(Value::as_bool).unwrap_or(false);
        let sure_sn = a
            .get("zaman_asimi_sn")
            .and_then(Value::as_u64)
            .unwrap_or(VARSAYILAN_SURE_SN)
            .clamp(1, AZAMI_SURE_SN);
        let (komut, gosterim, kaynak) = match (metin(a, "program"), metin(a, "betik")) {
            (Some(p), None) => {
                let argumanlar: Vec<String> = match a.get("argumanlar") {
                    None => Vec::new(),
                    Some(Value::Array(l)) => l
                        .iter()
                        .map(|v| {
                            v.as_str()
                                .map(str::to_string)
                                .ok_or_else(|| gecersiz("`argumanlar` yalnızca metin içerebilir."))
                        })
                        .collect::<Result<_, _>>()?,
                    Some(_) => return Err(gecersiz("`argumanlar` bir metin listesi olmalı.")),
                };
                let program = self.program_yolu(p)?;
                let gosterim = std::iter::once(p.to_string())
                    .chain(argumanlar.iter().map(|a| alintila(a)))
                    .collect::<Vec<_>>()
                    .join(" ");
                let ad = p.rsplit(['/', '\\']).next().unwrap_or(p).to_string();
                (
                    KomutBicimi::Program {
                        program,
                        argumanlar,
                    },
                    gosterim,
                    format!("kabuk:{ad}"),
                )
            }
            (None, Some(b)) => {
                if a.get("argumanlar").is_some() {
                    return Err(gecersiz(
                        "`argumanlar` yalnızca `program` ile kullanılır; betikte argümanları metne yaz.",
                    ));
                }
                if b.trim().is_empty() {
                    return Err(gecersiz("Betik boş."));
                }
                (
                    KomutBicimi::Betik {
                        metin: b.to_string(),
                    },
                    b.to_string(),
                    format!("kabuk:{}", program::yorumlayici_adi()),
                )
            }
            _ => {
                return Err(gecersiz(
                    "`program` ya da `betik`ten tam olarak biri verilmeli.",
                ));
            }
        };
        Ok(Istek {
            komut,
            gosterim,
            dizin,
            ag,
            sure: Duration::from_secs(sure_sn),
            kaynak,
        })
    }

    /// Çıplak ad olduğu gibi kalır (yalıtımın `PATH`'inde aranır); yol
    /// çalışma alanına göre çözülür. Mutlak yol reddedilir (aşama 2
    /// tasarımı §4.1).
    fn program_yolu(&self, p: &str) -> Result<String, AracHatasi> {
        if p.trim().is_empty() {
            return Err(gecersiz("Program adı boş."));
        }
        if !p.contains(['/', '\\']) {
            return Ok(p.to_string());
        }
        let yol = CalismaYolu::coz(p)?;
        Ok(self.alan.kok().join(yol.std_yolu()).display().to_string())
    }

    fn calisma_dizini(&self, dizin: &Option<CalismaYolu>) -> PathBuf {
        match dizin {
            None => self.alan.kok().to_path_buf(),
            Some(d) => self.alan.kok().join(d.std_yolu()),
        }
    }
}

/// Gösterim için argümanı tırnaklar. Yalnızca okunuş içindir; komut
/// argüman dizisi olarak çalışır.
fn alintila(a: &str) -> String {
    let guvenli = !a.is_empty()
        && a.chars()
            .all(|c| c.is_alphanumeric() || "-_./:=@%+,".contains(c));
    if guvenli {
        a.to_string()
    } else {
        format!("'{}'", a.replace('\'', r"'\''"))
    }
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

/// Açıklamadaki kısa gösterim: ilk satır, en çok 80 karakter.
fn kisa(gosterim: &str) -> String {
    let ilk = gosterim.lines().next().unwrap_or("");
    let mut kisa: String = ilk.chars().take(80).collect();
    if kisa.len() < ilk.len() || gosterim.lines().nth(1).is_some() {
        kisa.push('…');
    }
    kisa
}

#[async_trait]
impl Arac for KabukAraci {
    fn ad(&self) -> &str {
        "kabuk"
    }

    fn aciklama(&self) -> &str {
        "Çalışma alanında bir komut çalıştırır. Mümkünse `program` ve `argumanlar` ver; boru, \
         yönlendirme ya da birden çok komut gerekiyorsa `betik` ver (Linux'ta sh, Windows'ta \
         PowerShell). Ağ varsayılan olarak kapalıdır; ağ gerekiyorsa `ag: true` iste, bu her \
         kipte onay gerektirir. Standart girdi kapalıdır; etkileşim bekleyen komut çalıştırma. \
         Çıktı uzunsa baştan ve sondan kırpılır."
    }

    fn parametre_semasi(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "program": {"type": "string", "description": "Çıplak program adı (PATH'te aranır) ya da çalışma alanına göre yol"},
                "argumanlar": {"type": "array", "items": {"type": "string"}},
                "betik": {"type": "string", "description": "Kabukta çalışacak metin"},
                "dizin": {"type": "string", "description": "Çalışma alanına göre çalışma dizini; varsayılan kök"},
                "ag": {"type": "boolean", "default": false},
                "zaman_asimi_sn": {"type": "integer", "minimum": 1, "maximum": AZAMI_SURE_SN, "default": VARSAYILAN_SURE_SN}
            },
            "oneOf": [{"required": ["program"]}, {"required": ["betik"]}],
            "additionalProperties": false
        })
    }

    fn yazar_mi(&self) -> bool {
        true
    }

    async fn hazirla(&self, a: &Value) -> Result<Hazirlik, AracHatasi> {
        let istek = self.coz(a)?;
        let bicim = match istek.komut {
            KomutBicimi::Program { .. } => KabukBicimi::Program,
            KomutBicimi::Betik { .. } => KabukBicimi::Betik {
                yorumlayici: program::yorumlayici_adi().to_string(),
            },
        };
        Ok(Hazirlik {
            aciklama: format!("`{}` komutunu çalıştırmak istiyor.", kisa(&istek.gosterim)),
            tum_calisma_alani: true,
            kabuk: Some(KabukTarifi {
                komut: istek.gosterim,
                bicim,
                dizin: istek
                    .dizin
                    .as_ref()
                    .map_or_else(|| ".".to_string(), |d| d.as_str().to_string()),
                ag: istek.ag,
                yalitim: yalitim_ozeti(self.yalitici.durum()),
                ilk_cagri: !self.guven.guvenildi_mi(),
            }),
            ..Default::default()
        })
    }

    async fn calistir(
        &self,
        a: &Value,
        h: &Hazirlik,
        iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.calistir_akisli(a, h, iptal, CiktiGonderici::bos())
            .await
    }

    async fn calistir_akisli(
        &self,
        a: &Value,
        _h: &Hazirlik,
        iptal: CancellationToken,
        gonderici: CiktiGonderici,
    ) -> Result<AracCiktisi, AracHatasi> {
        if iptal.is_cancelled() {
            return Err(AracHatasi::Iptal);
        }
        let istek = self.coz(a)?;
        let calistirma = CalistirmaIstegi {
            komut: istek.komut,
            dizin: self.calisma_dizini(&istek.dizin),
            ortam: ortam::uret(std::env::vars()),
            ag: istek.ag,
        };
        let surec = self
            .yalitici
            .baslat(&calistirma)
            .map_err(|e| gecersiz(e.to_string()))?;
        // Komut başladı; çalışma alanındaki ilk onay kaydedilir.
        self.guven.guvenildi();
        let mut tampon = BasSonTampon::default();
        let mut canli = CanliAkis::yeni(gonderici, CANLI_SINIR);
        let sonuc = surec
            .bekle(istek.sure, iptal, |akis, veri| {
                tampon.ekle(veri);
                canli.ekle(akis, veri);
            })
            .await;
        let durum = match sonuc.bitis {
            Bitis::Iptal => return Err(AracHatasi::Iptal),
            Bitis::Cikti(Some(kod)) => format!("Çıkış kodu: {kod}"),
            Bitis::Cikti(None) => "Komut bir sinyalle sonlandı.".to_string(),
            Bitis::ZamanAsimi => format!(
                "Zaman aşımı ({} sn); süreç ağacı durduruldu.",
                istek.sure.as_secs()
            ),
        };
        let sure = format!("{:.1}", sonuc.sure.as_secs_f64()).replace('.', ",");
        let govde = if tampon.toplam() == 0 {
            "--- çıktı yok ---".to_string()
        } else {
            format!("--- çıktı ---\n{}", tampon.metin())
        };
        let cikti = AracCiktisi {
            icerik: format!("{durum}\nSüre: {sure} sn\n{govde}"),
            kaynak: istek.kaynak,
        };
        if sonuc.bitis == Bitis::Cikti(Some(0)) {
            Ok(cikti)
        } else {
            Err(AracHatasi::Basarisiz(cikti))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gosterimde_ozel_karakterli_arguman_tirnaklanir() {
        assert_eq!(alintila("test"), "test");
        assert_eq!(alintila("--features=e2e"), "--features=e2e");
        assert_eq!(alintila("iki kelime"), "'iki kelime'");
        assert_eq!(alintila(""), "''");
        assert_eq!(alintila("a'b"), r"'a'\''b'");
        assert_eq!(alintila("$(rm)"), "'$(rm)'");
    }

    #[test]
    fn aciklama_ilk_satiri_kisaltir() {
        assert_eq!(kisa("cargo test"), "cargo test");
        assert_eq!(kisa("echo a\necho b"), "echo a…");
        assert_eq!(kisa(&"x".repeat(100)), format!("{}…", "x".repeat(80)));
    }
}
```

**`crates/kalfa-tools/src/lib.rs`**: şu bloğu

```rust
pub mod diff;
pub mod duzenle;
pub mod listele;
pub mod oku;
```

şununla değiştir:

```rust
pub mod diff;
pub mod duzenle;
pub mod kabuk;
pub mod listele;
pub mod oku;
```

**`crates/kalfa-tools/src/lib.rs`**: şu bloğu

```rust
pub use calisma_alani::CalismaAlani;
pub use duzenle::DosyaDuzenle;
pub use listele::DosyaListele;
pub use oku::DosyaOku;
```

şununla değiştir:

```rust
pub use calisma_alani::CalismaAlani;
pub use duzenle::DosyaDuzenle;
pub use kabuk::{KabukAraci, KabukGuveni};
pub use listele::DosyaListele;
pub use oku::DosyaOku;
```

**`crates/kalfa-tools/src/lib.rs`**: şu bloğu

```rust
use kalfa_core::arac::Arac;

/// Yerleşik araçların tamamı (spec §12.1). Kabuk aracı daha sonraki aşamada
/// eklenir.
pub fn varsayilan_araclar(alan: Arc<CalismaAlani>) -> Vec<Arc<dyn Arac>> {
    vec![
```

şununla değiştir:

```rust
use kalfa_core::arac::Arac;

/// Yerleşik dosya araçları (spec §12.1). Kabuk aracı ayrıca kurulur
/// (`KabukAraci::yeni`): yalıtıcı ve çalışma alanının güven kaydını ister.
pub fn varsayilan_araclar(alan: Arc<CalismaAlani>) -> Vec<Arc<dyn Arac>> {
    vec![
```

- [ ] **Step 4: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa-tools --locked`
Expected: `tests/kabuk.rs`'in 9 testi ve `kabuk::cikti` ile `kabuk` birim testleri geçer. `calisma_dizini_baglanti_olamaz` Windows'ta sembolik bağlantı izni ister (mevcut bağlantı testleri gibi). Görevin sonundaki genel denetimleri de koştur.

- [ ] **Step 5: Commit**

```bash
git add crates/kalfa-tools
git commit -m "araçlar: kabuk aracı"
```

---

### Task 4: Uygulama: kabuğun bağlanması, yerel onay penceresi, çalışma alanı kaydı

**Files:**
- Create: `crates/kalfa-uygulama/src/calisma_alanlari.rs`, `crates/kalfa-uygulama/tests/kabuk.rs`
- Modify: `crates/kalfa-uygulama/src/{lib,uygulama,oturumlar,sozlesme,guvenlik}.rs`, `crates/kalfa-uygulama/src/e2e/mod.rs`

**Interfaces:**
- Consumes: Görev 1 (`Yalitimsiz`, `YalitimDurumu`), Görev 2 (`OturumTutamaci::bekleyen_onay`, `Komut::YerelOnay`, `OnayOgesi.kabuk/yerel_onay`, `YalitimOzeti`), Görev 3 (`KabukAraci`, `KabukGuveni`).
- Produces:
  - `calisma_alanlari::CalismaAlaniKaydi::{yeni(&Path), kimlik(&Path) -> String, kabuk_onaylandi_mi(&str), kabuk_onayla(&str)}`.
  - `UygulamaDurumu.yalitim: YalitimDurumu`.
  - `guvenlik::yerel_onay_metni(&[OnayOgesi]) -> String`.
  - `Uygulama::arac_onayi`'nın yeni davranışı (aşağıda).

**Davranış notları (testler sınar):**
- **Bağlam:** her oturumun araçlarına `KabukAraci` eklenir; yalıtıcı `Yalitimsiz`, güven kaydı çalışma alanının kimliğiyle `AlanGuveni`'dir.
- **Çalışma alanı kaydı:** veri dizinindeki `calisma_alanlari.json`. Kimlik, kanonik yolun SHA-256'sından 16 onaltılık karakterdir. Bozuk ya da okunamayan kayıt boş sayılır; yazma geçici dosya ve yeniden adlandırmayla yapılır.
- **`arac_onayi`:** onaylanan (`Onayla`) her öğe için `bekleyen_onay`'a bakılır. Yerel onaylı öğeler varsa tek bir yerel iletişim kutusu açılır: başlık "Komutu yalıtımsız çalıştır", metin `yerel_onay_metni`, düğmeler "Çalıştır" ve "Vazgeç".
  - Evet: yerel öğeler için `Komut::YerelOnay`.
  - Hayır: yerel öğeler "Kullanıcı yerel pencerede vazgeçti." gerekçesiyle reddedilir.
  - Diğer öğeler arayüzün kararıyla gider.
- **Kip açıklamaları:** yükseltme onayındaki metinler kabuğu anlatır (Sıkı, Proje, Serbest).

- [ ] **Step 1: Başarısız testleri yaz**

**`crates/kalfa-uygulama/tests/kabuk.rs`** (tam içerik):

```rust
//! Kabuk uygulamada (aşama 2 tasarımı §4.2, §5.2, §10.3): yalıtımsız komut
//! yalnızca yerel pencerede onaylanır; ilk çağrı çalışma alanına kaydedilir.

use std::time::Duration;

use kalfa_core::aktarim::KanalMesaji;
use kalfa_core::izin::{ILK_KABUK_UYARISI, Kip, YALITIMSIZ_UYARISI};
use kalfa_core::kimlik::OturumId;
use kalfa_core::protokol::{AracDurumu, Olay, OnayKarari, OnayOgesi};
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::{SahteSaglayici, SahteYanit};
use kalfa_sandbox::YalitimDurumu;
use kalfa_uygulama::sozlesme::OnayGirdisi;
use kalfa_uygulama::test_destek::{SahteKanal, TestUygulamasi, test_uygulamasi};
use serde_json::json;
use tempfile::TempDir;
use tokio::sync::mpsc::UnboundedReceiver;

struct Ortam {
    _veri: TempDir,
    _alan: TempDir,
    t: TestUygulamasi,
}

fn echo(metin: &str) -> serde_json::Value {
    if cfg!(windows) {
        json!({"program": "cmd", "argumanlar": ["/c", format!("echo {metin}")]})
    } else {
        json!({"program": "sh", "argumanlar": ["-c", format!("echo {metin}")]})
    }
}

async fn ortam(senaryo: Vec<SahteYanit>) -> Ortam {
    let veri = tempfile::tempdir().unwrap();
    let alan = tempfile::tempdir().unwrap();
    let saglayici = SahteSaglayici::yeni(senaryo);
    saglayici.sart_kabul(1).await.unwrap();
    let t = test_uygulamasi(veri.path(), saglayici, Some("evren_llm_a"));
    t.uygulama.baslat().await;
    t.iletisim.klasor_ver(alan.path());
    Ortam {
        _veri: veri,
        _alan: alan,
        t,
    }
}

impl Ortam {
    async fn yeni_oturum(&self) -> (OturumId, UnboundedReceiver<KanalMesaji>) {
        let alan = self.t.uygulama.calisma_alani_sec().await.unwrap();
        let oturum = self
            .t
            .uygulama
            .oturum_baslat(alan.kimlik, "glm-5.3".into())
            .await
            .unwrap()
            .oturum;
        let (kanal, rx) = SahteKanal::yeni();
        self.t.uygulama.oturumu_dinle(oturum, kanal).unwrap();
        (oturum, rx)
    }

    async fn bekle(
        &self,
        oturum: OturumId,
        rx: &mut UnboundedReceiver<KanalMesaji>,
        kosul: impl Fn(&Olay) -> bool,
    ) -> Vec<Olay> {
        let mut gorulen = Vec::new();
        loop {
            let mesaj = tokio::time::timeout(Duration::from_secs(20), rx.recv())
                .await
                .expect("olay gelmedi")
                .expect("kanal kapandı");
            let KanalMesaji::Olaylar { no, zarflar } = mesaj else {
                panic!("kanal kapandı");
            };
            self.t.uygulama.paketi_isledim(oturum, no).unwrap();
            for z in zarflar {
                let bulundu = kosul(&z.yuk);
                gorulen.push(z.yuk);
                if bulundu {
                    return gorulen;
                }
            }
        }
    }

    async fn onay_ogeleri(
        &self,
        oturum: OturumId,
        rx: &mut UnboundedReceiver<KanalMesaji>,
    ) -> Vec<OnayOgesi> {
        match self
            .bekle(oturum, rx, |o| matches!(o, Olay::OnayGerekiyor { .. }))
            .await
            .pop()
        {
            Some(Olay::OnayGerekiyor { ogeler, .. }) => ogeler,
            _ => unreachable!(),
        }
    }
}

fn onayla(o: &OnayOgesi) -> OnayGirdisi {
    OnayGirdisi {
        cagri: o.cagri,
        muhur: o.muhur.clone(),
        karar: OnayKarari::Onayla,
    }
}

fn sonuc(olaylar: &[Olay]) -> (AracDurumu, String) {
    olaylar
        .iter()
        .find_map(|o| match o {
            Olay::AracSonucu { durum, icerik, .. } => Some((*durum, icerik.clone())),
            _ => None,
        })
        .expect("araç sonucu")
}

#[tokio::test]
async fn yalitim_durumu_uygulama_durumunda_gorunur() {
    let o = ortam(Vec::new()).await;
    assert!(matches!(
        o.t.uygulama.uygulama_durumu().yalitim,
        YalitimDurumu::Yok { .. }
    ));
}

#[tokio::test]
async fn yalitimsiz_komut_yerel_pencerede_onaylaninca_calisir() {
    let o = ortam(vec![
        SahteYanit::arac("kabuk", echo("kabuktan-merhaba")),
        SahteYanit::metin("tamam"),
        SahteYanit::arac("kabuk", echo("ikinci")),
        SahteYanit::metin("bitti"),
    ])
    .await;
    let (oturum, mut rx) = o.yeni_oturum().await;
    // Serbest kipte bile yalıtımsız komut onay ister.
    o.t.iletisim.onay_ver(true);
    o.t.uygulama
        .izin_kipi_degistir(oturum, Kip::Serbest)
        .await
        .unwrap();
    o.t.uygulama
        .mesaj_gonder(oturum, "çalıştır".into())
        .await
        .unwrap();
    let ogeler = o.onay_ogeleri(oturum, &mut rx).await;
    assert!(ogeler[0].yerel_onay);
    assert!(ogeler[0].uyarilar.contains(&YALITIMSIZ_UYARISI.to_string()));
    assert!(ogeler[0].uyarilar.contains(&ILK_KABUK_UYARISI.to_string()));

    o.t.uygulama
        .arac_onayi(oturum, vec![onayla(&ogeler[0])])
        .await
        .unwrap();
    let sorulan = o.t.iletisim.sorulanlar().pop().unwrap();
    assert!(
        sorulan.starts_with("Komutu yalıtımsız çalıştır: "),
        "{sorulan}"
    );
    assert!(
        sorulan.contains("echo kabuktan-merhaba"),
        "komut çekirdekten okunur: {sorulan}"
    );

    let olaylar = o
        .bekle(oturum, &mut rx, |o| matches!(o, Olay::Tamamlandi { .. }))
        .await;
    let (durum, icerik) = sonuc(&olaylar);
    assert_eq!(durum, AracDurumu::Basarili, "{icerik}");
    assert!(icerik.contains("kabuktan-merhaba"));
    assert!(
        olaylar
            .iter()
            .any(|o| matches!(o, Olay::AracCiktisiParcasi { metin, .. } if metin.contains("kabuktan-merhaba"))),
        "çıktı canlı akar"
    );

    // Aynı çalışma alanındaki sonraki çağrıda ilk çağrı uyarısı yoktur;
    // yalıtımsız olduğu için yine yerel onay ister.
    o.t.uygulama
        .mesaj_gonder(oturum, "yine".into())
        .await
        .unwrap();
    let ogeler = o.onay_ogeleri(oturum, &mut rx).await;
    assert!(ogeler[0].yerel_onay);
    assert!(!ogeler[0].uyarilar.contains(&ILK_KABUK_UYARISI.to_string()));
}

#[tokio::test]
async fn yerel_pencerede_vazgecilirse_komut_calismaz() {
    let o = ortam(vec![
        SahteYanit::arac("kabuk", echo("calismamali")),
        SahteYanit::metin("peki"),
    ])
    .await;
    let (oturum, mut rx) = o.yeni_oturum().await;
    o.t.uygulama
        .mesaj_gonder(oturum, "çalıştır".into())
        .await
        .unwrap();
    let ogeler = o.onay_ogeleri(oturum, &mut rx).await;
    o.t.iletisim.onay_ver(false);
    o.t.uygulama
        .arac_onayi(oturum, vec![onayla(&ogeler[0])])
        .await
        .unwrap();
    let olaylar = o
        .bekle(oturum, &mut rx, |o| matches!(o, Olay::Tamamlandi { .. }))
        .await;
    let (durum, icerik) = sonuc(&olaylar);
    assert_eq!(durum, AracDurumu::Reddedildi);
    assert!(icerik.contains("yerel pencerede vazgeçti"), "{icerik}");
}
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa-uygulama --locked --test kabuk`
Expected: derleme hatası (`UygulamaDurumu.yalitim` yok).

- [ ] **Step 3: Kaydı ve bağlantıyı yaz**

**`crates/kalfa-uygulama/src/calisma_alanlari.rs`** (tam içerik):

```rust
//! Çalışma alanlarının kaydı (aşama 2 tasarımı §4.2): kabuğun ilk çağrısı
//! onaylanmış çalışma alanları. Kayıt uygulamanın veri dizinindedir; kabuk
//! komutları onu göremez ve değiştiremez.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use kalfa_core::muhur::ozet_hex;
use kalfa_tools::KabukGuveni;
use serde::{Deserialize, Serialize};

const DOSYA_ADI: &str = "calisma_alanlari.json";

#[derive(Debug, Default, Serialize, Deserialize)]
struct Dosya {
    #[serde(default)]
    kabuk_onaylanan: BTreeSet<String>,
}

pub struct CalismaAlaniKaydi {
    yol: PathBuf,
    /// İlk okumada dosyadan yüklenir.
    icerik: Mutex<Option<Dosya>>,
}

impl CalismaAlaniKaydi {
    pub fn yeni(veri_dizini: &Path) -> Self {
        Self {
            yol: veri_dizini.join(DOSYA_ADI),
            icerik: Mutex::new(None),
        }
    }

    /// Çalışma alanının kimliği: kanonik yolunun SHA-256'sından 16 onaltılık
    /// karakter.
    pub fn kimlik(kok: &Path) -> String {
        let kanonik = std::fs::canonicalize(kok).unwrap_or_else(|_| kok.to_path_buf());
        ozet_hex(kanonik.to_string_lossy().as_bytes())[..16].to_string()
    }

    fn ile<T>(&self, is: impl FnOnce(&mut Dosya) -> T) -> T {
        let mut icerik = self.icerik.lock().unwrap_or_else(|z| z.into_inner());
        let dosya = icerik.get_or_insert_with(|| {
            // Okunamayan ya da bozuk kayıt boş sayılır: en kötü durumda ilk
            // çağrı uyarısı yeniden gösterilir.
            std::fs::read(&self.yol)
                .ok()
                .and_then(|v| serde_json::from_slice(&v).ok())
                .unwrap_or_default()
        });
        is(dosya)
    }

    pub fn kabuk_onaylandi_mi(&self, kimlik: &str) -> bool {
        self.ile(|d| d.kabuk_onaylanan.contains(kimlik))
    }

    /// Kaydı yazar. Yazılamazsa kayıt bellekte kalır; sonraki açılışta ilk
    /// çağrı uyarısı yeniden gösterilir.
    pub fn kabuk_onayla(&self, kimlik: &str) {
        let metin = self.ile(|d| {
            d.kabuk_onaylanan.insert(kimlik.to_string());
            serde_json::to_vec_pretty(d)
        });
        let Ok(metin) = metin else { return };
        if let Some(ust) = self.yol.parent() {
            let _ = std::fs::create_dir_all(ust);
        }
        let gecici = self.yol.with_extension("json.yeni");
        if std::fs::write(&gecici, metin).is_ok() {
            let _ = std::fs::rename(&gecici, &self.yol);
        }
    }
}

/// Bir çalışma alanının kabuk güveni; kabuk aracına verilir.
pub(crate) struct AlanGuveni {
    pub kayit: Arc<CalismaAlaniKaydi>,
    pub kimlik: String,
}

impl KabukGuveni for AlanGuveni {
    fn guvenildi_mi(&self) -> bool {
        self.kayit.kabuk_onaylandi_mi(&self.kimlik)
    }

    fn guvenildi(&self) {
        if !self.guvenildi_mi() {
            self.kayit.kabuk_onayla(&self.kimlik);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn onay_kalici_ve_calisma_alanina_ozeldir() {
        let veri = tempfile::tempdir().unwrap();
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let (ka, kb) = (
            CalismaAlaniKaydi::kimlik(a.path()),
            CalismaAlaniKaydi::kimlik(b.path()),
        );
        assert_ne!(ka, kb);
        assert_eq!(ka.len(), 16);

        let kayit = CalismaAlaniKaydi::yeni(veri.path());
        assert!(!kayit.kabuk_onaylandi_mi(&ka));
        kayit.kabuk_onayla(&ka);
        assert!(kayit.kabuk_onaylandi_mi(&ka));
        assert!(!kayit.kabuk_onaylandi_mi(&kb));

        let yeniden = CalismaAlaniKaydi::yeni(veri.path());
        assert!(
            yeniden.kabuk_onaylandi_mi(&ka),
            "kayıt yeniden açılışta okunur"
        );
    }

    #[test]
    fn bozuk_kayit_bos_sayilir() {
        let veri = tempfile::tempdir().unwrap();
        std::fs::write(veri.path().join(DOSYA_ADI), "{bozuk").unwrap();
        let kayit = CalismaAlaniKaydi::yeni(veri.path());
        assert!(!kayit.kabuk_onaylandi_mi("abc"));
        kayit.kabuk_onayla("abc");
        assert!(CalismaAlaniKaydi::yeni(veri.path()).kabuk_onaylandi_mi("abc"));
    }
}
```

**`crates/kalfa-uygulama/src/lib.rs`**: şu bloğu

```rust
pub mod anahtar;
pub mod arayuz;
pub mod dizin;
pub mod evren;
```

şununla değiştir:

```rust
pub mod anahtar;
pub mod arayuz;
pub mod calisma_alanlari;
pub mod dizin;
pub mod evren;
```

**`crates/kalfa-uygulama/src/uygulama.rs`**: şu bloğu

```rust
use kalfa_core::oturum::OturumTutamaci;
use kalfa_core::sart_kapisi::SartKapisi;

use crate::anahtar::AnahtarKasasi;
use crate::arayuz::{Acici, Iletisim, Pencereler};
use crate::dizin;
use crate::evren::{DegisirSaglayici, SaglayiciUretici};
```

şununla değiştir:

```rust
use kalfa_core::oturum::OturumTutamaci;
use kalfa_core::sart_kapisi::SartKapisi;
use kalfa_sandbox::{Yalitici, Yalitimsiz};

use crate::anahtar::AnahtarKasasi;
use crate::arayuz::{Acici, Iletisim, Pencereler};
use crate::calisma_alanlari::CalismaAlaniKaydi;
use crate::dizin;
use crate::evren::{DegisirSaglayici, SaglayiciUretici};
```

**`crates/kalfa-uygulama/src/uygulama.rs`**: şu bloğu

```rust
    pub(crate) pencereler: Arc<dyn Pencereler>,
    pub(crate) aktarim: AktarimAyarlari,
    ic: Mutex<IcDurum>,
}
```

şununla değiştir:

```rust
    pub(crate) pencereler: Arc<dyn Pencereler>,
    pub(crate) aktarim: AktarimAyarlari,
    /// Kabuk komutlarını başlatan arka uç. Aşama 2a'da yalıtım yoktur.
    pub(crate) yalitici: Arc<dyn Yalitici>,
    pub(crate) alanlar: Arc<CalismaAlaniKaydi>,
    ic: Mutex<IcDurum>,
}
```

**`crates/kalfa-uygulama/src/uygulama.rs`**: şu bloğu

```rust
            pencereler: b.pencereler,
            aktarim: b.aktarim,
            ic: Mutex::new(IcDurum {
                baglanti: BaglantiDurumu::Var,
```

şununla değiştir:

```rust
            pencereler: b.pencereler,
            aktarim: b.aktarim,
            yalitici: Arc::new(Yalitimsiz),
            alanlar: Arc::new(CalismaAlaniKaydi::yeni(&b.veri_dizini)),
            ic: Mutex::new(IcDurum {
                baglanti: BaglantiDurumu::Var,
```

**`crates/kalfa-uygulama/src/sozlesme.rs`**: şu bloğu

```rust
use kalfa_core::protokol::OnayKarari;
use kalfa_core::sart_kapisi::SartMetni;
use serde::{Deserialize, Serialize};
```

şununla değiştir:

```rust
use kalfa_core::protokol::OnayKarari;
use kalfa_core::sart_kapisi::SartMetni;
use kalfa_sandbox::YalitimDurumu;
use serde::{Deserialize, Serialize};
```

**`crates/kalfa-uygulama/src/sozlesme.rs`**: şu bloğu

```rust
    /// yüklenen ana pencere onu kayıttan açar (1c tasarımı §5.6).
    pub son_oturum: Option<OturumId>,
}
```

şununla değiştir:

```rust
    /// yüklenen ana pencere onu kayıttan açar (1c tasarımı §5.6).
    pub son_oturum: Option<OturumId>,
    /// Kabuk komutlarının yalıtımı (aşama 2 tasarımı §10.3).
    pub yalitim: YalitimDurumu,
}
```

**`crates/kalfa-uygulama/src/oturumlar.rs`**: şu bloğu

```rust
use kalfa_core::kurtarma::kurtar;
use kalfa_core::oturum::{self, OlayAlici, OturumAyarlari, OturumBaglami, OturumTutamaci};
use kalfa_core::protokol::Komut;
use kalfa_tools::{CalismaAlani as AracAlani, varsayilan_araclar};
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinHandle;

use crate::arayuz::KanalYazici;
use crate::hata::KomutHatasi;
use crate::sozlesme::{
```

şununla değiştir:

```rust
use kalfa_core::kurtarma::kurtar;
use kalfa_core::oturum::{self, OlayAlici, OturumAyarlari, OturumBaglami, OturumTutamaci};
use kalfa_core::protokol::{Komut, OnayKarari, OnayOgesi};
use kalfa_tools::{CalismaAlani as AracAlani, KabukAraci, varsayilan_araclar};
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinHandle;

use crate::arayuz::KanalYazici;
use crate::calisma_alanlari::{AlanGuveni, CalismaAlaniKaydi};
use crate::hata::KomutHatasi;
use crate::sozlesme::{
```

**`crates/kalfa-uygulama/src/oturumlar.rs`**: şu bloğu

```rust
            varsayilan_model: modeller::VARSAYILAN_MODEL.to_string(),
            son_oturum: ic.son_oturum,
        }
    }
```

şununla değiştir:

```rust
            varsayilan_model: modeller::VARSAYILAN_MODEL.to_string(),
            son_oturum: ic.son_oturum,
            yalitim: self.yalitici.durum(),
        }
    }
```

**`crates/kalfa-uygulama/src/oturumlar.rs`**: şu bloğu

```rust
    }

    pub async fn arac_onayi(
        &self,
```

şununla değiştir:

```rust
    }

    /// Onay kararlarını çekirdeğe iletir. Yerel onay isteyen bir öğeyi
    /// arayüz onayladıysa Rust komutu yerel iletişim kutusunda yeniden sorar
    /// (aşama 2 tasarımı §5.2). Kutuda gösterilen komut arayüzden değil
    /// çekirdekten okunur; webview'a sızmış bir betik başka bir komutu
    /// onaylatamaz.
    pub async fn arac_onayi(
        &self,
```

**`crates/kalfa-uygulama/src/oturumlar.rs`**: şu bloğu

```rust
    ) -> Result<(), KomutHatasi> {
        let tutamac = self.tutamac(oturum)?;
        for k in kararlar {
            tutamac
                .gonder(Komut::AracOnayi {
                    cagri: k.cagri,
                    karar: k.karar,
                    muhur: k.muhur,
                })
                .await?;
        }
        Ok(())
```

şununla değiştir:

```rust
    ) -> Result<(), KomutHatasi> {
        let tutamac = self.tutamac(oturum)?;
        let yereller: Vec<OnayOgesi> = kararlar
            .iter()
            .filter(|k| k.karar == OnayKarari::Onayla)
            .filter_map(|k| tutamac.bekleyen_onay(k.cagri))
            .filter(|o| o.yerel_onay)
            .collect();
        let yerel_evet = !yereller.is_empty()
            && self
                .iletisim
                .onayla(
                    "Komutu yalıtımsız çalıştır",
                    &guvenlik::yerel_onay_metni(&yereller),
                    "Çalıştır",
                    "Vazgeç",
                )
                .await;
        for k in kararlar {
            let komut = match yereller.iter().find(|o| o.cagri == k.cagri) {
                Some(o) if yerel_evet => Komut::YerelOnay {
                    cagri: o.cagri,
                    muhur: o.muhur.clone(),
                },
                Some(o) => Komut::AracOnayi {
                    cagri: o.cagri,
                    karar: OnayKarari::Reddet {
                        gerekce: Some("Kullanıcı yerel pencerede vazgeçti.".into()),
                    },
                    muhur: o.muhur.clone(),
                },
                None => Komut::AracOnayi {
                    cagri: k.cagri,
                    karar: k.karar,
                    muhur: k.muhur,
                },
            };
            tutamac.gonder(komut).await?;
        }
        Ok(())
```

**`crates/kalfa-uygulama/src/oturumlar.rs`**: şu bloğu

```rust
        let siniflandirici = Arc::new(Siniflandirici::yeni(kok, &[]).map_err(ic_hata)?);
        let alan = AracAlani::ac(siniflandirici.clone())?;
        let araclar = Arc::new(AracKayitDefteri::yeni(varsayilan_araclar(alan)).map_err(ic_hata)?);
        Ok(OturumBaglami {
            saglayici: self.evren.clone(),
```

şununla değiştir:

```rust
        let siniflandirici = Arc::new(Siniflandirici::yeni(kok, &[]).map_err(ic_hata)?);
        let alan = AracAlani::ac(siniflandirici.clone())?;
        let guven = Arc::new(AlanGuveni {
            kayit: self.alanlar.clone(),
            kimlik: CalismaAlaniKaydi::kimlik(kok),
        });
        let mut araclar = varsayilan_araclar(alan.clone());
        araclar.push(Arc::new(KabukAraci::yeni(
            alan,
            self.yalitici.clone(),
            guven,
        )));
        let araclar = Arc::new(AracKayitDefteri::yeni(araclar).map_err(ic_hata)?);
        Ok(OturumBaglami {
            saglayici: self.evren.clone(),
```

**`crates/kalfa-uygulama/src/guvenlik.rs`**: şu bloğu

```rust

use kalfa_core::izin::Kip;
use url::Url;
```

şununla değiştir:

```rust

use kalfa_core::izin::Kip;
use kalfa_core::kabuk::YalitimOzeti;
use kalfa_core::protokol::OnayOgesi;
use url::Url;
```

**`crates/kalfa-uygulama/src/guvenlik.rs`**: şu bloğu

```rust
}

/// Yükseltme onayında hedef kipin ne getirdiği (üst belge §7.5). 1c'de kabuk
/// aracı yok; Proje ile Serbest dosya yazmada aynı davranır.
pub fn yukseltme_aciklamasi(yeni: Kip) -> &'static str {
    match yeni {
        Kip::Inceleme => "",
        Kip::Siki => {
            "Kalfa dosya yazmayı ve düzenlemeyi önerebilecek; her yazma onayınıza sunulur."
        }
        Kip::Proje => {
            "Kalfa çalışma alanı içindeki dosyaları onay sormadan yazabilecek. Korumalı yollar, \
             gizli dosyalar ve çalışma alanı dışındaki yollar yine onay ister."
        }
        Kip::Serbest => {
            "Kalfa çalışma alanı içindeki dosyaları onay sormadan yazabilecek. Serbest kip yalnızca \
             bu oturum boyunca geçerlidir. Korumalı yollar, gizli dosyalar ve çalışma alanı dışındaki \
             yollar yine onay ister."
        }
    }
}
```

şununla değiştir:

```rust
}

/// Yükseltme onayında hedef kipin ne getirdiği (üst belge §7.5, aşama 2
/// tasarımı §5.1). Yalıtımsız komut her kipte yerel pencerede onay ister.
pub fn yukseltme_aciklamasi(yeni: Kip) -> &'static str {
    match yeni {
        Kip::Inceleme => "",
        Kip::Siki => {
            "Kalfa dosya yazmayı, düzenlemeyi ve komut çalıştırmayı önerebilecek; her biri onayınıza \
             sunulur."
        }
        Kip::Proje => {
            "Kalfa çalışma alanı içindeki dosyaları onay sormadan yazabilecek. Korumalı yollar, \
             gizli dosyalar, çalışma alanı dışındaki yollar ve komutlar yine onay ister."
        }
        Kip::Serbest => {
            "Kalfa çalışma alanı içindeki dosyaları onay sormadan yazabilecek; yalıtım varsa \
             yalıtımlı ve ağsız komutları da onay sormadan çalıştırabilecek. Serbest kip yalnızca bu \
             oturum boyunca geçerlidir. Korumalı yollar, gizli dosyalar, çalışma alanı dışındaki \
             yollar, ağlı ve yalıtımsız komutlar yine onay ister."
        }
    }
}

/// Yalıtımsız komutun yerel onay metni (aşama 2 tasarımı §5.2). Komut
/// çekirdeğin bekleyen onayından gelir.
pub fn yerel_onay_metni(ogeler: &[OnayOgesi]) -> String {
    let mut metin = String::from(
        "Aşağıdaki komut yalıtımsız çalışacak: senin bütün yetkilerinle çalışır, ev dizinindeki \
         dosyaları okuyabilir ve ağa erişebilir.\n",
    );
    let mut talimat = None;
    for o in ogeler {
        let Some(k) = &o.kabuk else { continue };
        metin.push_str(&format!("\nKomut: {}\nDizin: {}\n", k.komut, k.dizin));
        if k.ag {
            metin.push_str("Model ağ erişimi istedi.\n");
        }
        if let YalitimOzeti::Yalitimsiz { talimat: t } = &k.yalitim {
            talimat.get_or_insert(t.clone());
        }
    }
    if let Some(t) = talimat {
        metin.push_str(&format!("\n{t}"));
    }
    metin
}
```

**`crates/kalfa-uygulama/src/e2e/mod.rs`**: şu bloğu

```rust
    #[serde(default)]
    pub calisma_alani: Option<PathBuf>,
    /// Yerel onay kutularının cevabı (izin kipi yükseltme, bağlantı açma).
    #[serde(default)]
    pub onay: bool,
```

şununla değiştir:

```rust
    #[serde(default)]
    pub calisma_alani: Option<PathBuf>,
    /// Yerel onay kutularının cevabı (izin kipi yükseltme, bağlantı açma,
    /// yalıtımsız komut).
    #[serde(default)]
    pub onay: bool,
```

- [ ] **Step 4: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa-uygulama --locked`
Expected: `tests/kabuk.rs`'in 3 testi ve `calisma_alanlari` birim testleri dahil bütün testler geçer. Görevin sonundaki genel denetimleri (`e2e` özelliği dahil) de koştur.

- [ ] **Step 5: Commit**

```bash
git add crates/kalfa-uygulama
git commit -m "uygulama: kabuk aracı, yerel onay penceresi ve çalışma alanı kaydı"
```

---

### Task 5: Arayüz: terminal görünümü, kabuk onay ayrıntısı, yalıtım göstergesi

**Files:**
- Create: `ui/src/durum/terminal.ts`, `ui/src/durum/terminal.test.ts`, `ui/src/ana/ansi.ts`, `ui/src/ana/ansi.test.ts`, `ui/src/ana/TerminalGorunumu.svelte`
- Modify: `ui/src/ipc/tipler.ts`, `ui/src/sahte/arka.ts`, `ui/src/bilesenler/ikonlar.ts`, `ui/src/dil/tr.ts`, `ui/src/durum/oturum.ts`, `ui/src/ana/{bicim.ts, onay.ts, KonusmaOgesi.svelte, OnaySayfasi.svelte, DurumCubugu.svelte}`, `ui/src/onizleme/senaryo.ts`
- Test: `ui/src/durum/oturum.test.ts`, `ui/src/ana/bicim.test.ts`, `ui/src/ana/ekranlar.test.ts` (değişir)

**Interfaces:**
- Consumes: Görev 2 ve 4'ün JSON biçimleri (`AracCiktisiParcasi`, `OnayOgesi.kabuk/yerel_onay`, `UygulamaDurumu.yalitim`).
- Produces:
  - `durum/terminal.ts`: `Terminal { satirlar, acik, karakter, atilan_satir, gosterilmeyen_bayt }`, `TerminalSatiri { akis, metin, kesik? }`, `BOS_TERMINAL`, `terminalEkle(t, akis, metin, atlananBayt)`, `AZAMI_SATIR = 5000`, `AZAMI_KARAKTER = 200_000`, `SATIR_SINIRI = 4000`.
  - `ana/ansi.ts`: `ansiParcala(satir) -> { metin, renk, kalin }[]`.
  - `EkranOgesi` (`AracCagrisi`) `terminal: Terminal | null` alanı; `bicim.kabukOzeti`.

**Davranış notları (testler sınar):**
- **Terminal durumu:**
  - Satır sonuyla bitmeyen parça aynı akışın sonraki parçasıyla birleşir; farklı akış yeni satır açar.
  - Satır başı (`\r`) satırın üzerine yazar.
  - `atlanan_bayt` bir "… N bayt gösterilmedi …" bilgi satırı ekler.
  - En çok 5 000 satır ve 200 000 karakter tutulur, fazlası baştan atılır ve sayılır; tek satır 4 000 karakterde kesilir ve devamı eklenmez.
  - Neden karakter sınırı: çıktı selinde Linux'taki webview (WebKitGTK, Xvfb) terminalde megabaytlarca metin varken saniyede 50–60 KB işleyebildi ve tur bitmedi (tasarım §10.1).
- **ANSI:** yalnızca SGR çizilir (kırmızı, yeşil, sarı, mavi, soluk; kalın); CSI, OSC ve tek karakterli kaçışlar (`ESC 7`) atılır. Metin olduğu gibi kalır; HTML olarak yorumlanmaz.
- **Kart:** kabuk kartının başlığı "Komut çalışıyor" / "Komut çalıştı" / "Komut", özeti sonucun ilk satırıdır ("Çıkış kodu: 101"), gövdesi terminaldir. Kayıttan gelen çağrıda terminal yoktur; sonuç eskisi gibi gösterilir.
- **Onay sayfası:** kabuk öğesinde diff yerine komut (betikte yorumlayıcı), dizin, ağ, yalıtım gösterilir; hedefi olmayan öğede "Nerede" satırı yoktur; yerel onaylı öğe varsa "yerel bir pencerede bir kez daha sorulur" notu görünür.
- **Durum çubuğu:** yalıtım hazırsa "Yalıtımlı" (yeşil), değilse "Yalıtımsız" (sarı, ipucunda talimat).

- [ ] **Step 1: Başarısız testleri yaz**

**`ui/src/durum/terminal.test.ts`** (tam içerik):

```ts
import { describe, expect, it } from 'vitest';
import {
  AZAMI_KARAKTER,
  AZAMI_SATIR,
  BOS_TERMINAL,
  SATIR_SINIRI,
  terminalEkle,
  type Terminal,
} from './terminal';

function metinler(t: Terminal): string[] {
  return t.satirlar.map((s) => `${s.akis}:${s.metin}`);
}

describe('terminal durumu (aşama 2 tasarımı §10.1)', () => {
  it('satır sonuyla bitmeyen parça aynı akışın sonraki parçasıyla birleşir', () => {
    let t = terminalEkle(BOS_TERMINAL, 'Standart', 'mer', 0);
    t = terminalEkle(t, 'Standart', 'haba\ndün', 0);
    t = terminalEkle(t, 'Standart', 'ya\n', 0);
    expect(metinler(t)).toEqual(['Standart:merhaba', 'Standart:dünya']);
    expect(t.acik).toBe(false);
  });

  it('farklı akış yeni satır açar', () => {
    let t = terminalEkle(BOS_TERMINAL, 'Standart', 'a', 0);
    t = terminalEkle(t, 'Hata', 'uyarı\n', 0);
    t = terminalEkle(t, 'Standart', 'b\n', 0);
    expect(metinler(t)).toEqual(['Standart:a', 'Hata:uyarı', 'Standart:b']);
  });

  it('satır başı satırın üzerine yazar', () => {
    let t = terminalEkle(BOS_TERMINAL, 'Standart', '%10', 0);
    t = terminalEkle(t, 'Standart', '\r%50', 0);
    t = terminalEkle(t, 'Standart', '\r%100\nbitti\n', 0);
    expect(metinler(t)).toEqual(['Standart:%100', 'Standart:bitti']);
  });

  it('boş satırlar korunur', () => {
    const t = terminalEkle(BOS_TERMINAL, 'Standart', 'a\n\nb\n', 0);
    expect(metinler(t)).toEqual(['Standart:a', 'Standart:', 'Standart:b']);
  });

  it('atılan bayt bir bilgi satırıyla bildirilir ve toplanır', () => {
    let t = terminalEkle(BOS_TERMINAL, 'Standart', 'a', 0);
    t = terminalEkle(t, 'Standart', 'b\n', 1200);
    t = terminalEkle(t, 'Standart', '', 30);
    expect(metinler(t)).toEqual([
      'Standart:a',
      'Bilgi:… 1200 bayt gösterilmedi …',
      'Standart:b',
      'Bilgi:… 30 bayt gösterilmedi …',
    ]);
    expect(t.gosterilmeyen_bayt).toBe(1230);
  });

  it('satır sınırı aşılınca en eskiler atılır ve sayılır', () => {
    let t = BOS_TERMINAL;
    for (let i = 0; i < AZAMI_SATIR + 10; i++) t = terminalEkle(t, 'Standart', `${i}\n`, 0);
    expect(t.satirlar).toHaveLength(AZAMI_SATIR);
    expect(t.atilan_satir).toBe(10);
    expect(t.satirlar[0].metin).toBe('10');
  });

  it('uzun satır kesilir; devamı eklenmez', () => {
    let t = terminalEkle(BOS_TERMINAL, 'Standart', 'x'.repeat(SATIR_SINIRI + 10), 0);
    t = terminalEkle(t, 'Standart', 'yyy\nsonraki\n', 0);
    expect(t.satirlar[0]).toEqual({ akis: 'Standart', metin: 'x'.repeat(SATIR_SINIRI), kesik: true });
    expect(t.satirlar[1].metin).toBe('sonraki');
    expect(t.karakter).toBe(SATIR_SINIRI + 'sonraki'.length);
  });

  it('karakter sınırı aşılınca en eski satırlar atılır', () => {
    let t = BOS_TERMINAL;
    const satir = 'a'.repeat(1000);
    for (let i = 0; i < AZAMI_KARAKTER / 1000 + 20; i++) {
      t = terminalEkle(t, 'Standart', `${satir}\n`, 0);
    }
    expect(t.karakter).toBeLessThanOrEqual(AZAMI_KARAKTER);
    expect(t.atilan_satir).toBe(20);
    expect(t.karakter).toBe(t.satirlar.reduce((n, s) => n + s.metin.length, 0));
  });

  it('eski durumu değiştirmez', () => {
    const once = terminalEkle(BOS_TERMINAL, 'Standart', 'a', 0);
    terminalEkle(once, 'Standart', 'b', 0);
    expect(metinler(once)).toEqual(['Standart:a']);
  });
});
```

**`ui/src/ana/ansi.test.ts`** (tam içerik):

```ts
import { describe, expect, it } from 'vitest';
import { ansiParcala } from './ansi';

describe('ANSI (aşama 2 tasarımı §10.1)', () => {
  it('kaçışsız metin tek parçadır', () => {
    expect(ansiParcala('düz metin')).toEqual([{ metin: 'düz metin', renk: null, kalin: false }]);
  });

  it('renk ve kalınlık çizilir, sıfırlama geri alır', () => {
    expect(ansiParcala('\x1b[1;31merror\x1b[0m: bozuk \x1b[32mok\x1b[39m son')).toEqual([
      { metin: 'error', renk: 'kirmizi', kalin: true },
      { metin: ': bozuk ', renk: null, kalin: false },
      { metin: 'ok', renk: 'yesil', kalin: false },
      { metin: ' son', renk: null, kalin: false },
    ]);
  });

  it('SGR dışındaki kaçışlar atılır', () => {
    expect(ansiParcala('\x1b[2K\x1b[1Gsatır\x1b]0;başlık\x07 sonu\x1b7')).toEqual([
      { metin: 'satır sonu', renk: null, kalin: false },
    ]);
  });

  it('HTML olarak yorumlanacak bir şey üretmez; metin olduğu gibi kalır', () => {
    expect(ansiParcala('<img src=x onerror=alert(1)>')).toEqual([
      { metin: '<img src=x onerror=alert(1)>', renk: null, kalin: false },
    ]);
  });
});
```

**`ui/src/durum/oturum.test.ts`**: şu bloğu

```ts
        uyarilar: [],
        muhur: 'm',
      },
    ],
```

şununla değiştir:

```ts
        uyarilar: [],
        muhur: 'm',
        kabuk: null,
        yerel_onay: false,
      },
    ],
```

**`ui/src/durum/oturum.test.ts`**: şu bloğu

```ts
  });

  it('onay isteği kart olur; yanıtlanınca çalışmaya döner', () => {
    let d = uygula(acilis(), {
```

şununla değiştir:

```ts
  });

  it('kabuk çıktısı kendi çağrısının terminaline işlenir (aşama 2 tasarımı §10.1)', () => {
    const d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(
        1,
        { tip: 'TurBasladi' },
        { tip: 'AracCagrisiBasladi', cagri: 'c-1', arac: 'kabuk', aciklama: 'a' },
        { tip: 'AracCagrisiBasladi', cagri: 'c-2', arac: 'kabuk', aciklama: 'b' },
        { tip: 'AracCiktisiParcasi', cagri: 'c-1', akis: 'Standart', metin: 'bir\n', atlanan_bayt: 0 },
        { tip: 'AracCiktisiParcasi', cagri: 'c-2', akis: 'Hata', metin: 'iki\n', atlanan_bayt: 5 },
        { tip: 'AracCiktisiParcasi', cagri: 'yok', akis: 'Standart', metin: 'x', atlanan_bayt: 0 },
      ),
    });
    const [a, b] = d.ogeler.filter((o) => o.tip === 'AracCagrisi');
    expect(a.tip === 'AracCagrisi' && a.terminal?.satirlar).toEqual([
      { akis: 'Standart', metin: 'bir' },
    ]);
    expect(b.tip === 'AracCagrisi' && b.terminal?.satirlar).toEqual([
      { akis: 'Bilgi', metin: '… 5 bayt gösterilmedi …' },
      { akis: 'Hata', metin: 'iki' },
    ]);
  });

  it('onay isteği kart olur; yanıtlanınca çalışmaya döner', () => {
    let d = uygula(acilis(), {
```

**`ui/src/durum/oturum.test.ts`**: şu bloğu

```ts
              uyarilar: [],
              muhur: 'm',
            },
          ],
```

şununla değiştir:

```ts
              uyarilar: [],
              muhur: 'm',
              kabuk: null,
              yerel_onay: false,
            },
          ],
```

**`ui/src/ana/bicim.test.ts`**: şu bloğu

```ts
  aracBasligi,
  aracHedefi,
  aracKartDurumu,
  ciktiOnizlemesi,
```

şununla değiştir:

```ts
  aracBasligi,
  aracHedefi,
  kabukOzeti,
  aracKartDurumu,
  ciktiOnizlemesi,
```

**`ui/src/ana/bicim.test.ts`**: şu bloğu

```ts
      durum: 'Basarili',
      sonuc: null,
      ...ek,
    }) as Extract<EkranOgesi, { tip: 'AracCagrisi' }>;
```

şununla değiştir:

```ts
      durum: 'Basarili',
      sonuc: null,
      terminal: null,
      ...ek,
    }) as Extract<EkranOgesi, { tip: 'AracCagrisi' }>;
```

**`ui/src/ana/bicim.test.ts`**: şu bloğu

```ts
    expect(aracHedefi(arac({ argumanlar: '{"yol":"src/lib.rs"}' }))).toBe('src/lib.rs');
    expect(aracHedefi(arac({ argumanlar: 'bozuk' }))).toBeUndefined();
  });
```

şununla değiştir:

```ts
    expect(aracHedefi(arac({ argumanlar: '{"yol":"src/lib.rs"}' }))).toBe('src/lib.rs');
    expect(aracHedefi(arac({ argumanlar: 'bozuk' }))).toBeUndefined();
  });

  it('kabuk: başlık, kayıttan gelen komut ve çıkış kodu özeti (aşama 2 tasarımı §10.1)', () => {
    expect(aracBasligi(arac({ arac: 'kabuk', durum: 'Suruyor' }))).toBe('Komut çalışıyor');
    expect(aracBasligi(arac({ arac: 'kabuk', durum: 'Basarisiz' }))).toBe('Komut');
    expect(
      aracHedefi(arac({ arac: 'kabuk', argumanlar: '{"program":"cargo","argumanlar":["test"]}' })),
    ).toBe('cargo test');
    expect(aracHedefi(arac({ arac: 'kabuk', argumanlar: '{"betik":"a | b\\nc"}' }))).toBe('a | b');
    expect(
      kabukOzeti(arac({ arac: 'kabuk', sonuc: '[kaynak: kabuk:cargo]\nÇıkış kodu: 101\nSüre: 2 sn' })),
    ).toBe('Çıkış kodu: 101');
    expect(kabukOzeti(arac({ sonuc: 'fn main() {}' }))).toBeUndefined();
  });
```

**`ui/src/ana/ekranlar.test.ts`**: şu bloğu

```ts
  });

  it('oturum ortasında anahtar geçersizleşirse hata kartında "Anahtarı değiştir" (§10)', async () => {
    const { arka } = await ciz();
```

şununla değiştir:

```ts
  });

  it('kabuk çağrısının çıktısı terminalde akar; kart çıkış kodunu gösterir', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      { tip: 'AracCagrisiBasladi', cagri: 'k1', arac: 'kabuk', aciklama: '`cargo test` komutunu çalıştırmak istiyor.' },
      {
        tip: 'AracCiktisiParcasi',
        cagri: 'k1',
        akis: 'Standart',
        metin: '\x1b[32mtest ok\x1b[0m\n<b>kalın değil</b>\n',
        atlanan_bayt: 0,
      },
    ]);
    const terminal = await screen.findByLabelText('Komut çıktısı');
    expect(terminal.textContent).toContain('test ok');
    expect(terminal.textContent).toContain('<b>kalın değil</b>');
    expect(terminal.querySelector('b')).toBeNull();
    expect(terminal.querySelector('.r-yesil')?.textContent).toBe('test ok');
    expect(screen.getByText('Komut çalışıyor')).toBeTruthy();
    arka.olaylar('o1', [
      {
        tip: 'AracSonucu',
        cagri: 'k1',
        arac: 'kabuk',
        durum: 'Basarisiz',
        icerik: '[kaynak: kabuk:cargo]\nÇıkış kodu: 101\nSüre: 1,0 sn',
      },
    ]);
    expect(await screen.findByText('Çıkış kodu: 101')).toBeTruthy();
    expect(screen.getByLabelText('Komut çıktısı').textContent).toContain('test ok');
  });

  it('durum çubuğu yalıtım durumunu gösterir', async () => {
    await ciz();
    const oge = screen.getByText('Yalıtımsız', { selector: '.durum-ogesi' });
    expect(oge.getAttribute('title')).toContain('yalıtımsız çalışıyor');
  });

  it('oturum ortasında anahtar geçersizleşirse hata kartında "Anahtarı değiştir" (§10)', async () => {
    const { arka } = await ciz();
```

**`ui/src/ana/ekranlar.test.ts`**: şu bloğu

```ts
  });

  it('Escape karar vermeden kapatır; istek kartta bekler', async () => {
    const { arka } = await ciz();
```

şununla değiştir:

```ts
  });

  it('kabuk öğesinde diff yerine komut, dizin, ağ ve yalıtım; yerel onay notu', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      {
        tip: 'OnayGerekiyor',
        islem: 'i2',
        ogeler: [
          onayOgesi({
            cagri: 'k1',
            arac: 'kabuk',
            aciklama: '`cargo test` komutunu çalıştırmak istiyor.',
            uyarilar: ['Yalıtımsız: komut senin bütün yetkilerinle çalışacak.'],
            kabuk: {
              komut: 'cargo test --features e2e',
              bicim: { tip: 'Program' },
              dizin: 'crates/kalfa',
              ag: true,
              yalitim: { tip: 'Yalitimsiz', talimat: 'yok' },
              ilk_cagri: false,
            },
            yerel_onay: true,
          }),
        ],
      },
    ]);
    await fireEvent.click(await screen.findByRole('button', { name: 'İncele' }));
    const sayfa = await screen.findByRole('dialog');
    expect(within(sayfa).getByText('cargo test --features e2e')).toBeTruthy();
    expect(within(sayfa).getByText('crates/kalfa')).toBeTruthy();
    expect(within(sayfa).getByText('Açık: komut internete erişebilecek')).toBeTruthy();
    expect(within(sayfa).getByText('Yalıtımsız')).toBeTruthy();
    expect(within(sayfa).getByText(/yerel bir pencerede bir kez daha sorulur/)).toBeTruthy();
    expect(within(sayfa).queryByText('Bu değişiklik için önizleme yok.')).toBeNull();
    await fireEvent.click(within(sayfa).getByRole('button', { name: 'Tümünü onayla' }));
    await waitFor(() => expect(arka.komutlar('arac_onayi')).toHaveLength(1));
  });

  it('Escape karar vermeden kapatır; istek kartta bekler', async () => {
    const { arka } = await ciz();
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cd ui && npm test`
Expected: yeni testler başarısız (modüller yok, alanlar eksik).

- [ ] **Step 3: Tipleri, durumu ve metinleri yaz**

**`ui/src/ipc/tipler.ts`**: şu bloğu

```ts
}

export interface OnayOgesi {
  cagri: CagriId;
```

şununla değiştir:

```ts
}

export type KabukBicimi = { tip: 'Program' } | { tip: 'Betik'; yorumlayici: string };

export type YalitimOzeti =
  | { tip: 'Yalitimli'; arka_uc: string; profil: string }
  | { tip: 'Yalitimsiz'; talimat: string };

/** Kabuk çağrısının onay kartında gösterilenler (aşama 2 tasarımı §10.2). */
export interface KabukTarifi {
  komut: string;
  bicim: KabukBicimi;
  dizin: string;
  ag: boolean;
  yalitim: YalitimOzeti;
  ilk_cagri: boolean;
}

export interface OnayOgesi {
  cagri: CagriId;
```

**`ui/src/ipc/tipler.ts`**: şu bloğu

```ts
  uyarilar: string[];
  muhur: string;
}

export type OnayKarari = { tip: 'Onayla' } | { tip: 'Reddet'; gerekce: string | null };
```

şununla değiştir:

```ts
  uyarilar: string[];
  muhur: string;
  kabuk: KabukTarifi | null;
  /** Onayı Rust yerel pencerede ayrıca sorar (aşama 2 tasarımı §5.2). */
  yerel_onay: boolean;
}

export type CiktiAkisi = 'Standart' | 'Hata';

export type OnayKarari = { tip: 'Onayla' } | { tip: 'Reddet'; gerekce: string | null };
```

**`ui/src/ipc/tipler.ts`**: şu bloğu

```ts
  | { tip: 'AracCagrisiBasladi'; cagri: CagriId; arac: string; aciklama: string }
  | { tip: 'AracSonucu'; cagri: CagriId; arac: string; durum: AracDurumu; icerik: string }
  | { tip: 'OnayGerekiyor'; islem: IslemId; ogeler: OnayOgesi[] }
  | { tip: 'KrediGozlemi'; gozlem: KrediGozlemi }
```

şununla değiştir:

```ts
  | { tip: 'AracCagrisiBasladi'; cagri: CagriId; arac: string; aciklama: string }
  | { tip: 'AracSonucu'; cagri: CagriId; arac: string; durum: AracDurumu; icerik: string }
  | {
      tip: 'AracCiktisiParcasi';
      cagri: CagriId;
      akis: CiktiAkisi;
      metin: string;
      /** Bu parçadan önce ekran yetişemediği için atılan baytlar. */
      atlanan_bayt: number;
    }
  | { tip: 'OnayGerekiyor'; islem: IslemId; ogeler: OnayOgesi[] }
  | { tip: 'KrediGozlemi'; gozlem: KrediGozlemi }
```

**`ui/src/ipc/tipler.ts`**: şu bloğu

```ts
export type DepoTuru = 'Kalici' | 'Bellekte';

export interface UygulamaDurumu {
  baglanti: 'Var' | 'Yok';
```

şununla değiştir:

```ts
export type DepoTuru = 'Kalici' | 'Bellekte';

export type YalitimYoklukNedeni =
  | 'BwrapYok'
  | 'KullaniciAdAlaniKapali'
  | 'KurulumGerekli'
  | 'KurulumBozuk'
  | 'Desteklenmiyor';

export type YalitimDurumu =
  | { tip: 'Hazir'; arka_uc: string; surum: string }
  | { tip: 'Yok'; neden: YalitimYoklukNedeni; talimat: string };

export interface UygulamaDurumu {
  baglanti: 'Var' | 'Yok';
```

**`ui/src/ipc/tipler.ts`**: şu bloğu

```ts
  varsayilan_model: string;
  son_oturum: OturumId | null;
}
```

şununla değiştir:

```ts
  varsayilan_model: string;
  son_oturum: OturumId | null;
  yalitim: YalitimDurumu;
}
```

**`ui/src/sahte/arka.ts`**: şu bloğu

```ts
    uyarilar: [],
    muhur: `muhur-${ek.cagri}`,
    ...ek,
  };
```

şununla değiştir:

```ts
    uyarilar: [],
    muhur: `muhur-${ek.cagri}`,
    kabuk: null,
    yerel_onay: false,
    ...ek,
  };
```

**`ui/src/sahte/arka.ts`**: şu bloğu

```ts
    varsayilan_model: 'glm-5.3',
    son_oturum: null,
  };
  /** `calisma_alani_sec`'in döneceği klasör; `null` vazgeçmektir. */
```

şununla değiştir:

```ts
    varsayilan_model: 'glm-5.3',
    son_oturum: null,
    yalitim: {
      tip: 'Yok',
      neden: 'Desteklenmiyor',
      talimat: 'Bu sürümde komutlar yalıtımsız çalışıyor.',
    },
  };
  /** `calisma_alani_sec`'in döneceği klasör; `null` vazgeçmektir. */
```

**`ui/src/bilesenler/ikonlar.ts`**: şu bloğu

```ts
  Send,
  Shield,
  Square,
  TriangleAlert,
  User,
```

şununla değiştir:

```ts
  Send,
  Shield,
  ShieldAlert,
  ShieldCheck,
  Square,
  SquareTerminal,
  TriangleAlert,
  User,
```

**`ui/src/bilesenler/ikonlar.ts`**: şu bloğu

```ts
  send: Send,
  shield: Shield,
  square: Square,
  'triangle-alert': TriangleAlert,
  user: User,
```

şununla değiştir:

```ts
  send: Send,
  shield: Shield,
  'shield-alert': ShieldAlert,
  'shield-check': ShieldCheck,
  square: Square,
  'square-terminal': SquareTerminal,
  'triangle-alert': TriangleAlert,
  user: User,
```

**`ui/src/dil/tr.ts`**: şu bloğu

```ts
  'arac.dosya_duzenle.bitti': 'Dosya düzenlendi',
  'arac.dosya_duzenle.ad': 'Dosya düzenleme',

  'onay.kart.baslik': 'Onay isteği: {sayi} değişiklik',
```

şununla değiştir:

```ts
  'arac.dosya_duzenle.bitti': 'Dosya düzenlendi',
  'arac.dosya_duzenle.ad': 'Dosya düzenleme',
  'arac.kabuk.suruyor': 'Komut çalışıyor',
  'arac.kabuk.bitti': 'Komut çalıştı',
  'arac.kabuk.ad': 'Komut',

  'terminal.etiket': 'Komut çıktısı',
  'terminal.atilan': 'En eski {sayi} satır gösterilmiyor.',
  'terminal.kesik': 'Satırın devamı gösterilmiyor; model çıktının tamamını alır.',

  'onay.kart.baslik': 'Onay isteği: {sayi} değişiklik',
```

**`ui/src/dil/tr.ts`**: şu bloğu

```ts
  'onay.sayfa.secilenleri_onayla': 'Seçilenleri onayla',
  'onay.sayfa.reddet': 'Reddet',

  'sart.kart.baslik': 'EVREN şartları (sürüm {surum})',
```

şununla değiştir:

```ts
  'onay.sayfa.secilenleri_onayla': 'Seçilenleri onayla',
  'onay.sayfa.reddet': 'Reddet',
  'onay.kabuk.komut': 'Komut',
  'onay.kabuk.betik': 'Betik ({yorumlayici})',
  'onay.kabuk.dizin': 'Dizin',
  'onay.kabuk.ag': 'Ağ',
  'onay.kabuk.ag_kapali': 'Kapalı',
  'onay.kabuk.ag_acik': 'Açık: komut internete erişebilecek',
  'onay.kabuk.yalitim': 'Yalıtım',
  'onay.kabuk.yalitimli': 'Yalıtımlı ({arka_uc})',
  'onay.kabuk.yalitimsiz': 'Yalıtımsız',
  'onay.kabuk.yerel':
    'Yalıtımsız komutlar, onayınızdan sonra yerel bir pencerede bir kez daha sorulur.',

  'sart.kart.baslik': 'EVREN şartları (sürüm {surum})',
```

**`ui/src/dil/tr.ts`**: şu bloğu

```ts
  'durum.token_ipucu': 'İstem {istem} · Tamamlama {tamamlama} · Düşünce {dusunce}',
  'durum.model_ipucu': 'Yönlendirilen model',

  'duyuru.onay': 'Onay isteği: {sayi} değişiklik kararınızı bekliyor.',
```

şununla değiştir:

```ts
  'durum.token_ipucu': 'İstem {istem} · Tamamlama {tamamlama} · Düşünce {dusunce}',
  'durum.model_ipucu': 'Yönlendirilen model',
  'durum.yalitimli': 'Yalıtımlı',
  'durum.yalitimsiz': 'Yalıtımsız',

  'duyuru.onay': 'Onay isteği: {sayi} değişiklik kararınızı bekliyor.',
```

**`ui/src/durum/terminal.ts`** (tam içerik):

```ts
// Kabuk çağrısının canlı çıktısı (aşama 2 tasarımı §10.1). Saf işlev; eski
// durumu değiştirmez. Görünüm sınırlıdır: en çok `AZAMI_SATIR` satır ve
// `AZAMI_KARAKTER` karakter tutar, fazlası baştan atılır ve sayılır; tek satır
// `SATIR_SINIRI` karakterde kesilir. Megabaytlarca metni yeniden yerleştirmek
// yazılım çizimli webview'ı kilitler (Linux'ta ölçüldü); tam çıktı modele
// gider, ekran yalnızca izlemek içindir.
import type { CiktiAkisi } from '../ipc/tipler';

export const AZAMI_SATIR = 5000;
export const AZAMI_KARAKTER = 200_000;
export const SATIR_SINIRI = 4000;

export interface TerminalSatiri {
  /** `Bilgi`: Kalfa'nın eklediği satır ("… gösterilmedi …"). */
  akis: CiktiAkisi | 'Bilgi';
  metin: string;
  /** Satır `SATIR_SINIRI`'nda kesildi; devamı gösterilmiyor. */
  kesik?: boolean;
}

export interface Terminal {
  satirlar: TerminalSatiri[];
  /** Son satır henüz satır sonuyla bitmedi; aynı akışın metni ona eklenir. */
  acik: boolean;
  /** Satırlardaki toplam karakter. */
  karakter: number;
  /** Sınır yüzünden baştan atılan satır sayısı. */
  atilan_satir: number;
  /** Ekran yetişemediği için gösterilmeyen bayt sayısı. */
  gosterilmeyen_bayt: number;
}

export const BOS_TERMINAL: Terminal = {
  satirlar: [],
  acik: false,
  karakter: 0,
  atilan_satir: 0,
  gosterilmeyen_bayt: 0,
};

function satir(akis: TerminalSatiri['akis'], metin: string): TerminalSatiri {
  return metin.length > SATIR_SINIRI
    ? { akis, metin: metin.slice(0, SATIR_SINIRI), kesik: true }
    : { akis, metin };
}

export function terminalEkle(
  t: Terminal,
  akis: CiktiAkisi,
  metin: string,
  atlananBayt: number,
): Terminal {
  const satirlar = [...t.satirlar];
  let karakter = t.karakter;
  let acik = t.acik;
  const ekle = (s: TerminalSatiri) => {
    satirlar.push(s);
    karakter += s.metin.length;
  };
  if (atlananBayt > 0) {
    ekle({ akis: 'Bilgi', metin: `… ${atlananBayt} bayt gösterilmedi …` });
    acik = false;
  }
  const parcalar = metin.split('\n');
  parcalar.forEach((parca, i) => {
    const son = satirlar.at(-1);
    // Satır başı (`\r`) satırın üzerine yazar: ilerleme çubuğunun son hâli kalır.
    const cr = parca.lastIndexOf('\r');
    const yazilan = cr >= 0 ? parca.slice(cr + 1) : parca;
    if (i === 0 && acik && son?.akis === akis) {
      const yeni = son.kesik && cr < 0 ? son : satir(akis, cr >= 0 ? yazilan : son.metin + yazilan);
      karakter += yeni.metin.length - son.metin.length;
      satirlar[satirlar.length - 1] = yeni;
    } else if (i < parcalar.length - 1 || yazilan !== '') {
      ekle(satir(akis, yazilan));
    }
  });
  if (metin !== '') acik = !metin.endsWith('\n');
  let fazla = 0;
  while (
    satirlar.length - fazla > 1 &&
    (satirlar.length - fazla > AZAMI_SATIR || karakter > AZAMI_KARAKTER)
  ) {
    karakter -= satirlar[fazla].metin.length;
    fazla += 1;
  }
  return {
    satirlar: fazla > 0 ? satirlar.slice(fazla) : satirlar,
    acik,
    karakter,
    atilan_satir: t.atilan_satir + fazla,
    gosterilmeyen_bayt: t.gosterilmeyen_bayt + atlananBayt,
  };
}
```

**`ui/src/durum/oturum.ts`**: şu bloğu

```ts
  Zarf,
} from '../ipc/tipler';

/** Oturumun o anki hâli; oturum listesindeki nokta ve Composer buna bakar. */
```

şununla değiştir:

```ts
  Zarf,
} from '../ipc/tipler';
import { BOS_TERMINAL, terminalEkle, type Terminal } from './terminal';

/** Oturumun o anki hâli; oturum listesindeki nokta ve Composer buna bakar. */
```

**`ui/src/durum/oturum.ts`**: şu bloğu

```ts
      durum: AracDurumu | 'Suruyor' | null;
      sonuc: string | null;
    })
  | (Ortak & {
```

şununla değiştir:

```ts
      durum: AracDurumu | 'Suruyor' | null;
      sonuc: string | null;
      /** Kabuğun canlı çıktısı; kayıttan gelen çağrıda yok. */
      terminal: Terminal | null;
    })
  | (Ortak & {
```

**`ui/src/durum/oturum.ts`**: şu bloğu

```ts
        durum: o.durum,
        sonuc: o.sonuc,
      };
    case 'TurSonu':
```

şununla değiştir:

```ts
        durum: o.durum,
        sonuc: o.sonuc,
        terminal: null,
      };
    case 'TurSonu':
```

**`ui/src/durum/oturum.ts`**: şu bloğu

```ts
        durum: 'Suruyor',
        sonuc: null,
      });
      return;
```

şununla değiştir:

```ts
        durum: 'Suruyor',
        sonuc: null,
        terminal: null,
      });
      return;
```

**`ui/src/durum/oturum.ts`**: şu bloğu

```ts
      const o = d.ogeler[i];
      if (o?.tip === 'AracCagrisi') d.ogeler[i] = { ...o, durum: yuk.durum, sonuc: yuk.icerik };
      return;
    }
```

şununla değiştir:

```ts
      const o = d.ogeler[i];
      if (o?.tip === 'AracCagrisi') d.ogeler[i] = { ...o, durum: yuk.durum, sonuc: yuk.icerik };
      return;
    }
    case 'AracCiktisiParcasi': {
      const i = d.ogeler.findLastIndex((o) => o.tip === 'AracCagrisi' && o.cagri === yuk.cagri);
      const o = d.ogeler[i];
      if (o?.tip === 'AracCagrisi') {
        d.ogeler[i] = {
          ...o,
          terminal: terminalEkle(o.terminal ?? BOS_TERMINAL, yuk.akis, yuk.metin, yuk.atlanan_bayt),
        };
      }
      return;
    }
```

**`ui/src/ana/ansi.ts`** (tam içerik):

```ts
// Terminal satırındaki ANSI kaçışları (aşama 2 tasarımı §10.1). Yalnızca SGR
// (renk ve kalınlık) çizilir; imleç hareketi, başlık ve diğer kaçışlar atılır.
// Sonuç metin parçalarıdır; görünüm onları metin düğümü olarak ekler, HTML
// olarak yorumlanmaz.

export type Renk = 'kirmizi' | 'yesil' | 'sari' | 'mavi' | 'soluk';

export interface AnsiParcasi {
  metin: string;
  renk: Renk | null;
  kalin: boolean;
}

// CSI (ESC [ ... son harf), OSC (ESC ] ... BEL ya da ESC \) ve tek karakterli
// kaçışlar (ESC 7, ESC M).
const KACIS = /\x1b\[([0-?]*)[ -/]*([@-~])|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b[0-~]/g;

const RENK: Record<number, Renk> = {
  31: 'kirmizi',
  91: 'kirmizi',
  32: 'yesil',
  92: 'yesil',
  33: 'sari',
  93: 'sari',
  34: 'mavi',
  94: 'mavi',
  35: 'mavi',
  95: 'mavi',
  36: 'mavi',
  96: 'mavi',
  90: 'soluk',
};

export function ansiParcala(satir: string): AnsiParcasi[] {
  const parcalar: AnsiParcasi[] = [];
  let renk: Renk | null = null;
  let kalin = false;
  let son = 0;
  const ekle = (metin: string) => {
    if (!metin) return;
    const onceki = parcalar.at(-1);
    if (onceki && onceki.renk === renk && onceki.kalin === kalin) onceki.metin += metin;
    else parcalar.push({ metin, renk, kalin });
  };
  for (const e of satir.matchAll(KACIS)) {
    ekle(satir.slice(son, e.index));
    son = e.index + e[0].length;
    if (e[2] !== 'm') continue;
    const kodlar = (e[1] ?? '').split(';').map((k) => (k === '' ? 0 : Number(k)));
    for (const kod of kodlar) {
      if (kod === 0) {
        renk = null;
        kalin = false;
      } else if (kod === 1) kalin = true;
      else if (kod === 22) kalin = false;
      else if (kod === 39) renk = null;
      else if (kod in RENK) renk = RENK[kod];
      else if ((kod >= 30 && kod <= 37) || (kod >= 90 && kod <= 97)) renk = null;
    }
  }
  ekle(satir.slice(son));
  return parcalar;
}
```

**`ui/src/ana/bicim.ts`**: şu bloğu

```ts
    'arac.dosya_duzenle.ad',
  ],
};
```

şununla değiştir:

```ts
    'arac.dosya_duzenle.ad',
  ],
  kabuk: ['arac.kabuk.suruyor', 'arac.kabuk.bitti', 'arac.kabuk.ad'],
};
```

**`ui/src/ana/bicim.ts`**: şu bloğu

```ts
  try {
    const a = JSON.parse(oge.argumanlar) as Record<string, unknown>;
    for (const alan of ['yol', 'desen', 'dizin']) {
      if (typeof a[alan] === 'string') return a[alan];
```

şununla değiştir:

```ts
  try {
    const a = JSON.parse(oge.argumanlar) as Record<string, unknown>;
    if (typeof a.betik === 'string') return a.betik.split('\n')[0];
    if (typeof a.program === 'string') {
      const argumanlar = Array.isArray(a.argumanlar) ? a.argumanlar : [];
      return [a.program, ...argumanlar].join(' ');
    }
    for (const alan of ['yol', 'desen', 'dizin']) {
      if (typeof a[alan] === 'string') return a[alan];
```

**`ui/src/ana/bicim.ts`**: şu bloğu

```ts
}

/** Araç çıktısının kartta gösterilen kısmı. */
export function ciktiOnizlemesi(
```

şununla değiştir:

```ts
}

/** Kabuk kartının özeti: sonucun ilk satırı ("Çıkış kodu: 0"). */
export function kabukOzeti(oge: AracOgesi): string | undefined {
  if (oge.arac !== 'kabuk' || !oge.sonuc) return undefined;
  // Sonuç `[kaynak: …]` etiketiyle başlar.
  return oge.sonuc.split('\n').find((s) => s && !s.startsWith('[kaynak:'));
}

/** Araç çıktısının kartta gösterilen kısmı. */
export function ciktiOnizlemesi(
```

**`ui/src/ana/onay.ts`**: şu bloğu

```ts
/** Dosya listesinde gösterilen kısa ad: hedefin son bileşeni. */
export function kisaAd(oge: OnayOgesi): string {
  const hedef = oge.hedefler[0];
  if (!hedef) return oge.arac;
```

şununla değiştir:

```ts
/** Dosya listesinde gösterilen kısa ad: hedefin son bileşeni. */
export function kisaAd(oge: OnayOgesi): string {
  if (oge.kabuk) return oge.kabuk.komut.split(/\s+/)[0] || oge.arac;
  const hedef = oge.hedefler[0];
  if (!hedef) return oge.arac;
```

- [ ] **Step 4: Ekranları yaz**

**`ui/src/ana/TerminalGorunumu.svelte`** (tam içerik):

```svelte
<!--
  Kabuk çağrısının canlı çıktısı (aşama 2 tasarımı §10.1). stdout ve stderr
  aynı akışta, stderr ayrı renkte. ANSI'den yalnızca renk ve kalınlık çizilir;
  metin düğümü olarak eklenir, HTML olarak yorumlanmaz. Kullanıcı en alttaysa
  yeni çıktıyla birlikte aşağı kayar.
-->
<script lang="ts">
  import { m } from '../dil/tr';
  import type { Terminal } from '../durum/terminal';
  import { ansiParcala } from './ansi';

  let { terminal }: { terminal: Terminal } = $props();

  let kutu = $state<HTMLPreElement>();
  let altta = true;

  $effect.pre(() => {
    void terminal.satirlar;
    if (kutu) altta = kutu.scrollHeight - kutu.scrollTop - kutu.clientHeight < 24;
  });

  $effect(() => {
    void terminal.satirlar;
    if (kutu && altta) kutu.scrollTop = kutu.scrollHeight;
  });
</script>

{#if terminal.atilan_satir > 0}
  <p class="atilan">{m('terminal.atilan', { sayi: terminal.atilan_satir })}</p>
{/if}
<pre class="terminal" bind:this={kutu} aria-label={m('terminal.etiket')}
  >{#each terminal.satirlar as satir, i (terminal.atilan_satir + i)}{#if i > 0}{'\n'}{/if}<span
      class={['satir', `a-${satir.akis}`]}
      >{#each ansiParcala(satir.metin) as p, j (j)}<span
          class={[p.renk && `r-${p.renk}`, { kalin: p.kalin }]}>{p.metin}</span
        >{/each}{#if satir.kesik}<span class="kesik" title={m('terminal.kesik')}>…</span>{/if}</span
    >{/each}</pre>

<style>
  .terminal {
    margin: 0;
    max-height: 320px;
    overflow: auto;
    padding: 10px 12px;
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-md);
    font: var(--text-small) / 1.5 var(--font-mono);
    color: var(--text-body);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .a-Hata {
    color: var(--status-warn);
  }
  .kesik {
    color: var(--text-muted);
  }
  .a-Bilgi {
    color: var(--text-muted);
    font-style: italic;
  }
  .r-kirmizi {
    color: var(--status-danger);
  }
  .r-yesil {
    color: var(--status-ok);
  }
  .r-sari {
    color: var(--status-warn);
  }
  .r-mavi {
    color: var(--status-info);
  }
  .r-soluk {
    color: var(--text-muted);
  }
  .kalin {
    font-weight: var(--fw-semibold);
  }
  .atilan {
    margin: 0 0 6px;
    font: var(--type-meta);
    color: var(--text-muted);
  }
</style>
```

**`ui/src/ana/KonusmaOgesi.svelte`**: şu bloğu

```svelte
  import Markdown from '../markdown/Markdown.svelte';
  import type { AnaDurum } from './anaDurum.svelte';
  import { aracBasligi, aracHedefi, aracKartDurumu, ciktiOnizlemesi } from './bicim';
  import OnayKarti from './OnayKarti.svelte';

  let { oge, durum, oturum }: { oge: EkranOgesi; durum: AnaDurum; oturum: OturumId } = $props();
```

şununla değiştir:

```svelte
  import Markdown from '../markdown/Markdown.svelte';
  import type { AnaDurum } from './anaDurum.svelte';
  import { aracBasligi, aracHedefi, aracKartDurumu, ciktiOnizlemesi, kabukOzeti } from './bicim';
  import OnayKarti from './OnayKarti.svelte';
  import TerminalGorunumu from './TerminalGorunumu.svelte';

  let { oge, durum, oturum }: { oge: EkranOgesi; durum: AnaDurum; oturum: OturumId } = $props();
```

**`ui/src/ana/KonusmaOgesi.svelte`**: şu bloğu

```svelte
    dosya_yaz: 'file-pen',
    dosya_duzenle: 'file-pen',
  };
```

şununla değiştir:

```svelte
    dosya_yaz: 'file-pen',
    dosya_duzenle: 'file-pen',
    kabuk: 'square-terminal',
  };
```

**`ui/src/ana/KonusmaOgesi.svelte`**: şu bloğu

```svelte
    tool={aracBasligi(oge)}
    target={aracHedefi(oge)}
    status={d.status}
    statusLabel={d.etiket}
    icon={ARAC_IKONU[oge.arac] ?? 'box'}
  >
    {#if oge.sonuc}
      {@const o = ciktiOnizlemesi(oge.sonuc)}
      <CodeBlock code={o.kod} truncated={o.kirpildi} maxHeight="320px" />
```

şununla değiştir:

```svelte
    tool={aracBasligi(oge)}
    target={aracHedefi(oge)}
    meta={kabukOzeti(oge)}
    status={d.status}
    statusLabel={d.etiket}
    icon={ARAC_IKONU[oge.arac] ?? 'box'}
  >
    {#if oge.terminal}
      <TerminalGorunumu terminal={oge.terminal} />
    {:else if oge.sonuc}
      {@const o = ciktiOnizlemesi(oge.sonuc)}
      <CodeBlock code={o.kod} truncated={o.kirpildi} maxHeight="320px" />
```

**`ui/src/ana/OnaySayfasi.svelte`**: şu bloğu

```svelte
  gerekçesi ve üç karar. Seçilmeyen çağrılar gerekçeyle reddedilir. Odak
  sayfanın içinde kalır; Escape karar vermeden kapatır. "Bu değişiklikler geri
  alınabilir" cümlesi geri alma gelene kadar yazılmaz.
-->
<script lang="ts">
```

şununla değiştir:

```svelte
  gerekçesi ve üç karar. Seçilmeyen çağrılar gerekçeyle reddedilir. Odak
  sayfanın içinde kalır; Escape karar vermeden kapatır. "Bu değişiklikler geri
  alınabilir" cümlesi geri alma gelene kadar yazılmaz. Kabuk çağrısında diff
  yerine komut, dizin, ağ ve yalıtım gösterilir (aşama 2 tasarımı §10.2).
-->
<script lang="ts">
```

**`ui/src/ana/OnaySayfasi.svelte`**: şu bloğu

```svelte
      <div class="onizleme">
        <div class="onizleme-ust">
          <Icon name="file-text" size={16} />
          <div class="yol-blogu">
            <div class="ad">{kisaAd(aktifOge)}</div>
```

şununla değiştir:

```svelte
      <div class="onizleme">
        <div class="onizleme-ust">
          <Icon name={aktifOge.kabuk ? 'square-terminal' : 'file-text'} size={16} />
          <div class="yol-blogu">
            <div class="ad">{kisaAd(aktifOge)}</div>
```

**`ui/src/ana/OnaySayfasi.svelte`**: şu bloğu

```svelte
          </div>
        </div>
        <div class="diff">
          {#each parcalar as p, i (i)}
            <DiffView hunkHeader={p.baslik} lines={p.satirlar} />
          {:else}
            <p class="bos">{m('onay.sayfa.diff_yok')}</p>
          {/each}
        </div>
      </div>
    </div>
```

şununla değiştir:

```svelte
          </div>
        </div>
        {#if aktifOge.kabuk}
          {@const k = aktifOge.kabuk}
          <dl class="kabuk">
            <dt>
              {k.bicim.tip === 'Betik'
                ? m('onay.kabuk.betik', { yorumlayici: k.bicim.yorumlayici })
                : m('onay.kabuk.komut')}
            </dt>
            <dd><pre class="komut">{k.komut}</pre></dd>
            <dt>{m('onay.kabuk.dizin')}</dt>
            <dd class="mono">{k.dizin}</dd>
            <dt>{m('onay.kabuk.ag')}</dt>
            <dd class={{ uyari: k.ag }}>
              {k.ag ? m('onay.kabuk.ag_acik') : m('onay.kabuk.ag_kapali')}
            </dd>
            <dt>{m('onay.kabuk.yalitim')}</dt>
            <dd class={{ uyari: k.yalitim.tip === 'Yalitimsiz' }}>
              {k.yalitim.tip === 'Yalitimli'
                ? m('onay.kabuk.yalitimli', { arka_uc: k.yalitim.arka_uc })
                : m('onay.kabuk.yalitimsiz')}
            </dd>
          </dl>
        {:else}
          <div class="diff">
            {#each parcalar as p, i (i)}
              <DiffView hunkHeader={p.baslik} lines={p.satirlar} />
            {:else}
              <p class="bos">{m('onay.sayfa.diff_yok')}</p>
            {/each}
          </div>
        {/if}
      </div>
    </div>
```

**`ui/src/ana/OnaySayfasi.svelte`**: şu bloğu

```svelte
        <dd>{aktifOge.aciklama}</dd>
      </div>
      <div class="bilgi">
        <dt><Icon name="folder" size={16} />{m('onay.sayfa.nerede')}</dt>
        <dd class="mono">
          {#each aktifOge.hedefler as h (h)}<span>{h}</span>{/each}
        </dd>
      </div>
      {#if aktifOge.uyarilar.length > 0}
        <div class="bilgi uyari">
```

şununla değiştir:

```svelte
        <dd>{aktifOge.aciklama}</dd>
      </div>
      {#if aktifOge.hedefler.length > 0}
        <div class="bilgi">
          <dt><Icon name="folder" size={16} />{m('onay.sayfa.nerede')}</dt>
          <dd class="mono">
            {#each aktifOge.hedefler as h (h)}<span>{h}</span>{/each}
          </dd>
        </div>
      {/if}
      {#if aktifOge.uyarilar.length > 0}
        <div class="bilgi uyari">
```

**`ui/src/ana/OnaySayfasi.svelte`**: şu bloğu

```svelte
      {/if}
    </dl>

    <div class="alt">
```

şununla değiştir:

```svelte
      {/if}
    </dl>

    {#if ogeler.some((o) => o.yerel_onay)}
      <p class="yerel"><Icon name="shield-alert" size={16} />{m('onay.kabuk.yerel')}</p>
    {/if}

    <div class="alt">
```

**`ui/src/ana/OnaySayfasi.svelte`**: şu bloğu

```svelte
    overflow: auto;
  }
  .bos {
    margin: 16px;
```

şununla değiştir:

```svelte
    overflow: auto;
  }
  .kabuk {
    display: grid;
    grid-template-columns: 90px 1fr;
    gap: 10px 14px;
    margin: 0;
    padding: 14px 16px;
    font: var(--type-small);
  }
  .kabuk dt {
    color: var(--text-muted);
  }
  .kabuk dd {
    min-width: 0;
    color: var(--text-body);
  }
  .komut {
    margin: 0;
    max-height: 240px;
    overflow: auto;
    font: var(--text-small) / 1.5 var(--font-mono);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .kabuk .uyari {
    color: var(--status-warn);
  }
  .yerel {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 0 16px;
    font: var(--type-small);
    color: var(--status-warn);
  }
  .bos {
    margin: 16px;
```

**`ui/src/ana/DurumCubugu.svelte`**: şu bloğu

```svelte
    {#if durum.uygulama?.baglanti === 'Yok'}
      <StatusItem icon="wifi-off" tone="danger">{m('durum.baglanti_yok')}</StatusItem>
    {/if}
    {#if durum.uygulama?.anahtar_deposu === 'Bellekte'}
```

şununla değiştir:

```svelte
    {#if durum.uygulama?.baglanti === 'Yok'}
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

Önizlemeye (`npm run dev`, `onizleme.html`) kabuk senaryosu: "test" geçen mesaj kabuk onayı ister, onaylanınca renkli çıktı akar.

**`ui/src/onizleme/senaryo.ts`**: şu bloğu

```ts
  '     }',
].join('\n');

function gecikmeli(adimlar: [number, () => void][]) {
```

şununla değiştir:

```ts
  '     }',
].join('\n');

/** Kabuk çağrısının örnek çıktısı (aşama 2 tasarımı §10.1): renkli satırlar, stderr. */
const KABUK_CIKTISI: [string, 'Standart' | 'Hata'][] = [
  ['   Compiling kalfa-api v0.1.0 (C:\\projeler\\kalfa-api)\n', 'Hata'],
  ['    Finished `test` profile [unoptimized] target(s) in 4.21s\n', 'Hata'],
  ['     Running unittests src\\lib.rs\n', 'Hata'],
  ['\nrunning 3 tests\n', 'Standart'],
  ['test auth::token_suresi_24_saat ... \x1b[32mok\x1b[0m\n', 'Standart'],
  ['test depo::token_ozetlenir ... \x1b[32mok\x1b[0m\n', 'Standart'],
  ['test depo::eski_kayit_okunur ... \x1b[1;31mFAILED\x1b[0m\n', 'Standart'],
  ['\ntest result: \x1b[31mFAILED\x1b[0m. 2 passed; 1 failed\n', 'Standart'],
];

function gecikmeli(adimlar: [number, () => void][]) {
```

**`ui/src/onizleme/senaryo.ts`**: şu bloğu

```ts
    const t = `t${tur}`;
    const gonder = (olaylar: Olay[]) => arka.olaylar(oturum, olaylar, t);
    gecikmeli([
      [200, () => gonder([{ tip: 'TurBasladi' }])],
```

şununla değiştir:

```ts
    const t = `t${tur}`;
    const gonder = (olaylar: Olay[]) => arka.olaylar(oturum, olaylar, t);
    // "test" geçen mesaj kabuk çağrısı ister (aşama 2 tasarımı §10).
    if (String(a.metin).includes('test')) {
      gecikmeli([
        [200, () => gonder([{ tip: 'TurBasladi' }])],
        [300, () => gonder([{ tip: 'TokenParcasi', metin: 'Testleri çalıştırıyorum.' }])],
        [
          400,
          () =>
            gonder([
              {
                tip: 'OnayGerekiyor',
                islem: `islem-${tur}`,
                ogeler: [
                  onayOgesi({
                    cagri: `k${tur}`,
                    arac: 'kabuk',
                    aciklama: '`cargo test --lib` komutunu çalıştırmak istiyor.',
                    uyarilar: [
                      'Yalıtımsız: komut senin bütün yetkilerinle çalışacak; ev dizinindeki dosyaları okuyabilir ve ağa erişebilir.',
                    ],
                    kabuk: {
                      komut: 'cargo test --lib',
                      bicim: { tip: 'Program' },
                      dizin: '.',
                      ag: false,
                      yalitim: { tip: 'Yalitimsiz', talimat: arka.uygulama.yalitim.tip === 'Yok' ? arka.uygulama.yalitim.talimat : '' },
                      ilk_cagri: false,
                    },
                    yerel_onay: true,
                  }),
                ],
              },
            ]),
        ],
      ]);
      return null;
    }
    gecikmeli([
      [200, () => gonder([{ tip: 'TurBasladi' }])],
```

**`ui/src/onizleme/senaryo.ts`**: şu bloğu

```ts
    const t = `t${tur}`;
    const gonder = (olaylar: Olay[]) => arka.olaylar(oturum, olaylar, t);
    gecikmeli([
      [
```

şununla değiştir:

```ts
    const t = `t${tur}`;
    const gonder = (olaylar: Olay[]) => arka.olaylar(oturum, olaylar, t);
    const kabuk = kararlar.find((k) => k.cagri.startsWith('k'));
    if (kabuk) {
      const cagri = kabuk.cagri;
      gecikmeli([
        [
          200,
          () =>
            gonder([
              {
                tip: 'AracCagrisiBasladi',
                cagri,
                arac: 'kabuk',
                aciklama: '`cargo test --lib` komutunu çalıştırmak istiyor.',
              },
            ]),
        ],
        ...KABUK_CIKTISI.map(([metin, akis]): [number, () => void] => [
          350,
          () => gonder([{ tip: 'AracCiktisiParcasi', cagri, akis, metin, atlanan_bayt: 0 }]),
        ]),
        [
          300,
          () =>
            gonder([
              {
                tip: 'AracSonucu',
                cagri,
                arac: 'kabuk',
                durum: 'Basarisiz',
                icerik: '[kaynak: kabuk:cargo]\nÇıkış kodu: 101\nSüre: 5,3 sn\n--- çıktı ---\n…',
              },
              { tip: 'TokenParcasi', metin: 'Bir test başarısız: `depo::eski_kayit_okunur`.' },
              { tip: 'Tamamlandi', neden: { tip: 'Normal' } },
            ]),
        ],
      ]);
      return null;
    }
    gecikmeli([
      [
```

- [ ] **Step 5: Testlerin geçtiğini gör**

Run: `cd ui && npm run check && npm test && npm run build`
Expected: 0 hata, 0 uyarı; 18 dosya, 205 test; paket denetimi temiz.

- [ ] **Step 6: Commit**

```bash
git add ui
git commit -m "ui: terminal görünümü, kabuk onay ayrıntısı ve yalıtım göstergesi"
```

---

### Task 6: Uçtan uca kabuk testleri

**Files:**
- Create: `e2e/testler/05-kabuk.test.mjs`

**Interfaces:**
- Consumes: 1c-4'ün `e2e/yardimci/kalfa.mjs`'i (`Kalfa.ac`, `gonder`, `dugme`, `ikonDugmesi`, `metniBekle`, `turSonuBekle`, `sureyleBekle`); sahte EVREN'in `onay: true` ayarı (yerel iletişim kutusunu onaylar).
- Produces: 3 test.

**Davranış notları:**
- Komutlar `node -e` ile yazılır; iki platformda aynı çalışır.
- İptal testi komut satırına bir işaret ekler, komutun ve torununun çalıştığını, iptalden sonra ikisinin de durduğunu süreç listesinden doğrular (Windows'ta `Win32_Process`, Linux'ta `/proc`).
- Çıktı seli testi 3 sn boyunca 64 KiB'lık satırlar yazan bir komut çalıştırır; tur `Normal` bitmeli, "Aktarım durdu" görünmemeli ve sonraki mesaj yanıtlanmalı.
- Terminal, konuşmadaki son kabuk çağrısınınkidir (`querySelectorAll(...).at(-1)`).

- [ ] **Step 1: Testleri yaz**

**`e2e/testler/05-kabuk.test.mjs`** (tam içerik):

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

/** İptal testindeki süreçleri bulmak için komut satırına eklenen işaret. */
const ISARET = `kalfa-e2e-kabuk-${process.pid}`;

const node = (betik, ...ek) => ({ program: 'node', argumanlar: ['-e', betik, ...ek] });

const SENARYOLAR = {
  kabuk: {
    yanitlar: [
      [
        { tur: 'metin', metin: 'Komutu çalıştırıyorum.' },
        {
          tur: 'arac',
          ad: 'kabuk',
          argumanlar: node("console.log('kabuktan-merhaba'); console.error('uyari-satiri')"),
        },
      ],
      [{ tur: 'arac_sonucu' }],
    ],
  },
  'kabuk-iptal': {
    yanitlar: [
      [
        {
          tur: 'arac',
          ad: 'kabuk',
          argumanlar: node(
            "require('child_process').spawn(process.execPath, " +
              "['-e', 'setInterval(() => {}, 1000)', process.argv[1] + '-torun'], { stdio: 'ignore' }); " +
              "setInterval(() => console.log('tik'), 200)",
            ISARET,
          ),
        },
      ],
      [{ tur: 'metin', metin: 'olmamalı' }],
    ],
  },
  'kabuk-sel': {
    yanitlar: [
      [
        {
          tur: 'arac',
          ad: 'kabuk',
          argumanlar: node(
            "const s = 'x'.repeat(65536) + '\\n'; const son = Date.now() + 3000; " +
              "function yaz() { while (Date.now() < son) { if (!process.stdout.write(s)) " +
              "return process.stdout.once('drain', yaz); } } yaz();",
          ),
        },
      ],
      [{ tur: 'metin', metin: 'Sel bitti.' }],
    ],
  },
};

/** Komut satırında işareti taşıyan süreçler (torun dahil). */
function isaretliSurecler() {
  if (process.platform === 'win32') {
    const cikti = execFileSync(
      'powershell.exe',
      [
        '-NoProfile',
        '-Command',
        `Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like '*${ISARET}*' -and $_.Name -eq 'node.exe' } | ForEach-Object { $_.ProcessId }; exit 0`,
      ],
      { encoding: 'utf8' },
    );
    return cikti.split(/\r?\n/).filter(Boolean);
  }
  return readdirSync('/proc').filter((ad) => {
    if (!/^\d+$/.test(ad)) return false;
    try {
      return readFileSync(`/proc/${ad}/cmdline`, 'utf8').includes(ISARET);
    } catch {
      return false;
    }
  });
}

/** Konuşmadaki son kabuk çağrısının terminali. */
async function terminalMetni(k) {
  return k.b.execute(
    () => [...document.querySelectorAll('[aria-label="Komut çıktısı"]')].at(-1)?.textContent ?? '',
  );
}

async function onayla(k) {
  await k.dugme('İncele');
  await k.dugme('Tümünü onayla');
}

describe('kabuk', () => {
  let k;
  before(async () => {
    k = await Kalfa.ac({
      sart: 'kabul',
      anahtar: GECERLI_ANAHTAR,
      onay: true,
      senaryolar: SENARYOLAR,
    });
    await k.pencere('ana');
    await k.yeniOturum();
  });
  after(async () => {
    await k?.kapat();
  });

  it('onaylanan komutun çıktısı canlı akar, sonucu modele gider', async () => {
    await k.gonder('kabuk');
    await k.metniBekle('Komutu çalıştırıyorum.');
    await onayla(k);
    assert.equal(await k.turSonuBekle(1), 'Normal');
    const terminal = await terminalMetni(k);
    assert.match(terminal, /kabuktan-merhaba/);
    assert.match(terminal, /uyari-satiri/);
    await k.metniBekle('Çıkış kodu: 0');
    await k.metniBekle('[kaynak: kabuk:node]');
  });

  it('süren komut iptal edilir; torunuyla birlikte durur', async () => {
    await k.gonder('kabuk-iptal');
    await onayla(k);
    await sureyleBekle(
      async () => (await terminalMetni(k)).includes('tik'),
      20000,
      'komut çıktı üretmedi',
    );
    assert.ok(isaretliSurecler().length >= 2, 'komut ve torunu çalışıyor olmalı');
    await k.ikonDugmesi('Turu durdur');
    assert.equal(await k.turSonuBekle(2), 'Iptal');
    await sureyleBekle(
      () => isaretliSurecler().length === 0,
      10000,
      `süreç ağacı durmadı: ${isaretliSurecler().join(', ')}`,
    );
  });

  it('çıktı seli arayüzü kilitlemez ve turu iptal ettirmez', async () => {
    await k.gonder('kabuk-sel');
    await onayla(k);
    assert.equal(await k.turSonuBekle(3, 60000), 'Normal');
    await k.metniBekle('Sel bitti.');
    const durdu = await k.b.execute(() => document.body.textContent.includes('Aktarım durdu'));
    assert.equal(durdu, false);
    await k.gonder('devam');
    await k.metniBekle('Yanıt: devam');
    assert.equal(await k.turSonuBekle(4), 'Normal');
  });
});
```

- [ ] **Step 2: Uçtan uca testleri koştur**

Run (Windows): `cd e2e && npm ci && npm test`
Run (Linux): `cd e2e && npm ci && xvfb-run -a npm test`
Expected: 24 test geçer (1c'nin 21'i ve bu 3'ü). Sürücüler ortamında yoksa bunu PR'a yaz ve devam et; Claude incelemede iki platformda koşturur.

- [ ] **Step 3: Commit**

```bash
git add e2e/testler/05-kabuk.test.mjs
git commit -m "e2e: kabuk: onay, canlı çıktı, iptal ve çıktı seli"
```

---

### Task 7: Belgeler ve son doğrulama

**Files:**
- Modify: `README.md`, `docs/GELISTIRME_DURUMU.md`

- [ ] **Step 1: Son doğrulama**

Global Constraints'teki bütün denetimleri koştur (Rust, `e2e` özelliği, arayüz). Uçtan uca testleri koşturabildiysen onları da. Sayılar aşağıdaki belgedekilerden farklıysa belgeyi kendi sonuçlarınla güncelle ve farkı PR yorumuna yaz.

- [ ] **Step 2: Belgeleri güncelle**

**`README.md`**: şu bloğu

```markdown
onayınla. Araçlarını WebAssembly eklentileriyle genişletebilirsin.

> **Durum: 1c tamamlandı; masaüstü uygulaması Windows ve Linux'ta çalışıyor.**
> Rust workspace'inde EVREN istemcisi (`kalfa-evren`), arayüzden bağımsız oturum
> çekirdeği (`kalfa-core`), beş yerleşik dosya aracı (`kalfa-tools`) ve
> masaüstü uygulaması (`kalfa-uygulama`, Tauri kabuğu `kalfa`) bulunuyor;
> Svelte arayüzü `ui/`'dadır. Kabuk aracı, yalıtım ve eklentiler sonraki
> aşamalarda. [Geliştirme durumu ve devam noktası](docs/GELISTIRME_DURUMU.md).
```

şununla değiştir:

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
```

**`docs/GELISTIRME_DURUMU.md`**: şu bloğu

```markdown
  bağlantılar (iç ya da dış, ayrıca Windows junction) izlenmez; yazma/
  düzenleme onaydan sonra içerik ya da hard link sayısı değişirse çalışmaz.
- **Uçtan uca:** gerçek dosya araçlarıyla, arayüz olmadan, okuma → onay →
  düzenleme → model yanıtı → kayıttan kurtarma akışı test edilir; aynı akış
```

şununla değiştir:

```markdown
  bağlantılar (iç ya da dış, ayrıca Windows junction) izlenmez; yazma/
  düzenleme onaydan sonra içerik ya da hard link sayısı değişirse çalışmaz.
- **1c — masaüstü uygulaması:** `kalfa-uygulama`, Tauri kabuğu `kalfa`,
  Svelte arayüzü `ui/`; ayrıntısı aşağıda.
- **2a — kabuk aracı:** `kabuk` (`kalfa-tools`) ve komut çalıştırma, süreç
  sınırları (`kalfa-sandbox`). Yalıtım henüz yok; her komut yerel pencerede
  onaylanır. Ayrıntısı aşağıda.
- **Uçtan uca:** gerçek dosya araçlarıyla, arayüz olmadan, okuma → onay →
  düzenleme → model yanıtı → kayıttan kurtarma akışı test edilir; aynı akış
```

**`docs/GELISTIRME_DURUMU.md`**: şu bloğu

```markdown
Linux arka ucu, 2c Windows prototipi, 2d Windows arka ucu.

Kabuk yalıtımı, bağlam özetleme, geri alma, gönderim günlüğü, oturum bütçesi,
güvenlik denetçisi ve WASM eklentileri sonraki aşamalardadır. Masaüstü
```

şununla değiştir:

```markdown
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

Sıradaki plan 2b: Linux arka ucu (bubblewrap + seccomp).

Kabuk yalıtımı, bağlam özetleme, geri alma, gönderim günlüğü, oturum bütçesi,
güvenlik denetçisi ve WASM eklentileri sonraki aşamalardadır. Masaüstü
```

- [ ] **Step 3: Commit**

```bash
git add README.md docs/GELISTIRME_DURUMU.md
git commit -m "docs: 2a kabuk aracı ve süreç sınırları"
```

---

## İnceleme (Claude)

- Plan commit'inden sonraki her commit'in plandaki görevle eşleşmesi; kodun plandakiyle aynı olması.
- Windows ve Linux'ta: Global Constraints'teki bütün denetimler ve uçtan uca testler (24).
- Güvenlik sınırlarının testleri: yerel onayın webview'la verilememesi (`oturum_kabuk.rs`, `tests/kabuk.rs`), ortam süzgeci, süreç ağacı, terminal görünümünün metin düğümü kullanması.
- Belgelerdeki sayıların inceleme sonuçlarıyla aynı olması.
