#Requires -RunAsAdministrator
<#
Kalfa 2c prototipi, AppContainer ek ölçümü: kurulum. Yönetici PowerShell'de,
geliştiricinin kendi hesabıyla yükseltilerek, bu dizinde:

    cargo build --offline
    powershell -ExecutionPolicy Bypass -File .\kap-kur.ps1

Yerel kullanıcı, grup, oturum açma hakkı ya da güvenlik duvarı kuralı yoktur.
Yaptıkları (hepsini kap-geri-al.ps1 geri alır):
- AppContainer profilleri (Kalfa.Prototip.AlanA, Kalfa.Prototip.AlanB) yoksa
  "kalfa-prototip kap hazirla" ile oluşturulur. Paket SID'leri ve araç zinciri
  yeteneğinin SID'i %LOCALAPPDATA%\KalfaPrototip\kap.json'dadır.
- Araç zinciri dizinlerine (.cargo\bin, .cargo\registry, .rustup,
  C:\Python314, C:\Program Files\nodejs) araç zinciri yeteneği için okuma izni.
  .cargo'nun kendisine izin verilmez: credentials.toml orada durur. Yönetici,
  C:\Python314 ve nodejs için gerekir.
- C:\ kökü: araç zinciri yeteneğine yalnızca kökü listeleme ve özniteliklerini
  okuma izni (miras verilmez; SetFileSecurityW ile, sürücüye yayılmadan). Git,
  cmd ve Node yolları üst dizinleri listeleyerek kanonikleştiriyor.
- Deneme ağacı C:\KalfaPrototip (her çalıştırmada sıfırdan):
  - Kök: geliştirici, SYSTEM ve Administrators; araç zinciri yeteneği yalnızca
    kökü listeleyebilir (ölçüm ağacı bir sürücü harfine bağlar).
  - bin\: prototipin kopyası; iki paket okur ve çalıştırır.
  - alan-a, alan-b, ozel-a, ozel-b: kendi paketine değiştirme izni ve Low
    bütünlük etiketi (AppContainer Low bütünlükte çalışır).
  - etiketsiz-a: A'ya değiştirme izni, etiket yok (karşılaştırma).
  - alan-a'da korumalı yollar yalnızca izinle çizilir; ret izinleri
    AppContainer'da paket SID'i için işlemiyor (ölçüldü):
    - .env: mirası kesilir, pakete izin yok.
    - .git: pakete kendisi için okuma ve yazma (silme yok), içi için
      değiştirme; .git yeniden adlandırılamaz.
    - .git\hooks ve .git\config: mirası kesilir, pakete yalnızca okuma.
    - ret-deneme.txt: eski yöntem, pakete okuma reddi (karşılaştırma).
- C:\KalfaPrototipProjeler ve %LOCALAPPDATA%\KalfaPrototip\kanarya.txt:
  okunmaması gereken iki dosya.
#>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ortak.ps1')

$KapKaydi = Join-Path $Yerel 'kap.json'
$KapKurulum = Join-Path $Yerel 'kap-kurulum.json'
$exe = Join-Path $PSScriptRoot 'target\debug\kalfa-prototip.exe'
if (-not (Test-Path $exe)) { throw 'Prototip derlenmemiş; önce: cargo build --offline' }

if (-not (Test-Path $KapKaydi)) {
    & $exe kap hazirla
    if ($LASTEXITCODE -ne 0) { throw 'kap hazirla başarısız.' }
}
$kap = Get-Content -Raw -Encoding UTF8 $KapKaydi | ConvertFrom-Json
$paketA = $kap.alan_a_sid
$paketB = $kap.alan_b_sid
$arac = $kap.arac_sid

# Yarıda kalmış bir kurulum yeniden çalıştırılabilir: izin verilmiş dizinler
# atlanır, ağaç sıfırdan kurulur.
$kurulum = [ordered]@{ izinli_dizinler = @(); kok_izni = $false }
if (Test-Path $KapKurulum) {
    $eski = Get-Content -Raw -Encoding UTF8 $KapKurulum | ConvertFrom-Json
    $kurulum.izinli_dizinler = @($eski.izinli_dizinler)
    $kurulum.kok_izni = [bool]$eski.kok_izni
}
function Kaydet { $kurulum | ConvertTo-Json | Set-Content -Encoding UTF8 $KapKurulum }
# Kayıt hemen yazılır: sonraki adım düşerse kap-geri-al.ps1 yine çalışır.
Kaydet

