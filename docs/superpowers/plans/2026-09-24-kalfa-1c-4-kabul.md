# Kalfa 1c-4 — Kabul Uygulama Planı

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Gerçek uygulamayı Windows 11 ve Ubuntu 24.04'te, sahte EVREN'li bir test derlemesiyle ve `tauri-driver` (WebDriver) üzerinden baştan sona sınamak. Üst belge §11.1'in kabul kapısını ölçmek.

**Architecture:**
- **Test derlemesi:** `e2e` adlı bir Cargo özelliğidir; sürüm derlemesine girmez.
- **`kalfa-uygulama` tarafı:**
  - Senaryolu sahte sağlayıcı ve ayardaki cevapları veren sahte iletişim kutuları.
  - Kuyruk ve yoldaki veri için en yüksek değer sayaçları.
  - Uzun oturum için kayıt üreteci.
- **`kalfa` tarafı:**
  - Bu özellikle sahteleri kurar.
  - Ana pencereye açılış sırasında eklenen yetkiyle dört test komutu verir.
- **Arayüz:**
  - `npm run build:e2e` ile, kanal dinleyicisini ölçen ve duraklatabilen bir kancayla derlenir.
  - Sürüm paketinde kanca olmadığını paket denetimi sınar.
- **Testler:** Yeni `e2e/` dizinindedir (WebdriverIO, `node:test`). Her test kendi Kalfa sürecini geçici bir veri dizini, çalışma alanı ve ayar dosyasıyla açar.

**Tech Stack:** Rust 1.98 · Tauri 2.11 (`CapabilityBuilder`, `RunEvent::ExitRequested`) · tauri-driver 2.0.6 · msedgedriver (WebView2 sürümüyle aynı) · WebKitWebDriver (webkit2gtk-driver 2.52) · Xvfb · WebdriverIO 9.32 · Node `node:test` · Vite 8 (`--mode e2e`) · Shiki 4.4 (Oniguruma motoru yalnızca testte)

**Spec:** `docs/superpowers/specs/2026-09-22-kalfa-1c-design.md` (1c tasarımı): §2, §4.2, §4.3, §5.6, §9.2, §11.4, §11.6, §13. Tasarım belgesinin 1c-4 kararlarıyla güncellenmiş hâli plan commit'indedir.

**Kod hakkında:** Bu plandaki kodun tamamı plan yazılırken ayrı bir kopyada derlendi ve doğrulandı (2026-09-24).

- **Windows 11 (WebView2 153, msedgedriver 153):**
  - `cargo test --workspace --locked`: 303 test geçti.
  - `e2e` özellikli testler de geçti; normal ve `e2e` derlemesinde clippy ve fmt temiz.
  - `ui/`'da check, test ve build temiz.
  - Uçtan uca: 21 test; son hâliyle iki tam koşuda ikisinde de geçti, gecikme p95'i 25 ms civarında.
- **Linux (Ubuntu 24.04, WSL2, Xvfb, WebKitGTK 2.52):**
  - `cargo test --workspace --locked`: 302 test geçti. Aradaki fark Windows'a özel junction testi.
  - Uçtan uca: aynı 21 test; son hâliyle iki tam koşuda ikisinde de geçti, gecikme p95'i 29–42 ms.

Kodu olduğu gibi kullan. Bir şey yine de derlenmezse testin sınadığı davranışı değiştirmeden düzelt; davranış değişmesi gerekiyorsa dur ve PR'a yorum yaz.

**Bu planın kapsamı dışında:**
- Elle kontroller (1c tasarımı §11.5): ekran okuyucu, yalnızca klavyeyle akış, tasarımla görsel karşılaştırma, Secret Service olmadan açılış.
- `same-file`'ın ReFS'teki kimliği: makinede ReFS birimi yok; durum dosyasında açık madde olarak kalır.
- Sürekli tümleştirme (CI) kurulumu.
- Sonraki aşamalar.

## PR ile yürütme

Kullanıcı kararı (2026-09-23): kodu Codex yazar, Claude inceler.

