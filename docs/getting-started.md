# 快速开始

## 前置依赖

| 依赖 | 用途 |
|------|------|
| Rust stable + Cargo | 编译与运行 |
| `local-infer-core` 模型目录 | OCR / embed / icon_index pack 来源 |
| `infer_core.dll` / `libinfer_core.so` | `ui-extractor` 运行时推理动态库 |

`ui-extractor` 不再负责模型下载与离线建索引；这些工作已迁移到 `local-infer-core`。

## 构建

```bash
cargo build --release
```

运行时推理由 `infer_core.dll` / `libinfer_core.so` 提供，`ui-extractor` 仅负责编排与 UI 树生成。

## 首次可用链路（Windows）

```powershell
# A. 构建 infer_core.dll
cd ..\local-infer-core
cargo build -p infer-core-ffi

# B. 回到 ui-extractor，安装模型包到 ./models
cd ..\ui-extractor
powershell -ExecutionPolicy Bypass -File .\scripts\install_packs.ps1 -Platform windows

# C. 复制动态库到 ui-extractor 二进制目录（避免 STATUS_DLL_NOT_FOUND）
Copy-Item -Force ..\local-infer-core\target\debug\infer_core.dll .\target\debug\infer_core.dll
```

## 首次设置

先准备一个符合 manifest 目录布局的 `models_dir`（推荐直接使用 `local-infer-core` 的 pack 下载脚本）：

```powershell
powershell -ExecutionPolicy Bypass -File ..\local-infer-core\scripts\download_all_packs.ps1
```

目录示例：

| 路径 | 说明 |
|------|------|
| `{models_dir}/ocr.paddle.ppocr6-tiny.onnx.fp32/` | OCR pack |
| `{models_dir}/embed.mobileclip2-s0.onnx.fp32/` | embed pack |
| `{models_dir}/icons.bundled.v1.mobileclip2-s0.int8/` | icon_index pack |

仅布局树、不需要 OCR 时：`ui-extractor extract --layout-only ...`  
缺某个 pack 时会在对应阶段报错（例如 OCR 或 icon 匹配），布局阶段仍可独立工作。

## 基本用法

```bash
# 完整提取（布局 + OCR + 图标）
cargo run --bin ui-extractor -- extract --input screenshot.png --format pretty `
  --models-dir ..\local-infer-core\crates\infer-core\tests\fixtures `
  --ocr-pack ocr.paddle.ppocr6-tiny.onnx.fp32 `
  --icon-index-pack icons.bundled.v1.mobileclip2-s0.int8

# 输出 JSON + 标注 PNG（蓝=容器，绿=文本，橙=图标）
cargo run --bin ui-extractor -- extract --input screenshot.png -o out.json --annotate

# 跳过图标
cargo run --bin ui-extractor -- extract --input screenshot.png --no-icon

# 批量回归 tests/cases
powershell -ExecutionPolicy Bypass -File .\scripts\test_cases.ps1
cargo run --bin ui-extractor -- cases --dir tests/cases
```

### Case 目录约定

```
tests/cases/<name>/
  input.png          # 原始输入（建议提交；历史 case 可能缺失）
  output.json        # golden 输出（提交）
  annotated.png      # 标注图（提交，可作为回归输入）
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

图标匹配依赖 `icon_index_pack` 与其 `embed_model_id`（由 `manifest.json` 关联）。

```powershell
cargo run --bin ui-extractor -- extract --input screenshot.png `
  --models-dir ..\local-infer-core\crates\infer-core\tests\fixtures `
  --icon-index-pack icons.bundled.v1.mobileclip2-s0.int8 `
  --min-cosine 0.72
```

离线“PNG -> embeddings.bin”构建入口已迁移到 `local-infer-core`（`icon-index-build`）。
