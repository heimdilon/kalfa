# Kalfa 2d — Windows yalıtımı: ayrı kullanıcı mı, AppContainer mı

**Tarih:** 2026-09-27
**Soru:** 2d-1'de AppContainer'ın çıkardığı izin sorunları, aşama 2 tasarımının
§9.7'sindeki yedek yolla (ayrı yerel kullanıcı ve kısıtlı belirteç) azalır mı?
**Girdiler:** 2c'nin iki bulgu raporu
([kısıtlı belirteç](2026-09-25-kalfa-2c-windows-prototip.md),
[AppContainer](2026-09-25-kalfa-2c-appcontainer.md)), 2d-1'in bulguları
(`docs/GELISTIRME_DURUMU.md`) ve bu raporun ölçümü (aşağıda).

## Sonuç

**Öneri: AppContainer'da kalınsın.** Ayrı kullanıcı yolu izin işini azaltmıyor:
o da ACL'lerle çiziliyor ve A11 onda da var. İki sorunu çözüyor (A15 sınıfı ve
Node/Python izinleri), ama karşılığında cargo'nun ağını (A8), okuma sınırını
(Dal B) ve ağ kısıtını (A9, A10) kaybettiriyor. Yeni bir crates.io bağımlılığı
iki yolda da yalıtımın içinde inmiyor; yalnızca kırıldığı yer farklı.

## Karar (kullanıcı, 2026-09-27)

Hedef: Kalfa'yı teknik olmayan biri kolayca kurabilmeli. Bu gözle:

1. **AppContainer kesinleşti.** Ayrı kullanıcı yolu gizli hesaplar, parolalar
   ve güvenlik duvarı kuralı ister; bozulduğunda son kullanıcı onaramaz.
2. **Araç zincirini Kalfa kendisi getirir** (Python, Node, Git). Kullanıcının
   kurduğu araçlar gelişmiş ayardır.
3. **Rust ve MSVC gelişmiş özelliktir;** varsayılan kurulum Visual Studio
   istemez.
4. **Yalıtım kurulamayan makinede kabuk kapalıdır** ve nedeni söylenir.
   Kullanıcıya "BT departmanınızla görüşün" denir; BT'nin sorunu anlaması için
   kopyalanabilir teknik ayrıntı da verilir. Yalıtımsız çalıştırma yalnızca
   Ayarlar'dan bilerek açılır.

Tasarıma işlenişi: aşama 2 tasarımı §9.8 ve §15.

## Ölçütler

| Ölçüt | AppContainer (2d-1, birleşti) | Ayrı kullanıcı (§9.7) | Lehine |
|---|---|---|---|
| A11: üst dizin listeleme | Gerekli. Hazırlık üst dizinlere paket SID'ine mirassız listeleme verir; `C:\Users`'a kurulum verir | Aynı sorun: ayrı kullanıcı geliştiricinin profilini listeleyemez (P4); 2c'de cargo, `C:\KalfaPrototip`'i okuyamayınca "could not canonicalize" uyardı. İzin iki SID'e verilir (kullanıcı ve kısıtlayıcı); `C:\Users` için yönetici gerekmez | Berabere |
| A15: DOS yol adı | Çözülemiyor. cargo paketi açamıyor; Python `realpath` sessizce yanlış yol döndürüyor; Node `realpathSync.native` EPERM (bugün) | Çözülüyor; cargo paketi açıyor (bugün) | Ayrı kullanıcı |
| cargo ve crates.io | İndirme çalışıyor (Schannel), açma düşüyor | Schannel `SEC_E_NO_CREDENTIALS`; bugün geliştiricinin kendi kısıtlı belirtecinde de yinelendi. cargo, rustup, curl ve Schannel'li git bağlanamaz | AppContainer (az farkla) |
| Node ve Python dizinleri | `izin-ver` (UAC, tek sefer) | Users okuyabildiği için gerekmez | Ayrı kullanıcı (küçük) |
| `.cargo\bin`, `.rustup` | Yükseltmesiz izin | Yükseltmesiz izin | Berabere |
| git, npm, PowerShell, cmd | Çalışıyor | Çalışıyor; git'in HTTPS'i için OpenSSL arka ucu şart | Berabere |
| Çalışma alanları arası ayrım | Tutuyor (P3) | Tutuyor (P3) | Berabere |
| Profil ve kimlik bilgileri | Korunuyor (P4) | Korunuyor (P4) | Berabere |
| Okuma sınırı | Dal A: yalnızca uygulama paketlerine açık yerler | Dal B: Users'ın okuyabildiği her klasör; profil dışındaki başka projeler (`C:\projeler`, `D:\…`) okunur | AppContainer (büyük) |
| Gizli dosya, korumalı yol | Tutuyor; yalnızca miras keserek (ret işlemiyor) | Tutuyor; ret izni de işliyor | Ayrı kullanıcı (küçük) |
| Ağsız kip, geri döngü, borular | Kapalı, kapalı, açılamıyor; güvenlik duvarı kuralı yok | Güvenlik duvarı kuralı gerekir; geri döngü açık (A9); borular salt okunur (A10) | AppContainer |
| Yönetici adımları | `kur` (sürücü kökleri, `C:\Users`), `izin-ver` | İki gizli yerel kullanıcı, saklanan parolalar, oturum hakları, güvenlik duvarı kuralı, araç zinciri izinleri | AppContainer |
| Başlatma | 22–29 ms | 57–61 ms (`CreateProcessWithLogonW` ve aracı) | AppContainer |
| Kod | Birleşti (2d-1) | Windows arka ucu ve yardımcının `calistir` aracısı yeniden yazılır | AppContainer |

