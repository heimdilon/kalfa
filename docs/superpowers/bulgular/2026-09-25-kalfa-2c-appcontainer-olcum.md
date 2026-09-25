# 2c AppContainer ek ölçümü (ham çıktı, 2026-09-25)

`kalfa-prototip kap olc` koşuları. Özet ve yorum:
[bulgular](2026-09-25-kalfa-2c-appcontainer.md). Komut çıktıları cmd'nin kod
sayfasıyla geldiği için Türkçe harfler bozuk görünebilir ("Eri�im engellendi").

## Tur 2: C:\ kök izniyle, korumalı yollar yalnızca izinle

- Alan A paket SID'i: `S-1-15-2-4242376595-3535215319-724918689-562824294-4273896609-4013993638-2039652301`
- Alan B paket SID'i: `S-1-15-2-3799135096-1909273654-2316338399-2669410182-2794661744-2501220789-2227987418`
- Araç zinciri yeteneği: `S-1-15-3-1024-3975571423-2778284315-984715428-3605687159-2177129449-3039832728-3726870227-2501160411`
- Yalıtımın PATH'i: `C:\Windows\System32;C:\Windows;C:\Windows\System32\Wbem;C:\Windows\System32\WindowsPowerShell\v1.0;C:\Program Files\Git\mingw64\bin;C:\Program Files\nodejs;C:\Python314;C:\Users\kullanici\.cargo\bin`
- Sürücü kipi: `Y:` → `C:\KalfaPrototip`
- P9 MSVC ortamı: `PATH=C:\Windows\System32;C:\Windows;C:\Windows\System32\Wbem;C:\Windows\System32\WindowsPowerShell\v1.0;C:\Program Files\Git\mingw64\bin;C:\Program Files\nodejs;C:\Python314;C:\Users\kullanici\.cargo\bin;C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.51.36231\bin\Hostx64\x64`, `LIB=C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.51.36231\lib\x64;C:\Program Files (x86)\Windows Kits\10\Lib\10.0.26100.0\um\x64;C:\Program Files (x86)\Windows Kits\10\Lib\10.0.26100.0\ucrt\x64`
- P6 ilk çağrı: 32 ms
- P6 sonraki 10 çağrı: en az 22 / ortanca 24 / en çok 29 ms
- P10 geliştirici penceresine gelen ileti sayısı: 0
- P7 sonrası: .git/config "[core]", pre-commit yerinde, hooks yerinde

