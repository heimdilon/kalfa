# Kalfa 5b — Oturum bütçesi: Tasarım

**Tarih:** 2026-10-12.
**Durum:** İnceleme taslağı; ürün uygulaması henüz yapılmadı.
**Taban:** `main`, `3133248` (5a tamamlandı).
**Üst belge:** [Kalfa tasarımı](2026-09-21-kalfa-design.md) §6.8.
**Plan:** [5b uygulama planı](../plans/2026-10-12-kalfa-5b-oturum-butcesi.md).

## 1. Amaç ve kabul ölçütü

Bir oturumun bütün üretim istekleri aynı istek/token bütçesini tüketir.
Ana model, güvenlik denetçisi, bağlam özetleyicisi ve eklentilerin gömme/
sıralama istekleri dahildir. Limit dolunca veya tüketim bilinmediğinde
yeni istek kullanıcı kararından önce başlamaz. Başlamış istekler sonuçlanır
ve kayda girer; araç yan etkileri otomatik tekrarlanmaz.

Başarı: dört çağrı yolunun sahte sağlayıcıyla ölçülen gerçek deneme sayısı
bütçe kaydıyla aynıdır; kurtarma limiti sıfırlamaz; eksik kullanım yeni
isteği keser; eşzamanlı talepler istek sınırını aşamaz. Kullanıcı durabilir
veya sonlu mutlak limitlerle devam edebilir.

Kredi bakiyesinden oturum maliyeti çıkarılmaz. Token bütçesi, yanıt geldikten
sonra bilinen tüketimin sınırıdır; önceden kesin bir token/kredi harcama
tavanı değildir. Son istek ve zaten başlamış paralel istekler token sınırını
aşabilir. Bu ayrım kullanıcıya gösterilir.

## 2. Kararlar ve kapsam

**Kullanıcı kararı (12 Ekim, bu çalışmanın sohbeti):** EVREN bir isteğin
token kullanımını bildirmezse yeni istekten önce durup sorulur. Eksik
kullanım sıfır sayılmaz. Aynı karar, kullanım vermeyen başarısız deneme,
iptal ve zaman aşımı için de geçerlidir; 429 tekrarından önce gerekirse
kullanıcı kararı beklenir.

**Plan önerileri; henüz kullanıcıyla kesinleşmedi:**

- Yeni oturum için 100 istek ve 1 000 000 bilinen token. Bunlar ölçülmüş
  optimum değerler değildir; korumacı başlangıç değerleridir. Büyük ilk
  RAG indekslemesi bütçe artırmayı gerektirebilir (5a öncesi tam indeks
  yaklaşık 2,62 milyon token tüketti).
- Limitler yalnız pozitif, sonlu tamsayıdır; 0, sınırsız anlamına gelmez.
  Arayüz girdileri JavaScript'in güvenli tamsayı aralığında doğrulanır.
- Ayarlar'daki varsayılanlar yeni oturumlara uygulanır. Açık/kurtarılan
  oturum kendi kaydedilmiş limitlerini kullanır.

Şart durumu/metni ve model kataloğu üretim bütçesinde değildir. Şart kabul
akışı değiştirilmez; ajan kullanıcı adına kabul çağrısı yapmaz. Ağlı kabuk
trafiği bu sayaçta değildir; mevcut host izin listesi ve yalıtım sınırları
sürer. Denetçi kipleri, eklenti başına 5 000 istek/256 MiB ve araç başına
sınırlar korunur. Bağlam eşiklerinin ayar ekranı, parasal bütçe, tokenizer,
otomatik bütçe yenileme ve sınırsız devam bu işin dışındadır.

## 3. Mevcut kodda çağrı ve kayıt noktaları

| Yol | Ağ denemesi | Bugünkü kayıt / 5b gereği |
|---|---|---|
| Ana | `oturum/model_turu.rs::akis_ac` → `Saglayici::akis` | `Istek` + `Kullanim`; akış ömrünün tamamı bir denemeye bağlanır |
| Denetçi | `denetci.rs::sohbet_ve_yeniden_dene` | İçteki 429 tekrarı bugün tek sonuca indirgenir; her deneme ayrı izin/kayıt alır |
| Özetleme | `oturum/baglam.rs::ozetle` | İçteki 429 tekrarı da ayrı sayılır; en çok dört başladıktan sonra durma/boşaltma korunur |
| Eklenti | `kalfa-plugin/src/host.rs::evren_cagir` → `EvrenBaglami::ek_istegi` | `EklentiIstegi` var, token yok; ham yanıttan yalnız kullanım meta verisi çekirdeğe taşınır |

