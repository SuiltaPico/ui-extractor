# ui-extractor 文档

从 UI 截图提取元素树、文本坐标与图标名称，输出 JSON，供纯文本模型理解界面布局。

## 文档索引

| 文档 | 内容 |
|------|------|
| [getting-started.md](getting-started.md) | 桌面端：依赖、模型、首次运行、CLI |
| [models.md](models.md) | manifest 模型包布局与 pack 选择 |
| [architecture.md](architecture.md) | 流水线架构与模块职责 |
| [android.md](android.md) | Android `.so` 构建、模型打包、JNI / C ABI |
| [dev/icon-matching.md](dev/icon-matching.md) | MobileCLIP2 向量检索与匹配策略 |
| [dev/mdi-icons.md](dev/mdi-icons.md) | MDI 图标库下载、栅格化、建索引 |

## 运行时依赖

`ui-extractor` 只保留布局/编排层，OCR 与图标嵌入推理由 `local-infer-core` 提供。
模型通过 pack id 从 `models_dir/{pack_id}/manifest.json` 解析，不再由本仓库脚本直接下载 ONNX/ncnn 文件。

```bash
cargo build --release
```

## 本地资源（不提交 git）

| 路径 | 说明 |
|------|------|
| `{models_dir}/{pack_id}/` | 由 `local-infer-core` 管理的模型包 |
| `assets/icons/`、`assets/svg/` | 可选图标素材（仅栅格化工具使用） |
| `android/jniLibs/` | 构建产物 `libui_extractor.so` |

## 工具链速查

| 用途 | 命令 / 脚本 |
|------|-------------|
| 下载官方模型包 | `..\local-infer-core\scripts\download_all_packs.ps1` |
| MDI 图标 SVG + PNG | `scripts/download_mdi_icons.ps1 -Rasterize` |
| 预计算嵌入索引 | `cargo run -p infer-core --bin icon-index-build -- ...`（在 `local-infer-core` 仓库） |
| Android `.so` | `scripts/build_android.ps1` |
| Windows Release 包（x64 / arm64） | `scripts/build_release_windows.ps1` |
| Android Release 包（arm64-v8a / x86_64） | `scripts/build_release_android.ps1` |
| GitHub Release（打 `v*` tag 触发） | `.github/workflows/release.yml` |
