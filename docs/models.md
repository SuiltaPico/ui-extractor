# 模型包说明

`ui-extractor` 已不再维护独立模型下载/转换流程。  
OCR、embed、icon_index 全部由 [`local-infer-core`](../../local-infer-core/docs/dev/PRODUCT.md) 的 manifest 模型包提供。

## 模型目录约定

运行时通过 `--models-dir` 指向模型根目录，按 `pack_id` 查找：

```text
{models_dir}/
├── ocr.paddle.ppocr6-tiny.onnx.fp32/
│   ├── manifest.json
│   ├── det.onnx
│   ├── rec.onnx
│   └── ppocrv6_tiny_dict.txt
├── embed.mobileclip2-s0.onnx.fp32/
└── icons.bundled.v1.mobileclip2-s0.int8/
```

## 推荐准备方式

在 `local-infer-core` 仓库执行：

```powershell
powershell -ExecutionPolicy Bypass -File ..\local-infer-core\scripts\download_all_packs.ps1
```

或按需下载单个 pack（同目录下 `download_*_pack.ps1` 脚本）。

## ui-extractor 侧用法

```powershell
ui-extractor extract --input screenshot.png `
  --models-dir ..\local-infer-core\crates\infer-core\tests\fixtures `
  --ocr-pack ocr.paddle.ppocr6-tiny.onnx.fp32 `
  --icon-index-pack icons.bundled.v1.mobileclip2-s0.int8
```

## 离线建索引

`PNG -> icons.bundled` 构建在 `local-infer-core`：

```powershell
cd ..\local-infer-core
powershell -ExecutionPolicy Bypass -File .\scripts\download_icons.ps1 -Rasterize
powershell -ExecutionPolicy Bypass -File .\tools\icon-index\build_bundled.ps1
```
