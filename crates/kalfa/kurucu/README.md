# Kalfa Windows kurucusu

`ui/` dizininde `npm ci` çalıştırıldıktan sonra Windows'ta
`node ../crates/kalfa/kurucu/derle.mjs` çalıştırın. Bu komut
`kalfa-yalitim.exe` yardımcısını kilitli Rust bağımlılıklarıyla
sürüm kipinde derler, Tauri'nin beklediği hedef üçlüsü adıyla yerel `binaries/`
dizinine kopyalar, arayüzü derler ve NSIS kurucusunu üretir. Sonuç
`target/release/bundle/nsis/` altındadır. `binaries/` derleme çıktısıdır;
depoya eklenmez.

Kurucu makine geneli kurulum için UAC ister. Dosyalar kopyalanınca
`kalfa-yalitim.exe kur` çalışır. Bu adım hata verirse kurulum sürer; Kalfa'nın
Ayarlar bölümündeki **Onar** kullanılabilir. Kaldırıcı dosyaları silmeden önce
`kalfa-yalitim.exe kaldir` çalıştırır. Bu adım başarısız olursa yardımcıyı
`%ProgramFiles%\Kalfa-Kurtarma\kalfa-yalitim.exe` yoluna kopyalar ve kalan
yalıtım izinleri için uyarı gösterir. Kopyalama da başarısız olursa kurucuyu
yeniden çalıştırarak yardımcıyı geri yükleyin.

**Bilinen sınır:** Sürüm yükseltirken kurucu varsayılan olarak eski sürümü
kaldırır; kaldırma kancası Ayarlar'dan açılan araç zinciri dizinlerinin
izinlerini de geri alır. Yeni kurulum yalnızca kök izinlerini kurar; dizinler
Ayarlar'dan yeniden eklenir. Araç zinciri 2d-3'te kurucuya girince bu dizinlere
gerek kalmaz.

Kurucunun elle kabul denemesi, [2d-2 planının 8. görevi](../../../docs/superpowers/plans/2026-09-27-kalfa-2d-2-son-kullanici.md)
uyarınca kullanıcı tarafından yapılır. Kullanıcı, **yönetici PowerShell** içinde
`./kabul.ps1 -KurucuYolu <setup.exe> -EskiYardimci <önceki kalfa-yalitim.exe>`
çalıştırır. Önceki 2d-1 kurulumu yoksa `-EskiYardimci` verilmez. Betik önceki
kurulumu kaldırır, kurucuyu görünür açar, `kurulum.json` kaydını denetler ve
önceden açık araç zinciri izinlerini geri yükler. Kurulum başarısızsa önceki
durumu geri kurmayı dener. Ardından uygulamanın yalıtım durumu, **Onar** ve
Windows kaldırıcı sınanır. Kaldırıcı hata verirse, yine yönetici PowerShell'de
`./geri-al.ps1` çalıştırılır; betik kurulu ya da kurtarma yardımcısını bulur.
Çalışma alanı izinleri Ayarlar'dan geri alınır.
