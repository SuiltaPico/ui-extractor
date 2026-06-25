# Install manifest-driven model packs from local-infer-core releases or local overrides.
param(
    [ValidateSet("windows", "android")]
    [string]$Platform = "windows",

    [string]$ModelsDir = "models",

    [string]$DistDir = "",

    [ValidateSet("auto", "release", "local")]
    [string]$Source = "",

    [switch]$Force
)
$ErrorActionPreference = "Stop"

$installArgs = @{
    Platform  = $Platform
    ModelsDir = $ModelsDir
}
if ($DistDir) {
    $installArgs.DistDir = $DistDir
    if (-not $Source) { $installArgs.Source = "local" }
}
if ($Source) { $installArgs.Source = $Source }
if (-not $installArgs.Source) { $installArgs.Source = "auto" }
if ($Force) { $installArgs.Force = $true }
& (Join-Path $PSScriptRoot "install_packs.ps1") @installArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Release assets ready ($Platform)."