- **Dal ve PR:** çalışma dalı `1c-4/kabul`, PR'ın hedefi `kalfa-asama-1`.
- **Hazır gelenler (plan commit'i):**
  - Plan ve tasarım belgesinin güncellemesi.
  - `crates/kalfa-uygulama/Cargo.toml`: `e2e` özelliği ve iki isteğe bağlı bağımlılık.
  - `crates/kalfa/Cargo.toml`: `e2e` özelliği.
  - `Cargo.lock`.
  - `e2e/package.json`, `e2e/package-lock.json`, `e2e/.npmrc`, `e2e/.gitignore`.

  Bu dosyaları değiştirme.
- **Ortam:**
  - **Araç sürümleri:** Rust 1.98.1 (`rust-toolchain.toml`); Node `^22.22.2 || >=24.15.0`, npm 10 ya da üstü.
  - **Ağ:** `npm ci` için ağ erişimi gerekir.
  - **Uçtan uca testler için:**
    - `tauri-driver` PATH'te olmalı (`cargo install tauri-driver --locked`).
    - Windows'ta WebView2 sürümüyle aynı `msedgedriver` gerekir; PATH'te değilse yolu `KALFA_E2E_EDGEDRIVER` ile verilir.
    - Linux'ta `webkit2gtk-driver` ve `xvfb` gerekir.
    - Sürücüleri kendin indirme ya da kurma; ortamında yoksa PR'a yaz. Claude uçtan uca testleri incelemede iki platformda koşturur.
  - **Linux'ta `kalfa` crate'i:** `libwebkit2gtk-4.1-dev` ister.
  - **Biçim:** biçimlendirici (Prettier ve benzeri) projede yok; koşturma. Kod plandaki biçimiyle yazılır. Rust için `cargo fmt` kullanılır.
- **Sıra:** görevler sırayla. Her görev planda yazan mesajla kendi commit'ini alır; her commit'ten sonra push'la. Geçmiş yeniden yazılmaz, force-push yapılmaz.
- **Soru ve sapma:** plandan sapman gerekirse dur ve PR'a yorum yaz; kendi başına karar verme.
- **İnceleme:** Claude PR'a yorum yazar, Windows'ta ve Linux'ta doğrular, uçtan uca testleri iki platformda koşturur; inceleme temizse birleştirir.

## Global Constraints

- **Araçlar:** Rust 1.98.1; `cargo` komutları `--locked` ile. Node `^22.22.2 || >=24.15.0`; kurulum `npm ci`.
- **Bağımlılık:**
  - Yeni bağımlılık eklenmez. `futures-util` ve `serde_json` workspace'tendir ve yalnızca `e2e` özelliğiyle gelir; plan commit'indedir.
  - `e2e/`'nin tek bağımlılığı `webdriverio` 9.32.0'dır.
  - `e2e/.npmrc` kurulum betiklerini kapatır (`ignore-scripts=true`); webdriverio'nun bağımlılıkları kurulumda tarayıcı sürücüsü indirmez.
- **Test derlemesi sürüme girmez (1c tasarımı §11.4):**
  - Rust'taki bütün e2e kodu `#[cfg(feature = "e2e")]` arkasındadır.
  - Arayüzdeki kanca yalnızca `import.meta.env.MODE === 'e2e'` dalından çağrılır; modülün üst düzeyinde yan etki yoktur.
  - `ui/scripts/paket-denetimi.mjs` sürüm paketinde `__kalfaE2e` arar.
  - `crates/kalfa/tests/izinler.rs` normal derlemede test komutlarının reddedildiğini sınar.
- **Test komutlarının yetkisi:** yalnızca ana pencere. Yetki çalışma anında `CapabilityBuilder` ile eklenir. `crates/kalfa/capabilities/*.json` ve `tauri.conf.json` değişmez.
- **Gerçek EVREN'e istek gitmez:**
  - Test derlemesinde sağlayıcı sahtedir.
  - Testlerin anahtarları sahtedir.
  - İşletim sisteminin kimlik deposuna dokunulmaz (`KALFA_ANAHTAR_DEPOSU=bellek`).
- **Şartlar (üst belge §11.8):**
  - Uygulama kodu şartları kullanıcı adına kabul etmez.
  - Uçtan uca testler sahte EVREN'in şartlarını, kullanıcı gibi şart penceresindeki "Kabul ediyorum" düğmesiyle kabul eder.
  - Canlı `POST /v1/terms/accept` hiçbir durumda çağrılmaz.
- **Tek süreç:**
  - Test derlemesinin uygulama kimliği `dev.kalfa.e2e`'dir; geliştiricinin açık Kalfa'sıyla çakışmaz.
  - Testler sırayla koşar; her test önceki Kalfa sürecinin kapanmasını bekler.
- **CSP (1c-3a):** şablonda sabit `style="…"` yok; `{@html}`, `innerHTML` ve `eval` yok. `ui/src/csp.test.ts` yeni dosyaları da tarar. İstisna: `e2e/` sayfaya bu CSP'nin engellediğini göstermek için betik ekler; o kod uygulamaya girmez.
- **Dil:**
  - Kod ve testlerdeki adlar ile açıklamalar Türkçedir.
  - Kullanıcıya görünen yeni metin yok.
  - Rust'ta `unwrap()` ve `expect()` yalnızca testlerde ve `test_destek.rs`'de kullanılır.
- **Her görevin sonunda:**
  - Rust tarafı: `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings` ve `cargo fmt --all -- --check` geçer.
  - `e2e` özelliğine dokunan görevlerde ayrıca `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked` ve `cargo clippy -p kalfa -p kalfa-uygulama --features e2e --all-targets --locked -- -D warnings` geçer.
  - Arayüze dokunan görevlerde `ui/` içinde `npm run check` (0 hata, 0 uyarı), `npm test` ve `npm run build` (paket denetimi dahil) geçer.
- **Commit:** mesajlar Türkçe (`uygulama: …`, `kabuk: …`, `ui: …`, `e2e: …`, `çekirdek: …`, `docs: …`). Codex'in commit'lerine Claude satırı eklenmez.
- **Dokunulmayanlar:**
  - Plan commit'indeki dosyalar.
  - `crates/kalfa/capabilities/`, `crates/kalfa/tauri.conf.json`, `ui/src/ipc/`, `design/`.
  - `.env`, `Screenshot*.png`, kök dizindeki `tasks.png`.

## Dosya Yapısı

```
crates/kalfa-uygulama/
  src/e2e/mod.rs           ayarlar (KALFA_E2E), sahte iletişim kutuları
  src/e2e/senaryo.rs       senaryo adımları → zamanlı akış; gecikme damgaları
  src/e2e/saglayici.rs     anahtara göre davranan sahte EVREN; ortak şart durumu
  src/e2e/olcum.rs         kuyruk ve yoldaki veri sayaçları; Uygulama::e2e_olcumler
  src/e2e/kayit.rs         Uygulama::e2e_kayit_uret (uzun oturum)
  src/lib.rs, src/oturumlar.rs            (değişir)
  tests/e2e_parcalari.rs, tests/oturum_tablosu.rs (değişir)
crates/kalfa/
  src/e2e.rs               sahtelerin kurulumu, yetki, test komutları
  src/lib.rs, src/komutlar.rs, src/komut_adlari.rs, src/iletisim.rs, build.rs (değişir)
  tests/izinler.rs         (değişir)
crates/kalfa-core/src/aktarim.rs, tests/aktarim.rs   pencere 8 ms (değişir)
ui/
  src/e2e/kanca.ts, kanca.test.ts          test kancası
  src/markdown/vurgula-motor.test.ts       Shiki motorlarının karşılaştırması
  src/ana/anaDurum.svelte.ts, src/markdown/vurgula.ts, package.json, .gitignore,
  scripts/paket-denetimi.mjs               (değişir)
e2e/
  calistir.mjs             derleme ve testler
  yardimci/kalfa.mjs       Kalfa'yı açan, pencere değiştiren, kayıt okuyan yardımcılar
  testler/01-ilk-acilis.test.mjs, 02-konusma.test.mjs, 03-guvenlik.test.mjs,
  04-kabul.test.mjs
README.md, docs/GELISTIRME_DURUMU.md       (değişir)
```

---

### Task 1: `kalfa-uygulama`: sahte EVREN, sahte iletişim kutuları, sayaçlar ve kayıt üreteci

Test derlemesinin Tauri'siz parçaları. Hepsi `e2e` özelliğinin arkasındadır.

**Files:**
- Create: `crates/kalfa-uygulama/src/e2e/{mod,senaryo,saglayici,olcum,kayit}.rs`
- Modify: `crates/kalfa-uygulama/src/lib.rs`, `crates/kalfa-uygulama/src/oturumlar.rs`
- Test: `crates/kalfa-uygulama/tests/e2e_parcalari.rs`

**Interfaces:**
- Consumes (1c-2):
  - `evren::SaglayiciUretici` ve `arayuz::{Iletisim, KanalYazici}`.
  - `Uygulama`'nın `oturum_dizini`, `aktarim` ve `baglam(&Path)` üyeleri.
  - `kalfa_core::aktarim::{OlayKaynagi, KrediTutamaci::yolda}`, `kalfa_core::oturum::{OlayAlici, OlayButcesi, baslat_kimlikle}`.
- Produces (`kalfa_uygulama::e2e`):
  - **Ayarlar:** `E2eAyarlari { sart, anahtar, calisma_alani, onay, senaryolar }`, `E2eAyarlari::ortamdan()`, `E2eAyarlari::oku(&Path)`, `SartBaslangici::{Gerekli, Kabul}`, `AYAR_DEGISKENI = "KALFA_E2E"`.
  - **Sahteler:** `SahteUretici::yeni(&E2eAyarlari)` (`SaglayiciUretici` uygular); `SahteIletisim::yeni(&E2eAyarlari)` (`Iletisim` uygular).
  - **Senaryo tipleri:** `Senaryo { yanitlar }`, `Adim`, `Desen`, `DAMGA_ONEKI = "[[e2e:"`, `GECERSIZ_ANAHTAR`, `AGSIZ_ANAHTAR`.
  - **Sayaçlar:** `OlcumluKaynak(OlayAlici)`, `OlcumluYazici::yeni(Box<dyn KanalYazici>, KrediTutamaci)`, `olcumleri_sifirla()`, `Olcumler`.
  - **`Uygulama` yöntemleri:** `e2e_olcumler() -> Olcumler`, `e2e_kayit_uret(PathBuf, u32) -> Result<OturumId, KomutHatasi>`.

**Davranış notları (testler sınar):**
- **Senaryo seçimi:** kullanıcının son mesajının metniyle (baştaki ve sondaki boşluk atılır) seçilir. Eşleşmeyen mesaja "Yanıt: <mesaj>" döner.
- **Yanıt sırası:**
  - Turun `i`. model isteği `yanitlar[i]`'yi alır; bu, son kullanıcı mesajından sonraki asistan mesajı sayısıdır.
  - Yanıtlar bitmişse "Tamamlandı." döner.
  - `arac_sonucu` adımı turun son araç sonucunu "Araç sonucu: …" diye yazar.
- **Damgalı parçalar:**
  - Metinleri `[[e2e:<no>:<Unix ms>]]` biçimindedir; zaman parçanın üretildiği andır.
  - Zamanlama bir saate göre yapılır, gecikmeler birikmez.
  - `paralel` desen her adımda bir düşünce ve bir metin parçası verir.
- **`sel`:** tüketildiği kadar hızlı, bitmeyen düşünce parçası verir; akış bırakılınca durur.
- **Anahtar ve şart:**
  - `evren_llm_gecersiz` her isteğe 401 (`AnahtarGecersiz`), `evren_llm_agsiz` ağ hatası döner.
  - Şart durumu bütün sağlayıcılarda ortaktır. Kabul edilmemişken `akis` `SartKabulGerekli` döner.
- **Sayaçlar:**
  - Kuyruk her okumadan önce ölçülür; yalnızca okuma kuyruğu küçülttüğü için bu, iki okuma arasındaki en yüksek değerdir.
  - Yoldaki veri her paket yazılırken ölçülür.
  - Arayüz hiç "işledim" demezse yoldaki paket krediye eşit olur, kuyruk bütçeye kadar dolar, tur iptal edilir ve kayıtta `TurBitti { Iptal }` bulunur.
- **Kayıt üreteci:**
  - Turları gerçek oturum döngüsünden varsayılan yanıtla geçirir.
  - İlk mesaj `1. soru (<n> turluk kayıt)` olur; oturum listesinde başlık budur.
  - Sonunda oturumu kapatır; kayıt kilidi bırakılır.

- [ ] **Step 1: Başarısız testleri yaz**

`crates/kalfa-uygulama/tests/e2e_parcalari.rs`:

```rust
//! Uçtan uca test derlemesinin parçaları (1c tasarımı §11.4): ayarlar, sahte
//! sağlayıcının senaryoları, aktarım sayaçları ve uzun kayıt üreteci.
//! `cargo test -p kalfa-uygulama --features e2e` ile koşar.
#![cfg(feature = "e2e")]

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use kalfa_core::aktarim::{AktarimAyarlari, KanalMesaji};
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::protokol::{AktarimDurmaNedeni, BitisNedeni, Olay};
use kalfa_core::saglayici::Saglayici;
use kalfa_evren::tipler::{ChatMessage, ChatRequest, FunctionCall, ToolCall};
use kalfa_evren::{AkisOlayi, EvrenError};
use kalfa_uygulama::anahtar::AnahtarKasasi;
use kalfa_uygulama::arayuz::Iletisim;
use kalfa_uygulama::dizin::{kayit_yolu, oturum_dizini};
use kalfa_uygulama::e2e::{
    AGSIZ_ANAHTAR, DAMGA_ONEKI, E2eAyarlari, GECERSIZ_ANAHTAR, SahteIletisim, SahteUretici,
    SartBaslangici, olcumleri_sifirla,
};
use kalfa_uygulama::evren::SaglayiciUretici;
use kalfa_uygulama::test_destek::{SahteAcici, SahteKanal, SahtePencereler};
use kalfa_uygulama::{Bilesenler, Uygulama};
use serde_json::json;

fn ayarlar(deger: serde_json::Value) -> E2eAyarlari {
    serde_json::from_value(deger).unwrap()
}

fn saglayici(ayarlar: &E2eAyarlari) -> Arc<dyn Saglayici> {
    SahteUretici::yeni(ayarlar).uret("evren_llm_e2e").unwrap()
}

async fn olaylar(s: &dyn Saglayici, mesajlar: Vec<ChatMessage>) -> Vec<AkisOlayi> {
    let akis = s.akis(ChatRequest::new("glm-5.3", mesajlar)).await.unwrap();
    akis.olaylar.map(|o| o.unwrap()).collect().await
}

fn metin(olay: &AkisOlayi) -> &str {
    match olay {
        AkisOlayi::Metin(m) | AkisOlayi::Dusunce(m) => m,
        diger => panic!("metin beklenirdi: {diger:?}"),
    }
}

/// `[[e2e:<no>:<ms>]]` damgasının numarası ve zamanı.
fn damga(parca: &str) -> (u32, f64) {
    let ic = parca
        .strip_prefix(DAMGA_ONEKI)
        .and_then(|p| p.strip_suffix("]]"))
        .unwrap_or_else(|| panic!("damga değil: {parca}"));
    let (no, ms) = ic.split_once(':').unwrap();
    (no.parse().unwrap(), ms.parse().unwrap())
}

fn simdi_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
        * 1000.0
}

#[test]
fn ayarlar_dosyadan_okunur_bilinmeyen_alan_reddedilir() {
    let dizin = tempfile::tempdir().unwrap();
    let yol = dizin.path().join("e2e.json");
    let alan = dizin.path().join("alan");
    std::fs::write(
        &yol,
        json!({
            "sart": "kabul",
            "anahtar": "evren_llm_e2e",
            "calisma_alani": alan,
            "onay": true,
            "senaryolar": {
                "yaz": {"yanitlar": [[{"tur": "metin", "metin": "Tamam."}]]}
            }
        })
        .to_string(),
    )
    .unwrap();
    let a = E2eAyarlari::oku(&yol).unwrap();
    assert_eq!(a.sart, SartBaslangici::Kabul);
    assert_eq!(a.anahtar.as_deref(), Some("evren_llm_e2e"));
    assert_eq!(a.calisma_alani.as_deref(), Some(alan.as_path()));
    assert!(a.onay);
    assert_eq!(a.senaryolar["yaz"].yanitlar.len(), 1);

    std::fs::write(&yol, r#"{"sart": "kabul", "bilinmeyen": 1}"#).unwrap();
    assert!(E2eAyarlari::oku(&yol).is_err());

    let bos = E2eAyarlari::default();
    assert_eq!(bos.sart, SartBaslangici::Gerekli);
    assert!(!bos.onay);
}

#[tokio::test]
async fn sahte_iletisim_ayarlardaki_cevabi_verir() {
    let a = ayarlar(json!({"calisma_alani": "/tmp/alan", "onay": true}));
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

#[tokio::test]
async fn eslesmeyen_mesaja_yanit_doner() {
    let s = saglayici(&ayarlar(json!({"sart": "kabul"})));
    let o = olaylar(s.as_ref(), vec![ChatMessage::user("  merhaba  ")]).await;
    assert_eq!(
        o,
        vec![
            AkisOlayi::Metin("Yanıt: merhaba".into()),
            AkisOlayi::Bitti {
                neden: "stop".into()
            }
        ]
    );
}

#[tokio::test]
async fn senaryo_turun_isteklerine_sirayla_yanit_verir() {
    let a = ayarlar(json!({
        "sart": "kabul",
        "senaryolar": {"yaz": {"yanitlar": [
            [
                {"tur": "dusunce", "metin": "Düşünüyorum."},
                {"tur": "metin", "metin": "Yazıyorum."},
                {"tur": "arac", "ad": "dosya_yaz", "argumanlar": {"yol": "a.txt", "icerik": "x"}}
            ],
            [{"tur": "arac_sonucu"}]
        ]}}
    }));
    let s = saglayici(&a);

    let ilk = olaylar(s.as_ref(), vec![ChatMessage::user("yaz")]).await;
    assert_eq!(ilk[0], AkisOlayi::Dusunce("Düşünüyorum.".into()));
    assert_eq!(ilk[1], AkisOlayi::Metin("Yazıyorum.".into()));
    let AkisOlayi::AracCagrilari(cagrilar) = &ilk[2] else {
        panic!("araç çağrısı beklenirdi: {:?}", ilk[2]);
    };
    assert_eq!(cagrilar.len(), 1);
    assert_eq!(cagrilar[0].function.name, "dosya_yaz");
    let argumanlar: serde_json::Value =
        serde_json::from_str(&cagrilar[0].function.arguments).unwrap();
    assert_eq!(argumanlar, json!({"yol": "a.txt", "icerik": "x"}));
    assert_eq!(
        ilk[3],
        AkisOlayi::Bitti {
            neden: "tool_calls".into()
        }
    );

    let cagri = ToolCall {
        id: cagrilar[0].id.clone(),
        kind: "function".into(),
        function: FunctionCall {
            name: "dosya_yaz".into(),
            arguments: "{}".into(),
        },
    };
    let ikinci = olaylar(
        s.as_ref(),
        vec![
            ChatMessage::user("yaz"),
            ChatMessage::assistant(None, vec![cagri.clone()]),
            ChatMessage::tool(cagri.id.clone(), "Kullanıcı reddetti: başka ad kullan"),
        ],
    )
    .await;
    assert_eq!(
        ikinci,
        vec![
            AkisOlayi::Metin("Araç sonucu: Kullanıcı reddetti: başka ad kullan".into()),
            AkisOlayi::Bitti {
                neden: "stop".into()
            }
        ]
    );

    // Senaryonun yanıtları bitti: tur kapanır.
    let ucuncu = olaylar(
        s.as_ref(),
        vec![
            ChatMessage::user("yaz"),
            ChatMessage::assistant(None, vec![cagri.clone()]),
            ChatMessage::tool(cagri.id.clone(), "tamam"),
            ChatMessage::assistant(Some("Araç sonucu: tamam".into()), Vec::new()),
        ],
    )
    .await;
    assert_eq!(ucuncu[0], AkisOlayi::Metin("Tamamlandı.".into()));
}

#[tokio::test]
async fn damgali_desenler_sirali_ve_zamanli_parca_uretir() {
    let a = ayarlar(json!({
        "sart": "kabul",
        "senaryolar": {
            "duzenli": {"yanitlar": [[{"tur": "damgali", "sayi": 5,
                "desen": {"ad": "duzenli", "aralik_ms": 2}}]]},
            "yigin": {"yanitlar": [[{"tur": "damgali", "sayi": 12,
                "desen": {"ad": "yigin", "en_az": 5, "en_cok": 6, "aralik_ms": 2}}]]},
            "paralel": {"yanitlar": [[{"tur": "damgali", "sayi": 3,
                "desen": {"ad": "paralel", "aralik_ms": 2}}]]}
        }
    }));
    let s = saglayici(&a);
    let once = simdi_ms();

    let duzenli = olaylar(s.as_ref(), vec![ChatMessage::user("duzenli")]).await;
    assert_eq!(duzenli.len(), 6);
    let damgalar: Vec<(u32, f64)> = duzenli[..5].iter().map(|o| damga(metin(o))).collect();
    assert_eq!(
        damgalar.iter().map(|d| d.0).collect::<Vec<_>>(),
        [1, 2, 3, 4, 5]
    );
    assert!(damgalar.windows(2).all(|w| w[0].1 <= w[1].1));
    assert!(damgalar[0].1 >= once - 1.0 && damgalar[4].1 <= simdi_ms() + 1.0);

    let yigin = olaylar(s.as_ref(), vec![ChatMessage::user("yigin")]).await;
    assert_eq!(yigin.len(), 13, "12 parça ve bitiş");
    assert!(yigin[..12].iter().all(|o| matches!(o, AkisOlayi::Metin(_))));

    let paralel = olaylar(s.as_ref(), vec![ChatMessage::user("paralel")]).await;
    let turler: Vec<&str> = paralel[..6]
        .iter()
        .map(|o| match o {
            AkisOlayi::Dusunce(_) => "d",
            AkisOlayi::Metin(_) => "m",
            _ => "?",
        })
        .collect();
    assert_eq!(turler, ["d", "m", "d", "m", "d", "m"]);
}

#[tokio::test]
async fn sel_okundukca_akar_birakilinca_durur() {
    let a = ayarlar(json!({
        "sart": "kabul",
        "senaryolar": {"sel": {"yanitlar": [[{"tur": "sel", "parca_bayt": 1024}]]}}
    }));
    let s = saglayici(&a);
    let akis = s
        .akis(ChatRequest::new("glm-5.3", vec![ChatMessage::user("sel")]))
        .await
        .unwrap();
    let alinan: Vec<AkisOlayi> = akis.olaylar.take(500).map(|o| o.unwrap()).collect().await;
    assert_eq!(alinan.len(), 500);
    assert!(alinan.iter().all(|o| metin(o).len() == 1024));
}

#[tokio::test]
async fn anahtara_gore_hata_ve_ortak_sart_durumu() {
    let uretici = SahteUretici::yeni(&E2eAyarlari::default());
    let gecersiz = uretici.uret(GECERSIZ_ANAHTAR).unwrap();
    assert!(matches!(
        gecersiz.sart_durumu().await,
        Err(EvrenError::AnahtarGecersiz)
    ));
    let agsiz = uretici.uret(AGSIZ_ANAHTAR).unwrap();
    assert!(matches!(agsiz.sart_durumu().await, Err(EvrenError::Ag(_))));

    let a = uretici.uret("evren_llm_a").unwrap();
    let b = uretici.uret("evren_llm_b").unwrap();
    assert!(!a.sart_durumu().await.unwrap().accepted);
    let istek = ChatRequest::new("glm-5.3", vec![ChatMessage::user("merhaba")]);
    assert!(matches!(
        a.akis(istek.clone()).await,
        Err(EvrenError::SartKabulGerekli { surum: Some(1) })
    ));
    assert!(b.sart_kabul(2).await.is_err());
    b.sart_kabul(1).await.unwrap();
    assert!(a.sart_durumu().await.unwrap().accepted);
    assert!(a.akis(istek).await.is_ok());
}

/// Anahtarı kasada, şartları kabul edilmiş, açılmış uygulama.
async fn uygulama(a: &E2eAyarlari, veri: &std::path::Path) -> Arc<Uygulama> {
    let kasa = AnahtarKasasi::yeni(None);
    kasa.yaz("evren_llm_e2e");
    let u = Uygulama::yeni(Bilesenler {
        veri_dizini: veri.to_path_buf(),
        kasa,
        uretici: Arc::new(SahteUretici::yeni(a)),
        iletisim: Arc::new(SahteIletisim::yeni(a)),
        acici: Arc::new(SahteAcici::default()),
        pencereler: Arc::new(SahtePencereler::default()),
        aktarim: AktarimAyarlari::default(),
    });
    u.baslat().await;
    u
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn islenmeyen_paketler_krediyi_ve_kuyruk_butcesini_asmaz() {
    let veri = tempfile::tempdir().unwrap();
    let alan = tempfile::tempdir().unwrap();
    let a = ayarlar(json!({
        "sart": "kabul",
        "calisma_alani": alan.path(),
        "senaryolar": {"sel": {"yanitlar": [[{"tur": "sel", "parca_bayt": 4096}]]}}
    }));
    let u = uygulama(&a, veri.path()).await;
    let secim = u.calisma_alani_sec().await.unwrap();
    let oturum = u
        .oturum_baslat(secim.kimlik, "glm-5.3".into())
        .await
        .unwrap()
        .oturum;
    let (kanal, mut rx) = SahteKanal::yeni();
    olcumleri_sifirla();
    u.oturumu_dinle(oturum, kanal).unwrap();
    u.mesaj_gonder(oturum, "sel".into()).await.unwrap();

    // Arayüz paketleri işlemiyor: kredi dolar, çekirdek kuyruğu bütçeye
    // kadar dolar ve tur iptal edilir. Sonra arayüz işlemeye başlar; kuyruğun
    // son olayı `AktarimDurdu`dur.
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let mut durdu = None;
    loop {
        let mesaj = tokio::time::timeout(Duration::from_secs(20), rx.recv())
            .await
            .expect("kanal mesajı gelmedi")
            .expect("kanal kapandı");
        let KanalMesaji::Olaylar { no, zarflar } = mesaj else {
            break;
        };
        for z in zarflar {
            if let Olay::AktarimDurdu { neden } = z.yuk {
                durdu = Some(neden);
            }
        }
        u.paketi_isledim(oturum, no).unwrap();
    }
    assert_eq!(durdu, Some(AktarimDurmaNedeni::TuketiciYetismedi));
    let kayit = kayit_yolu(&oturum_dizini(veri.path()), oturum);
    let iptal = OturumKaydi::oku(&kayit).unwrap().iter().any(|z| {
        matches!(
            z.yuk,
            KayitOgesi::TurBitti {
                neden: BitisNedeni::Iptal
            }
        )
    });
    assert!(iptal, "kayıtta iptal edilen tur yok");

    let o = u.e2e_olcumler();
    assert!(o.yolda_paket_en_cok <= o.kredi_paket, "{o:?}");
    assert!(o.yolda_bayt_en_cok <= o.kredi_bayt, "{o:?}");
    assert!(o.kuyruk_olay_en_cok <= o.butce_olay, "{o:?}");
    assert!(o.kuyruk_bayt_en_cok <= o.butce_bayt, "{o:?}");
    assert!(
        o.kuyruk_bayt_en_cok > o.butce_bayt / 2,
        "kuyruk dolmadı: {o:?}"
    );
    assert_eq!(o.yolda_paket_en_cok, o.kredi_paket, "{o:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn kayit_ureteci_istenen_sayida_tur_yazar() {
    let veri = tempfile::tempdir().unwrap();
    let alan = tempfile::tempdir().unwrap();
    let a = ayarlar(json!({"sart": "kabul", "calisma_alani": alan.path()}));
    let u = uygulama(&a, veri.path()).await;

    let oturum = u
        .e2e_kayit_uret(alan.path().to_path_buf(), 25)
        .await
        .unwrap();
    let kayitlar = OturumKaydi::oku(&kayit_yolu(&oturum_dizini(veri.path()), oturum)).unwrap();
    let bitenler = kayitlar
        .iter()
        .filter(|z| {
            matches!(
                z.yuk,
                KayitOgesi::TurBitti {
                    neden: BitisNedeni::Normal
                }
            )
        })
        .count();
    assert_eq!(bitenler, 25);

    // Kayıt kilidi bırakıldı: oturum kayıttan açılır; son 20 tur gelir.
    let acilis = u.oturum_ac(oturum).await.unwrap();
    assert!(acilis.salt_okunur.is_none());
    assert!(acilis.gecmis.daha_eski_var);
}
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa-uygulama --features e2e --locked --test e2e_parcalari`
Expected: FAIL — `unresolved import kalfa_uygulama::e2e`.

- [ ] **Step 3: Modülü bağla**

`crates/kalfa-uygulama/src/lib.rs`'de `#[cfg(feature = "test-destek")]` satırının hemen üstüne:

```rust
#[cfg(feature = "e2e")]
pub mod e2e;
```

`crates/kalfa-uygulama/src/oturumlar.rs`'de `oturumu_dinle` içinde şu iki satırı:

```rust
        let (paketleyici, kredi) = Paketleyici::yeni(alici, self.aktarim);
        let gorev = tokio::spawn(kopru::calistir(paketleyici, yazici));
```

şununla değiştir:

```rust
        // Uçtan uca test derlemesi kuyruğu ve yoldaki veriyi ölçer (1c
        // tasarımı §11.4).
        #[cfg(feature = "e2e")]
        let alici = crate::e2e::OlcumluKaynak(alici);
        let (paketleyici, kredi) = Paketleyici::yeni(alici, self.aktarim);
        #[cfg(feature = "e2e")]
        let yazici: Box<dyn KanalYazici> =
            Box::new(crate::e2e::OlcumluYazici::yeni(yazici, kredi.clone()));
        let gorev = tokio::spawn(kopru::calistir(paketleyici, yazici));
```

Aynı dosyada `fn baglam(&self, kok: &Path)` imzasını `pub(crate) fn baglam(&self, kok: &Path)` yap; kayıt üreteci kullanır.

- [ ] **Step 4: Modülleri yaz**

`crates/kalfa-uygulama/src/e2e/mod.rs`:

```rust
//! Uçtan uca test derlemesinin parçaları (1c tasarımı §11.4): senaryolu sahte
//! sağlayıcı, sahte yerel iletişim kutuları, aktarım sayaçları ve uzun kayıt
//! üreteci. Yalnızca `e2e` özelliğiyle derlenir; sürüm derlemesine girmez.
//!
//! Test koşucusu `KALFA_E2E` ortam değişkeniyle bir JSON ayar dosyası verir.

mod kayit;
mod olcum;
mod saglayici;
mod senaryo;

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::Deserialize;

use crate::arayuz::Iletisim;

pub use olcum::{Olcumler, OlcumluKaynak, OlcumluYazici, olcumleri_sifirla};
pub use saglayici::{AGSIZ_ANAHTAR, GECERSIZ_ANAHTAR, SahteUretici};
pub use senaryo::{Adim, DAMGA_ONEKI, Desen, Senaryo};

/// Ayar dosyasının yolunu taşıyan ortam değişkeni.
pub const AYAR_DEGISKENI: &str = "KALFA_E2E";

/// Test koşucusunun verdiği ayarlar.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct E2eAyarlari {
    /// Sahte EVREN'in açılıştaki şart durumu.
    #[serde(default)]
    pub sart: SartBaslangici,
    /// Açılışta kasaya konan anahtar; yoksa şart penceresi anahtar ister.
    #[serde(default)]
    pub anahtar: Option<String>,
    /// Klasör seçicinin döneceği çalışma alanı; yoksa seçimden vazgeçilir.
    #[serde(default)]
    pub calisma_alani: Option<PathBuf>,
    /// Yerel onay kutularının cevabı (izin kipi yükseltme, bağlantı açma).
    #[serde(default)]
    pub onay: bool,
    /// Kullanıcı mesajının metniyle seçilen senaryolar. Eşleşmeyen mesaja
    /// "Yanıt: <mesaj>" döner.
    #[serde(default)]
    pub senaryolar: HashMap<String, Senaryo>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SartBaslangici {
    /// Şartlar kabul edilmemiş; ilk açılışta metin gösterilir.
    #[default]
    Gerekli,
    /// Şartlar daha önce kabul edilmiş.
    Kabul,
}

impl E2eAyarlari {
    pub fn oku(yol: &Path) -> io::Result<Self> {
        let metin = std::fs::read(yol)?;
        serde_json::from_slice(&metin).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// `KALFA_E2E` tanımlıysa ayar dosyasını okur, değilse varsayılanları
    /// döner.
    pub fn ortamdan() -> io::Result<Self> {
        match std::env::var_os(AYAR_DEGISKENI) {
            Some(yol) => Self::oku(Path::new(&yol)),
            None => Ok(Self::default()),
        }
    }
}

/// Yerel iletişim kutularının yerine geçer: WebDriver işletim sisteminin
/// kutularını süremez.
pub struct SahteIletisim {
    calisma_alani: Option<PathBuf>,
    onay: bool,
}

impl SahteIletisim {
    pub fn yeni(ayarlar: &E2eAyarlari) -> Self {
        Self {
            calisma_alani: ayarlar.calisma_alani.clone(),
            onay: ayarlar.onay,
        }
    }
}

#[async_trait]
impl Iletisim for SahteIletisim {
    async fn klasor_sec(&self) -> Option<PathBuf> {
        self.calisma_alani.clone()
    }

    async fn onayla(&self, _baslik: &str, _mesaj: &str, _tamam: &str, _vazgec: &str) -> bool {
        self.onay
    }
}
```

`crates/kalfa-uygulama/src/e2e/senaryo.rs`:

```rust
//! Sahte sağlayıcının senaryoları: bir model isteğine verilecek yanıtın
//! adımları ve bunlardan üretilen zamanlı akış.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::stream;
use kalfa_evren::tipler::{FunctionCall, ToolCall};
use kalfa_evren::{AkisKanali, AkisOlayi};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::time::Instant;

/// Gecikme ölçümündeki damganın başı. Damga `[[e2e:<no>:<ms>]]` biçimindedir;
/// `<ms>` parçanın üretildiği anın Unix zamanıdır (milisaniye).
pub const DAMGA_ONEKI: &str = "[[e2e:";

static CAGRI_SAYACI: AtomicU64 = AtomicU64::new(1);

/// Bir kullanıcı mesajına verilecek yanıtlar. Turun `i`. model isteği
/// `yanitlar[i]`'yi alır: araç sonuçlarından sonra gelen istek bir sonrakini.
/// Yanıtlar bitmişse tur "Tamamlandı." ile biter.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Senaryo {
    pub yanitlar: Vec<Vec<Adim>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "tur", rename_all = "snake_case", deny_unknown_fields)]
pub enum Adim {
    /// Metin parçası.
    Metin { metin: String },
    /// Düşünce parçası.
    Dusunce { metin: String },
    /// Araç çağrısı. Yanıtın bütün çağrıları sonda tek olayda verilir ve
    /// yanıt `tool_calls` nedeniyle biter.
    Arac {
        ad: String,
        argumanlar: serde_json::Value,
    },
    /// Sonraki adımdan önce bekler.
    Bekle { ms: u64 },
    /// Turun son araç sonucunu "Araç sonucu: …" diye yazar. Ret gerekçesinin
    /// modele gittiği böyle görülür.
    AracSonucu,
    /// Gecikme ölçümü için zaman damgalı parçalar (1c tasarımı §11.4).
    Damgali { desen: Desen, sayi: u32 },
    /// Tüketildiği kadar hızlı, bitmeyen düşünce akışı (kabul kapısı 2). Tur
    /// iptal edilince durur. Düşünce paneli kapalıyken çizilmez: arayüz
    /// yeniden işlemeye başlayınca biriken megabaytları çizmekle uğraşmaz.
    Sel { parca_bayt: usize },
}

/// Damgalı parçaların zamanlaması.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "ad", rename_all = "snake_case", deny_unknown_fields)]
pub enum Desen {
    /// `aralik_ms`'de bir metin parçası.
    Duzenli { aralik_ms: u64 },
    /// `en_az`–`en_cok` parçalık yığınlar; yığınların arası `aralik_ms`.
    Yigin {
        en_az: u32,
        en_cok: u32,
        aralik_ms: u64,
    },
    /// Her `aralik_ms`'de bir düşünce ve bir metin parçası.
    Paralel { aralik_ms: u64 },
}

/// Şimdiki Unix zamanı (milisaniye).
pub fn simdi_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64() * 1000.0)
}

fn damga(no: u32) -> String {
    format!("{DAMGA_ONEKI}{no}:{:.3}]]", simdi_ms())
}

/// Adımları akışa çevirir. Üretici ayrı bir görevde çalışır ve akış
/// okundukça ilerler; akış bırakılınca (tur iptal) durur.
pub fn akis(adimlar: Vec<Adim>, son_arac_sonucu: Option<String>) -> AkisKanali {
    let (tx, rx) = mpsc::channel::<AkisOlayi>(1);
    tokio::spawn(async move {
        let _ = uret(adimlar, son_arac_sonucu, &tx).await;
    });
    Box::pin(stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|olay| (Ok(olay), rx))
    }))
}

/// Alıcı gitmişse (akış bırakıldı) `Err` döner.
struct Gonderici<'a>(&'a mpsc::Sender<AkisOlayi>);

impl Gonderici<'_> {
    async fn gonder(&self, olay: AkisOlayi) -> Result<(), ()> {
        self.0.send(olay).await.map_err(|_| ())
    }
}

async fn uret(
    adimlar: Vec<Adim>,
    son_arac_sonucu: Option<String>,
    tx: &mpsc::Sender<AkisOlayi>,
) -> Result<(), ()> {
    let g = Gonderici(tx);
    let mut cagrilar = Vec::new();
    for adim in adimlar {
        match adim {
            Adim::Metin { metin } => g.gonder(AkisOlayi::Metin(metin)).await?,
            Adim::Dusunce { metin } => g.gonder(AkisOlayi::Dusunce(metin)).await?,
            Adim::Arac { ad, argumanlar } => cagrilar.push(ToolCall {
                id: format!("e2e-cagri-{}", CAGRI_SAYACI.fetch_add(1, Ordering::Relaxed)),
                kind: "function".into(),
                function: FunctionCall {
                    name: ad,
                    arguments: argumanlar.to_string(),
                },
            }),
            Adim::Bekle { ms } => tokio::time::sleep(Duration::from_millis(ms)).await,
            Adim::AracSonucu => {
                let sonuc = son_arac_sonucu.clone().unwrap_or_else(|| "yok".into());
                g.gonder(AkisOlayi::Metin(format!("Araç sonucu: {sonuc}")))
                    .await?;
            }
            Adim::Damgali { desen, sayi } => damgali(&g, desen, sayi).await?,
            Adim::Sel { parca_bayt } => {
                let parca = "x".repeat(parca_bayt.max(1));
                loop {
                    g.gonder(AkisOlayi::Dusunce(parca.clone())).await?;
                    tokio::task::yield_now().await;
                }
            }
        }
    }
    let neden = if cagrilar.is_empty() {
        "stop"
    } else {
        g.gonder(AkisOlayi::AracCagrilari(cagrilar)).await?;
        "tool_calls"
    };
    g.gonder(AkisOlayi::Bitti {
        neden: neden.into(),
    })
    .await
}

/// Parçalar bir saate göre zamanlanır: gecikmeler birikmez.
async fn damgali(g: &Gonderici<'_>, desen: Desen, sayi: u32) -> Result<(), ()> {
    let baslangic = Instant::now();
    let mut no = 0;
    let mut adim = 0u64;
    while no < sayi {
        let (aralik, yigin, paralel) = match desen {
            Desen::Duzenli { aralik_ms } => (aralik_ms, 1, false),
            Desen::Yigin {
                en_az,
                en_cok,
                aralik_ms,
            } => {
                let en_az = en_az.max(1);
                let genislik = en_cok.saturating_sub(en_az) + 1;
                (aralik_ms, en_az + (adim as u32 % genislik), false)
            }
            Desen::Paralel { aralik_ms } => (aralik_ms, 1, true),
        };
        tokio::time::sleep_until(baslangic + Duration::from_millis(aralik * adim)).await;
        for _ in 0..yigin {
            if no >= sayi {
                break;
            }
            no += 1;
            if paralel {
                g.gonder(AkisOlayi::Dusunce(damga(no))).await?;
            }
            g.gonder(AkisOlayi::Metin(damga(no))).await?;
        }
        adim += 1;
    }
    Ok(())
}
```

`crates/kalfa-uygulama/src/e2e/saglayici.rs`:

```rust
//! Senaryolu sahte EVREN. Anahtara göre davranır; şart durumu uygulamanın
//! bütün sağlayıcılarında ortaktır.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use kalfa_core::saglayici::{Saglayici, SaglayiciAkisi};
use kalfa_evren::tipler::{ChatRequest, Role, TermsStatus, TermsText};
use kalfa_evren::{EvrenError, YanitMeta};

use super::senaryo::{self, Adim, Senaryo};
use super::{E2eAyarlari, SartBaslangici};
use crate::evren::SaglayiciUretici;

/// Bu anahtarla her istek 401 döner (arayüzün sahte arka ucundaki adla aynı).
pub const GECERSIZ_ANAHTAR: &str = "evren_llm_gecersiz";
/// Bu anahtarla EVREN'e ulaşılamaz.
pub const AGSIZ_ANAHTAR: &str = "evren_llm_agsiz";

const SART_SURUMU: u32 = 1;
const SART_METNI: &str =
    "# Sahte kullanım şartları\n\nBu metin uçtan uca test derlemesinin sahte EVREN'inden gelir.";

struct SahteEvren {
    senaryolar: HashMap<String, Senaryo>,
    sart_kabul: Mutex<bool>,
    istek_sayaci: AtomicU64,
}

impl SahteEvren {
    fn kabul(&self) -> MutexGuard<'_, bool> {
        self.sart_kabul.lock().unwrap_or_else(|z| z.into_inner())
    }
}

/// Anahtardan sahte sağlayıcı üretir.
pub struct SahteUretici(Arc<SahteEvren>);

impl SahteUretici {
    pub fn yeni(ayarlar: &E2eAyarlari) -> Self {
        Self(Arc::new(SahteEvren {
            senaryolar: ayarlar.senaryolar.clone(),
            sart_kabul: Mutex::new(ayarlar.sart == SartBaslangici::Kabul),
            istek_sayaci: AtomicU64::new(1),
        }))
    }
}

impl SaglayiciUretici for SahteUretici {
    fn uret(&self, anahtar: &str) -> Result<Arc<dyn Saglayici>, EvrenError> {
        Ok(Arc::new(SahteSaglayici {
            evren: self.0.clone(),
            anahtar: anahtar.to_string(),
        }))
    }
}

struct SahteSaglayici {
    evren: Arc<SahteEvren>,
    anahtar: String,
}

impl SahteSaglayici {
    fn denetle(&self) -> Result<(), EvrenError> {
        match self.anahtar.as_str() {
            GECERSIZ_ANAHTAR => Err(EvrenError::AnahtarGecersiz),
            AGSIZ_ANAHTAR => Err(EvrenError::Ag("sahte EVREN'e ulaşılamıyor".into())),
            _ => Ok(()),
        }
    }
}

/// İstekteki son kullanıcı mesajı, turdaki model isteğinin sırası (0'dan) ve
/// turun son araç sonucu.
fn istek_bilgisi(istek: &ChatRequest) -> (String, usize, Option<String>) {
    let Some(son) = istek.messages.iter().rposition(|m| m.role == Role::User) else {
        return (String::new(), 0, None);
    };
    let sonrasi = &istek.messages[son + 1..];
    let sira = sonrasi.iter().filter(|m| m.role == Role::Assistant).count();
    let arac = sonrasi
        .iter()
        .rev()
        .find(|m| m.role == Role::Tool)
        .and_then(|m| m.content.clone());
    let metin = istek.messages[son].content.clone().unwrap_or_default();
    (metin.trim().to_string(), sira, arac)
}

#[async_trait]
impl Saglayici for SahteSaglayici {
    async fn akis(&self, istek: ChatRequest) -> Result<SaglayiciAkisi, EvrenError> {
        self.denetle()?;
        if !*self.evren.kabul() {
            return Err(EvrenError::SartKabulGerekli {
                surum: Some(SART_SURUMU),
            });
        }
        let (metin, sira, arac) = istek_bilgisi(&istek);
        let adimlar = match self.evren.senaryolar.get(&metin) {
            Some(senaryo) => senaryo.yanitlar.get(sira).cloned().unwrap_or_else(|| {
                vec![Adim::Metin {
                    metin: "Tamamlandı.".into(),
                }]
            }),
            None => vec![Adim::Metin {
                metin: format!("Yanıt: {metin}"),
            }],
        };
        let no = self.evren.istek_sayaci.fetch_add(1, Ordering::Relaxed);
        Ok(SaglayiciAkisi {
            meta: YanitMeta {
                istek_id: Some(format!("e2e-istek-{no}")),
                yonlendirilen_model: Some(istek.model.clone()),
                kredi: None,
            },
            olaylar: senaryo::akis(adimlar, arac),
        })
    }

    async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError> {
        self.denetle()?;
        Ok(TermsStatus {
            current_version: SART_SURUMU,
            is_material: true,
            accepted: *self.evren.kabul(),
            doc_path: None,
        })
    }

    async fn sart_metni(&self) -> Result<TermsText, EvrenError> {
        self.denetle()?;
        Ok(TermsText {
            version: SART_SURUMU,
            content: SART_METNI.into(),
            doc_path: None,
        })
    }

    async fn sart_kabul(&self, surum: u32) -> Result<(), EvrenError> {
        self.denetle()?;
        if surum != SART_SURUMU {
            return Err(EvrenError::SartKabulGerekli {
                surum: Some(SART_SURUMU),
            });
        }
        *self.evren.kabul() = true;
        Ok(())
    }
}
```

`crates/kalfa-uygulama/src/e2e/olcum.rs`:

```rust
//! Aktarım sayaçları (1c tasarımı §11.4, kabul kapısı 2): arayüzün "işledim"
//! demediği veri ve çekirdek olay kuyruğu en çok ne kadar büyüdü?
//!
//! Kuyruk yalnızca paketleyici okuyunca küçülür; bu yüzden her okumadan önce
//! bakılan değer iki okuma arasındaki en yüksek değerdir. Yoldaki veri her
//! paket gönderilirken ölçülür.

use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};

use kalfa_core::aktarim::{KanalMesaji, KrediTutamaci, OlayKaynagi};
use kalfa_core::oturum::{OlayAlici, OlayButcesi};
use kalfa_core::protokol::{Olay, Zarf};
use serde::Serialize;

use crate::Uygulama;
use crate::arayuz::KanalYazici;

static YOLDA_PAKET: AtomicUsize = AtomicUsize::new(0);
static YOLDA_BAYT: AtomicUsize = AtomicUsize::new(0);
static KUYRUK_OLAY: AtomicUsize = AtomicUsize::new(0);
static KUYRUK_BAYT: AtomicUsize = AtomicUsize::new(0);

/// Süreç başladığından (ya da son sıfırlamadan) beri görülen en yüksek
/// değerler ve karşılaştırılacakları sınırlar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Olcumler {
    pub yolda_paket_en_cok: usize,
    pub yolda_bayt_en_cok: usize,
    pub kuyruk_olay_en_cok: usize,
    pub kuyruk_bayt_en_cok: usize,
    pub kredi_paket: usize,
    pub kredi_bayt: usize,
    pub butce_olay: usize,
    pub butce_bayt: usize,
}

pub fn olcumleri_sifirla() {
    for sayac in [&YOLDA_PAKET, &YOLDA_BAYT, &KUYRUK_OLAY, &KUYRUK_BAYT] {
        sayac.store(0, Ordering::SeqCst);
    }
}

impl Uygulama {
    pub fn e2e_olcumler(&self) -> Olcumler {
        let butce = OlayButcesi::default();
        Olcumler {
            yolda_paket_en_cok: YOLDA_PAKET.load(Ordering::SeqCst),
            yolda_bayt_en_cok: YOLDA_BAYT.load(Ordering::SeqCst),
            kuyruk_olay_en_cok: KUYRUK_OLAY.load(Ordering::SeqCst),
            kuyruk_bayt_en_cok: KUYRUK_BAYT.load(Ordering::SeqCst),
            kredi_paket: self.aktarim.kredi_paket,
            kredi_bayt: self.aktarim.kredi_bayt,
            butce_olay: butce.azami_olay(),
            butce_bayt: butce.azami_bayt(),
        }
    }
}

/// Olay kuyruğunu okumadan önce doluluğunu kaydeder.
pub struct OlcumluKaynak(pub OlayAlici);

impl OlayKaynagi for OlcumluKaynak {
    fn recv(&mut self) -> impl Future<Output = Option<Zarf<Olay>>> + Send {
        KUYRUK_OLAY.fetch_max(self.0.bekleyen_olay_sayisi(), Ordering::SeqCst);
        KUYRUK_BAYT.fetch_max(self.0.bekleyen_bayt(), Ordering::SeqCst);
        self.0.recv()
    }
}

/// Paketi kanala yazmadan önce yoldaki veriyi kaydeder. Paketleyici paketi
/// yola, `sonraki` dönmeden ekler.
pub struct OlcumluYazici {
    ic: Box<dyn KanalYazici>,
    kredi: KrediTutamaci,
}

impl OlcumluYazici {
    pub fn yeni(ic: Box<dyn KanalYazici>, kredi: KrediTutamaci) -> Self {
        Self { ic, kredi }
    }
}

impl KanalYazici for OlcumluYazici {
    fn gonder(&self, mesaj: KanalMesaji) -> Result<(), String> {
        let (paket, bayt) = self.kredi.yolda();
        YOLDA_PAKET.fetch_max(paket, Ordering::SeqCst);
        YOLDA_BAYT.fetch_max(bayt, Ordering::SeqCst);
        self.ic.gonder(mesaj)
    }
}
```

`crates/kalfa-uygulama/src/e2e/kayit.rs`:

```rust
//! Uzun oturum ölçümü için kayıt üreteci (1c tasarımı §11.4, kabul kapısı 4).
//! Turlar gerçek oturum döngüsünden sahte sağlayıcının varsayılan yanıtıyla
//! geçer; kayıt uygulamanın kendi yazdığı biçimdedir.

use std::path::PathBuf;

use kalfa_core::kimlik::OturumId;
use kalfa_core::oturum::{self, OturumAyarlari};
use kalfa_core::protokol::{Komut, Olay};

use crate::hata::KomutHatasi;
use crate::{Uygulama, dizin, modeller};

impl Uygulama {
    /// `kok` çalışma alanında `tur` turluk bir oturum kaydı üretir ve oturumu
    /// kapatır. Şartlar kabul edilmiş olmalıdır.
    pub async fn e2e_kayit_uret(&self, kok: PathBuf, tur: u32) -> Result<OturumId, KomutHatasi> {
        let baglam = self.baglam(&kok)?;
        std::fs::create_dir_all(&self.oturum_dizini)?;
        let oturum = OturumId::yeni();
        let mut ayarlar = OturumAyarlari::yeni(modeller::VARSAYILAN_MODEL, &kok);
        ayarlar.kayit_yolu = Some(dizin::kayit_yolu(&self.oturum_dizini, oturum));
        let (tutamac, mut alici) = oturum::baslat_kimlikle(oturum, ayarlar, baglam)?;
        for i in 1..=tur {
            // Oturum listesindeki başlık ilk mesajdır; kayıtlar böyle ayrılır.
            let metin = format!("{i}. soru ({tur} turluk kayıt)");
            tutamac.gonder(Komut::KullaniciMesaji { metin }).await?;
            loop {
                match alici.recv().await.map(|z| z.yuk) {
                    Some(Olay::Tamamlandi { .. }) => break,
                    Some(_) => {}
                    None => return Err(KomutHatasi::ic("Kayıt üretilirken oturum kapandı.")),
                }
            }
        }
        drop(alici);
        tutamac.kapanmasini_bekle().await;
        Ok(oturum)
    }
}
```

- [ ] **Step 5: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa-uygulama --features e2e --locked --test e2e_parcalari`
Expected: PASS — 9 test. Ardından Global Constraints'teki Rust denetimleri (`e2e` özellikli olanlar dahil). `cargo tree -p kalfa-uygulama -e normal --features e2e | grep -E "tauri|wry"` boş çıkar.

- [ ] **Step 6: Commit**

```bash
git add crates/kalfa-uygulama
git commit -m "uygulama: uçtan uca test derlemesinin sahte EVREN'i, sayaçları ve kayıt üreteci"
```

---

### Task 2: Çıkışta kapanan oturumların beklenmesi

Ana pencere kapanınca oturumlar kapanır. Süren tur iptal edilip kayda yazılır (1c tasarımı §5.6). Son pencere kapanınca Tauri süreci sonlandırır ve kapanış görevleri bu çıkışla yarışır. Yarış çoğunlukla kazanılır, ama yavaş bir diskte `TurBitti` kaydı yarım kalabilir.

Artık çıkış isteği, kapanan oturumlar bitene kadar ertelenir. Birim testi garantiyi sınar. 6. görevdeki pencere kapatma testi aynı yolu gerçek pencerede çalıştırır; yarış zamanlamaya bağlı olduğu için o test yarışı her koşuda yakalayamaz.

**Files:**
- Modify: `crates/kalfa-uygulama/src/oturumlar.rs`, `crates/kalfa/src/lib.rs`
- Test: `crates/kalfa-uygulama/tests/oturum_tablosu.rs`

**Interfaces:**
- Produces: `Uygulama::kapanan_oturum_var() -> bool`, `Uygulama::kapananlari_bekle()` (async).

- [ ] **Step 1: Başarısız testi yaz**

`crates/kalfa-uygulama/tests/oturum_tablosu.rs`'de `turu_iptal_etmek_tamamlandi_iptal_uretir` testinin hemen üstüne:

```rust
#[tokio::test]
async fn cikista_kapanan_oturumlarin_kaydi_tamamlanana_kadar_beklenir() {
    let o = ortam(vec![SahteYanit::Askida(vec![AkisOlayi::Metin(
        "yarım".into(),
    )])])
    .await;
    let oturum = o.yeni_oturum().await;
    let mut rx = o.dinle(oturum);
    o.t.uygulama
        .mesaj_gonder(oturum, "uzun iş".into())
        .await
        .unwrap();
    olay_bekle(&o.t, oturum, &mut rx, |x| {
        matches!(x, Olay::TokenParcasi { .. })
    })
    .await;
    assert!(!o.t.uygulama.kapanan_oturum_var());

    // Son pencere kapandı: süreç çıkmadan önce kapanışlar beklenir (1c
    // tasarımı §5.6); iptal edilen tur kayda yazılmış olur.
    let _ = o.t.uygulama.ana_pencere_gitti();
    assert!(o.t.uygulama.kapanan_oturum_var());
    o.t.uygulama.kapananlari_bekle().await;
    assert!(!o.t.uygulama.kapanan_oturum_var());
    let kayitlar = OturumKaydi::oku(&o.kayit(oturum)).unwrap();
    assert!(kayitlar.iter().any(|z| matches!(
        z.yuk,
        KayitOgesi::TurBitti {
            neden: BitisNedeni::Iptal
        }
    )));
}
```

- [ ] **Step 2: Testin başarısız olduğunu gör**

Run: `cargo test -p kalfa-uygulama --locked --test oturum_tablosu cikista`
Expected: FAIL — `no method named kapanan_oturum_var`.

- [ ] **Step 3: Yöntemleri yaz**

`crates/kalfa-uygulama/src/oturumlar.rs`'de `fn kapatmaya_basla` yönteminin hemen üstüne:

```rust
    /// Kapanmakta olan oturum var mı. Son pencere kapanınca süreç bunların
    /// kaydı tamamlanmadan çıkmaz (1c tasarımı §5.6).
    pub fn kapanan_oturum_var(&self) -> bool {
        !self.ic().kapaniyor.is_empty()
    }

    /// Kapanmakta olan bütün oturumların görevlerini bekler.
    pub async fn kapananlari_bekle(&self) {
        let kapananlar: Vec<(OturumId, OturumTutamaci)> = self
            .ic()
            .kapaniyor
            .iter()
            .map(|(oturum, tutamac)| (*oturum, tutamac.clone()))
            .collect();
        self.kapanislari_bekle(kapananlar).await;
    }
```

- [ ] **Step 4: Çıkışı ertele**

`crates/kalfa/src/lib.rs`'de `use tauri::{AppHandle, Builder, Manager, Runtime, WindowEvent};` satırını şununla değiştir:

```rust
use tauri::{AppHandle, Builder, ExitRequestApi, Manager, RunEvent, Runtime, WindowEvent};
```

`calistir`'ın sonundaki şu bloğu:

```rust
    if let Err(e) = komutlar::kur(kurucu).run(tauri::generate_context!()) {
        eprintln!("Kalfa çalıştırılamadı: {e}");
        std::process::exit(1);
    }
}
```

şununla değiştir:

```rust
    let app = match komutlar::kur(kurucu).build(tauri::generate_context!()) {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Kalfa çalıştırılamadı: {e}");
            std::process::exit(1);
        }
    };
    app.run(|app, olay| {
        if let RunEvent::ExitRequested { api, .. } = olay {
            cikisi_ertele(app, api);
        }
    });
}

