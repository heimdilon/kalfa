# Kalfa — dosya tekilleştirmeli G5 ve 20 aday (2026-10-14)

**Soru:** Sonuçların dosyaya göre tekilleştirilmesi
([#13](https://github.com/heimdilon/kalfa/pull/13)) ürüne girdikten sonra,
gerçek wasm/host yolunda 32 Türkçe sorgunun kaçında beklenen dosya ilk
beşte? Kosinüs aday sayısı 50'den 20'ye inerse kalite korunur mu?

**Onay:** Kullanıcının 14 Ekim'deki onayı ("onay gerektiren işleri yapmaya
başla"): canlı G5 doğrulaması ve 20 aday ölçümü. Şart kabul ucu çağrılmadı;
anahtar loglara yazılmadı (loglarda anahtar taraması: 0 eşleşme).

## Yöntem

- Düzenek `kalfa-rag/tests/degerlendirme.rs` (`#[ignore]`), `main`
  `0eab672` ve aşağıdaki düzenek düzeltmesi. Windows 11, ana klasör.
- İndeks: 12 Ekim'in tam indeksinin kopyası
  (`target/kalfa-rag-g5-20261014`). Her sorgu 20 sonuç ister; sonuçlar
  dosyaya göre tekil olduğundan ilk beş sonuç ilk beş farklı dosyadır.
- 50 aday ürün ayarıdır. 20 aday için `arama.rs`'teki `ADAY` yalnız bu koşu
  için yerelde 20 yapıldı, sonra geri alındı (commit'lenmedi).
- `KALFA_TOKEN_BUTCE`: 50 aday için 1 400 000, 20 aday için 600 000.

## Sonuç

| | 50 aday (ürün) | 20 aday |
|---|---:|---:|
| İlk beşte isabet | **28/32 (%88)** | 22/28; bütçe 28. sorgudan sonra doldu |
| Aynı ilk 28 sorguda isabet | 24/28 | 22/28 |
| Yeniden sıralama tokenı | 1 278 444 (sorgu başına ~39 950) | 573 406 / 27 istek (~21 240) |
| Sorgu süresi (uçtan uca) | ort. 2,6; p50 2,5; p95 3,3; en çok 3,7 sn | ort. 2,0; p50 1,8; p95 2,5 sn |
| Hız sınırı / başarısız istek | 0 / 0 | 0 / 0 (bütçe durdurması hariç) |

- 50 aday tekilleştirmeyle beklenen 28/32'yi verdi; G5 eşiği (%80) geçti.
  Kaçanlar: 1 (yarım araç çağrısı), 2 (onaydan sonra değişen dosya),
  11 (seccomp), 27 (geri alma). 12 Ekim'in farklı dosya sayımına göre 22
  (bağlantı sınırı) kazanıldı, 2 kaybedildi; toplam aynı.
- 20 aday ilk 28 sorguda 50 adayın kaçırdıklarına ek olarak 14 (gizli dosya
  taraması) ve 22'yi kaçırdı. Kalan dört sorgunun hepsi isabet olsa bile en
  çok 26/32 olur; 50 adayın altında kalır. Bu yüzden kalan dört sorgu için
  ayrıca token harcanmadı.
- 20 adayın token kazancı tekilleştirmeyle %60'tan ~%47'ye iner: 20 sonuç
  istendiğinde aday listesi farklı dosya sayısı 20'ye ulaşana kadar
  genişler (#13). Ürünün varsayılan sonuç sayısında genişleme daha azdır;
  bu ayrıca ölçülmedi.

## Karar

- **50 aday kalır.** 20 aday kaliteyi düşürdü (en iyi durumda 26/32 < 28/32).
- **G5, tekilleştirmeyle 28/32.**

## Yan bulgular

- **Düzenek döngüsü (düzeltildi):** İndeksleme bekleme döngüsü
  "İndeks eksik"i bütün çıktıda arıyordu. "indeks" sorgusunun en iyi sonucu
  bu metni düz yazı olarak içeren `arama.rs` testine denk gelince döngü hiç
  bitmedi (her tur bir sorgu gömmesi, ~21 token). Artık yalnız sonuçlardan
  önceki başlık satırlarına bakılıyor; birim testi
  `baslik_sonuc_metnini_saymaz`.
- **Tam yeniden gömme:** Kopyalanan indeks, ana klasörde 610 dosyanın
  tamamını yeniden gömdü (265 istek, 1 005 978 token). Parça özetleri
  eşleşmedi. Ana klasör CRLF ile açılmış (`core.autocrlf=true`); 12 Ekim'in
  worktree'si büyük olasılıkla LF'liydi. Bu neden doğrulanmadı.
- **256 KiB sınırı:** `docs/superpowers/plans/2026-09-21-kalfa-1b-cekirdek.md`
  CRLF ile 267 174 bayt; `AZAMI_DOSYA` (256 KiB) üstünde kaldığı için
  Windows checkout'unda indekse girmez. Satır sonu dosya boyutunu, dolayısıyla
  indekslenip indekslenmemeyi platforma göre değiştirir. Not edildi; ürün
  değişikliği yapılmadı.

## Maliyet

Bugünün bütün canlı G5 harcaması yanıtların `usage` alanından sayıldı,
yaklaşık **2,9 M token**:

| Koşu | Token |
|---|---:|
| İlk koşu: tam yeniden gömme ve döngü (durduruldu) | ~1 006 900 |
| Döngü teşhisi (iki kısa koşu) | ~4 000 |
| 50 aday | 1 283 782 |
| 20 aday (bütçede durdu) | 609 966 |

`X-Evren-Credits-Remaining` 1060 → 1060 (değişmedi).

## Kayıtlar

Loglar `target/kalfa-g5-20261014/` altında (depoya girmez): `aday50.log`,
`aday20.log`, `aday50-dongu.log`, `tam-cikti.log`.
