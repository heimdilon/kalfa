# Kalfa — Linux paketleri: Tasarım

**Tarih:** 2026-10-13.
**Durum:** Uygulandı ve birleşti (PR #6, `0e0949d`); VM kabulü 19/19 geçti (§9.2).
**Taban:** `main`, `4241994` (5b tamamlandı).
**Üst belge:** [Kalfa tasarımı](2026-09-21-kalfa-design.md) §12.1 (Dağıtım), risk 9;
[aşama 2 tasarımı](2026-09-24-kalfa-2-kabuk-yalitim-design.md) §8.3.

## 1. Amaç ve kabul ölçütü

Kalfa Linux'ta `.deb` ve `.rpm` paketiyle kurulur, kaldırılır ve yükseltilir.
Kurulu uygulama yerleşik `kalfa-rag` eklentisini ve ağ köprüsünü (`kalfa-kopru`)
bulur. Ubuntu 24.04 ve sonrasında, AppArmor ayrıcalıksız kullanıcı ad alanını
kısıtladığı hâlde, `.deb` ile kurulan Kalfa'nın yalıtımı elle ayar
gerektirmeden hazır olur. Sistemin `/usr/bin/bwrap`'ı ve onu kullanan diğer
uygulamalar (Flatpak vb.) etkilenmez.

Başarı: iki paket aynı sürümden tek betikle üretilir; paket içeriği, izinler
ve bakımcı betikleri denetlenir; Ubuntu 24.04 masaüstü VM'inde `.deb`
kurulunca Kalfa'nın yalıtım durumu `Hazir`, kaldırılınca profil ve dosyalar
gider, kullanıcı verisi kalır.

## 2. Kullanıcı kararları (13 Ekim)

1. **AppArmor:** profil yalnız Kalfa'nın kendi bwrap kopyasına
   (`/usr/lib/kalfa/bwrap`) ad alanı izni verir. Sistem bwrap'ına profil
   eklenmez; dağıtımın `bwrap-userns-restrict` profiline bağlanılmaz
   (sürümler arasında eklenip geri alındı, sınaması zor).
2. **Araç zinciri:** Linux'ta Python, Node ve git sistemden gelir; paketler
   bunları yalnız öneri (`Recommends`) olarak anar. Windows'taki gibi
   paketlenmez.
3. **Biçim:** `.deb` ve `.rpm` birlikte.
4. **Gerçek sınama:** kullanıcının kuracağı Ubuntu 24.04 masaüstü VM'i.
   WSL'de AppArmor yoktur (aşama 2 tasarımı §8.4).

## 3. Mevcut durum

- `tauri.conf.json` paketlemeyi açar; Windows'a özgü ayar
  `tauri.nsis.conf.json`'dadır ve `kurucu/derle.mjs` onu `--config` ile
  verir (NSIS, `kalfa-yalitim`, `kalfa-kopru`, araç zinciri, eklenti).
- `kalfa-uygulama::dizin::kopru_ikilisi` köprüyü exe'nin dizininde ve bir
  üstünde arar; Linux paketinde exe `/usr/bin/kalfa`, köprü
  `/usr/bin/kalfa-kopru`'dur.
- `yerlesik_eklenti_dizini` Tauri'nin kaynak dizinindeki `eklentiler/`'i
  kullanır; Linux'ta bu `/usr/lib/Kalfa/`'dır.
- `kalfa_araclari` Linux'ta boştur: araç zinciri sistemdendir (karar 2 ile
  uyumlu).
- `kalfa-sandbox::linux` bwrap'ı yalnız sabit sistem yollarında arar (§8.1:
  `PATH`'ten aranmaz). Algılama, ad alanı kısıtını `KullaniciAdAlaniKapali`
  olarak bildirir ve elle profil talimatı verir.
- Seccomp filtresi kaptaki `unshare`/`clone` çağrılarında `CLONE_NEWUSER`
  dahil ad alanı bayraklarını reddeder: kaptaki komut yeni kullanıcı ad
  alanı açamaz.

