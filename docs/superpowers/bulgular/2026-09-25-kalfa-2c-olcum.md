# 2c prototip ölçümü (ham çıktı, 2026-09-25)

`kalfa-prototip olc` son koşusu. Özet ve yorum: [bulgular](2026-09-25-kalfa-2c-windows-prototip.md).

- Yalıtım kullanıcıları Users grubunda: false
- Alan A SID'i (S-1-9): `S-1-9-3077111628-4291679663-2638862731-3617966291`
- Alan B SID'i (S-1-5-21): `S-1-5-21-1844063520-2187801149-1672550975-1679`
- Ortak okuma SID'i: `S-1-9-1080448123-619989354-1684932357-558793429`
- Yalıtımın PATH'i: `C:\Windows\System32;C:\Windows;C:\Windows\System32\Wbem;C:\Windows\System32\WindowsPowerShell\v1.0;C:\Program Files\Git\mingw64\bin;C:\Program Files\nodejs;C:\Python314;C:\Users\kullanici\.cargo\bin`
- Ağsız kullanıcı `KalfaPrototipAgsiz` (S-1-5-21-3180714630-2268369888-530128323-1009), ağlı `KalfaPrototipAgli` (S-1-5-21-3180714630-2268369888-530128323-1010)
- P6 ilk çağrı: 211 ms (profil önceden vardı)
- P6 sonraki 10 çağrı: en az 57 / ortanca 58 / en çok 61 ms
- P6 dönen tutamaçla Kalfa'nın işine atama: başarılı
- P10 geliştirici penceresine gelen ileti sayısı: 1
- P7 sonrası: .git/config "[core]", pre-commit yerinde, hooks yerinde

