# ui-extractor

从 UI 截图提取元素树、文本坐标与图标名称，输出 JSON，供纯文本模型理解界面布局。

**完整文档：** [docs/README.md](docs/README.md)

## 能力

| 模块 | 实现 |
|------|------|
| UI 树 | 灰度 → Canny → 膨胀 → 闭运算 → Suzuki-Abe 轮廓层级 |
| 文本 | PaddleOCR PP-OCRv5 mobile |
| 图标 | MobileCLIP2-S0 向量嵌入 + 图标库余弦检索（未命中保留 container） |

## 快速开始

```bash
cargo build --release
```

```powershell
# 模型 + 图标库（详见 docs/getting-started.md）
powershell -ExecutionPolicy Bypass -File scripts/download_models.ps1
powershell -ExecutionPolicy Bypass -File scripts/download_mobileclip2.ps1
powershell -ExecutionPolicy Bypass -File scripts/download_mdi_icons.ps1 -Rasterize
cargo run --release -- icon build-embeddings

ui-extractor extract --input screenshot.png --annotate
```

## 推理后端

| Feature | 平台 | 引擎 |
|---------|------|------|
| `backend-ort`（默认） | 桌面 / CI | ONNX Runtime |
| `backend-ncnn` | Android | ncnn |

```bash
cargo build --release --no-default-features --features backend-ncnn   # Android / 嵌入式
```

## 文档

| 文档 | 内容 |
|------|------|
| [docs/getting-started.md](docs/getting-started.md) | 桌面端设置与 CLI |
| [docs/models.md](docs/models.md) | ONNX / ncnn 模型、pnnx 转换 |
| [docs/architecture.md](docs/architecture.md) | 流水线架构 |
| [docs/android.md](docs/android.md) | Android `.so` 构建与 JNI |
| [docs/dev/icon-matching.md](docs/dev/icon-matching.md) | 图标向量检索 |
| [docs/dev/mdi-icons.md](docs/dev/mdi-icons.md) | MDI 图标库 |

## 输出示例

```json
{
  "width": 1080,
  "height": 1920,
  "root": {
    "bounds": { "x": 0, "y": 0, "width": 1080, "height": 1920 },
    "kind": "root",
    "children": [
      {
        "bounds": { "x": 110, "y": 212, "width": 24, "height": 24 },
        "kind": "icon",
        "name": "home",
        "confidence": 0.89
      }
    ]
  }
}
```

## 库 API

```rust
use ui_extractor::{extract_from_path, ExtractConfig};
let result = extract_from_path("screenshot.png".as_ref(), &ExtractConfig::default())?;
```

C ABI：[`include/ui_extractor.h`](include/ui_extractor.h)

## Android

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build_android.ps1 -DownloadNcnn
# → android/jniLibs/{arm64-v8a,x86_64}/libui_extractor.so
```

## Release 打包

打 tag（如 `v0.1.0`）后 GitHub Actions 自动发布 4 个 zip：Windows x64/arm64、Android arm64-v8a/x86_64。

本地打包（会自动下载模型并生成 `embeddings.bin`，首次约需数分钟）：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build_release_windows.ps1
powershell -ExecutionPolicy Bypass -File scripts/build_release_android.ps1
# → dist/ui-extractor-<version>-*.zip（含 models/ 与 assets/embeddings.bin）
```

详见 [docs/android.md](docs/android.md) 与 [android/README.md](android/README.md)。

## 后续

- [ ] potrace 兜底（未命中图标 → SVG path）
- [ ] 多 namespace 图标库
- [x] ncnn 端侧推理（Android）
- [ ] 可选 OpenCV 布局后端
