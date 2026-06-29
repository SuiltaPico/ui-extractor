# 快速开始

## 前置依赖

| 依赖 | 用途 |
|------|------|
| Rust stable + Cargo | 编译与运行 |
| 网络（首次） | 从 GitHub Release 拉 `infer_core` native lib 与模型 pack |
| `infer_core.dll` | 与 `ui-extractor.exe` 同目录（`scripts/build.ps1` 会自动复制） |

无需克隆 `local-infer-core` 源码，也**不必**两个仓库同级。

## 构建

```powershell
# 推荐：download + build + copy runtime dll
powershell -ExecutionPolicy Bypass -File .\scripts\build.ps1

# release
powershell -ExecutionPolicy Bypass -File .\scripts\build.ps1 -Profile release
```

`cargo build` 会在 `build.rs` 中自动从 GitHub Release 下载 `infer_core` 到 `.infer-core-release/`（tag 对齐 `Cargo.toml` version）。首次编译需要网络。运行前仍需将 `infer_core.dll` 放到 binary 同目录（`build.ps1` 会复制）。

## 首次可用链路（Windows）

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install_packs.ps1 -Platform windows
powershell -ExecutionPolicy Bypass -File .\scripts\build.ps1 -Profile release
```

## 模型目录

`install_packs.ps1` 从 [local-infer-core Release](https://github.com/SuiltaPico/local-infer-core/releases) 安装到 `./models`：

| 路径 | 说明 |
|------|------|
| `models/ocr.paddle.ppocr6-tiny.onnx.fp32/` | OCR pack |
| `models/embed.mobileclip2-s0.onnx.fp32/` | embed pack |
| `models/icons.bundled.v1.mobileclip2-s0.int8/` | icon_index pack |

pack 由 `install_packs.ps1` 从 GitHub Release 安装（URL：`{pack_id}.zip`，tag 对齐 `Cargo.toml` version）。

## 基本用法

```bash
cargo run --release --bin ui-extractor -- extract --input screenshot.png --format pretty `
  --models-dir models `
  --ocr-pack ocr.paddle.ppocr6-tiny.onnx.fp32 `
  --icon-index-pack icons.bundled.v1.mobileclip2-s0.int8

cargo run --release --bin ui-extractor -- extract --input screenshot.png -o out.json --annotate
cargo run --release --bin ui-extractor -- extract --input screenshot.png --no-icon
```

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\test_cases.ps1
```

### Case 目录约定

```
tests/cases/<name>/
  input.png
  output.json
  output.annotated.png   # 可选 golden
```

## 发布纪律

改 infer-core FFI 时：先发布 `local-infer-core` Release（同 tag），再构建 ui-extractor。

详见 [dev/rot-checklist.md](dev/rot-checklist.md)。
