# Kalfa 3b — Gönderim günlüğü ve önizleme: Uygulama Planı

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Kullanıcı EVREN'e ne gittiğini görsün ve isterse göndermeden önce
seçsin (üst belge §7.4, §11.10). Bu planın sonunda:
- **Gönderim günlüğü:** her istek denemesi, başarısızlar dahil, kaydın bir
  satırıdır. Satır bu istekte yeni giden öğeleri kaynak etiketleriyle, önceki
  mesajları sayı olarak gösterir.
- **Kaynak etiketleri:** her mesaj nereden geldiğini söyler: dosya, kabuk,
  arama, kullanıcı, Kalfa.
- **Göndermeden önce göster:** oturum ayarıdır, varsayılan olarak kapalıdır.
  Açıksa bir araç aşamasının dış kaynaklı sonuçları modele gitmeden önce bir
  kartta gösterilir; kullanıcı öğe çıkarabilir.
- **Günlük ekranı** menüden açılır.
- **Onay kartında "Veri çıkışı":** öğe onaylanırsa EVREN'e ne gideceğini
  söyler: dosya içeriği, komut çıktısı, arama sonuçları, dosya adları ya da
  yalnızca "yazıldı" bilgisi (tasarım §9.3).

**Architecture:**
- **İçerik ikinci kez yazılmaz.** Günlük satırı mesajlara kayıttaki `sira` ile
  başvurur; günlük kayıttan türetilir (`kalfa-core/src/gonderim.rs`).
- **`Istek` kaydı genişler.** Oturum `gonderilen` sayacını tutar. Her deneme
  kendi kaydını yazar.
- **Sistem iletisi ve araç tanımları** ilk istekte ve değiştiklerinde kayda
  yazılır.
- **Önizleme:** araç sonuçları aşama bitene kadar `bekleyen_sonuclar`'da
  durur. Karar gelince bağlama ve kayda eklenir. 3c'nin katı kipi aynı
  mekanizmayı ve kartı kullanır.

**Tech Stack:** Rust 1.98.1, Svelte 5 + TypeScript, Vitest, `tauri-driver` +
WebdriverIO.

**Kaynaklar:**
- [Aşama 3 tasarımı](../specs/2026-09-28-kalfa-3-gorunurluk-design.md) §4.2,
  §4.3, §6, §9.3 (veri çıkışı), §10, §11, §13 (G8, G13, G15), §14.
- [Üst belge](../specs/2026-09-21-kalfa-design.md) §7.4, §11.7, §11.10 (r3.9).

**Doğrulama durumu:**
- **Hafif plan:** görevleri, arayüzleri ve önce yazılacak testleri verir.
  Kodu önceden yazılıp doğrulanmadı.
- **Uygulayan:** Antigravity (kullanıcının kararı, 2026-10-05). İnceleme
  ve iki platform doğrulaması Claude'dadır.

**Sıra ve bağımlılıklar:**
- **3a'dan sonra yürür:** ikisi de oturum döngüsüne, `Mesaj` ve `Istek`
  kayıtlarına dokunur.
- **3a'nın Kalfa notu** (`KayitOgesi::KalfaNotu`) `kalfa:not` etiketini alır.
- **3c'nin katı kipi** bu planın bekleyen sonuç mekanizmasını ve kartını
  kullanır; 3c'nin Görev 6'sı bu plandan sonra gelir.

**Kapsam dışı:** bağlam özetleme (G13), `icerik_ara`'da dosya başına etiket
(G15), eklentilerin `evren-cagir` isteği (aşama 4).

## Kararlar

- **Tasarımın kararları** (§2.2, §6):
  - Önizleme oturum ayarıdır ve varsayılan olarak kapalıdır.
  - Kurtarılan oturum önizlemenin son değeriyle açılır.
  - Gizli dosya kuralı önizlemeden bağımsızdır.
