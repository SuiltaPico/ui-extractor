# Build Windows release binaries (x64 + arm64) and zip packages for GitHub Releases.
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
        if ($LASTEXITCODE -gt 0) { throw "rustup target add failed: $($t.Triple)" }

        $inferLibDir = Resolve-InferCoreWindowsLibDir `
            -Triple $t.Triple `
            -ReleaseRoot $InferCoreReleaseDir `
            -Repo $ReleaseRepo `
            -Tag $ReleaseTag
        Write-Host "Using infer-core release lib dir: $inferLibDir"
        $env:INFER_CORE_LIB_DIR = $inferLibDir

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

    $targetLabels = ($targets | ForEach-Object { $_.Label }) -join ", "
    $readme = @"
ui-extractor $Version ($targetLabels)

Desktop release using infer-core (ONNX Runtime / backend-ort).

Contents:
  ui-extractor.exe        - CLI (layout + pipeline; loads infer_core.dll at runtime)
  ui_extractor.dll        - native library (C ABI)
  infer_core.dll          - local-infer-core inference (OCR + embed)
  ui_extractor.dll.lib    - MSVC import library
  include/ui_extractor.h  - C ABI header
  (model packs are NOT bundled)

CLI:
  ui-extractor extract --input screenshot.png --annotate --models-dir <models_dir>

SDK integrate:
  1. Link ui_extractor.dll.lib and ship ui_extractor.dll next to your executable
  2. #include "ui_extractor.h"
  3. Download model packs from local-infer-core Releases (same tag) and extract to <models_dir>
     pack ids: ocr.paddle.ppocr6-*.onnx.fp32, embed.mobileclip2-s0.onnx.fp32,
               icons.bundled.v1.mobileclip2-s0.int8

Run the CLI with explicit --models-dir (or set LOCAL_INFER_ROOT).
"@
    foreach ($item in $built) {
        $stage = Join-Path (Get-ScratchDir) "ui-extractor-$($item.Label)"
        if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
        $includeDir = Join-Path $stage "include"
        New-Item -ItemType Directory -Force -Path $includeDir | Out-Null

        Copy-Item $item.Exe (Join-Path $stage "ui-extractor.exe")
        Copy-Item $item.Dll $stage
        $inferCoreDll = Join-Path (Split-Path $item.Exe -Parent) "infer_core.dll"
        if (Test-Path $inferCoreDll) {
            Copy-Item $inferCoreDll $stage
        }
        if (Test-Path $item.ImportLib) {
            Copy-Item $item.ImportLib $stage
        }
        Copy-Item $header (Join-Path $includeDir "ui_extractor.h")

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