/// Ana pencere kapanınca oturumları kapanır; süren tur iptal edilip kayda
/// yazılır (1c tasarımı §5.6). Son pencere kapandığında süreç bunlar bitmeden
/// çıkmaz; yoksa kayıt yarım kalır.
fn cikisi_ertele<R: Runtime>(app: &AppHandle<R>, api: ExitRequestApi) {
    let Some(durum) = app.try_state::<Arc<Uygulama>>() else {
        return;
    };
    if !durum.kapanan_oturum_var() {
        return;
    }
    api.prevent_exit();
    let uygulama = durum.inner().clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        uygulama.kapananlari_bekle().await;
        app.exit(0);
    });
}
```

Açıklama: `app.exit(0)` yeniden çıkış isteği üretir. O sırada kapanan oturum kalmadığı için süreç çıkar.

- [ ] **Step 5: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa-uygulama --locked --test oturum_tablosu`
Expected: PASS. Ardından Global Constraints'teki Rust denetimleri.

- [ ] **Step 6: Commit**

```bash
git add crates/kalfa-uygulama crates/kalfa/src/lib.rs
git commit -m "kabuk: süreç, kapanan oturumların kaydı tamamlanmadan çıkmaz"
```

---

### Task 3: Kabuk: `e2e` derlemesi

