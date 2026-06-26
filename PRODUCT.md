# ui-extractor — 产品目标

**ui-extractor** 从 UI 截图快速提取**元素树、文本坐标、图标名称**，输出 JSON（及可选标注图），供纯文本 LLM 理解界面。它是 **CLI / 动态库** 项目，**不包含**神经网络实现 — 推理全部交给 [local-infer-core](../local-infer-core/docs/dev/PRODUCT.md)。

相关仓库：

- [local-infer-core](../local-infer-core/docs/dev/PRODUCT.md) — OCR、嵌入、模型包
- [Mauchat](../mauchat/PRODUCT.md) — UI 自动化消费方

---

## 定位

| 做什么 | 不做什么 |
|--------|----------|
| 布局：灰度 → Canny → 轮廓 → UI 树 | 模型训练、量化、Release |
| Pipeline：OCR 词挂树、图标候选匹配 | NCNN（已弃用，移动端走 MNN via infer-core） |
| CLI、`ui_extractor.dll`、Dart FFI 包 | 本地 OCR 转写（Mauchat 直接用 infer-core） |
| Golden 测试、annotated.png、skeleton | 云端视觉 API |

一句话：**轻量 UI 语义层** — 算法 + 编排；模型即插即用。

---

## 与 local-infer-core 的分工

```
截图
  │
  ├─ layout/          ui-extractor（纯算法，零 ML）
  │
  ├─ ocr/             local-infer-core（按 manifest 加载 OCR 包）
  │
  ├─ icon match/      local-infer-core（embed 包 + icon_index 包）
  │
  └─ pipeline/        ui-extractor（坐标关联、JSON 序列化）
```

ui-extractor **运行时依赖** `infer_core.dll`（编译期链 `infer_core.lib`）；Release 打包含同版本 `infer_core.dll` 供 Mauchat 等成对部署。

---

## 输出

```json
{
  "root": {
    "bounds": { "x": 0, "y": 0, "width": 1080, "height": 2400 },
    "kind": "root",
    "children": [
      { "kind": "text", "text": "设置", "bounds": { … } },
      { "kind": "icon", "name": "mdi:cog", "confidence": 0.89, "bounds": { … } },
      { "kind": "container", "children": [ … ] }
    ]
  }
}
```

`kind`：`root` | `container` | `text` | `icon`

---

## 配置（目标形态）

不再硬编码 `pp-ocrv5_mobile_det.onnx` 等路径；改为包 id + 模型根目录：

```json
{
  "run_ocr": true,
  "run_icon": true,
  "models_dir": "/path/to/models",
  "ocr_pack": "ocr.paddle.ppocr6-tiny.onnx.fp32",
  "embed_pack": "embed.mobileclip2-s0.onnx.fp32",
  "icon_index_pack": "icons.bundled.v1.mobileclip2-s0.int8",
  "layout": { "min_area": 100 }
}
```

CLI 等价：

```bash
ui-extractor extract --input screenshot.png \
  --models-dir ./models \
  --ocr-pack ocr.paddle.ppocr6-tiny.onnx.fp32 \
  --annotate
```

---

## 模型与图标库：三种来源

1. **官方包** — 从 [local-infer-core Releases](../local-infer-core/docs/dev/PRODUCT.md#官方-releasegithub-releases) 下载 zip，解压到 `{models_dir}/{pack_id}/`
2. **自备包** — 符合 manifest v1 的目录，放入 `{models_dir}/` 即可
3. **自定义图标库** — 上传 PNG + 用 infer-core 工具离线建 `icon_index` 包（manifest 声明 `embed_model_id`）；与官方 `icons.bundled.v1.*` 并列，不替代其 Release 地位

文档（getting-started）需明确：

- 默认相对路径与 Release zip 布局
- 环境变量 `LOCAL_INFER_ROOT`
- 换 OCR / 嵌入 / 图标库只需改 pack id，无需改代码

---

## 交付物

| 产物 | 用途 |
|------|------|
| `ui-extractor` CLI | 开发、CI golden、脚本 |
| `ui_extractor.dll` / `.so` | Mauchat UI 自动化（与 `infer_core` 成对部署） |
| `dart/` pub 包 **`ui_extractor`** | Flutter FFI；**依赖** `local_infer_core` dart 包 |
| `tests/cases/` | 回归（input.jpg、output.json、annotated.png） |

Native hook **只下载 `ui_extractor` 动态库**；模型目录取自 `local_infer_core` 的 `modelsDir`（见 [local-infer-core `dart/`](../local-infer-core/dart/)）。

---

## 从现状迁移

| 现状 | 目标 |
|------|------|
| 内置 `ocr/`、`icon/embedder_*` | 迁入 local-infer-core |
| `backend-ort` / `backend-ncnn` 编译期二选一 | 仅依赖 infer-core；移动端 MNN 由 infer-core 负责 |
| 硬编码 PP-OCRv5 文件名 | manifest 包 id |
| `assets/embeddings.bin` 固定路径 | `icon_index` 模型包 |

保留：`layout/`、`pipeline/`、`annotate/`、`engine.rs` 编排、`ffi.rs` 导出。

---

## Mauchat 中的角色

- **浏览器 / App UI 自动化**：截图 → `ui_extractor` → JSON → LLM 理解界面 → `click_at` 等
- **不用于**简单「图片转文字」聊天转写（走 infer-core OCR plain_text）

详见 [Mauchat PRODUCT.md](../mauchat/PRODUCT.md)。

---

## 非目标

- 端到端 UI Agent（点击规划、任务编排 — 属 Mauchat）
- OmniParser / 大 VLM 替代方案
- 在线模型下载 UI（属 Mauchat 设置页）