| Kod | Senaryo | Beklenen | Sonuç | |
| --- | --- | --- | --- | --- |
| p1-sistem | hosts dosyasını okuma | okunur | 0: TAMAM (62 ms) | ✔ |
| p1-program | Program Files listeleme | okunur | 0: TAMAM (61 ms) | ✔ |
| p1-projeler-oku | C:\projeler benzeri klasörde dosya okuma | okunmaz | 0: PROJE-GIZLI (58 ms) | ✖ |
| p1-projeler-liste | C:\projeler benzeri klasörü listeleme | listelenmez | 0: gizli.txt (57 ms) | ✖ |
| p1-projeler-kisitsiz | aynısı, kısıtsız belirteçle (karşılaştırma) | okunur (kısıtsız) | 0: PROJE-GIZLI (59 ms) | · |
| p1-aap | ALL APPLICATION PACKAGES kısıtlayıcı SID olarak | kabul edilir (Dal A varsayımı) | 250: KALFA-PROTOTIP-HATA: CreateRestrictedToken: Win32 hatası 87 (31 ms) | ✖ |
| p1-yetenek | addan türetilen yetenek SID'i (S-1-15-3) kısıtlayıcı olarak | ölçüm (A7) | 250: KALFA-PROTOTIP-HATA: CreateRestrictedToken: Win32 hatası 87 (30 ms) | · |
| p1-alan | kendi çalışma alanında okuma ve yazma | çalışır | 0: ALAN-a / TAMAM (57 ms) | ✔ |
| p2-tam-cmd | cmd (tam) | çalışır | 0: Microsoft Windows [Version 10.0.26200.9457] (58 ms) | ✔ |
| p2-tam-powershell | powershell (tam) | çalışır | 0: 5 (231 ms) | ✔ |
| p2-tam-git | git (tam) | çalışır | 0: git version 2.55.0.windows.5 (80 ms) | ✔ |
| p2-tam-git-depo | git-depo (tam) | çalışır | 0: 45cc327 ilk (199 ms) | ✔ |
| p2-tam-node | node (tam) | çalışır | 0: v24.20.0 (86 ms) | ✔ |
| p2-tam-npm | npm (tam) | çalışır | 0: 11.19.0 (288 ms) | ✔ |
| p2-tam-python | python (tam) | çalışır | 0: 3.14.7 (105 ms) | ✔ |
| p2-tam-cargo | cargo (tam) | çalışır | 0: cargo 1.98.1 (797e8a9bc 2026-08-05) (103 ms) | ✔ |
| p2-tam-rustc | rustc (tam) | çalışır | 0: rustc 1.98.1 (48a229cea 2026-09-01) (105 ms) | ✔ |
| p2-everyone-yok-cmd | cmd (everyone-yok) | çalışır | 0: Microsoft Windows [Version 10.0.26200.9457] (54 ms) | ✔ |
| p2-everyone-yok-powershell | powershell (everyone-yok) | çalışır | 4294901760: Starting the CLR failed with HRESULT 80070005. /  (59 ms) | ✖ |
| p2-everyone-yok-git | git (everyone-yok) | çalışır | 128: fatal: could not open '/dev/null' for reading and writing: Permission denied (66 ms) | ✖ |
| p2-everyone-yok-git-depo | git-depo (everyone-yok) | çalışır | 128: fatal: could not open '/dev/null' for reading and writing: Permission denied (68 ms) | ✖ |
| p2-everyone-yok-node | node (everyone-yok) | çalışır | 0: v24.20.0 (78 ms) | ✔ |
| p2-everyone-yok-npm | npm (everyone-yok) | çalışır | 0: 11.19.0 (288 ms) | ✔ |
| p2-everyone-yok-python | python (everyone-yok) | çalışır | 3221225794:  (67 ms) | ✖ |
| p2-everyone-yok-cargo | cargo (everyone-yok) | çalışır | 3221225794: warn: could not canonicalize path C:\KalfaPrototip (86 ms) | ✖ |
| p2-everyone-yok-rustc | rustc (everyone-yok) | çalışır | 0: rustc 1.98.1 (48a229cea 2026-09-01) (94 ms) | ✔ |
| p2-users-yok-cmd | cmd (users-yok) | çalışır | 0:  (52 ms) | ✔ |
| p2-users-yok-powershell | powershell (users-yok) | çalışır | 3221225781:  (52 ms) | ✖ |
| p2-users-yok-git | git (users-yok) | çalışır | 1:  (53 ms) | ✖ |
| p2-users-yok-git-depo | git-depo (users-yok) | çalışır | 1:  (52 ms) | ✖ |
| p2-users-yok-node | node (users-yok) | çalışır | 1:  (56 ms) | ✖ |
| p2-users-yok-npm | npm (users-yok) | çalışır | 1:  (54 ms) | ✖ |
| p2-users-yok-python | python (users-yok) | çalışır | 1:  (59 ms) | ✖ |
| p2-users-yok-cargo | cargo (users-yok) | çalışır | 1:  (58 ms) | ✖ |
| p2-users-yok-rustc | rustc (users-yok) | çalışır | 1:  (58 ms) | ✖ |
| p2-restricted-yok-cmd | cmd (restricted-yok) | çalışır | 0: Microsoft Windows [Version 10.0.26200.9457] (71 ms) | ✔ |
| p2-restricted-yok-powershell | powershell (restricted-yok) | çalışır | 4294901760: Internal Windows PowerShell error.  Invoking managed Windows PowerShell failed with error 8007000e. /  (148 ms) | ✖ |
| p2-restricted-yok-git | git (restricted-yok) | çalışır | 0: git version 2.55.0.windows.5 (89 ms) | ✔ |
| p2-restricted-yok-git-depo | git-depo (restricted-yok) | çalışır | 0: 658f534 ilk (228 ms) | ✔ |
| p2-restricted-yok-node | node (restricted-yok) | çalışır | 0: v24.20.0 (81 ms) | ✔ |
| p2-restricted-yok-npm | npm (restricted-yok) | çalışır | 0: 11.19.0 (299 ms) | ✔ |
| p2-restricted-yok-python | python (restricted-yok) | çalışır | 0: 3.14.7 (103 ms) | ✔ |
| p2-restricted-yok-cargo | cargo (restricted-yok) | çalışır | 0: cargo 1.98.1 (797e8a9bc 2026-08-05) (94 ms) | ✔ |
| p2-restricted-yok-rustc | rustc (restricted-yok) | çalışır | 0: rustc 1.98.1 (48a229cea 2026-09-01) (95 ms) | ✔ |
| p2-oturum-yok-cmd | cmd (oturum-yok) | çalışır | 0: Microsoft Windows [Version 10.0.26200.9457] (56 ms) | ✔ |
| p2-oturum-yok-powershell | powershell (oturum-yok) | çalışır | 0: 5 (201 ms) | ✔ |
| p2-oturum-yok-git | git (oturum-yok) | çalışır | 0: git version 2.55.0.windows.5 (78 ms) | ✔ |
| p2-oturum-yok-git-depo | git-depo (oturum-yok) | çalışır | 0: 17315c3 ilk (196 ms) | ✔ |
| p2-oturum-yok-node | node (oturum-yok) | çalışır | 0: v24.20.0 (80 ms) | ✔ |
| p2-oturum-yok-npm | npm (oturum-yok) | çalışır | 0: 11.19.0 (301 ms) | ✔ |
| p2-oturum-yok-python | python (oturum-yok) | çalışır | 0: 3.14.7 (100 ms) | ✔ |
| p2-oturum-yok-cargo | cargo (oturum-yok) | çalışır | 0: cargo 1.98.1 (797e8a9bc 2026-08-05) (94 ms) | ✔ |
| p2-oturum-yok-rustc | rustc (oturum-yok) | çalışır | 0: rustc 1.98.1 (48a229cea 2026-09-01) (94 ms) | ✔ |
| p2-ortak-yok-cmd | cmd (ortak-yok) | çalışır | 0: Microsoft Windows [Version 10.0.26200.9457] (59 ms) | ✔ |
| p2-ortak-yok-powershell | powershell (ortak-yok) | çalışır | 0: 5 (233 ms) | ✔ |
| p2-ortak-yok-git | git (ortak-yok) | çalışır | 0: git version 2.55.0.windows.5 (79 ms) | ✔ |
| p2-ortak-yok-git-depo | git-depo (ortak-yok) | çalışır | 0: 7266527 ilk (202 ms) | ✔ |
| p2-ortak-yok-node | node (ortak-yok) | çalışır | 0: v24.20.0 (82 ms) | ✔ |
| p2-ortak-yok-npm | npm (ortak-yok) | çalışır | 0: 11.19.0 (299 ms) | ✔ |
| p2-ortak-yok-python | python (ortak-yok) | çalışır | 0: 3.14.7 (102 ms) | ✔ |
| p2-ortak-yok-cargo | cargo (ortak-yok) | çalışır | 1: 'cargo' is not recognized as an internal or external command, / operable program or batch file. (57 ms) | ✖ |
| p2-ortak-yok-rustc | rustc (ortak-yok) | çalışır | 1: 'rustc' is not recognized as an internal or external command, / operable program or batch file. (60 ms) | ✖ |
| p3-a-b-oku | A'nın SID'iyle B'yi okuma | okunmaz | 1: Eri�im engellendi. (57 ms) | ✔ |
| p3-a-b-yaz | A'nın SID'iyle B'ye yazma | yazılmaz | 1: Eri�im engellendi. (57 ms) | ✔ |
| p3-b-kendi | B'nin (S-1-5-21) SID'iyle kendi alanında okuma ve yazma | çalışır | 0: ALAN-b / TAMAM (59 ms) | ✔ |
| p3-b-a-oku | B'nin SID'iyle A'yı okuma | okunmaz | 1: Eri�im engellendi. (57 ms) | ✔ |
| p4-profil | geliştiricinin profilini listeleme | listelenmez | 1: File Not Found (58 ms) | ✔ |
| p4-kanarya | profildeki dosyayı okuma | okunmaz | 1: Eri�im engellendi. (58 ms) | ✔ |
| p4-kimlik | geliştiricinin Credentials dizini | listelenmez | 1: File Not Found (56 ms) | ✔ |
| p4-kanarya-kisitsiz | profildeki dosya, kısıtsız belirteçle | okunmaz | 1: Eri�im engellendi. (59 ms) | ✔ |
| p5-agsiz-dis | ağsız kullanıcı, dış HTTPS | bağlanmaz | 7: 000 (71 ms) | ✔ |
| p5-agsiz-geri-dongu | ağsız kullanıcı, 127.0.0.1 | ölçüm | 0: GERI-DONGU-OK (56 ms) | · |
| p5-boru-openssh-ssh-agent | \\.\pipe\openssh-ssh-agent | ölçüm | 0: MethodInvocationException: Exception calling "Connect" with "1" argument(s): "��lem zaman a��m�na u�rad�." (949 ms) | · |
| p5-boru-docker_engine | \\.\pipe\docker_engine | ölçüm | 0: MethodInvocationException: Exception calling "Connect" with "1" argument(s): "��lem zaman a��m�na u�rad�." (945 ms) | · |
| p5-agli-dis | ağlı kullanıcı, dış HTTPS | bağlanır | 35: 000 (150 ms) | ✖ |
| p5-agsiz-http | agsiz kullanıcı, dış HTTP (TLS'siz) | bağlanmaz | 7: 000 (75 ms) | ✔ |
| p5-agli-http | agli kullanıcı, dış HTTP (TLS'siz) | bağlanır | 0: 200 (200 ms) | ✔ |
| p5-agli-https-iptalsiz | ağlı kullanıcı, HTTPS, sertifika iptal denetimi kapalı | ölçüm | 35: 000 (138 ms) | · |
| p5-agli-https-kisitsiz | ağlı kullanıcı, HTTPS, kısıtsız belirteçle (karşılaştırma) | ölçüm | 0: 200 (294 ms) | · |
| p5-agli-python-https | ağlı kullanıcı, python ile HTTPS | ölçüm | 0: 200 (386 ms) | · |
| p5-schannel-ayrinti | ağlı kullanıcı, HTTPS, curl'ün schannel iletileri | ölçüm | 0: * schannel: disabled automatic use of client certificate / * schannel: AcquireCredentialsHandle failed: SEC_E_NO_CREDENTIALS (0x8009030e) - G�venlik paketinde k (153 ms) | · |
| p5-git-openssl | ağlı kullanıcı, git ls-remote HTTPS (openssl) | ölçüm | 0: 14432a28e180ec717933714aed7828fc7ed2fd29HEAD (779 ms) | · |
| p5-git-schannel | ağlı kullanıcı, git ls-remote HTTPS (schannel) | ölçüm | 128: fatal: unable to access 'https://github.com/rust-lang/cfg-if/': schannel: AcquireCredentialsHandle failed: SEC_E_NO_CREDENTIALS (0x8009030e) - G�venlik paketind (174 ms) | · |
| p5-cargo-ag | ağlı kullanıcı, cargo search (crates.io) | ölçüm | 101: warn: could not canonicalize path C:\KalfaPrototip / Updating crates.io index / warning: spurious network error (3 tries remaining): [35] SSL connect error (sch (11351 ms) | · |
| p5-schannel-ayricalikli | ağlı kullanıcı, HTTPS, ayrıcalıklar kaldırılmadan | ölçüm | 35: 000 (122 ms) | · |
| p5-agsiz-http-kisitsiz | ağsız kullanıcı, HTTP, kısıtsız belirteçle (karşılaştırma) | bağlanmaz | 7: 000 (84 ms) | ✔ |
| p5-schannel-authusers | ağlı kullanıcı, HTTPS, kümeye S-1-5-11 eklenince | ölçüm | 35: 000 (135 ms) | · |
| p5-schannel-interactive | ağlı kullanıcı, HTTPS, kümeye S-1-5-4 eklenince | ölçüm | 35: 000 (133 ms) | · |
| p5-schannel-konsol | ağlı kullanıcı, HTTPS, kümeye S-1-2-1 eklenince | ölçüm | 35: 000 (130 ms) | · |
| p5-schannel-local | ağlı kullanıcı, HTTPS, kümeye S-1-2-0 eklenince | ölçüm | 35: 000 (129 ms) | · |
| p5-schannel-yerel-hesap | ağlı kullanıcı, HTTPS, kümeye S-1-5-113 eklenince | ölçüm | 35: 000 (137 ms) | · |
| p5-schannel-oturum | ağlı kullanıcı, HTTPS, kümeye OTURUM eklenince | ölçüm | 35: 000 (174 ms) | · |
| p5-schannel-kullanici | ağlı kullanıcı, HTTPS, kümeye KULLANICI eklenince | ölçüm | 35: 000 (139 ms) | · |
| p5-boru-varsayilan-oku | geliştiricinin varsayılan izinli borusu (In) | ölçüm | 0:  (446 ms) | · |
| p5-boru-varsayilan-oku-yaz | geliştiricinin varsayılan izinli borusu (InOut) | ölçüm | 0: MethodInvocationException: Exception calling "Connect" with "1" argument(s): "Yola eri�im engellendi." (444 ms) | · |
| p7-gizli | .env okuma | okunmaz | 1: Eri�im engellendi. (59 ms) | ✔ |
| p7-config-yaz | .git/config'e ekleme | yazılmaz | 1: Eri�im engellendi. (60 ms) | ✔ |
| p7-config-sil | .git/config'i silme | silinmez | 1: C:\KalfaPrototip\alan-a\.git\config (57 ms) | ✔ |
| p7-config-uzerine | .git/config'in üzerine taşıma | taşınmaz | 1: 0 file(s) moved. (57 ms) | ✔ |
| p7-hook-yeni | .git/hooks'a dosya ekleme | eklenmez | 1: Eri�im engellendi. (54 ms) | ✔ |
| p7-hook-sil | pre-commit'i silme | silinmez | 1: C:\KalfaPrototip\alan-a\.git\hooks\pre-commit (56 ms) | ✔ |
| p7-hooks-tasi | .git/hooks dizinini yeniden adlandırma | taşınmaz | 1: Eri�im engellendi. (56 ms) | ✔ |
| p9-kutu | bağımlılıksız crate, çalışma alanına özel CARGO_HOME, ağsız | derlenir | 0: TAMAM (98 ms) | ✔ |
| p9-bagimli-ozel | bağımlı crate, çalışma alanına özel CARGO_HOME, ağsız | derlenmez (önbellek yok) | 101: warn: could not canonicalize path C:\KalfaPrototip / error: no matching package named `cfg-if` found / location searched: crates.io index (98 ms) | · |
| p9-bagimli-gelistirici | bağımlı crate, geliştiricinin CARGO_HOME'u salt okunur, ağsız | ölçüm | 0: TAMAM (11725 ms) | · |
| p10-kisitli | UI kısıtlarıyla ileti, pano, pencere bulma | üçü de engellenir | 0: ileti=engellendi / pano=acildi metin_var=true / pencere_bulma=buldu (50 ms) | · |
| p10-kisitsiz | UI kısıtları olmadan (karşılaştırma) | ölçüm | 0: ileti=gonderildi / pano=acildi metin_var=true / pencere_bulma=buldu (50 ms) | · |
