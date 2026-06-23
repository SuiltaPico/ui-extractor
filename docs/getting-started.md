# 桌面端快速开始

## 前置依赖

| 依赖 | 用途 |
|------|------|
| Rust stable + Cargo | 编译与运行 |
| Node.js + npm | 可选，仅 `download_mdi_icons.ps1` 拉取 MDI SVG |

Android / ncnn 相关见 [models.md](models.md) 与 [android.md](android.md)。

## 构建

```bash
cargo build --release
```

默认启用 `backend-ort`（ONNX Runtime）。首次编译时 `ort` / `oar-ocr` 会自动下载 ONNX Runtime 二进制。

## 首次设置（四步）

仓库**不包含**模型权重与图标资源。克隆后按顺序执行：

```powershell
# 1. OCR 模型（~21 MB）
powershell -ExecutionPolicy Bypass -File scripts/download_models.ps1

# 2. 图标嵌入模型 MobileCLIP2-S0（~46 MB）
powershell -ExecutionPolicy Bypass -File scripts/download_mobileclip2.ps1

# 3. 示例图标库：MDI SVG → PNG（需 Node.js；写入 assets/）
powershell -ExecutionPolicy Bypass -File scripts/download_mdi_icons.ps1 -Rasterize

# 4. 预计算嵌入索引（~7400 图标，CPU release 约 2–3 分钟）
cargo run --release -- icon build-embeddings
```

完成后目录应包含：

| 路径 | 说明 |
|------|------|
| `models/pp-ocrv5_mobile_det.onnx` | OCR 检测 |
| `models/pp-ocrv5_mobile_rec.onnx` | OCR 识别 |
| `models/ppocrv5_dict.txt` | 字形表 |
| `models/mobileclip2-s0-vision.onnx` | 图标嵌入 |
| `assets/svg/` | MDI SVG 源（可选） |
| `assets/icons/` | 模板 PNG（48×48，**仅离线建索引**） |
| `assets/embeddings.bin` | 预计算索引 |

第 3 步必须带 `-Rasterize`，否则没有 PNG，第 4 步会失败。也可自备任意 PNG 目录，跳过 MDI 脚本。

仅布局树、不需要 OCR 时：`ui-extractor extract --layout-only …`  
缺图标资源时会 warning 并跳过图标识别，不影响布局与 OCR。

## 基本用法

```bash
# 完整提取（布局 + OCR + 图标）
ui-extractor extract --input screenshot.png --format pretty

# 输出 JSON + 标注 PNG（蓝=容器，绿=文本，橙=图标）
ui-extractor extract --input screenshot.png -o out.json --annotate

# 跳过图标
ui-extractor extract --input screenshot.png --no-icon

# 批量回归 tests/cases
ui-extractor cases
ui-extractor cases --dir tests/cases
```

### Case 目录约定

```
tests/cases/<name>/
  input.png          # 输入（提交）
  output.json        # golden 输出（提交）
  annotated.png      # 标注图（提交）
  pipeline/          # --dump-pipeline 生成（gitignore）
  skeleton.html      # cases 生成（gitignore）
  timing.json        # cases 生成（gitignore）
```

## 输出 JSON 结构

```json
{
  "width": 1080,
  "height": 1920,
  "root": {
    "bounds": { "x": 0, "y": 0, "width": 1080, "height": 1920 },
    "kind": "root",
    "children": [
      {
        "bounds": { "x": 100, "y": 200, "width": 300, "height": 48 },
        "kind": "container",
        "children": [
          {
            "bounds": { "x": 110, "y": 212, "width": 24, "height": 24 },
            "kind": "icon",
            "name": "home",
            "confidence": 0.89
          }
        ]
      }
    ]
  }
}
```

## CLI 子命令

| 子命令 | 用途 |
|--------|------|
| `extract` | 单张截图：布局 + OCR + 图标 |
| `cases` | 批量处理 `tests/cases` |
| `icon build-embeddings` | PNG 目录 → `embeddings.bin` |
| `icon rasterize-svg` | SVG 批量栅格化为 PNG |
| `icon match` | 单图/区域匹配图标库 |
| `icon search` | 单图/区域 top-k 检索 |

## Rust 库 API

```rust
use ui_extractor::{extract_from_path, ExtractConfig};
use std::path::Path;

let result = extract_from_path(Path::new("screenshot.png"), &ExtractConfig::default())?;
println!("{}", serde_json::to_string_pretty(&result)?);
```

C ABI（Android / 其他语言）：见 [`include/ui_extractor.h`](../include/ui_extractor.h) 与 [android.md](android.md)。

## 图标识别参数

默认路径：`assets/embeddings.bin`、`models/mobileclip2-s0-vision.onnx`。

```powershell
ui-extractor extract --input screenshot.png `
  --embedding-index assets/embeddings.bin `
  --vision-model models/mobileclip2-s0-vision.onnx `
  --min-cosine 0.72
```

详见 [dev/icon-matching.md](dev/icon-matching.md)。
