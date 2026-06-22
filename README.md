# ui-extractor

从 UI 截图提取元素树和文本坐标，输出 JSON，供纯文本模型理解界面布局。

当前实现：

- UI 树识别：灰度 → Canny 边缘 → 膨胀 → 形态学闭运算 → 轮廓层级（Suzuki-Abe）
- 文本识别：PaddleOCR PP-OCRv5 mobile（ONNX，via `oar-ocr`）
- 图标识别：MobileCLIP2-S0 向量嵌入 + MDI 余弦检索（未命中保留为容器；potrace 兜底尚未实现）。资源见 [docs/dev/mdi-icons.md](docs/dev/mdi-icons.md)

## 构建

```bash
cargo build --release
```

纯 Rust 依赖（`image` + `imageproc` + `ort`），无需安装 OpenCV。ONNX Runtime 由 `ort` / `oar-ocr` 的 `download-binaries` feature 自动拉取。

## 首次设置

仓库**不包含** ONNX 权重与 MDI 资源（见 `.gitignore`）。克隆后需本地生成，共四步：

```powershell
# 1. OCR 模型（~21 MB，来自 oar-ocr release）
powershell -ExecutionPolicy Bypass -File scripts/download_models.ps1

# 2. 图标嵌入模型（~46 MB，MobileCLIP2-S0 vision）
powershell -ExecutionPolicy Bypass -File scripts/download_mobileclip2.ps1

# 3. MDI 图标库：SVG + 栅格化 PNG（需 Node.js/npm；约 7400 图标）
powershell -ExecutionPolicy Bypass -File scripts/download_mdi_icons.ps1 -Rasterize

# 4. 预计算嵌入索引（Rust 本地生成，~2.5 min）
cargo run --release --bin embed-mdi
```

完成后 `models/` 与 `assets/mdi/` 应包含：

| 路径 | 来源 | 说明 |
|------|------|------|
| `models/pp-ocrv5_mobile_det.onnx` | 脚本下载 | OCR 检测 |
| `models/pp-ocrv5_mobile_rec.onnx` | 脚本下载 | OCR 识别 |
| `models/ppocrv5_dict.txt` | 脚本下载 | 字形表 |
| `models/mobileclip2-s0-vision.onnx` | 脚本下载 | 图标嵌入 |
| `assets/mdi/svg/`、`meta.json` | 脚本下载 | MDI 官方包 `@mdi/svg@7.4.47` |
| `assets/mdi/png-48-black/` | **本地生成** | `download_mdi_icons.ps1 -Rasterize` → `rasterize-mdi` |
| `assets/mdi/embeddings.bin` | **本地生成** | `embed-mdi`（无对应 PowerShell 脚本） |

> **注意：** 仅跑 `scripts/` 不足以复现全部资源——`embeddings.bin` 必须执行第 4 步。第 3 步必须带 `-Rasterize`，否则没有 PNG，`embed-mdi` 会失败。

仅布局树、不需要 OCR 时，可加 `--layout-only` 跳过 OCR 模型；图标识别缺资源时会 warning 并跳过，不影响布局与 OCR。

详细说明：[docs/dev/icon-matching.md](docs/dev/icon-matching.md)、[docs/dev/mdi-icons.md](docs/dev/mdi-icons.md)

## 用法

```bash
# 完整提取（布局 + OCR）
ui-extractor extract --input screenshot.png --format pretty

# 单张图 + 标注 PNG
ui-extractor extract --input screenshot.png -o out.json --annotate

# 批量处理 tests/cases 下所有用例（input.png/jpg/... → output.json + annotated.png）
ui-extractor cases

# 指定 cases 目录
ui-extractor cases --dir tests/cases
```

每个 case 目录约定（**提交** `input.*`、`output.json`、`annotated.png`；`pipeline/`、`timing.json`、`skeleton.html` 为本地生成，已 gitignore）：

```
tests/cases/<name>/
  input.png          # 或 input.jpg / input.webp 等（提交）
  output.json        # golden 输出（提交）
  annotated.png      # 标注图（提交；蓝=容器，绿=文本，橙=图标）
  pipeline/          # --dump-pipeline 或 cases 生成（不提交）
  skeleton.html      # cases 生成（不提交）
  timing.json        # cases 生成（不提交）
```

单张图也可手动导出管线中间图：

```bash
ui-extractor extract --input screenshot.png -o out.json --dump-pipeline
# 写入 out.json 同目录下的 pipeline/
```

## 输出示例

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

## 库 API

```rust
use ui_extractor::{extract_from_path, ExtractConfig};
use std::path::Path;

let result = extract_from_path(Path::new("screenshot.png"), &ExtractConfig::default())?;
println!("{}", serde_json::to_string_pretty(&result)?);
```

## 模块结构

| 模块 | 职责 |
|------|------|
| `layout/` | 预处理、轮廓检测、UI 树构建 |
| `ocr/` | PaddleOCR 推理与 `OcrWord` 适配 |
| `pipeline/` | 串联布局与 OCR，按坐标关联文本 |
| `types/` | `Bounds`、`UiElement`、JSON 结构 |
| `icon/` | MobileCLIP2 嵌入、索引检索、IoU rerank、`attach_icons` |

工具二进制：

| 二进制 | 用途 |
|--------|------|
| `ui-extractor` | 主 CLI：布局 + OCR + 图标识别 |
| `rasterize-mdi` | MDI SVG → PNG（也可单独调用） |
| `embed-mdi` | MDI PNG → `embeddings.bin` |

## 图标识别

默认开启。从布局树筛选**无子节点、近似正方形**的叶子容器，crop 后嵌入 [MobileCLIP2-S0](https://huggingface.co/apple/MobileCLIP2-S0)，与 `assets/mdi/embeddings.bin` 余弦检索；top-10 + IoU rerank。详见 [docs/dev/icon-matching.md](docs/dev/icon-matching.md)。

```powershell
# 跳过图标识别
ui-extractor extract --input screenshot.png --no-icon

# 指定库路径与最低 cosine（默认 0.72）
ui-extractor extract --input screenshot.png `
  --model-dir models `
  --mdi-dir assets/mdi/png-48-black `
  --embedding-index assets/mdi/embeddings.bin `
  --icon-model models/mobileclip2-s0-vision.onnx `
  --icon-min-cosine 0.72
```

OCR 模型目录可用 `--model-dir` 指定（默认 `models/`）。

## 后续计划

- [ ] potrace 兜底（未命中时生成 SVG path）
- [ ] 多 namespace 图标库（brand / custom）
- [ ] ncnn 端侧推理（手机）
- [ ] 可选 OpenCV 后端
