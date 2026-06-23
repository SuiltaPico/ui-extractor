# Build ui-extractor cdylib for Android (arm64-v8a / x86_64) with ncnn backend.
param(
    [ValidateSet("arm64-v8a", "x86_64", "all")]
    [string]$Abi = "all",
    [switch]$DownloadNcnn
)
$ErrorActionPreference = "Stop"

$Root = Split-Path $PSScriptRoot -Parent
$NcnnAndroidRoot = Join-Path $Root "third_party\ncnn\android"

$Abis = if ($Abi -eq "all") { @("arm64-v8a", "x86_64") } else { @($Abi) }

if ($DownloadNcnn) {
    & (Join-Path $PSScriptRoot "download_ncnn_android.ps1") -Abi $Abi
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

function Resolve-NdkHome {
    foreach ($candidate in @($env:ANDROID_NDK_HOME, $env:NDK_HOME)) {
        if ($candidate -and (Test-Path $candidate)) {
            return $candidate
        }
    }

    $SdkNdk = Join-Path $env:LOCALAPPDATA "Android\Sdk\ndk"
    if (Test-Path $SdkNdk) {
        $latest = Get-ChildItem $SdkNdk -Directory | Sort-Object Name -Descending | Select-Object -First 1
        if ($latest) {
            return $latest.FullName
        }
    }

    throw "Android NDK not found. Set ANDROID_NDK_HOME or install NDK via Android Studio."
}

function Test-NcnnReady {
    param([string]$Name)
    Test-Path (Join-Path $NcnnAndroidRoot "$Name\lib\libncnn.a")
}

foreach ($name in $Abis) {
    if (-not (Test-NcnnReady $name)) {
        Write-Host @"
ncnn Android library not found for ${name}:
  $NcnnAndroidRoot\$name\lib\libncnn.a

Run:
  powershell -ExecutionPolicy Bypass -File scripts/download_ncnn_android.ps1 -Abi $name
or rebuild with -DownloadNcnn
"@
        exit 1
    }
}

$NdkHome = Resolve-NdkHome
$env:ANDROID_NDK_HOME = $NdkHome
$env:NDK_HOME = $NdkHome

Write-Host "NDK: $NdkHome"

Push-Location $Root
try {
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = 'SilentlyContinue'
    rustup target add aarch64-linux-android x86_64-linux-android 2>&1 | Out-Null
    $ErrorActionPreference = $prevEap
    if ($LASTEXITCODE -ne 0) { throw "rustup target add failed for Android targets" }

    foreach ($name in $Abis) {
        $env:NCNN_LIB_DIR = Join-Path $NcnnAndroidRoot "$name\lib"
        Write-Host "NCNN_LIB_DIR: $($env:NCNN_LIB_DIR)"
        Write-Host "Building Android ABI: $name"

        cargo ndk -t $name -o android/jniLibs build --release -p ui-extractor --no-default-features --features backend-ncnn --lib
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

        $so = Join-Path $Root "android\jniLibs\$name\libui_extractor.so"
        if (-not (Test-Path $so)) {
            throw "Build output not found: $so"
        }
        Write-Host "Built: android/jniLibs/$name/libui_extractor.so"
    }

    Write-Host "Copy models/ + assets/ into app assets on the Android project."
} finally {
    Pop-Location
}
