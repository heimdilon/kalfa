# Kalfa 3e — Küçük işler: Uygulama Planı

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Aşama 3'ü kapatan küçük işler. Bu planın sonunda:
- **"Bu oturumda izin ver" kısayolları:** uygun onayda kullanıcı aynı türden
  çağrılara oturum boyunca onaysız izin verebilir. Teklifi çekirdek yapar.
- **Kullanıcının ek gizli kalıpları** ve **gizli sayılmayanlar** (K7):
  - Varsayılan kalıpların yanlış pozitifi (`src/credentials.rs`,
    `.env.example`) istisnayla çözülür.
  - Varsayılan kalıplar kapatılamaz.
- **Ayarlar:** `ayarlar.json` ve Ayarlar sayfası. Gizli kalıplar, istisnalar,
  denetçi kipi (3c) ve ağ izin listesi (3d) burada toplanır.
- **Onay kartının son hâli:** "Kapsam" satırı; kartın öteki alanları 3a, 3b ve
  3d'den gelir.

**Architecture:**
- **Kısayol:**
  - `OnayOgesi.kisayol: Option<KisayolTeklifi>` yalnızca uygun öğede doludur.
    Webview teklifi yalnızca kabul edebilir (`Komut::AracOnayi { .., kisayol }`).
  - Planlamada `Oturum::karar`'dan sonra eşleşen kısayol kararı `Serbest`
    yapar. `tek_calistir`'ın karar karşılaştırması kısayolu hesaba katar.
  - Kısayollar oturum belleğindedir. Kip değişince silinir, kayda yazılmaz.
- **Gizli ayarları** (`kalfa-core/src/izin.rs`):
  - `GizliAyarlari`: `RwLock` içinde iki `GlobSet` (ek kalıplar, istisnalar),
    uygulama genelinde paylaşılır.
  - `Siniflandirici` onu her sınıflandırmada okur; açık oturumlar ve yalıtım
    değişikliği hemen uygular.
- **Ayar dosyası** (`kalfa-uygulama/src/ayarlar.rs`): `yalitim.json` gibi
  yazılır (geçici dosya, sonra taşıma).
  - Anahtarlar: `ek_gizli_kaliplar`, `gizli_sayilmayanlar`, `denetci` ve
    `ag_izin_listesi`.
  - 3c `denetci`'yi getirir. 3c'den önce yapılırsa dosyayı bu plan oluşturur.

**Tech Stack:** Rust 1.98.1 (globset), Svelte 5 + TypeScript, Vitest,
`tauri-driver` + WebdriverIO.

**Kaynaklar:**
- [Aşama 3 tasarımı](../specs/2026-09-28-kalfa-3-gorunurluk-design.md) §2.1
  (K7), §8.3 (kullanıcının kalıcı listesi), §9, §10, §11, §13 (G9), §14.
- [Üst belge](../specs/2026-09-21-kalfa-design.md) §7.2, §7.4, §7.6, §11.4
  (r3.9).
- [Aşama 2 tasarımı](../specs/2026-09-24-kalfa-2-kabuk-yalitim-design.md) §5.3
  (kabuk kısayolu), §7.2 (gizli tarama).

**Doğrulama durumu:**
- **Hafif plan:** görevleri, arayüzleri ve önce yazılacak testleri verir.
  Kodu önceden yazılıp doğrulanmadı.
- **G9:** paylaşılan kaydın her sınıflandırmada okunmasının maliyeti.
  Görev 2'de ölçülür.
- **Uygulayan:** kullanıcı karar verecek (tasarım §14).

**Sıra:** aşama 3'ün son planıdır (tasarım §14); onay kartının son hâlini
verir.
- 3a'nın geri alınabilirlik satırı ve 3b'nin veri çıkışı alanı hazır olmalı.
- 3d'nin kullanıcı listesi parametresi ve 3c'nin denetçi kipi hazır olmalı.

