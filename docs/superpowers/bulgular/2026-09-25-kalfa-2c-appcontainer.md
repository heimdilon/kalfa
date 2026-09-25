# Kalfa 2c — AppContainer ek ölçümü: bulgular

**Tarih:** 2026-09-25
**Tasarım:** [Aşama 2 tasarımı](../specs/2026-09-24-kalfa-2-kabuk-yalitim-design.md) §9.
**Önceki ölçüm:** [kısıtlı belirteç prototipi](2026-09-25-kalfa-2c-windows-prototip.md), sonuç Dal B.
**Prototip:** [`prototip/windows-yalitim/`](../../../prototip/windows-yalitim/README.md), `kap` komutu
(`src/kap.rs`, `src/kap_olc.rs`, `kap-kur.ps1`, `kap-geri-al.ps1`). Ürün kodu değildir.
**Ham ölçüm:** [`2026-09-25-kalfa-2c-appcontainer-olcum.md`](2026-09-25-kalfa-2c-appcontainer-olcum.md), iki tur.

## Sonuç: AppContainer ile Dal A

Kısıtlı belirteç yolunda geçmeyen P1 AppContainer'da geçti. Kısıtlı belirteç yolunda açık kalan A8,
A9 ve A10 da AppContainer'da çözülüyor.