## 4. Paket içeriği

| Yol | `.deb` | `.rpm` | Kaynak |
|---|---|---|---|
| `/usr/bin/kalfa` | ✓ | ✓ | Tauri |
| `/usr/bin/kalfa-kopru` | ✓ | ✓ | `externalBin` |
| `/usr/lib/Kalfa/eklentiler/kalfa-rag/` | ✓ | ✓ | `resources` |
| `/usr/share/applications/`, simgeler | ✓ | ✓ | Tauri |
| `/usr/lib/kalfa/bwrap` | ✓ | — | derleme makinesinin `bubblewrap` paketi |
| `/usr/share/kalfa/apparmor/kalfa-bwrap` | ✓ | — | `kurucu/linux/` |
| `/usr/share/doc/kalfa/bubblewrap/{copyright,KAYNAK}` | ✓ | — | LGPL bilgisi |

Bağımlılıklar:

- `.deb`: Tauri'nin ekledikleri (`libwebkit2gtk-4.1-0`, `libgtk-3-0`) ile
  bwrap kopyası için `libcap2`, `libselinux1`. Öneri: `python3`, `nodejs`,
  `git`. `bubblewrap` paketine bağlı değildir.
- `.rpm`: `bubblewrap` (Fedora'da ad alanı kısıtı yok; sistem bwrap'ı
  kullanılır) ve paylaşımlı kitaplık yetenekleri
  `libwebkit2gtk-4.1.so.0()(64bit)`, `libgtk-3.so.0()(64bit)`. Tauri'nin RPM
  paketleyicisi, `.deb`'tekinin aksine, WebKitGTK/GTK bağımlılığını kendisi
  eklemez (Codex incelemesi); kitaplık adı biçimi dağıtımın paket adından
  (`webkit2gtk4.1`, `libwebkit2gtk-4_1-0`) bağımsızdır. Öneri: `python3`,
  `nodejs`, `git`.

Ayar `crates/kalfa/tauri.linux-paket.conf.json`'dadır; derleme
`kurucu/derle-linux.mjs` ile yapılır (§7). Tauri'nin otomatik birleştirdiği
`tauri.linux.conf.json` adı kullanılmaz: geliştirme derlemesi paket
ayarından etkilenmemelidir.

## 5. Kalfa'nın bwrap kopyası ve AppArmor

**Neden kopya:** AppArmor profili yola bağlanır. `/usr/bin/bwrap`'a profil
eklemek dağıtımın ya da başka uygulamaların bwrap profilleriyle çakışır ve
makinedeki her bwrap kullanıcısının davranışını değiştirir. Kalfa'ya özel
yol yalnız bu kopyanın davranışını belirler.

**Kaynak:** Derleme makinesinin dağıtım paketindeki değiştirilmemiş
`/usr/bin/bwrap`. Sürüm `dpkg-query` ile okunur; `copyright` ve kaynak
bilgisi (`apt-get source bubblewrap=<sürüm>`) pakete yazılır (LGPL-2.0+).
Kopya setuid değildir. Güvenlik güncellemeleri Kalfa sürümüyle gelir;
sürüm notlarında bwrap sürümü yazılır.

**Profil:** AppArmor'un `bwrap-userns-restrict` desenini Kalfa'nın yoluna
uygular (`kurucu/linux/kalfa-bwrap`). `kalfa-bwrap` profili bwrap'a
kullanıcı ad alanını ve kurulum için yetenekleri verir; bwrap'ın başlattığı
her süreç (`pix … -> &kalfa-bwrap//&kalfa-bwrap-cocuk`) `audit deny
capability` taşıyan çocuk profile yığılır. Yığma gereklidir: bwrap
no-new-privs kullanır.

