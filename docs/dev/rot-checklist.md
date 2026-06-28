# 腐化清单

本文档记录 `ui-extractor` 与 [`local-infer-core`](https://github.com/SuiltaPico/local-infer-core) 的集成原则、偏离项与修复状态。

**最后核对：** 2026-06-29（native lib 单一来源：GitHub Release）

---

## 核心原则（不可妥协）

### 1. Native lib 唯一来源：GitHub Release

| 库 | Release 仓库 | 缓存目录 | 链接 / 运行时布局 |
|----|--------------|----------|-------------------|
| `infer_core.dll` / `libinfer_core.so` | `SuiltaPico/local-infer-core` | `.infer-core-release/{asset}/` | Windows: `lib/`；Android: `jniLibs/{abi}/` |
| `ui_extractor.dll` / `libui_extractor.so` | `SuiltaPico/ui-extractor` | Dart hook `outputDirectoryShared` | SDK zip（`lib/` 或 `jniLibs/`）；CLI 用 `*-bundle.zip` |

**禁止：**

- sibling `../local-infer-core` 目录 heuristic
- `build.rs` 内自动 `cargo build -p infer-core-ffi`
- 用环境变量指定 native lib / infer-core 源码路径（`LOCAL_INFER_CORE_ROOT`、`INFER_CORE_LIB_DIR`、`LOCAL_UI_EXTRACTOR_LIB` 等）
- Dart hook 回退到 `target/release` 或包内预装路径

**允许的唯一 override（显式、非 env）：**

- PowerShell 脚本参数：`-ReleaseTag`、`-ReleaseRepo`、`-DistDir`（模型 pack 本地 zip）
- Dart `pubspec.yaml` → `hooks.user_defines`：`release_repo`、`release_tag`
- `hooks.user_defines.skip_download`（CI 特殊场景）

**版本对齐：** `Cargo.toml` `version` → infer-core tag `v{version}`。改 FFI 契约时：**先** 发 local-infer-core Release，**再** bump / 发 ui-extractor。

### 2. 模型 pack：GitHub Release（默认）

- 目录布局：`{models_dir}/{pack_id}/manifest.json + …`
- 默认来源：`scripts/install_packs.ps1 -Source release`（URL 规则 + `install_packs` 内 pack 列表）
- 本地 zip 仅通过 `-DistDir` + `-Source local`（显式参数，非 env）

### 3. 双 DLL + 双模式 API（保留）

- **链接：** `infer_core.dll.lib` + 运行时 `infer_core.dll`（推理）
- **编排：** `ui_extractor.dll`（布局 / pipeline FFI）
- **API：** `open(config)` 自有 registry，或 `from_registry` / `createWithRegistry` 借用 Mauchat 的 registry

### 4. Rust 依赖分工

- git dep `infer-core`（`types-only`）：编译期类型与契约
- Release `infer_core` 动态库：运行期推理（与 types-only 解耦）

---

## 严重程度

| 级别 | 含义 |
|------|------|
| **P0** | 行为与 infer-core 契约冲突，或静默失效 |
| **P1** | 架构重复 / 集成层多路径 / 长期漂移 |
| **P2** | 孤儿脚本、Release 职责混乱 |
| **P3** | 文档滞后 |

---

## 总览

| 区域 | 状态 | 摘要 |
|------|------|------|
| `build.rs` | ✅ | 缺则自动拉 GitHub Release → `.infer-core-release/` |
| `infer_core_release.ps1` | ✅ | 下载 + 解析；无 sibling fallback |
| `infer_core_root.ps1` | ✅ | 已删除 |
| `install_packs.ps1` | ✅ | 默认 `release`；`scripts/packs/release.ps1` |
| `scripts/build.ps1` | ✅ | 开发入口：download + cargo + copy runtime dll |
| CLI / `ExtractEngine` | ✅ | manifest + `pack_id` |
| `src/infer/runtime.rs` | ✅ | types-only；无 env |
| `dart/` hook | ✅ | 仅 GitHub Release；`release_tag` in pubspec |
| `dart/` runtime | ✅ | 对齐 local_infer_core：`@Native` bundled / Android / `initUiExtractorLibrary` |
| Release zip | ✅ | 命名/布局对齐 infer-core；SDK + desktop `-bundle` + SHA256 |
| `catalog.json` | ✅ | 已删；pack URL = `releases/download/{tag}/{pack_id}.zip` |

---

## 已删除的腐化路径（2026-06-29）

| 项 | 说明 |
|----|------|
| `LOCAL_INFER_CORE_ROOT` / `INFER_CORE_LIB_DIR` | build 不再读取 |
| `build.rs` → `cargo build -p infer-core-ffi` | 不再隐式编译 sibling repo |
| `infer_core_root.ps1` | sibling 目录解析 |
| `infer_core_release.ps1` 本地 `infer_core.dll.lib` fallback | Release 必须自带 import lib |
| `install_packs` fixture / dev dist / env pack source | 仅 release 或 `-DistDir` |
| Dart `LOCAL_UI_EXTRACTOR_LIB` / `local_lib` / `target/release` 回退 | hook 仅 Release |
| README「两个 repo 同级 copy dll」 | 改为 `scripts/build.ps1` |

---

## 开发工作流（标准）

```powershell
# 1. 拉 infer-core native lib（链路与运行时 dll 同源）
powershell -ExecutionPolicy Bypass -File .\scripts\download_infer_core_release.ps1

# 2. 拉模型 pack
powershell -ExecutionPolicy Bypass -File .\scripts\install_packs.ps1 -Platform windows

# 3. 编译 + 复制 infer_core.dll 到 target/debug|release
powershell -ExecutionPolicy Bypass -File .\scripts\build.ps1
# 或 -Profile release

# 4. 回归
powershell -ExecutionPolicy Bypass -File .\scripts\test_cases.ps1
```

`cargo build` 单独执行时，`build.rs` 会自动下载 infer-core Release（需网络）。运行前仍需把 `infer_core.dll` 拷到 binary 目录：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install_packs.ps1 -Platform windows
powershell -ExecutionPolicy Bypass -File .\scripts\build.ps1 -Profile release

---

## 双模式 API（保留）

| 用户 | 做法 |
|------|------|
| standalone | `ExtractEngine::open(config)` / `ui_extractor_create(NULL, json, err)` |
| 共享 registry | `from_registry` / `createWithRegistry`（同一 `infer_core.dll`） |

---

## 目标架构

```
GitHub Release (local-infer-core)  →  .infer-core-release/  →  build.rs 链接 + 运行时 infer_core.dll
GitHub Release (ui-extractor)      →  Dart hook              →  ui_extractor.dll
install_packs.ps1 (+ release.ps1)     →  models/{pack_id}/
```

```
local_infer_core dart  →  infer_core（Release）+ Registry + RuntimeConfig
ui_extractor dart      →  ui_extractor.dll（Release）+ layout FFI
Mauchat                →  models_dir 一处；registry 可共享
```

---

## 待办 / 已知 ⚠️

| 项 | 级别 | 说明 |
|----|------|------|
| `LOCAL_INFER_ROOT` 运行时 | P3 | Mauchat 模型根目录 env；与 native lib 无关，可保留 |
| local-infer-core dart hook | P3 | 仍可能有 `local_lib`；ui-extractor 侧 hook + runtime 已 Release-only |

---

## 相关文档

- [../architecture.md](../architecture.md)
- [../models.md](../models.md)
- [../getting-started.md](../getting-started.md)
- [local-infer-core PRODUCT.md](https://github.com/SuiltaPico/local-infer-core/blob/master/docs/dev/PRODUCT.md)
