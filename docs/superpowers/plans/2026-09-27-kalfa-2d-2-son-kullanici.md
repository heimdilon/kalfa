# Kalfa 2d-2 — Son kullanıcı akışı: Uygulama Planı

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Teknik olmayan bir kullanıcı Kalfa'yı tek UAC'lik bir kurucuyla kurar. Yalıtım yoksa kabuk kapalıdır; kullanıcıya sade ileti, BT'ye teknik ayrıntı verilir. Bozulan kurulum "Onar"la düzelir.

**Architecture:**
- Kabuğun modele sunulup sunulmadığına her turda araç karar verir (`Arac::sunulur_mu`).
- Yalıtımsız çalıştırma, uygulama ayarındaki bir anahtara bağlıdır.
- Yalıtım durumu kullanıcı metninden ayrı bir teknik ayrıntı taşır.
- "Onar", kurulu sürümün Program Files'taki `kalfa-yalitim.exe`'sini UAC ile başlatır.
- NSIS kurucusu `kur`/`kaldir`'ı kancalarla çağırır.

**Tech Stack:** Rust 1.98.1 (Tauri 2, windows-sys), Svelte 5 + TypeScript, Tauri'nin NSIS kurucusu, WebdriverIO e2e.

**Spec:** [Aşama 2 tasarımı](../specs/2026-09-24-kalfa-2-kabuk-yalitim-design.md) §9.8, §5.1, §10.4, §10.5, §11, §15; [karşılaştırma ve kararlar](../bulgular/2026-09-27-kalfa-2d-ayri-kullanici-karsilastirma.md).

**Yürütme (kullanıcı kararı, 2026-09-27):** Codex'in kotası belirsiz. Kodu Claude
PR dalına görev görev yazar ve iki platformda doğrular. Bu yüzden plan hafiftir:
görevler, arayüzler ve testler. Tam kod PR'ın commit'lerindedir.

## Global Constraints

- **Araçlar:** Rust 1.98.1; `cargo` komutları `--locked` ile. Node `^22.22.2 || >=24.15.0`; kurulum `npm ci`.
- **Bağımlılık:** yeni crate eklenmez. Tek istisna Görev 8'de `ui/`'ye geliştirme bağımlılığı olarak `@tauri-apps/cli` (kullanıcı izniyle, 2026-09-27). Tauri'nin NSIS araçlarını indirmesi de bu izne dahildir.
- **Güvenlik sınırları:**
  - Araç onaylanan tarife göre çalışır. Yalıtımsız onaylanan komut, çalışma anında anahtar kapalıysa da başlamaz.
  - Yalıtım yokken ve anahtar kapalıyken kabuk modele sunulmaz. Model yine de çağırırsa reddedilir.
  - `kalfa-yalitim.exe` yalnızca, Program Files'ın altında Kalfa'nın yanında duruyorsa UAC ile başlatılır.
  - Test edilmeyen güvenlik sınırı yoktur.
- **`unsafe`:** yalnızca `crates/kalfa-sandbox/src/windows/`'da, her blok `// SAFETY:` açıklamasıyla.
- **Dil:**
  - Kod, test ve açıklamalar Türkçedir. Kullanıcıya görünen metinler `ui/src/dil/tr.ts`'te ya da Rust'ta sabittir ve "siz" diliyle yazılır. Arayüzde "yalıtım" denir.
  - `unwrap()`/`expect()` yalnızca testlerde ve `test_destek.rs`'de kullanılır.
  - Kullanıcı metni sadedir. Teknik terimler (AppContainer, SID, Win32 kodu) yalnızca teknik ayrıntıdadır.
- **Arayüz:** CSP sıkıdır. Arayüz yol göndermez; çalışma alanları kimlikle gelir. Pencerelere Tauri'nin çekirdek izinleri verilmez.
- **Her görevin sonunda:**
  - `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --all -- --check`.
  - Uygulamaya dokunulduysa `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked` ve aynı özellikle clippy.
  - Arayüze dokunulduysa `ui/`'da `npm run check` (0/0), `npm test`, `npm run build`.
  - Görev bitmeden önce Windows'ta ve WSL'de (Ubuntu-24.04) yeşil olmalıdır.
