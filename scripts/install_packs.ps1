# Install local-infer-core model packs into ui-extractor models_dir layout.
param(
    [ValidateSet("windows", "android")]
    [string]$Platform = "windows",

    [string]$ModelsDir = "models",

    # Explicit local zip directory (highest-priority local override).
    [string]$DistDir = "",

    # auto = local overrides -> GitHub Release -> dev sibling dist/fixtures
    # release = GitHub Release only (after skip-if-installed)
    # local = local overrides + dev fallbacks only (no network)
    [ValidateSet("auto", "release", "local")]
    [string]$Source = "auto",

    [string]$ReleaseRepo = "",
    [string]$ReleaseTag = "",
    [string]$CatalogPath = "",

    [switch]$Force
)
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "cargo_retry.ps1")
. (Join-Path $PSScriptRoot "infer_core_root.ps1")

$Root = Split-Path $PSScriptRoot -Parent
$ModelsRoot = if ([IO.Path]::IsPathRooted($ModelsDir)) { $ModelsDir } else { Join-Path $Root $ModelsDir }
$InferCoreRoot = Get-InferCoreRoot -UiExtractorRoot $Root
$CatalogScript = Join-Path $InferCoreRoot "scripts\packs\catalog.ps1"
if (-not (Test-Path $CatalogScript)) {
    throw "Missing infer-core catalog helpers: $CatalogScript"
}
. $CatalogScript

function Resolve-LocalZipDir {
    if ($DistDir) {
        return $(if ([IO.Path]::IsPathRooted($DistDir)) { $DistDir } else { Join-Path $Root $DistDir })
    }
    $envDir = $env:LOCAL_INFER_PACK_SOURCE
    if ($envDir) {
        return $(if ([IO.Path]::IsPathRooted($envDir)) { $envDir } else { Join-Path $Root $envDir })
    }
    return $null
}

function Resolve-DevDistDir {
    $candidate = Join-Path $InferCoreRoot "dist"
    if (Test-Path $candidate) { return $candidate }
    return $null
}

function Resolve-DevFixtureDir {
    param([string]$PackId)

    $envRoot = $env:LOCAL_INFER_FIXTURE_ROOT
    if ($envRoot) {
        $dir = Join-Path $envRoot $PackId
        if (Test-Path (Join-Path $dir "manifest.json")) { return $dir }
    }

    $candidate = Join-Path $InferCoreRoot "crates\infer-core\tests\fixtures\$PackId"
    if (Test-Path (Join-Path $candidate "manifest.json")) { return $candidate }
    return $null
}

function Set-IconEmbedModelId {
    param(
        [string]$IconPackDir,
        [string]$EmbedPackId
    )

    $manifestPath = Join-Path $IconPackDir "manifest.json"
    $manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
    if ($manifest.embed_model_id -eq $EmbedPackId) { return }
    $manifest.embed_model_id = $EmbedPackId
    ($manifest | ConvertTo-Json -Depth 20) + "`n" | Set-Content -Path $manifestPath -Encoding UTF8 -NoNewline
    Write-Host "patched icon embed_model_id -> $EmbedPackId"
}

