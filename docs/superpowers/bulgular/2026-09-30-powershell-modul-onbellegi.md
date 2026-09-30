# Windows PowerShell açılışı ve modül çözümleme önbelleği (2026-09-30)

## Belirti

CI taslağında (PR #28) GitHub'ın `windows-2025` makinesinde iki test, hem
yönetici hem standart kullanıcıyla düşüyordu:

- `kalfa-sandbox` `tests/yalitimsiz.rs::iptal_torun_sureci_de_durdurur`:
  2,5 sn'deki iptalden önce standart çıktı boş (`ParseIntError { kind: Empty }`).
- `kalfa-tools` `tests/kabuk.rs::ayni_turdaki_kabuklar_hazirliktan_sonra_da_calisir`:
  olay beklerken 5 sn'lik zaman aşımı.

Aynı koşuda PowerShell'le çalışan öteki testler yeşildi ama tek tek 10–60 sn
sürüyordu. Tam ortamla ölçülen `powershell -Command "exit 0"` 0,27 sn'ydi.

## Ölçüm

`windows-2025`, runneradmin, `powershell.exe -NoProfile -NonInteractive`,
ortam Kalfa'nın izin listesiyle (`kalfa-sandbox/src/ortam.rs`) süzülmüş, her
satır üç koşunun aralığı:

| Ortam | Betik | Süre |
|---|---|---|
| Kalfa'nın ortamı | `Write-Output tamam` | 25,1–37,1 sn |
| Kalfa'nın ortamı | `exit 0` (cmdlet yok) | 0,23–0,25 sn |
| + `PSModuleAnalysisCachePath` | `Write-Output tamam` | 0,37–0,45 sn |
| + `PSModulePath` (makine değeri) | `Write-Output tamam` | 24,8–25,1 sn |
| + `PSModulePath` (boş dizin) | `Write-Output tamam` | 0,32–0,36 sn |
| Tam ortam | `Write-Output tamam` | 0,31–0,38 sn |
| Kalfa'nın ortamı | `Start-Process ping` + `Write-Output` | 25,3 sn |
| + `PSModuleAnalysisCachePath` | `Start-Process ping` + `Write-Output` | 0,56 sn |

Makinede `PSModuleAnalysisCachePath=C:\PSModuleAnalysisCachePath\ModuleAnalysisCache`
(imaj önbelleği kurulumda ısıtıyor), `PSModulePath` Az 15.6.1, SQL Server ve
`Program Files\WindowsPowerShell\Modules` altındaki 54 modülü içeriyor.

## Kök neden

Windows PowerShell 5.1, ilk cmdlet çağrısında (`Write-Output`,
`Start-Process`) komut keşfi için modül yolundaki bütün modülleri çözümler ve
sonucu `PSModuleAnalysisCachePath`'teki önbellekten okur. Kalfa'nın izin
listesi bu değişkeni geçirmiyordu; PowerShell önbelleği
`%LOCALAPPDATA%\Microsoft\Windows\PowerShell\ModuleAnalysisCache`'te arıyor,
bulamıyor ve her komutta bütün modülleri baştan çözümlüyordu. Varsayılan yerdeki
önbellek art arda koşularda da ısınmadı (A1–A3 hep ~25 sn). Testlerdeki 2,5 ve
5 sn'lik sınırlar bu yüzden aşılıyordu; geliştirici makinelerinde modül sayısı
az olduğu için belirti görünmüyordu.

Sorun testlerde değil, ortam izin listesindeki eksik değişkendedir.

## Karar

`PSModuleAnalysisCachePath` Windows izin listesine eklendi (sır değil; yalnızca
bir dosya yolu). Yalıtımlı (AppContainer) çalıştırmada geçmez: kapta önbellek
çalışma alanına özel `LOCALAPPDATA`'dadır ve geliştiricinin paylaşılan önbellek
dosyası kaba gösterilmez.

`PSModulePath` eklenmedi: makine değeriyle süre değişmiyor, Kalfa PowerShell 7
içinden açıldığında da 7'nin modül yollarını Windows PowerShell'e taşıyıp
yerleşik modülleri bozabilir.
