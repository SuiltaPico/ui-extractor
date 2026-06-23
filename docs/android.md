# Android 构建与集成

桌面 / CI 默认 `backend-ort`（ONNX Runtime）。Android 使用 **ncnn**：CLIP 与 OCR 均走 ncnn，**不链接** ONNX Runtime。

## 前置条件

| 依赖 | 说明 |
|------|------|
| Rust + Android targets | `rustup target add aarch64-linux-android x86_64-linux-android` |
| [cargo-ndk](https://github.com/bbqsrc/cargo-ndk) | `cargo install cargo-ndk` |
| Android NDK | 设置 `ANDROID_NDK_HOME` 或 `NDK_HOME`（推荐 r26+ / 30.x） |
| ncnn Android 静态库 | `third_party/ncnn/android/{arm64-v8a,x86_64}/lib/libncnn.a`（`scripts/download_ncnn_android.ps1`） |
| ncnn 模型 + 图标索引 | 见 [models.md](models.md) |

## ncnn 静态库

从 [Tencent/ncnn releases](https://github.com/Tencent/ncnn/releases) 下载 Android 构建（推荐 **20260526**，与 pnnx 版本对齐）。可用脚本自动安装：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/download_ncnn_android.ps1
```

或手动解压到：

```
third_party/ncnn/android/arm64-v8a/
third_party/ncnn/android/x86_64/
  include/ncnn/
  lib/libncnn.a
```

## 模型与 assets

Android 运行时读取**文件系统路径**（非 APK asset 名），需启动时解压到可读目录。

| 用途 | 文件 |
|------|------|
| OCR 检测 | `models/pp-ocrv5_mobile_det.ncnn.param` + `.bin` |
| OCR 识别 | `models/pp-ocrv5_mobile_rec.ncnn.param` + `.bin` |
| 字形表 | `models/ppocrv5_dict.txt` |
| 图标嵌入 | `models/mobileclip2-s0-vision.ncnn.param` + `.bin` |
| 嵌入索引 | `assets/embeddings.bin` |

`assets/icons/` 仅用于离线 `icon build-embeddings`，运行时 APK 不必打包 PNG。

准备步骤见 [models.md](models.md)：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/download_models.ps1          # dict
powershell -ExecutionPolicy Bypass -File scripts/download_models_ncnn.ps1       # OCR ncnn
powershell -ExecutionPolicy Bypass -File scripts/download_mobileclip2.ps1     # ONNX for convert
powershell -ExecutionPolicy Bypass -File scripts/convert_models_ncnn.ps1      # MobileCLIP2 ncnn
# 图标库
powershell -ExecutionPolicy Bypass -File scripts/download_mdi_icons.ps1 -Rasterize
cargo run --release --no-default-features --features backend-ncnn -- icon build-embeddings
```

## 编译 `.so`

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build_android.ps1 -DownloadNcnn
# Release 打包（arm64-v8a + x86_64 zip）
powershell -ExecutionPolicy Bypass -File scripts/build_release_android.ps1
```

等价于对两个 ABI 分别：

```powershell
cargo ndk -t arm64-v8a -o android/jniLibs build --release `
  --no-default-features --features backend-ncnn --lib
cargo ndk -t x86_64 -o android/jniLibs build --release `
  --no-default-features --features backend-ncnn --lib
```

（脚本会自动设置 `NCNN_LIB_DIR`、NDK。）

**输出：**
- `android/jniLibs/arm64-v8a/libui_extractor.so`
- `android/jniLibs/x86_64/libui_extractor.so`

### 环境变量

| 变量 | 说明 |
|------|------|
| `NCNN_LIB_DIR` | `libncnn.a` 所在目录 |
| `ANDROID_NDK_HOME` / `NDK_HOME` | NDK 根目录 |

## Gradle 集成

1. 将 `android/jniLibs/` 拷入 Android 工程 `app/src/main/jniLibs/`
2. 加载 native 库：`System.loadLibrary("ui_extractor")` 以及 **`libc++_shared`**（ncnn 链接 `c++_shared`）
3. 将 `models/`、`assets/embeddings.bin`、`assets/icons/` 放入 APK `assets/`，启动时解压到 files 目录
4. JNI 封装 C ABI（见下）

## C ABI / JNI

头文件：[`include/ui_extractor.h`](../include/ui_extractor.h)

主要入口：

```c
const char *ui_extractor_version(void);

void *ui_extractor_create(const char *config_json, char **out_error);
void  ui_extractor_destroy(void *handle);

int ui_extractor_extract_bytes(
    void *handle,
    const uint8_t *data, size_t len,
    char **out_json, char **out_error);

void ui_extractor_string_free(char *s);
```

### 配置 JSON 示例（ncnn）

解压后的**绝对路径**传入 `model_dir` / `vision_model` / `embedding_index`：

```json
{
  "run_ocr": true,
  "run_icon": true,
  "ocr": {
    "model_dir": "/data/user/0/com.example.app/files/models"
  },
  "icon": {
    "embedding_index": "/data/user/0/com.example.app/files/assets/embeddings.bin",
    "vision_model": "/data/user/0/com.example.app/files/models/mobileclip2-s0-vision.ncnn.param",
    "template_size": 48,
    "min_cosine": 0.72
  }
}
```

`ocr.model_dir` 下应包含 `pp-ocrv5_mobile_det.ncnn.param/.bin`、`pp-ocrv5_mobile_rec.ncnn.param/.bin`、`ppocrv5_dict.txt`。

## Cargo feature

```toml
# 桌面（默认）
cargo build --release

# Android / 嵌入式
cargo build --release --no-default-features --features backend-ncnn
```

`backend-ncnn` 启用时：

- `IconEmbedder` 加载 `.ncnn.param` / `.ncnn.bin`
- OCR 使用 ncnn PP-OCRv5
- 不依赖 `ort`、`oar-ocr`

## 常见问题

| 现象 | 原因 | 处理 |
|------|------|------|
| 链接 `ncnn_*` 未定义 | `NCNN_LIB_DIR` 未设或库路径错误 | 检查 `third_party/ncnn/.../libncnn.a` |
| 运行时 load 模型失败 | pnnx 与 libncnn 版本不一致 | 成套对齐 ncnn + pnnx，见 [models.md](models.md) |
| `.ncnn.param` 0 字节 | 下载中断 | 重新运行 `download_models_ncnn.ps1` |
| pnnx / PyTorch 失败 | pip 版 pnnx | 改用便携版 pnnx，见 [models.md](models.md) |

## crates/ncnn-bind

Rust 侧不再依赖上游 [ncnnrs](https://github.com/Baiyuetribe/ncnnrs)。`crates/ncnn-bind/` 手写 ui-extractor 实际用到的 C API（`Mat` / `Net` / `Extractor` 等），无 bindgen、无 vendored patch。

升级 ncnn 时：替换 `third_party/ncnn/` 静态库与头文件，对齐 pnnx 版本，跑 Android 推理回归即可。
