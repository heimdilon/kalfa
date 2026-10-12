# Kalfa 4c — PR #52 inceleme ve doğrulama kaydı

**Tarih:** 2026-10-09.
**İnceleyen:** Codex.
**Kod:** `003035e8c7d6c503a67ed1d6bdd952cadf6985a5`, `4c/plan`.
**Taban:** `4b/plan`, `6f047318020424f4a9ecc5a8d4574d9afebc27f6`;
inceleme `git diff 4b/plan...4c/plan` üzerinden yapıldı.
**Kapsam:** RAG, SDK yardımcıları, günlük token sınırı ve ilgili test/CI
değişiklikleri. #51'in değişiklikleri bu PR'ın bulgusu olarak değerlendirilmedi.

## Sonuç

İki işlevsel bulgu `003035e` üzerinde sahte ortamla yeniden üretildi;
aşağıdaki kanıtlar o sürüme aittir. B1/B2 düzeltmeleri aynı dalda uygulandı;
regresyon ve iki platform sonuçları aşağıdaki düzeltme kaydında tutulur.
Tasarımın kuralları uygulamadaki sorunlara uydurulmadı. Açık ölçümler ve
#51 bağımlılığı nedeniyle PR taslak kalır.

## Bulgular

### B1 — P2: Boyutu uygun dosyanın 256'dan çok parçası atlanıyor

**Yer:** `crates/kalfa-rag/src/indeks.rs:81–86`.

`yeni_say > TOPLU_PARCA` dalı dosyayı kuyruktan çıkarıp eski kaydını da
siliyor. Böylece tasarım §11.2'nin 256 KiB dosya sınırının altındaki bir
dosya, parçalama sonucunda 256'dan çok yeni parça oluşunca hiçbir gömme
isteğine girmiyor. Sonraki arama aynı dosyayı yeniden okuyor ve tekrar
atlıyor; dosya aramada bulunamıyor. Önceden indeksli dosya büyüyerek bu
duruma gelirse eski kaydı da düşüyor.

**Yeniden üretim:** 300 ayrı satır; her satır 850 ASCII bayt, toplam
255 000 bayt. 2400 karakterlik pencere iki satır alıp bir satır ilerliyor:
299 parça oluşuyor. İlk indeksleme **0 indeksli dosya** ile dönüyor.

Geçici kaynak kopyasında mevcut `SahteOrtam` ile çalıştırılan testin özü:

```rust
let o = SahteOrtam::yeni();
let metin: String = (0..300)
    .map(|i| format!("{i:04} {}\n", "a".repeat(844)))
    .collect();
assert!(metin.len() <= indeks::AZAMI_DOSYA as usize);
assert!(parca::parcala(&metin).len() > indeks::TOPLU_PARCA);
o.dosya("gecerli.md", &metin);
let durum = indeks::guncelle(&o).unwrap();
assert_eq!(durum.indeksli, 1);
```

**Gerçek çıktı:** `bayt=255000, parça=299, indeksli=0`; beklenen 1 yerine
0 ile test düştü.

**Düzeltme:** Dosyanın parçaları birden çok toplu isteğe bölünmeli;
kesintide tamamlanmış dilimler korunmalı ve dosyanın tüm parçaları
yerleşmeden tamamlandı sayılmamalı. 256 KiB altı / 256 parça üstü bir dosya,
bu dosyanın güncellenmesi ve arada iptal/429 senaryosu regresyon testine girmeli.

### B2 — P2: Kalıcı 429'da eldeki indeks kısmi arama sonucu vermiyor

**Yer:** `crates/kalfa-rag/src/arama.rs:71–75`; yeniden sıralama da aynı
dosyada `:140–145` ile hatayı doğrudan yayıyor.

İndeksleyici 429'u yakalayıp eldeki indeksle devam ediyor; ardından arama
sorguyu gömmek için yeniden EVREN'i çağırıyor. Günlük sınır hâlâ doluysa
bu küçük istek de `sinir-asildi` alıyor ve araç bütün aramayı hata olarak
bitiriyor. Sorgu gömme geçip yeniden sıralama sınıra takılırsa da seçilmiş
adaylar sonuç olarak dönmüyor. §9.5/§11.2'nin kısmi sonuç davranışı bu
kalıcı sınır durumunu kapsamıyor.

