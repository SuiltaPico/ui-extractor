# Run golden case regression after installing local-infer-core packs.
param(
    [string]$ModelsDir = "models",
    [string]$DistDir = "",
    [string]$CasesDir = "tests/cases",
    [switch]$LayoutOnly,
    [switch]$NoIcon
)
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "cargo_retry.ps1")

$Root = Split-Path $PSScriptRoot -Parent
$InferCoreRoot = Join-Path (Split-Path $Root -Parent) "local-infer-core"
Push-Location $Root
try {
    $installArgs = @{ Platform = "windows"; Source = "local"; DistDir = $DistDir }
    if ($DistDir) { $installArgs.DistDir = $DistDir } else { $installArgs.DistDir = (Join-Path $InferCoreRoot "dist") }
    & (Join-Path $PSScriptRoot "install_packs.ps1") @installArgs

    Write-Host "Building infer-core-ffi (release)..."
    Push-Location $InferCoreRoot
    try {
        Invoke-CargoWithRetry @('build', '-p', 'infer-core-ffi', '--release', '--features', 'backend-ort')
    } finally {
        Pop-Location
    }

    Write-Host "Building ui-extractor (release)..."
    Invoke-CargoWithRetry @('build', '--release', '--bin', 'ui-extractor')

    $inferDll = Join-Path $InferCoreRoot "target\release\infer_core.dll"
    $hostRelease = Join-Path $Root "target\release"
    if (Test-Path $inferDll) {
        Copy-Item $inferDll $hostRelease -Force
        Write-Host "Copied infer_core.dll -> target/release/"
    } else {
        throw "Missing infer_core.dll at $inferDll (build infer-core-ffi first)"
    }

    $caseArgs = @(
        "run", "--release", "--",
        "cases",
        "--dir", $CasesDir,
        "--models-dir", $ModelsDir
    )
    if ($LayoutOnly) { $caseArgs += "--layout-only" }
    if ($NoIcon) { $caseArgs += "--no-icon" }

    Write-Host "Running golden cases..."
    Invoke-CargoWithRetry @caseArgs

    Write-Host "Golden case run completed."
} finally {
    Pop-Location
}
