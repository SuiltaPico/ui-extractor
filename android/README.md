# Android

Android 端使用 ncnn 后端（`backend-ncnn`），产出 `libui_extractor.so`（支持 **arm64-v8a**、**x86_64**）。

**完整说明见 [docs/android.md](../docs/android.md)**（模型准备、JNI、配置 JSON、故障排查）。

## 快速构建

```powershell
# 前置：NDK、models/*.ncnn.*、assets/（ncnn 库可用 -DownloadNcnn 自动下载）
rustup target add aarch64-linux-android x86_64-linux-android
cargo install cargo-ndk

powershell -ExecutionPolicy Bypass -File scripts/build_android.ps1 -DownloadNcnn
# 或打 Release 包：scripts/build_release_android.ps1
```

手动构建时必须 **`--no-default-features`**，否则会同时启用 `backend-ort` 与 `backend-ncnn` 导致编译失败：

```powershell
$env:NCNN_LIB_DIR = "third_party/ncnn/android/arm64-v8a/lib"
cargo ndk -t arm64-v8a -o android/jniLibs build --release `
  --no-default-features --features backend-ncnn --lib

$env:NCNN_LIB_DIR = "third_party/ncnn/android/x86_64/lib"
cargo ndk -t x86_64 -o android/jniLibs build --release `
  --no-default-features --features backend-ncnn --lib
```

输出：
- `android/jniLibs/arm64-v8a/libui_extractor.so`
- `android/jniLibs/x86_64/libui_extractor.so`

## 模型

ncnn 权重与 pnnx 转换见 [docs/models.md](../docs/models.md)。  
使用 [便携版 pnnx](https://github.com/pnnx/pnnx/releases) 转换 MobileCLIP2；OCR ncnn 可直接 `scripts/download_models_ncnn.ps1` 下载。

## C ABI

[`include/ui_extractor.h`](../include/ui_extractor.h)
