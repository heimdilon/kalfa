<#
Kalfa 2c prototipi: deneme ağacı (aşama 2 tasarımı §9.4'ün prototipteki
karşılığı). Yönetici gerekmez; kur.ps1'den sonra, ölçümden önce çalıştırılır:

    cargo build --offline
    powershell -ExecutionPolicy Bypass -File .\agac.ps1

Her çalıştırmada ağacı sıfırdan kurar:
- C:\KalfaPrototip: yalnızca geliştirici, SYSTEM ve Administrators erişir
  (üstten miras almaz).
  - bin\: prototipin kopyası; yalıtım okur ve çalıştırır, yazamaz.
  - alan-a, alan-b: iki çalışma alanı; ozel-a, ozel-b: onlara özel ev, geçici
    ve önbellek dizinleri. Her biri grup ve kendi çalışma alanı SID'i için
    değiştirme izni taşır.
  - alan-a'da .env'e okuma reddi, .git\hooks ve .git\config'e yazma ve silme
    reddi (grup için).
- C:\KalfaPrototipProjeler: C:\ kökünün varsayılan izinleriyle bir klasör
  (C:\projeler benzeri).
- %LOCALAPPDATA%\KalfaPrototip\kanarya.txt: geliştiricinin profilinde bir dosya.
Çalışma alanı SID'leri %LOCALAPPDATA%\KalfaPrototip\agac.json'a yazılır.
İzinler .NET ile verilir: icacls hiçbir hesaba karşılık gelmeyen SID'leri
kabul etmiyor. Ağacı geri-al.ps1 ya da "cargo run -- olc temizle" siler.
#>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ortak.ps1')

$kurulum = Kurulumu-Oku
if (-not $kurulum) { throw 'Kurulum yok; önce kur.ps1.' }
$exe = Join-Path $PSScriptRoot 'target\debug\kalfa-prototip.exe'
if (-not (Test-Path $exe)) { throw 'Prototip derlenmemiş; önce: cargo build --offline' }

$Kok = 'C:\KalfaPrototip'
$Projeler = 'C:\KalfaPrototipProjeler'
foreach ($d in @($Kok, $Projeler)) { if (Test-Path $d) { Remove-Item -Recurse -Force $d } }

function Rastgele-Sayi {
    $b = New-Object byte[] 4
    $r = [Security.Cryptography.RandomNumberGenerator]::Create()
    $r.GetBytes($b)
    $r.Dispose()
    [BitConverter]::ToUInt32($b, 0)
}

# A7: iki biçim. A: S-1-9 altında rastgele; B: yerel hesap biçiminde rastgele.
$alanA = 'S-1-9-' + ((1..4 | ForEach-Object { Rastgele-Sayi }) -join '-')
$alanB = 'S-1-5-21-' + ((1..3 | ForEach-Object { Rastgele-Sayi }) -join '-') + '-' + (1000 + (Rastgele-Sayi) % 1000)
$grup = $kurulum.grup_sid
$gelistirici = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value

$miras = [Security.AccessControl.InheritanceFlags]'ContainerInherit,ObjectInherit'
$yok = [Security.AccessControl.InheritanceFlags]::None
$yayma = [Security.AccessControl.PropagationFlags]::None

function Kural([string]$sid, [string]$hak, [bool]$dizin, [string]$tur = 'Allow') {
    New-Object Security.AccessControl.FileSystemAccessRule(
        (New-Object Security.Principal.SecurityIdentifier $sid),
        [Security.AccessControl.FileSystemRights]$hak,
        $(if ($dizin) { $miras } else { $yok }),
        $yayma,
        [Security.AccessControl.AccessControlType]$tur)
}

function Kurallari-Ekle([string]$yol, [array]$kurallar) {
    $o = Get-Item -LiteralPath $yol -Force
    $acl = $o.GetAccessControl('Access')
    foreach ($k in $kurallar) { $acl.AddAccessRule($k) }
    $o.SetAccessControl($acl)
}

function Yaz([string]$yol, [string]$icerik) {
    New-Item -ItemType Directory -Force (Split-Path $yol) | Out-Null
    [IO.File]::WriteAllText($yol, $icerik)
}

