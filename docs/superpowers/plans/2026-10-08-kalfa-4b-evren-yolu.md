# Kalfa 4b — EVREN yolu: Plan ve uygulama kaydı

**Goal:** Eklentinin `evren-cagir` host çağrısı gerçek olur. Bu planın sonunda:
- **İstemci:** `kalfa-evren` gömme (`/v1/embeddings`) ve yeniden sıralama
  (`/v1/rerank`) isteklerini gönderir; yanıt ham metin olarak döner.
- **Doğrulama:** eklentinin gövdesi uca göre izin listesiyle doğrulanır;
  model katalogda olmalı ve görevi uca uymalıdır. Anahtarı, adresi ve
  yöntemi host ekler.
- **İlk gönderim onayı (K3):** `evren` yeteneği olan eklentinin bir çalışma
  alanındaki ilk çağrısı her kipte yerel onay ister. Kart kaç dosyanın, kaç
  MB'ın gidebileceğini söyler.
- **Önizleme:** "Göndermeden önce göster" açıkken her eklenti isteği ağa
  çıkmadan durur. Kararlar: Gönder, Gönderme, Bu araç çağrısında hep gönder.
- **Günlük:** her eklenti isteği, başarısızı ve gönderilmeyeni dahil, kayda ve
  gönderim günlüğüne `eklenti:<ad>` amacıyla girer. Metin kayda yazılmaz.
- **Sınırlar:** çağrı başına 1 000 istek; oturum başına eklenti başına 5 000
  istek ve 256 MiB gövde. Oturum sayacı kurtarmada kayıttan yeniden kurulur.
- **S16:** önizleme açıkken sahte EVREN, karar gelmeden istek almaz.

**Architecture:**
- **`kalfa-evren`:** `EkUc` (`Gomme`, `Siralama`), `HamYanit`,
  `EvrenIstemcisi::ek_uc`. İstemci yönlendirme izlemez.
- **`kalfa-core`:**
  - `Saglayici::modeller` ve `Saglayici::ek_uc` (varsayılanı hata).
  - `AracOrtami`: aracın oturumla konuştuğu kanal. Canlı çıktı, gönderim
    talebi (`GonderimTalebi` → `GonderimYaniti`) ve isteğin sonucu
    (`OrtamIstegi::IstekBitti`). `Arac::calistir_ortamli`; varsayılanı
    `calistir_akisli`.
  - Araç aşaması kanalı dinler: oturum sınırı, önizleme kartı
    (`OnizlemeNedeni::Eklenti`), "hep gönder" kümesi, kayıt
    (`KayitOgesi::EklentiIstegi`), tur özeti sayısı.
  - İzin kararı: `EklentiTarifi.ilk_gonderim` → her kipte `YerelOnayli`.
  - Mühür: tarifin `dosya_sayisi` ve `bayt`'ı mühre girmez.
- **`kalfa-plugin`:**
  - `evren.rs`: `EvrenBaglami` (sağlayıcı, şart kapısı, model kataloğu
    önbelleği), `GonderimIzni` ve `BellekIzni`, `govde_dogrula`.
  - `host.rs`: `evren_cagir`'ın sırası (tasarım §9.2), okunan dosyaların
    listesi.
  - `EklentiAraci`: `ilk_gonderim`'i ve kartın dosya sayısını hesaplar;
    onaylı ilk gönderimde izni kaydeder.
- **`kalfa-uygulama`, `kalfa`, arayüz:** `gonderim_karari`'ya
  `hep_gonderilecekler`; önizleme kartının eklenti biçimi, günlükte eklenti
  satırı ve `Gonderilmedi`, onay sayfasında `EklentiCiktisi`.

**Kaynaklar:**
- [Aşama 4 tasarımı](../specs/2026-10-07-kalfa-4-eklenti-rag-design.md):
  §8.1–§8.7, §9, §15, §16.2, §16.3 (S16).
- [4a planı](2026-10-07-kalfa-4a-sozlesme-calisma-zamani.md).

