# ui-extractor — 实现者说明

**本仓库正在向「纯 UI 语义层」迁移；ML 迁入 [local-infer-core](../local-infer-core/)。**

## 先读什么

1. [local-infer-core/IMPLEMENTATION.md](../local-infer-core/IMPLEMENTATION.md) — **总上手文档（从这里理解全局）**
2. [local-infer-core/PRODUCT.md](../local-infer-core/PRODUCT.md) — manifest、包 id、LICENSE
3. [PRODUCT.md](PRODUCT.md) — 本仓库迁移后的目标形态

## 你今天看到的代码 = 迁移前

- `backend-ort`（默认）：`oar-ocr 0.6.3` + PP-OCR**v5** + MobileCLIP2 ONNX
- `backend-ncnn`：**将删除**，勿再投入
- 模型路径写死在 `src/ocr/mod.rs`、`ExtractorConfig`（Dart）

## 你要做的事（ui-extractor 侧）

1. `Cargo.toml` 增加 `infer-core` path 依赖
2. 删除/替换 `src/ocr/`、`src/icon/embedder_*.rs`、`src/ort_runtime.rs` → 调用 infer-core
3. `ExtractEngine` / `pipeline` / `ffi` 改为按 **pack id** + `models_dir` 配置
4. Golden tests (`tests/cases/`) 迁移后必须仍通过
5. 文档 `docs/getting-started.md` 改为指向 local-infer-core Releases 与 manifest 布局

## Dart 包 `ui_extractor`（`dart/`）

- **保留**在本仓库，但 **瘦身**：`pubspec` 增加 `local_infer_core` 依赖
- **删除** hook 里捆绑 `models/`、`embeddings.bin` 的逻辑（见 `dart/hook/build.dart` 今日实现）
- **删除** `BundledAssets` 找 fat zip；改传 `modelsDir` + pack id
- hook 只拉 `ui_extractor.dll`；与 `infer_core.dll` 同进程加载（Mauchat 两个 Native Assets）

详细 API 见 [local-infer-core/IMPLEMENTATION.md §6.2](../local-infer-core/IMPLEMENTATION.md#62-dart-包-ui_extractorui-extractor-仓库-dart)。

## 不要在这里做

- 新 OCR / embed runtime（含 MNN）
- 官方模型 zip Release（属 local-infer-core）
- NCNN 维护

## 图标索引

- 开发：`scripts/download_icon_libraries.ps1` 等多库 SVG → 本地建索引
- Release：只产出合并包 `icons.bundled.v1.*`（见 local-infer-core PRODUCT.md）