- **Commit:** Türkçe ve önekli (`sandbox:`, `çekirdek:`, `araçlar:`, `uygulama:`, `ui:`, `e2e:`, `docs:`); Claude'un commit'leri `Co-Authored-By` taşır.
- **Dokunulmayanlar:** `design/`, `.env`, `Screenshot*.png`, kökteki `tasks.png`.

## Dosya Yapısı

```
crates/kalfa-core/src/arac.rs                     sunulur_mu, tanimlar filtresi
crates/kalfa-core/src/oturum/arac_asamasi.rs      sunulmayan aracın reddi
crates/kalfa-tools/src/kabuk/mod.rs               YalitimsizIzni; sunulur_mu; çalışmada anahtar denetimi
crates/kalfa-sandbox/src/lib.rs                   YalitimDurumu::Yok.ayrinti; hazırlanmış alanlar
crates/kalfa-sandbox/src/windows/yalitici.rs      ayrıntı üretimi; hazırlanmış alanlar
crates/kalfa-sandbox/src/windows/sistem.rs        Windows sürümü; UAC ile başlatma          yeni
crates/kalfa-sandbox/src/linux/mod.rs             ayrıntı üretimi
crates/kalfa-uygulama/src/yalitim.rs              anahtar, rapor, onarım, hazırlanmış alanlar
crates/kalfa-uygulama/src/arayuz.rs               Yukseltici (UAC) arayüzü
crates/kalfa-uygulama/src/oturumlar.rs            varsayılan proje klasörü
crates/kalfa/src/{komutlar,iletisim,lib,komut_adlari}.rs   komutlar ve gerçek arayüzler
crates/kalfa/tauri.conf.json, crates/kalfa/kurucu/kancalar.nsh   kurucu              yeni
ui/src/ana/Ayarlar.svelte, ui/src/dil/tr.ts, ui/src/ipc/*    arayüz
e2e/testler/05-kabuk.test.mjs, e2e/yardimci/kalfa.mjs        Windows kabuk e2e
```

---

### Task 1: Kabuk yalıtım yokken kapalı

**Files:** `crates/kalfa-core/src/arac.rs`, `crates/kalfa-core/src/oturum/arac_asamasi.rs`, `crates/kalfa-tools/src/kabuk/mod.rs`, `crates/kalfa-tools/src/lib.rs`; testler `crates/kalfa-core/tests/`, `crates/kalfa-tools/tests/kabuk.rs`.

**Interfaces (Produces):**
- `Arac::sunulur_mu(&self) -> bool`, varsayılanı `true`.
- `AracKayitDefteri::tanimlar(kip)` sunulmayan aracı atlar. `planla` onu "Bu araç şu an kullanılamıyor: …" diye reddeder.
- `kalfa_tools::YalitimsizIzni` trait: `fn izinli_mi(&self) -> bool`.
- `KabukAraci::yeni(alan, yalitici, guven, izin: Arc<dyn YalitimsizIzni>)`.
- `KabukAraci::sunulur_mu()` = yalıtım hazır **ya da** `izin.izinli_mi()`.
- `hazirla` sunulmuyorsa hata döner. `calistir_akisli`, tarif `Yalitimsiz` ve anahtar kapalıysa başlatmadan hata döner.

- [ ] **Step 1: Başarısız testleri yaz**
  - `tanimlar_sunulmayan_araci_atlar` (core): sahte araç `sunulur_mu=false`.
  - `sunulmayan_arac_cagrisi_reddedilir` (core oturum testi): model çağırır, sonuç reddedildi, araç çalışmaz.
  - `kabuk_yalitim_yokken_ve_anahtar_kapaliyken_sunulmaz` (tools).
  - `kabuk_anahtar_aciksa_yalitimsiz_sunulur` (tools).
  - `yalitimsiz_onaylanan_komut_anahtar_kapaninca_baslamaz` (tools): hazırla anahtar açıkken yapılır; sonra anahtar kapatılır; `calistir_akisli` hata döner ve süreç başlamaz (işaret dosyası oluşmaz).
