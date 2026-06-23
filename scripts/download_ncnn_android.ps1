# Download and extract prebuilt ncnn static libraries for Android ABIs.
param(
    [string]$Version = "20260526",
    [ValidateSet("arm64-v8a", "x86_64", "all")]
    [string]$Abi = "all"
)
$ErrorActionPreference = "Stop"

$Root = Split-Path $PSScriptRoot -Parent
$NcnnRoot = Join-Path $Root "third_party\ncnn\android"
$Url = "https://github.com/Tencent/ncnn/releases/download/$Version/ncnn-$Version-android.zip"

$Abis = if ($Abi -eq "all") { @("arm64-v8a", "x86_64") } else { @($Abi) }

function Test-NcnnAbiReady {
    param([string]$Name)
    $lib = Join-Path $NcnnRoot "$Name\lib\libncnn.a"
    return Test-Path $lib
}

$missing = @($Abis | Where-Object { -not (Test-NcnnAbiReady $_) })
if ($missing.Count -eq 0) {
    Write-Host "ncnn Android libraries already present for: $($Abis -join ', ')"
    return
}

$zipPath = Join-Path $env:TEMP "ncnn-$Version-android.zip"
$extractRoot = Join-Path $env:TEMP "ncnn-$Version-android-extract"

Write-Host "Downloading ncnn $Version for Android..."
Invoke-WebRequest -Uri $Url -OutFile $zipPath

if (Test-Path $extractRoot) {
    Remove-Item -Recurse -Force $extractRoot
}
Expand-Archive -Path $zipPath -DestinationPath $extractRoot -Force

$sourceRoot = Join-Path $extractRoot "ncnn-$Version-android"
if (-not (Test-Path $sourceRoot)) {
    throw "Unexpected ncnn archive layout (missing $sourceRoot)"
}

New-Item -ItemType Directory -Force -Path $NcnnRoot | Out-Null

foreach ($name in $missing) {
    $src = Join-Path $sourceRoot $name
    $dest = Join-Path $NcnnRoot $name
    if (-not (Test-Path (Join-Path $src "lib\libncnn.a"))) {
        throw "ncnn archive missing ABI: $name"
    }
    if (Test-Path $dest) {
        Remove-Item -Recurse -Force $dest
    }
    Copy-Item -Recurse $src $dest
    Write-Host "Installed: third_party/ncnn/android/$name/"
}

Remove-Item -Force $zipPath -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force $extractRoot -ErrorAction SilentlyContinue
