# 图标识别（MobileCLIP2 向量检索）

图标识别在布局树中筛选候选区域，用 [MobileCLIP2-S0](https://huggingface.co/apple/MobileCLIP2-S0) 生成 512 维嵌入，与预计算索引做余弦检索；命中则输出 `kind: "icon"`，未命中保留 `container`。

MDI 资源下载与栅格化见 [mdi-icons.md](./mdi-icons.md)。

## 架构概览

```
布局树 leaf container（近似正方形、无文本子节点）
  → crop 截图区域
  → 预处理 → 256×256 RGB（白底黑 icon）
  → MobileCLIP2-S0 vision ONNX → 512 维嵌入（L2 归一化）
  → 与 embeddings.bin 暴力余弦检索（~7400 量级，毫秒级）
  → top-10 + mask IoU rerank
  → cosine ≥ min_cosine → { kind: "icon", name, confidence }
  → 否则保留 container（后续 potrace 兜底）
```

| 组件 | 路径 / 工具 | 说明 |
|------|-------------|------|
| Vision 模型 | `models/mobileclip2-s0-vision.onnx` | 来自 [plhery/mobileclip2-onnx](https://huggingface.co/plhery/mobileclip2-onnx) S0 |
| 嵌入索引 | `assets/mdi/embeddings.bin` | `embed-mdi` 离线生成 |
| MDI 模板 PNG | `assets/mdi/png-48-black/` | IoU rerank 用；与索引名称一一对应 |
| 离线建索引 | `cargo run --release --bin embed-mdi` | ~7400 图标 / ~2.5 分钟（CPU） |
| 下载模型 | `scripts/download_mobileclip2.ps1` | ~46MB |

## 模型规格

| 项 | 值 |
|----|-----|
| 模型 | MobileCLIP2-S0（vision encoder only） |
| 输入名 | `pixel_values` |
| 输入形状 | `[1, 3, 256, 256]` float32 |
| 输入数值 | RGB，像素 / 255 → `[0, 1]`；`mean=0, std=1`（不再额外归一化） |
| 输出名 | `image_embeds` |
| 输出形状 | `[1, 512]` |
| 检索前 | 对输出做 L2 归一化，余弦相似度 = 点积 |

桌面端通过 [`ort`](https://github.com/pykeio/ort) 加载 ONNX；与 OCR 共用 ONNX Runtime 二进制（`download-binaries` feature）。

## 预处理（关键）

**库内 PNG（`embed-mdi`）** 与 **截图 crop（运行时）** 最终都必须变成相同的 256×256 白底黑 icon RGB，但入口不同：

| 来源 | 入口函数 | 说明 |
|------|----------|------|
| MDI PNG（透明底） | `mdi_png_to_rgb256` | 按 alpha 合成到白底，再缩放到 256×256 |
| 截图灰度 crop | `icon_crop_to_rgb256` | 48×48 自适应二值 mask → 渲染为 RGB → 256×256 |

> 注意：MDI PNG 不能先 `to_gray()` 再阈值化——透明区域会变黑，导致所有图标嵌入相同。

## 匹配策略

默认配置（`IconConfig`）：

| 参数 | 默认 | 含义 |
|------|------|------|
| `min_cosine` | `0.72` | 低于此值视为未命中 |
| `rerank_top_k` | `10` | 嵌入检索 top-k |
| `min_iou` | `0.35` | rerank 时 mask IoU 下限 |
| `template_size` | `48` | mask 归一化边长 |

Rerank 接受条件（满足其一）：

- `cosine ≥ min_cosine` 且 `IoU ≥ min_iou`
- `cosine ≥ 0.85`（高置信嵌入，可忽略 IoU）

最终 `confidence` 为加权分：`0.7 × cosine + 0.3 × IoU`（仅 rerank 路径）；纯嵌入命中时等于 cosine。

## 嵌入索引格式（`embeddings.bin`）

```
magic     "MCL2" (4 bytes)
version   u32 = 1
dim       u32 = 512
count     u32
repeat count times:
  name_len  u16
  name      utf-8
vectors   count × dim × f32 (LE, 已 L2 归一化)
```

## CLI

```powershell
# 完整流水线依赖
powershell -ExecutionPolicy Bypass -File scripts/download_mdi_icons.ps1 -Rasterize
powershell -ExecutionPolicy Bypass -File scripts/download_mobileclip2.ps1
cargo run --release --bin embed-mdi

# 提取
cargo run --release --bin ui-extractor -- extract --input screenshot.png --annotate

# 调参
cargo run --release --bin ui-extractor -- extract --input screenshot.png `
  --mdi-dir assets/mdi/png-48-black `
  --embedding-index assets/mdi/embeddings.bin `
  --icon-model models/mobileclip2-s0-vision.onnx `
  --icon-min-cosine 0.72 `
  --no-icon   # 跳过图标识别
```

缺少 MDI 目录、索引或 ONNX 模型时打印 warning 并跳过图标识别，不影响布局与 OCR。

## 扩充图标库（规划）

当前仅内置 MDI（~7400）。向量检索的扩展成本低：**新库 = 新 PNG 目录 + 跑一遍 `embed-mdi`**，检索仍是 O(n) 点积，万级规模仍够快。

建议按 **namespace 分库**，而非单一巨大索引：

```
assets/
  mdi/          # 通用 Material Design 图标（已有）
  brand/        # 品牌 / 产品 logo（规划）
  custom/<app>/ # 特定产品私有库（规划）
```

原则：

1. **未命中优于乱命中** — 保持较高 `min_cosine`；可考虑 top1 − top2 margin 过小时不命名
2. **预处理一致** — 所有库统一白底黑 icon、256×256、同一 embedder
3. **数据驱动补库** — 从未命中的 leaf container 反推需要补充的图标
4. **分库检索** — 各库独立索引，检索后按 namespace 输出（如 `mdi:home`、`brand:google`）

## 部署分工（规划）

| 平台 | 推理后端 | 说明 |
|------|----------|------|
| 桌面 / CI | ONNX Runtime（`ort`） | 当前实现 |
| 手机 / 嵌入式 | ncnn | 共享 `embeddings.bin` 与预处理；需单独导出 MobileCLIP2-S0 |

## 相关源码

| 路径 | 职责 |
|------|------|
| `src/icon/preprocess.rs` | RGB 渲染、NCHW 张量 |
| `src/icon/embedding.rs` | ONNX 推理、`EmbeddingIndex` 读写 |
| `src/icon/library.rs` | MDI mask 加载、cosine + IoU 匹配 |
| `src/icon/mod.rs` | 候选筛选、`attach_icons` |
| `src/bin/embed_mdi.rs` | 离线建索引 |

## 后续

- [ ] potrace 兜底（未命中时生成 SVG path）
- [ ] 多 namespace 索引与检索
- [ ] ncnn 端侧推理
- [ ] margin / ambiguous 拒绝策略
