# Download infer-core native library zips from local-infer-core GitHub Releases.
param(
    [ValidateSet("windows", "android", "all")]
    [string]$Platform = "all",

    [string]$ReleaseRepo = "",
    [string]$ReleaseTag = "",

    [string]$OutDir = "",

    [switch]$Force
)
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "infer_core_release.ps1")

$assets = @()
if ($Platform -in @("windows", "all")) {
    $assets += $script:WindowsReleaseAssets.Values
}
if ($Platform -in @("android", "all")) {
    $assets += $script:AndroidReleaseAssets.Values
}

$root = Get-InferCoreReleaseRoot -OutDir $OutDir
New-Item -ItemType Directory -Force -Path $root | Out-Null

$repo = Get-InferCoreReleaseRepo -Repo $ReleaseRepo
$tag = Get-InferCoreReleaseTag -Tag $ReleaseTag
Write-Host "infer-core release: $repo @ $tag -> $root"

foreach ($asset in $assets) {
    $dir = Ensure-InferCoreReleaseAsset -AssetBaseName $asset -ReleaseRoot $root -Repo $repo -Tag $tag -Force:$Force
    Write-Host "Ready: $asset -> $dir"
}
