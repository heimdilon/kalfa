# 4c canlı değerlendirmesi: EVREN'in hız ve günlük token sınırları

**Tarih:** 2026-10-08
**Yapan:** Antigravity, kullanıcının izniyle (canlı EVREN istekleri; anahtar
`.env`'den süreç ortamına okundu, hiçbir yere yazılmadı). Şart ucu çağrılmadı.
**Amaç:** 4c'nin canlı değerlendirmesi (aşama 4 tasarımı §11.5, başarı ölçütü
§3.5) sırasında görülen 429'ları açıklamak; R19 ve G4 için veri.
**Ortam:** Windows 11, Rust 1.98.1, `crates/kalfa-rag/tests/degerlendirme.rs`.
Yoklamalar `curl` ile elle gönderildi (`curl -v` kullanılmadı; yalnızca yanıt
başlıkları ve gövdesi okundu). Betikler depoya girmedi.

---

## 1. Ne görüldü

1. İlk koşu (11:58–12:01) indeksin 18 dosyasını (16,7 MB, ~4 080 vektör,
   1024 boyut) yazdı, sonra 429 aldı. `kalfa-rag` beklendiği gibi indekslemeyi
   durdurup o ana kadarki indeksle döndü (§9.5, §11.2).
2. Sonraki iki koşuda (12:41 ve 12:53) her deneme hemen 429 aldı. Test,
   iletideki süreye göre 60 sn bekleyip yeniden denedi; altı beklemede indeks
   hiç ilerlemedi. Koşular elle durduruldu.
3. Elle yoklama, sınırın dakikalık değil **günlük** olduğunu gösterdi.

## 2. Yoklama

| İstek | Durum | Süre | Not |
|---|---|---|---|
| 1 kısa girdi | 200 | ~1 sn | `X-Ratelimit-Remaining-Tokens: 2499997` |
| 16 girdi × 2 000 karakter (16 695 token) | 200 | 1,1 sn | `Remaining-Tokens: 2483281` |
| 64 girdi × 2 000 karakter (tahmin 66 744 token) | 429 | 0,5 sn | `daily_token_limit_exceeded` |
| 256 girdi × 2 000 karakter (tahmin 267 157 token) | 429 | 1,2 sn | aynı |

Günlük sınırın yanıtı:

```
HTTP 429
Retry-After: 60
X-Ratelimit-Limit-Requests: 1200
X-Ratelimit-Remaining-Requests: 1198
X-Ratelimit-Reset: <bir sonraki dakika sınırı>

{"error":{"message":"Günlük token limitiniz aşıldı (tüm API anahtarlarınızın toplamı).",
  "type":"rate_limit_error","code":"daily_token_limit_exceeded",
  "evren":{"resets_at":"2026-10-09T00:00:00Z","estimated_prompt_tokens":66744,
           "remaining_tokens":17761,"limit_tokens":15000000}}}
```

## 3. Sonuçlar

- **İki ayrı sınır var.** Dakikalık: 2 500 000 token ve 1 200 istek
  (`X-Ratelimit-*`, `X-Ratelimit-Reset` Unix saniyesi). Günlük: 15 000 000
  token, bütün anahtarların toplamı; `resets_at`'te (UTC gece yarısı, yerel
  saatle 03:00) açılır.
