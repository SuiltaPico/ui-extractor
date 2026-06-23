# Convert ONNX models in models/ to ncnn (.ncnn.param + .ncnn.bin) via pnnx.
# Requires: pnnx on PATH, or portable pnnx.exe under third_party/pnnx/
#           ONNX files from scripts/download_*.ps1
$ErrorActionPreference = "Stop"

$Root = Split-Path $PSScriptRoot -Parent
$ModelsDir = Join-Path $Root "models"
if (-not (Test-Path $ModelsDir)) {
    throw "models/ not found — run scripts/download_models.ps1 and scripts/download_mobileclip2.ps1 first"
}

function Resolve-Pnnx {
    try {
        $cmd = Get-Command pnnx -ErrorAction Stop
        return $cmd.Source
    } catch {
        # Portable binary (recommended on Windows — no Python / PyTorch needed)
        $candidates = Get-ChildItem -Path (Join-Path $Root "third_party\pnnx") -Filter "pnnx.exe" -Recurse -ErrorAction SilentlyContinue |
            Sort-Object FullName -Descending
        if ($candidates) {
            return $candidates[0].FullName
        }
        foreach ($name in @("pnnx.exe", "pnnx")) {
            $candidate = Join-Path $Root "third_party/pnnx/$name"
            if (Test-Path $candidate) {
                return $candidate
            }
        }
        $found = Get-ChildItem -Path (Join-Path $Root "third_party/pnnx") -Filter "pnnx*" -Recurse -ErrorAction SilentlyContinue |
            Where-Object { -not $_.PSIsContainer -and $_.Name -match '^pnnx(\.exe)?$' } |
            Sort-Object FullName -Descending |
            Select-Object -First 1
        if ($found) {
            return $found.FullName
        }
    }
    return $null
}

$Pnnx = Resolve-Pnnx
if (-not $Pnnx) {
    Write-Host @"
pnnx not found.

Option A (recommended, no Python):
  1. Download pnnx-*-windows.zip from https://github.com/pnnx/pnnx/releases
  2. Extract pnnx.exe to third_party/pnnx/  (e.g. third_party/pnnx/pnnx-20241226-windows/pnnx.exe)

Option B: put pnnx on PATH (pip install pnnx also works but pulls PyTorch on Windows)

OCR ncnn weights can be downloaded pre-converted:
  powershell -ExecutionPolicy Bypass -File scripts/download_models_ncnn.ps1
"@
    exit 1
}

Write-Host "Using pnnx: $Pnnx"

Push-Location $ModelsDir
try {
    $Conversions = @(
        @{
            Onnx = "mobileclip2-s0-vision.onnx"
            Args = "inputshape=[1,3,256,256]"
        },
        @{
            Onnx = "pp-ocrv5_mobile_det.onnx"
            Args = "inputshape=[1,3,320,320] inputshape2=[1,3,256,256]"
        },
        @{
            Onnx = "pp-ocrv5_mobile_rec.onnx"
            Args = "inputshape=[1,3,48,160] inputshape2=[1,3,48,256]"
        }
    )

    foreach ($item in $Conversions) {
        $onnx = $item.Onnx
        if (-not (Test-Path $onnx)) {
            Write-Warning "skip (missing): $onnx"
            continue
        }

        $stem = [System.IO.Path]::GetFileNameWithoutExtension($onnx)
        $param = "$stem.ncnn.param"
        $bin = "$stem.ncnn.bin"
        if ((Test-Path $param) -and (Test-Path $bin)) {
            Write-Host "skip (exists): $param"
            continue
        }

        Write-Host "pnnx $onnx $($item.Args)"
        & $Pnnx $onnx @($item.Args -split '\s+')
        if ($LASTEXITCODE -ne 0) {
            throw "pnnx failed for $onnx (exit $LASTEXITCODE)"
        }
        if (-not ((Test-Path $param) -and (Test-Path $bin))) {
            # pnnx may emit underscores instead of hyphens in the stem (e.g. mobileclip2_s0_vision.ncnn.*)
            $altStem = $stem -replace '-', '_'
            $altParam = "$altStem.ncnn.param"
            $altBin = "$altStem.ncnn.bin"
            if ((Test-Path $altParam) -and (Test-Path $altBin)) {
                Copy-Item $altParam $param -Force
                Copy-Item $altBin $bin -Force
            }
        }
        if (-not ((Test-Path $param) -and (Test-Path $bin))) {
            throw "pnnx did not produce $param / $bin"
        }
    }

    Write-Host "ncnn models ready in $ModelsDir"
    Get-ChildItem -Filter "*.ncnn.*" | ForEach-Object { Write-Host "  $($_.Name)" }
} finally {
    Pop-Location
}
