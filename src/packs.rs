use std::path::{Path, PathBuf};

#[cfg(target_os = "android")]
pub const DEFAULT_OCR_PACK: &str = "ocr.paddle.ppocr6-tiny.mnn.fp32";
#[cfg(not(target_os = "android"))]
pub const DEFAULT_OCR_PACK: &str = "ocr.paddle.ppocr6-tiny.onnx.fp32";

#[cfg(target_os = "android")]
pub const DEFAULT_EMBED_PACK: &str = "embed.mobileclip2-s0.mnn.fp32";
#[cfg(not(target_os = "android"))]
pub const DEFAULT_EMBED_PACK: &str = "embed.mobileclip2-s0.onnx.fp32";

pub const DEFAULT_ICON_INDEX_PACK: &str = "icons.bundled.v1.mobileclip2-s0.int8";

/// Resolve models root: `LOCAL_INFER_ROOT` env, then `models_dir` argument, then `./models`.
pub fn resolve_models_dir(explicit: Option<&Path>) -> PathBuf {
    if let Ok(root) = std::env::var("LOCAL_INFER_ROOT") {
        if !root.is_empty() {
            return PathBuf::from(root);
        }
    }
    explicit
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("models"))
}
