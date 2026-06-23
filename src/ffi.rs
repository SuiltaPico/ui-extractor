//! C ABI for embedding `ui-extractor` as a dynamic library.
//!
//! All `char*` path arguments must be valid UTF-8. String outputs allocated by this
//! library must be freed with [`ui_extractor_string_free`].

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int, c_uint, c_void};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::ptr;

use image::DynamicImage;
use serde::Deserialize;

use crate::engine::ExtractEngine;
use crate::icon::{build_embedding_index, IconEmbedder, IconMatchOptions, IconPack, EMBED_DIM};
use crate::pipeline::ExtractConfig;
use crate::types::Bounds;
use crate::{ExtractError, IconConfig, LayoutConfig, OcrConfig};

const OK: c_int = 0;
const ERR: c_int = -1;

pub struct ExtractEngineHandle(ExtractEngine);
pub struct IconPackHandle(IconPack);

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

fn read_path(c: *const c_char) -> Result<&'static str, String> {
    if c.is_null() {
        return Err("null path".into());
    }
    unsafe { CStr::from_ptr(c) }
        .to_str()
        .map_err(|e| format!("invalid UTF-8 path: {e}"))
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
    model_dir: Option<String>,
    max_side: Option<u32>,
    min_confidence: Option<f32>,
}

