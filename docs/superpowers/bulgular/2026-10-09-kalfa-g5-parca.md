# G5 — Parça penceresi kıyası

**Hazırlık:** 2026-10-09. **Canlı ölçüm:** 2026-10-11.
**Durum:** Kullanıcının devam ve hız sınırından devam onaylarıyla üç
pencerenin kıyası tamamlandı. Mevcut **60 satır/2400 karakter/10 satır
örtüşme** korunur; parça kıyası mevcut karar için kapandı, G5 bütünü açık.
Küçük pencere ilk beş parçada 24/32, mevcut ve büyük 26/32; büyük pencere
sıralama tokenını %81,84 artırdı. 9 Ekim koşusu kalan yaklaşık 1 milyon
günlük token nedeniyle ertelenmişti; o gün canlı çağrı yapılmadı.

## Yöntem ve kapsam

Mevcut indeks yalnız okunur. İzinli yollar, manifest okuma izinleri,
gizli/korumalı dosyalar ve bağlantılar önceki G5 ile aynı denetlenir.
Yeni pencereler eksik parçalardan kurulmaz: her dosyanın güncel 60 satır
ürün parçalarının **tamamı**, eski metadatanın adet/sıra/satır/bayt/SHA-256
kaydıyla eşleşmelidir. Yeni/eksik/değişmiş veya izin dışı dosya elenir.
547 indeks dosyasından **408 tam doğrulanmış dosya**, **5 696 855 UTF-8
bayt** sabitlenir; 139 dosya elenir. 32 sorgunun bütün beklenen dosya
etiketleri bu kümede karşılanır. Kaynaklar bütün kiplerde aynıdır.

Önceki boyut kıyasının 411 dosya/3555 tekil sağlam parçasından farklı
bir kümedir; üç dosyanın sağlam kalmış parçaları yeni pencerelere karıştırılmaz.
Önceki skorlar bu yeni veri kümesinin tabanı sayılmaz. Mevcut pencere için
de yeni gömme ve sıralama yapılır; eski deney makbuzları kullanılmaz.

| Kip | En çok satır | En çok karakter | Örtüşme satırı | Parça | Toplu istek | Gömülen UTF-8 bayt |
|---|---:|---:|---:|---:|---:|---:|
| Küçük | 30 | 1200 | 5 | 6940 | 109 | 7 065 157 |
| Mevcut | 60 | 2400 | 10 | 3484 | 55 | 6 997 724 |
| Büyük | 120 | 4800 | 20 | 1787 | 28 | 6 875 758 |

Satır/karakter üst sınırı ve örtüşme birlikte ölçeklenir; bu deney tek
bir karakter sınırının bağımsız etkisini ayırmaz. Kısa pencerede örtüşme
en çok yarısı, uzun satır UTF-8 karakter sınırından bölünür. Test kopyasının
60/2400/10 çıktısı her gerçek dosyada **ürünün kaynak modülüyle birebir**
karşılaştırılır. Ürün parçalama kodu/parametreleri değiştirilmez.

Her kip için bütün kaynak vektörleri yeniden alınır; aynı İngilizce
talimat, `qwen3-embedding-8b`, **1024 boyut/base64**. 32 yeni sorgu
vektörü üç kipte ortaktır. Her sorgu/kipte kosinüsle ilk **50 aday**,
`qwen3-reranker-8b`, tam `top_n=50`. Kip sırası sorgu başına döner.
İlk beş parça ve ilk beş farklı dosyanın isabeti ayrı ölçülür.
Gömme yanıtı `qwen3-embedding-8b`, sıralama yanıtı
`qwen3-vl-reranker-8b` yönlendirmesi bildirmelidir; eksik/farklı model,
eksik/tekrarlı sıra, boyut/sonluluk/sıfır vektör ve kullanım eksikliği
deneyi durdurur. İstekler en çok 64 kaynak parçası ve **1 MiB JSON** olur.

## Canlı onay tarifi

