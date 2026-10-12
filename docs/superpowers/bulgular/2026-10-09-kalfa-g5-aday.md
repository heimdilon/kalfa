# G5 — aday sayısı ve dosya çeşitliliği ölçümü

**Tarih:** 2026-10-09. **Durum:** Kullanıcının bu ölçüme özel açık onayıyla
canlı karşılaştırma tamamlandı. G5'in aday/dosya karşılaştırması ölçüldü;
talimat/boyut/parça ve tam ilk indeksleme maliyeti açık. Ürün kodu değişmedi.

20 aday, bu sabit kümede 50 adayla aynı toplam dosya isabetini (26/32)
%60,33 daha az yeniden sıralama tokenıyla verdi; sorgu bazında bir kazanım
ve bir kayıp var. Dosyaları sonuçta tekilleştirmek 28/32'ye ulaştı. Aday
seçiminde dosya başına iki parçayla sınırlamak ilk beş parça başarısını
25/32'ye düşürdü. Bu son kip ürünün %80 hedefini bu kümede karşılamıyor.

## Soru ve yöntem

Mevcut 50 adayın yerine 20 aday yeniden sıralamaya giderse maliyet ve dosya
bulma başarısı nasıl değişir? Aynı dosyanın parçalarının ilk beş sonucu
doldurmasını azaltmak beklenen dosyayı bulmayı iyileştirir mi?

`crates/kalfa-rag/tests/g5_olcum.rs`, mevcut indeksi yalnız okur. Kalfa'nın
izinli dosya okuma yolunu, manifest globlarını, gizli/korumalı yol denetimini
ve kaynak parçasının SHA-256 doğrulamasını kullanır. Kaynak kapsamı
`crates/`, `ui/`, `e2e/`, `test-eklentileri/`, `docs/` ve kökteki
`README.md`, `AGENTS.md`, `Cargo.toml`, `rust-toolchain.toml` ile sınırlıdır.
Bağlantılar izlenmez; hard link'li kaynaklar okunmaz.

İndekste 547 dosya/3912 parça var. İlk kuru koşuda 411 dosya/3564 parça
doğrulandı; 348 parça kaynak değişimi/izin/kapsam nedeniyle dışarıda kaldı.
32 sorgunun tümü için en az bir beklenen dosya doğrulanmış kümede var.
Kaynak metinleri bellekte sabitlenir, yeniden gömülmez. İndeks ve vektör
dilimlerine yazılmaz. Bu, değişmeden kalmış parçalardan oluşan kontrollü
bir karşılaştırmadır; güncellenmiş tam ürün indeksinin kalite testi değildir.
Önceki 26/32 ölçümünün yerine geçmez.

| Kip | Kosinüs aday sayısı | Dosya başına en çok parça |
|---|---:|---:|
| `aday50` | 50 | 50 |
| `aday20` | 20 | 20 |
| `aday20-dosya2` | 20 | 2 |

Her sorgu yalnız bir kez mevcut İngilizce talimat ve 1024 boyut/base64 ile
gömülür. Üç kip aynı sorgu vektörünü ve kaynak kümesini kullanır; kip sırası
sorgu başına döner. Kosinüs, ürünün f32 toplama/normalleştirme ve eşitlikte
yol/parça sırası davranışını izler. Yeniden sıralama EVREN'e SDK üzerinden
gider; `top_n` tüm adayları ister. Her yanıttan hem ilk beş parça hem her
dosyanın en yüksek puanlı parçasından oluşan ilk beş farklı dosya çıkarılır.
İkinci çıktı ek API çağrısı gerektirmez. Ürünün `top_n=5` isteğine karşı
bağımsız bir kalite/süre eşdeğerliği iddiası değildir.

Başarı, beklenen dosyalardan en az birinin ilk beşte bulunmasıdır. Aday
kapsamı, ilk beşte farklı dosya sayısı, yeniden sıralama girdi tokenı,
HTTP istemcisinin istek süresi ve yönlendirilen model de kaydedilir.
Dosya düzeyindeki bu etiketler parça içeriğinin cevap kalitesini ölçmez.
Kosinüs/sıralama ayarları ürün kodunda değiştirilmez.

## Gönderim ve devam sınırları

- Hedef yalnız `https://evren-llmapi.ssyz.org.tr/v1`; şart kabul yolu yok.
- 32 tekli sorgu gömme, 96 yeniden sıralama isteği; her sıralama 20/50
  izinli kaynak parçası içerir. Tam indeksin metni gönderilmez.
- Yaklaşık 2–3 milyon girdi tokenı beklenir; raporlanan toplam için 5 milyon
  bütçe vardır. Bir sonraki çağrı için ihtiyatlı istek baytı payı ayrılır.
  Yanıtta kullanım alanı eksikse veya herhangi bir hata/sınır gelirse durur;
  otomatik yeniden deneme yapmaz. Başarısız isteğin sağlayıcı tüketimi
  bilinemez. Kredi bakiyesi/fiyatı ölçülmez.
