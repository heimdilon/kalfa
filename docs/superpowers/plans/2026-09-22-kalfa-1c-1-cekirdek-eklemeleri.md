# Kalfa 1c-1 — Çekirdek Eklemeleri (Tauri'siz) Uygulama Planı

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Masaüstü kabuğundan (1c-2) önce gereken, arayüzden bağımsız çekirdek parçalarını `kalfa-core`'a eklemek: olay paketleyici ve kredi, tek şart kapısı, çalışma alanı kilit yöneticisi, kayıt başına tek yazıcı, kayıttan geçmiş görünümü ve oturum listesi.

**Architecture:** Her parça Tauri'ye bağlı olmayan, `cargo test` ile sınanan bir `kalfa-core` modülüdür. Paketleyici (`aktarim`) oturumun olay kuyruğunu okur, olayları 16 ms'lik pencerelerde paketler ve arayüzün "işledim" demediği veri krediyi doldurunca kuyruktan okumayı bırakır. Şart kapısı (`sart_kapisi`) uygulama genelinde tektir; oturumlar 403 alınca ona kaydolur, kabul EVREN'e bir kez gider. Kilit yöneticisi kök dizini dosya sistemi kimliğiyle tanır; kayıt kilidi kaydın yanındaki bir kilit dosyasında tutulur. `gecmis` modülü kayıttan ekran öğeleri, sayfa, durum özeti ve oturum listesi üretir.

**Tech Stack:** Rust 1.98.1 · tokio (sync, time; testlerde `test-util`) · serde / serde_json · same-file · kalfa-evren (1a) · kalfa-core (1b)

**Spec:** `docs/superpowers/specs/2026-09-22-kalfa-1c-design.md` (1c tasarımı) — §3.3, §4, §5, §7.3, §9.3, §9.4, §11.1. Üst belge: `docs/superpowers/specs/2026-09-21-kalfa-design.md` — §4.1, §6.1, §6.6, §11.1, §11.8.

**Ön koşul:** 1a ve 1b tamam. Bu planın temel aldığı olay kuyruğu değişikliği Görev 1'de commit'lendi (`d02c22b`).

**Bu planın kapsamı dışında:** `kalfa` Tauri crate'i, pencereler, komut yetkileri, kimlik deposu (1c-2) · `ui/` Svelte arayüzü (1c-3) · gerçek pencereyle uçtan uca testler ve p95 ölçümü (1c-4).

**Kod örnekleri hakkında:** Örnekler mevcut kodun tiplerine göre yazıldı ama bu plan yazılırken derlenmedi. Uygulayıcı derleme hatasını düzeltirken testin sınadığı davranışı değiştirmez; davranış değişmesi gerekiyorsa durur ve sorar.

## PR ile yürütme

Kullanıcı kararı (2026-09-23): kodu Codex yazar, Claude inceler.

- **Dal ve PR:** çalışma dalı `1c-1/cekirdek`, PR'ın hedefi `kalfa-asama-1`. Uygulayıcı bu dalda çalışır. Ayrı bir PR açması gerekirse hedef dal olarak `1c-1/cekirdek`'i seçer.
- **Sıra:** görevler sırayla uygulanır; Görev 1 yapıldı, iş **Görev 2'den** başlar. Her görev planda yazan mesajla kendi commit'ini alır; her commit'ten sonra push'lanır. Geçmiş yeniden yazılmaz, force-push yapılmaz.
- **Doğrulama:** her görevin sonundaki komutlar uygulayıcının ortamında koşturulur. Kayıt kilidi ve kök kimliği platforma göre değiştiği için öbür platformu (Windows ya da Linux) inceleyici PR'ı birleştirmeden önce koşturur.
- **Soru ve sapma:** plandan sapmak gerekirse (davranış değişikliği, yeni bağımlılık, plandaki bir testin yanlış olması) uygulayıcı durur ve PR'a yorum yazar; kendi başına karar vermez.
- **İnceleme:** inceleyici PR'a yorum yazar. Düzeltmeler yeni commit'lerle gelir. İnceleme temizse ve iki platform yeşilse inceleyici PR'ı birleştirir.
- **Ortam:** Rust `rust-toolchain.toml`'dan kurulur (1.98.1, clippy ve rustfmt ile). 1c-1 Tauri'siz olduğu için başka sistem paketi gerekmez. `.env` yoktur ve gerekmez; canlı EVREN testleri (`#[ignore]`) koşturulmaz.

## Global Constraints

- **Araç zinciri:** Rust `1.98.1`, edition `2024` (workspace `rust-toolchain.toml`).
- **Bağımlılık yönü:** `kalfa-core` asla `kalfa-tools`'a, `tauri`'ye ya da bir UI kütüphanesine bağlanmaz. Yeni normal bağımlılık yalnızca `same-file` (workspace'te zaten dolaylı bağımlılık, `1.0.6`). Yeni dev bağımlılık yalnızca tokio'nun `test-util` özelliği.
- **Paketleyici başlangıç değerleri:** pencere `16 ms`, paket `64 KiB`, kredi `4` paket ya da `256 KiB`. Çekirdek olay bütçesi varsayılanı değişmez: `16.384` olay, `8 MiB`.
- **Kritik olaylar:** `OnayGerekiyor`, `Hata`, `Tamamlandi`, `AracCagrisiBasladi`, `AracSonucu`, `SartOnayiGerekiyor`, `AktarimDurdu` birleştirilmez, düşürülmez, pencereyi beklemeden gider.
- **Kayıt kilidi:** kaydın yanında `<kayıt>.kilit` dosyası; `std::fs::File::try_lock`. Kilit kaydın kendisine konmaz.
- **Oturum listesi:** baştan `16 KiB`, sondan `64 KiB` okunur; başlık en çok `80` karakter, kısaltma işareti `···`.
- **Şart kabulü:** EVREN'e yalnızca `SartKapisi::kabul_et` gönderir, kabul başına bir kez. Canlı EVREN'e karşı kabul hiçbir testte denenmez.
- **Kod kuralı:** kütüphane kodunda `unwrap()` / `expect()` yok; `test_destek` ve testler serbest.
- **Metin tonu:** kullanıcıya giden metinler "siz" diliyle; modele giden araç hataları "sen" kalır.
- **Her görevin sonunda:** `cargo test --workspace --quiet` yeşil; `cargo clippy --workspace --all-targets -- -D warnings` temiz; `cargo fmt --all -- --check` temiz. Canlı EVREN testleri (`#[ignore]`) koşturulmaz.
- **Commit:** mesajlar Türkçe, `kalfa-core: …` biçiminde. Kullanıcı kararı (2026-09-22): yalnızca Claude'un yazdığı commit'lerin sonuna `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` eklenir. Codex'in yazdığı commit'lere ve Görev 1'in commit'lerine bu satır eklenmez.
- **Dokunulmayanlar:** `.env`, `Screenshot*.png`, kök dizindeki `tasks.png`.

## Dosya Yapısı

```
Cargo.toml                                    same-file workspace bağımlılığı (değişir)
crates/kalfa-core/Cargo.toml                  same-file; dev: tokio test-util (değişir)
crates/kalfa-core/src/lib.rs                  aktarim, gecmis, sart_kapisi modülleri (değişir)
crates/kalfa-core/src/aktarim.rs              paketleyici, kredi, kanal mesajı (yeni)
crates/kalfa-core/src/sart_kapisi.rs          uygulama genelinde tek şart kapısı (yeni)
crates/kalfa-core/src/gecmis/mod.rs           kayıttan görünüm, sayfa, durum özeti (yeni)
crates/kalfa-core/src/gecmis/liste.rs         kayıt dizininden oturum listesi (yeni)
crates/kalfa-core/src/kilit.rs                KilitYoneticisi; yol tablosu temizliği (değişir)
crates/kalfa-core/src/kayit.rs                kayıt kilidi; yeni kayıt öğeleri (değişir)
crates/kalfa-core/src/kurtarma.rs             kurtarılan durumda çalışma alanı (değişir)
crates/kalfa-core/src/protokol.rs             AktarimDurdu, IstekBilgisi; SartlariKabulEt kalkar (değişir)
crates/kalfa-core/src/saglayici.rs            sart_durumu (değişir)
crates/kalfa-core/src/oturum/mod.rs           terminal olay, şart kapısı, kapanış bekleme, çalışma alanı, metin tonu (değişir)
crates/kalfa-core/src/oturum/model_turu.rs    şart akışı kapıya devredilir; istek bilgisi, kullanım kaydı (değişir)
crates/kalfa-core/src/test_destek.rs          şart durumu, yönlendirilen model, bağlamda kapı, Surucu::kapat (değişir)
crates/kalfa-core/tests/aktarim.rs            paketleme ve kredi (yeni)
crates/kalfa-core/tests/aktarim_oturum.rs     gerçek oturumla paketleyici (yeni)
crates/kalfa-core/tests/sart_kapisi.rs        kapının kendisi (yeni)
crates/kalfa-core/tests/kilit_yoneticisi.rs   kök kimliği ve yol tablosu (yeni)
crates/kalfa-core/tests/kayit_kilidi.rs       tek yazıcı (yeni)
crates/kalfa-core/tests/kayit_ayrintilari.rs  çalışma alanı, yönlendirilen model, kullanım (yeni)
crates/kalfa-core/tests/gecmis.rs             görünüm, sayfa, durum özeti (yeni)
crates/kalfa-core/tests/oturum_listesi.rs     oturum listesi (yeni)
crates/kalfa-core/tests/oturum_sart.rs        kapıyla şart akışı (baştan yazılır)
crates/kalfa-core/tests/{oturum_olay_aktarimi,oturum_metin,oturum_kurtarma,kayit_kurtarma,protokol,oturum_iptal_sinir}.rs  (değişir)
crates/kalfa-evren/src/hata.rs                metin tonu (değişir)
crates/kalfa-evren/tests/hata_kredi.rs        metin tonu testi (değişir)
crates/kalfa-tools/tests/uctan_uca.rs         bağlamda şart kapısı (değişir)
docs/GELISTIRME_DURUMU.md                     1c-1 sonuçları (değişir)
```

---

### Task 1: Olay kuyruğu değişikliğini doğrula ve commit'le

> **Durum: yapıldı (2026-09-23), PR açılmadan önce.** Commit'ler `d02c22b` (kod) ve `15bb90e` (durum dosyası). Windows'ta `cargo test --workspace` 182 geçti, 4 canlı test atlandı; Clippy ve biçim temiz. Uygulayıcı **Görev 2'den** başlar.

Çalışma ağacında, başka bir araçla yapılmış ve henüz commit'lenmemiş bir değişiklik var: çekirdek olay kuyruğunu sayı ve bayt bütçesiyle sınırlayan iş. Bu plan onun üzerine kurulur. Kod bu planın yürütücüsünce yazılmadığı için commit **kullanıcının açık onayıyla** yapılır.

**Files:**
- Commit (değiştirilmez): `crates/kalfa-core/src/oturum/mod.rs`, `crates/kalfa-core/tests/oturum_olay_aktarimi.rs`, `docs/superpowers/plans/2026-09-22-kalfa-1c-olay-aktarimi.md`, `README.md`, `docs/GELISTIRME_DURUMU.md`

**Interfaces:**
- Produces: `kalfa_core::oturum::{OlayAlici, OlayButcesi, OlayAktarimHatasi, OlayButcesiHatasi, TERMINAL_OLAY_BAYT_REZERVI}`; `OturumAyarlari::olay_butcesi`; `OlayAlici::{recv, try_recv, transport_hatasi, bekleyen_olay_sayisi, bekleyen_bayt, close}`.

- [x] **Step 1: Çalışma ağacını gör**

Run: `git status --short && git diff --stat`
Expected: yukarıdaki beş dosya değişmiş ya da yeni; başka kod dosyası yok. Farklıysa dur ve kullanıcıya sor.

- [x] **Step 2: Doğrulamayı koştur**

Run: `cargo test --workspace --quiet && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: testler geçer (değişikliğin kendi kaydı 182 geçti, 4 atlandı diyor), Clippy ve biçim temiz.

- [x] **Step 3: Farkı gözden geçir**

Run: `git diff crates/kalfa-core/src/oturum/mod.rs`
Kontrol: `OlayKanali::gonder` kuyruğa kabulden önce sayı ve bayt sınırını denetliyor; sınır aşılınca `basarisiz` hata durumunu kuruyor; `Oturum::yay` hata dönünce `iptal.cancel()` ve `kapandi = true` yapıyor; `OlayAlici` bırakılınca aktif tur iptal ediliyor.

- [x] **Step 4: Kontrol noktası — kullanıcı onayı**

Kullanıcı 2026-09-22'de onay verdi ("Evet, commit'le"). Adım 1–3 beklenmedik bir şey gösterirse (başka dosya, kırmızı test) commit'lenmez; kullanıcıya yeniden sorulur.

- [x] **Step 5: Commit'le**

```bash
git add crates/kalfa-core/src/oturum/mod.rs crates/kalfa-core/tests/oturum_olay_aktarimi.rs docs/superpowers/plans/2026-09-22-kalfa-1c-olay-aktarimi.md README.md
git commit -m "kalfa-core: olay kuyruğu sayı ve bayt bütçesiyle sınırlı (1c ilk adım)"
git add docs/GELISTIRME_DURUMU.md
git commit -m "docs: 1c kararları ve olay kuyruğu doğrulaması"
```

---

### Task 2: Terminal aktarım olayı (`Olay::AktarimDurdu`)

Bugün kuyruk durunca son olay teknik bir `Hata` metni ("Olay aktarımı durduruldu: olay sayısı bütçesi aşıldı (…)"). Arayüzün ne olduğunu ve ne yapılacağını söyleyebilmesi için bu olay nedenini taşıyan ayrı bir olay türü olur (1c tasarımı §4.4).

**Files:**
- Modify: `crates/kalfa-core/src/protokol.rs`
- Modify: `crates/kalfa-core/src/oturum/mod.rs`
- Modify: `crates/kalfa-core/tests/oturum_olay_aktarimi.rs`
- Test: `crates/kalfa-core/tests/protokol.rs`

**Interfaces:**
- Produces: `protokol::AktarimDurmaNedeni { TuketiciYetismedi, OlayCokBuyuk, Diger }` (`Copy`, serde); `Olay::AktarimDurdu { neden: AktarimDurmaNedeni }`; `OlayAktarimHatasi::durma_nedeni(&self) -> AktarimDurmaNedeni`.

- [ ] **Step 1: Başarısız testi yaz**

`crates/kalfa-core/tests/protokol.rs` içinde `butun_olaylar_gidip_gelir` listesine, `Olay::Tamamlandi { neden: BitisNedeni::Normal },` satırından sonra ekle:

```rust
        Olay::AktarimDurdu {
            neden: AktarimDurmaNedeni::OlayCokBuyuk,
        },
```

Dosyanın sonuna ekle:

```rust
#[test]
fn aktarim_durdu_json_bicimi() {
    assert_eq!(
        serde_json::to_value(Olay::AktarimDurdu {
            neden: AktarimDurmaNedeni::TuketiciYetismedi
        })
        .unwrap(),
        json!({"tip": "AktarimDurdu", "neden": "TuketiciYetismedi"})
    );
}
```

- [ ] **Step 2: Testin başarısız olduğunu gör**

Run: `cargo test -p kalfa-core --test protokol`
Expected: FAIL — `AktarimDurdu` ve `AktarimDurmaNedeni` tanımlı değil.

- [ ] **Step 3: Olayı ve nedeni ekle**

`crates/kalfa-core/src/protokol.rs` içinde `BitisNedeni` enum'undan sonra ekle:

```rust
/// Olay aktarımının neden durduğu (1c tasarımı §4.4). Arayüz gösterilecek
/// metni buna göre seçer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AktarimDurmaNedeni {
    /// Tüketici kuyruğu boşaltamadı: olay ya da bayt bütçesi doldu.
    TuketiciYetismedi,
    /// Tek bir olay bütçeden büyüktü.
    OlayCokBuyuk,
    /// Olay serileştirilemedi ya da alıcı kapandı.
    Diger,
}
```

`Olay` enum'unda `Tamamlandi { neden: BitisNedeni },` varyantından sonra ekle:

```rust
    /// Olay kuyruğu durdu ve oturum kapanıyor; kuyruğun son olayıdır.
    AktarimDurdu {
        neden: AktarimDurmaNedeni,
    },
```

- [ ] **Step 4: Çekirdek terminal olayı yeni türle yaysın**

`crates/kalfa-core/src/oturum/mod.rs` içinde protokol içe aktarımını değiştir:

```rust
use crate::protokol::{
    AktarimDurmaNedeni, AracDurumu, BitisNedeni, Komut, Olay, SEMA_SURUMU, Zarf,
};
```

`OlayAktarimHatasi` enum'undan hemen sonra ekle:

```rust
impl OlayAktarimHatasi {
    /// Arayüze bildirilen neden.
    pub fn durma_nedeni(&self) -> AktarimDurmaNedeni {
        match self {
            Self::OlaySayisiAsildi { .. } => AktarimDurmaNedeni::TuketiciYetismedi,
            Self::BaytButcesiAsildi { olay, azami, .. } if olay > azami => {
                AktarimDurmaNedeni::OlayCokBuyuk
            }
            Self::BaytButcesiAsildi { .. } => AktarimDurmaNedeni::TuketiciYetismedi,
            Self::AliciKapandi | Self::Serilestirme => AktarimDurmaNedeni::Diger,
        }
    }
}
```

`Oturum::yay` içindeki terminal zarfın yükünü değiştir. Eski:

```rust
                    yuk: Olay::Hata {
                        mesaj: format!("Olay aktarımı durduruldu: {hata}."),
                    },
```

Yeni:

```rust
                    yuk: Olay::AktarimDurdu {
                        neden: hata.durma_nedeni(),
                    },
```

Aynı dosyanın `#[cfg(test)] mod tests` bloğundaki `terminal_hata_rezervi_uuid_ve_mesajla_bile_yeterlidir` testini tümüyle şu iki testle değiştir:

```rust
    #[test]
    fn terminal_olay_rezerve_sigar() {
        let zarf = Zarf {
            sema: SEMA_SURUMU,
            oturum: OturumId::yeni(),
            tur: Some(TurId::yeni()),
            sira: u64::MAX,
            yuk: Olay::AktarimDurdu {
                neden: AktarimDurmaNedeni::TuketiciYetismedi,
            },
        };
        assert!(serilestirilmis_bayt(&zarf, TERMINAL_OLAY_BAYT_REZERVI).is_ok());
    }

    #[test]
    fn durma_nedeni_eslemesi() {
        assert_eq!(
            OlayAktarimHatasi::OlaySayisiAsildi {
                bekleyen: 3,
                azami: 3
            }
            .durma_nedeni(),
            AktarimDurmaNedeni::TuketiciYetismedi
        );
        assert_eq!(
            OlayAktarimHatasi::BaytButcesiAsildi {
                bekleyen: 0,
                olay: 11,
                azami: 10
            }
            .durma_nedeni(),
            AktarimDurmaNedeni::OlayCokBuyuk
        );
        assert_eq!(
            OlayAktarimHatasi::BaytButcesiAsildi {
                bekleyen: 8,
                olay: 5,
                azami: 10
            }
            .durma_nedeni(),
            AktarimDurmaNedeni::TuketiciYetismedi
        );
        assert_eq!(
            OlayAktarimHatasi::Serilestirme.durma_nedeni(),
            AktarimDurmaNedeni::Diger
        );
    }
```

- [ ] **Step 5: Olay aktarımı testlerini yeni olaya uyarla**

`crates/kalfa-core/tests/oturum_olay_aktarimi.rs` içinde içe aktarımı değiştir:

```rust
use kalfa_core::protokol::{AktarimDurmaNedeni, Komut, Olay, OnayKarari};
```

`yavas_alici_olay_sayisi_butcesini_asamaz` içindeki eski koşulu:

```rust
        if matches!(zarf.yuk, Olay::Hata { ref mesaj } if mesaj.contains("Olay aktarımı durduruldu"))
        {
            terminal_goruldu = true;
        }
```

şununla değiştir:

```rust
        if matches!(
            zarf.yuk,
            Olay::AktarimDurdu {
                neden: AktarimDurmaNedeni::TuketiciYetismedi
            }
        ) {
            terminal_goruldu = true;
        }
```

`buyuk_onay_olayi_yazma_baslamadan_turu_kapatir` içindeki eski koşulu:

```rust
        if matches!(zarf.yuk, Olay::Hata { mesaj } if mesaj.contains("Olay aktarımı durduruldu"))
        {
            terminal_goruldu = true;
            break;
        }
```

şununla değiştir:

```rust
        if matches!(zarf.yuk, Olay::AktarimDurdu { .. }) {
            terminal_goruldu = true;
            break;
        }
```

- [ ] **Step 6: Testleri koştur**

Run: `cargo test -p kalfa-core`
Expected: PASS (protokol, olay aktarımı ve birim testleri dahil).