- [ ] **Step 2:** Testler derlenmeyerek ya da düşerek başarısız olur.
- [ ] **Step 3:** Uygula. `KabukAraci::yeni`'nin bütün çağıranlarını güncelle; testlerde anahtar açık sahte.
- [ ] **Step 4:** Global Constraints'teki denetimler, iki platformda.
- [ ] **Step 5:** Commit `araçlar: yalıtım yokken kabuk modele sunulmaz; yalıtımsız çalıştırma anahtara bağlı`.

### Task 2: Yalıtımsız anahtarı (uygulama ve arayüz)

**Files:** `crates/kalfa-uygulama/src/{yalitim,oturumlar,uygulama,sozlesme}.rs`, `crates/kalfa-uygulama/src/e2e/*`, `crates/kalfa/src/{komutlar,komut_adlari,lib}.rs`, `ui/src/{ipc,dil,ana/Ayarlar.svelte}`, `e2e/yardimci/kalfa.mjs`.

**Interfaces:**
- Consumes: Görev 1'in `YalitimsizIzni`'si.
- Produces:
  - `yalitim.json`'da `yalitimsiz_izinli: bool` (varsayılan `false`). `AracZinciriKaydi` `YalitimAyarlari` olur ve `YalitimsizIzni`'yi uygular.
  - Komut `yalitimsizi_ayarla(acik: bool) -> bool`. Açarken yerel pencere riski anlatır, vazgeçilirse `false` döner. Kapatmak onaysızdır.
  - `UygulamaDurumu.yalitimsiz_izinli: bool`.
  - e2e ayar dosyasında `yalitimsiz_izinli`. e2e yardımcısı Windows'ta ve Linux'ta açık verir; kabuk testleri ondan önce koşan davranışı korur.
  - Arayüz: Ayarlar'da "Gelişmiş" bölümünde anahtar.

- [ ] **Step 1: Başarısız testleri yaz**
  - `yalitimsiz_anahtari_varsayilan_kapali_ve_kalici` (uygulama).
  - `yalitimsiz_acmak_yerel_onay_ister_vazgecince_kapali_kalir` (uygulama, sahte iletişim).
  - Vitest: Ayarlar anahtarı `yalitimsizi_ayarla`'yı çağırır, durum güncellenir.
- [ ] **Step 2:** Başarısız olduklarını gör.
- [ ] **Step 3:** Uygula; komut yalnızca ana pencereye açılır (`komut_adlari.rs`).
- [ ] **Step 4:** Denetimler; `e2e` özellikli testler ve uçtan uca testler iki platformda.
- [ ] **Step 5:** Commit `uygulama: yalıtımsız çalıştırma anahtarı (varsayılan kapalı)`.

### Task 3: Sade ileti ve BT için teknik ayrıntı

**Files:** `crates/kalfa-sandbox/src/{lib,linux/mod}.rs`, `crates/kalfa-sandbox/src/windows/{yalitici,sistem,mod}.rs`, `crates/kalfa-uygulama/src/yalitim.rs`, `crates/kalfa/src/{komutlar,komut_adlari}.rs`, `ui/src/{ipc,dil,ana/Ayarlar.svelte,ana/DurumCubugu.svelte}`.

**Interfaces:**
- `YalitimDurumu::Yok { neden, talimat, ayrinti: String }`. `talimat` geliştirici talimatı olarak kalır; `ayrinti` düşen denetimi ve işletim sistemi hata kodunu taşır (ör. `denetim=kok-izni; yol=C:\; hata=5 (ERROR_ACCESS_DENIED)`).
- `windows::sistem::surum() -> String`: ürün adı, `DisplayVersion`, yapı.`UBR`, `EditionID` (kayıt defterinden).
- Komut `yalitim_teknik_raporu() -> String`, düz metin:
  - Kalfa sürümü; platform ve işletim sistemi sürümü.
  - Durum ve nedeni; ayrıntı.
  - Gereksinim paragrafı (§9.8, Windows ve Linux için ayrı).
  - Tasarım belgesine bağlantı.