- İstekler arasındaki iki saniye ölçüm süresine katılmaz. Ağ/sağlayıcı yükü
  kontrol edilmez; tek örnekli bu kıyas servis gecikmesi garantisi vermez.
- Makbuzlar yalnız `target/kalfa-g5-olcum/<girdi özeti>/` altında saklanır:
  sorgu vektörü, sıralama konum/puanları, süre, token ve model. Kaynak metni
  veya anahtar saklanmaz. Tamamlanmış aynı gövde SHA-256'sı yeniden
  gönderilmez; önceki makbuzlar toplam bütçeye dahildir. Bu deney önbelleği
  ürün aramasına eklenmez ve commit'lenmez.
- Günlüğe kaynak metni, vektör veya kredi/kimlik doğrulama başlığı yazılmaz.
  Yalnız sorgu, aday/sonuç yolları-aralıkları-özetleri, puanlar ve kullanım
  ölçüleri yazılır. Anahtar kökteki `.env`'den yalnız süreç ortamına alınır.

İlk girdi kümesinin SHA-256'sı:
`aa2188db7796f2e1e28bccfd8e1b449820b77196031008421237202e9b362aaf`.
Meta SHA-256'sı:
`010bf6de9fab64ad5517136b652318729f4f49504bc50a4a99ea7e4fdec7cca6`.
Canlı koşu öncesinde `KALFA_G5_GIRDI_OZETI` onaylanan kümenin özetiyle
eşleşmelidir; farklı kaynak/dilim kümesinde ağ çağrısı başlamaz.

## Tekrar çalıştırma

Mevcut Windows indeksi gerekir. Anahtar gerekmeden, depo kökünde:

```powershell
$env:CARGO_NET_OFFLINE = 'true'
$env:KALFA_G5_KURU = '1'
cargo test -p kalfa-rag --test g5_olcum --locked --offline canli_aday_sayisi_ve_dosya_cesitliligi -- --ignored --nocapture
```

Canlı koşu yalnız bu gönderim/kredi harcaması onaylandıktan sonra yapılır.
`KALFA_G5_KURU` kaldırılır, `KALFA_G5_GIRDI_OZETI` kuru koşudaki özete
ayarlanır; `.env`'in anahtar/taban alanları süreç ortamına güvenli biçimde
alınır. Aynı test komutu çalıştırılır. `--ignored` takımının bütünü çalıştırılmaz.

## Canlı sonuç

Windows 11 Home 10.0.26200, Ryzen 5 5600 (6 çekirdek/12 mantıksal), Rust
1.98.1; kaynak tabanı `0643aa8ca643929769bf29499ff52e7dad6aa0a6`.
Koşu 14:37:50–14:43:38 UTC (17:37:50–17:43:38 İstanbul) aralığında yapıldı;
test 346,92 sn sürdü. Ağdan önce iki platformun ağır kontrolleri tamamlandı.

| Kip | Beklenen dosya adaylarda | İlk 5 parçada isabet | En çok 5 farklı dosyada isabet | 32 sorgu için sıralama tokenı | Yeni HTTP istek süresi medyanı |
|---|---:|---:|---:|---:|---:|
| `aday50` | 32/32 | 26/32 (%81,25) | 28/32 (%87,50) | 1 262 365 | 1 032,18 ms (32 istek) |
| `aday20` | 30/32 | 26/32 (%81,25) | 28/32 (%87,50) | 500 809 | 510,12 ms (31 istek) |
| `aday20-dosya2` | 32/32 | 25/32 (%78,125) | 28/32 (%87,50) | 502 402 | 323,79 ms (32 istek) |

50'den 20'ye geçiş sıralama tokenını **%60,33** azaltıyor. Dosya başına iki
parçayla 20 aday seçmek **%60,20** azaltıyor; aynı maliyet düzeyinde ilk
beş parça başarısını iyileştirmiyor. İlk beş parçada farklı dosya sayısı
ortalaması sırasıyla 3,906 / 4,031 / 4,125; adaylarda 23,656 / 12,031 / 14,500.
20 adayın 24. sorgusunda yalnız dört farklı dosya bulunduğundan farklı
dosya çıktısı dört sonuç içerir; boş sonuçla tamamlanmaz.

32 sorgu gömme 1146 token kullandı; istek medyanı **113,54 ms**. Gömme
`qwen/qwen3-embedding-8b`, sıralama `qwen/qwen3-vl-reranker-8b`'ye yönlendi.
32 gömme + 95 sıralama olmak üzere **127 başarılı HTTP isteğinde
2 251 896 girdi tokenı** raporlandı; hata/hız/günlük sınır olmadı. 96 kip
karşılaştırmasından biri (6. sorgu `aday20`) aynı gövdeyi daha önce aynı
sorgunun `aday20-dosya2` kipinde gönderdiği için makbuzdan alındı;
14 826 token yeniden harcanmadı. Tablodaki tokenlar kiplerin ayrı çalışması
halindeki toplamdır; gerçek koşu tüketimiyle karıştırılmaz. Süre tablosunda
bu tekrar bağımsız örnek sayılmaz. HTTP süre aralıkları 50 adayda
721,0–6601,6; 20 adayda 209,1–6181,0; sınırlı 20 adayda 190,6–3961,4 ms.

