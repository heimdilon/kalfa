# Windows NSIS kurucusunun sürüm iş akışındaki sınaması: geçici CI makinesinde
# sessiz kurar, dosyaları ve yalıtımın kök izinlerini denetler, sessiz kaldırır
# ve izinlerin geri alındığını doğrular. Kurucu yönetici ister ve sistem ACL'sini
# değiştirir; bu betik yalnızca CI makinesi içindir, geliştirici makinesinde
# çalıştırmayın (elle kabul için kurucu/kabul.ps1).
param(
    [Parameter(Mandatory = $true)] [string]$KurucuYolu,
    [switch]$G5M01,
    [switch]$G5Masaustu
)

$ErrorActionPreference = 'Stop'
if (($G5M01 -or $G5Masaustu) -and $env:GITHUB_ACTIONS -ne 'true') {
    throw 'G5 M01 yalnız geçici GitHub CI makinesinde çalıştırılır.'
}
$kurucu = (Resolve-Path -LiteralPath $KurucuYolu).Path
$kok = Join-Path $env:ProgramFiles 'Kalfa'
$kayitYolu = Join-Path $env:ProgramData 'Kalfa\yalitim\kurulum.json'

# Kökte yalıtımın yetenek SID'ine verilmiş mirassız izin kuralları.
function Yetenek-Kurallari([string]$Yol) {
    @((Get-Acl -LiteralPath $Yol).Access | Where-Object {
        $sid = try {
            $_.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value
        } catch {
            $_.IdentityReference.Value
        }
        $sid -like 'S-1-15-3-1024-*' -and
            $_.AccessControlType -eq [Security.AccessControl.AccessControlType]::Allow -and
            -not $_.IsInherited
    })
}

# Kurulumun yazması gereken izin (kabul.ps1 ile aynı): yayılmayan ve en az
# listeleme/okuma/geçiş haklarını (0x1000A1) taşıyan kural.
function Kok-Izni-Tam([string]$Yol) {
    @(Yetenek-Kurallari $Yol | Where-Object {
        $_.InheritanceFlags -eq [Security.AccessControl.InheritanceFlags]::None -and
            (([int]$_.FileSystemRights -band 0x1000A1) -eq 0x1000A1)
    }).Count -gt 0
}

Write-Host '== Sessiz kurulum'
$p = Start-Process -FilePath $kurucu -ArgumentList '/S' -Wait -PassThru
if ($p.ExitCode -ne 0) { throw "Kurucu başarısız (çıkış kodu $($p.ExitCode))." }

$dosyalar = @(
    'kalfa.exe', 'kalfa-yalitim.exe', 'kalfa-kopru.exe',
    'araclar\python\python.exe', 'araclar\node\node.exe',
    'eklentiler\kalfa-rag\eklenti.wasm', 'eklentiler\kalfa-rag\kalfa.toml'
)
foreach ($d in $dosyalar) {
    if (-not (Test-Path -LiteralPath (Join-Path $kok $d) -PathType Leaf)) {
        throw "Kurulumda eksik dosya: $d"
    }
}
Write-Host "Dosyalar tamam ($($dosyalar.Count))."

if (-not (Test-Path -LiteralPath $kayitYolu -PathType Leaf)) {
    throw 'Yalıtımın kurulum kaydı oluşmadı.'
}
$kokler = @((Get-Content -LiteralPath $kayitYolu -Raw | ConvertFrom-Json).kok_izinleri) |
    Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
if ($kokler.Count -eq 0) { throw 'Kurulum kaydında kök izni yok.' }
foreach ($k in $kokler) {
    if (-not (Kok-Izni-Tam $k)) { throw "Kök listeleme izni eksik ya da hatalı: $k" }
}
Write-Host "Yalıtım kök izinleri tamam: $($kokler -join ', ')"

# M01 başarısız olsa da kurucunun kaldırma ve izin geri alma denetimi koşar.
$g5Hatasi = $null
if ($G5M01 -or $G5Masaustu) {
    try {
        & pwsh -NoProfile -File (Join-Path $PSScriptRoot 'windows-testleri.ps1') -G5M01:$G5M01 -G5Masaustu:$G5Masaustu
        if ($LASTEXITCODE -ne 0) { throw "G5 M01 başarısız: $LASTEXITCODE" }
    } catch { $g5Hatasi = $_ }
}

Write-Host '== Sessiz kaldırma'
# `_?=` kaldırıcının kendini geçici dizine kopyalayıp hemen dönmesini önler.
$kaldirici = Join-Path $kok 'uninstall.exe'
$p = Start-Process -FilePath $kaldirici -ArgumentList '/S', "_?=$kok" -Wait -PassThru
if ($p.ExitCode -ne 0) { throw "Kaldırıcı başarısız (çıkış kodu $($p.ExitCode))." }
# `_?=` ile çalışan kaldırıcı yalnız kendini bırakır; kurulan her dosya gitmeli.
foreach ($d in $dosyalar) {
    if (Test-Path -LiteralPath (Join-Path $kok $d)) { throw "Kaldırılmayan dosya: $d" }
}
foreach ($d in 'araclar', 'eklentiler') {
    if (Test-Path -LiteralPath (Join-Path $kok $d)) { throw "Kaldırılmayan dizin: $d" }
}
if (Test-Path -LiteralPath $kayitYolu) { throw 'Yalıtımın kurulum kaydı kaldırılmadı.' }
foreach ($k in $kokler) {
    if ((Yetenek-Kurallari $k).Count -gt 0) { throw "Kök izni geri alınmadı: $k" }
}
Write-Host 'Kaldırma tamam: dosyalar, kurulum kaydı ve kök izinleri gitti.'

if ($null -ne $g5Hatasi) { throw $g5Hatasi }