- Arayüz:
  - Yalıtım yoksa Ayarlar ve durum çubuğu, nedene göre sade metin gösterir (`tr.ts`: `yalitim.sade.<neden>`). "BT departmanınızla görüşün" cümlesi yalnızca kullanıcının kendi onaramayacağı nedenlerde çıkar.
  - "Teknik ayrıntıları kopyala" `navigator.clipboard.writeText` kullanır; başarı ya da hata kısa bir bildirimle görünür.
  - "Yeniden dene" düğmesi kalır.

- [ ] **Step 1: Başarısız testleri yaz**
  - `windows_algilayici_ayrintiya_denetimi_ve_kodu_yazar` (sandbox, Windows): kurulum kaydı olmayan geçici `ProgramData` benzeri durumda `denetim=kurulum-kaydi`. Algılayıcıya kayıt dizini enjekte edilemiyorsa ayrıntı üreten saf işlev ayrıca test edilir.
  - `linux_bwrap_yokken_ayrinti` (sandbox, Linux, sahte algılayıcı).
  - `teknik_rapor_surum_durum_ve_gereksinim_icerir` (uygulama).
  - Vitest: nedenlere göre sade metin; kopyala düğmesi raporu panoya yazar (pano sahtesi).
- [ ] **Step 2:** Başarısız olduklarını gör.
- [ ] **Step 3:** Uygula; `YalitimDurumu`'nun bütün kurucularını güncelle.
- [ ] **Step 4:** Denetimler iki platformda.
- [ ] **Step 5:** Commit `sandbox, uygulama, ui: yalıtım yokken sade ileti ve BT için teknik rapor`.

### Task 4: Onar (UAC ile `kur`)

**Files:** `crates/kalfa-sandbox/src/windows/sistem.rs`, `crates/kalfa-uygulama/src/{arayuz,yalitim}.rs`, `crates/kalfa/src/{komutlar,komut_adlari,lib}.rs`, `ui/src/*`.

