# Download Material Design Icons (@mdi/svg) for icon similarity matching.
# SVG is the official distribution; PNG is generated locally via `ui-extractor icon rasterize-svg`.
param(
    [string]$Version = "7.4.47",
    [string]$OutDir = (Join-Path $PSScriptRoot "..\assets"),
    [switch]$Rasterize,
    [int]$Size = 48,
    [ValidateSet("black", "white")]
    [string]$Color = "black",
    [int]$Jobs = 0
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "cargo_retry.ps1")
$WorkDir = Join-Path (Get-ScratchDir) "ui-extractor-mdi-$Version"
$SvgSrc = Join-Path $WorkDir "node_modules\@mdi\svg\svg"
$MetaSrc = Join-Path $WorkDir "node_modules\@mdi\svg\meta.json"
$SvgDest = Join-Path $OutDir "svg"
$MetaDest = Join-Path $OutDir "meta.json"
$PngDest = Join-Path $OutDir "icons"

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

if (-not (Test-Path $SvgSrc)) {
    Write-Host "installing @mdi/svg@$Version ..."
    New-Item -ItemType Directory -Force -Path $WorkDir | Out-Null
    Push-Location $WorkDir
    try {
        if (-not (Test-Path "package.json")) {
            npm init -y | Out-Null
        }
        npm install "@mdi/svg@$Version" --no-save --silent
    } finally {
        Pop-Location
    }
}

if (-not (Test-Path $SvgSrc)) {
    throw "SVG source not found: $SvgSrc"
}

Write-Host "copying SVG files -> $SvgDest"
if (Test-Path $SvgDest) {
    Remove-Item -Recurse -Force $SvgDest
}
Copy-Item -Recurse $SvgSrc $SvgDest
Copy-Item -Force $MetaSrc $MetaDest

$count = (Get-ChildItem $SvgDest -Filter *.svg).Count
Write-Host "mdi ready: $count icons in $OutDir"
Write-Host "  svg:  $SvgDest"
Write-Host "  meta: $MetaDest"

if ($Rasterize) {
    $RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
    $exeName = if ($IsWindows -or ($env:OS -match "Windows")) { "ui-extractor.exe" } else { "ui-extractor" }
    $CliBin = Join-Path $RepoRoot (Join-Path "target" (Join-Path "release" $exeName))
    if (-not (Test-Path $CliBin)) {
        Write-Host "building ui-extractor (release) ..."
        Push-Location $RepoRoot
        try {
            Invoke-CargoWithRetry build --release --bin ui-extractor
        } finally {
            Pop-Location
        }
    }
    if (-not (Test-Path $CliBin)) {
        throw "missing ui-extractor: $CliBin"
    }

    $RasterArgs = @(
        "icon", "rasterize-svg",
        "--svg-dir", $SvgDest,
        "--out-dir", $PngDest,
        "--size", $Size,
        "--color", $Color
    )
    if ($Jobs -gt 0) {
        $RasterArgs += @("--jobs", $Jobs)
    }

    Write-Host "rasterizing to $PngDest ($Size px, $Color) ..."
    & $CliBin @RasterArgs
    if ($LASTEXITCODE -ne 0) {
        throw "ui-extractor icon rasterize-svg failed with exit code $LASTEXITCODE"
    }
    $pngCount = (Get-ChildItem $PngDest -Filter *.png -ErrorAction SilentlyContinue).Count
    Write-Host "png ready: $pngCount files in $PngDest"
}
