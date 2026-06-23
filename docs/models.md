# 模型与权重

ui-extractor 使用两类神经网络：**PaddleOCR PP-OCRv5 mobile**（文本）与 **MobileCLIP2-S0 vision**（图标嵌入）。  
桌面默认 ONNX；Android 使用 ncnn，共享同一套 `assets/embeddings.bin` 与 `ppocrv5_dict.txt`。

## 文件一览

### 桌面（ONNX / `backend-ort`）

| 文件 | 脚本 | 大小（约） |
|------|------|-----------|
| `models/pp-ocrv5_mobile_det.onnx` | `download_models.ps1` | ~5 MB |
| `models/pp-ocrv5_mobile_rec.onnx` | 同上 | ~16 MB |
| `models/ppocrv5_dict.txt` | 同上 | 小 |
| `models/mobileclip2-s0-vision.onnx` | `download_mobileclip2.ps1` | ~46 MB |

### Android（ncnn / `backend-ncnn`）

| 文件 | 获取方式 |
|------|----------|
| `models/pp-ocrv5_mobile_det.ncnn.param` + `.bin` | `download_models_ncnn.ps1`（预转换） |
| `models/pp-ocrv5_mobile_rec.ncnn.param` + `.bin` | 同上 |
| `models/mobileclip2-s0-vision.ncnn.param` + `.bin` | `convert_models_ncnn.ps1`（需先有 ONNX） |
| `models/ppocrv5_dict.txt` | `download_models.ps1`（与 ONNX 共用） |

所有 `models/*.onnx` 与 `models/*.ncnn.*` 均在 `.gitignore` 中，需本地生成。

## 桌面：下载 ONNX

```powershell
powershell -ExecutionPolicy Bypass -File scripts/download_models.ps1
powershell -ExecutionPolicy Bypass -File scripts/download_mobileclip2.ps1
```

## Android：准备 ncnn 权重

### 步骤 1 — OCR（推荐直接下载）

```powershell
powershell -ExecutionPolicy Bypass -File scripts/download_models_ncnn.ps1
powershell -ExecutionPolicy Bypass -File scripts/download_models.ps1   # 仅 dict，若尚未下载
```

来源：[ncnn-assets](https://github.com/nihui/ncnn-assets) 预转换 PP-OCRv5 mobile。

### 步骤 2 — MobileCLIP2（pnnx 转换）

需先有 `models/mobileclip2-s0-vision.onnx`，再运行：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/convert_models_ncnn.ps1
```

仅转换缺失的模型；已存在的 `.ncnn.param` / `.bin` 会跳过。

## pnnx：ONNX → ncnn
1. 从 [pnnx releases](https://github.com/pnnx/pnnx/releases) 下载对应平台 zip（Windows: `pnnx-*-windows.zip`）
2. 解压 `pnnx.exe` 到 `third_party/pnnx/`（例如 `third_party/pnnx/pnnx-20241226-windows/pnnx.exe`）
3. 运行 `scripts/convert_models_ncnn.ps1`

脚本查找顺序：`PATH` 上的 `pnnx` → `third_party/pnnx/**/pnnx.exe`。

### 手动转换示例

```powershell
cd models
pnnx mobileclip2-s0-vision.onnx inputshape=[1,3,256,256]
# 产出 mobileclip2-s0-vision.ncnn.param + .ncnn.bin
```

### 为什么不推荐 `pip install pnnx`

- pip 版依赖 PyTorch，Windows 上常见 `c10.dll` 加载失败
- 便携版单文件、无 Python 环境，与 ncnn 版本独立

## ncnn 绑定与版本

Android 静态库需与 pnnx 转换版本对齐（见下）。Rust 侧使用仓库内 **`crates/ncnn-bind`**：手写 ~20 个 C API 声明，**不依赖 ncnnrs / bindgen / 主机 LLVM**。

当前 Android 构建已验证 **ncnn 20260526** + **pnnx 20260526**（`libui_extractor.so` 可正常链接）。若降级至 20241226，pnnx 与模型亦需一并回退。

预编译库路径：`third_party/ncnn/android/arm64-v8a/`（见 [android.md](android.md)）。

### pnnx 与 libncnn 宜对齐

| 组件 | 建议 |
|------|------|
| pnnx 20260526 转模型 | libncnn 20260526 |
| pnnx 20241226 转模型 | libncnn 20241226 |

版本不一致时，常见图通常仍能跑；若 pnnx 生成了旧版 ncnn 不支持的算子，会在**运行时**加载模型失败。

## 图标索引（与后端无关）

| 文件 | 生成 |
|------|------|
| `assets/embeddings.bin` | `cargo run --release -- icon build-embeddings` |
| `assets/icons/*.png` | `download_mdi_icons.ps1 -Rasterize` 或自备 |

索引格式与 ONNX / ncnn 无关；Android 打包时与 ncnn 权重一并放入 `assets/`。

## 校验

```powershell
# ONNX
Get-ChildItem models -Filter *.onnx | Select-Object Name, Length

# ncnn（param 不应为 0 字节）
Get-ChildItem models -Filter *.ncnn.* | Select-Object Name, Length
```

若 `.ncnn.param` 为 0 字节，重新下载或转换。
