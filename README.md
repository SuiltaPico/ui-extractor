# ui-extractor

**English** | [中文](README.cn.md)

Extract UI element trees, text coordinates, and icon names from screenshots as JSON for text-only models to understand layout.

**Full docs:** [docs/README.md](docs/README.md)

## Capabilities

| Module | Implementation |
|--------|----------------|
| UI tree | Grayscale → Canny → dilate → close → Suzuki–Abe contour hierarchy |
| Text | OCR packs loaded via `local-infer-core` (manifest-driven) |
| Icons | Cosine retrieval via embed/icon_index packs from `local-infer-core` (unmatched regions stay as containers) |

## Quick start (Windows, zero to running)

No need to clone `local-infer-core` or keep both repos as siblings — GitHub Releases are enough.

### 1. Download binaries

Download `ui-extractor-windows-x64.zip` (or arm64) from [ui-extractor v0.1.0 Release](https://github.com/SuiltaPico/ui-extractor/releases/tag/v0.1.0) and extract anywhere, e.g. `C:\tools\ui-extractor`. The zip includes `ui-extractor.exe` and `infer_core.dll`.

### 2. Download model packs

Download from [local-infer-core v0.1.0 Release](https://github.com/SuiltaPico/local-infer-core/releases/tag/v0.1.0) and extract each zip under `models\{pack_id}\` (a `models` folder next to `ui-extractor.exe`):

| Asset zip | Extract to |
|-----------|------------|
| `ocr.paddle.ppocr6-tiny.onnx.fp32.zip` | `models\ocr.paddle.ppocr6-tiny.onnx.fp32\` |
| `embed.mobileclip2-s0.onnx.fp32.zip` | `models\embed.mobileclip2-s0.onnx.fp32\` |
| `icons.bundled.v1.mobileclip2-s0.int8.zip` | `models\icons.bundled.v1.mobileclip2-s0.int8\` |

### 3. Run

```powershell
cd C:\tools\ui-extractor
.\ui-extractor.exe extract --input screenshot.png --annotate `
  --models-dir .\models `
  --ocr-pack ocr.paddle.ppocr6-tiny.onnx.fp32 `
  --icon-index-pack icons.bundled.v1.mobileclip2-s0.int8
```

Point `--models-dir` at any folder, or set the `LOCAL_INFER_ROOT` environment variable for the model root.

If you see `0xc0000135` / `STATUS_DLL_NOT_FOUND`, `infer_core.dll` is not next to `ui-extractor.exe` and not on `PATH`.

### Build from source (optional)

When cloning and building locally, keep `ui-extractor` and `local-infer-core` as siblings (e.g. `D:\repo\ui-extractor` and `D:\repo\local-infer-core`), then:

```powershell
# Build infer_core.dll
cd D:\repo\local-infer-core
cargo build -p infer-core-ffi

# Install model packs + copy the dynamic library
cd D:\repo\ui-extractor
powershell -ExecutionPolicy Bypass -File .\scripts\install_packs.ps1 -Platform windows
Copy-Item -Force ..\local-infer-core\target\debug\infer_core.dll .\target\debug\infer_core.dll

# Verify
cargo run --bin ui-extractor -- extract --input .\tests\cases\zhihu\input.png --annotate `
  --models-dir .\models `
  --ocr-pack ocr.paddle.ppocr6-tiny.onnx.fp32 `
  --icon-index-pack icons.bundled.v1.mobileclip2-s0.int8
```

One-shot regression: `powershell -ExecutionPolicy Bypass -File .\scripts\test_cases.ps1`

## Inference backend

OCR and embedding inference run through `infer_core.dll` / `libinfer_core.so` from `local-infer-core`.

## Documentation

| Doc | Contents |
|-----|----------|
| [docs/getting-started.md](docs/getting-started.md) | Desktop setup and CLI |
| [docs/models.md](docs/models.md) | Manifest pack layout and pack selection |
| [docs/architecture.md](docs/architecture.md) | Pipeline architecture |
| [docs/android.md](docs/android.md) | Android `.so` build and JNI |
| [docs/dev/icon-matching.md](docs/dev/icon-matching.md) | Icon vector retrieval |
| [docs/dev/mdi-icons.md](docs/dev/mdi-icons.md) | MDI icon library |

## Output example

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

## Library API

```rust
use ui_extractor::{extract_from_path, ExtractConfig};
let result = extract_from_path("screenshot.png".as_ref(), &ExtractConfig::default())?;
```

## MCP Server & Agent Skill

- **MCP Server:** [`mcp-server/`](mcp-server/README.md) — exposes `extract_ui` / `check_setup` tools to Cursor and other AI clients
- **Agent Skill:** [`SKILL.md`](SKILL.md) — Release download + CLI/MCP guide ([Agent Skills](https://agentskills.io/home) format)
- **Cursor config:** [`.cursor/mcp.json`](.cursor/mcp.json)

```powershell
cd mcp-server
npm install && npm run build
```

C ABI: [`include/ui_extractor.h`](include/ui_extractor.h)

## Android

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build_android.ps1
# → android/jniLibs/{arm64-v8a,x86_64}/libui_extractor.so
```

## Release packaging

Tagging (e.g. `v0.1.0`) triggers GitHub Actions to publish four zips: Windows x64/arm64, Android arm64-v8a/x86_64.

Local packaging (model packs are managed by `local-infer-core`):

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build_release_windows.ps1
powershell -ExecutionPolicy Bypass -File scripts/build_release_android.ps1
```

Download model packs from the matching `local-infer-core` Release (`ocr.*` / `embed.*` / `icons.*`), or in a dev checkout run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/install_packs.ps1 -Platform windows
```

See [docs/android.md](docs/android.md) and [android/README.md](android/README.md).

## Roadmap

- [ ] potrace fallback (unmatched icons → SVG path)
- [ ] Multi-namespace icon libraries
- [x] Inference moved to `local-infer-core`
- [ ] Optional OpenCV layout backend
