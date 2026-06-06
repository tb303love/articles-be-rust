# build_all.ps1
$ANGULAR_PATH = "../frontend"

Write-Host "[1/4] Pokrecem Angular produkcioni build..." -ForegroundColor Cyan
Push-Location $ANGULAR_PATH
# Pokrećemo ng build i čekamo da završi
ng build --configuration production
Pop-Location

Write-Host "[2/4] Osvezavam frontend folder u Rust projektu..." -ForegroundColor Cyan
# Brišemo stari frontend folder iz Rusta ako postoji
if (Test-Path "frontend") { Remove-Item "frontend" -Recurse -Force }
# Kreiramo čist folder
New-Item -ItemType Directory -Force -Path "frontend" | Out-Null

# FIKSIRANO: Prvo dinamički pronalazimo gde je Angular tačno izbacio build fajlove.
# Ovo radi bez obzira na ime Angular projekta i rešava problem sa džoker znakom (*).
$ANGULAR_DIST_DIR = Get-ChildItem -Path "$ANGULAR_PATH\dist" -Directory | Select-Object -First 1

if ($ANGULAR_DIST_DIR) {
    # Proveravamo da li postoji podfolder 'browser' (Angular 17+) ili su fajlovi direktno u dist folderu
    $SOURCE_PATH = Join-Path $ANGULAR_DIST_DIR.FullName "browser"
    if (-not (Test-Path $SOURCE_PATH)) {
        $SOURCE_PATH = $ANGULAR_DIST_DIR.FullName
    }

    Write-Host "Pronađen Angular build na lokaciji: $SOURCE_PATH" -ForegroundColor Gray

    # FIKSIRANO: Kopiramo sadržaj foldera pravilno koristeći Get-ChildItem pipeline
    Get-ChildItem -Path $SOURCE_PATH | Copy-Item -Destination "frontend\" -Recurse -Force
} else {
    Write-Warning "KRITIČNO: Angular build nije generisao fajlove unutar dist foldera!"
}

Write-Host "[3/4] Kompajliram Rust server..." -ForegroundColor Cyan
cargo build

Write-Host "[4/4] Pripremam fajlove za lokalno testiranje produkcije..." -ForegroundColor Cyan
# Kreiramo strukturu u target/debug/ kako bi prod.ps1 i servis odmah radili
Copy-Item "cert.pem" "target\debug\" -Force
Copy-Item "key.pem" "target\debug\" -Force

if (Test-Path "target\debug\frontend") { Remove-Item "target\debug\frontend" -Recurse -Force }
# Kopiramo kompletan folder u target\debug
Copy-Item "frontend" "target\debug\" -Recurse -Force

Write-Host "USPESNO ZAVRSENO! Sve je spremno." -ForegroundColor Green
