# dev.ps1 (Smestiti u root workspace-a)
$env:RUN_ENV = "development"
$env:RUST_BACKTRACE = "full"

Write-Host "🚀 Pokrecem kompletan dev workspace..." -ForegroundColor Cyan

# 1. Pokretanje Angular frontenda u pozadini (odvojeni prozor da se ne mešaju logovi)
Write-Host "📦 Pokrecem Angular frontend (u novom prozoru)..." -ForegroundColor Yellow
Start-Process powershell -WorkingDirectory (Join-Path $PSScriptRoot "frontend") -ArgumentList "-NoExit", "-Command", "npm start"

# Alternativa ako ne želite novi prozor, otkomentarišite liniju ispod (ali logovi će se mešati):
# Start-Job -ScriptBlock { cd frontend; npm start }

# 2. Gasenje prethodne instance backenda (Windows ne dozvoljava cargo-u da prepise .exe koji je pokrenut)
$stari = Get-Process ogilenko-server -ErrorAction SilentlyContinue
if ($stari) {
    Write-Host "🛑 Gasim prethodnu instancu backenda (PID $($stari.Id -join ', '))..." -ForegroundColor Red
    $stari | Stop-Process -Force
    $stari | Wait-Process -Timeout 10 -ErrorAction SilentlyContinue
}

# 3. Pokretanje Rust backenda u trenutnom prozoru (ovde vidite logove i backtrace)
Write-Host "🦀 Pokrecem Rust backend..." -ForegroundColor Green
Push-Location (Join-Path $PSScriptRoot "backend")
try {
    cargo run -- local
} finally {
    Pop-Location
}
