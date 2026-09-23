# Kalfa 1c-2 — Tauri Kabuğu Uygulama Planı

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Kalfa'nın masaüstü kabuğunu kurmak: Tauri'siz uygulama mantığı (`kalfa-uygulama`: komut sözleşmesi, şart akışı, oturum tablosu, olay köprüsü, kimlik deposu) ve onu iki pencereye bağlayan ince Tauri kabuğu (`kalfa`: pencereler, 22 komut, pencere izinleri, yerel iletişim kutuları).

**Architecture:** Mantık `kalfa-uygulama`'dadır ve Tauri'ye bağlı değildir. Pencereye, yerel iletişim kutusuna, sistem tarayıcısına ve kanala `arayuz` modülündeki trait'lerle erişir; testler sahteleriyle koşar ve WebKitGTK istemez. `kalfa` crate'i bu trait'leri Tauri ile uygular. Her Tauri komutu `Uygulama`'nın aynı adlı yöntemine iner. Hangi pencerenin hangi komutu çağırabileceğini `capabilities/` belirler; Tauri'nin ve eklentilerin kendi komutları hiçbir pencereye açık değildir. Pencereleri yalnızca Rust açar; webview başka bir adrese gidemez, yeni pencere açamaz. Şartlar kullanıcı adına asla kabul edilmez: kabul isteği yalnızca şart penceresinin `sartlari_kabul_et` komutundan gider.

**Tech Stack:** Rust 1.98.1 · tauri 2.11.6 · tauri-build 2.6.3 · tauri-plugin-dialog 2.7.3 · tauri-plugin-opener 2.5.5 · tauri-plugin-single-instance 2.4.5 · keyring 4.2 · url 2.5 · tokio · async-trait · kalfa-core / kalfa-evren / kalfa-tools

**Spec:** `docs/superpowers/specs/2026-09-22-kalfa-1c-design.md` (1c tasarımı) — §3.1, §3.2, §4, §5, §6, §6.1, §7, §10, §11.2, §11.6, §13.

**Kod hakkında:** Bu plandaki kodun tamamı plan yazılırken ayrı bir kopyada derlendi ve doğrulandı (2026-09-23). Plan commit'indeki iskeletten başlayıp her görevin sonu ayrı ayrı denendi:
- **Windows 11 (her görev ayrı ayrı):** `cargo build --locked`, `cargo clippy -- -D warnings` ve `cargo test` geçti; son durumda 300 test geçti, 4 canlı test atlandı.
- **Linux (Ubuntu 24.04, WSL2):** `kalfa` crate'i dışındaki bütün crate'lerde clippy ve testler geçti.
- **Gerçek pencere (Windows, WebView2):** uygulama açıldı. Şart penceresinde anahtar kaydı ve kabul, ana pencerenin açılması, pencere izinleri, gezinme ve `window.open` engeli sahte bir EVREN sunucusuna karşı denendi (bkz. İnceleme).

Kodu olduğu gibi kullan. Bir şey yine de derlenmezse testin sınadığı davranışı değiştirmeden düzelt; davranış değişmesi gerekiyorsa dur ve PR'a yorum yaz.

**Bu planın kapsamı dışında:** iki pencerenin ekranları (1c-3b) · gerçek pencerede uçtan uca testler, kabul ölçümleri ve `e2e` özelliği (1c-4) · paketleme ve imzalama.

## PR ile yürütme

Kullanıcı kararı (2026-09-23): kodu Codex yazar, Claude inceler.

- **Dal ve PR:** çalışma dalı `1c-2/tauri-kabugu`, PR'ın hedefi `kalfa-asama-1`.
- **Hazır gelenler (plan commit'i):** plan commit'inde dört grup dosya hazır gelir. Bunları değiştirme; bağımlılık eklemen ya da sürüm değiştirmen gerekirse dur ve PR'a yorum yaz.
  - Kök `Cargo.toml` (iki yeni üye; `keyring`, `url`) ve `Cargo.lock`, doğrulanmış sürümlerle.
  - İki crate'in `Cargo.toml`'u, `crates/kalfa/tauri.conf.json`, `crates/kalfa/icons/` ve `crates/kalfa/src/main.rs`.
  - Derlenebilir boş gövdeler: `crates/kalfa-uygulama/src/lib.rs`, `crates/kalfa/src/lib.rs`, `crates/kalfa/build.rs`. Görevler bunların üzerine yazar.
  - Sözleşme düzeltmesi: tasarım belgesi §6.1 ile `ui/src/ipc/tipler.ts` ve `ui/src/ipc/sartlar.ts`.
- **Sözleşme düzeltmesi neden var:** 1c-1'in şart kapısı eşzamanlı ret ile kabulü ayırıyor. Bu yüzden `KabulYaniti`'ne `{ tip: "Kilitli" }` eklendi; `sart_metnini_goster` artık `SartMetni` değil pencerenin yeni durumunu (`SartPenceresiDurumu`) döner.
- **Ortam:**
  - **Windows:** Rust 1.98.1 (depodaki `rust-toolchain.toml`) ve Visual Studio C++ derleme araçları. `kalfa` crate'inin testleri Tauri'nin mock runtime'ında koşar; WebView2 gerekmez.
  - **Linux'ta `kalfa` crate'i:** derlemek için WebKitGTK geliştirme paketleri gerekir (Ubuntu 24.04: `libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`). Bunlar kurulamıyorsa 3. görevin testlerini koşturamadığını PR'a yaz; inceleyici Windows'ta doğrular. Diğer crate'ler bu paketleri istemez.
  - **Ağ:** `cargo` ilk derlemede crates.io'dan indirir.
  - **Canlı testler:** Canlı EVREN testleri (`#[ignore]`) koşturulmaz. Canlı `POST /v1/terms/accept` hiçbir koşulda denenmez.
- **Sıra:** görevler sırayla. Her görev planda yazan mesajla kendi commit'ini alır; her commit'ten sonra push'la. Geçmiş yeniden yazılmaz, force-push yapılmaz.
- **Soru ve sapma:** plandan sapman gerekirse dur ve PR'a yorum yaz; kendi başına karar verme.
- **İnceleme:** Claude PR'a yorum yazar. Windows'ta ve Linux'ta (`kalfa` crate'i hariç) doğrular. Uygulamayı sahte EVREN sunucusuyla gerçek pencerede dener. İnceleme temizse birleştirir.

## Global Constraints

- **Araçlar:** Rust 1.98.1; bütün `cargo` komutları `--locked` ile. Sürümler plan commit'indeki `Cargo.toml`/`Cargo.lock`'ta sabittir.
- **Bağımlılık yönü (1c tasarımı §3.2, §11.6):** `kalfa-uygulama` `tauri`'ye ya da bir webview kütüphanesine bağlanmaz. `cargo tree -p kalfa-uygulama -e normal | grep -E "tauri|wry|webkit"` boş olmalı. `kalfa-core` yine `kalfa-tools`'a ve `tauri`'ye bağlanmaz.
- **Şartlar (üst belge §11.8):** Kalfa şartları kullanıcı adına kabul etmez. Kabul isteğine giden tek yol `Uygulama::sartlari_kabul_et`, onu da yalnızca şart penceresi çağırabilir. Testlerde sahte sağlayıcının `kabul_edilen_surumler()` listesi yalnızca bu çağrıdan sonra dolar.
- **Pencere sınırları (1c tasarımı §6):**
  - Pencereleri yalnızca Rust açar: `tauri.conf.json`'da `windows: []`.
  - Webview yalnızca uygulamanın kendi sayfalarına gider (geliştirmede Vite adresine de). `window.open` reddedilir.
  - Hiçbir capability Tauri'nin (`core:*`) ya da eklentilerin (`dialog:*`, `opener:*`) izinlerini vermez.
  - Onay gerektiren işler (izin kipini yükseltmek, bağlantı açmak) webview'daki düğmeyle değil, yerel iletişim kutusuyla doğrulanır.
