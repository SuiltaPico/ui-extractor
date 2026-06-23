# Download all icon libraries (MDI + Fluent + Tabler + Font Awesome) and optionally rasterize PNG templates.
param(
    [string]$OutDir = (Join-Path $PSScriptRoot "..\assets"),
    [switch]$Rasterize,
    [int]$Size = 48,
    [ValidateSet("black", "white")]
    [string]$Color = "black",
    [int]$Jobs = 0,
    [string]$MdiVersion = "7.4.47",
    [string]$FluentVersion = "1.1.313",
    [string]$TablerVersion = "3.36.0",
    [string]$FaVersion = "6.7.2"
)

$ErrorActionPreference = "Stop"

$MdiArgs = @{
    OutDir = $OutDir
    Version = $MdiVersion
}
if ($Rasterize) { $MdiArgs.Rasterize = $true }
if ($Size -gt 0) { $MdiArgs.Size = $Size }
if ($Color) { $MdiArgs.Color = $Color }
if ($Jobs -gt 0) { $MdiArgs.Jobs = $Jobs }

Write-Host "=== MDI ==="
& (Join-Path $PSScriptRoot "download_mdi_icons.ps1") @MdiArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$LibArgs = @{
    OutDir = $OutDir
    FluentVersion = $FluentVersion
    TablerVersion = $TablerVersion
    FaVersion = $FaVersion
}
if ($Rasterize) { $LibArgs.Rasterize = $true }
if ($Size -gt 0) { $LibArgs.Size = $Size }
if ($Color) { $LibArgs.Color = $Color }
if ($Jobs -gt 0) { $LibArgs.Jobs = $Jobs }

Write-Host "=== Fluent / Tabler / Font Awesome ==="
& (Join-Path $PSScriptRoot "download_icon_libraries.ps1") @LibArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "all icon libraries downloaded under $OutDir/svg and $OutDir/icons"
