# dev.ps1 (Smestiti u root workspace-a)
$env:RUN_ENV = "development"
$env:RUST_BACKTRACE = "full"

Write-Host "🚀 Pokrecem kompletan dev workspace..." -ForegroundColor Cyan

# 1. Pokretanje Angular frontenda u pozadini (odvojeni prozor da se ne mešaju logovi)
Write-Host "📦 Pokrecem Angular frontend (u novom prozoru)..." -ForegroundColor Yellow
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd frontend; npm start"

# Alternativa ako ne želite novi prozor, otkomentarišite liniju ispod (ali logovi će se mešati):
# Start-Job -ScriptBlock { cd frontend; npm start }

# 2. Pokretanje Rust backenda u trenutnom prozoru (ovde vidite logove i backtrace)
Write-Host "🦀 Pokrecem Rust backend..." -ForegroundColor Green
cd backend
cargo run -- local
