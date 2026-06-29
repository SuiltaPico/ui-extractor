# Local dev build: download infer-core from GitHub Release, then cargo build.
param(
    [ValidateSet("debug", "release")]
    [string]$Profile = "debug",

    [string]$ReleaseRepo = "",
    [string]$ReleaseTag = "",

    [switch]$SkipDownload,
    [switch]$SkipCopyRuntime
)
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "cargo_retry.ps1")
. (Join-Path $PSScriptRoot "infer_core_release.ps1")

$Root = Split-Path $PSScriptRoot -Parent
Push-Location $Root
try {
    if (-not $SkipDownload) {
        & (Join-Path $PSScriptRoot "download_infer_core_release.ps1") -Platform windows -ReleaseRepo $ReleaseRepo -ReleaseTag $ReleaseTag
        if ($LASTEXITCODE -gt 0) { exit $LASTEXITCODE }
    }

    $cargoArgs = @('build', '--bin', 'ui-extractor')
    if ($Profile -eq "release") { $cargoArgs += '--release' }
    Write-Host "cargo $($cargoArgs -join ' ')"
    Invoke-CargoWithRetry @cargoArgs
    if ($LASTEXITCODE -gt 0) { exit $LASTEXITCODE }

    if (-not $SkipCopyRuntime) {
        $hostTriple = "x86_64-pc-windows-msvc"
        $cargoOut = if ($Profile -eq "release") {
            Join-Path $Root "target\release"
        } else {
            Join-Path $Root "target\debug"
        }
        Copy-InferCoreRuntimeDll -Triple $hostTriple -CargoOutDir $cargoOut -ReleaseRepo $ReleaseRepo -ReleaseTag $ReleaseTag
    }
} finally {
    Pop-Location
}
