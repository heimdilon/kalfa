# Yalnız geçici, kurucusu kurulmuş GitHub CI makinesi. Standart kullanıcıdan çağrılır.
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:KALFA_G5_MASAUSTU_CI -ne '1') {
    throw 'Gerçek Windows masaüstü kabulü yalnız geçici kurucu CI makinesindedir.'
}
$env:KALFA_E2E_UYGULAMA = Join-Path $env:GITHUB_WORKSPACE 'kabul\kalfa.exe'
$env:KALFA_E2E_EDGEDRIVER = Join-Path $env:GITHUB_WORKSPACE 'kabul\suruculer\msedgedriver.exe'
$env:KALFA_ARAC_ZINCIRI = Join-Path $env:ProgramFiles 'Kalfa\araclar'
$env:PATH = (Join-Path $env:GITHUB_WORKSPACE 'kabul') + ';' + $env:PATH
$gunluk = Join-Path $env:TEMP 'g5-masaustu.log'
Push-Location (Join-Path $env:GITHUB_WORKSPACE 'e2e')
try {
    node --test --test-concurrency=1 --test-reporter=tap yetenekler/g5-proje-akisi.test.mjs yetenekler/g5-surec.test.mjs yetenekler/g5-web-guvenligi.test.mjs testler/13-mcp.test.mjs *> $gunluk
    $kod = $LASTEXITCODE
    $metin = Get-Content -LiteralPath $gunluk -Raw
    Write-Output $metin
    foreach ($satir in '# tests 6', '# pass 6', '# fail 0', '# skipped 0') {
        if ($metin -notmatch ('(?m)^' + [regex]::Escape($satir) + '\r?$')) {
            throw "Gerçek masaüstü kabul sayımı eksik: $satir"
        }
    }
    if ($kod -ne 0) { throw "Masaüstü kabulü başarısız: $kod" }
} finally { Pop-Location }
