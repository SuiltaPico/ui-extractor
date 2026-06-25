# 图标识别（infer-core pack 检索）

图标识别在布局树中筛选候选区域，调用 `local-infer-core` 的 embed + icon_index 能力做余弦检索；命中输出 `kind: "icon"`，未命中保留 `container`。

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
| embed 模型包 | `embed.mobileclip2-s0.*` | 由 infer-core 按 manifest 加载 |
| 图标索引包 | `icons.bundled.v1.mobileclip2-s0.*` | 由 infer-core 按 manifest 加载 |
| 模板 PNG | `assets/icons/` | **仅离线建索引**；运行时不需要 |

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

具体后端（ORT/MNN）由 infer-core 在运行时决定，`ui-extractor` 不再关心底层引擎实现。

## 预处理（关键）

**库内 PNG（建索引）** 与 **截图 crop（运行时）** 必须变成相同的 256×256 白底黑 icon RGB：

| 来源 | 入口函数 | 说明 |
|------|----------|------|
| 模板 PNG（透明底） | infer-core 离线工具 | alpha 合成白底 → 256×256 |
| 截图灰度 crop | `icon_crop_to_rgb256` | 自适应二值 mask → RGB → 256×256 |

> MDI PNG 不能先 `to_gray()` 再阈值化——透明区域会变黑，导致嵌入相同。

## 匹配策略

默认配置（`IconConfig`）：

| 参数 | 默认 | 含义 |
|------|------|------|
| `min_cosine` | `0.72` | 低于此值视为未命中 |
| `template_size` | `48` | 截图 crop 归一化边长（预处理用） |

最终 `confidence` 等于最佳余弦相似度。

索引格式与 pack 结构见 `local-infer-core/PRODUCT.md`（`icon_index` 部分）。

## CLI

```powershell
# 准备 pack（在 local-infer-core 仓库）
powershell -ExecutionPolicy Bypass -File ..\local-infer-core\scripts\download_all_packs.ps1

# 提取
cargo run --release -- extract --input screenshot.png `
  --models-dir ..\local-infer-core\crates\infer-core\tests\fixtures `
  --icon-index-pack icons.bundled.v1.mobileclip2-s0.int8 `
  --min-cosine 0.72
```

缺少索引或 vision 模型时 warning 并跳过图标识别。

## 扩充图标库（规划）

向量检索扩展成本低：**新库 = 新 PNG 目录 + infer-core 工具重建 icon_index 包**。

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
| `src/icon/embedding.rs` | 本地索引读写结构（兼容旧工具链） |
| `src/icon/pack.rs` | 调用 infer-core 做匹配 |
| `src/icon/mod.rs` | 候选筛选、`attach_icons_with_pack` |

## 后续

- [ ] potrace 兜底
- [ ] 多 namespace 索引
- [x] 推理迁移到 local-infer-core
- [ ] margin / ambiguous 拒绝策略