Hedef `https://evren-llmapi.ssyz.org.tr/v1`. **12 211 parça** üç ayrı
kipte en çok **192 kaynak toplu gömme + 32 sorgu gömme + 96 sıralama =
320 HTTP** isteği. Her sıralamada 50 doğrulanmış parça; yaklaşık
**10–13 milyon girdi tokenı**, **16 milyon raporlanan token bütçesi**.
Bu tahmin fiyat/hesap faturası değildir. Hata/hız/günlük sınır ilk çağrıda
durdurur; ilk tarifte otomatik tekrar yoktur. 11 Ekim'de kullanıcı hız
sınırından devamı ayrıca onayladı: önce toplam en çok 321 HTTP, ikinci
sınırdan sonra kalan 136 başarılı/3 başarısız istek için en çok üç koşu
ve toplam **325 HTTP**. Yalnız sağlayıcının en çok 60 saniyelik hız sınırı
beklemesinde devam; günlük kota, başka hata veya daha uzun bekleme durdurur.
16 milyon raporlanan bütçe ve kaynak/istek tarifi korunur. Başarısız
isteklerin tüketimi bilinmez; bütçe hesap faturası için kesin üst sınır değildir.
Ham `kapsam.http_enfazla=320` özgün tarifin tekil çağrı üst sınırıdır;
başarısız denemeler için ek HTTP izni bu bölümde ayrıca kayıtlıdır.

Tarif tam dosya/metadata/sorgu-etiket SHA-256'larını, üç pencereyi,
parça özetlerini ve bütün grup aralıklarını, modeller/talimat/boyut,
aday/top_n ve bütçeyi bağlar. Deney SHA-256:
`6ef1516986e9e2333e4f7329c6e0d5d11ca71713f41379ac15bf0f45df8b7792`.
Girdi SHA-256:
`c3e780abf44776a5d81f20bf44e0aedd4a7c552ea845e45ea063714ba5e4c465`.
Meta SHA-256:
`010bf6de9fab64ad5517136b652318729f4f49504bc50a4a99ea7e4fdec7cca6`.

`KALFA_G5_PARCA_DENEY_OZETI` tam eşleşmeden anahtar okunmaz/ağa çıkılmaz.
Makbuzlar `target/kalfa-g5-parca/<deney özeti>/`'nde ayrı tutulur; tam
kaynak metni yok, vektörler yalnız yerel makbuzda ve commit dışındadır.
Aynı deneyin aynı gövdesi makbuzdan kullanılır; eski makbuz tokenları
devam bütçesine sayılır, eski süreler yeni HTTP medyanına girmez.
Ham JSONL kaynak metni/vektör/anahtar/istek gövdesi içermez.
Mevcut indeksin 18 dosyası önce/sonra ad/boyut/SHA-256 ile denetlenir.
`.env` anahtarı yalnız kimlik doğrulamaya alınır; şart kabul yolu yoktur.

## Canlı sonuç (2026-10-11)

Windows'ta **10:23:58–10:51:54 TSİ**; dört koşu, üç hız sınırı.
İlk 143 kaynak gömme yanıtından sonra 9 sn; sonraki 41 kaynak yanıtından
sonra 60 sn; kalan kaynaklar ve ilk 14 sorgunun 41 sıralamasından sonra
60 sn bildirildi. İlk iki devam kullanıcı tarafından ayrıca onaylandı;
son sınırda onaylı yardımcı 61 sn bekledi. Son koşu 247 makbuzu okudu,
**73 yeni HTTP/2 383 732 token** ile tamamlandı. Tamamlanan kaynak gömme
istekleri yeniden gönderilmedi.

| Ölçüt | Küçük: 30/1200/5 | Mevcut: 60/2400/10 | Büyük: 120/4800/20 |
|---|---:|---:|---:|
| İlk 50 adayda beklenen dosya | 31/32 | 32/32 | 32/32 |
| Sıralanan ilk 5 parçada beklenen dosya | 24/32 (%75,00) | 26/32 (%81,25) | 26/32 (%81,25) |
| Sıralanan ilk 5 farklı dosyada beklenen dosya | 25/32 (%78,13) | 28/32 (%87,50) | 27/32 (%84,38) |
| Kaynak gömme tokenı | 2 270 583 | 2 244 222 | 2 203 434 |
| 32 sıralama tokenı | 699 260 | 1 247 370 | 2 268 179 |
| Sıralama HTTP medyanı | 654,15 ms | 1101,24 ms | 2041,18 ms |
| Native debug kosinüs medyanı | 99,85 ms | 50,12 ms | 25,77 ms |
| Ham `f32` vektör alanı | 28 426 240 bayt | 14 270 464 bayt | 7 319 552 bayt |