#[derive(Debug, Deserialize, Default)]
struct IconConfigJson {
    embedding_index: Option<String>,
    vision_model: Option<String>,
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

fn config_from_json(json: &str) -> Result<ExtractConfig, String> {
    let parsed: EngineConfigJson =
        serde_json::from_str(json).map_err(|e| format!("invalid config JSON: {e}"))?;

    let mut layout = LayoutConfig::default();
    if let Some(min_area) = parsed.layout.min_area {
        layout.min_area = min_area;
    }

    let mut ocr = OcrConfig::default();
    if let Some(dir) = parsed.ocr.model_dir {
        ocr.model_dir = dir.into();
    }
    if let Some(max_side) = parsed.ocr.max_side {
        ocr.max_side = max_side;
    }
    if let Some(min_confidence) = parsed.ocr.min_confidence {
        ocr.min_confidence = min_confidence;
    }

    let mut icon = IconConfig::default();
    if let Some(v) = parsed.icon.embedding_index {
        icon.embedding_index = v.into();
    }
    if let Some(v) = parsed.icon.vision_model {
        icon.vision_model = v.into();
    }
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

    Ok(ExtractConfig {
        layout,
        ocr,
        icon,
        run_ocr: parsed.run_ocr,
        run_icon: parsed.run_icon,
        pipeline_dump_dir: None,
    })
}

fn map_extract_error(err: ExtractError) -> String {
    err.to_string()
}

fn load_image_bytes(bytes: &[u8]) -> Result<DynamicImage, String> {
    image::load_from_memory(bytes).map_err(|e| e.to_string())
}

fn icon_match_options(min_cosine: c_double) -> IconMatchOptions {
    IconMatchOptions { min_cosine }
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

/// Create an extractor from a JSON config string. Returns null on failure.
#[no_mangle]
pub extern "C" fn ui_extractor_create(
    config_json: *const c_char,
    out_error: *mut *mut c_char,
) -> *mut c_void {
    match catch_unwind(AssertUnwindSafe(|| {
        let json = read_path(config_json)?;
        let config = config_from_json(json)?;
        ExtractEngine::new(config)
            .map(|engine| Box::into_raw(Box::new(ExtractEngineHandle(engine))) as *mut c_void)
            .map_err(map_extract_error)
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
        let (result, _timings) = engine
            .0
            .extract_from_image(&img)
            .map_err(map_extract_error)?;
        let json = serde_json::to_string(&result).map_err(|e| e.to_string())?;
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
        let path = read_path(path)?;
        let (result, _timings) = engine
            .0
            .extract_from_path(Path::new(path))
            .map_err(map_extract_error)?;
        let json = serde_json::to_string(&result).map_err(|e| e.to_string())?;
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

/// Load an icon pack from a precomputed embedding index.
#[no_mangle]
pub extern "C" fn ui_icon_pack_load(
    embedding_index: *const c_char,
    vision_model: *const c_char,
    template_size: c_uint,
    min_cosine: c_double,
    out_error: *mut *mut c_char,
) -> *mut c_void {
    match catch_unwind(AssertUnwindSafe(|| {
        let embedding_index = read_path(embedding_index)?;
        let vision_model = read_path(vision_model)?;
        IconPack::load(
            embedding_index,
            vision_model,
            template_size,
            icon_match_options(min_cosine),
        )
        .map(|pack| Box::into_raw(Box::new(IconPackHandle(pack))) as *mut c_void)
        .map_err(map_extract_error)
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

/// Build an icon pack by embedding every PNG under `png_dir` (offline index build).
#[no_mangle]
pub extern "C" fn ui_icon_pack_build(
    png_dir: *const c_char,
    vision_model: *const c_char,
    template_size: c_uint,
    min_cosine: c_double,
    out_error: *mut *mut c_char,
) -> *mut c_void {
    match catch_unwind(AssertUnwindSafe(|| {
        let png_dir = read_path(png_dir)?;
        let vision_model = read_path(vision_model)?;
        IconPack::build_from_dir(
            png_dir,
            vision_model,
            template_size,
            icon_match_options(min_cosine),
        )
        .map(|pack| Box::into_raw(Box::new(IconPackHandle(pack))) as *mut c_void)
        .map_err(map_extract_error)
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

/// Destroy a handle from [`ui_icon_pack_load`] / [`ui_icon_pack_build`].
#[no_mangle]
pub unsafe extern "C" fn ui_icon_pack_destroy(handle: *mut c_void) {
    if !handle.is_null() {
        drop(Box::from_raw(handle as *mut IconPackHandle));
    }
}

/// Save the in-memory embedding index to disk (`embed-mdi` output format).
#[no_mangle]
pub extern "C" fn ui_icon_pack_save_embeddings(
    handle: *mut c_void,
    path: *const c_char,
    out_error: *mut *mut c_char,
) -> c_int {
    run(out_error, || {
        let pack = unsafe {
            (handle as *mut IconPackHandle)
                .as_mut()
                .ok_or_else(|| "null icon pack handle".to_string())?
        };
        let path = read_path(path)?;
        pack.0.save_embeddings(path).map_err(map_extract_error)?;
        Ok(())
    })
}

/// Embed a template PNG file (library-style, color on white). Writes `dim` floats to `out_embedding`.
#[no_mangle]
pub extern "C" fn ui_icon_pack_embed_png_file(
    handle: *mut c_void,
    png_path: *const c_char,
    out_embedding: *mut f32,
    dim: c_uint,
    out_error: *mut *mut c_char,
) -> c_int {
    run(out_error, || {
        let pack = unsafe {
            (handle as *mut IconPackHandle)
                .as_mut()
                .ok_or_else(|| "null icon pack handle".to_string())?
        };
        if dim as usize != EMBED_DIM {
            return Err(format!("expected embedding dim {EMBED_DIM}, got {dim}"));
        }
        if out_embedding.is_null() {
            return Err("null out_embedding".into());
        }
        let png_path = read_path(png_path)?;
        let embedding = pack
            .0
            .embed_template_png(png_path)
            .map_err(map_extract_error)?;
        unsafe {
            ptr::copy_nonoverlapping(embedding.as_ptr(), out_embedding, EMBED_DIM);
        }
        Ok(())
    })
}

/// Embed an icon crop from image bytes (screenshot-style). Writes `dim` floats to `out_embedding`.
#[no_mangle]
pub extern "C" fn ui_icon_pack_embed_image_bytes(
    handle: *mut c_void,
    data: *const u8,
    len: usize,
    out_embedding: *mut f32,
    dim: c_uint,
    out_error: *mut *mut c_char,
) -> c_int {
    run(out_error, || {
        let pack = unsafe {
            (handle as *mut IconPackHandle)
                .as_mut()
                .ok_or_else(|| "null icon pack handle".to_string())?
        };
        if dim as usize != EMBED_DIM {
            return Err(format!("expected embedding dim {EMBED_DIM}, got {dim}"));
        }
        if out_embedding.is_null() {
            return Err("null out_embedding".into());
        }
        let bytes = read_bytes(data, len)?;
        let img = load_image_bytes(bytes)?;
        let embedding = pack.0.embed_query_image(&img).map_err(map_extract_error)?;
        unsafe {
            ptr::copy_nonoverlapping(embedding.as_ptr(), out_embedding, EMBED_DIM);
        }
        Ok(())
    })
}

/// Match a precomputed embedding against the pack. Writes JSON `{ "name", "score" }` or `null`.
#[no_mangle]
pub extern "C" fn ui_icon_pack_match_embedding(
    handle: *mut c_void,
    embedding: *const f32,
    dim: c_uint,
    out_json: *mut *mut c_char,
    out_error: *mut *mut c_char,
) -> c_int {
    run(out_error, || {
        let pack = unsafe {
            (handle as *mut IconPackHandle)
                .as_ref()
                .ok_or_else(|| "null icon pack handle".to_string())?
        };
        if dim as usize != EMBED_DIM {
            return Err(format!("expected embedding dim {EMBED_DIM}, got {dim}"));
        }
        if embedding.is_null() {
            return Err("null embedding".into());
        }
        let slice = unsafe { std::slice::from_raw_parts(embedding, EMBED_DIM) };
        let hit = pack.0.match_embedding(slice);
        let json = match hit {
            Some(h) => serde_json::to_string(&h).map_err(|e| e.to_string())?,
            None => "null".to_string(),
        };
        if !out_json.is_null() {
            unsafe {
                *out_json = string_to_raw(json);
            }
        }
        Ok(())
    })
}

/// Cosine search top-k. Writes JSON array of `{ "name", "score" }`.
#[no_mangle]
pub extern "C" fn ui_icon_pack_search_embedding(
    handle: *mut c_void,
    embedding: *const f32,
    dim: c_uint,
    top_k: c_uint,
    out_json: *mut *mut c_char,
    out_error: *mut *mut c_char,
) -> c_int {
    run(out_error, || {
        let pack = unsafe {
            (handle as *mut IconPackHandle)
                .as_ref()
                .ok_or_else(|| "null icon pack handle".to_string())?
        };
        if dim as usize != EMBED_DIM {
            return Err(format!("expected embedding dim {EMBED_DIM}, got {dim}"));
        }
        if embedding.is_null() {
            return Err("null embedding".into());
        }
        let slice = unsafe { std::slice::from_raw_parts(embedding, EMBED_DIM) };
        let hits = pack.0.search_embedding(slice, top_k as usize);
        let json = serde_json::to_string(&hits).map_err(|e| e.to_string())?;
        if !out_json.is_null() {
            unsafe {
                *out_json = string_to_raw(json);
            }
        }
        Ok(())
    })
}

/// Match an image file against the pack (whole frame treated as icon). Writes JSON hit or `null`.
#[no_mangle]
pub extern "C" fn ui_icon_pack_match_image_file(
    handle: *mut c_void,
    path: *const c_char,
    out_json: *mut *mut c_char,
    out_error: *mut *mut c_char,
) -> c_int {
    run(out_error, || {
        let pack = unsafe {
            (handle as *mut IconPackHandle)
                .as_mut()
                .ok_or_else(|| "null icon pack handle".to_string())?
        };
        let path = read_path(path)?;
        let img = image::open(path).map_err(|e| e.to_string())?;
        let hit = pack.0.match_image(&img).map_err(map_extract_error)?;
        let json = match hit {
            Some(h) => serde_json::to_string(&h).map_err(|e| e.to_string())?,
            None => "null".to_string(),
        };
        if !out_json.is_null() {
            unsafe {
                *out_json = string_to_raw(json);
            }
        }
        Ok(())
    })
}

/// Match a region `(x, y, width, height)` inside an image file.
#[no_mangle]
pub extern "C" fn ui_icon_pack_match_region_file(
    handle: *mut c_void,
    path: *const c_char,
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
    out_json: *mut *mut c_char,
    out_error: *mut *mut c_char,
) -> c_int {
    run(out_error, || {
        let pack = unsafe {
            (handle as *mut IconPackHandle)
                .as_mut()
                .ok_or_else(|| "null icon pack handle".to_string())?
        };
        let path = read_path(path)?;
        let img = image::open(path).map_err(|e| e.to_string())?;
        let bounds = Bounds::new(x, y, width, height);
        let hit = pack.0.match_region(&img, &bounds);
        let json = match hit {
            Some(h) => serde_json::to_string(&h).map_err(|e| e.to_string())?,
            None => "null".to_string(),
        };
        if !out_json.is_null() {
            unsafe {
                *out_json = string_to_raw(json);
            }
        }
        Ok(())
    })
}

/// Standalone helper: build `embeddings.bin` from a PNG directory without keeping a pack handle.
#[no_mangle]
pub extern "C" fn ui_icon_build_embeddings_file(
    png_dir: *const c_char,
    vision_model: *const c_char,
    template_size: c_uint,
    out_path: *const c_char,
    out_error: *mut *mut c_char,
) -> c_int {
    run(out_error, || {
        let png_dir = read_path(png_dir)?;
        let vision_model = read_path(vision_model)?;
        let out_path = read_path(out_path)?;
        let mut embedder = IconEmbedder::load(Path::new(vision_model)).map_err(map_extract_error)?;
        let index = build_embedding_index(Path::new(png_dir), &mut embedder, template_size)
            .map_err(map_extract_error)?;
        index.save(Path::new(out_path)).map_err(map_extract_error)?;
        Ok(())
    })
}

/// Embedding vector dimension (512 for MobileCLIP2-S0).
#[no_mangle]
pub extern "C" fn ui_icon_embedding_dim() -> c_uint {
    EMBED_DIM as c_uint
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_engine_config() {
        let cfg = config_from_json(r#"{"run_ocr": false, "run_icon": false}"#).unwrap();
        assert!(!cfg.run_ocr);
        assert!(!cfg.run_icon);
    }
}
