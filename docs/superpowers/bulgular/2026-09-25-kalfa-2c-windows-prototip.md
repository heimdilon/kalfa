# Kalfa 2c — Windows yalıtım prototipi: bulgular

**Tarih:** 2026-09-25
**Tasarım:** [Aşama 2 tasarımı](../specs/2026-09-24-kalfa-2-kabuk-yalitim-design.md) §9.5. Bulgular
tasarım belgesine işlendi (§9.2–§9.6, §14).
**Prototip:** [`prototip/windows-yalitim/`](../../../prototip/windows-yalitim/README.md). Ürün
kodu değildir.
**Ham ölçüm:** [`2026-09-25-kalfa-2c-olcum.md`](2026-09-25-kalfa-2c-olcum.md), son koşunun
çıktısı.

> **Ek ölçüm:** Aynı senaryolar AppContainer'da da ölçüldü. P1 geçti ve A8–A10 çözülüyor
> ([AppContainer bulguları](2026-09-25-kalfa-2c-appcontainer.md)). Bu rapor kısıtlı belirteç
> yolunun sonucudur.

## Sonuç: Dal B

- **Kısıtlayıcı SID'ler kullanıcı klasörlerini ayıramıyor.** Tasarımın varsayımı `ALL APPLICATION
  PACKAGES`'tı. Windows, S-1-15 ile başlayan hiçbir SID'i kısıtlayıcı SID olarak kabul etmiyor.
  - Sistem ve program dizinlerini okutabilen kullanılabilir tek SID `Users`.
  - `Users` verilince, `Users`'ın okuyabildiği her klasör okunur. `C:\projeler` gibi, C:\ kökünün
    varsayılan izinlerini taşıyan klasörler de buna girer.
- **Yazma ayrımı ve profil koruması tutuyor.**
  - Çalışma alanları birbirini okuyamıyor ve birbirine yazamıyor.
  - Geliştiricinin profili (OneDrive dahil) okunmuyor.
  - Gizli dosya ve korumalı yollar korunuyor.
  - Araç zincirlerinin tamamı çalışıyor.

Tasarımın dallarına göre bu **Dal B**:

- Okuma sınırı, bilgisayardaki her yerel kullanıcının okuyabildiği dosyalar olur.
- Geniş izinli klasördeki çalışma alanı hazırlanırken uyarı gösterilir.
- Durum arayüzde ve belgede yazılır.

2d'yi açmadan önce çözülmesi gereken üç açık nokta var (aşağıda A8–A10). En önemlisi A8: Schannel
kısıtlı belirteçte çalışmıyor, bu yüzden cargo ağlı çalıştırmada crates.io'ya ulaşamıyor.

## Ortam

- Windows 11 Home 10.0.26200. Yönetici adımları geliştiricinin hesabıyla UAC üzerinden çalıştırıldı.
- Araçlar: Git for Windows 2.55.0, Node 24.20.0 (npm 11.19.0), Python 3.14.7 (`C:\Python314`), Rust
  1.98.1 (rustup, `%USERPROFILE%\.cargo`).
- **Zincir:** tasarımın §9.3'ündeki gibi.
  - Geliştirici, yalıtım kullanıcısını `CreateProcessWithLogonW` ile başlatır.
  - Yardımcı kendi belirtecinden kısıtlı bir belirteç türetir (`DISABLE_MAX_PRIVILEGE`, kısıtlayıcı
    SID'ler).
  - Komutu bir Job Object'e atar (`KILL_ON_JOB_CLOSE`, 256 süreç, bütün UI kısıtları) ve
    `CreateProcessAsUserW` ile başlatır.
- **Kullanıcılar:** `KalfaPrototipAgsiz` ve `KalfaPrototipAgli`, `KalfaPrototip` grubunda.
- **Deneme ağacı:** `C:\KalfaPrototip` (iki çalışma alanı, onlara özel dizinler). Ek olarak C:\
  kökünün varsayılan izinleriyle bir klasör ve geliştiricinin profilinde bir kanarya dosyası.
- Kurulumu ve ağacı `geri-al.ps1` geri alır.

## Ölçümler

| # | Soru | Sonuç |
|---|---|---|
| P1 | AAP ile sistem okunur, kullanıcı klasörleri okunmaz mı? | **Hayır.** `CreateRestrictedToken`, S-1-15 SID'lerine (AAP, ARAP, yetenek SID'leri) `ERROR_INVALID_PARAMETER` (87) veriyor. `Users` ile sistem ve Program Files okunuyor; C:\ altındaki varsayılan izinli klasör de okunuyor ve listeleniyor. |
| P2 | En küçük kısıtlayıcı küme? | **Çalışma alanı + ortak okuma + Everyone + Users + RESTRICTED.** Bu kümeyle cmd, PowerShell 5.1, git (`init`, `add`, `commit`), node, npm, python, cargo ve rustc çalışıyor. Hangi SID çıkarılınca ne düştüğü aşağıda. |
| P3 | A'nın SID'i B'yi okuyup yazamıyor mu? | **Geçti**, iki yönde de. |
| P4 | Profil ve kimlik bilgileri okunamıyor mu? | **Geçti.** Profil listelenmiyor, profildeki dosya okunmuyor, `Credentials` dizinine ulaşılmıyor. |
| P5 | Güvenlik duvarı ağsız kullanıcıyı kesiyor mu? | **Düzeltmeyle geçti.** Kural kısıtlı belirteçlere ilk hâlinde uygulanmıyordu (aşağıda). Geri döngü açık (A9); varsayılan izinli borular salt okunur açılıyor (A10). |
| P6 | `CreateProcessWithLogonW` gecikmesi, tutamaçlar | **Sonraki çağrılar 57–61 ms** (ortanca 58). Profil zaten varken ilk çağrı 211 ms; profil oluşturan ilk çağrının süresi ölçülemedi. Std tutamaçlar aktarılıyor. Dönen süreç tutamacı Kalfa'nın Job Object'ine atanabiliyor. |
| P7 | Gizli dosyaya ret, korumalı yolda silme ve taşıma reddi | **Geçti.** `.env` okunmuyor. `.git/config` yazılamıyor, silinemiyor, üzerine taşınamıyor. `.git/hooks`'a dosya eklenemiyor, içindeki dosya silinemiyor, dizin yeniden adlandırılamıyor. |
| P8 | Users grubundan çıkarmanın etkisi | **Etkisiz.** `BUILTIN\Users`, Authenticated Users ve INTERACTIVE'i üye olarak içeriyor. Oturum açan her kullanıcı fiilen Users üyesi: gruptan çıkarılmış kullanıcı da sistem dosyalarını okuyor. |
| P9 | Çalışma alanına özel `CARGO_HOME` ile ağ ihtiyacı | **Bağımlılıksız crate ağsız derleniyor, bağımlı crate derlenmiyor.** Geliştiricinin `CARGO_HOME`'u salt okunur verilince bağımlı crate ağsız derleniyor. |
| P10 | UI kısıtları | Geliştiricinin penceresine ileti **engelleniyor**. Pano açılabiliyor ama panoda metin yoktu; okuma ölçülemedi. Pencere bulma (`FindWindow`) engellenmiyor; bulunan tutamaç kullanılamıyor. Yükleyici hata kutuları UI kısıtlarına rağmen geliştiricinin masaüstünde açılıyordu; `SetErrorMode` ile kapandı. |

### P2: hangi SID ne işe yarıyor

| Kümeden çıkarılan | Düşenler | Neden |
|---|---|---|
| Everyone | PowerShell, git, python, cargo | `NUL` aygıtına yazma yalnızca Everyone'a açık (git `/dev/null`'ı açamıyor) |
| Users | cmd dışında hepsi | Sistem ve program dizinleri |
| RESTRICTED | PowerShell (CLR başlamıyor) | Çekirdek nesne dizinleri ve kayıt defteri |
| Ortak okuma SID'i | cargo, rustc | `.cargo\bin` ve `.rustup` geliştiricinin profilinde; yalnızca bu SID'e açılıyor |
| Oturum SID'i | Hiçbiri | Gerekmiyor |

Yalnızca kısıtlayıcı SID'lerle (Everyone ve RESTRICTED olmadan) süreç hiç başlamıyor:
`STATUS_ACCESS_DENIED`.

## Tasarıma işlenen düzeltmeler

1. **Kısıtlayıcı küme:** çalışma alanı SID'i, ortak okuma SID'i, Everyone, Users, RESTRICTED. AAP
   ve öteki S-1-15 SID'leri kullanılamaz.
2. **Varsayılan DACL:** komutun belirtecinin varsayılan DACL'i yalnızca kullanıcıya, SYSTEM'e ve
   çalışma alanı SID'ine izin verir. Everyone ya da Users girseydi, aynı yalıtım kullanıcısıyla
   çalışan başka bir çalışma alanının komutu bu süreci açabilirdi.
3. **Hata kipi:** `calistir`, komutu başlatmadan önce hata kutularını kapatır
   (`SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX`). Çocuk süreç bunu
   devralır.
4. **Güvenlik duvarı kuralının kullanıcı koşulu ortak okuma SID'ini de içerir:**
   `D:(A;;CC;;;<KalfaAgsiz>)(A;;CC;;;<ortak>)`.
   - Koşul, sürecin belirteciyle bu tanımlayıcıya karşı bir erişim denetimidir. Kısıtlı belirteçte
     denetim kısıtlayıcı SID'lerden de geçmek zorundadır.
   - Yalnızca kullanıcının SID'i varken kısıtsız belirteç engelleniyor, kısıtlı belirteç geçiyordu.
   - Ortak SID'i yalnızca yalıtımın kısıtlı belirteçleri taşır; başka kullanıcıların süreçleri
     kullanıcı SID'i denetiminden zaten geçemez.
5. **Users grubundan çıkarma adımı kalkar** (P8). Oturum açma hakları aynı kalır: gruba yerel oturum
   açma izni; ağdan, uzak masaüstünden, toplu iş ve hizmet olarak oturum açma reddi.
6. **İzinler Win32 API'siyle verilir.**
   - `icacls` hiçbir hesaba karşılık gelmeyen SID'leri reddediyor ("hesap adlarıyla güvenlik
     kimlikleri arasında eşleştirme yapılmadı").
   - Ret maskeleri `SYNCHRONIZE` ve `READ_CONTROL` içermez; onları reddetmek dosyayı okumak için
     açmayı da bozar.
7. **Çalışma alanı SID'inin biçimi (A7):** `S-1-9-` ve 4 rastgele 32 bit alt yetki. S-1-9 ve
   S-1-5-21 biçimleri ikisi de çalıştı; S-1-9 gerçek hesap SID'leriyle karışmaz.
8. **Uzun ömürlü `calistir` aracısı gerekmiyor (A6):** çağrı başına ~58 ms.

## 2d'den önce açık noktalar

- **A8 — Schannel kısıtlı belirteçte çalışmıyor.**
  - Belirti: `AcquireCredentialsHandle` `SEC_E_NO_CREDENTIALS` döndürüyor. Aynı kullanıcı kısıtsız
    belirteçle bağlanıyor.
  - Kümeye Authenticated Users, INTERACTIVE, LOCAL, konsol oturumu, yerel hesap, oturum ya da
    kullanıcı SID'ini eklemek düzeltmedi. Ayrıcalıkları korumak da düzeltmedi.
  - Etkilenenler: Schannel kullanan araçlar. cargo (crates.io), rustup, curl ve Schannel arka uçlu
    git bağlanamıyor.
  - Etkilenmeyenler: OpenSSL kullananlar. git (`http.sslBackend=openssl`), node, npm ve python HTTPS'e
    bağlanıyor.
  - Seçenekler:
    - Ağlı çalıştırmada git'e OpenSSL arka ucunu vermek; cargo için ayrı bir yol bulmak. Örneğin
      bağımlılıkları geliştiricinin önbelleğinden salt okunur kullanmak; P9 bunun çalıştığını
      gösterdi.
    - Ağlı çalıştırma için AppContainer'ı denemek.
    - Ağlı çalıştırmayı kısıtlı belirteçsiz yapmak. Bu, çalışma alanları arasındaki yazma ayrımını
      ağlı çalıştırmada kaybettirir.
- **A9 — Geri döngü açık.**
  - Windows güvenlik duvarı geri döngü trafiğini süzmüyor. Ağsız kullanıcı 127.0.0.1'deki
    servislere ulaşıyor: geliştirme sunucuları, yerel veritabanları, Docker'ın TCP ucu.
  - Seçenek: WFP ile doğrudan bir geri döngü filtresi (2d'de prototiplenir). Olmazsa bilinen sınır
    olarak belgelenir.
- **A10 — Varsayılan izinli adlandırılmış borular salt okunur açılıyor.**
  - Varsayılan boru DACL'i Everyone'a okuma verir; Everyone kümede olmak zorunda (`NUL`).
  - Geliştiricinin süreçlerinin varsayılan izinle açtığı borulara yalıtım salt okunur bağlanabiliyor
    ve sunucunun yazdığını okuyabiliyor. Okuma-yazma bağlantısı reddediliyor.
  - `openssh-ssh-agent` ve `docker_engine` bu makinede yoktu; ölçülemedi.

Ayrıca: cargo, çalışma alanının üst dizinini okuyamadığında "could not canonicalize path" uyarısı
veriyor; iş etkilenmiyor.
