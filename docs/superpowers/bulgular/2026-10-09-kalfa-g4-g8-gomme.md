# Kalfa G4/G8 — Canlı sorgu, toplu ve paralel gömme ölçümü

**Tarih:** 2026-10-09.
**Yapan:** Codex, kullanıcının bu ölçüme verdiği açık onayla.
**Ürün kodu:** `4448592d9ee88c92aa08483073934b0e14bd9db5`, PR #52.
**Durum:** Ölçüm ve v1 kararı tamamlandı. G4'teki sabit 7–9 sn varsayımı
güncel gözlemlere uymuyor. G8 için mevcut tek toplu istek korunur;
WIT 0.1'e `evren-cagir-coklu` eklenmez. Ürün kodu ve bağımlılıklar değişmedi.

## Sonuç ve karar

16 tekli sorguda HTTP/istemci süresi **medyan 117,75 ms**, aralık
**91,61–266,00 ms**. Aynı 256 parçayı bitirme süresi (kip başına üç tam örnek):

| Kip | Medyan sn | Aralık sn | Tam tur tokeni |
|---|---:|---:|---:|
| 1 × 256 sıralı | 5,855 | 5,851–6,067 | 193 012 |
| 4 × 64 sıralı | 6,361 | 6,288–6,600 | 193 012 |
| 16 × 16 sıralı | 7,596 | 7,592–7,674 | 193 012 |
| 2 × 128 paralel | 5,390 | 5,368–5,421 | 193 012 |
| 4 × 64 paralel | 4,893 | 4,860–4,904 | 193 012 |

Dört paralel istek tek toplu isteğe göre **0,963 sn / %16,4** daha kısa
sürer (yaklaşık 1,20 kat iş hacmi/süre). Küçük sıralı istekler daha yavaştır.
Yönlendirilen model bütün başarılı yanıtlarda `qwen/qwen3-embedding-8b`'dir.

Sorgu gömme, ilk bağlantıda da 7–9 sn beklemedi. 8 Ekim'deki tek/16 girdili
yaklaşık 1 sn ve 9 Ekim 13:05'teki uçtan uca 1,7–2,0 sn arama gözlemleri de
sabit bekleme varsayımıyla uyuşmuyordu
([önceki ölçüm](2026-10-08-kalfa-4c-evren-sinirlari.md)). Bunlar bir hizmet
süresi garantisi değildir; tüm günün ve sağlayıcı yükünün kontrolü yapılmadı.
Sorgu için mevcut tekli gömme yolu yeterlidir. R19, sağlayıcı gecikmesinin
değişebilmesi olarak tutulur; her aramanın en az 10 sn süreceği varsayımı kaldırılır.

Dört paralel isteğin kazancı bu düzende sınırlıdır; token maliyeti değişmez
ve birden fazla isteğin onay, iptal, kısmi başarısızlık ve depo sırasını
birlikte yönetmek gerekir. Mevcut 256 girdilik toplu istek v1 için korunur.
Bu **uygulama kararıdır**: gerçek wasm konuğunda yeni bir çoklu çağrı
uygulanıp ölçülmedi, bütün sağlayıcı yüklerinde paralelliğin gereksiz olduğu
kanıtlanmadı. Farklı bir iş yükünde karar yeniden ölçümle değiştirilebilir.

## Düzenek ve doğrulanan girdiler

- Windows 11 Home 10.0.26200, Ryzen 5 5600 (6 çekirdek / 12 mantıksal işlemci),
  Rust 1.98.1; test/dev profili. `kalfa-evren`in mevcut HTTP istemcisi ve
  SDK'nın 1024 boyutlu base64 gövde/çözme yardımcıları kullanıldı.
- Hedef `https://evren-llmapi.ssyz.org.tr/v1`; yalnızca `embeddings` çağrıldı.
  `.env` anahtarı süreç ortamında kullanıldı. Anahtar, bakiye, yanıt metni ve
  vektörler kaydedilmedi; şart kabul ucu çağrılmadı.
- Aynı **256 benzersiz gerçek parça**, toplam **550 295 karakter / 585 110
  UTF-8 bayt**. Eski indeksin eşleşen parça özetlerinden seçildi; sonrasında
  ölçüm indeksi okumaz. Dosya, LF'ye göre bayt aralığı ve SHA-256 kaydı
  `crates/kalfa-rag/tests/gomme-girdileri.json`'dadır. Metin saklanmaz.