Mevcut bütünleşme testi yalnızca **ikinci** EVREN isteğine bir kez 429
veriyor; üçüncü sorgu gömme ve sonraki yeniden sıralama başarılı oluyor.
Bu, sınırın sonraki isteklerde de devam etmesi durumunu doğrulamıyor.

**Yeniden üretim:** Sahte ortamda `eski.md` önce başarılı indekslendi;
ardından `yeni.md` eklendi. Dosya/depo çağrılarını aynı ortama ileten,
gömme ve sıralama çağrılarının tümünde günlük sınır döndüren bir `Ortam`
sarmalayıcısıyla `arama::ara` çağrıldı. Beklenen kısmi sonuç yerine:

```text
arama=Err(Hata::SinirAsildi("EVREN'in günlük token sınırına ulaşıldı"))
```

**Düzeltme:** Sınır sürdüğünde hangi durumda kaynaklı kısmi sonuç
verilebildiği açıkça uygulanıp test edilmeli. Sorgu vektörü elde edilemediyse
indeksin korunması tek başına anlamsal arama yapmaya yetmez; bu durumda
uydurulmuş sonuç üretilmez. Yeniden sıralama kesilirse mevcut benzerlik
adaylarıyla kaynaklı sonuç ve sınır notu döndürme yolu değerlendirilmeli.
Kalıcı 429 ve gömme/sıralama aşamalarında ayrı sınır testleri gerekir.

## Yeniden üretim düzeneği

Kaynaklar `git archive 003035e` ile geçici WSL kopyasına alındı; iki
`#[cfg(test)]` regresyon denemesi yalnızca bu kopyanın `src/lib.rs`'ine
eklendi. Ürün kodu değiştirilmedi. Komut:

```sh
CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/home/kalfatest/kalfa-4b-target \
cargo test -p kalfa-rag --lib --locked --offline inceleme_ -- --nocapture
```

İki yeni deneme beklenen davranışı sınadı ve ikisi de düştü (0 geçti,
2 düştü, 39 filtrelendi; çıkış 101). Bunlar mevcut takımdaki test hataları
değildir; incelemede eksik kapsamı göstermek için yazılan denemelerdir.
Canlı EVREN kullanılmadı; anahtar okunmadı, kredi harcanmadı.

## B1/B2 düzeltme kaydı

- **B1:** Tek dosya birden çok toplu isteğe bölünür. `meta.dosyalar` yalnızca
  tamamlanmış sürümleri tutar; `meta.bekleyen` bitmemiş dosyanın yazılmış
  parça konumlarını ve özetlerini korur. Sonraki çağrı dosyayı yeniden
  okuyup bu vektörleri özetle kullanır; metin saklanmaz. Eski meta bu alanı
  boş kabul eder. Canlı dilim hesabı, sıkıştırma ve temizlik iki tabloyu da
  kapsar. JSON kaçışları dahil gerçek gövde sınırı korunur.
- **B2:** Sorgu vektörü indekslemeden önce alınır. İndeksleme 429/günlük
  sınıra takılırsa eldeki tamamlanmış dosyalarla arama sürer. Yeniden
  sıralama da sınıra takılırsa özeti doğrulanmış adaylar kosinüs puanıyla ve
  açıklama notuyla döner. Diğer hata, izin reddi ve iptal yine iletilir.
  Sorgu gömmesi baştan reddedilirse anlamsal sonuç üretilemez; hata korunur
  ve indeksleme başlamaz. Kalıcı sorgu önbelleği eklenmedi.
- **Regresyon:** 13 birim ve 2 gerçek wasm/host bütünleşme testi eklendi.
  299 parçalık dosyanın 256+43 girdili isteklerle bitmesi; yarım dosyada
  iptal/429, vektör ve meta yazmalarının her adımında hata/kota; değişen ya
  da eski sürüme dönen yarım dosya; silme, eski meta ve JSON gövde sınırı
  sınanır. Sıralama sınırında kosinüs sırası/puanı, sonuç sayısı, 60 KiB
  UTF-8 bütçesi ve izin/iptal iletimi denetlenir. Gerçek host testi hız ve
  günlük sınırın sonraki istekte de sürdüğünü sınar.

**Düzeltmelerin doğrulaması:** Windows ve WSL'de aynı beş Rust/test dosyası
(SHA-256 değerleri eşleşti), Rust 1.98.1 ve mevcut bağımlılıklarla
çevrimdışı doğrulandı. Sonuçlar yerel komut günlüklerinden toplandı:

