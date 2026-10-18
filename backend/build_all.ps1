# build_all.ps1
$ANGULAR_PATH = "../frontend"
$TARGET_DIR = "target\release"

Write-Host "[1/4] Pokrecem Angular produkcioni build..." -ForegroundColor Cyan
Push-Location $ANGULAR_PATH
cmd /c ng build --configuration production
Pop-Location

# FIKSIRANO: Odmah kreiramo target/release folder tako da Cargo i fajlovi imaju gde da idu
if (-not (Test-Path $TARGET_DIR)) {
    New-Item -ItemType Directory -Force -Path $TARGET_DIR | Out-Null
}

Write-Host "[2/4] Kopiram Angular build direktno u produkcioni folder..." -ForegroundColor Cyan
# Brišemo stari frontend iz target/release ako postoji
$TARGET_FRONTEND = Join-Path $TARGET_DIR "frontend"
if (Test-Path $TARGET_FRONTEND) { Remove-Item $TARGET_FRONTEND -Recurse -Force }
New-Item -ItemType Directory -Force -Path $TARGET_FRONTEND | Out-Null

$ANGULAR_DIST_DIR = Get-ChildItem -Path "$ANGULAR_PATH\dist" -Directory | Select-Object -First 1

if ($ANGULAR_DIST_DIR) {
    $SOURCE_PATH = Join-Path $ANGULAR_DIST_DIR.FullName "browser"
    if (-not (Test-Path $SOURCE_PATH)) {
        $SOURCE_PATH = $ANGULAR_DIST_DIR.FullName
    }

    Write-Host "Kopiram fajlove sa: $SOURCE_PATH -> $TARGET_FRONTEND" -ForegroundColor Gray
    # Kopiramo direktno u target/release/frontend
    Get-ChildItem -Path $SOURCE_PATH | Copy-Item -Destination "$TARGET_FRONTEND\" -Recurse -Force
} else {
    Write-Warning "KRITIČNO: Angular build nije generisao fajlove!"
    exit 1
}

Write-Host "[3/4] Kompajliram Rust server (RELEASE MODE)..." -ForegroundColor Cyan
# Ako ste u Workspace-u, dodajte "-p backend" na kraj sledeće linije:
cargo build --release

Write-Host "[4/4] Kopiram SSL sertifikate u produkciju..." -ForegroundColor Cyan
Copy-Item "cert.pem" "$TARGET_DIR\" -Force
Copy-Item "key.pem" "$TARGET_DIR\" -Force

Write-Host "USPESNO ZAVRSENO! Sve je spremno u $TARGET_DIR folderu." -ForegroundColor Green
