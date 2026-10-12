# G5 — 1024/4096 boyut kıyası

**Tarih:** 2026-10-09. **Durum:** İki platformun tam kontrolleri ve
kullanıcının bu deneye özel açık onayını izleyen canlı koşu geçti.
1024/4096 karşılaştırması mevcut karar için kapandı; G5 bütünü açık.
Ürün **1024 boyutta** kalır: bu kümede 4096 hiçbir sorgunun isabetini
artırmadı, ham vektör alanı dört kat büyüdü.

## Yöntem

Mevcut indeks yalnız okunur. Güncel kaynağın bayt aralığı ve SHA-256'sıyla
eşleşen 411 dosya/3555 parça (7 177 104 UTF-8 bayt) bellekte sabitlenir.
Manifest/gizli/korumalı yol ve bağlantı denetimleri önceki G5'le aynıdır.
Önceki 3564 parçalık deneyden belge değişiklikleriyle 9 parça daha elendi;
bu yeni küme SHA-256'sı
`409234b0e4eab0516de51b416a16cd1f2e806a5fe01d1ec1d210e2e4707c8345`.
32 Türkçe sorgunun bütün beklenen dosyaları kümede vardır.

Model aynı `qwen3-embedding-8b`, talimat önceki ölçümün İngilizce talimatı;
iki kip 1024 ve 4096. [Modelin resmi kartı](https://huggingface.co/Qwen/Qwen3-Embedding-8B)
32–4096 çıktı boyutu ve MRL desteğini bildirir. EVREN uygulamasının
sayısal eşdeğerliği varsayılmaz: yeni 4096 vektörün ilk 1024 bileşeni,
her parçanın mevcut 1024 vektörüyle ayrı ayrı kosinüs ≥ **0,9999** vermeli.
32 sorgu da yeni 1024 ve 4096 API çağrılarıyla aynı denetimden geçer.
Uyumsuzlukta deney durur; sessizce yeni bir 1024 kaynak indeksi kurulmaz.
Bu tolerans model sürümünün kesin sabitliğini ispatlamaz; yakın sayısal
yönü doğrular ve kalan temsil farkını görünür kılar.

4096 kaynak vektörleri en çok 64 parça/1 MiB gövdeyle 56 toplu istekte
alınır; iki boyutun kosinüs hesabında vektörler normalize edilir. Her
sorguda dil/aday/parça boyutu değişmez; ilk 50 aday ve tam `top_n=50`
sıralama. İlk beş parça ve ilk beş farklı dosya ayrı değerlendirilir.
Sorgu/sıralama boyut sırası sorgu başına döner. Gömme yönlendirmesi
`qwen3-embedding-8b`, sıralama `qwen3-vl-reranker-8b` olmalıdır;
eksik/farklı yönlendirme kabul edilmez. Ürün kodu/WIT/bağımlılık değişmez.

## Onay kapsamı ve bütçe

Hedef `https://evren-llmapi.ssyz.org.tr/v1`.
En çok **56 kaynak toplu gömme + 64 sorgu gömme + 64 sıralama = 184 HTTP
isteği**. Kaynak gömmede 3555 doğrulanmış parça, sıralamada her istekte 50
aday; yaklaşık **4–6 milyon girdi tokenı**, **8 milyon raporlanan token
bütçesi**. Yaklaşık tüketim tahmindir; sınır/hata ilk çağrıda durdurur,
otomatik tekrar yoktur. Başarısız isteğin tüketimi bilinmez.
Mevcut indeks ve önceki deney makbuzları korunur.

Deney özeti kaynak kümesini, sorgu/etiketleri, iki boyutu, talimatı,
modelleri/yönlendirmeleri, 56 grup aralığını, ön ek eşiğini ve bütçeyi bağlar:
`f18fd66bbe5fc7961e39328f461859bced7cb64ea3fc109110aad724c20f39fa`.
Canlı koşu `KALFA_G5_BOYUT_DENEY_OZETI` eşleşmeden anahtar okumaz/ağa çıkmaz.
Makbuzlar `target/kalfa-g5-boyut/<deney özeti>/`'nde; tam kaynak metni yok,
vektörler yalnız bu yerel dizinde, commit'lenmez. Aynı gövde tekrar gönderilmez;
devam bütçesi eski makbuzları da sayar. Kaynak/anahtar/vektör/istek gövdesi
ham sonuç günlüğüne girmez. `.env` anahtarı yalnız kimlik doğrulamaya alınır;
şart kabul yolu ve paket indirme yoktur.

## Canlı sonuç

Windows 11 Home 10.0.26200, Ryzen 5 5600 (6 çekirdek/12 iş parçacığı),
Rust 1.98.1. Tek koşu, 9 Ekim **19:26:28–19:36:45 TSİ** (616,79 sn).
Gecikmeler 32 sorgunun medyanıdır; sağlayıcı yükü için garanti değildir.

| Ölçüt | 1024 | 4096 |
|---|---:|---:|
| İlk 50 adayda beklenen dosya | 32/32 | 32/32 |
| Yeniden sıralanan ilk 5 parçada beklenen dosya | 26/32 (%81,25) | 26/32 (%81,25) |
| Yeniden sıralanan ilk 5 farklı dosyada beklenen dosya | 28/32 (%87,50) | 28/32 (%87,50) |
| 32 sorgu gömme tokenı | 1 146 | 1 146 |
| 32 yeniden sıralama tokenı | 1 259 691 | 1 251 022 |
| Sorgu gömme HTTP medyanı | 110,14 ms | 127,21 ms |
| Yeniden sıralama HTTP medyanı | 1024,43 ms | 1017,21 ms |
| Native debug kosinüs medyanı | 53,42 ms | 209,08 ms |
| 3555 vektör için ham `f32` alanı | 14 561 280 bayt | 58 245 120 bayt |

İsabetlerin sorgu bazında karşılaştırması da eşittir: üç kalite ölçütünde
4096'nın kazandığı veya kaybettiği sorgu yok. İlk beş parçanın kaçırdığı
sorgular iki boyutta da **1, 3, 8, 11, 22, 27**; farklı dosya seçimi
3 (aynı dosyaya eşzamanlı yazma) ve 8'i (istek sınırı hatası) kurtarır.
Kalan dört sorgu **1, 11, 22, 27** (kurtarma, seccomp, dış bağlantı,
geri alma). Numaralar değerlendirme dosyasında birden başlar.
Bu eşitlik sıralamaların veya aday kümelerinin tümüyle aynı olduğu anlamına
gelmez. 4096'nın sıralama tüketimi 8669 token (%0,69) daha düşük; bu küçük
aday içerik farkı kalite artışı değildir. Daha büyük çıktının kaynak
gömme tokenı hakkında 1024 için yeni bir ölçüm yapılmadı.

56 kaynak gömme isteği **2 308 632 token** kullandı; HTTP sürelerinin
toplamı 95,59 sn. Bu değer toplam koşu süresi veya ürünün ilk indeksleme
süresi değildir; makbuz JSON çözme/yazma ve yerel hesap ayrıca vardır.
3555 kaynak çiftinin en düşük ön ek kosinüsü **0,9999056001**,
32 yeni sorgu çiftinin en düşüğü **0,9999349480**: hepsi 0,9999 eşiğini geçti.
120 gömme yanıtı `qwen/qwen3-embedding-8b`, 64 sıralama yanıtı
`qwen/qwen3-vl-reranker-8b` bildirdi; hepsi tarifteki yönlendirmeye uydu.

**184 yeni başarılı HTTP isteği, 4 821 637 raporlanan girdi tokenı.**
Hata, hız/günlük sınır, tekrar veya önceki makbuz kullanımı olmadı.
8 milyon bütçenin altında tamamlandı. Mevcut indeksin 18 dosyasının
adı/boyutu/SHA-256'sı önce ve sonra aynı; `urun_indeksine_yazildi=false`.
Yeni vektörler ayrı yerel deney dizininde tutulur.

**Karar:** İngilizce talimat/50 aday/**1024 boyut** korunur. 4096 ham
vektör alanını dört katına, bu native debug kosinüs hesabını 3,91 katına
çıkardı; kalite kazancı görülmedi. Ham alan yalnız `3555 × boyut × 4`
hesabıdır, ürün indeksinin metadata dahil disk boyutu veya konuk bellek
tepe değeri değildir. Native debug süreler wasm/host veya kullanıcıya
görünen uçtan uca gecikme değildir. Küçük, tek koşulu kontrollü alt küme
sonucu başka veri kümelerinde 4096'nın yararsız olduğunu kanıtlamaz.

Ham kayıt: [2026-10-09-kalfa-g5-boyut-windows.jsonl](2026-10-09-kalfa-g5-boyut-windows.jsonl),
**188 JSONL satırı**, SHA-256
`602dc8d6d70e24cff5e82d433229e83088f3d0dea4cd4ca7026137c1ef12f087`.
56 kaynak/64 sorgu/64 sıralama, girdiler/kapsam/ön ek denetimi/bitti.
Metin, vektör, anahtar ve istek gövdesi içermez. Aynı deneye ait yerel
makbuzların yanında Windows/WSL doğrulama günlükleri de saklanır.

## Doğrulama ve kalan işler

`kalfa-rag/tests/g5_olcum/boyut.rs` ana testin alt modülüdür; yeni canlı
test `#[ignore]`. Beş yeni test: boyut/ön ek, sonluluk/sıfır normu,
tam boyutlu kosinüs ve eşitlik sırası, toplu JSON/adet sınırı,
makbuz/model/sıralama/bütçe ve onay tarifinin girdilere bağlanması.
Toplam 13 çevrimdışı test ve anahtarsız kuru koşu Windows'ta geçti.
Rust 1.98.1, mevcut kilitli bağımlılıklarla çevrimdışı doğrulama:

| Kontrol | Windows | WSL Ubuntu 24.04, `kalfatest` |
|---|---|---|
| Workspace testleri | 1007 geçti, 0 düştü, 17 atlandı | 949 geçti, 0 düştü, 17 atlandı |
| E2e özellikli uygulama/kabuk testleri | 114 geçti | 113 geçti |
| Düz/e2e/wasm clippy, `-D warnings` | Üçü temiz | Üçü temiz |
| `cargo fmt --all -- --check` | Temiz | Temiz |

Kaynak SHA-256'ları iki platformda aynı:

- Ana test: `73db373bc20bceb6ba6248ffc5a71d96548f05e311c3bef7125bf7c5019a4f7d`.
- Boyut modülü: `3bf5c1710512ab5124997911fc6b1aabe8b4586730b2dca04126182ebf425097`.

Atlanan 17 testin tamamı canlı değildir; belge örneği/ölçüm de vardır.
WSL geçici kopyası kaldırıldı; ortak derleme target'ı korundu. Yalıtım/
arayüz yolları değişmedi; paket indirilmedi. Bu e2e özellikli Rust testleri,
`tauri-driver` arayüz uçtan uca testleri değildir.

Önceki talimat dili başı `0ffef46` için CI 138'in ilk Windows işi,
değişmemiş `iptali_yok_sayan_eklenti_iki_saniyede_birakilir` testinde
3,2284461 sn ölçtü; testin üst sınırı 2,9 sn. Yerel tam takımlarda ve
üç ayrı Windows tekrarında geçti; test veya güvenlik sınırı gevşetilmedi.
[CI 138](https://github.com/heimdilon/kalfa-arsiv/actions/runs/37954272157)
dört işiyle yeşil tamamlandı; Windows yeniden denemesinde geçti. Yeni
boyut commit'inin CI'ı da yerel doğrulamadan ayrı kontrol edilir.

Anahtarsız kuru koşu:

```powershell
$env:CARGO_NET_OFFLINE = 'true'
$env:KALFA_G5_KURU = '1'
cargo test -p kalfa-rag --test g5_olcum --locked --offline canli_boyut_karsilastirmasi -- --ignored --nocapture
```

Canlı koşu ancak bu yeni veri gönderimi/kredi harcaması açıkça onaylanınca,
`KALFA_G5_KURU` kaldırılıp deney özeti ve güvenli süreç kimlik doğrulamasıyla
aynı tek testte yapılır. Atlanan takımın bütünü çalıştırılmaz.
Bu native HTTP kontrollü alt küme deneyi, tam/güncel indeksin wasm/host
kalite testi ve uçtan uca gecikmesi değildir. 4096 kaynak gömme maliyeti
yalnız bu alt kümeye aittir; tam ilk indeksleme maliyetini kapatmaz.
Parça boyutu, tam/güncel kalite ve tam ilk indeksleme maliyeti açık kalır.