`kalfa-evren::butce::YanitButcesi` tek yanıtın bellek/bayt sınırıdır; oturum
bütçesinin yerine kullanılmaz. `GunlukToken::toplam()` istem + tamamlamadır;
düşünce tokenları tamamlamanın dökümüdür, ikinci kez eklenmez.

`OturumKaydi::yaz_kalici` yazma + `sync_data` yapar. Yeni başlatma izinleri
ve devam kararları bu yoldan kalıcılaştırılır; kayıt hatasında ağa çıkılmaz.
Kayıtsız çekirdek oturumu aynı bellekteki kapıyı kullanır, kurtarma garantisi
taşımaz. Kayıt dosyasına tek yazıcı yine `Oturum` görevidir.

## 4. Tek sahipli bütçe kapısı

Yeni `kalfa-core/src/butce.rs` saf durum geçişlerini ve oturuma bağlı
`ButceGonderici` kanalını içerir. Sayaçların tek sahibi `Oturum`'dur.
Sağlayıcıları uygulama genelinde saran bir bütçe eklenmez; iki oturum
birbirinin limitini tüketmez.

- Ana istek, ağdan hemen önce çekirdeğin izin yardımcısını çağırır.
- Denetçi ve özetleme yardımcıları her `sohbet` denemesinden önce oturuma
  bağlı kanaldan izin ister. Denetçi trait'ine bu bağlam eklenir; boş
  denetçi ağ çağrısı üretmediği için izin tüketmez.
- Eklentide mevcut izin/önizleme bittikten sonra bütçe izni istenir.
  `AracOrtami` o oturumun göndericisini taşır; `EvrenBaglami`'nın uygulama
  genelindeki katalog önbelleği oturum sayacını tutmaz.

Kanalın talebi `Baslat { amac, gonderim_baglami, yanit }`'dir. Başlatma
cevabı `IslemId` ve sahipli `ButceIzni` verir. Çekirdek her izin için bir
`oneshot` sonuç alıcısını bekler. İzin kopyalanamaz; açıkça bitiş bildirir
veya bırakılınca son bilinen kullanım/meta/sonucu gönderir. İzin teslim
edilemeden alıcı kapanırsa sonuç alıcısı kapanmayı bilinmeyen olarak
işler. Asenkron `Drop`, sınırlı kuyruğa `try_send` ile kaybolabilen bitiş
ve ateşlenip unutulan kayıt görevi yoktur. Ağ
denemesinin gönderim özeti/model/meta/sonucu mevcut günlük yoluyla aynı
kimliğe bağlanır (§6). Bir `denetle` ya da `ozetle` çağrısı iki HTTP denemesi
yaparsa iki kimlik alır. Düşürülmüş bir bekleme future'ı izin tüketmez;
izin verildikten sonraki iptal/bağlantı hatası deneme sayısını geri almaz.

Kabul sırası: mevcut izinler/şart/önizleme → sayaç kontrolü → başlangıç
kaydının kalıcı yazımı → sayaç geçişi → izin cevabı → sağlayıcı çağrısı.
Kayıt yazımı başarısızsa cevap izin değildir. Başlatma kaydından sonra
çökme veya kanal kapanması olursa kayıt muhafazakâr biçimde sayılır.

## 5. Sayaçlar ve eşzamanlılık

`ButceSinirlari { istek, token }`; `ButceDurumu` şu bilgileri tutar:
başlatma izni verilen denemeler, bilinen tokenlar, kullanım bilinmeyen
tamamlanmış denemeler, kullanıcının kabul ettiği bilinmeyen sayısı ve
aktif istek kimlikleri. Toplamlar oturum boyunca azalmaz.

Yeni izin ancak istek sayısı limitin altında, bilinen token limitin altında
ve yeni bilinmeyen tüketim yoksa verilir. İstek kontrolü ve artırımı tek
durum geçişidir. Dört paralel talebin her biri aynı eski sayacı okuyamaz.

Aktif denemeler yeni izin kapısı kapansa da tamamlanıp tek kez kaydedilir.
Bir sonucun iki kez gelmesi veya başlamamış kimlikle gelmesi kabul edilmez;
kurtarmada bozuk kayıt hatasıdır. Toplam taşması limit kapısını açamaz:
token toplamı `u64::MAX`'a doyar ve bütün geçerli limitler için kapalı kalır.

Kullanım bilinmeyen sayısı ve bilinen toplam ayrı tutulur. Kullanıcı
bilinmeyen tüketimi kabul edip devam ettiğinde geçmiş sıfırlanmaz, sıfır
token üretilmez; yalnız kabul edilen bilinmeyen sayısı ilerler. Daha sonra
başka bir kullanım eksikliği yeni karar gerektirir.

## 6. Token ve gönderim kaydı