- **Okuma sınırı tasarımın hedefini taşıyor.** Okunabilenler: sistem, izinli araç zinciri ve kendi
  çalışma alanı. Okunamayanlar:
  - `C:\` altındaki varsayılan izinli klasör (`C:\projeler` benzeri);
  - `C:\Users`, `C:\Users\Public` ve `C:\ProgramData`;
  - geliştiricinin profili (`.ssh`, `.claude`, `.cargo`, `AppData`).
- **Araç zincirlerinin tamamı çalışıyor:** cmd (`dir`, `cd` dahil), PowerShell 5.1 (FullLanguage),
  git (`init`, `add`, `commit`, HTTPS), node, npm, python, cargo ve rustc. cargo derlemesi de
  çalışıyor.
- **Schannel çalışıyor (A8).** İnternet yeteneğiyle curl, git (iki TLS arka ucu), python ve
  `cargo search` HTTPS'e bağlanıyor.
- **Geri döngü kapalı (A9).** Yetenekli ya da yeteneksiz, 127.0.0.1'e bağlanılamıyor.
- **Varsayılan izinli borular açılamıyor (A10).**
- **Ayrı kullanıcı gerekmiyor.** Parola, oturum açma hakkı ve güvenlik duvarı kuralı da yok.
  Ağsızlık, ağ yeteneği verilmemesidir; DNS dahil hiçbir bağlantı kurulmuyor.
- **P6:** çağrı başına 22–29 ms. Kısıtlı belirteç yolunda 57–61 ms'ydi.

Bedeli dört madde:

1. **`C:\` köküne yayılmayan bir listeleme izni.**
   - git, cmd ve Node yolları üst dizinleri listeleyerek kanonikleştiriyor.
   - İzin araç zinciri yeteneğine verilir: listeleme, öznitelik okuma ve geçiş; miras yok.
   - Kurulumda bir kez, yönetici olarak verilir. Aynı desen bu makinenin kökünde başka iki yetenek
     SID'i için zaten vardı.
2. **Korumalı yollar ret izniyle çizilemiyor.** AppContainer, paket SID'ine verilen ret izinlerini
   uygulamıyor. Korumalı yollar mirası keserek ve yalnızca izinle çizilir (§ Korumalı yollar).
3. **MSVC ortamını Kalfa verir.** rustc, Visual Studio'nun `ProgramData`'daki kurulum kaydını
   okuyamadığı için `link.exe`'yi bulamıyor. `PATH` ve `LIB` geliştirici tarafında çözülüp verilince
   derleme çalışıyor.
4. **Yönetici sahipli araç zinciri dizinleri.** `C:\Python314` ve `C:\Program Files\nodejs`
   `ALL APPLICATION PACKAGES`'a açık değil. İzinleri kurulumda yönetici olarak verilir. Profildeki
   `.cargo` ve `.rustup` için yönetici gerekmez.

**Karar:** Windows arka ucu AppContainer'a göre yeniden yazıldı (aşama 2 tasarımı §9, üst belge
r3.5). Kısıtlı belirteç yolu (Dal B) yedek olarak §9.7'de. Açık noktalar aşağıda (A11–A13, A15).

## Eşleme

| Kısıtlı belirteç yolu (§9.1–§9.4) | AppContainer |
|---|---|
| `KalfaAgsiz` / `KalfaAgli` kullanıcıları, `CreateProcessWithLogonW` | Yok; geliştiricinin kendi hesabı |
| Çalışma alanı SID'i (`S-1-9-…`) | Çalışma alanına bir AppContainer profili; paket SID'i (`S-1-15-2-…`) |
| Ortak okuma SID'i | Addan türetilen bir yetenek SID'i (`S-1-15-3-1024-…`) |
| Kısıtlayıcı küme (Everyone, Users, RESTRICTED) | Yok; sistem dizinleri `ALL APPLICATION PACKAGES`'a zaten açık |
| Güvenlik duvarı kuralı | Ağlı çalıştırmada `internetClient` yeteneği; ağsızda yetenek yok |
| Varsayılan DACL | AppContainer belirtecinin kendi varsayılanı; iki kap birbirini açamıyor (P3) |
| Job Object, `SetErrorMode` | Aynı |

**Zincir:**

- `CreateProcessW`'ye `STARTUPINFOEXW` verilir.
- `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES` paket SID'ini ve yetenekleri taşır.
- `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` yalnızca üç standart tutamacı geçirir.
- Süreç askıda başlar, Job Object'e atanır ve sürdürülür.
- Aracı süreç yoktur.

## Ölçümler

İki tur koşuldu:

- **Tur 1:** kök izni yoktu; korumalı yollar ret izniyle çizildi.
- **Tur 2:** kök izni verildi; korumalı yollar yalnızca izinle çizildi.

Tablo tur 2'nin sonucudur; fark varsa tur 1 ayrıca yazılır.

| # | Soru | Sonuç |
|---|---|---|
| P1 | Sistem okunur, kullanıcı klasörleri okunmaz mı? | **Geçti.** hosts ve Program Files okunuyor. `C:\projeler` benzeri klasör, `C:\Users`, `C:\Users\Public` ve `C:\ProgramData` okunmuyor, listelenmiyor. Kök izniyle `C:\`'nin yalnızca üst düzey adları listeleniyor. |
| P2 | Araç zincirleri | **Hepsi çalışıyor.** Araç zinciri yeteneği yokken `.cargo\bin`, Python ve nodejs'teki araçlar bulunamıyor, git çalışma dizinini çözemiyor. |
| P3 | İki çalışma alanı birbirine erişemiyor mu? | **Geçti.** A, B'yi okuyamıyor ve B'ye yazamıyor; B de A'yı okuyamıyor. Kap, geliştiricinin sürecini açamıyor. |
| P4 | Profil ve kimlik bilgileri | **Geçti.** Profil, kanarya, `Credentials`, `.ssh`, `.gitconfig`, `.cargo`, `.cargo\credentials.toml`, `.claude`, `Roaming`, `LocalLow` ve `Packages` erişilemiyor. |
| P5 | Ağ | Yeteneksiz: HTTP, HTTPS, DNS ve 127.0.0.1 kapalı. `internetClient` ile dış HTTP ve HTTPS açık (Schannel dahil), 127.0.0.1 yine kapalı. `privateNetworkClientServer` eklemek geri döngüyü açmıyor. Geliştiricinin varsayılan izinli borusuna "erişim engellendi". |
| P6 | Başlatma gecikmesi | İlk çağrı 31–32 ms, sonrakiler 22–29 ms. |
| P7 | Gizli dosya ve korumalı yollar | **Yalnızca izinle geçti.** Tur 1'de ret izinleri uygulanmadı: `.env` okundu, `.git/config` değişti, `pre-commit` silindi, `hooks` taşındı. |
| P8 | Users grubu | Geçersiz: ayrı kullanıcı yok. |
| P9 | cargo derlemesi, ağsız | MSVC ortamı verilince bağımlılıksız crate derleniyor. Geliştiricinin `CARGO_HOME`'u salt okunur verilince bağımlı crate derleniyor. Çalışma alanına özel `CARGO_HOME` ile bağımlı crate derlenmiyor (önbellek yok). MSVC ortamı verilmezse `link.exe` bulunamıyor. |
| P10 | UI | Geliştiricinin penceresine ileti ve pano engelleniyor, UI kısıtları olmadan da (düşük bütünlük). `FindWindow` buluyor. |

### Tur 1: `C:\` kökü listelenemeyince

- **git:** çalışma dizinini çözemiyor ("unable to get current working directory"). Git for Windows,
  dizinin uzun adını üst dizinlerden okuyor.
- **cmd:** `dir` düşüyor; `cd` ise ancak mutlak yolla kullanılınca düşüyor.
- **npm:** düşüyor. Node'un `realpath`'i `C:\`'nin özniteliklerini okuyamıyor (`EPERM: lstat 'C:\'`).
- **Sürücü harfi:** Çalışma alanı bir sürücü harfine bağlanınca (`DefineDosDevice`, `subst` gibi) git
  ile cmd düzeldi. npm düzelmedi, çünkü Node kendi kurulum yolunu da kanonikleştiriyor. npm,
  `--preserve-symlinks-main` ile çalıştı.