**Neden kısıtlı çocuk (güvenlik incelemesi, 13 Ekim):** Kopya her yerel
kullanıcı tarafından çalıştırılabilir. Yalnız `unconfined` + `userns`
veren bir profil, makinedeki her programın Kalfa'nın bwrap'ı üzerinden ad
alanı kısıtını aşmasına, yani `/usr/bin/bwrap`'a izin vermekle hemen hemen
aynı sonuca yol açardı. Kısıtın amacı, ayrıcalıksız kodun kullanıcı ad
alanındaki yeteneklerle çekirdek saldırı yüzeyine erişmesini önlemektir.
Çocuk profil yeteneği kaldırdığı için kopyayı çalıştıran başka bir program
da yeteneksiz bir ad alanı alır. Kalan etki: her yerel kullanıcı Kalfa'nın
bwrap'ıyla yeteneksiz bir kap açabilir; bu, Ubuntu'nun kendi
`bwrap-userns-restrict` profilinin kabul ettiği düzeydir. Kalfa'nın kendi
komutları yetenek kullanmaz; kaptaki yeni ad alanı ayrıca seccomp ile
kapalıdır (§3).

**Kurulum:** Profil `/usr/share/kalfa/apparmor/` altında gelir; `postinst`
yalnız `kernel.apparmor_restrict_unprivileged_userns` varsa (AppArmor 4'lü
Ubuntu çekirdekleri) ve `apparmor_parser` bulunuyorsa onu
`/etc/apparmor.d/kalfa-bwrap`'a kopyalar ve yükler. Debian 12'nin AppArmor
3'ü `abi <abi/4.0>`'ı bilmez; profil doğrudan `/etc/apparmor.d/`'ye
konsaydı oradaki AppArmor yüklemesini bozardı. Yükleme başarısızsa kurulum
sürer ve komut iletisi yazılır.

**Kaldırma:** `postrm remove|purge` profili çekirdekten ve
`/etc/apparmor.d/`'den kaldırır; `upgrade`'de dokunmaz, yeni `postinst`
yeniler. Kullanıcı verisi (`~/.local/share/kalfa`) silinmez.

**Kod:** `BWRAP_YOLLARI` önce `/usr/lib/kalfa/bwrap`'ı dener. Yol kök
sahipli paket dizinindedir; `PATH` kuralı (§8.1) korunur. Paket dışı
kurulumda davranış değişmez.

**Algılama iletisi:** `TALIMAT_AD_ALANI`, `.deb` kurulumunda profilin
`sudo dpkg-reconfigure kalfa` ile yeniden kurulabileceğini, paket dışı
kurulumda `.deb`'in kullanılmasını söyler. Eski ileti `/usr/bin/bwrap`'a
`unconfined` + `userns` profili yazmayı öneriyordu; §5'teki gerekçeyle
kaldırıldı. İleti sistem genelindeki kısıtı kapatmayı ve sistem bwrap'ına
sınırsız profili önermez. `GEREKSINIM` (BT paragrafı) paket kopyasını ve
profili anar.

## 6. Derleme tabanı

Tauri, en eski desteklenen dağıtımda derlemeyi önerir: daha yeni glibc ile
derlenen ikili eski sistemde açılmaz. Ubuntu 24.04'te (glibc 2.39) derlenen
paket Debian 12 / Pardus 23'te (glibc 2.36) çalışmaz. Sürüm paketleri
Debian 12 kabında derlenir; bwrap kopyası da o kabın `bubblewrap`
paketinden gelir. Ön doğrulama Ubuntu 24.04'te paket mekaniğini ölçer
(§9); taban kabı sürüm iş akışıyla gelir (§10).

## 7. Derleme betiği