`kalfa` crate'i `e2e` özelliğiyle şunları kullanır:
- EVREN istemcisi yerine sahte sağlayıcı.
- Yerel iletişim kutuları yerine ayardaki cevaplar.
- Ana pencereye açılışta verilen yetkiyle dört test komutu.

**Files:**
- Create: `crates/kalfa/src/e2e.rs`
- Modify: `crates/kalfa/src/komut_adlari.rs`, `crates/kalfa/build.rs`, `crates/kalfa/src/komutlar.rs`, `crates/kalfa/src/lib.rs`, `crates/kalfa/src/iletisim.rs`
- Test: `crates/kalfa/tests/izinler.rs`

**Interfaces:**
- Consumes (Görev 1): `kalfa_uygulama::e2e::{E2eAyarlari, Olcumler, SahteIletisim, SahteUretici, olcumleri_sifirla}`, `Uygulama::{e2e_olcumler, e2e_kayit_uret}`.
- Produces:
  - `komut_adlari::E2E_KOMUTLARI = ["e2e_olcumler", "e2e_olcumleri_sifirla", "e2e_kayit_uret", "e2e_ana_pencereyi_kapat"]`.
  - `kalfa_kabuk::e2e::yetki() -> CapabilityBuilder` (yalnızca `e2e` özelliğiyle).
  - **Komutlar** (argüman adları snake_case):
    - `e2e_olcumler() -> Olcumler`
    - `e2e_olcumleri_sifirla()`
    - `e2e_kayit_uret({ tur }) -> OturumId`: ayardaki çalışma alanında kayıt üretir.
    - `e2e_ana_pencereyi_kapat()`: pencereyi kapatma düğmesi gibi kapatır.

**Davranış notları:**
- **Açılış (`e2e` derlemesi):**
  - `KALFA_E2E`'deki ayarlar okunur.
  - `anahtar` verilmişse kasaya konur.
  - Test komutlarının yetkisi (`e2e` adlı, yalnızca `ana` penceresi) eklenir.
  - Çalışma alanı yolu durum olarak saklanır.
- **Normal derleme:** test komutları kayıtlı değildir; çağrı ACL'de "not allowed" ile reddedilir.
- **Uygulama manifest'i:** `build.rs` test komutlarını yalnızca `CARGO_FEATURE_E2E` tanımlıyken ekler.
- **Pencereyi kapatma komutu:** WebDriver'ın pencere kapatması son pencerede süreci doğrudan sonlandırır; uygulamanın kapanış yolunu (`CloseRequested`, `Destroyed`, `ExitRequested`) sınamaz. Bu komut bu yüzden vardır.

- [ ] **Step 1: Başarısız testleri yaz**

`crates/kalfa/tests/izinler.rs`'de ilk `use` satırını şununla değiştir:

```rust
use kalfa_kabuk::komut_adlari::{ANA_PENCERE_KOMUTLARI, E2E_KOMUTLARI, SART_PENCERESI_KOMUTLARI};
```

`fn json(yol: &str) -> Value {` satırının hemen üstüne:

```rust
/// Test komutları normal derlemede kayıtlı değildir; hiçbir pencere
/// çağıramaz (1c tasarımı §11.4).
#[cfg(not(feature = "e2e"))]
#[test]
fn e2e_komutlari_normal_derlemede_yok() {
    let app = uygulama();
    let ana = pencere(&app, "ana");
    for komut in E2E_KOMUTLARI {
        let hata = get_ipc_response(&ana, istek(komut))
            .expect_err(komut)
            .to_string();
        assert!(hata.contains("not allowed"), "{komut}: {hata}");
    }
}

/// Uçtan uca test derlemesinde test komutları yalnızca ana pencereye açılır.
#[cfg(feature = "e2e")]
#[test]
fn e2e_komutlari_yalnizca_ana_pencereye_acilir() {
    use tauri::Manager;
    let app = uygulama();
    app.add_capability(kalfa_kabuk::e2e::yetki()).unwrap();
    let ana = pencere(&app, "ana");
    let sartlar = pencere(&app, "sartlar");
    for komut in E2E_KOMUTLARI {
        assert!(izinli(&ana, komut), "ana pencere {komut} çağıramadı");
        assert!(
            !izinli(&sartlar, komut),
            "şart penceresi {komut} çağırabildi"
        );
    }
}
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa --locked --test izinler`
Expected: FAIL — `unresolved import kalfa_kabuk::komut_adlari::E2E_KOMUTLARI`.

- [ ] **Step 3: Komut adları, manifest ve işleyici**

`crates/kalfa/src/komut_adlari.rs`'nin sonuna:

```rust

/// Uçtan uca test derlemesinin komutları (`e2e` özelliği). Yalnızca o
/// derlemede kayıtlıdır; yetkisi açılışta ana pencereye verilir.
pub const E2E_KOMUTLARI: [&str; 4] = [
    "e2e_olcumler",
    "e2e_olcumleri_sifirla",
    "e2e_kayit_uret",
    "e2e_ana_pencereyi_kapat",
];
```

`crates/kalfa/build.rs`'de `komutlar.sort_unstable();` satırının hemen üstüne:

```rust
    if std::env::var_os("CARGO_FEATURE_E2E").is_some() {
        komutlar.extend(E2E_KOMUTLARI);
    }
```

`crates/kalfa/src/komutlar.rs`'de `kur`'un açıklamasını ve işleyici listesinin sonunu şöyle yap (`generate_handler!` komut başına `#[cfg]` kabul eder):

```rust
/// Komut işleyicisini kurar. Liste `komut_adlari.rs` ile aynıdır
/// (tests/izinler.rs denetler); uçtan uca test derlemesi test komutlarını
/// ekler.
pub fn kur<R: Runtime>(kurucu: Builder<R>) -> Builder<R> {
    kurucu.invoke_handler(tauri::generate_handler![
        // … mevcut 22 komut değişmeden …
        sart_metnini_goster,
        #[cfg(feature = "e2e")]
        crate::e2e::e2e_olcumler,
        #[cfg(feature = "e2e")]
        crate::e2e::e2e_olcumleri_sifirla,
        #[cfg(feature = "e2e")]
        crate::e2e::e2e_kayit_uret,
        #[cfg(feature = "e2e")]
        crate::e2e::e2e_ana_pencereyi_kapat,
    ])
}
```

- [ ] **Step 4: Test derlemesinin kurulumu**

`crates/kalfa/src/e2e.rs`:

```rust
//! Uçtan uca test derlemesi (1c tasarımı §11.4). Yalnızca `e2e` özelliğiyle
//! derlenir: EVREN yerine senaryolu sahte sağlayıcı, yerel iletişim kutuları
//! yerine ayardaki cevaplar ve ana pencereye açılan test komutları.

use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

use kalfa_core::kimlik::OturumId;
use kalfa_uygulama::Uygulama;
use kalfa_uygulama::anahtar::AnahtarKasasi;
use kalfa_uygulama::arayuz::Iletisim;
use kalfa_uygulama::e2e::{E2eAyarlari, Olcumler, SahteIletisim, SahteUretici, olcumleri_sifirla};
use kalfa_uygulama::evren::SaglayiciUretici;
use kalfa_uygulama::hata::KomutHatasi;
use tauri::ipc::CapabilityBuilder;
use tauri::{AppHandle, Manager, Runtime, State};

use crate::komut_adlari::E2E_KOMUTLARI;
use crate::pencereler::ANA;

/// Test komutlarının yetkisi: yalnızca ana pencere. Capability dosyalarına
/// girmez; test derlemesi açılırken eklenir.
pub fn yetki() -> CapabilityBuilder {
    E2E_KOMUTLARI
        .iter()
        .fold(CapabilityBuilder::new("e2e").window(ANA), |yetki, komut| {
            yetki.permission(format!("allow-{}", komut.replace('_', "-")))
        })
}

pub(crate) struct E2eCalismaAlani(Option<PathBuf>);

/// EVREN'in ve yerel iletişim kutularının yerine geçen sahteler.
pub(crate) type Sahteler = (Arc<dyn SaglayiciUretici>, Arc<dyn Iletisim>);

/// Ayarları `KALFA_E2E`'den okur, yetkiyi ekler, anahtarı kasaya koyar ve
/// sahte sağlayıcıyla sahte iletişim kutularını döner.
pub(crate) fn kur<R: Runtime>(
    app: &AppHandle<R>,
    kasa: &AnahtarKasasi,
) -> Result<Sahteler, Box<dyn Error>> {
    let ayarlar = E2eAyarlari::ortamdan()?;
    if let Some(anahtar) = &ayarlar.anahtar {
        kasa.yaz(anahtar);
    }
    app.add_capability(yetki())?;
    app.manage(E2eCalismaAlani(ayarlar.calisma_alani.clone()));
    Ok((
        Arc::new(SahteUretici::yeni(&ayarlar)),
        Arc::new(SahteIletisim::yeni(&ayarlar)),
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn e2e_olcumler(u: State<'_, Arc<Uygulama>>) -> Result<Olcumler, KomutHatasi> {
    Ok(u.e2e_olcumler())
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn e2e_olcumleri_sifirla() -> Result<(), KomutHatasi> {
    olcumleri_sifirla();
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn e2e_kayit_uret(
    u: State<'_, Arc<Uygulama>>,
    alan: State<'_, E2eCalismaAlani>,
    tur: u32,
) -> Result<OturumId, KomutHatasi> {
    let kok = alan
        .0
        .clone()
        .ok_or_else(|| KomutHatasi::gecersiz("Ayarlarda çalışma alanı yok."))?;
    u.e2e_kayit_uret(kok, tur).await
}

/// Ana pencereyi kapatma düğmesi gibi kapatır (kabul kapısı 3): pencere
/// kapanma isteği ve yok olma olayları Kalfa'nın kapanış yolundan geçer.
/// WebDriver'ın pencere kapatması son pencerede süreci doğrudan sonlandırır;
/// o yolu sınamaz.
#[tauri::command(rename_all = "snake_case")]
pub(crate) async fn e2e_ana_pencereyi_kapat<R: Runtime>(
    app: AppHandle<R>,
) -> Result<(), KomutHatasi> {
    if let Some(pencere) = app.get_webview_window(ANA) {
        pencere
            .close()
            .map_err(|e| KomutHatasi::ic(e.to_string()))?;
    }
    Ok(())
}
```

`crates/kalfa/src/lib.rs`'nin son hâli; Görev 2'deki `cikisi_ertele` de içindedir:

```rust
//! Kalfa masaüstü uygulamasının Tauri kabuğu (1c tasarımı §3, §6). İnce
//! tutulur: pencereler, komutlar, kanal bağdaştırıcısı, yerel iletişim
//! kutuları. Mantık `kalfa-uygulama`'dadır.

#[cfg(feature = "e2e")]
pub mod e2e;
mod iletisim;
mod kanal;
pub mod komut_adlari;
pub mod komutlar;
mod pencereler;

use std::error::Error;
use std::sync::Arc;

use kalfa_core::aktarim::AktarimAyarlari;
use kalfa_uygulama::anahtar::{AnahtarDeposu, AnahtarKasasi, SistemDeposu};
use kalfa_uygulama::dizin::{VERI_DIZINI_DEGISKENI, veri_dizini};
use kalfa_uygulama::{Bilesenler, Uygulama};
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Builder, ExitRequestApi, Manager, RunEvent, Runtime, WindowEvent};

use crate::iletisim::TauriAcici;
use crate::pencereler::TauriPencereler;

/// `bellek` ise işletim sisteminin kimlik deposu kullanılmaz (uçtan uca
/// testler geliştiricinin gerçek anahtarına dokunmasın diye).
pub const ANAHTAR_DEPOSU_DEGISKENI: &str = "KALFA_ANAHTAR_DEPOSU";

pub fn calistir() {
    // Tek Kalfa süreci (1c tasarımı §5.2); ilk eklenti olmalı.
    let kurucu =
        tauri::Builder::default().plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            pencereler::one_getir(app);
        }));
    let kurucu = eklentiler(kurucu)
        .setup(|app| {
            let uygulama = kur(app.handle())?;
            app.manage(uygulama.clone());
            tauri::async_runtime::spawn(async move { uygulama.baslat().await });
            Ok(())
        })
        .on_page_load(|webview, yuk| {
            // Yeniden yükleme de bir sayfa yüklemesidir: ana pencereye bağlı
            // oturumlar kapanır, yeni sayfa son oturumu kayıttan açar (1c
            // tasarımı §5.6).
            if webview.label() == pencereler::ANA && yuk.event() == PageLoadEvent::Started {
                ana_pencere_gitti(webview.app_handle());
            }
        })
        .on_window_event(|pencere, olay| {
            if pencere.label() == pencereler::ANA && matches!(olay, WindowEvent::Destroyed) {
                ana_pencere_gitti(pencere.app_handle());
            }
        });
    let app = match komutlar::kur(kurucu).build(tauri::generate_context!()) {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Kalfa çalıştırılamadı: {e}");
            std::process::exit(1);
        }
    };
    app.run(|app, olay| {
        if let RunEvent::ExitRequested { api, .. } = olay {
            cikisi_ertele(app, api);
        }
    });
}

/// Ana pencere kapanınca oturumları kapanır; süren tur iptal edilip kayda
/// yazılır (1c tasarımı §5.6). Son pencere kapandığında süreç bunlar bitmeden
/// çıkmaz; yoksa kayıt yarım kalır.
fn cikisi_ertele<R: Runtime>(app: &AppHandle<R>, api: ExitRequestApi) {
    let Some(durum) = app.try_state::<Arc<Uygulama>>() else {
        return;
    };
    if !durum.kapanan_oturum_var() {
        return;
    }
    api.prevent_exit();
    let uygulama = durum.inner().clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        uygulama.kapananlari_bekle().await;
        app.exit(0);
    });
}

/// Webview'a açılmayan eklentiler: iletişim kutuları ve sistem tarayıcısı
/// yalnızca Rust'tan kullanılır. Hiçbir capability bu eklentilerin
/// komutlarına izin vermez (tests/izinler.rs denetler).
pub fn eklentiler<R: Runtime>(kurucu: Builder<R>) -> Builder<R> {
    kurucu
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
}

fn ana_pencere_gitti<R: Runtime>(app: &AppHandle<R>) {
    let Some(durum) = app.try_state::<Arc<Uygulama>>() else {
        return;
    };
    let uygulama = durum.inner().clone();
    let kapananlar = uygulama.ana_pencere_gitti();
    if !kapananlar.is_empty() {
        tauri::async_runtime::spawn(async move { uygulama.kapanislari_bekle(kapananlar).await });
    }
}

fn kur<R: Runtime>(app: &AppHandle<R>) -> Result<Arc<Uygulama>, Box<dyn Error>> {
    let veri = veri_dizini(
        &app.path().data_dir()?,
        std::env::var_os(VERI_DIZINI_DEGISKENI),
    );
    let kalici: Option<Box<dyn AnahtarDeposu>> =
        if std::env::var(ANAHTAR_DEPOSU_DEGISKENI).as_deref() == Ok("bellek") {
            None
        } else {
            SistemDeposu::ac()
                .ok()
                .map(|depo| Box::new(depo) as Box<dyn AnahtarDeposu>)
        };
    let kasa = AnahtarKasasi::yeni(kalici);
    #[cfg(not(feature = "e2e"))]
    let (uretici, iletisim) = evren_bilesenleri(app);
    #[cfg(feature = "e2e")]
    let (uretici, iletisim) = e2e::kur(app, &kasa)?;
    Ok(Uygulama::yeni(Bilesenler {
        veri_dizini: veri,
        kasa,
        uretici,
        iletisim,
        acici: Arc::new(TauriAcici(app.clone())),
        pencereler: Arc::new(TauriPencereler(app.clone())),
        aktarim: AktarimAyarlari::default(),
    }))
}

/// EVREN istemcisi ve işletim sisteminin iletişim kutuları. Uçtan uca test
/// derlemesi bunların yerine `e2e::kur`'un sahtelerini kullanır.
#[cfg(not(feature = "e2e"))]
fn evren_bilesenleri<R: Runtime>(
    app: &AppHandle<R>,
) -> (
    Arc<dyn kalfa_uygulama::evren::SaglayiciUretici>,
    Arc<dyn kalfa_uygulama::arayuz::Iletisim>,
) {
    let taban = std::env::var(kalfa_uygulama::evren::TABAN_DEGISKENI)
        .ok()
        .filter(|_| cfg!(debug_assertions))
        .unwrap_or_else(|| kalfa_evren::VARSAYILAN_TABAN.to_string());
    (
        Arc::new(kalfa_uygulama::evren::EvrenUretici { taban }),
        Arc::new(iletisim::TauriIletisim(app.clone())),
    )
}
```

`crates/kalfa/src/iletisim.rs`'de modül açıklamasının sonuna, ilk `use` satırından önce:

