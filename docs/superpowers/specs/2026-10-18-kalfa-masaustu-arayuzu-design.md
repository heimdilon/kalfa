# Kalfa masaüstü arayüzü: onay kartı, araç diff'i, başlık çubuğu, sağ panel — Tasarım

**Tarih:** 2026-10-18.
**Durum:** r2, uygulandı: #25 (§4), #26 (§3), #27 (§5) ve #28 (§6) 18 Ekim'de
birleşti (main `032e9a3`). Uygulamadaki farklar planın Görev 4 notundadır.
r2: §4 yeni kayıt türü yerine mevcut `AracBasladi.yedekler`'i kullanır. Plan:
[2026-10-18-kalfa-masaustu-arayuzu.md](../plans/2026-10-18-kalfa-masaustu-arayuzu.md).
**Kaynak:** Kullanıcının 18 Ekim denemesi (CI kurucusu `bfe198a`, deneme
projesi `kalfa-deneme`). Açılır liste hatası ayrı PR'da (#23) düzeltildi.
**Taban:** `main`, `117dd8c`.
**İlgili:** [1c tasarımı](2026-09-22-kalfa-1c-design.md) §8.2 (onay),
[aşama 3 tasarımı](2026-09-28-kalfa-3-gorunurluk-design.md) §5.5, §5.11, §9.1
(tur özeti, geri alma, kısayol), [5a](2026-10-12-kalfa-5a-baglam-yonetimi-design.md)
(bağlam).

## 1. Amaç

Kullanıcı Kalfa'yı Claude Code ve Codex masaüstü uygulamalarına yakın bir
akışla kullanmak istiyor. Denemede dört eksik çıktı:

1. **Onay ayrı sayfa ister.** Konuşmadaki kartta yalnız "İncele" var; her
   komut için büyük onay sayfası açılıyor. Claude Code'daki gibi kartta
   doğrudan "Reddet / Bir kez izin ver / Bu oturumda izin ver" ve gerekirse
   "İncele" olmalı.
2. **Araç kartı neyin değiştiğini göstermiyor.** "Dosya düzenlendi —
   1 değişiklik yapıldı" yazıyor; kırmızı/yeşil diff yalnız tur özetinde ve
   sağ bölmede.
3. **Pencere çerçevesi Windows'un klasik çubuğu.** Kalfa'ya özel bir başlık
   çubuğu isteniyor.
4. **Sağ panel yalnız "Değişiklikler".** Claude Code'daki gibi üst çubuktan
   açılan bölümler isteniyor.

**Kabul ölçütü:** Kullanıcı deneme projesinde bir düzenleme ve bir komutu
onay sayfası açmadan konuşmadaki karttan onaylar; düzenlemenin diff'ini araç
kartında görür; pencereyi Kalfa'nın çubuğuyla küçültür, büyütür, kapatır ve
taşır; sağ paneli çubuktaki düğmelerle Ortam, Değişiklikler ve Dosyalar
arasında değiştirir. Güvenlik davranışı (mühür, riskli öğe, yerel onay,
yalıtım) değişmez.

## 2. Kararlar ve kapsam

Kullanıcının 18 Ekim kararları:

