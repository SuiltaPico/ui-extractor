//! C ABI for embedding `ui-extractor` as a dynamic library.
//!
//! All `char*` path arguments must be valid UTF-8. String outputs allocated by this
//! library must be freed with [`ui_extractor_string_free`].

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::ptr;

use image::DynamicImage;
use serde::{Deserialize, Serialize};

use crate::engine::ExtractEngine;
use crate::infer::Registry;
use crate::pipeline::{ExtractConfig, ExtractTimings};
use crate::types::ExtractResult;
use crate::{IconConfig, LayoutConfig, OcrConfig};

const OK: c_int = 0;
const ERR: c_int = -1;

pub struct ExtractEngineHandle(ExtractEngine);

fn set_error(out_error: *mut *mut c_char, message: impl Into<String>) {
    if !out_error.is_null() {
        unsafe {
            *out_error = string_to_raw(message);
        }
    }
}

fn clear_error(out_error: *mut *mut c_char) {
    if !out_error.is_null() {
        unsafe {
            *out_error = ptr::null_mut();
        }
    }
}

fn string_to_raw(message: impl Into<String>) -> *mut c_char {
    CString::new(message.into())
        .map(CString::into_raw)
        .unwrap_or(ptr::null_mut())
}

fn read_cstr(c: *const c_char) -> Result<&'static str, String> {
    if c.is_null() {
        return Err("null string".into());
    }
    unsafe { CStr::from_ptr(c) }
        .to_str()
        .map_err(|e| format!("invalid UTF-8: {e}"))
}

fn read_bytes(data: *const u8, len: usize) -> Result<&'static [u8], String> {
    if data.is_null() || len == 0 {
        return Err("empty image buffer".into());
    }
    Ok(unsafe { std::slice::from_raw_parts(data, len) })
}

fn run<F>(out_error: *mut *mut c_char, f: F) -> c_int
where
    F: FnOnce() -> Result<(), String>,
{
    match catch_unwind(AssertUnwindSafe(|| f())) {
        Ok(Ok(())) => {
            clear_error(out_error);
            OK
        }
        Ok(Err(message)) => {
            set_error(out_error, message);
            ERR
        }
        Err(_) => {
            set_error(out_error, "internal panic");
            ERR
        }
    }
}

#[derive(Debug, Deserialize)]
struct EngineConfigJson {
    #[serde(default = "default_true")]
    run_ocr: bool,
    #[serde(default = "default_true")]
    run_icon: bool,
    models_dir: Option<String>,
    ocr_pack: Option<String>,
    icon_index_pack: Option<String>,
    runtime: Option<serde_json::Value>,
    #[serde(default)]
    layout: LayoutConfigJson,
    #[serde(default)]
    ocr: OcrConfigJson,
    #[serde(default)]
    icon: IconConfigJson,
}

