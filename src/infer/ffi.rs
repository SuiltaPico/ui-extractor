use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
use std::ptr;

use crate::infer::error::{InferError, Result};

extern "C" {
    fn infer_string_free(s: *mut c_char);
    fn infer_floats_free(data: *mut f32, len: usize);
    fn infer_registry_create(
        models_dir: *const c_char,
        runtime_config_json: *const c_char,
        out_error: *mut *mut c_char,
    ) -> *mut c_void;
    fn infer_registry_destroy(handle: *mut c_void);
    fn infer_ocr_engine_load(
        registry: *mut c_void,
        pack_id: *const c_char,
        out_error: *mut *mut c_char,
    ) -> *mut c_void;
    fn infer_ocr_engine_destroy(engine: *mut c_void);
    fn infer_ocr_engine_apply_config(
        engine: *mut c_void,
        min_confidence: f32,
        max_side: u32,
        out_error: *mut *mut c_char,
    ) -> c_int;
    fn infer_ocr_recognize_timed(
        engine: *mut c_void,
        data: *const u8,
        len: usize,
        out_json: *mut *mut c_char,
        out_error: *mut *mut c_char,
    ) -> c_int;
    fn infer_embed_engine_load(
        registry: *mut c_void,
        pack_id: *const c_char,
        out_error: *mut *mut c_char,
    ) -> *mut c_void;
    fn infer_embed_engine_destroy(engine: *mut c_void);
    fn infer_embed_rgb256(
        engine: *mut c_void,
        rgb256: *const u8,
        rgb_len: usize,
        out_dim: *mut usize,
        out_error: *mut *mut c_char,
    ) -> *mut f32;
    fn infer_embed_rgb256_batch(
        engine: *mut c_void,
        rgb_batch: *const u8,
        rgb_len: usize,
        count: usize,
        out_count: *mut usize,
        out_dim: *mut usize,
        out_error: *mut *mut c_char,
    ) -> *mut f32;
    fn infer_registry_pack_ids_json(
        handle: *mut c_void,
        out_json: *mut *mut c_char,
        out_error: *mut *mut c_char,
    ) -> c_int;
    fn infer_registry_manifest_json(
        handle: *mut c_void,
        pack_id: *const c_char,
        out_json: *mut *mut c_char,
        out_error: *mut *mut c_char,
    ) -> c_int;
    fn infer_icon_index_load(
        registry: *mut c_void,
        pack_id: *const c_char,
        out_error: *mut *mut c_char,
    ) -> *mut c_void;
    fn infer_icon_index_destroy(index: *mut c_void);
    fn infer_icon_index_match_embedding(
        index: *mut c_void,
        embedding: *const f32,
        dim: usize,
        min_cosine: f32,
        out_json: *mut *mut c_char,
        out_error: *mut *mut c_char,
    ) -> c_int;
    fn infer_icon_index_match_embeddings_batch(
        index: *mut c_void,
        embeddings: *const f32,
        count: usize,
        dim: usize,
        min_cosine: f32,
        out_json: *mut *mut c_char,
        out_error: *mut *mut c_char,
    ) -> c_int;
    fn infer_icon_index_search(
        index: *mut c_void,
        embedding: *const f32,
        dim: usize,
        top_k: usize,
        out_json: *mut *mut c_char,
        out_error: *mut *mut c_char,
    ) -> c_int;
}

fn c_string(text: &str) -> Result<CString> {
    CString::new(text).map_err(|_| InferError::Ffi("string contains NUL byte".into()))
}

fn take_error(err: *mut c_char) -> InferError {
    if err.is_null() {
        return InferError::Ffi("unknown infer_core error".into());
    }
    let message = unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() };
    unsafe { infer_string_free(err) };
    InferError::Ffi(message)
}

fn take_string(out: *mut c_char) -> Result<String> {
    if out.is_null() {
        return Err(InferError::Ffi("null output string".into()));
    }
    let text = unsafe {
        let s = CStr::from_ptr(out).to_string_lossy().into_owned();
        infer_string_free(out);
        s
    };
    Ok(text)
}

pub fn registry_create(models_dir: &Path, runtime_json: Option<&str>) -> Result<*mut c_void> {
    let dir = c_string(models_dir.to_string_lossy().as_ref())?;
    let runtime = runtime_json
        .map(c_string)
        .transpose()?
        .map(|s| s.into_raw())
        .unwrap_or(ptr::null_mut());
    let mut err: *mut c_char = ptr::null_mut();
    let handle = unsafe {
        infer_registry_create(
            dir.as_ptr(),
            runtime,
            &mut err as *mut *mut c_char,
        )
    };
    if !runtime.is_null() {
        unsafe {
            drop(CString::from_raw(runtime));
        }
    }
    if handle.is_null() {
        return Err(take_error(err));
    }
    Ok(handle)
}

