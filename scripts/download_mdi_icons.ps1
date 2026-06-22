# Download Material Design Icons (@mdi/svg) for icon similarity matching.
# SVG is the official distribution; PNG is generated locally via `rasterize-mdi` (Rust + resvg).
param(
    [string]$Version = "7.4.47",
    [string]$OutDir = (Join-Path $PSScriptRoot "..\assets\mdi"),
    [switch]$Rasterize,
    [int]$Size = 48,
    [ValidateSet("black", "white")]
    [string]$Color = "black",
    [int]$Jobs = 0
)

$ErrorActionPreference = "Stop"
$WorkDir = Join-Path $env:TEMP "ui-extractor-mdi-$Version"
$SvgSrc = Join-Path $WorkDir "node_modules\@mdi\svg\svg"
$MetaSrc = Join-Path $WorkDir "node_modules\@mdi\svg\meta.json"
$SvgDest = Join-Path $OutDir "svg"
$MetaDest = Join-Path $OutDir "meta.json"
$PngDest = Join-Path $OutDir "png-$Size-$Color"

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
    $RasterBin = Join-Path $RepoRoot "target\release\rasterize-mdi.exe"
    if (-not (Test-Path $RasterBin)) {
        Write-Host "building rasterize-mdi (release) ..."
        Push-Location $RepoRoot
        try {
            cargo build --release --bin rasterize-mdi
        } finally {
            Pop-Location
        }
    }
    if (-not (Test-Path $RasterBin)) {
        throw "missing rasterizer: $RasterBin"
    }

    $RasterArgs = @(
        "--svg-dir", $SvgDest,
        "--out-dir", $PngDest,
        "--size", $Size,
        "--color", $Color
    )
    if ($Jobs -gt 0) {
        $RasterArgs += @("--jobs", $Jobs)
    }

    Write-Host "rasterizing to $PngDest ($Size px, $Color) ..."
    & $RasterBin @RasterArgs
    if ($LASTEXITCODE -ne 0) {
        throw "rasterize-mdi failed with exit code $LASTEXITCODE"
    }
    $pngCount = (Get-ChildItem $PngDest -Filter *.png -ErrorAction SilentlyContinue).Count
    Write-Host "png ready: $pngCount files in $PngDest"
}