- [ ] **Step 7: Commit'le**

```bash
git add crates/kalfa-core/src/protokol.rs crates/kalfa-core/src/oturum/mod.rs crates/kalfa-core/tests/oturum_olay_aktarimi.rs crates/kalfa-core/tests/protokol.rs
git commit -m "kalfa-core: aktarım durunca nedeni taşıyan AktarimDurdu olayı"
```

---

### Task 3: `aktarim` — paketleyici ve kredi

Olay kuyruğunu okuyup paketleyen ve krediyle sınırlayan birim (1c tasarımı §4). Paketleyici bir olay kaynağı üzerinden genel yazılır: uygulamada kaynak `OlayAlici`, testlerde zamanı denetlenebilen bir kanal.

**Files:**
- Create: `crates/kalfa-core/src/aktarim.rs`
- Modify: `crates/kalfa-core/src/lib.rs`
- Modify: `crates/kalfa-core/Cargo.toml`
- Test: `crates/kalfa-core/tests/aktarim.rs`
- Test: `crates/kalfa-core/tests/aktarim_oturum.rs`

**Interfaces:**
- Consumes: `oturum::OlayAlici::recv`, `protokol::{Olay, Zarf}`, Task 2'nin `Olay::AktarimDurdu`.
- Produces:
  - `aktarim::AktarimAyarlari { pencere: Duration, azami_paket_bayt: usize, kredi_paket: usize, kredi_bayt: usize }` (`Default`: 16 ms, 64 KiB, 4, 256 KiB)
  - `aktarim::KanalMesaji { Olaylar { no: u64, zarflar: Vec<Zarf<Olay>> }, Kapandi { no: u64 } }` (serde `tag = "tip"`), `KanalMesaji::no(&self) -> u64`
  - `aktarim::OlayKaynagi` trait: `fn recv(&mut self) -> impl Future<Output = Option<Zarf<Olay>>> + Send` (`OlayAlici` için uygulanmış)
  - `aktarim::Paketleyici<K: OlayKaynagi>`: `yeni(kaynak: K, ayarlar: AktarimAyarlari) -> (Self, KrediTutamaci)`, `async fn sonraki(&mut self) -> Option<KanalMesaji>`
  - `aktarim::KrediTutamaci` (`Clone`): `isledim(&self, no: u64) -> Result<(), IsledimHatasi>`, `yolda(&self) -> (usize, usize)`
  - `aktarim::IsledimHatasi::GonderilmemisPaket { no: u64, son: u64 }`
  - `aktarim::kritik_mi(&Olay) -> bool`

- [ ] **Step 1: tokio test-util'i ekle**

`crates/kalfa-core/Cargo.toml` içindeki `[dev-dependencies]` bölümüne ekle:

```toml
tokio = { workspace = true, features = ["test-util"] }
```

- [ ] **Step 2: Paketleme ve kredi testlerini yaz**

`crates/kalfa-core/tests/aktarim.rs`:

```rust
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use kalfa_core::aktarim::{
    AktarimAyarlari, IsledimHatasi, KanalMesaji, KrediTutamaci, OlayKaynagi, Paketleyici,
};
use kalfa_core::kimlik::{OturumId, TurId};
use kalfa_core::protokol::{BitisNedeni, Olay, SEMA_SURUMU, Zarf};
use tokio::sync::mpsc;
use tokio::time::Instant;

/// Kaç olay okunduğunu sayan test kaynağı.
struct TestKaynagi {
    rx: mpsc::UnboundedReceiver<Zarf<Olay>>,
    okunan: Arc<AtomicUsize>,
}

impl OlayKaynagi for TestKaynagi {
    fn recv(&mut self) -> impl Future<Output = Option<Zarf<Olay>>> + Send {
        let okunan = self.okunan.clone();
        let rx = &mut self.rx;
        async move {
            let zarf = rx.recv().await;
            if zarf.is_some() {
                okunan.fetch_add(1, Ordering::SeqCst);
            }
            zarf
        }
    }
}

struct Uretici {
    tx: mpsc::UnboundedSender<Zarf<Olay>>,
    oturum: OturumId,
    tur: TurId,
    sira: u64,
}

impl Uretici {
    fn gonder(&mut self, yuk: Olay) {
        self.sira += 1;
        self.tx
            .send(Zarf {
                sema: SEMA_SURUMU,
                oturum: self.oturum,
                tur: Some(self.tur),
                sira: self.sira,
                yuk,
            })
            .unwrap();
    }

    fn metin(&mut self, s: &str) {
        self.gonder(Olay::TokenParcasi { metin: s.into() });
    }

    fn dusunce(&mut self, s: &str) {
        self.gonder(Olay::DusunceParcasi { metin: s.into() });
    }

    fn hata(&mut self, s: &str) {
        self.gonder(Olay::Hata { mesaj: s.into() });
    }
}

fn kur(
    ayarlar: AktarimAyarlari,
) -> (
    Uretici,
    Paketleyici<TestKaynagi>,
    KrediTutamaci,
    Arc<AtomicUsize>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    let okunan = Arc::new(AtomicUsize::new(0));
    let kaynak = TestKaynagi {
        rx,
        okunan: okunan.clone(),
    };
    let (paketleyici, kredi) = Paketleyici::yeni(kaynak, ayarlar);
    let uretici = Uretici {
        tx,
        oturum: OturumId::yeni(),
        tur: TurId::yeni(),
        sira: 0,
    };
    (uretici, paketleyici, kredi, okunan)
}

fn olaylar(m: &KanalMesaji) -> Vec<Olay> {
    match m {
        KanalMesaji::Olaylar { zarflar, .. } => zarflar.iter().map(|z| z.yuk.clone()).collect(),
        KanalMesaji::Kapandi { .. } => Vec::new(),
    }
}

fn metin(s: &str) -> Olay {
    Olay::TokenParcasi { metin: s.into() }
}

#[tokio::test(start_paused = true)]
async fn pencere_icindeki_bitisik_metin_parcalari_birlesir() {
    let (mut u, mut p, _kredi, _) = kur(AktarimAyarlari::default());
    u.metin("Mer");
    u.metin("ha");
    u.metin("ba");
    let m = p.sonraki().await.unwrap();
    assert_eq!(m.no(), 1);
    assert_eq!(olaylar(&m), vec![metin("Merhaba")]);
    let KanalMesaji::Olaylar { zarflar, .. } = &m else {
        unreachable!()
    };
    assert_eq!(zarflar[0].sira, 3, "birleşen parça son parçanın sırasını taşır");
}

#[tokio::test(start_paused = true)]
async fn pencereden_sonra_gelen_parca_sonraki_pakete_kalir() {
    let (mut u, mut p, kredi, _) = kur(AktarimAyarlari::default());
    u.metin("a");
    let uretici = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        u.metin("b");
        tokio::time::sleep(Duration::from_millis(20)).await;
        u.metin("c");
        u
    });
    let bir = p.sonraki().await.unwrap();
    assert_eq!(olaylar(&bir), vec![metin("ab")]);
    kredi.isledim(bir.no()).unwrap();
    let iki = p.sonraki().await.unwrap();
    assert_eq!(olaylar(&iki), vec![metin("c")]);
    drop(uretici.await.unwrap());
}

#[tokio::test(start_paused = true)]
async fn farkli_turden_olay_birlestirmeyi_keser() {
    let (mut u, mut p, _kredi, _) = kur(AktarimAyarlari::default());
    u.metin("a");
    u.dusunce("d");
    u.metin("b");
    let m = p.sonraki().await.unwrap();
    assert_eq!(
        olaylar(&m),
        vec![
            metin("a"),
            Olay::DusunceParcasi { metin: "d".into() },
            metin("b")
        ]
    );
}

#[tokio::test(start_paused = true)]
async fn kritik_olay_pencereyi_beklemeden_paketi_kapatir() {
    let (mut u, mut p, kredi, _) = kur(AktarimAyarlari::default());
    u.metin("a");
    u.gonder(Olay::Tamamlandi {
        neden: BitisNedeni::Normal,
    });
    u.metin("b");
    let bas = Instant::now();
    let bir = p.sonraki().await.unwrap();
    assert_eq!(bas.elapsed(), Duration::ZERO, "kritik olay pencereyi beklemez");
    assert_eq!(
        olaylar(&bir),
        vec![
            metin("a"),
            Olay::Tamamlandi {
                neden: BitisNedeni::Normal
            }
        ]
    );
    kredi.isledim(bir.no()).unwrap();
    let iki = p.sonraki().await.unwrap();
    assert_eq!(olaylar(&iki), vec![metin("b")]);
}

#[tokio::test(start_paused = true)]
async fn ilk_olay_kritikse_tek_basina_hemen_gider() {
    let (mut u, mut p, _kredi, _) = kur(AktarimAyarlari::default());
    u.hata("bir");
    u.hata("iki");
    let bas = Instant::now();
    let m = p.sonraki().await.unwrap();
    assert_eq!(bas.elapsed(), Duration::ZERO);
    assert_eq!(
        olaylar(&m),
        vec![Olay::Hata {
            mesaj: "bir".into()
        }],
        "kritik olaylar birleştirilmez"
    );
}

#[tokio::test(start_paused = true)]
async fn paket_bayt_siniri_asilmaz_ve_olaylar_sirayla_gelir() {
    let ayar = AktarimAyarlari {
        azami_paket_bayt: 400,
        ..AktarimAyarlari::default()
    };
    let (mut u, mut p, kredi, _) = kur(ayar);
    for i in 0..10 {
        if i % 2 == 0 {
            u.metin(&format!("m{i}"));
        } else {
            u.dusunce(&format!("d{i}"));
        }
    }
    drop(u);
    let mut gelen = Vec::new();
    while let Some(m) = p.sonraki().await {
        kredi.isledim(m.no()).unwrap();
        if let KanalMesaji::Olaylar { zarflar, .. } = m {
            let toplam: usize = zarflar
                .iter()
                .map(|z| serde_json::to_vec(z).unwrap().len())
                .sum();
            assert!(zarflar.len() == 1 || toplam <= 400, "paket {toplam} bayt");
            gelen.extend(zarflar.into_iter().map(|z| z.sira));
        }
    }
    assert_eq!(gelen, (1..=10).collect::<Vec<u64>>());
}

#[tokio::test(start_paused = true)]
async fn siniri_asan_tek_olay_kendi_paketinde_gider() {
    let ayar = AktarimAyarlari {
        azami_paket_bayt: 100,
        ..AktarimAyarlari::default()
    };
    let (mut u, mut p, _kredi, _) = kur(ayar);
    let uzun = "x".repeat(500);
    u.metin(&uzun);
    u.dusunce("d");
    let m = p.sonraki().await.unwrap();
    assert_eq!(olaylar(&m), vec![metin(&uzun)]);
}

#[tokio::test(start_paused = true)]
async fn kredi_bitince_kaynaktan_okunmaz() {
    let ayar = AktarimAyarlari {
        kredi_paket: 2,
        ..AktarimAyarlari::default()
    };
    let (mut u, mut p, kredi, okunan) = kur(ayar);
    for i in 0..10 {
        u.hata(&format!("h{i}"));
    }
    let bir = p.sonraki().await.unwrap();
    let iki = p.sonraki().await.unwrap();
    assert_eq!((bir.no(), iki.no()), (1, 2));
    assert!(
        tokio::time::timeout(Duration::from_secs(1), p.sonraki())
            .await
            .is_err(),
        "kredi yokken paket gelmez"
    );
    assert_eq!(
        okunan.load(Ordering::SeqCst),
        2,
        "kredi yokken olay kuyruğundan okunmaz"
    );
    assert_eq!(kredi.yolda().0, 2);
    kredi.isledim(1).unwrap();
    let uc = p.sonraki().await.unwrap();
    assert_eq!(uc.no(), 3);
    assert_eq!(
        olaylar(&uc),
        vec![Olay::Hata {
            mesaj: "h2".into()
        }]
    );
}

#[tokio::test(start_paused = true)]
async fn krediden_buyuk_olay_yolda_paket_yokken_gider() {
    let ayar = AktarimAyarlari {
        kredi_bayt: 1000,
        ..AktarimAyarlari::default()
    };
    let (mut u, mut p, kredi, _) = kur(ayar);
    let buyuk = "b".repeat(2000);
    u.hata("küçük");
    u.hata(&buyuk);
    let bir = p.sonraki().await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(1), p.sonraki())
            .await
            .is_err(),
        "büyük olay yolda paket varken gitmez"
    );
    kredi.isledim(bir.no()).unwrap();
    let iki = p.sonraki().await.unwrap();
    assert_eq!(olaylar(&iki), vec![Olay::Hata { mesaj: buyuk }]);
    assert_eq!(kredi.yolda().0, 1);
}

#[tokio::test]
async fn isledim_gonderilmemis_paketi_reddeder_ve_tekrari_zararsizdir() {
    let (mut u, mut p, kredi, _) = kur(AktarimAyarlari::default());
    u.hata("h");
    let m = p.sonraki().await.unwrap();
    assert_eq!(
        kredi.isledim(5),
        Err(IsledimHatasi::GonderilmemisPaket { no: 5, son: 1 })
    );
    kredi.isledim(m.no()).unwrap();
    kredi.isledim(m.no()).unwrap();
    assert_eq!(kredi.yolda(), (0, 0));
}

#[tokio::test(start_paused = true)]
async fn kaynak_kapaninca_kapandi_gelir_ve_numaralar_bosluksuzdur() {
    let (mut u, mut p, kredi, _) = kur(AktarimAyarlari::default());
    u.hata("a");
    u.hata("b");
    drop(u);
    let mut numaralar = Vec::new();
    let mut son = None;
    while let Some(m) = p.sonraki().await {
        numaralar.push(m.no());
        kredi.isledim(m.no()).unwrap();
        son = Some(m);
    }
    assert_eq!(numaralar, vec![1, 2, 3]);
    assert_eq!(son, Some(KanalMesaji::Kapandi { no: 3 }));
}

#[test]
fn kanal_mesaji_json_bicimi() {
    let z = Zarf {
        sema: SEMA_SURUMU,
        oturum: OturumId::yeni(),
        tur: None,
        sira: 4,
        yuk: Olay::TurBasladi,
    };
    let j = serde_json::to_value(KanalMesaji::Olaylar {
        no: 1,
        zarflar: vec![z],
    })
    .unwrap();
    assert_eq!(j["tip"], "Olaylar");
    assert_eq!(j["no"], 1);
    assert_eq!(j["zarflar"][0]["sira"], 4);
    assert_eq!(
        serde_json::to_value(KanalMesaji::Kapandi { no: 2 }).unwrap(),
        serde_json::json!({"tip": "Kapandi", "no": 2})
    );
}
```

- [ ] **Step 3: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa-core --test aktarim`
Expected: FAIL — `kalfa_core::aktarim` modülü yok.

- [ ] **Step 4: Modülü yaz**

`crates/kalfa-core/src/lib.rs` içinde `pub mod arac;` satırından önce ekle:

```rust
pub mod aktarim;
```

`crates/kalfa-core/src/aktarim.rs`:

```rust
//! Çekirdekten arayüze olay aktarımı: paketleme ve kredi (1c tasarımı §4).
//!
//! Paketleyici olay kuyruğunu okur ve olayları kısa bir pencerede paketler.
//! Arayüzün "işledim" demediği veri krediyi doldurunca kuyruktan okumayı
//! bırakır: arayüz takılırsa birikim webview'da değil, sınırı belli olan
//! çekirdek kuyruğunda olur ve kuyruk dolunca oturumun iptal yolu çalışır.

use std::collections::VecDeque;
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::Notify;
use tokio::time::Instant;

use crate::oturum::OlayAlici;
use crate::protokol::{Olay, Zarf};

/// Paketleyici ayarları. Varsayılanlar başlangıç değeridir; gerçek arayüzle
/// yapılan gecikme ölçümüyle ayarlanır (1c tasarımı §11.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AktarimAyarlari {
    /// İlk olaydan sonra aynı pakete olay toplanan süre.
    pub pencere: Duration,
    /// Paketin hedef bayt üst sınırı; sınırdan büyük tek olay yine gider.
    pub azami_paket_bayt: usize,
    /// Arayüzün "işledim" demediği en fazla paket sayısı (en az 1 sayılır).
    pub kredi_paket: usize,
    /// Arayüzün "işledim" demediği en fazla bayt.
    pub kredi_bayt: usize,
}

impl Default for AktarimAyarlari {
    fn default() -> Self {
        Self {
            pencere: Duration::from_millis(16),
            azami_paket_bayt: 64 * 1024,
            kredi_paket: 4,
            kredi_bayt: 256 * 1024,
        }
    }
}

/// Arayüze giden kanal mesajı (1c tasarımı §4.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum KanalMesaji {
    /// Bir paket olay. `no` her abonelikte 1'den başlar ve boşluksuz artar.
    Olaylar { no: u64, zarflar: Vec<Zarf<Olay>> },
    /// Olay kuyruğu kapandı; bu abonelikten başka mesaj gelmez.
    Kapandi { no: u64 },
}

impl KanalMesaji {
    pub fn no(&self) -> u64 {
        match self {
            Self::Olaylar { no, .. } | Self::Kapandi { no } => *no,
        }
    }
}

/// Paketleyicinin okuduğu olay kaynağı. `recv` iptale dayanıklı olmalıdır:
/// dönen future bırakılırsa olay kaybolmamalıdır.
pub trait OlayKaynagi: Send {
    fn recv(&mut self) -> impl Future<Output = Option<Zarf<Olay>>> + Send;
}

impl OlayKaynagi for OlayAlici {
    fn recv(&mut self) -> impl Future<Output = Option<Zarf<Olay>>> + Send {
        OlayAlici::recv(self)
    }
}

