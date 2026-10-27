# prod.ps1 - lokalno pokretanje RELEASE build-a sa produkcionom konfiguracijom (backend\.env.production)
# Prvo pokrenuti .\build.ps1. Kod klijenta server radi kao Windows servis (vidi backend\wix\main.wxs).
Set-Location $PSScriptRoot

$Exe = "target\release\ogilenko-server.exe"
if (-not (Test-Path $Exe)) {
    Write-Host "GRESKA: $Exe ne postoji - prvo pokrenite .\build.ps1" -ForegroundColor Red
    exit 1
}

# Ucitavamo produkcione vrednosti (DATABASE_URL, SERVER_PORT, JWT_SECRET...) u env ovog procesa
foreach ($line in Get-Content "backend\.env.production") {
    if ($line -match '^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=(.*)$') {
        Set-Item -Path "Env:$($Matches[1])" -Value $Matches[2].Trim()
    }
}
# RUN_ENV=production => server postavlja radni folder na folder .exe-a, pa je putanja relativna na target\release
$env:FRONTEND_DIR = "./frontend"

Write-Host "[PROD] Pokrecem server sa produkcionom konfiguracijom..." -ForegroundColor Yellow
# `local` => direktno pokretanje u konzoli umesto registracije Windows servisa
& $Exe local
