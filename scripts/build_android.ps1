# Build ui-extractor cdylib for Android (arm64-v8a / x86_64) with MNN via infer-core.
param(
    [ValidateSet("arm64-v8a", "x86_64", "all")]
    [string]$Abi = "all",

    [switch]$DownloadMnn
)
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "cargo_retry.ps1")

$Root = Split-Path $PSScriptRoot -Parent
$InferCoreRoot = Join-Path (Split-Path $Root -Parent) "local-infer-core"
$MnnRoot = Join-Path $InferCoreRoot "third_party\mnn"
$MnnSource = Join-Path $MnnRoot "source"
$MnnAndroidRoot = Join-Path $MnnRoot "android"
$Abis = if ($Abi -eq "all") { @("arm64-v8a", "x86_64") } else { @($Abi) }

if ($DownloadMnn) {
    $dl = Join-Path $InferCoreRoot "scripts\download_mnn_android.ps1"
    if (-not (Test-Path $dl)) { throw "Missing infer-core script: $dl" }
    & $dl -Abi all
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

function Test-MnnAbiReady {
    param([string]$Name)
    Test-Path (Join-Path $MnnAndroidRoot "$Name\libMNN.so")
}

function Resolve-NdkLibcxxShared {
    param(
        [string]$NdkHome,
        [string]$Abi
    )

    $hostTag = if ($IsWindows -or $env:OS -eq "Windows_NT") { "windows-x86_64" } else { "linux-x86_64" }
    $triple = switch ($Abi) {
        "arm64-v8a" { "aarch64-linux-android" }
        "x86_64" { "x86_64-linux-android" }
        default { throw "unsupported Android ABI: $Abi" }
    }
    $candidate = Join-Path $NdkHome "toolchains\llvm\prebuilt\$hostTag\sysroot\usr\lib\$triple\libc++_shared.so"
    if (-not (Test-Path $candidate)) {
        throw "NDK libc++_shared.so not found: $candidate"
    }
    return $candidate
}

function Copy-MnnRuntimeLibs {
    param(
        [string]$Abi,
        [string]$JniDir,
        [string]$NdkHome
    )

    $runtimeLibs = @(
        "libMNN.so",
        "libc++_shared.so",
        "libMNN_CL.so",
        "libMNN_Vulkan.so"
    )
    foreach ($lib in $runtimeLibs) {
        $src = Join-Path $MnnAndroidRoot "$Abi\$lib"
        if (-not (Test-Path $src) -and $lib -eq "libc++_shared.so") {
            $src = Resolve-NdkLibcxxShared -NdkHome $NdkHome -Abi $Abi
        }
        if (Test-Path $src) {
            Copy-Item $src (Join-Path $JniDir $lib) -Force
        }
    }
}

foreach ($name in $Abis) {
    if (Test-MnnAbiReady $name) { continue }

    if ($name -eq "arm64-v8a") {
        Write-Host @"
MNN Android prebuilt library not found:
  $MnnAndroidRoot\arm64-v8a\libMNN.so

Run in local-infer-core:
  powershell -ExecutionPolicy Bypass -File scripts/download_mnn_android.ps1
or rebuild with -DownloadMnn
"@
        exit 1
    }

    if ($name -eq "x86_64") {
        $buildX86 = Join-Path $InferCoreRoot "scripts\build_mnn_android_x86_64.ps1"
        & $buildX86
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
}

if (-not (Test-Path (Join-Path $MnnSource "include\MNN\Interpreter.hpp"))) {
    $dl = Join-Path $InferCoreRoot "scripts\download_mnn_android.ps1"
    & $dl -Abi arm64-v8a
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

$NdkHome = Resolve-NdkHome
$env:ANDROID_NDK_HOME = $NdkHome
$env:NDK_HOME = $NdkHome
$env:MNN_SRC = $MnnSource

Write-Host "NDK: $NdkHome"
Write-Host "MNN_SRC: $MnnSource"

Push-Location $Root
try {
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = 'SilentlyContinue'
    rustup target add aarch64-linux-android x86_64-linux-android 2>&1 | Out-Null
    $ErrorActionPreference = $prevEap
    if ($LASTEXITCODE -ne 0) { throw "rustup target add failed for Android targets" }

    foreach ($name in $Abis) {
        $env:MNN_COMPILE = "0"
        $env:MNN_LINK = "dylib"
        $env:MNN_LIB_DIR = Join-Path $MnnAndroidRoot $name

        Write-Host "MNN_LIB_DIR: $($env:MNN_LIB_DIR)"
        Write-Host "Building Android ABI: $name (backend-mnn)"

        cargo ndk -t $name -o android/jniLibs build --release -p ui-extractor `
            --no-default-features --features backend-mnn --lib
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

        $jniDir = Join-Path $Root "android\jniLibs\$name"
        $so = Join-Path $jniDir "libui_extractor.so"
        if (-not (Test-Path $so)) {
            throw "Build output not found: $so"
        }

        Copy-MnnRuntimeLibs -Abi $name -JniDir $jniDir -NdkHome $NdkHome

        $libCount = (Get-ChildItem $jniDir -Filter "*.so").Count
        $sizeMb = [math]::Round((Get-Item $so).Length / 1MB, 2)
        Write-Host "Built: android/jniLibs/$name/libui_extractor.so ($sizeMb MB, $libCount .so files)"
    }

    Write-Host "Copy models/ MNN packs into app assets (see scripts/install_packs.ps1 -Platform android)."
} finally {
    Pop-Location
}