`Istek` ve `EklentiIstegi`'ne geriye uyumlu, isteğe bağlı `butce_islem`
alanı eklenir. `GonderimYaniti::Gonder` izin kimliğini taşır;
mevcut ortam bitişi bu sahipli izin üzerinden aynı kimliği ve
`Option<GunlukToken>` kullanımını iletir; iki ayrı bitiş göndericisi
kullanılmaz. WIT ve eklentinin aldığı ham yanıt değişmez. Bu sahipli değer
eklendiği için iç `GonderimYaniti`'nin `Clone/Eq` türetimleri ve ilgili
test karşılaştırmaları uyarlanır; izin çoğaltılabilir yapılmaz.

Her ağ denemesi için mevcut `Istek`/`EklentiIstegi` satırı yazılır; denetçi/
özetleyicinin iç tekrarları gizlenmez. Son `Denetim` ve `BaglamOzeti`
mantıksal sonucu bir kez yazar. Eski sonuca-toplu `Istek` yazımı kaldırılır;
aynı deneme hem yardımcıda hem son-işleyicide yazılmaz.

Yeni bütçe kayıtları:

| Kayıt | Alanlar / anlam |
|---|---|
| `ButceAyarlandi` | `sinirlar`; yeni oturumun başlangıç limitleri |
| `ButceIstegiBasladi` | `islem`, `amac`; kalıcı başlangıç izni |
| `ButceIstegiBitti` | `islem`, `kullanim: Option<GunlukToken>`; tek sonlandırma |
| `ButceDevamKarari` | `islem`, `sinirlar`, `onaylanan_bilinmeyen`; kalıcı kullanıcı kararı |
| `ButceDevralindi` | Eski kaydın bilinen en az istek/token tabanı ve belirsizliği |

Yeni bütçe muhasebesinde yalnız bütçe kayıtları sayılır. Eski `Kullanim`
satırlarıyla yeniden toplanmaz. Gönderim günlüğü yeni satırların tokenını
`butce_islem` üzerinden iliştirir; eşzamanlı sonuçlar "son satıra" eklenmez.
Eski satırlarda mevcut çözüm korunur. Tur özeti de her denemenin kullanımını
bir kez toplar. Akışta yinelenen kümülatif kullanım toplam olarak eklenmez;
bir isteğin son/geçerli kullanımı tek sonlandırmada sayılır. Çelişkili
veya eksik kullanım bilinmeyendir.

Eklenti yanıtı sınırlı JSON'dan, yeni paket olmadan okunur. Gömme/sıralamada
sayısal `prompt_tokens` ve yoksa 0 tamamlamaya izin verilir; sohbet için
istem ve tamamlama bilgisi gerekir. Yalnız geçerli `total_tokens` bulunan
uç yanıtında toplam isteme yazılır, tamamlamaya 0. Alanlar bozuk, negatif,
taşmış, birbiriyle çelişkili veya boş `usage` ise `None` olur. `None`, 0
değildir. Mevcut `Usage` tipinin `serde(default)` ile boş alanları 0'a
çevirdiği gözetilir: boş/eksik kullanımın bütçede gerçek 0 sayılmaması için
istemci ayrıştırma noktalarında alan varlığı korunur (§plan Görev 2).

## 7. Bekleme ve kullanıcı kararı

Yeni izin kapalıysa talep saklanır; `Oturum` kanal/süreç/komut olaylarını
işlemeye devam eder. Başlamış ağ istekleri bitince tek
`Olay::ButceKarariGerekli { islem, durum }` gönderilir. Kullanıcı yanıtından
önce hiçbir yeni üretim isteği açılmaz. Hâlihazırda çalışan yerel araçların
bitmesi bütçe izni anlamına gelmez.

Eklenti bütçe beklemesinde aynı host çağrısının izin future'ı bekler.
Bütçe hatası eklentiye 429 gibi gönderilmez: RAG toplu sınırını yanlış
yere küçültmez ve bütün araç çağrısı yeniden yürütülmez. WASM'ın mevcut
süre/yakıt/iptal sınırları korunur. Araç beklerken zaman aşımına uğrarsa
bekleyen talep çıkarılır; daha sonra gelen karar onu yeniden başlatamaz.

Ana istek ve denetçi/özetleme beklemelerinde bütçe kanalının alıcısı ilgili
`select!` döngülerine eklenir. Kendi görevinden izin bekleyerek kilitlenme
oluşturulmaz. Denetçi/özetlemede bütçe bekleme süresi ağın 30 saniyelik
süresini tüketmez; etkin ağ/tekrar bekleme süresinin mevcut toplam sınırı
korunur. Eklenti EVREN zaman aşımı 120 saniyedir; değiştirilmez.

`Komut::ButceKarari { islem, karar }`:

