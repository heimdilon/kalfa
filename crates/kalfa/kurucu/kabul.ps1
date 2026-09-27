param(
    [Parameter(Mandatory = $true)] [string]$KurucuYolu,
    [string]$EskiYardimci
)

$ErrorActionPreference = 'Stop'
$kimlik = [Security.Principal.WindowsIdentity]::GetCurrent()
$yonetici = [Security.Principal.WindowsPrincipal]::new($kimlik)
if (-not $yonetici.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Bu betiği yönetici PowerShell içinde siz çalıştırın; UAC ve kurucu penceresini siz yönetin.'
}
$kurucu = (Resolve-Path -LiteralPath $KurucuYolu).Path
$kayitYolu = Join-Path $env:ProgramData 'Kalfa\yalitim\kurulum.json'
$eskiDizinler = @()
$eskiKaldirildi = $false

if (Test-Path -LiteralPath $kayitYolu) {
    if (-not $EskiYardimci) {
        throw 'Önceki kurulum kaydı var. Eski yardımcının yolunu -EskiYardimci ile verin.'
    }
    $eski = (Resolve-Path -LiteralPath $EskiYardimci).Path
    $eskiKayit = Get-Content -LiteralPath $kayitYolu -Raw | ConvertFrom-Json
    $eskiDizinler = @($eskiKayit.izinli_dizinler)
    & $eski kaldir
    if ($LASTEXITCODE -ne 0) {
        throw "Önceki yalıtım kurulumu kaldırılamadı (çıkış kodu $LASTEXITCODE)."
    }
    $eskiKaldirildi = $true
}

try {
    # Kurucu etkileşimlidir; pencere kullanıcıya görünür. `-Wait` kurucunun
    # son sayfasında açılan Kalfa'yı da beklerdi; yalnızca kurucu beklenir.
    $sonuc = Start-Process -FilePath $kurucu -PassThru
    $sonuc.WaitForExit()
    if ($sonuc.ExitCode -ne 0) {
        throw "Kurucu başarısız oldu (çıkış kodu $($sonuc.ExitCode))."
    }
    if (-not (Test-Path -LiteralPath $kayitYolu -PathType Leaf)) {
        throw 'Kurucu bitti fakat yalıtımın kurulum kaydı oluşturulmadı.'
    }
    $yeniKayit = Get-Content -LiteralPath $kayitYolu -Raw | ConvertFrom-Json
    $kokler = @($yeniKayit.kok_izinleri) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    if ($kokler.Count -eq 0) {
        throw 'Kurulum kaydı var fakat kök izinleri kaydedilmemiş.'
    }
    foreach ($kok in $kokler) {
        $kurallar = (Get-Acl -LiteralPath $kok).Access
        $izinVar = @($kurallar | Where-Object {
            $sid = try {
                $_.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value
            } catch {
                $_.IdentityReference.Value
            }
            $sid -like 'S-1-15-3-1024-*' -and
                $_.AccessControlType -eq [Security.AccessControl.AccessControlType]::Allow -and
                -not $_.IsInherited -and
                $_.InheritanceFlags -eq [Security.AccessControl.InheritanceFlags]::None -and
                (([int]$_.FileSystemRights -band 0x1000A1) -eq 0x1000A1)
        }).Count -gt 0
        if (-not $izinVar) {
            throw "Kurulum kaydı var fakat kök listeleme izni eksik: $kok"
        }
    }
    $yeni = Join-Path $env:ProgramFiles 'Kalfa\kalfa-yalitim.exe'
    foreach ($dizin in $eskiDizinler) {
        & $yeni izin-ver $dizin
        if ($LASTEXITCODE -ne 0) {
            throw "Önceden açık araç zinciri dizini geri yüklenemedi: $dizin"
        }
    }
    Write-Host 'Kurulum kaydı oluştu. Kalfa içinde yalıtım durumunu ve Onar akışını sınayın; ardından Windows Ayarlar üzerinden kaldırın.'
} catch {
    $kurulu = Join-Path $env:ProgramFiles 'Kalfa\kalfa-yalitim.exe'
    if (Test-Path -LiteralPath $kurulu -PathType Leaf) {
        try {
            & (Join-Path $PSScriptRoot 'geri-al.ps1') -Yardimci $kurulu
        } catch {
            Write-Warning "Yeni yalıtım izinleri otomatik geri alınamadı: $_"
        }
    }
    if ($eskiKaldirildi) {
        & $eski kur
        if ($LASTEXITCODE -ne 0) {
            Write-Warning 'Önceki yalıtım kurulumu geri yüklenemedi.'
        } else {
            foreach ($dizin in $eskiDizinler) {
                & $eski izin-ver $dizin
                if ($LASTEXITCODE -ne 0) { Write-Warning "Önceki araç zinciri izni geri yüklenemedi: $dizin" }
            }
        }
    }
    throw
}
