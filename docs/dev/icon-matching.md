# 图标识别（MobileCLIP2 向量检索）

图标识别在布局树中筛选候选区域，用 [MobileCLIP2-S0](https://huggingface.co/apple/MobileCLIP2-S0) 生成 512 维嵌入，与预计算索引做余弦检索；命中则输出 `kind: "icon"`，未命中保留 `container`。

MDI 资源下载与栅格化见 [mdi-icons.md](./mdi-icons.md)。

## 架构概览

```
布局树 leaf container（近似正方形、无文本子节点）
  → crop 截图区域
  → 预处理 → 256×256 RGB（白底黑 icon）
  → MobileCLIP2-S0 vision → 512 维嵌入（L2 归一化）
  → 与 embeddings.bin 暴力余弦检索（~7400 量级，毫秒级）
  → cosine ≥ min_cosine → { kind: "icon", name, confidence }
  → 否则保留 container（后续 potrace 兜底）
```

| 组件 | 路径 / 工具 | 说明 |
|------|-------------|------|
| Vision 模型（桌面） | `models/mobileclip2-s0-vision.onnx` | ONNX Runtime |
| Vision 模型（Android） | `models/mobileclip2-s0-vision.ncnn.param` + `.bin` | ncnn |
| 嵌入索引 | `assets/embeddings.bin` | `icon build-embeddings` 离线生成 |
| 模板 PNG | `assets/icons/` | **仅离线建索引**；运行时不需要 |
| 下载模型 | `scripts/download_mobileclip2.ps1` | ~46 MB |

## 模型规格

| 项 | 值 |
|----|-----|
| 模型 | MobileCLIP2-S0（vision encoder only） |
| 输入名 | `pixel_values` |
| 输入形状 | `[1, 3, 256, 256]` float32 |
| 输入数值 | RGB，像素 / 255 → `[0, 1]`；`mean=0, std=1` |
| 输出名 | `image_embeds` |
| 输出形状 | `[1, 512]` |
| 检索前 | L2 归一化，余弦相似度 = 点积 |

桌面端通过 [`ort`](https://github.com/pykeio/ort) 加载 ONNX；Android 通过 `embedder_ncnn.rs` 加载 ncnn。索引 `embeddings.bin` 两种后端共用。

## 预处理（关键）

**库内 PNG（建索引）** 与 **截图 crop（运行时）** 必须变成相同的 256×256 白底黑 icon RGB：

| 来源 | 入口函数 | 说明 |
|------|----------|------|
| 模板 PNG（透明底） | `template_png_to_rgb256` | alpha 合成白底 → 256×256 |
| 截图灰度 crop | `icon_crop_to_rgb256` | 自适应二值 mask → RGB → 256×256 |

> MDI PNG 不能先 `to_gray()` 再阈值化——透明区域会变黑，导致嵌入相同。

## 匹配策略

默认配置（`IconConfig`）：

| 参数 | 默认 | 含义 |
|------|------|------|
| `min_cosine` | `0.72` | 低于此值视为未命中 |
| `template_size` | `48` | 截图 crop 归一化边长（预处理用） |

最终 `confidence` 等于最佳余弦相似度。

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
# 准备（PNG 仅用于离线建索引）
powershell -ExecutionPolicy Bypass -File scripts/download_mdi_icons.ps1 -Rasterize
powershell -ExecutionPolicy Bypass -File scripts/download_mobileclip2.ps1
cargo run --release -- icon build-embeddings

# 提取（运行时只需 embeddings.bin + vision 模型）
cargo run --release -- extract --input screenshot.png --annotate

# 调参
cargo run --release -- extract --input screenshot.png `
  --embedding-index assets/embeddings.bin `
  --vision-model models/mobileclip2-s0-vision.onnx `
  --min-cosine 0.72 `
  --no-icon
```

缺少索引或 vision 模型时 warning 并跳过图标识别。

## 扩充图标库（规划）

向量检索扩展成本低：**新库 = 新 PNG 目录 + 跑一遍 `icon build-embeddings`**。

建议按 **namespace 分库**：

```
assets/
  icons/        # 当前默认 MDI 模板
  brand/        # 品牌 logo（规划）
  custom/<app>/ # 产品私有库（规划）
```

原则：

1. **未命中优于乱命中** — 保持较高 `min_cosine`；可考虑 top1 − top2 margin
2. **预处理一致** — 统一白底黑 icon、256×256、同一 embedder
3. **分库检索** — 各库独立索引，输出 namespace（如 `mdi:home`）

## 相关源码

| 路径 | 职责 |
|------|------|
| `src/icon/preprocess.rs` | RGB 渲染、query mask、NCHW 张量 |
| `src/icon/embedding.rs` | 索引读写、余弦检索 |
| `src/icon/embedder_ort.rs` | ONNX 推理 |
| `src/icon/embedder_ncnn.rs` | ncnn 推理 |
| `src/icon/library.rs` | 加载 `embeddings.bin`、best match |
| `src/icon/mod.rs` | 候选筛选、`attach_icons` |
| `src/icon/embed.rs` | 离线建索引 |

## 后续

- [ ] potrace 兜底
- [ ] 多 namespace 索引
- [x] ncnn 端侧推理
- [ ] margin / ambiguous 拒绝策略