**Uygulayan:** Claude ve Haiku 5.5 alt ajanları (2026-10-08). Claude önce
ortak arayüzü yazdı ve derledi (`2aad1d1`). Dört alt ajan ayrı worktree'lerde
paralel çalıştı. Claude çıktıları birleştirdi, inceledi, S16'yı yazdı ve iki
platformda doğruladı.

| Görev | Kim | Dosyalar |
|---|---|---|
| Ortak arayüz: tipler, trait yöntemleri, sahte EVREN | Claude | `kalfa-evren/src/eklenti.rs`, `kalfa-core/src/{arac,gonderim,kayit,protokol,saglayici,test_destek}.rs` |
| Oturum: kanal, önizleme, sınır, kurtarma, izin kararı, mühür, günlük | alt ajan | `kalfa-core/src/{oturum,kurtarma,muhur,gonderim,gecmis}`, `tests/eklenti_ortami.rs` |
| Host: gövde doğrulama, `evren_cagir`, ilk gönderim, test eklentisinin `evren` aracı | alt ajan | `kalfa-plugin/src/{evren,host,arac,sinir}.rs`, `tests/evren.rs` |
| `ek_uc` testleri, canlı test | alt ajan | `kalfa-evren/tests/ek_uc.rs`, `tests/canli.rs` |
| Uygulama ve arayüz | alt ajan | `kalfa-uygulama/src/oturumlar.rs`, `kalfa/src/komutlar.rs`, `ui/src/` |
| S16, inceleme düzeltmeleri, doğrulama | Claude | `kalfa-plugin/tests/senaryo.rs` |

**Sıra:** 4a'dan sonra. 4c (`kalfa-rag`) bunun üstünde yürür.

**Kapsam dışı:**
- Kalıcı gönderim izni (`eklentiler.json`), Eklentiler ekranındaki "Gönderim
  iznini kaldır", eklentinin uygulamaya bağlanması: **4d**. 4b'de izin
  `GonderimIzni` trait'inin arkasındadır; tek uygulaması bellektedir
  (`BellekIzni`).
- Önizleme kartındaki "Metni göster" (`eklenti_istegi` komutu): **4d**.
  Gövde bekleyen önizlemede bellekte tutulur.
- Arayüzün ilk gönderim kartı için ayrı bir bileşeni: bugünkü onay kartı
  `aciklama`'yı gösterir; **4d**'de ayrı kart.

## Kararlar

1. **Eklenti isteği kayıtta ayrı bir öğedir** (`KayitOgesi::EklentiIstegi`).
   Tasarım §9.3 `KayitOgesi::Istek`'i söylüyordu. `Istek` sohbet isteğidir:
   kurtarma ve geçmiş onu sohbetin parçası sayar, ardından gelen
   `Kullanim`/`IstekKoptu` ona iliştirilir. Ayrı öğe bu mantığı bozmaz.
   Günlükte ikisi aynı satır biçimine döner (`GunlukSatiri.eklenti`).
2. **Gövde ve özet kayda girmez; önizlemede bellekte kalır** (karar 8). Öğe
   uç, model, metin sayısı, bayt, metinlerin SHA-256'sı ve o çağrıda okunan
   dosyaları (en çok 1 000) taşır.
3. **Oturum sınırı oturumda sayılır.** Talep sınırı aşacaksa istek ağa
   çıkmaz, kayda `Gonderilmedi` girer. Sayaç yalnızca `Gonder` kararıyla
   artar. Kurtarmada `Gonderilmedi` olmayan `EklentiIstegi`'ler sayılır.
   Bilinen boşluk: izin verilip sonucu yazılmadan çöken istek sayılmaz (çağrı
   başına en çok bir).
4. **Tur özetinin "EVREN çağrıları" yalnızca ağa çıkan istekleri sayar.**
5. **Önizleme kartı döngüyü durdurmaz.** Kart açıkken öteki araçlar sürer;
   karar `GonderimKarari` ile gelir. İptal bekleyen talepleri `Iptal`'le
   bırakır.