```rust
//!
//! Uçtan uca test derlemesi yerel kutuların yerine ayardaki cevapları kullanır
//! (`e2e`); orada `TauriIletisim` kullanılmaz.
#![cfg_attr(feature = "e2e", allow(dead_code))]
```

- [ ] **Step 5: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa --locked --test izinler` ve `cargo test -p kalfa --features e2e --locked --test izinler`
Expected: PASS — ikisinde de 6 test (her derleme kendi e2e testini koşar). Ardından Global Constraints'teki Rust denetimleri (`e2e` özellikli olanlar dahil).

- [ ] **Step 6: Commit**

```bash
git add crates/kalfa
git commit -m "kabuk: uçtan uca test derlemesi (sahte EVREN, test komutları)"
```

---

### Task 4: Arayüz: test kancası, paket denetimi ve Shiki motoru

**Files:**
- Create: `ui/src/e2e/kanca.ts`
- Modify: `ui/src/ana/anaDurum.svelte.ts`, `ui/package.json`, `ui/.gitignore`, `ui/scripts/paket-denetimi.mjs`, `ui/src/markdown/vurgula.ts`
- Test: `ui/src/e2e/kanca.test.ts`, `ui/src/markdown/vurgula-motor.test.ts`

**Interfaces:**
- Produces:
  - `window.__kalfaE2e: E2eKancasi`: `duraklat()`, `surdur()`, `gecikmeler(): number[]`, `sifirla()`, `bekleyen(): number`.
  - `ui/src/e2e/kanca.ts`: `sar(isleyici)`, `damgalar(mesaj)`, `kancayiKur()`.
  - `npm run build:e2e` → `ui/dist-e2e/`.
  - `vurgula.ts`: `TEMA` ve `DILLER` dışa açılır.

**Davranış notları:**
- **Gecikme:** damgalı parça duruma uygulanır. Ardından `requestAnimationFrame` ve bir `setTimeout(0)` sonrasında `Date.now() - damga` kaydedilir; bu, çizimden sonraki ilk andır.
- **Duraklatma:**
  - Kanal mesajları duruma uygulanmadan bekletilir; "işledim" gitmez, kredi dolar.
  - `surdur()` bekleyenleri geliş sırasıyla işler.
- **Kanca ve sürüm paketi:** kanca ilk dinlenen oturumla kurulur. Sürüm derlemesinde `import.meta.env.MODE` sabittir. Dal ve modül pakete girmez; paket denetimi `__kalfaE2e` ve `[[e2e:` izlerini arar.
- **Shiki motorları (1c tasarımı §13):**
  - JavaScript motoru seçilen 11 dilin örneklerinde Oniguruma'yla token token aynı sonucu verir.
  - Oniguruma'nın WASM'ı yalnızca testte yüklenir.

- [ ] **Step 1: Başarısız testleri yaz**

`ui/src/e2e/kanca.test.ts`:

```ts
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { KanalMesaji, Olay } from '../ipc/tipler';
import { damgalar, sar } from './kanca';

let sira = 0;
function paket(no: number, ...yukler: Olay[]): KanalMesaji {
  return {
    tip: 'Olaylar',
    no,
    zarflar: yukler.map((yuk) => ({ sema: 1, oturum: 'o-1', tur: 't-1', sira: ++sira, yuk })),
  };
}

const metin = (m: string): Olay => ({ tip: 'TokenParcasi', metin: m });
const dusunce = (m: string): Olay => ({ tip: 'DusunceParcasi', metin: m });

describe('damgalar', () => {
  it('metin ve düşünce parçalarındaki bütün damgaları okur', () => {
    const m = paket(
      1,
      metin('[[e2e:1:1000.5]][[e2e:2:1010]]'),
      dusunce('önce [[e2e:3:1020.25]] sonra'),
      { tip: 'Hata', mesaj: '[[e2e:4:1030]]' },
    );
    expect(damgalar(m)).toEqual([1000.5, 1010, 1020.25]);
  });

  it('damgasız ve kapanış mesajında boş döner', () => {
    expect(damgalar(paket(1, metin('merhaba')))).toEqual([]);
    expect(damgalar({ tip: 'Kapandi', no: 2 })).toEqual([]);
  });
});

describe('sar', () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'Date'] });
    vi.setSystemTime(10_000);
    // jsdom'un çizim döngüsü sahte saatle ilerlemez; kare 16 ms sayılır.
    vi.stubGlobal('requestAnimationFrame', (f: FrameRequestCallback) =>
      setTimeout(() => f(Date.now()), 16),
    );
  });

  afterEach(() => {
    window.__kalfaE2e?.surdur();
    window.__kalfaE2e?.sifirla();
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it('kancayı kurar ve damgalı parçanın çizimden sonraki gecikmesini ölçer', () => {
    const islenen: number[] = [];
    const dinle = sar((m) => islenen.push(m.no));
    const kanca = window.__kalfaE2e!;

    dinle(paket(1, metin('[[e2e:1:9990]]'), metin('[[e2e:2:9980]]')));
    expect(islenen).toEqual([1]);
    expect(kanca.gecikmeler()).toEqual([]);

    vi.advanceTimersByTime(20);
    expect(kanca.gecikmeler()).toHaveLength(2);
    const [a, b] = kanca.gecikmeler();
    expect(b - a).toBe(10);
    expect(a).toBeGreaterThanOrEqual(10);

    kanca.sifirla();
    expect(kanca.gecikmeler()).toEqual([]);
  });

  it('duraklatılınca mesajları bekletir, sürdürülünce sırayla işler', () => {
    const islenen: number[] = [];
    const dinle = sar((m) => islenen.push(m.no));
    const kanca = window.__kalfaE2e!;

    kanca.duraklat();
    dinle(paket(1, metin('a')));
    dinle(paket(2, metin('b')));
    dinle({ tip: 'Kapandi', no: 3 });
    expect(islenen).toEqual([]);
    expect(kanca.bekleyen()).toBe(3);

    kanca.surdur();
    expect(islenen).toEqual([1, 2, 3]);
    expect(kanca.bekleyen()).toBe(0);

    dinle(paket(4, metin('c')));
    expect(islenen).toEqual([1, 2, 3, 4]);
  });
});
```

`ui/src/markdown/vurgula-motor.test.ts`:

```ts
// Shiki'nin JavaScript düzenli ifade motoru, seçilen dillerde TextMate
// dilbilgilerinin asıl motoru Oniguruma ile aynı tokenları verir (1c tasarımı
// §9.2, §13). Oniguruma'nın WASM'ı yalnızca bu testte yüklenir; pakete girmez.
import { createHighlighterCore, type HighlighterCore } from 'shiki/core';
import { createJavaScriptRegexEngine } from 'shiki/engine/javascript';
import { createOnigurumaEngine } from 'shiki/engine/oniguruma';
import { beforeAll, describe, expect, it } from 'vitest';
import { DILLER, TEMA } from './vurgula';

type Dil = keyof typeof DILLER;

const ORNEKLER: Record<Dil, string> = {
  rust: String.raw`use std::collections::HashMap;

/// Belge yorumu
#[derive(Debug, Clone)]
pub struct Kayit<'a> { ad: &'a str, sayi: u64 }

impl<'a> Kayit<'a> {
    pub fn yeni(ad: &'a str) -> Self {
        let ham = r#"iç "tırnak" \n"#;
        let bayt = b'x';
        match ad.len() { 0 => panic!("boş: {}", ham), n if n > 3 => {}, _ => () }
        Self { ad, sayi: 0x1F_u64 + 1_000 + bayt as u64 }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> { let mut m: HashMap<&str, i32> = HashMap::new(); m.insert("a", -1); Ok(()) }
`,
  typescript: String.raw`import { type A, b } from './a';
export interface Kutu<T extends object = {}> { deger?: T; readonly ad: string }
enum Renk { Kirmizi = 'k', Mavi = 'm' }
const desen = /^[a-z]+\/(\d{2,})$/giu;
export async function* uret<T>(xs: T[]): AsyncGenerator<T> { for (const x of xs) yield x as T; }
const sablon = ` + '`toplam: ${1 + 2} ${`iç ${b}`}`' + String.raw`;
let n: number | undefined = a?.b ?? 0n;
class C<T> implements Kutu<T> { #gizli = 1; static readonly ad = 'c'; constructor(private readonly x: T) {} }
type Esle<T> = T extends (infer U)[] ? U : never;
`,
  javascript: String.raw`// @ts-check
'use strict';
const nesne = { a: 1, 'b-c': [2, 3], [Symbol.iterator]: function* () {} };
const bolme = 10 / 2 / 5, desen = /a\/b[/]c/g;
label: for (let i = 0; i < 3; i++) { if (i === 1) continue label; }
export default class extends Error { get ad() { return ` + '`x${this.message}`' + String.raw`; } }
const ok = async (x = 1, ...geri) => await import('./m.js');
`,
  json: `{
  "ad": "kalfa",
  "sürüm": 1.5e-3,
  "liste": [true, false, null, {"iç": "\\"kaçış\\" \\u00e7"}],
  "boş": {}
}
`,
  toml: `# yorum
[package]
name = "kalfa"
version = "0.1.0"
edition = '2024'
[dependencies.tauri]
version = "2.11"
features = ["test", 'x']
[[bin]]
name = "a"
zaman = 1979-05-27T07:32:00Z
sayi = +1_000
cok_satir = """
satır
"""
`,
  yaml: `# yorum
ad: kalfa
sürüm: 1.0
liste:
  - bir
  - "iki"
  - 'üç'
nesne: { a: 1, b: [x, y] }
cok: |
  satır bir
  satır iki
bagla: &a
  x: 1
kullan: *a
bos: ~
tarih: 2026-09-23
`,
  // `${…}` şablon dizesinde yorumlanacağı için satır satır.
  shellscript: [
    '#!/usr/bin/env bash',
    'set -euo pipefail',
    'AD="kalfa"; SAYI=$((1 + 2))',
    'for f in *.rs; do echo "dosya: $f ${AD:-yok} $(basename "$f")"; done',
    "if [[ -n \"$AD\" && $SAYI -gt 2 ]]; then printf '%s\\n' 'tek' > /dev/null 2>&1; fi",
    "cat <<'SON'",
    'belge $AD',
    'SON',
    'fonk() { local x=$1; return 0; }',
    'case "$1" in start|basla) fonk a ;; *) exit 1 ;; esac',
    '',
  ].join('\n'),
  python: String.raw`from __future__ import annotations
import re

@dataclass(frozen=True)
class Kayit:
    """Belge dizesi."""
    ad: str
    sayi: int = 0x1F

    def __repr__(self) -> str:
        return f"Kayit({self.ad!r}, {self.sayi:>4}) {{kaçış}}"

async def uret(xs: list[int]) -> AsyncIterator[int]:
    desen = re.compile(r"^\d+(?:\.\d+)?$")
    yield from (x ** 2 for x in xs if x % 2 == 0 and not None)
    lambda a, *b, **c: a
`,
  diff: `diff --git a/a.rs b/a.rs
index 1234567..89abcde 100644
--- a/a.rs
+++ b/a.rs
@@ -1,4 +1,4 @@ fn main
 fn main() {
-    let a = 1;
+    let a = 2;
 }
\\ No newline at end of file
`,
  html: `<!doctype html>
<html lang="tr">
<head>
  <meta charset="utf-8" />
  <title>Kalfa &amp; EVREN</title>
  <style>body { color: #333; margin: 0 auto; }</style>
  <script type="module">const x = 1 < 2 && "</b>"; console.log(x);</script>
</head>
<body class="kalfa-root" data-x='y'>
  <!-- yorum -->
  <a href="https://example.com/?a=1&b=2" onclick="return false">bağlantı</a>
  <input disabled type=checkbox>
</body>
</html>
`,
  css: `@import url("tokenlar.css");
:root { --renk: #b8754f; --bosluk: calc(4px * 2); }
.kalfa-root :focus-visible, a[href^="https"]::after {
  outline: 2px solid color-mix(in srgb, var(--renk) 40%, transparent);
  font: 500 14px/1.4 "Source Sans 3 Variable", sans-serif !important;
}
@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) { --renk: rgb(0 0 0 / 50%); } }
@keyframes don { from { transform: rotate(0deg) } to { transform: rotate(360deg) } }
`,
};

type Token = [string, string | undefined];

function tokenlar(v: HighlighterCore, kod: string, dil: Dil): Token[][] {
  return v
    .codeToTokens(kod, { lang: dil, theme: 'kalfa' })
    .tokens.map((satir) => satir.map((t): Token => [t.content, t.color]));
}

describe('Shiki JavaScript motoru', () => {
  let js: HighlighterCore;
  let onig: HighlighterCore;

  beforeAll(async () => {
    const langs = Object.values(DILLER).map((yukle) => yukle());
    js = await createHighlighterCore({
      themes: [TEMA],
      langs,
      engine: createJavaScriptRegexEngine(),
    });
    onig = await createHighlighterCore({
      themes: [TEMA],
      langs,
      engine: createOnigurumaEngine(import('shiki/wasm')),
    });
  });

  it('her dilin bir örneği var', () => {
    expect(Object.keys(ORNEKLER).sort()).toEqual(Object.keys(DILLER).sort());
  });

  it.each(Object.keys(DILLER) as Dil[])('%s: Oniguruma ile aynı tokenlar', (dil) => {
    expect(tokenlar(js, ORNEKLER[dil], dil)).toEqual(tokenlar(onig, ORNEKLER[dil], dil));
  });
});
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cd ui && npx vitest run src/e2e src/markdown/vurgula-motor`
Expected: FAIL — `Failed to resolve import "./kanca"`; `vurgula-motor`'da `TEMA`/`DILLER` dışa açık değil.

- [ ] **Step 3: Kancayı yaz ve bağla**

`ui/src/e2e/kanca.ts`:

```ts
// Uçtan uca test kancası (1c tasarımı §11.4). Yalnızca `npm run build:e2e`
// (`vite build --mode e2e`) derlemesine girer: `import.meta.env.MODE` sürüm
// derlemesinde sabit olduğu için çağıran dal ve bu modül pakete girmez (paket
// denetimi `__kalfaE2e`'yi arar). Modülün üst düzeyinde yan etki yoktur.
//
// - Gecikme: sahte sağlayıcının `[[e2e:<no>:<ms>]]` damgalı parçaları duruma
//   uygulandıktan sonraki ilk çizimde (`requestAnimationFrame` ardından)
//   `Date.now() - ms` kaydedilir. İki saat aynı makinededir.
// - Duraklatma: kanal mesajları duruma uygulanmadan bekletilir, "işledim"
//   gitmez. Sürdürülünce geliş sırasıyla işlenir.
import type { KanalMesaji } from '../ipc/tipler';

const DAMGA = /\[\[e2e:\d+:(\d+(?:\.\d+)?)\]\]/g;

export interface E2eKancasi {
  duraklat(): void;
  surdur(): void;
  /** Ölçülen gecikmeler (ms), ölçüm sırasıyla. */
  gecikmeler(): number[];
  sifirla(): void;
  /** Duraklatıldığı için bekleyen kanal mesajı sayısı. */
  bekleyen(): number;
}

declare global {
  interface Window {
    __kalfaE2e?: E2eKancasi;
  }
}

let durakli = false;
const bekleyenler: Array<() => void> = [];
const olculen: number[] = [];

/** Mesajdaki metin ve düşünce parçalarının damga zamanları. */
export function damgalar(mesaj: KanalMesaji): number[] {
  if (mesaj.tip !== 'Olaylar') return [];
  const zamanlar: number[] = [];
  for (const zarf of mesaj.zarflar) {
    const yuk = zarf.yuk;
    if (yuk.tip !== 'TokenParcasi' && yuk.tip !== 'DusunceParcasi') continue;
    for (const eslesme of yuk.metin.matchAll(DAMGA)) zamanlar.push(Number(eslesme[1]));
  }
  return zamanlar;
}

/** `window.__kalfaE2e`'yi kurar; tekrar çağrılırsa aynı durumu kullanır. */
export function kancayiKur(): E2eKancasi {
  const kanca: E2eKancasi = {
    duraklat: () => {
      durakli = true;
    },
    surdur: () => {
      durakli = false;
      for (const isle of bekleyenler.splice(0)) isle();
    },
    gecikmeler: () => [...olculen],
    sifirla: () => {
      olculen.length = 0;
    },
    bekleyen: () => bekleyenler.length,
  };
  window.__kalfaE2e = kanca;
  return kanca;
}

/** Kanal dinleyicisini ölçen ve duraklatılabilen bir dinleyiciyle sarar. */
export function sar(isleyici: (mesaj: KanalMesaji) => void): (mesaj: KanalMesaji) => void {
  kancayiKur();
  const isle = (mesaj: KanalMesaji) => {
    isleyici(mesaj);
    const zamanlar = damgalar(mesaj);
    if (zamanlar.length === 0) return;
    requestAnimationFrame(() =>
      setTimeout(() => {
        const simdi = Date.now();
        for (const t of zamanlar) olculen.push(simdi - t);
      }, 0),
    );
  };
  return (mesaj) => {
    if (durakli) bekleyenler.push(() => isle(mesaj));
    else isle(mesaj);
  };
}
```

`ui/src/ana/anaDurum.svelte.ts`'de `import { kanalIsleyicisi } from '../durum/dinleyici';` satırının altına:

```ts
import { sar as e2eSar } from '../e2e/kanca';
```

`#kur` içindeki `await ipc.oturumuDinle(oturum, isleyici);` satırını şununla değiştir:

```ts
    // Uçtan uca test derlemesinde dinleyici ölçülür ve duraklatılabilir
    // (1c tasarımı §11.4); sürüm derlemesinde bu dal yoktur.
    await ipc.oturumuDinle(oturum, import.meta.env.MODE === 'e2e' ? e2eSar(isleyici) : isleyici);
```

`ui/src/markdown/vurgula.ts`'de `const TEMA: ThemeRegistration = {` ve `const DILLER = {` satırlarının başına `export ` ekle.

- [ ] **Step 4: Derleme betiği ve paket denetimi**

`ui/package.json`'da `"build"` satırının altına:

```json
    "build:e2e": "vite build --mode e2e --outDir dist-e2e --emptyOutDir",
```

`ui/.gitignore`'a `dist-e2e/` satırını ekle.

`ui/scripts/paket-denetimi.mjs`'nin son hâli:

