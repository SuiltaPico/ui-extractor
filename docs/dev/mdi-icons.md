# MDI 图标库（已迁至 local-infer-core）

图标素材下载、SVG 栅格化、`icons.bundled` 索引构建已不在本仓库维护。

请在 [`local-infer-core`](https://github.com/SuiltaPico/local-infer-core) 使用：

```powershell
cd D:\repo\local-infer-core

# SVG 下载 + PNG 栅格化
powershell -ExecutionPolicy Bypass -File .\scripts\download_icons.ps1 -Rasterize

# 建 bundled 索引包（需 embed vision.onnx）
powershell -ExecutionPolicy Bypass -File .\tools\icon-index\build_bundled.ps1
```

运行时图标匹配架构见 [icon-matching.md](./icon-matching.md)。
