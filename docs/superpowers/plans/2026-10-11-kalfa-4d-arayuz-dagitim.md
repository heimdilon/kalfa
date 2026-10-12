# Kalfa 4d — Arayüz ve dağıtım: Plan ve uygulama kaydı

**Goal:** Eklentiler kullanıcıya ulaşır. Bu planın sonunda:
- **Eklentiler ekranı:** yerleşik ve kurulu eklentiler; durum, yetenek
  cümlesi, SHA-256, araçlar; klasörden kurma, güncelleme, kaldırma, devre dışı
  bırakma; çalışma alanında açma/kapama, EVREN gönderim izni ve depo.
- **Kalıcı kayıt:** `<veri>/eklentiler.json`. Gönderim izni ve çalışma
  alanındaki açıklık oradadır; eklenti aracı her turda kayda bakar.
- **Oturuma bağlama:** yüklenebilen her eklentinin araçları oturuma girer;
  kapalı, devre dışı, kaldırılmış ya da güncellenmiş eklentinin araçları
  açık oturumda da sunulmaz.
- **Kartlar:** onay sayfasında ilk gönderim kartı; önizleme kartında
  "Metni göster"; oturum panelinde eklentiler; durum çubuğunda EVREN günlük
  sınırı.
- **Dağıtım:** `kalfa-rag` NSIS kurucusunda yerleşik eklenti olarak gelir.
- **Testler:** S17, eklenti deposu ve kaydının yalıtımda görünmezliği,
  uçtan uca `11-eklenti`.

**Architecture:**
- **`kalfa-plugin`:** `GonderimIzni::acik_mi` (varsayılan `true`) ve
  `EklentiAraci::sunulur_mu`; `Manifest::yetenek_cumlesi`,
  `yeni_yetenekler`, `gonderim_kapsami_degisti`. Test eklentisinin `talimat`
  aracı (S17).
- **`kalfa-core`:** `OturumTutamaci::bekleyen_eklenti_govdesi(islem)`
  (önizlemede bekleyen gövde; karar ya da tur bitince bırakılır).
  `OnayOgesi.eklenti`: onay öğesi eklentinin tarifini taşır.
