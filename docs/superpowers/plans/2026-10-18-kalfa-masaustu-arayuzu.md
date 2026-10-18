# Kalfa masaüstü arayüzü — Uygulama planı

**Tarih:** 2026-10-18. **Durum:** tamamlandı; dört görev 18 Ekim'de
birleşti (main `032e9a3`).
**Tasarım:** [masaüstü arayüzü r2](../specs/2026-10-18-kalfa-masaustu-arayuzu-design.md)
(kullanıcı 18 Ekim'de onayladı).
**Taban:** `main`, `117dd8c`. **Görev:** KALFA-25. **Uygulayan:** Claude.

Dört görev, her biri ayrı dal ve PR; her biri iki platformda doğrulanır ve
kullanıcının açık onayıyla birleşir. Görev 4 başlık çubuğuna bağlıdır;
öbürleri bağımsızdır. Bu plan davranış sözleşmesidir; derlenmiş kod
bloğu içermez.

## Global Constraints

- Güvenlik davranışı değişmez: mühürlü onay, riskli öğenin tek tek
  seçilmesi, yerel onay penceresi, yalıtım ve gizli dosya kuralları.
- Arayüz arka uca dosya yolu göndermez; yeni komutlar kimlikle çalışır.
- Pencerelere Tauri çekirdek izinleri verilmez; yeni komutlar yalnız
  `ana-pencere` yeteneğine eklenir ve `komut_adlari` ile TS sözleşmesi aynı
  commit'te güncellenir.
- CSP: sabit `style="…"` ve ham HTML yok; dinamik değer `style:` yönergesiyle.
- Yeni crate ya da npm paketi yok. Üretimde `unwrap`/`expect` yok.
- Kullanıcıya görünen metinler `ui/src/dil/tr.ts`'te, "siz" diliyle.
- Modelin gördüğü araç sonucu metni değişmez.
- Her hata düzeltmesi ve yeni sınır, önce düşen testle gelir.

## Görev 1 — Araç kartında diff (tasarım §4)

**Dal:** `ui/arac-diff`.
**Dosyalar:** `kalfa-core/src/protokol.rs`, `oturum/arac_asamasi.rs`,
`gecmis/mod.rs`, ilgili testler; `ui/src/ipc/tipler.ts`,
`ui/src/durum/oturum.ts`, `ui/src/ana/KonusmaOgesi.svelte` (ya da
`bilesenler/ToolCallCard.svelte`), `ui/src/dil/tr.ts`, sahte arka uç ve
önizleme senaryosu.

1. [x] `DosyaDegisikligi` tipi ve `Olay::DosyaDegisiklikleri { cagri,
   dosyalar }`. `DosyaYedegi`'nden üretilir: `onceki == Yoktu` →
   `Olusturuldu`, öbürleri `Degisti`; `eklenen`, `silinen`, `diff` aynen.
2. [x] `arac_asamasi`: `AracCagrisiBasladi`'dan hemen sonra, yedek listesi
   boş değilse olay yayımlanır.
3. [x] `gecmis`: `GorunumOgesi::AracCagrisi.degisiklikler` (serde
   varsayılanı boş) `AracBasladi.yedekler`'den doldurulur.
4. [x] Arayüz: olay `AracCagrisi` öğesine bağlanır; geçmişten gelen liste
   aynı alana düşer. Kart: dosya adı, `+N −M`, `DiffView` ile en çok 20
   satır ve "Tamamını göster"; sonuç `Basarisiz`/`IptalEdildi` ise
   "uygulanmış olabilir" (yazma yarıda kalmış olabilir); `diff` yoksa
   "Diff gösterilmiyor".

**Kabul:** Çekirdek olay sırası, gizli dosyada `diff: None`, 64 KiB üstünde
`diff: None`, geçmişten açılan oturumda liste dolu, eski kayıt okunur;
arayüzde kart diff'i gösterir, başarısız çağrının değişikliğini "uygulanmış
olabilir" diye etiketler, uzun diff'i katlar.
**Commit:** `uygulama: araç kartında dosya diff'i`.
**Birleşti:** #25, `cff1c88`.

## Görev 2 — Konuşmadaki onay kartı (tasarım §3)

**Dal:** `ui/onay-karti`. Yalnız arayüz.

1. [x] `onay.ts`: kartın üç kararı için `kararlariUret` sarmalayıcıları.
2. [x] `OnayKarti`: öğe ayrıntıları (komut, dizin, rozetler, kısa diff),
   tablodaki düğmeler, riskli öğede onay düğmesi yok.
3. [x] Bekleyen kart yazma alanının üstünde yapışık; yanıtlanınca konuşmada
   kayıt satırı.

**Kabul:** Tasarım §3.4 testleri; e2e'de düzenleme ve yalıtımlı komut
karttan onaylanır, ağlı komut yalnız "İncele" ile.
**Commit:** `ui: konuşmadaki onay kartından karar`.
**Birleşti:** #26, `f8378fc`.

## Görev 3 — Kalfa'ya özel başlık çubuğu (tasarım §5)

**Dal:** `uygulama/baslik-cubugu`.

1. [x] `crates/kalfa`: `pencere_kucult`, `pencere_buyut_degistir`,
   `pencere_kapat`, `pencere_surukle`; yalnız çağıran pencereye uygulanır;
   izin dosyaları, `komut_adlari`, TS sözleşmesi.
2. [x] Ana pencere Windows'ta `decorations(false)` + gölge; Linux değişmez.
3. [x] `BaslikCubugu.svelte`: sol/orta/sağ bölümler, sürükleme, çift tıklama.

**Kabul:** Sözleşme testleri; Windows e2e'de küçült/büyüt/kapat; kapanma
akışı (oturum ve süreç temizliği) değişmez.
**Commit:** `uygulama: Kalfa'ya özel başlık çubuğu`.
**Birleşti:** #27, `90e729d`. Kapat düğmesinin etiketi
"Pencereyi kapat"tır: düz "Kapat" iletişim kutularının düğmesiyle çakışıyordu.

## Görev 4 — Sağ panel (tasarım §6)

**Dal:** `ui/sag-panel`. Görev 3'e bağlı.

1. [x] `calisma_alani_ozeti(oturum)`: `.git/HEAD`'den dal (git süreci yok).
2. [x] `calisma_alani_dizini` ve `calisma_alani_dosyasi`: kimlik tablosu,
   bağlantı izlenmez, alan dışı yok, gizli dosya içeriği dönmez, 256 KiB
   sınırı, dizin başına 2000 girdi. Yeni güvenlik sınırı: iki platformda
   sınır testleri.
3. [x] Panel: çubuk düğmeleri, Ortam / Değişiklikler / Dosyalar bölümleri,
   hatırlanan bölüm ve genişlik.

**Kabul:** Tasarım §6.4.
**Commit:** `uygulama: sağ panel ve salt okur dosya görünümü`.
**Birleşti:** #28, `032e9a3`.

Uygulamada plandan farklar:

- **Ayar deposu:** Son bölüm ve genişlik WebView deposunda (`localStorage`)
  hatırlanır; arayüzün başka ayar deposu yoktur.
- **Ortam toplamları:** Ortam'ın değişen dosyaları ve özetlenen araç çıktısı
  sayısı bütün kayıttan Rust'ta toplanır (`calisma_alani_ozeti`).
  Yüklenen geçmiş sayfasına bağlı değildir.
- **Git dalı:** Oturumun çalışma alanı tutamacıyla okunur. `gitdir:`'in
  gösterdiği HEAD ise `kalfa_tools::baglantisiz_oku` ile okunur: kökten
  tutamaçlarla, bağlantı izlemeden. UNC ve `\\?\` yolları açılmaz. Bu sınır
  güvenlik ve Codex incelemelerinden sonra eklendi.
- **Dosya görünümü tutamacı:** Görünüm, araçlara verilen çalışma alanı
  tutamacının aynısını kullanır.
- **Salt okur oturum:** Kayıttan salt okunur açılan oturumda Ortam ve
  Dosyalar kapalıdır.

Tasarımın plan sonrası tamamlanan üç ayrıntısı (`ui: masaüstü arayüzünün
kalan üç ayrıntısı`):

- [x] Araç kartında dosya adına tıklamak sağ paneli Değişiklikler'de o
  dosyanın son değişikliğinde açar (tasarım §4.3). Seçim
  `AnaDurum.degisiklikteGoster` ile gelir; Ortam'daki çıktı da aynı yolu
  kullanır. Dosya henüz listede yoksa (tur özeti gelmeden) panel ilk dosyada
  açılır.
- [x] Yapışık onay kartı öğe başına geri alınabilirliği sayfadaki metinle
  gösterir (§3.2); "Hayır" uyarı renginde.
- [x] Yanıtlanan onayın kayıt satırı "Onaylandı", "Reddedildi" ya da "Kısmen
  onaylandı" der (§3.2). Özet, arayüzün gönderdiği kararlardan çıkar
  (`OnayYanitlandi.karar`). Kayıttan açılan geçmişte onay satırı hiç yoktur
  (`GorunumOgesi` onay isteği taşımaz); bu yüzden özet yalnız canlı oturumda
  görünür.

## Doğrulama komutları

Her görevde AGENTS'taki varsayılan komutlar (workspace test, clippy, fmt)
iki platformda; dokunulan yere göre `kalfa-uygulama`/`kalfa` e2e özellikli
test ve clippy; arayüzde `npm run check`, `npm test`, `npm run build`.
Görev 2–4'te gerçek WebDriver e2e iki platformda.
