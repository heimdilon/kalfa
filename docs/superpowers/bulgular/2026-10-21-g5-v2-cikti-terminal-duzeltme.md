# G5 v2 — çıktı imleci ve terminal Enter düzeltmesi

Dal: `codex/g5-butunlesik-kabul`, PR [#44](https://github.com/heimdilon/kalfa/pull/44).
Taban: son Hat05 `d33f239` + son Hat01 `1d88a1a`; bütünleştirme `5a42f4e`.

## Değişiklik

Gerçek S02 kabulü, uzun komut çıktısının tamamının `komut_oku` tarafından döndüğünü, çekirdeğin 64 KiB sınırında kırptığını, buna karşın imlecin görünmeyen baytları da geçmiş saydığını gösterdi. Ortadaki hata model isteğine hiç ulaşmıyordu. Model köprüsü şimdi 16 KiB ham baytlık parçalar okur. Bozuk UTF-8 üç katına genişlese bile üst veriyle model sınırına sığar; imleç yalnız dönen ham baytları geçer. Normal uygulama/arayüz okuma API'si değişmez. UTF-8 sınırı ve bozuk baytlar için regresyon eklendi.

Terminal kartı Enter için CR (`\r`), borulu girdi için LF (`\n`) gönderir. Gerçek ConPTY satır okumasında LF'nin Enter yerine geçmemesi unit regresyonuyla yeniden üretildi; test önce düştü, dar ürün düzeltmesinden sonra geçti. Arayüz uçtan uca terminal deneyi ayrıca açıktır.

Test desteğinin `IstegeGore` yanıtı yalnız `test-destek` özelliğindedir. Modelin sonraki iş kimliği, imleci ve özeti gerçek önceki araç sonucundan türetilir. K06 sistem/alt dizin kapsamını, S01 gerçek iş sürerken tur dönüşünü, S03/S04 gerçek model kontrol çağrılarını, S05 soru kimliği/yanıtı/kompakt özet/kurtarma bağını ölçer. S02 oracle'ı, satır parça sınırında bölünebildiği için yalnız gerçek çıktı gövdelerini sırayla birleştirir.

## Doğrulama ve kabul sınırı

- Gerçek S02 kırmızı kanıtı ve ilk düzeltmeden sonraki Windows 8/8 sonucu `.yerel/g5-kabul/` altındadır. İlk tekrarların S02 oracle sınırı ve eksik MCP yardımcı ikilisi hataları da saklanmıştır; başarı diye sayılmaz.
- Son Windows/Linux iki tekrar, çekirdek/uygulama takımı, clippy ve tam arayüz sonuçları aşağıdaki son makbuzda güncellenir. Çalışan kontrol yeşil sayılmaz.
- S06 testi güvenli seri geri dönüşü doğrular; eşzamanlı sunucu+yazma **desteklenmiyor**, görev puanı Geçti değildir.
- G5'in 24 görevlik tam koşucusu, kalan 16 görev adaptörü ve gerçek Tauri süreç/MCP/web kabulü açık. En az20/24 kapısı henüz kapanmadı. Senaryolu model testleri canlı LLM başarısı değildir.
- 21 Ekim kullanıcı yanıtı: ücretli model/dış arama deneyi yapılmayacak. Geçici GitHub CI'da mevcut Sürüm paketleri workflow'unun bağımlılık indirip kurucuları kurması/kaldırması açıkça onaylandı. Geliştirici makinesinde kurulum/sistem ayarı yok. Merge onayı yok.
- Bağımsız tekrar incelemesi hesap kotası nedeniyle çalışamadı; önceki incelemenin bulguları giderildi ancak yeniden inceleme kapanmış sayılmaz.

## Son makbuz

Son doğrulama ve paket workflow sonucu bekleniyor; tam kabul iddiası yok.

## Tek sonraki iş

Mevcut son kontrolleri sonuçlandırıp PR başını makbuzlamak; ardından G5 M03 gerçek HTTP kayıt güncelleme+geri okuma adaptörünü eklemek. Ana checkout ve diğer uygulayıcıların worktree'leri korunur.
