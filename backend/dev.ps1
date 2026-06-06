# dev.ps1
$env:RUN_ENV="development"
# DODATO: Uključuje standardni backtrace za praćenje grešaka pri padu koda
$env:RUST_BACKTRACE=1

# Opciono: Ako želiš maksimalno detaljan backtrace (prikazuje i unutrašnje funkcije biblioteka), stavi:
# $env:RUST_BACKTRACE="full"

Write-Host "🚀 Pokrecem server u DEVELOPMENT rezimu sa ukljucenim Backtrace-om..." -ForegroundColor Green
cargo run -- local