32 ortak sorgu gömmesi **1146 token**, HTTP medyanı **121,45 ms**.
HTTP medyanları bu deneyin dört koşusunda **ilk kez gerçekten yapılan**
başarılı çağrılardan hesaplanır: her kipte 32 yeni sıralama örneği.
Makbuz okuma yeni HTTP örneği sayılmaz; aynı süre iki kez kullanılmaz.
Son koşunun yalnız yeni çağrı medyanları farklıdır ve örnek sayısı daha
küçüktür (küçük/mevcut/büyük: 19/18/18); tablodaki 32 örnekle karıştırılmaz.
Kosinüs süreleri son koşunun 32 sorgusudur. Kaynak HTTP süreleri toplamı
küçük/mevcut/büyük için **89,15/74,47/68,36 sn**; beklemeler, makbuz
çözme/yazma ve yerel hesap bu toplama girmez. Bunlar tam ürün indeksleme
süresi veya wasm/host gecikmesi değildir.

Mevcut pencereye göre eşleşmiş sorgular (numaralar birden başlar):

| Kip/ölçüt | Kazanan sorgular | Kaybeden sorgular |
|---|---|---|
| Küçük, ilk 5 parça | 14, 22 | 2, 6, 8, 19 |
| Küçük, ilk 5 farklı dosya | 22 | 2, 3, 6, 8 |
| Küçük, ilk 50 aday | — | 2 |
| Büyük, ilk 5 parça | 14 | 2 |
| Büyük, ilk 5 farklı dosya | — | 2 |
| Büyük, ilk 50 aday | — | — |

İlk beş parçada kaçırılan sorgular küçükte **1, 2, 3, 6, 8, 11, 19, 27**;
mevcutta **1, 3, 11, 14, 22, 27**; büyükte **1, 2, 3, 11, 22, 27**.
İlk beş farklı dosyada küçük **1, 2, 3, 6, 8, 11, 27**, mevcut
**1, 11, 22, 27**, büyük **1, 2, 11, 22, 27** sorgularını kaçırır.
Beklenen satır aralığı ayrıca etiketlenmediğinden ölçüt dosya isabetidir.

**320 başarılı + 3 hız sınırı = 323 HTTP denemesi**, **10 934 194 bilinen
girdi tokenı**; onaylı 325 deneme/16 milyon raporlanan bütçenin altında.
192 kaynak, 32 sorgu yanıtı `qwen/qwen3-embedding-8b`; 96 sıralama yanıtı
`qwen/qwen3-vl-reranker-8b` bildirdi. Günlük sınır veya başka hata yok.
Üç başarısız isteğin tüketimi bilinmez. Her koşunun öncesi/sonrası ve
koşular arasında ürün indeksinin 18 dosyasının ad/boyut/SHA-256'sı aynı;
`urun_indeksine_yazildi=false`. 320 makbuz ayrı yerel dizinde korunur.

**Karar:** İngilizce talimat/50 aday/1024 boyut ve mevcut **60/2400/10**
korunur. Küçük pencere sıralama tokenını %43,94 azaltırken kaliteyi düşürdü
ve ham vektör alanını yaklaşık iki katına çıkardı. Büyük pencere ilk beş
parçada net kazanım getirmedi, farklı dosyada bir sorgu kaybetti; sıralama
tokenı %81,84, HTTP medyanı %85,35 arttı. Daha küçük ham alan ve kosinüs
maliyeti bu kalite/maliyet dengesinde ürün parametresini değiştirmiyor.
Ham alan metadata dahil disk boyutu veya tepe bellek değildir. Bu 32
sorguluk tek kontrollü küme, başka depolarda en iyi pencereyi kanıtlamaz.

