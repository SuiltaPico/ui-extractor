# Download pre-converted PP-OCRv5 mobile ncnn models (det + rec) from ncnn-assets.
# MobileCLIP2 must still be converted locally via scripts/convert_models_ncnn.ps1.
$ErrorActionPreference = "Stop"

$Base = "https://github.com/nihui/ncnn-assets/raw/master/models"
$OutDir = Join-Path $PSScriptRoot "..\models"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$Files = @(
    @{ Remote = "PP_OCRv5_mobile_det.ncnn.param"; Local = "pp-ocrv5_mobile_det.ncnn.param" },
    @{ Remote = "PP_OCRv5_mobile_det.ncnn.bin"; Local = "pp-ocrv5_mobile_det.ncnn.bin" },
    @{ Remote = "PP_OCRv5_mobile_rec.ncnn.param"; Local = "pp-ocrv5_mobile_rec.ncnn.param" },
    @{ Remote = "PP_OCRv5_mobile_rec.ncnn.bin"; Local = "pp-ocrv5_mobile_rec.ncnn.bin" }
)

foreach ($file in $Files) {
    $dest = Join-Path $OutDir $file.Local
    if (Test-Path $dest) {
        Write-Host "skip (exists): $($file.Local)"
        continue
    }
    $url = "$Base/$($file.Remote)"
    Write-Host "downloading: $url"
    Invoke-WebRequest -Uri $url -OutFile $dest
}

# Dict is shared between ONNX and ncnn backends.
$Dict = Join-Path $OutDir "ppocrv5_dict.txt"
if (-not (Test-Path $Dict)) {
    Write-Host "dict missing — run scripts/download_models.ps1 for ppocrv5_dict.txt"
}

Write-Host "OCR ncnn models ready in $OutDir"
