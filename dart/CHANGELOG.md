## 0.1.0

- Initial release of Dart/Flutter bindings for ui-extractor.
- Native assets build hook (hooks 2.x) downloads prebuilt binaries from GitHub Releases.
- Supported platforms: Windows (x64, arm64) and Android (arm64-v8a, x86_64).
- High-level API: `UiExtractorEngine`, `ExtractorConfig`, `BundledAssets`.
- Override native library via `LOCAL_UI_EXTRACTOR_LIB` or hooks `local_lib` user define.