**Kapsam dışı:** içerikte anahtar biçimi taraması (G14); varsayılan kalıpları
kapatmak (r3.9'da kapalı).

## Kararlar

Kullanıcının kararı (tasarım §2.1, K7):

- **Varsayılan gizli kalıplar kapatılamaz.**
- **Yanlış pozitif "gizli sayılmayanlar" listesiyle çözülür.** Bu listeye
  eklemek yerel pencerede onaylanır, kaldırmak onaysızdır.
- **İstisna yalnızca adın eşleşmesini kaldırır.** Gizli bir dosyanın hard
  link'i yalıtımda gizli kalır.

Tasarımın kararları (§9):

- **Kısayol kuralları:**
  - Uyarı taşıyan öğe kısayol alamaz.
  - Kabuk kısayolu yalnızca program biçiminde, yalıtımlı ve ağsız komuta
    verilir.
  - "Tümünü onayla" kısayol vermez.
  - Ağlı kabuk için kısayol yoktur (üst belge §7.6).
- **Ek gizli kalıplar:** eklemek onaysızdır; kaldırmak yerel pencerede
  onaylanır.

Kullanıcıya kalan: uygulayan (Claude ya da Codex).

## Güvenlik serisinden sonra (2026-10-05)

Plan yazıldıktan sonra güvenlik serisi (#33–#45) birleşti. Bu plana etkisi:

- **Gizli dizinler (#37).** `gizli_mi` artık yolun her üst dizinini de
  kalıplarla karşılaştırır: `secrets/` gizliyse altındaki her dosya gizlidir.
  İstisna bu kurala şöyle uyar (Görev 2):
  - Her aday (yolun kendisi ve her üst dizini) ayrı değerlendirilir. Aday,
    bir gizli kalıba uyup hiçbir istisnaya uymuyorsa yol gizlidir.
  - Dosyaya verilen istisna gizli üst dizini açmaz. Dizini açmak isteyen,
    dizine istisna verir.
  - Görev 2'ye testler: `gizli_dizindeki_dosyaya_istisna_dizini_acmaz`
    (çekirdekte de), `dizine_istisna_altindakileri_acar_ek_kalip_kapatir`.
- **Çok bağlantılı okuma (#41).** Hard link'i olan dosyanın okunması
  `GIZLI_UYARISI` ile onaya çıkar. İstisna bu kuralı kaldırmaz: istisna
  yalnızca adın eşleşmesini kaldırır (Kararlar). Görev 2'ye test:
  `istisna_cok_baglantili_okumanin_onayini_kaldirmaz`.
- **Kısayol uygunluğu (Görev 1).** Uyarı taşıyan öğe kısayol almadığından
  çok bağlantılı okuma ve görünmez içerikli yazma (`Hazirlik.gorunmez_icerik`,
  #43) kendiliğinden dışarıda kalır. `uygunluk_tablosu` bu ikisini de sınar.
- **G9.** Üst dizin taraması `gizli_mi`'nin maliyetini yol derinliğiyle
  çarpar; G9 ölçümü bunu içerir.

## Global Constraints

- **Araçlar:** Rust 1.98.1; `cargo` komutları `--locked` ile. Node
  `^22.22.2 || >=24.15.0`; kurulum `npm ci`.
- **Bağımlılık:** workspace'e ve `ui/`'ye yeni paket girmez.
- **`unsafe`:** bu planda yeni `unsafe` yoktur.
- **Güvenlik sınırları:**
  - **Kısayol teklifi:**
    - Çekirdek, teklif etmediği bir kısayolu kabul etmez.
    - Webview yalnızca "kabul" bitini gönderir; kapsamı gönderemez.
  - **Kısayolun kapsamı ve ömrü:**
    - Kısayol uyarı taşıyan hiçbir öğeyi kapsamaz.
    - Kip değişince bütün kısayollar silinir. Kısayol kayda yazılmaz ve
      kurtarmada geri gelmez.
    - Kısayolla onaysız yürüyen çağrı Bulgu 3'ün onaysız değişiklik listesine
      girer.
  - **Gizli ayarlarında yerel pencere:**
    - Gizliliği azaltan her değişiklik yerel pencerede onaylanır: ek kalıbı
      kaldırmak ve istisna eklemek.
    - Gizliliği artıran değişiklik onaysızdır.
  - **Kalıp doğrulaması:**
    - Boş kalıp, mutlak yol, `..` içeren ve 256 karakterden uzun kalıp
      reddedilir.
    - Yalnızca joker karakterden oluşan istisna (`*`, `**`, `*.*`)
      reddedilir.
    - Her listede en çok 200 kalıp olur.
  - **Ayarın yeri:** depo içindeki hiçbir dosya ayarları değiştiremez; ayar
    veri dizinindedir ve yalıtımda görünmez.
  - Test edilmeyen güvenlik sınırı yoktur; her görev kendi sınırının testini
    getirir.
- **Dil:**
  - Kod, test ve açıklamalar Türkçedir.
  - Kullanıcıya görünen metinler `ui/src/dil/tr.ts`'te ya da Rust'ta sabittir
    ve "siz" diliyle yazılır. Arayüzde "yalıtım" denir.
  - `unwrap()` ve `expect()` yalnızca testlerde ve `test_destek.rs`'de
    kullanılır.
- **Arayüz:** CSP sıkıdır; sabit `style="…"` kullanılmaz, ham HTML basılmaz.
  Pencerelere Tauri'nin çekirdek izinleri verilmez. Arayüz yol göndermez.
- **Protokol:** yeni alanlar `#[serde(default)]` taşır. `ui/src/ipc/tipler.ts`
  aynı commit'te güncellenir.
- **Her görevin sonunda:**
  - `cargo test --workspace --locked`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`
  - `cargo fmt --all -- --check`
  - `kalfa-uygulama` ya da `kalfa`'ya dokunulduysa:
    `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked` ve aynı
    özellikle clippy.
  - `kalfa-sandbox` ya da `kalfa-tools`'a dokunulduysa, root olmayan
    kullanıcıyla yalıtım clippy'si ve yalıtım testleri.
  - Arayüze dokunulduysa `ui/` içinde: `npm run check` (0 hata, 0 uyarı),
    `npm test`, `npm run build`.
  - Görev, Windows'ta ve WSL'de (Ubuntu-24.04) yeşil olmadan bitmiş sayılmaz.
- **Commit:** Türkçe ve önekli (`çekirdek:`, `sandbox:`, `uygulama:`, `ui:`,
  `e2e:`, `docs:`). Claude'un commit'leri `Co-Authored-By` satırı taşır;
  Codex'inkiler taşımaz.
- **Dokunulmayanlar:** `design/`, `.env`, `Screenshot*.png`, `refs/shot-*.png`,
  kökteki `tasks.png`.

## Dosya Yapısı

```
crates/kalfa-core/src/kisayol.rs              yeni: KisayolTeklifi, Kisayol, uygunluk ve eşleşme
crates/kalfa-core/src/izin.rs                 GizliAyarlari; Siniflandirici'nin ek kalıpları ve istisnaları
crates/kalfa-core/src/oturum/{mod,arac_asamasi}.rs  teklif, kabul, planlamada eşleşme, tek_calistir
crates/kalfa-core/src/protokol.rs             OnayOgesi.kisayol, AracOnayi.kisayol, kısayol komutları ve olayı
crates/kalfa-sandbox/src/                     gizli taramanın istisnayı ve hard link kuralını uygulaması
crates/kalfa-uygulama/src/ayarlar.rs          yeni ya da 3c'nin: ayarlar.json
crates/kalfa-uygulama/src/{uygulama,oturumlar,sozlesme}.rs, crates/kalfa/src/  komutlar
ui/src/ana/{OnaySayfasi,Konusma,Ayarlar}.svelte, ui/src/{ipc/tipler.ts,dil/tr.ts,durum/}
e2e/testler/                                  kısayol ve istisna
docs/superpowers/*, docs/GELISTIRME_DURUMU.md
```

---

### Task 1: Kısayollar

**Files:**
- `crates/kalfa-core/src/kisayol.rs` (yeni),
  `crates/kalfa-core/src/oturum/{mod,arac_asamasi}.rs`,
  `crates/kalfa-core/src/protokol.rs`
- Testler: `crates/kalfa-core/tests/kisayol.rs` (yeni)

**Interfaces (Produces):**
- **`KisayolTeklifi`:**
  - `Dosya { arac, dizin }`: `dosya_yaz` ya da `dosya_duzenle`; dizin hedefin
    üst dizinidir, kökteki dosyada `.`.
  - `Kabuk { program, dizin }`.
  - `cumle()`: "Bu oturumda `src/` altındaki dosyaları `dosya_duzenle` ile
    onaysız düzenleyebilir." ya da "Bu oturumda `cargo`'yu `.` dizininde
    yalıtımlı ve ağsız çalıştırabilir."
- **`teklif(hazirlik, karar, arac) -> Option<KisayolTeklifi>`.** Yalnızca
  karar `Onayli` ve uyarı listesi boşsa teklif yapılır.
  - Kabukta ek koşullar: program biçimi, yalıtımlı, ağsız ve ilk çağrı değil.
  - Dosyada ek koşullar: tek hedef, yazma aracı.
- **Eşleşme:**
  - `Kisayol::kapsar(hazirlik, arac) -> bool`.
  - Dosya kısayolu alt dizinleri de kapsar. Yol bileşenleriyle karşılaştırılır;
    `src` kısayolu `srcx/`'i kapsamaz.
  - Kabuk kısayolunda program adı ve çalışma dizini eşleşmeli; argümanlar
    serbesttir.
- **Protokol:**
  - `OnayOgesi.kisayol: Option<KisayolTeklifi>`.
  - `Komut::AracOnayi { .., kisayol: bool }` (`#[serde(default)]`).
  - `Komut::KisayoluKaldir { kimlik }`.
  - `Olay::Kisayollar { liste }`.
- **Oturum:**
  - Kabulde teklif oturumun listesine girer. Teklif yoksa `kisayol: true`
    reddedilir ve onay geçmez: hata iletisi, kart açık kalır.
  - Planlamada `Oturum::karar`'dan sonra eşleşen kısayol kararı `Serbest`
    yapar. Çağrı onaysız değişiklik listesine girer.
  - `tek_calistir`, kısayolun kapsadığı kararı plandakiyle karşılaştırır.
    Kısayol kaldırıldıysa ya da kip değiştiyse işlem yapılmaz
    (`PolitikaDegisti`).
  - `KipDegisti` bütün kısayolları siler.

- [ ] **Step 1: Başarısız testleri yaz**
  - `uygunluk_tablosu`: tasarım §9.1'deki her uyarı (gizli, korumalı,
    yapılandırma, hard link, ilk kabuk, alan hazırlığı, ağlı, yalıtımsız,
    görünmez karakter, aynı adım, büyük dosya) teklifi engeller.
  - `teklif_edilmemis_kisayol_reddedilir`.
  - `kisayol_sonraki_cagriyi_onaysiz_yapar_ve_onaysiz_listeye_girer`.
  - `dosya_kisayolu_alt_dizini_kapsar_benzer_adi_kapsamaz`.
  - `kabuk_kisayolu_argumanlari_serbest_birakir_dizini_birakmaz`.
  - `kip_degisince_kisayollar_silinir`.
  - `kaldirilan_kisayol_bekleyen_calismayi_durdurur`.
  - `kisayol_kayda_yazilmaz_kurtarmada_yok`.
  - Senaryo (3c'nin düzeneği varsa): ağlı kabuk kısayol almaz.
- [ ] **Step 2–5:** uygula, denetle, commit `çekirdek: bu oturumda izin ver
  kısayolları`.

### Task 2: Gizli ayarları çekirdekte

**Files:** `crates/kalfa-core/src/izin.rs`; testler:
`crates/kalfa-core/tests/izin.rs`.

**Interfaces:**
- **`GizliAyarlari`:**
  - `yeni(ek: &[String], istisna: &[String]) -> Result<Self, KalipHatasi>`.
  - `degistir(&self, ek, istisna) -> Result<(), KalipHatasi>`. Doğrulama
    geçmezse eski hâl kalır.
  - `RwLock` içinde iki `GlobSet`.
- **`kalip_dogrula(k, istisna: bool) -> Result<(), KalipHatasi>`:** Global
  Constraints'teki kurallar.
- **`Siniflandirici::yeni_ayarla(kok, ayarlar: Arc<GizliAyarlari>)`.** Bir yol
  gizlidir, ancak:
  - varsayılan ya da ek bir kalıba uyuyorsa,
  - ve hiçbir istisnaya uymuyorsa.
  - Bağımlılık dizinleri kuralı aynen sürer.
- **G9:** `gizli_mi` ölçülür (100 000 yol). Okuma kilidi ölçülebilir bir
  maliyet getiriyorsa ayarın sürüm numarası önbelleğe alınır.

- [ ] **Step 1: Başarısız testleri yaz:**
  - `ek_kalip_gizli_yapar`, `istisna_varsayilan_kalibi_kaldirir`
    (`src/credentials.rs`, `.env.example`), `istisna_ek_kalibi_kaldirir`.
  - `bagimlilik_dizini_kurali_surer`.
  - `yalniz_joker_istisna_reddedilir`,
    `gecersiz_kaliplar_reddedilir_eski_hal_kalir`, `iki_yuzden_fazla_kalip`.
  - `degisiklik_acik_siniflandiricida_hemen_gecerli`.
- [ ] **Step 2–5:** uygula, denetle, commit `çekirdek: ek gizli kalıplar ve
  gizli sayılmayanlar`.

### Task 3: Yalıtımda istisna

**Files:** `crates/kalfa-sandbox/src/` (gizli tarama, Linux ve Windows
arka uçları); testler: `crates/kalfa-sandbox/tests/yalitim.rs`.

- [ ] Gizli tarama `Siniflandirici::gizli_mi`'yi kullandığı için istisna
  kendiliğinden uygulanır. Bu görev bunu gerçek yalıtımla sınar.
- [ ] **Testler** (tasarım §11.3):
  - `istisna_edilen_dosya_yalitimda_okunur`.
  - `gizli_dosyanin_hard_linki_istisnaya_uysa_da_okunamaz`: Linux'ta
    (aygıt, inode), Windows'ta dosya nesnesinin ACL'i.
  - `gizli_dizindeki_dosyaya_istisna_dizini_acmaz`.
- [ ] Commit `sandbox: gizli sayılmayanlar yalıtımda`.

### Task 4: `ayarlar.json` ve komutlar

**Files:**
- `crates/kalfa-uygulama/src/ayarlar.rs` (3c'nin ya da yeni),
  `crates/kalfa-uygulama/src/{uygulama,oturumlar,sozlesme}.rs`
- `crates/kalfa/src/{lib,komutlar,komut_adlari}.rs`,
  `crates/kalfa/capabilities/ana-pencere.json`
- Testler: `kalfa-uygulama` testleri

**Interfaces:**
- **`Ayarlar`:** `yalitim.json` gibi yüklenir ve yazılır.
  - Bozuk dosyada varsayılanlar kullanılır ve durum Ayarlar'da görünür.
  - Anahtarlar:
    - `ek_gizli_kaliplar`, `gizli_sayilmayanlar`.
    - `denetci` (3c).
    - `ag_izin_listesi` (3d): host, port, `ozel_ag`.
- **`Uygulama::baglam`:** sınıflandırıcıyı paylaşılan `GizliAyarlari`'yle
  kurar. Bugün `Siniflandirici::yeni(kok, &[])` ile kuruyor.
- **Komutlar:**
  - `gizli_ayarlari() -> GizliAyarGorunumu`: varsayılanlar salt okunurdur.
  - `ek_gizli_kalip_ekle(k)`: onaysız.
  - `ek_gizli_kalip_kaldir(k)`: yerel pencere.
  - `gizli_sayilmayan_ekle(k)`: yerel pencere; metin tasarım §9.2'deki
    cümledir.
  - `gizli_sayilmayan_kaldir(k)`: onaysız.
  - `ag_hostu_ekle(host, ozel_ag)`: yerel pencere. Pencere "özel ağ
    adreslerine bağlanabilir" seçeneğini söyler.
  - `ag_hostu_kaldir(host)`: onaysız.
  - Yerel pencere `Iletisim::onayla` ile açılır, `yalitimsizi_ayarla` gibi.
    Vazgeçilirse ayar değişmez.
- **Ağ listesi:** 3d'nin vekili kullanıcı listesini bu ayardan alır. Liste ya
  da işaret değişince profil özeti değişir; bekleyen onay geçersizleşir
  (tasarım §8.3).

- [ ] **Step 1: Başarısız testleri yaz:**
  - `kaldirmak_yerel_onay_ister_vazgecilince_degismez`.
  - `istisna_eklemek_yerel_onay_ister`, `eklemek_ve_kaldirmak_onaysiz`
    (gizliliği artıranlar).
  - `ayar_dosyasi_atomik_yazilir`, `bozuk_ayar_dosyasi_varsayilanla_acilir`.
  - `degisiklik_acik_oturumda_sonraki_cagrida_gecerli`.
  - `ag_hostu_eklemek_yerel_onay_ister_ve_profil_ozetini_degistirir`.
- [ ] **Step 2–5:** uygula, denetle, commit `uygulama: ayarlar.json ve gizli
  ayarları`.

### Task 5: Arayüz

**Files:** `ui/src/ana/{OnaySayfasi,Konusma,Ayarlar}.svelte`,
`ui/src/ipc/tipler.ts`, `ui/src/dil/tr.ts`, `ui/src/durum/`; Vitest.

- [ ] **Onay sayfası:** "Kapsam" satırı. Seçenekler "Yalnızca bu çağrı"
  (varsayılan) ya da onay kutusuyla kısayolun cümlesi. Yalnızca teklif varsa
  görünür. "Tümünü onayla" kısayol göndermez.
- [ ] **Konuşmanın üstünde** "Bu oturumdaki izinler (N)" listesi; her kısayol
  kaldırılabilir.
- [ ] **Ayarlar > "Gizli dosyalar":**
  - Varsayılanlar salt okunur ve katlanır.
  - Ek kalıplar ve "Gizli sayılmayanlar" ayrı listelerdir.
  - Açıklama: "Yalnızca adı eşleşen dosyalar gizli sayılır; içerik taranmaz."
- [ ] **Ayarlar > "Ağ":** kullanıcının host listesi ve "(özel ağ)" işareti.
- [ ] **Ayarlar > "Denetçi":** kip seçimi (3c'nin komutları).
- [ ] **Vitest** (tasarım §11.4):
  - Kapsam kutusu yalnızca teklif varsa görünür.
  - "Tümünü onayla" `kisayol: false` gönderir.
  - İstisna eklemek yerel onay komutunu çağırır.
  - Listeler ve hata iletileri doğru gösterilir.
- [ ] Commit `ui: kısayollar ve Ayarlar sayfası`.

### Task 6: Uçtan uca ve belgeler

- [ ] Uçtan uca:
  - Bir kısayol verilir, sonraki çağrı onaysız geçer; kip değişince
    kısayol kalkar.
  - Bir istisna eklenir, dosya onaysız okunur.
- [ ] `docs/GELISTIRME_DURUMU.md`: 3e'nin ve aşama 3'ün durumu; G9'un sonucu.
- [ ] Aşama 3 tasarımı §9 ve §13; bu planın "Uygulama sonucu" bölümü.
- [ ] Commit `e2e: kısayollar ve gizli sayılmayanlar`, `docs: 3e'nin sonucu`.

## Kabul ölçütleri

- Kısayol yalnızca teklif edilen, uyarısız öğede verilir. Kip değişince
  silinir. Onaysız yürüyen çağrı onaysız değişiklik listesinde görünür.
- İstisna yanlış pozitifi çözer (`src/credentials.rs` okunur); gizli bir
  dosyanın hard link'i yalıtımda gizli kalır.
- Gizliliği azaltan her ayar değişikliği yerel pencere ister; vazgeçilince
  hiçbir şey değişmez.
- Ayarlar sayfası gizli kalıpları, istisnaları, ağ listesini ve denetçi kipini
  gösterir ve değiştirir.
- Bütün denetimler Windows'ta ve WSL'de yeşildir.

## Uygulama sonucu

Dal `3e/kucuk-isler`, PR #23. Görev 1–6'yı Claude uyguladı.

- **Görevler:** 1–6'nın hepsi yapıldı. Tasarımdan ayrılan ve netleşen
  noktalar aşama 3 tasarımının §9.4'ündedir:
  - Kısayolun cümlesini arayüz kurar; plandaki Rust `cumle()` yok.
  - Görev 3'ün testleri `kalfa-sandbox`'ta değil `kalfa-tools`'un uyumluluk
    testlerindedir.
  - Ağ hostu eklemek de yerel pencerede onaylanır.
- **G9 (Görev 2):** 100 000 yolda sınıflandırma Windows'ta 68–98 ms,
  WSL'de 46–59 ms (sürüm derlemesi; üst uç 50 ek kalıp ve 50 istisnayla).
  Okuma kilidi ölçülebilir bir maliyet getirmedi; önbellek gerekmedi.
- **İnceleme düzeltmesi (Görev 4):** `ayarlar.json`'daki tek bir geçersiz
  kalıp iki gizli listeyi de düşürüyordu. Kullanıcının geçerli ek kalıpları
  sessizce kapanıyor, sonraki ayar yazımında dosyadan da siliniyordu. Artık
  yalnızca geçersiz kalıp ayıklanır
  (`bozuk_ayar_dosyasi_varsayilanla_acilir`).
- **Doğrulama:**
  - Windows: çalışma alanı 815 ve `e2e` özellikli 114 test; iki clippy ve
    fmt temiz. Yalıtım 43/43. Arayüz: Vitest 303/303, `npm run check` 0 hata
    0 uyarı. Uçtan uca 35/35.
  - Windows uyumluluk 21/22: `cargo_ozel_registry_kapta_doldurulamaz`
    ana dalda (`f1c0e32`) da aynı nedenle düşüyor. Ağlı kapta Schannel
    sertifika iptalini denetleyemiyor (`CRYPT_E_REVOCATION_OFFLINE`); 3d
    birleşirken geçiyordu, makinedeki iptal önbelleğine bağlı görünüyor.
  - WSL (`kalfatest`): çalışma alanı 757 ve `e2e` özellikli 113 test; üç
    clippy temiz; yalıtım 26/26, uyumluluk 15/15.
  - WSL uçtan uca 35/35. İlk iki tam koşuda gecikme ölçümü eşiği aştı
    (p95 52–61 ms, eşik 50). Tek başına beş koşuda 34–43 ms; üçüncü tam koşu
    geçti.