pub fn registry_destroy(handle: *mut c_void) {
    if !handle.is_null() {
        unsafe { infer_registry_destroy(handle) };
    }
}

pub fn ocr_engine_load(registry: *mut c_void, pack_id: &str) -> Result<*mut c_void> {
    let pack = c_string(pack_id)?;
    let mut err: *mut c_char = ptr::null_mut();
    let handle = unsafe {
        infer_ocr_engine_load(registry, pack.as_ptr(), &mut err as *mut *mut c_char)
    };
    if handle.is_null() {
        return Err(take_error(err));
    }
    Ok(handle)
}

pub fn ocr_engine_destroy(handle: *mut c_void) {
    if !handle.is_null() {
        unsafe { infer_ocr_engine_destroy(handle) };
    }
}

pub fn ocr_engine_apply_config(handle: *mut c_void, min_confidence: f32, max_side: u32) -> Result<()> {
    let mut err: *mut c_char = ptr::null_mut();
    let rc = unsafe {
        infer_ocr_engine_apply_config(
            handle,
            min_confidence,
            max_side,
            &mut err as *mut *mut c_char,
        )
    };
    if rc != 0 {
        return Err(take_error(err));
    }
    Ok(())
}

pub fn ocr_recognize_timed(handle: *mut c_void, image_bytes: &[u8]) -> Result<String> {
    let mut out_json: *mut c_char = ptr::null_mut();
    let mut err: *mut c_char = ptr::null_mut();
    let rc = unsafe {
        infer_ocr_recognize_timed(
            handle,
            image_bytes.as_ptr(),
            image_bytes.len(),
            &mut out_json as *mut *mut c_char,
            &mut err as *mut *mut c_char,
        )
    };
    if rc != 0 {
        return Err(take_error(err));
    }
    take_string(out_json)
}

pub fn embed_engine_load(registry: *mut c_void, pack_id: &str) -> Result<*mut c_void> {
    let pack = c_string(pack_id)?;
    let mut err: *mut c_char = ptr::null_mut();
    let handle = unsafe {
        infer_embed_engine_load(registry, pack.as_ptr(), &mut err as *mut *mut c_char)
    };
    if handle.is_null() {
        return Err(take_error(err));
    }
    Ok(handle)
}

pub fn embed_engine_destroy(handle: *mut c_void) {
    if !handle.is_null() {
        unsafe { infer_embed_engine_destroy(handle) };
    }
}

pub fn embed_rgb256(handle: *mut c_void, rgb_bytes: &[u8]) -> Result<Vec<f32>> {
    let mut dim = 0usize;
    let mut err: *mut c_char = ptr::null_mut();
    let ptr = unsafe {
        infer_embed_rgb256(
            handle,
            rgb_bytes.as_ptr(),
            rgb_bytes.len(),
            &mut dim as *mut usize,
            &mut err as *mut *mut c_char,
        )
    };
    if ptr.is_null() {
        return Err(take_error(err));
    }
    let values = unsafe {
        let slice = std::slice::from_raw_parts(ptr, dim);
        let owned = slice.to_vec();
        infer_floats_free(ptr, dim);
        owned
    };
    Ok(values)
}

pub fn embed_rgb256_batch(handle: *mut c_void, rgb_batches: &[&[u8]]) -> Result<Vec<Vec<f32>>> {
    if rgb_batches.is_empty() {
        return Ok(Vec::new());
    }
    let per_image = crate::infer::INPUT_SIZE as usize * crate::infer::INPUT_SIZE as usize * 3;
    for (i, bytes) in rgb_batches.iter().enumerate() {
        if bytes.len() != per_image {
            return Err(InferError::Embed(format!(
                "rgb256[{i}] must be {per_image} bytes, got {}",
                bytes.len()
            )));
        }
    }
    let flat: Vec<u8> = rgb_batches.iter().flat_map(|b| b.iter().copied()).collect();
    let mut count = 0usize;
    let mut dim = 0usize;
    let mut err: *mut c_char = ptr::null_mut();
    let ptr = unsafe {
        infer_embed_rgb256_batch(
            handle,
            flat.as_ptr(),
            flat.len(),
            rgb_batches.len(),
            &mut count as *mut usize,
            &mut dim as *mut usize,
            &mut err as *mut *mut c_char,
        )
    };
    if ptr.is_null() {
        return Err(take_error(err));
    }
    let values = unsafe {
        let slice = std::slice::from_raw_parts(ptr, count * dim);
        let owned = slice.to_vec();
        infer_floats_free(ptr, count * dim);
        owned
    };
    if dim == 0 {
        return Ok(vec![Vec::new(); count]);
    }
    Ok(values
        .chunks(dim)
        .map(|chunk| chunk.to_vec())
        .collect())
}