Ham sonuç: [2026-10-11-kalfa-g5-parca-windows.jsonl](2026-10-11-kalfa-g5-parca-windows.jsonl),
**323 satır**, SHA-256
`df17ac47fc3bec99323bc0a04863c11b1de3442d50713c959d6f677317e5a031`.
192 kaynak/32 sorgu/96 sıralama ve girdiler/kapsam/bitti; son koşuda
247 makbuz satırı, 73 yeni çağrı. İlk çağrıların birleştirilmiş HTTP
kaydı: [2026-10-11-kalfa-g5-parca-http-windows.jsonl](2026-10-11-kalfa-g5-parca-http-windows.jsonl),
**320 satır**, SHA-256
`ca9541cfa1e8103695714989cb6df1bcd4a0f7e2a655295dd3b7b741f6f00e74`.
`olcum_kosu` ilk başarılı çağrının koşusunu verir; sürelerin 32 örnekli
hesabı buradan üretilir. Tüm istek özetleri/tüketim/süre/model tam ham
kayıtla eşleşir; her başarılı gönderim bir kez sayılır.
Kaynak metni/vektör/anahtar/istek gövdesi içermez. İlk üç kesilen koşunun
günlükleri, makbuzlar ve onaylı sınırlı devam yardımcısı yerelde saklanır.

## Çevrimdışı doğrulama

Alt modül `kalfa-rag/tests/g5_olcum/parca.rs`; yeni canlı test `#[ignore]`.
Altı yeni test: ürünle eşdeğerlik, UTF-8/kapsama/sınırlar, tam kaynak
eşleştirmesinde değişme/ekleme/eksik parçanın reddi, onay tarifinin
girdilere bağlanması, model/vektör/sıralama/makbuz/bütçe ve grup/JSON sınırı.
Ürünün 12 parçalama testi de aynı gerçek kaynak modülünden burada koşar;
G5 dosyası toplam **31 çevrimdışı test/4 atlanan deney** içerir.
İlk dar clippy kontrolü ve anahtarsız kuru koşu geçti.

Kaynak SHA-256:

- Ana test: `05c3b20e56a6e48c59b79982b3c3f45386cc0aa3e070ebf824928a8e8c9870cd`.
- Yeni alt modül: `72dea9f23f5af2a144aa969ef6b4e05797fdcb4c71a853d868c3f1b32cfe9962`.
- Değişmemiş ürün parçalama kaynağı: `e004e40b1c3a6b813d9f3557c4ad2d3516cad897e185e7f602378ac8898980e1`.

Rust 1.98.1 ve mevcut kilitli bağımlılıklarla çevrimdışı doğrulandı.
9 Ekim hazırlığında ve 11 Ekim sonuç kaydında tam kontroller yeniden geçti:

| Kontrol | Windows | WSL Ubuntu 24.04, `kalfatest` |
|---|---|---|
| Workspace testleri | 1025 geçti, 0 düştü, 18 atlandı | 967 geçti, 0 düştü, 18 atlandı |
| E2e özellikli uygulama/kabuk testleri | 114 geçti | 113 geçti |
| Düz/e2e/wasm clippy, `-D warnings` | Üçü temiz | Üçü temiz |
| `cargo fmt --all -- --check` | Temiz | Temiz |

Üç kaynak SHA-256'sı iki platformda aynı. Paket/bağımlılık indirilmedi.
WSL geçici kopyası kaldırıldı; ortak derleme target'ı korundu. Bu
e2e özellikli Rust testleri `tauri-driver` arayüz testleri değildir.
Gerçek yalıtım/uyumluluk ve arayüz yolları değişmedi; bu testler koşulmadı.
Deney kaynağı `1aa1ac4` için
[CI 140](https://github.com/heimdilon/kalfa-arsiv/actions/runs/37969507051)
dört işiyle yeşildir. 11 Ekim'de üç kaynak SHA-256'sı ve kuru koşunun
deney özeti yeniden aynı çıktı. Sonuç kaydı ürün/test kodunu değiştirmez;
yeni başın CI'ı ayrıca kontrol edilir.

Anahtarsız kuru koşu:

```powershell
$env:CARGO_NET_OFFLINE = 'true'
$env:KALFA_G5_KURU = '1'
cargo test -p kalfa-rag --test g5_olcum --locked --offline canli_parca_karsilastirmasi -- --ignored --nocapture
```

11 Ekim canlı koşuları bu tek testte, `KALFA_G5_KURU` kaldırılıp deney
özeti ve güvenli süreç kimlik doğrulamasıyla yapıldı. Atlanan takımın
bütünü çalıştırılmadı. Sonuç kaydı için anahtar gerekmez.
Bu native HTTP kontrollü alt küme kıyası tam/güncel indeksin wasm/host
kalite testi veya uçtan uca gecikmesi değildir. Üç kipin kaynak gömme
maliyeti yalnız bu alt kümeye aittir; tam ilk indeksleme maliyeti açık.