#[derive(Debug, Deserialize, Default)]
struct LayoutConfigJson {
    min_area: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
struct OcrConfigJson {
    max_side: Option<u32>,
    min_confidence: Option<f32>,
}

#[derive(Debug, Deserialize, Default)]
struct IconConfigJson {
    template_size: Option<u32>,
    min_cosine: Option<f64>,
    min_side: Option<i32>,
    max_side: Option<i32>,
    min_aspect: Option<f64>,
    max_aspect: Option<f64>,
}

fn default_true() -> bool {
    true
}

fn config_from_json(json: &str, borrowed_registry: bool) -> Result<ExtractConfig, String> {
    use crate::packs::{resolve_models_dir, DEFAULT_ICON_INDEX_PACK, DEFAULT_OCR_PACK};

    let parsed: EngineConfigJson =
        serde_json::from_str(json).map_err(|e| format!("invalid config JSON: {e}"))?;

    let mut layout = LayoutConfig::default();
    if let Some(min_area) = parsed.layout.min_area {
        layout.min_area = min_area;
    }

    let mut ocr = OcrConfig::default();
    if let Some(max_side) = parsed.ocr.max_side {
        ocr.max_side = max_side;
    }
    if let Some(min_confidence) = parsed.ocr.min_confidence {
        ocr.min_confidence = min_confidence;
    }

    let mut icon = IconConfig::default();
    if let Some(v) = parsed.icon.template_size {
        icon.template_size = v;
    }
    if let Some(v) = parsed.icon.min_cosine {
        icon.min_cosine = v;
    }
    if let Some(v) = parsed.icon.min_side {
        icon.min_side = v;
    }
    if let Some(v) = parsed.icon.max_side {
        icon.max_side = v;
    }
    if let Some(v) = parsed.icon.min_aspect {
        icon.min_aspect = v;
    }
    if let Some(v) = parsed.icon.max_aspect {
        icon.max_aspect = v;
    }

    let runtime = match parsed.runtime {
        Some(v) => serde_json::from_value(v).map_err(|e| e.to_string())?,
        None => crate::infer::RuntimeConfig::default(),
    };

    let models_dir = if borrowed_registry {
        PathBuf::from(".")
    } else {
        resolve_models_dir(parsed.models_dir.as_deref().map(Path::new))
    };

    Ok(ExtractConfig {
        models_dir,
        runtime,
        ocr_pack: parsed
            .ocr_pack
            .unwrap_or_else(|| DEFAULT_OCR_PACK.to_string()),
        icon_index_pack: parsed
            .icon_index_pack
            .unwrap_or_else(|| DEFAULT_ICON_INDEX_PACK.to_string()),
        layout,
        ocr,
        icon,
        run_ocr: parsed.run_ocr,
        run_icon: parsed.run_icon,
        pipeline_dump_dir: None,
    })
}

fn map_extract_error(err: crate::ExtractError) -> String {
    err.to_string()
}

fn load_image_bytes(bytes: &[u8]) -> Result<DynamicImage, String> {
    image::load_from_memory(bytes).map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct ExtractJsonOutput {
    #[serde(flatten)]
    result: ExtractResult,
    timings: ExtractTimings,
}

fn extract_result_json(result: ExtractResult, timings: ExtractTimings) -> Result<String, String> {
    serde_json::to_string(&ExtractJsonOutput { result, timings })
        .map_err(|e| e.to_string())
}

fn open_engine(
    infer_registry: *mut c_void,
    config_json: &str,
) -> Result<ExtractEngine, String> {
    let borrowed = !infer_registry.is_null();
    let config = config_from_json(config_json, borrowed)?;
    if borrowed {
        let registry = Registry::from_borrowed(infer_registry);
        ExtractEngine::from_registry(registry, config).map_err(map_extract_error)
    } else {
        ExtractEngine::open(config).map_err(map_extract_error)
    }
}

/// Library version string (static, do not free).
#[no_mangle]
pub extern "C" fn ui_extractor_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

/// Free a string previously returned by this library.
#[no_mangle]
pub unsafe extern "C" fn ui_extractor_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/*
 * infer_registry: NULL → ui-extractor opens its own infer-core registry
 *                  non-NULL → borrow existing InferRegistry* (not destroyed on close)
 * config_json: layout/ocr/icon + pack ids; models_dir/runtime ignored when borrowing
 */
#[no_mangle]
pub extern "C" fn ui_extractor_create(
    infer_registry: *mut c_void,
    config_json: *const c_char,
    out_error: *mut *mut c_char,
) -> *mut c_void {
    match catch_unwind(AssertUnwindSafe(|| {
        let json = read_cstr(config_json)?;
        open_engine(infer_registry, json)
            .map(|engine| Box::into_raw(Box::new(ExtractEngineHandle(engine))) as *mut c_void)
    })) {
        Ok(Ok(ptr)) => {
            clear_error(out_error);
            ptr
        }
        Ok(Err(message)) => {
            set_error(out_error, message);
            ptr::null_mut()
        }
        Err(_) => {
            set_error(out_error, "internal panic");
            ptr::null_mut()
        }
    }
}

/// Standalone create: equivalent to `ui_extractor_create(NULL, config_json, out_error)`.
#[no_mangle]
pub extern "C" fn ui_extractor_create_standalone(
    config_json: *const c_char,
    out_error: *mut *mut c_char,
) -> *mut c_void {
    ui_extractor_create(ptr::null_mut(), config_json, out_error)
}

/// Borrow-mode alias for `ui_extractor_create(infer_registry, config_json, out_error)`.
#[no_mangle]
pub extern "C" fn ui_extractor_create_from_registry(
    infer_registry: *mut c_void,
    config_json: *const c_char,
    out_error: *mut *mut c_char,
) -> *mut c_void {
    ui_extractor_create(infer_registry, config_json, out_error)
}

/// Destroy a handle from [`ui_extractor_create`].
#[no_mangle]
pub unsafe extern "C" fn ui_extractor_destroy(handle: *mut c_void) {
    if !handle.is_null() {
        drop(Box::from_raw(handle as *mut ExtractEngineHandle));
    }
}

/// Extract UI tree from image bytes; writes JSON to `out_json` (caller frees).
#[no_mangle]
pub extern "C" fn ui_extractor_extract_bytes(
    handle: *mut c_void,
    data: *const u8,
    len: usize,
    out_json: *mut *mut c_char,
    out_error: *mut *mut c_char,
) -> c_int {
    run(out_error, || {
        let engine = unsafe {
            (handle as *mut ExtractEngineHandle)
                .as_mut()
                .ok_or_else(|| "null extractor handle".to_string())?
        };
        let bytes = read_bytes(data, len)?;
        let img = load_image_bytes(bytes)?;
        let (result, timings) = engine
            .0
            .extract_from_image(&img)
            .map_err(map_extract_error)?;
        let json = extract_result_json(result, timings)?;
        if !out_json.is_null() {
            unsafe {
                *out_json = string_to_raw(json);
            }
        }
        Ok(())
    })
}

/// Extract UI tree from an image file path; writes JSON to `out_json`.
#[no_mangle]
pub extern "C" fn ui_extractor_extract_file(
    handle: *mut c_void,
    path: *const c_char,
    out_json: *mut *mut c_char,
    out_error: *mut *mut c_char,
) -> c_int {
    run(out_error, || {
        let engine = unsafe {
            (handle as *mut ExtractEngineHandle)
                .as_mut()
                .ok_or_else(|| "null extractor handle".to_string())?
        };
        let path = read_cstr(path)?;
        let (result, timings) = engine
            .0
            .extract_from_path(Path::new(path))
            .map_err(map_extract_error)?;
        let json = extract_result_json(result, timings)?;
        if !out_json.is_null() {
            unsafe {
                *out_json = string_to_raw(json);
            }
        }
        Ok(())
    })
}

/// Reload icon embeddings after changing model paths in the engine config.
#[no_mangle]
pub extern "C" fn ui_extractor_reload_icon_pack(
    handle: *mut c_void,
    out_error: *mut *mut c_char,
) -> c_int {
    run(out_error, || {
        let engine = unsafe {
            (handle as *mut ExtractEngineHandle)
                .as_mut()
                .ok_or_else(|| "null extractor handle".to_string())?
        };
        engine.0.reload_icon_pack().map_err(map_extract_error)?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_engine_config() {
        let cfg = config_from_json(r#"{"run_ocr": false, "run_icon": false}"#, false).unwrap();
        assert!(!cfg.run_ocr);
        assert!(!cfg.run_icon);
    }

    #[test]
    fn borrowed_mode_ignores_models_dir_in_json() {
        let cfg = config_from_json(
            r#"{"models_dir":"/should/be/ignored","run_ocr":false,"run_icon":false}"#,
            true,
        )
        .unwrap();
        assert_eq!(cfg.models_dir, PathBuf::from("."));
    }

    #[test]
    fn extract_json_includes_timings_object() {
        use crate::types::{ExtractResult, UiElement, UiElementKind};
        use crate::ocr::OcrTimings;
        use crate::icon::{IconMatchStats, IconTimings};

        let result = ExtractResult {
            width: 100,
            height: 200,
            root: UiElement {
                bounds: crate::types::Bounds::new(0, 0, 100, 200),
                kind: UiElementKind::Root,
                children: vec![],
            },
        };
        let timings = ExtractTimings {
            gray_ms: 1.5,
            layout_ms: 42.0,
            parallel_ms: 120.0,
            ocr: OcrTimings {
                init_ms: 0.0,
                predict_ms: 80.0,
            },
            attach_words_ms: 3.0,
            icon: IconMatchStats {
                candidates: 4,
                matched: 2,
                timings: IconTimings {
                    load_ms: 0.0,
                    match_ms: 25.0,
                },
            },
            ..ExtractTimings::default()
        };

        let json = extract_result_json(result, timings).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["width"], 100);
        assert_eq!(parsed["timings"]["layout_ms"], 42.0);
        assert_eq!(parsed["timings"]["ocr"]["predict_ms"], 80.0);
        assert_eq!(parsed["timings"]["icon"]["matched"], 2);
    }
}
