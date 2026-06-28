# Install model packs from local-infer-core GitHub Releases into models_dir layout.
param(
    [ValidateSet("windows", "android")]
    [string]$Platform = "windows",

    [string]$ModelsDir = "models",

    [string]$DistDir = "",

    [ValidateSet("release", "local")]
    [string]$Source = "release",

    [string]$ReleaseRepo = "",
    [string]$ReleaseTag = "",

    [switch]$Force
)
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "cargo_retry.ps1")
. (Join-Path $PSScriptRoot "packs\release.ps1")

$Root = Split-Path $PSScriptRoot -Parent
$ModelsRoot = if ([IO.Path]::IsPathRooted($ModelsDir)) { $ModelsDir } else { Join-Path $Root $ModelsDir }

function Resolve-LocalZipDir {
    if ($DistDir) {
        return $(if ([IO.Path]::IsPathRooted($DistDir)) { $DistDir } else { Join-Path $Root $DistDir })
    }
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
        [switch]$Force
    )

    if ((Test-PackInstalled -ModelsRoot $ModelsRoot -PackId $PackId) -and -not $Force) {
        Write-Host "skip (exists): $PackId"
        return (Join-Path $ModelsRoot $PackId)
    }

    if ($SourceMode -eq "local") {
        if (-not $LocalZipDir) {
            throw "Source=local requires -DistDir with {pack_id}.zip files"
        }
        $zip = Join-Path $LocalZipDir "$PackId.zip"
        if (-not (Test-Path $zip)) {
            throw "Pack zip not found for local install: $zip"
        }
        Write-Host "install from local zip: $zip"
        $dest = Expand-PackZipFile -ZipPath $zip -PackId $PackId -DestRoot $ModelsRoot -Force:$Force
        Write-Host "installed: $PackId -> $dest"
        return $dest
    }

    $url = Get-PackReleaseUrl -PackId $PackId -Repo $ReleaseRepo -Tag $ReleaseTag
    $cacheDir = Join-Path (Get-ScratchDir) "local-infer-pack-cache"
    New-Item -ItemType Directory -Force -Path $cacheDir | Out-Null
    $cachedZip = Join-Path $cacheDir "$PackId.zip"

    if (-not (Test-Path $cachedZip) -or $Force) {
        Download-PackZip -Url $url -DestPath $cachedZip
    }

    $dest = Expand-PackZipFile -ZipPath $cachedZip -PackId $PackId -DestRoot $ModelsRoot -Force:$Force
    Write-Host "installed: $PackId -> $dest"
    return $dest
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
        -Force:$Force
    if ($packId -like "icons.bundled.*") {
        $iconPackDir = $dir
    }
}

if ($Platform -eq "android" -and $iconPackDir) {
    Set-IconEmbedModelId -IconPackDir $iconPackDir -EmbedPackId $set.EmbedPackId
}

Write-Host "Model packs ready for $Platform under $ModelsRoot (source: $Source)"
