# MDI 图标库

[Material Design Icons](https://pictogrammers.com/library/mdi/)（MDI）本地资源，供**离线**建 `embeddings.bin` 使用（运行时只需索引文件）。
当前推荐流程：用 MDI 生成 PNG 素材后，调用 `local-infer-core` 的 `icon-index-build` 生成最终索引包。

图标匹配架构见 [icon-matching.md](./icon-matching.md)。

## 目录结构

下载、栅格化并建索引后：

```
assets/
  svg/                  # ~7400+ 官方 SVG（@mdi/svg）
  meta.json             # 图标元数据（name、codepoint、tags）
  icons/                # 栅格化 PNG（48×48，黑色，透明底）
  embeddings.bin        # MobileCLIP2 预计算嵌入
```

以上路径均在 `.gitignore` 中，需本地生成。

`vision` 模型建议从 `local-infer-core` 的 embed pack 获取。

## 一键准备

```powershell
# 1. MDI SVG + PNG
powershell -ExecutionPolicy Bypass -File scripts/download_mdi_icons.ps1 -Rasterize

# 2. 预计算嵌入索引（在 local-infer-core 仓库）
cargo run -p infer-core --bin icon-index-build -- `
  --png-dir assets/icons `
  --vision-model <path-to-embed-pack-vision-model> `
  --out assets/embeddings.bin
```

第 2 步约 7400 图标 / 2–3 分钟（CPU release）。PNG 有增删时需重新运行。

## 前置依赖

| 依赖 | 用途 |
|------|------|
| Node.js + npm | `download_mdi_icons.ps1` 拉取 `@mdi/svg` |
| Rust toolchain | `ui-extractor`（栅格化）+ `local-infer-core`（建索引） |

## 下载 + 栅格化

```powershell
powershell -ExecutionPolicy Bypass -File scripts/download_mdi_icons.ps1 -Rasterize
```

默认输出：

- SVG → `assets/svg/`
- 元数据 → `assets/meta.json`
- PNG → `assets/icons/`（48px，黑色）

常用参数：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/download_mdi_icons.ps1 `
  -Rasterize -Version 7.4.47 -Size 48 -Color black -Jobs 8
```

脚本行为：

1. 在 `%TEMP%/ui-extractor-mdi-<version>/` 用 npm 安装 `@mdi/svg`
2. 复制 `svg/` 和 `meta.json` 到 `assets/`
3. 若带 `-Rasterize`：调用 `ui-extractor icon rasterize-svg`

## icon rasterize-svg

```powershell
cargo run --release -- icon rasterize-svg -- `
  --svg-dir assets/svg `
  --out-dir assets/icons `
  --size 48 `
  --color black `
  --jobs 8
```

| 参数 | 说明 | 默认 |
|------|------|------|
| `--svg-dir` | 输入 SVG 目录 | `assets/svg` |
| `--out-dir` | 输出 PNG 目录 | `assets/icons` |
| `--size` | 输出边长（px） | `48` |
| `--color` | `black` / `white` | `black` |
| `--jobs` | 并行线程数 | CPU 核心数 |
| `--skip-existing` | 跳过已存在 PNG | 否 |

参考性能（release，8 线程）：~7400 图标 / 3 秒。

## icon-index-build（infer-core）

```powershell
cargo run -p infer-core --bin icon-index-build -- `
  --png-dir assets/icons `
  --vision-model <path-to-embed-pack-vision-model> `
  --out assets/embeddings.bin
```

| 参数 | 说明 | 默认 |
|------|------|------|
| `--png-dir` | 输入 PNG 目录 | 无 |
| `--out` | 输出索引 | 无 |
| `--vision-model` | vision 模型路径 | 无 |
| `--template-size` | 中间 mask 尺寸 | `48` |

模板 PNG 使用 **RGBA alpha 合成白底**（`mdi_png_to_rgb256`），再缩放到 256×256。索引约 15 MB。

Android / MNN 或桌面 / ONNX 由 infer-core runtime 配置决定，命令入口一致。

## meta.json

JSON 数组，每项包含：

- `name` — 图标名，对应 `svg/<name>.svg` 和 `icons/<name>.png`
- `codepoint` — Unicode 码点
- `tags` / `aliases` — 分类与别名

命中后 `name` 写入 UI 树 `icon` 节点。

## 扩充其他图标库

MDI 仅覆盖 Material Design 风格。后续可按 namespace 增加目录，对每个目录：

1. 准备统一规格 PNG（透明底、单色 icon）
2. 用 `icon-index-build` 生成独立索引文件
3. 检索层按库分别匹配（见 [icon-matching.md](./icon-matching.md)）

## 数据来源

[@mdi/svg](https://www.npmjs.com/package/@mdi/svg)（[MaterialDesign-SVG](https://github.com/Templarian/MaterialDesign-SVG)）。无官方批量 PNG，由本仓库 `icon rasterize-svg` 本地生成。
