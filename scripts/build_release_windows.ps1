# Build Windows release binaries (x64 + arm64) and zip packages for GitHub Releases.
param(
    [string]$OutDir = "dist",
    [switch]$SkipPack,
    [switch]$SkipAssets
)
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "cargo_retry.ps1")

$Root = Split-Path $PSScriptRoot -Parent
Push-Location $Root
try {
    if (-not $SkipAssets) {
        Write-Host "Building host ui-extractor (release)..."
        Invoke-CargoWithRetry build --release --bin ui-extractor
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

        & (Join-Path $PSScriptRoot "prepare_release_assets.ps1") -Backend ort
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }

    $versionLine = Select-String -Path "Cargo.toml" -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if (-not $versionLine) { throw "Could not read version from Cargo.toml" }
    $Version = $versionLine.Matches[0].Groups[1].Value

    $targets = @(
        @{ Triple = "x86_64-pc-windows-msvc"; Label = "windows-x64" },
        @{ Triple = "aarch64-pc-windows-msvc"; Label = "windows-arm64" }
    )

    $built = @()
    foreach ($t in $targets) {
        Write-Host "Building $($t.Label) ($($t.Triple))..."
        $prevEap = $ErrorActionPreference
        $ErrorActionPreference = 'SilentlyContinue'
        rustup target add $t.Triple 2>&1 | Out-Null
        $ErrorActionPreference = $prevEap
        if ($LASTEXITCODE -ne 0) { throw "rustup target add failed: $($t.Triple)" }
        Invoke-CargoWithRetry build --release --target $t.Triple
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

        $releaseDir = Join-Path $Root "target\$($t.Triple)\release"
        $exe = Join-Path $releaseDir "ui-extractor.exe"
        $dll = Join-Path $releaseDir "ui_extractor.dll"
        $importLib = Join-Path $releaseDir "ui_extractor.dll.lib"
        if (-not (Test-Path $exe)) { throw "Build output not found: $exe" }
        if (-not (Test-Path $dll)) { throw "Build output not found: $dll" }
        $built += @{
            Label     = $t.Label
            Exe       = $exe
            Dll       = $dll
            ImportLib = $importLib
        }
    }

    if ($SkipPack) {
        Write-Host "SkipPack set; binaries left under target/<triple>/release/"
        return
    }

    $packRoot = Join-Path $Root $OutDir
    New-Item -ItemType Directory -Force -Path $packRoot | Out-Null

    $header = Join-Path $Root "include\ui_extractor.h"
    if (-not (Test-Path $header)) { throw "Missing C header: include/ui_extractor.h" }

    $targetLabels = ($targets | ForEach-Object { $_.Label }) -join ", "
    $readme = @"
ui-extractor $Version ($targetLabels)

Desktop release using ONNX Runtime (backend-ort).

Contents:
  ui-extractor.exe        - CLI
  ui_extractor.dll        - native library (C ABI)
  ui_extractor.dll.lib    - MSVC import library (link against the DLL)
  include/ui_extractor.h  - C ABI header
  models/                 - OCR + MobileCLIP2 ONNX weights
  assets/embeddings.bin   - MDI icon embedding index (~7400 icons)

CLI:
  ui-extractor extract --input screenshot.png --annotate

SDK integrate:
  1. Link ui_extractor.dll.lib and ship ui_extractor.dll next to your executable
  2. #include "ui_extractor.h"
  3. Copy models/ and assets/embeddings.bin (or set paths in config JSON)

Run the CLI from the extracted directory so default model paths resolve.
"@

    $modelFiles = @(
        "pp-ocrv5_mobile_det.onnx",
        "pp-ocrv5_mobile_rec.onnx",
        "ppocrv5_dict.txt",
        "mobileclip2-s0-vision.onnx"
    )

    foreach ($item in $built) {
        $stage = Join-Path (Get-ScratchDir) "ui-extractor-$($item.Label)"
        if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
        $includeDir = Join-Path $stage "include"
        New-Item -ItemType Directory -Force -Path $includeDir | Out-Null

        Copy-Item $item.Exe (Join-Path $stage "ui-extractor.exe")
        Copy-Item $item.Dll $stage
        if (Test-Path $item.ImportLib) {
            Copy-Item $item.ImportLib $stage
        }
        Copy-Item $header (Join-Path $includeDir "ui_extractor.h")

        if (-not $SkipAssets) {
            $modelsStage = Join-Path $stage "models"
            New-Item -ItemType Directory -Force -Path $modelsStage | Out-Null
            foreach ($name in $modelFiles) {
                Copy-Item (Join-Path $Root "models\$name") (Join-Path $modelsStage $name)
            }
            $assetsStage = Join-Path $stage "assets"
            New-Item -ItemType Directory -Force -Path $assetsStage | Out-Null
            Copy-Item (Join-Path $Root "assets\embeddings.bin") (Join-Path $assetsStage "embeddings.bin")
        }

        Set-Content -Path (Join-Path $stage "README.txt") -Value $readme -Encoding UTF8

        $zipName = "ui-extractor-$($item.Label).zip"
        $zipPath = Join-Path $packRoot $zipName
        if (Test-Path $zipPath) { Remove-Item -Force $zipPath }
        Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zipPath
        Remove-Item -Recurse -Force $stage
        Write-Host "Packaged: $OutDir/$zipName"
    }
} finally {
    Pop-Location
}
