# Build Android release .so (arm64-v8a + x86_64) and zip packages for GitHub Releases.
param(
    [string]$OutDir = "dist",
    [switch]$SkipPack,
    [switch]$SkipDownload,
    [switch]$SkipAssets
)
$ErrorActionPreference = "Stop"

$Root = Split-Path $PSScriptRoot -Parent
Push-Location $Root
try {
    if (-not $SkipAssets) {
        $prepareArgs = @{ Backend = "ncnn" }
        if ($SkipDownload) { $prepareArgs.SkipDownload = $true }
        & (Join-Path $PSScriptRoot "prepare_release_assets.ps1") @prepareArgs
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }

    $versionLine = Select-String -Path "Cargo.toml" -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if (-not $versionLine) { throw "Could not read version from Cargo.toml" }
    $Version = $versionLine.Matches[0].Groups[1].Value

    $buildArgs = @{ Abi = "all" }
    if (-not $SkipDownload) { $buildArgs.DownloadNcnn = $true }
    & (Join-Path $PSScriptRoot "build_android.ps1") @buildArgs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    if ($SkipPack) {
        Write-Host "SkipPack set; .so files left under android/jniLibs/"
        return
    }

    $packRoot = Join-Path $Root $OutDir
    New-Item -ItemType Directory -Force -Path $packRoot | Out-Null

    $header = Join-Path $Root "include\ui_extractor.h"
    if (-not (Test-Path $header)) { throw "Missing C header: include/ui_extractor.h" }

    $readme = @"
ui-extractor $Version (Android ncnn backend)

Contents:
  libui_extractor.so     - native library (backend-ncnn)
  include/ui_extractor.h - C ABI
  models/                - OCR + MobileCLIP2 ncnn weights + dict
  assets/embeddings.bin  - MDI icon embedding index (~7400 icons)

Integrate:
  1. Copy libui_extractor.so to app/src/main/jniLibs/<abi>/
  2. System.loadLibrary("ui_extractor") and System.loadLibrary("c++_shared")
  3. Copy models/ and assets/embeddings.bin into app assets (see docs/android.md)

See docs/android.md for JNI config JSON.
"@

    $modelFiles = @(
        "pp-ocrv5_mobile_det.ncnn.param",
        "pp-ocrv5_mobile_det.ncnn.bin",
        "pp-ocrv5_mobile_rec.ncnn.param",
        "pp-ocrv5_mobile_rec.ncnn.bin",
        "mobileclip2-s0-vision.ncnn.param",
        "mobileclip2-s0-vision.ncnn.bin",
        "ppocrv5_dict.txt"
    )

    $abis = @(
        @{ Name = "arm64-v8a"; Label = "android-arm64-v8a" },
        @{ Name = "x86_64"; Label = "android-x86_64" }
    )

    foreach ($abi in $abis) {
        $so = Join-Path $Root "android\jniLibs\$($abi.Name)\libui_extractor.so"
        if (-not (Test-Path $so)) { throw "Missing build output: $so" }

        $stage = Join-Path $env:TEMP "ui-extractor-$($abi.Label)"
        if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
        $includeDir = Join-Path $stage "include"
        New-Item -ItemType Directory -Force -Path $includeDir | Out-Null

        Copy-Item $so $stage
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

        $zipName = "ui-extractor-$($abi.Label).zip"
        $zipPath = Join-Path $packRoot $zipName
        if (Test-Path $zipPath) { Remove-Item -Force $zipPath }
        Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zipPath
        Remove-Item -Recurse -Force $stage
        Write-Host "Packaged: $OutDir/$zipName"
    }
} finally {
    Pop-Location
}