pub fn registry_pack_ids_json(handle: *mut c_void) -> Result<String> {
    let mut out_json: *mut c_char = ptr::null_mut();
    let mut err: *mut c_char = ptr::null_mut();
    let rc = unsafe {
        infer_registry_pack_ids_json(
            handle,
            &mut out_json as *mut *mut c_char,
            &mut err as *mut *mut c_char,
        )
    };
    if rc != 0 {
        return Err(take_error(err));
    }
    take_string(out_json)
}

pub fn registry_manifest_json(handle: *mut c_void, pack_id: &str) -> Result<String> {
    let pack = c_string(pack_id)?;
    let mut out_json: *mut c_char = ptr::null_mut();
    let mut err: *mut c_char = ptr::null_mut();
    let rc = unsafe {
        infer_registry_manifest_json(
            handle,
            pack.as_ptr(),
            &mut out_json as *mut *mut c_char,
            &mut err as *mut *mut c_char,
        )
    };
    if rc != 0 {
        return Err(take_error(err));
    }
    take_string(out_json)
}

pub fn icon_index_load(registry: *mut c_void, pack_id: &str) -> Result<*mut c_void> {
    let pack = c_string(pack_id)?;
    let mut err: *mut c_char = ptr::null_mut();
    let handle = unsafe {
        infer_icon_index_load(registry, pack.as_ptr(), &mut err as *mut *mut c_char)
    };
    if handle.is_null() {
        return Err(take_error(err));
    }
    Ok(handle)
}

pub fn icon_index_destroy(handle: *mut c_void) {
    if !handle.is_null() {
        unsafe { infer_icon_index_destroy(handle) };
    }
}

pub fn icon_index_match_embedding(
    handle: *mut c_void,
    embedding: &[f32],
    min_cosine: f32,
) -> Result<String> {
    let mut out_json: *mut c_char = ptr::null_mut();
    let mut err: *mut c_char = ptr::null_mut();
    let rc = unsafe {
        infer_icon_index_match_embedding(
            handle,
            embedding.as_ptr(),
            embedding.len(),
            min_cosine,
            &mut out_json as *mut *mut c_char,
            &mut err as *mut *mut c_char,
        )
    };
    if rc != 0 {
        return Err(take_error(err));
    }
    take_string(out_json)
}

pub fn icon_index_match_embeddings_batch(
    handle: *mut c_void,
    embeddings: &[f32],
    dim: usize,
    min_cosine: f32,
) -> Result<String> {
    if dim == 0 {
        return Err(InferError::Ffi("embedding dim must be > 0".into()));
    }
    if !embeddings.len().is_multiple_of(dim) {
        return Err(InferError::Ffi(format!(
            "embeddings length {} is not a multiple of dim {dim}",
            embeddings.len()
        )));
    }
    let count = embeddings.len() / dim;
    let mut out_json: *mut c_char = ptr::null_mut();
    let mut err: *mut c_char = ptr::null_mut();
    let rc = unsafe {
        infer_icon_index_match_embeddings_batch(
            handle,
            embeddings.as_ptr(),
            count,
            dim,
            min_cosine,
            &mut out_json as *mut *mut c_char,
            &mut err as *mut *mut c_char,
        )
    };
    if rc != 0 {
        return Err(take_error(err));
    }
    take_string(out_json)
}

pub fn icon_index_search(handle: *mut c_void, embedding: &[f32], top_k: usize) -> Result<String> {
    let mut out_json: *mut c_char = ptr::null_mut();
    let mut err: *mut c_char = ptr::null_mut();
    let rc = unsafe {
        infer_icon_index_search(
            handle,
            embedding.as_ptr(),
            embedding.len(),
            top_k,
            &mut out_json as *mut *mut c_char,
            &mut err as *mut *mut c_char,
        )
    };
    if rc != 0 {
        return Err(take_error(err));
    }
    take_string(out_json)
}