| Kontrol | Windows | WSL, Ubuntu 24.04, `kalfatest` |
|---|---|---|
| `cargo test --workspace --locked` | 994 geçti, 0 düştü, 13 atlandı | 936 geçti, 0 düştü, 13 atlandı |
| `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked` | 114 geçti, 0 düştü | 113 geçti, 0 düştü |
| Düz clippy, `--all-targets`, `-D warnings` | Temiz | Temiz |
| E2e özellikli clippy | Temiz | Temiz |
| SDK/RAG/test eklentileri, `wasm32-wasip2` clippy | Temiz | Temiz |
| `cargo fmt --all -- --check` | Temiz | Temiz |

Komutlara `--offline`, Windows testlerine ayrıca `--test-threads=1` verildi.
WSL kaynak kopyası `/home/kalfatest/kalfa-4c-duzeltme-20261009` iş sonunda
kaldırıldı; önceden var olan `/home/kalfatest/kalfa-4b-target` korundu.
Gerçek yalıtım/tauri-driver ve canlı EVREN testleri tekrar koşturulmadı.

[CI koşusu 133](https://github.com/heimdilon/kalfa-arsiv/actions/runs/37918442208),
`83d30a9` belge commit'i üzerinde dört işiyle geçti.
[CI koşusu 134](https://github.com/heimdilon/kalfa-arsiv/actions/runs/37924380503),
B1/B2 düzeltme commit'i `2153f52` üzerinde dört işiyle yeşildir.
[CI koşusu 135](https://github.com/heimdilon/kalfa-arsiv/actions/runs/37930374255),
G6 ölçüm commit'i `4448592` üzerinde dört işiyle yeşildir.
[CI koşusu 136](https://github.com/heimdilon/kalfa-arsiv/actions/runs/37937926487),
G4/G8 ölçüm commit'i `0643aa8` üzerinde dört işiyle yeşildir.

## G5 aday/dosya karşılaştırması (2026-10-09)

[Ölçüm ve ham kayıt](2026-10-09-kalfa-g5-aday.md): kullanıcının bu ölçüme
özel açık onayıyla 32 sorgu/üç aday kipi; mevcut indeksin kaynakla eşleşen
411 dosya/3564 parçası sabitlenir, tam/güncel indeks yeniden kurulmaz.
20/50 aday ilk beş parçada 26/32; 20 aday %60,33 az sıralama tokenıyla bir
sorgu kazanır, birini kaybeder. Sonuçta dosya tekilleştirme 28/32; önceden
dosya başına iki parça sınırı 25/32. Ürün davranışı değiştirilmedi.
127 başarılı HTTP isteği 2 251 896 token, bir aynı gövde makbuzdan alındı;
indeksin 18 dosyasının ad/boyut/özetleri önce/sonra aynı. Hata/sınır olmadı.

Yeni canlı test varsayılan olarak atlanır; altı çevrimdışı testiyle tam
Windows workspace 1000 geçti/15 atlandı, WSL 942 geçti/15 atlandı;
e2e özellikli 114/113 geçti. Düz/e2e/wasm clippy ve fmt iki platformda temiz.
Test kaynağının SHA-256'sı iki platformda aynı; paket indirilmedi.
WSL kopyası kaldırıldı, önceden var olan ortak target korundu.
[CI koşusu 137](https://github.com/heimdilon/kalfa-arsiv/actions/runs/37947611609),
`76afebe` üzerinde dört işiyle yeşil; G5 bütünü kapanmadı.

## G5 talimat dili karşılaştırması (2026-10-09)

[Ölçüm ve ham kayıt](2026-10-09-kalfa-g5-dil.md): kullanıcı bu yeni
32 sorgu/iki dil koşusunu ayrıca açıkça onayladı. Aynı kontrollü kaynak,
1024 boyut ve 50 aday; yalnız gömme görev açıklamasının dili değişti.
İngilizce ilk beş parçada 26/32, Türkçe 25/32; Türkçe bir sorgu kazanıp
ikisini kaybetti. İlk beş farklı dosyada iki dil aynı 28/32'yi verdi.
Ürün İngilizce talimatta kalır; boyut/parça/tam kalite ve ilk maliyet açık.

128 yeni başarılı HTTP isteği, 2 548 682 token; hata/sınır/tekrar yok.
İndeksin 18 dosyası önce/sonra aynı. Sekiz çevrimdışı testle Windows
1002 geçti/16 atlandı, WSL 944 geçti/16 atlandı; e2e özellikli 114/113.
Düz/e2e/wasm clippy ve fmt iki platformda temiz. Test kaynağı özetleri
eşleşti; WSL kopyası kaldırıldı, ortak target korundu. Yeni başın CI'sı
ayrıca doğrulanır; ürün kodu, WIT ve bağımlılıklar değişmedi.

## İlk incelemenin doğrulaması

`003035e` üzerinde, Rust 1.98.1 ve mevcut bağımlılıklarla çevrimdışı:

| Kontrol | Windows | WSL, Ubuntu 24.04, `kalfatest` |
|---|---|---|
| `cargo test --workspace --locked` | 979 geçti, 0 düştü, 13 atlandı | Geçti; CI aynı commit'te 921 geçti, 0 düştü, 13 atlandı |
| `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked` | 114 geçti, 0 düştü | Geçti; CI aynı commit'te 113 geçti, 0 düştü |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Temiz | Temiz |
| E2e özellikli clippy | Temiz | Temiz |
| SDK/RAG/test eklentileri, `wasm32-wasip2` clippy | Temiz | Temiz |
| `cargo fmt --all -- --check` | Temiz | Temiz |

Atlanan 13 testin tamamı canlı API testi değildir; ölçüm testleri ve SDK'nın
atlanmış belge örneği de bu sayıya dahildir.

Windows testlerine `--test-threads=1` verildi; CI'ın kullanıcı açan ve
sistem ayarı değiştiren betiği geliştirici makinesinde çalıştırılmadı.
WSL tam doğrulaması `/tmp/kalfa-4c-003035e` kaynak kopyasında koştu;
kopya WSL kapanınca kayboldu. Bu yüzden tablodaki Linux test sayıları
yerel başarı çıkışından ayrı olarak aynı commit'in CI günlüğünden alındı.
Yeniden üretim için kullanılan `/home/kalfatest/kalfa-4c-003035e-inceleme`
kopyası iş sonunda temizlenir. Önceden var olan ortak target korunur.

[CI koşusu 131](https://github.com/heimdilon/kalfa-arsiv/actions/runs/37915947170):
Rust biçimi, arayüz ve Linux işleri geçti; Windows işi yeni commit sonrası
iptal edildi. CI'ın üç clippy adımı Windows'ta da geçti. Arayüzün
`check/test/build` adımları bu commit'te
yeşildir; yerelde paket kurulmadı.

Gerçek yalıtım, uyumluluk ve `tauri-driver` uçtan uca testleri koşturulmadı;
4c bu yolları değiştirmiyor. Bunların yerini yeşil CI tutmaz. Canlı kalite
testi tekrar koşturulmadı; önceki 26/32 sonucu
[ölçüm kaydında](2026-10-08-kalfa-4c-evren-sinirlari.md) korunur.

## G5 boyut karşılaştırması (2026-10-09)

[Ölçüm ve ham kayıt](2026-10-09-kalfa-g5-boyut.md): kullanıcı bu yeni
3555 kaynak parçası/32 sorgu/iki boyut/50 aday ve 8 milyon bütçeli koşuyu
ayrıca açıkça onayladı. Aynı kontrollü 411 dosya/3555 parçada 1024 ve 4096
ilk beş parçada **26/32**, farklı dosyada **28/32**; isabet edilen sorgular
da aynı. 4096 ham vektör alanı dört kat; native debug kosinüs medyanı
3,91 kat. Kalite kazancı görülmedi; İngilizce/50 aday/**1024** korunur.
Kaynak ve yeni sorgu çiftlerinin ön ek kosinüsü ≥0,9999; tarif/model
yönlendirmesi denetimi geçti. 184 yeni başarılı HTTP, **4 821 637 token**;
hata/sınır/tekrar yok, indeksin 18 dosyası önce/sonra aynı.

Beş yeni çevrimdışı testle Windows workspace **1007 geçti/17 atlandı**,
WSL **949 geçti/17 atlandı**; e2e özellikli **114/113** geçti.
Düz/e2e/wasm clippy ve fmt iki platformda temiz. Ana test ve yeni alt
modül SHA-256'ları iki platformda aynı. WSL kopyası kaldırıldı, ortak
target korundu. Kaynak metni/vektör/anahtar ham kayda veya commit'e girmez;
ürün kodu, WIT ve bağımlılıklar değişmedi. Bu alt küme native HTTP deneyi,
tam/güncel ürün indeksi ve wasm/host uçtan uca kalite/gecikme değildir.

Önceki talimat dili başı `0ffef46` için
[CI 138](https://github.com/heimdilon/kalfa-arsiv/actions/runs/37954272157)
ilk Windows denemesinde değişmemiş
`iptali_yok_sayan_eklenti_iki_saniyede_birakilir` testinde 3,2284461 sn
ölçtü (üst sınır 2,9 sn). Yerel tam Windows/WSL takımlarında ve üç ayrı
Windows tekrarında geçti; test veya güvenlik sınırı değiştirilmedi.
CI 138 tekrarı dört işiyle yeşil. Boyut `b3488e2` için
[CI 139](https://github.com/heimdilon/kalfa-arsiv/actions/runs/37961173852)
dört işiyle yeşil.

## G5 parça penceresi kıyası (hazırlık 9 Ekim, canlı 11 Ekim)

[Yöntem, ham kayıt ve karar](2026-10-09-kalfa-g5-parca.md): 408 dosyanın
tamamı eski metadata ve güncel ürün parçalarıyla eşleşir; önceki kümenin
değişmiş dosyalardaki sağlam parçaları kullanılmaz. Aynı dosyalar üç
satır/karakter/örtüşme penceresiyle yeniden parçalanır. Altı yeni kontrol
ve ürün kaynağının 12 parçalama testi G5 dosyasında geçti; toplam 31
çevrimdışı test, 4 atlanan deney. Anahtarsız kuru koşu bütün beklenen
dosyaların varlığını, 12 211 parçayı ve en çok 320 HTTP isteğini doğrular.
9 Ekim'de kullanıcı kalan yaklaşık 1 milyon günlük token nedeniyle
canlı koşuyu erteledi; gönderim yapılmadı, onay tarifi küçültülmedi.
11 Ekim'de devam ve hız sınırından sınırlı devam açıkça onaylandı:
son sınır 325 HTTP/16 milyon raporlanan token. **320 başarılı + 3 hız
sınırı = 323 HTTP**, **10 934 194 bilinen token**; başarısız tüketim bilinmez.
İlk beş parçada küçük/mevcut/büyük **24/26/26**, ilk beş farklı dosyada
**25/28/27** isabet (32 sorgu). Küçük %43,94 az sıralama tokenıyla kalite
kaybetti; büyük %81,84 fazla tokenla net kalite kazandırmadı. Mevcut
**60/2400/10** korunur; parça kıyası mevcut karar için kapandı.
Ürün indeksi bütün koşularda ad/boyut/SHA-256 ile aynı; ürün kodu değişmedi.
Tam Windows workspace 1025 geçti/18 atlandı, WSL 967 geçti/18 atlandı;
e2e özellikli 114/113, üç clippy/fmt iki platformda temiz. Ana test,
alt modül ve değişmemiş ürün parçalama kaynağı SHA-256'ları eşleşti.
[CI 140](https://github.com/heimdilon/kalfa-arsiv/actions/runs/37969507051),
deney kaynağı `1aa1ac4` üzerinde dört işiyle yeşil. Sonuç kaydının yeni
başı ayrıca kontrol edilir.

## Devam noktası

1. #51 açık; birleşince #52'nin tabanını `kalfa-asama-1` yapın. Yeni başın
   CI'ını ve düzeltmelerin iki platform doğrulamasını kontrol edin.
2. G5'in tam/güncel indeksle wasm/host kalite doğrulaması, tam ilk
   indeksleme token maliyeti açık. Aday/dosya kıyası
   [ölçüldü](2026-10-09-kalfa-g5-aday.md); 20 aday/sonradan tekilleştirme
   sonraki doğrulama seçenekleridir. Talimat dili
   [ölçüldü](2026-10-09-kalfa-g5-dil.md); mevcut İngilizce korunur. Boyut
   [ölçüldü](2026-10-09-kalfa-g5-boyut.md); mevcut 1024 korunur. G6
   [iki platformda ölçülüp kapandı](2026-10-09-kalfa-g6-cozme.md); G4/G8
   [canlı ölçüm ve v1 kararıyla kapandı](2026-10-09-kalfa-g4-g8-gomme.md). Günlük sınırın
   ayrı arayüz durumu, kalıcı gönderim izni ve dağıtım 4d'de.
