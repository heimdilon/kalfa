#Requires -RunAsAdministrator
<#
Kalfa 2c prototipi: kurulumu geri alır. Yönetici PowerShell'de:

    powershell -ExecutionPolicy Bypass -File .\geri-al.ps1

Kurulum kaydına göre izinleri, güvenlik duvarı kuralını, oturum açma
haklarını, giriş ekranı gizlemesini, kullanıcıların profillerini,
kullanıcıları ve grubu siler; deneme ağacını (C:\KalfaPrototip,
C:\KalfaPrototipProjeler) ve %LOCALAPPDATA%\KalfaPrototip'i kaldırır. Kayıt
yoksa adlara göre bulabildiğini siler. Her adım bağımsızdır: biri düşerse
diğerleri yine denenir; sonda kalanlar listelenir.
#>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ortak.ps1')

$kalanlar = @()
function Adim([string]$ad, [scriptblock]$is) {
    try { & $is; Write-Host "✔ $ad" }
    catch { Write-Host "✖ $ad`: $($_.Exception.Message)"; $script:kalanlar += $ad }
}

$kurulum = Kurulumu-Oku
$grupSid = if ($kurulum) { $kurulum.grup_sid } else { (Get-LocalGroup -Name $Grup -ErrorAction SilentlyContinue).SID.Value }
$kullaniciSidleri = @($Agsiz, $Agli) | ForEach-Object {
    (Get-LocalUser -Name $_ -ErrorAction SilentlyContinue).SID.Value
} | Where-Object { $_ }

if ($kurulum) {
    foreach ($d in $kurulum.izinli_dizinler) {
        Adim "izin: $d" { Izin-Kaldir $d @($kurulum.grup_sid, $kurulum.ortak_sid) }
    }
}

Adim 'güvenlik duvarı kuralı' {
    Get-NetFirewallRule -Name $KuralAdi -ErrorAction SilentlyContinue | Remove-NetFirewallRule
}

if ($grupSid) {
    Adim 'oturum açma hakları' {
        foreach ($hak in $IzinVerilenHaklar + $ReddedilenHaklar) { [KalfaLsa]::Kaldir($grupSid, $hak) }
    }
}

Adim 'giriş ekranı gizlemesi' {
    if (Test-Path $UserListAnahtari) {
        foreach ($ad in @($Agsiz, $Agli)) {
            Remove-ItemProperty -Path $UserListAnahtari -Name $ad -ErrorAction SilentlyContinue
        }
        $bos = -not (Get-Item $UserListAnahtari).Property
        if ($kurulum -and $kurulum.userlist_olusturuldu -and $bos) { Remove-Item $UserListAnahtari }
    }
}

Adim 'kullanıcı profilleri' {
    Get-CimInstance Win32_UserProfile | Where-Object { $kullaniciSidleri -contains $_.SID } |
        ForEach-Object { Remove-CimInstance $_ }
}

Adim 'kullanıcılar' {
    foreach ($ad in @($Agsiz, $Agli)) {
        if (Get-LocalUser -Name $ad -ErrorAction SilentlyContinue) { Remove-LocalUser -Name $ad }
    }
}

Adim 'grup' {
    if (Get-LocalGroup -Name $Grup -ErrorAction SilentlyContinue) { Remove-LocalGroup -Name $Grup }
}

Adim 'deneme ağacı' {
    foreach ($d in @('C:\KalfaPrototip', 'C:\KalfaPrototipProjeler')) {
        if (Test-Path $d) { Remove-Item -Recurse -Force $d }
    }
}

Adim 'kurulum kaydı ve parolalar' {
    if (Test-Path $Yerel) { Remove-Item -Recurse -Force $Yerel }
}

Write-Host ""
if ($kalanlar.Count -eq 0) { Write-Host 'Geri alma tamam; makine prototipten önceki hâlinde.' }
else { Write-Host ("Kalanlar: {0}" -f ($kalanlar -join ', ')) }
