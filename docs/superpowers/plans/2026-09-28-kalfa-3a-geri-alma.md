# Kalfa 3a — Tur özeti ve geri alma: Uygulama Planı

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Kullanıcı bir turun dosya aracı değişikliklerini geri alabilsin. Proje ve Serbest kipte model onaysız yazar; bu planın sonunda o kiplerin güvenlik ağı vardır:
- **Tur özeti:** her tur bir özetle biter: dosya araçlarıyla değişen dosyalar, kabukla değişen dosyalar, komutlar ve EVREN çağrıları.
- **Geri alma:** "Turu geri al" önce ne yapılacağını gösterir, sonra mühürle yürütülür. Sonradan değişmiş dosyaya dokunmaz.
- **Onay kartı:** bir değişikliğin geri alınıp alınamayacağını söyler.
- **Oturumu silme:** oturum kaydı ve geri alma verisiyle birlikte silinebilir.

**Architecture:**
- **Önceki hâllerin deposu** (`kalfa-core/src/geri_alma.rs`):
  - Veri dizininde, kaydın dışındadır. Dosyaların adı içeriklerinin SHA-256'sıdır.
  - Kayda yalnızca özetler girer. Kayıt geçmiş açılışında tümüyle okunduğu için içerik oraya gömülmez.
- **Yakalama:**
  - Dosya araçları önceki içeriği ve sonraki özeti hazırlıkta verir.
  - Oturum yazmadan önce önceki içeriği depoya, özetleri de "başladı" kaydına kalıcı yazar.
- **Geri alma:**
  - Çekirdek planı turun yedeklerinden ve dosyaların şimdiki özetinden çıkarır.
  - Önizleme bir mühür taşır; yürütme bu mühürle ve kilit altında yapılır.
  - Çalışma alanına yazmak `GeriYukleyici` trait'iyle olur; `kalfa-tools` bunu dosya araçlarının güvenli yazma yoluyla uygular. `kalfa-core` `kalfa-tools`'a bağlanmaz.
- **Kabuk:** komuttan önce ve sonra meta veri görüntüsü alınır. Görüntü mevcut taramalara eklenir. Değişen dosyalar listelenir, geri alınmaz.
- **Arayüz:** tur özeti (`TurnSummary`), geri alma sayfası, onay kartında geri alınabilirlik, oturum listesinde "Sil".

**Tech Stack:** Rust 1.98.1 (cap-std 4, same-file, ignore, sha2), Svelte 5 + TypeScript, Vitest, `tauri-driver` + WebdriverIO.

**Kaynaklar:**
- [Aşama 3 tasarımı](../specs/2026-09-28-kalfa-3-gorunurluk-design.md) §2, §4, §5, §9.3 (geri alınabilirlik satırı), §10, §11, §13.
- [Üst belge](../specs/2026-09-21-kalfa-design.md) §6.6, §6.7, §7.5, §11.4 (r3.9).
- [Aşama 2 tasarımı](../specs/2026-09-24-kalfa-2-kabuk-yalitim-design.md) §7.2 (tarama), §9.4 (A11: üst dizinlere listeleme).

**Doğrulama durumu:**
- **Hafif plan:** görevleri, arayüzleri ve önce yazılacak testleri verir. Kodu önceden yazılıp doğrulanmadı.
- **Uygulayan:** Claude, yerel makinede (kullanıcı kararı K10). Plan hafif olduğu için AGENTS.md'deki Codex akışı bu planda uygulanmaz.
- **Platform doğrulaması:** Windows ve WSL (Ubuntu-24.04) doğrulamasını Claude yerel makinede yapar.
- **Ölçülecekler:** kabuk görüntüsünün maliyeti (G1), art arda iki kullanıcı mesajı (G2), deponun kaptan görünmemesi (G10) ve sınırlar (G12). Her birinin sonucu plandaki "Uygulama sonucu" bölümüne yazılır.

**Kapsam dışı:** 3b–3e (gönderim günlüğü, denetçi, ağ çıkışı, kısayollar, ek gizli kalıplar), geri almayı geri alma, kabuk değişikliklerini geri alma.

## Kararlar (kullanıcı, 2026-09-28)

Tasarımın §2.1 bölümündeki K1–K3 ve K8–K10:

1. **Önceki içerik oturumla birlikte saklanır.**
   - Veri dizininde ayrı bir depoda durur; dosyaların adı içeriklerinin özetidir.
   - Oturum silinene kadar kalır.
   - Sınırlar: dosya başına 8 MiB, oturum başına 512 MiB. Aşılınca en eski turların verisi silinir.
2. **Kabuğun değiştirdiği dosyalar** geri alınamaz, listelenir.
3. **Sonradan değişmiş dosyaya dokunulmaz;** dosya çakışma olarak listelenir, öteki dosyalar geri alınır.
4. **Gizli dosyanın önceki hâli saklanır (K8).** Diff'i kayda yazılmaz, depo EVREN'e gitmez; oturum silinince o da silinir.
5. **Geri alınamayacak kadar büyük dosyaya yazmak her kipte onay ister (K9).** Kayıtsız oturumda uygulanmaz.
6. **Planı Claude yerel makinede uygular (K10).**

Tasarımın 3a'ya giren öteki kararları (§2.2): 1–5 ve 10. Oturum silme yerel pencerede onaylanır.

## Global Constraints

- **Araçlar:** Rust 1.98.1; `cargo` komutları `--locked` ile. Node `^22.22.2 || >=24.15.0`; kurulum `npm ci`.
- **Bağımlılık:**
  - Workspace'e yeni crate girmez.
  - `kalfa-core` geçici dosya adları için var olan `uuid`'i kullanır. `tempfile` yalnızca dev bağımlılığıdır.
  - `ui/`'ye paket eklenmez.
- **Güvenlik sınırları:**
  - Depodaki dosyanın adı yalnızca 64 küçük onaltılık karakterdir. Başka ad reddedilir; ad bir yol parçası olamaz.
  - Geri yüklenecek içeriğin özeti dosyanın adıyla yeniden karşılaştırılır; tutmazsa yazılmaz.
  - Geri yazma, dosya araçlarının yolundan geçer: bileşenler bağlantı izlenmeden açılır, `.git` takma adı reddedilir, hard link'li dosyaya yazılmaz, yazmadan hemen önce şimdiki özet denetlenir.
  - Araç onaylanan tarife göre çalışır: `tek_calistir`'daki karar karşılaştırması büyük dosya kuralını da içerir.
  - Depo yalıtımın içinde görünmez ve hiçbir EVREN isteğine girmez.
  - Test edilmeyen güvenlik sınırı yoktur; her görev kendi sınırının testini getirir.
