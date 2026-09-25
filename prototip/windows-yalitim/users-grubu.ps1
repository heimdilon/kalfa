#Requires -RunAsAdministrator
<#
Kalfa 2c prototipi, P8: yalıtım kullanıcılarını Users grubuna ekler ya da
çıkarır. Yönetici PowerShell'de:

    powershell -ExecutionPolicy Bypass -File .\users-grubu.ps1 -Ekle
    powershell -ExecutionPolicy Bypass -File .\users-grubu.ps1 -Cikar
#>
param([switch]$Ekle, [switch]$Cikar)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ortak.ps1')

if ($Ekle -eq $Cikar) { throw 'Yalnızca biri: -Ekle ya da -Cikar.' }
$kurulum = Kurulumu-Oku
if (-not $kurulum) { throw 'Kurulum yok; önce kur.ps1.' }
$users = Users-Grubu
foreach ($ad in @($Agsiz, $Agli)) {
    $uye = Get-LocalGroupMember -Group $users | Where-Object { $_.Name -like "*\$ad" }
    if ($Ekle -and -not $uye) { Add-LocalGroupMember -Group $users -Member $ad }
    if ($Cikar -and $uye) { Remove-LocalGroupMember -Group $users -Member $ad }
}
$kurulum.users_grubunda = [bool]$Ekle
Kurulumu-Yaz $kurulum
Write-Host ("Users grubunda: {0}" -f [bool]$Ekle)