6. **Kart metni:** ilk gönderimde `Hazirlik.aciklama`, aracın bugünkü
   cümlesinin altına §8.4'ün metnini ekler (dosya sayısı, MB ondalık
   virgülle, 1 MB = 1 000 000 bayt).
7. **Şart kapısı model kataloğundan önce denetlenir.** Şart beklenirken
   eklenti için ağa hiç çıkılmaz. 403 `terms_not_accepted` eklentiye
   `evren("EVREN şartları yeniden onay bekliyor")` olarak döner; kapıyı
   sonraki sohbet isteği açar.
8. **`input` gömmede zorunludur.** Tasarım yalnızca `model`'i zorunlu
   saymıştı; girdisiz gömme isteğinin anlamı yok.

### Tasarımdan farklar

| Tasarım | Uygulama | Neden |
|---|---|---|
| Eklenti isteği `KayitOgesi::Istek` (§9.3) | `KayitOgesi::EklentiIstegi` (karar 1) | Sohbet kurtarma ve geçmiş mantığı |
| `kalfa-evren`'de `gomme(GommeIstegi)` ve `yeniden_sirala(SiralamaIstegi)` (§9.1) | Tek `ek_uc(EkUc, &Value)` | Gövde host'ta doğrulanmıştır; tipler yalnızca günlüğün alanını (`model`) çözer |
| Yanıt en çok 32 MiB (§7.4, §9.2) | İstemcinin JSON sınırı (8 MiB) önce işler | `kalfa-rag` base64 ve 256 parçalık isteklerle ~1,5 MB alır. 512 float girdilik bir yanıt (~11 MB) `YanitSiniri`'yle düşer. |
| "Yönlendirme izlenmez (bugünkü ayar)" (§9.2) | İstemci yönlendirmeyi şimdi izlemiyor (`Policy::none()`) | reqwest varsayılanı izliyordu; testi yazan ajan buldu. Bütün EVREN çağrıları için geçerli. |
| `EklentiOrtami` köprüsü `kalfa-uygulama/src/eklentiler/`'da (§5.1) | `EvrenBaglami` `kalfa-plugin`'de, `Saglayici` ve `SartKapisi` üzerinden | 4a'nın kararı sürdü: testler gerçek yoldan, sahte sağlayıcıyla koşar. Uygulama 4d'de yalnızca kurar. |
| Gönderim izni `eklentiler.json`'da (§8.4) | `GonderimIzni` trait'i; 4b'de bellekte | Kayıt dosyası 4d'de |
| "Metni göster" (§9.4) | 4d | Arayüzün eklenti ekranlarıyla birlikte |

Tasarım belgesi bu farklara göre güncellendi.

## Doğrulama

| | Windows | WSL (Ubuntu 24.04) |
|---|---|---|
| `cargo test --workspace` | 918 geçti, 0 düştü, 12 atlandı (canlı) | 861 geçti, 0 düştü, 12 atlandı |
| `cargo test -p kalfa-uygulama -p kalfa --features e2e` | 114 geçti | 113 geçti |
| Üç clippy, `cargo fmt --check` | temiz | temiz |
| Arayüz (`check`, `test`, `build`) | 0 hata, 0 uyarı; 309 test | aynı |
| Yalıtım / uyumluluk | — | 26 / 15 geçti |

Yeni testler: `kalfa-core/tests/eklenti_ortami.rs` (18),
`kalfa-plugin/tests/evren.rs` (17) ve `src/evren.rs` birim testleri (12),
`kalfa-evren/tests/ek_uc.rs` (11), S16 (`kalfa-plugin/tests/senaryo.rs`),
arayüzde 5. Alt ajanlar izin kapısını, "Gönderme" kararını, ilk gönderimin
birleştirilmesini, izin kopyasını, sınır kontrolünü, mühür sıfırlamayı ve
kurtarma sayımını bilerek bozup ilgili testin düştüğünü gördü.

## PR ile yürütme

Kod bu PR'da yazılmış ve doğrulanmıştır; uygulayıcıya iş kalmadı. İnceleme:
`git diff origin/kalfa-asama-1...4b/plan`, iki platformda yukarıdaki
komutlar.