Sorgu numaraları `degerlendirme.json`'da 1'den başlar (ham kayıtta
`sorgu_no` 0'dan başlar). 50'ye göre 20 aday ilk beş parçada 1. sorguyu
(kurtarma) kazandırdı, 2. sorguyu (onaydan sonra değişme) kaybettirdi.
Farklı dosya çıktısında 1'i kazandırdı, 3'ü (kilit) kaybettirdi; toplam
28/32 eşit olsa da sonuçlar sorgu bazında eşdeğer değildir. Önceden sınır
koyan 20 aday, 50'ye göre ilk beş parçada yalnız 2'yi kaybetti.
50 adayın farklı dosya çıktısı, parça çıktısına göre 3 (kilit) ve
14'ü (gizli dosya taraması) kazandırdı; 1 (kurtarma), 11 (seccomp),
22 (bağlantı güvenliği), 27 (geri alma) yine ilk beş dosyaya girmedi.

Mevcut indeksin **18 dosyasının ad, boyut ve SHA-256 değerlerinin tümü**
koşu öncesi/sonrası aynı kaldı. Deney makbuzları yerelde korunur; indeks
silinmedi. [Ham JSONL](2026-10-09-kalfa-g5-windows.jsonl) 131 satırdır:
girdi/kapsam, 32 sorgu, 96 kip karşılaştırması ve tamamlanma kaydı.
Metin ve vektör içermez. Sorgu kümesinin anlamsal JSON SHA-256'sı
`5b16464ca22795aa3f67035386f83c7283deab9151f1b46249dfff65c5fccb52`.
Kaynak belgelerinin sonradan güncellenmesi eski indeksle doğrulanan kümeyi
değiştirebilir; aynı deney için yukarıdaki kaynak tabanı/indeks ve girdi
özeti gerekir. Yeni özeti onaylanan eski özetmiş gibi kullanmayın.

## Çevrimdışı doğrulama ve karar

Yeni testin SHA-256'sı iki platformda aynı:
`af3041bb443e64358db6fac63d0e759f61c2cd0de1528a4366a3d79700de916d`.
Altı test; dosya sınırında uzak adayla doldurma, sonradan tekilleştirme,
kosinüs eşitliği, değiştirilmiş/UTF-8'i bölen kaynak, geçersiz vektör,
kaynak kapsamı ve kesik/tekrarlı makbuz/bütçe durumlarını kapsar.

| Kontrol | Windows | WSL2 Ubuntu 24.04, `kalfatest` |
|---|---|---|
| Tam workspace | 1000 geçti, 0 düştü, 15 atlandı | 942 geçti, 0 düştü, 15 atlandı |
| E2e özellikli testler | 114 geçti | 113 geçti |
| Düz/e2e/wasm clippy, fmt | Temiz | Temiz |
| G5 | Kuru koşu ve onaylı canlı ölçüm geçti | Altı çevrimdışı test geçti; canlı kaynak indeksi kopyalanmadı |

Mevcut bağımlılıklar ve `--locked --offline` kullanıldı; paket indirilmedi.
Arayüz değişmedi. Gerçek yalıtım/tauri-driver bu değişiklikte koşturulmadı.
WSL geçici kaynak kopyası kaldırıldı, ortak `kalfa-4b-target` korundu.
[CI 136](https://github.com/heimdilon/kalfa-arsiv/actions/runs/37937926487)
önceki `0643aa8` başında dört işiyle yeşil; bu yeni ölçümün başı ayrıca
CI'da doğrulanır.

20 aday ve sonradan dosya tekilleştirme, sonraki değerlendirmede sınanacak
uygun seçeneklerdir. 32 sorgunun toplam isabetinin eşitliği, tüm sorgularda
kalitenin korunmasını göstermiyor; 20 aday iki sorguda beklenen dosyayı
adaylara dahi almıyor. Aday seçimi sırasında dosya başına iki parça sınırı
bu kanıtla ürüne eklenmez. Ürünün 50 aday davranışı şimdilik korunur.
Tam/güncel indeksle wasm/host kalite doğrulaması ve G5'in talimat dili,
1024/4096 boyut, parça boyutu karşılaştırmaları; tam ilk indeksleme maliyeti
açık kalır. Dosya etiketi parça cevabını ölçmediğinden içerik kalitesi için
ayrı inceleme gerekir. Bu küçük, tek koşuluk küme servis garantisi değildir.