Write-Host 'Araç zinciri dizinlerine okuma izni…'
$adaylar = @(
    (Join-Path $env:USERPROFILE '.cargo\bin'),
    (Join-Path $env:USERPROFILE '.cargo\registry'),
    (Join-Path $env:USERPROFILE '.rustup'),
    'C:\Python314',
    'C:\Program Files\nodejs'
)
foreach ($d in $adaylar) {
    if (-not (Test-Path $d) -or $kurulum.izinli_dizinler -contains $d) { continue }
    Izin-Ver $d @($arac)
    $kurulum.izinli_dizinler += $d
    Kaydet
}

if (-not $kurulum.kok_izni) {
    Write-Host 'C:\ köküne listeleme izni…'
    # Kayıt önce yazılır: izin yarıda kalsa da kap-geri-al.ps1 kaldırmayı dener.
    $kurulum.kok_izni = $true
    Kaydet
    Kok-Izni-Ver $arac
}

Write-Host 'Deneme ağacı…'
$Kok = 'C:\KalfaPrototip'
$Projeler = 'C:\KalfaPrototipProjeler'
foreach ($d in @($Kok, $Projeler)) { if (Test-Path $d) { Remove-Item -Recurse -Force $d } }
$gelistirici = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value

$miras = [Security.AccessControl.InheritanceFlags]'ContainerInherit,ObjectInherit'
$yok = [Security.AccessControl.InheritanceFlags]::None
$yayma = [Security.AccessControl.PropagationFlags]::None

function Kural([string]$sid, [string]$hak, [bool]$dizin, [string]$tur = 'Allow', [string]$yayilim = 'None') {
    New-Object Security.AccessControl.FileSystemAccessRule(
        (New-Object Security.Principal.SecurityIdentifier $sid),
        [Security.AccessControl.FileSystemRights]$hak,
        $(if ($dizin) { $miras } else { $yok }),
        [Security.AccessControl.PropagationFlags]$yayilim,
        [Security.AccessControl.AccessControlType]$tur)
}

# Mirası kesilmiş izin: geliştirici, SYSTEM ve Administrators tam; ek kurallar.
function Korumali([string]$yol, [bool]$dizin, [array]$ek) {
    $o = Get-Item -LiteralPath $yol -Force
    $acl = if ($dizin) { New-Object Security.AccessControl.DirectorySecurity } else { New-Object Security.AccessControl.FileSecurity }
    $acl.SetAccessRuleProtection($true, $false)
    foreach ($s in @($gelistirici, 'S-1-5-18', 'S-1-5-32-544')) {
        $acl.AddAccessRule((Kural $s 'FullControl' $dizin))
    }
    foreach ($k in $ek) { $acl.AddAccessRule($k) }
    $o.SetAccessControl($acl)
}

function Kurallari-Ekle([string]$yol, [array]$kurallar) {
    $o = Get-Item -LiteralPath $yol -Force
    $acl = $o.GetAccessControl('Access')
    foreach ($k in $kurallar) { $acl.AddAccessRule($k) }
    $o.SetAccessControl($acl)
}

function Low-Etiketi([string]$yol) {
    & icacls $yol /setintegritylevel '(OI)(CI)L' | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Bütünlük etiketi konamadı: $yol" }
}

function Yaz([string]$yol, [string]$icerik) {
    New-Item -ItemType Directory -Force (Split-Path $yol) | Out-Null
    [IO.File]::WriteAllText($yol, $icerik)
}

# Kök: geliştirici, SYSTEM ve Administrators; üstten miras yok. Araç zinciri
# yeteneği yalnızca kökü listeler (sürücü kipinde kök, sürücünün köküdür).
New-Item -ItemType Directory -Force (Join-Path $Kok 'bin') | Out-Null
$kokAcl = New-Object Security.AccessControl.DirectorySecurity
$kokAcl.SetAccessRuleProtection($true, $false)
foreach ($s in @($gelistirici, 'S-1-5-18', 'S-1-5-32-544')) {
    $kokAcl.AddAccessRule((Kural $s 'FullControl' $true))
}
$kokAcl.AddAccessRule((Kural $arac 'ListDirectory, ReadAttributes, Traverse, Synchronize' $false))
(Get-Item $Kok).SetAccessControl($kokAcl)