/// Onay, hata, tamamlanma, araç, şart ve aktarım olayları birleştirilmez ve
/// pencereyi beklemeden gönderilir (1c tasarımı §4.2).
pub fn kritik_mi(olay: &Olay) -> bool {
    matches!(
        olay,
        Olay::OnayGerekiyor { .. }
            | Olay::Hata { .. }
            | Olay::Tamamlandi { .. }
            | Olay::AracCagrisiBasladi { .. }
            | Olay::AracSonucu { .. }
            | Olay::SartOnayiGerekiyor { .. }
            | Olay::AktarimDurdu { .. }
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum IsledimHatasi {
    #[error("{no} numaralı paket henüz gönderilmedi (son gönderilen: {son})")]
    GonderilmemisPaket { no: u64, son: u64 },
}

#[derive(Debug, Default)]
struct KrediDurumu {
    /// "İşledim" beklenen paketler: (numara, bayt).
    yolda: VecDeque<(u64, usize)>,
    yolda_bayt: usize,
    son_gonderilen: u64,
}

#[derive(Debug, Default)]
struct KrediOrtak {
    durum: Mutex<KrediDurumu>,
    uyandir: Notify,
}

impl KrediOrtak {
    fn durum(&self) -> MutexGuard<'_, KrediDurumu> {
        self.durum.lock().unwrap_or_else(|z| z.into_inner())
    }
}

/// Arayüzün "işledim" bildirimini paketleyiciye iletir. Paketleyici başka bir
/// görevde çalışırken de kullanılabilir.
#[derive(Debug, Clone)]
pub struct KrediTutamaci {
    ortak: Arc<KrediOrtak>,
}

impl KrediTutamaci {
    /// Arayüz `no` numaralı pakete kadar (o dahil) bütün paketleri işledi.
    /// Aynı ya da daha eski numarayı yeniden bildirmek zararsızdır.
    pub fn isledim(&self, no: u64) -> Result<(), IsledimHatasi> {
        let mut d = self.ortak.durum();
        if no > d.son_gonderilen {
            return Err(IsledimHatasi::GonderilmemisPaket {
                no,
                son: d.son_gonderilen,
            });
        }
        while d.yolda.front().is_some_and(|&(ilk, _)| ilk <= no) {
            if let Some((_, bayt)) = d.yolda.pop_front() {
                d.yolda_bayt = d.yolda_bayt.saturating_sub(bayt);
            }
        }
        drop(d);
        self.ortak.uyandir.notify_one();
        Ok(())
    }

    /// "İşledim" beklenen paket sayısı ve bayt toplamı.
    pub fn yolda(&self) -> (usize, usize) {
        let d = self.ortak.durum();
        (d.yolda.len(), d.yolda_bayt)
    }
}

/// Serileştirilmiş boyutu bilinen olay.
struct OlculuOlay {
    zarf: Zarf<Olay>,
    bayt: usize,
}

impl OlculuOlay {
    fn yeni(zarf: Zarf<Olay>) -> Self {
        let bayt = olc(&zarf);
        Self { zarf, bayt }
    }
}

fn olc<T: Serialize>(deger: &T) -> usize {
    serde_json::to_vec(deger).map_or(usize::MAX, |v| v.len())
}

/// Aynı turdaki bitişik iki metin parçasını birleştirir; birleşen parça son
/// parçanın sırasını taşır.
fn birlestir(onceki: &Zarf<Olay>, yeni: &Zarf<Olay>) -> Option<Zarf<Olay>> {
    if onceki.oturum != yeni.oturum || onceki.tur != yeni.tur {
        return None;
    }
    let yuk = match (&onceki.yuk, &yeni.yuk) {
        (Olay::TokenParcasi { metin: a }, Olay::TokenParcasi { metin: b }) => Olay::TokenParcasi {
            metin: format!("{a}{b}"),
        },
        (Olay::DusunceParcasi { metin: a }, Olay::DusunceParcasi { metin: b }) => {
            Olay::DusunceParcasi {
                metin: format!("{a}{b}"),
            }
        }
        _ => return None,
    };
    Some(Zarf {
        sema: yeni.sema,
        oturum: yeni.oturum,
        tur: yeni.tur,
        sira: yeni.sira,
        yuk,
    })
}

/// Doldurulmakta olan paket.
struct Paket {
    zarflar: Vec<Zarf<Olay>>,
    /// Zarfların serileştirilmiş boyutlarının toplamı.
    bayt: usize,
    /// Son zarfın boyutu; birleştirmede güncellenir.
    son_bayt: usize,
}

impl Paket {
    fn yeni(ilk: OlculuOlay) -> Self {
        Self {
            bayt: ilk.bayt,
            son_bayt: ilk.bayt,
            zarflar: vec![ilk.zarf],
        }
    }

    /// Olayı ekler; son parçayla birleşebiliyorsa birleştirir. Sınırı
    /// aşacaksa eklemez ve olayı geri verir.
    fn ekle(&mut self, olay: OlculuOlay, sinir: usize) -> Result<(), OlculuOlay> {
        let birlesik = self
            .zarflar
            .last()
            .and_then(|son| birlestir(son, &olay.zarf));
        if let Some(birlesik) = birlesik {
            let yeni_bayt = olc(&birlesik);
            let toplam = self
                .bayt
                .saturating_sub(self.son_bayt)
                .saturating_add(yeni_bayt);
            if toplam > sinir {
                return Err(olay);
            }
            if let Some(son) = self.zarflar.last_mut() {
                *son = birlesik;
            }
            self.bayt = toplam;
            self.son_bayt = yeni_bayt;
            return Ok(());
        }
        let toplam = self.bayt.saturating_add(olay.bayt);
        if toplam > sinir {
            return Err(olay);
        }
        self.bayt = toplam;
        self.son_bayt = olay.bayt;
        self.zarflar.push(olay.zarf);
        Ok(())
    }
}

/// Olay kuyruğunu paketleyip kanala hazırlayan birim.
pub struct Paketleyici<K: OlayKaynagi> {
    kaynak: K,
    ayarlar: AktarimAyarlari,
    ortak: Arc<KrediOrtak>,
    sonraki_no: u64,
    /// Kaynaktan alınmış ama henüz bir pakete girmemiş olay.
    bekleyen: Option<OlculuOlay>,
    kaynak_kapandi: bool,
    bitti: bool,
}

impl<K: OlayKaynagi> Paketleyici<K> {
    pub fn yeni(kaynak: K, ayarlar: AktarimAyarlari) -> (Self, KrediTutamaci) {
        let ortak = Arc::new(KrediOrtak::default());
        let paketleyici = Self {
            kaynak,
            ayarlar,
            ortak: ortak.clone(),
            sonraki_no: 1,
            bekleyen: None,
            kaynak_kapandi: false,
            bitti: false,
        };
        (paketleyici, KrediTutamaci { ortak })
    }

    /// Sonraki kanal mesajı; `Kapandi`dan sonra `None`.
    ///
    /// Kredi yoksa "işledim" gelene kadar bekler ve bu sırada kaynaktan okumaz.
    /// Kredi beklenirken future bırakılırsa olay kaybolmaz; paket doldurulurken
    /// bırakılırsa o paketin olayları kaybolur. Köprü bu yüzden `sonraki`yi
    /// ayrı bir görevde, `select!` dışında çağırır.
    pub async fn sonraki(&mut self) -> Option<KanalMesaji> {
        if self.bitti {
            return None;
        }
        self.kredi_bekle(1).await;
        if self.bekleyen.is_none() {
            let zarf = if self.kaynak_kapandi {
                None
            } else {
                self.kaynak.recv().await
            };
            match zarf {
                Some(zarf) => self.bekleyen = Some(OlculuOlay::yeni(zarf)),
                None => return Some(self.kapanis()),
            }
        }
        let gereken = self.bekleyen.as_ref().map_or(0, |o| o.bayt);
        self.kredi_bekle(gereken).await;
        let ilk = self.bekleyen.take()?;
        let sinir = self.paket_siniri();
        let zarflar = self.paketle(ilk, sinir).await;
        Some(self.gonder(zarflar))
    }

    /// İlk olayla başlayan paketi pencere dolana, bayt sınırına ulaşılana ya
    /// da kritik bir olay gelene kadar doldurur.
    async fn paketle(&mut self, ilk: OlculuOlay, sinir: usize) -> Vec<Zarf<Olay>> {
        let ilk_kritik = kritik_mi(&ilk.zarf.yuk);
        let mut paket = Paket::yeni(ilk);
        if ilk_kritik {
            return paket.zarflar;
        }
        let son = Instant::now() + self.ayarlar.pencere;
        while paket.bayt < sinir {
            let olay = match tokio::time::timeout_at(son, self.kaynak.recv()).await {
                Ok(Some(zarf)) => OlculuOlay::yeni(zarf),
                Ok(None) => {
                    self.kaynak_kapandi = true;
                    break;
                }
                Err(_) => break,
            };
            let kritik = kritik_mi(&olay.zarf.yuk);
            if let Err(sigmayan) = paket.ekle(olay, sinir) {
                self.bekleyen = Some(sigmayan);
                break;
            }
            if kritik {
                break;
            }
        }
        paket.zarflar
    }

    fn paket_siniri(&self) -> usize {
        let yolda_bayt = self.ortak.durum().yolda_bayt;
        self.ayarlar
            .azami_paket_bayt
            .min(self.ayarlar.kredi_bayt.saturating_sub(yolda_bayt))
    }

    /// `bayt` büyüklüğünde bir olay krediye sığana ya da yol boşalana kadar
    /// bekler.
    async fn kredi_bekle(&self, bayt: usize) {
        loop {
            let bildirim = self.ortak.uyandir.notified();
            if self.kredi_var(bayt) {
                return;
            }
            bildirim.await;
        }
    }

    fn kredi_var(&self, bayt: usize) -> bool {
        let d = self.ortak.durum();
        d.yolda.len() < self.ayarlar.kredi_paket.max(1)
            && (d.yolda.is_empty()
                || d.yolda_bayt.saturating_add(bayt) <= self.ayarlar.kredi_bayt)
    }

    fn gonder(&mut self, zarflar: Vec<Zarf<Olay>>) -> KanalMesaji {
        let no = self.numara_al();
        let mesaj = KanalMesaji::Olaylar { no, zarflar };
        let bayt = olc(&mesaj);
        let mut d = self.ortak.durum();
        d.yolda.push_back((no, bayt));
        d.yolda_bayt = d.yolda_bayt.saturating_add(bayt);
        d.son_gonderilen = no;
        drop(d);
        mesaj
    }

    fn kapanis(&mut self) -> KanalMesaji {
        self.bitti = true;
        let no = self.numara_al();
        self.ortak.durum().son_gonderilen = no;
        KanalMesaji::Kapandi { no }
    }

    fn numara_al(&mut self) -> u64 {
        let no = self.sonraki_no;
        self.sonraki_no += 1;
        no
    }
}
```

- [ ] **Step 5: Birim testlerini koştur**

Run: `cargo test -p kalfa-core --test aktarim`
Expected: PASS (12 test).

- [ ] **Step 6: Gerçek oturumla bütünleşik testi yaz**

`crates/kalfa-core/tests/aktarim_oturum.rs`:

```rust
use std::path::Path;
use std::time::Duration;

use kalfa_core::aktarim::{AktarimAyarlari, KanalMesaji, Paketleyici};
use kalfa_core::oturum::{OlayButcesi, OturumAyarlari, baslat};
use kalfa_core::protokol::{AktarimDurmaNedeni, Komut, Olay};
use kalfa_core::test_destek::{SahteSaglayici, SahteYanit, sanal_siniflandirici, test_baglami};
use kalfa_evren::AkisOlayi;

fn durma_nedeni(m: &KanalMesaji) -> Option<AktarimDurmaNedeni> {
    match m {
        KanalMesaji::Olaylar { zarflar, .. } => zarflar.iter().find_map(|z| match z.yuk {
            Olay::AktarimDurdu { neden } => Some(neden),
            _ => None,
        }),
        KanalMesaji::Kapandi { .. } => None,
    }
}

#[tokio::test]
async fn islemeyen_arayuzde_bellek_sinirli_kalir_tur_iptal_edilir_ve_neden_bildirilir() {
    let saglayici = SahteSaglayici::yeni(vec![SahteYanit::Askida(
        (0..200)
            .map(|i| AkisOlayi::Metin(format!("parça-{i} ")))
            .collect(),
    )]);
    let mut ayar = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    ayar.olay_butcesi = OlayButcesi::yeni(16, 1024 * 1024).unwrap();
    let (tutamac, alici) = baslat(
        ayar,
        test_baglami(saglayici, vec![], sanal_siniflandirici()),
    )
    .unwrap();
    let aktarim = AktarimAyarlari {
        azami_paket_bayt: 1024,
        kredi_paket: 1,
        ..AktarimAyarlari::default()
    };
    let (mut p, kredi) = Paketleyici::yeni(alici, aktarim);
    tutamac
        .gonder(Komut::KullaniciMesaji {
            metin: "uzun akış".into(),
        })
        .await
        .unwrap();

    // İlk paket gelir; arayüz "işledim" demez.
    let ilk = tokio::time::timeout(Duration::from_secs(2), p.sonraki())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ilk.no(), 1);
    let mut neden = durma_nedeni(&ilk);

    // Paketleyici kuyruktan okumayı bıraktığı için çekirdek bütçesi dolar,
    // tur iptal edilir.
    tokio::time::timeout(Duration::from_secs(2), async {
        while tutamac.aktif_tur().is_some() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("tur iptal edilmeli");
    assert_eq!(kredi.yolda().0, 1, "yolda en çok kredi kadar paket olur");

    // Arayüz yeniden işlemeye başlayınca neden görünür, sonra kanal kapanır.
    kredi.isledim(ilk.no()).unwrap();
    let mut son = None;
    while let Some(m) = tokio::time::timeout(Duration::from_secs(2), p.sonraki())
        .await
        .unwrap()
    {
        assert!(kredi.yolda().0 <= 1);
        kredi.isledim(m.no()).unwrap();
        neden = neden.or(durma_nedeni(&m));
        son = Some(m);
    }
    assert_eq!(neden, Some(AktarimDurmaNedeni::TuketiciYetismedi));
    assert!(matches!(son, Some(KanalMesaji::Kapandi { .. })));
}
```

- [ ] **Step 7: Bütünleşik testi koştur**

Run: `cargo test -p kalfa-core --test aktarim_oturum`
Expected: PASS.

- [ ] **Step 8: Workspace doğrulaması ve commit**

Run: `cargo test --workspace --quiet && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: temiz.

```bash
git add crates/kalfa-core/Cargo.toml crates/kalfa-core/src/lib.rs crates/kalfa-core/src/aktarim.rs crates/kalfa-core/tests/aktarim.rs crates/kalfa-core/tests/aktarim_oturum.rs Cargo.lock
git commit -m "kalfa-core: olay paketleyici ve kredi (1c tasarımı §4)"
```

---

### Task 4: Şart kapısı ve `Saglayici::sart_durumu`

Uygulama genelinde tek şart kapısı (1c tasarımı §7.3). Bu görev kapının kendisini ve sağlayıcıdaki şart durumu sorgusunu ekler; oturumlar kapıya Görev 5'te bağlanır.

**Files:**
- Create: `crates/kalfa-core/src/sart_kapisi.rs`
- Modify: `crates/kalfa-core/src/lib.rs`
- Modify: `crates/kalfa-core/src/saglayici.rs`
- Modify: `crates/kalfa-core/src/test_destek.rs`
- Test: `crates/kalfa-core/tests/sart_kapisi.rs`

**Interfaces:**
- Consumes: `saglayici::Saglayici::{sart_metni, sart_kabul}`; `kalfa_evren::tipler::{TermsStatus, TermsText}`.
- Produces:
  - `Saglayici::sart_durumu(&self) -> Result<TermsStatus, EvrenError>`
  - `sart_kapisi::SartMetni { surum: u32, esasli: bool, metin: String }` (serde)
  - `sart_kapisi::KapiGorunumu { Bosta, Gosteriliyor(SartMetni), Reddedildi }`
  - `sart_kapisi::SartKarari { Kabul { surum: u32 }, Ret }` (`Copy`)
  - `sart_kapisi::KabulSonucu { Kabul, YeniMetin(SartMetni) }`
  - `sart_kapisi::IlkDurum { KabulEdilmis, KabulGerekli(SartMetni) }`
  - `sart_kapisi::KapiDurumu { gorunum: KapiGorunumu, nesil: u64, son_karar: Option<SartKarari> }`
  - `SartKapisi::yeni(Arc<dyn Saglayici>) -> Self`; `durum(&self) -> KapiDurumu`; `izle(&self) -> watch::Receiver<KapiDurumu>`; `async ilk_durum(&self) -> Result<IlkDurum, EvrenError>`; `async gerekli(&self) -> Result<(SartMetni, u64), EvrenError>`; `async karar_bekle(&self, nesil: u64) -> SartKarari`; `async kabul_et(&self, surum: u32) -> Result<KabulSonucu, EvrenError>`; `reddet(&self)`
  - `SahteSaglayici::sart_durumu_hatasi(&self, Option<EvrenError>)`

- [ ] **Step 1: Başarısız testleri yaz**

`crates/kalfa-core/tests/sart_kapisi.rs`:

```rust
use std::sync::Arc;

use kalfa_core::saglayici::Saglayici;
use kalfa_core::sart_kapisi::{
    IlkDurum, KabulSonucu, KapiGorunumu, SartKapisi, SartKarari, SartMetni,
};
use kalfa_core::test_destek::SahteSaglayici;
use kalfa_evren::EvrenError;

fn kapi(s: &Arc<SahteSaglayici>) -> SartKapisi {
    SartKapisi::yeni(s.clone())
}

fn beklenen(surum: u32) -> SartMetni {
    SartMetni {
        surum,
        esasli: true,
        metin: "# Sahte kullanım şartları".into(),
    }
}

#[tokio::test]
async fn ilk_durum_kabul_edilmis() {
    let s = SahteSaglayici::yeni(vec![]);
    // Kullanıcı şartları daha önce kabul etmiş.
    s.sart_kabul(1).await.unwrap();
    let k = kapi(&s);
    assert_eq!(k.ilk_durum().await.unwrap(), IlkDurum::KabulEdilmis);
    assert_eq!(k.durum().gorunum, KapiGorunumu::Bosta);
}

#[tokio::test]
async fn ilk_durum_kabul_gerekliyse_metin_gosterilir() {
    let s = SahteSaglayici::yeni(vec![]);
    let k = kapi(&s);
    assert_eq!(
        k.ilk_durum().await.unwrap(),
        IlkDurum::KabulGerekli(beklenen(1))
    );
    assert_eq!(k.durum().gorunum, KapiGorunumu::Gosteriliyor(beklenen(1)));
}

#[tokio::test]
async fn ilk_durumda_anahtar_gecersizse_hata_doner() {
    let s = SahteSaglayici::yeni(vec![]);
    s.sart_durumu_hatasi(Some(EvrenError::AnahtarGecersiz));
    assert_eq!(kapi(&s).ilk_durum().await, Err(EvrenError::AnahtarGecersiz));
}

#[tokio::test]
async fn bekleyenler_tek_kabulle_uyanir() {
    let s = SahteSaglayici::yeni(vec![]);
    let k = Arc::new(kapi(&s));
    let (m1, n1) = k.gerekli().await.unwrap();
    let (m2, n2) = k.gerekli().await.unwrap();
    assert_eq!(
        (m1.clone(), n1),
        (m2, n2),
        "ikinci oturum gösterilen metne katılır"
    );
    let bekleyenler: Vec<_> = (0..2)
        .map(|_| {
            let k = k.clone();
            tokio::spawn(async move { k.karar_bekle(n1).await })
        })
        .collect();
    assert_eq!(k.kabul_et(m1.surum).await.unwrap(), KabulSonucu::Kabul);
    for b in bekleyenler {
        assert_eq!(b.await.unwrap(), SartKarari::Kabul { surum: 1 });
    }
    assert_eq!(s.kabul_edilen_surumler(), vec![1], "kabul EVREN'e bir kez gider");
    assert_eq!(k.durum().gorunum, KapiGorunumu::Bosta);
}

#[tokio::test]
async fn gosterilmeyen_surum_kabul_edilmez() {
    let s = SahteSaglayici::yeni(vec![]);
    let k = kapi(&s);
    assert_eq!(
        k.kabul_et(1).await.unwrap(),
        KabulSonucu::YeniMetin(beklenen(1)),
        "gösterilmemiş metin kabul edilemez"
    );
    assert_eq!(
        k.kabul_et(2).await.unwrap(),
        KabulSonucu::YeniMetin(beklenen(1))
    );
    assert!(s.kabul_edilen_surumler().is_empty());
}

#[tokio::test]
async fn okurken_surum_degisirse_yeni_metin_gosterilir() {
    let s = SahteSaglayici::yeni(vec![]);
    let k = kapi(&s);
    k.gerekli().await.unwrap();
    s.sart_surumunu_degistir(2);
    assert_eq!(
        k.kabul_et(1).await.unwrap(),
        KabulSonucu::YeniMetin(beklenen(2))
    );
    assert_eq!(k.kabul_et(2).await.unwrap(), KabulSonucu::Kabul);
    assert_eq!(s.kabul_edilen_surumler(), vec![2]);
}

#[tokio::test]
async fn ret_bekleyenlere_iletilir_ve_sonraki_istekte_metin_yeniden_gelir() {
    let s = SahteSaglayici::yeni(vec![]);
    let k = Arc::new(kapi(&s));
    let (_, n) = k.gerekli().await.unwrap();
    let bekleyen = {
        let k = k.clone();
        tokio::spawn(async move { k.karar_bekle(n).await })
    };
    k.reddet();
    assert_eq!(bekleyen.await.unwrap(), SartKarari::Ret);
    assert_eq!(k.durum().gorunum, KapiGorunumu::Reddedildi);
    let (m, _) = k.gerekli().await.unwrap();
    assert_eq!(m, beklenen(1));
    assert_eq!(k.durum().gorunum, KapiGorunumu::Gosteriliyor(beklenen(1)));
}
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa-core --test sart_kapisi`
Expected: FAIL — `sart_kapisi` modülü ve `sart_durumu_hatasi` yok.

- [ ] **Step 3: Sağlayıcıya şart durumu sorgusunu ekle**

`crates/kalfa-core/src/saglayici.rs` içinde içe aktarımı değiştir:

```rust
use kalfa_evren::tipler::{ChatRequest, TermsStatus, TermsText};
```

`Saglayici` trait'ine ekle (`sart_metni`'den önce):

```rust
    async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError>;
```

`impl Saglayici for EvrenSaglayici` içine ekle:

```rust
    async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError> {
        self.istemci.sart_durumu().await
    }
```

`crates/kalfa-core/src/test_destek.rs` içinde:

İçe aktarımı değiştir:

```rust
use kalfa_evren::tipler::{ChatRequest, FunctionCall, TermsStatus, TermsText, ToolCall};
```

`SahteSaglayici` yapısına alan ekle:

```rust
pub struct SahteSaglayici {
    senaryo: Mutex<VecDeque<SahteYanit>>,
    istekler: Mutex<Vec<ChatRequest>>,
    sart: Mutex<SahteSart>,
    durum_hatasi: Mutex<Option<EvrenError>>,
}
```

`SahteSaglayici::yeni` içindeki yapı ifadesine `durum_hatasi: Mutex::new(None),` ekle ve `impl SahteSaglayici` bloğuna şu yöntemi ekle:

```rust
    /// `sart_durumu` bu hatayı döner (`None`: normal davranış).
    pub fn sart_durumu_hatasi(&self, hata: Option<EvrenError>) {
        *self.durum_hatasi.lock().unwrap() = hata;
    }
```

`impl Saglayici for SahteSaglayici` içine ekle:

```rust
    async fn sart_durumu(&self) -> Result<TermsStatus, EvrenError> {
        if let Some(hata) = self.durum_hatasi.lock().unwrap().clone() {
            return Err(hata);
        }
        let s = self.sart.lock().unwrap();
        Ok(TermsStatus {
            current_version: s.surum,
            is_material: true,
            accepted: s.kabul_edilen.contains(&s.surum),
            doc_path: None,
        })
    }
```

- [ ] **Step 4: Kapıyı yaz**

`crates/kalfa-core/src/lib.rs` içinde `pub mod saglayici;` satırından sonra ekle:

```rust
pub mod sart_kapisi;
```

`crates/kalfa-core/src/sart_kapisi.rs`:

```rust
//! Uygulama genelinde tek şart kapısı (1c tasarımı §7.3, üst belge §11.8).
//!
//! 403 alan oturumlar kapıya kaydolup bekler. Kapı metni her sürüm için bir kez
//! alır, şart penceresine hangi sürümü gösterdiğini saklar ve kullanıcı kabul
//! edince kabul isteğini **bir kez** gönderir. Kabul yalnızca `kabul_et` ile
//! gider; onu da yalnızca şart penceresinin komutu çağırır (1c tasarımı §6).

use std::sync::Arc;

use kalfa_evren::EvrenError;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, watch};

use crate::saglayici::Saglayici;

/// Şart penceresinde gösterilen metin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SartMetni {
    pub surum: u32,
    /// EVREN'in `is_material` bilgisi: değişiklik esaslı mı.
    pub esasli: bool,
    pub metin: String,
}

/// Kapının dışarıdan görünen hâli; uygulama şart penceresini buna göre açar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KapiGorunumu {
    /// Bekleyen bir şart onayı yok.
    Bosta,
    /// Metin gösteriliyor; kullanıcının kararı bekleniyor.
    Gosteriliyor(SartMetni),
    /// Kullanıcı reddetti; uygulama kilitli görünümde.
    Reddedildi,
}

/// Bekleyen oturumların öğrendiği karar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SartKarari {
    Kabul { surum: u32 },
    Ret,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KabulSonucu {
    /// Kabul EVREN'e gönderildi; bekleyen oturumlar uyandı.
    Kabul,
    /// Sürüm gösterilenle aynı değildi ya da bu arada değişti; güncel metin
    /// gösteriliyor.
    YeniMetin(SartMetni),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IlkDurum {
    KabulEdilmis,
    KabulGerekli(SartMetni),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KapiDurumu {
    pub gorunum: KapiGorunumu,
    /// Her kararla bir artar.
    pub nesil: u64,
    pub son_karar: Option<SartKarari>,
}

pub struct SartKapisi {
    saglayici: Arc<dyn Saglayici>,
    /// Metin alma ve kabul gönderme sırayla yapılır; aynı anda iki istek gitmez.
    islem: Mutex<()>,
    durum: watch::Sender<KapiDurumu>,
}

impl SartKapisi {
    pub fn yeni(saglayici: Arc<dyn Saglayici>) -> Self {
        let (durum, _) = watch::channel(KapiDurumu {
            gorunum: KapiGorunumu::Bosta,
            nesil: 0,
            son_karar: None,
        });
        Self {
            saglayici,
            islem: Mutex::new(()),
            durum,
        }
    }

    pub fn durum(&self) -> KapiDurumu {
        self.durum.borrow().clone()
    }

    /// Kapının değişikliklerini izler; uygulama şart penceresini buna göre
    /// açar ya da kapatır.
    pub fn izle(&self) -> watch::Receiver<KapiDurumu> {
        self.durum.subscribe()
    }

    /// İlk açılış: şart durumu sorulur; kabul gerekiyorsa metin gösterime
    /// alınır.
    pub async fn ilk_durum(&self) -> Result<IlkDurum, EvrenError> {
        if self.saglayici.sart_durumu().await?.accepted {
            return Ok(IlkDurum::KabulEdilmis);
        }
        let (metin, _) = self.gerekli().await?;
        Ok(IlkDurum::KabulGerekli(metin))
    }

    /// 403 alan oturum çağırır. Gösterilen metin varsa ona katılır, yoksa
    /// EVREN'den alır. Dönen nesil `karar_bekle`'ye verilir.
    pub async fn gerekli(&self) -> Result<(SartMetni, u64), EvrenError> {
        let _sira = self.islem.lock().await;
        let mevcut = self.durum();
        if let KapiGorunumu::Gosteriliyor(metin) = mevcut.gorunum {
            return Ok((metin, mevcut.nesil));
        }
        let metin = self.metni_al().await?;
        self.goster(metin.clone());
        Ok((metin, mevcut.nesil))
    }

    /// `nesil`den sonraki ilk kararı bekler. Future bırakılırsa kapı
    /// etkilenmez; oturumun iptali böyle çalışır.
    pub async fn karar_bekle(&self, nesil: u64) -> SartKarari {
        let mut izleyici = self.durum.subscribe();
        loop {
            let karar = {
                let d = izleyici.borrow_and_update();
                if d.nesil > nesil { d.son_karar } else { None }
            };
            if let Some(karar) = karar {
                return karar;
            }
            if izleyici.changed().await.is_err() {
                return SartKarari::Ret;
            }
        }
    }

    /// Şart penceresinin "Kabul ediyorum" düğmesi. Sürüm gösterilenle aynı
    /// değilse kabul gönderilmez; kabul EVREN'e bir kez gider.
    pub async fn kabul_et(&self, surum: u32) -> Result<KabulSonucu, EvrenError> {
        let _sira = self.islem.lock().await;
        let gosterilen = match self.durum().gorunum {
            KapiGorunumu::Gosteriliyor(metin) => Some(metin.surum),
            _ => None,
        };
        if gosterilen != Some(surum) {
            let metin = self.metni_al().await?;
            self.goster(metin.clone());
            return Ok(KabulSonucu::YeniMetin(metin));
        }
        match self.saglayici.sart_kabul(surum).await {
            Ok(()) => {
                self.karar(KapiGorunumu::Bosta, SartKarari::Kabul { surum });
                Ok(KabulSonucu::Kabul)
            }
            Err(EvrenError::SartKabulGerekli { .. }) => {
                let metin = self.metni_al().await?;
                self.goster(metin.clone());
                Ok(KabulSonucu::YeniMetin(metin))
            }
            Err(e) => Err(e),
        }
    }

    /// "Kabul etmiyorum": bekleyen oturumların turları iptal edilir.
    pub fn reddet(&self) {
        self.karar(KapiGorunumu::Reddedildi, SartKarari::Ret);
    }

    fn goster(&self, metin: SartMetni) {
        self.durum
            .send_modify(|d| d.gorunum = KapiGorunumu::Gosteriliyor(metin));
    }

    fn karar(&self, gorunum: KapiGorunumu, karar: SartKarari) {
        self.durum.send_modify(|d| {
            d.gorunum = gorunum;
            d.nesil += 1;
            d.son_karar = Some(karar);
        });
    }

    async fn metni_al(&self) -> Result<SartMetni, EvrenError> {
        let durum = self.saglayici.sart_durumu().await?;
        let metin = self.saglayici.sart_metni().await?;
        Ok(SartMetni {
            surum: metin.version,
            esasli: durum.is_material,
            metin: metin.content,
        })
    }
}
```

- [ ] **Step 5: Testleri koştur**

Run: `cargo test -p kalfa-core --test sart_kapisi`
Expected: PASS (7 test).

- [ ] **Step 6: Workspace doğrulaması ve commit**

Run: `cargo test --workspace --quiet && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: temiz.

```bash
git add crates/kalfa-core/src/lib.rs crates/kalfa-core/src/sart_kapisi.rs crates/kalfa-core/src/saglayici.rs crates/kalfa-core/src/test_destek.rs crates/kalfa-core/tests/sart_kapisi.rs
git commit -m "kalfa-core: uygulama genelinde tek şart kapısı"
```

---

### Task 5: Oturumun şart akışını kapıya devretmek

Oturum kabul isteğini artık kendisi göndermez: 403 alınca kapıya kaydolur, `SartOnayiGerekiyor` yayar ve kararı bekler. `Komut::SartlariKabulEt` oturum komutu olmaktan çıkar; kabul edilen sürüm kayda `SartKabulEdildi` olarak yazılır (1c tasarımı §3.3, §7.3).

**Files:**
- Modify: `crates/kalfa-core/src/oturum/mod.rs`
- Modify: `crates/kalfa-core/src/oturum/model_turu.rs`
- Modify: `crates/kalfa-core/src/protokol.rs`
- Modify: `crates/kalfa-core/src/kayit.rs`
- Modify: `crates/kalfa-core/src/test_destek.rs`
- Modify: `crates/kalfa-tools/tests/uctan_uca.rs`
- Modify: `crates/kalfa-core/tests/oturum_metin.rs`
- Modify: `crates/kalfa-core/tests/protokol.rs`
- Test: `crates/kalfa-core/tests/oturum_sart.rs` (baştan yazılır)

**Interfaces:**
- Consumes: Görev 4'ün `SartKapisi::{gerekli, karar_bekle}`, `SartKarari`.
- Produces: `OturumBaglami::sart_kapisi: Arc<SartKapisi>`; `KayitOgesi::SartKabulEdildi { surum: u32 }`. `Komut::SartlariKabulEt` kaldırılır.

- [ ] **Step 1: Testleri kapıya göre yeniden yaz**

`crates/kalfa-core/tests/oturum_sart.rs` dosyasının tamamını şununla değiştir:

```rust
use std::path::Path;
use std::sync::Arc;

use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::oturum::OturumAyarlari;
use kalfa_core::protokol::{BitisNedeni, Komut, Olay};
use kalfa_core::sart_kapisi::{KabulSonucu, SartKapisi};
use kalfa_core::test_destek::*;
use kalfa_evren::EvrenError;

fn ayarlar() -> OturumAyarlari {
    OturumAyarlari::yeni("glm-5.3", Path::new("calisma"))
}

fn oturum(s: &Arc<SahteSaglayici>, ayar: OturumAyarlari) -> (Surucu, Arc<SartKapisi>) {
    let baglam = test_baglami(s.clone(), vec![], sanal_siniflandirici());
    let kapi = baglam.sart_kapisi.clone();
    (test_oturumu(baglam, ayar), kapi)
}

fn senaryo() -> Arc<SahteSaglayici> {
    SahteSaglayici::yeni(vec![
        SahteYanit::Hata(EvrenError::SartKabulGerekli { surum: Some(1) }),
        SahteYanit::metin("tamam"),
    ])
}

fn mesaj() -> Komut {
    Komut::KullaniciMesaji {
        metin: "selam".into(),
    }
}

async fn sart_bekle(d: &mut Surucu, beklenen: u32) {
    let z = d
        .bekle(|o| matches!(o, Olay::SartOnayiGerekiyor { .. }))
        .await;
    match z.yuk {
        Olay::SartOnayiGerekiyor { surum, metin } => {
            assert_eq!(surum, beklenen);
            assert_eq!(metin, "# Sahte kullanım şartları");
        }
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn sart_kabul_edilince_istek_yeniden_gonderilir() {
    let s = senaryo();
    let (mut d, kapi) = oturum(&s, ayarlar());
    d.gonder(mesaj()).await;
    sart_bekle(&mut d, 1).await;
    assert!(
        s.kabul_edilen_surumler().is_empty(),
        "kullanıcı onaylamadan kabul gönderilmez"
    );
    assert_eq!(kapi.kabul_et(1).await.unwrap(), KabulSonucu::Kabul);
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(d.metin(), "tamam");
    assert_eq!(s.kabul_edilen_surumler(), vec![1]);
    assert_eq!(s.istekler().len(), 2);
}

#[tokio::test]
async fn gosterilenden_farkli_surum_kabul_edilmez() {
    let s = senaryo();
    let (mut d, kapi) = oturum(&s, ayarlar());
    d.gonder(mesaj()).await;
    sart_bekle(&mut d, 1).await;
    assert!(matches!(
        kapi.kabul_et(2).await.unwrap(),
        KabulSonucu::YeniMetin(m) if m.surum == 1
    ));
    assert!(s.kabul_edilen_surumler().is_empty());
    assert_eq!(kapi.kabul_et(1).await.unwrap(), KabulSonucu::Kabul);
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(s.kabul_edilen_surumler(), vec![1]);
}

#[tokio::test]
async fn surum_okurken_degisirse_yeni_metin_gosterilir() {
    let s = senaryo();
    let (mut d, kapi) = oturum(&s, ayarlar());
    d.gonder(mesaj()).await;
    sart_bekle(&mut d, 1).await;
    s.sart_surumunu_degistir(2);
    assert!(matches!(
        kapi.kabul_et(1).await.unwrap(),
        KabulSonucu::YeniMetin(m) if m.surum == 2
    ));
    assert_eq!(kapi.kabul_et(2).await.unwrap(), KabulSonucu::Kabul);
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(
        s.kabul_edilen_surumler(),
        vec![2],
        "eski sürüm kabul edilmiş sayılmaz"
    );
}

#[tokio::test]
async fn sart_beklerken_iptal() {
    let s = senaryo();
    let (mut d, _kapi) = oturum(&s, ayarlar());
    d.gonder(mesaj()).await;
    let tur = d
        .bekle(|o| matches!(o, Olay::TurBasladi))
        .await
        .tur
        .unwrap();
    sart_bekle(&mut d, 1).await;
    d.gonder(Komut::Iptal { tur }).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);
    assert!(s.kabul_edilen_surumler().is_empty());
    assert_eq!(s.istekler().len(), 1);
}

#[tokio::test]
async fn ret_bekleyen_turu_iptal_eder() {
    let s = senaryo();
    let (mut d, kapi) = oturum(&s, ayarlar());
    d.gonder(mesaj()).await;
    sart_bekle(&mut d, 1).await;
    kapi.reddet();
    assert_eq!(d.tur_sonu().await, BitisNedeni::Iptal);
    assert!(s.kabul_edilen_surumler().is_empty());
}

#[tokio::test]
async fn iki_oturum_tek_kabulle_devam_eder() {
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::Hata(EvrenError::SartKabulGerekli { surum: Some(1) }),
        SahteYanit::Hata(EvrenError::SartKabulGerekli { surum: Some(1) }),
        SahteYanit::metin("bir"),
        SahteYanit::metin("iki"),
    ]);
    let baglam = test_baglami(s.clone(), vec![], sanal_siniflandirici());
    let kapi = baglam.sart_kapisi.clone();
    let mut a = test_oturumu(baglam.clone(), ayarlar());
    let mut b = test_oturumu(baglam, ayarlar());
    a.gonder(mesaj()).await;
    sart_bekle(&mut a, 1).await;
    b.gonder(mesaj()).await;
    sart_bekle(&mut b, 1).await;
    assert_eq!(kapi.kabul_et(1).await.unwrap(), KabulSonucu::Kabul);
    assert_eq!(a.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(b.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(
        s.kabul_edilen_surumler(),
        vec![1],
        "kabul EVREN'e bir kez gider"
    );
}

#[tokio::test]
async fn kabul_edilen_surum_kayda_yazilir() {
    let dizin = tempfile::tempdir().unwrap();
    let yol = dizin.path().join("oturum.jsonl");
    let mut ayar = ayarlar();
    ayar.kayit_yolu = Some(yol.clone());
    let s = senaryo();
    let (mut d, kapi) = oturum(&s, ayar);
    d.gonder(mesaj()).await;
    sart_bekle(&mut d, 1).await;
    kapi.kabul_et(1).await.unwrap();
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    assert!(
        kayitlar
            .iter()
            .any(|z| z.yuk == KayitOgesi::SartKabulEdildi { surum: 1 })
    );
}
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa-core --test oturum_sart`
Expected: FAIL — `OturumBaglami::sart_kapisi` ve `KayitOgesi::SartKabulEdildi` yok.

- [ ] **Step 3: Kayıt öğesini ekle, komutu kaldır**

`crates/kalfa-core/src/kayit.rs` içinde `KayitOgesi` enum'unun sonuna (`TurBitti`'den sonra) ekle:

```rust
    /// Oturum, kullanıcının şart penceresinde kabul ettiği sürümle sürdü.
    SartKabulEdildi {
        surum: u32,
    },
```

`crates/kalfa-core/src/protokol.rs` içindeki `Komut` enum'undan şu varyantı sil:

```rust
    SartlariKabulEt {
        surum: u32,
    },
```

- [ ] **Step 4: Bağlama kapıyı ekle**

`crates/kalfa-core/src/oturum/mod.rs` içinde `use crate::saglayici::Saglayici;` satırından sonra ekle:

```rust
use crate::sart_kapisi::SartKapisi;
```

`OturumBaglami` yapısına alan ekle:

```rust
    /// Uygulama genelindeki tek şart kapısı (1c tasarımı §7.3). Bütün oturumlar
    /// aynı kapıyı paylaşır.
    pub sart_kapisi: Arc<SartKapisi>,
```

`genel_komut` içindeki şu kolu sil:

```rust
            Komut::SartlariKabulEt { .. } => {
                self.hata("Şu an bekleyen bir şart onayı yok.").await;
            }
```

- [ ] **Step 5: Oturumun şart akışını kapıya devret**

`crates/kalfa-core/src/oturum/model_turu.rs` içinde `use crate::protokol::{BitisNedeni, Komut, Olay};` satırını şununla değiştir (`Komut` bu dosyada artık kullanılmıyor):

```rust
use crate::protokol::{BitisNedeni, Olay};
```

ve `use crate::saglayici::SaglayiciAkisi;` satırından sonra ekle:

```rust
use crate::sart_kapisi::SartKarari;
```

`sart_akisi` işlevinin tamamını şununla değiştir:

```rust
    /// Şart kapısı (1c tasarımı §7.3, üst belge §11.8). Oturum kabul isteğini
    /// kendisi göndermez: uygulama genelindeki kapıya kaydolur, metni yayar ve
    /// kullanıcının şart penceresindeki kararını bekler.
    async fn sart_akisi(&mut self) -> Result<(), BitisNedeni> {
        let kapi = self.baglam.sart_kapisi.clone();
        let (metin, nesil) = match self
            .iptal_edilebilir(async move { kapi.gerekli().await })
            .await?
        {
            Ok(v) => v,
            Err(e) => {
                self.hata(e.to_string()).await;
                return Err(BitisNedeni::Hata);
            }
        };
        self.yay(Olay::SartOnayiGerekiyor {
            surum: metin.surum,
            metin: metin.metin,
        })
        .await;
        let kapi = self.baglam.sart_kapisi.clone();
        match self
            .iptal_edilebilir(async move { kapi.karar_bekle(nesil).await })
            .await?
        {
            SartKarari::Kabul { surum } => {
                self.kaydet(KayitOgesi::SartKabulEdildi { surum }).await;
                Ok(())
            }
            SartKarari::Ret => Err(BitisNedeni::Iptal),
        }
    }
```

- [ ] **Step 6: Test desteğini ve bağlam kuran testleri uyarla**

`crates/kalfa-core/src/test_destek.rs` içinde `use crate::saglayici::{Saglayici, SaglayiciAkisi};` satırından sonra ekle:

```rust
use crate::sart_kapisi::SartKapisi;
```

`test_baglami` gövdesini şununla değiştir:

```rust
    OturumBaglami {
        sart_kapisi: Arc::new(SartKapisi::yeni(saglayici.clone())),
        saglayici,
        araclar: Arc::new(AracKayitDefteri::yeni(araclar).expect("araç kaydı")),
        siniflandirici,
        kilitler: Arc::new(KaynakKilitleri::yeni()),
    }
```

`crates/kalfa-tools/tests/uctan_uca.rs` içinde içe aktarımı değiştir:

```rust
use kalfa_core::saglayici::{EvrenSaglayici, Saglayici};
use kalfa_core::sart_kapisi::SartKapisi;
```

`canli_evren_ile_dosya_okuyup_yanitlar` içindeki bağlam kurulumunu şununla değiştir:

```rust
    let saglayici: Arc<dyn Saglayici> = Arc::new(EvrenSaglayici::yeni(istemci));
    let baglam = OturumBaglami {
        sart_kapisi: Arc::new(SartKapisi::yeni(saglayici.clone())),
        saglayici,
        araclar: Arc::new(AracKayitDefteri::yeni(varsayilan_araclar(calisma_alani)).unwrap()),
        siniflandirici: o.siniflandirici.clone(),
        kilitler: Arc::new(kalfa_core::kilit::KaynakKilitleri::yeni()),
    };
```

`crates/kalfa-core/tests/oturum_metin.rs` içindeki `bosta_gecersiz_komutlar_hata_verir` testini şununla değiştir:

```rust
#[tokio::test]
async fn bosta_gecersiz_komutlar_hata_verir() {
    let s = SahteSaglayici::yeni(vec![]);
    let mut d = oturum(&s, vec![], ayarlar());
    d.gonder(Komut::AracOnayi {
        cagri: CagriId::yeni(),
        karar: OnayKarari::Onayla,
        muhur: OnayMuhru("x".into()),
    })
    .await;
    let z = d
        .bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("artık geçerli değil")))
        .await;
    assert!(z.tur.is_none());
}
```

`crates/kalfa-core/tests/protokol.rs` içindeki `butun_komutlar_gidip_gelir` listesinden şu satırı sil:

```rust
        Komut::SartlariKabulEt { surum: 2 },
