# install.ps1 - pravi MSI instaler: kompletan build (build.ps1) + cargo wix
# Pokretanje iz korena projekta: .\install.ps1
Set-Location $PSScriptRoot

Write-Host "[1/2] Kompletan produkcioni build..." -ForegroundColor Cyan
& .\build.ps1
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host "[2/2] Pravim MSI instaler (WiX Toolset)..." -ForegroundColor Cyan
# cargo wix se pokrece iz backend\ jer su putanje u wix\main.wxs (cert.pem, key.pem) relativne na taj folder.
# --no-build: build.ps1 je vec napravio release binarni fajl.
Push-Location backend
cargo wix --no-build --nocapture
$wixExit = $LASTEXITCODE
Pop-Location

if ($wixExit -ne 0) {
    Write-Host "GRESKA: cargo wix (exit $wixExit)" -ForegroundColor Red
    exit 1
}

$msi = Get-ChildItem "target\wix\*.msi" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
Write-Host "GOTOVO! Instaler: $($msi.FullName)" -ForegroundColor Green
