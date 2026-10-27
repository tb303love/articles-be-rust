# build.ps1 - produkcioni build: Angular + Rust (release) + WiX fragment za frontend
# Pokretanje iz korena projekta: .\build.ps1
Set-Location $PSScriptRoot

$AngularDist = "frontend\dist\browser"
$StagedFront = "backend\frontend"           # odavde WiX pakuje frontend u MSI
$WixFragment = "backend\wix\frontend.wxs"   # generisan, ne commit-uje se
$ReleaseDir  = "target\release"             # workspace target (root Cargo.toml)

function Assert-Exit([int]$Code, [string]$Step) {
    if ($Code -ne 0) {
        Write-Host "GRESKA: $Step (exit $Code)" -ForegroundColor Red
        exit 1
    }
}

# Stabilan WiX ID iz relativne putanje - isti fajl dobija isti ID u svakom build-u
$Sha1 = [System.Security.Cryptography.SHA1]::Create()
function Get-WixId([string]$Prefix, [string]$RelPath) {
    $bytes = [Text.Encoding]::UTF8.GetBytes($RelPath.ToLowerInvariant())
    $hash = -join ($Sha1.ComputeHash($bytes) | Select-Object -First 10 | ForEach-Object { $_.ToString("x2") })
    "${Prefix}_$hash"
}

# Rekurzivno opisuje folder kao WiX Directory/Component elemente (jedan fajl = jedna komponenta)
function Add-WixDirectory([string]$Dir, [int]$Depth) {
    $indent = "  " * $Depth
    foreach ($file in Get-ChildItem -LiteralPath $Dir -File | Sort-Object Name) {
        $rel = $file.FullName.Substring($script:RootFull.Length + 1)
        $id = Get-WixId "fe" $rel
        $script:ComponentIds.Add($id)
        $src = [Security.SecurityElement]::Escape($file.FullName)
        [void]$script:Xml.AppendLine("$indent<Component Id=`"$id`" Guid=`"*`" Win64=`"yes`">")
        [void]$script:Xml.AppendLine("$indent  <File Id=`"$id`" Source=`"$src`" KeyPath=`"yes`" />")
        [void]$script:Xml.AppendLine("$indent</Component>")
    }
    foreach ($sub in Get-ChildItem -LiteralPath $Dir -Directory | Sort-Object Name) {
        $rel = $sub.FullName.Substring($script:RootFull.Length + 1)
        $name = [Security.SecurityElement]::Escape($sub.Name)
        [void]$script:Xml.AppendLine("$indent<Directory Id=`"$(Get-WixId 'fd' $rel)`" Name=`"$name`">")
        Add-WixDirectory $sub.FullName ($Depth + 1)
        [void]$script:Xml.AppendLine("$indent</Directory>")
    }
}

Write-Host "[1/4] Angular produkcioni build..." -ForegroundColor Cyan
Push-Location frontend
npm run build -- --configuration production
$angularExit = $LASTEXITCODE
Pop-Location
Assert-Exit $angularExit "Angular build"

if (-not (Test-Path $AngularDist)) {
    Write-Host "GRESKA: Angular build nije napravio $AngularDist" -ForegroundColor Red
    exit 1
}

Write-Host "[2/4] Pripremam frontend za instaler ($StagedFront)..." -ForegroundColor Cyan
if (Test-Path $StagedFront) { Remove-Item $StagedFront -Recurse -Force -ErrorAction Stop }
Copy-Item $AngularDist $StagedFront -Recurse -ErrorAction Stop

$script:RootFull = (Resolve-Path $StagedFront).Path
$script:ComponentIds = New-Object System.Collections.Generic.List[string]
$script:Xml = New-Object System.Text.StringBuilder
Add-WixDirectory $script:RootFull 3

$refs = ($script:ComponentIds | ForEach-Object { "      <ComponentRef Id=`"$_`" />" }) -join "`r`n"
$fragment = @"
<?xml version="1.0" encoding="utf-8"?>
<!-- GENERISANO iz build.ps1 - ne menjati rucno -->
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
  <Fragment>
    <DirectoryRef Id="FrontendFolder">
$($script:Xml.ToString().TrimEnd())
    </DirectoryRef>
    <ComponentGroup Id="AngularFrontend">
$refs
    </ComponentGroup>
  </Fragment>
</Wix>
"@
[IO.File]::WriteAllText((Join-Path $PSScriptRoot $WixFragment), $fragment, (New-Object Text.UTF8Encoding $false))
Write-Host "      $($script:ComponentIds.Count) fajlova upisano u $WixFragment" -ForegroundColor Gray

Write-Host "[3/4] Kompajliram Rust server (release)..." -ForegroundColor Cyan
cargo build --release -p backend
Assert-Exit $LASTEXITCODE "Rust build"

Write-Host "[4/4] Kopiram sertifikate i frontend u $ReleaseDir (za prod.ps1)..." -ForegroundColor Cyan
Copy-Item "backend\cert.pem", "backend\key.pem" $ReleaseDir -Force -ErrorAction Stop
$releaseFront = Join-Path $ReleaseDir "frontend"
if (Test-Path $releaseFront) { Remove-Item $releaseFront -Recurse -Force -ErrorAction Stop }
Copy-Item $StagedFront $releaseFront -Recurse -ErrorAction Stop

Write-Host "USPESNO! Server: $ReleaseDir\ogilenko-server.exe" -ForegroundColor Green