```

- [ ] **Step 7: Testleri koştur**

Run: `cargo test --workspace --quiet`
Expected: PASS (`oturum_sart` 7 test dahil).

- [ ] **Step 8: Doğrulama ve commit**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: temiz.

```bash
git add crates/kalfa-core/src crates/kalfa-core/tests/oturum_sart.rs crates/kalfa-core/tests/oturum_metin.rs crates/kalfa-core/tests/protokol.rs crates/kalfa-tools/tests/uctan_uca.rs
git commit -m "kalfa-core: oturumlar şart kabulünü tek kapıdan bekler; kabul bir kez gider"
```

---

### Task 6: Kilit yöneticisi ve yol tablosu temizliği

Çalışma alanı kökünü dosya sistemi kimliğiyle tanıyıp aynı köke aynı `KaynakKilitleri`'ni veren yönetici (üst belge §6.1, 1c tasarımı §5.3). Ayrıca `KaynakKilitleri`'nin yol tablosu artık büyümez.

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/kalfa-core/Cargo.toml`
- Modify: `crates/kalfa-core/src/kilit.rs`
- Test: `crates/kalfa-core/tests/kilit_yoneticisi.rs`

**Interfaces:**
- Produces: `kilit::KilitYoneticisi` (`Default`): `yeni() -> Self`, `al(&self, kok: &Path) -> io::Result<Arc<KaynakKilitleri>>`, `acik_kok_sayisi(&self) -> usize`; `KaynakKilitleri::yol_tablosu_boyutu(&self) -> usize` (yalnızca `test-destek` özelliğiyle).

