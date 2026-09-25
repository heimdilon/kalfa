#Requires -RunAsAdministrator
<#
Kalfa 2c prototipi, AppContainer ek ölçümü: kap-kur.ps1'i geri alır. Yönetici
PowerShell'de:

    powershell -ExecutionPolicy Bypass -File .\kap-geri-al.ps1

Araç zinciri izinlerini ve C:\ kökündeki izni kaldırır, deneme ağacını (C:\KalfaPrototip,
C:\KalfaPrototipProjeler) siler, AppContainer profillerini siler ve
%LOCALAPPDATA%\KalfaPrototip'i kaldırır. Her adım bağımsızdır: biri düşerse
diğerleri yine denenir; sonda kalanlar listelenir.
#>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ortak.ps1')

$KapKaydi = Join-Path $Yerel 'kap.json'
$KapKurulum = Join-Path $Yerel 'kap-kurulum.json'

$kalanlar = @()
function Adim([string]$ad, [scriptblock]$is) {
    try { & $is; Write-Host "✔ $ad" }
    catch { Write-Host "✖ $ad`: $($_.Exception.Message)"; $script:kalanlar += $ad }
}

if (-not ('KalfaKap' -as [type])) {
    Add-Type -TypeDefinition @'
using System.Runtime.InteropServices;
public static class KalfaKap {
    [DllImport("userenv.dll", CharSet = CharSet.Unicode)]
    public static extern int DeleteAppContainerProfile(string ad);
}
'@
}

$kap = if (Test-Path $KapKaydi) { Get-Content -Raw -Encoding UTF8 $KapKaydi | ConvertFrom-Json } else { $null }
$kurulum = if (Test-Path $KapKurulum) { Get-Content -Raw -Encoding UTF8 $KapKurulum | ConvertFrom-Json } else { $null }

if ($kap -and $kurulum) {
    foreach ($d in $kurulum.izinli_dizinler) {
        Adim "izin: $d" { Izin-Kaldir $d @($kap.arac_sid) }
    }
    if ($kurulum.kok_izni) {
        Adim 'izin: C:\ kökü' { Kok-Izni-Kaldir $kap.arac_sid }
    }
}

Adim 'deneme ağacı' {
    foreach ($d in @('C:\KalfaPrototip', 'C:\KalfaPrototipProjeler')) {
        if (Test-Path $d) { Remove-Item -Recurse -Force $d }
    }
}

foreach ($ad in @('Kalfa.Prototip.AlanA', 'Kalfa.Prototip.AlanB')) {
    Adim "AppContainer profili: $ad" {
        $hr = [KalfaKap]::DeleteAppContainerProfile($ad)
        if ($hr -ne 0) { throw ('HRESULT 0x{0:X8}' -f $hr) }
    }
}

Adim 'kayıtlar ve raporlar' {
    if (Test-Path $Yerel) { Remove-Item -Recurse -Force $Yerel }
}

# Profiller yükseltilmiş süreçten silinemeyebilir (2c'de öyle oldu).
foreach ($ad in @('kalfa.prototip.alana', 'kalfa.prototip.alanb')) {
    if (Test-Path (Join-Path $env:LOCALAPPDATA "Packages\$ad")) {
        Write-Host "Profil klasörü duruyor ($ad): yönetici olmayan bir PowerShell'de 'cargo run --offline -- kap sil' çalıştırın."
    }
}

Write-Host ''
if ($kalanlar.Count -eq 0) { Write-Host 'Geri alma tamam; makine AppContainer ölçümünden önceki hâlinde.' }
else { Write-Host ("Kalanlar: {0}" -f ($kalanlar -join ', ')) }
