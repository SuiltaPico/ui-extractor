# Build Android release .so (arm64-v8a + x86_64) and zip packages for GitHub Releases.
param(
    [string]$OutDir = "dist",
    [switch]$SkipPack,
    [switch]$SkipDownload,
    [switch]$SkipAssets,
    [string]$DistDir = "",
    [string]$ReleaseRepo = "",
    [string]$ReleaseTag = "",
    [string]$InferCoreReleaseDir = ""
)
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "cargo_retry.ps1")
. (Join-Path $PSScriptRoot "infer_core_release.ps1")

$Root = Split-Path $PSScriptRoot -Parent
Push-Location $Root
try {
    if ($SkipAssets) {
        Write-Host "SkipAssets is deprecated: release zips no longer bundle models."
    }

    $versionLine = Select-String -Path "Cargo.toml" -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if (-not $versionLine) { throw "Could not read version from Cargo.toml" }
    $Version = $versionLine.Matches[0].Groups[1].Value

    if (-not $SkipDownload) {
        & (Join-Path $PSScriptRoot "download_infer_core_release.ps1") -Platform android -ReleaseRepo $ReleaseRepo -ReleaseTag $ReleaseTag -OutDir $InferCoreReleaseDir
        if ($LASTEXITCODE -gt 0) { exit $LASTEXITCODE }
    }

    $buildArgs = @{
        Abi                  = "all"
        ReleaseRepo          = $ReleaseRepo
        ReleaseTag           = $ReleaseTag
        InferCoreReleaseDir  = $InferCoreReleaseDir
    }
    & (Join-Path $PSScriptRoot "build_android.ps1") @buildArgs
    if ($LASTEXITCODE -gt 0) { exit $LASTEXITCODE }

    if ($SkipPack) {
        Write-Host "SkipPack set; .so files left under android/jniLibs/"
        return
    }

    $packRoot = Join-Path $Root $OutDir
    New-Item -ItemType Directory -Force -Path $packRoot | Out-Null

    $header = Join-Path $Root "include\ui_extractor.h"
    if (-not (Test-Path $header)) { throw "Missing C header: include/ui_extractor.h" }

    $readme = @"
ui-extractor $Version (Android MNN via infer-core)

Contents:
  jniLibs/<abi>/*.so     - libui_extractor.so + MNN runtime
  include/ui_extractor.h - C ABI
  (model packs are NOT bundled)

Integrate:
  1. Copy jniLibs/<abi>/*.so into app/src/main/jniLibs/<abi>/
  2. System.loadLibrary("ui_extractor") (and libc++_shared if needed)
  3. Download model packs from local-infer-core Releases (same tag), then place them
     under your app models dir and pass modelsDir in config JSON.
     pack ids: ocr.paddle.ppocr6-*.mnn.fp32, embed.mobileclip2-s0.mnn.{fp32,int8},
               icons.bundled.v1.mobileclip2-s0.int8

See docs/android.md for JNI config JSON.
"@
    $abis = @(
        @{ Name = "arm64-v8a"; Label = "android-arm64-v8a" },
        @{ Name = "x86_64"; Label = "android-x86_64" }
    )

    foreach ($abi in $abis) {
        $jniSrc = Join-Path $Root "android\jniLibs\$($abi.Name)"
        $so = Join-Path $jniSrc "libui_extractor.so"
        if (-not (Test-Path $so)) { throw "Missing build output: $so" }

        $stage = Join-Path (Get-ScratchDir) "ui-extractor-$($abi.Label)"
        if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
        $includeDir = Join-Path $stage "include"
        $stageJni = Join-Path $stage "jniLibs\$($abi.Name)"
        New-Item -ItemType Directory -Force -Path $includeDir | Out-Null
        New-Item -ItemType Directory -Force -Path $stageJni | Out-Null

        Get-ChildItem $jniSrc -Filter "*.so" | ForEach-Object {
            Copy-Item $_.FullName (Join-Path $stageJni $_.Name) -Force
        }
        Copy-Item $header (Join-Path $includeDir "ui_extractor.h")

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