- Önce açılır liste hatası düzeltilir (PR #23); diğerleri bu tasarımla.
- Sağ panelin ilk sürümü: **Ortam/Bağlam/Çıktılar**, **Değişiklikler**,
  **Dosyalar**.
- **Kapsam dışı (sonraki aşama):** terminal ve web tarayıcı paneli, yan
  sohbet, arka plan görevleri, plan. Terminal yalıtımlı etkileşimli süreç
  (1.1a-1 altyapısı) ister; tarayıcı ayrı pencere ve CSP kuralları ister.
- MCP 1.1b bu işten sonra; açık başlatma onayı yine gerekir.

Değişmeyenler:

- Arayüz arka uca dosya yolu göndermez (AGENTS). Dosyalar paneli kimlikle
  çalışır (§6.3).
- Pencerelere Tauri'nin çekirdek izinleri verilmez. Başlık çubuğu kendi
  komutlarımızla çalışır (§5).
- CSP sıkıdır; yeni bileşenler sabit `style="…"` kullanmaz.

## 3. Konuşmadaki onay kartı

### 3.1 Bugün

`OnayKarti.svelte` öğe adlarını, açıklamaları ve uyarı sayısını gösterir;
tek düğmesi "İncele"dir. Karar `OnaySayfasi`'nda verilir: seçim, ret
gerekçesi, kısayol, mühürlü `OnayGirdisi`. Riskli öğeler (`riskliMi`: ağlı
komut, onaysız derleme dosyası değişikliğinden sonraki komut, görünmez
karakter) toplu onaylanamaz. `yerel_onay` taşıyan öğe ayrıca Rust'ın yerel
penceresinde sorulur.

### 3.2 Yeni kart

Bekleyen onay isteği, konuşmanın sonunda, yazma alanının hemen üstünde
**yapışık** bir kart olarak çıkar (Claude Code'daki gibi). Yanıtlanınca
yapışıklık kalkar; konuşmada küçük bir kayıt satırı kalır ("Onaylandı",
"Reddedildi", "Geçersiz").

Kartın içeriği öğe başına:

- **Dosya aracı:** dosya adı, `+eklenen −silinen` ve diff'in ilk 12 satırı;
  "Tamamını göster" kartın içinde açar.
- **Kabuk:** komut (tek aralıklı), dizin, yalıtım ve ağ rozetleri; ağlı
  komutta izinli hostlar.
- **MCP:** `sunucu · araç`, erişim rozeti (yalıtımlı / tam erişim).
- Uyarılar (sarı satır), "Bu değişiklik geri alınabilir" bilgisi.

Düğmeler:

| Durum | Düğmeler |
|---|---|
| Tek öğe, riskli değil | **Reddet** · **Bu oturumda izin ver** (yalnız `kisayol` teklifi varsa) · **Bir kez izin ver** (birincil) · İncele |
| Birden çok öğe, hiçbiri riskli değil | **Tümünü reddet** · **Tümünü onayla** (birincil) · İncele |
| En az bir riskli öğe | **Tümünü reddet** · **İncele** (birincil); riskli öğe kartta "tek tek seçilmeli" notu taşır |

"Bu oturumda izin ver"in altında kısayolun kapsamı küçük metinle yazar
(`kisayolCumlesi`).

### 3.3 Güvenlik sınırı

Kart yeni bir karar yolu açmaz; sayfanın kullandığı `kararlariUret`'i
kullanır:

- **Bir kez izin ver** = `secim: 'tumu'`, kısayolsuz.
- **Bu oturumda izin ver** = `secim: 'secilenler'`, öğe seçili, kısayol
  seçili. Sayfadaki kural aynen geçerli: kısayol yalnız teklifi olan öğede
  gider.
- **Reddet** = `secim: 'hicbiri'`, gerekçesiz. Gerekçe yazmak isteyen
  "İncele"yi kullanır.
- Kart, riskli öğe içeren isteği onaylayamaz (düğme yok); sayfa tek tek
  seçimi zorunlu tutmaya devam eder.
- `yerel_onay` taşıyan öğe karttan onaylansa da Rust'ın yerel penceresi
  açılır; arayüzün "kabul"üne güvenilmez (bugünkü gibi).
- Mühür (`muhur`) değişmeden taşınır. Bekleyen istek `Gecersiz` olunca
  düğmeler kaybolur.
- Kısayol tuşu yoktur: yanlışlıkla onayı önlemek için ilk sürümde karar
  yalnız tıklama ya da odaklanmış düğmede Enter/Boşluk ile verilir.

### 3.4 Testler

- `onay.ts`: kartın üç kararı `kararlariUret` ile aynı `OnayGirdisi`'yi üretir
  (mühür, kısayol, ret).
- Riskli öğede "Bir kez izin ver" ve "Tümünü onayla" yoktur.
- Kısayol teklifi olmayan öğede "Bu oturumda izin ver" yoktur.
- e2e: deneme düzenlemesi ve yalıtımlı komut karttan onaylanır; sayfa
  açılmaz. Ağlı komut yalnız "İncele" ile onaylanır.

## 4. Araç kartında diff

### 4.1 Bugün

Çekirdek her dosya yazma çağrısının diff'ini zaten hesaplar
(`arac_asamasi.rs`, `YazilanDosya.diff`) ama yalnız `TurOzeti`'ne koyar. Araç
kartı `AracSonucu.icerik`'i gösterir: modelin gördüğü kısa metin ("1
değişiklik yapıldı").

### 4.2 Yeni olay

Yazan çağrının "başladı" kaydı (`KayitOgesi::AracBasladi.yedekler`) dosya
başına yol, önceki hâl, `eklenen`, `silinen` ve diff'i zaten taşır. Diff
kayda girerken süzülmüştür: yalnız tek dosyalı yazmada, en çok `DIFF_SINIRI`
(64 KiB) ve gizli dosyada hiç yok. Yeni olay bu yedeklerden üretilir:

```rust
Olay::DosyaDegisiklikleri {
    cagri: CagriId,
    dosyalar: Vec<DosyaDegisikligi>,
}

pub struct DosyaDegisikligi {
    pub yol: CalismaYolu,      // çalışma alanına göreli
    pub islem: DosyaIslemi,    // Olusturuldu | Degisti
    pub eklenen: u32,
    pub silinen: u32,
    pub diff: Option<String>,  // kayıttaki süzülmüş diff
}
```

- Olay `AracCagrisiBasladi`'nın hemen ardından, `AracSonucu`'ndan önce
  gelir. Sonuç `Basarisiz` ya da `IptalEdildi` olursa kart diff'i
  "uygulanmadı" etiketiyle gösterir. Reddedilen çağrı başlamadığı için olay
  yoktur.
- Yeni kayıt türü gerekmez: geçmiş görünümü (`gecmis`) aynı bilgiyi
  `AracBasladi.yedekler`'den `GorunumOgesi::AracCagrisi.degisiklikler`'e
  koyar. Oturum yeniden açılınca diff görünür; yedeksiz eski kayıtta liste
  boştur ve kart bugünkü gibi davranır.
- `diff` yoksa (çok büyük, çok dosyalı ya da gizli) kart yalnız `+N −M`
  gösterir ve "Diff gösterilmiyor" der.
- Modelin gördüğü sonuç metni değişmez (`duzenle.rs`).
- Diff bugün de kayda ve `TurOzeti` ile arayüze gidiyor; yeni bir veri
  çıkışı yoktur (diff modele ya da EVREN'e gitmez).

### 4.3 Kart

`dosya_duzenle` / `dosya_yaz` kartı: başlıkta dosya adı ve `+N −M`; altında
`DiffView` ile en çok 20 satır, fazlası "Tamamını göster" ile açılır. Dosya
adına tıklamak sağ paneli Değişiklikler'de o dosyada açar.

### 4.4 Testler

- Çekirdek: başarılı düzenlemede olay sırası `AracCagrisiBasladi →
  DosyaDegisiklikleri → AracSonucu`; reddedilen çağrıda olay yok; gizli
  dosyada ve büyük diff'te `diff: None`.
- Geçmiş: kayıttan açılan oturumda `degisiklikler` dolu; yedeksiz eski kayıt
  okunur.
- Arayüz: kart diff'i gösterir, uygulanmayanı etiketler.

## 5. Kalfa'ya özel başlık çubuğu

### 5.1 Yerleşim

Ana pencere Windows'ta çerçevesiz açılır (`decorations(false)`, gölge açık).
36 px yükseklikte, uygulamanın renkleriyle bir çubuk:

- **Sol:** Kalfa simgesi, kenar çubuğunu aç/kapat.
- **Orta:** etkin oturumun başlığı ve proje adı (taşma kısalır).
- **Sağ:** sağ panel düğmeleri (§6.1), ayraç, pencere düğmeleri (küçült,
  büyüt/geri yükle, kapat). Kapat düğmesi kırmızı vurgulu.

Çubuğun boş yeri sürükleme alanıdır; çift tıklama büyütür/geri yükler.

### 5.2 İzinler

Tauri'nin `data-tauri-drag-region`'u çekirdek `core:window` izinleri ister;
AGENTS bunları pencerelere vermez. Bunun yerine `crates/kalfa`'ya dört komut
eklenir ve yalnız `ana-pencere` yeteneğine izin verilir:

- `pencere_kucult`, `pencere_buyut_degistir`, `pencere_kapat`,
  `pencere_surukle` (Rust tarafında `start_dragging()`).

Komutlar yalnız çağıran pencereye uygulanır; başka pencereyi hedef
almaz. Kapat, bugünkü kapanma akışını (oturum kapanışı, süreç
temizliği) aynen tetikler.

### 5.3 Sınırlar

- **Linux:** ilk sürümde yerel çerçeve kalır (pencere yöneticileri
  çerçevesiz pencerede yeniden boyutlandırma kenarı vermeyebilir).
- **Windows 11 yerleşim önerileri:** büyüt düğmesinin üstünde beklenince
  çıkan yerleşim menüsü özel düğmede çıkmaz. Win+Z ve pencereyi kenara
  sürükleme çalışır. Kabul edilen kayıp.
- Anahtar, şart ve yerel onay pencereleri yerel çerçevelerini korur.

### 5.4 Testler

- Komutların izin dosyaları ve `komut_adlari` sözleşme testi.
- e2e (Windows): küçült → pencere simge durumunda; büyüt → büyük; kapat →
  süreç kapanır. Sürükleme elle denenir.

## 6. Sağ panel

### 6.1 Çubuk düğmeleri

Başlık çubuğunun sağında üç düğme: **Ortam**, **Değişiklikler**,
**Dosyalar**. Düğme paneli o bölümde açar; açık bölümün düğmesine yeniden
basmak paneli kapatır. Bugünkü sağ bölme (sürüklenebilir tutamaç, %28–%50)
bu panelin kabı olur. Son açık bölüm ve genişlik arayüz ayarında hatırlanır.
Konuşmadaki "Değişiklikler" düğmesi aynı paneli açar.

### 6.2 Ortam

- **Ortam:** çalışma alanı adı ve kısaltılmış yolu (`~\…`), git dalı, izin
  kipi, yalıtım durumu, model ve efor.
  - Git dalı için yeni salt okur komut: `calisma_alani_ozeti(oturum)`. Rust,
    çalışma alanının `.git/HEAD`'ini okur (git süreci başlatmaz): dal adı,
    "ayrık HEAD" ya da "git deposu değil".
- **Bağlam:** son isteğin bağlam kullanımı, 5a'daki eşiğe oranla yüzde ve
  çubuk; özetleme olduysa sayısı.
- **Çıktılar:** bu oturumda değişen dosyalar (tur özetlerinin birleşimi);
  tıklayınca Değişiklikler'de açılır.

### 6.3 Dosyalar

Çalışma alanının salt okunur ağacı. Arayüz yol göndermez:

- `calisma_alani_dizini(oturum, dizin: Option<DosyaKimligi>)` → girdiler
  `{ kimlik, ad, tur: Dosya | Dizin | Baglanti, boyut }`. Kimlikler oturuma
  bağlı bir tabloda göreli yola eşlenir; oturum kapanınca silinir.
- `calisma_alani_dosyasi(oturum, kimlik)` → en çok 256 KiB metin; ikili
  dosyada "ikili dosya", gizli dosya kalıbına uyanda "gizli dosya; içerik
  gösterilmez", büyük dosyada ilk 256 KiB ve "kırpıldı".
- Sembolik bağlantı izlenmez; çalışma alanı dışına çıkan girdi gösterilmez.
- `.git` gizlenir; `node_modules`, `target` gibi büyük dizinler kapalı
  gelir ve açılınca listelenir (dizin başına en çok 2000 girdi).
- Düzenleme yok; dosyaya tıklamak salt okur önizleme açar.

Bu iki komut yeni bir **güvenlik sınırıdır**; testleri: çalışma alanı dışı
kimlik reddedilir, bağlantı izlenmez, gizli dosya içeriği dönmez, başka
oturumun kimliği geçersizdir.

### 6.4 Testler

- Rust: §6.2 ve §6.3 komutlarının sınır testleri (Windows ve Linux).
- Arayüz: panel bölümleri, düğmelerin aç/kapat davranışı, hatırlanan
  bölüm, Çıktılar'dan Değişiklikler'e geçiş.
- e2e: deneme projesinde Dosyalar'dan `gorevler.py` açılır; `.env` gizli
  gösterilir.

## 7. Uygulama sırası

Her biri ayrı PR ve iki platformda doğrulama:

1. **Araç diff'i** (§4): çekirdek olayı, kayıt, kart. En küçük ve diğerlerine
   bağımsız.
2. **Onay kartı** (§3): yalnız arayüz; `onay.ts` paylaşılır.
3. **Başlık çubuğu** (§5): Tauri komutları ve yetenek; Windows.
4. **Sağ panel** (§6): iki yeni salt okur komut ve panel; başlık çubuğuna
   bağlı (düğmeler orada).

Plan, bu belge onaylanınca `docs/superpowers/plans/`'a yazılır.

## 8. Açık sorular

Kullanıcının kararına bırakılanlar (varsayılanlar yukarıda):

- Onay kartında kısayol tuşu (ör. Ctrl+Enter = bir kez izin ver) olsun mu?
  Varsayılan: hayır.
- Linux'ta da özel başlık çubuğu istenir mi? Varsayılan: sonraki sürüm.
