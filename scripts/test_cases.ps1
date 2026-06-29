# Run golden case regression (infer-core native lib + model packs from GitHub Releases).
param(
    [string]$ModelsDir = "models",
    [string]$CasesDir = "tests/cases",
    [switch]$LayoutOnly,
    [switch]$NoIcon
)
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "cargo_retry.ps1")
. (Join-Path $PSScriptRoot "infer_core_release.ps1")

$Root = Split-Path $PSScriptRoot -Parent
Push-Location $Root
try {
    & (Join-Path $PSScriptRoot "download_infer_core_release.ps1") -Platform windows
    if ($LASTEXITCODE -gt 0) { exit $LASTEXITCODE }

    & (Join-Path $PSScriptRoot "install_packs.ps1") -Platform windows -Source release -ModelsDir $ModelsDir
    if ($LASTEXITCODE -gt 0) { exit $LASTEXITCODE }

    Write-Host "Building ui-extractor (release)..."
    Invoke-CargoWithRetry @('build', '--release', '--bin', 'ui-extractor')
    if ($LASTEXITCODE -gt 0) { exit $LASTEXITCODE }

    $hostTriple = "x86_64-pc-windows-msvc"
    $cargoOut = Join-Path $Root "target\release"
    Copy-InferCoreRuntimeDll -Triple $hostTriple -CargoOutDir $cargoOut

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
