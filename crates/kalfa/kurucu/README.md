# Kalfa Windows kurucusu

`ui/` dizininde `npm ci` çalıştırıldıktan sonra Windows'ta
`node ../crates/kalfa/kurucu/derle.mjs` çalıştırın. Bu komut
`kalfa-yalitim.exe` yardımcısını ve `kalfa-kopru.exe` köprüsünü kilitli Rust
bağımlılıklarıyla sürüm kipinde derler, Tauri'nin beklediği hedef üçlüsü
adıyla yerel `binaries/` dizinine kopyalar, arayüzü derler ve NSIS kurucusunu
üretir. Sonuç `target/release/bundle/nsis/` altındadır. `binaries/` derleme
çıktısıdır; depoya eklenmez.

Derleme, `araclar.json`'daki Python ve Node paketlerini resmi kaynaklarından
indirir (önbellek `target/kurucu-onbellek/`), SHA-256'larını denetler ve
`araclar/` dizinine açar. Özeti tutmayan dosya silinir ve derleme durur.
Python'a Kalfa'nın `python/sitecustomize.py`'si eklenir; yalıtımda geçici
dizin izni ve yol adı çözümü için gereklidir. Kurulumda araçlar
`$INSTDIR\araclar\python` ve `$INSTDIR\araclar\node` altındadır; lisans
metinleri (`LICENSE.txt`, `LICENSE`) yanlarındadır. `araclar/` derleme
çıktısıdır; depoya eklenmez. Sürüm değiştirmek için `araclar.json`'da adres,
iç dizin ve yayıncının yayımladığı özetle doğrulanmış SHA-256 birlikte
güncellenir. Ağsız testler: `node --test "crates/kalfa/kurucu/*.test.mjs"`.

Kurucu makine geneli kurulum için UAC ister. Dosyalar kopyalanınca
`kalfa-yalitim.exe kur` çalışır. Bu adım hata verirse kurulum sürer; Kalfa'nın
Ayarlar bölümündeki **Onar** kullanılabilir. Kaldırıcı dosyaları silmeden önce
`kalfa-yalitim.exe kaldir` çalıştırır. Bu adım başarısız olursa yardımcıyı
`%ProgramFiles%\Kalfa-Kurtarma\kalfa-yalitim.exe` yoluna kopyalar ve kalan
yalıtım izinleri için uyarı gösterir. Kopyalama da başarısız olursa kurucuyu
yeniden çalıştırarak yardımcıyı geri yükleyin.

Sürüm yükseltirken kurucu varsayılan olarak eski sürümü kaldırır; kaldırma
kancası Ayarlar'dan açılan araç zinciri dizinlerinin izinlerini de geri alır.
Yeni kurulum yalnızca kök izinlerini kurar. Ayarlar'daki **Onar**, kayıtlı
dizinlerden kabın okuyamadıklarını `kalfa-yalitim.exe kur --izin-ver <dizin>`
ile yeniden açar. Kalfa kendi Python ve Node'unu getirdiği için bu dizinler
yalnızca başka bir sürüm gerektiğinde kullanılır.

Kaldırma, oturum kayıtlarını ve Kalfa'nın verisini (`%APPDATA%\Kalfa`,
`%LOCALAPPDATA%\Kalfa`) silmez; Tauri'nin "uygulama verisini sil" seçeneği
yalnızca WebView önbelleğini siler.

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

## Linux paketleri

Linux'ta `ui/` dizininde `npm ci` çalıştırıldıktan sonra
`node crates/kalfa/kurucu/derle-linux.mjs` çalıştırın. Komut `kalfa-kopru`'yu
ve `kalfa-rag`'ı sürüm kipinde derler, derleme makinesinin `bubblewrap`
paketindeki `bwrap`'ı ve lisans bilgisini `binaries/`'e kopyalar, arayüzü
derler ve `tauri.linux-paket.conf.json` ile `.deb` ve `.rpm` üretir. Sonuç
`target/release/bundle/{deb,rpm}/` altındadır. Derleme makinesinde
`bubblewrap` kurulu olmalıdır. Paketler derlendikleri dağıtımın glibc'sini
ister; desteklenen en eski dağıtımda derleyin
([Linux paket tasarımı](../../../docs/superpowers/specs/2026-10-13-kalfa-linux-paketleri-design.md) §6).
Sürüm paketlerini `.github/workflows/linux-paket.yml` Debian 12 kabında
derler, `SHA256SUMS` üretir ve paketleri Debian 12, Ubuntu 24.04 ve Fedora
kaplarında kurup kaldırarak sınar.

`.deb`, Kalfa'nın kendi bwrap kopyasını `/usr/lib/kalfa/bwrap`'a koyar.
AppArmor'un ayrıcalıksız kullanıcı ad alanını kısıtladığı sistemlerde
(Ubuntu 23.10+) `postinst` `kalfa-bwrap` profilini `/etc/apparmor.d/`'ye
kurar ve yükler. Profil yalnız bu kopyaya ad alanı izni verir; kopyanın
başlattığı süreçler yeteneksiz bir alt profilde çalışır. Sistemin
`/usr/bin/bwrap`'ı etkilenmez. Profili onarmak için
`sudo dpkg-reconfigure kalfa`. Kaldırma profili de kaldırır; kullanıcı
verisi (`~/.local/share/kalfa`) kalır. `.rpm` sistemin
`bubblewrap` paketine bağlıdır ve profil taşımaz. Python, Node ve git
sistemden gelir; paketler onları yalnız önerir.

Elle kabul denemesi Ubuntu 24.04 masaüstü VM'inde, sudo yetkili sıradan
kullanıcıyla yapılır: `sh crates/kalfa/kurucu/linux/kabul.sh ./Kalfa_<sürüm>_amd64.deb`.
Betik kurulumu, profilleri, kopyanın yeteneksiz çocuğunu, sistem bwrap'ının
etkilenmediğini, yeniden kurulumu ve kaldırmayı sınar; uygulamadaki yalıtım
durumu elle denetlenir. Geri alma komutları betiğin başındadır.