- Kaynaklar sabit bir izin listesindeki core, sandbox, RAG kodu ve Eylül
  tasarım/plan belgeleridir. CRLF yalnızca LF'ye çevrilir. Her parçanın özeti
  ve benzersizliği tekrar doğrulanır; kaynak değişmişse ağ çağrısından önce durur.
- Girdi dizisinin JSON gösteriminin SHA-256'sı:
  `5f92b9e75fb1181f9d985860705d619981363d0809e8a36cd095db1421e027ab`.
  Windows ve WSL kuru koşularında aynı 256 girdi ve özet doğrulandı.
- Beş kip, üç tur: 1×256 sıralı, 4×64 sıralı, 16×16 sıralı,
  2×128 paralel, 4×64 paralel. Her turda kip sırası döner. Her kipten önce
  aynı gerçek Türkçe arama sorgusu tekli olarak gömülür.
- İstek gövdesi JSON kaçışları dahil en çok 1 MiB'dır. SDK her yanıtın
  vektör adedi/sırası, 1024 boyutu ve sonluluğunu doğrular. Bütün 256
  konumun tam bir kez dönmesi kontrol edilir. Toplu/sıralı/paralel
  vektörler referans çağrıya göre kosinüs ≥0,9999 ile karşılaştırılır.
  Kosinüsün en küçük gözlemi ham kayıttadır; bit eşitliği iddia edilmez.
- Süre `Instant` ile ölçülen geçen süredir. Toplu kip süresi görevleri
  başlatma, HTTP, yerel JSON ve SDK çözme dahil bütün istekler bitene kadardır.
  Girdi hazırlığı, referans karşılaştırması ve kipler arasındaki bekleme
  buna girmez. Tekli sorgu satırındaki `istek_ms`, HTTP ve istemcinin JSON
  çözmesini içerir; SDK çözme ayrıca `coz_ms`'dedir.
- Bir istemci/bağlantı havuzu kullanılır; ilk tekli sorgu soğuk bağlantıyı
  içerir. Aynı sorgu ve belgeler tekrar edilir; sunucu önbelleği bilinmiyor.
  Derleme ve tam testler canlı zamanlama aralığında çalıştırılmadı.

## Sınır, devam ve kullanım hesabı

İlk koşu 12 kipi tamamladı. Üçüncü turun dört paralel 64'lük çağrısında
üç yanıt geldi, biri `HizSiniri` / 60 sn aldı. Otomatik tekrar yapılmadı;
yarım kip ve başarılı üç isteğin kullanımı da kaydedildi. Bu sonuçlar
tam kiplerin medyanına katılmadı. Test başarısız çıkışla durdu; bu,
ürünün varsayılan test takımının düşüşü değildir.

