# ui-extractor

从 UI 截图提取元素树、文本坐标与图标名称，输出 JSON，供纯文本模型理解界面布局。

**完整文档：** [docs/README.md](docs/README.md)

## 能力

| 模块 | 实现 |
|------|------|
| UI 树 | 灰度 → Canny → 膨胀 → 闭运算 → Suzuki-Abe 轮廓层级 |
| 文本 | 通过 `local-infer-core` 加载 OCR pack（manifest 驱动） |
| 图标 | 通过 `local-infer-core` 的 embed/icon_index pack 做余弦检索（未命中保留 container） |

## 快速开始（Windows，从零可用）

> 假设两个仓库同级：`D:\repo\ui-extractor` 与 `D:\repo\local-infer-core`。

```powershell
# 1) 在 local-infer-core 构建推理动态库
cd D:\repo\local-infer-core
cargo build -p infer-core-ffi

# 2) 在 ui-extractor 准备模型包（默认装到 ui-extractor/models）
cd D:\repo\ui-extractor
powershell -ExecutionPolicy Bypass -File .\scripts\install_packs.ps1 -Platform windows

# 3) 让 ui-extractor CLI 能找到 infer_core.dll（首次必做）
Copy-Item -Force ..\local-infer-core\target\debug\infer_core.dll .\target\debug\infer_core.dll

# 4) 运行一次提取（验证完整链路：布局 + OCR + 图标）
cargo run --bin ui-extractor -- extract --input .\tests\cases\zhihu\input.png --annotate `
  --models-dir .\models `
  --ocr-pack ocr.paddle.ppocr6-tiny.onnx.fp32 `
  --icon-index-pack icons.bundled.v1.mobileclip2-s0.int8
```

如需一键回归用例：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\test_cases.ps1
```

若出现 `0xc0000135` / `STATUS_DLL_NOT_FOUND`，说明 `infer_core.dll` 不在可执行文件同目录或 `PATH` 中。

## 推理后端

`ui-extractor` 不再内置独立 ML 后端选择；运行时统一通过 `infer_core.dll`/`libinfer_core.so`（来自 `local-infer-core`）执行 OCR 与嵌入推理。

## 文档

| 文档 | 内容 |
|------|------|
| [docs/getting-started.md](docs/getting-started.md) | 桌面端设置与 CLI |
| [docs/models.md](docs/models.md) | manifest 模型包布局与 pack 选择 |
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
powershell -ExecutionPolicy Bypass -File scripts/build_android.ps1
# → android/jniLibs/{arm64-v8a,x86_64}/libui_extractor.so
```

## Release 打包

打 tag（如 `v0.1.0`）后 GitHub Actions 自动发布 4 个 zip：Windows x64/arm64、Android arm64-v8a/x86_64。

本地打包（模型包由 `local-infer-core` 管理）：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build_release_windows.ps1
powershell -ExecutionPolicy Bypass -File scripts/build_release_android.ps1
# → dist/ui-extractor-<version>-*.zip（仅二进制与头文件，不含 models）
```

模型包请从 `local-infer-core` 同版本 Release 下载（`ocr.*` / `embed.*` / `icons.*`），
或在开发环境运行：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/install_packs.ps1 -Platform windows
```

详见 [docs/android.md](docs/android.md) 与 [android/README.md](android/README.md)。

## 后续

- [ ] potrace 兜底（未命中图标 → SVG path）
- [ ] 多 namespace 图标库
- [x] 推理能力迁移到 `local-infer-core`
- [ ] 可选 OpenCV 布局后端
