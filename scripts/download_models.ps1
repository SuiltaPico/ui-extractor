# Download PP-OCRv5 mobile ONNX models for ui-extractor OCR.
$ErrorActionPreference = "Stop"
$Base = "https://github.com/GreatV/oar-ocr/releases/download/v0.3.0"
$OutDir = Join-Path $PSScriptRoot "..\models"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$Files = @(
    "pp-ocrv5_mobile_det.onnx",
    "pp-ocrv5_mobile_rec.onnx",
    "ppocrv5_dict.txt"
)

foreach ($File in $Files) {
    $Dest = Join-Path $OutDir $File
    if (Test-Path $Dest) {
        Write-Host "skip (exists): $File"
        continue
    }
    $Url = "$Base/$File"
    Write-Host "downloading: $Url"
    Invoke-WebRequest -Uri $Url -OutFile $Dest
}

Write-Host "models ready in $OutDir"