- [ ] **Step 1: Bağımlılığı ekle**

Kök `Cargo.toml` içindeki `[workspace.dependencies]` bölümüne, `regex` satırından sonra ekle:

```toml
same-file = "1.0.6"
```

`crates/kalfa-core/Cargo.toml` içindeki `[dependencies]` bölümüne ekle:

```toml
same-file.workspace = true
```

- [ ] **Step 2: Başarısız testleri yaz**

`crates/kalfa-core/tests/kilit_yoneticisi.rs`:

```rust
use std::sync::Arc;

use kalfa_core::kilit::{KaynakKilitleri, KilitYoneticisi};
use kalfa_core::yol::CalismaYolu;

#[test]
fn ayni_kok_ayni_kilitleri_paylasir() {
    let d = tempfile::tempdir().unwrap();
    let y = KilitYoneticisi::yeni();
    let a = y.al(d.path()).unwrap();
    let b = y.al(&d.path().join(".")).unwrap();
    assert!(Arc::ptr_eq(&a, &b));
}

#[test]
fn farkli_kokler_farkli_kilitler() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir(d.path().join("a")).unwrap();
    std::fs::create_dir(d.path().join("b")).unwrap();
    let y = KilitYoneticisi::yeni();
    let a = y.al(&d.path().join("a")).unwrap();
    let b = y.al(&d.path().join("b")).unwrap();
    assert!(!Arc::ptr_eq(&a, &b));
}

#[cfg(windows)]
#[test]
fn buyuk_kucuk_harf_farki_ayni_koku_verir() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir(d.path().join("Proje")).unwrap();
    let y = KilitYoneticisi::yeni();
    let a = y.al(&d.path().join("Proje")).unwrap();
    let b = y.al(&d.path().join("PROJE")).unwrap();
    assert!(Arc::ptr_eq(&a, &b));
}

#[cfg(unix)]
#[test]
fn sembolik_baglanti_ayni_koku_verir() {
    let d = tempfile::tempdir().unwrap();
    let hedef = d.path().join("proje");
    std::fs::create_dir(&hedef).unwrap();
    let bag = d.path().join("bag");
    std::os::unix::fs::symlink(&hedef, &bag).unwrap();
    let y = KilitYoneticisi::yeni();
    assert!(Arc::ptr_eq(&y.al(&hedef).unwrap(), &y.al(&bag).unwrap()));
}

#[cfg(windows)]
#[test]
fn junction_ayni_koku_verir() {
    let d = tempfile::tempdir().unwrap();
    let hedef = d.path().join("proje");
    std::fs::create_dir(&hedef).unwrap();
    let bag = d.path().join("bag");
    // Yalnızca bu testin geçici dizinindeki iki yol; kullanıcı girdisi yok.
    let cikti = std::process::Command::new("cmd.exe")
        .args(["/D", "/C", "mklink", "/J"])
        .arg(&bag)
        .arg(&hedef)
        .output()
        .unwrap();
    assert!(cikti.status.success(), "junction kurulamadı: {cikti:?}");
    let y = KilitYoneticisi::yeni();
    assert!(Arc::ptr_eq(&y.al(&hedef).unwrap(), &y.al(&bag).unwrap()));
}

#[test]
fn son_oturum_birakinca_kok_silinir() {
    let d = tempfile::tempdir().unwrap();
    let y = KilitYoneticisi::yeni();
    let a = y.al(d.path()).unwrap();
    assert_eq!(y.acik_kok_sayisi(), 1);
    drop(a);
    assert_eq!(y.acik_kok_sayisi(), 0);
}

#[tokio::test]
async fn yol_tablosu_buyumez() {
    let k = KaynakKilitleri::yeni();
    for i in 0..100 {
        let yol = CalismaYolu::coz(&format!("d{i}.txt")).unwrap();
        let _set = k.al(&[], &[yol], false).await;
    }
    assert!(k.yol_tablosu_boyutu() <= 1);
}
```

- [ ] **Step 3: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa-core --test kilit_yoneticisi`
Expected: FAIL — `KilitYoneticisi` ve `yol_tablosu_boyutu` yok.

- [ ] **Step 4: Yöneticiyi ve tablo temizliğini yaz**

`crates/kalfa-core/src/kilit.rs` dosyasının başındaki modül açıklamasını ve içe aktarımları şununla değiştir:

```rust
//! Yol bazında okuma/yazma kilitleri ve çalışma alanı başına kilit yöneticisi
//! (spec §6.1, 1c tasarımı §5.3).
//!
//! Önce genel kilit (paylaşımlı ya da özel), sonra yol kilitleri anahtar
//! sırasıyla alınır. Sıralı edinim kilitlenmeyi (deadlock) önler.

use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, Weak};

use same_file::Handle;
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};

use crate::yol::CalismaYolu;
```

`KaynakKilitleri::kilit` işlevini şununla değiştir ve hemen ardından test yardımcısını ekle:

```rust
    fn kilit(&self, anahtar: &str) -> Arc<RwLock<()>> {
        let mut harita = self.yollar.lock().unwrap_or_else(|z| z.into_inner());
        // Kimsenin tutmadığı ve beklemediği kilitler atılır; tablo büyümez.
        harita.retain(|_, kilit| Arc::strong_count(kilit) > 1);
        harita.entry(anahtar.to_string()).or_default().clone()
    }

    /// Yol kilidi tablosundaki girdi sayısı (testler için).
    #[cfg(feature = "test-destek")]
    pub fn yol_tablosu_boyutu(&self) -> usize {
        self.yollar.lock().unwrap_or_else(|z| z.into_inner()).len()
    }
```

Dosyanın sonuna ekle:

```rust
/// Çalışma alanı kökünü dosya sistemi kimliğiyle tanır (Windows'ta birim seri
/// numarası ve dosya dizini, Linux'ta aygıt ve inode) ve aynı köke aynı
/// kilitleri verir. Kök farklı yazılmış olsa da (büyük/küçük harf, junction,
/// sembolik bağlantı) kimlik aynıdır. Kilitler süreç içidir; iç içe iki
/// çalışma alanı ayrı yönetici alır (bilinen sınır).
#[derive(Default)]
pub struct KilitYoneticisi {
    kokler: Mutex<Vec<(Handle, Weak<KaynakKilitleri>)>>,
}

impl KilitYoneticisi {
    pub fn yeni() -> Self {
        Self::default()
    }

    /// `kok` dizininin kilitleri. Aynı kökü açan bütün oturumlar aynı
    /// kilitleri paylaşır; son oturum bırakınca kökün kaydı silinir.
    pub fn al(&self, kok: &Path) -> io::Result<Arc<KaynakKilitleri>> {
        let kimlik = Handle::from_path(kok)?;
        let mut kokler = self.kokler.lock().unwrap_or_else(|z| z.into_inner());
        kokler.retain(|(_, kilitler)| kilitler.strong_count() > 0);
        let mevcut = kokler
            .iter()
            .find(|(h, _)| *h == kimlik)
            .and_then(|(_, kilitler)| kilitler.upgrade());
        if let Some(kilitler) = mevcut {
            return Ok(kilitler);
        }
        let kilitler = Arc::new(KaynakKilitleri::yeni());
        kokler.push((kimlik, Arc::downgrade(&kilitler)));
        Ok(kilitler)
    }

    /// Hâlâ kullanılan kök sayısı.
    pub fn acik_kok_sayisi(&self) -> usize {
        let kokler = self.kokler.lock().unwrap_or_else(|z| z.into_inner());
        kokler
            .iter()
            .filter(|(_, kilitler)| kilitler.strong_count() > 0)
            .count()
    }
}
```

- [ ] **Step 5: Testleri koştur**

Run: `cargo test -p kalfa-core --test kilit_yoneticisi --test kilit`
Expected: PASS (Windows'ta 6, Linux'ta 5 yeni test; mevcut `kilit` testleri de geçer).

- [ ] **Step 6: Doğrulama ve commit**

Run: `cargo test --workspace --quiet && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: temiz.

```bash
git add Cargo.toml Cargo.lock crates/kalfa-core/Cargo.toml crates/kalfa-core/src/kilit.rs crates/kalfa-core/tests/kilit_yoneticisi.rs
git commit -m "kalfa-core: dosya sistemi kimliğiyle çalışma alanı kilit yöneticisi"
```

---

### Task 7: Kayıt kilidi (tek yazıcı) ve oturumun kapanmasını bekleme

Her kaydın yanındaki `<kayıt>.kilit` dosyasında işletim sistemi kilidi tutulur; kilit alınamazsa `ResourceBusy` döner (1c tasarımı §5.2). Kilit kaydın kendisine konmaz: Windows'ta dosya kilidi aynı süreçteki başka tutamaçlardan yapılan okumaları da engeller. Oturum kapatılıp aynı kayıt hemen yeniden açılabilsin diye tutamaç, oturum görevinin bitmesini beklemeyi sağlar.

**Files:**
- Modify: `crates/kalfa-core/src/kayit.rs`
- Modify: `crates/kalfa-core/src/oturum/mod.rs`
- Modify: `crates/kalfa-core/src/test_destek.rs`
- Test: `crates/kalfa-core/tests/kayit_kilidi.rs`

**Interfaces:**
- Produces: `kayit::kilit_yolu(&Path) -> PathBuf`; `OturumKaydi::devam_icin_ac(&Path) -> io::Result<(OturumKaydi, Vec<Zarf<KayitOgesi>>)>`; `OturumKaydi::ac` ve `olustur` artık kilit alır; `OturumTutamaci::kapanmasini_bekle(&self)`; `Surucu::kapat(self)`.
- Hata: kilit başkasındayken `io::ErrorKind::ResourceBusy`; `devam_et` bunu `OturumHatasi::Kayit(e)` olarak döner.

- [ ] **Step 1: Başarısız testleri yaz**

`crates/kalfa-core/tests/kayit_kilidi.rs`:

```rust
use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;

use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi, kilit_yolu};
use kalfa_core::kimlik::OturumId;
use kalfa_core::oturum::{OturumAyarlari, OturumHatasi, devam_et};
use kalfa_core::protokol::{BitisNedeni, Komut, SEMA_SURUMU, Zarf};
use kalfa_core::test_destek::*;

const COCUK_DEGISKENI: &str = "KALFA_KAYIT_KILIDI_COCUK";

fn kip_kaydi() -> Zarf<KayitOgesi> {
    Zarf {
        sema: SEMA_SURUMU,
        oturum: OturumId::yeni(),
        tur: None,
        sira: 1,
        yuk: KayitOgesi::KipDegisti { kip: Kip::Proje },
    }
}

#[test]
fn ayni_surecte_ikinci_yazici_acilamaz() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("o.jsonl");
    let ilk = OturumKaydi::olustur(&yol).unwrap();
    let hata = OturumKaydi::ac(&yol)
        .err()
        .expect("ikinci yazıcı açılmamalı");
    assert_eq!(hata.kind(), ErrorKind::ResourceBusy);
    drop(ilk);
    OturumKaydi::ac(&yol).expect("ilk yazıcı kapanınca açılabilmeli");
}

#[test]
fn kilit_tutulurken_kayit_okunabilir() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("o.jsonl");
    let mut k = OturumKaydi::olustur(&yol).unwrap();
    k.yaz(&kip_kaydi()).unwrap();
    assert_eq!(OturumKaydi::oku(&yol).unwrap().len(), 1);
}

#[test]
fn olmayan_kayit_icin_kilit_dosyasi_birakilmaz() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("yok.jsonl");
    let hata = OturumKaydi::devam_icin_ac(&yol)
        .err()
        .expect("olmayan kayıt açılmamalı");
    assert_eq!(hata.kind(), ErrorKind::NotFound);
    assert!(!kilit_yolu(&yol).exists());
}

#[test]
fn baska_surecte_ikinci_yazici_acilamaz() {
    if let Ok(yol) = std::env::var(COCUK_DEGISKENI) {
        // Çocuk süreç: kilit ebeveynde olduğu için açılamamalı.
        let kod = match OturumKaydi::ac(Path::new(&yol)) {
            Err(e) if e.kind() == ErrorKind::ResourceBusy => 0,
            _ => 1,
        };
        std::process::exit(kod);
    }
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("o.jsonl");
    let _kayit = OturumKaydi::olustur(&yol).unwrap();
    let durum = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "baska_surecte_ikinci_yazici_acilamaz",
            "--test-threads=1",
        ])
        .env(COCUK_DEGISKENI, &yol)
        .status()
        .unwrap();
    assert!(durum.success(), "başka süreçteki ikinci yazıcı reddedilmeli");
}

#[tokio::test]
async fn oturum_kapaninca_kayit_yeniden_acilabilir() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("oturum.jsonl");
    let mut ayar = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    ayar.kayit_yolu = Some(yol.clone());
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("tamam")]);
    let mut surucu = test_oturumu(
        test_baglami(s.clone(), vec![], sanal_siniflandirici()),
        ayar.clone(),
    );
    surucu
        .gonder(Komut::KullaniciMesaji {
            metin: "selam".into(),
        })
        .await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let acikken = devam_et(
        &yol,
        ayar.clone(),
        test_baglami(s.clone(), vec![], sanal_siniflandirici()),
    );
    assert!(
        matches!(acikken, Err(OturumHatasi::Kayit(e)) if e.kind() == ErrorKind::ResourceBusy),
        "açık oturumun kaydına ikinci yazıcı açılmaz"
    );

    surucu.kapat().await;
    let (_tutamac, _alici, durum) =
        devam_et(&yol, ayar, test_baglami(s, vec![], sanal_siniflandirici()))
            .expect("kapanan oturumun kaydı yeniden açılmalı");
    assert_eq!(durum.model, "glm-5.3");
}
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa-core --test kayit_kilidi`
Expected: FAIL — `kilit_yolu`, `devam_icin_ac` ve `Surucu::kapat` yok.

- [ ] **Step 3: Kayda kilidi ekle**

`crates/kalfa-core/src/kayit.rs` içinde modül açıklamasının sonuna şu paragrafı ekle:

```rust
//!
//! Aynı kayda aynı anda tek yazıcı açılır: kaydın yanındaki `<kayıt>.kilit`
//! dosyasında işletim sistemi kilidi tutulur (1c tasarımı §5.2). Kilit kaydın
//! kendisine konmaz; Windows'ta dosya kilidi aynı süreçteki okumaları da
//! engellerdi.
```

İçe aktarımları değiştir:

```rust
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
```

`OturumKaydi` yapısını ve `olustur`, `ac`, `hedefle` işlevlerini şununla değiştir (`yaz`, `yaz_kalici`, `oku` aynen kalır):

```rust
pub struct OturumKaydi {
    hedef: Box<dyn KayitHedefi>,
    /// Kaydın yanındaki kilit dosyası. Tutulduğu sürece aynı kayda ikinci bir
    /// yazıcı açılamaz; bırakılınca (drop) kilit çözülür.
    _kilit: Option<File>,
}

impl OturumKaydi {
    /// Yeni kayıt oluşturur. Dosya zaten varsa ya da kayıt başka bir
    /// yazıcıda açıksa hata döner.
    pub fn olustur(yol: &Path) -> io::Result<Self> {
        if let Some(ust) = yol.parent() {
            fs::create_dir_all(ust)?;
        }
        let kilit = kilitle(yol)?;
        let mut secenek = OpenOptions::new();
        secenek.create_new(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            secenek.mode(0o600);
        }
        Ok(Self {
            hedef: Box::new(secenek.open(yol)?),
            _kilit: Some(kilit),
        })
    }

    /// Var olan kaydı devam etmek üzere açar ve onarır (bkz. `devam_icin_ac`).
    pub fn ac(yol: &Path) -> io::Result<Self> {
        Self::devam_icin_ac(yol).map(|(kayit, _)| kayit)
    }

    /// Önce kilidi alır, sonra kaydı okur ve onarır: çökmede yarım kalan son
    /// satır kesilir, son geçerli satırın satır sonu eksikse eklenir. Kilit
    /// okumadan önce alındığı için arada başka bir yazıcı ekleme yapamaz.
    pub fn devam_icin_ac(yol: &Path) -> io::Result<(Self, Vec<Zarf<KayitOgesi>>)> {
        // Olmayan kayıt için kilit dosyası bırakmamak için önce varlığı sına.
        fs::metadata(yol)?;
        let kilit = kilitle(yol)?;
        let veri = fs::read(yol)?;
        let (kayitlar, gecerli) = coz(&veri)?;
        let mut dosya = OpenOptions::new().read(true).write(true).open(yol)?;
        dosya.set_len(gecerli as u64)?;
        dosya.seek(SeekFrom::End(0))?;
        if gecerli > 0 && veri[gecerli - 1] != b'\n' {
            dosya.write_all(b"\n")?;
        }
        dosya.sync_data()?;
        let kayit = Self {
            hedef: Box::new(dosya),
            _kilit: Some(kilit),
        };
        Ok((kayit, kayitlar))
    }

    pub fn hedefle(hedef: Box<dyn KayitHedefi>) -> Self {
        Self {
            hedef,
            _kilit: None,
        }
    }
```

