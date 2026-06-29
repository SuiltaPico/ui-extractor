# 腐化清单

本文档记录 `ui-extractor` 与 [`local-infer-core`](https://github.com/SuiltaPico/local-infer-core) 集成后的架构偏离项及修复状态。  
**最后核对：** 2026-06-29（腐化修复落地）

---

## 严重程度

| 级别 | 含义 |
|------|------|
| **P0** | 行为与 infer-core 契约冲突，或静默失效 |
| **P1** | 架构重复 / 与 infer-core 设计偏离，长期漂移风险 |
| **P2** | 孤儿代码、过时脚本、Release 职责混乱 |
| **P3** | 文档滞后、命名/注释过时 |

---

## 总览

| 区域 | 状态 | 摘要 |
|------|------|------|
| CLI / `ExtractEngine` 主流程 | ✅ | manifest + `pack_id` |
| `src/infer/runtime.rs` | ✅ | git dep `infer-core` types-only；无 env 逻辑 |
| `src/infer/icon_index.rs` | ✅ | `infer_icon_index_*` FFI 薄封装 |
| `src/infer/registry.rs` | ✅ | Owned/Borrowed；manifest 经 FFI 缓存 |
| `ExtractEngine` / C FFI | ✅ | `new(config, Option<Registry>)`；`ui_extractor_create(reg?, json)` |
| `dart/` | ✅ | 依赖 `local_infer_core`；`createWithRegistry` |
| Release zip | ⚠️ | hook 优先 `-slim` zip；完整 zip 仍作 fallback |
| `crates/ncnn-bind/` 等 | ✅ | 已删除 |
| 文档 | ✅ | README / icon-matching / getting-started 已更新 |

---

## 已修复项（摘要）

### P0 — RuntimeConfig 环境变量

- 删除 `from_env_or_default()`、`resolved_eps()` 等复活逻辑
- CLI 增加 `--runtime-config`；FFI JSON 缺 `runtime` 时用 `RuntimeConfig::default()`

### P1 — icon_index / registry / 双模式 API

- `IconIndex` 改走 `infer_icon_index_load` / `match_embedding` / `search`
- `Registry`：`RegistryOwnership` + `from_borrowed`；manifest 经 `infer_registry_manifest_json`
- `ExtractEngine::new` / `open` / `from_registry`；C `ui_extractor_create(infer_registry, json, err)`
- 移除 `IconPack::load`、`EmbedEngine::load`、`ui_icon_pack_*` legacy C API

### P2 — 遗留产物

- 删除 `crates/ncnn-bind/`、`scripts/download_models.ps1`
- Dart hook 优先下载 slim zip（仅 `ui_extractor.dll`）

### P3 — 文档

- `dart/README.md`、`docs/dev/icon-matching.md`、`docs/getting-started.md` 已对齐 manifest pack 模型

---

## 双模式 API 原则：懂 / 不懂 infer-core 都能用

**一条规则：** 要么传入 infer-core registry 实例（借用），要么不传——由 ui-extractor **自己** `infer_registry_create` 一个（自有）。

| 用户 | 怎么做 | 需要知道什么 |
|------|--------|--------------|
| **不懂 infer-core** | `ExtractEngine::open(config)` / `ui_extractor_create(NULL, json, err)` | 模型目录、pack 名、JSON 配置 |
| **懂 infer-core** | `ExtractEngine::from_registry(reg, config)` / `ui_extractor_create(reg, json, err)` | infer-core 生命周期 + 同一 `infer_core.dll` |

### Rust API

```rust
pub fn ExtractEngine::new(config: ExtractConfig, registry: Option<Registry>) -> Result<Self>;
pub fn ExtractEngine::open(config: ExtractConfig) -> Result<Self>;          // new(config, None)
pub fn ExtractEngine::from_registry(registry: Registry, config: ExtractConfig) -> Result<Self>;
```

### C FFI

```c
void *ui_extractor_create(void *infer_registry, const char *config_json, char **out_error);
#define ui_extractor_create_standalone(json, err) ui_extractor_create(NULL, (json), (err))
void *ui_extractor_create_from_registry(void *infer_registry, const char *config_json, char **out_error);
```

### Dart API

```dart
UiExtractorEngine.create(ExtractorConfig(...));
UiExtractorEngine.createWithRegistry(LocalInferRegistry registry, ExtractorLayoutConfig(...));
```

---

## 目标架构（总览）

```
local_infer_core dart  →  infer_core 单实例 + Registry + RuntimeConfig + PackCatalog
ui_extractor dart      →  仅 ui_extractor.dll + layout/编排 FFI；依赖 local_infer_core
Mauchat / 其他宿主       →  models_dir 装一次；RuntimeConfig 一处配置；registry 一处 create
```

Rust / native：

- git dep `infer-core`（`types-only`）：**类型与契约对齐**
- 动态 `infer_core.dll`：**推理运行时隔离**
- `ExtractEngine::from_registry` + `ui_extractor_create_from_registry`：**消除双 registry**

---

## 相关文档

- [../architecture.md](../architecture.md) — 流水线概览
- [../models.md](../models.md) — manifest 模型包
- [local-infer-core PRODUCT.md](https://github.com/SuiltaPico/local-infer-core/blob/master/docs/dev/PRODUCT.md)
- [local-infer-core DART_API.md](https://github.com/SuiltaPico/local-infer-core/blob/master/docs/dev/DART_API.md)
