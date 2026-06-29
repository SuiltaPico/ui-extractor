# Build Windows release binaries (x86_64 + aarch64) and zip packages for GitHub Releases.
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
        & (Join-Path $PSScriptRoot "download_infer_core_release.ps1") -Platform windows -ReleaseRepo $ReleaseRepo -ReleaseTag $ReleaseTag -OutDir $InferCoreReleaseDir
        if ($LASTEXITCODE -gt 0) { exit $LASTEXITCODE }
    }

    $targets = @(
        @{ Triple = "x86_64-pc-windows-msvc"; Label = "windows-x86_64" },
        @{ Triple = "aarch64-pc-windows-msvc"; Label = "windows-aarch64" }
    )

    $built = @()
    foreach ($t in $targets) {
        Write-Host "Building $($t.Label) ($($t.Triple))..."
        $prevEap = $ErrorActionPreference
        $ErrorActionPreference = 'SilentlyContinue'
        rustup target add $t.Triple 2>&1 | Out-Null
        $ErrorActionPreference = $prevEap
        if ($LASTEXITCODE -gt 0) { throw "rustup target add failed: $($t.Triple)" }

        $inferLibDir = Resolve-InferCoreWindowsLibDir `
            -Triple $t.Triple `
            -ReleaseRoot $InferCoreReleaseDir `
            -Repo $ReleaseRepo `
            -Tag $ReleaseTag
        Write-Host "Using infer-core release lib dir: $inferLibDir"

        Invoke-CargoWithRetry @('build', '--release', '--target', $t.Triple)
        if ($LASTEXITCODE -gt 0) { exit $LASTEXITCODE }

        $releaseDir = Join-Path $Root "target\$($t.Triple)\release"
        $inferDll = Join-Path $inferLibDir "infer_core.dll"
        if (Test-Path $inferDll) {
            Copy-Item $inferDll $releaseDir -Force
        }
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

    $sdkReadme = @"
ui-extractor $Version (SDK / Dart hook)

Desktop native library aligned with local-infer-core release layout.

Contents:
  lib/ui_extractor.dll       - native library (C ABI)
  lib/ui_extractor.dll.lib   - MSVC import library
  include/ui_extractor.h     - C ABI header

SDK integrate:
  1. Link lib/ui_extractor.dll.lib and ship lib/ui_extractor.dll next to your executable
  2. #include "ui_extractor.h"
  3. Ship infer_core.dll from local-infer-core Release (same tag)
  4. Download model packs from local-infer-core Releases and extract to <models_dir>
     pack ids: ocr.paddle.ppocr6-*.onnx.fp32, embed.mobileclip2-s0.onnx.fp32,
               icons.bundled.v1.mobileclip2-s0.int8
"@

    $bundleReadme = @"
ui-extractor $Version (CLI bundle)

Desktop CLI bundle with infer-core runtime DLL.

Contents:
  ui-extractor.exe        - CLI (layout + pipeline; loads infer_core.dll at runtime)
  ui_extractor.dll        - native library (C ABI)
  infer_core.dll          - local-infer-core inference (OCR + embed)
  ui_extractor.dll.lib    - MSVC import library
  include/ui_extractor.h  - C ABI header
  (model packs are NOT bundled)

CLI:
  ui-extractor extract --input screenshot.png --annotate --models-dir <models_dir>

Run with explicit --models-dir (or set LOCAL_INFER_ROOT).
"@

    foreach ($item in $built) {
        $sdkStage = Join-Path (Get-ScratchDir) "ui-extractor-$($item.Label)-sdk"
        if (Test-Path $sdkStage) { Remove-Item -Recurse -Force $sdkStage }
        $sdkLibDir = Join-Path $sdkStage "lib"
        $sdkIncludeDir = Join-Path $sdkStage "include"
        New-Item -ItemType Directory -Force -Path $sdkLibDir | Out-Null
        New-Item -ItemType Directory -Force -Path $sdkIncludeDir | Out-Null

        Copy-Item $item.Dll (Join-Path $sdkLibDir "ui_extractor.dll")
        if (Test-Path $item.ImportLib) {
            Copy-Item $item.ImportLib (Join-Path $sdkLibDir "ui_extractor.dll.lib")
        }
        Copy-Item $header (Join-Path $sdkIncludeDir "ui_extractor.h")
        Set-Content -Path (Join-Path $sdkStage "README.txt") -Value $sdkReadme -Encoding UTF8

        $sdkZipName = "ui-extractor-$($item.Label).zip"
        $sdkZipPath = Join-Path $packRoot $sdkZipName
        if (Test-Path $sdkZipPath) { Remove-Item -Force $sdkZipPath }
        Compress-Archive -Path (Join-Path $sdkStage "*") -DestinationPath $sdkZipPath
        Remove-Item -Recurse -Force $sdkStage
        Write-Host "Packaged: $OutDir/$sdkZipName"

        $bundleStage = Join-Path (Get-ScratchDir) "ui-extractor-$($item.Label)-bundle"
        if (Test-Path $bundleStage) { Remove-Item -Recurse -Force $bundleStage }
        $bundleIncludeDir = Join-Path $bundleStage "include"
        New-Item -ItemType Directory -Force -Path $bundleIncludeDir | Out-Null

        Copy-Item $item.Exe (Join-Path $bundleStage "ui-extractor.exe")
        Copy-Item $item.Dll $bundleStage
        $inferCoreDll = Join-Path (Split-Path $item.Exe -Parent) "infer_core.dll"
        if (Test-Path $inferCoreDll) {
            Copy-Item $inferCoreDll $bundleStage
        }
        if (Test-Path $item.ImportLib) {
            Copy-Item $item.ImportLib $bundleStage
        }
        Copy-Item $header (Join-Path $bundleIncludeDir "ui_extractor.h")
        Set-Content -Path (Join-Path $bundleStage "README.txt") -Value $bundleReadme -Encoding UTF8

        $bundleZipName = "ui-extractor-$($item.Label)-bundle.zip"
        $bundleZipPath = Join-Path $packRoot $bundleZipName
        if (Test-Path $bundleZipPath) { Remove-Item -Force $bundleZipPath }
        Compress-Archive -Path (Join-Path $bundleStage "*") -DestinationPath $bundleZipPath
        Remove-Item -Recurse -Force $bundleStage
        Write-Host "Packaged: $OutDir/$bundleZipName"
    }
} finally {
    Pop-Location
}