function Install-OnePack {
    param(
        [string]$PackId,
        [string]$ModelsRoot,
        [string]$SourceMode,
        [string]$LocalZipDir,
        [string]$ReleaseRepo,
        [string]$ReleaseTag,
        [string]$CatalogPath,
        [switch]$Force
    )

    if ((Test-PackInstalled -ModelsRoot $ModelsRoot -PackId $PackId) -and -not $Force) {
        Write-Host "skip (exists): $PackId"
        return (Join-Path $ModelsRoot $PackId)
    }

    $entry = Get-CatalogPackEntry -PackId $PackId -CatalogPath $CatalogPath
    $expectedSha = if ($entry) { [string]$entry.sha256 } else { "" }

    if ($SourceMode -in @("auto", "local")) {
        if ($LocalZipDir) {
            $zip = Join-Path $LocalZipDir "$PackId.zip"
            if (Test-Path $zip) {
                Write-Host "install from local zip: $zip"
                $dest = Expand-PackZipFile -ZipPath $zip -PackId $PackId -DestRoot $ModelsRoot -Force:$Force
                Write-Host "installed: $PackId -> $dest"
                return $dest
            }
        }

        $fixture = Resolve-DevFixtureDir -PackId $PackId
        if ($fixture) {
            Write-Host "install from fixture: $fixture"
            $dest = Install-PackFromDirectory -SourceDir $fixture -PackId $PackId -DestRoot $ModelsRoot -Force:$Force
            Write-Host "installed: $PackId -> $dest"
            return $dest
        }

        if ($SourceMode -eq "auto") {
            $devDist = Resolve-DevDistDir
            if ($devDist) {
                $zip = Join-Path $devDist "$PackId.zip"
                if (Test-Path $zip) {
                    Write-Host "install from dev dist: $zip"
                    $dest = Expand-PackZipFile -ZipPath $zip -PackId $PackId -DestRoot $ModelsRoot -Force:$Force
                    Write-Host "installed: $PackId -> $dest"
                    return $dest
                }
            }
        }
    }

    if ($SourceMode -in @("auto", "release")) {
        $url = Get-PackDownloadUrl -PackId $PackId -Repo $ReleaseRepo -Tag $ReleaseTag -CatalogPath $CatalogPath
        $cacheDir = Join-Path (Get-ScratchDir) "local-infer-pack-cache"
        New-Item -ItemType Directory -Force -Path $cacheDir | Out-Null
        $cachedZip = Join-Path $cacheDir "$PackId.zip"

        if (-not (Test-Path $cachedZip) -or $Force) {
            Download-PackZip -Url $url -DestPath $cachedZip -ExpectedSha256 $expectedSha
        } elseif ($expectedSha -and $expectedSha.Trim()) {
            $hash = (Get-FileHash -Path $cachedZip -Algorithm SHA256).Hash.ToLowerInvariant()
            if ($hash -ne $expectedSha.Trim().ToLowerInvariant()) {
                Remove-Item -Force $cachedZip
                Download-PackZip -Url $url -DestPath $cachedZip -ExpectedSha256 $expectedSha
            }
        }

        $dest = Expand-PackZipFile -ZipPath $cachedZip -PackId $PackId -DestRoot $ModelsRoot -Force:$Force
        Write-Host "installed: $PackId -> $dest"
        return $dest
    }

    throw @"
Could not install pack: $PackId
  Source=$SourceMode
  Tried: local zip dir, fixtures, dev dist (auto only), GitHub Release (auto/release)
  Override with -DistDir, LOCAL_INFER_PACK_SOURCE, or LOCAL_INFER_FIXTURE_ROOT
"@
}

$packSets = @{
    windows = @{
        Packs = @(
            "ocr.paddle.ppocr6-tiny.onnx.fp32",
            "embed.mobileclip2-s0.onnx.fp32",
            "icons.bundled.v1.mobileclip2-s0.int8"
        )
        EmbedPackId = "embed.mobileclip2-s0.onnx.fp32"
    }
    android = @{
        Packs = @(
            "ocr.paddle.ppocr6-tiny.mnn.fp32",
            "embed.mobileclip2-s0.mnn.fp32",
            "icons.bundled.v1.mobileclip2-s0.int8"
        )
        EmbedPackId = "embed.mobileclip2-s0.mnn.fp32"
    }
}

New-Item -ItemType Directory -Force -Path $ModelsRoot | Out-Null
$localZipDir = Resolve-LocalZipDir
$set = $packSets[$Platform]
$iconPackDir = $null

foreach ($packId in $set.Packs) {
    $dir = Install-OnePack `
        -PackId $packId `
        -ModelsRoot $ModelsRoot `
        -SourceMode $Source `
        -LocalZipDir $localZipDir `
        -ReleaseRepo $ReleaseRepo `
        -ReleaseTag $ReleaseTag `
        -CatalogPath $CatalogPath `
        -Force:$Force
    if ($packId -like "icons.bundled.*") {
        $iconPackDir = $dir
    }
}

if ($Platform -eq "android" -and $iconPackDir) {
    Set-IconEmbedModelId -IconPackDir $iconPackDir -EmbedPackId $set.EmbedPackId
}

Write-Host "Model packs ready for $Platform under $ModelsRoot (source mode: $Source)"