Dosyanın sonuna (coz'dan önce ya da sonra) ekle:

```rust
/// Kaydın kilit dosyasının yolu: `<kayıt>.kilit`.
pub fn kilit_yolu(kayit: &Path) -> PathBuf {
    let mut ad = kayit.as_os_str().to_owned();
    ad.push(".kilit");
    PathBuf::from(ad)
}

/// Kayıt kilidini alır; kilit başka bir yazıcıdaysa `ResourceBusy` döner.
fn kilitle(kayit: &Path) -> io::Result<File> {
    let mut secenek = OpenOptions::new();
    secenek.create(true).write(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        secenek.mode(0o600);
    }
    let dosya = secenek.open(kilit_yolu(kayit))?;
    match dosya.try_lock() {
        Ok(()) => Ok(dosya),
        Err(TryLockError::WouldBlock) => Err(io::Error::new(
            io::ErrorKind::ResourceBusy,
            "Bu oturum kaydı başka bir yerde açık.",
        )),
        Err(TryLockError::Error(e)) => Err(e),
    }
}
```

- [ ] **Step 4: Kayıttan devamda kilit okumadan önce alınsın; kapanış bildirilsin**

`crates/kalfa-core/src/oturum/mod.rs` içinde:

İçe aktarımı değiştir:

```rust
use tokio::sync::{mpsc, watch};
```

`OturumTutamaci` yapısını ve bloğunu şununla değiştir (mevcut yöntemler aynen kalır, yenisi eklenir):

```rust
#[derive(Clone)]
pub struct OturumTutamaci {
    oturum: OturumId,
    tx: mpsc::Sender<Komut>,
    aktif_tur: AktifTur,
    kapanis: watch::Receiver<bool>,
}
```

`impl OturumTutamaci` bloğuna ekle:

```rust
    /// Oturum görevi bitene kadar bekler. Döndüğünde oturumun kaydı ve kayıt
    /// kilidi bırakılmıştır; kayıt aynı süreçte yeniden açılabilir. Oturumu
    /// kapatmak için önce olay alıcısı bırakılır.
    pub async fn kapanmasini_bekle(&self) {
        let mut kapanis = self.kapanis.clone();
        let _ = kapanis.wait_for(|kapandi| *kapandi).await;
    }
```

`devam_et` içinde şu iki satırı:

```rust
    let durum = kurtar(&OturumKaydi::oku(kayit_yolu)?)?;
    let kayit = OturumKaydi::ac(kayit_yolu)?;
```

şununla değiştir:

```rust
    let (kayit, kayitlar) = OturumKaydi::devam_icin_ac(kayit_yolu)?;
    let durum = kurtar(&kayitlar)?;
```

`Oturum::kur` işlevinin tamamını şununla değiştir (alan listesi aynı; imza, `kapanis` kanalı ve dönüş değeri değişir):

```rust
    fn kur(
        kimlik: OturumId,
        ayarlar: OturumAyarlari,
        baglam: OturumBaglami,
        kayit: Option<OturumKaydi>,
        mesajlar: Vec<ChatMessage>,
        sira: u64,
    ) -> (Self, OturumTutamaci, OlayAlici, watch::Sender<bool>) {
        let (komut_tx, komut_rx) = mpsc::channel(KOMUT_KANALI);
        let (kapanis_tx, kapanis_rx) = watch::channel(false);
        let aktif_tur: AktifTur = Arc::new(Mutex::new(None));
        let (olay_tx, olay_rx) = OlayKanali::yeni(ayarlar.olay_butcesi, aktif_tur.clone());
        let olay_kapanma = olay_tx.kapanma();
        let oturum = Self {
            kimlik,
            ayarlar,
            baglam,
            komutlar: komut_rx,
            olaylar: olay_tx,
            olay_kapanma,
            kayit,
            kayit_hatasi: None,
            sira,
            mesajlar,
            tur: None,
            iptal: CancellationToken::new(),
            aktif_tur: aktif_tur.clone(),
            politika: Arc::new(AtomicU64::new(0)),
            kapandi: false,
        };
        (
            oturum,
            OturumTutamaci {
                oturum: kimlik,
                tx: komut_tx,
                aktif_tur,
                kapanis: kapanis_rx,
            },
            olay_rx,
            kapanis_tx,
        )
    }
```

`baslat_ic` içindeki kurulum ve görevi şununla değiştir:

```rust
    let (mut oturum, tutamac, alici, kapanis) =
        Oturum::kur(OturumId::yeni(), ayarlar, baglam, kayit, Vec::new(), 0);
    tokio::spawn(async move {
        let acilis = KayitOgesi::OturumAcildi {
            model: oturum.ayarlar.model.clone(),
            kip: oturum.ayarlar.kip,
        };
        oturum.kaydet(acilis).await;
        oturum.calis().await;
        // `calis` oturumu tüketti: kayıt ve kayıt kilidi bırakıldı.
        let _ = kapanis.send(true);
    });
    (tutamac, alici)
```

`devam_et` içinde `Oturum::kur(...)` çağrısının sonucunu `let (mut oturum, tutamac, alici, kapanis) = Oturum::kur(` diye al ve görevin sonunda `oturum.calis().await;` satırından sonra ekle:

```rust
        let _ = kapanis.send(true);
```

- [ ] **Step 5: Test sürücüsüne kapatmayı ekle**

`crates/kalfa-core/src/test_destek.rs` içinde `impl Surucu` bloğuna ekle:

```rust
    /// Olay alıcısını bırakır ve oturum görevi bitene kadar bekler; ardından
    /// kayıt yeniden açılabilir.
    pub async fn kapat(self) {
        let Surucu {
            tutamac, olaylar, ..
        } = self;
        drop(olaylar);
        tokio::time::timeout(Duration::from_secs(5), tutamac.kapanmasini_bekle())
            .await
            .expect("oturum kapanmalı");
    }
```

- [ ] **Step 6: Testleri koştur**

Run: `cargo test -p kalfa-core --test kayit_kilidi --test kayit_kurtarma --test oturum_kurtarma`
Expected: PASS.

- [ ] **Step 7: Doğrulama ve commit**

Run: `cargo test --workspace --quiet && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: temiz.

```bash
git add crates/kalfa-core/src/kayit.rs crates/kalfa-core/src/oturum/mod.rs crates/kalfa-core/src/test_destek.rs crates/kalfa-core/tests/kayit_kilidi.rs
git commit -m "kalfa-core: kayıt başına tek yazıcı; oturumun kapanmasını bekleme"
```

---

### Task 8: Çalışma alanı, yönlendirilen model ve kullanım kaydı

Kayıttan açılabilmek için `OturumAcildi` çalışma alanı yolunu taşır. Durum çubuğu için yönlendirilen model olay olarak yayılır; yönlendirilen model ve token kullanımı kayda da yazılır (1c tasarımı §3.3, §8.3).

**Files:**
- Modify: `crates/kalfa-core/src/kayit.rs`
- Modify: `crates/kalfa-core/src/kurtarma.rs`
- Modify: `crates/kalfa-core/src/protokol.rs`
- Modify: `crates/kalfa-core/src/oturum/mod.rs`
- Modify: `crates/kalfa-core/src/oturum/model_turu.rs`
- Modify: `crates/kalfa-core/src/test_destek.rs`
- Modify: `crates/kalfa-core/tests/{oturum_kurtarma,kayit_kurtarma,oturum_metin,protokol}.rs`
- Test: `crates/kalfa-core/tests/kayit_ayrintilari.rs`

**Interfaces:**
- Produces: `KayitOgesi::OturumAcildi { model, kip, calisma_alani: Option<PathBuf> }`; `KayitOgesi::Istek { amac, istek_id, kredi, yonlendirilen_model: Option<String> }`; `KayitOgesi::Kullanim { istem: u64, tamamlama: u64, dusunce: u64 }`; `Olay::IstekBilgisi { istek_id: Option<String>, yonlendirilen_model: Option<String> }`; `OturumAyarlari::calisma_alani: Option<PathBuf>`; `KurtarilmisDurum::calisma_alani: Option<PathBuf>`; `oturum::baslat_kimlikle(kimlik: OturumId, ayarlar: OturumAyarlari, baglam: OturumBaglami) -> Result<(OturumTutamaci, OlayAlici), OturumHatasi>`.
- Yeni alanlar `#[serde(default, skip_serializing_if = "Option::is_none")]`: alanı olmayan eski kayıtlar okunur.
- `baslat_kimlikle`: uygulama kayıt dosyasına oturum kimliğini ad olarak verir (1c tasarımı §5.1). Bu yüzden kimliği oturumdan önce üretmesi gerekir. `baslat` artık `baslat_kimlikle(OturumId::yeni(), …)` çağırır.

- [ ] **Step 1: Başarısız testleri yaz**

`crates/kalfa-core/tests/kayit_ayrintilari.rs`:

```rust
use std::path::{Path, PathBuf};

use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::OturumId;
use kalfa_core::kurtarma::kurtar;
use kalfa_core::oturum::{OturumAyarlari, baslat_kimlikle};
use kalfa_core::protokol::{BitisNedeni, Komut, Olay, Zarf};
use kalfa_core::test_destek::*;
use kalfa_evren::AkisOlayi;
use kalfa_evren::tipler::{CompletionTokensDetails, Usage};

fn kayitli(dizin: &Path) -> (OturumAyarlari, PathBuf) {
    let yol = dizin.join("oturum.jsonl");
    let mut ayar = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    ayar.kayit_yolu = Some(yol.clone());
    (ayar, yol)
}

fn selam() -> Komut {
    Komut::KullaniciMesaji {
        metin: "selam".into(),
    }
}

#[tokio::test]
async fn acilis_kaydi_calisma_alanini_tasir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![]);
    let surucu = test_oturumu(test_baglami(s, vec![], sanal_siniflandirici()), ayar);
    surucu.kapat().await;
    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    assert!(matches!(
        &kayitlar[0].yuk,
        KayitOgesi::OturumAcildi { calisma_alani: Some(a), .. } if a == Path::new("calisma")
    ));
    assert_eq!(
        kurtar(&kayitlar).unwrap().calisma_alani,
        Some(PathBuf::from("calisma"))
    );
}

#[test]
fn calisma_alanisiz_eski_acilis_kaydi_okunur() {
    let satir = r#"{"sema":1,"oturum":"6f1d0a4e-2b8c-4c1e-9a55-0d6b1f3e2a10","tur":null,"sira":1,"yuk":{"tip":"OturumAcildi","model":"glm-5.3","kip":"Siki"}}"#;
    let zarf: Zarf<KayitOgesi> = serde_json::from_str(satir).unwrap();
    assert!(matches!(
        zarf.yuk,
        KayitOgesi::OturumAcildi {
            calisma_alani: None,
            ..
        }
    ));
}

#[tokio::test]
async fn yonlendirilen_model_olay_olarak_yayilir_ve_kayda_yazilir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let s = SahteSaglayici::yeni(vec![SahteYanit::metin("tamam")]);
    let mut surucu = test_oturumu(test_baglami(s, vec![], sanal_siniflandirici()), ayar);
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert!(surucu.gecmis.iter().any(|z| matches!(
        &z.yuk,
        Olay::IstekBilgisi { yonlendirilen_model: Some(m), .. } if m == "glm-5.3"
    )));
    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    assert!(kayitlar.iter().any(|z| matches!(
        &z.yuk,
        KayitOgesi::Istek { yonlendirilen_model: Some(m), .. } if m == "glm-5.3"
    )));
}

#[tokio::test]
async fn token_kullanimi_kayda_yazilir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path());
    let kullanim = Usage {
        prompt_tokens: 10,
        completion_tokens: 5,
        total_tokens: 15,
        completion_tokens_details: Some(CompletionTokensDetails {
            reasoning_tokens: 2,
        }),
        evren: None,
    };
    let s = SahteSaglayici::yeni(vec![SahteYanit::Akis(vec![
        Ok(AkisOlayi::Metin("tamam".into())),
        Ok(AkisOlayi::Kullanim(kullanim)),
        Ok(AkisOlayi::Bitti {
            neden: "stop".into(),
        }),
    ])]);
    let mut surucu = test_oturumu(test_baglami(s, vec![], sanal_siniflandirici()), ayar);
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    assert!(kayitlar.iter().any(|z| z.yuk
        == KayitOgesi::Kullanim {
            istem: 10,
            tamamlama: 5,
            dusunce: 2
        }));
}

#[tokio::test]
async fn verilen_kimlikle_baslar() {
    let d = tempfile::tempdir().unwrap();
    let kimlik = OturumId::yeni();
    let yol = d.path().join(format!("{kimlik}.jsonl"));
    let mut ayar = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    ayar.kayit_yolu = Some(yol.clone());
    let s = SahteSaglayici::yeni(vec![]);
    let (tutamac, alici) =
        baslat_kimlikle(kimlik, ayar, test_baglami(s, vec![], sanal_siniflandirici())).unwrap();
    assert_eq!(tutamac.oturum_id(), kimlik);
    drop(alici);
    tokio::time::timeout(std::time::Duration::from_secs(5), tutamac.kapanmasini_bekle())
        .await
        .expect("oturum kapanmalı");
    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    assert_eq!(kayitlar[0].oturum, kimlik);
}
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa-core --test kayit_ayrintilari`
Expected: FAIL — yeni alanlar, `Kullanim`, `IstekBilgisi` ve `baslat_kimlikle` yok.

- [ ] **Step 3: Kayıt öğelerini ve olayı genişlet**

`crates/kalfa-core/src/kayit.rs` içinde içe aktarım satırına `PathBuf` zaten eklendi (Görev 7). `KayitOgesi` içindeki `OturumAcildi` ve `Istek` varyantlarını şununla değiştir ve `Kullanim`'ı ekle:

```rust
    OturumAcildi {
        model: String,
        kip: Kip,
        /// Oturumun açıldığı çalışma alanı; kayıttan açarken araçlar buna göre
        /// kurulur. Bu alanı olmayan eski kayıtlar yalnızca geçmiş gösterir.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        calisma_alani: Option<PathBuf>,
    },
```

```rust
    Istek {
        amac: String,
        istek_id: Option<String>,
        kredi: Option<KrediGozlemi>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        yonlendirilen_model: Option<String>,
    },
```

```rust
    /// Bir model isteğinin token kullanımı; kayıttan açılan oturumun durum
    /// çubuğu bundan toplanır.
    Kullanim {
        istem: u64,
        tamamlama: u64,
        dusunce: u64,
    },
```

`crates/kalfa-core/src/protokol.rs` içinde `Olay` enum'una `KrediGozlemi` varyantından önce ekle:

```rust
    /// Bir EVREN isteğinin kimliği ve yönlendirilen model (üst belge §6.4).
    IstekBilgisi {
        istek_id: Option<String>,
        yonlendirilen_model: Option<String>,
    },
```

- [ ] **Step 4: Çalışma alanını ayarlardan kayda ve kurtarmaya taşı**

`crates/kalfa-core/src/oturum/mod.rs` içinde `OturumAyarlari` yapısına alan ekle:

```rust
    /// Çalışma alanının kökü; açılış kaydına yazılır.
    pub calisma_alani: Option<PathBuf>,
```

`OturumAyarlari::yeni` içindeki yapı ifadesine ekle:

```rust
            calisma_alani: Some(calisma_alani.to_path_buf()),
```

`baslat_ic` içindeki açılış kaydını değiştir:

```rust
        let acilis = KayitOgesi::OturumAcildi {
            model: oturum.ayarlar.model.clone(),
            kip: oturum.ayarlar.kip,
            calisma_alani: oturum.ayarlar.calisma_alani.clone(),
        };
```

Kimliği dışarıdan alan başlatmayı ekle. `baslat` işlevini şununla değiştir ve hemen altına `baslat_kimlikle`'yi ekle:

```rust
/// Yeni bir oturum başlatır. Bir tokio çalışma zamanı içinde çağrılmalıdır.
pub fn baslat(
    ayarlar: OturumAyarlari,
    baglam: OturumBaglami,
) -> Result<(OturumTutamaci, OlayAlici), OturumHatasi> {
    baslat_kimlikle(OturumId::yeni(), ayarlar, baglam)
}

/// Verilen kimlikle yeni bir oturum başlatır. Uygulama kayıt dosyasına oturum
/// kimliğini ad olarak verir (1c tasarımı §5.1); bu yüzden kimliği oturumdan
/// önce üretir.
pub fn baslat_kimlikle(
    kimlik: OturumId,
    ayarlar: OturumAyarlari,
    baglam: OturumBaglami,
) -> Result<(OturumTutamaci, OlayAlici), OturumHatasi> {
    let kayit = match &ayarlar.kayit_yolu {
        Some(yol) => Some(OturumKaydi::olustur(yol)?),
        None => None,
    };
    Ok(baslat_ic(kimlik, ayarlar, baglam, kayit))
}
```

`baslat_kayitla` içindeki çağrıyı `baslat_ic(OturumId::yeni(), ayarlar, baglam, Some(kayit))` yap. `baslat_ic`'in imzasına ilk parametre olarak `kimlik: OturumId` ekle ve içindeki `Oturum::kur(OturumId::yeni(), …)` çağrısında `OturumId::yeni()` yerine `kimlik` kullan.

`crates/kalfa-core/src/kurtarma.rs` içinde `use std::collections::{HashMap, HashSet};` satırından sonra ekle:

```rust
use std::path::PathBuf;
```

`KurtarilmisDurum` yapısına `kip` alanından sonra ekle:

```rust
    pub calisma_alani: Option<PathBuf>,
```

`kurtar` içinde açılışı okuyan satırı değiştir:

```rust
    let KayitOgesi::OturumAcildi {
        model,
        kip,
        calisma_alani,
    } = &ilk.yuk
    else {
        return Err(KurtarmaHatasi::AcilisYok);
    };
```

ve dönen yapıya `kip,` satırından sonra ekle:

```rust
        calisma_alani: calisma_alani.clone(),
```

- [ ] **Step 5: Yönlendirilen modeli ve kullanımı yay/yaz**

`crates/kalfa-core/src/oturum/model_turu.rs` içinde `istek_kaydet` işlevini şununla değiştir:

```rust
    pub(super) async fn istek_kaydet(&mut self, amac: &str, meta: &YanitMeta) {
        let oge = KayitOgesi::Istek {
            amac: amac.to_string(),
            istek_id: meta.istek_id.clone(),
            kredi: meta.kredi.clone(),
            yonlendirilen_model: meta.yonlendirilen_model.clone(),
        };
        self.kaydet(oge).await;
        self.yay(Olay::IstekBilgisi {
            istek_id: meta.istek_id.clone(),
            yonlendirilen_model: meta.yonlendirilen_model.clone(),
        })
        .await;
        if let Some(gozlem) = meta.kredi.clone() {
            self.yay(Olay::KrediGozlemi { gozlem }).await;
        }
    }
```

`model_turu` içindeki kullanım kolunu şununla değiştir:

```rust
                    Some(Ok(AkisOlayi::Kullanim(u))) => {
                        let (istem, tamamlama, dusunce) =
                            (u.prompt_tokens, u.completion_tokens, u.reasoning_tokens());
                        self.kaydet(KayitOgesi::Kullanim {
                            istem,
                            tamamlama,
                            dusunce,
                        })
                        .await;
                        self.yay(Olay::TokenKullanimi {
                            istem,
                            tamamlama,
                            dusunce,
                        })
                        .await;
                    }
```

`crates/kalfa-core/src/test_destek.rs` içinde `SahteSaglayici::akis` başını şununla değiştir:

```rust
    async fn akis(&self, istek: ChatRequest) -> Result<SaglayiciAkisi, EvrenError> {
        let model = istek.model.clone();
        let sira = {
            let mut istekler = self.istekler.lock().unwrap();
            istekler.push(istek);
            istekler.len()
        };
```

ve aynı işlevdeki `meta`'yı şununla değiştir:

```rust
        let meta = YanitMeta {
            istek_id: Some(format!("sahte-{sira}")),
            yonlendirilen_model: Some(model),
            ..Default::default()
        };
```

- [ ] **Step 6: Mevcut testleri yeni alanlara uyarla**

`crates/kalfa-core/tests/oturum_kurtarma.rs` içinde `yarim_kayit`'taki açılış kaydını değiştir:

```rust
        KayitOgesi::OturumAcildi {
            model: "glm-5.3".into(),
            kip: Kip::Serbest,
            calisma_alani: None,
        },
```

`crates/kalfa-core/tests/kayit_kurtarma.rs` içindeki `acilis`'i değiştir:

```rust
fn acilis(kip: Kip) -> KayitOgesi {
    KayitOgesi::OturumAcildi {
        model: "glm-5.3".into(),
        kip,
        calisma_alani: None,
    }
}
```

`crates/kalfa-core/tests/oturum_metin.rs` içindeki `oturum_kaydi_yazilir` testinde açılış eşleşmesini değiştir:

```rust
        matches!(&k[0].yuk, KayitOgesi::OturumAcildi { model, kip: Kip::Siki, .. } if model == "glm-5.3")
```

`crates/kalfa-core/tests/protokol.rs` içindeki `butun_olaylar_gidip_gelir` listesine `Olay::KrediGozlemi { … }` öğesinden önce ekle:

```rust
        Olay::IstekBilgisi {
            istek_id: Some("r".into()),
            yonlendirilen_model: Some("glm-5.3".into()),
        },
```

- [ ] **Step 7: Testleri koştur**

Run: `cargo test --workspace --quiet`
Expected: PASS (`kayit_ayrintilari` 5 test dahil).

- [ ] **Step 8: Doğrulama ve commit**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: temiz.

```bash
git add crates/kalfa-core/src crates/kalfa-core/tests
git commit -m "kalfa-core: kayıtta çalışma alanı, yönlendirilen model ve token kullanımı; kimlikle başlatma"
```

---

### Task 9: Geçmiş görünümü, sayfa ve durum özeti

Kayıttan açılan oturumun ekrandaki geçmişi kayıt öğelerinden üretilir; her öğe kayıttaki `sira`'yı taşır. Sayfalama tur bazlıdır. Durum çubuğunun başlangıç değerleri kayıttan çıkarılır (1c tasarımı §5.5, §8.3, §9.3).

**Files:**
- Create: `crates/kalfa-core/src/gecmis/mod.rs`
- Modify: `crates/kalfa-core/src/lib.rs`
- Test: `crates/kalfa-core/tests/gecmis.rs`

**Interfaces:**
- Consumes: Görev 8'in `KayitOgesi::{Istek { yonlendirilen_model }, Kullanim, OturumAcildi { calisma_alani }}`.
- Produces:
  - `gecmis::GorunumOgesi` (serde `tag = "tip"`): `KullaniciMesaji { sira, tur, metin }`, `Yanit { sira, tur, metin: Option<String>, dusunce: Option<String> }`, `AracCagrisi { sira, tur, model_cagri_id, arac, argumanlar, durum: Option<AracDurumu>, sonuc: Option<String> }`, `TurSonu { sira, tur, neden: BitisNedeni }`; `GorunumOgesi::{sira, tur}`
  - `gecmis::gorunum_uret(&[Zarf<KayitOgesi>]) -> Vec<GorunumOgesi>`
  - `gecmis::GecmisSayfasi { ogeler: Vec<GorunumOgesi>, daha_eski_var: bool }`; `gecmis::sayfa(ogeler: &[GorunumOgesi], once_sira: Option<u64>, tur_sayisi: usize) -> GecmisSayfasi`
  - `gecmis::DurumOzeti { model, kip, yonlendirilen_model, son_kredi, istem, tamamlama, dusunce }` (`Default`); `gecmis::durum_ozeti(&[Zarf<KayitOgesi>]) -> DurumOzeti`

- [ ] **Step 1: Başarısız testleri yaz**

`crates/kalfa-core/tests/gecmis.rs`:

```rust
use std::path::Path;
use std::sync::Arc;

use kalfa_core::gecmis::{DurumOzeti, GorunumOgesi, durum_ozeti, gorunum_uret, sayfa};
use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::{CagriId, OturumId, TurId};
use kalfa_core::oturum::OturumAyarlari;
use kalfa_core::protokol::{
    AracDurumu, BitisNedeni, Komut, Olay, OnayKarari, SEMA_SURUMU, Zarf,
};
use kalfa_core::test_destek::*;
use kalfa_evren::KrediGozlemi;
use kalfa_evren::tipler::{ChatMessage, FunctionCall, ToolCall};
use serde_json::json;

struct Kayitci {
    oturum: OturumId,
    sira: u64,
    kayitlar: Vec<Zarf<KayitOgesi>>,
}

impl Kayitci {
    fn yeni() -> Self {
        Self {
            oturum: OturumId::yeni(),
            sira: 0,
            kayitlar: Vec::new(),
        }
    }

    fn ekle(&mut self, tur: Option<TurId>, yuk: KayitOgesi) -> u64 {
        self.sira += 1;
        self.kayitlar.push(Zarf {
            sema: SEMA_SURUMU,
            oturum: self.oturum,
            tur,
            sira: self.sira,
            yuk,
        });
        self.sira
    }

    fn mesaj(&mut self, tur: TurId, mesaj: ChatMessage) -> u64 {
        self.ekle(Some(tur), KayitOgesi::Mesaj { mesaj })
    }
}

fn cagri(id: &str, ad: &str) -> ToolCall {
    ToolCall {
        id: id.into(),
        kind: "function".into(),
        function: FunctionCall {
            name: ad.into(),
            arguments: "{\"yol\":\"a\"}".into(),
        },
    }
}

#[test]
fn metin_turunun_gorunumu() {
    let mut k = Kayitci::yeni();
    let tur = TurId::yeni();
    let s1 = k.mesaj(tur, ChatMessage::user("selam"));
    let mut yanit = ChatMessage::assistant(Some("merhaba".into()), vec![]);
    yanit.reasoning = Some("düşündüm".into());
    let s2 = k.mesaj(tur, yanit);
    let s3 = k.ekle(
        Some(tur),
        KayitOgesi::TurBitti {
            neden: BitisNedeni::Normal,
        },
    );
    assert_eq!(
        gorunum_uret(&k.kayitlar),
        vec![
            GorunumOgesi::KullaniciMesaji {
                sira: s1,
                tur: Some(tur),
                metin: "selam".into()
            },
            GorunumOgesi::Yanit {
                sira: s2,
                tur: Some(tur),
                metin: Some("merhaba".into()),
                dusunce: Some("düşündüm".into())
            },
            GorunumOgesi::TurSonu {
                sira: s3,
                tur: Some(tur),
                neden: BitisNedeni::Normal
            },
        ]
    );
}

#[test]
fn arac_cagrilarinin_durumlari() {
    let mut k = Kayitci::yeni();
    let tur = TurId::yeni();
    k.mesaj(tur, ChatMessage::user("üçünü yap"));
    let s = k.mesaj(
        tur,
        ChatMessage::assistant(
            None,
            vec![cagri("m1", "yaz"), cagri("m2", "yaz"), cagri("m3", "yaz")],
        ),
    );
    let (c1, c2, c3) = (CagriId::yeni(), CagriId::yeni(), CagriId::yeni());
    // m1 çalıştı ve bitti.
    k.ekle(
        Some(tur),
        KayitOgesi::AracBasladi {
            cagri: c1,
            model_cagri_id: "m1".into(),
            arac: "yaz".into(),
        },
    );
    k.mesaj(tur, ChatMessage::tool("m1", "yazıldı"));
    k.ekle(
        Some(tur),
        KayitOgesi::AracBitti {
            cagri: c1,
            durum: AracDurumu::Basarili,
        },
    );
    // m2 reddedildi: başladı kaydı yok; bitişi sonucun hemen ardından gelir.
    k.mesaj(tur, ChatMessage::tool("m2", "[kullanıcı reddetti] gerek yok"));
    k.ekle(
        Some(tur),
        KayitOgesi::AracBitti {
            cagri: c2,
            durum: AracDurumu::Reddedildi,
        },
    );
    // m3 başladı, Kalfa sonuç gelmeden kapandı.
    k.ekle(
        Some(tur),
        KayitOgesi::AracBasladi {
            cagri: c3,
            model_cagri_id: "m3".into(),
            arac: "yaz".into(),
        },
    );
    let araclar: Vec<(String, Option<AracDurumu>, Option<String>, u64)> =
        gorunum_uret(&k.kayitlar)
            .into_iter()
            .filter_map(|o| match o {
                GorunumOgesi::AracCagrisi {
                    model_cagri_id,
                    durum,
                    sonuc,
                    sira,
                    ..
                } => Some((model_cagri_id, durum, sonuc, sira)),
                _ => None,
            })
            .collect();
    assert_eq!(
        araclar,
        vec![
            (
                String::from("m1"),
                Some(AracDurumu::Basarili),
                Some(String::from("yazıldı")),
                s
            ),
            (
                String::from("m2"),
                Some(AracDurumu::Reddedildi),
                Some(String::from("[kullanıcı reddetti] gerek yok")),
                s
            ),
            (
                String::from("m3"),
                Some(AracDurumu::SonucuBilinmiyor),
                None,
                s
            ),
        ]
    );
}

#[tokio::test]
async fn gercek_oturum_kaydindan_gorunum() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("oturum.jsonl");
    let mut ayar = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
    ayar.kayit_yolu = Some(yol.clone());
    let yaz = Arc::new(SahteArac::yazan("yaz"));
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("yaz", json!({"yol": "a"})),
        SahteYanit::metin("tamam"),
    ]);
    let mut surucu = test_oturumu(test_baglami(s, vec![yaz], sanal_siniflandirici()), ayar);
    surucu
        .gonder(Komut::KullaniciMesaji {
            metin: "yaz".into(),
        })
        .await;
    let onay = match surucu
        .bekle(|o| matches!(o, Olay::OnayGerekiyor { .. }))
        .await
        .yuk
    {
        Olay::OnayGerekiyor { ogeler, .. } => ogeler[0].clone(),
        _ => unreachable!(),
    };
    surucu
        .gonder(Komut::AracOnayi {
            cagri: onay.cagri,
            karar: OnayKarari::Reddet {
                gerekce: Some("gerek yok".into()),
            },
            muhur: onay.muhur,
        })
        .await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    surucu.kapat().await;

    let ogeler = gorunum_uret(&OturumKaydi::oku(&yol).unwrap());
    assert!(matches!(
        &ogeler[0],
        GorunumOgesi::KullaniciMesaji { metin, .. } if metin == "yaz"
    ));
    assert!(ogeler.iter().any(|o| matches!(
        o,
        GorunumOgesi::AracCagrisi { arac, durum: Some(AracDurumu::Reddedildi), sonuc: Some(s), .. }
            if arac == "yaz" && s.contains("gerek yok")
    )));
    assert!(ogeler.iter().any(|o| matches!(
        o,
        GorunumOgesi::Yanit { metin: Some(m), .. } if m == "tamam"
    )));
    assert!(matches!(
        ogeler.last(),
        Some(GorunumOgesi::TurSonu {
            neden: BitisNedeni::Normal,
            ..
        })
    ));
}

#[test]
fn sayfalama_tur_bazlidir() {
    let mut k = Kayitci::yeni();
    let mut turlar = Vec::new();
    for i in 0..3 {
        let tur = TurId::yeni();
        turlar.push(tur);
        k.mesaj(tur, ChatMessage::user(format!("soru {i}")));
        k.mesaj(tur, ChatMessage::assistant(Some(format!("yanıt {i}")), vec![]));
        k.ekle(
            Some(tur),
            KayitOgesi::TurBitti {
                neden: BitisNedeni::Normal,
            },
        );
    }
    let ogeler = gorunum_uret(&k.kayitlar);
    let son = sayfa(&ogeler, None, 2);
    assert!(son.daha_eski_var);
    assert_eq!(son.ogeler.len(), 6);
    assert_eq!(son.ogeler[0].tur(), Some(turlar[1]));
    let once = sayfa(&ogeler, Some(son.ogeler[0].sira()), 2);
    assert!(!once.daha_eski_var);
    assert_eq!(once.ogeler.len(), 3);
    assert!(once.ogeler.iter().all(|o| o.tur() == Some(turlar[0])));
}

#[test]
fn durum_ozeti_kayittan_cikarilir() {
    let mut k = Kayitci::yeni();
    k.ekle(
        None,
        KayitOgesi::OturumAcildi {
            model: "glm-5.3".into(),
            kip: Kip::Siki,
            calisma_alani: None,
        },
    );
    k.ekle(
        None,
        KayitOgesi::ModelDegisti {
            model: "auto".into(),
        },
    );
    let gozlem = KrediGozlemi {
        kalan: None,
        tutulan: None,
        istek_id: Some("r1".into()),
        zaman_ms: 7,
    };
    k.ekle(
        None,
        KayitOgesi::Istek {
            amac: "ana".into(),
            istek_id: Some("r1".into()),
            kredi: Some(gozlem.clone()),
            yonlendirilen_model: Some("deepseek-v4-flash".into()),
        },
    );
    k.ekle(
        None,
        KayitOgesi::Kullanim {
            istem: 10,
            tamamlama: 5,
            dusunce: 2,
        },
    );
    k.ekle(
        None,
        KayitOgesi::Kullanim {
            istem: 20,
            tamamlama: 1,
            dusunce: 0,
        },
    );
    k.ekle(None, KayitOgesi::KipDegisti { kip: Kip::Proje });
    assert_eq!(
        durum_ozeti(&k.kayitlar),
        DurumOzeti {
            model: Some("auto".into()),
            kip: Some(Kip::Proje),
            yonlendirilen_model: Some("deepseek-v4-flash".into()),
            son_kredi: Some(gozlem),
            istem: 30,
            tamamlama: 6,
            dusunce: 2,
        }
    );
}
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa-core --test gecmis`
Expected: FAIL — `gecmis` modülü yok.

- [ ] **Step 3: Modülü yaz**

`crates/kalfa-core/src/lib.rs` içinde `pub mod git_koruma;` satırından önce ekle:

```rust
pub mod gecmis;
```

`crates/kalfa-core/src/gecmis/mod.rs`:

```rust
//! Kayıttan arayüz görünümü (1c tasarımı §5.5, §8.3, §9.3).
//!
//! Kayıttan açılan oturumun ekrandaki geçmişi kayıt öğelerinden üretilir. Her
//! öğe kayıttaki `sira`'yı taşır; canlı olaylar aynı sayaçtan devam ettiği için
//! arayüz ikisini tek sırada birleştirir.

use std::collections::HashMap;

use kalfa_evren::KrediGozlemi;
use kalfa_evren::tipler::Role;
use serde::{Deserialize, Serialize};

use crate::izin::Kip;
use crate::kayit::KayitOgesi;
use crate::kimlik::{CagriId, TurId};
use crate::protokol::{AracDurumu, BitisNedeni, Zarf};

/// Ekranda gösterilen bir geçmiş öğesi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum GorunumOgesi {
    KullaniciMesaji {
        sira: u64,
        tur: Option<TurId>,
        metin: String,
    },
    Yanit {
        sira: u64,
        tur: Option<TurId>,
        metin: Option<String>,
        dusunce: Option<String>,
    },
    AracCagrisi {
        sira: u64,
        tur: Option<TurId>,
        /// Modelin verdiği çağrı kimliği.
        model_cagri_id: String,
        arac: String,
        /// Modelin gönderdiği ham argüman (JSON metni).
        argumanlar: String,
        /// `None`: sonuç kaydı yok, çağrı hiç başlamadı.
        durum: Option<AracDurumu>,
        sonuc: Option<String>,
    },
    TurSonu {
        sira: u64,
        tur: Option<TurId>,
        neden: BitisNedeni,
    },
}

impl GorunumOgesi {
    pub fn sira(&self) -> u64 {
        match self {
            Self::KullaniciMesaji { sira, .. }
            | Self::Yanit { sira, .. }
            | Self::AracCagrisi { sira, .. }
            | Self::TurSonu { sira, .. } => *sira,
        }
    }

    pub fn tur(&self) -> Option<TurId> {
        match self {
            Self::KullaniciMesaji { tur, .. }
            | Self::Yanit { tur, .. }
            | Self::AracCagrisi { tur, .. }
            | Self::TurSonu { tur, .. } => *tur,
        }
    }
}

/// Kayıttan ekran öğelerini üretir.
pub fn gorunum_uret(kayitlar: &[Zarf<KayitOgesi>]) -> Vec<GorunumOgesi> {
    let mut ogeler: Vec<GorunumOgesi> = Vec::new();
    // Modelin çağrı kimliği → öğenin yeri.
    let mut model_yeri: HashMap<String, usize> = HashMap::new();
    // Kalfa'nın çağrı kimliği → öğenin yeri (`AracBasladi` ile öğrenilir).
    let mut kalfa_yeri: HashMap<CagriId, usize> = HashMap::new();
    // Son `tool` mesajının öğesi: başlamamış çağrının `AracBitti`'si ona aittir.
    let mut son_sonuc: Option<usize> = None;
    for z in kayitlar {
        match &z.yuk {
            KayitOgesi::Mesaj { mesaj } => match mesaj.role {
                Role::User => ogeler.push(GorunumOgesi::KullaniciMesaji {
                    sira: z.sira,
                    tur: z.tur,
                    metin: mesaj.content.clone().unwrap_or_default(),
                }),
                Role::Assistant => {
                    if mesaj.content.is_some() || mesaj.reasoning.is_some() {
                        ogeler.push(GorunumOgesi::Yanit {
                            sira: z.sira,
                            tur: z.tur,
                            metin: mesaj.content.clone(),
                            dusunce: mesaj.reasoning.clone(),
                        });
                    }
                    for tc in mesaj.tool_calls.iter().flatten() {
                        model_yeri.insert(tc.id.clone(), ogeler.len());
                        ogeler.push(GorunumOgesi::AracCagrisi {
                            sira: z.sira,
                            tur: z.tur,
                            model_cagri_id: tc.id.clone(),
                            arac: tc.function.name.clone(),
                            argumanlar: tc.function.arguments.clone(),
                            durum: None,
                            sonuc: None,
                        });
                    }
                }
                Role::Tool => {
                    son_sonuc = mesaj
                        .tool_call_id
                        .as_ref()
                        .and_then(|id| model_yeri.get(id).copied());
                    if let Some(GorunumOgesi::AracCagrisi { sonuc, .. }) =
                        son_sonuc.and_then(|i| ogeler.get_mut(i))
                    {
                        *sonuc = mesaj.content.clone();
                    }
                }
                Role::System => {}
            },
            KayitOgesi::AracBasladi {
                cagri,
                model_cagri_id,
                ..
            } => {
                if let Some(&i) = model_yeri.get(model_cagri_id) {
                    kalfa_yeri.insert(*cagri, i);
                    if let Some(GorunumOgesi::AracCagrisi { durum, .. }) = ogeler.get_mut(i) {
                        // "Bitti" kaydı gelene kadar sonuç bilinmiyor.
                        *durum = Some(AracDurumu::SonucuBilinmiyor);
                    }
                }
            }
            KayitOgesi::AracBitti { cagri, durum: bitis } => {
                let yer = kalfa_yeri.get(cagri).copied().or(son_sonuc);
                if let Some(GorunumOgesi::AracCagrisi { durum, .. }) =
                    yer.and_then(|i| ogeler.get_mut(i))
                {
                    *durum = Some(*bitis);
                }
                son_sonuc = None;
            }
            KayitOgesi::TurBitti { neden } => ogeler.push(GorunumOgesi::TurSonu {
                sira: z.sira,
                tur: z.tur,
                neden: neden.clone(),
            }),
            _ => {}
        }
    }
    ogeler
}

/// Geçmişin bir sayfası.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GecmisSayfasi {
    pub ogeler: Vec<GorunumOgesi>,
    /// Bu sayfadan daha eski öğe var mı.
    pub daha_eski_var: bool,
}

/// `once_sira`'dan (verilmezse sondan) geriye doğru en fazla `tur_sayisi`
/// turluk öğe. Tur, aynı `tur` kimliğini taşıyan ardışık öğelerdir.
pub fn sayfa(ogeler: &[GorunumOgesi], once_sira: Option<u64>, tur_sayisi: usize) -> GecmisSayfasi {
    let son = match once_sira {
        Some(s) => ogeler.partition_point(|o| o.sira() < s),
        None => ogeler.len(),
    };
    let mut bas = son;
    let mut sayilan = 0;
    while bas > 0 && sayilan < tur_sayisi {
        let tur = ogeler[bas - 1].tur();
        while bas > 0 && ogeler[bas - 1].tur() == tur {
            bas -= 1;
        }
        sayilan += 1;
    }
    GecmisSayfasi {
        ogeler: ogeler[bas..son].to_vec(),
        daha_eski_var: bas > 0,
    }
}

/// Durum çubuğunun kayıttan başlangıç değerleri (1c tasarımı §8.3).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DurumOzeti {
    pub model: Option<String>,
    pub kip: Option<Kip>,
    pub yonlendirilen_model: Option<String>,
    pub son_kredi: Option<KrediGozlemi>,
    pub istem: u64,
    pub tamamlama: u64,
    pub dusunce: u64,
}

pub fn durum_ozeti(kayitlar: &[Zarf<KayitOgesi>]) -> DurumOzeti {
    let mut o = DurumOzeti::default();
    for z in kayitlar {
        match &z.yuk {
            KayitOgesi::OturumAcildi { model, kip, .. } => {
                o.model = Some(model.clone());
                o.kip = Some(*kip);
            }
            KayitOgesi::ModelDegisti { model } => o.model = Some(model.clone()),
            KayitOgesi::KipDegisti { kip } => o.kip = Some(*kip),
            KayitOgesi::Istek {
                kredi,
                yonlendirilen_model,
                ..
            } => {
                if yonlendirilen_model.is_some() {
                    o.yonlendirilen_model = yonlendirilen_model.clone();
                }
                if kredi.is_some() {
                    o.son_kredi = kredi.clone();
                }
            }
            KayitOgesi::Kullanim {
                istem,
                tamamlama,
                dusunce,
            } => {
                o.istem = o.istem.saturating_add(*istem);
                o.tamamlama = o.tamamlama.saturating_add(*tamamlama);
                o.dusunce = o.dusunce.saturating_add(*dusunce);
            }
            _ => {}
        }
    }
    o
}
```

- [ ] **Step 4: Testleri koştur**

Run: `cargo test -p kalfa-core --test gecmis`
Expected: PASS (5 test).

- [ ] **Step 5: Doğrulama ve commit**

Run: `cargo test --workspace --quiet && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: temiz.

```bash
git add crates/kalfa-core/src/lib.rs crates/kalfa-core/src/gecmis/mod.rs crates/kalfa-core/tests/gecmis.rs
git commit -m "kalfa-core: kayıttan geçmiş görünümü, tur bazlı sayfa ve durum özeti"
```

---

### Task 10: Oturum listesi

Liste kayıt dizininden çıkarılır; ayrı bir dizin dosyası tutulmaz. Küçük kayıt bütünüyle, büyük kayıt baştan 16 KiB ve sondan 64 KiB okunur (1c tasarımı §5.4).

**Files:**
- Create: `crates/kalfa-core/src/gecmis/liste.rs`
- Modify: `crates/kalfa-core/src/gecmis/mod.rs`
- Modify: `crates/kalfa-core/src/kayit.rs`
- Test: `crates/kalfa-core/tests/oturum_listesi.rs`

**Interfaces:**
- Consumes: `kayit::coz` (bu görevde `pub(crate)` olur); Görev 8'in `OturumAcildi { calisma_alani }`.
- Produces: `gecmis::OturumDurumu { Bos, Tamamlandi, IptalEdildi, Hata, YarimKaldi, Okunamadi { neden: String } }`; `gecmis::OturumOzeti { yol: PathBuf, oturum: Option<OturumId>, calisma_alani: Option<PathBuf>, baslik: Option<String>, son_degisiklik_ms: u64, durum: OturumDurumu }`; `gecmis::oturumlari_listele(&Path) -> io::Result<Vec<OturumOzeti>>`; `gecmis::oturum_ozeti(&Path) -> OturumOzeti`.

- [ ] **Step 1: Başarısız testleri yaz**

`crates/kalfa-core/tests/oturum_listesi.rs`:

```rust
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use kalfa_core::gecmis::{OturumDurumu, oturumlari_listele};
use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::{OturumId, TurId};
use kalfa_core::protokol::{BitisNedeni, SEMA_SURUMU, Zarf};
use kalfa_evren::tipler::ChatMessage;

fn kayit_yaz(yol: &Path, ogeler: Vec<KayitOgesi>, zaman_sn: u64) {
    let oturum = OturumId::yeni();
    let tur = TurId::yeni();
    let mut k = OturumKaydi::olustur(yol).unwrap();
    for (i, yuk) in (1u64..).zip(ogeler) {
        k.yaz(&Zarf {
            sema: SEMA_SURUMU,
            oturum,
            tur: Some(tur),
            sira: i,
            yuk,
        })
        .unwrap();
    }
    drop(k);
    fs::File::options()
        .write(true)
        .open(yol)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(zaman_sn))
        .unwrap();
}