| Kod | Senaryo | Beklenen | Sonuç | |
| --- | --- | --- | --- | --- |
| p1-sistem | hosts dosyasını okuma | okunur | 0: TAMAM (31 ms) | ✔ |
| p1-program | Program Files listeleme | okunur | 0: TAMAM (31 ms) | ✔ |
| p1-projeler-oku | C:\projeler benzeri klasörde dosya okuma | okunmaz | 1: Eri�im engellendi. (29 ms) | ✔ |
| p1-projeler-liste | C:\projeler benzeri klasörü listeleme | listelenmez | 1: File Not Found (27 ms) | ✔ |
| p1-liste-kok | C:\ listeleme | ölçüm | 0: AMD / appverifUI.dll / inetpub (27 ms) | · |
| p1-liste-kullanicilar | C:\Users listeleme | ölçüm | 1: File Not Found (29 ms) | · |
| p1-liste-public | C:\Users\Public listeleme | ölçüm | 1: File Not Found (29 ms) | · |
| p1-liste-programdata | C:\ProgramData listeleme | ölçüm | 1: File Not Found (28 ms) | · |
| p1-alan | kendi çalışma alanında okuma ve yazma (Low etiketli) | çalışır | 0: ALAN-a / TAMAM (30 ms) | ✔ |
| p1-etiketsiz-oku | izinli ama bütünlük etiketsiz dizinde okuma | ölçüm | 0: ETIKETSIZ (30 ms) | · |
| p1-etiketsiz-yaz | izinli ama bütünlük etiketsiz dizine yazma | ölçüm (Low bütünlük) | 0: TAMAM (30 ms) | · |
| p1-lpac-sistem | LPAC: sistem | ölçüm | 0: TAMAM (27 ms) | · |
| p1-lpac-program | LPAC: program | ölçüm | 0: TAMAM (25 ms) | · |
| p2-yol-cmd | cmd (yol) | çalışır | 0: Microsoft Windows [Version 10.0.26200.9457] (28 ms) | ✔ |
| p2-yol-cmd-dir | cmd-dir (yol) | çalışır | 0: .env / .git / bagimli (29 ms) | ✔ |
| p2-yol-cmd-cd | cmd-cd (yol) | çalışır | 0: C:\KalfaPrototip\alan-a\d-cd (32 ms) | ✔ |
| p2-yol-powershell | powershell (yol) | çalışır | 0: 5 (171 ms) | ✔ |
| p2-yol-powershell-dil | powershell-dil (yol) | FullLanguage | 0: FullLanguage (205 ms) | ✔ |
| p2-yol-git | git (yol) | çalışır | 0: git version 2.55.0.windows.5 (67 ms) | ✔ |
| p2-yol-git-depo | git-depo (yol) | çalışır | 0: c6acbd8 ilk (214 ms) | ✔ |
| p2-yol-node | node (yol) | çalışır | 0: v24.20.0 (59 ms) | ✔ |
| p2-yol-npm | npm (yol) | çalışır | 0: 11.19.0 (344 ms) | ✔ |
| p2-yol-python | python (yol) | çalışır | 0: 3.14.7 (90 ms) | ✔ |
| p2-yol-cargo | cargo (yol) | çalışır | 0: cargo 1.98.1 (797e8a9bc 2026-08-05) (87 ms) | ✔ |
| p2-yol-rustc | rustc (yol) | çalışır | 0: rustc 1.98.1 (48a229cea 2026-09-01) (87 ms) | ✔ |
| p2-surucu-cmd | cmd (surucu) | çalışır | 0: Microsoft Windows [Version 10.0.26200.9457] (27 ms) | ✔ |
| p2-surucu-cmd-dir | cmd-dir (surucu) | çalışır | 0: .env / .git / bagimli (27 ms) | ✔ |
| p2-surucu-cmd-cd | cmd-cd (surucu) | çalışır | 0: Y:\alan-a\d-cd (26 ms) | ✔ |
| p2-surucu-powershell | powershell (surucu) | çalışır | 0: 5 (173 ms) | ✔ |
| p2-surucu-powershell-dil | powershell-dil (surucu) | FullLanguage | 0: FullLanguage (170 ms) | ✔ |
| p2-surucu-git | git (surucu) | çalışır | 0: git version 2.55.0.windows.5 (53 ms) | ✔ |
| p2-surucu-git-depo | git-depo (surucu) | çalışır | 0: 235c82a ilk (223 ms) | ✔ |
| p2-surucu-node | node (surucu) | çalışır | 0: v24.20.0 (50 ms) | ✔ |
| p2-surucu-npm | npm (surucu) | çalışır | 0: 11.19.0 (294 ms) | ✔ |
| p2-surucu-python | python (surucu) | çalışır | 0: 3.14.7 (74 ms) | ✔ |
| p2-surucu-cargo | cargo (surucu) | çalışır | 0: cargo 1.98.1 (797e8a9bc 2026-08-05) (78 ms) | ✔ |
| p2-surucu-rustc | rustc (surucu) | çalışır | 0: rustc 1.98.1 (48a229cea 2026-09-01) (71 ms) | ✔ |
| p2-arac-yok-cmd | cmd (arac-yok) | çalışır | 0: Microsoft Windows [Version 10.0.26200.9457] (24 ms) | ✔ |
| p2-arac-yok-cmd-dir | cmd-dir (arac-yok) | çalışır | 1: Eri�im engellendi. (26 ms) | ✖ |
| p2-arac-yok-cmd-cd | cmd-cd (arac-yok) | çalışır | 0: Y:\alan-a\d-cd (25 ms) | ✔ |
| p2-arac-yok-powershell | powershell (arac-yok) | çalışır | 0: 5 (185 ms) | ✔ |
| p2-arac-yok-powershell-dil | powershell-dil (arac-yok) | FullLanguage | 0: FullLanguage (183 ms) | ✔ |
| p2-arac-yok-git | git (arac-yok) | çalışır | 0: git version 2.55.0.windows.5 (51 ms) | ✔ |
| p2-arac-yok-git-depo | git-depo (arac-yok) | çalışır | 128: fatal: unable to get current working directory: Permission denied (55 ms) | ✖ |
| p2-arac-yok-node | node (arac-yok) | çalışır | 1: 'node' is not recognized as an internal or external command, / operable program or batch file. (25 ms) | ✖ |
| p2-arac-yok-npm | npm (arac-yok) | çalışır | 1: 'npm' is not recognized as an internal or external command, / operable program or batch file. (27 ms) | ✖ |
| p2-arac-yok-python | python (arac-yok) | çalışır | 1: 'python' is not recognized as an internal or external command, / operable program or batch file. (26 ms) | ✖ |
| p2-arac-yok-cargo | cargo (arac-yok) | çalışır | 1: 'cargo' is not recognized as an internal or external command, / operable program or batch file. (43 ms) | ✖ |
| p2-arac-yok-rustc | rustc (arac-yok) | çalışır | 1: 'rustc' is not recognized as an internal or external command, / operable program or batch file. (28 ms) | ✖ |
| p2-lpac-cmd | cmd (lpac) | çalışır | 0: Microsoft Windows [Version 10.0.26200.0] (26 ms) | ✔ |
| p2-lpac-cmd-dir | cmd-dir (lpac) | çalışır | 0: .env / .git / bagimli (27 ms) | ✔ |
| p2-lpac-cmd-cd | cmd-cd (lpac) | çalışır | 0: Y:\alan-a\d-cd (27 ms) | ✔ |
| p2-lpac-powershell | powershell (lpac) | çalışır | 4294639616: Encountered a problem reading the registry.  Cannot open registry key SOFTWARE\Microsoft\PowerShell.  System error: /   Eri_im engellendi. / . (24 ms) | ✖ |
| p2-lpac-powershell-dil | powershell-dil (lpac) | FullLanguage | 4294639616: Encountered a problem reading the registry.  Cannot open registry key SOFTWARE\Microsoft\PowerShell.  System error: /   Eri_im engellendi. / . (25 ms) | ✖ |
| p2-lpac-git | git (lpac) | çalışır | 1: Erişim engellendi. (33 ms) | ✖ |
| p2-lpac-git-depo | git-depo (lpac) | çalışır | 1: Erişim engellendi. (38 ms) | ✖ |
| p2-lpac-node | node (lpac) | çalışır | 1: Erişim engellendi. (33 ms) | ✖ |
| p2-lpac-npm | npm (lpac) | çalışır | 1:  (28 ms) | ✖ |
| p2-lpac-python | python (lpac) | çalışır | 1: Erişim engellendi. (34 ms) | ✖ |
| p2-lpac-cargo | cargo (lpac) | çalışır | 1: Erişim engellendi. (34 ms) | ✖ |
| p2-lpac-rustc | rustc (lpac) | çalışır | 1: Erişim engellendi. (40 ms) | ✖ |
| p2-lpac-kayit-cmd | cmd (lpac-kayit) | çalışır | 0: Microsoft Windows [Version 10.0.26200.9457] (29 ms) | ✔ |
| p2-lpac-kayit-cmd-dir | cmd-dir (lpac-kayit) | çalışır | 0: .env / .git / bagimli (26 ms) | ✔ |
| p2-lpac-kayit-cmd-cd | cmd-cd (lpac-kayit) | çalışır | 0: Y:\alan-a\d-cd (28 ms) | ✔ |
| p2-lpac-kayit-powershell | powershell (lpac-kayit) | çalışır | 4294901760: Windows PowerShell terminated with the following error:  /  'System.Management.Automation.Tracing.PSEtwLog' t�r ba_lat1c1s1 �zel durum d�nd�rd�. /  (73 ms) | ✖ |
| p2-lpac-kayit-powershell-dil | powershell-dil (lpac-kayit) | FullLanguage | 4294901760: Windows PowerShell terminated with the following error:  /  'System.Management.Automation.Tracing.PSEtwLog' t�r ba_lat1c1s1 �zel durum d�nd�rd�. /  (68 ms) | ✖ |
| p2-lpac-kayit-git | git (lpac-kayit) | çalışır | 0: git version 2.55.0.windows.5 (56 ms) | ✔ |
| p2-lpac-kayit-git-depo | git-depo (lpac-kayit) | çalışır | 0: 8545072 ilk (222 ms) | ✔ |
| p2-lpac-kayit-node | node (lpac-kayit) | çalışır | 0: v24.20.0 (52 ms) | ✔ |
| p2-lpac-kayit-npm | npm (lpac-kayit) | çalışır | 0: 11.19.0 (285 ms) | ✔ |
| p2-lpac-kayit-python | python (lpac-kayit) | çalışır | 0: 3.14.7 (79 ms) | ✔ |
| p2-lpac-kayit-cargo | cargo (lpac-kayit) | çalışır | 1: warn: could not canonicalize path Y:\alan-a / warn: could not canonicalize path Y:\ / warn: could not canonicalize path Y:\alan-a (67 ms) | ✖ |
| p2-lpac-kayit-rustc | rustc (lpac-kayit) | çalışır | 1: warn: could not canonicalize path Y:\alan-a / warn: could not canonicalize path Y:\ / warn: could not canonicalize path Y:\alan-a (64 ms) | ✖ |
| p3-a-b-oku | A'nın kabından B'yi okuma | okunmaz | 1: Eri�im engellendi. (38 ms) | ✔ |
| p3-a-b-yaz | A'nın kabından B'ye yazma | yazılmaz | 1: Eri�im engellendi. (39 ms) | ✔ |
| p3-b-kendi | B'nin kabından kendi alanında okuma ve yazma | çalışır | 0: ALAN-b / TAMAM (27 ms) | ✔ |
| p3-b-a-oku | B'nin kabından A'yı okuma | okunmaz | 1: Eri�im engellendi. (31 ms) | ✔ |
| p3-gelistirici-sureci | geliştiricinin sürecinin yolunu okuma | okunmaz | 0: Get-Process : Cannot find a process with the process identifier 36544. / At line:1 char:2 / + (Get-Process -Id 36544).Path (436 ms) | ✔ |
| p4-profil | geliştiricinin profilini listeleme | listelenmez | 1: File Not Found (29 ms) | ✔ |
| p4-kanarya | profildeki dosyayı okuma | okunmaz | 1: Eri�im engellendi. (29 ms) | ✔ |
| p4-kimlik | geliştiricinin Credentials dizini | listelenmez | 1: File Not Found (28 ms) | ✔ |
| p4-ssh | profil\.ssh listeleme | ölçüm | 1: Eri�im engellendi. (29 ms) | · |
| p4-gitconfig | profil\.gitconfig listeleme | ölçüm | 1: Eri�im engellendi. (29 ms) | · |
| p4-cargo | profil\.cargo listeleme | ölçüm | 1: Eri�im engellendi. (29 ms) | · |
| p4-cargo-kimlik | profil\.cargo\credentials.toml listeleme | ölçüm | 1: Eri�im engellendi. (30 ms) | · |
| p4-claude | profil\.claude listeleme | ölçüm | 1: Eri�im engellendi. (30 ms) | · |
| p4-roaming | profil\AppData\Roaming listeleme | ölçüm | 1: File Not Found (29 ms) | · |
| p4-locallow | profil\AppData\LocalLow listeleme | ölçüm | 1: File Not Found (29 ms) | · |
| p4-paketler | profil\AppData\Local\Packages listeleme | ölçüm | 1: File Not Found (29 ms) | · |
| p5-agsiz-https | yeteneksiz, dış HTTPS | bağlanmaz | 6: 000 (54 ms) | ✔ |
| p5-agsiz-http | yeteneksiz, dış HTTP | bağlanmaz | 6: 000 (46 ms) | ✔ |
| p5-agsiz-geri-dongu | yeteneksiz, 127.0.0.1 | bağlanmaz (A9) | 28: 000 (8036 ms) | ✔ |
| p5-agsiz-dns | yeteneksiz, ad çözme | ölçüm | 0: Server:  UnKnown / Address:  127.0.0.1 (48 ms) | · |
| p5-internet-https | internet: dış HTTPS (curl, Schannel) | bağlanır (A8) | 0: 200 (240 ms) | ✔ |
| p5-internet-geri-dongu | internet: 127.0.0.1 | ölçüm (A9) | 28: 000 (8032 ms) | · |
| p5-ozel-https | ozel: dış HTTPS (curl, Schannel) | bağlanır (A8) | 0: 200 (228 ms) | ✔ |
| p5-ozel-geri-dongu | ozel: 127.0.0.1 | ölçüm (A9) | 28: 000 (8028 ms) | · |
| p5-internet-http | internet: dış HTTP | bağlanır | 0: 200 (156 ms) | ✔ |
| p5-internet-python-https | internet: python ile HTTPS | bağlanır | 0: 200 (343 ms) | ✔ |
| p5-internet-git-openssl | internet: git ls-remote HTTPS (openssl) | bağlanır | 0: 14432a28e180ec717933714aed7828fc7ed2fd29HEAD (695 ms) | ✔ |
| p5-internet-git-schannel | internet: git ls-remote HTTPS (schannel) | bağlanır | 0: 14432a28e180ec717933714aed7828fc7ed2fd29HEAD (662 ms) | ✔ |
| p5-internet-cargo | internet: cargo search (crates.io) | bağlanır (A8) | 0: cfg-if = "1.0.5"    # A macro to ergonomically define an item depending on a large number of #[cfg] parameters. Structured like a… / ... and 91 crates more (use (992 ms) | ✔ |
| p5-lpac-internet-https | LPAC + internet: dış HTTPS | ölçüm | 2: curl: error initializing curl library (19 ms) | · |
| p5-boru-openssh-ssh-agent | \\.\pipe\openssh-ssh-agent | ölçüm | 0: MethodInvocationException: Exception calling "Connect" with "1" argument(s): "��lem zaman a��m�na u�rad�." (869 ms) | · |
| p5-boru-docker_engine | \\.\pipe\docker_engine | ölçüm | 0: MethodInvocationException: Exception calling "Connect" with "1" argument(s): "��lem zaman a��m�na u�rad�." (888 ms) | · |
| p5-boru-varsayilan-oku | geliştiricinin varsayılan izinli borusu (In) | açılmaz (A10) | 0: MethodInvocationException: Exception calling "Connect" with "1" argument(s): "Yola eri�im engellendi." (388 ms) | ✔ |
| p5-boru-varsayilan-oku-yaz | geliştiricinin varsayılan izinli borusu (InOut) | açılmaz (A10) | 0: MethodInvocationException: Exception calling "Connect" with "1" argument(s): "Yola eri�im engellendi." (387 ms) | ✔ |
| p7-kontrol | normal.txt'ye ekleme (karşılaştırma) | yazılır | 0:  (30 ms) | ✔ |
| p7-gizli | .env okuma | okunmaz | 1: Eri�im engellendi. (29 ms) | ✔ |
| p7-config-yaz | .git/config'e ekleme | yazılmaz | 1: Eri�im engellendi. (30 ms) | ✔ |
| p7-config-sil | .git/config'i silme | silinmez | 1: Y:\alan-a\.git\config (30 ms) | ✔ |
| p7-config-uzerine | .git/config'in üzerine taşıma | taşınmaz | 1: 0 file(s) moved. (29 ms) | ✔ |
| p7-hook-yeni | .git/hooks'a dosya ekleme | eklenmez | 1: Eri�im engellendi. (28 ms) | ✔ |
| p7-hook-sil | pre-commit'i silme | silinmez | 1: Y:\alan-a\.git\hooks\pre-commit (30 ms) | ✔ |
| p7-hooks-tasi | .git/hooks dizinini yeniden adlandırma | taşınmaz | 1: Eri�im engellendi. (29 ms) | ✔ |
| p7-git-tasi | .git dizinini yeniden adlandırma | taşınmaz | 1: Eri�im engellendi. (28 ms) | ✔ |
| p7-git-ic | .git içinde dosya oluşturma, yeniden adlandırma, silme | çalışır | 0: TAMAM (29 ms) | ✔ |
| p7-ret-deneme | paket SID'ine okuma reddi (eski yöntem) | ölçüm | 0: RET-DENEME (29 ms) | · |
| p9-kutu-msvcsiz | bağımlılıksız crate, MSVC ortamı verilmeden | ölçüm | 101: warn: could not canonicalize path Y:\alan-a\kutu / warn: could not canonicalize path Y:\alan-a / warn: could not canonicalize path Y:\ (240 ms) | · |
| p9-kutu-yol | bağımlılıksız crate, yol kipinde | ölçüm | 0: TAMAM (637 ms) | · |
| p9-kutu | bağımlılıksız crate, çalışma alanına özel CARGO_HOME | derlenir | 0: TAMAM (75 ms) | ✔ |
| p9-bagimli-ozel | bağımlı crate, çalışma alanına özel CARGO_HOME | derlenmez (önbellek yok) | 101: warn: could not canonicalize path Y:\alan-a\bagimli / warn: could not canonicalize path Y:\alan-a / warn: could not canonicalize path Y:\ (115 ms) | · |
| p9-bagimli-gelistirici | bağımlı crate, geliştiricinin CARGO_HOME'u salt okunur | ölçüm | 0: TAMAM (12423 ms) | · |
| p10-kisitli | UI kısıtlarıyla ileti, pano, pencere bulma | üçü de engellenir | 0: ileti=engellendi / pano=engellendi / pencere_bulma=buldu (101 ms) | · |
| p10-kisitsiz | UI kısıtları olmadan (karşılaştırma) | ölçüm | 0: ileti=engellendi / pano=engellendi / pencere_bulma=buldu (19 ms) | · |