- **`unsafe`:** bu planda yeni `unsafe` yoktur.
- **Dil:**
  - Kod, test ve açıklamalar Türkçedir.
  - Kullanıcıya görünen metinler `ui/src/dil/tr.ts`'te ya da Rust'ta sabittir ve "siz" diliyle yazılır. Arayüzde "yalıtım" denir.
  - `unwrap()` ve `expect()` yalnızca testlerde ve `test_destek.rs`'de kullanılır.
- **Arayüz:**
  - CSP sıkıdır; sabit `style="…"` kullanılmaz, ham HTML basılmaz.
  - Arayüz yol göndermez; oturum ve tur kimliğiyle çalışır.
  - Pencerelere Tauri'nin çekirdek izinleri verilmez.
- **Protokol:**
  - Yeni alanlar `#[serde(default)]` taşır; eski kayıtlar yüklenebilmelidir. Buna eski kaydın açılıp geçmişinin gösterilmesini sınayan bir test eşlik eder.
  - `ui/src/ipc/tipler.ts` aynı commit'te güncellenir.
  - Yeni olaylar `aktarim::kritik_mi`'ye eklenir.
- **Her görevin sonunda:**
  - `cargo test --workspace --locked`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`
  - `cargo fmt --all -- --check`
  - `kalfa-uygulama` ya da `kalfa`'ya dokunulduysa: `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked` ve aynı özellikle clippy.
  - `kalfa-sandbox` ya da `kalfa-tools`'a dokunulduysa, root olmayan kullanıcıyla: yalıtım clippy'si, `cargo test -p kalfa-sandbox --features yalitim-testleri --test yalitim` ve `cargo test -p kalfa-tools --features yalitim-testleri --test uyumluluk`.
  - Arayüze dokunulduysa `ui/` içinde: `npm run check` (0 hata, 0 uyarı), `npm test`, `npm run build`.
  - Görev, Windows'ta ve WSL'de (Ubuntu-24.04) yeşil olmadan bitmiş sayılmaz.
- **Commit:** Türkçe ve önekli (`çekirdek:`, `araçlar:`, `sandbox:`, `uygulama:`, `ui:`, `e2e:`, `docs:`). Claude'un commit'leri `Co-Authored-By` satırı taşır; Codex'inkiler taşımaz.
- **Dokunulmayanlar:** `design/`, `.env`, `Screenshot*.png`, `refs/shot-*.png`, kökteki `tasks.png`.

## Dosya Yapısı

```
crates/kalfa-core/src/geri_alma.rs            yeni: OncekiHalDeposu, DosyaYedegi, GeriYukleyici, plan ve mühür
crates/kalfa-core/src/tur_ozeti.rs            yeni: TurOzeti, TurBiriktirici
crates/kalfa-core/src/oturum/geri_alma.rs     yeni: önizleme ve yürütme komutları, sınır aşımında temizlik
crates/kalfa-core/src/oturum/{mod,arac_asamasi}.rs   depo, yakalama sırası, tur özeti, kabuk değişiklikleri
crates/kalfa-core/src/{arac,izin,kayit,protokol,kurtarma,aktarim}.rs
crates/kalfa-core/src/gecmis/mod.rs           TurOzeti, GeriAlma, KalfaNotu görünüm öğeleri
crates/kalfa-core/src/test_destek.rs          SahteArac yedekleri, SahteGeriYukleyici
crates/kalfa-tools/src/{yaz,duzenle,calisma_alani}.rs   önceki içerik, sonraki özet, olusacak_dizinler, GeriYukleyici
crates/kalfa-tools/src/kabuk/mod.rs           komut öncesi/sonrası görüntü, AracCiktisi.degisiklikler
crates/kalfa-sandbox/src/tarama.rs            Goruntu, karsilastir
crates/kalfa-uygulama/src/{dizin,oturumlar,sozlesme,uygulama}.rs   geri_alma_dizini, komutlar, oturumu silme
crates/kalfa/src/{lib,komutlar,komut_adlari}.rs, crates/kalfa/capabilities/ana-pencere.json
ui/src/bilesenler/TurnSummary.svelte          yeni (tasarım sistemindeki TurnSummary)
ui/src/ana/{TurOzeti,GeriAlmaSayfasi}.svelte  yeni
ui/src/ana/{Konusma,KonusmaOgesi,OnaySayfasi,OturumListesi}.svelte
ui/src/{ipc/tipler.ts,ipc/ana.ts,durum/oturum.ts,dil/tr.ts,sahte/arka.ts,onizleme/senaryo.ts}
e2e/testler/06-geri-alma.test.mjs             yeni
docs/superpowers/specs/*, docs/GELISTIRME_DURUMU.md
```

---

### Task 1: Önceki hâllerin deposu

**Files:**
- `crates/kalfa-core/src/geri_alma.rs` (yeni), `crates/kalfa-core/src/lib.rs`
- Testler: `crates/kalfa-core/tests/geri_alma_deposu.rs` (yeni)

**Interfaces (Produces):**
- **Sabitler:** `DOSYA_SINIRI: u64 = 8 MiB`, `OTURUM_SINIRI: u64 = 512 MiB`, `OTURUM_ALT_SINIRI: u64 = 448 MiB`, `DIFF_SINIRI: usize = 64 KiB`.
- **`ozet_gecerli_mi(ozet: &str) -> bool`:** ad tam olarak 64 küçük onaltılık karakterse `true`.
- **`OncekiHalDeposu`:**
  - `ac(dizin: &Path) -> io::Result<Self>`: dizini oluşturur. Unix'te izinleri `0700`'dır.
  - `sakla(&self, ozet: &str, icerik: &[u8]) -> io::Result<()>`:
    - Ad geçersizse ya da `muhur::ozet_hex(icerik) != ozet` ise `InvalidInput`.
    - Dosya varsa hiçbir şey yapmaz.
    - Yoksa içeriği `.<ozet>.<uuid>.yeni` adlı yeni bir dosyaya yazar (`create_new`; Unix'te `0600`), `sync_all` ile kalıcılaştırır ve `<ozet>` adına taşır. Unix'te dizin de kalıcılaştırılır.
  - `oku(&self, ozet: &str) -> io::Result<Vec<u8>>`: okuduğu içeriğin özeti adla tutmazsa `InvalidData` döner, ileti "önceki hâl bozuk".
  - `var_mi(&self, ozet) -> bool`, `boyut(&self) -> io::Result<u64>`, `sil(&self, ozet) -> io::Result<()>`.
  - `kullanilmayanlari_sil(&self, kullanilan: &HashSet<String>) -> io::Result<u64>`: kümede olmayan dosyaları ve yarım kalmış `.yeni` dosyalarını siler.
  - `tumunu_sil(self) -> io::Result<()>`.

- [ ] **Step 1: Başarısız testleri yaz**
  - `saklanan_icerik_ozetiyle_geri_okunur`
  - `ozetle_uyusmayan_icerik_saklanmaz`
  - `bozulan_dosya_okunmaz`: saklanan dosyanın baytları değiştirilir; `oku` `InvalidData` döner ve ileti "önceki hâl bozuk" içerir.
  - `ayni_icerik_bir_kez_saklanir`: iki `sakla` sonrasında tek dosya vardır ve `boyut` içeriğin uzunluğudur.
  - `gecersiz_ozet_adi_reddedilir`: `../x`, büyük harfli ad, 63 karakterlik ad, `/` içeren ad.
  - `kullanilmayan_ve_yarim_dosyalar_temizlenir`
  - `#[cfg(unix)] depo_izinleri_yalnizca_kullaniciya_acik`: dizin `0700`, dosya `0600`.
- [ ] **Step 2:** Testlerin başarısız olduğunu gör.
- [ ] **Step 3:** Uygula.
- [ ] **Step 4:** Global Constraints'teki denetimler, iki platformda.
- [ ] **Step 5:** Commit `çekirdek: geri alma için önceki hâllerin deposu`.

### Task 2: Dosya araçlarının hazırlığında önceki içerik ve sonraki özet

**Files:**
- `crates/kalfa-core/src/arac.rs` (`YazmaHedefi`), `crates/kalfa-core/src/test_destek.rs` (`SahteArac`)
- `crates/kalfa-tools/src/{yaz,duzenle,calisma_alani}.rs`
- Testler: `crates/kalfa-tools/tests/yazma_araclari.rs`, `crates/kalfa-core/tests/arac_muhur.rs`

**Interfaces:**
- **Consumes:** Task 1'in `DOSYA_SINIRI`'si.
- **Produces:**
  - **`YazmaHedefi` eklemeleri:** `onceki_icerik: Option<Arc<[u8]>>`, `buyuk: bool`, `sonraki_ozet: Option<String>`, `olusacak_dizinler: Vec<CalismaYolu>`.
    - `CalismaYolu`'nun `Default`'u olmadığı için `YazmaHedefi` de `Default` almaz; bütün kurucular güncellenir.
  - **`CalismaAlani::olusacak_dizinler(&self, yol: &CalismaYolu) -> io::Result<Vec<CalismaYolu>>`:** yolun henüz olmayan üst dizinleri, sığdan derine.
    - Varlık, bağlantı izlenmeden denetlenir (`open_dir_nofollow`).
    - Üst dizinlerden biri bağlantıysa hata döner; yazma zaten başarısız olurdu.
  - **`dosya_yaz`:** `sonraki_ozet = ozet_hex(icerik)`. Önceki içerik `DOSYA_SINIRI`'ne eşit ya da küçükse `onceki_icerik` olur; büyükse `buyuk = true` olur ve `onceki_ozet` yine verilir.
  - **`dosya_duzenle`:** `sonraki_ozet`, düzenlenmiş metnin özetidir; `onceki_icerik` okunan `veri`'dir.
  - **Mühür değişmez:** `muhur_hesapla` yeni alanları okumaz.

- [ ] **Step 1: Başarısız testleri yaz**
  - `yazma_hazirligi_onceki_icerigi_ve_sonraki_ozeti_tasir`
  - `yeni_dosyanin_hazirligi_olusacak_dizinleri_bildirir`: hiçbiri yokken `a/b/c.txt` için `[a, a/b]`; `a` varken `[a/b]`.
  - `duzenleme_hazirligi_sonraki_ozeti_tasir`
  - `buyuk_dosyanin_onceki_icerigi_tasinmaz`: 8 MiB + 1 baytlık dosya; `buyuk` doğru, `onceki_icerik` yok, `onceki_ozet` var.
  - `muhur_yeni_alanlardan_etkilenmez` (core): yalnızca yeni alanlarda farklı iki hazırlığın mührü aynıdır.
- [ ] **Step 2:** Testlerin başarısız olduğunu gör.
- [ ] **Step 3:** Uygula.
- [ ] **Step 4:** Denetimler, iki platformda.
- [ ] **Step 5:** Commit `araçlar: yazma hazırlığı önceki içeriği ve sonraki özeti taşır`.

### Task 3: Oturumda yakalama, kayıt ve büyük dosya kuralı

**Files:**
- `crates/kalfa-core/src/{geri_alma,kayit,izin,kurtarma}.rs`, `crates/kalfa-core/src/oturum/{mod,arac_asamasi}.rs`
- Testler: `crates/kalfa-core/tests/geri_alma_yakalama.rs` (yeni), `crates/kalfa-tools/tests/uctan_uca.rs`

**Interfaces:**
- **Consumes:** Task 1 ve Task 2.
- **Produces:**
  - **`OturumAyarlari.geri_alma_dizini: Option<PathBuf>`:**
    - `OturumAyarlari::yeni`'de `None`'dır; kayıtsız oturumda (`kayit_yolu: None`) yok sayılır.
    - `baslat_kimlikle` ve `devam_et` depoyu, kaydı açtıkları yerde açar ve `Oturum::kur`'a verir (`Oturum.depo: Option<OncekiHalDeposu>`). `Oturum::kur` `Result` dönmez; depo açılamazsa bu iki işlev `OturumHatasi::Kayit` döner ve oturum başlamaz.
  - **`Sinirlar`'a eklenenler:** `geri_alma_oturum: u64` (varsayılan `OTURUM_SINIRI`) ve `geri_alma_oturum_alt: u64` (varsayılan `OTURUM_ALT_SINIRI`). Testler bunları küçültür. Dosya sınırı (`DOSYA_SINIRI`) sabittir: araçlar onu oturumdan bağımsız uygular.
  - **`geri_alma::DosyaYedegi`:** `{ yol, onceki: OncekiHal, sonraki: Option<String>, olusan_dizinler, eklenen: u32, silinen: u32, diff: Option<String> }`.
    - `OncekiHal ::= Yoktu | Saklandi { ozet } | Buyuk { ozet, bayt }`; serde'de `tip` etiketli.
  - **`KayitOgesi::AracBasladi`'ya** `yedekler: Vec<DosyaYedegi>` eklenir: `#[serde(default, skip_serializing_if = "Vec::is_empty")]`.
  - **`Oturum::yurut`'ta sıra** (yazan araç, depo var):
    1. Her `onceki_icerik` için `depo.sakla(onceki_ozet, …)`. Hata olursa sonuç `[çalıştırılmadı: önceki hâl saklanamadı: <hata>]`, durum `Basarisiz` ve araç çalışmaz.
    2. `AracBasladi { .., yedekler }` `kaydet_kalici` ile yazılır.
  - **`geri_alma::diff_sayilari(diff: &str) -> (u32, u32)`:** başlık satırları hariç `+` ve `-` satırlarını sayar.
    - `diff`, `DIFF_SINIRI`'nin altındaysa ve yol gizli değilse (`Siniflandirici::gizli_mi`) kayda girer.
  - **`izin::GERI_ALINAMAZ_UYARISI`:** "Bu dosyanın önceki hâli geri alma için saklanamayacak kadar büyük (8 MiB üstü); bu değişiklik geri alınamaz."
  - **`Oturum::karar(kip, s, hazirlik, geri_alma: bool)`:** `geri_alma` doğruysa ve hedeflerden biri `buyuk` ise karara `Onayli { GERI_ALINAMAZ_UYARISI }` birleştirilir.
    - `Planlanan` bu yüzden `geri_alma`'yı da taşır; `tek_calistir`'daki karşılaştırma aynı kuralı uygular.
  - **Kurtarma:** `devam_et` depoyu açar ve kayıtta adı geçmeyen dosyaları siler (`kullanilmayanlari_sil`).

- [ ] **Step 1: Başarısız testleri yaz**
  - `yazmadan_once_onceki_hal_depoda_ve_kayitta` (gerçek araçlarla, Proje kipi, var olan dosyaya `dosya_yaz`):
    - Depoda eski içerik vardır.
    - `AracBasladi.yedekler`'de `Saklandi { ozet }` ve `sonraki` vardır.
    - Dosya yeni içeriktedir.
  - `yeni_dosya_yoktu_olarak_kaydedilir`
  - `depo_yazilamazsa_arac_calismaz`: `geri_alma_dizini` bir dosyayı gösterir. Sonuç "önceki hâl saklanamadı" içerir, dosya değişmez.
  - `proje_kipinde_buyuk_dosya_yazmasi_onay_ister`: `OnayGerekiyor` `GERI_ALINAMAZ_UYARISI` taşır.
  - `kayitsiz_oturumda_yedek_yok_ve_buyuk_dosya_onaysiz`
  - `gizli_dosyanin_diffi_kayda_yazilmaz`: Proje kipinde `.env`'e yazılır; yedekte `diff` yoktur, önceki hâl depodadır.
  - `devam_ederken_kaydi_olmayan_depo_dosyalari_silinir`
  - `eski_kayit_yedeksiz_acilir`: `yedekler` alanı olmayan bir kayıt açılır ve geçmişi gösterilir.
- [ ] **Step 2:** Testlerin başarısız olduğunu gör.
- [ ] **Step 3:** Uygula.
- [ ] **Step 4:** Denetimler, iki platformda.
- [ ] **Step 5:** Commit `çekirdek: yazmadan önce önceki hâl depoya ve başladı kaydına; büyük dosya onay ister`.

### Task 4: Çalışma alanına denetimli geri yazma (`GeriYukleyici`)

**Files:**
- `crates/kalfa-core/src/geri_alma.rs` (trait), `crates/kalfa-tools/src/calisma_alani.rs` (uygulama)
- Testler: `crates/kalfa-tools/tests/geri_yukleme.rs` (yeni)

**Interfaces (Produces):**
- **`DosyaDurumu { ozet: String, baglanti_sayisi: u64 }`.**
- **`trait GeriYukleyici: Send + Sync`:**
  - `durum(&self, yol: &CalismaYolu) -> io::Result<Option<DosyaDurumu>>`.
  - `geri_yaz(&self, yol: &CalismaYolu, icerik: Option<&[u8]>, beklenen: &str) -> io::Result<()>`.
  - `bos_dizini_kaldir(&self, yol: &CalismaYolu) -> io::Result<bool>`.
- **`impl GeriYukleyici for CalismaAlani`:**
  - **`durum`:** dosyayı bağlantı izlemeden açar (`dosya_ac`). Dönen değer içeriğin özeti ve `baglanti::sayi`'dır. Dosya yoksa `None`; normal dosya değilse `PermissionDenied`.
  - **`geri_yaz`:**
    - Yol `git_kimligi` ve `ust_dizin_denetimli(yol, false, …)` ile açılır: eksik dizin oluşturulmaz.
    - Dosya bağlantı izlenmeden okunup yazılmak üzere açılır.
    - Şimdiki özet `beklenen` değilse, ya da bağlantı sayısı birden büyükse, `PermissionDenied` döner ve ileti nedeni söyler.
    - `Some` içerik `set_len(0)` ve `write_all` ile yazılır. `None` içerik `remove_file` ile silinir; silmeden hemen önce özet denetlenir.
    - `.git` takma adı `git_takma_adi_reddet` ile reddedilir.
  - **`bos_dizini_kaldir`:** `remove_dir` çağrılır. `DirectoryNotEmpty` ya da `NotFound` gelirse `Ok(false)` döner.

- [ ] **Step 1: Başarısız testleri yaz**
  - `geri_yaz_onceki_icerigi_koyar`
  - `geri_yaz_ozet_tutmazsa_yazmaz`
  - `geri_yaz_icerik_yoksa_dosyayi_siler`
  - `hard_linkli_dosyaya_geri_yazilmaz`
  - `#[cfg(unix)] baglantili_ust_dizinden_geri_yazilamaz`
  - `#[cfg(windows)] junction_uzerinden_geri_yazilamaz`
  - `#[cfg(windows)] kisa_ad_ile_git_dizinine_geri_yazilamaz`: sistem sürücüsünde 8.3 adları kapalıysa test atlanır ve bunu yazar.
  - `bos_dizin_kaldirilir_dolu_dizin_kalir`
- [ ] **Step 2:** Testlerin başarısız olduğunu gör.
- [ ] **Step 3:** Uygula.
- [ ] **Step 4:** Denetimler, iki platformda (junction ve kısa ad testleri Windows'ta).
- [ ] **Step 5:** Commit `araçlar: geri alma için çalışma alanına denetimli geri yazma`.

### Task 5: Geri alma planı, mührü ve yürütmesi

**Files:**
- `crates/kalfa-core/src/{geri_alma,protokol,kayit,kurtarma,aktarim}.rs`, `crates/kalfa-core/src/oturum/{mod,geri_alma}.rs`, `crates/kalfa-core/src/gecmis/mod.rs`
- `crates/kalfa-core/src/test_destek.rs`: `SahteGeriYukleyici` (bellekte yol → içerik ve bağlantı sayısı)
- Testler: `crates/kalfa-core/tests/geri_alma_plani.rs` (yeni), `crates/kalfa-tools/tests/uctan_uca.rs`

**Interfaces:**
- **Consumes:** Task 1, 3 ve 4.
- **Produces:**
  - **`OturumBaglami.geri_yukleyici: Option<Arc<dyn GeriYukleyici>>`:** `None` ise geri alma kullanılamaz. `test_destek::test_baglami` `None` verir; geri alma testleri sahte ya da gerçek yükleyiciyi kendisi verir.
  - **`GeriAlmaDizini`** (oturumun belleğinde):
    - Her tur için: tur kimliği, ilk kullanıcı mesajının ilk 60 karakteri, `(CagriId, AracDurumu, DosyaYedegi)` listesi ve kabuk değişiklikleri (Task 6). Durum `AracBitti`'den gelir; kaydı olmayan çağrı `SonucuBilinmiyor`'dur.
    - Verisi silinen turlar.
    - Canlı olarak da, kayıttan da (`kurtar`) kurulur.
  - **Plan tipleri:**
    - `PlanOgesi { yol, eylem, simdiki: Option<String>, uyarilar: Vec<String>, diff: Option<String> }`
    - `eylem ::= GeriYukle | Sil | ZatenOnceki | Cakisma { neden } | GeriAlinamaz { neden }`
    - `CakismaNedeni ::= SonrakiTur { tur } | Komut | BasarisizCagri | Elle | HardLink | Okunamadi { hata }`
    - `GeriAlinamazNedeni ::= Buyuk | VeriSilindi`
  - **`plan_cikar(tur, &GeriAlmaDizini, &OncekiHalDeposu, &dyn GeriYukleyici, &Siniflandirici) -> io::Result<GeriAlmaPlani>`:** tasarımın §5.7 tablosunu uygular.
    - `S` yalnızca `Basarili` ve `SonucuBilinmiyor` çağrıların sonraki özetleridir; `Basarisiz` çağrılarınki `F`'dir ve eşleşmesi `Cakisma(BasarisizCagri)` olur. `IptalEdildi` çağrılar `A`'yı belirlemez.
    - `uyarilar`: korumalı yolda `KORUMALI_UYARISI`, yapılandırma dosyasında `YAPILANDIRMA_UYARISI`, gizli dosyada "gizli dosya; içerik gösterilmiyor", sonucu bilinmeyen çağrının dosyasında "Bu dosyaya yazan çağrının sonucu bilinmiyor."
    - `diff`'lerin toplamı en çok 1 MiB'tır; aşan öğelerde `None`.
  - **`plan_muhru(tur, &GeriAlmaPlani) -> OnayMuhru`:** SHA-256'ya girenler: `kalfa-geri-alma-v1`, tur ve her öğenin yolu, eylemi ve `simdiki` değeri.
  - **Protokol:**
    - `Komut::GeriAlmaOnizle { tur }`, `Komut::TuruGeriAl { tur, muhur }`.
    - `Olay::GeriAlmaPlani { tur, plan, muhur }`, `Olay::TurGeriAlindi { tur, sonuc: Vec<GeriAlmaSonucu> }`, `Olay::GeriAlmaVerisiSilindi { turlar }`. Üçü de `kritik_mi`'dir.
  - **Kayıt:** `GeriAlmaBasladi { tur, muhur }` (kalıcı), `GeriAlindi { tur, sonuc }`, `KalfaNotu { metin }`, `GeriAlmaVerisiSilindi { turlar }`.
    - `KalfaNotu` `Mesaj` değildir: kullanıcı rolündeki `Mesaj` başlığı, oturum listesinin "yarım kaldı" durumunu (`gecmis::liste`) ve kullanıcı balonunu etkilerdi.
  - **Görünüm:** `GorunumOgesi::GeriAlma { sira, tur, geri_alinan_tur, sonuc, yarim: bool }` ve `GorunumOgesi::KalfaNotu { sira, tur, metin }`.
    - `tur`, öteki öğelerdeki gibi kaydın kendi turudur (sayfalama bunu kullanır; geri alma tur dışında yapıldığı için boştur). Geri alınan tur `geri_alinan_tur`'dadır.
    - Kayıt öğelerinde (`GeriAlmaBasladi`, `GeriAlindi`) ve olaylarda da alan adı `geri_alinan_tur`'dur. Zarfın kendi `tur` alanıyla karışmaz.
  - **Komutlar tur sürerken:** `GeriAlmaOnizle` ve `TuruGeriAl`, `self.tur` doluyken `Olay::Hata` ile reddedilir: "Önceki tur sürüyor; bitmesini bekleyin ya da iptal edin."
  - **Yürütme:**
    1. `kilitler.al(&[], &yollar, false)` ile plandaki yolların yazma kilitleri alınır.
    2. Plan yeniden çıkarılır ve mühür karşılaştırılır. Tutmazsa yeni `GeriAlmaPlani` yayılır, ileti "Dosyalar önizlemeden sonra değişti" der ve hiçbir şey yazılmaz.
    3. `GeriAlmaBasladi` kalıcı yazılır.
    4. Öğeler yazılır: `GeriYukle` için depodaki içerik `oku` ile okunur (özet doğrulanır) ve `geri_yaz` çağrılır.
    5. `olusan_dizinler` derinden sığa `bos_dizini_kaldir` ile kaldırılır.
    6. `GeriAlindi` ve `KalfaNotu` yazılır; not bağlama `ChatMessage::user("[Kalfa] …")` olarak eklenir; `TurGeriAlindi` yayılır.
  - **Sınır aşımı:** her turun sonunda depo `Sinirlar.geri_alma_oturum`'u aşmışsa, `Sinirlar.geri_alma_oturum_alt`'a inene kadar en eski turların verisi silinir.
    - Kalan turların kullandığı içerik silinmez.
    - Kayda `GeriAlmaVerisiSilindi` yazılır ve aynı olay yayılır; arayüz o turların özetlerini günceller (Task 10).

- [ ] **Step 1: Başarısız testleri yaz** (sahte `GeriYukleyici` ile; sonuncusu gerçek araçlarla)
  - `plan_tablosu`: §5.7'nin her satırı — `ZatenOnceki`, `GeriYukle`, `Sil`, `GeriAlinamaz(Buyuk)`, `GeriAlinamaz(VeriSilindi)`, `Cakisma(BasarisizCagri)`, `Cakisma(Elle)`, `Cakisma(HardLink)`.
  - `ara_hal_de_geri_alinir`: A → B → C yazılmış ve dosya B'deyse eylem `GeriYukle(A)` olur.
  - `basarisiz_cagrinin_hali_kullanicinin_yazmasi_sayilmaz`: A → B çağrısı başarısız, sonraki çağrı C yazar, kullanıcı dosyayı B yapar; eylem `Cakisma(BasarisizCagri)` olur, dosya değişmez.
  - `sonucu_bilinmeyen_cagrinin_hali_geri_alinir`: `AracBitti`'si olmayan çağrının sonraki hâlindeki dosya `GeriYukle` olur ve uyarı taşır.
  - `sonraki_turla_cakisma_nedeni`
  - `onizlemeden_sonra_degisen_dosya_geri_almayi_durdurur`: yeni plan yayılır, dosya değişmez.
  - `tur_surerken_geri_alma_reddedilir`
  - `geri_alma_sonrasi_kalfa_notu_baglamda_ve_kayitta`: sahte sağlayıcının sonraki isteğinde not vardır. Oturumun başlığı ve listedeki durumu değişmez.
  - `yarim_kalan_geri_alma_yeniden_istenebilir`: `GeriAlmaBasladi` var ama `GeriAlindi` yok. Kurtarmada `yarim` işaretlidir; ikinci geri almada yapılmış olanlar `ZatenOnceki` olur.
  - `oturum_siniri_asilinca_en_eski_turun_verisi_silinir_paylasilan_kalir`: sınır `Sinirlar.geri_alma_oturum` ile küçültülür; kayıt yazılır ve `GeriAlmaVerisiSilindi` olayı yayılır.
  - `kayittan_acilan_oturumda_geri_alma` (gerçek araçlarla, `devam_et`).
- [ ] **Step 2:** Testlerin başarısız olduğunu gör.
- [ ] **Step 3:** Uygula.
- [ ] **Step 4:** Denetimler, iki platformda.
- [ ] **Step 5:** Commit `çekirdek: turu geri alma — önizleme, mühür ve yürütme`.

### Task 6: Kabuğun değiştirdiği dosyalar

**Files:**
- `crates/kalfa-sandbox/src/tarama.rs`
- `crates/kalfa-tools/src/kabuk/mod.rs`
- `crates/kalfa-core/src/{arac,kayit}.rs`, `crates/kalfa-core/src/oturum/arac_asamasi.rs`
- Testler: `tarama.rs` içindeki birim testleri, `crates/kalfa-tools/tests/kabuk.rs`, `crates/kalfa-tools/tests/uyumluluk.rs` (yalıtım özellikli), `crates/kalfa-core/tests/onaysiz_degisiklik.rs`

**Interfaces (Produces):**
- **`tarama::Goruntu`:** `{ girdiler: BTreeMap<PathBuf, Girdi>, eksik: bool }`; yollar köke görelidir.
  - `Girdi { tur: GirdiTuru, boyut: u64, zaman_ns: u128, kimlik: Option<(u64, u64)> }`.
  - `GirdiTuru ::= Dosya | Dizin | Baglanti`.
  - `kimlik` Unix'te `(dev, ino)`, Windows'ta `None`.
- **`Tarama.goruntu`** ve **`AdliGirdiler.goruntu`:** aynı taramada toplanır; `.git`'in içi girmez.
- **Sınırlar:** `GORUNTU_AZAMI_GIRDI = 500_000` ve `GORUNTU_AZAMI_SURE = 5 sn`. Aşılırsa `eksik = true` olur.
- **`tarama::karsilastir(once, sonra) -> Vec<Degisiklik { yol, tur }>`:**
  - `tur ::= Eklendi | Degisti | Silindi`.
  - Dosya ve bağlantılar karşılaştırılır. Boyut, zaman ya da kimlik farkı "değişti" sayılır.
- **`kalfa_core::arac::DosyaDegisiklikleri`:**
  - `{ dosyalar: Vec<DegisenDosya { yol: String, tur }>, dizinler: Vec<DizinSayisi { dizin, eklenen, degisen, silinen }>, eksik: bool }`.
  - Tek tek en çok `DEGISIKLIK_LISTE_SINIRI = 200` yol listelenir. Bağımlılık dizinlerinin (`BAGIMLILIK_DIZINLERI`) dışındaki dosyalar önce gelir.
- **`AracCiktisi.degisiklikler: Option<DosyaDegisiklikleri>`:** kabuk bunu hem `Ok`'ta hem `Basarisiz`'da doldurur.
- **Oturum:**
  - Kayda `KabukDegisiklikleri { cagri, degisiklikler }` yazar ve bunu turun biriktiricisine ekler.
  - Karar `Serbest` ise değişen dosyaları `onaysiz_yazilanlar`'a ekler.

- [ ] **Step 1: Başarısız testleri yaz**
  - `goruntu_eklenen_silinen_degisen_bulur` (sandbox)
  - `onceden_degismis_dosyanin_yeniden_degismesi_bulunur`: `M` durumu.
  - `gitignore_kapsamindaki_dosya_bulunur`
  - `git_ici_goruntuye_girmez`
  - `sinir_asilinca_goruntu_eksik`: test için küçük bir sınırla (`goruntu_sinirli`).
  - `kabugun_degistirdigi_dosyalar_sonuca_eklenir` (tools; yalıtımsız arka uçla betik).
  - `onaysiz_kabugun_degistirdigi_dosyalar_sonraki_kabuk_onayinda_listelenir` (core, sahte kabuk).
  - `yalitimli_kabugun_degisiklikleri_bulunur` (uyumluluk, gerçek yalıtım): Linux'ta `kalfatest`, Windows'ta kurulu makine.
- [ ] **Step 2:** Testlerin başarısız olduğunu gör.
- [ ] **Step 3:** Uygula.
- [ ] **Step 4: Denetimler ve ölçüm (G1)**
  - Denetimler iki platformda koşar; yalıtım testleri root olmayan kullanıcıyla.
  - Ölçüm: 100 000 dosyalık ağaçta taramanın `stat`'lı ve `stat`'sız süresi, Windows ve WSL'de. Sonuç "Uygulama sonucu"na yazılır.
- [ ] **Step 5:** Commit'ler:
  - `sandbox: kabuk öncesi ve sonrası meta veri görüntüsü`
  - `araçlar: kabuğun değiştirdiği dosyalar sonuca ve onaysız değişikliklere`

### Task 7: Tur özeti

**Files:**
- `crates/kalfa-core/src/tur_ozeti.rs` (yeni)
- `crates/kalfa-core/src/oturum/{mod,arac_asamasi,model_turu}.rs`
- `crates/kalfa-core/src/{protokol,kayit,aktarim}.rs`, `crates/kalfa-core/src/gecmis/mod.rs`
- Testler: `crates/kalfa-core/tests/tur_ozeti.rs` (yeni), `crates/kalfa-core/tests/gecmis.rs`

**Interfaces:**
- **Consumes:** Task 3, 5 ve 6.
- **Produces:**
  - **`TurOzeti`:** tasarımın §5.5 bölümündeki biçim.
    - `GeriAlinabilirlik ::= Evet | Hayir { neden: Buyuk | Kayitsiz | VeriSilindi | Kabuk }`.
  - **`TurBiriktirici`** (`Oturum`'da, `tur_calistir`'ın başında sıfırlanır). Beslendiği yerler:
    - `yurut`: yedekler ve her çağrının son durumu.
    - Kabuk değişiklikleri.
    - `Kullanim` kayıtları ve istek sayısı.
    - Güvenlik uyarısı sayısı.
  - **Kayıt ve olay:** `KayitOgesi::TurOzeti { ozet }` ve `Olay::TurOzeti { ozet }` (`kritik_mi`).
    - `TurBitti` ve `Tamamlandi`'dan sonra, `self.tur` hâlâ doluyken üretilir.
    - Oturum listesi son durumu `TurBitti`'den okumaya devam eder.
  - **Görünüm:** `GorunumOgesi::TurOzeti { sira, tur, ozet }`. `gorunum_uret` kayıttaki `GeriAlmaVerisiSilindi`'yi önceki özetlere uygular: o turların dosyaları `Hayir { neden: VeriSilindi }` olur.
  - **Dosyalar** yalnızca başarılı ya da sonucu bilinmeyen yazma çağrılarından gelir. Sonucu bilinmeyen dosya `sonucu_bilinmiyor` diye işaretlenir.

- [ ] **Step 1: Başarısız testleri yaz**
  - `tur_ozeti_dosyalari_komutlari_ve_istekleri_tasir`
  - `basarisiz_yazma_ozette_yok`
  - `iptal_edilen_turun_ozeti_kaydedilir`
  - `bos_tur_ozeti_kaydedilir`
  - `tur_ozeti_turun_son_ogesidir_ve_liste_durumunu_degistirmez`: `gorunum_uret`'te özet `TurSonu`'ndan sonra gelir; `oturum_ozeti` durumu `Tamamlandi` kalır.
  - `eski_kayit_ozetsiz_acilir`
  - `veri_silinen_turun_ozeti_gecmiste_geri_alinamaz`
- [ ] **Step 2:** Testlerin başarısız olduğunu gör.
- [ ] **Step 3:** Uygula.
- [ ] **Step 4:** Denetimler, iki platformda.
- [ ] **Step 5:** Commit `çekirdek: tur özeti`.

### Task 8: Onay kartında geri alınabilirlik

**Files:**
- `crates/kalfa-core/src/protokol.rs` (`OnayOgesi`), `crates/kalfa-core/src/oturum/arac_asamasi.rs` (`onay_topla`)
- Testler: `crates/kalfa-core/tests/geri_alma_yakalama.rs`

**Interfaces (Produces):**
- **`OnayOgesi.geri_alinabilir: Option<GeriAlinabilirlik>`** (`#[serde(default)]`):
  - Yazan dosya öğesi: `Evet`; `buyuk` ise `Hayir(Buyuk)`; kayıtsız oturumda `Hayir(Kayitsiz)`.
  - Kabuk öğesi: `Hayir(Kabuk)`.
  - Yalnızca okuyan öğe: `None`.

- [ ] **Step 1: Başarısız testleri yaz**
  - `onay_ogesi_geri_alinabilirligi`: kayıtlı oturum, kayıtsız oturum, büyük dosya, kabuk ve okuma.
- [ ] **Step 2:** Testlerin başarısız olduğunu gör.
- [ ] **Step 3:** Uygula.
- [ ] **Step 4:** Denetimler.
- [ ] **Step 5:** Commit `çekirdek: onay öğesi geri alınabilirliği taşır`.

### Task 9: Uygulama — depo dizini, komutlar, oturumu silme

**Files:**
- `crates/kalfa-uygulama/src/{dizin,oturumlar,sozlesme,uygulama}.rs`
- `crates/kalfa/src/{lib,komutlar,komut_adlari}.rs`, `crates/kalfa/capabilities/ana-pencere.json`
- Testler: `crates/kalfa-uygulama/tests/oturum_tablosu.rs`, `crates/kalfa/tests/izinler.rs`

**Interfaces:**
- **Consumes:** Task 3–5.
- **Produces:**
  - **`dizin::geri_alma_dizini(veri, yerel_veri, ortam) -> PathBuf`:** `yalitim_dizini`'nin kuralıyla. Windows'ta `%LOCALAPPDATA%\Kalfa\geri-alma`; Linux'ta ve `KALFA_VERI_DIZINI` verildiğinde `<veri>/geri-alma`.
  - **Kurulum:**
    - `Bilesenler.geri_alma_dizini` ve `Uygulama.geri_alma_dizini`.
    - `kalfa/src/lib.rs` dizini `yalitim_dizini` gibi hesaplar.
  - **`Uygulama::baglam`** `geri_yukleyici: Some(alan)` verir.
  - **`oturum_baslat` ve `oturum_ac`** `ayarlar.geri_alma_dizini = Some(<kök>/<oturum>)` verir.
  - **Komutlar** yalnızca ana pencereye açıktır:
    - `geri_alma_onizle(oturum, tur)` → `Komut::GeriAlmaOnizle`.
    - `turu_geri_al(oturum, tur, muhur)` → `Komut::TuruGeriAl`.
    - `oturumu_sil(oturum)`:
      1. Yerel pencere ister: "Oturum, kaydı ve geri alma verisi kalıcı olarak silinecek. Bu oturumun turları artık geri alınamaz."
      2. Oturum açıksa önce `oturumu_kapat` yolundan kapatılır.
      3. Kayıt kilidi alınır (`OturumKaydi::devam_icin_ac`); alınamazsa `KayitMesgul` döner.
      4. Kayıt, geri alma dizini ve kilit dosyası silinir.
    - `geri_alma_verisi_boyutu()` → bayt.
  - **Arayüz** yalnızca oturum ve tur kimliğini, mührü gönderir; yol göndermez.

- [ ] **Step 1: Başarısız testleri yaz**
  - `oturumu_sil_kaydi_ve_geri_alma_verisini_siler`
  - `acik_oturum_silinmeden_once_kapatilir`
  - `yerel_onay_reddedilirse_oturum_silinmez`
  - `baska_yerde_acik_kayit_silinmez`
  - `geri_alma_dizini_platforma_gore`
  - `izinler.rs`: yeni komutlar ana pencerede izinli, şart penceresinde değil.
- [ ] **Step 2:** Testlerin başarısız olduğunu gör.
- [ ] **Step 3:** Uygula.
- [ ] **Step 4:** Denetimler; `e2e` özellikli testler ve clippy.
- [ ] **Step 5:** Commit `uygulama: geri alma komutları ve oturumu silme`.

### Task 10: Arayüz

**Files:**
- `ui/src/ipc/{tipler,ana}.ts`, `ui/src/durum/oturum.ts`
- `ui/src/bilesenler/TurnSummary.svelte` (yeni), `ui/src/ana/{TurOzeti,GeriAlmaSayfasi}.svelte` (yeni)
- `ui/src/ana/{Konusma,KonusmaOgesi,OnaySayfasi,OturumListesi}.svelte`, `ui/src/dil/tr.ts`, `ui/src/sahte/arka.ts`, `ui/src/onizleme/senaryo.ts`
- Testler: `ui/src/durum/oturum.test.ts`, `ui/src/ana/ekranlar.test.ts`, `ui/src/bilesenler/sohbet.test.ts`

**Interfaces:**
- **`EkranOgesi` eklemeleri:** `TurOzeti`, `GeriAlma` ve `KalfaNotu`.
  - `uygula` şu olayları işler: `TurOzeti`, `GeriAlmaPlani` (sayfanın açılacağını durumda tutar), `TurGeriAlindi` ve `GeriAlmaVerisiSilindi` (o turların özetlerinden "Turu geri al" kalkar).
- **`TurnSummary`:** tasarım sistemindeki `TurnSummary.d.ts`'e göre (`items`, `onUndo`, `note`).
  - `onUndo` yalnızca en az bir dosya geri alınabiliyorsa verilir.
- **`GeriAlmaSayfasi`:** onay sayfasının düzeninde.
  - İçerik: öğeler ve eylemleri, geri alma yönünde diff, çakışma nedeni, `uyarilar`, gizli dosyada içerik yok, kabuk değişiklikleri (salt liste).
  - Düğmeler: "Geri al" ve "Vazgeç".
- **Onay sayfası:**
  - Öğe başına "Geri alınabilir mi" satırı.
  - Seçili öğelerin hepsi `Evet` ise başlıkta "Bu değişiklikler geri alınabilir." cümlesi.
  - `OnaySayfasi.svelte`'nin açıklamasındaki "geri alma gelene kadar yazılmaz" notu kaldırılır.
- **Oturum listesi:** "Sil" (`oturumu_sil`).
- **Metinler** `tr.ts`'te, "siz" diliyle: "Turu geri al", "Geri alınamaz", "Çakışma", "Önceki hâl", "Kabukla yapılan değişiklikler geri alınamaz.", "Liste eksik olabilir."

- [ ] **Step 1: Başarısız testleri yaz**
  - Durum (`oturum.test.ts`):
    - Tur özeti olayı ve geçmiş öğesi aynı ekran öğesini üretir.
    - `TurGeriAlindi` özeti günceller.
    - `GeriAlmaVerisiSilindi` ekrandaki eski özetin "Turu geri al" düğmesini kaldırır.
  - Ekranlar (`ekranlar.test.ts`):
    - Tur özeti satırları ve notu çizilir.
    - "Turu geri al…" geri alınamayan turda yoktur.
    - Geri alma sayfası eylemleri, çakışma nedenini ve gizli dosyada içeriğin gösterilmediğini çizer.
    - Onay sayfası geri alınabilirlik satırını ve başlık cümlesini gösterir; kabuk öğesi varken cümle yoktur.
    - Oturum listesinde "Sil" vardır.
- [ ] **Step 2:** Testlerin başarısız olduğunu gör.
- [ ] **Step 3:** Uygula. Ekran önizlemesine (`ui/onizleme.html`) tur özeti ve geri alma senaryosu eklenir.
- [ ] **Step 4:** `npm run check` (0 hata, 0 uyarı), `npm test`, `npm run build`.
- [ ] **Step 5:** Commit `ui: tur özeti, geri alma sayfası ve onayda geri alınabilirlik`.

### Task 11: Uçtan uca

**Files:**
- `e2e/testler/06-geri-alma.test.mjs` (yeni)
- Gerekirse `crates/kalfa-uygulama/src/e2e/senaryo.rs`

**Senaryolar** (sahte EVREN; senaryo kullanıcı mesajıyla seçilir):
- **Geri alma:**
  1. Proje kipinde model bir dosyayı yazar ve bir dosya oluşturur.
  2. Tur özeti görünür.
  3. "Turu geri al…" ve "Geri al" tıklanır.
  4. Yazılan dosya eski içeriğine döner, oluşturulan dosya silinir.
  5. Konuşmada Kalfa notu görünür.
- **Çakışma:** tur yazar, test diskteki dosyayı değiştirir, geri alma denenir. Sayfa çakışmayı gösterir ve dosya değişmez.
- **Kayıttan açma:** tur yazar, oturum kapatılıp kayıttan açılır, geri alma çalışır.
- **Oturumu silme:** sahte yerel pencere onaylar; kayıt dosyası ve geri alma dizini yok olur.

- [ ] **Step 1:** Testleri yaz ve başarısız olduklarını gör.
- [ ] **Step 2:** Gerekirse senaryo adımı ekle.
- [ ] **Step 3:** `cd e2e && npm test`: Windows'ta ve Linux'ta (`xvfb-run -a npm test`). Gecikme testleri eski eşikleriyle geçmeli.
- [ ] **Step 4:** Commit `e2e: turu geri alma`.

### Task 12: Belgeler

**Files:**
- `docs/superpowers/specs/2026-09-28-kalfa-3-gorunurluk-design.md` (§5, §13: uygulamada netleşenler ve G1, G2, G10, G12'nin sonuçları)
- `docs/superpowers/specs/2026-09-21-kalfa-design.md` (gerekirse §6.7)
- `docs/GELISTIRME_DURUMU.md`

- [ ] **Step 1:** Tasarım belgesine uygulamada netleşenleri ve ölçümleri yaz.
- [ ] **Step 2:** `docs/GELISTIRME_DURUMU.md`'ye 3a'nın kaydını ve iki platformdaki doğrulama tablosunu yaz. Commit'lemeden önce `git diff` ile başkasının düzenlemesi var mı bakılır.
- [ ] **Step 3:** Commit `docs: 3a — tur özeti ve geri alma`.

## Kabul ölçütleri

- **Geri alma:**
  - Proje kipinde turun dosya aracı değişiklikleri geri alınır: değişen dosya eski içeriğine döner, oluşturulan dosya silinir, turun oluşturduğu boş dizinler kalkar.
  - Sonradan elle değişmiş dosyaya dokunulmaz ve çakışma nedeniyle listelenir.
  - Aynı dosyaya yazan sonraki bir tur varsa ileti önce o turun geri alınmasını söyler.
- **Kalıcılık:**
  - Önceki hâl yazmadan önce depoda ve kayıttadır. Depo yazılamazsa araç çalışmaz.
  - Kalfa çöktükten sonra kayıttan açılan oturumda da geri alma çalışır.
  - Yarım kalan geri alma yeniden istenebilir ve hiçbir dosyayı iki kez değiştirmez.
- **Güvenlik:**
  - Geri yazma bağlantılı üst dizinden, junction'dan, `.git` takma adından ve hard link'ten geçemez.
  - Depo yalıtımda görünmez; hiçbir EVREN isteğine girmez.
  - Gizli dosyanın diff'i kayıtta yoktur.
- **Onay:**
  - 8 MiB'tan büyük dosyaya yazmak kayıtlı oturumda her kipte onay ister.
  - Onay kartı her yazan öğe için geri alınabilirliği söyler. Hepsi geri alınabiliyorsa "Bu değişiklikler geri alınabilir." cümlesi görünür.
- **Kabuk:** kabuğun değiştirdiği dosyalar, önceden değişmiş (`M`) ve `.gitignore` kapsamındakiler dahil, tur özetinde görünür. Sınır aşılırsa özet "eksik olabilir" der.
- **Oturum silme:** yerel onayla yapılır; kayıt, kilit dosyası ve geri alma verisi birlikte silinir.
- **Platformlar:** bütün denetimler Windows'ta ve WSL'de yeşildir. Windows testleri makinede Kalfa AppContainer profili ya da `S-1-15-2-` ACE'si bırakmaz.

## Uygulama sonucu

Uygulanınca doldurulur: görev başına commit ve plandan sapmalar, ayrıca G1, G2, G10 ve G12'nin ölçüm sonuçları.
