# ui_extractor Dart Bindings

从截图提取 UI 元素树（布局 + OCR + 图标识别）的 Dart/Flutter 绑定。

## 安装

```yaml
dependencies:
  ui_extractor:
    git:
      url: https://github.com/SuiltaPico/ui-extractor
      path: dart
```

首次 `dart pub get` / `flutter pub get` 时，`hook/build.dart` 会从 GitHub Release 下载对应平台的原生库和模型（需已发布 `v0.1.0` 等资源包）。

## 快速开始

```dart
import 'dart:io';
import 'package:ui_extractor/ui_extractor.dart';

Future<void> main() async {
  print('ui-extractor ${UiExtractor.version}');

  // 自动查找 build hook 解压的 models/ + assets/
  final engine = UiExtractorEngine.createBundled();

  // 或手动指定资源根目录（release zip 解压后的目录）
  // final engine = UiExtractorEngine.create(
  //   ExtractorConfig.fromAssetRoot(r'C:\path\to\ui-extractor-windows-x64'),
  // );

  try {
    final tree = engine.extractFile('screenshot.png');
    stdout.writeln(tree);

    // 也可以传内存中的图片
    // final bytes = await File('screenshot.png').readAsBytes();
    // final tree = engine.extractBytes(bytes);
  } finally {
    engine.dispose();
  }
}
```

## API 概览

| 类型 | 作用 |
|------|------|
| `UiExtractor.version` | 原生库版本 |
| `ExtractorConfig` | 传给 native 的 JSON 配置（模型路径、OCR/图标参数） |
| `ExtractorConfig.fromAssetRoot(dir)` | 从 release 包根目录生成配置 |
| `UiExtractorEngine.create(config)` | 创建引擎（加载模型，较慢，应复用） |
| `UiExtractorEngine.createBundled()` | 自动定位 hook 下载的资源 |
| `engine.extractFile(path)` | 提取 UI 树，返回 `Map<String, dynamic>` |
| `engine.extractBytes(bytes)` | 同上，输入为图片字节 |
| `engine.dispose()` | 释放 native 句柄 |

返回的 JSON 结构与 CLI 的 `ui-extractor extract` 输出一致，可直接 `jsonEncode` 或交给 LLM。

## 资源路径

Native 库需要**磁盘上的绝对/相对路径**访问模型，不能直接读 Dart AssetBundle。Build hook 会把 release zip 解压到 `.dart_tool/hooks_runner/.../ui-extractor-<platform>/`，其中包含：

```
ui-extractor-windows-x64/
  ui_extractor.dll
  models/
  assets/embeddings.bin
```

`createBundled()` 会尝试定位该目录；若找不到，回退到当前工作目录下的 `models/` 和 `assets/`（适合在 release zip 内直接运行）。

## 开发

本地无 GitHub Release 时，可手动构建 native 库并将 release zip 解压到某目录，再用 `ExtractorConfig.fromAssetRoot` 指向它。参见仓库根目录 `README.md`。