- **Kök izni:** Kök izni verilince (tur 2) yol kipinde hepsi çalıştı; sürücü harfi gerekmedi.

### LPAC

`ALL APPLICATION PACKAGES` dışındaki AppContainer (LPAC) denendi.

- Yeteneksiz LPAC'ta cmd dışında hiçbir araç başlamıyor.
- `registryRead` ve `lpacCom` yetenekleriyle git, node, npm ve python çalışıyor.
- PowerShell çalışmıyor (`PSEtwLog` başlatılamıyor), cargo düşüyor ve curl başlatılamıyor.

LPAC kullanılmaz. Normal AppContainer'ın `ALL APPLICATION PACKAGES` üzerinden okuyabildikleri,
profilde ve `C:\` altında tarandı; aşağıda.

### `ALL APPLICATION PACKAGES`'a açıkça okuma veren yerler

Tarama profili 3 düzey, `C:\`'yi 1 düzey kapsadı ve miras alınan izinleri saymadı. Bulunan yerler:

- `C:\Program Files`, `C:\Program Files (x86)` ve `C:\Windows`;
- profilde yalnızca `.claude-science\runtime\…` (başka bir uygulamanın kendi yalıtımı için verdiği
  izin).

## Korumalı yollar

Ret izni yerine yalnızca izin:

- **`.env` (gizli dosyalar):** mirası kesilir; geliştirici, SYSTEM ve Administrators tam, pakete
  izin yok.
- **`.git`:**
  - Mirası kesilir.
  - Pakete `.git`'in kendisi için okuma ve yazma verilir, silme verilmez. Bu yüzden `.git` yeniden
    adlandırılamıyor. Yoksa kap, `.git`'i kenara çekip kendi kancalarıyla yeni bir `.git`
    kurabilirdi.
  - Pakete `.git`'in içi için değiştirme verilir. git'in işleyişi (dosya oluşturma, yeniden
    adlandırma, silme) çalışıyor.
- **`.git\hooks` ve `.git\config`:** mirası kesilir; pakete yalnızca okuma. Yazma, silme, üzerine
  taşıma ve yeniden adlandırma reddediliyor.

## Kurulumda dikkat

- **`C:\` kökünün DACL'i:**
  - Yalnızca kökü değiştiren bir API'yle yazılmalı (`SetFileSecurityW`).
  - .NET'in `SetAccessControl`'ü (`SetNamedSecurityInfo`) mirası bütün sürücüye yeniden yayar.
  - `SetFileSecurityW`, `SE_DACL_AUTO_INHERITED` bayrağını ancak `SE_DACL_AUTO_INHERIT_REQ` ile
    birlikte verilirse koruyor. İlk sürüm bu bayrağı düşürdü (`D:PAI` → `D:P`). `kap-geri-al.ps1`
    bayrağı geri koyar.
- **Bütünlük etiketi gerekmiyor.** AppContainer düşük bütünlükte çalışıyor. Buna rağmen izinli ama
  etiketsiz (orta bütünlüklü) dizine yazabildi.
- **PowerShell'in geçici dizini:** Geçici dizin yazılabilir değilken PowerShell ConstrainedLanguage
  kipinde açıldı (tur 1'de, ağaç kurulmadan). Çalışma alanına özel `TEMP` yazılabilir olunca
  FullLanguage.

## Tur 2'den sonraki denemeler

Deneme ağacı kaldırıldıktan sonra, kapların kendi profil klasörleriyle (`kap dene`):

- **Kabın içinden başka bir paket SID'iyle süreç başlatılamıyor.** Deneme şöyle kuruldu: A'nın kabı,
  prototipi B'nin paket SID'iyle çalıştırmaya çalıştı.
  - `CreateProcessW` `ERROR_INVALID_PARAMETER` (87) döndü.
  - Aynı paketle başlatma çalıştı. Çocuğu yeni bir Job Object'e atamak `ERROR_NOT_SUPPORTED` (50)
    verdi; çocuk dıştaki işte kalır.
  - Paket SID'i addan türetildiği için gizli değildir. Çalışma alanları arasındaki ayrım bu kurala
    dayanır, adın gizliliğine değil.
- **Windows, AppContainer süreçlerinde `LOCALAPPDATA`'yı değiştiriyor:** verilen değerin sonuna
  `\Packages\<profil adı>\AC` ekleniyor. `TEMP` değişmiyor. Kalfa bu dizini çalışma alanına özel `ev`
  dizininde önceden oluşturmalı.
- **Profiller yükseltilmemiş süreçten silinmeli.** Kullanıcı `kap-geri-al.ps1`'i yönetici olarak
  çalıştırdı; bundan sonra iki profil ve kayıt klasörü yerinde kalmıştı. Yükseltilmemiş
  `kalfa-prototip kap sil` profilleri sildi. Kök DACL'i ise `D:PAI` hâline döndü: `AI` bayrağının
  düzeltmesi köke uygulanınca da çalıştı.

## Açık noktalar

- **A11 — Çalışma alanının üst dizinleri.**
  - Ölçülen ağaçta çalışma alanının üst dizinleri `C:\` (kök izni) ve `C:\KalfaPrototip` idi.
    `C:\KalfaPrototip`'te de yeteneğe listeleme izni vardı.
  - Profil altındaki bir çalışma alanında (ör. `C:\Users\<ad>\projeler\x`) her üst dizine listeleme
    izni gerekebilir. Bu, profil kökündeki adları açar; içerikleri açmaz.
  - Ölçülmedi.
  - Seçenekler: üst dizinlere listeleme izni vermek ya da çalışma alanını bir sürücü harfinden
    açmak. Sürücü harfi git ve cmd'yi düzeltti; Node'un kendi yolu için kök izni yine gerekiyor.
- **A12 — C derleyen `build.rs`** (`cc` crate'i: `cl.exe` ve `INCLUDE`) denenmedi.
- **A13 — Ölçülmeyen ağ hedefleri:**
  - `privateNetworkClientServer` ile bir LAN hedefi denenmedi.
  - `openssh-ssh-agent` ve `docker_engine` bu makinede yok.
- **A15 — Ağsız derlemede geliştiricinin cargo registry'si.** Windows'ta dizin bağlanamaz.
  Geliştiricinin `CARGO_HOME`'u salt okunur verilince bağımlı crate derlendi ama 12 sn sürdü.
  Muhtemel neden kilit beklemesi; ölçülmedi.