`node crates/kalfa/kurucu/derle-linux.mjs` (`ui/`'de `npm ci` sonrası):

1. `kalfa-kopru`'yu sürüm kipinde derler, `binaries/kalfa-kopru-<üçlü>`'ye
   kopyalar.
2. `kalfa-rag`'ı `wasm32-wasip2` için derler, `eklentiler/kalfa-rag/`'a
   koyar.
3. bwrap'ı, `copyright`'ı ve sürüm/kaynak notunu `binaries/`'e yazar.
4. Arayüzü derler ve `tauri build --config tauri.linux-paket.conf.json`
   çalıştırır. Çıktı `target/release/bundle/{deb,rpm}/`'dedir.

`binaries/` ve `eklentiler/` derleme çıktısıdır, depoya girmez.

## 8. Sınama

- **Birim:** `bwrap_yolu` sırası (paket kopyası önce) ve yeni talimat metni.
- **Paket (WSL, kurulumsuz):** `dpkg-deb -I/-c` ile bağımlılık, dosya,
  izin ve bakımcı betikleri; açılan paketteki bwrap'ın `--version` ve
  ad alanlı deneme çalıştırması; profilin `apparmor_parser -Q -K` ile
  sözdizimi.
- **VM (Ubuntu 24.04 masaüstü, kullanıcı kurar):** `apt install ./kalfa*.deb`;
  `aa-status`'ta `kalfa-bwrap` ve `kalfa-bwrap-cocuk`; sistem bwrap'ının
  hâlâ reddedildiği (`bwrap --unshare-user … true` başarısız), Kalfa
  kopyasının çalıştığı; kopya `--uid 0 --cap-add ALL` ile çağrılsa da kapta
  yetenek isteyen işlemin (`unshare -n`) reddedildiği (yalnız `unconfined`
  bir profil buna izin verirdi);
  uygulamanın yalıtım durumu `Hazir`; ağsız ve ağlı yalıtımlı komut;
  yükseltme (aynı paketi yeniden kur); `apt remove` sonrası profilin
  `aa-status`'tan ve `/etc/apparmor.d/`'den gittiği, `~/.local/share`'in
  kaldığı. Betik ve geri alma adımları plandadır.
- **`.rpm`:** içerik denetimi `rpm -qlp` ister; WSL'de `rpm` aracı yok.
  Kurulumu kullanıcı onayıyla (paket kurma) ya da bir Fedora kabıyla
  sınanır; aksi hâlde raporda sınanmadı yazılır.

## 9. Ön doğrulama

WSL Ubuntu 24.04 (`kalfatest`), commit `7b1ec30`, paket kurulmadan:

- `derle-linux.mjs` iki paketi üretti: `Kalfa_0.1.0_amd64.deb` (18,7 MB) ve
  `Kalfa-0.1.0-1.x86_64.rpm` (18,7 MB).
- `.deb` denetimi: `Depends: libcap2, libselinux1, libwebkit2gtk-4.1-0,
  libgtk-3-0`, `Recommends: python3, nodejs, git`; §4'teki bütün yollar
  doğru, sahip `0/0`; `/usr/lib/kalfa/bwrap`, `/usr/bin/kalfa` ve
  `/usr/bin/kalfa-kopru` `0755`; `postinst`/`postrm` çalıştırılabilir.
- Açılan paketteki bwrap `bubblewrap 0.9.0` (`0.9.0-1ubuntu0.3`) ve
  `--unshare-user --unshare-pid --unshare-net` denemesi çalıştı.
- Profil (`06ffdd2`'deki kısıtlı çocuklu biçim) `apparmor_parser -Q -K -T`
  ile geçerli.
- `/usr/bin/kalfa` `GLIBC_2.39` istiyor: §6'daki taban gereği doğrulandı.
- Kalfa'nın bwrap çağrısı `--cap-add` kullanmaz; çocuk profilin yetenek
  reddi Kalfa'nın komutlarını etkilemez.

Sınanmadı: gerçek AppArmor altında kurulum ve kaldırma (VM, kullanıcı
kurar; `kurucu/linux/kabul.sh`), `.rpm` içeriği ve kurulumu (WSL'de `rpm`
yok).

## 9.2 VM kabulü (13 Ekim)

Kullanıcının Ubuntu 24.04.5 LTS masaüstü VM'inde (VirtualBox, AppArmor
açık, `kernel.apparmor_restrict_unprivileged_userns = 1`), `c257a7c`'nin
`.deb`'i ile `kurucu/linux/kabul.sh`: **17 denetimin 17'si geçti, 0
düştü.**

- Ön koşul: kısıt açık; sistem bwrap'ı (`bubblewrap` paketi) kurulumdan
  önce reddediliyor.
- Kurulum: `apt` kurulumu; profil `/etc/apparmor.d/kalfa-bwrap`'ta;
  `kalfa-bwrap` ve `kalfa-bwrap-cocuk` yüklü.
- Kopya: ağsız ad alanı açıyor; `--uid 0 --cap-add ALL` ile çağrılsa da
  kapta `CAP_SYS_ADMIN` kullanılamıyor; sistem bwrap'ı kurulumdan sonra da
  reddediliyor.
- Yeniden kurulum: profil yüklü kalıyor.
- Kaldırma: profil dosyası ve çekirdekteki profil gitti; bwrap kopyası
  gitti.
- Uygulama (elle, kullanıcı bildirimi): Kalfa açıldı; bir oturumda ağsız
  (`echo`) ve ağlı (`curl`) birer kabuk komutu çalıştı.

İkinci koşu (`46bb8d5`'in `kabul.sh`'ı, aynı `.deb`; Codex incelemesinden
sonra): **19/19 geçti.** Eklenenler: elle denetim artık açık `[e/h]` yanıtıyla
sayılır (kullanıcı: yalıtım hazır, iki komut çalıştı) ve kaldırmadan sonra
gerçek veri dizini `~/.local/share/kalfa`'daki işaret dosyası yerinde.

`.rpm` başlığında (`46bb8d5`, `strings` ile) `bubblewrap`,
`libwebkit2gtk-4.1.so.0()(64bit)` ve `libgtk-3.so.0()(64bit)` gereksinimleri
var.

Sınanmadı: `.rpm` kurulumu; Debian 12 / Pardus / Fedora matrisi (sürüm
derlemesi Debian 12 tabanında gelince).

## 9.1 Uygulama

Paket mekaniği ön doğrulamada kodun kendisiyle sınandığı ve değişiklik
küçük olduğu için ayrı uygulama planı yazılmadı; tasarım, uygulama ve
kabul betiği aynı PR'dadır. Değişenler: `tauri.linux-paket.conf.json`,
`kurucu/derle-linux.mjs`, `kurucu/linux/{kalfa-bwrap,postinst,postrm,kabul.sh}`,
`kalfa-sandbox::linux` (`BWRAP_YOLLARI`, `TALIMAT_AD_ALANI`, `GEREKSINIM`
ve birim testi), `kurucu/README.md`, `.gitattributes` (betikler LF).

## 10. Kapsam dışı

- Sürüm iş akışı (Debian 12 kabında derleme, `SHA256SUMS`, taslak sürüm):
  ayrı iş olarak §12'de geldi.
- AppImage ve Flatpak.
- Paket imzası ve apt/dnf deposu.
- Linux'ta araç zinciri paketleme (karar 2).

## 12. Sürüm derlemesi (13 Ekim, KALFA-12)

`.github/workflows/linux-paket.yml` (KALFA-13'te `surum.yml` adını aldı; §13):

- **derle:** `debian:12` kabında derleme bağımlılıkları (`build-essential`,
  `pkg-config`, `libwebkit2gtk-4.1-dev`, `bubblewrap`), Node 24,
  `rust-toolchain.toml`'daki Rust; `derle-linux.mjs`. Paketteki bwrap kopyası
  Debian 12'nin `bubblewrap`'ından gelir. `kalfa`'nın en yeni glibc sembolü
  `GLIBC_2.36`'yı aşarsa iş düşer. `SHA256SUMS` paketlerle birlikte yapıt
  olarak saklanır.
- **sina:** paketler `debian:12`, `ubuntu:24.04` (`.deb`) ve `fedora:44`
  (`.rpm`) kaplarında özetle doğrulanır, paket yöneticisiyle kurulur;
  dosyalar, `ldd` ile kitaplık çözümü ve bwrap (`.deb`'de Kalfa kopyası,
  `.rpm`'de sistemin) denetlenir; paket kaldırılır.
- **yayimla:** yalnız `v*` etiketinde, sınama geçerse paketleri ve
  `SHA256SUMS`'u taslak GitHub sürümüne ekler. Etiket `tauri.conf.json`'daki
  sürümle (`v<sürüm>`) uyuşmazsa derleme işi düşer; sürüm notu paketteki
  bwrap kopyasının Debian sürümünü yazar (Codex incelemesi). Sürümü kullanıcı yayımlar;
  etiket de kullanıcının onayıyla atılır.

İş akışı paket dosyalarına (simgeler dahil) dokunan PR'larda da koşar. AppArmor davranışı
kapta gözlenemez; VM kabulü (`kabul.sh`) ayrıca yapılır.

**İlk koşu (PR #9, `9d5b8e1`, koşu 38445580145): bütün işler geçti.**
`kalfa`'nın en yeni glibc sembolü `GLIBC_2.34`; paketteki bwrap
`bubblewrap 0.8.0` (Debian 12). `.deb` Debian 12 ve Ubuntu 24.04 kaplarında,
`.rpm` Fedora 44 kabında kuruldu; `dnf`, `.rpm`'in yeni gereksinimleriyle
`webkit2gtk4.1` (2.54.1) ve `bubblewrap` (0.12.0) paketlerini çekti; `ldd`'de
çözülmeyen kitaplık yok; üç kapta kaldırma temiz. `.rpm` kurulumu ilk kez
böyle sınandı.

## 13. Windows kurucusu sürüm akışında (13 Ekim, KALFA-13)

İş akışı `surum.yml` ("Sürüm paketleri") adını aldı ve Windows'u da kapsar:

- **derle-windows (`windows-2025`):** Node 24 ve `rust-toolchain.toml`'daki
  Rust ile `kurucu/derle.mjs`. Python ve Node `araclar.json`'daki adres ve
  SHA-256 ile indirilir; özet tutmazsa derleme durur. NSIS kurucusu yapıt
  olarak saklanır.
- **sina-windows:** geçici CI makinesinde
  `.github/scripts/windows-kurucu-sinama.ps1`: `/S` ile sessiz kurulum;
  `Program Files\Kalfa` altında uygulama, yalıtım yardımcısı, köprü, araç
  zinciri ve eklenti; `kurulum.json` ve kayıttaki köklerde yalıtımın
  yetenek SID'ine (`S-1-15-3-1024-*`) mirassız izin; `/S _?=` ile sessiz
  kaldırma; dosyaların, kurulum kaydının ve kök izinlerinin gittiği. Betik
  sistem ACL'sini değiştirir; yalnız CI makinesi içindir (elle kabul
  `kurucu/kabul.ps1`).
- **yayimla:** Linux ve Windows sınamaları geçerse bütün paketler için tek
  `SHA256SUMS` üretir; sürüm notuna Windows kurucusunun getirdiği Python ve
  Node sürümlerini ekler.

**Koşu (PR #11, `bdd4e78`, 38463488003): bütün işler geçti.** İlk koşu
(`b3fa6f1`) temiz makinede bir hatayı gösterdi: `derle.mjs`'teki
`cargo build -p kalfa-yalitim -p kalfa-sandbox --bin kalfa-kopru` yalnız
köprüyü derliyordu; `kalfa-yalitim.exe` yerel makinede eski bir derlemeden
kaldığı için fark edilmemişti. Düzeltmede iki ikili de `--bin` ile anılır.
İkinci koşuda kurucu `Kalfa_0.1.0_x64-setup.exe` (47 MB) üretildi; sessiz
kurulumda yedi dosya yerinde, yalıtım kök izinleri `C:\`, `D:\` ve
`C:\Users`'ta; sessiz kaldırmada dosyalar, kurulum kaydı ve kök izinleri
gitti. Linux işleri de geçti; taslak sürüm etiket olmadığı için atlandı.
