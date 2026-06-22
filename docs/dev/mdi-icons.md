# MDI 图标库

[Material Design Icons](https://pictogrammers.com/library/mdi/)（MDI）本地资源，供图标向量检索与 IoU rerank 使用。

图标匹配架构见 [icon-matching.md](./icon-matching.md)。

## 目录结构

下载、栅格化并建索引后，`assets/mdi/` 大致如下：

```
assets/mdi/
  svg/                  # 约 7400+ 个官方 SVG（来自 @mdi/svg）
  meta.json             # 图标元数据（name、codepoint、tags 等）
  png-48-black/         # 栅格化 PNG（48×48，黑色图标，透明底）
  png-48-white/         # 可选：白色图标（深色 UI 背景）
  embeddings.bin        # MobileCLIP2-S0 预计算嵌入（embed-mdi 生成）
```

`assets/mdi/` 已在 `.gitignore` 中，需本地生成，不提交仓库。

`models/mobileclip2-s0-vision.onnx` 由 `scripts/download_mobileclip2.ps1` 下载，同样不提交。

## 一键准备（推荐顺序）

```powershell
# 1. MDI SVG + PNG
powershell -ExecutionPolicy Bypass -File scripts/download_mdi_icons.ps1 -Rasterize

# 2. MobileCLIP2-S0 vision ONNX
powershell -ExecutionPolicy Bypass -File scripts/download_mobileclip2.ps1

# 3. 预计算嵌入索引
cargo run --release --bin embed-mdi
```

第 3 步约 7400 图标 / ~2.5 分钟（CPU，release）。PNG 有增删时需重新运行。

## 前置依赖

| 依赖 | 用途 |
|------|------|
| Node.js + npm | `download_mdi_icons.ps1` 拉取 `@mdi/svg` |
| Rust toolchain | `rasterize-mdi`、`embed-mdi`、`ui-extractor` |

## 下载 + 栅格化 MDI

```powershell
powershell -ExecutionPolicy Bypass -File scripts/download_mdi_icons.ps1 -Rasterize
```

默认输出：

- SVG → `assets/mdi/svg/`
- 元数据 → `assets/mdi/meta.json`
- PNG → `assets/mdi/png-48-black/`（48px，黑色）

常用参数：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/download_mdi_icons.ps1 `
  -Rasterize -Version 7.4.47 -Size 48 -Color black -Jobs 8
```

脚本行为：

1. 在 `%TEMP%/ui-extractor-mdi-<version>/` 用 npm 安装 `@mdi/svg`
2. 复制 `svg/` 和 `meta.json` 到 `assets/mdi/`
3. 若带 `-Rasterize`：调用 `rasterize-mdi`（不存在时自动 `cargo build --release --bin rasterize-mdi`）

## rasterize-mdi

```powershell
cargo run --release --bin rasterize-mdi -- `
  --svg-dir assets/mdi/svg `
  --out-dir assets/mdi/png-48-black `
  --size 48 `
  --color black `
  --jobs 8
```

| 参数 | 说明 | 默认 |
|------|------|------|
| `--svg-dir` | 输入 SVG 目录 | `assets/mdi/svg` |
| `--out-dir` | 输出 PNG 目录 | `assets/mdi/png-48-black` |
| `--size` | 输出边长（px） | `48` |
| `--color` | `black` / `white` | `black` |
| `--jobs` | 并行线程数 | CPU 核心数 |
| `--skip-existing` | 跳过已存在的 PNG | 否 |

参考性能（release，8 线程）：约 7400 图标 / 3 秒。

## embed-mdi（嵌入索引）

```powershell
cargo run --release --bin embed-mdi
```

| 参数 | 说明 | 默认 |
|------|------|------|
| `--png-dir` | 输入 PNG 目录 | `assets/mdi/png-48-black` |
| `--out` | 输出索引 | `assets/mdi/embeddings.bin` |
| `--model` | vision ONNX | `models/mobileclip2-s0-vision.onnx` |
| `--template-size` | 中间 mask 尺寸 | `48` |

MDI PNG 使用 **RGBA alpha 合成白底**（`mdi_png_to_rgb256`），再缩放到 256×256 送入模型。索引文件约 15MB。

## meta.json

JSON 数组，每项包含：

- `name` — 图标名，对应 `svg/<name>.svg` 和 `png/<name>.png`
- `codepoint` — Unicode 码点
- `tags` / `aliases` — 分类与别名

命中后 `name` 写入 UI 树 `icon` 节点。

## 扩充其他图标库

MDI 仅覆盖 Material Design 风格。后续可按 namespace 增加目录（如 `assets/brand/`），对每个目录：

1. 准备统一规格的 PNG（透明底、单色 icon）
2. 用同一 `embed-mdi` 流程生成独立 `embeddings.bin`
3. 检索层按库分别匹配或合并（见 [icon-matching.md](./icon-matching.md)）

## 数据来源

[@mdi/svg](https://www.npmjs.com/package/@mdi/svg)（[MaterialDesign-SVG](https://github.com/Templarian/MaterialDesign-SVG)）。无官方批量 PNG，由本仓库 `rasterize-mdi` 本地生成。