Copy-Item $exe (Join-Path $Kok 'bin\kalfa-prototip.exe')
Kurallari-Ekle (Join-Path $Kok 'bin') @((Kural $paketA 'ReadAndExecute' $true), (Kural $paketB 'ReadAndExecute' $true))

foreach ($alanTanimi in @(@('a', $paketA), @('b', $paketB))) {
    $ad = $alanTanimi[0]; $sid = $alanTanimi[1]
    $alan = Join-Path $Kok "alan-$ad"
    $ozel = Join-Path $Kok "ozel-$ad"
    foreach ($d in @($alan, "$ozel\ev\AppData\Roaming", "$ozel\ev\AppData\Local", "$ozel\gecici", "$ozel\cargo", "$ozel\npm")) {
        New-Item -ItemType Directory -Force $d | Out-Null
    }
    Yaz "$alan\normal.txt" "ALAN-$ad`n"
    foreach ($d in @($alan, $ozel)) {
        Kurallari-Ekle $d @((Kural $sid 'Modify' $true))
        Low-Etiketi $d
    }
}

$e = Join-Path $Kok 'etiketsiz-a'
Yaz "$e\normal.txt" "ETIKETSIZ`n"
Kurallari-Ekle $e @((Kural $paketA 'Modify' $true))

# alan-a: gizli dosya, korumalı yollar ve iki küçük crate.
$alanA = Join-Path $Kok 'alan-a'
Yaz "$alanA\.env" "GIZLI-ENV`n"
Yaz "$alanA\ret-deneme.txt" "RET-DENEME`n"
Yaz "$alanA\.git\config" "[core]`n"
Yaz "$alanA\.git\hooks\pre-commit" "exit 0`n"
Yaz "$alanA\kutu\Cargo.toml" "[package]`nname = `"kutu`"`nversion = `"0.1.0`"`nedition = `"2021`"`n`n[workspace]`n"
Yaz "$alanA\kutu\src\main.rs" "fn main() { println!(`"kutu`"); }`n"
Yaz "$alanA\bagimli\Cargo.toml" "[package]`nname = `"bagimli`"`nversion = `"0.1.0`"`nedition = `"2021`"`n`n[workspace]`n`n[dependencies]`ncfg-if = `"=1.0.5`"`n"
Yaz "$alanA\bagimli\src\main.rs" "cfg_if::cfg_if! { if #[cfg(windows)] { fn f() -> &'static str { `"win`" } } else { fn f() -> &'static str { `"diger`" } } }`nfn main() { println!(`"{}`", f()); }`n"
# Korumalı yollar: yalnızca izin. Önce .git (alt öğelere yayılır), sonra
# mirası kesilen çocuklar.
Korumali "$alanA\.env" $false @()
Korumali "$alanA\.git" $true @(
    (Kural $paketA 'ReadAndExecute, Write' $false),
    (Kural $paketA 'Modify' $true 'Allow' 'InheritOnly'))
Korumali "$alanA\.git\hooks" $true @((Kural $paketA 'ReadAndExecute' $true))
Korumali "$alanA\.git\config" $false @((Kural $paketA 'ReadAndExecute' $false))
# Eski yöntem, karşılaştırma için. Ret maskeleri SYNCHRONIZE ve
# ReadPermissions içermez: onları reddetmek dosyayı açmayı da bozar.
Kurallari-Ekle "$alanA\ret-deneme.txt" @((Kural $paketA 'ReadData, ReadExtendedAttributes, ExecuteFile' $false 'Deny'))

Yaz "$Projeler\gizli.txt" "PROJE-GIZLI`n"
Yaz "$Yerel\kanarya.txt" "PROFIL-KANARYA`n"

Write-Host ''
Write-Host "Kurulum tamam. Kayıt: $KapKurulum"
Write-Host "Alan A paketi: $paketA"
Write-Host "Alan B paketi: $paketB"
Write-Host "Araç zinciri yeteneği: $arac"
Write-Host ("C:\ kök izni: {0}" -f $kurulum.kok_izni)
Write-Host 'İzin verilen dizinler:'
$kurulum.izinli_dizinler | ForEach-Object { Write-Host "  $_" }
