# ui_extractor Dart Bindings

从截图提取 UI 元素树（布局 + OCR + 图标识别）的 Dart/Flutter 绑定。

依赖 [`local_infer_core`](https://github.com/SuiltaPico/local-infer-core/tree/v0.1.0/dart) 共享 infer-core registry 与 `RuntimeConfig` 类型。

## 安装

```yaml
dependencies:
  ui_extractor:
    git:
      url: https://github.com/SuiltaPico/ui-extractor
      path: dart
  local_infer_core:
    git:
      url: https://github.com/SuiltaPico/local-infer-core
      path: dart
```

Build hook 仅下载并注册 `ui_extractor.dll`（优先 slim zip）；`infer_core.dll` 由 `local_infer_core` hook 提供。

## 快速开始

### 独立模式（自带 registry）

```dart
import 'package:ui_extractor/ui_extractor.dart';

final engine = UiExtractorEngine.create(
  ExtractorConfig.defaults(modelsDir: r'C:\path\to\models'),
);
try {
  final tree = engine.extractFile('screenshot.png');
} finally {
  engine.dispose();
}
```

### 与 local_infer_core 共享 registry（Mauchat）

```dart
import 'package:local_infer_core/local_infer_core.dart';
import 'package:ui_extractor/ui_extractor.dart';

final registry = await LocalInferRegistry.open(
  modelsDir: modelsDir,
  runtimeConfig: RuntimeConfig.auto(),
);
final engine = UiExtractorEngine.createWithRegistry(
  registry,
  const ExtractorLayoutConfig(),
);
try {
  final tree = engine.extractFile('screenshot.png');
} finally {
  engine.dispose();
  registry.dispose();
}
```

## API 概览

| 类型 | 作用 |
|------|------|
| `UiExtractor.version` | 原生库版本 |
| `ExtractorConfig` | 独立模式 JSON 配置（含 `modelsDir` / `runtime`） |
| `ExtractorLayoutConfig` | 借用 registry 时的布局/OCR/图标参数 |
| `ExtractorConfig.defaults()` | 默认 `./models` 或 `LOCAL_INFER_ROOT` |
| `UiExtractorEngine.create(config)` | 独立模式 |
| `UiExtractorEngine.createWithRegistry(registry, config)` | 借用 infer-core registry |
| `engine.extractFile(path)` | 提取 UI 树，返回 `Map<String, dynamic>` |
| `engine.dispose()` | 释放 native 句柄 |

返回的 JSON 结构与 CLI 的 `ui-extractor extract` 输出一致。

## 模型路径

模型 pack 安装到 `models_dir`（manifest 目录布局），由 `local_infer_core` 的 pack 脚本管理；运行时通过 infer-core registry 加载 `icons.bundled.*` 等 pack，不再使用旧的 `assets/embeddings.bin`。

## 开发

本地无 GitHub Release 时，构建 Rust crate 并设置 `LOCAL_UI_EXTRACTOR_LIB`，或将 release zip 解压后通过 `ExtractorConfig.defaults(modelsDir: ...)` 指定模型根目录。参见仓库根目录 `README.md`。