- **Komut sözleşmesi (1c tasarımı §6.1):** komut ve argüman adları snake_case, `ui/src/ipc/*.ts`'dekiyle aynı. Komutların hepsi `async`: Tauri eşzamanlı komutları ana iş parçacığında çalıştırır ve orada pencere kurmak Windows'ta kilitlenir.
- **Kimlik deposu (1c tasarımı §7.2):** anahtar işletim sisteminin deposunda; depo yoksa ya da yazılamıyorsa yalnızca bellekte, ve bu kullanıcıya söylenir (`anahtar_deposu: "Bellekte"`). Anahtar hiçbir günlüğe, kayda, olaya ya da komut yanıtına yazılmaz.
- **Kilitler:** `Mutex` kilidi hiçbir `await` boyunca tutulmaz.
- **Dil:** kullanıcıya giden her metin Türkçe ve "siz" diliyle; ünlem yok.
- **Kod:** kütüphane kodunda `unwrap()`/`expect()` yok (`test_destek.rs`, testler ve `build.rs` hariç). Her görevin sonunda `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` ve o görevin testleri temiz.
- **Commit:** mesajlar Türkçe (`kalfa-uygulama: …`, `kalfa: …`, `docs: …`). Codex'in commit'lerine Claude satırı eklenmez.
- **Dokunulmayanlar:** `ui/` (sözleşme düzeltmesi plan commit'inde yapıldı), `design/`, `crates/kalfa-core`, `crates/kalfa-evren`, `crates/kalfa-tools`, `.env`, `Screenshot*.png`, kök dizindeki `tasks.png`.

## Dosya Yapısı

```
Cargo.toml, Cargo.lock                    iki yeni üye; keyring, url (plan commit'inde hazır)
.gitignore                                crates/kalfa/gen/, permissions/autogenerated/ (hazır)
crates/kalfa-uygulama/                    Tauri'siz uygulama mantığı
  Cargo.toml                              (hazır)
  src/lib.rs                              modüller (Görev 1'de ara hâli, Görev 2'de son hâli)
  src/sozlesme.rs                         §6.1 tipleri
  src/hata.rs                             KomutHatasi
  src/modeller.rs, dizin.rs               model listesi; veri ve kayıt dizinleri
  src/guvenlik.rs                         gezinme, bağlantı ve kip yükseltme kuralları
  src/anahtar.rs                          kimlik deposu ve bellek yedeği
  src/evren.rs                            anahtarı değiştirilebilen EVREN sağlayıcısı
  src/arayuz.rs                           Iletisim, Acici, Pencereler, KanalYazici trait'leri
  src/uygulama.rs                         Uygulama, Bilesenler, iç durum
  src/sartlar.rs                          açılış ve şart penceresi akışı
  src/oturumlar.rs                        oturum tablosu ve oturum komutları
  src/kopru.rs                            paketleyici → kanal görevi
  src/test_destek.rs                      sahteler (özellik: test-destek)
  tests/kurallar.rs, sart_akisi.rs, oturum_tablosu.rs
crates/kalfa/                             Tauri kabuğu
  Cargo.toml, tauri.conf.json, icons/, src/main.rs   (hazır)
  build.rs                                uygulama manifest'i; Windows test manifest'i
  windows-test.manifest
  capabilities/ana-pencere.json, sart-penceresi.json
  src/lib.rs                              calistir, eklentiler, kurulum
  src/komut_adlari.rs                     iki pencerenin komut listesi
  src/komutlar.rs                         22 Tauri komutu
  src/pencereler.rs, iletisim.rs, kanal.rs
  tests/izinler.rs                        ACL ve yapılandırma testleri
docs/GELISTIRME_DURUMU.md                 1c-2 sonuçları (değişir)
```

---

### Task 1: `kalfa-uygulama` — sözleşme, pencere kuralları ve kimlik deposu

Komut sözleşmesinin Rust tipleri, komut hata tipi, model listesi, veri dizinleri, pencere sınırlarının Tauri'siz kuralları, kimlik deposu, anahtarı değiştirilebilen EVREN sağlayıcısı ve dış uçların trait'leri.

**Files:**
- Var (plan commit'inde): `crates/kalfa-uygulama/Cargo.toml`
- Modify: `crates/kalfa-uygulama/src/lib.rs` (ara hâli)
- Create: `crates/kalfa-uygulama/src/{sozlesme,hata,modeller,dizin,guvenlik,anahtar,evren,arayuz}.rs`
- Test: `crates/kalfa-uygulama/tests/kurallar.rs`

**Interfaces:**
- Consumes (1c-1):
  - `kalfa_core::gecmis::{OturumOzeti, OturumDurumu, GecmisSayfasi, DurumOzeti}`
  - `kalfa_core::sart_kapisi::SartMetni`
  - `kalfa_core::saglayici::{Saglayici, EvrenSaglayici, SaglayiciAkisi}` (`sart_durumu` dahil)
  - `kalfa_core::oturum::OturumHatasi`
  - `kalfa_core::kimlik::{OturumId, CagriId}`
  - `kalfa_core::muhur::OnayMuhru`, `kalfa_core::protokol::OnayKarari`
  - `kalfa_core::aktarim::KanalMesaji`
  - `kalfa_evren::{EvrenError, EvrenIstemcisi, ApiAnahtari}`
- Produces:
  - **`sozlesme`:** `BaglantiDurumu { Var, Yok }`, `DepoTuru { Kalici, Bellekte }`, `ModelProfili`, `ModelSecenegi`, `UygulamaDurumu`, `OturumSatiri` (+ `From<OturumOzeti>`), `CalismaAlani { kimlik: u64, yol: String }`, `SaltOkunurNedeni`, `OturumAcilisi`, `OnayGirdisi` (Deserialize), `AnahtarNedeni`, `SartPenceresiDurumu`, `AnahtarSonucu`, `KabulYaniti { Kabul, YeniMetin { metin }, Kilitli }`.
  - **`hata`:** `HataKodu`; `KomutHatasi { kod, mesaj }` (Serialize, Error). Yapıcılar: `yeni`, `oturum_yok`, `salt_okunur`, `gecersiz`, `ic`, `kayit(io::Error)`. Dönüşümler: `From<io::Error>`, `From<OturumHatasi>`, `From<EvrenError>`.
  - **`modeller`:** `VARSAYILAN_MODEL`, `modeller() -> Vec<ModelSecenegi>`, `bilinen_model_mi(&str) -> bool`.
  - **`dizin`:** `VERI_DIZINI_DEGISKENI`, `veri_dizini(&Path, Option<OsString>) -> PathBuf`, `oturum_dizini(&Path) -> PathBuf`, `kayit_yolu(&Path, OturumId) -> PathBuf`.
  - **`guvenlik`:** `gezinme_izinli(&Url, Option<&Url>) -> bool`, `acilabilir_adres(&str) -> Option<Url>`, `yukseltme_mi(Kip, Kip) -> bool`, `kip_adi(Kip) -> &'static str`, `yukseltme_aciklamasi(Kip) -> &'static str`.
  - **`anahtar`:** `AnahtarDeposu` trait (`oku`, `yaz`), `DepoHatasi(pub String)`, `SistemDeposu::{ac, SERVIS, KULLANICI}`, `BellekDeposu`. `AnahtarKasasi` metotları: `yeni(Option<Box<dyn AnahtarDeposu>>)`, `oku() -> Option<String>`, `yaz(&str) -> DepoTuru`, `tur() -> DepoTuru`.
  - **`evren`:** `TABAN_DEGISKENI`, `SaglayiciUretici` trait (`uret(&str) -> Result<Arc<dyn Saglayici>, EvrenError>`), `EvrenUretici { taban }`, `DegisirSaglayici` (`Default`, `ayarla`, `Saglayici` uygular).
  - **`arayuz`:** `Iletisim` (async `klasor_sec`, `onayla`), `Acici::ac(&Url)`, `Pencereler { ana_ac, sartlar_goster, sartlar_kapat }`, `KanalYazici::gonder(KanalMesaji)`.

- [ ] **Step 1: Başarısız testi yaz**

`crates/kalfa-uygulama/tests/kurallar.rs`:

```rust
use std::ffi::OsString;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use kalfa_core::izin::Kip;
use kalfa_core::kimlik::OturumId;
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::SahteSaglayici;
use kalfa_evren::EvrenError;
use kalfa_uygulama::anahtar::{AnahtarDeposu, AnahtarKasasi, BellekDeposu, DepoHatasi};
use kalfa_uygulama::dizin::{kayit_yolu, oturum_dizini, veri_dizini};
use kalfa_uygulama::evren::DegisirSaglayici;
use kalfa_uygulama::guvenlik::{acilabilir_adres, gezinme_izinli, yukseltme_mi};
use kalfa_uygulama::modeller::{VARSAYILAN_MODEL, bilinen_model_mi, modeller};
use kalfa_uygulama::sozlesme::{DepoTuru, ModelProfili};
use url::Url;

fn url(s: &str) -> Url {
    Url::parse(s).unwrap()
}

/// Kalıcı depo gibi davranan ama `yazilabilir` kapalıyken yazılamayan depo
/// (Secret Service geçici olarak çalışmıyor).
#[derive(Clone, Default)]
struct AcilirDepo {
    yazilabilir: Arc<AtomicBool>,
    ic: Arc<BellekDeposu>,
}

impl AnahtarDeposu for AcilirDepo {
    fn oku(&self) -> Result<Option<String>, DepoHatasi> {
        self.ic.oku()
    }

    fn yaz(&self, anahtar: &str) -> Result<(), DepoHatasi> {
        if !self.yazilabilir.load(Ordering::SeqCst) {
            return Err(DepoHatasi("Secret Service yok".into()));
        }
        self.ic.yaz(anahtar)
    }
}

#[test]
fn pencereler_yalnizca_uygulamanin_sayfalarina_gider() {
    let gelistirme = url("http://localhost:5173");
    for izinli in [
        "tauri://localhost/index.html",
        "http://tauri.localhost/sartlar.html",
        "https://tauri.localhost/",
    ] {
        assert!(gezinme_izinli(&url(izinli), None), "{izinli}");
    }
    assert!(gezinme_izinli(
        &url("http://localhost:5173/index.html"),
        Some(&gelistirme)
    ));
    for yasak in [
        "http://localhost:5173/index.html",
        "https://evren.example/",
        "http://localhost:5174/",
        "file:///C:/Windows/win.ini",
        "data:text/html,x",
        "javascript:alert(1)",
        "tauri://baska/",
    ] {
        assert!(!gezinme_izinli(&url(yasak), None), "{yasak}");
    }
    assert!(!gezinme_izinli(
        &url("http://localhost:5174/"),
        Some(&gelistirme)
    ));
}

#[test]
fn yalnizca_http_ve_https_acilir() {
    assert_eq!(
        acilabilir_adres("https://ornek.com/yol?a=1").map(|u| u.to_string()),
        Some("https://ornek.com/yol?a=1".to_string())
    );
    assert!(acilabilir_adres("http://ornek.com").is_some());
    for kotu in [
        "javascript:alert(1)",
        "file:///etc/passwd",
        "data:text/html,x",
        "mailto:a@b.c",
        "/goreli",
        "ornek.com",
        "http://",
    ] {
        assert!(acilabilir_adres(kotu).is_none(), "{kotu}");
    }
}

#[test]
fn saga_gecis_yukseltmedir() {
    use Kip::*;
    assert!(yukseltme_mi(Inceleme, Siki));
    assert!(yukseltme_mi(Siki, Proje));
    assert!(yukseltme_mi(Siki, Serbest));
    assert!(!yukseltme_mi(Serbest, Siki));
    assert!(!yukseltme_mi(Proje, Inceleme));
    assert!(!yukseltme_mi(Siki, Siki));
}

#[test]
fn veri_dizini_platforma_gore_ve_ortamla_degisir() {
    let taban = Path::new("veri");
    let beklenen = if cfg!(windows) { "Kalfa" } else { "kalfa" };
    assert_eq!(veri_dizini(taban, None), taban.join(beklenen));
    assert_eq!(
        veri_dizini(taban, Some(OsString::new())),
        taban.join(beklenen)
    );
    assert_eq!(
        veri_dizini(taban, Some(OsString::from("baska"))),
        Path::new("baska")
    );
    let oturum = OturumId::yeni();
    assert_eq!(
        kayit_yolu(&oturum_dizini(Path::new("v")), oturum),
        Path::new("v")
            .join("oturumlar")
            .join(format!("{oturum}.jsonl"))
    );
}

#[test]
fn model_listesi() {
    let liste = modeller();
    assert_eq!(liste.len(), 4);
    assert!(
        liste
            .iter()
            .any(|m| m.kimlik == "glm-5.3" && m.profil == ModelProfili::Derin)
    );
    assert!(bilinen_model_mi(VARSAYILAN_MODEL));
    assert!(bilinen_model_mi("auto"));
    assert!(!bilinen_model_mi("gpt-5"));
}

#[test]
fn kasa_kalici_depoya_yazar() {
    let kasa = AnahtarKasasi::yeni(Some(Box::new(BellekDeposu::default())));
    assert_eq!(kasa.tur(), DepoTuru::Kalici);
    assert_eq!(kasa.oku(), None);
    assert_eq!(kasa.yaz("evren_llm_a"), DepoTuru::Kalici);
    assert_eq!(kasa.oku().as_deref(), Some("evren_llm_a"));
}

#[test]
fn kasa_depo_yazilamazsa_bellekte_tutar() {
    let kasa = AnahtarKasasi::yeni(Some(Box::new(AcilirDepo::default())));
    assert_eq!(kasa.yaz("evren_llm_a"), DepoTuru::Bellekte);
    assert_eq!(kasa.tur(), DepoTuru::Bellekte);
    assert_eq!(kasa.oku().as_deref(), Some("evren_llm_a"));
}

#[test]
fn kalici_depoya_yazilan_anahtari_bellekteki_eskisi_golgelemez() {
    let depo = AcilirDepo::default();
    let kasa = AnahtarKasasi::yeni(Some(Box::new(depo.clone())));
    assert_eq!(kasa.yaz("evren_llm_eski"), DepoTuru::Bellekte);
    depo.yazilabilir.store(true, Ordering::SeqCst);
    assert_eq!(kasa.yaz("evren_llm_yeni"), DepoTuru::Kalici);
    assert_eq!(kasa.oku().as_deref(), Some("evren_llm_yeni"));
}

#[test]
fn kasa_depo_yoksa_bellekte_tutar() {
    let kasa = AnahtarKasasi::yeni(None);
    assert_eq!(kasa.tur(), DepoTuru::Bellekte);
    assert_eq!(kasa.yaz("evren_llm_a"), DepoTuru::Bellekte);
    assert_eq!(kasa.oku().as_deref(), Some("evren_llm_a"));
}

#[test]
fn bellek_deposu_okur_ve_yazar() {
    let depo = BellekDeposu::default();
    assert_eq!(depo.oku().unwrap(), None);
    depo.yaz("x").unwrap();
    assert_eq!(depo.oku().unwrap().as_deref(), Some("x"));
}

#[tokio::test]
async fn degisir_saglayici_anahtarsiz_401_doner_sonra_yenisini_kullanir() {
    let degisir = DegisirSaglayici::default();
    assert!(matches!(
        degisir.sart_durumu().await,
        Err(EvrenError::AnahtarGecersiz)
    ));
    let ilk = SahteSaglayici::yeni(Vec::new());
    degisir.ayarla(ilk.clone());
    assert_eq!(degisir.sart_metni().await.unwrap().version, 1);
    let ikinci = SahteSaglayici::yeni(Vec::new());
    ikinci.sart_surumunu_degistir(7);
    degisir.ayarla(ikinci as Arc<dyn Saglayici>);
    assert_eq!(degisir.sart_metni().await.unwrap().version, 7);
    assert!(ilk.kabul_edilen_surumler().is_empty());
}
```

- [ ] **Step 2: Testin başarısız olduğunu gör**

Run: `cargo test -p kalfa-uygulama --locked --test kurallar`
Expected: FAIL. Derleme hatası: `could not find anahtar in kalfa_uygulama` ve benzerleri.

- [ ] **Step 3: Modülleri yaz**

`crates/kalfa-uygulama/src/lib.rs` (ara hâli; Görev 2 son hâlini yazar):

```rust
//! Kalfa masaüstü uygulamasının Tauri'siz mantığı (1c tasarımı §3.1, §3.2).
//!
//! `kalfa` crate'i bunu Tauri komutlarına ve pencerelerine bağlar. Pencereye,
//! yerel iletişim kutusuna, sistem tarayıcısına ve kanala trait'lerle erişilir
//! (`arayuz`); testler sahteleriyle koşar ve WebKitGTK istemez.

pub mod anahtar;
pub mod arayuz;
pub mod dizin;
pub mod evren;
pub mod guvenlik;
pub mod hata;
pub mod modeller;
pub mod sozlesme;
```

`crates/kalfa-uygulama/src/sozlesme.rs`:

```rust
//! Komut sözleşmesinin uygulamaya ait tipleri (1c tasarımı §6.1). JSON
//! biçimleri arayüzdeki `ui/src/ipc/tipler.ts` ile aynıdır; biri değişirse
//! öbürü de aynı commit'te değişir.

use kalfa_core::gecmis::{DurumOzeti, GecmisSayfasi, OturumDurumu, OturumOzeti};
use kalfa_core::kimlik::{CagriId, OturumId};
use kalfa_core::muhur::OnayMuhru;
use kalfa_core::protokol::OnayKarari;
use kalfa_core::sart_kapisi::SartMetni;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BaglantiDurumu {
    Var,
    /// Açılışta EVREN'e ulaşılamadı; ana pencere şerit gösterir (1c tasarımı §7.1).
    Yok,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DepoTuru {
    /// İşletim sisteminin kimlik deposu.
    Kalici,
    /// Depo yok; anahtar Kalfa açık kaldıkça bellekte (1c tasarımı §7.2).
    Bellekte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ModelProfili {
    Hizli,
    Dengeli,
    Derin,
    Otomatik,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModelSecenegi {
    pub kimlik: String,
    pub profil: ModelProfili,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UygulamaDurumu {
    pub baglanti: BaglantiDurumu,
    pub anahtar_deposu: DepoTuru,
    pub modeller: Vec<ModelSecenegi>,
    pub varsayilan_model: String,
    /// Bu çalıştırmada en son açılan ya da başlatılan oturum; yeniden
    /// yüklenen ana pencere onu kayıttan açar (1c tasarımı §5.6).
    pub son_oturum: Option<OturumId>,
}

/// Oturum listesinin bir satırı (1c tasarımı §5.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OturumSatiri {
    /// Kayıt okunamadıysa `None`; o satır açılamaz.
    pub oturum: Option<OturumId>,
    /// Kayıt dosyasının adı.
    pub dosya: String,
    pub baslik: Option<String>,
    pub calisma_alani: Option<String>,
    pub son_degisiklik_ms: u64,
    pub durum: OturumDurumu,
}

impl From<OturumOzeti> for OturumSatiri {
    fn from(o: OturumOzeti) -> Self {
        Self {
            oturum: o.oturum,
            dosya: o
                .yol
                .file_name()
                .map(|ad| ad.to_string_lossy().into_owned())
                .unwrap_or_default(),
            baslik: o.baslik,
            calisma_alani: o.calisma_alani.map(|yol| yol.display().to_string()),
            son_degisiklik_ms: o.son_degisiklik_ms,
            durum: o.durum,
        }
    }
}

/// Yerel klasör seçiciden gelen çalışma alanı. Arayüz yol göndermez; bu
/// kimlikle başvurur. Kimlik yalnızca bu çalıştırmada geçerlidir (1c tasarımı
/// §5.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CalismaAlani {
    pub kimlik: u64,
    pub yol: String,
}

/// Kayıttan açılan oturumun neden yalnızca okunabildiği (1c tasarımı §10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum SaltOkunurNedeni {
    CalismaAlaniYok {
        yol: String,
    },
    /// Kayıt çalışma alanını yazmıyor (1c'den önceki kayıt).
    CalismaAlaniKayitsiz,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OturumAcilisi {
    pub oturum: OturumId,
    pub calisma_alani: Option<String>,
    pub gecmis: GecmisSayfasi,
    pub ozet: DurumOzeti,
    pub salt_okunur: Option<SaltOkunurNedeni>,
}

/// Onay sayfasındaki tek bir çağrının kararı.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OnayGirdisi {
    pub cagri: CagriId,
    pub muhur: OnayMuhru,
    pub karar: OnayKarari,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AnahtarNedeni {
    /// Kimlik deposunda anahtar yok (ilk açılış).
    Yok,
    /// Kayıtlı anahtar artık geçerli değil (401).
    KayitliGecersiz,
    /// Kullanıcı "Anahtarı değiştir"i seçti.
    Degistir,
}

/// Şart penceresinin göstereceği görünüm (1c tasarımı §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum SartPenceresiDurumu {
    AnahtarGerekli {
        neden: AnahtarNedeni,
    },
    SartGerekli {
        metin: SartMetni,
    },
    /// Kullanıcı şartları reddetti (1c tasarımı §7.4).
    Kilitli,
    /// İş bitti; pencere kapanır.
    Tamam,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum AnahtarSonucu {
    /// 401: anahtar saklanmadı.
    Gecersiz,
    /// EVREN'e ulaşılamadı: anahtar saklanmadı.
    Ulasilamadi,
    Kaydedildi {
        depo: DepoTuru,
        sonraki: SartPenceresiDurumu,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tip")]
pub enum KabulYaniti {
    Kabul,
    /// Sürüm gösterilenle aynı değildi ya da bu arada değişti.
    YeniMetin {
        metin: SartMetni,
    },
    /// Aynı anda "Kabul etmiyorum" da verildi; ret geçerli.
    Kilitli,
}
```

`crates/kalfa-uygulama/src/hata.rs`:

```rust
//! Komutların hata tipi (1c tasarımı §6.1). `mesaj` kullanıcıya gösterilir
//! ("siz" dili); `kod` arayüzün ne yapacağını seçer.

use std::io;

use kalfa_core::oturum::OturumHatasi;
use kalfa_evren::EvrenError;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum HataKodu {
    OturumYok,
    /// Kayıt başka bir süreçte açık.
    KayitMesgul,
    SaltOkunur,
    GecersizGirdi,
    Evren,
    Ic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[error("{mesaj}")]
pub struct KomutHatasi {
    pub kod: HataKodu,
    pub mesaj: String,
}

impl KomutHatasi {
    pub fn yeni(kod: HataKodu, mesaj: impl Into<String>) -> Self {
        Self {
            kod,
            mesaj: mesaj.into(),
        }
    }

    pub fn oturum_yok() -> Self {
        Self::yeni(HataKodu::OturumYok, "Bu oturum açık değil.")
    }

    pub fn salt_okunur() -> Self {
        Self::yeni(
            HataKodu::SaltOkunur,
            "Bu oturum yalnızca okunabilir; çalışma alanı bulunamadı.",
        )
    }

    pub fn gecersiz(mesaj: impl Into<String>) -> Self {
        Self::yeni(HataKodu::GecersizGirdi, mesaj)
    }

    pub fn ic(mesaj: impl Into<String>) -> Self {
        Self::yeni(HataKodu::Ic, mesaj)
    }

    /// Oturum kaydı okunurken çıkan hata; kaydın olmaması `OturumYok`tur.
    pub fn kayit(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::NotFound {
            return Self::yeni(HataKodu::OturumYok, "Bu oturumun kaydı bulunamadı.");
        }
        e.into()
    }
}

impl From<io::Error> for KomutHatasi {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::ResourceBusy {
            return Self::yeni(
                HataKodu::KayitMesgul,
                "Bu oturum başka bir Kalfa sürecinde açık.",
            );
        }
        Self::ic(e.to_string())
    }
}

impl From<OturumHatasi> for KomutHatasi {
    fn from(e: OturumHatasi) -> Self {
        match e {
            OturumHatasi::Kayit(io) => io.into(),
            OturumHatasi::Kapali => Self::oturum_yok(),
            diger => Self::ic(diger.to_string()),
        }
    }
}

impl From<EvrenError> for KomutHatasi {
    fn from(e: EvrenError) -> Self {
        Self::yeni(HataKodu::Evren, e.to_string())
    }
}
```

`crates/kalfa-uygulama/src/modeller.rs`:

```rust
//! Model seçici (üst belge §6.4). Yalnızca bu kimlikler kabul edilir.

use crate::sozlesme::{ModelProfili, ModelSecenegi};

pub const VARSAYILAN_MODEL: &str = "glm-5.3";

const MODELLER: [(&str, ModelProfili); 4] = [
    ("qwen3.8-flash-next", ModelProfili::Hizli),
    ("deepseek-v4-flash", ModelProfili::Dengeli),
    ("glm-5.3", ModelProfili::Derin),
    ("auto", ModelProfili::Otomatik),
];

pub fn modeller() -> Vec<ModelSecenegi> {
    MODELLER
        .iter()
        .map(|(kimlik, profil)| ModelSecenegi {
            kimlik: (*kimlik).to_string(),
            profil: *profil,
        })
        .collect()
}

pub fn bilinen_model_mi(model: &str) -> bool {
    MODELLER.iter().any(|(kimlik, _)| *kimlik == model)
}
```

`crates/kalfa-uygulama/src/dizin.rs`:

```rust
//! Kalfa'nın veri dizini (üst belge §6.6, 1c tasarımı §5.1): Windows'ta
//! `%APPDATA%\Kalfa`, Linux'ta `$XDG_DATA_HOME/kalfa`. Oturum kayıtları onun
//! altındaki `oturumlar` dizinindedir; dosya adı oturum kimliğidir.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use kalfa_core::kimlik::OturumId;

/// Testlerde ve uçtan uca denemelerde veri dizinini değiştirir.
pub const VERI_DIZINI_DEGISKENI: &str = "KALFA_VERI_DIZINI";

/// `platform_veri_dizini`: işletim sisteminin kullanıcı veri dizini (Tauri'de
/// `path().data_dir()`). `ortam`: `KALFA_VERI_DIZINI`'nin değeri.
pub fn veri_dizini(platform_veri_dizini: &Path, ortam: Option<OsString>) -> PathBuf {
    if let Some(dizin) = ortam.filter(|d| !d.is_empty()) {
        return PathBuf::from(dizin);
    }
    platform_veri_dizini.join(if cfg!(windows) { "Kalfa" } else { "kalfa" })
}

pub fn oturum_dizini(veri_dizini: &Path) -> PathBuf {
    veri_dizini.join("oturumlar")
}

pub fn kayit_yolu(oturum_dizini: &Path, oturum: OturumId) -> PathBuf {
    oturum_dizini.join(format!("{oturum}.jsonl"))
}
```

`crates/kalfa-uygulama/src/guvenlik.rs`:

```rust
//! Pencere sınırlarının Tauri'siz kuralları (1c tasarımı §6).

use kalfa_core::izin::Kip;
use url::Url;

/// Pencereler yalnızca uygulamanın kendi sayfalarına gidebilir. Derlenmiş
/// uygulamanın adresi Windows'ta `http://tauri.localhost`, Linux'ta
/// `tauri://localhost`'tur; geliştirmede Vite sunucusunun adresi de izinlidir.
pub fn gezinme_izinli(url: &Url, gelistirme_adresi: Option<&Url>) -> bool {
    match url.scheme() {
        "tauri" => url.host_str() == Some("localhost"),
        "http" | "https" => {
            url.host_str() == Some("tauri.localhost")
                || gelistirme_adresi.is_some_and(|g| g.origin() == url.origin())
        }
        _ => false,
    }
}

/// `baglanti_ac`'ın açabileceği adres: yalnızca mutlak `http` ve `https`
/// (1c tasarımı §6, §9.1). Arayüz de aynı denetimi yapar; bu ikinci katmandır.
pub fn acilabilir_adres(adres: &str) -> Option<Url> {
    let url = Url::parse(adres).ok()?;
    (matches!(url.scheme(), "http" | "https") && url.host().is_some()).then_some(url)
}

fn sira(kip: Kip) -> u8 {
    match kip {
        Kip::Inceleme => 0,
        Kip::Siki => 1,
        Kip::Proje => 2,
        Kip::Serbest => 3,
    }
}

/// İnceleme < Sıkı < Proje < Serbest; sağa her geçiş yükseltmedir ve yerel
/// iletişim kutusunda onaylanır. Kipi düşürmek onay istemez.
pub fn yukseltme_mi(eski: Kip, yeni: Kip) -> bool {
    sira(yeni) > sira(eski)
}

pub fn kip_adi(kip: Kip) -> &'static str {
    match kip {
        Kip::Inceleme => "İnceleme",
        Kip::Siki => "Sıkı",
        Kip::Proje => "Proje",
        Kip::Serbest => "Serbest",
    }
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

`crates/kalfa-uygulama/src/anahtar.rs` (kalıcı depoya yazılan anahtar, daha önce belleğe düşmüş eski anahtarı temizler; yoksa `oku` eskisini döndürürdü):

```rust
//! Kimlik deposu (1c tasarımı §7.2). Anahtar işletim sisteminin deposunda
//! (Windows Credential Manager, Linux Secret Service) tutulur. Depo yoksa ya
//! da yazılamıyorsa anahtar yalnızca Kalfa açık kaldıkça bellekte durur ve
//! kullanıcıya söylenir. Anahtar hiçbir günlüğe, kayda ya da arayüz durumuna
//! yazılmaz.

use std::sync::Mutex;

use crate::sozlesme::DepoTuru;

#[derive(Debug, thiserror::Error)]
#[error("Kimlik deposu kullanılamadı: {0}")]
pub struct DepoHatasi(pub String);

/// Tek bir anahtarı saklayan depo.
pub trait AnahtarDeposu: Send + Sync {
    fn oku(&self) -> Result<Option<String>, DepoHatasi>;
    fn yaz(&self, anahtar: &str) -> Result<(), DepoHatasi>;
}

/// İşletim sisteminin kimlik deposu (`keyring` 4, `v1` özelliği).
pub struct SistemDeposu {
    giris: keyring::Entry,
}

impl SistemDeposu {
    pub const SERVIS: &str = "Kalfa";
    pub const KULLANICI: &str = "evren-api-anahtari";

    /// Depo kurulamazsa (ör. Linux'ta Secret Service yok) hata döner.
    pub fn ac() -> Result<Self, DepoHatasi> {
        keyring::Entry::new(Self::SERVIS, Self::KULLANICI)
            .map(|giris| Self { giris })
            .map_err(|e| DepoHatasi(e.to_string()))
    }
}

impl AnahtarDeposu for SistemDeposu {
    fn oku(&self) -> Result<Option<String>, DepoHatasi> {
        match self.giris.get_password() {
            Ok(anahtar) => Ok(Some(anahtar)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(DepoHatasi(e.to_string())),
        }
    }

    fn yaz(&self, anahtar: &str) -> Result<(), DepoHatasi> {
        self.giris
            .set_password(anahtar)
            .map_err(|e| DepoHatasi(e.to_string()))
    }
}

/// Bellekte tutan depo.
#[derive(Default)]
pub struct BellekDeposu {
    deger: Mutex<Option<String>>,
}

impl AnahtarDeposu for BellekDeposu {
    fn oku(&self) -> Result<Option<String>, DepoHatasi> {
        Ok(self.deger.lock().unwrap_or_else(|z| z.into_inner()).clone())
    }

    fn yaz(&self, anahtar: &str) -> Result<(), DepoHatasi> {
        *self.deger.lock().unwrap_or_else(|z| z.into_inner()) = Some(anahtar.to_string());
        Ok(())
    }
}

/// Uygulamanın kullandığı kasa: kalıcı depoyu dener, yazılamazsa belleğe düşer.
pub struct AnahtarKasasi {
    kalici: Option<Box<dyn AnahtarDeposu>>,
    bellek: BellekDeposu,
    tur: Mutex<DepoTuru>,
}

impl AnahtarKasasi {
    /// `kalici` yoksa (depo kurulamadı) anahtar yalnızca bellekte tutulur.
    pub fn yeni(kalici: Option<Box<dyn AnahtarDeposu>>) -> Self {
        let tur = if kalici.is_some() {
            DepoTuru::Kalici
        } else {
            DepoTuru::Bellekte
        };
        Self {
            kalici,
            bellek: BellekDeposu::default(),
            tur: Mutex::new(tur),
        }
    }

    /// Kayıtlı anahtar. Bellekteki (bu çalıştırmada girilen) önce gelir.
    /// Kalıcı depo okunamazsa anahtar yok sayılır; kullanıcı yeniden girer.
    pub fn oku(&self) -> Option<String> {
        if let Ok(Some(anahtar)) = self.bellek.oku() {
            return Some(anahtar);
        }
        self.kalici.as_ref().and_then(|d| d.oku().ok().flatten())
    }

    /// Anahtarı saklar; nereye saklandığını döner.
    pub fn yaz(&self, anahtar: &str) -> DepoTuru {
        let tur = match &self.kalici {
            Some(depo) if depo.yaz(anahtar).is_ok() => {
                // Önceki bir yazımdan bellekte kalan anahtar yenisini gölgelemesin.
                *self.bellek.deger.lock().unwrap_or_else(|z| z.into_inner()) = None;
                DepoTuru::Kalici
            }
            _ => {
                // BellekDeposu hata vermez.
                let _ = self.bellek.yaz(anahtar);
                DepoTuru::Bellekte
            }
        };
        *self.tur.lock().unwrap_or_else(|z| z.into_inner()) = tur;
        tur
    }

    pub fn tur(&self) -> DepoTuru {
        *self.tur.lock().unwrap_or_else(|z| z.into_inner())
    }
}
```

`crates/kalfa-uygulama/src/evren.rs`:

```rust
//! EVREN sağlayıcısı. Anahtar değişince açık oturumlar da yenisini kullanır:
//! oturumlar sağlayıcıya, anahtarı çalışırken değiştirilebilen paylaşılan bir
//! sarmalayıcı üzerinden erişir (1c tasarımı §7.2).

use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use kalfa_core::saglayici::{EvrenSaglayici, Saglayici, SaglayiciAkisi};
use kalfa_evren::tipler::{ChatRequest, TermsStatus, TermsText};
use kalfa_evren::{ApiAnahtari, EvrenError, EvrenIstemcisi};

/// EVREN'in taban adresini değiştirir (yerel denemeler). Yalnızca hata
/// ayıklama derlemesinde okunur: sürümde anahtar başka bir adrese gidemez.
pub const TABAN_DEGISKENI: &str = "KALFA_EVREN_TABAN";

/// Anahtardan sağlayıcı üretir. Uygulamada gerçek EVREN istemcisi; testlerde
/// anahtara göre davranan sahte sağlayıcı.
pub trait SaglayiciUretici: Send + Sync {
    fn uret(&self, anahtar: &str) -> Result<Arc<dyn Saglayici>, EvrenError>;
}

pub struct EvrenUretici {
    pub taban: String,
}

impl SaglayiciUretici for EvrenUretici {
    fn uret(&self, anahtar: &str) -> Result<Arc<dyn Saglayici>, EvrenError> {
        let istemci = EvrenIstemcisi::yeni(self.taban.clone(), ApiAnahtari::yeni(anahtar))?;
        Ok(Arc::new(EvrenSaglayici::yeni(istemci)))
    }
}

/// Anahtarı çalışırken değiştirilebilen sağlayıcı. Anahtar yokken her istek
/// `AnahtarGecersiz` döner.
#[derive(Default)]
pub struct DegisirSaglayici {
    ic: RwLock<Option<Arc<dyn Saglayici>>>,
}

impl DegisirSaglayici {
    pub fn ayarla(&self, saglayici: Arc<dyn Saglayici>) {
        *self.ic.write().unwrap_or_else(|z| z.into_inner()) = Some(saglayici);
    }

    fn simdiki(&self) -> Result<Arc<dyn Saglayici>, EvrenError> {
        self.ic
            .read()
            .unwrap_or_else(|z| z.into_inner())
            .clone()
            .ok_or(EvrenError::AnahtarGecersiz)
    }
}

#[async_trait]
impl Saglayici for DegisirSaglayici {
    async fn akis(&self, istek: ChatRequest) -> Result<SaglayiciAkisi, EvrenError> {
        self.simdiki()?.akis(istek).await
    }

    async fn sart_metni(&self) -> Result<TermsText, EvrenError> {
        self.simdiki()?.sart_metni().await
    }

    async fn sart_kabul(&self, surum: u32) -> Result<(), EvrenError> {
        self.simdiki()?.sart_kabul(surum).await
    }

    async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError> {
        self.simdiki()?.sart_durumu().await
    }
}
```

`crates/kalfa-uygulama/src/arayuz.rs`:

```rust
//! Uygulamanın dış dünyaya açılan uçları. `kalfa` crate'i bunları Tauri ile
//! uygular; testler sahteleriyle (`test_destek`).

use std::path::PathBuf;

use async_trait::async_trait;
use kalfa_core::aktarim::KanalMesaji;
use url::Url;

/// Yerel iletişim kutuları. Webview'daki bir betik bunları taklit edemez.
#[async_trait]
pub trait Iletisim: Send + Sync {
    /// Yerel klasör seçici; vazgeçilirse `None`.
    async fn klasor_sec(&self) -> Option<PathBuf>;
    /// Yerel onay kutusu; `tamam` düğmesi seçilirse `true`.
    async fn onayla(&self, baslik: &str, mesaj: &str, tamam: &str, vazgec: &str) -> bool;
}

/// Adresi sistem tarayıcısında açar.
pub trait Acici: Send + Sync {
    fn ac(&self, adres: &Url) -> Result<(), String>;
}

/// İki pencereyi yalnızca Rust açar (1c tasarımı §6).
pub trait Pencereler: Send + Sync {
    /// Ana pencereyi açar; açıksa öne getirir.
    fn ana_ac(&self);
    /// Şart penceresini açar; açıksa yeniden yükler ve öne getirir. Pencere
    /// açılınca durumunu `sart_penceresi_durumu` ile okur.
    fn sartlar_goster(&self);
    fn sartlar_kapat(&self);
}

/// Bir oturumun kanal mesajlarının yazıldığı yer (Tauri'de `Channel`).
pub trait KanalYazici: Send + Sync + 'static {
    fn gonder(&self, mesaj: KanalMesaji) -> Result<(), String>;
}
```

- [ ] **Step 4: Testin geçtiğini gör**

Run: `cargo test -p kalfa-uygulama --locked --test kurallar`
Expected: PASS — `test result: ok. 11 passed`.

- [ ] **Step 5: Biçim ve lint**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
Expected: çıktı yok ya da yalnızca `Finished`.

- [ ] **Step 6: Commit**

```bash
git add crates/kalfa-uygulama
git commit -m "kalfa-uygulama: komut sözleşmesi, pencere kuralları ve kimlik deposu"
git push
```

---

### Task 2: `kalfa-uygulama` — şart akışı, oturum tablosu ve olay köprüsü

`Uygulama` (paylaşılan durum), açılış ve şart penceresi akışı, oturum tablosu ve bütün oturum komutları, paketleyiciyi kanala bağlayan köprü ve testlerin sahteleri.

**Files:**
- Modify: `crates/kalfa-uygulama/src/lib.rs` (son hâli)
- Create: `crates/kalfa-uygulama/src/{uygulama,sartlar,oturumlar,kopru,test_destek}.rs`
- Test: `crates/kalfa-uygulama/tests/sart_akisi.rs`, `crates/kalfa-uygulama/tests/oturum_tablosu.rs`

**Interfaces:**
- Consumes:
  - **Görev 1'in bütün modülleri.**
  - **1c-1 şart kapısı:** `sart_kapisi::{SartKapisi, IlkDurum, KabulSonucu, GerekliSonucu, KapiGorunumu, SartKarari}`; `SartKapisi` metotları `yeni`, `durum`, `izle`, `ilk_durum`, `gerekli(istek_nesli)`, `kabul_et`, `reddet() -> bool`.
  - **1c-1 aktarım ve kilit:** `aktarim::{Paketleyici, KrediTutamaci, OlayKaynagi, AktarimAyarlari}`, `kilit::KilitYoneticisi`.
  - **1c-1 geçmiş:** `gecmis::{gorunum_uret, sayfa, durum_ozeti, oturumlari_listele}`.
  - **1c-1 oturum:** `oturum::{baslat_kimlikle, devam_et, OturumTutamaci, OlayAlici, OturumAyarlari, OturumBaglami}`, `OturumTutamaci::kapanmasini_bekle`.
  - **1c-1 kayıt, kurtarma ve sahteler:** `kayit::OturumKaydi::oku`, `kurtarma::kurtar`, `test_destek::SahteSaglayici`.
  - **kalfa-tools:** `CalismaAlani::ac`, `varsayilan_araclar`.
- Produces:
  - **Temel:** `Bilesenler { veri_dizini, kasa, uretici, iletisim, acici, pencereler, aktarim }`; `Uygulama::yeni(Bilesenler) -> Arc<Uygulama>`, `sart_kapisi()`, `acik_oturum_sayisi()`.
  - **Şart penceresi:**
    - `baslat(self: &Arc<Self>)`
    - `sart_penceresi_durumu() -> SartPenceresiDurumu`
    - `anahtari_kaydet(self: &Arc<Self>, String) -> Result<AnahtarSonucu, KomutHatasi>`
    - `sartlari_kabul_et(u32) -> Result<KabulYaniti, _>`
    - `sartlari_reddet()`
    - `sart_metnini_goster() -> Result<SartPenceresiDurumu, _>`
    - `anahtar_penceresini_ac()`, `sart_penceresini_ac()`
  - **Ana pencere:**
    - `uygulama_durumu() -> UygulamaDurumu`
    - `oturumlari_listele() -> Result<Vec<OturumSatiri>, _>`
    - `calisma_alani_sec() -> Option<CalismaAlani>`
    - `oturum_baslat(u64, String) -> Result<OturumAcilisi, _>`, `oturum_ac(OturumId) -> Result<OturumAcilisi, _>`
    - `gecmis_sayfasi(OturumId, u64) -> Result<GecmisSayfasi, _>`
    - `oturumu_dinle(OturumId, Box<dyn KanalYazici>) -> Result<(), _>`, `paketi_isledim(OturumId, u64) -> Result<(), _>`
    - `mesaj_gonder(OturumId, String)`, `turu_iptal_et(OturumId)`, `arac_onayi(OturumId, Vec<OnayGirdisi>)`, `model_degistir(OturumId, String)`
    - `izin_kipi_degistir(OturumId, Kip) -> Result<Kip, _>`
    - `oturumu_kapat(OturumId)`, `baglanti_ac(String)`
  - **Pencere yaşam döngüsü:** `ana_pencere_gitti() -> Vec<(OturumId, OturumTutamaci)>` (eşzamanlı), `kapanislari_bekle(Vec<…>)`.
  - **`test_destek` (özellik `test-destek`):**
    - Sabitler ve sahteler: `GECERSIZ_ANAHTAR`, `AGSIZ_ANAHTAR`, `SahteUretici`, `SahteAcici`, `YazilamayanDepo`, `PaylasilanDepo`.
    - `SahteIletisim` (`klasor_ver`, `onay_ver`, `sorulanlar`), `SahtePencereler` (`olaylar`, `temizle`), `SahteKanal::yeni()`.
    - `TestUygulamasi`, `test_uygulamasi(&Path, SahteSaglayici, Option<&str>)`.

**Davranış notları (testler sınar):**
- **Açılış (§7.1):**
  - Kayıtlı anahtar yoksa anahtar formu açılır.
  - Anahtar varsa şart durumu sorulur. Kabul edilmişse ana pencere açılır. Kabul gerekiyorsa kapı izleyicisi şart penceresini açar; kabul gönderilmez.
  - 401 gelirse anahtar formu açılır; EVREN'e ulaşılamazsa ana pencere bağlantısız açılır.
- **Anahtar kaydı (§7.2):** girilen anahtar önce şart durumu sorgusuyla denenir; 401 ya da ağ hatasında saklanmaz.
- **Kapı izleyicisi:** şart penceresi komutu sürerken (`pencere_komutu > 0`) pencereyi yeniden yüklemez; komut yeni durumu kendisi döner.
- **Kabul:**
  - EVREN'e bir kez gider. Çift tıklamanın ikincisi yeniden göndermez, `Kabul` döner.
  - Retten sonra gelen eski kabul tıklaması `Kilitli` döner ve kabul göndermez.
  - Kapı boştayken ve son karar kabul değilken `GecersizGirdi` döner.
- **Ret:** bekleyen metin yokken verilen ret kilitlemez.
- **Oturumu açma (§5.1, §10):**
  - Aynı oturum ikinci kez açılmaz; açık olanın geçmişi döner. Eşzamanlı iki açma aynı oturumu verir.
  - Kaydı olmayan oturum `OturumYok` döner.
  - Çalışma alanı yoksa ya da kayıtta yazmıyorsa oturum çekirdekte başlatılmaz, `salt_okunur` dolu gelir.
- **Ana pencere yeniden yüklenince (§5.6):** oturumlar kapanır, kayıt kilidi bırakılır.

- [ ] **Step 1: Başarısız testleri yaz**

`crates/kalfa-uygulama/tests/sart_akisi.rs`:

```rust
//! Açılış ve şart penceresi akışı (1c tasarımı §7). Hiçbir akışta şart
//! kendiliğinden kabul edilmez: sahte sağlayıcının kabul listesi yalnızca
//! `sartlari_kabul_et` çağrılınca dolar.

use std::sync::Arc;
use std::time::Duration;

use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::SahteSaglayici;
use kalfa_uygulama::anahtar::{AnahtarDeposu, AnahtarKasasi};
use kalfa_uygulama::sozlesme::{
    AnahtarNedeni, AnahtarSonucu, BaglantiDurumu, DepoTuru, KabulYaniti, SartPenceresiDurumu,
};
use kalfa_uygulama::test_destek::{
    AGSIZ_ANAHTAR, GECERSIZ_ANAHTAR, SahteAcici, SahteIletisim, SahtePencereler, SahteUretici,
    TestUygulamasi, YazilamayanDepo, test_uygulamasi,
};
use kalfa_uygulama::{Bilesenler, Uygulama};

async fn bekle(mut kosul: impl FnMut() -> bool) {
    for _ in 0..200 {
        if kosul() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("koşul 2 saniyede gerçekleşmedi");
}

fn gerekli_surum(durum: &SartPenceresiDurumu) -> Option<u32> {
    match durum {
        SartPenceresiDurumu::SartGerekli { metin } => Some(metin.surum),
        _ => None,
    }
}

async fn acilmis(
    dizin: &std::path::Path,
    anahtar: Option<&str>,
    kabul_edilmis: bool,
) -> TestUygulamasi {
    let saglayici = SahteSaglayici::yeni(Vec::new());
    if kabul_edilmis {
        saglayici.sart_kabul(1).await.unwrap();
    }
    let t = test_uygulamasi(dizin, saglayici, anahtar);
    t.uygulama.baslat().await;
    t
}

#[tokio::test]
async fn anahtar_yoksa_anahtar_formu_acilir() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), None, false).await;
    assert_eq!(
        t.uygulama.sart_penceresi_durumu(),
        SartPenceresiDurumu::AnahtarGerekli {
            neden: AnahtarNedeni::Yok
        }
    );
    assert_eq!(t.pencereler.olaylar(), ["sartlar_goster"]);
}

#[tokio::test]
async fn sartlar_kabul_edilmisse_ana_pencere_acilir() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), true).await;
    assert_eq!(t.pencereler.olaylar(), ["ana_ac"]);
    assert_eq!(t.uygulama.uygulama_durumu().baglanti, BaglantiDurumu::Var);
    assert_eq!(
        t.saglayici.kabul_edilen_surumler(),
        [1],
        "yalnızca kurulumdaki kabul"
    );
}

#[tokio::test]
async fn kabul_gerekiyorsa_sart_penceresi_acilir_kabul_gonderilmez() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), false).await;
    bekle(|| t.pencereler.olaylar().contains(&"sartlar_goster")).await;
    assert_eq!(gerekli_surum(&t.uygulama.sart_penceresi_durumu()), Some(1));
    assert!(!t.pencereler.olaylar().contains(&"ana_ac"));
    assert!(t.saglayici.kabul_edilen_surumler().is_empty());
}

#[tokio::test]
async fn kayitli_anahtar_gecersizse_anahtar_formu() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some(GECERSIZ_ANAHTAR), false).await;
    assert_eq!(
        t.uygulama.sart_penceresi_durumu(),
        SartPenceresiDurumu::AnahtarGerekli {
            neden: AnahtarNedeni::KayitliGecersiz
        }
    );
    assert_eq!(t.pencereler.olaylar(), ["sartlar_goster"]);
}

#[tokio::test]
async fn evrene_ulasilamazsa_ana_pencere_baglantisiz_acilir() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some(AGSIZ_ANAHTAR), false).await;
    assert_eq!(t.pencereler.olaylar(), ["ana_ac"]);
    assert_eq!(t.uygulama.uygulama_durumu().baglanti, BaglantiDurumu::Yok);
}

#[tokio::test]
async fn gecersiz_ya_da_ulasilamayan_anahtar_saklanmaz() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), None, false).await;
    assert_eq!(
        t.uygulama
            .anahtari_kaydet(GECERSIZ_ANAHTAR.into())
            .await
            .unwrap(),
        AnahtarSonucu::Gecersiz
    );
    assert_eq!(
        t.uygulama
            .anahtari_kaydet(AGSIZ_ANAHTAR.into())
            .await
            .unwrap(),
        AnahtarSonucu::Ulasilamadi
    );
    assert_eq!(
        t.uygulama.anahtari_kaydet("   ".into()).await.unwrap(),
        AnahtarSonucu::Gecersiz
    );
    assert_eq!(t.depo.oku().unwrap(), None);
}

#[tokio::test]
async fn gecerli_anahtar_saklanir_ve_sart_metni_doner() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), None, false).await;
    let sonuc = t
        .uygulama
        .anahtari_kaydet(" evren_llm_a ".into())
        .await
        .unwrap();
    let AnahtarSonucu::Kaydedildi { depo, sonraki } = sonuc else {
        panic!("anahtar kaydedilmeliydi: {sonuc:?}");
    };
    assert_eq!(depo, DepoTuru::Kalici);
    assert_eq!(gerekli_surum(&sonraki), Some(1));
    assert_eq!(t.depo.oku().unwrap().as_deref(), Some("evren_llm_a"));
    tokio::time::sleep(Duration::from_millis(50)).await;
    // Pencere komutu sürerken kapı izleyicisi pencereyi yeniden yüklemez.
    assert_eq!(t.pencereler.olaylar(), ["sartlar_goster"]);
    assert!(t.saglayici.kabul_edilen_surumler().is_empty());
}

#[tokio::test]
async fn kabul_edilmis_hesabin_anahtari_ana_pencereyi_acar() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), None, false).await;
    t.saglayici.sart_kabul(1).await.unwrap();
    t.pencereler.temizle();
    let sonuc = t
        .uygulama
        .anahtari_kaydet("evren_llm_a".into())
        .await
        .unwrap();
    assert!(matches!(
        sonuc,
        AnahtarSonucu::Kaydedildi {
            sonraki: SartPenceresiDurumu::Tamam,
            ..
        }
    ));
    assert_eq!(t.pencereler.olaylar(), ["ana_ac", "sartlar_kapat"]);
}

#[tokio::test]
async fn kimlik_deposu_yazilamazsa_anahtar_bellekte_tutulur() {
    let d = tempfile::tempdir().unwrap();
    let saglayici = SahteSaglayici::yeni(Vec::new());
    let uygulama = Uygulama::yeni(Bilesenler {
        veri_dizini: d.path().to_path_buf(),
        kasa: AnahtarKasasi::yeni(Some(Box::new(YazilamayanDepo))),
        uretici: Arc::new(SahteUretici { saglayici }),
        iletisim: Arc::new(SahteIletisim::default()),
        acici: Arc::new(SahteAcici::default()),
        pencereler: Arc::new(SahtePencereler::default()),
        aktarim: Default::default(),
    });
    uygulama.baslat().await;
    let sonuc = uygulama
        .anahtari_kaydet("evren_llm_a".into())
        .await
        .unwrap();
    assert!(matches!(
        sonuc,
        AnahtarSonucu::Kaydedildi {
            depo: DepoTuru::Bellekte,
            ..
        }
    ));
    assert_eq!(
        uygulama.uygulama_durumu().anahtar_deposu,
        DepoTuru::Bellekte
    );
}

#[tokio::test]
async fn kabul_bir_kez_gider_ana_pencere_acilir_sart_penceresi_kapanir() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), false).await;
    bekle(|| t.pencereler.olaylar().contains(&"sartlar_goster")).await;
    assert_eq!(
        t.uygulama.sartlari_kabul_et(1).await.unwrap(),
        KabulYaniti::Kabul
    );
    assert_eq!(t.saglayici.kabul_edilen_surumler(), [1]);
    assert_eq!(
        t.uygulama.sart_penceresi_durumu(),
        SartPenceresiDurumu::Tamam
    );
    let olaylar = t.pencereler.olaylar();
    assert_eq!(&olaylar[olaylar.len() - 2..], ["ana_ac", "sartlar_kapat"]);
}

#[tokio::test]
async fn ikinci_tiklama_kabulu_yinelemez_sonraki_ret_kilitlemez() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), false).await;
    bekle(|| t.pencereler.olaylar().contains(&"sartlar_goster")).await;
    let (a, b) = tokio::join!(
        t.uygulama.sartlari_kabul_et(1),
        t.uygulama.sartlari_kabul_et(1)
    );
    assert_eq!(a.unwrap(), KabulYaniti::Kabul);
    assert_eq!(b.unwrap(), KabulYaniti::Kabul);
    assert_eq!(t.saglayici.kabul_edilen_surumler(), [1]);
    // Kabulden sonra gelen eski "Kabul etmiyorum" tıklaması bir şey değiştirmez.
    t.uygulama.sartlari_reddet();
    assert_eq!(
        t.uygulama.sart_penceresi_durumu(),
        SartPenceresiDurumu::Tamam
    );
}

#[tokio::test]
async fn retten_sonra_eski_kabul_tiklamasi_kabul_gondermez() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), false).await;
    bekle(|| t.pencereler.olaylar().contains(&"sartlar_goster")).await;
    t.uygulama.sartlari_reddet();
    assert_eq!(
        t.uygulama.sartlari_kabul_et(1).await.unwrap(),
        KabulYaniti::Kilitli
    );
    assert!(t.saglayici.kabul_edilen_surumler().is_empty());
    assert_eq!(
        t.uygulama.sart_penceresi_durumu(),
        SartPenceresiDurumu::Kilitli
    );
}

#[tokio::test]
async fn gosterilenden_farkli_surum_kabul_edilmez() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), false).await;
    bekle(|| t.pencereler.olaylar().contains(&"sartlar_goster")).await;
    let yanit = t.uygulama.sartlari_kabul_et(2).await.unwrap();
    assert!(matches!(yanit, KabulYaniti::YeniMetin { metin } if metin.surum == 1));
    assert!(t.saglayici.kabul_edilen_surumler().is_empty());
}

#[tokio::test]
async fn okurken_surum_degisirse_yeni_metin_gelir() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), false).await;
    bekle(|| t.pencereler.olaylar().contains(&"sartlar_goster")).await;
    t.saglayici.sart_surumunu_degistir(2);
    let yanit = t.uygulama.sartlari_kabul_et(1).await.unwrap();
    assert!(matches!(yanit, KabulYaniti::YeniMetin { metin } if metin.surum == 2));
    assert!(t.saglayici.kabul_edilen_surumler().is_empty());
    assert_eq!(gerekli_surum(&t.uygulama.sart_penceresi_durumu()), Some(2));
}

#[tokio::test]
async fn ret_kilitli_gorunume_gecer_metin_yeniden_gosterilebilir() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), false).await;
    bekle(|| t.pencereler.olaylar().contains(&"sartlar_goster")).await;
    t.uygulama.sartlari_reddet();
    assert_eq!(
        t.uygulama.sart_penceresi_durumu(),
        SartPenceresiDurumu::Kilitli
    );
    let durum = t.uygulama.sart_metnini_goster().await.unwrap();
    assert_eq!(gerekli_surum(&durum), Some(1));
    assert_eq!(gerekli_surum(&t.uygulama.sart_penceresi_durumu()), Some(1));
    assert!(t.saglayici.kabul_edilen_surumler().is_empty());
}

#[tokio::test]
async fn anahtar_penceresi_sart_beklerken_sart_metnini_gosterir() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), false).await;
    bekle(|| t.pencereler.olaylar().contains(&"sartlar_goster")).await;
    t.uygulama.anahtar_penceresini_ac();
    assert_eq!(gerekli_surum(&t.uygulama.sart_penceresi_durumu()), Some(1));

    let d2 = tempfile::tempdir().unwrap();
    let t2 = acilmis(d2.path(), Some("evren_llm_a"), true).await;
    t2.uygulama.anahtar_penceresini_ac();
    assert_eq!(
        t2.uygulama.sart_penceresi_durumu(),
        SartPenceresiDurumu::AnahtarGerekli {
            neden: AnahtarNedeni::Degistir
        }
    );
    assert_eq!(t2.pencereler.olaylar(), ["ana_ac", "sartlar_goster"]);
}

#[tokio::test]
async fn sart_penceresi_yalnizca_bekleyen_ya_da_kilitli_durumda_yeniden_acilir() {
    let d = tempfile::tempdir().unwrap();
    let t = acilmis(d.path(), Some("evren_llm_a"), true).await;
    t.uygulama.sart_penceresini_ac();
    assert_eq!(t.pencereler.olaylar(), ["ana_ac"]);
    // Bekleyen metin yokken verilen ret kilitlemez.
    t.uygulama.sartlari_reddet();
    t.uygulama.sart_penceresini_ac();
    assert_eq!(t.pencereler.olaylar(), ["ana_ac"]);

    let d2 = tempfile::tempdir().unwrap();
    let t2 = acilmis(d2.path(), Some("evren_llm_a"), false).await;
    bekle(|| t2.pencereler.olaylar().contains(&"sartlar_goster")).await;
    t2.uygulama.sartlari_reddet();
    t2.pencereler.temizle();
    t2.uygulama.sart_penceresini_ac();
    assert_eq!(t2.pencereler.olaylar(), ["sartlar_goster"]);
}
```

`crates/kalfa-uygulama/tests/oturum_tablosu.rs`:

```rust
//! Oturum tablosu (1c tasarımı §5): başlatma, köprü, kayıttan açma, kapatma,
//! kopma, izin kipi ve bağlantı açma.

use std::path::{Path, PathBuf};
use std::time::Duration;

use kalfa_core::aktarim::KanalMesaji;
use kalfa_core::gecmis::GorunumOgesi;
use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::{OturumId, TurId};
use kalfa_core::protokol::{BitisNedeni, Olay, SEMA_SURUMU, Zarf};
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::{SahteSaglayici, SahteYanit};
use kalfa_evren::AkisOlayi;
use kalfa_evren::tipler::ChatMessage;
use kalfa_uygulama::dizin::{kayit_yolu, oturum_dizini};
use kalfa_uygulama::hata::HataKodu;
use kalfa_uygulama::sozlesme::SaltOkunurNedeni;
use kalfa_uygulama::test_destek::{SahteKanal, TestUygulamasi, test_uygulamasi};
use tempfile::TempDir;
use tokio::sync::mpsc::UnboundedReceiver;

struct Ortam {
    veri: TempDir,
    alan: TempDir,
    t: TestUygulamasi,
}

/// Anahtarı kayıtlı ve şartları kabul edilmiş, açılmış uygulama.
async fn ortam(senaryo: Vec<SahteYanit>) -> Ortam {
    let veri = tempfile::tempdir().unwrap();
    let alan = tempfile::tempdir().unwrap();
    let saglayici = SahteSaglayici::yeni(senaryo);
    saglayici.sart_kabul(1).await.unwrap();
    let t = test_uygulamasi(veri.path(), saglayici, Some("evren_llm_a"));
    t.uygulama.baslat().await;
    t.iletisim.klasor_ver(alan.path());
    Ortam { veri, alan, t }
}

impl Ortam {
    async fn yeni_oturum(&self) -> OturumId {
        let alan = self.t.uygulama.calisma_alani_sec().await.unwrap();
        self.t
            .uygulama
            .oturum_baslat(alan.kimlik, "glm-5.3".into())
            .await
            .unwrap()
            .oturum
    }

    fn dinle(&self, oturum: OturumId) -> UnboundedReceiver<KanalMesaji> {
        let (kanal, rx) = SahteKanal::yeni();
        self.t.uygulama.oturumu_dinle(oturum, kanal).unwrap();
        rx
    }

    fn kayit(&self, oturum: OturumId) -> PathBuf {
        kayit_yolu(&oturum_dizini(self.veri.path()), oturum)
    }
}

/// Paketleri "işledim" diyerek okur; koşula uyan olay gelince durur.
async fn olay_bekle(
    t: &TestUygulamasi,
    oturum: OturumId,
    rx: &mut UnboundedReceiver<KanalMesaji>,
    kosul: impl Fn(&Olay) -> bool,
) -> Vec<Olay> {
    let mut gorulen = Vec::new();
    loop {
        let mesaj = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("olay 5 saniyede gelmedi")
            .expect("kanal kapandı");
        let KanalMesaji::Olaylar { no, zarflar } = mesaj else {
            panic!("kanal beklenmeden kapandı");
        };
        t.uygulama.paketi_isledim(oturum, no).unwrap();
        for z in zarflar {
            let bulundu = kosul(&z.yuk);
            gorulen.push(z.yuk);
            if bulundu {
                return gorulen;
            }
        }
    }
}

fn tamamlandi(o: &Olay) -> bool {
    matches!(o, Olay::Tamamlandi { .. })
}

#[tokio::test]
async fn calisma_alani_secilmeden_ya_da_bilinmeyen_modelle_baslamaz() {
    let o = ortam(Vec::new()).await;
    let hata =
        o.t.uygulama
            .oturum_baslat(99, "glm-5.3".into())
            .await
            .unwrap_err();
    assert_eq!(hata.kod, HataKodu::GecersizGirdi);
    let alan = o.t.uygulama.calisma_alani_sec().await.unwrap();
    let hata =
        o.t.uygulama
            .oturum_baslat(alan.kimlik, "gpt-5".into())
            .await
            .unwrap_err();
    assert_eq!(hata.kod, HataKodu::GecersizGirdi);
    assert_eq!(o.t.uygulama.acik_oturum_sayisi(), 0);
}

#[tokio::test]
async fn oturum_akar_kapatilir_ve_kayittan_acilir() {
    let o = ortam(vec![SahteYanit::metin("merhaba")]).await;
    let oturum = o.yeni_oturum().await;
    assert!(
        o.kayit(oturum).exists(),
        "kayıt dosyasının adı oturum kimliği"
    );
    let mut rx = o.dinle(oturum);
    o.t.uygulama
        .mesaj_gonder(oturum, "selam".into())
        .await
        .unwrap();
    let olaylar = olay_bekle(&o.t, oturum, &mut rx, tamamlandi).await;
    assert!(
        olaylar
            .iter()
            .any(|x| matches!(x, Olay::TokenParcasi { metin } if metin.contains("merhaba")))
    );
    assert_eq!(o.t.uygulama.uygulama_durumu().son_oturum, Some(oturum));

    o.t.uygulama.oturumu_kapat(oturum).await.unwrap();
    assert_eq!(o.t.uygulama.acik_oturum_sayisi(), 0);

    let acilis = o.t.uygulama.oturum_ac(oturum).await.unwrap();
    assert_eq!(acilis.salt_okunur, None);
    assert_eq!(
        acilis.calisma_alani,
        Some(o.alan.path().display().to_string())
    );
    assert!(
        acilis
            .gecmis
            .ogeler
            .iter()
            .any(|x| matches!(x, GorunumOgesi::KullaniciMesaji { metin, .. } if metin == "selam"))
    );
    assert!(
        acilis
            .gecmis
            .ogeler
            .iter()
            .any(|x| matches!(x, GorunumOgesi::Yanit { metin: Some(m), .. } if m == "merhaba"))
    );
    assert_eq!(acilis.ozet.model.as_deref(), Some("glm-5.3"));
    assert_eq!(o.t.uygulama.acik_oturum_sayisi(), 1);
}

#[tokio::test]
async fn acik_oturum_ikinci_kez_acilmaz_ve_ikinci_kez_dinlenmez() {
    let o = ortam(Vec::new()).await;
    let oturum = o.yeni_oturum().await;
    let _rx = o.dinle(oturum);
    o.t.uygulama.oturum_ac(oturum).await.unwrap();
    assert_eq!(o.t.uygulama.acik_oturum_sayisi(), 1);
    let (kanal, _rx2) = SahteKanal::yeni();
    let hata = o.t.uygulama.oturumu_dinle(oturum, kanal).unwrap_err();
    assert_eq!(hata.kod, HataKodu::GecersizGirdi);
}

#[tokio::test]
async fn kayit_baska_yerde_aciksa_mesgul_hatasi() {
    let o = ortam(Vec::new()).await;
    let oturum = o.yeni_oturum().await;
    o.t.uygulama.oturumu_kapat(oturum).await.unwrap();
    let _kilit = OturumKaydi::ac(&o.kayit(oturum)).unwrap();
    let hata = o.t.uygulama.oturum_ac(oturum).await.unwrap_err();
    assert_eq!(hata.kod, HataKodu::KayitMesgul);
    assert_eq!(hata.mesaj, "Bu oturum başka bir Kalfa sürecinde açık.");
}

#[tokio::test]
async fn eszamanli_iki_acma_ayni_oturumu_verir() {
    let o = ortam(Vec::new()).await;
    let oturum = o.yeni_oturum().await;
    o.t.uygulama.oturumu_kapat(oturum).await.unwrap();
    let (a, b) = tokio::join!(
        o.t.uygulama.oturum_ac(oturum),
        o.t.uygulama.oturum_ac(oturum)
    );
    assert_eq!(a.unwrap().oturum, oturum);
    assert_eq!(b.unwrap().oturum, oturum);
    assert_eq!(o.t.uygulama.acik_oturum_sayisi(), 1);
}

#[tokio::test]
async fn kaydi_olmayan_oturum_acilmaz() {
    let o = ortam(Vec::new()).await;
    let yok = OturumId::yeni();
    let hata = o.t.uygulama.oturum_ac(yok).await.unwrap_err();
    assert_eq!(hata.kod, HataKodu::OturumYok);
    assert_eq!(hata.mesaj, "Bu oturumun kaydı bulunamadı.");
    let hata = o.t.uygulama.gecmis_sayfasi(yok, 10).await.unwrap_err();
    assert_eq!(hata.kod, HataKodu::OturumYok);
}

#[tokio::test]
async fn calisma_alani_yoksa_yalnizca_gecmis_gosterilir() {
    let mut o = ortam(Vec::new()).await;
    let oturum = o.yeni_oturum().await;
    o.t.uygulama.oturumu_kapat(oturum).await.unwrap();
    let alan_yolu = o.alan.path().display().to_string();
    std::mem::replace(&mut o.alan, tempfile::tempdir().unwrap())
        .close()
        .unwrap();
    let acilis = o.t.uygulama.oturum_ac(oturum).await.unwrap();
    assert_eq!(
        acilis.salt_okunur,
        Some(SaltOkunurNedeni::CalismaAlaniYok { yol: alan_yolu })
    );
    assert_eq!(o.t.uygulama.acik_oturum_sayisi(), 0);
    let hata =
        o.t.uygulama
            .mesaj_gonder(oturum, "x".into())
            .await
            .unwrap_err();
    assert_eq!(hata.kod, HataKodu::OturumYok);
}

/// Çalışma alanını yazmayan (1c öncesi) kayıt: 21 tur, her birinde bir
/// kullanıcı mesajı ve bir yanıt.
fn eski_kayit(yol: &Path, oturum: OturumId) {
    let mut kayit = OturumKaydi::olustur(yol).unwrap();
    let mut sira = 0;
    let mut yaz = |tur: Option<TurId>, yuk: KayitOgesi| {
        sira += 1;
        kayit
            .yaz(&Zarf {
                sema: SEMA_SURUMU,
                oturum,
                tur,
                sira,
                yuk,
            })
            .unwrap();
    };
    yaz(
        None,
        KayitOgesi::OturumAcildi {
            model: "glm-5.3".into(),
            kip: Kip::Siki,
            calisma_alani: None,
        },
    );
    for i in 0..21 {
        let tur = Some(TurId::yeni());
        yaz(
            tur,
            KayitOgesi::Mesaj {
                mesaj: ChatMessage::user(format!("soru {i}")),
            },
        );
        yaz(
            tur,
            KayitOgesi::Mesaj {
                mesaj: ChatMessage::assistant(Some(format!("yanıt {i}")), Vec::new()),
            },
        );
        yaz(
            tur,
            KayitOgesi::TurBitti {
                neden: BitisNedeni::Normal,
            },
        );
    }
}

#[tokio::test]
async fn eski_kayit_salt_okunur_acilir_ve_sayfa_sayfa_okunur() {
    let o = ortam(Vec::new()).await;
    let oturum = OturumId::yeni();
    std::fs::create_dir_all(oturum_dizini(o.veri.path())).unwrap();
    eski_kayit(&o.kayit(oturum), oturum);

    let acilis = o.t.uygulama.oturum_ac(oturum).await.unwrap();
    assert_eq!(
        acilis.salt_okunur,
        Some(SaltOkunurNedeni::CalismaAlaniKayitsiz)
    );
    assert!(acilis.gecmis.daha_eski_var);
    let ilk_sorular: Vec<&str> = acilis
        .gecmis
        .ogeler
        .iter()
        .filter_map(|x| match x {
            GorunumOgesi::KullaniciMesaji { metin, .. } => Some(metin.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(ilk_sorular.len(), 20);
    assert_eq!(ilk_sorular[0], "soru 1");

    let en_eski = acilis.gecmis.ogeler[0].sira();
    let sayfa = o.t.uygulama.gecmis_sayfasi(oturum, en_eski).await.unwrap();
    assert!(!sayfa.daha_eski_var);
    assert!(matches!(
        &sayfa.ogeler[0],
        GorunumOgesi::KullaniciMesaji { metin, .. } if metin == "soru 0"
    ));
}

#[tokio::test]
async fn oturum_listesi_en_yeni_once() {
    let o = ortam(vec![SahteYanit::metin("a"), SahteYanit::metin("b")]).await;
    for mesaj in ["ilk oturum", "ikinci oturum"] {
        let oturum = o.yeni_oturum().await;
        let mut rx = o.dinle(oturum);
        o.t.uygulama
            .mesaj_gonder(oturum, mesaj.into())
            .await
            .unwrap();
        olay_bekle(&o.t, oturum, &mut rx, tamamlandi).await;
        o.t.uygulama.oturumu_kapat(oturum).await.unwrap();
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    let liste = o.t.uygulama.oturumlari_listele().await.unwrap();
    let basliklar: Vec<_> = liste.iter().map(|s| s.baslik.as_deref()).collect();
    assert_eq!(basliklar, [Some("ikinci oturum"), Some("ilk oturum")]);
    assert!(
        liste
            .iter()
            .all(|s| s.dosya.ends_with(".jsonl") && s.oturum.is_some())
    );
}

#[tokio::test]
async fn izin_kipi_yukseltmesi_yerel_onay_ister() {
    let o = ortam(Vec::new()).await;
    let oturum = o.yeni_oturum().await;
    let u = &o.t.uygulama;
    o.t.iletisim.onay_ver(false);
    assert_eq!(
        u.izin_kipi_degistir(oturum, Kip::Proje).await.unwrap(),
        Kip::Siki
    );
    assert_eq!(o.t.iletisim.sorulanlar().len(), 1);
    assert!(o.t.iletisim.sorulanlar()[0].contains("Sıkı yerine Proje"));
    o.t.iletisim.onay_ver(true);
    assert_eq!(
        u.izin_kipi_degistir(oturum, Kip::Serbest).await.unwrap(),
        Kip::Serbest
    );
    assert_eq!(o.t.iletisim.sorulanlar().len(), 2);
    assert_eq!(
        u.izin_kipi_degistir(oturum, Kip::Inceleme).await.unwrap(),
        Kip::Inceleme
    );
    assert_eq!(
        o.t.iletisim.sorulanlar().len(),
        2,
        "kip düşürmek onay istemez"
    );
}

#[tokio::test]
async fn baglanti_yalnizca_http_https_ve_onayla_acilir() {
    let o = ortam(Vec::new()).await;
    let u = &o.t.uygulama;
    for kotu in ["javascript:alert(1)", "file:///etc/passwd", "ornek.com"] {
        let hata = u.baglanti_ac(kotu.into()).await.unwrap_err();
        assert_eq!(hata.kod, HataKodu::GecersizGirdi, "{kotu}");
    }
    assert!(o.t.iletisim.sorulanlar().is_empty());
    o.t.iletisim.onay_ver(false);
    u.baglanti_ac("https://ornek.com".into()).await.unwrap();
    assert!(o.t.acici.acilanlar().is_empty());
    assert!(o.t.iletisim.sorulanlar()[0].contains("https://ornek.com/"));
    o.t.iletisim.onay_ver(true);
    u.baglanti_ac("https://ornek.com".into()).await.unwrap();
    assert_eq!(o.t.acici.acilanlar(), ["https://ornek.com/"]);
}

#[tokio::test]
async fn ana_pencere_gidince_tur_iptal_edilir_kilit_birakilir() {
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

    let kapananlar = o.t.uygulama.ana_pencere_gitti();
    assert_eq!(kapananlar.len(), 1);
    assert_eq!(o.t.uygulama.acik_oturum_sayisi(), 0);
    let u = o.t.uygulama.clone();
    tokio::spawn(async move { u.kapanislari_bekle(kapananlar).await });

    // Yeniden yüklenen sayfa aynı oturumu açar: önce kapanış beklenir.
    let acilis = o.t.uygulama.oturum_ac(oturum).await.unwrap();
    assert!(acilis.gecmis.ogeler.iter().any(|x| matches!(
        x,
        GorunumOgesi::TurSonu {
            neden: BitisNedeni::Iptal,
            ..
        }
    )));
}

#[tokio::test]
async fn turu_iptal_etmek_tamamlandi_iptal_uretir() {
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
    o.t.uygulama.turu_iptal_et(oturum).unwrap();
    let olaylar = olay_bekle(&o.t, oturum, &mut rx, tamamlandi).await;
    assert!(matches!(
        olaylar.last(),
        Some(Olay::Tamamlandi {
            neden: BitisNedeni::Iptal
        })
    ));
}
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa-uygulama --locked --test sart_akisi --test oturum_tablosu`
Expected: FAIL. Derleme hatası: `could not find test_destek in kalfa_uygulama`, `no Uygulama in the root` ve benzerleri.

- [ ] **Step 3: Modülleri yaz**

`crates/kalfa-uygulama/src/lib.rs` (son hâli):

```rust
//! Kalfa masaüstü uygulamasının Tauri'siz mantığı (1c tasarımı §3.1, §3.2).
//!
//! `kalfa` crate'i bunu Tauri komutlarına ve pencerelerine bağlar. Pencereye,
//! yerel iletişim kutusuna, sistem tarayıcısına ve kanala trait'lerle erişilir
//! (`arayuz`); testler sahteleriyle koşar ve WebKitGTK istemez.

pub mod anahtar;
pub mod arayuz;
pub mod dizin;
pub mod evren;
pub mod guvenlik;
pub mod hata;
pub mod modeller;
pub mod sozlesme;

mod kopru;
mod oturumlar;
mod sartlar;
mod uygulama;

#[cfg(feature = "test-destek")]
pub mod test_destek;

pub use uygulama::{Bilesenler, Uygulama};
```

`crates/kalfa-uygulama/src/uygulama.rs`:

```rust
//! Uygulamanın paylaşılan durumu. Komutlar `Uygulama`'nın yöntemleridir;
//! `kalfa` crate'i her komutu ince bir Tauri sarmalayıcısıyla bunlara bağlar.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use kalfa_core::aktarim::AktarimAyarlari;
use kalfa_core::kilit::KilitYoneticisi;
use kalfa_core::kimlik::OturumId;
use kalfa_core::oturum::OturumTutamaci;
use kalfa_core::sart_kapisi::SartKapisi;

use crate::anahtar::AnahtarKasasi;
use crate::arayuz::{Acici, Iletisim, Pencereler};
use crate::dizin;
use crate::evren::{DegisirSaglayici, SaglayiciUretici};
use crate::oturumlar::AcikOturum;
use crate::sozlesme::{AnahtarNedeni, BaglantiDurumu, SartPenceresiDurumu};

/// Uygulamayı kurmak için gerekenler; Tauri katmanı ya da test verir.
pub struct Bilesenler {
    pub veri_dizini: PathBuf,
    pub kasa: AnahtarKasasi,
    pub uretici: Arc<dyn SaglayiciUretici>,
    pub iletisim: Arc<dyn Iletisim>,
    pub acici: Arc<dyn Acici>,
    pub pencereler: Arc<dyn Pencereler>,
    pub aktarim: AktarimAyarlari,
}

pub struct Uygulama {
    pub(crate) oturum_dizini: PathBuf,
    pub(crate) kasa: AnahtarKasasi,
    pub(crate) uretici: Arc<dyn SaglayiciUretici>,
    pub(crate) evren: Arc<DegisirSaglayici>,
    pub(crate) kapi: Arc<SartKapisi>,
    pub(crate) kilitler: KilitYoneticisi,
    pub(crate) iletisim: Arc<dyn Iletisim>,
    pub(crate) acici: Arc<dyn Acici>,
    pub(crate) pencereler: Arc<dyn Pencereler>,
    pub(crate) aktarim: AktarimAyarlari,
    ic: Mutex<IcDurum>,
}

/// Kilitle korunan durum. Kilit hiçbir `await` boyunca tutulmaz.
pub(crate) struct IcDurum {
    pub baglanti: BaglantiDurumu,
    pub son_oturum: Option<OturumId>,
    pub oturumlar: HashMap<OturumId, AcikOturum>,
    /// Kapatılmış ama görevi henüz bitmemiş oturumlar; yeniden açılmadan önce
    /// beklenir (kayıt kilidi bırakılsın diye).
    pub kapaniyor: HashMap<OturumId, OturumTutamaci>,
    pub calisma_alanlari: HashMap<u64, PathBuf>,
    pub son_alan: u64,
    pub sart_penceresi: SartPenceresiDurumu,
    /// Süren şart penceresi komutu sayısı. Bu sırada kapı izleyicisi pencereyi
    /// yeniden yüklemez; komut yeni durumu kendisi döner.
    pub pencere_komutu: u32,
}

impl Uygulama {
    pub fn yeni(b: Bilesenler) -> Arc<Self> {
        let evren = Arc::new(DegisirSaglayici::default());
        let kapi = Arc::new(SartKapisi::yeni(evren.clone()));
        Arc::new(Self {
            oturum_dizini: dizin::oturum_dizini(&b.veri_dizini),
            kasa: b.kasa,
            uretici: b.uretici,
            evren,
            kapi,
            kilitler: KilitYoneticisi::yeni(),
            iletisim: b.iletisim,
            acici: b.acici,
            pencereler: b.pencereler,
            aktarim: b.aktarim,
            ic: Mutex::new(IcDurum {
                baglanti: BaglantiDurumu::Var,
                son_oturum: None,
                oturumlar: HashMap::new(),
                kapaniyor: HashMap::new(),
                calisma_alanlari: HashMap::new(),
                son_alan: 0,
                sart_penceresi: SartPenceresiDurumu::AnahtarGerekli {
                    neden: AnahtarNedeni::Yok,
                },
                pencere_komutu: 0,
            }),
        })
    }

    pub(crate) fn ic(&self) -> MutexGuard<'_, IcDurum> {
        self.ic.lock().unwrap_or_else(|z| z.into_inner())
    }

    /// Uygulama genelindeki tek şart kapısı.
    pub fn sart_kapisi(&self) -> &Arc<SartKapisi> {
        &self.kapi
    }

    /// Açık oturum sayısı (testler ve tanılama için).
    pub fn acik_oturum_sayisi(&self) -> usize {
        self.ic().oturumlar.len()
    }
}
```

`crates/kalfa-uygulama/src/sartlar.rs`:

```rust
//! Açılış ve şart penceresi akışı (1c tasarımı §7). Kalfa şartları kullanıcı
//! adına asla kabul etmez: kabul isteği yalnızca `sartlari_kabul_et`'ten,
//! yani şart penceresindeki "Kabul ediyorum" düğmesinden gider (üst belge
//! §11.8).

use std::sync::Arc;

use kalfa_core::sart_kapisi::{
    GerekliSonucu, IlkDurum, KabulSonucu, KapiGorunumu, SartKarari, SartMetni,
};
use kalfa_evren::EvrenError;

use crate::Uygulama;
use crate::hata::KomutHatasi;
use crate::sozlesme::{
    AnahtarNedeni, AnahtarSonucu, BaglantiDurumu, KabulYaniti, SartPenceresiDurumu,
};

/// Şart penceresi komutu sürerken kapı izleyicisi pencereyi yeniden yüklemez;
/// komut yeni durumu kendisi döner.
struct PencereKomutu<'a>(&'a Uygulama);

impl<'a> PencereKomutu<'a> {
    fn yeni(uygulama: &'a Uygulama) -> Self {
        uygulama.ic().pencere_komutu += 1;
        Self(uygulama)
    }
}

impl Drop for PencereKomutu<'_> {
    fn drop(&mut self) {
        let mut ic = self.0.ic();
        ic.pencere_komutu = ic.pencere_komutu.saturating_sub(1);
    }
}

impl Uygulama {
    /// Açılış (1c tasarımı §7.1): kayıtlı anahtar yoksa anahtar formu; varsa
    /// şart durumu sorulur. Kabul gerekiyorsa kapı metni göstermeye geçer ve
    /// kapı izleyicisi şart penceresini açar.
    pub async fn baslat(self: &Arc<Self>) {
        self.kapi_izleyicisini_baslat();
        let u = self.clone();
        let anahtar = tokio::task::spawn_blocking(move || u.kasa.oku())
            .await
            .ok()
            .flatten();
        let Some(anahtar) = anahtar else {
            return self.anahtar_iste(AnahtarNedeni::Yok);
        };
        let Ok(saglayici) = self.uretici.uret(&anahtar) else {
            return self.anahtar_iste(AnahtarNedeni::KayitliGecersiz);
        };
        self.evren.ayarla(saglayici);
        match self.kapi.ilk_durum().await {
            Ok(IlkDurum::KabulEdilmis) => {
                self.ic().sart_penceresi = SartPenceresiDurumu::Tamam;
                self.pencereler.ana_ac();
            }
            Ok(IlkDurum::KabulGerekli(_)) => {}
            Ok(IlkDurum::Reddedildi) => {
                self.ic().sart_penceresi = SartPenceresiDurumu::Kilitli;
                self.pencereler.sartlar_goster();
            }
            Err(EvrenError::AnahtarGecersiz) => self.anahtar_iste(AnahtarNedeni::KayitliGecersiz),
            Err(_) => {
                // EVREN'e ulaşılamıyor: geçmiş okunabilsin diye ana pencere açılır.
                self.ic().baglanti = BaglantiDurumu::Yok;
                self.pencereler.ana_ac();
            }
        }
    }

    fn anahtar_iste(&self, neden: AnahtarNedeni) {
        self.ic().sart_penceresi = SartPenceresiDurumu::AnahtarGerekli { neden };
        self.pencereler.sartlar_goster();
    }

    /// Kapı metin göstermeye geçtiğinde şart penceresini açar. Pencere
    /// açıksa yeniden yüklenir; ikinci pencere açılmaz (1c tasarımı §7.3).
    fn kapi_izleyicisini_baslat(self: &Arc<Self>) {
        let mut izleyici = self.kapi.izle();
        let zayif = Arc::downgrade(self);
        tokio::spawn(async move {
            loop {
                let gorunum = izleyici.borrow_and_update().gorunum.clone();
                let Some(uygulama) = zayif.upgrade() else {
                    return;
                };
                if let KapiGorunumu::Gosteriliyor(metin) = gorunum {
                    uygulama.sart_metni_geldi(metin);
                }
                drop(uygulama);
                if izleyici.changed().await.is_err() {
                    return;
                }
            }
        });
    }

    fn sart_metni_geldi(&self, metin: SartMetni) {
        let yeni = SartPenceresiDurumu::SartGerekli { metin };
        let goster = {
            let mut ic = self.ic();
            if ic.sart_penceresi == yeni {
                false
            } else {
                ic.sart_penceresi = yeni;
                ic.pencere_komutu == 0
            }
        };
        if goster {
            self.pencereler.sartlar_goster();
        }
    }

    pub fn sart_penceresi_durumu(&self) -> SartPenceresiDurumu {
        self.ic().sart_penceresi.clone()
    }

    /// Girilen anahtar önce şart durumu sorgusuyla denenir; 401 dönerse ya da
    /// EVREN'e ulaşılamazsa saklanmaz (1c tasarımı §7.2).
    pub async fn anahtari_kaydet(
        self: &Arc<Self>,
        anahtar: String,
    ) -> Result<AnahtarSonucu, KomutHatasi> {
        let _komut = PencereKomutu::yeni(self);
        let anahtar = anahtar.trim().to_string();
        if anahtar.is_empty() {
            return Ok(AnahtarSonucu::Gecersiz);
        }
        let Ok(saglayici) = self.uretici.uret(&anahtar) else {
            return Ok(AnahtarSonucu::Gecersiz);
        };
        match saglayici.sart_durumu().await {
            Ok(_) => {}
            Err(EvrenError::AnahtarGecersiz) => return Ok(AnahtarSonucu::Gecersiz),
            Err(EvrenError::Ag(_)) => return Ok(AnahtarSonucu::Ulasilamadi),
            Err(e) => return Err(e.into()),
        }
        let u = self.clone();
        let depo = tokio::task::spawn_blocking(move || u.kasa.yaz(&anahtar))
            .await
            .map_err(|e| KomutHatasi::ic(e.to_string()))?;
        self.evren.ayarla(saglayici);
        self.ic().baglanti = BaglantiDurumu::Var;
        let sonraki = match self.kapi.ilk_durum().await? {
            IlkDurum::KabulEdilmis => {
                self.sart_tamam();
                SartPenceresiDurumu::Tamam
            }
            IlkDurum::KabulGerekli(metin) => {
                let durum = SartPenceresiDurumu::SartGerekli { metin };
                self.ic().sart_penceresi = durum.clone();
                durum
            }
            IlkDurum::Reddedildi => {
                self.ic().sart_penceresi = SartPenceresiDurumu::Kilitli;
                SartPenceresiDurumu::Kilitli
            }
        };
        Ok(AnahtarSonucu::Kaydedildi { depo, sonraki })
    }

    /// "Kabul ediyorum". Sürüm gösterilenle aynı değilse kabul gönderilmez,
    /// güncel metin döner (1c tasarımı §7.3). Aynı anda verilen ret kabulü
    /// geçersiz kılar; pencere kilitli görünüme geçer.
    pub async fn sartlari_kabul_et(&self, surum: u32) -> Result<KabulYaniti, KomutHatasi> {
        let _komut = PencereKomutu::yeni(self);
        match self.kapi.kabul_et(surum).await? {
            KabulSonucu::Kabul => {
                self.sart_tamam();
                Ok(KabulYaniti::Kabul)
            }
            KabulSonucu::YeniMetin(metin) => {
                self.ic().sart_penceresi = SartPenceresiDurumu::SartGerekli {
                    metin: metin.clone(),
                };
                Ok(KabulYaniti::YeniMetin { metin })
            }
            KabulSonucu::Ret => {
                self.ic().sart_penceresi = SartPenceresiDurumu::Kilitli;
                Ok(KabulYaniti::Kilitli)
            }
            // Kapı artık metin göstermiyor: çift tıklamada ilk tıklama kabulü
            // çoktan göndermiştir. Son karar kabul değilse kabul edilmiş gibi
            // yapılmaz.
            KabulSonucu::Bosta => match self.kapi.durum().son_karar {
                Some(SartKarari::Kabul { .. }) => {
                    self.sart_tamam();
                    Ok(KabulYaniti::Kabul)
                }
                _ => Err(KomutHatasi::gecersiz("Onay bekleyen şart metni yok.")),
            },
        }
    }

    /// "Kabul etmiyorum": bekleyen turlar iptal edilir, pencere kilitli
    /// görünüme geçer (1c tasarımı §7.4).
    pub fn sartlari_reddet(&self) {
        let kilitli =
            self.kapi.reddet() || matches!(self.kapi.durum().gorunum, KapiGorunumu::Reddedildi);
        // Kapı bu arada kabulle kapandıysa ret bir şey değiştirmez.
        if kilitli {
            self.ic().sart_penceresi = SartPenceresiDurumu::Kilitli;
        }
    }

    /// Kilitli görünümdeki "Metni yeniden göster"; pencerenin yeni durumunu
    /// döner.
    pub async fn sart_metnini_goster(&self) -> Result<SartPenceresiDurumu, KomutHatasi> {
        let _komut = PencereKomutu::yeni(self);
        let nesil = self.kapi.durum().nesil;
        let durum = match self.kapi.gerekli(nesil).await? {
            GerekliSonucu::OnayGerekli { metin, .. } => SartPenceresiDurumu::SartGerekli { metin },
            GerekliSonucu::KabulEdildi { .. } => {
                self.sart_tamam();
                SartPenceresiDurumu::Tamam
            }
            GerekliSonucu::Ret => SartPenceresiDurumu::Kilitli,
        };
        self.ic().sart_penceresi = durum.clone();
        Ok(durum)
    }

    fn sart_tamam(&self) {
        self.ic().sart_penceresi = SartPenceresiDurumu::Tamam;
        self.pencereler.ana_ac();
        self.pencereler.sartlar_kapat();
    }

    /// Ana penceredeki "Anahtarı değiştir". Şart onayı bekleniyorsa önce şart
    /// metni gösterilir.
    pub fn anahtar_penceresini_ac(&self) {
        {
            let mut ic = self.ic();
            if !matches!(ic.sart_penceresi, SartPenceresiDurumu::SartGerekli { .. }) {
                ic.sart_penceresi = SartPenceresiDurumu::AnahtarGerekli {
                    neden: AnahtarNedeni::Degistir,
                };
            }
        }
        self.pencereler.sartlar_goster();
    }

    /// Kapatılmış şart penceresini yeniden açar. Şart penceresini kapatmak
    /// ret sayılmaz; bekleyen turlar beklemeyi sürdürür.
    pub fn sart_penceresini_ac(&self) {
        let ac = matches!(
            self.ic().sart_penceresi,
            SartPenceresiDurumu::SartGerekli { .. } | SartPenceresiDurumu::Kilitli
        );
        if ac {
            self.pencereler.sartlar_goster();
        }
    }
}
```

`crates/kalfa-uygulama/src/oturumlar.rs`:

```rust
//! Oturum tablosu (1c tasarımı §5): açık oturumlar, kayıttan açma, köprü,
//! kapatma. Aynı oturum ikinci kez açılmaz; açık olanın geçmişi döner.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use kalfa_core::aktarim::{KrediTutamaci, Paketleyici};
use kalfa_core::arac::AracKayitDefteri;
use kalfa_core::gecmis::{self, DurumOzeti, GecmisSayfasi};
use kalfa_core::izin::{Kip, Siniflandirici};
use kalfa_core::kayit::OturumKaydi;
use kalfa_core::kimlik::OturumId;
use kalfa_core::kurtarma::kurtar;
use kalfa_core::oturum::{self, OlayAlici, OturumAyarlari, OturumBaglami, OturumTutamaci};
use kalfa_core::protokol::Komut;
use kalfa_tools::{CalismaAlani as AracAlani, varsayilan_araclar};
use tokio::task::JoinHandle;

use crate::arayuz::KanalYazici;
use crate::hata::KomutHatasi;
use crate::sozlesme::{
    CalismaAlani, OnayGirdisi, OturumAcilisi, OturumSatiri, SaltOkunurNedeni, UygulamaDurumu,
};
use crate::{Uygulama, dizin, guvenlik, kopru, modeller};

/// Açılışta ve her eski sayfada yüklenen tur sayısı (1c tasarımı §9.3).
pub const SAYFA_TURU: usize = 20;

/// Kapatılan oturumun görevinin bitmesi için beklenen en uzun süre.
const KAPANIS_SURESI: Duration = Duration::from_secs(5);

pub(crate) struct AcikOturum {
    tutamac: OturumTutamaci,
    /// `oturumu_dinle` çağrılana kadar burada durur.
    alici: Option<OlayAlici>,
    kopru: Option<(JoinHandle<()>, KrediTutamaci)>,
    kip: Kip,
    yol: PathBuf,
    calisma_alani: PathBuf,
}

fn ic_hata(e: impl std::fmt::Display) -> KomutHatasi {
    KomutHatasi::ic(e.to_string())
}

impl Uygulama {
    pub fn uygulama_durumu(&self) -> UygulamaDurumu {
        let ic = self.ic();
        UygulamaDurumu {
            baglanti: ic.baglanti,
            anahtar_deposu: self.kasa.tur(),
            modeller: modeller::modeller(),
            varsayilan_model: modeller::VARSAYILAN_MODEL.to_string(),
            son_oturum: ic.son_oturum,
        }
    }

    pub async fn oturumlari_listele(&self) -> Result<Vec<OturumSatiri>, KomutHatasi> {
        let dizin = self.oturum_dizini.clone();
        let ozetler = tokio::task::spawn_blocking(move || gecmis::oturumlari_listele(&dizin))
            .await
            .map_err(ic_hata)??;
        Ok(ozetler.into_iter().map(OturumSatiri::from).collect())
    }

    /// Yerel klasör seçiciyi açar. Seçilen yol yalnızca bu çalıştırmada
    /// geçerli bir kimlikle arayüze verilir (1c tasarımı §5.1).
    pub async fn calisma_alani_sec(&self) -> Option<CalismaAlani> {
        let yol = self.iletisim.klasor_sec().await?;
        let mut ic = self.ic();
        ic.son_alan += 1;
        let kimlik = ic.son_alan;
        ic.calisma_alanlari.insert(kimlik, yol.clone());
        Some(CalismaAlani {
            kimlik,
            yol: yol.display().to_string(),
        })
    }

    pub async fn oturum_baslat(
        &self,
        calisma_alani: u64,
        model: String,
    ) -> Result<OturumAcilisi, KomutHatasi> {
        if !modeller::bilinen_model_mi(&model) {
            return Err(KomutHatasi::gecersiz(format!("Bilinmeyen model: {model}")));
        }
        let kok = self
            .ic()
            .calisma_alanlari
            .get(&calisma_alani)
            .cloned()
            .ok_or_else(|| {
                KomutHatasi::gecersiz("Çalışma alanı seçilmemiş. Klasörü yeniden seçin.")
            })?;
        let baglam = self.baglam(&kok)?;
        std::fs::create_dir_all(&self.oturum_dizini)?;
        let oturum = OturumId::yeni();
        let yol = dizin::kayit_yolu(&self.oturum_dizini, oturum);
        let mut ayarlar = OturumAyarlari::yeni(model.clone(), &kok);
        ayarlar.kayit_yolu = Some(yol.clone());
        let (tutamac, alici) = oturum::baslat_kimlikle(oturum, ayarlar, baglam)?;
        self.tabloya_ekle(
            oturum,
            AcikOturum {
                tutamac,
                alici: Some(alici),
                kopru: None,
                kip: Kip::Siki,
                yol,
                calisma_alani: kok.clone(),
            },
        );
        Ok(OturumAcilisi {
            oturum,
            calisma_alani: Some(kok.display().to_string()),
            gecmis: GecmisSayfasi {
                ogeler: Vec::new(),
                daha_eski_var: false,
            },
            ozet: DurumOzeti {
                model: Some(model),
                kip: Some(Kip::Siki),
                ..DurumOzeti::default()
            },
            salt_okunur: None,
        })
    }

    /// Oturumu kayıttan açar (1c tasarımı §5.1). Açıksa açık olanın geçmişini
    /// döner. Çalışma alanı yoksa ya da kayıtta yazmıyorsa oturum çekirdekte
    /// başlatılmaz; yalnızca geçmiş gösterilir.
    pub async fn oturum_ac(&self, oturum: OturumId) -> Result<OturumAcilisi, KomutHatasi> {
        self.kapanmayi_bekle(oturum).await;
        if let Some((yol, kok)) = self.acik_oturum(oturum) {
            return self.acilis(oturum, yol, Some(kok), None).await;
        }
        let yol = dizin::kayit_yolu(&self.oturum_dizini, oturum);
        let okunacak = yol.clone();
        let kayitlar = tokio::task::spawn_blocking(move || OturumKaydi::oku(&okunacak))
            .await
            .map_err(ic_hata)?
            .map_err(KomutHatasi::kayit)?;
        let durum = kurtar(&kayitlar).map_err(ic_hata)?;
        if durum.oturum != oturum {
            return Err(KomutHatasi::gecersiz(
                "Kayıt dosyasının adı içindeki oturum kimliğiyle uyuşmuyor.",
            ));
        }
        let Some(kok) = durum.calisma_alani.clone() else {
            let neden = SaltOkunurNedeni::CalismaAlaniKayitsiz;
            return self.acilis(oturum, yol, None, Some(neden)).await;
        };
        if !kok.is_dir() {
            let neden = SaltOkunurNedeni::CalismaAlaniYok {
                yol: kok.display().to_string(),
            };
            return self.acilis(oturum, yol, Some(kok), Some(neden)).await;
        }
        let baglam = self.baglam(&kok)?;
        let ayarlar = OturumAyarlari::yeni(durum.model.clone(), &kok);
        let (tutamac, alici, kurtarilan) = match oturum::devam_et(&yol, ayarlar, baglam) {
            Ok(v) => v,
            Err(e) => {
                // Aynı oturumu eşzamanlı bir çağrı (ör. çift tıklama) açtıysa
                // kayıt kilidi onda; açılmış oturumun geçmişi döner.
                if let Some((yol, kok)) = self.acik_oturum(oturum) {
                    return self.acilis(oturum, yol, Some(kok), None).await;
                }
                return Err(e.into());
            }
        };
        self.tabloya_ekle(
            oturum,
            AcikOturum {
                tutamac,
                alici: Some(alici),
                kopru: None,
                kip: kurtarilan.kip,
                yol: yol.clone(),
                calisma_alani: kok.clone(),
            },
        );
        self.acilis(oturum, yol, Some(kok), None).await
    }

    fn acik_oturum(&self, oturum: OturumId) -> Option<(PathBuf, PathBuf)> {
        self.ic()
            .oturumlar
            .get(&oturum)
            .map(|a| (a.yol.clone(), a.calisma_alani.clone()))
    }

    /// Kaydın son `SAYFA_TURU` turu ve durum çubuğunun başlangıç değerleri.
    async fn acilis(
        &self,
        oturum: OturumId,
        yol: PathBuf,
        kok: Option<PathBuf>,
        salt_okunur: Option<SaltOkunurNedeni>,
    ) -> Result<OturumAcilisi, KomutHatasi> {
        let (ilk_sayfa, ozet) = tokio::task::spawn_blocking(move || -> io::Result<_> {
            let kayitlar = OturumKaydi::oku(&yol)?;
            let ogeler = gecmis::gorunum_uret(&kayitlar);
            Ok((
                gecmis::sayfa(&ogeler, None, SAYFA_TURU),
                gecmis::durum_ozeti(&kayitlar),
            ))
        })
        .await
        .map_err(ic_hata)??;
        self.ic().son_oturum = Some(oturum);
        Ok(OturumAcilisi {
            oturum,
            calisma_alani: kok.map(|k| k.display().to_string()),
            gecmis: ilk_sayfa,
            ozet,
            salt_okunur,
        })
    }

    /// `once_sira`'dan önceki `SAYFA_TURU` tur. Kayıt açıkken de okunabilir:
    /// kilit kaydın yanındaki dosyadadır (1c tasarımı §5.2).
    pub async fn gecmis_sayfasi(
        &self,
        oturum: OturumId,
        once_sira: u64,
    ) -> Result<GecmisSayfasi, KomutHatasi> {
        let yol = dizin::kayit_yolu(&self.oturum_dizini, oturum);
        tokio::task::spawn_blocking(move || -> io::Result<GecmisSayfasi> {
            let kayitlar = OturumKaydi::oku(&yol)?;
            Ok(gecmis::sayfa(
                &gecmis::gorunum_uret(&kayitlar),
                Some(once_sira),
                SAYFA_TURU,
            ))
        })
        .await
        .map_err(ic_hata)?
        .map_err(KomutHatasi::kayit)
    }

    /// Oturumun olaylarını paketleyip `yazici`ya yazan köprüyü başlatır.
    /// Açılan oturum başına bir kez çağrılır.
    pub fn oturumu_dinle(
        &self,
        oturum: OturumId,
        yazici: Box<dyn KanalYazici>,
    ) -> Result<(), KomutHatasi> {
        let mut ic = self.ic();
        let acik = ic
            .oturumlar
            .get_mut(&oturum)
            .ok_or_else(KomutHatasi::oturum_yok)?;
        let alici = acik
            .alici
            .take()
            .ok_or_else(|| KomutHatasi::gecersiz("Bu oturum zaten dinleniyor."))?;
        let (paketleyici, kredi) = Paketleyici::yeni(alici, self.aktarim);
        let gorev = tokio::spawn(kopru::calistir(paketleyici, yazici));
        acik.kopru = Some((gorev, kredi));
        Ok(())
    }

    pub fn paketi_isledim(&self, oturum: OturumId, no: u64) -> Result<(), KomutHatasi> {
        let kredi = self
            .ic()
            .oturumlar
            .get(&oturum)
            .and_then(|a| a.kopru.as_ref().map(|(_, kredi)| kredi.clone()))
            .ok_or_else(KomutHatasi::oturum_yok)?;
        kredi
            .isledim(no)
            .map_err(|e| KomutHatasi::gecersiz(e.to_string()))
    }

    fn tutamac(&self, oturum: OturumId) -> Result<OturumTutamaci, KomutHatasi> {
        self.ic()
            .oturumlar
            .get(&oturum)
            .map(|a| a.tutamac.clone())
            .ok_or_else(KomutHatasi::oturum_yok)
    }

    pub async fn mesaj_gonder(&self, oturum: OturumId, metin: String) -> Result<(), KomutHatasi> {
        let tutamac = self.tutamac(oturum)?;
        self.ic().son_oturum = Some(oturum);
        tutamac.gonder(Komut::KullaniciMesaji { metin }).await?;
        Ok(())
    }

    pub fn turu_iptal_et(&self, oturum: OturumId) -> Result<(), KomutHatasi> {
        self.tutamac(oturum)?.iptal_et();
        Ok(())
    }

    pub async fn arac_onayi(
        &self,
        oturum: OturumId,
        kararlar: Vec<OnayGirdisi>,
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
    }

    pub async fn model_degistir(&self, oturum: OturumId, model: String) -> Result<(), KomutHatasi> {
        if !modeller::bilinen_model_mi(&model) {
            return Err(KomutHatasi::gecersiz(format!("Bilinmeyen model: {model}")));
        }
        self.tutamac(oturum)?
            .gonder(Komut::ModelDegistir { model })
            .await?;
        Ok(())
    }

    /// İzin kipini değiştirir. Yükseltme yerel iletişim kutusunda onaylanır;
    /// reddedilirse kip değişmez ve eski kip döner (1c tasarımı §6).
    pub async fn izin_kipi_degistir(&self, oturum: OturumId, kip: Kip) -> Result<Kip, KomutHatasi> {
        let (tutamac, eski) = {
            let ic = self.ic();
            let acik = ic
                .oturumlar
                .get(&oturum)
                .ok_or_else(KomutHatasi::oturum_yok)?;
            (acik.tutamac.clone(), acik.kip)
        };
        if kip == eski {
            return Ok(eski);
        }
        if guvenlik::yukseltme_mi(eski, kip) {
            let mesaj = format!(
                "İzin kipi {} yerine {} olacak.\n\n{}",
                guvenlik::kip_adi(eski),
                guvenlik::kip_adi(kip),
                guvenlik::yukseltme_aciklamasi(kip)
            );
            let onay = self
                .iletisim
                .onayla("İzin kipini yükselt", &mesaj, "Yükselt", "Vazgeç")
                .await;
            if !onay {
                return Ok(eski);
            }
        }
        tutamac.gonder(Komut::IzinKipiDegistir { kip }).await?;
        if let Some(acik) = self.ic().oturumlar.get_mut(&oturum) {
            acik.kip = kip;
        }
        Ok(kip)
    }

    /// Oturumu kapatır: süren tur iptal edilir, olay alıcısı bırakılır, kayıt
    /// kilidi çözülür (1c tasarımı §5.1).
    pub async fn oturumu_kapat(&self, oturum: OturumId) -> Result<(), KomutHatasi> {
        let tutamac = self
            .kapatmaya_basla(oturum)
            .ok_or_else(KomutHatasi::oturum_yok)?;
        {
            let mut ic = self.ic();
            if ic.son_oturum == Some(oturum) {
                ic.son_oturum = None;
            }
        }
        self.kapanisi_bekle(oturum, tutamac).await;
        Ok(())
    }

    /// Ana pencere yeniden yüklendi ya da kapandı: ona bağlı bütün oturumlar
    /// kapanır (1c tasarımı §5.6). Tablo hemen boşalır; yeni sayfa aynı oturumu
    /// açarsa önce kapanış beklenir. Görevlerin bitmesini `kapanislari_bekle`
    /// bekler.
    pub fn ana_pencere_gitti(&self) -> Vec<(OturumId, OturumTutamaci)> {
        let kimlikler: Vec<OturumId> = self.ic().oturumlar.keys().copied().collect();
        kimlikler
            .into_iter()
            .filter_map(|oturum| self.kapatmaya_basla(oturum).map(|t| (oturum, t)))
            .collect()
    }

    pub async fn kapanislari_bekle(&self, kapananlar: Vec<(OturumId, OturumTutamaci)>) {
        for (oturum, tutamac) in kapananlar {
            self.kapanisi_bekle(oturum, tutamac).await;
        }
    }

    fn kapatmaya_basla(&self, oturum: OturumId) -> Option<OturumTutamaci> {
        let acik = {
            let mut ic = self.ic();
            let acik = ic.oturumlar.remove(&oturum)?;
            ic.kapaniyor.insert(oturum, acik.tutamac.clone());
            acik
        };
        acik.tutamac.iptal_et();
        if let Some((gorev, _)) = &acik.kopru {
            // Paketleyici ve içindeki olay alıcısı bırakılır; oturum görevi
            // olay kuyruğunun kapandığını görüp biter.
            gorev.abort();
        }
        Some(acik.tutamac)
    }

    async fn kapanisi_bekle(&self, oturum: OturumId, tutamac: OturumTutamaci) {
        let _ = tokio::time::timeout(KAPANIS_SURESI, tutamac.kapanmasini_bekle()).await;
        self.ic().kapaniyor.remove(&oturum);
    }

    async fn kapanmayi_bekle(&self, oturum: OturumId) {
        let tutamac = self.ic().kapaniyor.get(&oturum).cloned();
        if let Some(tutamac) = tutamac {
            let _ = tokio::time::timeout(KAPANIS_SURESI, tutamac.kapanmasini_bekle()).await;
        }
    }

    fn tabloya_ekle(&self, oturum: OturumId, acik: AcikOturum) {
        let mut ic = self.ic();
        ic.son_oturum = Some(oturum);
        ic.oturumlar.insert(oturum, acik);
    }

    /// Çalışma alanının araçları ve ortak kilitleriyle oturum bağlamı.
    fn baglam(&self, kok: &Path) -> Result<OturumBaglami, KomutHatasi> {
        let siniflandirici = Arc::new(Siniflandirici::yeni(kok, &[]).map_err(ic_hata)?);
        let alan = AracAlani::ac(siniflandirici.clone())?;
        let araclar = Arc::new(AracKayitDefteri::yeni(varsayilan_araclar(alan)).map_err(ic_hata)?);
        Ok(OturumBaglami {
            saglayici: self.evren.clone(),
            araclar,
            siniflandirici,
            kilitler: self.kilitler.al(kok)?,
            sart_kapisi: self.kapi.clone(),
        })
    }

    /// Bağlantıyı sistem tarayıcısında açar: yalnızca http/https ve yalnızca
    /// kullanıcı yerel iletişim kutusunda onaylarsa (1c tasarımı §6).
    pub async fn baglanti_ac(&self, adres: String) -> Result<(), KomutHatasi> {
        let url = guvenlik::acilabilir_adres(&adres)
            .ok_or_else(|| KomutHatasi::gecersiz("Yalnızca http ve https adresleri açılabilir."))?;
        let mesaj = format!("Bu adres sistem tarayıcısında açılacak:\n\n{url}");
        if self
            .iletisim
            .onayla("Bağlantıyı aç", &mesaj, "Aç", "Vazgeç")
            .await
        {
            self.acici.ac(&url).map_err(KomutHatasi::ic)?;
        }
        Ok(())
    }
}
```

`crates/kalfa-uygulama/src/kopru.rs`:

```rust
//! Köprü: paketleyicinin mesajlarını kanala yazar (1c tasarımı §4). Her
//! oturum için ayrı bir görevde çalışır; `sonraki` paket doldururken
//! bırakılmaya dayanıklı olmadığı için `select!` içinde çağrılmaz.

use kalfa_core::aktarim::{KanalMesaji, OlayKaynagi, Paketleyici};

use crate::arayuz::KanalYazici;

pub(crate) async fn calistir<K: OlayKaynagi>(
    mut paketleyici: Paketleyici<K>,
    yazici: Box<dyn KanalYazici>,
) {
    while let Some(mesaj) = paketleyici.sonraki().await {
        let son = matches!(mesaj, KanalMesaji::Kapandi { .. });
        if yazici.gonder(mesaj).is_err() || son {
            break;
        }
    }
}
```

`crates/kalfa-uygulama/src/test_destek.rs`:

```rust
//! Testler için sahte bileşenler (`test-destek` özelliği). Uygulama koduna
//! girmez.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use kalfa_core::aktarim::{AktarimAyarlari, KanalMesaji};
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::SahteSaglayici;
use kalfa_evren::EvrenError;
use tokio::sync::mpsc;
use url::Url;

use crate::anahtar::{AnahtarDeposu, AnahtarKasasi, BellekDeposu};
use crate::arayuz::{Acici, Iletisim, KanalYazici, Pencereler};
use crate::evren::SaglayiciUretici;
use crate::{Bilesenler, Uygulama};

/// Bu anahtarla üretilen sağlayıcı her isteğe 401 döner.
pub const GECERSIZ_ANAHTAR: &str = "gecersiz";
/// Bu anahtarla üretilen sağlayıcı EVREN'e ulaşamaz.
pub const AGSIZ_ANAHTAR: &str = "agsiz";

/// Anahtara göre sağlayıcı: `GECERSIZ_ANAHTAR` ve `AGSIZ_ANAHTAR` dışındaki
/// her anahtar ortak sahte sağlayıcıyı verir.
pub struct SahteUretici {
    pub saglayici: Arc<SahteSaglayici>,
}

impl SaglayiciUretici for SahteUretici {
    fn uret(&self, anahtar: &str) -> Result<Arc<dyn Saglayici>, EvrenError> {
        let hata = match anahtar {
            GECERSIZ_ANAHTAR => EvrenError::AnahtarGecersiz,
            AGSIZ_ANAHTAR => EvrenError::Ag("bağlantı reddedildi".into()),
            _ => return Ok(self.saglayici.clone()),
        };
        let saglayici = SahteSaglayici::yeni(Vec::new());
        saglayici.sart_durumu_hatasi(Some(hata));
        Ok(saglayici)
    }
}

/// Yerel iletişim kutuları: klasör seçici verilen yolu döner, onay kutusu
/// verilen cevabı verir; sorulan başlık ve mesajlar kaydedilir.
#[derive(Default)]
pub struct SahteIletisim {
    klasor: Mutex<Option<PathBuf>>,
    onay: AtomicBool,
    sorulanlar: Mutex<Vec<String>>,
}

impl SahteIletisim {
    pub fn klasor_ver(&self, yol: &Path) {
        *self.klasor.lock().unwrap() = Some(yol.to_path_buf());
    }

    pub fn onay_ver(&self, evet: bool) {
        self.onay.store(evet, Ordering::SeqCst);
    }

    /// Sorulan onayların "başlık: mesaj" metinleri.
    pub fn sorulanlar(&self) -> Vec<String> {
        self.sorulanlar.lock().unwrap().clone()
    }
}

#[async_trait]
impl Iletisim for SahteIletisim {
    async fn klasor_sec(&self) -> Option<PathBuf> {
        self.klasor.lock().unwrap().clone()
    }

    async fn onayla(&self, baslik: &str, mesaj: &str, _tamam: &str, _vazgec: &str) -> bool {
        self.sorulanlar
            .lock()
            .unwrap()
            .push(format!("{baslik}: {mesaj}"));
        self.onay.load(Ordering::SeqCst)
    }
}

#[derive(Default)]
pub struct SahteAcici {
    acilanlar: Mutex<Vec<String>>,
}

impl SahteAcici {
    pub fn acilanlar(&self) -> Vec<String> {
        self.acilanlar.lock().unwrap().clone()
    }
}

impl Acici for SahteAcici {
    fn ac(&self, adres: &Url) -> Result<(), String> {
        self.acilanlar.lock().unwrap().push(adres.to_string());
        Ok(())
    }
}

/// Pencere işlemlerini sırasıyla kaydeder: "ana_ac", "sartlar_goster",
/// "sartlar_kapat".
#[derive(Default)]
pub struct SahtePencereler {
    olaylar: Mutex<Vec<&'static str>>,
}

impl SahtePencereler {
    pub fn olaylar(&self) -> Vec<&'static str> {
        self.olaylar.lock().unwrap().clone()
    }

    pub fn temizle(&self) {
        self.olaylar.lock().unwrap().clear();
    }
}

impl Pencereler for SahtePencereler {
    fn ana_ac(&self) {
        self.olaylar.lock().unwrap().push("ana_ac");
    }

    fn sartlar_goster(&self) {
        self.olaylar.lock().unwrap().push("sartlar_goster");
    }

    fn sartlar_kapat(&self) {
        self.olaylar.lock().unwrap().push("sartlar_kapat");
    }
}

/// Kanal mesajlarını bir `mpsc` kanalına aktarır.
pub struct SahteKanal {
    tx: mpsc::UnboundedSender<KanalMesaji>,
}

impl SahteKanal {
    pub fn yeni() -> (Box<Self>, mpsc::UnboundedReceiver<KanalMesaji>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Box::new(Self { tx }), rx)
    }
}

impl KanalYazici for SahteKanal {
    fn gonder(&self, mesaj: KanalMesaji) -> Result<(), String> {
        self.tx.send(mesaj).map_err(|e| e.to_string())
    }
}

/// Kalıcı depo gibi davranan ama yazılamayan depo (Secret Service çalışmıyor).
pub struct YazilamayanDepo;

impl AnahtarDeposu for YazilamayanDepo {
    fn oku(&self) -> Result<Option<String>, crate::anahtar::DepoHatasi> {
        Ok(None)
    }

    fn yaz(&self, _anahtar: &str) -> Result<(), crate::anahtar::DepoHatasi> {
        Err(crate::anahtar::DepoHatasi("Secret Service yok".into()))
    }
}

/// Testin de okuyabildiği kalıcı depo.
pub struct PaylasilanDepo(pub Arc<BellekDeposu>);

impl AnahtarDeposu for PaylasilanDepo {
    fn oku(&self) -> Result<Option<String>, crate::anahtar::DepoHatasi> {
        self.0.oku()
    }

    fn yaz(&self, anahtar: &str) -> Result<(), crate::anahtar::DepoHatasi> {
        self.0.yaz(anahtar)
    }
}

/// Sahteleriyle kurulmuş uygulama ve sahtelerine erişim.
pub struct TestUygulamasi {
    pub uygulama: Arc<Uygulama>,
    pub saglayici: Arc<SahteSaglayici>,
    /// Kalıcı depo gibi davranan depo.
    pub depo: Arc<BellekDeposu>,
    pub iletisim: Arc<SahteIletisim>,
    pub acici: Arc<SahteAcici>,
    pub pencereler: Arc<SahtePencereler>,
}

/// `kayitli_anahtar` verilirse kalıcı depoda o anahtar kayıtlıymış gibi
/// başlar. Veri dizini `dizin`dir.
pub fn test_uygulamasi(
    dizin: &Path,
    saglayici: Arc<SahteSaglayici>,
    kayitli_anahtar: Option<&str>,
) -> TestUygulamasi {
    let depo = Arc::new(BellekDeposu::default());
    if let Some(anahtar) = kayitli_anahtar {
        depo.yaz(anahtar).unwrap();
    }
    let iletisim = Arc::new(SahteIletisim::default());
    let acici = Arc::new(SahteAcici::default());
    let pencereler = Arc::new(SahtePencereler::default());
    let uygulama = Uygulama::yeni(Bilesenler {
        veri_dizini: dizin.to_path_buf(),
        kasa: AnahtarKasasi::yeni(Some(Box::new(PaylasilanDepo(depo.clone())))),
        uretici: Arc::new(SahteUretici {
            saglayici: saglayici.clone(),
        }),
        iletisim: iletisim.clone(),
        acici: acici.clone(),
        pencereler: pencereler.clone(),
        aktarim: AktarimAyarlari::default(),
    });
    TestUygulamasi {
        uygulama,
        saglayici,
        depo,
        iletisim,
        acici,
        pencereler,
    }
}
```

- [ ] **Step 4: Testlerin geçtiğini gör**

Run: `cargo test -p kalfa-uygulama --locked`
Expected: PASS — `kurallar` 11, `oturum_tablosu` 13, `sart_akisi` 17 test.

- [ ] **Step 5: Biçim, lint ve bağımlılık yönü**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
Expected: temiz.

Run: `cargo tree -p kalfa-uygulama -e normal | grep -E "tauri|wry|webkit"`
Expected: çıktı yok.

- [ ] **Step 6: Commit**

```bash
git add crates/kalfa-uygulama
git commit -m "kalfa-uygulama: şart akışı, oturum tablosu ve olay köprüsü"
git push
```

---

### Task 3: `kalfa` — Tauri kabuğu, pencereler, komutlar ve izinler

İki pencere, 22 komut, pencere izinleri, yerel iletişim kutuları, sistem tarayıcısı ve kanal bağdaştırıcısı. Testler Tauri'nin mock runtime'ında ACL'in gerçekten uygulandığını sınar: her pencere yalnızca kendi komutlarını çağırabilir, Tauri'nin ve eklentilerin komutları kapalıdır, tanımsız bir pencere hiçbir şey çağıramaz.

**Files:**
- Var (plan commit'inde): `crates/kalfa/Cargo.toml`, `crates/kalfa/tauri.conf.json`, `crates/kalfa/icons/`, `crates/kalfa/src/main.rs`
- Modify: `crates/kalfa/build.rs`, `crates/kalfa/src/lib.rs`
- Create: `crates/kalfa/windows-test.manifest`, `crates/kalfa/capabilities/ana-pencere.json`, `crates/kalfa/capabilities/sart-penceresi.json`
- Create: `crates/kalfa/src/{komut_adlari,komutlar,pencereler,iletisim,kanal}.rs`
- Test: `crates/kalfa/tests/izinler.rs`

**Interfaces:**
- Consumes: Görev 1–2'deki `Uygulama` yöntemleri, `Bilesenler`, `arayuz` trait'leri, `anahtar::{AnahtarKasasi, SistemDeposu, AnahtarDeposu}`, `dizin::{veri_dizini, VERI_DIZINI_DEGISKENI}`, `evren::{EvrenUretici, TABAN_DEGISKENI}`, `guvenlik::gezinme_izinli`; `kalfa_evren::VARSAYILAN_TABAN`.
- Produces:
  - **Kütüphane (`kalfa_kabuk`):** `calistir()`, `eklentiler<R>(Builder<R>) -> Builder<R>`, `ANAHTAR_DEPOSU_DEGISKENI`.
  - **Komutlar:** `komut_adlari::{ANA_PENCERE_KOMUTLARI: [&str; 17], SART_PENCERESI_KOMUTLARI: [&str; 6]}`, `komutlar::kur<R>(Builder<R>) -> Builder<R>`.
  - **Pencereler:** `pencereler::{ANA, SARTLAR, one_getir}`.

**Notlar:**
- **İzin adları:** `build.rs` komut listesini `komut_adlari.rs`'den `include!` ile okur ve `tauri_build::AppManifest::commands`'a verir. Tauri her komut için `allow-<komut, _ yerine ->` iznini üretir; capability dosyaları bu adları kullanır.
- **Windows test manifest'i:** Windows'ta Tauri'ye bağlanan test ikilisi Common Controls v6 manifest'i olmadan açılmaz (`STATUS_ENTRYPOINT_NOT_FOUND`). `build.rs` manifest'i yalnızca `tests/` altındaki testlere bağlar. Bu yüzden kütüphanenin ve ikilinin kendi test hedefleri `Cargo.toml`'da kapalıdır. `cargo:rustc-link-arg-tests` bir test hedefi ister; `tests/izinler.rs` bu görevde gelir.
- **Üretilen dosyalar:** Tauri `crates/kalfa/gen/` ve `crates/kalfa/permissions/autogenerated/` üretir; ikisi de `.gitignore`'da.
- **Ortam değişkenleri:** `KALFA_EVREN_TABAN` yalnızca hata ayıklama derlemesinde okunur (sürümde anahtar başka bir adrese gidemez). `KALFA_ANAHTAR_DEPOSU=bellek` işletim sisteminin deposunu kullanmaz. `KALFA_VERI_DIZINI` veri dizinini değiştirir.

- [ ] **Step 1: Komut listesini, izin dosyalarını ve başarısız testi yaz**

`crates/kalfa/src/komut_adlari.rs`:

```rust
// Pencerelerin komutları (1c tasarımı §6.1). `build.rs` bu dosyayı
// `include!` ile okuyup uygulama manifest'ini üretir; capability dosyaları
// ve `komutlar::kur`'daki işleyici aynı listeyi taşır (tests/izinler.rs
// denetler).

/// Ana pencerenin komutları.
pub const ANA_PENCERE_KOMUTLARI: [&str; 17] = [
    "uygulama_durumu",
    "oturumlari_listele",
    "calisma_alani_sec",
    "oturum_baslat",
    "oturum_ac",
    "oturumu_dinle",
    "paketi_isledim",
    "gecmis_sayfasi",
    "mesaj_gonder",
    "turu_iptal_et",
    "arac_onayi",
    "model_degistir",
    "izin_kipi_degistir",
    "oturumu_kapat",
    "baglanti_ac",
    "anahtar_penceresini_ac",
    "sart_penceresini_ac",
];

/// Şart penceresinin komutları. Şart kabulüne giden tek yol buradaki
/// `sartlari_kabul_et`'tir.
pub const SART_PENCERESI_KOMUTLARI: [&str; 6] = [
    "sart_penceresi_durumu",
    "anahtari_kaydet",
    "sartlari_kabul_et",
    "sartlari_reddet",
    "sart_metnini_goster",
    "baglanti_ac",
];
```

`crates/kalfa/capabilities/ana-pencere.json`:

```json
{
  "identifier": "ana-pencere",
  "description": "Ana pencere: oturum listesi, konuşma, onay kartları, durum çubuğu (1c tasarımı §6). Tauri'nin kendi izinleri verilmez; bu pencere pencere açamaz ve şart kabul edemez.",
  "windows": ["ana"],
  "permissions": [
    "allow-uygulama-durumu",
    "allow-oturumlari-listele",
    "allow-calisma-alani-sec",
    "allow-oturum-baslat",
    "allow-oturum-ac",
    "allow-oturumu-dinle",
    "allow-paketi-isledim",
    "allow-gecmis-sayfasi",
    "allow-mesaj-gonder",
    "allow-turu-iptal-et",
    "allow-arac-onayi",
    "allow-model-degistir",
    "allow-izin-kipi-degistir",
    "allow-oturumu-kapat",
    "allow-baglanti-ac",
    "allow-anahtar-penceresini-ac",
    "allow-sart-penceresini-ac"
  ]
}
```

`crates/kalfa/capabilities/sart-penceresi.json`:

```json
{
  "identifier": "sart-penceresi",
  "description": "Şart penceresi: anahtar formu, EVREN şart metni, kilitli görünüm (1c tasarımı §6, §7). Şart kabulüne giden tek yol buradaki sartlari_kabul_et'tir.",
  "windows": ["sartlar"],
  "permissions": [
    "allow-sart-penceresi-durumu",
    "allow-anahtari-kaydet",
    "allow-sartlari-kabul-et",
    "allow-sartlari-reddet",
    "allow-sart-metnini-goster",
    "allow-baglanti-ac"
  ]
}
```

`crates/kalfa/windows-test.manifest`:

```xml
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
</assembly>
```

`crates/kalfa/build.rs`:

```rust
include!("src/komut_adlari.rs");

fn main() {
    let mut komutlar: Vec<&str> = ANA_PENCERE_KOMUTLARI
        .iter()
        .chain(SART_PENCERESI_KOMUTLARI.iter())
        .copied()
        .collect();
    komutlar.sort_unstable();
    komutlar.dedup();
    // `AppManifest` 'static dilim ister; build betiği kısa ömürlüdür.
    let komutlar: &'static [&'static str] = Box::leak(komutlar.into_boxed_slice());
    println!("cargo:rerun-if-changed=src/komut_adlari.rs");
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(komutlar)),
    )
    .expect("tauri-build başarısız");

    // tauri-winres Common Controls v6 manifest'ini yalnızca ana ikiliye
    // bağlar; test ikilileri onsuz açılamaz (STATUS_ENTRYPOINT_NOT_FOUND).
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifest =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("windows-test.manifest");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
}
```

`crates/kalfa/tests/izinler.rs`:

```rust
//! Pencere izinleri (1c tasarımı §6, §11.2). Tauri'nin ACL'i mock runtime'da
//! da uygulanır: her pencere yalnızca kendi komutlarını çağırabilir, Tauri'nin
//! ve eklentilerin komutları hiçbir pencereye açık değildir.

use kalfa_kabuk::komut_adlari::{ANA_PENCERE_KOMUTLARI, SART_PENCERESI_KOMUTLARI};
use serde_json::Value;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

fn istek(komut: &str) -> InvokeRequest {
    InvokeRequest {
        cmd: komut.into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: if cfg!(windows) {
            "http://tauri.localhost"
        } else {
            "tauri://localhost"
        }
        .parse()
        .unwrap(),
        body: InvokeBody::default(),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.to_string(),
    }
}

/// Komutun ACL'den geçip geçmediği. `Uygulama` durumu yönetilmediği için
/// izinli komutlar da hata döner; ama o hata ACL hatası değildir.
fn izinli(pencere: &WebviewWindow<MockRuntime>, komut: &str) -> bool {
    match get_ipc_response(pencere, istek(komut)) {
        Ok(_) => true,
        Err(hata) => {
            let metin = hata.to_string();
            assert!(
                !metin.contains("not found"),
                "{komut} komutu kayıtlı değil: {metin}"
            );
            !metin.contains("not allowed")
        }
    }
}

fn uygulama() -> App<MockRuntime> {
    kalfa_kabuk::komutlar::kur(kalfa_kabuk::eklentiler(mock_builder()))
        .build(tauri::generate_context!())
        .unwrap()
}

fn pencere(app: &App<MockRuntime>, etiket: &str) -> WebviewWindow<MockRuntime> {
    WebviewWindowBuilder::new(app, etiket, WebviewUrl::default())
        .build()
        .unwrap()
}

/// Tauri'nin ve eklentilerin, webview'dan çağrılırsa tehlikeli olacak
/// komutları.
const CEKIRDEK_KOMUTLARI: [&str; 8] = [
    "plugin:window|close",
    "plugin:webview|create_webview_window",
    "plugin:event|emit",
    "plugin:path|resolve_directory",
    "plugin:app|app_hide",
    "plugin:dialog|open",
    "plugin:dialog|message",
    "plugin:opener|open_url",
];

#[test]
fn her_pencere_yalnizca_kendi_komutlarini_cagirir() {
    let app = uygulama();
    let ana = pencere(&app, "ana");
    let sartlar = pencere(&app, "sartlar");

    for komut in ANA_PENCERE_KOMUTLARI {
        assert!(izinli(&ana, komut), "ana pencere {komut} çağıramadı");
    }
    for komut in SART_PENCERESI_KOMUTLARI {
        assert!(izinli(&sartlar, komut), "şart penceresi {komut} çağıramadı");
    }
    // Şart kabulü yalnızca şart penceresinden gider (üst belge §11.8).
    for komut in SART_PENCERESI_KOMUTLARI {
        if !ANA_PENCERE_KOMUTLARI.contains(&komut) {
            assert!(!izinli(&ana, komut), "ana pencere {komut} çağırabildi");
        }
    }
    for komut in ANA_PENCERE_KOMUTLARI {
        if !SART_PENCERESI_KOMUTLARI.contains(&komut) {
            assert!(
                !izinli(&sartlar, komut),
                "şart penceresi {komut} çağırabildi"
            );
        }
    }
}

#[test]
fn cekirdek_ve_eklenti_komutlari_kapali() {
    let app = uygulama();
    for etiket in ["ana", "sartlar"] {
        let p = pencere(&app, etiket);
        for komut in CEKIRDEK_KOMUTLARI {
            assert!(!izinli(&p, komut), "{etiket} penceresi {komut} çağırabildi");
        }
    }
}

#[test]
fn baska_bir_pencere_hicbir_komutu_cagiramaz() {
    let app = uygulama();
    let baska = pencere(&app, "baska");
    for komut in ANA_PENCERE_KOMUTLARI
        .iter()
        .chain(SART_PENCERESI_KOMUTLARI.iter())
    {
        assert!(
            !izinli(&baska, komut),
            "tanımsız pencere {komut} çağırabildi"
        );
    }
}

fn json(yol: &str) -> Value {
    let tam = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(yol);
    serde_json::from_str(&std::fs::read_to_string(tam).unwrap()).unwrap()
}

fn izin_adlari(komutlar: &[&str]) -> Vec<String> {
    komutlar
        .iter()
        .map(|k| format!("allow-{}", k.replace('_', "-")))
        .collect()
}

#[test]
fn capability_dosyalari_komut_listeleriyle_ayni() {
    for (dosya, pencere, komutlar) in [
        (
            "capabilities/ana-pencere.json",
            "ana",
            &ANA_PENCERE_KOMUTLARI[..],
        ),
        (
            "capabilities/sart-penceresi.json",
            "sartlar",
            &SART_PENCERESI_KOMUTLARI[..],
        ),
    ] {
        let c = json(dosya);
        assert_eq!(c["windows"], serde_json::json!([pencere]), "{dosya}");
        assert!(c.get("webviews").is_none(), "{dosya}");
        assert!(
            c.get("remote").is_none(),
            "{dosya}: uzak köken izinli olmamalı"
        );
        let izinler: Vec<String> = c["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p.as_str().unwrap().to_string())
            .collect();
        assert_eq!(izinler, izin_adlari(komutlar), "{dosya}");
    }
}

#[test]
fn yapilandirma_pencere_acmaz_ve_csp_sikidir() {
    let c = json("tauri.conf.json");
    // Pencereleri yalnızca Rust açar (1c tasarımı §6).
    assert_eq!(c["app"]["windows"], serde_json::json!([]));
    assert_eq!(c["app"]["withGlobalTauri"], Value::Bool(false));
    let g = &c["app"]["security"];
    assert_eq!(g["freezePrototype"], Value::Bool(true));
    let csp = g["csp"].as_str().unwrap();
    for parca in [
        "default-src 'none'",
        "script-src 'self'",
        "style-src 'self'",
        "connect-src ipc: http://ipc.localhost",
        "base-uri 'none'",
        "form-action 'none'",
    ] {
        assert!(csp.contains(parca), "CSP'de {parca} yok: {csp}");
    }
    assert!(!csp.contains("unsafe"), "sürüm CSP'si gevşek: {csp}");
    assert!(
        !csp.contains("http://localhost"),
        "sürüm CSP'si geliştirme kökenine açık: {csp}"
    );
    let dev = g["devCsp"].as_str().unwrap();
    assert!(
        !dev.contains("unsafe-eval"),
        "geliştirme CSP'si eval'e açık: {dev}"
    );
}
```

- [ ] **Step 2: Testin başarısız olduğunu gör**

Run: `cargo test -p kalfa --locked`
Expected: FAIL. Derleme hatası: `could not find komut_adlari in kalfa_kabuk`, `cannot find function eklentiler` ve benzerleri.

- [ ] **Step 3: Kabuğu yaz**

`crates/kalfa/src/lib.rs`:

```rust
//! Kalfa masaüstü uygulamasının Tauri kabuğu (1c tasarımı §3, §6). İnce
//! tutulur: pencereler, komutlar, kanal bağdaştırıcısı, yerel iletişim
//! kutuları. Mantık `kalfa-uygulama`'dadır.

mod iletisim;
mod kanal;
pub mod komut_adlari;
pub mod komutlar;
mod pencereler;

use std::error::Error;
use std::sync::Arc;

use kalfa_core::aktarim::AktarimAyarlari;
use kalfa_evren::VARSAYILAN_TABAN;
use kalfa_uygulama::anahtar::{AnahtarDeposu, AnahtarKasasi, SistemDeposu};
use kalfa_uygulama::dizin::{VERI_DIZINI_DEGISKENI, veri_dizini};
use kalfa_uygulama::evren::{EvrenUretici, TABAN_DEGISKENI};
use kalfa_uygulama::{Bilesenler, Uygulama};
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Builder, Manager, Runtime, WindowEvent};

use crate::iletisim::{TauriAcici, TauriIletisim};
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
    if let Err(e) = komutlar::kur(kurucu).run(tauri::generate_context!()) {
        eprintln!("Kalfa çalıştırılamadı: {e}");
        std::process::exit(1);
    }
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
    let taban = std::env::var(TABAN_DEGISKENI)
        .ok()
        .filter(|_| cfg!(debug_assertions))
        .unwrap_or_else(|| VARSAYILAN_TABAN.to_string());
    let kalici: Option<Box<dyn AnahtarDeposu>> =
        if std::env::var(ANAHTAR_DEPOSU_DEGISKENI).as_deref() == Ok("bellek") {
            None
        } else {
            SistemDeposu::ac()
                .ok()
                .map(|depo| Box::new(depo) as Box<dyn AnahtarDeposu>)
        };
    Ok(Uygulama::yeni(Bilesenler {
        veri_dizini: veri,
        kasa: AnahtarKasasi::yeni(kalici),
        uretici: Arc::new(EvrenUretici { taban }),
        iletisim: Arc::new(TauriIletisim(app.clone())),
        acici: Arc::new(TauriAcici(app.clone())),
        pencereler: Arc::new(TauriPencereler(app.clone())),
        aktarim: AktarimAyarlari::default(),
    }))
}
```

`crates/kalfa/src/komutlar.rs`:

```rust
//! Pencerelerin komutları (1c tasarımı §6.1). Her komut `Uygulama`'nın aynı
//! adlı yöntemine iner; hangi pencerenin hangi komutu çağırabileceğini
//! `capabilities/` belirler. Komutların hepsi async'tir: Tauri eşzamanlı
//! komutları ana iş parçacığında çalıştırır ve orada pencere kurmak Windows'ta
//! kilitlenir.

use std::sync::Arc;

use kalfa_core::aktarim::KanalMesaji;
use kalfa_core::gecmis::GecmisSayfasi;
use kalfa_core::izin::Kip;
use kalfa_core::kimlik::OturumId;
use kalfa_uygulama::Uygulama;
use kalfa_uygulama::hata::KomutHatasi;
use kalfa_uygulama::sozlesme::{
    AnahtarSonucu, CalismaAlani, KabulYaniti, OnayGirdisi, OturumAcilisi, OturumSatiri,
    SartPenceresiDurumu, UygulamaDurumu,
};
use tauri::ipc::Channel;
use tauri::{Builder, Runtime, State};

use crate::kanal::TauriKanal;

type Sonuc<T> = Result<T, KomutHatasi>;

/// Komut işleyicisini kurar. Liste `komut_adlari.rs` ile aynıdır
/// (tests/izinler.rs denetler).
pub fn kur<R: Runtime>(kurucu: Builder<R>) -> Builder<R> {
    kurucu.invoke_handler(tauri::generate_handler![
        uygulama_durumu,
        oturumlari_listele,
        calisma_alani_sec,
        oturum_baslat,
        oturum_ac,
        oturumu_dinle,
        paketi_isledim,
        gecmis_sayfasi,
        mesaj_gonder,
        turu_iptal_et,
        arac_onayi,
        model_degistir,
        izin_kipi_degistir,
        oturumu_kapat,
        baglanti_ac,
        anahtar_penceresini_ac,
        sart_penceresini_ac,
        sart_penceresi_durumu,
        anahtari_kaydet,
        sartlari_kabul_et,
        sartlari_reddet,
        sart_metnini_goster,
    ])
}

// --- Ana pencere ---

#[tauri::command(rename_all = "snake_case")]
pub async fn uygulama_durumu(u: State<'_, Arc<Uygulama>>) -> Sonuc<UygulamaDurumu> {
    Ok(u.uygulama_durumu())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn oturumlari_listele(u: State<'_, Arc<Uygulama>>) -> Sonuc<Vec<OturumSatiri>> {
    u.oturumlari_listele().await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn calisma_alani_sec(u: State<'_, Arc<Uygulama>>) -> Sonuc<Option<CalismaAlani>> {
    Ok(u.calisma_alani_sec().await)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn oturum_baslat(
    u: State<'_, Arc<Uygulama>>,
    calisma_alani: u64,
    model: String,
) -> Sonuc<OturumAcilisi> {
    u.oturum_baslat(calisma_alani, model).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn oturum_ac(u: State<'_, Arc<Uygulama>>, oturum: OturumId) -> Sonuc<OturumAcilisi> {
    u.oturum_ac(oturum).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn oturumu_dinle(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    kanal: Channel<KanalMesaji>,
) -> Sonuc<()> {
    u.oturumu_dinle(oturum, Box::new(TauriKanal(kanal)))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn paketi_isledim(u: State<'_, Arc<Uygulama>>, oturum: OturumId, no: u64) -> Sonuc<()> {
    u.paketi_isledim(oturum, no)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn gecmis_sayfasi(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    once_sira: u64,
) -> Sonuc<GecmisSayfasi> {
    u.gecmis_sayfasi(oturum, once_sira).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mesaj_gonder(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    metin: String,
) -> Sonuc<()> {
    u.mesaj_gonder(oturum, metin).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn turu_iptal_et(u: State<'_, Arc<Uygulama>>, oturum: OturumId) -> Sonuc<()> {
    u.turu_iptal_et(oturum)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn arac_onayi(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    kararlar: Vec<OnayGirdisi>,
) -> Sonuc<()> {
    u.arac_onayi(oturum, kararlar).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_degistir(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    model: String,
) -> Sonuc<()> {
    u.model_degistir(oturum, model).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn izin_kipi_degistir(
    u: State<'_, Arc<Uygulama>>,
    oturum: OturumId,
    kip: Kip,
) -> Sonuc<Kip> {
    u.izin_kipi_degistir(oturum, kip).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn oturumu_kapat(u: State<'_, Arc<Uygulama>>, oturum: OturumId) -> Sonuc<()> {
    u.oturumu_kapat(oturum).await
}

/// İki pencerede de var: şart metnindeki bağlantılar da yerel onayla açılır.
#[tauri::command(rename_all = "snake_case")]
pub async fn baglanti_ac(u: State<'_, Arc<Uygulama>>, adres: String) -> Sonuc<()> {
    u.baglanti_ac(adres).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn anahtar_penceresini_ac(u: State<'_, Arc<Uygulama>>) -> Sonuc<()> {
    u.anahtar_penceresini_ac();
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn sart_penceresini_ac(u: State<'_, Arc<Uygulama>>) -> Sonuc<()> {
    u.sart_penceresini_ac();
    Ok(())
}

// --- Şart penceresi ---

#[tauri::command(rename_all = "snake_case")]
pub async fn sart_penceresi_durumu(u: State<'_, Arc<Uygulama>>) -> Sonuc<SartPenceresiDurumu> {
    Ok(u.sart_penceresi_durumu())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn anahtari_kaydet(u: State<'_, Arc<Uygulama>>, anahtar: String) -> Sonuc<AnahtarSonucu> {
    u.inner().anahtari_kaydet(anahtar).await
}

/// Şart kabulüne giden tek yol (üst belge §11.8).
#[tauri::command(rename_all = "snake_case")]
pub async fn sartlari_kabul_et(u: State<'_, Arc<Uygulama>>, surum: u32) -> Sonuc<KabulYaniti> {
    u.sartlari_kabul_et(surum).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn sartlari_reddet(u: State<'_, Arc<Uygulama>>) -> Sonuc<()> {
    u.sartlari_reddet();
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn sart_metnini_goster(u: State<'_, Arc<Uygulama>>) -> Sonuc<SartPenceresiDurumu> {
    u.sart_metnini_goster().await
}
```

`crates/kalfa/src/pencereler.rs`:

```rust
//! İki pencere yalnızca buradan açılır (1c tasarımı §6). Webview başka bir
//! adrese gidemez, yeni pencere açamaz.

use kalfa_uygulama::arayuz::Pencereler;
use kalfa_uygulama::guvenlik::gezinme_izinli;
use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const ANA: &str = "ana";
pub const SARTLAR: &str = "sartlar";

struct PencereTanimi {
    etiket: &'static str,
    sayfa: &'static str,
    baslik: &'static str,
    boyut: (f64, f64),
    en_kucuk: (f64, f64),
}

const ANA_PENCERE: PencereTanimi = PencereTanimi {
    etiket: ANA,
    sayfa: "index.html",
    baslik: "Kalfa",
    boyut: (1280.0, 800.0),
    en_kucuk: (960.0, 600.0),
};

const SART_PENCERESI: PencereTanimi = PencereTanimi {
    etiket: SARTLAR,
    sayfa: "sartlar.html",
    baslik: "Kalfa — EVREN hesabı",
    boyut: (760.0, 820.0),
    en_kucuk: (560.0, 600.0),
};

pub struct TauriPencereler<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> TauriPencereler<R> {
    /// Pencere açıksa öne getirir, değilse kurar.
    fn ac(&self, tanim: &PencereTanimi) -> Option<WebviewWindow<R>> {
        if let Some(pencere) = self.0.get_webview_window(tanim.etiket) {
            one_al(&pencere);
            return Some(pencere);
        }
        match kur(&self.0, tanim) {
            Ok(pencere) => Some(pencere),
            Err(e) => {
                eprintln!("Kalfa: {} penceresi açılamadı: {e}", tanim.etiket);
                None
            }
        }
    }
}

impl<R: Runtime> Pencereler for TauriPencereler<R> {
    fn ana_ac(&self) {
        self.ac(&ANA_PENCERE);
    }

    fn sartlar_goster(&self) {
        // Açık pencere yeniden yüklenir; yeni durumu `sart_penceresi_durumu`
        // ile okur. İkinci bir şart penceresi açılmaz.
        if let Some(pencere) = self.0.get_webview_window(SARTLAR) {
            if let Err(e) = pencere.reload() {
                eprintln!("Kalfa: şart penceresi yenilenemedi: {e}");
            }
            one_al(&pencere);
            return;
        }
        self.ac(&SART_PENCERESI);
    }

    fn sartlar_kapat(&self) {
        if let Some(pencere) = self.0.get_webview_window(SARTLAR)
            && let Err(e) = pencere.close()
        {
            eprintln!("Kalfa: şart penceresi kapatılamadı: {e}");
        }
    }
}

/// İkinci Kalfa süreci açılmak istendiğinde var olan pencere öne gelir.
pub fn one_getir<R: Runtime>(app: &AppHandle<R>) {
    let pencere = app
        .get_webview_window(ANA)
        .or_else(|| app.get_webview_window(SARTLAR));
    if let Some(pencere) = pencere {
        one_al(&pencere);
    }
}

fn one_al<R: Runtime>(pencere: &WebviewWindow<R>) {
    let _ = pencere.unminimize();
    let _ = pencere.show();
    let _ = pencere.set_focus();
}

fn kur<R: Runtime>(app: &AppHandle<R>, tanim: &PencereTanimi) -> tauri::Result<WebviewWindow<R>> {
    // Geliştirmede sayfalar Vite sunucusundan gelir; o köken yalnızca dev
    // derlemesinde izinlidir.
    let gelistirme = if tauri::is_dev() {
        app.config().build.dev_url.clone()
    } else {
        None
    };
    WebviewWindowBuilder::new(app, tanim.etiket, WebviewUrl::App(tanim.sayfa.into()))
        .title(tanim.baslik)
        .inner_size(tanim.boyut.0, tanim.boyut.1)
        .min_inner_size(tanim.en_kucuk.0, tanim.en_kucuk.1)
        .center()
        .on_navigation(move |adres| gezinme_izinli(adres, gelistirme.as_ref()))
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .build()
}
```

`crates/kalfa/src/iletisim.rs`:

```rust
//! Yerel iletişim kutuları ve sistem tarayıcısı (1c tasarımı §6). Onay
//! gerektiren işler webview'daki bir düğmeyle değil, işletim sisteminin
//! kutusuyla doğrulanır.

use std::path::PathBuf;

use async_trait::async_trait;
use kalfa_uygulama::arayuz::{Acici, Iletisim};
use tauri::{AppHandle, Manager, Runtime, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::oneshot;
use url::Url;

pub struct TauriIletisim<R: Runtime>(pub AppHandle<R>);

#[async_trait]
impl<R: Runtime> Iletisim for TauriIletisim<R> {
    async fn klasor_sec(&self) -> Option<PathBuf> {
        let (gonder, al) = oneshot::channel();
        let mut kutu = self.0.dialog().file().set_title("Çalışma alanı seçin");
        if let Some(pencere) = odakli_pencere(&self.0) {
            kutu = kutu.set_parent(&pencere);
        }
        kutu.pick_folder(move |secilen| {
            let _ = gonder.send(secilen);
        });
        al.await
            .ok()
            .flatten()
            .and_then(|yol| yol.simplified().into_path().ok())
    }

    async fn onayla(&self, baslik: &str, mesaj: &str, tamam: &str, vazgec: &str) -> bool {
        let (gonder, al) = oneshot::channel();
        let mut kutu = self
            .0
            .dialog()
            .message(mesaj)
            .title(baslik)
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom(
                tamam.to_string(),
                vazgec.to_string(),
            ));
        if let Some(pencere) = odakli_pencere(&self.0) {
            kutu = kutu.parent(&pencere);
        }
        kutu.show(move |tamam_mi| {
            let _ = gonder.send(tamam_mi);
        });
        al.await.unwrap_or(false)
    }
}

pub struct TauriAcici<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> Acici for TauriAcici<R> {
    fn ac(&self, adres: &Url) -> Result<(), String> {
        self.0
            .opener()
            .open_url(adres.as_str(), None::<&str>)
            .map_err(|e| e.to_string())
    }
}

/// Kutunun ait olduğu pencere; kutu onun önünde açılır.
fn odakli_pencere<R: Runtime>(app: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    app.webview_windows()
        .into_values()
        .find(|p| p.is_focused().unwrap_or(false))
}
```

`crates/kalfa/src/kanal.rs`:

```rust
//! Oturumun kanal mesajlarını webview'a taşıyan Tauri kanalı (1c tasarımı §4).

use kalfa_core::aktarim::KanalMesaji;
use kalfa_uygulama::arayuz::KanalYazici;
use tauri::ipc::Channel;

pub struct TauriKanal(pub Channel<KanalMesaji>);

impl KanalYazici for TauriKanal {
    fn gonder(&self, mesaj: KanalMesaji) -> Result<(), String> {
        self.0.send(mesaj).map_err(|e| e.to_string())
    }
}
```

- [ ] **Step 4: Testin geçtiğini gör**

Run: `cargo test -p kalfa --locked`
Expected: PASS — `test result: ok. 5 passed`.

- [ ] **Step 5: Biçim, lint ve bütün çalışma alanı**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
Expected: temiz.

Run: `cargo test --workspace --all-features --locked`
Expected (Windows): 300 test geçti, 0 başarısız, 4 canlı test atlandı.

- [ ] **Step 6: Commit**

```bash
git add crates/kalfa
git commit -m "kalfa: Tauri kabuğu, pencereler, komutlar ve pencere izinleri"
git push
```

---

### Task 4: Son doğrulama ve durum dosyası

**Files:**
- Modify: `docs/GELISTIRME_DURUMU.md`

- [ ] **Step 1: Bütün kontrolleri koştur**

Run (depo kökünden):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo tree -p kalfa-uygulama -e normal | grep -E "tauri|wry|webkit"
cargo tree -p kalfa-core -e normal | grep -E "kalfa-tools|tauri"
git grep -n -E "\.unwrap\(\)|\.expect\(" -- crates/kalfa-uygulama/src crates/kalfa/src
```

Expected:
- fmt ve clippy temiz; testlerde (Windows) 300 geçti, 4 atlandı.
- İki `cargo tree` satırı boş.
- `git grep` yalnızca `crates/kalfa-uygulama/src/test_destek.rs`'yi gösterir.

- [ ] **Step 2: Durum dosyasına 1c-2 bölümünü ekle**

`docs/GELISTIRME_DURUMU.md`'nin sonuna, 1c-3a bölümünden sonra ekle. Sayıları kendi çıktılarına göre yaz; Linux sütununu inceleyici doldurur.

```markdown
### 1c-2 — Tauri kabuğu (tamamlandı)

Plan: [`2026-09-23-kalfa-1c-2-tauri-kabugu.md`](superpowers/plans/2026-09-23-kalfa-1c-2-tauri-kabugu.md).
`kalfa-uygulama` (Tauri'siz uygulama mantığı: komut sözleşmesi, şart akışı,
oturum tablosu, olay köprüsü, kimlik deposu) ve `kalfa` (Tauri kabuğu: iki
pencere, 22 komut, pencere izinleri, yerel iletişim kutuları) eklendi. İki
pencerenin ekranları 1c-3b'de.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `cargo test --workspace --all-features --locked` | … geçti, 0 başarısız, 4 yok sayıldı | (inceleme) |
| Clippy / fmt | … | (inceleme) |
| `cargo tree -p kalfa-uygulama` içinde `tauri`/`wry`/`webkit` | … | (inceleme) |
| `unwrap()` / `expect()` taraması | … | (inceleme) |
```

- [ ] **Step 3: Commit**

```bash
git add docs/GELISTIRME_DURUMU.md
git commit -m "docs: 1c-2 Tauri kabuğu tamamlandı; doğrulama kaydı"
git push
```

---

## İnceleme (Claude)

Codex'in işi bitince inceleyici şunları yapar:

1. **Plana uygunluk:** kodu plan metniyle karşılaştırır; planda olmayan satırları ayrıca okur.
2. **Windows:** fmt, clippy ve `cargo test --workspace --all-features --locked`.
3. **Linux (WSL2, Ubuntu 24.04):** `kalfa` dışındaki crate'lerde clippy ve testler; `cargo tree -p kalfa-uygulama` denetimi. WebKitGTK paketleri kuruluysa `kalfa` crate'i de.
4. **Gerçek pencere (Windows):** uygulamayı sahte EVREN sunucusuyla açar ve pencerenin içinden komutları dener. Gerçek EVREN'e hiçbir istek gitmez.
5. **Durum dosyası:** Linux sütununu doldurur, PR'a yorum yazar; temizse birleştirir.

**Gerçek pencere denemesi.** Sahte EVREN (`node sahte-evren.mjs`, `127.0.0.1:8765`; yalnızca şart uçları):

```js
// Yerel sahte EVREN: yalnızca şart uçları. Gerçek EVREN'e hiçbir istek gitmez.
import { createServer } from 'node:http';

let kabul = false;
const gunluk = (...a) => console.log(new Date().toISOString(), ...a);

createServer((istek, yanit) => {
  let govde = '';
  istek.on('data', (p) => (govde += p));
  istek.on('end', () => {
    gunluk(istek.method, istek.url, govde);
    const json = (kod, veri) => {
      yanit.writeHead(kod, { 'content-type': 'application/json' });
      yanit.end(JSON.stringify(veri));
    };
    if (istek.headers.authorization !== 'Bearer evren_llm_sahte') {
      return json(401, { error: { message: 'invalid key', type: 'auth', code: 'invalid_api_key' } });
    }
    if (istek.method === 'GET' && istek.url === '/v1/terms/status') {
      return json(200, { current_version: 1, is_material: true, accepted: kabul });
    }
    if (istek.method === 'GET' && istek.url === '/v1/terms/text') {
      return json(200, { version: 1, content: 'Deneme şartları.' });
    }
    if (istek.method === 'POST' && istek.url === '/v1/terms/accept') {
      kabul = true;
      return json(200, { accepted: true });
    }
    json(404, { error: { message: 'yok', type: 'not_found' } });
  });
}).listen(8765, '127.0.0.1', () => gunluk('sahte EVREN 8765'));
```

WebView2'nin hata ayıklama portundan sayfada ifade değerlendiren betik (`node cdp.mjs <adres-parçası> <ifade>`, hedef listesi için `node cdp.mjs --liste`):

```js
// WebView2 sayfasında ifade değerlendirir: node cdp.mjs <url-parçası> <ifade>
// Hedef listesi için: node cdp.mjs --liste
const port = 9223;
const hedefler = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
if (process.argv[2] === '--liste') {
  for (const h of hedefler) console.log(h.type, h.url, h.title);
  process.exit(0);
}
const [parca, ifade] = process.argv.slice(2);
const hedef = hedefler.find((h) => h.type === 'page' && h.url.includes(parca));
if (!hedef) {
  console.error('hedef yok:', parca, hedefler.map((h) => h.url));
  process.exit(1);
}
const ws = new WebSocket(hedef.webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener('open', r, { once: true }));
ws.send(
  JSON.stringify({
    id: 1,
    method: 'Runtime.evaluate',
    params: { expression: ifade, awaitPromise: true, returnByValue: true },
  }),
);
const yanit = await new Promise((r) =>
  ws.addEventListener('message', (e) => {
    const m = JSON.parse(e.data);
    if (m.id === 1) r(m);
  }),
);
const s = yanit.result?.result;
console.log(JSON.stringify(s?.value ?? s ?? yanit, null, 1));
ws.close();
```

Uygulamayı başlatmak için önce `ui/` içinde `npm ci` çalıştırılır ve Vite geliştirme sunucusu (`npm run dev`, 5173) açılır. Sonra depo kökünden:

```bash
KALFA_ANAHTAR_DEPOSU=bellek KALFA_VERI_DIZINI=<geçici dizin> \
KALFA_EVREN_TABAN=http://127.0.0.1:8765/v1 \
WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223 \
cargo run -p kalfa
```

Beklenenler (plan yazılırken görüldü):

| Adım | Sonuç |
| --- | --- |
| Açılış (anahtar yok) | Yalnızca şart penceresi açılır; `sart_penceresi_durumu` → `AnahtarGerekli { neden: "Yok" }` |
| Şart penceresinden `uygulama_durumu` | `… not allowed on window "sartlar"` |
| `plugin:webview\|create_webview_window`, `plugin:dialog\|open` | `not allowed` |
| `window.open('https://example.com')` | `null` |
| `baglanti_ac('javascript:alert(1)')` | "Yalnızca http ve https adresleri açılabilir." |
| Kapı boştayken `sartlari_kabul_et(1)` | "Onay bekleyen şart metni yok."; sunucuya kabul gitmez |
| `anahtari_kaydet('evren_llm_yanlis')` | `Gecersiz` (sahte sunucu 401 döner) |
| `anahtari_kaydet(' evren_llm_sahte ')` | `Kaydedildi { depo: "Bellekte", sonraki: SartGerekli { metin: { surum: 1 } } }`; sunucuya kabul gitmez |
| `sartlari_kabul_et(1)` | `Kabul`; sunucu günlüğünde tek `POST /v1/terms/accept {"version":1}`; ana pencere açılır, şart penceresi kapanır |
| Ana pencereden `sartlari_kabul_et` | `… not allowed on window "ana"` |
| Ana pencerede `location.href = 'https://example.com'` | Sayfa `http://localhost:5173/`'te kalır |
| `oturum_ac` (kaydı olmayan kimlik) | `OturumYok`, "Bu oturumun kaydı bulunamadı." (canlı denemede ham işletim sistemi hatası görüldü, düzeltildi; `kaydi_olmayan_oturum_acilmaz` testi sınar) |