# Kök: yalnızca geliştirici, SYSTEM ve Administrators; üstten miras yok.
New-Item -ItemType Directory -Force (Join-Path $Kok 'bin') | Out-Null
$kokAcl = New-Object Security.AccessControl.DirectorySecurity
$kokAcl.SetAccessRuleProtection($true, $false)
foreach ($s in @($gelistirici, 'S-1-5-18', 'S-1-5-32-544')) {
    $kokAcl.AddAccessRule((Kural $s 'FullControl' $true))
}
(Get-Item $Kok).SetAccessControl($kokAcl)

# bin: yalıtım okur ve çalıştırır.
Copy-Item $exe (Join-Path $Kok 'bin\kalfa-prototip.exe')
Kurallari-Ekle (Join-Path $Kok 'bin') @(
    (Kural $grup 'ReadAndExecute' $true),
    (Kural $kurulum.ortak_sid 'ReadAndExecute' $true))

# Çalışma alanları ve onlara özel dizinler.
foreach ($a in @(@('a', $alanA), @('b', $alanB))) {
    $ad = $a[0]; $sid = $a[1]
    $alan = Join-Path $Kok "alan-$ad"
    $ozel = Join-Path $Kok "ozel-$ad"
    foreach ($d in @($alan, "$ozel\ev\AppData\Roaming", "$ozel\ev\AppData\Local", "$ozel\gecici", "$ozel\cargo", "$ozel\npm")) {
        New-Item -ItemType Directory -Force $d | Out-Null
    }
    Yaz "$alan\normal.txt" "ALAN-$ad`n"
    foreach ($d in @($alan, $ozel)) {
        Kurallari-Ekle $d @((Kural $grup 'Modify' $true), (Kural $sid 'Modify' $true))
    }
}

# alan-a: gizli dosya, korumalı yollar ve iki küçük crate.
$a = Join-Path $Kok 'alan-a'
Yaz "$a\.env" "GIZLI-ENV`n"
Yaz "$a\.git\config" "[core]`n"
Yaz "$a\.git\hooks\pre-commit" "exit 0`n"
Yaz "$a\kutu\Cargo.toml" "[package]`nname = `"kutu`"`nversion = `"0.1.0`"`nedition = `"2021`"`n`n[workspace]`n"
Yaz "$a\kutu\src\main.rs" "fn main() { println!(`"kutu`"); }`n"
Yaz "$a\bagimli\Cargo.toml" "[package]`nname = `"bagimli`"`nversion = `"0.1.0`"`nedition = `"2021`"`n`n[workspace]`n`n[dependencies]`ncfg-if = `"=1.0.5`"`n"
Yaz "$a\bagimli\src\main.rs" "cfg_if::cfg_if! { if #[cfg(windows)] { fn f() -> &'static str { `"win`" } } else { fn f() -> &'static str { `"diger`" } } }`nfn main() { println!(`"{}`", f()); }`n"
# Ret maskeleri SYNCHRONIZE ve ReadPermissions içermez: onları reddetmek
# dosyayı okumak için açmayı da bozar.
Kurallari-Ekle "$a\.env" @((Kural $grup 'ReadData, ReadExtendedAttributes, ExecuteFile' $false 'Deny'))
Kurallari-Ekle "$a\.git\hooks" @((Kural $grup 'WriteData, AppendData, WriteExtendedAttributes, WriteAttributes, Delete, DeleteSubdirectoriesAndFiles' $true 'Deny'))
Kurallari-Ekle "$a\.git\config" @((Kural $grup 'WriteData, AppendData, WriteExtendedAttributes, WriteAttributes, Delete' $false 'Deny'))

# C:\projeler benzeri: C:\ kökünün varsayılan izinleriyle.
Yaz "$Projeler\gizli.txt" "PROJE-GIZLI`n"
Yaz "$Yerel\kanarya.txt" "PROFIL-KANARYA`n"

@{ alan_a_sid = $alanA; alan_b_sid = $alanB } | ConvertTo-Json | Set-Content -Encoding UTF8 (Join-Path $Yerel 'agac.json')
Write-Host "Deneme ağacı hazır: $Kok"
Write-Host "Alan A SID'i: $alanA"
Write-Host "Alan B SID'i: $alanB"