- **Günlük sınırda da `Retry-After: 60` geliyor.** Yalnızca başlığa bakan
  istemci 60 sn sonra yeniden dener ve yine 429 alır. Asıl süre gövdedeki
  `evren.resets_at`'te. Gövdede kalan token (`remaining_tokens`) ve isteğin
  tahmini (`estimated_prompt_tokens`) de var: kalan tokenden büyük istek
  reddediliyor, küçüğü geçiyor (16 girdilik istek geçti, 64'lük geçmedi).
- **İstemci düzeltildi.** `kalfa-evren` artık `daily_token_limit_exceeded`'ı
  ayrı bir hata olarak döner (`EvrenError::GunlukSinir { sifirlanma }`):
  - Sohbet ve güvenlik denetçisi bunu beklemeden ve yeniden denemeden bildirir.
    Daha önce sohbet 60 sn bekleyip bir kez yeniden deniyordu, denetçi de öyle.
  - Eklenti `sinir-asildi("EVREN'in günlük token sınırına ulaşıldı …")` alır;
    `kalfa-rag` indekslemeyi durdurup kısmi sonuç verir (§9.5 değişmedi).
  - `resets_at` kullanıcıya gösterildiği için yalnızca zaman damgası
    karakterlerinden oluşuyorsa alınır (izin listesi).
- **Değerlendirme testi** günlük sınırda beklemeden, sıfırlanma anını söyleyip
  durur; yalnızca dakikalık sınırda bekler.
- **Token bütçesi.** Kalfa deposunun ilk indekslemesi için ~1 M token
  öngörülmüştü. Kısmi indeks bile ~4 080 parça tutuyor; günün 15 M'lik
  sınırının ne kadarının indekslemeden, ne kadarının aynı hesabın başka
  kullanımlarından (sohbet, öteki anahtarlar) geldiği ayrılamıyor. Sınır
  açılınca tam indekslemenin harcadığı token `usage.prompt_tokens`'tan
  toplanmalı.
- **G4 (sabit bekleme).** Bugünkü yoklamada 16 girdilik gömme 1,1 sn sürdü;
  ön ölçümdeki ~7–9 sn'lik sabit bekleme görülmedi. Tek ölçümdür; G4 açık
  kalır.

## 4. Değerlendirme sonucu (2026-10-09)

Günlük sınır açıldıktan sonra 13:05'te koşturuldu (Windows). Test, eklenti
isteklerinin `usage.prompt_tokens`'ını uca göre toplayan bir sarmalayıcıyla
koşar; sarmalayıcı şart kabulünü iletmez.

- **§3.5 geçti: 32 sorgunun 26'sında (%81) beklenen dosya ilk 5'te.**
- **İndeks:** 547 dosya, ~4 120 parça (16,9 MB vektör). Dünkü kısmi indeks
  kaldığı yerden sürdü: bugün yalnızca 2 gömme isteği (27 898 token) gerekti,
  7 sn sürdü, hız sınırına takılmadı. Dünkü bölümün tokeni sayılmadı; tam ilk
  indekslemenin maliyeti bu yüzden ölçülemedi.
- **Arama süresi:** her arama 1,7–2,0 sn (sorgu gömme + yeniden sıralama).
  Ön ölçümdeki ~7–9 sn'lik sabit bekleme bugün de görülmedi (G4, R19).
- **Arama maliyeti:** 33 aramada sorgu gömme ~1 150 token, yeniden sıralama
  1 296 320 token. **Her arama ~39 000 token**, neredeyse tamamı yeniden
  sıralamadan. Günlük 15 M sınırla bu, başka kullanım yoksa günde ~380 arama
  eder. Yeniden sıralanan aday sayısı ya da aday metin boyu bütçenin asıl
  düğmesidir (G5).
- **Düşen 6 sorgu:**

  | Sorgu | Beklenen |
  |---|---|
  | yarım kalmış araç çağrısı oturum yeniden açılınca nasıl işaretleniyor | `kalfa-core/src/kurtarma.rs` |
  | iki araç aynı dosyaya aynı anda yazmasın diye ne kullanılıyor | `kalfa-core/src/kilit.rs` |
  | Linux'ta yalıtılmış komut hangi sistem çağrılarını yapamıyor | `kalfa-sandbox/src/linux/seccomp.rs` |
  | komut çalışmadan önce çalışma alanındaki gizli dosyalar nasıl taranıyor | `kalfa-sandbox/src/tarama.rs` |
  | sistem tarayıcısında açılacak bağlantılar nasıl sınırlanıyor | `kalfa-uygulama/src/guvenlik.rs` ve iki dosya daha |
  | bir turda yapılan dosya değişiklikleri nasıl geri alınıyor | `kalfa-core/src/geri_alma.rs`, `…/oturum/geri_alma.rs` |

  Ortak örüntü: tasarım ve plan belgeleri ilk 5'i dolduruyor ve aynı dosya
  birkaç kez çıkıyor (örneğin `2026-09-25-kalfa-2b-yalitim.md` üç kez). Sonuçta
  dosya başına parça sınırı ya da tekilleştirme isabeti artırabilir (G5).

## 5. Açık kalanlar

- Tam ilk indekslemenin token maliyeti: indeks silinip yeniden kurulursa
  ölçülür (~4 120 parça; bir günlük sınırın önemli bir kısmını tutabilir).
- G5: dosya başına sonuç sınırı, yeniden sıralama aday sayısı, talimatın dili.
- Arayüz günlük sınırı şimdilik genel hata iletisi olarak gösterir; ayrı bir
  durum (örneğin durum çubuğunda sıfırlanma saati) 4d'de düşünülebilir.
- Tasarım §9.5'e günlük sınır eklendi.
