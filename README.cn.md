# ui-extractor

[English](README.md) | **中文**

从 UI 截图提取元素树、文本坐标与图标名称，输出 JSON，供纯文本模型理解界面布局。

**完整文档：** [docs/README.md](docs/README.md)

## 能力

| 模块 | 实现 |
|------|------|
| UI 树 | 灰度 → Canny → 膨胀 → 闭运算 → Suzuki-Abe 轮廓层级 |
| 文本 | 通过 `local-infer-core` 加载 OCR pack（manifest 驱动） |
| 图标 | 通过 `local-infer-core` 的 embed/icon_index pack 做余弦检索（未命中保留 container） |

## 快速开始（Windows，从零可用）

无需克隆 `local-infer-core` 源码，也**不必**两个仓库同级；从 GitHub Release 下载即可运行。

### 1. 下载二进制

从 [ui-extractor v0.1.0 Release](https://github.com/SuiltaPico/ui-extractor/releases/tag/v0.1.0) 下载 `ui-extractor-windows-x86_64-bundle.zip`（或 `windows-aarch64-bundle`），解压到任意目录，例如 `C:\tools\ui-extractor`。包内已含 `ui-extractor.exe` 与 `infer_core.dll`。

### 2. 下载模型包

从 [local-infer-core v0.1.0 Release](https://github.com/SuiltaPico/local-infer-core/releases/tag/v0.1.0) 下载并解压到 `models\{pack_id}\`（与 `ui-extractor.exe` 同级的 `models` 目录）：

| 资产 zip | 解压目标 |
|----------|----------|
| `ocr.paddle.ppocr6-tiny.onnx.fp32.zip` | `models\ocr.paddle.ppocr6-tiny.onnx.fp32\` |
| `embed.mobileclip2-s0.onnx.fp32.zip` | `models\embed.mobileclip2-s0.onnx.fp32\` |
| `icons.bundled.v1.mobileclip2-s0.int8.zip` | `models\icons.bundled.v1.mobileclip2-s0.int8\` |

### 3. 运行

```powershell
cd C:\tools\ui-extractor
.\ui-extractor.exe extract --input screenshot.png --annotate `
  --models-dir .\models `
  --ocr-pack ocr.paddle.ppocr6-tiny.onnx.fp32 `
  --icon-index-pack icons.bundled.v1.mobileclip2-s0.int8
```

也可将 `--models-dir` 指向任意目录，或通过环境变量 `LOCAL_INFER_ROOT` 指定模型根路径。

若出现 `0xc0000135` / `STATUS_DLL_NOT_FOUND`，说明 `infer_core.dll` 不在 `ui-extractor.exe` 同目录或 `PATH` 中。

### 从源码开发（可选）

克隆本仓库即可开发，**无需** `local-infer-core` 同级目录：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\download_infer_core_release.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\install_packs.ps1 -Platform windows
powershell -ExecutionPolicy Bypass -File .\scripts\build.ps1

cargo run --bin ui-extractor -- extract --input .\tests\cases\zhihu\input.png --annotate `
  --models-dir .\models `
  --ocr-pack ocr.paddle.ppocr6-tiny.onnx.fp32 `
  --icon-index-pack icons.bundled.v1.mobileclip2-s0.int8
```

一键回归：`powershell -ExecutionPolicy Bypass -File .\scripts\test_cases.ps1`

集成原则见 [docs/dev/rot-checklist.md](docs/dev/rot-checklist.md)。

## 推理后端

运行时通过 `infer_core.dll`/`libinfer_core.so`（来自 `local-infer-core`）执行 OCR 与嵌入推理。

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

## MCP Server 与 Agent Skill

- **MCP Server**：[`mcp-server/`](mcp-server/README.md) — 向 Cursor 等 AI 客户端暴露 `extract_ui` / `check_setup` 工具
- **Agent Skill**：[`SKILL.md`](SKILL.md) — Release 下载 + CLI/MCP 使用指南（[Agent Skills](https://agentskills.io/home) 格式）
- **Cursor 配置**：[`.cursor/mcp.json`](.cursor/mcp.json)

```powershell
cd mcp-server
npm install && npm run build
```

C ABI：[`include/ui_extractor.h`](include/ui_extractor.h)

## Android

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build_android.ps1
# → android/jniLibs/{arm64-v8a,x86_64}/libui_extractor.so
```

## Release 打包

打 tag（如 `v0.1.0`）后 GitHub Actions 自动发布 SDK zip（`ui-extractor-windows-{x86_64,aarch64}.zip`、Android arm64-v8a/x86_64）、桌面 CLI bundle（`*-bundle.zip`）及 `SHA256SUMS.txt`。

本地打包（模型包由 `local-infer-core` 管理）：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build_release_windows.ps1
powershell -ExecutionPolicy Bypass -File scripts/build_release_android.ps1
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
