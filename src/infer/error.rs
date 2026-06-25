use thiserror::Error;

pub type Result<T> = std::result::Result<T, InferError>;

#[derive(Debug, Error)]
pub enum InferError {
    #[error("manifest: {0}")]
    Manifest(String),

    #[error("license: {0}")]
    License(String),

    #[error("pack not found: {0}")]
    PackNotFound(String),

    #[error("ocr: {0}")]
    Ocr(String),

    #[error("embed: {0}")]
    Embed(String),

    #[error("icon index: {0}")]
    IconIndex(String),

    #[error("runtime: {0}")]
    Runtime(String),

    #[error("ffi: {0}")]
    Ffi(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}