```js
// Derlenmiş paketin denetimi (1c tasarımı §3.1, §6). `npm run build` sonunda
// koşar; bir kural bozulursa derleme başarısız olur.
//
// - HTML'de satır içi betik, stil özniteliği, <style> etiketi ve dış adres yok.
// - Şart penceresinin açılışta yüklediği dosyalar ana pencerenin sohbet
//   komutlarını içermez.
// - Uçtan uca test kancası (`src/e2e/`) sürüm paketine girmez.
import { readdirSync, readFileSync } from 'node:fs';

const dist = new URL('../dist/', import.meta.url);
const oku = (ad) => readFileSync(new URL(ad, dist), 'utf8');
const hatalar = [];

for (const sayfa of ['index.html', 'sartlar.html']) {
  const html = oku(sayfa);
  for (const [etiket] of html.matchAll(/<script\b[^>]*>/g)) {
    if (!/\bsrc=/.test(etiket)) hatalar.push(`${sayfa}: satır içi betik: ${etiket}`);
  }
  if (/\sstyle=/.test(html)) hatalar.push(`${sayfa}: style özniteliği`);
  if (/<style\b/.test(html)) hatalar.push(`${sayfa}: <style> etiketi`);
  if (/(?:src|href)="(?:https?:)?\/\//.test(html)) hatalar.push(`${sayfa}: dış adres`);
}

// §6.1'de yalnızca `baglanti_ac` iki pencere tarafından da kullanılır.
// Geri kalan ana pencere komutları şart penceresinin paketine girmemelidir.
const ANA_PENCERE_OZEL = [
  'uygulama_durumu',
  'oturumlari_listele',
  'calisma_alani_sec',
  'oturum_baslat',
  'oturum_ac',
  'oturumu_dinle',
  'paketi_isledim',
  'gecmis_sayfasi',
  'mesaj_gonder',
  'turu_iptal_et',
  'arac_onayi',
  'model_degistir',
  'izin_kipi_degistir',
  'oturumu_kapat',
  'anahtar_penceresini_ac',
  'sart_penceresini_ac',
];

function javascriptGrafi(baslangic, manifest) {
  const gorulen = new Set();
  const bekleyen = [...baslangic];
  while (bekleyen.length) {
    const anahtar = bekleyen.pop();
    if (gorulen.has(anahtar)) continue;
    const girdi = manifest[anahtar];
    if (!girdi) {
      hatalar.push(`sartlar.html → ${anahtar}: paket manifestinde girdi yok`);
      continue;
    }
    gorulen.add(anahtar);
    for (const baglanti of [...(girdi.imports ?? []), ...(girdi.dynamicImports ?? [])]) {
      bekleyen.push(baglanti);
    }
  }
  return [...gorulen]
    .map((anahtar) => manifest[anahtar].file)
    .filter((dosya) => /\.m?js$/.test(dosya));
}

const manifest = JSON.parse(oku('.vite/manifest.json'));
const sartlarGrafigi = javascriptGrafi(['sartlar.html'], manifest);
for (const dosya of sartlarGrafigi) {
  const icerik = oku(dosya);
  for (const komut of ANA_PENCERE_OZEL) {
    if (icerik.includes(komut)) {
      hatalar.push(`sartlar.html → ${dosya}: ana pencere komutu "${komut}" içeriyor`);
    }
  }
}

// Test kancası yalnızca `npm run build:e2e` derlemesindedir (1c tasarımı §11.4).
for (const dosya of readdirSync(new URL('assets/', dist))) {
  if (!/\.m?js$/.test(dosya)) continue;
  const icerik = oku(`assets/${dosya}`);
  for (const iz of ['__kalfaE2e', '[[e2e:']) {
    if (icerik.includes(iz)) hatalar.push(`assets/${dosya}: uçtan uca test kancası ("${iz}") içeriyor`);
  }
}

if (hatalar.length) {
  console.error('Paket denetimi başarısız:\n' + hatalar.map((h) => `  - ${h}`).join('\n'));
  process.exit(1);
}
console.log('Paket denetimi temiz.');
```

- [ ] **Step 5: Testlerin ve derlemelerin geçtiğini gör**

Run: `cd ui && npm run check && npm test && npm run build && npm run build:e2e`
Expected:
- **`npm run check`:** 0 hata, 0 uyarı.
- **`npm test`:** 16 dosya, 184 test geçer.
- **`npm run build`:** `Paket denetimi temiz.`
- **Kanca sürüm paketinde yok:** `grep -l __kalfaE2e dist/assets/*.js` boş çıkar.
- **Kanca e2e paketinde var:** `grep -l __kalfaE2e dist-e2e/assets/*.js` bir dosya bulur.

- [ ] **Step 6: Commit**

```bash
git add ui
git commit -m "ui: uçtan uca test kancası, paket denetimi ve Shiki motorlarının karşılaştırması"
```

---

### Task 5: Uçtan uca test paketi: ilk açılış, konuşma, güvenlik

**Files:**
- Create: `e2e/calistir.mjs`, `e2e/yardimci/kalfa.mjs`, `e2e/testler/01-ilk-acilis.test.mjs`, `e2e/testler/02-konusma.test.mjs`, `e2e/testler/03-guvenlik.test.mjs`
- Hazır (plan commit'i): `e2e/package.json`, `e2e/package-lock.json`, `e2e/.npmrc`, `e2e/.gitignore`

**Interfaces:**
- Consumes:
  - Görev 1'in ayar biçimi (`sart`, `anahtar`, `calisma_alani`, `onay`, `senaryolar`) ve senaryo adımları (`metin`, `dusunce`, `arac`, `bekle`, `arac_sonucu`, `damgali`, `sel`).
  - Görev 3'ün test komutları, Görev 4'ün `window.__kalfaE2e`'si.
- Produces (`yardimci/kalfa.mjs`):
  - **Açma ve kapama:** `Kalfa.ac(ayarlar)`, `kapat()`.
  - **Pencere ve IPC:** `pencere('ana' | 'sartlar')`, `pencereSayisi()`, `invoke(komut, args)`.
  - **Kayıt:** `kayit(oturum)`, `oturumlar()`.
  - **Etkileşim:** `anahtarGir`, `dugme(metin)`, `kapatanDugme(metin)`, `ikonDugmesi(etiket)`, `yeniOturum()`, `composer()`, `gonder(metin)`.
  - **Konuşma okuma:** `konusma()`, `metniBekle(parca)`, `turSonlari()`, `turSonuBekle(sayi)`, `seciliOturum()`.
  - **Dışa açık işlevler:** `surecler()`, `bekle`, `sureyleBekle`, `yuzdelik`.
  - **Anahtar sabitleri:** `GECERLI_ANAHTAR`, `GECERSIZ_ANAHTAR`, `AGSIZ_ANAHTAR`.

**Davranış notları:**
- **Koşucu (`calistir.mjs`):**
  - Önce `npm run build:e2e` koşar, sonra uygulamayı derler: `--release --features e2e,tauri/custom-protocol`, `CARGO_TARGET_DIR=target/e2e`.
  - `TAURI_CONFIG` uygulama kimliğini `dev.kalfa.e2e`, arayüz dizinini `ui/dist-e2e` yapar.
  - Testleri `node --test --test-concurrency=1` ile sırayla koşar.
  - `--derleme-yok` derlemeyi atlar; dosya adları verilirse yalnızca onlar koşar.
- **Pencereler:** pencere, adresin yolundan tanınır. Windows'ta `http://tauri.localhost/…`, Linux'ta `tauri://localhost/…` gelir; başlık iki platformda aynıdır.
- **Süreçler:** yolu testin uygulamasıyla aynı olan süreçlerdir. Windows'ta PowerShell ile, Linux'ta `/proc/*/exe` ile bulunur. Her açılış önceki sürecin kapanmasını bekler (tek süreç kilidi).
- **Pencereyi kapatan tıklama:** WebKitWebDriver tıklamanın yanıtından önce pencere kapanırsa "no such window" döner; `kapatanDugme` bunu beklenen sonuç sayar.
- **CSP testi:**
  - WebDriver'ın çalıştırdığı betik sayfanın bağlamında değildir. Chromium orada `eval`'i ihlali bildirerek çalıştırır.
  - Test dizeden kodu bu yüzden sayfanın kendi bağlamında çalışan dize zamanlayıcısıyla (`setTimeout('…')`) sınar.
- **IPC izi:** IPC çağrıları Windows'ta kaynak zamanlamasına `http://ipc.localhost/<komut>` olarak düşer. Güvenlik testi zararlı çıktıdan sonra beklenmeyen komut arar.

- [ ] **Step 1: Bağımlılıkları kur**

Run: `cd e2e && npm ci`
Expected: 238 paket; kurulum betiği çalışmaz (`.npmrc`).

- [ ] **Step 2: Koşucuyu ve yardımcıları yaz**

`e2e/calistir.mjs`:

```js
// Uçtan uca testlerin koşucusu (1c tasarımı §11.4).
//
//   node calistir.mjs [--derleme-yok] [testler/<dosya>.test.mjs …]
//
// 1. Arayüzü test kancasıyla derler (`ui/dist-e2e`).
// 2. Uygulamayı `e2e` özelliği ve gömülü arayüzle, ayrı bir hedef dizinine
//    sürüm profilinde derler (`target/e2e/release/kalfa`). Uygulama kimliği
//    `dev.kalfa.e2e` olur: geliştiricinin açık Kalfa'sıyla çakışmaz.
// 3. Testleri sırayla koşar; her test kendi Kalfa sürecini açar.
//
// Windows'ta `msedgedriver` (WebView2 sürümüyle aynı) PATH'te ya da
// `KALFA_E2E_EDGEDRIVER`'da olmalıdır. Linux'ta `WebKitWebDriver` ve bir ekran
// (`xvfb-run -a npm test`) gerekir. `tauri-driver` iki platformda da PATH'te
// olmalıdır.
import { spawnSync } from 'node:child_process';
import { readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const e2e = dirname(fileURLToPath(import.meta.url));
const kok = join(e2e, '..');
const hedef = join(kok, 'target', 'e2e');
const uygulama = join(hedef, 'release', process.platform === 'win32' ? 'kalfa.exe' : 'kalfa');

const argumanlar = process.argv.slice(2);
const derleme = !argumanlar.includes('--derleme-yok');
const secilen = argumanlar.filter((a) => !a.startsWith('--'));

function calistir(komut, args, secenekler = {}) {
  console.log(`> ${komut} ${args.join(' ')}`);
  const sonuc = spawnSync(komut, args, {
    stdio: 'inherit',
    shell: process.platform === 'win32',
    ...secenekler,
  });
  if (sonuc.status !== 0) process.exit(sonuc.status ?? 1);
}

if (derleme) {
  calistir('npm', ['run', 'build:e2e'], { cwd: join(kok, 'ui') });
  calistir(
    'cargo',
    ['build', '--release', '--locked', '-p', 'kalfa', '--features', 'e2e,tauri/custom-protocol'],
    {
      cwd: kok,
      env: {
        ...process.env,
        CARGO_TARGET_DIR: hedef,
        TAURI_CONFIG: JSON.stringify({
          identifier: 'dev.kalfa.e2e',
          build: { frontendDist: '../../ui/dist-e2e' },
        }),
      },
    },
  );
}

const testler = secilen.length
  ? secilen
  : readdirSync(join(e2e, 'testler'))
      .filter((d) => d.endsWith('.test.mjs'))
      .sort()
      .map((d) => join('testler', d));

calistir(
  process.execPath,
  ['--test', '--test-concurrency=1', '--test-reporter=spec', ...testler],
  { cwd: e2e, shell: false, env: { ...process.env, KALFA_E2E_UYGULAMA: uygulama } },
);
```

`e2e/yardimci/kalfa.mjs`:

```js
// Kalfa'yı `tauri-driver` üzerinden açan ve süren yardımcılar (1c tasarımı
// §11.4). Her `Kalfa.ac` kendi geçici veri dizini, çalışma alanı ve ayar
// dosyasıyla yeni bir süreç başlatır; `kapat` süreci bitirir ve dizini siler.
import { execFileSync, spawn } from 'node:child_process';
import {
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  readdirSync,
  readlinkSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { connect } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { remote } from 'webdriverio';

/** Sahte EVREN'in geçerli saydığı herhangi bir anahtar. */
export const GECERLI_ANAHTAR = 'evren_llm_e2e_gizli_anahtar_7f3a';
/** Sahte EVREN'in 401 döndüğü anahtar (`kalfa_uygulama::e2e`). */
export const GECERSIZ_ANAHTAR = 'evren_llm_gecersiz';
/** Sahte EVREN'e ulaşılamayan anahtar. */
export const AGSIZ_ANAHTAR = 'evren_llm_agsiz';

const SURUCU_PORTU = 4444;
const YEREL_PORT = 4445;
const WINDOWS = process.platform === 'win32';

const uygulama = process.env.KALFA_E2E_UYGULAMA;
if (!uygulama) throw new Error('KALFA_E2E_UYGULAMA yok; testleri `npm test` ile koşun.');

export const bekle = (ms) => new Promise((r) => setTimeout(r, ms));

export async function sureyleBekle(kosul, sure, mesaj) {
  const son = Date.now() + sure;
  while (Date.now() < son) {
    if (await kosul()) return;
    await bekle(100);
  }
  throw new Error(mesaj);
}

function portAcik(port) {
  return new Promise((coz) => {
    const s = connect(port, '127.0.0.1');
    s.once('connect', () => {
      s.destroy();
      coz(true);
    });
    s.once('error', () => coz(false));
  });
}

/** Bu testin derlediği Kalfa'nın süreçleri: kimlik ve yerleşik bellek (bayt). */
export function surecler() {
  if (WINDOWS) {
    const yol = uygulama.replaceAll("'", "''");
    const cikti = execFileSync(
      'powershell.exe',
      [
        '-NoProfile',
        '-Command',
        `Get-Process kalfa -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq '${yol}' } | ForEach-Object { "$($_.Id) $($_.WorkingSet64)" }; exit 0`,
      ],
      { encoding: 'utf8' },
    );
    return cikti
      .split(/\r?\n/)
      .filter(Boolean)
      .map((s) => {
        const [pid, bellek] = s.trim().split(' ').map(Number);
        return { pid, bellek };
      });
  }
  const bulunan = [];
  for (const ad of readdirSync('/proc')) {
    if (!/^\d+$/.test(ad)) continue;
    try {
      if (readlinkSync(`/proc/${ad}/exe`) !== uygulama) continue;
      const durum = readFileSync(`/proc/${ad}/status`, 'utf8');
      const kb = Number(/VmRSS:\s+(\d+)/.exec(durum)?.[1] ?? 0);
      bulunan.push({ pid: Number(ad), bellek: kb * 1024 });
    } catch {
      // Süreç bu arada bitti ya da başka kullanıcınınki.
    }
  }
  return bulunan;
}

/** Önceki testten kalan süreç yenisinin tek süreç kilidine takılır. */
async function sureclerBitsin(sure = 10000) {
  try {
    await sureyleBekle(() => surecler().length === 0, sure, 'Kalfa süreci kapanmadı');
  } catch {
    for (const { pid } of surecler()) {
      try {
        process.kill(pid, 'SIGKILL');
      } catch {
        // Zaten bitti.
      }
    }
    await sureyleBekle(() => surecler().length === 0, 5000, 'Kalfa süreci öldürülemedi');
  }
}

export class Kalfa {
  /**
   * Kalfa'yı açar. `ayarlar` sahte EVREN'in ayarlarıdır
   * (`kalfa_uygulama::e2e::E2eAyarlari`); çalışma alanı verilmezse geçici
   * dizindeki `alan` kullanılır.
   */
  static async ac(ayarlar = {}) {
    await sureclerBitsin();
    const kok = mkdtempSync(join(tmpdir(), 'kalfa-e2e-'));
    const veri = join(kok, 'veri');
    const alan = join(kok, 'alan');
    mkdirSync(veri);
    mkdirSync(alan);
    const ayarDosyasi = join(kok, 'e2e.json');
    writeFileSync(ayarDosyasi, JSON.stringify({ calisma_alani: alan, ...ayarlar }));

    const args = ['--port', String(SURUCU_PORTU), '--native-port', String(YEREL_PORT)];
    if (WINDOWS && process.env.KALFA_E2E_EDGEDRIVER) {
      args.push('--native-driver', process.env.KALFA_E2E_EDGEDRIVER);
    }
    const gunluk = openSync(join(kok, 'surucu.log'), 'a');
    const surucu = spawn('tauri-driver', args, {
      env: {
        ...process.env,
        KALFA_VERI_DIZINI: veri,
        KALFA_ANAHTAR_DEPOSU: 'bellek',
        KALFA_E2E: ayarDosyasi,
      },
      stdio: ['ignore', gunluk, gunluk],
    });
    await sureyleBekle(() => portAcik(SURUCU_PORTU), 15000, 'tauri-driver başlamadı');
    const b = await remote({
      hostname: '127.0.0.1',
      port: SURUCU_PORTU,
      logLevel: 'silent',
      capabilities: {
        'tauri:options': { application: uygulama },
        'wdio:enforceWebDriverClassic': true,
      },
    });
    return new Kalfa(b, surucu, { kok, veri, alan });
  }

  constructor(b, surucu, dizinler) {
    this.b = b;
    this.surucu = surucu;
    this.kok = dizinler.kok;
    this.veri = dizinler.veri;
    this.alan = dizinler.alan;
  }

  async kapat() {
    await this.b.deleteSession().catch(() => {});
    this.surucu.kill();
    await sureclerBitsin();
    rmSync(this.kok, { recursive: true, force: true, maxRetries: 10, retryDelay: 300 });
  }

  /** Pencereye geçer: `ana` ya da `sartlar`. Açılmasını bekler. */
  async pencere(ad, sure = 15000) {
    const yol = ad === 'sartlar' ? /\/sartlar\.html$/ : /^\/?(index\.html)?$/;
    await this.b.waitUntil(
      async () => {
        for (const h of await this.b.getWindowHandles().catch(() => [])) {
          try {
            await this.b.switchToWindow(h);
            if (yol.test(await this.b.execute(() => location.pathname))) return true;
          } catch {
            // Pencere bu arada kapandı ya da yeniden yükleniyor.
          }
        }
        return false;
      },
      { timeout: sure, interval: 200, timeoutMsg: `${ad} penceresi açılmadı` },
    );
  }

  async pencereSayisi() {
    return (await this.b.getWindowHandles()).length;
  }

  /** Bulunulan pencereden bir komut çağırır; hata metniyle reddedilir. */
  async invoke(komut, args = {}) {
    const [tamam, deger] = await this.b.executeAsync(
      (k, a, bitti) => {
        window.__TAURI_INTERNALS__.invoke(k, a).then(
          (d) => bitti([true, d ?? null]),
          (e) => bitti([false, typeof e === 'string' ? e : JSON.stringify(e)]),
        );
      },
      komut,
      args,
    );
    if (!tamam) throw new Error(deger);
    return deger;
  }

  /** Oturumun kayıt öğeleri (`yuk` alanları), yazılış sırasıyla. */
  kayit(oturum) {
    return readFileSync(join(this.veri, 'oturumlar', `${oturum}.jsonl`), 'utf8')
      .split('\n')
      .filter(Boolean)
      .map((s) => JSON.parse(s).yuk);
  }

  /** Kayıt dizinindeki oturum kimlikleri. */
  oturumlar() {
    return readdirSync(join(this.veri, 'oturumlar'))
      .filter((d) => d.endsWith('.jsonl'))
      .map((d) => d.slice(0, -'.jsonl'.length));
  }

  // --- Şart penceresi ---

  async anahtarGir(anahtar) {
    const girdi = await this.b.$('input[type=password]');
    await girdi.waitForDisplayed({ timeout: 15000 });
    await girdi.setValue(anahtar);
    await (await this.b.$('button[type=submit]')).click();
  }

  // --- Ana pencere ---

  async dugme(metin) {
    const d = await this.b.$(`button=${metin}`);
    await d.waitForClickable({ timeout: 15000 });
    await d.click();
  }

  /**
   * Pencereyi kapatan düğmeye basar ("Kabul ediyorum"). Pencere tıklamanın
   * yanıtından önce kapanabilir; sürücünün "no such window" hatası o zaman
   * beklenen sonuçtur.
   */
  async kapatanDugme(metin) {
    const d = await this.b.$(`button=${metin}`);
    await d.waitForClickable({ timeout: 15000 });
    await d.click().catch((e) => {
      if (!/no such window/.test(String(e))) throw e;
    });
  }

  async ikonDugmesi(etiket) {
    const d = await this.b.$(`button[aria-label="${etiket}"]`);
    await d.waitForClickable({ timeout: 15000 });
    await d.click();
  }

  async yeniOturum() {
    await this.ikonDugmesi('Yeni oturum');
    await (await this.composer()).waitForEnabled({ timeout: 15000 });
  }

  composer() {
    return this.b.$('textarea[aria-label^="Bir şey sorun"]');
  }

  async gonder(metin) {
    const c = await this.composer();
    await c.waitForEnabled({ timeout: 15000 });
    await c.setValue(metin);
    await this.ikonDugmesi('Gönder');
  }

  /** Konuşma sütununun görünen metni. */
  async konusma() {
    return this.b.execute(() => document.querySelector('.akis')?.textContent ?? '');
  }

  async metniBekle(parca, sure = 20000) {
    await this.b.waitUntil(async () => (await this.konusma()).includes(parca), {
      timeout: sure,
      interval: 100,
      timeoutMsg: `konuşmada görünmedi: ${parca}`,
    });
  }

  /** Tur sonu çizgilerinin nedenleri (`Normal`, `Iptal`, …), sırayla. */
  async turSonlari() {
    return this.b.execute(() =>
      [...document.querySelectorAll('.tur-sonu')].map(
        (e) => [...e.classList].find((c) => c.startsWith('n-'))?.slice(2) ?? '',
      ),
    );
  }

  async turSonuBekle(sayi, sure = 30000) {
    await this.b.waitUntil(async () => (await this.turSonlari()).length >= sayi, {
      timeout: sure,
      interval: 100,
      timeoutMsg: `${sayi}. tur bitmedi`,
    });
    return (await this.turSonlari())[sayi - 1];
  }

  /** Seçili oturumun kimliği (uygulama durumundaki son oturum). */
  async seciliOturum() {
    return (await this.invoke('uygulama_durumu')).son_oturum;
  }
}

export function yuzdelik(degerler, p) {
  const sirali = [...degerler].sort((a, b) => a - b);
  const i = Math.min(sirali.length - 1, Math.ceil((p / 100) * sirali.length) - 1);
  return sirali[Math.max(0, i)];
}
```