- **`kalfa-uygulama`:** `eklentiler.rs` (on komut), `eklentiler/kayit.rs`
  (`eklentiler.json`, `AlanIzni`), `eklentiler/yukleyici.rs` (klasör ve
  dizin okuma, `Motor`, önbellek, oturuma bağlama). `baglam` async oldu.
  `DegisirSaglayici` günlük sınırı izler. Dizinler: `kurulu_eklenti_dizini`,
  `eklenti_verisi_dizini` (Windows'ta `%LOCALAPPDATA%\Kalfa\eklenti-verisi`),
  `yerlesik_eklenti_dizini` (kaynak dizini ya da `KALFA_YERLESIK_EKLENTILER`).
  E2e sahte EVREN gömme ve sıralamayı yanıtlar.
- **`kalfa`:** on komut, ana pencere yetkileri, dizinlerin bağlanması.
  `derle.mjs` `kalfa-rag`'ı sürüm profiliyle derler; `tauri.nsis.conf.json`
  `eklentiler/`'i kaynak olarak alır.
- **Arayüz:** `Eklentiler.svelte`, `IlkGonderimKarti.svelte`,
  `EklentiMetni.svelte`; menü, oturum paneli, onay sayfası, önizleme kartı,
  durum çubuğu; sahte arka uç ve önizleme senaryoları.

**Kaynaklar:**
- [Aşama 4 tasarımı](../specs/2026-10-07-kalfa-4-eklenti-rag-design.md):
  §8.4, §9.4, §10, §13, §14, §15, §16.2, §16.3 (S17), §16.5, §17.
- [4b planı](2026-10-08-kalfa-4b-evren-yolu.md) (4d'ye bırakılanlar),
  [4c planı](2026-10-09-kalfa-4c-rag.md).

**Uygulayan:** Claude ve iki `kalfa-sonnet` alt ajanı (2026-10-11). Claude
ortak arayüzü yazdı ve derledi (`7cacd8e`, `9fb4a2e`). Alt ajan A
`kalfa-uygulama`'nın eklenti mantığını, alt ajan C arayüzü ayrı
worktree'lerde yazdı; ajanlar commit atmadı. Claude kurucuyu, S17'yi,
uyumluluk testini, e2e sahte EVREN'ini ve `11-eklenti`'yi yazdı; ajan
çıktılarını inceledi, birleştirdi ve iki platformda doğruladı.

| Görev | Kim | Dosyalar |
|---|---|---|
| Ortak arayüz: komut imzaları, sözleşme tipleri, dizinler, `acik_mi`, yetenek cümlesi, bekleyen gövde, onay öğesinin tarifi | Claude | `kalfa-plugin/src/{evren,arac,manifest}.rs`, `kalfa-core/src/{protokol.rs,oturum/}`, `kalfa-uygulama/src/{sozlesme,dizin,uygulama}.rs`, `kalfa/src/`, `ui/src/ipc/` |
| Kayıt, kurulum, güncelleme, kaldırma, alan, izin, oturuma bağlama, günlük sınır | alt ajan A | `kalfa-uygulama/src/eklentiler*`, `oturumlar.rs`, `evren.rs`, `guvenlik.rs`, `tests/eklentiler.rs` |
| Eklentiler ekranı, kartlar, oturum paneli, durum çubuğu | alt ajan C | `ui/src/ana/`, `ui/src/dil/tr.ts`, `ui/src/sahte/`, `ui/src/onizleme/` |
| Kurucu, S17, uyumluluk testi, e2e sahte EVREN'i, `11-eklenti`, inceleme düzeltmeleri, doğrulama | Claude | `kalfa/kurucu/derle.mjs`, `kalfa/tauri.nsis.conf.json`, `kalfa-plugin/tests/senaryo.rs`, `kalfa-tools/tests/uyumluluk.rs`, `kalfa-uygulama/src/e2e/saglayici.rs`, `e2e/` |

**Sıra:** 4c'nin (#52) üstüne yığılıdır: `kalfa-rag` ve günlük sınır
(`EvrenError::GunlukSinir`) 4c'dedir. #52 birleşince bu dal ana dala
taşınır.

**Kapsam dışı:**
- **Linux paketi:** depoda Linux paketlemesi (`.deb`, `.rpm`) henüz yok;
  `kalfa-rag` yalnızca NSIS kurucusuna girer. Paketleme gelince aynı
  `eklentiler/` dizini kaynak olarak eklenir (§17.3).
- **Oturum açıkken kurulan eklenti:** yeni araçları o oturuma eklenmez;
  oturum yeniden açılınca gelir. Eski sürümün araçları hemen sunulmaz olur.
- **`kalfa-rag`'ın dakikalık pencerede takılması:** 4c ölçümünün bulgusu
  ([kayıt](../bulgular/2026-10-11-kalfa-4c-ilk-indeks.md)); 429 gövdesi
  kaydedilince ayrı iş.

## Kararlar

1. **Kurulum kartı yerel onay penceresidir.** Tasarım §13.2 arayüzde bir
   kart ve ardından yerel onay söylüyordu. Tek komut (`eklenti_klasorden_kur`)
   klasörü seçer, doğrular, kart metnini (ad, sürüm, açıklama, SHA-256,
   kaynak, yetenek cümlesi, araçlar; güncellemede yeni yetenekler ve
   silinecek izinler) Rust'ta üretip yerel pencerede sorar ve kurar. Komutlar
   arasında durum tutulmaz; webview'a sızmış bir betik kurulumu başka bir
   klasöre yönlendiremez.
2. **Kurulan baytlar doğrulanan baytlardır.** Klasör bir kez belleğe okunur
   (`kalfa.toml` ≤ 64 KiB, tek `.wasm` ≤ 64 MiB, bağlantı izlenmez), aynı
   baytlar yüklenir, gösterilir ve yazılır.
3. **Açıklık ve izin her turda kayıttan okunur** (`AlanIzni`). Eklenti
   aracı, eklenti kayıtta duruyor, etkin, kurulu sürümü (SHA-256 ve kurulum
   zamanı) aracın yüklendiği sürümle aynı ve o alanda açıksa sunulur.
   İzin yazılamazsa araç çalışmaz.
4. **Yerleşik eklentinin kapsamı izlenir.** Kayıt son görülen manifesti
   saklar; yeni manifest gönderim kapsamını değiştirdiyse bütün gönderim
   izinleri silinir. Yerleşik eklentinin SHA-256'sı karşılaştırılmaz
   (dosyalarına yalnızca yönetici yazabilir, §13.1).
5. **Manifestin gösterilen alanları tek satırlı olmalı.** Açıklamada ya da
   `dosya-oku` kalıbında satır sonu veya görünmez karakter varsa kurulum
   reddedilir: eklenti yerel pencerede sahte satırlar çizemez.
6. **Tarihler Unix ms'dir** (`kurulum_ms`, `gonderim_izni_ms`); kod
   tabanının öteki kayıtlarıyla aynı. Tasarımdaki RFC 3339 örneği yerine.
7. **Günlük sınır durumu uygulama katmanındadır.** `DegisirSaglayici`
   sohbet, denetçi ve eklenti isteklerinden `GunlukSinir`'i görünce saklar,
   başarılı bir istek görünce kaldırır. Arayüz turun sonunda ve açılışta
   `gunluk_sinir` ile sorar. Çekirdeğe yeni olay eklenmedi.
8. **İlk gönderimin yerel penceresi neyi onayladığını söyler.** Alt ajan A
   buldu: pencere "Çalışma alanını yalıtım için hazırla" başlığıyla ve boş
   metinle açılıyordu. Başlık "Eklenti EVREN'e veri gönderecek", metin
   kartın açıklamasıdır (Markdown kalın işaretleri atılır, görünmez
   karakterler işaretlenir). Öğe onay öğesindeki tariften
   (`eklenti.ilk_gonderim`) tanınır.
9. **Arayüz eklentinin metnini düz metin olarak çizer** (`GorunurMetin`).
   Yetenek cümlesindeki ters tırnaklı kalıplar Markdown'sız `<code>` olur;
   kötü niyetli bir glob Markdown bağlantısı enjekte edemez.

### Tasarımdan farklar

| Tasarım | Uygulama | Neden |
|---|---|---|
| Kurulum kartı arayüzde, onay yerel pencerede (§13.2) | Kart metni yerel pencerededir (karar 1) | Komutlar arasında durum yok; arayüz klasörü ya da baytları yönlendiremez |
| `eklentiler.json`'da RFC 3339 zamanlar (§13.4) | Unix ms; yerleşik girdide `manifest` metni | Kod tabanıyla tutarlılık; yerleşik kapsam izleme (karar 4) |
| Linux paketine `kalfa-rag` (§17.3) | Yalnızca NSIS | Linux paketlemesi henüz yok |
| Günlük sınırın ayrı arayüz durumu (4c açık işi) | `gunluk_sinir` komutu ve durum çubuğu öğesi | Çekirdek değişmedi (karar 7) |
| Eklenti deposunun uyumluluk testi (§16.2) | Depo, `eklentiler.json` ve kurulu `.wasm` birlikte; kayda yazma da sınanır | Kabuk kendine gönderim izni yazamamalı |

Tasarım belgesi bu farklara göre güncellendi.

## Doğrulama

Baş `017aacd` (inceleme düzeltmeleri dahil), 2026-10-11.

| | Windows | WSL (Ubuntu 24.04, `kalfatest`) |
|---|---|---|
| `cargo test --workspace` | 1090 geçti, 0 düştü, 18 atlandı (canlı) | geçti, 0 düştü (sayı kaydedilmedi) |
| `cargo test -p kalfa-uygulama -p kalfa --features e2e` | 175 geçti | geçti |
| Üç clippy (+ yalıtım özellikli), `cargo fmt --check` | temiz | temiz |
| Arayüz (`check`, `test`, `build`) | 0 hata, 0 uyarı; 388 test; paket denetimi temiz | aynı |
| Yalıtım / uyumluluk | uyumluluk 24/24 (AppContainer) | yalıtım 26, uyumluluk 16/16 |
| Uçtan uca (`e2e/`) | 38/38 (`11-eklenti` dahil), gecikme p95 25–26 ms | 38/38, gecikme p95 33–40 ms |

Yeni testler: `kalfa-uygulama/tests/eklentiler.rs` (29) ve birim testleri,
`kalfa-plugin` manifest (2) ve S17 (senaryo + açıklama sınırı),
`kalfa-core/tests/eklenti_ortami.rs`'e bekleyen gövde denetimleri, uyumluluk
testi `eklenti_deposu_ve_kaydi_yalitimda_gorunmez`, arayüzde 79, e2e
`11-eklenti` (3). Bozma denemeleri: `AlanIzni::acik_mi` her zaman `true`
yapılınca dört test düştü; arayüz ajanı 20 bozmayı testlerin yakaladığını
gördü.

**İncelemede düzeltilenler:**
- Eklentinin ilk gönderiminin yerel penceresi yanlış başlık ve boş metinle
  açılıyordu (karar 8).
- Uçtan uca derleme sürüm profilinde olduğu için yerleşik eklenti
  değişkenini okumuyordu; `e2e` özelliği de açar.
- İzin panelinin başlığı kapalı eklentileri de sayıyordu; yerleşik
  `kalfa-rag`'la 3e'nin kısayol e2e'si düştü. Başlık yalnızca açık
  eklentileri sayar; panel eklentiler için görünür kalabilir.
- Linux'ta bwrap yalıtım dizininin üstlerini kabın içinde tmpfs'te kurar;
  kabuğun kayda yazması oraya gider. Uyumluluk testi gerçek kaydın
  değişmediğini denetler.

**G1 (4d):** `kalfa.exe` (Windows, sürüm) 24,68 MB'tan 41,85 MB'a
(+17,2 MB, %70). Tasarım +~14 MB öngörmüştü.

## PR ile yürütme

Kod bu PR'da yazılmış ve doğrulanmıştır; uygulayıcıya iş kalmadı. İnceleme:
`git diff origin/4c/plan...4d/plan`, iki platformda AGENTS.md'deki
komutlar ve `e2e/` (`11-eklenti`).

### Codex incelemesi — 11 Ekim

Yerleşik eklentiyle aynı adlı eski kurulum kaydı desteklenen bir durumdur:
yükleyici kurulu kaydı kullanılmıyor diye işaretler, kaldırılabilmesi için
listede tutar. Eklentiler ekranı ve oturum izin paneli yalnız `ad` ile
anahtarlanıyordu; iki kayıt gelince Svelte `each_key_duplicate` hatasıyla
ekranı bozuyordu. İki liste artık yerleşik/kurulu ayrımı ve adla anahtarlanır.
Arayüz sahtesinde kaldırma yalnız kurulu kaydı siler; yerleşik korunur.

İki yeni regresyon, düzeltme öncesinde aynı hata ile düştü; düzeltmeden
sonra geçti. Biri kullanılmayan kurulumun kaldırılmasını ve yerleşiğin
korunmasını, diğeri izin panelindeki iki satırın doğru açık/kapalı ve
etkin/etkisiz durumunu sınar. Üretim Rust kodu değişmedi.

`eb86baa` üzerine bu arayüz yaması, mevcut kilitli bağımlılıklarla ve
paket indirmeden iki platformda doğrulandı:

| Kontrol | Windows | WSL Ubuntu 24.04, `kalfatest` |
|---|---|---|
| `cargo test --workspace --locked --offline` | 1090 geçti, 0 düştü, 18 atlandı | 1033 geçti, 0 düştü, 18 atlandı |
| e2e özellikli Rust testleri | 175 geçti | 175 geçti |
| Düz/e2e/wasm clippy, `cargo fmt --check` | temiz | temiz |
| Arayüz `check`, `test`, `build` | 0 hata/uyarı, 390 test; paket denetimi temiz | aynı |

Bu geçişte gerçek AppContainer/bwrap uyumluluğu ve `tauri-driver` e2e'si
yeniden çalıştırılmadı; yukarıdaki 38/38 ve yalıtım sonuçları Claude'un
değişmeyen arka uç için `017aacd` doğrulamasıdır. Günlükler Windows'ta
`target/4d-inceleme-20261011/windows`, WSL'de ortak target altındaki
`4d-inceleme-20261011/wsl` dizininde korunur. Yayınlanmış `9980516` için
[CI 148](https://github.com/heimdilon/kalfa-arsiv/actions/runs/38145500135)
dört işiyle yeşildi. 12 Ekim'de #52 `7ada5fa` olarak birleşti; #53'ün
18 commit'i `bc3aaf4`'ten yeni ana dala çakışmasız taşındı ve tabanı
`kalfa-asama-1` oldu. Taşımayla gelen fark yalnız #52'nin sonraki belge
commit'leri ve `degerlendirme.rs`'teki test seçeneğidir; 4d kodu değişmedi.
