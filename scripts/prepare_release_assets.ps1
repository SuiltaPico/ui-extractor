# Download models and build embeddings.bin for release packaging.
param(
    [ValidateSet("ort", "ncnn")]
    [string]$Backend = "ort",
    [switch]$SkipEmbeddings,
    [switch]$SkipDownload,
    [string]$PnnxVersion = "20260526"
)
$ErrorActionPreference = "Stop"

$Root = Split-Path $PSScriptRoot -Parent
$ModelsDir = Join-Path $Root "models"
$AssetsDir = Join-Path $Root "assets"
$EmbeddingsPath = Join-Path $AssetsDir "embeddings.bin"

function Test-NonEmptyFile([string]$Path) {
    return (Test-Path $Path) -and ((Get-Item $Path).Length -gt 0)
}

function Ensure-Pnnx {
    try {
        $cmd = Get-Command pnnx -ErrorAction Stop
        return $cmd.Source
    } catch {
        $candidates = @(
            (Join-Path $Root "third_party/pnnx/pnnx.exe")
            (Join-Path $Root "third_party/pnnx/pnnx")
        )
        foreach ($path in $candidates) {
            if (Test-Path $path) { return $path }
        }
        $found = Get-ChildItem -Path (Join-Path $Root "third_party/pnnx") -Filter "pnnx*" -Recurse -ErrorAction SilentlyContinue |
            Where-Object { -not $_.PSIsContainer -and $_.Name -match '^pnnx(\.exe)?$' } |
            Sort-Object FullName -Descending |
            Select-Object -First 1
        if ($found) { return $found.FullName }
    }

    if ($SkipDownload) {
        throw "pnnx not found (required to convert MobileCLIP2 to ncnn). Install pnnx or run without -SkipDownload."
    }

    $isWindows = $IsWindows -or ($env:OS -match "Windows")
    $zipName = if ($isWindows) {
        "pnnx-$PnnxVersion-windows.zip"
    } else {
        "pnnx-$PnnxVersion-linux.zip"
    }
    $url = "https://github.com/pnnx/pnnx/releases/download/$PnnxVersion/$zipName"
    $destDir = Join-Path $Root "third_party/pnnx/pnnx-$PnnxVersion"
    $zipPath = Join-Path $env:TEMP $zipName

    Write-Host "Downloading pnnx: $url"
    Invoke-WebRequest -Uri $url -OutFile $zipPath
    if (Test-Path $destDir) { Remove-Item -Recurse -Force $destDir }
    Expand-Archive -Path $zipPath -DestinationPath $destDir -Force
    Remove-Item -Force $zipPath

    $binary = Get-ChildItem -Path $destDir -Filter "pnnx*" -Recurse |
        Where-Object { -not $_.PSIsContainer -and $_.Name -match '^pnnx(\.exe)?$' } |
        Select-Object -First 1
    if (-not $binary) { throw "pnnx binary not found after extracting $zipName" }
    return $binary.FullName
}

function Ensure-OrtModels {
    if ($SkipDownload) { return }
    & (Join-Path $PSScriptRoot "download_models.ps1")
    & (Join-Path $PSScriptRoot "download_mobileclip2.ps1")
}

function Ensure-NcnnModels {
    if ($SkipDownload) { return }
    & (Join-Path $PSScriptRoot "download_models_ncnn.ps1")
    & (Join-Path $PSScriptRoot "download_models.ps1")
    & (Join-Path $PSScriptRoot "download_mobileclip2.ps1")

    $mobileclipParam = Join-Path $ModelsDir "mobileclip2-s0-vision.ncnn.param"
    $mobileclipBin = Join-Path $ModelsDir "mobileclip2-s0-vision.ncnn.bin"
    if ((Test-NonEmptyFile $mobileclipParam) -and (Test-NonEmptyFile $mobileclipBin)) {
        Write-Host "skip MobileCLIP2 ncnn convert (exists)"
        return
    }

    $pnnx = Ensure-Pnnx
    Write-Host "Using pnnx: $pnnx"
    $env:PATH = "$(Split-Path $pnnx -Parent);$env:PATH"

    Push-Location $Root
    try {
        & (Join-Path $PSScriptRoot "convert_models_ncnn.ps1")
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    } finally {
        Pop-Location
    }
}

function Ensure-Embeddings {
    if ($SkipEmbeddings) { return }
    if (Test-NonEmptyFile $EmbeddingsPath) {
        Write-Host "skip embeddings (exists): $EmbeddingsPath"
        return
    }

    $iconsDir = Join-Path $AssetsDir "icons"
    $iconCount = 0
    if (Test-Path $iconsDir) {
        $iconCount = (Get-ChildItem $iconsDir -Filter *.png -ErrorAction SilentlyContinue).Count
    }
    if ($iconCount -eq 0) {
        Write-Host "Building icon PNG templates from MDI..."
        & (Join-Path $PSScriptRoot "download_mdi_icons.ps1") -Rasterize
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }

    # embeddings.bin is backend-agnostic; build with ONNX (works on Windows and Linux CI).
    $onnxVision = Join-Path $ModelsDir "mobileclip2-s0-vision.onnx"
    if (-not (Test-NonEmptyFile $onnxVision)) {
        if ($SkipDownload) {
            throw "Missing $onnxVision — run download_mobileclip2.ps1 or omit -SkipDownload"
        }
        & (Join-Path $PSScriptRoot "download_mobileclip2.ps1")
    }

    Write-Host "Building embeddings.bin (this may take a few minutes)..."
    Push-Location $Root
    try {
        cargo run --release -- icon build-embeddings
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    } finally {
        Pop-Location
    }

    if (-not (Test-NonEmptyFile $EmbeddingsPath)) {
        throw "embeddings build did not produce $EmbeddingsPath"
    }
}

function Assert-ReleaseAssets {
    param([ValidateSet("ort", "ncnn")][string]$Kind)

    $required = if ($Kind -eq "ort") {
        @(
            "pp-ocrv5_mobile_det.onnx",
            "pp-ocrv5_mobile_rec.onnx",
            "ppocrv5_dict.txt",
            "mobileclip2-s0-vision.onnx"
        )
    } else {
        @(
            "pp-ocrv5_mobile_det.ncnn.param",
            "pp-ocrv5_mobile_det.ncnn.bin",
            "pp-ocrv5_mobile_rec.ncnn.param",
            "pp-ocrv5_mobile_rec.ncnn.bin",
            "mobileclip2-s0-vision.ncnn.param",
            "mobileclip2-s0-vision.ncnn.bin",
            "ppocrv5_dict.txt"
        )
    }

    foreach ($name in $required) {
        $path = Join-Path $ModelsDir $name
        if (-not (Test-NonEmptyFile $path)) {
            throw "Missing release asset: $path"
        }
    }
    if (-not (Test-NonEmptyFile $EmbeddingsPath)) {
        throw "Missing release asset: $EmbeddingsPath"
    }
}

Push-Location $Root
try {
    switch ($Backend) {
        "ort" { Ensure-OrtModels }
        "ncnn" { Ensure-NcnnModels }
    }
    Ensure-Embeddings
    Assert-ReleaseAssets -Kind $Backend
    Write-Host "Release assets ready ($Backend)."
} finally {
    Pop-Location
}
