# 架构概览

服务于边缘计算场景：从 UI 截图提取**元素坐标**、**文本内容**与**图标名称**，输出 JSON，供纯文本 LLM 理解界面。  
当前架构中，神经网络推理能力已下沉到 `local-infer-core`，`ui-extractor` 只保留布局与编排层。

## 流水线

```
输入截图 (PNG/JPEG/WebP)
    │
    ├─ layout/     灰度 → Canny → 膨胀 → 闭运算 → Suzuki-Abe 轮廓
    │              → 轴对齐 UI 树（container / root）
    │
    ├─ infer/      调用 infer_core.dll / libinfer_core.so
    │              → OCR / embed / icon_index（manifest pack）
    │              → OcrWord 列表（文本 + bounds + 置信度）
    │
    ├─ pipeline/   按坐标将 OCR 词挂到布局树叶子
    │              → 带 text 字段的 UiElement
    │
    └─ icon/       筛选正方形 leaf container → crop
                   → infer-core embed + icon_index 余弦检索
                   → kind: "icon" 或保留 container
    │
    ▼
输出 JSON (+ 可选 annotated.png)
```

## 模块职责

| 模块 | 路径 | 职责 |
|------|------|------|
| 布局 | `src/layout/` | 预处理、轮廓检测、UI 树构建 |
| 推理桥接 | `src/infer/` | infer-core FFI 封装、runtime config、manifest 结构 |
| OCR 适配 | `src/ocr/` | 对 infer-core OCR 输出做 ui-extractor 结构转换 |
| 流水线 | `src/pipeline/` | 串联布局与 OCR，坐标关联 |
| 类型 | `src/types/` | `Bounds`、`UiElement`、JSON 序列化 |
| 图标 | `src/icon/` | 候选筛选、区域裁剪、调用 infer-core 检索 |
| 引擎 | `src/engine.rs` | 可复用提取引擎（CLI + FFI 共用） |
| FFI | `src/ffi.rs` | C ABI，`cdylib` 导出 |
| 标注 | `src/annotate.rs` | 可视化 bounding box |

## UI 树识别（layout）

1. 转灰度
2. Canny 边缘检测
3. 形态学膨胀 + 闭运算，连接断裂边缘
4. `find_contours_hierarchy`（Suzuki-Abe）提取轮廓与父子关系
5. 过滤过小区域，构建轴对齐 `UiElement` 树

可调参数见 `LayoutConfig`（如 `min_area`）。

## 文本识别（OCR）

- 由 infer-core 根据 `ocr_pack`（manifest）加载对应 OCR 引擎
- 输出文本框 polygon → 转 axis-aligned bounds
- 输出 UTF-8 字符串 + 置信度
- 与布局树按 IoU / 包含关系关联

## 图标识别（icon）

1. 从布局树选候选：无子节点、近似正方形、边长在合理范围
2. crop 截图区域，预处理为 **256×256 白底黑 icon RGB**
3. 调用 infer-core embed + icon_index 进行 512 维余弦检索
4. 与 `icon_index_pack` 的向量库匹配
5. `cosine ≥ min_cosine` 则 `kind: "icon"`
6. 未命中保留 `container`（规划：potrace SVG 兜底）

详见 [dev/icon-matching.md](dev/icon-matching.md)。

## 输出类型

```rust
UiElement {
    bounds: Bounds,           // x, y, width, height
    kind: "root" | "container" | "text" | "icon",
    text: Option<String>,     // OCR
    name: Option<String>,     // 图标名
    confidence: Option<f64>,
    children: Vec<UiElement>,
}
```

## 测试用例

`tests/cases/<name>/` 存放输入截图、golden `output.json` 与 `annotated.png`。  
`ui-extractor cases` 批量回归并生成 `timing.json`、`skeleton.html`（本地 gitignore）。

## 后续规划

- [ ] potrace 兜底（未命中图标时输出 SVG path）
- [ ] 多 namespace 图标库（brand / custom）
- [x] 推理逻辑迁移到 local-infer-core
- [ ] 可选 OpenCV 布局后端
