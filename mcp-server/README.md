# ui-extractor MCP Server

[MCP](https://modelcontextprotocol.io/docs/getting-started/intro) server exposing [ui-extractor](../README.md) screenshot analysis to AI clients (Cursor, Claude Desktop, etc.).

## Tools

| Tool | Description |
|------|-------------|
| `check_setup` | Verify binary, `infer_core` dynamic library, and model packs |
| `extract_ui` | Run layout + OCR + icon extraction on a screenshot |

## Prompts

| Prompt | Description |
|--------|-------------|
| `analyze_ui` | Guide the agent to extract and analyze clickable UI targets |

## Prerequisites

End users: download from GitHub Releases (no source build required).

1. **CLI**: [ui-extractor Release](https://github.com/SuiltaPico/ui-extractor/releases) — pick `ui-extractor-windows-x86_64-bundle.zip` (includes `infer_core.dll`)
2. **Model packs**: [local-infer-core Release](https://github.com/SuiltaPico/local-infer-core/releases) (same tag) — extract `ocr.*`, `embed.*`, `icons.*` zips into `models/{pack_id}/`

See [SKILL.md](../SKILL.md) for the full Release setup script and CLI examples.

Developers building from source: see [docs/getting-started.md](../docs/getting-started.md).

## Build

```bash
cd mcp-server
npm install
npm run build
```

## Run (stdio)

```bash
node dist/index.js
```

Environment variables:

| Variable | Default |
|----------|---------|
| `UI_EXTRACTOR_BIN` | `../target/release/ui-extractor` |
| `UI_EXTRACTOR_MODELS_DIR` | `../models` |
| `LOCAL_INFER_ROOT` | same as models dir |
| `UI_EXTRACTOR_OCR_PACK` | `ocr.paddle.ppocr6-tiny.onnx.fp32` |
| `UI_EXTRACTOR_ICON_INDEX_PACK` | `icons.bundled.v1.mobileclip2-s0.int8` |

`infer_core.dll` 默认从 `../.infer-core-release/` 或 binary 同目录探测（见 `mcp-server/src/config.ts`）。

## Cursor

Project config lives in [`.cursor/mcp.json`](../.cursor/mcp.json). Adjust paths for your machine, then enable the server in Cursor Settings → MCP.

Example (Windows, Release install):

```json
{
  "mcpServers": {
    "ui-extractor": {
      "command": "node",
      "args": ["D:/path/to/ui-extractor/mcp-server/dist/index.js"],
      "env": {
        "UI_EXTRACTOR_BIN": "C:/Users/you/ui-extractor/ui-extractor.exe",
        "UI_EXTRACTOR_MODELS_DIR": "C:/Users/you/ui-extractor/models"
      }
    }
  }
}
```

## Agent Skill

See [SKILL.md](../SKILL.md) at the repo root — Release download workflow, CLI usage, MCP config, and troubleshooting.
