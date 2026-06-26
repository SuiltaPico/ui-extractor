# Build ui-extractor cdylib for Android (arm64-v8a / x86_64) linked against infer-core (MNN).
param(
    [ValidateSet("arm64-v8a", "x86_64", "all")]
    [string]$Abi = "all",

    [switch]$DownloadMnn,
    [string]$ReleaseRepo = "",
    [string]$ReleaseTag = "",
    [string]$InferCoreReleaseDir = ""
)
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "cargo_retry.ps1")
. (Join-Path $PSScriptRoot "infer_core_release.ps1")

$Root = Split-Path $PSScriptRoot -Parent
$Abis = if ($Abi -eq "all") { @("arm64-v8a", "x86_64") } else { @($Abi) }

if ($DownloadMnn) {
    Write-Host "DownloadMnn is deprecated: infer-core Android libs come from local-infer-core Releases."
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

Write-Host "NDK: $NdkHome"

Push-Location $Root
try {
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = 'SilentlyContinue'
    rustup target add aarch64-linux-android x86_64-linux-android 2>&1 | Out-Null
    $ErrorActionPreference = $prevEap
    if ($LASTEXITCODE -gt 0) { throw "rustup target add failed for Android targets" }

    foreach ($name in $Abis) {
        $inferJni = Resolve-InferCoreAndroidJniDir `
            -Abi $name `
            -ReleaseRoot $InferCoreReleaseDir `
            -Repo $ReleaseRepo `
            -Tag $ReleaseTag
        $inferSo = Join-Path $inferJni "libinfer_core.so"
        if (-not (Test-Path $inferSo)) {
            throw "infer-core release output not found: $inferSo"
        }

        $env:INFER_CORE_LIB_DIR = $inferJni
        Remove-Item Env:MNN_LIB_DIR -ErrorAction SilentlyContinue
        Remove-Item Env:MNN_COMPILE -ErrorAction SilentlyContinue
        Remove-Item Env:MNN_LINK -ErrorAction SilentlyContinue
        Remove-Item Env:MNN_SRC -ErrorAction SilentlyContinue

        Write-Host "Building ui-extractor Android ABI: $name (linking infer_core from $inferJni)"
        cargo ndk -t $name -o android/jniLibs build --release -p ui-extractor --lib
        if ($LASTEXITCODE -gt 0) { exit $LASTEXITCODE }

        $jniDir = Join-Path $Root "android\jniLibs\$name"
        New-Item -ItemType Directory -Force -Path $jniDir | Out-Null
        Get-ChildItem $inferJni -Filter "*.so" | ForEach-Object {
            Copy-Item $_.FullName (Join-Path $jniDir $_.Name) -Force
        }

        $so = Join-Path $jniDir "libui_extractor.so"
        if (-not (Test-Path $so)) {
            throw "Build output not found: $so"
        }

        $libCount = (Get-ChildItem $jniDir -Filter "*.so").Count
        $sizeMb = [math]::Round((Get-Item $so).Length / 1MB, 2)
        Write-Host "Built: android/jniLibs/$name/libui_extractor.so ($sizeMb MB, $libCount .so files)"
    }

    Write-Host "Copy models/ MNN packs into app assets (see scripts/install_packs.ps1 -Platform android)."
} finally {
    Pop-Location
}
