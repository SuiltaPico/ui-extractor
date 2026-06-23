# ui-extractor 文档

从 UI 截图提取元素树、文本坐标与图标名称，输出 JSON，供纯文本模型理解界面布局。

## 文档索引

| 文档 | 内容 |
|------|------|
| [getting-started.md](getting-started.md) | 桌面端：依赖、模型、首次运行、CLI |
| [models.md](models.md) | ONNX / ncnn 模型下载与 pnnx 转换 |
| [architecture.md](architecture.md) | 流水线架构与模块职责 |
| [android.md](android.md) | Android `.so` 构建、模型打包、JNI / C ABI |
| [dev/icon-matching.md](dev/icon-matching.md) | MobileCLIP2 向量检索与匹配策略 |
| [dev/mdi-icons.md](dev/mdi-icons.md) | MDI 图标库下载、栅格化、建索引 |

## 推理后端

| Feature | 平台 | 推理引擎 | OCR | 图标嵌入 |
|---------|------|----------|-----|----------|
| `backend-ort`（默认） | 桌面 / CI | ONNX Runtime | `oar-ocr` | MobileCLIP2 ONNX |
| `backend-ncnn` | Android / 嵌入式 | ncnn | PP-OCRv5 ncnn | MobileCLIP2 ncnn |

```bash
# 桌面（默认）
cargo build --release

# Android / ncnn
cargo build --release --no-default-features --features backend-ncnn
```

## 本地资源（不提交 git）

| 路径 | 说明 |
|------|------|
| `models/*.onnx` | ONNX 权重（桌面） |
| `models/*.ncnn.*` | ncnn 权重（Android） |
| `assets/icons/`、`assets/svg/`、`assets/embeddings.bin` | 图标库 |
| `third_party/ncnn/` | 预编译 ncnn 静态库 |
| `third_party/pnnx/` | 便携版 pnnx（可选，用于 ONNX→ncnn） |
| `crates/ncnn-bind/` | 手写 ncnn C API 最小绑定（无 bindgen） |
| `android/jniLibs/` | 构建产物 `libui_extractor.so` |

## 工具链速查

| 用途 | 命令 / 脚本 |
|------|-------------|
| 下载 ONNX OCR | `scripts/download_models.ps1` |
| 下载 MobileCLIP2 ONNX | `scripts/download_mobileclip2.ps1` |
| 下载 OCR ncnn（预转换） | `scripts/download_models_ncnn.ps1` |
| ONNX → ncnn（MobileCLIP2 等） | `scripts/convert_models_ncnn.ps1` |
| MDI 图标 SVG + PNG | `scripts/download_mdi_icons.ps1 -Rasterize` |
| 预计算嵌入索引 | `cargo run --release -- icon build-embeddings` |
| Android `.so` | `scripts/build_android.ps1` |
| 下载 ncnn Android 库 | `scripts/download_ncnn_android.ps1` |
| Windows Release 包（x64 / arm64） | `scripts/build_release_windows.ps1` |
| Android Release 包（arm64-v8a / x86_64） | `scripts/build_release_android.ps1` |
| GitHub Release（打 `v*` tag 触发） | `.github/workflows/release.yml` |
| 下载 ncnn（Android） | `scripts/download_ncnn_android.ps1` |
| Windows Release 包 | `scripts/build_release_windows.ps1` |
| Android Release 包 | `scripts/b uild_release_android.ps1` |
| GitHub Release（打 tag） | push `v*` → `.github/workflows/release.yml` |

> ncnn 模型转换使用 [便携版 pnnx](https://github.com/pnnx/pnnx/releases) 即可；`pip install pnnx` 会拉 PyTorch，在 Windows 上容易出问题。