- [ ] **Step 3: Testleri yaz**

`e2e/testler/01-ilk-acilis.test.mjs`:

```js
// İlk açılış (1c tasarımı §7, §11.4): anahtar → şart → kabul → ana pencere;
// ret → kilitli görünüm; geçersiz anahtar ve ulaşılamayan EVREN → form hatası.
// Anahtar ana pencereye hiç girmez.
import assert from 'node:assert/strict';
import { afterEach, describe, it } from 'node:test';
import { AGSIZ_ANAHTAR, GECERLI_ANAHTAR, GECERSIZ_ANAHTAR, Kalfa } from '../yardimci/kalfa.mjs';

describe('ilk açılış', () => {
  let k;
  afterEach(async () => {
    await k?.kapat();
    k = undefined;
  });

  it('anahtar, şart metni ve kabulden sonra ana pencere açılır', async () => {
    k = await Kalfa.ac({ sart: 'gerekli' });
    await k.pencere('sartlar');
    await k.anahtarGir(GECERLI_ANAHTAR);

    const kabul = await k.b.$('button=Kabul ediyorum');
    await kabul.waitForClickable({ timeout: 15000 });
    const metin = await k.b.$('body').getText();
    assert.match(metin, /EVREN kullanım şartları/);
    assert.match(metin, /Sahte kullanım şartları/);
    await k.kapatanDugme('Kabul ediyorum');

    await k.pencere('ana');
    await k.b.waitUntil(async () => (await k.pencereSayisi()) === 1, {
      timeout: 10000,
      timeoutMsg: 'şart penceresi kapanmadı',
    });
    await (await k.b.$('button[aria-label="Yeni oturum"]')).waitForDisplayed({ timeout: 15000 });
  });

  it('anahtar ana pencerenin DOM’una ve durumuna girmez', async () => {
    k = await Kalfa.ac({ sart: 'gerekli' });
    await k.pencere('sartlar');
    await k.anahtarGir(GECERLI_ANAHTAR);
    await k.kapatanDugme('Kabul ediyorum');
    await k.pencere('ana');
    await k.yeniOturum();
    await k.gonder('merhaba');
    await k.metniBekle('Yanıt: merhaba');

    const izler = await k.b.execute(() => {
      const depo = (s) => [...Array(s.length).keys()].map((i) => `${s.key(i)}=${s.getItem(s.key(i))}`);
      return [
        document.documentElement.outerHTML,
        ...depo(localStorage),
        ...depo(sessionStorage),
        document.cookie,
      ].join('\n');
    });
    assert.ok(!izler.includes(GECERLI_ANAHTAR), 'anahtar ana pencerenin DOM’unda ya da deposunda');
    const durum = JSON.stringify(await k.invoke('uygulama_durumu'));
    assert.ok(!durum.includes(GECERLI_ANAHTAR), 'anahtar uygulama durumunda');
  });

  it('şartlar reddedilince kilitli görünüm gelir, ana pencere açılmaz', async () => {
    k = await Kalfa.ac({ sart: 'gerekli' });
    await k.pencere('sartlar');
    await k.anahtarGir(GECERLI_ANAHTAR);
    await k.dugme('Kabul etmiyorum');
    const baslik = await k.b.$('h1=Şartlar kabul edilmedi');
    await baslik.waitForDisplayed({ timeout: 15000 });
    await (await k.b.$('button=Metni yeniden göster')).waitForDisplayed();
    assert.equal(await k.pencereSayisi(), 1);
  });

  it('geçersiz anahtar formda hata verir ve saklanmaz', async () => {
    k = await Kalfa.ac({ sart: 'gerekli' });
    await k.pencere('sartlar');
    await k.anahtarGir(GECERSIZ_ANAHTAR);
    const uyari = await k.b.$('[role="alert"]');
    await uyari.waitForDisplayed({ timeout: 15000 });
    assert.match(await uyari.getText(), /Anahtar geçersiz ya da eksik/);
    await (await k.b.$('input[type=password]')).waitForDisplayed();
  });

  it('EVREN’e ulaşılamazsa formda hata verir', async () => {
    k = await Kalfa.ac({ sart: 'gerekli' });
    await k.pencere('sartlar');
    await k.anahtarGir(AGSIZ_ANAHTAR);
    const uyari = await k.b.$('[role="alert"]');
    await uyari.waitForDisplayed({ timeout: 15000 });
    assert.match(await uyari.getText(), /EVREN'e ulaşılamıyor/);
  });
});
```

`e2e/testler/02-konusma.test.mjs`:

```js
// Konuşma (1c tasarımı §1, §8, §11.4): mesaj → akış → onay sayfası → yazma;
// ret gerekçesinin modele gitmesi; iptal; oturumu kapatıp kayıttan açma ve
// geçmişin doğru sırayla gelmesi.
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa } from '../yardimci/kalfa.mjs';

const SENARYOLAR = {
  yaz: {
    yanitlar: [
      [
        { tur: 'metin', metin: 'Notu yazıyorum.' },
        { tur: 'arac', ad: 'dosya_yaz', argumanlar: { yol: 'notlar.txt', icerik: 'merhaba\n' } },
      ],
      [{ tur: 'arac_sonucu' }],
    ],
  },
  'yaz-ret': {
    yanitlar: [
      [{ tur: 'arac', ad: 'dosya_yaz', argumanlar: { yol: 'ret.txt', icerik: 'x\n' } }],
      [{ tur: 'arac_sonucu' }],
    ],
  },
  uzun: {
    yanitlar: [[{ tur: 'damgali', sayi: 600, desen: { ad: 'duzenli', aralik_ms: 100 } }]],
  },
};

describe('konuşma', () => {
  let k;
  before(async () => {
    k = await Kalfa.ac({ sart: 'kabul', anahtar: GECERLI_ANAHTAR, senaryolar: SENARYOLAR });
    await k.pencere('ana');
    await k.yeniOturum();
  });
  after(async () => {
    await k?.kapat();
  });

  it('akan yanıttan sonra onaylanan değişiklik dosyaya yazılır', async () => {
    await k.gonder('yaz');
    await k.metniBekle('Notu yazıyorum.');
    await k.dugme('İncele');
    await k.dugme('Tümünü onayla');
    assert.equal(await k.turSonuBekle(1), 'Normal');
    assert.equal(readFileSync(join(k.alan, 'notlar.txt'), 'utf8'), 'merhaba\n');
    await k.metniBekle('Araç sonucu:');
  });

  it('reddedilen değişikliğin gerekçesi modele gider, dosya yazılmaz', async () => {
    await k.gonder('yaz-ret');
    await k.dugme('İncele');
    const gerekce = await k.b.$('textarea[placeholder^="Reddetme nedeninizi"]');
    await gerekce.waitForDisplayed({ timeout: 15000 });
    await gerekce.setValue('başka bir ad kullan');
    await k.dugme('Reddet');
    assert.equal(await k.turSonuBekle(2), 'Normal');
    await k.metniBekle('[kullanıcı reddetti] başka bir ad kullan');
    assert.ok(!existsSync(join(k.alan, 'ret.txt')));
  });

  it('süren tur iptal edilir', async () => {
    await k.gonder('uzun');
    await k.metniBekle('[[e2e:3:');
    await k.ikonDugmesi('Turu durdur');
    assert.equal(await k.turSonuBekle(3), 'Iptal');
  });

  it('kapatılan oturum kayıttan açılır; geçmiş sırasıyla gelir', async () => {
    const oturum = await k.seciliOturum();
    await k.ikonDugmesi('Oturumu kapat');
    await (await k.composer()).waitForExist({ reverse: true, timeout: 10000 });

    const satir = await k.b.$('button.oturum-satiri');
    await satir.waitForClickable({ timeout: 10000 });
    await satir.click();
    await k.turSonuBekle(3);

    const kullanici = await k.b.execute(() =>
      [...document.querySelectorAll('article.mesaj.kullanici .duz')].map((e) => e.textContent),
    );
    assert.deepEqual(kullanici, ['yaz', 'yaz-ret', 'uzun']);
    assert.deepEqual(await k.turSonlari(), ['Normal', 'Normal', 'Iptal']);
    const bitenler = k
      .kayit(oturum)
      .filter((y) => y.tip === 'TurBitti')
      .map((y) => y.neden.tip);
    assert.deepEqual(bitenler, ['Normal', 'Normal', 'Iptal']);

    // Kayıttan açılan oturum yazılabilir: kayıt kilidi yeniden alındı.
    await k.gonder('devam');
    await k.metniBekle('Yanıt: devam');
    assert.equal(await k.turSonuBekle(4), 'Normal');
  });
});
```

`e2e/testler/03-guvenlik.test.mjs`:

```js
// Güvenlik sınırları gerçek webview'da (1c tasarımı §2, §6, §9.1, §11.4):
// pencere yetkileri, CSP ve model çıktısındaki betik, görsel ve bağlantılar.
import assert from 'node:assert/strict';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa } from '../yardimci/kalfa.mjs';

const ZARARLI = [
  'Deneme <script>window.__xss = 1</script> sonu.',
  '',
  '<img src="x" onerror="window.__xss = 2">',
  '',
  '![uzak görsel](https://example.invalid/iz.png)',
  '',
  '[betik bağlantısı](javascript:window.__xss=3)',
  '',
  '[dosya bağlantısı](file:///etc/passwd)',
  '',
  '<iframe src="https://example.invalid/"></iframe>',
].join('\n');

const SENARYOLAR = {
  zararli: { yanitlar: [[{ tur: 'metin', metin: ZARARLI }]] },
};

/** Webview'ın yüklediği kaynakların ve IPC çağrılarının adresleri. */
const kaynaklar = (k) =>
  k.b.execute(() => performance.getEntriesByType('resource').map((e) => e.name));

describe('güvenlik', () => {
  let k;
  before(async () => {
    k = await Kalfa.ac({ sart: 'kabul', anahtar: GECERLI_ANAHTAR, senaryolar: SENARYOLAR });
    await k.pencere('ana');
    await k.yeniOturum();
  });
  after(async () => {
    await k?.kapat();
  });

  it('ana pencere şart kabul edemez, anahtar kaydedemez, pencere açamaz', async () => {
    for (const [komut, args] of [
      ['sartlari_kabul_et', { surum: 1 }],
      ['anahtari_kaydet', { anahtar: 'evren_llm_x' }],
      ['sart_penceresi_durumu', {}],
      ['plugin:webview|create_webview_window', {}],
      ['plugin:opener|open_url', { url: 'https://example.com' }],
      ['plugin:dialog|open', {}],
    ]) {
      await assert.rejects(k.invoke(komut, args), /not allowed/, `${komut} reddedilmedi`);
    }
  });

  // WebDriver'ın çalıştırdığı betik sayfanın değil sürücünün bağlamındadır;
  // Chromium orada `eval`'i ihlal bildirerek çalıştırır. Bu yüzden eval, sayfanın
  // kendi bağlamında çalışan dize zamanlayıcısıyla sınanır.
  it('CSP satır içi betiği, satır içi olay işleyicisini ve dizeden kodu engeller', async () => {
    const sonuc = await k.b.executeAsync((bitti) => {
      const ihlaller = [];
      document.addEventListener('securitypolicyviolation', (e) =>
        ihlaller.push({ yonerge: e.effectiveDirective || e.violatedDirective, adres: e.blockedURI }),
      );
      const betik = document.createElement('script');
      betik.textContent = 'window.__csp = 1';
      document.body.append(betik);
      const dugme = document.createElement('button');
      dugme.setAttribute('onclick', 'window.__csp = 2');
      document.body.append(dugme);
      dugme.click();
      setTimeout('window.__csp = 3', 0);
      betik.remove();
      dugme.remove();
      setTimeout(() => bitti({ csp: window.__csp ?? null, ihlaller }), 300);
    });
    assert.equal(sonuc.csp, null, 'satır içi kod çalıştı');
    const betikIhlali = sonuc.ihlaller.filter((i) => i.yonerge.startsWith('script-src'));
    assert.ok(betikIhlali.length >= 3, `CSP ihlalleri eksik: ${JSON.stringify(sonuc.ihlaller)}`);
    assert.ok(
      betikIhlali.some((i) => i.adres === 'eval'),
      `dizeden kod ihlali yok: ${JSON.stringify(sonuc.ihlaller)}`,
    );
  });

  it('model çıktısındaki betik çalışmaz; görsel, çerçeve ve tehlikeli bağlantı oluşmaz', async () => {
    const once = new Set(await kaynaklar(k));
    await k.gonder('zararli');
    assert.equal(await k.turSonuBekle(1), 'Normal');
    await k.metniBekle('uzak görsel');

    const dom = await k.b.execute(() => {
      const akis = document.querySelector('.akis');
      return {
        xss: window.__xss ?? null,
        tehlikeli: akis.querySelectorAll('script, img, iframe, object, embed, a[href], [onerror]')
          .length,
        metin: akis.textContent,
        dugmeler: [...akis.querySelectorAll('button')].map((b) => b.textContent.trim()),
      };
    });
    assert.equal(dom.xss, null, 'model çıktısındaki betik çalıştı');
    assert.equal(dom.tehlikeli, 0, 'konuşmada betik, görsel, çerçeve ya da bağlantı var');
    assert.match(dom.metin, /<script>window.__xss = 1<\/script>/, 'ham HTML metin olarak görünmüyor');
    assert.ok(!dom.dugmeler.includes('betik bağlantısı'), 'javascript: bağlantıya dönüştü');
    assert.ok(!dom.dugmeler.includes('dosya bağlantısı'), 'file: bağlantıya dönüştü');

    // Bağlantı metnine tıklamak bir şey yapmaz.
    const betikMetni = await k.b.$('//*[contains(@class,"akis")]//*[text()[contains(.,"betik bağlantısı")]]');
    if (await betikMetni.isExisting()) await betikMetni.click();

    const yeni = (await kaynaklar(k)).filter((a) => !once.has(a));
    assert.ok(
      !yeni.some((a) => a.includes('example.invalid')),
      `uzak kaynak yüklendi: ${yeni.join(', ')}`,
    );
    const ipc = yeni
      .filter((a) => /ipc/.test(a))
      .map((a) => decodeURIComponent(new URL(a).pathname.slice(1)));
    const beklenen = new Set(['mesaj_gonder', 'paketi_isledim', 'oturumlari_listele']);
    const beklenmeyen = ipc.filter((c) => !beklenen.has(c));
    assert.deepEqual(beklenmeyen, [], `beklenmeyen IPC çağrıları: ${beklenmeyen.join(', ')}`);
    assert.equal(await k.b.execute(() => window.__xss ?? null), null);
  });

  it('şart penceresi oturum komutlarını çağıramaz', async () => {
    await k.b.execute(() => {
      const d = [...document.querySelectorAll('button')].find(
        (b) => b.textContent.trim() === 'Anahtarı değiştir',
      );
      d.click();
    });
    await k.pencere('sartlar');
    for (const [komut, args] of [
      ['oturumlari_listele', {}],
      ['mesaj_gonder', { oturum: '00000000-0000-0000-0000-000000000000', metin: 'x' }],
      ['calisma_alani_sec', {}],
      ['e2e_olcumler', {}],
    ]) {
      await assert.rejects(k.invoke(komut, args), /not allowed/, `${komut} reddedilmedi`);
    }
  });
});
```

- [ ] **Step 4: Testleri koştur**

Run (Windows): `cd e2e && npm test -- testler/01-ilk-acilis.test.mjs testler/02-konusma.test.mjs testler/03-guvenlik.test.mjs`. `msedgedriver` PATH'te değilse önce `KALFA_E2E_EDGEDRIVER`'ı ayarla.
Run (Linux): `cd e2e && xvfb-run -a npm test -- testler/01-ilk-acilis.test.mjs testler/02-konusma.test.mjs testler/03-guvenlik.test.mjs`
Expected: 13 test geçer. İlk koşu uygulamayı sürüm profilinde derler; bu birkaç dakika sürer. Sürücüler ortamında yoksa bu adımı atla ve PR'a yaz.

- [ ] **Step 5: Commit**

```bash
git add e2e
git commit -m "e2e: gerçek pencerede ilk açılış, konuşma ve güvenlik testleri"
```

---

### Task 6: Kabul kapısı ve paketleme penceresi

Üst belge §11.1'in kabul kapısını ölçer: gecikme, duraklayan arayüz, arayüz kopması, uzun oturum. Plan yazılırken yapılan ölçüm paketleme penceresinin 16 ms'den 8 ms'ye inmesini gerektirdi:

