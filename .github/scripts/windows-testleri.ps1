# Windows testlerini yönetici olmayan bir kullanıcıyla koşturur.
#
# GitHub'ın Windows makinesi işleri yükseltilmiş yönetici olarak koşar;
# kalfa-sandbox'un Windows testleri ise geliştiricinin sıradan hesabını
# varsayar (sahiplik, DACL mirası, yöneticiye ait dizinler). Betik geçici bir
# standart kullanıcı açar ve testleri onun adına, zamanlanmış bir görevle
# koşturur. Test ikilileri önceden (`cargo test --no-run`) derlenmiş olmalıdır.
#
# Yalnızca her işten sonra silinen CI makinesi içindir; geliştirici
# makinesinde çalıştırmayın.

$ErrorActionPreference = 'Stop'

$kullanici = 'kalfatest'
$gorevAdi = 'kalfa-testleri'
$calisma = $env:GITHUB_WORKSPACE
$klasor = Join-Path $env:RUNNER_TEMP 'kalfatest'
$icBetik = Join-Path $klasor 'testler.cmd'
$gunluk = Join-Path $klasor 'testler.log'
$sonuc = Join-Path $klasor 'sonuc.txt'
# Testlerin geçici dizini, geliştirici makinesindeki gibi sistem sürücüsünde.
# Hesabın profil Temp'i GitHub makinesinde DACL otomatik miras (AI) bayrağı
# taşımıyor; icacls'in yazdığı bu dizin ve altındakiler taşır.
$gecici = Join-Path $env:SystemDrive 'kalfatest-tmp'
$cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
$rustupHome = if ($env:RUSTUP_HOME) { $env:RUSTUP_HOME } else { Join-Path $env:USERPROFILE '.rustup' }

New-Item -ItemType Directory -Force $klasor, $gecici | Out-Null

# Rastgele parola; günlükte maskelenir.
$baytlar = [byte[]]::new(24)
[System.Security.Cryptography.RandomNumberGenerator]::Fill($baytlar)
$parola = [Convert]::ToBase64String($baytlar) + 'aA1!'
Write-Output "::add-mask::$parola"

$bilgisayar = [ADSI]"WinNT://$env:COMPUTERNAME,computer"
$hesap = $bilgisayar.Create('User', $kullanici)
$hesap.SetPassword($parola)
$hesap.SetInfo()
# Zamanlanmış görev "toplu iş olarak oturum açma" hakkı ister; bu grup o
# hakkı taşır ve yönetici yetkisi vermez.
([ADSI]"WinNT://$env:COMPUTERNAME/Performance Log Users,group").Add("WinNT://$env:COMPUTERNAME/$kullanici,user")

# Bağlantı testleri: Geliştirici Modu, yönetici olmayan kullanıcıya sembolik
# bağlantı oluşturma izni verir.
$gelistirici = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock'
New-Item -Force $gelistirici | Out-Null
Set-ItemProperty $gelistirici -Name AllowDevelopmentWithoutDevLicense -Value 1 -Type DWord

# Depo ve `target` (cargo yazar), araç zinciri (cargo ve rustup kilit
# dosyaları yazar), bu betiğin klasörü ve testlerin geçici dizini.
foreach ($yol in @($calisma, $cargoHome, $rustupHome, $klasor, $gecici)) {
    & icacls.exe $yol /grant "${kullanici}:(OI)(CI)M" /Q | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "icacls failed: $yol" }
}


# Zamanlanmış görev etkileşimsiz bir masaüstünde koşar; oranın masaüstü
# yığını küçüktür ve aynı anda açılan çok sayıda PowerShell (her biri bir
# conhost ile) süreç açılışını kilitliyor. Testler tek iş parçacığıyla koşar.
$komutlar = @(
    'cargo test --workspace --locked --no-fail-fast -- --test-threads=1',
    'cargo test -p kalfa-uygulama -p kalfa --features e2e --locked --no-fail-fast -- --test-threads=1'
)
# Görevin koşturduğu toplu dosya; yönetici oturumunun PATH'ini taşır.
$satirlar = @(
    '@echo off',
    "set `"PATH=$($env:PATH)`"",
    "set `"CARGO_HOME=$cargoHome`"",
    "set `"RUSTUP_HOME=$rustupHome`"",
    'set CARGO_TERM_COLOR=always',
    'set CARGO_INCREMENTAL=0',
    "cd /d `"$calisma`"",
    "set `"TEMP=$gecici`"",
    "set `"TMP=$gecici`"",
    'set KOD=0'
)
foreach ($komut in $komutlar) {
    $satirlar += "echo ^>^>^> $komut>> `"$gunluk`""
    $satirlar += "$komut >> `"$gunluk`" 2>&1"
    $satirlar += 'if %ERRORLEVEL% neq 0 set KOD=1'
}
# `echo 0> dosya` tanıtıcı yönlendirmesi sayılır; yönlendirme öne yazılır.
$satirlar += ">`"$sonuc`" echo %KOD%"
Set-Content -Encoding ascii $icBetik $satirlar

$eylem = New-ScheduledTaskAction -Execute 'cmd.exe' -Argument "/d /c `"$icBetik`""
$ayar = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Hours 1)
Register-ScheduledTask -TaskName $gorevAdi -Action $eylem -Settings $ayar -User $kullanici -Password $parola -RunLevel Limited | Out-Null
Start-ScheduledTask -TaskName $gorevAdi

# Görevin günlüğü ilerledikçe basılır.
$script:okunan = 0
function Gunlugu-Bas {
    if (Test-Path $gunluk) {
        $tum = @(Get-Content $gunluk)
        if ($tum.Count -gt $script:okunan) {
            $tum[$script:okunan..($tum.Count - 1)] | Write-Output
            $script:okunan = $tum.Count
        }
    }
}
$sonZaman = (Get-Date).AddMinutes(50)
while (-not (Test-Path $sonuc)) {
    Start-Sleep -Seconds 10
    Gunlugu-Bas
    if (Test-Path $sonuc) { break }
    $durum = (Get-ScheduledTask -TaskName $gorevAdi).State
    if ($durum -ne 'Running' -and $durum -ne 'Queued') {
        Start-Sleep -Seconds 2
        if (Test-Path $sonuc) { break }
        $bilgi = Get-ScheduledTaskInfo -TaskName $gorevAdi
        throw "Task stopped without a result (state $durum, last result $($bilgi.LastTaskResult))"
    }
    if ((Get-Date) -gt $sonZaman) { throw 'Timed out waiting for tests' }
}
Gunlugu-Bas
$kod = [int](Get-Content $sonuc)
Write-Output "Exit code: $kod"
exit $kod
