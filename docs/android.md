# Android 构建与集成

`ui-extractor` 在 Android 端只负责 UI 树提取与编排；推理由 `infer_core` 动态库提供。  
本文档聚焦 `libui_extractor.so` 构建与部署，模型包流程统一走 `local-infer-core`。

## 前置条件

| 依赖 | 说明 |
|------|------|
| Rust + Android targets | `rustup target add aarch64-linux-android x86_64-linux-android` |
| [cargo-ndk](https://github.com/bbqsrc/cargo-ndk) | `cargo install cargo-ndk` |
| Android NDK | 设置 `ANDROID_NDK_HOME` 或 `NDK_HOME`（推荐 r26+ / 30.x） |
| infer-core 动态库 | 与 `ui_extractor` 同版本部署（`infer_core.dll` / `libinfer_core.so`） |
| 模型包目录 | 见 [models.md](models.md)（由 `local-infer-core` 准备） |

## 模型与 assets

Android 运行时读取**文件系统目录**（非 APK asset 名），需启动时解压到可读路径。

准备步骤见 [models.md](models.md)：

```powershell
powershell -ExecutionPolicy Bypass -File ..\local-infer-core\scripts\download_all_packs.ps1
```

## 编译 `.so`

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build_android.ps1
# Release 打包（arm64-v8a + x86_64 zip）
powershell -ExecutionPolicy Bypass -File scripts/build_release_android.ps1
```

等价于对两个 ABI 分别：

```powershell
cargo ndk -t arm64-v8a -o android/jniLibs build --release `
  --lib
cargo ndk -t x86_64 -o android/jniLibs build --release `
  --lib
```

**输出：**
- `android/jniLibs/arm64-v8a/libui_extractor.so`
- `android/jniLibs/x86_64/libui_extractor.so`

### 环境变量

| 变量 | 说明 |
|------|------|
| `ANDROID_NDK_HOME` / `NDK_HOME` | NDK 根目录 |

## Gradle 集成

1. 将 `android/jniLibs/` 拷入 Android 工程 `app/src/main/jniLibs/`
2. 确保同时部署 `ui_extractor` 与 `infer_core` 两个动态库
3. 将模型包目录放入 APK `assets/`，启动时解压到 files 目录（如 `{files}/models/`）
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

### 配置 JSON 示例

解压后的**绝对路径**传入 `models_dir` + pack id：

```json
{
  "run_ocr": true,
  "run_icon": true,
  "models_dir": "/data/user/0/com.example.app/files/models",
  "ocr_pack": "ocr.paddle.ppocr6-tiny.mnn.fp32",
  "icon_index_pack": "icons.bundled.v1.mobileclip2-s0.int8"
}
```

## 常见问题

| 现象 | 原因 | 处理 |
|------|------|------|
| 启动时报找不到 infer_core | 未同时部署 `infer_core` 动态库 | 与 `ui_extractor` 一起打包并先加载 |
| 运行时找不到 pack | `models_dir` 下缺少对应 pack_id 目录 | 用 `local-infer-core` 脚本补齐模型包 |
