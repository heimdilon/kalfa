#Requires -RunAsAdministrator
<#
Kalfa 2c prototipi: kurulum (aşama 2 tasarımı §9.2, §9.5). Yönetici
PowerShell'de, geliştiricinin kendi hesabıyla yükseltilerek çalıştırılır:

    powershell -ExecutionPolicy Bypass -File .\kur.ps1

Yaptıkları (hepsini geri-al.ps1 geri alır):
- Yerel grup KalfaPrototip ve iki yerel kullanıcı: KalfaPrototipAgsiz,
  KalfaPrototipAgli. Parolalar rastgeledir (32 bayt), geliştiricinin DPAPI'siyle
  %LOCALAPPDATA%\KalfaPrototip\parolalar.bin'e şifreli yazılır; ekrana basılmaz.
- Kullanıcılar Users grubunda olmaz (-UsersGrubunda verilmedikçe). Gruba yerel
  oturum açma hakkı verilir; ağdan, uzak masaüstünden, toplu iş ve hizmet
  olarak oturum açma reddedilir. Kullanıcılar giriş ekranında gizlenir.
- Güvenlik duvarı: KalfaPrototipAgsiz'ın bütün giden trafiğini engelleyen kural.
  Kullanıcı koşulu ortak okuma SID'ini de içerir; yoksa kural kısıtlı
  belirteçlere uygulanmıyor (guvenlik-duvari.ps1'e bakın).
- Ortak okuma SID'i üretilir; araç zinciri dizinlerine (.cargo\bin,
  .cargo\registry, .rustup, C:\Python314) grup ve bu SID için okuma izni
  verilir. .cargo'nun kendisine izin verilmez: credentials.toml orada durur.
#>
param([switch]$UsersGrubunda)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ortak.ps1')

if (Kurulumu-Oku) {
    throw "Kurulum zaten var ($KurulumDosyasi). Önce geri-al.ps1 çalıştırın."
}

function Rastgele-Bayt([int]$n) {
    $b = New-Object byte[] $n
    $r = [Security.Cryptography.RandomNumberGenerator]::Create()
    $r.GetBytes($b)
    $r.Dispose()
    $b
}

function Rastgele-Sid {
    $b = Rastgele-Bayt 16
    $parcalar = 0..3 | ForEach-Object { [BitConverter]::ToUInt32($b, $_ * 4) }
    'S-1-9-' + ($parcalar -join '-')
}

$kurulum = [ordered]@{
    grup = $Grup
    grup_sid = $null
    ortak_sid = Rastgele-Sid
    agsiz = $null
    agli = $null
    users_grubunda = [bool]$UsersGrubunda
    izinli_dizinler = @()
    userlist_olusturuldu = $false
}

Write-Host "Grup ve kullanıcılar oluşturuluyor…"
$g = New-LocalGroup -Name $Grup -Description 'Kalfa 2c prototipi (geçici)'
$kurulum.grup_sid = $g.SID.Value
$parolalar = @{}
foreach ($ad in @($Agsiz, $Agli)) {
    $parola = [Convert]::ToBase64String((Rastgele-Bayt 32))
    $parolalar[$ad] = $parola
    $guvenli = ConvertTo-SecureString $parola -AsPlainText -Force
    $k = New-LocalUser -Name $ad -Password $guvenli -PasswordNeverExpires -UserMayNotChangePassword `
        -AccountNeverExpires -Description 'Kalfa 2c prototipi (geçici)'
    Add-LocalGroupMember -Group $Grup -Member $ad
    $users = Users-Grubu
    $uye = Get-LocalGroupMember -Group $users | Where-Object { $_.SID -eq $k.SID }
    if ($UsersGrubunda -and -not $uye) { Add-LocalGroupMember -Group $users -Member $ad }
    if (-not $UsersGrubunda -and $uye) { Remove-LocalGroupMember -Group $users -Member $ad }
    $bilgi = [ordered]@{ ad = $ad; sid = $k.SID.Value }
    if ($ad -eq $Agsiz) { $kurulum.agsiz = $bilgi } else { $kurulum.agli = $bilgi }
}

# Parolalar: geliştiricinin DPAPI'si (kullanıcı kapsamı).
New-Item -ItemType Directory -Force $Yerel | Out-Null
$json = [Text.Encoding]::UTF8.GetBytes(($parolalar | ConvertTo-Json -Compress))
$sifreli = [Security.Cryptography.ProtectedData]::Protect(
    $json, $null, [Security.Cryptography.DataProtectionScope]::CurrentUser)
[IO.File]::WriteAllBytes($ParolaDosyasi, $sifreli)
# Kurulum kaydı hemen yazılır: sonraki adım düşerse geri-al.ps1 yine çalışır.
Kurulumu-Yaz $kurulum

Write-Host "Oturum açma hakları…"
foreach ($hak in $IzinVerilenHaklar + $ReddedilenHaklar) {
    [KalfaLsa]::Ekle($kurulum.grup_sid, $hak)
}

Write-Host "Giriş ekranında gizleme…"
if (-not (Test-Path $UserListAnahtari)) {
    New-Item -Path $UserListAnahtari -Force | Out-Null
    $kurulum.userlist_olusturuldu = $true
}
foreach ($ad in @($Agsiz, $Agli)) {
    New-ItemProperty -Path $UserListAnahtari -Name $ad -Value 0 -PropertyType DWord -Force | Out-Null
}
Kurulumu-Yaz $kurulum

Write-Host "Güvenlik duvarı kuralı…"
New-NetFirewallRule -Name $KuralAdi -DisplayName 'Kalfa 2c prototipi: ağsız kullanıcı' `
    -Direction Outbound -Action Block -Profile Any `
    -LocalUser "D:(A;;CC;;;$($kurulum.agsiz.sid))(A;;CC;;;$($kurulum.ortak_sid))" | Out-Null

Write-Host "Araç zinciri dizinlerine okuma izni…"
$adaylar = @(
    (Join-Path $env:USERPROFILE '.cargo\bin'),
    (Join-Path $env:USERPROFILE '.cargo\registry'),
    (Join-Path $env:USERPROFILE '.rustup'),
    'C:\Python314'
)
foreach ($d in $adaylar) {
    if (-not (Test-Path $d)) { continue }
    Izin-Ver $d @($kurulum.grup_sid, $kurulum.ortak_sid)
    $kurulum.izinli_dizinler += $d
    Kurulumu-Yaz $kurulum
}

Write-Host ""
Write-Host "Kurulum tamam. Kayıt: $KurulumDosyasi"
Write-Host ("Users grubunda: {0}" -f [bool]$UsersGrubunda)
Write-Host "İzin verilen dizinler:"
$kurulum.izinli_dizinler | ForEach-Object { Write-Host "  $_" }