- `DevamEt { sinirlar }`: istek ve token limitleri mevcut tüketimin
  üstünde olmalıdır. Kullanım eksikliği tek engelse aynı limitler yeterli
  olabilir. Karar ekranda gösterilen bilinmeyen tüketimi açıkça kabul eder.
  Sayaçları sıfırlamaz. Yeni koşullar doğrulanıp karar kalıcı yazıldıktan
  sonra aynı bekleyen iş sürer.
- `Durdur`: tur iptal edilir, bekleyen izinler iptal edilir; başlamış
  isteklerin sonuçları/belirsizliği korunur. Oturum silinmez.

Karar kimliği bekleyen kartla eşleşmeli; aktif ağ isteği varken, kapanmış
karttan veya farklı oturumdan gelen karar geçersizdir. Yeni bir belirsiz
sonuç kartın durumu değiştiyse eski karar kabul edilmez. Önizleme ve bütçe
kartlarının kimlikleri birbirine izin vermez. Bütçe devamı gizli dosya,
denetçi, eklenti ilk gönderim ya da yalıtım izni vermez.

## 8. Kurtarma

Yeni kayıtta son limit/karar ile kimlikli başlangıç ve sonuçlar oynatılır.
Sonuçsuz başlangıç bir istek ve bilinmeyen tüketim olarak kalır; yeniden
açılışta bunun için `ButceIstegiBitti { kullanim: None }` kalıcı yazılır.
Bu kayıt tamamlanmadan yeni izin veya devam kararı verilmez. Sonraki
istekten önce sorulur. Yeni oturum açıp eski kaydı devam ettirmek sayacı
yenilemez. Bilinmeyenin kabulü de kayıttan geri gelir.

5b öncesi kayıt açılır. Mevcut `Istek` ve gönderilmiş `EklentiIstegi`
satırlarından bilinen en az deneme/token sayısı çıkarılır. `Gonderilmedi`
sayılmaz. Eski denetçi/özet tekrarları ve eklenti kullanımı tam
belirlenemediği için geçmiş **tam sayılmış** gösterilmez; ilk yeni
istekten önce bu belirsizlik kullanıcıya sunulur. `ButceDevralindi`
geçişi bir kez yazılır, tekrar kurtarmada eski ve yeni sayaçlar çift
toplanmaz. Bozuk kimlik/çift sonuç/karar geri çekilmesi hata verir.

Şema 1'de kalır; yeni sürümün eski kaydı açması test edilir. Eski Kalfa
sürümünün yeni enum varyantlarını okuyabildiği iddia edilmez.

## 9. Arayüz ve ayarlar

Sohbette oturumun istek ve bilinen token toplamı/limiti gösterilir.
Kullanımı bilinmeyen denemeler varsa ayrıca gösterilir; "kesin toplam"
yazılmaz. Bütçe kartında neden, tüketim, eksik kullanım ve iki sonlu limit
girdisi bulunur. İşlemler: "Bütçeyle devam et" ve "Turu durdur".
Eksik tüketimi kabul etme anlamı devam düğmesinin yanında açıklanır.

`ayarlar.json`'a `oturum_butcesi` alanı `serde(default)` ile eklenir.
Bozuk/sıfır değer önceki ayarları veya yeni oturumların güvenli
varsayılanlarını kullanır; açık oturumun kaydedilmiş limiti değiştirilmez.
Rust arka uç ve TypeScript girdileri doğrular. Arayüzden dosya yolu
alınmaz, CSP/izin listeleri genişletilmez, metinler `ui/src/dil/tr.ts`'tedir.

Kurtarılan oturum başlangıç görünümü bütçe durumunu da taşır; pencereyi
kapatıp açmak veya olay akışını yeniden bağlamak sayaç görünümünü silmez.
Yeni IPC komutları yalnız ana pencerenin mevcut sözleşmesine eklenir.

## 10. Ön doğrulama ve uygulama sınırı

[Bağımlılıksız prototip](../../../prototip/oturum-butcesi/README.md):
Windows ve WSL Ubuntu 24.04 (`kalfatest`), Rust 1.98.1; **11/11 test**.
Sekiz paralel talep/dört izin, eksik tüketimde yeni istek reddi, başlamış
sonuçların işlenmesi, eski karar, kurtarma, çift sonuç, izin alıcısının
kapanması ve taşma sınandı.

Bu sonuç sayaç/kabul modelinindir. Asenkron kanal, kalıcı JSONL, gerçek
sağlayıcı, eski kayıt göçü, Tauri ve arayüz entegrasyonu uygulanmış veya
derlenmiş sayılmaz. Plan bunları ayrı testlerle zorunlu kılar. Canlı EVREN
ölçümü yapılmadı; önerilen varsayılanlar ürün ölçümü değildir.