| Platform | 16 ms'de p95 | 8 ms'de p95 |
|---|---|---|
| Windows | 41 ms | 25–27 ms |
| Linux | 47 ms (koşuya göre 61 ms'ye kadar) | 29–42 ms |

Kabul hedefi 50 ms'dir. 20 ms aralıkla gelen parçalar pencerede birleşmez; her biri pencerenin sonunu bekler. Kredi ve paket sınırı değişmez.

**Files:**
- Create: `e2e/testler/04-kabul.test.mjs`
- Modify: `crates/kalfa-core/src/aktarim.rs`, `crates/kalfa-core/tests/aktarim.rs`

**Davranış notları:**
- **Gecikme:** üç desen ölçülür; her birinde 250 parça, paralel desende 500 ölçüm. Her desende p95 < 50 ms olmalıdır. Ölçümden önce ölçülmeyen bir ısınma turu gönderilir; uygulamanın ilk turu webview'ın ilk çizimlerini ve derleyici ısınmasını da içerir.
- **Küçültülmüş pencere:** pencere küçükken tur biter; arayüz paketleri işlemeyi sürdürür (1c tasarımı §4.3, §13).
- **Duraklayan arayüz:**
  - Duraklarken arayüze en çok kredi kadar paket gelir.
  - Tur iptal edilir ve kayda yazılır.
  - Sürdürülünce `AktarimDurdu` uyarısı görünür.
  - Sayaçlar yoldaki verinin krediyi, kuyruğun bütçeyi aşmadığını ve kuyruğun gerçekten dolduğunu (bütçenin yarısından fazla) gösterir.
  - Akıl sağlığı denetimi: Kalfa sürecinin belleği 256 MB'tan fazla büyümez.
- **Arayüz kopması:**
  - Sayfa yeniden yüklenirse tur iptal edilir. Yeni sayfa oturumu kayıttan açar ve oturuma yeni mesaj gönderilebilir; kayıt kilidi bırakılmıştır.
  - Pencere `e2e_ana_pencereyi_kapat` ile kapanırsa süreç çıkar; kayıtta `TurBitti { Iptal }` bulunur.
- **Uzun oturum:**
  - 500 turluk kayıt 20 turluk kayıtla aynı mertebede açılır. Süre 10 katından, DOM düğümü 1,5 katından az olmalıdır.
  - Isınmanın avantajı kısa kayda kalsın diye önce uzun kayıt açılır.
- **Sonuçlar:** `e2e/sonuclar/<platform>.json`'a yazılır ve ekrana basılır.

- [ ] **Step 1: Pencere testini varsayılana göreli yap**

`crates/kalfa-core/tests/aktarim.rs`'de `pencereden_sonra_gelen_parca_sonraki_pakete_kalir` testinin başını şöyle yap. Sabit süreler yerine pencerenin yarısı ve iki katı kullanılır:

```rust
async fn pencereden_sonra_gelen_parca_sonraki_pakete_kalir() {
    let ayar = AktarimAyarlari::default();
    let (mut u, mut p, kredi, _) = kur(ayar);
    u.metin("a");
    let uretici = tokio::spawn(async move {
        tokio::time::sleep(ayar.pencere / 2).await;
        u.metin("b");
        tokio::time::sleep(ayar.pencere * 2).await;
```

Run: `cargo test -p kalfa-core --locked --test aktarim`
Expected: PASS; test henüz 16 ms'lik pencereyle koşar.

- [ ] **Step 2: Kabul testlerini yaz**

`e2e/testler/04-kabul.test.mjs`:

```js
// Üst belge §11.1 kabul kapısı (1c tasarımı §2, §11.4): gecikme, duraklayan
// arayüz, arayüz kopması ve uzun oturum. Ölçümler `sonuclar/<platform>.json`'a
// yazılır ve docs/GELISTIRME_DURUMU.md'ye aktarılır.
import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { after, before, describe, it } from 'node:test';
import {
  GECERLI_ANAHTAR,
  Kalfa,
  bekle,
  surecler,
  sureyleBekle,
  yuzdelik,
} from '../yardimci/kalfa.mjs';

const PARCA = 250;

const SENARYOLAR = {
  isinma: {
    yanitlar: [[{ tur: 'damgali', sayi: 100, desen: { ad: 'duzenli', aralik_ms: 20 } }]],
  },
  'gecikme-duzenli': {
    yanitlar: [[{ tur: 'damgali', sayi: PARCA, desen: { ad: 'duzenli', aralik_ms: 20 } }]],
  },
  'gecikme-yigin': {
    yanitlar: [
      [{ tur: 'damgali', sayi: PARCA, desen: { ad: 'yigin', en_az: 5, en_cok: 10, aralik_ms: 100 } }],
    ],
  },
  'gecikme-paralel': {
    yanitlar: [[{ tur: 'damgali', sayi: PARCA, desen: { ad: 'paralel', aralik_ms: 20 } }]],
  },
  kucuk: {
    yanitlar: [[{ tur: 'damgali', sayi: 150, desen: { ad: 'duzenli', aralik_ms: 20 } }]],
  },
  sel: { yanitlar: [[{ tur: 'sel', parca_bayt: 4096 }]] },
  uzun: {
    yanitlar: [[{ tur: 'damgali', sayi: 600, desen: { ad: 'duzenli', aralik_ms: 100 } }]],
  },
};

const AYARLAR = { sart: 'kabul', anahtar: GECERLI_ANAHTAR, senaryolar: SENARYOLAR };

const sonuc = { platform: process.platform, tarih: new Date().toISOString() };
const yuvarla = (x) => Math.round(x * 10) / 10;
const MB = 1024 * 1024;
const bellek = () => Math.max(0, ...surecler().map((s) => s.bellek));

after(() => {
  const dizin = join(import.meta.dirname, '..', 'sonuclar');
  mkdirSync(dizin, { recursive: true });
  writeFileSync(join(dizin, `${process.platform}.json`), JSON.stringify(sonuc, null, 2));
  console.log(JSON.stringify(sonuc, null, 2));
});

/** Kayıttaki bitmiş tur sayısı ve sonuncusunun nedeni. */
function bitenler(k, oturum) {
  const t = k.kayit(oturum).filter((y) => y.tip === 'TurBitti');
  return { sayi: t.length, son: t.at(-1)?.neden.tip };
}

describe('kabul kapısı: aynı oturumda', () => {
  let k;
  let tur = 0;
  before(async () => {
    k = await Kalfa.ac(AYARLAR);
    await k.pencere('ana');
    await k.yeniOturum();
    // Kanca ilk dinlenen oturumla kurulur.
    await k.b.waitUntil(() => k.b.execute(() => Boolean(window.__kalfaE2e)), { timeout: 10000 });
    // Isınma turu ölçülmez: uygulamanın ilk turu webview'ın ilk çizimlerini ve
    // derleyici ısınmasını da içerir; kabul kapısı akışın sürekli durumdaki
    // gecikmesini sorar.
    await k.gonder('isinma');
    assert.equal(await k.turSonuBekle(++tur), 'Normal');
  });
  after(async () => {
    await k?.kapat();
  });

  for (const desen of ['duzenli', 'yigin', 'paralel']) {
    it(`gecikme (${desen}): p95 < 50 ms`, async () => {
      await k.b.execute(() => window.__kalfaE2e.sifirla());
      await k.gonder(`gecikme-${desen}`);
      assert.equal(await k.turSonuBekle(++tur), 'Normal');
      const beklenen = desen === 'paralel' ? 2 * PARCA : PARCA;
      await k.b.waitUntil(
        async () => (await k.b.execute(() => window.__kalfaE2e.gecikmeler().length)) >= beklenen,
        { timeout: 5000, timeoutMsg: 'bütün parçalar ölçülmedi' },
      );
      const g = await k.b.execute(() => window.__kalfaE2e.gecikmeler());
      const ozet = {
        parca: g.length,
        p50: yuvarla(yuzdelik(g, 50)),
        p95: yuvarla(yuzdelik(g, 95)),
        p99: yuvarla(yuzdelik(g, 99)),
        en_cok: yuvarla(Math.max(...g)),
      };
      (sonuc.gecikme ??= {})[desen] = ozet;
      assert.equal(g.length, beklenen);
      assert.ok(ozet.p95 < 50, `p95 ${ozet.p95} ms: ${JSON.stringify(ozet)}`);
    });
  }

  it('küçültülmüş pencerede paketler işlenmeye devam eder', async () => {
    await k.gonder('kucuk');
    await k.metniBekle('[[e2e:5:');
    await k.b.minimizeWindow();
    try {
      // Tur yaklaşık 3 saniye sürer; pencere küçükken biterse arayüz
      // paketleri işlemeyi sürdürmüştür.
      assert.equal(await k.turSonuBekle(++tur, 15000), 'Normal');
    } finally {
      await k.b.maximizeWindow();
    }
    assert.ok(!(await k.konusma()).includes('Arayüz uzun süre yanıt vermediği'));
  });

  it('duraklayan arayüz: yoldaki veri krediyi, kuyruk bütçeyi aşmaz; tur iptal edilir', async () => {
    const oturum = await k.seciliOturum();
    const once = bitenler(k, oturum).sayi;
    await k.invoke('e2e_olcumleri_sifirla');
    const bellekOnce = bellek();
    let bellekEnCok = bellekOnce;

    await k.b.execute(() => window.__kalfaE2e.duraklat());
    await k.gonder('sel');
    await sureyleBekle(
      () => {
        bellekEnCok = Math.max(bellekEnCok, bellek());
        return bitenler(k, oturum).sayi > once;
      },
      30000,
      'bütçe dolunca tur iptal edilmedi',
    );
    assert.equal(bitenler(k, oturum).son, 'Iptal');
    const bekleyen = await k.b.execute(() => window.__kalfaE2e.bekleyen());

    await k.b.execute(() => window.__kalfaE2e.surdur());
    await k.metniBekle('Arayüz uzun süre yanıt vermediği için oturum durduruldu.');
    const o = await k.invoke('e2e_olcumler');
    sonuc.duraklama = {
      ...o,
      duraklarken_gelen_paket: bekleyen,
      bellek_once_mb: yuvarla(bellekOnce / MB),
      bellek_en_cok_mb: yuvarla(bellekEnCok / MB),
    };
    assert.ok(bekleyen <= o.kredi_paket, `duraklarken ${bekleyen} paket geldi`);
    assert.ok(o.yolda_paket_en_cok <= o.kredi_paket, JSON.stringify(o));
    assert.ok(o.yolda_bayt_en_cok <= o.kredi_bayt, JSON.stringify(o));
    assert.ok(o.kuyruk_olay_en_cok <= o.butce_olay, JSON.stringify(o));
    assert.ok(o.kuyruk_bayt_en_cok <= o.butce_bayt, JSON.stringify(o));
    assert.ok(o.kuyruk_bayt_en_cok > o.butce_bayt / 2, `kuyruk dolmadı: ${JSON.stringify(o)}`);
    // Akıl sağlığı: sürecin belleği bütçenin birkaç katından fazla büyümez.
    assert.ok(bellekEnCok - bellekOnce < 256 * MB, JSON.stringify(sonuc.duraklama));
  });

  it('tur sürerken sayfa yeniden yüklenirse tur iptal edilir ve oturum kayıttan açılır', async () => {
    await k.yeniOturum();
    const oturum = await k.seciliOturum();
    await k.gonder('uzun');
    await k.metniBekle('[[e2e:3:');
    await k.b.refresh();
    await k.pencere('ana');
    // Yeni sayfa son oturumu kayıttan açar; kayıt kilidi bırakılmıştır.
    assert.equal(await k.turSonuBekle(1), 'Iptal');
    assert.deepEqual(bitenler(k, oturum), { sayi: 1, son: 'Iptal' });
    await k.gonder('devam');
    await k.metniBekle('Yanıt: devam');
    assert.equal(await k.turSonuBekle(2), 'Normal');
  });
});

describe('kabul kapısı: pencere kapanması', () => {
  it('tur sürerken ana pencere kapanırsa tur iptal edilir', async () => {
    const k = await Kalfa.ac(AYARLAR);
    try {
      await k.pencere('ana');
      await k.yeniOturum();
      const oturum = await k.seciliOturum();
      await k.gonder('uzun');
      await k.metniBekle('[[e2e:3:');
      // Kapatma düğmesinin yolu: komutun yanıtı beklenmez, pencere kapanır.
      await k.b.execute(() => {
        window.__TAURI_INTERNALS__.invoke('e2e_ana_pencereyi_kapat');
      });
      await sureyleBekle(() => surecler().length === 0, 15000, 'Kalfa kapanmadı');
      assert.deepEqual(bitenler(k, oturum), { sayi: 1, son: 'Iptal' });
    } finally {
      await k.kapat();
    }
  });
});

describe('kabul kapısı: uzun oturum', () => {
  let k;
  before(async () => {
    k = await Kalfa.ac(AYARLAR);
    await k.pencere('ana');
  });
  after(async () => {
    await k?.kapat();
  });

  /** Kaydın satırına tıklayıp son mesaj çizilene kadar geçen süre. */
  const ac = (tur) =>
    k.b.executeAsync((tur, bitti) => {
      const baslik = `(${tur} turluk kayıt)`;
      const satir = [...document.querySelectorAll('button.oturum-satiri')].find((b) =>
        b.textContent.includes(baslik),
      );
      const t0 = performance.now();
      satir.click();
      const bak = () => {
        const son = [...document.querySelectorAll('article.mesaj.kullanici .duz')].at(-1);
        if (son?.textContent === `${tur}. soru ${baslik}`) {
          requestAnimationFrame(() =>
            setTimeout(
              () =>
                bitti({
                  sure_ms: performance.now() - t0,
                  dom_dugumu: document.getElementsByTagName('*').length,
                  js_yigini_mb: performance.memory
                    ? performance.memory.usedJSHeapSize / 1048576
                    : null,
                }),
              0,
            ),
          );
        } else setTimeout(bak, 2);
      };
      bak();
    }, tur);

  it('500 turluk kayıt 20 turluk kayıtla aynı mertebede açılır', async () => {
    await k.invoke('e2e_kayit_uret', { tur: 20 });
    await k.invoke('e2e_kayit_uret', { tur: 500 });
    await k.b.refresh();
    await k.pencere('ana');
    await k.b.waitUntil(
      async () => (await k.b.$$('button.oturum-satiri')).length === 2,
      { timeout: 10000, timeoutMsg: 'kayıtlar listede yok' },
    );
    // Önce uzun kayıt açılır: ısınmanın avantajı kısa kayda kalır.
    const b500 = await ac(500);
    const r500 = { ...b500, bellek_mb: yuvarla(bellek() / MB) };
    await bekle(500);
    const b20 = await ac(20);
    const r20 = { ...b20, bellek_mb: yuvarla(bellek() / MB) };
    for (const r of [r20, r500]) {
      r.sure_ms = yuvarla(r.sure_ms);
      if (r.js_yigini_mb !== null) r.js_yigini_mb = yuvarla(r.js_yigini_mb);
    }
    sonuc.uzun_oturum = { tur_20: r20, tur_500: r500 };
    assert.ok(
      r500.sure_ms < Math.max(10 * r20.sure_ms, r20.sure_ms + 500),
      JSON.stringify(sonuc.uzun_oturum),
    );
    assert.ok(r500.dom_dugumu < 1.5 * r20.dom_dugumu, JSON.stringify(sonuc.uzun_oturum));
  });
});
```

- [ ] **Step 3: Pencereyi 8 ms'ye indir**

`crates/kalfa-core/src/aktarim.rs`'de `AktarimAyarlari`'nın açıklamasını ve varsayılanını şöyle yap:

```rust
/// Paketleyici ayarları. Varsayılanlar gerçek arayüzle yapılan gecikme
/// ölçümüyle seçildi (1c tasarımı §11.4): 16 ms'lik pencerede token'ın ekrana
/// düşmesinin p95'i Windows'ta 41, Linux'ta 47 ms'ydi; 8 ms'de 26 ve 30–42 ms.
/// Paket sınırı ve kredi başlangıç değerinde kaldı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AktarimAyarlari {
    /// İlk olaydan sonra aynı pakete olay toplanan süre (varsayılan 8 ms).
    pub pencere: Duration,
```

ve `Default` içinde `pencere: Duration::from_millis(16),` → `pencere: Duration::from_millis(8),`.

Run: `cargo test -p kalfa-core --locked`
Expected: PASS.

- [ ] **Step 4: Bütün uçtan uca testleri koştur**

Run (Windows): `cd e2e && npm test`
Run (Linux): `cd e2e && xvfb-run -a npm test`
Expected: 21 test geçer; `e2e/sonuclar/<platform>.json` yazılır. Plan doğrulamasındaki değerler Görev 7'deki tablodadır. Sürücüler yoksa bu adımı atla ve PR'a yaz.

- [ ] **Step 5: Commit**

```bash
git add crates/kalfa-core e2e/testler/04-kabul.test.mjs
git commit -m "çekirdek, e2e: kabul kapısı testleri; paketleme penceresi 8 ms"
```

---

### Task 7: Belgeler ve son doğrulama

**Files:**
- Modify: `README.md`, `docs/GELISTIRME_DURUMU.md`

- [ ] **Step 1: README**

`README.md`'deki durum kutusunu (`> **Durum: çekirdek ve dosya araçları tamamlandı; …` ile başlayan dört satır) şununla değiştir:

```markdown
> **Durum: 1c tamamlandı; masaüstü uygulaması Windows ve Linux'ta çalışıyor.**
> Rust workspace'inde EVREN istemcisi (`kalfa-evren`), arayüzden bağımsız oturum
> çekirdeği (`kalfa-core`), beş yerleşik dosya aracı (`kalfa-tools`) ve
> masaüstü uygulaması (`kalfa-uygulama`, Tauri kabuğu `kalfa`) bulunuyor;
> Svelte arayüzü `ui/`'dadır. Kabuk aracı, yalıtım ve eklentiler sonraki
> aşamalarda. [Geliştirme durumu ve devam noktası](docs/GELISTIRME_DURUMU.md).
```

"Geliştirme" bölümünde "Bağlantı testleri Windows'ta sembolik bağlantı oluşturma izni gerektirir." satırından sonra, `---` ayracından önce şunu ekle:

````markdown
Linux'ta `kalfa` crate'i WebKitGTK geliştirme paketlerini ister
(`libwebkit2gtk-4.1-dev`).

Arayüz (`ui/`, Node `^22.22.2 || >=24.15.0`):

```sh
cd ui && npm ci && npm run check && npm test && npm run build
```

Uçtan uca testler gerçek uygulamayı sahte EVREN'li bir test derlemesiyle
`tauri-driver` üzerinden sürer:

```sh
cargo test -p kalfa-uygulama -p kalfa --features e2e
cd e2e && npm ci && npm test        # Linux'ta: xvfb-run -a npm test
```

- `tauri-driver` gerekir: `cargo install tauri-driver --locked`.
- **Windows:** WebView2 sürümüyle aynı `msedgedriver` gerekir. PATH'te değilse
  yolu `KALFA_E2E_EDGEDRIVER` ile verin.
- **Linux:** `webkit2gtk-driver` ve `xvfb` paketleri gerekir.

`npm test` önce arayüzü test kancasıyla (`ui/dist-e2e`), sonra uygulamayı
`target/e2e/` altına derler. Ölçümler `e2e/sonuclar/`'a yazılır.
````

- [ ] **Step 2: Durum dosyası**

`docs/GELISTIRME_DURUMU.md`'de:

1. İlk satırdaki `Güncelleme: 2026-09-22.` → `Güncelleme: 2026-09-24.`
2. "Verilen kararlar" listesinin 8. maddesinde `İlk olay 16 ms'lik pencere açar; pencerede biriken olaylar tek pakette` satırını şu iki satırla değiştir:

   ```markdown
      İlk olay 16 ms'lik pencere açar (1c-4'te ölçümle 8 ms'ye indi); pencerede
      biriken olaylar tek pakette
   ```

3. `### Tasarım belgesi` başlığından sonuna kadar olan bölümü şununla değiştir. Bu bölüm 1c-4 bölümünü de içerir:

```
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
```

Tablolardaki sayılar plan doğrulamasının ölçümleridir. Kendi Windows koşunda sayılar farklıysa Windows sütununu `e2e/sonuclar/win32.json`'daki değerlerle güncelle. Linux sütununu Claude incelemede günceller. Kontrol tablosundaki test sayılarını kendi koşunun sonuçlarıyla karşılaştır; farklıysa düzelt.

- [ ] **Step 3: Son doğrulama**

Run:

```bash
cargo test --workspace --locked
cargo test -p kalfa-uygulama -p kalfa --features e2e --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy -p kalfa -p kalfa-uygulama --features e2e --all-targets --locked -- -D warnings
cargo fmt --all -- --check
cargo tree -p kalfa-core -e normal | grep -E "kalfa-tools|tauri"
cargo tree -p kalfa-uygulama -e normal --features e2e | grep -E "tauri|wry"
cd ui && npm ci && npm run check && npm test && npm run build
cd ../e2e && npm ci && npm test
```

Expected: hepsi geçer; iki `cargo tree` satırı boş çıkar.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/GELISTIRME_DURUMU.md
git commit -m "docs: 1c-4 kabul ölçümü ve uçtan uca testler"
```

---

## İnceleme (Claude)

1. **Kodu planla karşılaştır.**
   - Sapma varsa gerekçesi PR'da mı?
   - E2e kodu yalnızca `cfg(feature = "e2e")` ve `MODE === 'e2e'` arkasında mı?
   - Capability dosyaları ve `tauri.conf.json` değişmemiş mi?
2. **İki platformda Rust ve arayüz.** Windows'ta ve WSL'de (Ubuntu 24.04) Görev 7'nin son doğrulaması. Linux'ta `kalfa` crate'i artık testlere girer (WebKitGTK kurulu).
3. **İki platformda uçtan uca.**
   - Windows'ta ve WSL'de (Xvfb) `cd e2e && npm test` koşar; 21 testin geçmesi beklenir.
   - Kararlılık için tam paket en az iki kez koşar.
   - Ölçümler durum dosyasına işlenir; Linux sütunu o koşudan doldurulur.
4. **Sürüm derlemesinde test komutu yok.**
   - `cargo build --release -p kalfa` (e2e özelliği olmadan) ile derlenen ikilide `e2e_olcumler` dizgisi bulunmaz.
   - `ui/dist`'te `__kalfaE2e` yoktur.
5. **Güvenlik sınırları.** Test komutlarının şart penceresinden reddedildiği ve normal derlemede kayıtlı olmadığı testlerde görülür. Ana pencereden `sartlari_kabul_et`'in reddi de gerçek webview'da görülür.
