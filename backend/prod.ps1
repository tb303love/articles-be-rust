# prod.ps1
$env:RUN_ENV="production"
Write-Host "[PROD] Pokrecem server u PRODUCTION rezimu..." -ForegroundColor Yellow
./target/debug/ogilenko-server.exe
