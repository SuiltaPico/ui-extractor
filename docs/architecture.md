# 架构概览

服务于边缘计算场景：从 UI 截图提取**所有可见元素的坐标**、**文本内容**与**图标名称**，输出 JSON，供纯文本 LLM 理解界面。

## 流水线

```
输入截图 (PNG/JPEG/WebP)
    │
    ├─ layout/     灰度 → Canny → 膨胀 → 闭运算 → Suzuki-Abe 轮廓
    │              → 轴对齐 UI 树（container / root）
    │
    ├─ ocr/        PP-OCRv5 mobile 检测 + 识别
    │              → OcrWord 列表（文本 + bounds + 置信度）
    │
    ├─ pipeline/   按坐标将 OCR 词挂到布局树叶子
    │              → 带 text 字段的 UiElement
    │
    └─ icon/       筛选正方形 leaf container → crop
                   → MobileCLIP2-S0 嵌入 → embeddings.bin 余弦检索
                   → kind: "icon" 或保留 container
    │
    ▼
输出 JSON (+ 可选 annotated.png)
```

## 模块职责

| 模块 | 路径 | 职责 |
|------|------|------|
| 布局 | `src/layout/` | 预处理、轮廓检测、UI 树构建 |
| OCR | `src/ocr/` | PaddleOCR 推理；`ort.rs`（桌面）/ `ncnn.rs`（Android） |
| 流水线 | `src/pipeline/` | 串联布局与 OCR，坐标关联 |
| 类型 | `src/types/` | `Bounds`、`UiElement`、JSON 序列化 |
| 图标 | `src/icon/` | 预处理、嵌入、索引、`attach_icons` |
| 推理抽象 | `src/inference/` | ncnn 封装（Android feature） |
| 引擎 | `src/engine.rs` | 可复用提取引擎（CLI + FFI 共用） |
| FFI | `src/ffi.rs` | C ABI，`cdylib` 导出 |
| 标注 | `src/annotate.rs` | 可视化 bounding box |

## 推理后端

通过 Cargo feature 二选一编译：

| Feature | OCR | 图标嵌入 | 依赖 |
|---------|-----|----------|------|
| `backend-ort` | `oar-ocr` + ONNX | `embedder_ort.rs` | `ort`, `oar-ocr` |
| `backend-ncnn` | `ocr/ncnn.rs` | `embedder_ncnn.rs` | `ncnn-bind`, 预编译 `libncnn` |

布局与图标检索逻辑与后端无关；仅模型文件格式不同（`.onnx` vs `.ncnn.param`/`.bin`）。

## UI 树识别（layout）

1. 转灰度
2. Canny 边缘检测
3. 形态学膨胀 + 闭运算，连接断裂边缘
4. `find_contours_hierarchy`（Suzuki-Abe）提取轮廓与父子关系
5. 过滤过小区域，构建轴对齐 `UiElement` 树

可调参数见 `LayoutConfig`（如 `min_area`）。

## 文本识别（OCR）

- 模型：PaddleOCR **PP-OCRv5 mobile**（检测 + 识别）
- 检测输出文本框 polygon → 转 axis-aligned bounds
- 识别输出 UTF-8 字符串 + 置信度
- 与布局树按 IoU / 包含关系关联

## 图标识别（icon）

1. 从布局树选候选：无子节点、近似正方形、边长在合理范围
2. crop 截图区域，预处理为 **256×256 白底黑 icon RGB**
3. MobileCLIP2-S0 vision → 512 维 L2 归一化嵌入
4. 与 `embeddings.bin` 暴力余弦检索（~7400 量级）
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
- [x] ncnn 端侧推理（Android）
- [ ] 可选 OpenCV 布局后端
