param(
    [string]$Yardimci
)

$ErrorActionPreference = 'Stop'
$kimlik = [Security.Principal.WindowsIdentity]::GetCurrent()
$yonetici = [Security.Principal.WindowsPrincipal]::new($kimlik)
if (-not $yonetici.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Bu betiği yönetici PowerShell içinde siz çalıştırın.'
}
$kuruluYardimci = Join-Path $env:ProgramFiles 'Kalfa\kalfa-yalitim.exe'
$kurtarmaYardimcisi = Join-Path $env:ProgramFiles 'Kalfa-Kurtarma\kalfa-yalitim.exe'
if (-not $Yardimci) {
    if (Test-Path -LiteralPath $kuruluYardimci -PathType Leaf) {
        $Yardimci = $kuruluYardimci
    } else {
        $Yardimci = $kurtarmaYardimcisi
    }
}
if (-not (Test-Path -LiteralPath $Yardimci -PathType Leaf)) {
    throw "Yalıtım yardımcısı bulunamadı: $Yardimci"
}

& $Yardimci kaldir
if ($LASTEXITCODE -ne 0) {
    throw "Yalıtım izinleri geri alınamadı (çıkış kodu $LASTEXITCODE). Kayıt ve izinleri BT departmanınızla inceleyin."
}
$kayit = Join-Path $env:ProgramData 'Kalfa\yalitim\kurulum.json'
if (Test-Path -LiteralPath $kayit) {
    throw "Yardımcı başarı bildirdi ama kurulum kaydı duruyor: $kayit"
}
if ($Yardimci -eq $kurtarmaYardimcisi) {
    Remove-Item -LiteralPath $kurtarmaYardimcisi -Force
    Remove-Item -LiteralPath (Split-Path -Parent $kurtarmaYardimcisi) -ErrorAction SilentlyContinue
}
Write-Host 'Yalıtımın yönetici izinleri geri alındı. Uygulamayı Windows Ayarlar üzerinden kaldırabilirsiniz.'