fn acilis() -> KayitOgesi {
    KayitOgesi::OturumAcildi {
        model: "glm-5.3".into(),
        kip: Kip::Siki,
        calisma_alani: Some("C:/proje".into()),
    }
}

fn kullanici(m: &str) -> KayitOgesi {
    KayitOgesi::Mesaj {
        mesaj: ChatMessage::user(m),
    }
}

fn bitti(neden: BitisNedeni) -> KayitOgesi {
    KayitOgesi::TurBitti { neden }
}

#[test]
fn liste_en_yeni_once_baslik_ve_durumla_gelir() {
    let d = tempfile::tempdir().unwrap();
    kayit_yaz(
        &d.path().join("a.jsonl"),
        vec![
            acilis(),
            kullanici("  \nBirinci iş\nayrıntı"),
            bitti(BitisNedeni::Normal),
        ],
        1_000,
    );
    let uzun = "u".repeat(100);
    kayit_yaz(
        &d.path().join("b.jsonl"),
        vec![acilis(), kullanici(&uzun)],
        2_000,
    );
    let liste = oturumlari_listele(d.path()).unwrap();
    assert_eq!(liste.len(), 2, "kilit dosyaları listeye girmez");
    assert_eq!(liste[0].yol, d.path().join("b.jsonl"));
    assert_eq!(liste[0].durum, OturumDurumu::YarimKaldi);
    let kisa = format!("{}···", "u".repeat(80));
    assert_eq!(liste[0].baslik.as_deref(), Some(kisa.as_str()));
    assert_eq!(liste[1].baslik.as_deref(), Some("Birinci iş"));
    assert_eq!(liste[1].durum, OturumDurumu::Tamamlandi);
    assert_eq!(liste[1].calisma_alani.as_deref(), Some(Path::new("C:/proje")));
    assert_eq!(liste[1].son_degisiklik_ms, 1_000_000);
}