## AppContainer'da kalmanın bedeli (2d-2)

1. **A15, cargo:** kap geliştiricinin registry'sini salt okunur kullanır
   (2d-1'de çalıştı ama 12 sn sürdü, §14 A15; `CARGO_NET_OFFLINE` ile
   yeniden ölçülmeli). Eksik bağımlılığı geliştirici yalıtım dışında
   `cargo fetch` ile indirir. Bunu Kalfa'nın yerel onayla yalıtımsız çalıştırması
   yeni bir ürün kararıdır: yalıtımsız yerel onay yolu bugün yalnızca yalıtım
   yokken kullanılıyor (§5.1). Kalfa'nın paketi kendisinin açması önerilmez:
   çalışma alanının `.cargo/config.toml`'u ve kabın yazdığı `.crate` dosyaları
   geliştiricinin yetkisiyle işlenirdi.
2. **A15 sınıfı, öteki araçlar:** DOS yol adını isteyen her araç kapta düşer ya
   da yanlış sonuç alır. 2d-2'nin uyumluluk testlerine Python (`pip`, `venv`,
   `pytest`) ve `realpathSync.native` kullanan Node araçları girmeli; sonuç
   §9.6'ya yazılır.
3. **A13** hâlâ ölçülmedi.

## Bu raporun ölçümü

Yönetici gerekmedi, ACL değiştirilmedi. Ayrı kullanıcı yolunun kısıtlı belirteci
2c prototipinin `calistir` alt komutuyla, **geliştiricinin kendi hesabından**
türetildi. Kısıtlayıcı küme Everyone, Users ve RESTRICTED. Geliştiricinin
hesabında süreçler bu kümeyle, oturum SID'i eklense de başlamıyor (0xC0000142);
2c'de ayrı kullanıcıyla başlıyordu. Bu yüzden kısıtlı belirtecin etkisi
kümeye geliştiricinin SID'i eklenerek ölçüldü: ACL'ler geliştiricininki gibi
davranır, farkı yalnızca belirtecin kısıtlı olması yaratır. AppContainer tarafı
ürünün uyumluluk düzeneğinde (`kalfa-tools/tests/uyumluluk.rs`'e geçici,
commit'lenmeyen bir testle) ölçüldü; profil ve ACE kaydı temizlendi.

| Yoklama | Kısıtsız | Kısıtlı belirteç (+ geliştirici SID'i) | AppContainer |
|---|---|---|---|
| Python `nt._getfinalpathname` (DOS) | Çalıştı | Çalıştı | `WinError 5` |
| Python `os.path.realpath(strict=True)` | Çalıştı | Çalıştı | `WinError 5` |
| Python `os.path.realpath` | — | — | Yanlış yol (son bileşen iki kez); açılışta "Failed to find real location of python.exe" |
| Node `realpathSync.native` | Çalıştı | Çalıştı | `EPERM` |
| Node `realpathSync` (JS) | Çalıştı | Çalıştı | Çalıştı |
| `curl.exe` (Schannel), crates.io | 200 | `SEC_E_NO_CREDENTIALS` | Çalışıyor (2c) |
| `cargo build --offline`, `.crate` önbellekte, `src` boş (paket açma) | Çalıştı | Çalıştı | Düşüyor (A15, 2d-1) |
| `cargo fetch`, boş registry, ağlı | Çalıştı | `SEC_E_NO_CREDENTIALS` (11,6 sn) | İndiriyor, açarken düşüyor (2d-1) |

**Sınır:** A15'in ayrı kullanıcı tarafı ayrı kullanıcıyla değil, kısıtlı
belirteçle ölçüldü. Mount Manager'ın izni belirli bir kullanıcıya verilmediğinden
sonucun aynı olması beklenir. Kesinlik gerekirse 2c'nin `kur.ps1` /
`geri-al.ps1` betikleriyle (yönetici) ayrı kullanıcıda yinelenebilir.