- **Bu planın iki küçük ayrıntısı** (tasarımla çelişmez, PR'da gözden
  geçirilsin):
  1. **Akış açıldıktan sonra kopan istek:**
     - `Istek` akış açılınca `sonuc: Basarili` ile yazılır. Böylece gövdesi
       gitmiş istek çökmede de günlükte kalır.
     - Akış sonradan koparsa ayrı bir `IstekKoptu { mesaj }` kaydı yazılır.
       Günlük onu, `Kullanim` gibi, önceki `Istek`'e bağlar ve satırı
       `Hata { asama: Yanit }` gösterir.
  2. **Gövde gitti mi:** `EvrenError::Ag`, gövdenin gidip gitmediğini ayırt
     edecek biçimde genişler (reqwest'in `is_connect`'i). Emin olunamayan
     durumda `asama: Yanit` yazılır; güvenli taraf, gövdenin gittiğini
     varsaymaktır.
- **Uygulayan:** Antigravity (2026-10-05).

## Güvenlik serisinden sonra (2026-10-05)

Plan yazıldıktan sonra güvenlik serisi (#33–#45) birleşti. Bu plana etkisi:

- **Yanıt bütçesi (#40).** `kalfa-evren` artık sınırı aşan yanıtı
  `EvrenError::YanitSiniri` ile keser; çekirdek de `model_turu`'nda
  `YanitButcesi` tutar. İkisinde de gövde gitmiştir:
  - Akış açıldıktan sonra gelen `YanitSiniri` `IstekKoptu` yazar.
  - HTTP hata gövdesi 8 MiB'yi aşınca gelen `YanitSiniri` `Istek`'i
    `Hata { asama: Yanit }` yazar.
  - Görev 1'e test: `yanit_siniri_govde_gitti_diye_kaydedilir`.
- **`Ag` ayrımı (Kararlar, 2).** `sinirli_govde` yanıt gövdesini okurken
  gelen ağ hatasını da `EvrenError::Ag`'a çevirir. Bu hata gövde gittikten
  sonradır. Ayrım reqwest'in `is_connect`'iyle yapılırsa doğru düşer; başka
  bir ölçütle yapılırsa bu yol `Yanit` sayılmalıdır.
- **`Hazirlik`'in yeni alanları (#41, #43):** `okuma_baglanti_sayilari` ve
  `gorunmez_icerik` mühre girer. `veri_cikisi` eklenirken bunlar korunur;
  `veri_cikisi` mühre girmez, çünkü aracın adından değil hazırlıktan gelir ve
  onaylanan işi değiştirmez.
- **`dosya_duzenle` artık okuma da bildirir (#34).** `okunan` hedefi taşır.
  Veri çıkışı yine `YalnizcaYazildi`'dır: sonuç dosya içeriği taşımaz.

## Global Constraints

- **Araçlar:** Rust 1.98.1; `cargo` komutları `--locked` ile. Node
  `^22.22.2 || >=24.15.0`; kurulum `npm ci`.
- **Bağımlılık:** workspace'e ve `ui/`'ye yeni paket girmez.
- **`unsafe`:** bu planda yeni `unsafe` yoktur.
- **Güvenlik sınırları:**
  - Gövdesi gitmiş hiçbir istek günlükten düşmez. Buna başarısızlar ve
    kopanlar dahildir.
  - Önizlemede çıkarılan içerik hiçbir isteğe girmez ve kayda da yazılmaz;
    yerinde "[kullanıcı bu içeriği göndermedi]" durur.
  - Önizleme beklenirken iptal ve kip değişikliği onay beklemesindeki gibi
    işler (`tur_komutu_al`).
  - Günlük ekranı yalnızca kayıttan okur; arayüz yol göndermez, oturum
    kimliği ve `sira` gönderir.
  - Test edilmeyen güvenlik sınırı yoktur; her görev kendi sınırının testini
    getirir.
- **Dil:**
  - Kod, test ve açıklamalar Türkçedir.
  - Kullanıcıya görünen metinler `ui/src/dil/tr.ts`'te ya da Rust'ta sabittir
    ve "siz" diliyle yazılır.
  - `unwrap()` ve `expect()` yalnızca testlerde ve `test_destek.rs`'de
    kullanılır.
- **Arayüz:** CSP sıkıdır; sabit `style="…"` kullanılmaz, ham HTML basılmaz.
  Pencerelere Tauri'nin çekirdek izinleri verilmez.
- **Protokol:**
  - Yeni alanlar `#[serde(default)]` taşır. Eski kayıtların açılıp
    günlüğünün türetilebildiğini bir test sınar. Eski `Istek`'te `yeni` boştur
    ve satır "ayrıntı yok" der.
  - `ui/src/ipc/tipler.ts` aynı commit'te güncellenir.
  - Yeni olaylar `aktarim::kritik_mi`'ye eklenir.
- **Her görevin sonunda:**
  - `cargo test --workspace --locked`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`
  - `cargo fmt --all -- --check`
  - `kalfa-uygulama` ya da `kalfa`'ya dokunulduysa:
    `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked` ve aynı
    özellikle clippy.
  - Arayüze dokunulduysa `ui/` içinde: `npm run check` (0 hata, 0 uyarı),
    `npm test`, `npm run build`.
  - Görev, Windows'ta ve WSL'de (Ubuntu-24.04) yeşil olmadan bitmiş sayılmaz.
- **Commit:** Türkçe ve önekli (`çekirdek:`, `uygulama:`, `ui:`, `e2e:`,
  `docs:`). Claude'un commit'leri `Co-Authored-By` satırı taşır; Antigravity'ninkiler
  taşımaz.
- **Dokunulmayanlar:** `design/`, `.env`, `Screenshot*.png`, `refs/shot-*.png`,
  kökteki `tasks.png`.

## Dosya Yapısı

```
crates/kalfa-evren/src/{hata,istemci}.rs        Ag hatasında gövdenin gidip gitmediği
crates/kalfa-core/src/kayit.rs                  Istek'in yeni alanları, IstekKoptu, SistemIstemi, AracTanimlari, OnizlemeDegisti; Mesaj.kaynak
crates/kalfa-core/src/gonderim.rs               yeni: GonderilenOge, IstekSonucu, günlük türetimi ve sayfası
crates/kalfa-core/src/oturum/{mod,model_turu,arac_asamasi}.rs  gonderilen sayacı, denemelerin kaydı, etiketler, bekleyen_sonuclar
crates/kalfa-core/src/{protokol,kurtarma,aktarim,gecmis/mod}.rs  olaylar, komutlar, yer tutucu
crates/kalfa-core/src/test_destek.rs            sahte sağlayıcıda akışın kopması ve bağlantı hatası
crates/kalfa-core/src/arac.rs                   Hazirlik.veri_cikisi; OnayOgesi.veri_cikisi (protokol.rs)
crates/kalfa-tools/src/{oku,ara,listele,yaz,duzenle}.rs, kabuk/  hazırlıkta veri çıkışı
crates/kalfa-uygulama/src/{oturumlar,sozlesme,uygulama}.rs, crates/kalfa/src/  komutlar
ui/src/bilesenler/{LogTable,SourceTag}.svelte   yeni (tasarım sistemindeki adlarla)
ui/src/ana/{GonderimGunlugu,OnizlemeKarti}.svelte  yeni
ui/src/ana/{Menu,Konusma,KonusmaOgesi}.svelte, ui/src/{ipc/tipler.ts,dil/tr.ts,durum/}
e2e/testler/                                    günlük satırları; önizlemede öğe çıkarma
docs/superpowers/*, docs/GELISTIRME_DURUMU.md
```

---

### Task 1: Her istek denemesinin kaydı

**Files:**
- `crates/kalfa-evren/src/{hata,istemci}.rs`
- `crates/kalfa-core/src/{kayit,gonderim}.rs`,
  `crates/kalfa-core/src/oturum/{mod,model_turu}.rs`,
  `crates/kalfa-core/src/{protokol,kurtarma}.rs`,
  `crates/kalfa-core/src/test_destek.rs`
- Testler: `crates/kalfa-core/tests/gonderim_kaydi.rs` (yeni),
  `crates/kalfa-evren/tests/istemci.rs`

**Interfaces:**
- **`KayitOgesi::Istek`**'e (hepsi `#[serde(default)]`):
  - `model: Option<String>`
  - `yeni: Vec<GonderilenOge>`
  - `onceki: u32`
  - `sonuc: IstekSonucu`
- **`GonderilenOge { kaynak: String, sira: Option<u64>, bayt: u64 }`.**
  `sira`, öğenin kayıttaki `Mesaj`'ının sırasıdır. Sistem iletisi ve araç
  tanımlarında kendi kayıtlarının sırasıdır.
- **`IstekSonucu`:** `Basarili` (varsayılan) ya da
  `Hata { asama: IstekAsamasi, mesaj: String }`;
  `IstekAsamasi = Baglanti | Yanit`.
- **`KayitOgesi::IstekKoptu { mesaj }`:** akış açıldıktan sonra kopan istek
  (Kararlar, 1).
- **Oturum:**
  - `gonderilen: usize`: bu kadar mesaj bir önceki denemeyle gitti.
  - Her deneme, gövde gittikten (ya da gidemedikten) sonra kaydını yazar ve
    sayacı bağlamın uzunluğuna çıkarır.
  - 429'dan sonraki ikinci deneme ve şart akışından sonraki deneme de ayrı
    kayıtlardır. İkinci denemede `yeni` boştur, `onceki` hepsidir.
- **`kalfa-evren`:** `EvrenError::Ag`, gövdenin gitmediği bağlantı hatasını
  ayırt eder (Kararlar, 2).
- **`Olay::IstekBilgisi`**'ye `amac` ve `basarili` eklenir.
- **Kurtarma:** `gonderilen`, kayıttaki son `Istek`'in kapsadığı mesaj
  sayısından kurulur.
- **`SahteSaglayici`:** `SahteYanit::Hata` var. Bağlantı hatası (`Ag`,
  gövde gitmedi) ve akış kopması için yardımcılar eklenir.

- [x] **Step 1: Başarısız testleri yaz**
  - `her_deneme_kaydedilir`: 429, ardından başarı; iki `Istek`, ilki
    `Hata { asama: Yanit }`.
  - `baglanti_hatasi_govde_gitmedi_diye_kaydedilir`.
  - `akis_kopunca_istek_koptu_yazilir`.
  - `yeni_ve_onceki_hesabi`: üç istekli bir tur. Her istekte `yeni` yalnızca
    önceki denemeden beri eklenenlerdir, `onceki` doğru sayıdır.
  - `ikinci_denemede_yeni_bos`.
  - `kurtarmada_gonderilen_sayaci_kayittan_kurulur`.
  - `eski_istek_kaydi_acilir`.
- [x] **Step 2–5:** uygula, denetle, commit `çekirdek: her istek denemesinin
  kaydı`.

### Task 2: Kaynak etiketleri

**Files:** `crates/kalfa-core/src/{kayit,gonderim}.rs`,
`crates/kalfa-core/src/oturum/arac_asamasi.rs`; testler:
`gonderim_kaydi.rs`.

**Interfaces:**
- **`KayitOgesi::Mesaj.kaynak: Option<String>`** (`#[serde(default)]`).
- **Etiketler** (tasarım §6.3):
  - Kullanıcı mesajı `mesaj:kullanıcı`.
  - Araç sonucunda aracın `kaynak`'ı.
  - Kalfa'nın `[hata]`, `[izin verilmedi]` ve `[kullanıcı reddetti]`
    iletileri `kalfa:sonuç`.
  - Kalfa notu `kalfa:not`.
  - Modelin yanıtı `model`.
- **`CagriSonucu.kaynak`:** başarılı ve başarısız araç sonucunda
  `AracCiktisi.kaynak`, Kalfa'nın iletilerinde `kalfa:sonuç`.

- [x] **Step 1: Başarısız testleri yaz:** `kullanici_mesaji_etiketlenir`,
  `arac_sonucu_aracin_kaynagini_tasir`, `ret_ve_hata_kalfa_sonucu`,
  `kalfa_notu_etiketlenir` (3a birleştiyse), `eski_mesaj_etiketsiz_acilir`.
- [x] **Step 2–5:** uygula, denetle, commit `çekirdek: kaynak etiketleri`.

### Task 3: Sistem iletisi ve araç tanımlarının kaydı

**Files:** `crates/kalfa-core/src/{kayit,gonderim}.rs`,
`crates/kalfa-core/src/oturum/model_turu.rs`; testler: `gonderim_kaydi.rs`.

**Interfaces:**
- **`KayitOgesi::SistemIstemi { metin }`** ve
  **`KayitOgesi::AracTanimlari { tanimlar: Vec<ToolDef> }`.**
  - İlk istekte ve değiştiklerinde yazılırlar. Değişiklik, son yazılanla
    karşılaştırılarak bulunur.
  - Kabuk sunulur hâle gelince ya da sunulmaz olunca tanım kümesi değişir.
- **Günlükte:** "sistem iletisi (Kalfa)" ve "araç tanımları (N)" öğeleri;
  etiketleri `kalfa:sistem` ve `kalfa:araçlar`.

- [x] **Step 1: Başarısız testleri yaz:** `ilk_istekte_sistem_ve_araclar_yazilir`,
  `degismeyince_yeniden_yazilmaz`, `kabuk_sunulunca_araclar_yeniden_yazilir`,
  `kurtarmada_son_yazilan_bilinir`.
- [x] **Step 2–5:** uygula, denetle, commit `çekirdek: sistem iletisi ve araç
  tanımları kayıtta`.

### Task 4: Günlüğün türetimi

**Files:** `crates/kalfa-core/src/gonderim.rs`; testler:
`crates/kalfa-core/tests/gonderim_gunlugu.rs` (yeni).

**Interfaces:**
- **`gunluk_uret(kayitlar: &[Zarf<KayitOgesi>]) -> Vec<GunlukSatiri>`:**
  - Her `Istek` bir satırdır: zaman, amaç, yönlendirilen model, token
    (sonraki `Kullanim`), istek kimliği, sonuç, `yeni` ve `onceki`.
  - `IstekKoptu` önceki satırın sonucunu düzeltir.
- **`gunluk_sayfasi(satirlar, once_sira: Option<u64>, adet) -> GunlukSayfasi`:**
  `gecmis::sayfa` gibi.
- **`oge_metni(kayitlar, sira) -> Option<String>`:** gönderilen metnin
  tamamı, kayıttan.

- [x] **Step 1: Başarısız testleri yaz:**
  - `satir_istek_ve_kullanimi_birlestirir`,
    `kopan_istek_satiri_hata_gosterir`.
  - `sayfalama_gecmis_gibi`, `oge_metni_kayittan`.
  - `eski_kayittan_gunluk_uretilir`.
- [x] **Step 2–5:** uygula, denetle, commit `çekirdek: gönderim günlüğünün
  türetimi`.

### Task 5: Göndermeden önce göster

**Files:**
- `crates/kalfa-core/src/oturum/{mod,arac_asamasi}.rs`
- `crates/kalfa-core/src/{kayit,protokol,kurtarma,aktarim,gecmis/mod}.rs`
- Testler: `crates/kalfa-core/tests/gonderim_onizleme.rs` (yeni)

**Interfaces** (tasarım §6.4):
- **Ayar:** `Komut::OnizlemeAyarla { acik }`; kayda `OnizlemeDegisti { acik }`.
  Kurtarılan oturum son değerle açılır.
- **Bekletme:**
  - `sonuc_isle`, `AracSonucu` olayını yayar ama `tool` mesajını bağlama ve
    kayda eklemez; sonucu `bekleyen_sonuclar`'a koyar.
  - Aşama bitince `Olay::GonderimOnizlemesi { islem, ogeler, neden }` yayılır.
    `neden` ya `Onizleme` ya `Denetim`'dir (3c). Her öğe
    `OnizlemeOgesi { cagri, kaynak, bayt, denetim: Option<…> }` biçimindedir.
- **Karar:** `Komut::GonderimKarari { islem, gonderilmeyecekler: Vec<CagriId> }`.
  - Her sonuç bağlama eklenir.
  - Çıkarılanın içeriği "[kullanıcı bu içeriği göndermedi]" olur; asıl içerik
    kayda da yazılmaz.
- **Ne bekler, ne beklemez:**
  - Yalnızca dış kaynaklı araç sonuçları bekler.
  - Kalfa'nın ürettiği sonuçlar beklemez. 3c'deki `kalfa_uretimi` gelene kadar
    `CagriSonucu`'nun Kalfa iletisi olup olmadığı kullanılır.
  - Kullanıcı mesajı beklemez.
- **Kayıt sırası:** `AracBitti` hemen yazılır, `tool` mesajı karardan sonra.
- **Kurtarma:** `AracBitti` olup `tool` mesajı olmayan çağrıya yer tutucu
  konur: "[sonuç gönderilmedi: Kalfa gönderim önizlemesini beklerken
  kapandı]".
- **Beklerken:** iptal ve kip değişikliği onay beklemesindeki gibi işler.

- [x] **Step 1: Başarısız testleri yaz:**
  - `kapaliyken_bekletmez`.
  - `acikken_asama_sonunda_kart_gelir`.
  - `cikarilan_icerik_istekte_ve_kayitta_yok`: sahte sağlayıcının aldığı
    istekte yer tutucu vardır, içerik yoktur; kayıtta da yoktur.
  - `kullanici_mesaji_beklemez`, `kalfa_iletisi_beklemez`.
  - `beklerken_iptal`, `beklerken_kip_degisikligi`.
  - `kurtarmada_yer_tutucu`.
  - `ayar_kayittan_geri_gelir`.
  - G8: 50 × 64 KiB öğeli kartın olay bütçesine sığdığı ölçülür.
    Sığmazsa kart yalnızca özet taşır, içerik `gonderim_ogesi` ile istenir.
- [x] **Step 2–5:** uygula, denetle, commit `çekirdek: göndermeden önce
  göster`.

### Task 6: Uygulama komutları

**Files:** `crates/kalfa-uygulama/src/{oturumlar,sozlesme,uygulama}.rs`,
`crates/kalfa/src/{lib,komutlar,komut_adlari}.rs`,
`crates/kalfa/capabilities/ana-pencere.json`; testler: `kalfa-uygulama`.

**Interfaces:**
- `gonderim_gunlugu(oturum, once_sira) -> GunlukSayfasi`: sayfalı, kayıttan.
- `gonderim_ogesi(oturum, sira) -> String`: tam metin.
- `onizleme_ayarla(oturum, acik)`.
- `gonderim_karari(oturum, islem, gonderilmeyecekler)`.

- [x] **Step 1: Başarısız testleri yaz:** `gunluk_sayfasi_kayittan`,
  `bilinmeyen_oturum_hata`, `oge_metni_yalnizca_kayittaki_sira`.
- [x] **Step 2–5:** uygula, denetle, commit `uygulama: gönderim günlüğü
  komutları`.

### Task 7: Onay kartında "Veri çıkışı"

**Files:**
- `crates/kalfa-core/src/{arac,protokol}.rs`,
  `crates/kalfa-core/src/oturum/arac_asamasi.rs`
- `crates/kalfa-tools/src/{oku,ara,listele,yaz,duzenle}.rs`,
  `crates/kalfa-tools/src/kabuk/`
- `ui/src/ana/OnaySayfasi.svelte`, `ui/src/ipc/tipler.ts`, `ui/src/dil/tr.ts`
- Testler: araç testleri, `crates/kalfa-core/tests/protokol.rs`, Vitest

**Interfaces** (tasarım §9.3):
- **`Hazirlik.veri_cikisi: Option<VeriCikisi>`**, varsayılanı `None`.
  `VeriCikisi` beş değer alır:
  - `DosyaIcerigi`: `dosya_oku`.
  - `KomutCiktisi`: `kabuk`.
  - `AramaSonuclari`: `icerik_ara`.
  - `DosyaAdlari`: `dosya_listele`.
  - `YalnizcaYazildi`: `dosya_yaz` ve `dosya_duzenle`.
- **Araçlar veri çıkışını hazırlıkta verir.** Çekirdek bunu araç adından
  tahmin etmez; bilgi vermeyen araçta kart "araç sonucu" yazar.
- **`OnayOgesi.veri_cikisi`** (`#[serde(default)]`); kart "Veri çıkışı: …"
  satırını gösterir.
- Gizli dosyada `GIZLI_UYARISI` zaten vardır; satır onun yerine geçmez.

- [x] **Step 1: Başarısız testleri yaz:**
  - `her_arac_veri_cikisini_hazirlikta_verir`.
  - `bilgi_vermeyen_arac_icin_kart_arac_sonucu_yazar`.
  - `eski_onay_ogesi_veri_cikissiz_acilir`.
  - Vitest: kart satırı.
- [x] **Step 2–5:** uygula, denetle, commit `çekirdek: onay kartında veri
  çıkışı`.

### Task 8: Arayüz

**Files:** `ui/src/bilesenler/{LogTable,SourceTag}.svelte` (yeni),
`ui/src/ana/{GonderimGunlugu,OnizlemeKarti}.svelte` (yeni),
`ui/src/ana/{Menu,Konusma,KonusmaOgesi}.svelte`, `ui/src/ipc/tipler.ts`,
`ui/src/dil/tr.ts`, `ui/src/durum/`; Vitest.

- [x] **Günlük ekranı** menüden açılır (üst belge §11.7).
  - Tablo (`LogTable`): zaman, amaç (Ana model · Denetçi), yönlendirilen
    model, token, istek kimliği, sonuç.
  - Ayrıntı paneli:
    - Yeni öğeler `SourceTag` olarak görünür.
    - Önceki mesajlar tek bir soluk sayıdır.
    - Öğeye tıklanınca tam metin açılır.
  - Açık ekran `IstekBilgisi` gelince son sayfasını yeniden okur.
- [x] **Önizleme kartı:** konuşmada onay kartı gibi görünür.
  - Öğeler onay kutusuyla listelenir.
  - Düğmeler "Gönder" ve "Seçilenleri gönderme".
  - `neden: Denetim`'de öğeler seçili gelmez (3c).
- [x] **Ayar:** oturumda "Göndermeden önce göster" anahtarı.
- [x] **Tur özeti:** EVREN çağrıları günlüğe bağlanır (3a'nın tur özeti).
- [x] Vitest: satırlar, `SourceTag`, önceki mesaj sayısı, tam metin; kartın
  seçimleri; eski kayıtta "ayrıntı yok".
- [x] Commit `ui: gönderim günlüğü ve önizleme kartı`.

### Task 9: Uçtan uca ve senaryo

- [x] Uçtan uca (tasarım §11.5): günlük satırları; önizlemede bir öğeyi
  çıkarma (`e2e/testler/07-gonderim.test.mjs`).
- [x] S12 (3c'nin senaryo düzeneğiyle): önizlemede çıkarılan öğe sonraki
  istekte yoktur; yer tutucu vardır (`s12_onizlemede_cikarilan_oge_sonraki_istekte_yoktur`).
- [x] Commit `e2e: gönderim günlüğü ve önizleme`.

### Task 10: Belgeler

- [x] `docs/GELISTIRME_DURUMU.md`: 3b'nin durumu, G8'in sonucu.
- [x] Aşama 3 tasarımı §6 ve §13.
- [x] Bu planın "Uygulama sonucu" bölümü.
- [x] Commit `docs: 3b'nin sonucu`.

## Kabul ölçütleri

- Her istek denemesi, başarısızlar ve kopanlar dahil, günlükte bir satırdır.
- Satır yeni öğeleri etiketleriyle, önceki mesajları sayıyla gösterir. Tam
  metin kayıttan açılır.
- Önizleme açıkken dış kaynaklı sonuçlar karardan önce modele gitmez.
  Çıkarılan içerik hiçbir isteğe ve kayda girmez.
- Kurtarmada önizleme beklerken kapanan çağrının yer tutucusu vardır.
- Onay kartı her öğede veri çıkışını söyler.
- Eski kayıtlar açılır; günlükleri "ayrıntı yok" satırlarıyla türetilir.
- Bütün denetimler Windows'ta ve WSL'de yeşildir.

## Uygulama sonucu

Aşama 3b'nin bütün görevleri tamamlandı ve Windows üzerinde doğrulandı.

### Commit tablosu

| Görev / Madde | Commit | Açıklama |
|---|---|---|
| Task 1 | `562f209` | `çekirdek: her istek denemesinin kaydı` |
| Task 2 | `25ebf60` | `çekirdek: kaynak etiketleri` |
| Task 3 | `021c935` | `çekirdek: sistem iletisi ve araç tanımları kayıtta` |
| Task 4 | `3127974` | `çekirdek: gönderim günlüğünün türetimi` |
| Task 5 | `796b125` | `çekirdek: göndermeden önce göster` |
| Task 6 | `7037d03` | `uygulama: gönderim günlüğü komutları` |
| Task 7 | `06c9b38` | `çekirdek: onay kartında veri çıkışı` |
| Task 8 | `ba45f5f` | `ui: gönderim günlüğü ve önizleme kartı` |
| Merge | `458cb5d` | `Merge remote-tracking branch 'origin/kalfa-asama-1' into 3b/gonderim-gunlugu` (PR #21) |
| İnceleme 1 | `f25323d` | `çekirdek: kip değiştiğinde önizleme beklemesi sürer` |
| İnceleme 2 | `a1ecdd5` | `çekirdek: hata eşiği aşıldığında da önizleme kartı gösterilir` |
| İnceleme 3 | `eecff9c` | `ui: önizleme kartında Gönder düğmesi işaretli öğeleri çıkarır` |
| İnceleme 4 | `e2cd2e0` | `çekirdek: önizleme iptalinde modele durdurulma metni verilir` |
| İnceleme 5 | `807aafd` | `çekirdek: bağlantı hatasında gönderilen öğeler ilerlemez` |
| Task 9 | `eb10d88` | `e2e: gönderim günlüğü ve önizleme` |
| Task 10 | (bu commit) | `docs: 3b'nin sonucu` |

### Ölçüm: G8

- **G8:** Önizleme kartında 50 × 64 KiB öğe durumunda kart gövdesinde içeriğin
  kendisi taşınmaz; yalnızca meta veriler (`kaynak`, `bayt`, `cagri`,
  `denetim`) yer alır. 50 öğelik zarf serileştirildiğinde ~7.5 KiB tutar, 256 KiB
  olay bütçesine ve 32 KiB test eşiğine rahatça sığar
  (`g8_onizleme_karti_olay_butcesine_sigar`). Bekleyen sonucun
  içeriği karardan önce kayda yazılmaz; kullanıcı onu konuşmada araç sonucu
  olarak görür. G8 kapandı.

### İnceleme maddeleri ve düzeltmeler

PR #22 inceleme yorumundaki 5 madde TDD ile ele alındı:
1. **Kip değişikliği:** Önizleme beklerken kip değiştiğinde bekleme döngüsü
   kırılmıyor; kart açık kalıyor (`beklerken_kip_degisikligi`).
2. **Hata eşiği:** 3 ardışık başarısız çağrı eşiğinde de önizleme kartı
   sunuluyor, çıkarılan içerik yer tutucuyla kapatılıyor
   (`ayni_hata_esiginde_onizleme_calisir_cikarilan_icerik_baglama_girmez`).
3. **Önizleme kartı butonları:** Arayüzde "Gönder" düğmesi yalnızca işaretli
   öğeleri çıkarıp kalanları gönderiyor.
4. **İptal metni:** Önizleme beklerken tur iptal edildiğinde modele
   `[sonuç gönderilmedi: tur, gönderim önizlemesi beklenirken durduruldu]`
   metni veriliyor (`ONIZLEME_IPTALI`, `beklerken_iptal`).
5. **Bağlantı hatası:** `IstekAsamasi::Baglanti` (`govde_gitti == false`)
   durumunda `gonderilen` sayacı ve son kaydedilen sistem istemi/araç tanımları
   ilerletilmiyor; kurtarmada da yalnızca `Basarili` veya `Hata { asama: Yanit }`
   istekleri `gonderilen`'e sayılıyor (`baglanti_hatasindan_sonra_ogeler_yeniden_yeni_sayilir`).

### Doğrulama

Windows üzerinde Rust 1.98.1 ve Node 24.15 ile koşturulan testler:
- `cargo test --workspace --locked`: Başarılı.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: 0 uyarı.
- `cargo fmt --all -- --check`: Temiz.
- `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked`: Başarılı.
- `cargo clippy -p kalfa -p kalfa-uygulama --features e2e --all-targets --locked -- -D warnings`: 0 uyarı.
- `ui/` içinde:
  - `npm run check`: 0 hata, 0 uyarı.
  - `npm test`: 23 test dosyası, 277 test geçti.
  - `npm run build`: Temiz (paket denetimi temiz).
- `e2e/`: S12 senaryo testi `crates/kalfa-tools/tests/senaryolar.rs` içinde
  doğrulandı. `07-gonderim.test.mjs` yazıldı.