#[test]
fn bos_bozuk_ve_olmayan_dizin() {
    let d = tempfile::tempdir().unwrap();
    kayit_yaz(&d.path().join("bos.jsonl"), vec![acilis()], 1);
    fs::write(d.path().join("bozuk.jsonl"), "bu bir kayıt değil\n").unwrap();
    let liste = oturumlari_listele(d.path()).unwrap();
    let durum = |ad: &str| {
        liste
            .iter()
            .find(|o| o.yol.ends_with(ad))
            .unwrap()
            .durum
            .clone()
    };
    assert_eq!(durum("bos.jsonl"), OturumDurumu::Bos);
    assert!(matches!(durum("bozuk.jsonl"), OturumDurumu::Okunamadi { .. }));
    assert!(oturumlari_listele(&d.path().join("yok")).unwrap().is_empty());
}

#[test]
fn buyuk_kayitta_son_durum_sondan_okunur() {
    let d = tempfile::tempdir().unwrap();
    let mut ogeler = vec![acilis(), kullanici("uzun iş")];
    ogeler.extend((0..3000).map(|_| KayitOgesi::KipDegisti { kip: Kip::Proje }));
    ogeler.push(bitti(BitisNedeni::Iptal));
    let yol = d.path().join("buyuk.jsonl");
    kayit_yaz(&yol, ogeler, 5);
    assert!(
        fs::metadata(&yol).unwrap().len() > 80 * 1024,
        "test kaydı baş ve son pencereden büyük olmalı"
    );
    let liste = oturumlari_listele(d.path()).unwrap();
    assert_eq!(liste[0].baslik.as_deref(), Some("uzun iş"));
    assert_eq!(liste[0].durum, OturumDurumu::IptalEdildi);
}
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa-core --test oturum_listesi`
Expected: FAIL — `oturumlari_listele` yok.

- [ ] **Step 3: Kayıt çözücüsünü modül içine aç**

`crates/kalfa-core/src/kayit.rs` içinde `fn coz(` tanımını `pub(crate) fn coz(` yap.

- [ ] **Step 4: Listeyi yaz**

`crates/kalfa-core/src/gecmis/mod.rs` içinde `use std::collections::HashMap;` satırından önce ekle:

```rust
mod liste;

pub use liste::{OturumDurumu, OturumOzeti, oturum_ozeti, oturumlari_listele};
```

`crates/kalfa-core/src/gecmis/liste.rs`:

```rust
//! Kayıt dizininden oturum listesi (1c tasarımı §5.4).
//!
//! Ayrı bir dizin dosyası tutulmaz; kaynak her zaman kayıttır. Her kaydın
//! başından çalışma alanı ve başlık, sonundan son durum okunur.

use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use kalfa_evren::tipler::Role;
use serde::{Deserialize, Serialize};

use crate::kayit::{KayitOgesi, coz};
use crate::kimlik::OturumId;
use crate::protokol::{BitisNedeni, Zarf};

/// Başlık ve çalışma alanı için okunan baş kısım.
const BAS_BAYT: u64 = 16 * 1024;
/// Son durum için okunan son kısım.
const SON_BAYT: u64 = 64 * 1024;
const AZAMI_BASLIK: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum OturumDurumu {
    /// Henüz kullanıcı mesajı yok.
    Bos,
    Tamamlandi,
    IptalEdildi,
    Hata,
    /// Son turun `TurBitti` kaydı yok: Kalfa tur sürerken kapanmış.
    YarimKaldi,
    Okunamadi { neden: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OturumOzeti {
    pub yol: PathBuf,
    pub oturum: Option<OturumId>,
    pub calisma_alani: Option<PathBuf>,
    pub baslik: Option<String>,
    /// Son değişiklik zamanı, Unix epoch milisaniyesi.
    pub son_degisiklik_ms: u64,
    pub durum: OturumDurumu,
}

/// Dizindeki `.jsonl` kayıtlarının özetleri, en yeni önce. Dizin yoksa boş
/// liste. Okunamayan kayıt listeden düşmez; `Okunamadi` olarak görünür.
pub fn oturumlari_listele(dizin: &Path) -> io::Result<Vec<OturumOzeti>> {
    let girisler = match fs::read_dir(dizin) {
        Ok(g) => g,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut ozetler = Vec::new();
    for giris in girisler {
        let yol = giris?.path();
        if yol.extension().and_then(|u| u.to_str()) == Some("jsonl") {
            ozetler.push(oturum_ozeti(&yol));
        }
    }
    ozetler.sort_by(|a, b| b.son_degisiklik_ms.cmp(&a.son_degisiklik_ms));
    Ok(ozetler)
}

/// Tek kaydın özeti. Hata döndürmez; okunamayan kayıt `Okunamadi` olur.
pub fn oturum_ozeti(yol: &Path) -> OturumOzeti {
    let son_degisiklik_ms = degisiklik_ms(yol);
    match ozet_oku(yol) {
        Ok(o) => OturumOzeti {
            yol: yol.to_path_buf(),
            oturum: Some(o.oturum),
            calisma_alani: o.calisma_alani,
            baslik: o.baslik,
            son_degisiklik_ms,
            durum: o.durum,
        },
        Err(e) => OturumOzeti {
            yol: yol.to_path_buf(),
            oturum: None,
            calisma_alani: None,
            baslik: None,
            son_degisiklik_ms,
            durum: OturumDurumu::Okunamadi {
                neden: e.to_string(),
            },
        },
    }
}

struct Okunan {
    oturum: OturumId,
    calisma_alani: Option<PathBuf>,
    baslik: Option<String>,
    durum: OturumDurumu,
}

fn ozet_oku(yol: &Path) -> io::Result<Okunan> {
    let mut dosya = File::open(yol)?;
    let boyut = dosya.metadata()?.len();
    let (bas, son, tamami) = if boyut <= BAS_BAYT + SON_BAYT {
        let hepsi = coz(&aralik_oku(&mut dosya, 0, boyut)?)?.0;
        (hepsi.clone(), hepsi, true)
    } else {
        let bas = satirlari_coz(&aralik_oku(&mut dosya, 0, BAS_BAYT)?, false)?;
        let son = satirlari_coz(&aralik_oku(&mut dosya, boyut - SON_BAYT, SON_BAYT)?, true)?;
        (bas, son, false)
    };
    let Some(ilk) = bas.first() else {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Kayıt boş."));
    };
    let KayitOgesi::OturumAcildi { calisma_alani, .. } = &ilk.yuk else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Kayıt bir açılış kaydıyla başlamıyor.",
        ));
    };
    Ok(Okunan {
        oturum: ilk.oturum,
        calisma_alani: calisma_alani.clone(),
        baslik: baslik(&bas),
        durum: durum_bul(&son, tamami),
    })
}

fn aralik_oku(dosya: &mut File, bas: u64, uzunluk: u64) -> io::Result<Vec<u8>> {
    dosya.seek(SeekFrom::Start(bas))?;
    let mut veri = Vec::new();
    dosya.by_ref().take(uzunluk).read_to_end(&mut veri)?;
    Ok(veri)
}

/// Bir aralıktaki tam satırları çözer. Son parça satır sonuyla bitmediği için
/// yarımdır ve atlanır. `ilki_yarim`: aralık bir satırın ortasından başlar,
/// ilk satır da atlanır.
fn satirlari_coz(veri: &[u8], ilki_yarim: bool) -> io::Result<Vec<Zarf<KayitOgesi>>> {
    let mut satirlar: Vec<&[u8]> = veri.split(|b| *b == b'\n').collect();
    let _ = satirlar.pop();
    let mut kayitlar = Vec::new();
    for satir in satirlar.into_iter().skip(usize::from(ilki_yarim)) {
        if satir.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let zarf = serde_json::from_slice(satir)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        kayitlar.push(zarf);
    }
    Ok(kayitlar)
}

fn baslik(kayitlar: &[Zarf<KayitOgesi>]) -> Option<String> {
    let metin = kayitlar.iter().find_map(|z| match &z.yuk {
        KayitOgesi::Mesaj { mesaj } if mesaj.role == Role::User => mesaj.content.clone(),
        _ => None,
    })?;
    let satir = metin.lines().map(str::trim).find(|s| !s.is_empty())?;
    if satir.chars().count() <= AZAMI_BASLIK {
        return Some(satir.to_string());
    }
    let mut kisa: String = satir.chars().take(AZAMI_BASLIK).collect();
    kisa.push_str("···");
    Some(kisa)
}

fn durum_bul(kayitlar: &[Zarf<KayitOgesi>], tamami: bool) -> OturumDurumu {
    let son_mesaj = kayitlar
        .iter()
        .rev()
        .find(|z| matches!(&z.yuk, KayitOgesi::Mesaj { mesaj } if mesaj.role == Role::User))
        .map(|z| z.sira);
    let son_bitis = kayitlar.iter().rev().find_map(|z| match &z.yuk {
        KayitOgesi::TurBitti { neden } => Some((z.sira, neden)),
        _ => None,
    });
    match (son_mesaj, son_bitis) {
        (Some(mesaj), Some((bitis, neden))) if bitis > mesaj => bitisten(neden),
        (None, Some((_, neden))) => bitisten(neden),
        (None, None) if tamami => OturumDurumu::Bos,
        _ => OturumDurumu::YarimKaldi,
    }
}

fn bitisten(neden: &BitisNedeni) -> OturumDurumu {
    match neden {
        BitisNedeni::Normal | BitisNedeni::Sinir { .. } => OturumDurumu::Tamamlandi,
        BitisNedeni::Iptal => OturumDurumu::IptalEdildi,
        BitisNedeni::Hata => OturumDurumu::Hata,
    }
}

fn degisiklik_ms(yol: &Path) -> u64 {
    fs::metadata(yol)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}
```

- [ ] **Step 5: Testleri koştur**

Run: `cargo test -p kalfa-core --test oturum_listesi`
Expected: PASS (3 test).

- [ ] **Step 6: Doğrulama ve commit**

Run: `cargo test --workspace --quiet && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: temiz.

```bash
git add crates/kalfa-core/src/kayit.rs crates/kalfa-core/src/gecmis crates/kalfa-core/tests/oturum_listesi.rs
git commit -m "kalfa-core: kayıt dizininden oturum listesi"
```

---

### Task 11: Kullanıcıya giden metinlerde "siz"

Tasarım sistemi arayüzde "siz" diliyle konuşur (1c tasarımı §9.4). Kullanıcıya giden üç metin "sen" diliyle yazılmış; modele giden araç hataları "sen" kalır.

**Files:**
- Modify: `crates/kalfa-evren/src/hata.rs`
- Modify: `crates/kalfa-core/src/oturum/mod.rs`
- Test: `crates/kalfa-evren/tests/hata_kredi.rs`
- Test: `crates/kalfa-core/tests/oturum_metin.rs`
- Test: `crates/kalfa-core/tests/oturum_iptal_sinir.rs`

- [ ] **Step 1: Başarısız testleri yaz**

`crates/kalfa-evren/tests/hata_kredi.rs` dosyasının sonuna ekle:

```rust
#[test]
fn anahtar_hatasi_kullaniciya_siz_diye_hitap_eder() {
    let m = EvrenError::AnahtarGecersiz.to_string();
    assert!(m.contains("anahtarınızı kontrol edip yeniden girin"), "{m}");
}
```

`crates/kalfa-core/tests/oturum_metin.rs` içindeki `kayit_yazilamazsa_oturum_durur_ve_kayitsiza_donmez` testinde ikinci `bekle` koşulunu değiştir:

```rust
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("yazılamadığı için bu oturum durduruldu") && mesaj.ends_with("Yeni bir oturum açın.")))
        .await;
```

`crates/kalfa-core/tests/oturum_iptal_sinir.rs` içindeki `tur_sirasinda_yeni_mesaj_reddedilir` testinde `bekle` koşulunu değiştir:

```rust
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj == "Önceki tur sürüyor; bitmesini bekleyin ya da iptal edin."))
        .await;
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cargo test -p kalfa-evren --test hata_kredi && cargo test -p kalfa-core --test oturum_metin --test oturum_iptal_sinir`
Expected: FAIL — metinler hâlâ "sen" diliyle.

- [ ] **Step 3: Metinleri değiştir**

`crates/kalfa-evren/src/hata.rs`:

```rust
    #[error("Anahtar geçersiz ya da eksik. EVREN portalından anahtarınızı kontrol edip yeniden girin.")]
    AnahtarGecersiz,
```

`crates/kalfa-core/src/oturum/mod.rs` içinde:

```rust
                            "Oturum kaydı yazılamadığı için bu oturum durduruldu ({neden}). Yeni bir oturum açın."
```

```rust
                self.hata("Önceki tur sürüyor; bitmesini bekleyin ya da iptal edin.")
                    .await;
```

- [ ] **Step 4: Testleri koştur**

Run: `cargo test --workspace --quiet`
Expected: PASS.

- [ ] **Step 5: Doğrulama ve commit**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: temiz.

```bash
git add crates/kalfa-evren/src/hata.rs crates/kalfa-evren/tests/hata_kredi.rs crates/kalfa-core/src/oturum/mod.rs crates/kalfa-core/tests/oturum_metin.rs crates/kalfa-core/tests/oturum_iptal_sinir.rs
git commit -m "kalfa-core, kalfa-evren: kullanıcıya giden hata metinleri siz diliyle"
```

---

### Task 12: Son doğrulama ve durum dosyası

Kayıt kilidi (flock / LockFileEx) ve kök kimliği (aygıt+inode / birim+dizin) platforma göre değişir; plan iki platformda da doğrulanır. Uygulayıcı kendi ortamında doğrular; öbür platformu inceleyici PR'ı birleştirmeden önce doğrular (bkz. "PR ile yürütme").

**Files:**
- Modify: `docs/GELISTIRME_DURUMU.md`

- [ ] **Step 1: Uygulayıcının ortamında doğrulama**

Run:

```bash
cargo test --workspace --quiet
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo tree -p kalfa-core -e normal | grep -E "kalfa-tools|tauri"
git grep -n -E "\.unwrap\(\)|\.expect\(" -- crates/kalfa-core/src ':!crates/kalfa-core/src/test_destek.rs'
git status research/
```

Expected: testler geçer; Clippy ve biçim temiz; `cargo tree` çıktısı boş; `research/` temiz. `git grep` eşleşmeleri yalnızca `#[cfg(test)]` test modüllerinde olabilir (ör. `oturum/mod.rs` içindeki `mod tests`); kütüphane kodunda eşleşme varsa düzeltilir. Windows'ta Windows'a özel testler (`junction_*`, `buyuk_kucuk_harf_*`) de koşar; Linux'ta derlenmezler, `sembolik_baglanti_ayni_koku_verir` ise Linux'ta koşar.

- [ ] **Step 2: Durum dosyasını güncelle**

`docs/GELISTIRME_DURUMU.md` içinde "Devam eden aşama — 1c" bölümüne, "Tasarım belgesi" alt başlığından önce şu alt başlığı ekle. Tabloda yalnızca Adım 1'i koştuğun platformun sütununu gerçek çıktılarla doldur; öbür sütunda `(inceleme)` kalsın, onu inceleyici doldurur:

```markdown
### 1c-1 — çekirdek eklemeleri (tamamlandı)

Plan: [`2026-09-22-kalfa-1c-1-cekirdek-eklemeleri.md`](superpowers/plans/2026-09-22-kalfa-1c-1-cekirdek-eklemeleri.md).
`kalfa-core`'a eklenenler: olay paketleyici ve kredi (`aktarim`), terminal
`AktarimDurdu` olayı, tek şart kapısı (`sart_kapisi`; oturumlar kabul isteğini
kendisi göndermez), çalışma alanı kilit yöneticisi, kayıt başına tek yazıcı,
kayıtta çalışma alanı / yönlendirilen model / token kullanımı, geçmiş
görünümü ve oturum listesi (`gecmis`).

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `cargo test --workspace` | (sayılar) | (sayılar) |
| Clippy / fmt | (sonuç) | (sonuç) |
| `cargo tree` izolasyonu | (sonuç) | (sonuç) |
```

- [ ] **Step 3: Commit ve push**

```bash
git add docs/GELISTIRME_DURUMU.md
git commit -m "docs: 1c-1 çekirdek eklemeleri tamamlandı; doğrulama kaydı"
git push
```

- [ ] **Step 4 (inceleyici): Öbür platformda doğrulama ve birleştirme**

İnceleyici PR dalını alır, Adım 1'in komutlarını öbür platformda koşar (Windows ya da WSL'deki Ubuntu 24.04), tablonun boş sütununu doldurup commit'ler ve inceleme temizse PR'ı birleştirir.

## Plan sonu kontrol listesi

- [ ] Paketleyici: pencere, bitişik birleştirme, kritik olay, paket sınırı, kredi, krediden büyük olay, boşluksuz numara, kapanış (tests/aktarim.rs).
- [ ] Duraklayan tüketicide yolda en çok kredi kadar paket; çekirdek bütçesi dolunca tur iptal ve `AktarimDurdu { TuketiciYetismedi }` (tests/aktarim_oturum.rs).
- [ ] Şart kabulü EVREN'e bir kez gider; iki bekleyen oturum birlikte uyanır; ret turları iptal eder; kabul edilen sürüm kayda yazılır.
- [ ] `Komut::SartlariKabulEt` yok; ana pencere şart kabul edemez hâle 1c-2'de komut yetkileriyle gelir.
- [ ] Aynı kök farklı yazılışla / bağlantıyla aynı kilitleri alır; yol tablosu büyümez.
- [ ] Aynı kayda aynı süreçte ve başka süreçte ikinci yazıcı açılmaz; kilit tutulurken kayıt okunur; oturum kapanınca kayıt yeniden açılır.
- [ ] Geçmiş görünümü gerçek bir oturum kaydında doğru; sayfalama tur bazlı; durum özeti doğru.
- [ ] Oturum listesi: en yeni önce; başlık 80 karakter ve `···`; büyük kayıtta son durum sondan okunur; bozuk kayıt `Okunamadi`.
- [ ] Kullanıcıya giden metinler "siz" diliyle.
- [ ] Windows ve Linux'ta `cargo test --workspace` yeşil; Clippy, fmt, `cargo tree` ve `unwrap`/`expect` taraması temiz.
