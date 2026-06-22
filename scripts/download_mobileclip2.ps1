# Download MobileCLIP2-S0 vision encoder ONNX for icon embedding.
$ErrorActionPreference = "Stop"
$OutDir = Join-Path $PSScriptRoot "..\models"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$Base = "https://huggingface.co/plhery/mobileclip2-onnx/resolve/main/onnx/s0"
$Files = @(
    "vision_model.onnx"
)

foreach ($File in $Files) {
    $Dest = Join-Path $OutDir "mobileclip2-s0-vision.onnx"
    if (Test-Path $Dest) {
        Write-Host "skip (exists): mobileclip2-s0-vision.onnx"
        continue
    }
    $Url = "$Base/$File"
    Write-Host "downloading: $Url"
    Invoke-WebRequest -Uri $Url -OutFile $Dest
}

Write-Host "MobileCLIP2-S0 vision model ready in $OutDir"
