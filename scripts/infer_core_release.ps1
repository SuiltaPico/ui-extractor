# Resolve and download infer-core native library zips from local-infer-core GitHub Releases.
$script:DefaultInferCoreReleaseRepo = "SuiltaPico/local-infer-core"

$script:WindowsReleaseAssets = @{
    "x86_64-pc-windows-msvc"  = "infer-core-windows-x86_64"
    "aarch64-pc-windows-msvc" = "infer-core-windows-aarch64"
}

$script:AndroidReleaseAssets = @{
    "arm64-v8a" = "infer-core-android-arm64-v8a"
    "x86_64"    = "infer-core-android-x86_64"
}

function Get-InferCoreReleaseRepo {
    param([string]$Repo = "")

    if ($Repo) { return $Repo }
    return $script:DefaultInferCoreReleaseRepo
}

function Get-InferCoreReleaseTag {
    param(
        [string]$Tag = "",
        [string]$UiExtractorRoot = ""
    )

    if ($Tag) {
        return $(if ($Tag -match '^v') { $Tag } else { "v$Tag" })
    }
    if ($env:GITHUB_REF_NAME) {
        $refTag = $env:GITHUB_REF_NAME
        return $(if ($refTag -match '^v') { $refTag } else { "v$refTag" })
    }

    if (-not $UiExtractorRoot) {
        $UiExtractorRoot = Split-Path $PSScriptRoot -Parent
    }
    $versionLine = Select-String -Path (Join-Path $UiExtractorRoot "Cargo.toml") -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if ($versionLine) {
        return "v$($versionLine.Matches[0].Groups[1].Value)"
    }

    throw "Could not resolve infer-core release tag (pass -ReleaseTag or bump Cargo.toml version)"
}

function Get-InferCoreReleaseUrl {
    param(
        [Parameter(Mandatory)][string]$AssetBaseName,
        [string]$Repo = "",
        [string]$Tag = ""
    )

    $resolvedRepo = Get-InferCoreReleaseRepo -Repo $Repo
    $resolvedTag = Get-InferCoreReleaseTag -Tag $Tag
    return "https://github.com/$resolvedRepo/releases/download/$resolvedTag/$AssetBaseName.zip"
}

function Get-InferCoreReleaseRoot {
    param([string]$OutDir = "")

    if ($OutDir) {
        return $(if ([IO.Path]::IsPathRooted($OutDir)) { $OutDir } else { Join-Path (Split-Path $PSScriptRoot -Parent) $OutDir })
    }
    return Join-Path (Split-Path $PSScriptRoot -Parent) ".infer-core-release"
}

function Get-InferCoreReleaseAssetBaseName {
    param(
        [string]$Triple = "",
        [string]$Abi = ""
    )

    if ($Triple -and $script:WindowsReleaseAssets.ContainsKey($Triple)) {
        return $script:WindowsReleaseAssets[$Triple]
    }
    if ($Abi -and $script:AndroidReleaseAssets.ContainsKey($Abi)) {
        return $script:AndroidReleaseAssets[$Abi]
    }

    throw "Unknown infer-core release asset (triple=$Triple abi=$Abi)"
}

function Ensure-InferCoreReleaseAsset {
    param(
        [Parameter(Mandatory)][string]$AssetBaseName,
        [string]$ReleaseRoot = "",
        [string]$Repo = "",
        [string]$Tag = "",
        [switch]$Force
    )

    . (Join-Path $PSScriptRoot "cargo_retry.ps1")

    $root = Get-InferCoreReleaseRoot -OutDir $ReleaseRoot
    $extractDir = Join-Path $root $AssetBaseName
    $marker = Join-Path $extractDir ".extracted"
    if ((Test-Path $marker) -and -not $Force) {
        return $extractDir
    }

    $cacheDir = Join-Path (Get-ScratchDir) "infer-core-release-cache"
    New-Item -ItemType Directory -Force -Path $cacheDir | Out-Null
    $zipPath = Join-Path $cacheDir "$AssetBaseName.zip"

    $url = Get-InferCoreReleaseUrl -AssetBaseName $AssetBaseName -Repo $Repo -Tag $Tag
    if (-not (Test-Path $zipPath) -or $Force) {
        Write-Host "Downloading infer-core release asset: $url"
        Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing
    }

    if (Test-Path $extractDir) { Remove-Item -Recurse -Force $extractDir }
    New-Item -ItemType Directory -Force -Path $extractDir | Out-Null
    Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force
    Set-Content -Path $marker -Value $url -Encoding UTF8

    return $extractDir
}

function Resolve-InferCoreWindowsLibDir {
    param(
        [Parameter(Mandatory)][string]$Triple,
        [string]$ReleaseRoot = "",
        [string]$Repo = "",
        [string]$Tag = "",
        [switch]$Force
    )

    $asset = Get-InferCoreReleaseAssetBaseName -Triple $Triple
    $extractDir = Ensure-InferCoreReleaseAsset -AssetBaseName $asset -ReleaseRoot $ReleaseRoot -Repo $Repo -Tag $Tag -Force:$Force
    $libDir = Join-Path $extractDir "lib"
    $dll = Join-Path $libDir "infer_core.dll"
    if (-not (Test-Path $dll)) {
        throw "Invalid infer-core Windows release layout (expected lib/infer_core.dll): $asset"
    }

    $importLib = Join-Path $libDir "infer_core.dll.lib"
    if (-not (Test-Path $importLib)) {
        throw @"
infer-core Windows release is missing infer_core.dll.lib (required to link ui-extractor).
Publish a new local-infer-core release with lib/infer_core.dll.lib.
Asset: $asset
"@
    }

    return (Resolve-Path $libDir).Path
}

function Resolve-InferCoreAndroidJniDir {
    param(
        [Parameter(Mandatory)][string]$Abi,
        [string]$ReleaseRoot = "",
        [string]$Repo = "",
        [string]$Tag = "",
        [switch]$Force
    )

    $asset = Get-InferCoreReleaseAssetBaseName -Abi $Abi
    $extractDir = Ensure-InferCoreReleaseAsset -AssetBaseName $asset -ReleaseRoot $ReleaseRoot -Repo $Repo -Tag $Tag -Force:$Force
    $jniDir = Join-Path $extractDir "jniLibs\$Abi"
    $inferSo = Join-Path $jniDir "libinfer_core.so"
    if (-not (Test-Path $inferSo)) {
        throw "Invalid infer-core Android release layout (expected jniLibs/$Abi/libinfer_core.so): $asset"
    }

    return (Resolve-Path $jniDir).Path
}

function Copy-InferCoreRuntimeDll {
    param(
        [Parameter(Mandatory)][string]$Triple,
        [Parameter(Mandatory)][string]$CargoOutDir,
        [string]$ReleaseRoot = "",
        [string]$Repo = "",
        [string]$Tag = ""
    )

    $libDir = Resolve-InferCoreWindowsLibDir -Triple $Triple -ReleaseRoot $ReleaseRoot -Repo $Repo -Tag $Tag
    $dll = Join-Path $libDir "infer_core.dll"
    New-Item -ItemType Directory -Force -Path $CargoOutDir | Out-Null
    Copy-Item $dll $CargoOutDir -Force
    Write-Host "Copied infer_core.dll -> $CargoOutDir"
}
