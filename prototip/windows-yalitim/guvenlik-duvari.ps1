#Requires -RunAsAdministrator
<#
Kalfa 2c prototipi, P5: ağsız kullanıcının güvenlik duvarı kuralını kısıtlı
belirteçlere de uygulanır hâle getirir. Yönetici PowerShell'de:

    powershell -ExecutionPolicy Bypass -File .\guvenlik-duvari.ps1

Neden: kuralın kullanıcı koşulu, sürecin belirteciyle bir güvenlik
tanımlayıcısına karşı erişim denetimi yapar. Kısıtlı belirteçte bu denetim
kısıtlayıcı SID'lerden de geçmek zorundadır. Tanımlayıcıda yalnızca
kullanıcının SID'i varken koşul tutmuyor ve kural uygulanmıyordu (ölçüldü:
kısıtsız belirteç engelleniyor, kısıtlı geçiyor). Tanımlayıcıya ortak okuma
SID'i eklenir. Bu SID'i yalnızca yalıtımın kısıtlı belirteçleri taşır; başka
kullanıcıların süreçleri kullanıcı SID'i denetiminden zaten geçemez.
#>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ortak.ps1')

$kurulum = Kurulumu-Oku
if (-not $kurulum) { throw 'Kurulum yok; önce kur.ps1.' }
$tanim = "D:(A;;CC;;;$($kurulum.agsiz.sid))(A;;CC;;;$($kurulum.ortak_sid))"
Set-NetFirewallRule -Name $KuralAdi -LocalUser $tanim
Write-Host "Kuralın kullanıcı koşulu: $tanim"
