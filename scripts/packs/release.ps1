# GitHub Release URL helpers for model packs (no catalog.json).
$script:DefaultReleaseRepo = "SuiltaPico/local-infer-core"
$script:DefaultReleaseTag = "v0.1.0"

function Get-UiExtractorRoot {
    $here = $PSScriptRoot
    if (-not $here) { $here = Split-Path -Parent $MyInvocation.MyCommand.Path }
    return (Split-Path (Split-Path $here -Parent) -Parent)
}

function Get-ReleaseTag {
    param(
        [string]$Tag = "",
        [string]$UiExtractorRoot = ""
    )

    if ($Tag) { return $(if ($Tag -match '^v') { $Tag } else { "v$Tag" }) }
    if ($env:GITHUB_REF -match '^refs/tags/(.+)$') {
        $refTag = $Matches[1]
        return $(if ($refTag -match '^v') { $refTag } else { "v$refTag" })
    }

    if (-not $UiExtractorRoot) {
        $UiExtractorRoot = Get-UiExtractorRoot
    }
    $versionLine = Select-String -Path (Join-Path $UiExtractorRoot "Cargo.toml") -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if ($versionLine) {
        return "v$($versionLine.Matches[0].Groups[1].Value)"
    }

    return $script:DefaultReleaseTag
}

function Get-ReleaseRepo {
    param([string]$Repo = "")

    if ($Repo) { return $Repo }
    return $script:DefaultReleaseRepo
}

function Get-PackReleaseUrl {
    param(
        [Parameter(Mandatory)][string]$PackId,
        [string]$Repo = "",
        [string]$Tag = ""
    )

    $repo = Get-ReleaseRepo -Repo $Repo
    $vTag = Get-ReleaseTag -Tag $Tag
    return "https://github.com/$repo/releases/download/$vTag/$PackId.zip"
}

function Test-PackInstalled {
    param(
        [Parameter(Mandatory)][string]$ModelsRoot,
        [Parameter(Mandatory)][string]$PackId
    )

    return Test-Path (Join-Path (Join-Path $ModelsRoot $PackId) "manifest.json")
}

function Expand-PackZipFile {
    param(
        [Parameter(Mandatory)][string]$ZipPath,
        [Parameter(Mandatory)][string]$PackId,
        [Parameter(Mandatory)][string]$DestRoot,
        [switch]$Force
    )

    if (-not (Test-Path $ZipPath)) {
        throw "Pack zip not found: $ZipPath"
    }

    $dest = Join-Path $DestRoot $PackId
    if ((Test-Path $dest) -and -not $Force) {
        if (Test-Path (Join-Path $dest "manifest.json")) {
            return $dest
        }
    }

    if (Test-Path $dest) { Remove-Item -Recurse -Force $dest }
    New-Item -ItemType Directory -Force -Path $dest | Out-Null
    Expand-Archive -Path $ZipPath -DestinationPath $dest -Force

    $manifest = Join-Path $dest "manifest.json"
    if (-not (Test-Path $manifest)) {
        throw "Invalid pack zip (missing manifest.json): $ZipPath"
    }

    $id = (Get-Content $manifest -Raw | ConvertFrom-Json).id
    if ($id -ne $PackId) {
        throw "manifest.id mismatch in ${ZipPath}: expected $PackId, got $id"
    }

    return $dest
}

function Install-PackFromDirectory {
    param(
        [Parameter(Mandatory)][string]$SourceDir,
        [Parameter(Mandatory)][string]$PackId,
        [Parameter(Mandatory)][string]$DestRoot,
        [switch]$Force
    )

    if (-not (Test-Path (Join-Path $SourceDir "manifest.json"))) {
        throw "Source pack dir missing manifest.json: $SourceDir"
    }

    $dest = Join-Path $DestRoot $PackId
    if ((Test-Path $dest) -and -not $Force) {
        if (Test-Path (Join-Path $dest "manifest.json")) {
            return $dest
        }
    }

    if (Test-Path $dest) { Remove-Item -Recurse -Force $dest }
    Copy-Item -Path $SourceDir -Destination $dest -Recurse -Force

    $id = (Get-Content (Join-Path $dest "manifest.json") -Raw | ConvertFrom-Json).id
    if ($id -ne $PackId) {
        throw "manifest.id mismatch in ${SourceDir}: expected $PackId, got $id"
    }

    return $dest
}

function Download-PackZip {
    param(
        [Parameter(Mandatory)][string]$Url,
        [Parameter(Mandatory)][string]$DestPath
    )

    $parent = Split-Path $DestPath -Parent
    if ($parent -and -not (Test-Path $parent)) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }

    Write-Host "downloading: $Url"
    Invoke-WebRequest -Uri $Url -OutFile $DestPath -UseBasicParsing
    return $DestPath
}