**Interfaces:**
- `windows::sistem::yonetici_olarak_calistir(exe: &Path, argumanlar: &[&str]) -> io::Result<Option<u32>>`. `ShellExecuteExW` "runas", `SEE_MASK_NOCLOSEPROCESS`; bekler ve çıkış kodunu döner. Kullanıcı UAC'de vazgeçerse `Ok(None)` döner (`ERROR_CANCELLED`).
- `windows::sistem::kurulu_yardimci() -> Option<PathBuf>`: `current_exe()`'nin dizini `FOLDERID_ProgramFiles`'ın (ya da `ProgramFilesX86`'nın) altındaysa ve yanında `kalfa-yalitim.exe` varsa onun yolu. Yollar kanonik ve `sade` karşılaştırılır.
- `kalfa_uygulama::arayuz::Yukseltici` trait (`async fn kur(&self) -> OnarimSonucu`); gerçeği `kalfa`'da, sahtesi testlerde.
- `OnarimSonucu { Tamam, Vazgecildi, Basarisiz(String), KuruluDegil }`.
- Komut `yalitimi_onar() -> OnarimSonucu`: yerel pencere, sonra `kur`, sonra `yeniden_algila`. Linux'ta `KuruluDegil` ve talimat.
- Arayüz: durum `Hazir` değilse ve platform Windows'sa "Onar". `KuruluDegil`'de geliştirme talimatı gösterilir.

- [ ] **Step 1: Başarısız testleri yaz**
  - `kurulu_yardimci_program_files_disinda_yok` (sandbox, Windows): `target\debug`'dan çalışan test ikilisi için `None`.
  - `onar_yerel_onaysiz_kur_calistirmaz`, `onar_vazgecilirse_durum_degismez`, `onar_basarida_yeniden_algilar` (uygulama, sahte `Yukseltici`).
  - Vitest: Onar düğmesinin görünürlüğü ve sonuç iletileri.
- [ ] **Step 2:** Başarısız olduklarını gör.
- [ ] **Step 3:** Uygula.
- [ ] **Step 4:** Denetimler iki platformda. Gerçek UAC yolu Görev 8'de kurulu sürümle elle denenir.
- [ ] **Step 5:** Commit `uygulama: Onar — kurulu yardımcıyı UAC ile çalıştırır`.

### Task 5: Varsayılan proje klasörü

**Files:** `crates/kalfa-uygulama/src/{arayuz,oturumlar}.rs`, `crates/kalfa-uygulama/src/e2e/mod.rs`, `crates/kalfa/src/iletisim.rs`.

**Interfaces:**
- `Iletisim::klasor_sec(&self, baslik: &str, baslangic: Option<&Path>) -> Option<PathBuf>`.
- Çalışma alanı seçilirken `<ev>/Kalfa Projeleri` yoksa oluşturulur ve başlangıç olarak verilir. Oluşturulamazsa başlangıç verilmez.
- Araç zinciri seçicisi başlangıç vermez.

- [ ] **Step 1:** Başarısız test: `calisma_alani_secici_kalfa_projeleri_ile_acilir` (uygulama, sahte iletişim başlangıcı kaydeder, dizin oluşmuştur).
- [ ] **Step 2–4:** Uygula; denetimler iki platformda.
- [ ] **Step 5:** Commit `uygulama: çalışma alanı seçici Kalfa Projeleri'nde açılır`.

### Task 6: Hazırlanmış çalışma alanları (Ayarlar)

**Files:** `crates/kalfa-sandbox/src/{lib,windows/yalitici,windows/ace_kaydi}.rs`, `crates/kalfa-uygulama/src/yalitim.rs`, `crates/kalfa/src/*`, `ui/src/*`.

**Interfaces:**
- `Yalitici::hazirlanmis_alanlar(&self) -> Vec<HazirAlan>`. `HazirAlan { kimlik: String, kok: PathBuf }`; kök, ACE kaydında alanın kökündeki devralınan izinden gelir. Varsayılanı boştur.
- `Yalitici::alani_kaldir_kimlikle(&self, kimlik: &str) -> Result<(), String>`. Windows'ta var olan `alani_kaldir`'ı çağırır; varsayılanı hatadır.
- Komutlar:
  - `yalitim_calisma_alanlari() -> Vec<{kimlik, yol}>`.
  - `yalitim_calisma_alanini_kaldir(kimlik)`: yerel onayla çalışır; açık oturumu olan alan reddedilir.
- Arayüz: Ayarlar'da liste ve "İzinleri geri al ve kaldır".

- [ ] **Step 1: Başarısız testleri yaz**
  - `hazirlanmis_alanlar_ace_kaydindan_gelir` (sandbox, Windows, `yalitim-testleri`): hazırla, listede var; kaldır, listede yok; profil silinmiş.
  - `alan_kaldirma_yerel_onay_ister_ve_acik_oturumu_reddeder` (uygulama, sahte yalıtıcı).
  - Vitest: liste ve kaldır düğmesi.
- [ ] **Step 2–4:** Uygula; denetimler iki platformda, ayrıca `kalfa-sandbox` Windows yalıtım testleri.
- [ ] **Step 5:** Commit `uygulama, ui: hazırlanmış çalışma alanları listelenir ve kaldırılır`.

### Task 7: Windows'ta kabuk uçtan uca testleri

**Files:** `e2e/testler/05-kabuk.test.mjs`, `e2e/yardimci/kalfa.mjs`, gerekirse `crates/kalfa-uygulama/src/e2e/*`.

**Interfaces:** Consumes Görev 2 (anahtar) ve Görev 6 (kaldırma komutu).
- Kurulumlu Windows'ta kabuk testleri gerçek yalıtımda koşar. Çalışma alanının ilk hazırlığını sahte iletişim kutusu onaylar.
- Her test dosyası sonunda kendi çalışma alanlarını `yalitim_calisma_alanini_kaldir` ile kaldırır. Koşudan önce ve sonra `Kalfa.Alan.*` profili sayısı aynıdır.
- Kurulumsuz makinede (ve Linux'ta bwrap yokken) kabuk testleri yalıtımsız anahtarıyla koşar.
- Dosya başındaki "Windows'ta yalıtım aşama 2d'ye kadar yoktur" açıklaması güncellenir.

- [ ] **Step 1:** Profil sayımı yapan yardımcıyı ve kaldırma adımını ekle. Önce kaldırmasız koşup profilin kaldığını gör.
- [ ] **Step 2:** Kaldırmayı bağla; iki koşuda profil sayısı değişmez.
- [ ] **Step 3:** Uçtan uca testler Windows'ta iki kez, WSL'de iki kez.
- [ ] **Step 4:** Commit `e2e: Windows kabuk testleri gerçek yalıtımda; çalışma alanları temizlenir`.

### Task 8: NSIS kurucusu

**Files:** `ui/package.json`, `ui/package-lock.json` (`@tauri-apps/cli`), `crates/kalfa/tauri.conf.json`, `crates/kalfa/kurucu/kancalar.nsh` (yeni), `crates/kalfa/kurucu/README.md` (yeni).

**Interfaces:**
- `bundle.targets`: Windows'ta `nsis`. `bundle.windows.nsis.installMode: "perMachine"`; `installerHooks: "kurucu/kancalar.nsh"`.
- `kalfa-yalitim.exe` kurulum dizinine, `kalfa.exe`'nin yanına girer (`bundle.externalBin`, hedef üçlüsü adıyla). Derleme adımı onu önce `--release` derler ve `binaries/`'e kopyalar.
- Kancalar:
  - `NSIS_HOOK_POSTINSTALL`: `"$INSTDIR\kalfa-yalitim.exe" kur`, sessiz. Başarısızsa kurulum sürer.
  - `NSIS_HOOK_PREUNINSTALL`: `"$INSTDIR\kalfa-yalitim.exe" kaldir`. Başarısızsa kaldırıcı kalanları bir iletiyle söyler.

- [x] **Step 1:** CLI'yi ekle, kurucuyu derle (`crates/kalfa/kurucu/derle.mjs`; aşağıdaki uygulama notuna bakın).
- [x] **Step 2 (kullanıcı, UAC; 2026-09-27'de yapıldı, aşağıdaki kabul notu):**
  1. Kurucuyu çalıştırın.
  2. `kurulum.json`'da kök izinlerinin kurulduğunu doğrulayın.
  3. Kalfa'yı açın; durum `Hazir` olmalı.
  4. Ayarlar'da "Onar"ı deneyin.
  5. Kaldırın; kök izinleri kalkmalı.

  Makinede 2d-1'den kalan `kur` kurulumu olduğu için önce `kalfa-yalitim kaldir` ile sıfırlanır. Geri alma kaldırıcıdır; kaldırıcı düşerse `kalfa-yalitim.exe kaldir` elle çalıştırılır.
- [x] **Step 3:** Commit `uygulama: NSIS kurucusu — makine geneli, kurulumda kur, kaldırmada kaldir`.

### Task 9: Belgeler

**Files:** `docs/GELISTIRME_DURUMU.md`, aşama 2 tasarımı (§9.1 sadeleşir: kopyalama yok, Onar yalnızca kurulu sürümde; §10.4, §10.5 komutları; §12.5), bu plan.

- [x] Uygulanırken bulunanları ve doğrulama tablosunu (Windows, Linux) yaz. Commit `docs: 2d-2 tamamlandı`.

---

## Uygulama notu (2026-09-27)

- Tauri'nin `--config` seçeneği ana yapılandırmayı seçmez; ek ayar
  birleştirir. Bu nedenle Görev 8'in derleme betiği CLI'yi
  `crates/kalfa/` dizininden çalıştırır. Yan ikili
  `tauri.nsis.conf.json` dosyasında yalnızca kurucu derlemesine eklenir;
  otomatik `tauri.windows.conf.json` normal `cargo test` derlemesinde henüz
  kopyalanmamış yardımcının aranmasına yol açtı.
- Kaldırma kancasında `kaldir` hata verirse kurulu yardımcı, kurulum
  dizini silinmeden Program Files'taki ayrı kurtarma dizinine kopyalanır.
  Kullanıcının çalıştıracağı `geri-al.ps1` bu dosyayı da bulur.
  - Görev 8'in bağımlılık indirmesi önce otomatik izin denetiminde reddedildi.
    Kullanıcının bu oturumdaki açık izniyle `@tauri-apps/cli@2.11.4` ve NSIS
    araçları indirildi; Windows x64 kurucusu derlendi. Gerçek UAC kabulü
    kullanıcı tarafından yapılacak.
  - Windows e2e derlemesi Codex ortamında Tauri'nin ev dizini API'sinden
    `unknown path` aldı. Yalnızca Windows e2e derlemesi geçici ev kullanıyor;
    uygulama açıldıktan sonra WebView2 işleyicisi bağlantıyı kapattı. Sürücü
    sürümü WebView2 ile eşleşiyor; kurulu Windows kabulü hâlâ gerekli.
- §9.8'deki Python, Node ve Git'i kurucuya koyma kararı Görev 8'de yoktur;
  bu bir çelişki değildir. Self-review onu 2d-3'e koyar.

## İnceleme notu (2026-09-27, Claude)

- Kök `sade` biçimde saklandığı için klasörü silinen ya da taşınan alanın
  kimliği yeniden hesaplanamıyordu; alan Ayarlar'da görünmüyor ve
  kaldırılamıyordu. Kimlik artık `\\?\` öneki geri eklenerek bulunur
  (`kayitli_kok_kimligi`); gerçek yalıtım testi
  `silinen_calisma_alani_listelenir_ve_kaldirilir`.
- Kaldırma kancasının iletileri `/SD IDOK` ile sessiz kaldırmada takılmaz.
  `derle.mjs` kabuk kullanmaz.
- **Bilinen sınır (kullanıcı kararı):** Tauri kurucusu sürüm yükseltirken
  varsayılan olarak eski sürümü kaldırır. Kaldırma kancası `kaldir`'ı
  çalıştırır ve Ayarlar'dan açılan araç zinciri dizinlerinin izinleri gider;
  yeni kurulumun `kur`'u yalnızca kök izinlerini geri getirir. Kullanıcı
  dizinleri Ayarlar'dan yeniden ekler. Araç zinciri 2d-3'te Program Files'taki
  Kalfa dizinine geldiğinde bu dizinlere gerek kalmaz; 2d-3 planı bu sınırı
  yeniden değerlendirir.

## Kabul notu (2026-09-27, kullanıcı ve Claude)

Kullanıcı `kabul.ps1` ile kurdu, Kalfa'yı Başlat menüsünden açtı, Ayarlar'da
durum `Hazir` göründü. `kalfa-yalitim kaldir` ile bozulan kurulumu "Onar" yerel
onay ve UAC ile düzeltti. Windows kaldırıcısından sonra Program Files dizini,
`kurulum.json`, kısayol ve kaldırma kaydı gitti; `C:\`, `D:\`, `C:\Users` ve
iki araç zinciri dizininde Kalfa'nın yetenek SID'i kalmadı; kurtarma dizini
oluşmadı.

- `Start-Process -Wait`, PowerShell 5.1'de kurucunun son sayfasında açılan
  Kalfa'yı da bekliyordu; betik Kalfa kapanana kadar sürdü. Artık yalnızca
  kurucu süreci beklenir.
- Kurucunun "uygulama verisini sil" seçeneği yalnızca
  `dev.kalfa.masaustu` dizinlerini (WebView önbelleği) siler; Kalfa'nın
  `%APPDATA%\Kalfa` ve `%LOCALAPPDATA%\Kalfa` verisine dokunmaz. ACE kaydının
  kalması doğrudur; oturum kayıtlarının silinip silinmeyeceği 2d-3'te
  kararlaştırılır.

---

## Self-review

- **§9.8 kapsamı:**
  - Kurucu ve kancalar → G8.
  - Araç zinciri kurucuda → 2d-3.
  - Proje klasörü → G5.
  - Onar → G4.
  - Kabuk kapalı, sade ileti ve teknik ayrıntı → G1, G3.
  - Yalıtımsız anahtarı → G2.
- **§10.4:** hazırlanmış çalışma alanları → G6. Önbellek silme ve ek ortam değişkenleri 2d-2'de yoktur; 2d-3'le birlikte değerlendirilir.
- **§15 (2d-2):** Windows kabuk uçtan uca testleri ve profil temizliği → G7.
