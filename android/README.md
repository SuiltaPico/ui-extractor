# Android

Android 端产出 `libui_extractor.so`（支持 **arm64-v8a**、**x86_64**）。  
OCR/图标推理由 `infer_core` 动态库提供，需与 `ui_extractor` 成对部署。

**完整说明见 [docs/android.md](../docs/android.md)**（模型准备、JNI、配置 JSON、故障排查）。

## 快速构建

```powershell
# 前置：NDK、models_dir（pack 布局）
rustup target add aarch64-linux-android x86_64-linux-android
cargo install cargo-ndk

powershell -ExecutionPolicy Bypass -File scripts/build_android.ps1
# 或打 Release 包：scripts/build_release_android.ps1
```

手动构建：

```powershell
cargo ndk -t arm64-v8a -o android/jniLibs build --release `
  --lib

cargo ndk -t x86_64 -o android/jniLibs build --release `
  --lib
```

输出：
- `android/jniLibs/arm64-v8a/libui_extractor.so`
- `android/jniLibs/x86_64/libui_extractor.so`

## 模型

模型包准备见 [docs/models.md](../docs/models.md)。  
推荐在 `local-infer-core` 仓库执行 `scripts/download_all_packs.ps1`，并将得到的 `models_dir` 解压到 Android 可读目录。

## C ABI

[`include/ui_extractor.h`](../include/ui_extractor.h)