Çevrimdışı kontrollerden sonra aynı izin ve bütçeyle son üç kip yeniden
çalıştırıldı; hepsi tamamlandı. Böylece beş kipin her birinde üç tam örnek
var. İlk koşu UTC 13:12:54–13:14:13, devam UTC 13:26:51–13:27:14
(Türkiye'de 16:12–16:27). Devamda önceki 12 kip tekrarlanmadı. Yarım kalan
kipin üç başarılı isteği yalnızca kullanım hesabına girer. En küçük
karşılaştırma kosinüsü **0,9999362**; bütün tam yanıtlar sayı/sıra/boyut
denetimlerinden geçti.

| Bölüm | Başarılı istek | `usage.prompt_tokens` |
|---|---:|---:|
| 15 tam kip | 81 | 2 895 180 |
| Yarım kipte gelen üç yanıt | 3 | 151 984 |
| 16 tekli sorgu | 16 | 656 |
| **Toplam** | **100** | **3 047 820** |

Toplam 101 HTTP denemesi: 100 başarılı, 1 hız sınırı. Başarısız ilk test ve
başarılı devam ayrı tutulur; reddedilen isteğin kullanımı toplama eklenmez.

Bu sayılar başarılı yanıtlardaki `usage.prompt_tokens` toplamıdır. Kesin
kredi harcaması/bakiye farkı ölçülmedi. Reddedilen istekte kullanım gelmedi;
hesabın diğer anahtarlarının kullanımı bilinmiyor. Günlük sınır görülmedi.
Hız sınırı, toplu ve paralel planın ortak bütçeyi de gözetmesi gerektiğini
gösterir; yalnızca çağrı sayısını çoğaltmak token bütçesini artırmaz.

## Yeniden üretim

`crates/kalfa-rag/tests/gomme_olcum.rs` varsayılan olarak `#[ignore]`'dır.
Anahtar depo kökündeki `.env`'den yalnızca süreç ortamına alınır; canlı
koşu kredi harcar. Aynı commit'in kaynakları ve Rust 1.98.1 gerekir.

```sh
# Ağ çağırmadan aynı parçaları doğrulamak:
KALFA_OLCUM_ORTAMI=linux KALFA_OLCUM_KURU=1 cargo test -p kalfa-rag --test gomme_olcum canli_gomme_toplu_ve_paralel_olcumu --locked --offline -- --ignored --nocapture

# Canlı: EVREN_API_KEY / EVREN_BASE_URL süreç ortamına alınmış olmalı.
KALFA_OLCUM_ORTAMI=linux cargo test -p kalfa-rag --test gomme_olcum canli_gomme_toplu_ve_paralel_olcumu --locked --offline -- --ignored --nocapture
```

PowerShell'de bu değişkenler `$env:KALFA_OLCUM_ORTAMI='windows'` ve
`$env:KALFA_OLCUM_KURU='1'` ile verilir. Canlı koşudan önce kuru değişken
kaldırılır. `KALFA_OLCUM_BASLANGIC=12` yalnızca son üç kipi çalıştırır;
izin verilen aralık 0–14'tür. Bu devam, kısmi yanıtları tam kipmiş gibi
birleştirmez; yeni referans vektörleri o süreçteki ilk tam kipten gelir.
Yeni kod kipler arasında süreye katılmayan 2 sn ara verir. İlk koşuda bu
ara yoktu; ilk 12 kipin süre ölçen kodu aynıydı. İlk ölçüm kaynağının
SHA-256'sı `a2133fd0ccb3bae6e9a4640b7f8d725243eb3d7b9667548e7f342bd72045a281`.

Ham JSONL: [ilk koşu ve hız sınırı](2026-10-09-kalfa-g4-g8-windows-ilk.jsonl),
[son üç kip](2026-10-09-kalfa-g4-g8-windows-devam.jsonl).

## Depo doğrulaması ve kalan işler

| Kontrol | Windows | WSL2 Ubuntu 24.04, `kalfatest` |
|---|---|---|
| Tam workspace testleri | 994 geçti, 0 düştü, 14 atlandı | 936 geçti, 0 düştü, 14 atlandı |
| E2e özellikli testler | 114 geçti | 113 geçti |
| Düz, e2e ve wasm clippy; fmt | Temiz | Temiz |
| Girdi kuru koşusu | 256 parça doğrulandı | Aynı 256 parça ve özet doğrulandı |

Yeni canlı ölçüm testi varsayılan olarak atlandığı için atlanan test
sayısı 13'ten 14'e çıktı; önceki testler devre dışı bırakılmadı. Test ve
girdi dosyalarının doğrulanan SHA-256 değerleri sırasıyla
`a2a99187b8a1f56858ffcd9d14f9463c79cbfa66c3e43ed5e0a63a21668f15f0` ve
`6a2ffeb2673be3d53a6b29273e2251990e38770255cea05d88a8ab895aab8c87`.

Canlı performans ölçümü Windows'tadır; WSL'de ağ çağırmadan girdi ve tam
test/derleme doğrulaması yapıldı. Aynı kaynak ve girdi dosyalarının SHA-256
değerleri iki platformda eşleşti. WSL geçici kaynak kopyası kaldırıldı;
mevcut paylaşılan target ve canlı indeks korundu.

G5 kalite, talimat dili, 1024/4096 boyut, parça/aday sayısı ve dosya
çeşitliliği; tam ilk indeksleme token maliyeti açık kalır. Bu 256 parçanın
kullanımı tüm deponun ilk indeksleme maliyeti gibi sunulmaz. Rerank ve
uçtan uca arama bu ölçüme katılmadı; önceki 26/32 değerlendirme sonucu
yeni bir kalite sonucu olarak tekrarlanmadı.
