# Kalfa 4c — Tam ilk indekslemenin maliyeti (2026-10-11)

**Soru:** Kalfa deposunun boş bir indeksten tam indekslenmesi kaç token tutar
(aşama 4 tasarımı §11.5; 4c planındaki açık iş)? 8–9 Ekim'deki indeks iki güne
yayıldığı için sayaç önceki bölümü saymamıştı.

**Kullanıcı onayı:** 11 Ekim'de "kalan kredim 4 milyon" diyerek onayladı. Bu
sayı EVREN'in günlük kalan tokenıdır (`X-Evren-Daily-Remaining-Tokens`);
`X-Evren-Credits-Remaining` 1060'ta sabit kaldı ve gömme isteklerinde
değişmedi. Bugünkü harcama bu bütçenin içindedir.

## Yöntem

- Düzenek `kalfa-rag/tests/degerlendirme.rs` (gerçek wasm/host yolu,
  `#[ignore]`). Yeni seçenekler:
  - `KALFA_RAG_DEPO`: ayrı ve boş indeks dizini
    (`target/kalfa-rag-ilk-indeks-20261011`). Mevcut canlı indeks
    (`target/kalfa-rag-degerlendirme`) değişmedi.
  - `KALFA_TOKEN_BUTCE`: bu koşuda sayılan `usage.prompt_tokens` sınıra
    ulaşınca yeni istek gönderilmez.
  - `KALFA_YALNIZ_INDEKS`: indeksleme bitince sorgulara geçilmez.
  - İndeksleme döngüsünde yeniden sıralama gönderilmez; her döngü turunda
    boşa ~39 000 token gidiyordu.
- Ürün ayarları: 1024 boyut, İngilizce talimat, 60/2400/10 parça penceresi,
  256 parça ya da 1 MiB'lık toplu istek.
- Kaynak: 4c dalının çalışma ağacı, manifestin `dosya-oku` globlarına uyan,
  256 KiB'tan küçük ve kilit dosyası olmayan **566 dosya, 6 588 976 bayt**.

## Sonuç

| | Değer |
|---|---:|
| Dosya | 566 |
| Parça | 4 056 |
| Parça metni (örtüşme dahil) | 8 096 537 bayt (dosya baytının 1,23 katı) |
| Diskteki indeks | 17 MB |
| Doğrudan ölçülen | 3 309 parça, 6 672 209 bayt, **2 157 582 token** |
| Ölçülen oran | 0,3234 token / parça baytı |
| Hesaplanan (aşağıya bakın) | 747 parça, 1 424 328 bayt, ~460 600 token |
| **Toplam ilk indeksleme** | **~2,62 M token** |

- Günlük 15 M'lik sınırın yaklaşık %17,5'i. 9 Ekim'de öngörülen ~1 M, iki
  buçuk katından fazla çıktı.
- **Hesaplanan kısım:** ilk koşu üç toplu istekten sonra durduruldu
  (121 dosya, 747 parça). Bu isteklerin tokenı sayaçta yok; ölçülen 3 309
  parçanın oranıyla hesaplandı. Oran parça türüne göre değişebilir; toplamın
  %18'i bu yüzden tahmindir.
- **Kalan 159 dosya** son koşuda 4 istekte, 370 732 tokenda ve 17 sn'de
  bitti. Bayt oranıyla yapılan tahmin 389 312 idi (%5 fazla).

## Bulgu: dakikalık pencerede takılma

Devam koşusunda 15 toplu istek geçtikten sonra (407/566 dosya, 1 786 850
token) **aynı 429 yaklaşık 30 dakika boyunca her dakika** geldi ve indeks hiç
ilerlemedi. Her döngü turunda yalnızca sorgu gömmesi (~21 token) geçti.

- Durdurduktan hemen sonra 3 tokenlık bir istekte
  `X-Ratelimit-Limit-Tokens: 2500000`, `X-Ratelimit-Remaining-Tokens: 112592`.
  Bir toplu istek ~90–180 bin token tutuyor; kalan pencere ondan küçüktü.
- 30 dakika sonra aynı başlık `2499997` gösterdi ve kalan 159 dosya ilk
  denemede geçti. Takılma sırasında günlük kalan değişmedi (reddedilen
  istekler günlükten düşülmedi).
- 429 gövdesi kaydedilmedi (`EvrenError::HizSiniri` gövdeyi taşımıyor).
  Pencerenin neden 30 dakika dolmadığı bilinmiyor: paylaşılan bir sınır,
  reddedilen isteklerin tahmini tokenı ya da kayan bir pencere olabilir.
- **Ürün etkisi:** pencere bir toplu isteğin altına düşerse `kalfa-rag`
  aynı büyüklükteki isteği yeniden dener ve ilerleyemez. Arama yine kosinüs
  sonucuyla ve "İndeks eksik" notuyla döner, ama indeks tamamlanmaz. Olası
  çözüm: art arda 429'da toplu isteği küçültmek. Bu 4c'nin kapsamı dışında
  bırakıldı; bir sonraki canlı koşuda 429 gövdesi kaydedilip karar verilecek.

## Kayıtlar

- İndeks: `target/kalfa-rag-ilk-indeks-20261011` (566 dosya, korunuyor).
  Kesinti öncesi anlık görüntüsü: `…-kesinti-oncesi` (`meta` ile 3 dilim).
- Günlükler oturumun geçici dizinindedir; sayılar yukarıdaki tablodadır.
- Bu ölçüm sorgu kalitesini ölçmez. 32 sorguluk tam/güncel kalite
  doğrulaması bu indeksle, günlük sınır sıfırlandıktan sonra yapılır
  (~1,25 M token; bugünkü ~2,62 M ile birlikte onaylı 4 M'nin içinde).