### ALL APPLICATION PACKAGES'a açıkça okuma veren dizinler

Profil 3 düzey, `C:\` 1 düzey; miras alınan izinler sayılmaz.

- `C:\Users\kullanici\.claude-science\runtime\0.1.48-release (S-1-15-2-1)`
- `C:\Users\kullanici\.claude-science\runtime\0.1.48-release\winsbx (S-1-15-2-1)`
- `C:\Program Files (S-1-15-2-1, S-1-15-2-2)`
- `C:\Program Files (x86) (S-1-15-2-1, S-1-15-2-2)`
- `C:\Windows (S-1-15-2-1, S-1-15-2-2)`

## Tur 1: kök izni yok, korumalı yollar ret izniyle

- Alan A paket SID'i: `S-1-15-2-4242376595-3535215319-724918689-562824294-4273896609-4013993638-2039652301`
- Alan B paket SID'i: `S-1-15-2-3799135096-1909273654-2316338399-2669410182-2794661744-2501220789-2227987418`
- Araç zinciri yeteneği: `S-1-15-3-1024-3975571423-2778284315-984715428-3605687159-2177129449-3039832728-3726870227-2501160411`
- Yalıtımın PATH'i: `C:\Windows\System32;C:\Windows;C:\Windows\System32\Wbem;C:\Windows\System32\WindowsPowerShell\v1.0;C:\Program Files\Git\mingw64\bin;C:\Program Files\nodejs;C:\Python314;C:\Users\kullanici\.cargo\bin`
- Sürücü kipi: `Y:` → `C:\KalfaPrototip`
- P6 ilk çağrı: 31 ms
- P6 sonraki 10 çağrı: en az 22 / ortanca 25 / en çok 28 ms
- P10 geliştirici penceresine gelen ileti sayısı: 0
- P7 sonrası: .git/config "y", pre-commit SİLİNDİ, hooks TAŞINDI

| Kod | Senaryo | Beklenen | Sonuç | |
| --- | --- | --- | --- | --- |
| p1-sistem | hosts dosyasını okuma | okunur | 0: TAMAM (31 ms) | ✔ |
| p1-program | Program Files listeleme | okunur | 1: Eri�im engellendi. (30 ms) | ✖ |
| p1-projeler-oku | C:\projeler benzeri klasörde dosya okuma | okunmaz | 1: Eri�im engellendi. (30 ms) | ✔ |
| p1-projeler-liste | C:\projeler benzeri klasörü listeleme | listelenmez | 1: Eri�im engellendi. (28 ms) | ✔ |
| p1-liste-kok | C:\ listeleme | ölçüm | 1: Eri�im engellendi. (29 ms) | · |
| p1-liste-kullanicilar | C:\Users listeleme | ölçüm | 1: Eri�im engellendi. (32 ms) | · |
| p1-liste-public | C:\Users\Public listeleme | ölçüm | 1: Eri�im engellendi. (28 ms) | · |
| p1-liste-programdata | C:\ProgramData listeleme | ölçüm | 1: Eri�im engellendi. (28 ms) | · |
| p1-alan | kendi çalışma alanında okuma ve yazma (Low etiketli) | çalışır | 0: ALAN-a / TAMAM (33 ms) | ✔ |
| p1-etiketsiz-oku | izinli ama bütünlük etiketsiz dizinde okuma | ölçüm | 0: ETIKETSIZ (33 ms) | · |
| p1-etiketsiz-yaz | izinli ama bütünlük etiketsiz dizine yazma | ölçüm (Low bütünlük) | 0: TAMAM (29 ms) | · |
| p1-lpac-sistem | LPAC: sistem | ölçüm | 0: TAMAM (24 ms) | · |
| p1-lpac-program | LPAC: program | ölçüm | 1: Erişim engellendi. (25 ms) | · |
| p2-yol-cmd | cmd (yol) | çalışır | 0: Microsoft Windows [Version 10.0.26200.9457] (28 ms) | ✔ |
| p2-yol-cmd-dir | cmd-dir (yol) | çalışır | 1: Eri�im engellendi. (28 ms) | ✖ |
| p2-yol-cmd-cd | cmd-cd (yol) | çalışır | 0: C:\KalfaPrototip\alan-a\d-cd (29 ms) | ✔ |
| p2-yol-powershell | powershell (yol) | çalışır | 0: 5 (238 ms) | ✔ |
| p2-yol-powershell-dil | powershell-dil (yol) | FullLanguage | 0: FullLanguage (167 ms) | ✔ |
| p2-yol-git | git (yol) | çalışır | 0: git version 2.55.0.windows.5 (50 ms) | ✔ |
| p2-yol-git-depo | git-depo (yol) | çalışır | 0: 2ed3985 ilk (180 ms) | ✔ |
| p2-yol-node | node (yol) | çalışır | 0: v24.20.0 (506 ms) | ✔ |
| p2-yol-npm | npm (yol) | çalışır | 1: node:fs:2806 / const out = binding.lstat(base, false, undefined, true /* throwIfNoEntry */); / ^ (141 ms) | ✖ |
| p2-yol-python | python (yol) | çalışır | 0: 3.14.7 (217 ms) | ✔ |
| p2-yol-cargo | cargo (yol) | çalışır | 0: cargo 1.98.1 (797e8a9bc 2026-08-05) (148 ms) | ✔ |
| p2-yol-rustc | rustc (yol) | çalışır | 0: rustc 1.98.1 (48a229cea 2026-09-01) (896 ms) | ✔ |
| p2-surucu-cmd | cmd (surucu) | çalışır | 0: Microsoft Windows [Version 10.0.26200.9457] (22 ms) | ✔ |
| p2-surucu-cmd-dir | cmd-dir (surucu) | çalışır | 0: .env / .git / bagimli (25 ms) | ✔ |
| p2-surucu-cmd-cd | cmd-cd (surucu) | çalışır | 0: Y:\alan-a\d-cd (23 ms) | ✔ |
| p2-surucu-powershell | powershell (surucu) | çalışır | 0: 5 (153 ms) | ✔ |
| p2-surucu-powershell-dil | powershell-dil (surucu) | FullLanguage | 0: FullLanguage (150 ms) | ✔ |
| p2-surucu-git | git (surucu) | çalışır | 0: git version 2.55.0.windows.5 (44 ms) | ✔ |
| p2-surucu-git-depo | git-depo (surucu) | çalışır | 0: 7e2b6d4 ilk (181 ms) | ✔ |
| p2-surucu-node | node (surucu) | çalışır | 0: v24.20.0 (45 ms) | ✔ |
| p2-surucu-npm | npm (surucu) | çalışır | 1: node:fs:2806 / const out = binding.lstat(base, false, undefined, true /* throwIfNoEntry */); / ^ (128 ms) | ✖ |
| p2-surucu-python | python (surucu) | çalışır | 0: 3.14.7 (68 ms) | ✔ |
| p2-surucu-cargo | cargo (surucu) | çalışır | 0: cargo 1.98.1 (797e8a9bc 2026-08-05) (62 ms) | ✔ |
| p2-surucu-rustc | rustc (surucu) | çalışır | 0: rustc 1.98.1 (48a229cea 2026-09-01) (58 ms) | ✔ |
| p2-arac-yok-cmd | cmd (arac-yok) | çalışır | 0: Microsoft Windows [Version 10.0.26200.9457] (22 ms) | ✔ |
| p2-arac-yok-cmd-dir | cmd-dir (arac-yok) | çalışır | 1: Eri�im engellendi. (21 ms) | ✖ |
| p2-arac-yok-cmd-cd | cmd-cd (arac-yok) | çalışır | 0: Y:\alan-a\d-cd (24 ms) | ✔ |
| p2-arac-yok-powershell | powershell (arac-yok) | çalışır | 0: 5 (173 ms) | ✔ |
| p2-arac-yok-powershell-dil | powershell-dil (arac-yok) | FullLanguage | 0: FullLanguage (162 ms) | ✔ |
| p2-arac-yok-git | git (arac-yok) | çalışır | 0: git version 2.55.0.windows.5 (44 ms) | ✔ |
| p2-arac-yok-git-depo | git-depo (arac-yok) | çalışır | 128: fatal: unable to get current working directory: Permission denied (50 ms) | ✖ |
| p2-arac-yok-node | node (arac-yok) | çalışır | 1: 'node' is not recognized as an internal or external command, / operable program or batch file. (22 ms) | ✖ |
| p2-arac-yok-npm | npm (arac-yok) | çalışır | 1: 'npm' is not recognized as an internal or external command, / operable program or batch file. (23 ms) | ✖ |
| p2-arac-yok-python | python (arac-yok) | çalışır | 1: 'python' is not recognized as an internal or external command, / operable program or batch file. (23 ms) | ✖ |
| p2-arac-yok-cargo | cargo (arac-yok) | çalışır | 1: 'cargo' is not recognized as an internal or external command, / operable program or batch file. (25 ms) | ✖ |
| p2-arac-yok-rustc | rustc (arac-yok) | çalışır | 1: 'rustc' is not recognized as an internal or external command, / operable program or batch file. (23 ms) | ✖ |
| p2-lpac-cmd | cmd (lpac) | çalışır | 0: Microsoft Windows [Version 10.0.26200.0] (23 ms) | ✔ |
| p2-lpac-cmd-dir | cmd-dir (lpac) | çalışır | 0: .env / .git / bagimli (24 ms) | ✔ |
| p2-lpac-cmd-cd | cmd-cd (lpac) | çalışır | 0: Y:\alan-a\d-cd (24 ms) | ✔ |
| p2-lpac-powershell | powershell (lpac) | çalışır | 4294639616: Encountered a problem reading the registry.  Cannot open registry key SOFTWARE\Microsoft\PowerShell.  System error: /   Eri_im engellendi. / . (22 ms) | ✖ |
| p2-lpac-powershell-dil | powershell-dil (lpac) | FullLanguage | 4294639616: Encountered a problem reading the registry.  Cannot open registry key SOFTWARE\Microsoft\PowerShell.  System error: /   Eri_im engellendi. / . (22 ms) | ✖ |
| p2-lpac-git | git (lpac) | çalışır | 1: Erişim engellendi. (29 ms) | ✖ |
| p2-lpac-git-depo | git-depo (lpac) | çalışır | 1: Erişim engellendi. (32 ms) | ✖ |
| p2-lpac-node | node (lpac) | çalışır | 1: Erişim engellendi. (30 ms) | ✖ |
| p2-lpac-npm | npm (lpac) | çalışır | 1:  (25 ms) | ✖ |
| p2-lpac-python | python (lpac) | çalışır | 1: Erişim engellendi. (29 ms) | ✖ |
| p2-lpac-cargo | cargo (lpac) | çalışır | 1: Erişim engellendi. (29 ms) | ✖ |
| p2-lpac-rustc | rustc (lpac) | çalışır | 1: Erişim engellendi. (30 ms) | ✖ |
| p2-lpac-kayit-cmd | cmd (lpac-kayit) | çalışır | 0: Microsoft Windows [Version 10.0.26200.9457] (23 ms) | ✔ |
| p2-lpac-kayit-cmd-dir | cmd-dir (lpac-kayit) | çalışır | 0: .env / .git / bagimli (22 ms) | ✔ |
| p2-lpac-kayit-cmd-cd | cmd-cd (lpac-kayit) | çalışır | 0: Y:\alan-a\d-cd (23 ms) | ✔ |
| p2-lpac-kayit-powershell | powershell (lpac-kayit) | çalışır | 4294901760: Windows PowerShell terminated with the following error:  /  'System.Management.Automation.Tracing.PSEtwLog' t�r ba_lat1c1s1 �zel durum d�nd�rd�. /  (71 ms) | ✖ |
| p2-lpac-kayit-powershell-dil | powershell-dil (lpac-kayit) | FullLanguage | 4294901760: Windows PowerShell terminated with the following error:  /  'System.Management.Automation.Tracing.PSEtwLog' t�r ba_lat1c1s1 �zel durum d�nd�rd�. /  (61 ms) | ✖ |
| p2-lpac-kayit-git | git (lpac-kayit) | çalışır | 0: git version 2.55.0.windows.5 (43 ms) | ✔ |
| p2-lpac-kayit-git-depo | git-depo (lpac-kayit) | çalışır | 0: 33dd6e8 ilk (178 ms) | ✔ |
| p2-lpac-kayit-node | node (lpac-kayit) | çalışır | 0: v24.20.0 (47 ms) | ✔ |
| p2-lpac-kayit-npm | npm (lpac-kayit) | çalışır | 1: node:fs:2806 / const out = binding.lstat(base, false, undefined, true /* throwIfNoEntry */); / ^ (132 ms) | ✖ |
| p2-lpac-kayit-python | python (lpac-kayit) | çalışır | 0: 3.14.7 (71 ms) | ✔ |
| p2-lpac-kayit-cargo | cargo (lpac-kayit) | çalışır | 1: warn: could not canonicalize path Y:\alan-a / warn: could not canonicalize path Y:\ / warn: could not canonicalize path Y:\alan-a (68 ms) | ✖ |
| p2-lpac-kayit-rustc | rustc (lpac-kayit) | çalışır | 1: warn: could not canonicalize path Y:\alan-a / warn: could not canonicalize path Y:\ / warn: could not canonicalize path Y:\alan-a (58 ms) | ✖ |
| p3-a-b-oku | A'nın kabından B'yi okuma | okunmaz | 1: Eri�im engellendi. (26 ms) | ✔ |
| p3-a-b-yaz | A'nın kabından B'ye yazma | yazılmaz | 1: Eri�im engellendi. (29 ms) | ✔ |
| p3-b-kendi | B'nin kabından kendi alanında okuma ve yazma | çalışır | 0: ALAN-b / TAMAM (27 ms) | ✔ |
| p3-b-a-oku | B'nin kabından A'yı okuma | okunmaz | 1: Eri�im engellendi. (27 ms) | ✔ |
| p3-gelistirici-sureci | geliştiricinin sürecinin yolunu okuma | okunmaz | 0: Get-Process : Cannot find a process with the process identifier 10776. / At line:1 char:2 / + (Get-Process -Id 10776).Path (544 ms) | ✔ |
| p4-profil | geliştiricinin profilini listeleme | listelenmez | 1: Eri�im engellendi. (28 ms) | ✔ |
| p4-kanarya | profildeki dosyayı okuma | okunmaz | 1: Eri�im engellendi. (27 ms) | ✔ |
| p4-kimlik | geliştiricinin Credentials dizini | listelenmez | 1: Eri�im engellendi. (28 ms) | ✔ |
| p4-ssh | profil\.ssh listeleme | ölçüm | 1: Eri�im engellendi. (29 ms) | · |
| p4-gitconfig | profil\.gitconfig listeleme | ölçüm | 1: Eri�im engellendi. (27 ms) | · |
| p4-cargo | profil\.cargo listeleme | ölçüm | 1: Eri�im engellendi. (28 ms) | · |
| p4-cargo-kimlik | profil\.cargo\credentials.toml listeleme | ölçüm | 1: Eri�im engellendi. (28 ms) | · |
| p4-claude | profil\.claude listeleme | ölçüm | 1: Eri�im engellendi. (28 ms) | · |
| p4-roaming | profil\AppData\Roaming listeleme | ölçüm | 1: Eri�im engellendi. (28 ms) | · |
| p4-locallow | profil\AppData\LocalLow listeleme | ölçüm | 1: Eri�im engellendi. (26 ms) | · |
| p4-paketler | profil\AppData\Local\Packages listeleme | ölçüm | 1: Eri�im engellendi. (27 ms) | · |
| p5-agsiz-https | yeteneksiz, dış HTTPS | bağlanmaz | 6: 000 (50 ms) | ✔ |
| p5-agsiz-http | yeteneksiz, dış HTTP | bağlanmaz | 6: 000 (31 ms) | ✔ |
| p5-agsiz-geri-dongu | yeteneksiz, 127.0.0.1 | bağlanmaz (A9) | 28: 000 (8036 ms) | ✔ |
| p5-agsiz-dns | yeteneksiz, ad çözme | ölçüm | 0: Server:  UnKnown / Address:  127.0.0.1 (45 ms) | · |
| p5-internet-https | internet: dış HTTPS (curl, Schannel) | bağlanır (A8) | 0: 200 (237 ms) | ✔ |
| p5-internet-geri-dongu | internet: 127.0.0.1 | ölçüm (A9) | 28: 000 (8034 ms) | · |
| p5-ozel-https | ozel: dış HTTPS (curl, Schannel) | bağlanır (A8) | 0: 200 (229 ms) | ✔ |
| p5-ozel-geri-dongu | ozel: 127.0.0.1 | ölçüm (A9) | 28: 000 (8034 ms) | · |
| p5-internet-http | internet: dış HTTP | bağlanır | 0: 200 (151 ms) | ✔ |
| p5-internet-python-https | internet: python ile HTTPS | bağlanır | 0: 200 (413 ms) | ✔ |
| p5-internet-git-openssl | internet: git ls-remote HTTPS (openssl) | bağlanır | 0: 14432a28e180ec717933714aed7828fc7ed2fd29HEAD (950 ms) | ✔ |
| p5-internet-git-schannel | internet: git ls-remote HTTPS (schannel) | bağlanır | 0: 14432a28e180ec717933714aed7828fc7ed2fd29HEAD (700 ms) | ✔ |
| p5-internet-cargo | internet: cargo search (crates.io) | bağlanır (A8) | 0: cfg-if = "1.0.5"    # A macro to ergonomically define an item depending on a large number of #[cfg] parameters. Structured like a… / ... and 91 crates more (use (1178 ms) | ✔ |
| p5-lpac-internet-https | LPAC + internet: dış HTTPS | ölçüm | 2: curl: error initializing curl library (20 ms) | · |
| p5-boru-openssh-ssh-agent | \\.\pipe\openssh-ssh-agent | ölçüm | 0: MethodInvocationException: Exception calling "Connect" with "1" argument(s): "��lem zaman a��m�na u�rad�." (896 ms) | · |
| p5-boru-docker_engine | \\.\pipe\docker_engine | ölçüm | 0: MethodInvocationException: Exception calling "Connect" with "1" argument(s): "��lem zaman a��m�na u�rad�." (870 ms) | · |
| p5-boru-varsayilan-oku | geliştiricinin varsayılan izinli borusu (In) | açılmaz (A10) | 0: MethodInvocationException: Exception calling "Connect" with "1" argument(s): "Yola eri�im engellendi." (360 ms) | ✔ |
| p5-boru-varsayilan-oku-yaz | geliştiricinin varsayılan izinli borusu (InOut) | açılmaz (A10) | 0: MethodInvocationException: Exception calling "Connect" with "1" argument(s): "Yola eri�im engellendi." (383 ms) | ✔ |
| p7-kontrol | normal.txt'ye ekleme (karşılaştırma) | yazılır | 0:  (27 ms) | ✔ |
| p7-gizli | .env okuma | okunmaz | 0: GIZLI-ENV (32 ms) | ✖ |
| p7-config-yaz | .git/config'e ekleme | yazılmaz | 0:  (31 ms) | ✖ |
| p7-config-sil | .git/config'i silme | silinmez | 0:  (27 ms) | ✖ |
| p7-config-uzerine | .git/config'in üzerine taşıma | taşınmaz | 0: 1 file(s) moved. (29 ms) | ✖ |
| p7-hook-yeni | .git/hooks'a dosya ekleme | eklenmez | 0:  (28 ms) | ✖ |
| p7-hook-sil | pre-commit'i silme | silinmez | 0:  (27 ms) | ✖ |
| p7-hooks-tasi | .git/hooks dizinini yeniden adlandırma | taşınmaz | 0:  (30 ms) | ✖ |
| p9-kutu-yol | bağımlılıksız crate, yol kipinde | ölçüm | 101: warn: could not canonicalize path C:\KalfaPrototip\alan-a\kutu / warn: could not canonicalize path C:\KalfaPrototip\alan-a / warn: could not canonicalize path C (402 ms) | · |
| p9-kutu | bağımlılıksız crate, çalışma alanına özel CARGO_HOME | derlenir | 101: warn: could not canonicalize path Y:\alan-a\kutu / warn: could not canonicalize path Y:\alan-a / warn: could not canonicalize path Y:\ (173 ms) | ✖ |
| p9-bagimli-ozel | bağımlı crate, çalışma alanına özel CARGO_HOME | derlenmez (önbellek yok) | 101: warn: could not canonicalize path Y:\alan-a\bagimli / warn: could not canonicalize path Y:\alan-a / warn: could not canonicalize path Y:\ (110 ms) | · |
| p9-bagimli-gelistirici | bağımlı crate, geliştiricinin CARGO_HOME'u salt okunur | ölçüm | 101: warn: could not canonicalize path Y:\alan-a\bagimli / warn: could not canonicalize path Y:\alan-a / warn: could not canonicalize path Y:\ (12045 ms) | · |
| p10-kisitli | UI kısıtlarıyla ileti, pano, pencere bulma | üçü de engellenir | 0: ileti=engellendi / pano=engellendi / pencere_bulma=buldu (98 ms) | · |
| p10-kisitsiz | UI kısıtları olmadan (karşılaştırma) | ölçüm | 0: ileti=engellendi / pano=engellendi / pencere_bulma=buldu (19 ms) | · |


### ALL APPLICATION PACKAGES'a açıkça okuma veren dizinler

Profil 3 düzey, `C:\` 1 düzey; miras alınan izinler sayılmaz.

- `C:\Users\kullanici\.claude-science\runtime\0.1.48-release (S-1-15-2-1)`
- `C:\Users\kullanici\.claude-science\runtime\0.1.48-release\winsbx (S-1-15-2-1)`
- `C:\Program Files (S-1-15-2-1, S-1-15-2-2)`
- `C:\Program Files (x86) (S-1-15-2-1, S-1-15-2-2)`
- `C:\Windows (S-1-15-2-1, S-1-15-2-2)`
